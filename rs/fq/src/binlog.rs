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
//!    sink, and the rest hang on the [`Binlog`] trait implemented for
//!    [`Connection`] (they pull the file handle from
//!    `connection.f_binlog` themselves).
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
//!   `connection->f_binlog`, `quic->binlog_dir`).  Free functions
//!   keyed on `Quic*` become inherent methods on [`Quic`]; those keyed
//!   on `Connection*` become methods on the [`Binlog`] trait
//!   (implemented for [`Connection`]).
//! * `Path*` — only ever accessed inside
//!   `binlog_get_path_id(connection, path_x)`, which dereferences `path_x`
//!   to read `unique_path_id`.  Every observed caller passes a
//!   non-NULL path handle, so this is `&mut Path` for parity with
//!   the unified-log dispatch trait (which takes the path mutably
//!   for the same hooks).
//! * `const ConnectionId*` (in [`pdu`] / [`packet`]) →
//!   `&ConnectionId`.  The C contract is "must be non-NULL"; every
//!   caller passes `&connection->initial_connection_id`.
//! * `ConnectionId* dcid` (in [`Binlog::packet_lost`]) is
//!   nullable per `loss_recovery.c` — when no remote CID is known
//!   the C call site passes NULL and the body emits a single zero
//!   length byte.  Maps to `Option<&ConnectionId>`.  The C signature
//!   drops `const` but the body only reads through the pointer, so
//!   the Rust equivalent stays a shared borrow.
//! * `packet_header* ph` (in [`Binlog::dropped_packet`],
//!   [`Binlog::outgoing_packet`]) — the body only *reads*
//!   `ph->ptype` in the dropped path and reconstructs a fresh header
//!   in the outgoing path.  In [`Binlog::outgoing_packet`] the
//!   parsed-header buffer is constructed locally, so no pointer
//!   crosses the API boundary.  In [`packet`] /
//!   [`Binlog::dropped_packet`] we map `ph` to `&PacketHeader`
//!   (immutable borrow) — matching the unified-log trait shape.
//! * `ConnectionId connection_id` (in [`tls_ticket`]) is `Copy` and
//!   pass-by-value, mirroring the C ABI.
//! * `const struct sockaddr*` pairs (`addr_peer`, `addr_local` in
//!   [`pdu`]) → `&core::net::SocketAddr`, matching the convention
//!   established in [`crate::logger`].
//! * `const uint8_t* + size_t` argument pairs collapse to `&[u8]`
//!   (`bytes`/`bytes_max` in [`packet`], `params`/`param_length` in
//!   [`Binlog::transport_extension`], `ticket`/`ticket_length`
//!   in [`tls_ticket`], `sni`/`sni_len` and `alpn`/`alpn_len` in
//!   [`Binlog::negotiated_alpn`]).  An empty slice models the C
//!   `(NULL, 0)` callers cleanly.
//! * [`Binlog::outgoing_packet`] carries *two* buffers: the
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
//! * `char const* trigger` (in [`Binlog::packet_lost`]) is
//!   always a non-NULL static string literal at the call sites
//!   grepped in `loss_recovery.c`, so it maps to `&str`.
//!
//! Error handling: [`Quic::set_binlog`] mirrors the C `int` return
//! (always `0` today, but reserved for future failure modes) as
//! `Result<(), Error>` per the project's error-handling convention.

use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::path::{Path as FsPath, PathBuf};
use std::rc::Rc;

use core::net::SocketAddr;

use crate::Error;
use crate::Instant;
use crate::bytestream::{BYTESTREAM_MAX_BUFFER_SIZE, ByteStream, ByteStreamBuf};
use crate::frames::FrameType;
use crate::internal::{
    Connection, Epoch, PacketHeader, PacketType, Path, SUPPORTED_VERSIONS, frames_varint_decode,
    frames_varint_skip, varint_decode, varint_encode,
};
use crate::logger::{Logger, LoggerRef};
use crate::{ConnectionId, PacketContext, Quic};

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
// Helpers private to this module.

/// Big-endian 4-byte length prefix + payload.  C: the trailing
/// `fwrite(head, 4, …); fwrite(data, len, …)` pair that every
/// binlog record ends with.
fn write_record(f: &mut File, payload: &[u8]) {
    let head = (payload.len() as u32).to_be_bytes();
    let _ = f.write_all(&head);
    let _ = f.write_all(payload);
}

fn connection_id_hexa(cid: &ConnectionId) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(cid.len() * 2);
    for &b in cid.as_bytes() {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Open a binlog file and write its fixed 16-byte stream header.
///
/// C: `create_binlog` (`logwriter.c:1123-1145`).
pub fn create_binlog<P: AsRef<FsPath>>(
    binlog_file: P,
    creation_time: u64,
    is_multipath_supported: bool,
) -> Option<File> {
    let path = binlog_file.as_ref();
    let mut f_binlog = match File::create(path) {
        Ok(file) => file,
        Err(_) => {
            log::debug!("Cannot open file {} for write.", path.display());
            return None;
        }
    };

    let mut buf = ByteStreamBuf::default();
    let mut stream = buf.stream(16)?;
    let flags: u16 = if is_multipath_supported { 0x01 } else { 0 };
    let header_result = stream
        .write_u32(crate::fourcc(b'q', b'l', b'o', b'g'))
        .and_then(|()| stream.write_u16(flags))
        .and_then(|()| stream.write_u16(0x01))
        .and_then(|()| stream.write_u64(creation_time));

    if header_result.is_err() || f_binlog.write_all(stream.as_bytes()).is_err() {
        log::debug!("Cannot write header for file {}.", path.display());
        return None;
    }

    Some(f_binlog)
}

/// Write the per-record common prefix.  C: `binlog_compose_event_header`.
fn compose_event_header(
    msg: &mut ByteStream<'_>,
    cid: &ConnectionId,
    current_time: Instant,
    path_id: u64,
    event_type: LogEventType,
) {
    let _ = msg.write_cid(cid);
    let _ = msg.write_varint(current_time.ticks());
    let _ = msg.write_varint(path_id);
    let _ = msg.write_varint(event_type as u64);
}

/// C: `binlog_get_path_id`.
fn get_path_id(connection: &Connection, path_x: &Path) -> u64 {
    if connection.is_multipath_enabled {
        path_x.unique_path_id
    } else {
        0
    }
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

/// Append a varint-encoded `value` to `out`.
fn append_varint(out: &mut Vec<u8>, value: u64) {
    let mut buf = [0u8; 8];
    let n = varint_encode(&mut buf, value);
    out.extend_from_slice(&buf[..n]);
}

/// Append a length-prefixed copy of `frame_bytes` to `out`.
/// C: `picoquic_binlog_frame`.
fn append_frame(out: &mut Vec<u8>, frame_bytes: &[u8]) {
    append_varint(out, frame_bytes.len() as u64);
    out.extend_from_slice(frame_bytes);
}

fn supported_version_index(version: u32) -> Option<i32> {
    SUPPORTED_VERSIONS
        .iter()
        .position(|v| *v as u32 == version)
        .map(|index| index as i32)
}

/// Parse the encrypted wire header for [`Binlog::outgoing_packet`].
///
/// C calls `picoquic_parse_packet_header(..., pcnx = cnx, receiving = 0)`.
/// In that mode short headers use the peer CID length from the connection
/// before the packet-number field; long-header packet type decoding is driven
/// by the negotiated version table.
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
        // Short header — only 1-RTT in flight.
        ph.packet_context = PacketContext::Application;
        ph.epoch = Epoch::OneRtt;
        ph.payload_length_value = 0;
        if send_buffer.len() < 1 + outgoing_short_dcid_len {
            ph.packet_type = PacketType::Error;
            ph.offset = send_buffer.len();
            ph.payload_length = 0;
            return ph;
        }
        if let Some(cid) =
            ConnectionId::clone_from_slice(&send_buffer[1..1 + outgoing_short_dcid_len])
        {
            ph.dest_connection_id = cid;
        } else {
            ph.packet_type = PacketType::Error;
            ph.offset = send_buffer.len();
            ph.payload_length = 0;
            return ph;
        }
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

    // Long-header payload-length varint, then PN.
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

// Frame parsers used by `binlog_frames`.

fn skip_fixed(bytes: &[u8], size: usize) -> Option<&[u8]> {
    bytes.get(size..)
}

fn read_length(bytes: &[u8]) -> Option<(usize, &[u8])> {
    let mut n64 = 0u64;
    let rest = frames_varint_decode(bytes, &mut n64)?;
    let n = n64 as usize;
    if (n as u64) != n64 {
        return None;
    }
    Some((n, rest))
}

fn log_stream_error<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let length = bytes_in.len().min(26);
    append_frame(out, &bytes_in[..length]);
    None
}

fn log_stream_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    if bytes_in.is_empty() {
        return None;
    }
    let ftype = bytes_in[0];
    let mut bytes = match skip_fixed(bytes_in, 1) {
        Some(rest) => rest,
        None => return log_stream_error(out, bytes_begin),
    };
    bytes = match frames_varint_skip(bytes) {
        Some(rest) => rest,
        None => return log_stream_error(out, bytes_begin),
    };
    if (ftype & 4) != 0 {
        bytes = match frames_varint_skip(bytes) {
            Some(rest) => rest,
            None => return log_stream_error(out, bytes_begin),
        };
    }

    let has_length = (ftype & 2) != 0;
    let length: usize;
    if has_length {
        let (l, rest) = match read_length(bytes) {
            Some(v) => v,
            None => return log_stream_error(out, bytes_begin),
        };
        length = l;
        bytes = rest;
    } else {
        length = bytes.len();
    }

    let mut extra_bytes: usize = 8;
    if length < extra_bytes {
        extra_bytes = length;
    }

    if has_length {
        let header_len = bytes_begin.len() - bytes.len();
        let copy_end = header_len + extra_bytes.min(bytes.len());
        append_frame(out, &bytes_begin[..copy_end]);
    } else {
        let head_len = bytes_begin.len() - bytes.len();
        let mut log_buffer = Vec::with_capacity(head_len + 8 + extra_bytes);
        log_buffer.extend_from_slice(&bytes_begin[..head_len]);
        let mut len_buf = [0u8; 8];
        let n = varint_encode(&mut len_buf, length as u64);
        log_buffer.extend_from_slice(&len_buf[..n]);
        let payload_start = head_len;
        let avail = bytes_begin.len().saturating_sub(payload_start);
        let take = extra_bytes.min(avail);
        log_buffer.extend_from_slice(&bytes_begin[payload_start..payload_start + take]);
        append_frame(out, &log_buffer);
    }

    skip_fixed(bytes, length)
}

fn log_ack_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut ftype = 0u64;
    let _ = frames_varint_decode(bytes_in, &mut ftype)?;
    let mut bytes = frames_varint_skip(bytes_in)?;
    if ftype == FrameType::PathAck as u64 || ftype == FrameType::PathAckEcn as u64 {
        bytes = frames_varint_skip(bytes)?;
    }
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let mut nb_blocks = 0u64;
    bytes = frames_varint_decode(bytes, &mut nb_blocks)?;
    bytes = frames_varint_skip(bytes)?;
    for _ in 0..nb_blocks {
        bytes = frames_varint_skip(bytes)?;
        bytes = frames_varint_skip(bytes)?;
    }
    if ftype == FrameType::AckEcn as u64 || ftype == FrameType::PathAckEcn as u64 {
        bytes = frames_varint_skip(bytes)?;
        bytes = frames_varint_skip(bytes)?;
        bytes = frames_varint_skip(bytes)?;
    }
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_simple_n_varints<'a>(
    out: &mut Vec<u8>,
    bytes_in: &'a [u8],
    skip_byte: bool,
    n: usize,
) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = if skip_byte {
        skip_fixed(bytes_in, 1)?
    } else {
        frames_varint_skip(bytes_in)?
    };
    for _ in 0..n {
        bytes = frames_varint_skip(bytes)?;
    }
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_close_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let (length, rest) = read_length(bytes)?;
    bytes = skip_fixed(rest, length)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_app_close_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    let (length, rest) = read_length(bytes)?;
    bytes = skip_fixed(rest, length)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_new_connection_id_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    if bytes.is_empty() {
        return None;
    }
    let cid_len = bytes[0] as usize;
    bytes = skip_fixed(bytes, 1 + cid_len)?;
    bytes = skip_fixed(bytes, crate::RESET_SECRET_SIZE)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_path_new_connection_id_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    if bytes.is_empty() {
        return None;
    }
    let cid_len = bytes[0] as usize;
    bytes = skip_fixed(bytes, 1 + cid_len)?;
    bytes = skip_fixed(bytes, crate::RESET_SECRET_SIZE)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_new_token_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    let (length, rest) = read_length(bytes)?;
    bytes = skip_fixed(rest, length)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_path_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes = skip_fixed(bytes_in, 1 + 8)?;
    let consumed = bytes_in.len() - bytes.len();
    append_frame(out, &bytes_in[..consumed]);
    Some(bytes)
}

fn log_crypto_hs_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = skip_fixed(bytes_in, 1)?;
    bytes = frames_varint_skip(bytes)?;
    let (length, rest) = read_length(bytes)?;
    bytes = rest;
    let header_len = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..header_len]);
    bytes = skip_fixed(bytes, length)?;
    Some(bytes)
}

fn log_handshake_done_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes = skip_fixed(bytes_in, 1)?;
    let consumed = bytes_in.len() - bytes.len();
    append_frame(out, &bytes_in[..consumed]);
    Some(bytes)
}

fn log_datagram_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    if bytes_in.is_empty() {
        return None;
    }
    let ftype = bytes_in[0];
    let mut bytes = skip_fixed(bytes_in, 1)?;
    let length: usize;
    if ftype & 1 != 0 {
        let (l, rest) = read_length(bytes)?;
        length = l;
        bytes = rest;
    } else {
        length = bytes.len();
    }
    let header_len = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..header_len]);
    bytes = skip_fixed(bytes, length)?;
    Some(bytes)
}

fn log_time_stamp_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_path_abandon_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_path_available_or_backup_frame<'a>(
    out: &mut Vec<u8>,
    bytes_in: &'a [u8],
) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_ack_frequency_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_immediate_ack_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let bytes = frames_varint_skip(bytes_in)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_padding<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    if bytes_in.is_empty() {
        return None;
    }
    append_frame(out, &bytes_in[..1]);
    let ftype = bytes_in[0];
    let mut idx = 0usize;
    while idx < bytes_in.len() && bytes_in[idx] == ftype {
        idx += 1;
    }
    Some(&bytes_in[idx..])
}

fn log_bdp_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let (ip_len, rest) = read_length(bytes)?;
    bytes = skip_fixed(rest, ip_len)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_observed_address_frame<'a>(
    out: &mut Vec<u8>,
    bytes_in: &'a [u8],
    ftype: u64,
) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let ip_len = if (ftype & 1) == 0 { 4 } else { 16 };
    let data_len = ip_len + 2;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = skip_fixed(bytes, data_len)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}

fn log_erroring_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let copied = bytes_in.len().min(8);
    append_frame(out, &bytes_in[..copied]);
    None
}

/// Walk a packet payload's frame stream, appending one
/// length-prefixed entry per frame to `out`.  C:
/// `picoquic_binlog_frames`.
fn binlog_frames(out: &mut Vec<u8>, mut bytes: &[u8]) {
    loop {
        if bytes.is_empty() {
            return;
        }
        let mut ftype = 0u64;
        let ftype_ll = varint_decode(bytes, &mut ftype);
        if ftype_ll == 0 {
            return;
        }
        if ftype < 64 && ftype_ll != 1 {
            return;
        }

        if ftype >= FrameType::StreamRangeMin as u64 && ftype <= FrameType::StreamRangeMax as u64 {
            match log_stream_frame(out, bytes) {
                Some(rest) => bytes = rest,
                None => return,
            }
            continue;
        }

        let next: Option<&[u8]> = match ftype {
            v if v == FrameType::Ack as u64
                || v == FrameType::AckEcn as u64
                || v == FrameType::PathAck as u64
                || v == FrameType::PathAckEcn as u64 =>
            {
                log_ack_frame(out, bytes)
            }
            v if v == FrameType::RetireConnectionId as u64 => {
                log_simple_n_varints(out, bytes, true, 1)
            }
            v if v == FrameType::PathRetireConnectionId as u64 => {
                log_simple_n_varints(out, bytes, false, 2)
            }
            v if v == FrameType::Padding as u64 || v == FrameType::Ping as u64 => {
                log_padding(out, bytes)
            }
            v if v == FrameType::ResetStream as u64 => log_simple_n_varints(out, bytes, true, 3),
            v if v == FrameType::ResetStreamAt as u64 => log_simple_n_varints(out, bytes, true, 4),
            v if v == FrameType::ConnectionClose as u64 => log_close_frame(out, bytes),
            v if v == FrameType::ApplicationClose as u64 => log_app_close_frame(out, bytes),
            v if v == FrameType::MaxData as u64 => log_simple_n_varints(out, bytes, true, 1),
            v if v == FrameType::MaxStreamData as u64 => log_simple_n_varints(out, bytes, true, 2),
            v if v == FrameType::MaxStreamsBidir as u64
                || v == FrameType::MaxStreamsUnidir as u64 =>
            {
                log_simple_n_varints(out, bytes, true, 1)
            }
            v if v == FrameType::DataBlocked as u64 => log_simple_n_varints(out, bytes, true, 1),
            v if v == FrameType::StreamDataBlocked as u64 => {
                log_simple_n_varints(out, bytes, true, 2)
            }
            v if v == FrameType::StreamsBlockedBidir as u64
                || v == FrameType::StreamsBlockedUnidir as u64 =>
            {
                log_simple_n_varints(out, bytes, true, 1)
            }
            v if v == FrameType::NewConnectionId as u64 => log_new_connection_id_frame(out, bytes),
            v if v == FrameType::PathNewConnectionId as u64 => {
                log_path_new_connection_id_frame(out, bytes)
            }
            v if v == FrameType::StopSending as u64 => log_simple_n_varints(out, bytes, true, 2),
            v if v == FrameType::PathChallenge as u64 || v == FrameType::PathResponse as u64 => {
                log_path_frame(out, bytes)
            }
            v if v == FrameType::CryptoHs as u64 => log_crypto_hs_frame(out, bytes),
            v if v == FrameType::NewToken as u64 => log_new_token_frame(out, bytes),
            v if v == FrameType::HandshakeDone as u64 => log_handshake_done_frame(out, bytes),
            v if v == FrameType::Datagram as u64 || v == FrameType::DatagramL as u64 => {
                log_datagram_frame(out, bytes)
            }
            v if v == FrameType::AckFrequency as u64 => log_ack_frequency_frame(out, bytes),
            v if v == FrameType::ImmediateAck as u64 => log_immediate_ack_frame(out, bytes),
            v if v == FrameType::TimeStamp as u64 => log_time_stamp_frame(out, bytes),
            v if v == FrameType::PathAbandon as u64 => log_path_abandon_frame(out, bytes),
            v if v == FrameType::PathBackup as u64 || v == FrameType::PathAvailable as u64 => {
                log_path_available_or_backup_frame(out, bytes)
            }
            v if v == FrameType::Bdp as u64 => log_bdp_frame(out, bytes),
            v if v == FrameType::ObservedAddressV4 as u64
                || v == FrameType::ObservedAddressV6 as u64 =>
            {
                log_observed_address_frame(out, bytes, ftype)
            }
            _ => log_erroring_frame(out, bytes),
        };

        match next {
            Some(rest) => bytes = rest,
            None => return,
        }
    }
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
    f: &mut File,
    cid: &ConnectionId,
    receiving: bool,
    current_time: Instant,
    addr_peer: &SocketAddr,
    addr_local: &SocketAddr,
    packet_length: usize,
    unique_path_id: u64,
    ecn: u8,
) {
    let mut buf = ByteStreamBuf::default();
    let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
        return;
    };

    let event = if receiving {
        LogEventType::PduRecv
    } else {
        LogEventType::PduSent
    };
    compose_event_header(&mut msg, cid, current_time, 0, event);

    let _ = msg.write_addr(addr_peer);
    let _ = msg.write_varint(packet_length as u64);
    let _ = msg.write_addr(addr_local);
    let _ = msg.write_varint(unique_path_id);
    let _ = msg.write_u8(ecn);

    let payload: Vec<u8> = msg.as_bytes().to_vec();
    drop(msg);
    write_record(f, &payload);
}

/// Write a decrypted-packet record to `f`.  Binary alternative to
/// `log_decrypted_segment()`.
///
/// C: `void binlog_packet(FILE*, const ConnectionId*,
/// uint64_t, int, uint64_t, const packet_header*,
/// const uint8_t*, size_t)`.
pub fn packet(
    f: &mut File,
    cid: &ConnectionId,
    path_id: u64,
    receiving: bool,
    current_time: Instant,
    ph: &PacketHeader,
    bytes: &[u8],
) {
    // Buffer the whole record (header + framing) into a `Vec<u8>`,
    // then write 4-byte length + payload.  The C version reserves
    // four bytes via `fseek`/`ftell` because `picoquic_binlog_frames`
    // writes straight to the FILE; the buffered shape is the
    // natural Rust equivalent.
    let mut payload: Vec<u8> = Vec::new();

    {
        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };

        let event = if receiving {
            LogEventType::PacketRecv
        } else {
            LogEventType::PacketSent
        };
        compose_event_header(&mut msg, cid, current_time, path_id, event);

        let _ = msg.write_varint(bytes.len() as u64);

        // Packed header byte: quic_bit_is_zero<<6 | spin<<1 | key_phase.
        let flags: u8 = (if ph.quic_bit_is_zero { 64 } else { 0 })
            + (if ph.spin { 2 } else { 0 })
            + (if ph.key_phase { 1 } else { 0 });
        let _ = msg.write_u8(flags);
        let _ = msg.write_varint(ph.payload_length as u64);
        let _ = msg.write_varint(ph.packet_type as u64);
        let _ = msg.write_varint(ph.packet_number_full);

        let _ = msg.write_cid(&ph.dest_connection_id);
        let _ = msg.write_cid(&ph.src_connection_id);

        if ph.packet_type != PacketType::OneRttProtected
            && ph.packet_type != PacketType::VersionNegotiation
        {
            let _ = msg.write_u32(ph.version);
        }

        if ph.packet_type == PacketType::Initial {
            let _ = msg.write_varint(ph.token_bytes.len() as u64);
            let _ = msg.write_bytes(&ph.token_bytes);
        }

        payload.extend_from_slice(msg.as_bytes());
    }

    if ph.packet_type == PacketType::VersionNegotiation || ph.packet_type == PacketType::Retry {
        let payload_slice = bytes.get(ph.offset..).unwrap_or(&[]);
        append_frame(&mut payload, payload_slice);
    } else if ph.packet_type != PacketType::Error {
        let end = ph.offset.saturating_add(ph.payload_length).min(bytes.len());
        let frames_slice = bytes.get(ph.offset..end).unwrap_or(&[]);
        binlog_frames(&mut payload, frames_slice);
    }

    write_record(f, &payload);
}

/// Write a TLS session-ticket record to `f`.  Binary alternative
/// to `log_tls_ticket()`.  The connection-id parameter is `Copy`
/// and passed by value to mirror the C ABI.
///
/// C: `void binlog_picotls_ticket(FILE*, ConnectionId,
/// uint8_t*, uint16_t)`.
pub fn tls_ticket(f: &mut File, cnx_id: ConnectionId, ticket: &[u8]) {
    let mut buf = ByteStreamBuf::default();
    let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
        return;
    };
    compose_event_header(
        &mut msg,
        &cnx_id,
        Instant::from_ticks(0),
        0,
        LogEventType::TlsKeyUpdate,
    );
    let _ = msg.write_varint(ticket.len() as u64);
    let _ = msg.write_bytes(ticket);

    let payload = msg.as_bytes().to_vec();
    drop(msg);
    write_record(f, &payload);
}

// ---------------------------------------------------------------------------
// High-level per-event writers — methods on [`Connection`].
//
// Each method pulls the file handle from `connection.f_binlog` and
// delegates to one of the low-level writers above (or composes
// several records).  Bundling them as a [`Binlog`] trait (rather
// than inherent methods on `Connection`) lets callers opt into the
// capability with `use crate::binlog::Binlog`, scopes the names to
// the trait, and keeps the names short — no `binlog_` prefix
// needed since the trait disambiguates.

/// Binary trace recording for a [`Connection`].  Each method writes
/// one record (or composes a few records) into the connection's
/// `f_binlog` file; bringing the trait into scope opts the caller
/// into the capability.  C: the per-event writers in
/// `quic/binlog.c`.
pub trait Binlog {
    /// Report that a packet was dropped due to some error.
    ///
    /// C: `void binlog_dropped_packet(Connection*, Path*,
    /// packet_header*, size_t, int, uint64_t)`.
    fn dropped_packet(
        &mut self,
        path_x: &mut Path,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    );

    /// Report that a packet was buffered waiting for decryption.
    ///
    /// C: `void binlog_buffered_packet(Connection*, Path*,
    /// packet_type_enum, uint64_t)`.
    fn buffered_packet(&mut self, path_x: &mut Path, ptype: PacketType, current_time: Instant);

    /// Binary alternative to `log_outgoing_segment()`.  `bytes`
    /// is the unencrypted payload (the C `length` parameter is
    /// folded into the slice); `send_buffer` is the encrypted,
    /// padded wire form.  `pn_length` is the offset of the
    /// packet-number field inside `bytes`.
    ///
    /// C: `void binlog_outgoing_packet(Connection*, Path*,
    /// uint8_t*, uint64_t, size_t, size_t, uint8_t*, size_t,
    /// uint64_t)`.
    fn outgoing_packet(
        &mut self,
        path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: Instant,
    );

    /// Log a packet-lost event.  `dcid` is `None` when the remote
    /// connection ID is unknown — the C body emits a single zero
    /// byte in that case.
    ///
    /// C: `void binlog_packet_lost(Connection*, Path*,
    /// packet_type_enum, uint64_t, char const*,
    /// ConnectionId*, size_t, uint64_t)`.
    fn packet_lost(
        &mut self,
        path_x: &mut Path,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: Instant,
    );

    /// Log negotiated SNI / ALPN.  Empty `sni`/`alpn` slices stand
    /// in for the C `(NULL, 0)` callers.
    ///
    /// C: `void binlog_negotiated_alpn(Connection*, int,
    /// uint8_t const*, size_t, uint8_t const*, size_t,
    /// const PtlsIovec*, size_t)`.
    fn negotiated_alpn(&mut self, is_local: bool, sni: &[u8], alpn: &[u8], alpn_list: &[&[u8]]);

    /// Binary alternative to `log_transport_extension()`.
    ///
    /// C: `void binlog_transport_extension(Connection*, int, size_t,
    /// uint8_t*)`.
    fn transport_extension(&mut self, is_local: bool, params: &[u8]);

    /// Write a free-form information message.
    ///
    /// C: `picoquic_binlog_message_v` (`logwriter.c:1237-1272`).
    fn message_v(&mut self, args: core::fmt::Arguments<'_>);

    /// Open the per-connection binlog file and emit the
    /// `new_connection` record.  Idempotent — the C body is a no-op
    /// when neither `quic->binlog_dir` nor `quic->qlog_dir` are
    /// set, or when `quic->bin_log_fns` is `NULL`.
    ///
    /// C: `void binlog_new_connection(Connection*)`.
    fn new_connection(&mut self);

    /// Emit the `connection_close` record and close the
    /// per-connection binlog file.  Safe to call when no binlog is
    /// currently open (the C body guards on `connection->f_binlog !=
    /// NULL`).
    ///
    /// C: `void binlog_close_connection(Connection*)`.
    fn close_connection(&mut self);

    /// Log the state of the congestion controller, retransmission
    /// queues, etc.  Called either just after processing an
    /// incoming packet or just after sending one.
    ///
    /// C: `void binlog_cc_dump(Connection*, Path*, uint64_t)`.
    fn cc_dump(&mut self, path_x: &mut Path, current_time: Instant);
}

impl Binlog for Connection {
    fn dropped_packet(
        &mut self,
        path_x: &mut Path,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    ) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        // Reserve four bytes for the chunk size; patch afterwards
        // — mirrors the C source.
        let _ = msg.write_u32(0);
        compose_event_header(
            &mut msg,
            &cid,
            current_time,
            path_id,
            LogEventType::PacketDropped,
        );
        let _ = msg.write_varint(ph.packet_type as u64);
        let _ = msg.write_varint(packet_size as u64);
        let _ = msg.write_varint(err as u64);

        let body_len = (msg.len().saturating_sub(4)) as u32;
        let mut payload = msg.as_bytes().to_vec();
        drop(msg);
        payload[..4].copy_from_slice(&body_len.to_be_bytes());

        if let Some(f) = self.f_binlog.as_mut() {
            let _ = f.write_all(&payload);
        }
    }

    fn buffered_packet(&mut self, path_x: &mut Path, ptype: PacketType, current_time: Instant) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        let _ = msg.write_u32(0);
        compose_event_header(
            &mut msg,
            &cid,
            current_time,
            path_id,
            LogEventType::PacketBuffered,
        );
        let _ = msg.write_varint(ptype as u64);
        let _ = msg.write_str("keys_unavailable");

        let body_len = (msg.len().saturating_sub(4)) as u32;
        let mut payload = msg.as_bytes().to_vec();
        drop(msg);
        payload[..4].copy_from_slice(&body_len.to_be_bytes());
        if let Some(f) = self.f_binlog.as_mut() {
            let _ = f.write_all(&payload);
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
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);

        // The C body calls `picoquic_parse_packet_header(cnx->quic,
        // send_buffer, …, &pcnx, 0)` with `pcnx` pre-set to this
        // connection.  Mirror the outgoing short-header CID-length
        // rule locally before emitting the binlog record.
        let outgoing_short_dcid_len = initial_remote_connection_id(self).len();
        let mut ph = parse_outgoing_header(
            send_buffer,
            outgoing_short_dcid_len,
            self.version_index,
            self.local_parameters.do_grease_quic_bit,
            self.is_loss_bit_enabled_outgoing,
        );

        let checksum_length: usize = {
            let epoch = match ph.packet_type {
                PacketType::OneRttProtected => crate::internal::Epoch::OneRtt,
                PacketType::ZeroRttProtected => crate::internal::Epoch::ZeroRtt,
                PacketType::Handshake => crate::internal::Epoch::Handshake,
                _ => crate::internal::Epoch::Initial,
            };
            if self.crypto_context[epoch as usize].aead_encrypt.is_some() {
                self.get_checksum_length(epoch)
            } else {
                16
            }
        };

        ph.packet_number_full = sequence_number;
        ph.packet_number_truncated = sequence_number as u32;
        if ph.packet_type != PacketType::Retry && ph.packet_number_offset != 0 {
            ph.offset = ph.packet_number_offset + pn_length;
            ph.payload_length = ph.payload_length.saturating_sub(pn_length);
        }
        if ph.packet_type != PacketType::VersionNegotiation {
            if ph.payload_length > checksum_length {
                ph.payload_length -= checksum_length;
            } else {
                ph.payload_length = 0;
            }
        }

        if let Some(f) = self.f_binlog.as_mut() {
            packet(f, &cid, path_id, false, current_time, &ph, bytes);
        }
    }

    fn packet_lost(
        &mut self,
        path_x: &mut Path,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: Instant,
    ) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        let _ = msg.write_u32(0);
        compose_event_header(
            &mut msg,
            &cid,
            current_time,
            path_id,
            LogEventType::PacketLost,
        );
        let _ = msg.write_varint(ptype as u64);
        let _ = msg.write_varint(sequence_number);
        let _ = msg.write_str(trigger);
        match dcid {
            Some(d) => {
                let _ = msg.write_cid(d);
            }
            None => {
                let _ = msg.write_u8(0);
            }
        }
        let _ = msg.write_varint(packet_size as u64);

        let body_len = (msg.len().saturating_sub(4)) as u32;
        let mut payload = msg.as_bytes().to_vec();
        drop(msg);
        payload[..4].copy_from_slice(&body_len.to_be_bytes());
        if let Some(f) = self.f_binlog.as_mut() {
            let _ = f.write_all(&payload);
        }
    }

    fn negotiated_alpn(&mut self, is_local: bool, sni: &[u8], alpn: &[u8], alpn_list: &[&[u8]]) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let now = self.quic_time();

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        compose_event_header(&mut msg, &cid, now, 0, LogEventType::AlpnUpdate);
        let _ = msg.write_varint(if is_local { 1 } else { 0 });
        let _ = msg.write_varint(sni.len() as u64);
        if !sni.is_empty() {
            let _ = msg.write_bytes(sni);
        }
        let _ = msg.write_varint(alpn_list.len() as u64);
        for entry in alpn_list {
            let _ = msg.write_varint(entry.len() as u64);
            let _ = msg.write_bytes(entry);
        }
        let _ = msg.write_varint(alpn.len() as u64);
        if !alpn.is_empty() {
            let _ = msg.write_bytes(alpn);
        }

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }

    fn transport_extension(&mut self, is_local: bool, params: &[u8]) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let now = self.quic_time();

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        compose_event_header(&mut msg, &cid, now, 0, LogEventType::ParamUpdate);
        let _ = msg.write_varint(if is_local { 1 } else { 0 });
        let _ = msg.write_varint(params.len() as u64);
        if !params.is_empty() {
            let _ = msg.write_bytes(params);
        }

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }

    fn message_v(&mut self, args: core::fmt::Arguments<'_>) {
        if self.f_binlog.is_none() {
            return;
        }

        let cid = self.initial_connection_id;
        let now = self.quic_time();
        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };

        compose_event_header(&mut msg, &cid, now, 0, LogEventType::InfoMessage);

        let message = args.to_string();
        let max_len = msg.remaining().saturating_sub(1);
        let message_len = message.len().min(max_len);
        let _ = msg.write_bytes(&message.as_bytes()[..message_len]);

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }

    fn new_connection(&mut self) {
        let quic_ptr = self.quic_ptr;
        if quic_ptr.is_null() {
            return;
        }

        let (bin_dir, use_unique_log_names, creation_time) = unsafe {
            // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal`
            // and remains valid while the connection is live.  This block only
            // snapshots context-level logging configuration and checks the
            // open-log cap before the connection-owned file is replaced.
            let quic = &mut *quic_ptr;
            if quic.bin_log_fns.is_none()
                || quic.current_number_of_open_logs >= quic.max_simultaneous_logs
            {
                return;
            }
            let Some(bin_dir) = quic.binlog_dir.as_ref().or(quic.qlog_dir.as_ref()).cloned() else {
                return;
            };
            (bin_dir, quic.use_unique_log_names, quic.time())
        };

        self.f_binlog = None;

        let cid_name = connection_id_hexa(&self.initial_connection_id);
        let role = if self.client_mode { "client" } else { "server" };
        let file_name = if use_unique_log_names {
            format!("{}.{:x}.{}.log", cid_name, self.log_unique, role)
        } else {
            format!("{}.{}.log", cid_name, role)
        };
        let log_filename = bin_dir.join(file_name);
        if log_filename.to_string_lossy().len() >= 512 {
            return;
        }
        self.binlog_file_name = Some(log_filename.clone());

        let Some(f_binlog) = create_binlog(
            &log_filename,
            creation_time,
            self.local_parameters.initial_max_path_id > 0,
        ) else {
            self.binlog_file_name = None;
            return;
        };
        self.f_binlog = Some(f_binlog);

        unsafe {
            // SAFETY: same ownership invariant as above; this mirrors the C
            // context-level open-log accounting increment after successful
            // binlog creation.
            (*quic_ptr).current_number_of_open_logs =
                (*quic_ptr).current_number_of_open_logs.saturating_add(1);
        }

        let cid = self.initial_connection_id;
        let start_time = self.start_time;
        let client_mode = self.client_mode;
        let proposed_version = self.proposed_version;
        let cc_id: &'static str = self
            .congestion_alg
            .map(|a| a.congestion_algorithm_id)
            .unwrap_or("");
        let spin_policy = self.spin_policy as u64;
        let remote_cid = initial_remote_connection_id(self);

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        compose_event_header(&mut msg, &cid, start_time, 0, LogEventType::NewConnection);
        let _ = msg.write_u8(if client_mode { 1 } else { 0 });
        let _ = msg.write_u32(proposed_version);
        let _ = msg.write_cid(&remote_cid);
        let _ = msg.write_str(cc_id);
        let _ = msg.write_varint(spin_policy);

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }

    fn close_connection(&mut self) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let now = self.quic_time();

        let mut buf = ByteStreamBuf::default();
        if let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) {
            compose_event_header(&mut msg, &cid, now, 0, LogEventType::ConnectionClose);
            let payload = msg.as_bytes().to_vec();
            drop(msg);

            if let Some(f) = self.f_binlog.as_mut() {
                write_record(f, &payload);
            }
        }

        if let Some(f) = self.f_binlog.as_mut() {
            let _ = f.flush();
        }

        // Close the file before auto-qlog conversion, matching C's
        // `cnx->f_binlog = picoquic_file_close(...)`.
        self.f_binlog = None;
        let quic_ptr = self.quic_ptr;

        if !quic_ptr.is_null() {
            // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal`
            // and remains valid while the connection is live.  Borrow only
            // the disjoint context fields needed for the close hook; the
            // callback receives a shared connection borrow, as required by
            // `AutoQlog::run`.
            unsafe {
                let qlog_dir = &(*quic_ptr).qlog_dir;
                let autoqlog_fn = &mut (*quic_ptr).autoqlog_fn;
                if qlog_dir.is_some()
                    && let Some(autoqlog) = autoqlog_fn.as_mut()
                {
                    let _ = autoqlog.run(self);
                }
            }
        }

        self.binlog_file_name = None;

        if !quic_ptr.is_null() {
            // SAFETY: same ownership invariant as above; this mirrors the C
            // context-level log-accounting decrement after the file was closed.
            unsafe {
                let open_logs = &mut (*quic_ptr).current_number_of_open_logs;
                if *open_logs > 0 {
                    *open_logs -= 1;
                }
            }
        }
    }

    fn cc_dump(&mut self, path_x: &mut Path, current_time: Instant) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);
        let is_multipath = self.is_multipath_enabled;

        // Snapshot the packet-context fields up front — multipath
        // chooses path_x's pkt_ctx, single-path uses the connection's
        // application-context entry.
        let (
            send_sequence,
            highest_acknowledged,
            highest_acknowledged_time,
            latest_time_acknowledged,
        ) = if is_multipath {
            (
                path_x.pkt_ctx.send_sequence,
                path_x.pkt_ctx.highest_acknowledged,
                path_x.pkt_ctx.highest_acknowledged_time,
                path_x.pkt_ctx.latest_time_acknowledged,
            )
        } else {
            let ctx = &self.pkt_ctx[crate::PacketContext::Application as usize];
            (
                ctx.send_sequence,
                ctx.highest_acknowledged,
                ctx.highest_acknowledged_time,
                ctx.latest_time_acknowledged,
            )
        };

        let path_cwin = path_x.cwin;
        let one_way = path_x.one_way_delay_sample.ticks();
        let rtt_sample = path_x.rtt_sample.ticks();
        let smoothed_rtt = path_x.smoothed_rtt.ticks();
        let rtt_min = path_x.rtt_min.ticks();
        let bw = path_x.bandwidth_estimate;
        let rate = path_x.receive_rate_estimate;
        let mtu = path_x.send_mtu as u64;
        let pacing_pkt_time = path_x.pacing.packet_time_microsec.ticks();
        let nb_losses_found = path_x.nb_losses_found;
        let nb_spurious_path = path_x.nb_spurious;
        let peak_bw = path_x.peak_bandwidth_estimate;
        let bytes_in_transit = path_x.bytes_in_transit;
        let limited = path_x.last_bw_estimate_path_limited;

        let nb_retx_total = self.nb_retransmission_total;
        let nb_spurious_cnx = self.nb_spurious;
        let cwin_blocked = self.cwin_blocked;
        let flow_blocked = self.flow_blocked;
        let stream_blocked = self.stream_blocked;
        let start_time = self.start_time.ticks();

        // Optional CC observation: paths[0] gets queried for its
        // congestion_alg_state, mirroring the C body.
        let cc_obs: Option<(u64, u64)> = self.congestion_alg.and_then(|alg| {
            self.paths.first().and_then(|p0| {
                if p0.congestion_alg_state.is_some() {
                    alg.algorithm.alg_observe(p0)
                } else {
                    None
                }
            })
        });

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        compose_event_header(
            &mut msg,
            &cid,
            current_time,
            path_id,
            LogEventType::CcUpdate,
        );

        let _ = msg.write_varint(send_sequence);
        if highest_acknowledged != u64::MAX {
            let _ = msg.write_varint(1);
            let _ = msg.write_varint(highest_acknowledged);
            let _ = msg.write_varint(highest_acknowledged_time.ticks().saturating_sub(start_time));
            let _ = msg.write_varint(latest_time_acknowledged.ticks().saturating_sub(start_time));
        } else {
            let _ = msg.write_varint(0);
        }

        let _ = msg.write_varint(path_cwin);
        let _ = msg.write_varint(one_way);
        let _ = msg.write_varint(rtt_sample);
        let _ = msg.write_varint(smoothed_rtt);
        let _ = msg.write_varint(rtt_min);
        let _ = msg.write_varint(bw);
        let _ = msg.write_varint(rate);
        let _ = msg.write_varint(mtu);
        let _ = msg.write_varint(pacing_pkt_time);
        if is_multipath {
            let _ = msg.write_varint(nb_losses_found);
            let _ = msg.write_varint(nb_spurious_path);
        } else {
            let _ = msg.write_varint(nb_retx_total);
            let _ = msg.write_varint(nb_spurious_cnx);
        }
        let _ = msg.write_varint(if cwin_blocked { 1 } else { 0 });
        let _ = msg.write_varint(if flow_blocked { 1 } else { 0 });
        let _ = msg.write_varint(if stream_blocked { 1 } else { 0 });

        match cc_obs {
            Some((cc_state, cc_param)) => {
                let _ = msg.write_varint(cc_state);
                let _ = msg.write_varint(cc_param);
            }
            None => {
                let _ = msg.write_varint(0);
                let _ = msg.write_varint(0);
            }
        }

        let _ = msg.write_varint(peak_bw);
        let _ = msg.write_varint(bytes_in_transit);
        let _ = msg.write_varint(if limited { 1 } else { 0 });

        let payload = msg.as_bytes().to_vec();
        drop(msg);
        if let Some(f) = self.f_binlog.as_mut() {
            write_record(f, &payload);
        }
    }
}

struct BinlogLogger;

impl Logger for BinlogLogger {
    fn quic_app_message(
        &mut self,
        _quic: &mut Quic,
        _cid: &ConnectionId,
        _args: core::fmt::Arguments<'_>,
    ) {
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
        quic.binlog_close();
    }

    fn app_message(&mut self, connection: &mut Connection, args: core::fmt::Arguments<'_>) {
        if connection.f_binlog.is_some() {
            Binlog::message_v(connection, args);
        }
    }

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
    ) {
        if !connection.is_still_logging() {
            return;
        }
        let cid = connection.initial_connection_id;
        if let Some(f) = connection.f_binlog.as_mut() {
            crate::binlog::pdu(
                f,
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

    fn packet(
        &mut self,
        connection: &mut Connection,
        path_x: Option<&mut Path>,
        receiving: bool,
        current_time: Instant,
        ph: &PacketHeader,
        bytes: &[u8],
    ) {
        if !connection.is_still_logging() {
            return;
        }
        let cid = connection.initial_connection_id;
        let path_id = path_x
            .as_deref()
            .map(|path| get_path_id(connection, path))
            .unwrap_or(0);
        if let Some(f) = connection.f_binlog.as_mut() {
            crate::binlog::packet(f, &cid, path_id, receiving, current_time, ph, bytes);
        }
    }

    fn dropped_packet(
        &mut self,
        connection: &mut Connection,
        path_x: Option<&mut Path>,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    ) {
        if !connection.is_still_logging() || connection.f_binlog.is_none() {
            return;
        }

        if let Some(path_x) = path_x {
            Binlog::dropped_packet(connection, path_x, ph, packet_size, err, current_time);
            return;
        }

        let cid = connection.initial_connection_id;
        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        let _ = msg.write_u32(0);
        compose_event_header(&mut msg, &cid, current_time, 0, LogEventType::PacketDropped);
        let _ = msg.write_varint(ph.packet_type as u64);
        let _ = msg.write_varint(packet_size as u64);
        let _ = msg.write_varint(err as u64);

        let body_len = (msg.len().saturating_sub(4)) as u32;
        let mut payload = msg.as_bytes().to_vec();
        drop(msg);
        payload[..4].copy_from_slice(&body_len.to_be_bytes());
        if let Some(f) = connection.f_binlog.as_mut() {
            let _ = f.write_all(&payload);
        }
    }

    fn buffered_packet(
        &mut self,
        connection: &mut Connection,
        path_x: &mut Path,
        ptype: PacketType,
        current_time: Instant,
    ) {
        if connection.f_binlog.is_some() && connection.is_still_logging() {
            Binlog::buffered_packet(connection, path_x, ptype, current_time);
        }
    }

    fn outgoing_packet(
        &mut self,
        connection: &mut Connection,
        path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: Instant,
    ) {
        if connection.f_binlog.is_some() && connection.is_still_logging() {
            Binlog::outgoing_packet(
                connection,
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
        connection: &mut Connection,
        path_x: &mut Path,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: Instant,
    ) {
        if connection.f_binlog.is_some() && connection.is_still_logging() {
            Binlog::packet_lost(
                connection,
                path_x,
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }
    }

    fn negotiated_alpn(
        &mut self,
        connection: &mut Connection,
        is_local: bool,
        sni: &[u8],
        alpn: &[u8],
        alpn_list: &[&[u8]],
    ) {
        if connection.f_binlog.is_some() {
            Binlog::negotiated_alpn(connection, is_local, sni, alpn, alpn_list);
        }
    }

    fn transport_extension(&mut self, connection: &mut Connection, is_local: bool, params: &[u8]) {
        if connection.f_binlog.is_some() {
            Binlog::transport_extension(connection, is_local, params);
        }
    }

    fn tls_ticket(&mut self, connection: &mut Connection, ticket: &[u8]) {
        if !connection.is_still_logging() {
            return;
        }
        let cid = connection.initial_connection_id;
        if let Some(f) = connection.f_binlog.as_mut() {
            crate::binlog::tls_ticket(f, cid, ticket);
        }
    }

    fn new_connection(&mut self, connection: &mut Connection) {
        Binlog::new_connection(connection);
    }

    fn close_connection(&mut self, connection: &mut Connection) {
        if connection.f_binlog.is_some() {
            Binlog::close_connection(connection);
        }
    }

    fn cc_dump(&mut self, connection: &mut Connection, path_x: &mut Path, current_time: Instant) {
        if connection.f_binlog.is_some() && connection.is_still_logging() {
            Binlog::cc_dump(connection, path_x, current_time);
        }
    }
}

/// C: `binlog_app_message` (picoquic/logwriter.c:1300)
///
/// Per-connection unified-log adapter: emit the app message only when this
/// connection currently owns an open binlog file.
#[allow(dead_code)]
fn binlog_app_message(connection: &mut Connection, args: core::fmt::Arguments<'_>) {
    if connection.f_binlog.is_some() {
        Binlog::message_v(connection, args);
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
        binlog_dir: Option<&(impl AsRef<FsPath> + ?Sized)>,
    ) -> Result<(), Error> {
        // Mirror C `quic->binlog_dir = strdup(binlog_dir)` (NULL-clear
        // path included): swap out the previous directory and store
        // the new one.
        self.binlog_dir = binlog_dir.map(|p| PathBuf::from(p.as_ref()));
        self.enable_binlog();
        Ok(())
    }

    /// No-op context-level close callback — binlog close is per-connection
    /// only (see [`Binlog::close_connection`]).  Wired into the C
    /// `binlog_functions` vtable so the unified-logging dispatcher has a
    /// consistent function-pointer shape; in Rust the per-context close path
    /// calls this directly when tearing down a [`Quic`] instance that has the
    /// binlog backend enabled.
    ///
    /// C: `picoquic/logwriter.c:1309-1315`.
    pub fn binlog_close(&self) {}

    /// Enable binary logging without setting a directory — used
    /// when autoqlog wants the binlog stream as scratch space.
    /// Equivalent to [`Quic::set_binlog`] minus the directory
    /// bookkeeping.
    ///
    /// C: `void picoquic_enable_binlog(picoquic_quic_t*)`.
    pub fn enable_binlog(&mut self) {
        // C: `quic->bin_log_fns = &binlog_functions;`.
        let logger = self
            .bin_log_fns
            .get_or_insert_with(|| {
                let logger: LoggerRef = Rc::new(RefCell::new(BinlogLogger));
                logger
            })
            .clone();
        for connection in self.connections.iter_mut() {
            connection.bin_log_fns = Some(logger.clone());
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn stream_frame_with_length_logs_length_varint_and_payload_preview() {
        let bytes = [0x0a, 0x01, 0x03, 0xaa, 0xbb, 0xcc, 0xdd];
        let mut out = Vec::new();

        let rest = log_stream_frame(&mut out, &bytes);

        assert_eq!(rest, Some(&bytes[6..]));
        assert_eq!(out, [0x06, 0x0a, 0x01, 0x03, 0xaa, 0xbb, 0xcc]);
    }

    #[test]
    fn zero_length_stream_frame_with_length_logs_encoded_length() {
        let bytes = [0x0a, 0x01, 0x00, 0xff];
        let mut out = Vec::new();

        let rest = log_stream_frame(&mut out, &bytes);

        assert_eq!(rest, Some(&bytes[3..]));
        assert_eq!(out, [0x03, 0x0a, 0x01, 0x00]);
    }

    #[test]
    fn short_stream_frame_logs_cautious_error_prefix() {
        let bytes = [0x0a];
        let mut out = Vec::new();

        let rest = log_stream_frame(&mut out, &bytes);

        assert_eq!(rest, None);
        assert_eq!(out, [0x01, 0x0a]);
    }

    #[test]
    fn outgoing_short_header_uses_connection_remote_cid_length() {
        let bytes = [0x64, 0x01, 0x02, 0x03, 0x04, 0xaa, 0xbb, 0xcc, 0xdd];

        let ph = parse_outgoing_header(&bytes, 4, 0, false, true);

        assert_eq!(ph.packet_type, PacketType::OneRttProtected);
        assert_eq!(
            ph.dest_connection_id,
            ConnectionId::clone_from_slice(&[0x01, 0x02, 0x03, 0x04]).unwrap()
        );
        assert_eq!(ph.offset, 5);
        assert_eq!(ph.packet_number_offset, 5);
        assert_eq!(ph.payload_length, 4);
        assert_eq!(ph.version_index, 0);
        assert_eq!(ph.epoch, Epoch::OneRtt);
        assert_eq!(ph.packet_context, PacketContext::Application);
        assert!(ph.has_spin_bit);
        assert!(ph.has_loss_bits);
        assert!(ph.spin);
        assert!(ph.key_phase);
    }

    #[test]
    fn outgoing_long_header_uses_version_specific_packet_type() {
        let bytes = [
            0xd3, 0x6b, 0x33, 0x43, 0xcf, 0x01, 0x11, 0x01, 0x22, 0x00, 0x05, 0xaa, 0xbb, 0xcc,
            0xdd, 0xee,
        ];

        let ph = parse_outgoing_header(&bytes, 0, 0, false, false);

        assert_eq!(ph.packet_type, PacketType::Initial);
        assert_eq!(ph.version, crate::internal::Version::V2 as u32);
        assert_eq!(ph.version_index, 1);
        assert_eq!(ph.offset, 11);
        assert_eq!(ph.packet_number_offset, 11);
        assert_eq!(ph.payload_length, 5);
        assert_eq!(ph.epoch, Epoch::Initial);
        assert_eq!(ph.packet_context, PacketContext::Initial);
    }
}
