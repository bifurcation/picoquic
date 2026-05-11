//! mbedTLS-backed crypto helpers.
//!
//! These functions are the Rust counterparts for the `ptls_mbedtls_*`
//! provider surface exercised by `picoquictest/mbedtls_test.c`.

use crate::Error;

use ::mbedtls::cipher::raw::{Cipher, CipherId, CipherMode, Operation};
use ::mbedtls::hash::{Hkdf, Md, Type as MdType};
use ::mbedtls::pk::{ECDSA_MAX_LEN, EcGroupId, Pk};
use ::mbedtls::rng::{CtrDrbg, OsEntropy, Random};
use ::mbedtls::x509::Certificate;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MbedTlsCipher {
    Aes128Ecb,
    Aes128Ctr,
    Aes256Ecb,
    Aes256Ctr,
    Chacha20,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MbedTlsEcGroup {
    Secp256r1,
    X25519,
}

pub fn load(_unload: bool) {}

pub fn init() -> Result<(), Error> {
    let mut byte = [0u8; 1];
    random_bytes(&mut byte)
}

pub fn free() {}

pub fn random_bytes(out: &mut [u8]) -> Result<(), Error> {
    let mut rng = rng()?;
    rng.random(out).map_err(|_| Error::Tls)
}

pub fn sha256_hash(data: &[u8]) -> Result<[u8; 32], Error> {
    let mut out = [0u8; 32];
    Md::hash(MdType::Sha256, data, &mut out).map_err(|_| Error::Tls)?;
    Ok(out)
}

pub fn sha256_hash_incremental(parts: &[&[u8]]) -> Result<[u8; 32], Error> {
    let mut ctx = Md::new(MdType::Sha256).map_err(|_| Error::Tls)?;
    for part in parts {
        ctx.update(part).map_err(|_| Error::Tls)?;
    }
    let mut out = [0u8; 32];
    ctx.finish(&mut out).map_err(|_| Error::Tls)?;
    Ok(out)
}

pub fn hkdf_expand_label_sha256(
    label: &str,
    base_label: &str,
    secret: &[u8],
    output: &mut [u8],
) -> Result<(), Error> {
    let info = hkdf_label_info(label, base_label, output.len())?;
    Hkdf::hkdf_expand(MdType::Sha256, secret, &info, output).map_err(|_| Error::Tls)
}

pub fn cipher_double_apply(
    spec: MbedTlsCipher,
    key: &[u8; 32],
    iv: &[u8; 16],
    is_encrypt: bool,
    input: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let first = cipher_apply(spec, key, iv, is_encrypt, input)?;
    let second = cipher_apply(spec, key, iv, is_encrypt, &first)?;
    Ok((first, second))
}

pub fn aes128gcm_encrypt(
    secret: &[u8],
    prefix_label: &str,
    packet: u64,
    aad: &[u8],
    plain: &[u8],
) -> Result<Vec<u8>, Error> {
    let (key, iv) = derive_aes128gcm_key_iv(secret, prefix_label)?;
    let nonce = quic_nonce(&iv, packet);
    let mut cipher = Cipher::setup(CipherId::Aes, CipherMode::GCM, 128).map_err(|_| Error::Tls)?;
    cipher
        .set_key(Operation::Encrypt, &key)
        .map_err(|_| Error::Tls)?;
    cipher.set_iv(&nonce).map_err(|_| Error::Tls)?;
    let mut out = vec![0u8; plain.len() + 16];
    let len = cipher
        .encrypt_auth(aad, plain, &mut out, 16)
        .map_err(|_| Error::Tls)?;
    out.truncate(len);
    Ok(out)
}

pub fn aes128gcm_decrypt(
    secret: &[u8],
    prefix_label: &str,
    packet: u64,
    aad: &[u8],
    cipher_and_tag: &[u8],
) -> Result<Vec<u8>, Error> {
    let (key, iv) = derive_aes128gcm_key_iv(secret, prefix_label)?;
    let nonce = quic_nonce(&iv, packet);
    let mut cipher = Cipher::setup(CipherId::Aes, CipherMode::GCM, 128).map_err(|_| Error::Tls)?;
    cipher
        .set_key(Operation::Decrypt, &key)
        .map_err(|_| Error::Tls)?;
    cipher.set_iv(&nonce).map_err(|_| Error::Tls)?;
    let mut out = vec![0u8; cipher_and_tag.len()];
    let len = cipher
        .decrypt_auth(aad, cipher_and_tag, &mut out, 16)
        .map_err(|_| Error::Tls)?;
    out.truncate(len);
    Ok(out)
}

pub fn key_exchange_round_trip(group: MbedTlsEcGroup) -> Result<(), Error> {
    if Pk::from_public_key(&[]).is_ok() {
        return Err(Error::Tls);
    }

    let group_id = match group {
        MbedTlsEcGroup::Secp256r1 => EcGroupId::SecP256R1,
        MbedTlsEcGroup::X25519 => EcGroupId::Curve25519,
    };
    let mut rng = rng()?;
    let mut client = Pk::generate_ec(&mut rng, group_id).map_err(|_| Error::Tls)?;
    let mut server = Pk::generate_ec(&mut rng, group_id).map_err(|_| Error::Tls)?;

    let server_public_der = server.write_public_der_vec().map_err(|_| Error::Tls)?;
    let client_public_der = client.write_public_der_vec().map_err(|_| Error::Tls)?;
    let server_public = Pk::from_public_key(&server_public_der).map_err(|_| Error::Tls)?;
    let client_public = Pk::from_public_key(&client_public_der).map_err(|_| Error::Tls)?;

    let mut client_secret = vec![0u8; 80];
    let mut server_secret = vec![0u8; 80];
    let client_len = client
        .agree(&server_public, &mut client_secret, &mut rng)
        .map_err(|_| Error::Tls)?;
    let server_len = server
        .agree(&client_public, &mut server_secret, &mut rng)
        .map_err(|_| Error::Tls)?;
    client_secret.truncate(client_len);
    server_secret.truncate(server_len);
    if client_secret == server_secret {
        Ok(())
    } else {
        Err(Error::Tls)
    }
}

pub fn load_private_key_and_sign(path: &std::path::Path) -> Result<Vec<u8>, Error> {
    let mut key = load_private_key(path)?;
    sign_test_hash(&mut key)
}

pub fn public_key_der_from_private_file(path: &std::path::Path) -> Result<Vec<u8>, Error> {
    let mut key = load_private_key(path)?;
    key.write_public_der_vec().map_err(|_| Error::Tls)
}

pub fn public_key_der_from_cert_file(path: &std::path::Path) -> Result<Vec<u8>, Error> {
    let pem = pem_file_with_nul(path)?;
    let mut cert = Certificate::from_pem(&pem).map_err(|_| Error::InvalidFile)?;
    cert.public_key_mut()
        .write_public_der_vec()
        .map_err(|_| Error::Tls)
}

pub fn sign_verify_fixture(
    key_path: &std::path::Path,
    cert_path: &std::path::Path,
    trusted_path: &std::path::Path,
    server_name: &str,
) -> Result<(), Error> {
    let mut key = load_private_key(key_path)?;
    let cert_pem = pem_file_with_nul(cert_path)?;
    let trusted_pem = pem_file_with_nul(trusted_path)?;

    let chain = Certificate::from_pem_multiple(&cert_pem).map_err(|_| Error::InvalidFile)?;
    let trust_ca = Certificate::from_pem_multiple(&trusted_pem).map_err(|_| Error::InvalidFile)?;
    let mut verify_info = String::new();
    Certificate::verify_with_expected_common_name(
        &chain,
        &trust_ca,
        None,
        Some(&mut verify_info),
        Some(server_name),
    )
    .map_err(|_| Error::Tls)?;

    let mut cert = Certificate::from_pem(&cert_pem).map_err(|_| Error::InvalidFile)?;
    let signature = sign_message_hash(&mut key, TEST_SIGN_VERIFY_MESSAGE)?;
    let digest = sha256_hash(TEST_SIGN_VERIFY_MESSAGE)?;
    cert.public_key_mut()
        .verify(MdType::Sha256, &digest, &signature)
        .map_err(|_| Error::Tls)
}

fn rng() -> Result<CtrDrbg, Error> {
    let entropy = Arc::new(OsEntropy::new());
    CtrDrbg::new(entropy, None).map_err(|_| Error::Tls)
}

fn pem_file_with_nul(path: &std::path::Path) -> Result<Vec<u8>, Error> {
    let mut bytes = std::fs::read(path).map_err(|_| Error::NoSuchFile)?;
    if !bytes.ends_with(&[0]) {
        bytes.push(0);
    }
    Ok(bytes)
}

fn load_private_key(path: &std::path::Path) -> Result<Pk, Error> {
    let pem = pem_file_with_nul(path)?;
    Pk::from_private_key(&pem, None).map_err(|_| Error::InvalidFile)
}

fn sign_test_hash(key: &mut Pk) -> Result<Vec<u8>, Error> {
    let hash: [u8; 32] = core::array::from_fn(|i| (i + 1) as u8);
    sign_hash(key, &hash)
}

fn sign_message_hash(key: &mut Pk, message: &[u8]) -> Result<Vec<u8>, Error> {
    let hash = sha256_hash(message)?;
    sign_hash(key, &hash)
}

fn sign_hash(key: &mut Pk, hash: &[u8]) -> Result<Vec<u8>, Error> {
    let mut rng = rng()?;
    let mut signature = vec![0u8; signature_capacity(key)];
    let len = key
        .sign(MdType::Sha256, hash, &mut signature, &mut rng)
        .map_err(|_| Error::Tls)?;
    signature.truncate(len);
    key.verify(MdType::Sha256, hash, &signature)
        .map_err(|_| Error::Tls)?;
    Ok(signature)
}

fn signature_capacity(key: &Pk) -> usize {
    let byte_len = key.len().div_ceil(8);
    byte_len.max(ECDSA_MAX_LEN)
}

fn cipher_apply(
    spec: MbedTlsCipher,
    key: &[u8; 32],
    iv: &[u8; 16],
    is_encrypt: bool,
    input: &[u8],
) -> Result<Vec<u8>, Error> {
    let (cipher_id, mode, key_len, iv_slice) = match spec {
        MbedTlsCipher::Aes128Ecb => (CipherId::Aes, CipherMode::ECB, 16, None),
        MbedTlsCipher::Aes128Ctr => (CipherId::Aes, CipherMode::CTR, 16, Some(&iv[..])),
        MbedTlsCipher::Aes256Ecb => (CipherId::Aes, CipherMode::ECB, 32, None),
        MbedTlsCipher::Aes256Ctr => (CipherId::Aes, CipherMode::CTR, 32, Some(&iv[..])),
        MbedTlsCipher::Chacha20 => (CipherId::Chacha20, CipherMode::STREAM, 32, Some(&iv[..])),
    };
    let operation = if is_encrypt {
        Operation::Encrypt
    } else {
        Operation::Decrypt
    };
    let mut cipher =
        Cipher::setup(cipher_id, mode, (key_len * 8) as u32).map_err(|_| Error::Tls)?;
    cipher
        .set_key(operation, &key[..key_len])
        .map_err(|_| Error::Tls)?;
    if let Some(iv) = iv_slice {
        cipher.set_iv(iv).map_err(|_| Error::Tls)?;
    }
    let mut out = vec![0u8; input.len() + 16];
    let len = if is_encrypt {
        cipher.encrypt(input, &mut out).map_err(|_| Error::Tls)?
    } else {
        cipher.decrypt(input, &mut out).map_err(|_| Error::Tls)?
    };
    out.truncate(len);
    Ok(out)
}

fn derive_aes128gcm_key_iv(
    secret: &[u8],
    prefix_label: &str,
) -> Result<([u8; 16], [u8; 12]), Error> {
    let mut key = [0u8; 16];
    let mut iv = [0u8; 12];
    hkdf_expand_label_sha256("key", prefix_label, secret, &mut key)?;
    hkdf_expand_label_sha256("iv", prefix_label, secret, &mut iv)?;
    Ok((key, iv))
}

fn hkdf_label_info(label: &str, base_label: &str, output_len: usize) -> Result<Vec<u8>, Error> {
    if output_len > u16::MAX as usize {
        return Err(Error::InvalidArgument);
    }
    let mut full_label = Vec::with_capacity(base_label.len() + label.len());
    full_label.extend_from_slice(base_label.as_bytes());
    full_label.extend_from_slice(label.as_bytes());
    if full_label.len() > u8::MAX as usize {
        return Err(Error::InvalidArgument);
    }

    let mut info = Vec::with_capacity(2 + 1 + full_label.len() + 1);
    info.extend_from_slice(&(output_len as u16).to_be_bytes());
    info.push(full_label.len() as u8);
    info.extend_from_slice(&full_label);
    info.push(0);
    Ok(info)
}

fn quic_nonce(iv: &[u8; 12], packet: u64) -> [u8; 12] {
    let mut nonce = *iv;
    for (n, p) in nonce[4..].iter_mut().zip(packet.to_be_bytes()) {
        *n ^= p;
    }
    nonce
}

const TEST_SIGN_VERIFY_MESSAGE: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64,
];
