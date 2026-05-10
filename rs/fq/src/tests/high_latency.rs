//! Test cases for `picoquictest/high_latency_test.c`.
//!
//! Exercises the QUIC stack over simulated high-latency satellite links
//! (5-second one-way delay) with various congestion-control algorithms.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, test_api_init_send_recv_scenario, tls_api_connection_loop,
    tls_api_data_sending_loop, tls_api_one_scenario_body_verify, tls_api_one_scenario_init_ex,
    wait_client_connection_ready,
};
use crate::internal::{Version, init_transport_parameters};
use crate::tp::TransportParameters;
use crate::{
    CongestionAlgorithm, ConnectionId, Duration, Instant, get_congestion_algorithm,
    register_all_congestion_control_algorithms,
};

// ---------------------------------------------------------------------------
// Shared large scenario: 100 × 1 MB streams.
// C: `hilat_scenario_100mb[]` in `picoquictest/high_latency_test.c`.

const HILAT_SCENARIO_100MB: &[TestApiStreamDesc] = &{
    const fn desc(id: u64) -> TestApiStreamDesc {
        TestApiStreamDesc {
            stream_id: id,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 1_000_000,
        }
    }
    [
        desc(4),
        desc(8),
        desc(12),
        desc(16),
        desc(20),
        desc(24),
        desc(28),
        desc(32),
        desc(36),
        desc(40),
        desc(44),
        desc(48),
        desc(52),
        desc(56),
        desc(60),
        desc(64),
        desc(68),
        desc(72),
        desc(76),
        desc(80),
        desc(84),
        desc(88),
        desc(92),
        desc(96),
        desc(100),
        desc(104),
        desc(108),
        desc(112),
        desc(116),
        desc(120),
        desc(124),
        desc(128),
        desc(132),
        desc(136),
        desc(140),
        desc(144),
        desc(148),
        desc(152),
        desc(156),
        desc(160),
        desc(164),
        desc(168),
        desc(172),
        desc(176),
        desc(180),
        desc(184),
        desc(188),
        desc(192),
        desc(196),
        desc(200),
        desc(204),
        desc(208),
        desc(212),
        desc(216),
        desc(220),
        desc(224),
        desc(228),
        desc(232),
        desc(236),
        desc(240),
        desc(244),
        desc(248),
        desc(252),
        desc(256),
        desc(260),
        desc(264),
        desc(268),
        desc(272),
        desc(276),
        desc(280),
        desc(284),
        desc(288),
        desc(292),
        desc(296),
        desc(300),
        desc(304),
        desc(308),
        desc(312),
        desc(316),
        desc(320),
        desc(324),
        desc(328),
        desc(332),
        desc(336),
        desc(340),
        desc(344),
        desc(348),
        desc(352),
        desc(356),
        desc(360),
        desc(364),
        desc(368),
        desc(372),
        desc(376),
        desc(380),
        desc(384),
        desc(388),
        desc(392),
        desc(396),
        desc(400),
    ]
};

fn high_latency_ccalgo(name: &str) -> &'static CongestionAlgorithm {
    register_all_congestion_control_algorithms();
    get_congestion_algorithm(name).unwrap_or_else(|| panic!("cc algo not found: {name}"))
}

// ---------------------------------------------------------------------------
// Core helper.
// C: `high_latency_one` in `picoquictest/high_latency_test.c`.

fn high_latency_one(
    test_id: u8,
    ccalgo: &'static CongestionAlgorithm,
    scenario: &[TestApiStreamDesc],
    max_completion_time: u64,
    latency: u64,
    mbps_up: u64,
    mbps_down: u64,
    jitter: u64,
    has_loss: bool,
    do_preemptive: bool,
    seed_bw: bool,
) {
    let mut simulated_time = Instant::from_ticks(0);
    let picosec_per_byte_up = (1_000_000u64 * 8) / mbps_up;
    let picosec_per_byte_down = (1_000_000u64 * 8) / mbps_down;

    let mut client_params = TransportParameters::default();
    init_transport_parameters(&mut client_params);
    client_params.enable_time_stamp = 3;

    let mut server_params = TransportParameters::default();
    init_transport_parameters(&mut server_params);
    server_params.enable_time_stamp = 3;

    // Build an initial CID with test-specific bytes, matching the C layout.
    let mut cid_bytes = [0u8; 8];
    cid_bytes[0] = 0x1a;
    cid_bytes[1] = 0x7e;
    cid_bytes[2] = test_id;
    cid_bytes[3] = if mbps_up > 0xff { 0xff } else { mbps_up as u8 };
    cid_bytes[4] = if mbps_down > 0xff {
        0xff
    } else {
        mbps_down as u8
    };
    cid_bytes[5] = if latency > 16_000_000 {
        0xff
    } else {
        (latency / 100_000) as u8
    };
    cid_bytes[6] = if jitter > 255_000 {
        0xff
    } else {
        (jitter / 1_000) as u8
    };
    cid_bytes[7] = if has_loss { 0x30 } else { 0x00 };
    if seed_bw {
        cid_bytes[7] |= 0x80;
    }
    if do_preemptive {
        cid_bytes[7] ^= 0x0f;
    }
    let initial_cid = ConnectionId::clone_from_slice(&cid_bytes).expect("initial CID");

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        Some(&client_params),
        Some(&server_params),
        Some(&initial_cid),
    )
    .expect("test context");

    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_preemptive_repeat(do_preemptive);
    test_ctx.qserver.set_preemptive_repeat_policy(do_preemptive);

    test_ctx.c_to_s_link.jitter = jitter;
    test_ctx.c_to_s_link.microsec_latency = latency;
    test_ctx.c_to_s_link.picosec_per_byte = picosec_per_byte_up;
    test_ctx.s_to_c_link.microsec_latency = latency;
    test_ctx.s_to_c_link.picosec_per_byte = picosec_per_byte_down;
    test_ctx.s_to_c_link.jitter = jitter;
    test_ctx.stream0_flow_release = true;
    test_ctx.immediate_exit = true;

    if seed_bw {
        let estimated_rtt = 2 * latency;
        let estimated_bdp = estimated_rtt * 125_000 * mbps_up / 1_000_000;
        let server_ip = test_ctx.server_addr.ip();
        test_ctx.cnx_client().seed_bandwidth(
            Duration::from_ticks(estimated_rtt),
            estimated_bdp,
            server_ip,
        );
    }

    test_ctx.cnx_client().set_pmtud_required(true);

    let _ = test_ctx.qserver.set_qlog(".");
    let _ = test_ctx.qclient.set_qlog(".");

    let mut loss_mask: u64 = if has_loss { 0x1000_0000 } else { 0 };
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        2 * latency,
        &mut simulated_time,
    )
    .expect("connection loop");
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time).expect("client ready");
    test_api_init_send_recv_scenario(&mut test_ctx, scenario).expect("scenario init");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, max_completion_time)
        .expect("scenario verify");

    if do_preemptive {
        assert!(
            test_ctx.cnx_client().preemptive_repeat_count() > 0,
            "expected non-zero preemptive repeats"
        );
    }
}

// ---------------------------------------------------------------------------
// Exported tests.

/// C: `high_latency_basic_test` in `picoquictest/high_latency_test.c`.
#[test]
fn high_latency_basic() {
    let latency = 5_000_000u64;
    let newreno = high_latency_ccalgo("newreno");
    high_latency_one(
        0xba,
        newreno,
        &[TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 2_000,
        }],
        latency * 7,
        latency,
        10,
        10,
        0,
        false,
        false,
        false,
    );
}

/// C: `high_latency_bbr_test` in `picoquictest/high_latency_test.c`.
#[test]
fn high_latency_bbr() {
    let latency = 5_000_000u64;
    let bbr = high_latency_ccalgo("bbr");
    high_latency_one(
        0xbb,
        bbr,
        HILAT_SCENARIO_100MB,
        145_000_000,
        latency,
        10,
        10,
        0,
        false,
        false,
        false,
    );
}

/// C: `high_latency_cubic_test` in `picoquictest/high_latency_test.c`.
#[test]
fn high_latency_cubic() {
    let latency = 5_000_000u64;
    let cubic = high_latency_ccalgo("cubic");
    high_latency_one(
        0xcb,
        cubic,
        HILAT_SCENARIO_100MB,
        200_000_000,
        latency,
        10,
        10,
        0,
        false,
        false,
        false,
    );
}

/// C: `high_latency_probeRTT_test` in `picoquictest/high_latency_test.c`.
#[test]
fn high_latency_probertt() {
    let latency = 5_000_000u64;
    let bbr = high_latency_ccalgo("bbr");
    high_latency_one(
        0xf1,
        bbr,
        HILAT_SCENARIO_100MB,
        839_000_000,
        latency,
        1,
        1,
        0,
        false,
        false,
        false,
    );
}
