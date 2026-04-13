//! Pacing implementation for rate-limited packet transmission.
//!
//! This module implements a leaky bucket algorithm for pacing packet
//! transmissions to avoid bursts that could cause congestion.
//!
//! Translated from picoquic/pacing.c (standalone functions only).
//! Functions that use picoquic_path_t or picoquic_quic_t remain in C.

/// Pacing state structure matching C `picoquic_pacing_t`.
///
/// Uses a leaky bucket algorithm where:
/// - `bucket_nanosec` accumulates transmission credits over time
/// - `packet_time_nanosec` is the cost to send one full-size packet
/// - Transmission is allowed when bucket >= packet_time
#[repr(C)]
pub struct PacingState {
    /// Current pacing rate in bytes per second.
    pub rate: u64,
    /// Last time the bucket was evaluated (microseconds).
    pub evaluation_time: u64,
    /// Maximum bucket capacity (nanoseconds).
    pub bucket_max: i64,
    /// Packet time in microseconds (for external use).
    pub packet_time_microsec: u64,
    /// Maximum quantum seen.
    pub quantum_max: u64,
    /// Maximum rate seen.
    pub rate_max: u64,
    /// Flag indicating bandwidth pause.
    pub bandwidth_pause: i32,
    /// Current bucket level (nanoseconds). Can go negative.
    pub bucket_nanosec: i64,
    /// Time to send one full packet (nanoseconds).
    pub packet_time_nanosec: i64,
}

impl PacingState {
    /// Initialize pacing state to high speed default.
    pub fn init(&mut self, current_time: u64) {
        self.evaluation_time = current_time;
        self.bucket_nanosec = 16;
        self.bucket_max = 16;
        self.packet_time_nanosec = 1;
        self.packet_time_microsec = 1;
        // Other fields default to 0
        self.rate = 0;
        self.quantum_max = 0;
        self.rate_max = 0;
        self.bandwidth_pause = 0;
    }

    /// Update the leaky bucket based on elapsed time.
    fn update_bucket(&mut self, current_time: u64) {
        // Prevent bucket from going too negative
        if self.bucket_nanosec < -self.packet_time_nanosec {
            self.bucket_nanosec = -self.packet_time_nanosec;
        }

        if current_time > self.evaluation_time {
            // Add nanoseconds of credit for elapsed microseconds
            self.bucket_nanosec += ((current_time - self.evaluation_time) * 1000) as i64;
            self.evaluation_time = current_time;

            // Cap at maximum
            if self.bucket_nanosec > self.bucket_max {
                self.bucket_nanosec = self.bucket_max;
            }
        }
    }

    /// Check whether pacing blocks transmission.
    ///
    /// Returns `true` if transmission is blocked (not enough credits).
    pub fn is_blocked(&self) -> bool {
        self.bucket_nanosec < self.packet_time_nanosec
    }

    /// Update pacing state after sending a packet.
    ///
    /// Deducts transmission time from the bucket based on packet length.
    pub fn after_send(&mut self, length: usize, send_mtu: usize, current_time: u64) {
        self.update_bucket(current_time);

        // Calculate packet time proportional to length vs MTU
        let packet_time_nanosec =
            (self.packet_time_nanosec as u64 * length as u64).div_ceil(send_mtu as u64);

        self.bucket_nanosec -= packet_time_nanosec as i64;
    }
}

// =============================================================================
// FFI exports
// =============================================================================

/// FFI export: Initialize pacing state.
///
/// # Safety
/// `pacing` must point to a valid `PacingState` struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_pacing_init(pacing: *mut PacingState, current_time: u64) {
    (*pacing).init(current_time);
}

/// FFI export: Check if pacing blocks transmission.
///
/// Returns 1 if blocked, 0 if transmission is allowed.
///
/// # Safety
/// `pacing` must point to a valid `PacingState` struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_is_pacing_blocked(pacing: *const PacingState) -> std::ffi::c_int {
    if (*pacing).is_blocked() {
        1
    } else {
        0
    }
}

/// FFI export: Update pacing after sending a packet.
///
/// # Safety
/// `pacing` must point to a valid `PacingState` struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_update_pacing_data_after_send(
    pacing: *mut PacingState,
    length: usize,
    send_mtu: usize,
    current_time: u64,
) {
    (*pacing).after_send(length, send_mtu, current_time);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacing_init() {
        let mut pacing = PacingState {
            rate: 0,
            evaluation_time: 0,
            bucket_max: 0,
            packet_time_microsec: 0,
            quantum_max: 0,
            rate_max: 0,
            bandwidth_pause: 0,
            bucket_nanosec: 0,
            packet_time_nanosec: 0,
        };

        pacing.init(1000);

        assert_eq!(pacing.evaluation_time, 1000);
        assert_eq!(pacing.bucket_nanosec, 16);
        assert_eq!(pacing.bucket_max, 16);
        assert_eq!(pacing.packet_time_nanosec, 1);
        assert_eq!(pacing.packet_time_microsec, 1);
    }

    #[test]
    fn test_pacing_is_blocked() {
        let mut pacing = PacingState {
            rate: 0,
            evaluation_time: 0,
            bucket_max: 1000,
            packet_time_microsec: 1,
            quantum_max: 0,
            rate_max: 0,
            bandwidth_pause: 0,
            bucket_nanosec: 100,
            packet_time_nanosec: 50,
        };

        // bucket (100) >= packet_time (50), so NOT blocked
        assert!(!pacing.is_blocked());

        pacing.bucket_nanosec = 10;
        // bucket (10) < packet_time (50), so blocked
        assert!(pacing.is_blocked());
    }

    #[test]
    fn test_pacing_update_bucket() {
        let mut pacing = PacingState {
            rate: 0,
            evaluation_time: 1000,
            bucket_max: 10000,
            packet_time_microsec: 1,
            quantum_max: 0,
            rate_max: 0,
            bandwidth_pause: 0,
            bucket_nanosec: 100,
            packet_time_nanosec: 50,
        };

        // Advance time by 5 microseconds = 5000 nanoseconds of credit
        pacing.update_bucket(1005);

        assert_eq!(pacing.evaluation_time, 1005);
        assert_eq!(pacing.bucket_nanosec, 5100); // 100 + 5000
    }

    #[test]
    fn test_pacing_after_send() {
        let mut pacing = PacingState {
            rate: 0,
            evaluation_time: 1000,
            bucket_max: 10000,
            packet_time_microsec: 1,
            quantum_max: 0,
            rate_max: 0,
            bandwidth_pause: 0,
            bucket_nanosec: 1000,
            packet_time_nanosec: 100, // 100ns per full MTU packet
        };

        // Send a full MTU packet
        pacing.after_send(1200, 1200, 1000);
        assert_eq!(pacing.bucket_nanosec, 900); // 1000 - 100

        // Send a half MTU packet
        pacing.after_send(600, 1200, 1000);
        // (100 * 600 + 1199) / 1200 = 50
        assert_eq!(pacing.bucket_nanosec, 850); // 900 - 50
    }
}
