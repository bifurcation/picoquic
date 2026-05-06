//! SipHash-2-4 reference implementation, vendored from the upstream
//! public-domain release.  The C source exposes a single `siphash`
//! function whose `outlen` parameter selects between the 64-bit and
//! 128-bit digest variants; only the 64-bit variant has any in-tree
//! caller, so the Rust API drops the 128-bit form entirely and
//! returns the 64-bit digest as a `u64` (every call site immediately
//! converted the byte digest with `from_le_bytes`).
//!
//! Phase 1 contract: signatures only; Phase 4 fills the body.
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
pub fn siphash(input: &[u8], key: &[u8; 16]) -> u64 {
    fn sip_round(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
        *v0 = v0.wrapping_add(*v1);
        *v1 = v1.rotate_left(13);
        *v1 ^= *v0;
        *v0 = v0.rotate_left(32);
        *v2 = v2.wrapping_add(*v3);
        *v3 = v3.rotate_left(16);
        *v3 ^= *v2;
        *v0 = v0.wrapping_add(*v3);
        *v3 = v3.rotate_left(21);
        *v3 ^= *v0;
        *v2 = v2.wrapping_add(*v1);
        *v1 = v1.rotate_left(17);
        *v1 ^= *v2;
        *v2 = v2.rotate_left(32);
    }

    let k0 = u64::from_le_bytes(key[..8].try_into().unwrap());
    let k1 = u64::from_le_bytes(key[8..16].try_into().unwrap());

    let mut v0: u64 = 0x736f6d6570736575;
    let mut v1: u64 = 0x646f72616e646f6d;
    let mut v2: u64 = 0x6c7967656e657261;
    let mut v3: u64 = 0x7465646279746573;

    v3 ^= k1;
    v2 ^= k0;
    v1 ^= k1;
    v0 ^= k0;

    let mut b: u64 = (input.len() as u64) << 56;
    let chunks = input.chunks_exact(8);
    let tail = chunks.remainder();

    for chunk in chunks {
        let m = u64::from_le_bytes(chunk.try_into().unwrap());
        v3 ^= m;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= m;
    }

    for (i, &byte) in tail.iter().enumerate() {
        b |= (byte as u64) << (i * 8);
    }

    v3 ^= b;
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= b;
    v2 ^= 0xff;
    for _ in 0..4 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[cfg(test)]
mod test {}
