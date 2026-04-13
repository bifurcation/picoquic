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

    // Flow control fields
    /// Maximum data the peer is willing to receive (from MAX_DATA frames).
    pub maxdata_remote: u64,
    /// Whether we've sent a BLOCKED frame and are waiting for MAX_DATA.
    pub sent_blocked_frame: bool,

    // Multipath fields
    /// Maximum path ID the peer is willing to use (from MAX_PATH_ID frames).
    pub max_path_id_remote: u64,

    // ACK frequency fields (from ACK_FREQUENCY frames)
    /// Whether ACK frequency extension was negotiated.
    pub is_ack_frequency_negotiated: bool,
    /// Minimum ACK delay from local parameters.
    pub local_min_ack_delay: u64,
    /// Current sequence number for remote ACK frequency updates.
    pub ack_frequency_sequence_remote: u64,
    /// Current ACK gap (packet threshold) from remote.
    pub ack_gap_remote: u64,
    /// Current ACK delay (time threshold) from remote.
    pub ack_delay_remote: u64,
    /// Whether to ignore packet reordering for ACK generation.
    pub ack_ignore_order_remote: bool,
    /// Reordering threshold from remote.
    pub ack_reordering_threshold_remote: u64,
    /// Maximum ACK gap seen (statistics).
    pub max_ack_gap_remote: u64,
    /// Maximum ACK delay seen (statistics).
    pub max_ack_delay_remote: u64,
    /// Minimum ACK delay seen (statistics).
    pub min_ack_delay_remote: u64,

    // Time stamp fields
    /// Whether the time stamp extension is enabled.
    pub is_time_stamp_enabled: bool,
    /// Remote peer's ACK delay exponent (used to decode time stamps).
    pub remote_ack_delay_exponent: u8,
}

/// ACK frequency validation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum AckFrequencyResult {
    /// Successfully applied the ACK frequency update.
    Success = 0,
    /// Extension not negotiated.
    NotNegotiated = 1,
    /// ACK delay is below minimum.
    DelayBelowMin = 2,
    /// Packets value is zero (invalid).
    ZeroPackets = 3,
    /// Invalid ignore_order value.
    InvalidIgnoreOrder = 4,
    /// Sequence number is not newer than current.
    OldSequence = 5,
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
            maxdata_remote: 0,
            sent_blocked_frame: false,
            max_path_id_remote: 0,
            is_ack_frequency_negotiated: false,
            local_min_ack_delay: 0,
            ack_frequency_sequence_remote: 0,
            ack_gap_remote: 0,
            ack_delay_remote: 0,
            ack_ignore_order_remote: false,
            ack_reordering_threshold_remote: 0,
            max_ack_gap_remote: 0,
            max_ack_delay_remote: 0,
            min_ack_delay_remote: u64::MAX,
            is_time_stamp_enabled: false,
            remote_ack_delay_exponent: 0,
        }
    }

    /// Apply a received MAX_DATA frame value.
    ///
    /// Updates maxdata_remote if the new value is larger, and clears
    /// the sent_blocked_frame flag since we can now send more data.
    ///
    /// Returns true if the value was updated.
    pub fn apply_max_data(&mut self, maxdata: u64) -> bool {
        if maxdata > self.maxdata_remote {
            self.maxdata_remote = maxdata;
            self.sent_blocked_frame = false;
            true
        } else {
            false
        }
    }

    /// Apply a received MAX_PATH_ID frame value.
    ///
    /// Updates max_path_id_remote if the new value is larger.
    ///
    /// Returns true if the value was updated.
    pub fn apply_max_path_id(&mut self, max_path_id: u64) -> bool {
        if max_path_id > self.max_path_id_remote {
            self.max_path_id_remote = max_path_id;
            true
        } else {
            false
        }
    }

    /// Apply a received ACK_FREQUENCY frame.
    ///
    /// Validates and applies the ACK frequency parameters if valid.
    /// Returns the result indicating success or the type of error.
    pub fn apply_ack_frequency(
        &mut self,
        seq: u64,
        packets: u64,
        microsec: u64,
        ignore_order: u8,
        reordering_threshold: u64,
    ) -> AckFrequencyResult {
        use AckFrequencyResult::*;

        // Validation checks
        if !self.is_ack_frequency_negotiated {
            return NotNegotiated;
        }
        if microsec < self.local_min_ack_delay {
            return DelayBelowMin;
        }
        if packets == 0 {
            return ZeroPackets;
        }
        if ignore_order > 1 {
            return InvalidIgnoreOrder;
        }

        // Check sequence number is newer
        let delta = seq as i64 - self.ack_frequency_sequence_remote as i64;
        if delta <= 0 {
            return OldSequence;
        }

        // Apply the update
        self.ack_frequency_sequence_remote = seq;
        self.ack_gap_remote = packets;
        self.ack_delay_remote = microsec;
        self.ack_ignore_order_remote = ignore_order != 0;
        self.ack_reordering_threshold_remote = reordering_threshold;

        // Update statistics
        if packets > self.max_ack_gap_remote {
            self.max_ack_gap_remote = packets;
        }
        if microsec > self.max_ack_delay_remote {
            self.max_ack_delay_remote = microsec;
        } else if microsec < self.min_ack_delay_remote {
            self.min_ack_delay_remote = microsec;
        }

        Success
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
    pub maxdata_remote: u64,
    pub sent_blocked_frame: c_int,
    pub max_path_id_remote: u64,
    // ACK frequency fields
    pub is_ack_frequency_negotiated: c_int,
    pub local_min_ack_delay: u64,
    pub ack_frequency_sequence_remote: u64,
    pub ack_gap_remote: u64,
    pub ack_delay_remote: u64,
    pub ack_ignore_order_remote: c_int,
    pub ack_reordering_threshold_remote: u64,
    pub max_ack_gap_remote: u64,
    pub max_ack_delay_remote: u64,
    pub min_ack_delay_remote: u64,
    // Time stamp fields
    pub is_time_stamp_enabled: c_int,
    pub remote_ack_delay_exponent: u8,
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
            maxdata_remote: self.maxdata_remote,
            sent_blocked_frame: self.sent_blocked_frame != 0,
            max_path_id_remote: self.max_path_id_remote,
            is_ack_frequency_negotiated: self.is_ack_frequency_negotiated != 0,
            local_min_ack_delay: self.local_min_ack_delay,
            ack_frequency_sequence_remote: self.ack_frequency_sequence_remote,
            ack_gap_remote: self.ack_gap_remote,
            ack_delay_remote: self.ack_delay_remote,
            ack_ignore_order_remote: self.ack_ignore_order_remote != 0,
            ack_reordering_threshold_remote: self.ack_reordering_threshold_remote,
            max_ack_gap_remote: self.max_ack_gap_remote,
            max_ack_delay_remote: self.max_ack_delay_remote,
            min_ack_delay_remote: self.min_ack_delay_remote,
            is_time_stamp_enabled: self.is_time_stamp_enabled != 0,
            remote_ack_delay_exponent: self.remote_ack_delay_exponent,
        }
    }

    /// Update C struct from Rust Connection.
    pub fn from_rust(&mut self, cnx: &Connection) {
        self.is_multipath_enabled = if cnx.is_multipath_enabled { 1 } else { 0 };
        self.cwin_blocked = if cnx.cwin_blocked { 1 } else { 0 };
        self.pkt_ctx_app_send_sequence = cnx.pkt_ctx_app_send_sequence;
        self.pkt_ctx_app_highest_acknowledged = cnx.pkt_ctx_app_highest_acknowledged;
        self.pkt_ctx_app_latest_time_acknowledged = cnx.pkt_ctx_app_latest_time_acknowledged;
        self.path0_rtt_min = cnx.path0_rtt_min;
        self.maxdata_remote = cnx.maxdata_remote;
        self.sent_blocked_frame = if cnx.sent_blocked_frame { 1 } else { 0 };
        self.max_path_id_remote = cnx.max_path_id_remote;
        self.is_ack_frequency_negotiated = if cnx.is_ack_frequency_negotiated {
            1
        } else {
            0
        };
        self.local_min_ack_delay = cnx.local_min_ack_delay;
        self.ack_frequency_sequence_remote = cnx.ack_frequency_sequence_remote;
        self.ack_gap_remote = cnx.ack_gap_remote;
        self.ack_delay_remote = cnx.ack_delay_remote;
        self.ack_ignore_order_remote = if cnx.ack_ignore_order_remote { 1 } else { 0 };
        self.ack_reordering_threshold_remote = cnx.ack_reordering_threshold_remote;
        self.max_ack_gap_remote = cnx.max_ack_gap_remote;
        self.max_ack_delay_remote = cnx.max_ack_delay_remote;
        self.min_ack_delay_remote = cnx.min_ack_delay_remote;
        self.is_time_stamp_enabled = if cnx.is_time_stamp_enabled { 1 } else { 0 };
        self.remote_ack_delay_exponent = cnx.remote_ack_delay_exponent;
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
            maxdata_remote: 65536,
            sent_blocked_frame: 1,
            max_path_id_remote: 10,
            is_ack_frequency_negotiated: 1,
            local_min_ack_delay: 1000,
            ack_frequency_sequence_remote: 5,
            ack_gap_remote: 2,
            ack_delay_remote: 25000,
            ack_ignore_order_remote: 0,
            ack_reordering_threshold_remote: 1,
            max_ack_gap_remote: 2,
            max_ack_delay_remote: 25000,
            min_ack_delay_remote: 25000,
            is_time_stamp_enabled: 1,
            remote_ack_delay_exponent: 3,
        };

        let cnx = c_view.to_rust();
        assert!(cnx.is_multipath_enabled);
        assert!(cnx.cwin_blocked);
        assert_eq!(cnx.pkt_ctx_app_send_sequence, 200);
        assert_eq!(cnx.path0_rtt_min, 5000);
        assert_eq!(cnx.maxdata_remote, 65536);
        assert!(cnx.sent_blocked_frame);
        assert_eq!(cnx.max_path_id_remote, 10);
        assert!(cnx.is_ack_frequency_negotiated);
        assert_eq!(cnx.ack_frequency_sequence_remote, 5);
        assert!(cnx.is_time_stamp_enabled);
        assert_eq!(cnx.remote_ack_delay_exponent, 3);
    }

    #[test]
    fn test_apply_max_data_increases() {
        let mut cnx = Connection::default();
        cnx.maxdata_remote = 1000;
        cnx.sent_blocked_frame = true;

        // Larger value should update and clear blocked flag
        assert!(cnx.apply_max_data(2000));
        assert_eq!(cnx.maxdata_remote, 2000);
        assert!(!cnx.sent_blocked_frame);
    }

    #[test]
    fn test_apply_max_data_no_decrease() {
        let mut cnx = Connection::default();
        cnx.maxdata_remote = 2000;
        cnx.sent_blocked_frame = true;

        // Smaller or equal value should not update
        assert!(!cnx.apply_max_data(1000));
        assert_eq!(cnx.maxdata_remote, 2000);
        assert!(cnx.sent_blocked_frame); // Flag unchanged
    }

    #[test]
    fn test_c_connection_view_roundtrip() {
        let mut c_view = CConnectionView {
            is_multipath_enabled: 0,
            cwin_blocked: 0,
            pkt_ctx_app_send_sequence: 0,
            pkt_ctx_app_highest_acknowledged: 0,
            pkt_ctx_app_latest_time_acknowledged: 0,
            path0_rtt_min: 0,
            maxdata_remote: 1000,
            sent_blocked_frame: 1,
            max_path_id_remote: 5,
            is_ack_frequency_negotiated: 1,
            local_min_ack_delay: 1000,
            ack_frequency_sequence_remote: 0,
            ack_gap_remote: 2,
            ack_delay_remote: 25000,
            ack_ignore_order_remote: 0,
            ack_reordering_threshold_remote: 1,
            max_ack_gap_remote: 2,
            max_ack_delay_remote: 25000,
            min_ack_delay_remote: u64::MAX,
            is_time_stamp_enabled: 0,
            remote_ack_delay_exponent: 0,
        };

        let mut cnx = c_view.to_rust();
        cnx.apply_max_data(5000);
        cnx.apply_max_path_id(10);
        // Apply ACK frequency with seq=1 (newer than 0)
        let result = cnx.apply_ack_frequency(1, 4, 50000, 0, 2);
        assert_eq!(result, AckFrequencyResult::Success);
        c_view.from_rust(&cnx);

        assert_eq!(c_view.maxdata_remote, 5000);
        assert_eq!(c_view.sent_blocked_frame, 0);
        assert_eq!(c_view.max_path_id_remote, 10);
        assert_eq!(c_view.ack_frequency_sequence_remote, 1);
        assert_eq!(c_view.ack_gap_remote, 4);
        assert_eq!(c_view.ack_delay_remote, 50000);
    }

    #[test]
    fn test_apply_max_path_id_increases() {
        let mut cnx = Connection::default();
        cnx.max_path_id_remote = 5;

        // Larger value should update
        assert!(cnx.apply_max_path_id(10));
        assert_eq!(cnx.max_path_id_remote, 10);
    }

    #[test]
    fn test_apply_max_path_id_no_decrease() {
        let mut cnx = Connection::default();
        cnx.max_path_id_remote = 10;

        // Smaller or equal value should not update
        assert!(!cnx.apply_max_path_id(5));
        assert_eq!(cnx.max_path_id_remote, 10);
    }

    #[test]
    fn test_apply_ack_frequency_success() {
        let mut cnx = Connection::default();
        cnx.is_ack_frequency_negotiated = true;
        cnx.local_min_ack_delay = 1000;

        let result = cnx.apply_ack_frequency(1, 2, 25000, 0, 1);
        assert_eq!(result, AckFrequencyResult::Success);
        assert_eq!(cnx.ack_frequency_sequence_remote, 1);
        assert_eq!(cnx.ack_gap_remote, 2);
        assert_eq!(cnx.ack_delay_remote, 25000);
        assert!(!cnx.ack_ignore_order_remote);
        assert_eq!(cnx.ack_reordering_threshold_remote, 1);
    }

    #[test]
    fn test_apply_ack_frequency_not_negotiated() {
        let mut cnx = Connection::default();
        cnx.is_ack_frequency_negotiated = false;

        let result = cnx.apply_ack_frequency(1, 2, 25000, 0, 1);
        assert_eq!(result, AckFrequencyResult::NotNegotiated);
    }

    #[test]
    fn test_apply_ack_frequency_delay_below_min() {
        let mut cnx = Connection::default();
        cnx.is_ack_frequency_negotiated = true;
        cnx.local_min_ack_delay = 50000;

        let result = cnx.apply_ack_frequency(1, 2, 25000, 0, 1);
        assert_eq!(result, AckFrequencyResult::DelayBelowMin);
    }

    #[test]
    fn test_apply_ack_frequency_zero_packets() {
        let mut cnx = Connection::default();
        cnx.is_ack_frequency_negotiated = true;
        cnx.local_min_ack_delay = 1000;

        let result = cnx.apply_ack_frequency(1, 0, 25000, 0, 1);
        assert_eq!(result, AckFrequencyResult::ZeroPackets);
    }

    #[test]
    fn test_apply_ack_frequency_old_sequence() {
        let mut cnx = Connection::default();
        cnx.is_ack_frequency_negotiated = true;
        cnx.local_min_ack_delay = 1000;
        cnx.ack_frequency_sequence_remote = 5;

        // Same sequence should be rejected
        let result = cnx.apply_ack_frequency(5, 2, 25000, 0, 1);
        assert_eq!(result, AckFrequencyResult::OldSequence);

        // Older sequence should be rejected
        let result = cnx.apply_ack_frequency(3, 2, 25000, 0, 1);
        assert_eq!(result, AckFrequencyResult::OldSequence);
    }

    #[test]
    fn test_apply_ack_frequency_updates_stats() {
        let mut cnx = Connection::default();
        cnx.is_ack_frequency_negotiated = true;
        cnx.local_min_ack_delay = 1000;
        cnx.min_ack_delay_remote = u64::MAX;

        // First update - 25000 > 0 (max), so max is updated but not min
        let result = cnx.apply_ack_frequency(1, 2, 25000, 0, 1);
        assert_eq!(result, AckFrequencyResult::Success);
        assert_eq!(cnx.max_ack_gap_remote, 2);
        assert_eq!(cnx.max_ack_delay_remote, 25000);
        // min not updated because we took the "if" branch
        assert_eq!(cnx.min_ack_delay_remote, u64::MAX);

        // Update with smaller delay - 20000 < 25000 (max), so check min
        // 20000 < u64::MAX, so min is updated
        let result = cnx.apply_ack_frequency(2, 5, 20000, 0, 1);
        assert_eq!(result, AckFrequencyResult::Success);
        assert_eq!(cnx.max_ack_gap_remote, 5);
        assert_eq!(cnx.max_ack_delay_remote, 25000); // Not updated (20000 < 25000)
        assert_eq!(cnx.min_ack_delay_remote, 20000); // Updated (20000 < u64::MAX)

        // Update with even smaller delay
        let result = cnx.apply_ack_frequency(3, 3, 15000, 0, 1);
        assert_eq!(result, AckFrequencyResult::Success);
        assert_eq!(cnx.min_ack_delay_remote, 15000);
    }
}
