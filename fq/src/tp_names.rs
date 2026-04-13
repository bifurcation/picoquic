//! Transport parameter names and constants.
//!
//! Translated from picoquic/tp_names.c.
//!
//! This module provides string names for QUIC transport parameters as defined
//! in RFC 9000 and various QUIC extensions.

// =============================================================================
// Transport Parameter Constants (from picoquic.h)
// =============================================================================

/// RFC 9000 transport parameters.
pub mod tp {
    /// Original destination connection ID (RFC 9000).
    pub const ORIGINAL_CONNECTION_ID: u64 = 0;
    /// Maximum idle timeout in milliseconds (RFC 9000).
    pub const IDLE_TIMEOUT: u64 = 1;
    /// Stateless reset token (RFC 9000).
    pub const STATELESS_RESET_TOKEN: u64 = 2;
    /// Maximum UDP payload size (RFC 9000).
    pub const MAX_PACKET_SIZE: u64 = 3;
    /// Initial maximum data (RFC 9000).
    pub const INITIAL_MAX_DATA: u64 = 4;
    /// Initial max stream data for local-initiated bidirectional streams (RFC 9000).
    pub const INITIAL_MAX_STREAM_DATA_BIDI_LOCAL: u64 = 5;
    /// Initial max stream data for remote-initiated bidirectional streams (RFC 9000).
    pub const INITIAL_MAX_STREAM_DATA_BIDI_REMOTE: u64 = 6;
    /// Initial max stream data for unidirectional streams (RFC 9000).
    pub const INITIAL_MAX_STREAM_DATA_UNI: u64 = 7;
    /// Initial max bidirectional streams (RFC 9000).
    pub const INITIAL_MAX_STREAMS_BIDI: u64 = 8;
    /// Initial max unidirectional streams (RFC 9000).
    pub const INITIAL_MAX_STREAMS_UNI: u64 = 9;
    /// ACK delay exponent (RFC 9000).
    pub const ACK_DELAY_EXPONENT: u64 = 10;
    /// Maximum ACK delay in milliseconds (RFC 9000).
    pub const MAX_ACK_DELAY: u64 = 11;
    /// Disable active connection migration (RFC 9000).
    pub const DISABLE_MIGRATION: u64 = 12;
    /// Server's preferred address (RFC 9000).
    pub const SERVER_PREFERRED_ADDRESS: u64 = 13;
    /// Active connection ID limit (RFC 9000).
    pub const ACTIVE_CONNECTION_ID_LIMIT: u64 = 14;
    /// Initial source connection ID during handshake (RFC 9000).
    pub const HANDSHAKE_CONNECTION_ID: u64 = 15;
    /// Retry source connection ID (RFC 9000).
    pub const RETRY_CONNECTION_ID: u64 = 16;
    /// Version negotiation (RFC 9368).
    pub const VERSION_NEGOTIATION: u64 = 0x11;

    /// Maximum datagram frame size (draft-ietf-quic-datagram).
    pub const MAX_DATAGRAM_FRAME_SIZE: u64 = 32;

    /// Test large client hello (picoquic internal).
    pub const TEST_LARGE_CHELLO: u64 = 3127;

    /// Enable loss bit (draft-ferrieuxhamchaoui-quic-lossbits).
    pub const ENABLE_LOSS_BIT: u64 = 0x1057;

    /// Minimum ACK delay (draft-ietf-quic-ack-frequency).
    pub const MIN_ACK_DELAY: u64 = 0xff04de1b;

    /// Enable timestamp (draft-huitema-quic-ts).
    pub const ENABLE_TIME_STAMP: u64 = 0x7158;

    /// Grease QUIC bit (RFC 9287).
    pub const GREASE_QUIC_BIT: u64 = 0x2ab2;

    /// Enable BDP frame (draft-kuhn-quic-0rtt-bdp).
    pub const ENABLE_BDP_FRAME: u64 = 0xebd9;

    /// Initial max path ID (draft-ietf-quic-multipath).
    pub const INITIAL_MAX_PATH_ID: u64 = 0x3e;

    /// Address discovery (draft-seemann-quic-address-discovery).
    pub const ADDRESS_DISCOVERY: u64 = 0x9f81a176;

    /// Reset stream at offset (draft-ietf-quic-reliable-stream-reset).
    pub const RESET_STREAM_AT: u64 = 0x17f7586d2cb571;
}

// =============================================================================
// Transport Parameter Name Lookup
// =============================================================================

/// Get the name of a transport parameter.
///
/// Returns "unknown" if the transport parameter number is not recognized.
///
/// # Arguments
/// * `tp_number` - The transport parameter number
///
/// # Returns
/// A static string with the parameter name
pub fn tp_name(tp_number: u64) -> &'static str {
    match tp_number {
        tp::ORIGINAL_CONNECTION_ID => "original_connection_id",
        tp::IDLE_TIMEOUT => "idle_timeout",
        tp::STATELESS_RESET_TOKEN => "stateless_reset_token",
        tp::MAX_PACKET_SIZE => "max_packet_size",
        tp::INITIAL_MAX_DATA => "initial_max_data",
        tp::INITIAL_MAX_STREAM_DATA_BIDI_LOCAL => "initial_max_stream_data_bidi_local",
        tp::INITIAL_MAX_STREAM_DATA_BIDI_REMOTE => "initial_max_stream_data_bidi_remote",
        tp::INITIAL_MAX_STREAM_DATA_UNI => "initial_max_stream_data_uni",
        tp::INITIAL_MAX_STREAMS_BIDI => "initial_max_streams_bidi",
        tp::INITIAL_MAX_STREAMS_UNI => "initial_max_streams_uni",
        tp::ACK_DELAY_EXPONENT => "ack_delay_exponent",
        tp::MAX_ACK_DELAY => "max_ack_delay",
        tp::DISABLE_MIGRATION => "disable_migration",
        tp::SERVER_PREFERRED_ADDRESS => "server_preferred_address",
        tp::ACTIVE_CONNECTION_ID_LIMIT => "active_connection_id_limit",
        tp::RETRY_CONNECTION_ID => "retry_connection_id",
        tp::HANDSHAKE_CONNECTION_ID => "handshake_connection_id",
        tp::MAX_DATAGRAM_FRAME_SIZE => "max_datagram_frame_size",
        tp::TEST_LARGE_CHELLO => "large_chello",
        tp::ENABLE_LOSS_BIT => "enable_loss_bit",
        tp::MIN_ACK_DELAY => "min_ack_delay",
        tp::ENABLE_TIME_STAMP => "enable_time_stamp",
        tp::GREASE_QUIC_BIT => "grease_quic_bit",
        tp::VERSION_NEGOTIATION => "version_negotiation",
        tp::ENABLE_BDP_FRAME => "enable_bdp_frame",
        tp::INITIAL_MAX_PATH_ID => "initial_max_path_id",
        tp::ADDRESS_DISCOVERY => "address_discovery",
        tp::RESET_STREAM_AT => "reset_stream_at",
        _ => "unknown",
    }
}

// =============================================================================
// FFI Export
// =============================================================================

/// Get transport parameter name (FFI export).
///
/// Returns a pointer to a null-terminated static string. The returned string
/// is valid for the lifetime of the program.
///
/// # Safety
/// The returned pointer is always valid and points to a null-terminated string.
#[no_mangle]
pub extern "C" fn picoquic_tp_name(tp_number: u64) -> *const std::ffi::c_char {
    // Use explicit null-terminated byte strings for FFI
    let name: &[u8] = match tp_number {
        tp::ORIGINAL_CONNECTION_ID => b"original_connection_id\0",
        tp::IDLE_TIMEOUT => b"idle_timeout\0",
        tp::STATELESS_RESET_TOKEN => b"stateless_reset_token\0",
        tp::MAX_PACKET_SIZE => b"max_packet_size\0",
        tp::INITIAL_MAX_DATA => b"initial_max_data\0",
        tp::INITIAL_MAX_STREAM_DATA_BIDI_LOCAL => b"initial_max_stream_data_bidi_local\0",
        tp::INITIAL_MAX_STREAM_DATA_BIDI_REMOTE => b"initial_max_stream_data_bidi_remote\0",
        tp::INITIAL_MAX_STREAM_DATA_UNI => b"initial_max_stream_data_uni\0",
        tp::INITIAL_MAX_STREAMS_BIDI => b"initial_max_streams_bidi\0",
        tp::INITIAL_MAX_STREAMS_UNI => b"initial_max_streams_uni\0",
        tp::ACK_DELAY_EXPONENT => b"ack_delay_exponent\0",
        tp::MAX_ACK_DELAY => b"max_ack_delay\0",
        tp::DISABLE_MIGRATION => b"disable_migration\0",
        tp::SERVER_PREFERRED_ADDRESS => b"server_preferred_address\0",
        tp::ACTIVE_CONNECTION_ID_LIMIT => b"active_connection_id_limit\0",
        tp::RETRY_CONNECTION_ID => b"retry_connection_id\0",
        tp::HANDSHAKE_CONNECTION_ID => b"handshake_connection_id\0",
        tp::MAX_DATAGRAM_FRAME_SIZE => b"max_datagram_frame_size\0",
        tp::TEST_LARGE_CHELLO => b"large_chello\0",
        tp::ENABLE_LOSS_BIT => b"enable_loss_bit\0",
        tp::MIN_ACK_DELAY => b"min_ack_delay\0",
        tp::ENABLE_TIME_STAMP => b"enable_time_stamp\0",
        tp::GREASE_QUIC_BIT => b"grease_quic_bit\0",
        tp::VERSION_NEGOTIATION => b"version_negotiation\0",
        tp::ENABLE_BDP_FRAME => b"enable_bdp_frame\0",
        tp::INITIAL_MAX_PATH_ID => b"initial_max_path_id\0",
        tp::ADDRESS_DISCOVERY => b"address_discovery\0",
        tp::RESET_STREAM_AT => b"reset_stream_at\0",
        _ => b"unknown\0",
    };
    name.as_ptr() as *const std::ffi::c_char
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tp_constants() {
        // Verify RFC 9000 parameters
        assert_eq!(tp::ORIGINAL_CONNECTION_ID, 0);
        assert_eq!(tp::IDLE_TIMEOUT, 1);
        assert_eq!(tp::STATELESS_RESET_TOKEN, 2);
        assert_eq!(tp::MAX_PACKET_SIZE, 3);
        assert_eq!(tp::INITIAL_MAX_DATA, 4);
        assert_eq!(tp::INITIAL_MAX_STREAM_DATA_BIDI_LOCAL, 5);
        assert_eq!(tp::INITIAL_MAX_STREAM_DATA_BIDI_REMOTE, 6);
        assert_eq!(tp::INITIAL_MAX_STREAM_DATA_UNI, 7);
        assert_eq!(tp::INITIAL_MAX_STREAMS_BIDI, 8);
        assert_eq!(tp::INITIAL_MAX_STREAMS_UNI, 9);
        assert_eq!(tp::ACK_DELAY_EXPONENT, 10);
        assert_eq!(tp::MAX_ACK_DELAY, 11);
        assert_eq!(tp::DISABLE_MIGRATION, 12);
        assert_eq!(tp::SERVER_PREFERRED_ADDRESS, 13);
        assert_eq!(tp::ACTIVE_CONNECTION_ID_LIMIT, 14);
        assert_eq!(tp::HANDSHAKE_CONNECTION_ID, 15);
        assert_eq!(tp::RETRY_CONNECTION_ID, 16);
        assert_eq!(tp::VERSION_NEGOTIATION, 0x11);
    }

    #[test]
    fn test_tp_extension_constants() {
        // Verify extension parameters
        assert_eq!(tp::MAX_DATAGRAM_FRAME_SIZE, 32);
        assert_eq!(tp::TEST_LARGE_CHELLO, 3127);
        assert_eq!(tp::ENABLE_LOSS_BIT, 0x1057);
        assert_eq!(tp::MIN_ACK_DELAY, 0xff04de1b);
        assert_eq!(tp::ENABLE_TIME_STAMP, 0x7158);
        assert_eq!(tp::GREASE_QUIC_BIT, 0x2ab2);
        assert_eq!(tp::ENABLE_BDP_FRAME, 0xebd9);
        assert_eq!(tp::INITIAL_MAX_PATH_ID, 0x3e);
        assert_eq!(tp::ADDRESS_DISCOVERY, 0x9f81a176);
        assert_eq!(tp::RESET_STREAM_AT, 0x17f7586d2cb571);
    }

    #[test]
    fn test_tp_name_rfc9000() {
        assert_eq!(
            tp_name(tp::ORIGINAL_CONNECTION_ID),
            "original_connection_id"
        );
        assert_eq!(tp_name(tp::IDLE_TIMEOUT), "idle_timeout");
        assert_eq!(tp_name(tp::STATELESS_RESET_TOKEN), "stateless_reset_token");
        assert_eq!(tp_name(tp::MAX_PACKET_SIZE), "max_packet_size");
        assert_eq!(tp_name(tp::INITIAL_MAX_DATA), "initial_max_data");
        assert_eq!(
            tp_name(tp::INITIAL_MAX_STREAM_DATA_BIDI_LOCAL),
            "initial_max_stream_data_bidi_local"
        );
        assert_eq!(
            tp_name(tp::INITIAL_MAX_STREAM_DATA_BIDI_REMOTE),
            "initial_max_stream_data_bidi_remote"
        );
        assert_eq!(
            tp_name(tp::INITIAL_MAX_STREAM_DATA_UNI),
            "initial_max_stream_data_uni"
        );
        assert_eq!(
            tp_name(tp::INITIAL_MAX_STREAMS_BIDI),
            "initial_max_streams_bidi"
        );
        assert_eq!(
            tp_name(tp::INITIAL_MAX_STREAMS_UNI),
            "initial_max_streams_uni"
        );
        assert_eq!(tp_name(tp::ACK_DELAY_EXPONENT), "ack_delay_exponent");
        assert_eq!(tp_name(tp::MAX_ACK_DELAY), "max_ack_delay");
        assert_eq!(tp_name(tp::DISABLE_MIGRATION), "disable_migration");
        assert_eq!(
            tp_name(tp::SERVER_PREFERRED_ADDRESS),
            "server_preferred_address"
        );
        assert_eq!(
            tp_name(tp::ACTIVE_CONNECTION_ID_LIMIT),
            "active_connection_id_limit"
        );
        assert_eq!(tp_name(tp::RETRY_CONNECTION_ID), "retry_connection_id");
        assert_eq!(
            tp_name(tp::HANDSHAKE_CONNECTION_ID),
            "handshake_connection_id"
        );
    }

    #[test]
    fn test_tp_name_extensions() {
        assert_eq!(
            tp_name(tp::MAX_DATAGRAM_FRAME_SIZE),
            "max_datagram_frame_size"
        );
        assert_eq!(tp_name(tp::TEST_LARGE_CHELLO), "large_chello");
        assert_eq!(tp_name(tp::ENABLE_LOSS_BIT), "enable_loss_bit");
        assert_eq!(tp_name(tp::MIN_ACK_DELAY), "min_ack_delay");
        assert_eq!(tp_name(tp::ENABLE_TIME_STAMP), "enable_time_stamp");
        assert_eq!(tp_name(tp::GREASE_QUIC_BIT), "grease_quic_bit");
        assert_eq!(tp_name(tp::VERSION_NEGOTIATION), "version_negotiation");
        assert_eq!(tp_name(tp::ENABLE_BDP_FRAME), "enable_bdp_frame");
        assert_eq!(tp_name(tp::INITIAL_MAX_PATH_ID), "initial_max_path_id");
        assert_eq!(tp_name(tp::ADDRESS_DISCOVERY), "address_discovery");
        assert_eq!(tp_name(tp::RESET_STREAM_AT), "reset_stream_at");
    }

    #[test]
    fn test_tp_name_unknown() {
        assert_eq!(tp_name(0xFFFFFFFF), "unknown");
        assert_eq!(tp_name(9999), "unknown");
    }

    #[test]
    fn test_tp_name_ffi() {
        use std::ffi::CStr;

        unsafe {
            let name_ptr = picoquic_tp_name(tp::IDLE_TIMEOUT);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "idle_timeout");
        }
    }
}
