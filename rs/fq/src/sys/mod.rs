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
//! This module only declares backend submodules; concrete backend
//! implementations live behind their feature gates.

#[cfg(feature = "sys-picotls")]
pub mod picotls;

#[cfg(feature = "sys-openssl")]
pub mod openssl;

#[cfg(feature = "sys-mbedtls")]
pub mod mbedtls;

/// Load or unload the mbedtls crypto provider.
///
/// C: `picoquic_mbedtls_load`.
#[cfg(feature = "sys-mbedtls")]
pub fn mbedtls_load(unload: bool) {
    mbedtls::load(unload);
}

/// No-op when the backend is not compiled in, matching the C
/// `#ifndef PICOQUIC_WITH_MBEDTLS` body.
#[cfg(not(feature = "sys-mbedtls"))]
pub fn mbedtls_load(_unload: bool) {}
