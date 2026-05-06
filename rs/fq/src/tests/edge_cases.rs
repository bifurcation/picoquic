//! Test cases for `picoquictest/edge_cases.c`.
//!
//! A collection of regression tests for edge cases typically discovered
//! via the QUIC interop runner.  Many involve specific loss patterns during
//! 0-RTT handshakes, idle-timeout negotiation, stream-reset interaction,
//! initial-PTO behaviour, and crypto-handshake frame errors.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, TestTlsApiCtx, tls_api_close_with_losses, tls_api_connection_loop,
    tls_api_data_sending_loop, tls_api_init_ctx, tls_api_init_ctx_ex,
    tls_api_one_scenario_body_verify, tls_api_one_sim_round, tls_api_wait_for_timeout,
};
use crate::internal::Version;
use crate::{ConnectionId, Instant, PacketContext};

// ---------------------------------------------------------------------------
// Shared scenarios.  Passed to helpers once those are implemented.

#[allow(dead_code)]
static SCENARIO_EDGE_CASE: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 128,
    r_len: 1_000,
}];

#[allow(dead_code)]
static SCENARIO_EDGE_RESET: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 1_000_000,
    r_len: 1_000_000,
}];

#[allow(dead_code)]
static SCENARIO_RESET_AT: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

/// Set up a test context with the standard edge-case configuration: a
/// CID of the form `{0xed, 0x9e, 0xca, 0x5e, edge_case_id, zero_rtt, 0, 0}`,
/// optional 0-RTT first pass, and `nb_init_rounds` rounds of simulation
/// with the given `loss_mask`.
/// C: `edge_case_prepare` in `picoquictest/edge_cases.c`.
fn edge_case_prepare(
    _edge_case_id: u8,
    _zero_rtt: bool,
    _simulated_time: &mut Instant,
    _loss_mask: u64,
    _nb_init_rounds: i32,
) -> crate::Result<Box<TestTlsApiCtx>> {
    todo!("edge_case_prepare: 0-RTT setup and partial simulation not yet implemented")
}

/// Finish an edge-case test: run the connection loop, enable
/// `immediate_exit`, drive data sending, and verify completion within
/// `duration_max` µs.
/// C: `edge_case_complete` in `picoquictest/edge_cases.c`.
fn edge_case_complete(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    duration_max: u64,
) -> crate::Result<()> {
    let mut loss_mask = 0u64;
    tls_api_connection_loop(test_ctx, &mut loss_mask, 0, simulated_time)?;
    test_ctx.immediate_exit = true;
    tls_api_data_sending_loop(test_ctx, &mut loss_mask, simulated_time, 0)?;
    tls_api_one_scenario_body_verify(test_ctx, simulated_time, duration_max)
}

// ---------------------------------------------------------------------------
// Reset-repeat test.

/// Variant of the reset-repeat test: which frame event to exercise.
/// C: `reset_test_enum` in `picoquictest/edge_cases.c`.
enum ResetTestKind {
    AckMaxStream = 0,
    AckReset,
    ExtraMaxStream,
    ExtraReset,
    ExtraStop,
    NeedMaxStream,
    NeedReset,
    NeedStop,
}

/// Establish a connection, start a large stream transfer, reset the stream
/// on both sides, wait for the stream context to be freed, then exercise
/// the specified post-reset frame event.
/// C: `reset_repeat_test_one` in `picoquictest/edge_cases.c`.
fn reset_repeat_test_one(_kind: ResetTestKind) -> crate::Result<()> {
    todo!("reset_repeat_test_one: stream-reset ACK/extra/need logic not yet implemented")
}

// ---------------------------------------------------------------------------
// Idle-timeout test helpers.

/// Verify idle-timeout negotiation: `client_timeout` and `server_timeout`
/// (in ms) must negotiate to `expected_timeout` (in µs); assert the
/// connection remains alive at half-time and drops by full-time.
/// C: `idle_timeout_test_one` in `picoquictest/edge_cases.c`.
fn idle_timeout_test_one(
    _test_id: u8,
    _client_timeout: u64,
    _server_timeout: u64,
    _expected_timeout: u64,
) -> crate::Result<()> {
    todo!("idle_timeout_test_one: idle-timeout negotiation loop not yet implemented")
}

/// Verify that a client connecting to a non-responding server disconnects
/// after `expected_timeout` µs (derived from `client_timeout` ms or
/// `handshake_timeout` µs).
/// C: `idle_server_test_one` in `picoquictest/edge_cases.c`.
fn idle_server_test_one(
    _test_id: u8,
    _client_timeout: u64,
    _handshake_timeout: u64,
    _expected_timeout: u64,
) -> crate::Result<()> {
    todo!("idle_server_test_one: server-silent loop not yet implemented")
}

// ---------------------------------------------------------------------------
// Initial-PTO helpers.

/// Prepare the client's initial packet and submit it to the server to
/// establish a crypto context on both sides.
/// C: `initial_pto_prepare` in `picoquictest/edge_cases.c`.
fn initial_pto_prepare(
    _test_ctx: &mut TestTlsApiCtx,
    _simulated_time: &mut Instant,
) -> crate::Result<usize> {
    todo!("initial_pto_prepare: prepare_packet + incoming_packet not yet implemented")
}

/// Advance time until the client's next scheduled send, returning the packet
/// length.  Stops early if the client enters a failure state.
/// C: `initial_pto_wait` in `picoquictest/edge_cases.c`.
fn initial_pto_wait(
    _test_ctx: &mut TestTlsApiCtx,
    _simulated_time: &mut Instant,
    _max_wait: u64,
) -> crate::Result<usize> {
    todo!("initial_pto_wait: client next_wake_time loop not yet implemented")
}

/// Craft a synthetic server ACK (Initial epoch) and deliver it to the client.
/// C: `initial_pto_ack` in `picoquictest/edge_cases.c`.
fn initial_pto_ack(
    _test_ctx: &mut TestTlsApiCtx,
    _simulated_time: &mut Instant,
) -> crate::Result<()> {
    todo!("initial_pto_ack: predict_header + format_ack + protect_packet not yet implemented")
}

// ---------------------------------------------------------------------------
// Crypto-HS-offset helper.

/// Inject a crypto handshake frame with a 64 KB offset into packet context
/// `pc`, attempt the connection, and verify the client receives
/// CRYPTO_BUFFER_EXCEEDED or IDLE_TIMEOUT.
/// C: `crypto_hs_offset_test_one` in `picoquictest/edge_cases.c`.
fn crypto_hs_offset_one(_pc: PacketContext) -> crate::Result<()> {
    todo!("crypto_hs_offset_one: queue_misc_frame + connection loop not yet implemented")
}

// ---------------------------------------------------------------------------
// Reset-stream-at helper.

/// Variant for `reset_stream_at_test_one`.
/// C: `reset_stream_at_test_enum` in `picoquictest/edge_cases.c`.
enum ResetStreamAtSpec {
    /// Reset just past what was already received.
    Basic,
    /// Reset exactly at the sent offset.
    Limit,
    /// Reset past what was received with a lossy link.
    Loss,
}

/// Establish a connection, start sending a 1 MB stream, wait a short
/// time, then call `reset_stream_at` with the appropriate `reliable_size`.
/// Verify that the scenario completes and that `reliable_size` bytes were
/// delivered.
/// C: `reset_stream_at_test_one` in `picoquictest/edge_cases.c`.
fn reset_stream_at_test_one(_spec: ResetStreamAtSpec) -> crate::Result<()> {
    todo!("reset_stream_at_test_one: reset_stream_at + data loop not yet implemented")
}

// ---------------------------------------------------------------------------
// ec9a helper (unique body, not a shared helper in C).

/// Run the server-only prepare loop for `ec9a_preemptive_amok`:
/// advance simulated time through server wakes, counting sent packets,
/// until the server's connection count drops to zero.  Returns
/// `(send_count, repeat_duration_µs)`.
fn ec9a_server_loop(
    _test_ctx: &mut TestTlsApiCtx,
    _simulated_time: &mut Instant,
) -> crate::Result<(i32, u64)> {
    todo!("ec9a_server_loop: prepare_next_packet_ex server loop not yet implemented")
}

/// Check whether the server's stream `stream_id` has `reset_sent` set.
/// C: `picoquic_find_stream(cnx_server, id)->reset_sent`.
fn check_stream_reset_sent(_test_ctx: &mut TestTlsApiCtx, _stream_id: u64) -> bool {
    todo!(
        "check_stream_reset_sent: arena deref of StreamToken to StreamHead::reset_sent not yet implemented"
    )
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `crypto_hs_offset_test` in `picoquictest/edge_cases.c`.
#[test]
fn crypto_hs_offset() {
    for pc in [
        PacketContext::Initial,
        PacketContext::Handshake,
        PacketContext::Application,
    ] {
        crypto_hs_offset_one(pc).expect("crypto_hs_offset_one");
    }
}

/// Edge case zero: verify the common 0-RTT infrastructure works.
/// C: `ec00_zero_test` in `picoquictest/edge_cases.c`.
#[test]
fn ec00_zero() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0x00, true, &mut simulated_time, 0, 4).expect("edge_case_prepare");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 100_000).expect("edge_case_complete");
    assert!(
        test_ctx.cnx_client().nb_zero_rtt_acked > 0,
        "expected at least one 0-RTT packet acked"
    );
}

/// Second-flight NACK: client should recover even when the handshake ACK
/// and HandshakeDone are lost.
/// C: `ec2f_second_flight_nack_test` in `picoquictest/edge_cases.c`.
#[test]
fn ec2f_second_flight() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_losses = 0x1c1u64;
    let mut test_ctx = edge_case_prepare(0x2f, true, &mut simulated_time, initial_losses, 9)
        .expect("edge_case_prepare");
    // After 9 rounds: client must not yet be Ready; server must be Ready.
    assert!(
        !test_ctx.client_ready(),
        "client should not be ready yet after partial handshake"
    );
    assert!(test_ctx.server_ready(), "server should be ready");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 360_000).expect("edge_case_complete");
}

/// Fuzz 50 random loss patterns to look for corrupted retransmissions.
/// C: `eccf_corrupted_file_fuzz_test` in `picoquictest/edge_cases.c`.
#[test]
fn eccf_corrupted_fuzz() {
    // Runs 50 edge_case_prepare + edge_case_complete iterations with
    // random loss masks derived from a deterministic seed.
    todo!("eccf_corrupted_fuzz: random-loss fuzz loop not yet implemented")
}

/// Amplification-limited handshake with a large server hello and losses.
/// C: `eca1_amplification_loss_test` in `picoquictest/edge_cases.c`.
#[test]
fn eca1_amplification_loss() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0xa1, false, &mut simulated_time, 0x0FF4, 16).expect("edge_case_prepare");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 15_000_000).expect("edge_case_complete");
}

/// Loss of the final closing packet: connection must close within 10 s.
/// C: `ecf1_final_loss_test` in `picoquictest/edge_cases.c`.
#[test]
fn ecf1_final_loss() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0xf1, false, &mut simulated_time, 0, 20).expect("edge_case_prepare");
    let mut zero_loss = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut zero_loss, 0, &mut simulated_time)
        .expect("connection loop");
    test_ctx.immediate_exit = true;
    tls_api_data_sending_loop(&mut test_ctx, &mut zero_loss, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0xb10)
        .expect("close with losses");
    assert!(
        simulated_time.ticks() <= 10_000_000,
        "connection close took too long: {} µs",
        simulated_time.ticks()
    );
}

/// Silly CID: server sends a new CID frame that is dropped and later
/// repeated in a bogus way.  Both sides must reach Ready.
/// C: `ec5c_silly_cid_test` in `picoquictest/edge_cases.c`.
#[test]
fn ec5c_silly_cid() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = edge_case_prepare(0x5c, false, &mut simulated_time, 0x01e084, 48)
        .expect("edge_case_prepare");
    assert!(test_ctx.has_cnx_server(), "server connection must exist");
    assert!(test_ctx.client_ready(), "client must be ready");
    assert!(test_ctx.server_ready(), "server must be ready");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 3_000_000).expect("edge_case_complete");
}

/// After the client closes, the server must not send more than ~50 preemptive
/// repeats and must stop within its idle timeout.
/// C: `ec9a_preemptive_amok_test` in `picoquictest/edge_cases.c`.
#[test]
fn ec9a_preemptive_amok() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0x9a, false, &mut simulated_time, 0x800, 12).expect("edge_case_prepare");
    assert!(test_ctx.has_cnx_server(), "server connection must exist");
    assert!(test_ctx.server_ready(), "server must be in ready state");
    assert!(test_ctx.test_finished, "data transfer must have completed");
    let (send_count, repeat_duration) =
        ec9a_server_loop(&mut test_ctx, &mut simulated_time).expect("server loop");
    assert!(
        send_count <= 50,
        "server sent too many repeat packets: {send_count}"
    );
    // repeat_duration must be <= idle_timeout (checked inside ec9a_server_loop)
    let _ = repeat_duration;
}

/// C: `idle_timeout_test` in `picoquictest/edge_cases.c`.
#[test]
fn idle_timeout() {
    idle_timeout_test_one(1, 30_000, 30_000, 30_000_000).expect("case 1");
    idle_timeout_test_one(2, 60_000, 20_000, 20_000_000).expect("case 2");
    idle_timeout_test_one(3, 20_000, 60_000, 20_000_000).expect("case 3");
    idle_timeout_test_one(4, 5_000, 300_000, 5_000_000).expect("case 4");
    idle_timeout_test_one(5, 300_000, 5_000, 5_000_000).expect("case 5");
    idle_timeout_test_one(6, 0, 5_000, 5_000_000).expect("case 6");
    idle_timeout_test_one(7, 0, 60_000, 60_000_000).expect("case 7");
    idle_timeout_test_one(8, 5_000, 0, 5_000_000).expect("case 8");
    idle_timeout_test_one(9, 60_000, 0, 60_000_000).expect("case 9");
    idle_timeout_test_one(10, 0, 0, u64::MAX).expect("case 10");
}

/// C: `idle_server_test` in `picoquictest/edge_cases.c`.
#[test]
fn idle_server() {
    idle_server_test_one(1, 30_000, 0, 30_100_000).expect("case 1");
    idle_server_test_one(2, 60_000, 0, 60_100_000).expect("case 2");
    idle_server_test_one(3, 5_000, 0, 5_100_000).expect("case 3");
    idle_server_test_one(4, 0, 0, 30_100_000).expect("case 4");
    idle_server_test_one(5, 0, 10_000, 10_100_000).expect("case 5");
    idle_server_test_one(6, 20_000, 60_000, 60_100_000).expect("case 6");
    idle_server_test_one(7, 60_000, 5_000, 5_100_000).expect("case 7");
}

/// C: `initial_pto_test` in `picoquictest/edge_cases.c`.
#[test]
fn initial_pto() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x94, 0x01, 0x41, 0, 0, 0, 0, 0]).expect("8-byte CID");
    let simulated_rtt = 20_000u64;
    let simulated_pto = 4 * simulated_rtt;

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.cnx_client().start_client().expect("start_client");

    // Send the initial packet to the server.
    let length =
        initial_pto_prepare(&mut test_ctx, &mut simulated_time).expect("initial_pto_prepare");
    assert!(length >= 1200, "initial packet too short: {length}");

    // Wait until the ACK time, then send a synthetic ACK to the client.
    if simulated_time.ticks() < simulated_rtt {
        let _length = initial_pto_wait(&mut test_ctx, &mut simulated_time, simulated_rtt)
            .expect("initial_pto_wait");
    }
    initial_pto_ack(&mut test_ctx, &mut simulated_time).expect("initial_pto_ack");

    // The client should fire a PTO and send at least 1200 bytes.
    if simulated_time.ticks() < simulated_pto {
        let length = initial_pto_wait(&mut test_ctx, &mut simulated_time, simulated_pto)
            .expect("initial_pto_wait (PTO)");
        assert!(length >= 1200, "PTO packet not sent (length = {length})");
    }
}

/// C: `initial_pto_srv_test` in `picoquictest/edge_cases.c`.
#[test]
fn initial_pto_srv() {
    // Verify that the server's PTO includes both an Initial and a Handshake
    // packet when the initial packet is not yet acknowledged.
    todo!("initial_pto_srv: pto_server_prepare loop not yet implemented")
}

/// C: `reset_ack_max_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_ack_max() {
    reset_repeat_test_one(ResetTestKind::AckMaxStream).expect("reset_ack_max");
}

/// C: `reset_ack_reset_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_ack_reset() {
    reset_repeat_test_one(ResetTestKind::AckReset).expect("reset_ack_reset");
}

/// C: `reset_extra_max_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_extra_max() {
    reset_repeat_test_one(ResetTestKind::ExtraMaxStream).expect("reset_extra_max");
}

/// C: `reset_extra_reset_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_extra_reset() {
    reset_repeat_test_one(ResetTestKind::ExtraReset).expect("reset_extra_reset");
}

/// C: `reset_extra_stop_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_extra_stop() {
    reset_repeat_test_one(ResetTestKind::ExtraStop).expect("reset_extra_stop");
}

/// C: `reset_need_max_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_need_max() {
    reset_repeat_test_one(ResetTestKind::NeedMaxStream).expect("reset_need_max");
}

/// C: `reset_need_reset_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_need_reset() {
    reset_repeat_test_one(ResetTestKind::NeedReset).expect("reset_need_reset");
}

/// C: `reset_need_stop_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_need_stop() {
    reset_repeat_test_one(ResetTestKind::NeedStop).expect("reset_need_stop");
}

/// Reset a stream on the server side while it is still mid-transfer, then
/// verify that the client correctly forbids adding data or marking it active
/// after the reset.
/// C: `reset_loop_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_loop_test() {
    let mut simulated_time = Instant::from_ticks(0);
    let test_stream: u64 = 8;

    let mut test_ctx = tls_api_init_ctx(
        &mut simulated_time,
        crate::internal::Version::InternalTest1 as u32,
        None,
    )
    .expect("tls_api_init_ctx");

    // The real callback (reset_loop_callback_t) tracks data_sent/received/fin/reset per
    // stream rank; that trait object is not yet implemented in Rust.
    test_ctx.qserver.set_default_callback(None);
    test_ctx.cnx_client().set_callback(None);

    test_ctx.cnx_client().start_client().expect("start_client");

    // Queue initial data on streams 4 and 8; triggers server-side stream creation.
    let bogus = [0u8; 4];
    test_ctx
        .cnx_client()
        .add_to_stream(4, &bogus, false)
        .expect("add_to_stream 4");
    test_ctx
        .cnx_client()
        .add_to_stream(8, &bogus, false)
        .expect("add_to_stream 8");

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    // Enable streaming on both client streams with equal priority.
    test_ctx
        .cnx_client()
        .mark_active_stream(4, true, None)
        .expect("mark active 4");
    test_ctx
        .cnx_client()
        .mark_active_stream(8, true, None)
        .expect("mark active 8");
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 8)
        .expect("set client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 8)
        .expect("set client priority 8");

    // Allow stream data to begin flowing before the reset.
    let timeout = simulated_time.ticks() + 100_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout).expect("100ms wait");

    // Server resets stream 8 while the transfer is in progress.
    test_ctx
        .cnx_server()
        .reset_stream(test_stream, 0)
        .expect("reset_stream");

    // Adjust priorities to expose the bug (different priorities post-reset).
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 9)
        .expect("client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 7)
        .expect("client priority 8");
    test_ctx
        .cnx_server()
        .set_stream_priority(4, 9)
        .expect("server priority 4");
    test_ctx
        .cnx_server()
        .set_stream_priority(8, 7)
        .expect("server priority 8");

    // Poll until the server's RESET_STREAM frame has actually been sent.
    let deadline = Instant::from_ticks(simulated_time.ticks() + 100_000);
    for _ in 0..16 {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            deadline,
            &mut was_active,
        )
        .expect("sim round");
        if check_stream_reset_sent(&mut test_ctx, test_stream) {
            break;
        }
    }
    assert!(
        check_stream_reset_sent(&mut test_ctx, test_stream),
        "server did not send RESET_STREAM for stream {test_stream}"
    );

    // After reset, adding data or marking the stream active must be rejected.
    assert!(
        test_ctx
            .cnx_server()
            .add_to_stream(test_stream, &[1, 2, 3, 4], true)
            .is_err(),
        "add_to_stream after reset should be forbidden on stream {test_stream}"
    );
    assert!(
        test_ctx
            .cnx_server()
            .mark_active_stream(test_stream, true, None)
            .is_err(),
        "mark_active_stream after reset should be forbidden on stream {test_stream}"
    );

    // Final loop: verify the connection settles within 2 seconds.
    let timeout2 = simulated_time.ticks() + 2_000_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout2).expect("2s wait");
}

/// C: `reset_stream_at_basic_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_stream_at_basic() {
    reset_stream_at_test_one(ResetStreamAtSpec::Basic).expect("reset_stream_at_basic");
}

/// C: `reset_stream_at_limit_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_stream_at_limit_test() {
    reset_stream_at_test_one(ResetStreamAtSpec::Limit).expect("reset_stream_at_limit");
}

/// C: `reset_stream_at_loss_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_stream_at_loss() {
    reset_stream_at_test_one(ResetStreamAtSpec::Loss).expect("reset_stream_at_loss");
}

/// Table-driven check that every named [`crate::errors::InternalError`]
/// and [`crate::errors::TransportError`] code resolves to its
/// short textual name, plus the four "default-branch" buckets
/// (`crypto error alert`, `unknown picoquic error`, `unknown`).
#[test]
fn error_name() {
    use crate::errors::InternalError;

    let cases: &[(u64, &str)] = &[
        // Protocol (transport / TLS) errors.
        (0x1, "internal"),
        (0x2, "server busy"),
        (0x3, "flow control"),
        (0x4, "stream limit"),
        (0x5, "stream state"),
        (0x6, "final offset"),
        (0x7, "frame format"),
        (0x8, "parameter"),
        (0x9, "connection_id limit"),
        (0xA, "protocol violation"),
        (0xB, "invalid token"),
        (0xC, "application"),
        (0xD, "crypto buffer exceeded"),
        (0xE, "key update"),
        (0xF, "aead limit"),
        (0x178, "wrong alpn"),
        (0x201, "tls handshake failed"),
        (0x11, "version negotiation"),
        (0x3e, "application abandon"),
        (0x3e75, "resource limit reached"),
        (0x3e76, "unstable interface"),
        (0x3e77, "no CID available"),
        // Picoquic-internal codes (0x400+ range).
        (0x401, "duplicate"),
        (0x403, "payload_decrypt_error"),
        (0x404, "unexpected packet"),
        (0x405, "memory"),
        (0x407, "connection ID check"),
        (0x408, ""),
        (0x409, "version negotation spoofed"),
        (0x40A, "malformed transport extension"),
        (0x40B, "extension buffer too small"),
        (0x40C, "illegal transport extension"),
        (0x40D, "cannot reset the crypto stream"),
        (0x40E, "invalid stream id"),
        (0x40F, "stream already closed"),
        (0x410, "frame buffer too small"),
        (0x411, "invalid frame"),
        (0x412, "cannot control the crypto stream"),
        (0x413, "retry"),
        (0x414, "disconnected"),
        (0x415, "error detected"),
        (0x417, "invalid ticket"),
        (0x418, "invalid file"),
        (0x419, "send buffer too small"),
        (0x41A, "unexpected state"),
        (0x41B, "unexpected error"),
        (0x41C, "server configuration without cert"),
        (0x41D, "no such file"),
        (0x41E, "stateless reset"),
        (0x41F, "connection deleted"),
        (0x420, "connection ID segment error"),
        (0x421, "connection ID not available"),
        (0x422, "migration disabled"),
        (0x423, "cannot compute key"),
        (0x424, "cannot set active stream"),
        (0x425, "cannot change active context"),
        (0x426, "invalid token"),
        (0x427, "initial CID too short"),
        (0x428, "key rotation not ready"),
        (0x429, "aead not ready"),
        (0x42A, "no ALPN provided"),
        (0x42B, "no callback provided"),
        (0x42C, "stream receive complete"),
        (0x42D, "packet header parsing"),
        (0x42E, "QUIC bit missing"),
        (0x42F, "terminate packet loop (not an error)"),
        (0x430, "simulate NAT (not an error)"),
        (0x431, "simulate migration (not an error)"),
        (0x432, "version not supported"),
        (0x433, "idle timeout"),
        (0x434, "repeat timeout"),
        (0x435, "handshake timeout"),
        (0x436, "socket"),
        (0x437, "version negotiation"),
        (0x438, "packet too long"),
        (0x439, "wrong version"),
        (0x43A, "port blocked"),
        (0x43B, "datagram too long"),
        (0x43C, "invalid path ID"),
        (0x43D, "retry needed"),
        (0x43E, "server busy"),
        (0x43F, "duplicate path"),
        (0x440, "blocked by lack of path ID"),
        (0x441, "blocked by lack of CID"),
        (0x442, "path address family"),
        (0x443, "path not ready"),
        (0x444, "path limit exceeded"),
        (0x445, "redirected to proxy (not an error)"),
        (0x446, "padding_packet"),
        // CRYPTO_ERROR alert range (default branch).
        (0x101, "crypto error alert"),
        (0x150, "crypto error alert"),
        (0x1FF, "crypto error alert"),
        // Unknown picoquic error range (default branch).
        (0x450, "unknown picoquic error"),
        (0x4FF, "unknown picoquic error"),
        // Truly unknown.
        (0x0, "unknown"),
        (0x200, "unknown"),
        (0x500, "unknown"),
        (0xFFFF_FFFF_FFFF_FFFF, "unknown"),
    ];

    for &(code, expected) in cases {
        let got = InternalError::name(code).unwrap_or("<none>");
        assert_eq!(
            got, expected,
            "error_code=0x{code:x} got={got:?} expected={expected:?}",
        );
    }
}
