//! BBRv3 congestion control algorithm.
//!
//! Translated from picoquic/bbr.c.
//!
//! BBRv3 is a model-based congestion control algorithm that builds an explicit
//! model of the network path including bottleneck bandwidth and round-trip
//! propagation delay. It uses this model to control sending rate and achieve
//! high throughput with low latency and packet loss.
//!
//! Key features:
//! - Estimates bottleneck bandwidth (max_bw) and minimum RTT (min_rtt)
//! - Uses pacing to control sending rate
//! - Probe BW cycle with UP/DOWN/CRUISE/REFILL phases
//! - Probe RTT state to refresh min_rtt estimate
//! - ECN support for L4S networks

use crate::cc_common::MinMaxRtt;

// =============================================================================
// BBRv3 Constants
// =============================================================================

/// Discount factor of 1% used to scale BBR.bw to produce BBR.pacing_rate.
pub const BBR_PACING_MARGIN_PERCENT: u64 = 1;

/// Maximum tolerated packet loss (default: 20%).
pub const BBR_LOSS_THRESH: f64 = 0.2;

/// Multiplicative decrease on packet loss (default: 0.7).
pub const BBR_BETA: f64 = 0.7;

/// Relative amount of headroom left for other flows (default: 0.15).
pub const BBR_HEADROOM: f64 = 0.15;

/// Minimum pipe congestion window in MTU units.
pub const BBR_MIN_PIPE_CWND: u64 = 4;

/// Filter length for max bandwidth (2 cycles).
pub const BBR_MAX_BW_FILTER_LEN: usize = 2;

/// Filter length for extra acked computation.
pub const BBR_EXTRA_ACKED_FILTER_LEN: usize = 10;

/// Length of min RTT filter in microseconds (10 seconds).
pub const BBR_MIN_RTT_FILTER_LEN: u64 = 10_000_000;

/// Number of RTT samples retained to filter out jitter.
pub const BBR_RTT_JITTER_BUFFER_LEN: usize = 7;

/// CWND gain during Probe RTT state.
pub const BBR_PROBE_RTT_CWND_GAIN: f64 = 0.5;

/// Duration of Probe RTT phase in microseconds (200ms).
pub const BBR_PROBE_RTT_DURATION: u64 = 200_000;

/// Interval between Probe RTT phases in microseconds (5 seconds).
pub const BBR_PROBE_RTT_INTERVAL: u64 = 5_000_000;

/// Pacing gain during startup (4*ln(2) ~ 2.77).
pub const BBR_STARTUP_PACING_GAIN: f64 = 2.77;

/// CWND gain during startup.
pub const BBR_STARTUP_CWND_GAIN: f64 = 2.0;

/// Threshold for bandwidth growth detection in startup.
pub const BBR_STARTUP_INCREASE_THRESHOLD: f64 = 1.25;

/// Pacing gain during startup resume.
pub const BBR_STARTUP_RESUME_PACING_GAIN: f64 = 1.25;

/// CWND gain during startup resume.
pub const BBR_STARTUP_RESUME_CWND_GAIN: f64 = 1.25;

/// Increase threshold for startup resume.
pub const BBR_STARTUP_RESUME_INCREASE_THRESHOLD: f64 = 1.125;

/// Pacing gain during Probe BW DOWN phase.
pub const BBR_PROBE_BW_DOWN_PACING_GAIN: f64 = 0.9;

/// CWND gain during Probe BW DOWN phase.
pub const BBR_PROBE_BW_DOWN_CWND_GAIN: f64 = 2.0;

/// Pacing gain during Probe BW CRUISE phase.
pub const BBR_PROBE_BW_CRUISE_PACING_GAIN: f64 = 1.0;

/// CWND gain during Probe BW CRUISE phase.
pub const BBR_PROBE_BW_CRUISE_CWND_GAIN: f64 = 2.0;

/// Pacing gain during Probe BW REFILL phase.
pub const BBR_PROBE_BW_REFILL_PACING_GAIN: f64 = 1.0;

/// CWND gain during Probe BW REFILL phase.
pub const BBR_PROBE_BW_REFILL_CWND_GAIN: f64 = 2.0;

/// Pacing gain during Probe BW UP phase.
pub const BBR_PROBE_BW_UP_PACING_GAIN: f64 = 1.25;

/// CWND gain during Probe BW UP phase.
pub const BBR_PROBE_BW_UP_CWND_GAIN: f64 = 2.25;

/// Number of rounds to wait before exiting app-limited state.
pub const BBR_APP_LIMITED_ROUNDS_THRESHOLD: u32 = 3;

/// Margin factor for avoiding firing RTT Probe too often.
pub const BBR_MIN_RTT_MARGIN_PERCENT: u64 = 5;

/// RTT threshold for "long RTT" mode in microseconds (250ms).
pub const BBR_LONG_RTT_THRESHOLD: u64 = 250_000;

/// ECN CE threshold for excessive congestion (20%).
pub const BBR_EXCESSIVE_ECN_CE: f64 = 0.2;

/// Initial RTT value in microseconds (1ms).
pub const PICOQUIC_INITIAL_RTT: u64 = 1_000;

/// Initial congestion window.
pub const PICOQUIC_CWIN_INITIAL: u64 = 10;

/// Minimum RTT threshold for jitter detection.
pub const PICOQUIC_MINRTT_THRESHOLD: u64 = 10_000;

/// Margin for min RTT comparison.
pub const PICOQUIC_MINRTT_MARGIN: u64 = 1_000;

/// Target RTT for Reno-like behavior.
pub const PICOQUIC_TARGET_RENO_RTT: u64 = 250_000;

/// Target RTT for satellite links.
pub const PICOQUIC_TARGET_SATELLITE_RTT: u64 = 600_000;

// =============================================================================
// BBRv3 State Enums
// =============================================================================

/// BBRv3 algorithm states.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrAlgState {
    Startup = 0,
    Drain = 1,
    ProbeBwDown = 2,
    ProbeBwCruise = 3,
    ProbeBwRefill = 4,
    ProbeBwUp = 5,
    ProbeRtt = 6,
    StartupLongRtt = 7,
    StartupResume = 8,
}

/// BBRv3 ACK processing phases.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrAckPhase {
    ProbeStarting = 0,
    ProbeStopping = 1,
    Refilling = 2,
    ProbeFeedback = 3,
}

// =============================================================================
// Experimental Flags
// =============================================================================

/// Control flags for BBR experimental improvements.
#[derive(Debug, Clone, Copy, Default)]
pub struct BbrExpFlags {
    pub do_early_exit: bool,
    pub do_rapid_start: bool,
    pub do_handle_suspension: bool,
    pub do_control_lost: bool,
    pub do_exit_probe_bw_up_on_delay: bool,
    pub do_enter_probe_bw_after_limited: bool,
}

impl BbrExpFlags {
    /// Create flags with all experiments enabled (default).
    pub fn all_enabled() -> Self {
        Self {
            do_early_exit: true,
            do_rapid_start: true,
            do_handle_suspension: true,
            do_control_lost: true,
            do_exit_probe_bw_up_on_delay: true,
            do_enter_probe_bw_after_limited: true,
        }
    }
}

// =============================================================================
// Per-ACK State
// =============================================================================

/// State associated with processing an ACK.
#[derive(Debug, Clone, Default)]
pub struct BbrPerAckState {
    /// Volume delivered between acked packet and current time.
    pub delivered: u64,
    /// Delivery rate sample when packet was just acked.
    pub delivery_rate: u64,
    /// RTT sample from this ACK.
    pub rtt_sample: u64,
    /// Volume of data acked by current ACK.
    pub newly_acked: u64,
    /// Volume of data marked lost on ACK received.
    pub newly_lost: u64,
    /// Estimate of in-flight data at the time the packet was sent.
    pub tx_in_flight: u64,
    /// Volume lost between transmission of packet and arrival of ACK.
    pub lost: u64,
    /// ECN CE count.
    pub ecn_ce: u64,
    /// ECN fraction for this ACK.
    pub ecn_frac: f64,
    /// ECN alpha (EWMA of ecn_frac).
    pub ecn_alpha: f64,
    /// App marked limited at time of ACK?
    pub is_app_limited: bool,
    /// CWND limited at time of ACK?
    pub is_cwnd_limited: bool,
}

// =============================================================================
// BBRv3 State Structure
// =============================================================================

/// BBRv3 congestion control state.
#[derive(Debug, Clone)]
pub struct BbrState {
    // Algorithm state
    pub state: BbrAlgState,
    pub round_start_pn: u64,
    pub round_count: u32,
    pub rounds_since_probe: u32,
    pub round_start: bool,
    pub next_round_delivered: u64,

    // Output
    pub pacing_rate: f64,
    pub send_quantum: u64,
    pub prior_cwnd: u64,

    // Pacing state
    pub pacing_gain: f64,
    pub next_departure_time: u64,

    // CWND state
    pub cwnd_gain: f64,
    pub packet_conservation: bool,

    // Data rate parameters
    pub max_bw: u64,
    pub bw_hi: u64,
    pub bw_lo: u64,
    pub bw: u64,

    // RTT parameters
    pub min_rtt: u64,
    pub rtt_jitter_buffer: [u64; BBR_RTT_JITTER_BUFFER_LEN],
    pub rtt_jitter_cycle: u64,
    pub rtt_short_term_min: u64,
    pub rtt_short_term_max: u64,
    pub last_rtt_sample_stamp: u64,
    pub nb_rtt_excess: u32,

    // Data volume parameters
    pub bdp: u64,
    pub extra_acked: u64,
    pub offload_budget: u64,
    pub max_inflight: u64,
    pub inflight_hi: u64,
    pub inflight_lo: u64,

    // State for responding to congestion
    pub bw_latest: u64,
    pub inflight_latest: u64,

    // Max BW filter
    pub max_bw_filter: [u64; BBR_MAX_BW_FILTER_LEN],
    pub cycle_count: u32,

    // Extra acked estimation
    pub extra_acked_interval_start: u64,
    pub extra_acked_delivered: u64,
    pub extra_acked_filter: [u64; BBR_EXTRA_ACKED_FILTER_LEN],

    // Startup parameters
    pub filled_pipe: bool,
    pub full_bw: u64,
    pub full_bw_count: u32,

    // Probe RTT parameters
    pub min_rtt_stamp: u64,
    pub probe_rtt_min_delay: u64,
    pub probe_rtt_min_stamp: u64,
    pub probe_rtt_done_stamp: u64,
    pub min_rtt_margin: u64,
    pub probe_rtt_expired: bool,
    pub probe_rtt_round_done: bool,
    pub idle_restart: bool,
    pub path_is_app_limited: bool,

    // Probe BW parameters
    pub probe_probe_bw_quickly: bool,
    pub bw_probe_wait: u64,
    pub bw_probe_ceiling: u64,
    pub cycle_stamp: u64,
    pub rounds_since_bw_probe: u32,
    pub bw_probe_up_cnt: u32,
    pub bw_probe_up_rounds: u32,
    pub bw_probe_samples: u32,
    pub bw_probe_up_acks: u64,
    pub ack_phase: BbrAckPhase,
    pub rtt_too_high_in_round: bool,

    // Loss and recovery
    pub loss_in_round: bool,
    pub loss_round_start: bool,
    pub loss_round_delivered: u64,
    pub is_in_recovery: bool,
    pub is_pto_recovery: bool,
    pub recovery_packet_number: u64,
    pub recovery_delivered: u64,

    // Lost feedback handling
    pub is_handling_lost_feedback: bool,
    pub cwin_before_lost_feedback: u64,

    // App limited management
    pub app_limited_round_count: u32,
    pub app_limited_this_round: bool,

    // ECN marks
    pub ecn_ect1_last_round: u64,
    pub ecn_ce_last_round: u64,
    pub ecn_alpha: f64,

    // Random state
    pub random_context: u64,

    // Startup long RTT
    pub rtt_filter: MinMaxRtt,
    pub bdp_seed: u64,
    pub probe_bdp_seed: bool,

    // Experimental extensions
    pub wifi_shadow_rtt: u64,
    pub quantum_ratio: f64,
    pub exp_flags: BbrExpFlags,
}

impl Default for BbrState {
    fn default() -> Self {
        Self {
            state: BbrAlgState::Startup,
            round_start_pn: 0,
            round_count: 0,
            rounds_since_probe: 0,
            round_start: false,
            next_round_delivered: 0,
            pacing_rate: 0.0,
            send_quantum: 0,
            prior_cwnd: 0,
            pacing_gain: BBR_STARTUP_PACING_GAIN,
            next_departure_time: 0,
            cwnd_gain: BBR_STARTUP_CWND_GAIN,
            packet_conservation: false,
            max_bw: 0,
            bw_hi: 0,
            bw_lo: u64::MAX,
            bw: 0,
            min_rtt: u64::MAX,
            rtt_jitter_buffer: [0; BBR_RTT_JITTER_BUFFER_LEN],
            rtt_jitter_cycle: 0,
            rtt_short_term_min: u64::MAX,
            rtt_short_term_max: 0,
            last_rtt_sample_stamp: 0,
            nb_rtt_excess: 0,
            bdp: 0,
            extra_acked: 0,
            offload_budget: 0,
            max_inflight: 0,
            inflight_hi: u64::MAX,
            inflight_lo: u64::MAX,
            bw_latest: 0,
            inflight_latest: 0,
            max_bw_filter: [0; BBR_MAX_BW_FILTER_LEN],
            cycle_count: 0,
            extra_acked_interval_start: 0,
            extra_acked_delivered: 0,
            extra_acked_filter: [0; BBR_EXTRA_ACKED_FILTER_LEN],
            filled_pipe: false,
            full_bw: 0,
            full_bw_count: 0,
            min_rtt_stamp: 0,
            probe_rtt_min_delay: u64::MAX,
            probe_rtt_min_stamp: 0,
            probe_rtt_done_stamp: 0,
            min_rtt_margin: 0,
            probe_rtt_expired: false,
            probe_rtt_round_done: false,
            idle_restart: false,
            path_is_app_limited: false,
            probe_probe_bw_quickly: false,
            bw_probe_wait: 0,
            bw_probe_ceiling: 0,
            cycle_stamp: 0,
            rounds_since_bw_probe: 0,
            bw_probe_up_cnt: u32::MAX,
            bw_probe_up_rounds: 0,
            bw_probe_samples: 0,
            bw_probe_up_acks: 0,
            ack_phase: BbrAckPhase::ProbeStarting,
            rtt_too_high_in_round: false,
            loss_in_round: false,
            loss_round_start: false,
            loss_round_delivered: 0,
            is_in_recovery: false,
            is_pto_recovery: false,
            recovery_packet_number: u64::MAX,
            recovery_delivered: 0,
            is_handling_lost_feedback: false,
            cwin_before_lost_feedback: 0,
            app_limited_round_count: 0,
            app_limited_this_round: false,
            ecn_ect1_last_round: 0,
            ecn_ce_last_round: 0,
            ecn_alpha: 0.0,
            random_context: 0,
            rtt_filter: MinMaxRtt::default(),
            bdp_seed: 0,
            probe_bdp_seed: false,
            wifi_shadow_rtt: 0,
            quantum_ratio: 0.001,
            exp_flags: BbrExpFlags::all_enabled(),
        }
    }
}

// =============================================================================
// Windowed Filter Functions
// =============================================================================

/// Update a windowed max filter and return the current maximum.
pub fn update_windowed_max_filter(
    filter: &mut [u64],
    value: u64,
    cycle: u32,
    filter_len: usize,
) -> u64 {
    let idx = (cycle as usize) % filter_len;
    if filter[idx] < value {
        filter[idx] = value;
    }
    filter
        .iter()
        .take(filter_len)
        .copied()
        .max()
        .unwrap_or(value)
}

/// Start a new period in the windowed max filter.
pub fn start_windowed_max_filter_period(filter: &mut [u64], cycle: u32, filter_len: usize) {
    let idx = (cycle as usize) % filter_len;
    filter[idx] = 0;
}

/// Update a windowed min filter and return the current minimum.
pub fn update_windowed_min_filter(
    filter: &mut [u64],
    value: u64,
    cycle: u32,
    filter_len: usize,
) -> u64 {
    let idx = (cycle as usize) % filter_len;
    filter[idx] = value;
    filter
        .iter()
        .take(filter_len)
        .copied()
        .min()
        .unwrap_or(value)
}

// =============================================================================
// BbrState Implementation
// =============================================================================

impl BbrState {
    /// Initialize random context.
    pub fn init_random(&mut self, current_time: u64, is_client: bool, unique_path_id: u64) {
        let mut random_context = 0xfedcba9876543210u64;
        random_context ^= current_time;
        if is_client {
            random_context = random_context.wrapping_add(0x0123456789abcdefu64);
        }
        if unique_path_id > 0 && unique_path_id != u64::MAX {
            random_context = random_context.wrapping_mul(unique_path_id + 1);
        }
        self.random_context = random_context;
    }

    /// Initialize full pipe detection state.
    pub fn init_full_pipe(&mut self) {
        self.filled_pipe = false;
        self.full_bw = 0;
        self.full_bw_count = 0;
    }

    /// Initialize round counting.
    pub fn init_round_counting(&mut self, sequence_number: u64) {
        self.next_round_delivered = 0;
        self.round_start = false;
        self.round_count = 0;
        self.round_start_pn = sequence_number;
    }

    /// Reset congestion signals.
    pub fn reset_congestion_signals(&mut self) {
        self.loss_in_round = false;
        self.rtt_too_high_in_round = false;
        self.bw_latest = 0;
        self.inflight_latest = 0;
    }

    /// Reset lower bounds.
    pub fn reset_lower_bounds(&mut self) {
        self.bw_lo = u64::MAX;
        self.inflight_lo = u64::MAX;
    }

    /// Reset RTT jitter buffer.
    pub fn reset_rtt_jitter_buffer(&mut self, rtt_init_value: u64, current_time: u64) {
        self.rtt_jitter_cycle = 0;
        self.last_rtt_sample_stamp = current_time;
        self.rtt_short_term_min = rtt_init_value;
        self.rtt_short_term_max = rtt_init_value;
        self.probe_rtt_min_delay = rtt_init_value;
        self.nb_rtt_excess = 0;
    }

    /// Initialize pacing rate based on initial RTT.
    pub fn init_pacing_rate(&mut self, smoothed_rtt: u64, rtt_variant: u64) {
        let initial_rtt = if smoothed_rtt != PICOQUIC_INITIAL_RTT || rtt_variant != 0 {
            smoothed_rtt
        } else {
            PICOQUIC_INITIAL_RTT
        };
        let nominal_bandwidth = (1_000_000u64 * PICOQUIC_CWIN_INITIAL) as f64 / initial_rtt as f64;
        self.pacing_rate = BBR_STARTUP_PACING_GAIN * nominal_bandwidth;
    }

    /// Enter startup state.
    pub fn enter_startup(&mut self) {
        self.state = BbrAlgState::Startup;
        self.pacing_gain = BBR_STARTUP_PACING_GAIN;
        self.cwnd_gain = BBR_STARTUP_CWND_GAIN;
    }

    /// Re-enter startup state (after recovery or bandwidth increase).
    pub fn reenter_startup(&mut self) {
        self.full_bw = 0;
        self.filled_pipe = false;
        self.full_bw_count = 0;
        self.probe_probe_bw_quickly = true;
        self.enter_startup();
    }

    /// Enter startup resume state.
    pub fn enter_startup_resume(&mut self) {
        self.state = BbrAlgState::StartupResume;
        self.pacing_gain = BBR_STARTUP_RESUME_PACING_GAIN;
        self.cwnd_gain = BBR_STARTUP_RESUME_CWND_GAIN;
    }

    /// Enter drain state.
    pub fn enter_drain(&mut self) {
        self.state = BbrAlgState::Drain;
        self.pacing_gain = 1.0 / BBR_STARTUP_CWND_GAIN;
        self.cwnd_gain = BBR_STARTUP_CWND_GAIN;
    }

    /// Enter Probe RTT state.
    pub fn enter_probe_rtt(&mut self) {
        self.state = BbrAlgState::ProbeRtt;
        self.pacing_gain = 1.0;
        self.cwnd_gain = BBR_PROBE_RTT_CWND_GAIN;
    }

    /// Start Probe BW DOWN phase.
    pub fn start_probe_bw_down(&mut self, current_time: u64, delivered: u64) {
        self.pacing_gain = BBR_PROBE_BW_DOWN_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_DOWN_CWND_GAIN;
        self.reset_congestion_signals();
        self.bw_probe_up_cnt = u32::MAX;

        if self.probe_probe_bw_quickly && self.exp_flags.do_rapid_start {
            self.pick_probe_wait_early();
        } else {
            self.pick_probe_wait();
        }

        self.cycle_stamp = current_time;
        self.ack_phase = BbrAckPhase::ProbeStopping;
        self.next_round_delivered = delivered;
        self.state = BbrAlgState::ProbeBwDown;
        self.nb_rtt_excess = 0;
        self.app_limited_round_count = 0;
        self.app_limited_this_round = false;
    }

    /// Start Probe BW CRUISE phase.
    pub fn start_probe_bw_cruise(&mut self) {
        self.pacing_gain = BBR_PROBE_BW_CRUISE_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_CRUISE_CWND_GAIN;
        self.state = BbrAlgState::ProbeBwCruise;
    }

    /// Start Probe BW REFILL phase.
    pub fn start_probe_bw_refill(&mut self, delivered: u64) {
        self.pacing_gain = BBR_PROBE_BW_REFILL_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_REFILL_CWND_GAIN;
        self.reset_lower_bounds();
        self.bw_probe_up_rounds = 0;
        self.bw_probe_up_acks = 0;
        self.full_bw = self.max_bw;
        self.ack_phase = BbrAckPhase::Refilling;
        self.next_round_delivered = delivered;
        self.state = BbrAlgState::ProbeBwRefill;
    }

    /// Start Probe BW UP phase.
    pub fn start_probe_bw_up(
        &mut self,
        current_time: u64,
        delivered: u64,
        cwin: u64,
        send_mtu: u64,
    ) {
        self.nb_rtt_excess = 0;
        self.pacing_gain = BBR_PROBE_BW_UP_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_UP_CWND_GAIN;
        self.ack_phase = BbrAckPhase::ProbeStarting;
        self.next_round_delivered = delivered;
        self.cycle_stamp = current_time;
        self.state = BbrAlgState::ProbeBwUp;
        self.raise_inflight_hi_slope(cwin, send_mtu);
    }

    /// Enter Probe BW state (from drain or probe RTT).
    pub fn enter_probe_bw(&mut self, current_time: u64, delivered: u64) {
        self.bw_probe_ceiling = self.bw + self.bw / 2;
        self.start_probe_bw_down(current_time, delivered);
    }

    /// Pick random wait time for next BW probe (normal).
    pub fn pick_probe_wait(&mut self) {
        self.rounds_since_bw_probe = self.random_int_between(0, 1) as u32;

        if self.min_rtt < BBR_LONG_RTT_THRESHOLD {
            self.bw_probe_wait = 2_000_000 + self.random_int_between(0, 1_000_000);
        } else {
            self.bw_probe_wait = 8 * self.min_rtt + self.random_int_between(0, 4 * self.min_rtt);
        }
    }

    /// Pick random wait time for next BW probe (early).
    pub fn pick_probe_wait_early(&mut self) {
        self.rounds_since_bw_probe = self.random_int_between(0, 1) as u32;

        if self.min_rtt < BBR_LONG_RTT_THRESHOLD {
            self.bw_probe_wait = self.min_rtt + self.random_int_between(0, BBR_LONG_RTT_THRESHOLD);
        } else {
            self.bw_probe_wait = self.min_rtt + self.random_int_between(0, self.min_rtt);
        }
    }

    /// Generate a random integer between low and high (inclusive).
    pub fn random_int_between(&mut self, low: u64, high: u64) -> u64 {
        if high <= low {
            return low;
        }
        let range = high - low + 1;
        // Simple LCG random number generator
        self.random_context = self
            .random_context
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        low + (self.random_context % range)
    }

    /// Check if currently in a Probe BW state.
    pub fn is_in_probe_bw_state(&self) -> bool {
        matches!(
            self.state,
            BbrAlgState::ProbeBwDown
                | BbrAlgState::ProbeBwCruise
                | BbrAlgState::ProbeBwRefill
                | BbrAlgState::ProbeBwUp
        )
    }

    /// Check if currently probing for bandwidth (actively increasing).
    pub fn is_probing_bw(&self) -> bool {
        !matches!(
            self.state,
            BbrAlgState::ProbeBwDown
                | BbrAlgState::ProbeBwCruise
                | BbrAlgState::Drain
                | BbrAlgState::ProbeRtt
        )
    }

    /// Check if in loss recovery.
    pub fn in_loss_recovery(&self) -> bool {
        self.is_in_recovery
    }

    /// Raise the inflight_hi slope.
    pub fn raise_inflight_hi_slope(&mut self, cwin: u64, send_mtu: u64) {
        let growth_this_round = send_mtu << self.bw_probe_up_rounds;
        self.bw_probe_up_rounds = (self.bw_probe_up_rounds + 1).min(30);
        let up_cnt = (cwin / growth_this_round) as u32;
        self.bw_probe_up_cnt = up_cnt.max(1);
    }

    /// Compute BDP multiple with given gain and bandwidth.
    pub fn bdp_multiple_with_bw(&self, gain: f64, bw: u64, send_mtu: u64) -> u64 {
        if self.min_rtt == u64::MAX {
            return PICOQUIC_CWIN_INITIAL * send_mtu;
        }
        let bdp = (self.min_rtt as f64 * bw as f64 / 1_000_000.0) as u64;
        (gain * bdp as f64) as u64
    }

    /// Compute BDP multiple with current bandwidth.
    pub fn bdp_multiple(&self, gain: f64, send_mtu: u64) -> u64 {
        self.bdp_multiple_with_bw(gain, self.bw, send_mtu)
    }

    /// Compute inflight with headroom.
    pub fn inflight_with_headroom(&self, send_mtu: u64) -> u64 {
        if self.inflight_hi == u64::MAX {
            return u64::MAX;
        }
        let inflight_with_headroom = ((1.0 - BBR_HEADROOM) * self.inflight_hi as f64) as u64;
        inflight_with_headroom.max(BBR_MIN_PIPE_CWND * send_mtu)
    }

    /// Compute target inflight.
    pub fn target_inflight(&self, cwin: u64) -> u64 {
        self.bdp.min(cwin)
    }

    /// Compute inflight with given bandwidth and gain.
    pub fn inflight_with_bw(&self, gain: f64, bw: u64, send_mtu: u64, cwin: u64) -> u64 {
        let inflight = self.bdp_multiple_with_bw(gain, bw, send_mtu);
        self.quantization_budget(inflight, send_mtu, cwin)
    }

    /// Compute quantization budget.
    pub fn quantization_budget(&self, inflight: u64, send_mtu: u64, _cwin: u64) -> u64 {
        let offload_budget = 3 * self.send_quantum;
        let mut result = inflight.max(offload_budget);
        result = result.max(BBR_MIN_PIPE_CWND * send_mtu);
        if self.state == BbrAlgState::ProbeBwUp {
            result += 2 * send_mtu;
        }
        // Ensure we don't exceed current CWND in certain states
        if self.is_in_probe_bw_state()
            && self.state != BbrAlgState::ProbeBwCruise
            && self.inflight_hi > 0
            && self.inflight_hi < u64::MAX
            && result > self.inflight_hi
        {
            result = self.inflight_hi;
        }
        result.max(BBR_MIN_PIPE_CWND * send_mtu)
    }

    /// Check if RTT is too high.
    pub fn is_rtt_too_high(&self) -> bool {
        self.nb_rtt_excess > BBR_RTT_JITTER_BUFFER_LEN as u32
    }

    /// Check if inflight is too high based on loss and ECN.
    pub fn is_inflight_too_high(
        &self,
        rs: &BbrPerAckState,
        tx_in_flight: u64,
        send_mtu: u64,
    ) -> bool {
        if rs.ecn_alpha > BBR_EXCESSIVE_ECN_CE {
            return true;
        }
        // Check loss rate
        if rs.lost > ((tx_in_flight as f64) * BBR_LOSS_THRESH) as u64 && rs.lost > 3 * send_mtu {
            return true;
        }
        false
    }

    /// Update max bandwidth filter.
    pub fn update_max_bw(&mut self, rs: &BbrPerAckState) {
        if rs.delivery_rate
            >= self.max_bw_filter[(self.cycle_count as usize) % BBR_MAX_BW_FILTER_LEN]
            || !rs.is_app_limited
        {
            self.max_bw = update_windowed_max_filter(
                &mut self.max_bw_filter,
                rs.delivery_rate,
                self.cycle_count,
                BBR_MAX_BW_FILTER_LEN,
            );
        }
    }

    /// Advance max bandwidth filter to new cycle.
    pub fn advance_max_bw_filter(&mut self) {
        self.cycle_count += 1;
        self.ack_phase = BbrAckPhase::ProbeStarting;
        start_windowed_max_filter_period(
            &mut self.max_bw_filter,
            self.cycle_count,
            BBR_MAX_BW_FILTER_LEN,
        );
    }

    /// Bound bandwidth for model.
    pub fn bound_bw_for_model(&mut self) {
        self.bw = self.max_bw;
        if self.bw > self.bw_lo {
            self.bw = self.bw_lo;
        }
        if self.bw > self.bw_hi && self.bw_hi != 0 {
            self.bw = self.bw_hi;
        }
    }

    /// Update BDP estimate.
    pub fn update_bdp(&mut self) {
        if self.min_rtt < u64::MAX && self.bw > 0 {
            self.bdp = (self.min_rtt as f64 * self.bw as f64 / 1_000_000.0) as u64;
        }
    }

    /// Set pacing rate with given gain.
    pub fn set_pacing_rate_with_gain(&mut self, pacing_gain: f64) {
        let rate =
            pacing_gain * (self.bw as f64 * (100 - BBR_PACING_MARGIN_PERCENT) as f64) / 100.0;

        // Handle startup resume with BDP seed
        let mut final_rate = rate;
        if self.state == BbrAlgState::StartupResume
            && !self.filled_pipe
            && self.bdp_seed > 0
            && self.min_rtt > 0
        {
            let bdp_rate = (self.bdp_seed as f64 * 1_000_000.0) / self.min_rtt as f64;
            if bdp_rate > final_rate {
                final_rate = bdp_rate;
            }
        }

        if self.filled_pipe || final_rate > self.pacing_rate {
            self.pacing_rate = final_rate;
        }
    }

    /// Set pacing rate using current pacing gain.
    pub fn set_pacing_rate(&mut self) {
        self.set_pacing_rate_with_gain(self.pacing_gain);
    }

    /// Set send quantum based on pacing rate.
    pub fn set_send_quantum(&mut self, send_mtu: u64) {
        let floor = if self.pacing_rate < 150_000.0 {
            send_mtu
        } else {
            2 * send_mtu
        };

        self.send_quantum = (self.pacing_rate * self.quantum_ratio) as u64;
        self.send_quantum = self.send_quantum.min(0x10000);
        self.send_quantum = self.send_quantum.max(floor);
    }

    /// Update max inflight.
    pub fn update_max_inflight(&mut self, send_mtu: u64) {
        let mut inflight = self.bdp_multiple(self.cwnd_gain, send_mtu);
        inflight += self.extra_acked;

        if self.min_rtt < self.wifi_shadow_rtt && self.min_rtt > 0 {
            inflight = (inflight as f64 * self.wifi_shadow_rtt as f64 / self.min_rtt as f64) as u64;
        }

        self.max_inflight = self.quantization_budget(inflight, send_mtu, u64::MAX);
    }

    /// Save current CWND for later restoration.
    pub fn save_cwnd(&self, cwin: u64) -> u64 {
        if !self.in_loss_recovery() && self.state != BbrAlgState::ProbeRtt {
            cwin
        } else {
            self.prior_cwnd.max(cwin)
        }
    }

    /// Restore saved CWND.
    pub fn restore_cwnd(&self, cwin: u64) -> u64 {
        self.prior_cwnd.max(cwin)
    }

    /// Enter fast recovery.
    pub fn enter_fast_recovery(
        &mut self,
        cwin: u64,
        bytes_in_transit: u64,
        send_mtu: u64,
        newly_acked: u64,
        sequence_number: u64,
        delivered: u64,
    ) -> u64 {
        self.prior_cwnd = self.save_cwnd(cwin);
        let additional_cwnd = newly_acked.max(send_mtu);
        let new_cwin = bytes_in_transit + additional_cwnd;
        self.recovery_packet_number = sequence_number;
        self.packet_conservation = true;
        self.is_in_recovery = true;
        self.is_pto_recovery = false;
        self.recovery_delivered = delivered;
        new_cwin
    }

    /// Enter RTO recovery.
    pub fn enter_rto(
        &mut self,
        cwin: u64,
        bytes_in_transit: u64,
        send_mtu: u64,
        lost_packet_number: u64,
        delivered: u64,
    ) -> u64 {
        if !self.is_in_recovery {
            self.prior_cwnd = self.save_cwnd(cwin);
            self.is_in_recovery = true;
        }
        if !self.is_pto_recovery {
            self.recovery_packet_number = lost_packet_number;
            self.is_pto_recovery = true;
            self.recovery_delivered = delivered;
            return bytes_in_transit + send_mtu;
        }
        cwin
    }

    /// Exit recovery.
    pub fn exit_recovery(&mut self, cwin: u64, current_time: u64) -> u64 {
        if !self.is_in_recovery {
            return cwin;
        }

        let new_cwin = self.restore_cwnd(cwin);
        self.recovery_packet_number = u64::MAX;
        self.packet_conservation = false;

        if self.is_pto_recovery && self.exp_flags.do_handle_suspension {
            self.reenter_startup();
        } else if self.state == BbrAlgState::ProbeBwUp {
            // Will transition to DOWN on next update
        }

        self.recovery_delivered = 0;
        self.is_in_recovery = false;
        self.is_pto_recovery = false;
        self.probe_rtt_min_stamp = current_time;
        self.min_rtt_stamp = current_time;

        new_cwin
    }

    /// Handle spurious loss.
    pub fn handle_spurious_loss(
        &mut self,
        lost_packet_number: u64,
        cwin: u64,
        current_time: u64,
    ) -> u64 {
        if self.recovery_packet_number <= lost_packet_number && self.is_pto_recovery {
            self.exit_recovery(cwin, current_time)
        } else {
            cwin
        }
    }

    /// Check if elapsed time has passed in current phase.
    pub fn has_elapsed_in_phase(&self, interval: u64, current_time: u64) -> bool {
        current_time > self.cycle_stamp + interval
    }

    /// Check if it's time to transition from DOWN to CRUISE.
    pub fn check_time_to_cruise(&self, bytes_in_transit: u64, send_mtu: u64) -> bool {
        if bytes_in_transit > self.inflight_with_headroom(send_mtu) {
            return false;
        }
        bytes_in_transit <= self.inflight_with_bw(1.0, self.max_bw, send_mtu, u64::MAX)
    }

    /// Check startup full bandwidth detection.
    pub fn check_startup_full_bandwidth(&mut self, rs: &BbrPerAckState) {
        if self.filled_pipe || !self.round_start || rs.is_app_limited {
            return;
        }

        // Using 5/4 test instead of 1.25 double comparison
        if 4 * self.max_bw >= 5 * self.full_bw {
            self.full_bw = self.max_bw;
            self.full_bw_count = 0;
            if rs.ecn_frac < 0.2 {
                return;
            }
        }

        self.full_bw_count += 1;
        if self.full_bw_count >= 3 || rs.ecn_frac >= BBR_EXCESSIVE_ECN_CE {
            self.filled_pipe = true;
        }
    }

    /// Check startup high loss.
    pub fn check_startup_high_loss(
        &mut self,
        rs: &BbrPerAckState,
        tx_in_flight: u64,
        send_mtu: u64,
    ) {
        if self.is_inflight_too_high(rs, tx_in_flight, send_mtu) {
            self.filled_pipe = true;
        }
    }

    /// Set BDP seed for careful resume.
    pub fn set_bdp_seed(&mut self, bdp_seed: u64) {
        self.bdp_seed = bdp_seed;
        if self.state == BbrAlgState::Startup && self.bdp_seed > self.max_bw {
            self.enter_startup_resume();
        }
    }

    /// Update RTT jitter buffer.
    pub fn update_rtt_jitter_buffer(&mut self, rs: &BbrPerAckState, current_time: u64) {
        if current_time > self.last_rtt_sample_stamp + 1000 {
            let idx = (self.rtt_jitter_cycle as usize) % BBR_RTT_JITTER_BUFFER_LEN;
            self.rtt_jitter_buffer[idx] = rs.rtt_sample;
            self.rtt_jitter_cycle += 1;
            self.last_rtt_sample_stamp = current_time;

            // Recompute min/max
            self.rtt_short_term_min = u64::MAX;
            self.rtt_short_term_max = 0;
            let count = (self.rtt_jitter_cycle as usize).min(BBR_RTT_JITTER_BUFFER_LEN);
            for i in 0..count {
                let rtt = self.rtt_jitter_buffer[i];
                if rtt > self.rtt_short_term_max {
                    self.rtt_short_term_max = rtt;
                }
                if rtt < self.rtt_short_term_min {
                    self.rtt_short_term_min = rtt;
                }
            }
        }
    }

    /// Adapt min RTT margin based on bandwidth.
    pub fn adapt_min_rtt_margin(&mut self, send_mtu: u64) {
        let mut margin = (self.min_rtt * BBR_MIN_RTT_MARGIN_PERCENT) * 100 / 1_000_000;
        if self.max_bw > 0 {
            margin += 2 * send_mtu * 1_000_000 / self.max_bw;
        }
        self.min_rtt_margin = margin;
    }

    /// Initialize lower bounds on first congestion event.
    pub fn init_lower_bounds(&mut self, cwin: u64) {
        if self.bw_lo == u64::MAX {
            self.bw_lo = self.max_bw;
        }
        if self.inflight_lo == u64::MAX {
            self.inflight_lo = cwin;
        }
    }

    /// Adjust lower bounds based on loss.
    pub fn loss_lower_bounds(&mut self) {
        self.bw_lo = ((BBR_BETA * self.bw_lo as f64) as u64).max(self.bw_latest);
        self.inflight_lo = ((BBR_BETA * self.inflight_lo as f64) as u64).max(self.inflight_latest);
    }

    /// Update latest delivery signals.
    pub fn update_latest_delivery_signals(&mut self, rs: &BbrPerAckState, delivered: u64) {
        self.loss_round_start = false;

        if self.bw_latest < rs.delivery_rate {
            self.bw_latest = rs.delivery_rate;
        }
        if self.inflight_latest < rs.delivered {
            self.inflight_latest = rs.delivered;
        }

        let prior_delivered = delivered.saturating_sub(rs.delivered);
        if prior_delivered >= self.loss_round_delivered {
            self.loss_round_delivered = delivered;
            self.loss_round_start = true;
        }
    }

    /// Advance latest delivery signals at end of round.
    pub fn advance_latest_delivery_signals(&mut self, rs: &BbrPerAckState) {
        if self.loss_round_start {
            self.bw_latest = rs.delivery_rate;
            self.inflight_latest = rs.delivered;
        }
    }

    /// Update round tracking.
    pub fn update_round(&mut self, ack_number: u64, delivered: u64) {
        if ack_number >= self.round_start_pn {
            self.round_start_pn = ack_number;
            self.next_round_delivered = delivered;
            self.round_count += 1;
            self.rounds_since_probe += 1;
            self.round_start = true;
            start_windowed_max_filter_period(
                &mut self.extra_acked_filter,
                self.round_count,
                BBR_EXTRA_ACKED_FILTER_LEN,
            );
        } else {
            self.round_start = false;
        }
    }

    /// Update ACK aggregation estimate.
    pub fn update_ack_aggregation(&mut self, rs: &BbrPerAckState, cwin: u64, current_time: u64) {
        let interval = current_time.saturating_sub(self.extra_acked_interval_start);
        let expected_delivered = (self.bw as f64 * interval as f64 / 1_000_000.0) as u64;

        if self.extra_acked_delivered <= expected_delivered {
            self.extra_acked_delivered = 0;
            self.extra_acked_interval_start = current_time;
        }

        self.extra_acked_delivered += rs.newly_acked;
        let extra = self
            .extra_acked_delivered
            .saturating_sub(expected_delivered)
            .min(cwin);

        self.extra_acked = update_windowed_max_filter(
            &mut self.extra_acked_filter,
            extra,
            self.round_count,
            BBR_EXTRA_ACKED_FILTER_LEN,
        );
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbr_state_default() {
        let state = BbrState::default();
        assert_eq!(state.state, BbrAlgState::Startup);
        assert_eq!(state.pacing_gain, BBR_STARTUP_PACING_GAIN);
        assert_eq!(state.cwnd_gain, BBR_STARTUP_CWND_GAIN);
        assert!(!state.filled_pipe);
        assert_eq!(state.min_rtt, u64::MAX);
    }

    #[test]
    fn test_bbr_alg_states() {
        assert_eq!(BbrAlgState::Startup as u8, 0);
        assert_eq!(BbrAlgState::Drain as u8, 1);
        assert_eq!(BbrAlgState::ProbeBwDown as u8, 2);
        assert_eq!(BbrAlgState::ProbeBwCruise as u8, 3);
        assert_eq!(BbrAlgState::ProbeBwRefill as u8, 4);
        assert_eq!(BbrAlgState::ProbeBwUp as u8, 5);
        assert_eq!(BbrAlgState::ProbeRtt as u8, 6);
        assert_eq!(BbrAlgState::StartupLongRtt as u8, 7);
        assert_eq!(BbrAlgState::StartupResume as u8, 8);
    }

    #[test]
    fn test_bbr_exp_flags() {
        let flags = BbrExpFlags::all_enabled();
        assert!(flags.do_early_exit);
        assert!(flags.do_rapid_start);
        assert!(flags.do_handle_suspension);
        assert!(flags.do_control_lost);
        assert!(flags.do_exit_probe_bw_up_on_delay);
        assert!(flags.do_enter_probe_bw_after_limited);

        let default_flags = BbrExpFlags::default();
        assert!(!default_flags.do_early_exit);
    }

    #[test]
    fn test_windowed_max_filter() {
        let mut filter = [0u64; 3];

        let max = update_windowed_max_filter(&mut filter, 100, 0, 3);
        assert_eq!(max, 100);

        let max = update_windowed_max_filter(&mut filter, 50, 1, 3);
        assert_eq!(max, 100);

        let max = update_windowed_max_filter(&mut filter, 200, 2, 3);
        assert_eq!(max, 200);

        // Start new period
        start_windowed_max_filter_period(&mut filter, 3, 3);
        let max = update_windowed_max_filter(&mut filter, 30, 3, 3);
        // Slot 0 now has 30, but slot 1 has 50, slot 2 has 200
        assert_eq!(max, 200);
    }

    #[test]
    fn test_enter_states() {
        let mut state = BbrState::default();

        state.enter_startup();
        assert_eq!(state.state, BbrAlgState::Startup);
        assert_eq!(state.pacing_gain, BBR_STARTUP_PACING_GAIN);

        state.enter_drain();
        assert_eq!(state.state, BbrAlgState::Drain);
        assert_eq!(state.pacing_gain, 1.0 / BBR_STARTUP_CWND_GAIN);

        state.enter_probe_rtt();
        assert_eq!(state.state, BbrAlgState::ProbeRtt);
        assert_eq!(state.cwnd_gain, BBR_PROBE_RTT_CWND_GAIN);
    }

    #[test]
    fn test_probe_bw_states() {
        let mut state = BbrState::default();
        state.min_rtt = 50_000;
        state.bw = 1_000_000;

        state.start_probe_bw_down(1_000_000, 100);
        assert_eq!(state.state, BbrAlgState::ProbeBwDown);
        assert_eq!(state.pacing_gain, BBR_PROBE_BW_DOWN_PACING_GAIN);

        state.start_probe_bw_cruise();
        assert_eq!(state.state, BbrAlgState::ProbeBwCruise);
        assert_eq!(state.pacing_gain, BBR_PROBE_BW_CRUISE_PACING_GAIN);

        state.start_probe_bw_refill(200);
        assert_eq!(state.state, BbrAlgState::ProbeBwRefill);
        assert_eq!(state.bw_lo, u64::MAX);

        state.start_probe_bw_up(2_000_000, 300, 50_000, 1200);
        assert_eq!(state.state, BbrAlgState::ProbeBwUp);
        assert_eq!(state.pacing_gain, BBR_PROBE_BW_UP_PACING_GAIN);
    }

    #[test]
    fn test_is_in_probe_bw_state() {
        let mut state = BbrState::default();

        state.state = BbrAlgState::Startup;
        assert!(!state.is_in_probe_bw_state());

        state.state = BbrAlgState::ProbeBwDown;
        assert!(state.is_in_probe_bw_state());

        state.state = BbrAlgState::ProbeBwCruise;
        assert!(state.is_in_probe_bw_state());

        state.state = BbrAlgState::ProbeBwRefill;
        assert!(state.is_in_probe_bw_state());

        state.state = BbrAlgState::ProbeBwUp;
        assert!(state.is_in_probe_bw_state());

        state.state = BbrAlgState::ProbeRtt;
        assert!(!state.is_in_probe_bw_state());
    }

    #[test]
    fn test_random_int_between() {
        let mut state = BbrState::default();
        state.random_context = 12345;

        for _ in 0..100 {
            let r = state.random_int_between(0, 10);
            assert!(r <= 10);
        }

        let r = state.random_int_between(5, 5);
        assert_eq!(r, 5);
    }

    #[test]
    fn test_bdp_multiple() {
        let mut state = BbrState::default();
        state.min_rtt = 50_000; // 50ms
        state.bw = 1_000_000; // 1 Mbps

        // BDP = 50ms * 1Mbps = 50_000 * 1 = 50_000 bytes
        let bdp = state.bdp_multiple(1.0, 1200);
        assert!(bdp > 0);

        // With gain
        let bdp_2x = state.bdp_multiple(2.0, 1200);
        assert!(bdp_2x > bdp);
    }

    #[test]
    fn test_recovery() {
        let mut state = BbrState::default();

        // Enter fast recovery
        let new_cwin = state.enter_fast_recovery(50_000, 30_000, 1200, 1200, 100, 10_000);
        assert!(state.is_in_recovery);
        assert!(!state.is_pto_recovery);
        assert_eq!(new_cwin, 30_000 + 1200);

        // Exit recovery
        let restored = state.exit_recovery(new_cwin, 2_000_000);
        assert!(!state.is_in_recovery);
        assert!(restored >= new_cwin);
    }

    #[test]
    fn test_enter_rto() {
        let mut state = BbrState::default();

        let new_cwin = state.enter_rto(50_000, 10_000, 1200, 50, 5_000);
        assert!(state.is_in_recovery);
        assert!(state.is_pto_recovery);
        assert_eq!(new_cwin, 10_000 + 1200);
    }

    #[test]
    fn test_check_startup_full_bandwidth() {
        let mut state = BbrState::default();
        state.round_start = true;
        state.full_bw = 1_000_000;

        let rs = BbrPerAckState {
            is_app_limited: false,
            ecn_frac: 0.0,
            ..Default::default()
        };

        // First call with max_bw not much higher - count goes up
        state.max_bw = 1_100_000;
        state.check_startup_full_bandwidth(&rs);
        assert_eq!(state.full_bw_count, 1);
        assert!(!state.filled_pipe);

        // Second call
        state.check_startup_full_bandwidth(&rs);
        assert_eq!(state.full_bw_count, 2);

        // Third call - should fill pipe
        state.check_startup_full_bandwidth(&rs);
        assert!(state.filled_pipe);
    }

    #[test]
    fn test_inflight_too_high() {
        let state = BbrState::default();

        let rs_low_loss = BbrPerAckState {
            ecn_alpha: 0.1,
            lost: 100,
            ..Default::default()
        };
        assert!(!state.is_inflight_too_high(&rs_low_loss, 100_000, 1200));

        let rs_high_ecn = BbrPerAckState {
            ecn_alpha: 0.3, // > BBR_EXCESSIVE_ECN_CE
            lost: 0,
            ..Default::default()
        };
        assert!(state.is_inflight_too_high(&rs_high_ecn, 100_000, 1200));

        let rs_high_loss = BbrPerAckState {
            ecn_alpha: 0.1,
            lost: 25_000, // > 20% of 100_000
            ..Default::default()
        };
        assert!(state.is_inflight_too_high(&rs_high_loss, 100_000, 1200));
    }

    #[test]
    fn test_update_max_bw() {
        let mut state = BbrState::default();
        state.cycle_count = 0;

        let rs = BbrPerAckState {
            delivery_rate: 1_000_000,
            is_app_limited: false,
            ..Default::default()
        };

        state.update_max_bw(&rs);
        assert_eq!(state.max_bw, 1_000_000);

        // Higher rate updates
        let rs2 = BbrPerAckState {
            delivery_rate: 2_000_000,
            is_app_limited: false,
            ..Default::default()
        };
        state.update_max_bw(&rs2);
        assert_eq!(state.max_bw, 2_000_000);
    }

    #[test]
    fn test_bound_bw_for_model() {
        let mut state = BbrState::default();
        state.max_bw = 1_000_000;
        state.bw_lo = 800_000;
        state.bw_hi = 1_200_000;

        state.bound_bw_for_model();
        assert_eq!(state.bw, 800_000); // min of max_bw and bw_lo
    }

    #[test]
    fn test_set_pacing_rate() {
        let mut state = BbrState::default();
        state.bw = 1_000_000;
        state.filled_pipe = true;

        state.set_pacing_rate_with_gain(1.0);
        // pacing_rate = 1.0 * (1_000_000 * 99 / 100) = 990_000
        assert!((state.pacing_rate - 990_000.0).abs() < 1.0);
    }

    #[test]
    fn test_set_send_quantum() {
        let mut state = BbrState::default();
        state.pacing_rate = 1_000_000.0;
        state.quantum_ratio = 0.001;

        state.set_send_quantum(1200);
        // quantum = 1_000_000 * 0.001 = 1000, floor = 2 * 1200 = 2400
        assert_eq!(state.send_quantum, 2400);
    }

    #[test]
    fn test_constants() {
        assert_eq!(BBR_PACING_MARGIN_PERCENT, 1);
        assert!((BBR_LOSS_THRESH - 0.2).abs() < f64::EPSILON);
        assert!((BBR_BETA - 0.7).abs() < f64::EPSILON);
        assert_eq!(BBR_MIN_PIPE_CWND, 4);
        assert_eq!(BBR_MAX_BW_FILTER_LEN, 2);
        assert_eq!(BBR_MIN_RTT_FILTER_LEN, 10_000_000);
        assert!((BBR_STARTUP_PACING_GAIN - 2.77).abs() < 0.01);
    }

    #[test]
    fn test_has_elapsed_in_phase() {
        let mut state = BbrState::default();
        state.cycle_stamp = 1_000_000;

        assert!(!state.has_elapsed_in_phase(500_000, 1_400_000));
        assert!(state.has_elapsed_in_phase(500_000, 1_600_000));
    }

    #[test]
    fn test_init_and_reset() {
        let mut state = BbrState::default();

        state.init_random(1000, true, 1);
        assert_ne!(state.random_context, 0);

        state.init_full_pipe();
        assert!(!state.filled_pipe);
        assert_eq!(state.full_bw, 0);

        state.init_round_counting(100);
        assert_eq!(state.round_start_pn, 100);

        state.reset_congestion_signals();
        assert!(!state.loss_in_round);

        state.reset_lower_bounds();
        assert_eq!(state.bw_lo, u64::MAX);
        assert_eq!(state.inflight_lo, u64::MAX);
    }
}
