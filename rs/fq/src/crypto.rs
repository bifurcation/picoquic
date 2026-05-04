//! Pure trait surface for crypto primitives.
//!
//! This module captures the *shape* of the crypto operations
//! picoquic needs from a backend.  It contains no
//! implementation-specific code — picotls, OpenSSL, minicrypto,
//! and so on each provide concrete impls under [`crate::sys`].
//!
//! TLS-layer trait surface (sessions, packet/header protection,
//! application callbacks) lives in [`crate::tls`]; this module
//! covers the lower-level crypto primitives that the TLS backend
//! and a few QUIC-specific paths (lb CID encryption, retry
//! integrity, initial-secret derivation) reach for directly.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.

extern crate alloc;
use alloc::boxed::Box;

use crate::Error;

// ---------------------------------------------------------------------------
// Cipher suites.

/// One QUIC-eligible TLS 1.3 cipher suite.  Each backend exposes
/// one [`CipherSuite`] impl per supported suite (AES-128-GCM-SHA256,
/// AES-256-GCM-SHA384, ChaCha20-Poly1305-SHA256).
///
/// QUIC pins the AEAD tag length to 16 bytes (RFC 9001 §5.3) and
/// the initial-secret HKDF to SHA-256 — so the suite trait only
/// has to expose the negotiated AEAD/header-protection key
/// constructors and the hash for traffic-secret rotation.
pub trait CipherSuite: Send + Sync {
    /// IANA cipher-suite code (e.g. `0x1301` for
    /// AES-128-GCM-SHA256).
    fn id(&self) -> u16;

    /// Construct a fresh streaming-hash instance for this suite's
    /// negotiated hash algorithm.  Used by the traffic-secret
    /// rotation path.
    fn new_hash(&self) -> Box<dyn digest::DynDigest>;

    /// Build a packet-protection AEAD key from `secret`.  C:
    /// `aead_create_*`.
    fn packet_key(
        &self,
        is_encrypt: bool,
        secret: &[u8],
    ) -> Result<Box<dyn crate::tls::PacketKey>, Error>;

    /// Build a header-protection key from `secret`.
    fn header_key(&self, secret: &[u8]) -> Result<Box<dyn crate::tls::HeaderKey>, Error>;
}

// ---------------------------------------------------------------------------
// Crypto-error reporting.

/// Optional backend hook for surfacing the latest crypto error in
/// human-readable form.  Backends that maintain a thread-local
/// error queue (OpenSSL) use this; backends that don't can return
/// `None`.  C: `picoquic_explain_crypto_error_t` /
/// `picoquic_clear_crypto_errors_t`.
pub trait CryptoErrorReporter: Send + Sync {
    /// Most recent error as `(file_line, errno)`, or `None` if no
    /// error is pending.
    fn explain(&self) -> Option<(&'static str, i32)>;

    /// Drain the backend's pending-error queue.
    fn clear(&self);
}

#[cfg(test)]
mod test {}
