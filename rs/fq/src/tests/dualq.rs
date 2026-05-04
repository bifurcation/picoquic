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
//! Phase 1 contract: signatures only — every function body is
//! `todo!()`.  Bodies and the empty test module land in later
//! phases.
//!
//! Translation policy notes for this module:
//!
//! * The C vtable struct `picoquictest_aqm_t` is already a Rust
//!   trait ([`TestAqm`](crate::tests::util::TestAqm)) in
//!   [`crate::utils`].  The C "embed `super` and cast pointer"
//!   inheritance pattern collapses to `impl TestAqm for Dualq`; the
//!   C `super` field is dropped.
//! * Both queue heads (`queue_first` / `queue_last` in
//!   [`DualqQueue`]) stay raw `*mut TestSimPacket`, matching the
//!   intrusive-linked-list shape used by the parent sim-link.  Phase
//!   3 will dereference inside `unsafe` blocks with `// SAFETY:`
//!   notes (or refactor to `VecDeque`).
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
use crate::tests::util::{TestAqm, TestSimLink, TestSimPacket};

// ---------------------------------------------------------------------------
// Tunables.

/// Simulation link-rate ceiling: 125,000,000 bytes/sec ≈ 1 Gbps.
/// C: `#define DUALQ_MAX_LINK_RATE 125000000`.
pub const DUALQ_MAX_LINK_RATE: u64 = 125_000_000;

// ---------------------------------------------------------------------------
// Per-queue state.

/// One classified queue (L4S or Classic) inside a [`Dualq`].
/// C: `dualq_queue_t`.
///
/// Pointer-shape choices:
///
/// * `queue_first` / `queue_last` stay raw pointers — same intrusive
///   list pattern as [`crate::tests::util::TestSimLink`].  The queue does
///   not own the node allocations on its own; the parent [`Dualq`]
///   reaches them through these raw heads and hands ownership back
///   to callers via [`Dualq::dequeue_one`].
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
    pub fn enqueue(&mut self, _packet: TestSimPacket) {
        todo!()
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
#[derive(Default)]
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
    pub update_next: u64,
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
    pub last_input_time: u64,
}

// ---------------------------------------------------------------------------
// Trait impl — replaces the C "embedded vtable + cast" inheritance.

impl TestAqm for Dualq {
    /// C: `dualq_submit`.  Queues the packet, updates
    /// `last_input_time`, and runs the dequeue/PI2 update pass.
    fn submit(&mut self, _link: &mut TestSimLink, _packet: TestSimPacket, _current_time: u64) {
        todo!()
    }

    /// C: `dualq_reset` — runs the dequeue/PI2 update pass at
    /// `current_time`.
    fn reset(&mut self, _link: &mut TestSimLink, _current_time: u64) {
        todo!()
    }

    /// C: `dualq_release` — drains both queues onto the link as
    /// dropped packets.  The C body also `free(self)`s and nulls
    /// `link->aqm_state`; in Rust the caller drops the
    /// `Option<Box<dyn TestAqm>>` slot to do the same.
    fn release(&mut self, _link: &mut TestSimLink) {
        todo!()
    }

    /// C: `dualq_has_pending` — true iff either queue has at least
    /// one packet ready to admit.
    fn has_pending(&mut self) -> bool {
        todo!()
    }

    /// C: `dualq_admit_pending` — runs the dequeue/PI2 update pass
    /// at `current_time`.
    fn admit_pending(&mut self, _link: &mut TestSimLink, _current_time: u64) {
        todo!()
    }
}

impl Dualq {
    /// Install a fresh DualQ AQM on `link`, sized for an L4S queue
    /// of at most `l4s_max` bytes.  C: `int
    /// dualq_configure(picoquictest_sim_link_t* link, uint64_t
    /// l4s_max)`.
    ///
    /// Returns [`Error::Memory`] on the C `ERROR_MEMORY`
    /// allocation-failure path.
    pub fn install(_link: &mut TestSimLink, _l4s_max: u64) -> Result<(), Error> {
        todo!()
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
    pub fn dequeue_one(&mut self, _current_time: u64) -> Option<(TestSimPacket, bool)> {
        todo!()
    }
}

#[cfg(test)]
mod test {}
