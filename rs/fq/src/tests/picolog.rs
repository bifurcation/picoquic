//! Test cases for `picoquictest/picolog*.c`.
//!
//! `picolog_basic_test` exercises `cidset`, `logreader`, and `svg_convert`
//! from `loglib/`.  The small helpers below mirror the C routines needed by
//! this test case.

#![allow(non_snake_case)]

use std::fmt::Write as FmtWrite;
use std::io::{BufRead, Read, Seek, SeekFrom, Write as IoWrite};

use super::util::compare_text_files;
use crate::Instant;
use crate::bytestream::{BYTESTREAM_MAX_BUFFER_SIZE, ByteStream};
use crate::frames::FrameType;
use crate::internal::PacketType;

// ---------------------------------------------------------------------------
// Log conversion helpers.

/// Opaque handle to a set of connection IDs read from a binlog.
/// C: `picohash_table*` (cidset).
struct CidSet {
    cids: Vec<crate::ConnectionId>,
    read_failed: bool,
}

impl CidSet {
    fn is_empty(&self) -> bool {
        self.cids.is_empty()
    }
}

/// Create an empty [`CidSet`].  C: `cidset_create`.
fn cidset_create() -> Option<Box<CidSet>> {
    Some(Box::new(CidSet {
        cids: Vec::new(),
        read_failed: false,
    }))
}

/// Parse `f_binlog` and populate `cids` with every CID observed.
/// C: `binlog_list_cids`.
fn binlog_list_cids(f_binlog: &mut std::fs::File, cids: &mut CidSet) {
    cids.read_failed = fileread_binlog(f_binlog, |s| {
        let cid = s.read_cid()?;
        if !cids.cids.iter().any(|known| known == &cid) {
            cids.cids.push(cid);
        }
        Ok(())
    })
    .is_err();
}

/// Write a human-readable dump of `cids` to `out`.
/// C: `cidset_print`.
fn cidset_print(out: &mut std::fs::File, cids: &CidSet) {
    for cid in &cids.cids {
        let _ = write!(out, "  <");
        for byte in cid.as_bytes() {
            let _ = write!(out, "{byte:02x}");
        }
        let _ = writeln!(out, ">");
    }
}

/// Return `true` if `cid` appears in `cids`.
/// C: `cidset_has_cid`.
fn cidset_has_cid(cids: &CidSet, cid: &crate::ConnectionId) -> bool {
    cids.cids.iter().any(|known| known == cid)
}

/// Iterate over every CID in `cids`, calling `f(cid, ctx)` for each.
/// C: `cidset_iterate`.
fn cidset_iterate(
    cids: &CidSet,
    f: fn(&crate::ConnectionId, &mut SvgConvertCtx) -> i32,
    ctx: &mut SvgConvertCtx,
) -> i32 {
    if cids.read_failed {
        return -1;
    }
    for cid in &cids.cids {
        let ret = f(cid, ctx);
        if ret != 0 {
            return ret;
        }
    }
    0
}

/// Context passed to the per-CID conversion callback.
/// C: `app_conversion_context_t`.
#[allow(dead_code)]
struct SvgConvertCtx {
    binlog_name: String,
    out_dir: &'static str,
    f_binlog: Option<std::fs::File>,
    f_template: Option<std::fs::File>,
}

/// Convert one connection's trace from binlog to SVG.
/// C: `svg_convert` (loglib).
fn svg_convert(cid: &crate::ConnectionId, ctx: &mut SvgConvertCtx) -> i32 {
    svg_convert_impl(cid, ctx).map_or(-1, |_| 0)
}

struct SvgPacketHeader {
    packet_type: u64,
    pn64: u64,
}

/// Expand a path relative to the picoquic solution directory.
/// C: `picoquic_get_input_path`.
fn get_input_path(fragment: &str) -> crate::Result<String> {
    Ok(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(fragment)
        .to_string_lossy()
        .into_owned())
}

fn fileread_binlog<F>(bin_log: &mut std::fs::File, mut cb: F) -> crate::Result<()>
where
    F: FnMut(&mut ByteStream<'_>) -> crate::Result<()>,
{
    bin_log
        .seek(SeekFrom::Start(16))
        .map_err(|_| crate::Error::Generic)?;

    loop {
        let mut head = [0u8; 4];
        match bin_log.read_exact(&mut head) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(_) => return Err(crate::Error::Generic),
        }

        let len = u32::from_be_bytes(head) as usize;
        if len > BYTESTREAM_MAX_BUFFER_SIZE {
            return Err(crate::Error::Generic);
        }

        let mut buf = vec![0u8; len];
        bin_log
            .read_exact(&mut buf)
            .map_err(|_| crate::Error::Generic)?;
        let mut stream = ByteStream::from_slice(&mut buf);
        cb(&mut stream)?;
    }
}

fn read_packet_header(s: &mut ByteStream<'_>) -> crate::Result<SvgPacketHeader> {
    let _header_flags = s.read_u8()?;
    let _payload_length = s.read_vlen()?;
    let packet_type = s.read_varint()?;
    let pn64 = s.read_varint()?;
    let _dest_cnx_id = s.read_cid()?;
    let _srce_cnx_id = s.read_cid()?;

    if packet_type != PacketType::OneRttProtected as u64
        && packet_type != PacketType::VersionNegotiation as u64
    {
        let _vn = s.read_u32()?;
    }

    if packet_type == PacketType::Initial as u64 {
        let token_length = s.read_vlen()?;
        s.skip(token_length)?;
    }

    Ok(SvgPacketHeader { packet_type, pn64 })
}

fn packet_type_name(packet_type: u64) -> &'static str {
    match packet_type {
        0 => "error",
        1 => "version_negotiation",
        2 => "initial",
        3 => "retry",
        4 => "handshake",
        5 => "0RTT",
        6 => "1RTT",
        _ => "unknown",
    }
}

fn binlog_convert(
    f_binlog: &mut std::fs::File,
    cid_filter: &crate::ConnectionId,
    out: &mut std::fs::File,
) -> crate::Result<()> {
    let mut packet_count = 0usize;
    fileread_binlog(f_binlog, |s| {
        let cid = s.read_cid()?;
        if &cid != cid_filter {
            return Ok(());
        }

        let time = s.read_varint()?;
        let path_id = s.read_varint()?;
        let event_id = s.read_varint()?;

        match event_id {
            0x0010 => {
                let _client_mode = s.read_u8()?;
                let _proposed_version = s.read_u32()?;
                let _remote_cnxid = s.read_cid()?;
            }
            0x0008 | 0x0009 => {
                let received = event_id == 0x0009;
                let packet_length = s.read_varint()?;
                let ph = read_packet_header(s)?;
                svg_packet_start(
                    out,
                    &mut packet_count,
                    time,
                    path_id,
                    packet_length,
                    &ph,
                    received,
                )
                .map_err(|_| crate::Error::Generic)?;

                while s.remaining() > 0 {
                    let len = s.read_vlen()?;
                    if s.remaining() < len {
                        return Err(crate::Error::BufferTooSmall);
                    }
                    let mut frame_bytes = s.tail()[..len].to_vec();
                    s.skip(len)?;
                    let mut frame = ByteStream::from_slice(&mut frame_bytes);
                    svg_packet_frame(out, &mut frame).map_err(|_| crate::Error::Generic)?;
                }

                writeln!(out, "</text>").map_err(|_| crate::Error::Generic)?;
            }
            _ => {}
        }

        Ok(())
    })
}

fn svg_packet_start(
    out: &mut std::fs::File,
    packet_count: &mut usize,
    time: u64,
    _path_id: u64,
    size: u64,
    ph: &SvgPacketHeader,
    received: bool,
) -> std::io::Result<()> {
    let event_height = 32;
    let x_pos = 50;
    let y_pos = 32 + (*packet_count as i32) * event_height;
    let dir = if received { "in" } else { "out" };
    let time1 = time / 1000;
    let time01 = (time % 1000) / 100;

    writeln!(
        out,
        "  <use x=\"{x_pos}\" y=\"{y_pos}\" xlink:href=\"#packet-{dir}\" />"
    )?;
    writeln!(
        out,
        "  <text x=\"{}\" y=\"{}\" text-anchor=\"end\" class=\"time\">{time1}.{time01} ms</text>",
        x_pos - 4,
        y_pos + 8
    )?;

    if received {
        writeln!(
            out,
            "  <text x=\"{}\" y=\"{}\" text-anchor=\"start\" class=\"seq_{dir}\">{}</text>",
            600 - x_pos + 4,
            y_pos - 4,
            ph.pn64
        )?;
        writeln!(
            out,
            "  <text x=\"{}\" y=\"{}\" text-anchor=\"start\" class=\"arw\">{size} b</text>",
            600 - 80,
            y_pos - 2
        )?;
        writeln!(
            out,
            "  <text x=\"{}\" y=\"{}\" text-anchor=\"end\" class=\"frm\" xml:space=\"preserve\">{} </text>",
            600 - 80,
            y_pos - 2,
            packet_type_name(ph.packet_type)
        )?;
        write!(
            out,
            "  <text x=\"{}\" y=\"{}\" text-anchor=\"end\" class=\"frm\" xml:space=\"preserve\">",
            600 - x_pos - 30,
            y_pos + 10
        )?;
    } else {
        writeln!(
            out,
            "  <text x=\"{}\" y=\"{}\" text-anchor=\"end\" class=\"seq_{dir}\">{}</text>",
            x_pos - 4,
            y_pos - 4,
            ph.pn64
        )?;
        writeln!(
            out,
            "  <text x=\"{}\" y=\"{}\" text-anchor=\"end\" class=\"arw\">{size} b</text>",
            80,
            y_pos - 2
        )?;
        writeln!(
            out,
            "  <text x=\"{}\" y=\"{}\" text-anchor=\"start\" class=\"frm\" xml:space=\"preserve\"> {}</text>",
            80,
            y_pos - 2,
            packet_type_name(ph.packet_type)
        )?;
        write!(
            out,
            "  <text x=\"{}\" y=\"{}\" text-anchor=\"start\" class=\"frm\" xml:space=\"preserve\">",
            x_pos + 30,
            y_pos + 10
        )?;
    }

    *packet_count += 1;
    Ok(())
}

fn svg_packet_frame(out: &mut std::fs::File, s: &mut ByteStream<'_>) -> std::io::Result<()> {
    let frame_type = s
        .read_varint()
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;

    if frame_type >= FrameType::StreamRangeMin as u64
        && frame_type <= FrameType::StreamRangeMax as u64
    {
        let stream_id = s
            .read_varint()
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
        write!(out, " stream[{stream_id}] ")
    } else {
        write!(
            out,
            " {} ",
            FrameType::name(frame_type).unwrap_or("unknown")
        )
    }
}

fn svg_convert_impl(cid: &crate::ConnectionId, ctx: &mut SvgConvertCtx) -> crate::Result<()> {
    let mut cid_name = String::with_capacity(2 * cid.len());
    for byte in cid.as_bytes() {
        let _ = write!(&mut cid_name, "{byte:02x}");
    }

    let out_file = std::path::Path::new(ctx.out_dir).join(format!("{cid_name}.svg"));
    let mut f_txtlog = std::fs::File::create(out_file).map_err(|_| crate::Error::Generic)?;
    let f_binlog = ctx.f_binlog.as_mut().ok_or(crate::Error::Generic)?;
    let f_template = ctx.f_template.as_mut().ok_or(crate::Error::Generic)?;
    let mut template_reader = std::io::BufReader::new(f_template);

    let mut line = String::new();
    loop {
        line.clear();
        let n = template_reader
            .read_line(&mut line)
            .map_err(|_| crate::Error::Generic)?;
        if n == 0 {
            break;
        }
        if line == "#\n" {
            binlog_convert(f_binlog, cid, &mut f_txtlog)?;
        } else {
            f_txtlog
                .write_all(line.as_bytes())
                .map_err(|_| crate::Error::Generic)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------

/// C: `picolog_basic_test` in `picoquictest/picolog_test.c`.
#[test]
fn picolog_basic() {
    let mut _simulated_time = Instant::from_ticks(0);

    let log_input =
        get_input_path("picoquictest/picolog_test_input.log").expect("get log input path");
    let svg_template_path = get_input_path("loglib/template.svg").expect("get svg template path");

    let mut f_binlog = std::fs::File::open(&log_input).expect("open binlog");
    let f_template = std::fs::File::open(&svg_template_path).expect("open svg template");

    let mut cids = cidset_create().expect("cidset_create");

    binlog_list_cids(&mut f_binlog, &mut cids);
    assert!(!cids.is_empty(), "cids must not be empty");

    {
        let mut cid_prints = std::fs::File::create("./cidset.txt").expect("create cidset output");
        cidset_print(&mut cid_prints, &cids);
    }

    let cid_test = crate::ConnectionId::clone_from_slice(&[11, 12, 13, 14, 15, 16, 17, 18])
        .expect("build test CID");
    assert!(!cidset_has_cid(&cids, &cid_test), "unexpected CID in set");

    let mut ctx = SvgConvertCtx {
        binlog_name: log_input.clone(),
        out_dir: ".",
        f_binlog: Some(std::fs::File::open(&log_input).expect("reopen binlog")),
        f_template: Some(f_template),
    };
    let ret = cidset_iterate(&cids, svg_convert, &mut ctx);
    assert_eq!(ret, 0, "cidset_iterate failed");

    let svglog_ref = get_input_path("picoquictest/svglog_ref.svg").expect("get svglog ref path");
    compare_text_files("./0102030405060708.svg", &svglog_ref)
        .expect("svg output matches reference");
}
