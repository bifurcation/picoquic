//! Test cases for `picoquictest/hashtest.c`.

#![allow(non_snake_case)]

/// C: `siphash_test` in `picoquictest/hashtest.c`.
///
/// Known-answer test for [`crate::siphash::siphash`].  The C
/// reference values come from the upstream public-domain release.
#[test]
fn siphash() {
    use crate::siphash::siphash;

    let mut test = [0u8; 1024];
    let mut k = [0u8; 16];

    // Mirror the C `hash_test_init`: deterministic data + key.
    for (i, b) in test.iter_mut().enumerate() {
        *b = (i + (i >> 8)) as u8;
    }
    let k_len = k.len();
    for (i, b) in k.iter_mut().enumerate() {
        *b = (k_len - i) as u8;
    }

    let lengths = [1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024];
    let expected: [u64; 12] = [
        0xa9b7_8693_5f98_d6b8,
        0x3fb6_4f2d_81eb_f107,
        0xcd34_491a_7b43_7e1b,
        0x5fbe_9177_0928_6bc4,
        0xb2cc_76e0_f81d_6e2f,
        0x09e6_9c0f_7075_3651,
        0xc615_b534_9acc_0cc2,
        0x9653_79fb_0e26_e150,
        0x85a2_86cf_c4a6_2574,
        0x5f77_4367_aeea_9f83,
        0xd04e_e1d4_20e9_bc22,
        0x0a7a_d665_5680_779e,
    ];

    for (i, &len) in lengths.iter().enumerate() {
        let h = siphash(&test[..len], &k);
        assert_eq!(h, expected[i], "siphash[{i}] for len={len}");
    }
}

/// C: `picohash_test` in `picoquictest/hashtest.c`.
///
/// Drives the standard hash-table API ([`crate::hash::HashTable`])
/// through inserts, lookups, collision-handling, and deletions.
/// The C body uses a custom hash function `hashtest_hash` and a
/// custom comparator; the Rust counterpart parameterises on
/// `Hash + Eq` which the standard `i32` impls cover, so the
/// custom-function plumbing collapses.
#[test]
fn picohash() {
    todo!("picohash_test (HashTable API still settling)")
}

/// C: `picohash_embedded_test` in `picoquictest/hashtest.c`.
///
/// Same as [`picohash`] but with the embedded-item flavour
/// (`picohash_create_ex` with a `key_to_item` callback).  The
/// Rust hash table uses `HashToken` membership for the same
/// "found-it/where-is-it" invariant.
#[test]
fn picohash_embedded() {
    todo!("picohash_embedded_test (HashTable API still settling)")
}

/// C: `picohash_bytes_test` in `picoquictest/hashtest.c`.
///
/// Known-answer test for `picohash_bytes` — picoquic's
/// non-cryptographic byte-mixing hash used for misc bookkeeping.
/// Phase 4 will route this to the same `siphash` impl with a
/// non-secret key, matching the C behaviour.
#[test]
fn picohash_bytes() {
    todo!("picohash_bytes_test (no crate::hash::picohash_bytes counterpart yet)")
}
