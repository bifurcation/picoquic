//! Test cases for `picoquictest/delay_tolerant_test.c`.
//!
//! Exercises QUIC in "delay-tolerant networking" (DTN) scenarios with
//! very high one-way latency (60 s for lunar-range; 20 min for deep space).
//! Tests verify that the stack can complete handshakes, transfer data, and
//! remain quiet (no spurious retransmissions) over these extreme RTTs.

#![allow(non_snake_case)]

use super::util::{TestApiStreamDesc, tls_api_one_scenario_body, tls_api_one_scenario_init_ex};
use crate::internal::{Version, init_transport_parameters};
use crate::{CongestionAlgorithm, Duration, get_congestion_algorithm};

// ---------------------------------------------------------------------------
// Shared spec type.  C: `dtn_test_spec_t`.

/// Configuration for one delay-tolerant networking test scenario.
/// C: `dtn_test_spec_t` in `picoquictest/delay_tolerant_test.c`.
#[allow(dead_code)]
struct DtnTestSpec {
    /// One-way link latency in µs.
    latency: u64,
    /// Maximum allowed simulation time in µs (0 = unconstrained).
    max_completion_time: u64,
    /// Congestion algorithm for both client and server.
    ccalgo: &'static CongestionAlgorithm,
    /// Stream scenario to run after the handshake.
    scenario: &'static [TestApiStreamDesc],
    /// Upload bandwidth in Mbps.
    mbps_up: u64,
    /// Download bandwidth in Mbps.
    mbps_down: u64,
    /// Override for initial flow-control credit (0 = use stack default).
    initial_flow_control_credit: u64,
    /// If non-zero, assert that total packet count ≤ this value.
    max_number_of_packets: u64,
    /// Inject packet losses if `true`.
    has_loss: bool,
}

/// Build a "basic" DTN spec at 60-second one-way latency over 10 Mbps.
/// C: `dtn_set_basic_test_spec`.
fn dtn_basic_spec() -> DtnTestSpec {
    const LATENCY: u64 = 60_000_000; // 60 seconds in µs
    crate::register_all_congestion_control_algorithms();
    DtnTestSpec {
        latency: LATENCY,
        max_completion_time: 8 * LATENCY,
        ccalgo: get_congestion_algorithm("newreno").expect("newreno algo"),
        scenario: DTN_SCENARIO_BASIC,
        mbps_up: 10,
        mbps_down: 10,
        initial_flow_control_credit: 0,
        max_number_of_packets: 0,
        has_loss: false,
    }
}

/// Run one DTN test end-to-end.
/// C: `dtn_test_one` in `picoquictest/delay_tolerant_test.c`.
fn dtn_test_one(test_id: u8, spec: &DtnTestSpec) {
    let mut simulated_time = crate::Instant::from_ticks(0);
    let picosec_per_byte_up = (1_000_000u64 * 8) / spec.mbps_up;
    let picosec_per_byte_down = (1_000_000u64 * 8) / spec.mbps_down;

    let mut client_parameters = crate::TransportParameters::default();
    init_transport_parameters(&mut client_parameters);
    client_parameters.enable_time_stamp = 3;
    client_parameters.max_idle_timeout = Duration::from_ticks((spec.latency * 5) / 1000);
    if spec.initial_flow_control_credit > client_parameters.initial_max_data {
        client_parameters.initial_max_data = spec.initial_flow_control_credit;
    }
    if spec.initial_flow_control_credit > client_parameters.initial_max_stream_data_bidi_local {
        client_parameters.initial_max_stream_data_bidi_local = spec.initial_flow_control_credit;
    }
    if spec.initial_flow_control_credit > client_parameters.initial_max_stream_data_bidi_remote {
        client_parameters.initial_max_stream_data_bidi_remote = spec.initial_flow_control_credit;
    }

    let mut server_parameters = crate::TransportParameters::default();
    init_transport_parameters(&mut server_parameters);
    server_parameters.enable_time_stamp = 3;
    server_parameters.max_idle_timeout = client_parameters.max_idle_timeout;

    let mut initial_cid =
        crate::ConnectionId::clone_from_slice(&[0xde, 0x40, 0, 0, 0, 0, 0, 0]).expect("8-byte CID");
    initial_cid.as_bytes_mut()[2] = test_id;

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        Some(&client_parameters),
        Some(&server_parameters),
        Some(&initial_cid),
    )
    .expect("tls_api_one_scenario_init_ex");

    test_ctx
        .qserver
        .set_default_congestion_algorithm(spec.ccalgo);
    test_ctx.cnx_client().set_congestion_algorithm(spec.ccalgo);

    test_ctx.c_to_s_link.microsec_latency = spec.latency;
    test_ctx.c_to_s_link.picosec_per_byte = picosec_per_byte_up;
    test_ctx.s_to_c_link.microsec_latency = spec.latency;
    test_ctx.s_to_c_link.picosec_per_byte = picosec_per_byte_down;
    test_ctx.stream0_flow_release = true;
    test_ctx.immediate_exit = true;

    test_ctx.cnx_client().set_pmtud_required(true);

    test_ctx.qserver.set_qlog(".").expect("server qlog");
    test_ctx.qserver.set_log_level(1);
    test_ctx.qclient.set_qlog(".").expect("client qlog");
    test_ctx.qclient.set_log_level(1);

    let init_loss_mask = if spec.has_loss { 0x1000_0000 } else { 0 };
    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        spec.scenario,
        init_loss_mask,
        0,
        0,
        2 * spec.latency,
        spec.max_completion_time,
    )
    .expect("DTN scenario");

    if spec.max_number_of_packets != 0 {
        let cnx = test_ctx.cnx_client();
        let number_of_packets = cnx.nb_packets_sent + cnx.nb_packets_received;
        assert!(
            number_of_packets <= spec.max_number_of_packets,
            "expected at most {} packets, got {}",
            spec.max_number_of_packets,
            number_of_packets
        );
    }
}

// ---------------------------------------------------------------------------
// Static scenarios.  C: file-scope arrays in `delay_tolerant_test.c`.

static DTN_SCENARIO_BASIC: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 2_000,
}];

static DTN_SCENARIO_DATA: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 100_000_000,
}];

static DTN_SCENARIO_SILENCE: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 257,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 4,
        q_len: 257,
        r_len: 257,
    },
    TestApiStreamDesc {
        stream_id: 12,
        previous_stream_id: 8,
        q_len: 257,
        r_len: 257,
    },
];

// ---------------------------------------------------------------------------
// Test entries.

/// C: `dtn_basic_test` in `picoquictest/delay_tolerant_test.c`.
#[test]
fn dtn_basic() {
    let mut spec = dtn_basic_spec();
    spec.max_number_of_packets = 120;
    dtn_test_one(0xba, &spec);
}

/// C: `dtn_data_test` in `picoquictest/delay_tolerant_test.c`.
#[test]
fn dtn_data() {
    let mut spec = dtn_basic_spec();
    spec.scenario = DTN_SCENARIO_DATA;
    spec.initial_flow_control_credit = 100_000_000; // 100 MB
    spec.max_completion_time = 500_000_000; // ~8 min 20 sec
    dtn_test_one(0xda, &spec);
}

/// C: `dtn_silence_test` in `picoquictest/delay_tolerant_test.c`.
#[test]
fn dtn_silence() {
    let mut spec = dtn_basic_spec();
    spec.scenario = DTN_SCENARIO_SILENCE;
    spec.max_number_of_packets = 120;
    spec.max_completion_time = 481_000_000; // 8 min: 2 handshake + 2 per tx
    dtn_test_one(0x51, &spec);
}

/// C: `dtn_twenty_test` in `picoquictest/delay_tolerant_test.c`.
#[test]
fn dtn_twenty() {
    let mut spec = dtn_basic_spec();
    spec.latency = 20 * 60_000_000; // 20 minutes in µs
    spec.max_completion_time = 8 * spec.latency;
    spec.max_number_of_packets = 190;
    dtn_test_one(0x20, &spec);
}
