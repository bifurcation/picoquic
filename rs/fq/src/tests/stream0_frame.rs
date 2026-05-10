//! Test cases for `picoquictest/stream0_frame_test.c`.
//!
//! Exercises STREAM-0 / TLS-stream frame decode, provide-stream-buffer
//! round-trip, output-stream list management, stream ID rank encoding,
//! splay-tree integrity, and local-stream-ID reuse.

#![allow(non_snake_case)]

use core::any::Any;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::errors::InternalError;
use crate::internal::{
    Connection, DEFAULT_0RTT_WINDOW, Quic, StreamDataBufferArgument, StreamDataNode, StreamHead,
    StreamToken, decode_crypto_hs_frame, decode_stream_frame, format_stream_frame_header,
    parse_stream_header,
};
use crate::stream::{Direction, Role, StreamId};
use crate::{
    CallbackEvent, ConnectionId, Error, Instant, RESET_SECRET_SIZE, State, StreamDataCallback,
};

// ---------------------------------------------------------------------------
// Internal helpers translated from the matching C routines.

struct PacketDef {
    packet: &'static [u8],
}

struct FrameCase {
    name: &'static str,
    list: &'static [PacketDef],
    expected_length: usize,
}

const V0_1: &[u8] = &[0x10, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
const V0_2: &[u8] = &[0x14, 0, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
const V0_3: &[u8] = &[
    0x14, 0x40, 0, 0x40, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
];
const V0_4: &[u8] = &[
    0x16, 0x40, 0, 0x80, 0, 0, 30, 0x40, 10, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
];
const V0_5: &[u8] = &[
    0x16, 0x80, 0, 0, 0, 0xC0, 0, 0, 0, 0, 0, 0, 40, 0x40, 10, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 0, 0, 0, 0, 0,
];
const V0_45_OVERLAP: &[u8] = &[
    0x16, 0, 0x40, 35, 0x40, 10, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

static LIST_V1: &[PacketDef] = &[
    PacketDef { packet: V0_1 },
    PacketDef { packet: V0_2 },
    PacketDef { packet: V0_3 },
    PacketDef { packet: V0_4 },
    PacketDef { packet: V0_5 },
];
static LIST_V2: &[PacketDef] = &[
    PacketDef { packet: V0_2 },
    PacketDef { packet: V0_3 },
    PacketDef { packet: V0_1 },
    PacketDef { packet: V0_5 },
    PacketDef { packet: V0_4 },
];
static LIST_V3: &[PacketDef] = &[
    PacketDef { packet: V0_1 },
    PacketDef { packet: V0_2 },
    PacketDef { packet: V0_3 },
    PacketDef { packet: V0_2 },
    PacketDef { packet: V0_3 },
    PacketDef { packet: V0_4 },
    PacketDef { packet: V0_4 },
    PacketDef { packet: V0_5 },
    PacketDef {
        packet: V0_45_OVERLAP,
    },
];

static STREAM_CASES: &[FrameCase] = &[
    FrameCase {
        name: "test_v1",
        list: LIST_V1,
        expected_length: 50,
    },
    FrameCase {
        name: "test_v2",
        list: LIST_V2,
        expected_length: 50,
    },
    FrameCase {
        name: "test_v3",
        list: LIST_V3,
        expected_length: 50,
    },
];

const TLSV0_1: &[u8] = &[0x18, 0, 0x0A, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
const TLSV0_2: &[u8] = &[0x18, 10, 0x0A, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];
const TLSV0_3: &[u8] = &[
    0x18, 0x40, 20, 0x40, 0x0A, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
];
const TLSV0_4: &[u8] = &[
    0x18, 0x80, 0, 0, 30, 0x40, 10, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
];
const TLSV0_5: &[u8] = &[
    0x18, 0xC0, 0, 0, 0, 0, 0, 0, 40, 0x40, 10, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 0, 0, 0, 0,
    0,
];
const TLSV0_45_OVERLAP: &[u8] = &[
    0x18, 0x40, 35, 0x40, 10, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];
const TLSV0_550_OVERLAP2: &[u8] = &[
    0x18, 0x05, 0x2d, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50,
];
const TLSV0_05: &[u8] = &[0x18, 0, 0x05, 1, 2, 3, 4, 5];

static TLSLIST_V1: &[PacketDef] = &[
    PacketDef { packet: TLSV0_1 },
    PacketDef { packet: TLSV0_2 },
    PacketDef { packet: TLSV0_3 },
    PacketDef { packet: TLSV0_4 },
    PacketDef { packet: TLSV0_5 },
];
static TLSLIST_V2: &[PacketDef] = &[
    PacketDef { packet: TLSV0_2 },
    PacketDef { packet: TLSV0_3 },
    PacketDef { packet: TLSV0_1 },
    PacketDef { packet: TLSV0_5 },
    PacketDef { packet: TLSV0_4 },
];
static TLSLIST_V3: &[PacketDef] = &[
    PacketDef { packet: TLSV0_1 },
    PacketDef { packet: TLSV0_2 },
    PacketDef { packet: TLSV0_3 },
    PacketDef { packet: TLSV0_2 },
    PacketDef { packet: TLSV0_3 },
    PacketDef { packet: TLSV0_4 },
    PacketDef { packet: TLSV0_4 },
    PacketDef { packet: TLSV0_5 },
    PacketDef {
        packet: TLSV0_45_OVERLAP,
    },
];
static TLSLIST_V4: &[PacketDef] = &[
    PacketDef { packet: TLSV0_3 },
    PacketDef { packet: TLSV0_4 },
    PacketDef {
        packet: TLSV0_550_OVERLAP2,
    },
    PacketDef { packet: TLSV0_05 },
];

static TLS_CASES: &[FrameCase] = &[
    FrameCase {
        name: "tlstest_v1",
        list: TLSLIST_V1,
        expected_length: 50,
    },
    FrameCase {
        name: "tlstest_v2",
        list: TLSLIST_V2,
        expected_length: 50,
    },
    FrameCase {
        name: "tlstest_v3",
        list: TLSLIST_V3,
        expected_length: 50,
    },
    FrameCase {
        name: "tlstest_v4",
        list: TLSLIST_V4,
        expected_length: 50,
    },
];

fn empty_stream_data_node() -> StreamDataNode {
    StreamDataNode {
        stream_data_membership: None,
        offset: 0,
        data: [0u8; crate::MAX_PACKET_SIZE],
        length: 0,
    }
}

fn new_test_quic(current_time: Instant) -> crate::Result<Box<Quic>> {
    Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .ok_or(Error::Memory)
}

fn test_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1000)
}

fn create_test_connection(
    quic: &mut Quic,
    current_time: Instant,
    client_mode: bool,
) -> crate::Result<&mut Connection> {
    let cid = ConnectionId::with_size(0).ok_or(Error::InvalidArgument)?;
    quic.create_connection(
        cid,
        cid,
        Some(&test_addr()),
        current_time,
        0,
        Some("test-sni"),
        Some("test-alpn"),
        client_mode,
    )
    .ok_or(Error::Memory)
}

fn verify_stream_tree(stream: &StreamHead, expected_length: usize) -> crate::Result<()> {
    let mut data_rank = 0usize;
    let mut node_token = stream.stream_data_tree.first();
    while let Some(token) = node_token {
        let data_token = *stream
            .stream_data_tree
            .get(token)
            .ok_or(Error::InvalidFrame)?;
        let data = stream
            .stream_data_nodes
            .get(data_token)
            .ok_or(Error::InvalidFrame)?;
        for byte in &data.data[..data.length] {
            data_rank += 1;
            if *byte != data_rank as u8 {
                return Err(Error::InvalidFrame);
            }
        }
        node_token = stream.stream_data_tree.next(token);
    }

    if data_rank == expected_length {
        Ok(())
    } else {
        Err(Error::InvalidFrame)
    }
}

fn delete_stream_token(cnx: &mut Connection, token: StreamToken) {
    let splay_token = cnx.streams.get_mut(token).and_then(|stream| {
        stream.is_output_stream = false;
        stream.stream_tree_membership.take()
    });
    if let Some(splay_token) = splay_token {
        cnx.stream_tree.remove(splay_token);
    }
    if let Some(pos) = cnx
        .output_streams
        .iter()
        .position(|&existing| existing == token)
    {
        cnx.output_streams.remove(pos);
    }
    cnx.streams.remove(token);
}

fn output_stream_ids(cnx: &Connection) -> crate::Result<Vec<u64>> {
    cnx.output_streams
        .iter()
        .map(|token| {
            cnx.streams
                .get(*token)
                .map(|stream| stream.stream_id)
                .ok_or(Error::InvalidState)
        })
        .collect()
}

fn check_output_streams(cnx: &Connection, expected: &[u64]) -> crate::Result<()> {
    if output_stream_ids(cnx)?.as_slice() == expected {
        Ok(())
    } else {
        Err(Error::InvalidState)
    }
}

fn with_detached_stream<R>(
    cnx: &mut Connection,
    token: StreamToken,
    f: impl FnOnce(&mut Connection, &mut StreamHead) -> crate::Result<R>,
) -> crate::Result<R> {
    let mut streams = core::mem::take(&mut cnx.streams);
    let result = match streams.get_mut(token) {
        Some(stream) => f(cnx, stream),
        None => Err(Error::InvalidState),
    };
    cnx.streams = streams;
    result
}

struct StreamOutputCallback;

impl StreamDataCallback for StreamOutputCallback {
    fn callback(
        &mut self,
        _connection: &mut Connection,
        _stream_id: u64,
        _bytes: &[u8],
        _fin_or_event: CallbackEvent,
        _stream_ctx: Option<&mut dyn Any>,
    ) -> i32 {
        0
    }
}

fn stream_output_test_delete(
    cnx: &mut Connection,
    stream_id: u64,
    reset_or_fin: bool,
) -> crate::Result<()> {
    let token = cnx.find_stream(stream_id).ok_or(Error::InvalidState)?;
    let is_last = cnx.output_streams.len() == 1 && cnx.output_streams.front() == Some(&token);
    let client_mode = cnx.client_mode;

    with_detached_stream(cnx, token, |cnx, stream| {
        if !stream.is_output_stream {
            return Err(Error::InvalidState);
        }
        if reset_or_fin {
            stream.reset_requested = true;
            stream.reset_sent = true;
            stream.reset_acked = true;
        } else {
            stream.fin_requested = true;
            stream.fin_sent = true;
            stream.sack_list.reset(
                0,
                stream.sent_offset.saturating_add(1),
                Instant::from_ticks(0),
            )?;
        }
        stream.is_active = false;
        if StreamId(stream_id).is_bidir() {
            stream.fin_received = true;
            stream.fin_signalled = true;
        }

        if !stream.is_stream_closed(client_mode) {
            return Err(Error::InvalidState);
        }
        cnx.remove_output_stream(stream);
        cnx.delete_stream_if_closed(stream);
        Ok(())
    })?;

    let ready = cnx.find_ready_stream();
    if ready.is_none() && !is_last {
        return Err(Error::InvalidState);
    }
    if ready.is_some() && is_last {
        return Err(Error::InvalidState);
    }
    if ready == Some(token) {
        return Err(Error::InvalidState);
    }
    for existing in cnx.output_streams.iter().copied() {
        if cnx
            .streams
            .get(existing)
            .ok_or(Error::InvalidState)?
            .stream_id
            == stream_id
        {
            return Err(Error::InvalidState);
        }
    }
    if cnx.find_stream(stream_id).is_some() {
        return Err(Error::InvalidState);
    }
    Ok(())
}

/// C: `StreamZeroFrameOneTest` — decode STREAM-0 frames from `name`'s
/// packet list into a fresh connection and verify the splay-tree content.
fn stream_zero_frame_one_test(name: &str) -> crate::Result<()> {
    let test = STREAM_CASES
        .iter()
        .find(|case| case.name == name)
        .ok_or(Error::InvalidArgument)?;
    let current_time = Instant::from_ticks(0);
    let mut quic = new_test_quic(current_time)?;
    let cnx = create_test_connection(&mut quic, current_time, true)?;
    cnx.client_mode = false;

    for packet in test.list {
        let mut received = empty_stream_data_node();
        decode_stream_frame(cnx, packet.packet, &mut received, current_time)
            .ok_or(Error::InvalidFrame)?;
    }

    let stream = cnx.first_stream().ok_or(Error::InvalidFrame)?;
    let stream = cnx.streams.get(stream).ok_or(Error::InvalidFrame)?;
    if stream.stream_id != 0 {
        return Err(Error::InvalidFrame);
    }
    verify_stream_tree(stream, test.expected_length)
}

/// C: `TlsStreamFrameOneTest` — decode CRYPTO handshake frames from
/// `name`'s packet list and verify the TLS-stream data tree.
fn tls_stream_frame_one_test(name: &str) -> crate::Result<()> {
    let test = TLS_CASES
        .iter()
        .find(|case| case.name == name)
        .ok_or(Error::InvalidArgument)?;
    let current_time = Instant::from_ticks(0);
    let mut quic = new_test_quic(current_time)?;
    let cnx = create_test_connection(&mut quic, current_time, true)?;
    let epoch = 2;

    for packet in test.list {
        let mut received = empty_stream_data_node();
        decode_crypto_hs_frame(cnx, packet.packet, &mut received, epoch)
            .ok_or(Error::InvalidFrame)?;
    }

    verify_stream_tree(&cnx.tls_stream[epoch as usize], test.expected_length)
}

/// C: `provide_stream_buffer_test_one` — sets up a stream frame header,
/// calls `picoquic_provide_stream_data_buffer`, fills with test data,
/// decodes the header, and verifies the round-trip.
fn provide_stream_buffer_test_one(
    stream_id: u64,
    stream_offset: u64,
    size_test: usize,
    is_fin: bool,
) -> crate::Result<()> {
    let mut packet = [0u8; 512];
    let mut test_data = [0u8; 512];
    let tail_len = {
        let tail = format_stream_frame_header(&mut packet, stream_id, stream_offset)
            .ok_or(Error::BufferTooSmall)?;
        tail.len()
    };
    let byte_index = packet.len() - tail_len;
    let byte_space = tail_len;
    let mut allowed_space = byte_space;

    let length = match size_test {
        0 => byte_space,
        1 => byte_space - 1,
        2 => byte_space - 2,
        3 => byte_space - 3,
        4 => 1,
        5 => 0,
        6 => {
            allowed_space = byte_space - 1;
            byte_space
        }
        _ => return Err(Error::InvalidArgument),
    };

    let provided = {
        let mut stream_data_context = StreamDataBufferArgument {
            bytes: &mut packet,
            byte_index,
            byte_space,
            allowed_space,
            length: 0,
            is_fin: 0,
            is_still_active: 0,
            app_buffer: &[],
        };
        match crate::provide_stream_data_buffer(&mut stream_data_context, length, is_fin, false) {
            None => false,
            Some(data_ptr) => {
                if data_ptr.len() != length {
                    return Err(Error::InvalidFrame);
                }
                for (i, byte) in test_data.iter_mut().enumerate().take(length) {
                    *byte = (i as u8) ^ 0x7f;
                }
                data_ptr.copy_from_slice(&test_data[..length]);
                true
            }
        }
    };

    if size_test == 6 {
        return if provided {
            Err(Error::InvalidFrame)
        } else {
            Ok(())
        };
    }
    if !provided {
        return Err(Error::InvalidFrame);
    }

    let packet_start = packet
        .iter()
        .position(|&byte| byte != crate::frames::FrameType::Padding as u8)
        .ok_or(Error::InvalidFrame)?;
    let mut received_stream_id = 0;
    let mut received_offset = 0;
    let mut received_length = 0;
    let mut received_fin = 0;
    let mut consumed = 0;
    if parse_stream_header(
        &packet[packet_start..],
        packet.len() - packet_start,
        &mut received_stream_id,
        &mut received_offset,
        &mut received_length,
        &mut received_fin,
        &mut consumed,
    ) != 0
    {
        return Err(Error::InvalidFrame);
    }

    if received_stream_id != stream_id
        || received_offset != stream_offset
        || received_length != length
        || (received_fin != 0) != is_fin
    {
        return Err(Error::InvalidFrame);
    }
    if length > 0
        && packet[packet_start + consumed..packet_start + consumed + length] != test_data[..length]
    {
        return Err(Error::InvalidFrame);
    }
    Ok(())
}

/// C: `stream_output_test` body — creates eight streams, exercises the
/// output-stream list ordering (including a `MAX_STREAMS` bump), marks
/// streams active, then deletes them in a specific order verifying
/// `find_ready_stream` behaviour at each step.
fn stream_output_test_body() -> crate::Result<()> {
    let current_time = Instant::from_ticks(0);
    let mut quic = new_test_quic(current_time)?;
    let cnx = create_test_connection(&mut quic, current_time, true)?;
    let values = [0, 3, 4, 1, 2, 8, 5, 7];
    let output1 = [0, 1, 2, 4, 5];
    let output2 = [0, 1, 2, 4, 5, 8];
    let delete_order = [1, 0, 4, 2, 5, 8];

    cnx.set_callback(Some(Box::new(StreamOutputCallback)));
    cnx.maxdata_remote = DEFAULT_0RTT_WINDOW as u64;
    cnx.remote_parameters.initial_max_stream_data_bidi_remote = DEFAULT_0RTT_WINDOW as u64;
    cnx.remote_parameters.initial_max_stream_data_uni = DEFAULT_0RTT_WINDOW as u64;
    cnx.max_stream_id_bidir_remote = if cnx.client_mode { 4 } else { 0 };
    cnx.max_stream_id_unidir_remote = if cnx.client_mode { 10 } else { 0 };
    cnx.high_priority_stream_id = 1;

    for stream_id in values.iter().take(7).copied() {
        cnx.create_stream(stream_id)?;
    }
    check_output_streams(cnx, &output1)?;

    let old_limit = cnx.max_stream_id_bidir_remote;
    cnx.max_stream_id_bidir_remote = 8;
    cnx.add_output_streams(old_limit, 8, true);
    check_output_streams(cnx, &output2)?;

    if cnx.find_ready_stream().is_some() {
        return Err(Error::InvalidState);
    }

    for stream_id in output_stream_ids(cnx)? {
        let stream = cnx.find_stream(stream_id).ok_or(Error::InvalidState)?;
        cnx.streams
            .get_mut(stream)
            .ok_or(Error::InvalidState)?
            .maxdata_remote = 4096;
        cnx.mark_active_stream(stream_id, true, None)?;
    }

    let ready = cnx.find_ready_stream().ok_or(Error::InvalidState)?;
    let ready = cnx.streams.get(ready).ok_or(Error::InvalidState)?;
    if ready.stream_id != output2[0] {
        return Err(Error::InvalidState);
    }

    for (i, stream_id) in delete_order.iter().copied().enumerate() {
        stream_output_test_delete(cnx, stream_id, (i & 1) != 0)?;
    }
    Ok(())
}

/// C: `stream_splay_test` body — inserts seven streams in non-monotone
/// order, verifies splay-tree structural invariants and first/last
/// pointers after each insertion, then deletes them in insertion order
/// and checks the invariants again.
fn stream_splay_test_body() -> crate::Result<()> {
    let current_time = Instant::from_ticks(0);
    let mut quic = new_test_quic(current_time)?;
    let cnx = create_test_connection(&mut quic, current_time, true)?;
    let values = [3, 4, 1, 2, 8, 5, 7];
    let ordered = [1, 2, 3, 4, 5, 7, 8];
    let values_first = [3, 3, 1, 1, 1, 1, 1];
    let values_last = [3, 4, 4, 4, 8, 8, 8];
    let value2_first = [1, 1, 2, 5, 5, 7, 0];
    let value2_last = [8, 8, 8, 8, 7, 7, 0];

    for (i, stream_id) in values.iter().copied().enumerate() {
        cnx.create_stream(stream_id)?;
        if cnx.stream_tree.len() != i + 1 {
            return Err(Error::InvalidState);
        }
        let first = cnx.first_stream().ok_or(Error::InvalidState)?;
        let last = cnx.last_stream().ok_or(Error::InvalidState)?;
        if cnx.streams.get(first).ok_or(Error::InvalidState)?.stream_id != values_first[i]
            || cnx.streams.get(last).ok_or(Error::InvalidState)?.stream_id != values_last[i]
        {
            return Err(Error::InvalidState);
        }
    }

    let mut stream = cnx.first_stream();
    for expected in ordered {
        let token = stream.ok_or(Error::InvalidState)?;
        let found = cnx.streams.get(token).ok_or(Error::InvalidState)?;
        if found.stream_id != expected {
            return Err(Error::InvalidState);
        }
        stream = cnx.next_stream(token);
    }
    if stream.is_some() {
        return Err(Error::InvalidState);
    }

    for (i, stream_id) in values.iter().copied().enumerate() {
        let token = cnx.find_stream(stream_id).ok_or(Error::InvalidState)?;
        delete_stream_token(cnx, token);
        if cnx.stream_tree.len() != 6 - i {
            return Err(Error::InvalidState);
        }
        if i < 6 {
            let first = cnx.first_stream().ok_or(Error::InvalidState)?;
            let last = cnx.last_stream().ok_or(Error::InvalidState)?;
            if cnx.streams.get(first).ok_or(Error::InvalidState)?.stream_id != value2_first[i]
                || cnx.streams.get(last).ok_or(Error::InvalidState)?.stream_id != value2_last[i]
            {
                return Err(Error::InvalidState);
            }
        }
    }

    if cnx.stream_tree.is_empty() {
        Ok(())
    } else {
        Err(Error::InvalidState)
    }
}

/// C: `stream_state_local_reuse_test` body — opens a local stream,
/// deletes it, then verifies that calling `set_app_stream_ctx` on the
/// recycled ID returns `STREAM_ALREADY_CLOSED` (not `STREAM_STATE_ERROR`).
fn stream_state_local_reuse_body() -> crate::Result<()> {
    let current_time = Instant::from_ticks(0);
    let mut quic = new_test_quic(current_time)?;
    let cnx = create_test_connection(&mut quic, current_time, true)?;
    cnx.client_mode = true;

    let sid1 = cnx.next_local_stream_id(false);
    let sid2 = cnx.next_local_stream_id(false);
    if sid1 != sid2 {
        return Err(Error::InvalidState);
    }

    let stream = cnx.create_stream(sid1)?;
    delete_stream_token(cnx, stream);
    if cnx.next_stream_id[(sid1 & 3) as usize] <= sid1 {
        return Err(Error::InvalidState);
    }

    match cnx.set_app_stream_ctx(sid2, None) {
        Err(Error::Protocol(code)) if code == InternalError::StreamAlreadyClosed as u64 => {}
        _ => return Err(Error::InvalidState),
    }

    if matches!(
        cnx.connection_state,
        State::Disconnecting
            | State::Disconnected
            | State::HandshakeFailure
            | State::HandshakeFailureResend
    ) || cnx.local_error == 0x5
    {
        Err(Error::InvalidState)
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Exported tests.

/// C: `StreamZeroFrameTest` in `picoquictest/stream0_frame_test.c`.
#[test]
fn streamzeroframe() {
    for name in &["test_v1", "test_v2", "test_v3"] {
        stream_zero_frame_one_test(name).unwrap_or_else(|_| panic!("{name}"));
    }
}

/// C: `TlsStreamFrameTest` in `picoquictest/stream0_frame_test.c`.
#[test]
fn tlsstreamframe() {
    for name in &["tlstest_v1", "tlstest_v2", "tlstest_v3", "tlstest_v4"] {
        tls_stream_frame_one_test(name).unwrap_or_else(|_| panic!("{name}"));
    }
}

/// C: `provide_stream_buffer_test` in `picoquictest/stream0_frame_test.c`.
#[test]
fn provide_stream_buffer() {
    let stream_ids: [u64; 4] = [0, 7, 127, 0x10000];
    let offsets: [u64; 4] = [0, 1, 65, 0x10000];
    for &sid in &stream_ids {
        for &off in &offsets {
            for is_fin in [false, true] {
                for size_test in 0..7usize {
                    provide_stream_buffer_test_one(sid, off, size_test, is_fin)
                        .expect("provide_stream_buffer_test_one");
                }
            }
        }
    }
}

/// C: `stream_output_test` in `picoquictest/stream0_frame_test.c`.
#[test]
fn stream_output() {
    stream_output_test_body().expect("stream_output_test");
}

/// C: `stream_rank_test` in `picoquictest/stream0_frame_test.c`.
///
/// Verifies the QUIC stream ID rank macros:
///   `STREAM_RANK_FROM_ID(id)  = (id >> 2) + 1`
///   `STREAM_ID_FROM_RANK(rank, client, uni) = ((rank-1) << 2) | (uni << 1) | server`
#[test]
fn stream_rank() {
    let ranks: [u64; 5] = [1, 2, 3, 1000, 10000];
    let client_bidir: [u64; 5] = [0, 4, 8, 3996, 39996];
    let client_unidir: [u64; 5] = [2, 6, 10, 3998, 39998];
    let server_bidir: [u64; 5] = [1, 5, 9, 3997, 39997];
    let server_unidir: [u64; 5] = [3, 7, 11, 3999, 39999];

    for i in 0..5 {
        let r = ranks[i];

        assert_eq!(
            StreamId(client_bidir[i]).rank(),
            r,
            "rank_from_id client_bidir[{i}]"
        );
        assert_eq!(
            StreamId::from_parts(r, Role::Client, Direction::Bidir).0,
            client_bidir[i],
            "id_from_rank client_bidir[{i}]"
        );

        assert_eq!(
            StreamId(client_unidir[i]).rank(),
            r,
            "rank_from_id client_unidir[{i}]"
        );
        assert_eq!(
            StreamId::from_parts(r, Role::Client, Direction::Unidir).0,
            client_unidir[i],
            "id_from_rank client_unidir[{i}]"
        );

        assert_eq!(
            StreamId(server_bidir[i]).rank(),
            r,
            "rank_from_id server_bidir[{i}]"
        );
        assert_eq!(
            StreamId::from_parts(r, Role::Server, Direction::Bidir).0,
            server_bidir[i],
            "id_from_rank server_bidir[{i}]"
        );

        assert_eq!(
            StreamId(server_unidir[i]).rank(),
            r,
            "rank_from_id server_unidir[{i}]"
        );
        assert_eq!(
            StreamId::from_parts(r, Role::Server, Direction::Unidir).0,
            server_unidir[i],
            "id_from_rank server_unidir[{i}]"
        );
    }
}

/// C: `stream_splay_test` in `picoquictest/stream0_frame_test.c`.
#[test]
fn stream_splay() {
    stream_splay_test_body().expect("stream_splay_test");
}

/// C: `stream_state_local_reuse_test` in `picoquictest/stream0_frame_test.c`.
#[test]
fn stream_state_local_reuse() {
    stream_state_local_reuse_body().expect("stream_state_local_reuse_test");
}
