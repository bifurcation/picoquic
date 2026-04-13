//! FastCC congestion control algorithm.
//!
//! Translated from picoquic/fastcc.c.
//!
//! FastCC is a delay-based congestion control algorithm that aims to
//! maintain low queuing delay by monitoring RTT variations.

use crate::cc_common::{MinMaxRtt, CWIN_INITIAL, CWIN_MINIMUM};

// =============================================================================
// FastCC Constants
// =============================================================================

/// Minimum ACK delay for bandwidth measurement (microseconds).
pub const MIN_ACK_DELAY_FOR_BANDWIDTH: u64 = 5000;

/// Bandwidth fraction for pacing.
pub const BANDWIDTH_FRACTION: f64 = 0.5;

/// Number of RTT events before triggering delay-based congestion response.
pub const REPEAT_THRESHOLD: i32 = 4;

/// Multiplicative decrease factor for delay-based congestion.
pub const BETA: f64 = 0.125;

/// Multiplicative decrease factor for heavy loss.
pub const BETA_HEAVY_LOSS: f64 = 0.5;

/// Alpha factor for window growth evaluation.
pub const EVAL_ALPHA: f64 = 0.25;

/// Maximum delay threshold (microseconds).
pub const DELAY_THRESHOLD_MAX: u64 = 25000;

/// Number of periods to track for RTT minimum.
pub const NB_PERIOD: usize = 6;

/// Duration of one period (microseconds).
pub const PERIOD: u64 = 1_000_000;

// =============================================================================
// FastCC Algorithm State
// =============================================================================

/// FastCC algorithm states.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastCcAlgState {
    /// Initial state - aggressive window growth.
    Initial = 0,
    /// Evaluation state - moderate window growth based on delay.
    Eval = 1,
    /// Freeze state - no window growth during recovery.
    Freeze = 2,
}

/// FastCC congestion control state.
#[derive(Debug, Clone)]
pub struct FastCcState {
    pub alg_state: FastCcAlgState,
    /// When to exit the freeze state.
    pub end_of_freeze: u64,
    /// Last acknowledgment time.
    pub last_ack_time: u64,
    /// ACK interval.
    pub ack_interval: u64,
    /// Bytes acknowledged.
    pub nb_bytes_ack: u64,
    /// Accumulate byte count until RTT measured.
    pub nb_bytes_ack_since_rtt: u64,
    /// End of current epoch.
    pub end_of_epoch: u64,
    /// Recovery sequence number.
    pub recovery_sequence: u64,
    /// Minimum RTT observed.
    pub rtt_min: u64,
    /// Delay threshold for congestion detection.
    pub delay_threshold: u64,
    /// Rolling minimum RTT for this epoch.
    pub rolling_rtt_min: u64,
    /// Last RTT minimums for each period.
    pub last_rtt_min: [u64; NB_PERIOD],
    /// Number of congestion events.
    pub nb_cc_events: i32,
    /// Whether last freeze was due to timeout.
    pub last_freeze_was_timeout: bool,
    /// Whether last freeze was not due to delay.
    pub last_freeze_was_not_delay: bool,
    /// Whether rtt_min is trusted.
    pub rtt_min_is_trusted: bool,
    /// RTT filter for HyStart loss detection.
    pub rtt_filter: MinMaxRtt,
}

impl Default for FastCcState {
    fn default() -> Self {
        Self {
            alg_state: FastCcAlgState::Initial,
            end_of_freeze: 0,
            last_ack_time: 0,
            ack_interval: 0,
            nb_bytes_ack: 0,
            nb_bytes_ack_since_rtt: 0,
            end_of_epoch: 0,
            recovery_sequence: 0,
            rtt_min: 0,
            delay_threshold: 0,
            rolling_rtt_min: 0,
            last_rtt_min: [0; NB_PERIOD],
            nb_cc_events: 0,
            last_freeze_was_timeout: false,
            last_freeze_was_not_delay: false,
            rtt_min_is_trusted: false,
            rtt_filter: MinMaxRtt::default(),
        }
    }
}

impl FastCcState {
    /// Compute delay threshold from minimum RTT.
    ///
    /// Returns rtt_min / 8, capped at DELAY_THRESHOLD_MAX.
    pub fn compute_delay_threshold(rtt_min: u64) -> u64 {
        let delay = rtt_min / 8;
        if delay > DELAY_THRESHOLD_MAX {
            DELAY_THRESHOLD_MAX
        } else {
            delay
        }
    }

    /// Reset FastCC state to initial values.
    ///
    /// Parameters:
    /// - smoothed_rtt: Current smoothed RTT from the path
    /// - current_time: Current timestamp
    ///
    /// Returns the initial congestion window.
    pub fn reset(&mut self, smoothed_rtt: u64, current_time: u64) -> u64 {
        *self = Self::default();
        self.alg_state = FastCcAlgState::Initial;
        self.rtt_min = smoothed_rtt;
        self.rolling_rtt_min = self.rtt_min;
        self.delay_threshold = Self::compute_delay_threshold(self.rtt_min);
        self.end_of_epoch = current_time + PERIOD;
        CWIN_INITIAL
    }

    /// Seed congestion window from bandwidth estimate.
    ///
    /// Only applies in Initial state. Returns new cwin if updated.
    pub fn seed_cwin(&mut self, cwin: u64, bytes_in_flight: u64) -> u64 {
        if self.alg_state == FastCcAlgState::Initial && cwin < bytes_in_flight {
            bytes_in_flight
        } else {
            cwin
        }
    }

    /// Handle congestion event (loss, ECN, or delay).
    ///
    /// Parameters:
    /// - cwin: Current congestion window
    /// - current_time: Current timestamp
    /// - sequence_number: Current send sequence number
    /// - is_delay: Whether this is a delay-based event
    /// - is_timeout: Whether this is a timeout event
    ///
    /// Returns the new congestion window.
    pub fn notify_congestion(
        &mut self,
        cwin: u64,
        current_time: u64,
        sequence_number: u64,
        is_delay: bool,
        is_timeout: bool,
    ) -> u64 {
        // Don't treat additional events during same freeze interval
        if self.alg_state == FastCcAlgState::Freeze
            && (!is_timeout || !self.last_freeze_was_timeout)
            && (!is_delay || !self.last_freeze_was_not_delay)
        {
            return cwin;
        }

        self.last_freeze_was_not_delay = !is_delay;
        self.last_freeze_was_timeout = is_timeout;
        self.alg_state = FastCcAlgState::Freeze;
        self.end_of_freeze = current_time + self.rtt_min;
        self.recovery_sequence = sequence_number;
        self.nb_cc_events = 0;

        let mut new_cwin = if is_delay {
            cwin - (BETA * cwin as f64) as u64
        } else {
            cwin / 2
        };

        if is_timeout || new_cwin < CWIN_MINIMUM {
            new_cwin = CWIN_MINIMUM;
        }

        new_cwin
    }

    /// Check if we should exit freeze state.
    ///
    /// Parameters:
    /// - current_time: Current timestamp
    /// - ack_number: Highest acknowledged packet number
    ///
    /// Returns true if freeze state was exited.
    pub fn check_exit_freeze(&mut self, current_time: u64, ack_number: u64) -> bool {
        if self.alg_state == FastCcAlgState::Freeze
            && (current_time > self.end_of_freeze || self.recovery_sequence <= ack_number)
        {
            if self.last_freeze_was_timeout {
                self.alg_state = FastCcAlgState::Initial;
            } else {
                self.alg_state = FastCcAlgState::Eval;
            }
            self.last_freeze_was_not_delay = false;
            self.last_freeze_was_timeout = false;
            self.nb_cc_events = 0;
            self.nb_bytes_ack_since_rtt = 0;
            true
        } else {
            false
        }
    }

    /// Process acknowledgment notification.
    ///
    /// Returns the number of bytes to add to nb_bytes_ack_since_rtt.
    pub fn on_ack(&mut self, nb_bytes_acknowledged: u64) -> u64 {
        if self.alg_state != FastCcAlgState::Freeze {
            self.nb_bytes_ack_since_rtt += nb_bytes_acknowledged;
            nb_bytes_acknowledged
        } else {
            0
        }
    }

    /// Process RTT measurement notification.
    ///
    /// Parameters:
    /// - cwin: Current congestion window
    /// - rtt_measurement: RTT sample
    /// - current_time: Current timestamp
    /// - is_app_limited: Whether sender is application-limited
    ///
    /// Returns (new_cwin, should_trigger_congestion).
    pub fn on_rtt_measurement(
        &mut self,
        cwin: u64,
        rtt_measurement: u64,
        current_time: u64,
        is_app_limited: bool,
    ) -> (u64, bool) {
        self.rtt_filter.filter_rtt_min_max(rtt_measurement);

        if self.rtt_filter.is_init {
            // Manage epochs
            if current_time > self.end_of_epoch {
                // End of epoch: reset min RTT to min of remembered periods
                self.rtt_min = u64::MAX;
                for i in (1..NB_PERIOD).rev() {
                    self.last_rtt_min[i] = self.last_rtt_min[i - 1];
                    if self.last_rtt_min[i] > 0 && self.last_rtt_min[i] < self.rtt_min {
                        self.rtt_min = self.last_rtt_min[i];
                    }
                }
                self.delay_threshold = Self::compute_delay_threshold(self.rtt_min);
                self.last_rtt_min[0] = self.rolling_rtt_min;
                self.rolling_rtt_min = self.rtt_filter.sample_max;
                self.end_of_epoch = current_time + PERIOD;
            } else if self.rtt_filter.sample_max < self.rolling_rtt_min || self.rolling_rtt_min == 0
            {
                // Update rolling minimum
                self.rolling_rtt_min = self.rtt_filter.sample_max;
                if self.rolling_rtt_min < self.rtt_min {
                    self.rtt_min = self.rolling_rtt_min;
                }
            }
        }

        if self.alg_state == FastCcAlgState::Freeze {
            return (cwin, false);
        }

        let delta_rtt = if rtt_measurement < self.rtt_min {
            self.delay_threshold = Self::compute_delay_threshold(self.rtt_min);
            0
        } else if self.rtt_min_is_trusted {
            rtt_measurement - self.rtt_min
        } else {
            self.rtt_min = rtt_measurement;
            self.rolling_rtt_min = rtt_measurement;
            self.rtt_min_is_trusted = true;
            0
        };

        if delta_rtt < self.delay_threshold {
            let alpha = if self.alg_state != FastCcAlgState::Initial {
                (1.0 - (delta_rtt as f64 / self.delay_threshold as f64)) * EVAL_ALPHA
            } else {
                1.0
            };

            self.nb_cc_events = 0;

            // Increase window if not app-limited
            let new_cwin = if !is_app_limited {
                cwin + (alpha * self.nb_bytes_ack_since_rtt as f64) as u64
            } else {
                cwin
            };
            self.nb_bytes_ack_since_rtt = 0;

            (new_cwin, false)
        } else {
            // May be congested
            self.nb_cc_events += 1;
            if self.nb_cc_events >= REPEAT_THRESHOLD {
                (cwin, true) // Should trigger congestion
            } else {
                (cwin, false)
            }
        }
    }

    /// Handle spurious repeat notification.
    pub fn on_spurious_repeat(&mut self) {
        if self.nb_cc_events > 0 {
            self.nb_cc_events -= 1;
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_delay_threshold() {
        // rtt_min / 8
        assert_eq!(FastCcState::compute_delay_threshold(80000), 10000);
        assert_eq!(FastCcState::compute_delay_threshold(160000), 20000);
        // Capped at max
        assert_eq!(
            FastCcState::compute_delay_threshold(400000),
            DELAY_THRESHOLD_MAX
        );
    }

    #[test]
    fn test_fastcc_state_default() {
        let state = FastCcState::default();
        assert_eq!(state.alg_state, FastCcAlgState::Initial);
        assert_eq!(state.rtt_min, 0);
        assert_eq!(state.nb_cc_events, 0);
    }

    #[test]
    fn test_fastcc_reset() {
        let mut state = FastCcState::default();
        state.alg_state = FastCcAlgState::Freeze;
        state.nb_cc_events = 5;

        let cwin = state.reset(100_000, 1_000_000);

        assert_eq!(state.alg_state, FastCcAlgState::Initial);
        assert_eq!(state.rtt_min, 100_000);
        assert_eq!(state.delay_threshold, 12500); // 100000/8
        assert_eq!(state.end_of_epoch, 1_000_000 + PERIOD);
        assert_eq!(cwin, CWIN_INITIAL);
    }

    #[test]
    fn test_fastcc_seed_cwin() {
        let mut state = FastCcState::default();
        state.alg_state = FastCcAlgState::Initial;

        // Should update if bytes_in_flight > cwin
        let new_cwin = state.seed_cwin(10_000, 50_000);
        assert_eq!(new_cwin, 50_000);

        // Should not update if cwin >= bytes_in_flight
        let new_cwin = state.seed_cwin(50_000, 10_000);
        assert_eq!(new_cwin, 50_000);

        // Should not update in non-Initial state
        state.alg_state = FastCcAlgState::Eval;
        let new_cwin = state.seed_cwin(10_000, 50_000);
        assert_eq!(new_cwin, 10_000);
    }

    #[test]
    fn test_fastcc_notify_congestion_delay() {
        let mut state = FastCcState::default();
        state.rtt_min = 100_000;

        let cwin = state.notify_congestion(100_000, 1_000, 50, true, false);

        // Delay: cwin -= BETA * cwin = 100000 - 12500 = 87500
        assert_eq!(cwin, 87_500);
        assert_eq!(state.alg_state, FastCcAlgState::Freeze);
        assert_eq!(state.end_of_freeze, 1_000 + 100_000);
        assert_eq!(state.recovery_sequence, 50);
    }

    #[test]
    fn test_fastcc_notify_congestion_loss() {
        let mut state = FastCcState::default();
        state.rtt_min = 100_000;

        let cwin = state.notify_congestion(100_000, 1_000, 50, false, false);

        // Loss: cwin = cwin / 2 = 50000
        assert_eq!(cwin, 50_000);
        assert_eq!(state.alg_state, FastCcAlgState::Freeze);
    }

    #[test]
    fn test_fastcc_notify_congestion_timeout() {
        let mut state = FastCcState::default();
        state.rtt_min = 100_000;

        let cwin = state.notify_congestion(100_000, 1_000, 50, false, true);

        // Timeout: cwin = CWIN_MINIMUM
        assert_eq!(cwin, CWIN_MINIMUM);
        assert!(state.last_freeze_was_timeout);
    }

    #[test]
    fn test_fastcc_check_exit_freeze() {
        let mut state = FastCcState::default();
        state.alg_state = FastCcAlgState::Freeze;
        state.end_of_freeze = 1000;
        state.recovery_sequence = 50;
        state.last_freeze_was_timeout = false;

        // Should exit after end_of_freeze
        assert!(state.check_exit_freeze(1001, 40));
        assert_eq!(state.alg_state, FastCcAlgState::Eval);
    }

    #[test]
    fn test_fastcc_check_exit_freeze_timeout() {
        let mut state = FastCcState::default();
        state.alg_state = FastCcAlgState::Freeze;
        state.end_of_freeze = 1000;
        state.recovery_sequence = 50;
        state.last_freeze_was_timeout = true;

        // Should return to Initial after timeout freeze
        assert!(state.check_exit_freeze(1001, 40));
        assert_eq!(state.alg_state, FastCcAlgState::Initial);
    }

    #[test]
    fn test_fastcc_on_ack() {
        let mut state = FastCcState::default();
        state.alg_state = FastCcAlgState::Eval;

        let bytes = state.on_ack(1000);
        assert_eq!(bytes, 1000);
        assert_eq!(state.nb_bytes_ack_since_rtt, 1000);

        // Should not accumulate in Freeze state
        state.alg_state = FastCcAlgState::Freeze;
        state.nb_bytes_ack_since_rtt = 0;
        let bytes = state.on_ack(1000);
        assert_eq!(bytes, 0);
        assert_eq!(state.nb_bytes_ack_since_rtt, 0);
    }

    #[test]
    fn test_fastcc_on_rtt_measurement_low_delay() {
        let mut state = FastCcState::default();
        state.alg_state = FastCcAlgState::Eval;
        state.rtt_min = 50_000;
        state.rtt_min_is_trusted = true;
        state.delay_threshold = FastCcState::compute_delay_threshold(50_000);
        state.nb_bytes_ack_since_rtt = 1000;

        // Low delay: should increase window
        let (cwin, should_congestion) = state.on_rtt_measurement(50_000, 51_000, 1000, false);

        assert!(!should_congestion);
        assert!(cwin > 50_000);
        assert_eq!(state.nb_bytes_ack_since_rtt, 0);
    }

    #[test]
    fn test_fastcc_on_rtt_measurement_high_delay() {
        let mut state = FastCcState::default();
        state.alg_state = FastCcAlgState::Eval;
        state.rtt_min = 50_000;
        state.rtt_min_is_trusted = true;
        state.delay_threshold = 6250; // 50000/8

        // High delay: should count events
        for i in 0..REPEAT_THRESHOLD {
            let (_, should_congestion) = state.on_rtt_measurement(50_000, 100_000, 1000, false);
            if i < REPEAT_THRESHOLD - 1 {
                assert!(!should_congestion);
            } else {
                assert!(should_congestion);
            }
        }
    }

    #[test]
    fn test_fastcc_on_spurious_repeat() {
        let mut state = FastCcState::default();
        state.nb_cc_events = 3;

        state.on_spurious_repeat();
        assert_eq!(state.nb_cc_events, 2);

        state.nb_cc_events = 0;
        state.on_spurious_repeat();
        assert_eq!(state.nb_cc_events, 0); // Can't go negative
    }
}
