//! Test cases for `picoquictest/satellite_test.c`.
//!
//! Exercises the QUIC stack over simulated satellite links
//! (300 ms one-way delay, 250 Mbps down / variable up) with various
//! congestion-control algorithms, loss, jitter, and bandwidth seeding.

#![allow(non_snake_case)]

use super::util::{tls_api_one_scenario_body, tls_api_one_scenario_init_ex};
use crate::binlog::Binlog as _;
use crate::internal::{Version, init_transport_parameters};
use crate::tp::TransportParameters;
use crate::{CongestionAlgorithm, ConnectionId, Duration, Instant, get_congestion_algorithm};

// ---------------------------------------------------------------------------
// Core helper.
// C: `satellite_test_one` in `picoquictest/satellite_test.c`.

#[allow(clippy::too_many_arguments)]
fn satellite_test_one(
    ccalgo: &'static CongestionAlgorithm,
    data_size: usize,
    max_completion_time: u64,
    mbps_up: u64,
    mbps_down: u64,
    jitter: u64,
    has_loss: bool,
    do_preemptive: bool,
    seed_bw: bool,
    low_flow: bool,
    flow_control: bool,
) {
    let mut simulated_time = Instant::from_ticks(0);
    let latency: u64 = 300_000;
    let picosec_per_byte_up = (1_000_000u64 * 8) / mbps_up;
    let picosec_per_byte_down = (1_000_000u64 * 8) / mbps_down;

    let mut client_params = TransportParameters::default();
    init_transport_parameters(&mut client_params);
    client_params.enable_time_stamp = 3;

    let mut server_params = TransportParameters::default();
    init_transport_parameters(&mut server_params);
    server_params.enable_time_stamp = 3;

    if low_flow || flow_control {
        let bdp_s = (mbps_up * latency * 2) / 8;
        let bdp_c = (mbps_up * latency * 2) / 8;
        if low_flow {
            server_params.initial_max_data = bdp_s / 2;
            client_params.initial_max_data = bdp_c / 2;
        } else {
            server_params.initial_max_data = bdp_s * 2;
            client_params.initial_max_data = bdp_c * 2;
        }
    }

    // Build an initial CID with algo/link-specific bytes, matching the C layout.
    let mut cid_bytes = [0u8; 8];
    cid_bytes[0] = 0x5a;
    cid_bytes[1] = 0x4e;
    cid_bytes[2] = ccalgo.congestion_algorithm_number;
    cid_bytes[3] = if mbps_up > 0xff { 0xff } else { mbps_up as u8 };
    cid_bytes[4] = if mbps_down > 0xff {
        0xff
    } else {
        mbps_down as u8
    };
    cid_bytes[5] = if latency > 2_550_000 {
        0xff
    } else {
        (latency / 10_000) as u8
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
        cid_bytes[7] |= 0x40;
    }
    if has_loss {
        cid_bytes[7] |= 0x20;
    }
    if low_flow {
        cid_bytes[7] |= 0x10;
    }
    if flow_control {
        cid_bytes[7] |= 0x08;
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
    test_ctx.qserver.set_preemptive_repeat_policy(do_preemptive);
    test_ctx.cnx_client().set_preemptive_repeat(do_preemptive);

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

    if low_flow || flow_control {
        let max_data = server_params.initial_max_data;
        test_ctx.qserver.set_max_data_control(max_data);
    }

    test_ctx.cnx_client().set_pmtud_required(true);

    test_ctx.qclient.set_binlog(Some(".")).ok();
    test_ctx.qclient.use_long_log = true;
    test_ctx.cnx_client().new_connection();

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        &[],
        if has_loss { 0x1000_0000u64 } else { 0 },
        data_size as i32,
        0,
        2 * latency,
        max_completion_time,
    )
    .expect("scenario completed");

    if do_preemptive {
        let nb_preemptive = test_ctx.cnx_client().preemptive_repeat_count();
        assert!(nb_preemptive > 0, "expected non-zero preemptive repeats");

        let bdp = mbps_up * latency * 2;
        let send_mtu = test_ctx.cnx_client().primary_path_send_mtu();
        let bdp_p = bdp / (8 * send_mtu);
        let bdp_p_plus = bdp_p + (bdp_p / 2);
        assert!(
            nb_preemptive <= bdp_p_plus,
            "preemptive repeats {} > BDP(packets) {}",
            nb_preemptive,
            bdp_p
        );
    }

    if flow_control {
        let bdp = mbps_up * latency * 2;
        let send_mtu = test_ctx.cnx_client().primary_path_send_mtu();
        let bdp_p = bdp / (8 * send_mtu);
        let nb_max = 3 * bdp_p;
        let allocated = test_ctx.qserver.nb_data_nodes_allocated_max as u64;
        assert!(
            allocated <= nb_max,
            "allocated nodes {} > 3*{}",
            allocated,
            bdp_p
        );
    }
}

// ---------------------------------------------------------------------------
// Exported tests.

/// C: `satellite_basic_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_basic() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        5_500_000,
        250,
        3,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}

/// C: `satellite_seeded_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_seeded() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        4_900_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}

/// C: `satellite_seeded_bbr1_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_seeded_bbr1() {
    let bbr1 = get_congestion_algorithm("bbr1").expect("bbr1");
    satellite_test_one(
        bbr1,
        100_000_000,
        5_500_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}

/// C: `satellite_loss_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_loss() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        8_000_000,
        250,
        3,
        0,
        true,
        false,
        false,
        false,
        false,
    );
}

/// C: `satellite_loss_fc_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_loss_fc() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        12_500_000,
        250,
        3,
        0,
        true,
        false,
        false,
        false,
        true,
    );
}

/// C: `satellite_preemptive_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_preemptive() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        7_100_000,
        250,
        3,
        0,
        true,
        true,
        false,
        false,
        false,
    );
}

/// C: `satellite_jitter_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_jitter() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        6_700_000,
        250,
        3,
        3_000,
        false,
        false,
        false,
        false,
        false,
    );
}

/// C: `satellite_medium_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_medium() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        18_200_000,
        50,
        10,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}

/// C: `satellite_small_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_small() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        81_500_000,
        10,
        2,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}

/// C: `satellite_small_up_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_small_up() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        400_000_000,
        2,
        10,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}

/// C: `satellite_bbr1_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_bbr1() {
    let bbr1 = get_congestion_algorithm("bbr1").expect("bbr1");
    satellite_test_one(
        bbr1,
        100_000_000,
        7_000_000,
        250,
        3,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}

/// C: `satellite_cubic_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_cubic() {
    let cubic = get_congestion_algorithm("cubic").expect("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        6_500_000,
        250,
        3,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}

/// C: `satellite_cubic_seeded_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_cubic_seeded() {
    let cubic = get_congestion_algorithm("cubic").expect("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        5_000_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}

/// C: `satellite_cubic_loss_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_cubic_loss() {
    let cubic = get_congestion_algorithm("cubic").expect("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        7_500_000,
        250,
        3,
        0,
        true,
        false,
        false,
        false,
        false,
    );
}

/// C: `satellite_dcubic_seeded_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_dcubic_seeded() {
    let dcubic = get_congestion_algorithm("dcubic").expect("dcubic");
    satellite_test_one(
        dcubic,
        100_000_000,
        5_300_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}

/// C: `satellite_prague_seeded_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_prague_seeded() {
    let prague = get_congestion_algorithm("prague").expect("prague");
    satellite_test_one(
        prague,
        100_000_000,
        5_300_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}

/// C: `satellite_preemptive_fc_test` in `picoquictest/satellite_test.c`.
#[test]
fn satellite_preemptive_fc() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr, 10_000_000, 20_000_000, 20, 2, 0, true, true, false, true, false,
    );
}
