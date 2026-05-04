//! Translation of `quic/quic.h`.
//!
//! `quic.h` is the kitchen-sink public header for the
//! quic-core library: it defines the protocol error codes, the
//! transport-parameter identifiers, the application-facing enums, the
//! opaque QUIC / connection / path types, the public structs that
//! cross the API boundary (`TransportParameters`, `PathQuality`,
//! `PerAckState`, `CongestionAlgorithm`, …)
//! and the dozens of free functions that make up the application API.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//!
//! Pointer-shape and translation policy notes that apply throughout
//! this module:
//!
//! * `Quic`, `Connection`, `Path` are
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
//!   `CongestionAlgorithm` vtable, where the C side
//!   always installs the four together.
//! * `void*` "callback context" arguments are folded into the trait
//!   implementor's state.  Per-stream / per-path application
//!   contexts that flow through the stack (e.g. `stream_ctx`) stay
//!   as `*mut c_void` — they are produced by the application and
//!   handed back unchanged.
//! * C `int` / `int64_t` return values that encode 0/-1 or 0/error
//!   status become `Result<T, Error>` — the crate-level `Error` enum
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

// Many translated functions mirror C signatures with >7 parameters.
// Builder patterns or shape changes are out of scope for Phase 1.
#![allow(clippy::too_many_arguments)]

pub mod arena;
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
#[cfg(test)]
pub mod tests;
pub mod textlog;
pub mod tls_api;
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

/// Crate-level error type for fallible operations.
///
/// Phase 1 stubs return `Result<T, Error>` even though the
/// `todo!()` bodies don't yet decide which variant to produce —
/// Phase 3 will fill that in based on the C control flow.  The
/// initial variant set covers the broad failure shapes; Phase 3
/// can add more as needed.  Marked `#[non_exhaustive]` so adding
/// variants later isn't a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Catch-all placeholder for paths Phase 1 has not yet
    /// distinguished.  Phase 3 should narrow these.
    Generic,
    /// Allocation failed.
    Memory,
    /// Caller supplied an invalid argument.
    InvalidArgument,
    /// A buffer the caller supplied was too small.
    BufferTooSmall,
    /// Caller-supplied file does not exist.
    NoSuchFile,
    /// Caller-supplied file was malformed.
    InvalidFile,
    /// Connection has been disconnected.
    Disconnected,
    /// Connection state didn't permit the requested operation.
    InvalidState,
    /// Operation produced or received a malformed QUIC frame.
    InvalidFrame,
    /// TLS / crypto error.
    Tls,
    /// Other QUIC protocol error; the wrapped value is the
    /// numeric `ERROR_*` code defined in this module.
    Protocol(u64),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl core::error::Error for Error {}

/// Crate-wide `Result` alias defaulting the error type to [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;

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

/// Connection-state machine, listing the QUIC connection states a
/// `Connection` walks through from initial handshake to teardown.
/// Discriminants follow the declaration order of the C `state_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum State {
    ClientInit,
    ClientInitSent,
    ClientRenegotiate,
    ClientRetryReceived,
    ClientInitResent,
    ServerInit,
    ServerHandshake,
    ClientHandshakeStart,
    HandshakeFailure,
    HandshakeFailureResend,
    ClientAlmostReady,
    ServerFalseStart,
    ServerAlmostReady,
    ClientReadyStart,
    Ready,
    Disconnecting,
    ClosingReceived,
    Closing,
    Draining,
    Disconnected,
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

/// Encryption-level packet context.  Each value selects one of the
/// three QUIC packet-number spaces (Initial, Handshake, 1-RTT).
/// The C enum's trailing `nb_packet_context` count is exposed as
/// the [`NB_PACKET_CONTEXT`] constant below.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketContext {
    Application = 0,
    Handshake = 1,
    Initial = 2,
}

/// Number of packet contexts (matches the variant count of
/// [`PacketContext`]).
pub const NB_PACKET_CONTEXT: usize = 3;

/// Path-MTU-discovery policy for a connection.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PmtudPolicy {
    /// Default opportunistic PMTUD.
    #[default]
    Basic = 0,
    /// Force PMTUD as soon as possible.
    Required = 1,
    /// Only do PMTUD if a lot of data has to be sent.
    Delayed = 2,
    /// Never do PMTUD.
    Blocked = 3,
}

/// Spin-bit variant policy.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum SpinbitVersion {
    /// Default behaviour, per the spin-bit draft.
    #[default]
    Basic = 0,
    /// Randomise per packet.
    Random = 1,
    /// Null behaviour, randomised per path.
    Null = 2,
    /// Test-only "always on" mode.  Not valid as a per-connection
    /// override (server only; see [`Quic::set_default_spinbit_policy`]).
    On = 3,
}

/// Loss-bit support level.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LossbitVersion {
    /// Loss bits disabled.
    #[default]
    None = 0,
    /// This endpoint sets the loss bits but does not interpret peer's.
    SendOnly = 1,
    /// This endpoint both sets and receives loss bits.
    SendReceive = 2,
}

/// Path scheduling status — whether a path participates in normal
/// scheduling or is held in reserve.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PathStatus {
    /// Path is available for sending.
    #[default]
    Available = 0,
    /// Path is held as a backup; data only flows when no Available
    /// path remains.
    Backup = 1,
}

// ---------------------------------------------------------------------------
// Connection ID.

pub const CONNECTION_ID_MIN_SIZE: usize = 0;
pub const CONNECTION_ID_MAX_SIZE: usize = 20;

/// Fixed-capacity QUIC connection ID.  C: `ConnectionId`.
///
/// Stored as a 20-byte buffer plus a length so the type is `Copy`,
/// matching the C usage where connection IDs are passed by value
/// in many APIs (`get_local_connection_id`, `create_connection`,
/// …).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct ConnectionId {
    pub id: [u8; CONNECTION_ID_MAX_SIZE],
    pub id_len: u8,
}

// ---------------------------------------------------------------------------
// IO vectors.
//
// The C `picoquic_iovec_t` (and the picotls-side `PtlsIovec*` that
// aliased it) are gone — every place that used to take an iovec
// pair now takes a `&[u8]` (single buffer) or `&[&[u8]]` /
// `Vec<Vec<u8>>` (list of buffers), per the project's
// "use Rust's slice/vec types" policy.

// ---------------------------------------------------------------------------
// Opaque types (forward declarations).
//
// `Quic`, `Connection`, and `Path` are
// defined in `internal.h`.  Phase 1 declares them as empty
// structs so callers can refer to them; the real layout lands when
// the internal header is translated.  The `_opaque` field prevents
// callers from constructing one accidentally, and the `Debug` impl
// gives the structs a printable shape for log lines.

// Full bodies live in `crate::internal`; pull them in for use within
// this module's signatures (no re-export — callers reach them as
// `crate::internal::*`).
use crate::internal::{Connection, Path, Quic};

// ---------------------------------------------------------------------------
// Application callback events.

/// Event type passed to the application's stream / connection
/// callback (see [`StreamDataCb`]).  Identifies which sort of
/// notification the stack is delivering — stream data, lifecycle
/// transition, datagram event, path event, etc.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CallbackEvent {
    StreamData = 0,
    StreamFin,
    StreamReset,
    StopSending,
    StatelessReset,
    Close,
    ApplicationClose,
    StreamGap,
    PrepareToSend,
    AlmostReady,
    Ready,
    Datagram,
    VersionNegotiation,
    RequestAlpnList,
    SetAlpn,
    PacingChanged,
    PrepareDatagram,
    DatagramAcked,
    DatagramLost,
    DatagramSpurious,
    PathAvailable,
    PathSuspended,
    PathDeleted,
    PathQualityChanged,
    PathAddressObserved,
    AppWakeup,
    NextPathAllowed,
}

// ---------------------------------------------------------------------------
// Transport parameters.

/// Server's preferred address advertised in transport parameters.
/// C: `TpPreferredAddress`.  `is_defined` was an `int`
/// flag in C; promoted to `bool`.
// Field names mirror the camelCase identifiers from the C struct verbatim.
#[allow(non_snake_case)]
#[derive(Debug, Default, Copy, Clone)]
pub struct TpPreferredAddress {
    pub is_defined: bool,
    pub ipv4Address: [u8; 4],
    pub ipv4Port: u16,
    pub ipv6Address: [u8; 16],
    pub ipv6Port: u16,
    pub connection_id: ConnectionId,
    pub statelessResetToken: [u8; 16],
}

/// Version negotiation TP payload.  C:
/// `TpVersionNegotiation`.  The flexible-length
/// `received` and `supported` arrays use `Vec<u32>`; the explicit
/// `nb_received` / `nb_supported` length fields disappear (the
/// `Vec` carries its length).
#[derive(Debug, Default, Clone)]
pub struct TpVersionNegotiation {
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
/// handshake.  C: `TransportParameters`.
///
/// `migration_disabled` was `unsigned int` in C, used as a Boolean
/// flag; promoted to `bool`.  Same for `do_grease_quic_bit`,
/// `enable_bdp_frame`, `is_reset_stream_at_enabled`.
/// `enable_loss_bit` and `enable_time_stamp` and
/// `address_discovery_mode` are kept as integers because callers
/// inspect the low bits separately ("want / can" flags).
#[derive(Debug, Default, Clone)]
pub struct TransportParameters {
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
    pub preferred_address: TpPreferredAddress,
    pub max_datagram_frame_size: u32,
    pub enable_loss_bit: i32,
    /// `(x & 1)` want, `(x & 2)` can.
    pub enable_time_stamp: i32,
    pub min_ack_delay: u64,
    pub do_grease_quic_bit: bool,
    pub version_negotiation: TpVersionNegotiation,
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

/// Wall-clock microseconds since the Unix epoch.
///
/// The C body reads the OS clock; in Rust this requires the `std`
/// feature (`std::time::SystemTime`).  The `cfg`-gated split lands
/// when the crate gains its `std` feature and the rest of the
/// `no_std + alloc` plumbing — for Phase 1 the function is just a
/// `todo!()` stub.
pub fn current_time() -> u64 {
    todo!()
}

impl Quic {
    /// Virtual time used by this QUIC context (wall-clock or
    /// simulated, depending on whether a simulated-time pointer was
    /// supplied at creation).  C: `get_quic_time`.
    pub fn time(&self) -> u64 {
        todo!()
    }
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
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32;
}

/// ALPN-selection callback.  Returns the index of the chosen ALPN
/// in `list`, or any value `>= list.len()` to signal "none of the
/// proposed ALPNs is supported".  C: `AlpnSelect` (the C `_v2`
/// flavour is folded in — it only differed in the iovec type, which
/// is now just `&[u8]`).
pub trait AlpnSelect {
    fn select(&mut self, quic: &mut Quic, list: &[&[u8]]) -> usize;
}

/// Callback that produces a server-environment-compatible CID.
/// Folds the C `void* connection_id_cb_data` into the implementor's state.
/// C: `ConnectionIdCb`.
pub trait ConnectionIdCb {
    fn produce(
        &mut self,
        quic: &mut Quic,
        connection_id_local: ConnectionId,
        connection_id_remote: ConnectionId,
    ) -> ConnectionId;
}

/// Packet-fuzzer callback.  Folds the C `void* fuzz_ctx` into the
/// implementor.  Returns the new packet length (which may equal
/// the input).  C: `Fuzz`.
pub trait Fuzz {
    fn fuzz(
        &mut self,
        connection: &mut Connection,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32;
}

/// Direct-receive callback for streams marked with
/// `mark_direct_receive_stream`.  C:
/// `StreamDirectReceive`.  Folds `direct_receive_ctx`
/// into the implementor.
pub trait StreamDirectReceive {
    fn receive(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        fin: bool,
        bytes: &[u8],
        offset: u64,
    ) -> i32;
}

// ---------------------------------------------------------------------------
// Path quality and per-ack state.

/// Per-path quality snapshot reported by
/// `get_path_quality`.  C: `PathQuality`.
#[derive(Debug, Default, Copy, Clone)]
pub struct PathQuality {
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

/// Datagram-readiness signalling passed to
/// [`provide_datagram_buffer_ex`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DatagramActive {
    /// Application has nothing more to send.
    #[default]
    NotActive = 0,
    /// Application can send on any path.
    AnyPath = 1,
    /// Application can send only on the current path.
    ThisPathOnly = 2,
    /// Application can send on the current path *and* others.
    ThisPathAndOthers = 3,
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

/// Congestion-control event fed to a [`CongestionControl`]
/// implementation's `alg_notify` hook.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CongestionNotification {
    /// At least one packet was acknowledged.
    Acknowledgement,
    /// A packet was retransmitted.
    Repeat,
    /// A retransmission timer expired.
    Timeout,
    /// A retransmission was found to have been spurious.
    SpuriousRepeat,
    /// A new RTT sample is available.
    RttMeasurement,
    /// ECN-marked traffic was acknowledged.
    EcnEc,
    /// The send path is blocked by the congestion window.
    CwinBlocked,
    /// Seed the congestion window from external knowledge.
    SeedCwin,
    /// Reset congestion-control state.
    Reset,
    /// Notification of lost feedback.
    LostFeedback,
}

/// Per-ACK state passed to the congestion-control algorithm.  C:
/// `PerAckState`.
///
/// Single-bit `unsigned int : 1` bitfields collapse to `bool`.  `pc`
/// stays an `i32` (the C field used `int` instead of the enum
/// itself to avoid include dependencies; we follow suit so the
/// translation can be checked field-by-field).
#[derive(Debug, Default, Copy, Clone)]
pub struct PerAckState {
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
pub trait CongestionControl {
    fn alg_init(&self, path_x: &mut Path, option_string: Option<&str>, current_time: u64);

    fn alg_notify(
        &self,
        connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: u64,
    );

    fn alg_delete(&self, path_x: &mut Path);

    /// Optional observation hook — many algorithms leave this
    /// unimplemented (`NULL` in C).  Callers that don't care can
    /// rely on the default `None` return.
    fn alg_observe(&self, _path_x: &Path) -> Option<(u64, u64)> {
        None
    }
}

/// Congestion-control algorithm descriptor.  C:
/// `CongestionAlgorithm`.
///
/// The four function pointers from the C struct fold into a single
/// trait-object reference; the algorithm identifier and numeric tag
/// stay alongside it.  `congestion_algorithm_id` is `&'static str`
/// because the C side stores compile-time string literals.
pub struct CongestionAlgorithm {
    pub congestion_algorithm_id: &'static str,
    pub congestion_algorithm_number: u8,
    pub ecn_mark: u8,
    pub algorithm: &'static dyn CongestionControl,
}

impl core::fmt::Debug for CongestionAlgorithm {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CongestionAlgorithm")
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
pub fn congestion_control_algorithms() -> &'static [&'static CongestionAlgorithm] {
    todo!()
}

// ---------------------------------------------------------------------------
// ALPN list.

/// Application-protocol identifiers used during session
/// negotiation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Alpn {
    /// No ALPN selected / unrecognised.
    #[default]
    Undef = 0,
    Http0_9,
    Http3,
    Quicperf,
}

/// One entry in the ALPN dispatch table.  `len` is implicit in
/// `alpn_val.len()` but kept as a separate field for source-level
/// parity with the C struct; Phase 3 may collapse to a single
/// `&'static [u8]` slice.
#[derive(Debug, Copy, Clone)]
pub struct AlpnEntry {
    pub alpn_code: Alpn,
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
/// during the TLS callback.  `tls_context` is the opaque handle
/// the TLS stack passed to the application callback; Phase 2's
/// dependency abstraction replaces it with a typed reference.
pub fn add_proposed_alpn(_tls_context: *mut c_void, _alpn: &str) -> Result<(), Error> {
    todo!()
}

impl Connection {
    /// Negotiated ALPN value (borrowed), or `None` when none was
    /// selected.
    pub fn tls_negotiated_alpn(&self) -> Option<&str> {
        todo!()
    }

    /// SNI value the peer presented during the handshake, or `None`
    /// when none was given.
    pub fn tls_sni(&self) -> Option<&str> {
        todo!()
    }

    /// Reasons the connection closed.  Returns `(local_reason,
    /// remote_reason, local_application_reason,
    /// remote_application_reason)`; the C signature exposed these as
    /// four `uint64_t*` out-parameters.
    pub fn close_reasons(&self) -> (u64, u64, u64, u64) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Per-context settings (logging, diagnostics, pool sizing,
// port-blocking).

impl Quic {
    /// Install a per-context packet fuzzer; `None` removes any
    /// previously installed fuzzer.
    pub fn set_fuzz(&mut self, _fuzzer: Option<Box<dyn Fuzz>>) {
        todo!()
    }

    /// `1` → log every packet, `0` → log only the first
    /// [`LOG_PACKET_MAX_SEQUENCE`] packets per connection.
    pub fn set_log_level(&mut self, _log_level: i32) {
        todo!()
    }

    /// Toggle randomised log-file names (defeating accidental
    /// collisions when clients pick non-random initial CIDs).
    pub fn set_use_unique_log_names(&mut self, _use_unique_log_names: bool) {
        todo!()
    }

    /// Toggle SSL-keylog output.  Phase 1 follows the canonical
    /// build (`WITHOUT_SSLKEYLOG` undefined); a `cfg`-gated variant
    /// lands when build options are translated.
    pub fn set_sslkeylog_enabled(&mut self, _enable_sslkeylog: bool) {
        todo!()
    }

    /// Whether SSL-keylog output is enabled on this context.
    pub fn is_sslkeylog_enabled(&self) -> bool {
        todo!()
    }

    /// Configure packet-number randomisation: `0` (no
    /// randomisation), `1` (Initial PNs only), `2` (all PN spaces).
    pub fn set_random_initial(&mut self, _random_initial: i32) {
        todo!()
    }

    /// Toggle packet-train mode (groups outbound packets into
    /// coalesced trains).
    pub fn set_packet_train_mode(&mut self, _train_mode: bool) {
        todo!()
    }

    /// Configure default padding policy applied to outbound packets.
    pub fn set_padding_policy(&mut self, _padding_min_size: u32, _padding_multiple: u32) {
        todo!()
    }

    /// Set (or clear, with `None`) the keylog destination file.
    pub fn set_key_log_file(&mut self, _keylog_filename: Option<&str>) {
        todo!()
    }

    /// Read `SSLKEYLOGFILE` from the process environment (gated
    /// behind the build-time `WITHOUT_SSLKEYLOG` opt-out and the
    /// per-context `is_sslkeylog_enabled` flag) and install it via
    /// [`Quic::set_key_log_file`].  C:
    /// `picoquic_set_key_log_file_from_env`.
    ///
    /// The environment lookup is a `std`-only operation (it goes
    /// through `getenv`); Phase 3 will feature-gate the body.
    pub fn set_key_log_file_from_env(&mut self) {
        todo!()
    }

    /// Adjust the connection-pool ceiling.  Cannot grow past the
    /// limit chosen at context creation.
    pub fn adjust_max_connections(&mut self, _max_nb_connections: u32) -> Result<(), Error> {
        todo!()
    }

    /// Number of connections currently registered with this context.
    pub fn current_number_connections(&self) -> u32 {
        todo!()
    }

    /// Set the half-open-connection threshold above which the server
    /// switches to retry-token mode.
    pub fn set_max_half_open_retry_threshold(&mut self, _max_half_open_before_retry: u32) {
        todo!()
    }

    /// Retrieve the configured half-open retry threshold.
    pub fn max_half_open_retry_threshold(&self) -> u32 {
        todo!()
    }

    /// Toggle per-context port blocking.
    pub fn set_port_blocking_disabled(&mut self, _is_port_blocking_disabled: bool) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Free-standing port / address blocklist queries (no per-context
// state — these consult a process-wide allowlist).

/// Returns `true` when `port` is on the QUIC well-known-port
/// blocklist.
pub fn check_port_blocked(_port: u16) -> bool {
    todo!()
}

/// Returns `true` when `addr_from` belongs to a blocked address
/// range.
pub fn check_addr_blocked(_addr_from: &SocketAddr) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------
// QUIC context: construction, TLS configuration, default policies.
//
// `picoquic_create` is the canonical constructor in the C source;
// here it is `Quic::new`.  The setters that follow are all on the
// QUIC context; per-connection siblings live in `impl Connection` further
// down.

impl Quic {
    /// Build a QUIC context with the supplied certificate paths,
    /// default callbacks, and reset seed.
    ///
    /// Pointer-shape choices, derived from `quic/quicctx.c:634` and
    /// `quic/sockloop.c:2009`:
    ///
    /// * Every `char const*` parameter is `Option<&str>` (the C
    ///   source passes `NULL` to mean "absent").
    /// * `default_callback_fn` + `default_callback_ctx` collapse to
    ///   one `Option<Box<dyn StreamDataCb>>`.
    /// * `connection_id_callback` + `connection_id_callback_data` collapse to
    ///   `Option<Box<dyn ConnectionIdCb>>`.
    /// * `reset_seed[16]` is `[u8; RESET_SECRET_SIZE]` taken by
    ///   value (the C body deep-copies it into the context).
    /// * `p_simulated_time: *mut u64` becomes `Option<&mut u64>`;
    ///   the QUIC context retains the borrow across calls.
    /// * `ticket_encryption_key` + `ticket_encryption_key_length`
    ///   collapse to a borrowed `Option<&[u8]>`.
    ///
    /// Returns `None` when context creation fails (the C side
    /// returned `NULL`).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
    ) -> Option<Box<Quic>> {
        todo!()
    }

    // The C `picoquic_free` entry point is dropped from the Rust API
    // — context cleanup is the job of `Drop`, which Phase 3 will
    // implement.

    /// Toggle low-memory mode (smaller buffers, more aggressive
    /// reclaim).
    pub fn set_low_memory_mode(&mut self, _low_memory_mode: bool) -> Result<(), Error> {
        todo!()
    }

    /// Configure the server cookie / retry-token mode.
    pub fn set_cookie_mode(&mut self, _cookie_mode: i32) {
        todo!()
    }

    /// Restrict TLS cipher-suite selection to the given IANA ID.
    pub fn set_cipher_suite(&mut self, _cipher_suite_id: u16) -> Result<(), Error> {
        todo!()
    }

    /// Restrict TLS key-exchange selection to the given IANA group.
    pub fn set_key_exchange(&mut self, _key_exchange_id: u16) -> Result<(), Error> {
        todo!()
    }

    /// Replace the default transport parameters used for new
    /// connections.
    pub fn set_default_tp(&mut self, _tp: &TransportParameters) -> Result<(), Error> {
        todo!()
    }

    /// Borrow this context's default transport parameters.
    pub fn default_tp(&self) -> &TransportParameters {
        todo!()
    }

    /// Override a single transport-parameter slot in the default
    /// set.  `tp_type` is the wire ID; values for unknown types are
    /// stored verbatim and emitted as extension parameters.
    pub fn set_default_tp_value(&mut self, _tp_type: u64, _tp_value: u64) -> Result<(), Error> {
        todo!()
    }

    /// Install the TLS certificate chain.  The context takes
    /// ownership of `certs` (each entry is one DER-encoded
    /// certificate).
    pub fn set_tls_certificate_chain(&mut self, _certs: Vec<Vec<u8>>) {
        todo!()
    }

    /// Install the TLS root certificate set.  The C `int` return
    /// distinguished load vs. store failure (`-1` / `-2`); Phase 1
    /// collapses both into [`Error::Generic`] pending refinement in
    /// Phase 4.
    pub fn set_tls_root_certificates(&mut self, _certs: Vec<Vec<u8>>) -> Result<(), Error> {
        todo!()
    }

    /// Install a no-op certificate verifier (test / interop only).
    pub fn set_null_verifier(&mut self) {
        todo!()
    }

    /// Install the TLS private key (caller-owned bytes; the context
    /// copies on the way in).
    pub fn set_tls_key(&mut self, _key: &[u8]) -> Result<(), Error> {
        todo!()
    }

    /// Install a custom certificate-verification callback.  The
    /// QUIC context takes ownership of the verifier; `Drop` on the
    /// box subsumes the C `free_fn` custodial hook.
    pub fn set_verify_certificate_callback(
        &mut self,
        _cb: Box<dyn crate::crypto_provider_api::VerifyCertificate>,
    ) {
        todo!()
    }

    /// Toggle whether this context demands client-side TLS
    /// authentication.
    pub fn set_client_authentication(&mut self, _client_authentication: bool) {
        todo!()
    }

    /// Toggle whether the TLS exporter API is available on this
    /// context.
    pub fn set_use_exporter(&mut self, _use_exporter: bool) {
        todo!()
    }

    /// Reject server-mode connections on this context (clients
    /// only).
    pub fn enforce_client_only(&mut self, _do_enforce: bool) {
        todo!()
    }

    /// Default packet-padding policy (multiple, min-size).
    pub fn set_default_padding(&mut self, _padding_multiple: u32, _padding_minsize: u32) {
        todo!()
    }

    /// Default spin-bit policy applied to new connections.
    pub fn set_default_spinbit_policy(
        &mut self,
        _default_spinbit_policy: SpinbitVersion,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Default loss-bit policy applied to new connections.
    pub fn set_default_lossbit_policy(&mut self, _default_lossbit_policy: LossbitVersion) {
        todo!()
    }

    /// Default multipath option (per the multipath QUIC draft).
    pub fn set_default_multipath_option(&mut self, _multipath_option: i32) {
        todo!()
    }

    /// Default address-discovery mode (per the address-discovery
    /// draft): `0`=none, `1`=provide-only, `2`=receive-only,
    /// `3`=both.
    pub fn set_default_address_discovery_mode(&mut self, _mode: i32) {
        todo!()
    }

    /// Cap the congestion window across all connections on this
    /// context.
    pub fn set_cwin_max(&mut self, _cwin_max: u64) {
        todo!()
    }

    /// Cap the maximum stream data control window across this
    /// context.
    pub fn set_max_data_control(&mut self, _max_data: u64) {
        todo!()
    }

    /// Default idle-timeout (in milliseconds) advertised on new
    /// connections.
    pub fn set_default_idle_timeout(&mut self, _idle_timeout_ms: u64) {
        todo!()
    }
}

impl Connection {
    /// Replace the local transport parameters for this connection
    /// before the handshake completes.
    pub fn set_transport_parameters(&mut self, _tp: &TransportParameters) {
        todo!()
    }

    /// Borrow the local (`get_local = true`) or remote transport
    /// parameters of this connection.
    pub fn transport_parameters(&self, _get_local: bool) -> &TransportParameters {
        todo!()
    }

    /// Export TLS keying material to `out` using `label`.  Returns
    /// the number of bytes written.
    pub fn export_secret(&mut self, _label: &str, _out: &mut [u8]) -> Result<usize, Error> {
        todo!()
    }

    /// Per-connection spin-bit policy override.
    pub fn set_spinbit_policy(&mut self, _spinbit_policy: SpinbitVersion) -> Result<(), Error> {
        todo!()
    }
}

impl Quic {
    /// Default per-connection handshake-timeout (microseconds).
    pub fn set_default_handshake_timeout(&mut self, _handshake_timeout_us: u64) {
        todo!()
    }

    /// Default per-connection crypto-epoch length (in encrypted
    /// bytes before triggering a key update).
    pub fn set_default_crypto_epoch_length(&mut self, _crypto_epoch_length_max: u64) {
        todo!()
    }

    /// Retrieve the configured default crypto-epoch length.
    pub fn default_crypto_epoch_length(&self) -> u64 {
        todo!()
    }

    /// Local-CID length in bytes (the value advertised in new
    /// connections).
    pub fn local_cid_length(&self) -> u8 {
        todo!()
    }

    /// Returns `true` when `cid` was issued by this context.
    pub fn is_local_cid(&self, _cid: &ConnectionId) -> bool {
        todo!()
    }

    /// Load issued-retry-tokens from a persistent store.
    pub fn load_retry_tokens(&mut self, _token_store_filename: &str) -> Result<(), Error> {
        todo!()
    }

    /// Persist session tickets to disk.
    pub fn save_session_tickets(&mut self, _ticket_store_filename: &str) -> Result<(), Error> {
        todo!()
    }

    /// Persist outstanding retry tokens to disk.
    pub fn save_retry_tokens(&mut self, _token_store_filename: &str) -> Result<(), Error> {
        todo!()
    }

    /// Toggle BDP-frame extension on new connections (per the
    /// 0-RTT-BDP draft).
    pub fn set_default_bdp_frame_option(&mut self, _enable_bdp_frame: bool) {
        todo!()
    }

    /// Configure the local CID length (in bytes) advertised on new
    /// connections.
    pub fn set_default_connection_id_length(&mut self, _cid_length: u8) -> Result<(), Error> {
        todo!()
    }

    /// Default per-connection-ID time-to-live before retirement, in
    /// microseconds.
    pub fn set_default_connection_id_ttl(&mut self, _ttl_usec: u64) {
        todo!()
    }

    /// Retrieve the configured default connection-ID TTL.
    pub fn default_connection_id_ttl(&self) -> u64 {
        todo!()
    }

    /// Cap the maximum MTU PMTUD will probe up to.
    pub fn set_mtu_max(&mut self, _mtu_max: u32) {
        todo!()
    }

    /// Install (or remove, with `None`) the ALPN-selection callback.
    /// The C `_v2` flavour (which differed only in iovec type) is
    /// gone — both call sites land on this single entry point.
    pub fn set_alpn_select_fn(&mut self, _alpn_select_fn: Option<Box<dyn AlpnSelect>>) {
        todo!()
    }

    /// Install (or remove, with `None`) the default stream/event
    /// callback applied to new connections.  The `(callback_fn,
    /// callback_ctx)` pair from C collapses into one trait object.
    pub fn set_default_callback(&mut self, _callback: Option<Box<dyn StreamDataCb>>) {
        todo!()
    }

    /// Default minimum interval between stateless-reset emissions
    /// (microseconds).
    pub fn set_default_stateless_reset_min_interval(&mut self, _min_interval_usec: u64) {
        todo!()
    }

    /// Cap the number of connections that may emit logs
    /// simultaneously.
    pub fn set_max_simultaneous_logs(&mut self, _max_simultaneous_logs: u32) {
        todo!()
    }

    /// Retrieve the configured maximum-simultaneous-logs cap.
    pub fn max_simultaneous_logs(&self) -> u32 {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Connection lifecycle and path management.

impl Quic {
    /// Create a new connection bound to this QUIC context.
    /// `client_mode` selects between the client and server roles
    /// (`char` flag in C, promoted to `bool` here).
    ///
    /// The returned `&mut Connection` borrows from the context because the
    /// C side stores the new connection in the context's hash
    /// tables.
    #[allow(clippy::too_many_arguments)]
    pub fn create_connection(
        &mut self,
        _initial_cnx_id: ConnectionId,
        _remote_cnx_id: ConnectionId,
        _addr_to: Option<&SocketAddr>,
        _start_time: u64,
        _preferred_version: u32,
        _sni: Option<&str>,
        _alpn: Option<&str>,
        _client_mode: bool,
    ) -> Option<&mut Connection> {
        todo!()
    }

    /// Convenience wrapper around [`Self::create_connection`] for the
    /// client side; `addr` is required.
    pub fn create_client_connection(
        &mut self,
        _addr: &SocketAddr,
        _start_time: u64,
        _preferred_version: u32,
        _sni: Option<&str>,
        _alpn: Option<&str>,
        _callback: Option<Box<dyn StreamDataCb>>,
    ) -> Option<&mut Connection> {
        todo!()
    }

    /// Default callback enablement for path-state events on new
    /// connections.
    pub fn set_path_callbacks_default(&mut self, _are_enabled: bool) {
        todo!()
    }

    /// Default thresholds for the path-quality-update callback.
    pub fn set_default_quality_update(&mut self, _pacing_rate_delta: u64, _rtt_delta: u64) {
        todo!()
    }
}

impl Connection {
    /// Begin the client-side handshake on this connection.
    pub fn start_client(&mut self) -> Result<(), Error> {
        todo!()
    }

    /// Begin an ordered close.
    pub fn close(&mut self, _application_reason_code: u64) -> Result<(), Error> {
        todo!()
    }

    /// Same as [`Self::close`] but carries a textual `error_reason`.
    /// `None` matches the C `NULL` case.
    pub fn close_with_reason(
        &mut self,
        _application_reason_code: u64,
        _error_reason: Option<&str>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Force-close the connection without waiting for the protocol
    /// drain.
    pub fn close_immediate(&mut self) {
        todo!()
    }

    /// Delete the connection.  In the C API this releases the
    /// connection's slot inside its QUIC context; in Rust the
    /// resources drop when `Connection` itself does, so this remains a
    /// `todo!()` until Phase 3 wires up the deletion semantics.
    pub fn delete(&mut self) {
        todo!()
    }

    /// Override the application-set wake time.
    pub fn set_app_wake_time(&mut self, _app_wake_time: u64) {
        todo!()
    }

    /// Set the version the client should request on next handshake.
    pub fn set_desired_version(&mut self, _desired_version: u32) {
        todo!()
    }

    /// Record a peer-rejected version (for diagnostics / VN frames).
    pub fn set_rejected_version(&mut self, _rejected_version: u32) {
        todo!()
    }

    /// Probe a new local↔peer path tuple.
    pub fn probe_new_path(
        &mut self,
        _addr_peer: &SocketAddr,
        _addr_local: &SocketAddr,
        _current_time: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Probe a new path with explicit interface index and
    /// preferred-address flag.
    pub fn probe_new_path_ex(
        &mut self,
        _addr_peer: &SocketAddr,
        _addr_local: &SocketAddr,
        _if_index: i32,
        _current_time: u64,
        _to_preferred_address: bool,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Probe a new tuple on an existing path object.
    #[allow(clippy::too_many_arguments)]
    pub fn probe_new_tuple(
        &mut self,
        _path_x: &mut Path,
        _addr_peer: &SocketAddr,
        _addr_local: &SocketAddr,
        _if_index: i32,
        _current_time: u64,
        _to_preferred_address: bool,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Toggle path-state event callbacks for this connection.
    pub fn set_path_callbacks(&mut self, _are_enabled: bool) {
        todo!()
    }

    /// Attach opaque application data to a specific path.
    /// `app_path_ctx` is produced and consumed by the application
    /// without the stack interpreting it.
    pub fn set_app_path_ctx(
        &mut self,
        _unique_path_id: u64,
        _app_path_ctx: Option<Box<dyn core::any::Any>>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Tear down a path.
    pub fn abandon_path(
        &mut self,
        _unique_path_id: u64,
        _reason: u64,
        _current_time: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Issue a fresh CID for the given path.
    pub fn refresh_path_connection_id(&mut self, _unique_path_id: u64) -> Result<(), Error> {
        todo!()
    }

    /// Pin a stream to a specific path.
    pub fn set_stream_path_affinity(
        &mut self,
        _stream_id: u64,
        _unique_path_id: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Mark a path as Available or Backup.
    pub fn set_path_status(
        &mut self,
        _unique_path_id: u64,
        _status: PathStatus,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Subscribe to "new path allowed" events.
    ///
    /// The C signature returned the answer through an
    /// `int* is_already_allowed` parameter that doubled as a status
    /// flag; the Rust shape splits that: `Ok(true)` if a new path is
    /// already allowed (caller can proceed immediately), `Ok(false)`
    /// if the caller will be notified later by callback.
    pub fn subscribe_new_path_allowed(&mut self) -> Result<bool, Error> {
        todo!()
    }

    /// Override the interface index for the first path.
    pub fn set_first_if_index(&mut self, _if_index: u32) -> Result<(), Error> {
        todo!()
    }

    /// Look up a path's address.  `local` selects which: `1` =
    /// local, `2` = peer, `3` = peer's observed.  The C side
    /// returned this through a `struct sockaddr_storage*`
    /// out-parameter; here it folds into the `Result`.
    pub fn path_addr(&self, _unique_path_id: u64, _local: i32) -> Result<SocketAddr, Error> {
        todo!()
    }

    /// Snapshot a path's quality metrics.
    pub fn path_quality(&self, _unique_path_id: u64) -> Result<PathQuality, Error> {
        todo!()
    }

    /// Snapshot the default path's quality metrics.
    pub fn default_path_quality(&self) -> PathQuality {
        todo!()
    }

    /// Subscribe to quality-update events on a specific path with
    /// the given thresholds.
    pub fn subscribe_to_quality_update_per_path(
        &mut self,
        _unique_path_id: u64,
        _pacing_rate_delta: u64,
        _rtt_delta: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Subscribe to quality-update events on every path of this
    /// connection.
    pub fn subscribe_to_quality_update(&mut self, _pacing_rate_delta: u64, _rtt_delta: u64) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Connection iteration, timing, accessors, and frame queueing.

impl Connection {
    /// Trigger the next TLS key rotation.
    pub fn start_key_rotation(&mut self) -> Result<(), Error> {
        todo!()
    }

    /// Borrow the QUIC context that owns this connection.
    pub fn quic(&mut self) -> &mut Quic {
        todo!()
    }

    /// Walk to the next connection in the QUIC context's list, if
    /// any.  (Named `next_in_list` rather than `next` to avoid
    /// confusion with the `Iterator::next` shape — Phase 3 may
    /// turn this into a proper `Iterator` impl on `Quic`.)
    pub fn next_in_list(&mut self) -> Option<&mut Connection> {
        todo!()
    }

    /// Compute the number of microseconds until this connection
    /// next needs attention, capped at `delay_max`.
    pub fn wake_delay(&self, _current_time: u64, _delay_max: i64) -> i64 {
        todo!()
    }

    /// Connection state-machine position.
    pub fn state(&self) -> State {
        todo!()
    }

    /// Override the per-connection padding policy.
    pub fn set_padding_policy(&mut self, _padding_multiple: u32, _padding_minsize: u32) {
        todo!()
    }

    /// Read the per-connection padding policy as `(multiple,
    /// min-size)`.
    pub fn padding_policy(&self) -> (u32, u32) {
        todo!()
    }

    /// Set the per-connection crypto-epoch length.
    pub fn set_crypto_epoch_length(&mut self, _crypto_epoch_length_max: u64) {
        todo!()
    }

    /// Read the per-connection crypto-epoch length.
    pub fn crypto_epoch_length(&self) -> u64 {
        todo!()
    }

    /// Override the connection's PMTUD policy.
    pub fn set_pmtud_policy(&mut self, _pmtud_policy: PmtudPolicy) {
        todo!()
    }

    /// Obsolete; prefer [`Self::set_pmtud_policy`].  Kept for
    /// source-level parity with the C API (`connection_set_pmtud_required`).
    pub fn set_pmtud_required(&mut self, _is_pmtud_required: bool) {
        todo!()
    }

    /// Returns `true` when the handshake completed using a
    /// pre-shared key (PSK).
    pub fn tls_is_psk_handshake(&self) -> bool {
        todo!()
    }

    /// Peer address of the default path.  C side returned an
    /// aliasing `struct sockaddr*` into internal storage; the Rust
    /// translation returns by value.
    pub fn peer_addr(&self) -> SocketAddr {
        todo!()
    }

    /// Local address of the default path.
    pub fn local_addr(&self) -> SocketAddr {
        todo!()
    }

    /// Local interface index for the default path.
    pub fn local_if_index(&self) -> u32 {
        todo!()
    }

    /// Set the local address for the default path.
    pub fn set_local_addr(&mut self, _addr: &SocketAddr) -> Result<(), Error> {
        todo!()
    }

    /// Local connection ID currently in use.
    pub fn local_connection_id(&self) -> ConnectionId {
        todo!()
    }

    /// Remote connection ID currently in use.
    pub fn remote_connection_id(&self) -> ConnectionId {
        todo!()
    }

    /// Initial connection ID picked at handshake start.
    pub fn initial_connection_id(&self) -> ConnectionId {
        todo!()
    }

    /// Client-side initial connection ID (mirrors C
    /// `get_client_connection_id`).
    pub fn client_connection_id(&self) -> ConnectionId {
        todo!()
    }

    /// Server-side initial connection ID (mirrors C
    /// `get_server_connection_id`).
    pub fn server_connection_id(&self) -> ConnectionId {
        todo!()
    }

    /// Connection ID used for log entries on this connection.
    pub fn logging_connection_id(&self) -> ConnectionId {
        todo!()
    }

    /// Wall-clock start time of this connection.
    pub fn start_time(&self) -> u64 {
        todo!()
    }

    /// Whether 0-RTT data may be sent on this connection.
    pub fn is_0rtt_available(&self) -> bool {
        todo!()
    }

    /// Whether the connection has any outstanding data still queued
    /// for transmission.
    pub fn is_backlog_empty(&self) -> bool {
        todo!()
    }

    /// Install a per-connection stream/event callback (the
    /// `callback_fn` + `callback_ctx` pair from C collapse to a
    /// single trait object).  See [`Quic::set_default_callback`].
    pub fn set_callback(&mut self, _callback: Option<Box<dyn StreamDataCb>>) {
        todo!()
    }

    /// Borrow this connection's callback (`None` when none was
    /// installed).
    pub fn callback(&self) -> Option<&dyn StreamDataCb> {
        todo!()
    }

    /// Queue a connection-level frame for transmission.
    pub fn queue_misc_frame(
        &mut self,
        _bytes: &[u8],
        _is_pure_ack: bool,
        _pc: PacketContext,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Queue a datagram frame for transmission.
    pub fn queue_datagram_frame(&mut self, _bytes: &[u8]) -> Result<(), Error> {
        todo!()
    }
}

impl Quic {
    /// Borrow the first connection registered with this context.
    pub fn first_connection(&mut self) -> Option<&mut Connection> {
        todo!()
    }

    /// Compute the number of microseconds until *any* connection on
    /// this context next needs attention, capped at `delay_max`.
    pub fn next_wake_delay(&self, _current_time: u64, _delay_max: i64) -> i64 {
        todo!()
    }

    /// Wall-clock time at which the next event is scheduled.
    pub fn next_wake_time(&self, _current_time: u64) -> u64 {
        todo!()
    }

    /// Borrow the connection currently advancing through its state
    /// machine, if any (`get_cnx_in_progress` in C).
    pub fn connection_in_progress(&mut self) -> Option<&mut Connection> {
        todo!()
    }

    /// Default PMTUD policy applied to new connections.
    pub fn set_default_pmtud_policy(&mut self, _pmtud_policy: PmtudPolicy) {
        todo!()
    }

    /// Borrow the default stream callback installed on this context.
    pub fn default_callback(&self) -> Option<&dyn StreamDataCb> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Packet I/O.

impl Quic {
    /// Submit a received packet to the QUIC context for dispatch.
    /// `addr_from` and `addr_to` are borrowed for the duration of
    /// the call; `received_ecn` carries the IP-level ECN code-point
    /// byte.
    #[allow(clippy::too_many_arguments)]
    pub fn incoming_packet(
        &mut self,
        _bytes: &mut [u8],
        _addr_from: &SocketAddr,
        _addr_to: &SocketAddr,
        _if_index_to: i32,
        _received_ecn: u8,
        _current_time: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Same as [`Self::incoming_packet`] but additionally identifies
    /// the connection that consumed the packet.
    #[allow(clippy::too_many_arguments)]
    pub fn incoming_packet_ex(
        &mut self,
        _bytes: &mut [u8],
        _addr_from: &SocketAddr,
        _addr_to: &SocketAddr,
        _if_index_to: i32,
        _received_ecn: u8,
        _current_time: u64,
    ) -> Result<Option<&mut Connection>, Error> {
        todo!()
    }
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
    pub log_cid: ConnectionId,
    /// Connection that produced the packet (the C
    /// `p_last_connection`).  `None` when the QUIC context had no work to
    /// do.
    pub last_connection: Option<&'a mut Connection>,
    /// Optional GSO segment size when the packet is a coalesced
    /// train; `None` for a single-packet send.
    pub send_msg_size: Option<usize>,
}

impl Quic {
    /// Drive the next packet onto the wire.  Folds the seven
    /// out-parameters of the C signature into a [`PreparedPacket`].
    pub fn prepare_next_packet_ex(
        &mut self,
        _current_time: u64,
        _send_buffer: &mut [u8],
    ) -> Result<PreparedPacket<'_>, Error> {
        todo!()
    }

    /// Same shape as [`Self::prepare_next_packet_ex`] but without
    /// GSO segment reporting.
    pub fn prepare_next_packet(
        &mut self,
        _current_time: u64,
        _send_buffer: &mut [u8],
    ) -> Result<PreparedPacket<'_>, Error> {
        todo!()
    }
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

impl Connection {
    /// Prepare the next packet on this connection (the `_ex`
    /// flavour reports GSO segment size when the packet is a
    /// coalesced train).
    pub fn prepare_packet_ex(
        &mut self,
        _current_time: u64,
        _send_buffer: &mut [u8],
    ) -> Result<PreparedCnxPacket, Error> {
        todo!()
    }

    /// Same shape as [`Self::prepare_packet_ex`] without
    /// GSO-segment reporting.
    pub fn prepare_packet(
        &mut self,
        _current_time: u64,
        _send_buffer: &mut [u8],
    ) -> Result<PreparedCnxPacket, Error> {
        todo!()
    }

    /// Notify this connection that a destination became
    /// unreachable.
    pub fn notify_destination_unreachable(
        &mut self,
        _current_time: u64,
        _addr_peer: &SocketAddr,
        _addr_local: &SocketAddr,
        _if_index: i32,
        _socket_err: i32,
    ) {
        todo!()
    }
}

impl Quic {
    /// Notify the connection identified by `connection_id` that a
    /// destination became unreachable.
    #[allow(clippy::too_many_arguments)]
    pub fn notify_destination_unreachable_by_connection_id(
        &mut self,
        _connection_id: &ConnectionId,
        _current_time: u64,
        _addr_peer: &SocketAddr,
        _addr_local: &SocketAddr,
        _if_index: i32,
        _socket_err: i32,
    ) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Streams.

impl Connection {
    /// Mark a stream as direct-receive: the stack hands incoming
    /// stream payload straight to `direct_receive` instead of
    /// queueing it for the application's regular callback.
    pub fn mark_direct_receive_stream(
        &mut self,
        _stream_id: u64,
        _direct_receive: Box<dyn StreamDirectReceive>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Attach opaque application data to a stream.
    pub fn set_app_stream_ctx(
        &mut self,
        _stream_id: u64,
        _app_stream_ctx: Option<Box<dyn core::any::Any>>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Detach the application data attached by
    /// [`Self::set_app_stream_ctx`].
    pub fn unlink_app_stream_ctx(&mut self, _stream_id: u64) {
        todo!()
    }

    /// Toggle whether the stack should poll the application for
    /// more data on this stream.
    pub fn mark_active_stream(
        &mut self,
        _stream_id: u64,
        _is_active: bool,
        _v_stream_ctx: Option<Box<dyn core::any::Any>>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Toggle whether this stream is excluded from coalesced packet
    /// trains.
    pub fn set_stream_not_coalesced(
        &mut self,
        _stream_id: u64,
        _is_not_coalesced: bool,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Set per-stream priority (smaller is higher).
    pub fn set_stream_priority(
        &mut self,
        _stream_id: u64,
        _stream_priority: u8,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Mark a stream as high-priority (skip ahead of normal
    /// streams).
    pub fn mark_high_priority_stream(
        &mut self,
        _stream_id: u64,
        _is_high_priority: bool,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Override the priority used for outbound datagrams on this
    /// connection.
    pub fn set_datagram_priority(&mut self, _datagram_priority: u8) {
        todo!()
    }
}

impl Quic {
    /// Default per-stream priority applied to new streams.
    pub fn set_default_priority(&mut self, _default_stream_priority: u8) {
        todo!()
    }

    /// Default datagram priority applied to new connections.
    pub fn set_default_datagram_priority(&mut self, _default_datagram_priority: u8) {
        todo!()
    }
}

/// C: `provide_stream_data_buffer`.  `context` is the
/// `context` is the stack-supplied handle the application receives
/// in the `callback_prepare_to_send` callback (a wrapper around
/// the internal `StreamDataBufferArgument`).  Returns the
/// borrowed buffer the application should fill, or `None` on
/// error.  Lifetime ties the returned slice to `context` so the
/// borrow ends with the callback.
pub fn provide_stream_data_buffer<'a>(
    _context: &'a mut crate::internal::StreamDataBufferArgument<'a>,
    _nb_bytes: usize,
    _is_fin: bool,
    _is_still_active: bool,
) -> Option<&'a mut [u8]> {
    todo!()
}

impl Connection {
    /// Append `data` to a stream's send buffer (`set_fin` closes the
    /// stream when the data is fully delivered).
    pub fn add_to_stream(
        &mut self,
        _stream_id: u64,
        _data: &[u8],
        _set_fin: bool,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Reset just the per-stream application context.
    pub fn reset_stream_ctx(&mut self, _stream_id: u64) {
        todo!()
    }

    /// Same as [`Self::add_to_stream`] but also installs an
    /// application-supplied stream context for callbacks.
    pub fn add_to_stream_with_ctx(
        &mut self,
        _stream_id: u64,
        _data: &[u8],
        _set_fin: bool,
        _app_stream_ctx: Option<Box<dyn core::any::Any>>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Send a STREAM_RESET frame for this stream.
    pub fn reset_stream(&mut self, _stream_id: u64, _local_stream_error: u64) -> Result<(), Error> {
        todo!()
    }

    /// Send a STREAM_RESET_AT frame (per the reliable-stream-reset
    /// draft) for this stream.
    pub fn reset_stream_at(
        &mut self,
        _stream_id: u64,
        _local_stream_error: u64,
        _reliable_size: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Open the flow-control window for an inbound stream up to the
    /// expected payload size.
    pub fn open_flow_control(
        &mut self,
        _stream_id: u64,
        _expected_data_size: u64,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Toggle application-managed flow control on a stream.
    pub fn set_app_flow_control(
        &mut self,
        _stream_id: u64,
        _use_app_flow_control: bool,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Allocate the next locally-initiated stream ID.
    pub fn next_local_stream_id(&mut self, _is_unidir: bool) -> u64 {
        todo!()
    }

    /// Send a STOP_SENDING frame for this stream.
    pub fn stop_sending(&mut self, _stream_id: u64, _local_stream_error: u64) -> Result<(), Error> {
        todo!()
    }

    /// Drop a stream from local bookkeeping (rejecting further peer
    /// frames).
    pub fn discard_stream(
        &mut self,
        _stream_id: u64,
        _local_stream_error: u16,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Toggle datagram readiness for this connection.
    pub fn mark_datagram_ready(&mut self, _is_ready: bool) -> Result<(), Error> {
        todo!()
    }

    /// Per-path datagram readiness (multipath connections).
    pub fn mark_datagram_ready_path(
        &mut self,
        _unique_path_id: u64,
        _is_path_ready: bool,
    ) -> Result<(), Error> {
        todo!()
    }
}

/// C: `provide_datagram_buffer`.  Old API, prefer
/// [`provide_datagram_buffer_ex`].
pub fn provide_datagram_buffer<'a>(
    _context: &'a mut crate::internal::StreamDataBufferArgument<'a>,
    _length: usize,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn provide_datagram_buffer_ex<'a>(
    _context: &'a mut crate::internal::StreamDataBufferArgument<'a>,
    _length: usize,
    _is_active: DatagramActive,
) -> Option<&'a mut [u8]> {
    todo!()
}

// ---------------------------------------------------------------------------
// Misc per-context tunables and per-connection accessors.

impl Quic {
    /// Configure the optimistic-ACK throttling policy
    /// (`sequence_hole_pseudo_period` is the period in packets
    /// between intentional ACK gaps used to fingerprint optimistic
    /// peers).
    pub fn set_optimistic_ack_policy(&mut self, _sequence_hole_pseudo_period: u32) {
        todo!()
    }

    /// Toggle preemptive-repeat at the context level.
    pub fn set_preemptive_repeat_policy(&mut self, _do_repeat: bool) {
        todo!()
    }
}

impl Connection {
    /// Override preemptive-repeat for this connection.
    pub fn set_preemptive_repeat(&mut self, _do_repeat: bool) {
        todo!()
    }

    /// Enable keep-alives at the given interval (microseconds).
    pub fn enable_keep_alive(&mut self, _interval: u64) {
        todo!()
    }

    /// Disable any previously-enabled keep-alive.
    pub fn disable_keep_alive(&mut self) {
        todo!()
    }

    /// Returns `true` for client-initiated connections.
    pub fn is_client(&self) -> bool {
        todo!()
    }

    /// Local error code reported on close (0 if none).
    pub fn local_error(&self) -> u64 {
        todo!()
    }

    /// Remote error code reported on close.
    pub fn remote_error(&self) -> u64 {
        todo!()
    }

    /// Application-level error reported on close.
    pub fn application_error(&self) -> u64 {
        todo!()
    }

    /// Per-stream error reported by the peer.
    pub fn remote_stream_error(&self, _stream_id: u64) -> u64 {
        todo!()
    }

    /// Total bytes of stream data sent on this connection.
    pub fn data_sent(&self) -> u64 {
        todo!()
    }

    /// Total bytes of stream data received on this connection.
    pub fn data_received(&self) -> u64 {
        todo!()
    }

    /// `true` while the connection still streams events into its
    /// log (capped per [`Quic::set_max_simultaneous_logs`]).
    pub fn is_still_logging(&self) -> bool {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Congestion-control registry.
//
// The registry itself is process-wide, so the entry points stay as
// free functions; the per-context / per-connection setters fold
// into methods.

/// Register a slice of congestion-control algorithms with the
/// global registry.  The C signature took `CongestionAlgorithm
/// const**` plus a length; the Rust shape collapses both into a
/// borrowed slice with `'static` element lifetime — algorithms are
/// typically file-scope statics, mirroring how
/// `register_all_cc_algorithms.c` builds the list.
pub fn register_congestion_control_algorithms(_alg: &'static [&'static CongestionAlgorithm]) {
    todo!()
}

/// Convenience wrapper around
/// [`register_congestion_control_algorithms`] that pulls in every
/// algorithm shipped with the crate.
pub fn register_all_congestion_control_algorithms() {
    todo!()
}

/// Look up a registered algorithm by name (`alg_id`).
pub fn get_congestion_algorithm(_alg_id: &str) -> Option<&'static CongestionAlgorithm> {
    todo!()
}

impl Quic {
    /// Set the default congestion-control algorithm applied to new
    /// connections on this context.
    pub fn set_default_congestion_algorithm(&mut self, _algo: &'static CongestionAlgorithm) {
        todo!()
    }

    /// Same as [`Self::set_default_congestion_algorithm`] but
    /// passes through a per-algorithm options string.
    pub fn set_default_congestion_algorithm_ex(
        &mut self,
        _alg: &'static CongestionAlgorithm,
        _alg_option_string: Option<&str>,
    ) {
        todo!()
    }

    /// Convenience: select the default algorithm by name (looking up
    /// in the registry).
    pub fn set_default_congestion_algorithm_by_name(&mut self, _alg_name: &str) {
        todo!()
    }
}

impl Connection {
    /// Override the congestion-control algorithm for this
    /// connection.
    pub fn set_congestion_algorithm(&mut self, _algo: &'static CongestionAlgorithm) {
        todo!()
    }

    /// Same as [`Self::set_congestion_algorithm`] but passes
    /// through a per-algorithm options string.
    pub fn set_congestion_algorithm_ex(
        &mut self,
        _alg: &'static CongestionAlgorithm,
        _alg_option_string: Option<&str>,
    ) {
        todo!()
    }

    /// Set the priority limit above which streams bypass congestion
    /// control.
    pub fn set_priority_limit_for_bypass(&mut self, _priority_limit: u8) {
        todo!()
    }

    /// Toggle whether the application receives feedback-loss
    /// notifications.
    pub fn set_feedback_loss_notification(&mut self, _should_notify: bool) {
        todo!()
    }

    /// Force the next probe upward (BBR / Cubic probe-up).
    pub fn request_forced_probe_up(&mut self, _request_forced_probe_up: bool) {
        todo!()
    }

    /// Subscribe to pacing-rate change notifications, with the
    /// given relative thresholds.
    pub fn subscribe_pacing_rate_updates(
        &mut self,
        _decrease_threshold: u64,
        _increase_threshold: u64,
    ) {
        todo!()
    }

    /// Current pacing rate (bytes per second).
    pub fn pacing_rate(&self) -> u64 {
        todo!()
    }

    /// Current congestion window (bytes).
    pub fn cwin(&self) -> u64 {
        todo!()
    }

    /// Smoothed round-trip-time estimate (microseconds).
    pub fn rtt(&self) -> u64 {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// ECH / ESNI.

impl Quic {
    /// Configure server-side Encrypted-ClientHello (ECH) by loading
    /// the private key and config from disk.
    pub fn ech_configure(
        &mut self,
        _ech_private_key_file_name: Option<&str>,
        _ech_config_file_name: Option<&str>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Release any installed ECH context.
    pub fn release_ech_ctx(&mut self) {
        todo!()
    }
}

impl Connection {
    /// Configure client-side ECH on this connection.
    pub fn ech_configure_client(&mut self, _config_data: &[u8]) -> Result<(), Error> {
        todo!()
    }

    /// Returns `true` when the handshake used ECH.
    pub fn is_ech_handshake(&self) -> bool {
        todo!()
    }

    /// Borrow the retry-config bytes the server returned (empty
    /// when no retry config is available).  Two C `uint8_t**` /
    /// `size_t*` output parameters fold into this single borrow.
    pub fn ech_retry_config(&self) -> &[u8] {
        todo!()
    }
}

/// Generate a fresh ECH config file on disk.
pub fn ech_create_config_file(
    _public_name: &str,
    _private_key_file: &str,
    _ech_config_file: &str,
) -> Result<(), Error> {
    todo!()
}

// ---------------------------------------------------------------------------
// Base64 helpers.

/// C: `base64_decode`.  The C signature output an owned
/// buffer via `uint8_t** v` + `size_t* v_len`; the Rust translation
/// returns the decoded bytes by value.
pub fn base64_decode(_b64_txt: &str) -> Result<Vec<u8>, Error> {
    todo!()
}

/// C: `base64_encode`.  The C signature wrote into a
/// caller-supplied buffer with a fallible "buffer too small" path;
/// the Rust translation owns the result `String` and reports
/// errors only when the input is malformed.
pub fn base64_encode(_v: &[u8]) -> Result<String, Error> {
    todo!()
}

#[cfg(test)]
mod test {}
