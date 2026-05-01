//! Translation of `picoquic/picoquic_binlog.h`.
//!
//! Public surface of the binary trace ("binlog") backend.  The
//! header exposes two flavours of entry point:
//!
//! 1. **Top-level helpers** (`picoquic_set_binlog`,
//!    `picoquic_enable_binlog`) that install the binlog vtable on a
//!    QUIC context.  `binlog_dir == NULL` in C is the "stop tracing"
//!    sentinel; `picoquic_enable_binlog` only flips the vtable
//!    pointer and is used when autoqlog wants the binary stream
//!    without a dedicated directory.
//! 2. **Per-event writers** (`binlog_pdu`, `binlog_packet`,
//!    `binlog_dropped_packet`, …) that emit one trace record.  In C
//!    they all bottom out in `fwrite()` against either an
//!    out-of-band `FILE*` (the three "low-level" entry points
//!    `binlog_pdu` / `binlog_packet` / `binlog_picotls_ticket`) or
//!    `cnx->f_binlog` pulled from the connection.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//! Bodies and the empty-test module land in later phases.
//!
//! Pointer-shape and translation policy notes for this module:
//!
//! * `FILE*` (the three low-level writers) → `&mut picoquic_file_t`,
//!   re-using the opaque file handle introduced in
//!   [`crate::picoquic::picoquic_utils`].  These calls borrow the
//!   handle for the duration of one record write — ownership stays
//!   with the connection (`cnx->f_binlog`) or the caller.  Note that
//!   the binlog stream is *binary*, so the text-side
//!   `&mut dyn core::fmt::Write` convention used by
//!   [`crate::picoquic::picoquic_logger::picoquic_textlog_picotls_ticket`]
//!   does not apply here.
//! * `picoquic_quic_t*` / `picoquic_cnx_t*` — every observed caller
//!   passes a non-NULL handle and the body mutates internal state
//!   (`quic->bin_log_fns`, `cnx->f_binlog`, `quic->binlog_dir`).
//!   Both map to `&mut`.
//! * `picoquic_path_t*` — only ever accessed inside
//!   `binlog_get_path_id(cnx, path_x)`, which dereferences `path_x`
//!   to read `unique_path_id`.  Every observed caller passes a
//!   non-NULL path handle, so this is `&mut picoquic_path_t` for
//!   parity with the unified-log dispatch trait (which takes the
//!   path mutably for the same hooks).
//! * `const picoquic_connection_id_t*` (in `binlog_pdu` /
//!   `binlog_packet`) → `&picoquic_connection_id_t`.  The C contract
//!   is "must be non-NULL"; every caller passes
//!   `&cnx->initial_cnxid`.
//! * `picoquic_connection_id_t* dcid` (in `binlog_packet_lost`) is
//!   nullable per `loss_recovery.c` — when no remote CID is known
//!   the C call site passes NULL and the body emits a single zero
//!   length byte.  Maps to `Option<&picoquic_connection_id_t>`.
//!   The C signature drops `const` but the body only reads through
//!   the pointer, so the Rust equivalent stays a shared borrow.
//! * `picoquic_packet_header* ph` (in `binlog_dropped_packet`,
//!   `binlog_outgoing_packet`) — the body only *reads* `ph->ptype`
//!   in the dropped path and reconstructs a fresh header in the
//!   outgoing path.  In `binlog_outgoing_packet` the parsed-header
//!   buffer is constructed locally, so no pointer crosses the API
//!   boundary.  In `binlog_packet`/`binlog_dropped_packet` we map
//!   `ph` to `&picoquic_packet_header` (immutable borrow) — matching
//!   the unified-log trait shape.
//! * `picoquic_connection_id_t cnx_id` (in `binlog_picotls_ticket`)
//!   is `Copy` and pass-by-value, mirroring the C ABI.
//! * `const struct sockaddr*` pairs (`addr_peer`, `addr_local` in
//!   `binlog_pdu`) → `&core::net::SocketAddr`, matching the
//!   convention established in
//!   [`crate::picoquic::picoquic_unified_log`].
//! * `const uint8_t* + size_t` argument pairs collapse to `&[u8]`
//!   (`bytes`/`bytes_max` in `binlog_packet`,
//!   `params`/`param_length` in `binlog_transport_extension`,
//!   `ticket`/`ticket_length` in `binlog_picotls_ticket`,
//!   `sni`/`sni_len` and `alpn`/`alpn_len` in
//!   `binlog_negotiated_alpn`).  An empty slice models the C
//!   `(NULL, 0)` callers cleanly.
//! * `binlog_outgoing_packet` carries *two* buffers: the
//!   unencrypted `bytes` (length passed separately as `length`) and
//!   the encrypted `send_buffer` (length passed separately as
//!   `send_length`).  Each pair collapses to one `&[u8]`.  The
//!   `pn_length` parameter — the offset of the packet-number field
//!   inside `bytes` — survives as a separate `usize`.
//! * `const ptls_iovec_t* alpn_list, size_t alpn_count` →
//!   `&[ptls_iovec_t]`, mirroring the unified-log trait shape.
//! * `int receiving` / `int is_local` are pure 0/1 flags promoted to
//!   `bool`; `int err` is a real signed status code so it stays
//!   `i32`.
//! * `unsigned char ecn` collapses to `u8`.
//! * `char const* binlog_dir` is the C "stop tracing when NULL"
//!   sentinel, so [`picoquic_set_binlog`] takes `Option<&str>`.
//!   `Some(path)` installs the vtable and copies the directory name
//!   into the QUIC context; `None` leaves the directory cleared
//!   while still wiring up the vtable.
//! * `char const* trigger` (in `binlog_packet_lost`) is always a
//!   non-NULL static string literal at the call sites grepped in
//!   `loss_recovery.c`, so it maps to `&str`.
//!
//! Error handling: [`picoquic_set_binlog`] mirrors the C `int`
//! return (always `0` today, but reserved for future failure modes)
//! as `Result<(), ()>`.  TODO(error-enum): swap `()` for the
//! crate-level `Error` once it lands.

// Several writers preserve the C parameter list verbatim so the
// translation reads as a one-for-one mirror; clippy complains about
// the resulting argument counts.
#![allow(clippy::too_many_arguments)]
// `picoquic_set_binlog` returns a placeholder unit error until the
// crate-level `Error` enum lands.
#![allow(clippy::result_unit_err)]

use core::net::SocketAddr;

use crate::picoquic::picoquic::{picoquic_connection_id_t, picoquic_quic_t, ptls_iovec_t};
use crate::picoquic::picoquic_internal::{
    picoquic_cnx_t, picoquic_packet_header, picoquic_packet_type_enum, picoquic_path_t,
};
use crate::picoquic::picoquic_utils::picoquic_file_t;

// ---------------------------------------------------------------------------
// Event-tag enum.
//
// The C `picoquic_log_event_type` is a sparse enum with explicit
// hex constants (the values are baked into the binary log format).
// Translated as a Rust enum with `#[repr(u32)]` so each variant
// keeps its wire-format tag; `repr(C)` is *not* used because the
// type does not cross an FFI boundary — the discriminant is only
// serialized as a varint by `binlog_compose_event_header`.

/// Event tag emitted at the start of every binary log record.  The
/// underlying integer values are part of the binlog wire format and
/// must not drift from the C enum.
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum picoquic_log_event_type {
    picoquic_log_event_pdu_sent = 0x0002,
    picoquic_log_event_pdu_recv = 0x0003,

    picoquic_log_event_packet_sent = 0x0008,
    picoquic_log_event_packet_recv = 0x0009,

    picoquic_log_event_new_connection = 0x0010,
    picoquic_log_event_connection_close = 0x0011,
    picoquic_log_event_connection_id_update = 0x0012,
    picoquic_log_event_packet_lost = 0x0013,
    picoquic_log_event_packet_dropped = 0x0014,
    picoquic_log_event_packet_buffered = 0x0015,

    picoquic_log_event_tls_key_update = 0x0020,
    picoquic_log_event_tls_key_retired = 0x0021,

    picoquic_log_event_version_update = 0x0035,
    picoquic_log_event_param_update = 0x0036,
    picoquic_log_event_alpn_update = 0x0037,
    picoquic_log_event_cc_update = 0x0038,
    picoquic_log_event_stream_update = 0x0039,
    picoquic_log_event_info_message = 0x003a,

    picoquic_log_event_frame_sent = 0x0082,
    picoquic_log_event_frame_recv = 0x0083,
}

// ---------------------------------------------------------------------------
// Per-event binary writers.

/// Log PDU arrival or departure.
///
/// C: `void binlog_pdu(FILE*, const picoquic_connection_id_t*, int,
/// uint64_t, const struct sockaddr*, const struct sockaddr*, size_t,
/// uint64_t, unsigned char)`.
pub fn binlog_pdu(
    _f: &mut picoquic_file_t,
    _cid: &picoquic_connection_id_t,
    _receiving: bool,
    _current_time: u64,
    _addr_peer: &SocketAddr,
    _addr_local: &SocketAddr,
    _packet_length: usize,
    _unique_path_id: u64,
    _ecn: u8,
) {
    todo!()
}

/// Binary alternative to `picoquic_log_decrypted_segment()`.
///
/// C: `void binlog_packet(FILE*, const picoquic_connection_id_t*,
/// uint64_t, int, uint64_t, const picoquic_packet_header*,
/// const uint8_t*, size_t)`.
pub fn binlog_packet(
    _f: &mut picoquic_file_t,
    _cid: &picoquic_connection_id_t,
    _path_id: u64,
    _receiving: bool,
    _current_time: u64,
    _ph: &picoquic_packet_header,
    _bytes: &[u8],
) {
    todo!()
}

/// Report that a packet was dropped due to some error.
///
/// C: `void binlog_dropped_packet(picoquic_cnx_t*, picoquic_path_t*,
/// picoquic_packet_header*, size_t, int, uint64_t)`.
pub fn binlog_dropped_packet(
    _cnx: &mut picoquic_cnx_t,
    _path_x: &mut picoquic_path_t,
    _ph: &picoquic_packet_header,
    _packet_size: usize,
    _err: i32,
    _current_time: u64,
) {
    todo!()
}

/// Report that a packet was buffered waiting for decryption.
///
/// C: `void binlog_buffered_packet(picoquic_cnx_t*, picoquic_path_t*,
/// picoquic_packet_type_enum, uint64_t)`.
pub fn binlog_buffered_packet(
    _cnx: &mut picoquic_cnx_t,
    _path_x: &mut picoquic_path_t,
    _ptype: picoquic_packet_type_enum,
    _current_time: u64,
) {
    todo!()
}

/// Binary alternative to `picoquic_log_outgoing_segment()`.  `bytes`
/// is the unencrypted payload (the C `length` parameter is folded
/// into the slice); `send_buffer` is the encrypted, padded wire
/// form.  `pn_length` is the offset of the packet-number field
/// inside `bytes`.
///
/// C: `void binlog_outgoing_packet(picoquic_cnx_t*, picoquic_path_t*,
/// uint8_t*, uint64_t, size_t, size_t, uint8_t*, size_t, uint64_t)`.
pub fn binlog_outgoing_packet(
    _cnx: &mut picoquic_cnx_t,
    _path_x: &mut picoquic_path_t,
    _bytes: &[u8],
    _sequence_number: u64,
    _pn_length: usize,
    _send_buffer: &[u8],
    _current_time: u64,
) {
    todo!()
}

/// Log a packet-lost event.  `dcid` is `None` when the remote
/// connection ID is unknown — the C body emits a single zero byte
/// in that case.
///
/// C: `void binlog_packet_lost(picoquic_cnx_t*, picoquic_path_t*,
/// picoquic_packet_type_enum, uint64_t, char const*,
/// picoquic_connection_id_t*, size_t, uint64_t)`.
pub fn binlog_packet_lost(
    _cnx: &mut picoquic_cnx_t,
    _path_x: &mut picoquic_path_t,
    _ptype: picoquic_packet_type_enum,
    _sequence_number: u64,
    _trigger: &str,
    _dcid: Option<&picoquic_connection_id_t>,
    _packet_size: usize,
    _current_time: u64,
) {
    todo!()
}

/// Log negotiated SNI / ALPN.  Empty `sni`/`alpn` slices stand in
/// for the C `(NULL, 0)` callers.
///
/// C: `void binlog_negotiated_alpn(picoquic_cnx_t*, int,
/// uint8_t const*, size_t, uint8_t const*, size_t,
/// const ptls_iovec_t*, size_t)`.
pub fn binlog_negotiated_alpn(
    _cnx: &mut picoquic_cnx_t,
    _is_local: bool,
    _sni: &[u8],
    _alpn: &[u8],
    _alpn_list: &[ptls_iovec_t],
) {
    todo!()
}

/// Binary alternative to `picoquic_log_transport_extension()`.
///
/// C: `void binlog_transport_extension(picoquic_cnx_t*, int, size_t,
/// uint8_t*)`.
pub fn binlog_transport_extension(_cnx: &mut picoquic_cnx_t, _is_local: bool, _params: &[u8]) {
    todo!()
}

/// Binary alternative to `picoquic_log_tls_ticket()`.  The
/// connection-id parameter is `Copy` and passed by value to mirror
/// the C ABI.
///
/// C: `void binlog_picotls_ticket(FILE*, picoquic_connection_id_t,
/// uint8_t*, uint16_t)`.
pub fn binlog_picotls_ticket(
    _f: &mut picoquic_file_t,
    _cnx_id: picoquic_connection_id_t,
    _ticket: &[u8],
) {
    todo!()
}

/// Open the per-connection binlog file and emit the
/// `new_connection` record.  Idempotent — the C body is a no-op
/// when neither `quic->binlog_dir` nor `quic->qlog_dir` are set, or
/// when `quic->bin_log_fns` is `NULL`.
///
/// C: `void binlog_new_connection(picoquic_cnx_t*)`.
pub fn binlog_new_connection(_cnx: &mut picoquic_cnx_t) {
    todo!()
}

/// Emit the `connection_close` record and close the per-connection
/// binlog file.  Safe to call when no binlog is currently open
/// (the C body guards on `cnx->f_binlog != NULL`).
///
/// C: `void binlog_close_connection(picoquic_cnx_t*)`.
pub fn binlog_close_connection(_cnx: &mut picoquic_cnx_t) {
    todo!()
}

/// Log the state of the congestion controller, retransmission
/// queues, etc.  Called either just after processing an incoming
/// packet or just after sending one.
///
/// C: `void binlog_cc_dump(picoquic_cnx_t*, picoquic_path_t*,
/// uint64_t)`.
pub fn binlog_cc_dump(
    _cnx: &mut picoquic_cnx_t,
    _path_x: &mut picoquic_path_t,
    _current_time: u64,
) {
    todo!()
}

// ---------------------------------------------------------------------------
// Top-level wiring on the QUIC context.

/// Set the binary-log directory and install the binlog vtable on
/// `quic`.  Pass `None` to clear the directory while still leaving
/// the vtable in place — that is the C "stop binary tracing"
/// sentinel (`binlog_dir == NULL`).
///
/// C: `int picoquic_set_binlog(picoquic_quic_t*, char const*)` — the
/// return is `0` today but reserved for failure modes; mapped to
/// `Result<(), ()>` per the project's error-handling convention.
// TODO(error-enum): swap `()` for the crate's `Error` once it lands.
pub fn picoquic_set_binlog(
    _quic: &mut picoquic_quic_t,
    _binlog_dir: Option<&str>,
) -> Result<(), ()> {
    todo!()
}

/// Enable binary logging without setting a directory — used when
/// autoqlog wants the binlog stream as scratch space.  Equivalent
/// to `picoquic_set_binlog` minus the directory bookkeeping.
///
/// C: `void picoquic_enable_binlog(picoquic_quic_t*)`.
pub fn picoquic_enable_binlog(_quic: &mut picoquic_quic_t) {
    todo!()
}

#[cfg(test)]
mod test {}
