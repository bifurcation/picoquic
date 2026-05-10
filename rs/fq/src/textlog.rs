//! Translation of `quic/logger.h`.
//!
//! Public surface of the quic *text* logger backend.  Once a text
//! log file is installed on a [`Quic`] context, the context's
//! text-log slot points at the textlog implementation of
//! [`crate::logger::Logger`] and every per-event log
//! call fans out through that vtable.  The install/teardown entry
//! points hang as inherent methods on [`Quic`]
//! ([`Quic::set_textlog`] / [`Quic::textlog_close`]).
//!
//! Pointer-shape and translation policy notes for this module:
//!
//! * `Quic*` — every observed caller (`config.c::963`,
//!   the recursive call in `logger.c::set_textlog`) passes
//!   a non-NULL, mutable context handle.  Maps to `&mut
//!   Quic`.
//! * `char const* textlog_file` — the C contract uses `NULL` as the
//!   "stop the text log" sentinel, so the parameter maps to
//!   `Option<&str>`.  `Some("-")` is the established
//!   redirect-to-stdout shortcut and is preserved as-is.
//! * The C return is `int` (0 on success, `-1` on file-open
//!   failure).  Mapped to `Result<(), Error>` per the project's
//!   error-handling convention.
//! * `picoquic_textlog_picotls_ticket` was the textlog backend's
//!   TLS-ticket pretty-printer.  Phase 4 reintroduces it as a
//!   private helper inside the textlog `Logger` impl; no public
//!   API needs it.
//! * `picoquic_log_fin_or_event_name` is declared in the C header
//!   but never defined or referenced — dropped from the Rust API.

use core::net::SocketAddr;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path as FsPath;
use std::rc::Rc;

use crate::Error;
use crate::Quic;
use crate::errors::InternalError;
use crate::frames::FrameType;
use crate::internal::{
    Connection, PacketHeader, PacketType, Path, parse_stream_header, skip_frame,
};
use crate::logger::{Logger, LoggerRef, prepare_outgoing_packet_header};
use crate::utils::frames_varint_decode;
use crate::{ConnectionId, Duration, Instant, State};

struct TextLogger;

fn write_txtlog_message(out: &mut dyn Write, cid: &ConnectionId, args: core::fmt::Arguments<'_>) {
    let cid64 = cid.val64();
    if cid64 != 0 {
        let _ = write!(out, "{cid64:016x}: ");
    }
    let _ = out.write_fmt(args);
    let _ = out.write_all(b"\n");
}

fn write_quic_txtlog_message(quic: &mut Quic, cid: &ConnectionId, args: core::fmt::Arguments<'_>) {
    if let Some(out) = quic.f_log.as_deref_mut() {
        write_txtlog_message(out, cid, args);
    }
}

fn write_connection_txtlog_message(connection: &mut Connection, args: core::fmt::Arguments<'_>) {
    if connection.quic_ptr.is_null() {
        return;
    }

    let cid = connection.initial_connection_id;
    // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal` and
    // remains valid while the connection is live. This only mutably borrows
    // the context-level text-log sink, which is disjoint from the connection
    // arena entry borrowed by the caller.
    let f_log = unsafe { &mut (*connection.quic_ptr).f_log };
    if let Some(out) = f_log.as_deref_mut() {
        write_txtlog_message(out, &cid, args);
    }
}

fn write_textlog_prefix_initial_cid64(out: &mut dyn Write, cnx_id64: u64) {
    if cnx_id64 != 0 {
        let _ = write!(out, "{cnx_id64:016x}: ");
    }
}

fn write_textlog_connection_id(out: &mut dyn Write, cid: &ConnectionId) {
    let _ = out.write_all(b"<");
    for byte in cid.as_bytes() {
        let _ = write!(out, "{:02x}", *byte);
    }
    let _ = out.write_all(b">");
}

fn textlog_ptype_name(ptype: PacketType) -> &'static str {
    match ptype {
        PacketType::Error => "error",
        PacketType::VersionNegotiation => "version negotiation",
        PacketType::Initial => "initial",
        PacketType::Retry => "retry",
        PacketType::Handshake => "handshake",
        PacketType::ZeroRttProtected => "0rtt protected",
        PacketType::OneRttProtected => "1rtt protected",
        PacketType::TypeMax => "unknown",
    }
}

fn write_textlog_packet_header(
    out: &mut dyn Write,
    log_cnxid64: u64,
    ph: &PacketHeader,
    receiving: bool,
) {
    write_textlog_prefix_initial_cid64(out, log_cnxid64);
    let _ = write!(
        out,
        "{} packet type: {} ({}), ",
        if receiving { "Receiving" } else { "Sending" },
        ph.packet_type as u8,
        textlog_ptype_name(ph.packet_type)
    );
    let _ = write!(
        out,
        "S{}, Q{},",
        u8::from(ph.spin),
        u8::from(!ph.quic_bit_is_zero)
    );

    match ph.packet_type {
        PacketType::OneRttProtected => {
            let _ = out.write_all(b"\n");
            write_textlog_prefix_initial_cid64(out, log_cnxid64);
            let _ = out.write_all(b"    ");
            write_textlog_connection_id(out, &ph.dest_connection_id);
            let _ = write!(
                out,
                ", Seq: {} ({}), Phi: {},",
                ph.packet_number_truncated,
                ph.packet_number_full,
                u8::from(ph.key_phase)
            );
            if ph.has_loss_bits {
                let _ = write!(
                    out,
                    " Q({}), L({}),",
                    u8::from(ph.loss_bit_q),
                    u8::from(ph.loss_bit_l)
                );
            }
            let _ = out.write_all(b"\n");
        }
        PacketType::VersionNegotiation => {
            let _ = out.write_all(b"\n");
            write_textlog_prefix_initial_cid64(out, log_cnxid64);
            let _ = out.write_all(b"    ");
            write_textlog_connection_id(out, &ph.dest_connection_id);
            let _ = out.write_all(b", ");
            write_textlog_connection_id(out, &ph.src_connection_id);
            let _ = out.write_all(b"\n");
        }
        _ => {
            let _ = write!(out, " Version {:x},", ph.version);
            let _ = out.write_all(b"\n");
            write_textlog_prefix_initial_cid64(out, log_cnxid64);
            let _ = out.write_all(b"    ");
            write_textlog_connection_id(out, &ph.dest_connection_id);
            let _ = out.write_all(b", ");
            write_textlog_connection_id(out, &ph.src_connection_id);
            let _ = writeln!(
                out,
                ", Seq: {}, pl: {}",
                ph.packet_number_truncated, ph.payload_length_value
            );
            if ph.packet_type == PacketType::Initial {
                write_textlog_prefix_initial_cid64(out, log_cnxid64);
                let _ = write!(out, "    Token length: {}", ph.token_bytes.len());
                if !ph.token_bytes.is_empty() {
                    let printed_length = ph.token_bytes.len().min(16);
                    let _ = out.write_all(b", Token: ");
                    for byte in &ph.token_bytes[..printed_length] {
                        let _ = write!(out, "{:02x}", *byte);
                    }
                    if printed_length < ph.token_bytes.len() {
                        let _ = out.write_all(b"...");
                    }
                }
                let _ = out.write_all(b"\n");
            }
        }
    }
}

fn textlog_frame_name(frame_id: u64) -> &'static str {
    FrameType::name(frame_id).unwrap_or("unknown")
}

fn write_textlog_hex_prefix(out: &mut dyn Write, bytes: &[u8], max_len: usize) {
    for byte in bytes.iter().take(max_len) {
        let _ = write!(out, "{:02x}", *byte);
    }
    if bytes.len() > max_len {
        let _ = out.write_all(b"...");
    }
}

fn write_textlog_negotiation_packet(
    out: &mut dyn Write,
    log_cnxid64: u64,
    bytes: &[u8],
    ph: &PacketHeader,
) {
    write_textlog_prefix_initial_cid64(out, log_cnxid64);
    let _ = out.write_all(b"    versions: ");

    let mut byte_index = ph.offset.min(bytes.len());
    while byte_index + 4 <= bytes.len() {
        let vn = u32::from_be_bytes([
            bytes[byte_index],
            bytes[byte_index + 1],
            bytes[byte_index + 2],
            bytes[byte_index + 3],
        ]);
        byte_index += 4;
        let _ = write!(out, "{vn:02x}, ");
    }
    let _ = out.write_all(b"\n");
}

fn write_textlog_retry_packet(
    out: &mut dyn Write,
    log_cnxid64: u64,
    bytes: &[u8],
    ph: &PacketHeader,
) {
    let byte_index = ph.offset.min(bytes.len());
    let payload_length = ph.payload_length;
    let checksum_length = 16usize;

    if checksum_length >= payload_length {
        write_textlog_prefix_initial_cid64(out, log_cnxid64);
        let _ = writeln!(
            out,
            "    packet too short, checksum: {checksum_length} bytes, only {payload_length} bytes available."
        );
        return;
    }

    let token_length = payload_length - checksum_length;
    write_textlog_prefix_initial_cid64(out, log_cnxid64);
    let _ = writeln!(
        out,
        "    Token length: {token_length}, Checksum length: {checksum_length}"
    );

    if token_length > 0 {
        let available = bytes.len().saturating_sub(byte_index);
        let printed_length = token_length.min(16).min(available);
        write_textlog_prefix_initial_cid64(out, log_cnxid64);
        let _ = out.write_all(b"    Token: ");
        write_textlog_hex_prefix(
            out,
            &bytes[byte_index..byte_index + printed_length],
            printed_length,
        );
        if printed_length < token_length {
            let _ = out.write_all(b"...");
        }
        let _ = out.write_all(b"\n");
    }
}

fn write_textlog_incorrect_frame_id(out: &mut dyn Write, bytes: &[u8]) {
    let id_length = bytes.len().min(8);
    let _ = out.write_all(b"    Incorrect frame id: ");
    write_textlog_hex_prefix(out, &bytes[..id_length], id_length);
    if bytes.len() > id_length {
        let _ = out.write_all(b"...");
    }
    let _ = out.write_all(b"\n");
}

fn write_textlog_malformed_frame(out: &mut dyn Write, frame_id: u64, bytes: &[u8]) {
    let _ = write!(
        out,
        "    Malformed {} frame: ",
        textlog_frame_name(frame_id)
    );
    write_textlog_hex_prefix(out, bytes, 8);
    let _ = out.write_all(b"\n");
}

fn write_textlog_unknown_frame(out: &mut dyn Write, frame_id: u64, bytes: &[u8]) {
    let _ = write!(out, "    Unknown frame, type: {frame_id} (0x");
    let printed_length = bytes.len().min(8);
    for byte in &bytes[..printed_length] {
        let _ = write!(out, "{:02x}", *byte);
    }
    if bytes.len() > printed_length {
        let _ = writeln!(out, "... + {} bytes)", bytes.len() - printed_length);
    } else {
        let _ = out.write_all(b")\n");
    }
}

fn write_textlog_stream_frame(out: &mut dyn Write, bytes: &[u8]) -> usize {
    let mut stream_id = 0;
    let mut offset = 0;
    let mut data_length = 0;
    let mut fin = 0;
    let mut header_len = 0;

    if parse_stream_header(
        bytes,
        bytes.len(),
        &mut stream_id,
        &mut offset,
        &mut data_length,
        &mut fin,
        &mut header_len,
    ) != 0
    {
        write_textlog_malformed_frame(out, bytes.first().copied().unwrap_or(0) as u64, bytes);
        return bytes.len();
    }

    let data_start = header_len.min(bytes.len());
    let data_end = data_start.saturating_add(data_length).min(bytes.len());
    let data = &bytes[data_start..data_end];
    let _ = write!(
        out,
        "    {} {stream_id}, offset {offset}, length {data_length}, fin = {fin}: ",
        textlog_frame_name(bytes[0] as u64)
    );
    write_textlog_hex_prefix(out, data, 8);
    let _ = out.write_all(b"\n");

    data_start.saturating_add(data_length).min(bytes.len())
}

fn write_textlog_frames(out: &mut dyn Write, log_cnxid64: u64, bytes: &[u8]) {
    let mut byte_index = 0usize;
    while byte_index < bytes.len() {
        let frame_bytes = &bytes[byte_index..];
        let Some((rest, frame_id)) = frames_varint_decode(frame_bytes) else {
            write_textlog_prefix_initial_cid64(out, log_cnxid64);
            write_textlog_incorrect_frame_id(out, frame_bytes);
            break;
        };
        let frame_id_len = frame_bytes.len() - rest.len();

        write_textlog_prefix_initial_cid64(out, log_cnxid64);
        if frame_id < 64 && frame_id_len != 1 {
            write_textlog_incorrect_frame_id(out, frame_bytes);
            break;
        }

        if ((FrameType::StreamRangeMin as u64)..=(FrameType::StreamRangeMax as u64))
            .contains(&frame_id)
        {
            byte_index += write_textlog_stream_frame(out, frame_bytes);
            continue;
        }

        match frame_id {
            x if x == FrameType::Padding as u64 || x == FrameType::Ping as u64 => {
                let mut nb = 0usize;
                while byte_index + nb < bytes.len() && bytes[byte_index + nb] == frame_id as u8 {
                    nb += 1;
                }
                let _ = writeln!(out, "    {}, {nb} bytes", textlog_frame_name(frame_id));
                byte_index += nb.max(1);
            }
            _ => {
                if FrameType::name(frame_id).is_none() {
                    write_textlog_unknown_frame(out, frame_id, frame_bytes);
                    break;
                }

                let mut consumed = 0usize;
                let mut pure_ack = 0;
                if skip_frame(frame_bytes, frame_bytes.len(), &mut consumed, &mut pure_ack) == 0
                    && consumed > 0
                {
                    let _ = writeln!(out, "    {}", textlog_frame_name(frame_id));
                    byte_index += consumed;
                } else {
                    write_textlog_malformed_frame(out, frame_id, frame_bytes);
                    break;
                }
            }
        }
    }
}

fn write_textlog_decrypted_segment(
    out: &mut dyn Write,
    log_cnxid64: u64,
    ph: &PacketHeader,
    receiving: bool,
    bytes: &[u8],
    err: i32,
) {
    write_textlog_packet_header(out, log_cnxid64, ph, receiving);

    if err != 0 {
        write_textlog_prefix_initial_cid64(out, log_cnxid64);
        if err == InternalError::StatelessReset as i32 {
            let _ = out.write_all(b"   Stateless reset.\n");
        } else {
            let _ = writeln!(out, "   Header or encryption error: {err:x}.");
        }
    } else if ph.packet_type == PacketType::VersionNegotiation {
        write_textlog_negotiation_packet(out, log_cnxid64, bytes, ph);
    } else if ph.packet_type == PacketType::Retry {
        write_textlog_retry_packet(out, log_cnxid64, bytes, ph);
    } else if ph.packet_type != PacketType::Error {
        write_textlog_prefix_initial_cid64(out, log_cnxid64);
        let _ = writeln!(
            out,
            "    {} {} bytes",
            if receiving { "Decrypted" } else { "Prepared" },
            ph.payload_length
        );
        if let Some(payload) = bytes.get(ph.offset..) {
            let frame_length = payload.len().min(ph.payload_length);
            write_textlog_frames(out, log_cnxid64, &payload[..frame_length]);
        }
    }

    let _ = out.write_all(b"\n");
}

fn write_textlog_decrypted_segment_error(
    out: &mut dyn Write,
    log_cnxid64: u64,
    ph: &PacketHeader,
    err: i32,
) {
    write_textlog_decrypted_segment(out, log_cnxid64, ph, true, &[], err);
}

fn write_textlog_dropped_packet(
    out: &mut dyn Write,
    log_cnxid64: u64,
    ph: &PacketHeader,
    packet_size: usize,
    err: i32,
) {
    if err == InternalError::PaddingPacket as i32 {
        write_textlog_prefix_initial_cid64(out, log_cnxid64);
        let _ = writeln!(out, "Dropped padding packet, size: {packet_size}.");
        let _ = out.write_all(b"\n");
    } else {
        write_textlog_decrypted_segment_error(out, log_cnxid64, ph, err);
    }
}

fn write_textlog_time(
    out: &mut dyn Write,
    start_time: Instant,
    current_time: Instant,
    label1: &str,
    label2: &str,
) {
    let delta_t = current_time.ticks().saturating_sub(start_time.ticks());
    let time_sec = delta_t / 1_000_000;
    let time_usec = delta_t % 1_000_000;

    let _ = write!(out, "{label1}{time_sec}.{time_usec:06}{label2}");
}

fn write_textlog_buffered_packet(
    out: &mut dyn Write,
    log_cnxid64: u64,
    start_time: Instant,
    current_time: Instant,
    ptype: PacketType,
) {
    write_textlog_prefix_initial_cid64(out, log_cnxid64);
    write_textlog_time(out, start_time, current_time, "T= ", ", ");
    let _ = writeln!(
        out,
        "Keys unavailable, buffered packet type {}.",
        ptype as u8
    );
}

#[allow(clippy::too_many_arguments)]
fn write_textlog_packet_lost(
    out: &mut dyn Write,
    log_cnxid64: u64,
    start_time: Instant,
    current_time: Instant,
    ptype: PacketType,
    unique_path_id: u64,
    sequence_number: u64,
    trigger: &str,
    dcid: Option<&ConnectionId>,
    packet_size: usize,
) {
    write_textlog_prefix_initial_cid64(out, log_cnxid64);
    write_textlog_time(out, start_time, current_time, "T= ", ", ");
    let _ = write!(
        out,
        "Lost packet type {}, path {}, number {}, size {}",
        ptype as u8, unique_path_id, sequence_number, packet_size
    );
    if let Some(dcid) = dcid {
        let _ = out.write_all(b", DCID ");
        write_textlog_connection_id(out, dcid);
    }
    let _ = writeln!(out, ", reason: {trigger}");
}

#[derive(Copy, Clone)]
struct TextlogCongestionState {
    unique_path_id: u64,
    cwin: u64,
    bytes_in_transit: u64,
    nb_retransmission_total: u64,
    rtt_min: Duration,
    smoothed_rtt: Duration,
    rtt_variant: Duration,
    max_ack_delay: Duration,
    connection_state: State,
}

fn write_textlog_congestion_state(
    out: &mut dyn Write,
    log_cnxid64: u64,
    start_time: Instant,
    current_time: Instant,
    cc_state: TextlogCongestionState,
) {
    write_textlog_prefix_initial_cid64(out, log_cnxid64);
    write_textlog_time(out, start_time, current_time, "T= ", ", ");
    let _ = writeln!(
        out,
        "path_id: {},cwin: {},flight: {},nb_ret: {},rtt_min: {},rtt: {},rtt_var: {},max_ack_delay: {},state: {}",
        cc_state.unique_path_id,
        cc_state.cwin as i32,
        cc_state.bytes_in_transit as i32,
        cc_state.nb_retransmission_total as i32,
        cc_state.rtt_min.ticks() as i32,
        cc_state.smoothed_rtt.ticks() as i32,
        cc_state.rtt_variant.ticks() as i32,
        cc_state.max_ack_delay.ticks() as i32,
        cc_state.connection_state as i32,
    );
}

fn write_textlog_alpn_target(out: &mut dyn Write, alpn: &[u8]) {
    if alpn.len() < 64 {
        let end = alpn
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(alpn.len());
        let _ = out.write_all(&alpn[..end]);
        return;
    }

    let prefix = &alpn[..60];
    if let Some(end) = prefix.iter().position(|byte| *byte == 0) {
        let _ = out.write_all(&prefix[..end]);
    } else {
        let _ = out.write_all(prefix);
        let _ = out.write_all(b"...");
    }
}

fn write_textlog_negotiated_alpn(
    out: &mut dyn Write,
    log_cnxid64: u64,
    received: bool,
    alpn_list: &[&[u8]],
) {
    write_textlog_prefix_initial_cid64(out, log_cnxid64);
    let _ = write!(
        out,
        "{} ALPN list ({}): ",
        if received { "Received" } else { "Sending" },
        alpn_list.len()
    );

    for (index, alpn) in alpn_list.iter().enumerate() {
        if index != 0 {
            let _ = out.write_all(b", ");
        }
        write_textlog_alpn_target(out, alpn);
    }

    let _ = out.write_all(b"\n");
}

impl Logger for TextLogger {
    fn quic_app_message(
        &mut self,
        quic: &mut Quic,
        cid: &ConnectionId,
        args: core::fmt::Arguments<'_>,
    ) {
        write_quic_txtlog_message(quic, cid, args);
    }

    fn quic_pdu(
        &mut self,
        _quic: &mut Quic,
        _receiving: bool,
        _current_time: Instant,
        _cid64: u64,
        _addr_peer: &SocketAddr,
        _addr_local: &SocketAddr,
        _packet_length: usize,
    ) {
    }

    fn quic_close(&mut self, quic: &mut Quic) {
        quic.textlog_close();
    }

    fn app_message(&mut self, connection: &mut Connection, args: core::fmt::Arguments<'_>) {
        write_connection_txtlog_message(connection, args);
    }

    fn pdu(
        &mut self,
        _connection: &mut Connection,
        _receiving: bool,
        _current_time: Instant,
        _addr_peer: &SocketAddr,
        _addr_local: &SocketAddr,
        _packet_length: usize,
        _unique_path_id: u64,
        _ecn: u8,
    ) {
    }

    fn packet(
        &mut self,
        connection: &mut Connection,
        _path_x: Option<&mut Path>,
        receiving: bool,
        _current_time: Instant,
        ph: &PacketHeader,
        bytes: &[u8],
    ) {
        if connection.quic_ptr.is_null() || !connection.is_still_logging() {
            return;
        }

        let log_cnxid64 = connection.initial_connection_id.val64();
        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal` and
        // remains valid while the connection is live. This only mutably borrows
        // the context-level text-log sink, which is disjoint from the borrowed
        // connection and path state used by the callback.
        let f_log = unsafe { &mut (*connection.quic_ptr).f_log };
        if let Some(out) = f_log.as_deref_mut() {
            write_textlog_decrypted_segment(out, log_cnxid64, ph, receiving, bytes, 0);
        }
    }

    fn dropped_packet(
        &mut self,
        connection: &mut Connection,
        _path_x: Option<&mut Path>,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        _current_time: Instant,
    ) {
        if connection.quic_ptr.is_null() {
            return;
        }

        let log_cnxid64 = connection.initial_connection_id.val64();
        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal` and
        // remains valid while the connection is live. This only mutably borrows
        // the context-level text-log sink, which is disjoint from the borrowed
        // connection and path state used by the callback.
        let f_log = unsafe { &mut (*connection.quic_ptr).f_log };
        if let Some(out) = f_log.as_deref_mut() {
            write_textlog_dropped_packet(out, log_cnxid64, ph, packet_size, err);
        }
    }

    fn buffered_packet(
        &mut self,
        connection: &mut Connection,
        _path_x: &mut Path,
        ptype: PacketType,
        current_time: Instant,
    ) {
        if connection.quic_ptr.is_null() {
            return;
        }

        let log_cnxid64 = connection.initial_connection_id.val64();
        let start_time = connection.start_time;
        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal` and
        // remains valid while the connection is live. This only mutably borrows
        // the context-level text-log sink, which is disjoint from the borrowed
        // connection and path state used by the callback.
        let f_log = unsafe { &mut (*connection.quic_ptr).f_log };
        if let Some(out) = f_log.as_deref_mut() {
            write_textlog_buffered_packet(out, log_cnxid64, start_time, current_time, ptype);
        }
    }

    fn outgoing_packet(
        &mut self,
        connection: &mut Connection,
        _path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        _current_time: Instant,
    ) {
        if connection.quic_ptr.is_null() || !connection.is_still_logging() {
            return;
        }

        let log_cnxid64 = connection.initial_connection_id.val64();
        let ph =
            prepare_outgoing_packet_header(connection, send_buffer, sequence_number, pn_length);
        let err = if ph.packet_type == PacketType::Error {
            InternalError::PacketHeaderParsing as i32
        } else {
            0
        };
        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal` and
        // remains valid while the connection is live. This only mutably borrows
        // the context-level text-log sink, which is disjoint from the borrowed
        // connection and path state used by the callback.
        let f_log = unsafe { &mut (*connection.quic_ptr).f_log };
        if let Some(out) = f_log.as_deref_mut() {
            write_textlog_decrypted_segment(out, log_cnxid64, &ph, false, bytes, err);
        }
    }

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
    ) {
        if connection.quic_ptr.is_null() || !connection.is_still_logging() {
            return;
        }

        let log_cnxid64 = connection.initial_connection_id.val64();
        let start_time = connection.start_time;
        let unique_path_id = path_x
            .as_deref()
            .map(|path| path.unique_path_id)
            .unwrap_or(0);
        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal` and
        // remains valid while the connection is live. This only mutably borrows
        // the context-level text-log sink, which is disjoint from the borrowed
        // connection and path state used by the callback.
        let f_log = unsafe { &mut (*connection.quic_ptr).f_log };
        if let Some(out) = f_log.as_deref_mut() {
            write_textlog_packet_lost(
                out,
                log_cnxid64,
                start_time,
                current_time,
                ptype,
                unique_path_id,
                sequence_number,
                trigger,
                dcid,
                packet_size,
            );
        }
    }

    fn negotiated_alpn(
        &mut self,
        connection: &mut Connection,
        is_local: bool,
        _sni: &[u8],
        _alpn: &[u8],
        alpn_list: &[&[u8]],
    ) {
        if connection.quic_ptr.is_null() || !connection.is_still_logging() {
            return;
        }

        let log_cnxid64 = connection.initial_connection_id.val64();
        let received = !is_local;
        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal` and
        // remains valid while the connection is live. This only mutably borrows
        // the context-level text-log sink, which is disjoint from the borrowed
        // connection state used to format the ALPN event.
        let f_log = unsafe { &mut (*connection.quic_ptr).f_log };
        if let Some(out) = f_log.as_deref_mut() {
            write_textlog_negotiated_alpn(out, log_cnxid64, received, alpn_list);
        }
    }

    fn transport_extension(
        &mut self,
        _connection: &mut Connection,
        _is_local: bool,
        _params: &[u8],
    ) {
    }

    fn tls_ticket(&mut self, _connection: &mut Connection, _ticket: &[u8]) {}

    fn new_connection(&mut self, _connection: &mut Connection) {}

    // C: textlog_close_connection
    fn close_connection(&mut self, _connection: &mut Connection) {}

    /// C: `picoquic/logger.c:textlog_cc_dump`.
    fn cc_dump(&mut self, connection: &mut Connection, path_x: &mut Path, current_time: Instant) {
        if connection.quic_ptr.is_null() {
            return;
        }

        let log_cnxid64 = connection.initial_connection_id.val64();
        let start_time = connection.start_time;
        let cc_state = TextlogCongestionState {
            unique_path_id: path_x.unique_path_id,
            cwin: path_x.cwin,
            bytes_in_transit: path_x.bytes_in_transit,
            nb_retransmission_total: connection.nb_retransmission_total,
            rtt_min: path_x.rtt_min,
            smoothed_rtt: path_x.smoothed_rtt,
            rtt_variant: path_x.rtt_variant,
            max_ack_delay: path_x.max_ack_delay,
            connection_state: connection.connection_state,
        };
        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal` and
        // remains valid while the connection is live. This only mutably borrows
        // the context-level text-log sink, which is disjoint from the borrowed
        // connection and supplied path state used to format the CC snapshot.
        let f_log = unsafe { &mut (*connection.quic_ptr).f_log };
        if let Some(out) = f_log.as_deref_mut() {
            write_textlog_congestion_state(out, log_cnxid64, start_time, current_time, cc_state);
        }
    }
}

impl Quic {
    fn install_text_logger(&mut self) {
        let logger = self
            .text_log_fns
            .get_or_insert_with(|| {
                let logger: LoggerRef = Rc::new(RefCell::new(TextLogger));
                logger
            })
            .clone();

        for connection in self.connections.iter_mut() {
            connection.text_log_fns = Some(logger.clone());
        }
    }

    fn clear_text_logger(&mut self) {
        self.text_log_fns = None;
        for connection in self.connections.iter_mut() {
            connection.text_log_fns = None;
        }
    }

    /// Set the text log file and start tracing into it.
    ///
    /// Pass `None` to stop the text log (the C "set to NULL"
    /// sentinel).  `Some("-")` redirects output to stdout without
    /// taking ownership of the handle, matching the C body in
    /// `logger.c::set_textlog`.
    ///
    /// C: `int picoquic_set_textlog(picoquic_quic_t*, char const*)`.
    pub fn set_textlog(
        &mut self,
        textlog_file: Option<&(impl AsRef<FsPath> + ?Sized)>,
    ) -> Result<(), Error> {
        self.textlog_close();

        let Some(path) = textlog_file else {
            return Ok(());
        };
        let path = path.as_ref();

        if path == FsPath::new("-") {
            self.f_log = Some(Box::new(std::io::stdout()));
            self.should_close_log = false;
        } else {
            match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
            {
                Ok(f) => {
                    self.f_log = Some(Box::new(f));
                    self.should_close_log = true;
                }
                Err(_) => return Err(Error::NoSuchFile),
            }
        }

        self.install_text_logger();
        Ok(())
    }

    /// Close the text log, e.g., when closing the QUIC context.
    /// Safe to call when no text log is currently installed — the C
    /// body guards on `quic->F_log != NULL` and on the
    /// `quic->should_close_log` flag (so an external `stdout`
    /// handle is left alone).
    ///
    /// C: `void picoquic_textlog_close(picoquic_quic_t*)`.
    pub fn textlog_close(&mut self) {
        self.f_log = None;
        self.should_close_log = false;
        self.clear_text_logger();
    }
}

impl crate::internal::Connection {
    /// Dispatch a pre-formatted application message to every installed
    /// log backend (text, binlog, qlog) on this connection.  The C
    /// `va_list` parameter collapses to [`core::fmt::Arguments`] —
    /// call sites use `format_args!` as the Rust variadic substitute.
    ///
    /// C: `picoquic/unified_log.c:picoquic_log_app_message_v`
    pub fn log_app_message_v(&mut self, args: core::fmt::Arguments<'_>) {
        crate::logger::Log::app_message(self, args);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn txtlog_message_prefixes_non_zero_cid() {
        let cid = ConnectionId::clone_from_slice(&[0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12])
            .expect("cid");
        let mut out = Vec::new();

        write_txtlog_message(&mut out, &cid, format_args!("This is an app message test."));

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            "0b0c0d0e0f101112: This is an app message test.\n"
        );
    }

    #[test]
    fn dropped_padding_packet_uses_special_textlog_line() {
        let ph = PacketHeader::default();
        let mut out = Vec::new();

        write_textlog_dropped_packet(
            &mut out,
            0x0b0c_0d0e_0f10_1112,
            &ph,
            37,
            InternalError::PaddingPacket as i32,
        );

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            "0b0c0d0e0f101112: Dropped padding packet, size: 37.\n\n"
        );
    }

    #[test]
    fn dropped_non_padding_packet_logs_decrypted_segment_error() {
        let mut ph = PacketHeader {
            dest_connection_id: ConnectionId::clone_from_slice(&[1, 2, 3, 4]).expect("dest cid"),
            packet_type: PacketType::OneRttProtected,
            packet_number_truncated: 18,
            packet_number_full: 4660,
            spin: true,
            key_phase: true,
            has_loss_bits: true,
            loss_bit_q: true,
            ..PacketHeader::default()
        };
        ph.quic_bit_is_zero = false;

        let mut out = Vec::new();
        write_textlog_dropped_packet(
            &mut out,
            0x0b0c_0d0e_0f10_1112,
            &ph,
            1200,
            InternalError::AeadCheck as i32,
        );

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            concat!(
                "0b0c0d0e0f101112: Receiving packet type: 6 (1rtt protected), S1, Q1,\n",
                "0b0c0d0e0f101112:     <01020304>, Seq: 18 (4660), Phi: 1, Q(1), L(0),\n",
                "0b0c0d0e0f101112:    Header or encryption error: 403.\n",
                "\n"
            )
        );
    }

    #[test]
    fn outgoing_decrypted_segment_logs_prepared_frames() {
        let mut ph = PacketHeader {
            dest_connection_id: ConnectionId::clone_from_slice(&[1, 2, 3, 4]).expect("dest cid"),
            packet_type: PacketType::OneRttProtected,
            packet_number_truncated: 7,
            packet_number_full: 7,
            payload_length: 3,
            ..PacketHeader::default()
        };
        ph.quic_bit_is_zero = false;
        let mut out = Vec::new();

        write_textlog_decrypted_segment(&mut out, 0x0b0c_0d0e_0f10_1112, &ph, false, &[0, 0, 1], 0);

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            concat!(
                "0b0c0d0e0f101112: Sending packet type: 6 (1rtt protected), S0, Q1,\n",
                "0b0c0d0e0f101112:     <01020304>, Seq: 7 (7), Phi: 0,\n",
                "0b0c0d0e0f101112:     Prepared 3 bytes\n",
                "0b0c0d0e0f101112:     padding, 2 bytes\n",
                "0b0c0d0e0f101112:     ping, 1 bytes\n",
                "\n"
            )
        );
    }

    #[test]
    fn buffered_packet_logs_prefix_time_and_packet_type() {
        let mut out = Vec::new();

        write_textlog_buffered_packet(
            &mut out,
            0x0b0c_0d0e_0f10_1112,
            Instant::from_ticks(1_000_000),
            Instant::from_ticks(3_040_005),
            PacketType::Handshake,
        );

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            "0b0c0d0e0f101112: T= 2.040005, Keys unavailable, buffered packet type 4.\n"
        );
    }

    #[test]
    fn packet_lost_logs_prefix_time_path_number_size_dcid_and_reason() {
        let mut out = Vec::new();
        let dcid = ConnectionId::clone_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd]).expect("dcid");

        write_textlog_packet_lost(
            &mut out,
            0x0b0c_0d0e_0f10_1112,
            Instant::from_ticks(1_000_000),
            Instant::from_ticks(3_040_005),
            PacketType::OneRttProtected,
            17,
            0x1234,
            "timer",
            Some(&dcid),
            1200,
        );

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            "0b0c0d0e0f101112: T= 2.040005, Lost packet type 6, path 17, number 4660, size 1200, DCID <aabbccdd>, reason: timer\n"
        );
    }

    #[test]
    fn packet_lost_omits_dcid_when_unknown() {
        let mut out = Vec::new();

        write_textlog_packet_lost(
            &mut out,
            0,
            Instant::from_ticks(0),
            Instant::from_ticks(42),
            PacketType::Initial,
            0,
            7,
            "ack",
            None,
            128,
        );

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            "T= 0.000042, Lost packet type 2, path 0, number 7, size 128, reason: ack\n"
        );
    }

    #[test]
    fn congestion_state_logs_supplied_path_snapshot() {
        let mut out = Vec::new();
        let cc_state = TextlogCongestionState {
            unique_path_id: 17,
            cwin: 12_345,
            bytes_in_transit: 678,
            nb_retransmission_total: 5,
            rtt_min: Duration::from_ticks(1_111),
            smoothed_rtt: Duration::from_ticks(2_222),
            rtt_variant: Duration::from_ticks(333),
            max_ack_delay: Duration::from_ticks(44),
            connection_state: State::Ready,
        };

        write_textlog_congestion_state(
            &mut out,
            0x0b0c_0d0e_0f10_1112,
            Instant::from_ticks(1_000_000),
            Instant::from_ticks(3_040_005),
            cc_state,
        );

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            "0b0c0d0e0f101112: T= 2.040005, path_id: 17,cwin: 12345,flight: 678,nb_ret: 5,rtt_min: 1111,rtt: 2222,rtt_var: 333,max_ack_delay: 44,state: 14\n"
        );
    }

    #[test]
    fn negotiated_alpn_logs_received_count_and_entries() {
        let mut out = Vec::new();
        let list: [&[u8]; 2] = [&b"h3"[..], &b"hq-interop"[..]];

        write_textlog_negotiated_alpn(&mut out, 0x0b0c_0d0e_0f10_1112, true, &list);

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            "0b0c0d0e0f101112: Received ALPN list (2): h3, hq-interop\n"
        );
    }

    #[test]
    fn negotiated_alpn_logs_sending_and_truncates_long_entries() {
        let mut out = Vec::new();
        let long = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789xx";
        let list: [&[u8]; 1] = [&long[..]];

        write_textlog_negotiated_alpn(&mut out, 0, false, &list);

        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            "Sending ALPN list (1): abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ01234567...\n"
        );
    }
}
