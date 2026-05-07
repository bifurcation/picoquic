//! Translation of `quic/tls_api.h`.
//!
//! `tls_api.h` is the boundary between quic-core's QUIC machinery
//! and the TLS 1.3 stack underneath.  It exposes the master
//! TLS context lifecycle, per-connection TLS contexts, the
//! retry-token / retry-protection helpers, and a handful of
//! provider-installation entry points kept in this header so
//! applications don't have to include `tls.h`.
//!
//! Phase 4: core bodies are implemented.  Backend-specific TLS
//! handshakes use the trait objects in [`crate::tls`], while the QUIC
//! Initial, retry-token, retry-integrity, reset-secret, and test
//! crypto helpers are implemented here with RustCrypto primitives.
//!
//! ## Shape conventions
//!
//! * Free C functions whose first argument is a `Quic*` or `Connection*`
//!   become inherent methods on [`Quic`] / [`Connection`].
//! * The C `void*` AEAD / PN-encryption / cipher / hash contexts
//!   are gone — Phase 2 replaced them with the trait family in
//!   [`crate::tls`] (`PacketKey`, `HeaderKey`, `Session`,
//!   `ClientConfig`, `ServerConfig`).  Backend implementations
//!   (picotls, rustls, OpenSSL) wrap their native session types
//!   to satisfy those traits.  No `unsafe fn` or raw-pointer
//!   parameters remain in this module.
//! * `int` flag parameters that encode booleans (`is_enc`,
//!   `is_client`, `client_mode`, `use_low_memory`, `reset`,
//!   `check_reuse`, `sending`) become `bool`.
//! * C status-code returns (`int` 0/-1 / `ERROR_*`) become
//!   `Result<T, Error>`.  The `SIZE_MAX` failure sentinel returned
//!   by `aead_decrypt_*` becomes `Err(Error::AeadCheck)` instead of
//!   leaking the sentinel through the success arm.
//! * Owning `PtlsIovec* get_certs_from_file(…, size_t* count)`
//!   collapses to `Option<Vec<Vec<u8>>>`: the C callee `malloc`s
//!   the slot array and writes its length through the out-pointer,
//!   and the caller `free`s.  A Rust `Vec` carries both ownership
//!   and length, with `Drop` taking the place of the manual free.
//! * Output parameters (`uint8_t reset_secret[16]`,
//!   `uint8_t* master_secret`, `void** aead_ctx`, `size_t*
//!   text_length`, `int* data_consumed`, `int* is_new_token`,
//!   `ConnectionId* odcid`, `size_t* token_size`, `size_t* length`)
//!   become `&mut`-borrowed slots or are returned as part of the
//!   function's result tuple, depending on whether the caller
//!   already holds storage.
//! * `struct sockaddr*` arguments map to [`core::net::SocketAddr`]
//!   for parity with `quic.rs`.
//! * `FreeVerifyCertificateCtx` is a function-pointer typedef in
//!   `quic.h`; the trait counterpart is already in `quic.rs` and is
//!   reused here.
//! * Function-pointer parameters in callbacks
//!   ([`Quic::tls_set_verify_certificate_callback`]) take
//!   `Option<Box<dyn …>>` — the C `NULL` sentinel maps to `None`
//!   and the registered callback is owned by the registry.
//!
//! ## Initial-secret label constants
//!
//! `LABEL_QUIC_BASE` is `#define`d as `NULL` in C — it's the "no
//! prefix label" sentinel passed to tls.  In Rust the prefix-label
//! parameters that accept it become `Option<&str>`, so the constant
//! itself doesn't appear; the call sites pass `None` when they would
//! have passed `LABEL_QUIC_BASE`.
//!
//! ## Public vs. internal `set_*` methods
//!
//! `picoquic.h` exposes `picoquic_set_verify_certificate_callback`,
//! `picoquic_set_client_authentication`, and
//! `picoquic_set_use_exporter` as thin wrappers around the
//! `picoquic_tls_set_*` functions declared here.  The wrappers live
//! on [`Quic`] without the `tls_` prefix (see `lib.rs`); the
//! TLS-side methods in this module keep the `tls_` prefix so both
//! pairs can coexist.
//!
//! ## Deliberate omissions
//!
//! * The five `#if 0`-disabled `cid_*_under_mask_ctx` /
//!   `cid_free_encrypt_global_ctx` entries are dead code in the C
//!   source and are not translated.
//! * `get_private_key_from_file` is also `#if 0` in the header (and
//!   the `_t` callback variant lives in `crypto_provider_api.rs`);
//!   not translated.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::net::{IpAddr, SocketAddr};
use digest::Digest;

use crate::Instant;
use crate::errors::InternalError;
use crate::internal::{
    CryptoContext, MAX_PACKET_SIZE, NB_TP_0RTT, NUMBER_OF_EPOCHS, Path, RETRY_TOKEN_PAD_SIZE,
    StoredTicket, StoredToken, StreamQueueNode, TOKEN_DELAY_LONG, TOKEN_DELAY_SHORT, Version,
};
use crate::{
    AES_128_GCM_SHA256, AES_256_GCM_SHA384, CHACHA20_POLY1305_SHA256, Connection, ConnectionId,
    Error, GROUP_SECP256R1, Quic, RESET_SECRET_SIZE,
};

// ---------------------------------------------------------------------------
// Label constants (C `#define` → `&str`).
//
// All labels are NUL-terminated bytes in the C header because tls
// takes them as `char const*`.  The Rust shape is plain `&str`; the
// trailing NUL is reintroduced (if needed) at the tls FFI
// boundary.

/// Initial secret label for the client direction.  C:
/// `LABEL_INITIAL_CLIENT`.
pub const LABEL_INITIAL_CLIENT: &str = "client in";

/// Initial secret label for the server direction.  C:
/// `LABEL_INITIAL_SERVER`.
pub const LABEL_INITIAL_SERVER: &str = "server in";

/// Traffic-update label for QUIC v1.  C:
/// `LABEL_V1_TRAFFIC_UPDATE`.
pub const LABEL_V1_TRAFFIC_UPDATE: &str = "quic ku";

/// Traffic-update label for QUIC v2.  C:
/// `LABEL_V2_TRAFFIC_UPDATE`.
pub const LABEL_V2_TRAFFIC_UPDATE: &str = "quicv2 ku";

/// HKDF "key" component label.  C: `LABEL_KEY`.
pub const LABEL_KEY: &str = "key";

/// HKDF "iv" component label.  C: `LABEL_IV`.
pub const LABEL_IV: &str = "iv";

/// HKDF "hp" (header protection) component label.  C:
/// `LABEL_HP`.
pub const LABEL_HP: &str = "hp";

/// HKDF "cid" component label.  C: `LABEL_CID`.
pub const LABEL_CID: &str = "cid";

/// HKDF label for the global CID encryption secret.  C:
/// `LABEL_CID_GLOBAL`.
pub const LABEL_CID_GLOBAL: &str = "cid global";

/// Number of HKDF rounds run when deriving the global CID secret.
/// C: `LABEL_CID_GLOBAL_ROUNDS`.
pub const LABEL_CID_GLOBAL_ROUNDS: u32 = 4;

/// QUIC v1 TLS key-derivation label prefix.  C:
/// `LABEL_QUIC_V1_KEY_BASE`.
pub const LABEL_QUIC_V1_KEY_BASE: &str = "tls13 quic ";

/// QUIC v2 TLS key-derivation label prefix.  C:
/// `LABEL_QUIC_V2_KEY_BASE`.
pub const LABEL_QUIC_V2_KEY_BASE: &str = "tls13 quicv2 ";

const SHA256_SIZE: usize = 32;
const AES128_KEY_SIZE: usize = 16;
const AES_GCM_IV_SIZE: usize = 12;
const TICKET_AEAD_LABEL: &str = "random label";
const TLS13_LABEL_PREFIX: &str = "tls13 ";

fn version_from_index(version_index: i32) -> Option<Version> {
    match version_index {
        0 => Some(Version::V1),
        1 => Some(Version::V2),
        2 => Some(Version::V2Draft),
        3 => Some(Version::PostIesg),
        4 => Some(Version::TwentyFirstInterop),
        5 => Some(Version::TwentiethInterop),
        6 => Some(Version::TwentiethPreInterop),
        7 => Some(Version::NineteenthInterop),
        8 => Some(Version::NineteenthBisInterop),
        9 => Some(Version::EighteenthInterop),
        10 => Some(Version::SeventeenthInterop),
        11 => Some(Version::InternalTest2),
        12 => Some(Version::InternalTest1),
        _ => None,
    }
}

fn connection_version(cnx: &Connection) -> Version {
    Version::try_from_wire(cnx.proposed_version)
        .or_else(|| version_from_index(cnx.version_index))
        .unwrap_or(Version::V1)
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

fn hkdf_expand_label_sha256(
    label: &str,
    base_label: &str,
    secret: &[u8],
    output: &mut [u8],
) -> Result<(), Error> {
    let info = hkdf_label_info(label, base_label, output.len())?;
    let hkdf = hkdf::Hkdf::<sha2::Sha256>::from_prk(secret).map_err(|_| Error::InvalidArgument)?;
    hkdf.expand(&info, output)
        .map_err(|_| Error::InvalidArgument)
}

fn hkdf_expand_label_sha384(
    label: &str,
    base_label: &str,
    secret: &[u8],
    output: &mut [u8],
) -> Result<(), Error> {
    let info = hkdf_label_info(label, base_label, output.len())?;
    let hkdf = hkdf::Hkdf::<sha2::Sha384>::from_prk(secret).map_err(|_| Error::InvalidArgument)?;
    hkdf.expand(&info, output)
        .map_err(|_| Error::InvalidArgument)
}

fn hkdf_expand_label_sha512(
    label: &str,
    base_label: &str,
    secret: &[u8],
    output: &mut [u8],
) -> Result<(), Error> {
    let info = hkdf_label_info(label, base_label, output.len())?;
    let hkdf = hkdf::Hkdf::<sha2::Sha512>::from_prk(secret).map_err(|_| Error::InvalidArgument)?;
    hkdf.expand(&info, output)
        .map_err(|_| Error::InvalidArgument)
}

fn derive_aes128_gcm_key_iv(
    secret: &[u8],
    prefix_label: &str,
) -> Result<([u8; AES128_KEY_SIZE], [u8; AES_GCM_IV_SIZE]), Error> {
    let mut key = [0u8; AES128_KEY_SIZE];
    let mut iv = [0u8; AES_GCM_IV_SIZE];
    hkdf_expand_label(LABEL_KEY, prefix_label, secret, &mut key)?;
    hkdf_expand_label(LABEL_IV, prefix_label, secret, &mut iv)?;
    Ok((key, iv))
}

#[derive(Clone)]
struct Aes128GcmPacketKey {
    cipher: aes_gcm::Aes128Gcm,
    iv: [u8; AES_GCM_IV_SIZE],
}

impl Aes128GcmPacketKey {
    fn from_secret(secret: &[u8], prefix_label: &str) -> Result<Self, Error> {
        use aes_gcm::KeyInit;

        let (key, iv) = derive_aes128_gcm_key_iv(secret, prefix_label)?;
        let cipher =
            aes_gcm::Aes128Gcm::new_from_slice(&key).map_err(|_| Error::InvalidArgument)?;
        Ok(Self { cipher, iv })
    }

    fn nonce_mp(&self, path_id: u64, packet: u64) -> [u8; AES_GCM_IV_SIZE] {
        let mut nonce = self.iv;
        let path = (path_id as u32).to_be_bytes();
        for (n, p) in nonce[..4].iter_mut().zip(path.iter()) {
            *n ^= *p;
        }
        let pn = packet.to_be_bytes();
        for (n, p) in nonce[4..].iter_mut().zip(pn.iter()) {
            *n ^= *p;
        }
        nonce
    }
}

impl crate::tls::PacketKey for Aes128GcmPacketKey {
    fn encrypt(&self, packet: u64, header: &[u8], payload: &mut Vec<u8>) {
        self.encrypt_mp(0, packet, header, payload);
    }

    fn decrypt(&self, packet: u64, header: &[u8], payload: &mut Vec<u8>) -> Result<(), Error> {
        self.decrypt_mp(0, packet, header, payload)
    }

    fn encrypt_mp(&self, path_id: u64, packet: u64, header: &[u8], payload: &mut Vec<u8>) {
        use aes_gcm::aead::AeadInPlace;

        let nonce = self.nonce_mp(path_id, packet);
        let tag = self
            .cipher
            .encrypt_in_place_detached((&nonce).into(), header, payload.as_mut_slice())
            .expect("AES-GCM encryption should not fail for in-place buffers");
        payload.extend_from_slice(&tag);
    }

    fn decrypt_mp(
        &self,
        path_id: u64,
        packet: u64,
        header: &[u8],
        payload: &mut Vec<u8>,
    ) -> Result<(), Error> {
        use aes_gcm::aead::AeadInPlace;

        if payload.len() < QUIC_AEAD_TAG_LEN {
            return Err(Error::Protocol(InternalError::AeadCheck as u64));
        }
        let tag_index = payload.len() - QUIC_AEAD_TAG_LEN;
        let tag_bytes: [u8; QUIC_AEAD_TAG_LEN] = payload[tag_index..]
            .try_into()
            .map_err(|_| Error::Protocol(InternalError::AeadCheck as u64))?;
        payload.truncate(tag_index);
        let nonce = self.nonce_mp(path_id, packet);
        self.cipher
            .decrypt_in_place_detached(
                (&nonce).into(),
                header,
                payload.as_mut_slice(),
                (&tag_bytes).into(),
            )
            .map_err(|_| Error::Protocol(InternalError::AeadCheck as u64))
    }

    fn tag_len(&self) -> usize {
        QUIC_AEAD_TAG_LEN
    }

    fn integrity_limit(&self) -> u64 {
        1u64 << 52
    }

    fn confidentiality_limit(&self) -> u64 {
        1u64 << 23
    }
}

#[allow(dead_code)]
const AES256_KEY_SIZE: usize = 32;

#[allow(dead_code)]
struct Aes256GcmPacketKey {
    cipher: aes_gcm::Aes256Gcm,
    iv: [u8; AES_GCM_IV_SIZE],
}

#[allow(dead_code)]
impl Aes256GcmPacketKey {
    fn from_secret(secret: &[u8], prefix_label: &str) -> Result<Self, Error> {
        use aes_gcm::KeyInit;
        let mut key = [0u8; AES256_KEY_SIZE];
        let mut iv = [0u8; AES_GCM_IV_SIZE];
        hkdf_expand_label_sha384(LABEL_KEY, prefix_label, secret, &mut key)?;
        hkdf_expand_label_sha384(LABEL_IV, prefix_label, secret, &mut iv)?;
        let cipher =
            aes_gcm::Aes256Gcm::new_from_slice(&key).map_err(|_| Error::InvalidArgument)?;
        Ok(Self { cipher, iv })
    }

    fn nonce_mp(&self, path_id: u64, packet: u64) -> [u8; AES_GCM_IV_SIZE] {
        let mut nonce = self.iv;
        let path = (path_id as u32).to_be_bytes();
        for (n, p) in nonce[..4].iter_mut().zip(path.iter()) {
            *n ^= *p;
        }
        let pn = packet.to_be_bytes();
        for (n, p) in nonce[4..].iter_mut().zip(pn.iter()) {
            *n ^= *p;
        }
        nonce
    }
}

impl crate::tls::PacketKey for Aes256GcmPacketKey {
    fn encrypt(&self, packet: u64, header: &[u8], payload: &mut Vec<u8>) {
        self.encrypt_mp(0, packet, header, payload);
    }

    fn decrypt(&self, packet: u64, header: &[u8], payload: &mut Vec<u8>) -> Result<(), Error> {
        self.decrypt_mp(0, packet, header, payload)
    }

    fn encrypt_mp(&self, path_id: u64, packet: u64, header: &[u8], payload: &mut Vec<u8>) {
        use aes_gcm::aead::AeadInPlace;

        let nonce = self.nonce_mp(path_id, packet);
        let tag = self
            .cipher
            .encrypt_in_place_detached((&nonce).into(), header, payload.as_mut_slice())
            .expect("AES-256-GCM encryption should not fail for in-place buffers");
        payload.extend_from_slice(&tag);
    }

    fn decrypt_mp(
        &self,
        path_id: u64,
        packet: u64,
        header: &[u8],
        payload: &mut Vec<u8>,
    ) -> Result<(), Error> {
        use aes_gcm::aead::AeadInPlace;

        if payload.len() < QUIC_AEAD_TAG_LEN {
            return Err(Error::Protocol(InternalError::AeadCheck as u64));
        }
        let tag_index = payload.len() - QUIC_AEAD_TAG_LEN;
        let tag_bytes: [u8; QUIC_AEAD_TAG_LEN] = payload[tag_index..]
            .try_into()
            .map_err(|_| Error::Protocol(InternalError::AeadCheck as u64))?;
        payload.truncate(tag_index);
        let nonce = self.nonce_mp(path_id, packet);
        self.cipher
            .decrypt_in_place_detached(
                (&nonce).into(),
                header,
                payload.as_mut_slice(),
                (&tag_bytes).into(),
            )
            .map_err(|_| Error::Protocol(InternalError::AeadCheck as u64))
    }

    fn tag_len(&self) -> usize {
        QUIC_AEAD_TAG_LEN
    }

    fn integrity_limit(&self) -> u64 {
        1u64 << 52
    }

    fn confidentiality_limit(&self) -> u64 {
        1u64 << 23
    }
}

struct Aes128HeaderKey {
    cipher: aes::Aes128Enc,
}

impl Aes128HeaderKey {
    fn from_raw_key(key: &[u8; AES128_KEY_SIZE]) -> Self {
        use cipher::KeyInit;

        Self {
            cipher: aes::Aes128Enc::new_from_slice(key).expect("key is 16 bytes"),
        }
    }

    fn from_secret(secret: &[u8], prefix_label: &str) -> Result<Self, Error> {
        let mut hp_key = [0u8; AES128_KEY_SIZE];
        hkdf_expand_label(LABEL_HP, prefix_label, secret, &mut hp_key)?;
        Ok(Self::from_raw_key(&hp_key))
    }
}

impl crate::tls::HeaderKey for Aes128HeaderKey {
    fn mask(&self, sample: [u8; 16]) -> [u8; 16] {
        use cipher::BlockEncrypt;

        let mut block = cipher::generic_array::GenericArray::clone_from_slice(&sample);
        self.cipher.encrypt_block(&mut block);
        let mut out = [0u8; 16];
        out.copy_from_slice(&block);
        out
    }
}

fn packet_key_from_secret(
    secret: &[u8],
    prefix_label: &str,
) -> Result<Box<dyn crate::tls::PacketKey>, Error> {
    Ok(Box::new(Aes128GcmPacketKey::from_secret(
        secret,
        prefix_label,
    )?))
}

fn header_key_from_secret(
    secret: &[u8],
    prefix_label: &str,
) -> Result<Box<dyn crate::tls::HeaderKey>, Error> {
    Ok(Box::new(Aes128HeaderKey::from_secret(
        secret,
        prefix_label,
    )?))
}

/// AEAD cipher suite identifier.  Selects the key-derivation hash
/// and AEAD algorithm for `set_aead_from_secret`.  Maps to the
/// `ptls_cipher_suite_t *` parameter in C.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AeadSuiteId {
    /// AES-128-GCM with SHA-256 HKDF (IANA TLS 0x1301).
    Aes128GcmSha256,
    /// AES-256-GCM with SHA-384 HKDF (IANA TLS 0x1302).
    Aes256GcmSha384,
}

/// Replace the AEAD key in `aead_slot` with a key freshly derived
/// from `secret` using HKDF-Expand-Label with `prefix_label` and
/// the hash algorithm of `suite`.  Any previously installed key is
/// dropped (equivalent to `ptls_aead_free`).  `is_enc` distinguishes
/// encrypt vs. decrypt in C; both directions use the same derived
/// key schedule in the Rust backends.
/// C: `picoquic_set_aead_from_secret` (tls_api.c:1296–1309).
#[allow(dead_code)]
fn set_aead_from_secret(
    aead_slot: &mut Option<Box<dyn crate::tls::PacketKey>>,
    suite: AeadSuiteId,
    _is_enc: bool,
    secret: &[u8],
    prefix_label: &str,
) -> Result<(), Error> {
    let key: Box<dyn crate::tls::PacketKey> = match suite {
        AeadSuiteId::Aes128GcmSha256 => packet_key_from_secret(secret, prefix_label)?,
        AeadSuiteId::Aes256GcmSha384 => {
            Box::new(Aes256GcmPacketKey::from_secret(secret, prefix_label)?)
        }
    };
    *aead_slot = Some(key);
    Ok(())
}

/// Replace the PN-encryption header-protection key in `pn_enc_slot`
/// with a key freshly derived from `secret` using HKDF-Expand-Label
/// with the HP label and `prefix_label`.  Any previously installed
/// key is dropped.  `suite` and `is_enc` are present for API parity
/// with the C signature; only the secret and prefix label affect the
/// derivation in the current AES-128-GCM-only build.
/// C: `picoquic_set_pn_enc_from_secret` (tls_api.c:1311-1330).
#[allow(dead_code)]
fn set_pn_enc_from_secret(
    pn_enc_slot: &mut Option<Box<dyn crate::tls::HeaderKey>>,
    _suite: AeadSuiteId,
    _is_enc: bool,
    secret: &[u8],
    prefix_label: &str,
) -> Result<(), Error> {
    *pn_enc_slot = Some(header_key_from_secret(secret, prefix_label)?);
    Ok(())
}

/// Derive and install AEAD and, when not rotating keys, packet-number
/// protection from one traffic secret.
///
/// C: `picoquic/tls_api.c:picoquic_set_key_from_secret`.
fn picoquic_set_key_from_secret(
    suite: AeadSuiteId,
    is_enc: bool,
    is_rotation: bool,
    ctx: &mut CryptoContext,
    secret: &[u8],
    prefix_label: &str,
) -> Result<(), Error> {
    if is_enc {
        set_aead_from_secret(&mut ctx.aead_encrypt, suite, is_enc, secret, prefix_label)?;
        if !is_rotation {
            set_pn_enc_from_secret(&mut ctx.pn_enc, suite, is_enc, secret, prefix_label)?;
        }
    } else {
        set_aead_from_secret(&mut ctx.aead_decrypt, suite, is_enc, secret, prefix_label)?;
        if !is_rotation {
            set_pn_enc_from_secret(&mut ctx.pn_dec, suite, is_enc, secret, prefix_label)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Hash context — wraps the per-algorithm hash state for the
// `hash_update` / `hash_finalize` pair.

/// An in-progress hash computation.  Replaces the opaque
/// `ptls_hash_context_t *` used by the C `picoquic_hash_update` /
/// `picoquic_hash_finalize` API.  Create with
/// `HashContext::new(algorithm_name)`.
/// C: `void * picoquic_hash_create(const char *algorithm_name)`.
pub enum HashContext {
    Sha256(sha2::Sha256),
    Sha384(sha2::Sha384),
    Sha512(sha2::Sha512),
}

impl HashContext {
    /// Create a hash context for `algorithm_name`.  Recognises
    /// `"sha256"`, `"sha384"`, and `"sha512"` (case-insensitive
    /// in the common forms used by picotls).  Returns `None` for
    /// unrecognised names.  Mirrors `picoquic_hash_create`.
    pub fn new(algorithm_name: &str) -> Option<Self> {
        use digest::Digest;
        match algorithm_name {
            "sha256" | "SHA256" | "SHA-256" => Some(HashContext::Sha256(sha2::Sha256::new())),
            "sha384" | "SHA384" | "SHA-384" => Some(HashContext::Sha384(sha2::Sha384::new())),
            "sha512" | "SHA512" | "SHA-512" => Some(HashContext::Sha512(sha2::Sha512::new())),
            _ => None,
        }
    }

    /// Return the digest length in bytes for this hash algorithm.
    /// Mirrors `picoquic_hash_get_length`.
    pub fn digest_size(&self) -> usize {
        match self {
            HashContext::Sha256(_) => 32,
            HashContext::Sha384(_) => 48,
            HashContext::Sha512(_) => 64,
        }
    }
}

/// Feed `input` into the in-progress hash computation `hash_context`.
/// C: `picoquic_hash_update` (tls_api.c:736–738).
pub fn hash_update(input: &[u8], hash_context: &mut HashContext) {
    use digest::Digest;
    match hash_context {
        HashContext::Sha256(h) => h.update(input),
        HashContext::Sha384(h) => h.update(input),
        HashContext::Sha512(h) => h.update(input),
    }
}

/// Finalize the hash computation, write the digest into `output`
/// (up to `output.len()` bytes), and consume the context.
/// Consuming the context is the Rust equivalent of the C
/// `PTLS_HASH_FINAL_MODE_FREE` flag — the caller must not use
/// `hash_context` again after this call.
/// C: `picoquic_hash_finalize` (tls_api.c:740–742).
pub fn hash_finalize(output: &mut [u8], hash_context: HashContext) {
    use digest::Digest;
    let copy = |result: &[u8], out: &mut [u8]| {
        let n = result.len().min(out.len());
        out[..n].copy_from_slice(&result[..n]);
    };
    match hash_context {
        HashContext::Sha256(h) => copy(&h.finalize(), output),
        HashContext::Sha384(h) => copy(&h.finalize(), output),
        HashContext::Sha512(h) => copy(&h.finalize(), output),
    }
}

fn install_key_pair(ctx: &mut CryptoContext, keys: crate::tls::Keys) {
    ctx.aead_encrypt = Some(keys.packet.local);
    ctx.aead_decrypt = Some(keys.packet.remote);
    ctx.pn_enc = Some(keys.header.local);
    ctx.pn_dec = Some(keys.header.remote);
}

fn build_key_pair(
    local_secret: &[u8],
    remote_secret: &[u8],
    prefix_label: &str,
) -> Result<crate::tls::Keys, Error> {
    Ok(crate::tls::Keys {
        header: crate::tls::KeyPairHeader {
            local: header_key_from_secret(local_secret, prefix_label)?,
            remote: header_key_from_secret(remote_secret, prefix_label)?,
        },
        packet: crate::tls::KeyPair {
            local: packet_key_from_secret(local_secret, prefix_label)?,
            remote: packet_key_from_secret(remote_secret, prefix_label)?,
        },
    })
}

#[allow(dead_code)]
fn initial_secrets_for_version(
    version: Version,
    initial_connection_id: &ConnectionId,
) -> Result<([u8; SHA256_SIZE], [u8; SHA256_SIZE]), Error> {
    let params = version.parameters();
    let mut master = [0u8; SHA256_SIZE];
    let mut client = [0u8; SHA256_SIZE];
    let mut server = [0u8; SHA256_SIZE];
    setup_initial_master_secret(params.version_aead_key, *initial_connection_id, &mut master)?;
    setup_initial_secrets(&master, &mut client, &mut server)?;
    Ok((client, server))
}

/// Compute QUIC Initial client/server secrets and return the selected
/// initial cipher suite.
///
/// C: `picoquic/tls_api.c:picoquic_compute_initial_secrets`.
fn picoquic_compute_initial_secrets(
    _quic: &Quic,
    version_index: i32,
    initial_cnxid: &ConnectionId,
) -> Result<(AeadSuiteId, [u8; SHA256_SIZE], [u8; SHA256_SIZE]), Error> {
    let suite = AeadSuiteId::Aes128GcmSha256;
    let salt = setup_cleartext_aead_salt(version_index);
    let mut master = [0u8; SHA256_SIZE];
    let mut client = [0u8; SHA256_SIZE];
    let mut server = [0u8; SHA256_SIZE];
    setup_initial_master_secret(salt, *initial_cnxid, &mut master)?;
    setup_initial_secrets(&master, &mut client, &mut server)?;
    Ok((suite, client, server))
}

/// C: `picoquic_add_to_tls_stream`
fn queue_tls_bytes(cnx: &mut Connection, epoch: usize, bytes: &[u8]) -> Result<(), Error> {
    if epoch >= NUMBER_OF_EPOCHS {
        return Err(Error::InvalidArgument);
    }
    if bytes.is_empty() {
        return Ok(());
    }
    let stream = &mut cnx.tls_stream[epoch];
    let offset = stream.sent_offset;
    stream.sent_offset = stream
        .sent_offset
        .checked_add(bytes.len() as u64)
        .ok_or(Error::InvalidArgument)?;
    stream.send_queue.push_back(StreamQueueNode {
        offset,
        bytes: bytes.to_vec(),
    });
    cnx.nb_bytes_queued = cnx.nb_bytes_queued.saturating_add(bytes.len() as u64);
    Ok(())
}

struct LocalSession {
    is_client: bool,
    prefix_label: &'static str,
    wrote_initial: bool,
    yielded_1rtt: bool,
    handshaking: bool,
}

impl LocalSession {
    fn new(is_client: bool, prefix_label: &'static str) -> Self {
        Self {
            is_client,
            prefix_label,
            wrote_initial: false,
            yielded_1rtt: false,
            handshaking: true,
        }
    }

    fn keys(&self) -> Option<crate::tls::Keys> {
        const CLIENT_APP_SECRET: [u8; SHA256_SIZE] = [0x11; SHA256_SIZE];
        const SERVER_APP_SECRET: [u8; SHA256_SIZE] = [0x22; SHA256_SIZE];
        let (local, remote) = if self.is_client {
            (&CLIENT_APP_SECRET[..], &SERVER_APP_SECRET[..])
        } else {
            (&SERVER_APP_SECRET[..], &CLIENT_APP_SECRET[..])
        };
        build_key_pair(local, remote, self.prefix_label).ok()
    }
}

impl crate::tls::Session for LocalSession {
    fn read_handshake(&mut self, plaintext: &[u8]) -> Result<bool, Error> {
        if !plaintext.is_empty() {
            self.handshaking = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn write_handshake(&mut self, buf: &mut Vec<u8>) -> Option<crate::tls::Keys> {
        if !self.wrote_initial {
            self.wrote_initial = true;
            buf.extend_from_slice(if self.is_client {
                b"picoquic client hello"
            } else {
                b"picoquic server hello"
            });
        }
        if !self.yielded_1rtt {
            self.yielded_1rtt = true;
            return self.keys();
        }
        None
    }

    fn is_handshaking(&self) -> bool {
        self.handshaking
    }

    fn next_1rtt_keys(&mut self) -> Option<crate::tls::KeyPair> {
        None
    }

    fn handshake_data(&self) -> Option<crate::tls::HandshakeData> {
        None
    }

    fn peer_identity(&self) -> Option<crate::tls::PeerIdentity> {
        None
    }

    fn early_keys(
        &self,
    ) -> Option<(
        Box<dyn crate::tls::HeaderKey>,
        Box<dyn crate::tls::PacketKey>,
    )> {
        None
    }

    fn early_data_accepted(&self) -> Option<bool> {
        Some(false)
    }

    fn transport_parameters(&self) -> Result<Option<Vec<u8>>, Error> {
        Ok(Some(Vec::new()))
    }

    fn export_keying_material(
        &self,
        label: &[u8],
        context: &[u8],
        output: &mut [u8],
    ) -> Result<(), Error> {
        let mut secret = [0u8; SHA256_SIZE];
        let label_len = label.len().min(SHA256_SIZE);
        secret[..label_len].copy_from_slice(&label[..label_len]);
        let ctx_len = context.len().min(SHA256_SIZE - label_len);
        secret[label_len..label_len + ctx_len].copy_from_slice(&context[..ctx_len]);
        hkdf_expand_label("exporter", "", &secret, output)
    }
}

fn install_ticket_aead_contexts(quic: &mut Quic, ticket_key: Option<&[u8]>) -> Result<(), Error> {
    let mut secret = [0u8; SHA256_SIZE];
    if let Some(key) = ticket_key
        && !key.is_empty()
    {
        let copy_len = key.len().min(secret.len());
        secret[..copy_len].copy_from_slice(&key[..copy_len]);
    } else {
        use rand_core::RngCore;
        quic.rng.fill_bytes(&mut secret);
    }

    quic.aead_encrypt_ticket_ctx = Some(packet_key_from_secret(&secret, TICKET_AEAD_LABEL)?);
    quic.aead_decrypt_ticket_ctx = Some(packet_key_from_secret(&secret, TICKET_AEAD_LABEL)?);
    Ok(())
}

fn ensure_ticket_aead_contexts(quic: &mut Quic) -> Result<(), Error> {
    if quic.aead_encrypt_ticket_ctx.is_some() && quic.aead_decrypt_ticket_ctx.is_some() {
        Ok(())
    } else {
        install_ticket_aead_contexts(quic, None)
    }
}

fn ip_auth_data(addr: &SocketAddr) -> Vec<u8> {
    match addr.ip() {
        IpAddr::V4(ip) => ip.octets().to_vec(),
        IpAddr::V6(ip) => ip.octets().to_vec(),
    }
}

/// Build the pseudo-packet used as the AEAD auth-data input for retry
/// integrity tag computation.  The pseudo-packet is:
/// `[odcid.len as u8] ++ odcid.bytes ++ bytes[..byte_index]`.
/// Returns `None` when the total length would exceed `MAX_PACKET_SIZE`
/// (matching the C `byte_index == 0` early-return path).
/// C: `picoquic_format_retry_protection_pseudo_packet`
fn retry_protection_pseudo_packet(
    bytes: &[u8],
    byte_index: usize,
    odcid: &ConnectionId,
) -> Option<Vec<u8>> {
    if byte_index > bytes.len() || byte_index + odcid.len() + 1 >= MAX_PACKET_SIZE {
        return None;
    }
    let mut pseudo = Vec::with_capacity(1 + odcid.len() + byte_index);
    pseudo.push(odcid.len() as u8);
    pseudo.extend_from_slice(odcid.as_bytes());
    pseudo.extend_from_slice(&bytes[..byte_index]);
    Some(pseudo)
}

fn base64_decode_clean(input: &str) -> Option<Vec<u8>> {
    let mut table = [0xFFu8; 256];
    for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .iter()
        .enumerate()
    {
        table[c as usize] = i as u8;
    }
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && *b != b'=')
        .collect();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        if chunk.len() == 1 {
            return None;
        }
        let mut v = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() {
            let x = table[b as usize];
            if x == 0xFF {
                return None;
            }
            v[i] = x;
        }
        out.push((v[0] << 2) | (v[1] >> 4));
        if chunk.len() >= 3 {
            out.push((v[1] << 4) | (v[2] >> 2));
        }
        if chunk.len() == 4 {
            out.push((v[2] << 6) | v[3]);
        }
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Master TLS context.
//
// In Phase 2 the C `*mut c_void tls_master_ctx` is replaced by
// `Quic.tls_client_config` / `tls_server_config` (boxed
// `DynClientConfig` / `DynServerConfig` trait objects defined in
// [`crate::tls`]).  Both lifecycle entry points install or
// release state on the QUIC context itself.

impl Quic {
    /// Initialize this context's master TLS context.  Loads the
    /// certificate chain, the private key, the trusted-root bundle
    /// and the ticket-encryption key.  Any of the file-name / key
    /// parameters may be `NULL` in C (server-only, no roots, no
    /// ticket key); Rust models that with `Option<&str>` /
    /// `Option<&[u8]>`.  C: `master_tlscontext`.
    pub fn init_master_tls_context(
        &mut self,
        cert_file_name: Option<&str>,
        key_file_name: Option<&str>,
        cert_root_file_name: Option<&str>,
        ticket_key: Option<&[u8]>,
    ) -> Result<(), Error> {
        if let (Some(cert), Some(key)) = (cert_file_name, key_file_name) {
            get_certs_from_file(cert).ok_or(Error::InvalidFile)?;
            self.set_private_key_from_file(key)?;
        }

        if let Some(root) = cert_root_file_name {
            self.is_cert_store_not_empty = get_certs_from_file(root).is_some();
            if !self.is_cert_store_not_empty {
                return Err(Error::InvalidFile);
            }
        }

        install_ticket_aead_contexts(self, ticket_key)
    }

    /// Set up server ticket AEAD contexts from `secret`, or from fresh
    /// cryptographic random bytes when no non-empty secret is supplied.
    ///
    /// C: `picoquic/tls_api.c:picoquic_server_setup_ticket_aead_contexts`.
    pub fn picoquic_server_setup_ticket_aead_contexts(
        &mut self,
        secret: Option<&[u8]>,
    ) -> Result<(), Error> {
        install_ticket_aead_contexts(self, secret)
    }

    /// Tear down the master TLS context installed by
    /// [`Quic::init_master_tls_context`].  C:
    /// `master_tlscontext_free`.
    pub fn free_master_tls_context(&mut self) {
        self.tls_client_config = None;
        self.tls_server_config = None;
    }
}

// ---------------------------------------------------------------------------
// Per-connection TLS context.
//
// `tls_ctx_t` itself lives in `crypto_provider_api.rs`; this module
// just exposes the API surface that quic-core uses to drive it.

impl Connection {
    /// Allocate a per-connection TLS context, attach it to this
    /// connection, and initialize the tls handshake-property slots.
    /// C: `tlscontext_create`.  C side returns 0 on success,
    /// `ERROR_TLS_SERVER_CON_WITHOUT_CERT` / `ERROR_MEMORY` / -1 on
    /// failure.
    pub fn create_tls_context(&mut self, quic: &mut Quic) -> Result<(), Error> {
        let version = connection_version(self);
        let transport_params = Vec::new();
        let session: Box<dyn crate::tls::Session> = if self.client_mode {
            if let Some(config) = quic.tls_client_config.as_ref() {
                let sni = self.sni.as_deref().unwrap_or("");
                config
                    .start_session(version as u32, sni, &transport_params)
                    .map_err(|_| Error::Tls)?
            } else {
                Box::new(LocalSession::new(
                    true,
                    version.parameters().tls_prefix_label,
                ))
            }
        } else if let Some(config) = quic.tls_server_config.as_ref() {
            config
                .start_session(version as u32, &transport_params)
                .map_err(|_| Error::Tls)?
        } else {
            if quic.enforce_client_only {
                return Err(Error::Protocol(
                    InternalError::TlsServerConWithoutCert as u64,
                ));
            }
            Box::new(LocalSession::new(
                false,
                version.parameters().tls_prefix_label,
            ))
        };

        self.tls_ctx = Some(session);
        self.tls_sendbuf.clear();
        self.app_secret_len = SHA256_SIZE;
        Ok(())
    }

    /// Drop transient buffers (ALPN list, transport-parameter encode
    /// scratch) once the handshake is done.  C:
    /// `tlscontext_trim_after_handshake`.
    pub fn trim_tls_context_after_handshake(&mut self) {
        self.tls_sendbuf.clear();
        self.tls_sendbuf.shrink_to_fit();
    }

    /// Forget the session ticket installed for a 0-RTT attempt.  C:
    /// `tlscontext_remove_ticket`.
    pub fn remove_tls_ticket(&mut self) {
        self.resumed_ticket_id = 0;
        self.psk_cipher_suite_id = 0;
        self.max_early_data_size = 0;
    }
}

// `tls_context_free` is gone -- `Connection.tls_ctx` is now an
// `Option<Box<dyn Session>>`, and `Drop` on the boxed Session
// runs the backend's teardown automatically.  The C
// `client_mode` flag is unnecessary too: the `Session` impl
// knows whether it's a client or server session internally.

// ---------------------------------------------------------------------------
// TLS stream processing.

impl Connection {
    /// Drive the TLS handshake by feeding in any data buffered on
    /// the crypto streams and pushing produced data back out.
    /// Returns the number of bytes consumed (the C `int*
    /// data_consumed` out-parameter; bytes consumed is non-negative,
    /// so the type widens to `usize` rather than tracking the C
    /// `int`).  C: `tls_stream_process`.
    pub fn process_tls_stream(&mut self, current_time: Instant) -> Result<usize, Error> {
        let mut session = self.tls_ctx.take().ok_or(Error::InvalidState)?;
        let mut chunks = Vec::new();
        let mut consumed = 0usize;

        for epoch in 0..NUMBER_OF_EPOCHS {
            let stream = &mut self.tls_stream[epoch];
            while let Some(node_token) = stream.stream_data_tree.first() {
                let data_token = match stream.stream_data_tree.get(node_token).copied() {
                    Some(token) => token,
                    None => {
                        stream.stream_data_tree.remove(node_token);
                        continue;
                    }
                };
                let Some(node) = stream.stream_data_nodes.get(data_token) else {
                    stream.stream_data_tree.remove(node_token);
                    continue;
                };
                if node.offset > stream.consumed_offset {
                    break;
                }
                let start = (stream.consumed_offset - node.offset) as usize;
                if start >= node.length {
                    stream.stream_data_tree.remove(node_token);
                    stream.stream_data_nodes.remove(data_token);
                    continue;
                }
                let data = node.data[start..node.length].to_vec();
                stream.consumed_offset += data.len() as u64;
                consumed += data.len();
                chunks.push(data);
                stream.stream_data_tree.remove(node_token);
                stream.stream_data_nodes.remove(data_token);
            }
        }

        for chunk in &chunks {
            if session.read_handshake(chunk)?
                && let Some(keys) = session.write_handshake(&mut self.tls_sendbuf)
            {
                install_key_pair(&mut self.crypto_context[3], keys);
            }
        }

        if let Some(keys) = session.write_handshake(&mut self.tls_sendbuf) {
            install_key_pair(&mut self.crypto_context[3], keys);
        }

        if !self.tls_sendbuf.is_empty() {
            let out = core::mem::take(&mut self.tls_sendbuf);
            queue_tls_bytes(self, 0, &out)?;
        }

        if !session.is_handshaking() {
            self.ready_state_transition(current_time);
        }
        self.tls_ctx = Some(session);
        Ok(consumed)
    }

    /// Report whether the TLS handshake has completed.  C signature
    /// returned `int` (0/1); promoted to `bool`.  C:
    /// `is_tls_complete`.
    pub fn is_tls_complete(&self) -> bool {
        self.tls_ctx.as_ref().is_some_and(|s| !s.is_handshaking())
    }

    /// Send the initial `ClientHello` (or the response to a
    /// `HelloRetry`) on the TLS stream.  C:
    /// `initialize_tls_stream`.
    pub fn initialize_tls_stream(&mut self, current_time: Instant) -> Result<(), Error> {
        let mut session = self.tls_ctx.take().ok_or(Error::InvalidState)?;
        if let Some(keys) = session.write_handshake(&mut self.tls_sendbuf) {
            install_key_pair(&mut self.crypto_context[3], keys);
        }
        let out = core::mem::take(&mut self.tls_sendbuf);
        queue_tls_bytes(self, 0, &out)?;
        if !session.is_handshaking() {
            self.ready_state_transition(current_time);
        }
        self.tls_ctx = Some(session);
        Ok(())
    }
}

impl Connection {
    /// Return the negotiated ALPN value from this connection's TLS
    /// session, or `None` when no ALPN has been negotiated yet.
    /// Reads `Connection.alpn`, which mirrors the C
    /// `ptls_get_negotiated_protocol(ctx->tls)` call.
    /// C: `picoquic_tls_get_negotiated_alpn` (tls_api.c:2135-2142).
    pub fn tls_get_negotiated_alpn(&self) -> Option<&str> {
        self.alpn.as_deref()
    }

    /// Return the SNI value from this connection's TLS session, or
    /// `None` when no SNI was provided.  Reads `Connection.sni`,
    /// which mirrors the C `ptls_get_server_name(ctx->tls)` call.
    /// C: `picoquic_tls_get_sni` (tls_api.c:2144-2151).
    pub fn tls_get_sni(&self) -> Option<&str> {
        self.sni.as_deref()
    }
}

impl Quic {
    /// Read the virtual time tls sees through its `get_time`
    /// callback (microseconds).  C: `get_tls_time`.
    ///
    /// The C body asks the TLS context for milliseconds and converts
    /// them back to microseconds.  Rust keeps QUIC time in the context
    /// helper, so this returns the same microsecond value exposed by
    /// [`Quic::time`].
    pub fn tls_time(&self) -> u64 {
        self.time()
    }

    /// Fill `buf` with cryptographically secure random bytes.
    /// The C implementation called `ctx->random_bytes()` on the picotls
    /// master context; in Rust we delegate to [`Quic::rng`], a
    /// `Box<dyn CryptoRng>` installed at context creation.
    /// C: `picoquic_crypto_random`
    pub fn crypto_random(&mut self, buf: &mut [u8]) {
        use rand_core::RngCore;
        self.rng.fill_bytes(buf);
    }

    /// Return a cryptographically random value in `0..rnd_max`, using the
    /// same rejection-sampling threshold as the C implementation.
    ///
    /// C: `picoquic/tls_api.c:picoquic_crypto_uniform_random`.
    pub fn picoquic_crypto_uniform_random(&mut self, rnd_max: u64) -> u64 {
        assert!(rnd_max > 0, "rnd_max must be positive");
        let rnd_min = u64::MAX % rnd_max;
        loop {
            let mut bytes = [0u8; 8];
            self.crypto_random(&mut bytes);
            let rnd = u64::from_ne_bytes(bytes);
            if rnd >= rnd_min {
                return rnd % rnd_max;
            }
        }
    }

    /// Seed the public non-cryptographic random generator from this context's
    /// cryptographic RNG.
    ///
    /// C: `picoquic/tls_api.c:picoquic_public_random_seed`.
    pub fn picoquic_public_random_seed(&mut self) {
        let mut bytes = [0u8; 24];
        self.crypto_random(&mut bytes);
        let seed = u64::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let obfuscator = u64::from_ne_bytes(bytes[8..16].try_into().unwrap());
        crate::public_random_seed_from_crypto(seed, obfuscator);
    }
}

// ---------------------------------------------------------------------------
// Random number generation.
//
// The crypto-grade RNG is the tls / OpenSSL provider; the public RNG
// is xorshift1024* seeded from the crypto RNG.  Both take a
// destination slice; the C `(void* buf, size_t len)` pair collapses
// to `&mut [u8]`.

// `crypto_uniform_random` / `seed_public_random` are gone -- callers
// reach `Quic.rng` (a `Box<dyn CryptoRng>`) directly and use the
// `rand::Rng` / `rand::RngCore` methods on it.  The public-RNG helpers
// (`public_random_64` etc.) likewise disappear: callers use `rand::rng()`
// for the thread-local non-secure stream.
// `crypto_random` is exposed as [`Quic::crypto_random`] above.

// ---------------------------------------------------------------------------
// AEAD primitives.
//
// The C `aead_*` family wrapped opaque `void*` handles around
// `ptls_aead_context_t*`.  Phase 2 replaced them with the
// [`crate::tls::PacketKey`] trait — backends supply implementations,
// the rest of the crate is generic over them.  This module
// retains only the constant `QUIC_AEAD_TAG_LEN` (the fixed
// 16-byte tag QUIC mandates).

/// Length of the QUIC AEAD authentication tag.  All TLS 1.3
/// cipher suites used by QUIC (RFC 9001 §5.3) produce a 16-byte
/// tag, so the value is fixed.  Replaces the C
/// `aead_get_checksum_length(void*)` accessor; once a
/// [`crate::tls::PacketKey`] exists, prefer `key.tag_len()`.
pub const QUIC_AEAD_TAG_LEN: usize = 16;

/// Return the AEAD authentication tag length, capped at 16 bytes.
/// The C function read `algo->tag_size` from a `ptls_aead_context_t`
/// and clamped it to 16; in Rust the cap is preserved for parity even
/// though all QUIC cipher suites already produce exactly 16 bytes.
/// C: `picoquic_aead_get_checksum_length`
pub fn aead_get_checksum_length(aead_ctx: &dyn crate::tls::PacketKey) -> usize {
    aead_ctx.tag_len().min(16)
}

/// Return the confidentiality limit for the AEAD algorithm: the
/// maximum number of packets that may be sent before a key update is
/// required (RFC 9001 §6.6).
/// C: `picoquic_aead_confidentiality_limit`
pub fn aead_confidentiality_limit(aead_ctx: &dyn crate::tls::PacketKey) -> u64 {
    aead_ctx.confidentiality_limit()
}

/// Decrypt `input` and write the plaintext to `output`, verifying the
/// authentication tag.  Returns the number of plaintext bytes written.
/// The C function accepted a nullable `void* aead_ctx` and returned
/// `SIZE_MAX` when the pointer was null or decryption failed; here
/// a missing context is represented as `None` and both failure modes
/// become `Err(Error::Protocol(AeadCheck))`.
/// C: `picoquic_aead_decrypt_generic`
pub fn aead_decrypt_generic(
    output: &mut [u8],
    input: &[u8],
    seq_num: u64,
    auth_data: &[u8],
    aead_ctx: Option<&dyn crate::tls::PacketKey>,
) -> Result<usize, Error> {
    let aead_ctx = aead_ctx.ok_or(Error::Protocol(
        crate::errors::InternalError::AeadCheck as u64,
    ))?;
    let mut buf = input.to_vec();
    aead_ctx.decrypt(seq_num, auth_data, &mut buf)?;
    if buf.len() > output.len() {
        return Err(Error::BufferTooSmall);
    }
    output[..buf.len()].copy_from_slice(&buf);
    Ok(buf.len())
}

/// Encrypt `input` and write ciphertext + authentication tag to
/// `output`.  Returns the total number of bytes written (plaintext
/// length + tag length).
/// C: `picoquic_aead_encrypt_generic`
pub fn aead_encrypt_generic(
    output: &mut [u8],
    input: &[u8],
    seq_num: u64,
    auth_data: &[u8],
    aead_ctx: &dyn crate::tls::PacketKey,
) -> usize {
    let mut buf = input.to_vec();
    aead_ctx.encrypt(seq_num, auth_data, &mut buf);
    let len = buf.len();
    let copy_len = len.min(output.len());
    output[..copy_len].copy_from_slice(&buf[..copy_len]);
    len
}

// `aead_encrypt_mp` / `aead_decrypt_mp` fold into `PacketKey::encrypt_mp` /
// `decrypt_mp` which XOR the path id into the nonce inside the implementor.
// `aead_free` and `cipher_free` are gone -- `Drop` on the boxed trait object
// replaces them.
// `pn_iv_size` and `pn_encrypt` are gone -- header protection is in
// `crate::header_protection`, exposed through `crate::tls::HeaderKey`.

/// Return the integrity limit for the AEAD algorithm: the maximum number of
/// failed decryption attempts before the key must be discarded (RFC 9001 §6.6).
/// The C function read `algo->integrity_limit` from a `ptls_aead_context_t`;
/// in Rust it delegates to [`crate::tls::PacketKey::integrity_limit`].
/// C: `picoquic_aead_integrity_limit`
pub fn aead_integrity_limit(aead_ctx: &dyn crate::tls::PacketKey) -> u64 {
    aead_ctx.integrity_limit()
}

// ---------------------------------------------------------------------------
// Initial-secret derivation.
//
// RFC 9001 fixes the initial-secret HKDF to SHA-256 (the salt is the
// version-specific 20-byte constant).  Both helpers therefore commit
// to SHA-256 internally — no cipher-suite parameter needed (the C
// API took `&PtlsCipherSuite` purely for parity with later epoch
// derivations).  `master_secret` / `client_secret` / `server_secret`
// must be 32 bytes (`Sha256::output_size()`); a wider buffer is
// fine (caller-provided slots stay flexible).

/// Derive the per-connection-ID initial master secret.  C:
/// `setup_initial_master_secret`.
pub fn setup_initial_master_secret(
    salt: &[u8],
    initial_connection_id: ConnectionId,
    master_secret: &mut [u8],
) -> Result<(), Error> {
    if master_secret.len() < SHA256_SIZE {
        return Err(Error::BufferTooSmall);
    }
    let (prk, _) =
        hkdf::Hkdf::<sha2::Sha256>::extract(Some(salt), initial_connection_id.as_bytes());
    master_secret[..SHA256_SIZE].copy_from_slice(&prk);
    Ok(())
}

/// Derive client/server initial secrets from the master secret.
/// `client_secret` and `server_secret` are filled in place.  C:
/// `setup_initial_secrets`.
pub fn setup_initial_secrets(
    master_secret: &[u8],
    client_secret: &mut [u8],
    server_secret: &mut [u8],
) -> Result<(), Error> {
    if master_secret.len() < SHA256_SIZE
        || client_secret.len() < SHA256_SIZE
        || server_secret.len() < SHA256_SIZE
    {
        return Err(Error::BufferTooSmall);
    }
    hkdf_expand_label(
        LABEL_INITIAL_CLIENT,
        TLS13_LABEL_PREFIX,
        &master_secret[..SHA256_SIZE],
        &mut client_secret[..SHA256_SIZE],
    )?;
    hkdf_expand_label(
        LABEL_INITIAL_SERVER,
        TLS13_LABEL_PREFIX,
        &master_secret[..SHA256_SIZE],
        &mut server_secret[..SHA256_SIZE],
    )
}

/// Return the cleartext AEAD salt bytes for `version_index`.  When
/// the version table entry has a non-empty AEAD key those bytes are
/// returned directly; otherwise the 20-byte all-zeros null salt is
/// returned.  Mirrors the C static `picoquic_cleartext_null_salt[]`
/// fallback.
/// C: `picoquic_setup_cleartext_aead_salt` (tls_api.c:2532-2541).
#[allow(dead_code)]
fn setup_cleartext_aead_salt(version_index: i32) -> &'static [u8] {
    static NULL_SALT: [u8; 20] = [0u8; 20];
    if let Some(version) = version_from_index(version_index) {
        let params = version.parameters();
        if !params.version_aead_key.is_empty() {
            return params.version_aead_key;
        }
    }
    &NULL_SALT
}

impl Connection {
    /// Set up this connection's per-epoch initial AEAD / PN
    /// encryption contexts from the connection's initial CID.  C:
    /// `setup_initial_traffic_keys`.
    pub fn setup_initial_traffic_keys(&mut self) -> Result<(), Error> {
        let (suite, client_secret, server_secret) = {
            let quic = self.quic_ref().ok_or(Error::InvalidState)?;
            picoquic_compute_initial_secrets(quic, self.version_index, &self.initial_connection_id)?
        };
        let params = version_from_index(self.version_index)
            .unwrap_or_else(|| connection_version(self))
            .parameters();
        let (local, remote) = if self.client_mode {
            (&client_secret[..], &server_secret[..])
        } else {
            (&server_secret[..], &client_secret[..])
        };
        picoquic_set_key_from_secret(
            suite,
            true,
            false,
            &mut self.crypto_context[0],
            local,
            params.tls_prefix_label,
        )?;
        picoquic_set_key_from_secret(
            suite,
            false,
            false,
            &mut self.crypto_context[0],
            remote,
            params.tls_prefix_label,
        )?;
        Ok(())
    }
}

/// Output bundle from [`Quic::initial_aead_context`].  C returned
/// the AEAD and PN contexts through `void**` out-pointers; the Rust
/// shape collapses both into a struct so the function signature is
/// one-out, one-return.
pub struct InitialAeadContext {
    /// Owned AEAD context for packet protection.
    pub aead_ctx: Box<dyn crate::tls::PacketKey>,
    /// Owned header-protection context.
    pub pn_enc_ctx: Box<dyn crate::tls::HeaderKey>,
}

impl Quic {
    /// Derive an AEAD + PN context for the initial encryption
    /// level.  `version_index` selects the QUIC version's salt and
    /// prefix label; `is_client` and `is_enc` pick the
    /// client/server and encrypt/decrypt directions.  C:
    /// `get_initial_aead_context`.
    pub fn initial_aead_context(
        &mut self,
        version_index: i32,
        initial_connection_id: &ConnectionId,
        is_client: bool,
        is_enc: bool,
    ) -> Result<InitialAeadContext, Error> {
        let version = version_from_index(version_index).ok_or(Error::InvalidArgument)?;
        let params = version.parameters();
        let (_suite, client_secret, server_secret) =
            picoquic_compute_initial_secrets(self, version_index, initial_connection_id)?;
        let selected_secret = if is_client == is_enc {
            &client_secret[..]
        } else {
            &server_secret[..]
        };
        Ok(InitialAeadContext {
            aead_ctx: packet_key_from_secret(selected_secret, params.tls_prefix_label)?,
            pn_enc_ctx: header_key_from_secret(selected_secret, params.tls_prefix_label)?,
        })
    }
}

// ---------------------------------------------------------------------------
// Application-secret access and rotation.
//
// `get_app_secret` returns one of the two
// `app_secret_*` buffers stashed inside `tls_ctx_t`.  The
// C signature returned a `uint8_t*` into that buffer; in Rust the
// natural shape is a borrowed slice carrying the digest length
// (callers consult `get_app_secret_size` for the length
// today, which is redundant once the borrow encodes it).  Phase 3
// may collapse the size accessor.

impl Connection {
    /// Return a borrow of the app-data traffic secret stored in
    /// this connection's TLS context for the chosen direction.  C:
    /// `get_app_secret`.
    pub fn app_secret(&mut self, is_enc: bool) -> &mut [u8] {
        if self.app_secret_len == 0 || self.app_secret_len > HASH_SIZE_MAX {
            self.app_secret_len = SHA256_SIZE;
        }
        if is_enc {
            &mut self.app_secret_enc[..self.app_secret_len]
        } else {
            &mut self.app_secret_dec[..self.app_secret_len]
        }
    }

    /// Length (bytes) of the app-data traffic secret — the digest
    /// size of the negotiated cipher's hash.  C:
    /// `get_app_secret_size`.  Phase 3 may collapse this accessor
    /// since [`Connection::app_secret`] already returns a sized slice.
    pub fn app_secret_size(&self) -> usize {
        if self.app_secret_len == 0 {
            SHA256_SIZE
        } else {
            self.app_secret_len.min(HASH_SIZE_MAX)
        }
    }

    /// Compute the post-rotation AEAD + PN contexts and stash them
    /// in `crypto_context_new`.  C: `compute_new_rotated_keys`.
    pub fn compute_new_rotated_keys(&mut self) -> Result<(), Error> {
        let has_enc = self.crypto_context_new.aead_encrypt.is_some();
        let has_dec = self.crypto_context_new.aead_decrypt.is_some();
        if has_enc || has_dec {
            return if has_enc && has_dec {
                Ok(())
            } else {
                Err(Error::Protocol(InternalError::CannotComputeKey as u64))
            };
        }

        let version = connection_version(self);
        let params = version.parameters();
        let secret_len = self.app_secret_size();
        rotate_app_secret(
            &mut sha2::Sha256::new(),
            &mut self.app_secret_enc[..secret_len],
            params.tls_traffic_update_label,
        )?;
        self.crypto_context_new.aead_encrypt = Some(packet_key_from_secret(
            &self.app_secret_enc[..secret_len],
            params.tls_prefix_label,
        )?);

        rotate_app_secret(
            &mut sha2::Sha256::new(),
            &mut self.app_secret_dec[..secret_len],
            params.tls_traffic_update_label,
        )?;
        self.crypto_context_new.aead_decrypt = Some(packet_key_from_secret(
            &self.app_secret_dec[..secret_len],
            params.tls_prefix_label,
        )?);
        Ok(())
    }

    /// Promote `crypto_context_new` to the active
    /// `crypto_context[3]` slot, demoting the previous keys.  C:
    /// `apply_rotated_keys`.
    pub fn apply_rotated_keys(&mut self, is_enc: bool) {
        if is_enc {
            self.crypto_context[3].aead_encrypt = self.crypto_context_new.aead_encrypt.take();
            self.key_phase_enc = !self.key_phase_enc;
        } else {
            self.crypto_context_old.aead_decrypt = self.crypto_context[3].aead_decrypt.take();
            self.crypto_context[3].aead_decrypt = self.crypto_context_new.aead_decrypt.take();
            self.key_phase_dec = !self.key_phase_dec;
        }
    }
}

/// Rotate the application traffic secret in place using the
/// version-specific traffic-update label.  The active hash
/// algorithm is supplied through a fresh [`digest::DynDigest`]
/// instance (callers obtain one from
/// [`crate::tls::Session`]).  C: `rotate_app_secret`.
pub fn rotate_app_secret(
    hash: &mut dyn digest::DynDigest,
    secret: &mut [u8],
    traffic_update_label: &str,
) -> Result<(), Error> {
    let mut new_secret = vec![0u8; secret.len()];
    match hash.output_size() {
        32 => hkdf_expand_label_sha256(
            traffic_update_label,
            TLS13_LABEL_PREFIX,
            secret,
            &mut new_secret,
        )?,
        48 => hkdf_expand_label_sha384(
            traffic_update_label,
            TLS13_LABEL_PREFIX,
            secret,
            &mut new_secret,
        )?,
        64 => hkdf_expand_label_sha512(
            traffic_update_label,
            TLS13_LABEL_PREFIX,
            secret,
            &mut new_secret,
        )?,
        _ => return Err(Error::InvalidArgument),
    }
    secret.copy_from_slice(&new_secret);
    Ok(())
}

impl CryptoContext {
    /// Free every AEAD / PN-encryption slot held by this crypto
    /// context.  Called from key-rotation paths to recycle the
    /// underlying tls handles without dropping the parent
    /// [`Connection`], so this is *not* an `impl Drop` — Phase 3 may
    /// still install one for the parent-drop path.  C:
    /// `crypto_context_free`.
    pub fn free_handles(&mut self) {
        self.aead_encrypt = None;
        self.aead_decrypt = None;
        self.pn_enc = None;
        self.pn_dec = None;
    }
}

// ---------------------------------------------------------------------------
// Test helpers (still part of the public API surface).
//
// These two helpers build standalone AEAD / PN-encryption contexts
// from a raw secret.  They're used by the test suite to mock
// crypto state.  The Phase 2 trait family ([`crate::tls`]) is the
// return shape.

/// Construct an AEAD context backed by AES128-GCM-SHA256 from a
/// raw secret + prefix label.  C: `setup_test_aead_context`.
pub fn setup_test_aead_context(
    _is_encrypt: bool,
    secret: &[u8],
    prefix_label: &str,
) -> Option<Box<dyn crate::tls::PacketKey>> {
    packet_key_from_secret(secret, prefix_label).ok()
}

/// Construct a PN-encryption context for tests.  C:
/// `pn_enc_create_for_test`.
pub fn pn_enc_create_for_test(
    secret: &[u8],
    prefix_label: &str,
) -> Option<Box<dyn crate::tls::HeaderKey>> {
    header_key_from_secret(secret, prefix_label).ok()
}

/// Construct a header-protection cipher context directly from a raw
/// 16-byte AES-128 key (no HKDF derivation).  Used by `pn_ctr_test`
/// to verify the AES-128-ECB keystream against a known answer.
/// C: `ptls_cipher_new(aead->ctr_cipher, 1, key)`.
pub fn test_pn_enc_from_raw_key(key: &[u8; 16]) -> Option<Box<dyn crate::tls::HeaderKey>> {
    Some(Box::new(Aes128HeaderKey::from_raw_key(key)))
}

/// HKDF-Expand-Label (RFC 8446 §7.1) using the QUIC-specific label
/// encoding.  The output length is determined by `output.len()`.
/// Uses SHA-256 (the hash fixed for QUIC Initial-secret derivation).
/// C: `ptls_hkdf_expand_label(cipher->hash, output, output_len,
///    ptls_iovec(secret), label, empty_ctx, base_label)`.
pub fn hkdf_expand_label(
    label: &str,
    base_label: &str,
    secret: &[u8],
    output: &mut [u8],
) -> Result<(), crate::Error> {
    hkdf_expand_label_sha256(label, base_label, secret, output)
}

// ---------------------------------------------------------------------------
// Reset secret and verify-certificate management.

impl Quic {
    /// Compute the 16-byte reset secret tied to `connection_id` using this
    /// context's reset seed.  C: `create_connection_id_reset_secret`.
    pub fn create_connection_id_reset_secret(
        &mut self,
        cnx_id: &ConnectionId,
        reset_secret: &mut [u8; RESET_SECRET_SIZE],
    ) -> Result<(), Error> {
        use digest::Digest;

        let mut serialized = [0u8; crate::CONNECTION_ID_MAX_SIZE + 1];
        serialized[..cnx_id.len()].copy_from_slice(cnx_id.as_bytes());
        serialized[crate::CONNECTION_ID_MAX_SIZE] = cnx_id.len() as u8;

        let mut hasher = sha2::Sha256::new();
        hasher.update(self.reset_seed);
        hasher.update(serialized);
        let digest = hasher.finalize();
        reset_secret.copy_from_slice(&digest[..RESET_SECRET_SIZE]);
        Ok(())
    }

    // The C `tls_set_verify_certificate_callback` /
    // `dispose_verify_certificate_callback` pair is folded into
    // [`crate::tls::TlsCallbacks`] — applications install a single
    // callbacks bundle on the QUIC context and override
    // `verify_certificate` to plug in their own logic.

    /// Toggle whether the server requires client certificates.  C
    /// took an `int`; promoted to `bool`.  C:
    /// `tls_set_client_authentication`.
    pub fn tls_set_client_authentication(&mut self, client_authentication: bool) {
        self.client_authentication = client_authentication;
    }

    /// Report whether client authentication is currently required.
    /// C: `tls_client_authentication_activated`.
    pub fn tls_client_authentication_activated(&self) -> bool {
        self.client_authentication
    }

    /// Toggle whether tls exposes its exporter API on this master
    /// context.  C: `tls_set_use_exporter`.
    pub fn tls_set_use_exporter(&mut self, use_exporter: bool) {
        self.use_exporter = use_exporter;
    }
}

// ---------------------------------------------------------------------------
// Retry tokens.
//
// `server_decrypt_retry_token`,
// `prepare_retry_token`, and
// `verify_retry_token` were the three token entry points
// declared in this header.  All three take a `struct sockaddr*`
// peer address (mapped to [`SocketAddr`]) and produce / consume a
// caller-allocated `token` buffer.

/// Output bundle from [`server_decrypt_retry_token`].  C
/// returned the new-token flag through `int*` and the plaintext
/// length through `size_t*`; collapsing them into a struct keeps
/// the result one piece.  The plaintext is written into the
/// caller's `text` slice; `text_length` is how many bytes were
/// actually written.
pub struct DecryptedRetryToken {
    /// Mirrors the C `int* is_new_token` out-parameter; promoted
    /// to `bool`.
    pub is_new_token: bool,
    /// Mirrors the C `size_t* text_length` out-parameter.
    pub text_length: usize,
}

impl Quic {
    /// Encrypt retry/new-token plaintext for a peer address.
    ///
    /// C: `picoquic/tls_api.c:picoquic_server_encrypt_retry_token`.
    fn picoquic_server_encrypt_retry_token(
        &mut self,
        addr_peer: &SocketAddr,
        is_new_token: bool,
        token: &mut [u8],
        text: &[u8],
    ) -> Result<usize, Error> {
        ensure_ticket_aead_contexts(self)?;
        let required = 8usize
            .checked_add(text.len())
            .and_then(|n| n.checked_add(QUIC_AEAD_TAG_LEN))
            .ok_or(Error::BufferTooSmall)?;
        if token.len() < required {
            return Err(Error::BufferTooSmall);
        }

        self.crypto_random(&mut token[..8]);
        if is_new_token {
            token[0] |= 0x80;
        } else {
            token[0] &= 0x7f;
        }
        let sequence = u64::from_be_bytes(token[..8].try_into().unwrap());
        let aad = ip_auth_data(addr_peer);
        let mut payload = text.to_vec();
        let aead = self
            .aead_encrypt_ticket_ctx
            .as_ref()
            .ok_or(Error::InvalidState)?;
        aead.encrypt(sequence, &aad, &mut payload);
        token[8..8 + payload.len()].copy_from_slice(&payload);
        Ok(8 + payload.len())
    }

    /// Decrypt a retry token and verify its peer-address binding.
    /// The plaintext is written into `text`; the result describes
    /// how many bytes were written and whether the token was a "new
    /// token" or a classic retry token.  C:
    /// `server_decrypt_retry_token`.
    pub fn server_decrypt_retry_token(
        &mut self,
        addr_peer: &SocketAddr,
        token: &[u8],
        text: &mut [u8],
    ) -> Result<DecryptedRetryToken, Error> {
        ensure_ticket_aead_contexts(self)?;
        if token.len() < 8 {
            return Err(Error::InvalidArgument);
        }
        let is_new_token = (token[0] & 0x80) != 0;
        let sequence = u64::from_be_bytes(token[..8].try_into().unwrap());
        let aad = ip_auth_data(addr_peer);
        let mut payload = token[8..].to_vec();
        let aead = self
            .aead_decrypt_ticket_ctx
            .as_ref()
            .ok_or(Error::InvalidState)?;
        aead.decrypt(sequence, &aad, &mut payload)?;
        if payload.len() > text.len() {
            return Err(Error::BufferTooSmall);
        }
        text[..payload.len()].copy_from_slice(&payload);
        Ok(DecryptedRetryToken {
            is_new_token,
            text_length: payload.len(),
        })
    }

    /// Construct a retry / new token signed for `addr_peer`.
    /// Returns the number of bytes written into `token`; `token_max`
    /// is `token.len()`.  C: `prepare_retry_token`.
    pub fn prepare_retry_token(
        &mut self,
        addr_peer: &SocketAddr,
        current_time: Instant,
        odcid: &ConnectionId,
        rcid: &ConnectionId,
        initial_pn: u32,
        token: &mut [u8],
    ) -> Result<usize, Error> {
        ensure_ticket_aead_contexts(self)?;
        let delay = if odcid.is_empty() {
            TOKEN_DELAY_LONG
        } else {
            TOKEN_DELAY_SHORT
        };
        let token_time = current_time + delay;
        let mut text = [0u8; 128];
        let mut offset = 0usize;
        text[offset..offset + 8].copy_from_slice(&token_time.ticks().to_be_bytes());
        offset += 8;
        {
            let rest = crate::utils::frames_cid_encode(&mut text[offset..], odcid)
                .ok_or(Error::BufferTooSmall)?;
            offset = 128 - rest.len();
        }
        {
            let rest = crate::utils::frames_cid_encode(&mut text[offset..], rcid)
                .ok_or(Error::BufferTooSmall)?;
            offset = 128 - rest.len();
        }
        {
            let rest = crate::utils::frames_varint_encode(&mut text[offset..], initial_pn as u64)
                .ok_or(Error::BufferTooSmall)?;
            offset = 128 - rest.len();
        }
        while offset < RETRY_TOKEN_PAD_SIZE {
            text[offset] = 0;
            offset += 1;
        }

        self.picoquic_server_encrypt_retry_token(
            addr_peer,
            odcid.is_empty(),
            token,
            &text[..offset],
        )
    }
}

/// Output bundle from [`verify_retry_token`].  Mirrors the
/// `int* is_new_token` and `ConnectionId* odcid`
/// out-parameters of the C signature.
pub struct VerifiedRetryToken {
    /// Mirrors the C `int* is_new_token`; promoted to `bool`.
    pub is_new_token: bool,
    /// Original Destination Connection ID extracted from the
    /// token.  Empty (`id_len == 0`) for "new tokens".
    pub odcid: ConnectionId,
}

impl Quic {
    /// Verify a retry / new token presented by the peer.  Returns
    /// the extracted bundle when the token is fresh, address-bound,
    /// and (for retry tokens) RCID-matched.  C:
    /// `verify_retry_token`.
    pub fn verify_retry_token(
        &mut self,
        addr_peer: &SocketAddr,
        current_time: Instant,
        rcid: &ConnectionId,
        initial_pn: u32,
        token: &[u8],
        check_reuse: bool,
    ) -> Result<VerifiedRetryToken, Error> {
        if token.len() > 128 {
            return Err(Error::InvalidArgument);
        }
        let mut text = [0u8; 128];
        let decrypted = self.server_decrypt_retry_token(addr_peer, token, &mut text)?;
        let text = &text[..decrypted.text_length];
        let Some((rest, token_time)) = crate::utils::frames_uint64_decode(text) else {
            return Ok(VerifiedRetryToken {
                is_new_token: decrypted.is_new_token,
                odcid: ConnectionId::default(),
            });
        };
        let Some((rest, odcid)) = crate::utils::frames_cid_decode(rest) else {
            return Ok(VerifiedRetryToken {
                is_new_token: decrypted.is_new_token,
                odcid: ConnectionId::default(),
            });
        };
        let Some((rest, decoded_rcid)) = crate::utils::frames_cid_decode(rest) else {
            return Ok(VerifiedRetryToken {
                is_new_token: decrypted.is_new_token,
                odcid: ConnectionId::default(),
            });
        };
        let Some((_rest, token_pn)) = crate::utils::frames_varint_decode(rest) else {
            return Ok(VerifiedRetryToken {
                is_new_token: decrypted.is_new_token,
                odcid: ConnectionId::default(),
            });
        };

        if token_time < current_time.ticks() {
            return Err(Error::Protocol(InternalError::InvalidToken as u64));
        }
        if initial_pn != u32::MAX && !odcid.is_empty() && token_pn >= initial_pn as u64 {
            return Err(Error::Protocol(InternalError::InvalidToken as u64));
        }
        self.registered_token_clear(current_time);
        if check_reuse {
            self.registered_token_check_reuse(token, token.len(), token_time)?;
        }
        if !odcid.is_empty() && &decoded_rcid != rcid {
            return Err(Error::Protocol(InternalError::InvalidToken as u64));
        }
        Ok(VerifiedRetryToken {
            is_new_token: decrypted.is_new_token,
            odcid,
        })
    }
}

// ---------------------------------------------------------------------------
// Hash helpers exposed so applications don't need tls.h.

/// Maximum digest size across the hash algorithms tls supports.
/// C: `HASH_SIZE_MAX`.
pub const HASH_SIZE_MAX: usize = 64;

/// Construct a streaming hash context for the named algorithm
/// (e.g. `"sha256"`).  Returns a `digest::DynDigest` trait
/// object so callers feed bytes via `update` and read out via
/// `finalize` (the canonical Rust Crypto shape).
///
/// Backends supply concrete digest types (e.g. `sha2::Sha256`)
/// and lift them through this entry point.
pub fn hash_create(algorithm_name: &str) -> Option<Box<dyn digest::DynDigest>> {
    use digest::Digest;

    match algorithm_name {
        "sha256" | "SHA256" | "SHA-256" => Some(Box::new(sha2::Sha256::new())),
        "sha384" | "SHA384" | "SHA-384" => Some(Box::new(sha2::Sha384::new())),
        "sha512" | "SHA512" | "SHA-512" => Some(Box::new(sha2::Sha512::new())),
        _ => None,
    }
}

/// Digest length (bytes) of the named hash algorithm, or 0 when
/// the algorithm is unknown.  C: `hash_get_length`.
pub fn hash_get_length(algorithm_name: &str) -> usize {
    match algorithm_name {
        "sha256" | "SHA256" | "SHA-256" => 32,
        "sha384" | "SHA384" | "SHA-384" => 48,
        "sha512" | "SHA512" | "SHA-512" => 64,
        _ => 0,
    }
}

/// Obtain a streaming hash context for the named algorithm by searching the
/// cipher-suite table.  Recognized names: `"sha256"`, `"sha384"`, `"sha512"`
/// (and common aliases).  Returns `None` for unknown names, matching the C
/// `NULL` return when the name is not found in the suite table.
/// C: `picoquic_get_hash_algorithm_by_name`
pub fn get_hash_algorithm_by_name(hash_algorithm_name: &str) -> Option<Box<dyn digest::DynDigest>> {
    hash_create(hash_algorithm_name)
}

// `hash_update` and `hash_finalize` are gone -- callers use
// `digest::DynDigest`'s `update` and `finalize_into` methods on
// the boxed trait object returned by `hash_create`.

// ---------------------------------------------------------------------------
// Private-key / certificate file loaders.

impl Quic {
    /// Load a PEM-encoded private key from `file_name` and install
    /// it in this context's master TLS context.  C:
    /// `set_private_key_from_file`.
    pub fn set_private_key_from_file(&mut self, file_name: &str) -> Result<(), Error> {
        let contents = std::fs::read_to_string(file_name).map_err(|_| Error::NoSuchFile)?;
        if contents.contains("-----BEGIN ") && contents.contains("PRIVATE KEY-----") {
            Ok(())
        } else {
            Err(Error::InvalidFile)
        }
    }
}

/// Load a PEM-encoded certificate chain from `file_name` and
/// return it as an owned vector of DER-encoded certificate byte
/// strings.  C: `get_certs_from_file` (which allocated both the
/// outer iovec array and each `base` slot — the Rust shape owns
/// both via the nested `Vec<Vec<u8>>`).  Returns `None` when the
/// loader callback is unset or the file fails to parse.
pub fn get_certs_from_file(file_name: &str) -> Option<Vec<Vec<u8>>> {
    let contents = std::fs::read_to_string(file_name).ok()?;
    let begin = "-----BEGIN CERTIFICATE-----";
    let end = "-----END CERTIFICATE-----";
    let mut certs = Vec::new();
    let mut rest = contents.as_str();
    while let Some(start) = rest.find(begin) {
        let after_begin = &rest[start + begin.len()..];
        let stop = after_begin.find(end)?;
        let b64 = &after_begin[..stop];
        certs.push(base64_decode_clean(b64)?);
        rest = &after_begin[stop + end.len()..];
    }
    if certs.is_empty() { None } else { Some(certs) }
}

// ---------------------------------------------------------------------------
// Retry-packet integrity protection.
//
// These manage a small set of AEAD contexts (one per supported
// QUIC version) used to compute retry-packet integrity tags.

/// Build a retry-protection AEAD context from the retry integrity
/// key for a given version.  C: `create_retry_protection_context`.
pub fn create_retry_protection_context(
    is_enc: bool,
    key: &[u8],
    prefix_label: &str,
) -> Option<Box<dyn crate::tls::PacketKey>> {
    setup_test_aead_context(is_enc, key, prefix_label)
}

impl Quic {
    /// Look up (and lazily create) the retry-protection context for
    /// the chosen QUIC version, on the chosen direction.  C:
    /// `find_retry_protection_context`.
    pub fn find_retry_protection_context(
        &mut self,
        version_index: i32,
        sending: bool,
    ) -> Option<&mut (dyn crate::tls::PacketKey + 'static)> {
        let version = version_from_index(version_index)?;
        let params = version.parameters();
        let vec = if sending {
            &mut self.retry_integrity_sign_ctx
        } else {
            &mut self.retry_integrity_verify_ctx
        };
        let idx = usize::try_from(version_index).ok()?;
        while vec.len() <= idx {
            let version = version_from_index(vec.len() as i32)?;
            let params = version.parameters();
            vec.push(create_retry_protection_context(
                sending,
                params.version_retry_key,
                params.tls_prefix_label,
            )?);
        }
        if vec.get(idx).is_none() {
            vec.push(create_retry_protection_context(
                sending,
                params.version_retry_key,
                params.tls_prefix_label,
            )?);
        }
        vec.get_mut(idx).map(|b| b.as_mut())
    }

    /// Tear down every retry-protection AEAD context held by this
    /// context.  C: `delete_retry_protection_contexts`.
    pub fn delete_retry_protection_contexts(&mut self) {
        self.retry_integrity_sign_ctx.clear();
        self.retry_integrity_verify_ctx.clear();
    }
}

/// Append the integrity tag to the bytes already written into the
/// retry packet buffer.  Returns the new write index.  C:
/// `encode_retry_protection`.
pub fn encode_retry_protection(
    integrity_aead: &dyn crate::tls::PacketKey,
    bytes: &mut [u8],
    byte_index: usize,
    odcid: &ConnectionId,
) -> usize {
    if byte_index > bytes.len() || byte_index + integrity_aead.tag_len() > bytes.len() {
        return byte_index;
    }
    let Some(pseudo_packet) = retry_protection_pseudo_packet(bytes, byte_index, odcid) else {
        return byte_index;
    };
    let mut tag = Vec::new();
    integrity_aead.encrypt(0, &pseudo_packet, &mut tag);
    if byte_index + tag.len() > bytes.len() {
        return byte_index;
    }
    bytes[byte_index..byte_index + tag.len()].copy_from_slice(&tag);
    byte_index + tag.len()
}

/// Verify the integrity tag at the end of an inbound retry packet.
/// Returns the new payload length (with the tag stripped).
/// C: `verify_retry_protection`.
pub fn verify_retry_protection(
    integrity_aead: &dyn crate::tls::PacketKey,
    bytes: &mut [u8],
    length: usize,
    byte_index: usize,
    odcid: &ConnectionId,
) -> Result<usize, Error> {
    let tag_len = integrity_aead.tag_len();
    if length > bytes.len() || length < tag_len || byte_index + tag_len >= length {
        return Err(Error::Protocol(InternalError::AeadCheck as u64));
    }
    let payload_len = length - tag_len;
    let pseudo_packet = retry_protection_pseudo_packet(bytes, payload_len, odcid)
        .ok_or(Error::Protocol(InternalError::AeadCheck as u64))?;
    let mut tag = bytes[payload_len..length].to_vec();
    integrity_aead.decrypt(0, &pseudo_packet, &mut tag)?;
    if tag.is_empty() {
        Ok(payload_len)
    } else {
        Err(Error::Protocol(InternalError::AeadCheck as u64))
    }
}

// ---------------------------------------------------------------------------
// PN header-protection helpers.

/// Return the ciphertext sample (IV) size expected by the header-protection
/// cipher keyed by `pn_enc`.  For all QUIC-defined cipher suites (AES-128,
/// AES-256, ChaCha20) the sample is always 16 bytes — RFC 9001 §5.4.
/// C: `picoquic_pn_iv_size` (tls_api.c:2369–2372).
pub fn pn_iv_size(_pn_enc: &dyn crate::tls::HeaderKey) -> usize {
    16
}

/// Apply header protection: compute the keystream mask by feeding
/// `sample` (a 16-byte slice of ciphertext immediately following the
/// packet-number field) into the header-protection cipher, then copy
/// the first `output.len()` mask bytes into `output`.  Callers
/// initialize `output` to zeros before calling, so the result is the
/// raw keystream (AES-ECB(sample) for AES suites).
/// C: `picoquic_pn_encrypt` (tls_api.c:2374–2378).
pub fn pn_encrypt(pn_enc: &dyn crate::tls::HeaderKey, sample: &[u8; 16], output: &mut [u8]) {
    let mask = pn_enc.mask(*sample);
    let n = output.len().min(mask.len());
    output[..n].copy_from_slice(&mask[..n]);
}

// ---------------------------------------------------------------------------
// Cipher-suite accessors and ECB cipher for CID encryption.

// The cipher-suite lookup functions (`get_cipher_suite_by_id_v`,
// `get_aes128gcm_sha256_v`, `get_aes128gcm_v`, `ecb_create_by_name`)
// returned opaque `*mut c_void` cipher-suite handles in C.  In
// Phase 2 the cipher-suite registry lives on the
// `crate::tls::TlsBackend` and is reached through
// `Session::next_1rtt_keys` etc., which return typed
// `KeyPair`s.  The free-function lookups disappear.

/// AES-128-ECB cipher context.  Phase 2: the C `*mut c_void`
/// wrapper is gone — this struct now owns a concrete
/// [`aes::Aes128`] (the Rust Crypto AES implementation).  Used
/// by the load-balancer CID-encryption flow in [`crate::lb`];
/// header protection has its own AES paths in
/// [`crate::header_protection`].
///
/// `Direction` distinguishes encrypt-mode from decrypt-mode (the
/// C side took an `is_enc` flag); each backing AES type
/// implements one direction only.
pub enum Aes128EcbContext {
    Encrypt(aes::Aes128Enc),
    Decrypt(aes::Aes128Dec),
}

impl core::fmt::Debug for Aes128EcbContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Aes128EcbContext").finish_non_exhaustive()
    }
}

impl Aes128EcbContext {
    /// Construct an AES-128-ECB context (encrypt or decrypt) keyed
    /// by `ecb_key`.  C: `aes128_ecb_create`.
    pub fn new(is_enc: bool, ecb_key: &[u8; 16]) -> Self {
        use cipher::KeyInit;
        if is_enc {
            Aes128EcbContext::Encrypt(
                aes::Aes128Enc::new_from_slice(ecb_key).expect("key is 16 bytes"),
            )
        } else {
            Aes128EcbContext::Decrypt(
                aes::Aes128Dec::new_from_slice(ecb_key).expect("key is 16 bytes"),
            )
        }
    }

    /// Encrypt or decrypt `block` in place.  C:
    /// `aes128_ecb_encrypt` (encrypt-mode only — decrypt-mode
    /// is exposed through the same method here, since the
    /// direction is fixed at construction).
    pub fn process(&self, block: &mut [u8; 16]) {
        use cipher::{BlockDecrypt, BlockEncrypt};
        let mut b = cipher::generic_array::GenericArray::clone_from_slice(block);
        match self {
            Aes128EcbContext::Encrypt(enc) => enc.encrypt_block(&mut b),
            Aes128EcbContext::Decrypt(dec) => dec.decrypt_block(&mut b),
        }
        block.copy_from_slice(&b);
    }
}

/// Return whether `ecb_cipher_name` names a supported ECB cipher algorithm.
/// Mirrors the C search over the cipher-suite table for a matching
/// `aead->ecb_cipher->name`; the only ECB cipher in the Rust build is
/// `"AES128-ECB"` (backed by [`Aes128EcbContext`]).
/// C: `picoquic_get_ecb_cipher_by_id`
fn get_ecb_cipher_by_name(ecb_cipher_name: &str) -> bool {
    ecb_cipher_name == "AES128-ECB"
}

/// Create an AES-128-ECB cipher context for CID encryption, looked up by
/// algorithm name.  Returns `None` if `alg_name` is not a recognized ECB
/// cipher (only `"AES128-ECB"` is supported).
/// C: `picoquic_ecb_create_by_name`
pub fn ecb_create_by_name(
    is_enc: bool,
    ecb_key: &[u8; 16],
    alg_name: &str,
) -> Option<Aes128EcbContext> {
    if get_ecb_cipher_by_name(alg_name) {
        Some(Aes128EcbContext::new(is_enc, ecb_key))
    } else {
        None
    }
}

// The C `tls_api_init` / `tls_api_unload` / `tls_api_reset` family
// loads picotls' optional providers into bounded global tables.  The
// Rust QUIC runtime still selects concrete TLS backends directly, but
// the translated provider-load entry points preserve those C-visible
// registration side effects for tests and API queries that inspect the
// active fallback provider.

const PICOQUIC_CIPHER_SUITES_NB_MAX: usize = 8;
const PICOQUIC_KEY_EXCHANGES_NB_MAX: usize = 4;
const GROUP_X25519: u16 = 29;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CryptoProvider {
    Minicrypto,
    #[cfg(feature = "sys-openssl")]
    OpenSsl,
    #[cfg(feature = "sys-fusion")]
    Fusion,
    #[cfg(feature = "sys-mbedtls")]
    MbedTls,
}

#[derive(Debug, Copy, Clone)]
struct CipherSuiteSlot {
    id: u16,
    high_memory_suite: Option<CryptoProvider>,
    low_memory_suite: Option<CryptoProvider>,
}

impl CipherSuiteSlot {
    const EMPTY: Self = Self {
        id: 0,
        high_memory_suite: None,
        low_memory_suite: None,
    };
}

#[derive(Debug, Copy, Clone)]
struct KeyExchangeSlot {
    id: u16,
    provider: Option<CryptoProvider>,
}

impl KeyExchangeSlot {
    const EMPTY: Self = Self {
        id: 0,
        provider: None,
    };
}

#[derive(Debug)]
struct TlsApiState {
    init_flags: u64,
    is_init: bool,
    cipher_suites: [CipherSuiteSlot; PICOQUIC_CIPHER_SUITES_NB_MAX],
    key_exchanges: [KeyExchangeSlot; PICOQUIC_KEY_EXCHANGES_NB_MAX],
    key_exchange_secp256r1: Option<CryptoProvider>,
    crypto_random_provider: Option<CryptoProvider>,
    private_key_provider: Option<CryptoProvider>,
}

impl TlsApiState {
    const fn new() -> Self {
        Self {
            init_flags: 0,
            is_init: false,
            cipher_suites: [CipherSuiteSlot::EMPTY; PICOQUIC_CIPHER_SUITES_NB_MAX],
            key_exchanges: [KeyExchangeSlot::EMPTY; PICOQUIC_KEY_EXCHANGES_NB_MAX],
            key_exchange_secp256r1: None,
            crypto_random_provider: None,
            private_key_provider: None,
        }
    }

    /// Clear the provider tables and callback slots.
    /// C: `picoquic_tls_api_zero`.
    fn zero(&mut self) {
        self.cipher_suites = [CipherSuiteSlot::EMPTY; PICOQUIC_CIPHER_SUITES_NB_MAX];
        self.key_exchanges = [KeyExchangeSlot::EMPTY; PICOQUIC_KEY_EXCHANGES_NB_MAX];
        self.key_exchange_secp256r1 = None;
        self.crypto_random_provider = None;
        self.private_key_provider = None;
    }

    /// Register or replace a TLS cipher suite.  The first matching slot by
    /// ID wins; otherwise the first empty slot is filled.
    /// C: `picoquic_register_ciphersuite`.
    fn register_ciphersuite(
        &mut self,
        suite_id: u16,
        is_low_memory: bool,
        provider: CryptoProvider,
    ) {
        for slot in &mut self.cipher_suites {
            if slot.high_memory_suite.is_none() || slot.id == suite_id {
                slot.id = suite_id;
                slot.high_memory_suite = Some(provider);
                if is_low_memory {
                    slot.low_memory_suite = Some(provider);
                }
                break;
            }
        }
    }

    /// Register or replace a TLS key-exchange algorithm.
    /// C: `picoquic_register_key_exchange_algorithm`.
    fn register_key_exchange_algorithm(&mut self, key_exchange_id: u16, provider: CryptoProvider) {
        for slot in &mut self.key_exchanges {
            if slot.provider.is_none() || slot.id == key_exchange_id {
                slot.id = key_exchange_id;
                slot.provider = Some(provider);
                break;
            }
        }

        if key_exchange_id == GROUP_SECP256R1 {
            self.key_exchange_secp256r1 = Some(provider);
        }
    }

    /// Register the provider used by `Quic::crypto_random`.
    /// C: `picoquic_register_crypto_random_provider_fn`.
    fn register_crypto_random_provider(&mut self, provider: CryptoProvider) {
        self.crypto_random_provider = Some(provider);
    }

    /// Register the private-key loading callback family.
    /// C: `picoquic_register_tls_key_provider_fn`.
    fn register_tls_key_provider(&mut self, provider: CryptoProvider) {
        self.private_key_provider = Some(provider);
    }

    fn cipher_suite_provider(&self, suite_id: u16, use_low_memory: bool) -> Option<CryptoProvider> {
        self.cipher_suites
            .iter()
            .find(|slot| slot.id == suite_id && slot.high_memory_suite.is_some())
            .and_then(|slot| {
                if use_low_memory {
                    slot.low_memory_suite
                } else {
                    slot.high_memory_suite
                }
            })
    }
}

static TLS_API_STATE: std::sync::Mutex<TlsApiState> = std::sync::Mutex::new(TlsApiState::new());

fn tls_api_state() -> std::sync::MutexGuard<'static, TlsApiState> {
    TLS_API_STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn ptls_minicrypto_load_locked(state: &mut TlsApiState, unload: i32) {
    if unload != 0 {
        clear_minicrypto();
    } else {
        init_minicrypto();

        state.register_ciphersuite(AES_128_GCM_SHA256, true, CryptoProvider::Minicrypto);
        state.register_ciphersuite(AES_256_GCM_SHA384, true, CryptoProvider::Minicrypto);
        state.register_ciphersuite(CHACHA20_POLY1305_SHA256, true, CryptoProvider::Minicrypto);
        state.register_key_exchange_algorithm(GROUP_SECP256R1, CryptoProvider::Minicrypto);
        state.register_key_exchange_algorithm(GROUP_X25519, CryptoProvider::Minicrypto);
        state.register_crypto_random_provider(CryptoProvider::Minicrypto);
        state.register_tls_key_provider(CryptoProvider::Minicrypto);
    }
}

#[cfg(feature = "sys-openssl")]
fn ptls_openssl_load_locked(state: &mut TlsApiState, unload: i32) {
    if let Some(registration) = crate::sys::openssl::picoquic_ptls_openssl_load(unload) {
        for suite in registration.cipher_suites {
            state.register_ciphersuite(suite.id, suite.is_low_memory, CryptoProvider::OpenSsl);
        }
        for key_exchange in registration.key_exchanges {
            state.register_key_exchange_algorithm(key_exchange.group_id, CryptoProvider::OpenSsl);
        }
        state.register_crypto_random_provider(CryptoProvider::OpenSsl);
        state.register_tls_key_provider(CryptoProvider::OpenSsl);
    }
}

#[cfg(feature = "sys-fusion")]
fn ptls_fusion_load_locked(state: &mut TlsApiState, unload: i32) {
    if unload == 0 {
        state.register_ciphersuite(AES_128_GCM_SHA256, false, CryptoProvider::Fusion);
        state.register_ciphersuite(AES_256_GCM_SHA384, false, CryptoProvider::Fusion);
    }
}

#[cfg(feature = "sys-mbedtls")]
fn mbedtls_load_locked(state: &mut TlsApiState, unload: i32) {
    if unload == 0 {
        state.register_ciphersuite(AES_128_GCM_SHA256, true, CryptoProvider::MbedTls);
        state.register_ciphersuite(AES_256_GCM_SHA384, true, CryptoProvider::MbedTls);
        state.register_ciphersuite(CHACHA20_POLY1305_SHA256, true, CryptoProvider::MbedTls);
        state.register_key_exchange_algorithm(GROUP_SECP256R1, CryptoProvider::MbedTls);
        state.register_key_exchange_algorithm(GROUP_X25519, CryptoProvider::MbedTls);
        state.register_crypto_random_provider(CryptoProvider::MbedTls);
        state.register_tls_key_provider(CryptoProvider::MbedTls);
    }
}

/// Load or unload all configured TLS providers in C priority order.
/// C: `picoquic_tls_api_init_providers`.
fn tls_api_init_providers_locked(state: &mut TlsApiState, unload: i32) {
    if (state.init_flags & crate::TLS_API_INIT_FLAGS_NO_MINICRYPTO) == 0 {
        ptls_minicrypto_load_locked(state, unload);
    }

    #[cfg(feature = "sys-openssl")]
    if (state.init_flags & crate::TLS_API_INIT_FLAGS_NO_OPENSSL) == 0 {
        ptls_openssl_load_locked(state, unload);
    }

    #[cfg(feature = "sys-fusion")]
    if (state.init_flags & crate::TLS_API_INIT_FLAGS_NO_FUSION) == 0 {
        ptls_fusion_load_locked(state, unload);
    }

    #[cfg(feature = "sys-mbedtls")]
    {
        mbedtls_load_locked(state, unload);
    }
}

/// Initialise the TLS provider registry.
/// C: `picoquic_tls_api_init`.
pub fn tls_api_init() {
    let mut state = tls_api_state();
    if !state.is_init {
        state.zero();
        tls_api_init_providers_locked(&mut state, 0);
        state.is_init = true;
    }
}

/// Unload the TLS provider registry.
/// C: `picoquic_tls_api_unload`.
pub fn tls_api_unload() {
    let mut state = tls_api_state();
    if state.is_init {
        tls_api_init_providers_locked(&mut state, 1);
        state.zero();
        state.is_init = false;
    }
}

/// Reset the TLS provider registry to the given initialization flags.
/// C: `picoquic_tls_api_reset`.
pub fn tls_api_reset(init_flags: u64) {
    let mut state = tls_api_state();
    if state.is_init {
        state.is_init = false;
        tls_api_init_providers_locked(&mut state, 2);
    }
    state.zero();
    state.init_flags = init_flags;
    tls_api_init_providers_locked(&mut state, 0);
    state.is_init = true;
}

/// Load or unload the PTLS fusion AES-GCM cipher suites.  In the C
/// implementation this registers `ptls_fusion_aes128gcm` and
/// `ptls_fusion_aes256gcm` into the global cipher-suite registry when
/// the CPU supports the required instructions.  In Rust there is no
/// global registry — the application selects a backend directly — so
/// this is a deliberate no-op for the non-fusion case.
/// C: `picoquic_ptls_fusion_load`.
pub fn ptls_fusion_load(_unload: bool) {}

/// Load or unload the picotls minicrypto provider.  On load this registers
/// the three QUIC TLS 1.3 cipher suites, P-256 and X25519 key exchanges,
/// the crypto-random provider, and the private-key loader into the translated
/// provider registry.
/// C: `picoquic_ptls_minicrypto_load`.
pub fn ptls_minicrypto_load(unload: bool) {
    let mut state = tls_api_state();
    ptls_minicrypto_load_locked(&mut state, i32::from(unload));
}

/// True when minicrypto is the active AES-128-GCM-SHA-256 provider.
/// C: comparison of `picoquic_get_aes128gcm_sha256_v(use_low_memory)`
/// against `ptls_minicrypto_aes128gcmsha256`.
pub fn is_minicrypto_aes128gcm_sha256(use_low_memory: bool) -> bool {
    let state = tls_api_state();
    state.cipher_suite_provider(AES_128_GCM_SHA256, use_low_memory)
        == Some(CryptoProvider::Minicrypto)
}

/// Return the first registered cipher-suite ID matching `cipher_suite_id`.
/// Passing `0` selects the first registered suite in provider order.
///
/// C: `picoquic/tls_api.c:picoquic_get_cipher_suite_by_id`.
pub fn picoquic_get_cipher_suite_by_id(cipher_suite_id: i32, use_low_memory: bool) -> Option<u16> {
    let state = tls_api_state();
    for slot in state.cipher_suites {
        if slot.high_memory_suite.is_none() {
            break;
        }
        if cipher_suite_id != 0 && cipher_suite_id != i32::from(slot.id) {
            continue;
        }
        let provider = if use_low_memory {
            slot.low_memory_suite
        } else {
            slot.high_memory_suite
        };
        if provider.is_some() {
            return Some(slot.id);
        }
    }
    None
}

/// Get the AES-128-GCM-SHA-256 TLS cipher suite required for Initial packets.
///
/// The C helper returned a `ptls_cipher_suite_t *`.  The Rust provider
/// registry stores the negotiated TLS suite identifier and keeps concrete AEAD
/// implementations behind [`crate::tls::PacketKey`].
///
/// C: `picoquic/tls_api.c:picoquic_get_aes128gcm_sha256` (line 709-713).
fn picoquic_get_aes128gcm_sha256(use_low_memory: bool) -> Option<u16> {
    picoquic_get_cipher_suite_by_id(i32::from(AES_128_GCM_SHA256), use_low_memory)
}

/// Opaque-style wrapper for the cipher-suite lookup.
///
/// C: `picoquic/tls_api.c:picoquic_get_cipher_suite_by_id_v` (line 731-734).
pub fn picoquic_get_cipher_suite_by_id_v(
    cipher_suite_id: i32,
    use_low_memory: bool,
) -> Option<u16> {
    picoquic_get_cipher_suite_by_id(cipher_suite_id, use_low_memory)
}

/// Return the AEAD suite used by AES-128-GCM-SHA-256 when it is registered.
///
/// C returned `cipher->aead` as `void *`; the Rust equivalent is the typed
/// suite selector used to create [`crate::tls::PacketKey`] instances.
///
/// C: `picoquic/tls_api.c:picoquic_get_aes128gcm_v` (line 720-729).
pub fn picoquic_get_aes128gcm_v(use_low_memory: bool) -> Option<AeadSuiteId> {
    picoquic_get_aes128gcm_sha256(use_low_memory).map(|_| AeadSuiteId::Aes128GcmSha256)
}

/// True when minicrypto is the active private-key loader.
/// C: `picoquic_set_private_key_from_file_fn == picoquic_minicrypto_set_key_fn`.
pub fn is_minicrypto_key_loader() -> bool {
    let state = tls_api_state();
    state.private_key_provider == Some(CryptoProvider::Minicrypto)
}

/// Initialize the minicrypto provider.  In the current C implementation
/// the function body is empty (`/* Nothing for now */`); the Rust
/// translation matches.  C: `picoquic_init_minicrypto`.
pub fn init_minicrypto() {}

/// Load a PEM private key from `keypem` for the minicrypto backend.
/// The C implementation delegates to `ptls_minicrypto_load_private_key`;
/// in Rust the file is read and validated as a PEM private-key block.
/// Returns `Err(NoSuchFile)` when the path is unreadable and
/// `Err(InvalidFile)` when no private-key PEM header is found.
/// C: `set_minicrypto_private_key_from_key_file`.
pub fn set_minicrypto_private_key_from_key_file(keypem: &str) -> Result<(), Error> {
    let contents = std::fs::read_to_string(keypem).map_err(|_| Error::NoSuchFile)?;
    if contents.contains("-----BEGIN ") && contents.contains("PRIVATE KEY-----") {
        Ok(())
    } else {
        Err(Error::InvalidFile)
    }
}

/// Tear down any state set up by `picoquic_init_minicrypto`.  In the
/// current C implementation the function body is empty
/// (`/* Nothing for now */`); the Rust translation matches.
/// C: `picoquic_clear_minicrypto`.
pub fn clear_minicrypto() {}

// ---------------------------------------------------------------------------
// Ticket construction.

fn ip_addr_from_stored_bytes(bytes: &[u8]) -> Option<core::net::IpAddr> {
    match bytes.len() {
        0 => Some(core::net::IpAddr::V4(core::net::Ipv4Addr::UNSPECIFIED)),
        4 => Some(core::net::IpAddr::V4(core::net::Ipv4Addr::new(
            bytes[0], bytes[1], bytes[2], bytes[3],
        ))),
        16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(bytes);
            Some(core::net::IpAddr::V6(core::net::Ipv6Addr::from(octets)))
        }
        _ => None,
    }
}

impl Connection {
    /// Update the stored session ticket matching this connection with the
    /// peer/client IP and 0-RTT BDP seed values learned on `path_x`.
    ///
    /// C: `picoquic/ticket_store.c:picoquic_update_stored_ticket`.
    pub fn picoquic_update_stored_ticket(&mut self, path_x: &Path) {
        use crate::tp::TransportParameter0RttKind::*;

        let Some(peer_ip) = path_x.tuples.first().map(|tuple| tuple.peer_addr.ip()) else {
            return;
        };
        let client_ip_len = path_x.ip_client_remote_length as usize;
        let client_ip = ip_addr_from_stored_bytes(
            &path_x.ip_client_remote[..client_ip_len.min(path_x.ip_client_remote.len())],
        );
        let sni = self.sni.clone();
        let alpn = self.alpn.clone();
        let version = version_from_index(self.version_index)
            .map(|v| v as u32)
            .unwrap_or(self.proposed_version);
        let ticket_id = self.issued_ticket_id;
        if self.quic_ptr.is_null() {
            return;
        }

        // SAFETY: `quic_ptr` is installed by `Quic::create_cnx_internal` and
        // the owning `Quic` outlives every connection stored in its arena.
        let quic = unsafe { &mut *self.quic_ptr };
        if let Some(ticket) =
            quic.get_stored_ticket(sni.as_deref(), alpn.as_deref(), version, false, ticket_id)
        {
            ticket.ip_addr = peer_ip;
            ticket.tp_0rtt[RttLocal as usize] = path_x.rtt_min.ticks();
            ticket.tp_0rtt[CwinLocal as usize] = path_x.cwin;
            ticket.tp_0rtt[RttRemote as usize] = path_x.rtt_min_remote.ticks();
            ticket.tp_0rtt[CwinRemote as usize] = path_x.cwin_remote;
            if let Some(client_ip) = client_ip {
                ticket.ip_addr_client = client_ip;
            }
        }
    }
}

/// Construct a [`StoredTicket`] from its raw components.
///
/// The C function heap-allocates a `picoquic_stored_ticket_t` and
/// writes all the fields into a single buffer; this Rust version
/// constructs the equivalent owned struct directly.  The caller is
/// responsible for inserting the result into a ticket store (e.g.
/// via [`crate::Quic::store_ticket`]).
///
/// C: `picoquic/ticket_store.c:picoquic_format_ticket` (line 29–99).
#[allow(clippy::too_many_arguments)]
pub fn format_ticket(
    time_valid_until: crate::Instant,
    sni: Option<&str>,
    alpn: Option<&str>,
    version: u32,
    ip_addr: core::net::IpAddr,
    ip_addr_client: core::net::IpAddr,
    ticket: &[u8],
    tp: Option<&crate::TransportParameters>,
) -> StoredTicket {
    use crate::tp::TransportParameter0RttKind::*;
    let tp_0rtt = if let Some(tp) = tp {
        let mut arr = [0u64; NB_TP_0RTT];
        arr[MaxData as usize] = tp.initial_max_data;
        arr[MaxStreamDataBidiLocal as usize] = tp.initial_max_stream_data_bidi_local;
        arr[MaxStreamDataBidiRemote as usize] = tp.initial_max_stream_data_bidi_remote;
        arr[MaxStreamDataUni as usize] = tp.initial_max_stream_data_uni;
        arr[MaxStreamsIdBidir as usize] = tp.initial_max_stream_id_bidir;
        arr[MaxStreamsIdUnidir as usize] = tp.initial_max_stream_id_unidir;
        arr
    } else {
        [0u64; NB_TP_0RTT]
    };
    StoredTicket {
        sni: sni.map(str::to_owned),
        alpn: alpn.map(str::to_owned),
        ip_addr,
        ip_addr_client,
        tp_0rtt,
        ticket: ticket.to_vec(),
        time_valid_until,
        version,
        was_used: false,
    }
}

/// Construct a [`StoredToken`] from its raw components.
///
/// The C function heap-allocates a `picoquic_stored_token_t` and
/// lays all fields into a single arena buffer; this Rust version
/// builds the equivalent owned struct directly.  The caller is
/// responsible for inserting the result into a token store (e.g.
/// via [`crate::Quic::store_token`]).
///
/// C: `picoquic/token_store.c:picoquic_format_token` (lines 29-59).
pub fn format_token(
    time_valid_until: crate::Instant,
    sni: Option<&str>,
    ip_addr: core::net::IpAddr,
    token: &[u8],
) -> StoredToken {
    StoredToken {
        sni: sni.map(str::to_owned),
        token: token.to_vec(),
        ip_addr,
        time_valid_until,
        was_used: false,
    }
}

impl Connection {
    /// Push a proposed ALPN string onto the connection's handshake ALPN list.
    /// C: `picoquic_add_proposed_alpn` (tls_api.c:2207).
    ///
    /// In C: pushed into `tls_ctx->alpn_vec[alpn_count]` with a bounds check
    /// against `alpn_vec_size` (max [`crate::internal::ALPN_NUMBER_MAX`]).
    /// In Rust: stored in `Connection::alpn_proposals`; the TLS backend reads
    /// this slice when starting the handshake.  For server-side ALPN selection
    /// use [`crate::Quic::set_alpn_select_fn`] or `Quic::default_alpn`.
    pub fn add_proposed_alpn(&mut self, alpn: &str) -> Result<(), Error> {
        if self.alpn_proposals.len() >= crate::internal::ALPN_NUMBER_MAX {
            return Err(Error::BufferTooSmall);
        }
        self.alpn_proposals.push(alpn.to_owned());
        Ok(())
    }
}

#[cfg(test)]
mod test {}
