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

/// Load or unload the mbedtls crypto provider.  In the C implementation
/// this populates the global cipher-suite / key-exchange registry when
/// `PICOQUIC_WITH_MBEDTLS` is defined; without that flag the C body is
/// an explicit no-op (`/* Nothing to do, as the module is not loaded. */`).
/// Rust selects backends directly rather than through a global registry,
/// so this is a no-op until a `sys-mbedtls` feature and submodule are
/// added.
/// C: `picoquic_mbedtls_load`.
pub fn mbedtls_load(_unload: bool) {}
