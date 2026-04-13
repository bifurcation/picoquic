//! Path structure for QUIC connections.
//!
//! This module provides a partial safe Rust representation of picoquic_path_t,
//! with only the fields needed by translated functions. Fields are added
//! incrementally as more functions are translated.

use std::ffi::c_int;

// =============================================================================
// Safe Rust Path struct (partial)
// =============================================================================

/// Partial safe Rust representation of a QUIC path.
///
/// Only contains fields accessed by currently translated functions.
/// Additional fields will be added as more code is translated.
#[derive(Debug, Clone)]
pub struct Path {
    // RTT and timing
    pub smoothed_rtt: u64,
    pub rtt_min: u64,

    // Congestion control
    pub cwin: u64,
    pub peak_bandwidth_estimate: u64,

    // MTU
    pub send_mtu: u64,

    // Packet context fields (flattened from pkt_ctx for multipath)
    pub pkt_ctx_send_sequence: u64,
    pub pkt_ctx_highest_acknowledged: u64,
    pub pkt_ctx_latest_time_acknowledged: u64,
    /// Sequence number of first pending packet (None if no pending packets)
    pub pkt_ctx_pending_first_sequence: Option<u64>,

    // Back-reference to connection state needed for some operations
    pub cnx_is_multipath_enabled: bool,
    pub cnx_cwin_blocked: bool,
}

impl Path {
    /// Create a new Path with default values.
    pub fn new() -> Self {
        Self {
            smoothed_rtt: 0,
            rtt_min: u64::MAX,
            cwin: 0,
            peak_bandwidth_estimate: 0,
            send_mtu: 1252, // Default MTU
            pkt_ctx_send_sequence: 0,
            pkt_ctx_highest_acknowledged: 0,
            pkt_ctx_latest_time_acknowledged: 0,
            pkt_ctx_pending_first_sequence: None,
            cnx_is_multipath_enabled: false,
            cnx_cwin_blocked: false,
        }
    }

    /// Get the lowest packet number not yet acknowledged.
    ///
    /// Returns the sequence number of the first pending packet, or
    /// highest_acknowledged + 1 if there are no pending packets.
    pub fn get_lowest_not_ack(&self) -> u64 {
        self.pkt_ctx_pending_first_sequence
            .unwrap_or(self.pkt_ctx_highest_acknowledged + 1)
    }
}

impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// C-compatible structs for FFI
// =============================================================================

/// Minimal C struct for reading path fields needed by cc_common functions.
///
/// This is NOT the full picoquic_path_t - it's a view into specific fields.
/// The FFI layer extracts these fields from the actual C struct.
#[repr(C)]
pub struct CPathView {
    pub smoothed_rtt: u64,
    pub rtt_min: u64,
    pub cwin: u64,
    pub peak_bandwidth_estimate: u64,
    pub send_mtu: u64,
    pub pkt_ctx_send_sequence: u64,
    pub pkt_ctx_highest_acknowledged: u64,
    pub pkt_ctx_latest_time_acknowledged: u64,
    /// 0 if no pending packets, otherwise sequence number of first pending
    pub pkt_ctx_pending_first_sequence: u64,
    /// 1 if pending_first is non-null, 0 otherwise
    pub pkt_ctx_has_pending: c_int,
    pub cnx_is_multipath_enabled: c_int,
    pub cnx_cwin_blocked: c_int,
}

impl CPathView {
    /// Convert to safe Rust Path struct.
    pub fn to_rust(&self) -> Path {
        Path {
            smoothed_rtt: self.smoothed_rtt,
            rtt_min: self.rtt_min,
            cwin: self.cwin,
            peak_bandwidth_estimate: self.peak_bandwidth_estimate,
            send_mtu: self.send_mtu,
            pkt_ctx_send_sequence: self.pkt_ctx_send_sequence,
            pkt_ctx_highest_acknowledged: self.pkt_ctx_highest_acknowledged,
            pkt_ctx_latest_time_acknowledged: self.pkt_ctx_latest_time_acknowledged,
            pkt_ctx_pending_first_sequence: if self.pkt_ctx_has_pending != 0 {
                Some(self.pkt_ctx_pending_first_sequence)
            } else {
                None
            },
            cnx_is_multipath_enabled: self.cnx_is_multipath_enabled != 0,
            cnx_cwin_blocked: self.cnx_cwin_blocked != 0,
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
    fn test_path_default() {
        let path = Path::default();
        assert_eq!(path.smoothed_rtt, 0);
        assert_eq!(path.rtt_min, u64::MAX);
        assert_eq!(path.cwin, 0);
        assert!(!path.cnx_is_multipath_enabled);
    }

    #[test]
    fn test_get_lowest_not_ack_with_pending() {
        let mut path = Path::default();
        path.pkt_ctx_highest_acknowledged = 100;
        path.pkt_ctx_pending_first_sequence = Some(50);

        assert_eq!(path.get_lowest_not_ack(), 50);
    }

    #[test]
    fn test_get_lowest_not_ack_no_pending() {
        let mut path = Path::default();
        path.pkt_ctx_highest_acknowledged = 100;
        path.pkt_ctx_pending_first_sequence = None;

        assert_eq!(path.get_lowest_not_ack(), 101);
    }

    #[test]
    fn test_c_path_view_conversion() {
        let c_view = CPathView {
            smoothed_rtt: 50000,
            rtt_min: 10000,
            cwin: 65536,
            peak_bandwidth_estimate: 1000000,
            send_mtu: 1280,
            pkt_ctx_send_sequence: 200,
            pkt_ctx_highest_acknowledged: 150,
            pkt_ctx_latest_time_acknowledged: 12345,
            pkt_ctx_pending_first_sequence: 100,
            pkt_ctx_has_pending: 1,
            cnx_is_multipath_enabled: 1,
            cnx_cwin_blocked: 0,
        };

        let path = c_view.to_rust();
        assert_eq!(path.smoothed_rtt, 50000);
        assert_eq!(path.rtt_min, 10000);
        assert_eq!(path.send_mtu, 1280);
        assert_eq!(path.pkt_ctx_pending_first_sequence, Some(100));
        assert!(path.cnx_is_multipath_enabled);
        assert!(!path.cnx_cwin_blocked);
    }
}
