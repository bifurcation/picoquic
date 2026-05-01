//! Translation of `picoquic/picoquic_unified_log.h`.
//!
//! Unified logging API.  The picoquic library can produce three
//! complementary logs per QUIC context — a textual trace, a
//! structured binary trace, and a qlog — but most applications only
//! enable a subset.  Each backend is documented as a vtable of
//! function pointers (`UnifiedLogging` in C) that the
//! application optionally installs on the QUIC context.  The free
//! functions in this module are the dispatch layer: they fan a
//! single log event out to whichever backends are registered.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.
//!
//! Pointer-shape and translation policy notes for this module:
//!
//! * The C `UnifiedLogging` is a 16-slot vtable.  Per
//!   the Phase 1 rule "function pointers map to traits" we collapse
//!   the whole vtable into a single trait,
//!   [`UnifiedLogging`], because every backend
//!   installs all sixteen entries together.  A QUIC context will
//!   eventually hold three optional `Box<dyn UnifiedLogging>`
//!   slots (`text_log_fns` / `bin_log_fns` / `qlog_fns`); that
//!   restructuring lands when `picoquic_internal.h` is translated.
//! * `picoquic_quic_t*` / `picoquic_cnx_t*` — every observed caller
//!   passes a non-NULL pointer and the logger callbacks may mutate
//!   internal state; both map to `&mut`.
//! * `picoquic_path_t*` — non-NULL for every per-path log event
//!   except `picoquic_log_dropped_packet` and `picoquic_log_packet`,
//!   where `packet.c` may pass NULL when the path lookup failed.
//!   Those two get `Option<&mut picoquic_path_t>`; the rest get
//!   `&mut picoquic_path_t`.
//! * `picoquic_connection_id_t*` — `dcid` in `log_packet_lost` is
//!   nullable per `loss_recovery.c` (the call site explicitly
//!   substitutes NULL when no remote CID is known); the `cid`
//!   parameter to `log_quic_app_message` is always non-NULL and
//!   read-only.  `Option<&picoquic_connection_id_t>` and
//!   `&picoquic_connection_id_t` respectively.
//! * `struct sockaddr*` → `&core::net::SocketAddr`, matching the
//!   convention established in [`crate`].
//! * `va_list` / variadic — collapsed to `core::fmt::Arguments<'_>`,
//!   matching the existing [`crate::picoquic_log_app_message`]
//!   stub.  The C `_v` variants disappear (in Rust the `format_args!`
//!   macro produces `Arguments` at the call site, so the variadic
//!   and `va_list` flavours are redundant).
//! * `int receiving` / `int is_local` — pure 0/1 flags, promoted to
//!   `bool`.
//! * `const uint8_t* + size_t` pairs collapse to `&[u8]`; a NULL
//!   pointer with len 0 in C maps cleanly to an empty Rust slice.
//! * `picoquic_log_dropped_packet`'s `raw_data` parameter is
//!   `UNUSED` in the C implementation but remains in the wrapper
//!   signature; preserved here as `_raw_data: &[u8]` for source
//!   parity (Phase 3 may drop it once the wrapper is implemented).

#![allow(non_camel_case_types)]

use core::net::SocketAddr;

use crate::internal::{picoquic_packet_header, picoquic_packet_type_enum};
use crate::{
    picoquic_cnx_t, picoquic_connection_id_t, picoquic_path_t, picoquic_quic_t, ptls_iovec_t,
};

// ---------------------------------------------------------------------------
// Unified-logger vtable.
//
// One trait covers all sixteen function-pointer slots of
// `UnifiedLogging`.  Every concrete logger (text /
// binary / qlog) implements the full trait — the C source enforces
// "if a logging type is documented, all three functions for that
// type shall be documented as well" by convention; the trait
// requirement makes that explicit.

/// Vtable trait covering every entry of the C
/// `picoquic_unified_logging_t` struct.
pub trait UnifiedLogging {
    /// Log an application-supplied message that is not attached to a
    /// live connection — the QUIC context plus a connection-id hint
    /// supply the routing.  C: `picoquic_log_quic_app_message_fn`.
    fn log_quic_app_message(
        &mut self,
        quic: &mut picoquic_quic_t,
        cid: &picoquic_connection_id_t,
        args: core::fmt::Arguments<'_>,
    );

    /// Log arrival or departure of an UDP datagram for an unknown
    /// connection.  C: `picoquic_log_quic_pdu_fn`.
    fn log_quic_pdu(
        &mut self,
        quic: &mut picoquic_quic_t,
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
    fn log_quic_close(&mut self, quic: &mut picoquic_quic_t);

    /// Log a free-form application message attached to a
    /// connection.  C: `picoquic_log_app_message_fn` (the `va_list`
    /// flavour collapses to `core::fmt::Arguments`).
    fn log_app_message(&mut self, cnx: &mut picoquic_cnx_t, args: core::fmt::Arguments<'_>);

    /// Log arrival or departure of an UDP datagram on a connection.
    /// C: `picoquic_log_pdu_fn`.
    fn log_pdu(
        &mut self,
        cnx: &mut picoquic_cnx_t,
        receiving: bool,
        current_time: u64,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        packet_length: usize,
        unique_path_id: u64,
        ecn: u8,
    );

    /// Log a decrypted packet.  `receiving == true` for arrivals.
    /// C: `picoquic_log_packet_fn`.
    fn log_packet(
        &mut self,
        cnx: &mut picoquic_cnx_t,
        path_x: Option<&mut picoquic_path_t>,
        receiving: bool,
        current_time: u64,
        ph: &picoquic_packet_header,
        bytes: &[u8],
    );

    /// Report that a packet was dropped due to some error.  C:
    /// `picoquic_log_dropped_packet_fn`.
    fn log_dropped_packet(
        &mut self,
        cnx: &mut picoquic_cnx_t,
        path_x: Option<&mut picoquic_path_t>,
        ph: &picoquic_packet_header,
        packet_size: usize,
        err: i32,
        current_time: u64,
    );

    /// Report that a packet was buffered waiting for decryption.
    /// C: `picoquic_log_buffered_packet_fn`.
    fn log_buffered_packet(
        &mut self,
        cnx: &mut picoquic_cnx_t,
        path_x: &mut picoquic_path_t,
        ptype: picoquic_packet_type_enum,
        current_time: u64,
    );

    /// Log that a packet was formatted, ready to be sent.  `bytes`
    /// is the unencrypted form (length carried in the slice);
    /// `send_buffer` is the encrypted-and-padded wire form.
    /// `pn_length` is the length of the packet-number field within
    /// `bytes`.  C: `picoquic_log_outgoing_packet_fn`.
    fn log_outgoing_packet(
        &mut self,
        cnx: &mut picoquic_cnx_t,
        path_x: &mut picoquic_path_t,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: u64,
    );

    /// Log a packet-lost event.  `dcid` may be `None` when the
    /// remote connection ID is not known at the time of detection.
    /// C: `picoquic_log_packet_lost_fn`.
    fn log_packet_lost(
        &mut self,
        cnx: &mut picoquic_cnx_t,
        path_x: &mut picoquic_path_t,
        ptype: picoquic_packet_type_enum,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&picoquic_connection_id_t>,
        packet_size: usize,
        current_time: u64,
    );

    /// Log negotiated ALPN.  Empty `sni` / `alpn` slices stand in
    /// for the C `(NULL, 0)` callers.  C:
    /// `picoquic_log_negotiated_alpn_fn`.
    fn log_negotiated_alpn(
        &mut self,
        cnx: &mut picoquic_cnx_t,
        is_local: bool,
        sni: &[u8],
        alpn: &[u8],
        alpn_list: &[ptls_iovec_t],
    );

    /// Log a transport-extension blob, formatted by the local peer
    /// (`is_local == true`) or received from the remote peer.  C:
    /// `picoquic_log_transport_extension_fn`.
    fn log_transport_extension(&mut self, cnx: &mut picoquic_cnx_t, is_local: bool, params: &[u8]);

    /// Log a TLS session ticket.  C: `picoquic_log_tls_ticket_fn`
    /// (the field on the vtable is named `log_picotls_ticket` in
    /// C; preserved verbatim).
    fn log_picotls_ticket(&mut self, cnx: &mut picoquic_cnx_t, ticket: &[u8]);

    /// Log the start of a connection.  C:
    /// `picoquic_log_new_connection_fn`.
    fn log_new_connection(&mut self, cnx: &mut picoquic_cnx_t);

    /// Log the end of a connection.  C:
    /// `picoquic_log_close_connection_fn`.
    fn log_close_connection(&mut self, cnx: &mut picoquic_cnx_t);

    /// Log a snapshot of congestion-control parameters for one
    /// path.  C: `picoquic_log_cc_dump_fn` (note: the public
    /// wrapper [`picoquic_log_cc_dump`] iterates paths and invokes
    /// this method per-path).
    fn log_cc_dump(
        &mut self,
        cnx: &mut picoquic_cnx_t,
        path_x: &mut picoquic_path_t,
        current_time: u64,
    );
}

// ---------------------------------------------------------------------------
// Public dispatch layer.
//
// These free functions are the shape the application calls.  Each
// fans out the event to whichever of the three logger slots
// (`text_log_fns`, `bin_log_fns`, `qlog_fns`) the QUIC context has
// installed.  Phase 1 leaves bodies as `todo!()`; Phase 3 fills in
// the dispatch.

/// Log an event that cannot be attached to a specific connection.
/// C: `picoquic_log_context_free_app_message`.  The C variadic is
/// folded into `core::fmt::Arguments<'_>`; callers form the
/// formatted message at the call site with `format_args!`.
pub fn picoquic_log_context_free_app_message(
    _quic: &mut picoquic_quic_t,
    _cid: &picoquic_connection_id_t,
    _args: core::fmt::Arguments<'_>,
) {
    todo!()
}

/// Log arrival or departure of an UDP datagram for an unknown
/// connection.  C: `picoquic_log_quic_pdu`.
pub fn picoquic_log_quic_pdu(
    _quic: &mut picoquic_quic_t,
    _receiving: bool,
    _current_time: u64,
    _cid64: u64,
    _addr_peer: &SocketAddr,
    _addr_local: &SocketAddr,
    _packet_length: usize,
) {
    todo!()
}

/// Close the resource allocated for logs in the QUIC context — fans
/// out a `log_quic_close` to every installed backend.  C:
/// `picoquic_log_close_logs`.
pub fn picoquic_log_close_logs(_quic: &mut picoquic_quic_t) {
    todo!()
}

/// Log an event relating to a specific connection.  C:
/// `picoquic_log_app_message` (the `_v` variadic twin collapses
/// into the same Rust function — see also the forward-declaration
/// stub at [`crate::picoquic_log_app_message`],
/// which exists because `picoquic.h` re-declares the same symbol.
/// The translation of unified_log is the canonical home).
pub fn picoquic_log_app_message(_cnx: &mut picoquic_cnx_t, _args: core::fmt::Arguments<'_>) {
    todo!()
}

/// Log arrival or departure of an UDP datagram on a connection.
/// C: `picoquic_log_pdu`.
pub fn picoquic_log_pdu(
    _cnx: &mut picoquic_cnx_t,
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
/// `path_x` is `None` when the path lookup failed (C source passes
/// NULL).  C: `picoquic_log_packet`.
pub fn picoquic_log_packet(
    _cnx: &mut picoquic_cnx_t,
    _path_x: Option<&mut picoquic_path_t>,
    _receiving: bool,
    _current_time: u64,
    _ph: &picoquic_packet_header,
    _bytes: &[u8],
) {
    todo!()
}

/// Report that a packet was dropped due to some error.  `_raw_data`
/// is `UNUSED(raw_data)` in the C wrapper but kept in the
/// signature for source parity; Phase 3 may drop it.  C:
/// `picoquic_log_dropped_packet`.
pub fn picoquic_log_dropped_packet(
    _cnx: &mut picoquic_cnx_t,
    _path_x: Option<&mut picoquic_path_t>,
    _ph: &picoquic_packet_header,
    _packet_size: usize,
    _err: i32,
    _raw_data: &[u8],
    _current_time: u64,
) {
    todo!()
}

/// Report that a packet was buffered waiting for decryption.  C:
/// `picoquic_log_buffered_packet`.
pub fn picoquic_log_buffered_packet(
    _cnx: &mut picoquic_cnx_t,
    _path_x: &mut picoquic_path_t,
    _ptype: picoquic_packet_type_enum,
    _current_time: u64,
) {
    todo!()
}

/// Log that a packet was formatted, ready to be sent.  `bytes` is
/// the unencrypted packet (slice length subsumes the C `length`
/// parameter); `send_buffer` is the encrypted wire form.
/// `pn_length` is the length of the packet-number field within
/// `bytes`.  C: `picoquic_log_outgoing_packet`.
pub fn picoquic_log_outgoing_packet(
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
/// connection ID is unknown.  C: `picoquic_log_packet_lost`.
pub fn picoquic_log_packet_lost(
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

/// Log negotiated ALPN.  Empty `sni`/`alpn` slices match the C
/// callers that pass `(NULL, 0)`.  C: `picoquic_log_negotiated_alpn`.
pub fn picoquic_log_negotiated_alpn(
    _cnx: &mut picoquic_cnx_t,
    _is_local: bool,
    _sni: &[u8],
    _alpn: &[u8],
    _alpn_list: &[ptls_iovec_t],
) {
    todo!()
}

/// Log a transport-extension blob.  `is_local == true` when the
/// extension was formatted by the local peer; `false` when it was
/// received.  C: `picoquic_log_transport_extension`.
pub fn picoquic_log_transport_extension(
    _cnx: &mut picoquic_cnx_t,
    _is_local: bool,
    _params: &[u8],
) {
    todo!()
}

/// Log a TLS session ticket.  C: `picoquic_log_tls_ticket`.
pub fn picoquic_log_tls_ticket(_cnx: &mut picoquic_cnx_t, _ticket: &[u8]) {
    todo!()
}

/// Log the start of a connection.  C: `picoquic_log_new_connection`.
pub fn picoquic_log_new_connection(_cnx: &mut picoquic_cnx_t) {
    todo!()
}

/// Log the end of a connection.  C: `picoquic_log_close_connection`.
pub fn picoquic_log_close_connection(_cnx: &mut picoquic_cnx_t) {
    todo!()
}

/// Log a snapshot of congestion-control parameters across every
/// path on the connection.  Iterates `cnx->path[…]` internally and
/// dispatches per-path.  C: `picoquic_log_cc_dump`.
pub fn picoquic_log_cc_dump(_cnx: &mut picoquic_cnx_t, _current_time: u64) {
    todo!()
}
