//! Test cases for `picoquictest/intformattest.c`.

#![allow(non_snake_case)]

use crate::internal::{
    format_16, format_24, format_32, format_64, parse_16, parse_24, parse_32, parse_64,
};
use crate::utils::{
    frames_uint16_encode, frames_uint24_encode, frames_uint32_encode, frames_uint64_encode,
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
/// 16 / 24 / 32 / 64-bit big-endian formatters and parsers, including
/// the `picoquic_frames_uint*_encode` family that returns a tail pointer.
#[test]
fn intformat() {
    let mut buf = [0u8; 8];

    for new_encoding in 0..2 {
        for &n in TEST_NUMBERS {
            let n16 = n as u16;
            if new_encoding == 0 {
                format_16(&mut buf, n16);
            } else {
                let advance = {
                    let rest = frames_uint16_encode(&mut buf, n16).expect("u16 encode fits");
                    8 - rest.len()
                };
                assert_eq!(advance, 2, "u16 encoder advanced wrong length");
            }
            assert_eq!(decode_number(&buf, 2), n16 as u64, "u16 BE bytes mismatch");
            assert_eq!(parse_16(&buf), n16, "parse_16 roundtrip");
        }

        for &n in TEST_NUMBERS {
            let n24 = (n & 0xFF_FFFF) as u32;
            if new_encoding == 0 {
                format_24(&mut buf, n24);
            } else {
                let advance = {
                    let rest = frames_uint24_encode(&mut buf, n24).expect("u24 encode fits");
                    8 - rest.len()
                };
                assert_eq!(advance, 3, "u24 encoder advanced wrong length");
            }
            assert_eq!(decode_number(&buf, 3), n24 as u64, "u24 BE bytes mismatch");
            assert_eq!(parse_24(&buf), n24, "parse_24 roundtrip");
        }

        for &n in TEST_NUMBERS {
            let n32 = n as u32;
            if new_encoding == 0 {
                format_32(&mut buf, n32);
            } else {
                let advance = {
                    let rest = frames_uint32_encode(&mut buf, n32).expect("u32 encode fits");
                    8 - rest.len()
                };
                assert_eq!(advance, 4, "u32 encoder advanced wrong length");
            }
            assert_eq!(decode_number(&buf, 4), n32 as u64, "u32 BE bytes mismatch");
            assert_eq!(parse_32(&buf), n32, "parse_32 roundtrip");
        }

        for &n in TEST_NUMBERS {
            if new_encoding == 0 {
                format_64(&mut buf, n);
            } else {
                let advance = {
                    let rest = frames_uint64_encode(&mut buf, n).expect("u64 encode fits");
                    8 - rest.len()
                };
                assert_eq!(advance, 8, "u64 encoder advanced wrong length");
            }
            assert_eq!(decode_number(&buf, 8), n, "u64 BE bytes mismatch");
            assert_eq!(parse_64(&buf), n, "parse_64 roundtrip");
        }
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
    use crate::internal::{
        frames_varint_decode, frames_varint_encode_length, varint_decode, varint_encode,
    };
    use crate::utils::frames_varint_encode;

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
        for is_new_decode in [false, true] {
            for buf_size in 0..=(case.length + 2).min(15) {
                test_buf[..case.length].copy_from_slice(&case.encoding[..case.length]);
                let mut n64 = 0u64;
                let length = if is_new_decode {
                    frames_varint_decode(&test_buf[..buf_size], &mut n64)
                        .map_or(0, |rest| buf_size - rest.len())
                } else {
                    varint_decode(&test_buf[..buf_size], &mut n64)
                };
                let expected_length = if buf_size < case.length {
                    0
                } else {
                    case.length
                };
                assert_eq!(
                    length, expected_length,
                    "case {idx}, is_new_decode={is_new_decode}, buf_size={buf_size}: wrong length"
                );
                if length != 0 {
                    assert_eq!(
                        n64, case.decoded,
                        "case {idx}, is_new_decode={is_new_decode}, buf_size={buf_size}: wrong value"
                    );
                }
            }
        }

        if case.is_canonical {
            for is_new_encode in [false, true] {
                let mut encoding = [0u8; 8];
                let coded_length = if is_new_encode {
                    frames_varint_encode(&mut encoding, case.decoded)
                        .map_or(usize::MAX, |rest| 8 - rest.len())
                } else {
                    varint_encode(&mut encoding[..case.length], case.decoded)
                };
                assert_eq!(
                    coded_length, case.length,
                    "case {idx}, is_new_encode={is_new_encode}: wrong encoded length"
                );
                assert!(
                    coded_length <= encoding.len(),
                    "case {idx}, is_new_encode={is_new_encode}: encoded length exceeds buffer"
                );
                assert_eq!(
                    &encoding[..coded_length],
                    &case.encoding[..coded_length],
                    "case {idx}, is_new_encode={is_new_encode}: wrong bytes"
                );
            }
        }
    }

    for (idx, case) in cases.iter().enumerate() {
        if case.is_canonical {
            assert_eq!(
                frames_varint_encode_length(case.decoded),
                case.length,
                "case {idx}: wrong predicted frame varint length"
            );
        }
    }
}

/// Integer square root used by the picoquic test suite.
/// Defined in `picoquictest/intformattest.c` as a non-static helper;
/// reproduced here for use by the Rust port.
/// C: `picoquic_sqrt_for_tests`.
fn sqrt_for_tests(y: u64) -> u64 {
    if y < 6 {
        return [0u64, 1, 1, 1, 2, 2][y as usize];
    }
    let mut x_min = 2u64;
    let mut x_max = if y / 2 > 0xffff_ffff {
        0xffff_ffff
    } else {
        y / 2
    };
    let mut x = 0u64;
    for _ in 0..64 {
        x = (x_min + x_max) / 2;
        if x_min + 1 >= x_max {
            break;
        }
        let x2 = x * x;
        if x2 < y {
            x_min = x;
        } else if x2 > y {
            x_max = x;
        } else {
            break;
        }
    }
    x
}

/// C: `sqrt_for_test_test` in `picoquictest/intformattest.c`.
///
/// Verifies `sqrt_for_tests(x * (x+i))` returns `x` for `i ∈ {0,1}`
/// and every `x` in a doubling sequence up to `0xffffffff`.
#[test]
fn sqrt_for_test() {
    let mut x_base: u64 = 0;
    while x_base < 0xffff_ffff {
        for i in 0u64..2 {
            let y = x_base * (x_base + i);
            let x = sqrt_for_tests(y);
            assert_eq!(x, x_base, "sqrt_for_tests({x_base}*({x_base}+{i})) = {x}");
        }
        x_base = 2 * x_base + 1;
    }
}
