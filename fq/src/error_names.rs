//! Error code names and constants.
//!
//! Translated from picoquic/error_names.c.
//!
//! This module provides string names for QUIC error codes as defined
//! in RFC 9000 and picoquic-specific local error codes.

use std::ffi::CStr;

// =============================================================================
// Error Code Constants (from picoquic.h)
// =============================================================================

/// QUIC transport error codes (RFC 9000).
pub mod transport {
    /// Internal error (RFC 9000).
    pub const INTERNAL_ERROR: u64 = 0x1;
    /// Server is busy (RFC 9000).
    pub const SERVER_BUSY: u64 = 0x2;
    /// Flow control error (RFC 9000).
    pub const FLOW_CONTROL_ERROR: u64 = 0x3;
    /// Stream limit error (RFC 9000).
    pub const STREAM_LIMIT_ERROR: u64 = 0x4;
    /// Stream state error (RFC 9000).
    pub const STREAM_STATE_ERROR: u64 = 0x5;
    /// Final offset error (RFC 9000).
    pub const FINAL_OFFSET_ERROR: u64 = 0x6;
    /// Frame format error (RFC 9000).
    pub const FRAME_FORMAT_ERROR: u64 = 0x7;
    /// Transport parameter error (RFC 9000).
    pub const PARAMETER_ERROR: u64 = 0x8;
    /// Connection ID limit error (RFC 9000).
    pub const CONNECTION_ID_LIMIT_ERROR: u64 = 0x9;
    /// Protocol violation (RFC 9000).
    pub const PROTOCOL_VIOLATION: u64 = 0xA;
    /// Invalid token (RFC 9000).
    pub const INVALID_TOKEN: u64 = 0xB;
    /// Application error (RFC 9000).
    pub const APPLICATION_ERROR: u64 = 0xC;
    /// Crypto buffer exceeded (RFC 9000).
    pub const CRYPTO_BUFFER_EXCEEDED: u64 = 0xD;
    /// Key update error (RFC 9000).
    pub const KEY_UPDATE_ERROR: u64 = 0xE;
    /// AEAD limit reached (RFC 9000).
    pub const AEAD_LIMIT_REACHED: u64 = 0xF;
    /// Version negotiation error (RFC 9369).
    pub const VERSION_NEGOTIATION_ERROR: u64 = 0x11;
    /// Application abandon (draft-ietf-quic-multipath).
    pub const APPLICATION_ABANDON: u64 = 0x3e;
    /// Resource limit reached (draft-ietf-quic-multipath).
    pub const RESOURCE_LIMIT_REACHED: u64 = 0x3e75;
    /// Unstable interface (draft-ietf-quic-multipath).
    pub const UNSTABLE_INTERFACE: u64 = 0x3e76;
    /// No CID available (draft-ietf-quic-multipath).
    pub const NO_CID_AVAILABLE: u64 = 0x3e77;
}

/// TLS-related error codes.
pub mod tls {
    /// Wrong ALPN (TLS alert).
    pub const ALERT_WRONG_ALPN: u64 = 0x178;
    /// TLS handshake failed.
    pub const HANDSHAKE_FAILED: u64 = 0x201;
}

/// Picoquic local error codes.
pub mod error {
    /// Error class base value.
    pub const CLASS: u64 = 0x400;
    /// Duplicate packet/connection.
    pub const DUPLICATE: u64 = CLASS + 1;
    /// AEAD check failed.
    pub const AEAD_CHECK: u64 = CLASS + 3;
    /// Unexpected packet received.
    pub const UNEXPECTED_PACKET: u64 = CLASS + 4;
    /// Memory allocation failed.
    pub const MEMORY: u64 = CLASS + 5;
    /// Spurious repeat detected.
    pub const SPURIOUS_REPEAT: u64 = CLASS + 6;
    /// Connection ID check failed.
    pub const CNXID_CHECK: u64 = CLASS + 7;
    /// Initial packet too short.
    pub const INITIAL_TOO_SHORT: u64 = CLASS + 8;
    /// Version negotiation spoofed.
    pub const VERSION_NEGOTIATION_SPOOFED: u64 = CLASS + 9;
    /// Malformed transport extension.
    pub const MALFORMED_TRANSPORT_EXTENSION: u64 = CLASS + 10;
    /// Extension buffer too small.
    pub const EXTENSION_BUFFER_TOO_SMALL: u64 = CLASS + 11;
    /// Illegal transport extension.
    pub const ILLEGAL_TRANSPORT_EXTENSION: u64 = CLASS + 12;
    /// Cannot reset stream zero.
    pub const CANNOT_RESET_STREAM_ZERO: u64 = CLASS + 13;
    /// Invalid stream ID.
    pub const INVALID_STREAM_ID: u64 = CLASS + 14;
    /// Stream already closed.
    pub const STREAM_ALREADY_CLOSED: u64 = CLASS + 15;
    /// Frame buffer too small.
    pub const FRAME_BUFFER_TOO_SMALL: u64 = CLASS + 16;
    /// Invalid frame.
    pub const INVALID_FRAME: u64 = CLASS + 17;
    /// Cannot control stream zero.
    pub const CANNOT_CONTROL_STREAM_ZERO: u64 = CLASS + 18;
    /// Retry required.
    pub const RETRY: u64 = CLASS + 19;
    /// Disconnected.
    pub const DISCONNECTED: u64 = CLASS + 20;
    /// Error detected.
    pub const DETECTED: u64 = CLASS + 21;
    /// Invalid ticket.
    pub const INVALID_TICKET: u64 = CLASS + 23;
    /// Invalid file.
    pub const INVALID_FILE: u64 = CLASS + 24;
    /// Send buffer too small.
    pub const SEND_BUFFER_TOO_SMALL: u64 = CLASS + 25;
    /// Unexpected state.
    pub const UNEXPECTED_STATE: u64 = CLASS + 26;
    /// Unexpected error.
    pub const UNEXPECTED_ERROR: u64 = CLASS + 27;
    /// TLS server configured without certificate.
    pub const TLS_SERVER_CON_WITHOUT_CERT: u64 = CLASS + 28;
    /// No such file.
    pub const NO_SUCH_FILE: u64 = CLASS + 29;
    /// Stateless reset received.
    pub const STATELESS_RESET: u64 = CLASS + 30;
    /// Connection deleted.
    pub const CONNECTION_DELETED: u64 = CLASS + 31;
    /// Connection ID segment error.
    pub const CNXID_SEGMENT: u64 = CLASS + 32;
    /// Connection ID not available.
    pub const CNXID_NOT_AVAILABLE: u64 = CLASS + 33;
    /// Migration disabled.
    pub const MIGRATION_DISABLED: u64 = CLASS + 34;
    /// Cannot compute key.
    pub const CANNOT_COMPUTE_KEY: u64 = CLASS + 35;
    /// Cannot set active stream.
    pub const CANNOT_SET_ACTIVE_STREAM: u64 = CLASS + 36;
    /// Cannot change active context.
    pub const CANNOT_CHANGE_ACTIVE_CONTEXT: u64 = CLASS + 37;
    /// Invalid token.
    pub const INVALID_TOKEN: u64 = CLASS + 38;
    /// Initial CID too short.
    pub const INITIAL_CID_TOO_SHORT: u64 = CLASS + 39;
    /// Key rotation not ready.
    pub const KEY_ROTATION_NOT_READY: u64 = CLASS + 40;
    /// AEAD not ready.
    pub const AEAD_NOT_READY: u64 = CLASS + 41;
    /// No ALPN provided.
    pub const NO_ALPN_PROVIDED: u64 = CLASS + 42;
    /// No callback provided.
    pub const NO_CALLBACK_PROVIDED: u64 = CLASS + 43;
    /// Stream receive complete (not an error).
    pub const STREAM_RECEIVE_COMPLETE: u64 = CLASS + 44;
    /// Packet header parsing error.
    pub const PACKET_HEADER_PARSING: u64 = CLASS + 45;
    /// QUIC bit missing.
    pub const QUIC_BIT_MISSING: u64 = CLASS + 46;
    /// Terminate packet loop (not an error).
    pub const NO_ERROR_TERMINATE_PACKET_LOOP: u64 = CLASS + 47;
    /// Simulate NAT (not an error).
    pub const NO_ERROR_SIMULATE_NAT: u64 = CLASS + 48;
    /// Simulate migration (not an error).
    pub const NO_ERROR_SIMULATE_MIGRATION: u64 = CLASS + 49;
    /// Version not supported.
    pub const VERSION_NOT_SUPPORTED: u64 = CLASS + 50;
    /// Idle timeout.
    pub const IDLE_TIMEOUT: u64 = CLASS + 51;
    /// Repeat timeout.
    pub const REPEAT_TIMEOUT: u64 = CLASS + 52;
    /// Handshake timeout.
    pub const HANDSHAKE_TIMEOUT: u64 = CLASS + 53;
    /// Socket error.
    pub const SOCKET_ERROR: u64 = CLASS + 54;
    /// Version negotiation.
    pub const VERSION_NEGOTIATION: u64 = CLASS + 55;
    /// Packet too long.
    pub const PACKET_TOO_LONG: u64 = CLASS + 56;
    /// Packet wrong version.
    pub const PACKET_WRONG_VERSION: u64 = CLASS + 57;
    /// Port blocked.
    pub const PORT_BLOCKED: u64 = CLASS + 58;
    /// Datagram too long.
    pub const DATAGRAM_TOO_LONG: u64 = CLASS + 59;
    /// Invalid path ID.
    pub const PATH_ID_INVALID: u64 = CLASS + 60;
    /// Retry needed.
    pub const RETRY_NEEDED: u64 = CLASS + 61;
    /// Server busy.
    pub const SERVER_BUSY: u64 = CLASS + 62;
    /// Duplicate path.
    pub const PATH_DUPLICATE: u64 = CLASS + 63;
    /// Blocked by lack of path ID.
    pub const PATH_ID_BLOCKED: u64 = CLASS + 64;
    /// Blocked by lack of CID.
    pub const PATH_CID_BLOCKED: u64 = CLASS + 65;
    /// Path address family error.
    pub const PATH_ADDRESS_FAMILY: u64 = CLASS + 66;
    /// Path not ready.
    pub const PATH_NOT_READY: u64 = CLASS + 67;
    /// Path limit exceeded.
    pub const PATH_LIMIT_EXCEEDED: u64 = CLASS + 68;
    /// Redirected to proxy (not an error).
    pub const REDIRECTED: u64 = CLASS + 69;
    /// Padding packet (random bytes at end of datagram).
    pub const PADDING_PACKET: u64 = CLASS + 70;
}

// =============================================================================
// Error Name Lookup (Core Logic)
// =============================================================================

/// Get the name of an error code as a C string.
///
/// This is the core lookup function - all logic lives here.
/// Returns "unknown" if the error code is not recognized.
///
/// # Arguments
/// * `error_code` - The error code number
///
/// # Returns
/// A static CStr with the error name
pub fn error_name_cstr(error_code: u64) -> &'static CStr {
    match error_code {
        // Protocol errors defined in QUIC spec
        transport::INTERNAL_ERROR => c"internal",
        transport::SERVER_BUSY => c"server busy",
        transport::FLOW_CONTROL_ERROR => c"flow control",
        transport::STREAM_LIMIT_ERROR => c"stream limit",
        transport::STREAM_STATE_ERROR => c"stream state",
        transport::FINAL_OFFSET_ERROR => c"final offset",
        transport::FRAME_FORMAT_ERROR => c"frame format",
        transport::PARAMETER_ERROR => c"parameter",
        transport::CONNECTION_ID_LIMIT_ERROR => c"connection_id limit",
        transport::PROTOCOL_VIOLATION => c"protocol violation",
        transport::INVALID_TOKEN => c"invalid token",
        transport::APPLICATION_ERROR => c"application",
        transport::CRYPTO_BUFFER_EXCEEDED => c"crypto buffer exceeded",
        transport::KEY_UPDATE_ERROR => c"key update",
        transport::AEAD_LIMIT_REACHED => c"aead limit",
        tls::ALERT_WRONG_ALPN => c"wrong alpn",
        tls::HANDSHAKE_FAILED => c"tls handshake failed",
        transport::VERSION_NEGOTIATION_ERROR => c"version negotiation",
        transport::APPLICATION_ABANDON => c"application abandon",
        transport::RESOURCE_LIMIT_REACHED => c"resource limit reached",
        transport::UNSTABLE_INTERFACE => c"unstable interface",
        transport::NO_CID_AVAILABLE => c"no CID available",
        // Picoquic local error codes
        error::DUPLICATE => c"duplicate",
        error::AEAD_CHECK => c"payload_decrypt_error",
        error::UNEXPECTED_PACKET => c"unexpected packet",
        error::MEMORY => c"memory",
        error::CNXID_CHECK => c"connection ID check",
        error::INITIAL_TOO_SHORT => c"",
        error::VERSION_NEGOTIATION_SPOOFED => c"version negotation spoofed",
        error::MALFORMED_TRANSPORT_EXTENSION => c"malformed transport extension",
        error::EXTENSION_BUFFER_TOO_SMALL => c"extension buffer too small",
        error::ILLEGAL_TRANSPORT_EXTENSION => c"illegal transport extension",
        error::CANNOT_RESET_STREAM_ZERO => c"cannot reset the crypto stream",
        error::INVALID_STREAM_ID => c"invalid stream id",
        error::STREAM_ALREADY_CLOSED => c"stream already closed",
        error::FRAME_BUFFER_TOO_SMALL => c"frame buffer too small",
        error::INVALID_FRAME => c"invalid frame",
        error::CANNOT_CONTROL_STREAM_ZERO => c"cannot control the crypto stream",
        error::RETRY => c"retry",
        error::DISCONNECTED => c"disconnected",
        error::DETECTED => c"error detected",
        error::INVALID_TICKET => c"invalid ticket",
        error::INVALID_FILE => c"invalid file",
        error::SEND_BUFFER_TOO_SMALL => c"send buffer too small",
        error::UNEXPECTED_STATE => c"unexpected state",
        error::UNEXPECTED_ERROR => c"unexpected error",
        error::TLS_SERVER_CON_WITHOUT_CERT => c"server configuration without cert",
        error::NO_SUCH_FILE => c"no such file",
        error::STATELESS_RESET => c"stateless reset",
        error::CONNECTION_DELETED => c"connection deleted",
        error::CNXID_SEGMENT => c"connection ID segment error",
        error::CNXID_NOT_AVAILABLE => c"connection ID not available",
        error::MIGRATION_DISABLED => c"migration disabled",
        error::CANNOT_COMPUTE_KEY => c"cannot compute key",
        error::CANNOT_SET_ACTIVE_STREAM => c"cannot set active stream",
        error::CANNOT_CHANGE_ACTIVE_CONTEXT => c"cannot change active context",
        error::INVALID_TOKEN => c"invalid token",
        error::INITIAL_CID_TOO_SHORT => c"initial CID too short",
        error::KEY_ROTATION_NOT_READY => c"key rotation not ready",
        error::AEAD_NOT_READY => c"aead not ready",
        error::NO_ALPN_PROVIDED => c"no ALPN provided",
        error::NO_CALLBACK_PROVIDED => c"no callback provided",
        error::STREAM_RECEIVE_COMPLETE => c"stream receive complete",
        error::PACKET_HEADER_PARSING => c"packet header parsing",
        error::QUIC_BIT_MISSING => c"QUIC bit missing",
        error::NO_ERROR_TERMINATE_PACKET_LOOP => c"terminate packet loop (not an error)",
        error::NO_ERROR_SIMULATE_NAT => c"simulate NAT (not an error)",
        error::NO_ERROR_SIMULATE_MIGRATION => c"simulate migration (not an error)",
        error::VERSION_NOT_SUPPORTED => c"version not supported",
        error::IDLE_TIMEOUT => c"idle timeout",
        error::REPEAT_TIMEOUT => c"repeat timeout",
        error::HANDSHAKE_TIMEOUT => c"handshake timeout",
        error::SOCKET_ERROR => c"socket",
        error::VERSION_NEGOTIATION => c"version negotiation",
        error::PACKET_TOO_LONG => c"packet too long",
        error::PACKET_WRONG_VERSION => c"wrong version",
        error::PORT_BLOCKED => c"port blocked",
        error::DATAGRAM_TOO_LONG => c"datagram too long",
        error::PATH_ID_INVALID => c"invalid path ID",
        error::RETRY_NEEDED => c"retry needed",
        error::SERVER_BUSY => c"server busy",
        error::PATH_DUPLICATE => c"duplicate path",
        error::PATH_ID_BLOCKED => c"blocked by lack of path ID",
        error::PATH_CID_BLOCKED => c"blocked by lack of CID",
        error::PATH_ADDRESS_FAMILY => c"path address family",
        error::PATH_NOT_READY => c"path not ready",
        error::PATH_LIMIT_EXCEEDED => c"path limit exceeded",
        error::REDIRECTED => c"redirected to proxy (not an error)",
        error::PADDING_PACKET => c"padding_packet",
        _ => {
            // Check for crypto error alerts (0x100-0x1FF)
            if error_code > 0x100 && error_code < 0x200 {
                c"crypto error alert"
            }
            // Check for unknown picoquic errors (0x400-0x4FF)
            else if error_code > 0x400 && error_code < 0x500 {
                c"unknown picoquic error"
            } else {
                c"unknown"
            }
        }
    }
}

/// Get the name of an error code.
///
/// Convenience wrapper that returns `&str` for Rust callers.
///
/// # Arguments
/// * `error_code` - The error code number
///
/// # Returns
/// A static string with the error name
pub fn error_name(error_code: u64) -> &'static str {
    // Safe: all our CStr literals are valid UTF-8
    error_name_cstr(error_code)
        .to_str()
        .expect("error names are ASCII")
}

// =============================================================================
// FFI Export (Thin Wrapper Only)
// =============================================================================

/// Get error name (FFI export).
///
/// Returns a pointer to a null-terminated static string.
#[no_mangle]
pub extern "C" fn picoquic_error_name(error_code: u64) -> *const std::ffi::c_char {
    error_name_cstr(error_code).as_ptr()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_error_constants() {
        assert_eq!(transport::INTERNAL_ERROR, 0x1);
        assert_eq!(transport::SERVER_BUSY, 0x2);
        assert_eq!(transport::FLOW_CONTROL_ERROR, 0x3);
        assert_eq!(transport::STREAM_LIMIT_ERROR, 0x4);
        assert_eq!(transport::STREAM_STATE_ERROR, 0x5);
        assert_eq!(transport::FINAL_OFFSET_ERROR, 0x6);
        assert_eq!(transport::FRAME_FORMAT_ERROR, 0x7);
        assert_eq!(transport::PARAMETER_ERROR, 0x8);
        assert_eq!(transport::CONNECTION_ID_LIMIT_ERROR, 0x9);
        assert_eq!(transport::PROTOCOL_VIOLATION, 0xA);
        assert_eq!(transport::INVALID_TOKEN, 0xB);
        assert_eq!(transport::APPLICATION_ERROR, 0xC);
        assert_eq!(transport::CRYPTO_BUFFER_EXCEEDED, 0xD);
        assert_eq!(transport::KEY_UPDATE_ERROR, 0xE);
        assert_eq!(transport::AEAD_LIMIT_REACHED, 0xF);
        assert_eq!(transport::VERSION_NEGOTIATION_ERROR, 0x11);
    }

    #[test]
    fn test_tls_error_constants() {
        assert_eq!(tls::ALERT_WRONG_ALPN, 0x178);
        assert_eq!(tls::HANDSHAKE_FAILED, 0x201);
    }

    #[test]
    fn test_local_error_constants() {
        assert_eq!(error::CLASS, 0x400);
        assert_eq!(error::DUPLICATE, 0x401);
        assert_eq!(error::AEAD_CHECK, 0x403);
        assert_eq!(error::MEMORY, 0x405);
        assert_eq!(error::IDLE_TIMEOUT, 0x433);
    }

    #[test]
    fn test_error_name_transport() {
        assert_eq!(error_name(transport::INTERNAL_ERROR), "internal");
        assert_eq!(error_name(transport::SERVER_BUSY), "server busy");
        assert_eq!(error_name(transport::FLOW_CONTROL_ERROR), "flow control");
        assert_eq!(
            error_name(transport::PROTOCOL_VIOLATION),
            "protocol violation"
        );
        assert_eq!(error_name(transport::AEAD_LIMIT_REACHED), "aead limit");
    }

    #[test]
    fn test_error_name_local() {
        assert_eq!(error_name(error::DUPLICATE), "duplicate");
        assert_eq!(error_name(error::MEMORY), "memory");
        assert_eq!(error_name(error::DISCONNECTED), "disconnected");
        assert_eq!(error_name(error::IDLE_TIMEOUT), "idle timeout");
        assert_eq!(error_name(error::STATELESS_RESET), "stateless reset");
    }

    #[test]
    fn test_error_name_crypto_alert() {
        // Crypto alerts are in range 0x100-0x1FF
        assert_eq!(error_name(0x101), "crypto error alert");
        assert_eq!(error_name(0x150), "crypto error alert");
        assert_eq!(error_name(0x1FF), "crypto error alert");
    }

    #[test]
    fn test_error_name_unknown_picoquic() {
        // Unknown picoquic errors in range 0x400-0x4FF
        assert_eq!(error_name(0x4FF), "unknown picoquic error");
    }

    #[test]
    fn test_error_name_unknown() {
        assert_eq!(error_name(0xFFFFFFFF), "unknown");
        assert_eq!(error_name(0x300), "unknown");
    }

    #[test]
    fn test_error_name_cstr() {
        assert_eq!(error_name_cstr(transport::INTERNAL_ERROR), c"internal");
        assert_eq!(error_name_cstr(error::MEMORY), c"memory");
        assert_eq!(error_name_cstr(0x150), c"crypto error alert");
        assert_eq!(error_name_cstr(0xFFFF), c"unknown");
    }

    #[test]
    fn test_error_name_ffi() {
        use std::ffi::CStr;

        unsafe {
            let name_ptr = picoquic_error_name(transport::INTERNAL_ERROR);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "internal");

            let name_ptr = picoquic_error_name(error::DISCONNECTED);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "disconnected");

            let name_ptr = picoquic_error_name(0x150);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "crypto error alert");
        }
    }
}
