//! Test cases for `picoquictest/hashtest.c`.

#![allow(non_snake_case)]

/// Deterministic data + key initialiser shared by both hash tests.
/// C: `hash_test_init` in `picoquictest/hashtest.c`.
fn hash_test_init(test: &mut [u8], k: &mut [u8; 16]) {
    let len = test.len();
    for (i, b) in test.iter_mut().enumerate() {
        *b = (i.wrapping_add(i >> 8)) as u8;
    }
    let k_len = k.len();
    for (i, b) in k.iter_mut().enumerate() {
        *b = (k_len - i) as u8;
    }
    let _ = len;
}

/// C: `picohash_test` in `picoquictest/hashtest.c`.
///
/// Drives the hash-table API ([`crate::hash::HashTable`]) through
/// inserts, lookups, collision-handling, and deletions.  The C body
/// used a custom hash function and intrusive items; the Rust API
/// parameterises on `Hash + Eq` via `u64` keys.
#[test]
fn picohash() {
    use crate::hash::HashTable;

    let mut t: HashTable<u64, ()> = HashTable::new(32).expect("create hash table");

    assert_eq!(t.len(), 0);

    // Insert odd values 1, 3, 5, 7, 9.
    for i in (1u64..10).step_by(2) {
        assert!(t.insert(i, ()).is_ok(), "insert({i}) failed");
    }
    assert_eq!(t.len(), 5);

    // Every inserted value is retrievable.
    for i in (1u64..10).step_by(2) {
        assert!(t.lookup(&i).is_some(), "lookup({i}) failed");
    }

    // Create collisions: for k in {1, 5}, insert k + 32*j for j in 1..=k.
    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.insert(key, ()).is_ok(), "insert({key}) failed");
        }
    }
    // Original 5 + 1 + 5 = 11.
    assert_eq!(t.len(), 11);

    // Collision entries are retrievable.
    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.lookup(&key).is_some(), "lookup({key}) failed");
        }
    }

    // Even values 0, 2, 4, 6, 8, 10 were never inserted.
    for i in (0u64..=10).step_by(2) {
        assert!(t.lookup(&i).is_none(), "lookup({i}) returned invalid item");
    }

    // Delete values 1 and 9 (first and near-last of the originals).
    for i in (1u64..10).step_by(4) {
        let tok = t.lookup(&i).expect("pre-delete lookup");
        t.remove(tok);
    }
    assert_eq!(t.len(), 8);

    // Deleted values are gone.
    for i in (1u64..10).step_by(4) {
        assert!(t.lookup(&i).is_none(), "deleted value {i} still found");
    }
}

/// C: `picohash_embedded_test` in `picoquictest/hashtest.c`.
///
/// Same exercise as [`picohash`] but exercises the `with_seed`
/// constructor (the C `picohash_create_ex` path with a
/// `key_to_item` callback and explicit `hash_seed`).
#[test]
fn picohash_embedded() {
    use crate::hash::HashTable;

    let hash_seed: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut t: HashTable<u64, ()> =
        HashTable::with_seed(32, &hash_seed).expect("create hash table with seed");

    assert_eq!(t.len(), 0);

    for i in (1u64..10).step_by(2) {
        assert!(t.insert(i, ()).is_ok(), "insert({i}) failed");
    }
    assert_eq!(t.len(), 5);

    for i in (1u64..10).step_by(2) {
        assert!(t.lookup(&i).is_some(), "lookup({i}) failed");
    }

    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.insert(key, ()).is_ok(), "insert({key}) failed");
        }
    }
    assert_eq!(t.len(), 11);

    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.lookup(&key).is_some(), "lookup({key}) failed");
        }
    }

    for i in (0u64..=10).step_by(2) {
        assert!(t.lookup(&i).is_none(), "lookup({i}) returned invalid item");
    }

    for i in (1u64..10).step_by(4) {
        let tok = t.lookup(&i).expect("pre-delete lookup");
        t.remove(tok);
    }
    assert_eq!(t.len(), 8);

    for i in (1u64..10).step_by(4) {
        assert!(t.lookup(&i).is_none(), "deleted value {i} still found");
    }
}

/// C: `picohash_bytes_test` in `picoquictest/hashtest.c`.
///
/// Known-answer test for `picohash_bytes` — picoquic's
/// non-cryptographic byte-mixing hash.  In the Rust port this is
/// [`crate::hash::hash_bytes`].
#[test]
fn picohash_bytes() {
    use crate::hash::hash_bytes;

    let mut test = [0u8; 1024];
    let mut k = [0u8; 16];
    hash_test_init(&mut test, &mut k);

    let lengths: [usize; 12] = [1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024];
    let expected: [u64; 12] = [
        0x0301_6721_e32d_7aa7,
        0x6420_8401_ad85_bed5,
        0x4458_7b02_0947_9519,
        0x14a4_8174_8ee6_d77e,
        0x9a44_370f_d1b8_c1ee,
        0x2708_1725_c416_4c1a,
        0x2f1f_325d_a756_df85,
        0x2aa4_fda7_96f9_ffff,
        0x8ded_0692_d703_8037,
        0x7893_f939_9f50_7284,
        0x47a0_65db_eea7_7343,
        0xb543_a5b3_c675_127d,
    ];

    for (i, &len) in lengths.iter().enumerate() {
        let h = hash_bytes(&test[..len], &k);
        assert_eq!(h, expected[i], "picohash_bytes[{i}] for len={len}");
    }
}

/// C: `siphash_test` in `picoquictest/hashtest.c`.
///
/// Known-answer test for [`crate::siphash::siphash`].  The C
/// reference values come from the upstream public-domain release.
#[test]
fn siphash() {
    use crate::siphash::siphash;

    let mut test = [0u8; 1024];
    let mut k = [0u8; 16];
    hash_test_init(&mut test, &mut k);

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
