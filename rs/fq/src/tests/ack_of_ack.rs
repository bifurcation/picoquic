//! Test cases for `picoquictest/ack_of_ack_test.c`.
//!
//! Exercises the ACK-of-ACK pruning logic: after the peer acknowledges an
//! ACK frame, ranges that were fully covered by that ACK are removed from
//! the local SACK list.  Three scenarios are tested — a simple tail trim,
//! a two-range prune, and a no-op case where the ACK only partially covers
//! a range.

#![allow(non_snake_case)]

use crate::Instant;
use crate::frames::FrameType;
use crate::internal::{SackList, varint_encode};

/// One ACK-of-ACK test scenario.
struct AoaCase {
    name: &'static str,
    /// Initial SACK ranges to load, ordered largest-first.
    initial: &'static [(u64, u64)],
    /// Ranges encoded into the ACK frame, ordered largest-first.
    ack: &'static [(u64, u64)],
    /// Expected SACK list after processing, ordered largest-first.
    result: &'static [(u64, u64)],
}

static CASES: &[AoaCase] = &[
    AoaCase {
        name: "simple",
        initial: &[(1, 9)],
        ack: &[(1, 8)],
        result: &[(9, 9)],
    },
    AoaCase {
        name: "two ranges",
        initial: &[(8, 9), (5, 6), (1, 3)],
        ack: &[(5, 6), (1, 3)],
        result: &[(8, 9)],
    },
    AoaCase {
        name: "no op",
        initial: &[(8, 9), (1, 6)],
        ack: &[(5, 6), (1, 3)],
        result: &[(8, 9), (1, 6)],
    },
];

/// Fill `sack_list` with (start, end) ranges.  C: `fill_test_sack_list`.
fn fill_sack_list(sack_list: &mut SackList, ranges: &[(u64, u64)]) {
    for &(start, end) in ranges {
        sack_list
            .insert_item(start, end, Instant::from_ticks(0))
            .expect("insert_item");
    }
}

/// Encode an ACK frame from `ranges` (largest-first) into `bytes` and
/// return the number of bytes written.  C: `build_test_ack`.
fn build_test_ack(ranges: &[(u64, u64)], bytes: &mut [u8]) -> usize {
    let mut idx = 0;
    bytes[idx] = FrameType::Ack as u8;
    idx += 1;
    // Largest acknowledged packet number.
    idx += varint_encode(&mut bytes[idx..], ranges[0].1);
    // ACK delay = 0 for these tests.
    idx += varint_encode(&mut bytes[idx..], 0);
    // Number of ACK blocks minus one.
    bytes[idx] = (ranges.len() - 1) as u8;
    idx += 1;
    // First ACK range.
    idx += varint_encode(&mut bytes[idx..], ranges[0].1 - ranges[0].0);
    // Subsequent gap + range pairs.
    for i in 1..ranges.len() {
        let gap = ranges[i - 1].0 - ranges[i].1 - 2;
        idx += varint_encode(&mut bytes[idx..], gap);
        idx += varint_encode(&mut bytes[idx..], ranges[i].1 - ranges[i].0);
    }
    idx
}

/// Assert that `sack_list` matches `expected` (ordered largest-first).
/// C: `cmp_test_sack_list`.
fn cmp_sack_list(sack_list: &mut SackList, expected: &[(u64, u64)]) {
    let mut nb_compared = 0;
    let mut tok = sack_list.last_item();
    for &(exp_start, exp_end) in expected {
        let t = tok.expect("sack item exists");
        let (start, end) = {
            let item = sack_list.sack_items.get(t).expect("valid sack token");
            (item.start_of_sack_range, item.end_of_sack_range)
        };
        assert_eq!(start, exp_start, "start mismatch at index {nb_compared}");
        assert_eq!(end, exp_end, "end mismatch at index {nb_compared}");
        nb_compared += 1;
        tok = sack_list.sack_previous_item(t);
    }
    assert!(
        tok.is_none() && nb_compared == expected.len(),
        "sack list length mismatch: expected {} ranges",
        expected.len()
    );
}

/// C: `ack_of_ack_test` in `picoquictest/ack_of_ack_test.c`.
#[test]
fn ack_of_ack() {
    for case in CASES {
        let mut sack_list = SackList::new();
        fill_sack_list(&mut sack_list, case.initial);

        let mut ack_buf = [0u8; 1024];
        let ack_length = build_test_ack(case.ack, &mut ack_buf);
        let mut consumed = 0usize;
        let ret = sack_list.process_ack_of_ack_frame(&mut ack_buf, ack_length, &mut consumed, 0);
        assert_eq!(
            ret, 0,
            "process_ack_of_ack_frame failed for case '{}'",
            case.name
        );

        cmp_sack_list(&mut sack_list, case.result);
    }
}
