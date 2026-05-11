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
//! All tests mirror the C cases guarded by `PICOQUIC_WITH_MBEDTLS`.

#![allow(non_snake_case)]

use super::util::{
    TEST_ALPN, TEST_SNI, TestApiStreamDesc, test_api_init_send_recv_scenario,
    tls_api_connection_loop, tls_api_data_sending_loop, tls_api_init_ctx_ex2_ecdsa,
    tls_api_one_scenario_body_verify,
};
use crate::internal::Version;
use crate::tls_api::{
    TlsProviderKind, ptls_mbedtls_free, ptls_mbedtls_init, tls_provider_snapshot,
};
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
// Each wraps the C static helper of the same name.

fn fixture_path(path_ref: &str) -> std::path::PathBuf {
    let direct = std::path::PathBuf::from(path_ref);
    if direct.exists() {
        direct
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path_ref)
    }
}

struct TlsApiResetGuard;

impl TlsApiResetGuard {
    fn mbedtls_only() -> Self {
        reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL | TLS_API_INIT_FLAGS_NO_FUSION);
        Self
    }
}

impl Drop for TlsApiResetGuard {
    fn drop(&mut self) {
        reset_tls_api(0);
    }
}

struct MbedTlsRuntimeGuard;

impl MbedTlsRuntimeGuard {
    fn init() -> Self {
        ptls_mbedtls_init().expect("ptls_mbedtls_init");
        Self
    }
}

impl Drop for MbedTlsRuntimeGuard {
    fn drop(&mut self) {
        ptls_mbedtls_free();
    }
}

fn assert_mbedtls_provider_snapshot() {
    let snapshot = tls_provider_snapshot();
    for suite_id in [
        crate::AES_128_GCM_SHA256,
        crate::AES_256_GCM_SHA384,
        crate::CHACHA20_POLY1305_SHA256,
    ] {
        let suite = snapshot
            .cipher_suites
            .iter()
            .find(|suite| suite.id == suite_id)
            .unwrap_or_else(|| panic!("missing cipher suite {suite_id:#x}"));
        assert_eq!(suite.high_memory_suite, Some(TlsProviderKind::MbedTls));
        assert_eq!(suite.low_memory_suite, Some(TlsProviderKind::MbedTls));
    }

    assert_eq!(
        snapshot.key_exchange_secp256r1,
        Some(TlsProviderKind::MbedTls)
    );
    for group_id in [crate::GROUP_SECP256R1, 29u16] {
        let key_exchange = snapshot
            .key_exchanges
            .iter()
            .find(|key_exchange| key_exchange.id == group_id)
            .unwrap_or_else(|| panic!("missing key exchange {group_id:#x}"));
        assert_eq!(key_exchange.provider, Some(TlsProviderKind::MbedTls));
    }

    assert_eq!(
        snapshot.crypto_random_provider,
        Some(TlsProviderKind::MbedTls)
    );
    assert_eq!(snapshot.private_key_loader, Some(TlsProviderKind::MbedTls));
    assert_eq!(
        snapshot.sign_certificate_disposer,
        Some(TlsProviderKind::MbedTls)
    );
    assert_eq!(snapshot.cert_chain_loader, Some(TlsProviderKind::MbedTls));
    assert_eq!(snapshot.public_key_loader, Some(TlsProviderKind::MbedTls));
    assert_eq!(
        snapshot.verify_certificate_provider,
        Some(TlsProviderKind::MbedTls)
    );
}

fn sha256_bytes(parts: &[&[u8]]) -> [u8; 32] {
    use digest::Digest;

    let mut hasher = sha2::Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn aes128_ecb_crypt(block: &mut [u8; 16], key: &[u8; 16], encrypt: bool) {
    use cipher::{BlockDecrypt, BlockEncrypt, KeyInit};

    let cipher = aes::Aes128::new_from_slice(key).expect("AES-128 key length");
    let mut ga = cipher::Block::<aes::Aes128>::clone_from_slice(block);
    if encrypt {
        cipher.encrypt_block(&mut ga);
    } else {
        cipher.decrypt_block(&mut ga);
    }
    block.copy_from_slice(&ga);
}

fn aes256_ecb_crypt(block: &mut [u8; 16], key: &[u8; 32], encrypt: bool) {
    use cipher::{BlockDecrypt, BlockEncrypt, KeyInit};

    let cipher = aes::Aes256::new_from_slice(key).expect("AES-256 key length");
    let mut ga = cipher::Block::<aes::Aes256>::clone_from_slice(block);
    if encrypt {
        cipher.encrypt_block(&mut ga);
    } else {
        cipher.decrypt_block(&mut ga);
    }
    block.copy_from_slice(&ga);
}

fn aes128_ctr_apply(data: &mut [u8], key: &[u8; 16], iv: &[u8; 16]) {
    use cipher::{BlockEncrypt, KeyInit};

    let cipher = aes::Aes128::new_from_slice(key).expect("AES-128 key length");
    let mut counter = u128::from_be_bytes(*iv);
    for chunk in data.chunks_mut(16) {
        let counter_bytes = counter.to_be_bytes();
        let mut stream = cipher::Block::<aes::Aes128>::clone_from_slice(&counter_bytes);
        cipher.encrypt_block(&mut stream);
        for (b, k) in chunk.iter_mut().zip(stream.iter()) {
            *b ^= *k;
        }
        counter = counter.wrapping_add(1);
    }
}

fn aes256_ctr_apply(data: &mut [u8], key: &[u8; 32], iv: &[u8; 16]) {
    use cipher::{BlockEncrypt, KeyInit};

    let cipher = aes::Aes256::new_from_slice(key).expect("AES-256 key length");
    let mut counter = u128::from_be_bytes(*iv);
    for chunk in data.chunks_mut(16) {
        let counter_bytes = counter.to_be_bytes();
        let mut stream = cipher::Block::<aes::Aes256>::clone_from_slice(&counter_bytes);
        cipher.encrypt_block(&mut stream);
        for (b, k) in chunk.iter_mut().zip(stream.iter()) {
            *b ^= *k;
        }
        counter = counter.wrapping_add(1);
    }
}

fn chacha20_apply(data: &mut [u8], key: &[u8; 32], iv: &[u8; 16]) {
    use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};

    let mut cipher = chacha20::ChaCha20::new(key.into(), (&iv[4..16]).into());
    cipher.seek(u32::from_be_bytes(iv[..4].try_into().expect("counter")));
    cipher.apply_keystream(data);
}

/// Initialise the PSA/mbedTLS library and verify random-byte generation.
/// C: local `test_random()` inside `mbedtls_crypto_test`.
fn mbedtls_test_random() -> crate::Result<()> {
    const LEN: usize = 1021;
    const MAX_SUM_1021: u64 = 149_505;
    const MIN_SUM_1021: u64 = 110_849;

    let mut buf = [0u8; LEN];
    #[cfg(feature = "sys-mbedtls")]
    crate::sys::mbedtls::random_bytes(&mut buf)?;
    #[cfg(not(feature = "sys-mbedtls"))]
    return Err(crate::Error::Generic);

    let sum = buf.iter().map(|&b| b as u64).sum::<u64>();
    if (MIN_SUM_1021..=MAX_SUM_1021).contains(&sum) {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Test the mbedTLS SHA-256 hash against the minicrypto reference.
/// C: local `test_hash(ptls_mbedtls_sha256, ptls_minicrypto_sha256)`.
fn mbedtls_test_hash() -> crate::Result<()> {
    let input = [0xbau8; 1234];
    let final_ref = sha256_bytes(&[&input]);
    #[cfg(feature = "sys-mbedtls")]
    let final_hash = crate::sys::mbedtls::sha256_hash(&input)?;
    #[cfg(not(feature = "sys-mbedtls"))]
    return Err(crate::Error::Generic);
    if final_hash != final_ref {
        return Err(crate::Error::Generic);
    }

    let final_split = crate::sys::mbedtls::sha256_hash_incremental(&[
        &input[..input.len() - 17],
        &input[input.len() - 17..],
    ])?;
    if final_split != final_ref {
        return Err(crate::Error::Generic);
    }

    let hash1 = crate::sys::mbedtls::sha256_hash_incremental(&[&input[..input.len() - 126]])?;
    let hash2 = crate::sys::mbedtls::sha256_hash_incremental(&[&input[input.len() - 126..]])?;
    let href1 = sha256_bytes(&[&input[..input.len() - 126]]);
    let href2 = sha256_bytes(&[&input[input.len() - 126..]]);
    if hash1 == href1 && hash2 == href2 {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Test HKDF label expansion with the mbedTLS SHA-256 hash.
/// C: local `test_label(ptls_mbedtls_sha256, ptls_minicrypto_sha256)`.
fn mbedtls_test_label() -> crate::Result<()> {
    let secret = [0x5eu8; 32];
    let mut out = [0u8; 16];
    let mut reference = [0u8; 16];
    #[cfg(feature = "sys-mbedtls")]
    crate::sys::mbedtls::hkdf_expand_label_sha256("label", "label_prefix", &secret, &mut out)?;
    #[cfg(not(feature = "sys-mbedtls"))]
    return Err(crate::Error::Generic);
    crate::tls_api::hkdf_expand_label("label", "label_prefix", &secret, &mut reference)?;
    if out == reference {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Test all five mbedTLS ciphers against minicrypto reference ciphers.
/// C: local `test_cipher(cipher_test[i], cipher_ref[i])` for i in 0..5.
fn mbedtls_test_ciphers() -> crate::Result<()> {
    #[cfg(not(feature = "sys-mbedtls"))]
    return Err(crate::Error::Generic);

    #[cfg(feature = "sys-mbedtls")]
    use crate::sys::mbedtls::MbedTlsCipher;

    let key32 = [0x55u8; 32];
    let key16: [u8; 16] = key32[..16].try_into().expect("key");
    let iv = [0x33u8; 16];
    let input = [0xaau8; 16];

    let cases = [
        MbedTlsCipher::Aes128Ecb,
        MbedTlsCipher::Aes128Ctr,
        MbedTlsCipher::Aes256Ecb,
        MbedTlsCipher::Aes256Ctr,
        MbedTlsCipher::Chacha20,
    ];

    for spec in cases {
        let (out1, out2) =
            crate::sys::mbedtls::cipher_double_apply(spec, &key32, &iv, true, &input)?;
        let mut ref1 = input;
        let mut ref2 = input;
        apply_reference_cipher(spec, &mut ref1, true, &key16, &key32, &iv);
        ref2.copy_from_slice(&ref1);
        apply_reference_cipher(spec, &mut ref2, true, &key16, &key32, &iv);
        if out1.as_slice() != ref1 || out2.as_slice() != ref2 {
            return Err(crate::Error::Generic);
        }

        let (dec1, dec2) =
            crate::sys::mbedtls::cipher_double_apply(spec, &key32, &iv, false, &out2)?;
        if dec1.as_slice() != ref1 || dec2.as_slice() != input {
            return Err(crate::Error::Generic);
        }
    }

    Ok(())
}

#[cfg(feature = "sys-mbedtls")]
fn apply_reference_cipher(
    spec: crate::sys::mbedtls::MbedTlsCipher,
    block: &mut [u8; 16],
    encrypt: bool,
    key16: &[u8; 16],
    key32: &[u8; 32],
    iv: &[u8; 16],
) {
    use crate::sys::mbedtls::MbedTlsCipher;

    match spec {
        MbedTlsCipher::Aes128Ecb => aes128_ecb_crypt(block, key16, encrypt),
        MbedTlsCipher::Aes128Ctr => aes128_ctr_apply(block, key16, iv),
        MbedTlsCipher::Aes256Ecb => aes256_ecb_crypt(block, key32, encrypt),
        MbedTlsCipher::Aes256Ctr => aes256_ctr_apply(block, key32, iv),
        MbedTlsCipher::Chacha20 => chacha20_apply(block, key32, iv),
    }
}

/// Test AES-128-GCM AEAD encrypt/decrypt with mbedTLS vs minicrypto.
/// C: local `test_aead(ptls_mbedtls_aes128gcm, ptls_mbedtls_sha256, ...)`.
fn mbedtls_test_aead() -> crate::Result<()> {
    #[cfg(not(feature = "sys-mbedtls"))]
    return Err(crate::Error::Generic);

    let secret = [0x58u8; 32];
    let aad = [0xaau8; 17];
    let seq = 12_345;
    let original = vec![0x12u8; 1234];
    let encrypted =
        crate::sys::mbedtls::aes128gcm_encrypt(&secret, "test_aead", seq, &aad, &original)?;

    let enc = crate::tls_api::setup_test_aead_context(true, &secret, "test_aead")
        .ok_or(crate::Error::Tls)?;
    let mut reference = original.clone();
    enc.encrypt(seq, &aad, &mut reference);
    if encrypted != reference || encrypted.len() != original.len() + enc.tag_len() {
        return Err(crate::Error::Generic);
    }

    let decrypted =
        crate::sys::mbedtls::aes128gcm_decrypt(&secret, "test_aead", seq, &aad, &encrypted)?;
    if decrypted == original {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Test ECDH key exchange (secp256r1 and x25519) mixing mbedTLS and minicrypto.
/// C: local `test_key_exchange(...)` combinations.
fn mbedtls_test_key_exchange() -> crate::Result<()> {
    #[cfg(not(feature = "sys-mbedtls"))]
    return Err(crate::Error::Generic);

    #[cfg(feature = "sys-mbedtls")]
    {
        crate::sys::mbedtls::key_exchange_round_trip(
            crate::sys::mbedtls::MbedTlsEcGroup::Secp256r1,
        )?;
        crate::sys::mbedtls::key_exchange_round_trip(crate::sys::mbedtls::MbedTlsEcGroup::X25519)
    }
}

/// Load one private key from a DER PEM file and sign a message with it.
/// C: `mbedtls_test_load_one_der_key(path_ref)`.
fn mbedtls_test_load_one_der_key(path_ref: &str) -> crate::Result<()> {
    #[cfg(feature = "sys-mbedtls")]
    {
        crate::sys::mbedtls::load_private_key_and_sign(&fixture_path(path_ref)).map(|_| ())
    }
    #[cfg(not(feature = "sys-mbedtls"))]
    {
        let _ = path_ref;
        Err(crate::Error::Generic)
    }
}

/// Check that loading each of the expected failure cases fails.
/// C: the negative assertions in `mbedtls_load_key_fail_test`.
fn mbedtls_test_load_key_fail_cases() -> crate::Result<()> {
    for path in [
        "certs/no_such_file.pem",
        "certs/not_a_valid_pem_file.pem",
        "certs/rsa/cert.pem",
        "certs/ed25519/key.pem",
    ] {
        if mbedtls_test_load_one_der_key(path).is_ok() {
            return Err(crate::Error::Generic);
        }
    }
    Ok(())
}

/// Extract the public key from a certificate and compare to the private key.
/// C: `test_retrieve_pubkey_one(key_path_ref, cert_path_ref)`.
fn mbedtls_test_retrieve_pubkey_one(key_path_ref: &str, cert_path_ref: &str) -> crate::Result<()> {
    #[cfg(not(feature = "sys-mbedtls"))]
    return Err(crate::Error::Generic);

    let private_public_key =
        crate::sys::mbedtls::public_key_der_from_private_file(&fixture_path(key_path_ref))?;
    let certificate_public_key =
        crate::sys::mbedtls::public_key_der_from_cert_file(&fixture_path(cert_path_ref))?;
    if private_public_key == certificate_public_key {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Sign with mbedTLS and verify the signature end-to-end.
/// C: `test_sign_verify_one(key, cert, ca, server_name, config, config)`.
fn mbedtls_test_sign_verify_one(
    key_path_ref: &str,
    cert_path_ref: &str,
    ca_path_ref: &str,
    server_name: &str,
) -> crate::Result<()> {
    #[cfg(feature = "sys-mbedtls")]
    {
        crate::sys::mbedtls::sign_verify_fixture(
            &fixture_path(key_path_ref),
            &fixture_path(cert_path_ref),
            &fixture_path(ca_path_ref),
            server_name,
        )
    }
    #[cfg(not(feature = "sys-mbedtls"))]
    {
        let _ = (key_path_ref, cert_path_ref, ca_path_ref, server_name);
        Err(crate::Error::Generic)
    }
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

    let mut test_ctx = tls_api_init_ctx_ex2_ecdsa(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some(TEST_ALPN),
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
    let _runtime = MbedTlsRuntimeGuard::init();
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
    let _runtime = MbedTlsRuntimeGuard::init();
    let _tls_api_reset = TlsApiResetGuard::mbedtls_only();
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
    let _tls_api_reset = TlsApiResetGuard::mbedtls_only();
    mbedtls_test_load_key_fail_cases().expect("load_key_fail_cases");
}

/// Extract public keys from certificates and compare against private keys.
/// C: `mbedtls_retrieve_pubkey_test` (inside `#ifdef PICOQUIC_WITH_MBEDTLS`).
#[test]
fn mbedtls_retrieve_pubkey() {
    let _tls_api_reset = TlsApiResetGuard::mbedtls_only();
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
    let _runtime = MbedTlsRuntimeGuard::init();
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
    assert_mbedtls_provider_snapshot();

    mbedtls_test_random().expect("registered random provider");
    mbedtls_test_load_one_der_key("certs/rsa/key.pem").expect("registered private key loader");
    mbedtls_test_sign_verify_one(
        "certs/rsa/key.pem",
        "certs/rsa/cert.pem",
        "certs/test-ca.crt",
        "rsa.test.example.com",
    )
    .expect("registered certificate verifier");

    reset_tls_api(0);
}
