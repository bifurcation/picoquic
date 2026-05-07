//! Test cases for the simulator harness in `picoquic/sim_link.c`.

#![allow(non_snake_case)]

use crate::tests::util::{
    JitterMode, TestSimLink, TestSimPacket, test_gauss_random, test_poisson_random, test_random,
    test_uniform_random,
};
use crate::{Instant, MAX_PACKET_SIZE};

/// Consume one bit of the 64-bit rotating loss mask and return whether
/// this packet should be lost.  Returns `false` when `loss_mask` is
/// `None` (matching the C `NULL` case).
///
/// C: `picoquic/sim_link.c:picoquictest_sim_link_testloss`.
#[allow(dead_code)]
fn testloss(loss_mask: &mut Option<u64>) -> bool {
    if let Some(mask) = loss_mask.as_mut() {
        let loss_bit = *mask & 1;
        *mask = (*mask >> 1) | (loss_bit << 63);
        loss_bit != 0
    } else {
        false
    }
}

/// Determine whether the current packet should be dropped according to
/// the link's burst-loss model.
///
/// C: `picoquic/sim_link.c:picoquictest_sim_link_simloss`.
#[allow(dead_code)]
fn simloss(link: &mut TestSimLink, current_time: Instant) -> bool {
    if link.nb_loss_in_burst == 0 {
        return false;
    }
    let ct = current_time.ticks();
    if link.packets_sent > link.packets_sent_next_burst {
        let picosec_wait = link.nb_loss_in_burst * link.picosec_per_byte * 1536;
        link.packets_sent_next_burst = link.packets_sent + link.packets_between_losses;
        link.nb_losses_this_burst = link.nb_loss_in_burst - 1;
        link.end_of_burst_time = Instant::from_ticks(ct + picosec_wait / 1_000_000);
        true
    } else if link.nb_losses_this_burst > 0 {
        if ct > link.end_of_burst_time.ticks() {
            link.nb_losses_this_burst = 0;
            false
        } else {
            link.nb_losses_this_burst -= 1;
            true
        }
    } else {
        false
    }
}

/// Compute one Wi-Fi-style jitter sample for a simulator link.
///
/// C: `picoquic/sim_link.c:picoquictest_sim_link_wifi_jitter`.
#[allow(dead_code)]
pub fn picoquictest_sim_link_wifi_jitter(link: &mut TestSimLink) -> u64 {
    const EXP_MINUS_1_X40000000: u64 = 395_007_542;
    const PRIMARY_JITTER: u64 = 1000;
    let n1 = test_poisson_random(&mut link.jitter_seed, EXP_MINUS_1_X40000000);
    let mut jitter = n1 * PRIMARY_JITTER;
    if n1 > 0 {
        jitter -= test_uniform_random(&mut link.jitter_seed, PRIMARY_JITTER);
    }

    if link.jitter > 1000 {
        let mut r = test_random(&mut link.jitter_seed);
        r ^= r >> 30;
        r &= 0x3fff_ffff;
        r = r.wrapping_mul(84_000);
        if r < ((link.jitter - 1000) << 30) {
            const EXP_MINUS_12_X40000000: u64 = 6597;
            const SECONDARY_JITTER: u64 = 7500;
            let n2 = test_poisson_random(&mut link.jitter_seed, EXP_MINUS_12_X40000000);
            jitter += n2 * SECONDARY_JITTER;
            if n2 > 1 {
                jitter -= test_uniform_random(&mut link.jitter_seed, SECONDARY_JITTER);
            }
        }
    }
    jitter
}

/// Compute one jitter sample for a simulator link.
///
/// C: `picoquic/sim_link.c:picoquictest_sim_link_jitter`.
pub fn picoquictest_sim_link_jitter(link: &mut TestSimLink) -> u64 {
    if link.jitter_mode == JitterMode::Wifi {
        picoquictest_sim_link_wifi_jitter(link)
    } else {
        let mut x = test_gauss_random(&mut link.jitter_seed);
        if x < -3.0 {
            x = -3.0;
        }
        x /= 3.0;
        let jitter = link.jitter as i64 + (x * link.jitter as f64) as i64;
        jitter.max(0) as u64
    }
}

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

#[test]
fn sim_link_jitter_sample() {
    let mut link = TestSimLink::create(0.01, 10_000, None, 0, Instant::from_ticks(0))
        .expect("sim link allocation");
    link.jitter = 1000;
    let _ = picoquictest_sim_link_jitter(&mut link);
    link.jitter_mode = JitterMode::Wifi;
    let _ = picoquictest_sim_link_jitter(&mut link);
}
