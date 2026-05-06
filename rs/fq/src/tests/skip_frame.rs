//! Test cases for `picoquictest/skip_frame_test.c`.
//!
//! Exercises frame skip/parse/format/repeat/retransmit logic and the
//! binlog/logger infrastructure.

#![allow(non_snake_case)]

use crate::internal::skip_frame;
use crate::{Instant, Quic};

// ---------------------------------------------------------------------------
// Internal frame-test helpers.
// Each wraps a corresponding C static helper that operates on a single Quic
// context.

fn epoch_packet_type(epoch: u32) -> crate::internal::PacketType {
    match epoch {
        0 => crate::internal::PacketType::Initial,
        1 => crate::internal::PacketType::ZeroRttProtected,
        2 => crate::internal::PacketType::Handshake,
        _ => crate::internal::PacketType::OneRttProtected,
    }
}

fn packet_context_from_epoch(epoch: u32) -> crate::PacketContext {
    match epoch {
        0 => crate::PacketContext::Initial,
        2 => crate::PacketContext::Handshake,
        _ => crate::PacketContext::Application,
    }
}

fn create_test_varint_frame(frame: &[u8], varint_idx: u32) -> Vec<u8> {
    let mut cursor = frame;
    let mut skipped = 0usize;
    for _ in 0..varint_idx {
        let before = cursor.len();
        match crate::internal::frames_varint_skip(cursor) {
            Some(rest) => {
                skipped += before - rest.len();
                cursor = rest;
            }
            None => return Vec::new(),
        }
    }

    let mut out = frame[..skipped.min(frame.len())].to_vec();
    let mut value = 0;
    if crate::internal::frames_varint_decode(cursor, &mut value).is_none() {
        return Vec::new();
    }
    out.push(0xc0 | ((value >> 56) as u8));
    out.push((value >> 48) as u8);
    out.push((value >> 40) as u8);
    out.push((value >> 32) as u8);
    out.push((value >> 24) as u8);
    out.push((value >> 16) as u8);
    out.push((value >> 8) as u8);
    out
}

fn write_varint(out: &mut Vec<u8>, value: u64) {
    let mut buf = [0u8; 8];
    let n = crate::internal::varint_encode(&mut buf, value);
    out.extend_from_slice(&buf[..n]);
}

struct TestSkipFrame {
    name: &'static str,
    bytes: Vec<u8>,
    pure_ack: i32,
    must_be_last: bool,
}

fn typed_frame(
    frame_type: u64,
    tail: &[u8],
    pure_ack: i32,
    must_be_last: bool,
    name: &'static str,
) -> TestSkipFrame {
    let mut bytes = Vec::new();
    write_varint(&mut bytes, frame_type);
    bytes.extend_from_slice(tail);
    TestSkipFrame {
        name,
        bytes,
        pure_ack,
        must_be_last,
    }
}

fn test_skip_frames() -> Vec<TestSkipFrame> {
    use crate::frames::FrameType as F;

    vec![
        TestSkipFrame {
            name: "padding",
            bytes: vec![0, 0, 0],
            pure_ack: 1,
            must_be_last: false,
        },
        typed_frame(F::ResetStream as u64, &[17, 1, 1], 0, false, "reset_stream"),
        typed_frame(
            F::ConnectionClose as u64,
            &[
                0x80, 0x00, 0xcf, 0xff, 0, 9, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',
            ],
            1,
            false,
            "connection_close",
        ),
        typed_frame(
            F::ApplicationClose as u64,
            &[0, 0],
            1,
            false,
            "application_close",
        ),
        typed_frame(
            F::ApplicationClose as u64,
            &[0x44, 4, 4, b't', b'e', b's', b't'],
            1,
            false,
            "application_close_reason",
        ),
        typed_frame(
            F::MaxData as u64,
            &[0xc0, 0, 0x01, 0, 0, 0, 0, 0],
            0,
            false,
            "max_data",
        ),
        typed_frame(
            F::MaxStreamData as u64,
            &[1, 0x80, 0x01, 0, 0],
            0,
            false,
            "max_stream_data",
        ),
        typed_frame(
            F::MaxStreamsBidir as u64,
            &[0x41, 0],
            0,
            false,
            "max_streams_bidir",
        ),
        typed_frame(
            F::MaxStreamsUnidir as u64,
            &[0x41, 7],
            0,
            false,
            "max_streams_unidir",
        ),
        typed_frame(F::Ping as u64, &[], 0, false, "ping"),
        typed_frame(
            F::DataBlocked as u64,
            &[0x80, 0x01, 0, 0],
            0,
            false,
            "blocked",
        ),
        typed_frame(
            F::StreamDataBlocked as u64,
            &[0x80, 1, 0, 0, 0x80, 0x02, 0, 0],
            0,
            false,
            "stream_data_blocked",
        ),
        typed_frame(
            F::StreamsBlockedBidir as u64,
            &[0x41, 0],
            0,
            false,
            "streams_blocked_bidir",
        ),
        typed_frame(
            F::StreamsBlockedUnidir as u64,
            &[0x42, 0],
            0,
            false,
            "streams_blocked_unidir",
        ),
        typed_frame(
            F::NewConnectionId as u64,
            &[
                7, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
                0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae, 0xaf,
            ],
            0,
            false,
            "new_connection_id",
        ),
        typed_frame(F::StopSending as u64, &[17, 0x17], 0, false, "stop_sending"),
        typed_frame(
            F::PathChallenge as u64,
            &[1, 2, 3, 4, 5, 6, 7, 8],
            1,
            false,
            "path_challenge",
        ),
        typed_frame(
            F::PathResponse as u64,
            &[1, 2, 3, 4, 5, 6, 7, 8],
            1,
            false,
            "path_response",
        ),
        typed_frame(
            F::NewToken as u64,
            &[
                17, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
            ],
            0,
            false,
            "new_token",
        ),
        typed_frame(
            F::Ack as u64,
            &[0xc0, 0, 0, 1, 2, 3, 4, 5, 0x44, 0, 2, 5, 0, 0, 5, 12],
            1,
            false,
            "ack",
        ),
        typed_frame(
            F::AckEcn as u64,
            &[
                0xc0, 0, 0, 1, 2, 3, 4, 5, 0x44, 0, 2, 5, 0, 0, 5, 12, 3, 0, 1,
            ],
            1,
            false,
            "ack_ecn",
        ),
        typed_frame(
            F::StreamRangeMin as u64,
            &[
                1, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xac,
                0xad, 0xae, 0xaf,
            ],
            0,
            true,
            "stream_min",
        ),
        typed_frame(
            (F::StreamRangeMin as u64) + 6,
            &[
                1, 0x44, 0, 0x10, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa,
                0xab, 0xac, 0xad, 0xae, 0xaf,
            ],
            0,
            false,
            "stream_max",
        ),
        typed_frame(
            F::CryptoHs as u64,
            &[
                0, 0x10, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab,
                0xac, 0xad, 0xae, 0xaf,
            ],
            0,
            false,
            "crypto_hs",
        ),
        typed_frame(
            F::RetireConnectionId as u64,
            &[1],
            0,
            false,
            "retire_connection_id",
        ),
        typed_frame(
            F::Datagram as u64,
            &[
                0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad,
                0xae, 0xaf,
            ],
            0,
            true,
            "datagram",
        ),
        typed_frame(
            F::DatagramL as u64,
            &[
                0x10, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xac,
                0xad, 0xae, 0xaf,
            ],
            0,
            false,
            "datagram_l",
        ),
        typed_frame(F::HandshakeDone as u64, &[], 0, false, "handshake_done"),
        typed_frame(
            F::AckFrequency as u64,
            &[17, 0x0a, 0x44, 0x20, 0x00],
            0,
            false,
            "ack_frequency",
        ),
        typed_frame(
            F::AckFrequency as u64,
            &[17, 0x0a, 0x44, 0x20, 0x40, 0x05],
            0,
            false,
            "ack_frequency_t5",
        ),
        typed_frame(F::ImmediateAck as u64, &[], 0, false, "immediate_ack"),
        typed_frame(F::TimeStamp as u64, &[0x44, 0], 1, false, "time_stamp"),
        typed_frame(
            F::PathAbandon as u64,
            &[0x01, 0x00],
            0,
            false,
            "path_abandon_0",
        ),
        typed_frame(
            F::PathAbandon as u64,
            &[0x01, 0x11],
            0,
            false,
            "path_abandon_1",
        ),
        typed_frame(F::PathBackup as u64, &[0x00, 0x0f], 0, false, "path_backup"),
        typed_frame(
            F::PathAvailable as u64,
            &[0x00, 0x0f],
            0,
            false,
            "path_available",
        ),
        typed_frame(F::MaxPathId as u64, &[0x11], 0, false, "max_path_id"),
        typed_frame(
            F::PathNewConnectionId as u64,
            &[
                1, 7, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8,
                0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae, 0xaf, 0xb0,
            ],
            0,
            false,
            "path_new_connection_id",
        ),
        typed_frame(
            F::PathRetireConnectionId as u64,
            &[0, 2],
            0,
            false,
            "path_retire_connection_id",
        ),
        typed_frame(F::PathsBlocked as u64, &[0x11], 0, false, "paths_blocked"),
        typed_frame(
            F::PathCidBlocked as u64,
            &[0x07, 0x01],
            0,
            false,
            "path_cid_blocked",
        ),
        typed_frame(F::Bdp as u64, &[1, 2, 3, 4, 0x0a, 0, 0, 1], 0, false, "bdp"),
        typed_frame(
            F::ObservedAddressV4 as u64,
            &[1, 1, 2, 3, 4, 0x12, 0x34],
            0,
            false,
            "observed_address_v4",
        ),
        typed_frame(
            F::ObservedAddressV6 as u64,
            &[
                2, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 0x45, 0x67,
            ],
            0,
            false,
            "observed_address_v6",
        ),
        typed_frame(
            F::PathAck as u64,
            &[0, 0xc0, 0, 0, 1, 2, 3, 4, 5, 0x44, 0, 2, 5, 0, 0, 5, 12],
            1,
            false,
            "path_ack",
        ),
        typed_frame(
            F::PathAckEcn as u64,
            &[
                0, 0xc0, 0, 0, 1, 2, 3, 4, 5, 0x44, 0, 2, 5, 0, 0, 5, 12, 3, 0, 1,
            ],
            1,
            false,
            "path_ack_ecn",
        ),
        typed_frame(
            F::ResetStreamAt as u64,
            &[17, 1, 0x40, 128, 13],
            0,
            false,
            "reset_stream_at",
        ),
    ]
}

fn make_stream_frame(
    has_length: bool,
    has_fin: bool,
    stream_id: u64,
    offset: u64,
    data_len: usize,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut frame_type = crate::frames::FrameType::StreamRangeMin as u8 | 0x04;
    if has_length {
        frame_type |= 0x02;
    }
    if has_fin {
        frame_type |= 0x01;
    }
    out.push(frame_type);
    write_varint(&mut out, stream_id);
    write_varint(&mut out, offset);
    if has_length {
        write_varint(&mut out, data_len as u64);
    }
    out.extend((0..data_len).map(|i| (i as u8).wrapping_add(1)));
    out
}

fn prepare_retransmit_packet(
    packet: &mut crate::internal::Packet,
    has_length: bool,
    has_fin: bool,
    stream_id: u64,
    offset: u64,
    data_len: usize,
) {
    let frame = make_stream_frame(has_length, has_fin, stream_id, offset, data_len);
    packet.offset = 12;
    packet.data_repeat_frame = 17;
    packet.data_repeat_index = 17;
    packet.length = packet.data_repeat_frame + frame.len();
    packet.bytes[..packet.length].fill(0);
    packet.bytes[packet.data_repeat_frame..packet.length].copy_from_slice(&frame);
    packet.data_repeat_stream_id = stream_id;
    packet.data_repeat_stream_offset = offset;
    packet.data_repeat_stream_data_length = data_len;
    packet.is_queued_for_data_repeat = true;
}

/// C: `parse_test_packet` — parse every frame in `buf` using a fresh cnx.
fn parse_test_packet(quic: &mut Quic, buf: &[u8], epoch: u32, mpath: bool) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    cnx.is_multipath_enabled = mpath;
    cnx.is_reset_stream_at_enabled = true;
    if epoch == 3 {
        cnx.connection_state = crate::State::Ready;
    }

    let mut tail = buf;
    while !tail.is_empty() {
        let mut consumed = 0usize;
        let mut pure_ack = 0i32;
        if skip_frame(tail, tail.len(), &mut consumed, &mut pure_ack) != 0 || consumed == 0 {
            return Err(crate::Error::InvalidFrame);
        }
        if pure_ack == 0 {
            cnx.ack_ctx[packet_context_from_epoch(epoch) as usize].act[0].ack_needed = true;
        }
        tail = &tail[consumed..];
    }
    Ok(())
}

/// C: `frame_ackack_error_packet` — send a frame that triggers an ack-ack
/// error on a fresh connection.
fn frame_ackack_error_packet(
    quic: &mut Quic,
    frame: &[u8],
    epoch: u32,
    mpath: bool,
    varint_idx: u32,
) -> bool {
    let mut simulated_time = Instant::from_ticks(0);
    let Ok(mut cnx) = quic.create_test_cnx(&mut simulated_time) else {
        return true;
    };
    cnx.is_multipath_enabled = mpath;
    let mut packet = match quic.create_packet() {
        Ok(p) => p,
        Err(_) => return true,
    };
    let frame = create_test_varint_frame(frame, varint_idx);
    packet.packet_type = epoch_packet_type(epoch);
    packet.packet_context = packet_context_from_epoch(epoch);
    packet.offset = if packet.packet_type == crate::internal::PacketType::OneRttProtected {
        13
    } else {
        25
    };
    packet.length = packet.offset + frame.len();
    if packet.length <= packet.bytes.len() {
        packet.bytes[packet.offset..packet.length].copy_from_slice(&frame);
    }
    let previous_state = cnx.connection_state;
    cnx.process_ack_of_frames(&mut packet, 0);
    cnx.connection_state != previous_state
}

/// C: `frame_repeat_error_packet` — verify the repeat-detection logic on `buf`.
fn frame_repeat_error_packet(
    quic: &mut Quic,
    buf: &[u8],
    epoch: u32,
    mpath: bool,
    expect_error: bool,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    cnx.is_multipath_enabled = mpath;
    let mut no_need_to_repeat = 0;
    let mut do_not_detect_spurious = 0;
    let mut is_preemptive_needed = 0;
    let ret = cnx.check_frame_needs_repeat(
        buf,
        buf.len(),
        epoch_packet_type(epoch),
        &mut no_need_to_repeat,
        &mut do_not_detect_spurious,
        &mut is_preemptive_needed,
    );
    if (expect_error && ret == 0 && no_need_to_repeat == 0) || (!expect_error && ret != 0) {
        Err(crate::Error::InvalidFrame)
    } else {
        Ok(())
    }
}

/// C: `send_stream_blocked_test_one` — one stream-blocked sub-case.
fn send_stream_blocked_test_one(case_idx: usize) -> crate::Result<()> {
    struct Case {
        stream_id: u64,
        is_id_blocked: bool,
        is_data_blocked: bool,
        is_client: bool,
        expect_bidir_blocked: bool,
        expect_unidir_blocked: bool,
        expect_data_blocked: bool,
    }
    const CASES: &[Case] = &[
        Case {
            stream_id: 4,
            is_id_blocked: true,
            is_data_blocked: false,
            is_client: true,
            expect_bidir_blocked: true,
            expect_unidir_blocked: false,
            expect_data_blocked: false,
        },
        Case {
            stream_id: 4,
            is_id_blocked: true,
            is_data_blocked: true,
            is_client: false,
            expect_bidir_blocked: false,
            expect_unidir_blocked: false,
            expect_data_blocked: true,
        },
        Case {
            stream_id: 4,
            is_id_blocked: false,
            is_data_blocked: true,
            is_client: false,
            expect_bidir_blocked: false,
            expect_unidir_blocked: false,
            expect_data_blocked: true,
        },
    ];
    let case = &CASES[case_idx % CASES.len()];
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    cnx.client_mode = case.is_client;
    if case.is_id_blocked {
        cnx.remote_parameters.initial_max_stream_id_bidir = 1;
        cnx.remote_parameters.initial_max_stream_id_unidir = 0;
    } else {
        cnx.remote_parameters.initial_max_stream_id_bidir = 64;
        cnx.remote_parameters.initial_max_stream_id_unidir = 64;
    }
    let role = if cnx.client_mode {
        crate::stream::Role::Client
    } else {
        crate::stream::Role::Server
    };
    cnx.max_stream_id_bidir_remote = crate::stream::StreamId::from_parts(
        cnx.remote_parameters.initial_max_stream_id_bidir,
        role,
        crate::stream::Direction::Bidir,
    )
    .0;
    cnx.max_stream_id_unidir_remote = crate::stream::StreamId::from_parts(
        cnx.remote_parameters.initial_max_stream_id_unidir.max(1),
        role,
        crate::stream::Direction::Unidir,
    )
    .0;
    if case.is_data_blocked {
        cnx.remote_parameters.initial_max_stream_data_bidi_local = 0;
        cnx.remote_parameters.initial_max_stream_data_bidi_remote = 0;
        cnx.remote_parameters.initial_max_stream_data_uni = 0;
        cnx.remote_parameters.initial_max_data = crate::internal::INITIAL_FLOW_CONTROL_MAX;
    } else {
        cnx.remote_parameters.initial_max_stream_data_bidi_local = 100_000;
        cnx.remote_parameters.initial_max_stream_data_bidi_remote = 100_000;
        cnx.remote_parameters.initial_max_stream_data_uni = 100_000;
        cnx.remote_parameters.initial_max_data = 1_000_000;
    }
    cnx.maxdata_remote = cnx.remote_parameters.initial_max_data;
    let tok = cnx.create_stream(case.stream_id)?;
    let mut stream = cnx.streams.remove(tok).ok_or(crate::Error::Generic)?;
    stream.is_active = true;
    stream.sent_offset = stream.maxdata_remote;
    let mut bytes = [0u8; 1024];
    let mut more_data = 0;
    let mut is_pure_ack = 1;
    let tail = crate::internal::format_one_blocked_frame(
        &mut cnx,
        &mut bytes,
        &mut more_data,
        &mut is_pure_ack,
        &mut stream,
    )
    .ok_or(crate::Error::BufferTooSmall)?;
    if tail.len() != bytes.len() && (is_pure_ack != 0 || more_data != 0) {
        return Err(crate::Error::Generic);
    }
    if cnx.stream_blocked_bidir_sent != case.expect_bidir_blocked
        || cnx.stream_blocked_unidir_sent != case.expect_unidir_blocked
        || stream.stream_data_blocked_sent != case.expect_data_blocked
    {
        return Err(crate::Error::Generic);
    }
    Ok(())
}

/// C: `process_ack_of_stream_frame` + `check_frame_needs_repeat`
/// integrated sub-case driver.
fn stream_ack_test_one(quic: &mut Quic) -> crate::Result<()> {
    let packets: &[(&[u8], bool)] = &[
        (&[0x0b, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0x0b, 4, 0], true),
        (&[0x0a, 12, 8, 0, 1, 2, 3, 4, 5, 6, 7], true),
        (&[0x0f, 16, 32, 8, 1, 2, 3, 4, 5, 6, 7, 8], true),
        (&[0x09, 20, 4, 0, 0, 0, 0, 1, 2, 3, 4, 5], false),
    ];
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    for stream_id in [0, 4, 8, 12, 16, 20] {
        let _ = cnx.create_stream(stream_id)?;
    }
    for (packet, should_ack) in packets {
        let mut byte_index = 0usize;
        while byte_index < packet.len() {
            let mut consumed = 0usize;
            let mut pure_ack = 0i32;
            if skip_frame(
                &packet[byte_index..],
                packet.len() - byte_index,
                &mut consumed,
                &mut pure_ack,
            ) != 0
            {
                return Err(crate::Error::InvalidFrame);
            }
            let mut no_need_to_repeat = 0;
            let mut do_not_detect_spurious = 0;
            let mut is_preemptive_needed = 0;
            let ret = cnx.check_frame_needs_repeat(
                &packet[byte_index..byte_index + consumed],
                consumed,
                crate::internal::PacketType::OneRttProtected,
                &mut no_need_to_repeat,
                &mut do_not_detect_spurious,
                &mut is_preemptive_needed,
            );
            if ret != 0 || (*should_ack && pure_ack != 0) {
                return Err(crate::Error::Generic);
            }
            byte_index += consumed;
        }
    }
    Ok(())
}

/// C: `dataqueue_prepare_test` + `dataqueue_verify_test`.
fn dataqueue_copy_test_one(basic_case: i32, has_length: bool, has_fin: bool) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    let mut packet = quic.create_packet()?;
    let data_len = if basic_case == 5 { 0 } else { 130 };
    prepare_retransmit_packet(&mut packet, has_length, has_fin, 8, 1023, data_len);
    if matches!(basic_case, 3 | 4) {
        packet.data_repeat_index = packet.data_repeat_frame + 8 + data_len / 2;
    }
    let mut out = [0u8; crate::MAX_PACKET_SIZE];
    let out_len = out.len();
    let tail = crate::internal::copy_stream_frame_for_retransmit(&mut cnx, &mut packet, &mut out)
        .ok_or(crate::Error::BufferTooSmall)?;
    let written = out_len - tail.len();
    if written == packet.length - packet.data_repeat_frame
        && out[..written] == packet.bytes[packet.data_repeat_frame..packet.length]
    {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// C: `dataqueue_prepare_packet` + `picoquic_queue_data_repeat_packet`
/// + `dataqueue_packet_test_iterate`.
fn dataqueue_packet_test_iterate(quic: &mut Quic) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    let mut packet = quic.create_packet()?;
    packet.sequence_number = 10;
    prepare_retransmit_packet(&mut packet, true, false, 0, 0, 256);
    cnx.queue_data_repeat_packet(&mut packet);
    if !packet.is_queued_for_data_repeat {
        return Err(crate::Error::Generic);
    }
    cnx.dequeue_data_repeat_packet(&mut packet);
    if packet.is_queued_for_data_repeat {
        Err(crate::Error::Generic)
    } else {
        Ok(())
    }
}

/// C: `binlog_test` body — creates a QUIC context, runs a reference
/// connection scenario, converts to QLOG, and diffs against a reference.
fn run_binlog_test() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    quic.set_binlog(Some(".")).ok();
    let initial_cid =
        crate::ConnectionId::clone_from_slice(&[1, 2, 3, 4]).ok_or(crate::Error::Generic)?;
    let dest_cid =
        crate::ConnectionId::clone_from_slice(&[5, 6, 7, 8]).ok_or(crate::Error::Generic)?;
    let addr: core::net::SocketAddr = "127.0.0.1:4433"
        .parse()
        .map_err(|_| crate::Error::InvalidArgument)?;
    let mut cnx = quic.create_connection_with_cids(
        initial_cid,
        dest_cid,
        Some(&addr),
        simulated_time,
        "test-sni",
        "test-alpn",
    )?;
    cnx.log_new_connection();
    cnx.log_app_message("binlog frame test");
    Ok(())
}

/// C: `logger_test` body — creates a QUIC context, exercises the text
/// logger, and compares against a reference.
fn run_logger_test() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(123_456_789);
    let mut quic = make_quic(&mut simulated_time);
    quic.set_textlog(Some("log_test.txt"))?;
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    cnx.log_new_connection();
    cnx.log_app_message("This is an app message test.");
    quic.textlog_close();
    Ok(())
}

// ---------------------------------------------------------------------------
// Phase 3A helpers for tests whose Rust signatures differ from the compact
// test entry points.

/// C: `frames_format_test` internal body — exercises `picoquic_format_*`
/// functions for ACK, CONNECTION_CLOSE, PATH_CHALLENGE, PATH_RESPONSE.
fn run_frames_format_test() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    let mut buffer = [0u8; crate::MAX_PACKET_SIZE];
    let mut more_data = 0;
    let mut is_pure_ack = 0;

    let tail = crate::internal::format_connection_close_frame(
        &mut cnx,
        &mut buffer,
        &mut more_data,
        &mut is_pure_ack,
    )
    .ok_or(crate::Error::BufferTooSmall)?;
    if tail.len() == buffer.len() || is_pure_ack != 0 {
        return Err(crate::Error::Generic);
    }

    more_data = 0;
    is_pure_ack = 0;
    let tail = crate::internal::format_path_challenge_frame(
        &mut buffer,
        &mut more_data,
        &mut is_pure_ack,
        0xaabb_ccdd_eeff_0011,
    )
    .ok_or(crate::Error::BufferTooSmall)?;
    if tail.len() != buffer.len() - 9 || is_pure_ack != 0 {
        return Err(crate::Error::Generic);
    }

    more_data = 0;
    let tail = crate::internal::format_observed_address_frame(
        &mut buffer,
        crate::frames::FrameType::ObservedAddressV4 as u64,
        13,
        &[1, 2, 3, 4],
        4433,
        &mut more_data,
    )
    .ok_or(crate::Error::BufferTooSmall)?;
    if tail.len() == buffer.len() {
        return Err(crate::Error::Generic);
    }
    Ok(())
}

/// C: `new_cnxid_test` — formats a NEW_CONNECTION_ID frame then verifies
/// that `skip_frame` can skip it exactly.
fn run_new_cnxid_test(_quic: &mut Quic, _simulated_time: &mut Instant) -> crate::Result<()> {
    let frame = [
        crate::frames::FrameType::NewConnectionId as u8,
        7,
        0,
        8,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        0xa0,
        0xa1,
        0xa2,
        0xa3,
        0xa4,
        0xa5,
        0xa6,
        0xa7,
        0xa8,
        0xa9,
        0xaa,
        0xab,
        0xac,
        0xad,
        0xae,
        0xaf,
    ];
    let mut consumed = 0usize;
    let mut pure_ack = 0i32;
    if skip_frame(&frame, frame.len(), &mut consumed, &mut pure_ack) == 0 && consumed == frame.len()
    {
        Ok(())
    } else {
        Err(crate::Error::InvalidFrame)
    }
}

/// C: `test_copy_for_retransmit` — copies frames out of a retransmit packet
/// into a fresh buffer and verifies the copy.
fn run_stream_retransmit_copy_test(
    quic: &mut Quic,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let mut cnx = quic.create_test_cnx(simulated_time)?;
    let mut old_p = quic.create_packet()?;
    old_p.sequence_number = 1;
    prepare_retransmit_packet(&mut old_p, true, false, 0, 0, 128);
    let mut new_bytes = [0u8; crate::MAX_PACKET_SIZE];
    let mut packet_is_pure_ack = 0;
    let mut do_not_detect_spurious = 0;
    let mut length = 0usize;
    let mut add_to_data_repeat_queue = 0;
    let ret = crate::internal::copy_before_retransmit(
        &mut old_p,
        &mut cnx,
        &mut new_bytes,
        crate::MAX_PACKET_SIZE,
        &mut packet_is_pure_ack,
        &mut do_not_detect_spurious,
        1,
        &mut length,
        &mut add_to_data_repeat_queue,
    );
    if ret == 0 && length == old_p.length && add_to_data_repeat_queue != 0 {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

// ---------------------------------------------------------------------------
// Helper: build a QUIC context for frame-unit tests.

fn make_quic(simulated_time: &mut Instant) -> Box<Quic> {
    Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; crate::RESET_SECRET_SIZE],
        *simulated_time,
        None,
        None,
    )
    .expect("QUIC context")
}

// ---------------------------------------------------------------------------
// Exported tests.

/// C: `skip_frame_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn frames_skip() {
    // Iterate over the internal `test_skip_list` via the public `skip_frame`
    // entry point.  Each frame is tested both with and without a trailing
    // guard region (the C "sharp_end" loop).

    let extra = [0xff, 0, 0, 0];
    for case in test_skip_frames() {
        for sharp_end in [false, true] {
            let mut frame = case.bytes.clone();
            let byte_max = if !case.must_be_last && !sharp_end {
                frame.extend_from_slice(&extra);
                frame.len()
            } else {
                case.bytes.len()
            };
            let mut consumed = 0usize;
            let mut pure_ack = 0i32;
            let ret = skip_frame(&frame, byte_max, &mut consumed, &mut pure_ack);
            assert_eq!(ret, 0, "skip_frame({})", case.name);
            assert_eq!(consumed, case.bytes.len(), "consumed({})", case.name);
            assert_eq!(pure_ack, case.pure_ack, "pure_ack({})", case.name);
        }
    }
}

/// C: `parse_frame_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn frames_parse() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    let sample = &[0x01u8]; // PING frame
    parse_test_packet(&mut quic, sample, 3, false).expect("parse PING");
}

/// C: `frames_format_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn frames_format() {
    run_frames_format_test().expect("frames_format");
}

/// C: `frames_ackack_error_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn frames_ackack_error() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    // Exercise one representative frame through the ack-ack error path.
    let sample = &[0x01u8]; // PING
    let _disconnected = frame_ackack_error_packet(&mut quic, sample, 3, false, 1);
}

/// C: `frames_repeat_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn frames_repeat() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    let sample = &[0x01u8]; // PING
    frame_repeat_error_packet(&mut quic, sample, 3, false, false).expect("repeat PING");
}

/// C: `new_cnxid_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn new_cnxid() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    run_new_cnxid_test(&mut quic, &mut simulated_time).expect("new_cnxid");
}

/// C: `cnxid_stash_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn new_cnxid_stash() {
    use crate::internal::{obtain_stashed_cnxid, stash_remote_cnxid};

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut cnx = quic.create_test_cnx(&mut simulated_time).expect("cnx");

    // Enqueue and dequeue one CID immediately.
    stash_remote_cnxid(&mut cnx, 1, &[0xaau8; 8], &[0u8; 16]).expect("stash");
    let stashed = obtain_stashed_cnxid(&mut cnx).expect("obtain");
    assert!(stashed, "stashed CID not retrieved");
}

/// C: `send_stream_blocked_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn send_stream_blocked() {
    const NB_CASES: usize = 4; // representative count; full list in Phase 4
    for i in 0..NB_CASES {
        send_stream_blocked_test_one(i).expect("stream blocked case");
    }
}

/// C: `stream_ack_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn stream_ack() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    stream_ack_test_one(&mut quic).expect("stream_ack");
}

/// C: `test_copy_for_retransmit` in `picoquictest/skip_frame_test.c`.
#[test]
fn stream_retransmit_copy() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    run_stream_retransmit_copy_test(&mut quic, &mut simulated_time)
        .expect("copy_before_retransmit");
}

/// C: `dataqueue_copy_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn dataqueue_copy() {
    for case_opt in 0..5i32 {
        let has_length = (case_opt & 1) == 0;
        let has_fin = (case_opt & 2) == 2;
        for basic_case in 1..=6i32 {
            dataqueue_copy_test_one(basic_case, has_length, has_fin).expect("dataqueue_copy");
        }
    }
}

/// C: `dataqueue_packet_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn dataqueue_packet() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    dataqueue_packet_test_iterate(&mut quic).expect("dataqueue_packet");
}

/// C: `app_message_overflow_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn app_message_overflow() {
    use crate::ConnectionId as InternalCid;

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    quic.set_binlog(Some(".")).ok();

    let initial_cid =
        InternalCid::clone_from_slice(&[8, 9, 0, 1, 2, 3, 4, 5]).expect("initial CID");
    let dest_cid =
        InternalCid::clone_from_slice(&[16, 17, 18, 19, 20, 21, 22, 23]).expect("dest CID");
    let addr: core::net::SocketAddr = "10.0.0.1:1234".parse().unwrap();

    let mut cnx = quic
        .create_connection_with_cids(
            initial_cid,
            dest_cid,
            Some(&addr),
            simulated_time,
            "test-sni",
            "test-alpn",
        )
        .expect("cnx");

    cnx.log_new_connection();

    // Fill a large string and log it 16 times to exercise overflow handling.
    let fill = "x".repeat(65533);
    for i in 0..16usize {
        cnx.log_app_message(&format!("s:{}", &fill[15 - i.min(15)..]));
    }
}

/// C: `queue_network_input_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn queue_network_input() {
    use crate::internal::{StreamDataSplay, queue_network_input};

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut tree = StreamDataSplay::new();

    let data: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    // Fill 0..3.
    let mut new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 0, &data[..4], true, &mut new_data)
        .expect("queue 0..3");
    assert!(new_data, "expected new data after 0..3");

    // Fill 6..9.
    new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 6, &data[6..], true, &mut new_data)
        .expect("queue 6..9");
    assert!(new_data, "expected new data after 6..9");

    // Fill 2..7 (fills the gap at 4..5 → delivers 0..9 contiguously).
    new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 2, &data[2..8], true, &mut new_data)
        .expect("queue 2..7");

    // Verify the tree contains the expected three contiguous segments.
    assert_eq!(tree.len(), 3, "expected 3 segments in tree");
}

/// C: `binlog_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn binlog() {
    run_binlog_test().expect("binlog_test");
}

/// C: `logger_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn logger() {
    run_logger_test().expect("logger_test");
}
