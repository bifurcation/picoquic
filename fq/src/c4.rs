//! C4 congestion control algorithm.
//!
//! Translated from picoquic/c4.c.
//!
//! C4 is an experimental rate-based congestion control algorithm that
//! combines delay-based signals with loss and ECN feedback. It uses
//! probe cycles with adaptive intensity based on success/failure.

use crate::cc_common::CWIN_INITIAL;

// =============================================================================
// C4 Constants
// =============================================================================

/// Maximum delay threshold (25ms in microseconds).
pub const DELAY_THRESHOLD_MAX: u64 = 25_000;

/// Alpha value for neutral rate (100%).
pub const ALPHA_NEUTRAL_1024: u64 = 1024;

/// Alpha for recovery (93.75%).
pub const ALPHA_RECOVER_1024: u64 = 960;

/// Alpha for secondary recovery (87.5%).
pub const ALPHA_RECOVER2_1024: u64 = 896;

/// Alpha for cruising (100%).
pub const ALPHA_CRUISE_1024: u64 = 1024;

/// Alpha for pushing (125%).
pub const ALPHA_PUSH_1024: u64 = 1280;

/// Alpha for low push (106.25%).
pub const ALPHA_PUSH_LOW_1024: u64 = 1088;

/// Alpha for very low push (103.125%).
pub const ALPHA_PUSH_VERY_LOW_1024: u64 = 1056;

/// Alpha for initial phase (200%).
pub const ALPHA_INITIAL: u64 = 2048;

/// Alpha for previous low (93.75%).
pub const ALPHA_PREVIOUS_LOW: u64 = 960;

/// Beta for loss reduction (25%).
pub const BETA_LOSS_1024: u64 = 256;

/// Number of packets before considering losses in startup.
pub const NB_PACKETS_BEFORE_LOSS: u64 = 20;

/// Number of cruise cycles before push.
pub const NB_CRUISE_BEFORE_PUSH: u64 = 4;

/// RTT margin for delay calculation (15ms).
pub const RTT_MARGIN_DELAY: u64 = 15_000;

/// Minimum RTT threshold (1ms).
pub const MAX_RTT_MIN: u64 = 1_000;

/// Maximum jitter for RTT correction (250ms).
pub const MAX_JITTER: u64 = 250_000;

/// Shift for ECN alpha EWMA (g = 1/16).
pub const ECN_SHIFT_G: u32 = 4;

/// Maximum probe level.
pub const PROBE_LEVEL_MAX: i32 = 3;

/// Default probe level.
pub const PROBE_LEVEL_DEFAULT: i32 = 1;

/// Push rate by probe level.
pub const PUSH_RATE_BY_PROBE_LEVEL: [u64; 4] = [
    ALPHA_PUSH_VERY_LOW_1024,
    ALPHA_PUSH_LOW_1024,
    ALPHA_PUSH_1024,
    ALPHA_PUSH_1024,
];

// =============================================================================
// C4 Algorithm State
// =============================================================================

/// C4 algorithm states.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C4AlgState {
    Initial = 0,
    Recovery = 1,
    Cruising = 2,
    Pushing = 3,
}

/// Types of congestion events.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C4Congestion {
    None = 0,
    Delay = 1,
    Ecn = 2,
    Loss = 3,
}

/// C4 congestion control state.
#[derive(Debug, Clone)]
pub struct C4State {
    pub alg_state: C4AlgState,
    /// Nominal data rate (bytes per second).
    pub nominal_rate: u64,
    /// Estimate of queue-free max RTT.
    pub nominal_max_rtt: u64,
    /// CWND value during Initial phase.
    pub initial_cwnd: u64,
    /// Rough estimate of min RTT for buffer estimation.
    pub running_min_rtt: u64,
    /// Current alpha multiplier (scaled by 1024).
    pub alpha_1024_current: u64,
    /// Previous alpha multiplier (scaled by 1024).
    pub alpha_1024_previous: u64,
    /// Number of packets seen in startup.
    pub nb_packets_in_startup: u64,
    /// Sequence number of first packet in era.
    pub era_sequence: u64,
    /// Cruise periods left before push.
    pub nb_cruise_left_before_push: u64,
    /// Seeded CWIN from previous trials.
    pub seed_cwin: u64,
    /// Data rate from seeded CWIN.
    pub seed_rate: u64,

    /// Probe intensity level (0-3).
    pub probe_level: i32,
    /// Eras without rate increase.
    pub nb_eras_no_increase: i32,
    /// Previous push rate.
    pub push_rate_old: u64,
    /// Alpha during push phase.
    pub push_alpha: u64,

    /// Maximum RTT observed in era.
    pub era_max_rtt: u64,
    /// Minimum RTT observed in era.
    pub era_min_rtt: u64,

    /// Delay threshold for congestion.
    pub delay_threshold: u64,
    /// Recent excess delay above threshold.
    pub recent_delay_excess: u64,

    /// Last lost packet number for loss rate.
    pub last_lost_packet_number: u64,
    /// Smoothed packet loss rate.
    pub smoothed_drop_rate: f64,

    /// ECN alpha (average marking rate, scaled by 1024).
    pub ecn_alpha: u64,
    /// Running total of ECT1 marks.
    pub ecn_ect1: u64,
    /// Running total of CE marks.
    pub ecn_ce: u64,
    /// ECN threshold for congestion.
    pub ecn_threshold: u64,

    /// Whether congestion was notified this era.
    pub congestion_notified: bool,
    /// Whether push was not app-limited.
    pub push_was_not_limited: bool,
    /// Whether to use seeded CWIN.
    pub use_seed_cwin: bool,
    /// Whether we re-entered initial after jitter.
    pub initial_after_jitter: bool,
    /// Whether excess CE occurred after push.
    pub excess_ce_after_push: bool,
}

impl Default for C4State {
    fn default() -> Self {
        Self {
            alg_state: C4AlgState::Initial,
            nominal_rate: 0,
            nominal_max_rtt: 0,
            initial_cwnd: CWIN_INITIAL,
            running_min_rtt: u64::MAX,
            alpha_1024_current: ALPHA_INITIAL,
            alpha_1024_previous: ALPHA_INITIAL,
            nb_packets_in_startup: 0,
            era_sequence: 0,
            nb_cruise_left_before_push: 0,
            seed_cwin: 0,
            seed_rate: 0,
            probe_level: PROBE_LEVEL_DEFAULT,
            nb_eras_no_increase: 0,
            push_rate_old: 0,
            push_alpha: 0,
            era_max_rtt: 0,
            era_min_rtt: u64::MAX,
            delay_threshold: 0,
            recent_delay_excess: 0,
            last_lost_packet_number: 0,
            smoothed_drop_rate: 0.0,
            ecn_alpha: 0,
            ecn_ect1: 0,
            ecn_ce: 0,
            ecn_threshold: 0,
            congestion_notified: false,
            push_was_not_limited: false,
            use_seed_cwin: false,
            initial_after_jitter: false,
            excess_ce_after_push: false,
        }
    }
}

impl C4State {
    /// Multiply value by coefficient/1024.
    #[inline]
    fn mult1024(coef: u64, value: u64) -> u64 {
        (value * coef) >> 10
    }

    /// Compute sensitivity based on nominal rate (0-1024 scale).
    ///
    /// Higher sensitivity at higher data rates for fairness.
    pub fn sensitivity_1024(&self) -> u64 {
        if self.nominal_rate < 50_000 {
            0
        } else if self.nominal_rate > 10_000_000 {
            1024
        } else if self.nominal_rate < 1_000_000 {
            (self.nominal_rate - 50_000) * 963 / 950_000
        } else {
            963 + ((self.nominal_rate - 1_000_000) * 61 / 9_000_000)
        }
    }

    /// Compute delay threshold for congestion detection.
    pub fn compute_delay_threshold(&self) -> u64 {
        let sensitivity = self.sensitivity_1024();
        let fraction = 64 + Self::mult1024(1024 - sensitivity, 196);
        let delay = Self::mult1024(fraction, self.nominal_max_rtt);
        if delay > DELAY_THRESHOLD_MAX {
            DELAY_THRESHOLD_MAX
        } else {
            delay
        }
    }

    /// Compute ECN threshold for congestion detection.
    pub fn compute_ecn_threshold(&self) -> u64 {
        let sensitivity = self.sensitivity_1024();
        192 - Self::mult1024(sensitivity, 96)
    }

    /// Compute loss rate threshold for congestion detection.
    pub fn compute_loss_threshold(&self) -> f64 {
        let sensitivity = self.sensitivity_1024();
        let fraction = sensitivity as f64 / 1024.0;
        0.02 + 0.50 * (1.0 - fraction)
    }

    /// Reset state to initial values.
    pub fn reset(&mut self) -> u64 {
        *self = Self::default();
        self.enter_initial();
        CWIN_INITIAL
    }

    /// Enter initial (slow start) state.
    pub fn enter_initial(&mut self) {
        self.alg_state = C4AlgState::Initial;
        self.probe_level = PROBE_LEVEL_DEFAULT;
        self.alpha_1024_current = ALPHA_INITIAL;
        self.nb_packets_in_startup = 0;
        self.nb_eras_no_increase = 0;
        self.ecn_alpha = 0;
        self.growth_reset();
    }

    /// Reset era tracking.
    pub fn era_reset(&mut self, sequence_number: u64) {
        self.era_sequence = sequence_number;
        self.era_max_rtt = 0;
        self.era_min_rtt = u64::MAX;
        self.alpha_1024_previous = self.alpha_1024_current;
    }

    /// Reset growth tracking for push evaluation.
    pub fn growth_reset(&mut self) {
        self.congestion_notified = false;
        self.push_was_not_limited = false;
        self.push_rate_old = self.nominal_rate;
        self.push_alpha = self.alpha_1024_current;
    }

    /// Evaluate whether the previous push resulted in growth.
    pub fn growth_evaluate(&self) -> bool {
        if self.push_alpha > ALPHA_PUSH_LOW_1024 {
            let target_rate =
                (3 * self.push_rate_old + Self::mult1024(self.push_alpha, self.push_rate_old)) / 4;
            self.nominal_rate > target_rate
        } else {
            self.nominal_rate > self.push_rate_old && !self.congestion_notified
        }
    }

    /// Enter recovery state.
    pub fn enter_recovery(&mut self, sequence_number: u64, c_mode: C4Congestion) {
        if self.alg_state == C4AlgState::Initial {
            self.growth_reset();
        }

        if self.alg_state != C4AlgState::Recovery {
            self.excess_ce_after_push = c_mode == C4Congestion::Ecn;
            self.alg_state = C4AlgState::Recovery;
            self.era_reset(sequence_number);
            self.alpha_1024_current = ALPHA_RECOVER_1024;
        }
    }

    /// Exit recovery and transition to cruise or initial.
    pub fn exit_recovery(&mut self, sequence_number: u64) {
        let is_growing = self.growth_evaluate();
        if is_growing {
            if !self.excess_ce_after_push {
                self.probe_level += 1;
            }
        } else if self.push_was_not_limited {
            self.probe_level = 1;
            if self.excess_ce_after_push {
                self.probe_level = 0;
            }
        }
        self.growth_reset();
        self.recent_delay_excess = 0;
        self.smoothed_drop_rate = 0.0;
        self.ecn_alpha = 0;

        if self.probe_level > PROBE_LEVEL_MAX {
            self.enter_initial();
        } else {
            self.enter_cruise(sequence_number);
        }
    }

    /// Enter cruise state.
    pub fn enter_cruise(&mut self, sequence_number: u64) {
        self.era_reset(sequence_number);
        self.use_seed_cwin = false;

        if self.probe_level > PROBE_LEVEL_DEFAULT {
            self.nb_cruise_left_before_push = 0;
        } else if self.nb_cruise_left_before_push == 0 {
            self.nb_cruise_left_before_push = if self.probe_level == 0 {
                1
            } else {
                NB_CRUISE_BEFORE_PUSH
            };
        }
        self.alpha_1024_current = ALPHA_CRUISE_1024;
        self.alg_state = C4AlgState::Cruising;
    }

    /// Enter push state.
    pub fn enter_push(&mut self, sequence_number: u64) {
        self.alpha_1024_current = PUSH_RATE_BY_PROBE_LEVEL[self.probe_level.clamp(0, 3) as usize];
        self.push_alpha = self.alpha_1024_current;
        self.era_reset(sequence_number);
        self.alg_state = C4AlgState::Pushing;
    }

    /// Update RTT tracking.
    pub fn update_rtt(&mut self, rtt_measurement: u64) {
        if rtt_measurement > self.era_max_rtt {
            self.era_max_rtt = rtt_measurement;
        }
        if rtt_measurement < self.era_min_rtt {
            self.era_min_rtt = rtt_measurement;
        }
        if rtt_measurement < self.running_min_rtt {
            self.running_min_rtt = rtt_measurement;
        }

        if self.nominal_max_rtt == 0 {
            self.nominal_max_rtt = rtt_measurement;
            if self.nominal_max_rtt < MAX_RTT_MIN {
                self.nominal_max_rtt = MAX_RTT_MIN;
            }
            self.delay_threshold = self.compute_delay_threshold();
            self.recent_delay_excess = 0;
        } else {
            let target_rtt = self.nominal_max_rtt + self.delay_threshold;
            if rtt_measurement > target_rtt {
                self.recent_delay_excess = rtt_measurement - target_rtt;
            } else {
                self.recent_delay_excess = 0;
            }
        }
    }

    /// Update loss rate based on lost packet number.
    pub fn update_loss_rate(&mut self, lost_packet_number: u64) {
        const SMOOTHED_LOSS_SCOPE: u64 = 64;
        const SMOOTHED_LOSS_FACTOR: f64 = 0.015625; // 1/64

        let mut next_number = self.last_lost_packet_number;

        if lost_packet_number > next_number {
            if next_number + SMOOTHED_LOSS_SCOPE < lost_packet_number {
                next_number = lost_packet_number - SMOOTHED_LOSS_SCOPE;
            }

            while next_number < lost_packet_number {
                self.smoothed_drop_rate *= 1.0 - SMOOTHED_LOSS_FACTOR;
                next_number += 1;
            }

            self.smoothed_drop_rate += (1.0 - self.smoothed_drop_rate) * SMOOTHED_LOSS_FACTOR;
            self.last_lost_packet_number = lost_packet_number;
        }
    }

    /// Update ECN alpha based on marks.
    pub fn update_ecn_alpha(&mut self, ecn_ect1_total: u64, ecn_ce_total: u64) {
        let delta_ect1 = ecn_ect1_total.saturating_sub(self.ecn_ect1);
        let delta_ce = ecn_ce_total.saturating_sub(self.ecn_ce);

        self.ecn_ect1 = ecn_ect1_total;
        self.ecn_ce = ecn_ce_total;

        if delta_ce > 0 || delta_ect1 > 0 {
            let frac = (delta_ce * 1024) / (delta_ce + delta_ect1);

            if frac > self.ecn_alpha && frac >= 512 {
                self.ecn_alpha = frac;
            } else {
                let alpha_shifted = self.ecn_alpha << ECN_SHIFT_G;
                let alpha_shifted = alpha_shifted - self.ecn_alpha + frac;
                self.ecn_alpha = alpha_shifted >> ECN_SHIFT_G;
            }
        }
    }

    /// Compute beta reduction factor for congestion response.
    pub fn compute_beta(&self, c_mode: C4Congestion) -> u64 {
        match c_mode {
            C4Congestion::Loss => {
                (BETA_LOSS_1024 + Self::mult1024(self.sensitivity_1024(), BETA_LOSS_1024)) / 2
            }
            C4Congestion::Ecn => {
                if self.ecn_threshold > 0 {
                    let beta = (self.ecn_alpha.saturating_sub(self.ecn_threshold)) * 1024
                        / self.ecn_threshold;
                    if beta > BETA_LOSS_1024 {
                        BETA_LOSS_1024
                    } else {
                        beta
                    }
                } else {
                    BETA_LOSS_1024
                }
            }
            C4Congestion::Delay => {
                if self.delay_threshold > 0 {
                    let beta = self.recent_delay_excess * 1024 / self.delay_threshold;
                    if beta > BETA_LOSS_1024 {
                        BETA_LOSS_1024
                    } else {
                        beta
                    }
                } else {
                    0
                }
            }
            C4Congestion::None => 0,
        }
    }

    /// Apply congestion notification and reduce rate.
    ///
    /// Returns the beta value used for reduction.
    pub fn notify_congestion(&mut self, c_mode: C4Congestion, sequence_number: u64) -> u64 {
        self.congestion_notified = true;
        let beta = self.compute_beta(c_mode);

        if self.alg_state == C4AlgState::Recovery {
            if self.alpha_1024_current == ALPHA_RECOVER_1024 {
                self.alpha_1024_current = ALPHA_RECOVER2_1024;
                self.era_sequence = sequence_number;
            }
            if c_mode == C4Congestion::Ecn {
                self.excess_ce_after_push = true;
            }
        } else {
            if self.alg_state != C4AlgState::Pushing {
                self.nominal_rate -= Self::mult1024(beta, self.nominal_rate);
                if c_mode == C4Congestion::Loss {
                    self.nominal_max_rtt -= Self::mult1024(beta, self.nominal_max_rtt);
                    if self.nominal_max_rtt < MAX_RTT_MIN {
                        self.nominal_max_rtt = MAX_RTT_MIN;
                    }
                    self.delay_threshold = self.compute_delay_threshold();
                }
            }
            self.enter_recovery(sequence_number, c_mode);
        }

        if c_mode != C4Congestion::Delay {
            self.recent_delay_excess = 0;
        }

        beta
    }

    /// Seed the CWIN from external estimate.
    pub fn seed_cwin(&mut self, bytes_in_flight: u64) {
        if self.alg_state == C4AlgState::Initial {
            self.use_seed_cwin = true;
            self.seed_cwin = bytes_in_flight;
        }
    }

    /// Check if era has ended (first packet acknowledged).
    pub fn era_check(&self, lowest_not_ack: u64) -> bool {
        lowest_not_ack > self.era_sequence
    }

    /// Compute pacing rate based on current alpha and nominal rate.
    pub fn compute_pacing_rate(&self) -> u64 {
        Self::mult1024(self.alpha_1024_current, self.nominal_rate)
    }

    /// Compute target CWIN based on state.
    pub fn compute_target_cwin(&self, send_mtu: u64) -> u64 {
        let pacing_rate = self.compute_pacing_rate();
        let mut target_cwin = CWIN_INITIAL;

        if self.nominal_max_rtt != 0 && self.nominal_rate != 0 {
            target_cwin = bytes_from_rate(self.nominal_max_rtt, pacing_rate);
        }

        if self.alg_state == C4AlgState::Initial {
            if target_cwin < self.initial_cwnd {
                target_cwin = self.initial_cwnd;
            }
        } else {
            let delta_rtt_target = if self.nominal_max_rtt < 4 * RTT_MARGIN_DELAY {
                self.nominal_max_rtt / 4
            } else {
                RTT_MARGIN_DELAY
            };
            target_cwin += bytes_from_rate(delta_rtt_target, pacing_rate);

            if self.alg_state == C4AlgState::Pushing && self.alpha_1024_current > 1024 {
                let delta_alpha = self.alpha_1024_current - 1024;
                let delta_rate = Self::mult1024(delta_alpha, self.nominal_rate);
                let delta_cwin = bytes_from_rate(self.nominal_max_rtt, delta_rate);
                if delta_cwin < send_mtu {
                    target_cwin += send_mtu - delta_cwin;
                }
            }
        }

        target_cwin
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Compute bytes from rate and time interval.
#[inline]
pub fn bytes_from_rate(interval_us: u64, rate_bytes_per_sec: u64) -> u64 {
    (rate_bytes_per_sec as u128 * interval_us as u128 / 1_000_000) as u64
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c4_state_default() {
        let state = C4State::default();
        assert_eq!(state.alg_state, C4AlgState::Initial);
        assert_eq!(state.alpha_1024_current, ALPHA_INITIAL);
        assert_eq!(state.probe_level, PROBE_LEVEL_DEFAULT);
        assert_eq!(state.running_min_rtt, u64::MAX);
    }

    #[test]
    fn test_c4_reset() {
        let mut state = C4State::default();
        state.alg_state = C4AlgState::Cruising;
        state.nominal_rate = 1_000_000;

        let cwin = state.reset();

        assert_eq!(state.alg_state, C4AlgState::Initial);
        assert_eq!(state.alpha_1024_current, ALPHA_INITIAL);
        assert_eq!(cwin, CWIN_INITIAL);
    }

    #[test]
    fn test_sensitivity_low_rate() {
        let mut state = C4State::default();
        state.nominal_rate = 10_000;
        assert_eq!(state.sensitivity_1024(), 0);

        state.nominal_rate = 50_000;
        assert_eq!(state.sensitivity_1024(), 0);
    }

    #[test]
    fn test_sensitivity_high_rate() {
        let mut state = C4State::default();
        state.nominal_rate = 10_000_000;
        assert_eq!(state.sensitivity_1024(), 1024);

        state.nominal_rate = 20_000_000;
        assert_eq!(state.sensitivity_1024(), 1024);
    }

    #[test]
    fn test_sensitivity_medium_rate() {
        let mut state = C4State::default();
        state.nominal_rate = 500_000;
        let sensitivity = state.sensitivity_1024();
        assert!(sensitivity > 0);
        assert!(sensitivity < 963);
    }

    #[test]
    fn test_compute_delay_threshold() {
        let mut state = C4State::default();
        state.nominal_rate = 1_000_000;
        state.nominal_max_rtt = 100_000;

        let threshold = state.compute_delay_threshold();
        assert!(threshold > 0);
        assert!(threshold <= DELAY_THRESHOLD_MAX);
    }

    #[test]
    fn test_compute_ecn_threshold() {
        let mut state = C4State::default();

        // Low rate = low sensitivity = high threshold
        state.nominal_rate = 10_000;
        let low_threshold = state.compute_ecn_threshold();

        // High rate = high sensitivity = low threshold
        state.nominal_rate = 10_000_000;
        let high_threshold = state.compute_ecn_threshold();

        assert!(low_threshold > high_threshold);
    }

    #[test]
    fn test_compute_loss_threshold() {
        let mut state = C4State::default();

        // Low rate = low sensitivity = high loss threshold
        state.nominal_rate = 10_000;
        let low_threshold = state.compute_loss_threshold();

        // High rate = high sensitivity = low loss threshold
        state.nominal_rate = 10_000_000;
        let high_threshold = state.compute_loss_threshold();

        assert!(low_threshold > high_threshold);
        assert!(high_threshold >= 0.02);
        assert!(low_threshold <= 0.52);
    }

    #[test]
    fn test_enter_recovery() {
        let mut state = C4State::default();
        state.alg_state = C4AlgState::Cruising;

        state.enter_recovery(100, C4Congestion::Loss);

        assert_eq!(state.alg_state, C4AlgState::Recovery);
        assert_eq!(state.alpha_1024_current, ALPHA_RECOVER_1024);
        assert_eq!(state.era_sequence, 100);
    }

    #[test]
    fn test_enter_recovery_already_in_recovery() {
        let mut state = C4State::default();
        state.alg_state = C4AlgState::Recovery;
        state.alpha_1024_current = ALPHA_RECOVER2_1024;

        state.enter_recovery(200, C4Congestion::Loss);

        // Should not change if already in recovery
        assert_eq!(state.alpha_1024_current, ALPHA_RECOVER2_1024);
    }

    #[test]
    fn test_enter_cruise() {
        let mut state = C4State::default();
        state.probe_level = PROBE_LEVEL_DEFAULT;

        state.enter_cruise(100);

        assert_eq!(state.alg_state, C4AlgState::Cruising);
        assert_eq!(state.alpha_1024_current, ALPHA_CRUISE_1024);
        assert_eq!(state.nb_cruise_left_before_push, NB_CRUISE_BEFORE_PUSH);
    }

    #[test]
    fn test_enter_push() {
        let mut state = C4State::default();
        state.probe_level = 2;

        state.enter_push(100);

        assert_eq!(state.alg_state, C4AlgState::Pushing);
        assert_eq!(state.alpha_1024_current, PUSH_RATE_BY_PROBE_LEVEL[2]);
        assert_eq!(state.push_alpha, PUSH_RATE_BY_PROBE_LEVEL[2]);
    }

    #[test]
    fn test_update_rtt() {
        let mut state = C4State::default();
        state.nominal_max_rtt = 50_000;
        state.delay_threshold = 10_000;

        // RTT below threshold - no excess
        state.update_rtt(55_000);
        assert_eq!(state.recent_delay_excess, 0);

        // RTT above threshold - excess
        state.update_rtt(70_000);
        assert_eq!(state.recent_delay_excess, 10_000);
    }

    #[test]
    fn test_update_loss_rate() {
        let mut state = C4State::default();
        state.last_lost_packet_number = 0;
        state.smoothed_drop_rate = 0.0;

        state.update_loss_rate(10);
        assert!(state.smoothed_drop_rate > 0.0);
        assert_eq!(state.last_lost_packet_number, 10);
    }

    #[test]
    fn test_update_ecn_alpha() {
        let mut state = C4State::default();
        state.ecn_ect1 = 0;
        state.ecn_ce = 0;
        state.ecn_alpha = 0;

        // 50% CE marks
        state.update_ecn_alpha(50, 50);
        assert_eq!(state.ecn_alpha, 512);
    }

    #[test]
    fn test_compute_beta_loss() {
        let mut state = C4State::default();
        state.nominal_rate = 1_000_000;

        let beta = state.compute_beta(C4Congestion::Loss);
        assert!(beta > 0);
        assert!(beta <= BETA_LOSS_1024);
    }

    #[test]
    fn test_growth_evaluate() {
        let mut state = C4State::default();
        state.push_alpha = ALPHA_PUSH_1024;
        state.push_rate_old = 1_000_000;
        state.nominal_rate = 1_500_000;

        assert!(state.growth_evaluate());

        state.nominal_rate = 900_000;
        assert!(!state.growth_evaluate());
    }

    #[test]
    fn test_seed_cwin() {
        let mut state = C4State::default();
        state.alg_state = C4AlgState::Initial;

        state.seed_cwin(100_000);

        assert!(state.use_seed_cwin);
        assert_eq!(state.seed_cwin, 100_000);
    }

    #[test]
    fn test_seed_cwin_not_in_initial() {
        let mut state = C4State::default();
        state.alg_state = C4AlgState::Cruising;

        state.seed_cwin(100_000);

        assert!(!state.use_seed_cwin);
    }

    #[test]
    fn test_era_check() {
        let mut state = C4State::default();
        state.era_sequence = 50;

        assert!(!state.era_check(40));
        assert!(!state.era_check(50));
        assert!(state.era_check(51));
    }

    #[test]
    fn test_compute_pacing_rate() {
        let mut state = C4State::default();
        state.nominal_rate = 1_000_000;
        state.alpha_1024_current = 1024; // 100%

        assert_eq!(state.compute_pacing_rate(), 1_000_000);

        state.alpha_1024_current = 1280; // 125%
        assert_eq!(state.compute_pacing_rate(), 1_250_000);
    }

    #[test]
    fn test_bytes_from_rate() {
        // 1 MB/s for 100ms = 100KB
        assert_eq!(bytes_from_rate(100_000, 1_000_000), 100_000);
    }

    #[test]
    fn test_constants() {
        assert_eq!(ALPHA_NEUTRAL_1024, 1024);
        assert_eq!(ALPHA_PUSH_1024, 1280);
        assert_eq!(BETA_LOSS_1024, 256);
        assert_eq!(PROBE_LEVEL_MAX, 3);
    }
}
