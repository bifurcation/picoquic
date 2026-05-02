//! Translation of `quic/quic.h`.
//!
//! `quic.h` is the kitchen-sink public header for the
//! quic-core library: it defines the protocol error codes, the
//! transport-parameter identifiers, the application-facing enums, the
//! opaque QUIC / connection / path types, the public structs that
//! cross the API boundary (`tp_t`, `path_quality_t`,
//! `per_ack_state_t`, `congestion_algorithm_t`, …)
//! and the dozens of free functions that make up the application API.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//!
//! Pointer-shape and translation policy notes that apply throughout
//! this module:
//!
//! * `quic_t`, `cnx_t`, `path_t` are
//!   opaque types defined in `internal.h`.  Phase 1 declares
//!   them as empty structs here so the public API can refer to them;
//!   the real layout lands when the internal header is translated.
//! * `struct sockaddr*` parameters and fields map to
//!   [`core::net::SocketAddr`].  The C source uses tagged unions of
//!   `sockaddr_in` / `sockaddr_in6`, which `SocketAddr` models
//!   directly.  Output parameters of type `struct sockaddr_storage*`
//!   become a `SocketAddr` returned by value.
//! * Function-pointer typedefs become traits (one trait per typedef
//!   by default).  Trait grouping is used for the four-function
//!   `congestion_algorithm_t` vtable, where the C side
//!   always installs the four together.
//! * `void*` "callback context" arguments are folded into the trait
//!   implementor's state.  Per-stream / per-path application
//!   contexts that flow through the stack (e.g. `stream_ctx`) stay
//!   as `*mut c_void` — they are produced by the application and
//!   handed back unchanged.
//! * C `int` / `int64_t` return values that encode 0/-1 or 0/error
//!   status become `Result<T, ()>` — the crate-level `Error` enum
//!   doesn't exist yet, so `()` is a placeholder per the Phase 1
//!   contract.  `i32` / `i64` are kept where the C value is a real
//!   integer (e.g., wake delays in microseconds, interface index).
//! * Single-bit `unsigned int : 1` bitfields collapse to `bool`.
//! * `va_list` has no Rust counterpart; the `_v` log helpers are
//!   omitted in favour of one entry point that takes
//!   `core::fmt::Arguments<'_>`.
//! * `extern` global state (`congestion_control_algorithms`)
//!   becomes accessor functions that hand out a borrowed slice; the
//!   companion `_nb_` length variable disappears (the slice carries
//!   its length).

#![allow(non_camel_case_types)]
// `tp_*`, `nb_packet_context`, and the rest mirror
// C `#define` / enum tag names verbatim — Rust's `non_upper_case_globals`
// lint disagrees with that style, so silence it module-wide.
#![allow(non_upper_case_globals)]
// Phase 1 stubs return `Result<T, ()>` until the crate-level `Error`
// type lands; clippy's `result_unit_err` is silenced module-wide.
#![allow(clippy::result_unit_err)]
// Many translated functions mirror C signatures with >7 parameters.
// Builder patterns or shape changes are out of scope for Phase 1.
#![allow(clippy::too_many_arguments)]

pub mod binlog;
pub mod bytestream;
pub mod cc_common;
pub mod config;
pub mod crypto_provider_api;
pub mod hash;
pub mod internal;
pub mod lb;
pub mod logger;
pub mod packet_loop;
pub mod performance_log;
pub mod qlog;
pub mod siphash;
pub mod socks;
pub mod splay;
pub mod test_dualq;
pub mod tls_api;
pub mod unified_log;
pub mod utils;

use core::ffi::c_void;
use core::net::SocketAddr;

// ---------------------------------------------------------------------------
// Version string and error class.
//
// Error values come straight from `#define` and would naturally be
// untyped in C; in Rust we commit to `u64` because the application
// API takes them as `uint64_t error_code` parameters and returns
// them in `get_local_error` etc.

/// Library version string.  C: `VERSION`.
pub const VERSION: &str = "1.1.48.0";

/// Base offset for quic's internal error codes.  Allocated in
/// the `0x400`+ range so they never collide with QUIC transport or
/// TLS alert codes.
pub const ERROR_CLASS: u64 = 0x400;

pub const ERROR_DUPLICATE: u64 = ERROR_CLASS + 1;
pub const ERROR_AEAD_CHECK: u64 = ERROR_CLASS + 3;
pub const ERROR_UNEXPECTED_PACKET: u64 = ERROR_CLASS + 4;
pub const ERROR_MEMORY: u64 = ERROR_CLASS + 5;
pub const ERROR_CNXID_CHECK: u64 = ERROR_CLASS + 7;
pub const ERROR_INITIAL_TOO_SHORT: u64 = ERROR_CLASS + 8;
pub const ERROR_VERSION_NEGOTIATION_SPOOFED: u64 = ERROR_CLASS + 9;
pub const ERROR_MALFORMED_TRANSPORT_EXTENSION: u64 = ERROR_CLASS + 10;
pub const ERROR_EXTENSION_BUFFER_TOO_SMALL: u64 = ERROR_CLASS + 11;
pub const ERROR_ILLEGAL_TRANSPORT_EXTENSION: u64 = ERROR_CLASS + 12;
pub const ERROR_CANNOT_RESET_STREAM_ZERO: u64 = ERROR_CLASS + 13;
pub const ERROR_INVALID_STREAM_ID: u64 = ERROR_CLASS + 14;
pub const ERROR_STREAM_ALREADY_CLOSED: u64 = ERROR_CLASS + 15;
pub const ERROR_FRAME_BUFFER_TOO_SMALL: u64 = ERROR_CLASS + 16;
pub const ERROR_INVALID_FRAME: u64 = ERROR_CLASS + 17;
pub const ERROR_CANNOT_CONTROL_STREAM_ZERO: u64 = ERROR_CLASS + 18;
pub const ERROR_RETRY: u64 = ERROR_CLASS + 19;
pub const ERROR_DISCONNECTED: u64 = ERROR_CLASS + 20;
pub const ERROR_DETECTED: u64 = ERROR_CLASS + 21;
pub const ERROR_INVALID_TICKET: u64 = ERROR_CLASS + 23;
pub const ERROR_INVALID_FILE: u64 = ERROR_CLASS + 24;
pub const ERROR_SEND_BUFFER_TOO_SMALL: u64 = ERROR_CLASS + 25;
pub const ERROR_UNEXPECTED_STATE: u64 = ERROR_CLASS + 26;
pub const ERROR_UNEXPECTED_ERROR: u64 = ERROR_CLASS + 27;
pub const ERROR_TLS_SERVER_CON_WITHOUT_CERT: u64 = ERROR_CLASS + 28;
pub const ERROR_NO_SUCH_FILE: u64 = ERROR_CLASS + 29;
pub const ERROR_STATELESS_RESET: u64 = ERROR_CLASS + 30;
pub const ERROR_CONNECTION_DELETED: u64 = ERROR_CLASS + 31;
pub const ERROR_CNXID_SEGMENT: u64 = ERROR_CLASS + 32;
pub const ERROR_CNXID_NOT_AVAILABLE: u64 = ERROR_CLASS + 33;
pub const ERROR_MIGRATION_DISABLED: u64 = ERROR_CLASS + 34;
pub const ERROR_CANNOT_COMPUTE_KEY: u64 = ERROR_CLASS + 35;
pub const ERROR_CANNOT_SET_ACTIVE_STREAM: u64 = ERROR_CLASS + 36;
pub const ERROR_CANNOT_CHANGE_ACTIVE_CONTEXT: u64 = ERROR_CLASS + 37;
pub const ERROR_INVALID_TOKEN: u64 = ERROR_CLASS + 38;
pub const ERROR_INITIAL_CID_TOO_SHORT: u64 = ERROR_CLASS + 39;
pub const ERROR_KEY_ROTATION_NOT_READY: u64 = ERROR_CLASS + 40;
pub const ERROR_AEAD_NOT_READY: u64 = ERROR_CLASS + 41;
pub const ERROR_NO_ALPN_PROVIDED: u64 = ERROR_CLASS + 42;
pub const ERROR_NO_CALLBACK_PROVIDED: u64 = ERROR_CLASS + 43;
pub const STREAM_RECEIVE_COMPLETE: u64 = ERROR_CLASS + 44;
pub const ERROR_PACKET_HEADER_PARSING: u64 = ERROR_CLASS + 45;
pub const ERROR_QUIC_BIT_MISSING: u64 = ERROR_CLASS + 46;
pub const NO_ERROR_TERMINATE_PACKET_LOOP: u64 = ERROR_CLASS + 47;
pub const NO_ERROR_SIMULATE_NAT: u64 = ERROR_CLASS + 48;
pub const NO_ERROR_SIMULATE_MIGRATION: u64 = ERROR_CLASS + 49;
pub const ERROR_VERSION_NOT_SUPPORTED: u64 = ERROR_CLASS + 50;
pub const ERROR_IDLE_TIMEOUT: u64 = ERROR_CLASS + 51;
pub const ERROR_REPEAT_TIMEOUT: u64 = ERROR_CLASS + 52;
pub const ERROR_HANDSHAKE_TIMEOUT: u64 = ERROR_CLASS + 53;
pub const ERROR_SOCKET_ERROR: u64 = ERROR_CLASS + 54;
pub const ERROR_VERSION_NEGOTIATION: u64 = ERROR_CLASS + 55;
pub const ERROR_PACKET_TOO_LONG: u64 = ERROR_CLASS + 56;
pub const ERROR_PACKET_WRONG_VERSION: u64 = ERROR_CLASS + 57;
pub const ERROR_PORT_BLOCKED: u64 = ERROR_CLASS + 58;
pub const ERROR_DATAGRAM_TOO_LONG: u64 = ERROR_CLASS + 59;
pub const ERROR_PATH_ID_INVALID: u64 = ERROR_CLASS + 60;
pub const ERROR_RETRY_NEEDED: u64 = ERROR_CLASS + 61;
pub const ERROR_SERVER_BUSY: u64 = ERROR_CLASS + 62;
pub const ERROR_PATH_DUPLICATE: u64 = ERROR_CLASS + 63;
pub const ERROR_PATH_ID_BLOCKED: u64 = ERROR_CLASS + 64;
pub const ERROR_PATH_CID_BLOCKED: u64 = ERROR_CLASS + 65;
pub const ERROR_PATH_ADDRESS_FAMILY: u64 = ERROR_CLASS + 66;
pub const ERROR_PATH_NOT_READY: u64 = ERROR_CLASS + 67;
pub const ERROR_PATH_LIMIT_EXCEEDED: u64 = ERROR_CLASS + 68;
/// Not actually an error: signals that the packet was captured by a
/// proxy and needs no further processing.
pub const ERROR_REDIRECTED: u64 = ERROR_CLASS + 69;
pub const ERROR_PADDING_PACKET: u64 = ERROR_CLASS + 70;

// ---------------------------------------------------------------------------
// Protocol errors defined by the QUIC and TLS specs.

pub const TRANSPORT_INTERNAL_ERROR: u64 = 0x1;
pub const TRANSPORT_SERVER_BUSY: u64 = 0x2;
pub const TRANSPORT_FLOW_CONTROL_ERROR: u64 = 0x3;
pub const TRANSPORT_STREAM_LIMIT_ERROR: u64 = 0x4;
pub const TRANSPORT_STREAM_STATE_ERROR: u64 = 0x5;
pub const TRANSPORT_FINAL_OFFSET_ERROR: u64 = 0x6;
pub const TRANSPORT_FRAME_FORMAT_ERROR: u64 = 0x7;
pub const TRANSPORT_PARAMETER_ERROR: u64 = 0x8;
pub const TRANSPORT_CONNECTION_ID_LIMIT_ERROR: u64 = 0x9;
pub const TRANSPORT_PROTOCOL_VIOLATION: u64 = 0xA;
pub const TRANSPORT_INVALID_TOKEN: u64 = 0xB;
pub const TRANSPORT_APPLICATION_ERROR: u64 = 0xC;
pub const TRANSPORT_CRYPTO_BUFFER_EXCEEDED: u64 = 0xD;
pub const TRANSPORT_KEY_UPDATE_ERROR: u64 = 0xE;
pub const TRANSPORT_AEAD_LIMIT_REACHED: u64 = 0xF;

pub const TLS_ALERT_WRONG_ALPN: u64 = 0x178;
pub const TLS_HANDSHAKE_FAILED: u64 = 0x201;
pub const TRANSPORT_VERSION_NEGOTIATION_ERROR: u64 = 0x11;

/// Per draft quic-multipath 20.
pub const TRANSPORT_APPLICATION_ABANDON: u64 = 0x3e;
pub const TRANSPORT_RESOURCE_LIMIT_REACHED: u64 = 0x3e75;
pub const TRANSPORT_UNSTABLE_INTERFACE: u64 = 0x3e76;
pub const TRANSPORT_NO_CID_AVAILABLE: u64 = 0x3e77;

/// C macro `TRANSPORT_CRYPTO_ERROR(Alert)`.  Combines the
/// TLS-alert range marker (`0x100`) with the alert byte.
pub const fn transport_crypto_error(alert: u8) -> u16 {
    0x100 | (alert as u16)
}

// ---------------------------------------------------------------------------
// Packet sizes and miscellaneous constants.

pub const MAX_PACKET_SIZE: usize = 1536;
pub const INITIAL_MTU_IPV4: usize = 1252;
pub const INITIAL_MTU_IPV6: usize = 1232;
pub const RESET_SECRET_SIZE: usize = 16;
pub const RESET_PACKET_PAD_SIZE: usize = 23;
pub const RESET_PACKET_MIN_SIZE: usize = RESET_PACKET_PAD_SIZE + RESET_SECRET_SIZE;
pub const MAX_CRYPTO_BUFFER_GAP: usize = 16384;

pub const LOG_PACKET_MAX_SEQUENCE: u32 = 100;

/// First 4 bytes of `SHA256("QUIC Masque")`.  Used as a sentinel
/// interface index where no real OS interface is meaningful.
pub const RESERVED_IF_INDEX: u32 = 0x09cb8ed3;

// TLS cipher-suite identifiers (IANA).
pub const AES_128_GCM_SHA256: u16 = 0x1301;
pub const AES_256_GCM_SHA384: u16 = 0x1302;
pub const CHACHA20_POLY1305_SHA256: u16 = 0x1303;

pub const GROUP_SECP256R1: u16 = 23;

// ECN code points for the IP header `tos` byte.
pub const ECN_ECT_0: u8 = 0x02;
pub const ECN_ECT_1: u8 = 0x01;
pub const ECN_CE: u8 = 0x03;

/// C macro `FOURCC(a, b, c, d)`.  Produces a 32-bit code from four
/// bytes in little-endian order.
pub const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    ((d as u32) << 24) | ((c as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

// ---------------------------------------------------------------------------
// Connection state.

/// Mirrors the C `state_enum`.  Discriminants follow the
/// declaration order of the C enum.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum state_enum {
    state_client_init,
    state_client_init_sent,
    state_client_renegotiate,
    state_client_retry_received,
    state_client_init_resent,
    state_server_init,
    state_server_handshake,
    state_client_handshake_start,
    state_handshake_failure,
    state_handshake_failure_resend,
    state_client_almost_ready,
    state_server_false_start,
    state_server_almost_ready,
    state_client_ready_start,
    state_ready,
    state_disconnecting,
    state_closing_received,
    state_closing,
    state_draining,
    state_disconnected,
}

// ---------------------------------------------------------------------------
// Transport-parameter identifiers.

/// QUIC transport-parameter identifiers.  Wire values exceed `u32`
/// for several extension parameters, hence the `#[repr(u64)]`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u64)]
pub enum Tp {
    OriginalConnectionId = 0,
    IdleTimeout = 1,
    StatelessResetToken = 2,
    MaxPacketSize = 3,
    InitialMaxData = 4,
    InitialMaxStreamDataBidiLocal = 5,
    InitialMaxStreamDataBidiRemote = 6,
    InitialMaxStreamDataUni = 7,
    InitialMaxStreamsBidi = 8,
    InitialMaxStreamsUni = 9,
    AckDelayExponent = 10,
    MaxAckDelay = 11,
    DisableMigration = 12,
    ServerPreferredAddress = 13,
    ActiveConnectionIdLimit = 14,
    HandshakeConnectionId = 15,
    RetryConnectionId = 16,
    /// Per `draft-quic-multipath 20`.
    InitialMaxPathId = 0x3e,
    VersionNegotiation = 0x11,
    /// Per `draft-pauly-quic-datagram-05`.
    MaxDatagramFrameSize = 32,
    TestLargeChello = 3127,
    EnableLossBit = 0x1057,
    /// `(x & 1)` ↔ "want timestamps", `(x & 2)` ↔ "can send timestamps".
    EnableTimeStamp = 0x7158,
    GreaseQuicBit = 0x2ab2,
    /// Per `draft-kuhn-quic-0rtt-bdp-09`.
    EnableBdpFrame = 0xebd9,
    MinAckDelay = 0xff04de1b,
    /// Per `draft-seemann-quic-address-discovery`.
    AddressDiscovery = 0x9f81a176,
    /// Per `draft-ietf-quic-reliable-stream-reset-07`.
    ResetStreamAt = 0x17f7586d2cb571,
}

// ---------------------------------------------------------------------------
// Packet contexts and enumerated policy types.

/// Mirrors `packet_context_enum`.  The trailing
/// `nb_packet_context` was a count; the Rust idiom is the
/// `pub const` below, leaving the enum as just the real variants.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum packet_context_enum {
    packet_context_application = 0,
    packet_context_handshake = 1,
    packet_context_initial = 2,
}

/// Number of packet contexts.  C: `nb_packet_context`.
pub const nb_packet_context: usize = 3;

/// PMTUD policy for a connection.  C: `pmtud_policy_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum pmtud_policy_enum {
    /// Default opportunistic PMTUD.
    #[default]
    pmtud_basic = 0,
    /// Force PMTUD as soon as possible.
    pmtud_required = 1,
    /// Only do PMTUD if a lot of data has to be sent.
    pmtud_delayed = 2,
    /// Never do PMTUD.
    pmtud_blocked = 3,
}

/// Spin-bit variant policy.  C: `spinbit_version_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum spinbit_version_enum {
    /// Default behaviour, per the spin-bit draft.
    #[default]
    spinbit_basic = 0,
    /// Randomise per packet.
    spinbit_random = 1,
    /// Null behaviour, randomised per path.
    spinbit_null = 2,
    /// Test-only "always on" mode.  Not valid as a per-connection
    /// override (server only; see `set_default_spinbit_policy`).
    spinbit_on = 3,
}

/// Loss-bit support level.  C: `lossbit_version_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum lossbit_version_enum {
    #[default]
    lossbit_none = 0,
    lossbit_send_only = 1,
    lossbit_send_receive = 2,
}

/// Path scheduling status.  C: `path_status_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum path_status_enum {
    #[default]
    path_status_available = 0,
    path_status_backup = 1,
}

// ---------------------------------------------------------------------------
// Connection ID.

pub const CONNECTION_ID_MIN_SIZE: usize = 0;
pub const CONNECTION_ID_MAX_SIZE: usize = 20;

/// Fixed-capacity QUIC connection ID.  C: `connection_id_t`.
///
/// Stored as a 20-byte buffer plus a length so the type is `Copy`,
/// matching the C usage where connection IDs are passed by value
/// in many APIs (`get_local_cnxid`, `create_cnx`,
/// …).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct connection_id_t {
    pub id: [u8; CONNECTION_ID_MAX_SIZE],
    pub id_len: u8,
}

// ---------------------------------------------------------------------------
// IO vectors.
//
// `ptls_iovec_t` is forward-declared from the tls library, which
// is an external dependency that hasn't been translated yet.
// `iovec_t` is quic's matching shape, intended for
// applications that don't want a hard dependency on tls.h.  The
// two are layout-compatible; in C the application can cast a
// `ptls_iovec_t*` to a `iovec_t*`.  Phase 1 keeps both as
// distinct opaque/struct types; Phase 3 may revisit if a true
// shared layout is needed at the FFI boundary.

/// Forward declaration of `ptls_iovec_t` from tls.  The real
/// definition lands when tls bindings are introduced.
pub struct ptls_iovec_t {
    _opaque: [u8; 0],
}

/// quic-defined IO vector, layout-compatible with
/// `ptls_iovec_t` so applications can cast between the two without
/// pulling in `tls.h`.  C: `iovec_t`.
///
/// `repr(C)` is kept because C code aliases this with
/// `ptls_iovec_t*` — the layout *is* the contract.  `base` stays a
/// raw pointer rather than `&[u8]` because the buffer's lifetime is
/// not tied to the iovec; Phase 3 may revisit at specific call
/// sites.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct iovec_t {
    pub base: *mut u8,
    pub len: usize,
}

// ---------------------------------------------------------------------------
// Opaque types (forward declarations).
//
// `quic_t`, `cnx_t`, and `path_t` are
// defined in `internal.h`.  Phase 1 declares them as empty
// structs so callers can refer to them; the real layout lands when
// the internal header is translated.  The `_opaque` field prevents
// callers from constructing one accidentally, and the `Debug` impl
// gives the structs a printable shape for log lines.

// Full bodies live in `crate::internal`; pull them in for use within
// this module's signatures (no re-export — callers reach them as
// `crate::internal::*`).
use crate::internal::{cnx_t, path_t, quic_t};

// ---------------------------------------------------------------------------
// Application callback events.

/// Event type for the application stream/data callback.  C:
/// `call_back_event_t`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum call_back_event_t {
    callback_stream_data = 0,
    callback_stream_fin,
    callback_stream_reset,
    callback_stop_sending,
    callback_stateless_reset,
    callback_close,
    callback_application_close,
    callback_stream_gap,
    callback_prepare_to_send,
    callback_almost_ready,
    callback_ready,
    callback_datagram,
    callback_version_negotiation,
    callback_request_alpn_list,
    callback_set_alpn,
    callback_pacing_changed,
    callback_prepare_datagram,
    callback_datagram_acked,
    callback_datagram_lost,
    callback_datagram_spurious,
    callback_path_available,
    callback_path_suspended,
    callback_path_deleted,
    callback_path_quality_changed,
    callback_path_address_observed,
    callback_app_wakeup,
    callback_next_path_allowed,
}

// ---------------------------------------------------------------------------
// Transport parameters.

/// Server's preferred address advertised in transport parameters.
/// C: `tp_preferred_address_t`.  `is_defined` was an `int`
/// flag in C; promoted to `bool`.
// Field names mirror the camelCase identifiers from the C struct verbatim.
#[allow(non_snake_case)]
#[derive(Debug, Default, Copy, Clone)]
pub struct tp_preferred_address_t {
    pub is_defined: bool,
    pub ipv4Address: [u8; 4],
    pub ipv4Port: u16,
    pub ipv6Address: [u8; 16],
    pub ipv6Port: u16,
    pub connection_id: connection_id_t,
    pub statelessResetToken: [u8; 16],
}

/// Version negotiation TP payload.  C:
/// `tp_version_negotiation_t`.  The flexible-length
/// `received` and `supported` arrays use `Vec<u32>`; the explicit
/// `nb_received` / `nb_supported` length fields disappear (the
/// `Vec` carries its length).
#[derive(Debug, Default, Clone)]
pub struct tp_version_negotiation_t {
    /// Version found in TP, should match envelope.
    pub current: u32,
    /// Version that triggered a previous version negotiation.
    pub previous: u32,
    /// Versions received in a prior VN packet (client side only).
    pub received: Vec<u32>,
    /// Compatible versions supported by the peer (client side only).
    pub supported: Vec<u32>,
}

/// Full set of QUIC transport parameters carried during the
/// handshake.  C: `tp_t`.
///
/// `migration_disabled` was `unsigned int` in C, used as a Boolean
/// flag; promoted to `bool`.  Same for `do_grease_quic_bit`,
/// `enable_bdp_frame`, `is_reset_stream_at_enabled`.
/// `enable_loss_bit` and `enable_time_stamp` and
/// `address_discovery_mode` are kept as integers because callers
/// inspect the low bits separately ("want / can" flags).
#[derive(Debug, Default, Clone)]
pub struct tp_t {
    pub initial_max_stream_data_bidi_local: u64,
    pub initial_max_stream_data_bidi_remote: u64,
    pub initial_max_stream_data_uni: u64,
    pub initial_max_data: u64,
    pub initial_max_stream_id_bidir: u64,
    pub initial_max_stream_id_unidir: u64,
    pub max_idle_timeout: u64,
    pub max_packet_size: u32,
    /// Stored in microseconds for convenience.
    pub max_ack_delay: u32,
    pub active_connection_id_limit: u32,
    pub ack_delay_exponent: u8,
    pub migration_disabled: bool,
    pub preferred_address: tp_preferred_address_t,
    pub max_datagram_frame_size: u32,
    pub enable_loss_bit: i32,
    /// `(x & 1)` want, `(x & 2)` can.
    pub enable_time_stamp: i32,
    pub min_ack_delay: u64,
    pub do_grease_quic_bit: bool,
    pub version_negotiation: tp_version_negotiation_t,
    pub enable_bdp_frame: bool,
    pub initial_max_path_id: u64,
    /// `0`=none, `1`=provide-only, `2`=receive-only, `3`=both.
    pub address_discovery_mode: i32,
    pub is_reset_stream_at_enabled: bool,
}

// ---------------------------------------------------------------------------
// Stream-ID helpers.

pub const STREAM_ID_TYPE_MASK: u64 = 3;
pub const STREAM_ID_CLIENT_INITIATED: u64 = 0;
pub const STREAM_ID_SERVER_INITIATED: u64 = 1;
pub const STREAM_ID_BIDIR: u64 = 0;
pub const STREAM_ID_UNIDIR: u64 = 2;

pub const STREAM_ID_CLIENT_INITIATED_BIDIR: u64 = STREAM_ID_CLIENT_INITIATED | STREAM_ID_BIDIR;
pub const STREAM_ID_SERVER_INITIATED_BIDIR: u64 = STREAM_ID_SERVER_INITIATED | STREAM_ID_BIDIR;
pub const STREAM_ID_CLIENT_INITIATED_UNIDIR: u64 = STREAM_ID_CLIENT_INITIATED | STREAM_ID_UNIDIR;
pub const STREAM_ID_SERVER_INITIATED_UNIDIR: u64 = STREAM_ID_SERVER_INITIATED | STREAM_ID_UNIDIR;

pub const STREAM_ID_CLIENT_MAX_INITIAL_BIDIR: u64 =
    STREAM_ID_CLIENT_INITIATED_BIDIR + ((65535 - 1) * 4);
pub const STREAM_ID_SERVER_MAX_INITIAL_BIDIR: u64 =
    STREAM_ID_SERVER_INITIATED_BIDIR + ((65535 - 1) * 4);
pub const STREAM_ID_CLIENT_MAX_INITIAL_UNIDIR: u64 =
    STREAM_ID_CLIENT_INITIATED_UNIDIR + ((65535 - 1) * 4);
pub const STREAM_ID_SERVER_MAX_INITIAL_UNIDIR: u64 =
    STREAM_ID_SERVER_INITIATED_UNIDIR + ((65535 - 1) * 4);

/// C macro `IS_CLIENT_STREAM_ID(id)`.
pub const fn is_client_stream_id(id: u64) -> bool {
    (id & 1) == 0
}

/// C macro `IS_BIDIR_STREAM_ID(id)`.
pub const fn is_bidir_stream_id(id: u64) -> bool {
    (id & 2) == 0
}

// ---------------------------------------------------------------------------
// Time management.

/// C: `current_time`.  Returns wall-clock microseconds.
///
/// The C body reads the OS clock; in Rust this requires the `std`
/// feature (`std::time::SystemTime`).  The `cfg`-gated split lands
/// when the crate gains its `std` feature and the rest of the
/// `no_std + alloc` plumbing — for Phase 1 the function is just a
/// `todo!()` stub.
pub fn current_time() -> u64 {
    todo!()
}

/// C: `get_quic_time`.  Returns the virtual time used by
/// the QUIC context (wall-clock or simulated, depending on how the
/// context was created).
pub fn get_quic_time(_quic: &quic_t) -> u64 {
    todo!()
}

// ---------------------------------------------------------------------------
// Application callbacks (function-pointer typedefs → traits).

/// Stream/event callback installed on a QUIC context or connection.
/// Folds the C `void* callback_ctx` into the trait implementor's
/// state; `stream_ctx` is per-stream and remains a raw pointer
/// because it is produced and consumed by the application without
/// the stack interpreting it.
///
/// Returns the C `int` directly: `0` for success, or one of the
/// `_*` codes (e.g. `STREAM_RECEIVE_COMPLETE`) for
/// special signalling.  Phase 3 may refine to `Result` once the
/// crate-level error type lands.
///
/// `bytes` is `&[u8]` rather than `*const u8 + size_t`; events that
/// carry no payload pass `&[]`.
pub trait StreamDataCb {
    fn callback(
        &mut self,
        cnx: &mut cnx_t,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: call_back_event_t,
        stream_ctx: *mut c_void,
    ) -> i32;
}

/// ALPN-selection callback.  Returns the index of the chosen ALPN
/// in `list`, or any value `>= list.len()` to signal "none of the
/// proposed ALPNs is supported".  C: `AlpnSelect`.
pub trait AlpnSelect {
    fn select(&mut self, quic: &mut quic_t, list: &[ptls_iovec_t]) -> usize;
}

/// V2 ALPN-selection callback using `iovec_t` instead of
/// `ptls_iovec_t`.  C: `AlpnSelectV2`.
pub trait AlpnSelectV2 {
    fn select(&mut self, quic: &mut quic_t, list: &[iovec_t]) -> usize;
}

/// Callback that produces a server-environment-compatible CID.
/// Folds the C `void* cnx_id_cb_data` into the implementor's state.
/// C: `ConnectionIdCb`.
pub trait ConnectionIdCb {
    fn produce(
        &mut self,
        quic: &mut quic_t,
        cnx_id_local: connection_id_t,
        cnx_id_remote: connection_id_t,
    ) -> connection_id_t;
}

/// Packet-fuzzer callback.  Folds the C `void* fuzz_ctx` into the
/// implementor.  Returns the new packet length (which may equal
/// the input).  C: `Fuzz`.
pub trait Fuzz {
    fn fuzz(
        &mut self,
        cnx: &mut cnx_t,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32;
}

/// Forward declaration of `ptls_verify_certificate_t` from tls.
pub struct ptls_verify_certificate_t {
    _opaque: [u8; 0],
}

/// Signature-verification callback installed by a certificate
/// verifier.  C: `VerifySignCb`.  Returns 0 on
/// match.
pub trait VerifySignCb {
    fn verify(&mut self, data: &[u8], signature: &[u8]) -> i32;
}

/// Certificate-chain verification callback.  C:
/// `VerifyCertificateCb`.  Returns 0 when the chain
/// validates and populates `verify_sign` with a signature
/// verifier for subsequent handshake messages.
pub trait VerifyCertificateCb {
    fn verify(
        &mut self,
        cnx: &mut cnx_t,
        certs: &[ptls_iovec_t],
        verify_sign: &mut Option<Box<dyn VerifySignCb>>,
    ) -> i32;
}

/// Free hook for the verifier context.  C:
/// `FreeVerifyCertificateCtx`.  In Rust this normally
/// folds into `Drop`, but the trait is kept for source-level parity
/// with the C API surface.
pub trait FreeVerifyCertificateCtx {
    fn free(&mut self, ctx: &mut ptls_verify_certificate_t);
}

/// Direct-receive callback for streams marked with
/// `mark_direct_receive_stream`.  C:
/// `StreamDirectReceive`.  Folds `direct_receive_ctx`
/// into the implementor.
pub trait StreamDirectReceive {
    fn receive(
        &mut self,
        cnx: &mut cnx_t,
        stream_id: u64,
        fin: bool,
        bytes: &[u8],
        offset: u64,
    ) -> i32;
}

// ---------------------------------------------------------------------------
// Path quality and per-ack state.

/// Per-path quality snapshot reported by
/// `get_path_quality`.  C: `path_quality_t`.
#[derive(Debug, Default, Copy, Clone)]
pub struct path_quality_t {
    /// Receive rate estimate in bytes per second.
    pub receive_rate_estimate: u64,
    /// Pacing rate in bytes per second.
    pub pacing_rate: u64,
    /// Number of bytes in the congestion window.
    pub cwin: u64,
    /// Smoothed RTT estimate in microseconds.
    pub rtt: u64,
    /// Most recent RTT sample.
    pub rtt_sample: u64,
    /// Estimated RTT variability.
    pub rtt_variant: u64,
    /// Minimum observed RTT since path creation.
    pub rtt_min: u64,
    /// Maximum observed RTT since path creation.
    pub rtt_max: u64,
    pub sent: u64,
    pub lost: u64,
    pub timer_losses: u64,
    pub spurious_losses: u64,
    pub max_spurious_rtt: u64,
    pub max_reorder_delay: u64,
    pub max_reorder_gap: u64,
    pub bytes_in_transit: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

// ---------------------------------------------------------------------------
// Datagram and congestion APIs.

/// Datagram-readiness signalling for
/// `provide_datagram_buffer_ex`.  C:
/// `datagram_active_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum datagram_active_enum {
    #[default]
    datagram_not_active = 0,
    datagram_active_any_path = 1,
    datagram_active_this_path_only = 2,
    datagram_active_this_path_and_others = 3,
}

/// Default stream priority used when none is set explicitly.
pub const DEFAULT_STREAM_PRIORITY: u8 = 9;

/// Reference packet size used as the cautious upper bound when
/// queueing datagrams before path MTU is known.  In C this is
/// defined in terms of `ENFORCED_INITIAL_MTU`, which lives
/// in the internal header.  The numeric value (1252 bytes, the IPv4
/// initial MTU floor) is duplicated here to keep the public API
/// translatable in Phase 1 — the constant will be re-exported from
/// the internal module once it is translated.
pub const DATAGRAM_QUEUE_CAUTIOUS_LENGTH: usize = INITIAL_MTU_IPV4;

/// Congestion-control event fed to the algorithm callbacks.  C:
/// `congestion_notification_t`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum congestion_notification_t {
    congestion_notification_acknowledgement,
    congestion_notification_repeat,
    congestion_notification_timeout,
    congestion_notification_spurious_repeat,
    congestion_notification_rtt_measurement,
    congestion_notification_ecn_ec,
    congestion_notification_cwin_blocked,
    congestion_notification_seed_cwin,
    congestion_notification_reset,
    /// Notification of lost feedback.
    congestion_notification_lost_feedback,
}

/// Per-ACK state passed to the congestion-control algorithm.  C:
/// `per_ack_state_t`.
///
/// Single-bit `unsigned int : 1` bitfields collapse to `bool`.  `pc`
/// stays an `i32` (the C field used `int` instead of the enum
/// itself to avoid include dependencies; we follow suit so the
/// translation can be checked field-by-field).
#[derive(Debug, Default, Copy, Clone)]
pub struct per_ack_state_t {
    pub rtt_measurement: u64,
    pub send_delay: u64,
    pub one_way_delay: u64,
    pub nb_bytes_acknowledged: u64,
    pub nb_bytes_newly_lost: u64,
    pub nb_bytes_lost_since_packet_sent: u64,
    pub nb_bytes_delivered_since_packet_sent: u64,
    pub inflight_prior: u64,
    pub lost_packet_number: u64,
    pub lost_packet_sent_time: u64,
    pub pc: i32,
    pub is_app_limited: bool,
    pub is_cwnd_limited: bool,
}

/// Congestion-control algorithm vtable.  In C this is four
/// independent function-pointer typedefs grouped into a struct
/// (`congestion_algorithm_init`,
/// `_notify`, `_delete`, `_observe`).  The C side always installs
/// the four together, so they collapse to a single Rust trait per
/// the "trait grouping is a judgment call" rule.
///
/// `option_string` arrives as an optional borrowed `&str`; the C
/// version accepted a nullable `char const*` that callers either
/// owned for the duration of the call or set to `NULL`.
pub trait CongestionAlgorithm {
    fn alg_init(&self, path_x: &mut path_t, option_string: Option<&str>, current_time: u64);

    fn alg_notify(
        &self,
        cnx: &mut cnx_t,
        path_x: &mut path_t,
        notification: congestion_notification_t,
        ack_state: &per_ack_state_t,
        current_time: u64,
    );

    fn alg_delete(&self, path_x: &mut path_t);

    /// Optional observation hook — many algorithms leave this
    /// unimplemented (`NULL` in C).  Callers that don't care can
    /// rely on the default `None` return.
    fn alg_observe(&self, _path_x: &path_t) -> Option<(u64, u64)> {
        None
    }
}

/// Congestion-control algorithm descriptor.  C:
/// `congestion_algorithm_t`.
///
/// The four function pointers from the C struct fold into a single
/// trait-object reference; the algorithm identifier and numeric tag
/// stay alongside it.  `congestion_algorithm_id` is `&'static str`
/// because the C side stores compile-time string literals.
pub struct congestion_algorithm_t {
    pub congestion_algorithm_id: &'static str,
    pub congestion_algorithm_number: u8,
    pub ecn_mark: u8,
    pub algorithm: &'static dyn CongestionAlgorithm,
}

impl core::fmt::Debug for congestion_algorithm_t {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("congestion_algorithm_t")
            .field("congestion_algorithm_id", &self.congestion_algorithm_id)
            .field(
                "congestion_algorithm_number",
                &self.congestion_algorithm_number,
            )
            .field("ecn_mark", &self.ecn_mark)
            .finish_non_exhaustive()
    }
}

/// Return the registered congestion-control algorithm table.  In
/// C this was a pair of `extern` globals
/// (`congestion_control_algorithms` and
/// `nb_congestion_control_algorithms`); the Rust API
/// exposes a single accessor returning a borrowed slice — the
/// length is implicit.  The slice is empty until
/// [`register_congestion_control_algorithms`] (or its
/// `_all_` convenience wrapper) is called.
pub fn congestion_control_algorithms() -> &'static [&'static congestion_algorithm_t] {
    todo!()
}

// ---------------------------------------------------------------------------
// ALPN list.

/// Application-protocol identifiers used during session
/// negotiation.  C: `alpn_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum alpn_enum {
    #[default]
    alpn_undef = 0,
    alpn_http_0_9,
    alpn_http_3,
    alpn_quicperf,
}

/// One entry in the ALPN dispatch table.  C:
/// `alpn_list_t`.  `len` is implicit in the byte slice but
/// kept as a separate field for source-level parity with the C
/// struct; Phase 3 may collapse to a single `&'static [u8]` slice.
#[derive(Debug, Copy, Clone)]
pub struct alpn_list_t {
    pub alpn_code: alpn_enum,
    pub alpn_val: &'static str,
    pub len: usize,
}

// ---------------------------------------------------------------------------
// MTU helper.

/// C macro `MTU_OVERHEAD(p_s_addr)`.  Returns the IP+UDP
/// overhead for the address family of `addr`: 28 bytes for IPv4
/// (20 IP + 8 UDP), 48 bytes for IPv6 (40 IP + 8 UDP).
pub const fn mtu_overhead(addr: &SocketAddr) -> u32 {
    match addr {
        SocketAddr::V4(_) => 28,
        SocketAddr::V6(_) => 48,
    }
}

// ---------------------------------------------------------------------------
// Free functions — handshake and error helpers.

/// C: `is_handshake_error`.  Returns `true` when the given
/// error code is a TLS handshake error.
pub fn is_handshake_error(_error_code: u64) -> bool {
    todo!()
}

/// C: `error_name`.  Returns a short textual name for
/// `error_code`, or `None` if unrecognised.
pub fn error_name(_error_code: u64) -> Option<&'static str> {
    todo!()
}

/// C: `tp_name`.  Returns a textual name for transport
/// parameter `tp_number`.  Takes a raw `u64` (rather than `Tp`)
/// because the C function answers for unknown / extension IDs too.
pub fn tp_name(_tp_number: u64) -> Option<&'static str> {
    todo!()
}

/// C: `frame_name`.  Returns a textual name for frame type
/// `frame_type`.
pub fn frame_name(_frame_type: u64) -> Option<&'static str> {
    todo!()
}

/// C: `add_proposed_alpn`.  Provision an ALPN context
/// during the TLS callback.  `tls_context` is the opaque pointer
/// the TLS stack passed to the application callback.
pub fn add_proposed_alpn(_tls_context: *mut c_void, _alpn: &str) -> Result<(), ()> {
    todo!()
}

/// C: `tls_get_negotiated_alpn`.  Returns the negotiated
/// ALPN value as a borrowed string, or `None` when none was
/// selected.
pub fn tls_get_negotiated_alpn(_cnx: &cnx_t) -> Option<&str> {
    todo!()
}

/// C: `tls_get_sni`.  Returns the SNI value the peer
/// presented during the handshake, or `None` when none was given.
pub fn tls_get_sni(_cnx: &cnx_t) -> Option<&str> {
    todo!()
}

/// C: `set_fuzz`.  Install a per-context packet fuzzer.
/// `None` removes the fuzzer.
pub fn set_fuzz(_quic: &mut quic_t, _fuzzer: Option<Box<dyn Fuzz>>) {
    todo!()
}

// ---------------------------------------------------------------------------
// Logging and diagnostics.

/// C: `log_app_message` (and its `_v` variadic twin).
/// Both C entry points are folded into one Rust function taking
/// `core::fmt::Arguments<'_>`; the variadic flavour is redundant in
/// Rust because formatting is done at the call site via the
/// `format_args!` macro.
pub fn log_app_message(_cnx: &mut cnx_t, _args: core::fmt::Arguments<'_>) {
    todo!()
}

/// C: `set_log_level`.  `1` → log every packet, `0` →
/// log only the first 100 packets per connection.
pub fn set_log_level(_quic: &mut quic_t, _log_level: i32) {
    todo!()
}

/// C: `use_unique_log_names`.  Toggles randomised
/// log-file names (defeating accidental collisions when clients
/// pick non-random initial CIDs).
pub fn use_unique_log_names(_quic: &mut quic_t, _use_unique_log_names: bool) {
    todo!()
}

/// C: `enable_sslkeylog`.
///
/// Phase 1 follows the canonical build (`WITHOUT_SSLKEYLOG`
/// undefined); a `cfg`-gated variant lands when build options are
/// translated.
pub fn enable_sslkeylog(_quic: &mut quic_t, _enable_sslkeylog: bool) {
    todo!()
}

/// C: `is_sslkeylog_enabled`.
pub fn is_sslkeylog_enabled(_quic: &quic_t) -> bool {
    todo!()
}

/// C: `set_random_initial`.  Values are `0` (no
/// randomisation), `1` (only Initial PNs) or `2` (all PN spaces).
pub fn set_random_initial(_quic: &mut quic_t, _random_initial: i32) {
    todo!()
}

/// C: `set_packet_train_mode`.
pub fn set_packet_train_mode(_quic: &mut quic_t, _train_mode: bool) {
    todo!()
}

/// C: `set_padding_policy`.
pub fn set_padding_policy(_quic: &mut quic_t, _padding_min_size: u32, _padding_multiple: u32) {
    todo!()
}

/// C: `set_key_log_file`.  `None` clears the keylog file.
pub fn set_key_log_file(_quic: &mut quic_t, _keylog_filename: Option<&str>) {
    todo!()
}

// ---------------------------------------------------------------------------
// Connection-pool sizing and stats.

/// C: `adjust_max_connections`.  Cannot grow past the
/// limit chosen at context creation.
pub fn adjust_max_connections(_quic: &mut quic_t, _max_nb_connections: u32) -> Result<(), ()> {
    todo!()
}

pub fn current_number_connections(_quic: &quic_t) -> u32 {
    todo!()
}

pub fn set_max_half_open_retry_threshold(_quic: &mut quic_t, _max_half_open_before_retry: u32) {
    todo!()
}

pub fn get_max_half_open_retry_threshold(_quic: &quic_t) -> u32 {
    todo!()
}

/// Fan-out version of "why was this connection closed?".  C:
/// `get_close_reasons`.  The four `uint64_t*` output
/// parameters fold into a single returned tuple of `(local_reason,
/// remote_reason, local_application_reason,
/// remote_application_reason)`.
pub fn get_close_reasons(_cnx: &cnx_t) -> (u64, u64, u64, u64) {
    todo!()
}

// ---------------------------------------------------------------------------
// Port-blocking and address helpers.

pub fn check_port_blocked(_port: u16) -> bool {
    todo!()
}

pub fn check_addr_blocked(_addr_from: &SocketAddr) -> bool {
    todo!()
}

pub fn disable_port_blocking(_quic: &mut quic_t, _is_port_blocking_disabled: bool) {
    todo!()
}

// ---------------------------------------------------------------------------
// QUIC context create / dispose.

/// C: `create`.  Builds a QUIC context with the supplied
/// certificate paths, default callbacks, and reset seed.
///
/// Pointer-shape choices, derived from `quicctx.c:634` and
/// `sockloop.c:2009`:
///
/// * Every `char const*` parameter is `Option<&str>` (the C source
///   passes `NULL` to mean "absent" — see the sockloop call site).
/// * `default_callback_fn` + `default_callback_ctx` collapse to one
///   `Option<Box<dyn StreamDataCb>>`.
/// * `cnx_id_callback` + `cnx_id_callback_data` collapse to
///   `Option<Box<dyn ConnectionIdCb>>`.
/// * `reset_seed[16]` is `[u8; RESET_SECRET_SIZE]` taken by
///   value (the C body deep-copies it into `quic->reset_seed`).
/// * `p_simulated_time: *mut u64` becomes `Option<&'a mut u64>`;
///   the QUIC context retains the borrow across calls.
/// * `ticket_encryption_key` + `ticket_encryption_key_length`
///   collapse to a borrowed `Option<&[u8]>`.
///
/// Returns `None` when context creation fails (the C side returned
/// `NULL`).
pub fn create(
    _max_nb_connections: u32,
    _cert_file_name: Option<&str>,
    _key_file_name: Option<&str>,
    _cert_root_file_name: Option<&str>,
    _default_alpn: Option<&str>,
    _default_callback: Option<Box<dyn StreamDataCb>>,
    _cnx_id_callback: Option<Box<dyn ConnectionIdCb>>,
    _reset_seed: [u8; RESET_SECRET_SIZE],
    _current_time: u64,
    _p_simulated_time: Option<&mut u64>,
    _ticket_file_name: Option<&str>,
    _ticket_encryption_key: Option<&[u8]>,
) -> Option<Box<quic_t>> {
    todo!()
}

/// C: `free`.  The Rust translation simply consumes the
/// `Box<quic_t>` so its contents are dropped at end of
/// scope, mirroring the C `free(quic)` call.
#[allow(clippy::boxed_local)]
pub fn free(_quic: Box<quic_t>) {
    todo!()
}

pub fn set_low_memory_mode(_quic: &mut quic_t, _low_memory_mode: bool) -> Result<(), ()> {
    todo!()
}

pub fn set_cookie_mode(_quic: &mut quic_t, _cookie_mode: i32) {
    todo!()
}

pub fn set_cipher_suite(_quic: &mut quic_t, _cipher_suite_id: u16) -> Result<(), ()> {
    todo!()
}

pub fn set_key_exchange(_quic: &mut quic_t, _key_exchange_id: u16) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Default and per-connection transport parameters.

pub fn set_default_tp(_quic: &mut quic_t, _tp: &tp_t) -> Result<(), ()> {
    todo!()
}

pub fn get_default_tp(_quic: &quic_t) -> &tp_t {
    todo!()
}

pub fn set_default_tp_value(_quic: &mut quic_t, _tp_type: u64, _tp_value: u64) -> Result<(), ()> {
    todo!()
}

pub fn set_transport_parameters(_cnx: &mut cnx_t, _tp: &tp_t) {
    todo!()
}

pub fn get_transport_parameters(_cnx: &cnx_t, _get_local: bool) -> &tp_t {
    todo!()
}

// ---------------------------------------------------------------------------
// TLS configuration.

/// C: `set_tls_certificate_chain`.  The QUIC context takes
/// ownership of the certs vector, so it is consumed by value
/// (`Vec<ptls_iovec_t>`).
pub fn set_tls_certificate_chain(_quic: &mut quic_t, _certs: Vec<ptls_iovec_t>) {
    todo!()
}

/// C: `set_tls_root_certificates`.  See above for
/// ownership.  The C `int` return distinguishes load vs. store
/// failures (`-1` and `-2`); Phase 1 collapses both to `Err(())`
/// pending the crate-level error type.
pub fn set_tls_root_certificates(_quic: &mut quic_t, _certs: Vec<ptls_iovec_t>) -> Result<(), ()> {
    todo!()
}

pub fn set_null_verifier(_quic: &mut quic_t) {
    todo!()
}

/// C: `set_tls_key`.  Caller retains ownership of the key
/// buffer; we copy on the way in.
pub fn set_tls_key(_quic: &mut quic_t, _key: &[u8]) -> Result<(), ()> {
    todo!()
}

/// C: `set_verify_certificate_callback`.  The verifier
/// context is owned by the QUIC context after this call (the C side
/// stashes the pointer and later runs `free_fn` on it).  Passing it
/// as `Box<ptls_verify_certificate_t>` makes the ownership transfer
/// explicit; clippy's `boxed_local` is silenced because the Box is
/// the contract, not the implementation.
#[allow(clippy::boxed_local)]
pub fn set_verify_certificate_callback(
    _quic: &mut quic_t,
    _cb: Box<ptls_verify_certificate_t>,
    _free_fn: Box<dyn FreeVerifyCertificateCtx>,
) {
    todo!()
}

pub fn set_client_authentication(_quic: &mut quic_t, _client_authentication: bool) {
    todo!()
}

pub fn set_use_exporter(_quic: &mut quic_t, _use_exporter: bool) {
    todo!()
}

/// C: `export_secret`.  Writes exported keying material
/// into `out` and returns the number of bytes written.
pub fn export_secret(_cnx: &mut cnx_t, _label: &str, _out: &mut [u8]) -> Result<usize, ()> {
    todo!()
}

pub fn enforce_client_only(_quic: &mut quic_t, _do_enforce: bool) {
    todo!()
}

// ---------------------------------------------------------------------------
// Default policies on the QUIC context.

pub fn set_default_padding(_quic: &mut quic_t, _padding_multiple: u32, _padding_minsize: u32) {
    todo!()
}

pub fn set_default_spinbit_policy(
    _quic: &mut quic_t,
    _default_spinbit_policy: spinbit_version_enum,
) -> Result<(), ()> {
    todo!()
}

pub fn set_spinbit_policy(
    _cnx: &mut cnx_t,
    _spinbit_policy: spinbit_version_enum,
) -> Result<(), ()> {
    todo!()
}

pub fn set_default_lossbit_policy(
    _quic: &mut quic_t,
    _default_lossbit_policy: lossbit_version_enum,
) {
    todo!()
}

pub fn set_default_multipath_option(_quic: &mut quic_t, _multipath_option: i32) {
    todo!()
}

pub fn set_default_address_discovery_mode(_quic: &mut quic_t, _mode: i32) {
    todo!()
}

pub fn set_cwin_max(_quic: &mut quic_t, _cwin_max: u64) {
    todo!()
}

pub fn set_max_data_control(_quic: &mut quic_t, _max_data: u64) {
    todo!()
}

pub fn set_default_idle_timeout(_quic: &mut quic_t, _idle_timeout_ms: u64) {
    todo!()
}

pub fn set_default_handshake_timeout(_quic: &mut quic_t, _handshake_timeout_us: u64) {
    todo!()
}

pub fn set_default_crypto_epoch_length(_quic: &mut quic_t, _crypto_epoch_length_max: u64) {
    todo!()
}

pub fn get_default_crypto_epoch_length(_quic: &quic_t) -> u64 {
    todo!()
}

pub fn get_local_cid_length(_quic: &quic_t) -> u8 {
    todo!()
}

pub fn is_local_cid(_quic: &quic_t, _cid: &connection_id_t) -> bool {
    todo!()
}

// Session-ticket and retry-token persistence.

pub fn load_retry_tokens(_quic: &mut quic_t, _token_store_filename: &str) -> Result<(), ()> {
    todo!()
}

pub fn save_session_tickets(_quic: &mut quic_t, _ticket_store_filename: &str) -> Result<(), ()> {
    todo!()
}

pub fn save_retry_tokens(_quic: &mut quic_t, _token_store_filename: &str) -> Result<(), ()> {
    todo!()
}

pub fn set_default_bdp_frame_option(_quic: &mut quic_t, _enable_bdp_frame: bool) {
    todo!()
}

pub fn set_default_connection_id_length(_quic: &mut quic_t, _cid_length: u8) -> Result<(), ()> {
    todo!()
}

pub fn set_default_connection_id_ttl(_quic: &mut quic_t, _ttl_usec: u64) {
    todo!()
}

pub fn get_default_connection_id_ttl(_quic: &quic_t) -> u64 {
    todo!()
}

pub fn set_mtu_max(_quic: &mut quic_t, _mtu_max: u32) {
    todo!()
}

/// C: `set_alpn_select_fn`.
pub fn set_alpn_select_fn(_quic: &mut quic_t, _alpn_select_fn: Option<Box<dyn AlpnSelect>>) {
    todo!()
}

/// C: `set_alpn_select_fn_v2`.
pub fn set_alpn_select_fn_v2(_quic: &mut quic_t, _alpn_select_fn: Option<Box<dyn AlpnSelectV2>>) {
    todo!()
}

/// C: `set_default_callback`.  The combined `(callback_fn,
/// callback_ctx)` pair from C folds into a single trait object.
pub fn set_default_callback(_quic: &mut quic_t, _callback: Option<Box<dyn StreamDataCb>>) {
    todo!()
}

pub fn set_default_stateless_reset_min_interval(_quic: &mut quic_t, _min_interval_usec: u64) {
    todo!()
}

pub fn set_max_simultaneous_logs(_quic: &mut quic_t, _max_simultaneous_logs: u32) {
    todo!()
}

pub fn get_max_simultaneous_logs(_quic: &quic_t) -> u32 {
    todo!()
}

// ---------------------------------------------------------------------------
// Connection lifecycle.

/// C: `create_cnx`.  `client_mode` was a `char` flag in C;
/// promoted to `bool`.
///
/// The returned `&mut cnx_t` borrows from `quic` because
/// the C side stores the new connection in the context's hash
/// tables and the application accesses it through the same context.
pub fn create_cnx<'a>(
    _quic: &'a mut quic_t,
    _initial_cnx_id: connection_id_t,
    _remote_cnx_id: connection_id_t,
    _addr_to: Option<&SocketAddr>,
    _start_time: u64,
    _preferred_version: u32,
    _sni: Option<&str>,
    _alpn: Option<&str>,
    _client_mode: bool,
) -> Option<&'a mut cnx_t> {
    todo!()
}

/// C: `create_client_cnx`.  Convenience wrapper around
/// [`create_cnx`] for the client side; `addr` is required.
pub fn create_client_cnx<'a>(
    _quic: &'a mut quic_t,
    _addr: &SocketAddr,
    _start_time: u64,
    _preferred_version: u32,
    _sni: Option<&str>,
    _alpn: Option<&str>,
    _callback: Option<Box<dyn StreamDataCb>>,
) -> Option<&'a mut cnx_t> {
    todo!()
}

pub fn start_client_cnx(_cnx: &mut cnx_t) -> Result<(), ()> {
    todo!()
}

/// C: `close`.  Begin an ordered close.
pub fn close(_cnx: &mut cnx_t, _application_reason_code: u64) -> Result<(), ()> {
    todo!()
}

/// C: `close_ex`.  Same as [`close`] but carries
/// a textual `error_reason`.  `None` matches the C `NULL` case.
pub fn close_ex(
    _cnx: &mut cnx_t,
    _application_reason_code: u64,
    _error_reason: Option<&str>,
) -> Result<(), ()> {
    todo!()
}

pub fn close_immediate(_cnx: &mut cnx_t) {
    todo!()
}

/// C: `delete_cnx`.  Consumes the connection so its
/// resources drop at end of scope.
pub fn delete_cnx(_cnx: &mut cnx_t) {
    todo!()
}

pub fn set_app_wake_time(_cnx: &mut cnx_t, _app_wake_time: u64) {
    todo!()
}

pub fn set_desired_version(_cnx: &mut cnx_t, _desired_version: u32) {
    todo!()
}

pub fn set_rejected_version(_cnx: &mut cnx_t, _rejected_version: u32) {
    todo!()
}

// ---------------------------------------------------------------------------
// Path management.

pub fn probe_new_path(
    _cnx: &mut cnx_t,
    _addr_peer: &SocketAddr,
    _addr_local: &SocketAddr,
    _current_time: u64,
) -> Result<(), ()> {
    todo!()
}

pub fn probe_new_path_ex(
    _cnx: &mut cnx_t,
    _addr_peer: &SocketAddr,
    _addr_local: &SocketAddr,
    _if_index: i32,
    _current_time: u64,
    _to_preferred_address: bool,
) -> Result<(), ()> {
    todo!()
}

pub fn probe_new_tuple(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _addr_peer: &SocketAddr,
    _addr_local: &SocketAddr,
    _if_index: i32,
    _current_time: u64,
    _to_preferred_address: bool,
) -> Result<(), ()> {
    todo!()
}

pub fn enable_path_callbacks(_cnx: &mut cnx_t, _are_enabled: bool) {
    todo!()
}

pub fn enable_path_callbacks_default(_quic: &mut quic_t, _are_enabled: bool) {
    todo!()
}

/// C: `set_app_path_ctx`.  `app_path_ctx` is opaque
/// application data and stays a raw pointer.
pub fn set_app_path_ctx(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _app_path_ctx: *mut c_void,
) -> Result<(), ()> {
    todo!()
}

pub fn abandon_path(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _reason: u64,
    _current_time: u64,
) -> Result<(), ()> {
    todo!()
}

pub fn refresh_path_connection_id(_cnx: &mut cnx_t, _unique_path_id: u64) -> Result<(), ()> {
    todo!()
}

pub fn set_stream_path_affinity(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _unique_path_id: u64,
) -> Result<(), ()> {
    todo!()
}

pub fn set_path_status(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _status: path_status_enum,
) -> Result<(), ()> {
    todo!()
}

/// C: `subscribe_new_path_allowed`.  The C signature took
/// `int* is_already_allowed` as both an output and a status flag;
/// the Rust shape returns `Ok(true)` if a new path is already
/// allowed (caller can proceed immediately), `Ok(false)` if the
/// caller will be notified later by callback, and `Err(())` on
/// error.
pub fn subscribe_new_path_allowed(_cnx: &mut cnx_t) -> Result<bool, ()> {
    todo!()
}

pub fn set_first_if_index(_cnx: &mut cnx_t, _if_index: u32) -> Result<(), ()> {
    todo!()
}

/// C: `get_path_addr`.  The C `int local` argument selects
/// which address to return: `1` = local, `2` = peer, `3` = peer's
/// observed.  Output `struct sockaddr_storage*` folds into the
/// returned `SocketAddr`.
pub fn get_path_addr(_cnx: &cnx_t, _unique_path_id: u64, _local: i32) -> Result<SocketAddr, ()> {
    todo!()
}

pub fn get_path_quality(_cnx: &cnx_t, _unique_path_id: u64) -> Result<path_quality_t, ()> {
    todo!()
}

pub fn get_default_path_quality(_cnx: &cnx_t) -> path_quality_t {
    todo!()
}

pub fn subscribe_to_quality_update_per_path(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _pacing_rate_delta: u64,
    _rtt_delta: u64,
) -> Result<(), ()> {
    todo!()
}

pub fn subscribe_to_quality_update(_cnx: &mut cnx_t, _pacing_rate_delta: u64, _rtt_delta: u64) {
    todo!()
}

pub fn default_quality_update(_quic: &mut quic_t, _pacing_rate_delta: u64, _rtt_delta: u64) {
    todo!()
}

// ---------------------------------------------------------------------------
// Connection iteration and timing.

pub fn start_key_rotation(_cnx: &mut cnx_t) -> Result<(), ()> {
    todo!()
}

pub fn get_quic_ctx(_cnx: &mut cnx_t) -> &mut quic_t {
    todo!()
}

pub fn get_first_cnx(_quic: &mut quic_t) -> Option<&mut cnx_t> {
    todo!()
}

pub fn get_next_cnx(_cnx: &mut cnx_t) -> Option<&mut cnx_t> {
    todo!()
}

pub fn get_next_wake_delay(_quic: &quic_t, _current_time: u64, _delay_max: i64) -> i64 {
    todo!()
}

pub fn get_wake_delay(_cnx: &cnx_t, _current_time: u64, _delay_max: i64) -> i64 {
    todo!()
}

pub fn get_earliest_cnx_to_wake(_quic: &mut quic_t, _max_wake_time: u64) -> Option<&mut cnx_t> {
    todo!()
}

pub fn get_next_wake_time(_quic: &quic_t, _current_time: u64) -> u64 {
    todo!()
}

pub fn get_cnx_state(_cnx: &cnx_t) -> state_enum {
    todo!()
}

pub fn get_cnx_in_progress(_quic: &mut quic_t) -> Option<&mut cnx_t> {
    todo!()
}

pub fn cnx_set_padding_policy(_cnx: &mut cnx_t, _padding_multiple: u32, _padding_minsize: u32) {
    todo!()
}

/// C: `cnx_get_padding_policy`.  Two `uint32_t*` output
/// parameters fold into a returned tuple `(padding_multiple,
/// padding_minsize)`.
pub fn cnx_get_padding_policy(_cnx: &cnx_t) -> (u32, u32) {
    todo!()
}

pub fn cnx_set_spinbit_policy(_cnx: &mut cnx_t, _spinbit_policy: spinbit_version_enum) {
    todo!()
}

pub fn set_crypto_epoch_length(_cnx: &mut cnx_t, _crypto_epoch_length_max: u64) {
    todo!()
}

pub fn get_crypto_epoch_length(_cnx: &cnx_t) -> u64 {
    todo!()
}

pub fn set_default_pmtud_policy(_quic: &mut quic_t, _pmtud_policy: pmtud_policy_enum) {
    todo!()
}

pub fn cnx_set_pmtud_policy(_cnx: &mut cnx_t, _pmtud_policy: pmtud_policy_enum) {
    todo!()
}

/// Obsolete: prefer [`cnx_set_pmtud_policy`].  Kept for
/// source-level parity with the C API.
pub fn cnx_set_pmtud_required(_cnx: &mut cnx_t, _is_pmtud_required: bool) {
    todo!()
}

pub fn tls_is_psk_handshake(_cnx: &cnx_t) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------
// Address helpers.

/// C: `get_peer_addr`.  The C side returned an aliasing
/// `struct sockaddr*` into internal storage; the Rust translation
/// returns `SocketAddr` by value.
pub fn get_peer_addr(_cnx: &cnx_t) -> SocketAddr {
    todo!()
}

/// C: `get_local_addr`.  Same shape as
/// [`get_peer_addr`].
pub fn get_local_addr(_cnx: &cnx_t) -> SocketAddr {
    todo!()
}

pub fn get_local_if_index(_cnx: &cnx_t) -> u32 {
    todo!()
}

pub fn set_local_addr(_cnx: &mut cnx_t, _addr: &SocketAddr) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Connection ID accessors.

pub fn get_local_cnxid(_cnx: &cnx_t) -> connection_id_t {
    todo!()
}

pub fn get_remote_cnxid(_cnx: &cnx_t) -> connection_id_t {
    todo!()
}

pub fn get_initial_cnxid(_cnx: &cnx_t) -> connection_id_t {
    todo!()
}

pub fn get_client_cnxid(_cnx: &cnx_t) -> connection_id_t {
    todo!()
}

pub fn get_server_cnxid(_cnx: &cnx_t) -> connection_id_t {
    todo!()
}

pub fn get_logging_cnxid(_cnx: &cnx_t) -> connection_id_t {
    todo!()
}

// ---------------------------------------------------------------------------
// Connection-level state.

pub fn get_cnx_start_time(_cnx: &cnx_t) -> u64 {
    todo!()
}

pub fn is_0rtt_available(_cnx: &cnx_t) -> bool {
    todo!()
}

pub fn is_cnx_backlog_empty(_cnx: &cnx_t) -> bool {
    todo!()
}

/// C: `set_callback`.  See [`set_default_callback`]
/// for the (`callback_fn`, `callback_ctx`) → trait-object collapse.
pub fn set_callback(_cnx: &mut cnx_t, _callback: Option<Box<dyn StreamDataCb>>) {
    todo!()
}

pub fn get_default_callback_function(_quic: &quic_t) -> Option<&dyn StreamDataCb> {
    todo!()
}

/// C: `get_default_callback_context`.  In the trait
/// translation the "context" is the trait object's state; this
/// accessor returns the same trait reference as
/// [`get_default_callback_function`].  The C twin is kept
/// as a separate API for source-level parity.
pub fn get_default_callback_context(_quic: &quic_t) -> Option<&dyn StreamDataCb> {
    todo!()
}

pub fn get_callback_function(_cnx: &cnx_t) -> Option<&dyn StreamDataCb> {
    todo!()
}

pub fn get_callback_context(_cnx: &cnx_t) -> Option<&dyn StreamDataCb> {
    todo!()
}

// ---------------------------------------------------------------------------
// Frame queueing.

pub fn queue_misc_frame(
    _cnx: &mut cnx_t,
    _bytes: &[u8],
    _is_pure_ack: bool,
    _pc: packet_context_enum,
) -> Result<(), ()> {
    todo!()
}

pub fn queue_datagram_frame(_cnx: &mut cnx_t, _bytes: &[u8]) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Packet I/O.

/// C: `incoming_packet`.  `addr_from` and `addr_to` are
/// borrowed for the duration of the call.  `received_ecn` carries
/// the IP-level ECN code-point byte.
pub fn incoming_packet(
    _quic: &mut quic_t,
    _bytes: &mut [u8],
    _addr_from: &SocketAddr,
    _addr_to: &SocketAddr,
    _if_index_to: i32,
    _received_ecn: u8,
    _current_time: u64,
) -> Result<(), ()> {
    todo!()
}

/// C: `incoming_packet_ex`.  Same as
/// [`incoming_packet`] but additionally identifies the
/// connection that consumed the packet.
pub fn incoming_packet_ex<'a>(
    _quic: &'a mut quic_t,
    _bytes: &mut [u8],
    _addr_from: &SocketAddr,
    _addr_to: &SocketAddr,
    _if_index_to: i32,
    _received_ecn: u8,
    _current_time: u64,
) -> Result<Option<&'a mut cnx_t>, ()> {
    todo!()
}

/// Result of preparing a packet for transmission.  Folds the C
/// out-parameter pile (`send_length`, `p_addr_to`, `p_addr_from`,
/// `if_index`, `log_cid`, `send_msg_size`) into a single struct.
#[derive(Debug)]
pub struct PreparedPacket<'a> {
    /// Number of bytes written into `send_buffer`.
    pub send_length: usize,
    /// Destination address.
    pub addr_to: SocketAddr,
    /// Source address.
    pub addr_from: SocketAddr,
    pub if_index: i32,
    /// Connection ID that should be logged for this packet.
    pub log_cid: connection_id_t,
    /// Connection that produced the packet (the C
    /// `p_last_cnx`).  `None` when the QUIC context had no work to
    /// do.
    pub last_cnx: Option<&'a mut cnx_t>,
    /// Optional GSO segment size when the packet is a coalesced
    /// train; `None` for a single-packet send.
    pub send_msg_size: Option<usize>,
}

/// C: `prepare_next_packet_ex`.  Folds the seven
/// out-parameters of the C signature into a [`PreparedPacket`].
pub fn prepare_next_packet_ex<'a>(
    _quic: &'a mut quic_t,
    _current_time: u64,
    _send_buffer: &mut [u8],
) -> Result<PreparedPacket<'a>, ()> {
    todo!()
}

/// C: `prepare_next_packet`.  Same shape as
/// [`prepare_next_packet_ex`] but without GSO segment
/// reporting.
pub fn prepare_next_packet<'a>(
    _quic: &'a mut quic_t,
    _current_time: u64,
    _send_buffer: &mut [u8],
) -> Result<PreparedPacket<'a>, ()> {
    todo!()
}

/// Result of preparing a single connection's packet.  Subset of
/// [`PreparedPacket`] that drops the cross-connection fields.
#[derive(Debug)]
pub struct PreparedCnxPacket {
    pub send_length: usize,
    pub addr_to: SocketAddr,
    pub addr_from: SocketAddr,
    pub if_index: i32,
    pub send_msg_size: Option<usize>,
}

pub fn prepare_packet_ex(
    _cnx: &mut cnx_t,
    _current_time: u64,
    _send_buffer: &mut [u8],
) -> Result<PreparedCnxPacket, ()> {
    todo!()
}

pub fn prepare_packet(
    _cnx: &mut cnx_t,
    _current_time: u64,
    _send_buffer: &mut [u8],
) -> Result<PreparedCnxPacket, ()> {
    todo!()
}

pub fn notify_destination_unreachable(
    _cnx: &mut cnx_t,
    _current_time: u64,
    _addr_peer: &SocketAddr,
    _addr_local: &SocketAddr,
    _if_index: i32,
    _socket_err: i32,
) {
    todo!()
}

pub fn notify_destination_unreachable_by_cnxid(
    _quic: &mut quic_t,
    _cnxid: &connection_id_t,
    _current_time: u64,
    _addr_peer: &SocketAddr,
    _addr_local: &SocketAddr,
    _if_index: i32,
    _socket_err: i32,
) {
    todo!()
}

// ---------------------------------------------------------------------------
// Streams.

pub fn mark_direct_receive_stream(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _direct_receive: Box<dyn StreamDirectReceive>,
) -> Result<(), ()> {
    todo!()
}

/// C: `set_app_stream_ctx`.  `app_stream_ctx` is opaque
/// application data, kept as a raw pointer.
pub fn set_app_stream_ctx(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _app_stream_ctx: *mut c_void,
) -> Result<(), ()> {
    todo!()
}

pub fn unlink_app_stream_ctx(_cnx: &mut cnx_t, _stream_id: u64) {
    todo!()
}

/// C: `mark_active_stream`.  `is_active` was an `int`
/// flag in C; promoted to `bool`.
pub fn mark_active_stream(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _is_active: bool,
    _v_stream_ctx: *mut c_void,
) -> Result<(), ()> {
    todo!()
}

pub fn set_stream_not_coalesced(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _is_not_coalesced: bool,
) -> Result<(), ()> {
    todo!()
}

pub fn set_default_priority(_quic: &mut quic_t, _default_stream_priority: u8) {
    todo!()
}

pub fn set_stream_priority(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _stream_priority: u8,
) -> Result<(), ()> {
    todo!()
}

pub fn mark_high_priority_stream(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _is_high_priority: bool,
) -> Result<(), ()> {
    todo!()
}

pub fn set_default_datagram_priority(_quic: &mut quic_t, _default_datagram_priority: u8) {
    todo!()
}

pub fn set_datagram_priority(_cnx: &mut cnx_t, _datagram_priority: u8) {
    todo!()
}

/// C: `provide_stream_data_buffer`.  `context` is the
/// opaque stack-supplied pointer the application receives in the
/// `callback_prepare_to_send` callback.  Returns the
/// borrowed buffer the application should fill, or `None` on
/// error.  The returned slice borrows from the stack-internal
/// packet-build buffer; the lifetime here is `'static` only as a
/// Phase 1 stand-in — Phase 3 will tie it to the callback frame
/// lifetime, possibly via a wrapper handle type.
pub fn provide_stream_data_buffer(
    _context: *mut c_void,
    _nb_bytes: usize,
    _is_fin: bool,
    _is_still_active: bool,
) -> Option<&'static mut [u8]> {
    todo!()
}

pub fn add_to_stream(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _data: &[u8],
    _set_fin: bool,
) -> Result<(), ()> {
    todo!()
}

pub fn reset_stream_ctx(_cnx: &mut cnx_t, _stream_id: u64) {
    todo!()
}

pub fn add_to_stream_with_ctx(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _data: &[u8],
    _set_fin: bool,
    _app_stream_ctx: *mut c_void,
) -> Result<(), ()> {
    todo!()
}

pub fn reset_stream(_cnx: &mut cnx_t, _stream_id: u64, _local_stream_error: u64) -> Result<(), ()> {
    todo!()
}

pub fn reset_stream_at(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _local_stream_error: u64,
    _reliable_size: u64,
) -> Result<(), ()> {
    todo!()
}

pub fn open_flow_control(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _expected_data_size: u64,
) -> Result<(), ()> {
    todo!()
}

pub fn set_app_flow_control(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _use_app_flow_control: bool,
) -> Result<(), ()> {
    todo!()
}

pub fn get_next_local_stream_id(_cnx: &mut cnx_t, _is_unidir: bool) -> u64 {
    todo!()
}

pub fn stop_sending(_cnx: &mut cnx_t, _stream_id: u64, _local_stream_error: u64) -> Result<(), ()> {
    todo!()
}

pub fn discard_stream(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _local_stream_error: u16,
) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Datagrams.

pub fn mark_datagram_ready(_cnx: &mut cnx_t, _is_ready: bool) -> Result<(), ()> {
    todo!()
}

pub fn mark_datagram_ready_path(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _is_path_ready: bool,
) -> Result<(), ()> {
    todo!()
}

/// C: `provide_datagram_buffer`.  Old API, prefer
/// [`provide_datagram_buffer_ex`].
pub fn provide_datagram_buffer(_context: *mut c_void, _length: usize) -> Option<&'static mut [u8]> {
    todo!()
}

pub fn provide_datagram_buffer_ex(
    _context: *mut c_void,
    _length: usize,
    _is_active: datagram_active_enum,
) -> Option<&'static mut [u8]> {
    todo!()
}

// ---------------------------------------------------------------------------
// Misc per-context tunables.

pub fn set_optimistic_ack_policy(_quic: &mut quic_t, _sequence_hole_pseudo_period: u32) {
    todo!()
}

pub fn set_preemptive_repeat_policy(_quic: &mut quic_t, _do_repeat: bool) {
    todo!()
}

pub fn set_preemptive_repeat_per_cnx(_cnx: &mut cnx_t, _do_repeat: bool) {
    todo!()
}

pub fn enable_keep_alive(_cnx: &mut cnx_t, _interval: u64) {
    todo!()
}

pub fn disable_keep_alive(_cnx: &mut cnx_t) {
    todo!()
}

pub fn is_client(_cnx: &cnx_t) -> bool {
    todo!()
}

pub fn get_local_error(_cnx: &cnx_t) -> u64 {
    todo!()
}

pub fn get_remote_error(_cnx: &cnx_t) -> u64 {
    todo!()
}

pub fn get_application_error(_cnx: &cnx_t) -> u64 {
    todo!()
}

pub fn get_remote_stream_error(_cnx: &cnx_t, _stream_id: u64) -> u64 {
    todo!()
}

pub fn get_data_sent(_cnx: &cnx_t) -> u64 {
    todo!()
}

pub fn get_data_received(_cnx: &cnx_t) -> u64 {
    todo!()
}

pub fn cnx_is_still_logging(_cnx: &cnx_t) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------
// Congestion-control registry.

/// C: `register_congestion_control_algorithms`.  The C
/// signature took `congestion_algorithm_t const**` plus a
/// length; the Rust shape collapses both into a borrowed slice with
/// `'static` element lifetime — algorithms are typically file-scope
/// statics, mirroring how `register_all_cc_algorithms.c` builds the
/// list.
pub fn register_congestion_control_algorithms(_alg: &'static [&'static congestion_algorithm_t]) {
    todo!()
}

pub fn register_all_congestion_control_algorithms() {
    todo!()
}

pub fn get_congestion_algorithm(_alg_id: &str) -> Option<&'static congestion_algorithm_t> {
    todo!()
}

pub fn set_default_congestion_algorithm(
    _quic: &mut quic_t,
    _algo: &'static congestion_algorithm_t,
) {
    todo!()
}

pub fn set_default_congestion_algorithm_ex(
    _quic: &mut quic_t,
    _alg: &'static congestion_algorithm_t,
    _alg_option_string: Option<&str>,
) {
    todo!()
}

pub fn set_default_congestion_algorithm_by_name(_quic: &mut quic_t, _alg_name: &str) {
    todo!()
}

pub fn set_congestion_algorithm(_cnx: &mut cnx_t, _algo: &'static congestion_algorithm_t) {
    todo!()
}

pub fn set_congestion_algorithm_ex(
    _cnx: &mut cnx_t,
    _alg: &'static congestion_algorithm_t,
    _alg_option_string: Option<&str>,
) {
    todo!()
}

pub fn set_priority_limit_for_bypass(_cnx: &mut cnx_t, _priority_limit: u8) {
    todo!()
}

pub fn set_feedback_loss_notification(_cnx: &mut cnx_t, _should_notify: bool) {
    todo!()
}

pub fn request_forced_probe_up(_cnx: &mut cnx_t, _request_forced_probe_up: bool) {
    todo!()
}

pub fn subscribe_pacing_rate_updates(
    _cnx: &mut cnx_t,
    _decrease_threshold: u64,
    _increase_threshold: u64,
) {
    todo!()
}

pub fn get_pacing_rate(_cnx: &cnx_t) -> u64 {
    todo!()
}

pub fn get_cwin(_cnx: &cnx_t) -> u64 {
    todo!()
}

pub fn get_rtt(_cnx: &cnx_t) -> u64 {
    todo!()
}

// ---------------------------------------------------------------------------
// ECH / ESNI.

pub fn ech_configure_quic_ctx(
    _quic: &mut quic_t,
    _ech_private_key_file_name: Option<&str>,
    _ech_config_file_name: Option<&str>,
) -> Result<(), ()> {
    todo!()
}

pub fn release_quic_ech_ctx(_quic: &mut quic_t) {
    todo!()
}

pub fn ech_configure_client(_cnx: &mut cnx_t, _config_data: &[u8]) -> Result<(), ()> {
    todo!()
}

pub fn is_ech_handshake(_cnx: &cnx_t) -> bool {
    todo!()
}

/// C: `ech_get_retry_config`.  Two `uint8_t**` /
/// `size_t*` output parameters fold into a single returned
/// `&[u8]` borrow into the connection's retry config buffer
/// (empty when no retry config is available).
pub fn ech_get_retry_config(_cnx: &cnx_t) -> &[u8] {
    todo!()
}

pub fn ech_create_config_file(
    _public_name: &str,
    _private_key_file: &str,
    _ech_config_file: &str,
) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Base64 helpers.

/// C: `base64_decode`.  The C signature output an owned
/// buffer via `uint8_t** v` + `size_t* v_len`; the Rust translation
/// returns the decoded bytes by value.
pub fn base64_decode(_b64_txt: &str) -> Result<Vec<u8>, ()> {
    todo!()
}

/// C: `base64_encode`.  The C signature wrote into a
/// caller-supplied buffer with a fallible "buffer too small" path;
/// the Rust translation owns the result `String` and reports
/// errors only when the input is malformed.
pub fn base64_encode(_v: &[u8]) -> Result<String, ()> {
    todo!()
}

#[cfg(test)]
mod test {}
