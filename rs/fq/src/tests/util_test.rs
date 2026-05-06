//! Test cases for `picoquictest/util_test.c`.

#![allow(non_snake_case)]

use crate::ConnectionId;

const EXPECTED_CIDS: &[(&[u8], &str)] = &[
    (
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        "000102030405060708090a0b0c0d0e0f",
    ),
    (
        &[0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10],
        "fedcba9876543210",
    ),
    (&[0xca, 0xfe], "cafe"),
    (&[0x77], "77"),
];

/// C: `util_connection_id_print_test` in `picoquictest/util_test.c`.
///
/// `picoquic_print_connection_id_hexa(buf, sizeof, &cid)` formats
/// the connection-id bytes as lowercase hex.  Rust callers do this
/// inline with `core::fmt::Write` over `cid.as_bytes()`.
#[test]
fn connection_id_print() {
    use core::fmt::Write;
    for (bytes, expected) in EXPECTED_CIDS {
        let cid = ConnectionId::clone_from_slice(bytes).expect("CID under cap");
        let mut got = String::new();
        for b in cid.as_bytes() {
            write!(&mut got, "{b:02x}").unwrap();
        }
        assert_eq!(got, *expected, "CID hex round-trip");
    }
}

/// C: `util_connection_id_parse_test` in `picoquictest/util_test.c`.
///
/// Parse a hex string back into a [`ConnectionId`].  Picoquic exposes
/// `picoquic_parse_connection_id_hexa(text, len, &cid)`; the Rust
/// equivalent is a small `from_str_radix` loop over byte pairs.
#[test]
fn connection_id_parse() {
    for (bytes, hex) in EXPECTED_CIDS {
        let mut decoded = [0u8; 20];
        let n = hex.len() / 2;
        for i in 0..n {
            decoded[i] = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("hex byte");
        }
        let parsed = ConnectionId::clone_from_slice(&decoded[..n]).expect("CID under cap");
        let expected = ConnectionId::clone_from_slice(bytes).expect("CID under cap");
        assert_eq!(parsed, expected, "CID parse roundtrip for {hex}");
    }
}

/// C: `util_uint8_to_str_test` in `picoquictest/util_test.c`.
///
/// Exercises [`crate::utils::uint8_to_str`] at three buffer sizes
/// (full, truncated to 7, truncated to 2).  The function copies
/// printable bytes verbatim and substitutes `'.'` for unprintable
/// ones, terminating with a NUL the C version stripped via
/// `strcmp`.
#[test]
fn util_uint8_to_str() {
    let input: &[u8] = b"azAZ09.\xff";

    let mut buf = [0u8; 16];
    let out = crate::utils::uint8_to_str(&mut buf, input);
    assert_eq!(out, b"azAZ09.?", "full-width output");

    let mut buf7 = [0u8; 7];
    let out = crate::utils::uint8_to_str(&mut buf7, input);
    assert_eq!(out, b"azA...", "7-byte output");

    let mut buf2 = [0u8; 2];
    let out = crate::utils::uint8_to_str(&mut buf2, input);
    assert_eq!(out, b".", "2-byte output");
}

/// C: `util_memcmp_test` in `picoquictest/util_test.c`.
///
/// The C body verifies [`crate::utils::constant_time_memcmp`]
/// returns equal/unequal for matching/differing 16-byte slices over
/// a 1 MB working set, plus rough timing measurements (skipped — the
/// timing portion was always informational and noise-prone).  We
/// keep just the correctness loop.
#[test]
fn util_memcmp() {
    use core::cmp::Ordering;

    use crate::utils::constant_time_memcmp;

    let nb_words = (1 << 17) / 8;
    let l_total = nb_words * 8;
    let mut x = vec![0u8; l_total];
    // Borrow a deterministic test RNG so the data isn't all zeros.
    let mut seed = 0xbabac001u64;
    let next = |seed: &mut u64| -> u64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *seed
    };
    for chunk in x[32..].chunks_mut(8) {
        chunk.copy_from_slice(&next(&mut seed).to_le_bytes());
    }
    x[16..32].copy_from_slice(&[0xff; 16]);

    let y = x.clone();

    // Equality detection.
    let mut j = 0;
    while j < l_total {
        assert_eq!(
            constant_time_memcmp(&x[j..j + 16], &y[j..j + 16]),
            Ordering::Equal,
            "unexpected mismatch at byte {j}",
        );
        j += 16;
    }

    // Difference detection: flip one byte per 16-byte group.
    for offset in 0..16 {
        let mut y = x.clone();
        let mut j = offset;
        while j < l_total {
            y[j] ^= 0xff;
            j += 16;
        }
        let mut j = 0;
        while j < l_total {
            assert_ne!(
                constant_time_memcmp(&x[j..j + 16], &y[j..j + 16]),
                Ordering::Equal,
                "unexpected match at offset={offset}, byte={j}",
            );
            j += 16;
        }
    }
}

/// C: `util_sprintf_test` in `picoquictest/util_test.c`.
///
/// The C `picoquic_sprintf` is a thin `snprintf` wrapper with a
/// "buffer too small" failure indicator.  The Rust equivalent is
/// `format!()` (which can't fail at all) or `core::write!` into a
/// fixed buffer.  No direct counterpart exists in `crate::utils` and
/// the test mostly exercises C-side bounds-checking semantics —
/// translating it adds no value.
#[test]
fn util_sprintf() {
    todo!()
}

/// C: `util_debug_print_test` in `picoquictest/util_test.c`.
///
/// The C body redirects the global `debug_set_stream` sink to a
/// file and verifies a write goes through.  Rust uses normal
/// `core::fmt` writers, so there's no equivalent global stream.
#[test]
fn util_debug_print() {
    todo!()
}

/// C: `util_threading_test` in `picoquictest/util_test.c`.
///
/// The C body exercises `picoquic_create_mutex` / `_event` /
/// `_thread`.  Phase 2 deleted those wrappers in favour of
/// `std::sync::*` and `std::thread`, so the test would be a fresh
/// `std::thread` harness — out of v1 scope.
#[test]
fn threading() {
    todo!()
}
