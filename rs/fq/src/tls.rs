//! TLS-for-QUIC trait surface, modeled on `quinn-proto::crypto`.
//!
//! Quinn dependency: **none** — we lift the trait shapes
//! verbatim into our own module.  Borrow inspiration; don't
//! link the crate.
//!
//! This module defines the trait surface; concrete backends
//! (picotls, rustls, OpenSSL) live in sibling modules
//! (`tls_picotls.rs`, `tls_rustls.rs`, `tls_openssl.rs`)
//! gated by the matching Cargo features.
//!
//! Phase 4 keeps this module as the TLS trait contract; concrete
//! backend behavior lives in sibling modules.
//!
//! ## Structure
//!
//! * [`Session`] — per-connection TLS state.  One impl per
//!   backend; each backend wraps its native session type.
//!   Replaces the C `*mut c_void` `tls_ctx` and `tls_sendbuf`
//!   fields.
//!
//! * [`ClientConfig`] / [`ServerConfig`] — builder traits;
//!   pick one to start a [`Session`].
//!
//! * [`PacketKey`] — packet protection (the AEAD layer).
//!   Hides the underlying AEAD primitive from picoquic.
//!
//! * [`HeaderKey`] — header protection.  See also
//!   [`crate::header_protection::HeaderProtector`], which is the
//!   concrete implementation; backends supply `HeaderKey`s
//!   that delegate to it.
//!
//! * [`Keys`] / [`KeyPair`] — bundles of keys per epoch.
//!
//! * [`TlsCallbacks`] — application-supplied hooks the TLS
//!   layer invokes (ALPN selection, ticket store, certificate
//!   verification).  One trait, default impls everywhere
//!   except the genuinely application-specific bits.

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::Error;

// ---------------------------------------------------------------------------
// Per-connection TLS state.

/// Per-connection TLS state.  Drives the handshake forward,
/// surfaces handshake events, exposes negotiated keys.
///
/// Modeled on `quinn_proto::crypto::Session`.  The `Send` bound
/// is preserved so a connection can be moved between threads
/// even though picoquic itself is single-threaded in v1
/// (matters for v2 multi-threading and for users who run
/// multiple Quic contexts on different threads).
pub trait Session: Send {
    /// Read handshake bytes received from the peer.  Returns
    /// `Ok(true)` when the handshake transitions to a new
    /// epoch (new keys are now available via
    /// [`Self::next_1rtt_keys`] / [`Self::write_handshake`]).
    fn read_handshake(&mut self, plaintext: &[u8]) -> Result<bool, Error>;

    /// Write handshake bytes for the peer.  Returns
    /// `Some(Keys)` when an epoch transition produces fresh
    /// outbound keys.
    fn write_handshake(&mut self, buf: &mut Vec<u8>) -> Option<Keys>;

    /// `true` until the handshake completes.  After the
    /// handshake the session is in steady state and this
    /// returns `false`.
    fn is_handshaking(&self) -> bool;

    /// Negotiated 1-RTT keys.  Returns `Some` once after the
    /// handshake completes; subsequent calls return `None`
    /// until a key update.
    fn next_1rtt_keys(&mut self) -> Option<KeyPair>;

    /// Drain traffic-key updates produced since the last call.
    ///
    /// Backends call this side channel when their QUIC traffic-key
    /// callback fires.  The QUIC layer uses the events both to cache
    /// application traffic secrets for key rotation and to emit
    /// SSLKEYLOGFILE lines when key logging is configured.
    fn take_key_log_events(&mut self) -> Vec<KeyLogEvent> {
        Vec::new()
    }

    /// Server-side handshake parameters (SNI, ALPN, etc.).
    /// Available after the first ClientHello.
    fn handshake_data(&self) -> Option<HandshakeData>;

    /// Peer's certificate chain (or whatever identity material
    /// the backend exposes).  Available after the handshake.
    fn peer_identity(&self) -> Option<PeerIdentity>;

    /// Early-data (0-RTT) keys, if the session offers them.
    fn early_keys(&self) -> Option<(Box<dyn HeaderKey>, Box<dyn PacketKey>)>;

    /// Whether the server accepted the client's 0-RTT data.
    fn early_data_accepted(&self) -> Option<bool>;

    /// Decode the peer's QUIC transport parameters (TLS
    /// extension 57; RFC 9001 §8.2).  Returns the decoded
    /// transport parameters as raw bytes — picoquic parses
    /// them with its own decoder.
    fn transport_parameters(&self) -> Result<Option<Vec<u8>>, Error>;

    /// HKDF-Expand-Label using the session's exporter master
    /// secret.  Used by QUIC for stateless-reset tokens etc.
    fn export_keying_material(
        &self,
        label: &[u8],
        context: &[u8],
        output: &mut [u8],
    ) -> Result<(), Error>;

    /// Returns `true` when the TLS handshake used ECH.
    ///
    /// C: `ptls_is_ech_handshake(tls_ctx->tls, NULL, NULL, NULL)`
    fn is_ech_handshake(&self) -> bool {
        false
    }

    /// ECH retry-config bytes the server sent back.  Empty when none.
    ///
    /// C: `tls_ctx->retry_configs.{base,len}` (`picoquic_ech_get_retry_config`)
    fn retry_configs(&self) -> &[u8] {
        &[]
    }
}

/// One TLS traffic-secret update.
///
/// C: `picoquic_update_traffic_key_callback` receives `is_enc`,
/// `epoch`, the raw traffic secret, and reads the TLS ClientHello
/// random from picotls for SSLKEYLOGFILE output.
pub struct KeyLogEvent {
    /// `true` for the local/encrypt direction, `false` for
    /// remote/decrypt.
    pub is_enc: bool,
    /// QUIC crypto epoch: 1 = 0-RTT, 2 = handshake, 3 = 1-RTT.
    pub epoch: usize,
    /// TLS client random associated with this connection.
    pub client_random: [u8; 32],
    /// Raw traffic secret for this direction and epoch.
    pub secret: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Builder traits for client / server sessions.

/// Builder for client-side sessions.  Each backend exposes one
/// concrete `ClientConfig` type; the application constructs it
/// with backend-specific options (cert store, ALPN list, etc.)
/// and then calls [`ClientConfig::start_session`] per outgoing
/// connection.
pub trait ClientConfig {
    type Session: Session;

    /// Start a fresh client-side TLS session.
    ///
    /// * `version` — QUIC wire version (passed to TLS as the
    ///   transport-parameter version field).
    /// * `server_name` — SNI value.
    /// * `transport_params` — already-encoded transport-parameters
    ///   blob to ship in the TLS extension.
    fn start_session(
        &self,
        version: u32,
        server_name: &str,
        transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError>;
}

/// Builder for server-side sessions.
pub trait ServerConfig {
    type Session: Session;

    fn start_session(
        &self,
        version: u32,
        transport_params: &[u8],
    ) -> Result<Self::Session, ConfigError>;
}

/// Top-level backend trait.  Each backend (picotls, rustls,
/// OpenSSL) implements this once at the crate level.
///
/// In v1 `Quic` holds boxed [`DynClientConfig`] /
/// [`DynServerConfig`] trait objects (so a single binary can carry
/// multiple backends and pick at runtime).  An ergonomic generic
/// wrapper (`Quic<T: TlsBackend>`) for compile-time-only selection
/// is a candidate Phase-4 follow-up.
pub trait TlsBackend {
    type Client: ClientConfig;
    type Server: ServerConfig;
}

// ---------------------------------------------------------------------------
// Dyn-compatible wrappers around `ClientConfig` / `ServerConfig`.
//
// `ClientConfig::start_session` returns `Self::Session`, which makes
// the trait itself non-dyn-compatible.  The `Dyn*Config` wrappers
// re-export the same operation but Box the returned session so the
// caller doesn't need to know the concrete `Self::Session` type.
// Blanket impls plumb every `ClientConfig` / `ServerConfig` through
// to these.

/// Dyn-compatible mirror of [`ClientConfig`].
pub trait DynClientConfig: Send {
    /// Start a fresh client-side TLS session, boxing the result.
    fn start_session(
        &self,
        version: u32,
        server_name: &str,
        transport_params: &[u8],
    ) -> Result<Box<dyn Session>, ConfigError>;
}

impl<T> DynClientConfig for T
where
    T: ClientConfig + Send,
    T::Session: 'static,
{
    fn start_session(
        &self,
        version: u32,
        server_name: &str,
        transport_params: &[u8],
    ) -> Result<Box<dyn Session>, ConfigError> {
        ClientConfig::start_session(self, version, server_name, transport_params)
            .map(|s| Box::new(s) as Box<dyn Session>)
    }
}

/// Dyn-compatible mirror of [`ServerConfig`].
pub trait DynServerConfig: Send {
    fn start_session(
        &self,
        version: u32,
        transport_params: &[u8],
    ) -> Result<Box<dyn Session>, ConfigError>;
}

impl<T> DynServerConfig for T
where
    T: ServerConfig + Send,
    T::Session: 'static,
{
    fn start_session(
        &self,
        version: u32,
        transport_params: &[u8],
    ) -> Result<Box<dyn Session>, ConfigError> {
        ServerConfig::start_session(self, version, transport_params)
            .map(|s| Box::new(s) as Box<dyn Session>)
    }
}

// ---------------------------------------------------------------------------
// Packet protection (AEAD layer).

/// One direction of packet protection (client-to-server, or
/// vice versa).  Hides the AEAD primitive from the rest of
/// picoquic; backend implementations hold an `aead::AeadInPlace`
/// from a Rust Crypto crate.
pub trait PacketKey: Send {
    /// In-place encrypt: `payload[..]` is the plaintext on
    /// entry; on success the ciphertext + 16-byte auth tag
    /// fills `payload`.  `header` is the AAD.  `packet` is the
    /// QUIC packet sequence number, used to derive the AEAD
    /// nonce.
    fn encrypt(&self, packet: u64, header: &[u8], payload: &mut Vec<u8>);

    /// In-place decrypt + tag verify.  Shrinks `payload` by
    /// the tag length on success.
    fn decrypt(&self, packet: u64, header: &[u8], payload: &mut Vec<u8>) -> Result<(), Error>;

    /// Multipath variant of [`Self::encrypt`].  The C helper
    /// `picoquic_aead_encrypt_mp` xors the low 32 bits of `path_id`
    /// into the AEAD IV before applying the normal packet-number
    /// nonce transform.
    fn encrypt_mp(&self, path_id: u64, packet: u64, header: &[u8], payload: &mut Vec<u8>);

    /// Multipath variant of [`Self::decrypt`], matching
    /// `picoquic_aead_decrypt_mp`.
    fn decrypt_mp(
        &self,
        path_id: u64,
        packet: u64,
        header: &[u8],
        payload: &mut Vec<u8>,
    ) -> Result<(), Error>;

    /// AEAD tag length (16 for all QUIC-defined cipher suites).
    fn tag_len(&self) -> usize;

    /// Maximum number of packets that may be received before
    /// the integrity guarantee weakens (RFC 9001 §6.6).
    fn integrity_limit(&self) -> u64;

    /// Maximum number of packets that may be sent before the
    /// confidentiality guarantee weakens (RFC 9001 §6.6).
    fn confidentiality_limit(&self) -> u64;
}

/// Pair of keys for one direction (typically one for encrypt,
/// one for decrypt).  Both ends of a connection hold one
/// `KeyPair` per epoch.
pub struct KeyPair {
    /// Encrypt-side key (used for outbound packets).
    pub local: Box<dyn PacketKey>,
    /// Decrypt-side key (used for inbound packets).
    pub remote: Box<dyn PacketKey>,
}

// ---------------------------------------------------------------------------
// Header protection.

/// Header-protection key.  All QUIC cipher suites use a single AES
/// (or ChaCha20) ECB-style transform: feed it a 16-byte sample of
/// ciphertext and it returns a 16-byte mask whose first 5 bytes
/// XOR into the protected header.  The trait surface here is
/// exactly that bare cipher op — picoquic-side logic
/// ([`crate::header_protection`]) handles sampling, slicing the
/// header, and applying the mask.
pub trait HeaderKey: Send {
    /// Apply the ECB-style block to `sample`, producing the 16-byte
    /// header-protection mask.  RFC 9001 §5.4.
    fn mask(&self, sample: [u8; 16]) -> [u8; 16];
}

/// Bundle of keys associated with a single epoch.
pub struct Keys {
    pub header: KeyPairHeader,
    pub packet: KeyPair,
}

/// Pair of header-protection keys.
pub struct KeyPairHeader {
    pub local: Box<dyn HeaderKey>,
    pub remote: Box<dyn HeaderKey>,
}

// ---------------------------------------------------------------------------
// Side-channel data surfaced through `Session`.

/// Server-side handshake data.  Available after the first
/// ClientHello arrives.
pub struct HandshakeData {
    /// SNI (server-name-indication) value from the ClientHello.
    pub sni: Option<Vec<u8>>,
    /// Negotiated ALPN value, if any.
    pub alpn: Option<Vec<u8>>,
}

/// Peer identity (typically the certificate chain).
pub enum PeerIdentity {
    /// X.509 certificate chain.  Each element is one DER-encoded
    /// certificate.
    Certificates(Vec<Vec<u8>>),
}

// ---------------------------------------------------------------------------
// Application-supplied callbacks.

/// Hooks the TLS layer invokes on the application.  One trait
/// covers ALPN selection, the resumption-ticket store, and
/// certificate verification.  Default implementations cover
/// the simple cases; override for custom behavior.
///
/// The application installs an implementation at QUIC-context
/// construction time (one entry point — no C-style global
/// registry of function pointers).
pub trait TlsCallbacks: Send {
    /// Negotiate ALPN from the client's proposal list.  Default:
    /// pick the first proposal that matches the server's
    /// configured ALPN list (set on the backend's config).
    fn select_alpn(&mut self, _list: &[&[u8]]) -> Option<usize> {
        None
    }

    /// Server-side: look up a previously-issued session ticket
    /// keyed by the SNI / ALPN combination.  Default: return
    /// `None` (no resumption).
    fn lookup_ticket(&mut self, _sni: &str, _alpn: &str) -> Option<Vec<u8>> {
        None
    }

    /// Server-side: persist a freshly-issued session ticket.
    /// Default: discard.
    fn store_ticket(&mut self, _sni: &str, _alpn: &str, _ticket: &[u8]) {}

    /// Verify the peer certificate chain.  Default: trust any
    /// chain rooted in the system trust store (backends that
    /// don't have a system store provide their own default).
    fn verify_certificate(&mut self, _certs: &[&[u8]]) -> Result<(), Error> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Backend-construction errors.

/// Backend-construction failure (bad config, unsupported
/// version, etc.).  Distinct from the runtime-error
/// [`crate::Error`] so type-system can enforce that
/// `start_session` doesn't return a generic `Error`.
#[derive(Debug)]
pub struct ConfigError {
    pub message: alloc::string::String,
}

#[cfg(test)]
mod test {}
