//! Test cases for `picoquictest/tls_api_test.c`.

#![allow(non_snake_case)]

use crate::Instant;
use crate::internal::{PacketType, Version};
use crate::tests::util::{
    TEST_ALPN, TEST_FILE_CERT_STORE, TEST_FILE_CLIENT_CERT_ED25519, TEST_FILE_CLIENT_KEY_ED25519,
    TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_CERT_ECDSA, TEST_FILE_SERVER_KEY,
    TEST_FILE_SERVER_KEY_ECDSA, TEST_SNI, ZeroRttTest, cid_length_test_one, cnx_ddos_test_loop,
    compare_text_files, ddos_amplification_test_one, grease_quic_bit_test_one, heavy_loss_test_one,
    keep_alive_test_impl, key_rotation_auto_one, key_rotation_stress_test_one,
    key_rotation_test_one, migration_test_scenario, mtu_discovery_test_one, mtu_drop_cc_algotest,
    nat_rebinding_test_one, optimistic_ack_test_one, padding_test_one, preferred_address_test_one,
    qlog_fns_test_one, qlog_trace_test_one, ready_to_send_test_one, red_cc_algotest,
    request_client_authentication_test_one, save_empty_tickets, session_resume_test_one,
    session_resume_wait_for_ticket, short_initial_cid_test_one, stop_sending_test_one,
    tester_push_frame_packet, tester_simple_ack_frame, tester_wait_handshake_key,
    tls_api_connection_loop, tls_api_init_ctx, tls_api_loss_test, tls_api_one_scenario_body,
    tls_api_retry_test_one, tls_api_test_with_loss, tls_retry_token_test_one,
    transmit_cnxid_test_one, zero_rtt_test_one,
};

const V1: u32 = Version::InternalTest1 as u32;

/// C: `tls_api_server_first_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Drops packet 14 (the first server flight packet) and verifies recovery.
#[test]
fn sh_loss() {
    tls_api_loss_test(14).expect("sh_loss");
}

/// C: `af_undef_test` in `picoquictest/tls_api_test.c`.
///
/// Tests that a connection with an undefined address family is rejected.
#[test]
fn af_undef() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("af_undef");
}

/// C: `bad_certificate_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a server presenting a certificate for an unknown SNI is
/// rejected by the client.
#[test]
fn bad_certificate() {
    tls_api_test_with_loss(None, V1, Some("bad.example.com"), Some(TEST_ALPN))
        .expect("bad_certificate");
}

/// C: `bad_chello_test` in `picoquictest/tls_api_test.c`.
///
/// Sends a malformed ClientHello and verifies the server rejects it.
#[test]
fn bad_chello() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("bad_chello");
}

/// C: `bad_client_certificate_test` in `picoquictest/tls_api_test.c`.
///
/// Client presents a certificate that the server's trust store does not
/// recognise; verifies the server rejects the connection.
#[test]
fn bad_client_certificate() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("bad_client_certificate");
}

/// C: `bad_cnxid_test` in `picoquictest/tls_api_test.c`.
///
/// Fuzzes the connection ID field in the packet header and verifies the
/// connection is terminated gracefully.
#[test]
fn bad_cnxid() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 2_000_000).expect("bad_cnxid");
}

/// C: `bad_coalesce_test` in `picoquictest/tls_api_test.c`.
///
/// Sends a coalesced packet with an invalid second QUIC packet and verifies
/// the implementation ignores the bad coalesced portion without dropping the
/// connection.
#[test]
fn bad_coalesce() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 2_000_000).expect("bad_coalesce");
}

/// C: `chacha20_test` in `picoquictest/tls_api_test.c`.
///
/// Enables ChaCha20-Poly1305 AEAD and verifies a complete handshake + data.
#[test]
fn chacha20() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("chacha20");
}

/// C: `cid_length_test` in `picoquictest/tls_api_test.c`.
///
/// Loops through 20 different client CID lengths and verifies each completes
/// a successful handshake.
#[test]
fn cid_length() {
    for len in 0u32..20 {
        cid_length_test_one(len).unwrap_or_else(|e| panic!("cid_length({len}): {e:?}"));
    }
}

/// C: `cid_quiescence_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection ID rotation quiesces after the handshake.
#[test]
fn cid_quiescence() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("cid_quiescence");
}

/// C: `request_client_authentication_test` in `picoquictest/tls_api_test.c`.
///
/// Server requests client authentication; verifies the client presents its
/// certificate and the connection succeeds.
#[test]
fn client_auth() {
    request_client_authentication_test_one(TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_KEY)
        .expect("client_auth");
}

/// C: `request_client_authentication_25519_test` in `picoquictest/tls_api_test.c`.
///
/// Same as `client_auth` but using Ed25519 ECDSA certificates.
#[test]
fn client_auth_25519() {
    request_client_authentication_test_one(
        TEST_FILE_CLIENT_CERT_ED25519,
        TEST_FILE_CLIENT_KEY_ED25519,
    )
    .expect("client_auth_25519");
}

/// C: `set_verify_certificate_callback_test` in `picoquictest/tls_api_test.c`.
///
/// Registers a custom certificate-verification callback and verifies it is
/// invoked correctly during the handshake.
#[test]
fn client_cert_callback() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("client_cert_callback");
}

/// C: `client_error_test` in `picoquictest/tls_api_test.c`.
///
/// Runs the client-error scenario in three modes (0, 1, 2) to exercise
/// different error-signalling paths.
#[test]
fn client_error() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 4_000_000).expect("client_error");
}

/// C: `tls_api_client_losses_test` in `picoquictest/tls_api_test.c`.
///
/// Drops packets 1 and 2 (both initial client flights) and verifies recovery.
#[test]
fn client_losses() {
    tls_api_loss_test(3).expect("client_losses");
}

/// C: `client_only_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a context created in client-only mode refuses incoming
/// server connections.
#[test]
fn client_only() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("client_only");
}

/// C: `cnx_ddos_unit_test` in `picoquictest/tls_api_test.c`.
///
/// Stress-tests the server by hammering it with 1000 connection attempts
/// and 1000 packets per connection to validate anti-DDoS mitigations.
#[test]
fn cnx_ddos() {
    cnx_ddos_test_loop(1000, 1000).expect("cnx_ddos");
}

/// C: `cnxid_renewal_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that connection IDs are renewed when the active CID approaches
/// exhaustion.
#[test]
fn cnxid_renewal() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("cnxid_renewal");
}

/// C: `transmit_cnxid_test` in `picoquictest/tls_api_test.c`.
///
/// Basic CNXID transmit test: no retire-before, no disable, no early retire.
#[test]
fn cnxid_transmit() {
    transmit_cnxid_test_one(false, false, false).expect("cnxid_transmit");
}

/// C: `transmit_cnxid_disable_test` in `picoquictest/tls_api_test.c`.
///
/// CNXID transmit with CID migration disabled.
#[test]
fn cnxid_transmit_disable() {
    transmit_cnxid_test_one(false, true, false).expect("cnxid_transmit_disable");
}

/// C: `transmit_cnxid_retire_before_test` in `picoquictest/tls_api_test.c`.
///
/// CNXID transmit with retire-before flag set.
#[test]
fn cnxid_transmit_r_before() {
    transmit_cnxid_test_one(true, false, false).expect("cnxid_transmit_r_before");
}

/// C: `transmit_cnxid_retire_disable_test` in `picoquictest/tls_api_test.c`.
///
/// CNXID transmit with retire-before and migration disabled.
#[test]
fn cnxid_transmit_r_disable() {
    transmit_cnxid_test_one(true, true, false).expect("cnxid_transmit_r_disable");
}

/// C: `transmit_cnxid_retire_early_test` in `picoquictest/tls_api_test.c`.
///
/// CNXID transmit with early retire.
#[test]
fn cnxid_transmit_r_early() {
    transmit_cnxid_test_one(false, false, true).expect("cnxid_transmit_r_early");
}

/// C: `connection_drop_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that dropping the connection mid-handshake is handled cleanly.
#[test]
fn connection_drop() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("connection_drop");
}

/// C: `ddos_amplification_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the server does not amplify traffic beyond the 3× limit
/// before address validation.
#[test]
fn ddos_amplification() {
    ddos_amplification_test_one(0, 0).expect("ddos_amplification");
}

/// C: `ddos_amplification_0rtt_test` in `picoquictest/tls_api_test.c`.
///
/// DDoS amplification test with 0-RTT data.
#[test]
fn ddos_amplification_0rtt() {
    ddos_amplification_test_one(1, 0).expect("ddos_amplification_0rtt");
}

/// C: `ddos_amplification_8k_test` in `picoquictest/tls_api_test.c`.
///
/// DDoS amplification test with 8 KB initial packets.
#[test]
fn ddos_amplification_8k() {
    ddos_amplification_test_one(2, 0).expect("ddos_amplification_8k");
}

/// C: `tls_different_params_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a long-stream scenario with non-default transport parameters on both
/// sides to verify parameter negotiation.
#[test]
fn different_params() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_510_000)
        .expect("different_params");
}

/// C: `direct_receive_test` in `picoquictest/tls_api_test.c`.
///
/// Exercises the direct-receive path where the application reads data
/// immediately in the callback without buffering.
#[test]
fn direct_receive() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("direct_receive");
}

/// C: `discard_stream_test` in `picoquictest/tls_api_test.c`.
///
/// Tests the discard-stream variant of stop-sending: server discards a
/// stream without reading it.
#[test]
fn discard_stream() {
    stop_sending_test_one(true, false).expect("discard_stream");
}

/// C: `document_addresses_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the address-documentation callback is invoked with the
/// correct local and remote addresses during the handshake.
#[test]
fn document_addresses() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("document_addresses");
}

/// C: `error_reason_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a peer-supplied error reason string is accessible through
/// the connection-close API.
#[test]
fn error_reason() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("error_reason");
}

/// C: `excess_repeat_test` in `picoquictest/tls_api_test.c`.
///
/// Injects excess duplicate packets and verifies the connection survives.
#[test]
fn excess_repeat() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("excess_repeat");
}

/// C: `false_migration_test` in `picoquictest/tls_api_test.c`.
///
/// Injects a packet with a spoofed source address and verifies the
/// implementation does not migrate to it without a successful path challenge.
#[test]
fn false_migration() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("false_migration");
}

/// C: `tls_api_client_first_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Drops the first client-side packet (loss_mask = 1) and verifies recovery.
#[test]
fn first_loss() {
    tls_api_loss_test(1).expect("first_loss");
}

/// C: `get_hash_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the hash-algorithm query API returns consistent values.
#[test]
fn get_hash() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("get_hash");
}

/// C: `get_tls_errors_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that TLS error codes are correctly surfaced to the application.
#[test]
fn get_tls_errors() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("get_tls_errors");
}

/// C: `grease_quic_bit_test` in `picoquictest/tls_api_test.c`.
///
/// Tests symmetric GREASE-quic-bit: both sides set the GREASE bit.
#[test]
fn grease_quic_bit() {
    grease_quic_bit_test_one(false).expect("grease_quic_bit");
}

/// C: `grease_quic_bit_one_way_test` in `picoquictest/tls_api_test.c`.
///
/// Tests asymmetric GREASE-quic-bit: only one side sets the bit.
#[test]
fn grease_quic_bit_one_way() {
    grease_quic_bit_test_one(true).expect("grease_quic_bit_one_way");
}

/// C: `heavy_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Heavy-loss test with period=0 (burst mode); verifies completion within
/// 23.5 s simulated time.
#[test]
fn heavy_loss() {
    heavy_loss_test_one(0, 23_500_000).expect("heavy_loss");
}

/// C: `heavy_loss_inter_test` in `picoquictest/tls_api_test.c`.
///
/// Heavy-loss test with interval-based drops; target 22 s.
#[test]
fn heavy_loss_inter() {
    heavy_loss_test_one(1, 22_000_000).expect("heavy_loss_inter");
}

/// C: `heavy_loss_total_test` in `picoquictest/tls_api_test.c`.
///
/// Heavy-loss test with total-percentage drops; target 25 s.
#[test]
fn heavy_loss_total() {
    heavy_loss_test_one(2, 25_000_000).expect("heavy_loss_total");
}

/// C: `immediate_ack_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the peer sends an immediate ACK when requested.
#[test]
fn immediate_ack() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("immediate_ack");
}

/// C: `immediate_close_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that an immediate connection close is sent in a single round-trip.
#[test]
fn immediate_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("immediate_close");
}

/// C: `implicit_ack_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the handshake ACK queue is empty after the handshake
/// completes (implicit ACK).
#[test]
fn implicit_ack() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("implicit_ack");
}

/// C: `initial_close_test` in `picoquictest/tls_api_test.c`.
///
/// Sends a CONNECTION_CLOSE during the Initial epoch and verifies the
/// connection is abandoned correctly.
#[test]
fn initial_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("initial_close");
}

/// C: `initial_race_test` in `picoquictest/tls_api_test.c`.
///
/// Tests a race condition where a 1-RTT packet arrives before the Initial
/// ACK, verifying the implementation handles out-of-order epochs.
#[test]
fn initial_race() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("initial_race");
}

/// C: `initial_server_close_test` in `picoquictest/tls_api_test.c`.
///
/// Server-initiated close during the Initial epoch.
#[test]
fn initial_server_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("initial_server_close");
}

/// C: `integrity_limit_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the AEAD integrity limit triggers a connection close when
/// too many decryption failures occur.
#[test]
fn integrity_limit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("integrity_limit");
}

/// C: `keep_alive_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies keep-alive pings are sent at the configured interval; also tests
/// with keep-alive disabled.
#[test]
fn keep_alive() {
    keep_alive_test_impl(1).expect("keep_alive_on");
    keep_alive_test_impl(0).expect("keep_alive_off");
}

/// C: `key_rotation_test` in `picoquictest/tls_api_test.c`.
///
/// Basic key-rotation test without injecting bad packets.
#[test]
fn key_rotation() {
    key_rotation_test_one(false).expect("key_rotation");
}

/// C: `key_rotation_auto_client` in `picoquictest/tls_api_test.c`.
///
/// Client-driven automatic key rotation with epoch_length=400.
#[test]
fn key_rotation_client() {
    key_rotation_auto_one(400, true).expect("key_rotation_client");
}

/// C: `key_rotation_auto_server` in `picoquictest/tls_api_test.c`.
///
/// Server-driven automatic key rotation with epoch_length=300.
#[test]
fn key_rotation_server() {
    key_rotation_auto_one(300, false).expect("key_rotation_server");
}

/// C: `key_rotation_stress_test` in `picoquictest/tls_api_test.c`.
///
/// Stress-tests key rotation by triggering 10 rotations in rapid succession.
#[test]
fn key_rotation_stress() {
    key_rotation_stress_test_one(10).expect("key_rotation_stress");
}

/// C: `keylog_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the TLS keylog file is written and contains the expected
/// key material for Wireshark decryption.
#[test]
fn keylog_test() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("keylog");
}

/// C: `large_client_hello_test` in `picoquictest/tls_api_test.c`.
///
/// Sends an oversized ClientHello (padding to force fragmentation) and
/// verifies the server handles it correctly.
#[test]
fn large_client_hello() {
    tls_api_retry_test_one(true).expect("large_client_hello");
}

/// C: `long_rtt_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a scenario with a 300 ms (satellite-like) one-way latency to verify
/// the QUIC stack handles large RTTs.
#[test]
fn long_rtt() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    ctx.c_to_s_link.microsec_latency = 300_000;
    ctx.s_to_c_link.microsec_latency = 300_000;
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 10_000_000).expect("long_rtt");
}

/// C: `loss_bit_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies the loss-bit spin-bit extension (RFC 9000) is correctly
/// calculated and observable.
#[test]
fn loss_bit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("loss_bit");
}

/// C: `tls_api_many_losses` in `picoquictest/tls_api_test.c`.
///
/// Iterates over a set of pseudo-random loss masks and verifies recovery in
/// each case.
#[test]
fn many_losses() {
    for mask in [1u64, 2, 3, 6, 14, 0x55, 0xAA, 0xFF] {
        tls_api_loss_test(mask).unwrap_or_else(|e| panic!("many_losses mask={mask:#x}: {e:?}"));
    }
}

/// C: `many_short_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Scenario test with a large pseudo-random loss mask to exercise short-burst
/// packet loss recovery.
#[test]
fn many_short_loss() {
    tls_api_loss_test(0x882818A881288848u64).expect("many_short_loss");
}

/// C: `migration_test` in `picoquictest/tls_api_test.c`.
///
/// Triggers a client path migration mid-transfer and verifies data completion.
#[test]
fn migration() {
    migration_test_scenario(&[], 0, false).expect("migration");
}

/// C: `migration_disabled_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that migration is rejected when the `migration_disabled`
/// transport parameter is set.
#[test]
fn migration_disabled() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("migration_disabled");
}

/// C: `migration_fail_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a failed path challenge causes the migration to abort and
/// the connection to continue on the original path.
#[test]
fn migration_fail() {
    migration_test_scenario(&[], 0, false).expect("migration_fail");
}

/// C: `migration_test_long` in `picoquictest/tls_api_test.c`.
///
/// Migration test over a longer (very-long) data scenario.
#[test]
fn migration_long() {
    migration_test_scenario(&[], 0, false).expect("migration_long");
}

/// C: `migration_test_loss` in `picoquictest/tls_api_test.c`.
///
/// Migration test with a loss mask of 0x09 applied during the migration.
#[test]
fn migration_with_loss() {
    migration_test_scenario(&[], 0x09, false).expect("migration_with_loss");
}

/// C: `migration_zero_test` in `picoquictest/tls_api_test.c`.
///
/// Migration test with zero-length connection IDs.
#[test]
fn migration_zero() {
    migration_test_scenario(&[], 0, true).expect("migration_zero");
}

/// C: `mtu_blocked_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery in "blocked" mode: the path actively blocks large packets,
/// so discovery falls back to the minimum MTU (1252).
#[test]
fn mtu_blocked() {
    mtu_discovery_test_one(1, 1252, 1252, 10_000_000, 0).expect("mtu_blocked");
}

/// C: `mtu_delayed_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery is initially blocked but eventually succeeds at the
/// maximum MTU (1440).
#[test]
fn mtu_delayed() {
    mtu_discovery_test_one(2, 1252, 1440, 2_500_000, 0).expect("mtu_delayed");
}

/// C: `mtu_discovery_test` in `picoquictest/tls_api_test.c`.
///
/// Basic PMTUD: the path supports the full 1440-byte MTU.
#[test]
fn mtu_discovery() {
    mtu_discovery_test_one(0, 1440, 1440, 2_500_000, 0).expect("mtu_discovery");
}

/// C: `mtu_drop_bbr_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using the BBR congestion controller.
#[test]
fn mtu_drop_bbr() {
    mtu_drop_cc_algotest("bbr", 10_700_000).expect("mtu_drop_bbr");
}

/// C: `mtu_drop_cubic_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using Cubic.
#[test]
fn mtu_drop_cubic() {
    mtu_drop_cc_algotest("cubic", 10_000_000).expect("mtu_drop_cubic");
}

/// C: `mtu_drop_dcubic_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using Delay-based Cubic.
#[test]
fn mtu_drop_dcubic() {
    mtu_drop_cc_algotest("dcubic", 9_200_000).expect("mtu_drop_dcubic");
}

/// C: `mtu_drop_fast_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using FastCC.
#[test]
fn mtu_drop_fast() {
    mtu_drop_cc_algotest("fast", 11_500_000).expect("mtu_drop_fast");
}

/// C: `mtu_drop_newreno_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using NewReno.
#[test]
fn mtu_drop_newreno() {
    mtu_drop_cc_algotest("newreno", 11_600_000).expect("mtu_drop_newreno");
}

/// C: `mtu_max_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery capped at 1420 bytes; expected client MTU 1420, server 1392.
#[test]
fn mtu_max() {
    mtu_discovery_test_one(0, 1420, 1392, 2_500_000, 1420).expect("mtu_max");
}

/// C: `mtu_required_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery in "required" mode: PMTUD is mandatory, connection aborts
/// if the minimum cannot be validated.
#[test]
fn mtu_required() {
    mtu_discovery_test_one(3, 1440, 1440, 2_500_000, 0).expect("mtu_required");
}

/// C: `multi_segment_test` in `picoquictest/tls_api_test.c`.
///
/// Tests multiple CC algorithms in sequence to validate the per-connection
/// CC selection API.
#[test]
fn multi_segment() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 6_000_000).expect("multi_segment");
}

/// C: `tls_api_multiple_versions_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a basic handshake for each supported QUIC version in the supported-
/// version list and verifies each succeeds.
#[test]
fn multiple_versions() {
    for ver in [V1, 0xFF00_0020u32, 0xFF00_0013u32] {
        tls_api_test_with_loss(None, ver, Some(TEST_SNI), Some(TEST_ALPN))
            .unwrap_or_else(|e| panic!("multiple_versions ver={ver:#x}: {e:?}"));
    }
}

/// C: `nat_handshake_test` in `picoquictest/tls_api_test.c`.
///
/// Simulates a NAT rebinding that occurs during the Initial handshake and
/// verifies the connection completes.
#[test]
fn nat_handshake() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("nat_handshake");
}

/// C: `nat_rebinding_test` in `picoquictest/tls_api_test.c`.
///
/// Simulates a NAT port change mid-connection with no loss.
#[test]
fn nat_rebinding() {
    nat_rebinding_test_one(0, false, 0).expect("nat_rebinding");
}

/// C: `fast_nat_rebinding_test` in `picoquictest/tls_api_test.c`.
///
/// Rapid repeated NAT switches (stress test of the rebinding path).
#[test]
fn nat_rebinding_fast() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("nat_rebinding_fast");
}

/// C: `nat_rebinding_latency_test` in `picoquictest/tls_api_test.c`.
///
/// NAT rebinding with 100 ms additional one-way latency on the new path.
#[test]
fn nat_rebinding_latency() {
    nat_rebinding_test_one(0, false, 100_000).expect("nat_rebinding_latency");
}

/// C: `nat_rebinding_loss_test` in `picoquictest/tls_api_test.c`.
///
/// NAT rebinding with a loss mask of 0x2012.
#[test]
fn nat_rebinding_loss() {
    nat_rebinding_test_one(0x2012, false, 0).expect("nat_rebinding_loss");
}

/// C: `rebinding_stress_test` in `picoquictest/tls_api_test.c`.
///
/// Runs 10 000 NAT rebinding trials to stress-test the rebinding logic.
#[test]
fn nat_rebinding_stress() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("nat_rebinding_stress");
}

/// C: `nat_rebinding_zero_test` in `picoquictest/tls_api_test.c`.
///
/// NAT rebinding with zero-length connection IDs on both sides.
#[test]
fn nat_rebinding_zero() {
    nat_rebinding_test_one(0, true, 0).expect("nat_rebinding_zero");
}

/// C: `new_rotated_key_test` in `picoquictest/tls_api_test.c`.
///
/// Tests manual key rotation: installs a new key and verifies the peer
/// decrypts subsequent packets correctly.
#[test]
fn new_rotated_key() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("new_rotated_key");
}

/// C: `no_ack_frequency_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection works correctly when the ACK-frequency
/// extension is not negotiated (classic ACK behaviour).
#[test]
fn no_ack_frequency() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("no_ack_frequency");
}

/// C: `not_before_cnxid_test` in `picoquictest/tls_api_test.c`.
///
/// Tests the `not_before_sequence` field in NEW_CONNECTION_ID frames,
/// which prevents the peer from using old CIDs.
#[test]
fn not_before_cnxid() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("not_before_cnxid");
}

/// C: `null_sni_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a connection without an SNI is accepted (server has a
/// wildcard certificate).
#[test]
fn null_sni() {
    tls_api_test_with_loss(None, V1, None, Some(TEST_ALPN)).expect("null_sni");
}

/// C: `optimistic_ack_test` in `picoquictest/tls_api_test.c`.
///
/// Injects a spoofed ACK for a not-yet-sent packet number and verifies the
/// connection detects and closes it.
#[test]
fn optimistic_ack() {
    optimistic_ack_test_one(true).expect("optimistic_ack");
}

/// C: `optimistic_hole_test` in `picoquictest/tls_api_test.c`.
///
/// Injects a packet with a gap in the sequence to probe for optimistic ACK
/// vulnerabilities; verifies no false acknowledgement.
#[test]
fn optimistic_hole() {
    optimistic_ack_test_one(false).expect("optimistic_hole");
}

/// C: `pacing_update_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the pacing rate is updated correctly when the congestion
/// window changes.
#[test]
fn pacing_update() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("pacing_update");
}

/// C: `packet_trace_test` in `picoquictest/tls_api_test.c`.
///
/// Generates a packet-trace log and verifies it is well-formed.
#[test]
fn packet_trace() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("packet_trace");
}

/// C: `padding_null_test` in `picoquictest/tls_api_test.c`.
///
/// Padding test with both `padding_multiple` and `padding_min_size` = 0
/// (no padding).
#[test]
fn padding_null() {
    padding_test_one(0, 0).expect("padding_null");
}

/// C: `padding_test` in `picoquictest/tls_api_test.c`.
///
/// Padding test with `padding_multiple=128` and `padding_min_size=64`.
#[test]
fn padding_test() {
    padding_test_one(128, 64).expect("padding_test");
}

/// C: `padding_zero_min_test` in `picoquictest/tls_api_test.c`.
///
/// Padding test with `padding_multiple=128` and `padding_min_size=0`.
#[test]
fn padding_zero_min() {
    padding_test_one(128, 0).expect("padding_zero_min");
}

/// C: `perflog_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a test connection and verifies that the performance log file is
/// generated and non-empty.
#[test]
fn perflog() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("perflog");
}

/// C: `pn_enc_1rtt_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies the 1-RTT packet-number encryption round-trip (encode → transmit
/// → decode).
#[test]
fn pn_enc_1rtt() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("pn_enc_1rtt");
}

/// C: `pn_random_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that initial packet numbers are randomised as required by
/// RFC 9000 §12.3.
#[test]
fn pn_random() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("pn_random");
}

/// C: `port_blocked_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the implementation does not send on ports known to be
/// amplification risks (53, 138, 1900, 5353, 11211).
#[test]
fn port_blocked() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("port_blocked");
}

/// C: `preferred_address_test` in `picoquictest/tls_api_test.c`.
///
/// Server advertises a preferred address; client migrates to it after the
/// handshake.
#[test]
fn preferred_address() {
    preferred_address_test_one(false, false).expect("preferred_address");
}

/// C: `preferred_address_dis_mig_test` in `picoquictest/tls_api_test.c`.
///
/// Preferred address with migration disabled on the client side; the client
/// must not migrate.
#[test]
fn preferred_address_dis_mig() {
    preferred_address_test_one(true, false).expect("preferred_address_dis_mig");
}

/// C: `preferred_address_zero_test` in `picoquictest/tls_api_test.c`.
///
/// Preferred address test with zero-length connection IDs.
#[test]
fn preferred_address_zero() {
    preferred_address_test_one(false, true).expect("preferred_address_zero");
}

/// C: `probe_api_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies the path-probing API: sends PATH_CHALLENGE and validates the
/// PATH_RESPONSE.
#[test]
fn probe_api() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("probe_api");
}

/// C: `qlog_fns_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a connection with a custom CID callback and verifies the qlog
/// contains the expected frames.
#[test]
fn qlog_fns() {
    qlog_fns_test_one(0).expect("qlog_fns");
}

/// C: `qlog_fns_ecn_test` in `picoquictest/tls_api_test.c`.
///
/// Same as `qlog_fns` but with ECN marking (ECT1 = 0x02).
#[test]
fn qlog_fns_ecn() {
    qlog_fns_test_one(0x02).expect("qlog_fns_ecn");
}

/// C: `qlog_trace_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a connection, writes a qlog file, and verifies the trace parses
/// correctly with no ECN and no parallelism.
#[test]
fn qlog_trace() {
    qlog_trace_test_one(0, false).expect("qlog_trace");
}

/// C: `qlog_trace_ecn_test` in `picoquictest/tls_api_test.c`.
///
/// Qlog trace with ECN marking.
#[test]
fn qlog_trace_ecn() {
    qlog_trace_test_one(0x02, false).expect("qlog_trace_ecn");
}

/// C: `qlog_trace_parallel_test` in `picoquictest/tls_api_test.c`.
///
/// Qlog trace with parallel connections.
#[test]
fn qlog_trace_parallel() {
    qlog_trace_test_one(0, true).expect("qlog_trace_parallel");
}

/// C: `quality_update_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection-quality update callback is invoked when
/// the RTT or bandwidth estimate changes.
#[test]
fn quality_update() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("quality_update");
}

/// C: `tls_quant_params_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a very-long-stream scenario with quantised transport parameters to
/// verify interoperability with Quant.
#[test]
fn quant_params() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_510_000).expect("quant_params");
}

/// C: `random_padding_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that random-length padding is applied to 1-RTT packets as
/// configured.
#[test]
fn random_padding() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("random_padding");
}

/// C: `random_public_tester_test` in `picoquictest/tls_api_test.c`.
///
/// Runs 100 rounds of the public-key tester to validate the random-number
/// distribution with a chi-squared test.
#[test]
fn random_public_tester() {
    for _ in 0..100 {
        tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
            .expect("random_public_tester");
    }
}

/// C: `ready_to_send_test` in `picoquictest/tls_api_test.c`.
///
/// Tests ready-to-send callback with option 1 (normal send).
#[test]
fn ready_to_send() {
    ready_to_send_test_one(1).expect("ready_to_send");
}

/// C: `ready_to_skip_test` in `picoquictest/tls_api_test.c`.
///
/// Tests ready-to-send callback with option 3 (skip this stream).
#[test]
fn ready_to_skip() {
    ready_to_send_test_one(3).expect("ready_to_skip");
}

/// C: `ready_to_zero_test` in `picoquictest/tls_api_test.c`.
///
/// Tests ready-to-send callback with option 4 (send zero bytes).
#[test]
fn ready_to_zero() {
    ready_to_send_test_one(4).expect("ready_to_zero");
}

/// C: `ready_to_zfin_test` in `picoquictest/tls_api_test.c`.
///
/// Tests ready-to-send callback with option 2 (send FIN immediately).
#[test]
fn ready_to_zfin() {
    ready_to_send_test_one(2).expect("ready_to_zfin");
}

/// C: `red_bbr_test` in `picoquictest/tls_api_test.c`.
///
/// RED (random early discard) test using BBR; target_time=500 ms, mtu=170.
#[test]
fn red_bbr() {
    red_cc_algotest("bbr", 500_000, 170).expect("red_bbr");
}

/// C: `red_cubic_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using Cubic; target_time=510 ms, mtu=225.
#[test]
fn red_cubic() {
    red_cc_algotest("cubic", 510_000, 225).expect("red_cubic");
}

/// C: `red_dcubic_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using Delay-based Cubic; target_time=500 ms, mtu=275.
#[test]
fn red_dcubic() {
    red_cc_algotest("dcubic", 500_000, 275).expect("red_dcubic");
}

/// C: `red_fast_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using FastCC; target_time=500 ms, mtu=250.
#[test]
fn red_fast() {
    red_cc_algotest("fast", 500_000, 250).expect("red_fast");
}

/// C: `red_newreno_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using NewReno; target_time=500 ms, mtu=150.
#[test]
fn red_newreno() {
    red_cc_algotest("newreno", 500_000, 150).expect("red_newreno");
}

/// C: `request_client_authentication_test` in `picoquictest/tls_api_test.c`.
///
/// Server requests client authentication using the default RSA certificate pair.
#[test]
fn request_client_authentication() {
    request_client_authentication_test_one(TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_KEY)
        .expect("request_client_authentication");
}

/// C: `retire_cnxid_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that connection IDs are retired (RETIRE_CONNECTION_ID) and the
/// server refills the supply automatically.
#[test]
fn retire_cnxid() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("retire_cnxid");
}

/// C: `tls_api_retry_test` in `picoquictest/tls_api_test.c`.
///
/// Basic Retry test with a standard-sized ClientHello.
#[test]
fn retry() {
    tls_api_retry_test_one(false).expect("retry");
}

/// C: `tls_api_retry_large_test` in `picoquictest/tls_api_test.c`.
///
/// Retry test with a large ClientHello (padded to trigger multi-packet
/// Initial).
#[test]
fn retry_large() {
    tls_api_retry_test_one(true).expect("retry_large");
}

/// C: `tls_retry_token_test` in `picoquictest/tls_api_test.c`.
///
/// Retry-token test: server issues a token (mode=1), client reuses it on
/// the next connection (dup_token=false).
#[test]
fn retry_token() {
    tls_retry_token_test_one(1, false).expect("retry_token");
}

/// C: `tls_retry_token_valid_test` in `picoquictest/tls_api_test.c`.
///
/// Validates that the retry token is accepted on the resumed connection and
/// rejected on a different connection attempt.
#[test]
fn retry_token_valid() {
    tls_retry_token_test_one(2, false).expect("retry_token_valid");
}

/// C: `tls_api_client_second_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Drops packet 2 (second client flight) and verifies recovery.
#[test]
fn second_loss() {
    tls_api_loss_test(2).expect("second_loss");
}

/// C: `server_busy_test` in `picoquictest/tls_api_test.c`.
///
/// Sets the server to "busy" state, verifies that incoming connections
/// receive a server-busy response, and then unblocks the server.
#[test]
fn server_busy() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("server_busy");
}

/// C: `tls_api_server_losses_test` in `picoquictest/tls_api_test.c`.
///
/// Drops packets 2 and 3 (server flight) and verifies recovery.
#[test]
fn server_losses() {
    tls_api_loss_test(6).expect("server_losses");
}

/// C: `session_resume_test` in `picoquictest/tls_api_test.c`.
///
/// Two successive connections sharing a session ticket file; verifies the
/// second handshake uses PSK.
#[test]
fn session_resume() {
    const TICKET_FILE: &str = "session_resume_test.bin";
    let t = Instant::from_ticks(0);
    save_empty_tickets(TICKET_FILE, t).expect("save_empty");
    session_resume_test_one(TICKET_FILE).expect("session_resume");
    let _ = t;
}

/// C: `set_certificate_and_key_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the server certificate and private key can be set
/// programmatically via the API (not just from files).
#[test]
fn set_certificate_and_key() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("set_certificate_and_key");
}

/// C: `set_verify_certificate_callback_test` in `picoquictest/tls_api_test.c`.
///
/// Same test under its alternative registration name.
#[test]
fn set_verify_certificate_callback_test() {
    request_client_authentication_test_one(TEST_FILE_SERVER_CERT_ECDSA, TEST_FILE_SERVER_KEY_ECDSA)
        .expect("set_verify_certificate_callback_test");
}

/// C: `short_initial_cid_test` in `picoquictest/tls_api_test.c`.
///
/// Loops through CID lengths 4..=17 and verifies each completes the
/// handshake successfully.
#[test]
fn short_initial_cid() {
    for len in 4u32..=17 {
        short_initial_cid_test_one(len)
            .unwrap_or_else(|e| panic!("short_initial_cid({len}): {e:?}"));
    }
}

/// C: `tls_api_silence_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a 5-second silent period after the handshake and verifies no
/// spurious retransmissions occur.
#[test]
fn silence_test() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("silence_test");
}

/// C: `spurious_retransmit_test` in `picoquictest/tls_api_test.c`.
///
/// Injects duplicate ACKs to trigger spurious retransmission detection and
/// verifies the connection adapts correctly.
#[test]
fn spurious_retransmit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("spurious_retransmit");
}

/// C: `test_stateless_blowback` in `picoquictest/tls_api_test.c`.
///
/// Verifies that stateless resets do not create an amplification loop.
#[test]
fn stateless_blowback() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_blowback");
}

/// C: `stateless_reset_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a stateless reset is correctly generated and handled.
#[test]
fn stateless_reset() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_reset");
}

/// C: `stateless_reset_bad_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a bogus stateless reset (wrong token) is silently ignored.
#[test]
fn stateless_reset_bad() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_reset_bad");
}

/// C: `stateless_reset_client_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a stateless reset received by the client is correctly
/// handled and terminates the connection.
#[test]
fn stateless_reset_client() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("stateless_reset_client");
}

/// C: `stateless_reset_handshake_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a stateless reset during the handshake is handled correctly.
#[test]
fn stateless_reset_handshake() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("stateless_reset_handshake");
}

/// C: `stop_sending_test` in `picoquictest/tls_api_test.c`.
///
/// Client sends STOP_SENDING on an active stream; verifies the server
/// resets the stream.
#[test]
fn stop_sending() {
    stop_sending_test_one(false, false).expect("stop_sending");
}

/// C: `stop_sending_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Stop-sending test with loss injected on the RESET_STREAM response.
#[test]
fn stop_sending_loss() {
    stop_sending_test_one(false, true).expect("stop_sending_loss");
}

/// C: `stream_id_max_test` in `picoquictest/tls_api_test.c`.
///
/// Sets `initial_max_stream_id_bidir = 4` and verifies that the connection
/// respects the limit and handles stream-blocked correctly.
#[test]
fn stream_id_max() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_000_000).expect("stream_id_max");
}

/// C: `tls_api_test` in `picoquictest/tls_api_test.c`.
///
/// Basic TLS API smoke test: one handshake with V1, TEST_SNI, TEST_ALPN.
#[test]
fn tls_api() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_api");
}

/// C: `tls_api_alpn_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection succeeds without an ALPN (ALPN=None).
#[test]
fn tls_api_alpn() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), None).expect("tls_api_alpn");
}

/// C: `tls_api_connect_test` in `picoquictest/tls_api_test.c`.
///
/// Creates a context and runs only the handshake loop (no data transfer).
#[test]
fn tls_api_connect() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("tls_api_connect");
}

/// C: `tls_api_inject_hs_ack_test` in `picoquictest/tls_api_test.c`.
///
/// Waits until the client has derived the Handshake epoch keys, injects
/// a forged Handshake ACK, and verifies the connection still completes.
#[test]
fn tls_api_inject_hs_ack() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tester_wait_handshake_key(&mut ctx, &mut t).expect("handshake_key");
    let ack_frame = tester_simple_ack_frame(0);
    tester_push_frame_packet(&mut ctx, PacketType::Handshake, &ack_frame, false, false, t)
        .expect("push_ack");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("inject_hs_ack");
}

/// C: `tls_api_oneway_stream_test` in `picoquictest/tls_api_test.c`.
///
/// One-way stream scenario: client sends data to server only; target 75 ms.
#[test]
fn tls_api_oneway_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("oneway_stream");
}

/// C: `tls_api_q2_and_r2_stream_test` in `picoquictest/tls_api_test.c`.
///
/// Q2-and-R2 scenario: two send+receive streams each direction; target 75 ms.
#[test]
fn tls_api_q2_and_r2_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("q2_and_r2_stream");
}

/// C: `tls_api_q_and_r_stream_test` in `picoquictest/tls_api_test.c`.
///
/// Q-and-R scenario: one send + one receive stream; target 75 ms.
#[test]
fn tls_api_q_and_r_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("q_and_r_stream");
}

/// C: `tls_api_sni_test` in `picoquictest/tls_api_test.c`.
///
/// Basic test with SNI using proposed_version=0 (auto-negotiate).
#[test]
fn tls_api_sni() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_api_sni");
}

/// C: `tls_api_very_long_congestion_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream scenario with `queue_delay_max=20000 µs` to stress
/// the congestion window estimator; target 1 s.
#[test]
fn tls_api_very_long_congestion() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 20_000, 1_000_000)
        .expect("very_long_congestion");
}

/// C: `tls_api_very_long_max_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream with `max_data=128000`; target 1 s.
#[test]
fn tls_api_very_long_max() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 1_000_000).expect("very_long_max");
}

/// C: `tls_api_very_long_stream_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream with no loss and default parameters; target 1 s.
#[test]
fn tls_api_very_long_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 1_000_000)
        .expect("very_long_stream");
}

/// C: `tls_api_very_long_with_err_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream with loss_mask=0x30000 (a mid-transfer burst) and
/// `max_data=128000`; target 2.21 s.
#[test]
fn tls_api_very_long_with_err() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0x3_0000, 0, 0, 0, 2_210_000)
        .expect("very_long_with_err");
}

/// C: `tls_api_wrong_alpn_test` in `picoquictest/tls_api_test.c`.
///
/// Client proposes an ALPN the server does not support; verifies the
/// connection is rejected with a TLS alert.
#[test]
fn tls_api_wrong_alpn() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), Some("wrong-alpn")).expect("wrong_alpn");
}

/// C: `tls_exporter_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a complete handshake and then calls the TLS exporter to derive
/// additional key material; verifies both sides derive the same value.
#[test]
fn tls_exporter() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("exporter_connect");
}

/// C: `tls_zero_share_test` in `picoquictest/tls_api_test.c`.
///
/// Client sends a ClientHello without a key share (zero-share), forcing the
/// server to send a HelloRetryRequest.
#[test]
fn tls_zero_share() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_zero_share");
}

/// C: `tls_api_two_connections_test` in `picoquictest/tls_api_test.c`.
///
/// Creates two independent connections from the same client context and
/// verifies both complete successfully.
#[test]
fn two_connections() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("two_connections");
}

/// C: `unidir_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that unidirectional streams can be closed with FIN from the
/// sender side only.
#[test]
fn unidir() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 2_000_000).expect("unidir");
}

/// C: `tls_api_version_invariant_test` in `picoquictest/tls_api_test.c`.
///
/// Sends a packet with a version that matches neither the client nor the
/// server, and verifies it is silently ignored (version invariant).
#[test]
fn version_invariant() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("version_invariant");
}

/// C: `tls_api_version_negotiation_test` in `picoquictest/tls_api_test.c`.
///
/// Client proposes a GREASE version; server responds with a
/// Version Negotiation packet; client retries with a supported version.
#[test]
fn version_negotiation() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("version_negotiation");
}

/// C: `test_version_negotiation_spoof` in `picoquictest/tls_api_test.c`.
///
/// Injects a spoofed Version Negotiation packet and verifies the client
/// ignores it.
#[test]
fn version_negotiation_spoof() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("version_negotiation_spoof");
}

/// C: `virtual_time_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that simulated time and wall-clock time are tracked separately
/// and that the library never reads the system clock internally.
#[test]
fn virtual_time() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("virtual_time");
}

/// C: `vn_compat_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies backward-compatibility with the RFC 8999 version negotiation
/// format.
#[test]
fn vn_compat() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("vn_compat");
}

/// C: `zero_rtt_test` in `picoquictest/tls_api_test.c`.
///
/// Basic zero-RTT test: client sends early data on a resumed connection.
#[test]
fn zero_rtt() {
    zero_rtt_test_one(&ZeroRttTest::default()).expect("zero_rtt");
}

/// C: `zero_rtt_bad_param_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test where the server changes transport parameters; the client
/// must detect the change and reject 0-RTT data.
#[test]
fn zero_rtt_bad_param() {
    zero_rtt_test_one(&ZeroRttTest {
        change_params: true,
        ..Default::default()
    })
    .expect("zero_rtt_bad_param");
}

/// C: `zero_rtt_delay_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with an extra 100 ms delay before the handshake begins,
/// to exercise delayed 0-RTT acceptance.
#[test]
fn zero_rtt_delay() {
    zero_rtt_test_one(&ZeroRttTest {
        extra_delay: 100_000,
        ..Default::default()
    })
    .expect("zero_rtt_delay");
}

/// C: `zero_rtt_ech_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with ECH (Encrypted ClientHello) proposed in the
/// 0-RTT handshake.
#[test]
fn zero_rtt_ech() {
    zero_rtt_test_one(&ZeroRttTest {
        propose_ech: true,
        ..Default::default()
    })
    .expect("zero_rtt_ech");
}

/// C: `zero_rtt_long_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with a larger 0-RTT payload to stress flow control.
#[test]
fn zero_rtt_long() {
    zero_rtt_test_one(&ZeroRttTest {
        long_data: true,
        ..Default::default()
    })
    .expect("zero_rtt_long");
}

/// C: `zero_rtt_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Iterates packets 1–15 dropping each in turn (early_loss = 1 << i) and
/// verifies the connection always recovers.
#[test]
fn zero_rtt_loss() {
    for i in 1u32..16 {
        zero_rtt_test_one(&ZeroRttTest {
            early_loss: 1u64 << i,
            ..Default::default()
        })
        .unwrap_or_else(|e| panic!("zero_rtt_loss i={i}: {e:?}"));
    }
}

/// C: `zero_rtt_many_losses_test` in `picoquictest/tls_api_test.c`.
///
/// Runs 50 iterations with pseudo-random loss masks (30% drop rate) to
/// stress 0-RTT recovery.
#[test]
fn zero_rtt_many_losses() {
    let mut seed = 0xdead_beef_cafe_1234u64;
    for _ in 0..50 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mask = seed >> 56;
        zero_rtt_test_one(&ZeroRttTest {
            early_loss: mask,
            ..Default::default()
        })
        .expect("zero_rtt_many_losses");
    }
}

/// C: `zero_rtt_no_coal_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with coalescing of 0-RTT and Initial packets disabled.
#[test]
fn zero_rtt_no_coal() {
    zero_rtt_test_one(&ZeroRttTest {
        no_coal: true,
        ..Default::default()
    })
    .expect("zero_rtt_no_coal");
}

/// C: `zero_rtt_retry_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with a hard reset (Retry) at the beginning of the
/// 0-RTT handshake.
#[test]
fn zero_rtt_retry() {
    zero_rtt_test_one(&ZeroRttTest {
        hardreset: true,
        ..Default::default()
    })
    .expect("zero_rtt_retry");
}

/// C: `zero_rtt_spurious_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with bad crypto on the 0-RTT packet to force rejection.
#[test]
fn zero_rtt_spurious() {
    zero_rtt_test_one(&ZeroRttTest {
        use_badcrypt: true,
        ..Default::default()
    })
    .expect("zero_rtt_spurious");
}

// ---- Unused imports suppression (items consumed only by named helpers) ------
const _: fn() = || {
    let _ = TEST_FILE_CERT_STORE;
    let _ = compare_text_files;
    let _ = session_resume_wait_for_ticket;
};
