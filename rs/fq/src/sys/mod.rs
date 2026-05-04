//! Concrete implementations of [`crate::tls`] / [`crate::crypto`]
//! traits, one submodule per backend.
//!
//! The application picks one of these at QUIC-context construction
//! time and hands it the [`crate::tls::TlsBackend`] surface; the
//! rest of picoquic stays generic over the chosen backend.
//!
//! Submodules are gated by Cargo features so a binary only pays
//! for the backends it actually uses.
//!
//! Phase 1 contract: signatures only — every body is `todo!()`.

#[cfg(feature = "sys-picotls")]
pub mod picotls;

#[cfg(feature = "sys-openssl")]
pub mod openssl;
