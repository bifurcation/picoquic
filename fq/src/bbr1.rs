//! BBR1 congestion control algorithm.
//!
//! Translated from picoquic/bbr1.c.
//!
//! BBR1 (Bottleneck Bandwidth and Round-trip propagation time) tracks the
//! bottleneck bandwidth and minimum RTT to control the sending rate and
//! congestion window.

use crate::cc_common::{MinMaxRtt, CWIN_INITIAL, CWIN_MINIMUM};

// =============================================================================
// BBR1 Constants
// =============================================================================

/// Length of the bottleneck bandwidth filter (number of rounds).
pub const BTL_BW_FILTER_LENGTH: usize = 10;

/// Length of the RT prop filter (not used directly, but matches C).
pub const RT_PROP_FILTER_LENGTH: usize = 10;

/// High gain for startup phase: 2/ln(2).
pub const HIGH_GAIN: f64 = 2.8853900817779;

/// Minimum pipe cwnd in MSS units.
pub const MIN_PIPE_CWND_MSS: u64 = 4;

/// Length of the gain cycle.
pub const GAIN_CYCLE_LEN: usize = 8;

/// Probe RTT interval (10 seconds in microseconds).
pub const PROBE_RTT_INTERVAL: u64 = 10_000_000;

/// Probe RTT duration (200ms in microseconds).
pub const PROBE_RTT_DURATION: u64 = 200_000;

/// Low pacing rate threshold (1.2 Mbps = 150000 B/s).
pub const PACING_RATE_LOW: f64 = 150_000.0;

/// Medium pacing rate threshold (24 Mbps = 3000000 B/s).
pub const PACING_RATE_MEDIUM: f64 = 3_000_000.0;

/// Maximum cycle start index.
pub const GAIN_CYCLE_MAX_START: u32 = 5;

/// Minimum RTT rounds for leaky bucket sampling.
pub const LT_BW_INTERVAL_MIN_RTT: u32 = 4;

/// Scale factor for loss ratio calculations.
pub const LT_BW_RATIO_SCALE: u64 = 1024;

/// Target loss ratio scaled (205/1024 ~= 20%).
pub const LT_BW_RATIO_SCALED_TARGET: u64 = 205;

/// Maximum RTT rounds for leaky bucket interval.
pub const LT_BW_INTERVAL_MAX_RTT: u32 = LT_BW_INTERVAL_MIN_RTT * 4;

/// Inverse of acceptable bandwidth difference ratio.
pub const LT_BW_RATIO_INVERSE: u64 = 8;

/// Acceptable bandwidth difference in bytes per second.
pub const LT_BW_BYTES_PER_SEC_DIFF: u64 = 4000;

/// Maximum RTT rounds in leaky bucket limited mode.
pub const LT_BW_MAX_RTTS: i32 = 48;

/// RTT threshold for switching to HyStart (50ms in microseconds).
pub const HYSTART_THRESHOLD_RTT: u64 = 50_000;

/// Pacing gain cycle values.
pub const PACING_GAIN_CYCLE: [f64; GAIN_CYCLE_LEN] = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.25, 0.75];

// =============================================================================
// BBR1 Algorithm State
// =============================================================================

/// BBR1 algorithm states.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bbr1AlgState {
    Startup = 0,
    Drain = 1,
    ProbeBw = 2,
    ProbeRtt = 3,
    StartupLongRtt = 4,
}

/// BBR1 congestion control state.
#[derive(Debug, Clone)]
pub struct Bbr1State {
    pub state: Bbr1AlgState,
    /// Bottleneck bandwidth estimate.
    pub btl_bw: u64,
    /// Delivered count at start of next round.
    pub next_round_delivered: u64,
    /// Bandwidth filter for each round.
    pub btl_bw_filter: [u64; BTL_BW_FILTER_LENGTH],
    /// Full bandwidth reached in startup.
    pub full_bw: u64,
    /// Round-trip propagation time (minimum RTT).
    pub rt_prop: u64,
    /// Timestamp when rt_prop was last updated.
    pub rt_prop_stamp: u64,
    /// Timestamp at start of current cycle phase.
    pub cycle_stamp: u64,
    /// Timestamp when probe RTT completes.
    pub probe_rtt_done_stamp: u64,
    /// Saved cwnd before entering probe RTT.
    pub prior_cwnd: u64,
    /// Bytes in flight before processing ACK.
    pub prior_in_flight: u64,
    /// Bytes acknowledged since last processed.
    pub bytes_delivered: u64,
    /// Send quantum (pacing burst size).
    pub send_quantum: u64,
    /// RTT filter for HyStart loss detection.
    pub rtt_filter: MinMaxRtt,
    /// Target congestion window.
    pub target_cwnd: u64,
    /// Current pacing gain.
    pub pacing_gain: f64,
    /// Current cwnd gain.
    pub cwnd_gain: f64,
    /// Current pacing rate in bytes per second.
    pub pacing_rate: f64,
    /// Current index in gain cycle.
    pub cycle_index: u32,
    /// Starting index of gain cycle.
    pub cycle_start: u32,
    /// Number of rounds completed.
    pub round_count: i32,
    /// Rounds without significant bandwidth increase.
    pub full_bw_count: i32,
    /// Rounds in leaky bucket limited mode.
    pub lt_rtt_cnt: i32,
    /// Leaky bucket bandwidth estimate.
    pub lt_bw: u64,
    /// Timestamp at start of leaky bucket interval.
    pub lt_last_stamp: u64,
    /// Bytes lost in previous round.
    pub previous_round_lost: u64,
    /// Bytes delivered at start of sampling.
    pub previous_sampling_delivered: u64,
    /// Bytes lost at start of sampling.
    pub previous_sampling_lost: u64,
    /// Timestamp when last loss was considered.
    pub loss_interval_start: u64,
    /// Sequence number after congestion notification.
    pub congestion_sequence: u64,
    /// Cwnd before suspension (for restoration).
    pub cwin_before_suspension: u64,

    /// WiFi shadow RTT for minimum RTT calculations.
    pub wifi_shadow_rtt: u64,
    /// Quantum ratio for send quantum calculation.
    pub quantum_ratio: f64,

    // Bit flags
    /// Whether the pipe has been filled (exited startup).
    pub filled_pipe: bool,
    /// Whether this is the start of a new round.
    pub round_start: bool,
    /// Whether rt_prop has expired.
    pub rt_prop_expired: bool,
    /// Whether probe RTT round is complete.
    pub probe_rtt_round_done: bool,
    /// Whether restarting from idle.
    pub idle_restart: bool,
    /// Whether in packet conservation mode.
    pub packet_conservation: bool,
    /// Whether btl_bw increased this round.
    pub btl_bw_increased: bool,
    /// Whether using leaky bucket limited bandwidth.
    pub lt_use_bw: bool,
    /// Whether sampling for leaky bucket.
    pub lt_is_sampling: bool,
    /// Whether last loss was a timeout.
    pub last_loss_was_timeout: bool,
    /// Whether to cycle on loss.
    pub cycle_on_loss: bool,
    /// Whether path is suspended due to timeout.
    pub is_suspended: bool,
    /// Whether suspension is nearly over.
    pub is_suspension_nearly_over: bool,
}

impl Default for Bbr1State {
    fn default() -> Self {
        Self {
            state: Bbr1AlgState::Startup,
            btl_bw: 0,
            next_round_delivered: 0,
            btl_bw_filter: [0; BTL_BW_FILTER_LENGTH],
            full_bw: 0,
            rt_prop: u64::MAX,
            rt_prop_stamp: 0,
            cycle_stamp: 0,
            probe_rtt_done_stamp: 0,
            prior_cwnd: 0,
            prior_in_flight: 0,
            bytes_delivered: 0,
            send_quantum: 0,
            rtt_filter: MinMaxRtt::default(),
            target_cwnd: 0,
            pacing_gain: HIGH_GAIN,
            cwnd_gain: HIGH_GAIN,
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
}

impl Bbr1State {
    /// Compute minimum pipe cwnd for a given MTU.
    #[inline]
    pub fn min_pipe_cwnd(send_mtu: u64) -> u64 {
        send_mtu * MIN_PIPE_CWND_MSS
    }

    /// Get the effective bottleneck bandwidth (either measured or limited).
    #[inline]
    pub fn get_btl_bw(&self) -> u64 {
        if self.lt_use_bw {
            self.lt_bw
        } else {
            self.btl_bw
        }
    }

    /// Enter startup state.
    pub fn enter_startup(&mut self) {
        self.state = Bbr1AlgState::Startup;
        self.pacing_gain = HIGH_GAIN;
        self.cwnd_gain = HIGH_GAIN;
    }

    /// Enter startup with long RTT (HyStart mode).
    pub fn enter_startup_long_rtt(&mut self, rtt_min: u64) -> u64 {
        const TARGET_RENO_RTT: u64 = 100_000;
        const TARGET_SATELLITE_RTT: u64 = 300_000;

        self.state = Bbr1AlgState::StartupLongRtt;

        let mut cwnd = CWIN_INITIAL;
        if rtt_min > TARGET_RENO_RTT {
            let ref_rtt = if rtt_min > TARGET_SATELLITE_RTT {
                TARGET_SATELLITE_RTT
            } else {
                rtt_min
            };
            cwnd = (cwnd as f64 * ref_rtt as f64 / TARGET_RENO_RTT as f64) as u64;
        }
        cwnd
    }

    /// Set the send quantum based on pacing rate and MTU.
    pub fn set_send_quantum(&mut self, send_mtu: u64) {
        if self.pacing_rate < PACING_RATE_LOW {
            self.send_quantum = send_mtu;
        } else if self.pacing_rate < PACING_RATE_MEDIUM {
            self.send_quantum = 2 * send_mtu;
        } else {
            self.send_quantum = (self.pacing_rate * self.quantum_ratio) as u64;
            if self.send_quantum > 0x10000 {
                self.send_quantum = 0x10000;
            }
        }
    }

    /// Calculate inflight data for a given gain.
    pub fn inflight(&self, gain: f64) -> u64 {
        if self.rt_prop == u64::MAX {
            return CWIN_INITIAL;
        }

        let rt_target = if self.rt_prop < self.wifi_shadow_rtt {
            self.wifi_shadow_rtt
        } else {
            self.rt_prop
        };

        // Bandwidth is in bytes per second, rt_prop in microseconds
        let estimated_bdp = (self.get_btl_bw() as f64 * rt_target as f64) / 1_000_000.0;
        let quanta = 3 * self.send_quantum;
        (gain * estimated_bdp) as u64 + quanta
    }

    /// Update target cwnd based on current gain.
    pub fn update_target_cwnd(&mut self) {
        self.target_cwnd = self.inflight(self.cwnd_gain);
    }

    /// Update RT prop (minimum RTT).
    pub fn update_rt_prop(&mut self, rtt_sample: u64, current_time: u64) {
        self.rt_prop_expired = current_time > self.rt_prop_stamp + PROBE_RTT_INTERVAL
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

    /// Check if it's time for the next cycle phase.
    pub fn is_next_cycle_phase(
        &self,
        prior_in_flight: u64,
        packets_lost: u64,
        current_time: u64,
    ) -> bool {
        let mut is_full_length =
            self.cycle_on_loss || (current_time - self.cycle_stamp) > self.rt_prop;

        if self.pacing_gain != 1.0 {
            if self.pacing_gain > 1.0 {
                is_full_length &=
                    packets_lost > 0 || prior_in_flight >= self.inflight(self.pacing_gain);
            } else {
                is_full_length &= prior_in_flight <= self.inflight(1.0);
            }
        }
        is_full_length
    }

    /// Set minimal gain for low bandwidth situations.
    pub fn set_minimal_gain(&mut self) {
        if self.pacing_gain > 1.0 && self.rt_prop > 0 {
            let target_cwin = bytes_from_rate(self.rt_prop, self.btl_bw);
            if target_cwin < 4 * 1536 {
                // MAX_PACKET_SIZE
                let d_gain = (4.0 * 1536.0) / target_cwin as f64;
                if d_gain > self.pacing_gain {
                    self.pacing_gain = d_gain;
                }
            }
        }
    }

    /// Advance to the next cycle phase.
    pub fn advance_cycle_phase(&mut self, current_time: u64) {
        self.cycle_on_loss = false;
        self.cycle_stamp = current_time;
        self.cycle_index += 1;

        if self.cycle_index >= GAIN_CYCLE_LEN as u32 {
            let mut start = self.cycle_start;
            if self.btl_bw_increased {
                self.btl_bw_increased = false;
                start += 1;
                if start > GAIN_CYCLE_MAX_START {
                    start = GAIN_CYCLE_MAX_START;
                }
            } else {
                start = start.saturating_sub(1);
            }
            self.cycle_index = start;
            self.cycle_start = start;
        }

        self.pacing_gain = PACING_GAIN_CYCLE[self.cycle_index as usize];
        self.set_minimal_gain();
    }

    /// Reset probe BW mode.
    pub fn reset_probe_bw_mode(&mut self, current_time: u64) {
        self.state = Bbr1AlgState::ProbeBw;
        self.cycle_index = 2;
        self.advance_cycle_phase(current_time);
    }

    /// Check if the pipe is full (bandwidth not growing).
    pub fn check_full_pipe(&mut self, rs_is_app_limited: bool) {
        if !self.filled_pipe && self.round_start && !rs_is_app_limited {
            if self.btl_bw >= (self.full_bw as f64 * 1.25) as u64 {
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

    /// Enter drain state after startup.
    pub fn enter_drain(&mut self) {
        self.state = Bbr1AlgState::Drain;
        self.pacing_gain = 1.0 / HIGH_GAIN;
        self.cwnd_gain = HIGH_GAIN;
    }

    /// Enter probe BW state.
    pub fn enter_probe_bw(&mut self, rt_prop: u64, current_time: u64) {
        const TARGET_RENO_RTT: u64 = 100_000;
        const TARGET_SATELLITE_RTT: u64 = 300_000;

        self.state = Bbr1AlgState::ProbeBw;
        self.pacing_gain = 1.0;
        self.cwnd_gain = 2.0;

        let start = if rt_prop > TARGET_RENO_RTT {
            let ref_rt = if rt_prop > TARGET_SATELLITE_RTT {
                TARGET_SATELLITE_RTT
            } else {
                rt_prop
            };
            let s = ref_rt / TARGET_RENO_RTT;
            if s > GAIN_CYCLE_MAX_START as u64 {
                GAIN_CYCLE_MAX_START
            } else {
                s as u32
            }
        } else {
            2
        };

        self.cycle_index = start;
        self.cycle_start = start;
        self.btl_bw_increased = true;

        self.advance_cycle_phase(current_time);
    }

    /// Enter probe RTT state.
    pub fn enter_probe_rtt(&mut self) {
        self.state = Bbr1AlgState::ProbeRtt;
        self.pacing_gain = 1.0;
        self.cwnd_gain = 1.0;
    }

    /// Check if in loss recovery.
    #[inline]
    pub fn in_loss_recovery(&self) -> bool {
        self.packet_conservation
    }

    /// Save cwnd before entering a special state.
    pub fn save_cwnd(&self, cwin: u64) -> u64 {
        if (self.in_loss_recovery() || self.state == Bbr1AlgState::ProbeBw)
            && cwin < self.prior_cwnd
        {
            self.prior_cwnd
        } else {
            cwin
        }
    }

    /// Restore cwnd after exiting a special state.
    pub fn restore_cwnd(&self, cwin: u64) -> u64 {
        if cwin < self.prior_cwnd {
            self.prior_cwnd
        } else {
            cwin
        }
    }

    /// Set pacing rate with a specific gain.
    pub fn set_pacing_rate_with_gain(&mut self, pacing_gain: f64) {
        let rate = pacing_gain * self.get_btl_bw() as f64;
        if self.filled_pipe || rate > self.pacing_rate {
            self.pacing_rate = rate;
        }
    }

    /// Set pacing rate using current gain.
    pub fn set_pacing_rate(&mut self) {
        let gain = self.pacing_gain;
        self.set_pacing_rate_with_gain(gain);
    }

    /// Reset leaky bucket sampling interval.
    pub fn lt_bw_reset_interval(
        &mut self,
        delivered: u64,
        total_bytes_lost: u64,
        current_time: u64,
    ) {
        self.lt_last_stamp = current_time;
        self.previous_sampling_delivered = delivered;
        self.previous_sampling_lost = total_bytes_lost;
        self.previous_round_lost = total_bytes_lost;
        self.lt_rtt_cnt = 0;
    }

    /// Reset leaky bucket sampling state.
    pub fn lt_bw_reset_sampling(
        &mut self,
        delivered: u64,
        total_bytes_lost: u64,
        current_time: u64,
    ) {
        self.lt_bw = 0;
        self.lt_use_bw = false;
        self.lt_is_sampling = false;
        self.lt_bw_reset_interval(delivered, total_bytes_lost, current_time);
    }

    /// Handle congestion notification.
    ///
    /// Returns new cwin if it should be updated.
    pub fn notify_congestion(
        &mut self,
        cwin: u64,
        current_time: u64,
        smoothed_rtt: u64,
        is_timeout: bool,
    ) -> u64 {
        // Filter repeated loss events
        if (self.cycle_on_loss || current_time < self.loss_interval_start + smoothed_rtt)
            && (!is_timeout || self.last_loss_was_timeout)
        {
            return cwin;
        }

        let new_cwin = if is_timeout || cwin < CWIN_MINIMUM {
            if !self.is_suspended {
                self.is_suspended = true;
                self.cwin_before_suspension = cwin;
            }
            CWIN_MINIMUM
        } else {
            cwin / 2
        };

        self.loss_interval_start = current_time;
        self.last_loss_was_timeout = is_timeout;

        // Update state based on current phase
        if self.state == Bbr1AlgState::StartupLongRtt || self.state == Bbr1AlgState::Startup {
            self.filled_pipe = true;
            self.enter_drain();
        } else {
            self.cycle_on_loss = true;
        }

        new_cwin
    }

    /// Handle spurious repeat notification - mark suspension as nearly over.
    pub fn suspension_almost_over(&mut self, lost_packet_number: u64) {
        if self.is_suspended
            && self.cwin_before_suspension > 0
            && !self.is_suspension_nearly_over
            && self.congestion_sequence >= lost_packet_number
        {
            self.is_suspension_nearly_over = true;
        }
    }

    /// Exit suspension and restore cwin.
    pub fn suspension_exit(&mut self) -> Option<u64> {
        if self.is_suspended && self.is_suspension_nearly_over {
            let cwin = self.cwin_before_suspension;
            self.is_suspended = false;
            self.is_suspension_nearly_over = false;
            Some(cwin)
        } else {
            self.is_suspended = false;
            self.is_suspension_nearly_over = false;
            None
        }
    }

    /// Reset BBR1 state.
    pub fn reset(&mut self, current_time: u64) -> u64 {
        *self = Self::default();
        self.rt_prop = u64::MAX;
        self.rt_prop_stamp = current_time;
        self.cycle_stamp = current_time;
        self.enter_startup();
        CWIN_INITIAL
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute bytes from rate and time interval.
///
/// Returns (rate_bytes_per_sec * interval_microsec) / 1_000_000
#[inline]
pub fn bytes_from_rate(interval_us: u64, rate_bytes_per_sec: u64) -> u64 {
    (rate_bytes_per_sec as u128 * interval_us as u128 / 1_000_000) as u64
}

/// Compute rate from bytes and time interval.
///
/// Returns (bytes * 1_000_000) / interval_microsec
#[inline]
pub fn rate_from_bytes(bytes: u64, interval_us: u64) -> u64 {
    if interval_us == 0 {
        0
    } else {
        (bytes as u128 * 1_000_000 / interval_us as u128) as u64
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbr1_state_default() {
        let state = Bbr1State::default();
        assert_eq!(state.state, Bbr1AlgState::Startup);
        assert_eq!(state.rt_prop, u64::MAX);
        assert_eq!(state.pacing_gain, HIGH_GAIN);
        assert_eq!(state.cwnd_gain, HIGH_GAIN);
        assert!(!state.filled_pipe);
    }

    #[test]
    fn test_bbr1_reset() {
        let mut state = Bbr1State::default();
        state.state = Bbr1AlgState::ProbeBw;
        state.filled_pipe = true;
        state.round_count = 100;

        let cwin = state.reset(1_000_000);

        assert_eq!(state.state, Bbr1AlgState::Startup);
        assert_eq!(state.rt_prop, u64::MAX);
        assert_eq!(state.rt_prop_stamp, 1_000_000);
        assert!(!state.filled_pipe);
        assert_eq!(cwin, CWIN_INITIAL);
    }

    #[test]
    fn test_get_btl_bw() {
        let mut state = Bbr1State::default();
        state.btl_bw = 1_000_000;
        state.lt_bw = 500_000;

        // Not using limited bandwidth
        assert_eq!(state.get_btl_bw(), 1_000_000);

        // Using limited bandwidth
        state.lt_use_bw = true;
        assert_eq!(state.get_btl_bw(), 500_000);
    }

    #[test]
    fn test_min_pipe_cwnd() {
        assert_eq!(Bbr1State::min_pipe_cwnd(1200), 4800);
        assert_eq!(Bbr1State::min_pipe_cwnd(1500), 6000);
    }

    #[test]
    fn test_enter_startup() {
        let mut state = Bbr1State::default();
        state.state = Bbr1AlgState::ProbeBw;
        state.pacing_gain = 1.0;

        state.enter_startup();

        assert_eq!(state.state, Bbr1AlgState::Startup);
        assert_eq!(state.pacing_gain, HIGH_GAIN);
        assert_eq!(state.cwnd_gain, HIGH_GAIN);
    }

    #[test]
    fn test_enter_startup_long_rtt() {
        let mut state = Bbr1State::default();

        // Short RTT - no adjustment
        let cwnd = state.enter_startup_long_rtt(50_000);
        assert_eq!(state.state, Bbr1AlgState::StartupLongRtt);
        assert_eq!(cwnd, CWIN_INITIAL);

        // Long RTT - adjusted cwnd
        let cwnd = state.enter_startup_long_rtt(200_000);
        // 15360 * 200000 / 100000 = 30720
        assert_eq!(cwnd, 30720);

        // Very long RTT - capped at satellite
        let cwnd = state.enter_startup_long_rtt(500_000);
        // 15360 * 300000 / 100000 = 46080
        assert_eq!(cwnd, 46080);
    }

    #[test]
    fn test_set_send_quantum() {
        let mut state = Bbr1State::default();
        let mtu = 1200u64;

        // Low pacing rate
        state.pacing_rate = 100_000.0;
        state.set_send_quantum(mtu);
        assert_eq!(state.send_quantum, mtu);

        // Medium pacing rate
        state.pacing_rate = 1_000_000.0;
        state.set_send_quantum(mtu);
        assert_eq!(state.send_quantum, 2 * mtu);

        // High pacing rate
        state.pacing_rate = 10_000_000.0;
        state.quantum_ratio = 0.001;
        state.set_send_quantum(mtu);
        assert_eq!(state.send_quantum, 10_000);
    }

    #[test]
    fn test_inflight() {
        let mut state = Bbr1State::default();
        state.btl_bw = 1_000_000; // 1 MB/s
        state.rt_prop = 100_000; // 100ms
        state.send_quantum = 1200;

        // BDP = 1_000_000 * 100_000 / 1_000_000 = 100_000 bytes
        // With gain=1.0: 100_000 + 3*1200 = 103_600
        let inflight = state.inflight(1.0);
        assert_eq!(inflight, 103_600);

        // With gain=2.0: 200_000 + 3600 = 203_600
        let inflight = state.inflight(2.0);
        assert_eq!(inflight, 203_600);
    }

    #[test]
    fn test_inflight_no_rt_prop() {
        let state = Bbr1State::default();
        assert_eq!(state.inflight(1.0), CWIN_INITIAL);
    }

    #[test]
    fn test_update_rt_prop() {
        let mut state = Bbr1State::default();
        state.rt_prop = 100_000;
        state.rt_prop_stamp = 0;

        // Smaller RTT updates rt_prop
        state.update_rt_prop(80_000, 1_000);
        assert_eq!(state.rt_prop, 80_000);
        assert_eq!(state.rt_prop_stamp, 1_000);

        // Larger RTT doesn't update (unless expired)
        state.update_rt_prop(90_000, 2_000);
        assert_eq!(state.rt_prop, 80_000);

        // Check expiration
        state.update_rt_prop(90_000, PROBE_RTT_INTERVAL + 20 * 80_000 + 2);
        assert!(state.rt_prop_expired);
        assert_eq!(state.rt_prop, 90_000);
    }

    #[test]
    fn test_check_full_pipe() {
        let mut state = Bbr1State::default();
        state.round_start = true;
        state.full_bw = 1_000_000;
        state.btl_bw = 1_000_000;

        // Not enough growth
        state.check_full_pipe(false);
        assert_eq!(state.full_bw_count, 1);
        assert!(!state.filled_pipe);

        state.check_full_pipe(false);
        assert_eq!(state.full_bw_count, 2);

        state.check_full_pipe(false);
        assert_eq!(state.full_bw_count, 3);
        assert!(state.filled_pipe);
    }

    #[test]
    fn test_check_full_pipe_growth() {
        let mut state = Bbr1State::default();
        state.round_start = true;
        state.full_bw = 1_000_000;

        // Significant growth (25%+)
        state.btl_bw = 1_300_000;
        state.check_full_pipe(false);
        assert_eq!(state.full_bw, 1_300_000);
        assert_eq!(state.full_bw_count, 0);
        assert!(!state.filled_pipe);
    }

    #[test]
    fn test_enter_drain() {
        let mut state = Bbr1State::default();

        state.enter_drain();

        assert_eq!(state.state, Bbr1AlgState::Drain);
        assert!((state.pacing_gain - 1.0 / HIGH_GAIN).abs() < 0.001);
        assert_eq!(state.cwnd_gain, HIGH_GAIN);
    }

    #[test]
    fn test_enter_probe_bw() {
        let mut state = Bbr1State::default();

        state.enter_probe_bw(50_000, 1_000);

        assert_eq!(state.state, Bbr1AlgState::ProbeBw);
        assert_eq!(state.cwnd_gain, 2.0);
        assert!(state.btl_bw_increased);
    }

    #[test]
    fn test_enter_probe_rtt() {
        let mut state = Bbr1State::default();

        state.enter_probe_rtt();

        assert_eq!(state.state, Bbr1AlgState::ProbeRtt);
        assert_eq!(state.pacing_gain, 1.0);
        assert_eq!(state.cwnd_gain, 1.0);
    }

    #[test]
    fn test_save_restore_cwnd() {
        let mut state = Bbr1State::default();
        state.prior_cwnd = 50_000;
        state.packet_conservation = true;

        // In recovery with lower cwin - use prior
        assert_eq!(state.save_cwnd(40_000), 50_000);

        // Higher cwin - use current
        assert_eq!(state.save_cwnd(60_000), 60_000);

        // Restore from lower
        assert_eq!(state.restore_cwnd(40_000), 50_000);

        // Restore from higher
        assert_eq!(state.restore_cwnd(60_000), 60_000);
    }

    #[test]
    fn test_notify_congestion_timeout() {
        let mut state = Bbr1State::default();
        state.state = Bbr1AlgState::Startup;

        let new_cwin = state.notify_congestion(100_000, 1_000, 50_000, true);

        assert_eq!(new_cwin, CWIN_MINIMUM);
        assert!(state.is_suspended);
        assert_eq!(state.cwin_before_suspension, 100_000);
        assert!(state.filled_pipe);
        assert_eq!(state.state, Bbr1AlgState::Drain);
    }

    #[test]
    fn test_notify_congestion_loss() {
        let mut state = Bbr1State::default();
        state.state = Bbr1AlgState::ProbeBw;

        // Use current_time > smoothed_rtt to pass the filter
        let new_cwin = state.notify_congestion(100_000, 100_000, 50_000, false);

        assert_eq!(new_cwin, 50_000);
        assert!(!state.is_suspended);
        assert!(state.cycle_on_loss);
    }

    #[test]
    fn test_suspension_flow() {
        let mut state = Bbr1State::default();
        state.is_suspended = true;
        state.cwin_before_suspension = 100_000;
        state.congestion_sequence = 50;

        // Mark as nearly over
        state.suspension_almost_over(40);
        assert!(state.is_suspension_nearly_over);

        // Exit suspension
        let cwin = state.suspension_exit();
        assert_eq!(cwin, Some(100_000));
        assert!(!state.is_suspended);
        assert!(!state.is_suspension_nearly_over);
    }

    #[test]
    fn test_bytes_from_rate() {
        // 1 MB/s for 100ms = 100KB
        assert_eq!(bytes_from_rate(100_000, 1_000_000), 100_000);

        // 10 MB/s for 1s = 10MB
        assert_eq!(bytes_from_rate(1_000_000, 10_000_000), 10_000_000);
    }

    #[test]
    fn test_rate_from_bytes() {
        // 100KB in 100ms = 1 MB/s
        assert_eq!(rate_from_bytes(100_000, 100_000), 1_000_000);

        // Zero interval
        assert_eq!(rate_from_bytes(100_000, 0), 0);
    }

    #[test]
    fn test_advance_cycle_phase() {
        let mut state = Bbr1State::default();
        state.state = Bbr1AlgState::ProbeBw;
        state.cycle_index = 0;
        state.cycle_start = 0;

        state.advance_cycle_phase(1_000);

        assert_eq!(state.cycle_index, 1);
        assert_eq!(state.pacing_gain, PACING_GAIN_CYCLE[1]);
        assert!(!state.cycle_on_loss);
    }

    #[test]
    fn test_advance_cycle_phase_wrap() {
        let mut state = Bbr1State::default();
        state.state = Bbr1AlgState::ProbeBw;
        state.cycle_index = GAIN_CYCLE_LEN as u32 - 1;
        state.cycle_start = 0;
        state.btl_bw_increased = false;

        state.advance_cycle_phase(1_000);

        // Should wrap to cycle_start (0)
        assert_eq!(state.cycle_index, 0);
    }

    #[test]
    fn test_pacing_gain_cycle_values() {
        // Verify the gain cycle matches C implementation
        assert_eq!(PACING_GAIN_CYCLE[0], 1.0);
        assert_eq!(PACING_GAIN_CYCLE[5], 1.0);
        assert_eq!(PACING_GAIN_CYCLE[6], 1.25);
        assert_eq!(PACING_GAIN_CYCLE[7], 0.75);
    }
}
