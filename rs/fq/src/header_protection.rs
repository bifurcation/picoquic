//! QUIC header protection (packet-number encryption).
//!
//! Per RFC 9001 §5.4, header protection masks five bytes of the
//! packet header (the first byte's low bits plus the packet
//! number) with output derived from the AEAD-encrypted payload.
//! The mask comes from running a sample of the ciphertext
//! through a *non-AEAD* cipher: AES-128/256 ECB for AES suites,
//! ChaCha20 with the sample as a nonce for ChaCha20 suites.
//!
//! Phase 2 decision (per the plan): header protection is QUIC
//! logic, not TLS-stack logic.  We implement it ourselves in
//! this crate over the `cipher` crate's primitives, rather
//! than delegating to the TLS backend.  Backends supply only
//! the *key bytes* derived from the handshake; this module
//! holds the algorithm.

extern crate alloc;
use alloc::boxed::Box;

use cipher::{BlockEncrypt, KeyInit};

use crate::Error;
use crate::tls::HeaderKey;

// ---------------------------------------------------------------------------
// AES-based header protection (cipher suites
// `TLS_AES_128_GCM_SHA256` and `TLS_AES_256_GCM_SHA384`).

/// AES-ECB header protector.  `C` is `aes::Aes128` or
/// `aes::Aes256`; both implement
/// [`cipher::BlockEncrypt<BlockSize = U16>`].
pub struct AesHeaderProtector<C> {
    #[allow(dead_code)] // Phase 4 wires this through to the body.
    cipher: C,
}

impl<C> AesHeaderProtector<C>
where
    C: KeyInit + BlockEncrypt + Send,
{
    /// Construct from a freshly-derived header-protection key.
    pub fn new(_key: &[u8]) -> Result<Self, Error> {
        todo!()
    }

    /// Compute the 16-byte mask from a 16-byte sample.  QUIC
    /// uses bytes `[0..5]` of the result; the rest are wasted
    /// but the AES API gives us a full block.
    pub fn mask(&self, _sample: &[u8; 16]) -> [u8; 16] {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// ChaCha20-based header protection (cipher suite
// `TLS_CHACHA20_POLY1305_SHA256`).
//
// The header-protection cipher for ChaCha20 suites is plain
// ChaCha20 (no Poly1305): the first 4 bytes of the sample are
// the block counter, the next 12 bytes are the nonce, and the
// "mask" is the first 16 bytes of the keystream.

/// ChaCha20 header protector.
pub struct ChaCha20HeaderProtector {
    #[allow(dead_code)] // Phase 4 wires this through to the body.
    key: [u8; 32],
}

impl ChaCha20HeaderProtector {
    pub fn new(_key: &[u8]) -> Result<Self, Error> {
        todo!()
    }

    pub fn mask(&self, _sample: &[u8; 16]) -> [u8; 16] {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// `HeaderKey` impls.
//
// Each protector implements [`crate::tls::HeaderKey`] so it can
// be used through the TLS abstraction.  The encrypt / decrypt
// methods walk the packet header per RFC 9001 §5.4.

/// One direction of AES-based header protection wrapped as a
/// [`HeaderKey`].
pub struct AesHeaderKey<C> {
    #[allow(dead_code)]
    protector: AesHeaderProtector<C>,
}

impl<C> AesHeaderKey<C>
where
    C: KeyInit + BlockEncrypt + Send + 'static,
{
    /// Construct an [`AesHeaderKey`] boxed as a `HeaderKey`
    /// trait object.  Named `boxed` (not `new`) so clippy
    /// doesn't grumble that `new` should return `Self`.
    pub fn boxed(key: &[u8]) -> Result<Box<dyn HeaderKey>, Error> {
        Ok(Box::new(Self {
            protector: AesHeaderProtector::<C>::new(key)?,
        }))
    }
}

impl<C> HeaderKey for AesHeaderKey<C>
where
    C: KeyInit + BlockEncrypt + Send,
{
    fn mask(&self, sample: [u8; 16]) -> [u8; 16] {
        self.protector.mask(&sample)
    }
}

/// One direction of ChaCha20-based header protection wrapped
/// as a [`HeaderKey`].
pub struct ChaCha20HeaderKey {
    #[allow(dead_code)]
    protector: ChaCha20HeaderProtector,
}

impl ChaCha20HeaderKey {
    /// Construct a [`ChaCha20HeaderKey`] boxed as a `HeaderKey`
    /// trait object.
    pub fn boxed(key: &[u8]) -> Result<Box<dyn HeaderKey>, Error> {
        Ok(Box::new(Self {
            protector: ChaCha20HeaderProtector::new(key)?,
        }))
    }
}

impl HeaderKey for ChaCha20HeaderKey {
    fn mask(&self, sample: [u8; 16]) -> [u8; 16] {
        self.protector.mask(&sample)
    }
}

#[cfg(test)]
mod test {}
