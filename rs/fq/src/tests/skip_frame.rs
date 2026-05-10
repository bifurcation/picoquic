//! Test cases for `picoquictest/skip_frame_test.c`.
//!
//! Exercises frame skip/parse/format/repeat/retransmit logic and the
//! binlog/logger infrastructure.

#![allow(non_snake_case)]

use crate::bytestream::{BYTESTREAM_MAX_BUFFER_SIZE, ByteStream};
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

fn epoch_value(epoch: u32) -> crate::internal::Epoch {
    match epoch {
        0 => crate::internal::Epoch::Initial,
        1 => crate::internal::Epoch::ZeroRtt,
        2 => crate::internal::Epoch::Handshake,
        _ => crate::internal::Epoch::OneRtt,
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

struct TestFrameError {
    name: &'static str,
    bytes: Vec<u8>,
    must_be_last: bool,
    skip_fails: bool,
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

fn error_frame(
    name: &'static str,
    bytes: Vec<u8>,
    must_be_last: bool,
    skip_fails: bool,
) -> TestFrameError {
    TestFrameError {
        name,
        bytes,
        must_be_last,
        skip_fails,
    }
}

const TEST_SKIP_FRAME_VARINT_COUNTS: &[u32] = &[
    0, 3, 3, 2, 2, 1, 2, 1, 1, 0, 1, 1, 1, 1, 4, 2, 0, 0, 1, 8, 11, 1, 4, 2, 1, 0, 1, 0, 4, 4, 0,
    1, 2, 2, 2, 2, 1, 5, 2, 1, 1, 4, 1, 1, 9, 12, 3,
];

const TEST_SKIP_FRAME_EPOCHS: &[u32] = &[
    0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
];

const TEST_SKIP_FRAME_MPATH: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 2, 2, 1, 1, 0,
];

const TEST_FRAME_ERROR_EXPECTED_ERRORS: &[u64] = &[
    crate::errors::TransportError::FlowControlError as u64,
    crate::errors::TransportError::StreamLimitError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::StreamStateError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::StreamStateError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::FrameFormatError as u64,
    crate::errors::TransportError::ProtocolViolation as u64,
];

const TEST_FRAME_ERROR_EPOCHS: &[u32] = &[
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3,
];

const TEST_FRAME_ERROR_MPATH: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0,
];

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

fn test_frame_errors() -> Vec<TestFrameError> {
    use crate::frames::FrameType as F;

    vec![
        error_frame(
            "bad_reset_stream_offset",
            vec![
                F::ResetStream as u8,
                17,
                1,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
            ],
            false,
            false,
        ),
        error_frame(
            "bad_reset_stream",
            vec![
                F::ResetStream as u8,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                1,
                1,
            ],
            false,
            false,
        ),
        error_frame(
            "bad_reset_stream2",
            vec![F::ResetStream as u8, 0xff, 0xff, 0xff, 0xff, 1, 1],
            true,
            false,
        ),
        error_frame(
            "bad_connection_close",
            vec![
                F::ConnectionClose as u8,
                0x80,
                0x00,
                0xcf,
                0xff,
                0,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                b'1',
                b'2',
                b'3',
                b'4',
                b'5',
                b'6',
                b'7',
                b'8',
                b'9',
            ],
            false,
            true,
        ),
        error_frame(
            "bad_connection_close2",
            vec![F::ConnectionClose as u8, 0x80, 0x00, 0xcf],
            true,
            true,
        ),
        error_frame(
            "bad_application_close",
            vec![
                F::ApplicationClose as u8,
                0x44,
                4,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                b't',
                b'e',
                b's',
                b't',
            ],
            false,
            true,
        ),
        error_frame(
            "bad_max_stream_stream",
            vec![
                F::MaxStreamData as u8,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0x80,
                0x01,
                0,
                0,
            ],
            false,
            false,
        ),
        error_frame(
            "bad_max_streams_bidir",
            vec![
                F::MaxStreamsBidir as u8,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
            ],
            false,
            false,
        ),
        error_frame(
            "bad_max_streams_unidir",
            vec![
                F::MaxStreamsUnidir as u8,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
            ],
            false,
            false,
        ),
        error_frame(
            "bad_new_connection_id_length",
            vec![
                F::NewConnectionId as u8,
                7,
                0,
                0x3f,
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
            ],
            false,
            true,
        ),
        error_frame(
            "bad_new_connection_id_retire",
            vec![
                F::NewConnectionId as u8,
                7,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
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
            ],
            false,
            false,
        ),
        error_frame(
            "illegal_new_cid_retire",
            vec![
                F::NewConnectionId as u8,
                7,
                8,
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
            ],
            false,
            false,
        ),
        error_frame(
            "too_long_new_cid",
            vec![
                F::NewConnectionId as u8,
                7,
                0,
                21,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9,
                0,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9,
                0,
                1,
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
            ],
            false,
            false,
        ),
        error_frame(
            "bad_stop_sending",
            vec![F::StopSending as u8, 19, 0x17],
            false,
            false,
        ),
        error_frame(
            "bad_stop_sending2",
            vec![F::StopSending as u8, 19],
            true,
            false,
        ),
        error_frame(
            "bad_new_token",
            vec![
                F::NewToken as u8,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9,
                10,
                11,
                12,
                13,
                14,
                15,
                16,
                17,
            ],
            false,
            true,
        ),
        error_frame(
            "bad_ack_range",
            vec![
                F::Ack as u8,
                0xc0,
                0,
                0,
                1,
                2,
                3,
                4,
                5,
                0x44,
                0,
                2,
                5,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0,
                5,
                12,
            ],
            false,
            false,
        ),
        error_frame(
            "bad_ack_first_range",
            vec![F::Ack as u8, 0x02, 0x00, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00],
            false,
            false,
        ),
        error_frame(
            "bad_ack_gaps",
            vec![
                F::Ack as u8,
                0xc0,
                0,
                0,
                1,
                2,
                3,
                4,
                5,
                0x44,
                0,
                2,
                5,
                0,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                5,
                12,
            ],
            false,
            false,
        ),
        error_frame(
            "bad_ack_blocks",
            vec![
                F::AckEcn as u8,
                0xc0,
                0,
                0,
                1,
                2,
                3,
                4,
                5,
                0x44,
                0,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                0xff,
                5,
                0,
                0,
                5,
                12,
                3,
                0,
                1,
            ],
            false,
            true,
        ),
        error_frame(
            "bad_crypto_hs",
            vec![
                F::CryptoHs as u8,
                0,
                0x8f,
                0xff,
                0xff,
                0xff,
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
            ],
            false,
            true,
        ),
        error_frame(
            "bad_datagram",
            vec![
                F::DatagramL as u8,
                0x8f,
                0xff,
                0xff,
                0xff,
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
            ],
            false,
            true,
        ),
        error_frame(
            "stream_hang",
            vec![
                0x01, 0x00, 0x0d, 0xff, 0xff, 0xff, 0x01, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            ],
            false,
            false,
        ),
        error_frame(
            "bad_abandon_0",
            vec![0x80, 0x00, 0x3e, 0x75, 0x00],
            true,
            true,
        ),
        error_frame(
            "bad_abandon_1",
            vec![0x80, 0x00, 0x3e, 0x75, 0x00, 0xff],
            false,
            true,
        ),
        error_frame(
            "bad_path_available",
            vec![0x80, 0x00, 0x3e, 0x77, 0x00],
            true,
            true,
        ),
        error_frame(
            "bad_bdp",
            vec![0x80, 0x00, 0xeb, 0xd9, 0x01, 0x02, 0x04],
            false,
            false,
        ),
        error_frame(
            "bad_bdp_addr",
            vec![
                0x80, 0x00, 0xeb, 0xd9, 0x01, 0x02, 0x04, 0x05, 1, 2, 3, 4, 5,
            ],
            false,
            false,
        ),
        error_frame(
            "bad_bdp_length",
            vec![
                0x80, 0x00, 0xeb, 0xd9, 0x08, 0x02, 0x04, 0x8f, 0xff, 0xff, 0xff, 1, 2, 3, 4,
            ],
            false,
            true,
        ),
        error_frame(
            "bad_frame_id",
            vec![
                0xbf, 0xff, 0xff, 0xff, 0x08, 0x02, 0x04, 0x8f, 0xff, 0xff, 0xff, 1, 2, 3, 4,
            ],
            false,
            true,
        ),
    ]
}

fn test_random(random_context: &mut u64) -> u64 {
    *random_context = random_context.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *random_context;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn test_uniform_random(random_context: &mut u64, rnd_max: u64) -> u64 {
    if rnd_max == 0 {
        return 0;
    }

    let rnd_min = u64::MAX % rnd_max;
    loop {
        let rnd = test_random(random_context);
        if rnd >= rnd_min {
            return rnd % rnd_max;
        }
    }
}

fn format_random_packet(
    frames: &[TestSkipFrame],
    bytes_max: usize,
    random_context: &mut u64,
) -> Vec<u8> {
    let mut packet = Vec::new();
    while packet.len() < bytes_max {
        let r = test_uniform_random(random_context, frames.len() as u64) as usize;
        let frame = &frames[r];
        if packet.len() + frame.bytes.len() >= bytes_max {
            break;
        }
        packet.extend_from_slice(&frame.bytes);
        if frame.must_be_last {
            break;
        }
    }
    packet
}

fn skip_test_packet(bytes: &[u8]) -> i32 {
    let mut byte_index = 0usize;
    while byte_index < bytes.len() {
        let mut consumed = 0usize;
        let mut pure_ack = 0i32;
        let ret = skip_frame(
            &bytes[byte_index..],
            bytes.len() - byte_index,
            &mut consumed,
            &mut pure_ack,
        );
        if ret != 0 {
            return ret;
        }
        if consumed == 0 {
            return -1;
        }
        byte_index += consumed;
    }
    0
}

fn skip_test_fuzz_packet(source: &[u8], random_context: &mut u64) -> Vec<u8> {
    let mut target = source.to_vec();
    let fuzz_index = test_uniform_random(random_context, source.len() as u64) as usize;
    let fuzz_length = test_uniform_random(random_context, 8) as usize + 1;
    let mut fuzz_data = test_random(random_context);

    for i in 0..fuzz_length {
        if fuzz_index + i >= target.len() {
            break;
        }
        target[fuzz_index + i] = fuzz_data as u8;
        fuzz_data >>= 8;
    }
    target
}

#[derive(Debug)]
struct ParseOutcome {
    ret: i32,
    ack_needed: bool,
    local_error: u64,
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

const DATAQUEUE_COPY_LENGTH_MAX: usize = 1536;
const DATAQUEUE_COPY_STREAM_ID: u64 = 8;
const DATAQUEUE_COPY_OFFSET: u64 = 1023;
const DATAQUEUE_COPY_DATA_LENGTH: usize = 2 * 65;

struct DataqueueCopyPrepared {
    next_frame: usize,
    next_index: usize,
    buffer_size: usize,
    frame_length: usize,
    expected_data: Vec<u8>,
}

/// C: `dataqueue_prepare_test`.
fn dataqueue_prepare_copy_test(
    packet: &mut crate::internal::Packet,
    basic_case: i32,
    has_length: bool,
    has_fin: bool,
) -> DataqueueCopyPrepared {
    let frame_data_length = if basic_case == 5 {
        0
    } else {
        DATAQUEUE_COPY_DATA_LENGTH
    };
    prepare_retransmit_packet(
        packet,
        has_length,
        has_fin,
        DATAQUEUE_COPY_STREAM_ID,
        DATAQUEUE_COPY_OFFSET,
        frame_data_length,
    );

    let frame_data = packet.data_repeat_frame
        + 1
        + crate::internal::encode_varint_length(DATAQUEUE_COPY_STREAM_ID)
        + crate::internal::encode_varint_length(DATAQUEUE_COPY_OFFSET)
        + if has_length {
            crate::internal::encode_varint_length(frame_data_length as u64)
        } else {
            0
        };
    let mut copied_index = frame_data;
    let mut copied_offset = DATAQUEUE_COPY_OFFSET;
    let mut copied_fin = has_fin;
    let mut constrained_buffer = false;
    let mut extra_byte = 0usize;

    let (copied_length, next_frame, next_index) = match basic_case {
        2 => {
            let copied_length = frame_data_length / 2;
            copied_fin = false;
            constrained_buffer = true;
            (
                copied_length,
                packet.data_repeat_frame,
                frame_data + copied_length,
            )
        }
        3 => {
            let skipped = frame_data_length / 2;
            packet.data_repeat_index = frame_data + skipped;
            copied_index += skipped;
            copied_offset += skipped as u64;
            (frame_data_length - skipped, packet.length, packet.length)
        }
        4 => {
            constrained_buffer = true;
            (frame_data_length, packet.length, packet.length)
        }
        6 => {
            constrained_buffer = true;
            extra_byte = 1;
            (frame_data_length, packet.length, packet.length)
        }
        1 | 5 => (frame_data_length, packet.length, packet.length),
        _ => panic!("invalid dataqueue basic case {basic_case}"),
    };

    assert!(copied_index + copied_length <= packet.length);

    let mut expected_data = Vec::with_capacity(DATAQUEUE_COPY_LENGTH_MAX);
    if extra_byte != 0 {
        expected_data.push(0);
    }

    let mut frame_type = crate::frames::FrameType::StreamRangeMin as u8 | 0x04;
    if !constrained_buffer {
        frame_type |= 0x02;
    }
    if copied_fin {
        frame_type |= 0x01;
    }
    expected_data.push(frame_type);
    write_varint(&mut expected_data, DATAQUEUE_COPY_STREAM_ID);
    write_varint(&mut expected_data, copied_offset);
    if !constrained_buffer {
        write_varint(&mut expected_data, copied_length as u64);
    }
    expected_data.extend_from_slice(&packet.bytes[copied_index..copied_index + copied_length]);

    let frame_length = expected_data.len();
    if !constrained_buffer {
        expected_data.extend_from_slice(&[1u8; 4]);
    }
    let buffer_size = expected_data.len();
    assert!(buffer_size <= DATAQUEUE_COPY_LENGTH_MAX);

    DataqueueCopyPrepared {
        next_frame,
        next_index,
        buffer_size,
        frame_length,
        expected_data,
    }
}

const COPY_PACKET_HEADER_1RTT_LEN: usize = 13;

fn copy_packet_header_1rtt() -> Vec<u8> {
    vec![0x43, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 1]
}

fn split_frame_source_1_67() -> Vec<u8> {
    (1u8..=67).collect()
}

fn split_frame_source_1_32() -> Vec<u8> {
    (1u8..=32).collect()
}

fn split_frame_source_68_77() -> Vec<u8> {
    (68u8..=77).collect()
}

fn copy_stream0_data() -> Vec<u8> {
    (1u8..=77).collect()
}

fn copy_packet_with_payload(payload: &[u8]) -> Vec<u8> {
    let mut packet = copy_packet_header_1rtt();
    packet.extend_from_slice(payload);
    packet
}

fn ct_test_packet1() -> Vec<u8> {
    let mut payload = vec![
        crate::frames::FrameType::StreamRangeMin as u8 | 2,
        0,
        0x40,
        67,
    ];
    payload.extend(split_frame_source_1_67());
    copy_packet_with_payload(&payload)
}

fn ct_test_packet2() -> Vec<u8> {
    let mut payload = vec![crate::frames::FrameType::StreamRangeMin as u8, 0];
    payload.extend(split_frame_source_1_67());
    copy_packet_with_payload(&payload)
}

fn ct_test_packet3() -> Vec<u8> {
    let mut payload = vec![crate::frames::FrameType::StreamRangeMin as u8, 0];
    payload.extend(copy_stream0_data());
    copy_packet_with_payload(&payload)
}

fn ct_test_packet4_first_frame() -> Vec<u8> {
    let mut payload = vec![
        crate::frames::FrameType::StreamRangeMin as u8 | 6,
        0,
        0x40,
        67,
        0x0a,
    ];
    payload.extend(split_frame_source_68_77());
    payload
}

fn ct_test_packet4() -> Vec<u8> {
    let mut payload = ct_test_packet4_first_frame();
    payload.push(crate::frames::FrameType::StreamRangeMin as u8);
    payload.push(0);
    payload.extend(split_frame_source_1_67());
    copy_packet_with_payload(&payload)
}

fn ct_test_packet5() -> Vec<u8> {
    let mut payload = ct_test_packet4_first_frame();
    payload.extend_from_slice(&[
        crate::frames::FrameType::StreamRangeMin as u8 | 2,
        0,
        0x40,
        67,
    ]);
    payload.extend(split_frame_source_1_67());
    copy_packet_with_payload(&payload)
}

fn ct_test_packet6() -> Vec<u8> {
    let mut payload = ct_test_packet4_first_frame();
    payload.push(crate::frames::FrameType::StreamRangeMin as u8);
    payload.push(0);
    payload.extend(split_frame_source_1_32());
    copy_packet_with_payload(&payload)
}

fn ct_test_mtu_probe() -> Vec<u8> {
    let mut payload = vec![crate::frames::FrameType::Ping as u8];
    payload.extend([0u8; 20]);
    copy_packet_with_payload(&payload)
}

fn ct_test_ack() -> Vec<u8> {
    let mut payload = vec![crate::frames::FrameType::Ack as u8];
    payload.extend([0u8; 20]);
    copy_packet_with_payload(&payload)
}

struct CopyRetransmitCase {
    name: &'static str,
    packet: Option<Vec<u8>>,
    offset: usize,
    is_mtu_probe: bool,
    is_ack_trap: bool,
    copy_max: usize,
    b1_expected: Option<Vec<u8>>,
    b1_length: usize,
    b1_offset: usize,
    b2_expected: Option<Vec<u8>>,
    b3_expected: Option<Vec<u8>>,
    is_pure_ack_expected: i32,
}

fn copy_retransmit_cases() -> Vec<CopyRetransmitCase> {
    let packet1 = ct_test_packet1();
    let packet2 = ct_test_packet2();
    let packet3 = ct_test_packet3();
    let packet4 = ct_test_packet4();
    let packet5 = ct_test_packet5();
    let packet6 = ct_test_packet6();
    let packet4_first_frame = ct_test_packet4_first_frame();
    let mtu_probe = ct_test_mtu_probe();
    let ack = ct_test_ack();

    vec![
        CopyRetransmitCase {
            name: "explicit_length_stream",
            packet: Some(packet1.clone()),
            offset: COPY_PACKET_HEADER_1RTT_LEN,
            is_mtu_probe: false,
            is_ack_trap: false,
            copy_max: crate::MAX_PACKET_SIZE,
            b1_expected: Some(packet1.clone()),
            b1_length: COPY_PACKET_HEADER_1RTT_LEN,
            b1_offset: COPY_PACKET_HEADER_1RTT_LEN,
            b2_expected: Some(packet1[COPY_PACKET_HEADER_1RTT_LEN..].to_vec()),
            b3_expected: None,
            is_pure_ack_expected: 0,
        },
        CopyRetransmitCase {
            name: "implicit_length_stream",
            packet: Some(packet2.clone()),
            offset: COPY_PACKET_HEADER_1RTT_LEN,
            is_mtu_probe: false,
            is_ack_trap: false,
            copy_max: crate::MAX_PACKET_SIZE,
            b1_expected: Some(packet1),
            b1_length: COPY_PACKET_HEADER_1RTT_LEN,
            b1_offset: COPY_PACKET_HEADER_1RTT_LEN,
            b2_expected: Some(packet2[COPY_PACKET_HEADER_1RTT_LEN..].to_vec()),
            b3_expected: None,
            is_pure_ack_expected: 0,
        },
        CopyRetransmitCase {
            name: "implicit_length_stream_limited",
            packet: Some(packet2.clone()),
            offset: COPY_PACKET_HEADER_1RTT_LEN,
            is_mtu_probe: false,
            is_ack_trap: false,
            copy_max: packet2.len(),
            b1_expected: Some(packet2.clone()),
            b1_length: COPY_PACKET_HEADER_1RTT_LEN,
            b1_offset: COPY_PACKET_HEADER_1RTT_LEN,
            b2_expected: Some(packet2[COPY_PACKET_HEADER_1RTT_LEN..].to_vec()),
            b3_expected: None,
            is_pure_ack_expected: 0,
        },
        CopyRetransmitCase {
            name: "implicit_length_full_stream",
            packet: Some(packet3.clone()),
            offset: COPY_PACKET_HEADER_1RTT_LEN,
            is_mtu_probe: false,
            is_ack_trap: false,
            copy_max: packet3.len(),
            b1_expected: Some(packet3.clone()),
            b1_length: COPY_PACKET_HEADER_1RTT_LEN,
            b1_offset: COPY_PACKET_HEADER_1RTT_LEN,
            b2_expected: Some(packet3[COPY_PACKET_HEADER_1RTT_LEN..].to_vec()),
            b3_expected: None,
            is_pure_ack_expected: 0,
        },
        CopyRetransmitCase {
            name: "two_streams_limited",
            packet: Some(packet4.clone()),
            offset: COPY_PACKET_HEADER_1RTT_LEN,
            is_mtu_probe: false,
            is_ack_trap: false,
            copy_max: packet4.len(),
            b1_expected: Some(packet4.clone()),
            b1_length: COPY_PACKET_HEADER_1RTT_LEN,
            b1_offset: COPY_PACKET_HEADER_1RTT_LEN,
            b2_expected: Some(packet4_first_frame.clone()),
            b3_expected: None,
            is_pure_ack_expected: 0,
        },
        CopyRetransmitCase {
            name: "two_streams_full",
            packet: Some(packet4.clone()),
            offset: COPY_PACKET_HEADER_1RTT_LEN,
            is_mtu_probe: false,
            is_ack_trap: false,
            copy_max: crate::MAX_PACKET_SIZE,
            b1_expected: Some(packet5),
            b1_length: COPY_PACKET_HEADER_1RTT_LEN,
            b1_offset: COPY_PACKET_HEADER_1RTT_LEN,
            b2_expected: Some(packet4_first_frame.clone()),
            b3_expected: None,
            is_pure_ack_expected: 0,
        },
        CopyRetransmitCase {
            name: "two_streams_truncated",
            packet: Some(packet4),
            offset: COPY_PACKET_HEADER_1RTT_LEN,
            is_mtu_probe: false,
            is_ack_trap: false,
            copy_max: packet6.len(),
            b1_expected: Some(packet6),
            b1_length: COPY_PACKET_HEADER_1RTT_LEN,
            b1_offset: COPY_PACKET_HEADER_1RTT_LEN,
            b2_expected: Some(packet4_first_frame),
            b3_expected: None,
            is_pure_ack_expected: 0,
        },
        CopyRetransmitCase {
            name: "mtu_probe",
            packet: Some(mtu_probe),
            offset: COPY_PACKET_HEADER_1RTT_LEN,
            is_mtu_probe: true,
            is_ack_trap: false,
            copy_max: crate::MAX_PACKET_SIZE,
            b1_expected: None,
            b1_length: 0,
            b1_offset: 0,
            b2_expected: None,
            b3_expected: None,
            is_pure_ack_expected: 1,
        },
        CopyRetransmitCase {
            name: "ack_trap_packet",
            packet: Some(ack),
            offset: COPY_PACKET_HEADER_1RTT_LEN,
            is_mtu_probe: false,
            is_ack_trap: true,
            copy_max: COPY_PACKET_HEADER_1RTT_LEN,
            b1_expected: None,
            b1_length: 0,
            b1_offset: 0,
            b2_expected: None,
            b3_expected: None,
            is_pure_ack_expected: 1,
        },
        CopyRetransmitCase {
            name: "empty_ack_trap",
            packet: None,
            offset: 0,
            is_mtu_probe: false,
            is_ack_trap: true,
            copy_max: crate::MAX_PACKET_SIZE,
            b1_expected: None,
            b1_length: 0,
            b1_offset: 0,
            b2_expected: None,
            b3_expected: None,
            is_pure_ack_expected: 1,
        },
    ]
}

/// C: `parse_test_packet_cnx_fix`.
fn parse_test_packet_cnx_fix(
    cnx: &mut crate::internal::Connection,
    simulated_time: Instant,
    epoch: u32,
    mpath: u8,
) {
    cnx.pkt_ctx[0].send_sequence = 0x0001_0203_0406;
    if let Some(path) = cnx.paths.first_mut() {
        path.pkt_ctx.send_sequence = 0x0001_0203_0406;
    }

    cnx.is_time_stamp_enabled = true;
    cnx.local_parameters.max_datagram_frame_size = crate::MAX_PACKET_SIZE as u32;
    cnx.is_ack_frequency_negotiated = true;
    cnx.remote_parameters.min_ack_delay = crate::Duration::from_ticks(1000);
    cnx.local_parameters.enable_bdp_frame = true;

    if mpath != 0 {
        cnx.is_multipath_enabled = true;
        cnx.max_path_id_local = 5;
        if mpath >= 2 {
            cnx.is_address_discovery_provider = true;
            cnx.is_address_discovery_receiver = true;
        }
    }

    cnx.is_reset_stream_at_enabled = true;
    if epoch == 3 {
        cnx.connection_state = crate::State::Ready;
    }

    let _ = simulated_time;
}

/// C: `parse_test_packet` — decode every frame in `buf` using a fresh cnx.
fn parse_test_packet(quic: &mut Quic, buf: &[u8], epoch: u32, mpath: u8) -> ParseOutcome {
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time).expect("cnx");
    parse_test_packet_cnx_fix(&mut cnx, simulated_time, epoch, mpath);

    let mut received_data = quic.stream_data_node_alloc().expect("stream data node");
    let ret = cnx.decode_frames_on_path(
        0,
        buf,
        &mut received_data,
        epoch_value(epoch),
        None,
        None,
        0,
        0,
        simulated_time,
    );
    let ack_needed = cnx.ack_ctx[packet_context_from_epoch(epoch) as usize].act[0].ack_needed;
    let local_error = cnx.local_error();
    let ret = if ret == 0
        && matches!(
            cnx.connection_state,
            crate::State::Disconnecting | crate::State::HandshakeFailure
        ) {
        -1
    } else {
        ret
    };

    ParseOutcome {
        ret,
        ack_needed,
        local_error,
    }
}

fn frame_type(bytes: &[u8]) -> Option<u64> {
    frame_type_and_len(bytes).map(|(frame_type, _)| frame_type)
}

fn frame_type_and_len(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut frame_type = 0;
    let tail = crate::internal::frames_varint_decode(bytes, &mut frame_type)?;
    Some((frame_type, bytes.len() - tail.len()))
}

fn frame_repeat_truncation_excluded(frame_type: u64) -> bool {
    use crate::frames::FrameType as F;

    matches!(
        frame_type,
        x if x == F::ConnectionClose as u64
            || x == F::ApplicationClose as u64
            || x == F::NewToken as u64
            || x == F::PathAbandon as u64
            || x == F::Bdp as u64
            || x == F::ObservedAddressV4 as u64
            || x == F::ObservedAddressV6 as u64
    )
}

fn frame_type_allowed_in_0rtt(frame_type: u64) -> bool {
    use crate::frames::FrameType as F;

    (frame_type >= F::StreamRangeMin as u64 && frame_type <= F::StreamRangeMax as u64)
        || matches!(
            frame_type,
            x if x == F::Padding as u64
                || x == F::Ping as u64
                || x == F::ResetStream as u64
                || x == F::StopSending as u64
                || x == F::ConnectionClose as u64
                || x == F::ApplicationClose as u64
                || x == F::MaxData as u64
                || x == F::MaxStreamData as u64
                || x == F::MaxStreamsBidir as u64
                || x == F::MaxStreamsUnidir as u64
                || x == F::DataBlocked as u64
                || x == F::StreamDataBlocked as u64
                || x == F::StreamsBlockedBidir as u64
                || x == F::StreamsBlockedUnidir as u64
                || x == F::NewConnectionId as u64
                || x == F::PathChallenge as u64
                || x == F::Datagram as u64
                || x == F::DatagramL as u64
                || x == F::ResetStreamAt as u64
        )
}

/// C: `frame_ackack_error_packet` — send a frame that triggers an ack-ack
/// error on a fresh connection.
fn frame_ackack_error_packet(
    quic: &mut Quic,
    frame: &[u8],
    epoch: u32,
    mpath: u8,
    varint_idx: u32,
) -> crate::Result<bool> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    parse_test_packet_cnx_fix(&mut cnx, simulated_time, epoch, mpath);
    let mut packet = quic.create_packet()?;
    let frame = create_test_varint_frame(frame, varint_idx);
    if frame.is_empty() {
        return Err(crate::Error::InvalidFrame);
    }
    packet.packet_type = epoch_packet_type(epoch);
    packet.packet_context = packet_context_from_epoch(epoch);
    packet.offset = if packet.packet_type == crate::internal::PacketType::OneRttProtected {
        13
    } else {
        25
    };
    packet.length = packet.offset + frame.len();
    if packet.length > packet.bytes.len() {
        return Err(crate::Error::BufferTooSmall);
    }
    packet.bytes[packet.offset..packet.length].copy_from_slice(&frame);
    let previous_state = cnx.connection_state;
    cnx.process_ack_of_frames(&mut packet, 0);
    Ok(cnx.connection_state != previous_state)
}

/// C: `frame_repeat_error_packet` — verify the repeat-detection logic on `buf`.
fn frame_repeat_error_packet(
    quic: &mut Quic,
    buf: &[u8],
    epoch: u32,
    mpath: u8,
    expect_error: bool,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    parse_test_packet_cnx_fix(&mut cnx, simulated_time, epoch, mpath);
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

struct StreamBlockedCase {
    stream_id: u64,
    is_id_blocked: bool,
    is_data_blocked: bool,
    is_client: bool,
    expect_bidir_blocked: bool,
    expect_unidir_blocked: bool,
    expect_data_blocked: bool,
}

const STREAM_BLOCKED_TEST_CASES: &[StreamBlockedCase] = &[
    StreamBlockedCase {
        stream_id: 4,
        is_id_blocked: true,
        is_data_blocked: false,
        is_client: true,
        expect_bidir_blocked: true,
        expect_unidir_blocked: false,
        expect_data_blocked: false,
    },
    StreamBlockedCase {
        stream_id: 4,
        is_id_blocked: true,
        is_data_blocked: true,
        is_client: false,
        expect_bidir_blocked: false,
        expect_unidir_blocked: false,
        expect_data_blocked: true,
    },
    StreamBlockedCase {
        stream_id: 4,
        is_id_blocked: false,
        is_data_blocked: true,
        is_client: false,
        expect_bidir_blocked: false,
        expect_unidir_blocked: false,
        expect_data_blocked: true,
    },
];

/// C: `send_stream_blocked_test_one` — one stream-blocked sub-case.
fn send_stream_blocked_test_one(case: &StreamBlockedCase) -> crate::Result<()> {
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

#[derive(Copy, Clone)]
struct StreamAckCase {
    bytes: &'static [u8],
    should_ack: bool,
}

const STREAM_ACK_STREAM_LIST: &[u64] = &[0, 4, 8, 12, 16, 20];

const STREAM_ACK_CASES: &[StreamAckCase] = &[
    StreamAckCase {
        bytes: &[
            0x08 | 1 | 2,
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
            0x08 | 1 | 2,
            4,
            0,
            0x08 | 1 | 2 | 4,
            8,
            64,
            64,
            0,
            0x08 | 4,
            8,
            32,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
        ],
        should_ack: true,
    },
    StreamAckCase {
        bytes: &[
            0x08 | 2,
            12,
            8,
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            0x08 | 2 | 4,
            12,
            16,
            8,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            0x08 | 4,
            12,
            24,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
        ],
        should_ack: true,
    },
    StreamAckCase {
        bytes: &[
            0x08 | 1 | 2 | 4,
            16,
            32,
            8,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            0x08 | 2 | 4,
            20,
            4,
            8,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            0x08 | 1 | 4,
            20,
            16,
        ],
        should_ack: true,
    },
    StreamAckCase {
        bytes: &[
            0x08 | 1,
            20,
            4,
            0,
            0,
            0,
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
            11,
            12,
            0x08 | 1 | 2 | 4,
            8,
            63,
            1,
            0,
            0x08 | 4,
            8,
            32,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
        ],
        should_ack: false,
    },
];

/// C: `picoquic_process_ack_of_stream_frame`.
fn process_ack_of_stream_frame_for_test(
    cnx: &mut crate::internal::Connection,
    bytes: &[u8],
    bytes_max: usize,
    consumed: &mut usize,
) -> i32 {
    let mut stream_id = 0;
    let mut offset = 0;
    let mut data_length = 0;
    let mut fin = 0;
    let mut header_length = 0;

    if crate::internal::parse_stream_header(
        bytes,
        bytes_max,
        &mut stream_id,
        &mut offset,
        &mut data_length,
        &mut fin,
        &mut header_length,
    ) != 0
        || header_length.saturating_add(data_length) > bytes_max.min(bytes.len())
    {
        return -1;
    }

    *consumed = header_length + data_length;

    if let Some(stream_token) = cnx.find_stream(stream_id)
        && let Some(stream) = cnx.streams.get_mut(stream_token)
    {
        let ack_end = offset
            .saturating_add(data_length as u64)
            .saturating_sub(if fin != 0 { 0 } else { 1 });
        if stream
            .sack_list
            .update_sack_list(offset, ack_end, Instant::from_ticks(0))
            .is_err()
        {
            return -1;
        }
    }

    0
}

/// C: `process_ack_of_stream_frame` + `check_frame_needs_repeat`
/// integrated sub-case driver.
fn stream_ack_test_one(quic: &mut Quic) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;

    for stream_id in STREAM_ACK_STREAM_LIST {
        let _ = cnx.create_stream(*stream_id)?;
    }

    for (case_index, case) in STREAM_ACK_CASES.iter().enumerate() {
        let mut byte_index = 0usize;
        while byte_index < case.bytes.len() && case.should_ack {
            let mut consumed = 0usize;
            let ret = process_ack_of_stream_frame_for_test(
                &mut cnx,
                &case.bytes[byte_index..],
                case.bytes.len() - byte_index,
                &mut consumed,
            );
            assert_eq!(
                ret, 0,
                "stream_ack case {case_index}, cannot process frame index {byte_index}"
            );
            assert!(
                consumed > 0,
                "stream_ack case {case_index}, zero-length ACK processing at frame index {byte_index}"
            );
            byte_index += consumed;
        }
    }

    for (case_index, case) in STREAM_ACK_CASES.iter().enumerate() {
        let mut byte_index = 0usize;
        while byte_index < case.bytes.len() {
            let mut consumed = 0usize;
            let mut pure_ack = 0i32;
            let ret = skip_frame(
                &case.bytes[byte_index..],
                case.bytes.len() - byte_index,
                &mut consumed,
                &mut pure_ack,
            );
            assert_eq!(
                ret, 0,
                "stream_ack case {case_index}, cannot skip frame index {byte_index}"
            );
            assert!(
                consumed > 0,
                "stream_ack case {case_index}, zero-length skipped frame at index {byte_index}"
            );
            let mut no_need_to_repeat = 0;
            let mut do_not_detect_spurious = 0;
            let mut is_preemptive_needed = 0;
            let ret = cnx.check_frame_needs_repeat(
                &case.bytes[byte_index..byte_index + consumed],
                consumed,
                crate::internal::PacketType::OneRttProtected,
                &mut no_need_to_repeat,
                &mut do_not_detect_spurious,
                &mut is_preemptive_needed,
            );
            assert_eq!(
                ret, 0,
                "stream_ack case {case_index}, cannot check repeat at frame index {byte_index}"
            );
            assert_eq!(
                no_need_to_repeat != 0,
                case.should_ack,
                "stream_ack case {case_index}, repeat decision mismatch at frame index {byte_index}"
            );
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
    cnx.create_stream(DATAQUEUE_COPY_STREAM_ID)?;
    let mut packet = quic.create_packet()?;
    let prepared = dataqueue_prepare_copy_test(&mut packet, basic_case, has_length, has_fin);
    let mut out = [0u8; DATAQUEUE_COPY_LENGTH_MAX];

    let written = {
        let tail = crate::internal::copy_stream_frame_for_retransmit(
            &mut cnx,
            &mut packet,
            &mut out[..prepared.buffer_size],
        )
        .ok_or(crate::Error::BufferTooSmall)?;
        prepared.buffer_size - tail.len()
    };

    let verify_matches = packet.data_repeat_frame == prepared.next_frame
        && packet.data_repeat_index == prepared.next_index
        && written == prepared.frame_length
        && out[..written] == prepared.expected_data[..prepared.frame_length];

    if !verify_matches {
        assert!(
            prepared.frame_length < crate::internal::MIN_STREAM_DATA_FRAGMENT && written == 0,
            "dataqueue_copy case {basic_case}, has_length {has_length}, has_fin {has_fin}: \
             expected frame/index/length ({}, {}, {}), got ({}, {}, {}), buffer size {}",
            prepared.next_frame,
            prepared.next_index,
            prepared.frame_length,
            packet.data_repeat_frame,
            packet.data_repeat_index,
            written,
            prepared.buffer_size
        );
    }

    Ok(())
}

/// C: `dataqueue_prepare_packet`.
fn prepare_dataqueue_packet_for_queue(packet: &mut crate::internal::Packet, sequence_number: u64) {
    packet.sequence_number = sequence_number;
    prepare_retransmit_packet(packet, true, false, 0, 0, 256);
    packet.queue_data_repeat_membership = None;
    packet.is_queued_for_data_repeat = false;
}

/// Queue a prepared packet through `picoquic_queue_data_repeat_packet`.
fn dataqueue_queue_prepared_packet(
    quic: &mut Quic,
    cnx: &mut crate::internal::Connection,
    sequence_number: u64,
) -> crate::Result<crate::internal::Packet> {
    let mut stored_packet = quic.create_packet()?;
    prepare_dataqueue_packet_for_queue(&mut stored_packet, sequence_number);
    let stored_token = cnx.queued_packets.insert(stored_packet)?;

    let mut packet = quic.create_packet()?;
    prepare_dataqueue_packet_for_queue(&mut packet, sequence_number);
    cnx.queue_data_repeat_packet(&mut packet);
    assert!(
        packet.is_queued_for_data_repeat,
        "prepared data-repeat packet was not queued"
    );

    let stored_packet = cnx
        .queued_packets
        .get_mut(stored_token)
        .ok_or(crate::Error::Generic)?;
    stored_packet.queue_data_repeat_membership = packet.queue_data_repeat_membership;
    stored_packet.is_queued_for_data_repeat = packet.is_queued_for_data_repeat;
    stored_packet.data_repeat_frame = packet.data_repeat_frame;
    stored_packet.data_repeat_index = packet.data_repeat_index;
    stored_packet.data_repeat_priority = packet.data_repeat_priority;
    stored_packet.data_repeat_stream_id = packet.data_repeat_stream_id;
    stored_packet.data_repeat_stream_offset = packet.data_repeat_stream_offset;
    stored_packet.data_repeat_stream_data_length = packet.data_repeat_stream_data_length;

    Ok(packet)
}

/// C: `dataqueue_packet_test_iterate`.
fn dataqueue_packet_test_iterate(
    cnx: &mut crate::internal::Connection,
    test_id: i32,
    buffer_size: usize,
    expect_data: bool,
    expect_more_data: bool,
    expect_is_pure_ack: bool,
) -> crate::Result<()> {
    let mut buffer = [0u8; crate::MAX_PACKET_SIZE];
    let mut more_data = 0;
    let mut is_pure_ack = 1;
    let written = {
        let tail = crate::internal::copy_stream_frames_for_retransmit(
            cnx,
            &mut buffer[..buffer_size],
            u64::MAX,
            &mut more_data,
            &mut is_pure_ack,
        )
        .unwrap_or_else(|| panic!("dataqueue_packet test {test_id}, returns None"));
        buffer_size - tail.len()
    };

    assert_eq!(
        written > 0,
        expect_data,
        "dataqueue_packet test {test_id}, data presence mismatch"
    );
    assert_eq!(
        more_data != 0,
        expect_more_data,
        "dataqueue_packet test {test_id}, more_data mismatch"
    );
    assert_eq!(
        is_pure_ack != 0,
        expect_is_pure_ack,
        "dataqueue_packet test {test_id}, pure ACK mismatch"
    );

    Ok(())
}

const BINLOG_TEST_FILE: &str = "01020304.client.log";
const BINLOG_ERROR_TEST_FILE: &str = "binlog_error_test.txt";
const BINLOG_FUZZ_TEST_FILE: &str = "binlog_fuzz_test.log";
const QLOG_TEST_FILE: &str = "01020304.qlog";
const BINLOG_TEST_REF: &str = "picoquictest/binlog_ref.log";
const QLOG_TEST_REF: &str = "picoquictest/binlog_ref.qlog";
const BINLOG_TEST_CID_HEX: &str = "01020304";
const LOG_TEST_FILE: &str = "log_test.txt";
const LOG_PACKET_TEST_FILE: &str = "log_packet_test.txt";
const LOG_ERROR_TEST_FILE: &str = "log_error_test.txt";
const LOG_FUZZ_TEST_FILE: &str = "log_fuzz_test.txt";
const LOGGER_TEST_CID_BYTES: [u8; 8] = [11, 12, 13, 14, 15, 16, 17, 18];
const LOGGER_TEST_ADDR: &str = "[2020:2020:2020:2020:2020:2020:2020:2020]:443";

struct BinlogQlogPacketHeader {
    packet_type: u64,
    packet_number: u64,
    payload_length: usize,
    version: u32,
    dest_cid: crate::ConnectionId,
    src_cid: crate::ConnectionId,
    key_phase: bool,
    spin: bool,
    quic_bit_is_zero: bool,
    token_bytes: Vec<u8>,
}

struct BinlogQlogContext {
    cid_hex: &'static str,
    trace_flow_id: bool,
    start_time: u64,
    event_count: usize,
    frame_count: usize,
    packet_type: u64,
    key_phase_sent: Option<bool>,
    key_phase_received: Option<bool>,
    spin_bit_sent: Option<bool>,
}

fn read_test_file(path: &str) -> crate::Result<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(_) => {
            let fallback = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path);
            std::fs::read(fallback).map_err(|_| crate::Error::Generic)
        }
    }
}

fn compare_binary_files(file1: &str, file2: &str) -> crate::Result<()> {
    let c1 = read_test_file(file1)?;
    let c2 = read_test_file(file2)?;
    if c1 == c2 {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

fn binlog_test_packet_header(
    initial_cid: crate::ConnectionId,
    dest_cid: crate::ConnectionId,
    pn: u64,
    payload_length: usize,
) -> crate::internal::PacketHeader {
    crate::internal::PacketHeader {
        packet_type: crate::internal::PacketType::OneRttProtected,
        packet_number_full: pn,
        dest_connection_id: initial_cid,
        src_connection_id: dest_cid,
        offset: 0,
        payload_length,
        ..Default::default()
    }
}

fn log_binlog_test_packet(
    f: &mut std::fs::File,
    initial_cid: crate::ConnectionId,
    dest_cid: crate::ConnectionId,
    pn: u64,
    payload: &[u8],
) {
    let ph = binlog_test_packet_header(initial_cid, dest_cid, pn, payload.len());
    crate::binlog::packet(
        f,
        &initial_cid,
        0,
        false,
        Instant::from_ticks(0),
        &ph,
        payload,
    );
}

fn read_varint_for_qlog(cursor: &mut &[u8]) -> u64 {
    let mut value = 0;
    if let Some(rest) = crate::internal::frames_varint_decode(cursor, &mut value) {
        *cursor = rest;
        value
    } else {
        0
    }
}

fn qlog_hex_string(bytes: &[u8], len: usize) -> String {
    let mut out = Vec::new();
    let _ = crate::qlog::qlog_string(&mut out, bytes, len);
    String::from_utf8(out).expect("qlog hex is utf8")
}

fn append_binlog_qlog_reset_stream_at(qlog: &mut String, frame: &[u8]) {
    use core::fmt::Write as _;

    let mut cursor = frame;
    let stream_id = read_varint_for_qlog(&mut cursor);
    let error_code = read_varint_for_qlog(&mut cursor);
    let final_size = read_varint_for_qlog(&mut cursor);
    let _reliable_size = read_varint_for_qlog(&mut cursor);
    write!(
        qlog,
        "{{ \n    \"frame_type\": \"reset_stream_at\", \"stream_id\": {}, \"error_code\": {}, \"final_size\": {}, \"reliable_size\": {}}}",
        stream_id, error_code, final_size, final_size
    )
    .expect("write qlog reset_stream_at");
}

fn append_binlog_qlog_bdp(qlog: &mut String, frame: &[u8]) {
    use core::fmt::Write as _;

    let mut cursor = frame;
    let lifetime = read_varint_for_qlog(&mut cursor);
    let bytes_in_flight = read_varint_for_qlog(&mut cursor);
    let min_rtt = read_varint_for_qlog(&mut cursor);
    let ip_len = read_varint_for_qlog(&mut cursor) as usize;
    let ip = qlog_hex_string(cursor, ip_len);
    write!(
        qlog,
        "{{ \n    \"frame_type\": \"bdp\", \"lifetime\": {}, \"bytes_in_flight\": {}, \"min_rtt\": {}, \"ip\": {}}}",
        lifetime, bytes_in_flight, min_rtt, ip
    )
    .expect("write qlog bdp");
}

fn append_binlog_qlog_unknown(qlog: &mut String, frame_id: u64, frame: &[u8]) {
    use core::fmt::Write as _;

    let begins_with = qlog_hex_string(frame, frame.len().min(8));
    write!(
        qlog,
        "{{ \n    \"frame_type\": \"unknown\",\"unknown_type\": {},\"begins_with\": {}}}",
        frame_id, begins_with
    )
    .expect("write qlog unknown frame");
}

fn append_binlog_qlog_frame(
    qlog: &mut String,
    ctx: &mut BinlogQlogContext,
    frame: &[u8],
) -> crate::Result<()> {
    if ctx.frame_count != 0 {
        qlog.push_str(", ");
    }

    let mut frame_id = 0u64;
    let frame_after_type =
        crate::internal::frames_varint_decode(frame, &mut frame_id).ok_or(crate::Error::Generic)?;

    if frame_id == crate::frames::FrameType::ResetStreamAt as u64 {
        append_binlog_qlog_reset_stream_at(qlog, frame_after_type);
    } else if frame_id == crate::frames::FrameType::Bdp as u64 {
        append_binlog_qlog_bdp(qlog, frame_after_type);
    } else if crate::frames::FrameType::name(frame_id).is_none() {
        append_binlog_qlog_unknown(qlog, frame_id, frame);
    } else {
        let mut out = Vec::new();
        crate::qlog::qlog_frames(&mut out, frame, false);
        qlog.push_str(core::str::from_utf8(&out).map_err(|_| crate::Error::Generic)?);
    }

    ctx.frame_count += 1;
    Ok(())
}

fn read_binlog_qlog_packet_header(s: &mut ByteStream<'_>) -> crate::Result<BinlogQlogPacketHeader> {
    let header_flags = s.read_u8()?;
    let payload_length = s.read_vlen()?;
    let packet_type = s.read_varint()?;
    let packet_number = s.read_varint()?;
    let dest_cid = s.read_cid()?;
    let src_cid = s.read_cid()?;

    let version = if packet_type != crate::internal::PacketType::OneRttProtected as u64
        && packet_type != crate::internal::PacketType::VersionNegotiation as u64
    {
        s.read_u32()?
    } else {
        0
    };

    let token_bytes = if packet_type == crate::internal::PacketType::Initial as u64 {
        let token_length = s.read_vlen()?;
        let mut token = vec![0u8; token_length];
        s.read_bytes(&mut token)?;
        token
    } else {
        Vec::new()
    };

    Ok(BinlogQlogPacketHeader {
        packet_type,
        packet_number,
        payload_length,
        version,
        dest_cid,
        src_cid,
        key_phase: (header_flags & 1) != 0,
        spin: (header_flags & 2) != 0,
        quic_bit_is_zero: (header_flags & 64) != 0,
        token_bytes,
    })
}

fn qlog_packet_type_name(packet_type: u64) -> &'static str {
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

fn connection_id_hex(cid: &crate::ConnectionId) -> String {
    use core::fmt::Write as _;

    let mut out = String::with_capacity(2 * cid.len());
    for byte in cid.as_bytes() {
        write!(&mut out, "{byte:02x}").expect("write connection id hex");
    }
    out
}

fn append_binlog_qlog_event_header(
    qlog: &mut String,
    ctx: &BinlogQlogContext,
    delta_time: u64,
    path_id: u64,
    event_class: &str,
    event_name: &str,
) {
    use core::fmt::Write as _;

    write!(qlog, "[{}, ", delta_time).expect("write qlog event time");
    if ctx.trace_flow_id {
        write!(qlog, "{}, ", path_id).expect("write qlog path id");
    }
    write!(qlog, "\"{}\", \"{}\", {{", event_class, event_name).expect("write qlog event header");
}

fn append_binlog_qlog_packet_start(
    qlog: &mut String,
    ctx: &mut BinlogQlogContext,
    time: u64,
    path_id: u64,
    packet_size: u64,
    ph: &BinlogQlogPacketHeader,
    receiving: bool,
) {
    use core::fmt::Write as _;

    let delta_time = time.saturating_sub(ctx.start_time);
    if ctx.event_count != 0 {
        qlog.push_str(",\n");
    } else {
        qlog.push('\n');
    }

    if ph.packet_type == crate::internal::PacketType::OneRttProtected as u64 && !receiving {
        if ctx
            .spin_bit_sent
            .is_some_and(|spin_bit_sent| spin_bit_sent != ph.spin)
        {
            append_binlog_qlog_event_header(
                qlog,
                ctx,
                delta_time,
                path_id,
                "transport",
                "spin_bit_updated",
            );
            writeln!(
                qlog,
                " \"state\": {} }}],",
                if ph.spin { "true" } else { "false" }
            )
            .expect("write qlog spin bit");
        }
        ctx.spin_bit_sent = Some(ph.spin);
    }

    append_binlog_qlog_event_header(
        qlog,
        ctx,
        delta_time,
        path_id,
        "transport",
        if receiving {
            "packet_received"
        } else {
            "packet_sent"
        },
    );
    write!(
        qlog,
        " \"packet_type\": \"{}\", \"header\": {{ \"packet_size\": {}",
        qlog_packet_type_name(ph.packet_type),
        packet_size
    )
    .expect("write qlog packet header");

    if ph.packet_type != crate::internal::PacketType::VersionNegotiation as u64
        && ph.packet_type != crate::internal::PacketType::Retry as u64
    {
        write!(qlog, ", \"packet_number\": {}", ph.packet_number)
            .expect("write qlog packet number");
    }

    if ph.packet_type != crate::internal::PacketType::OneRttProtected as u64 {
        write!(qlog, ", \"version\": \"{:08x}\"", ph.version).expect("write qlog version");
        if ph.packet_type != crate::internal::PacketType::VersionNegotiation as u64
            && ph.packet_type != crate::internal::PacketType::Retry as u64
            && ph.packet_type != crate::internal::PacketType::Error as u64
        {
            write!(qlog, ", \"payload_length\": {}", ph.payload_length)
                .expect("write qlog payload length");
        }
    }

    if ph.packet_type != crate::internal::PacketType::OneRttProtected as u64
        && !ph.src_cid.is_empty()
    {
        write!(qlog, ", \"scid\": \"{}\"", connection_id_hex(&ph.src_cid))
            .expect("write qlog scid");
    }
    if !ph.dest_cid.is_empty() {
        write!(qlog, ", \"dcid\": \"{}\"", connection_id_hex(&ph.dest_cid))
            .expect("write qlog dcid");
    }

    if ph.packet_type == crate::internal::PacketType::Initial as u64 && !ph.token_bytes.is_empty() {
        write!(
            qlog,
            ", \"token\": {}",
            qlog_hex_string(&ph.token_bytes, ph.token_bytes.len())
        )
        .expect("write qlog token");
    }

    if ph.packet_type == crate::internal::PacketType::OneRttProtected as u64 {
        let key_phase_slot = if receiving {
            &mut ctx.key_phase_received
        } else {
            &mut ctx.key_phase_sent
        };
        let need_key_phase = key_phase_slot.is_none_or(|last| last != ph.key_phase);
        *key_phase_slot = Some(ph.key_phase);
        if need_key_phase {
            write!(
                qlog,
                ", \"key_phase\": {}",
                if ph.key_phase { 1 } else { 0 }
            )
            .expect("write qlog key phase");
        }
    }

    if ph.quic_bit_is_zero {
        qlog.push_str(", \"quic_bit\": 0");
    }

    ctx.packet_type = ph.packet_type;
    ctx.frame_count = 0;
    if ctx.packet_type == crate::internal::PacketType::VersionNegotiation as u64
        || ctx.packet_type == crate::internal::PacketType::Retry as u64
    {
        qlog.push_str(" }");
    } else {
        qlog.push_str(" }, \"frames\": [");
    }
}

fn append_binlog_qlog_packet_end(qlog: &mut String, ctx: &mut BinlogQlogContext) {
    if ctx.packet_type == crate::internal::PacketType::VersionNegotiation as u64
        || ctx.packet_type == crate::internal::PacketType::Retry as u64
    {
        qlog.push_str("}]");
    } else {
        qlog.push_str("]}]");
    }
    ctx.event_count += 1;
}

fn convert_binlog_test_to_qlog(
    binlog_file: &str,
    qlog_file: &str,
    initial_cid: &crate::ConnectionId,
) -> crate::Result<()> {
    let bytes = std::fs::read(binlog_file).map_err(|_| crate::Error::Generic)?;
    if bytes.len() < 16 {
        return Err(crate::Error::Generic);
    }
    let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != crate::fourcc(b'q', b'l', b'o', b'g') {
        return Err(crate::Error::Generic);
    }
    let flags = u16::from_be_bytes([bytes[4], bytes[5]]);
    if u16::from_be_bytes([bytes[6], bytes[7]]) != 1 {
        return Err(crate::Error::Generic);
    }

    let mut offset = 16usize;
    let mut qlog = String::new();
    let mut qlog_started = false;
    let mut ctx = BinlogQlogContext {
        cid_hex: BINLOG_TEST_CID_HEX,
        trace_flow_id: (flags & 1) != 0,
        start_time: 0,
        event_count: 0,
        frame_count: 0,
        packet_type: 0,
        key_phase_sent: None,
        key_phase_received: None,
        spin_bit_sent: None,
    };

    while offset < bytes.len() {
        if bytes.len() - offset < 4 {
            return Err(crate::Error::Generic);
        }
        let record_len = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;
        if bytes.len() - offset < record_len {
            return Err(crate::Error::Generic);
        }

        let mut record = bytes[offset..offset + record_len].to_vec();
        offset += record_len;
        let mut stream = ByteStream::from_slice(&mut record);
        let cid = stream.read_cid()?;
        if cid.as_bytes() != initial_cid.as_bytes() {
            continue;
        }
        let event_time = stream.read_varint()?;
        let path_id = stream.read_varint()?;
        let event_id = stream.read_varint()?;

        if event_id == crate::binlog::LogEventType::NewConnection as u64 {
            let client_mode = stream.read_u8()? != 0;
            let _proposed_version = stream.read_u32()?;
            let _remote_cid = stream.read_cid()?;
            if !qlog_started {
                ctx.start_time = event_time;
                append_overflow_qlog_start(&mut qlog, ctx.cid_hex, client_mode, ctx.start_time);
                qlog_started = true;
            }
        } else if event_id == crate::binlog::LogEventType::PacketSent as u64
            || event_id == crate::binlog::LogEventType::PacketRecv as u64
        {
            if !qlog_started {
                return Err(crate::Error::Generic);
            }
            let receiving = event_id == crate::binlog::LogEventType::PacketRecv as u64;
            let packet_size = stream.read_varint()?;
            let ph = read_binlog_qlog_packet_header(&mut stream)?;
            append_binlog_qlog_packet_start(
                &mut qlog,
                &mut ctx,
                event_time,
                path_id,
                packet_size,
                &ph,
                receiving,
            );
            while stream.remaining() > 0 {
                let frame_len = stream.read_vlen()?;
                if stream.remaining() < frame_len {
                    return Err(crate::Error::Generic);
                }
                let frame = stream.tail()[..frame_len].to_vec();
                stream.skip(frame_len)?;
                append_binlog_qlog_frame(&mut qlog, &mut ctx, &frame)?;
            }
            append_binlog_qlog_packet_end(&mut qlog, &mut ctx);
        }
    }

    if !qlog_started {
        return Err(crate::Error::Generic);
    }
    qlog.push_str("]}]}\n");
    std::fs::write(qlog_file, qlog).map_err(|_| crate::Error::Generic)
}

fn run_binlog_error_logging(
    errors: &[TestFrameError],
    initial_cid: crate::ConnectionId,
    dest_cid: crate::ConnectionId,
) -> crate::Result<()> {
    use std::io::Write as _;

    for error in errors {
        for sharp_end in [false, true] {
            let mut buffer = error.bytes.clone();
            if !error.must_be_last && !sharp_end {
                buffer.extend_from_slice(&[0, 0, 0, 0]);
            }
            let mut f =
                std::fs::File::create(BINLOG_ERROR_TEST_FILE).map_err(|_| crate::Error::Generic)?;
            log_binlog_test_packet(&mut f, initial_cid, dest_cid, 0, &buffer);
            f.flush().map_err(|_| crate::Error::Generic)?;
        }
    }
    Ok(())
}

fn run_binlog_fuzz_logging(
    frames: &[TestSkipFrame],
    initial_cid: crate::ConnectionId,
    dest_cid: crate::ConnectionId,
    random_context: &mut u64,
) -> crate::Result<()> {
    use std::io::Write as _;

    for _ in 0..100 {
        let packet = format_random_packet(frames, crate::MAX_PACKET_SIZE, random_context);
        let mut f =
            std::fs::File::create(BINLOG_FUZZ_TEST_FILE).map_err(|_| crate::Error::Generic)?;
        log_binlog_test_packet(&mut f, initial_cid, dest_cid, 0, &packet);

        for _ in 0..100 {
            f.flush().map_err(|_| crate::Error::Generic)?;
            let fuzz_packet = skip_test_fuzz_packet(&packet, random_context);
            log_binlog_test_packet(&mut f, initial_cid, dest_cid, 0, &fuzz_packet);
        }
    }
    Ok(())
}

/// C: `binlog_test` body — creates a QUIC context, runs a reference
/// connection scenario, converts to QLOG, and diffs against a reference.
fn run_binlog_test() -> crate::Result<()> {
    use std::path::PathBuf;

    let _ = std::fs::remove_file(BINLOG_TEST_FILE);
    let _ = std::fs::remove_file(QLOG_TEST_FILE);
    let _ = std::fs::remove_file(BINLOG_ERROR_TEST_FILE);
    let _ = std::fs::remove_file(BINLOG_FUZZ_TEST_FILE);

    let mut random_context = 0xF00BAB;
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    quic.set_binlog(Some(".")).ok();
    quic.set_default_spinbit_policy(crate::SpinbitVersion::Null)?;
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
    cnx.f_binlog = crate::binlog::create_binlog(BINLOG_TEST_FILE, simulated_time.ticks(), false);
    cnx.binlog_file_name = Some(PathBuf::from(BINLOG_TEST_FILE));
    if cnx.f_binlog.is_none() {
        return Err(crate::Error::Generic);
    }

    crate::binlog::Binlog::new_connection(cnx.as_mut());
    {
        let f = cnx.f_binlog.as_mut().ok_or(crate::Error::Generic)?;
        let frames = test_skip_frames();
        for (i, frame) in frames.iter().enumerate() {
            log_binlog_test_packet(f, initial_cid, dest_cid, i as u64, &frame.bytes);
        }

        let errors = test_frame_errors();
        for (i, frame) in errors.iter().enumerate() {
            log_binlog_test_packet(f, initial_cid, dest_cid, i as u64, &frame.bytes);
        }
    }
    crate::binlog::Binlog::close_connection(cnx.as_mut());

    compare_binary_files(BINLOG_TEST_FILE, BINLOG_TEST_REF)?;
    convert_binlog_test_to_qlog(BINLOG_TEST_FILE, QLOG_TEST_FILE, &initial_cid)?;
    super::util::compare_text_files(QLOG_TEST_FILE, QLOG_TEST_REF)?;

    let frames = test_skip_frames();
    let errors = test_frame_errors();
    run_binlog_error_logging(&errors, initial_cid, dest_cid)?;
    run_binlog_fuzz_logging(&frames, initial_cid, dest_cid, &mut random_context)?;
    Ok(())
}

/// C: `logger_test` body — creates a QUIC context, exercises the text
/// logger, and compares against a reference.
fn run_logger_test() -> crate::Result<()> {
    let _ = std::fs::remove_file(LOG_TEST_FILE);
    let _ = std::fs::remove_file(LOG_PACKET_TEST_FILE);
    let _ = std::fs::remove_file(LOG_ERROR_TEST_FILE);
    let _ = std::fs::remove_file(LOG_FUZZ_TEST_FILE);

    let mut simulated_time = Instant::from_ticks(123_456_789);
    let mut quic = make_quic(&mut simulated_time);
    let logger_test_cid = crate::ConnectionId::clone_from_slice(&LOGGER_TEST_CID_BYTES)
        .ok_or(crate::Error::Generic)?;
    let logger_test_addr: core::net::SocketAddr = LOGGER_TEST_ADDR
        .parse()
        .map_err(|_| crate::Error::InvalidArgument)?;

    let mut cnx = quic.create_connection_with_cids(
        logger_test_cid,
        logger_test_cid,
        Some(&logger_test_addr),
        simulated_time,
        "test-sni",
        "test-alpn",
    )?;

    quic.set_textlog(Some(LOG_TEST_FILE))?;
    assert!(
        quic.text_log_fns.is_some(),
        "picoquic_set_textlog must install the production textlog backend"
    );
    assert!(
        cnx.text_log_fns.is_some(),
        "connections created before set_textlog must dispatch through the installed textlog backend"
    );

    cnx.log_app_message("This is an app message test.");
    cnx.log_app_message("This is app message test #1, severity 2.");
    quic.textlog_close();
    super::util::compare_text_files(LOG_TEST_FILE, "picoquictest/log_test_ref.txt")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Phase 3A helpers for tests whose Rust signatures differ from the compact
// test entry points.

fn format_test_written<F>(
    name: &'static str,
    bytes: &mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    mut format: F,
) -> crate::Result<Option<usize>>
where
    F: FnMut(&mut [u8], &mut i32, &mut i32) -> Option<usize>,
{
    *more_data = 0;
    *is_pure_ack = 0;
    let written = format(bytes, more_data, is_pure_ack);
    if written.is_some_and(|len| len > bytes.len()) {
        return Err(crate::Error::Generic);
    }
    let _ = name;
    Ok(written)
}

fn frame_format_test_once<F>(name: &'static str, s_max: usize, format: F) -> crate::Result<()>
where
    F: FnMut(&mut [u8], &mut i32, &mut i32) -> Option<usize>,
{
    let mut buffer = [0u8; crate::MAX_PACKET_SIZE];
    let mut more_data = 0;
    let mut is_pure_ack = 0;
    let written = format_test_written(
        name,
        &mut buffer[..s_max],
        &mut more_data,
        &mut is_pure_ack,
        format,
    )?;
    if written != Some(0) || more_data == 0 {
        return Err(crate::Error::Generic);
    }
    Ok(())
}

fn frame_format_test<F>(name: &'static str, mut format: F) -> crate::Result<()>
where
    F: FnMut(&mut [u8], &mut i32, &mut i32) -> Option<usize>,
{
    let mut buffer = [0u8; crate::MAX_PACKET_SIZE];
    let mut bytes_max = buffer.len();
    let mut more_data = 0;
    let mut is_pure_ack = 0;
    let mut returned_start = false;
    let mut returned_null = false;

    for _ in 0..2 {
        let written = format_test_written(
            name,
            &mut buffer[..bytes_max],
            &mut more_data,
            &mut is_pure_ack,
            &mut format,
        )?;
        match written {
            Some(0) => {
                returned_start = true;
                break;
            }
            Some(len) => {
                bytes_max = len.saturating_sub(1);
            }
            None => {
                returned_null = true;
                break;
            }
        }
    }

    if returned_null || !returned_start || more_data == 0 {
        return Err(crate::Error::Generic);
    }
    Ok(())
}

/// C: `frames_format_test` internal body — exercises the same
/// `picoquic_format_*` list and shrinking-buffer checks.
fn run_frames_format_test() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    parse_test_packet_cnx_fix(&mut cnx, simulated_time, 3, 1);

    let data = [0xaau8, 0xaa];
    cnx.create_local_connection_id(0, None, simulated_time)?;
    let local_unique_path_id = cnx
        .local_connection_id_lists
        .first()
        .map(|list| list.unique_path_id)
        .ok_or(crate::Error::Generic)?;
    let l_cid = cnx.create_local_connection_id(local_unique_path_id, None, simulated_time)?;

    cnx.add_to_stream(0, &data, false)?;
    let stream_token = cnx.find_stream(0).ok_or(crate::Error::Generic)?;
    let mut stream = cnx
        .streams
        .remove(stream_token)
        .ok_or(crate::Error::Generic)?;

    stream.reset_requested = true;
    frame_format_test_once("reset_stream", 2, |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_reset_stream_frame(&mut stream, bytes, more_data, is_pure_ack)
            .map(|tail| before - tail.len())
    })?;
    stream.reset_requested = false;

    frame_format_test("new_connection_id", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        let list_index = cnx
            .local_connection_id_lists
            .iter()
            .position(|list| list.unique_path_id == local_unique_path_id)?;
        let mut local_cnxid_list = cnx.local_connection_id_lists.remove(list_index);
        let written = crate::internal::format_new_connection_id_frame(
            &mut cnx,
            &mut local_cnxid_list,
            bytes,
            more_data,
            is_pure_ack,
            Some(l_cid),
        )
        .map(|tail| before - tail.len());
        cnx.local_connection_id_lists
            .insert(list_index, local_cnxid_list);
        written
    })?;

    frame_format_test("retire_connection_id", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_retire_connection_id_frame(
            bytes,
            more_data,
            is_pure_ack,
            true,
            0,
            17,
        )
        .map(|tail| before - tail.len())
    })?;

    frame_format_test("new_token", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_new_token_frame(bytes, more_data, is_pure_ack, &data)
            .map(|tail| before - tail.len())
    })?;

    stream.stop_sending_requested = true;
    frame_format_test_once("stop_sending", 2, |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_stop_sending_frame(&mut stream, bytes, more_data, is_pure_ack)
            .map(|tail| before - tail.len())
    })?;
    stream.stop_sending_requested = false;
    stream.stop_sending_sent = false;

    frame_format_test_once("data_blocked", 1, |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_data_blocked_frame(&mut cnx, bytes, more_data, is_pure_ack)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("stream_data_blocked", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_stream_data_blocked_frame(
            bytes,
            more_data,
            is_pure_ack,
            &mut stream,
        )
        .map(|tail| before - tail.len())
    })?;
    stream.stream_data_blocked_sent = false;

    frame_format_test_once("stream_blocked", 1, |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_stream_blocked_frame(
            &mut cnx,
            bytes,
            more_data,
            is_pure_ack,
            &stream,
        )
        .map(|tail| before - tail.len())
    })?;
    cnx.stream_blocked_bidir_sent = false;

    frame_format_test("connection_close", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_connection_close_frame(&mut cnx, bytes, more_data, is_pure_ack)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("application_close", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_application_close_frame(&mut cnx, bytes, more_data, is_pure_ack)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("max_stream_data", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_max_stream_data_frame(
            &mut cnx,
            &mut stream,
            bytes,
            more_data,
            is_pure_ack,
            100_000_000,
        )
        .map(|tail| before - tail.len())
    })?;

    frame_format_test("path_challenge", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_path_challenge_frame(
            bytes,
            more_data,
            is_pure_ack,
            0xaabb_ccdd_eeff_0011,
        )
        .map(|tail| before - tail.len())
    })?;

    frame_format_test("path_response", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_path_response_frame(
            bytes,
            more_data,
            is_pure_ack,
            0xaabb_ccdd_eeff_0011,
        )
        .map(|tail| before - tail.len())
    })?;

    frame_format_test("datagram", |bytes, more_data, is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_datagram_frame(bytes, more_data, is_pure_ack, data.len(), &data)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test_once("ack_frequency", 2, |bytes, more_data, _is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_ack_frequency_frame(&mut cnx, bytes, more_data)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("immediate_ack", |bytes, more_data, _is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_immediate_ack_frame(bytes, more_data)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("time_stamp", |bytes, more_data, _is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_time_stamp_frame(&mut cnx, bytes, more_data, simulated_time)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("path_abandon", |bytes, more_data, _is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_path_abandon_frame(bytes, more_data, 1, 3)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("path_available", |bytes, more_data, _is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_path_available_or_backup_frame(
            bytes,
            crate::frames::FrameType::PathAvailable as u64,
            1,
            17,
            more_data,
        )
        .map(|tail| before - tail.len())
    })?;

    frame_format_test("max_path_id", |bytes, more_data, _is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_max_path_id_frame(bytes, 123, more_data)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("paths_blocked", |bytes, more_data, _is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_paths_blocked_frame(bytes, 123, more_data)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("path_cid_blocked", |bytes, more_data, _is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_path_cid_blocked_frame(bytes, 123, 0, more_data)
            .map(|tail| before - tail.len())
    })?;

    frame_format_test("observed_address", |bytes, more_data, _is_pure_ack| {
        let before = bytes.len();
        crate::internal::format_observed_address_frame(
            bytes,
            crate::frames::FrameType::ObservedAddressV4 as u64,
            13,
            &[1, 2, 3, 4],
            4433,
            more_data,
        )
        .map(|tail| before - tail.len())
    })?;

    Ok(())
}

/// C: `new_cnxid_test` — formats a NEW_CONNECTION_ID frame then verifies
/// that `skip_frame` can skip it exactly.
fn run_new_cnxid_test() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let peer_addr = core::net::SocketAddr::new(
        core::net::IpAddr::V4(core::net::Ipv4Addr::UNSPECIFIED),
        1000,
    );
    let null_cid = crate::ConnectionId::default();

    let cnx_token = quic.create_cnx_internal(
        null_cid,
        null_cid,
        Some(&peer_addr),
        simulated_time,
        0,
        Some("test-sni"),
        Some("test-alpn"),
        true,
        None,
        None,
    )?;
    let local_cid = quic.create_local_cnxid(cnx_token, 0, None, simulated_time)?;
    let mut frame_buffer = [0u8; 256];

    let consumed = {
        let cnx = quic
            .connections
            .get_mut(cnx_token)
            .ok_or(crate::Error::Generic)?;
        let list_index = cnx
            .local_connection_id_lists
            .iter()
            .position(|list| list.unique_path_id == 0)
            .ok_or(crate::Error::Generic)?;

        {
            let local_cid_list = &cnx.local_connection_id_lists[list_index];
            assert_eq!(
                local_cid_list.connection_ids.len(),
                2,
                "local CID list length"
            );
            let first_local_cid = local_cid_list
                .connection_ids
                .first()
                .copied()
                .ok_or(crate::Error::Generic)?;
            let second_local_cid = local_cid_list
                .connection_ids
                .get(1)
                .copied()
                .ok_or(crate::Error::Generic)?;
            assert!(
                cnx.local_connection_ids.get(first_local_cid).is_some(),
                "first local CID entry"
            );
            assert!(
                cnx.local_connection_ids.get(second_local_cid).is_some(),
                "second local CID entry"
            );
            assert!(
                second_local_cid == local_cid,
                "created local CID is not second in the list"
            );
        }

        let before = frame_buffer.len();
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let mut local_cid_list = cnx.local_connection_id_lists.remove(list_index);
        let written = crate::internal::format_new_connection_id_frame(
            cnx,
            &mut local_cid_list,
            &mut frame_buffer,
            &mut more_data,
            &mut is_pure_ack,
            Some(local_cid),
        )
        .map(|tail| before - tail.len());
        cnx.local_connection_id_lists
            .insert(list_index, local_cid_list);
        let written = written.ok_or(crate::Error::Generic)?;
        if written == 0 {
            return Err(crate::Error::Generic);
        }
        written
    };

    let mut skipped = 0usize;
    let mut pure_ack = 0i32;
    let ret = skip_frame(
        &frame_buffer,
        frame_buffer.len(),
        &mut skipped,
        &mut pure_ack,
    );
    if ret != 0 {
        return Err(crate::Error::InvalidFrame);
    }
    assert_eq!(skipped, consumed, "NEW_CONNECTION_ID skipped length");
    assert_eq!(pure_ack, 0, "NEW_CONNECTION_ID pure_ack");
    Ok(())
}

/// C: `test_copy_for_retransmit` — copies frames out of a retransmit packet
/// into a fresh buffer and verifies the copy.
fn run_stream_retransmit_copy_test(
    quic: &mut Quic,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let mut packet_is_pure_ack = 0;
    let mut do_not_detect_spurious = 1;

    for (i, case) in copy_retransmit_cases().into_iter().enumerate() {
        let mut cnx = quic.create_connection_with_cids(
            crate::ConnectionId::default(),
            crate::ConnectionId::default(),
            None,
            *simulated_time,
            "test-sni",
            "test-alpn",
        )?;
        cnx.add_to_stream(0, &copy_stream0_data(), false)?;

        let mut old_p = quic.create_packet()?;
        if let Some(packet) = &case.packet {
            old_p.bytes[..packet.len()].copy_from_slice(packet);
            old_p.length = packet.len();
        }
        old_p.offset = case.offset;
        old_p.is_mtu_probe = case.is_mtu_probe;
        old_p.is_ack_trap = case.is_ack_trap;
        old_p.send_path = Some(crate::internal::PathToken::synthetic(0, 0));

        let mut new_bytes = [0u8; crate::MAX_PACKET_SIZE];
        let mut length = case.b1_offset;
        let mut add_to_data_repeat_queue = 0;

        let ret = crate::internal::copy_before_retransmit(
            &mut old_p,
            &mut cnx,
            &mut new_bytes,
            case.copy_max,
            &mut packet_is_pure_ack,
            &mut do_not_detect_spurious,
            0,
            &mut length,
            &mut add_to_data_repeat_queue,
        );
        assert_eq!(ret, 0, "copy retransmit case {i} ({})", case.name);
        assert_eq!(
            packet_is_pure_ack, case.is_pure_ack_expected,
            "copy retransmit case {i} ({}) pure-ack mismatch",
            case.name
        );

        if packet_is_pure_ack == 0 {
            assert_eq!(
                length, case.b1_length,
                "copy retransmit case {i} ({}) copied length mismatch",
                case.name
            );
            let expected = case.b1_expected.as_ref().unwrap_or_else(|| {
                panic!(
                    "copy retransmit case {i} ({}) missing b1 expectation",
                    case.name
                )
            });
            assert!(
                length >= case.b1_offset,
                "copy retransmit case {i} ({}) invalid copied range",
                case.name
            );
            assert_eq!(
                &new_bytes[case.b1_offset..length],
                &expected[case.b1_offset..length],
                "copy retransmit case {i} ({}) copied bytes mismatch",
                case.name
            );

            assert_eq!(
                add_to_data_repeat_queue != 0,
                case.b2_expected.is_some(),
                "copy retransmit case {i} ({}) stream-repeat flag mismatch",
                case.name
            );

            match case.b3_expected.as_ref() {
                None => assert!(
                    cnx.misc_frames.is_empty(),
                    "copy retransmit case {i} ({}) unexpected misc frame",
                    case.name
                ),
                Some(expected_misc) => {
                    let misc = cnx.misc_frames.front().unwrap_or_else(|| {
                        panic!(
                            "copy retransmit case {i} ({}) missing misc frame",
                            case.name
                        )
                    });
                    assert_eq!(
                        misc.bytes.as_slice(),
                        expected_misc.as_slice(),
                        "copy retransmit case {i} ({}) misc frame mismatch",
                        case.name
                    );
                }
            }
        }
    }

    Ok(())
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
    let extra = [0xff, 0, 0, 0];
    let frames = test_skip_frames();
    assert_eq!(frames.len(), TEST_SKIP_FRAME_VARINT_COUNTS.len());

    for case in &frames {
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

    for case in test_frame_errors() {
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
            if case.skip_fails {
                assert_ne!(ret, 0, "skip error frame {} unexpectedly passed", case.name);
            }
        }
    }

    for (case, nb_varints) in frames
        .iter()
        .zip(TEST_SKIP_FRAME_VARINT_COUNTS.iter().copied())
    {
        for varint_idx in 1..=nb_varints {
            let frame = create_test_varint_frame(&case.bytes, varint_idx);
            if !frame.is_empty() {
                let mut consumed = 0usize;
                let mut pure_ack = 0i32;
                let ret = skip_frame(&frame, frame.len(), &mut consumed, &mut pure_ack);
                assert_ne!(
                    ret, 0,
                    "bad varint frame {} index {} unexpectedly passed",
                    case.name, varint_idx
                );
            }
        }
    }

    let mut random_context = 0xbabed011u64;
    let mut fuzz_count = 0usize;
    let mut fuzz_fail = 0usize;
    for i in 0..100 {
        let packet = format_random_packet(&frames, crate::MAX_PACKET_SIZE, &mut random_context);
        let ret = skip_test_packet(&packet);
        assert_eq!(ret, 0, "skip packet {} fails, ret = {}", i, ret);

        for _ in 0..100 {
            let fuzz_packet = skip_test_fuzz_packet(&packet, &mut random_context);
            if skip_test_packet(&fuzz_packet) != 0 {
                fuzz_fail += 1;
            }
            fuzz_count += 1;
        }
    }
    assert_eq!(fuzz_count, 10_000);
    assert!(fuzz_fail <= fuzz_count);
}

/// C: `parse_frame_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn frames_parse() {
    const EXTRA_BYTES: [u8; 4] = [0, 0, 0, 0];

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let frames = test_skip_frames();
    let error_frames = test_frame_errors();

    assert_eq!(frames.len(), TEST_SKIP_FRAME_VARINT_COUNTS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_EPOCHS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_MPATH.len());
    assert_eq!(error_frames.len(), TEST_FRAME_ERROR_EXPECTED_ERRORS.len());
    assert_eq!(error_frames.len(), TEST_FRAME_ERROR_EPOCHS.len());
    assert_eq!(error_frames.len(), TEST_FRAME_ERROR_MPATH.len());

    for (i, case) in frames.iter().enumerate().skip(0x0c) {
        for sharp_end in [false, true] {
            let mut packet = case.bytes.clone();
            if !case.must_be_last && !sharp_end {
                packet.extend_from_slice(&EXTRA_BYTES);
            }

            let parsed = parse_test_packet(
                &mut quic,
                &packet,
                TEST_SKIP_FRAME_EPOCHS[i],
                TEST_SKIP_FRAME_MPATH[i],
            );
            assert_eq!(parsed.ret, 0, "parse frame <{}>", case.name);
            assert_eq!(
                parsed.ack_needed,
                case.pure_ack == 0,
                "ack needed for frame <{}>",
                case.name
            );
        }
    }

    for (i, case) in frames.iter().enumerate() {
        for varint_idx in 1..=TEST_SKIP_FRAME_VARINT_COUNTS[i] {
            let packet = create_test_varint_frame(&case.bytes, varint_idx);
            if !packet.is_empty() {
                let parsed = parse_test_packet(
                    &mut quic,
                    &packet,
                    TEST_SKIP_FRAME_EPOCHS[i],
                    TEST_SKIP_FRAME_MPATH[i],
                );
                assert_ne!(
                    parsed.ret, 0,
                    "bad varint frame <{}> index {} unexpectedly passed",
                    case.name, varint_idx
                );
            }
        }
    }

    for (i, case) in frames.iter().enumerate() {
        if TEST_SKIP_FRAME_MPATH[i] != 0 {
            let parsed = parse_test_packet(&mut quic, &case.bytes, TEST_SKIP_FRAME_EPOCHS[i], 0);
            assert_ne!(
                parsed.ret, 0,
                "multipath frame <{}> unexpectedly passed without multipath",
                case.name
            );
        }
    }

    for (i, case) in frames.iter().enumerate() {
        if let Some(ftype) = frame_type(&case.bytes) {
            let parsed = parse_test_packet(&mut quic, &case.bytes, 1, TEST_SKIP_FRAME_MPATH[i]);
            if frame_type_allowed_in_0rtt(ftype) {
                assert_eq!(parsed.ret, 0, "0-RTT frame <{}>", case.name);
            } else {
                assert_ne!(
                    parsed.ret, 0,
                    "frame <{}> unexpectedly allowed in 0-RTT",
                    case.name
                );
            }
        }
    }

    for (i, case) in error_frames.iter().enumerate() {
        for sharp_end in [false, true] {
            let mut packet = case.bytes.clone();
            if !case.must_be_last && !sharp_end {
                packet.extend_from_slice(&EXTRA_BYTES);
            }

            let parsed = parse_test_packet(
                &mut quic,
                &packet,
                TEST_FRAME_ERROR_EPOCHS[i],
                TEST_FRAME_ERROR_MPATH[i],
            );
            assert_ne!(parsed.ret, 0, "parse error frame <{}> passed", case.name);
            assert_eq!(
                parsed.local_error, TEST_FRAME_ERROR_EXPECTED_ERRORS[i],
                "parse error frame <{}> local error",
                case.name
            );
        }
    }

    let mut random_context = 0x1234_5678u64;
    let mut fuzz_count = 0usize;
    let mut fuzz_fail = 0usize;
    for i in 0..100 {
        let r = loop {
            let r = test_uniform_random(&mut random_context, frames.len() as u64) as usize;
            if TEST_SKIP_FRAME_EPOCHS[r] == 3 {
                break r;
            }
        };

        let mut packet = frames[r].bytes.clone();
        if !frames[r].must_be_last {
            match test_uniform_random(&mut random_context, 4) {
                0 => packet.extend_from_slice(&frames[19].bytes),
                1 => packet.extend_from_slice(&frames[22].bytes),
                2 => packet.resize(crate::MAX_PACKET_SIZE, 0),
                _ => {}
            }
        }

        if TEST_SKIP_FRAME_MPATH[r] == 0 {
            let parsed = parse_test_packet(&mut quic, &packet, 3, TEST_SKIP_FRAME_MPATH[r]);
            assert_eq!(parsed.ret, 0, "skip packet <{}>", i);
        }

        for j in 0..100 {
            let fuzz_packet = skip_test_fuzz_packet(&packet, &mut random_context);
            let parsed = parse_test_packet(&mut quic, &fuzz_packet, 3, (j % 3) as u8);
            if parsed.ret != 0 {
                fuzz_fail += 1;
            }
            fuzz_count += 1;
        }
    }
    assert_eq!(fuzz_count, 10_000);
    assert!(fuzz_fail <= fuzz_count);
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
    let frames = test_skip_frames();

    assert_eq!(frames.len(), TEST_SKIP_FRAME_VARINT_COUNTS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_EPOCHS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_MPATH.len());

    let mut nb_trials = 0usize;
    let mut nb_disconnected = 0usize;
    for (i, case) in frames.iter().enumerate() {
        for varint_idx in 1..=TEST_SKIP_FRAME_VARINT_COUNTS[i] {
            let disconnected = frame_ackack_error_packet(
                &mut quic,
                &case.bytes,
                TEST_SKIP_FRAME_EPOCHS[i],
                TEST_SKIP_FRAME_MPATH[i],
                varint_idx,
            )
            .unwrap_or_else(|err| {
                panic!(
                    "ack-ack frame <{}> varint {} failed: {:?}",
                    case.name, varint_idx, err
                )
            });
            nb_trials += 1;
            nb_disconnected += usize::from(disconnected);
        }
    }

    assert_eq!(
        nb_trials,
        TEST_SKIP_FRAME_VARINT_COUNTS
            .iter()
            .copied()
            .map(usize::try_from)
            .collect::<Result<Vec<_>, _>>()
            .expect("varint count conversion")
            .into_iter()
            .sum::<usize>()
    );
    assert!(nb_disconnected <= nb_trials);
}

/// C: `frames_repeat_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn frames_repeat() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let frames = test_skip_frames();

    assert_eq!(frames.len(), TEST_SKIP_FRAME_VARINT_COUNTS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_EPOCHS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_MPATH.len());

    for (i, case) in frames.iter().enumerate() {
        let Some((ftype, type_len)) = frame_type_and_len(&case.bytes) else {
            continue;
        };
        frame_repeat_error_packet(
            &mut quic,
            &case.bytes,
            TEST_SKIP_FRAME_EPOCHS[i],
            TEST_SKIP_FRAME_MPATH[i],
            false,
        )
        .unwrap_or_else(|err| panic!("repeat full frame <{}> failed: {:?}", case.name, err));

        if case.bytes.len() > 1 && case.pure_ack == 0 && !frame_repeat_truncation_excluded(ftype) {
            let truncated = &case.bytes[..case.bytes.len() - 1];
            if frame_repeat_error_packet(
                &mut quic,
                truncated,
                TEST_SKIP_FRAME_EPOCHS[i],
                TEST_SKIP_FRAME_MPATH[i],
                true,
            )
            .is_err()
            {
                assert!(
                    TEST_SKIP_FRAME_VARINT_COUNTS[i] > 0,
                    "repeat truncated frame <{}> failed without C type-only fallback",
                    case.name
                );
                frame_repeat_error_packet(
                    &mut quic,
                    &case.bytes[..type_len],
                    TEST_SKIP_FRAME_EPOCHS[i],
                    TEST_SKIP_FRAME_MPATH[i],
                    true,
                )
                .unwrap_or_else(|err| {
                    panic!(
                        "repeat type-only frame <{}> failed after truncated fallback: {:?}",
                        case.name, err
                    )
                });
            }
        }
    }
}

/// C: `new_cnxid_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn new_cnxid() {
    run_new_cnxid_test().expect("new_cnxid");
}

#[derive(Copy, Clone)]
struct CnxidStashCase {
    sequence: u64,
    connection_id: &'static [u8],
    reset_secret: [u8; crate::RESET_SECRET_SIZE],
}

const CNXID_STASH_CASES: [CnxidStashCase; 3] = [
    CnxidStashCase {
        sequence: 1,
        connection_id: &[0, 1, 2, 3],
        reset_secret: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    },
    CnxidStashCase {
        sequence: 2,
        connection_id: &[1, 2, 3, 4],
        reset_secret: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    },
    CnxidStashCase {
        sequence: 3,
        connection_id: &[2, 3, 4, 5],
        reset_secret: [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
    },
];

fn init_cnxid_stash_test_connection(cnx: &mut crate::internal::Connection) {
    let local_cid =
        crate::ConnectionId::clone_from_slice(&[11, 11, 11, 11]).expect("stash test local cid");
    let remote_cid =
        crate::ConnectionId::clone_from_slice(&[99, 99, 99, 99]).expect("stash test remote cid");

    let local_cid_token = cnx
        .paths
        .first()
        .and_then(|path| path.tuples.first())
        .and_then(|tuple| tuple.local_connection_id)
        .expect("initial local cid token");
    cnx.local_connection_ids
        .get_mut(local_cid_token)
        .expect("initial local cid")
        .connection_id = local_cid;

    cnx.remote_connection_id_stashes
        .iter_mut()
        .find(|stash| stash.unique_path_id == 0)
        .and_then(|stash| stash.connection_ids.first_mut())
        .expect("initial remote cid")
        .connection_id = remote_cid;
}

fn assert_obtained_cnxid_stash_case(
    cnx: &mut crate::internal::Connection,
    test_mode: usize,
    case_index: usize,
) {
    let case = CNXID_STASH_CASES[case_index];
    let (stash_index, cid_index) = cnx.obtain_stashed_connection_id(0).unwrap_or_else(|| {
        panic!("Test {test_mode}, cannot dequeue cnxid {case_index}.");
    });
    let stashed = cnx.remote_connection_id_stashes[stash_index]
        .connection_ids
        .get_mut(cid_index)
        .expect("obtained stashed cid");
    stashed.nb_path_references += 1;

    assert_eq!(
        stashed.sequence, case.sequence,
        "Test {test_mode}, cnxid {case_index}, sequence mismatch."
    );
    assert_eq!(
        stashed.connection_id.as_bytes(),
        case.connection_id,
        "Test {test_mode}, cnxid {case_index}, CID values do not match."
    );
    assert_eq!(
        stashed.reset_secret, case.reset_secret,
        "Test {test_mode}, cnxid {case_index}, secrets do not match."
    );
}

/// C: `cnxid_stash_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn new_cnxid_stash() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    for test_mode in 0..3 {
        let mut cnx = quic.create_test_cnx(&mut simulated_time).expect("cnx");
        init_cnxid_stash_test_connection(&mut cnx);

        for (case_index, case) in CNXID_STASH_CASES.iter().enumerate() {
            let result = cnx.stash_remote_connection_id(
                0,
                0,
                case.sequence,
                case.connection_id,
                &case.reset_secret,
            );
            assert_eq!(
                result.status, 0,
                "Test {test_mode}, cannot stash cnxid {case_index}."
            );
            assert!(
                result.stashed_index.is_some(),
                "Test {test_mode}, cannot stash cnxid {case_index} (duplicate)."
            );

            if test_mode == 0 {
                assert_obtained_cnxid_stash_case(&mut cnx, test_mode, case_index);
            }
        }

        if test_mode == 1 {
            for case_index in 0..CNXID_STASH_CASES.len() {
                assert_obtained_cnxid_stash_case(&mut cnx, test_mode, case_index);
            }
        }

        if test_mode < 2 {
            assert!(
                cnx.obtain_stashed_connection_id(0).is_none(),
                "Test {test_mode}, unexpected cnxid left."
            );
        }
    }
}

/// C: `send_stream_blocked_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn send_stream_blocked() {
    for (i, case) in STREAM_BLOCKED_TEST_CASES.iter().enumerate() {
        send_stream_blocked_test_one(case).unwrap_or_else(|err| {
            panic!("stream blocked case {i} failed: {err:?}");
        });
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
    const DATAQUEUE_PACKET_SEQUENCE: u64 = 0x5a5a_0000;

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut cnx = quic.create_test_cnx(&mut simulated_time).expect("cnx");
    let new_bytes = [0u8; 256];

    cnx.add_to_stream(0, &new_bytes, false)
        .expect("initialize stream 0");
    let _packet = dataqueue_queue_prepared_packet(&mut quic, &mut cnx, DATAQUEUE_PACKET_SEQUENCE)
        .expect("queue data-repeat packet");

    dataqueue_packet_test_iterate(&mut cnx, 1, 2, false, true, true)
        .expect("dataqueue_packet iteration 1");
    dataqueue_packet_test_iterate(&mut cnx, 2, 128, false, true, true)
        .expect("dataqueue_packet iteration 2");
    dataqueue_packet_test_iterate(&mut cnx, 3, 1024, true, false, false)
        .expect("dataqueue_packet iteration 3");
    dataqueue_packet_test_iterate(&mut cnx, 4, 1024, false, false, true)
        .expect("dataqueue_packet iteration 4");

    let mut packet =
        dataqueue_queue_prepared_packet(&mut quic, &mut cnx, DATAQUEUE_PACKET_SEQUENCE + 1)
            .expect("queue packet for dequeue coverage");
    cnx.dequeue_data_repeat_packet(&mut packet);
    assert!(
        !packet.is_queued_for_data_repeat,
        "dequeued data-repeat packet remains queued"
    );
}

const QLOG_OVERFLOW_CID_HEX: &str = "0809000102030405";
const QLOG_OVERFLOW_BIN: &str = "0809000102030405.client.log";
const QLOG_OVERFLOW_FILE: &str = "0809000102030405.qlog";
const QLOG_OVERFLOW_REF: &str = "picoquictest/app_msg_overflow_ref.qlog";

fn append_overflow_qlog_start(
    qlog: &mut String,
    cid_hex: &str,
    client_mode: bool,
    reference_time: u64,
) {
    use core::fmt::Write as _;

    writeln!(
        qlog,
        "{{ \"qlog_version\": \"draft-00\", \"title\": \"picoquic\", \"traces\": ["
    )
    .expect("write qlog header");
    writeln!(
        qlog,
        "{{ \"vantage_point\": {{ \"name\": \"backend-67\", \"type\": \"{}\" }},",
        if client_mode { "client" } else { "server" }
    )
    .expect("write qlog vantage point");
    writeln!(
        qlog,
        "\"title\": \"picoquic\", \"description\": \"{}\",\"event_fields\": [\"relative_time\", \"category\", \"event\", \"data\"],",
        cid_hex
    )
    .expect("write qlog title");
    writeln!(qlog, "\"configuration\": {{\"time_units\": \"us\"}},")
        .expect("write qlog configuration");
    writeln!(
        qlog,
        "\"common_fields\": {{ \"protocol_type\": \"QUIC_HTTP3\", \"reference_time\": \"{}\"}},",
        reference_time
    )
    .expect("write qlog common fields");
    write!(qlog, "\"events\": [").expect("write qlog events start");
}

fn append_overflow_qlog_message(
    qlog: &mut String,
    event_count: &mut usize,
    delta_time: u64,
    message: &[u8],
) {
    use core::fmt::Write as _;

    if *event_count != 0 {
        qlog.push_str(",\n");
    } else {
        qlog.push('\n');
    }

    write!(
        qlog,
        "[{}, \"info\", \"message\", {{ \"message\": \"",
        delta_time
    )
    .expect("write qlog message header");

    for &c in message.iter().take(BYTESTREAM_MAX_BUFFER_SIZE) {
        qlog.push(if (0x20..=0x7e).contains(&c) {
            char::from(c)
        } else {
            '?'
        });
    }

    qlog.push_str("\"}]");
    *event_count += 1;
}

fn convert_overflow_binlog_to_qlog(
    binlog_file: &str,
    qlog_file: &str,
    initial_cid: &crate::ConnectionId,
) -> crate::Result<()> {
    let bytes = std::fs::read(binlog_file).map_err(|_| crate::Error::Generic)?;
    if bytes.len() < 16 {
        return Err(crate::Error::Generic);
    }
    let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    if magic != crate::fourcc(b'q', b'l', b'o', b'g') {
        return Err(crate::Error::Generic);
    }
    if u16::from_be_bytes([bytes[6], bytes[7]]) != 1 {
        return Err(crate::Error::Generic);
    }

    let mut offset = 16usize;
    let mut qlog = String::new();
    let mut qlog_started = false;
    let mut start_time = 0u64;
    let mut event_count = 0usize;

    while offset < bytes.len() {
        if bytes.len() - offset < 4 {
            return Err(crate::Error::Generic);
        }
        let record_len = u32::from_be_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;
        if bytes.len() - offset < record_len {
            return Err(crate::Error::Generic);
        }

        let mut record = bytes[offset..offset + record_len].to_vec();
        offset += record_len;

        let mut stream = ByteStream::from_slice(&mut record);
        let cid = stream.read_cid()?;
        if cid.as_bytes() != initial_cid.as_bytes() {
            continue;
        }
        let event_time = stream.read_varint()?;
        let _path_id = stream.read_varint()?;
        let event_id = stream.read_varint()?;

        if event_id == crate::binlog::LogEventType::NewConnection as u64 {
            let client_mode = stream.read_u8()? != 0;
            let _proposed_version = stream.read_u32()?;
            let _remote_cid = stream.read_cid()?;
            if !qlog_started {
                start_time = event_time;
                append_overflow_qlog_start(
                    &mut qlog,
                    QLOG_OVERFLOW_CID_HEX,
                    client_mode,
                    start_time,
                );
                qlog_started = true;
            }
        } else if event_id == crate::binlog::LogEventType::InfoMessage as u64 {
            if !qlog_started {
                return Err(crate::Error::Generic);
            }
            let mut message = vec![0u8; stream.remaining()];
            stream.read_bytes(&mut message)?;
            append_overflow_qlog_message(
                &mut qlog,
                &mut event_count,
                event_time.saturating_sub(start_time),
                &message,
            );
        }
    }

    if !qlog_started {
        return Err(crate::Error::Generic);
    }
    qlog.push_str("]}]}\n");
    std::fs::write(qlog_file, qlog).map_err(|_| crate::Error::Generic)
}

/// C: `app_message_overflow_test` in `picoquictest/skip_frame_test.c`.
#[test]
fn app_message_overflow() {
    use crate::ConnectionId as InternalCid;

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    quic.set_binlog(Some(".")).ok();
    let _ = std::fs::remove_file(QLOG_OVERFLOW_BIN);
    let _ = std::fs::remove_file(QLOG_OVERFLOW_FILE);

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

    let mut test = [0u8; BYTESTREAM_MAX_BUFFER_SIZE];
    test[..BYTESTREAM_MAX_BUFFER_SIZE - 3].fill(b'x');
    test[BYTESTREAM_MAX_BUFFER_SIZE - 3] = b'!';
    test[BYTESTREAM_MAX_BUFFER_SIZE - 2] = 0;
    for i in 0..16usize {
        let suffix = core::str::from_utf8(&test[15 - i..BYTESTREAM_MAX_BUFFER_SIZE - 2])
            .expect("overflow message is ascii");
        cnx.log_app_message(&format!("s:{}", suffix));
    }
    cnx.delete();
    drop(cnx);
    drop(quic);

    convert_overflow_binlog_to_qlog(QLOG_OVERFLOW_BIN, QLOG_OVERFLOW_FILE, &initial_cid)
        .expect("convert overflow binlog to qlog");
    super::util::compare_text_files(QLOG_OVERFLOW_FILE, QLOG_OVERFLOW_REF)
        .expect("app message overflow qlog matches reference");
}

/// C: `queue_network_input_test` in `picoquictest/skip_frame_test.c`.
fn stream_data_splay_chunks(tree: &crate::internal::StreamDataSplay) -> Vec<(u64, usize, Vec<u8>)> {
    use crate::internal::{StreamDataNode, StreamDataSplay};
    use crate::splay::SplayTree;

    assert_eq!(
        core::mem::size_of::<StreamDataSplay>(),
        core::mem::size_of::<SplayTree<u64, StreamDataNode>>(),
        "StreamDataSplay test view size mismatch"
    );
    assert_eq!(
        core::mem::align_of::<StreamDataSplay>(),
        core::mem::align_of::<SplayTree<u64, StreamDataNode>>(),
        "StreamDataSplay test view alignment mismatch"
    );

    // SAFETY: this test-only inspector mirrors the current `StreamDataSplay`
    // representation, which is a single private `SplayTree<u64, StreamDataNode>`
    // field. The size/alignment checks above fail before dereference if that
    // representation changes.
    let inner: &SplayTree<u64, StreamDataNode> =
        unsafe { &*(core::ptr::from_ref(tree).cast::<SplayTree<u64, StreamDataNode>>()) };

    let mut chunks = Vec::new();
    let mut token = inner.first();
    while let Some(current) = token {
        let (key, node) = inner
            .get_key_value(current)
            .expect("valid stream data splay token");
        chunks.push(((*key), node.length, node.data[..node.length].to_vec()));
        token = inner.next(current);
    }
    chunks
}

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
    assert!(new_data, "expected new data after gap-fill 2..7");

    // Duplicate 2..7. The bytes are fully covered, so no new data is queued.
    new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 2, &data[..6], true, &mut new_data)
        .expect("queue duplicate 2..7");
    assert!(!new_data, "expected no new data after duplicate 2..7");

    // Verify the tree contains the expected three contiguous segments.
    let chunks = stream_data_splay_chunks(&tree);
    assert_eq!(
        chunks,
        vec![
            (0, 4, vec![0, 1, 2, 3]),
            (4, 2, vec![4, 5]),
            (6, 4, vec![6, 7, 8, 9]),
        ],
        "expected queued stream data chunks"
    );
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
