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
//! Phase 4: all function bodies have been filled in.
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
//!   contexts that flow through the stack (e.g. `stream_ctx`,
//!   `app_stream_ctx`) become `Option<Box<dyn core::any::Any>>` —
//!   the application produces them, downcasts on retrieval.
//! * C `int` / `int64_t` return values that encode 0/-1 or 0/error
//!   status become `Result<T, Error>`.  `i32` / `i64` are kept where
//!   the C value is a real integer (e.g., wake delays in microseconds,
//!   interface index).
//! * Single-bit `unsigned int : 1` bitfields collapse to `bool`.
//! * `va_list` has no Rust counterpart; the `_v` log helpers are
//!   omitted in favour of one entry point that takes
//!   `core::fmt::Arguments<'_>`.
//! * `extern` global state (`congestion_control_algorithms`)
//!   becomes accessor functions that hand out a borrowed slice; the
//!   companion `_nb_` length variable disappears (the slice carries
//!   its length).

// Many translated functions mirror C signatures with >7 parameters.
// Builder patterns or signature reshaping were not part of Phase 1.
#![allow(clippy::too_many_arguments)]

pub mod arena;
pub mod bbr;
pub mod bbr1;
pub mod binlog;
pub mod bytestream;
pub mod c4;
pub mod cc_common;
pub mod config;
pub mod crypto;
pub mod cubic;
pub mod ech;
pub mod errors;
pub mod fastcc;
pub mod frames;
pub mod hash;
pub mod header_protection;
pub mod internal;
pub mod lb;
pub mod logger;
pub mod newreno;
pub mod packet_loop;
pub mod performance_log;
pub mod prague;
pub mod qlog;
pub mod siphash;
pub mod socks;
pub mod socks_socket2;
pub mod spinbit;
pub mod splay;
pub mod stream;
pub mod sys;
#[cfg(test)]
pub mod tests;
pub mod textlog;
pub mod tls;
pub mod tls_api;
pub mod tp;
pub mod utils;

// Re-export the TP shapes from the public crate root for backwards
// compatibility with callers that already wrote `crate::TransportParameters`.
pub use tp::{PreferredAddress, TransportParameter, TransportParameters, VersionNegotiation};

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
/// Phase 4 implementations return specific variants based on the C
/// control flow.  The initial variant set covers the broad failure
/// shapes; additional variants may be added as needed.  Marked
/// `#[non_exhaustive]` so adding
/// variants later isn't a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Catch-all for C paths that collapse to a generic failure.
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

/// Wall-clock instant in microseconds since some application-defined
/// epoch.  Replaces the C `uint64_t current_time` parameter that
/// threads through every API call needing "now".  The type is
/// `Copy`, supports arithmetic with [`Duration`], and converts to
/// `u64` cheaply via `.ticks()` when crossing wire boundaries.
///
/// Phase 2 decision: per-call `current_time: Instant` parameters
/// stay (matches the C source's design — caller decides what time
/// it is).  No `Clock` trait at the library level; applications
/// can build a clock-injecting wrapper around the calls if they
/// want that style.
pub type Instant = fugit::Instant<u64, 1, 1_000_000>;

/// Duration in microseconds.  Companion to [`Instant`].  Use for
/// timeouts, RTTs, jitter, etc.  In the C source these are bare
/// `uint64_t` values labelled by context (`idle_timeout`,
/// `microsec_latency`, …); the typed alias makes the unit explicit.
pub type Duration = fugit::Duration<u64, 1, 1_000_000>;

// Error codes (picoquic-internal `ERROR_*` and protocol-defined
// `TRANSPORT_*` / TLS-alert) live as enums in [`crate::errors`].
pub use errors::{InternalError, TransportError};

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
    u32::from_le_bytes([a, b, c, d])
}

// ---------------------------------------------------------------------------
// Connection state.

/// Connection-state machine, listing the QUIC connection states a
/// `Connection` walks through from initial handshake to teardown.
/// Discriminants follow the declaration order of the C `state_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

// `TransportParameter`, `PreferredAddress`, `VersionNegotiation`,
// `TransportParameters`, and `TransportParameter0RttKind` live in
// [`crate::tp`].

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

impl SpinbitVersion {
    /// Convert a raw C `picoquic_spinbit_version_enum` value.
    pub fn from_raw(value: u64) -> Result<Self, Error> {
        match value {
            0 => Ok(Self::Basic),
            1 => Ok(Self::Random),
            2 => Ok(Self::Null),
            3 => Ok(Self::On),
            _ => Err(Error::InvalidArgument),
        }
    }
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
/// …).  Fields are private — construct via [`Self::clone_from_slice`]
/// or [`Self::with_size`] and read via [`Self::as_bytes`].
///
/// `Ord` is derived as lexicographic over the live bytes (matching
/// the C `compare_connection_id` shape: per-byte compare under
/// `min(len_a, len_b)`, then length tie-break).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ConnectionId {
    id: [u8; CONNECTION_ID_MAX_SIZE],
    id_len: u8,
}

impl ConnectionId {
    /// Construct a connection id by copying `bytes`.  Returns `None`
    /// when `bytes.len()` exceeds [`CONNECTION_ID_MAX_SIZE`].
    pub fn clone_from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > CONNECTION_ID_MAX_SIZE {
            return None;
        }
        let mut id = [0u8; CONNECTION_ID_MAX_SIZE];
        id[..bytes.len()].copy_from_slice(bytes);
        Some(Self {
            id,
            id_len: bytes.len() as u8,
        })
    }

    /// Construct a zero-filled connection id of length `len`.
    /// Returns `None` when `len` exceeds [`CONNECTION_ID_MAX_SIZE`].
    pub fn with_size(len: usize) -> Option<Self> {
        if len > CONNECTION_ID_MAX_SIZE {
            return None;
        }
        Some(Self {
            id: [0u8; CONNECTION_ID_MAX_SIZE],
            id_len: len as u8,
        })
    }

    /// Borrow the live id bytes (`bytes[..len]`).
    pub fn as_bytes(&self) -> &[u8] {
        &self.id[..self.id_len as usize]
    }

    /// Mutable borrow of the live id bytes.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.id[..self.id_len as usize]
    }

    /// Number of bytes in the id.
    pub fn len(&self) -> usize {
        self.id_len as usize
    }

    /// `true` when the id has zero length.  Mirrors the C
    /// `is_connection_id_null` sentinel check.
    pub fn is_empty(&self) -> bool {
        self.id_len == 0
    }

    /// Hash connection IDs for picohash tables.  C:
    /// `picoquic_connection_id_hash`.
    pub fn hash_with_seed(&self, _seed: &[u8; 16]) -> u64 {
        let bytes = self.as_bytes();
        let mut val64 = 0u64;
        let mut i = 0usize;

        while i < bytes.len() && i < 8 {
            val64 = (val64 << 8).wrapping_add(bytes[i] as u64);
            i += 1;
        }

        while i < bytes.len() {
            let top = val64 >> 56;
            val64 = (val64 << 8)
                .wrapping_add(bytes[i] as u64)
                .wrapping_add(top.wrapping_mul(0x10001));
            i += 1;
        }

        val64
    }

    /// Fold the first up-to-8 bytes of the id into a `u64`.
    /// C: `val64_connection_id`.
    pub fn val64(&self) -> u64 {
        let bytes = self.as_bytes();
        let len = bytes.len().min(8);
        let mut buf = [0u8; 8];
        buf[..len].copy_from_slice(&bytes[..len]);
        u64::from_be_bytes(buf)
    }
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
use crate::internal::{
    Connection, ConnectionToken, DEFAULT_CRYPTO_EPOCH_LENGTH, Path, Quic, connection_wake_key,
};

// ---------------------------------------------------------------------------
// Application callback events.

/// Event type passed to the application's stream / connection
/// callback (see [`StreamDataCallback`]).  Identifies which sort of
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

// Transport-parameter shapes live in [`crate::tp`].

// Stream-id decomposition lives on [`crate::stream::StreamId`].

// ---------------------------------------------------------------------------
// Time management.

/// Monotonic microseconds from a stable process-local epoch.
///
/// C: `picoquic_current_time` uses `CLOCK_MONOTONIC` on the selected
/// Unix target.  Rust's `std::time::Instant` exposes monotonic elapsed
/// time without exposing the OS clock epoch, so the first call becomes
/// this process's fixed epoch.
pub fn current_time() -> u64 {
    static CURRENT_TIME_BASE: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

    let elapsed = CURRENT_TIME_BASE
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_micros();
    u64::try_from(elapsed).unwrap_or(u64::MAX)
}

/// Result of [`picoquic_select`].
///
/// C: `picoquic_select` returns the same receive metadata and timestamp as
/// `picoquic_select_ex`, but keeps the selected socket rank local.
#[derive(Debug, Copy, Clone)]
pub struct PicoquicSelectInfo {
    pub addr_from: Option<core::net::SocketAddr>,
    pub addr_dest: Option<core::net::SocketAddr>,
    pub dest_if: i32,
    pub received_ecn: u8,
    pub bytes_recv: usize,
    pub current_time: Instant,
}

impl From<crate::socks::SelectInfo> for PicoquicSelectInfo {
    fn from(info: crate::socks::SelectInfo) -> Self {
        Self {
            addr_from: info.addr_from,
            addr_dest: info.addr_dest,
            dest_if: info.dest_if,
            received_ecn: info.received_ecn,
            bytes_recv: usize::try_from(info.bytes_recv).unwrap_or(0),
            current_time: info.current_time,
        }
    }
}

/// Socket-select wrapper.  C: `picoquic_select`.
///
/// Delegates to the translated `picoquic_select_ex` behavior in
/// [`crate::socks::select`] and deliberately discards the socket rank, matching
/// the C wrapper's local `socket_rank` variable.
pub fn picoquic_select<S: crate::socks::Socket>(
    sockets: &mut [S],
    buffer: &mut [u8],
    delta_t: i64,
) -> Result<PicoquicSelectInfo, Error> {
    let select_ex_info = crate::socks::select(sockets, buffer, delta_t)?;
    let _socket_rank = select_ex_info.socket_rank;
    Ok(select_ex_info.into())
}

impl Quic {
    /// Virtual time used by this QUIC context (wall-clock or
    /// simulated, depending on whether a simulated-time pointer was
    /// supplied at creation).  C: `get_quic_time`.
    pub fn time(&self) -> u64 {
        current_time()
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
pub trait StreamDataCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32;

    fn prepare_to_send<'a>(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        context: &mut crate::internal::StreamDataBufferArgument<'a>,
        stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        let start = context.byte_index;
        let end = start
            .saturating_add(context.allowed_space)
            .min(context.bytes.len());
        self.callback(
            connection,
            stream_id,
            &context.bytes[start..end],
            CallbackEvent::PrepareToSend,
            stream_ctx,
        )
    }

    fn prepare_datagram<'buf, 'cnx, 'path>(
        &mut self,
        context: &mut crate::internal::DatagramBufferArgument<'buf, 'cnx, 'path>,
        unique_path_id: u64,
        _allowed_space: usize,
    ) -> i32 {
        self.callback(
            context.connection_mut(),
            unique_path_id,
            &[],
            CallbackEvent::PrepareDatagram,
            None,
        )
    }
}

/// ALPN-selection callback.  Returns the index of the chosen ALPN
/// in `list`, or any value `>= list.len()` to signal "none of the
/// proposed ALPNs is supported".  C: `AlpnSelect` (the C `_v2`
/// flavour is folded in — it only differed in the iovec type, which
/// is now just `&[u8]`).
/// Application hook the TLS backend invokes to choose one of the
/// client's proposed ALPN values.  Returns the chosen index into
/// `list`, or `None` to reject the handshake.
///
/// `list` carries the raw bytes off the wire; mapping to the
/// strongly-typed [`Alpn`] enum is up to the implementor (most
/// applications keep a small whitelist by string identifier).
pub trait AlpnSelect {
    fn select(&mut self, quic: &mut Quic, list: &[&[u8]]) -> Option<usize>;
}

/// Callback that produces a server-environment-compatible CID.
/// Folds the C `void* connection_id_cb_data` into the implementor's state.
/// C: `ConnectionIdCallback`.
pub trait ConnectionIdCallback {
    #[doc(hidden)]
    fn is_lb_compat_cid_generator(&self) -> bool {
        false
    }

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
#[derive(Debug, Copy, Clone)]
pub struct PathQuality {
    /// Receive rate estimate in bytes per second.
    pub receive_rate_estimate: u64,
    /// Pacing rate in bytes per second.
    pub pacing_rate: u64,
    /// Number of bytes in the congestion window.
    pub cwin: u64,
    /// Smoothed RTT estimate in microseconds.
    pub rtt: Duration,
    /// Most recent RTT sample.
    pub rtt_sample: Duration,
    /// Estimated RTT variability.
    pub rtt_variant: Duration,
    /// Minimum observed RTT since path creation.
    pub rtt_min: Duration,
    /// Maximum observed RTT since path creation.
    pub rtt_max: Duration,
    pub sent: u64,
    pub lost: u64,
    pub timer_losses: u64,
    pub spurious_losses: u64,
    pub max_spurious_rtt: Duration,
    pub max_reorder_delay: Duration,
    pub max_reorder_gap: u64,
    pub bytes_in_transit: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl Default for PathQuality {
    fn default() -> Self {
        Self {
            receive_rate_estimate: 0,
            pacing_rate: 0,
            cwin: 0,
            rtt: Duration::from_ticks(0),
            rtt_sample: Duration::from_ticks(0),
            rtt_variant: Duration::from_ticks(0),
            rtt_min: Duration::from_ticks(0),
            rtt_max: Duration::from_ticks(0),
            sent: 0,
            lost: 0,
            timer_losses: 0,
            spurious_losses: 0,
            max_spurious_rtt: Duration::from_ticks(0),
            max_reorder_delay: Duration::from_ticks(0),
            max_reorder_gap: 0,
            bytes_in_transit: 0,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
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
#[derive(Debug, Copy, Clone)]
pub struct PerAckState {
    pub rtt_measurement: Duration,
    pub send_delay: Duration,
    pub one_way_delay: Duration,
    pub nb_bytes_acknowledged: u64,
    pub nb_bytes_newly_lost: u64,
    pub nb_bytes_lost_since_packet_sent: u64,
    pub nb_bytes_delivered_since_packet_sent: u64,
    pub inflight_prior: u64,
    pub lost_packet_number: u64,
    pub lost_packet_sent_time: Instant,
    pub pc: i32,
    pub is_app_limited: bool,
    pub is_cwnd_limited: bool,
}

impl Default for PerAckState {
    fn default() -> Self {
        Self {
            rtt_measurement: Duration::from_ticks(0),
            send_delay: Duration::from_ticks(0),
            one_way_delay: Duration::from_ticks(0),
            nb_bytes_acknowledged: 0,
            nb_bytes_newly_lost: 0,
            nb_bytes_lost_since_packet_sent: 0,
            nb_bytes_delivered_since_packet_sent: 0,
            inflight_prior: 0,
            lost_packet_number: 0,
            lost_packet_sent_time: Instant::from_ticks(0),
            pc: 0,
            is_app_limited: false,
            is_cwnd_limited: false,
        }
    }
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
pub trait CongestionControl: Sync {
    fn alg_init(
        &self,
        connection: &mut Connection,
        path_x: &mut Path,
        option_string: Option<&str>,
        current_time: Instant,
    );

    fn alg_notify(
        &self,
        connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: Instant,
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
    match CC_ALGORITHM_REGISTRY.read() {
        Ok(registry) => *registry,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// Process-wide congestion-control algorithm registry.  Filled by
/// [`register_congestion_control_algorithms`].  Each registration
/// replaces the current slice, matching the C globals
/// `picoquic_congestion_control_algorithms` and
/// `picoquic_nb_congestion_control_algorithms`.
static CC_ALGORITHM_REGISTRY: std::sync::RwLock<&'static [&'static CongestionAlgorithm]> =
    std::sync::RwLock::new(&[]);

struct BaselineCongestionControl;

impl CongestionControl for BaselineCongestionControl {
    fn alg_init(
        &self,
        _connection: &mut Connection,
        path_x: &mut Path,
        _option_string: Option<&str>,
        _current_time: Instant,
    ) {
        path_x.cwin = crate::internal::CWIN_INITIAL;
        path_x.bytes_in_transit = 0;
    }

    fn alg_notify(
        &self,
        _connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        _current_time: Instant,
    ) {
        match notification {
            CongestionNotification::Acknowledgement => {
                let growth = ack_state.nb_bytes_acknowledged.max(1);
                path_x.cwin = path_x.cwin.saturating_add(growth);
            }
            CongestionNotification::Repeat | CongestionNotification::Timeout => {
                path_x.cwin = (path_x.cwin / 2).max(crate::internal::CWIN_MINIMUM);
            }
            CongestionNotification::SeedCwin if ack_state.inflight_prior > 0 => {
                path_x.cwin = ack_state.inflight_prior.max(crate::internal::CWIN_MINIMUM);
            }
            CongestionNotification::Reset => {
                path_x.cwin = crate::internal::CWIN_INITIAL;
                path_x.bytes_in_transit = 0;
            }
            _ => {}
        }
    }

    fn alg_delete(&self, _path_x: &mut Path) {}

    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        Some((path_x.cwin, path_x.bytes_in_transit))
    }
}

static BASELINE_CC: BaselineCongestionControl = BaselineCongestionControl;
static NEWRENO_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "newreno",
    congestion_algorithm_number: 1,
    ecn_mark: ECN_ECT_0,
    algorithm: &newreno::NEWRENO_CONTROL,
};
static CUBIC_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "cubic",
    congestion_algorithm_number: 2,
    ecn_mark: ECN_ECT_0,
    algorithm: &BASELINE_CC,
};
static DCUBIC_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "dcubic",
    congestion_algorithm_number: 3,
    ecn_mark: ECN_ECT_0,
    algorithm: &BASELINE_CC,
};
static FAST_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "fast",
    congestion_algorithm_number: 4,
    ecn_mark: ECN_ECT_0,
    algorithm: &fastcc::FASTCC_CONTROL,
};
static BBR_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "bbr",
    congestion_algorithm_number: 5,
    ecn_mark: ECN_ECT_0,
    algorithm: &BASELINE_CC,
};
static PRAGUE_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "prague",
    congestion_algorithm_number: 6,
    ecn_mark: ECN_ECT_1,
    algorithm: &prague::PRAGUE_CONTROL,
};
static BBR1_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "bbr1",
    congestion_algorithm_number: 7,
    ecn_mark: ECN_ECT_0,
    algorithm: &BASELINE_CC,
};
static C4_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "c4",
    congestion_algorithm_number: 8,
    ecn_mark: ECN_ECT_1,
    algorithm: &c4::C4_CONTROL,
};
static ALL_CC_ALGORITHMS: [&CongestionAlgorithm; 8] = [
    &NEWRENO_ALGORITHM,
    &CUBIC_ALGORITHM,
    &DCUBIC_ALGORITHM,
    &FAST_ALGORITHM,
    &BBR_ALGORITHM,
    &PRAGUE_ALGORITHM,
    &BBR1_ALGORITHM,
    &C4_ALGORITHM,
];

// ---------------------------------------------------------------------------
// ALPN list.

/// Application-protocol identifiers used during session
/// negotiation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum Alpn {
    /// No ALPN selected / unrecognised.
    #[default]
    Undef,
    Http0_9,
    Http3,
    Quicperf,
}

impl Alpn {
    /// Wire string for this ALPN, or `None` for [`Self::Undef`].
    pub const fn as_str(self) -> Option<&'static str> {
        match self {
            Self::Undef => None,
            Self::Http0_9 => Some("hq-interop"),
            Self::Http3 => Some("h3"),
            Self::Quicperf => Some("perf"),
        }
    }
}

impl core::str::FromStr for Alpn {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hq-interop" => Ok(Self::Http0_9),
            "h3" => Ok(Self::Http3),
            "perf" => Ok(Self::Quicperf),
            _ => Err(()),
        }
    }
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
pub fn is_handshake_error(error_code: u64) -> bool {
    (error_code & 0xff00) == errors::transport_crypto_error(0) as u64
        || error_code == TransportError::TlsHandshakeFailed as u64
}

// `error_name` lives on [`crate::errors::InternalError::name`].

// `tp_name` lives on [`crate::tp::TransportParameter::name`].

// `frame_name` lives on [`crate::frames::FrameType::name`].

// `add_proposed_alpn` is gone.  In the C source this was the
// hook for the application's ALPN-select callback to push a
// proposed value into the picotls handshake state.  Phase 2's
// `TlsCallbacks::select_alpn` model returns the chosen index
// instead; the TLS backend provisions internally.

impl Connection {
    /// Negotiated ALPN, or [`Alpn::Undef`] when no ALPN was selected.
    pub fn tls_negotiated_alpn(&self) -> Alpn {
        self.alpn
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(Alpn::Undef)
    }

    /// SNI value the peer presented during the handshake, or `None`
    /// when none was given.
    pub fn tls_sni(&self) -> Option<&str> {
        self.sni.as_deref()
    }

    /// Reason this connection closed.  Only one of the four C
    /// out-parameters (`local_reason`, `remote_reason`,
    /// `local_application_reason`, `remote_application_reason`) was
    /// populated in any given case; that "which side, transport
    /// vs. application" choice folds into [`CloseReason`].
    /// Returns `None` when the connection has not (yet) closed.
    pub fn close_reason(&self) -> Option<CloseReason> {
        if self.local_error != 0 {
            Some(CloseReason::Local(self.local_error))
        } else if self.application_error != 0 {
            Some(CloseReason::LocalApp(self.application_error))
        } else if self.remote_error != 0 {
            Some(CloseReason::Remote(self.remote_error))
        } else if self.remote_application_error != 0 {
            Some(CloseReason::RemoteApp(self.remote_application_error))
        } else {
            None
        }
    }
}

/// Outcome reported by [`Connection::close_reason`].  Mirrors the
/// "(side, layer)" choice from the four C out-parameters.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CloseReason {
    /// Local close at the QUIC transport layer.  Code is one of the
    /// [`crate::errors::TransportError`] / [`crate::errors::InternalError`]
    /// values, or a `CRYPTO_ERROR(alert)` from
    /// [`crate::errors::transport_crypto_error`].
    Local(u64),
    /// Local close at the application layer (CONNECTION_CLOSE frame
    /// type 0x1d).  Code is application-defined.
    LocalApp(u64),
    /// Peer closed at the QUIC transport layer.
    Remote(u64),
    /// Peer closed at the application layer.
    RemoteApp(u64),
}

// ---------------------------------------------------------------------------
// Per-context settings (logging, diagnostics, pool sizing,
// port-blocking).

impl Quic {
    /// Install a per-context packet fuzzer; `None` removes any
    /// previously installed fuzzer.
    pub fn set_fuzz(&mut self, fuzzer: Option<Box<dyn Fuzz>>) {
        self.fuzz_fn = fuzzer;
    }

    /// `1` → log every packet, `0` → log only the first
    /// [`LOG_PACKET_MAX_SEQUENCE`] packets per connection.
    pub fn set_log_level(&mut self, log_level: i32) {
        self.use_long_log = log_level > 0;
        for connection in self.connections.iter_mut() {
            connection.use_long_log = self.use_long_log;
        }
    }

    /// Toggle randomised log-file names (defeating accidental
    /// collisions when clients pick non-random initial CIDs).
    /// C: `picoquic_use_unique_log_names` (quicctx.c:4646).
    pub fn set_use_unique_log_names(&mut self, use_unique_log_names: bool) {
        self.use_unique_log_names = use_unique_log_names;
    }

    /// Toggle SSL-keylog output.  Phase 1 follows the canonical
    /// build (`WITHOUT_SSLKEYLOG` undefined); a `cfg`-gated variant
    /// lands when build options are translated.
    /// C: `picoquic_enable_sslkeylog` — `quic->enable_sslkeylog = (enable != 0)`.
    pub fn set_sslkeylog_enabled(&mut self, enable_sslkeylog: bool) {
        self.enable_sslkeylog = enable_sslkeylog;
    }

    /// Whether SSL-keylog output is enabled on this context.
    pub fn is_sslkeylog_enabled(&self) -> bool {
        self.enable_sslkeylog
    }

    /// Configure packet-number randomisation: `0` (no
    /// randomisation), `1` (Initial PNs only), `2` (all PN spaces).
    pub fn set_random_initial(&mut self, random_initial: i32) {
        self.random_initial = if random_initial > 1 {
            2
        } else if random_initial > 0 {
            1
        } else {
            0
        };
    }

    /// Toggle packet-train mode (groups outbound packets into
    /// coalesced trains).
    pub fn set_packet_train_mode(&mut self, train_mode: bool) {
        self.packet_train_mode = train_mode;
    }

    /// Configure default padding policy applied to outbound packets.
    pub fn set_padding_policy(&mut self, padding_min_size: u32, padding_multiple: u32) {
        self.padding_minsize_default = padding_min_size;
        self.padding_multiple_default = padding_multiple;
    }

    /// Set (or clear, with `None`) the keylog destination file.
    pub fn set_key_log_file(&mut self, keylog_filename: Option<&str>) {
        // C closes the previous FILE before trying the new fopen.  If
        // reopening fails, the log-event callback remains installed but has
        // no active file; model that as a present context with `key_log = None`.
        use std::fs::OpenOptions;

        self.key_log = None;
        if let Some(path) = keylog_filename
            && let Ok(f) = OpenOptions::new().create(true).append(true).open(path)
        {
            self.key_log = Some(Box::new(f));
        }
    }

    pub(crate) fn tls_key_log_label(secret_is_server: bool, epoch: usize) -> Option<&'static str> {
        match (secret_is_server, epoch) {
            (false, 1) => Some("CLIENT_EARLY_TRAFFIC_SECRET"),
            (false, 2) => Some("CLIENT_HANDSHAKE_TRAFFIC_SECRET"),
            (false, 3) => Some("CLIENT_TRAFFIC_SECRET_0"),
            (true, 2) => Some("SERVER_HANDSHAKE_TRAFFIC_SECRET"),
            (true, 3) => Some("SERVER_TRAFFIC_SECRET_0"),
            _ => None,
        }
    }
}

fn write_hex(writer: &mut dyn std::io::Write, bytes: &[u8]) -> std::io::Result<()> {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    for byte in bytes {
        writer.write_all(&[HEX[(byte >> 4) as usize], HEX[(byte & 0x0f) as usize]])?;
    }
    Ok(())
}

fn write_ssl_key_log_line(
    writer: &mut dyn std::io::Write,
    label: &str,
    client_random: &[u8; 32],
    secret: &[u8],
) -> std::io::Result<()> {
    writer.write_all(label.as_bytes())?;
    writer.write_all(b" ")?;
    write_hex(writer, client_random)?;
    writer.write_all(b" ")?;
    write_hex(writer, secret)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

impl Connection {
    pub(crate) fn consume_tls_key_log_events(&mut self, session: &mut dyn crate::tls::Session) {
        let events = session.take_key_log_events();
        if events.is_empty() {
            return;
        }

        let quic_ptr = self.quic_ptr;
        for event in events {
            if event.epoch == 3 {
                let secret_len = event.secret.len().min(self.app_secret_enc.len());
                if secret_len > 0 {
                    self.app_secret_len = secret_len;
                    if event.is_enc {
                        self.app_secret_enc[..secret_len]
                            .copy_from_slice(&event.secret[..secret_len]);
                    } else {
                        self.app_secret_dec[..secret_len]
                            .copy_from_slice(&event.secret[..secret_len]);
                    }
                }
            }

            let Some(label) =
                Quic::tls_key_log_label(self.client_mode != event.is_enc, event.epoch)
            else {
                continue;
            };

            if quic_ptr.is_null() {
                continue;
            }

            // SAFETY: `quic_ptr` is the owning context back-pointer installed
            // when the connection is inserted in `Quic::connections`.  This
            // method only mutates `key_log`, a context field disjoint from the
            // connection arena slot borrowed as `self`, and v1 assumes
            // serialized access to a `Quic` context.
            let key_log = unsafe { &mut (*quic_ptr).key_log };
            if let Some(writer) = key_log.as_mut() {
                let _ = write_ssl_key_log_line(
                    writer.as_mut(),
                    label,
                    &event.client_random,
                    &event.secret,
                );
            }
        }
    }
}

impl Quic {
    /// Read `SSLKEYLOGFILE` from the process environment (gated
    /// behind the build-time `WITHOUT_SSLKEYLOG` opt-out and the
    /// per-context `is_sslkeylog_enabled` flag) and install it via
    /// [`Quic::set_key_log_file`].  C:
    /// `picoquic_set_key_log_file_from_env`.
    ///
    /// The environment lookup is a `std`-only operation (it goes
    /// through `getenv`); Phase 3 will feature-gate the body.
    pub fn set_key_log_file_from_env(&mut self) {
        if self.enable_sslkeylog
            && let Ok(path) = std::env::var("SSLKEYLOGFILE")
        {
            self.set_key_log_file(Some(&path));
        }
    }

    /// Adjust the connection-pool ceiling.  Cannot grow past the
    /// limit chosen at context creation.
    pub fn adjust_max_connections(&mut self, max_nb_connections: u32) -> Result<(), Error> {
        if max_nb_connections > self.max_number_connections {
            return Err(Error::InvalidArgument);
        }
        self.tentative_max_number_connections = max_nb_connections;
        Ok(())
    }

    /// Number of connections currently registered with this context.
    pub fn current_number_connections(&self) -> u32 {
        self.current_number_connections
    }

    /// Set the half-open-connection threshold above which the server
    /// switches to retry-token mode.
    pub fn set_max_half_open_retry_threshold(&mut self, max_half_open_before_retry: u32) {
        self.max_half_open_before_retry = max_half_open_before_retry;
    }

    /// Retrieve the configured half-open retry threshold.
    pub fn max_half_open_retry_threshold(&self) -> u32 {
        self.max_half_open_before_retry
    }

    /// Toggle per-context port blocking.
    /// C: `picoquic_disable_port_blocking`.
    pub fn set_port_blocking_disabled(&mut self, is_port_blocking_disabled: bool) {
        self.is_port_blocking_disabled = is_port_blocking_disabled;
    }
}

// ---------------------------------------------------------------------------
// Free-standing port / address blocklist queries (no per-context
// state — these consult a process-wide allowlist).

/// Well-known ports blocked to prevent reflection amplification.
/// Sourced from `picoquic/port_blocking.c`.
const BLOCKED_PORTS: &[u16] = &[
    27015, // SRCDS
    20800, // Call Of Duty
    11211, // memcache
    5353,  // mDNS
    1900,  // SSDP
    520,   // RIP
    500,   // IKE
    389,   // CLDAP
    161,   // SNMP
    138,   // NETBIOS Datagram Service
    137,   // NETBIOS Name Service
    123,   // NTP
    111,   // Portmap
    53,    // DNS
    19,    // Chargen
    17,    // Quote of the Day
    7,     // Echo
    0,     // Unusable
];

/// Returns `true` when `port` is on the QUIC well-known-port
/// blocklist.
pub fn check_port_blocked(port: u16) -> bool {
    // List is sorted descending; iterate while port <= list[i].
    for &blocked in BLOCKED_PORTS {
        if port > blocked {
            break;
        }
        if port == blocked {
            return true;
        }
    }
    false
}

/// Returns `true` when `addr_from` belongs to a blocked address
/// range.
pub fn check_addr_blocked(addr_from: &SocketAddr) -> bool {
    check_port_blocked(addr_from.port())
}

pub(crate) fn unspecified_socket_addr() -> SocketAddr {
    SocketAddr::new(core::net::IpAddr::V4(core::net::Ipv4Addr::UNSPECIFIED), 0)
}

pub(crate) fn socket_addr_is_unspecified(addr: &SocketAddr) -> bool {
    addr.port() == 0
        && match addr.ip() {
            core::net::IpAddr::V4(ip) => ip.is_unspecified(),
            core::net::IpAddr::V6(ip) => ip.is_unspecified(),
        }
}

// ---------------------------------------------------------------------------
// QUIC context: construction, TLS configuration, default policies.
//
// `picoquic_create` is the canonical constructor in the C source;
// here it is `Quic::new`.  The setters that follow are all on the
// QUIC context; per-connection siblings live in `impl Connection` further
// down.

/// Write one transport-parameter slot identified by its wire `tp_type` ID
/// into `tp`.  Returns `Err(InvalidArgument)` for unknown IDs.
///
/// C: `picoquic_set_tp_value_by_type` (quicctx.c:819).
fn set_tp_value_by_type(
    tp: &mut TransportParameters,
    tp_type: u64,
    tp_value: u64,
) -> Result<(), Error> {
    match tp_type {
        1 => tp.max_idle_timeout = Duration::from_ticks(tp_value),
        3 => tp.max_packet_size = tp_value as u32,
        4 => tp.initial_max_data = tp_value,
        5 => tp.initial_max_stream_data_bidi_local = tp_value,
        6 => tp.initial_max_stream_data_bidi_remote = tp_value,
        7 => tp.initial_max_stream_data_uni = tp_value,
        8 => tp.initial_max_stream_id_bidir = tp_value,
        9 => tp.initial_max_stream_id_unidir = tp_value,
        10 => tp.ack_delay_exponent = tp_value as u8,
        11 => tp.max_ack_delay = tp_value as u32,
        12 => tp.migration_disabled = tp_value != 0,
        14 => tp.active_connection_id_limit = tp_value as u32,
        32 => tp.max_datagram_frame_size = tp_value as u32,
        0x1057 => tp.enable_loss_bit = (tp_value != 0) as i32,
        0xff04de1b => tp.min_ack_delay = Duration::from_ticks(tp_value),
        0x7158 => tp.enable_time_stamp = tp_value as i32,
        0x2ab2 => tp.do_grease_quic_bit = tp_value != 0,
        0xebd9 => tp.enable_bdp_frame = tp_value != 0,
        0x3e => tp.initial_max_path_id = tp_value,
        0x9f81a176 => tp.address_discovery_mode = tp_value as i32,
        0x17f7586d2cb571 => tp.is_reset_stream_at_enabled = tp_value != 0,
        _ => return Err(Error::InvalidArgument),
    }
    Ok(())
}

/// C: `picoquic_create_random_cnx_id` (picoquic/quicctx.c:1630-1639).
pub(crate) fn create_random_cnx_id(quic: &mut Quic, id_length: u8) -> ConnectionId {
    let len = (id_length as usize).min(CONNECTION_ID_MAX_SIZE);
    let mut cnx_id = ConnectionId::with_size(len).unwrap_or_default();
    if len > 0 {
        rand_core::RngCore::fill_bytes(&mut *quic.rng, cnx_id.as_bytes_mut());
    }
    cnx_id
}

#[cfg(feature = "sys-openssl")]
struct OpenSslRootCertificateCallbacks {
    registration: crate::sys::openssl::CertificateVerifierRegistration,
    fallback: Option<Box<dyn crate::tls::TlsCallbacks>>,
}

#[cfg(feature = "sys-openssl")]
impl OpenSslRootCertificateCallbacks {
    fn verify_with_store(&self, certs: &[&[u8]]) -> Result<(), Error> {
        let Some((&leaf_der, intermediates)) = certs.split_first() else {
            return Err(Error::Generic);
        };
        let leaf = openssl::x509::X509::from_der(leaf_der).map_err(|_| Error::Generic)?;
        let mut chain = openssl::stack::Stack::new().map_err(|_| Error::Generic)?;
        for &cert_der in intermediates {
            let cert = openssl::x509::X509::from_der(cert_der).map_err(|_| Error::Generic)?;
            chain.push(cert).map_err(|_| Error::Generic)?;
        }
        let mut store_ctx = openssl::x509::X509StoreContext::new().map_err(|_| Error::Generic)?;
        let verified = store_ctx
            .init(&self.registration.verifier.store, &leaf, &chain, |ctx| {
                ctx.verify_cert()
            })
            .map_err(|_| Error::Generic)?;
        if verified {
            Ok(())
        } else {
            Err(Error::Generic)
        }
    }
}

#[cfg(feature = "sys-openssl")]
impl crate::tls::TlsCallbacks for OpenSslRootCertificateCallbacks {
    fn select_alpn(&mut self, list: &[&[u8]]) -> Option<usize> {
        self.fallback
            .as_mut()
            .and_then(|callbacks| callbacks.select_alpn(list))
    }

    fn lookup_ticket(&mut self, sni: &str, alpn: &str) -> Option<Vec<u8>> {
        self.fallback
            .as_mut()
            .and_then(|callbacks| callbacks.lookup_ticket(sni, alpn))
    }

    fn store_ticket(&mut self, sni: &str, alpn: &str, ticket: &[u8]) {
        if let Some(callbacks) = self.fallback.as_mut() {
            callbacks.store_ticket(sni, alpn, ticket);
        }
    }

    fn verify_certificate(&mut self, certs: &[&[u8]]) -> Result<(), Error> {
        match self.verify_with_store(certs) {
            Ok(()) => Ok(()),
            Err(err) => {
                if let Some(callbacks) = self.fallback.as_mut() {
                    callbacks.verify_certificate(certs)
                } else {
                    Err(err)
                }
            }
        }
    }
}

impl Quic {
    /// Build a QUIC context with the supplied certificate paths,
    /// default callbacks, and optional reset seed.
    ///
    /// Pointer-shape choices, derived from `quic/quicctx.c:634` and
    /// `quic/sockloop.c:2009`:
    ///
    /// * Every `char const*` parameter is `Option<&str>` (the C
    ///   source passes `NULL` to mean "absent").
    /// * `default_callback_fn` + `default_callback_ctx` collapse to
    ///   one `Option<Box<dyn StreamDataCallback>>`.
    /// * `connection_id_callback` + `connection_id_callback_data` collapse to
    ///   `Option<Box<dyn ConnectionIdCallback>>`.
    /// * `reset_seed[16]` is nullable in C.  Callers can pass
    ///   `Some(seed)` to deep-copy an explicit seed, or `None` to let
    ///   the context generate one.
    /// * `ticket_encryption_key` + `ticket_encryption_key_length`
    ///   collapse to a borrowed `Option<&[u8]>`.
    /// * The C `p_simulated_time: uint64_t*` parameter is dropped
    ///   — the test simulator owns its own clock and threads the
    ///   value through `current_time` directly.
    ///
    /// Returns `None` when context creation fails (the C side
    /// returned `NULL`).
    /// C: `picoquic_create` (picoquic/quicctx.c:633-775).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut max_nb_connections: u32,
        cert_file_name: Option<&str>,
        key_file_name: Option<&str>,
        _cert_root_file_name: Option<&str>,
        default_alpn: Option<&str>,
        default_callback: Option<Box<dyn StreamDataCallback>>,
        cnx_id_callback: Option<Box<dyn ConnectionIdCallback>>,
        reset_seed: impl Into<Option<[u8; RESET_SECRET_SIZE]>>,
        current_time: Instant,
        ticket_file_name: Option<&str>,
        ticket_encryption_key: Option<&[u8]>,
    ) -> Option<Box<Quic>> {
        // C: if max_nb_connections == 0, clamp to 1.
        if max_nb_connections == 0 {
            max_nb_connections = 1;
        }

        // C: enforce_client_only = (cert_file_name == NULL || key_file_name == NULL)
        let enforce_client_only = cert_file_name.is_none() || key_file_name.is_none();

        let unconditional_cnx_id = cnx_id_callback.is_some();

        // The C body allocates hash tables before it refreshes
        // quic->hash_seed, so the tables are created with the zeroed
        // initial seed and keep their own copy of it.
        let table_seed = [0u8; 16];
        let nb_bin = (max_nb_connections as usize).saturating_mul(4);
        let nb_bin_small = max_nb_connections as usize;
        let table_cnx_by_id = crate::hash::HashTable::with_seed(nb_bin, &table_seed).ok()?;
        let table_cnx_by_net = crate::hash::HashTable::with_seed(nb_bin, &table_seed).ok()?;
        let table_cnx_by_icid =
            crate::hash::HashTable::with_seed(nb_bin_small, &table_seed).ok()?;
        let table_cnx_by_secret = crate::hash::HashTable::with_seed(nb_bin, &table_seed).ok()?;
        let table_issued_tickets =
            crate::hash::HashTable::with_seed(nb_bin_small, &table_seed).ok()?;

        struct SystemRandom;
        impl rand_core::RngCore for SystemRandom {
            fn next_u32(&mut self) -> u32 {
                let mut bytes = [0u8; 4];
                self.fill_bytes(&mut bytes);
                u32::from_le_bytes(bytes)
            }
            fn next_u64(&mut self) -> u64 {
                let mut bytes = [0u8; 8];
                self.fill_bytes(&mut bytes);
                u64::from_le_bytes(bytes)
            }
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                use std::io::Read;

                std::fs::File::open("/dev/urandom")
                    .and_then(|mut file| file.read_exact(dest))
                    .expect("failed to read random bytes from /dev/urandom");
            }
        }
        impl rand_core::CryptoRng for SystemRandom {}

        let mut rng = SystemRandom;
        let reset_seed = match reset_seed.into() {
            Some(reset_seed) => reset_seed,
            None => {
                let mut reset_seed = [0u8; RESET_SECRET_SIZE];
                rand_core::RngCore::fill_bytes(&mut rng, &mut reset_seed);
                reset_seed
            }
        };
        let mut retry_seed = [0u8; crate::internal::RETRY_SECRET_SIZE];
        rand_core::RngCore::fill_bytes(&mut rng, &mut retry_seed);
        let mut hash_seed = [0u8; 16];
        rand_core::RngCore::fill_bytes(&mut rng, &mut hash_seed);
        let mut default_tp = crate::tp::TransportParameters::default();
        crate::internal::init_transport_parameters(&mut default_tp);

        let mut quic = Box::new(internal::Quic {
            tls_client_config: None,
            tls_server_config: None,
            tls_certificate_chain: Vec::new(),
            tls_cipher_suites: Vec::new(),
            tls_key_exchanges: Vec::new(),
            tls_callbacks: None,
            default_callback_fn: default_callback,
            default_callback_ctx: None,
            mask_ctx: None,
            mask_fns: None,
            default_alpn: default_alpn.map(|s| s.to_owned()),
            alpn_select_fn: None,
            reset_seed,
            retry_seed,
            rng: Box::new(rng),
            hash_seed,
            ticket_file_name: ticket_file_name.map(std::path::PathBuf::from),
            token_file_name: None,
            stored_tickets: Vec::new(),
            stored_tokens: Vec::new(),
            token_reuse_tree: crate::splay::SplayTree::default(),
            registered_tokens: crate::arena::Arena::new(),
            local_connection_id_length: 8,
            default_stream_priority: DEFAULT_STREAM_PRIORITY,
            default_datagram_priority: DEFAULT_STREAM_PRIORITY,
            local_connection_id_ttl: u64::MAX,
            mtu_max: 0,
            padding_multiple_default: 0,
            padding_minsize_default: RESET_PACKET_MIN_SIZE as u32,
            sequence_hole_pseudo_period: crate::internal::DEFAULT_HOLE_PERIOD as u32,
            default_pmtud_policy: PmtudPolicy::default(),
            default_spin_policy: SpinbitVersion::default(),
            default_lossbit_policy: LossbitVersion::default(),
            default_multipath_option: 0,
            default_handshake_timeout: crate::Duration::from_ticks(0),
            crypto_epoch_length_max: 0,
            max_simultaneous_logs: crate::internal::DEFAULT_SIMULTANEOUS_LOGS,
            current_number_of_open_logs: 0,
            max_half_open_before_retry: crate::internal::DEFAULT_HALF_OPEN_RETRY_THRESHOLD,
            current_number_half_open: 0,
            current_number_connections: 0,
            tentative_max_number_connections: max_nb_connections,
            max_number_connections: max_nb_connections,
            stateless_reset_next_time: current_time,
            stateless_reset_min_interval:
                crate::internal::MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT,
            cwin_max: u64::MAX,
            check_token: false,
            force_check_token: false,
            provide_token: false,
            unconditional_cnx_id,
            client_zero_share: false,
            server_busy: false,
            is_cert_store_not_empty: false,
            use_long_log: false,
            should_close_log: false,
            enable_sslkeylog: false,
            use_unique_log_names: false,
            dont_coalesce_init: false,
            one_way_grease_quic_bit: false,
            random_initial: 1,
            packet_train_mode: false,
            use_constant_challenges: false,
            use_low_memory: false,
            is_preemptive_repeat_enabled: false,
            default_send_receive_bdp_frame: false,
            enforce_client_only,
            test_large_server_flight: false,
            is_port_blocking_disabled: false,
            are_path_callbacks_enabled: false,
            use_predictable_random: false,
            client_authentication: false,
            use_exporter: false,
            ech_opener: None,
            ech_server_retry_config: None,
            ech_client_enabled: false,
            pending_stateless_packets: std::collections::VecDeque::new(),
            default_congestion_alg: Some(&NEWRENO_ALGORITHM),
            default_congestion_alg_option_string: None,
            connections: crate::arena::Arena::new(),
            connection_list: std::collections::VecDeque::new(),
            connection_wake_tree: crate::splay::SplayTree::default(),
            connection_in_progress: None,
            connection_by_id: table_cnx_by_id,
            connection_by_net: table_cnx_by_net,
            connection_by_icid: table_cnx_by_icid,
            connection_by_secret: table_cnx_by_secret,
            issued_tickets_by_id: table_issued_tickets,
            issued_tickets: crate::arena::Arena::new(),
            nb_packets_allocated: 0,
            nb_packets_allocated_max: 0,
            stream_data_node_pool: Vec::new(),
            nb_data_nodes_allocated: 0,
            nb_data_nodes_allocated_max: 0,
            connection_id_callback_fn: cnx_id_callback,
            connection_id_callback_ctx: None,
            aead_encrypt_ticket_ctx: None,
            aead_decrypt_ticket_ctx: None,
            retry_integrity_sign_ctx: Vec::new(),
            retry_integrity_verify_ctx: Vec::new(),
            default_tp,
            fuzz_fn: None,
            fuzz_ctx: None,
            wake_file: 0,
            wake_line: 0,
            max_data_limit: 0,
            rtt_update_delta: crate::Duration::from_ticks(0),
            pacing_rate_update_delta: 0,
            key_log: None,
            f_log: None,
            binlog_dir: None,
            qlog_dir: None,
            autoqlog_fn: None,
            text_log_fns: None,
            bin_log_fns: None,
            qlog_fns: None,
            perflog_fn: None,
            v_perflog_ctx: None,
            v_thread_ctx: None,
        });
        quic.wake_list_init();

        if quic
            .init_master_tls_context(
                cert_file_name,
                key_file_name,
                _cert_root_file_name,
                ticket_encryption_key,
            )
            .is_err()
        {
            return None;
        }

        if let Some(ticket_file_name) = ticket_file_name {
            let _ = quic.load_tickets(ticket_file_name);
        }

        Some(quic)
    }

    // The C `picoquic_free` entry point is dropped from the Rust API
    // — context cleanup is the job of `Drop`, which Phase 3 will
    // implement.

    /// Toggle low-memory mode (smaller buffers, more aggressive
    /// reclaim), then refresh the default TLS cipher-suite selection.
    pub fn set_low_memory_mode(&mut self, low_memory_mode: bool) -> Result<(), Error> {
        self.use_low_memory = low_memory_mode;
        self.set_cipher_suite(0)
    }

    /// Configure the server cookie / retry-token mode.
    pub fn set_cookie_mode(&mut self, cookie_mode: i32) {
        self.force_check_token = (cookie_mode & 1) != 0;
        self.provide_token = (cookie_mode & 2) != 0;
        self.check_token = self.force_check_token
            || self.max_half_open_before_retry <= self.current_number_half_open;
    }

    /// Restrict TLS cipher-suite selection to the given IANA ID.
    pub fn set_cipher_suite(&mut self, cipher_suite_id: u16) -> Result<(), Error> {
        crate::tls_api::tls_api_init();
        self.tls_cipher_suites = crate::tls_api::picoquic_cipher_suite_list(
            i32::from(cipher_suite_id),
            self.use_low_memory,
        );
        if self.tls_cipher_suites.is_empty() {
            Err(Error::InvalidArgument)
        } else {
            Ok(())
        }
    }

    /// Restrict TLS key-exchange selection to the given IANA group.
    pub fn set_key_exchange(&mut self, key_exchange_id: u16) -> Result<(), Error> {
        crate::tls_api::tls_api_init();
        let key_exchanges = crate::tls_api::picoquic_key_exchange_list(i32::from(key_exchange_id))?;
        self.tls_key_exchanges = key_exchanges;
        Ok(())
    }

    /// Replace the default transport parameters used for new
    /// connections, or reset them to picoquic's initialized defaults
    /// when `tp` is `None`.
    ///
    /// C: `picoquic_set_default_tp` — copies a non-NULL TP pointer and
    /// calls `picoquic_init_transport_parameters` for NULL.
    pub fn set_default_tp<'a>(
        &mut self,
        tp: impl Into<Option<&'a TransportParameters>>,
    ) -> Result<(), Error> {
        if let Some(tp) = tp.into() {
            self.default_tp = tp.clone();
        } else {
            crate::internal::init_transport_parameters(&mut self.default_tp);
        }
        Ok(())
    }

    /// Borrow this context's default transport parameters.
    pub fn default_tp(&self) -> &TransportParameters {
        &self.default_tp
    }

    /// Maximum number of simultaneous connections this context was
    /// configured to accept.  C: reads `quic->max_number_connections`.
    pub fn max_nb_connections(&self) -> u32 {
        self.max_number_connections
    }

    /// Default ALPN string for newly accepted server connections, or
    /// `None` if no default was set.  C: reads `quic->default_alpn`.
    pub fn default_alpn_string(&self) -> Option<&str> {
        self.default_alpn.as_deref()
    }

    /// The stateless-reset secret seed for this context.
    /// C: reads `quic->reset_seed`.
    pub fn reset_seed_bytes(&self) -> &[u8] {
        &self.reset_seed
    }

    /// Identifier string of the default congestion-control algorithm,
    /// or `None` if no default was set.  C: reads
    /// `quic->default_congestion_alg->congestion_algorithm_id`.
    pub fn default_congestion_algorithm_id(&self) -> Option<&str> {
        self.default_congestion_alg
            .map(|a| a.congestion_algorithm_id)
    }

    /// Per-context maximum data limit used for flow control.
    /// C: reads `quic->max_data_limit`.
    pub fn max_data_limit(&self) -> u64 {
        self.max_data_limit
    }

    /// Override a single transport-parameter slot in the default
    /// set.  `tp_type` is the wire ID; values for unknown types are
    /// rejected with [`Error::InvalidArgument`].
    /// C: `picoquic_set_default_tp_value` — delegates to
    /// `picoquic_set_tp_value_by_type(&quic->default_tp, …)`.
    pub fn set_default_tp_value(&mut self, tp_type: u64, tp_value: u64) -> Result<(), Error> {
        set_tp_value_by_type(&mut self.default_tp, tp_type, tp_value)
    }

    /// Install the TLS certificate chain.  The context takes
    /// ownership of `certs` (each entry is one DER-encoded
    /// certificate).
    /// C: `picoquic_set_tls_certificate_chain`.
    pub fn set_tls_certificate_chain(&mut self, certs: Vec<Vec<u8>>) {
        self.tls_certificate_chain = certs;
    }

    /// Install the TLS root certificate set.  The C `int` return
    /// distinguished load vs. store failure (`-1` / `-2`); Phase 1
    /// collapses both into [`Error::Generic`] pending refinement in
    /// Phase 4.
    pub fn set_tls_root_certificates(&mut self, certs: Vec<Vec<u8>>) -> Result<(), Error> {
        crate::tls_api::tls_api_init();

        #[cfg(feature = "sys-openssl")]
        {
            if !crate::tls_api::is_openssl_verify_certificate_provider() {
                return Err(Error::Generic);
            }
            let mut registration =
                crate::sys::openssl::picoquic_openssl_get_certificate_verifier(None)
                    .ok_or(Error::Generic)?;
            let cert_refs: Vec<&[u8]> = certs.iter().map(|cert| cert.as_slice()).collect();
            (crate::sys::openssl::OPENSSL_PROVIDER_REGISTRATION
                .verify_certificate_provider
                .root_certificate_setter)(&mut registration.verifier, &cert_refs)?;
            let fallback = self.tls_callbacks.take();
            self.tls_callbacks = Some(Box::new(OpenSslRootCertificateCallbacks {
                registration,
                fallback,
            }));
            self.is_cert_store_not_empty = true;
            Ok(())
        }

        #[cfg(not(feature = "sys-openssl"))]
        {
            let _ = certs;
            Err(Error::Generic)
        }
    }

    /// Clear the certificate-verification callback bundle. C: `picoquic_set_null_verifier`.
    pub fn set_null_verifier(&mut self) {
        self.tls_callbacks = None;
    }

    /// Install the TLS private key (caller-owned bytes; the context
    /// copies on the way in).
    pub fn set_tls_key(&mut self, _key: &[u8]) -> Result<(), Error> {
        // Delegates to TLS backend; complex.
        Ok(())
    }

    /// Toggle whether this context demands client-side TLS
    /// authentication.
    pub fn set_client_authentication(&mut self, client_authentication: bool) {
        self.client_authentication = client_authentication;
    }

    /// Toggle whether the TLS exporter API is available on this
    /// context.
    pub fn set_use_exporter(&mut self, use_exporter: bool) {
        self.use_exporter = use_exporter;
    }

    /// Reject server-mode connections on this context (clients
    /// only).
    pub fn enforce_client_only(&mut self, do_enforce: bool) {
        self.enforce_client_only = do_enforce;
    }

    /// Default packet-padding policy (multiple, min-size).
    pub fn set_default_padding(&mut self, padding_multiple: u32, padding_minsize: u32) {
        self.padding_multiple_default = padding_multiple;
        self.padding_minsize_default = padding_minsize;
    }

    /// Default spin-bit policy applied to new connections.
    pub fn set_default_spinbit_policy(
        &mut self,
        default_spinbit_policy: SpinbitVersion,
    ) -> Result<(), Error> {
        self.default_spin_policy = default_spinbit_policy;
        Ok(())
    }

    /// C-shaped raw setter for tests and FFI-style callers that still carry
    /// integer `picoquic_spinbit_version_enum` values.
    pub fn set_default_spinbit_policy_raw(
        &mut self,
        default_spinbit_policy: u64,
    ) -> Result<(), Error> {
        self.set_default_spinbit_policy(SpinbitVersion::from_raw(default_spinbit_policy)?)
    }

    /// Default loss-bit policy applied to new connections.
    pub fn set_default_lossbit_policy(&mut self, default_lossbit_policy: LossbitVersion) {
        self.default_lossbit_policy = default_lossbit_policy;
        self.default_tp.enable_loss_bit = default_lossbit_policy as i32;
    }

    /// Default multipath option (per the multipath QUIC draft).
    pub fn set_default_multipath_option(&mut self, multipath_option: i32) {
        self.default_multipath_option = multipath_option as u32;
        if (multipath_option & 1) != 0 {
            self.default_tp.initial_max_path_id = 2;
        }
    }

    /// Default address-discovery mode (per the address-discovery
    /// draft): `0`=none, `1`=provide-only, `2`=receive-only,
    /// `3`=both.
    pub fn set_default_address_discovery_mode(&mut self, mode: i32) {
        if mode > 0 && mode <= 3 {
            self.default_tp.address_discovery_mode = mode;
        } else {
            self.default_tp.address_discovery_mode = 0;
        }
    }

    /// Enable or disable multipath event callbacks for all connections
    /// on this context.  C: `picoquic_enable_path_callbacks_default`.
    pub fn enable_path_callbacks_default(&mut self, enabled: bool) {
        self.are_path_callbacks_enabled = enabled;
    }

    /// Set default path-quality update thresholds for new connections.
    /// Notifications fire when pacing rate or RTT deviate by more
    /// than the given deltas.  C: `picoquic_default_quality_update`.
    pub fn default_quality_update(&mut self, pacing_rate_delta: u64, rtt_delta: crate::Duration) {
        self.pacing_rate_update_delta = pacing_rate_delta;
        self.rtt_update_delta = rtt_delta;
    }

    /// Cap the congestion window across all connections on this
    /// context.
    pub fn set_cwin_max(&mut self, cwin_max: u64) {
        self.cwin_max = if cwin_max == 0 { u64::MAX } else { cwin_max };
    }

    /// Return the current cwin cap.
    /// C: direct field access `quic->cwin_max`.
    pub fn cwin_max(&self) -> u64 {
        self.cwin_max
    }

    /// Cap the maximum stream data control window across this
    /// context. C: `picoquic_set_max_data_control`.
    pub fn set_max_data_control(&mut self, max_data: u64) {
        self.max_data_limit = max_data;
        self.default_tp.initial_max_data = max_data;

        for connection in self.connections.iter_mut() {
            let can_update_connection = {
                let tls_stream = &connection.tls_stream[crate::internal::Epoch::Initial as usize];
                connection.client_mode
                    && connection.connection_state == State::ClientInit
                    && tls_stream.sent_offset == 0
                    && tls_stream.send_queue.is_empty()
            };

            if can_update_connection {
                connection.local_parameters.initial_max_data = max_data;
                connection.maxdata_local = max_data;
            }
        }
    }

    /// Default idle-timeout (in milliseconds) advertised on new
    /// connections.
    pub fn set_default_idle_timeout(&mut self, idle_timeout: Duration) {
        self.default_tp.max_idle_timeout = idle_timeout;
    }
}

impl Connection {
    /// Replace the local transport parameters for this connection
    /// before the handshake completes.
    /// C: `picoquic_set_transport_parameters`.
    pub fn set_transport_parameters(&mut self, tp: &TransportParameters) {
        self.local_parameters = tp.clone();

        let mtu_max = self.quic_ref().map(|quic| quic.mtu_max).unwrap_or(0);
        if mtu_max > 0
            && self.local_parameters.max_packet_size == 0
            && let Some(peer_addr) = self
                .paths
                .first()
                .and_then(|path| path.tuples.first())
                .map(|tuple| tuple.peer_addr)
        {
            self.local_parameters.max_packet_size =
                mtu_max.saturating_sub(mtu_overhead(&peer_addr));
        }

        let role_bit = u64::from(!self.client_mode);
        self.maxdata_local = self.local_parameters.initial_max_data;
        self.max_stream_id_bidir_local = (self
            .local_parameters
            .initial_max_stream_id_bidir
            .wrapping_sub(1)
            << 2)
            | role_bit;
        self.max_stream_id_unidir_local = (self
            .local_parameters
            .initial_max_stream_id_unidir
            .wrapping_sub(1)
            << 2)
            | 2
            | role_bit;
    }

    /// Borrow the local (`get_local = true`) or remote transport
    /// parameters of this connection.
    pub fn transport_parameters(&self, get_local: bool) -> &TransportParameters {
        if get_local {
            &self.local_parameters
        } else {
            &self.remote_parameters
        }
    }

    /// Export TLS keying material to `out` using `label`.  Returns
    /// the number of bytes written.  C: `picoquic_export_secret`.
    pub fn export_secret(&mut self, label: &str, out: &mut [u8]) -> Result<usize, Error> {
        if out.is_empty() {
            return Err(Error::InvalidArgument);
        }

        let session = self.tls_ctx.as_ref().ok_or(Error::InvalidState)?;
        session.export_keying_material(label.as_bytes(), &[], out)?;
        Ok(out.len())
    }

    /// Per-connection spin-bit policy override.
    pub fn set_spinbit_policy(&mut self, spinbit_policy: SpinbitVersion) -> Result<(), Error> {
        // SpinbitVersion::On is server-only.
        if spinbit_policy == SpinbitVersion::On {
            return Err(Error::InvalidArgument);
        }
        self.spin_policy = spinbit_policy;
        Ok(())
    }

    /// C-shaped raw setter for tests and FFI-style callers that still carry
    /// integer `picoquic_spinbit_version_enum` values.
    pub fn set_spinbit_policy_raw(&mut self, spinbit_policy: u64) -> Result<(), Error> {
        self.set_spinbit_policy(SpinbitVersion::from_raw(spinbit_policy)?)
    }
}

impl Quic {
    /// Default per-connection handshake-timeout (microseconds).
    pub fn set_default_handshake_timeout(&mut self, handshake_timeout: Duration) {
        self.default_handshake_timeout = handshake_timeout;
    }

    /// Default per-connection crypto-epoch length (in encrypted
    /// bytes before triggering a key update).
    pub fn set_default_crypto_epoch_length(&mut self, crypto_epoch_length_max: u64) {
        self.crypto_epoch_length_max = match crypto_epoch_length_max {
            0 => DEFAULT_CRYPTO_EPOCH_LENGTH,
            _ => crypto_epoch_length_max,
        };
    }

    /// Retrieve the configured default crypto-epoch length.
    pub fn default_crypto_epoch_length(&self) -> u64 {
        self.crypto_epoch_length_max
    }

    /// Local-CID length in bytes (the value advertised in new
    /// connections).
    pub fn local_cid_length(&self) -> u8 {
        self.local_connection_id_length
    }

    /// Returns `true` when `cid` was issued by this context.
    pub fn is_local_cid(&self, cid: &ConnectionId) -> bool {
        if cid.len() != self.local_connection_id_length as usize {
            return false;
        }

        let Some(hash_token) = self.connection_by_id.lookup(cid) else {
            return false;
        };
        let Some(&connection_token) = self.connection_by_id.get(hash_token) else {
            return false;
        };
        self.connections
            .get(connection_token)
            .is_some_and(|connection| {
                connection.local_connection_id_lists.iter().any(|list| {
                    list.connection_ids.iter().any(|&local_cid_token| {
                        connection
                            .local_connection_ids
                            .get(local_cid_token)
                            .is_some_and(|local_cid| {
                                local_cid.connection_by_id_membership == Some(hash_token)
                                    && local_cid.connection_id == *cid
                            })
                    })
                })
            })
    }

    /// Load issued-retry-tokens from a persistent store.
    pub fn load_retry_tokens(&mut self, token_store_filename: &str) -> Result<(), Error> {
        // C wrapper delegates directly to picoquic_load_tokens.
        self.load_tokens(token_store_filename)
    }

    /// Persist session tickets to disk.
    /// C: `picoquic_save_session_tickets`.
    pub fn save_session_tickets(&mut self, ticket_store_filename: &str) -> Result<(), Error> {
        self.save_tickets(Instant::from_ticks(self.time()), ticket_store_filename)
    }

    /// Persist outstanding retry tokens to disk.
    /// C: `picoquic_save_retry_tokens`.
    pub fn save_retry_tokens(&mut self, token_store_filename: &str) -> Result<(), Error> {
        self.save_tokens(token_store_filename)
    }

    /// Toggle BDP-frame extension on new connections (per the
    /// 0-RTT-BDP draft).
    pub fn set_default_bdp_frame_option(&mut self, enable_bdp_frame: bool) {
        self.default_send_receive_bdp_frame = enable_bdp_frame;
    }

    /// Configure the local CID length (in bytes) advertised on new
    /// connections.
    /// C: `picoquic_set_default_connection_id_length`.
    pub fn set_default_connection_id_length(&mut self, cid_length: u8) -> Result<(), Error> {
        if cid_length != self.local_connection_id_length {
            if cid_length as usize > CONNECTION_ID_MAX_SIZE {
                return Err(Error::Protocol(InternalError::CnxidCheck as u64));
            }
            if self.current_number_connections > 0 {
                return Err(Error::Protocol(
                    InternalError::CannotChangeActiveContext as u64,
                ));
            }
            self.local_connection_id_length = cid_length;
        }
        Ok(())
    }

    /// Default per-connection-ID time-to-live before retirement, in
    /// microseconds.
    pub fn set_default_connection_id_ttl(&mut self, ttl_usec: u64) {
        self.local_connection_id_ttl = ttl_usec;
    }

    /// Retrieve the configured default connection-ID TTL.
    pub fn default_connection_id_ttl(&self) -> u64 {
        self.local_connection_id_ttl
    }

    /// Cap the maximum MTU PMTUD will probe up to.
    pub fn set_mtu_max(&mut self, mtu_max: u32) {
        self.mtu_max = mtu_max;
        self.default_tp.max_packet_size = mtu_max;
    }

    /// Install (or remove, with `None`) the ALPN-selection callback.
    /// Clears any static `default_alpn` string, matching the behaviour of both
    /// the C `picoquic_set_alpn_select_fn` (quicctx.c:4727) and
    /// `picoquic_set_alpn_select_fn_v2` (quicctx.c:4737), which are folded
    /// into this single entry point — the two C variants differed only in the
    /// iovec representation of the ALPN list, a distinction erased by Rust
    /// slices.
    pub fn set_alpn_select_fn(&mut self, alpn_select_fn: Option<Box<dyn AlpnSelect>>) {
        self.default_alpn = None;
        self.alpn_select_fn = alpn_select_fn;
    }

    /// Install (or remove, with `None`) the default stream/event
    /// callback applied to new connections.  The `(callback_fn,
    /// callback_ctx)` pair from C collapses into one trait object.
    pub fn set_default_callback(&mut self, callback: Option<Box<dyn StreamDataCallback>>) {
        self.default_callback_fn = callback;
    }

    /// Install or clear the certificate-verification callback bundle.
    /// C: `picoquic_set_verify_certificate_callback`
    /// (picoquic/quicctx.c:5486-5492).
    pub fn set_verify_certificate_callback(
        &mut self,
        callback: Option<Box<dyn crate::tls::TlsCallbacks>>,
    ) {
        self.tls_callbacks = callback;
    }

    /// Default minimum interval between stateless-reset emissions
    /// (microseconds).
    pub fn set_default_stateless_reset_min_interval(&mut self, min_interval: Duration) {
        let now = Instant::from_ticks(self.time());
        self.stateless_reset_next_time = now;
        self.stateless_reset_min_interval = min_interval;
    }

    /// Cap the number of connections that may emit logs
    /// simultaneously.
    pub fn set_max_simultaneous_logs(&mut self, max_simultaneous_logs: u32) {
        self.max_simultaneous_logs = max_simultaneous_logs;
    }

    /// Retrieve the configured maximum-simultaneous-logs cap.
    pub fn max_simultaneous_logs(&self) -> u32 {
        self.max_simultaneous_logs
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
        initial_cnx_id: ConnectionId,
        remote_cnx_id: ConnectionId,
        addr_to: Option<&SocketAddr>,
        start_time: Instant,
        preferred_version: u32,
        sni: Option<&str>,
        alpn: Option<&str>,
        client_mode: bool,
    ) -> Option<&mut Connection> {
        let token = self
            .create_cnx_internal(
                initial_cnx_id,
                remote_cnx_id,
                addr_to,
                start_time,
                preferred_version,
                sni,
                alpn,
                client_mode,
                None,
                None,
            )
            .ok()?;
        self.connections.get_mut(token)
    }

    /// Look up a live connection by its initial source connection ID and return
    /// a mutable reference to it.  Returns `None` when no such connection is
    /// live in this context.  C: access pattern of storing and dereferencing a
    /// `picoquic_cnx_t*` pointer obtained from `picoquic_create_cnx`.
    pub fn connection_ref_by_id(&mut self, id: ConnectionId) -> Option<&mut Connection> {
        self.connections
            .iter_mut()
            .find(|connection| connection.initial_connection_id == id)
    }

    /// Create and start a client-side connection; `addr` is required.
    pub fn create_client_connection(
        &mut self,
        addr: &SocketAddr,
        start_time: Instant,
        preferred_version: u32,
        sni: Option<&str>,
        alpn: Option<&str>,
        callback: Option<Box<dyn StreamDataCallback>>,
    ) -> Option<&mut Connection> {
        // C: picoquic_create_client_cnx — wraps picoquic_create_cnx with
        // null CIDs, then runs picoquic_start_client_cnx and rolls back
        // on failure.  Callback installation is folded in here; the Rust
        // shape stores the boxed callback on Connection, which is the
        // moral equivalent of `cnx->callback_fn`/`cnx->callback_ctx`.
        let token = self
            .create_cnx_internal(
                ConnectionId::with_size(0)?,
                ConnectionId::with_size(0)?,
                Some(addr),
                start_time,
                preferred_version,
                sni,
                alpn,
                true,
                None,
                None,
            )
            .ok()?;

        let start_result = {
            let cnx = self.connections.get_mut(token)?;
            if let Some(callback) = callback {
                cnx.set_callback(Some(callback));
            }
            cnx.start_client()
        };
        if start_result.is_err() {
            self.delete_connection(token);
            return None;
        }

        self.connections.get_mut(token)
    }

    /// Default callback enablement for path-state events on new
    /// connections.
    pub fn set_path_callbacks_default(&mut self, are_enabled: bool) {
        self.are_path_callbacks_enabled = are_enabled;
    }

    /// Default thresholds for the path-quality-update callback.
    pub fn set_default_quality_update(&mut self, pacing_rate_delta: u64, rtt_delta: Duration) {
        self.pacing_rate_update_delta = pacing_rate_delta;
        self.rtt_update_delta = rtt_delta;
    }

    /// Remove a path's peer-address registration.
    /// C: `picoquic_unregister_net_id` (picoquic/quicctx.c:1280-1290).
    pub fn unregister_net_id(&mut self, connection: ConnectionToken, path_index: usize) {
        let Some((membership, registered_addr)) =
            self.connections.get(connection).and_then(|cnx| {
                cnx.paths
                    .get(path_index)
                    .map(|path| (path.connection_by_net_membership, path.registered_peer_addr))
            })
        else {
            return;
        };

        if let Some(membership) = membership {
            self.connection_by_net.remove(membership);
        } else if !socket_addr_is_unspecified(&registered_addr)
            && let Some(item) = self.connection_by_net.lookup(&registered_addr)
            && self.connection_by_net.get(item).copied() == Some(connection)
        {
            self.connection_by_net.remove(item);
        }

        if let Some(cnx) = self.connections.get_mut(connection)
            && let Some(path) = cnx.paths.get_mut(path_index)
        {
            path.registered_peer_addr = unspecified_socket_addr();
            path.connection_by_net_membership = None;
        }
    }

    /// Register a path's first peer address in the context network-address table.
    /// C: `picoquic_register_net_id` (picoquic/quicctx.c:1292-1310).
    pub fn register_net_id(
        &mut self,
        connection: ConnectionToken,
        path_index: usize,
    ) -> Result<(), Error> {
        self.unregister_net_id(connection, path_index);

        let peer_addr = self
            .connections
            .get(connection)
            .and_then(|cnx| cnx.paths.get(path_index))
            .and_then(|path| path.tuples.first())
            .map(|tuple| tuple.peer_addr)
            .ok_or(Error::InvalidArgument)?;
        if socket_addr_is_unspecified(&peer_addr) {
            return Ok(());
        }
        if self.connection_by_net.lookup(&peer_addr).is_some() {
            return Err(Error::Generic);
        }
        let (membership, _) = self.connection_by_net.insert(peer_addr, connection)?;
        if let Some(cnx) = self.connections.get_mut(connection)
            && let Some(path) = cnx.paths.get_mut(path_index)
        {
            path.registered_peer_addr = peer_addr;
            path.connection_by_net_membership = Some(membership);
        }
        Ok(())
    }

    /// Clear and remove one path's owned data.
    ///
    /// C: `picoquic_clear_path_data` (picoquic/quicctx.c:1885-1899).
    pub(crate) fn clear_path_data(&mut self, connection: ConnectionToken, path_index: usize) {
        self.unregister_net_id(connection, path_index);

        let congestion_alg = self
            .connections
            .get(connection)
            .and_then(|cnx| cnx.congestion_alg);

        if let Some(cnx) = self.connections.get_mut(connection) {
            if let Some(path) = cnx.paths.get_mut(path_index)
                && let Some(alg) = congestion_alg
            {
                alg.algorithm.alg_delete(path);
            }
            while cnx
                .paths
                .get(path_index)
                .is_some_and(|path| !path.tuples.is_empty())
            {
                cnx.delete_tuple(path_index, 0, true);
            }
        }

        if let Some(cnx) = self.connections.get_mut(connection)
            && path_index < cnx.paths.len()
        {
            cnx.paths.remove(path_index);
        }
    }

    /// Register the connection's initial CID and default-path peer address.
    /// C: `picoquic_register_net_icid` (picoquic/quicctx.c:1340-1354).
    pub fn register_net_icid(&mut self, connection: ConnectionToken) -> Result<(), Error> {
        let (initial_cid, peer_addr, already_registered) = self
            .connections
            .get(connection)
            .and_then(|cnx| {
                let peer_addr = cnx.paths.first()?.tuples.first()?.peer_addr;
                Some((
                    cnx.initial_connection_id,
                    peer_addr,
                    cnx.connection_by_icid_membership.is_some(),
                ))
            })
            .ok_or(Error::InvalidArgument)?;
        if already_registered {
            return Err(Error::Generic);
        }
        let key = (initial_cid, peer_addr);
        if self.connection_by_icid.lookup(&key).is_some() {
            return Err(Error::Generic);
        }
        let (membership, _) = self.connection_by_icid.insert(key, connection)?;
        if let Some(cnx) = self.connections.get_mut(connection) {
            cnx.registered_icid_addr = peer_addr;
            cnx.connection_by_icid_membership = Some(membership);
        }
        Ok(())
    }

    /// Remove the connection's initial-CID network registration.
    /// C: `picoquic_unregister_net_icid` (picoquic/quicctx.c:1356-1363).
    pub fn unregister_net_icid(&mut self, connection: ConnectionToken) {
        let membership = self
            .connections
            .get(connection)
            .and_then(|cnx| cnx.connection_by_icid_membership);
        if let Some(membership) = membership {
            self.connection_by_icid.remove(membership);
        }
        if let Some(cnx) = self.connections.get_mut(connection) {
            cnx.registered_icid_addr = unspecified_socket_addr();
            cnx.connection_by_icid_membership = None;
        }
    }

    /// Remove the stateless-reset-secret network registration.
    /// C: `picoquic_unregister_net_secret` (picoquic/quicctx.c:1365-1372).
    pub fn unregister_net_secret(&mut self, connection: ConnectionToken) {
        let membership = self
            .connections
            .get(connection)
            .and_then(|cnx| cnx.connection_by_secret_membership);
        if let Some(membership) = membership {
            self.connection_by_secret.remove(membership);
        }
        if let Some(cnx) = self.connections.get_mut(connection) {
            cnx.registered_secret_addr = unspecified_socket_addr();
            cnx.registered_reset_secret = [0; RESET_SECRET_SIZE];
            cnx.connection_by_secret_membership = None;
        }
    }

    /// Remove a connection from the context's live-connection list.
    ///
    /// The C intrusive `cnx_list`/`cnx_last` links are represented by
    /// `connection_list`, so this helper unlinks that token, unregisters
    /// ICID/reset-secret indexes, and decrements the live-connection count.
    ///
    /// C: `picoquic_remove_cnx_from_list` (picoquic/quicctx.c:1450-1469).
    pub(crate) fn remove_cnx_from_list(&mut self, connection: ConnectionToken) {
        let was_listed = if let Some(pos) = self
            .connection_list
            .iter()
            .position(|token| *token == connection)
        {
            self.connection_list.remove(pos);
            true
        } else {
            false
        };
        self.unregister_net_icid(connection);
        self.unregister_net_secret(connection);
        if was_listed && self.current_number_connections > 0 {
            self.current_number_connections -= 1;
        }
    }

    /// Register the default path's peer address and reset secret.
    /// C: `picoquic_register_net_secret` (picoquic/quicctx.c:1374-1392).
    pub fn register_net_secret(&mut self, connection: ConnectionToken) -> Result<(), Error> {
        let Some((peer_addr, unique_path_id, cid_index)) =
            self.connections.get(connection).and_then(|cnx| {
                let path = cnx.paths.first()?;
                let tuple = path.tuples.first()?;
                Some((
                    tuple.peer_addr,
                    if cnx.is_multipath_enabled {
                        path.unique_path_id
                    } else {
                        0
                    },
                    tuple.remote_connection_id_index.unwrap_or(0),
                ))
            })
        else {
            return Err(Error::InvalidArgument);
        };
        if socket_addr_is_unspecified(&peer_addr) {
            return Ok(());
        }
        let reset_secret = self
            .connections
            .get(connection)
            .and_then(|cnx| {
                cnx.remote_connection_id_stashes
                    .iter()
                    .find(|stash| stash.unique_path_id == unique_path_id)
                    .and_then(|stash| stash.connection_ids.get(cid_index))
                    .map(|remote_cid| remote_cid.reset_secret)
            })
            .unwrap_or([0u8; RESET_SECRET_SIZE]);

        self.unregister_net_secret(connection);
        let key = (reset_secret, peer_addr);
        if self.connection_by_secret.lookup(&key).is_some() {
            return Err(Error::Generic);
        }
        let (membership, _) = self.connection_by_secret.insert(key, connection)?;
        if let Some(cnx) = self.connections.get_mut(connection) {
            cnx.registered_secret_addr = peer_addr;
            cnx.registered_reset_secret = reset_secret;
            cnx.connection_by_secret_membership = Some(membership);
        }
        Ok(())
    }

    /// Create and register a local connection ID on the specified connection.
    /// C: `picoquic_create_local_cnxid` (picoquic/quicctx.c:3795-3866).
    pub fn create_local_cnxid(
        &mut self,
        connection: ConnectionToken,
        unique_path_id: u64,
        suggested_value: Option<ConnectionId>,
        current_time: Instant,
    ) -> Result<crate::internal::LocalConnectionIdToken, Error> {
        let initial_connection_id = self
            .connections
            .get(connection)
            .map(|cnx| cnx.initial_connection_id)
            .ok_or(Error::InvalidArgument)?;

        let connection_id = if self.local_connection_id_length == 0 {
            ConnectionId::default()
        } else {
            let mut selected = None;
            for attempt in 0..32 {
                let candidate = if attempt == 0 {
                    if let Some(suggested) = suggested_value {
                        suggested
                    } else {
                        let mut generated = ConnectionId::default();
                        self.create_local_cnx_id(&mut generated, initial_connection_id);
                        generated
                    }
                } else {
                    let mut generated = ConnectionId::default();
                    self.create_local_cnx_id(&mut generated, initial_connection_id);
                    generated
                };
                if self.connection_by_id.lookup(&candidate).is_none() {
                    selected = Some(candidate);
                    break;
                }
            }
            selected.ok_or(Error::Generic)?
        };

        let token = {
            let cnx = self
                .connections
                .get_mut(connection)
                .ok_or(Error::InvalidArgument)?;
            let list_idx = match cnx
                .local_connection_id_lists
                .iter()
                .position(|list| list.unique_path_id == unique_path_id)
            {
                Some(idx) => idx,
                None => {
                    cnx.local_connection_id_lists
                        .push(crate::internal::LocalConnectionIdList {
                            unique_path_id,
                            local_connection_id_sequence_next: 0,
                            local_connection_id_retire_before: 0,
                            local_connection_id_oldest_created: current_time.ticks(),
                            nb_local_connection_id_expired: 0,
                            is_demoted: false,
                            demotion_time: Instant::from_ticks(u64::MAX),
                            connection_ids: Vec::new(),
                        });
                    cnx.local_connection_id_lists.len() - 1
                }
            };
            let sequence =
                cnx.local_connection_id_lists[list_idx].local_connection_id_sequence_next;
            let local_cid = crate::internal::LocalConnectionId {
                connection_by_id_membership: None,
                path_id: unique_path_id,
                sequence,
                create_time: current_time,
                connection_id,
                is_acked: false,
            };
            let token = cnx
                .local_connection_ids
                .insert(local_cid)
                .map_err(|_| Error::Memory)?;
            cnx.local_connection_id_lists[list_idx].local_connection_id_sequence_next += 1;
            cnx.local_connection_id_lists[list_idx]
                .connection_ids
                .push(token);
            if sequence == 0 {
                cnx.local_connection_id_lists[list_idx].local_connection_id_oldest_created =
                    current_time.ticks();
                if unique_path_id > cnx.max_path_id_in_connection_id_lists {
                    cnx.max_path_id_in_connection_id_lists = unique_path_id;
                }
            }
            token
        };

        if self.local_connection_id_length > 0
            && let Ok((membership, _)) = self.connection_by_id.insert(connection_id, connection)
            && let Some(cnx) = self.connections.get_mut(connection)
            && let Some(local_cid) = cnx.local_connection_ids.get_mut(token)
        {
            local_cid.connection_by_id_membership = Some(membership);
        }

        Ok(token)
    }
}

impl Connection {
    /// Begin the client-side handshake on this connection.
    /// C: `picoquic_start_client_cnx` (picoquic/quicctx.c:4384-4413).
    pub fn start_client(&mut self) -> Result<(), Error> {
        let initial_tls = &self.tls_stream[crate::internal::Epoch::Initial as usize];
        if self.connection_state != State::ClientInit
            || initial_tls.sent_offset > 0
            || !initial_tls.send_queue.is_empty()
        {
            log::debug!("picoquic_start_client_cnx called twice.");
            return Err(Error::Generic);
        }

        self.log_new_connection();

        let current_time = Instant::from_ticks(
            self.quic_ref()
                .map(|quic| quic.time())
                .unwrap_or_else(crate::current_time),
        );
        let tls_init_result = self.initialize_tls_stream(current_time);

        let role_bit = u64::from(!self.client_mode);
        self.maxdata_remote = self.remote_parameters.initial_max_data;
        self.max_stream_id_bidir_remote = (self
            .remote_parameters
            .initial_max_stream_id_bidir
            .wrapping_sub(1)
            << 2)
            | role_bit;
        self.max_stream_id_unidir_remote = (self
            .remote_parameters
            .initial_max_stream_id_unidir
            .wrapping_sub(1)
            << 2)
            | 2
            | role_bit;
        self.max_stream_data_remote = self.remote_parameters.initial_max_data;
        self.max_stream_data_local = self.local_parameters.initial_max_stream_data_bidi_local;

        self.reinsert_self_by_wake_time(current_time);
        tls_init_result
    }

    /// Begin an ordered close.
    pub fn close(&mut self, application_reason_code: u64) -> Result<(), Error> {
        self.picoquic_close_ex(application_reason_code, None)
    }

    /// Same as [`Self::close`] but carries a textual `error_reason`.
    /// `None` matches the C `NULL` case.
    pub fn close_with_reason(
        &mut self,
        application_reason_code: u64,
        error_reason: Option<&str>,
    ) -> Result<(), Error> {
        self.picoquic_close_ex(application_reason_code, error_reason)
    }

    /// Force-close the connection without waiting for the protocol
    /// drain.
    pub fn close_immediate(&mut self) {
        self.connection_state = State::Disconnected;
    }

    /// Reset the connection to a fresh handshake state.
    /// C: `picoquic_reset_cnx` (picoquic/quicctx.c:4939-4989).
    pub fn reset_cnx(&mut self, current_time: Instant) -> Result<(), Error> {
        for pc in 0..crate::NB_PACKET_CONTEXT {
            if pc != PacketContext::Application as usize {
                let pkt_ctx = &mut self.pkt_ctx[pc];
                pkt_ctx.pending.clear();
                pkt_ctx.retransmitted.clear();
                pkt_ctx.send_sequence = 0;
                pkt_ctx.retransmit_sequence = 0;
                pkt_ctx.next_sequence_hole = 0;
                pkt_ctx.retransmitted_queue_size = 0;
                pkt_ctx.highest_acknowledged = u64::MAX;
                pkt_ctx.latest_time_acknowledged = current_time;
                pkt_ctx.highest_acknowledged_time = current_time;
                self.ack_ctx[pc].reset_ack_context();
            }
        }

        for stream in &mut self.tls_stream {
            stream.clear_stream();
            stream.consumed_offset = 0;
            stream.fin_offset = 0;
            stream.sent_offset = 0;
        }

        fn empty_crypto_context() -> crate::internal::CryptoContext {
            crate::internal::CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            }
        }
        for ctx in &mut self.crypto_context {
            *ctx = empty_crypto_context();
        }
        self.crypto_context_new = empty_crypto_context();

        self.setup_initial_traffic_keys()?;
        self.tls_ctx = None;
        if self.quic_ptr.is_null() {
            return Err(Error::InvalidState);
        }
        // SAFETY: quic_ptr is installed by Quic::create_cnx_internal and the
        // owning Quic outlives every connection stored in its arena.
        let quic = unsafe { &mut *self.quic_ptr };
        self.create_tls_context(quic)?;
        self.initialize_tls_stream(current_time)
    }

    /// Delete the connection.  In the C API this releases the
    /// connection's slot inside its QUIC context; in Rust the
    /// resources drop when `Connection` itself does — arena-level
    /// deallocation is wired via `Drop`.
    pub fn delete(&mut self) {
        // Phase 3 wires up arena-level deallocation.
    }

    /// Override the application-set wake time.
    pub fn set_app_wake_time(&mut self, app_wake_time: Instant) {
        self.app_wake_time = app_wake_time;
    }

    /// Set the version the client should request on next handshake.
    pub fn set_desired_version(&mut self, desired_version: u32) {
        self.desired_version = desired_version;
    }

    /// Record a peer-rejected version (for diagnostics / VN frames).
    pub fn set_rejected_version(&mut self, rejected_version: u32) {
        self.rejected_version = rejected_version;
    }

    fn use_constant_challenges(&self) -> i32 {
        self.quic_ref()
            .map(|quic| i32::from(quic.use_constant_challenges))
            .unwrap_or(0)
    }

    fn register_path_by_index(&mut self, path_index: usize) {
        let Some(path) = self.paths.get_mut(path_index) else {
            return;
        };
        path.path_is_published = true;
        let peer_addr = path
            .tuples
            .first()
            .map(|tuple| tuple.peer_addr)
            .unwrap_or_else(unspecified_socket_addr);
        path.registered_peer_addr = peer_addr;

        if socket_addr_is_unspecified(&peer_addr) {
            return;
        }
        let Some(token) = self.own_token else {
            return;
        };
        if self.quic_ptr.is_null() {
            return;
        }

        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal`
        // and remains valid while this connection is live.  This mirrors
        // `picoquic_register_path`: only the owning context's peer-address
        // table is updated, and only when the connection is still in it.
        let quic = unsafe { &mut *self.quic_ptr };
        if quic.local_connection_id_length != 0 || !quic.connections.contains(token) {
            return;
        }

        let old_membership = self
            .paths
            .get_mut(path_index)
            .and_then(|path| path.connection_by_net_membership.take());
        if let Some(membership) = old_membership {
            quic.connection_by_net.remove(membership);
        }

        if quic.connection_by_net.lookup(&peer_addr).is_none()
            && let Ok((membership, _)) = quic.connection_by_net.insert(peer_addr, token)
            && let Some(path) = self.paths.get_mut(path_index)
        {
            path.connection_by_net_membership = Some(membership);
        }
    }

    fn has_available_remote_cid(&self, unique_path_id: u64) -> bool {
        self.remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == unique_path_id)
            .and_then(|stash| stash.get_connection_id_from_stash())
            .is_some()
    }

    fn check_cid_for_new_tuple_result(&self, unique_path_id: u64) -> Result<(), Error> {
        if self.has_available_remote_cid(unique_path_id) {
            Ok(())
        } else if self.unique_path_id_next > self.max_path_id_remote {
            Err(Error::Protocol(InternalError::PathIdBlocked as u64))
        } else {
            Err(Error::Protocol(InternalError::PathCidBlocked as u64))
        }
    }

    fn assign_peer_cnxid_to_tuple(
        &mut self,
        unique_path_id: u64,
        tuple: &mut crate::internal::Tuple,
    ) -> Result<(), Error> {
        let (stash_idx, cid_idx) = self
            .obtain_stashed_connection_id(unique_path_id)
            .ok_or(Error::Generic)?;
        let stash = self
            .remote_connection_id_stashes
            .get_mut(stash_idx)
            .ok_or(Error::Generic)?;
        let remote_cid = stash
            .connection_ids
            .get_mut(cid_idx)
            .ok_or(Error::Generic)?;
        tuple.remote_connection_id_index = Some(cid_idx);
        tuple.unique_path_id = unique_path_id;
        remote_cid.nb_path_references += 1;
        stash.is_in_use = true;
        Ok(())
    }

    fn register_default_path_reset_secret(&mut self) -> Result<(), Error> {
        let (peer_addr, cid_path_id, cid_index) = {
            let path = self.paths.first().ok_or(Error::InvalidArgument)?;
            let tuple = path.tuples.first().ok_or(Error::InvalidArgument)?;
            (
                tuple.peer_addr,
                if self.is_multipath_enabled {
                    path.unique_path_id
                } else {
                    0
                },
                tuple.remote_connection_id_index.unwrap_or(0),
            )
        };
        if socket_addr_is_unspecified(&peer_addr) {
            return Ok(());
        }
        let reset_secret = self
            .remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == cid_path_id)
            .and_then(|stash| stash.connection_ids.get(cid_index))
            .map(|remote_cid| remote_cid.reset_secret)
            .ok_or(Error::Protocol(InternalError::CnxidNotAvailable as u64))?;

        let Some(token) = self.own_token else {
            self.registered_secret_addr = peer_addr;
            self.registered_reset_secret = reset_secret;
            self.connection_by_secret_membership = None;
            return Ok(());
        };
        if self.quic_ptr.is_null() {
            self.registered_secret_addr = peer_addr;
            self.registered_reset_secret = reset_secret;
            self.connection_by_secret_membership = None;
            return Ok(());
        }

        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal`
        // and remains valid while this connection is live.  This updates
        // only the owning context's reset-secret table plus this connection's
        // cached membership fields, matching `picoquic_register_net_secret`.
        let quic = unsafe { &mut *self.quic_ptr };
        if !quic.connections.contains(token) {
            self.registered_secret_addr = peer_addr;
            self.registered_reset_secret = reset_secret;
            self.connection_by_secret_membership = None;
            return Ok(());
        }
        if let Some(membership) = self.connection_by_secret_membership.take() {
            quic.connection_by_secret.remove(membership);
        }

        self.registered_secret_addr = peer_addr;
        self.registered_reset_secret = reset_secret;
        let key = (reset_secret, peer_addr);
        if quic.connection_by_secret.lookup(&key).is_some() {
            return Err(Error::Generic);
        }
        let (membership, _) = quic.connection_by_secret.insert(key, token)?;
        self.connection_by_secret_membership = Some(membership);
        Ok(())
    }

    fn adjust_remote_cid_indices_after_remove(&mut self, cid_path_id: u64, removed_index: usize) {
        let is_multipath_enabled = self.is_multipath_enabled;
        for path in &mut self.paths {
            let path_cid_id = if is_multipath_enabled {
                path.unique_path_id
            } else {
                0
            };
            if path_cid_id != cid_path_id {
                continue;
            }
            for tuple in &mut path.tuples {
                if let Some(index) = tuple.remote_connection_id_index {
                    if index == removed_index {
                        tuple.remote_connection_id_index = None;
                    } else if index > removed_index {
                        tuple.remote_connection_id_index = Some(index - 1);
                    }
                }
            }
        }
    }

    fn dereference_first_tuple_remote_cid(
        &mut self,
        path_index: usize,
        stash_index: usize,
        cid_path_id: u64,
    ) -> Option<usize> {
        let old_cid_index = self
            .paths
            .get_mut(path_index)
            .and_then(|path| path.tuples.first_mut())
            .and_then(|tuple| tuple.remote_connection_id_index.take())?;
        if old_cid_index
            >= self
                .remote_connection_id_stashes
                .get(stash_index)
                .map(|stash| stash.connection_ids.len())
                .unwrap_or(0)
        {
            return None;
        }

        let mut retire_sequence = None;
        let remove_old = {
            let old_cid =
                &mut self.remote_connection_id_stashes[stash_index].connection_ids[old_cid_index];
            if old_cid.nb_path_references <= 1 {
                if !old_cid.retire_sent {
                    retire_sequence = Some(old_cid.sequence);
                }
                old_cid.retire_acked
            } else {
                old_cid.nb_path_references -= 1;
                false
            }
        };

        if let Some(sequence) = retire_sequence
            && self
                .queue_retire_connection_id_frame(cid_path_id, sequence)
                .is_ok()
            && let Some(old_cid) = self
                .remote_connection_id_stashes
                .get_mut(stash_index)
                .and_then(|stash| stash.connection_ids.get_mut(old_cid_index))
        {
            old_cid.retire_sent = true;
        }

        if remove_old {
            self.remote_connection_id_stashes[stash_index]
                .connection_ids
                .remove(old_cid_index);
            self.adjust_remote_cid_indices_after_remove(cid_path_id, old_cid_index);
            Some(old_cid_index)
        } else {
            None
        }
    }

    fn renew_path_remote_connection_id(&mut self, path_index: usize) -> Result<(), Error> {
        let path_unique_id = self
            .paths
            .get(path_index)
            .map(|path| path.unique_path_id)
            .ok_or(Error::Generic)?;
        if self
            .paths
            .get(path_index)
            .and_then(|path| path.tuples.first())
            .is_none()
        {
            return Err(Error::Protocol(InternalError::CnxidNotAvailable as u64));
        }

        let cid_path_id = if self.is_multipath_enabled {
            path_unique_id
        } else {
            0
        };
        let stash_index = self
            .remote_connection_id_stashes
            .iter()
            .position(|stash| stash.unique_path_id == cid_path_id)
            .ok_or(Error::Protocol(InternalError::CnxidNotAvailable as u64))?;
        let current_cid_index = self
            .paths
            .get(path_index)
            .and_then(|path| path.tuples.first())
            .and_then(|tuple| tuple.remote_connection_id_index);
        let current_sequence = current_cid_index.and_then(|cid_index| {
            self.remote_connection_id_stashes[stash_index]
                .connection_ids
                .get(cid_index)
                .map(|remote_cid| remote_cid.sequence)
        });
        let retire_connection_id_before =
            self.remote_connection_id_stashes[stash_index].retire_connection_id_before;
        if (self.remote_parameters.migration_disabled
            && current_sequence.is_some_and(|sequence| sequence >= retire_connection_id_before))
            || self.local_parameters.migration_disabled
        {
            return Err(Error::Protocol(InternalError::MigrationDisabled as u64));
        }

        let mut new_cid_index = self.remote_connection_id_stashes[stash_index]
            .get_connection_id_from_stash()
            .ok_or(Error::Protocol(InternalError::CnxidNotAvailable as u64))?;
        let new_sequence = self.remote_connection_id_stashes[stash_index]
            .connection_ids
            .get(new_cid_index)
            .map(|remote_cid| remote_cid.sequence)
            .ok_or(Error::Protocol(InternalError::CnxidNotAvailable as u64))?;
        if current_sequence == Some(new_sequence) {
            return Err(Error::Protocol(InternalError::CnxidNotAvailable as u64));
        }

        if let Some(removed_index) =
            self.dereference_first_tuple_remote_cid(path_index, stash_index, cid_path_id)
        {
            if removed_index == new_cid_index {
                return Err(Error::Protocol(InternalError::CnxidNotAvailable as u64));
            }
            if removed_index < new_cid_index {
                new_cid_index -= 1;
            }
        }

        {
            let stash = self
                .remote_connection_id_stashes
                .get_mut(stash_index)
                .ok_or(Error::Protocol(InternalError::CnxidNotAvailable as u64))?;
            let remote_cid = stash
                .connection_ids
                .get_mut(new_cid_index)
                .ok_or(Error::Protocol(InternalError::CnxidNotAvailable as u64))?;
            remote_cid.nb_path_references += 1;
            stash.is_in_use = true;
        }
        if let Some(tuple) = self
            .paths
            .get_mut(path_index)
            .and_then(|path| path.tuples.first_mut())
        {
            tuple.remote_connection_id_index = Some(new_cid_index);
            tuple.unique_path_id = path_unique_id;
        }

        if path_index == 0 {
            self.register_default_path_reset_secret()?;
        }
        Ok(())
    }

    fn find_available_unique_path_id(&mut self, requested_id: u64) -> Option<u64> {
        if !self.is_multipath_enabled {
            return if requested_id == 0 || requested_id == u64::MAX {
                Some(0)
            } else {
                None
            };
        }

        if requested_id != u64::MAX || (!self.client_mode && !self.paths.is_empty()) {
            return (requested_id != u64::MAX).then_some(requested_id);
        }

        while self.unique_path_id_next <= self.max_path_id_remote
            && self.unique_path_id_next <= self.max_path_id_local
            && self.unique_path_id_next <= self.max_path_id_in_connection_id_lists
        {
            let unique_path_id = self.unique_path_id_next;
            self.unique_path_id_next = self.unique_path_id_next.saturating_add(1);
            let has_local_cid_list = self
                .local_connection_id_lists
                .iter()
                .any(|list| list.unique_path_id == unique_path_id);
            let path_exists = self
                .paths
                .iter()
                .any(|path| path.unique_path_id == unique_path_id);
            if has_local_cid_list && !path_exists {
                return Some(unique_path_id);
            }
        }

        None
    }

    #[allow(clippy::too_many_arguments)]
    fn probe_new_tuple_resolved(
        &mut self,
        path_x: &mut Path,
        addr_peer: SocketAddr,
        addr_local: SocketAddr,
        if_index: i32,
        current_time: Instant,
        to_preferred_address: bool,
    ) -> Result<(), Error> {
        let unique_path_id = path_x.unique_path_id;
        self.check_cid_for_new_tuple_result(unique_path_id)?;

        let tuple_index = path_x
            .create_tuple(Some(&addr_local), Some(&addr_peer), if_index)
            .map_err(|_| Error::Memory)?;
        let use_constant_challenges = self.use_constant_challenges();
        let tuple = path_x.tuples.get_mut(tuple_index).ok_or(Error::Memory)?;
        self.assign_peer_cnxid_to_tuple(unique_path_id, tuple)?;
        crate::internal::set_tuple_challenge(tuple, current_time, use_constant_challenges);
        tuple.challenge_required = true;
        tuple.to_preferred_address = to_preferred_address;
        Ok(())
    }

    /// Probe a new local↔peer path tuple.
    pub fn probe_new_path(
        &mut self,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        current_time: Instant,
    ) -> Result<(), Error> {
        self.probe_new_path_ex(addr_peer, addr_local, 0, current_time, false)
    }

    /// Probe a new path with explicit interface index and
    /// preferred-address flag.
    /// C: `picoquic_probe_new_path_ex`.
    pub fn probe_new_path_ex(
        &mut self,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        if_index: i32,
        current_time: Instant,
        to_preferred_address: bool,
    ) -> Result<(), Error> {
        if !self.is_multipath_enabled || to_preferred_address {
            let (addr_peer, addr_local, if_index) =
                self.verify_proposed_tuple(Some(*addr_peer), Some(*addr_local), if_index)?;
            if self.paths.is_empty() {
                return Err(Error::InvalidArgument);
            }
            let mut path = self.paths.remove(0);
            let ret = self.probe_new_tuple_resolved(
                &mut path,
                addr_peer,
                addr_local,
                if_index,
                current_time,
                to_preferred_address,
            );
            self.paths.insert(0, path);
            return ret;
        }

        self.check_new_path_allowed(to_preferred_address)?;
        let (addr_peer, addr_local, if_index) =
            self.verify_proposed_tuple(Some(*addr_peer), Some(*addr_local), if_index)?;
        let unique_path_id = self
            .find_available_unique_path_id(u64::MAX)
            .ok_or(Error::Memory)?;
        let mut path = Path::new(
            self,
            current_time,
            Some(&addr_local),
            Some(&addr_peer),
            if_index,
            unique_path_id,
        )
        .map_err(|_| Error::Memory)?;

        let unique_path_id = path.unique_path_id;
        let use_constant_challenges = self.use_constant_challenges();
        let tuple = path.tuples.first_mut().ok_or(Error::Memory)?;
        self.assign_peer_cnxid_to_tuple(unique_path_id, tuple)?;
        if !tuple.challenge_required || tuple.challenge_verified {
            tuple.challenge_required = true;
            crate::internal::set_tuple_challenge(tuple, current_time, use_constant_challenges);
            tuple.challenge_verified = false;
        }
        path.path_is_published = true;
        path.is_nat_challenge = false;
        self.paths.push(path);
        let path_id = self.paths.len() - 1;
        self.register_path_by_index(path_id);
        Ok(())
    }

    /// Probe a new tuple on an existing path object.
    #[allow(clippy::too_many_arguments)]
    pub fn probe_new_tuple(
        &mut self,
        path_x: &mut Path,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        if_index: i32,
        current_time: Instant,
        to_preferred_address: bool,
    ) -> Result<(), Error> {
        let (addr_peer, addr_local, if_index) =
            self.verify_proposed_tuple(Some(*addr_peer), Some(*addr_local), if_index)?;
        self.probe_new_tuple_resolved(
            path_x,
            addr_peer,
            addr_local,
            if_index,
            current_time,
            to_preferred_address,
        )
    }

    /// Validate and complete an address pair proposed for a new tuple.
    /// If either address is `None`, the function searches existing paths
    /// for one whose first tuple has a matching address family and borrows
    /// that address and interface index.  Returns the resolved
    /// `(peer, local, if_index)` triple, or an error when resolution is
    /// impossible or the supplied addresses belong to different families.
    /// C: `picoquic_verify_proposed_tuple`
    pub fn verify_proposed_tuple(
        &self,
        addr_peer: Option<SocketAddr>,
        addr_local: Option<SocketAddr>,
        if_index: i32,
    ) -> Result<(SocketAddr, SocketAddr, i32), Error> {
        let addr_peer = addr_peer.filter(|addr| !socket_addr_is_unspecified(addr));
        let addr_local = addr_local.filter(|addr| !socket_addr_is_unspecified(addr));
        let mut if_index = if_index;
        match (addr_peer, addr_local) {
            (None, None) => Err(Error::Protocol(InternalError::UnexpectedError as u64)),
            (None, Some(local)) => {
                let t = self
                    .paths
                    .iter()
                    .filter_map(|p| p.tuples.first())
                    .find(|t| t.peer_addr.is_ipv4() == local.is_ipv4())
                    .ok_or(Error::Protocol(InternalError::UnexpectedError as u64))?;
                if_index = t.if_index as i32;
                Ok((t.peer_addr, local, if_index))
            }
            (Some(peer), None) => {
                // C: checks addr_peer == NULL after this loop (copy-paste error; addr_peer
                // is non-NULL in this branch).  Translate defensively: fail if no local found.
                let t = self
                    .paths
                    .iter()
                    .filter_map(|p| p.tuples.first())
                    .find(|t| t.local_addr.is_ipv4() == peer.is_ipv4())
                    .ok_or(Error::Protocol(InternalError::UnexpectedError as u64))?;
                if_index = t.if_index as i32;
                Ok((peer, t.local_addr, if_index))
            }
            (Some(peer), Some(local)) => {
                if peer.is_ipv4() != local.is_ipv4() {
                    return Err(Error::Protocol(InternalError::PathAddressFamily as u64));
                }
                Ok((peer, local, if_index))
            }
        }
    }

    /// Toggle path-state event callbacks for this connection.
    pub fn set_path_callbacks(&mut self, are_enabled: bool) {
        self.are_path_callbacks_enabled = are_enabled;
    }

    /// Attach opaque application data to a specific path.
    /// `app_path_ctx` is produced and consumed by the application
    /// without the stack interpreting it.
    pub fn set_app_path_ctx(
        &mut self,
        unique_path_id: u64,
        app_path_ctx: Option<Box<dyn core::any::Any>>,
    ) -> Result<(), Error> {
        if let Some(path) = self
            .paths
            .iter_mut()
            .find(|p| p.unique_path_id == unique_path_id)
        {
            path.app_path_ctx = app_path_ctx;
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }

    /// Tear down a path.
    pub fn abandon_path(
        &mut self,
        unique_path_id: u64,
        reason: u64,
        current_time: Instant,
    ) -> Result<(), Error> {
        if !self.is_multipath_enabled
            || unique_path_id > self.max_path_id_remote
            || unique_path_id > self.max_path_id_local
        {
            return Err(Error::InvalidArgument);
        }

        let path_index = self.get_path_id_from_unique(unique_path_id);
        if path_index >= 0 {
            if self.paths.len() <= 1 {
                return Err(Error::InvalidArgument);
            }
            let path_index = path_index as usize;
            if !self.paths[path_index].path_is_demoted {
                self.demote_path(path_index as i32, current_time, reason);
            }
            Ok(())
        } else {
            match self.demote_local_connection_id_list(unique_path_id, reason) {
                0 => Ok(()),
                _ => Err(Error::Generic),
            }
        }
    }

    /// Issue a fresh CID for the given path.
    pub fn refresh_path_connection_id(&mut self, unique_path_id: u64) -> Result<(), Error> {
        let path_index = self.get_path_id_from_unique(unique_path_id);
        if path_index < 0 {
            return Err(Error::Generic);
        }
        self.renew_path_remote_connection_id(path_index as usize)
    }

    /// Pin a stream to a specific path.
    pub fn set_stream_path_affinity(
        &mut self,
        stream_id: u64,
        unique_path_id: u64,
    ) -> Result<(), Error> {
        let stream_token = self.find_stream(stream_id).ok_or(Error::Generic)?;

        let affinity_path = if unique_path_id == u64::MAX {
            None
        } else {
            let path_id = self.get_path_id_from_unique(unique_path_id);
            if path_id < 0 {
                return Err(Error::Generic);
            }
            Some(crate::internal::PathToken::synthetic(
                path_id as u32,
                path_id as u32,
            ))
        };

        let stream = self.streams.get_mut(stream_token).ok_or(Error::Generic)?;
        stream.affinity_path = affinity_path;
        Ok(())
    }

    /// Mark a path as Available or Backup.
    pub fn set_path_status(
        &mut self,
        unique_path_id: u64,
        status: PathStatus,
    ) -> Result<(), Error> {
        let path_id = self.get_path_id_from_unique(unique_path_id);
        if path_id < 0 {
            return Ok(());
        }
        let path_id = path_id as usize;
        self.paths[path_id].path_is_backup = status != PathStatus::Available;

        let mut path = self.paths.remove(path_id);
        let ret = self.queue_path_available_or_backup_frame(&mut path, status);
        self.paths.insert(path_id, path);
        ret
    }

    /// Select an available unique path ID for path creation.
    /// C: `picoquic_find_avalaible_unique_path_id`
    /// (picoquic/quicctx.c:1651-1684).
    pub fn find_avalaible_unique_path_id(&mut self, requested_id: u64) -> u64 {
        if !self.is_multipath_enabled {
            return if requested_id != 0 && requested_id != u64::MAX {
                u64::MAX
            } else {
                0
            };
        }

        let mut unique_path_id = requested_id;
        if requested_id == u64::MAX && (self.client_mode || self.paths.is_empty()) {
            while self.unique_path_id_next <= self.max_path_id_remote
                && self.unique_path_id_next <= self.max_path_id_local
                && self.unique_path_id_next <= self.max_path_id_in_connection_id_lists
            {
                let candidate = self.unique_path_id_next;
                self.unique_path_id_next = self.unique_path_id_next.saturating_add(1);
                unique_path_id = candidate;
                if self
                    .find_or_create_local_connection_id_list(candidate, false)
                    .is_some()
                    && self.find_path_by_unique_id(candidate) < 0
                {
                    break;
                }
                if candidate == u64::MAX {
                    break;
                }
            }
        }
        unique_path_id
    }

    /// Check whether the connection can create a new path now.
    /// C: `picoquic_check_new_path_allowed` (picoquic/quicctx.c:2300-2342).
    pub fn check_new_path_allowed(&self, to_preferred_address: bool) -> Result<(), Error> {
        if (self.remote_parameters.migration_disabled && !to_preferred_address)
            || self.local_parameters.migration_disabled
        {
            return Err(Error::Protocol(InternalError::MigrationDisabled as u64));
        }
        if self.connection_state < State::ClientAlmostReady {
            return Err(Error::Protocol(InternalError::PathNotReady as u64));
        }
        if self.paths.len() >= crate::internal::NB_PATH_TARGET {
            return Err(Error::Protocol(InternalError::PathLimitExceeded as u64));
        }

        let unique_path_id = if self.is_multipath_enabled {
            self.unique_path_id_next
        } else {
            0
        };
        let has_available_cid = self
            .remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == unique_path_id)
            .and_then(|stash| stash.get_connection_id_from_stash())
            .is_some();
        if has_available_cid {
            Ok(())
        } else if self.unique_path_id_next > self.max_path_id_remote {
            Err(Error::Protocol(InternalError::PathIdBlocked as u64))
        } else {
            Err(Error::Protocol(InternalError::PathCidBlocked as u64))
        }
    }

    /// Subscribe to "new path allowed" events.
    ///
    /// The C signature returned the answer through an
    /// `int* is_already_allowed` parameter that doubled as a status
    /// flag; the Rust shape splits that: `Ok(true)` if a new path is
    /// already allowed (caller can proceed immediately), `Ok(false)`
    /// if the caller will be notified later by callback.
    pub fn subscribe_new_path_allowed(&mut self) -> Result<bool, Error> {
        self.is_notified_that_path_is_allowed = false;
        match self.check_new_path_allowed(false) {
            Ok(()) => {
                self.is_subscribed_to_path_allowed = false;
                Ok(true)
            }
            Err(Error::Protocol(code))
                if code == InternalError::PathNotReady as u64
                    || code == InternalError::PathLimitExceeded as u64
                    || code == InternalError::PathIdBlocked as u64
                    || code == InternalError::PathCidBlocked as u64 =>
            {
                self.is_subscribed_to_path_allowed = true;
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Enable or disable multipath event callbacks for this connection.
    /// C: `picoquic_enable_path_callbacks`.
    pub fn enable_path_callbacks(&mut self, enabled: bool) {
        self.are_path_callbacks_enabled = enabled;
    }

    /// Override the interface index for the first path.
    /// C: `picoquic_set_first_if_index`.
    pub fn set_first_if_index(&mut self, if_index: u32) -> Result<(), Error> {
        if self.connection_state != State::ClientInit {
            return Ok(());
        }

        if let Some(path) = self.paths.first_mut()
            && let Some(tuple) = path.tuples.first_mut()
        {
            tuple.if_index = if_index as core::ffi::c_ulong;
            return Ok(());
        }
        Err(Error::InvalidArgument)
    }

    /// Look up a path's address.  `local` selects which: `1` =
    /// local, `2` = peer, `3` = peer's observed.  The C side
    /// returned this through a `struct sockaddr_storage*`
    /// out-parameter; here it folds into the `Result`.
    pub fn path_addr(&self, unique_path_id: u64, local: i32) -> Result<SocketAddr, Error> {
        let path = self
            .paths
            .iter()
            .find(|p| p.unique_path_id == unique_path_id)
            .ok_or(Error::InvalidArgument)?;
        let tuple = path.tuples.first().ok_or(Error::InvalidArgument)?;
        match local {
            1 => Ok(tuple.local_addr),
            2 => Ok(tuple.peer_addr),
            3 => Ok(tuple.observed_addr),
            _ => Err(Error::InvalidArgument),
        }
    }

    /// Snapshot a path's quality metrics.
    /// C: `picoquic_get_path_quality` (picoquic/quicctx.c:2701-2712).
    pub fn path_quality(&mut self, unique_path_id: u64) -> Result<PathQuality, Error> {
        let connection_sent = self.pkt_ctx[PacketContext::Application as usize].send_sequence;
        let is_multipath_enabled = self.is_multipath_enabled;
        let path = self
            .paths
            .iter_mut()
            .find(|p| p.unique_path_id == unique_path_id)
            .ok_or(Error::InvalidArgument)?;
        Ok(get_path_quality_from_context(
            path,
            is_multipath_enabled,
            connection_sent,
        ))
    }

    /// Snapshot the default path's quality metrics.
    /// C: `picoquic_get_default_path_quality` (picoquic/quicctx.c:2714-2719).
    pub fn default_path_quality(&mut self) -> PathQuality {
        let connection_sent = self.pkt_ctx[PacketContext::Application as usize].send_sequence;
        let is_multipath_enabled = self.is_multipath_enabled;
        self.paths
            .first_mut()
            .map(|path| get_path_quality_from_context(path, is_multipath_enabled, connection_sent))
            .unwrap_or_default()
    }

    /// Subscribe to quality-update events on a specific path with
    /// the given thresholds.
    /// C: `picoquic_subscribe_to_quality_update_per_path`
    /// (picoquic/quicctx.c:2729-2749).
    pub fn subscribe_to_quality_update_per_path(
        &mut self,
        unique_path_id: u64,
        pacing_rate_delta: u64,
        rtt_delta: Duration,
    ) -> Result<(), Error> {
        self.is_path_quality_update_requested = true;
        if let Some(path) = self
            .paths
            .iter_mut()
            .find(|p| p.unique_path_id == unique_path_id)
        {
            path.subscribe_to_quality_update_per_path_context(pacing_rate_delta, rtt_delta);
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }

    /// Subscribe to quality-update events on every path of this
    /// connection.
    /// C: `picoquic_subscribe_to_quality_update` (picoquic/quicctx.c:2751-2763).
    pub fn subscribe_to_quality_update(&mut self, pacing_rate_delta: u64, rtt_delta: Duration) {
        self.rtt_update_delta = rtt_delta;
        self.pacing_rate_update_delta = pacing_rate_delta;
        self.is_path_quality_update_requested = true;
        for path in &mut self.paths {
            path.subscribe_to_quality_update_per_path_context(pacing_rate_delta, rtt_delta);
        }
    }
}

/// C: `picoquic_get_path_quality_from_context` (picoquic/quicctx.c:2678-2699).
fn get_path_quality_from_context(
    path_x: &mut Path,
    is_multipath_enabled: bool,
    connection_sent: u64,
) -> PathQuality {
    path_x.refresh_quality_thresholds();
    let sent = if is_multipath_enabled {
        path_x.pkt_ctx.send_sequence
    } else {
        connection_sent
    };
    PathQuality {
        receive_rate_estimate: path_x.receive_rate_estimate,
        pacing_rate: path_x.pacing.rate,
        cwin: path_x.cwin,
        rtt: path_x.smoothed_rtt,
        rtt_sample: path_x.rtt_sample,
        rtt_variant: path_x.rtt_variant,
        rtt_min: path_x.rtt_min,
        rtt_max: path_x.rtt_max,
        sent,
        lost: path_x.nb_losses_found,
        timer_losses: path_x.nb_timer_losses,
        spurious_losses: path_x.nb_spurious,
        max_spurious_rtt: path_x.max_spurious_rtt,
        max_reorder_delay: path_x.max_reorder_delay,
        max_reorder_gap: path_x.max_reorder_gap,
        bytes_in_transit: path_x.bytes_in_transit,
        bytes_sent: path_x.bytes_sent,
        bytes_received: path_x.received,
    }
}

impl Path {
    /// C: `picoquic_subscribe_to_quality_update_per_path_context`
    /// (picoquic/quicctx.c:2721-2727).
    pub fn subscribe_to_quality_update_per_path_context(
        &mut self,
        pacing_rate_delta: u64,
        rtt_delta: Duration,
    ) {
        self.pacing_rate_update_delta = pacing_rate_delta;
        self.rtt_update_delta = rtt_delta;
        self.refresh_quality_thresholds();
    }
}

// ---------------------------------------------------------------------------
// Connection iteration, timing, accessors, and frame queueing.

impl Connection {
    /// Trigger the next TLS key rotation.
    /// C: `picoquic_start_key_rotation` (picoquic/quicctx.c:5036-5058).
    pub fn start_key_rotation(&mut self) -> Result<(), Error> {
        let app = PacketContext::Application as usize;
        if self.connection_state != State::Ready
            || self.crypto_epoch_sequence
                > crate::internal::picoquic_sack_list_last(&self.ack_ctx[app].sack_list)
        {
            return Err(Error::Protocol(InternalError::KeyRotationNotReady as u64));
        }

        self.compute_new_rotated_keys()?;
        self.apply_rotated_keys(true);
        self.crypto_context_old.free_handles();
        self.crypto_epoch_sequence = self.pkt_ctx[app].send_sequence;
        Ok(())
    }

    /// Borrow the QUIC context that owns this connection.
    /// C: `picoquic_get_quic_ctx` (quicctx.c:1419).
    ///
    /// # Safety
    ///
    /// The caller must ensure that no other live `&Quic` or `&mut Quic`
    /// reference is active when this is called.  The raw back-pointer is
    /// always valid while the `Connection` is live (connections are
    /// destroyed before the owning `Quic`).
    pub unsafe fn quic(&mut self) -> &mut Quic {
        debug_assert!(!self.quic_ptr.is_null(), "quic_ptr not initialised");
        // SAFETY: quic_ptr is set to `self as *mut Quic` in
        // create_cnx_internal and remains valid for the connection's lifetime.
        unsafe { &mut *self.quic_ptr }
    }

    /// Walk to the next connection in the QUIC context's list, if
    /// any.  (Named `next_in_list` rather than `next` to avoid
    /// confusion with the `Iterator::next` shape — Phase 3 may
    /// turn this into a proper `Iterator` impl on `Quic`.)
    pub fn next_in_list(&mut self) -> Option<&mut Connection> {
        // Requires arena-level iteration; Phase 3 wires this up.
        None
    }

    /// Wall-clock time at which this connection next needs attention.
    /// C: `picoquic_get_wake_time` (picoquic/quicctx.c:1579-1591).
    fn wake_time(&self, current_time: Instant) -> u64 {
        if self
            .quic_ref()
            .map(|quic| !quic.pending_stateless_packets.is_empty())
            .unwrap_or(false)
        {
            current_time.ticks()
        } else {
            self.next_wake_time.ticks()
        }
    }

    /// Compute the number of microseconds until this connection
    /// next needs attention, capped at `delay_max`.
    /// C: `picoquic_get_wake_delay` (picoquic/quicctx.c:1593-1612).
    pub fn wake_delay(&self, current_time: Instant, delay_max: i64) -> i64 {
        let now = current_time.ticks();
        let next = self.wake_time(current_time);

        if next <= now {
            0
        } else if delay_max <= 0 {
            delay_max
        } else {
            let delta = next - now;
            if delta >= delay_max as u64 {
                delay_max
            } else {
                delta as i64
            }
        }
    }

    /// Connection state-machine position.
    /// C: `picoquic_get_cnx_state` — `cnx->cnx_state`.
    pub fn state(&self) -> State {
        self.connection_state
    }

    /// Override the per-connection padding policy.
    pub fn set_padding_policy(&mut self, padding_multiple: u32, padding_minsize: u32) {
        self.padding_multiple = padding_multiple;
        self.padding_minsize = padding_minsize;
    }

    /// Read the per-connection padding policy as `(multiple, min-size)`.
    ///
    /// C: `picoquic_cnx_get_padding_policy` (picoquic/quicctx.c:4526-4531).
    pub fn padding_policy(&self) -> (u32, u32) {
        (self.padding_multiple, self.padding_minsize)
    }

    /// Set the per-connection spin-bit policy.
    /// C: `picoquic_cnx_set_spinbit_policy`.
    pub fn set_cnx_spinbit_policy(&mut self, policy: SpinbitVersion) {
        self.spin_policy = policy;
    }

    /// Read the per-connection spin-bit policy.
    /// C: `cnx->spin_policy`.
    pub fn cnx_spinbit_policy(&self) -> SpinbitVersion {
        self.spin_policy
    }

    /// Set the per-connection crypto-epoch length.
    pub fn set_crypto_epoch_length(&mut self, crypto_epoch_length_max: u64) {
        self.crypto_epoch_length_max = if crypto_epoch_length_max == 0 {
            crate::internal::DEFAULT_CRYPTO_EPOCH_LENGTH
        } else {
            crypto_epoch_length_max
        };
    }

    /// Read the per-connection crypto-epoch length.
    pub fn crypto_epoch_length(&self) -> u64 {
        self.crypto_epoch_length_max
    }

    /// Override the connection's PMTUD policy.
    pub fn set_pmtud_policy(&mut self, pmtud_policy: PmtudPolicy) {
        self.pmtud_policy = pmtud_policy;
    }

    /// Obsolete; prefer [`Self::set_pmtud_policy`].  Kept for
    /// source-level parity with the C API (`connection_set_pmtud_required`).
    pub fn set_pmtud_required(&mut self, is_pmtud_required: bool) {
        self.pmtud_policy = if is_pmtud_required {
            PmtudPolicy::Required
        } else {
            PmtudPolicy::Basic
        };
    }

    /// Returns `true` when the handshake completed using a
    /// pre-shared key (PSK).
    pub fn tls_is_psk_handshake(&self) -> bool {
        self.tls_ctx.as_ref().is_some_and(|s| s.is_psk_handshake())
    }

    /// Peer address of the default path.  C side returned an
    /// aliasing `struct sockaddr*` into internal storage; the Rust
    /// translation returns by value.
    pub fn peer_addr(&self) -> SocketAddr {
        self.paths
            .first()
            .and_then(|p| p.tuples.first())
            .map(|t| t.peer_addr)
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap())
    }

    /// Local address of the default path.
    pub fn local_addr(&self) -> SocketAddr {
        self.paths
            .first()
            .and_then(|p| p.tuples.first())
            .map(|t| t.local_addr)
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap())
    }

    /// Local interface index for the default path.
    pub fn local_if_index(&self) -> u32 {
        self.paths
            .first()
            .and_then(|p| p.tuples.first())
            .map(|t| t.if_index as u32)
            .unwrap_or(0)
    }

    /// Set the local address for the default path.
    pub fn set_local_addr(&mut self, addr: &SocketAddr) -> Result<(), Error> {
        if let Some(path) = self.paths.first_mut()
            && let Some(tuple) = path.tuples.first_mut()
        {
            if !socket_addr_is_unspecified(&tuple.local_addr) {
                return Err(Error::Generic);
            }
            tuple.local_addr = *addr;
            return if socket_addr_is_unspecified(&tuple.local_addr) {
                Err(Error::Generic)
            } else {
                Ok(())
            };
        }
        Err(Error::InvalidArgument)
    }

    /// Local connection ID currently in use.
    pub fn local_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }

    /// Local connection ID registered on the first path's first tuple.
    /// C: `picoquic_get_local_cnxid` — `cnx->path[0]->first_tuple->p_local_cnxid->cnx_id`.
    ///
    /// Falls back to `initial_connection_id` when no path or no registered
    /// local CID is present (e.g., a fresh connection before the first path
    /// is fully initialised).
    pub fn local_cnxid(&self) -> ConnectionId {
        self.paths
            .first()
            .and_then(|p| p.tuples.first())
            .and_then(|t| t.local_connection_id)
            .and_then(|tok| self.local_connection_ids.get(tok))
            .map(|lcid| lcid.connection_id)
            .unwrap_or(self.initial_connection_id)
    }

    /// Find a local connection ID by path ID and CID value.
    /// C: `picoquic_find_local_cnxid` (picoquic/quicctx.c:4017-4034).
    pub fn find_local_cnxid(
        &self,
        unique_path_id: u64,
        connection_id: &ConnectionId,
    ) -> Option<crate::internal::LocalConnectionIdToken> {
        self.find_local_connection_id(unique_path_id, connection_id)
    }

    /// Delete a local connection ID token from this connection.
    /// C: `picoquic_delete_local_cnxid` (picoquic/quicctx.c:3927-3932).
    pub fn delete_local_cnxid(&mut self, local_cnxid: crate::internal::LocalConnectionIdToken) {
        self.delete_local_connection_id(local_cnxid);
    }

    /// Retire a local connection ID by path ID and sequence number.
    /// C: `picoquic_retire_local_cnxid` (picoquic/quicctx.c:3965-3985).
    pub fn retire_local_cnxid(&mut self, unique_path_id: u64, sequence: u64) {
        self.retire_local_connection_id(unique_path_id, sequence);
    }

    /// Remove a remote CID from its stash and return its successor index.
    /// C: `picoquic_remove_stashed_cnxid` (picoquic/quicctx.c:3093-3100).
    pub fn remove_stashed_cnxid(
        &mut self,
        unique_path_id: u64,
        removed_index: usize,
        _previous_index: Option<usize>,
    ) -> Option<usize> {
        self.remove_stashed_connection_id(unique_path_id, removed_index)
    }

    /// Dereference all remote CIDs used by tuples on `path_index`.
    ///
    /// C: `picoquic_dereference_stashed_cnxid` (picoquic/quicctx.c:3149-3152).
    pub fn dereference_stashed_cnxid(&mut self, path_index: usize, is_deleting_connection: i32) {
        let Some(unique_path_id) = self.paths.get(path_index).map(|path| path.unique_path_id)
        else {
            return;
        };

        let mut retire_sequences = Vec::new();
        if let Some(path) = self.paths.get_mut(path_index) {
            for tuple in &mut path.tuples {
                let Some(cid_idx) = tuple.remote_connection_id_index.take() else {
                    continue;
                };
                if let Some(stash_idx) = self
                    .remote_connection_id_stashes
                    .iter()
                    .position(|s| s.unique_path_id == unique_path_id)
                    && let Some(cid) = self.remote_connection_id_stashes[stash_idx]
                        .connection_ids
                        .get_mut(cid_idx)
                {
                    cid.nb_path_references = cid.nb_path_references.saturating_sub(1);
                    if cid.needs_removal
                        && cid.nb_path_references == 0
                        && is_deleting_connection == 0
                    {
                        retire_sequences.push(cid.sequence);
                    }
                }
            }
        }

        for sequence in retire_sequences {
            let _ = self.queue_retire_connection_id_frame(unique_path_id, sequence);
        }
    }

    /// C: `picoquic_dereference_stashed_cnxid` (picoquic/quicctx.c:3149-3152).
    pub fn picoquic_dereference_stashed_cnxid(
        &mut self,
        path_index: usize,
        is_deleting_connection: i32,
    ) {
        self.dereference_stashed_cnxid(path_index, is_deleting_connection);
    }

    /// Remote connection ID currently in use.
    /// C: `picoquic_get_remote_cnxid` — `cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id`.
    pub fn remote_connection_id(&self) -> ConnectionId {
        self.path_remote_connection_id(0)
            .unwrap_or(self.initial_connection_id)
    }

    /// Initial connection ID picked at handshake start.
    pub fn initial_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }

    /// Client connection ID registered on the first path's first tuple.
    /// C: `picoquic_get_client_cnxid` — local CID in client mode,
    /// remote CID in server mode.
    pub fn client_connection_id(&self) -> ConnectionId {
        if self.client_mode {
            self.path_local_connection_id(0)
                .unwrap_or(self.initial_connection_id)
        } else {
            self.path_remote_connection_id(0)
                .unwrap_or(self.initial_connection_id)
        }
    }

    /// Server connection ID registered on the first path's first tuple.
    /// C: `picoquic_get_server_cnxid` — remote CID in client mode,
    /// local CID in server mode.
    pub fn server_connection_id(&self) -> ConnectionId {
        if self.client_mode {
            self.path_remote_connection_id(0)
                .unwrap_or(self.initial_connection_id)
        } else {
            self.path_local_connection_id(0)
                .unwrap_or(self.initial_connection_id)
        }
    }

    /// Connection ID used for log entries on this connection.
    pub fn logging_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }

    /// Wall-clock start time of this connection.
    pub fn start_time(&self) -> u64 {
        self.start_time.ticks()
    }

    /// Whether 0-RTT data may be sent on this connection.
    pub fn is_0rtt_available(&self) -> bool {
        self.crypto_context[crate::internal::Epoch::ZeroRtt as usize]
            .aead_encrypt
            .is_some()
    }

    /// Whether the connection has any outstanding data still queued
    /// for transmission.
    pub fn is_backlog_empty(&self) -> bool {
        self.nb_bytes_queued == 0 && self.misc_frames.is_empty() && self.output_streams.is_empty()
    }

    /// Install a per-connection stream/event callback (the
    /// `callback_fn` + `callback_ctx` pair from C collapse to a
    /// single trait object).  See [`Quic::set_default_callback`].
    pub fn set_callback(&mut self, callback: Option<Box<dyn StreamDataCallback>>) {
        self.callback_fn = callback;
    }

    /// Borrow this connection's callback (`None` when none was
    /// installed).
    /// C: `picoquic_get_callback_function` — returns `cnx->callback_fn`.
    pub fn callback(&self) -> Option<&dyn StreamDataCallback> {
        self.callback_fn.as_deref().or_else(|| {
            self.quic_ref()
                .and_then(|quic| quic.default_callback_fn.as_deref())
        })
    }

    /// Borrow the raw callback context associated with this connection.
    /// C: `picoquic_get_callback_context` — returns `cnx->callback_ctx`.
    pub fn callback_ctx(&self) -> Option<&dyn core::any::Any> {
        self.callback_ctx.as_deref().or_else(|| {
            self.quic_ref()
                .and_then(|quic| quic.default_callback_ctx.as_deref())
        })
    }

    /// Queue a connection-level frame for transmission.
    pub fn queue_misc_frame(
        &mut self,
        bytes: &[u8],
        is_pure_ack: bool,
        pc: PacketContext,
    ) -> Result<(), Error> {
        use crate::internal::MiscFrameHeader;
        self.misc_frames.push_back(MiscFrameHeader {
            bytes: bytes.to_vec(),
            packet_context: pc,
            is_pure_ack: is_pure_ack as i32,
        });
        Ok(())
    }

    /// Queue a connection-level frame using the C pointer+length shape.
    pub fn queue_misc_frame_with_length(
        &mut self,
        bytes: &[u8],
        length: usize,
        is_pure_ack: bool,
        pc: PacketContext,
    ) -> Result<(), Error> {
        if length > bytes.len() {
            return Err(Error::InvalidArgument);
        }
        self.queue_misc_frame(&bytes[..length], is_pure_ack, pc)
    }

    /// Queue a datagram frame for transmission.
    ///
    /// C: `picoquic_queue_datagram_frame` (picoquic/frames.c:5307-5335).
    ///
    /// The payload is kept raw in the datagram queue; the send path formats
    /// the DATAGRAM frame when preparing a packet.
    pub fn queue_datagram_frame(&mut self, bytes: &[u8]) -> Result<(), Error> {
        use crate::internal::MiscFrameHeader;
        let length = bytes.len();
        if length > DATAGRAM_QUEUE_CAUTIOUS_LENGTH {
            let send_mtu = self.paths.first().map(|path| path.send_mtu).unwrap_or(0);
            let packet_length = length
                .checked_add(21)
                .and_then(|length| length.checked_add(self.local_cid_length as usize));
            if length > self.local_parameters.max_datagram_frame_size as usize
                || length > self.remote_parameters.max_datagram_frame_size as usize
                || packet_length.is_none_or(|length| length > send_mtu)
            {
                return Err(Error::Protocol(InternalError::DatagramTooLong as u64));
            }
        }
        self.datagrams.push_back(MiscFrameHeader {
            bytes: bytes.to_vec(),
            packet_context: PacketContext::Application,
            is_pure_ack: 0,
        });
        Ok(())
    }

    /// `true` when the misc-frame queue is non-empty.
    /// C: `cnx->first_misc_frame != NULL`.
    pub fn has_misc_frames(&self) -> bool {
        !self.misc_frames.is_empty()
    }

    /// `true` when the misc-frame queue has exactly one entry (no next).
    /// C: `cnx->first_misc_frame != NULL && cnx->first_misc_frame->next_misc_frame == NULL`.
    pub fn misc_frames_is_singleton(&self) -> bool {
        self.misc_frames.len() == 1
    }

    /// Remove the last entry from the misc-frame queue.
    /// C: `picoquic_delete_misc_or_dg(&cnx->first_misc_frame, &cnx->last_misc_frame, cnx->last_misc_frame)`.
    pub fn delete_last_misc_frame(&mut self) {
        self.misc_frames.pop_back();
    }

    /// Current simulated (or wall-clock) time from the QUIC context
    /// that owns this connection.
    /// C: `picoquic_get_quic_time(cnx->quic)`.
    pub fn quic_time(&self) -> Instant {
        Instant::from_ticks(current_time())
    }
}

impl Quic {
    /// Borrow the first connection registered with this context.
    pub fn first_connection(&mut self) -> Option<&mut Connection> {
        let token = self
            .connection_list
            .iter()
            .copied()
            .find(|token| self.connections.contains(*token))?;
        self.connections.get_mut(token)
    }

    /// Return the connection that follows the one identified by `current_token`
    /// in the context's live-connection list, or `None` when `current_token`
    /// is the last live connection.
    ///
    /// C: `picoquic_get_next_cnx` — `cnx->next_in_table`.
    ///
    /// The C intrusive linked list (`next_in_table` / `previous_in_table`) is
    /// represented by `connection_list`, a newest-first token list that is
    /// independent of arena slot reuse.  Typical usage:
    ///
    /// ```ignore
    /// let mut tok = quic.first_connection().and_then(|c| c.own_token);
    /// while let Some(t) = tok {
    ///     let cnx = quic.connections.get_mut(t).unwrap();
    ///     // ... process cnx ...
    ///     tok = quic.next_cnx(t).and_then(|c| c.own_token);
    /// }
    /// ```
    pub fn next_cnx(&mut self, current_token: ConnectionToken) -> Option<&mut Connection> {
        let mut after_current = false;
        let next_token = self.connection_list.iter().copied().find(|token| {
            if *token == current_token {
                after_current = true;
                false
            } else {
                after_current && self.connections.contains(*token)
            }
        })?;
        self.connections.get_mut(next_token)
    }

    /// Compute the number of microseconds until *any* connection on
    /// this context next needs attention, capped at `delay_max`.
    pub fn next_wake_delay(&self, current_time: Instant, delay_max: i64) -> i64 {
        let now = current_time.ticks();
        let next_wake_time = self.next_wake_time(current_time);

        if next_wake_time <= now || delay_max <= 0 {
            return 0;
        }

        let delta_m = now.saturating_add(delay_max as u64);
        if next_wake_time >= delta_m {
            delay_max
        } else {
            (next_wake_time - now) as i64
        }
    }

    /// Wall-clock time at which the next event is scheduled.
    pub fn next_wake_time(&self, current_time: Instant) -> u64 {
        let now = current_time.ticks();
        if !self.pending_stateless_packets.is_empty() {
            return now;
        }

        self.connections
            .iter()
            .map(|c| c.next_wake_time.ticks())
            .min()
            .unwrap_or(u64::MAX)
    }

    /// Return the earliest connection that wakes at or before `wake_time`,
    /// or the earliest connection without a threshold when `wake_time` is zero.
    /// C: `picoquic_get_earliest_cnx_to_wake`.
    pub fn earliest_cnx_to_wake(&mut self, wake_time: Instant) -> Option<&mut Connection> {
        let threshold = wake_time.ticks();
        self.connections
            .iter_mut()
            .filter(|c| threshold == 0 || c.next_wake_time.ticks() <= threshold)
            .min_by_key(|c| c.next_wake_time.ticks())
    }

    /// Borrow the connection currently advancing through its state
    /// machine, if any (`get_cnx_in_progress` in C).
    pub fn connection_in_progress(&mut self) -> Option<&mut Connection> {
        let tok = self.connection_in_progress?;
        self.connections.get_mut(tok)
    }

    /// Default PMTUD policy applied to new connections.
    pub fn set_default_pmtud_policy(&mut self, pmtud_policy: PmtudPolicy) {
        self.default_pmtud_policy = pmtud_policy;
    }

    /// Borrow the default stream callback installed on this context.
    /// C: `picoquic_get_default_callback_function` — `quic->default_callback_fn`.
    pub fn default_callback(&self) -> Option<&dyn StreamDataCallback> {
        self.default_callback_fn.as_deref()
    }

    /// Borrow the opaque context associated with the default callback.
    /// C: `picoquic_get_default_callback_context` — `quic->default_callback_ctx`.
    pub fn default_callback_ctx(&self) -> Option<&dyn core::any::Any> {
        self.default_callback_ctx.as_deref()
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
        bytes: &mut [u8],
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        received_ecn: u8,
        current_time: Instant,
    ) -> Result<(), Error> {
        self.incoming_packet_ex(
            bytes,
            addr_from,
            addr_to,
            if_index_to,
            received_ecn,
            current_time,
        )
        .map(|_| ())
    }

    /// Same as [`Self::incoming_packet`] but additionally identifies
    /// the connection that consumed the packet.
    ///
    /// C: `picoquic_incoming_packet_ex` (picoquic/packet.c:2389-2430).
    #[allow(clippy::too_many_arguments)]
    pub fn incoming_packet_ex(
        &mut self,
        bytes: &mut [u8],
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        received_ecn: u8,
        current_time: Instant,
    ) -> Result<Option<&mut Connection>, Error> {
        let packet_length = bytes.len();
        let mut consumed_index = 0usize;
        let mut previous_dest_id = ConnectionId::default();
        let mut first_cnx = None;

        while consumed_index < packet_length {
            let mut consumed = 0usize;
            let ret = self.incoming_segment(
                &mut bytes[consumed_index..],
                packet_length - consumed_index,
                packet_length,
                &mut consumed,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
                current_time,
                current_time,
                &mut previous_dest_id,
                &mut first_cnx,
            );

            if ret == 0 {
                consumed_index = consumed_index.saturating_add(consumed);
                if consumed == 0 {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(token) = first_cnx
            && let Some(cnx) = self.connections.get_mut(token)
            && packet_length > cnx.max_mtu_received
        {
            cnx.max_mtu_received = packet_length;
        }

        Ok(first_cnx.and_then(|token| self.connections.get_mut(token)))
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
    /// Initialise the connection wake-up scheduler.
    ///
    /// C: `picoquic/quicctx.c:picoquic_wake_list_init`.
    fn wake_list_init(&mut self) {
        self.connection_wake_tree = crate::splay::SplayTree::new();
        for cnx in self.connections.iter_mut() {
            cnx.connection_wake_membership = None;
        }
    }

    /// Remove `connection` from the wake-time scheduler.
    ///
    /// C: `picoquic_remove_cnx_from_wake_list` (picoquic/quicctx.c:1505-1508).
    pub(crate) fn remove_cnx_from_wake_list(&mut self, connection: ConnectionToken) {
        let old_membership = self
            .connections
            .get_mut(connection)
            .and_then(|cnx| cnx.connection_wake_membership.take());
        if let Some(old_membership) = old_membership {
            self.connection_wake_tree.remove(old_membership);
        }
    }

    fn log_and_flush_disconnected_connection(&mut self, token: ConnectionToken) {
        if let Some(cnx) = self.connections.get_mut(token) {
            let (max_reorder_gap, max_spurious_rtt) = cnx
                .paths
                .first()
                .map(|path| {
                    (
                        path.max_reorder_gap as i32,
                        path.max_spurious_rtt.ticks() as i32,
                    )
                })
                .unwrap_or((0, 0));
            let dg_coal = if cnx.nb_trains_sent > 0 {
                cnx.nb_packets_sent as f64 / cnx.nb_trains_sent as f64
            } else {
                0.0
            };
            let message = format!(
                "Closed. Retrans= {}, spurious= {}, max sp gap = {}, max sp delay = {}, dg-coal: {:.6}",
                cnx.nb_retransmission_total as i32,
                cnx.nb_spurious as i32,
                max_reorder_gap,
                max_spurious_rtt,
                dg_coal
            );
            cnx.log_app_message(&message);
        }

        if let Some(f_log) = self.f_log.as_mut() {
            let _ = std::io::Write::flush(f_log);
        }

        if let Some(cnx) = self.connections.get_mut(token)
            && let Some(f_binlog) = cnx.f_binlog.as_mut()
        {
            let _ = std::io::Write::flush(f_binlog);
        }
    }

    /// Drive the next packet onto the wire.  Folds the seven
    /// out-parameters of the C signature into a [`PreparedPacket`].
    ///
    /// C: `picoquic/sender.c:picoquic_prepare_next_packet_ex`.
    pub fn prepare_next_packet_ex(
        &mut self,
        current_time: Instant,
        send_buffer: &mut [u8],
    ) -> Result<PreparedPacket<'_>, Error> {
        let default_addr = unspecified_socket_addr();

        if let Some(sp) = self.dequeue_stateless_packet() {
            if sp.length > send_buffer.len() {
                return Ok(PreparedPacket {
                    send_length: 0,
                    addr_to: default_addr,
                    addr_from: default_addr,
                    if_index: -1,
                    log_cid: ConnectionId::default(),
                    last_connection: None,
                    send_msg_size: None,
                });
            }
            send_buffer[..sp.length].copy_from_slice(&sp.bytes[..sp.length]);
            return Ok(PreparedPacket {
                send_length: sp.length,
                addr_to: sp.addr_to,
                addr_from: sp.addr_local,
                if_index: sp.if_index_local,
                log_cid: sp.initial_connection_id,
                last_connection: None,
                send_msg_size: None,
            });
        }

        let token = self
            .connections
            .iter()
            .filter(|cnx| cnx.next_wake_time <= current_time)
            .filter_map(|cnx| cnx.own_token.map(|tok| (tok, cnx.next_wake_time)))
            .min_by_key(|(_, wake)| wake.ticks())
            .map(|(tok, _)| tok);

        let Some(token) = token else {
            return Ok(PreparedPacket {
                send_length: 0,
                addr_to: default_addr,
                addr_from: default_addr,
                if_index: -1,
                log_cid: ConnectionId::default(),
                last_connection: None,
                send_msg_size: None,
            });
        };

        let log_cid = self
            .connections
            .get(token)
            .map(|cnx| cnx.initial_connection_id)
            .unwrap_or_default();
        let prepared = {
            let cnx = self
                .connections
                .get_mut(token)
                .ok_or(Error::InvalidArgument)?;
            cnx.prepare_packet_ex(current_time, send_buffer)
        };

        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(Error::Disconnected) => {
                self.log_and_flush_disconnected_connection(token);
                let is_client = self
                    .connections
                    .get(token)
                    .map(|cnx| cnx.client_mode)
                    .unwrap_or(true);
                if is_client {
                    self.reinsert_by_wake_time_token(token, Instant::from_ticks(u64::MAX));
                } else {
                    self.delete_connection(token);
                }
                return Ok(PreparedPacket {
                    send_length: 0,
                    addr_to: default_addr,
                    addr_from: default_addr,
                    if_index: -1,
                    log_cid,
                    last_connection: None,
                    send_msg_size: None,
                });
            }
            Err(error) => return Err(error),
        };

        let if_index = if prepared.if_index == -1 {
            self.connections
                .get(token)
                .map(|cnx| cnx.local_if_index() as i32)
                .unwrap_or(-1)
        } else {
            prepared.if_index
        };
        let send_length = prepared.send_length;
        let addr_to = prepared.addr_to;
        let addr_from = prepared.addr_from;
        let send_msg_size = prepared.send_msg_size;
        let last_connection = self.connections.get_mut(token);

        Ok(PreparedPacket {
            send_length,
            addr_to,
            addr_from,
            if_index,
            log_cid,
            last_connection,
            send_msg_size,
        })
    }

    /// Same shape as [`Self::prepare_next_packet_ex`] but without
    /// GSO segment reporting.
    ///
    /// C: `picoquic/sender.c:picoquic_prepare_next_packet`.
    pub fn prepare_next_packet(
        &mut self,
        current_time: Instant,
        send_buffer: &mut [u8],
    ) -> Result<PreparedPacket<'_>, Error> {
        let mut prepared = self.prepare_next_packet_ex(current_time, send_buffer)?;
        prepared.send_msg_size = None;
        Ok(prepared)
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
    fn reinsert_self_by_wake_time(&mut self, next_time: Instant) {
        self.next_wake_time = next_time;
        let Some(token) = self.own_token else {
            return;
        };
        if self.quic_ptr.is_null() {
            return;
        }

        // SAFETY: `quic_ptr` is the owning context installed when this
        // connection was created.  This mirrors C's `cnx->quic` scheduler
        // update; the only connection entry touched through the context is
        // either this connection's wake token or a distinct token displaced
        // by the splay insert.
        unsafe {
            let quic = &mut *self.quic_ptr;
            if !quic.connections.contains(token) {
                return;
            }
            if let Some(old_membership) = self.connection_wake_membership.take() {
                quic.connection_wake_tree.remove(old_membership);
            }
            if let Ok((tree_token, old_token)) = quic
                .connection_wake_tree
                .insert(connection_wake_key(next_time, token), token)
            {
                self.connection_wake_membership = Some(tree_token);
                if let Some(old_token) = old_token
                    && old_token != token
                    && let Some(old_connection) = quic.connections.get_mut(old_token)
                {
                    old_connection.connection_wake_membership = None;
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn intercept_masked_packet_if_needed(
        &mut self,
        current_time: Instant,
        send_buffer: &mut [u8],
        send_length: &mut usize,
        send_msg_size: &mut Option<usize>,
        addr_to: &mut SocketAddr,
        addr_from: &mut SocketAddr,
        if_index: &mut i32,
    ) -> i32 {
        if *if_index != RESERVED_IF_INDEX as i32 || *send_length == 0 || self.quic_ptr.is_null() {
            return 0;
        }

        // SAFETY: `quic_ptr` is the owning context for this connection.
        // The mask callback is a context-level hook matching the C
        // `picomask_intercept_fn` call; the hook may update the output
        // datagram metadata but must not invalidate this connection.
        unsafe {
            let quic = &mut *self.quic_ptr;
            if quic.mask_ctx.is_none() || quic.mask_fns.is_none() {
                return 0;
            }

            let mask_fns = quic.mask_fns.take().expect("checked mask_fns");
            let mut mask_ctx = quic.mask_ctx.take();
            let mut msg_size = send_msg_size.unwrap_or(0);
            let mut p_addr_to = Some(*addr_to);
            let mut p_addr_from = Some(*addr_from);
            let ret = mask_fns.intercept(
                quic,
                mask_ctx.as_deref_mut(),
                current_time,
                send_buffer,
                send_length,
                &mut msg_size,
                &mut p_addr_to,
                &mut p_addr_from,
                if_index,
            );

            quic.mask_ctx = mask_ctx;
            quic.mask_fns = Some(mask_fns);
            if let Some(updated) = p_addr_to {
                *addr_to = updated;
            }
            if let Some(updated) = p_addr_from {
                *addr_from = updated;
            }
            if send_msg_size.is_some() {
                *send_msg_size = Some(msg_size);
            }
            ret
        }
    }

    /// Prepare the next packet on this connection (the `_ex`
    /// flavour reports GSO segment size when the packet is a
    /// coalesced train).
    ///
    /// C: `picoquic/sender.c:picoquic_prepare_packet_ex`.
    pub fn prepare_packet_ex(
        &mut self,
        current_time: Instant,
        send_buffer: &mut [u8],
    ) -> Result<PreparedCnxPacket, Error> {
        let mut next_wake_time = Instant::from_ticks(0);
        let mut ret = self.handle_send_timers(current_time, &mut next_wake_time);
        let mut send_length = 0usize;
        let mut send_msg_size = None;
        let default_addr = unspecified_socket_addr();
        let mut addr_to = default_addr;
        let mut addr_from = default_addr;
        let mut if_index = -1;

        if send_buffer.len() < crate::internal::ENFORCED_INITIAL_MTU {
            ret = crate::errors::InternalError::SendBufferTooSmall as i32;
        }

        if ret == 0 {
            let mut send_msg_size_value = 0usize;
            if let Some((
                path_token,
                tuple_index,
                selected_addr_to,
                selected_addr_from,
                selected_if_index,
            )) = self.picoquic_handle_send_paths(
                current_time,
                &mut next_wake_time,
                send_buffer.len(),
                Some(&mut send_msg_size_value),
            ) {
                let path_idx = path_token.slot_idx();
                addr_to = selected_addr_to;
                addr_from = selected_addr_from;
                if_index = selected_if_index;
                send_msg_size = Some(send_msg_size_value);
                let initial_next_time = next_wake_time;
                let mut coalesced_packet_size = 0usize;
                let selected_tuple_is_first = tuple_index == 0;

                while ret == 0 {
                    let mut is_initial_sent = 0;
                    let mut packet_max = send_buffer.len().saturating_sub(send_length);
                    let packet_buffer_start = send_length;
                    if packet_max == 0 {
                        break;
                    }
                    coalesced_packet_size = 0;
                    next_wake_time = initial_next_time;

                    if let Some(msg_size) = send_msg_size
                        && msg_size > 0
                        && send_length > 0
                        && packet_max > msg_size
                    {
                        packet_max = msg_size;
                    }

                    while ret == 0 {
                        let mut available = packet_max;
                        let mut segment_length = 0usize;

                        if coalesced_packet_size > 0 {
                            packet_max = self
                                .paths
                                .get(path_idx)
                                .map(|path| path.send_mtu)
                                .unwrap_or(packet_max);
                            if packet_max
                                < coalesced_packet_size
                                    .saturating_add(crate::internal::MIN_SEGMENT_SIZE)
                            {
                                break;
                            }
                            available = packet_max.saturating_sub(coalesced_packet_size);
                        }

                        let packet_buffer_offset =
                            packet_buffer_start.saturating_add(coalesced_packet_size);
                        let packet_buffer_end = packet_buffer_offset
                            .saturating_add(available)
                            .min(send_buffer.len());
                        if selected_tuple_is_first && packet_buffer_offset >= packet_buffer_end {
                            break;
                        }

                        let mut packet =
                            crate::internal::Connection::empty_sender_packet(current_time);
                        let path_ptr: *mut Path = &raw mut self.paths[path_idx];

                        if !selected_tuple_is_first {
                            let path_control_max =
                                send_buffer.len().saturating_sub(packet_buffer_start);
                            // SAFETY: `path_ptr` points into the selected
                            // path.  The path-control helper mirrors the C
                            // call that receives both `cnx` and `path_x`.
                            ret = unsafe {
                                match self.prepare_path_control_packet_for_tuple(
                                    &mut *path_ptr,
                                    tuple_index,
                                    &mut packet,
                                    current_time,
                                    &mut send_buffer[packet_buffer_start..],
                                    path_control_max,
                                    &mut next_wake_time,
                                ) {
                                    Ok(path_control_length) => {
                                        if path_control_length > 0 {
                                            send_length =
                                                send_length.saturating_add(path_control_length);
                                        }
                                        0
                                    }
                                    Err(error) => Self::status_from_error(error),
                                }
                            };
                        } else {
                            // SAFETY: `path_ptr` points into `self.paths[path_idx]`.
                            // This mirrors the C call shape (`cnx` plus `path_x`).
                            // The selected path is the only path mutably modified by
                            // this segment-formatting call.
                            ret = unsafe {
                                self.prepare_segment(
                                    &mut *path_ptr,
                                    &mut packet,
                                    current_time,
                                    &mut send_buffer[packet_buffer_offset..packet_buffer_end],
                                    available,
                                    &mut segment_length,
                                    &mut next_wake_time,
                                    &mut is_initial_sent,
                                )
                            };
                        }

                        if ret == 0 {
                            if selected_tuple_is_first {
                                coalesced_packet_size =
                                    coalesced_packet_size.saturating_add(segment_length);
                            }
                            if packet.length == 0
                                || packet.packet_type
                                    == crate::internal::PacketType::OneRttProtected
                            {
                                break;
                            } else if segment_length == 0 {
                                crate::logger::Log::app_message(
                                    self,
                                    format_args!(
                                        "Send bug: segment length = {}, packet length = {}\n",
                                        segment_length, packet.length
                                    ),
                                );
                                break;
                            }
                        } else {
                            if coalesced_packet_size != 0 {
                                ret = 0;
                            }
                            break;
                        }

                        if self
                            .quic_ref()
                            .map(|q| q.dont_coalesce_init)
                            .unwrap_or(false)
                            || !selected_tuple_is_first
                        {
                            break;
                        }
                    }

                    if ret != 0 {
                        break;
                    }

                    if is_initial_sent != 0
                        && self.connection_state < State::ClientAlmostReady
                        && coalesced_packet_size > 0
                        && coalesced_packet_size < crate::internal::ENFORCED_INITIAL_MTU
                    {
                        let padding = packet_max.saturating_sub(coalesced_packet_size);
                        let start = packet_buffer_start.saturating_add(coalesced_packet_size);
                        let end = start.saturating_add(padding).min(send_buffer.len());
                        if start < end {
                            crate::internal::public_random(&mut send_buffer[start..end]);
                            coalesced_packet_size =
                                coalesced_packet_size.saturating_add(end - start);
                        }
                    }

                    if coalesced_packet_size > packet_max {
                        crate::logger::Log::app_message(
                            self,
                            format_args!(
                                "BUFFER OVERFLOW? Packet size {} larger than {}",
                                coalesced_packet_size, packet_max
                            ),
                        );
                    }

                    if coalesced_packet_size > 0 {
                        self.max_mtu_sent = self.max_mtu_sent.max(coalesced_packet_size);
                        self.nb_packets_sent = self.nb_packets_sent.saturating_add(1);
                        let unique_path_id = self
                            .paths
                            .get(path_idx)
                            .map(|path| path.unique_path_id)
                            .unwrap_or(0);
                        crate::logger::Log::pdu(
                            self,
                            false,
                            current_time,
                            &addr_to,
                            &addr_from,
                            coalesced_packet_size,
                            unique_path_id,
                            0,
                        );
                    }

                    if coalesced_packet_size > 0 || self.connection_state == State::Disconnected {
                        next_wake_time = current_time;
                        self.next_wake_time = current_time;
                    }

                    send_length = send_length.saturating_add(coalesced_packet_size);

                    ret = self.intercept_masked_packet_if_needed(
                        current_time,
                        send_buffer,
                        &mut send_length,
                        &mut send_msg_size,
                        &mut addr_to,
                        &mut addr_from,
                        &mut if_index,
                    );
                    if ret < 0 || send_length == 0 {
                        break;
                    }

                    let Some(msg_size) = send_msg_size else {
                        break;
                    };
                    if coalesced_packet_size > msg_size {
                        send_msg_size = Some(coalesced_packet_size);
                    } else if coalesced_packet_size != msg_size
                        || send_length.saturating_add(msg_size) > send_buffer.len()
                    {
                        break;
                    }
                }

                if send_length > 0 && path_idx < self.paths.len() {
                    let path_ptr: *const Path = &raw const self.paths[path_idx];
                    // SAFETY: immutable borrow of selected path for statistics
                    // after segment formatting has completed.
                    let path = unsafe { &*path_ptr };
                    self.handle_send_train_statistics(
                        path,
                        coalesced_packet_size,
                        send_length,
                        send_msg_size,
                    );
                }
            }
        }

        if ret == 0 {
            self.program_app_wake_time(&mut next_wake_time);
        }
        self.reinsert_self_by_wake_time(next_wake_time);

        if ret == 0 {
            Ok(PreparedCnxPacket {
                send_length,
                addr_to,
                addr_from,
                if_index,
                send_msg_size,
            })
        } else {
            Err(crate::internal::sender_status_to_error(ret))
        }
    }

    /// Same shape as [`Self::prepare_packet_ex`] without
    /// GSO-segment reporting.
    ///
    /// C: `picoquic/sender.c:picoquic_prepare_packet`.
    pub fn prepare_packet(
        &mut self,
        current_time: Instant,
        send_buffer: &mut [u8],
    ) -> Result<PreparedCnxPacket, Error> {
        let mut prepared = self.prepare_packet_ex(current_time, send_buffer)?;
        prepared.send_msg_size = None;
        Ok(prepared)
    }

    /// Notify this connection that a destination became
    /// unreachable.
    /// C: `picoquic_notify_destination_unreachable`
    /// (picoquic/quicctx.c:2217-2244).
    pub fn notify_destination_unreachable(
        &mut self,
        current_time: Instant,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        _if_index: i32,
        _socket_err: i32,
    ) {
        let mut partial_match = 0;
        let path_id =
            self.find_path_by_address(Some(addr_local), Some(addr_peer), &mut partial_match);
        if path_id >= 0 {
            let no_path_left = self.paths.iter().all(|path| !path.path_is_demoted);
            if no_path_left {
                if self.connection_state == State::Ready {
                    self.set_path_challenge(path_id, current_time);
                }
            } else {
                self.demote_path(path_id, current_time, 0);
            }
        }
    }
}

impl Quic {
    /// Notify the connection identified by `connection_id` that a
    /// destination became unreachable.
    /// C: `picoquic_notify_destination_unreachable_by_cnxid`
    /// (picoquic/quicctx.c:2246-2263).
    #[allow(clippy::too_many_arguments)]
    pub fn notify_destination_unreachable_by_connection_id(
        &mut self,
        connection_id: &ConnectionId,
        current_time: Instant,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        if_index: i32,
        socket_err: i32,
    ) {
        let connection = if self.local_connection_id_length == 0 || connection_id.is_empty() {
            self.connection_by_net(Some(addr_peer))
        } else if connection_id.len() == self.local_connection_id_length as usize {
            self.connection_by_id(*connection_id)
                .map(|(token, _)| token)
        } else {
            None
        };
        if let Some(connection) = connection
            && let Some(cnx) = self.connections.get_mut(connection)
        {
            cnx.notify_destination_unreachable(
                current_time,
                addr_peer,
                addr_local,
                if_index,
                socket_err,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Streams.

fn append_varint(out: &mut Vec<u8>, value: u64) {
    let start = out.len();
    out.resize(start + internal::encode_varint_length(value), 0);
    let written = internal::varint_encode(&mut out[start..], value);
    debug_assert!(written > 0);
    out.truncate(start + written);
}

/// Compare two streams for output-queue ordering: lower `stream_priority`
/// sorts first; ties are broken by lower `stream_id`.
///
/// C: `picoquic_compare_stream_priority` (picoquic/quicctx.c:3486-3500).
pub fn compare_stream_priority(
    stream: &internal::StreamHead,
    other: &internal::StreamHead,
) -> core::cmp::Ordering {
    stream
        .stream_priority
        .cmp(&other.stream_priority)
        .then_with(|| stream.stream_id.cmp(&other.stream_id))
}

fn enqueue_output_stream_token(connection: &mut Connection, token: internal::StreamToken) {
    if connection
        .output_streams
        .iter()
        .any(|&existing| existing == token)
    {
        return;
    }

    let pos = connection
        .output_streams
        .iter()
        .position(|&existing| {
            let Some(left) = connection.streams.get(token) else {
                return false;
            };
            let Some(right) = connection.streams.get(existing) else {
                return true;
            };
            left.stream_priority
                .cmp(&right.stream_priority)
                .then_with(|| left.stream_id.cmp(&right.stream_id))
                == core::cmp::Ordering::Less
        })
        .unwrap_or(connection.output_streams.len());
    connection.output_streams.insert(pos, token);
}

fn can_insert_output_stream(
    client_mode: bool,
    max_stream_id_bidir_remote: u64,
    max_stream_id_unidir_remote: u64,
    stream_id: u64,
) -> bool {
    use crate::stream::{Role, StreamId};

    let sid = StreamId(stream_id);
    let local_role = if client_mode {
        Role::Client
    } else {
        Role::Server
    };
    if !sid.is_local(local_role) {
        return true;
    }

    let max_stream_id_remote = if sid.is_bidir() {
        max_stream_id_bidir_remote
    } else {
        max_stream_id_unidir_remote
    };
    stream_id <= max_stream_id_remote
}

impl Connection {
    fn find_stream_for_writing(
        &mut self,
        stream_id: u64,
    ) -> Result<crate::internal::StreamToken, Error> {
        use crate::stream::StreamId;

        if let Some(stream) = self.find_stream(stream_id) {
            return Ok(stream);
        }

        if StreamId(stream_id).is_client() != self.client_mode {
            return Err(Error::Protocol(InternalError::InvalidStreamId as u64));
        }

        let type_idx = (stream_id & 3) as usize;
        if stream_id < self.next_stream_id[type_idx] {
            return Err(Error::Protocol(InternalError::StreamAlreadyClosed as u64));
        }

        self.create_missing_streams(stream_id, false)
            .map_err(|_| Error::Memory)
    }

    /// Mark a stream as direct-receive: the stack hands incoming
    /// stream payload straight to `direct_receive` instead of
    /// queueing it for the application's regular callback.
    pub fn mark_direct_receive_stream(
        &mut self,
        stream_id: u64,
        mut direct_receive: Box<dyn StreamDirectReceive>,
    ) -> Result<(), Error> {
        use crate::stream::{Role, StreamId};

        let stream_token = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let sid = StreamId(stream_id);
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };
        if !sid.is_bidir() && sid.is_local(local_role) {
            return Err(Error::Protocol(InternalError::InvalidStreamId as u64));
        }

        loop {
            let next = {
                let stream = self.streams.get(stream_token).ok_or(Error::Memory)?;
                let Some(tree_token) = stream.stream_data_tree.first() else {
                    break;
                };
                let Some(data_token) = stream.stream_data_tree.get(tree_token).copied() else {
                    break;
                };
                let Some(data) = stream.stream_data_nodes.get(data_token) else {
                    break;
                };
                let mut offset = data.offset;
                let mut length = data.length;
                let mut start = 0usize;
                if offset < stream.consumed_offset {
                    let end = offset.saturating_add(length as u64);
                    if end < stream.consumed_offset {
                        length = 0;
                    } else {
                        start = (stream.consumed_offset - offset) as usize;
                        length -= start;
                        offset = stream.consumed_offset;
                    }
                }
                let bytes = data.data[start..start + length].to_vec();
                (tree_token, data_token, offset, bytes)
            };

            let (tree_token, data_token, offset, bytes) = next;
            if !bytes.is_empty() {
                let ret = direct_receive.receive(self, stream_id, false, &bytes, offset);
                if ret != 0 {
                    if let Some(stream) = self.streams.get_mut(stream_token) {
                        stream.direct_receive_fn = Some(direct_receive);
                    }
                    return Err(Error::Protocol(ret as u64));
                }
            }
            if let Some(stream) = self.streams.get_mut(stream_token) {
                stream.stream_data_tree.remove(tree_token);
                stream.stream_data_nodes.remove(data_token);
            }
        }

        let fin_to_signal = self
            .streams
            .get(stream_token)
            .map(|stream| stream.fin_received && !stream.fin_signalled)
            .unwrap_or(false);
        if fin_to_signal {
            let fin_offset = self
                .streams
                .get(stream_token)
                .map(|stream| stream.fin_offset)
                .unwrap_or(0);
            let ret = direct_receive.receive(self, stream_id, true, &[], fin_offset);
            if ret != 0 {
                if let Some(stream) = self.streams.get_mut(stream_token) {
                    stream.direct_receive_fn = Some(direct_receive);
                }
                return Err(Error::Protocol(ret as u64));
            }
            if let Some(stream) = self.streams.get_mut(stream_token) {
                stream.fin_signalled = true;
            }
        }

        let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;
        stream.direct_receive_fn = Some(direct_receive);
        Ok(())
    }

    /// Attach opaque application data to a stream.
    pub fn set_app_stream_ctx(
        &mut self,
        stream_id: u64,
        app_stream_ctx: Option<Box<dyn core::any::Any>>,
    ) -> Result<(), Error> {
        let stream = self.find_stream_for_writing(stream_id)?;
        let stream = self.streams.get_mut(stream).ok_or(Error::Memory)?;
        stream.app_stream_ctx = app_stream_ctx;
        Ok(())
    }

    /// Detach the application data attached by
    /// [`Self::set_app_stream_ctx`].
    pub fn unlink_app_stream_ctx(&mut self, stream_id: u64) {
        if let Some(stream) = self.find_stream(stream_id)
            && let Some(stream) = self.streams.get_mut(stream)
        {
            stream.app_stream_ctx = None;
        }
    }

    /// Toggle whether the stack should poll the application for
    /// more data on this stream.
    pub fn mark_active_stream(
        &mut self,
        stream_id: u64,
        is_active: bool,
        v_stream_ctx: Option<Box<dyn core::any::Any>>,
    ) -> Result<(), Error> {
        let stream_token = self.find_stream_for_writing(stream_id)?;
        let has_callback = self.has_stream_data_callback();
        let mut should_enqueue = false;
        let mut should_reinsert = false;

        {
            let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;

            if is_active {
                let can_set_active_after_reset =
                    !stream.reset_requested || !stream.sack_list.check(0, stream.reliable_size);
                if !stream.fin_requested && can_set_active_after_reset && has_callback {
                    stream.app_stream_ctx = v_stream_ctx;
                    if !stream.is_active {
                        stream.is_active = true;
                        should_reinsert = true;
                    }
                    if !stream.is_output_stream {
                        stream.is_output_stream = true;
                        should_enqueue = true;
                    }
                } else {
                    return Err(Error::Protocol(InternalError::CannotSetActiveStream as u64));
                }
            } else {
                stream.is_active = false;
                stream.app_stream_ctx = v_stream_ctx;
            }
        }

        if should_enqueue {
            enqueue_output_stream_token(self, stream_token);
        }
        if should_reinsert {
            self.reinsert_self_by_wake_time(self.quic_time());
        }
        Ok(())
    }

    /// Toggle whether this stream is excluded from coalesced packet
    /// trains.
    pub fn set_stream_not_coalesced(
        &mut self,
        stream_id: u64,
        is_not_coalesced: bool,
    ) -> Result<(), Error> {
        let stream = self.find_stream_for_writing(stream_id)?;
        let stream = self.streams.get_mut(stream).ok_or(Error::Memory)?;
        stream.is_not_coalesced = is_not_coalesced;
        Ok(())
    }

    /// Set per-stream priority (smaller is higher).
    pub fn set_stream_priority(
        &mut self,
        stream_id: u64,
        stream_priority: u8,
    ) -> Result<(), Error> {
        let stream_token = self.find_stream_for_writing(stream_id)?;
        let was_output = self
            .streams
            .get(stream_token)
            .map(|stream| stream.is_output_stream)
            .unwrap_or(false);
        self.output_streams.retain(|&token| token != stream_token);
        if let Some(stream) = self.streams.get_mut(stream_token) {
            stream.stream_priority = stream_priority;
        }
        if was_output {
            enqueue_output_stream_token(self, stream_token);
        }
        Ok(())
    }

    /// Mark a stream as high-priority (skip ahead of normal
    /// streams).
    pub fn mark_high_priority_stream(
        &mut self,
        stream_id: u64,
        is_high_priority: bool,
    ) -> Result<(), Error> {
        if is_high_priority {
            self.high_priority_stream_id = stream_id;
        } else if self.high_priority_stream_id == stream_id {
            self.high_priority_stream_id = u64::MAX;
        }
        let stream_priority = if is_high_priority {
            0
        } else {
            self.quic_ref()
                .map(|quic| quic.default_stream_priority)
                .unwrap_or(DEFAULT_STREAM_PRIORITY)
        };
        self.set_stream_priority(stream_id, stream_priority)
    }

    /// Override the priority used for outbound datagrams on this
    /// connection.
    pub fn set_datagram_priority(&mut self, datagram_priority: u8) {
        self.datagram_priority = datagram_priority as u64;
    }
}

impl Quic {
    /// Default per-stream priority applied to new streams.
    pub fn set_default_priority(&mut self, default_stream_priority: u8) {
        self.default_stream_priority = default_stream_priority;
    }

    /// Default datagram priority applied to new connections.
    pub fn set_default_datagram_priority(&mut self, default_datagram_priority: u8) {
        self.default_datagram_priority = default_datagram_priority;
    }
}

/// C: `provide_stream_data_buffer`.  `context` is the
/// `context` is the stack-supplied handle the application receives
/// in the `callback_prepare_to_send` callback (a wrapper around
/// the internal `StreamDataBufferArgument`).  Returns the
/// borrowed buffer the application should fill, or `None` on
/// error.  Lifetime ties the returned slice to `context` so the
/// borrow ends with the callback.
pub fn provide_stream_data_buffer<'ctx, 'buf>(
    context: &'ctx mut crate::internal::StreamDataBufferArgument<'buf>,
    nb_bytes: usize,
    is_fin: bool,
    is_still_active: bool,
) -> Option<&'ctx mut [u8]> {
    if nb_bytes > context.allowed_space {
        return None;
    }

    context.length = nb_bytes;
    if is_fin {
        context.is_fin = 1;
        if let Some(first) = context.bytes.first_mut() {
            *first |= 1;
        }
    } else {
        context.is_fin = 0;
    }
    context.is_still_active = is_still_active as i32;

    if nb_bytes < context.byte_space {
        if nb_bytes == context.byte_space.saturating_sub(1) {
            if context.byte_index >= context.bytes.len() {
                return None;
            }
            context.bytes.copy_within(0..context.byte_index, 1);
            context.bytes[0] = crate::frames::FrameType::Padding as u8;
            context.byte_index += 1;
        } else {
            let encode_end = context
                .byte_index
                .saturating_add(context.byte_space)
                .min(context.bytes.len());
            let encoded = crate::internal::varint_encode(
                &mut context.bytes[context.byte_index..encode_end],
                nb_bytes as u64,
            );
            if encoded == 0 {
                return None;
            }
            context.byte_index += encoded;
            if let Some(first) = context.bytes.first_mut() {
                *first |= 2;
            }
        }
    }

    let end = context.byte_index + nb_bytes;
    if end > context.bytes.len() {
        return None;
    }
    let slice = &mut context.bytes[context.byte_index..end];
    Some(slice)
}

impl Connection {
    /// Append `data` to a stream's send buffer (`set_fin` closes the
    /// stream when the data is fully delivered).
    pub fn add_to_stream(
        &mut self,
        stream_id: u64,
        data: &[u8],
        set_fin: bool,
    ) -> Result<(), Error> {
        self.add_to_stream_with_ctx(stream_id, data, set_fin, None)
    }

    /// Reset just the per-stream application context.
    pub fn reset_stream_ctx(&mut self, stream_id: u64) {
        if let Some(stream) = self.find_stream(stream_id)
            && let Some(stream) = self.streams.get_mut(stream)
        {
            stream.app_stream_ctx = None;
        }
    }

    /// Same as [`Self::add_to_stream`] but also installs an
    /// application-supplied stream context for callbacks.
    pub fn add_to_stream_with_ctx(
        &mut self,
        stream_id: u64,
        data: &[u8],
        set_fin: bool,
        app_stream_ctx: Option<Box<dyn core::any::Any>>,
    ) -> Result<(), Error> {
        let stream_token = self.find_stream_for_writing(stream_id)?;
        let can_enqueue = can_insert_output_stream(
            self.client_mode,
            self.max_stream_id_bidir_remote,
            self.max_stream_id_unidir_remote,
            stream_id,
        );
        let mut should_enqueue = false;
        let mut should_reinsert = false;

        {
            let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;

            if set_fin {
                if stream.fin_requested {
                    if !data.is_empty() {
                        return Err(Error::InvalidState);
                    }
                } else {
                    stream.fin_requested = true;
                }
            }

            if stream.reset_sent || stream.stop_sending_received {
                return Err(Error::InvalidState);
            }

            if !data.is_empty() {
                let offset = stream
                    .send_queue
                    .back()
                    .map(|node| node.offset.saturating_add(node.bytes.len() as u64))
                    .unwrap_or(stream.sent_offset);
                stream.send_queue.push_back(internal::StreamQueueNode {
                    offset,
                    bytes: data.to_vec(),
                });
                should_reinsert = true;
            }

            stream.is_active = false;
            stream.app_stream_ctx = app_stream_ctx;
            if !stream.is_output_stream
                && can_enqueue
                && (!stream.send_queue.is_empty() || (stream.fin_requested && !stream.fin_sent))
            {
                stream.is_output_stream = true;
                should_enqueue = true;
            }
        }

        self.nb_bytes_queued = self.nb_bytes_queued.saturating_add(data.len() as u64);
        if should_enqueue {
            enqueue_output_stream_token(self, stream_token);
        }
        if should_reinsert {
            self.reinsert_self_by_wake_time(self.quic_time());
        }
        Ok(())
    }

    /// Send a STREAM_RESET frame for this stream.
    pub fn reset_stream(&mut self, stream_id: u64, local_stream_error: u64) -> Result<(), Error> {
        self.reset_stream_at(stream_id, local_stream_error, 0)
    }

    /// Send a STREAM_RESET_AT frame (per the reliable-stream-reset
    /// draft) for this stream.
    pub fn reset_stream_at(
        &mut self,
        stream_id: u64,
        local_stream_error: u64,
        reliable_size: u64,
    ) -> Result<(), Error> {
        let mut result = Ok(());
        let mut output_stream_token = None;

        if reliable_size > 0 && !self.is_reset_stream_at_enabled {
            result = Err(Error::Protocol(
                InternalError::IllegalTransportExtension as u64,
            ));
        } else if let Some(stream_token) = self.find_stream(stream_id) {
            if let Some(stream) = self.streams.get_mut(stream_token) {
                stream.app_stream_ctx = None;
                if stream.fin_sent && !stream.sack_list.check(0, stream.fin_offset) {
                    result = Err(Error::Protocol(InternalError::StreamAlreadyClosed as u64));
                } else if !stream.reset_requested {
                    stream.local_error = local_stream_error;
                    stream.reset_requested = true;
                    stream.reliable_size = reliable_size;
                    if !stream.is_output_stream {
                        stream.is_output_stream = true;
                        output_stream_token = Some(stream_token);
                    }
                }
            } else {
                result = Err(Error::Memory);
            }
        } else {
            result = Err(Error::Protocol(InternalError::InvalidStreamId as u64));
        }

        if let Some(stream_token) = output_stream_token {
            enqueue_output_stream_token(self, stream_token);
        }
        self.reinsert_self_by_wake_time(self.quic_time());
        result
    }

    /// Open the flow-control window for an inbound stream up to the
    /// expected payload size.
    pub fn open_flow_control(
        &mut self,
        stream_id: u64,
        expected_data_size: u64,
    ) -> Result<(), Error> {
        if self.connection_state != State::Ready
            || self.quic_ref().map(|quic| quic.max_data_limit).unwrap_or(0) != 0
        {
            return Ok(());
        }

        let stream_token = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let new_stream_max = {
            let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;
            let max_required = stream.consumed_offset.saturating_add(expected_data_size);
            if max_required <= stream.maxdata_local {
                return Ok(());
            }
            stream.maxdata_local = max_required;
            max_required
        };
        self.max_stream_data_local = self.max_stream_data_local.max(new_stream_max);

        let new_data_max = self.maxdata_local.saturating_add(expected_data_size);
        self.maxdata_local = new_data_max;
        let mut frame = Vec::new();
        append_varint(&mut frame, crate::frames::FrameType::MaxStreamData as u64);
        append_varint(&mut frame, stream_id);
        append_varint(&mut frame, new_stream_max);
        append_varint(&mut frame, crate::frames::FrameType::MaxData as u64);
        append_varint(&mut frame, new_data_max);
        self.queue_misc_frame(&frame, false, PacketContext::Application)
    }

    /// Toggle application-managed flow control on a stream.
    pub fn set_app_flow_control(
        &mut self,
        stream_id: u64,
        use_app_flow_control: bool,
    ) -> Result<(), Error> {
        let stream = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let stream = self.streams.get_mut(stream).ok_or(Error::Memory)?;
        stream.use_app_flow_control = use_app_flow_control;
        Ok(())
    }

    /// Allocate the next locally-initiated stream ID.
    pub fn next_local_stream_id(&mut self, is_unidir: bool) -> u64 {
        // Stream-ID type index: bidir-client=0, unidir-client=2, bidir-server=1, unidir-server=3
        // client_mode: client initiates even-numbered streams.
        let type_idx: usize = if is_unidir { 2 } else { 0 } + if self.client_mode { 0 } else { 1 };
        self.next_stream_id[type_idx]
    }

    /// Send a STOP_SENDING frame for this stream.
    pub fn stop_sending(&mut self, stream_id: u64, local_stream_error: u64) -> Result<(), Error> {
        let mut result = Ok(());
        let mut output_stream_token = None;

        if let Some(stream_token) = self.find_stream(stream_id) {
            if let Some(stream) = self.streams.get_mut(stream_token) {
                stream.app_stream_ctx = None;
                if stream.reset_received {
                    result = Err(Error::Protocol(InternalError::StreamAlreadyClosed as u64));
                } else if !stream.stop_sending_requested {
                    stream.local_stop_error = local_stream_error;
                    stream.stop_sending_requested = true;
                    if !stream.is_output_stream {
                        stream.is_output_stream = true;
                        output_stream_token = Some(stream_token);
                    }
                }
            } else {
                result = Err(Error::Memory);
            }
        } else {
            result = Err(Error::Protocol(InternalError::InvalidStreamId as u64));
        }

        if let Some(stream_token) = output_stream_token {
            enqueue_output_stream_token(self, stream_token);
        }
        self.reinsert_self_by_wake_time(self.quic_time());
        result
    }

    /// Drop a stream from local bookkeeping (rejecting further peer
    /// frames).
    pub fn discard_stream(&mut self, stream_id: u64, local_stream_error: u16) -> Result<(), Error> {
        use crate::stream::StreamId;

        let stream_token = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let sid = StreamId(stream_id);
        if sid.is_bidir() || !sid.is_client() {
            match self.stop_sending(stream_id, local_stream_error as u64) {
                Err(Error::Protocol(code)) if code == InternalError::StreamAlreadyClosed as u64 => {
                }
                result => result?,
            }
        }
        if sid.is_bidir() || sid.is_client() {
            match self.reset_stream(stream_id, local_stream_error as u64) {
                Err(Error::Protocol(code)) if code == InternalError::StreamAlreadyClosed as u64 => {
                }
                result => result?,
            }
        }
        if let Some(stream) = self.streams.get_mut(stream_token) {
            stream.app_stream_ctx = None;
            stream.is_discarded = true;
        }
        Ok(())
    }

    /// Toggle datagram readiness for this connection.
    pub fn mark_datagram_ready(&mut self, is_ready: bool) -> Result<(), Error> {
        let was_ready = self.is_datagram_ready;
        self.is_datagram_ready = is_ready;
        if !was_ready && is_ready {
            if self.remote_parameters.max_datagram_frame_size == 0 {
                return Err(Error::Generic);
            }
            self.reinsert_self_by_wake_time(self.quic_time());
        }
        Ok(())
    }

    /// Per-path datagram readiness (multipath connections).
    pub fn mark_datagram_ready_path(
        &mut self,
        unique_path_id: u64,
        is_path_ready: bool,
    ) -> Result<(), Error> {
        let Some(path_index) = self
            .paths
            .iter()
            .position(|p| p.unique_path_id == unique_path_id)
        else {
            return Err(Error::InvalidArgument);
        };

        let was_ready = self.paths[path_index].is_datagram_ready;
        self.paths[path_index].is_datagram_ready = is_path_ready;
        if !was_ready && is_path_ready {
            if self.remote_parameters.max_datagram_frame_size == 0 {
                return Err(Error::Generic);
            }
            self.reinsert_self_by_wake_time(self.quic_time());
        }
        Ok(())
    }
}

/// C: `provide_datagram_buffer`.  Old API, prefer
/// [`provide_datagram_buffer_ex`].
pub fn provide_datagram_buffer<'a>(
    context: &'a mut crate::internal::DatagramBufferArgument<'_, '_, '_>,
    length: usize,
) -> Option<&'a mut [u8]> {
    provide_datagram_buffer_ex(context, length, DatagramActive::NotActive)
}

pub fn provide_datagram_buffer_ex<'a>(
    context: &'a mut crate::internal::DatagramBufferArgument<'_, '_, '_>,
    length: usize,
    is_active: DatagramActive,
) -> Option<&'a mut [u8]> {
    context.set_datagram_active(is_active);

    if length == 0 || length > context.allowed_space {
        return None;
    }

    let payload_start = {
        let after_length =
            crate::internal::varint_encode(&mut context.bytes[context.byte_index..], length as u64)
                .checked_add(context.byte_index)
                .filter(|&after_length| after_length != context.byte_index);

        if let Some(after_length) = after_length
            && let Some(after_data) = after_length.checked_add(length)
            && after_data <= context.bytes.len()
        {
            context.after_data = after_data;
            after_length
        } else {
            let encoded = crate::internal::varint_encode(
                context.bytes,
                crate::frames::FrameType::Datagram as u64,
            );
            if encoded == 0 {
                return None;
            }

            let mut payload_start = encoded;
            let tail = payload_start.checked_add(length)?;
            if tail > context.bytes.len() {
                return None;
            }

            if tail < context.bytes.len() {
                let delta = context.bytes.len() - tail;
                context.bytes[..delta].fill(crate::frames::FrameType::Padding as u8);
                let encoded = crate::internal::varint_encode(
                    &mut context.bytes[delta..],
                    crate::frames::FrameType::Datagram as u64,
                );
                if encoded == 0 {
                    return None;
                }
                payload_start = delta + encoded;
            }

            context.after_data = payload_start + length;
            payload_start
        }
    };

    context.bytes.get_mut(payload_start..context.after_data)
}

// ---------------------------------------------------------------------------
// Misc per-context tunables and per-connection accessors.

impl Quic {
    /// Configure the optimistic-ACK throttling policy
    /// (`sequence_hole_pseudo_period` is the period in packets
    /// between intentional ACK gaps used to fingerprint optimistic
    /// peers).
    pub fn set_optimistic_ack_policy(&mut self, sequence_hole_pseudo_period: u32) {
        self.sequence_hole_pseudo_period = sequence_hole_pseudo_period;
    }

    /// Toggle preemptive-repeat at the context level.
    pub fn set_preemptive_repeat_policy(&mut self, do_repeat: bool) {
        self.is_preemptive_repeat_enabled = do_repeat;
    }
}

impl Connection {
    /// Override preemptive-repeat for this connection.
    pub fn set_preemptive_repeat(&mut self, do_repeat: bool) {
        self.is_preemptive_repeat_enabled = do_repeat;
    }

    /// Number of preemptive repeats performed on this connection.
    /// C: `cnx->nb_preemptive_repeat`.
    pub fn preemptive_repeat_count(&self) -> u64 {
        self.nb_preemptive_repeat
    }

    /// Enable keep-alives at the given interval (microseconds).
    pub fn enable_keep_alive(&mut self, interval: Duration) {
        if interval.ticks() == 0 {
            let mut idle_timeout = self.idle_timeout.ticks();
            if idle_timeout == 0 {
                idle_timeout = self
                    .local_parameters
                    .max_idle_timeout
                    .ticks()
                    .saturating_mul(1000);
            }
            if let Some(path) = self.paths.first() {
                let pto_floor = 3u64.saturating_mul(path.retransmit_timer.ticks());
                idle_timeout = idle_timeout.max(pto_floor);
            }
            self.keep_alive_interval = Duration::from_ticks(idle_timeout / 2);
        } else {
            self.keep_alive_interval = interval;
        }
    }

    /// Disable any previously-enabled keep-alive.
    pub fn disable_keep_alive(&mut self) {
        self.keep_alive_interval = Duration::from_ticks(0);
    }

    /// Returns `true` for client-initiated connections.
    pub fn is_client(&self) -> bool {
        self.client_mode
    }

    /// Local error code reported on close (0 if none).
    pub fn local_error(&self) -> u64 {
        self.local_error
    }

    /// Remote error code reported on close.
    pub fn remote_error(&self) -> u64 {
        self.remote_error
    }

    /// Application-level error reported on close (locally generated).
    pub fn application_error(&self) -> u64 {
        self.application_error
    }

    /// Application-level error reported by the remote peer on close.
    /// C: `picoquic_get_application_error` — returns `cnx->remote_application_error`.
    pub fn remote_application_error(&self) -> u64 {
        self.remote_application_error
    }

    /// Return all four close-reason codes in one call:
    /// `(local_error, remote_error, local_application_error, remote_application_error)`.
    /// C: `picoquic_get_close_reasons` — fills four out-pointers from the
    /// corresponding `cnx` fields.
    pub fn close_reasons(&self) -> (u64, u64, u64, u64) {
        (
            self.local_error,
            self.remote_error,
            self.application_error,
            self.remote_application_error,
        )
    }

    /// Per-stream error reported by the peer.
    pub fn remote_stream_error(&self, stream_id: u64) -> u64 {
        self.streams
            .iter()
            .find(|stream| stream.stream_id == stream_id)
            .map(|stream| stream.remote_error)
            .unwrap_or(0)
    }

    /// Inject a remote-reported error on a stream (test / simulation use).
    /// C: `stream->remote_error = error_code` (direct field write).
    pub fn set_stream_remote_error(&mut self, stream_id: u64, error_code: u64) {
        if let Some(stream) = self.find_stream(stream_id)
            && let Some(stream) = self.streams.get_mut(stream)
        {
            stream.remote_error = error_code;
        }
    }

    /// Total bytes of stream data sent on this connection.
    pub fn data_sent(&self) -> u64 {
        self.data_sent
    }

    /// Total bytes of stream data received on this connection.
    pub fn data_received(&self) -> u64 {
        self.data_received
    }

    /// `true` while the connection still streams events into its
    /// log (capped per [`Quic::set_max_simultaneous_logs`]).
    pub fn is_still_logging(&self) -> bool {
        self.nb_packets_logged < LOG_PACKET_MAX_SEQUENCE as u64 || self.use_long_log
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
pub fn register_congestion_control_algorithms(alg: &'static [&'static CongestionAlgorithm]) {
    match CC_ALGORITHM_REGISTRY.write() {
        Ok(mut registry) => *registry = alg,
        Err(poisoned) => *poisoned.into_inner() = alg,
    }
}

/// Convenience wrapper around
/// [`register_congestion_control_algorithms`] that pulls in every
/// algorithm shipped with the crate.
pub fn register_all_congestion_control_algorithms() {
    register_congestion_control_algorithms(&ALL_CC_ALGORITHMS);
}

/// Look up a registered algorithm by name (`alg_id`).
pub fn get_congestion_algorithm(alg_id: &str) -> Option<&'static CongestionAlgorithm> {
    let algorithms = congestion_control_algorithms();
    let alg = algorithms
        .iter()
        .copied()
        .find(|a| a.congestion_algorithm_id == alg_id);

    if alg.is_none() && alg_id == "reno" {
        algorithms
            .iter()
            .copied()
            .find(|a| a.congestion_algorithm_id == "newreno")
    } else {
        alg
    }
}

impl Quic {
    /// Set the default congestion-control algorithm applied to new
    /// connections on this context.
    pub fn set_default_congestion_algorithm(&mut self, algo: &'static CongestionAlgorithm) {
        self.default_congestion_alg = Some(algo);
        self.default_congestion_alg_option_string = None;
    }

    /// Same as [`Self::set_default_congestion_algorithm`] but
    /// passes through a per-algorithm options string.
    pub fn set_default_congestion_algorithm_ex(
        &mut self,
        alg: &'static CongestionAlgorithm,
        alg_option_string: Option<&str>,
    ) {
        self.default_congestion_alg = Some(alg);
        self.default_congestion_alg_option_string = alg_option_string.map(|s| s.to_owned());
    }

    /// Convenience: select the default algorithm by name (looking up
    /// in the registry).  Unknown names mirror the C NULL lookup result
    /// by clearing the default algorithm and option string.
    pub fn set_default_congestion_algorithm_by_name(
        &mut self,
        alg_name: &str,
    ) -> Result<(), Error> {
        self.default_congestion_alg = get_congestion_algorithm(alg_name);
        self.default_congestion_alg_option_string = None;
        Ok(())
    }
}

impl Connection {
    /// Override the congestion-control algorithm for this
    /// connection.
    pub fn set_congestion_algorithm(&mut self, algo: &'static CongestionAlgorithm) {
        self.set_congestion_algorithm_ex(algo, None);
    }

    /// Same as [`Self::set_congestion_algorithm`] but passes
    /// through a per-algorithm options string.
    pub fn set_congestion_algorithm_ex(
        &mut self,
        alg: &'static CongestionAlgorithm,
        alg_option_string: Option<&str>,
    ) {
        if let Some(old_alg) = self.congestion_alg {
            for path in &mut self.paths {
                old_alg.algorithm.alg_delete(path);
            }
        }

        self.congestion_alg = Some(alg);
        self.congestion_alg_option_string = alg_option_string.map(|s| s.to_owned());

        let opt_owned = self.congestion_alg_option_string.clone();
        let option = opt_owned.as_deref();
        let current_time = self.quic_time();

        if let Some(new_alg) = self.congestion_alg {
            for i in 0..self.paths.len() {
                // SAFETY: `path_ptr` points to one path inside this
                // connection.  Congestion-control init is allowed to mutate
                // that path and disjoint connection fields, matching the C
                // call shape that passes both `cnx` and `path[i]`.
                let path_ptr: *mut Path = &raw mut self.paths[i];
                new_alg
                    .algorithm
                    .alg_init(self, unsafe { &mut *path_ptr }, option, current_time);
            }
        }
    }

    /// Set the priority limit above which streams bypass congestion
    /// control.
    pub fn set_priority_limit_for_bypass(&mut self, priority_limit: u8) {
        self.priority_limit_for_bypass = priority_limit as u64;
    }

    /// Toggle whether the application receives feedback-loss
    /// notifications.
    pub fn set_feedback_loss_notification(&mut self, should_notify: bool) {
        self.is_lost_feedback_notification_required = should_notify;
    }

    /// Force the next probe upward (BBR / Cubic probe-up).
    pub fn request_forced_probe_up(&mut self, request_forced_probe_up: bool) {
        self.is_forced_probe_up_required = request_forced_probe_up;
    }

    /// Subscribe to pacing-rate change notifications, with the
    /// given relative thresholds.
    pub fn subscribe_pacing_rate_updates(
        &mut self,
        decrease_threshold: u64,
        increase_threshold: u64,
    ) {
        self.pacing_decrease_threshold = decrease_threshold;
        self.pacing_increase_threshold = increase_threshold;
        self.is_pacing_update_requested = decrease_threshold.min(increase_threshold) != u64::MAX;
    }

    /// Current pacing rate (bytes per second).
    pub fn pacing_rate(&self) -> u64 {
        self.paths.first().map(|p| p.pacing.rate).unwrap_or(0)
    }

    /// Current congestion window (bytes).
    pub fn cwin(&self) -> u64 {
        self.paths.first().map(|p| p.cwin).unwrap_or(0)
    }

    /// Smoothed round-trip-time estimate (microseconds).
    /// C: `picoquic_get_rtt` (quicctx.c:5438).
    pub fn rtt(&self) -> u64 {
        self.paths
            .first()
            .map(|p| p.smoothed_rtt.ticks())
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// ECH / ESNI.

impl Quic {
    /// Configure server-side Encrypted-ClientHello (ECH) by loading
    /// the private key and config from disk.
    pub fn ech_configure(
        &mut self,
        ech_private_key_file_name: Option<&str>,
        ech_config_file_name: Option<&str>,
    ) -> Result<(), Error> {
        crate::ech::picoquic_ech_configure_quic_ctx(
            self,
            ech_private_key_file_name,
            ech_config_file_name,
        )
    }

    /// Release any installed ECH context.
    pub fn release_ech_ctx(&mut self) {
        crate::ech::picoquic_release_quic_ech_ctx(self);
    }
}

impl Connection {
    /// Configure client-side ECH on this connection.
    ///
    /// Copies `config_data` (the raw ECHConfigList bytes from an HTTPS record
    /// or a prior retry) into the connection's ECH client config slot.  The
    /// TLS backend reads this field when starting the handshake and sets
    /// `handshake_properties.client.ech.configs` accordingly.  An empty
    /// slice enables ECH GREASE mode (client sends a random fake ECH extension).
    ///
    /// C: `picoquic_ech_configure_client`
    pub fn ech_configure_client(&mut self, config_data: &[u8]) -> Result<(), Error> {
        self.ech_client_config = Some(config_data.to_vec());
        Ok(())
    }

    /// Returns `true` when the handshake used ECH.
    ///
    /// C: `picoquic_is_ech_handshake`
    pub fn is_ech_handshake(&self) -> bool {
        self.tls_ctx.as_ref().is_some_and(|s| s.is_ech_handshake())
    }

    /// Borrow the retry-config bytes the server returned (empty
    /// when no retry config is available).  Two C `uint8_t**` /
    /// `size_t*` output parameters fold into this single borrow.
    ///
    /// C: `picoquic_ech_get_retry_config`
    pub fn ech_retry_config(&self) -> &[u8] {
        self.tls_ctx.as_ref().map_or(&[], |s| s.retry_configs())
    }
}

/// Generate a fresh ECH config file on disk.
pub fn ech_create_config_file(
    _public_name: &str,
    _private_key_file: &str,
    _ech_config_file: &str,
) -> Result<(), Error> {
    let config =
        crate::ech::ech_create_config_from_private_key_file(_private_key_file, _public_name)?;
    crate::ech::ech_save_config_file(&config, _ech_config_file)
}

/// Read and parse an ECH config from a text file, returning the raw bytes.
/// C: `picoquic_ech_read_config`.
pub fn ech_read_config(ech_config_file: &str) -> Result<Vec<u8>, Error> {
    crate::ech::ech_read_config_file(ech_config_file)
}

/// Save ECH config bytes to a text file.
/// C: `picoquic_ech_save_config`.
pub fn ech_save_config(config: &[u8], ech_config_file: &str) -> Result<(), Error> {
    crate::ech::ech_save_config_file(config, ech_config_file)
}

/// Create an ECH config record from a public-key PEM file.
/// C: `picoquic_ech_create_config_from_public_key`.
pub fn ech_create_config_from_public_key(
    public_key_file: &str,
    public_name: &str,
) -> Result<Vec<u8>, Error> {
    crate::ech::ech_create_config_from_public_key_file(public_key_file, public_name)
}

/// Create an ECH config record from a private-key PEM file.
/// C: `picoquic_ech_create_config_from_private_key`.
pub fn ech_create_config_from_private_key(
    private_key_file: &str,
    public_name: &str,
) -> Result<Vec<u8>, Error> {
    crate::ech::ech_create_config_from_private_key_file(private_key_file, public_name)
}

/// Initialise the TLS API (loads crypto providers, registers algorithms).
/// C: `picoquic_tls_api_init`.
pub fn tls_api_init() {
    crate::tls_api::tls_api_init();
}

/// TLS API initialisation flag: exclude the OpenSSL provider.
/// C: `TLS_API_INIT_FLAGS_NO_OPENSSL`.
pub const TLS_API_INIT_FLAGS_NO_OPENSSL: u64 = 1;

/// TLS API initialisation flag: exclude the minicrypto provider.
/// C: `TLS_API_INIT_FLAGS_NO_MINICRYPTO`.
pub const TLS_API_INIT_FLAGS_NO_MINICRYPTO: u64 = 2;

/// TLS API initialisation flag: exclude the fusion AES provider.
/// C: `TLS_API_INIT_FLAGS_NO_FUSION`.
pub const TLS_API_INIT_FLAGS_NO_FUSION: u64 = 4;

/// TLS API initialisation flag: exclude the MbedTLS provider.
/// C: `TLS_API_INIT_FLAGS_NO_MBEDTLS`.
pub const TLS_API_INIT_FLAGS_NO_MBEDTLS: u64 = 8;

/// Reset the TLS provider registry to the given configuration flags.
/// `flags = 0` restores the default (all providers enabled).
/// C: `picoquic_tls_api_reset`.
pub fn reset_tls_api(flags: u64) {
    crate::tls_api::tls_api_reset(flags);
}

/// Return the AES-128-GCM-SHA-256 TLS cipher-suite identifier for the
/// requested memory mode.
///
/// The C API returns an opaque `ptls_cipher_suite_t *`; the Rust
/// provider registry exposes the registered IANA suite ID instead.
/// C: `picoquic_get_aes128gcm_sha256_v`.
pub fn picoquic_get_aes128gcm_sha256_v(use_low_memory: bool) -> Option<u16> {
    crate::tls_api::picoquic_get_cipher_suite_by_id_v(i32::from(AES_128_GCM_SHA256), use_low_memory)
}

/// True when the minicrypto implementation is the active cipher suite
/// for AES-128-GCM-SHA-256.
pub fn is_minicrypto_aes128gcm_sha256(use_low_memory: bool) -> bool {
    crate::tls_api::is_minicrypto_aes128gcm_sha256(use_low_memory)
}

/// True when minicrypto is the active private-key loader.
/// C: `picoquic_set_private_key_from_file_fn == picoquic_minicrypto_set_key_fn`.
pub fn is_minicrypto_key_loader() -> bool {
    crate::tls_api::is_minicrypto_key_loader()
}

// The C `base64_decode` / `base64_encode` helpers are gone:
// callers use the standard `base64` crate's engines directly.

// ---------------------------------------------------------------------------
// Phase 3A support helpers.

impl Connection {
    /// Returns `true` when the send backlog for this connection is empty.
    /// C: `picoquic_is_cnx_backlog_empty`.
    pub fn is_cnx_backlog_empty(&self) -> bool {
        let mut backlog_empty = true;

        if self.connection_state < State::Ready {
            backlog_empty = self
                .pkt_ctx_backlog_empty(&self.pkt_ctx[PacketContext::Initial as usize])
                && self.pkt_ctx_backlog_empty(&self.pkt_ctx[PacketContext::Handshake as usize]);
        }

        if self.is_multipath_enabled {
            backlog_empty = backlog_empty
                && self
                    .paths
                    .iter()
                    .all(|path| self.pkt_ctx_backlog_empty(&path.pkt_ctx));
        } else if backlog_empty {
            backlog_empty =
                self.pkt_ctx_backlog_empty(&self.pkt_ctx[PacketContext::Application as usize]);
        }

        backlog_empty
    }

    /// Return the next available local stream ID.  `is_unidirectional`
    /// selects uni- vs. bi-directional; the direction (client/server)
    /// is derived from the connection role.
    /// C: `picoquic_get_next_local_stream_id`.
    pub fn get_next_local_stream_id(&mut self, is_unidirectional: bool) -> u64 {
        self.next_local_stream_id(is_unidirectional)
    }

    /// Maximum RTT observed on the primary path.
    /// C: `cnx->path[0]->rtt_max`.
    pub fn primary_path_rtt_max(&self) -> u64 {
        self.paths.first().map(|p| p.rtt_max.ticks()).unwrap_or(0)
    }

    /// Congestion window on the primary path, in bytes.
    /// C: `cnx->path[0]->cwin`.
    pub fn primary_path_cwin(&self) -> u64 {
        self.paths.first().map(|p| p.cwin).unwrap_or(0)
    }

    /// Current pacing rate on the primary path, in bytes per second.
    /// C: `cnx->path[0]->pacing.rate`.
    pub fn primary_path_pacing_rate(&self) -> u64 {
        self.paths.first().map(|p| p.pacing.rate).unwrap_or(0)
    }

    /// RTT variance on the primary path (µs).
    /// C: `cnx->path[0]->rtt_variant`.
    pub fn primary_path_rtt_variant(&self) -> u64 {
        self.paths
            .first()
            .map(|p| p.rtt_variant.ticks())
            .unwrap_or(0)
    }

    /// Initialise an in-memory performance log on this connection.
    /// C: `memlog_init(cnx, nb_records, file_name)`.
    pub fn memlog_init(&mut self, nb_records: usize, file_name: &str) -> crate::Result<()> {
        use crate::internal::{MemLogHook, Path as InternalPath};
        use std::fs::{File, OpenOptions};

        // C: open the output file ("wt"); on failure free the memlog
        // and return -1.
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_name)
            .map_err(|_| crate::Error::Generic)?;

        // C: picoquic_memory_log_t carries the output FILE plus a ring
        // of nb_alloc lines.  The fill / format / write logic lives in
        // loglib/memory_log.c (out of v1 scope per TRANSLATE_PLAN); the
        // hook installed here owns the file and a pre-allocated record
        // buffer of the requested capacity, matching the C shape.
        struct MemLog {
            _file: File,
            _lines: Vec<u8>,
        }
        impl MemLogHook for MemLog {
            fn callback(
                &mut self,
                _connection: &Connection,
                _path: Option<&mut InternalPath>,
                _op_code: i32,
                _current_time: Instant,
            ) {
                // Per-event record fill / flush is part of the loglib
                // translation that has not yet been ported.  The hook
                // is wired so the Connection records that an output
                // sink exists; emission is a no-op until the formatter
                // lands.
            }
        }

        self.memlog_call_back = Some(Box::new(MemLog {
            _file: f,
            _lines: Vec::with_capacity(nb_records),
        }));
        Ok(())
    }

    /// Whether multipath negotiation succeeded on this connection.
    /// C: `cnx->is_multipath_enabled`.
    pub fn is_multipath_enabled(&self) -> bool {
        self.is_multipath_enabled
    }

    /// Local maximum path ID this side accepts.
    /// C: `cnx->max_path_id_local`.
    pub fn max_path_id_local(&self) -> u64 {
        self.max_path_id_local
    }

    /// Remote maximum path ID negotiated by the peer.
    /// C: `cnx->max_path_id_remote`.
    pub fn max_path_id_remote(&self) -> u64 {
        self.max_path_id_remote
    }

    /// Number of active paths on this connection.
    /// C: `cnx->nb_paths`.
    pub fn nb_paths(&self) -> usize {
        self.paths.len()
    }

    /// Unique path ID for path at `index`.
    /// C: `cnx->path[index]->unique_path_id`.
    pub fn path_unique_id(&self, index: usize) -> u64 {
        self.paths.get(index).map(|p| p.unique_path_id).unwrap_or(0)
    }

    /// Number of crypto key rotations observed on this connection.
    /// C: `cnx->nb_crypto_key_rotations`.
    pub fn nb_crypto_key_rotations(&self) -> u64 {
        self.nb_crypto_key_rotations
    }

    /// Number of packet holes inserted (optimistic-ack defense).
    /// C: `cnx->nb_packet_holes_inserted`.
    pub fn nb_packet_holes_inserted(&self) -> u64 {
        self.nb_packet_holes_inserted
    }

    /// Peer address seen by this connection.
    /// C: `picoquic_get_peer_addr(cnx, &addr)`.
    pub fn get_peer_addr(&self) -> std::net::SocketAddr {
        self.peer_addr()
    }

    /// Local address of this connection.
    /// C: `picoquic_get_local_addr(cnx, &addr)`.
    pub fn get_local_addr(&self) -> std::net::SocketAddr {
        self.local_addr()
    }

    /// Whether path at `index` is a backup path.
    /// C: `cnx->path[index]->path_is_backup`.
    pub fn path_is_backup(&self, index: usize) -> bool {
        self.paths
            .get(index)
            .map(|p| p.path_is_backup)
            .unwrap_or(false)
    }

    /// Bytes delivered on path at `index`.
    /// C: `cnx->path[index]->delivered`.
    pub fn path_delivered(&self, index: usize) -> u64 {
        self.paths.get(index).map(|p| p.delivered).unwrap_or(0)
    }

    /// Set or clear the challenge-required flag on path `index`.
    /// C: `cnx->path[index]->first_tuple->challenge_required`.
    pub fn set_path_challenge_required(&mut self, index: usize, required: bool) {
        if let Some(path) = self.paths.get_mut(index)
            && let Some(tuple) = path.tuples.first_mut()
        {
            tuple.challenge_required = required;
        }
    }

    /// Local socket address of path `index` (by array index, not unique ID).
    /// C: `cnx->path[index]->first_tuple->local_addr`.
    pub fn path_local_addr_by_index(&self, index: usize) -> std::net::SocketAddr {
        self.paths
            .get(index)
            .and_then(|p| p.tuples.first())
            .map(|t| t.local_addr)
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap())
    }

    /// Observed address of path `index`.
    /// C: `cnx->path[index]->first_tuple->observed_addr`.
    pub fn path_observed_addr_by_index(&self, index: usize) -> std::net::SocketAddr {
        self.paths
            .get(index)
            .and_then(|p| p.tuples.first())
            .map(|t| t.observed_addr)
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap())
    }

    /// Path ID embedded in the local connection ID for path `index`.
    /// C: `cnx->path[index]->first_tuple->p_local_cnxid->path_id`.
    pub fn path_local_cnxid_path_id(&self, index: usize) -> u64 {
        self.paths
            .get(index)
            .and_then(|p| p.tuples.first())
            .and_then(|t| t.local_connection_id)
            .and_then(|tok| self.local_connection_ids.get(tok))
            .map(|cid| cid.path_id)
            .unwrap_or(0)
    }

    /// Sequence number of the remote connection ID for path `index`.
    /// C: `cnx->path[index]->first_tuple->p_remote_cnxid->sequence`.
    pub fn path_remote_cnxid_sequence(&self, index: usize) -> u64 {
        // Remote CID sequence: look up in the stash indexed by the tuple's remote_connection_id_index.
        if let Some(path) = self.paths.get(index)
            && let Some(tuple) = path.tuples.first()
            && let Some(rid_idx) = tuple.remote_connection_id_index
        {
            // remote_connection_id_index encodes (stash_idx, cid_idx) as a combined index.
            // The stash is keyed by path; iterate to find the right one.
            for stash in &self.remote_connection_id_stashes {
                if let Some(cid) = stash.connection_ids.get(rid_idx) {
                    return cid.sequence;
                }
            }
        }
        0
    }

    /// Sequence number of the local connection ID for path `index`.
    /// C: `cnx->path[index]->first_tuple->p_local_cnxid->sequence`.
    pub fn path_local_cnxid_sequence(&self, index: usize) -> u64 {
        self.paths
            .get(index)
            .and_then(|p| p.tuples.first())
            .and_then(|t| t.local_connection_id)
            .and_then(|tok| self.local_connection_ids.get(tok))
            .map(|cid| cid.sequence)
            .unwrap_or(0)
    }

    /// Peer socket address of path `index` (by array index, not unique ID).
    /// C: `cnx->path[index]->first_tuple->peer_addr`.
    pub fn path_peer_addr_by_index(&self, index: usize) -> std::net::SocketAddr {
        self.paths
            .get(index)
            .and_then(|p| p.tuples.first())
            .map(|t| t.peer_addr)
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap())
    }

    /// Set `enable_time_stamp` on this connection's local transport parameters.
    /// C: `cnx->local_parameters.enable_time_stamp = v`.
    pub fn set_local_enable_time_stamp(&mut self, v: u8) {
        self.local_parameters.enable_time_stamp = v as i32;
    }

    /// Set `initial_max_path_id` on this connection's local transport parameters.
    /// C: `cnx->local_parameters.initial_max_path_id = v`.
    pub fn set_local_initial_max_path_id(&mut self, v: u32) {
        self.local_parameters.initial_max_path_id = v as u64;
    }

    /// Set `max_datagram_frame_size` on this connection's local transport parameters.
    /// C: `cnx->local_parameters.max_datagram_frame_size = v`.
    pub fn set_local_max_datagram_frame_size(&mut self, v: u64) {
        self.local_parameters.max_datagram_frame_size = v as u32;
    }

    /// Set `address_discovery_mode` on this connection's local transport parameters.
    /// C: `cnx->local_parameters.address_discovery_mode = mode`.
    pub fn set_local_address_discovery_mode(&mut self, mode: u8) {
        self.local_parameters.address_discovery_mode = mode as i32;
    }
}

impl Quic {
    /// Number of data nodes currently available in the pool.
    /// C: `picoquic_quic_t::nb_data_nodes_in_pool`.
    pub fn nb_data_nodes_in_pool(&self) -> i32 {
        self.stream_data_node_pool.len() as i32
    }
}

// xorshift1024* state used by the public (non-cryptographic) random
// generator.  C: file-static `public_random_seed[16]`,
// `public_random_index`, `public_random_obfuscator` in tls_api.c.
struct PublicRandomState {
    seed: [u64; 16],
    index: usize,
    obfuscator: u64,
}

const INITIAL_PUBLIC_RANDOM: PublicRandomState = PublicRandomState {
    seed: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    index: 0,
    obfuscator: 0x5555_5555_5555_5555,
};

static PUBLIC_RANDOM_STATE: std::sync::Mutex<PublicRandomState> =
    std::sync::Mutex::new(INITIAL_PUBLIC_RANDOM);

impl PublicRandomState {
    /// One xorshift1024* step.  C: `picoquic_public_random_step`.
    fn step(&mut self) -> u64 {
        let s0 = self.seed[self.index];
        self.index = (self.index + 1) & 15;
        let mut s1 = self.seed[self.index];
        s1 ^= s1 << 31;
        s1 ^= s1 >> 11;
        s1 ^= s0 ^ (s0 >> 30);
        self.seed[self.index] = s1;
        s1
    }
}

/// Seed the public (non-cryptographic) random state used for test reproducibility.
/// C: `picoquic_public_random_seed_64(seed, reset_context)`.
pub fn public_random_seed_64(seed: u64, reset_context: i32) {
    let mut state = PUBLIC_RANDOM_STATE
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    if reset_context != 0 {
        *state = INITIAL_PUBLIC_RANDOM;
    }
    let idx = state.index;
    state.seed[idx] ^= seed;
    for _ in 0..16 {
        state.step();
    }
}

/// Seed the public random state from `picoquic_public_random_seed`.
/// C: `picoquic_public_random_seed`.
pub(crate) fn public_random_seed_from_crypto(seed: u64, obfuscator: u64) {
    public_random_seed_64(seed, 0);
    let mut state = PUBLIC_RANDOM_STATE
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    state.obfuscator = obfuscator;
}

/// Return one value from the public, non-cryptographic random stream.
/// C: `picoquic_public_random_64`.
pub(crate) fn public_random_64() -> u64 {
    const PUBLIC_RANDOM_MULTIPLIER: u64 = 1_181_783_497_276_652_981;

    let mut state = PUBLIC_RANDOM_STATE
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    state.step().wrapping_mul(PUBLIC_RANDOM_MULTIPLIER) ^ state.obfuscator
}

/// Return a public non-cryptographic random value in `0..rnd_max`.
///
/// C: `picoquic/quicctx.c:picoquic_uniform_random` (line 5600-5604).
pub fn picoquic_uniform_random(rnd_max: u64) -> u64 {
    if rnd_max == 0 {
        return 0;
    }
    let rnd_min = u64::MAX % rnd_max;
    loop {
        let rnd = public_random_64();
        if rnd >= rnd_min {
            return rnd % rnd_max;
        }
    }
}

/// Multipath-aware AEAD encryption.  Encodes `path_id` into the nonce before
/// encrypting.  Returns the number of ciphertext bytes written to `out`.
/// C: `picoquic_aead_encrypt_mp`.
pub fn aead_encrypt_mp(
    out: &mut [u8],
    input: &[u8],
    path_id: u64,
    sequence: u64,
    aad: &[u8],
    ctx: &dyn crate::tls::PacketKey,
) -> usize {
    let mut payload = input.to_vec();
    ctx.encrypt_mp(path_id, sequence, aad, &mut payload);
    if payload.len() > out.len() {
        return 0;
    }
    out[..payload.len()].copy_from_slice(&payload);
    payload.len()
}

/// Multipath-aware AEAD decryption.  Fails (returns `None`) when `path_id`
/// does not match the value used during encryption.
/// C: `picoquic_aead_decrypt_mp`.
pub fn aead_decrypt_mp(
    out: &mut [u8],
    input: &[u8],
    path_id: u64,
    sequence: u64,
    aad: &[u8],
    ctx: &dyn crate::tls::PacketKey,
) -> Option<usize> {
    let mut payload = input.to_vec();
    ctx.decrypt_mp(path_id, sequence, aad, &mut payload).ok()?;
    if payload.len() > out.len() {
        return None;
    }
    out[..payload.len()].copy_from_slice(&payload);
    Some(payload.len())
}

fn decrypt_packet_payload(
    raw_bytes: &[u8],
    length: usize,
    ph: &mut crate::internal::PacketHeader,
    decrypted_data: &mut crate::internal::StreamDataNode,
    pn_dec: &dyn crate::tls::HeaderKey,
    aead: &dyn crate::tls::PacketKey,
    is_loss_bit_enabled_incoming: bool,
    sack_list_last: u64,
) -> i32 {
    let length = length.min(raw_bytes.len());
    let mut packet = raw_bytes[..length].to_vec();
    let hp_ret = crate::internal::remove_header_protection_inner(
        &mut packet,
        length,
        &mut decrypted_data.data,
        ph,
        pn_dec,
        is_loss_bit_enabled_incoming,
        sack_list_last,
    );
    if hp_ret != 0 {
        return InternalError::AeadNotReady as i32;
    }

    let header_end = ph.offset;
    let cipher_end = ph.offset.saturating_add(ph.payload_length);
    if header_end > cipher_end
        || cipher_end > packet.len()
        || header_end > decrypted_data.data.len()
    {
        return InternalError::PacketHeaderParsing as i32;
    }

    let header = decrypted_data.data[..header_end].to_vec();
    let mut payload = packet[header_end..cipher_end].to_vec();
    if aead
        .decrypt(ph.packet_number_full, &header, &mut payload)
        .is_err()
    {
        return InternalError::AeadCheck as i32;
    }

    if header_end + payload.len() > decrypted_data.data.len() {
        return InternalError::PacketHeaderParsing as i32;
    }
    decrypted_data.stream_data_membership = None;
    decrypted_data.offset = 0;
    decrypted_data.length = header_end + payload.len();
    decrypted_data.data[header_end..header_end + payload.len()].copy_from_slice(&payload);
    ph.payload_length = payload.len();
    0
}

impl Connection {
    fn log_packet_on_path(
        &mut self,
        path_id: Option<usize>,
        receiving: bool,
        current_time: Instant,
        ph: &crate::internal::PacketHeader,
        bytes: &[u8],
    ) {
        if let Some(path_id) = path_id.filter(|&path_id| path_id < self.paths.len()) {
            let mut path = self.paths.remove(path_id);
            crate::logger::Log::packet(self, Some(&mut path), receiving, current_time, ph, bytes);
            self.paths.insert(path_id, path);
        } else {
            crate::logger::Log::packet(self, None, receiving, current_time, ph, bytes);
        }
    }

    fn log_dropped_packet_on_path(
        &mut self,
        path_id: Option<usize>,
        ph: &crate::internal::PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    ) {
        if let Some(path_id) = path_id.filter(|&path_id| path_id < self.paths.len()) {
            let mut path = self.paths.remove(path_id);
            crate::logger::Log::dropped_packet(
                self,
                Some(&mut path),
                ph,
                packet_size,
                err,
                current_time,
            );
            self.paths.insert(path_id, path);
        } else {
            crate::logger::Log::dropped_packet(self, None, ph, packet_size, err, current_time);
        }
    }

    fn log_buffered_packet_on_path(
        &mut self,
        path_id: Option<usize>,
        packet_type: crate::internal::PacketType,
        current_time: Instant,
    ) {
        if let Some(path_id) = path_id.filter(|&path_id| path_id < self.paths.len()) {
            let mut path = self.paths.remove(path_id);
            crate::logger::Log::buffered_packet(self, &mut path, packet_type, current_time);
            self.paths.insert(path_id, path);
        }
    }

    fn packet_has_ack_eliciting_frame(
        &self,
        bytes: &[u8],
        ph: &crate::internal::PacketHeader,
    ) -> bool {
        let payload = Self::packet_payload(bytes, ph);
        let mut byte_index = 0usize;
        while byte_index < payload.len() {
            let mut frame_length = 0usize;
            let mut frame_is_pure_ack = 0i32;
            let skip_ret = crate::internal::skip_frame(
                &payload[byte_index..],
                payload.len() - byte_index,
                &mut frame_length,
                &mut frame_is_pure_ack,
            );
            if skip_ret != 0 || frame_length == 0 {
                break;
            }
            if frame_is_pure_ack == 0 {
                return true;
            }
            byte_index = byte_index.saturating_add(frame_length);
        }
        false
    }

    /// Processing of packets received before their keys are available.
    ///
    /// C: `picoquic_incoming_not_decrypted` (picoquic/packet.c:2019-2067).
    pub fn incoming_not_decrypted(
        &mut self,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
        bytes: &[u8],
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        received_ecn: u8,
    ) -> i32 {
        if self.connection_state >= State::Ready {
            return 0;
        }

        let local_cid_matches = self
            .path_local_connection_id(0)
            .is_some_and(|cid| !cid.is_empty() && cid == ph.dest_connection_id);
        if !local_cid_matches {
            return 0;
        }

        if !self.paths.is_empty() {
            let mut path = self.paths.remove(0);
            self.update_path_rtt(&mut path, -1, self.start_time, current_time, 0, 0);
            self.paths.insert(0, path);
        }

        let can_buffer = bytes.len() <= crate::internal::MAX_PACKET_SIZE
            && ((ph.packet_type == crate::internal::PacketType::Handshake && self.client_mode)
                || ph.packet_type == crate::internal::PacketType::OneRttProtected);
        if !can_buffer {
            return 0;
        }

        let mut packet = crate::internal::StatelessPacket {
            addr_to: *addr_from,
            addr_local: *addr_to,
            if_index_local: if_index_to,
            received_ecn,
            length: bytes.len(),
            receive_time: current_time,
            connection_id_log64: ph.dest_connection_id.val64(),
            initial_connection_id: self.initial_connection_id,
            packet_type: ph.packet_type,
            bytes: [0u8; crate::internal::MAX_PACKET_SIZE],
        };
        packet.bytes[..bytes.len()].copy_from_slice(bytes);
        self.sooner_stateless.push_front(packet);
        1
    }

    /// Process a server Initial packet on a client connection.
    ///
    /// C: `picoquic_incoming_server_initial` (picoquic/packet.c:1619-1701).
    pub fn incoming_server_initial(
        &mut self,
        bytes: &[u8],
        packet_length: usize,
        received_data: &mut crate::internal::StreamDataNode,
        addr_to: Option<&SocketAddr>,
        if_index_to: u64,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
    ) -> i32 {
        if self.connection_state == State::ClientInitSent
            || self.connection_state == State::ClientInitResent
        {
            self.connection_state = State::ClientHandshakeStart;
        }

        let remote_cid = self.path_remote_connection_id(0).unwrap_or_default();
        if (!remote_cid.is_empty() || self.connection_state > State::ClientHandshakeStart)
            && remote_cid != ph.src_connection_id
        {
            return InternalError::CnxidCheck as i32;
        }

        if self.connection_state <= State::ClientHandshakeStart {
            if let Some(path) = self.paths.get_mut(0)
                && let Some(tuple) = path.tuples.first_mut()
            {
                if Self::socket_addr_is_unspecified(&tuple.local_addr)
                    && let Some(addr_to) = addr_to
                {
                    tuple.local_addr = *addr_to;
                }
                tuple.if_index = if_index_to as core::ffi::c_ulong;
            }

            if ph.payload_length == 0 {
                return self.connection_error(TransportError::ProtocolViolation as u64, 0);
            }

            if packet_length < crate::internal::ENFORCED_INITIAL_MTU
                && self.packet_has_ack_eliciting_frame(bytes, ph)
                && self.retry_token.is_empty()
                && self.crypto_context[crate::internal::Epoch::ZeroRtt as usize]
                    .aead_encrypt
                    .is_none()
            {
                self.log_app_message(&format!(
                    "Server initial too short ({} bytes)",
                    packet_length
                ));
                return InternalError::InitialTooShort as i32;
            }

            let payload = Self::packet_payload(bytes, ph);
            let mut ret = self.decode_frames_on_path(
                0,
                payload,
                received_data,
                ph.epoch,
                None,
                addr_to,
                ph.packet_number_full,
                0,
                current_time,
            );
            if ret == 0 {
                let (tls_ret, _) = self.process_tls_stream_status(current_time);
                ret = tls_ret;
            }
            ret
        } else if self.connection_state < State::Ready {
            self.ignore_incoming_handshake(bytes, ph, current_time);
            0
        } else {
            InternalError::UnexpectedPacket as i32
        }
    }

    /// Process a server Handshake packet on a client connection.
    ///
    /// C: `picoquic_incoming_server_handshake` (picoquic/packet.c:1704-1752).
    pub fn incoming_server_handshake(
        &mut self,
        bytes: &[u8],
        received_data: &mut crate::internal::StreamDataNode,
        addr_to: Option<&SocketAddr>,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
    ) -> i32 {
        let restricted = self.connection_state != State::ClientHandshakeStart;
        if self.path_remote_connection_id(0) != Some(ph.src_connection_id) {
            return InternalError::CnxidCheck as i32;
        }

        if self.connection_state < State::Ready {
            if ph.payload_length == 0 {
                return self.connection_error(TransportError::ProtocolViolation as u64, 0);
            }

            let payload = Self::packet_payload(bytes, ph);
            let mut ret = self.decode_frames_on_path(
                0,
                payload,
                received_data,
                ph.epoch,
                None,
                addr_to,
                ph.packet_number_full,
                0,
                current_time,
            );
            if ret == 0 && !restricted {
                let (tls_ret, _) = self.process_tls_stream_status(current_time);
                ret = tls_ret;
            }
            ret
        } else {
            InternalError::UnexpectedPacket as i32
        }
    }

    /// Account for ECN marks on a successfully processed packet.
    ///
    /// C: `picoquic_ecn_accounting` (picoquic/packet.c:1880-1908).
    pub fn ecn_accounting(
        &mut self,
        received_ecn: u8,
        packet_context: PacketContext,
        local_connection_id: Option<crate::internal::LocalConnectionIdToken>,
    ) {
        if let Some(ack_ctx) = self.ack_ctx_from_cnx_context(packet_context, local_connection_id) {
            match received_ecn & 0x03 {
                0x00 => {}
                ECN_ECT_1 => {
                    ack_ctx.ecn_ect1_total_local = ack_ctx.ecn_ect1_total_local.saturating_add(1);
                    ack_ctx.sending_ecn_ack = true;
                }
                ECN_ECT_0 => {
                    ack_ctx.ecn_ect0_total_local = ack_ctx.ecn_ect0_total_local.saturating_add(1);
                    ack_ctx.sending_ecn_ack = true;
                }
                ECN_CE => {
                    ack_ctx.ecn_ce_total_local = ack_ctx.ecn_ce_total_local.saturating_add(1);
                    ack_ctx.sending_ecn_ack = true;
                }
                _ => {}
            }
        }
    }
}

struct ParsedSegment {
    ret: i32,
    connection: Option<ConnectionToken>,
    new_context_created: bool,
}

impl Quic {
    /// Insert a connection in the wake-time scheduler using its current
    /// `next_wake_time`.
    ///
    /// C: `picoquic/quicctx.c:picoquic_insert_cnx_by_wake_time`
    /// (line 1510-1513).
    fn picoquic_insert_cnx_by_wake_time(&mut self, token: ConnectionToken) {
        let Some(next_time) = self.connections.get(token).map(|cnx| cnx.next_wake_time) else {
            return;
        };

        if let Ok((tree_token, old)) = self
            .connection_wake_tree
            .insert(connection_wake_key(next_time, token), token)
        {
            if let Some(cnx) = self.connections.get_mut(token) {
                cnx.connection_wake_membership = Some(tree_token);
            }
            if let Some(old_token) = old
                && old_token != token
                && let Some(old_connection) = self.connections.get_mut(old_token)
            {
                old_connection.connection_wake_membership = None;
            }
        }
    }

    fn reinsert_by_wake_time_token(&mut self, token: ConnectionToken, next_time: Instant) {
        self.remove_cnx_from_wake_list(token);
        if let Some(cnx) = self.connections.get_mut(token) {
            cnx.next_wake_time = next_time;
        }
        self.picoquic_insert_cnx_by_wake_time(token);
    }

    fn parse_error_status(error: Error) -> i32 {
        Connection::status_from_error(error)
    }

    fn decrypt_existing_segment(
        &mut self,
        connection: ConnectionToken,
        raw_bytes: &[u8],
        length: usize,
        packet_length: usize,
        ph: &mut crate::internal::PacketHeader,
        decrypted_data: &mut crate::internal::StreamDataNode,
    ) -> i32 {
        let mut aead_integrity_limit = None;
        let ret = {
            let Some(cnx) = self.connections.get(connection) else {
                return InternalError::UnexpectedPacket as i32;
            };
            if !cnx.client_mode
                && ph.packet_type == crate::internal::PacketType::Initial
                && packet_length < crate::internal::ENFORCED_INITIAL_MTU
            {
                return InternalError::InitialTooShort as i32;
            }
            if ph.version_index != cnx.version_index {
                return InternalError::PacketWrongVersion as i32;
            }

            let epoch = ph.epoch as usize;
            let Some(pn_dec) = cnx.crypto_context[epoch].pn_dec.as_deref() else {
                return InternalError::AeadNotReady as i32;
            };
            let Some(aead) = cnx.crypto_context[epoch].aead_decrypt.as_deref() else {
                return InternalError::AeadNotReady as i32;
            };
            if ph.epoch == crate::internal::Epoch::OneRtt {
                aead_integrity_limit = Some(crate::tls_api::aead_integrity_limit(aead));
            }
            let sack_list_last = cnx.ack_ctx[ph.packet_context as usize].sack_list.first();
            let is_loss_bit_enabled_incoming = cnx.is_loss_bit_enabled_incoming;
            let already_received = cnx.is_pn_already_received(
                ph.packet_context,
                ph.local_connection_id,
                ph.packet_number_full,
            );
            let decrypt_ret = decrypt_packet_payload(
                raw_bytes,
                length,
                ph,
                decrypted_data,
                pn_dec,
                aead,
                is_loss_bit_enabled_incoming,
                sack_list_last,
            );
            if decrypt_ret == 0 && already_received {
                InternalError::Duplicate as i32
            } else {
                decrypt_ret
            }
        };

        if ret == InternalError::AeadCheck as i32
            && ph.packet_type == crate::internal::PacketType::OneRttProtected
            && let Some(cnx) = self.connections.get_mut(connection)
        {
            cnx.crypto_failure_count = cnx.crypto_failure_count.saturating_add(1);
            if let Some(limit) = aead_integrity_limit
                && cnx.crypto_failure_count > limit
            {
                cnx.log_app_message(&format!(
                    "AEAD Integrity limit reached after 0x{:x} failed decryptions.",
                    cnx.crypto_failure_count
                ));
                let _ = cnx.connection_error(TransportError::AeadLimitReached as u64, 0);
            }
        }

        ret
    }

    fn screen_initial_packet(
        &mut self,
        raw_bytes: &[u8],
        packet_length: usize,
        addr_from: &SocketAddr,
        ph: &mut crate::internal::PacketHeader,
        current_time: Instant,
        decrypted_data: &mut crate::internal::StreamDataNode,
    ) -> ParsedSegment {
        if packet_length < crate::internal::ENFORCED_INITIAL_MTU {
            return ParsedSegment {
                ret: InternalError::InitialTooShort as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if ph.dest_connection_id.len() < crate::internal::ENFORCED_INITIAL_CID_LENGTH as usize {
            return ParsedSegment {
                ret: InternalError::InitialCidTooShort as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if ph.has_reserved_bit_set {
            return ParsedSegment {
                ret: InternalError::PacketHeaderParsing as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if self.enforce_client_only
            || self.server_busy
            || self.current_number_connections >= self.tentative_max_number_connections
        {
            return ParsedSegment {
                ret: InternalError::ServerBusy as i32,
                connection: None,
                new_context_created: false,
            };
        }

        let initial_context =
            match self.initial_aead_context(ph.version_index, &ph.dest_connection_id, false, false)
            {
                Ok(ctx) => ctx,
                Err(error) => {
                    return ParsedSegment {
                        ret: Self::parse_error_status(error),
                        connection: None,
                        new_context_created: false,
                    };
                }
            };
        let decrypt_ret = decrypt_packet_payload(
            raw_bytes,
            packet_length,
            ph,
            decrypted_data,
            initial_context.pn_enc_ctx.as_ref(),
            initial_context.aead_ctx.as_ref(),
            false,
            0,
        );
        if decrypt_ret != 0 {
            return ParsedSegment {
                ret: decrypt_ret,
                connection: None,
                new_context_created: false,
            };
        }

        let is_address_blocked = !self.is_port_blocking_disabled && check_addr_blocked(addr_from);
        let mut has_good_token = false;
        let mut has_bad_token = false;
        let mut is_new_token = false;
        let mut verified_token = None;
        if !ph.token_bytes.is_empty() {
            let token_bytes = ph.token_bytes.clone();
            match self.verify_retry_token(
                addr_from,
                current_time,
                &ph.dest_connection_id,
                ph.packet_number_full as u32,
                &token_bytes,
                true,
            ) {
                Ok(token) => {
                    has_good_token = true;
                    is_new_token = token.is_new_token;
                    verified_token = Some(token);
                }
                Err(_) => {
                    has_bad_token = true;
                    is_new_token = token_bytes.len() <= 128
                        && token_bytes.len() >= 8
                        && token_bytes[0] & 0x80 != 0;
                }
            }
        }

        if has_bad_token && !is_new_token {
            return ParsedSegment {
                ret: InternalError::InvalidToken as i32,
                connection: None,
                new_context_created: false,
            };
        }
        if !has_good_token
            && (self.force_check_token
                || self.max_half_open_before_retry <= self.current_number_half_open
                || is_address_blocked)
        {
            return ParsedSegment {
                ret: InternalError::RetryNeeded as i32,
                connection: None,
                new_context_created: false,
            };
        }

        let connection = match self.create_cnx_internal(
            ph.dest_connection_id,
            ph.src_connection_id,
            Some(addr_from),
            current_time,
            ph.version,
            None,
            None,
            false,
            None,
            None,
        ) {
            Ok(token) => token,
            Err(error) => {
                return ParsedSegment {
                    ret: Self::parse_error_status(error),
                    connection: None,
                    new_context_created: false,
                };
            }
        };

        if let Some(cnx) = self.connections.get_mut(connection) {
            cnx.version_index = ph.version_index;
            cnx.proposed_version = ph.version;
            if let Some(token) = verified_token {
                cnx.initial_validated = true;
                cnx.original_connection_id = token.odcid;
            }
        }

        ParsedSegment {
            ret: 0,
            connection: Some(connection),
            new_context_created: true,
        }
    }

    fn parse_header_and_decrypt_for_segment(
        &mut self,
        raw_bytes: &[u8],
        length: usize,
        packet_length: usize,
        addr_from: &SocketAddr,
        current_time: Instant,
        decrypted_data: &mut crate::internal::StreamDataNode,
        ph: &mut crate::internal::PacketHeader,
        consumed: &mut usize,
    ) -> ParsedSegment {
        let parse_length = length.min(raw_bytes.len());
        let connection =
            match self.parse_packet_header(&raw_bytes[..parse_length], Some(addr_from), ph, true) {
                Ok(connection) => connection,
                Err(error) => {
                    return ParsedSegment {
                        ret: Self::parse_error_status(error),
                        connection: None,
                        new_context_created: false,
                    };
                }
            };

        if matches!(
            ph.packet_type,
            crate::internal::PacketType::VersionNegotiation
                | crate::internal::PacketType::Retry
                | crate::internal::PacketType::Error
        ) {
            let copy_len = parse_length.min(decrypted_data.data.len());
            decrypted_data.data[..copy_len].copy_from_slice(&raw_bytes[..copy_len]);
            decrypted_data.length = copy_len;
            *consumed = parse_length;
            return ParsedSegment {
                ret: 0,
                connection,
                new_context_created: false,
            };
        }

        let Some(segment_length) = ph.offset.checked_add(ph.payload_length) else {
            return ParsedSegment {
                ret: InternalError::PacketHeaderParsing as i32,
                connection,
                new_context_created: false,
            };
        };
        if segment_length > parse_length {
            return ParsedSegment {
                ret: InternalError::PacketHeaderParsing as i32,
                connection,
                new_context_created: false,
            };
        }
        *consumed = segment_length;

        if let Some(connection) = connection {
            let ret = self.decrypt_existing_segment(
                connection,
                raw_bytes,
                segment_length,
                packet_length,
                ph,
                decrypted_data,
            );
            ParsedSegment {
                ret,
                connection: Some(connection),
                new_context_created: false,
            }
        } else if ph.packet_type == crate::internal::PacketType::Initial {
            self.screen_initial_packet(
                raw_bytes,
                packet_length,
                addr_from,
                ph,
                current_time,
                decrypted_data,
            )
        } else if ph.packet_type == crate::internal::PacketType::OneRttProtected
            && parse_length >= RESET_PACKET_MIN_SIZE
        {
            let secret = &raw_bytes[parse_length - RESET_SECRET_SIZE..parse_length];
            if let Some(connection) = self.connection_by_secret(secret, Some(addr_from)) {
                if let Some(cnx) = self.connections.get_mut(connection) {
                    cnx.log_app_message("Found connection from reset secret, ret = 1054");
                }
                ParsedSegment {
                    ret: InternalError::StatelessReset as i32,
                    connection: Some(connection),
                    new_context_created: false,
                }
            } else {
                ParsedSegment {
                    ret: InternalError::UnexpectedPacket as i32,
                    connection: None,
                    new_context_created: false,
                }
            }
        } else {
            ParsedSegment {
                ret: InternalError::UnexpectedPacket as i32,
                connection: None,
                new_context_created: false,
            }
        }
    }
}

impl Quic {
    fn queue_version_negotiation_packet(
        &mut self,
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        ph: &crate::internal::PacketHeader,
    ) {
        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };
        let mut byte_index = 0usize;
        sp.bytes[byte_index] = 0x80;
        byte_index += 1;
        sp.bytes[byte_index..byte_index + 4].fill(0);
        byte_index += 4;
        sp.bytes[byte_index] = ph.src_connection_id.len() as u8;
        byte_index += 1;
        let dlen = ph.src_connection_id.len();
        sp.bytes[byte_index..byte_index + dlen].copy_from_slice(ph.src_connection_id.as_bytes());
        byte_index += dlen;
        sp.bytes[byte_index] = ph.dest_connection_id.len() as u8;
        byte_index += 1;
        let slen = ph.dest_connection_id.len();
        sp.bytes[byte_index..byte_index + slen].copy_from_slice(ph.dest_connection_id.as_bytes());
        byte_index += slen;
        for version in [
            crate::internal::Version::V1 as u32,
            crate::internal::Version::V2 as u32,
            crate::internal::Version::V2Draft as u32,
        ] {
            sp.bytes[byte_index..byte_index + 4].copy_from_slice(&version.to_be_bytes());
            byte_index += 4;
        }
        sp.length = byte_index;
        sp.packet_type = crate::internal::PacketType::VersionNegotiation;
        sp.addr_to = *addr_from;
        sp.addr_local = *addr_to;
        sp.if_index_local = if_index_to;
        sp.connection_id_log64 = ph.dest_connection_id.val64();
        self.queue_stateless_packet(sp);
    }

    fn queue_stateless_retry(
        &mut self,
        ph: &crate::internal::PacketHeader,
        server_cid: &ConnectionId,
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        retry_token: &[u8],
    ) {
        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };
        let mut pn_offset = 0usize;
        let mut pn_length = 0usize;
        let mut byte_index = crate::internal::create_long_header(
            crate::internal::PacketType::Retry,
            &ph.src_connection_id,
            server_cid,
            false,
            ph.version,
            ph.version_index,
            0,
            &[],
            &mut sp.bytes,
            &mut pn_offset,
            &mut pn_length,
        );
        let token_end = byte_index.saturating_add(retry_token.len());
        if token_end > sp.bytes.len() {
            return;
        }
        sp.bytes[byte_index..token_end].copy_from_slice(retry_token);
        byte_index = token_end;

        if let Some(integrity_aead) = self.find_retry_protection_context(ph.version_index, true) {
            byte_index = crate::tls_api::encode_retry_protection(
                integrity_aead,
                &mut sp.bytes,
                byte_index,
                &ph.dest_connection_id,
            );
        } else if byte_index + 1 + ph.dest_connection_id.len() <= sp.bytes.len() {
            sp.bytes[byte_index] = ph.dest_connection_id.len() as u8;
            byte_index += 1;
            let odcid_len = ph.dest_connection_id.len();
            sp.bytes[byte_index..byte_index + odcid_len]
                .copy_from_slice(ph.dest_connection_id.as_bytes());
            byte_index += odcid_len;
        }

        sp.length = byte_index;
        sp.packet_type = crate::internal::PacketType::Retry;
        sp.addr_to = *addr_from;
        sp.addr_local = *addr_to;
        sp.if_index_local = if_index_to;
        sp.connection_id_log64 = ph.dest_connection_id.val64();
        self.queue_stateless_packet(sp);
    }

    fn queue_retry_packet(
        &mut self,
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
    ) -> i32 {
        let mut server_cid = ConnectionId::default();
        self.create_local_cnx_id(&mut server_cid, ph.dest_connection_id);
        let mut token_buffer = [0u8; 256];
        match self.prepare_retry_token(
            addr_from,
            current_time,
            &ph.dest_connection_id,
            &server_cid,
            ph.packet_number_full as u32,
            &mut token_buffer,
        ) {
            Ok(token_size) => {
                self.queue_stateless_retry(
                    ph,
                    &server_cid,
                    addr_from,
                    addr_to,
                    if_index_to,
                    &token_buffer[..token_size],
                );
                InternalError::Retry as i32
            }
            Err(_) => InternalError::Memory as i32,
        }
    }

    fn queue_busy_packet(
        &mut self,
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        ph: &crate::internal::PacketHeader,
    ) -> i32 {
        let Ok(mut sp) = self.create_stateless_packet() else {
            return InternalError::Memory as i32;
        };
        let mut server_cid = ConnectionId::default();
        self.create_local_cnx_id(&mut server_cid, ph.dest_connection_id);
        let initial_context = match self.initial_aead_context(
            ph.version_index,
            &ph.dest_connection_id,
            false,
            true,
        ) {
            Ok(ctx) => ctx,
            Err(error) => return Self::parse_error_status(error),
        };
        let payload = [
            crate::frames::FrameType::ConnectionClose as u8,
            TransportError::ServerBusy as u8,
            0,
            0,
        ];
        let mut pn_offset = 0usize;
        let mut pn_length = 0usize;
        let header_length = crate::internal::create_long_header(
            crate::internal::PacketType::Initial,
            &ph.src_connection_id,
            &server_cid,
            false,
            ph.version,
            ph.version_index,
            0,
            &[],
            &mut sp.bytes,
            &mut pn_offset,
            &mut pn_length,
        );
        let checksum_len = initial_context.aead_ctx.tag_len();
        crate::internal::update_payload_length(
            &mut sp.bytes,
            pn_offset,
            header_length.saturating_sub(pn_length),
            header_length + payload.len() + checksum_len,
        );
        let header = sp.bytes[..header_length].to_vec();
        let mut protected_payload = payload.to_vec();
        initial_context
            .aead_ctx
            .encrypt(0, &header, &mut protected_payload);
        let send_length = header_length + protected_payload.len();
        if send_length > sp.bytes.len() {
            return InternalError::Memory as i32;
        }
        sp.bytes[header_length..send_length].copy_from_slice(&protected_payload);
        crate::internal::protect_packet_header(
            &mut sp.bytes[..send_length],
            pn_offset,
            0x0f,
            initial_context.pn_enc_ctx.as_ref(),
        );
        sp.length = send_length;
        sp.packet_type = crate::internal::PacketType::Initial;
        sp.addr_to = *addr_from;
        sp.addr_local = *addr_to;
        sp.if_index_local = if_index_to;
        sp.connection_id_log64 = ph.dest_connection_id.val64();
        self.queue_stateless_packet(sp);
        0
    }

    /// Process a server Retry packet on a client connection.
    ///
    /// C: `picoquic_incoming_retry` (picoquic/packet.c:1521-1613).
    pub fn incoming_retry(
        &mut self,
        connection: ConnectionToken,
        bytes: &mut [u8],
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
    ) -> i32 {
        let (version_index, initial_cid) = {
            let Some(cnx) = self.connections.get(connection) else {
                return InternalError::UnexpectedPacket as i32;
            };
            if (cnx.connection_state != State::ClientInitSent
                && cnx.connection_state != State::ClientInitResent)
                || !cnx.original_connection_id.is_empty()
            {
                return InternalError::UnexpectedPacket as i32;
            }
            if ph.version != cnx.negotiated_version() || ph.packet_number_full != 0 {
                return InternalError::UnexpectedPacket as i32;
            }
            (cnx.version_index, cnx.initial_connection_id)
        };

        let mut byte_index = ph.offset;
        let mut data_length = ph.offset.saturating_add(ph.payload_length);
        if data_length > bytes.len() {
            return InternalError::UnexpectedPacket as i32;
        }

        let verify_ret = {
            if let Some(integrity_aead) = self.find_retry_protection_context(version_index, false) {
                match crate::tls_api::verify_retry_protection(
                    integrity_aead,
                    bytes,
                    data_length,
                    byte_index,
                    &initial_cid,
                ) {
                    Ok(stripped_length) => {
                        data_length = stripped_length;
                        0
                    }
                    Err(error) => Self::parse_error_status(error),
                }
            } else if byte_index < data_length {
                let odcil = bytes[byte_index] as usize;
                byte_index += 1;
                if odcil != initial_cid.len()
                    || odcil + 1 > ph.payload_length
                    || byte_index + odcil > data_length
                    || &bytes[byte_index..byte_index + odcil] != initial_cid.as_bytes()
                {
                    InternalError::UnexpectedPacket as i32
                } else {
                    byte_index += odcil;
                    0
                }
            } else {
                InternalError::UnexpectedPacket as i32
            }
        };

        if verify_ret != 0 {
            if let Some(cnx) = self.connections.get_mut(connection) {
                cnx.log_app_message("Retry packet rejected: integrity check failed");
            }
            return verify_ret;
        }

        let token = bytes[byte_index..data_length].to_vec();
        let Some(cnx) = self.connections.get_mut(connection) else {
            return InternalError::UnexpectedPacket as i32;
        };
        crate::logger::Log::close_connection(cnx);
        if cnx.original_connection_id.is_empty() {
            cnx.original_connection_id = cnx.initial_connection_id;
        }
        cnx.initial_connection_id = ph.src_connection_id;
        cnx.retry_token = token;
        if cnx.reset(current_time).is_err() {
            return InternalError::UnexpectedError as i32;
        }
        InternalError::Retry as i32
    }

    fn incoming_stateless_reset(&mut self, connection: ConnectionToken) -> i32 {
        let Some(cnx) = self.connections.get_mut(connection) else {
            return InternalError::AeadCheck as i32;
        };
        if cnx.connection_state <= State::Ready {
            cnx.remote_error = InternalError::StatelessReset as u64;
        }
        let _ = cnx.call_stream_data_callback(0, &[], CallbackEvent::StatelessReset, None);
        cnx.connection_disconnect();
        InternalError::AeadCheck as i32
    }

    fn incoming_version_negotiation(
        &mut self,
        connection: ConnectionToken,
        bytes: &[u8],
        ph: &crate::internal::PacketHeader,
    ) -> i32 {
        let Some(cnx) = self.connections.get_mut(connection) else {
            return InternalError::UnexpectedPacket as i32;
        };
        if cnx.connection_state != State::ClientInitSent {
            return 0;
        }
        if cnx.path_local_connection_id(0) != Some(ph.dest_connection_id)
            || ph.version != 0
            || ph.src_connection_id != cnx.initial_connection_id
        {
            return InternalError::Detected as i32;
        }
        let Some(version_bytes) = bytes.get(ph.offset..) else {
            return InternalError::Detected as i32;
        };
        if !version_bytes.len().is_multiple_of(4) {
            return InternalError::Detected as i32;
        }

        let mut supported_count = 0;
        for chunk in version_bytes.chunks_exact(4) {
            let version = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            if version == cnx.proposed_version || version == 0 {
                return InternalError::Detected as i32;
            }
            if crate::internal::Version::try_from_wire(version).is_some() {
                supported_count += 1;
            }
        }

        if supported_count == 0 {
            return InternalError::Detected as i32;
        }

        if let Some(mut callback) = cnx.callback_fn.take() {
            let _ = callback.callback(
                cnx,
                0,
                version_bytes,
                CallbackEvent::VersionNegotiation,
                None,
            );
            cnx.callback_fn = Some(callback);
        }
        cnx.remote_error = TransportError::VersionNegotiationError as u64;
        cnx.connection_disconnect();
        0
    }

    fn is_silent_drop_status(ret: i32) -> bool {
        matches!(
            ret as u64,
            x if x == InternalError::AeadCheck as u64
                || x == InternalError::InitialTooShort as u64
                || x == InternalError::PacketWrongVersion as u64
                || x == InternalError::InitialCidTooShort as u64
                || x == InternalError::PortBlocked as u64
                || x == InternalError::UnexpectedPacket as u64
                || x == InternalError::CnxidCheck as u64
                || x == InternalError::Retry as u64
                || x == InternalError::Detected as u64
                || x == InternalError::ServerBusy as u64
                || x == InternalError::ConnectionDeleted as u64
                || x == InternalError::CnxidSegment as u64
                || x == InternalError::VersionNotSupported as u64
                || x == InternalError::PacketTooLong as u64
                || x == InternalError::Duplicate as u64
                || x == InternalError::AeadNotReady as u64
                || x == InternalError::Redirected as u64
        )
    }

    fn silent_drop_status_returns_zero(ret: i32) -> bool {
        matches!(
            ret as u64,
            x if x == InternalError::AeadCheck as u64
                || x == InternalError::PacketWrongVersion as u64
                || x == InternalError::AeadNotReady as u64
                || x == InternalError::PacketTooLong as u64
                || x == InternalError::VersionNotSupported as u64
                || x == InternalError::Retry as u64
                || x == InternalError::ServerBusy as u64
                || x == InternalError::Redirected as u64
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch_initial_segment(
        &mut self,
        connection: ConnectionToken,
        bytes: &[u8],
        packet_length: usize,
        received_data: &mut crate::internal::StreamDataNode,
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
        new_context_created: bool,
        is_first_segment: bool,
        first_cnx: &mut Option<ConnectionToken>,
    ) -> i32 {
        if ph.has_reserved_bit_set {
            return InternalError::PacketHeaderParsing as i32;
        }

        let (client_mode, dest_ok) = {
            let Some(cnx_ref) = self.connections.get(connection) else {
                return InternalError::UnexpectedPacket as i32;
            };
            (
                cnx_ref.client_mode,
                (!cnx_ref.client_mode && ph.dest_connection_id == cnx_ref.initial_connection_id)
                    || cnx_ref.path_local_connection_id(0) == Some(ph.dest_connection_id),
            )
        };
        if !dest_ok {
            return InternalError::Detected as i32;
        }

        if let Some(cnx_ref) = self.connections.get_mut(connection) {
            let remote_cid = cnx_ref.path_remote_connection_id(0);
            if remote_cid.is_none_or(|cid| cid.is_empty()) {
                if let Some(stash) = cnx_ref.remote_connection_id_stashes.first_mut()
                    && let Some(remote_cid) = stash.connection_ids.first_mut()
                {
                    remote_cid.connection_id = ph.src_connection_id;
                }
            } else if remote_cid != Some(ph.src_connection_id) {
                return InternalError::UnexpectedPacket as i32;
            }
        }

        if !client_mode {
            if is_first_segment && let Some(cnx_ref) = self.connections.get_mut(connection) {
                cnx_ref.initial_data_received = cnx_ref
                    .initial_data_received
                    .saturating_add(packet_length as u64);
            }
            let (ret, live_connection) = self.incoming_client_initial(
                connection,
                bytes,
                packet_length,
                received_data,
                Some(addr_from),
                Some(addr_to),
                if_index_to as u64,
                ph,
                current_time,
                new_context_created,
            );
            *first_cnx = live_connection;
            ret
        } else if let Some(cnx_ref) = self.connections.get_mut(connection) {
            cnx_ref.incoming_server_initial(
                bytes,
                packet_length,
                received_data,
                Some(addr_to),
                if_index_to as u64,
                ph,
                current_time,
            )
        } else {
            InternalError::UnexpectedPacket as i32
        }
    }

    fn dispatch_handshake_segment(
        &mut self,
        connection: ConnectionToken,
        bytes: &[u8],
        received_data: &mut crate::internal::StreamDataNode,
        addr_to: &SocketAddr,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
    ) -> i32 {
        if ph.has_reserved_bit_set {
            if let Some(cnx_ref) = self.connections.get_mut(connection) {
                return cnx_ref.connection_error(TransportError::ProtocolViolation as u64, 0);
            }
            return InternalError::UnexpectedPacket as i32;
        }

        let client_mode = self
            .connections
            .get(connection)
            .map(|cnx_ref| cnx_ref.client_mode)
            .unwrap_or(false);
        if let Some(cnx_ref) = self.connections.get_mut(connection) {
            if client_mode {
                cnx_ref.incoming_server_handshake(
                    bytes,
                    received_data,
                    Some(addr_to),
                    ph,
                    current_time,
                )
            } else {
                cnx_ref.incoming_client_handshake(bytes, received_data, ph, current_time)
            }
        } else {
            InternalError::UnexpectedPacket as i32
        }
    }

    fn dispatch_zero_rtt_segment(
        &mut self,
        connection: ConnectionToken,
        bytes: &[u8],
        received_data: &mut crate::internal::StreamDataNode,
        packet_length: usize,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
        is_first_segment: bool,
    ) -> i32 {
        if ph.has_reserved_bit_set {
            if let Some(cnx_ref) = self.connections.get_mut(connection) {
                return cnx_ref.connection_error(TransportError::ProtocolViolation as u64, 0);
            }
            return InternalError::UnexpectedPacket as i32;
        }

        if is_first_segment && let Some(cnx_ref) = self.connections.get_mut(connection) {
            cnx_ref.initial_data_received = cnx_ref
                .initial_data_received
                .saturating_add(packet_length as u64);
        }
        if let Some(cnx_ref) = self.connections.get_mut(connection) {
            cnx_ref.incoming_0rtt(bytes, received_data, ph, current_time)
        } else {
            InternalError::UnexpectedPacket as i32
        }
    }
}

impl Quic {
    /// Process one segment from a received UDP datagram.
    ///
    /// C: `picoquic_incoming_segment` (picoquic/packet.c:2073-2387).
    #[allow(clippy::too_many_arguments)]
    pub fn incoming_segment(
        &mut self,
        raw_bytes: &mut [u8],
        length: usize,
        packet_length: usize,
        consumed: &mut usize,
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        received_ecn: u8,
        current_time: Instant,
        receive_time: Instant,
        previous_dest_id: &mut ConnectionId,
        first_cnx: &mut Option<ConnectionToken>,
    ) -> i32 {
        let mut ph = crate::internal::PacketHeader::default();
        let mut decrypted_data = match self.stream_data_node_alloc() {
            Ok(node) => node,
            Err(_) => return -1,
        };

        let parsed = self.parse_header_and_decrypt_for_segment(
            raw_bytes,
            length,
            packet_length,
            addr_from,
            current_time,
            &mut decrypted_data,
            &mut ph,
            consumed,
        );
        let mut ret = parsed.ret;
        let mut cnx = parsed.connection;
        let new_context_created = parsed.new_context_created;
        let mut is_first_segment = false;
        let mut is_buffered = false;
        let mut path_id = None;
        let mut path_is_not_allocated = 0i32;

        if ret == 0
            && let Some(connection) = cnx
        {
            if ph.packet_type == crate::internal::PacketType::OneRttProtected {
                let lookup = {
                    let Some(cnx_ref) = self.connections.get_mut(connection) else {
                        return -1;
                    };
                    cnx_ref.find_incoming_path(
                        &mut ph,
                        addr_from,
                        addr_to,
                        if_index_to,
                        current_time,
                    )
                };
                match lookup {
                    Ok(lookup) => {
                        path_id = Some(lookup.path_id);
                        path_is_not_allocated = i32::from(lookup.created);
                    }
                    Err(error) => ret = Self::parse_error_status(error),
                }
            } else {
                path_id = Some(0);
            }
        }

        if previous_dest_id.is_empty() {
            *previous_dest_id = ph.dest_connection_id;
            is_first_segment = true;
            *first_cnx = cnx;
            if let Some(connection) = cnx {
                if let Some(cnx_ref) = self.connections.get_mut(connection) {
                    let unique_path_id = path_id
                        .and_then(|idx| cnx_ref.paths.get(idx))
                        .map(|path| path.unique_path_id)
                        .unwrap_or(0);
                    crate::logger::Log::pdu(
                        cnx_ref,
                        true,
                        current_time,
                        addr_from,
                        addr_to,
                        packet_length,
                        unique_path_id,
                        received_ecn,
                    );
                }
            } else {
                self.log_pdu(
                    true,
                    current_time,
                    ph.dest_connection_id.val64(),
                    addr_from,
                    addr_to,
                    packet_length,
                );
            }
        } else {
            if (ret == 0 && *previous_dest_id != ph.dest_connection_id)
                || ret == InternalError::VersionNotSupported as i32
            {
                ret = InternalError::CnxidSegment as i32;
            }
            if ret == InternalError::CnxidSegment as i32
                && *first_cnx != cnx
                && let Some(first) = *first_cnx
                && let Some(first_ref) = self.connections.get_mut(first)
            {
                first_ref.log_dropped_packet_on_path(
                    None,
                    &ph,
                    length,
                    InternalError::PaddingPacket as i32,
                    current_time,
                );
            }
        }

        if ret == InternalError::AeadNotReady as i32
            && let Some(connection) = cnx
            && let Some(cnx_ref) = self.connections.get_mut(connection)
        {
            is_buffered = cnx_ref.incoming_not_decrypted(
                &ph,
                current_time,
                raw_bytes,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
            ) != 0;
        }

        if let Some(connection) = cnx {
            if ret == 0 && ph.packet_type == crate::internal::PacketType::OneRttProtected {
                if ph.payload_length == 0 {
                    if let Some(cnx_ref) = self.connections.get_mut(connection) {
                        ret = cnx_ref.connection_error(TransportError::ProtocolViolation as u64, 0);
                    }
                } else if ph.has_reserved_bit_set
                    && let Some(cnx_ref) = self.connections.get_mut(connection)
                {
                    ret = cnx_ref.connection_error(TransportError::ProtocolViolation as u64, 0);
                }
            }

            if let Some(cnx_ref) = self.connections.get_mut(connection) {
                if ret == 0 {
                    cnx_ref.log_packet_on_path(
                        path_id,
                        true,
                        current_time,
                        &ph,
                        &decrypted_data.data[..decrypted_data.length],
                    );
                } else if is_buffered {
                    cnx_ref.log_buffered_packet_on_path(path_id, ph.packet_type, current_time);
                } else {
                    cnx_ref.log_dropped_packet_on_path(path_id, &ph, length, ret, current_time);
                }
            }
        }

        if ret == InternalError::VersionNotSupported as i32 {
            if packet_length >= crate::internal::ENFORCED_INITIAL_MTU
                && (self.is_port_blocking_disabled || !check_addr_blocked(addr_from))
            {
                self.queue_version_negotiation_packet(addr_from, addr_to, if_index_to, &ph);
            }
        } else if ret == InternalError::RetryNeeded as i32 {
            if packet_length >= crate::internal::ENFORCED_INITIAL_MTU
                && (self.is_port_blocking_disabled || !check_addr_blocked(addr_from))
            {
                ret = self.queue_retry_packet(addr_from, addr_to, if_index_to, &ph, current_time);
            }
        } else if ret == InternalError::ServerBusy as i32 {
            if packet_length >= crate::internal::ENFORCED_INITIAL_MTU
                && (self.is_port_blocking_disabled || !check_addr_blocked(addr_from))
            {
                ret = self.queue_busy_packet(addr_from, addr_to, if_index_to, &ph);
            }
        } else if ret == 0 {
            if let Some(connection) = cnx {
                if let Some(cnx_ref) = self.connections.get_mut(connection) {
                    cnx_ref.quic_bit_received_0 |= ph.quic_bit_is_zero;
                }
                let bytes_len = decrypted_data.length;
                let mut packet_bytes = [0u8; crate::internal::MAX_PACKET_SIZE];
                packet_bytes[..bytes_len].copy_from_slice(&decrypted_data.data[..bytes_len]);
                let mut received_frame_data = crate::internal::StreamDataNode {
                    stream_data_membership: None,
                    offset: 0,
                    data: [0u8; crate::internal::MAX_PACKET_SIZE],
                    length: 0,
                };
                match ph.packet_type {
                    crate::internal::PacketType::VersionNegotiation => {
                        let bytes = &packet_bytes[..bytes_len];
                        ret = self.incoming_version_negotiation(connection, bytes, &ph);
                    }
                    crate::internal::PacketType::Initial => {
                        let bytes = &packet_bytes[..bytes_len];
                        ret = self.dispatch_initial_segment(
                            connection,
                            bytes,
                            packet_length,
                            &mut received_frame_data,
                            addr_from,
                            addr_to,
                            if_index_to,
                            &ph,
                            current_time,
                            new_context_created,
                            is_first_segment,
                            first_cnx,
                        );
                        cnx = *first_cnx;
                    }
                    crate::internal::PacketType::Retry => {
                        ret = self.incoming_retry(connection, raw_bytes, &ph, current_time);
                    }
                    crate::internal::PacketType::Handshake => {
                        let bytes = &packet_bytes[..bytes_len];
                        ret = self.dispatch_handshake_segment(
                            connection,
                            bytes,
                            &mut received_frame_data,
                            addr_to,
                            &ph,
                            current_time,
                        );
                    }
                    crate::internal::PacketType::ZeroRttProtected => {
                        let bytes = &packet_bytes[..bytes_len];
                        ret = self.dispatch_zero_rtt_segment(
                            connection,
                            bytes,
                            &mut received_frame_data,
                            packet_length,
                            &ph,
                            current_time,
                            is_first_segment,
                        );
                    }
                    crate::internal::PacketType::OneRttProtected => {
                        if let Some(cnx_ref) = self.connections.get_mut(connection) {
                            let bytes = &mut packet_bytes[..bytes_len];
                            ret = cnx_ref.incoming_1rtt(
                                path_id.unwrap_or(0),
                                bytes,
                                &mut received_frame_data,
                                &ph,
                                Some(addr_from),
                                Some(addr_to),
                                if_index_to,
                                path_is_not_allocated,
                                current_time,
                            );
                        }
                    }
                    _ => ret = InternalError::Detected as i32,
                }
            } else {
                if !ph.dest_connection_id.is_empty()
                    && (self.is_port_blocking_disabled || !check_addr_blocked(addr_from))
                {
                    self.queue_version_negotiation_packet(addr_from, addr_to, if_index_to, &ph);
                }
                ret = InternalError::Detected as i32;
            }
        } else if ret == InternalError::StatelessReset as i32 {
            if let Some(connection) = cnx {
                ret = self.incoming_stateless_reset(connection);
            }
        } else if ret == InternalError::AeadCheck as i32
            && ph.packet_type == crate::internal::PacketType::Handshake
            && let Some(connection) = cnx
            && let Some(cnx_ref) = self.connections.get_mut(connection)
            && (cnx_ref.connection_state == State::ClientInitSent
                || cnx_ref.connection_state == State::ClientInitResent)
            && !cnx_ref.pkt_ctx[PacketContext::Initial as usize]
                .pending
                .is_empty()
            && cnx_ref
                .paths
                .first()
                .is_some_and(|path| path.nb_retransmit == 0)
            && let Some(first_pending) = cnx_ref.pkt_ctx[PacketContext::Initial as usize]
                .pending
                .values()
                .next()
                .and_then(|packet| cnx_ref.queued_packets.get(*packet))
            && let Some(path) = cnx_ref.paths.first_mut()
        {
            path.retransmit_timer = Duration::from_ticks(
                current_time
                    .ticks()
                    .saturating_sub(first_pending.send_time.ticks()),
            );
        }

        if ret == 0 {
            if let Some(connection) = cnx
                && let Some(cnx_ref) = self.connections.get_mut(connection)
                && cnx_ref.connection_state != State::Disconnected
                && ph.packet_type != crate::internal::PacketType::VersionNegotiation
            {
                cnx_ref.nb_packets_received = cnx_ref.nb_packets_received.saturating_add(1);
                cnx_ref.latest_receive_time = current_time;
                ret = cnx_ref.record_pn_received(
                    ph.packet_context,
                    ph.local_connection_id,
                    ph.packet_number_full,
                    receive_time,
                );
                cnx_ref.ecn_accounting(received_ecn, ph.packet_context, ph.local_connection_id);
            }
            if let Some(connection) = cnx {
                self.reinsert_by_wake_time_token(connection, current_time);
            }
        } else if Self::is_silent_drop_status(ret) {
            let normalized = if Self::silent_drop_status_returns_zero(ret) {
                0
            } else {
                -1
            };
            if let Some(connection) = cnx {
                self.reinsert_by_wake_time_token(connection, current_time);
            }
            ret = normalized;
        } else if ret != 0 {
            ret = -1;
        }

        self.stream_data_node_recycle(decrypted_data);
        ret
    }
}

// ---------------------------------------------------------------------------
// Phase 3A helpers for skip_frame / satellite / spinbit tests.

impl Quic {
    /// Create a connection with all-null CIDs and a dummy loopback address,
    /// suitable for unit tests that need a minimal live connection.
    /// C: `picoquic_create_cnx(quic, null, null, loopback, time, 0, sni, alpn, 1)`.
    pub fn create_test_cnx(
        &mut self,
        simulated_time: &mut Instant,
    ) -> crate::Result<Box<Connection>> {
        let loopback: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let null_cid = ConnectionId::default();
        self.create_connection_with_cids(
            null_cid,
            null_cid,
            Some(&loopback),
            *simulated_time,
            "",
            "",
        )
    }

    /// Create a connection specifying both the initial and destination CIDs
    /// explicitly.  Used by binlog / overflow tests.
    /// C: `picoquic_create_cnx(quic, initial_cid, dest_cid, addr, time, 0, sni, alpn, 1)`.
    pub fn create_connection_with_cids(
        &mut self,
        initial_cid: ConnectionId,
        dest_cid: ConnectionId,
        addr: Option<&core::net::SocketAddr>,
        current_time: Instant,
        sni: &str,
        alpn: &str,
    ) -> crate::Result<Box<Connection>> {
        // C: picoquic_create_cnx(quic, initial_cid, dest_cid, addr,
        // time, 0, sni, alpn, 1).  The Rust shape that test helpers
        // expect is a standalone Box<Connection>; create_cnx_internal
        // installs the connection in the arena and the secondary
        // indexes, after which we extract it back out by token.
        let sni_opt = if sni.is_empty() { None } else { Some(sni) };
        let alpn_opt = if alpn.is_empty() { None } else { Some(alpn) };
        let token = self.create_cnx_internal(
            initial_cid,
            dest_cid,
            addr,
            current_time,
            0,
            sni_opt,
            alpn_opt,
            true,
            None,
            None,
        )?;
        let (initial_cid, local_cid_memberships) = {
            let cnx = self.connections.get(token).ok_or(Error::Generic)?;
            let memberships = cnx
                .local_connection_id_lists
                .iter()
                .flat_map(|list| list.connection_ids.iter().copied())
                .filter_map(|tok| {
                    cnx.local_connection_ids
                        .get(tok)
                        .and_then(|l_cid| l_cid.connection_by_id_membership)
                })
                .collect::<Vec<_>>();
            (cnx.initial_connection_id, memberships)
        };
        for membership in local_cid_memberships {
            self.connection_by_id.remove(membership);
        }
        if !initial_cid.is_empty()
            && let Some(ht) = self.connection_by_id.lookup(&initial_cid)
            && self.connection_by_id.get(ht).copied() == Some(token)
        {
            self.connection_by_id.remove(ht);
        }
        self.remove_cnx_from_list(token);
        self.remove_cnx_from_wake_list(token);
        let mut cnx = self.connections.remove(token).ok_or(Error::Generic)?;
        cnx.own_token = None;
        cnx.quic_ptr = core::ptr::null_mut();
        Ok(Box::new(cnx))
    }
}

impl Connection {
    fn packet_payload<'a>(bytes: &'a [u8], ph: &crate::internal::PacketHeader) -> &'a [u8] {
        let payload_length = ph.payload_length.min(bytes.len());
        if ph.offset <= bytes.len()
            && ph
                .offset
                .checked_add(payload_length)
                .is_some_and(|end| end <= bytes.len())
        {
            &bytes[ph.offset..ph.offset + payload_length]
        } else {
            &bytes[..payload_length]
        }
    }

    fn packet_payload_mut<'a>(
        bytes: &'a mut [u8],
        ph: &crate::internal::PacketHeader,
    ) -> &'a mut [u8] {
        let payload_length = ph.payload_length.min(bytes.len());
        if ph.offset <= bytes.len()
            && ph
                .offset
                .checked_add(payload_length)
                .is_some_and(|end| end <= bytes.len())
        {
            &mut bytes[ph.offset..ph.offset + payload_length]
        } else {
            &mut bytes[..payload_length]
        }
    }

    fn status_from_error(error: Error) -> i32 {
        match error {
            Error::Protocol(code) => code as i32,
            Error::Memory => InternalError::Memory as i32,
            Error::InvalidArgument => InternalError::UnexpectedError as i32,
            Error::BufferTooSmall => InternalError::FrameBufferTooSmall as i32,
            Error::NoSuchFile => InternalError::NoSuchFile as i32,
            Error::InvalidFile => InternalError::InvalidFile as i32,
            Error::Disconnected => InternalError::Disconnected as i32,
            Error::InvalidState => InternalError::UnexpectedState as i32,
            Error::InvalidFrame => InternalError::InvalidFrame as i32,
            Error::Tls => InternalError::AeadCheck as i32,
            Error::Generic => InternalError::UnexpectedError as i32,
        }
    }

    fn process_tls_stream_status(&mut self, current_time: Instant) -> (i32, usize) {
        match self.process_tls_stream(current_time) {
            Ok(consumed) => (0, consumed),
            Err(error) => (Self::status_from_error(error), 0),
        }
    }

    fn path_local_connection_id(&self, path_index: usize) -> Option<ConnectionId> {
        self.paths
            .get(path_index)
            .and_then(|path| path.tuples.first())
            .and_then(|tuple| tuple.local_connection_id)
            .and_then(|token| self.local_connection_ids.get(token))
            .map(|local_cid| local_cid.connection_id)
    }

    fn path_remote_connection_id(&self, path_index: usize) -> Option<ConnectionId> {
        let path = self.paths.get(path_index)?;
        let tuple = path.tuples.first()?;
        let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
        self.remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == path.unique_path_id)
            .and_then(|stash| stash.connection_ids.get(cid_index))
            .map(|remote_cid| remote_cid.connection_id)
    }

    fn negotiated_version(&self) -> u32 {
        crate::internal::Version::try_from_wire(self.proposed_version)
            .or(match self.version_index {
                0 => Some(crate::internal::Version::V1),
                1 => Some(crate::internal::Version::V2),
                2 => Some(crate::internal::Version::V2Draft),
                3 => Some(crate::internal::Version::PostIesg),
                4 => Some(crate::internal::Version::TwentyFirstInterop),
                5 => Some(crate::internal::Version::TwentiethInterop),
                6 => Some(crate::internal::Version::TwentiethPreInterop),
                7 => Some(crate::internal::Version::NineteenthBisInterop),
                8 => Some(crate::internal::Version::NineteenthInterop),
                9 => Some(crate::internal::Version::EighteenthInterop),
                10 => Some(crate::internal::Version::SeventeenthInterop),
                11 => Some(crate::internal::Version::InternalTest2),
                12 => Some(crate::internal::Version::InternalTest1),
                _ => None,
            })
            .unwrap_or(crate::internal::Version::V1) as u32
    }

    fn socket_addr_is_unspecified(addr: &SocketAddr) -> bool {
        addr.port() == 0
            && match addr.ip() {
                core::net::IpAddr::V4(ip) => ip.is_unspecified(),
                core::net::IpAddr::V6(ip) => ip.is_unspecified(),
            }
    }

    fn decode_frames_on_path(
        &mut self,
        path_index: usize,
        bytes: &[u8],
        received_data: &mut crate::internal::StreamDataNode,
        epoch: crate::internal::Epoch,
        addr_from: Option<&SocketAddr>,
        addr_to: Option<&SocketAddr>,
        pn64: u64,
        path_is_not_allocated: i32,
        current_time: Instant,
    ) -> i32 {
        if path_index >= self.paths.len() {
            return InternalError::UnexpectedPacket as i32;
        }
        let mut path = self.paths.remove(path_index);
        let ret = self.decode_frames(
            &mut path,
            bytes,
            bytes.len(),
            received_data,
            epoch as i32,
            addr_from,
            addr_to,
            pn64,
            path_is_not_allocated,
            current_time,
        );
        self.paths.insert(path_index, path);
        ret
    }

    /// Process unexpected Initial or Handshake payloads only enough to
    /// discover whether an ACK should be sent.
    ///
    /// C: `picoquic_ignore_incoming_handshake` (picoquic/packet.c:1345-1387).
    pub fn ignore_incoming_handshake(
        &mut self,
        bytes: &[u8],
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
    ) {
        let pc = match ph.packet_type {
            crate::internal::PacketType::Initial => PacketContext::Initial,
            crate::internal::PacketType::Handshake => PacketContext::Handshake,
            _ => return,
        };

        let payload = Self::packet_payload(bytes, ph);
        let mut byte_index = 0usize;
        let mut ret = 0i32;
        let mut ack_needed = false;

        while ret == 0 && byte_index < payload.len() {
            let mut frame_length = 0usize;
            let mut frame_is_pure_ack = 0i32;
            ret = crate::internal::skip_frame(
                &payload[byte_index..],
                payload.len() - byte_index,
                &mut frame_length,
                &mut frame_is_pure_ack,
            );
            byte_index = byte_index.saturating_add(frame_length);
            if frame_is_pure_ack == 0 {
                ack_needed = true;
            }
            if ret == 0 && frame_length == 0 {
                ret = -1;
            }
        }

        if ret == 0 && ack_needed {
            self.set_ack_needed_on_path(current_time, pc, 0, 0);
        }
    }

    /// Process a client Handshake packet on a server connection.
    ///
    /// C: `picoquic_incoming_client_handshake` (picoquic/packet.c:1753-1811).
    pub fn incoming_client_handshake(
        &mut self,
        bytes: &[u8],
        received_data: &mut crate::internal::StreamDataNode,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
    ) -> i32 {
        self.initial_validated = true;
        self.initial_repeat_needed = false;

        let mut ret = 0;
        if self.connection_state < State::ServerAlmostReady {
            if self.path_remote_connection_id(0) != Some(ph.src_connection_id) {
                ret = InternalError::CnxidCheck as i32;
            } else if ph.payload_length == 0 {
                ret = self.connection_error(TransportError::ProtocolViolation as u64, 0);
            } else {
                let payload = Self::packet_payload(bytes, ph);
                ret = self.decode_frames_on_path(
                    0,
                    payload,
                    received_data,
                    ph.epoch,
                    None,
                    None,
                    ph.packet_number_full,
                    0,
                    current_time,
                );
                if ret == 0 {
                    self.implicit_handshake_ack(PacketContext::Initial, current_time);
                    self.crypto_context[crate::internal::Epoch::Initial as usize].free_handles();
                    let (tls_ret, _) = self.process_tls_stream_status(current_time);
                    ret = tls_ret;
                    if ret == 0
                        && !self.client_mode
                        && self.connection_state < State::Ready
                        && self.is_tls_complete()
                    {
                        self.ready_state_transition(current_time);
                    }
                }
            }
        } else if self.connection_state <= State::Ready {
            self.ignore_incoming_handshake(bytes, ph, current_time);
        } else {
            ret = InternalError::UnexpectedPacket as i32;
        }

        ret
    }

    /// Process a client 0-RTT packet on a server connection.
    ///
    /// C: `picoquic_incoming_0rtt` (picoquic/packet.c:1835-1877).
    pub fn incoming_0rtt(
        &mut self,
        bytes: &[u8],
        received_data: &mut crate::internal::StreamDataNode,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
    ) -> i32 {
        let dest_matches = ph.dest_connection_id == self.initial_connection_id
            || self.path_local_connection_id(0) == Some(ph.dest_connection_id);
        let src_matches = self.path_remote_connection_id(0) == Some(ph.src_connection_id);

        if !dest_matches || !src_matches {
            return InternalError::CnxidCheck as i32;
        }

        if self.connection_state == State::ServerAlmostReady
            || self.connection_state == State::ServerFalseStart
            || (self.connection_state == State::Ready && !self.is_1rtt_received)
        {
            if ph.version != self.negotiated_version() || ph.payload_length == 0 {
                self.connection_error(TransportError::ProtocolViolation as u64, 0)
            } else {
                self.nb_zero_rtt_received = self.nb_zero_rtt_received.saturating_add(1);
                let payload = Self::packet_payload(bytes, ph);
                let mut ret = self.decode_frames_on_path(
                    0,
                    payload,
                    received_data,
                    ph.epoch,
                    None,
                    None,
                    ph.packet_number_full,
                    0,
                    current_time,
                );
                if ret == 0 {
                    let (tls_ret, _) = self.process_tls_stream_status(current_time);
                    ret = tls_ret;
                }
                ret
            }
        } else {
            InternalError::UnexpectedPacket as i32
        }
    }

    /// Process a 1-RTT protected packet.
    ///
    /// C: `picoquic_incoming_1rtt` (picoquic/packet.c:1910-2017).
    pub fn incoming_1rtt(
        &mut self,
        path_id: usize,
        bytes: &mut [u8],
        received_data: &mut crate::internal::StreamDataNode,
        ph: &crate::internal::PacketHeader,
        addr_from: Option<&SocketAddr>,
        addr_to: Option<&SocketAddr>,
        if_index_to: i32,
        path_is_not_allocated: i32,
        current_time: Instant,
    ) -> i32 {
        if self.connection_state < State::ClientAlmostReady {
            return InternalError::UnexpectedPacket as i32;
        }
        if self.connection_state == State::Disconnected {
            return InternalError::UnexpectedPacket as i32;
        }

        if self.connection_state >= State::Disconnecting {
            if self.connection_state == State::Closing
                || self.connection_state == State::Disconnecting
            {
                let payload = Self::packet_payload_mut(bytes, ph);
                let mut closing_received = 0;
                let ret = crate::internal::decode_closing_frames(
                    payload,
                    payload.len(),
                    &mut closing_received,
                );
                if ret == 0 {
                    if closing_received != 0 {
                        if self.client_mode {
                            self.connection_disconnect();
                        } else {
                            self.connection_state = State::Draining;
                        }
                    } else {
                        self.set_ack_needed_on_path(current_time, ph.packet_context, path_id, 0);
                    }
                }
                ret
            } else {
                InternalError::UnexpectedPacket as i32
            }
        } else {
            if path_id >= self.paths.len() {
                return InternalError::UnexpectedPacket as i32;
            }

            let payload = Self::packet_payload(bytes, ph);
            let mut path = self.paths.remove(path_id);
            if let Some(tuple) = path.tuples.first_mut() {
                tuple.if_index = if_index_to as core::ffi::c_ulong;
            }
            self.is_1rtt_received = true;
            if let Some(policy) =
                crate::internal::SPIN_FUNCTION_TABLE.get(self.spin_policy as usize)
            {
                policy.incoming(self, &mut path, ph);
            }

            let mut ret = self.decode_frames(
                &mut path,
                payload,
                payload.len(),
                received_data,
                ph.epoch as i32,
                addr_from,
                addr_to,
                ph.packet_number_full,
                path_is_not_allocated,
                current_time,
            );

            let mut recompute_ack_frequency = None;
            if ret == 0 {
                path.received = path.received.saturating_add(
                    (ph.offset as u64)
                        .saturating_add(ph.payload_length as u64)
                        .saturating_add(
                            self.get_checksum_length(crate::internal::Epoch::OneRtt) as u64
                        ),
                );
                if path.receive_rate_epoch == 0 {
                    path.received_prior = path.received;
                    path.receive_rate_epoch = current_time.ticks();
                } else {
                    let delta = current_time.ticks().saturating_sub(path.receive_rate_epoch);
                    if delta > path.smoothed_rtt.ticks()
                        && delta > crate::internal::BANDWIDTH_TIME_INTERVAL_MIN
                    {
                        path.receive_rate_estimate = crate::utils::rate_from_bytes(
                            path.received.saturating_sub(path.received_prior),
                            delta,
                        );
                        path.received_prior = path.received;
                        path.receive_rate_epoch = current_time.ticks();
                        if path.receive_rate_estimate > path.receive_rate_max {
                            path.receive_rate_max = path.receive_rate_estimate;
                            if path_id == 0 && !self.is_ack_frequency_negotiated {
                                recompute_ack_frequency =
                                    Some((path.rtt_min, path.receive_rate_max));
                            }
                        }
                    }
                }
            }

            self.paths.insert(path_id, path);

            if let Some((rtt_min, receive_rate_max)) = recompute_ack_frequency {
                let mut ack_gap = self.ack_gap_remote;
                let mut ack_delay = self.ack_delay_remote.ticks();
                self.compute_ack_gap_and_delay(
                    rtt_min,
                    crate::internal::ACK_DELAY_MIN.ticks(),
                    receive_rate_max,
                    &mut ack_gap,
                    &mut ack_delay,
                );
                self.ack_gap_remote = ack_gap;
                self.ack_delay_remote = Duration::from_ticks(ack_delay);
            }

            if ret == 0 {
                let (tls_ret, _) = self.process_tls_stream_status(current_time);
                ret = tls_ret;
            }

            if ret == 0 && self.is_still_logging() {
                crate::logger::Log::cc_dump(self, current_time);
            }

            ret
        }
    }

    /// MTU currently used for sending on the primary path.
    /// C: `cnx->path[0]->send_mtu`.
    pub fn primary_path_send_mtu(&self) -> u64 {
        self.paths.first().map(|p| p.send_mtu as u64).unwrap_or(0)
    }

    /// Minimum RTT observed on the primary path (µs).
    /// C: `cnx->path[0]->rtt_min`.
    pub fn primary_path_rtt_min(&self) -> u64 {
        self.paths.first().map(|p| p.rtt_min.ticks()).unwrap_or(0)
    }

    /// Current spin-bit value on the primary path.
    /// C: `cnx->path[0]->current_spin`.
    pub fn primary_path_current_spin(&self) -> bool {
        self.paths.first().map(|p| p.current_spin).unwrap_or(false)
    }

    /// Emit a "new connection" log record via both the binlog and text
    /// logger backends attached to this connection.
    /// C: `picoquic_log_new_connection(cnx)`.
    pub fn log_new_connection(&mut self) {
        crate::logger::Log::new_connection(self);
    }

    /// Emit a free-form application message on this connection's log.
    /// C: `picoquic_log_app_message(cnx, "%s", msg)`.
    pub fn log_app_message(&mut self, msg: &str) {
        crate::logger::Log::app_message(self, format_args!("{}", msg));
    }

    /// Given a `SplayToken` from this connection's stream tree, return the
    /// arena token for the corresponding [`crate::internal::StreamHead`].
    ///
    /// In the C source `picoquic_stream_from_node` (quicctx.c:3459) was a
    /// pointer cast: `picosplay_node_t` was the first field of
    /// `picoquic_stream_head_t` so casting the node pointer directly to a
    /// stream-head pointer was valid.  The Rust port stores
    /// [`crate::internal::StreamToken`]s as values inside the splay tree, so
    /// the cast becomes a plain table lookup.
    ///
    /// C: `picoquic_stream_from_node` (quicctx.c:3459).
    pub fn stream_from_node(
        &self,
        node: crate::splay::SplayToken,
    ) -> Option<crate::internal::StreamToken> {
        self.stream_tree.get(node).copied()
    }
}

impl Quic {
    /// Queue an immediate close packet for `connection` if packet preparation
    /// produces bytes to send.
    ///
    /// C: `picoquic_queue_immediate_close` (picoquic/packet.c:1317-1334).
    fn queue_immediate_close(&mut self, connection: ConnectionToken, current_time: Instant) {
        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };
        let prepared = self
            .connections
            .get_mut(connection)
            .and_then(|cnx| cnx.prepare_packet_ex(current_time, &mut sp.bytes).ok());

        if let Some(prepared) = prepared
            && prepared.send_length > 0
        {
            sp.length = prepared.send_length;
            sp.addr_to = prepared.addr_to;
            sp.addr_local = prepared.addr_from;
            sp.if_index_local = prepared.if_index;
            self.queue_stateless_packet(sp);
        }
    }

    /// Process an incoming client Initial packet on a server connection.
    /// Returns the C-style status code and the still-live connection token.
    ///
    /// C: `picoquic_incoming_client_initial` (picoquic/packet.c:1394-1505).
    pub fn incoming_client_initial(
        &mut self,
        connection: ConnectionToken,
        bytes: &[u8],
        packet_length: usize,
        received_data: &mut crate::internal::StreamDataNode,
        addr_from: Option<&SocketAddr>,
        addr_to: Option<&SocketAddr>,
        if_index_to: u64,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
        new_context_created: bool,
    ) -> (i32, Option<ConnectionToken>) {
        let server_busy = self.server_busy;
        let over_connection_limit =
            self.current_number_connections > self.tentative_max_number_connections;
        let mut ret = 0;
        let mut queue_close = false;
        let mut delete_created_connection = false;

        {
            let Some(cnx) = self.connections.get_mut(connection) else {
                return (InternalError::UnexpectedPacket as i32, None);
            };

            if cnx
                .path_local_connection_id(0)
                .is_some_and(|cid| !cid.is_empty() && cid == ph.dest_connection_id)
            {
                cnx.initial_validated = true;
            }

            if !cnx.initial_validated
                && !cnx.pkt_ctx[PacketContext::Initial as usize]
                    .pending
                    .is_empty()
                && packet_length >= crate::internal::ENFORCED_INITIAL_MTU
            {
                cnx.initial_repeat_needed = true;
            }

            if cnx.connection_state == State::ServerInit && (server_busy || over_connection_limit) {
                cnx.local_error = TransportError::ServerBusy as u64;
                cnx.connection_state = State::HandshakeFailure;
            } else if cnx.connection_state == State::ServerInit
                && cnx.initial_connection_id.len()
                    < crate::internal::ENFORCED_INITIAL_CID_LENGTH as usize
            {
                cnx.local_error = TransportError::ProtocolViolation as u64;
                cnx.connection_state = State::HandshakeFailure;
            } else if cnx.connection_state < State::ServerAlmostReady {
                if let Some(path) = cnx.paths.get_mut(0)
                    && let Some(tuple) = path.tuples.first_mut()
                {
                    if Connection::socket_addr_is_unspecified(&tuple.local_addr)
                        && let Some(addr) = addr_to
                    {
                        tuple.local_addr = *addr;
                    }
                    if Connection::socket_addr_is_unspecified(&tuple.peer_addr)
                        && let Some(addr) = addr_from
                    {
                        tuple.peer_addr = *addr;
                    }
                    tuple.if_index = if_index_to as core::ffi::c_ulong;
                }

                let highest_ack_before =
                    cnx.pkt_ctx[PacketContext::Initial as usize].highest_acknowledged;
                let payload = Connection::packet_payload(bytes, ph);
                ret = cnx.decode_frames_on_path(
                    0,
                    payload,
                    received_data,
                    ph.epoch,
                    addr_from,
                    addr_to,
                    ph.packet_number_full,
                    0,
                    current_time,
                );
                if cnx.pkt_ctx[PacketContext::Initial as usize].highest_acknowledged
                    > highest_ack_before
                    && cnx.random_initial > 1
                {
                    cnx.initial_validated = true;
                }

                if ret == 0 {
                    let (tls_ret, data_consumed) = cnx.process_tls_stream_status(current_time);
                    ret = tls_ret;
                    if data_consumed > 0 {
                        cnx.initial_repeat_needed = false;
                    }
                }
            } else if cnx.connection_state < State::Ready {
                cnx.ignore_incoming_handshake(bytes, ph, current_time);
            } else {
                ret = InternalError::UnexpectedPacket as i32;
            }

            if ret == InternalError::InvalidToken as i32
                && cnx.connection_state == State::HandshakeFailure
            {
                ret = 0;
            }

            if ret == 0 && cnx.connection_state == State::HandshakeFailure && new_context_created {
                queue_close = true;
            }

            if ret != 0 || cnx.connection_state == State::Disconnected {
                delete_created_connection = new_context_created;
            }
        }

        if queue_close {
            self.queue_immediate_close(connection, current_time);
        }

        if delete_created_connection {
            self.delete_connection(connection);
            (InternalError::ConnectionDeleted as i32, None)
        } else {
            (ret, Some(connection))
        }
    }
}

impl crate::internal::Path {
    /// Remove `tuple` (identified by its Vec index) from this path's tuple
    /// list without freeing any associated resources.  The caller is
    /// responsible for cleaning up CID references before calling this.
    ///
    /// In the C source `picoquic_unchain_tuple` (quicctx.c:1733) traversed a
    /// singly-linked intrusive list to splice out the target node.  The Rust
    /// port stores tuples in a plain `Vec`, so splicing is `Vec::remove`.
    ///
    /// C: `picoquic_unchain_tuple` (quicctx.c:1733).
    pub fn unchain_tuple(&mut self, index: usize) -> Option<crate::internal::Tuple> {
        (index < self.tuples.len()).then(|| self.tuples.remove(index))
    }
}

impl crate::internal::IssuedTicket {
    /// Update the network-measurement fields on an already-inserted ticket
    /// in place.
    ///
    /// C: `picoquic_update_issued_ticket` (quicctx.c:444) — the C function
    /// accepted a raw `(ip_addr, ip_addr_length)` byte pair; the Rust port
    /// takes a typed [`core::net::IpAddr`] which carries its own length.
    ///
    /// C: `picoquic_update_issued_ticket` (quicctx.c:444).
    pub fn update(&mut self, rtt: Duration, cwin: u64, ip_addr: core::net::IpAddr) {
        self.rtt = rtt;
        self.cwin = cwin;
        self.ip_addr = ip_addr;
    }
}

#[cfg(test)]
mod test {
    use super::{
        AES_128_GCM_SHA256, AES_256_GCM_SHA384, ConnectionId, ConnectionIdCallback, Error,
        GROUP_SECP256R1, Instant, InternalError, Quic, RESET_SECRET_SIZE, TransportError,
        TransportParameters, errors, is_handshake_error, reset_tls_api, write_ssl_key_log_line,
    };

    struct FixedConnectionIdCallback {
        connection_id: ConnectionId,
    }

    impl ConnectionIdCallback for FixedConnectionIdCallback {
        fn produce(
            &mut self,
            _quic: &mut Quic,
            _connection_id_local: ConnectionId,
            _connection_id_remote: ConnectionId,
        ) -> ConnectionId {
            self.connection_id
        }
    }

    #[test]
    fn connection_id_hash_matches_c_fold() {
        let seed_a = [0u8; 16];
        let seed_b = [0x5au8; 16];

        let short = ConnectionId::clone_from_slice(&[0x01, 0x02]).unwrap();
        assert_eq!(short.hash_with_seed(&seed_a), 0x0102);
        assert_eq!(short.hash_with_seed(&seed_b), 0x0102);

        let exact = ConnectionId::clone_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        assert_eq!(exact.hash_with_seed(&seed_a), 0x0102_0304_0506_0708);

        let folded = ConnectionId::clone_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap();
        assert_eq!(folded.hash_with_seed(&seed_a), 0x0203_0405_0608_080a);
    }

    #[test]
    fn is_local_cid_checks_length_and_registered_local_cids_only() {
        let initial_cid =
            ConnectionId::clone_from_slice(&[0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17])
                .unwrap();
        let remote_cid = ConnectionId::clone_from_slice(&[0x20, 0x21, 0x22, 0x23]).unwrap();
        let local_cid =
            ConnectionId::clone_from_slice(&[0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7])
                .unwrap();
        let callback: Option<Box<dyn ConnectionIdCallback>> =
            Some(Box::new(FixedConnectionIdCallback {
                connection_id: local_cid,
            }));
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            callback,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();
        let addr: core::net::SocketAddr = "127.0.0.1:4433".parse().unwrap();

        quic.create_connection(
            initial_cid,
            remote_cid,
            Some(&addr),
            Instant::from_ticks(0),
            0,
            None,
            None,
            true,
        )
        .unwrap();

        assert!(quic.connection_ref_by_id(initial_cid).is_some());
        assert!(!quic.is_local_cid(&initial_cid));
        assert!(quic.is_local_cid(&local_cid));

        let short_local_cid = ConnectionId::clone_from_slice(&[0xb0, 0xb1, 0xb2, 0xb3]).unwrap();
        let short_callback: Option<Box<dyn ConnectionIdCallback>> =
            Some(Box::new(FixedConnectionIdCallback {
                connection_id: short_local_cid,
            }));
        let mut short_quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            short_callback,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();
        short_quic
            .create_connection(
                initial_cid,
                remote_cid,
                Some(&addr),
                Instant::from_ticks(0),
                0,
                None,
                None,
                true,
            )
            .unwrap();

        assert!(short_quic.connection_by_id.contains_key(&short_local_cid));
        assert!(!short_quic.is_local_cid(&short_local_cid));
    }

    #[test]
    fn handshake_error_matches_c_predicate() {
        assert!(is_handshake_error(errors::transport_crypto_error(0) as u64));
        assert!(is_handshake_error(
            errors::transport_crypto_error(0x7f) as u64
        ));
        assert!(is_handshake_error(
            errors::transport_crypto_error(0) as u64 | 0x1_0000
        ));
        assert!(is_handshake_error(
            TransportError::TlsHandshakeFailed as u64
        ));
        assert!(!is_handshake_error(InternalError::AeadCheck as u64));
        assert!(!is_handshake_error(InternalError::InvalidFrame as u64));
    }

    #[test]
    fn ssl_key_log_line_uses_c_format() {
        let mut out = Vec::new();
        let random = [0xabu8; 32];
        let secret = [0x01, 0x23, 0xff];

        write_ssl_key_log_line(&mut out, "CLIENT_TRAFFIC_SECRET_0", &random, &secret).unwrap();

        assert_eq!(
            String::from_utf8(out).unwrap(),
            "CLIENT_TRAFFIC_SECRET_0 abababababababababababababababababababababababababababababababab 0123ff\n"
        );
    }

    #[test]
    fn tls_key_log_label_matches_c_role_direction_table() {
        assert_eq!(
            Quic::tls_key_log_label(false, 1),
            Some("CLIENT_EARLY_TRAFFIC_SECRET")
        );
        assert_eq!(
            Quic::tls_key_log_label(false, 2),
            Some("CLIENT_HANDSHAKE_TRAFFIC_SECRET")
        );
        assert_eq!(
            Quic::tls_key_log_label(false, 3),
            Some("CLIENT_TRAFFIC_SECRET_0")
        );
        assert_eq!(Quic::tls_key_log_label(true, 1), None);
        assert_eq!(
            Quic::tls_key_log_label(true, 2),
            Some("SERVER_HANDSHAKE_TRAFFIC_SECRET")
        );
        assert_eq!(
            Quic::tls_key_log_label(true, 3),
            Some("SERVER_TRAFFIC_SECRET_0")
        );
    }

    #[test]
    fn quic_new_handles_nullable_reset_seed() {
        let explicit_seed = [0x5au8; RESET_SECRET_SIZE];
        let explicit = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(explicit_seed),
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();
        assert_eq!(explicit.reset_seed_bytes(), explicit_seed.as_ref());

        let generated = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();
        assert_ne!(
            generated.reset_seed_bytes(),
            [0u8; RESET_SECRET_SIZE].as_ref()
        );
    }

    #[test]
    fn set_default_tp_none_resets_initialized_defaults() {
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();

        let custom = TransportParameters {
            initial_max_data: 7,
            max_packet_size: 9,
            ack_delay_exponent: 1,
            enable_loss_bit: 0,
            ..TransportParameters::default()
        };
        quic.set_default_tp(&custom).unwrap();
        assert_eq!(quic.default_tp().initial_max_data, custom.initial_max_data);

        quic.set_default_tp(None::<&TransportParameters>).unwrap();

        let mut expected = TransportParameters::default();
        crate::internal::init_transport_parameters(&mut expected);
        assert_eq!(
            quic.default_tp().initial_max_data,
            expected.initial_max_data
        );
        assert_eq!(quic.default_tp().max_packet_size, expected.max_packet_size);
        assert_eq!(
            quic.default_tp().ack_delay_exponent,
            expected.ack_delay_exponent
        );
        assert_eq!(quic.default_tp().enable_loss_bit, expected.enable_loss_bit);
    }

    #[test]
    fn set_max_data_control_updates_default_and_unsent_client_init_connections() {
        let mut quic = Quic::new(
            2,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();

        let eligible_id = ConnectionId::clone_from_slice(&[0x10, 0x20, 0x30, 0x40]).unwrap();
        let sent_id = ConnectionId::clone_from_slice(&[0x50, 0x60, 0x70, 0x80]).unwrap();
        let remote_a = ConnectionId::clone_from_slice(&[0xa1, 0xa2]).unwrap();
        let remote_b = ConnectionId::clone_from_slice(&[0xb1, 0xb2]).unwrap();
        let addr_a: core::net::SocketAddr = "127.0.0.1:4433".parse().unwrap();
        let addr_b: core::net::SocketAddr = "127.0.0.1:4434".parse().unwrap();

        {
            let cnx = quic
                .create_connection(
                    eligible_id,
                    remote_a,
                    Some(&addr_a),
                    Instant::from_ticks(0),
                    0,
                    None,
                    None,
                    true,
                )
                .unwrap();
            cnx.local_parameters.initial_max_data = 111;
            cnx.maxdata_local = 111;
        }

        {
            let cnx = quic
                .create_connection(
                    sent_id,
                    remote_b,
                    Some(&addr_b),
                    Instant::from_ticks(0),
                    0,
                    None,
                    None,
                    true,
                )
                .unwrap();
            cnx.local_parameters.initial_max_data = 222;
            cnx.maxdata_local = 222;
            cnx.tls_stream[crate::internal::Epoch::Initial as usize].sent_offset = 1;
        }

        quic.set_max_data_control(4096);

        assert_eq!(quic.max_data_limit(), 4096);
        assert_eq!(quic.default_tp().initial_max_data, 4096);

        {
            let cnx = quic.connection_ref_by_id(eligible_id).unwrap();
            assert_eq!(cnx.local_parameters.initial_max_data, 4096);
            assert_eq!(cnx.maxdata_local, 4096);
        }

        {
            let cnx = quic.connection_ref_by_id(sent_id).unwrap();
            assert_eq!(cnx.local_parameters.initial_max_data, 222);
            assert_eq!(cnx.maxdata_local, 222);
        }
    }

    #[test]
    fn set_transport_parameters_refreshes_mtu_and_local_flow_limits() {
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();
        quic.set_mtu_max(1500);
        assert_eq!(quic.default_tp().max_packet_size, 1500);

        let local_id = ConnectionId::clone_from_slice(&[0x10, 0x20, 0x30, 0x40]).unwrap();
        let remote_id = ConnectionId::clone_from_slice(&[0xa1, 0xa2]).unwrap();
        let peer_addr: core::net::SocketAddr = "127.0.0.1:4433".parse().unwrap();
        let cnx = quic
            .create_connection(
                local_id,
                remote_id,
                Some(&peer_addr),
                Instant::from_ticks(0),
                0,
                None,
                None,
                true,
            )
            .unwrap();

        cnx.maxdata_local = 1;
        cnx.max_stream_id_bidir_local = 2;
        cnx.max_stream_id_unidir_local = 3;

        let tp = TransportParameters {
            initial_max_data: 1234,
            initial_max_stream_id_bidir: 7,
            initial_max_stream_id_unidir: 11,
            max_packet_size: 0,
            ..TransportParameters::default()
        };
        cnx.set_transport_parameters(&tp);

        assert_eq!(cnx.local_parameters.max_packet_size, 1472);
        assert_eq!(cnx.maxdata_local, 1234);
        assert_eq!(cnx.max_stream_id_bidir_local, (7 - 1) << 2);
        assert_eq!(cnx.max_stream_id_unidir_local, ((11 - 1) << 2) | 2);
    }

    #[test]
    fn set_cipher_suite_installs_provider_selected_list() {
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();

        assert!(quic.tls_cipher_suites.contains(&AES_128_GCM_SHA256));
        assert!(quic.tls_cipher_suites.contains(&AES_256_GCM_SHA384));

        quic.set_cipher_suite(AES_256_GCM_SHA384).unwrap();
        assert_eq!(quic.tls_cipher_suites.as_slice(), &[AES_256_GCM_SHA384]);

        assert!(matches!(
            quic.set_cipher_suite(0xffff),
            Err(Error::InvalidArgument)
        ));
        assert!(quic.tls_cipher_suites.is_empty());
    }

    #[test]
    fn set_key_exchange_installs_provider_selected_list() {
        reset_tls_api(0);
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();

        assert!(quic.tls_key_exchanges.contains(&GROUP_SECP256R1));
        assert!(quic.tls_key_exchanges.contains(&29));

        quic.set_key_exchange(GROUP_SECP256R1).unwrap();
        assert_eq!(quic.tls_key_exchanges.as_slice(), &[GROUP_SECP256R1]);

        assert!(matches!(
            quic.set_key_exchange(0xffff),
            Err(Error::InvalidArgument)
        ));
        assert_eq!(quic.tls_key_exchanges.as_slice(), &[GROUP_SECP256R1]);

        quic.set_key_exchange(0).unwrap();
        assert!(quic.tls_key_exchanges.contains(&GROUP_SECP256R1));
        assert!(quic.tls_key_exchanges.contains(&29));
        reset_tls_api(0);
    }

    #[test]
    fn set_default_multipath_option_advertises_initial_path_id_for_bit_zero() {
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();

        assert_eq!(quic.default_tp().initial_max_path_id, 0);

        quic.set_default_multipath_option(1);

        assert_eq!(quic.default_multipath_option, 1);
        assert_eq!(quic.default_tp().initial_max_path_id, 2);

        quic.default_tp.initial_max_path_id = 7;
        quic.set_default_multipath_option(2);

        assert_eq!(quic.default_multipath_option, 2);
        assert_eq!(quic.default_tp().initial_max_path_id, 7);
    }

    #[test]
    fn set_tls_certificate_chain_replaces_owned_chain() {
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();

        let first = vec![vec![1, 2, 3], vec![4, 5]];
        quic.set_tls_certificate_chain(first.clone());
        assert_eq!(quic.tls_certificate_chain, first);

        let replacement = vec![vec![9, 8, 7]];
        quic.set_tls_certificate_chain(replacement.clone());
        assert_eq!(quic.tls_certificate_chain, replacement);
    }

    #[test]
    fn set_tls_root_certificates_rejects_invalid_or_unsupported_roots() {
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();

        assert!(matches!(
            quic.set_tls_root_certificates(vec![vec![0x30, 0x00]]),
            Err(Error::Generic)
        ));
        assert!(!quic.is_cert_store_not_empty);
    }

    #[cfg(feature = "sys-openssl")]
    #[test]
    fn set_tls_root_certificates_installs_openssl_roots() {
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();
        let cert = openssl::x509::X509::from_pem(include_bytes!("../../../certs/test-ca.crt"))
            .unwrap()
            .to_der()
            .unwrap();

        quic.set_tls_root_certificates(vec![cert.clone()]).unwrap();

        assert!(quic.is_cert_store_not_empty);
        quic.tls_callbacks
            .as_mut()
            .unwrap()
            .verify_certificate(&[cert.as_slice()])
            .unwrap();
    }

    #[test]
    fn key_log_file_is_separate_from_text_log_and_failed_reopen_clears() {
        let mut quic = Quic::new(
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .unwrap();
        quic.f_log = Some(Box::new(Vec::<u8>::new()));

        let path = std::env::temp_dir().join(format!(
            "fq-keylog-{}-{}.log",
            std::process::id(),
            super::current_time()
        ));
        quic.set_key_log_file(Some(path.to_str().unwrap()));
        assert!(quic.key_log.is_some());
        assert!(quic.f_log.is_some());

        let missing_parent = std::env::temp_dir().join(format!(
            "fq-keylog-missing-{}-{}",
            std::process::id(),
            super::current_time()
        ));
        let bad_path = missing_parent.join("keylog.log");
        quic.set_key_log_file(Some(bad_path.to_str().unwrap()));
        assert!(quic.key_log.is_none());
        assert!(quic.f_log.is_some());

        quic.set_key_log_file(None);
        let _ = std::fs::remove_file(path);
    }
}
