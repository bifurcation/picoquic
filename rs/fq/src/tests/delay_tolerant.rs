//! Test cases for `picoquictest/delay_tolerant_test.c`.
//!
//! Exercises QUIC in "delay-tolerant networking" (DTN) scenarios with
//! very high one-way latency (60 s for lunar-range; 20 min for deep space).
//! Tests verify that the stack can complete handshakes, transfer data, and
//! remain quiet (no spurious retransmissions) over these extreme RTTs.

#![allow(non_snake_case)]

use super::util::TestApiStreamDesc;
use crate::{CongestionAlgorithm, get_congestion_algorithm};

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
fn dtn_test_one(_test_id: u8, _spec: &DtnTestSpec) {
    todo!("dtn_test_one: simulation loop not yet implemented")
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
