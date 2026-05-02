//! SipHash-2-4 reference implementation, vendored from the upstream
//! public-domain release.  The C source exposes a single `siphash`
//! function whose `outlen` parameter selects between the 64-bit and
//! 128-bit digest variants; the Rust API splits that into two
//! functions so the digest size is fixed at the type level and
//! neither a runtime assertion nor a fallible return is needed.
//!
//! Phase 1 contract: signatures only; the bodies are `todo!()`.

/// Compute the 64-bit SipHash-2-4 digest of `input` under `key`.
///
/// The 128-bit secret `key` is taken by reference to a fixed-size
/// array because the algorithm reads exactly 16 bytes.  The digest
/// is returned by value as a little-endian byte array — callers
/// that want a `u64` can apply [`u64::from_le_bytes`].
///
/// C: `siphash(..., outlen = 8)`.
pub fn siphash_64(_input: &[u8], _key: &[u8; 16]) -> [u8; 8] {
    todo!()
}

/// Compute the 128-bit SipHash-2-4 digest of `input` under `key`.
///
/// Same shape as [`siphash_64`] but produces a 16-byte digest.
/// No in-tree caller currently uses this variant, but the C API
/// exposes it and the algorithm supports it natively.
///
/// C: `siphash(..., outlen = 16)`.
pub fn siphash_128(_input: &[u8], _key: &[u8; 16]) -> [u8; 16] {
    todo!()
}

#[cfg(test)]
mod test {}
