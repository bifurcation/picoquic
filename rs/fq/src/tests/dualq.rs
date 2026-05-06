//! Translation of `picoquic/picoquictest_dualq.h` (with bodies in
//! `picoquictest/dualq_aqm.c`).
//!
//! DualQ-Coupled AQM used by the test suite (RFC 9332 timestamp
//! variant).  Sits behind a [`TestSimLink`]'s `aqm_state` slot and
//! classifies arriving packets into an L4S queue ([`Dualq::lq`]) or
//! a Classic queue ([`Dualq::cq`]); marks/drops on dequeue using the
//! PI2 controller; and feeds admitted packets back into the link's
//! transit queue.
//!
//! The implementation mirrors `picoquic/dualq_aqm.c`.
//!
//! Translation policy notes for this module:
//!
//! * The C vtable struct `picoquictest_aqm_t` is already a Rust
//!   trait ([`TestAqm`](crate::tests::util::TestAqm)) in
//!   [`crate::utils`].  The C "embed `super` and cast pointer"
//!   inheritance pattern collapses to `impl TestAqm for Dualq`; the
//!   C `super` field is dropped.
//! * The C dualq pair of `queue_first` / `queue_last` raw
//!   `*mut TestSimPacket` heads (the intrusive-linked-list shape
//!   used by the parent sim-link) is replaced by an owning
//!   `VecDeque<TestSimPacket>` per queue.  Submit pushes;
//!   dequeue pops.  No raw pointers, no `unsafe`.
//! * The C `dualq_dequeue_one`'s `int* should_drop` out-parameter
//!   folds into a tuple return — `Option<(Box<...>, bool)>`
//!   represents "no packet ready" / "(packet, drop?)".
//! * The C `dualq_release` self-frees with `free(self)` and clears
//!   `link->aqm_state`.  In Rust the trait method takes
//!   `&mut self`; the actual deallocation rides on the
//!   `Option<Box<dyn TestAqm>>` slot in the link being reset to
//!   `None` by the caller after `release` drains the queues —
//!   `release` itself only handles the queue drain.

use crate::Error;
use crate::Instant;
use crate::tests::util::{TestAqm, TestSimLink, TestSimPacket};

// ---------------------------------------------------------------------------
// Tunables.

/// Simulation link-rate ceiling: 125,000,000 bytes/sec ≈ 1 Gbps.
/// C: `#define DUALQ_MAX_LINK_RATE 125000000`.
pub const DUALQ_MAX_LINK_RATE: u64 = 125_000_000;
const PICOQUIC_ECN_ECT_1: u8 = 0x01;
const PICOQUIC_ECN_CE: u8 = 0x03;

// ---------------------------------------------------------------------------
// Per-queue state.

/// One classified queue (L4S or Classic) inside a [`Dualq`].
/// C: `dualq_queue_t`.
///
/// Storage shape:
///
/// * The C `queue_first` / `queue_last` pair (intrusive linked
///   list, same pattern as [`crate::tests::util::TestSimLink`])
///   is replaced by an owning `VecDeque<TestSimPacket>`.  Submit
///   pushes to the back; dequeue pops from the front and hands
///   ownership to the caller via [`Dualq::dequeue_one`].
/// * `count` mirrors the C `int`, kept as `i32` for ABI parity with
///   the dualq tests that inspect it directly.
#[derive(Default)]
pub struct DualqQueue {
    pub queue_bytes: u64,
    /// Running sum of drop probability — when it crosses 1.0 the
    /// `dualq_recur` helper "fires" and the head packet is
    /// dropped/marked.  C: `double sum_p`.
    pub sum_p: f64,
    /// Packets in the queue.  Replaces the C `queue_first` /
    /// `queue_last` head/tail pair.  `count` is `packets.len()`
    /// (the C field is gone).
    pub packets: std::collections::VecDeque<TestSimPacket>,
}

impl DualqQueue {
    /// Append `packet` to the tail of this queue.
    /// C: `dualq_enqueue_queue(dualq_queue_t* xq,
    /// picoquictest_sim_packet_t* packet)`.
    pub fn enqueue(&mut self, packet: TestSimPacket) {
        self.queue_bytes += packet.length as u64;
        self.packets.push_back(packet);
    }

    fn dequeue(&mut self) -> Option<TestSimPacket> {
        let packet = self.packets.pop_front()?;
        if self.packets.is_empty() {
            self.queue_bytes = 0;
        } else if (packet.length as u64) < self.queue_bytes {
            self.queue_bytes -= packet.length as u64;
        } else {
            self.queue_bytes = 1;
        }
        Some(packet)
    }
}

// ---------------------------------------------------------------------------
// AQM state.

/// DualQ Coupled AQM state attached to a sim link.  C:
/// `dualq_state_t`.
///
/// The C struct embedded a `picoquictest_aqm_t` vtable in its first
/// field (`super`); in Rust the equivalent is `impl TestAqm for
/// Dualq`, so the explicit `super` field is dropped.  All other
/// fields mirror the C layout one-for-one.  Field names follow Rust
/// conventions: the RFC 9332 symbols `p_Cmax`, `p'_L`, etc., become
/// `p_c_max`, `p_prime_l`, and so on.
pub struct Dualq {
    // -- Initialization parameters -----------------------------------
    /// PI2 queue-delay target for both L4S and Classic, in
    /// microseconds.
    pub target: u64,
    /// Coupling factor `k`.
    pub k: f64,
    /// Above this drop probability, the classic queue uses drops
    /// instead of marks.  C: `p_Cmax`.
    pub p_c_max: f64,
    /// Interval between PI2 parameter updates, in microseconds.
    /// C: `Tupdate`.
    pub t_update: u64,
    /// PI integral gain in Hz.
    pub pi2_alpha: f64,
    /// PI proportional gain in MHz (1 / microsecond).
    pub pi2_beta: f64,
    /// Above this queue-size threshold, the L queue behaves as
    /// classic.  C: `maxTh`.
    pub max_th: u64,
    /// Queue size above which the L queue starts CE marking.
    /// C: `minTh`.
    pub min_th: u64,
    /// `max_th - min_th`.
    pub range: u64,
    /// Above this drop probability, the L4S queue uses drops
    /// instead of marks.  C: `p_Lmax`.
    pub p_l_max: f64,
    /// Maximum size of L4S + Classic queues, in bytes.
    pub limit: u64,
    /// Counter used for weighted fair queuing — 15 ticks for L4S,
    /// 1 tick for Classic.  C: `int schedule_tick`.
    pub schedule_tick: i32,

    // -- Queues ------------------------------------------------------
    /// L4S queue of pending packets.
    pub lq: DualqQueue,
    /// Classic queue of pending packets.
    pub cq: DualqQueue,

    // -- PI2 controller scratch space --------------------------------
    /// Current length of the classic queue, in microseconds.
    pub curq: i64,
    /// Previous length of the classic queue, in microseconds.
    pub prevq: i64,
    /// Time at which the PI2 parameters should next be updated.
    pub update_next: Instant,
    pub lq_average_queue: u64,
    /// `p'` coefficient (RFC 9332): nominal mark rate of the L4S
    /// queue derived from the classic queue length.  C: `pprime`.
    pub p_prime: f64,
    /// `p'_L`: mark rate of the L4S queue computed from L4S queue
    /// length, before coupling.  C: `pprime_L`.
    pub p_prime_l: f64,
    /// Actual mark rate of the L4S queue, after combining with
    /// `p_cl`.  C: `p_L`.
    pub p_l: f64,
    /// Coupled L4S probability: `p_prime_l * k`.  C: `p_CL`.
    pub p_cl: f64,
    /// Nominal drop rate of the classic queue (`p_prime_l^2`).
    /// C: `p_C`.
    pub p_c: f64,

    // -- Picoquic NS data --------------------------------------------
    /// Time of the last `submit` call (microseconds).
    pub last_input_time: Instant,
}

impl Default for Dualq {
    fn default() -> Self {
        Self {
            target: 0,
            k: 0.0,
            p_c_max: 0.0,
            t_update: 0,
            pi2_alpha: 0.0,
            pi2_beta: 0.0,
            max_th: 0,
            min_th: 0,
            range: 0,
            p_l_max: 0.0,
            limit: 0,
            schedule_tick: 0,
            lq: DualqQueue::default(),
            cq: DualqQueue::default(),
            curq: 0,
            prevq: 0,
            update_next: Instant::from_ticks(0),
            lq_average_queue: 0,
            p_prime: 0.0,
            p_prime_l: 0.0,
            p_l: 0.0,
            p_cl: 0.0,
            p_c: 0.0,
            last_input_time: Instant::from_ticks(0),
        }
    }
}

// ---------------------------------------------------------------------------
// Trait impl — replaces the C "embedded vtable + cast" inheritance.

impl TestAqm for Dualq {
    /// C: `dualq_submit`.  Queues the packet, updates
    /// `last_input_time`, and runs the dequeue/PI2 update pass.
    fn submit(&mut self, link: &mut TestSimLink, packet: TestSimPacket, current_time: Instant) {
        self.enqueue(link, packet, current_time);
        self.last_input_time = current_time;
        self.update_it(link, current_time);
    }

    /// C: `dualq_reset` — runs the dequeue/PI2 update pass at
    /// `current_time`.
    fn reset(&mut self, link: &mut TestSimLink, current_time: Instant) {
        self.update_it(link, current_time);
    }

    /// C: `dualq_release` — drains both queues onto the link as
    /// dropped packets.  The C body also `free(self)`s and nulls
    /// `link->aqm_state`; in Rust the caller drops the
    /// `Option<Box<dyn TestAqm>>` slot to do the same.
    fn release(&mut self, link: &mut TestSimLink) {
        while let Some(packet) = self.lq.dequeue() {
            link.enqueue(packet, Instant::from_ticks(0), true);
        }
        while let Some(packet) = self.cq.dequeue() {
            link.enqueue(packet, Instant::from_ticks(0), true);
        }
    }

    /// C: `dualq_has_pending` — true iff either queue has at least
    /// one packet ready to admit.
    fn has_pending(&mut self) -> bool {
        !self.lq.packets.is_empty() || !self.cq.packets.is_empty()
    }

    /// C: `dualq_admit_pending` — runs the dequeue/PI2 update pass
    /// at `current_time`.
    fn admit_pending(&mut self, link: &mut TestSimLink, current_time: Instant) {
        self.update_it(link, current_time);
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

impl Dualq {
    fn total_queue_bytes(&self) -> u64 {
        self.cq.queue_bytes + self.lq.queue_bytes
    }

    fn enqueue(
        &mut self,
        link: &mut TestSimLink,
        mut packet: TestSimPacket,
        current_time: Instant,
    ) {
        if self.total_queue_bytes() + packet.length as u64 > self.limit {
            link.enqueue(packet, Instant::from_ticks(0), true);
        } else {
            packet.arrival_time = current_time;
            if packet.ecn_mark == PICOQUIC_ECN_ECT_1 || packet.ecn_mark == PICOQUIC_ECN_CE {
                self.lq.enqueue(packet);
            } else {
                self.cq.enqueue(packet);
            }
        }
    }

    fn recur(queue: &mut DualqQueue, likelihood: f64) -> bool {
        queue.sum_p += likelihood;
        if queue.sum_p > 1.0 {
            queue.sum_p -= 1.0;
            true
        } else {
            false
        }
    }

    fn scheduler(&mut self) -> Option<(TestSimPacket, bool)> {
        let mut is_lq = (self.schedule_tick & 0x0f) != 0;
        let mut packet = if is_lq {
            self.lq.dequeue()
        } else {
            self.cq.dequeue()
        };
        if packet.is_none() {
            is_lq = !is_lq;
            packet = if is_lq {
                self.lq.dequeue()
            } else {
                self.cq.dequeue()
            };
        }

        self.schedule_tick = (self.schedule_tick + 1) & 0x0f;
        packet.map(|packet| (packet, is_lq))
    }

    fn laqm(&self, current_time: Instant) -> f64 {
        let mut p_prime = 0.0;
        let mut lq_time = 0;
        if self.lq.packets.len() > 1
            && let Some(packet) = self.lq.packets.front()
            && packet.arrival_time.ticks() < current_time.ticks()
        {
            lq_time = current_time.ticks() - packet.arrival_time.ticks();
        }
        if lq_time >= self.max_th {
            p_prime = 1.0;
        } else if lq_time > self.min_th {
            p_prime = (lq_time - self.min_th) as f64 / self.range as f64;
        }
        p_prime
    }

    fn pi2_update(&mut self, current_time: Instant) {
        let current_ticks = current_time.ticks();
        let cq_time = self
            .cq
            .packets
            .front()
            .filter(|packet| packet.arrival_time.ticks() < current_ticks)
            .map(|packet| current_ticks - packet.arrival_time.ticks())
            .unwrap_or(0);
        let lq_time = self
            .lq
            .packets
            .front()
            .filter(|packet| packet.arrival_time.ticks() < current_ticks)
            .map(|packet| current_ticks - packet.arrival_time.ticks())
            .unwrap_or(0);

        self.curq = cq_time.max(lq_time) as i64;
        let target_delta = self.curq - self.target as i64;
        let delta_q = self.curq - self.prevq;

        self.p_prime += self.pi2_alpha * target_delta as f64 + self.pi2_beta * delta_q as f64;
        self.p_prime = self.p_prime.clamp(0.0, 1.0);

        self.p_cl = self.p_prime * self.k;
        if self.p_cl > 1.0 {
            self.p_cl = 1.0;
        }
        self.p_c = self.p_prime * self.p_prime_l;
        self.prevq = self.curq;
    }

    fn update_it(&mut self, link: &mut TestSimLink, current_time: Instant) {
        while link.queue_time.ticks() <= current_time.ticks() {
            if let Some((packet, should_drop)) = self.dequeue_one(current_time) {
                link.enqueue(packet, current_time, should_drop);
            } else {
                break;
            }
        }

        if current_time.ticks() >= self.update_next.ticks() {
            self.pi2_update(current_time);
            self.update_next = Instant::from_ticks(current_time.ticks() + self.t_update);
        }
    }

    fn params_init(&mut self, l4s_max: u64, link: &TestSimLink) {
        self.limit = if link.queue_delay_max <= link.microsec_latency || link.picosec_per_byte == 0
        {
            (250u64 * DUALQ_MAX_LINK_RATE) / 1_000_000
        } else {
            let queue_delay = link.queue_delay_max - link.microsec_latency;
            (queue_delay * 1_000_000) / link.picosec_per_byte
        };
        self.k = 2.0;
        self.target = 15_000;
        let rtt_max = 100_000u64;
        self.p_c_max = 1.0 / (self.k * self.k);
        if self.p_c_max > 1.0 {
            self.p_c_max = 1.0;
        }
        self.t_update = rtt_max / 3;
        if self.t_update > self.target {
            self.t_update = self.target;
        }
        self.pi2_alpha = (0.1 * self.t_update as f64) / (rtt_max as f64 * rtt_max as f64);
        self.pi2_beta = 0.3 / rtt_max as f64;

        if l4s_max == 0 {
            self.max_th = 1200;
            self.min_th = 800;
        } else {
            self.max_th = l4s_max;
            self.min_th = if l4s_max > 1200 { 800 } else { l4s_max / 3 };
        }
        self.range = self.max_th - self.min_th;
        self.p_l_max = 1.0;
    }

    /// Install a fresh DualQ AQM on `link`, sized for an L4S queue
    /// of at most `l4s_max` bytes.  C: `int
    /// dualq_configure(picoquictest_sim_link_t* link, uint64_t
    /// l4s_max)`.
    ///
    /// Returns [`Error::Memory`] on the C `ERROR_MEMORY`
    /// allocation-failure path.
    pub fn install(link: &mut TestSimLink, l4s_max: u64) -> Result<(), Error> {
        if let Some(mut aqm) = link.aqm_state.take() {
            if let Some(dualq) = aqm.as_any_mut().downcast_mut::<Dualq>() {
                dualq.params_init(l4s_max, link);
                link.aqm_state = Some(aqm);
                return Ok(());
            }
            aqm.release(link);
        }

        let mut dualq = Dualq::default();
        dualq.params_init(l4s_max, link);
        link.aqm_state = Some(Box::new(dualq));
        Ok(())
    }

    /// Run the scheduler / mark / drop logic for one packet,
    /// returning the chosen packet (if any) along with the
    /// should-drop flag.  C: `picoquictest_sim_packet_t*
    /// dualq_dequeue_one(dualq_state_t* dualq, uint64_t
    /// current_time, int* should_drop)`.
    ///
    /// The C `should_drop` out-parameter folds into the tuple
    /// return.  `None` mirrors the C `NULL` "no packet ready" path;
    /// the C body always sets `*should_drop = 0` before returning
    /// `NULL`, so no flag survives that branch.
    ///
    /// Exposed publicly to mirror the C surface used by the
    /// `dualq_aqm_test.c` unit tests; Phase 2 translates those
    /// tests directly.
    pub fn dequeue_one(&mut self, current_time: Instant) -> Option<(TestSimPacket, bool)> {
        let (mut packet, is_lq) = self.scheduler()?;
        let mut should_drop = false;

        if is_lq {
            if self.p_cl < self.p_l_max {
                self.p_prime_l = self.laqm(current_time);
                self.p_l = self.p_prime_l.max(self.p_cl);
                if Self::recur(&mut self.lq, self.p_prime_l) {
                    packet.ecn_mark = PICOQUIC_ECN_CE;
                }
            } else if Self::recur(&mut self.lq, self.p_c) {
                should_drop = true;
            } else if Self::recur(&mut self.lq, self.p_cl) {
                packet.ecn_mark = PICOQUIC_ECN_CE;
            }
        } else if Self::recur(&mut self.cq, self.p_c) {
            if packet.ecn_mark == 0 || self.p_c >= self.p_c_max {
                should_drop = true;
            } else {
                packet.ecn_mark = PICOQUIC_ECN_CE;
            }
        }

        Some((packet, should_drop))
    }
}

#[cfg(test)]
mod test {}
