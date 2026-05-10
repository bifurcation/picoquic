//! Test cases for `picoquictest/cpu_limited.c`.
//!
//! Exercises CPU-limited client scenarios with different congestion-control
//! algorithms (NewReno, Cubic, BBR) and stream patterns (small vs. large
//! transfers, batched initial streams, flow-control-limited safe mode).
//!
//! The simulator models a client that can only process a fixed number of
//! packets per second: each receive or send call consumes `incoming_cpu_time`
//! / `prepare_cpu_time` microseconds, and arrivals during that window back up
//! in a bounded queue.

#![allow(non_snake_case)]

use super::util::{TestApiStreamDesc, tls_api_init_ctx_ex, tls_api_one_scenario_body};
use crate::internal::Version;
use crate::{CongestionAlgorithm, ConnectionId, Instant, get_congestion_algorithm};

// ---------------------------------------------------------------------------
// Shared config type.  C: `limited_test_config_t`.

struct LimitedTestConfig {
    test_id: u8,
    ccalgo: &'static CongestionAlgorithm,
    incoming_cpu_time: u64,
    prepare_cpu_time: u64,
    packet_queue_max: usize,
    nb_initial_steps: usize,
    nb_final_steps: usize,
    max_completion_time: u64,
    microsec_latency: u64,
    picosec_per_byte: u64,
    flow_control_max: u64,
    nb_losses_max: u64,
}

/// Build a default limited-client config.  C: `limited_config_set_default`.
fn limited_config_default(test_id: u8) -> LimitedTestConfig {
    crate::register_all_congestion_control_algorithms();

    LimitedTestConfig {
        test_id,
        ccalgo: get_congestion_algorithm("newreno").expect("newreno algo"),
        incoming_cpu_time: 2_000,
        prepare_cpu_time: 2_000,
        packet_queue_max: 16,
        nb_final_steps: 2,
        nb_initial_steps: 0,
        max_completion_time: 0,
        microsec_latency: 50_000,
        picosec_per_byte: 80_000, // 100 Mbps
        flow_control_max: 0,
        nb_losses_max: 0,
    }
}

/// Build a scenario of `nb_initial_steps` small (32 KB response) streams
/// followed by `nb_final_steps` large (1 MB response) streams.
/// C: `limited_client_create_scenario`.
fn limited_client_create_scenario(
    nb_initial_steps: usize,
    nb_final_steps: usize,
) -> Vec<TestApiStreamDesc> {
    let nb_steps = nb_initial_steps + nb_final_steps;
    let mut scenario = Vec::with_capacity(nb_steps);
    let mut previous_stream_id: u64 = 0;

    for _ in 0..nb_initial_steps {
        let old_prev = previous_stream_id;
        previous_stream_id += 4;
        scenario.push(TestApiStreamDesc {
            stream_id: previous_stream_id,
            previous_stream_id: old_prev,
            q_len: 257,
            r_len: 32_000,
        });
    }

    // Note: previous_stream_id is held constant across the final loop, and
    // stream IDs are spaced by the loop index — faithfully transcribed from C.
    for i in nb_initial_steps..nb_steps {
        scenario.push(TestApiStreamDesc {
            stream_id: previous_stream_id + 4 * (i as u64 + 1),
            previous_stream_id,
            q_len: 257,
            r_len: 1_000_000,
        });
    }

    scenario
}

/// Run one cpu-limited client scenario end-to-end.
/// C: `limited_client_test_one`.
fn limited_client_test_one(config: LimitedTestConfig) {
    let mut simulated_time = Instant::from_ticks(0);

    let initial_cid =
        ConnectionId::clone_from_slice(&[0x11, 0x01, 0xc1, 0x1e, 0x44, config.test_id, 0, 0])
            .expect("8-byte CID");

    let scenario = limited_client_create_scenario(config.nb_initial_steps, config.nb_final_steps);

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    let ccalgo = config.ccalgo;
    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(ccalgo);

    test_ctx.client_endpoint.incoming_cpu_time = config.incoming_cpu_time;
    test_ctx.client_endpoint.prepare_cpu_time = config.prepare_cpu_time;
    test_ctx.client_endpoint.packet_queue_max = config.packet_queue_max;

    test_ctx.qserver.use_long_log = true;
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qclient.use_long_log = true;
    test_ctx.qclient.set_qlog(".").ok();

    test_ctx.c_to_s_link.microsec_latency = config.microsec_latency;
    test_ctx.c_to_s_link.picosec_per_byte = config.picosec_per_byte;
    test_ctx.s_to_c_link.microsec_latency = config.microsec_latency;
    test_ctx.s_to_c_link.picosec_per_byte = config.picosec_per_byte;

    if config.flow_control_max != 0 {
        test_ctx
            .qclient
            .set_max_data_control(config.flow_control_max);
    }

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        &scenario,
        0,
        0,
        0,
        4 * config.microsec_latency,
        config.max_completion_time,
    )
    .expect("scenario body");

    if config.nb_losses_max != 0 {
        assert!(
            test_ctx.cnx_server().nb_retransmission_total < config.nb_losses_max,
            "got {} retransmissions, expected < {}",
            test_ctx.cnx_server().nb_retransmission_total,
            config.nb_losses_max,
        );
    }
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `limited_reno_test` in `picoquictest/cpu_limited.c`.
#[test]
fn limited_reno() {
    let mut config = limited_config_default(1);
    config.ccalgo = get_congestion_algorithm("newreno").expect("newreno algo");
    config.max_completion_time = 4_600_000;
    limited_client_test_one(config);
}

/// C: `limited_cubic_test` in `picoquictest/cpu_limited.c`.
#[test]
fn limited_cubic() {
    let mut config = limited_config_default(2);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic algo");
    config.max_completion_time = 4_200_000;
    limited_client_test_one(config);
}

/// C: `limited_bbr_test` in `picoquictest/cpu_limited.c`.
#[test]
fn limited_bbr() {
    let mut config = limited_config_default(3);
    config.ccalgo = get_congestion_algorithm("bbr").expect("bbr algo");
    config.max_completion_time = 4_100_000;
    limited_client_test_one(config);
}

/// C: `limited_batch_test` in `picoquictest/cpu_limited.c`.
#[test]
fn limited_batch() {
    let mut config = limited_config_default(4);
    config.ccalgo = get_congestion_algorithm("bbr").expect("bbr algo");
    config.max_completion_time = 6_200_000;
    config.nb_initial_steps = 10;
    limited_client_test_one(config);
}

/// C: `limited_safe_test` in `picoquictest/cpu_limited.c`.
#[test]
fn limited_safe() {
    let mut config = limited_config_default(5);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic algo");
    config.max_completion_time = 5_450_000;
    // Bug noted in original C: there should be 0 or maybe 1 losses.
    config.nb_losses_max = 6;
    config.flow_control_max = 57_344;
    limited_client_test_one(config);
}
