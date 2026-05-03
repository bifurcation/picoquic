//! SipHash-2-4 reference implementation, vendored from the upstream
//! public-domain release.  The C source exposes a single `siphash`
//! function whose `outlen` parameter selects between the 64-bit and
//! 128-bit digest variants; only the 64-bit variant has any in-tree
//! caller, so the Rust API drops the 128-bit form entirely and
//! returns the 64-bit digest as a `u64` (every call site immediately
//! converted the byte digest with `from_le_bytes`).
//!
//! Phase 1 contract: signatures only; the body is `todo!()`.
//!
//! Phase 4 should consider replacing this hand-rolled implementation
//! with the `siphasher` crate, which exposes the same algorithm
//! behind a `Hasher` interface and is no_std-friendly.

/// Compute the 64-bit SipHash-2-4 digest of `input` under `key`.
///
/// The 128-bit secret `key` is taken by reference to a fixed-size
/// array because the algorithm reads exactly 16 bytes.
///
/// C: `siphash(..., outlen = 8)` followed by `from_le_bytes` at every
/// call site.
pub fn siphash(_input: &[u8], _key: &[u8; 16]) -> u64 {
    todo!()
}

#[cfg(test)]
mod test {}
