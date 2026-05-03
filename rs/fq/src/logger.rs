//! Translation of `quic/unified_log.h`.
//!
//! Logging API.  The quic library can produce three complementary
//! logs per QUIC context — a textual trace (see [`crate::textlog`]),
//! a structured binary trace (see [`crate::binlog`]), and a qlog
//! (see [`crate::qlog`]) — but most applications only enable a
//! subset.  Each backend is described by a single [`Logger`] trait
//! object that the application optionally installs on the QUIC
//! context.  The inherent methods on [`Quic`] and [`Connection`]
//! in this module are the dispatch layer: they fan a single log
//! event out to whichever backends are registered.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//!
//! Pointer-shape and translation policy notes for this module:
//!
//! * The C `picoquic_unified_logging_t` is a 16-slot vtable.  Per
//!   the Phase 1 rule "function pointers map to traits" we collapse
//!   the whole vtable into a single trait, [`Logger`],
//!   because every backend installs all sixteen entries together.
//!   A QUIC context will eventually hold three optional
//!   `Box<dyn Logger>` slots (`text_log_fns` / `bin_log_fns`
//!   / `qlog_fns`); that restructuring lands when `internal.h` is
//!   translated.
//! * `Quic*` / `Connection*` — every observed caller passes a non-NULL
//!   pointer and the logger callbacks may mutate internal state.
//!   Free functions whose first argument is one of these become
//!   inherent methods on [`Quic`] / [`Connection`].
//! * `Path*` — non-NULL for every per-path log event except
//!   [`Logger::dropped_packet`] and
//!   [`Logger::packet`], where `packet.c` may pass NULL when
//!   the path lookup failed.  Those two get `Option<&mut Path>`; the
//!   rest get `&mut Path`.
//! * `ConnectionId*` — `dcid` in [`Logger::packet_lost`] is
//!   nullable per `loss_recovery.c` (the call site explicitly
//!   substitutes NULL when no remote CID is known); the `cid`
//!   parameter to [`Logger::quic_app_message`] is always
//!   non-NULL and read-only.  `Option<&ConnectionId>` and
//!   `&ConnectionId` respectively.
//! * `struct sockaddr*` → `&core::net::SocketAddr`, matching the
//!   convention established in [`crate`].
//! * `va_list` / variadic — collapsed to [`core::fmt::Arguments`].
//!   The C `_v` variants disappear: the `format_args!` macro at the
//!   call site is the Rust substitute for both the variadic and the
//!   `va_list` flavours.
//! * `int receiving` / `int is_local` — pure 0/1 flags, promoted to
//!   `bool`.
//! * `const uint8_t* + size_t` pairs collapse to `&[u8]`; a NULL
//!   pointer with len 0 in C maps cleanly to an empty Rust slice.
//! * `picoquic_log_dropped_packet`'s `raw_data` parameter is
//!   `UNUSED` in the C wrapper and never reaches the trait, so it is
//!   dropped from the Rust API entirely.

use core::net::SocketAddr;

use crate::internal::{PacketHeader, PacketType};
use crate::{Connection, ConnectionId, Path, Quic};

// ---------------------------------------------------------------------------
// Unified-logger vtable.
//
// One trait covers all sixteen function-pointer slots of
// `picoquic_unified_logging_t`.  Every concrete logger (text /
// binary / qlog) implements the full trait — the C source enforces
// "if a logging type is documented, all three functions for that
// type shall be documented as well" by convention; the trait
// requirement makes that explicit.
//
// Method names drop the `log_` prefix from the C function-pointer
// typedefs since the trait name already conveys "logging".

/// One installable logging backend.  Each method emits one record
/// of the named kind into whatever sink the backend owns (text
/// file, binary trace, qlog).
pub trait Logger {
    /// Emit a free-form application message routed by a connection-id
    /// hint rather than a live `Connection` handle.  C:
    /// `picoquic_log_quic_app_message_fn`.
    fn quic_app_message(
        &mut self,
        quic: &mut Quic,
        cid: &ConnectionId,
        args: core::fmt::Arguments<'_>,
    );

    /// Emit a context-level UDP-datagram arrival or departure for
    /// an unknown connection.  C: `picoquic_log_quic_pdu_fn`.
    fn quic_pdu(
        &mut self,
        quic: &mut Quic,
        receiving: bool,
        current_time: u64,
        cid64: u64,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        packet_length: usize,
    );

    /// Release any QUIC-context-level resource the backend owns
    /// (file handles, qlog buffers, …).  Invoked at context
    /// teardown.  C: `picoquic_log_quic_close`.
    fn quic_close(&mut self, quic: &mut Quic);

    /// Emit a free-form application message attached to a live
    /// connection.  C: `picoquic_log_app_message_fn` — the `va_list`
    /// flavour collapses to [`core::fmt::Arguments`].
    fn app_message(&mut self, cnx: &mut Connection, args: core::fmt::Arguments<'_>);

    /// Emit a per-connection UDP-datagram arrival or departure
    /// record.  C: `picoquic_log_pdu_fn`.
    fn pdu(
        &mut self,
        cnx: &mut Connection,
        receiving: bool,
        current_time: u64,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        packet_length: usize,
        unique_path_id: u64,
        ecn: u8,
    );

    /// Emit a decrypted-packet record.  `receiving == true` for
    /// arrivals.  C: `picoquic_log_packet_fn`.
    fn packet(
        &mut self,
        cnx: &mut Connection,
        path_x: Option<&mut Path>,
        receiving: bool,
        current_time: u64,
        ph: &PacketHeader,
        bytes: &[u8],
    );

    /// Emit a record that the packet was dropped due to some error.
    /// C: `picoquic_log_dropped_packet_fn`.
    fn dropped_packet(
        &mut self,
        cnx: &mut Connection,
        path_x: Option<&mut Path>,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: u64,
    );

    /// Emit a record that the packet was buffered waiting for
    /// decryption.  C: `picoquic_log_buffered_packet_fn`.
    fn buffered_packet(
        &mut self,
        cnx: &mut Connection,
        path_x: &mut Path,
        ptype: PacketType,
        current_time: u64,
    );

    /// Emit a record that a packet was formatted, ready to be sent.
    /// `bytes` is the unencrypted form (length carried in the
    /// slice); `send_buffer` is the encrypted-and-padded wire form.
    /// `pn_length` is the length of the packet-number field within
    /// `bytes`.  C: `picoquic_log_outgoing_packet_fn`.
    fn outgoing_packet(
        &mut self,
        cnx: &mut Connection,
        path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: u64,
    );

    /// Emit a packet-lost record.  `dcid` may be `None` when the
    /// remote connection ID is not known at the time of detection.
    /// C: `picoquic_log_packet_lost_fn`.
    fn packet_lost(
        &mut self,
        cnx: &mut Connection,
        path_x: &mut Path,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: u64,
    );

    /// Emit a negotiated-ALPN record.  Empty `sni` / `alpn` slices
    /// stand in for the C `(NULL, 0)` callers.  C:
    /// `picoquic_log_negotiated_alpn_fn`.
    fn negotiated_alpn(
        &mut self,
        cnx: &mut Connection,
        is_local: bool,
        sni: &[u8],
        alpn: &[u8],
        alpn_list: &[&[u8]],
    );

    /// Emit a transport-extension record formatted by the local peer
    /// (`is_local == true`) or received from the remote peer.  C:
    /// `picoquic_log_transport_extension_fn`.
    fn transport_extension(&mut self, cnx: &mut Connection, is_local: bool, params: &[u8]);

    /// Emit a TLS session-ticket record.  C:
    /// `picoquic_log_tls_ticket_fn`.
    fn tls_ticket(&mut self, cnx: &mut Connection, ticket: &[u8]);

    /// Emit a connection-start record.  C:
    /// `picoquic_log_new_connection_fn`.
    fn new_connection(&mut self, cnx: &mut Connection);

    /// Emit a connection-end record.  C:
    /// `picoquic_log_close_connection_fn`.
    fn close_connection(&mut self, cnx: &mut Connection);

    /// Emit a snapshot of congestion-control parameters for one
    /// path.  C: `picoquic_log_cc_dump_fn` — the public dispatcher
    /// [`Connection::log_cc_dump`] iterates the connection's paths and
    /// invokes this method per-path.
    fn cc_dump(&mut self, cnx: &mut Connection, path_x: &mut Path, current_time: u64);
}

// ---------------------------------------------------------------------------
// Public dispatch layer.
//
// Each method below fans the event out to whichever of the three
// logger slots (`text_log_fns`, `bin_log_fns`, `qlog_fns`) the QUIC
// context has installed.  Phase 1 leaves bodies as `todo!()`;
// Phase 3 fills in the dispatch.

impl Quic {
    /// Log an application-supplied message that is not attached to a
    /// live connection.  The connection-id `cid` is used purely as a
    /// routing hint by the backends.  Callers form the formatted
    /// message at the call site with `format_args!`.
    ///
    /// C: `picoquic_log_context_free_app_message`.
    pub fn log_app_message(&mut self, _cid: &ConnectionId, _args: core::fmt::Arguments<'_>) {
        todo!()
    }

    /// Log arrival or departure of a UDP datagram for an unknown
    /// connection.  `cid64` is the would-be connection-id rendered
    /// as a 64-bit value.
    ///
    /// C: `picoquic_log_quic_pdu`.
    pub fn log_pdu(
        &mut self,
        _receiving: bool,
        _current_time: u64,
        _cid64: u64,
        _addr_peer: &SocketAddr,
        _addr_local: &SocketAddr,
        _packet_length: usize,
    ) {
        todo!()
    }

    /// Tear down every installed logging backend, releasing the
    /// resources each one holds (file handles, qlog buffers, …).
    /// Invoked at QUIC-context teardown.
    ///
    /// C: `picoquic_log_close_logs`.
    pub fn close_logs(&mut self) {
        todo!()
    }
}

impl Connection {
    /// Append `args` to the connection's text log.  In C this exists
    /// in two flavours (`picoquic_log_app_message` and a `_v`/
    /// `va_list` twin); Rust folds them into one entry point — the
    /// `format_args!` macro at the call site is the variadic
    /// substitute.
    ///
    /// C: `picoquic_log_app_message`.
    pub fn log_app_message(&mut self, _args: core::fmt::Arguments<'_>) {
        todo!()
    }

    /// Log arrival or departure of a UDP datagram on this
    /// connection.
    ///
    /// C: `picoquic_log_pdu`.
    pub fn log_pdu(
        &mut self,
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

    /// Log a decrypted packet.  `receiving == true` for an arrival.
    /// `path_x` is `None` when the path lookup failed (C source
    /// passes NULL).
    ///
    /// C: `picoquic_log_packet`.
    pub fn log_packet(
        &mut self,
        _path_x: Option<&mut Path>,
        _receiving: bool,
        _current_time: u64,
        _ph: &PacketHeader,
        _bytes: &[u8],
    ) {
        todo!()
    }

    /// Report that a packet was dropped due to some error.  The C
    /// wrapper takes a `raw_data` buffer that it then ignores
    /// (`UNUSED(raw_data)`); the parameter is dropped here.
    ///
    /// C: `picoquic_log_dropped_packet`.
    pub fn log_dropped_packet(
        &mut self,
        _path_x: Option<&mut Path>,
        _ph: &PacketHeader,
        _packet_size: usize,
        _err: i32,
        _current_time: u64,
    ) {
        todo!()
    }

    /// Report that a packet was buffered waiting for decryption.
    ///
    /// C: `picoquic_log_buffered_packet`.
    pub fn log_buffered_packet(
        &mut self,
        _path_x: &mut Path,
        _ptype: PacketType,
        _current_time: u64,
    ) {
        todo!()
    }

    /// Log that a packet was formatted, ready to be sent.  `bytes`
    /// is the unencrypted packet (slice length subsumes the C
    /// `length` parameter); `send_buffer` is the encrypted wire
    /// form.  `pn_length` is the length of the packet-number field
    /// within `bytes`.
    ///
    /// C: `picoquic_log_outgoing_packet`.
    pub fn log_outgoing_packet(
        &mut self,
        _path_x: &mut Path,
        _bytes: &[u8],
        _sequence_number: u64,
        _pn_length: usize,
        _send_buffer: &[u8],
        _current_time: u64,
    ) {
        todo!()
    }

    /// Log a packet-lost event.  `dcid` is `None` when the remote
    /// connection ID is unknown.
    ///
    /// C: `picoquic_log_packet_lost`.
    pub fn log_packet_lost(
        &mut self,
        _path_x: &mut Path,
        _ptype: PacketType,
        _sequence_number: u64,
        _trigger: &str,
        _dcid: Option<&ConnectionId>,
        _packet_size: usize,
        _current_time: u64,
    ) {
        todo!()
    }

    /// Log negotiated SNI/ALPN.  Empty `sni`/`alpn` slices match the
    /// C callers that pass `(NULL, 0)`.
    ///
    /// C: `picoquic_log_negotiated_alpn`.
    pub fn log_negotiated_alpn(
        &mut self,
        _is_local: bool,
        _sni: &[u8],
        _alpn: &[u8],
        _alpn_list: &[&[u8]],
    ) {
        todo!()
    }

    /// Log a transport-extension blob.  `is_local == true` when the
    /// extension was formatted by the local peer; `false` when it
    /// was received.
    ///
    /// C: `picoquic_log_transport_extension`.
    pub fn log_transport_extension(&mut self, _is_local: bool, _params: &[u8]) {
        todo!()
    }

    /// Log a TLS session ticket.
    ///
    /// C: `picoquic_log_tls_ticket`.
    pub fn log_tls_ticket(&mut self, _ticket: &[u8]) {
        todo!()
    }

    /// Log the start of this connection.
    ///
    /// C: `picoquic_log_new_connection`.
    pub fn log_new_connection(&mut self) {
        todo!()
    }

    /// Log the end of this connection.
    ///
    /// C: `picoquic_log_close_connection`.
    pub fn log_close_connection(&mut self) {
        todo!()
    }

    /// Log a snapshot of congestion-control parameters across every
    /// path on the connection.  Iterates `cnx->path[…]` internally
    /// and dispatches per-path through
    /// [`Logger::cc_dump`].
    ///
    /// C: `picoquic_log_cc_dump`.
    pub fn log_cc_dump(&mut self, _current_time: u64) {
        todo!()
    }
}
