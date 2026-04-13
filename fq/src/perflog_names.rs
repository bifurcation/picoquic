//! Performance log parameter names and constants.
//!
//! Translated from picoquic/performance_log.c (picoquic_perflog_param_name).
//!
//! This module provides string names for performance log columns
//! as used in picoquic CSV performance logging.

use std::ffi::CStr;

// =============================================================================
// Performance Log Column Constants (from performance_log.h)
// =============================================================================

/// Performance log column indices.
pub mod perflog {
    /// Is client (boolean).
    pub const IS_CLIENT: u32 = 0;
    /// Number of packets received.
    pub const NB_PACKETS_RECEIVED: u32 = 1;
    /// Number of packet trains sent.
    pub const NB_TRAINS_SENT: u32 = 2;
    /// Number of short trains.
    pub const NB_TRAINS_SHORT: u32 = 3;
    /// Trains blocked by congestion window.
    pub const NB_TRAINS_BLOCKED_CWIN: u32 = 4;
    /// Trains blocked by pacing.
    pub const NB_TRAINS_BLOCKED_PACING: u32 = 5;
    /// Trains blocked by other reasons.
    pub const NB_TRAINS_BLOCKED_OTHERS: u32 = 6;
    /// Number of packets sent.
    pub const NB_PACKETS_SENT: u32 = 7;
    /// Total retransmissions.
    pub const NB_RETRANSMISSION_TOTAL: u32 = 8;
    /// Spurious retransmissions.
    pub const NB_SPURIOUS: u32 = 9;
    /// Delayed ACK option.
    pub const DELAYED_ACK_OPTION: u32 = 10;
    /// Minimum ACK delay (remote).
    pub const MIN_ACK_DELAY_REMOTE: u32 = 11;
    /// Maximum ACK delay (remote).
    pub const MAX_ACK_DELAY_REMOTE: u32 = 12;
    /// Maximum ACK gap (remote).
    pub const MAX_ACK_GAP_REMOTE: u32 = 13;
    /// Minimum ACK delay (local).
    pub const MIN_ACK_DELAY_LOCAL: u32 = 14;
    /// Maximum ACK delay (local).
    pub const MAX_ACK_DELAY_LOCAL: u32 = 15;
    /// Maximum ACK gap (local).
    pub const MAX_ACK_GAP_LOCAL: u32 = 16;
    /// Maximum MTU sent.
    pub const MAX_MTU_SENT: u32 = 17;
    /// Maximum MTU received.
    pub const MAX_MTU_RECEIVED: u32 = 18;
    /// 0-RTT status.
    pub const ZERO_RTT: u32 = 19;
    /// Smoothed RTT.
    pub const SRTT: u32 = 20;
    /// Minimum RTT.
    pub const MINRTT: u32 = 21;
    /// Congestion window.
    pub const CWIN: u32 = 22;
    /// Congestion control algorithm.
    pub const CCALGO: u32 = 23;
    /// Maximum bandwidth estimate.
    pub const BWE_MAX: u32 = 24;
    /// Maximum pacing quantum.
    pub const PACING_QUANTUM_MAX: u32 = 25;
    /// Pacing rate.
    pub const PACING_RATE: u32 = 26;
}

// =============================================================================
// Performance Log Parameter Name Lookup (Core Logic)
// =============================================================================

/// Get the name of a performance log parameter as a C string.
///
/// This is the core lookup function - all logic lives here.
/// Returns None if the column index is not recognized.
///
/// # Arguments
/// * `rank` - The column index
///
/// # Returns
/// An Option containing a static CStr with the parameter name, or None
pub fn perflog_param_name_cstr(rank: u32) -> Option<&'static CStr> {
    match rank {
        perflog::IS_CLIENT => Some(c"is_client"),
        perflog::NB_PACKETS_RECEIVED => Some(c"pkt_recv"),
        perflog::NB_TRAINS_SENT => Some(c"trains_s"),
        perflog::NB_TRAINS_SHORT => Some(c"t_short"),
        perflog::NB_TRAINS_BLOCKED_CWIN => Some(c"tb_cwin"),
        perflog::NB_TRAINS_BLOCKED_PACING => Some(c"tb_pacing"),
        perflog::NB_TRAINS_BLOCKED_OTHERS => Some(c"tb_others"),
        perflog::NB_PACKETS_SENT => Some(c"pkt_sent"),
        perflog::NB_RETRANSMISSION_TOTAL => Some(c"retrans."),
        perflog::NB_SPURIOUS => Some(c"spurious"),
        perflog::DELAYED_ACK_OPTION => Some(c"delayed_ack_option"),
        perflog::MIN_ACK_DELAY_REMOTE => Some(c"min_ack_delay_remote"),
        perflog::MAX_ACK_DELAY_REMOTE => Some(c"max_ack_delay_remote"),
        perflog::MAX_ACK_GAP_REMOTE => Some(c"max_ack_gap_remote"),
        perflog::MIN_ACK_DELAY_LOCAL => Some(c"min_ack_delay_local"),
        perflog::MAX_ACK_DELAY_LOCAL => Some(c"max_ack_delay_local"),
        perflog::MAX_ACK_GAP_LOCAL => Some(c"max_ack_gap_local"),
        perflog::MAX_MTU_SENT => Some(c"max_mtu_sent"),
        perflog::MAX_MTU_RECEIVED => Some(c"max_mtu_received"),
        perflog::ZERO_RTT => Some(c"zero_rtt"),
        perflog::SRTT => Some(c"srtt"),
        perflog::MINRTT => Some(c"minrtt"),
        perflog::CWIN => Some(c"cwin"),
        perflog::CCALGO => Some(c"ccalgo"),
        perflog::BWE_MAX => Some(c"bwe_max"),
        perflog::PACING_QUANTUM_MAX => Some(c"p_quantum"),
        perflog::PACING_RATE => Some(c"p_rate"),
        _ => None,
    }
}

/// Get the name of a performance log parameter.
///
/// Convenience wrapper that returns `Option<&str>` for Rust callers.
///
/// # Arguments
/// * `rank` - The column index
///
/// # Returns
/// An Option containing a static string with the parameter name, or None
pub fn perflog_param_name(rank: u32) -> Option<&'static str> {
    perflog_param_name_cstr(rank).map(|cstr| {
        // Safe: all our CStr literals are valid UTF-8
        cstr.to_str().expect("perflog param names are ASCII")
    })
}

// =============================================================================
// FFI Export (Thin Wrapper Only)
// =============================================================================

/// Get performance log parameter name (FFI export).
///
/// Returns a pointer to a null-terminated static string, or NULL if not found.
#[no_mangle]
pub extern "C" fn picoquic_perflog_param_name(rank: u32) -> *const std::ffi::c_char {
    match perflog_param_name_cstr(rank) {
        Some(cstr) => cstr.as_ptr(),
        None => std::ptr::null(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perflog_constants() {
        assert_eq!(perflog::IS_CLIENT, 0);
        assert_eq!(perflog::NB_PACKETS_RECEIVED, 1);
        assert_eq!(perflog::NB_TRAINS_SENT, 2);
        assert_eq!(perflog::NB_TRAINS_SHORT, 3);
        assert_eq!(perflog::NB_TRAINS_BLOCKED_CWIN, 4);
        assert_eq!(perflog::NB_TRAINS_BLOCKED_PACING, 5);
        assert_eq!(perflog::NB_TRAINS_BLOCKED_OTHERS, 6);
        assert_eq!(perflog::NB_PACKETS_SENT, 7);
        assert_eq!(perflog::NB_RETRANSMISSION_TOTAL, 8);
        assert_eq!(perflog::NB_SPURIOUS, 9);
        assert_eq!(perflog::DELAYED_ACK_OPTION, 10);
        assert_eq!(perflog::SRTT, 20);
        assert_eq!(perflog::CWIN, 22);
        assert_eq!(perflog::PACING_RATE, 26);
    }

    #[test]
    fn test_perflog_param_name() {
        assert_eq!(perflog_param_name(perflog::IS_CLIENT), Some("is_client"));
        assert_eq!(
            perflog_param_name(perflog::NB_PACKETS_RECEIVED),
            Some("pkt_recv")
        );
        assert_eq!(
            perflog_param_name(perflog::NB_TRAINS_SENT),
            Some("trains_s")
        );
        assert_eq!(
            perflog_param_name(perflog::NB_TRAINS_BLOCKED_CWIN),
            Some("tb_cwin")
        );
        assert_eq!(
            perflog_param_name(perflog::NB_RETRANSMISSION_TOTAL),
            Some("retrans.")
        );
        assert_eq!(perflog_param_name(perflog::SRTT), Some("srtt"));
        assert_eq!(perflog_param_name(perflog::CWIN), Some("cwin"));
        assert_eq!(perflog_param_name(perflog::CCALGO), Some("ccalgo"));
        assert_eq!(perflog_param_name(perflog::PACING_RATE), Some("p_rate"));
    }

    #[test]
    fn test_perflog_param_name_unknown() {
        assert_eq!(perflog_param_name(100), None);
        assert_eq!(perflog_param_name(0xFFFFFFFF), None);
    }

    #[test]
    fn test_perflog_param_name_cstr() {
        assert_eq!(
            perflog_param_name_cstr(perflog::IS_CLIENT),
            Some(c"is_client")
        );
        assert_eq!(perflog_param_name_cstr(perflog::SRTT), Some(c"srtt"));
        assert_eq!(perflog_param_name_cstr(100), None);
    }

    #[test]
    fn test_perflog_param_name_ffi() {
        use std::ffi::CStr;

        unsafe {
            let name_ptr = picoquic_perflog_param_name(perflog::IS_CLIENT);
            assert!(!name_ptr.is_null());
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "is_client");

            let name_ptr = picoquic_perflog_param_name(perflog::CWIN);
            assert!(!name_ptr.is_null());
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "cwin");

            // Unknown rank returns NULL
            let name_ptr = picoquic_perflog_param_name(100);
            assert!(name_ptr.is_null());
        }
    }
}
