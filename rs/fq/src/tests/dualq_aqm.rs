//! Test cases for `picoquictest/dualq_aqm_test.c`.
//!
//! Unit tests for the DualQ Coupled AQM (RFC 9332) implementation used
//! in the network simulator.  The test suite is structured as five
//! sequential sub-tests:
//!
//! 1. `dualq_ctx_test`   — create / release the context.
//! 2. `dualq_enqueue`    — verify the per-queue enqueue API.
//! 3. `dualq_dequeue`    — verify the dequeue scheduler.
//! 4. `dualq_submit`     — verify submit routing and drop-on-overflow.
//! 5. `dualq_sustain`    — end-to-end throughput with mixed ECN traffic.

#![allow(non_snake_case)]

use crate::Instant;
use crate::tests::dualq::{Dualq, DualqQueue};
use crate::tests::util::{TestAqm, TestSimLink, TestSimPacket};

// ---------------------------------------------------------------------------
// Sub-test helpers.  Each mirrors one C helper function.

const ECN_ECT_0: u8 = 0x02;
const ECN_ECT_1: u8 = 0x01;
const ECN_CE: u8 = 0x03;
const ECN_SEQUENCE: [u8; 5] = [0, ECN_ECT_0, 0, ECN_ECT_1, ECN_CE];
const QUEUE_ID: [u8; 5] = [0, 0, 0, 1, 1];

struct DualqTestCtx {
    link: TestSimLink,
    simulated_time: Instant,
}

fn check(ok: bool) -> crate::Result<()> {
    if ok {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

fn dualq_test_get_ctx() -> crate::Result<DualqTestCtx> {
    let simulated_time = Instant::from_ticks(0);
    let mut link = TestSimLink::create(0.01, 25_000, None, 50_000, simulated_time)?;
    Dualq::install(&mut link, 5_000)?;
    Ok(DualqTestCtx {
        link,
        simulated_time,
    })
}

fn with_dualq<R>(
    link: &mut TestSimLink,
    f: impl FnOnce(&mut Dualq, &mut TestSimLink) -> crate::Result<R>,
) -> crate::Result<R> {
    let mut aqm = link.aqm_state.take().ok_or(crate::Error::Generic)?;
    let result = match aqm.as_any_mut().downcast_mut::<Dualq>() {
        Some(dualq) => f(dualq, link),
        None => Err(crate::Error::Generic),
    };
    link.aqm_state = Some(aqm);
    result
}

fn dualq_test_get_packet(ecn_mark: u8, length: usize) -> crate::Result<TestSimPacket> {
    let mut packet = TestSimPacket::create()?;
    packet.ecn_mark = ecn_mark;
    packet.length = length;
    Ok(packet)
}

fn queue_for_mut(dualq: &mut Dualq, queue_id: u8) -> &mut DualqQueue {
    if queue_id == 0 {
        &mut dualq.cq
    } else {
        &mut dualq.lq
    }
}

fn queue_bytes(dualq: &Dualq, queue_id: u8) -> u64 {
    if queue_id == 0 {
        dualq.cq.queue_bytes
    } else {
        dualq.lq.queue_bytes
    }
}

fn packet_key(packet: &TestSimPacket) -> (u64, usize, u8, u8) {
    (
        packet.arrival_time.ticks(),
        packet.length,
        packet.ecn_mark,
        packet.bytes[0],
    )
}

fn dualq_test_check_queue(link: &TestSimLink) -> crate::Result<()> {
    let mut previous_time = 0;
    for packet in &link.packets {
        let arrival_time = packet.arrival_time.ticks();
        check(arrival_time > previous_time)?;
        previous_time = arrival_time;
    }
    Ok(())
}

/// Verify that `test_set_minimal_cnx_with_time` + `TestSimLink::create` +
/// `Dualq::install` complete without error.
/// C: `dualq_test_ctx_test`.
fn dualq_ctx_test() -> crate::Result<()> {
    let mut ctx = dualq_test_get_ctx()?;
    with_dualq(&mut ctx.link, |_dualq, _link| Ok(()))
}

/// Enqueue one packet per ECN value into the matching queue and verify
/// that `queue_bytes`, head, and tail pointers update correctly.
/// C: `dualq_enqueue_test`.
fn dualq_enqueue() -> crate::Result<()> {
    let mut ctx = dualq_test_get_ctx()?;

    with_dualq(&mut ctx.link, |dualq, link| {
        for i in 0..ECN_SEQUENCE.len() {
            let packet = dualq_test_get_packet(ECN_SEQUENCE[i], 1000)?;
            let new_key = packet_key(&packet);
            let link_queue_len = link.packets.len();
            let xq = queue_for_mut(dualq, QUEUE_ID[i]);
            let old_bytes = xq.queue_bytes;
            let old_len = xq.packets.len();
            let old_front = xq.packets.front().map(packet_key);

            xq.enqueue(packet);

            check(xq.packets.len() == old_len + 1)?;
            check(xq.queue_bytes == old_bytes + 1000)?;
            check(xq.packets.back().map(packet_key) == Some(new_key))?;
            if old_len == 0 {
                check(xq.packets.front().map(packet_key) == Some(new_key))?;
            } else {
                check(xq.packets.front().map(packet_key) == old_front)?;
            }
            check(link.packets.len() == link_queue_len)?;
        }
        Ok(())
    })
}

/// Load five packets, dequeue until both queues are empty, and verify
/// all five packets are delivered within 100 ms simulated time.
/// C: `dualq_dequeue_test`.
fn dualq_dequeue() -> crate::Result<()> {
    let mut ctx = dualq_test_get_ctx()?;

    with_dualq(&mut ctx.link, |dualq, _link| {
        for i in 0..ECN_SEQUENCE.len() {
            let packet = dualq_test_get_packet(ECN_SEQUENCE[i], 1000)?;
            queue_for_mut(dualq, QUEUE_ID[i]).enqueue(packet);
        }

        let mut trials = 0;
        let mut received = 0;
        while dualq.lq.queue_bytes > 0 || dualq.cq.queue_bytes > 0 {
            trials += 1;
            check(trials <= 1000)?;
            if let Some((_packet, _should_drop)) = dualq.dequeue_one(ctx.simulated_time) {
                received += 1;
            } else {
                ctx.simulated_time = Instant::from_ticks(ctx.simulated_time.ticks() + 1000);
            }
        }

        check(received == 5)?;
        check(dualq.cq.queue_bytes == 0 && dualq.cq.packets.is_empty())?;
        check(dualq.lq.queue_bytes == 0 && dualq.lq.packets.is_empty())?;
        check(ctx.simulated_time.ticks() <= 100_000)
    })
}

/// Submit 50 packets and verify that: the first packet goes directly to
/// the link when the link queue is idle; subsequent packets go to the AQM
/// queues; and at least one packet is dropped when the combined queue
/// exceeds `dqs.limit`.
/// C: `dualq_submit_test`.
fn dualq_submit() -> crate::Result<()> {
    let mut ctx = dualq_test_get_ctx()?;
    let mut one_was_dropped = false;

    with_dualq(&mut ctx.link, |dualq, link| {
        for i in 0..50 {
            let i_queue = i % ECN_SEQUENCE.len();
            let packet = dualq_test_get_packet(ECN_SEQUENCE[i_queue], 1000)?;
            let old_bytes = queue_bytes(dualq, QUEUE_ID[i_queue]);
            let old_queue_time = link.queue_time;
            let old_total = dualq.cq.queue_bytes + dualq.lq.queue_bytes + packet.length as u64;

            dualq.submit(link, packet, ctx.simulated_time);

            if old_queue_time.ticks() <= ctx.simulated_time.ticks() {
                check(link.queue_time != old_queue_time)?;
            } else {
                check(link.queue_time == old_queue_time)?;
                if old_total > dualq.limit {
                    check(queue_bytes(dualq, QUEUE_ID[i_queue]) == old_bytes)?;
                    one_was_dropped = true;
                    break;
                } else {
                    check(queue_bytes(dualq, QUEUE_ID[i_queue]) != old_bytes)?;
                    dualq_test_check_queue(link)?;
                }
            }
        }
        Ok(())
    })?;

    check(one_was_dropped)
}

/// Drive 100 mixed-ECN packets through the full submit→dequeue pipeline,
/// interleaving arrivals, admissions, and submissions; verify all 100 are
/// accounted for (received + dropped) within 125 ms simulated time.
/// C: `dualq_sustain_test`.
fn dualq_sustain() -> crate::Result<()> {
    #[derive(Copy, Clone)]
    enum SustainAction {
        Arrival,
        Admission,
        Submit,
    }

    fn receive(ctx: &mut DualqTestCtx, nb_received: &mut i32) -> crate::Result<()> {
        check(ctx.link.dequeue(ctx.simulated_time).is_some())?;
        *nb_received += 1;
        dualq_test_check_queue(&ctx.link)
    }

    fn admit(ctx: &mut DualqTestCtx) {
        ctx.link.admit_pending(ctx.simulated_time);
    }

    fn submit(ctx: &mut DualqTestCtx, nb_sent: &mut i32) -> crate::Result<()> {
        let mut packet = dualq_test_get_packet(ECN_SEQUENCE[*nb_sent as usize % 5], 1000)?;
        packet.bytes[0] = *nb_sent as u8;
        *nb_sent += 1;
        ctx.link.submit(packet, ctx.simulated_time);
        dualq_test_check_queue(&ctx.link)
    }

    let mut ctx = dualq_test_get_ctx()?;
    let mut submit_time = ctx.simulated_time.ticks();
    let max_time = 125_000;
    let mut nb_received = 0;
    let mut nb_sent = 0;

    loop {
        let mut action_time = u64::MAX;
        let mut next_action = None;
        let arrival_time = ctx.link.next_arrival(Instant::from_ticks(action_time));
        if arrival_time < action_time {
            next_action = Some(SustainAction::Arrival);
            action_time = arrival_time;
        }

        let admission_time = ctx
            .link
            .next_admission(ctx.simulated_time, Instant::from_ticks(action_time));
        if admission_time < action_time {
            next_action = Some(SustainAction::Admission);
            action_time = admission_time;
        }

        let can_submit = with_dualq(&mut ctx.link, |dualq, _link| {
            Ok(dualq.cq.queue_bytes + dualq.lq.queue_bytes + 1000 <= dualq.limit)
        })?;
        if nb_sent < 100 && can_submit && submit_time < action_time {
            next_action = Some(SustainAction::Submit);
            action_time = submit_time;
        }

        let Some(next_action) = next_action else {
            break;
        };

        if ctx.simulated_time.ticks() < action_time {
            check(action_time <= ctx.simulated_time.ticks() + 4000)?;
            ctx.simulated_time = Instant::from_ticks(action_time);
        } else {
            action_time = ctx.simulated_time.ticks();
        }

        match next_action {
            SustainAction::Submit => {
                submit(&mut ctx, &mut nb_sent)?;
                if nb_sent % 7 == 0 {
                    submit_time += 4000;
                }
            }
            SustainAction::Arrival => receive(&mut ctx, &mut nb_received)?,
            SustainAction::Admission => admit(&mut ctx),
        }

        let _ = action_time;
    }

    check(nb_received as u64 + ctx.link.packets_dropped == 100)?;
    check(ctx.simulated_time.ticks() <= max_time)
}

// ---------------------------------------------------------------------------
// Test entry.

/// Run the full DualQ AQM unit-test suite.
/// C: `dualq_aqm_test` in `picoquictest/dualq_aqm_test.c`.
#[test]
fn dualq_aqm() {
    dualq_ctx_test().expect("dualq_ctx_test");
    dualq_enqueue().expect("dualq_enqueue");
    dualq_dequeue().expect("dualq_dequeue");
    dualq_submit().expect("dualq_submit");
    dualq_sustain().expect("dualq_sustain");
}
