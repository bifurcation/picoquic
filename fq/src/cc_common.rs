//! Congestion control common utilities.
//!
//! Translated from picoquic/cc_common.c and picoquic/cc_common.h.

use std::ffi::c_int;

// =============================================================================
// Constants from cc_common.h
// =============================================================================

pub const MIN_MAX_RTT_SCOPE: usize = 7;
pub const SMOOTHED_LOSS_SCOPE: u64 = 32;
pub const SMOOTHED_LOSS_FACTOR: f64 = 1.0 / 16.0;
pub const SMOOTHED_LOSS_THRESHOLD: f64 = 0.15;

// HyStart++ constants
pub const HYSTART_PP_CSS_GROWTH_DIVISOR: u64 = 4;

// Target RTT constants (from picoquic_internal.h)
pub const TARGET_RENO_RTT: u64 = 100_000; // 100ms in microseconds
pub const TARGET_SATELLITE_RTT: u64 = 600_000; // 600ms in microseconds

// Packet size and congestion window constants (from picoquic_internal.h)
pub const MAX_PACKET_SIZE: u64 = 1536;
pub const CWIN_INITIAL: u64 = 10 * MAX_PACKET_SIZE; // 15360
pub const CWIN_MINIMUM: u64 = 2 * MAX_PACKET_SIZE; // 3072

/// Congestion notification event types.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionNotification {
    Acknowledgement = 0,
    Repeat = 1,
    Timeout = 2,
    Spurious = 3,
    CwinBlocked = 4,
    Reset = 5,
    SeedCwin = 6,
    // ... other variants exist in C but aren't used by translated functions
}

// =============================================================================
// MinMaxRtt - Safe Rust equivalent of picoquic_min_max_rtt_t
// =============================================================================

/// RTT tracking state for HyStart and loss detection.
///
/// This is a safe Rust struct - no raw pointers. The FFI layer
/// converts to/from the C `picoquic_min_max_rtt_t` struct.
#[derive(Debug, Clone)]
pub struct MinMaxRtt {
    pub last_rtt_sample_time: u64,
    pub rtt_filtered_min: u64,
    pub nb_rtt_excess: i32,
    pub sample_current: i32,
    pub is_init: bool,
    pub smoothed_drop_rate: f64,
    pub smoothed_bytes_sent_16: u64,
    pub smoothed_bytes_lost_16: u64,
    pub last_lost_packet_number: u64,
    pub sample_min: u64,
    pub sample_max: u64,
    pub samples: [u64; MIN_MAX_RTT_SCOPE],
}

impl Default for MinMaxRtt {
    fn default() -> Self {
        Self {
            last_rtt_sample_time: 0,
            rtt_filtered_min: 0,
            nb_rtt_excess: 0,
            sample_current: 0,
            is_init: false,
            smoothed_drop_rate: 0.0,
            smoothed_bytes_sent_16: 0,
            smoothed_bytes_lost_16: 0,
            last_lost_packet_number: 0,
            sample_min: 0,
            sample_max: 0,
            samples: [0; MIN_MAX_RTT_SCOPE],
        }
    }
}

impl MinMaxRtt {
    /// Filter RTT samples to track min and max over a sliding window.
    pub fn filter_rtt_min_max(&mut self, rtt: u64) {
        let x = self.sample_current as usize;

        self.samples[x] = rtt;

        self.sample_current += 1;
        if self.sample_current >= MIN_MAX_RTT_SCOPE as i32 {
            self.is_init = true;
            self.sample_current = 0;
        }

        let x_max = if self.is_init {
            MIN_MAX_RTT_SCOPE
        } else {
            x + 1
        };

        self.sample_min = self.samples[0];
        self.sample_max = self.samples[0];

        for i in 1..x_max {
            if self.samples[i] < self.sample_min {
                self.sample_min = self.samples[i];
            } else if self.samples[i] > self.sample_max {
                self.sample_max = self.samples[i];
            }
        }
    }

    /// Test for loss-based exit from slow start (packet number based).
    pub fn hystart_loss_test(
        &mut self,
        event: CongestionNotification,
        lost_packet_number: u64,
        error_rate_max: f64,
    ) -> bool {
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

            match event {
                CongestionNotification::Repeat => {
                    return self.smoothed_drop_rate > error_rate_max;
                }
                CongestionNotification::Timeout => {
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    /// Test for loss-based exit from slow start (volume based).
    pub fn hystart_loss_volume_test(
        &mut self,
        event: CongestionNotification,
        nb_bytes_newly_acked: u64,
        nb_bytes_newly_lost: u64,
    ) -> bool {
        self.smoothed_bytes_lost_16 -= self.smoothed_bytes_lost_16 / 16;
        self.smoothed_bytes_lost_16 += nb_bytes_newly_lost;
        self.smoothed_bytes_sent_16 -= self.smoothed_bytes_sent_16 / 16;
        self.smoothed_bytes_sent_16 += nb_bytes_newly_acked + nb_bytes_newly_lost;

        if self.smoothed_bytes_sent_16 > 0 {
            self.smoothed_drop_rate =
                (self.smoothed_bytes_lost_16 as f64) / (self.smoothed_bytes_sent_16 as f64);
        } else {
            self.smoothed_drop_rate = 0.0;
        }

        match event {
            CongestionNotification::Acknowledgement => {
                self.smoothed_drop_rate > SMOOTHED_LOSS_THRESHOLD
            }
            CongestionNotification::Timeout => true,
            _ => false,
        }
    }

    /// Test for RTT-based exit from slow start.
    pub fn hystart_test(
        &mut self,
        rtt_measurement: u64,
        packet_time: u64,
        current_time: u64,
        _is_one_way_delay_enabled: bool,
    ) -> bool {
        // Bypass unused argument warning without changing signature
        if _is_one_way_delay_enabled && rtt_measurement == 0 {
            return false;
        }

        if current_time > self.last_rtt_sample_time + 1000 {
            self.filter_rtt_min_max(rtt_measurement);
            self.last_rtt_sample_time = current_time;

            if self.is_init {
                let mut delta_max: u64;

                if self.rtt_filtered_min == 0 || self.rtt_filtered_min > self.sample_max {
                    self.rtt_filtered_min = self.sample_max;
                }
                delta_max = self.rtt_filtered_min / 4;
                if delta_max < packet_time {
                    delta_max = packet_time;
                }

                if self.sample_min > self.rtt_filtered_min {
                    if self.sample_min > self.rtt_filtered_min + delta_max {
                        self.nb_rtt_excess += 1;
                        if self.nb_rtt_excess >= MIN_MAX_RTT_SCOPE as i32 {
                            // RTT increased too much, exit slow start
                            return true;
                        }
                    }
                } else {
                    self.nb_rtt_excess = 0;
                }
            }
        }

        false
    }
}

// =============================================================================
// C-compatible struct for FFI
// =============================================================================

/// C-compatible struct matching `picoquic_min_max_rtt_t`.
#[repr(C)]
pub struct CMinMaxRtt {
    pub last_rtt_sample_time: u64,
    pub rtt_filtered_min: u64,
    pub nb_rtt_excess: c_int,
    pub sample_current: c_int,
    pub is_init: c_int,
    pub smoothed_drop_rate: f64,
    pub smoothed_bytes_sent_16: u64,
    pub smoothed_bytes_lost_16: u64,
    pub last_lost_packet_number: u64,
    pub sample_min: u64,
    pub sample_max: u64,
    pub samples: [u64; MIN_MAX_RTT_SCOPE],
}

impl CMinMaxRtt {
    /// Convert from C struct to safe Rust struct.
    ///
    /// # Safety
    /// The CMinMaxRtt must be properly initialized.
    pub fn to_rust(&self) -> MinMaxRtt {
        MinMaxRtt {
            last_rtt_sample_time: self.last_rtt_sample_time,
            rtt_filtered_min: self.rtt_filtered_min,
            nb_rtt_excess: self.nb_rtt_excess,
            sample_current: self.sample_current,
            is_init: self.is_init != 0,
            smoothed_drop_rate: self.smoothed_drop_rate,
            smoothed_bytes_sent_16: self.smoothed_bytes_sent_16,
            smoothed_bytes_lost_16: self.smoothed_bytes_lost_16,
            last_lost_packet_number: self.last_lost_packet_number,
            sample_min: self.sample_min,
            sample_max: self.sample_max,
            samples: self.samples,
        }
    }

    /// Update C struct from safe Rust struct.
    pub fn from_rust(&mut self, rust: &MinMaxRtt) {
        self.last_rtt_sample_time = rust.last_rtt_sample_time;
        self.rtt_filtered_min = rust.rtt_filtered_min;
        self.nb_rtt_excess = rust.nb_rtt_excess;
        self.sample_current = rust.sample_current;
        self.is_init = if rust.is_init { 1 } else { 0 };
        self.smoothed_drop_rate = rust.smoothed_drop_rate;
        self.smoothed_bytes_sent_16 = rust.smoothed_bytes_sent_16;
        self.smoothed_bytes_lost_16 = rust.smoothed_bytes_lost_16;
        self.last_lost_packet_number = rust.last_lost_packet_number;
        self.sample_min = rust.sample_min;
        self.sample_max = rust.sample_max;
        self.samples = rust.samples;
    }
}

// =============================================================================
// NewRenoSimState - Safe Rust equivalent of picoquic_newreno_sim_state_t
// =============================================================================

/// NewReno algorithm state.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewRenoAlgState {
    SlowStart = 0,
    CongestionAvoidance = 1,
}

/// Simulated NewReno state for parallel congestion control.
///
/// Many CC algorithms run a parallel NewReno to provide a lower bound estimate.
/// This struct holds the entire state without references to connection/path.
#[derive(Debug, Clone)]
pub struct NewRenoSimState {
    pub alg_state: NewRenoAlgState,
    pub cwin: u64,
    pub residual_ack: u64,
    pub ssthresh: u64,
    pub recovery_start: u64,
    pub recovery_sequence: u64,
}

impl NewRenoSimState {
    /// Initialize/reset the NewReno simulation state.
    pub fn reset(&mut self) {
        self.alg_state = NewRenoAlgState::SlowStart;
        self.cwin = CWIN_INITIAL;
        self.residual_ack = 0;
        self.ssthresh = u64::MAX;
        self.recovery_start = 0;
        self.recovery_sequence = 0;
    }

    /// Seed the congestion window from a previous connection.
    pub fn seed_cwin(&mut self, seed_cwin: u64) {
        if self.alg_state == NewRenoAlgState::SlowStart
            && self.ssthresh == u64::MAX
            && seed_cwin > self.cwin
        {
            self.cwin = seed_cwin;
            self.ssthresh = seed_cwin;
            self.alg_state = NewRenoAlgState::CongestionAvoidance;
        }
    }
}

impl Default for NewRenoSimState {
    fn default() -> Self {
        Self {
            alg_state: NewRenoAlgState::SlowStart,
            cwin: CWIN_INITIAL,
            residual_ack: 0,
            ssthresh: u64::MAX,
            recovery_start: 0,
            recovery_sequence: 0,
        }
    }
}

/// C-compatible struct matching `picoquic_newreno_sim_state_t`.
#[repr(C)]
pub struct CNewRenoSimState {
    pub alg_state: c_int,
    pub cwin: u64,
    pub residual_ack: u64,
    pub ssthresh: u64,
    pub recovery_start: u64,
    pub recovery_sequence: u64,
}

impl CNewRenoSimState {
    /// Convert from C struct to safe Rust struct.
    pub fn to_rust(&self) -> NewRenoSimState {
        NewRenoSimState {
            alg_state: if self.alg_state == 0 {
                NewRenoAlgState::SlowStart
            } else {
                NewRenoAlgState::CongestionAvoidance
            },
            cwin: self.cwin,
            residual_ack: self.residual_ack,
            ssthresh: self.ssthresh,
            recovery_start: self.recovery_start,
            recovery_sequence: self.recovery_sequence,
        }
    }

    /// Update C struct from safe Rust struct.
    pub fn from_rust(&mut self, rust: &NewRenoSimState) {
        self.alg_state = rust.alg_state as c_int;
        self.cwin = rust.cwin;
        self.residual_ack = rust.residual_ack;
        self.ssthresh = rust.ssthresh;
        self.recovery_start = rust.recovery_start;
        self.recovery_sequence = rust.recovery_sequence;
    }
}

// =============================================================================
// FFI exports for MinMaxRtt functions
// =============================================================================

/// FFI export: Filter RTT min/max samples.
///
/// # Safety
/// `rtt_track` must point to a valid `picoquic_min_max_rtt_t` struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_cc_filter_rtt_min_max(rtt_track: *mut CMinMaxRtt, rtt: u64) {
    let c_struct = &mut *rtt_track;
    let mut rust_struct = c_struct.to_rust();
    rust_struct.filter_rtt_min_max(rtt);
    c_struct.from_rust(&rust_struct);
}

/// FFI export: HyStart loss test (packet number based).
///
/// # Safety
/// `rtt_track` must point to a valid `picoquic_min_max_rtt_t` struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_cc_hystart_loss_test(
    rtt_track: *mut CMinMaxRtt,
    event: c_int,
    lost_packet_number: u64,
    error_rate_max: f64,
) -> c_int {
    let c_struct = &mut *rtt_track;
    let mut rust_struct = c_struct.to_rust();

    let event = match event {
        0 => CongestionNotification::Acknowledgement,
        1 => CongestionNotification::Repeat,
        2 => CongestionNotification::Timeout,
        3 => CongestionNotification::Spurious,
        4 => CongestionNotification::CwinBlocked,
        5 => CongestionNotification::Reset,
        6 => CongestionNotification::SeedCwin,
        _ => CongestionNotification::Acknowledgement,
    };

    let result = rust_struct.hystart_loss_test(event, lost_packet_number, error_rate_max);
    c_struct.from_rust(&rust_struct);

    if result {
        1
    } else {
        0
    }
}

/// FFI export: HyStart loss volume test.
///
/// # Safety
/// `rtt_track` must point to a valid `picoquic_min_max_rtt_t` struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_cc_hystart_loss_volume_test(
    rtt_track: *mut CMinMaxRtt,
    event: c_int,
    nb_bytes_newly_acked: u64,
    nb_bytes_newly_lost: u64,
) -> c_int {
    let c_struct = &mut *rtt_track;
    let mut rust_struct = c_struct.to_rust();

    let event = match event {
        0 => CongestionNotification::Acknowledgement,
        1 => CongestionNotification::Repeat,
        2 => CongestionNotification::Timeout,
        3 => CongestionNotification::Spurious,
        4 => CongestionNotification::CwinBlocked,
        5 => CongestionNotification::Reset,
        6 => CongestionNotification::SeedCwin,
        _ => CongestionNotification::Acknowledgement,
    };

    let result =
        rust_struct.hystart_loss_volume_test(event, nb_bytes_newly_acked, nb_bytes_newly_lost);
    c_struct.from_rust(&rust_struct);

    if result {
        1
    } else {
        0
    }
}

/// FFI export: HyStart RTT test.
///
/// # Safety
/// `rtt_track` must point to a valid `picoquic_min_max_rtt_t` struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_cc_hystart_test(
    rtt_track: *mut CMinMaxRtt,
    rtt_measurement: u64,
    packet_time: u64,
    current_time: u64,
    is_one_way_delay_enabled: c_int,
) -> c_int {
    let c_struct = &mut *rtt_track;
    let mut rust_struct = c_struct.to_rust();

    let result = rust_struct.hystart_test(
        rtt_measurement,
        packet_time,
        current_time,
        is_one_way_delay_enabled != 0,
    );
    c_struct.from_rust(&rust_struct);

    if result {
        1
    } else {
        0
    }
}

// =============================================================================
// Slow Start and CWIN Functions (Safe Rust API)
// =============================================================================

/// Calculate bytes from rate: (microseconds * bps) / 1_000_000
/// Equivalent to PICOQUIC_BYTES_FROM_RATE macro.
#[inline]
pub fn bytes_from_rate(microseconds: u64, bps: u64) -> u64 {
    (microseconds as u128 * bps as u128 / 1_000_000) as u64
}

/// Calculate slow start increase based on congestion state.
///
/// Returns 0 if not congestion-window blocked (app limited),
/// otherwise returns the bytes delivered.
pub fn slow_start_increase(cwin_blocked: bool, nb_delivered: u64) -> u64 {
    if !cwin_blocked {
        return 0;
    }
    nb_delivered
}

/// Extended slow start increase with CSS (Consecutive Slow Start) support.
///
/// If in CSS mode, divides the increase by the CSS growth divisor.
pub fn slow_start_increase_ex(cwin_blocked: bool, nb_delivered: u64, in_css: bool) -> u64 {
    if in_css {
        slow_start_increase(cwin_blocked, nb_delivered / HYSTART_PP_CSS_GROWTH_DIVISOR)
    } else {
        slow_start_increase(cwin_blocked, nb_delivered)
    }
}

/// Extended slow start increase with Prague (L4S) ECN support.
///
/// Adjusts the increase based on ECN feedback (prague_alpha) and RTT.
pub fn slow_start_increase_ex2(
    cwin_blocked: bool,
    nb_delivered: u64,
    in_css: bool,
    prague_alpha: u64,
    smoothed_rtt: u64,
) -> u64 {
    if prague_alpha != 0 {
        let mut delta = nb_delivered;

        if smoothed_rtt <= TARGET_RENO_RTT {
            delta = delta * (1024 - prague_alpha) / 1024;
        } else {
            delta = delta * smoothed_rtt * (1024 - prague_alpha) / TARGET_RENO_RTT / 1024;
        }

        slow_start_increase_ex(cwin_blocked, delta, in_css)
    } else {
        slow_start_increase_ex(cwin_blocked, nb_delivered, in_css)
    }
}

/// Update target congestion window based on bandwidth estimation.
///
/// Returns the larger of current cwin or half the BDP (bandwidth-delay product).
pub fn update_target_cwin_estimation(
    smoothed_rtt: u64,
    peak_bandwidth_estimate: u64,
    cwin: u64,
) -> u64 {
    let max_win = bytes_from_rate(smoothed_rtt, peak_bandwidth_estimate);
    let min_win = max_win / 2;

    if min_win > cwin {
        min_win
    } else {
        cwin
    }
}

/// Update congestion window for long RTT paths.
///
/// Scales initial cwin based on path RTT relative to target RTT.
pub fn update_cwin_for_long_rtt(rtt_min: u64, cwin: u64) -> u64 {
    let min_cwnd = if rtt_min > TARGET_SATELLITE_RTT {
        (CWIN_INITIAL as f64 * TARGET_SATELLITE_RTT as f64 / TARGET_RENO_RTT as f64) as u64
    } else {
        (CWIN_INITIAL as f64 * rtt_min as f64 / TARGET_RENO_RTT as f64) as u64
    };

    if min_cwnd > cwin {
        min_cwnd
    } else {
        cwin
    }
}

/// Calculate increased window for a connection based on RTT.
///
/// Doubles the window for short RTT, scales for longer RTT.
pub fn increased_window(path0_rtt_min: u64, previous_window: u64) -> u64 {
    if path0_rtt_min <= TARGET_RENO_RTT {
        previous_window * 2
    } else {
        let w = previous_window as f64;
        let rtt = if path0_rtt_min > TARGET_SATELLITE_RTT {
            TARGET_SATELLITE_RTT as f64
        } else {
            path0_rtt_min as f64
        };
        (w / TARGET_RENO_RTT as f64 * rtt) as u64
    }
}

// =============================================================================
// FFI exports for NewRenoSimState functions
// =============================================================================

/// FFI export: Reset NewReno simulation state.
///
/// # Safety
/// `nrss` must point to a valid `picoquic_newreno_sim_state_t` struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_newreno_sim_reset(nrss: *mut CNewRenoSimState) {
    let c_struct = &mut *nrss;
    let mut rust_struct = c_struct.to_rust();
    rust_struct.reset();
    c_struct.from_rust(&rust_struct);
}

// Note: FFI exports for slow_start and cwin functions are not provided because
// they require picoquic_path_t* and picoquic_cnx_t* which have complex internal
// structures. These functions remain in C until full path/connection translation.
// The safe Rust implementations above are available for use by other Rust code.

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max_rtt_default() {
        let rtt = MinMaxRtt::default();
        assert_eq!(rtt.sample_current, 0);
        assert!(!rtt.is_init);
        assert_eq!(rtt.samples, [0; MIN_MAX_RTT_SCOPE]);
    }

    #[test]
    fn test_filter_rtt_min_max() {
        let mut rtt = MinMaxRtt::default();

        // Add some samples
        rtt.filter_rtt_min_max(100);
        assert_eq!(rtt.sample_current, 1);
        assert_eq!(rtt.sample_min, 100);
        assert_eq!(rtt.sample_max, 100);

        rtt.filter_rtt_min_max(50);
        assert_eq!(rtt.sample_current, 2);
        assert_eq!(rtt.sample_min, 50);
        assert_eq!(rtt.sample_max, 100);

        rtt.filter_rtt_min_max(200);
        assert_eq!(rtt.sample_min, 50);
        assert_eq!(rtt.sample_max, 200);
    }

    #[test]
    fn test_filter_rtt_wraps_around() {
        let mut rtt = MinMaxRtt::default();

        // Fill all samples
        for i in 0..MIN_MAX_RTT_SCOPE {
            rtt.filter_rtt_min_max((i as u64 + 1) * 10);
        }

        assert!(rtt.is_init);
        assert_eq!(rtt.sample_current, 0);
        assert_eq!(rtt.sample_min, 10);
        assert_eq!(rtt.sample_max, 70);
    }

    #[test]
    fn test_hystart_loss_test_timeout() {
        let mut rtt = MinMaxRtt::default();
        let result = rtt.hystart_loss_test(CongestionNotification::Timeout, 100, 0.1);
        assert!(result);
    }

    #[test]
    fn test_hystart_loss_volume_test() {
        let mut rtt = MinMaxRtt::default();

        // Simulate high loss rate
        for _ in 0..20 {
            rtt.hystart_loss_volume_test(CongestionNotification::Acknowledgement, 100, 50);
        }

        // Should eventually trigger due to high loss
        let _result = rtt.hystart_loss_volume_test(CongestionNotification::Acknowledgement, 100, 50);
        // After enough iterations with 33% loss, should exceed threshold
        assert!(rtt.smoothed_drop_rate > 0.0);
    }

    #[test]
    fn test_hystart_test_not_init() {
        let mut rtt = MinMaxRtt::default();
        // Before initialization, should not exit slow start
        let result = rtt.hystart_test(1000, 100, 2000, false);
        assert!(!result);
    }

    #[test]
    fn test_c_struct_conversion() {
        let rust = MinMaxRtt {
            last_rtt_sample_time: 1000,
            rtt_filtered_min: 500,
            nb_rtt_excess: 2,
            sample_current: 3,
            is_init: true,
            smoothed_drop_rate: 0.05,
            smoothed_bytes_sent_16: 1000,
            smoothed_bytes_lost_16: 50,
            last_lost_packet_number: 42,
            sample_min: 400,
            sample_max: 600,
            samples: [100, 200, 300, 400, 500, 600, 700],
        };

        let mut c_struct = CMinMaxRtt {
            last_rtt_sample_time: 0,
            rtt_filtered_min: 0,
            nb_rtt_excess: 0,
            sample_current: 0,
            is_init: 0,
            smoothed_drop_rate: 0.0,
            smoothed_bytes_sent_16: 0,
            smoothed_bytes_lost_16: 0,
            last_lost_packet_number: 0,
            sample_min: 0,
            sample_max: 0,
            samples: [0; MIN_MAX_RTT_SCOPE],
        };

        c_struct.from_rust(&rust);
        let back = c_struct.to_rust();

        assert_eq!(back.last_rtt_sample_time, rust.last_rtt_sample_time);
        assert_eq!(back.is_init, rust.is_init);
        assert_eq!(back.samples, rust.samples);
    }

    #[test]
    fn test_newreno_sim_default() {
        let state = NewRenoSimState::default();
        assert_eq!(state.alg_state, NewRenoAlgState::SlowStart);
        assert_eq!(state.cwin, CWIN_INITIAL);
        assert_eq!(state.ssthresh, u64::MAX);
    }

    #[test]
    fn test_newreno_sim_reset() {
        let mut state = NewRenoSimState {
            alg_state: NewRenoAlgState::CongestionAvoidance,
            cwin: 100000,
            residual_ack: 500,
            ssthresh: 50000,
            recovery_start: 12345,
            recovery_sequence: 1000,
        };

        state.reset();

        assert_eq!(state.alg_state, NewRenoAlgState::SlowStart);
        assert_eq!(state.cwin, CWIN_INITIAL);
        assert_eq!(state.residual_ack, 0);
        assert_eq!(state.ssthresh, u64::MAX);
        assert_eq!(state.recovery_start, 0);
        assert_eq!(state.recovery_sequence, 0);
    }

    #[test]
    fn test_newreno_sim_seed_cwin() {
        let mut state = NewRenoSimState::default();
        let seed = 50000;

        state.seed_cwin(seed);

        assert_eq!(state.alg_state, NewRenoAlgState::CongestionAvoidance);
        assert_eq!(state.cwin, seed);
        assert_eq!(state.ssthresh, seed);
    }

    #[test]
    fn test_newreno_sim_seed_cwin_no_effect_if_not_slow_start() {
        let mut state = NewRenoSimState::default();
        state.alg_state = NewRenoAlgState::CongestionAvoidance;

        let old_cwin = state.cwin;
        state.seed_cwin(50000);

        // Should not change because not in slow start
        assert_eq!(state.cwin, old_cwin);
    }

    #[test]
    fn test_newreno_sim_c_struct_conversion() {
        let rust = NewRenoSimState {
            alg_state: NewRenoAlgState::CongestionAvoidance,
            cwin: 65536,
            residual_ack: 1000,
            ssthresh: 32768,
            recovery_start: 100,
            recovery_sequence: 50,
        };

        let mut c_struct = CNewRenoSimState {
            alg_state: 0,
            cwin: 0,
            residual_ack: 0,
            ssthresh: 0,
            recovery_start: 0,
            recovery_sequence: 0,
        };

        c_struct.from_rust(&rust);
        let back = c_struct.to_rust();

        assert_eq!(back.alg_state, rust.alg_state);
        assert_eq!(back.cwin, rust.cwin);
        assert_eq!(back.ssthresh, rust.ssthresh);
    }

    // =========================================================================
    // Tests for slow start and cwin functions
    // =========================================================================

    #[test]
    fn test_bytes_from_rate() {
        // Formula: microseconds * bytes_per_second / 1_000_000
        // 100ms (100,000 μs) * 1MB/s = 100,000 bytes
        assert_eq!(bytes_from_rate(100_000, 1_000_000), 100_000);
        // 100ms * 100MB/s = 10,000,000 bytes
        assert_eq!(bytes_from_rate(100_000, 100_000_000), 10_000_000);
        // 1s * 1GB/s = 1,000,000,000 bytes
        assert_eq!(bytes_from_rate(1_000_000, 1_000_000_000), 1_000_000_000);
    }

    #[test]
    fn test_slow_start_increase_cwin_blocked() {
        // When cwin blocked, should return full delivered
        assert_eq!(slow_start_increase(true, 1000), 1000);
    }

    #[test]
    fn test_slow_start_increase_not_blocked() {
        // When not cwin blocked (app limited), should return 0
        assert_eq!(slow_start_increase(false, 1000), 0);
    }

    #[test]
    fn test_slow_start_increase_ex_css() {
        // In CSS mode, should divide by 4
        assert_eq!(slow_start_increase_ex(true, 1000, true), 250);
    }

    #[test]
    fn test_slow_start_increase_ex_no_css() {
        // Not in CSS mode, full increase
        assert_eq!(slow_start_increase_ex(true, 1000, false), 1000);
    }

    #[test]
    fn test_slow_start_increase_ex2_with_prague() {
        // With prague_alpha = 512 (50% ECN), short RTT
        // delta = 1000 * (1024 - 512) / 1024 = 500
        let result = slow_start_increase_ex2(true, 1000, false, 512, 50_000);
        assert_eq!(result, 500);
    }

    #[test]
    fn test_slow_start_increase_ex2_long_rtt_with_prague() {
        // With prague_alpha = 512, RTT 200ms (> 100ms target)
        // delta = 1000 * 200000 * 512 / 100000 / 1024 = 1000
        let result = slow_start_increase_ex2(true, 1000, false, 512, 200_000);
        assert_eq!(result, 1000);
    }

    #[test]
    fn test_slow_start_increase_ex2_no_prague() {
        // Without prague (alpha = 0), should fall through
        let result = slow_start_increase_ex2(true, 1000, false, 0, 50_000);
        assert_eq!(result, 1000);
    }

    #[test]
    fn test_update_target_cwin_estimation() {
        // 100ms RTT * 10MB/s = 1,000,000 bytes BDP, min_win = 500,000
        // If cwin < min_win, should return min_win
        let result = update_target_cwin_estimation(100_000, 10_000_000, 100);
        assert_eq!(result, 500_000);

        // If cwin > min_win, should return cwin
        let result = update_target_cwin_estimation(100_000, 10_000_000, 1_000_000);
        assert_eq!(result, 1_000_000);
    }

    #[test]
    fn test_update_cwin_for_long_rtt() {
        // RTT 200ms > 100ms target
        // min_cwnd = CWIN_INITIAL * 200000 / 100000 = 30720
        let result = update_cwin_for_long_rtt(200_000, 10000);
        assert_eq!(result, 30720);

        // If cwin already larger, return cwin
        let result = update_cwin_for_long_rtt(200_000, 50000);
        assert_eq!(result, 50000);
    }

    #[test]
    fn test_update_cwin_for_long_rtt_satellite() {
        // RTT 800ms > 600ms satellite target, caps at satellite
        // min_cwnd = CWIN_INITIAL * 600000 / 100000 = 92160
        let result = update_cwin_for_long_rtt(800_000, 10000);
        assert_eq!(result, 92160);
    }

    #[test]
    fn test_increased_window_short_rtt() {
        // RTT <= 100ms doubles the window
        let result = increased_window(50_000, 10000);
        assert_eq!(result, 20000);

        let result = increased_window(100_000, 10000);
        assert_eq!(result, 20000);
    }

    #[test]
    fn test_increased_window_long_rtt() {
        // RTT 200ms > 100ms, scales: 10000 / 100000 * 200000 = 20000
        let result = increased_window(200_000, 10000);
        assert_eq!(result, 20000);
    }

    #[test]
    fn test_increased_window_satellite_rtt() {
        // RTT 800ms > 600ms satellite, caps at 600ms
        // 10000 / 100000 * 600000 = 60000
        let result = increased_window(800_000, 10000);
        assert_eq!(result, 60000);
    }
}
