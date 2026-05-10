//! Test cases for `picoquictest/l4s_test.c`.
//!
//! L4S (Low Latency, Low Loss, Scalable Throughput) congestion tests.
//! Each entry exercises a specific congestion-control algorithm under
//! DualQ AQM conditions.  The shared helper `l4s_congestion_test`
//! mirrors the C static function of the same name.

#![allow(non_snake_case)]

use super::dualq::Dualq;
use super::util::{
    TestApiStreamDesc, VaryLinkSpec, tls_api_init_ctx_ex2, tls_api_one_scenario_body_ex,
};
use crate::internal::Version;
use crate::{CongestionAlgorithm, ConnectionId, Instant, get_congestion_algorithm};

/// Four sequential streams used by all L4S tests.  C: `test_scenario_l4s[]`.
const TEST_SCENARIO_L4S: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 4,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 12,
        previous_stream_id: 8,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 16,
        previous_stream_id: 12,
        q_len: 257,
        r_len: 1_000_000,
    },
];

/// Updown bandwidth schedule: 1 s at 10 Mbps → 2 s at 2 Mbps → recover.
/// C: `l4s_link_updown[]`.
static L4S_LINK_UPDOWN: &[VaryLinkSpec] = &[
    VaryLinkSpec {
        duration: 1_000_000,
        bits_per_second_up: 10_000_000,
        bits_per_second_down: 10_000_000,
        microsec_latency: 10_000,
    },
    VaryLinkSpec {
        duration: 2_000_000,
        bits_per_second_up: 2_000_000,
        bits_per_second_down: 2_000_000,
        microsec_latency: 10_000,
    },
    VaryLinkSpec {
        duration: 8_000_000,
        bits_per_second_up: 10_000_000,
        bits_per_second_down: 10_000_000,
        microsec_latency: 10_000,
    },
];

/// Core L4S congestion test.  C: `l4s_congestion_test`.
fn l4s_congestion_test(
    ccalgo: &'static CongestionAlgorithm,
    do_l4s: bool,
    max_completion_time: u64,
    max_losses: u64,
    max_rttvar: u64,
    link_states: &[VaryLinkSpec],
) {
    let mut simulated_time = Instant::from_ticks(0);
    let l4s_max = max_rttvar;

    let mut queue_delay_max: u64 = 20_000;
    for ls in link_states {
        let candidate = 2 * ls.microsec_latency;
        if candidate > queue_delay_max {
            queue_delay_max = candidate;
        }
    }

    let mut initial_cid_bytes = [0x45u8, 0xcc, 0, 0, 0, 0, 0, 0];
    initial_cid_bytes[2] = ccalgo.congestion_algorithm_number;
    initial_cid_bytes[3] = link_states.len() as u8;
    let initial_cid =
        ConnectionId::clone_from_slice(&initial_cid_bytes).expect("8-byte initial CID");

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    test_ctx.c_to_s_link.queue_delay_max = queue_delay_max;
    test_ctx.s_to_c_link.queue_delay_max = queue_delay_max;

    test_ctx.qclient.set_default_congestion_algorithm(ccalgo);
    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(ccalgo);

    if do_l4s {
        test_ctx.ecn_support = 1;
        test_ctx.packet_ecn_default = 1; // PICOQUIC_ECN_ECT_1
        Dualq::install(&mut test_ctx.c_to_s_link, l4s_max).expect("dualq configure c_to_s");
        Dualq::install(&mut test_ctx.s_to_c_link, l4s_max).expect("dualq configure s_to_c");
    }

    test_ctx.qserver.set_qlog(".").ok();

    tls_api_one_scenario_body_ex(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_L4S,
        0,
        0,
        0,
        queue_delay_max,
        max_completion_time,
        link_states,
    )
    .expect("tls_api_one_scenario_body_ex");

    assert!(
        test_ctx.has_cnx_server(),
        "server connection must exist after scenario"
    );
    assert!(
        test_ctx.cnx_server().nb_retransmission_total <= max_losses,
        "retransmissions {} exceeded max {}",
        test_ctx.cnx_server().nb_retransmission_total,
        max_losses,
    );
    assert!(
        test_ctx.cnx_server().primary_path_rtt_variant() <= max_rttvar,
        "rtt_variant {} exceeded max {}",
        test_ctx.cnx_server().primary_path_rtt_variant(),
        max_rttvar,
    );
}

/// Reference test: NewReno under L4S (each CE mark causes congestion).
/// C: `l4s_reno_test`.
#[test]
fn l4s_reno() {
    let ccalgo = get_congestion_algorithm("newreno").expect("newreno cc algo");
    l4s_congestion_test(ccalgo, true, 5_600_000, 45, 3_000, &[]);
}

/// Prague (L4S-native) under DualQ AQM.  C: `l4s_prague_test`.
#[test]
fn l4s_prague() {
    let ccalgo = get_congestion_algorithm("prague").expect("prague cc algo");
    l4s_congestion_test(ccalgo, true, 4_100_000, 9, 4_500, &[]);
}

/// Prague under DualQ AQM with updown bandwidth variation.
/// C: `l4s_prague_updown_test`.
#[test]
fn l4s_prague_updown() {
    crate::register_all_congestion_control_algorithms();
    let ccalgo = get_congestion_algorithm("prague").expect("prague cc algo");
    l4s_congestion_test(ccalgo, true, 6_300_000, 55, 6_000, L4S_LINK_UPDOWN);
}

/// BBR under DualQ AQM.  C: `l4s_bbr_test`.
#[test]
fn l4s_bbr() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    l4s_congestion_test(ccalgo, true, 3_800_000, 21, 3_000, &[]);
}

/// C4 under DualQ AQM.  C: `l4s_c4_test`.
#[test]
fn l4s_c4() {
    crate::register_all_congestion_control_algorithms();
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    l4s_congestion_test(ccalgo, true, 3_600_000, 30, 3_000, &[]);
}

/// BBR under DualQ AQM with updown bandwidth variation.
/// C: `l4s_bbr_updown_test` (Windows 32-bit guard omitted — not applicable).
#[test]
fn l4s_bbr_updown() {
    crate::register_all_congestion_control_algorithms();
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    l4s_congestion_test(ccalgo, true, 5_800_000, 69, 3_000, L4S_LINK_UPDOWN);
}
