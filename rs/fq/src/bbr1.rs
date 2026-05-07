//! Translation of `picoquic/bbr1.c` — BBRv1 congestion control.
//!
//! Phase 4B: initial batch (BBR1AfterOneRoundtripInFastRecovery,
//! BBR1CheckFullPipe, BBR1EnterProbeRTT, BBR1EnterStartup).
//! Phase 4B missing batch (BBR1EnterStartupLongRTT, BBR1GetBtlBW,
//! BBR1ModulateCwndForProbeRTT, BBR1ModulateCwndForRecovery, BBR1RestoreCwnd).
//! Phase 4B approved missing batch (BBR1AdvanceCyclePhase, BBR1CheckProbeRTT,
//! BBR1ExitFastRecovery, BBR1ExitProbeRTT, plus helpers).
//! Phase 4B required-missing batch (BBR1ExitStartupSeedBDP, BBR1HandleProbeRTT,
//! BBR1Inflight, BBR1SaveCwnd, BBR1SetCwnd).
//! Phase 4B required-missing batch 2 (BBR1SetPacingRateWithGain,
//! BBR1UpdateControlParameters, BBR1UpdateModelAndState, plus helpers
//! BBR1UpdateBtlBw, BBR1CheckCyclePhase, BBR1CheckDrain).

use crate::cc_common::{ConnectionCc, MinMaxRtt, PathCc};
use crate::internal::{
    CWIN_INITIAL, CWIN_MINIMUM, Connection, MAX_PACKET_SIZE, Path, TARGET_RENO_RTT,
    TARGET_SATELLITE_RTT,
};
use crate::utils::{bytes_from_rate, rate_from_bytes};
use crate::{CongestionNotification, Instant, PerAckState};

// ---------------------------------------------------------------------------
// Constants from bbr1.c `#define`s.

const BBR1_HIGH_GAIN: f64 = 2.8853900817779; // C: BBR1_HIGH_GAIN (2/ln(2))
const BBR1_BTL_BW_FILTER_LENGTH: usize = 10; // C: BBR1_BTL_BW_FILTER_LENGTH
const BBR1_PROBE_RTT_INTERVAL: u64 = 10_000_000; // 10 sec in microseconds
const BBR1_PROBE_RTT_DURATION: u64 = 200_000; // 200ms in microseconds
const BBR1_PACING_RATE_LOW: f64 = 150_000.0; // 1.2 Mbps in B/s
const BBR1_PACING_RATE_MEDIUM: f64 = 3_000_000.0; // 24 Mbps in B/s
const BBR1_GAIN_CYCLE_LEN: usize = 8; // C: BBR1_GAIN_CYCLE_LEN
const BBR1_GAIN_CYCLE_MAX_START: u32 = 5; // C: BBR1_GAIN_CYCLE_MAX_START
const BBR1_LT_BW_INTERVAL_MIN_RTT: i32 = 4; // C: BBR1_LT_BW_INTERVAL_MIN_RTT
const BBR1_LT_BW_INTERVAL_MAX_RTT: i32 = 16; // C: 4*BBR1_LT_BW_INTERVAL_MIN_RTT
const BBR1_LT_BW_RATIO_SCALE: u64 = 1024; // C: BBR1_LT_BW_RATIO_SCALE
const BBR1_LT_BW_RATIO_SCALED_TARGET: u64 = 205; // C: BBR1_LT_BW_RATIO_SCALED_TARGET
const BBR1_LT_BW_RATIO_INVERSE: u64 = 8; // C: BBR1_LT_BW_RATIO_INVERSE
const BBR1_LT_BW_BYTES_PER_SEC_DIFF: u64 = 4000; // C: BBR1_LT_BW_BYTES_PER_SEC_DIFF
const BBR1_LT_BW_MAX_RTTS: i32 = 48; // C: BBR1_LT_BW_MAX_RTTS
/// RTT threshold above which slow start transitions to `StartupLongRtt`.
/// C: `BBR1_HYSTART_THRESHOLD_RTT` (50 ms in microseconds).
const BBR1_HYSTART_THRESHOLD_RTT: u64 = 50_000;

/// C: `bbr1_pacing_gain_cycle` (picoquic/bbr1.c:246)
static BBR1_PACING_GAIN_CYCLE: [f64; BBR1_GAIN_CYCLE_LEN] =
    [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.25, 0.75];

// ---------------------------------------------------------------------------
// Algorithm-state enum.  C: `picoquic_bbr1_alg_state_t`.

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Bbr1AlgState {
    #[default]
    Startup = 0,
    Drain,
    ProbeBw,
    ProbeRtt,
    StartupLongRtt,
}

// ---------------------------------------------------------------------------
// BBR1 state.  C: `picoquic_bbr1_state_t`.
//
// Type deviations from C:
//   * single-bit `unsigned int x : 1` bitfields → `bool`
//   * `const char* option_string` → `Option<String>` (owned copy)
//   * `picoquic_min_max_rtt_t rtt_filter` → `MinMaxRtt`
//   * `int` round/count fields → `i32` (faithfully signed)

pub struct Bbr1State {
    pub state: Bbr1AlgState,
    pub btl_bw: u64,
    pub next_round_delivered: u64,
    pub btl_bw_filter: [u64; BBR1_BTL_BW_FILTER_LENGTH],
    pub full_bw: u64,
    pub rt_prop: u64,
    pub rt_prop_stamp: u64,
    pub cycle_stamp: u64,
    pub probe_rtt_done_stamp: u64,
    pub prior_cwnd: u64,
    pub prior_in_flight: u64,
    pub bytes_delivered: u64,
    pub send_quantum: u64,
    pub rtt_filter: MinMaxRtt,
    pub target_cwnd: u64,
    pub pacing_gain: f64,
    pub cwnd_gain: f64,
    pub pacing_rate: f64,
    pub cycle_index: u32,
    pub cycle_start: u32,
    pub round_count: i32,
    pub full_bw_count: i32,
    pub lt_rtt_cnt: i32,
    pub lt_bw: u64,
    pub lt_last_stamp: u64,
    pub previous_round_lost: u64,
    pub previous_sampling_delivered: u64,
    pub previous_sampling_lost: u64,
    pub loss_interval_start: u64,
    pub congestion_sequence: u64,
    pub cwin_before_suspension: u64,
    pub option_string: Option<String>,
    pub wifi_shadow_rtt: u64,
    pub quantum_ratio: f64,
    pub filled_pipe: bool,
    pub round_start: bool,
    pub rt_prop_expired: bool,
    pub probe_rtt_round_done: bool,
    pub idle_restart: bool,
    pub packet_conservation: bool,
    pub btl_bw_increased: bool,
    pub lt_use_bw: bool,
    pub lt_is_sampling: bool,
    pub last_loss_was_timeout: bool,
    pub cycle_on_loss: bool,
    pub is_suspended: bool,
    pub is_suspension_nearly_over: bool,
}

impl Default for Bbr1State {
    fn default() -> Self {
        Self {
            state: Bbr1AlgState::Startup,
            btl_bw: 0,
            next_round_delivered: 0,
            btl_bw_filter: [0; BBR1_BTL_BW_FILTER_LENGTH],
            full_bw: 0,
            rt_prop: 0,
            rt_prop_stamp: 0,
            cycle_stamp: 0,
            probe_rtt_done_stamp: 0,
            prior_cwnd: 0,
            prior_in_flight: 0,
            bytes_delivered: 0,
            send_quantum: 0,
            rtt_filter: MinMaxRtt::default(),
            target_cwnd: 0,
            pacing_gain: 0.0,
            cwnd_gain: 0.0,
            pacing_rate: 0.0,
            cycle_index: 0,
            cycle_start: 0,
            round_count: 0,
            full_bw_count: 0,
            lt_rtt_cnt: 0,
            lt_bw: 0,
            lt_last_stamp: 0,
            previous_round_lost: 0,
            previous_sampling_delivered: 0,
            previous_sampling_lost: 0,
            loss_interval_start: 0,
            congestion_sequence: 0,
            cwin_before_suspension: 0,
            option_string: None,
            wifi_shadow_rtt: 0,
            quantum_ratio: 0.0,
            filled_pipe: false,
            round_start: false,
            rt_prop_expired: false,
            probe_rtt_round_done: false,
            idle_restart: false,
            packet_conservation: false,
            btl_bw_increased: false,
            lt_use_bw: false,
            lt_is_sampling: false,
            last_loss_was_timeout: false,
            cycle_on_loss: false,
            is_suspended: false,
            is_suspension_nearly_over: false,
        }
    }
}

impl Bbr1State {
    /// C: `BBR1EnterStartup` (picoquic/bbr1.c:327)
    ///
    /// Enter the Startup state with the high-gain pacing and cwnd multipliers.
    pub fn enter_startup(&mut self) {
        self.state = Bbr1AlgState::Startup;
        self.pacing_gain = BBR1_HIGH_GAIN;
        self.cwnd_gain = BBR1_HIGH_GAIN;
    }

    /// C: `BBR1CheckFullPipe` (picoquic/bbr1.c:771)
    ///
    /// Detect when the pipe is full during Startup.  On each round start
    /// (unless app-limited or already detected), check whether `btl_bw`
    /// has grown by ≥ 25%.  After three consecutive rounds without
    /// sufficient growth, set `filled_pipe`.
    pub fn check_full_pipe(&mut self, rs_is_app_limited: bool) {
        if !self.filled_pipe && self.round_start && !rs_is_app_limited {
            if self.btl_bw as f64 >= self.full_bw as f64 * 1.25 {
                self.full_bw = self.btl_bw;
                self.full_bw_count = 0;
            } else {
                self.full_bw_count += 1;
                if self.full_bw_count >= 3 {
                    self.filled_pipe = true;
                }
            }
        }
    }

    /// C: `BBR1EnterProbeRTT` (picoquic/bbr1.c:885)
    ///
    /// Enter ProbeRTT state: coast at unity gains to drain the queue and
    /// obtain a fresh minimum-RTT sample.
    pub fn enter_probe_rtt(&mut self) {
        self.state = Bbr1AlgState::ProbeRtt;
        self.pacing_gain = 1.0;
        self.cwnd_gain = 1.0;
    }

    /// C: `BBR1AfterOneRoundtripInFastRecovery` (picoquic/bbr1.c:1107)
    ///
    /// After the first full round-trip in fast recovery, disable packet
    /// conservation so the congestion window can grow normally again.
    pub fn after_one_roundtrip_in_fast_recovery(&mut self) {
        self.packet_conservation = false;
    }

    /// C: `BBR1HandleRestartFromIdle` (picoquic/bbr1.c:1057)
    ///
    /// When the path restarts from an idle, app-limited state, remember the
    /// idle restart and neutralize pacing gain while in ProbeBW.
    pub fn handle_restart_from_idle(&mut self, bytes_in_transit: u64, is_app_limited: bool) {
        if bytes_in_transit == 0 && is_app_limited {
            self.idle_restart = true;
            if self.state == Bbr1AlgState::ProbeBw {
                self.set_pacing_rate_with_gain(1.0);
            }
        }
    }

    /// C: `BBR1OnTransmit` (picoquic/bbr1.c:1083-1086)
    ///
    /// Transmission-side hook; delegates to the idle-restart check exactly as
    /// the C body does.
    pub fn on_transmit(&mut self, bytes_in_transit: u64, is_app_limited: bool) {
        self.handle_restart_from_idle(bytes_in_transit, is_app_limited);
    }

    /// C: `BBR1OnAllPacketsLost` (picoquic/bbr1.c:1091)
    ///
    /// Save the current congestion window, then reduce the path cwnd to one
    /// MTU after all packets in flight are declared lost.
    pub fn on_all_packets_lost(&mut self, path_x: &mut Path) {
        self.prior_cwnd = self.save_cwnd(path_x);
        path_x.cwin = path_x.send_mtu as u64;
    }

    /// C: `BBR1OnEnterFastRecovery` (picoquic/bbr1.c:1097)
    ///
    /// Enter packet-conservation recovery with cwnd equal to bytes already in
    /// transit plus at least one MTU of newly delivered data.
    pub fn on_enter_fast_recovery(
        &mut self,
        path_x: &mut Path,
        bytes_in_transit: u64,
        mut bytes_delivered: u64,
    ) {
        if bytes_delivered < path_x.send_mtu as u64 {
            bytes_delivered = path_x.send_mtu as u64;
        }
        self.prior_cwnd = self.save_cwnd(path_x);
        path_x.cwin = bytes_in_transit + bytes_delivered;
        self.packet_conservation = true;
    }

    /// C: `BBR1GetBtlBW` (picoquic/bbr1.c:304)
    ///
    /// Return the effective bottleneck bandwidth: the long-term estimate when
    /// `lt_use_bw` is set, otherwise the filter-max `btl_bw`.
    fn get_btl_bw(&self) -> u64 {
        if self.lt_use_bw {
            self.lt_bw
        } else {
            self.btl_bw
        }
    }

    /// C: `BBR1EnterStartupLongRTT` (picoquic/bbr1.c:309)
    ///
    /// Switch into `StartupLongRtt` state and ensure `cwin` is at least
    /// `CWIN_INITIAL` scaled by the path RTT (capped at the satellite ceiling).
    pub fn enter_startup_long_rtt(&mut self, path_x: &mut Path) {
        let mut cwnd = CWIN_INITIAL;
        self.state = Bbr1AlgState::StartupLongRtt;
        if path_x.rtt_min > TARGET_RENO_RTT {
            let rtt_cap = if path_x.rtt_min > TARGET_SATELLITE_RTT {
                TARGET_SATELLITE_RTT
            } else {
                path_x.rtt_min
            };
            cwnd = (cwnd as f64 * rtt_cap.ticks() as f64 / TARGET_RENO_RTT.ticks() as f64) as u64;
        }
        if cwnd > path_x.cwin {
            path_x.cwin = cwnd;
        }
    }

    /// C: `BBR1RestoreCwnd` (picoquic/bbr1.c:918)
    ///
    /// Restore `cwin` to `prior_cwnd` if the current window has shrunk below it.
    pub fn restore_cwnd(&self, path_x: &mut Path) {
        if path_x.cwin < self.prior_cwnd {
            path_x.cwin = self.prior_cwnd;
        }
    }

    /// C: `BBR1ModulateCwndForRecovery` (picoquic/bbr1.c:996)
    ///
    /// Adjust `cwin` for the recovery phase: subtract lost bytes (floored at
    /// one MTU), then enforce packet-conservation if active.
    pub fn modulate_cwnd_for_recovery(
        &mut self,
        path_x: &mut Path,
        bytes_in_transit: u64,
        bytes_lost: u64,
        bytes_delivered: u64,
    ) {
        if bytes_lost > 0 {
            if path_x.cwin > bytes_lost {
                path_x.cwin -= bytes_lost;
            } else {
                path_x.cwin = path_x.send_mtu as u64;
            }
        }
        if self.packet_conservation && path_x.cwin < bytes_in_transit + bytes_delivered {
            path_x.cwin = bytes_in_transit + bytes_delivered;
        }
    }

    /// C: `BBR1ModulateCwndForProbeRTT` (picoquic/bbr1.c:1015)
    ///
    /// While in `ProbeRtt`, cap `cwin` at `BBR1_MIN_PIPE_CWND` (4 × MTU) so
    /// the queue drains and a fresh minimum-RTT sample can be obtained.
    pub fn modulate_cwnd_for_probe_rtt(&self, path_x: &mut Path) {
        if self.state == Bbr1AlgState::ProbeRtt {
            let min_pipe = path_x.send_mtu as u64 * 4;
            if path_x.cwin > min_pipe {
                path_x.cwin = min_pipe;
            }
        }
    }

    /// C: `BBR1SetSendQuantum` (picoquic/bbr1.c:334)
    ///
    /// Choose the send quantum (burst size) based on current pacing rate.
    pub fn set_send_quantum(&mut self, path_x: &Path) {
        if self.pacing_rate < BBR1_PACING_RATE_LOW {
            self.send_quantum = path_x.send_mtu as u64;
        } else if self.pacing_rate < BBR1_PACING_RATE_MEDIUM {
            self.send_quantum = 2 * path_x.send_mtu as u64;
        } else {
            self.send_quantum = (self.pacing_rate * self.quantum_ratio) as u64;
            if self.send_quantum > 0x10000 {
                self.send_quantum = 0x10000;
            }
        }
    }

    /// C: `BBR1UpdateRTprop` (picoquic/bbr1.c:679)
    ///
    /// Update the minimum RTT propagation estimate and its timestamp.
    pub fn update_rt_prop(&mut self, rtt_sample: u64, current_time: u64) {
        self.rt_prop_expired = current_time > self.rt_prop_stamp + BBR1_PROBE_RTT_INTERVAL
            && current_time > self.rt_prop_stamp + 20 * self.rt_prop;
        if rtt_sample <= self.rt_prop || self.rt_prop_expired {
            self.rt_prop = rtt_sample;
            self.rt_prop_stamp = current_time;
        } else {
            let delta = rtt_sample - self.rt_prop;
            if 20 * delta < self.rt_prop {
                self.rt_prop_stamp = current_time;
            }
        }
    }

    /// C: `BBR1ltbwResetInterval` (picoquic/bbr1.c:494)
    ///
    /// Reset the long-term bandwidth sampling interval to the current time and
    /// delivery/loss counters.
    pub fn ltbw_reset_interval(&mut self, path_x: &Path, current_time: u64) {
        self.lt_last_stamp = current_time;
        self.previous_sampling_delivered = path_x.delivered;
        self.previous_sampling_lost = path_x.total_bytes_lost;
        self.previous_round_lost = path_x.total_bytes_lost;
        self.lt_rtt_cnt = 0;
    }

    /// C: `BBR1SetMinimalGain` (picoquic/bbr1.c:715)
    ///
    /// Raise `pacing_gain` when the BDP is so small that 4 MTUs exceed the
    /// estimated in-flight window, ensuring we send at least that burst.
    pub fn set_minimal_gain(&mut self) {
        if self.pacing_gain > 1.0 && self.rt_prop > 0 {
            let target_cwin = bytes_from_rate(self.rt_prop, self.btl_bw);
            let min_cwin = 4 * MAX_PACKET_SIZE as u64;
            if target_cwin < min_cwin {
                let d_gain = min_cwin as f64 / target_cwin as f64;
                if d_gain > self.pacing_gain {
                    self.pacing_gain = d_gain;
                }
            }
        }
    }

    /// C: `InLossRecovery1` (picoquic/bbr1.c:902)
    fn in_loss_recovery(&self) -> bool {
        self.packet_conservation
    }

    /// C: `picoquic_bbr1_observe` (picoquic/bbr1.c:1323)
    ///
    /// Return the current algorithm-state tag and bottleneck-bandwidth estimate
    /// so the surrounding scheduler can inspect the congestion state.
    pub fn observe(&self) -> (u64, u64) {
        (self.state as u64, self.btl_bw)
    }

    /// C: `picoquic_bbr1_set_options` (picoquic/bbr1.c:379)
    ///
    /// Parse `option_string` and apply recognised keys:
    /// `T<digits>` → `wifi_shadow_rtt` (microseconds);
    /// `Q<digits>[.<digits>]` → `quantum_ratio` (decimal fraction).
    /// Unknown characters and `:` separators are silently skipped.
    #[allow(dead_code)]
    fn set_options(&mut self) {
        let opt = match &self.option_string {
            Some(s) => s.clone(),
            None => return,
        };
        let mut chars = opt.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                'T' => {
                    let mut u: u64 = 0;
                    while let Some(&d) = chars.peek() {
                        if d.is_ascii_digit() {
                            u = u * 10 + (d as u64 - '0' as u64);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    self.wifi_shadow_rtt = u;
                }
                'Q' => {
                    // The C outer `while` is an `if` — executes at most once.
                    if chars.peek().is_none() {
                        continue;
                    }
                    let mut d: f64 = 0.0;
                    let mut div: f64 = 1.0;
                    let mut dotted = false;
                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_digit() {
                            let digit = ch as u64 - '0' as u64;
                            if !dotted {
                                d = d * 10.0 + digit as f64;
                            } else {
                                div /= 10.0;
                                d += div * digit as f64;
                            }
                            chars.next();
                        } else if ch == '.' {
                            if dotted {
                                break;
                            } else {
                                dotted = true;
                                chars.next();
                            }
                        } else {
                            break;
                        }
                    }
                    self.quantum_ratio = d;
                    // C falls through to ':' (ignore) here; no extra action needed.
                }
                _ => { /* ignore separator or unknown key */ }
            }
        }
    }

    /// C: `picoquic_bbr1_suspension_almost_over` (picoquic/bbr1.c:1162)
    ///
    /// After a spurious-repeat notification, mark the suspension as nearly
    /// over so `suspension_exit` can restore the saved window on the next
    /// acknowledgement.
    pub fn suspension_almost_over(&mut self, lost_packet_number: u64) {
        if self.is_suspended
            && self.cwin_before_suspension > 0
            && !self.is_suspension_nearly_over
            && self.congestion_sequence >= lost_packet_number
        {
            self.is_suspension_nearly_over = true;
        }
    }

    /// C: `BBR1AdvanceCyclePhase` (picoquic/bbr1.c:731)
    ///
    /// Advance the probe-BW gain cycle by one slot.  When the index wraps
    /// past the end, the start position is adjusted up (if bandwidth
    /// increased this round) or down (if it did not), providing adaptive
    /// probing density.  `pacing_gain` is read from the gain table and the
    /// minimal-gain floor is re-applied.
    pub fn advance_cycle_phase(&mut self, current_time: u64) {
        self.cycle_on_loss = false;
        self.cycle_stamp = current_time;
        self.cycle_index += 1;
        if self.cycle_index >= BBR1_GAIN_CYCLE_LEN as u32 {
            let mut start = self.cycle_start;
            if self.btl_bw_increased {
                self.btl_bw_increased = false;
                start += 1;
                if start > BBR1_GAIN_CYCLE_MAX_START {
                    start = BBR1_GAIN_CYCLE_MAX_START;
                }
            } else {
                start = start.saturating_sub(1);
            }
            self.cycle_index = start;
            self.cycle_start = start;
        }
        self.pacing_gain = BBR1_PACING_GAIN_CYCLE[self.cycle_index as usize];
        self.set_minimal_gain();
    }

    /// C: `BBR1SaveCwnd` (picoquic/bbr1.c:907)
    ///
    /// Return the cwnd value to save before entering a drain, probe-RTT, or
    /// recovery state.  If we are already in loss recovery or probe-BW and
    /// `prior_cwnd` is larger than the current window, `prior_cwnd` is the
    /// better snapshot to preserve.
    pub fn save_cwnd(&self, path_x: &Path) -> u64 {
        if (self.in_loss_recovery() || self.state == Bbr1AlgState::ProbeBw)
            && path_x.cwin < self.prior_cwnd
        {
            self.prior_cwnd
        } else {
            path_x.cwin
        }
    }

    /// C: `BBR1ltbwResetSampling` (picoquic/bbr1.c:503)
    ///
    /// Discard all long-term bandwidth sampling state and start fresh.
    fn ltbw_reset_sampling(&mut self, path_x: &Path, current_time: u64) {
        self.lt_bw = 0;
        self.lt_use_bw = false;
        self.lt_is_sampling = false;
        self.ltbw_reset_interval(path_x, current_time);
    }

    /// C: `BBR1ltbwIntervalDone` (picoquic/bbr1.c:511)
    ///
    /// Close one long-term sampling interval at bandwidth `bw`.  If this is
    /// the second consecutive interval and the two rates agree closely,
    /// enable the long-term estimate; otherwise record `bw` and reset the
    /// interval counter.
    fn ltbw_interval_done(&mut self, path_x: &Path, bw: u64, current_time: u64) {
        if self.lt_bw > 0 {
            let diff = bw.abs_diff(self.lt_bw);
            if diff * BBR1_LT_BW_RATIO_INVERSE < self.lt_bw || diff < BBR1_LT_BW_BYTES_PER_SEC_DIFF
            {
                self.lt_bw = (self.lt_bw + bw) / 2;
                self.lt_use_bw = true;
                self.pacing_gain = 1.0;
                self.lt_rtt_cnt = 0;
                return;
            }
        }
        self.lt_bw = bw;
        self.ltbw_reset_interval(path_x, current_time);
    }

    /// C: `BBR1ResetProbeBwMode` (picoquic/bbr1.c:764)
    ///
    /// Reset probe-BW to cycle index 2 and immediately advance one phase.
    fn reset_probe_bw_mode(&mut self, current_time: u64) {
        self.state = Bbr1AlgState::ProbeBw;
        self.cycle_index = 2;
        self.advance_cycle_phase(current_time);
    }

    /// C: `BBR1ltbwSampling` (picoquic/bbr1.c:530)
    ///
    /// Drive the long-term bandwidth sampler.  Detects pacing-limited
    /// intervals where loss rate exceeds the target ratio, computes a
    /// sustained bandwidth, and feeds the result to `ltbw_interval_done`.
    pub fn ltbw_sampling(&mut self, path_x: &Path, current_time: u64) {
        let losses = path_x
            .total_bytes_lost
            .saturating_sub(self.previous_round_lost);

        if self.lt_use_bw && self.state == Bbr1AlgState::ProbeBw && self.round_start {
            self.lt_rtt_cnt += 1;
            if self.lt_rtt_cnt > BBR1_LT_BW_MAX_RTTS {
                self.ltbw_reset_sampling(path_x, current_time);
                self.reset_probe_bw_mode(current_time);
                return;
            }
        }

        if !self.lt_is_sampling {
            if losses == 0 {
                return;
            }
            self.ltbw_reset_sampling(path_x, current_time);
            self.lt_is_sampling = true;
        }

        if path_x.last_bw_estimate_path_limited {
            self.ltbw_reset_sampling(path_x, current_time);
            return;
        }

        if !self.round_start {
            return;
        }

        self.lt_rtt_cnt += 1;
        self.previous_round_lost = path_x.total_bytes_lost;

        if self.lt_rtt_cnt < BBR1_LT_BW_INTERVAL_MIN_RTT {
            return;
        }
        if self.lt_rtt_cnt > BBR1_LT_BW_INTERVAL_MAX_RTT {
            self.ltbw_reset_sampling(path_x, current_time);
            return;
        }

        if losses == 0 {
            return;
        }

        if path_x.delivered <= self.previous_sampling_delivered {
            return;
        }

        let interval_losses = path_x
            .total_bytes_lost
            .saturating_sub(self.previous_sampling_lost);
        let delivered = path_x.delivered - self.previous_sampling_delivered;

        if interval_losses * BBR1_LT_BW_RATIO_SCALE < BBR1_LT_BW_RATIO_SCALED_TARGET * delivered {
            return;
        }

        let interval_microsec = current_time - self.lt_last_stamp;
        if interval_microsec < 1000 {
            return;
        }

        let bw = rate_from_bytes(delivered, interval_microsec);
        self.ltbw_interval_done(path_x, bw, current_time);
    }

    /// C: `BBR1EnterProbeBW` (picoquic/bbr1.c:787)
    ///
    /// Enter probe-BW state.  The initial cycle-start index is chosen from
    /// the measured propagation delay so that high-latency paths start with
    /// more steady-state phases before probing.
    pub fn enter_probe_bw(&mut self, path_x: &Path, current_time: u64) {
        self.state = Bbr1AlgState::ProbeBw;
        self.pacing_gain = 1.0;
        self.cwnd_gain = 2.0;

        let start = if self.rt_prop > TARGET_RENO_RTT.ticks() {
            let ref_rt = if self.rt_prop > TARGET_SATELLITE_RTT.ticks() {
                TARGET_SATELLITE_RTT.ticks()
            } else {
                self.rt_prop
            };
            ((ref_rt / TARGET_RENO_RTT.ticks()) as u32).min(BBR1_GAIN_CYCLE_MAX_START)
        } else {
            2
        };

        self.cycle_index = start;
        self.cycle_start = start;
        self.btl_bw_increased = true;
        self.advance_cycle_phase(current_time);
        self.ltbw_sampling(path_x, current_time);
    }

    /// C: `BBR1ExitProbeRTT` (picoquic/bbr1.c:892)
    ///
    /// Leave ProbeRTT: if the pipe is full transition to probe-BW, otherwise
    /// return to Startup to keep growing the bandwidth estimate.
    pub fn exit_probe_rtt(&mut self, path_x: &Path, current_time: u64) {
        if self.filled_pipe {
            self.enter_probe_bw(path_x, current_time);
        } else {
            self.enter_startup();
        }
    }

    /// C: `BBR1HandleProbeRTT` (picoquic/bbr1.c:926)
    ///
    /// While in ProbeRTT: drain the queue to `BBR1_MIN_PIPE_CWND`, then wait
    /// for one full round-trip and the minimum probe-RTT duration before
    /// restoring the saved cwnd and exiting the state.
    pub fn handle_probe_rtt(
        &mut self,
        path_x: &mut Path,
        bytes_in_transit: u64,
        current_time: u64,
    ) {
        let min_pipe_cwnd = path_x.send_mtu as u64 * 4;
        if self.probe_rtt_done_stamp == 0 && bytes_in_transit <= min_pipe_cwnd {
            self.probe_rtt_done_stamp = current_time + BBR1_PROBE_RTT_DURATION;
            self.probe_rtt_round_done = false;
            self.next_round_delivered = path_x.delivered;
        } else if self.probe_rtt_done_stamp != 0 {
            if self.round_start {
                self.probe_rtt_round_done = true;
            }
            if self.probe_rtt_round_done && current_time > self.probe_rtt_done_stamp {
                self.rt_prop_stamp = current_time;
                self.restore_cwnd(path_x);
                self.exit_probe_rtt(path_x, current_time);
            }
        }
    }

    /// C: `BBR1CheckProbeRTT` (picoquic/bbr1.c:955)
    ///
    /// Periodically enter ProbeRTT to obtain a fresh minimum-RTT sample when
    /// the current estimate has expired and the connection is not idle.
    pub fn check_probe_rtt(&mut self, path_x: &mut Path, bytes_in_transit: u64, current_time: u64) {
        if self.state != Bbr1AlgState::ProbeRtt && self.rt_prop_expired && !self.idle_restart {
            self.enter_probe_rtt();
            let prior = self.save_cwnd(path_x);
            self.prior_cwnd = prior;
            self.probe_rtt_done_stamp = 0;
        }

        if self.state == Bbr1AlgState::ProbeRtt {
            self.handle_probe_rtt(path_x, bytes_in_transit, current_time);
            self.idle_restart = false;
        }
    }

    /// C: `BBR1Inflight` (picoquic/bbr1.c:350)
    ///
    /// Compute the estimated in-flight window for `gain`.  Uses the effective
    /// bottleneck bandwidth and the measured propagation delay (or the Wi-Fi
    /// shadow RTT when it is larger) to estimate BDP, then adds 3 × send_quantum
    /// headroom.  Returns CWIN_INITIAL until rt_prop is first measured.
    pub fn inflight(&self, gain: f64) -> u64 {
        let mut cwnd = CWIN_INITIAL;
        if self.rt_prop != u64::MAX {
            let rt_target = self.rt_prop.max(self.wifi_shadow_rtt);
            let estimated_bdp = self.get_btl_bw() as f64 * rt_target as f64 / 1_000_000.0;
            cwnd = (gain * estimated_bdp) as u64 + 3 * self.send_quantum;
        }
        cwnd
    }

    /// C: `BBR1UpdateTargetCwnd` (picoquic/bbr1.c:367)
    fn update_target_cwnd(&mut self) {
        let gain = self.cwnd_gain;
        self.target_cwnd = self.inflight(gain);
    }

    /// C: `BBR1EnterDrain` (picoquic/bbr1.c:814)
    fn enter_drain(&mut self, path_x: &mut Path, current_time: u64) {
        path_x.is_ssthresh_initialized = true;
        self.state = Bbr1AlgState::Drain;
        self.pacing_gain = 1.0 / BBR1_HIGH_GAIN;
        self.cwnd_gain = BBR1_HIGH_GAIN;
        self.ltbw_sampling(path_x, current_time);
    }

    /// C: `BBR1ExitStartupSeedBDP` (picoquic/bbr1.c:860)
    ///
    /// Bootstrap bandwidth and RTT estimates from a pre-seeded BDP (e.g. from
    /// a prior connection's cached metrics), set the path's initial cwnd, then
    /// immediately enter Drain and, if the queue is already small, ProbeBW.
    pub fn exit_startup_seed_bdp(&mut self, path_x: &mut Path, bdp: u64, current_time: u64) {
        let bandwidth_estimate = rate_from_bytes(bdp, path_x.rtt_min.ticks());
        path_x.cwin = bdp;
        if bandwidth_estimate > self.btl_bw_filter[0] {
            self.btl_bw_filter[0] = bandwidth_estimate;
            if bandwidth_estimate > self.btl_bw {
                self.btl_bw = bandwidth_estimate;
                self.btl_bw_increased = true;
            }
        }
        self.update_rt_prop(path_x.rtt_min.ticks(), current_time);
        self.enter_drain(path_x, current_time);
        let bytes_in_transit = path_x.bytes_in_transit;
        if bytes_in_transit <= self.inflight(1.0) {
            self.enter_probe_bw(path_x, current_time);
        }
    }

    /// C: `BBR1SetCwnd` (picoquic/bbr1.c:1025)
    ///
    /// Update the congestion window: refresh the target, apply recovery
    /// modulation, then grow toward the target (or unconstrained in Startup).
    /// Always enforce a 4 × MTU floor and ProbeRTT capping.
    pub fn set_cwnd(
        &mut self,
        path_x: &mut Path,
        bytes_in_transit: u64,
        bytes_lost: u64,
        bytes_delivered: u64,
    ) {
        self.update_target_cwnd();
        self.modulate_cwnd_for_recovery(path_x, bytes_in_transit, bytes_lost, bytes_delivered);
        if !self.packet_conservation {
            if self.filled_pipe {
                path_x.cwin += bytes_delivered;
                if path_x.cwin > self.target_cwnd {
                    path_x.cwin = self.target_cwnd;
                }
            } else if path_x.cwin < self.target_cwnd || path_x.delivered < CWIN_INITIAL {
                path_x.cwin += bytes_delivered;
            }
            let min_pipe = path_x.send_mtu as u64 * 4;
            if path_x.cwin < min_pipe {
                path_x.cwin = min_pipe;
            }
        }
        self.modulate_cwnd_for_probe_rtt(path_x);
    }

    /// C: `BBR1ExitFastRecovery` (picoquic/bbr1.c:1112)
    ///
    /// End fast-recovery: clear packet conservation and restore the saved
    /// congestion window.
    pub fn exit_fast_recovery(&mut self, path_x: &mut Path) {
        self.packet_conservation = false;
        self.restore_cwnd(path_x);
    }

    /// C: `BBR1SetPacingRateWithGain` (picoquic/bbr1.c:982)
    ///
    /// Set `pacing_rate` to `pacing_gain × btl_bw`.  Once the pipe is filled
    /// the rate can only increase; before that, it can also decrease (startup
    /// ramp-up tracks the growing bandwidth estimate).
    pub fn set_pacing_rate_with_gain(&mut self, pacing_gain: f64) {
        let rate = pacing_gain * self.get_btl_bw() as f64;
        if self.filled_pipe || rate > self.pacing_rate {
            self.pacing_rate = rate;
        }
    }

    /// C: `BBR1SetPacingRate` (picoquic/bbr1.c:991)
    fn set_pacing_rate(&mut self) {
        let gain = self.pacing_gain;
        self.set_pacing_rate_with_gain(gain);
    }

    /// C: `BBR1IsNextCyclePhase` (picoquic/bbr1.c:698)
    ///
    /// Return true when it is time to advance the probe-BW gain cycle.  The
    /// cycle must have run for at least one `rt_prop` (or a loss occurred on
    /// the current cycle).  During the gain-up phase the cycle also requires
    /// either a loss event or that in-flight bytes have reached the target
    /// window; during gain-down the queue must have drained.
    fn is_next_cycle_phase(
        &self,
        prior_in_flight: u64,
        packets_lost: u64,
        current_time: u64,
    ) -> bool {
        let is_full_length = self.cycle_on_loss || (current_time - self.cycle_stamp) > self.rt_prop;
        if self.pacing_gain != 1.0 {
            if self.pacing_gain > 1.0 {
                is_full_length
                    && (packets_lost > 0 || prior_in_flight >= self.inflight(self.pacing_gain))
            } else {
                is_full_length && prior_in_flight <= self.inflight(1.0)
            }
        } else {
            is_full_length
        }
    }

    /// C: `BBR1CheckCyclePhase` (picoquic/bbr1.c:756)
    ///
    /// Advance the probe-BW gain cycle when the current phase has run long
    /// enough (delegating the check to `is_next_cycle_phase`).
    fn check_cycle_phase(&mut self, packets_lost: u64, current_time: u64) {
        if self.state == Bbr1AlgState::ProbeBw {
            let prior = self.prior_in_flight;
            if self.is_next_cycle_phase(prior, packets_lost, current_time) {
                self.advance_cycle_phase(current_time);
            }
        }
    }

    /// C: `BBR1CheckDrain` (picoquic/bbr1.c:824)
    ///
    /// Transition from Startup → Drain when the pipe is full, then from
    /// Drain → ProbeBW once the queue has drained (bytes in transit ≤
    /// estimated BDP).
    fn check_drain(&mut self, path_x: &mut Path, bytes_in_transit: u64, current_time: u64) {
        if self.state == Bbr1AlgState::Startup && self.filled_pipe {
            self.enter_drain(path_x, current_time);
        }
        if self.state == Bbr1AlgState::Drain {
            let target = self.inflight(1.0);
            if bytes_in_transit <= target {
                self.enter_probe_bw(path_x, current_time);
            }
        }
    }

    /// C: `BBR1UpdateBtlBw` (picoquic/bbr1.c:616)
    ///
    /// Update the bottleneck-bandwidth filter each round trip.  In Startup,
    /// the estimate is floored at half the peak observation so a single
    /// low sample cannot collapse the estimate.  A minimum bandwidth derived
    /// from `CWIN_MINIMUM / rt_prop` prevents the estimate from underflowing
    /// when losses keep occurring on a near-idle path.
    ///
    /// On each round start the filter array is shifted right (oldest entry
    /// dropped) and the round maximum is placed in slot 0; between round
    /// starts, slot 0 is updated in place when the current observation
    /// exceeds the stored value.
    fn update_btl_bw(&mut self, path_x: &Path, current_time: u64) {
        let mut bandwidth_estimate = path_x.bandwidth_estimate;

        if self.state == Bbr1AlgState::Startup
            && bandwidth_estimate < path_x.peak_bandwidth_estimate / 2
        {
            bandwidth_estimate = path_x.peak_bandwidth_estimate / 2;
        }

        if self.rt_prop > 0 {
            let min_bandwidth = rate_from_bytes(CWIN_MINIMUM, self.rt_prop);
            if bandwidth_estimate < min_bandwidth {
                bandwidth_estimate = min_bandwidth;
            }
        }

        if path_x.delivered_last_packet >= self.next_round_delivered {
            self.next_round_delivered = path_x.delivered;
            self.round_count += 1;
            self.round_start = true;
        } else {
            self.round_start = false;
        }

        self.ltbw_sampling(path_x, current_time);

        if self.round_start {
            if bandwidth_estimate > self.btl_bw || !path_x.last_bw_estimate_path_limited {
                // Shift filter right, dropping the oldest slot, tracking the max.
                self.btl_bw = 0;
                for i in (0..BBR1_BTL_BW_FILTER_LENGTH - 1).rev() {
                    let b = self.btl_bw_filter[i];
                    self.btl_bw_filter[i + 1] = b;
                    if b > self.btl_bw {
                        self.btl_bw = b;
                    }
                }
                // btl_bw_filter[0] still holds the previous round's value here.
                self.btl_bw_increased |= bandwidth_estimate > self.btl_bw_filter[0];
                self.btl_bw_filter[0] = bandwidth_estimate;
                if bandwidth_estimate > self.btl_bw {
                    self.btl_bw = bandwidth_estimate;
                }
            } else {
                self.btl_bw_increased = false;
            }
        } else if bandwidth_estimate > self.btl_bw_filter[0] {
            self.btl_bw_filter[0] = bandwidth_estimate;
            if bandwidth_estimate > self.btl_bw {
                self.btl_bw = bandwidth_estimate;
                self.btl_bw_increased = true;
            }
        }
    }

    /// C: `BBR1UpdateModelAndState` (picoquic/bbr1.c:971)
    ///
    /// Per-ACK model update: refresh the bottleneck bandwidth filter, advance
    /// the probe-BW cycle if warranted, detect pipe-full and state transitions
    /// (Startup → Drain → ProbeBW), update the propagation-delay estimate, and
    /// enter/handle ProbeRTT when the RTT estimate has expired.
    pub fn update_model_and_state(
        &mut self,
        path_x: &mut Path,
        rtt_sample: u64,
        bytes_in_transit: u64,
        packets_lost: u64,
        current_time: u64,
    ) {
        self.update_btl_bw(path_x, current_time);
        self.check_cycle_phase(packets_lost, current_time);
        let app_limited = path_x.last_bw_estimate_path_limited;
        self.check_full_pipe(app_limited);
        self.check_drain(path_x, bytes_in_transit, current_time);
        self.update_rt_prop(rtt_sample, current_time);
        self.check_probe_rtt(path_x, bytes_in_transit, current_time);
    }

    /// C: `BBR1UpdateControlParameters` (picoquic/bbr1.c:1050)
    ///
    /// Per-ACK control-parameter update: refresh the pacing rate, recompute
    /// the send quantum, and update the congestion window.
    pub fn update_control_parameters(
        &mut self,
        path_x: &mut Path,
        bytes_in_transit: u64,
        packets_lost: u64,
        bytes_delivered: u64,
    ) {
        self.set_pacing_rate();
        self.set_send_quantum(path_x);
        self.set_cwnd(path_x, bytes_in_transit, packets_lost, bytes_delivered);
    }

    /// C: `BBR1ExitStartupLongRtt` (picoquic/bbr1.c:835)
    ///
    /// Declare the long-RTT startup over: reset the round filter, mark the
    /// pipe full, optionally correct `rt_prop` from the filter maximum if the
    /// stored value looks pathological, then enter Drain (and ProbeBW when
    /// the queue has already drained).
    fn exit_startup_long_rtt(&mut self, path_x: &mut Path, current_time: u64) {
        self.next_round_delivered = path_x.delivered;
        self.round_count += 1;
        self.round_start = true;
        self.full_bw = self.btl_bw;
        self.full_bw_count = 3;
        self.filled_pipe = true;
        if (self.rtt_filter.is_init || self.rtt_filter.sample_current > 0)
            && self.rt_prop > 30_000_000
            && self.rtt_filter.sample_max.ticks() < self.rt_prop
        {
            self.rt_prop = self.rtt_filter.sample_max.ticks();
            self.rt_prop_stamp = current_time;
        }
        self.enter_drain(path_x, current_time);
        let bytes_in_transit = path_x.bytes_in_transit;
        if bytes_in_transit <= self.inflight(1.0) {
            self.enter_probe_bw(path_x, current_time);
        }
    }

    /// C: `picoquic_bbr1_suspension_exit` (picoquic/bbr1.c:1174)
    ///
    /// When both `is_suspended` and `is_suspension_nearly_over` are set,
    /// restore the pre-suspension cwnd and re-install the pacing rate.
    /// Always clears both suspension flags.
    fn suspension_exit(&mut self, path_x: &mut Path) {
        if self.is_suspended && self.is_suspension_nearly_over {
            path_x.cwin = self.cwin_before_suspension;
            path_x.update_pacing_rate(self.pacing_rate, self.send_quantum);
        }
        self.is_suspended = false;
        self.is_suspension_nearly_over = false;
    }

    /// C: `BBR1UpdateOnACK` (picoquic/bbr1.c:1074)
    ///
    /// Per-ACK combined update: model-and-state first, then control parameters.
    fn update_on_ack(
        &mut self,
        path_x: &mut Path,
        rtt_sample: u64,
        bytes_in_transit: u64,
        packets_lost: u64,
        bytes_delivered: u64,
        current_time: u64,
    ) {
        self.update_model_and_state(
            path_x,
            rtt_sample,
            bytes_in_transit,
            packets_lost,
            current_time,
        );
        self.update_control_parameters(path_x, bytes_in_transit, packets_lost, bytes_delivered);
    }

    /// C: `picoquic_bbr1_reset` (picoquic/bbr1.c:449)
    ///
    /// Zero all algorithm state (`memset` in C), reset `cwin` to the initial
    /// window, set `rt_prop = UINT64_MAX`, apply option defaults, then run the
    /// standard startup initialisation sequence.
    pub fn reset(&mut self, path_x: &mut Path, current_time: u64) {
        // C: memset(bbr1_state, 0, sizeof(...)) — option_string pointer is zeroed.
        *self = Self {
            state: Bbr1AlgState::Startup,
            btl_bw: 0,
            next_round_delivered: 0,
            btl_bw_filter: [0; BBR1_BTL_BW_FILTER_LENGTH],
            full_bw: 0,
            rt_prop: 0,
            rt_prop_stamp: 0,
            cycle_stamp: 0,
            probe_rtt_done_stamp: 0,
            prior_cwnd: 0,
            prior_in_flight: 0,
            bytes_delivered: 0,
            send_quantum: 0,
            rtt_filter: MinMaxRtt::default(),
            target_cwnd: 0,
            pacing_gain: 0.0,
            cwnd_gain: 0.0,
            pacing_rate: 0.0,
            cycle_index: 0,
            cycle_start: 0,
            round_count: 0,
            full_bw_count: 0,
            lt_rtt_cnt: 0,
            lt_bw: 0,
            lt_last_stamp: 0,
            previous_round_lost: 0,
            previous_sampling_delivered: 0,
            previous_sampling_lost: 0,
            loss_interval_start: 0,
            congestion_sequence: 0,
            cwin_before_suspension: 0,
            option_string: None,
            wifi_shadow_rtt: 0,
            quantum_ratio: 0.0,
            filled_pipe: false,
            round_start: false,
            rt_prop_expired: false,
            probe_rtt_round_done: false,
            idle_restart: false,
            packet_conservation: false,
            btl_bw_increased: false,
            lt_use_bw: false,
            lt_is_sampling: false,
            last_loss_was_timeout: false,
            cycle_on_loss: false,
            is_suspended: false,
            is_suspension_nearly_over: false,
        };
        path_x.cwin = CWIN_INITIAL;
        self.rt_prop = u64::MAX;
        self.set_options(); // no-op: option_string is None after zero-init
        if self.quantum_ratio == 0.0 {
            self.quantum_ratio = 0.001;
        }
        self.rt_prop_stamp = current_time;
        self.cycle_stamp = current_time;
        self.cycle_index = 0;
        self.cycle_start = 0;
        self.enter_startup();
        self.set_send_quantum(path_x);
        self.update_target_cwnd();
    }

    /// C: `picoquic_bbr1_notify_congestion` (picoquic/bbr1.c:1120)
    ///
    /// React to ECN, sustained loss, or timeout: filter repeated events, halve
    /// (or clamp) the congestion window, record the sequence number of the
    /// event, and drive the state machine into Drain (from Startup) or set the
    /// `cycle_on_loss` flag (from ProbeBW).
    pub fn notify_congestion(
        &mut self,
        cnx: &Connection,
        path_x: &mut Path,
        current_time: u64,
        is_timeout: bool,
    ) {
        // Filter repeated loss events within the same RTT or on the same cycle.
        if (self.cycle_on_loss
            || current_time < self.loss_interval_start + path_x.smoothed_rtt.ticks())
            && (!is_timeout || self.last_loss_was_timeout)
        {
            return;
        }
        if is_timeout || path_x.cwin < CWIN_MINIMUM {
            if !self.is_suspended {
                self.is_suspended = true;
                self.cwin_before_suspension = path_x.cwin;
            }
            path_x.cwin = CWIN_MINIMUM;
        } else {
            path_x.cwin /= 2;
        }
        self.loss_interval_start = current_time;
        self.last_loss_was_timeout = is_timeout;
        self.congestion_sequence = cnx.sequence_number(path_x);

        if self.state == Bbr1AlgState::StartupLongRtt {
            self.exit_startup_long_rtt(path_x, current_time);
        } else if self.state == Bbr1AlgState::Startup {
            self.filled_pipe = true;
            self.enter_drain(path_x, current_time);
        } else {
            self.cycle_on_loss = true;
        }
    }

    /// C: `picoquic_bbr1_notify` (picoquic/bbr1.c:1195)
    ///
    /// Dispatch a generic congestion-control notification to the appropriate
    /// BBR1 handler.  This is the main entry point called by the QUIC sender
    /// on every relevant event (ACK, loss, ECN, reset, …).
    pub fn notify(
        &mut self,
        cnx: &Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: u64,
    ) {
        path_x.is_cc_data_updated = true;

        match notification {
            CongestionNotification::EcnEc
                if ack_state.lost_packet_number >= self.congestion_sequence =>
            {
                self.notify_congestion(cnx, path_x, current_time, false);
            }
            CongestionNotification::Repeat | CongestionNotification::Timeout
                if ack_state.lost_packet_number >= self.congestion_sequence
                    && self.rtt_filter.hystart_loss_test(
                        notification,
                        ack_state.lost_packet_number,
                        0.20,
                    ) =>
            {
                let is_timeout = notification == CongestionNotification::Timeout;
                self.notify_congestion(cnx, path_x, current_time, is_timeout);
            }
            CongestionNotification::SpuriousRepeat if self.is_suspended => {
                self.suspension_almost_over(ack_state.lost_packet_number);
            }
            CongestionNotification::Acknowledgement => {
                if self.is_suspended {
                    self.suspension_exit(path_x);
                }
                self.bytes_delivered += ack_state.nb_bytes_acknowledged;

                if self.state == Bbr1AlgState::Startup
                    && path_x.rtt_min.ticks() > BBR1_HYSTART_THRESHOLD_RTT
                {
                    self.enter_startup_long_rtt(path_x);
                }

                if self.state == Bbr1AlgState::StartupLongRtt {
                    let rtt_meas = if cnx.is_time_stamp_enabled {
                        ack_state.one_way_delay
                    } else {
                        ack_state.rtt_measurement
                    };
                    let packet_time =
                        Instant::from_ticks(path_x.pacing.packet_time_microsec.ticks());
                    if self.rtt_filter.hystart_test(
                        rtt_meas,
                        packet_time,
                        Instant::from_ticks(current_time),
                        cnx.is_time_stamp_enabled,
                    ) {
                        self.exit_startup_long_rtt(path_x, current_time);
                    }
                }

                if self.state == Bbr1AlgState::StartupLongRtt {
                    self.update_btl_bw(path_x, current_time);
                    if ack_state.rtt_measurement.ticks() <= self.rt_prop {
                        self.rt_prop = ack_state.rtt_measurement.ticks();
                        self.rt_prop_stamp = current_time;
                    }
                    if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
                        let increase = path_x.slow_start_increase(self.bytes_delivered);
                        path_x.cwin += increase;
                    }
                    self.bytes_delivered = 0;

                    let max_win = bytes_from_rate(self.rt_prop, path_x.peak_bandwidth_estimate);
                    let min_win = max_win / 2;

                    if path_x.cwin < min_win {
                        path_x.cwin = min_win;
                    } else if path_x.smoothed_rtt > TARGET_RENO_RTT {
                        path_x.pacing.bandwidth_pause = 1;
                    }
                    path_x.update_pacing_data(1);
                } else {
                    let rtt_sample = ack_state.rtt_measurement.ticks();
                    let bytes_in_transit = path_x.bytes_in_transit;
                    let bytes_delivered = self.bytes_delivered;
                    self.update_on_ack(
                        path_x,
                        rtt_sample,
                        bytes_in_transit,
                        0,
                        bytes_delivered,
                        current_time,
                    );
                    self.prior_in_flight = path_x.bytes_in_transit;
                    self.bytes_delivered = 0;
                    if self.pacing_rate > 0.0 {
                        path_x.update_pacing_rate(self.pacing_rate, self.send_quantum);
                    }
                }
            }
            CongestionNotification::CwinBlocked => {}
            CongestionNotification::Reset => {
                self.reset(path_x, current_time);
            }
            CongestionNotification::SeedCwin => {
                if self.state == Bbr1AlgState::StartupLongRtt {
                    self.exit_startup_seed_bdp(
                        path_x,
                        ack_state.nb_bytes_acknowledged,
                        current_time,
                    );
                    path_x.update_pacing_data(1);
                } else if self.state == Bbr1AlgState::Startup {
                    // Estimate bandwidth from the seeded BDP; account for the
                    // div-by-two inside BBR1UpdateBtlBw with *2 (C comment: "Hack").
                    let smoothed_rtt_us = path_x.smoothed_rtt.ticks() as f64;
                    if smoothed_rtt_us > 0.0 {
                        let seed_bw =
                            ack_state.nb_bytes_acknowledged as f64 / smoothed_rtt_us * 1_000_000.0;
                        let bwe = (seed_bw as u64) * 2;
                        if path_x.bandwidth_estimate_max < bwe {
                            path_x.bandwidth_estimate_max = bwe;
                            self.update_btl_bw(path_x, current_time);
                            self.set_pacing_rate();
                            if self.pacing_rate > 0.0 {
                                path_x.update_pacing_rate(self.pacing_rate, self.send_quantum);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// C: `picoquic_bbr1_init` (picoquic/bbr1.c:469)
///
/// Allocate and initialise the per-path BBRv1 state.  The C implementation
/// stores a malloc'd `picoquic_bbr1_state_t` in `path_x->congestion_alg_state`;
/// Rust stores the typed state in `Box<dyn Any>`.
#[allow(dead_code)]
fn picoquic_bbr1_init(path_x: &mut Path, option_string: Option<&str>, current_time: u64) {
    let mut bbr1_state = Bbr1State {
        option_string: option_string.map(str::to_owned),
        ..Bbr1State::default()
    };
    bbr1_state.reset(path_x, current_time);
    path_x.congestion_alg_state = Some(Box::new(bbr1_state));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> Bbr1State {
        Bbr1State {
            state: Bbr1AlgState::default(),
            btl_bw: 0,
            next_round_delivered: 0,
            btl_bw_filter: [0; BBR1_BTL_BW_FILTER_LENGTH],
            full_bw: 0,
            rt_prop: 0,
            rt_prop_stamp: 0,
            cycle_stamp: 0,
            probe_rtt_done_stamp: 0,
            prior_cwnd: 0,
            prior_in_flight: 0,
            bytes_delivered: 0,
            send_quantum: 0,
            rtt_filter: MinMaxRtt::default(),
            target_cwnd: 0,
            pacing_gain: 1.0,
            cwnd_gain: 1.0,
            pacing_rate: 0.0,
            cycle_index: 0,
            cycle_start: 0,
            round_count: 0,
            full_bw_count: 0,
            lt_rtt_cnt: 0,
            lt_bw: 0,
            lt_last_stamp: 0,
            previous_round_lost: 0,
            previous_sampling_delivered: 0,
            previous_sampling_lost: 0,
            loss_interval_start: 0,
            congestion_sequence: 0,
            cwin_before_suspension: 0,
            option_string: None,
            wifi_shadow_rtt: 0,
            quantum_ratio: 0.001,
            filled_pipe: false,
            round_start: false,
            rt_prop_expired: false,
            probe_rtt_round_done: false,
            idle_restart: false,
            packet_conservation: false,
            btl_bw_increased: false,
            lt_use_bw: false,
            lt_is_sampling: false,
            last_loss_was_timeout: false,
            cycle_on_loss: false,
            is_suspended: false,
            is_suspension_nearly_over: false,
        }
    }

    #[test]
    fn enter_startup_sets_state_and_gains() {
        let mut s = make_state();
        s.enter_startup();
        assert_eq!(s.state, Bbr1AlgState::Startup);
        assert!((s.pacing_gain - BBR1_HIGH_GAIN).abs() < f64::EPSILON);
        assert!((s.cwnd_gain - BBR1_HIGH_GAIN).abs() < f64::EPSILON);
    }

    #[test]
    fn check_full_pipe_growing_resets_count() {
        let mut s = make_state();
        s.round_start = true;
        s.full_bw = 1000;
        s.btl_bw = 1300; // >= 1.25 * 1000
        s.check_full_pipe(false);
        assert_eq!(s.full_bw, 1300);
        assert_eq!(s.full_bw_count, 0);
        assert!(!s.filled_pipe);
    }

    #[test]
    fn check_full_pipe_fills_after_3_stall_rounds() {
        let mut s = make_state();
        s.round_start = true;
        s.full_bw = 1000;
        s.btl_bw = 900; // < 1.25 * 1000
        s.check_full_pipe(false);
        assert_eq!(s.full_bw_count, 1);
        assert!(!s.filled_pipe);
        s.check_full_pipe(false);
        assert_eq!(s.full_bw_count, 2);
        s.check_full_pipe(false);
        assert_eq!(s.full_bw_count, 3);
        assert!(s.filled_pipe);
    }

    #[test]
    fn check_full_pipe_skips_when_app_limited() {
        let mut s = make_state();
        s.round_start = true;
        s.full_bw = 1000;
        s.btl_bw = 900;
        s.check_full_pipe(true); // app-limited → skip
        assert_eq!(s.full_bw_count, 0);
        assert!(!s.filled_pipe);
    }

    #[test]
    fn check_full_pipe_skips_when_already_filled() {
        let mut s = make_state();
        s.round_start = true;
        s.filled_pipe = true;
        s.full_bw = 1000;
        s.btl_bw = 900;
        s.check_full_pipe(false);
        assert_eq!(s.full_bw_count, 0); // guard skips the body
    }

    #[test]
    fn enter_probe_rtt_sets_state_and_unity_gains() {
        let mut s = make_state();
        s.enter_probe_rtt();
        assert_eq!(s.state, Bbr1AlgState::ProbeRtt);
        assert!((s.pacing_gain - 1.0).abs() < f64::EPSILON);
        assert!((s.cwnd_gain - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn after_one_roundtrip_clears_packet_conservation() {
        let mut s = make_state();
        s.packet_conservation = true;
        s.after_one_roundtrip_in_fast_recovery();
        assert!(!s.packet_conservation);
    }

    // --- BBR1GetBtlBW ---

    #[test]
    fn get_btl_bw_uses_lt_bw_when_flag_set() {
        let mut s = make_state();
        s.btl_bw = 1_000_000;
        s.lt_bw = 500_000;
        s.lt_use_bw = true;
        assert_eq!(s.get_btl_bw(), 500_000);
    }

    #[test]
    fn get_btl_bw_uses_btl_bw_when_flag_clear() {
        let mut s = make_state();
        s.btl_bw = 1_000_000;
        s.lt_bw = 500_000;
        s.lt_use_bw = false;
        assert_eq!(s.get_btl_bw(), 1_000_000);
    }

    // --- BBR1UpdateRTprop ---

    #[test]
    fn update_rt_prop_lower_sample_updates() {
        let mut s = make_state();
        s.rt_prop = 10_000;
        s.rt_prop_stamp = 0;
        s.update_rt_prop(8_000, 1_000_000);
        assert_eq!(s.rt_prop, 8_000);
        assert_eq!(s.rt_prop_stamp, 1_000_000);
    }

    #[test]
    fn update_rt_prop_slightly_higher_refreshes_stamp() {
        let mut s = make_state();
        s.rt_prop = 10_000;
        s.rt_prop_stamp = 0;
        // delta = 500, 20*500 = 10_000 = rt_prop → not < → stamp not refreshed
        // delta = 100, 20*100 = 2000 < 10_000 → stamp refreshed
        s.update_rt_prop(10_100, 1_000_000);
        assert_eq!(s.rt_prop, 10_000); // unchanged
        assert_eq!(s.rt_prop_stamp, 1_000_000); // refreshed
    }

    #[test]
    fn update_rt_prop_expired_replaces_value() {
        let mut s = make_state();
        s.rt_prop = 1_000;
        s.rt_prop_stamp = 0;
        // current_time > stamp + 10_000_000 AND > stamp + 20*1000 = 20_000
        s.update_rt_prop(5_000, 11_000_000);
        assert!(s.rt_prop_expired);
        assert_eq!(s.rt_prop, 5_000);
    }

    // --- BBR1SetMinimalGain ---

    #[test]
    fn set_minimal_gain_raises_when_bdp_too_small() {
        let mut s = make_state();
        s.pacing_gain = 1.5;
        s.rt_prop = 1_000; // 1 ms
        s.btl_bw = 100_000; // 100 KB/s → BDP = 100 bytes  << 4*1536
        s.set_minimal_gain();
        // target_cwin = 1000 * 100_000 / 1_000_000 = 100
        // d_gain = (4*1536) / 100 = 61.44
        assert!(s.pacing_gain > 1.5);
    }

    #[test]
    fn set_minimal_gain_no_change_when_bdp_large() {
        let mut s = make_state();
        s.pacing_gain = 1.5;
        s.rt_prop = 100_000; // 100 ms
        s.btl_bw = 10_000_000; // 10 MB/s → BDP = 1_000_000 >> 4*1536
        s.set_minimal_gain();
        assert!((s.pacing_gain - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn set_minimal_gain_skips_when_pacing_gain_le_one() {
        let mut s = make_state();
        s.pacing_gain = 0.75;
        s.rt_prop = 1_000;
        s.btl_bw = 100_000;
        s.set_minimal_gain();
        assert!((s.pacing_gain - 0.75).abs() < f64::EPSILON);
    }

    // --- InLossRecovery1 ---

    #[test]
    fn in_loss_recovery_reflects_packet_conservation() {
        let mut s = make_state();
        assert!(!s.in_loss_recovery());
        s.packet_conservation = true;
        assert!(s.in_loss_recovery());
    }

    // --- picoquic_bbr1_observe ---

    #[test]
    fn observe_returns_state_and_btl_bw() {
        let mut s = make_state();
        s.state = Bbr1AlgState::ProbeBw;
        s.btl_bw = 5_000_000;
        let (cc_state, cc_param) = s.observe();
        assert_eq!(cc_state, Bbr1AlgState::ProbeBw as u64);
        assert_eq!(cc_param, 5_000_000);
    }

    // --- picoquic_bbr1_suspension_almost_over ---

    #[test]
    fn suspension_almost_over_sets_flag_when_conditions_met() {
        let mut s = make_state();
        s.is_suspended = true;
        s.cwin_before_suspension = 10_000;
        s.congestion_sequence = 100;
        s.suspension_almost_over(100);
        assert!(s.is_suspension_nearly_over);
    }

    #[test]
    fn suspension_almost_over_no_op_when_not_suspended() {
        let mut s = make_state();
        s.is_suspended = false;
        s.cwin_before_suspension = 10_000;
        s.congestion_sequence = 100;
        s.suspension_almost_over(100);
        assert!(!s.is_suspension_nearly_over);
    }

    #[test]
    fn suspension_almost_over_no_op_when_packet_after_sequence() {
        let mut s = make_state();
        s.is_suspended = true;
        s.cwin_before_suspension = 10_000;
        s.congestion_sequence = 50;
        s.suspension_almost_over(100); // lost_packet_number > congestion_sequence
        assert!(!s.is_suspension_nearly_over);
    }

    // --- BBR1AdvanceCyclePhase ---

    #[test]
    fn advance_cycle_phase_increments_index() {
        let mut s = make_state();
        s.cycle_index = 3;
        s.cycle_start = 3;
        s.pacing_gain = 9.9;
        s.advance_cycle_phase(1_000_000);
        assert_eq!(s.cycle_index, 4);
        assert!((s.pacing_gain - BBR1_PACING_GAIN_CYCLE[4]).abs() < f64::EPSILON);
        assert!(!s.cycle_on_loss);
        assert_eq!(s.cycle_stamp, 1_000_000);
    }

    #[test]
    fn advance_cycle_phase_wraps_with_btl_bw_increased() {
        let mut s = make_state();
        s.cycle_index = BBR1_GAIN_CYCLE_LEN as u32 - 1; // 7
        s.cycle_start = 2;
        s.btl_bw_increased = true;
        s.advance_cycle_phase(0);
        // wrapped: start was 2, btl_bw_increased → start = 3
        assert_eq!(s.cycle_start, 3);
        assert_eq!(s.cycle_index, 3);
        assert!(!s.btl_bw_increased);
    }

    #[test]
    fn advance_cycle_phase_wraps_without_increase_decrements_start() {
        let mut s = make_state();
        s.cycle_index = BBR1_GAIN_CYCLE_LEN as u32 - 1;
        s.cycle_start = 3;
        s.btl_bw_increased = false;
        s.advance_cycle_phase(0);
        // wrapped: start was 3, not increased → start = 2
        assert_eq!(s.cycle_start, 2);
        assert_eq!(s.cycle_index, 2);
    }

    #[test]
    fn advance_cycle_phase_caps_start_at_max() {
        let mut s = make_state();
        s.cycle_index = BBR1_GAIN_CYCLE_LEN as u32 - 1;
        s.cycle_start = BBR1_GAIN_CYCLE_MAX_START; // already at max
        s.btl_bw_increased = true;
        s.advance_cycle_phase(0);
        assert_eq!(s.cycle_start, BBR1_GAIN_CYCLE_MAX_START);
    }

    #[test]
    fn advance_cycle_phase_gain_from_table() {
        let mut s = make_state();
        // index 6 in the gain table is 1.25, index 7 is 0.75
        s.cycle_index = 5;
        s.advance_cycle_phase(0);
        assert!((s.pacing_gain - 1.25).abs() < f64::EPSILON);
        s.advance_cycle_phase(0);
        assert!((s.pacing_gain - 0.75).abs() < f64::EPSILON);
    }

    // --- picoquic_bbr1_set_options ---

    #[test]
    fn set_options_parses_t_prefix() {
        let mut s = make_state();
        s.option_string = Some("T123456".to_string());
        s.set_options();
        assert_eq!(s.wifi_shadow_rtt, 123_456);
    }

    #[test]
    fn set_options_parses_q_prefix_integer() {
        let mut s = make_state();
        s.option_string = Some("Q5".to_string());
        s.set_options();
        assert!((s.quantum_ratio - 5.0).abs() < 1e-9);
    }

    #[test]
    fn set_options_parses_q_prefix_decimal() {
        let mut s = make_state();
        s.option_string = Some("Q0.001".to_string());
        s.set_options();
        assert!((s.quantum_ratio - 0.001).abs() < 1e-9);
    }

    #[test]
    fn set_options_none_is_noop() {
        let mut s = make_state();
        s.option_string = None;
        s.wifi_shadow_rtt = 99;
        s.set_options();
        assert_eq!(s.wifi_shadow_rtt, 99); // unchanged
    }

    // --- BBR1Inflight ---

    #[test]
    fn inflight_returns_cwin_initial_when_rt_prop_not_measured() {
        let mut s = make_state();
        s.rt_prop = u64::MAX; // sentinel: rt_prop not yet measured
        assert_eq!(s.inflight(2.0), CWIN_INITIAL);
    }

    #[test]
    fn inflight_computes_bdp_with_unity_gain() {
        let mut s = make_state();
        s.rt_prop = 100_000; // 100 ms in microseconds
        s.btl_bw = 1_000_000; // 1 MB/s
        s.send_quantum = 1_200;
        // estimated_bdp = 1_000_000 * 100_000 / 1_000_000.0 = 100_000
        // cwnd = 1.0 * 100_000 + 3 * 1_200 = 103_600
        assert_eq!(s.inflight(1.0), 103_600);
    }

    #[test]
    fn inflight_applies_gain_multiplier() {
        let mut s = make_state();
        s.rt_prop = 100_000;
        s.btl_bw = 1_000_000;
        s.send_quantum = 0;
        // bdp = 100_000 bytes; gain=2.0 → cwnd = 200_000
        assert_eq!(s.inflight(2.0), 200_000);
    }

    #[test]
    fn inflight_uses_wifi_shadow_rtt_when_larger() {
        let mut s = make_state();
        s.rt_prop = 50_000;
        s.wifi_shadow_rtt = 100_000; // larger → used as rt_target
        s.btl_bw = 1_000_000;
        s.send_quantum = 0;
        // rt_target = 100_000; bdp = 100_000; gain=1.0 → cwnd = 100_000
        assert_eq!(s.inflight(1.0), 100_000);
    }

    #[test]
    fn inflight_uses_lt_bw_when_flag_set() {
        let mut s = make_state();
        s.rt_prop = 100_000;
        s.btl_bw = 1_000_000;
        s.lt_bw = 500_000;
        s.lt_use_bw = true;
        s.send_quantum = 0;
        // get_btl_bw() returns lt_bw = 500_000; bdp = 50_000
        assert_eq!(s.inflight(1.0), 50_000);
    }

    // --- update_target_cwnd (indirectly tested via inflight) ---

    #[test]
    fn update_target_cwnd_stores_inflight_at_cwnd_gain() {
        let mut s = make_state();
        s.rt_prop = 100_000;
        s.btl_bw = 1_000_000;
        s.send_quantum = 0;
        s.cwnd_gain = 2.0;
        s.update_target_cwnd();
        // inflight(2.0) with bdp=100_000 = 200_000
        assert_eq!(s.target_cwnd, 200_000);
    }
}
