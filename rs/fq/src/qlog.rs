//! Translation of `quic/qlog.h` plus the qlog encoding helpers
//! mirrored from `loglib/qlog.c` and `loglib/qlog_frames.c`.
//!
//! The C library splits qlog into two layers: the public hook
//! `picoquic_set_qlog` that turns on per-connection qlog writers
//! (the unified-logging vtable in `loglib/qlog_fns.c`), and a body
//! of pure formatting helpers that JSON-encode bytestreams into a
//! qlog file.  This module exposes both: the [`Quic::set_qlog`]
//! entry point stores the directory on the context, and the free
//! functions below implement the byte-stream formatters that the
//! `qlog_error_test` suite exercises directly.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::frames::FrameType;
use crate::internal::{frames_varint_decode, varint_decode};
use crate::tp::TransportParameter;
use crate::{Error, Quic, Result};

impl Quic {
    /// Enable qlog tracing on this context, writing one qlog file per
    /// connection into `qlog_dir`.  Installs the unified-logging
    /// vtable; subsequent connections on this context will stream
    /// qlog records until the context is dropped.
    ///
    /// Both observed call sites guard on the directory being set
    /// before calling, so the parameter is borrowed (no
    /// `Option<…>`); the C `int` status (0 / -1) becomes
    /// `Result<()>`.
    ///
    /// C: `int set_qlog(Quic*, char const*)`.
    pub fn set_qlog(&mut self, qlog_dir: &(impl AsRef<Path> + ?Sized)) -> Result<()> {
        // C: quic->qlog_fns = &qlog_fns; quic->qlog_dir = strdup(qlog_dir).
        // The vtable install lives in `loglib/qlog_fns.c` (out of v1
        // scope); store the directory so connections that look at
        // `quic->qlog_dir` for the autoqlog filename find it.
        self.qlog_dir = Some(PathBuf::from(qlog_dir.as_ref()));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal qlog encoding helpers (used by qlog_error_test).

fn io<W: Write>(out: &mut W, args: core::fmt::Arguments<'_>) -> Result<()> {
    out.write_fmt(args).map_err(|_| Error::Generic)
}

/// Write a QLOG-escaped string of `l` bytes from `s`.  Returns `Err` when
/// the stream had fewer bytes available than `l` (truncation).
/// C: `qlog_string` in `loglib/qlog.c`.
pub fn qlog_string<W: Write>(out: &mut W, s: &[u8], l: usize) -> Result<()> {
    let error_found = l > s.len();
    let avail = s.len().min(l);
    io(out, format_args!("\""))?;
    for &b in &s[..avail] {
        io(out, format_args!("{:02x}", b))?;
    }
    if error_found {
        io(out, format_args!("... coding error!"))?;
    }
    io(out, format_args!("\""))?;
    if error_found {
        Err(Error::InvalidFrame)
    } else {
        Ok(())
    }
}

/// Write printable-ASCII QLOG characters from `s` up to `l` bytes.  Returns
/// `Err` when the stream had fewer bytes than `l`.
/// C: `qlog_chars` in `loglib/qlog.c`.
pub fn qlog_chars<W: Write>(out: &mut W, s: &[u8], l: usize) -> Result<()> {
    let error_found = l > s.len();
    let avail = s.len().min(l);
    io(out, format_args!("\""))?;
    for &c in &s[..avail] {
        if c == b'"' || c == b'\\' {
            io(out, format_args!("\\{}", c as char))?;
        } else if (b' '..127).contains(&c) {
            io(out, format_args!("{}", c as char))?;
        } else {
            io(out, format_args!("\\{:02x}", c))?;
        }
    }
    if error_found {
        io(out, format_args!("... coding error!"))?;
    }
    io(out, format_args!("\""))?;
    if error_found {
        Err(Error::InvalidFrame)
    } else {
        Ok(())
    }
}

/// Read one byte at `s[*pos]`, advancing `*pos`.  Mirrors C
/// `byteread_int8`.
fn read_u8(s: &[u8], pos: &mut usize) -> Option<u8> {
    if *pos >= s.len() {
        return None;
    }
    let b = s[*pos];
    *pos += 1;
    Some(b)
}

/// Read a big-endian u16 at `s[*pos..*pos+2]`, advancing `*pos`.
/// Mirrors C `byteread_int16`.
fn read_u16_be(s: &[u8], pos: &mut usize) -> Option<u16> {
    if *pos + 2 > s.len() {
        return None;
    }
    let v = u16::from_be_bytes([s[*pos], s[*pos + 1]]);
    *pos += 2;
    Some(v)
}

/// Internal `qlog_string` variant that walks a cursor over `s[..limit]`,
/// emitting `l` hex bytes from the cursor (and a `... coding error!`
/// marker on truncation).  Used by the preferred-address /
/// transport-extensions helpers that need to mutate the cursor as they
/// emit JSON.
fn qlog_string_cursor<W: Write>(
    out: &mut W,
    s: &[u8],
    pos: &mut usize,
    limit: usize,
    l: usize,
) -> Result<()> {
    let error_found = *pos + l > limit;
    io(out, format_args!("\""))?;
    let mut written = 0;
    while written < l && *pos < limit {
        io(out, format_args!("{:02x}", s[*pos]))?;
        *pos += 1;
        written += 1;
    }
    if error_found {
        io(out, format_args!("... coding error!"))?;
    }
    io(out, format_args!("\""))?;
    if error_found {
        Err(Error::InvalidFrame)
    } else {
        Ok(())
    }
}

/// Write a QLOG preferred-address extension from `s` of `len` bytes.
/// C: `qlog_preferred_address` in `loglib/qlog.c`.
pub fn qlog_preferred_address<W: Write>(out: &mut W, s: &[u8], len: usize) {
    let limit = s.len().min(len);
    let mut pos = 0usize;

    // IPv4 address (4 bytes) + IPv4 port (2 bytes BE).
    let _ = io(out, format_args!("\"ip_v4\": \""));
    for i in 0..4 {
        if pos >= limit {
            break;
        }
        let sep = if i == 0 { "" } else { "." };
        let _ = io(out, format_args!("{}{}", sep, s[pos]));
        pos += 1;
    }
    let port4 = read_u16_be(s, &mut pos).unwrap_or(0);
    let _ = io(out, format_args!("\", \"port_v4\":{}", port4));

    // IPv6 address (8 hex shorts) + IPv6 port (2 bytes BE).
    let _ = io(out, format_args!(", \"ip_v6\": \""));
    for i in 0..8 {
        let sep = if i == 0 { "" } else { ":" };
        let chunk = read_u16_be(s, &mut pos).unwrap_or(0);
        let _ = io(out, format_args!("{}{:x}", sep, chunk));
    }
    let port6 = read_u16_be(s, &mut pos).unwrap_or(0);
    let _ = io(out, format_args!("\", \"port_v6\" : {}", port6));

    // Connection ID (length byte + bytes) and 16-byte stateless reset
    // token; trailing slack is dumped under "extra_bytes".
    let cid_len = read_u8(s, &mut pos).unwrap_or(0) as usize;
    let _ = io(out, format_args!(", \"connection_id\": "));
    let _ = qlog_string_cursor(out, s, &mut pos, limit, cid_len);
    let _ = io(out, format_args!(", \"stateless_reset_token\": "));
    let _ = qlog_string_cursor(out, s, &mut pos, limit, 16);
    if pos < limit {
        // C: emits a leading `"` here too — preserved verbatim.
        let _ = io(out, format_args!("\", \"extra_bytes\": "));
        let remaining = limit - pos;
        let _ = qlog_string_cursor(out, s, &mut pos, limit, remaining);
    }
}

/// Write a QLOG version-negotiation TP from `s` of `len` bytes.
/// C: `qlog_tp_version_negotiation` in `loglib/qlog.c`.
pub fn qlog_tp_version_negotiation<W: Write>(out: &mut W, s: &[u8], len: usize) {
    if len > s.len() {
        let _ = io(
            out,
            format_args!(",\n    \"vnego_parameter_length\": {}", len),
        );
        let _ = io(out, format_args!(",\n    \"bytes_available\": {}", s.len()));
        return;
    }

    let limit = len;
    let mut pos = 0usize;
    let _ = io(out, format_args!("{{ "));
    if (len & 3) != 0 || len == 0 {
        let _ = io(out, format_args!("\"bad_length\": \"{}", len));
    } else {
        let _ = io(out, format_args!("\"chosen\": \""));
        for _ in 0..4 {
            if pos >= limit {
                break;
            }
            let _ = io(out, format_args!("{:02x}", s[pos]));
            pos += 1;
        }
        let _ = io(out, format_args!("\""));
        if pos < limit {
            let mut is_first = true;
            let _ = io(out, format_args!(", \"others\": ["));
            loop {
                let sep = if is_first { "\"" } else { ", \"" };
                let _ = io(out, format_args!("{}", sep));
                is_first = false;
                for _ in 0..4 {
                    if pos >= limit {
                        break;
                    }
                    let _ = io(out, format_args!("{:02x}", s[pos]));
                    pos += 1;
                }
                let _ = io(out, format_args!("\""));
                if pos >= limit {
                    break;
                }
            }
            let _ = io(out, format_args!("]"));
        }
    }
    let _ = io(out, format_args!("}}"));
}

fn qlog_vint_transport_extension<W: Write>(
    out: &mut W,
    ext_name: &str,
    s: &[u8],
    pos: &mut usize,
    limit: usize,
    len: u64,
) -> Result<()> {
    let saved_ptr = *pos;
    let mut val: u64 = 0;
    let consumed = if *pos < limit {
        varint_decode(&s[*pos..limit], &mut val)
    } else {
        0
    };
    let ret = if consumed == 0 {
        -1
    } else {
        *pos += consumed;
        0
    };

    io(out, format_args!("\"{}\" : ", ext_name))?;
    if ret != 0 || saved_ptr + len as usize != *pos {
        *pos = saved_ptr;
        qlog_string_cursor(out, s, pos, limit, len as usize)?;
    } else {
        io(out, format_args!("{}", val))?;
    }
    Ok(())
}

fn qlog_boolean_transport_extension<W: Write>(
    out: &mut W,
    ext_name: &str,
    s: &[u8],
    pos: &mut usize,
    limit: usize,
    len: u64,
) -> Result<()> {
    io(out, format_args!("\"{}\" : ", ext_name))?;
    if len != 0 {
        qlog_string_cursor(out, s, pos, limit, len as usize)?;
    } else {
        io(out, format_args!("\"\""))?;
    }
    Ok(())
}

fn tp_name_for(tp: u64) -> String {
    TransportParameter::name(tp)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{:x}", tp))
}

/// Write QLOG transport extensions from `s` up to `tp_length` bytes.
/// C: `qlog_transport_extensions` in `loglib/qlog.c`.
pub fn qlog_transport_extensions<W: Write>(out: &mut W, s: &[u8], tp_length: usize) -> Result<()> {
    if tp_length > s.len() {
        io(
            out,
            format_args!(",\n    \"transport_parameter_length\": {}", tp_length),
        )?;
        io(out, format_args!(",\n    \"bytes_available\": {}", s.len()))?;
        return Ok(());
    }

    // C: parameter-coding errors set `ret = -1` and break the loop,
    // but the top-level test ignores that return; the JSON output
    // already records the truncation, so only IO failures surface as
    // `Err` here.
    let limit = tp_length;
    let mut pos = 0usize;

    while pos < limit {
        let saved_ptr = pos;

        let extension_type_opt = {
            let mut val: u64 = 0;
            let consumed = if pos < limit {
                varint_decode(&s[pos..limit], &mut val)
            } else {
                0
            };
            if consumed == 0 {
                None
            } else {
                pos += consumed;
                Some(val)
            }
        };

        let extension_length_opt = if extension_type_opt.is_some() {
            let mut val: u64 = 0;
            let consumed = if pos < limit {
                varint_decode(&s[pos..limit], &mut val)
            } else {
                0
            };
            if consumed == 0 {
                None
            } else {
                pos += consumed;
                Some(val)
            }
        } else {
            None
        };

        io(out, format_args!(",\n    "))?;

        let (extension_type, extension_length) = match (extension_type_opt, extension_length_opt) {
            (Some(t), Some(l)) if (limit - pos) as u64 >= l => (t, l),
            _ => {
                pos = saved_ptr;
                let len = limit - pos;
                io(out, format_args!("\"Parameter_coding_error\": "))?;
                let _ = qlog_string_cursor(out, s, &mut pos, limit, len);
                break;
            }
        };

        // RFC 9000 + extensions transport-parameter dispatch (mirrors
        // the switch in the C `qlog_transport_extensions`).
        match extension_type {
            // Variable-length integer parameters.
            5 | 6 | 7 | 4 | 8 | 1 | 3 | 10 | 9 | 11 | 14 | 32 | 0x1057 | 0xff04de1b | 0xebd9
            | 0x3e | 0x9f81a176 | 0x17f7586d2cb571 => {
                let name = tp_name_for(extension_type);
                qlog_vint_transport_extension(out, &name, s, &mut pos, limit, extension_length)?;
            }
            // Bytestring parameters.
            2 | 0 | 16 | 15 => {
                let name = tp_name_for(extension_type);
                io(out, format_args!("\"{}\": ", name))?;
                let _ = qlog_string_cursor(out, s, &mut pos, limit, extension_length as usize);
            }
            // Server preferred address.
            13 => {
                let name = tp_name_for(extension_type);
                io(out, format_args!("\"{}\": ", name))?;
                let end = (pos + extension_length as usize).min(limit);
                qlog_preferred_address(out, &s[pos..end], extension_length as usize);
                pos = end;
                io(out, format_args!("}}"))?;
            }
            // Boolean parameters.
            12 | 0x7158 | 0x2ab2 => {
                let name = tp_name_for(extension_type);
                qlog_boolean_transport_extension(out, &name, s, &mut pos, limit, extension_length)?;
            }
            // Version negotiation.
            0x11 => {
                let name = tp_name_for(extension_type);
                io(out, format_args!("\"{}\": ", name))?;
                let end = (pos + extension_length as usize).min(limit);
                qlog_tp_version_negotiation(out, &s[pos..end], extension_length as usize);
                pos = end;
            }
            // Unknown extensions: dump as hex with the numeric ID.
            _ => {
                io(out, format_args!("\"{:x}\": ", extension_type))?;
                let _ = qlog_string_cursor(out, s, &mut pos, limit, extension_length as usize);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// qlog_frames — JSON frame dump from `loglib/qlog_frames.c`.

fn frame_name_for(ftype: u64) -> String {
    FrameType::name(ftype)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn json_uint<W: Write>(out: &mut W, key: &str, value: u64) -> Result<()> {
    io(out, format_args!("\"{}\": {}", key, value))
}

fn json_str<W: Write>(out: &mut W, key: &str, value: &str) -> Result<()> {
    io(out, format_args!("\"{}\": \"{}\"", key, value))
}

fn json_bool<W: Write>(out: &mut W, key: &str, value: bool) -> Result<()> {
    let v = if value { "true" } else { "false" };
    io(out, format_args!("\"{}\": {}", key, v))
}

fn frame_hex_string<'a, W: Write>(out: &mut W, bytes: &'a [u8], l: u64) -> Option<&'a [u8]> {
    let l = l as usize;
    let _ = io(out, format_args!("\""));
    if l > bytes.len() {
        let _ = io(out, format_args!("... coding error!"));
        let _ = io(out, format_args!("\""));
        None
    } else {
        for b in &bytes[..l] {
            let _ = io(out, format_args!("{:02x}", b));
        }
        let _ = io(out, format_args!("\""));
        Some(&bytes[l..])
    }
}

fn frame_one_param<'a, W: Write>(out: &mut W, bytes: &'a [u8], name: &str) -> Option<&'a [u8]> {
    let mut v: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut v)?;
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, name, v);
    Some(rest)
}

fn frame_two_params<'a, W: Write>(
    out: &mut W,
    bytes: &'a [u8],
    n1: &str,
    n2: &str,
) -> Option<&'a [u8]> {
    let mut a: u64 = 0;
    let mut b: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut a)?;
    let rest = frames_varint_decode(rest, &mut b)?;
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, n1, a);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, n2, b);
    Some(rest)
}

fn frame_stream<'a, W: Write>(out: &mut W, first: &'a [u8]) -> Option<&'a [u8]> {
    if first.is_empty() {
        return None;
    }
    let first_byte = first[0];
    let has_len = first_byte & 2 != 0;
    let has_off = first_byte & 4 != 0;
    let fin = first_byte & 1 != 0;
    let mut pos = 1usize;
    let mut stream_id: u64 = 0;
    let mut offset: u64 = 0;

    let consumed = if pos < first.len() {
        varint_decode(&first[pos..], &mut stream_id)
    } else {
        0
    };
    if consumed == 0 {
        return None;
    }
    pos += consumed;

    if has_off {
        let c = if pos < first.len() {
            varint_decode(&first[pos..], &mut offset)
        } else {
            0
        };
        if c == 0 {
            return None;
        }
        pos += c;
    }

    let length: u64 = if has_len {
        let mut val: u64 = 0;
        let c = if pos < first.len() {
            varint_decode(&first[pos..], &mut val)
        } else {
            0
        };
        if c == 0 || pos + c + val as usize > first.len() {
            return None;
        }
        pos += c;
        val
    } else {
        (first.len() - pos) as u64
    };

    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "id", stream_id);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "offset", offset);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "length", length);
    let _ = io(out, format_args!(", "));
    let _ = json_bool(out, "fin", fin);
    // C: trailing space for old-log compatibility.
    let _ = io(out, format_args!(" "));

    if !has_len {
        let _ = io(out, format_args!(", \"has_length\": false"));
    }

    let mut extra = 8u64.min(length);
    if extra > (first.len() - pos) as u64 {
        extra = (first.len() - pos) as u64;
    }
    if extra > 0 {
        let _ = io(out, format_args!(", \"begins_with\": "));
        let _ = frame_hex_string(out, &first[pos..], extra);
    }

    let advance = (pos as u64).saturating_add(length) as usize;
    if advance > first.len() {
        return None;
    }
    Some(&first[advance..])
}

fn frame_ack<'a, W: Write>(
    out: &mut W,
    first: &'a [u8],
    has_path_id: bool,
    has_ecn: bool,
) -> Option<&'a [u8]> {
    if first.is_empty() {
        return None;
    }
    // C: byte_index = picoquic_decode_varint_length(bytes[0]).
    let mut pos = 1usize << ((first[0] & 0xC0) >> 6);
    let mut path_id: u64 = 0;
    let mut largest: u64 = 0;
    let mut ack_delay: u64 = 0;
    let mut num_block: u64 = 0;

    if has_path_id {
        let c = if pos < first.len() {
            varint_decode(&first[pos..], &mut path_id)
        } else {
            0
        };
        if c == 0 {
            return None;
        }
        pos += c;
    }
    let c = if pos < first.len() {
        varint_decode(&first[pos..], &mut largest)
    } else {
        0
    };
    if c == 0 {
        return None;
    }
    pos += c;
    let c = if pos < first.len() {
        varint_decode(&first[pos..], &mut ack_delay)
    } else {
        0
    };
    if c == 0 {
        return None;
    }
    pos += c;
    let c = if pos < first.len() {
        varint_decode(&first[pos..], &mut num_block)
    } else {
        0
    };
    if c == 0 {
        return None;
    }
    pos += c;

    if has_path_id {
        let _ = io(out, format_args!(", \"path_id\": {}", path_id));
    }
    let _ = io(out, format_args!(", \"ack_delay\": {}", ack_delay));
    let _ = io(out, format_args!(", \"acked_ranges\": ["));

    let mut bytes: &[u8] = &first[pos..];
    let mut broke = false;
    for i in 0..=num_block {
        let mut skip: u64 = 0;
        let mut range: u64 = 0;
        if i != 0 {
            match frames_varint_decode(bytes, &mut skip) {
                Some(rest) => bytes = rest,
                None => {
                    let _ = io(out, format_args!("[-1, -1]"));
                    broke = true;
                    break;
                }
            }
        }
        match frames_varint_decode(bytes, &mut range) {
            Some(rest) => bytes = rest,
            None => {
                let _ = io(out, format_args!("[-1, -1]"));
                broke = true;
                break;
            }
        }
        if largest < skip + range {
            let _ = io(out, format_args!("[-1, -1]"));
            broke = true;
            break;
        }
        if i != 0 {
            let s = skip + 1;
            largest -= s;
            let _ = io(out, format_args!(", "));
        }
        let start_range = (largest as i64) - (range as i64);
        let end_range = largest as i64;
        let _ = io(out, format_args!("[{}, {}]", start_range, end_range));
        largest -= range + 1;
    }
    let _ = io(out, format_args!("]"));

    if has_ecn && !broke {
        const ECN_NAME: [&str; 3] = ["ect0", "ect1", "ce"];
        for ecn_name in ECN_NAME.iter() {
            let mut v: u64 = 0;
            match frames_varint_decode(bytes, &mut v) {
                Some(rest) => {
                    bytes = rest;
                    let _ = io(out, format_args!(", \"{}\": {}", ecn_name, v));
                }
                None => break,
            }
        }
    }

    if broke { None } else { Some(bytes) }
}

fn frame_reset<'a, W: Write>(out: &mut W, bytes: &'a [u8], is_at: bool) -> Option<&'a [u8]> {
    let mut stream_id: u64 = 0;
    let mut error_code: u64 = 0;
    let mut final_size: u64 = 0;
    let mut reliable_size: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut stream_id)?;
    let rest = frames_varint_decode(rest, &mut error_code)?;
    let rest = frames_varint_decode(rest, &mut final_size)?;
    let rest = if is_at {
        frames_varint_decode(rest, &mut reliable_size)?
    } else {
        rest
    };
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "stream_id", stream_id);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "error_code", error_code);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "final_size", final_size);
    if is_at {
        let _ = io(out, format_args!(", "));
        let _ = json_uint(out, "reliable_size", reliable_size);
    }
    Some(rest)
}

fn frame_connection_close<'a, W: Write>(
    out: &mut W,
    bytes: &'a [u8],
    app_error: bool,
) -> Option<&'a [u8]> {
    let mut error_code: u64 = 0;
    let mut trigger_frame_type: u64 = 0;
    let mut reason_length: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut error_code)?;
    let rest = if app_error {
        rest
    } else {
        frames_varint_decode(rest, &mut trigger_frame_type)?
    };
    let mut rest = frames_varint_decode(rest, &mut reason_length)?;

    let space = if app_error {
        "application"
    } else {
        "transport"
    };
    let _ = io(out, format_args!(", \"error_space\": \"{}\", ", space));
    let _ = json_uint(out, "error_code", error_code);
    if !app_error && trigger_frame_type != 0 {
        match FrameType::name(trigger_frame_type) {
            Some(name) => {
                let _ = io(out, format_args!(", \"trigger_frame_type\": \"{}\"", name));
            }
            None => {
                let _ = io(
                    out,
                    format_args!(", \"trigger_frame_type\": \"{:x}\"", trigger_frame_type),
                );
            }
        }
    }
    if reason_length > 0 {
        if (rest.len() as u64) >= reason_length {
            let _ = io(out, format_args!(", \"reason\": \""));
            for &c in &rest[..reason_length as usize] {
                let ch = if (0x20..=0x7e).contains(&c) {
                    c as char
                } else {
                    '.'
                };
                let _ = io(out, format_args!("{}", ch));
            }
            let _ = io(out, format_args!("\""));
            rest = &rest[reason_length as usize..];
        } else {
            let _ = io(out, format_args!(", \"reason\": \"encoding error\""));
            return None;
        }
    }
    Some(rest)
}

fn frame_max_streams<'a, W: Write>(
    out: &mut W,
    bytes: &'a [u8],
    is_bidir: bool,
    is_blocked: bool,
) -> Option<&'a [u8]> {
    let _ = io(out, format_args!(", "));
    let stype = if is_bidir {
        "bidirectional"
    } else {
        "unidirectional"
    };
    let _ = json_str(out, "stream_type", stype);
    let mut v: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut v)?;
    let _ = io(out, format_args!(", "));
    let key = if is_blocked { "limit" } else { "maximum" };
    let _ = json_uint(out, key, v);
    Some(rest)
}

fn frame_new_connection_id<'a, W: Write>(
    out: &mut W,
    bytes: &'a [u8],
    has_path_id: bool,
) -> Option<&'a [u8]> {
    let mut path_id: u64 = 0;
    let mut sequence_number: u64 = 0;
    let mut retire_prior_to: u64 = 0;

    let mut rest = bytes;
    if has_path_id {
        rest = frames_varint_decode(rest, &mut path_id)?;
        let _ = io(out, format_args!(", "));
        let _ = json_uint(out, "path_id", path_id);
    }
    rest = frames_varint_decode(rest, &mut sequence_number)?;
    rest = frames_varint_decode(rest, &mut retire_prior_to)?;
    if rest.is_empty() {
        return None;
    }
    let cid_length = rest[0];
    rest = &rest[1..];

    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "sequence_number", sequence_number);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "retire_before", retire_prior_to);
    let _ = io(out, format_args!(", \"connection_id\": "));
    let rest = frame_hex_string(out, rest, cid_length as u64)?;
    let _ = io(out, format_args!(", \"reset_token\": "));
    frame_hex_string(out, rest, 16)
}

fn frame_crypto_hs<'a, W: Write>(out: &mut W, bytes: &'a [u8]) -> Option<&'a [u8]> {
    let mut offset: u64 = 0;
    let mut length: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut offset)?;
    let rest = frames_varint_decode(rest, &mut length)?;
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "offset", offset);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "length", length);
    if (rest.len() as u64) >= length {
        Some(&rest[length as usize..])
    } else {
        None
    }
}

fn frame_new_token<'a, W: Write>(out: &mut W, bytes: &'a [u8]) -> Option<&'a [u8]> {
    let mut tok_len: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut tok_len)?;
    let _ = io(out, format_args!(", \"new_token\": "));
    frame_hex_string(out, rest, tok_len)
}

fn frame_path_challenge_response<'a, W: Write>(
    out: &mut W,
    bytes: &'a [u8],
    is_challenge: bool,
) -> Option<&'a [u8]> {
    if bytes.len() < 8 {
        return None;
    }
    let label = if is_challenge {
        "path_challenge"
    } else {
        "path_response"
    };
    let _ = io(out, format_args!(", \"{}\": ", label));
    frame_hex_string(out, bytes, 8)
}

fn frame_datagram<'a, W: Write>(
    out: &mut W,
    bytes: &'a [u8],
    has_length: bool,
) -> Option<&'a [u8]> {
    if has_length {
        let mut length: u64 = 0;
        let rest = frames_varint_decode(bytes, &mut length)?;
        if (rest.len() as u64) < length {
            return None;
        }
        let _ = io(out, format_args!(", "));
        let _ = json_uint(out, "length", length);
        Some(&rest[length as usize..])
    } else {
        // C: bytes = bytes_max — consume the rest of the buffer.
        Some(&bytes[bytes.len()..])
    }
}

fn frame_ack_frequency<'a, W: Write>(out: &mut W, bytes: &'a [u8]) -> Option<&'a [u8]> {
    let mut a: u64 = 0;
    let mut b: u64 = 0;
    let mut c: u64 = 0;
    let mut d: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut a)?;
    let rest = frames_varint_decode(rest, &mut b)?;
    let rest = frames_varint_decode(rest, &mut c)?;
    let rest = frames_varint_decode(rest, &mut d)?;
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "sequence_number", a);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "packet_tolerance", b);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "max_ack_delay", c);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "reordering_threshold", d);
    Some(rest)
}

fn frame_ip_address<W: Write>(out: &mut W, addr: &[u8]) {
    let _ = io(out, format_args!("\""));
    if addr.len() == 4 {
        for (x, &b) in addr.iter().enumerate() {
            let sep = if x == 0 { "" } else { "." };
            let _ = io(out, format_args!("{}{}", sep, b));
        }
    } else if addr.len() == 16 {
        for x in 0..8 {
            let w = u16::from_be_bytes([addr[2 * x], addr[2 * x + 1]]);
            let sep = if x == 0 { "" } else { ":" };
            let _ = io(out, format_args!("{}{:x}", sep, w));
        }
    } else {
        let _ = io(out, format_args!("invalid address length {}", addr.len()));
    }
    let _ = io(out, format_args!("\""));
}

fn frame_bdp<'a, W: Write>(out: &mut W, bytes: &'a [u8]) -> Option<&'a [u8]> {
    let mut lifetime: u64 = 0;
    let mut bif: u64 = 0;
    let mut min_rtt: u64 = 0;
    let mut ip_len: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut lifetime)?;
    let rest = frames_varint_decode(rest, &mut bif)?;
    let rest = frames_varint_decode(rest, &mut min_rtt)?;
    let rest = frames_varint_decode(rest, &mut ip_len)?;
    // C side restricts saved_ip to length 4 (the would-be 16 branch is
    // unreachable thanks to a pre-existing `(ip_len != 4 && ip_len !=
    // 4)` typo); preserve the test exactly.
    if ip_len != 4 || (rest.len() as u64) < ip_len {
        return None;
    }
    let saved_ip = &rest[..ip_len as usize];
    let rest = &rest[ip_len as usize..];

    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "lifetime", lifetime);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "recon_bytes_in_flight", bif);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "recon_min_rtt", min_rtt);
    let _ = io(out, format_args!(", \"saved_ip\": "));
    frame_ip_address(out, saved_ip);
    Some(rest)
}

fn frame_observed_address<'a, W: Write>(
    out: &mut W,
    bytes: &'a [u8],
    ftype: u64,
) -> Option<&'a [u8]> {
    let mut sequence: u64 = 0;
    let rest = frames_varint_decode(bytes, &mut sequence)?;
    let addr_len = if (ftype & 1) == 0 { 4 } else { 16 };
    if rest.len() < addr_len + 2 {
        return None;
    }
    let addr = &rest[..addr_len];
    let port = u16::from_be_bytes([rest[addr_len], rest[addr_len + 1]]);
    let rest = &rest[addr_len + 2..];

    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "sequence", sequence);
    let _ = io(out, format_args!(", \"address\": "));
    frame_ip_address(out, addr);
    let _ = io(out, format_args!(", "));
    let _ = json_uint(out, "port", port as u64);
    Some(rest)
}

/// Write QLOG frame descriptions for all frames in `bytes[..bytes_max]`.
/// Returns a slice pointing one byte past the last consumed byte.
/// C: `qlog_frames` in `loglib/qlog_frames.c`.
pub fn qlog_frames<'a, W: Write>(out: &mut W, bytes: &'a [u8], skip_padding: bool) -> &'a [u8] {
    let mut comma_if_needed = "";
    let mut cursor: &[u8] = bytes;
    let mut last: &[u8] = bytes;

    while !cursor.is_empty() {
        let first_byte_pos = cursor;
        let mut frame_id: u64 = 0;
        let after_id = match frames_varint_decode(cursor, &mut frame_id) {
            Some(rest) => rest,
            None => break,
        };

        // Padding fast-path: collapse a run of 0x00 bytes silently.
        if frame_id == FrameType::Padding as u64 && skip_padding {
            cursor = after_id;
            while !cursor.is_empty() && cursor[0] == 0 {
                cursor = &cursor[1..];
            }
            last = cursor;
            continue;
        }

        let _ = io(
            out,
            format_args!(
                "{}{{ \n    \"frame_type\": \"{}\"",
                comma_if_needed,
                frame_name_for(frame_id)
            ),
        );
        comma_if_needed = ", ";

        let stream_min = FrameType::StreamRangeMin as u64;
        let stream_max = FrameType::StreamRangeMax as u64;
        let next: Option<&[u8]> = if (stream_min..=stream_max).contains(&frame_id) {
            frame_stream(out, first_byte_pos)
        } else {
            match frame_id {
                // Padding (no skip): collapse trailing zero bytes too.
                0x00 => {
                    let mut c = after_id;
                    while !c.is_empty() && c[0] == 0 {
                        c = &c[1..];
                    }
                    Some(c)
                }
                0x01 | 0x1e | 0x1f => Some(after_id), // ping / handshake_done / immediate_ack
                0x02 => frame_ack(out, first_byte_pos, false, false),
                0x03 => frame_ack(out, first_byte_pos, false, true),
                0x3e => frame_ack(out, first_byte_pos, true, false),
                0x3f => frame_ack(out, first_byte_pos, true, true),
                0x04 => frame_reset(out, after_id, false),
                0x24 => frame_reset(out, after_id, true),
                0x1c => frame_connection_close(out, after_id, false),
                0x1d => frame_connection_close(out, after_id, true),
                0x10 => frame_one_param(out, after_id, "maximum"),
                0x11 => frame_two_params(out, after_id, "stream_id", "maximum"),
                0x12 => frame_max_streams(out, after_id, true, false),
                0x13 => frame_max_streams(out, after_id, false, false),
                0x14 => frame_one_param(out, after_id, "limit"),
                0x15 => frame_two_params(out, after_id, "stream_id", "limit"),
                0x16 => frame_max_streams(out, after_id, true, true),
                0x17 => frame_max_streams(out, after_id, false, true),
                0x18 => frame_new_connection_id(out, after_id, false),
                0x3e78 => frame_new_connection_id(out, after_id, true),
                0x05 => frame_two_params(out, after_id, "stream_id", "error_code"),
                0x1a => frame_path_challenge_response(out, after_id, true),
                0x1b => frame_path_challenge_response(out, after_id, false),
                0x06 => frame_crypto_hs(out, after_id),
                0x07 => frame_new_token(out, after_id),
                0x19 => frame_one_param(out, after_id, "sequence_number"),
                0x3e79 => frame_two_params(out, after_id, "path_id", "sequence_number"),
                0x30 => frame_datagram(out, after_id, false),
                0x31 => frame_datagram(out, after_id, true),
                0xaf => frame_ack_frequency(out, after_id),
                757 => frame_one_param(out, after_id, "time_stamp"),
                0x3e75 => frame_two_params(out, after_id, "path_id", "reason"),
                0x3e76 => frame_two_params(out, after_id, "path_id", "sequence"),
                0x3e77 => frame_two_params(out, after_id, "path_id", "sequence"),
                0xebd9 => frame_bdp(out, after_id),
                0x3e7a => frame_one_param(out, after_id, "max_path_id"),
                0x3e7b => frame_one_param(out, after_id, "max_path_id"),
                0x3e7c => frame_two_params(out, after_id, "path_id", "next_sequence_number"),
                0x9f81a6 | 0x9f81a7 => frame_observed_address(out, after_id, frame_id),
                _ => {
                    let _ = io(out, format_args!(", "));
                    let _ = json_uint(out, "unknown_frame_type", frame_id);
                    Some(after_id)
                }
            }
        };

        let _ = io(out, format_args!("}}"));

        match next {
            Some(rest) => {
                cursor = rest;
                last = rest;
            }
            None => break,
        }
    }

    last
}

#[cfg(test)]
mod test {}
