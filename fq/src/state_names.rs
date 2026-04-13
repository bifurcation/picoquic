//! Connection state names and constants.
//!
//! Translated from picoquic/logger.c (textlog_state_name).
//!
//! This module provides string names for QUIC connection states
//! as used in picoquic logging.

use std::ffi::CStr;

// =============================================================================
// Connection State Constants (from picoquic.h)
// =============================================================================

/// QUIC connection state values.
pub mod state {
    /// Client: initial state before sending.
    pub const CLIENT_INIT: u32 = 0;
    /// Client: initial packet sent.
    pub const CLIENT_INIT_SENT: u32 = 1;
    /// Client: renegotiating.
    pub const CLIENT_RENEGOTIATE: u32 = 2;
    /// Client: retry received from server.
    pub const CLIENT_RETRY_RECEIVED: u32 = 3;
    /// Client: initial packet resent after retry.
    pub const CLIENT_INIT_RESENT: u32 = 4;
    /// Server: initial state.
    pub const SERVER_INIT: u32 = 5;
    /// Server: processing handshake.
    pub const SERVER_HANDSHAKE: u32 = 6;
    /// Client: handshake started.
    pub const CLIENT_HANDSHAKE_START: u32 = 7;
    /// Handshake failed.
    pub const HANDSHAKE_FAILURE: u32 = 8;
    /// Handshake failure, resending close.
    pub const HANDSHAKE_FAILURE_RESEND: u32 = 9;
    /// Client: almost ready (1-RTT keys available).
    pub const CLIENT_ALMOST_READY: u32 = 10;
    /// Server: false start (0-RTT accepted).
    pub const SERVER_FALSE_START: u32 = 11;
    /// Server: almost ready.
    pub const SERVER_ALMOST_READY: u32 = 12;
    /// Client: ready to start.
    pub const CLIENT_READY_START: u32 = 13;
    /// Connection ready for application data.
    pub const READY: u32 = 14;
    /// Disconnecting (close sent).
    pub const DISCONNECTING: u32 = 15;
    /// Close received from peer.
    pub const CLOSING_RECEIVED: u32 = 16;
    /// Closing (exchanging close frames).
    pub const CLOSING: u32 = 17;
    /// Draining (waiting for timeout).
    pub const DRAINING: u32 = 18;
    /// Disconnected (connection terminated).
    pub const DISCONNECTED: u32 = 19;
}

// =============================================================================
// State Name Lookup (Core Logic)
// =============================================================================

/// Get the name of a connection state as a C string.
///
/// This is the core lookup function - all logic lives here.
/// Returns "unknown" if the state is not recognized.
///
/// # Arguments
/// * `state` - The connection state value
///
/// # Returns
/// A static CStr with the state name
pub fn state_name_cstr(state: u32) -> &'static CStr {
    match state {
        state::CLIENT_INIT => c"client_init",
        state::CLIENT_INIT_SENT => c"client_init_sent",
        state::CLIENT_RENEGOTIATE => c"client_renegotiate",
        state::CLIENT_RETRY_RECEIVED => c"client_retry_received",
        state::CLIENT_INIT_RESENT => c"client_init_resent",
        state::SERVER_INIT => c"server_init",
        state::SERVER_HANDSHAKE => c"server_handshake",
        state::CLIENT_HANDSHAKE_START => c"client_handshake_start",
        state::HANDSHAKE_FAILURE => c"handshake_failure",
        state::HANDSHAKE_FAILURE_RESEND => c"handshake_failure_resend",
        state::CLIENT_ALMOST_READY => c"client_almost_ready",
        state::SERVER_FALSE_START => c"server_false_start",
        state::SERVER_ALMOST_READY => c"server_almost_ready",
        state::CLIENT_READY_START => c"client_ready_start",
        state::READY => c"ready",
        state::DISCONNECTING => c"disconnecting",
        state::CLOSING_RECEIVED => c"closing_received",
        state::CLOSING => c"closing",
        state::DRAINING => c"draining",
        state::DISCONNECTED => c"disconnected",
        _ => c"unknown",
    }
}

/// Get the name of a connection state.
///
/// Convenience wrapper that returns `&str` for Rust callers.
///
/// # Arguments
/// * `state` - The connection state value
///
/// # Returns
/// A static string with the state name
pub fn state_name(state: u32) -> &'static str {
    // Safe: all our CStr literals are valid UTF-8
    state_name_cstr(state)
        .to_str()
        .expect("state names are ASCII")
}

// =============================================================================
// FFI Export (Thin Wrapper Only)
// =============================================================================

/// Get connection state name (FFI export).
///
/// Returns a pointer to a null-terminated static string.
#[no_mangle]
pub extern "C" fn picoquic_state_name(state: u32) -> *const std::ffi::c_char {
    state_name_cstr(state).as_ptr()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_constants() {
        assert_eq!(state::CLIENT_INIT, 0);
        assert_eq!(state::CLIENT_INIT_SENT, 1);
        assert_eq!(state::CLIENT_RENEGOTIATE, 2);
        assert_eq!(state::CLIENT_RETRY_RECEIVED, 3);
        assert_eq!(state::CLIENT_INIT_RESENT, 4);
        assert_eq!(state::SERVER_INIT, 5);
        assert_eq!(state::SERVER_HANDSHAKE, 6);
        assert_eq!(state::CLIENT_HANDSHAKE_START, 7);
        assert_eq!(state::HANDSHAKE_FAILURE, 8);
        assert_eq!(state::HANDSHAKE_FAILURE_RESEND, 9);
        assert_eq!(state::CLIENT_ALMOST_READY, 10);
        assert_eq!(state::SERVER_FALSE_START, 11);
        assert_eq!(state::SERVER_ALMOST_READY, 12);
        assert_eq!(state::CLIENT_READY_START, 13);
        assert_eq!(state::READY, 14);
        assert_eq!(state::DISCONNECTING, 15);
        assert_eq!(state::CLOSING_RECEIVED, 16);
        assert_eq!(state::CLOSING, 17);
        assert_eq!(state::DRAINING, 18);
        assert_eq!(state::DISCONNECTED, 19);
    }

    #[test]
    fn test_state_name_client() {
        assert_eq!(state_name(state::CLIENT_INIT), "client_init");
        assert_eq!(state_name(state::CLIENT_INIT_SENT), "client_init_sent");
        assert_eq!(state_name(state::CLIENT_RENEGOTIATE), "client_renegotiate");
        assert_eq!(
            state_name(state::CLIENT_RETRY_RECEIVED),
            "client_retry_received"
        );
        assert_eq!(state_name(state::CLIENT_INIT_RESENT), "client_init_resent");
        assert_eq!(
            state_name(state::CLIENT_HANDSHAKE_START),
            "client_handshake_start"
        );
        assert_eq!(
            state_name(state::CLIENT_ALMOST_READY),
            "client_almost_ready"
        );
        assert_eq!(state_name(state::CLIENT_READY_START), "client_ready_start");
    }

    #[test]
    fn test_state_name_server() {
        assert_eq!(state_name(state::SERVER_INIT), "server_init");
        assert_eq!(state_name(state::SERVER_HANDSHAKE), "server_handshake");
        assert_eq!(state_name(state::SERVER_FALSE_START), "server_false_start");
        assert_eq!(
            state_name(state::SERVER_ALMOST_READY),
            "server_almost_ready"
        );
    }

    #[test]
    fn test_state_name_handshake() {
        assert_eq!(state_name(state::HANDSHAKE_FAILURE), "handshake_failure");
        assert_eq!(
            state_name(state::HANDSHAKE_FAILURE_RESEND),
            "handshake_failure_resend"
        );
    }

    #[test]
    fn test_state_name_connection() {
        assert_eq!(state_name(state::READY), "ready");
        assert_eq!(state_name(state::DISCONNECTING), "disconnecting");
        assert_eq!(state_name(state::CLOSING_RECEIVED), "closing_received");
        assert_eq!(state_name(state::CLOSING), "closing");
        assert_eq!(state_name(state::DRAINING), "draining");
        assert_eq!(state_name(state::DISCONNECTED), "disconnected");
    }

    #[test]
    fn test_state_name_unknown() {
        assert_eq!(state_name(100), "unknown");
        assert_eq!(state_name(0xFFFFFFFF), "unknown");
    }

    #[test]
    fn test_state_name_cstr() {
        assert_eq!(state_name_cstr(state::CLIENT_INIT), c"client_init");
        assert_eq!(state_name_cstr(state::READY), c"ready");
        assert_eq!(state_name_cstr(state::DISCONNECTED), c"disconnected");
        assert_eq!(state_name_cstr(100), c"unknown");
    }

    #[test]
    fn test_state_name_ffi() {
        use std::ffi::CStr;

        unsafe {
            let name_ptr = picoquic_state_name(state::CLIENT_INIT);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "client_init");

            let name_ptr = picoquic_state_name(state::READY);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "ready");

            let name_ptr = picoquic_state_name(100);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "unknown");
        }
    }
}
