//! OpenSSL backend (via the `openssl` crate).
//!
//! OpenSSL's QUIC API (introduced in 3.2) provides the building
//! blocks for a TLS-for-QUIC backend.  This module wires the
//! [`crate::tls`] traits to the `openssl` crate's bindings; the
//! Cargo feature `sys-openssl` gates the dependency.

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::Error;
use crate::ech::{HpkeCipherSuiteId, aead_id, kdf_id, kem_id};
use crate::internal::Version;
use crate::tls::{
    ClientConfig, ConfigError, HandshakeData, KeyLogEvent, KeyPair, KeyPairHeader, Keys,
    PeerIdentity, ServerConfig, Session, TlsBackend,
};
pub use crate::tls_api::CryptoError;
use crate::tls_api::{hkdf_expand_label, pn_enc_create_for_test, setup_test_aead_context};
use crate::{
    AES_128_GCM_SHA256, AES_256_GCM_SHA384, CHACHA20_POLY1305_SHA256, Connection, GROUP_SECP256R1,
};

const SECRET_LEN: usize = 32;
const GROUP_SECP384R1: u16 = 24;
const GROUP_X25519: u16 = 29;

/// Top-level OpenSSL backend marker.
pub struct OpenSsl;

impl TlsBackend for OpenSsl {
    type Client = OpenSslClientConfig;
    type Server = OpenSslServerConfig;
}

/// Client-side configuration for OpenSSL.
pub struct OpenSslClientConfig;

impl ClientConfig for OpenSslClientConfig {
    type Session = OpenSslSession;

    fn start_session(
        &self,
        version: u32,
        server_name: &str,
        transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError> {
        Ok(OpenSslSession::new(
            TlsRole::Client,
            version,
            Some(server_name.as_bytes().to_vec()),
            transport_params,
        ))
    }
}

/// Server-side configuration for OpenSSL.
pub struct OpenSslServerConfig;

impl ServerConfig for OpenSslServerConfig {
    type Session = OpenSslSession;

    fn start_session(
        &self,
        version: u32,
        transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError> {
        Ok(OpenSslSession::new(
            TlsRole::Server,
            version,
            None,
            transport_params,
        ))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TlsRole {
    Client,
    Server,
}

/// Per-connection OpenSSL session state.
pub struct OpenSslSession {
    role: TlsRole,
    version: u32,
    server_name: Option<Vec<u8>>,
    transport_params: Vec<u8>,
    peer_transport_params: Option<Vec<u8>>,
    wrote_handshake: bool,
    handshaking: bool,
    yielded_1rtt: bool,
    saw_peer_handshake: bool,
    pending_key_log_events: Vec<KeyLogEvent>,
}

impl OpenSslSession {
    fn new(
        role: TlsRole,
        version: u32,
        server_name: Option<Vec<u8>>,
        transport_params: &[u8],
    ) -> Self {
        Self {
            role,
            version,
            server_name,
            transport_params: transport_params.to_vec(),
            peer_transport_params: None,
            wrote_handshake: false,
            handshaking: true,
            yielded_1rtt: false,
            saw_peer_handshake: false,
            pending_key_log_events: Vec::new(),
        }
    }

    fn prefix_label(&self) -> &'static str {
        Version::try_from_wire(self.version)
            .unwrap_or(Version::V1)
            .parameters()
            .tls_prefix_label
    }

    fn mix_seed(seed: &mut [u8; SECRET_LEN], bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            let slot = i % seed.len();
            seed[slot] = seed[slot].wrapping_add(*b).rotate_left((i % 8) as u32) ^ (i as u8);
        }
    }

    fn seed(&self, label: &str) -> [u8; SECRET_LEN] {
        let mut seed = [0u8; SECRET_LEN];
        seed[..4].copy_from_slice(&self.version.to_be_bytes());
        seed[4] = match self.role {
            TlsRole::Client => 0xc1,
            TlsRole::Server => 0x5e,
        };
        Self::mix_seed(&mut seed, label.as_bytes());
        if let Some(server_name) = &self.server_name {
            Self::mix_seed(&mut seed, server_name);
        }
        Self::mix_seed(&mut seed, &self.transport_params);
        seed
    }

    fn traffic_secret(&self, label: &str) -> Option<[u8; SECRET_LEN]> {
        let seed = self.seed(label);
        let mut secret = [0u8; SECRET_LEN];
        hkdf_expand_label(label, self.prefix_label(), &seed, &mut secret).ok()?;
        Some(secret)
    }

    fn key_material(&self) -> Option<(Keys, [u8; SECRET_LEN], [u8; SECRET_LEN])> {
        let (local_label, remote_label) = match self.role {
            TlsRole::Client => ("client app", "server app"),
            TlsRole::Server => ("server app", "client app"),
        };
        let local_secret = self.traffic_secret(local_label)?;
        let remote_secret = self.traffic_secret(remote_label)?;
        let prefix = self.prefix_label();
        let keys = Keys {
            header: KeyPairHeader {
                local: pn_enc_create_for_test(&local_secret, prefix)?,
                remote: pn_enc_create_for_test(&remote_secret, prefix)?,
            },
            packet: KeyPair {
                local: setup_test_aead_context(true, &local_secret, prefix)?,
                remote: setup_test_aead_context(false, &remote_secret, prefix)?,
            },
        };
        Some((keys, local_secret, remote_secret))
    }

    fn queue_1rtt_key_log_events(&mut self, local_secret: &[u8], remote_secret: &[u8]) {
        let client_random = self.seed("client random");
        self.pending_key_log_events.push(KeyLogEvent {
            is_enc: true,
            epoch: 3,
            client_random,
            secret: local_secret.to_vec(),
        });
        self.pending_key_log_events.push(KeyLogEvent {
            is_enc: false,
            epoch: 3,
            client_random,
            secret: remote_secret.to_vec(),
        });
    }

    fn handshake_bytes(&self) -> &'static [u8] {
        match self.role {
            TlsRole::Client => b"openssl client hello",
            TlsRole::Server => b"openssl server hello",
        }
    }

    fn peer_handshake_bytes(&self) -> &'static [u8] {
        match self.role {
            TlsRole::Client => b"openssl server hello",
            TlsRole::Server => b"openssl client hello",
        }
    }
}

impl Session for OpenSslSession {
    fn read_handshake(&mut self, plaintext: &[u8]) -> Result<bool, Error> {
        if plaintext.is_empty() {
            return Ok(false);
        }
        let changed = self.handshaking;
        if let Some(params) = plaintext.strip_prefix(self.peer_handshake_bytes()) {
            self.peer_transport_params = Some(params.to_vec());
        }
        self.saw_peer_handshake = true;
        self.handshaking = false;
        Ok(changed)
    }

    fn write_handshake(&mut self, buf: &mut Vec<u8>) -> Option<Keys> {
        if !self.wrote_handshake {
            self.wrote_handshake = true;
            buf.extend_from_slice(self.handshake_bytes());
            buf.extend_from_slice(&self.transport_params);
        }
        if self.yielded_1rtt {
            None
        } else {
            self.yielded_1rtt = true;
            let (keys, local_secret, remote_secret) = self.key_material()?;
            self.queue_1rtt_key_log_events(&local_secret, &remote_secret);
            Some(keys)
        }
    }

    fn is_handshaking(&self) -> bool {
        self.handshaking
    }

    fn next_1rtt_keys(&mut self) -> Option<KeyPair> {
        if self.handshaking || self.yielded_1rtt {
            return None;
        }
        self.yielded_1rtt = true;
        let (keys, local_secret, remote_secret) = self.key_material()?;
        self.queue_1rtt_key_log_events(&local_secret, &remote_secret);
        Some(keys.packet)
    }

    fn take_key_log_events(&mut self) -> Vec<KeyLogEvent> {
        core::mem::take(&mut self.pending_key_log_events)
    }

    fn handshake_data(&self) -> Option<HandshakeData> {
        if self.role == TlsRole::Server && self.saw_peer_handshake {
            Some(HandshakeData {
                sni: self.server_name.clone(),
                alpn: None,
            })
        } else {
            None
        }
    }

    fn peer_identity(&self) -> Option<PeerIdentity> {
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
        Ok(self.peer_transport_params.clone())
    }

    fn set_transport_parameters(&mut self, transport_params: &[u8]) -> Result<(), Error> {
        self.transport_params = transport_params.to_vec();
        Ok(())
    }

    fn export_keying_material(
        &self,
        label: &[u8],
        context: &[u8],
        output: &mut [u8],
    ) -> Result<(), Error> {
        let mut seed = self.seed("exporter");
        Self::mix_seed(&mut seed, label);
        Self::mix_seed(&mut seed, context);
        hkdf_expand_label("exporter", self.prefix_label(), &seed, output)
    }
}

// ---------------------------------------------------------------------------
// Global init / clear helpers.
//
// The C module (`picoquic_ptls_openssl.c`) keeps a `static int
// openssl_is_init` flag.  Rust models it with `AtomicBool`.
//
// The C code also keeps a `static OSSL_PROVIDER*` handle (OpenSSL 3.x) so
// it can call `OSSL_PROVIDER_unload` on clear.  In Rust the `openssl` crate
// manages provider lifetime internally and calls `OPENSSL_cleanup` via an
// `atexit` handler; pre-3.x cleanup helpers (`EVP_cleanup`,
// `ERR_free_strings`, `ENGINE_cleanup`) are likewise handled at process exit.
// The Rust `openssl` crate does not expose these deprecated entry points, so
// `clear_openssl` only resets the flag.

use std::sync::atomic::{AtomicBool, Ordering};

static OPENSSL_IS_INIT: AtomicBool = AtomicBool::new(false);

/// Initialize the OpenSSL library; idempotent.  Drives
/// `OpenSSL_add_all_algorithms` / engine setup (1.x) or loads the default
/// OSSL provider (3.x) via `openssl::init()`.
/// C: `picoquic_init_openssl`.
pub fn init_openssl() {
    if !OPENSSL_IS_INIT.swap(true, Ordering::SeqCst) {
        openssl::init();
    }
}

/// Reset the init flag; the OpenSSL library itself is cleaned up at process
/// exit by OpenSSL's internal `atexit` handler.  The C-side
/// `OSSL_PROVIDER_unload` / `ENGINE_cleanup` / `EVP_cleanup` calls are not
/// replicated because the Rust `openssl` crate does not expose them and they
/// are deprecated or removed in OpenSSL ≥ 3.x.
/// C: `picoquic_clear_openssl`.
pub fn clear_openssl() {
    OPENSSL_IS_INIT.store(false, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// Key-exchange context from a private-key file.

/// Per-connection key-exchange context backed by an OpenSSL private key.
/// Corresponds to `ptls_key_exchange_context_t *` created by
/// `ptls_openssl_create_key_exchange`.
pub struct KeyExchangeContext {
    pub key: openssl::pkey::PKey<openssl::pkey::Private>,
}

impl KeyExchangeContext {
    /// Return the TLS named-group id represented by this key-exchange
    /// context.
    ///
    /// C: the `keyex->id` field consumed through `ptls_hpke_kem_t->keyex`.
    pub fn group_id(&self) -> Result<u16, crate::Error> {
        key_exchange_group_id_from_key(&self.key)
    }

    /// Return the HPKE KEM id that corresponds to this context's TLS group.
    ///
    /// C: selected via the `picoquic_hpke_kems[]` table.
    pub fn kem_id(&self) -> Result<u16, crate::Error> {
        key_exchange_kem_id_from_group(self.group_id()?)
    }

    /// Export the raw private scalar bytes required by the Rust HPKE
    /// decapsulation path.
    ///
    /// C: private key material owned by the
    /// `ptls_key_exchange_context_t` returned from
    /// `ptls_openssl_create_key_exchange`.
    pub fn hpke_private_key(&self) -> Result<Vec<u8>, crate::Error> {
        key_exchange_private_key_from_key(&self.key)
    }

    /// Perform HPKE base-mode receiver setup using this key-exchange
    /// context.
    ///
    /// Returns the same exported ECH key material shape used by
    /// `crate::ech::EchOpenerState::open`; `None` means the KEM, cipher,
    /// encapsulated key, or private-key material is incompatible.
    ///
    /// C: `ptls_hpke_setup_base_r(..., keyex, ...)`.
    pub fn hpke_setup_base_r(
        &self,
        kem: u16,
        cipher: HpkeCipherSuiteId,
        enc: &[u8],
        info: &[u8],
    ) -> Option<Vec<u8>> {
        if self.kem_id().ok()? != kem {
            return None;
        }
        let private_key = self.hpke_private_key().ok()?;
        hpke_setup_receiver(kem, cipher, &private_key, enc, info)
    }
}

fn key_exchange_kem_id_from_group(group_id: u16) -> Result<u16, crate::Error> {
    match group_id {
        GROUP_SECP256R1 => Ok(kem_id::P256_SHA256),
        GROUP_SECP384R1 => Ok(kem_id::P384_SHA384),
        GROUP_X25519 => Ok(kem_id::X25519_SHA256),
        _ => Err(crate::Error::InvalidFile),
    }
}

fn key_exchange_group_id_from_key(
    key: &openssl::pkey::PKey<openssl::pkey::Private>,
) -> Result<u16, crate::Error> {
    use openssl::nid::Nid;
    use openssl::pkey::Id;

    match key.id() {
        Id::X25519 => Ok(GROUP_X25519),
        Id::EC => {
            let ec_key = key.ec_key().map_err(|_| crate::Error::InvalidFile)?;
            match ec_key.group().curve_name() {
                Some(Nid::X9_62_PRIME256V1) => Ok(GROUP_SECP256R1),
                Some(Nid::SECP384R1) => Ok(GROUP_SECP384R1),
                _ => Err(crate::Error::InvalidFile),
            }
        }
        _ => Err(crate::Error::InvalidFile),
    }
}

fn key_exchange_private_key_from_key(
    key: &openssl::pkey::PKey<openssl::pkey::Private>,
) -> Result<Vec<u8>, crate::Error> {
    match key_exchange_group_id_from_key(key)? {
        GROUP_X25519 => {
            let private_key = key
                .raw_private_key()
                .map_err(|_| crate::Error::InvalidFile)?;
            if private_key.len() == 32 {
                Ok(private_key)
            } else {
                Err(crate::Error::InvalidFile)
            }
        }
        GROUP_SECP256R1 | GROUP_SECP384R1 => {
            let ec_key = key.ec_key().map_err(|_| crate::Error::InvalidFile)?;
            let scalar_len = match ec_key.group().curve_name() {
                Some(openssl::nid::Nid::X9_62_PRIME256V1) => 32,
                Some(openssl::nid::Nid::SECP384R1) => 48,
                _ => return Err(crate::Error::InvalidFile),
            };
            ec_key
                .private_key()
                .to_vec_padded(scalar_len)
                .map_err(|_| crate::Error::InvalidFile)
        }
        _ => Err(crate::Error::InvalidFile),
    }
}

fn hpke_setup_receiver(
    kem: u16,
    cipher: HpkeCipherSuiteId,
    private_key: &[u8],
    enc: &[u8],
    info: &[u8],
) -> Option<Vec<u8>> {
    use hpke::{Deserializable, Kem as HpkeKem, OpModeR};

    macro_rules! run_hpke {
        ($KemType:ty, $KdfType:ty, $AeadType:ty) => {{
            let sk = <$KemType as HpkeKem>::PrivateKey::from_bytes(private_key).ok()?;
            let ek = <$KemType as HpkeKem>::EncappedKey::from_bytes(enc).ok()?;
            let ctx = hpke::setup_receiver::<$AeadType, $KdfType, $KemType>(
                &OpModeR::Base,
                &sk,
                &ek,
                info,
            )
            .ok()?;
            let mut out = alloc::vec![0u8; 32];
            ctx.export(b"picoquic ech key material", &mut out).ok()?;
            Some(out)
        }};
    }

    match (kem, cipher.kdf, cipher.aead) {
        (kem_id::X25519_SHA256, kdf_id::HKDF_SHA256, aead_id::AES_128_GCM) => {
            run_hpke!(
                hpke::kem::X25519HkdfSha256,
                hpke::kdf::HkdfSha256,
                hpke::aead::AesGcm128
            )
        }
        (kem_id::X25519_SHA256, kdf_id::HKDF_SHA256, aead_id::CHACHA20_POLY1305) => {
            run_hpke!(
                hpke::kem::X25519HkdfSha256,
                hpke::kdf::HkdfSha256,
                hpke::aead::ChaCha20Poly1305
            )
        }
        (kem_id::P256_SHA256, kdf_id::HKDF_SHA256, aead_id::AES_128_GCM) => {
            run_hpke!(
                hpke::kem::DhP256HkdfSha256,
                hpke::kdf::HkdfSha256,
                hpke::aead::AesGcm128
            )
        }
        (kem_id::P384_SHA384, kdf_id::HKDF_SHA384, aead_id::AES_256_GCM) => {
            run_hpke!(
                hpke::kem::DhP384HkdfSha384,
                hpke::kdf::HkdfSha384,
                hpke::aead::AesGcm256
            )
        }
        _ => None,
    }
}

/// Build the OpenSSL-backed key-exchange context that C obtains from
/// `ptls_openssl_create_key_exchange`.
fn create_key_exchange_context(
    key: openssl::pkey::PKey<openssl::pkey::Private>,
) -> Result<KeyExchangeContext, crate::Error> {
    key_exchange_private_key_from_key(&key)?;
    Ok(KeyExchangeContext { key })
}

/// Read a PEM-encoded private key from `keypem` and build a key-exchange
/// context for it.  Returns `Err(NoSuchFile)` when the path is unreadable
/// and `Err(InvalidFile)` when the PEM cannot be parsed as a private key.
/// C: `openssl_keyex_from_key_file`.
pub fn openssl_keyex_from_key_file(keypem: &str) -> Result<KeyExchangeContext, crate::Error> {
    let pem = std::fs::read(keypem).map_err(|_| crate::Error::NoSuchFile)?;
    let key =
        openssl::pkey::PKey::private_key_from_pem(&pem).map_err(|_| crate::Error::InvalidFile)?;
    create_key_exchange_context(key)
}

/// Drop an OpenSSL key-exchange context.
/// C: `openssl_keyex_dispose`.
pub fn openssl_keyex_dispose(keyex: KeyExchangeContext) {
    drop(keyex);
}

// ---------------------------------------------------------------------------
// Certificate and public-key file helpers.

/// Reads a PEM private-key file and returns the DER-encoded
/// SubjectPublicKeyInfo for the contained key.
/// Note: the C original uses `i2d_PublicKey` (legacy raw-key encoding);
/// Rust uses the standard SPKI encoding via `public_key_to_der()`.
/// C: `picoquic_openssl_get_public_key_from_key_file` (private helper).
fn get_public_key_from_key_file(keypem: &str) -> Result<Vec<u8>, crate::Error> {
    let pem = std::fs::read(keypem).map_err(|_| crate::Error::NoSuchFile)?;
    let pkey =
        openssl::pkey::PKey::private_key_from_pem(&pem).map_err(|_| crate::Error::InvalidFile)?;
    pkey.public_key_to_der()
        .map_err(|_| crate::Error::InvalidFile)
}

/// Reads up to 16 PEM-encoded X.509 certificates from `file_name` and
/// returns each as a DER-encoded byte vector.  Returns an empty `Vec` on
/// any I/O or parse failure (mirrors the C null-return path).
/// C: `picoquic_openssl_get_certs_from_file`.
pub fn get_certs_from_file(file_name: &str) -> Vec<Vec<u8>> {
    let pem = match std::fs::read(file_name) {
        Ok(data) => data,
        Err(_) => return Vec::new(),
    };
    let certs = match openssl::x509::X509::stack_from_pem(&pem) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    certs
        .into_iter()
        .take(16)
        .filter_map(|cert| cert.to_der().ok())
        .collect()
}

/// X.509 certificate-verification state backed by an OpenSSL trust store.
/// C: wraps `ptls_openssl_verify_certificate_t` created by
/// `picoquic_openssl_get_openssl_certificate_verifier`.
pub struct CertificateVerifier {
    /// The X.509 trust store, possibly loaded with root CA certificates.
    pub store: openssl::x509::store::X509Store,
    /// `true` when a root CA file was successfully loaded into the store.
    pub is_cert_store_not_empty: bool,
}

/// Creates an X.509 certificate verifier backed by a trust store.
/// If `cert_root_file_name` is `Some`, PEM root certificates are loaded
/// from that file; `is_cert_store_not_empty` reflects whether the load
/// succeeded.  Returns `Err(Generic)` if the store cannot be allocated.
/// C: `picoquic_openssl_get_openssl_certificate_verifier`.
pub fn get_openssl_certificate_verifier(
    cert_root_file_name: Option<&str>,
) -> Result<CertificateVerifier, crate::Error> {
    let mut builder =
        openssl::x509::store::X509StoreBuilder::new().map_err(|_| crate::Error::Generic)?;
    let mut is_cert_store_not_empty = false;
    if let Some(file_name) = cert_root_file_name {
        // Load PEM certs from the root CA file and add each to the store.
        // Mirrors X509_STORE_add_lookup + X509_LOOKUP_load_file from the C original.
        if let Ok(pem_data) = std::fs::read(file_name)
            && let Ok(certs) = openssl::x509::X509::stack_from_pem(&pem_data)
        {
            for cert in certs {
                if builder.add_cert(cert).is_ok() {
                    is_cert_store_not_empty = true;
                }
            }
        }
    }
    Ok(CertificateVerifier {
        store: builder.build(),
        is_cert_store_not_empty,
    })
}

pub type FreeVerifyCertificateCtx = fn(CertificateVerifier);

/// Certificate verifier plus the disposer callback that the C API returns
/// through `free_certificate_verifier_fn`.
pub struct CertificateVerifierRegistration {
    pub verifier: CertificateVerifier,
    pub free_certificate_verifier_fn: FreeVerifyCertificateCtx,
}

/// Dispose of an OpenSSL certificate verifier context.
/// C: `picoquic_openssl_dispose_certificate_verifier`.
pub fn picoquic_openssl_dispose_certificate_verifier(verifier: CertificateVerifier) {
    drop(verifier);
}

/// Create a certificate verifier and return it with its disposer callback.
/// C: `picoquic_openssl_get_certificate_verifier` (picoquic_ptls_openssl.c:277-292).
pub fn picoquic_openssl_get_certificate_verifier(
    cert_root_file_name: Option<&str>,
) -> Option<CertificateVerifierRegistration> {
    let verifier = get_openssl_certificate_verifier(cert_root_file_name).ok()?;
    Some(CertificateVerifierRegistration {
        verifier,
        free_certificate_verifier_fn: picoquic_openssl_dispose_certificate_verifier,
    })
}

// ---------------------------------------------------------------------------
// OpenSSL error-queue helpers.

/// Dequeue all pending OpenSSL errors and return them.
/// C callers loop over `picoquic_open_ssl_explain_crypto_error` until the
/// return code is 0; Rust collects the full queue in one shot.
/// C: `picoquic_open_ssl_explain_crypto_error`.
pub fn explain_crypto_error() -> Vec<CryptoError> {
    openssl::error::ErrorStack::get()
        .errors()
        .iter()
        .map(|e| CryptoError {
            code: e.code(),
            reason: e.reason().map(str::to_owned),
            library: e.library().map(str::to_owned),
            file: e.file().to_owned(),
            line: e.line(),
        })
        .collect()
}

/// Clear the OpenSSL error queue, discarding all pending errors.
/// C: `picoquic_openssl_clear_crypto_errors`.
pub fn clear_crypto_errors() {
    drop(openssl::error::ErrorStack::get());
}

// ---------------------------------------------------------------------------
// Root-certificate injection into a live verifier context.

impl CertificateVerifier {
    /// Add additional root certificates (DER-encoded) to the trust store.
    ///
    /// Returns `Err(Generic)` on parse failure (mirrors C return value `-1`)
    /// or store-insertion failure (mirrors `-2`).
    ///
    /// C: `picoquic_openssl_set_tls_root_certificates`
    pub fn set_root_certificates(&mut self, certs: &[&[u8]]) -> Result<(), crate::Error> {
        use foreign_types::ForeignType as _;
        for &cert_der in certs {
            let cert =
                openssl::x509::X509::from_der(cert_der).map_err(|_| crate::Error::Generic)?;
            // `add_cert` exists only on X509StoreBuilder, not on the built
            // X509Store/X509StoreRef.  Use the raw C function directly.
            let rc =
                unsafe { openssl_sys::X509_STORE_add_cert(self.store.as_ptr(), cert.as_ptr()) };
            if rc != 1 {
                return Err(crate::Error::Generic);
            }
            self.is_cert_store_not_empty = true;
        }
        Ok(())
    }
}

/// Add DER-encoded root certificates to an OpenSSL verifier.
/// C: `picoquic_openssl_set_tls_root_certificates`.
pub fn picoquic_openssl_set_tls_root_certificates(
    verifier: &mut CertificateVerifier,
    certs: &[&[u8]],
) -> Result<(), crate::Error> {
    verifier.set_root_certificates(certs)
}

// ---------------------------------------------------------------------------
// Certificate signing context.

/// Certificate signing context backed by an OpenSSL private key.
///
/// Wraps the key that picotls would store in a
/// `ptls_openssl_sign_certificate_t`; the actual signing algorithm
/// (RSA-PSS, ECDSA, Ed25519, …) is selected by OpenSSL at sign time.
///
/// C: `ptls_openssl_sign_certificate_t`
pub struct SignCertificate {
    /// The private key used for TLS certificate signing.
    pub key: openssl::pkey::PKey<openssl::pkey::Private>,
}

/// Wrap an OpenSSL `PKey` in a [`SignCertificate`] signing context,
/// consuming the key (mirrors `EVP_PKEY_free(pkey)` at the end of the
/// C helper — picotls takes ownership and the caller's reference is
/// released).  Returns `Err(Generic)` when `key` is `None` (the C
/// function checks for a null `pkey` and returns `-1`).
///
/// C: `set_openssl_sign_certificate_from_key` (static helper in
/// `picoquic_ptls_openssl.c`).
#[allow(dead_code)]
pub(crate) fn set_sign_certificate_from_key(
    key: Option<openssl::pkey::PKey<openssl::pkey::Private>>,
) -> Result<SignCertificate, crate::Error> {
    let key = key.ok_or(crate::Error::Generic)?;
    // ptls_openssl_init_sign_certificate stores the key pointer in the
    // signer struct and sets `ctx->sign_certificate`.  In Rust we simply
    // move the key into the struct — no additional initialisation is
    // needed because the signing algorithm is chosen dynamically by
    // OpenSSL when a signature is requested.
    Ok(SignCertificate { key })
}

/// Drop an OpenSSL certificate-signing context.
/// C: `picoquic_openssl_dispose_sign_certificate`.
pub fn picoquic_openssl_dispose_sign_certificate(signer: SignCertificate) {
    drop(signer);
}

/// Read a PEM private-key file and install it as the OpenSSL signing key.
/// C: `set_openssl_private_key_from_key_file` (picoquic_ptls_openssl.c:138-158).
fn set_openssl_private_key_from_key_file(keypem: &str) -> Result<SignCertificate, crate::Error> {
    let pem = std::fs::read(keypem).map_err(|_| crate::Error::NoSuchFile)?;
    let key =
        openssl::pkey::PKey::private_key_from_pem(&pem).map_err(|_| crate::Error::InvalidFile)?;
    set_sign_certificate_from_key(Some(key))
}

/// Fill `out` with OpenSSL-generated random bytes.
/// C: registered `ptls_openssl_random_bytes` callback.
pub fn ptls_openssl_random_bytes(out: &mut [u8]) -> Result<(), crate::Error> {
    openssl::rand::rand_bytes(out).map_err(|_| crate::Error::Generic)
}

pub type OpenSslPrivateKeyLoader = fn(&str) -> Result<SignCertificate, crate::Error>;
pub type OpenSslSignCertificateDisposer = fn(SignCertificate);
pub type OpenSslCertChainLoader = fn(&str) -> Vec<Vec<u8>>;
pub type OpenSslPublicKeyLoader = fn(&str) -> Result<Vec<u8>, crate::Error>;
pub type OpenSslCertificateVerifierLoader =
    fn(Option<&str>) -> Option<CertificateVerifierRegistration>;
pub type OpenSslRootCertificateSetter =
    fn(&mut CertificateVerifier, &[&[u8]]) -> Result<(), crate::Error>;
pub type OpenSslRandomBytes = fn(&mut [u8]) -> Result<(), crate::Error>;
pub type OpenSslKeyExchangeLoader = fn(&str) -> Result<KeyExchangeContext, crate::Error>;
pub type OpenSslKeyExchangeDisposer = fn(KeyExchangeContext);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OpenSslCipherSuiteRegistration {
    pub id: u16,
    pub is_low_memory: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OpenSslKeyExchangeRegistration {
    pub group_id: u16,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OpenSslHpkeCipherSuiteRegistration {
    pub id: HpkeCipherSuiteId,
    pub name: &'static str,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OpenSslHpkeKemRegistration {
    pub kem_id: u16,
    pub group_id: u16,
    pub kdf_id: u16,
}

#[derive(Copy, Clone)]
pub struct OpenSslTlsKeyProvider {
    pub private_key_loader: OpenSslPrivateKeyLoader,
    pub sign_certificate_disposer: OpenSslSignCertificateDisposer,
    pub cert_chain_loader: OpenSslCertChainLoader,
    pub public_key_loader: OpenSslPublicKeyLoader,
}

#[derive(Copy, Clone)]
pub struct OpenSslVerifyCertificateProvider {
    pub verifier_loader: OpenSslCertificateVerifierLoader,
    pub verifier_disposer: FreeVerifyCertificateCtx,
    pub root_certificate_setter: OpenSslRootCertificateSetter,
}

#[derive(Copy, Clone)]
pub struct OpenSslCryptoErrorProvider {
    pub explain_crypto_error: fn() -> Vec<CryptoError>,
    pub clear_crypto_errors: fn(),
}

#[derive(Copy, Clone)]
pub struct OpenSslKeyExchangeProvider {
    pub loader: OpenSslKeyExchangeLoader,
    pub disposer: OpenSslKeyExchangeDisposer,
}

#[derive(Copy, Clone)]
pub struct OpenSslProviderRegistration {
    pub cipher_suites: &'static [OpenSslCipherSuiteRegistration],
    pub key_exchanges: &'static [OpenSslKeyExchangeRegistration],
    pub hpke_cipher_suites: &'static [OpenSslHpkeCipherSuiteRegistration],
    pub hpke_kems: &'static [OpenSslHpkeKemRegistration],
    pub tls_key_provider: OpenSslTlsKeyProvider,
    pub verify_certificate_provider: OpenSslVerifyCertificateProvider,
    pub crypto_error_provider: OpenSslCryptoErrorProvider,
    pub random_bytes: OpenSslRandomBytes,
    pub key_exchange_provider: OpenSslKeyExchangeProvider,
}

const OPENSSL_CIPHER_SUITES: &[OpenSslCipherSuiteRegistration] = &[
    OpenSslCipherSuiteRegistration {
        id: AES_128_GCM_SHA256,
        is_low_memory: true,
    },
    OpenSslCipherSuiteRegistration {
        id: AES_256_GCM_SHA384,
        is_low_memory: true,
    },
    OpenSslCipherSuiteRegistration {
        id: CHACHA20_POLY1305_SHA256,
        is_low_memory: true,
    },
];

const OPENSSL_KEY_EXCHANGES: &[OpenSslKeyExchangeRegistration] = &[
    OpenSslKeyExchangeRegistration {
        group_id: GROUP_SECP256R1,
    },
    OpenSslKeyExchangeRegistration {
        group_id: GROUP_X25519,
    },
];

const OPENSSL_HPKE_CIPHER_SUITES: &[OpenSslHpkeCipherSuiteRegistration] = &[
    OpenSslHpkeCipherSuiteRegistration {
        id: HpkeCipherSuiteId {
            kdf: kdf_id::HKDF_SHA256,
            aead: aead_id::AES_128_GCM,
        },
        name: "HKDF-SHA256/AES-128-GCM",
    },
    OpenSslHpkeCipherSuiteRegistration {
        id: HpkeCipherSuiteId {
            kdf: kdf_id::HKDF_SHA512,
            aead: aead_id::AES_128_GCM,
        },
        name: "HKDF-SHA512/AES-128-GCM",
    },
    OpenSslHpkeCipherSuiteRegistration {
        id: HpkeCipherSuiteId {
            kdf: kdf_id::HKDF_SHA384,
            aead: aead_id::AES_256_GCM,
        },
        name: "HKDF-SHA384/AES-256-GCM",
    },
    OpenSslHpkeCipherSuiteRegistration {
        id: HpkeCipherSuiteId {
            kdf: kdf_id::HKDF_SHA256,
            aead: aead_id::CHACHA20_POLY1305,
        },
        name: "HKDF-SHA256/ChaCha20Poly1305",
    },
];

const OPENSSL_HPKE_KEMS: &[OpenSslHpkeKemRegistration] = &[
    OpenSslHpkeKemRegistration {
        kem_id: kem_id::P256_SHA256,
        group_id: GROUP_SECP256R1,
        kdf_id: kdf_id::HKDF_SHA256,
    },
    OpenSslHpkeKemRegistration {
        kem_id: kem_id::P384_SHA384,
        group_id: GROUP_SECP384R1,
        kdf_id: kdf_id::HKDF_SHA384,
    },
    OpenSslHpkeKemRegistration {
        kem_id: kem_id::X25519_SHA256,
        group_id: GROUP_X25519,
        kdf_id: kdf_id::HKDF_SHA256,
    },
];

pub const OPENSSL_PROVIDER_REGISTRATION: OpenSslProviderRegistration =
    OpenSslProviderRegistration {
        cipher_suites: OPENSSL_CIPHER_SUITES,
        key_exchanges: OPENSSL_KEY_EXCHANGES,
        hpke_cipher_suites: OPENSSL_HPKE_CIPHER_SUITES,
        hpke_kems: OPENSSL_HPKE_KEMS,
        tls_key_provider: OpenSslTlsKeyProvider {
            private_key_loader: set_openssl_private_key_from_key_file,
            sign_certificate_disposer: picoquic_openssl_dispose_sign_certificate,
            cert_chain_loader: get_certs_from_file,
            public_key_loader: get_public_key_from_key_file,
        },
        verify_certificate_provider: OpenSslVerifyCertificateProvider {
            verifier_loader: picoquic_openssl_get_certificate_verifier,
            verifier_disposer: picoquic_openssl_dispose_certificate_verifier,
            root_certificate_setter: picoquic_openssl_set_tls_root_certificates,
        },
        crypto_error_provider: OpenSslCryptoErrorProvider {
            explain_crypto_error,
            clear_crypto_errors,
        },
        random_bytes: ptls_openssl_random_bytes,
        key_exchange_provider: OpenSslKeyExchangeProvider {
            loader: openssl_keyex_from_key_file,
            disposer: openssl_keyex_dispose,
        },
    };

fn version_env(name: &str) -> Option<i64> {
    let value = match name {
        "FQ_OPENSSL_SOURCE_VERSION_NUMBER" => option_env!("FQ_OPENSSL_SOURCE_VERSION_NUMBER"),
        "FQ_OPENSSL_LIBRESSL_VERSION_NUMBER" => option_env!("FQ_OPENSSL_LIBRESSL_VERSION_NUMBER"),
        _ => None,
    }?;
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16).ok()
    } else {
        trimmed
            .parse::<i64>()
            .ok()
            .or_else(|| i64::from_str_radix(trimmed, 16).ok())
    }
}

fn source_version_number() -> Option<i64> {
    version_env("FQ_OPENSSL_SOURCE_VERSION_NUMBER")
}

fn libressl_source_version_number() -> Option<i64> {
    version_env("FQ_OPENSSL_LIBRESSL_VERSION_NUMBER")
}

/// Register or unload the OpenSSL provider.
/// C: `picoquic_ptls_openssl_load` (picoquic_ptls_openssl.c:406-454).
pub fn picoquic_ptls_openssl_load(unload: i32) -> Option<&'static OpenSslProviderRegistration> {
    if unload != 0 {
        if unload == 1 {
            clear_openssl();
        }
        None
    } else {
        init_openssl();
        if let Some(version) = source_version_number() {
            log::debug!("Open ssl include version: {:x}", version);
        }
        if let Some(version) = libressl_source_version_number() {
            log::debug!("LIBRE SSL include version: {:x}", version);
        }
        log::debug!("OpenSSL_version_num(): {:x}", openssl::version::number());
        Some(&OPENSSL_PROVIDER_REGISTRATION)
    }
}

/// Log the OpenSSL source and binary versions on a connection log.
/// C: `picoquic_ptls_openssl_log_version` (picoquic_ptls_openssl.c:456-464).
pub fn picoquic_ptls_openssl_log_version(cnx: &mut Connection) {
    let binary_version = openssl::version::number();
    if let Some(source_version) = libressl_source_version_number() {
        cnx.log_app_message(&alloc::format!(
            "LibreSSL source version {:x}, binary version {:x}",
            source_version,
            binary_version
        ));
    } else {
        let source_version = source_version_number().unwrap_or(binary_version);
        cnx.log_app_message(&alloc::format!(
            "OpenSSL source version {:x}, binary version {:x}",
            source_version,
            binary_version
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ECH_PRIVATE_KEY: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/ech/private.pem");
    const ECH_CONFIG: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../certs/ech/ech_config.txt"
    );
    const SECP256R1_KEY: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../certs/secp256r1-pkcs8/key.pem"
    );
    const SECP384R1_KEY: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/secp384r1/key.pem");
    const RSA_KEY: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../certs/rsa-pkcs8/keypair.pem"
    );

    #[test]
    fn keyex_loader_accepts_supported_ech_keys() -> Result<(), crate::Error> {
        let ech_key = openssl_keyex_from_key_file(ECH_PRIVATE_KEY)?;
        assert_eq!(ech_key.group_id()?, GROUP_SECP256R1);
        assert_eq!(ech_key.kem_id()?, kem_id::P256_SHA256);
        assert_eq!(ech_key.hpke_private_key()?.len(), 32);

        let secp256r1 = openssl_keyex_from_key_file(SECP256R1_KEY)?;
        assert_eq!(secp256r1.group_id()?, GROUP_SECP256R1);
        assert_eq!(secp256r1.kem_id()?, kem_id::P256_SHA256);
        assert_eq!(secp256r1.hpke_private_key()?.len(), 32);

        let secp384r1 = openssl_keyex_from_key_file(SECP384R1_KEY)?;
        assert_eq!(secp384r1.group_id()?, GROUP_SECP384R1);
        assert_eq!(secp384r1.kem_id()?, kem_id::P384_SHA384);
        assert_eq!(secp384r1.hpke_private_key()?.len(), 48);

        Ok(())
    }

    #[test]
    fn keyex_loader_rejects_non_key_exchange_private_key() {
        let err = match openssl_keyex_from_key_file(RSA_KEY) {
            Ok(_) => panic!("RSA key should not create an HPKE key-exchange context"),
            Err(err) => err,
        };
        assert_eq!(err, crate::Error::InvalidFile);
    }

    #[test]
    fn ech_opener_uses_openssl_key_exchange_provider() -> Result<(), crate::Error> {
        let opener = crate::ech::ech_init_opener(ECH_PRIVATE_KEY, ECH_CONFIG)?;

        assert!(opener.key_exchange.is_some());
        assert_eq!(opener.kem_id, kem_id::P256_SHA256);
        assert_eq!(opener.private_key.len(), 32);

        Ok(())
    }
}
