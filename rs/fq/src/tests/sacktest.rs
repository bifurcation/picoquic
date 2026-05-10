//! Tests for `picoquictest/sacktest.c` — SACK list and ACK-frame logic.
//!
//! Tests covered:
//! * [`ack_sack`]    — C `sacktest`: basic SACK-list record / query.
//! * [`ack_send`]    — C `sendacktest`: ACK-frame formatting.
//! * [`ack_loop`]    — C `sendack_loop_test`: gap/delay-triggered ACK.
//! * [`ack_range`]   — C `ackrange_test`: standalone SACK-list range ops.
//! * [`ack_disorder`]— C `ack_disorder_test`: out-of-order arrival stats.
//! * [`ack_horizon`] — C `ack_horizon_test`: horizon-delay variant.

#![allow(non_snake_case)]

use crate::frames::FrameType;
use crate::internal::{SackList, picoquic_sack_list_first_range, varint_decode, varint_skip};
use crate::{ConnectionId, Duration, Instant, PacketContext, Quic, RESET_SECRET_SIZE};

use super::util;

// ---------------------------------------------------------------------------
// Shared test data

const TEST_PNS: &[u64] = &[
    3, 4, 0, 2, 7, 8, 11, 12, 13, 17, 19, 21, 18, 16, 20, 10, 5, 6, 9, 1, 14, 15,
];

struct ExpectedAck {
    highest_received: u64,
    last_range: u64,
    num_blocks: u64,
}

// expected_ack[] from C sacktest.c, in the same order as TEST_PNS.
const EXPECTED_ACKS: &[ExpectedAck] = &[
    ExpectedAck {
        highest_received: 3,
        last_range: 0,
        num_blocks: 0,
    },
    ExpectedAck {
        highest_received: 4,
        last_range: 1,
        num_blocks: 0,
    },
    ExpectedAck {
        highest_received: 4,
        last_range: 1,
        num_blocks: 1,
    },
    ExpectedAck {
        highest_received: 4,
        last_range: 2,
        num_blocks: 1,
    },
    ExpectedAck {
        highest_received: 7,
        last_range: 0,
        num_blocks: 2,
    },
    ExpectedAck {
        highest_received: 8,
        last_range: 1,
        num_blocks: 2,
    },
    ExpectedAck {
        highest_received: 11,
        last_range: 0,
        num_blocks: 3,
    },
    ExpectedAck {
        highest_received: 12,
        last_range: 1,
        num_blocks: 2,
    },
    ExpectedAck {
        highest_received: 13,
        last_range: 2,
        num_blocks: 1,
    },
    ExpectedAck {
        highest_received: 17,
        last_range: 0,
        num_blocks: 2,
    },
    ExpectedAck {
        highest_received: 19,
        last_range: 0,
        num_blocks: 2,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 0,
        num_blocks: 3,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 0,
        num_blocks: 2,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 0,
        num_blocks: 1,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 5,
        num_blocks: 0,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 5,
        num_blocks: 1,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 5,
        num_blocks: 2,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 5,
        num_blocks: 2,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 5,
        num_blocks: 1,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 5,
        num_blocks: 1,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 5,
        num_blocks: 1,
    },
    ExpectedAck {
        highest_received: 21,
        last_range: 21,
        num_blocks: 0,
    },
];

// ack_range[] table from C ackrange_test — (range_min, range_max) pairs.
const ACK_RANGES: &[(u64, u64)] = &[
    (1, 1000),
    (0, 0),
    (3001, 4000),
    (4001, 5000),
    (6001, 7000),
    (5001, 6000),
    (1501, 2500),
    (501, 1500),
    (501, 7500),
];

// ---------------------------------------------------------------------------
// Local helpers

/// C: `ack_range_mask` in sacktest.c.
fn ack_range_mask(mask: &mut u64, mut highest: u64, range: u64) {
    for _ in 0..range {
        *mask |= 1u64 << (highest & 63);
        highest = highest.wrapping_sub(1);
    }
}

/// Parse and validate an encoded ACK frame.  C: `basic_ack_parse` in
/// sacktest.c; translated to assert-on-failure rather than returning -1.
fn basic_ack_parse(
    bytes: &[u8],
    expected: &ExpectedAck,
    previous_mask: &mut u64,
    expected_mask: u64,
) {
    assert!(!bytes.is_empty(), "empty ACK frame buffer");
    assert_eq!(bytes[0], FrameType::Ack as u8, "wrong frame type byte");

    let bytes_max = bytes.len();
    let mut byte_index = 1usize;

    let mut largest = 0u64;
    let l_largest = varint_decode(&bytes[byte_index..], &mut largest);
    assert_ne!(l_largest, 0, "varint_decode largest failed");
    byte_index += l_largest;

    let l_delay = varint_skip(&bytes[byte_index..]);
    assert_ne!(l_delay, 0, "varint_skip ack_delay failed");
    byte_index += l_delay;

    let mut num_block = 0u64;
    let l_num_block = varint_decode(&bytes[byte_index..], &mut num_block);
    assert_ne!(l_num_block, 0, "varint_decode num_block failed");
    byte_index += l_num_block;

    let mut last_range = 0u64;
    let l_last_range = varint_decode(&bytes[byte_index..], &mut last_range);
    assert_ne!(l_last_range, 0, "varint_decode last_range failed");
    byte_index += l_last_range;

    assert!(
        last_range <= largest,
        "last_range {last_range} > largest {largest}"
    );
    let mut acked_mask = 0u64;
    ack_range_mask(&mut acked_mask, largest, last_range + 1);
    let mut gap_begin = largest.wrapping_sub(last_range).wrapping_sub(1);

    for _ in 0..num_block {
        let mut gap = 0u64;
        let l_gap = varint_decode(&bytes[byte_index..], &mut gap);
        assert_ne!(l_gap, 0, "varint_decode gap failed");
        byte_index += l_gap;
        gap += 1;

        let mut ack_range = 0u64;
        let l_range = varint_decode(&bytes[byte_index..], &mut ack_range);
        assert_ne!(l_range, 0, "varint_decode ack_range failed");
        byte_index += l_range;

        assert!(gap <= gap_begin, "gap {gap} > gap_begin {gap_begin}");
        gap_begin -= gap;

        assert!(
            gap_begin >= ack_range,
            "gap_begin {gap_begin} < ack_range {ack_range}"
        );
        ack_range += 1;
        ack_range_mask(&mut acked_mask, gap_begin, ack_range);
        gap_begin -= ack_range;
    }

    assert_eq!(byte_index, bytes_max, "ACK frame not fully consumed");
    assert_eq!(largest, expected.highest_received, "largest mismatch");
    assert_eq!(last_range, expected.last_range, "last_range mismatch");
    assert_eq!(num_block, expected.num_blocks, "num_block mismatch");

    acked_mask |= *previous_mask;
    assert_eq!(acked_mask, expected_mask, "acked_mask mismatch");
    *previous_mask = acked_mask;
}

/// Inner body of `ack_loop` for a single (ack_gap, ack_delay_us) pair.
/// C: `sendack_loop_test_one`.
fn ack_loop_one(ack_gap: u64, ack_delay_us: u64) {
    let pc = PacketContext::Application;
    let t0 = Instant::from_ticks(0);
    let mut quic = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("quic");
    let cnx = quic
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            None,
            t0,
            0,
            Some(util::TEST_SNI),
            Some("minimal"),
            true,
        )
        .expect("cnx");

    let l_cid = cnx.create_local_connection_id(0, None, t0).expect("l_cid");

    cnx.sending_ecn_ack = false;
    cnx.ack_delay_remote = Duration::from_ticks(ack_delay_us);
    cnx.ack_gap_remote = ack_gap;

    util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

    let mut largest_received_number = 0u64;
    let mut largest_ack_number = 0u64;
    let mut largest_time_sent = 0u64;
    let mut next_wake_time = Instant::from_ticks(u64::MAX);
    let mut bytes = [0u8; 256];

    for (i, &pn) in TEST_PNS.iter().enumerate() {
        let mut ack_sent = false;
        let mut out_of_order = false;
        let current_time_base = i as u64 * 1000;

        assert_eq!(
            cnx.record_pn_received(pc, Some(l_cid), pn, Instant::from_ticks(current_time_base),),
            0,
            "record_pn_received at step {i}"
        );

        if largest_received_number + 1 != pn {
            out_of_order = true;
        }
        cnx.set_ack_needed_on_path(
            Instant::from_ticks(current_time_base),
            pc,
            0,
            out_of_order as i32,
        );

        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        if largest_received_number < pn {
            largest_received_number = pn;
        }

        for k in 0..5u64 {
            let mut more_data = 0i32;
            let current_time = current_time_base + k * 10;

            let ack_written = if cnx.is_ack_needed(
                Instant::from_ticks(current_time),
                &mut next_wake_time,
                pc,
                0,
            ) {
                let written = util::format_ack_frame_written(
                    cnx,
                    &mut bytes,
                    &mut more_data,
                    Instant::from_ticks(current_time),
                    pc,
                    0,
                )
                .expect("format_ack_frame returned None unexpectedly");
                assert_eq!(more_data, 0, "more_data set at ({i},{k})");
                written
            } else {
                0
            };

            if ack_written == 0 {
                if !ack_sent {
                    assert!(
                        largest_ack_number + ack_gap >= largest_received_number,
                        "missing ack by number at ({i},{k})"
                    );
                    assert!(
                        largest_time_sent.wrapping_add(ack_delay_us) >= largest_time_sent,
                        "missing ack by time at ({i},{k})"
                    );
                }
            } else {
                assert!(!ack_sent, "duplicate ack at ({i},{k})");
                assert!(
                    !(largest_ack_number + ack_gap > largest_received_number
                        && largest_time_sent + ack_delay_us > current_time
                        && !out_of_order),
                    "ack sent before time or number at ({i},{k})"
                );
                ack_sent = true;
                largest_ack_number = largest_received_number;
                largest_time_sent = current_time;
            }
        }
    }
}

/// Inner body shared by `ack_disorder` and `ack_horizon`.
/// C: `ack_disorder_test_one`.
fn ack_disorder_one(log_name: &str, horizon_delay: i64, range_average_max: f64) {
    const NB_RANGES: usize = 1000;
    const NB_EVEN_RANGES: usize = NB_RANGES / 2;
    const NB_ODD_RANGES: usize = NB_RANGES - NB_EVEN_RANGES;
    const LOW_LATENCY: u64 = 11_111;
    const HIGH_LATENCY: u64 = 300_000;
    const ACK_INTERVAL: u64 = 100_000;
    const PACKET_INTERVAL: u64 = 1_000;

    struct AckItem {
        pn: u64,
        arrive_time: u64,
        ackk_time: u64,
        nb_ranges_arrive: usize,
        nb_ranges_ack: usize,
    }

    let mut ackk_list: Vec<AckItem> = Vec::with_capacity(NB_RANGES);
    let mut i_ackk: usize = 0;
    let mut i_even_arrive: usize = 0;
    let mut i_odd_arrive: usize = 0;
    let mut t_even_arrive: u64 = LOW_LATENCY;
    let mut t_odd_arrive: u64 = PACKET_INTERVAL + HIGH_LATENCY;

    let mut sack0 = SackList::new();
    sack0.horizon_delay = horizon_delay;

    while i_even_arrive < NB_EVEN_RANGES || i_odd_arrive < NB_ODD_RANGES || i_ackk < ackk_list.len()
    {
        let mut next_time = u64::MAX;
        let mut i_action: i32 = -1;

        if i_odd_arrive < NB_ODD_RANGES {
            i_action = 0;
            next_time = t_odd_arrive;
        }
        if i_even_arrive < NB_EVEN_RANGES && t_even_arrive < next_time {
            i_action = 1;
            next_time = t_even_arrive;
        }
        if i_ackk < ackk_list.len() && ackk_list[i_ackk].ackk_time < next_time {
            i_action = 2;
            next_time = ackk_list[i_ackk].ackk_time;
        }

        match i_action {
            0 => {
                let pn = 2 * i_odd_arrive as u64 + 1;
                i_odd_arrive += 1;
                t_odd_arrive += PACKET_INTERVAL;
                sack0
                    .update(pn, pn, Instant::from_ticks(next_time))
                    .expect("update sack (odd)");
                assert!(ackk_list.len() < NB_RANGES, "ackk_list overflow");
                let nb = sack0.size();
                ackk_list.push(AckItem {
                    pn,
                    arrive_time: next_time,
                    ackk_time: next_time + ACK_INTERVAL,
                    nb_ranges_arrive: nb,
                    nb_ranges_ack: 0,
                });
            }
            1 => {
                let pn = 2 * i_even_arrive as u64;
                i_even_arrive += 1;
                t_even_arrive += PACKET_INTERVAL;
                sack0
                    .update(pn, pn, Instant::from_ticks(next_time))
                    .expect("update sack (even)");
                assert!(ackk_list.len() < NB_RANGES, "ackk_list overflow");
                let nb = sack0.size();
                ackk_list.push(AckItem {
                    pn,
                    arrive_time: next_time,
                    ackk_time: next_time + ACK_INTERVAL,
                    nb_ranges_arrive: nb,
                    nb_ranges_ack: 0,
                });
            }
            2 => {
                let pn = ackk_list[i_ackk].pn;
                sack0.process_ack_of_ack_range(None, pn, pn);
                ackk_list[i_ackk].nb_ranges_ack = sack0.size();
                i_ackk += 1;
            }
            _ => unreachable!("unexpected i_action {i_action}"),
        }
    }

    // Write CSV log (mirrors C picoquic_file_open / fprintf).
    {
        use std::io::Write as _;
        let mut f = std::fs::File::create(log_name).expect("create log file");
        writeln!(f, "pn,arrive_time,ackk_time,nb_ranges_arrive,nb_ranges_ack").unwrap();
        for item in &ackk_list {
            writeln!(
                f,
                "{}, {}, {}, {}, {}",
                item.pn,
                item.arrive_time,
                item.ackk_time,
                item.nb_ranges_arrive,
                item.nb_ranges_ack,
            )
            .unwrap();
        }
    }

    let sum_ranges_ack: usize = ackk_list.iter().map(|it| it.nb_ranges_ack).sum();
    let range_average = sum_ranges_ack as f64 / ackk_list.len() as f64;
    assert!(
        range_average <= range_average_max,
        "got {range_average} ACK ranges, larger than expected {range_average_max}"
    );

    sack0.free();
}

// ---------------------------------------------------------------------------
// Tests

/// C: `sacktest` in `picoquictest/sacktest.c`.
///
/// Records packet numbers from [`TEST_PNS`] into a minimal connection's
/// SACK list, verifying after each step that `is_pn_already_received`
/// and `record_pn_received` are consistent.
#[test]
fn ack_sack() {
    let pc = PacketContext::Application;
    let t0 = Instant::from_ticks(0);
    let mut quic = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("quic");

    // Phase 1: basic pn=0 case (mirrors C before the reset).
    {
        let cnx = quic
            .create_connection(
                ConnectionId::default(),
                ConnectionId::default(),
                None,
                t0,
                0,
                Some(util::TEST_SNI),
                Some("minimal"),
                true,
            )
            .expect("cnx");

        let l_cid = cnx.create_local_connection_id(0, None, t0).expect("l_cid");

        assert!(
            !cnx.is_pn_already_received(pc, Some(l_cid), 0),
            "pn 0 should not be received yet"
        );
        assert_eq!(
            cnx.record_pn_received(pc, Some(l_cid), 0, t0),
            0,
            "first record of pn 0 should return 0"
        );
        assert!(
            cnx.is_pn_already_received(pc, Some(l_cid), 0),
            "pn 0 should now be received"
        );
        // C: picoquic_sack_list_first == 0 (min PN), == Rust last()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.last(), 0);
        // C: picoquic_sack_list_last == 0 (max PN), == Rust first()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.first(), 0);
        // C: no second range after the first ascending SACK range.
        assert!(picoquic_sack_list_first_range(&cnx.ack_ctx[pc as usize].sack_list).is_none());
    }

    // Phase 2: fresh connection (mirrors picoquic_test_reset_minimal_cnx).
    {
        let cnx = quic
            .create_connection(
                ConnectionId::default(),
                ConnectionId::default(),
                None,
                t0,
                0,
                Some(util::TEST_SNI),
                Some("minimal"),
                true,
            )
            .expect("cnx after reset");

        let l_cid = cnx
            .create_local_connection_id(0, None, t0)
            .expect("l_cid after reset");

        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        let mut highest_seen = 0u64;
        let mut highest_seen_time = t0;

        for (i, &pn) in TEST_PNS.iter().enumerate() {
            let current_time = Instant::from_ticks(i as u64 * 100 + 1);

            if pn > highest_seen {
                highest_seen = pn;
                highest_seen_time = current_time;
            }

            assert_eq!(
                cnx.record_pn_received(pc, Some(l_cid), pn, current_time),
                0,
                "record pn {pn} at step {i}"
            );
            util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

            for &pn_j in &TEST_PNS[..=i] {
                assert!(
                    cnx.is_pn_already_received(pc, Some(l_cid), pn_j),
                    "pn {pn_j} should be received at step {i}"
                );
                assert_eq!(
                    cnx.record_pn_received(pc, Some(l_cid), pn_j, current_time),
                    1,
                    "duplicate record of pn {pn_j} should return 1 at step {i}"
                );
            }

            for &pn_j in &TEST_PNS[i + 1..] {
                assert!(
                    !cnx.is_pn_already_received(pc, Some(l_cid), pn_j),
                    "pn {pn_j} should not yet be received at step {i}"
                );
            }
        }

        // Final state: [0..21] all received, contiguous.
        // C: picoquic_sack_list_last == 21 (max) → Rust first()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.first(), 21);
        // C: picoquic_sack_list_first == 0 (min) → Rust last()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.last(), 0);
        assert_eq!(
            cnx.ack_ctx[pc as usize].time_stamp_largest_received,
            highest_seen_time
        );
        // C: no second range after the first ascending SACK range.
        assert!(picoquic_sack_list_first_range(&cnx.ack_ctx[pc as usize].sack_list).is_none());
    }
}

/// C: `sendacktest` in `picoquictest/sacktest.c`.
///
/// Records [`TEST_PNS`] in order, formats an ACK frame after each, and
/// validates the wire encoding against [`EXPECTED_ACKS`].
#[test]
fn ack_send() {
    let pc = PacketContext::Application;
    let t0 = Instant::from_ticks(0);
    let mut quic = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("quic");
    let cnx = quic
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            None,
            t0,
            0,
            Some(util::TEST_SNI),
            Some("minimal"),
            true,
        )
        .expect("cnx");

    cnx.sending_ecn_ack = false;
    let l_cid = cnx.create_local_connection_id(0, None, t0).expect("l_cid");

    util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

    let mut received_mask = 0u64;
    let mut previous_mask = 0u64;
    let mut bytes = [0u8; 256];

    for (i, &pn) in TEST_PNS.iter().enumerate() {
        let current_time = Instant::from_ticks(i as u64 * 100);

        assert_eq!(
            cnx.record_pn_received(pc, Some(l_cid), pn, current_time),
            0,
            "record pn {pn} at step {i}"
        );
        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        let mut more_data = 0i32;
        let written = util::format_ack_frame_written(cnx, &mut bytes, &mut more_data, t0, pc, 0)
            .expect("format_ack_frame at step {i}");

        received_mask |= 1u64 << (pn & 63);
        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        basic_ack_parse(
            &bytes[..written],
            &EXPECTED_ACKS[i],
            &mut previous_mask,
            received_mask,
        );
    }
}

/// C: `sendack_loop_test` in `picoquictest/sacktest.c`.
///
/// Runs the gap/delay-triggered ACK loop for three (ack_gap, ack_delay)
/// combinations to verify `is_ack_needed` / `format_ack_frame` fire at
/// the right moments.
#[test]
fn ack_loop() {
    let combinations: [(u64, u64); 3] = [(0, 0), (2, 1000), (10000, 25)];
    for (ack_gap, ack_delay) in combinations {
        ack_loop_one(ack_gap, ack_delay);
    }
}

/// C: `ackrange_test` in `picoquictest/sacktest.c`.
///
/// Inserts ranges from [`ACK_RANGES`] into a standalone SACK list,
/// checking for correct idempotency and final contiguous coverage.
#[test]
fn ack_range() {
    let t0 = Instant::from_ticks(0);
    let mut sack0 = SackList::new();

    for (i, &(rmin, rmax)) in ACK_RANGES.iter().enumerate() {
        // C: picoquic_check_sack_list == 0 means not yet in sack.
        assert!(
            !sack0.check(rmin, rmax),
            "range ({rmin},{rmax}) already in sack at step {i}"
        );
        sack0.update(rmin, rmax, t0).expect("update sack");
        util::check_ack_ranges(&mut sack0);

        for &(jmin, jmax) in ACK_RANGES.iter().take(i) {
            assert!(
                sack0.check(jmin, jmax),
                "range ({jmin},{jmax}) not in sack at step {i}"
            );
        }
    }

    // Final state: [0..7500] fully covered (no gaps).
    // C: picoquic_sack_list_first == 0 (min) → Rust last()
    assert_eq!(sack0.last(), 0);
    // C: picoquic_sack_list_last == 7500 (max) → Rust first()
    assert_eq!(sack0.first(), 7500);
    // C: no second range after the first ascending SACK range.
    assert!(picoquic_sack_list_first_range(&sack0).is_none());

    sack0.free();
}

/// C: `ack_disorder_test` in `picoquictest/sacktest.c`.
///
/// Simulates out-of-order arrival on two paths and verifies the average
/// number of ACK ranges stays below 133.
#[test]
fn ack_disorder() {
    ack_disorder_one("ack_disorder_test.csv", 0, 133.0);
}

/// C: `ack_horizon_test` in `picoquictest/sacktest.c`.
///
/// Same as `ack_disorder` but with a horizon delay of 1 s; the average
/// range count budget rises to 196.
#[test]
fn ack_horizon() {
    ack_disorder_one("ack_horizon_test.csv", 1_000_000, 196.0);
}
