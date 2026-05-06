//! Test cases for `picoquictest/congestion_test.c`.
//!
//! Exercises the network simulator with multiple congestion-control algorithms
//! (Cubic, C4, FastCC, BBR, BBR1, NewReno, dCubic) across a variety of link
//! configurations: symmetric/asymmetric paths, jitter, slow long-haul links,
//! a 2-second blackhole window, app-limited flows, and a CWIN-max cap.
//!
//! Helper functions mirror the C static helpers directly; every `#[test]`
//! entry is a thin wrapper that selects parameters and delegates.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, check_bytes_in_flight, save_empty_tickets, test_api_init_send_recv_scenario,
    tls_api_data_sending_loop, tls_api_init_ctx_ex, tls_api_one_scenario_body,
    tls_api_one_scenario_body_connect, tls_api_one_scenario_body_verify,
    tls_api_one_scenario_init_ex,
};
use crate::internal::{Version, init_transport_parameters};
use crate::tp::TransportParameters;
use crate::{CongestionAlgorithm, ConnectionId, Duration, Instant, get_congestion_algorithm};

// ---------------------------------------------------------------------------
// Shared stream scenarios.  C: file-scope statics in congestion_test.c.

/// Four sequential 1 MB streams.  C: `test_scenario_congestion[]`.
const TEST_SCENARIO_CONGESTION: &[TestApiStreamDesc] = &[
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

/// Ten independent 1 MB streams (10 MB total).  C: `test_scenario_10mb[]`.
const TEST_SCENARIO_10MB: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 12,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 16,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 20,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 24,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 28,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 32,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 36,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 40,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
];

/// One 1 MB stream.  C: `test_scenario_very_long[]`.
const TEST_SCENARIO_VERY_LONG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

const TICKET_FILE_NAME: &str = "resume_tests_tickets.bin";
const APP_LIMIT_TRACE_QLOG: &str = "acc1020304050607.server.qlog";
const CWIN_MAX_TRACE_QLOG: &str = "c9149a0102030405.server.qlog";

// ---------------------------------------------------------------------------
// BDP test option enum.  C: `bdp_test_option_enum`.

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum BdpTestOption {
    None = 0,
    Basic = 1,
    Rtt = 2,
    Ip = 3,
    Delay = 4,
    Reno = 5,
    Cubic = 6,
    Short = 7,
    ShortLo = 8,
    ShortHi = 9,
    Bbr1 = 10,
}

// ---------------------------------------------------------------------------
// Private test helpers.

/// Run a congestion-control scenario on a symmetric 1 Mbps, 10 ms link.
/// C: `congestion_control_test`.
fn congestion_control_test(
    ccalgo: &'static CongestionAlgorithm,
    max_completion_time: u64,
    jitter: u64,
    jitter_id: u8,
) {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid = ConnectionId::clone_from_slice(&[
        0xcc,
        0xcc,
        ccalgo.congestion_algorithm_number,
        jitter_id,
        0,
        0,
        0,
        0,
    ])
    .expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(ccalgo);

    test_ctx.c_to_s_link.jitter = jitter;
    test_ctx.s_to_c_link.jitter = jitter;

    test_ctx.qserver.set_qlog(".").ok();

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_CONGESTION,
        0,
        0,
        0,
        20_000 + 2 * jitter,
        max_completion_time,
    )
    .expect("scenario body");
}

/// Run a "long" congestion test: connect, send at 1 Mbps for 1024 rounds,
/// multiply latency by 5, then drain to completion.
/// C: `congestion_long_test`.
fn congestion_long_test(ccalgo: &'static CongestionAlgorithm) {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xbb, 0xcc, 0x10, 0, 0, 0, 0, 0]).expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(ccalgo);

    test_ctx.c_to_s_link.jitter = 0;
    test_ctx.s_to_c_link.jitter = 0;
    test_ctx.c_to_s_link.picosec_per_byte = 8_000_000; // 1 Mbps

    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qserver.use_long_log = true;

    tls_api_one_scenario_body_connect(&mut test_ctx, &mut simulated_time, 0, 0).expect("connect");

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_CONGESTION)
        .expect("init scenario");

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 1024)
        .expect("data loop 1 (1024 rounds)");

    test_ctx.c_to_s_link.microsec_latency *= 5;
    test_ctx.s_to_c_link.microsec_latency *= 5;

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data loop 2 (drain)");

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 15_000_000)
        .expect("scenario verify");
}

/// Run a performance test: 10 MB download on an asymmetric link.
/// C: `performance_test_one`.
fn performance_test_one(
    max_completion_time: u64,
    mbps: u64,
    rkbps: u64,
    latency: u64,
    jitter: u64,
    buffer_size: u64,
    server_params: Option<&TransportParameters>,
) {
    let mut simulated_time = Instant::from_ticks(0x0005a138fbde8743u64);
    let picosec_per_byte_down = (1_000_000u64 * 8) / mbps;
    let picosec_per_byte_up = (1_000_000_000u64 * 8)
        .checked_div(rkbps)
        .unwrap_or(picosec_per_byte_down);
    let buffer_id = (buffer_size * 16) / (latency + jitter);
    let initial_cid = ConnectionId::clone_from_slice(&[
        0xbbu8,
        0xcc,
        0,
        if rkbps > 0xff { 0xff } else { rkbps as u8 },
        if mbps > 0xff { 0xff } else { mbps as u8 },
        if latency > 2_550_000 {
            0xff
        } else {
            (latency / 10_000) as u8
        },
        if jitter > 255_000 {
            0xff
        } else {
            (jitter / 1_000) as u8
        },
        if buffer_id > 255 {
            0xff
        } else {
            buffer_id as u8
        },
    ])
    .expect("8-byte CID");

    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        server_params,
        Some(&initial_cid),
    )
    .expect("tls_api_one_scenario_init_ex");

    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(ccalgo);
    test_ctx.qserver.use_long_log = true;

    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qclient.set_qlog(".").ok();

    test_ctx.c_to_s_link.jitter = jitter;
    test_ctx.c_to_s_link.microsec_latency = latency;
    test_ctx.c_to_s_link.picosec_per_byte = picosec_per_byte_up;
    test_ctx.s_to_c_link.microsec_latency = latency;
    test_ctx.s_to_c_link.picosec_per_byte = picosec_per_byte_down;
    test_ctx.s_to_c_link.jitter = jitter;

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_10MB,
        0,
        0,
        0,
        buffer_size,
        max_completion_time,
    )
    .expect("scenario body");
}

/// Symmetric-path performance shorthand.  C: `performance_test`.
fn performance_test(
    max_completion_time: u64,
    mbps: u64,
    latency: u64,
    jitter: u64,
    buffer_size: u64,
) {
    performance_test_one(
        max_completion_time,
        mbps,
        0,
        latency,
        jitter,
        buffer_size,
        None,
    );
}

/// Run the two-pass BDP option test.  C: `bdp_option_test_one`.
fn bdp_option_test_one(bdp_test_option: BdpTestOption) {
    let mut simulated_time = Instant::from_ticks(0);
    let proposed_version: u32 = 0;
    let mut max_completion_time = 6_800_000u64;
    let latency = 300_000u64;
    let mut buffer_size = 2 * latency;
    let initial_cid_template = [0xbdu8, 0x80, 0, 0, 0, 0, 0, 0];
    let ccalgo_bbr = get_congestion_algorithm("bbr").expect("bbr cc algo");
    let mut ccalgo: &'static CongestionAlgorithm = ccalgo_bbr;

    save_empty_tickets(TICKET_FILE_NAME, simulated_time).expect("init empty tickets");

    for i in 0u8..2 {
        if i == 1 && bdp_test_option == BdpTestOption::Delay {
            simulated_time =
                Instant::from_ticks(simulated_time.ticks() + 48 * 3_600 * 1_000_000u64);
        }

        let mut initial_cid_bytes = initial_cid_template;
        initial_cid_bytes[2] = i;
        initial_cid_bytes[3] = bdp_test_option as u8;
        let initial_cid = ConnectionId::clone_from_slice(&initial_cid_bytes).expect("8-byte CID");

        let ticket_file = if i == 0 { None } else { Some(TICKET_FILE_NAME) };

        let mut test_ctx = tls_api_init_ctx_ex(
            &mut simulated_time,
            proposed_version,
            ticket_file,
            Some(&initial_cid),
        )
        .expect("tls_api_init_ctx_ex");

        // Link setup.
        test_ctx.c_to_s_link.microsec_latency = latency;
        test_ctx.s_to_c_link.microsec_latency = latency;
        test_ctx.c_to_s_link.picosec_per_byte = (1_000_000u64 * 8) / 20;
        test_ctx.s_to_c_link.picosec_per_byte = (1_000_000u64 * 8) / 20;

        // Per-option link / timing overrides.
        if matches!(
            bdp_test_option,
            BdpTestOption::Short | BdpTestOption::ShortLo | BdpTestOption::ShortHi
        ) {
            max_completion_time = 4_500_000;
            test_ctx.c_to_s_link.microsec_latency = 100_000;
            test_ctx.s_to_c_link.microsec_latency = 100_000;
            buffer_size = 2 * test_ctx.c_to_s_link.microsec_latency;
            if i == 0 {
                if bdp_test_option == BdpTestOption::ShortLo {
                    test_ctx.c_to_s_link.picosec_per_byte *= 2;
                    test_ctx.s_to_c_link.picosec_per_byte *= 2;
                } else if bdp_test_option == BdpTestOption::ShortHi {
                    test_ctx.c_to_s_link.picosec_per_byte /= 2;
                    test_ctx.s_to_c_link.picosec_per_byte /= 2;
                }
            } else if bdp_test_option == BdpTestOption::ShortLo {
                max_completion_time = 4_650_000;
            }
        } else if i > 0 {
            match bdp_test_option {
                BdpTestOption::None => {}
                BdpTestOption::Basic => {
                    max_completion_time = 5_900_000;
                }
                BdpTestOption::Rtt => {
                    max_completion_time = 4_610_000;
                    test_ctx.c_to_s_link.microsec_latency = 50_000;
                    test_ctx.s_to_c_link.microsec_latency = 50_000;
                    buffer_size = 2 * test_ctx.c_to_s_link.microsec_latency;
                }
                BdpTestOption::Ip => {
                    test_ctx.set_client_addr(0x08080808, 2345);
                    max_completion_time = 9_000_000;
                }
                BdpTestOption::Delay => {
                    max_completion_time = 8_000_000;
                }
                BdpTestOption::Reno => {
                    max_completion_time = 6_750_000;
                }
                _ => {}
            }
        }

        // Congestion algorithm selection.
        match bdp_test_option {
            BdpTestOption::Reno => {
                ccalgo = get_congestion_algorithm("newreno").expect("newreno cc algo");
            }
            BdpTestOption::Cubic => {
                ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
                max_completion_time = 10_000_000;
            }
            BdpTestOption::Bbr1 => {
                ccalgo = get_congestion_algorithm("bbr1").expect("bbr1 cc algo");
            }
            _ => {}
        }

        test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
        test_ctx.cnx_client().set_congestion_algorithm(ccalgo);
        test_ctx.qclient.set_default_bdp_frame_option(true);
        test_ctx.qserver.set_default_bdp_frame_option(true);
        test_ctx.qserver.use_long_log = true;
        test_ctx.qserver.set_qlog(".").ok();

        let mut server_params = TransportParameters::default();
        init_transport_parameters(&mut server_params);
        let mut client_params = TransportParameters::default();
        init_transport_parameters(&mut client_params);
        server_params.enable_bdp_frame = true;
        client_params.enable_bdp_frame = true;
        client_params.initial_max_stream_data_bidi_remote = 1_000_000;
        client_params.initial_max_data = 10_000_000;
        test_ctx
            .cnx_client()
            .set_transport_parameters(&client_params);
        test_ctx
            .qserver
            .set_default_tp(&server_params)
            .expect("set_default_tp");

        tls_api_one_scenario_body(
            &mut test_ctx,
            &mut simulated_time,
            TEST_SCENARIO_10MB,
            0,
            0,
            0,
            buffer_size,
            if i == 0 { 0 } else { max_completion_time },
        )
        .expect("scenario body");

        // BDP negotiation checks.
        if i == 1 && bdp_test_option != BdpTestOption::Delay {
            assert!(
                test_ctx.cnx_client().nb_zero_rtt_acked > 0,
                "bdp {bdp_test_option:?} i={i}: no 0-RTT data acked",
            );
        }
        assert!(
            test_ctx.cnx_client().send_receive_bdp_frame,
            "bdp {bdp_test_option:?} i={i}: bdp option not negotiated on client",
        );
        assert!(
            test_ctx.cnx_server().send_receive_bdp_frame,
            "bdp {bdp_test_option:?} i={i}: bdp option not negotiated on server",
        );

        if i == 1 {
            if !matches!(
                bdp_test_option,
                BdpTestOption::Cubic | BdpTestOption::Delay | BdpTestOption::Ip
            ) {
                let (retrans, sent) = {
                    let s = test_ctx.cnx_server();
                    (s.nb_retransmission_total, s.nb_packets_sent)
                };
                assert!(
                    retrans * 10 <= sent,
                    "bdp {bdp_test_option:?} i={i}: too many losses {retrans}/{sent}",
                );
            }

            assert!(
                test_ctx.cnx_client().paths[0].is_bdp_sent,
                "bdp {bdp_test_option:?} i={i}: bdp frame not sent by client",
            );

            let expects_cwin_seed = matches!(
                bdp_test_option,
                BdpTestOption::Basic
                    | BdpTestOption::Reno
                    | BdpTestOption::Short
                    | BdpTestOption::ShortHi
                    | BdpTestOption::ShortLo
                    | BdpTestOption::Cubic
                    | BdpTestOption::Bbr1
            );
            let cwin_seeded = test_ctx.cnx_server().cwin_notified_from_seed;
            if expects_cwin_seed {
                assert!(
                    cwin_seeded,
                    "bdp {bdp_test_option:?} i={i}: cwin not seeded on server",
                );
            } else {
                assert!(
                    !cwin_seeded,
                    "bdp {bdp_test_option:?} i={i}: unexpected cwin seed on server",
                );
            }
        }

        // Save tickets for the second pass.
        assert!(
            !test_ctx.qclient.stored_tickets.is_empty(),
            "bdp {bdp_test_option:?} i={i}: no ticket received",
        );
        test_ctx
            .qclient
            .save_session_tickets(TICKET_FILE_NAME)
            .expect("save tickets");
    }
}

/// Blackhole recovery test: 2-second link outage in the middle of a transfer.
/// C: `blackhole_test_one`.
fn blackhole_test_one(ccalgo: &'static CongestionAlgorithm, max_completion_time: u64, jitter: u64) {
    let mut simulated_time = Instant::from_ticks(0);
    let latency: u64 = 15_000;
    let picosec_per_byte_10 = (1_000_000u64 * 8) / 10;
    let mut initial_cid_bytes = [0xb1u8, 0xac, 2, 3, 4, 5, 6, 7];
    initial_cid_bytes[2] = ccalgo.congestion_algorithm_number;
    let initial_cid = ConnectionId::clone_from_slice(&initial_cid_bytes).expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(ccalgo);

    test_ctx.c_to_s_link.jitter = jitter;
    test_ctx.c_to_s_link.microsec_latency = latency;
    test_ctx.c_to_s_link.picosec_per_byte = picosec_per_byte_10;
    test_ctx.s_to_c_link.microsec_latency = latency;
    test_ctx.s_to_c_link.picosec_per_byte = picosec_per_byte_10;
    test_ctx.s_to_c_link.jitter = jitter;
    test_ctx.blackhole_end = 7_000_000;
    test_ctx.blackhole_start = 5_000_000;

    test_ctx.qserver.set_qlog(".").ok();

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_10MB,
        0,
        0,
        0,
        2 * latency,
        max_completion_time,
    )
    .expect("scenario body");
}

/// Single-algorithm app-limited congestion test.
/// C: `app_limit_cc_test_one`.
fn app_limit_cc_test_one(ccalgo: &'static CongestionAlgorithm, max_completion_time: u64) {
    let mut simulated_time = Instant::from_ticks(0);
    let latency: u64 = 300_000;
    let picosec_per_byte_1 = 1_000_000u64 * 8; // 1 bps link
    let cwin_limit: u64 = 120_000;
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xacu8, 0xc1, 2, 3, 4, 5, 6, 7]).expect("8-byte CID");

    std::fs::remove_file(APP_LIMIT_TRACE_QLOG).ok();

    let mut client_params = TransportParameters::default();
    init_transport_parameters(&mut client_params);
    client_params.initial_max_data = 40_000;

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        Some(&client_params),
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_one_scenario_init_ex");

    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(ccalgo);
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qserver.use_long_log = true;
    test_ctx
        .qclient
        .set_max_data_control(client_params.initial_max_data);

    test_ctx.c_to_s_link.jitter = 0;
    test_ctx.c_to_s_link.microsec_latency = latency;
    test_ctx.c_to_s_link.picosec_per_byte = picosec_per_byte_1;
    test_ctx.s_to_c_link.microsec_latency = latency;
    test_ctx.s_to_c_link.picosec_per_byte = picosec_per_byte_1;
    test_ctx.s_to_c_link.jitter = 0;

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_VERY_LONG,
        0,
        0,
        0,
        2 * latency,
        max_completion_time,
    )
    .expect("scenario body");

    let bytes_in_flight_max = check_bytes_in_flight(APP_LIMIT_TRACE_QLOG).expect("check qlog");
    assert!(
        bytes_in_flight_max <= cwin_limit,
        "app_limit_cc ({} algo): max_in_flight={bytes_in_flight_max} > limit={cwin_limit}",
        ccalgo.congestion_algorithm_id,
    );
}

/// Single-algorithm CWIN-max test.
/// C: `cwin_max_test_one`.
fn cwin_max_test_one(
    ccalgo: &'static CongestionAlgorithm,
    cwin_limit: u64,
    max_completion_time: u64,
) {
    let mut simulated_time = Instant::from_ticks(0);
    let latency: u64 = 300_000;
    let picosec_per_byte_100 = (1_000_000u64 * 8) / 100;
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xc9u8, 0x14, 0x9a, 1, 2, 3, 4, 5]).expect("8-byte CID");

    std::fs::remove_file(CWIN_MAX_TRACE_QLOG).ok();

    let client_params = TransportParameters::default();

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        Some(&client_params),
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_one_scenario_init_ex");

    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(ccalgo);
    test_ctx.qserver.set_cwin_max(0x10000);
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qserver.use_long_log = true;
    test_ctx
        .qclient
        .set_max_data_control(client_params.initial_max_data);

    test_ctx.c_to_s_link.jitter = 0;
    test_ctx.c_to_s_link.microsec_latency = latency;
    test_ctx.c_to_s_link.picosec_per_byte = picosec_per_byte_100;
    test_ctx.s_to_c_link.microsec_latency = latency;
    test_ctx.s_to_c_link.picosec_per_byte = picosec_per_byte_100;
    test_ctx.s_to_c_link.jitter = 0;

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_VERY_LONG,
        0,
        0,
        0,
        2 * latency,
        max_completion_time,
    )
    .expect("scenario body");

    let bytes_in_flight_max = check_bytes_in_flight(CWIN_MAX_TRACE_QLOG).expect("check qlog");
    assert!(
        bytes_in_flight_max <= cwin_limit,
        "cwin_max ({} algo): max_in_flight={bytes_in_flight_max} > limit={cwin_limit}",
        ccalgo.congestion_algorithm_id,
    );
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `blackhole_test` in `picoquictest/congestion_test.c`.
#[test]
fn blackhole() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    blackhole_test_one(ccalgo, 15_000_000, 0);
}

/// C: `cubic_test` in `picoquictest/congestion_test.c`.
#[test]
fn cubic() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    congestion_control_test(ccalgo, 3_500_000, 0, 0);
}

/// C: `cubic_jitter_test` in `picoquictest/congestion_test.c`.
#[test]
fn cubic_jitter() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    congestion_control_test(ccalgo, 3_550_000, 5_000, 5);
}

/// C: `c4_test` in `picoquictest/congestion_test.c`.
#[test]
fn c4() {
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    congestion_control_test(ccalgo, 3_600_000, 0, 0);
}

/// C: `c4_jitter_test` in `picoquictest/congestion_test.c`.
#[test]
fn c4_jitter() {
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    congestion_control_test(ccalgo, 3_650_000, 5_000, 5);
}

/// C: `fastcc_test` in `picoquictest/congestion_test.c`.
#[test]
fn fastcc() {
    let ccalgo = get_congestion_algorithm("fastcc").expect("fastcc cc algo");
    congestion_control_test(ccalgo, 3_700_000, 0, 0);
}

/// C: `fastcc_jitter_test` in `picoquictest/congestion_test.c`.
#[test]
fn fastcc_jitter() {
    let ccalgo = get_congestion_algorithm("fastcc").expect("fastcc cc algo");
    congestion_control_test(ccalgo, 4_050_000, 5_000, 5);
}

/// C: `bbr_test` in `picoquictest/congestion_test.c`.
#[test]
fn bbr() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    congestion_control_test(ccalgo, 3_500_000, 0, 0);
}

/// C: `bbr_jitter_test` in `picoquictest/congestion_test.c`.
#[test]
fn bbr_jitter() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    congestion_control_test(ccalgo, 3_600_000, 5_000, 5);
}

/// C: `bbr_long_test` in `picoquictest/congestion_test.c`.
#[test]
fn bbr_long() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    congestion_long_test(ccalgo);
}

/// C: `c4_long_test` in `picoquictest/congestion_test.c`.
#[test]
fn c4_long() {
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    congestion_long_test(ccalgo);
}

/// C: `bbr_performance_test` in `picoquictest/congestion_test.c`.
///
/// 10 MB on a 100 Mbps link with 10 ms RTT and 3 ms jitter.
#[test]
fn bbr_performance() {
    let latency = 10_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(1_050_000, 100, latency, jitter, buffer);
}

/// C: `bbr_slow_long_test` in `picoquictest/congestion_test.c`.
///
/// 10 MB on a 1 Mbps link with 300 ms RTT.
#[test]
fn bbr_slow_long() {
    let latency = 300_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(81_000_000, 1, latency, jitter, buffer);
}

/// C: `bbr_one_second_test` in `picoquictest/congestion_test.c`.
///
/// 10 MB on a 1 Mbps link with 1 s RTT (pathological scenario).
#[test]
fn bbr_one_second() {
    let latency = 1_000_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(90_000_000, 1, latency, jitter, buffer);
}

/// C: `gbps_performance_test` in `picoquictest/congestion_test.c`.
///
/// 10 MB on a 1 Gbps link with 4 ms RTT (AWS-like scenario).
#[test]
fn bbr_gbps() {
    let latency = 4_000u64;
    let jitter = 2_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(250_000, 1_000, latency, jitter, buffer);
}

/// C: `bbr_asym100_test` in `picoquictest/congestion_test.c`.
///
/// 10 MB on 10 Mbps down / 100 kbps up, 1 ms RTT.
#[test]
fn bbr_asym100() {
    performance_test_one(8_500_000, 10, 100, 1_000, 750, 50_000, None);
}

/// C: `bbr_asym100_nodelay_test` in `picoquictest/congestion_test.c`.
///
/// Same as `bbr_asym100` but with delayed-ACK negotiation disabled.
#[test]
fn bbr_asym100_nodelay() {
    let mut server_params = TransportParameters::default();
    init_transport_parameters(&mut server_params);
    server_params.min_ack_delay = Duration::from_ticks(0);
    performance_test_one(8_500_000, 10, 100, 1_000, 750, 50_000, Some(&server_params));
}

/// C: `bbr_asym400_test` in `picoquictest/congestion_test.c`.
///
/// 10 MB on 40 Mbps down / 400 kbps up, 1 ms RTT.
#[test]
fn bbr_asym400() {
    performance_test_one(2_350_000, 40, 400, 1_000, 750, 50_000, None);
}

/// C: `bbr1_test` in `picoquictest/congestion_test.c`.
#[test]
fn bbr1() {
    let ccalgo = get_congestion_algorithm("bbr1").expect("bbr1 cc algo");
    congestion_control_test(ccalgo, 3_600_000, 0, 0);
}

/// C: `bbr1_long_test` in `picoquictest/congestion_test.c`.
#[test]
fn bbr1_long() {
    let ccalgo = get_congestion_algorithm("bbr1").expect("bbr1 cc algo");
    congestion_long_test(ccalgo);
}

/// C: `bdp_basic_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_basic() {
    bdp_option_test_one(BdpTestOption::Basic);
}

/// C: `bdp_delay_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_delay() {
    bdp_option_test_one(BdpTestOption::Delay);
}

/// C: `bdp_ip_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_ip() {
    bdp_option_test_one(BdpTestOption::Ip);
}

/// C: `bdp_rtt_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_rtt() {
    bdp_option_test_one(BdpTestOption::Rtt);
}

/// C: `bdp_reno_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_reno() {
    bdp_option_test_one(BdpTestOption::Reno);
}

/// C: `bdp_cubic_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_cubic() {
    bdp_option_test_one(BdpTestOption::Cubic);
}

/// C: `bdp_short_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_short() {
    bdp_option_test_one(BdpTestOption::Short);
}

/// C: `bdp_short_hi_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_short_hi() {
    bdp_option_test_one(BdpTestOption::ShortHi);
}

/// C: `bdp_short_lo_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_short_lo() {
    bdp_option_test_one(BdpTestOption::ShortLo);
}

/// C: `bdp_bbr1_test` in `picoquictest/congestion_test.c`.
#[test]
fn bdp_bbr1() {
    bdp_option_test_one(BdpTestOption::Bbr1);
}

/// C: `app_limit_cc_test` in `picoquictest/congestion_test.c`.
///
/// Runs the app-limited test with NewReno, Cubic, dCubic, BBR, FastCC, and BBR1.
#[test]
fn app_limit_cc() {
    let algo_names = ["newreno", "cubic", "dcubic", "bbr", "fastcc", "bbr1"];
    let max_completion_times: [u64; 6] = [
        22_000_000, 23_500_000, 22_000_000, 21_000_000, 25_000_000, 25_000_000,
    ];

    for (name, &max_time) in algo_names.iter().zip(max_completion_times.iter()) {
        let ccalgo =
            get_congestion_algorithm(name).unwrap_or_else(|| panic!("cc algo not found: {name}"));
        app_limit_cc_test_one(ccalgo, max_time);
    }
}

/// C: `cwin_max_test` in `picoquictest/congestion_test.c`.
///
/// Runs the CWIN-max cap test with NewReno, Cubic, dCubic, BBR, FastCC, and BBR1.
#[test]
fn cwin_max() {
    let algo_names = ["newreno", "cubic", "dcubic", "bbr", "fastcc", "bbr1"];
    let max_completion_times: [u64; 6] = [
        11_000_000, 11_000_000, 11_000_000, 11_000_000, 12_100_000, 11_000_000,
    ];

    for (name, &max_time) in algo_names.iter().zip(max_completion_times.iter()) {
        let ccalgo =
            get_congestion_algorithm(name).unwrap_or_else(|| panic!("cc algo not found: {name}"));
        cwin_max_test_one(ccalgo, 68_000, max_time);
    }
}
