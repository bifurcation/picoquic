//! Tests for `picoquictest/ack_frequency_test.c`.
//!
//! Verifies that the ACK-frequency extension is negotiated correctly
//! and that the resulting ACK gap / delay / packet-interval values
//! fall within expected bounds after a simulated data transfer.
//!
//! Tests covered:
//! * [`ackfrq_basic`] — C `ackfrq_basic_test`: 10 ms RTT, cubic, moderate bandwidth.
//! * [`ackfrq_short`] — C `ackfrq_short_test`: 10 µs RTT, cubic, moderate bandwidth.

#![allow(non_snake_case)]

use crate::internal::Version;
use crate::{CongestionAlgorithm, Duration, Instant, get_congestion_algorithm};

use super::util::{
    TestApiStreamDesc, test_api_init_send_recv_scenario, tls_api_connection_loop,
    tls_api_data_sending_loop, tls_api_one_scenario_body_verify, tls_api_one_scenario_init_ex,
    wait_client_connection_ready,
};

// ---------------------------------------------------------------------------
// Shared scenario: one stream, 257-byte query, 1 MB response.
// C: `test_scenario_ackfrq[]`.

const TEST_SCENARIO_ACKFRQ: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

// ---------------------------------------------------------------------------
// Per-test parameter bundle.

struct AckfrqTestSpec {
    /// One-way link latency in microseconds.
    latency: u64,
    /// Link data rate (server→client) in picoseconds per byte.
    picosec_per_byte_down: u64,
    /// Link data rate (client→server) in picoseconds per byte.
    picosec_per_byte_up: u64,
    /// Congestion-control algorithm to install on both sides.
    ccalgo: &'static CongestionAlgorithm,
    /// Maximum simulation time allowed for the scenario (0 = no limit).
    target_time: u64,
    /// Maximum allowed `max_ack_delay_remote` on the client connection.
    max_ack_delay_remote: Duration,
    /// Maximum allowed `max_ack_gap_remote` on the client connection.
    max_ack_gap_remote: u64,
    /// Maximum allowed `min_ack_delay_remote` on the client connection.
    min_ack_delay_remote: Duration,
    /// Expected average inter-packet interval on the server (±25 %).
    target_interval: Duration,
}

// ---------------------------------------------------------------------------
// Shared test body.  C: `ackfrq_test_one`.

fn ackfrq_test_one(spec: &AckfrqTestSpec) {
    let mut simulated_time = Instant::from_ticks(0);
    // id[2] encodes the test variant; ackfrq_test_basic == 0 so the
    // zero byte already carries the right value.
    let initial_cid = crate::ConnectionId::clone_from_slice(&[0xac, 0xf8, 0, 0, 0, 0, 0, 0])
        .expect("8-byte initial CID");

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_one_scenario_init_ex");

    test_ctx
        .qserver
        .set_default_congestion_algorithm(spec.ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(spec.ccalgo);

    test_ctx.c_to_s_link.microsec_latency = spec.latency;
    test_ctx.s_to_c_link.microsec_latency = spec.latency;

    if spec.picosec_per_byte_down > 0 {
        test_ctx.s_to_c_link.picosec_per_byte = spec.picosec_per_byte_down;
    }
    if spec.picosec_per_byte_up > 0 {
        test_ctx.c_to_s_link.picosec_per_byte = spec.picosec_per_byte_up;
    }

    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qserver.set_log_level(1);
    test_ctx.qclient.set_log_level(1);
    test_ctx.qclient.use_long_log = true;
    test_ctx.qserver.use_long_log = true;

    test_ctx.cnx_client().start_client().ok();

    let mut loss_mask: u64 = 0;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)
        .expect("client connection ready");

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_ACKFRQ)
        .expect("init send/recv scenario");

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, spec.target_time)
        .expect("scenario body verify");

    assert!(
        test_ctx.cnx_client().is_ack_frequency_negotiated,
        "ACK frequency not negotiated at client"
    );
    assert!(
        test_ctx.cnx_server().is_ack_frequency_negotiated,
        "ACK frequency not negotiated at server"
    );

    assert!(
        test_ctx.cnx_client().max_ack_delay_remote <= spec.max_ack_delay_remote,
        "max_ack_delay_remote {:?} > {:?}",
        test_ctx.cnx_client().max_ack_delay_remote,
        spec.max_ack_delay_remote,
    );
    assert!(
        test_ctx.cnx_client().min_ack_delay_remote <= spec.min_ack_delay_remote,
        "min_ack_delay_remote {:?} > {:?}",
        test_ctx.cnx_client().min_ack_delay_remote,
        spec.min_ack_delay_remote,
    );
    assert!(
        test_ctx.cnx_client().max_ack_gap_remote <= spec.max_ack_gap_remote,
        "max_ack_gap_remote {} > {}",
        test_ctx.cnx_client().max_ack_gap_remote,
        spec.max_ack_gap_remote,
    );

    // Verify that the average inter-ACK interval is within 25 % of the
    // expected target.  C: `duration / nb_packets_received`.
    {
        let server = test_ctx.cnx_server();
        let duration_ticks = simulated_time
            .ticks()
            .saturating_sub(server.start_time.ticks());
        let interval_ticks = duration_ticks / server.nb_packets_received;
        let interval_min = interval_ticks - (interval_ticks >> 2);
        let interval_max = interval_ticks + (interval_ticks >> 2);
        let target_ticks = spec.target_interval.ticks();
        assert!(
            target_ticks >= interval_min && target_ticks <= interval_max,
            "interval {} <> target {}",
            interval_ticks,
            target_ticks,
        );
    }
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `ackfrq_basic_test` in `picoquictest/ack_frequency_test.c`.
///
/// 10 ms RTT, cubic, 100 Mbit/s symmetric.  Checks that ACK frequency
/// negotiation completes and that max-delay ≤ 6 ms, gap ≤ 40, min-delay
/// ≤ 1 ms, and average inter-packet interval ≈ 4 ms.
#[test]
fn ackfrq_basic() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    ackfrq_test_one(&AckfrqTestSpec {
        latency: 10_000,
        picosec_per_byte_up: 80_000,
        picosec_per_byte_down: 80_000,
        ccalgo,
        target_time: 0,
        max_ack_delay_remote: Duration::from_ticks(6_000),
        max_ack_gap_remote: 40,
        min_ack_delay_remote: Duration::from_ticks(1_000),
        target_interval: Duration::from_ticks(4_000),
    });
}

/// C: `ackfrq_short_test` in `picoquictest/ack_frequency_test.c`.
///
/// 10 µs RTT, cubic, 100 Mbit/s symmetric.  Checks that ACK frequency
/// negotiation completes with tighter bounds: max-delay ≤ 1 ms, gap ≤ 32,
/// min-delay ≤ 1 ms, and average inter-packet interval ≈ 1 ms.
#[test]
fn ackfrq_short() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    ackfrq_test_one(&AckfrqTestSpec {
        latency: 10,
        picosec_per_byte_up: 80_000,
        picosec_per_byte_down: 80_000,
        ccalgo,
        target_time: 0,
        max_ack_delay_remote: Duration::from_ticks(1_000),
        max_ack_gap_remote: 32,
        min_ack_delay_remote: Duration::from_ticks(1_000),
        target_interval: Duration::from_ticks(1_000),
    });
}
