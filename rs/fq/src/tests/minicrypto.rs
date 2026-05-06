//! Test cases for `picoquictest/minicrypto_test.c`.
//!
//! Tests the minicrypto backend (picotls minicrypto provider):
//! * `minicrypto` — TLS handshake using only the minicrypto backend.
//! * `minicrypto_is_last` — Verify minicrypto is the active fallback cipher/key backend.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, test_api_init_send_recv_scenario, tls_api_connection_loop,
    tls_api_data_sending_loop, tls_api_init_ctx_ex2, tls_api_one_scenario_body_verify,
};
use crate::internal::Version;
use crate::{
    Instant, TLS_API_INIT_FLAGS_NO_OPENSSL, is_minicrypto_aes128gcm_sha256,
    is_minicrypto_key_loader, reset_tls_api,
};

/// Small 2 KB stream used to verify basic minicrypto connectivity.
/// C: `test_scenario_minicrypto[]`.
const TEST_SCENARIO_MINICRYPTO: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 2000,
    r_len: 2000,
}];

/// Full TLS handshake + data transfer using only the minicrypto backend.
/// C: `minicrypto_test`.
#[test]
fn minicrypto() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;
    let target_time: u64 = 1_000_000;

    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL);

    let initial_cid =
        crate::ConnectionId::clone_from_slice(&[0x81, 0x81, 0xc8, 0x19, 0x40, 0, 6, 7])
            .expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 20_000, &mut simulated_time)
        .expect("connection loop");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MINICRYPTO)
        .expect("init scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
        .expect("scenario verify");

    reset_tls_api(0);
}

/// Verify that minicrypto is the active (last-resort) cipher and key backend.
/// C: `minicrypto_is_last_test`.
///
/// Expected values depend on which TLS backends are compiled in.  When only
/// minicrypto is available (no `sys-openssl`, `sys-mbedtls`, or `sys-fusion`),
/// all three checks return `true`.
#[test]
fn minicrypto_is_last() {
    reset_tls_api(0);

    let using_high = is_minicrypto_aes128gcm_sha256(false);
    let using_low = is_minicrypto_aes128gcm_sha256(true);
    let using_key = is_minicrypto_key_loader();

    let expected_high = !cfg!(any(
        feature = "sys-mbedtls",
        feature = "sys-openssl",
        feature = "sys-fusion",
    ));
    let expected_low = !cfg!(any(feature = "sys-mbedtls", feature = "sys-openssl"));
    let expected_key = !cfg!(any(
        feature = "sys-mbedtls",
        feature = "sys-openssl",
        feature = "sys-fusion",
    ));

    assert_eq!(
        using_high, expected_high,
        "aes128gcm_sha256 (high-memory): expected minicrypto={expected_high}, got {using_high}",
    );
    assert_eq!(
        using_low, expected_low,
        "aes128gcm_sha256 (low-memory): expected minicrypto={expected_low}, got {using_low}",
    );
    assert_eq!(
        using_key, expected_key,
        "key loader: expected minicrypto={expected_key}, got {using_key}",
    );
}
