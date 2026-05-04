//! Test cases for `picoquictest/pn2pn64test.c`.

#![allow(non_snake_case)]

use crate::internal::get_packet_number64;

struct Case {
    highest: u64,
    mask: u64,
    pn: u32,
    expected: u64,
}

const CASES: &[Case] = &[
    Case {
        highest: 0x10000,
        mask: 0xFFFF_FFFF_FFFF_0000,
        pn: 0x8000,
        expected: 0x18000,
    },
    Case {
        highest: 0xFFFE,
        mask: 0xFFFF_FFFF_FFFF_0000,
        pn: 0x8000,
        expected: 0x8000,
    },
    Case {
        highest: 0xFFFF,
        mask: 0xFFFF_FFFF_FFFF_0000,
        pn: 0x8000,
        expected: 0x8000,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0xDEAD_BEF0,
        expected: 0xDEAD_BEF0,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0xDEAD_BEEF,
        expected: 0xDEAD_BEEF,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0xDEAD_BEEE,
        expected: 0xDEAD_BEEE,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0,
        expected: 0x1_0000_0000,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 1,
        expected: 0x1_0000_0001,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0x1000_0000,
        expected: 0x1_1000_0000,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0x5EAD_BEEE,
        expected: 0x1_5EAD_BEEE,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0x5EAD_BEF0,
        expected: 0x5EAD_BEF0,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0x5EAD_BEEF,
        expected: 0x1_5EAD_BEEF,
    },
    Case {
        highest: 0x5EAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0xDEAD_BEEF,
        expected: 0xDEAD_BEEF,
    },
    Case {
        highest: 0x1_5EAD_BEEF,
        mask: 0xFFFF_FFFF_0000_0000,
        pn: 0xDEAD_BEEF,
        expected: 0x1_DEAD_BEEF,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_0000,
        pn: 0xBEF0,
        expected: 0xDEAD_BEF0,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_0000,
        pn: 0xBEEF,
        expected: 0xDEAD_BEEF,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_0000,
        pn: 0xBEEE,
        expected: 0xDEAD_BEEE,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_0000,
        pn: 0x3EEE,
        expected: 0xDEAE_3EEE,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_0000,
        pn: 0x3EEF,
        expected: 0xDEAE_3EEF,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_0000,
        pn: 0x3EF0,
        expected: 0xDEAD_3EF0,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_FF00,
        pn: 0xF0,
        expected: 0xDEAD_BEF0,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_FF00,
        pn: 0xEF,
        expected: 0xDEAD_BEEF,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_FF00,
        pn: 0xEE,
        expected: 0xDEAD_BEEE,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_FFFF_FF00,
        pn: 0x7F,
        expected: 0xDEAD_BE7F,
    },
    Case {
        highest: 0xDEAD_BE71,
        mask: 0xFFFF_FFFF_FFFF_FF00,
        pn: 0xEF,
        expected: 0xDEAD_BEEF,
    },
    Case {
        highest: 0xDEAD_BE70,
        mask: 0xFFFF_FFFF_FFFF_FF00,
        pn: 0xEF,
        expected: 0xDEAD_BEEF,
    },
    Case {
        highest: 0xDEAD_BE6F,
        mask: 0xFFFF_FFFF_FFFF_FF00,
        pn: 0xEF,
        expected: 0xDEAD_BEEF,
    },
    Case {
        highest: 0xDEAD_BE6E,
        mask: 0xFFFF_FFFF_FFFF_FF00,
        pn: 0xEF,
        expected: 0xDEAD_BDEF,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_C000_0000,
        pn: 0x1EAD_BEF0,
        expected: 0xDEAD_BEF0,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_C000_0000,
        pn: 0x1EAD_BEEF,
        expected: 0xDEAD_BEEF,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_C000_0000,
        pn: 0x1EAD_BEEE,
        expected: 0xDEAD_BEEE,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_C000_0000,
        pn: 0,
        expected: 0xC000_0000,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_C000_0000,
        pn: 1,
        expected: 0xC000_0001,
    },
    Case {
        highest: 0xDEAD_BEEF,
        mask: 0xFFFF_FFFF_C000_0000,
        pn: 0x1000_0000,
        expected: 0xD000_0000,
    },
    Case {
        highest: 0x5EAD_BEEF,
        mask: 0xFFFF_FFFF_C000_0000,
        pn: 0x1EAD_BEEF,
        expected: 0x5EAD_BEEF,
    },
    Case {
        highest: 0x1_5EAD_BEEF,
        mask: 0xFFFF_FFFF_C000_0000,
        pn: 0x1EAD_BEEF,
        expected: 0x1_5EAD_BEEF,
    },
];

/// C: `pn2pn64test` in `picoquictest/pn2pn64test.c`.
///
/// Known-answer table for [`crate::internal::get_packet_number64`],
/// the truncated→full packet-number reconstruction routine
/// (RFC 9000 §A.3).
#[test]
fn pn2pn64() {
    for (i, case) in CASES.iter().enumerate() {
        let got = get_packet_number64(case.highest, case.mask, case.pn);
        assert_eq!(
            got, case.expected,
            "case {i}: highest={:#x} mask={:#x} pn={:#x}",
            case.highest, case.mask, case.pn,
        );
    }
}
