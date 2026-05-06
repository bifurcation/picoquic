//! Test cases for `picoquictest/qlog_frame_test.c`.

#![allow(non_snake_case)]

use crate::frames::FrameType;
use crate::qlog::qlog_frames as qlog_frames_write;
use crate::tests::util::compare_text_files as compare_text_files_impl;

/// One entry from the C `test_skip_list[]` array in `skip_frame_test.c`.
/// C: `test_skip_frames_t`.
struct SkipFrameEntry {
    name: &'static str,
    val: &'static [u8],
}

/// The frame test vectors from `picoquictest/skip_frame_test.c`.
/// C: `test_skip_list[]` / `nb_test_skip_list`.
fn test_skip_list() -> Vec<SkipFrameEntry> {
    vec![
        SkipFrameEntry {
            name: "padding",
            val: &[0, 0, 0],
        },
        SkipFrameEntry {
            name: "reset_stream",
            val: &[FrameType::ResetStream as u8, 17, 1, 1],
        },
        SkipFrameEntry {
            name: "connection_close",
            val: &[
                FrameType::ConnectionClose as u8,
                0x80,
                0x00,
                0xcf,
                0xff,
                0,
                9,
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
        },
        SkipFrameEntry {
            name: "application_close",
            val: &[FrameType::ApplicationClose as u8, 0, 0],
        },
        SkipFrameEntry {
            name: "application_close",
            val: &[
                FrameType::ApplicationClose as u8,
                0x44,
                4,
                4,
                b't',
                b'e',
                b's',
                b't',
            ],
        },
        SkipFrameEntry {
            name: "max_data",
            val: &[FrameType::MaxData as u8, 0xc0, 0, 0x01, 0, 0, 0, 0, 0],
        },
        SkipFrameEntry {
            name: "max_stream_data",
            val: &[FrameType::MaxStreamData as u8, 1, 0x80, 0x01, 0, 0],
        },
        SkipFrameEntry {
            name: "max_streams_bidir",
            val: &[FrameType::MaxStreamsBidir as u8, 0x41, 0],
        },
        SkipFrameEntry {
            name: "max_streams_unidir",
            val: &[FrameType::MaxStreamsUnidir as u8, 0x41, 7],
        },
        SkipFrameEntry {
            name: "ping",
            val: &[FrameType::Ping as u8],
        },
        SkipFrameEntry {
            name: "blocked",
            val: &[FrameType::DataBlocked as u8, 0x80, 0x01, 0, 0],
        },
        SkipFrameEntry {
            name: "stream_data_blocked",
            val: &[
                FrameType::StreamDataBlocked as u8,
                0x80,
                1,
                0,
                0,
                0x80,
                0x02,
                0,
                0,
            ],
        },
        SkipFrameEntry {
            name: "streams_blocked_bidir",
            val: &[FrameType::StreamsBlockedBidir as u8, 0x41, 0x00],
        },
        SkipFrameEntry {
            name: "streams_blocked_unidir",
            val: &[FrameType::StreamsBlockedUnidir as u8, 0x42, 0x00],
        },
        SkipFrameEntry {
            name: "new_connection_id",
            val: &[
                FrameType::NewConnectionId as u8,
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
            ],
        },
        SkipFrameEntry {
            name: "stop_sending",
            val: &[FrameType::StopSending as u8, 17, 0x17],
        },
        SkipFrameEntry {
            name: "challenge",
            val: &[FrameType::PathChallenge as u8, 1, 2, 3, 4, 5, 6, 7, 8],
        },
        SkipFrameEntry {
            name: "response",
            val: &[FrameType::PathResponse as u8, 1, 2, 3, 4, 5, 6, 7, 8],
        },
        SkipFrameEntry {
            name: "new_token",
            val: &[
                FrameType::NewToken as u8,
                17,
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
        },
        SkipFrameEntry {
            name: "ack",
            val: &[
                FrameType::Ack as u8,
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
                0,
                5,
                12,
            ],
        },
        SkipFrameEntry {
            name: "ack_ecn",
            val: &[
                FrameType::AckEcn as u8,
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
                0,
                5,
                12,
                3,
                0,
                1,
            ],
        },
        SkipFrameEntry {
            name: "stream_min",
            val: &[
                FrameType::StreamRangeMin as u8,
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
        },
        SkipFrameEntry {
            name: "stream_max",
            val: &[
                FrameType::StreamRangeMin as u8 + 2 + 4,
                1,
                0x44,
                0,
                0x10,
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
        },
        SkipFrameEntry {
            name: "crypto_hs",
            val: &[
                FrameType::CryptoHs as u8,
                0,
                0x10,
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
        },
        SkipFrameEntry {
            name: "retire_connection_id",
            val: &[FrameType::RetireConnectionId as u8, 1],
        },
        SkipFrameEntry {
            name: "datagram",
            val: &[
                FrameType::Datagram as u8,
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
        },
        SkipFrameEntry {
            name: "datagram_l",
            val: &[
                FrameType::DatagramL as u8,
                0x10,
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
        },
        SkipFrameEntry {
            name: "handshake_done",
            val: &[FrameType::HandshakeDone as u8],
        },
        SkipFrameEntry {
            name: "ack_frequency",
            val: &[
                0x40,
                FrameType::AckFrequency as u8,
                17,
                0x0a,
                0x44,
                0x20,
                0x00,
            ],
        },
        SkipFrameEntry {
            name: "ack_frequency_t5",
            val: &[
                0x40,
                FrameType::AckFrequency as u8,
                17,
                0x0a,
                0x44,
                0x20,
                0x40,
                0x05,
            ],
        },
        SkipFrameEntry {
            name: "immediate_ack",
            val: &[FrameType::ImmediateAck as u8],
        },
        SkipFrameEntry {
            name: "time_stamp",
            val: &[
                0x40u8 | ((FrameType::TimeStamp as u64 >> 8) as u8),
                (FrameType::TimeStamp as u64 & 0xff) as u8,
                0x44,
                0,
            ],
        },
        SkipFrameEntry {
            name: "path_abandon_0",
            val: &[
                0x40u8 | ((FrameType::PathAbandon as u64 >> 8) as u8),
                (FrameType::PathAbandon as u64 & 0xff) as u8,
                0x01,
                0x00,
            ],
        },
        SkipFrameEntry {
            name: "path_abandon_1",
            val: &[
                0x40u8 | ((FrameType::PathAbandon as u64 >> 8) as u8),
                (FrameType::PathAbandon as u64 & 0xff) as u8,
                0x01,
                0x11,
            ],
        },
        SkipFrameEntry {
            name: "path_backup",
            val: &[
                0x40u8 | ((FrameType::PathBackup as u64 >> 8) as u8),
                (FrameType::PathBackup as u64 & 0xff) as u8,
                0x00,
                0x0f,
            ],
        },
        SkipFrameEntry {
            name: "path_available",
            val: &[
                0x40u8 | ((FrameType::PathAvailable as u64 >> 8) as u8),
                (FrameType::PathAvailable as u64 & 0xff) as u8,
                0x00,
                0x0f,
            ],
        },
        SkipFrameEntry {
            name: "max paths",
            val: &[
                0x40u8 | ((FrameType::MaxPathId as u64 >> 8) as u8),
                (FrameType::MaxPathId as u64 & 0xff) as u8,
                0x11,
            ],
        },
        SkipFrameEntry {
            name: "path_new_connection_id",
            val: &[
                0x40u8 | ((FrameType::PathNewConnectionId as u64 >> 8) as u8),
                (FrameType::PathNewConnectionId as u64 & 0xff) as u8,
                1,
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
                0xb0,
            ],
        },
        SkipFrameEntry {
            name: "path_retire_connection_id",
            val: &[
                0x40u8 | ((FrameType::PathRetireConnectionId as u64 >> 8) as u8),
                (FrameType::PathRetireConnectionId as u64 & 0xff) as u8,
                0,
                2,
            ],
        },
        SkipFrameEntry {
            name: "paths blocked",
            val: &[
                0x40u8 | ((FrameType::PathsBlocked as u64 >> 8) as u8),
                (FrameType::PathsBlocked as u64 & 0xff) as u8,
                0x11,
            ],
        },
        SkipFrameEntry {
            name: "path cid blocked",
            val: &[
                0x40u8 | ((FrameType::PathCidBlocked as u64 >> 8) as u8),
                (FrameType::PathCidBlocked as u64 & 0xff) as u8,
                0x07,
                0x01,
            ],
        },
        SkipFrameEntry {
            name: "bdp",
            val: &[
                0x80u8 | ((FrameType::Bdp as u64 >> 24) as u8),
                (FrameType::Bdp as u64 >> 16) as u8,
                (FrameType::Bdp as u64 >> 8) as u8,
                (FrameType::Bdp as u64 & 0xff) as u8,
                0x01,
                0x02,
                0x03,
                0x04,
                0x0a,
                0x0,
                0x0,
                0x01,
            ],
        },
        SkipFrameEntry {
            name: "observed_address_v4",
            val: &[
                0x80u8 | ((FrameType::ObservedAddressV4 as u64 >> 24) as u8),
                (FrameType::ObservedAddressV4 as u64 >> 16) as u8,
                (FrameType::ObservedAddressV4 as u64 >> 8) as u8,
                (FrameType::ObservedAddressV4 as u64 & 0xff) as u8,
                1,
                0x1,
                0x2,
                0x3,
                0x4,
                0x12,
                0x34,
            ],
        },
        SkipFrameEntry {
            name: "observed_address_v6",
            val: &[
                0x80u8 | ((FrameType::ObservedAddressV6 as u64 >> 24) as u8),
                (FrameType::ObservedAddressV6 as u64 >> 16) as u8,
                (FrameType::ObservedAddressV6 as u64 >> 8) as u8,
                (FrameType::ObservedAddressV6 as u64 & 0xff) as u8,
                2,
                0x1,
                0x2,
                0x3,
                0x4,
                0x5,
                0x6,
                0x7,
                0x8,
                0x9,
                0xa,
                0xb,
                0xc,
                0xd,
                0xe,
                0xf,
                0x0,
                0x45,
                0x67,
            ],
        },
        SkipFrameEntry {
            name: "path_ack",
            val: &[
                FrameType::PathAck as u8,
                0,
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
                0,
                5,
                12,
            ],
        },
        SkipFrameEntry {
            name: "path_ack_ecn",
            val: &[
                FrameType::PathAckEcn as u8,
                0,
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
                0,
                5,
                12,
                3,
                0,
                1,
            ],
        },
        SkipFrameEntry {
            name: "reset_stream_at",
            val: &[FrameType::ResetStreamAt as u8, 17, 1, 0x40, 128, 13],
        },
    ]
}

/// Compare two text files byte-for-byte.
/// C: `picoquic_test_compare_text_files`.
fn compare_text_files(a: &str, b: &str) -> crate::Result<()> {
    compare_text_files_impl(a, b)
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

/// C: `qlog_frames_test` in `picoquictest/qlog_frame_test.c`.
#[test]
fn qlog_frames() {
    let test_ref = get_input_path("picoquictest/qlog_frames_test_ref.txt")
        .expect("resolve qlog frames ref path");

    let mut out: Vec<u8> = Vec::new();
    let list = test_skip_list();

    let mut need_comma = "";
    out.extend_from_slice(b"[\n");
    for entry in &list {
        out.extend_from_slice(need_comma.as_bytes());
        out.extend_from_slice(format!("{{ \"test\": \"{}\", \"frame\": ", entry.name).as_bytes());
        need_comma = ",\n";

        qlog_frames_write(&mut out, entry.val, false);
        out.extend_from_slice(b"}");
    }
    out.extend_from_slice(b"\n]\n");

    let output_file = "qlog_frames_test.json";
    std::fs::write(output_file, &out).expect("write qlog frames test output");

    compare_text_files(&test_ref, output_file).expect("qlog frames output matches reference");
}
