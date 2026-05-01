//! Translation of `quic/siphash.h`.
//!
//! The SipHash-2-4 reference implementation, vendored into quic
//! verbatim from the upstream public-domain release.  quic only
//! uses it through `hash_siphash` (`hash.c:204`), which
//! always asks for an 8-byte digest, but the underlying primitive
//! supports the 16-byte variant too — both are kept here so the
//! Rust translation stays a one-to-one mirror of the C source.
//!
//! Phase 1 contract: signature only; the body is `todo!()`.

/// Compute a SipHash digest.
///
/// * `input` — message bytes.  C: `const void *in` plus `size_t
///   inlen`; the implementation only reads through them, never
///   writes, and the length-zero case is well-defined.
/// * `key` — the 128-bit secret key.  C took `const void *k` and
///   then read exactly 16 bytes; the only caller in-tree
///   (`hash.c:208`) hands over a 16-byte buffer, so the safe
///   shape is a fixed-size array reference.
/// * `out` — destination digest.  C took `uint8_t *out` plus
///   `size_t outlen`, asserting `outlen ∈ {8, 16}`.  The slice
///   carries its length so the runtime check moves into the body.
///   `&mut [u8]` matches both call patterns quic uses (an 8-byte
///   stack buffer in `hash_siphash`) and any future 16-byte
///   user.
///
/// Returns `Ok(())` on success.  The C signature is `int` and the
/// body always returns `0`; the `Result<(), ()>` shape is the
/// translation plan's placeholder until the crate-level `Error` enum
/// exists.  Phase 3 will likely upgrade the assertion on `out.len()`
/// into a real error variant.
// TODO(error-enum): swap `()` for the crate's `Error` once it lands.
#[allow(clippy::result_unit_err)]
pub fn siphash(_input: &[u8], _key: &[u8; 16], _out: &mut [u8]) -> Result<(), ()> {
    todo!()
}

#[cfg(test)]
mod test {}
