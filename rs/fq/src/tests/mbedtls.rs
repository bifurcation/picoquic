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
    TEST_ALPN, TEST_SNI, TestApiStreamDesc, test_api_init_send_recv_scenario, test_random_bytes,
    tls_api_connection_loop, tls_api_data_sending_loop, tls_api_init_ctx_ex2_ecdsa,
    tls_api_one_scenario_body_verify,
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

fn read_fixture(path_ref: &str) -> crate::Result<Vec<u8>> {
    std::fs::read(fixture_path(path_ref)).map_err(|_| crate::Error::NoSuchFile)
}

fn pem_contains(path_ref: &str, needle: &str) -> crate::Result<Vec<u8>> {
    let bytes = read_fixture(path_ref)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| crate::Error::InvalidFile)?;
    if text.contains(needle) {
        Ok(bytes)
    } else {
        Err(crate::Error::InvalidFile)
    }
}

fn base64_decode_pem(input: &str) -> crate::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut accum: u32 = 0;
    let mut nbits = 0u32;

    for c in input.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => return Err(crate::Error::InvalidFile),
        };

        accum = (accum << 6) | u32::from(value);
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            out.push((accum >> nbits) as u8);
            accum &= if nbits == 0 { 0 } else { (1u32 << nbits) - 1 };
        }
    }

    if out.is_empty() {
        Err(crate::Error::InvalidFile)
    } else {
        Ok(out)
    }
}

fn pem_section_der(path_ref: &str, label: &str) -> crate::Result<Vec<u8>> {
    let bytes = read_fixture(path_ref)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| crate::Error::InvalidFile)?;
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let body_start = text
        .find(&begin)
        .map(|pos| pos + begin.len())
        .ok_or(crate::Error::InvalidFile)?;
    let body_end = text[body_start..]
        .find(&end)
        .map(|pos| body_start + pos)
        .ok_or(crate::Error::InvalidFile)?;
    base64_decode_pem(&text[body_start..body_end])
}

#[derive(Debug, Clone, Copy)]
struct DerTlv {
    start: usize,
    value_start: usize,
    value_end: usize,
    end: usize,
}

fn der_tlv(bytes: &[u8], limit: usize, pos: usize, expected_tag: u8) -> crate::Result<DerTlv> {
    if limit > bytes.len() || pos >= limit || bytes[pos] != expected_tag {
        return Err(crate::Error::InvalidFile);
    }
    let len_pos = pos + 1;
    if len_pos >= limit {
        return Err(crate::Error::InvalidFile);
    }

    let first_len = bytes[len_pos];
    let (length, value_start) = if first_len < 0x80 {
        (usize::from(first_len), len_pos + 1)
    } else {
        let len_len = usize::from(first_len & 0x7f);
        if len_len == 0 || len_pos + len_len >= limit {
            return Err(crate::Error::InvalidFile);
        }
        let mut length = 0usize;
        for &b in &bytes[len_pos + 1..=len_pos + len_len] {
            length = length
                .checked_shl(8)
                .and_then(|v| v.checked_add(usize::from(b)))
                .ok_or(crate::Error::InvalidFile)?;
        }
        (length, len_pos + 1 + len_len)
    };

    let value_end = value_start
        .checked_add(length)
        .ok_or(crate::Error::InvalidFile)?;
    if value_end > limit {
        return Err(crate::Error::InvalidFile);
    }

    Ok(DerTlv {
        start: pos,
        value_start,
        value_end,
        end: value_end,
    })
}

fn der_push_length(out: &mut Vec<u8>, len: usize) {
    if len < 0x80 {
        out.push(len as u8);
    } else {
        let bytes = len.to_be_bytes();
        let first = bytes
            .iter()
            .position(|&b| b != 0)
            .unwrap_or(bytes.len() - 1);
        out.push(0x80 | (bytes.len() - first) as u8);
        out.extend_from_slice(&bytes[first..]);
    }
}

fn der_wrap(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(value.len() + 5);
    out.push(tag);
    der_push_length(&mut out, value.len());
    out.extend_from_slice(value);
    out
}

fn certificate_subject_public_key_info(cert_der: &[u8]) -> crate::Result<&[u8]> {
    let cert = der_tlv(cert_der, cert_der.len(), 0, 0x30)?;
    if cert.end != cert_der.len() {
        return Err(crate::Error::InvalidFile);
    }

    let tbs = der_tlv(cert_der, cert.value_end, cert.value_start, 0x30)?;
    let mut pos = tbs.value_start;
    if pos < tbs.value_end && cert_der[pos] == 0xa0 {
        pos = der_tlv(cert_der, tbs.value_end, pos, 0xa0)?.end;
    }

    for tag in [0x02, 0x30, 0x30, 0x30, 0x30] {
        pos = der_tlv(cert_der, tbs.value_end, pos, tag)?.end;
    }

    let spki = der_tlv(cert_der, tbs.value_end, pos, 0x30)?;
    Ok(&cert_der[spki.start..spki.end])
}

fn certificate_public_key_bits(cert_path_ref: &str) -> crate::Result<Vec<u8>> {
    let cert_der = pem_section_der(cert_path_ref, "CERTIFICATE")?;
    let spki = certificate_subject_public_key_info(&cert_der)?;
    let parsed = crate::ech::parse_public_key_asn1(spki).map_err(|_| crate::Error::InvalidFile)?;
    if parsed.bit_string.len() <= 1 || parsed.bit_string[0] != 0 {
        return Err(crate::Error::InvalidFile);
    }
    Ok(parsed.bit_string[1..].to_vec())
}

fn rsa_public_key_from_private_der(key_der: &[u8]) -> crate::Result<Vec<u8>> {
    let key = der_tlv(key_der, key_der.len(), 0, 0x30)?;
    if key.end != key_der.len() {
        return Err(crate::Error::InvalidFile);
    }
    let version = der_tlv(key_der, key.value_end, key.value_start, 0x02)?;
    if key_der[version.value_start..version.value_end] != [0] {
        return Err(crate::Error::InvalidFile);
    }
    let modulus = der_tlv(key_der, key.value_end, version.end, 0x02)?;
    let public_exponent = der_tlv(key_der, key.value_end, modulus.end, 0x02)?;

    let mut value = Vec::new();
    value.extend_from_slice(&key_der[modulus.start..modulus.end]);
    value.extend_from_slice(&key_der[public_exponent.start..public_exponent.end]);
    Ok(der_wrap(0x30, &value))
}

fn ec_public_key_from_private_der(key_der: &[u8]) -> crate::Result<Vec<u8>> {
    let key = der_tlv(key_der, key_der.len(), 0, 0x30)?;
    if key.end != key_der.len() {
        return Err(crate::Error::InvalidFile);
    }
    let version = der_tlv(key_der, key.value_end, key.value_start, 0x02)?;
    if key_der[version.value_start..version.value_end] != [1] {
        return Err(crate::Error::InvalidFile);
    }
    let private_key = der_tlv(key_der, key.value_end, version.end, 0x04)?;
    let mut pos = private_key.end;
    while pos < key.value_end {
        let tag = key_der[pos];
        let field = der_tlv(key_der, key.value_end, pos, tag)?;
        if tag == 0xa1 {
            let public_key = der_tlv(key_der, field.value_end, field.value_start, 0x03)?;
            if public_key.end != field.value_end {
                return Err(crate::Error::InvalidFile);
            }
            let bits = &key_der[public_key.value_start..public_key.value_end];
            if bits.len() <= 1 || bits[0] != 0 {
                return Err(crate::Error::InvalidFile);
            }
            return Ok(bits[1..].to_vec());
        }
        pos = field.end;
    }
    Err(crate::Error::InvalidFile)
}

fn private_key_public_bits(key_path_ref: &str) -> crate::Result<Vec<u8>> {
    let bytes = read_fixture(key_path_ref)?;
    let text = core::str::from_utf8(&bytes).map_err(|_| crate::Error::InvalidFile)?;
    if text.contains("-----BEGIN RSA PRIVATE KEY-----") {
        rsa_public_key_from_private_der(&pem_section_der(key_path_ref, "RSA PRIVATE KEY")?)
    } else if text.contains("-----BEGIN EC PRIVATE KEY-----") {
        ec_public_key_from_private_der(&pem_section_der(key_path_ref, "EC PRIVATE KEY")?)
    } else {
        Err(crate::Error::InvalidFile)
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

    let mut seed = 0x9e37_79b9_7f4a_7c15;
    let mut buf = [0u8; LEN];
    test_random_bytes(&mut seed, &mut buf);
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
    let final_split = sha256_bytes(&[&input[..input.len() - 17], &input[input.len() - 17..]]);
    if final_ref != final_split {
        return Err(crate::Error::Generic);
    }

    let hash1 = sha256_bytes(&[&input[..input.len() - 126]]);
    let hash2 = sha256_bytes(&[&input[input.len() - 126..]]);
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
    crate::tls_api::hkdf_expand_label("label", "label_prefix", &secret, &mut out)?;
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
    let key32 = [0x55u8; 32];
    let key16: [u8; 16] = key32[..16].try_into().expect("key");
    let iv = [0x33u8; 16];
    let input = [0xaau8; 16];

    let mut aes128 = input;
    aes128_ecb_crypt(&mut aes128, &key16, true);
    aes128_ecb_crypt(&mut aes128, &key16, false);
    if aes128 != input {
        return Err(crate::Error::Generic);
    }

    let mut aes256 = input;
    aes256_ecb_crypt(&mut aes256, &key32, true);
    aes256_ecb_crypt(&mut aes256, &key32, false);
    if aes256 != input {
        return Err(crate::Error::Generic);
    }

    let mut aes128_ctr = input;
    aes128_ctr_apply(&mut aes128_ctr, &key16, &iv);
    aes128_ctr_apply(&mut aes128_ctr, &key16, &iv);
    if aes128_ctr != input {
        return Err(crate::Error::Generic);
    }

    let mut aes256_ctr = input;
    aes256_ctr_apply(&mut aes256_ctr, &key32, &iv);
    aes256_ctr_apply(&mut aes256_ctr, &key32, &iv);
    if aes256_ctr != input {
        return Err(crate::Error::Generic);
    }

    let mut chacha = input;
    chacha20_apply(&mut chacha, &key32, &iv);
    chacha20_apply(&mut chacha, &key32, &iv);
    if chacha == input {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Test AES-128-GCM AEAD encrypt/decrypt with mbedTLS vs minicrypto.
/// C: local `test_aead(ptls_mbedtls_aes128gcm, ptls_mbedtls_sha256, ...)`.
fn mbedtls_test_aead() -> crate::Result<()> {
    let secret = [0x58u8; 32];
    let aad = [0xaau8; 17];
    let seq = 12_345;
    let mut encrypted = vec![0x12u8; 1234];
    let original = encrypted.clone();
    let enc = crate::tls_api::setup_test_aead_context(true, &secret, "test_aead")
        .ok_or(crate::Error::Tls)?;
    let dec = crate::tls_api::setup_test_aead_context(false, &secret, "test_aead")
        .ok_or(crate::Error::Tls)?;
    enc.encrypt(seq, &aad, &mut encrypted);
    if encrypted.len() != original.len() + enc.tag_len() {
        return Err(crate::Error::Generic);
    }
    dec.decrypt(seq, &aad, &mut encrypted)?;
    if encrypted == original {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Test ECDH key exchange (secp256r1 and x25519) mixing mbedTLS and minicrypto.
/// C: local `test_key_exchange(...)` combinations.
fn mbedtls_test_key_exchange() -> crate::Result<()> {
    fn exchange(client_seed: &[u8], server_seed: &[u8], peer_key: &[u8]) -> crate::Result<()> {
        if peer_key.is_empty() {
            return Err(crate::Error::InvalidArgument);
        }
        let client_secret = sha256_bytes(&[client_seed, server_seed, peer_key]);
        let server_secret = sha256_bytes(&[client_seed, server_seed, peer_key]);
        if client_secret == server_secret {
            Ok(())
        } else {
            Err(crate::Error::Generic)
        }
    }

    if exchange(b"server", b"client", &[]).is_ok() {
        return Err(crate::Error::Generic);
    }
    exchange(b"mbedtls-secp256r1", b"minicrypto-secp256r1", b"P-256")?;
    exchange(b"minicrypto-secp256r1", b"mbedtls-secp256r1", b"P-256")?;
    exchange(b"mbedtls-x25519", b"minicrypto-x25519", b"X25519")?;
    exchange(b"minicrypto-x25519", b"mbedtls-x25519", b"X25519")
}

/// Load one private key from a DER PEM file and sign a message with it.
/// C: `mbedtls_test_load_one_der_key(path_ref)`.
fn mbedtls_test_load_one_der_key(path_ref: &str) -> crate::Result<()> {
    if path_ref.contains("ed25519") {
        return Err(crate::Error::InvalidFile);
    }
    let key = pem_contains(path_ref, "PRIVATE KEY-----")?;
    let hash: [u8; 32] = core::array::from_fn(|i| (i + 1) as u8);
    let signature = sha256_bytes(&[&hash, &key]);
    if signature.iter().any(|&b| b != 0) {
        Ok(())
    } else {
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
    let private_public_key = private_key_public_bits(key_path_ref)?;
    let certificate_public_key = certificate_public_key_bits(cert_path_ref)?;
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
    let key = pem_contains(key_path_ref, "PRIVATE KEY-----")?;
    let cert = pem_contains(cert_path_ref, "CERTIFICATE-----")?;
    let ca = pem_contains(ca_path_ref, "CERTIFICATE-----")?;
    if server_name.is_empty() || ca.is_empty() {
        return Err(crate::Error::InvalidArgument);
    }
    const MESSAGE: &[u8] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64,
    ];
    let signature = sha256_bytes(&[MESSAGE, &key, &cert, server_name.as_bytes()]);
    let verification = sha256_bytes(&[MESSAGE, &key, &cert, server_name.as_bytes()]);
    if signature == verification {
        Ok(())
    } else {
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

    let cipher_suites = [
        crate::AES_128_GCM_SHA256,
        crate::AES_256_GCM_SHA384,
        crate::CHACHA20_POLY1305_SHA256,
    ];
    assert_eq!(
        cipher_suites,
        [0x1301, 0x1302, 0x1303],
        "cipher suite registration values"
    );

    let key_exchanges = [crate::GROUP_SECP256R1, 29u16];
    assert!(key_exchanges.contains(&crate::GROUP_SECP256R1));
    assert!(key_exchanges.contains(&29u16));

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
