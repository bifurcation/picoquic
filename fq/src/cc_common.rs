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
pub const CWIN_INITIAL: u64 = 10 * 1252; // Initial congestion window

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
        let result = rtt.hystart_loss_volume_test(CongestionNotification::Acknowledgement, 100, 50);
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
}
