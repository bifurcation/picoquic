//! CUBIC congestion control algorithm.
//!
//! Translated from picoquic/cubic.c.
//!
//! CUBIC is a TCP-friendly congestion control algorithm that uses a cubic
//! function for window growth instead of the linear function used by NewReno.

use crate::cc_common::{CongestionNotification, MinMaxRtt, CWIN_INITIAL, CWIN_MINIMUM};

// =============================================================================
// CUBIC Constants
// =============================================================================

/// CUBIC growth constant C (determines aggressiveness).
pub const CUBIC_C: f64 = 0.4;

/// CUBIC beta for ECN (multiplicative decrease factor).
pub const CUBIC_BETA_ECN: f64 = 7.0 / 8.0; // 0.875

/// CUBIC beta for loss (multiplicative decrease factor).
pub const CUBIC_BETA: f64 = 3.0 / 4.0; // 0.75

// =============================================================================
// CUBIC Algorithm State
// =============================================================================

/// CUBIC algorithm states.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CubicAlgState {
    SlowStart = 0,
    Recovery = 1,
    CongestionAvoidance = 2,
}

/// CUBIC congestion control state.
#[derive(Debug, Clone)]
pub struct CubicState {
    pub alg_state: CubicAlgState,
    pub recovery_sequence: u64,
    pub start_of_epoch: u64,
    pub previous_start_of_epoch: u64,
    pub previous_alg_state: CubicAlgState,
    pub previous_ssthresh: u64,
    pub previous_cwin: u64,
    /// K: Time to reach W_max from the current window.
    pub k: f64,
    /// W_max: Window size at last congestion event (in MSS units).
    pub w_max: f64,
    /// W_last_max: Previous W_max for fast convergence.
    pub w_last_max: f64,
    /// W_reno: Parallel Reno window for TCP-friendliness.
    pub w_reno: f64,
    /// Slow start threshold.
    pub ssthresh: u64,
    /// RTT filter for HyStart loss detection.
    pub rtt_filter: MinMaxRtt,
}

impl Default for CubicState {
    fn default() -> Self {
        Self {
            alg_state: CubicAlgState::SlowStart,
            recovery_sequence: 0,
            start_of_epoch: 0,
            previous_start_of_epoch: 0,
            previous_alg_state: CubicAlgState::SlowStart,
            previous_ssthresh: u64::MAX,
            previous_cwin: CWIN_INITIAL,
            k: 0.0,
            w_max: f64::MAX / CWIN_INITIAL as f64,
            w_last_max: f64::MAX / CWIN_INITIAL as f64,
            w_reno: CWIN_INITIAL as f64,
            ssthresh: u64::MAX,
            rtt_filter: MinMaxRtt::default(),
        }
    }
}

impl CubicState {
    /// Reset CUBIC state to initial values.
    pub fn reset(&mut self, send_mtu: u64, current_time: u64) {
        self.rtt_filter = MinMaxRtt::default();
        self.alg_state = CubicAlgState::SlowStart;
        self.ssthresh = u64::MAX;
        self.w_last_max = self.ssthresh as f64 / send_mtu as f64;
        self.w_max = self.w_last_max;
        self.start_of_epoch = current_time;
        self.previous_start_of_epoch = current_time;
        self.previous_alg_state = self.alg_state;
        self.previous_cwin = CWIN_INITIAL;
        self.previous_ssthresh = u64::MAX;
        self.w_reno = CWIN_INITIAL as f64;
        self.recovery_sequence = 0;
        self.k = 0.0;
    }

    /// Compute K for entering congestion avoidance.
    ///
    /// K is the time (in seconds) it takes to grow from the reduced window
    /// back to W_max after a congestion event.
    pub fn compute_k(&mut self) {
        self.k = cubic_root(self.w_max * (1.0 - CUBIC_BETA_ECN) / CUBIC_C);
    }

    /// Enter congestion avoidance state.
    pub fn enter_avoidance(&mut self, current_time: u64) {
        self.compute_k();
        self.alg_state = CubicAlgState::CongestionAvoidance;
        self.start_of_epoch = current_time;
        self.previous_start_of_epoch = self.start_of_epoch;
    }

    /// Compute W_cubic(t) = C * (t - K)^3 + W_max
    ///
    /// This is the CUBIC window growth function.
    pub fn w_cubic(&self, current_time: u64) -> f64 {
        let delta_t_sec = ((current_time - self.start_of_epoch) as f64 / 1_000_000.0) - self.k;
        (CUBIC_C * delta_t_sec * delta_t_sec * delta_t_sec) + self.w_max
    }

    /// Enter recovery state after congestion event.
    ///
    /// Parameters:
    /// - notification: Type of congestion event
    /// - cwin: Current congestion window
    /// - send_mtu: Path MTU
    /// - sequence_number: Current send sequence number
    /// - current_time: Current timestamp
    ///
    /// Returns the new congestion window.
    pub fn enter_recovery(
        &mut self,
        notification: CongestionNotification,
        cwin: u64,
        send_mtu: u64,
        sequence_number: u64,
        current_time: u64,
    ) -> u64 {
        self.recovery_sequence = sequence_number;
        self.previous_start_of_epoch = self.start_of_epoch;
        self.previous_alg_state = self.alg_state;
        self.previous_ssthresh = self.ssthresh;
        self.previous_cwin = cwin;

        // Update W_max to current window (in MSS units)
        self.w_max = cwin as f64 / send_mtu as f64;

        // Apply fast convergence
        if self.w_max < self.w_last_max {
            self.w_last_max = self.w_max;
            self.w_max *= CUBIC_BETA_ECN;
        } else {
            self.w_last_max = self.w_max;
        }

        // Compute new ssthresh
        self.ssthresh = (self.w_max * CUBIC_BETA_ECN * send_mtu as f64) as u64;

        if self.ssthresh < CWIN_MINIMUM {
            // If things are that bad, fall back to slow start
            self.alg_state = CubicAlgState::SlowStart;
            self.ssthresh = u64::MAX;
            self.previous_start_of_epoch = self.start_of_epoch;
            self.start_of_epoch = current_time;
            self.w_reno = CWIN_MINIMUM as f64;
            CWIN_MINIMUM
        } else if notification == CongestionNotification::Timeout {
            self.previous_start_of_epoch = self.start_of_epoch;
            self.start_of_epoch = current_time;
            self.alg_state = CubicAlgState::SlowStart;
            CWIN_MINIMUM
        } else {
            // Enter congestion avoidance immediately
            self.enter_avoidance(current_time);

            // Compute the initial window for both Reno and Cubic
            let w_cubic = self.w_cubic(current_time);
            let win_cubic = (w_cubic * send_mtu as f64) as u64;
            self.w_reno = cwin as f64 / 2.0;

            // The formulas guarantee w_cubic >= w_reno at this point
            win_cubic
        }
    }

    /// Handle spurious loss detection - restore previous state.
    pub fn correct_spurious(&mut self, send_mtu: u64, current_time: u64) -> Option<u64> {
        if self.ssthresh == u64::MAX {
            return None;
        }

        self.w_max = self.w_last_max;
        self.start_of_epoch = self.previous_start_of_epoch;
        self.alg_state = self.previous_alg_state;

        if self.alg_state != CubicAlgState::SlowStart {
            self.enter_avoidance(self.previous_start_of_epoch);
            let w_cubic = self.w_cubic(current_time);
            self.w_reno = w_cubic * send_mtu as f64;
            self.ssthresh = (self.w_max * CUBIC_BETA * send_mtu as f64) as u64;
            Some(self.w_reno as u64)
        } else {
            self.ssthresh = self.previous_ssthresh;
            self.w_reno = self.previous_cwin as f64;
            Some(self.previous_cwin)
        }
    }
}

// =============================================================================
// Pure Mathematical Functions
// =============================================================================

/// Compute the cube root of x.
///
/// Used to compute K in the CUBIC algorithm.
/// Uses Rust's native cbrt() which is more numerically stable than
/// the custom Newton-Raphson implementation in the C code.
#[inline]
pub fn cubic_root(x: f64) -> f64 {
    x.cbrt()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubic_root_perfect_cubes() {
        // Test perfect cubes
        assert!((cubic_root(1.0) - 1.0).abs() < 0.001);
        assert!((cubic_root(8.0) - 2.0).abs() < 0.001);
        assert!((cubic_root(27.0) - 3.0).abs() < 0.001);
        assert!((cubic_root(64.0) - 4.0).abs() < 0.001);
        assert!((cubic_root(125.0) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_cubic_root_fractional() {
        // Test fractional values
        assert!((cubic_root(0.125) - 0.5).abs() < 0.001);
        assert!((cubic_root(0.001) - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_cubic_root_large() {
        // Test larger values
        assert!((cubic_root(1000.0) - 10.0).abs() < 0.01);
        assert!((cubic_root(1000000.0) - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_cubic_root_zero() {
        assert_eq!(cubic_root(0.0), 0.0);
    }

    #[test]
    fn test_cubic_root_typical_values() {
        // Typical values seen in CUBIC calculations
        // For W_max=100 MSS, x = 100 * 0.125 / 0.4 = 31.25
        let x = 100.0 * (1.0 - CUBIC_BETA_ECN) / CUBIC_C;
        let k = cubic_root(x);
        // K should be cube root of 31.25 ≈ 3.15
        assert!((k - 3.15).abs() < 0.1);
    }

    #[test]
    fn test_cubic_state_default() {
        let state = CubicState::default();
        assert_eq!(state.alg_state, CubicAlgState::SlowStart);
        assert_eq!(state.ssthresh, u64::MAX);
        assert_eq!(state.recovery_sequence, 0);
    }

    #[test]
    fn test_cubic_state_reset() {
        let mut state = CubicState::default();
        state.alg_state = CubicAlgState::CongestionAvoidance;
        state.ssthresh = 50000;
        state.recovery_sequence = 100;

        state.reset(1200, 1000);

        assert_eq!(state.alg_state, CubicAlgState::SlowStart);
        assert_eq!(state.ssthresh, u64::MAX);
        assert_eq!(state.recovery_sequence, 0);
        assert_eq!(state.start_of_epoch, 1000);
    }

    #[test]
    fn test_cubic_w_cubic() {
        let mut state = CubicState::default();
        state.w_max = 100.0;
        state.start_of_epoch = 0;
        state.compute_k();

        // At t=K, W_cubic should equal W_max
        let time_at_k = (state.k * 1_000_000.0) as u64;
        let w = state.w_cubic(time_at_k);
        assert!((w - state.w_max).abs() < 1.0);
    }

    #[test]
    fn test_cubic_enter_recovery_timeout() {
        let mut state = CubicState::default();
        state.alg_state = CubicAlgState::CongestionAvoidance;

        let cwin = state.enter_recovery(CongestionNotification::Timeout, 100_000, 1200, 50, 1000);

        // Timeout should reset to minimum cwin and slow start
        assert_eq!(cwin, CWIN_MINIMUM);
        assert_eq!(state.alg_state, CubicAlgState::SlowStart);
        assert_eq!(state.recovery_sequence, 50);
    }

    #[test]
    fn test_cubic_enter_recovery_repeat() {
        let mut state = CubicState::default();
        state.alg_state = CubicAlgState::CongestionAvoidance;

        let cwin = state.enter_recovery(CongestionNotification::Repeat, 100_000, 1200, 50, 1000);

        // Should enter congestion avoidance with reduced window
        assert_eq!(state.alg_state, CubicAlgState::CongestionAvoidance);
        assert!(cwin < 100_000);
        assert_eq!(state.recovery_sequence, 50);
    }

    #[test]
    fn test_cubic_enter_avoidance() {
        let mut state = CubicState::default();
        state.w_max = 100.0;

        state.enter_avoidance(5000);

        assert_eq!(state.alg_state, CubicAlgState::CongestionAvoidance);
        assert_eq!(state.start_of_epoch, 5000);
        assert!(state.k > 0.0);
    }
}
