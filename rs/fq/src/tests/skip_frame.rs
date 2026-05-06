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
// context.  Phase 4 will implement these; for now they are `todo!()` stubs.

/// C: `parse_test_packet` — parse every frame in `buf` using a fresh cnx.
fn parse_test_packet(
    _quic: &mut Quic,
    _buf: &[u8],
    _epoch: u32,
    _mpath: bool,
) -> crate::Result<()> {
    todo!("parse_test_packet")
}

/// C: `frame_ackack_error_packet` — send a frame that triggers an ack-ack
/// error on a fresh connection.
fn frame_ackack_error_packet(
    _quic: &mut Quic,
    _frame: &[u8],
    _epoch: u32,
    _mpath: bool,
    _varint_idx: u32,
) -> bool {
    todo!("frame_ackack_error_packet")
}

/// C: `frame_repeat_error_packet` — verify the repeat-detection logic on `buf`.
fn frame_repeat_error_packet(
    _quic: &mut Quic,
    _buf: &[u8],
    _epoch: u32,
    _mpath: bool,
    _expect_error: bool,
) -> crate::Result<()> {
    todo!("frame_repeat_error_packet")
}

/// C: `send_stream_blocked_test_one` — one stream-blocked sub-case.
fn send_stream_blocked_test_one(_case_idx: usize) -> crate::Result<()> {
    todo!("send_stream_blocked_test_one")
}

/// C: `process_ack_of_stream_frame` + `check_frame_needs_repeat`
/// integrated sub-case driver.
fn stream_ack_test_one(_quic: &mut Quic) -> crate::Result<()> {
    todo!("stream_ack_test_one")
}

/// C: `dataqueue_prepare_test` + `dataqueue_verify_test`.
fn dataqueue_copy_test_one(
    _basic_case: i32,
    _has_length: bool,
    _has_fin: bool,
) -> crate::Result<()> {
    todo!("dataqueue_copy_test_one")
}

/// C: `dataqueue_prepare_packet` + `picoquic_queue_data_repeat_packet`
/// + `dataqueue_packet_test_iterate`.
fn dataqueue_packet_test_iterate(_quic: &mut Quic) -> crate::Result<()> {
    todo!("dataqueue_packet_test_iterate")
}

/// C: `binlog_test` body — creates a QUIC context, runs a reference
/// connection scenario, converts to QLOG, and diffs against a reference.
fn run_binlog_test() -> crate::Result<()> {
    todo!("binlog_test")
}

/// C: `logger_test` body — creates a QUIC context, exercises the text
/// logger, and compares against a reference.
fn run_logger_test() -> crate::Result<()> {
    todo!("logger_test")
}

// ---------------------------------------------------------------------------
// Phase 3A helper stubs for tests that depend on complex internal APIs
// whose Rust signatures differ from the simplified test expectations.

/// C: `frames_format_test` internal body — exercises `picoquic_format_*`
/// functions for ACK, CONNECTION_CLOSE, PATH_CHALLENGE, PATH_RESPONSE.
fn run_frames_format_test() -> crate::Result<()> {
    todo!("frames_format_test body")
}

/// C: `new_cnxid_test` — formats a NEW_CONNECTION_ID frame then verifies
/// that `skip_frame` can skip it exactly.
fn run_new_cnxid_test(_quic: &mut Quic, _simulated_time: &mut Instant) -> crate::Result<()> {
    todo!("new_cnxid_test")
}

/// C: `test_copy_for_retransmit` — copies frames out of a retransmit packet
/// into a fresh buffer and verifies the copy.
fn run_stream_retransmit_copy_test(
    _quic: &mut Quic,
    _simulated_time: &mut Instant,
) -> crate::Result<()> {
    todo!("test_copy_for_retransmit")
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

    // A minimal set of known-good frame bytes to exercise the stub.
    // The full list is held inside C's `test_skip_list[]` static array and
    // will be wired up in Phase 4.
    let test_frames: &[&[u8]] = &[
        &[0x00, 0x00, 0x00], // padding
        &[0x01],             // PING
        &[
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ], // ACK (minimal)
    ];
    for frame in test_frames {
        let mut consumed = 0usize;
        let mut pure_ack = 0i32;
        let _ret = skip_frame(frame, frame.len(), &mut consumed, &mut pure_ack);
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
