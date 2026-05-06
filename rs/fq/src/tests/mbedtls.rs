//! Test cases for `picoquictest/mbedtls_test.c`.
//!
//! Tests the mbedTLS crypto provider:
//!
//! * `mbedtls` — TLS handshake + data transfer using only the mbedTLS backend.
//! * `mbedtls_crypto` — Raw hash / cipher / AEAD / key-exchange round-trips.
//! * `mbedtls_load_key` — Loading private keys from PEM files.
//! * `mbedtls_load_key_fail` — Expected failures when key loading is invalid.
//! * `mbedtls_retrieve_pubkey` — Public-key extraction from DER certificates.
//! * `mbedtls_sign_verify` — Sign/verify end-to-end with mbedTLS.
//! * `mbedtls_configure` — Verifies that mbedTLS properly registers cipher
//!   suites, key-exchange algorithms, and function pointers.
//!
//! All tests require the mbedTLS backend (`PICOQUIC_WITH_MBEDTLS`).  Stubs
//! will panic at `todo!()` until `crate::sys::mbedtls` is implemented.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, test_api_init_send_recv_scenario, tls_api_connection_loop,
    tls_api_data_sending_loop, tls_api_init_ctx_ex2, tls_api_one_scenario_body_verify,
};
use crate::internal::Version;
use crate::{Instant, TLS_API_INIT_FLAGS_NO_FUSION, TLS_API_INIT_FLAGS_NO_OPENSSL, reset_tls_api};

/// Small 2 KB stream used to verify basic mbedTLS connectivity.
/// C: `test_scenario_mbedtls[]`.
const TEST_SCENARIO_MBEDTLS: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 2000,
    r_len: 2000,
}];

// ---------------------------------------------------------------------------
// Helpers for the mbedTLS-specific sub-tests.
// Each wraps the C static helper of the same name; bodies are `todo!()`
// until `crate::sys::mbedtls` provides the low-level picotls wrappers.

/// Initialise the PSA/mbedTLS library and verify random-byte generation.
/// C: local `test_random()` inside `mbedtls_crypto_test`.
fn mbedtls_test_random() -> crate::Result<()> {
    todo!("mbedtls_test_random: requires crate::sys::mbedtls::random_bytes")
}

/// Test the mbedTLS SHA-256 hash against the minicrypto reference.
/// C: local `test_hash(ptls_mbedtls_sha256, ptls_minicrypto_sha256)`.
fn mbedtls_test_hash() -> crate::Result<()> {
    todo!("mbedtls_test_hash: requires crate::sys::mbedtls hash algorithm wrappers")
}

/// Test HKDF label expansion with the mbedTLS SHA-256 hash.
/// C: local `test_label(ptls_mbedtls_sha256, ptls_minicrypto_sha256)`.
fn mbedtls_test_label() -> crate::Result<()> {
    todo!("mbedtls_test_label: requires crate::sys::mbedtls hash algorithm wrappers")
}

/// Test all five mbedTLS ciphers against minicrypto reference ciphers.
/// C: local `test_cipher(cipher_test[i], cipher_ref[i])` for i in 0..5.
fn mbedtls_test_ciphers() -> crate::Result<()> {
    todo!("mbedtls_test_ciphers: requires crate::sys::mbedtls cipher algorithm wrappers")
}

/// Test AES-128-GCM AEAD encrypt/decrypt with mbedTLS vs minicrypto.
/// C: local `test_aead(ptls_mbedtls_aes128gcm, ptls_mbedtls_sha256, ...)`.
fn mbedtls_test_aead() -> crate::Result<()> {
    todo!("mbedtls_test_aead: requires crate::sys::mbedtls AEAD algorithm wrappers")
}

/// Test ECDH key exchange (secp256r1 and x25519) mixing mbedTLS and minicrypto.
/// C: local `test_key_exchange(...)` combinations.
fn mbedtls_test_key_exchange() -> crate::Result<()> {
    todo!("mbedtls_test_key_exchange: requires crate::sys::mbedtls key-exchange wrappers")
}

/// Load one private key from a DER PEM file and sign a message with it.
/// C: `mbedtls_test_load_one_der_key(path_ref)`.
fn mbedtls_test_load_one_der_key(_path_ref: &str) -> crate::Result<()> {
    todo!(
        "mbedtls_test_load_one_der_key: requires ptls_mbedtls_load_private_key / sign_certificate"
    )
}

/// Check that loading each of the expected failure cases fails.
/// C: the negative assertions in `mbedtls_load_key_fail_test`.
fn mbedtls_test_load_key_fail_cases() -> crate::Result<()> {
    todo!("mbedtls_test_load_key_fail_cases: requires ptls_mbedtls_load_private_key error paths")
}

/// Extract the public key from a certificate and compare to the private key.
/// C: `test_retrieve_pubkey_one(key_path_ref, cert_path_ref)`.
fn mbedtls_test_retrieve_pubkey_one(
    _key_path_ref: &str,
    _cert_path_ref: &str,
) -> crate::Result<()> {
    todo!("mbedtls_test_retrieve_pubkey_one: requires ptls_mbedtls_get_public_key_info")
}

/// Sign with mbedTLS and verify the signature end-to-end.
/// C: `test_sign_verify_one(key, cert, ca, server_name, config, config)`.
fn mbedtls_test_sign_verify_one(
    _key_path_ref: &str,
    _cert_path_ref: &str,
    _ca_path_ref: &str,
    _server_name: &str,
) -> crate::Result<()> {
    todo!(
        "mbedtls_test_sign_verify_one: requires ptls_mbedtls sign_certificate / verify_certificate"
    )
}

// ---------------------------------------------------------------------------
// Public test entries.

/// Full TLS handshake + data transfer using only the mbedTLS backend.
/// C: `mbedtls_test`.
#[test]
fn mbedtls() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;
    let target_time: u64 = 1_000_000;

    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL | TLS_API_INIT_FLAGS_NO_FUSION);

    let initial_cid = crate::ConnectionId::clone_from_slice(&[0x99, 0xbe, 0xd7, 0x15, 0, 0, 0, 0])
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
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MBEDTLS).expect("init scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
        .expect("scenario verify");

    reset_tls_api(0);
}

/// Raw hash / cipher / AEAD / key-exchange round-trips with mbedTLS.
/// C: `mbedtls_crypto_test` (inside `#ifdef PICOQUIC_WITH_MBEDTLS`).
#[test]
fn mbedtls_crypto() {
    // Initialise the PSA/mbedTLS library.
    mbedtls_test_random().expect("test_random");
    mbedtls_test_hash().expect("test_hash");
    mbedtls_test_label().expect("test_label");
    mbedtls_test_ciphers().expect("test_ciphers");
    mbedtls_test_aead().expect("test_aead");
    mbedtls_test_key_exchange().expect("test_key_exchange");
    // PSA/mbedTLS library is de-initialised by the helper internals.
}

/// Load RSA, secp256r1, secp384r1, secp521r1, and PKCS8 private keys.
/// C: `mbedtls_load_key_test` (inside `#ifdef PICOQUIC_WITH_MBEDTLS`).
#[test]
fn mbedtls_load_key() {
    mbedtls_test_load_one_der_key("certs/rsa/key.pem").expect("rsa key");
    mbedtls_test_load_one_der_key("certs/secp256r1/key.pem").expect("secp256r1 key");
    mbedtls_test_load_one_der_key("certs/secp384r1/key.pem").expect("secp384r1 key");
    mbedtls_test_load_one_der_key("certs/secp521r1/key.pem").expect("secp521r1 key");
    mbedtls_test_load_one_der_key("certs/secp256r1-pkcs8/key.pem").expect("secp256r1 pkcs8 key");
    mbedtls_test_load_one_der_key("certs/rsa-pkcs8/key.pem").expect("rsa pkcs8 key");
}

/// Verify that loading invalid keys fails as expected.
/// C: `mbedtls_load_key_fail_test` (inside `#ifdef PICOQUIC_WITH_MBEDTLS`).
#[test]
fn mbedtls_load_key_fail() {
    mbedtls_test_load_key_fail_cases().expect("load_key_fail_cases");
}

/// Extract public keys from certificates and compare against private keys.
/// C: `mbedtls_retrieve_pubkey_test` (inside `#ifdef PICOQUIC_WITH_MBEDTLS`).
#[test]
fn mbedtls_retrieve_pubkey() {
    mbedtls_test_retrieve_pubkey_one("certs/rsa/key.pem", "certs/rsa/cert.pem")
        .expect("rsa pubkey");
    mbedtls_test_retrieve_pubkey_one("certs/secp256r1/key.pem", "certs/secp256r1/cert.pem")
        .expect("secp256r1 pubkey");
    mbedtls_test_retrieve_pubkey_one("certs/secp384r1/key.pem", "certs/secp384r1/cert.pem")
        .expect("secp384r1 pubkey");
    mbedtls_test_retrieve_pubkey_one("certs/secp521r1/key.pem", "certs/secp521r1/cert.pem")
        .expect("secp521r1 pubkey");
}

/// End-to-end signature and verification with mbedTLS.
/// C: `mbedtls_sign_verify_test` (inside `#ifdef PICOQUIC_WITH_MBEDTLS`).
#[test]
fn mbedtls_sign_verify() {
    mbedtls_test_sign_verify_one(
        "certs/rsa/key.pem",
        "certs/rsa/cert.pem",
        "certs/test-ca.crt",
        "rsa.test.example.com",
    )
    .expect("rsa sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1/key.pem",
        "certs/secp256r1/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp384r1/key.pem",
        "certs/secp384r1/cert.pem",
        "certs/test-ca.crt",
        "secp384r1.test.example.com",
    )
    .expect("secp384r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp521r1/key.pem",
        "certs/secp521r1/cert.pem",
        "certs/test-ca.crt",
        "secp521r1.test.example.com",
    )
    .expect("secp521r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1-pkcs8/key.pem",
        "certs/secp256r1-pkcs8/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1-pkcs8 sign_verify");
}

/// Verify that mbedTLS properly registers its cipher suites, key-exchange
/// algorithms, and function pointers with the picoquic TLS provider system.
/// C: `mbedtls_configure_test` (inside `#ifdef PICOQUIC_WITH_MBEDTLS`).
#[test]
fn mbedtls_configure() {
    // Bring up the TLS provider registry with only the mbedTLS backend.
    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL | TLS_API_INIT_FLAGS_NO_FUSION);

    // Verify that AES-128-GCM, AES-256-GCM, and ChaCha20-Poly1305 are
    // registered via the mbedTLS provider (high- and low-memory variants).
    // Verify secp256r1 and x25519 key exchange via mbedTLS.
    // Verify that private-key, cert-verifier, and random function pointers
    // all point to the mbedTLS implementations.
    //
    // These checks require `crate::sys::mbedtls` provider introspection APIs
    // that are not yet implemented.
    todo!("mbedtls_configure: requires cipher suite / key exchange / fn-ptr introspection APIs")
}
