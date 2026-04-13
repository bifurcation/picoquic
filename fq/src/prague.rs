//! Prague (L4S) congestion control algorithm.
//!
//! Translated from picoquic/prague.c.
//!
//! Prague is a modification of NewReno for L4S (Low Latency, Low Loss,
//! Scalable Throughput) networks. It uses ECN CE marks with an alpha
//! coefficient to smoothly adjust the congestion window.

use crate::cc_common::{MinMaxRtt, CWIN_INITIAL, CWIN_MINIMUM};

// =============================================================================
// Prague Constants
// =============================================================================

/// Number of RTT periods for Reno-like behavior.
pub const NB_RTT_RENO: u32 = 4;

/// Shift value for alpha EWMA gain (g = 1/2^4 = 1/16).
pub const SHIFT_G: u32 = 4;

/// Inverse of gain parameter (2^SHIFT_G = 16).
pub const G_INV: u64 = 1 << SHIFT_G;

// =============================================================================
// Prague Algorithm State
// =============================================================================

/// Prague algorithm states.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PragueAlgState {
    SlowStart = 0,
    CongestionAvoidance = 1,
}

/// Prague congestion control state.
#[derive(Debug, Clone)]
pub struct PragueState {
    pub alg_state: PragueAlgState,
    /// Alpha coefficient for ECN-based window reduction (scaled by 1024).
    pub alpha: u64,
    /// Residual bytes for window increase calculation.
    pub residual_ack: u64,
    /// Slow start threshold.
    pub ssthresh: u64,
    /// Timestamp when recovery started.
    pub recovery_stamp: u64,
    /// Sequence number at start of recovery.
    pub recovery_sequence: u64,
    /// Send sequence at start of L4S update.
    pub l4s_update_sent: u64,
    /// Send sequence at start of epoch.
    pub l4s_epoch_send: u64,
    /// ECT1 count at start of epoch.
    pub l4s_epoch_ect1: u64,
    /// CE count at start of epoch.
    pub l4s_epoch_ce: u64,
    /// ECT1 count at last packet.
    pub l4s_packet_ect1: u64,
    /// CE count at last packet.
    pub l4s_packet_ce: u64,
    /// RTT filter for HyStart.
    pub rtt_filter: MinMaxRtt,
}

impl Default for PragueState {
    fn default() -> Self {
        Self {
            alg_state: PragueAlgState::SlowStart,
            alpha: 0,
            residual_ack: 0,
            ssthresh: u64::MAX,
            recovery_stamp: 0,
            recovery_sequence: 0,
            l4s_update_sent: 0,
            l4s_epoch_send: 0,
            l4s_epoch_ect1: 0,
            l4s_epoch_ce: 0,
            l4s_packet_ect1: 0,
            l4s_packet_ce: 0,
            rtt_filter: MinMaxRtt::default(),
        }
    }
}

impl PragueState {
    /// Initialize Prague state to Reno-like defaults.
    pub fn init_reno(&mut self) -> u64 {
        self.alg_state = PragueAlgState::SlowStart;
        self.ssthresh = u64::MAX;
        self.alpha = 0;
        CWIN_INITIAL
    }

    /// Reset L4S measurement context.
    pub fn reset_l4s(&mut self, send_sequence: u64, ecn_ect1_total: u64, ecn_ce_total: u64) {
        self.l4s_epoch_send = send_sequence;
        self.l4s_epoch_ect1 = ecn_ect1_total;
        self.l4s_epoch_ce = ecn_ce_total;
        self.alpha = 0;
    }

    /// Full reset of Prague state.
    pub fn reset(&mut self, send_sequence: u64, ecn_ect1_total: u64, ecn_ce_total: u64) -> u64 {
        let cwin = self.init_reno();
        self.reset_l4s(send_sequence, ecn_ect1_total, ecn_ce_total);
        cwin
    }

    /// Initialize a new era (epoch).
    pub fn initialize_era(
        &mut self,
        ecn_ect1_total: u64,
        ecn_ce_total: u64,
        current_time: u64,
        sequence_number: u64,
    ) {
        self.l4s_epoch_ect1 = ecn_ect1_total;
        self.l4s_epoch_ce = ecn_ce_total;
        self.recovery_stamp = current_time;
        self.recovery_sequence = sequence_number;
    }

    /// Enter recovery state after loss.
    ///
    /// Returns the new congestion window.
    pub fn enter_recovery(
        &mut self,
        cwin: u64,
        ecn_ect1_total: u64,
        ecn_ce_total: u64,
        current_time: u64,
        sequence_number: u64,
    ) -> u64 {
        self.ssthresh = cwin / 2;
        if self.ssthresh < CWIN_MINIMUM {
            self.ssthresh = CWIN_MINIMUM;
        }

        self.alg_state = PragueAlgState::CongestionAvoidance;
        self.initialize_era(ecn_ect1_total, ecn_ce_total, current_time, sequence_number);

        self.ssthresh
    }

    /// Update alpha coefficient based on ECN feedback.
    ///
    /// Alpha is an EWMA of the CE fraction, scaled by 1024.
    pub fn update_alpha(
        &mut self,
        delta_ect1: u64,
        delta_ce: u64,
        current_time: u64,
        smoothed_rtt: u64,
    ) {
        let mut frac = if delta_ce > 0 {
            (delta_ce * 1024) / (delta_ce + delta_ect1)
        } else {
            0
        };

        let mut is_suspect = false;

        // Check for suspect epoch (idle period followed by burst)
        if self.l4s_update_sent != 0
            && frac >= 512
            && self.alpha < 128
            && current_time - self.recovery_stamp > smoothed_rtt
        {
            is_suspect = true;
            frac = 128;
        }

        if delta_ce > 0 || delta_ect1 > 0 {
            if frac > self.alpha && (frac >= 512 || is_suspect) {
                // Sudden onset of congestion - use frac directly
                self.alpha = frac;
            } else {
                // EWMA: alpha = alpha * (1 - g) + frac * g
                // Where g = 1/G_INV = 1/16
                let alpha_shifted = self.alpha << SHIFT_G;
                let alpha_shifted = alpha_shifted - self.alpha + frac;
                self.alpha = alpha_shifted >> SHIFT_G;
            }
        }
    }

    /// Process ACK in congestion avoidance state.
    ///
    /// Returns (new_cwin, new_ssthresh).
    #[allow(clippy::too_many_arguments)]
    pub fn process_ack(
        &mut self,
        cwin: u64,
        send_mtu: u64,
        nb_bytes_acknowledged: u64,
        ack_number: u64,
        ecn_ect1_total: u64,
        ecn_ce_total: u64,
        current_time: u64,
        smoothed_rtt: u64,
        sequence_number: u64,
    ) -> (u64, u64) {
        let mut new_cwin = cwin;

        if ack_number > self.recovery_sequence {
            // New period - update alpha and do CWND reduction
            let delta_ect1 = ecn_ect1_total.saturating_sub(self.l4s_epoch_ect1);
            let delta_ce = ecn_ce_total.saturating_sub(self.l4s_epoch_ce);

            if delta_ce + delta_ect1 > 0 {
                self.update_alpha(delta_ect1, delta_ce, current_time, smoothed_rtt);

                // Update ssthresh and CWIN based on alpha
                let delta_cwin = (cwin * self.alpha) / 2048;
                new_cwin = cwin.saturating_sub(delta_cwin);
                if new_cwin < CWIN_MINIMUM {
                    new_cwin = CWIN_MINIMUM;
                }
                self.ssthresh = new_cwin;
            }

            // Reset era limits
            self.initialize_era(ecn_ect1_total, ecn_ce_total, current_time, sequence_number);
        }

        // Increment CWND based on non-CE fraction
        if ecn_ect1_total >= self.l4s_packet_ect1 && ecn_ce_total >= self.l4s_packet_ce {
            let delta_ect1_ack = ecn_ect1_total - self.l4s_packet_ect1;
            let delta_ce_ack = ecn_ce_total - self.l4s_packet_ce;
            let mut ack_bytes = nb_bytes_acknowledged;

            if delta_ce_ack + delta_ect1_ack > 0 {
                let frac_not_ce = delta_ect1_ack as f64 / (delta_ce_ack + delta_ect1_ack) as f64;
                ack_bytes = (frac_not_ce * ack_bytes as f64) as u64;
            }

            new_cwin += send_mtu * ack_bytes / new_cwin;
        }

        (new_cwin, self.ssthresh)
    }

    /// Process ACK in slow start state.
    ///
    /// Returns (new_cwin, should_enter_recovery, should_enter_ca).
    pub fn process_start_ack(
        &mut self,
        cwin: u64,
        ecn_ce_total: u64,
        nb_bytes_acknowledged: u64,
    ) -> (u64, bool, bool) {
        let mut new_cwin = cwin;

        if ecn_ce_total > self.l4s_epoch_ce {
            // CE mark received - exit and enter recovery
            return (new_cwin, true, false);
        }

        // Slow start increase
        new_cwin += nb_bytes_acknowledged;

        // Check if we've reached ssthresh
        let should_enter_ca = new_cwin >= self.ssthresh;

        (new_cwin, false, should_enter_ca)
    }

    /// Handle RTT measurement in slow start.
    ///
    /// Returns true if HyStart triggered exit from slow start.
    pub fn check_hystart_exit(&mut self, cwin: u64) -> bool {
        if self.alg_state == PragueAlgState::SlowStart && self.ssthresh == u64::MAX {
            // HyStart triggered - exit slow start
            self.ssthresh = cwin;
            self.alg_state = PragueAlgState::CongestionAvoidance;
            true
        } else {
            false
        }
    }

    /// Transition to congestion avoidance.
    pub fn enter_congestion_avoidance(&mut self, cwin: u64) {
        self.ssthresh = cwin;
        self.alg_state = PragueAlgState::CongestionAvoidance;
    }

    /// Check if in slow start with uninitialized ssthresh.
    pub fn is_initial_slow_start(&self) -> bool {
        self.alg_state == PragueAlgState::SlowStart && self.ssthresh == u64::MAX
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prague_state_default() {
        let state = PragueState::default();
        assert_eq!(state.alg_state, PragueAlgState::SlowStart);
        assert_eq!(state.ssthresh, u64::MAX);
        assert_eq!(state.alpha, 0);
    }

    #[test]
    fn test_prague_init_reno() {
        let mut state = PragueState::default();
        state.alg_state = PragueAlgState::CongestionAvoidance;
        state.ssthresh = 50_000;
        state.alpha = 100;

        let cwin = state.init_reno();

        assert_eq!(state.alg_state, PragueAlgState::SlowStart);
        assert_eq!(state.ssthresh, u64::MAX);
        assert_eq!(state.alpha, 0);
        assert_eq!(cwin, CWIN_INITIAL);
    }

    #[test]
    fn test_prague_reset_l4s() {
        let mut state = PragueState::default();
        state.alpha = 500;

        state.reset_l4s(100, 50, 10);

        assert_eq!(state.l4s_epoch_send, 100);
        assert_eq!(state.l4s_epoch_ect1, 50);
        assert_eq!(state.l4s_epoch_ce, 10);
        assert_eq!(state.alpha, 0);
    }

    #[test]
    fn test_prague_enter_recovery() {
        let mut state = PragueState::default();
        state.alg_state = PragueAlgState::SlowStart;

        let cwin = state.enter_recovery(100_000, 50, 10, 1_000, 100);

        assert_eq!(cwin, 50_000);
        assert_eq!(state.ssthresh, 50_000);
        assert_eq!(state.alg_state, PragueAlgState::CongestionAvoidance);
        assert_eq!(state.recovery_stamp, 1_000);
        assert_eq!(state.recovery_sequence, 100);
    }

    #[test]
    fn test_prague_enter_recovery_minimum() {
        let mut state = PragueState::default();

        let cwin = state.enter_recovery(CWIN_MINIMUM, 0, 0, 1_000, 100);

        assert_eq!(cwin, CWIN_MINIMUM);
        assert_eq!(state.ssthresh, CWIN_MINIMUM);
    }

    #[test]
    fn test_prague_update_alpha_no_ce() {
        let mut state = PragueState::default();
        state.alpha = 100;

        state.update_alpha(100, 0, 1_000, 50_000);

        // With no CE marks, alpha should decrease via EWMA
        // alpha = (100 * 15 + 0) / 16 = 93
        assert!(state.alpha < 100);
    }

    #[test]
    fn test_prague_update_alpha_with_ce() {
        let mut state = PragueState::default();
        state.alpha = 0;
        state.l4s_update_sent = 0;

        // 50% CE marks: frac = 50 * 1024 / 100 = 512
        state.update_alpha(50, 50, 1_000, 50_000);

        // With frac >= 512 and alpha < 128, should use frac directly
        assert_eq!(state.alpha, 512);
    }

    #[test]
    fn test_prague_update_alpha_ewma() {
        let mut state = PragueState::default();
        state.alpha = 256;
        state.l4s_update_sent = 1;

        // 25% CE marks: frac = 25 * 1024 / 100 = 256
        // Since frac (256) <= alpha (256) and frac < 512, use EWMA
        state.update_alpha(75, 25, 1_000, 50_000);

        // alpha = (256 * 15 + 256) / 16 = 256 (stable)
        assert_eq!(state.alpha, 256);
    }

    #[test]
    fn test_prague_process_ack_new_period() {
        let mut state = PragueState::default();
        state.alg_state = PragueAlgState::CongestionAvoidance;
        state.recovery_sequence = 50;
        state.l4s_epoch_ect1 = 0;
        state.l4s_epoch_ce = 0;
        state.l4s_packet_ect1 = 0;
        state.l4s_packet_ce = 0;

        // Process ACK with CE marks
        let (new_cwin, new_ssthresh) = state.process_ack(
            100_000, // cwin
            1200,    // send_mtu
            1200,    // nb_bytes_acknowledged
            60,      // ack_number (> recovery_sequence)
            80,      // ecn_ect1_total
            20,      // ecn_ce_total (20% CE)
            1_000,   // current_time
            50_000,  // smoothed_rtt
            60,      // sequence_number
        );

        // Alpha should be updated, cwin should be reduced
        assert!(state.alpha > 0);
        assert!(new_cwin < 100_000);
        // ssthresh is set before cwin increment, so cwin >= ssthresh
        assert!(new_cwin >= new_ssthresh);
        assert!(new_ssthresh < 100_000);
    }

    #[test]
    fn test_prague_process_start_ack_ce() {
        let mut state = PragueState::default();
        state.l4s_epoch_ce = 0;

        let (cwin, should_recover, _) = state.process_start_ack(50_000, 1, 1000);

        assert_eq!(cwin, 50_000);
        assert!(should_recover);
    }

    #[test]
    fn test_prague_process_start_ack_growth() {
        let mut state = PragueState::default();
        state.l4s_epoch_ce = 0;
        state.ssthresh = u64::MAX;

        let (cwin, should_recover, should_ca) = state.process_start_ack(50_000, 0, 1000);

        assert_eq!(cwin, 51_000);
        assert!(!should_recover);
        assert!(!should_ca);
    }

    #[test]
    fn test_prague_process_start_ack_reach_ssthresh() {
        let mut state = PragueState::default();
        state.l4s_epoch_ce = 0;
        state.ssthresh = 51_000;

        let (cwin, should_recover, should_ca) = state.process_start_ack(50_000, 0, 2000);

        assert_eq!(cwin, 52_000);
        assert!(!should_recover);
        assert!(should_ca);
    }

    #[test]
    fn test_prague_check_hystart_exit() {
        let mut state = PragueState::default();
        state.alg_state = PragueAlgState::SlowStart;
        state.ssthresh = u64::MAX;

        let triggered = state.check_hystart_exit(50_000);

        assert!(triggered);
        assert_eq!(state.ssthresh, 50_000);
        assert_eq!(state.alg_state, PragueAlgState::CongestionAvoidance);
    }

    #[test]
    fn test_prague_check_hystart_exit_not_initial() {
        let mut state = PragueState::default();
        state.alg_state = PragueAlgState::SlowStart;
        state.ssthresh = 100_000; // Already initialized

        let triggered = state.check_hystart_exit(50_000);

        assert!(!triggered);
        assert_eq!(state.ssthresh, 100_000); // Unchanged
    }

    #[test]
    fn test_prague_is_initial_slow_start() {
        let mut state = PragueState::default();

        assert!(state.is_initial_slow_start());

        state.ssthresh = 50_000;
        assert!(!state.is_initial_slow_start());

        state.ssthresh = u64::MAX;
        state.alg_state = PragueAlgState::CongestionAvoidance;
        assert!(!state.is_initial_slow_start());
    }

    #[test]
    fn test_constants() {
        assert_eq!(SHIFT_G, 4);
        assert_eq!(G_INV, 16);
        assert_eq!(NB_RTT_RENO, 4);
    }
}
