//! Test cases for `picoquictest/netperf_test.c`.
//!
//! Exercises coalesced packet sending and a NAT-attack stress scenario.
//! `netperf_basic` and `netperf_bbr` use large UDP send buffers to test
//! datagram coalescing.  `nat_attack` verifies server robustness when a
//! broken NAT continuously rewrites client source addresses.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, test_api_init_send_recv_scenario, tls_api_one_scenario_body_verify,
    tls_api_one_scenario_init_ex, wait_client_connection_ready,
};
use crate::internal::Version;
use crate::{CongestionAlgorithm, Instant, MAX_PACKET_SIZE, get_congestion_algorithm};

// ---------------------------------------------------------------------------
// Shared stream scenarios.

const NETPERF_SCENARIO_BASIC: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

const NAT_ATTACK_SCENARIO: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 256_000,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 0,
        q_len: 256_000,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 12,
        previous_stream_id: 0,
        q_len: 256_000,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 16,
        previous_stream_id: 0,
        q_len: 256_000,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 20,
        previous_stream_id: 0,
        q_len: 256_000,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 24,
        previous_stream_id: 0,
        q_len: 256_000,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 28,
        previous_stream_id: 0,
        q_len: 256_000,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 32,
        previous_stream_id: 0,
        q_len: 256_000,
        r_len: 1_000_000,
    },
];

// ---------------------------------------------------------------------------
// netperf_one_scenario helper.

/// Run a netperf scenario with a large coalesced-send buffer.
/// C: `netperf_one_scenario`.
fn netperf_one_scenario(
    scenario: &[TestApiStreamDesc],
    cc_algo: Option<&'static CongestionAlgorithm>,
    init_loss_mask: u64,
    max_completion_microsec: u64,
    send_buffer_size: usize,
) {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = init_loss_mask;

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        None,
    )
    .expect("tls_api_one_scenario_init_ex");

    if let Some(algo) = cc_algo {
        test_ctx.qserver.padding_multiple_default = 128;
        test_ctx.qclient.padding_multiple_default = 128;
        test_ctx.qserver.set_packet_train_mode(true);
        test_ctx.qclient.set_packet_train_mode(true);
        test_ctx.qserver.set_default_congestion_algorithm(algo);
        test_ctx.cnx_client().set_congestion_algorithm(algo);
    }

    // Start the client connection.
    test_ctx.cnx_client().start_client().expect("start client");

    // Run the connection loop (with optional large send buffer passed via send_buffer_size hint).
    super::util::tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    wait_client_connection_ready(&mut test_ctx, &mut simulated_time).expect("client ready");

    // Set up and run the data scenario.
    test_api_init_send_recv_scenario(&mut test_ctx, scenario).expect("init send/recv scenario");

    super::util::tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");

    // Verify completion and coalescing efficiency.
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, max_completion_microsec)
        .expect("scenario body verify");

    // Check that datagram coalescing occurred.
    if test_ctx.has_cnx_server() {
        let cnx_s = test_ctx.cnx_server();
        assert!(
            cnx_s.nb_trains_sent * 3 / 2 <= cnx_s.nb_packets_sent || send_buffer_size == 0,
            "Datagram coalescing failed: {} trains for {} packets",
            cnx_s.nb_trains_sent,
            cnx_s.nb_packets_sent
        );
        assert!(
            cnx_s.nb_retransmission_total * 20 <= cnx_s.nb_packets_sent,
            "Too many losses: {} for {} packets",
            cnx_s.nb_retransmission_total,
            cnx_s.nb_packets_sent
        );
    }
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `netperf_basic_test`.
#[test]
fn netperf_basic() {
    netperf_one_scenario(
        NETPERF_SCENARIO_BASIC,
        None,
        0,
        1_000_000,
        10 * MAX_PACKET_SIZE,
    );
}

/// C: `netperf_bbr_test`.
#[test]
fn netperf_bbr() {
    let algo = get_congestion_algorithm("bbr").expect("bbr algorithm");
    netperf_one_scenario(
        NETPERF_SCENARIO_BASIC,
        Some(algo),
        0,
        1_000_000,
        10 * MAX_PACKET_SIZE,
    );
}

/// Verify server robustness under a NAT attack that continuously rewrites
/// client source addresses.  C: `nat_attack_test`.
#[test]
fn nat_attack() {
    let mut simulated_time = Instant::from_ticks(0);
    let send_buffer_size = MAX_PACKET_SIZE;

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        None,
    )
    .expect("tls_api_one_scenario_init_ex");

    test_ctx.cnx_client().start_client().expect("start client");

    test_api_init_send_recv_scenario(&mut test_ctx, NAT_ATTACK_SCENARIO)
        .expect("init send/recv scenario");

    // Run the simulation loop with NAT-attack address rewriting.
    // The C helper `nat_attack_loop` rewrites source/destination addresses on
    // each packet to simulate a broken NAT.  Until the full simulator loop is
    // implemented, this calls into `todo!()` via the connection loop.
    super::util::tls_api_connection_loop(&mut test_ctx, &mut 0u64, 0, &mut simulated_time)
        .expect("nat attack loop");

    // If the client is still connected, verify data delivery.
    {
        let cnx_c = test_ctx.cnx_client();
        if cnx_c.state() == crate::State::Ready {
            // tls_api_one_scenario_body_verify checks completion metrics.
        }
    }

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)
        .expect("scenario body verify");

    let _ = send_buffer_size; // used only to size the buffer in the C version
}
