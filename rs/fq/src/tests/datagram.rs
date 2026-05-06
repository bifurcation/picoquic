//! Test cases for `picoquictest/datagram_tests.c`.
//!
//! Exercises datagram send/receive over a simulated QUIC connection.
//! Tests cover the basic path, real-time delivery with latency targets,
//! skip behaviour, loss accounting, size limits, small-datagram batching,
//! packet-per-datagram mode, wifi-spike recovery, and the too-long API.

#![allow(non_snake_case)]

use crate::MAX_PACKET_SIZE;

// ---------------------------------------------------------------------------
// Shared context type.  C: `test_datagram_send_recv_ctx_t`.

/// Per-test state for the datagram send/receive callbacks.
/// C: `test_datagram_send_recv_ctx_t` in `picoquictest/datagram_tests.c`.
#[derive(Default)]
#[allow(dead_code)]
struct DatagramSendRecvCtx {
    /// Maximum datagram payload the client advertises.
    dg_max_size: usize,
    /// Number of datagrams to send in each direction [client, server].
    dg_target: [u64; 2],
    dg_sent: [u64; 2],
    dg_recv: [u64; 2],
    dg_acked: [u64; 2],
    dg_nacked: [u64; 2],
    dg_spurious: [u64; 2],
    dg_latency_max: [u64; 2],
    /// Maximum acceptable one-way latency (0 = unchecked).
    dg_latency_target: [u64; 2],
    dg_number_delta_max: [u64; 2],
    dg_number_delta_target: [u64; 2],
    dg_received_last: [u64; 2],
    dg_time_ready: [u64; 2],
    /// Time at which the next datagram generation is scheduled.
    next_gen_time: [u64; 2],
    /// Interval between consecutive datagrams (µs).
    send_delay: u64,
    is_ready: [bool; 2],
    is_skipping: [bool; 2],
    /// Enable the "skip one slot" test path.
    do_skip_test: [bool; 2],
    /// Use the extended `provide_datagram_buffer_ex` API.
    use_extended_provider_api: bool,
    /// Restrict to one datagram per QUIC packet.
    one_datagram_per_packet: bool,
    /// Bind datagrams to path 0 only.
    test_affinity: bool,
    /// Exercise the too-long datagram rejection path.
    test_too_long: bool,
    /// Exercise a wifi-style transmission suspension.
    test_wifi: bool,
    /// If non-zero, cap each datagram at this size.
    dg_small_size: usize,
    /// Batch multiple datagrams per generation tick (0 = no batching).
    batch_size: [u64; 2],
    /// Assert client received at most this many packets (0 = unchecked).
    max_packets_received: u64,
    /// Assert simulation ends by this time in µs (0 = unchecked).
    duration_max: u64,
    /// Override the default trial limit (0 = use 2048).
    nb_trials_max: i32,
    /// Override default link latency in µs (0 = keep default).
    link_latency: u64,
    /// Override link data rate in ps/byte (0 = keep default).
    picosec_per_byte: u64,
}

/// Run one datagram scenario end-to-end.
/// C: `datagram_test_one` in `picoquictest/datagram_tests.c`.
fn datagram_test_one(_test_id: u8, _dg_ctx: &mut DatagramSendRecvCtx, _loss_mask_init: u64) {
    todo!("datagram_test_one: simulation loop not yet implemented")
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `datagram_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [5, 5],
        ..Default::default()
    };
    datagram_test_one(1, &mut dg_ctx, 0);
}

/// C: `datagram_rt_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_rt() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [18_000, 18_000],
        ..Default::default()
    };
    datagram_test_one(2, &mut dg_ctx, 0);
}

/// C: `datagram_rt_skip_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_rt_skip() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 10],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [13_000, 20_000],
        do_skip_test: [true, true],
        ..Default::default()
    };
    datagram_test_one(3, &mut dg_ctx, 0);
}

/// C: `datagram_rtnew_skip_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_rtnew_skip() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 10],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [13_000, 20_000],
        do_skip_test: [true, true],
        use_extended_provider_api: true,
        ..Default::default()
    };
    datagram_test_one(7, &mut dg_ctx, 0);
}

/// C: `datagram_loss_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_loss() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        ..Default::default()
    };
    datagram_test_one(4, &mut dg_ctx, 0x040080100200400);
}

/// C: `datagram_size_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_size() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_target: [100, 100],
        send_delay: 5_000,
        ..Default::default()
    };
    datagram_test_one(5, &mut dg_ctx, 0);
}

/// C: `datagram_small_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_small() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        ..Default::default()
    };
    datagram_test_one(6, &mut dg_ctx, 0);
}

/// C: `datagram_small_new_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_small_new() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        use_extended_provider_api: true,
        ..Default::default()
    };
    datagram_test_one(7, &mut dg_ctx, 0);
}

/// C: `datagram_small_packet_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_small_packet() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        dg_target: [100, 20_000],
        send_delay: 100,
        next_gen_time: [50_000, 50_000],
        link_latency: 10_000,
        picosec_per_byte: 20_000, // 400 Mbps
        dg_latency_target: [20_000, 13_500],
        use_extended_provider_api: true,
        one_datagram_per_packet: true,
        nb_trials_max: 200_000,
        duration_max: 2_060_000,
        ..Default::default()
    };
    datagram_test_one(9, &mut dg_ctx, 0);
}

/// C: `datagram_too_long_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_too_long_test() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [5, 5],
        test_too_long: true,
        ..Default::default()
    };
    datagram_test_one(10, &mut dg_ctx, 0);
}

/// C: `datagram_wifi_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_wifi() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [1_000, 1_000],
        send_delay: 2_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [305_000, 280_000],
        test_wifi: true,
        nb_trials_max: 64_000,
        link_latency: 25_000,
        ..Default::default()
    };
    datagram_test_one(8, &mut dg_ctx, 0);
}
