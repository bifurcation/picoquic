//! Translation of `quic/unified_log.h`.
//!
//! Logging API.  The quic library can produce three complementary
//! logs per QUIC context — a textual trace (see [`crate::textlog`]),
//! a structured binary trace (see [`crate::binlog`]), and a qlog
//! (see [`crate::qlog`]) — but most applications only enable a
//! subset.  Each backend is described by a single [`Logger`] trait
//! object that the application optionally installs on the QUIC
//! context.  Per-connection dispatch happens through the [`Log`]
//! trait (implemented on [`Connection`]); per-context dispatch
//! happens through inherent methods on [`Quic`].  Either layer
//! fans the event out to whichever backends are registered.
//!
//! Phase 4 status: unified-log dispatch bodies are translated.
//!
//! Pointer-shape and translation policy notes for this module:
//!
//! * The C `picoquic_unified_logging_t` is a 16-slot vtable.  Per
//!   the Phase 1 rule "function pointers map to traits" we collapse
//!   the whole vtable into a single trait, [`Logger`],
//!   because every backend installs all sixteen entries together.
//!   QUIC contexts and connections hold three optional shared logger
//!   handles (`text_log_fns` / `bin_log_fns` / `qlog_fns`).  The
//!   shared handle is the safe Rust replacement for the C
//!   `cnx->quic` back-pointer used by the dispatch wrappers.
//! * `Quic*` / `Connection*` — every observed caller passes a non-NULL
//!   pointer and the logger callbacks may mutate internal state.
//!   Free functions keyed on `Quic*` become inherent methods on
//!   [`Quic`]; those keyed on `Connection*` become methods on the
//!   [`Log`] trait (implemented for [`Connection`]).
//! * `Path*` — non-NULL for every per-path log event except
//!   [`Logger::dropped_packet`], [`Logger::packet`], and
//!   [`Logger::packet_lost`], where the C source may pass NULL when
//!   the path lookup failed or a lost packet has no send path.  Those
//!   three get `Option<&mut Path>`; the rest get `&mut Path`.
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
use std::cell::RefCell;
use std::rc::Rc;

use crate::Instant;
use crate::internal::{Epoch, PacketHeader, PacketType, SUPPORTED_VERSIONS, frames_varint_decode};
use crate::{Connection, ConnectionId, PacketContext, Path, Quic};

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
        current_time: Instant,
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
    fn app_message(&mut self, connection: &mut Connection, args: core::fmt::Arguments<'_>);

    /// Emit a per-connection UDP-datagram arrival or departure
    /// record.  C: `picoquic_log_pdu_fn`.
    fn pdu(
        &mut self,
        connection: &mut Connection,
        receiving: bool,
        current_time: Instant,
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
        connection: &mut Connection,
        path_x: Option<&mut Path>,
        receiving: bool,
        current_time: Instant,
        ph: &PacketHeader,
        bytes: &[u8],
    );

    /// Emit a record that the packet was dropped due to some error.
    /// C: `picoquic_log_dropped_packet_fn`.
    fn dropped_packet(
        &mut self,
        connection: &mut Connection,
        path_x: Option<&mut Path>,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    );

    /// Emit a record that the packet was buffered waiting for
    /// decryption.  C: `picoquic_log_buffered_packet_fn`.
    fn buffered_packet(
        &mut self,
        connection: &mut Connection,
        path_x: &mut Path,
        ptype: PacketType,
        current_time: Instant,
    );

    /// Emit a record that a packet was formatted, ready to be sent.
    /// `bytes` is the unencrypted form (length carried in the
    /// slice); `send_buffer` is the encrypted-and-padded wire form.
    /// `pn_length` is the length of the packet-number field within
    /// `bytes`.  C: `picoquic_log_outgoing_packet_fn`.
    fn outgoing_packet(
        &mut self,
        connection: &mut Connection,
        path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: Instant,
    );

    /// Emit a packet-lost record.  `dcid` may be `None` when the
    /// remote connection ID is not known at the time of detection.
    /// C: `picoquic_log_packet_lost_fn`.
    fn packet_lost(
        &mut self,
        connection: &mut Connection,
        path_x: Option<&mut Path>,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: Instant,
    );

    /// Emit a negotiated-ALPN record.  Empty `sni` / `alpn` slices
    /// stand in for the C `(NULL, 0)` callers.  C:
    /// `picoquic_log_negotiated_alpn_fn`.
    fn negotiated_alpn(
        &mut self,
        connection: &mut Connection,
        is_local: bool,
        sni: &[u8],
        alpn: &[u8],
        alpn_list: &[&[u8]],
    );

    /// Emit a transport-extension record formatted by the local peer
    /// (`is_local == true`) or received from the remote peer.  C:
    /// `picoquic_log_transport_extension_fn`.
    fn transport_extension(&mut self, connection: &mut Connection, is_local: bool, params: &[u8]);

    /// Emit a TLS session-ticket record.  C:
    /// `picoquic_log_tls_ticket_fn`.
    fn tls_ticket(&mut self, connection: &mut Connection, ticket: &[u8]);

    /// Emit a connection-start record.  C:
    /// `picoquic_log_new_connection_fn`.
    fn new_connection(&mut self, connection: &mut Connection);

    /// Emit a connection-end record.  C:
    /// `picoquic_log_close_connection_fn`.
    fn close_connection(&mut self, connection: &mut Connection);

    /// Emit a snapshot of congestion-control parameters for one
    /// path.  C: `picoquic_log_cc_dump_fn` — the public dispatcher
    /// [`Log::cc_dump`] iterates the connection's paths and
    /// invokes this method per-path.
    fn cc_dump(&mut self, connection: &mut Connection, path_x: &mut Path, current_time: Instant);
}

pub(crate) type LoggerRef = Rc<RefCell<dyn Logger>>;

fn logger_ref(slot: &Option<LoggerRef>) -> Option<LoggerRef> {
    slot.as_ref().map(Rc::clone)
}

fn option_path<'a>(path_x: &'a mut Option<&mut Path>) -> Option<&'a mut Path> {
    path_x.as_mut().map(|path| &mut **path)
}

fn initial_remote_connection_id(connection: &Connection) -> ConnectionId {
    let path_unique_id = connection
        .paths
        .first()
        .map(|p| p.unique_path_id)
        .unwrap_or(0);
    let remote_index = connection
        .paths
        .first()
        .and_then(|p| p.tuples.first())
        .and_then(|t| t.remote_connection_id_index)
        .unwrap_or(0);

    connection
        .remote_connection_id_stashes
        .iter()
        .find(|stash| stash.unique_path_id == path_unique_id)
        .and_then(|stash| stash.connection_ids.get(remote_index))
        .or_else(|| {
            connection
                .remote_connection_id_stashes
                .first()
                .and_then(|stash| stash.connection_ids.first())
        })
        .map(|remote| remote.connection_id)
        .unwrap_or_default()
}

fn supported_version_index(version: u32) -> Option<i32> {
    SUPPORTED_VERSIONS
        .iter()
        .position(|v| *v as u32 == version)
        .map(|index| index as i32)
}

fn parse_outgoing_header(
    send_buffer: &[u8],
    outgoing_short_dcid_len: usize,
    connection_version_index: i32,
    do_grease_quic_bit: bool,
    has_loss_bit: bool,
) -> PacketHeader {
    let mut ph = PacketHeader::default();
    if send_buffer.is_empty() {
        ph.packet_type = PacketType::Error;
        return ph;
    }

    let flags = send_buffer[0];
    if flags & 0x80 == 0 {
        ph.packet_context = PacketContext::Application;
        ph.epoch = Epoch::OneRtt;
        ph.payload_length_value = 0;
        if send_buffer.len() < 1 + outgoing_short_dcid_len {
            ph.packet_type = PacketType::Error;
            ph.offset = send_buffer.len();
            ph.payload_length = 0;
            return ph;
        }
        let Some(cid) =
            ConnectionId::clone_from_slice(&send_buffer[1..1 + outgoing_short_dcid_len])
        else {
            ph.packet_type = PacketType::Error;
            ph.offset = send_buffer.len();
            ph.payload_length = 0;
            return ph;
        };
        ph.dest_connection_id = cid;
        ph.offset = 1 + outgoing_short_dcid_len;
        ph.packet_number_offset = ph.offset;
        ph.version_index = connection_version_index;
        ph.quic_bit_is_zero = (flags & 0x40) == 0;
        ph.packet_type = if !ph.quic_bit_is_zero || do_grease_quic_bit {
            PacketType::OneRttProtected
        } else {
            PacketType::Error
        };
        ph.has_spin_bit = true;
        ph.spin = (flags & 0x20) != 0;
        ph.key_phase = (flags & 0x04) != 0;
        ph.packet_number_mask = 0;
        ph.packet_number_truncated = 0;
        if has_loss_bit {
            ph.has_loss_bits = true;
            ph.loss_bit_l = (flags & 0x08) != 0;
            ph.loss_bit_q = (flags & 0x10) != 0;
        }
        ph.payload_length = if ph.packet_type == PacketType::Error {
            0
        } else {
            send_buffer.len() - ph.offset
        };
        return ph;
    }

    if send_buffer.len() < 5 {
        ph.packet_type = PacketType::Error;
        return ph;
    }

    let version = u32::from_be_bytes([
        send_buffer[1],
        send_buffer[2],
        send_buffer[3],
        send_buffer[4],
    ]);
    ph.version = version;
    let mut pos = 5usize;

    if version != 0 {
        let Some(version_index) = supported_version_index(version) else {
            ph.packet_type = PacketType::Error;
            ph.version_index = -1;
            return ph;
        };
        ph.version_index = version_index;
    }

    if pos >= send_buffer.len() {
        ph.packet_type = PacketType::Error;
        return ph;
    }
    let dcid_len = send_buffer[pos] as usize;
    pos += 1;
    if pos + dcid_len > send_buffer.len() {
        ph.packet_type = PacketType::Error;
        return ph;
    }
    let Some(cid) = ConnectionId::clone_from_slice(&send_buffer[pos..pos + dcid_len]) else {
        ph.packet_type = PacketType::Error;
        return ph;
    };
    ph.dest_connection_id = cid;
    pos += dcid_len;

    if pos >= send_buffer.len() {
        ph.packet_type = PacketType::Error;
        return ph;
    }
    let scid_len = send_buffer[pos] as usize;
    pos += 1;
    if pos + scid_len > send_buffer.len() {
        ph.packet_type = PacketType::Error;
        return ph;
    }
    let Some(cid) = ConnectionId::clone_from_slice(&send_buffer[pos..pos + scid_len]) else {
        ph.packet_type = PacketType::Error;
        return ph;
    };
    ph.src_connection_id = cid;
    pos += scid_len;

    if version == 0 {
        ph.packet_type = PacketType::VersionNegotiation;
        ph.offset = pos;
        ph.payload_length_value = send_buffer.len().saturating_sub(pos);
        ph.payload_length = ph.payload_length_value;
        return ph;
    }

    ph.packet_type = crate::internal::parse_long_packet_type(flags, ph.version_index);
    ph.quic_bit_is_zero = (flags & 0x40) == 0;
    ph.spin = false;
    ph.has_spin_bit = false;

    match ph.packet_type {
        PacketType::Initial => {
            ph.packet_context = PacketContext::Initial;
            ph.epoch = Epoch::Initial;
            let mut tok_len = 0u64;
            match frames_varint_decode(&send_buffer[pos..], &mut tok_len) {
                Some(rest) => {
                    pos = send_buffer.len() - rest.len();
                    let Some(token_len) = usize::try_from(tok_len).ok() else {
                        ph.packet_type = PacketType::Error;
                        ph.offset = send_buffer.len();
                        return ph;
                    };
                    let token_end = pos.saturating_add(token_len);
                    if token_end > send_buffer.len() {
                        ph.packet_type = PacketType::Error;
                        ph.offset = send_buffer.len();
                        return ph;
                    }
                    ph.token_bytes = send_buffer[pos..token_end].to_vec();
                    pos = token_end;
                }
                None => {
                    ph.packet_type = PacketType::Error;
                    return ph;
                }
            }
        }
        PacketType::ZeroRttProtected => {
            ph.packet_context = PacketContext::Application;
            ph.epoch = Epoch::ZeroRtt;
        }
        PacketType::Handshake => {
            ph.packet_context = PacketContext::Handshake;
            ph.epoch = Epoch::Handshake;
        }
        PacketType::Retry => {
            ph.packet_context = PacketContext::Initial;
            ph.epoch = Epoch::Initial;
        }
        _ => {
            ph.packet_type = PacketType::Error;
            return ph;
        }
    }

    if ph.packet_type == PacketType::Retry {
        ph.offset = pos;
        ph.packet_number_offset = pos;
        if send_buffer.len() > pos {
            ph.payload_length_value = send_buffer.len() - pos;
            ph.payload_length = ph.payload_length_value;
        } else {
            ph.packet_type = PacketType::Error;
        }
        return ph;
    }

    let mut payload_len = 0u64;
    let after_len = match frames_varint_decode(&send_buffer[pos..], &mut payload_len) {
        Some(r) => r,
        None => {
            ph.packet_type = PacketType::Error;
            return ph;
        }
    };
    let len_consumed = (send_buffer.len() - after_len.len()) - pos;
    pos += len_consumed;
    let Ok(payload_len) = usize::try_from(payload_len) else {
        ph.packet_type = PacketType::Error;
        return ph;
    };
    if after_len.len() < payload_len {
        ph.packet_type = PacketType::Error;
        ph.payload_length = send_buffer.len().saturating_sub(ph.offset);
        ph.payload_length_value = ph.payload_length;
        return ph;
    }
    ph.packet_number_offset = pos;
    ph.offset = pos;
    ph.payload_length_value = payload_len;
    ph.payload_length = payload_len;
    if ph.quic_bit_is_zero && !do_grease_quic_bit {
        ph.packet_type = PacketType::Error;
    }

    ph
}

pub(crate) fn prepare_outgoing_packet_header(
    connection: &Connection,
    send_buffer: &[u8],
    sequence_number: u64,
    pn_length: usize,
) -> PacketHeader {
    let outgoing_short_dcid_len = initial_remote_connection_id(connection).len();
    let mut ph = parse_outgoing_header(
        send_buffer,
        outgoing_short_dcid_len,
        connection.version_index,
        connection.local_parameters.do_grease_quic_bit,
        connection.is_loss_bit_enabled_outgoing,
    );

    ph.packet_number_full = sequence_number;
    ph.packet_number_truncated = sequence_number as u32;

    let mut checksum_length = 16usize;
    if ph.packet_type != PacketType::Retry {
        let epoch = match ph.packet_type {
            PacketType::OneRttProtected => Epoch::OneRtt,
            PacketType::ZeroRttProtected => Epoch::ZeroRtt,
            PacketType::Handshake => Epoch::Handshake,
            _ => Epoch::Initial,
        };
        if connection.crypto_context[epoch as usize]
            .aead_encrypt
            .is_some()
        {
            checksum_length = connection.get_checksum_length(epoch);
        }
        if ph.packet_number_offset != 0 {
            ph.offset = ph.packet_number_offset + pn_length;
            ph.payload_length = ph.payload_length.saturating_sub(pn_length);
        }
    }

    if ph.packet_type != PacketType::VersionNegotiation {
        ph.payload_length = ph.payload_length.saturating_sub(checksum_length);
    }

    ph
}

// ---------------------------------------------------------------------------
// Public dispatch layer.
//
// Each method below fans the event out to whichever of the three
// logger slots (`text_log_fns`, `bin_log_fns`, `qlog_fns`) the QUIC
// context has installed.

impl Quic {
    /// Log an application-supplied message that is not attached to a
    /// live connection.  The connection-id `cid` is used purely as a
    /// routing hint by the backends.  Callers form the formatted
    /// message at the call site with `format_args!`.
    ///
    /// C: `picoquic_log_context_free_app_message`.
    pub fn log_app_message(&mut self, cid: &ConnectionId, args: core::fmt::Arguments<'_>) {
        if self.f_log.is_some()
            && let Some(text) = logger_ref(&self.text_log_fns)
        {
            text.borrow_mut().quic_app_message(self, cid, args);
        }
    }

    /// Log arrival or departure of a UDP datagram for an unknown
    /// connection.  `cid64` is the would-be connection-id rendered
    /// as a 64-bit value.
    ///
    /// C: `picoquic_log_quic_pdu`.
    pub fn log_pdu(
        &mut self,
        receiving: bool,
        current_time: Instant,
        cid64: u64,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        packet_length: usize,
    ) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().quic_pdu(
                self,
                receiving,
                current_time,
                cid64,
                addr_peer,
                addr_local,
                packet_length,
            );
            self.text_log_fns = Some(text);
        }
    }

    /// Tear down every installed logging backend, releasing the
    /// resources each one holds (file handles, qlog buffers, …).
    /// Invoked at QUIC-context teardown.
    ///
    /// C: `picoquic_log_close_logs`.
    pub fn close_logs(&mut self) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().quic_close(self);
        }
        if let Some(bin) = logger_ref(&self.bin_log_fns) {
            bin.borrow_mut().quic_close(self);
        }
        if let Some(q) = logger_ref(&self.qlog_fns) {
            q.borrow_mut().quic_close(self);
        }
    }
}

/// Per-connection logging dispatcher.  Bringing this trait into
/// scope opts a caller into the unified-logging surface; each
/// method fans the event out to whichever [`Logger`] backends the
/// QUIC context has installed.  The C names (`picoquic_log_*`) drop
/// their `log_` prefix here — the trait name carries the verb.
pub trait Log {
    /// Append `args` to the connection's text log.  In C this exists
    /// in two flavours (`picoquic_log_app_message` and a `_v`/
    /// `va_list` twin); Rust folds them into one entry point — the
    /// `format_args!` macro at the call site is the variadic
    /// substitute.
    ///
    /// C: `picoquic_log_app_message`.
    fn app_message(&mut self, args: core::fmt::Arguments<'_>);

    /// Log arrival or departure of a UDP datagram on this
    /// connection.
    ///
    /// C: `picoquic_log_pdu`.
    fn pdu(
        &mut self,
        receiving: bool,
        current_time: Instant,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        packet_length: usize,
        unique_path_id: u64,
        ecn: u8,
    );

    /// Log a decrypted packet.  `receiving == true` for an arrival.
    /// `path_x` is `None` when the path lookup failed (C source
    /// passes NULL).
    ///
    /// C: `picoquic_log_packet`.
    fn packet(
        &mut self,
        path_x: Option<&mut Path>,
        receiving: bool,
        current_time: Instant,
        ph: &PacketHeader,
        bytes: &[u8],
    );

    /// Report that a packet was dropped due to some error.  The C
    /// wrapper takes a `raw_data` buffer that it then ignores
    /// (`UNUSED(raw_data)`); the parameter is dropped here.
    ///
    /// C: `picoquic_log_dropped_packet`.
    fn dropped_packet(
        &mut self,
        path_x: Option<&mut Path>,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    );

    /// Report that a packet was buffered waiting for decryption.
    ///
    /// C: `picoquic_log_buffered_packet`.
    fn buffered_packet(&mut self, path_x: &mut Path, ptype: PacketType, current_time: Instant);

    /// Log that a packet was formatted, ready to be sent.  `bytes`
    /// is the unencrypted packet (slice length subsumes the C
    /// `length` parameter); `send_buffer` is the encrypted wire
    /// form.  `pn_length` is the length of the packet-number field
    /// within `bytes`.
    ///
    /// C: `picoquic_log_outgoing_packet`.
    fn outgoing_packet(
        &mut self,
        path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: Instant,
    );

    /// Log a packet-lost event.  `path_x` is `None` when the lost
    /// packet has no send path; `dcid` is `None` when the remote
    /// connection ID is unknown.
    ///
    /// C: `picoquic_log_packet_lost`.
    fn packet_lost(
        &mut self,
        path_x: Option<&mut Path>,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: Instant,
    );

    /// Log negotiated SNI/ALPN.  Empty `sni`/`alpn` slices match the
    /// C callers that pass `(NULL, 0)`.
    ///
    /// C: `picoquic_log_negotiated_alpn`.
    fn negotiated_alpn(&mut self, is_local: bool, sni: &[u8], alpn: &[u8], alpn_list: &[&[u8]]);

    /// Log a transport-extension blob.  `is_local == true` when the
    /// extension was formatted by the local peer; `false` when it
    /// was received.
    ///
    /// C: `picoquic_log_transport_extension`.
    fn transport_extension(&mut self, is_local: bool, params: &[u8]);

    /// Log a TLS session ticket.
    ///
    /// C: `picoquic_log_tls_ticket`.
    fn tls_ticket(&mut self, ticket: &[u8]);

    /// Log the start of this connection.
    ///
    /// C: `picoquic_log_new_connection`.
    fn new_connection(&mut self);

    /// Log the end of this connection.
    ///
    /// C: `picoquic_log_close_connection`.
    fn close_connection(&mut self);

    /// Log a snapshot of congestion-control parameters across every
    /// path on the connection.  Iterates `connection->path[…]`
    /// internally and dispatches per-path through
    /// [`Logger::cc_dump`].
    ///
    /// C: `picoquic_log_cc_dump`.
    fn cc_dump(&mut self, current_time: Instant);
}

impl Log for Connection {
    fn app_message(&mut self, args: core::fmt::Arguments<'_>) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().app_message(self, args);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().app_message(self, args);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().app_message(self, args);
        }
    }

    fn pdu(
        &mut self,
        receiving: bool,
        current_time: Instant,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        packet_length: usize,
        unique_path_id: u64,
        ecn: u8,
    ) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().pdu(
                self,
                receiving,
                current_time,
                addr_peer,
                addr_local,
                packet_length,
                unique_path_id,
                ecn,
            );
        }

        if self.f_binlog.is_some() {
            if let Some(bin) = logger_ref(&self.bin_log_fns) {
                bin.borrow_mut().pdu(
                    self,
                    receiving,
                    current_time,
                    addr_peer,
                    addr_local,
                    packet_length,
                    unique_path_id,
                    ecn,
                );
            } else {
                let cid = self.initial_connection_id;
                if let Some(f_binlog) = self.f_binlog.as_mut() {
                    crate::binlog::pdu(
                        f_binlog,
                        &cid,
                        receiving,
                        current_time,
                        addr_peer,
                        addr_local,
                        packet_length,
                        unique_path_id,
                        ecn,
                    );
                }
            }
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().pdu(
                self,
                receiving,
                current_time,
                addr_peer,
                addr_local,
                packet_length,
                unique_path_id,
                ecn,
            );
        }
    }

    fn packet(
        &mut self,
        path_x: Option<&mut Path>,
        receiving: bool,
        current_time: Instant,
        ph: &PacketHeader,
        bytes: &[u8],
    ) {
        if !self.is_still_logging() {
            return;
        }

        let mut path_x = path_x;

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().packet(
                self,
                option_path(&mut path_x),
                receiving,
                current_time,
                ph,
                bytes,
            );
        }

        if self.f_binlog.is_some() {
            if let Some(bin) = logger_ref(&self.bin_log_fns) {
                bin.borrow_mut().packet(
                    self,
                    option_path(&mut path_x),
                    receiving,
                    current_time,
                    ph,
                    bytes,
                );
            } else {
                let cid = self.initial_connection_id;
                let path_id = if self.is_multipath_enabled {
                    path_x
                        .as_deref()
                        .map(|path| path.unique_path_id)
                        .unwrap_or(0)
                } else {
                    0
                };
                if let Some(f_binlog) = self.f_binlog.as_mut() {
                    crate::binlog::packet(
                        f_binlog,
                        &cid,
                        path_id,
                        receiving,
                        current_time,
                        ph,
                        bytes,
                    );
                }
            }
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().packet(
                self,
                option_path(&mut path_x),
                receiving,
                current_time,
                ph,
                bytes,
            );
        }
    }

    fn dropped_packet(
        &mut self,
        path_x: Option<&mut Path>,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    ) {
        if !self.is_still_logging() {
            return;
        }

        let mut path_x = path_x;

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().dropped_packet(
                self,
                option_path(&mut path_x),
                ph,
                packet_size,
                err,
                current_time,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().dropped_packet(
                self,
                option_path(&mut path_x),
                ph,
                packet_size,
                err,
                current_time,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().dropped_packet(
                self,
                option_path(&mut path_x),
                ph,
                packet_size,
                err,
                current_time,
            );
        }
    }

    fn buffered_packet(&mut self, path_x: &mut Path, ptype: PacketType, current_time: Instant) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .buffered_packet(self, path_x, ptype, current_time);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut()
                .buffered_packet(self, path_x, ptype, current_time);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut()
                .buffered_packet(self, path_x, ptype, current_time);
        }
    }

    fn outgoing_packet(
        &mut self,
        path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: Instant,
    ) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().outgoing_packet(
                self,
                path_x,
                bytes,
                sequence_number,
                pn_length,
                send_buffer,
                current_time,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().outgoing_packet(
                self,
                path_x,
                bytes,
                sequence_number,
                pn_length,
                send_buffer,
                current_time,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().outgoing_packet(
                self,
                path_x,
                bytes,
                sequence_number,
                pn_length,
                send_buffer,
                current_time,
            );
        }
    }

    fn packet_lost(
        &mut self,
        path_x: Option<&mut Path>,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: Instant,
    ) {
        if !self.is_still_logging() {
            return;
        }

        let mut path_x = path_x;

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().packet_lost(
                self,
                option_path(&mut path_x),
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().packet_lost(
                self,
                option_path(&mut path_x),
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().packet_lost(
                self,
                option_path(&mut path_x),
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }
    }

    fn negotiated_alpn(&mut self, is_local: bool, sni: &[u8], alpn: &[u8], alpn_list: &[&[u8]]) {
        if self.is_still_logging()
            && let Some(text) = logger_ref(&self.text_log_fns)
        {
            text.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }
    }

    fn transport_extension(&mut self, is_local: bool, params: &[u8]) {
        if self.is_still_logging()
            && let Some(text) = logger_ref(&self.text_log_fns)
        {
            text.borrow_mut()
                .transport_extension(self, is_local, params);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().transport_extension(self, is_local, params);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut()
                .transport_extension(self, is_local, params);
        }
    }

    fn tls_ticket(&mut self, ticket: &[u8]) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().tls_ticket(self, ticket);
        }

        if self.f_binlog.is_some() {
            if let Some(bin) = logger_ref(&self.bin_log_fns) {
                bin.borrow_mut().tls_ticket(self, ticket);
            } else if self.is_still_logging() {
                let cid = self.initial_connection_id;
                if let Some(f_binlog) = self.f_binlog.as_mut() {
                    crate::binlog::tls_ticket(f_binlog, cid, ticket);
                }
            }
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().tls_ticket(self, ticket);
        }
    }

    fn new_connection(&mut self) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().new_connection(self);
        }

        if let Some(bin) = logger_ref(&self.bin_log_fns) {
            bin.borrow_mut().new_connection(self);
        }

        if let Some(qlog) = logger_ref(&self.qlog_fns) {
            qlog.borrow_mut().new_connection(self);
        }
    }

    fn close_connection(&mut self) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().close_connection(self);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().close_connection(self);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().close_connection(self);
        }
    }

    fn cc_dump(&mut self, current_time: Instant) {
        let mut paths = core::mem::take(&mut self.paths);

        if let Some(mut memlog) = self.memlog_call_back.take() {
            if let Some(path0) = paths.first_mut() {
                memlog.callback(self, Some(path0), 0, current_time);
            }
            self.memlog_call_back = Some(memlog);
        }

        if self.is_still_logging() {
            for path_x in &mut paths {
                if !path_x.is_cc_data_updated {
                    continue;
                }

                if let Some(text) = logger_ref(&self.text_log_fns) {
                    text.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.f_binlog.is_some()
                    && let Some(bin) = logger_ref(&self.bin_log_fns)
                {
                    bin.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.qlog_ctx.is_some()
                    && let Some(qlog) = logger_ref(&self.qlog_fns)
                {
                    qlog.borrow_mut().cc_dump(self, path_x, current_time);
                }

                path_x.is_cc_data_updated = false;
            }
        }

        self.paths = paths;
    }
}
