//! Tests for `picoquictest/cert_verify_test.c`.
//!
//! Verifies that the client correctly validates server TLS certificates:
//! correct cert + CA, bad cert, wrong SNI, no CA list, and no SNI.

use crate::Instant;

use super::util::{
    TEST_BAD_SNI, TEST_FILE_CERT_STORE, TEST_FILE_SERVER_BAD_CERT, TEST_FILE_SERVER_CERT,
    TEST_FILE_SERVER_KEY, TEST_SNI, cert_verify_set_ctx, tls_api_connection_loop,
};

/// Drive the handshake and assert success or failure.
/// C: `cert_verify_test_one` in `picoquictest/cert_verify_test.c`.
fn cert_verify_test_one(
    expect_success: bool,
    cert_file: Option<&str>,
    key_file: Option<&str>,
    root_certs_file: Option<&str>,
    sni: Option<&str>,
) {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = cert_verify_set_ctx(
        &mut simulated_time,
        cert_file,
        key_file,
        root_certs_file,
        sni,
    )
    .expect("cert_verify_set_ctx");
    let mut loss_mask: u64 = 0;
    let result = tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time);
    if expect_success {
        assert!(result.is_ok(), "expected TLS handshake to succeed");
    } else {
        assert!(result.is_err(), "expected TLS handshake to fail");
    }
}

/// C: `cert_verify_bad_cert_test` in `picoquictest/cert_verify_test.c`.
#[test]
fn cert_verify_bad_cert() {
    cert_verify_test_one(
        false,
        Some(TEST_FILE_SERVER_BAD_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_SNI),
    );
}

/// C: `cert_verify_bad_sni_test` in `picoquictest/cert_verify_test.c`.
#[test]
fn cert_verify_bad_sni() {
    cert_verify_test_one(
        false,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_BAD_SNI),
    );
}

/// C: `cert_verify_null_test` in `picoquictest/cert_verify_test.c`.
/// No CA list — verification falls back to SNI matching only.
#[test]
fn cert_verify_null() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        None,
        Some(TEST_SNI),
    );
}

/// C: `cert_verify_null_sni_test` in `picoquictest/cert_verify_test.c`.
/// No SNI — client signals it does not care about the server name.
#[test]
fn cert_verify_null_sni() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        None,
    );
}

/// C: `cert_verify_rsa_test` in `picoquictest/cert_verify_test.c`.
/// RSA cert with matching CA added to trusted list.
#[test]
fn cert_verify_rsa() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_SNI),
    );
}
