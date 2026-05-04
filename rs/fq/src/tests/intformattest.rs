//! Test cases for `picoquictest/intformattest.c`.

#![allow(non_snake_case)]

use crate::internal::{
    format_16, format_24, format_32, format_64, parse_16, parse_24, parse_32, parse_64,
};

const TEST_NUMBERS: &[u64] = &[
    0,
    1,
    0xFFFF_FFFF_FFFF_FFFF,
    0xDEAD_BEEF,
    0x1234_5678_DEAD_BEEF,
];

/// Decode a big-endian integer of `length` bytes from `bytes`.
/// Mirror of the C `decode_number` helper used by the test.
fn decode_number(bytes: &[u8], length: usize) -> u64 {
    let mut n = 0u64;
    for &b in &bytes[..length] {
        n = (n << 8) | b as u64;
    }
    n
}

/// C: `intformattest` in `picoquictest/intformattest.c`.
///
/// Roundtrip every value in [`TEST_NUMBERS`] through each of the
/// 16 / 24 / 32 / 64-bit big-endian formatters and parsers.  The
/// C version also exercises a parallel `picoquic_frames_uint*_encode`
/// family that returns a tail pointer; that encoding is a Phase 4
/// addition and is omitted here.
#[test]
fn intformat() {
    let mut buf = [0u8; 8];

    for &n in TEST_NUMBERS {
        let n16 = n as u16;
        format_16(&mut buf, n16);
        assert_eq!(decode_number(&buf, 2), n16 as u64, "u16 BE bytes mismatch");
        assert_eq!(parse_16(&buf), n16, "parse_16 roundtrip");

        let n24 = (n & 0xFF_FFFF) as u32;
        format_24(&mut buf, n24);
        assert_eq!(decode_number(&buf, 3), n24 as u64, "u24 BE bytes mismatch");
        assert_eq!(parse_24(&buf), n24, "parse_24 roundtrip");

        let n32 = n as u32;
        format_32(&mut buf, n32);
        assert_eq!(decode_number(&buf, 4), n32 as u64, "u32 BE bytes mismatch");
        assert_eq!(parse_32(&buf), n32, "parse_32 roundtrip");

        format_64(&mut buf, n);
        assert_eq!(decode_number(&buf, 8), n, "u64 BE bytes mismatch");
        assert_eq!(parse_64(&buf), n, "parse_64 roundtrip");
    }
}

/// C: `varint_test` in `picoquictest/intformattest.c`.
///
/// Known-answer test for the QUIC varint encoder/decoder
/// ([`crate::internal::varint_encode`] / [`varint_decode`]).
/// Cases lifted verbatim from the C source (and from RFC 9000
/// Appendix A.1).
#[test]
fn varint() {
    use crate::internal::{varint_decode, varint_encode};

    struct Case {
        encoding: [u8; 8],
        length: usize,
        decoded: u64,
        is_canonical: bool,
    }

    let cases = [
        Case {
            encoding: [0, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 1,
            decoded: 0,
            is_canonical: true,
        },
        Case {
            encoding: [1, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 1,
            decoded: 1,
            is_canonical: true,
        },
        Case {
            encoding: [63, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 1,
            decoded: 63,
            is_canonical: true,
        },
        Case {
            encoding: [0x40, 64, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 2,
            decoded: 64,
            is_canonical: true,
        },
        Case {
            encoding: [0x7F, 0xFF, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 2,
            decoded: 0x3FFF,
            is_canonical: true,
        },
        Case {
            encoding: [0x80, 0, 0x40, 0, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 4,
            decoded: 0x4000,
            is_canonical: true,
        },
        Case {
            encoding: [0xBF, 0xFF, 0xFF, 0xFF, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 4,
            decoded: 0x3FFF_FFFF,
            is_canonical: true,
        },
        Case {
            encoding: [0xC0, 0, 0, 0, 0x40, 0, 0, 0],
            length: 8,
            decoded: 0x4000_0000,
            is_canonical: true,
        },
        Case {
            encoding: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            length: 8,
            decoded: 0x3FFF_FFFF_FFFF_FFFF,
            is_canonical: true,
        },
        Case {
            encoding: [0xc2, 0x19, 0x7c, 0x5e, 0xff, 0x14, 0xe8, 0x8c],
            length: 8,
            decoded: 151_288_809_941_952_652,
            is_canonical: true,
        },
        Case {
            encoding: [0x9d, 0x7f, 0x3e, 0x7d, 0xff, 0x14, 0xe8, 0x8c],
            length: 4,
            decoded: 494_878_333,
            is_canonical: true,
        },
        Case {
            encoding: [0xC0, 0, 0, 0, 0x1d, 0x7f, 0x3e, 0x7d],
            length: 8,
            decoded: 494_878_333,
            is_canonical: false,
        },
        Case {
            encoding: [0x7b, 0xbd, 0x3e, 0x7d, 0xff, 0x14, 0xe8, 0x8c],
            length: 2,
            decoded: 15_293,
            is_canonical: true,
        },
        Case {
            encoding: [0x80, 0, 0x3b, 0xbd, 0x3e, 0x7d, 0xff, 0x14],
            length: 4,
            decoded: 15_293,
            is_canonical: false,
        },
        Case {
            encoding: [0xC0, 0, 0, 0, 0, 0, 0x3b, 0xbd],
            length: 8,
            decoded: 15_293,
            is_canonical: false,
        },
        Case {
            encoding: [0x25, 0xbd, 0x3e, 0x7d, 0xff, 0x14, 0xe8, 0x8c],
            length: 1,
            decoded: 37,
            is_canonical: true,
        },
        Case {
            encoding: [0x40, 0x25, 0xbd, 0x3e, 0x7d, 0xff, 0x14, 0xe8],
            length: 2,
            decoded: 37,
            is_canonical: false,
        },
    ];

    let mut test_buf = [0xCCu8; 16];

    for (idx, case) in cases.iter().enumerate() {
        // Decode: walk every prefix length up to length+2 and check
        // that under-reads return 0 while a sufficient buffer
        // returns the canonical length.
        for buf_size in 0..=(case.length + 2).min(15) {
            test_buf[..case.length].copy_from_slice(&case.encoding[..case.length]);
            let mut n64 = 0u64;
            let length = varint_decode(&test_buf[..buf_size], &mut n64);
            let expected_length = if buf_size < case.length {
                0
            } else {
                case.length
            };
            assert_eq!(
                length, expected_length,
                "case {idx}, buf_size={buf_size}: wrong length"
            );
            if length != 0 {
                assert_eq!(
                    n64, case.decoded,
                    "case {idx}, buf_size={buf_size}: wrong value"
                );
            }
        }

        // Encode: only canonical encodings round-trip.
        if case.is_canonical {
            let mut encoding = [0u8; 8];
            let coded = varint_encode(&mut encoding, case.decoded);
            assert_eq!(coded, case.length, "case {idx}: wrong encoded length");
            assert_eq!(
                &encoding[..coded],
                &case.encoding[..coded],
                "case {idx}: wrong bytes"
            );
        }
    }
}

/// C: `sqrt_for_test_test` in `picoquictest/intformattest.c`.
///
/// The C body verifies `picoquic_sqrt_for_tests(x*(x+1))` returns
/// `x` for every `x` in a doubling sequence up to `0xffffffff`.
/// The Rust translation is a `todo!()` until a `sqrt_for_tests`
/// helper lands (it's a test-suite-only utility used by the BBR
/// tests).
#[test]
fn sqrt_for_test() {
    todo!("sqrt_for_test_test (no Rust counterpart yet)")
}
