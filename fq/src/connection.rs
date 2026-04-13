//! Connection structure for QUIC connections.
//!
//! This module provides a partial safe Rust representation of picoquic_cnx_t,
//! with only the fields needed by translated functions. Fields are added
//! incrementally as more functions are translated.

use std::ffi::c_int;

// =============================================================================
// Safe Rust Connection struct (partial)
// =============================================================================

/// Partial safe Rust representation of a QUIC connection.
///
/// Only contains fields accessed by currently translated functions.
/// Additional fields will be added as more code is translated.
#[derive(Debug, Clone)]
pub struct Connection {
    /// Whether multipath is enabled for this connection.
    pub is_multipath_enabled: bool,

    /// Whether the connection is blocked by congestion window.
    pub cwin_blocked: bool,

    // Packet context fields for non-multipath mode
    // (picoquic_packet_context_application)
    pub pkt_ctx_app_send_sequence: u64,
    pub pkt_ctx_app_highest_acknowledged: u64,
    pub pkt_ctx_app_latest_time_acknowledged: u64,

    // Primary path fields (path[0])
    pub path0_rtt_min: u64,
}

impl Connection {
    /// Create a new Connection with default values.
    pub fn new() -> Self {
        Self {
            is_multipath_enabled: false,
            cwin_blocked: false,
            pkt_ctx_app_send_sequence: 0,
            pkt_ctx_app_highest_acknowledged: 0,
            pkt_ctx_app_latest_time_acknowledged: 0,
            path0_rtt_min: u64::MAX,
        }
    }

    /// Get the sequence number for CC algorithms.
    ///
    /// In multipath mode, uses the path's packet context.
    /// Otherwise, uses the connection's application packet context.
    pub fn get_cc_sequence_number(&self, path_send_sequence: u64) -> u64 {
        if self.is_multipath_enabled {
            path_send_sequence
        } else {
            self.pkt_ctx_app_send_sequence
        }
    }

    /// Get the highest acknowledged packet number for CC algorithms.
    pub fn get_cc_ack_number(&self, path_highest_acked: u64) -> u64 {
        if self.is_multipath_enabled {
            path_highest_acked
        } else {
            self.pkt_ctx_app_highest_acknowledged
        }
    }

    /// Get the time the highest acked packet was sent.
    pub fn get_cc_ack_sent_time(&self, path_latest_time_acked: u64) -> u64 {
        if self.is_multipath_enabled {
            path_latest_time_acked
        } else {
            self.pkt_ctx_app_latest_time_acknowledged
        }
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// C-compatible structs for FFI
// =============================================================================

/// Minimal C struct for reading connection fields needed by cc_common functions.
#[repr(C)]
pub struct CConnectionView {
    pub is_multipath_enabled: c_int,
    pub cwin_blocked: c_int,
    pub pkt_ctx_app_send_sequence: u64,
    pub pkt_ctx_app_highest_acknowledged: u64,
    pub pkt_ctx_app_latest_time_acknowledged: u64,
    pub path0_rtt_min: u64,
}

impl CConnectionView {
    /// Convert to safe Rust Connection struct.
    pub fn to_rust(&self) -> Connection {
        Connection {
            is_multipath_enabled: self.is_multipath_enabled != 0,
            cwin_blocked: self.cwin_blocked != 0,
            pkt_ctx_app_send_sequence: self.pkt_ctx_app_send_sequence,
            pkt_ctx_app_highest_acknowledged: self.pkt_ctx_app_highest_acknowledged,
            pkt_ctx_app_latest_time_acknowledged: self.pkt_ctx_app_latest_time_acknowledged,
            path0_rtt_min: self.path0_rtt_min,
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
    fn test_connection_default() {
        let cnx = Connection::default();
        assert!(!cnx.is_multipath_enabled);
        assert!(!cnx.cwin_blocked);
        assert_eq!(cnx.path0_rtt_min, u64::MAX);
    }

    #[test]
    fn test_get_cc_sequence_number_multipath() {
        let mut cnx = Connection::default();
        cnx.is_multipath_enabled = true;
        cnx.pkt_ctx_app_send_sequence = 100;

        // In multipath mode, should use path's value
        assert_eq!(cnx.get_cc_sequence_number(50), 50);
    }

    #[test]
    fn test_get_cc_sequence_number_singlepath() {
        let mut cnx = Connection::default();
        cnx.is_multipath_enabled = false;
        cnx.pkt_ctx_app_send_sequence = 100;

        // In single path mode, should use connection's value
        assert_eq!(cnx.get_cc_sequence_number(50), 100);
    }

    #[test]
    fn test_c_connection_view_conversion() {
        let c_view = CConnectionView {
            is_multipath_enabled: 1,
            cwin_blocked: 1,
            pkt_ctx_app_send_sequence: 200,
            pkt_ctx_app_highest_acknowledged: 150,
            pkt_ctx_app_latest_time_acknowledged: 12345,
            path0_rtt_min: 5000,
        };

        let cnx = c_view.to_rust();
        assert!(cnx.is_multipath_enabled);
        assert!(cnx.cwin_blocked);
        assert_eq!(cnx.pkt_ctx_app_send_sequence, 200);
        assert_eq!(cnx.path0_rtt_min, 5000);
    }
}
