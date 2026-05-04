//! Translation of `quic/binlog.h`.
//!
//! Public surface of the binary trace ("binlog") backend.  The
//! header exposes two flavours of entry point:
//!
//! 1. **Top-level wiring** ([`Quic::set_binlog`],
//!    [`Quic::enable_binlog`]) that installs the binlog vtable on a
//!    QUIC context.  `binlog_dir == NULL` in C is the "stop tracing"
//!    sentinel; [`Quic::enable_binlog`] only flips the vtable
//!    pointer and is used when autoqlog wants the binary stream
//!    without a dedicated directory.
//! 2. **Per-event writers** that emit one trace record.  In C they
//!    all bottom out in `fwrite()` against either an out-of-band
//!    `FILE*` or `connection->f_binlog` pulled from the connection.  The
//!    Rust split mirrors that: the three file-only writers ([`pdu`],
//!    [`packet`], [`tls_ticket`]) are free functions on a [`File`]
//!    sink, and the rest hang as inherent methods on [`Connection`] (they
//!    pull the file handle from `connection.f_binlog` themselves).
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//! Bodies and the empty-test module land in later phases.
//!
//! Pointer-shape and translation policy notes for this module:
//!
//! * `FILE*` (the three low-level writers) → `&mut std::fs::File`.
//!   These calls borrow the handle for the duration of one record
//!   write — ownership stays with the connection (`connection->f_binlog`)
//!   or the caller.  The binlog stream is *binary*, so the text-side
//!   `&mut impl core::fmt::Write` convention used by
//!   [`crate::textlog`] does not apply here.
//! * `Quic*` / `Connection*` — every observed caller passes a non-NULL
//!   handle and the body mutates internal state (`quic->bin_log_fns`,
//!   `connection->f_binlog`, `quic->binlog_dir`).  Free functions whose
//!   first argument was one of these become inherent methods on the
//!   corresponding type.
//! * `Path*` — only ever accessed inside
//!   `binlog_get_path_id(connection, path_x)`, which dereferences `path_x`
//!   to read `unique_path_id`.  Every observed caller passes a
//!   non-NULL path handle, so this is `&mut Path` for parity with
//!   the unified-log dispatch trait (which takes the path mutably
//!   for the same hooks).
//! * `const ConnectionId*` (in [`pdu`] / [`packet`]) →
//!   `&ConnectionId`.  The C contract is "must be non-NULL"; every
//!   caller passes `&connection->initial_connection_id`.
//! * `ConnectionId* dcid` (in [`Connection::binlog_packet_lost`]) is
//!   nullable per `loss_recovery.c` — when no remote CID is known
//!   the C call site passes NULL and the body emits a single zero
//!   length byte.  Maps to `Option<&ConnectionId>`.  The C signature
//!   drops `const` but the body only reads through the pointer, so
//!   the Rust equivalent stays a shared borrow.
//! * `packet_header* ph` (in [`Connection::binlog_dropped_packet`],
//!   [`Connection::binlog_outgoing_packet`]) — the body only *reads*
//!   `ph->ptype` in the dropped path and reconstructs a fresh header
//!   in the outgoing path.  In [`Connection::binlog_outgoing_packet`] the
//!   parsed-header buffer is constructed locally, so no pointer
//!   crosses the API boundary.  In [`packet`] /
//!   [`Connection::binlog_dropped_packet`] we map `ph` to `&PacketHeader`
//!   (immutable borrow) — matching the unified-log trait shape.
//! * `ConnectionId connection_id` (in [`tls_ticket`]) is `Copy` and
//!   pass-by-value, mirroring the C ABI.
//! * `const struct sockaddr*` pairs (`addr_peer`, `addr_local` in
//!   [`pdu`]) → `&core::net::SocketAddr`, matching the convention
//!   established in [`crate::logger`].
//! * `const uint8_t* + size_t` argument pairs collapse to `&[u8]`
//!   (`bytes`/`bytes_max` in [`packet`], `params`/`param_length` in
//!   [`Connection::binlog_transport_extension`], `ticket`/`ticket_length`
//!   in [`tls_ticket`], `sni`/`sni_len` and `alpn`/`alpn_len` in
//!   [`Connection::binlog_negotiated_alpn`]).  An empty slice models the C
//!   `(NULL, 0)` callers cleanly.
//! * [`Connection::binlog_outgoing_packet`] carries *two* buffers: the
//!   unencrypted `bytes` (length passed separately as `length`) and
//!   the encrypted `send_buffer` (length passed separately as
//!   `send_length`).  Each pair collapses to one `&[u8]`.  The
//!   `pn_length` parameter — the offset of the packet-number field
//!   inside `bytes` — survives as a separate `usize`.
//! * `const PtlsIovec* alpn_list, size_t alpn_count` →
//!   `&[&[u8]]`, mirroring the unified-log trait shape.
//! * `int receiving` / `int is_local` are pure 0/1 flags promoted to
//!   `bool`; `int err` is a real signed status code so it stays
//!   `i32`.
//! * `unsigned char ecn` collapses to `u8`.
//! * `char const* binlog_dir` is the C "stop tracing when NULL"
//!   sentinel, so [`Quic::set_binlog`] takes `Option<&str>`.
//!   `Some(path)` installs the vtable and copies the directory name
//!   into the QUIC context; `None` leaves the directory cleared
//!   while still wiring up the vtable.
//! * `char const* trigger` (in [`Connection::binlog_packet_lost`]) is
//!   always a non-NULL static string literal at the call sites
//!   grepped in `loss_recovery.c`, so it maps to `&str`.
//!
//! Error handling: [`Quic::set_binlog`] mirrors the C `int` return
//! (always `0` today, but reserved for future failure modes) as
//! `Result<(), Error>` per the project's error-handling convention.

use std::fs::File;
use std::path::Path as FsPath;

use core::net::SocketAddr;

use crate::Instant;
use crate::Error;
use crate::internal::{Connection, PacketHeader, PacketType, Path};
use crate::{ConnectionId, Quic};

// ---------------------------------------------------------------------------
// Event-tag enum.
//
// The C `picoquic_log_event_type` is a sparse enum with explicit
// hex constants — the values are baked into the binary log format
// and must round-trip through the wire untouched.  Translated as a
// Rust enum with `#[repr(u32)]` so each variant keeps its
// wire-format tag; `repr(C)` is *not* used because the type does
// not cross an FFI boundary — the discriminant is only serialized
// as a varint by `binlog_compose_event_header`.

/// Event tag emitted at the start of every binary log record.  The
/// underlying integer values are part of the binlog wire format and
/// must not drift from the C enum.
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LogEventType {
    PduSent = 0x0002,
    PduRecv = 0x0003,

    PacketSent = 0x0008,
    PacketRecv = 0x0009,

    NewConnection = 0x0010,
    ConnectionClose = 0x0011,
    ConnectionIdUpdate = 0x0012,
    PacketLost = 0x0013,
    PacketDropped = 0x0014,
    PacketBuffered = 0x0015,

    TlsKeyUpdate = 0x0020,
    TlsKeyRetired = 0x0021,

    VersionUpdate = 0x0035,
    ParamUpdate = 0x0036,
    AlpnUpdate = 0x0037,
    CcUpdate = 0x0038,
    StreamUpdate = 0x0039,
    InfoMessage = 0x003a,

    FrameSent = 0x0082,
    FrameRecv = 0x0083,
}

// ---------------------------------------------------------------------------
// Low-level per-event writers.
//
// Free functions on a [`File`] sink — these don't carry a `Connection`
// handle, so calls are namespaced through the module path
// (`binlog::pdu(...)`, `binlog::packet(...)`,
// `binlog::tls_ticket(...)`).  Callers usually go through the
// [`Connection`] methods below, which thread through `connection.f_binlog`; the
// file-only writers are kept public for the contexts where the
// caller already owns the handle (e.g., the binlog backend's
// implementation of [`crate::logger::Logger`]).

/// Write a PDU arrival/departure record to `f`.
///
/// C: `void binlog_pdu(FILE*, const ConnectionId*, int,
/// uint64_t, const struct sockaddr*, const struct sockaddr*, size_t,
/// uint64_t, unsigned char)`.
pub fn pdu(
    _f: &mut File,
    _cid: &ConnectionId,
    _receiving: bool,
    _current_time: Instant,
    _addr_peer: &SocketAddr,
    _addr_local: &SocketAddr,
    _packet_length: usize,
    _unique_path_id: u64,
    _ecn: u8,
) {
    todo!()
}

/// Write a decrypted-packet record to `f`.  Binary alternative to
/// `log_decrypted_segment()`.
///
/// C: `void binlog_packet(FILE*, const ConnectionId*,
/// uint64_t, int, uint64_t, const packet_header*,
/// const uint8_t*, size_t)`.
pub fn packet(
    _f: &mut File,
    _cid: &ConnectionId,
    _path_id: u64,
    _receiving: bool,
    _current_time: Instant,
    _ph: &PacketHeader,
    _bytes: &[u8],
) {
    todo!()
}

/// Write a TLS session-ticket record to `f`.  Binary alternative
/// to `log_tls_ticket()`.  The connection-id parameter is `Copy`
/// and passed by value to mirror the C ABI.
///
/// C: `void binlog_picotls_ticket(FILE*, ConnectionId,
/// uint8_t*, uint16_t)`.
pub fn tls_ticket(_f: &mut File, _cnx_id: ConnectionId, _ticket: &[u8]) {
    todo!()
}

// ---------------------------------------------------------------------------
// High-level per-event writers — methods on [`Connection`].
//
// Each method pulls the file handle from `connection.f_binlog` and
// delegates to one of the low-level writers above (or composes
// several records).  The `binlog_` prefix on each method name
// keeps the binary-trace API distinct from the unified-log
// dispatch methods (`connection.log_*`) on the same type.

// REVIEW(open): replace these `impl Connection` blocks with a local
// trait (`Binlog`) implemented on `Connection`, so callers can opt into
// the capability and the `binlog_` prefix can drop.  Same shape
// applies to the per-Connection blocks in `logger`, `qlog`, and the
// `cc_*` accessors in `cc_common`.  Land it once Phase 2 has
// settled the dependency-trait conventions.
impl Connection {
    /// Report that a packet was dropped due to some error.
    ///
    /// C: `void binlog_dropped_packet(Connection*, Path*,
    /// packet_header*, size_t, int, uint64_t)`.
    pub fn binlog_dropped_packet(
        &mut self,
        _path_x: &mut Path,
        _ph: &PacketHeader,
        _packet_size: usize,
        _err: i32,
        _current_time: Instant,
    ) {
        todo!()
    }

    /// Report that a packet was buffered waiting for decryption.
    ///
    /// C: `void binlog_buffered_packet(Connection*, Path*,
    /// packet_type_enum, uint64_t)`.
    pub fn binlog_buffered_packet(
        &mut self,
        _path_x: &mut Path,
        _ptype: PacketType,
        _current_time: Instant,
    ) {
        todo!()
    }

    /// Binary alternative to `log_outgoing_segment()`.  `bytes`
    /// is the unencrypted payload (the C `length` parameter is
    /// folded into the slice); `send_buffer` is the encrypted,
    /// padded wire form.  `pn_length` is the offset of the
    /// packet-number field inside `bytes`.
    ///
    /// C: `void binlog_outgoing_packet(Connection*, Path*,
    /// uint8_t*, uint64_t, size_t, size_t, uint8_t*, size_t,
    /// uint64_t)`.
    pub fn binlog_outgoing_packet(
        &mut self,
        _path_x: &mut Path,
        _bytes: &[u8],
        _sequence_number: u64,
        _pn_length: usize,
        _send_buffer: &[u8],
        _current_time: Instant,
    ) {
        todo!()
    }

    /// Log a packet-lost event.  `dcid` is `None` when the remote
    /// connection ID is unknown — the C body emits a single zero
    /// byte in that case.
    ///
    /// C: `void binlog_packet_lost(Connection*, Path*,
    /// packet_type_enum, uint64_t, char const*,
    /// ConnectionId*, size_t, uint64_t)`.
    pub fn binlog_packet_lost(
        &mut self,
        _path_x: &mut Path,
        _ptype: PacketType,
        _sequence_number: u64,
        _trigger: &str,
        _dcid: Option<&ConnectionId>,
        _packet_size: usize,
        _current_time: Instant,
    ) {
        todo!()
    }

    /// Log negotiated SNI / ALPN.  Empty `sni`/`alpn` slices stand
    /// in for the C `(NULL, 0)` callers.
    ///
    /// C: `void binlog_negotiated_alpn(Connection*, int,
    /// uint8_t const*, size_t, uint8_t const*, size_t,
    /// const PtlsIovec*, size_t)`.
    pub fn binlog_negotiated_alpn(
        &mut self,
        _is_local: bool,
        _sni: &[u8],
        _alpn: &[u8],
        _alpn_list: &[&[u8]],
    ) {
        todo!()
    }

    /// Binary alternative to `log_transport_extension()`.
    ///
    /// C: `void binlog_transport_extension(Connection*, int, size_t,
    /// uint8_t*)`.
    pub fn binlog_transport_extension(&mut self, _is_local: bool, _params: &[u8]) {
        todo!()
    }

    /// Open the per-connection binlog file and emit the
    /// `new_connection` record.  Idempotent — the C body is a no-op
    /// when neither `quic->binlog_dir` nor `quic->qlog_dir` are
    /// set, or when `quic->bin_log_fns` is `NULL`.
    ///
    /// C: `void binlog_new_connection(Connection*)`.
    pub fn binlog_new_connection(&mut self) {
        todo!()
    }

    /// Emit the `connection_close` record and close the
    /// per-connection binlog file.  Safe to call when no binlog is
    /// currently open (the C body guards on `connection->f_binlog !=
    /// NULL`).
    ///
    /// C: `void binlog_close_connection(Connection*)`.
    pub fn binlog_close_connection(&mut self) {
        todo!()
    }

    /// Log the state of the congestion controller, retransmission
    /// queues, etc.  Called either just after processing an
    /// incoming packet or just after sending one.
    ///
    /// C: `void binlog_cc_dump(Connection*, Path*, uint64_t)`.
    pub fn binlog_cc_dump(&mut self, _path_x: &mut Path, _current_time: Instant) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Top-level wiring on the QUIC context.

impl Quic {
    /// Set the binary-log directory and install the binlog vtable
    /// on this context.  Pass `None` to clear the directory while
    /// still leaving the vtable in place — that is the C "stop
    /// binary tracing" sentinel (`binlog_dir == NULL`).
    ///
    /// C: `int picoquic_set_binlog(picoquic_quic_t*, char const*)`
    /// — the return is `0` today but reserved for failure modes;
    /// mapped to `Result<(), Error>` per the project's
    /// error-handling convention.
    pub fn set_binlog(
        &mut self,
        _binlog_dir: Option<&(impl AsRef<FsPath> + ?Sized)>,
    ) -> Result<(), Error> {
        todo!()
    }

    /// Enable binary logging without setting a directory — used
    /// when autoqlog wants the binlog stream as scratch space.
    /// Equivalent to [`Quic::set_binlog`] minus the directory
    /// bookkeeping.
    ///
    /// C: `void picoquic_enable_binlog(picoquic_quic_t*)`.
    pub fn enable_binlog(&mut self) {
        todo!()
    }
}

#[cfg(test)]
mod test {}
