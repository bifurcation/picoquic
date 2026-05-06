//! Test cases for `picoquictest/quic_tester.c`.

#![allow(non_snake_case)]

use super::util::{
    TestTlsApiCtx, tester_push_frame_packet, tester_simple_ack_frame, tester_wait_handshake_key,
    tls_api_connection_loop, tls_api_init_ctx_ex, tls_api_test_with_loss_final,
};
use crate::internal::{PacketType, Version};
use crate::{ConnectionId, Instant};

const PICOQUIC_TEST_SNI: &str = "test.example.com";
const PICOQUIC_TEST_ALPN: &str = "picoquic_test";

fn delete_ctx(ctx: Option<Box<TestTlsApiCtx>>) {
    drop(ctx);
}

/// C: `initial_ping_test` in `picoquictest/quic_tester.c`.
#[test]
fn initial_ping() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;

    let initial_cid =
        ConnectionId::clone_from_slice(&[0x4e, 0x54, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11])
            .expect("build initial CID");

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.set_qlog(".").ok();

    let ping_frame: &[u8] = &[1];
    tester_push_frame_packet(
        &mut test_ctx,
        PacketType::Initial,
        ping_frame,
        true,
        false,
        simulated_time,
    )
    .expect("push ping packet");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    tls_api_test_with_loss_final(
        &mut test_ctx,
        PICOQUIC_TEST_SNI,
        PICOQUIC_TEST_ALPN,
        &mut simulated_time,
    )
    .expect("test with loss final");

    delete_ctx(Some(test_ctx));
}

/// C: `initial_ping_ack_test` in `picoquictest/quic_tester.c`.
#[test]
fn initial_ping_ack() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;

    let initial_cid =
        ConnectionId::clone_from_slice(&[0x4e, 0x54, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12])
            .expect("build initial CID");

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.s_to_c_link.microsec_latency = 1;
    test_ctx.c_to_s_link.microsec_latency = 1;

    let ping_frame: &[u8] = &[1];
    tester_push_frame_packet(
        &mut test_ctx,
        PacketType::Initial,
        ping_frame,
        true,
        true,
        simulated_time,
    )
    .expect("push queued ping packet");

    tester_wait_handshake_key(&mut test_ctx, &mut simulated_time).expect("wait for handshake key");

    let ack_frame = tester_simple_ack_frame(1);
    tester_push_frame_packet(
        &mut test_ctx,
        PacketType::Initial,
        &ack_frame,
        true,
        false,
        simulated_time,
    )
    .expect("push initial ACK packet");

    tester_push_frame_packet(
        &mut test_ctx,
        PacketType::Handshake,
        &ack_frame,
        false,
        false,
        simulated_time,
    )
    .expect("push handshake ACK packet");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    tls_api_test_with_loss_final(
        &mut test_ctx,
        PICOQUIC_TEST_SNI,
        PICOQUIC_TEST_ALPN,
        &mut simulated_time,
    )
    .expect("test with loss final");

    delete_ctx(Some(test_ctx));
}
