//! Timing and RTT management.
//!
//! Translated from picoquic/timing.c.
//!
//! This module provides:
//! - Retransmit timer computation with exponential backoff
//! - RTT estimation and smoothing
//! - One-way delay estimation
//!
//! The algorithms follow RFC 9002 (QUIC Loss Detection and Congestion Control).

use std::ffi::c_int;

// =============================================================================
// Timing Constants (from picoquic_internal.h)
// =============================================================================

/// Target satellite RTT (610ms) - practical maximum for non-pathological RTT.
pub const TARGET_SATELLITE_RTT: u64 = 610_000;

/// Initial maximum retransmit timer (1 second).
pub const INITIAL_MAX_RETRANSMIT_TIMER: u64 = 1_000_000;

/// Large retransmit timer threshold (2 seconds).
pub const LARGE_RETRANSMIT_TIMER: u64 = 2_000_000;

/// Maximum handshake time (30 seconds).
pub const MICROSEC_HANDSHAKE_MAX: u64 = 30_000_000;

/// Connection state threshold for "ready" state.
/// States below this are handshake states.
pub const STATE_CLIENT_READY_START: u32 = 13;

// =============================================================================
// RTT State
// =============================================================================

/// RTT measurement state for a path.
///
/// This struct holds the RTT-related fields from picoquic_path_t.
#[derive(Debug, Clone, Default)]
pub struct RttState {
    /// Base retransmit timer value.
    pub retransmit_timer: u64,
    /// Number of retransmissions since last ACK.
    pub nb_retransmit: u64,
    /// Most recent RTT sample.
    pub rtt_sample: u64,
    /// Smoothed RTT (EWMA).
    pub smoothed_rtt: u64,
    /// RTT variance.
    pub rtt_variant: u64,
    /// Minimum RTT observed.
    pub rtt_min: u64,
    /// Maximum RTT observed.
    pub rtt_max: u64,
    /// Whether RTT has been initialized.
    pub rtt_is_initialized: bool,
    /// One-way delay sample.
    pub one_way_delay_sample: u64,
    /// Number of delay outliers detected.
    pub nb_delay_outliers: u64,
    // Period tracking for batched updates
    /// Packet number at start of measurement period.
    pub rtt_packet_previous_period: u64,
    /// Time at start of measurement period.
    pub rtt_time_previous_period: u64,
    /// Number of RTT estimates in current period.
    pub nb_rtt_estimate_in_period: u64,
    /// Sum of RTT estimates in current period.
    pub sum_rtt_estimate_in_period: u64,
    /// Minimum RTT in current period.
    pub min_rtt_estimate_in_period: u64,
    /// Maximum RTT in current period.
    pub max_rtt_estimate_in_period: u64,
}

impl RttState {
    /// Create a new RTT state with default values.
    pub fn new() -> Self {
        Self {
            min_rtt_estimate_in_period: u64::MAX,
            ..Default::default()
        }
    }

    /// Reset period counters after an update.
    pub fn reset_period(&mut self) {
        self.nb_rtt_estimate_in_period = 0;
        self.sum_rtt_estimate_in_period = 0;
        self.max_rtt_estimate_in_period = 0;
        self.min_rtt_estimate_in_period = u64::MAX;
    }
}

/// Connection timing parameters.
#[derive(Debug, Clone, Default)]
pub struct ConnectionTiming {
    /// Idle timeout in microseconds.
    pub idle_timeout: u64,
    /// Connection state (picoquic_state_enum).
    pub cnx_state: u32,
    /// Maximum idle timeout from local parameters (milliseconds).
    pub local_max_idle_timeout: u64,
    /// Maximum ACK delay from remote parameters.
    pub remote_max_ack_delay: u64,
    /// Start time of connection.
    pub start_time: u64,
    /// Phase delay for one-way measurements.
    pub phase_delay: i64,
    /// Whether client mode.
    pub client_mode: bool,
    /// Whether timestamp extension is enabled.
    pub is_time_stamp_enabled: bool,
}

// =============================================================================
// Retransmit Timer Computation
// =============================================================================

/// Compute the current retransmit timer with exponential backoff.
///
/// The algorithm:
/// 1. Start with base retransmit_timer
/// 2. For retransmits 1-2: double the timer each time (shift left)
/// 3. For retransmits 3+: use a smoother exponential curve
/// 4. Cap at idle_timeout/16 if idle_timeout > 15
/// 5. During handshake: cap at INITIAL_MAX_RETRANSMIT_TIMER
/// 6. After ready: cap at LARGE_RETRANSMIT_TIMER (or 1.5x smoothed_rtt for satellite)
///
/// # Arguments
/// * `rtt` - RTT state for the path
/// * `conn` - Connection timing parameters
///
/// # Returns
/// The computed retransmit timeout in microseconds
pub fn current_retransmit_timer(rtt: &RttState, conn: &ConnectionTiming) -> u64 {
    let mut rto = rtt.retransmit_timer;

    // Apply exponential backoff based on retransmit count
    if rtt.nb_retransmit > 0 {
        if rtt.nb_retransmit < 3 {
            // Simple doubling for first 2 retransmits
            rto <<= rtt.nb_retransmit;
        } else {
            // Smoother curve for subsequent retransmits
            let mut n1 = rtt.nb_retransmit - 2;
            if n1 > 18 {
                n1 = 18;
            }
            rto <<= 2 + (n1 / 4);
            let n1_mod = n1 & 3;
            rto += (n1_mod * rto) >> 2;
        }

        // Cap at fraction of idle timeout
        if conn.idle_timeout > 15 {
            let max_rto = conn.idle_timeout >> 4;
            if rto > max_rto {
                rto = max_rto;
            }
        }
    }

    // Apply state-specific limits
    if conn.cnx_state < STATE_CLIENT_READY_START {
        // During handshake
        if MICROSEC_HANDSHAKE_MAX / 1000 < conn.local_max_idle_timeout {
            // Special case of very long delays
            rto = rtt.retransmit_timer << rtt.nb_retransmit;
            let max_rto = conn.local_max_idle_timeout * 100;
            if rto > max_rto {
                rto = max_rto;
            }
        } else if rto > INITIAL_MAX_RETRANSMIT_TIMER {
            rto = INITIAL_MAX_RETRANSMIT_TIMER;
        }
    } else if rto > LARGE_RETRANSMIT_TIMER {
        // After connection ready
        let mut alt_rto = LARGE_RETRANSMIT_TIMER;
        if rtt.rtt_min > TARGET_SATELLITE_RTT {
            // For satellite links, use 1.5x smoothed RTT
            alt_rto = (rtt.smoothed_rtt * 3) >> 1;
        }
        if alt_rto < rto {
            rto = alt_rto;
        }
    }

    rto
}

// =============================================================================
// One-Way Delay Estimation
// =============================================================================

/// Update one-way delay estimate from timestamp.
///
/// This function estimates the one-way delay using the timestamp extension.
/// It handles clock drift by adjusting the phase_delay estimate.
///
/// # Arguments
/// * `rtt` - RTT state to update
/// * `conn` - Connection timing (will update phase_delay)
/// * `send_time` - Time the packet was sent
/// * `current_time` - Current time
/// * `ack_delay` - ACK delay reported by peer
/// * `time_stamp` - Timestamp from peer (0 if not available)
pub fn update_one_way_delay(
    rtt: &mut RttState,
    conn: &mut ConnectionTiming,
    send_time: u64,
    current_time: u64,
    ack_delay: u64,
    time_stamp: u64,
) {
    if time_stamp == 0 {
        return;
    }

    // Initialize phase delay if not yet known
    if conn.phase_delay == i64::MAX {
        conn.phase_delay = (rtt.rtt_sample / 2) as i64;
        if !conn.client_mode {
            conn.phase_delay = -conn.phase_delay;
        }
    }

    // Compute local timestamp equivalent
    let time_stamp_local_raw =
        time_stamp as i64 - ack_delay as i64 + conn.start_time as i64 + conn.phase_delay;
    let mut is_time_stamp_valid = true;

    // Validate and potentially adjust phase
    if time_stamp_local_raw < 0 || (time_stamp_local_raw as u64) < send_time {
        // Timestamp appears too early - adjust phase
        let min_phase =
            send_time as i64 - time_stamp as i64 + ack_delay as i64 - conn.start_time as i64;
        let adjusted = time_stamp as i64 - ack_delay as i64 + conn.start_time as i64 + min_phase;
        if adjusted > 0 && (adjusted as u64) <= current_time {
            conn.phase_delay = min_phase;
        } else {
            is_time_stamp_valid = false;
        }
    } else if (time_stamp_local_raw as u64) > current_time {
        // Timestamp appears too late - adjust phase
        let max_phase =
            current_time as i64 - time_stamp as i64 + ack_delay as i64 - conn.start_time as i64;
        let adjusted = time_stamp as i64 - ack_delay as i64 + conn.start_time as i64 + max_phase;
        if adjusted > 0 && (adjusted as u64) >= send_time {
            conn.phase_delay = max_phase;
        } else {
            is_time_stamp_valid = false;
        }
    }

    if is_time_stamp_valid {
        let time_stamp_local =
            time_stamp as i64 - ack_delay as i64 + conn.start_time as i64 + conn.phase_delay;
        rtt.one_way_delay_sample = (time_stamp_local - send_time as i64) as u64;
    } else {
        rtt.nb_delay_outliers += 1;
    }
}

// =============================================================================
// RTT Update
// =============================================================================

/// Result of RTT update containing values to pass to congestion control.
#[derive(Debug, Clone, Default)]
pub struct RttUpdateResult {
    /// The RTT estimate used for this update.
    pub rtt_estimate: u64,
    /// One-way delay (if timestamp enabled).
    pub one_way_delay: u64,
    /// Whether this was the first RTT sample.
    pub is_first: bool,
    /// Whether the smoothed RTT was updated this call.
    pub smoothed_updated: bool,
}

/// Update RTT estimates from an ACK.
///
/// This implements the RTT estimation algorithm from RFC 9002:
/// - Compute RTT sample from send_time and current_time
/// - Subtract ack_delay (with limits)
/// - Update min/max RTT
/// - Batch samples over a measurement period
/// - Update smoothed RTT and variance periodically
///
/// # Arguments
/// * `rtt` - RTT state to update
/// * `conn` - Connection timing parameters
/// * `send_time` - Time the acknowledged packet was sent
/// * `current_time` - Current time
/// * `ack_delay` - ACK delay reported by peer
/// * `time_stamp` - Timestamp from peer (0 if not available)
/// * `highest_acked` - Highest packet number acknowledged (for period tracking)
///
/// # Returns
/// RTT update result with values for congestion control notification
pub fn update_rtt(
    rtt: &mut RttState,
    conn: &mut ConnectionTiming,
    send_time: u64,
    current_time: u64,
    ack_delay: u64,
    time_stamp: u64,
    highest_acked: u64,
) -> RttUpdateResult {
    let mut result = RttUpdateResult::default();
    let is_first = !rtt.rtt_is_initialized;
    result.is_first = is_first;

    // Compute raw RTT estimate
    let mut rtt_estimate = current_time.saturating_sub(send_time);

    // Subtract ack_delay (with validation)
    if !is_first && ack_delay > 0 && conn.cnx_state >= STATE_CLIENT_READY_START {
        let max_delay = conn.remote_max_ack_delay;
        let effective_delay = if ack_delay > max_delay {
            max_delay
        } else {
            ack_delay
        };
        if rtt.rtt_min + effective_delay < rtt_estimate {
            rtt_estimate -= effective_delay;
        }
    }

    rtt.rtt_sample = rtt_estimate;
    result.rtt_estimate = rtt_estimate;

    // Accumulate samples for the measurement period
    rtt.nb_rtt_estimate_in_period += 1;
    rtt.sum_rtt_estimate_in_period += rtt_estimate;

    if rtt.nb_rtt_estimate_in_period == 1 {
        rtt.min_rtt_estimate_in_period = rtt_estimate;
        rtt.max_rtt_estimate_in_period = rtt_estimate;
    } else {
        if rtt_estimate > rtt.max_rtt_estimate_in_period {
            rtt.max_rtt_estimate_in_period = rtt_estimate;
        }
        if rtt_estimate < rtt.min_rtt_estimate_in_period {
            rtt.min_rtt_estimate_in_period = rtt_estimate;
        }
    }

    // Update instantaneous bounds
    if rtt.retransmit_timer < rtt_estimate {
        rtt.retransmit_timer = rtt_estimate;
    }
    if rtt.rtt_min > rtt_estimate {
        rtt.rtt_min = rtt_estimate;
    }
    if rtt.rtt_max < rtt_estimate {
        rtt.rtt_max = rtt_estimate;
    }

    // Update one-way delay if timestamp available
    if time_stamp > 0 {
        update_one_way_delay(rtt, conn, send_time, current_time, ack_delay, time_stamp);
    }
    if conn.is_time_stamp_enabled {
        result.one_way_delay = rtt.one_way_delay_sample;
    }

    // Check if we should update smoothed RTT (end of period)
    let should_update = highest_acked > rtt.rtt_packet_previous_period
        || rtt.rtt_time_previous_period + (rtt_estimate / 4) > current_time;

    if should_update {
        result.smoothed_updated = true;
        rtt.rtt_time_previous_period = current_time;

        // Use average if we have multiple samples
        let period_estimate = if rtt.nb_rtt_estimate_in_period > 1 {
            rtt.sum_rtt_estimate_in_period / rtt.nb_rtt_estimate_in_period
        } else {
            rtt_estimate
        };

        if is_first {
            // First sample: initialize
            rtt.smoothed_rtt = period_estimate;
            rtt.rtt_variant = period_estimate / 2;
            rtt.rtt_min = rtt.min_rtt_estimate_in_period;
            rtt.rtt_is_initialized = true;
        } else {
            // Subsequent samples: EWMA update
            // Compute variance sample from min/max in period
            let rtt_var_sample = if rtt.smoothed_rtt > rtt.max_rtt_estimate_in_period {
                rtt.smoothed_rtt - rtt.min_rtt_estimate_in_period
            } else if rtt.smoothed_rtt < rtt.min_rtt_estimate_in_period {
                rtt.max_rtt_estimate_in_period - rtt.smoothed_rtt
            } else {
                let var_min = rtt.smoothed_rtt - rtt.min_rtt_estimate_in_period;
                let var_max = rtt.max_rtt_estimate_in_period - rtt.smoothed_rtt;
                if var_min > var_max {
                    var_min
                } else {
                    var_max
                }
            };

            // EWMA: rtt_variant = 3/4 * rtt_variant + 1/4 * sample
            rtt.rtt_variant = (3 * rtt.rtt_variant + rtt_var_sample) / 4;
            // EWMA: smoothed_rtt = 7/8 * smoothed_rtt + 1/8 * estimate
            rtt.smoothed_rtt = (7 * rtt.smoothed_rtt + period_estimate) / 8;
        }

        // Update retransmit timer: smoothed + 3*variant + max_ack_delay
        rtt.retransmit_timer = rtt.smoothed_rtt + 3 * rtt.rtt_variant + conn.remote_max_ack_delay;

        // Reset period counters
        rtt.rtt_packet_previous_period = highest_acked;
        rtt.reset_period();
    }

    result
}

// =============================================================================
// FFI Exports
// =============================================================================

/// C-compatible RTT state for FFI.
#[repr(C)]
pub struct CRttState {
    pub retransmit_timer: u64,
    pub nb_retransmit: u64,
    pub rtt_sample: u64,
    pub smoothed_rtt: u64,
    pub rtt_variant: u64,
    pub rtt_min: u64,
    pub rtt_max: u64,
    pub rtt_is_initialized: c_int,
    pub one_way_delay_sample: u64,
    pub nb_delay_outliers: u64,
    pub rtt_packet_previous_period: u64,
    pub rtt_time_previous_period: u64,
    pub nb_rtt_estimate_in_period: u64,
    pub sum_rtt_estimate_in_period: u64,
    pub min_rtt_estimate_in_period: u64,
    pub max_rtt_estimate_in_period: u64,
}

/// C-compatible connection timing for FFI.
#[repr(C)]
pub struct CConnectionTiming {
    pub idle_timeout: u64,
    pub cnx_state: u32,
    pub local_max_idle_timeout: u64,
    pub remote_max_ack_delay: u64,
    pub start_time: u64,
    pub phase_delay: i64,
    pub client_mode: c_int,
    pub is_time_stamp_enabled: c_int,
}

impl From<&CRttState> for RttState {
    fn from(c: &CRttState) -> Self {
        Self {
            retransmit_timer: c.retransmit_timer,
            nb_retransmit: c.nb_retransmit,
            rtt_sample: c.rtt_sample,
            smoothed_rtt: c.smoothed_rtt,
            rtt_variant: c.rtt_variant,
            rtt_min: c.rtt_min,
            rtt_max: c.rtt_max,
            rtt_is_initialized: c.rtt_is_initialized != 0,
            one_way_delay_sample: c.one_way_delay_sample,
            nb_delay_outliers: c.nb_delay_outliers,
            rtt_packet_previous_period: c.rtt_packet_previous_period,
            rtt_time_previous_period: c.rtt_time_previous_period,
            nb_rtt_estimate_in_period: c.nb_rtt_estimate_in_period,
            sum_rtt_estimate_in_period: c.sum_rtt_estimate_in_period,
            min_rtt_estimate_in_period: c.min_rtt_estimate_in_period,
            max_rtt_estimate_in_period: c.max_rtt_estimate_in_period,
        }
    }
}

impl RttState {
    /// Sync state back to C struct.
    pub fn sync_to_c(&self, c: &mut CRttState) {
        c.retransmit_timer = self.retransmit_timer;
        c.nb_retransmit = self.nb_retransmit;
        c.rtt_sample = self.rtt_sample;
        c.smoothed_rtt = self.smoothed_rtt;
        c.rtt_variant = self.rtt_variant;
        c.rtt_min = self.rtt_min;
        c.rtt_max = self.rtt_max;
        c.rtt_is_initialized = self.rtt_is_initialized as c_int;
        c.one_way_delay_sample = self.one_way_delay_sample;
        c.nb_delay_outliers = self.nb_delay_outliers;
        c.rtt_packet_previous_period = self.rtt_packet_previous_period;
        c.rtt_time_previous_period = self.rtt_time_previous_period;
        c.nb_rtt_estimate_in_period = self.nb_rtt_estimate_in_period;
        c.sum_rtt_estimate_in_period = self.sum_rtt_estimate_in_period;
        c.min_rtt_estimate_in_period = self.min_rtt_estimate_in_period;
        c.max_rtt_estimate_in_period = self.max_rtt_estimate_in_period;
    }
}

impl From<&CConnectionTiming> for ConnectionTiming {
    fn from(c: &CConnectionTiming) -> Self {
        Self {
            idle_timeout: c.idle_timeout,
            cnx_state: c.cnx_state,
            local_max_idle_timeout: c.local_max_idle_timeout,
            remote_max_ack_delay: c.remote_max_ack_delay,
            start_time: c.start_time,
            phase_delay: c.phase_delay,
            client_mode: c.client_mode != 0,
            is_time_stamp_enabled: c.is_time_stamp_enabled != 0,
        }
    }
}

impl ConnectionTiming {
    /// Sync phase_delay back to C struct.
    pub fn sync_to_c(&self, c: &mut CConnectionTiming) {
        c.phase_delay = self.phase_delay;
    }
}

/// Compute retransmit timer (FFI export).
///
/// # Safety
/// The `rtt` and `conn` pointers must be valid and point to initialized structs.
#[no_mangle]
pub unsafe extern "C" fn fq_current_retransmit_timer(
    rtt: *const CRttState,
    conn: *const CConnectionTiming,
) -> u64 {
    let rtt_state = RttState::from(&*rtt);
    let conn_timing = ConnectionTiming::from(&*conn);
    current_retransmit_timer(&rtt_state, &conn_timing)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(TARGET_SATELLITE_RTT, 610_000);
        assert_eq!(INITIAL_MAX_RETRANSMIT_TIMER, 1_000_000);
        assert_eq!(LARGE_RETRANSMIT_TIMER, 2_000_000);
        assert_eq!(MICROSEC_HANDSHAKE_MAX, 30_000_000);
    }

    #[test]
    fn test_rtt_state_new() {
        let rtt = RttState::new();
        assert_eq!(rtt.min_rtt_estimate_in_period, u64::MAX);
        assert!(!rtt.rtt_is_initialized);
    }

    #[test]
    fn test_current_retransmit_timer_no_retransmit() {
        let rtt = RttState {
            retransmit_timer: 100_000,
            nb_retransmit: 0,
            ..Default::default()
        };
        let conn = ConnectionTiming {
            cnx_state: STATE_CLIENT_READY_START + 1,
            ..Default::default()
        };
        assert_eq!(current_retransmit_timer(&rtt, &conn), 100_000);
    }

    #[test]
    fn test_current_retransmit_timer_first_retransmit() {
        let rtt = RttState {
            retransmit_timer: 100_000,
            nb_retransmit: 1,
            ..Default::default()
        };
        let conn = ConnectionTiming {
            cnx_state: STATE_CLIENT_READY_START + 1,
            ..Default::default()
        };
        // Should double (shift left by 1)
        assert_eq!(current_retransmit_timer(&rtt, &conn), 200_000);
    }

    #[test]
    fn test_current_retransmit_timer_second_retransmit() {
        let rtt = RttState {
            retransmit_timer: 100_000,
            nb_retransmit: 2,
            ..Default::default()
        };
        let conn = ConnectionTiming {
            cnx_state: STATE_CLIENT_READY_START + 1,
            ..Default::default()
        };
        // Should quadruple (shift left by 2)
        assert_eq!(current_retransmit_timer(&rtt, &conn), 400_000);
    }

    #[test]
    fn test_current_retransmit_timer_cap_at_large() {
        let rtt = RttState {
            retransmit_timer: 500_000,
            nb_retransmit: 5,
            smoothed_rtt: 100_000,
            rtt_min: 50_000, // Not satellite
            ..Default::default()
        };
        let conn = ConnectionTiming {
            cnx_state: STATE_CLIENT_READY_START + 1,
            ..Default::default()
        };
        let timer = current_retransmit_timer(&rtt, &conn);
        assert!(timer <= LARGE_RETRANSMIT_TIMER);
    }

    #[test]
    fn test_current_retransmit_timer_satellite() {
        let rtt = RttState {
            retransmit_timer: 800_000,
            nb_retransmit: 3,
            smoothed_rtt: 700_000,
            rtt_min: TARGET_SATELLITE_RTT + 1, // Satellite link
            ..Default::default()
        };
        let conn = ConnectionTiming {
            cnx_state: STATE_CLIENT_READY_START + 1,
            ..Default::default()
        };
        let timer = current_retransmit_timer(&rtt, &conn);
        // Should use 1.5x smoothed_rtt = 1_050_000
        assert_eq!(timer, 1_050_000);
    }

    #[test]
    fn test_current_retransmit_timer_handshake() {
        let rtt = RttState {
            retransmit_timer: 500_000,
            nb_retransmit: 3,
            ..Default::default()
        };
        let conn = ConnectionTiming {
            cnx_state: 5, // During handshake
            local_max_idle_timeout: 10_000,
            ..Default::default()
        };
        let timer = current_retransmit_timer(&rtt, &conn);
        assert!(timer <= INITIAL_MAX_RETRANSMIT_TIMER);
    }

    #[test]
    fn test_update_rtt_first_sample() {
        let mut rtt = RttState::new();
        let mut conn = ConnectionTiming {
            cnx_state: STATE_CLIENT_READY_START + 1,
            remote_max_ack_delay: 25_000,
            ..Default::default()
        };

        let send_time = 1_000_000;
        let current_time = 1_100_000; // 100ms RTT
        let result = update_rtt(&mut rtt, &mut conn, send_time, current_time, 0, 0, 1);

        assert!(result.is_first);
        assert!(rtt.rtt_is_initialized);
        assert_eq!(rtt.smoothed_rtt, 100_000);
        assert_eq!(rtt.rtt_variant, 50_000); // half of first sample
        assert_eq!(rtt.rtt_min, 100_000);
    }

    #[test]
    fn test_update_rtt_subsequent_sample() {
        let mut rtt = RttState {
            rtt_is_initialized: true,
            smoothed_rtt: 100_000,
            rtt_variant: 10_000,
            rtt_min: 90_000,
            rtt_max: 110_000,
            retransmit_timer: 130_000,
            min_rtt_estimate_in_period: u64::MAX,
            ..Default::default()
        };
        let mut conn = ConnectionTiming {
            cnx_state: STATE_CLIENT_READY_START + 1,
            remote_max_ack_delay: 25_000,
            ..Default::default()
        };

        let send_time = 2_000_000;
        let current_time = 2_095_000; // 95ms RTT
        let result = update_rtt(&mut rtt, &mut conn, send_time, current_time, 0, 0, 2);

        assert!(!result.is_first);
        // Smoothed should move slightly toward 95ms
        assert!(rtt.smoothed_rtt < 100_000);
        assert!(rtt.smoothed_rtt > 95_000);
    }

    #[test]
    fn test_update_rtt_with_ack_delay() {
        let mut rtt = RttState {
            rtt_is_initialized: true,
            smoothed_rtt: 100_000,
            rtt_variant: 10_000,
            rtt_min: 90_000,
            retransmit_timer: 130_000,
            min_rtt_estimate_in_period: u64::MAX,
            ..Default::default()
        };
        let mut conn = ConnectionTiming {
            cnx_state: STATE_CLIENT_READY_START + 1,
            remote_max_ack_delay: 25_000,
            ..Default::default()
        };

        let send_time = 3_000_000;
        let current_time = 3_120_000; // 120ms including 20ms ack delay
        let ack_delay = 20_000;
        let result = update_rtt(
            &mut rtt,
            &mut conn,
            send_time,
            current_time,
            ack_delay,
            0,
            3,
        );

        // RTT sample should be 100ms (120ms - 20ms ack delay)
        assert_eq!(result.rtt_estimate, 100_000);
    }

    #[test]
    fn test_update_one_way_delay() {
        let mut rtt = RttState {
            rtt_sample: 100_000,
            ..Default::default()
        };
        let mut conn = ConnectionTiming {
            start_time: 0,
            phase_delay: i64::MAX, // Will be initialized
            client_mode: true,
            ..Default::default()
        };

        let send_time = 1_000_000;
        let current_time = 1_100_000;
        let ack_delay = 5_000;
        let time_stamp = 1_050_000; // Peer's timestamp

        update_one_way_delay(
            &mut rtt,
            &mut conn,
            send_time,
            current_time,
            ack_delay,
            time_stamp,
        );

        // Phase should be initialized to rtt_sample/2 = 50_000
        assert_eq!(conn.phase_delay, 50_000);
        // One-way delay should be computed
        assert!(rtt.one_way_delay_sample > 0);
    }
}
