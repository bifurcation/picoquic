//! Test cases for the simulator harness in `picoquic/sim_link.c`.

#![allow(non_snake_case)]

use crate::tests::util::{TestSimLink, TestSimPacket};
use crate::{Instant, MAX_PACKET_SIZE};

/// C: `sim_link_one_test` in `picoquic/sim_link.c`.
fn sim_link_one_test(loss_mask: Option<u64>, queue_delay_max: u64, nb_losses: u64) {
    let mut departure_time = Instant::from_ticks(0);
    let mut link = TestSimLink::create(
        0.01,
        10_000,
        loss_mask,
        queue_delay_max,
        Instant::from_ticks(0),
    )
    .expect("sim link allocation");
    let mut dequeued = 0u64;
    let mut queued = 0u64;
    const NB_PACKETS: u64 = 16;

    loop {
        if queued >= NB_PACKETS {
            departure_time = Instant::from_ticks(u64::MAX);
        }

        let current_time = Instant::from_ticks(link.next_arrival(departure_time));

        if let Some(_packet) = link.dequeue(current_time) {
            dequeued += 1;
        } else if queued < NB_PACKETS {
            let mut packet = TestSimPacket::create().expect("sim packet allocation");
            packet.length = MAX_PACKET_SIZE;
            link.submit(packet, departure_time);
            departure_time = Instant::from_ticks(departure_time.ticks() + 250);
            queued += 1;
        } else {
            break;
        }
    }

    assert_eq!(
        dequeued + nb_losses,
        NB_PACKETS,
        "unexpected sim-link delivery count",
    );
}

/// C: `sim_link_test` in `picoquictest/<harness>.c`.
#[test]
fn sim_link() {
    sim_link_one_test(Some(0), 0, 0);
    sim_link_one_test(Some(8), 0, 1);
    sim_link_one_test(Some(0x18), 0, 2);
}
