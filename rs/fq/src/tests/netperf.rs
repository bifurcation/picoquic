//! Test cases for `picoquictest/netperf_test.c`.
//!
//! Exercises coalesced packet sending and a NAT-attack stress scenario.
//! `netperf_basic` and `netperf_bbr` use large UDP send buffers to test
//! datagram coalescing.  `nat_attack` verifies server robustness when a
//! broken NAT continuously rewrites client source addresses.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, TestTlsApiCtx, test_api_init_send_recv_scenario,
    tls_api_init_ctx_ex2_delayed, tls_api_one_scenario_body_verify, tls_api_one_scenario_init_ex,
    tls_api_one_scenario_verify, tls_api_one_sim_round, wait_client_connection_ready,
};
use crate::internal::Version;
use crate::{
    CongestionAlgorithm, Instant, MAX_PACKET_SIZE, State, get_congestion_algorithm,
    register_all_congestion_control_algorithms,
};

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

    let mut test_ctx = tls_api_init_ctx_ex2_delayed(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        None,
        None,
        None,
    )
    .expect("tls_api_init_ctx_ex2_delayed");

    test_ctx.set_send_buffer_size(send_buffer_size);

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
    let completion_time = {
        let start_time = test_ctx.cnx_client().start_time.ticks();
        simulated_time.ticks().saturating_sub(start_time)
    };
    let test_finished = test_ctx.test_finished;
    let (server_trains, server_packets) = if test_ctx.has_cnx_server() {
        let cnx_s = test_ctx.cnx_server();
        (cnx_s.nb_trains_sent, cnx_s.nb_packets_sent)
    } else {
        (0, 0)
    };

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, max_completion_microsec)
        .unwrap_or_else(|err| {
            panic!(
                "scenario body verify failed: {err:?}; completion_time={completion_time}, max_completion_microsec={max_completion_microsec}, test_finished={test_finished}, server_trains={server_trains}, server_packets={server_packets}"
            )
        });

    // Check that datagram coalescing occurred.
    assert!(test_ctx.has_cnx_server(), "Cannot check server stats");
    let cnx_s = test_ctx.cnx_server();
    assert!(
        cnx_s.nb_trains_sent * 3 / 2 <= cnx_s.nb_packets_sent,
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

fn nat_attack_loop(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    do_attack: bool,
) -> crate::Result<()> {
    let mut nb_loops = 0u32;
    let mut nb_inactive = 0u32;

    loop {
        let client_connected = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|cnx| cnx.connection_state != State::Disconnected)
            .unwrap_or(false);
        if !client_connected {
            break;
        }

        if do_attack {
            if let Some(packet) = test_ctx.s_to_c_link.packets.front_mut() {
                packet.addr_to = Some(test_ctx.client_addr);
            }
            if let Some(packet) = test_ctx.c_to_s_link.packets.front_mut() {
                let mut rewritten = packet.addr_from.unwrap_or(test_ctx.client_addr);
                rewritten.set_port(test_ctx.client_addr.port().wrapping_add(nb_loops as u16));
                packet.addr_from = Some(rewritten);
            }
        }

        let mut was_active = false;
        tls_api_one_sim_round(
            test_ctx,
            simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
            if nb_inactive > 64 {
                return Err(crate::Error::InvalidState);
            }
        }

        nb_loops += 1;
        if nb_loops > 100_000 {
            return Err(crate::Error::InvalidState);
        }

        if test_ctx.test_finished {
            let client_empty = test_ctx
                .qclient
                .first_cnx_mut()
                .map(|cnx| cnx.is_backlog_empty())
                .unwrap_or(true);
            let server_empty = test_ctx
                .qserver
                .first_cnx_mut()
                .map(|cnx| cnx.is_backlog_empty())
                .unwrap_or(true);
            if client_empty && server_empty {
                break;
            }
        }
    }

    Ok(())
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
    register_all_congestion_control_algorithms();
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

    nat_attack_loop(&mut test_ctx, &mut simulated_time, true).expect("nat attack loop");

    if test_ctx.cnx_client().state() == State::Ready {
        tls_api_one_scenario_verify(&test_ctx).expect("scenario verify");
    }
}
