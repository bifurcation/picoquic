//! Frame type names and constants.
//!
//! Translated from picoquic/frame_names.c.
//!
//! This module provides string names for QUIC frame types as defined
//! in RFC 9000 and various QUIC extensions.

// =============================================================================
// Frame Type Constants (from picoquic_internal.h)
// =============================================================================

/// QUIC frame type constants.
pub mod frame_type {
    /// Padding frame (RFC 9000).
    pub const PADDING: u64 = 0x00;
    /// Ping frame (RFC 9000).
    pub const PING: u64 = 0x01;
    /// ACK frame (RFC 9000).
    pub const ACK: u64 = 0x02;
    /// ACK frame with ECN counts (RFC 9000).
    pub const ACK_ECN: u64 = 0x03;
    /// Reset stream frame (RFC 9000).
    pub const RESET_STREAM: u64 = 0x04;
    /// Stop sending frame (RFC 9000).
    pub const STOP_SENDING: u64 = 0x05;
    /// Crypto handshake frame (RFC 9000).
    pub const CRYPTO_HS: u64 = 0x06;
    /// New token frame (RFC 9000).
    pub const NEW_TOKEN: u64 = 0x07;
    /// Stream frame range minimum (RFC 9000).
    pub const STREAM_RANGE_MIN: u64 = 0x08;
    /// Stream frame range maximum (RFC 9000).
    pub const STREAM_RANGE_MAX: u64 = 0x0f;
    /// Max data frame (RFC 9000).
    pub const MAX_DATA: u64 = 0x10;
    /// Max stream data frame (RFC 9000).
    pub const MAX_STREAM_DATA: u64 = 0x11;
    /// Max streams bidirectional frame (RFC 9000).
    pub const MAX_STREAMS_BIDIR: u64 = 0x12;
    /// Max streams unidirectional frame (RFC 9000).
    pub const MAX_STREAMS_UNIDIR: u64 = 0x13;
    /// Data blocked frame (RFC 9000).
    pub const DATA_BLOCKED: u64 = 0x14;
    /// Stream data blocked frame (RFC 9000).
    pub const STREAM_DATA_BLOCKED: u64 = 0x15;
    /// Streams blocked bidirectional frame (RFC 9000).
    pub const STREAMS_BLOCKED_BIDIR: u64 = 0x16;
    /// Streams blocked unidirectional frame (RFC 9000).
    pub const STREAMS_BLOCKED_UNIDIR: u64 = 0x17;
    /// New connection ID frame (RFC 9000).
    pub const NEW_CONNECTION_ID: u64 = 0x18;
    /// Retire connection ID frame (RFC 9000).
    pub const RETIRE_CONNECTION_ID: u64 = 0x19;
    /// Path challenge frame (RFC 9000).
    pub const PATH_CHALLENGE: u64 = 0x1a;
    /// Path response frame (RFC 9000).
    pub const PATH_RESPONSE: u64 = 0x1b;
    /// Connection close frame (RFC 9000).
    pub const CONNECTION_CLOSE: u64 = 0x1c;
    /// Application close frame (RFC 9000).
    pub const APPLICATION_CLOSE: u64 = 0x1d;
    /// Handshake done frame (RFC 9000).
    pub const HANDSHAKE_DONE: u64 = 0x1e;
    /// Immediate ACK frame (draft-ietf-quic-ack-frequency).
    pub const IMMEDIATE_ACK: u64 = 0x1f;
    /// Reset stream at offset (draft-ietf-quic-reliable-stream-reset).
    pub const RESET_STREAM_AT: u64 = 0x24;
    /// Datagram frame without length (draft-ietf-quic-datagram).
    pub const DATAGRAM: u64 = 0x30;
    /// Datagram frame with length (draft-ietf-quic-datagram).
    pub const DATAGRAM_L: u64 = 0x31;
    /// Path ACK frame (draft-ietf-quic-multipath).
    pub const PATH_ACK: u64 = 0x3e;
    /// Path ACK frame with ECN (draft-ietf-quic-multipath).
    pub const PATH_ACK_ECN: u64 = 0x3f;
    /// ACK frequency frame (draft-ietf-quic-ack-frequency).
    pub const ACK_FREQUENCY: u64 = 0xaf;
    /// Time stamp frame (draft-huitema-quic-ts).
    pub const TIME_STAMP: u64 = 757;
    /// Path abandon frame (draft-ietf-quic-multipath).
    pub const PATH_ABANDON: u64 = 0x3e75;
    /// Path backup frame (draft-ietf-quic-multipath).
    pub const PATH_BACKUP: u64 = 0x3e76;
    /// Path available frame (draft-ietf-quic-multipath).
    pub const PATH_AVAILABLE: u64 = 0x3e77;
    /// Path new connection ID frame (draft-ietf-quic-multipath).
    pub const PATH_NEW_CONNECTION_ID: u64 = 0x3e78;
    /// Path retire connection ID frame (draft-ietf-quic-multipath).
    pub const PATH_RETIRE_CONNECTION_ID: u64 = 0x3e79;
    /// Max path ID frame (draft-ietf-quic-multipath).
    pub const MAX_PATH_ID: u64 = 0x3e7a;
    /// Paths blocked frame (draft-ietf-quic-multipath).
    pub const PATHS_BLOCKED: u64 = 0x3e7b;
    /// Path CID blocked frame (draft-ietf-quic-multipath).
    pub const PATH_CID_BLOCKED: u64 = 0x3e7c;
    /// BDP frame (draft-kuhn-quic-0rtt-bdp).
    pub const BDP: u64 = 0xebd9;
    /// Observed address IPv4 (draft-seemann-quic-address-discovery).
    pub const OBSERVED_ADDRESS_V4: u64 = 0x9f81a6;
    /// Observed address IPv6 (draft-seemann-quic-address-discovery).
    pub const OBSERVED_ADDRESS_V6: u64 = 0x9f81a7;
}

// =============================================================================
// Frame Name Lookup
// =============================================================================

/// Check if a frame type is a stream frame.
#[inline]
pub fn is_stream_frame(ftype: u64) -> bool {
    (frame_type::STREAM_RANGE_MIN..=frame_type::STREAM_RANGE_MAX).contains(&ftype)
}

/// Get the name of a frame type.
///
/// Returns "unknown" if the frame type is not recognized.
///
/// # Arguments
/// * `ftype` - The frame type number
///
/// # Returns
/// A static string with the frame type name
pub fn frame_name(ftype: u64) -> &'static str {
    // Check for stream frames first (range 0x08-0x0f)
    if is_stream_frame(ftype) {
        return "stream";
    }

    match ftype {
        frame_type::PADDING => "padding",
        frame_type::RESET_STREAM => "reset_stream",
        frame_type::RESET_STREAM_AT => "reset_stream_at",
        frame_type::CONNECTION_CLOSE | frame_type::APPLICATION_CLOSE => "connection_close",
        frame_type::MAX_DATA => "max_data",
        frame_type::MAX_STREAM_DATA => "max_stream_data",
        frame_type::MAX_STREAMS_BIDIR | frame_type::MAX_STREAMS_UNIDIR => "max_streams",
        frame_type::PING => "ping",
        frame_type::DATA_BLOCKED => "data_blocked",
        frame_type::STREAM_DATA_BLOCKED => "stream_data_blocked",
        frame_type::STREAMS_BLOCKED_BIDIR | frame_type::STREAMS_BLOCKED_UNIDIR => "streams_blocked",
        frame_type::NEW_CONNECTION_ID => "new_connection_id",
        frame_type::PATH_NEW_CONNECTION_ID => "path_new_connection_id",
        frame_type::STOP_SENDING => "stop_sending",
        frame_type::ACK => "ack",
        frame_type::PATH_CHALLENGE => "path_challenge",
        frame_type::PATH_RESPONSE => "path_response",
        frame_type::CRYPTO_HS => "crypto",
        frame_type::NEW_TOKEN => "new_token",
        frame_type::ACK_ECN => "ack",
        frame_type::PATH_ACK | frame_type::PATH_ACK_ECN => "path_ack",
        frame_type::RETIRE_CONNECTION_ID => "retire_connection_id",
        frame_type::PATH_RETIRE_CONNECTION_ID => "path_retire_connection_id",
        frame_type::HANDSHAKE_DONE => "handshake_done",
        frame_type::DATAGRAM | frame_type::DATAGRAM_L => "datagram",
        frame_type::ACK_FREQUENCY => "ack_frequency",
        frame_type::IMMEDIATE_ACK => "immediate_ack",
        frame_type::TIME_STAMP => "time_stamp",
        frame_type::PATH_ABANDON => "path_abandon",
        frame_type::PATH_BACKUP => "path_backup",
        frame_type::PATH_AVAILABLE => "path_available",
        frame_type::BDP => "bdp",
        frame_type::MAX_PATH_ID => "max_path_id",
        frame_type::PATHS_BLOCKED => "paths_blocked",
        frame_type::PATH_CID_BLOCKED => "path_cid_blocked",
        frame_type::OBSERVED_ADDRESS_V4 => "observed_address_v4",
        frame_type::OBSERVED_ADDRESS_V6 => "observed_address_v6",
        _ => "unknown",
    }
}

// =============================================================================
// FFI Export
// =============================================================================

/// Get frame type name (FFI export).
///
/// Returns a pointer to a null-terminated static string.
///
/// # Safety
/// The returned pointer is always valid and points to a null-terminated string.
#[no_mangle]
pub extern "C" fn picoquic_frame_name(ftype: u64) -> *const std::ffi::c_char {
    // Check for stream frames first
    if is_stream_frame(ftype) {
        return c"stream".as_ptr();
    }

    let name: &std::ffi::CStr = match ftype {
        frame_type::PADDING => c"padding",
        frame_type::RESET_STREAM => c"reset_stream",
        frame_type::RESET_STREAM_AT => c"reset_stream_at",
        frame_type::CONNECTION_CLOSE | frame_type::APPLICATION_CLOSE => c"connection_close",
        frame_type::MAX_DATA => c"max_data",
        frame_type::MAX_STREAM_DATA => c"max_stream_data",
        frame_type::MAX_STREAMS_BIDIR | frame_type::MAX_STREAMS_UNIDIR => c"max_streams",
        frame_type::PING => c"ping",
        frame_type::DATA_BLOCKED => c"data_blocked",
        frame_type::STREAM_DATA_BLOCKED => c"stream_data_blocked",
        frame_type::STREAMS_BLOCKED_BIDIR | frame_type::STREAMS_BLOCKED_UNIDIR => {
            c"streams_blocked"
        }
        frame_type::NEW_CONNECTION_ID => c"new_connection_id",
        frame_type::PATH_NEW_CONNECTION_ID => c"path_new_connection_id",
        frame_type::STOP_SENDING => c"stop_sending",
        frame_type::ACK => c"ack",
        frame_type::PATH_CHALLENGE => c"path_challenge",
        frame_type::PATH_RESPONSE => c"path_response",
        frame_type::CRYPTO_HS => c"crypto",
        frame_type::NEW_TOKEN => c"new_token",
        frame_type::ACK_ECN => c"ack",
        frame_type::PATH_ACK | frame_type::PATH_ACK_ECN => c"path_ack",
        frame_type::RETIRE_CONNECTION_ID => c"retire_connection_id",
        frame_type::PATH_RETIRE_CONNECTION_ID => c"path_retire_connection_id",
        frame_type::HANDSHAKE_DONE => c"handshake_done",
        frame_type::DATAGRAM | frame_type::DATAGRAM_L => c"datagram",
        frame_type::ACK_FREQUENCY => c"ack_frequency",
        frame_type::IMMEDIATE_ACK => c"immediate_ack",
        frame_type::TIME_STAMP => c"time_stamp",
        frame_type::PATH_ABANDON => c"path_abandon",
        frame_type::PATH_BACKUP => c"path_backup",
        frame_type::PATH_AVAILABLE => c"path_available",
        frame_type::BDP => c"bdp",
        frame_type::MAX_PATH_ID => c"max_path_id",
        frame_type::PATHS_BLOCKED => c"paths_blocked",
        frame_type::PATH_CID_BLOCKED => c"path_cid_blocked",
        frame_type::OBSERVED_ADDRESS_V4 => c"observed_address_v4",
        frame_type::OBSERVED_ADDRESS_V6 => c"observed_address_v6",
        _ => c"unknown",
    };
    name.as_ptr()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_constants() {
        assert_eq!(frame_type::PADDING, 0x00);
        assert_eq!(frame_type::PING, 0x01);
        assert_eq!(frame_type::ACK, 0x02);
        assert_eq!(frame_type::ACK_ECN, 0x03);
        assert_eq!(frame_type::RESET_STREAM, 0x04);
        assert_eq!(frame_type::STOP_SENDING, 0x05);
        assert_eq!(frame_type::CRYPTO_HS, 0x06);
        assert_eq!(frame_type::NEW_TOKEN, 0x07);
        assert_eq!(frame_type::STREAM_RANGE_MIN, 0x08);
        assert_eq!(frame_type::STREAM_RANGE_MAX, 0x0f);
        assert_eq!(frame_type::MAX_DATA, 0x10);
        assert_eq!(frame_type::CONNECTION_CLOSE, 0x1c);
        assert_eq!(frame_type::HANDSHAKE_DONE, 0x1e);
    }

    #[test]
    fn test_is_stream_frame() {
        assert!(!is_stream_frame(0x07));
        assert!(is_stream_frame(0x08));
        assert!(is_stream_frame(0x0a));
        assert!(is_stream_frame(0x0f));
        assert!(!is_stream_frame(0x10));
    }

    #[test]
    fn test_frame_name_rfc9000() {
        assert_eq!(frame_name(frame_type::PADDING), "padding");
        assert_eq!(frame_name(frame_type::PING), "ping");
        assert_eq!(frame_name(frame_type::ACK), "ack");
        assert_eq!(frame_name(frame_type::ACK_ECN), "ack");
        assert_eq!(frame_name(frame_type::RESET_STREAM), "reset_stream");
        assert_eq!(frame_name(frame_type::STOP_SENDING), "stop_sending");
        assert_eq!(frame_name(frame_type::CRYPTO_HS), "crypto");
        assert_eq!(frame_name(frame_type::NEW_TOKEN), "new_token");
        assert_eq!(frame_name(frame_type::MAX_DATA), "max_data");
        assert_eq!(frame_name(frame_type::MAX_STREAM_DATA), "max_stream_data");
        assert_eq!(frame_name(frame_type::MAX_STREAMS_BIDIR), "max_streams");
        assert_eq!(frame_name(frame_type::MAX_STREAMS_UNIDIR), "max_streams");
        assert_eq!(frame_name(frame_type::DATA_BLOCKED), "data_blocked");
        assert_eq!(
            frame_name(frame_type::STREAM_DATA_BLOCKED),
            "stream_data_blocked"
        );
        assert_eq!(
            frame_name(frame_type::STREAMS_BLOCKED_BIDIR),
            "streams_blocked"
        );
        assert_eq!(
            frame_name(frame_type::NEW_CONNECTION_ID),
            "new_connection_id"
        );
        assert_eq!(
            frame_name(frame_type::RETIRE_CONNECTION_ID),
            "retire_connection_id"
        );
        assert_eq!(frame_name(frame_type::PATH_CHALLENGE), "path_challenge");
        assert_eq!(frame_name(frame_type::PATH_RESPONSE), "path_response");
        assert_eq!(frame_name(frame_type::CONNECTION_CLOSE), "connection_close");
        assert_eq!(
            frame_name(frame_type::APPLICATION_CLOSE),
            "connection_close"
        );
        assert_eq!(frame_name(frame_type::HANDSHAKE_DONE), "handshake_done");
    }

    #[test]
    fn test_frame_name_stream() {
        // All stream frame variants should return "stream"
        for ftype in 0x08..=0x0f {
            assert_eq!(frame_name(ftype), "stream", "frame type {:#x}", ftype);
        }
    }

    #[test]
    fn test_frame_name_extensions() {
        assert_eq!(frame_name(frame_type::DATAGRAM), "datagram");
        assert_eq!(frame_name(frame_type::DATAGRAM_L), "datagram");
        assert_eq!(frame_name(frame_type::ACK_FREQUENCY), "ack_frequency");
        assert_eq!(frame_name(frame_type::IMMEDIATE_ACK), "immediate_ack");
        assert_eq!(frame_name(frame_type::TIME_STAMP), "time_stamp");
        assert_eq!(frame_name(frame_type::PATH_ACK), "path_ack");
        assert_eq!(frame_name(frame_type::PATH_ABANDON), "path_abandon");
        assert_eq!(frame_name(frame_type::BDP), "bdp");
        assert_eq!(frame_name(frame_type::MAX_PATH_ID), "max_path_id");
        assert_eq!(frame_name(frame_type::RESET_STREAM_AT), "reset_stream_at");
    }

    #[test]
    fn test_frame_name_unknown() {
        assert_eq!(frame_name(0xFFFFFFFF), "unknown");
        assert_eq!(frame_name(9999), "unknown");
    }

    #[test]
    fn test_frame_name_ffi() {
        use std::ffi::CStr;

        unsafe {
            let name_ptr = picoquic_frame_name(frame_type::ACK);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "ack");

            // Test stream frame
            let name_ptr = picoquic_frame_name(0x0a);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "stream");

            let name_ptr = picoquic_frame_name(0xFFFF);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "unknown");
        }
    }
}
