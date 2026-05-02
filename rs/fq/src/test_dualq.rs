//! Translation of `quic/test_dualq.h` (and matching
//! body in `quic/dualq_aqm.c`).
//!
//! DualQ-Coupled AQM used by the test suite (RFC 9332 timestamp
//! variant).  Sits behind a [`test_sim_link_t`]'s `aqm_state`
//! slot and classifies arriving packets into an L4S queue (`lq`) or
//! a Classic queue (`cq`); marks/drops on dequeue using the PI2
//! controller; and feeds admitted packets back into the link's
//! transit queue.
//!
//! Phase 1 contract: signatures only — every function body is
//! `todo!()`.  Bodies and the empty test module land in later
//! phases.
//!
//! Translation policy notes for this module:
//!
//! * The C vtable struct `test_aqm_t` is already a Rust
//!   trait ([`testAqmT`](crate::utils::testAqmT))
//!   in [`crate::utils`].  The C "embed
//!   `super` and cast pointer" inheritance pattern collapses to
//!   `impl testAqmT for dualq_state_t`; the C `super`
//!   field is dropped.
//! * Both queue heads (`queue_first` / `queue_last` in
//!   [`dualq_queue_t`]) stay raw `*mut test_sim_packet_t`,
//!   matching the intrusive-linked-list shape used by the parent
//!   sim-link.  Phase 3 will dereference inside `unsafe` blocks
//!   with `// SAFETY:` notes (or refactor to `VecDeque`).
//! * `dualq_dequeue_one`'s C `int* should_drop` out-parameter folds
//!   into a tuple return — `Option<(Box<...>, bool)>` represents
//!   "no packet ready" / "(packet, drop?)".
//! * `dualq_configure` returns `Result<(), Error>` because v1 has no
//!   top-level [`crate::Error`] enum yet — TODO once it lands,
//!   replace `Err(())` with the corresponding `Error::Memory`
//!   variant (the C body returns `ERROR_MEMORY`).
//! * The C `dualq_release` self-frees with `free(self)` and clears
//!   `link->aqm_state`.  In Rust the trait method takes
//!   `&mut self`; the actual deallocation rides on the
//!   `Option<Box<dyn testAqmT>>` slot in the link being
//!   reset to `None` by the caller after `release` drains the
//!   queues — `release` itself only handles the queue drain.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// Status-code returns stand in for the missing top-level `Error`
// enum — see module docstring.

use crate::Error;
use crate::utils::{test_sim_link_t, test_sim_packet_t, testAqmT};

// ---------------------------------------------------------------------------
// Tunables.

/// Simulation link-rate ceiling: 125,000,000 bytes/sec ≈ 1 Gbps.
/// C: `#define DUALQ_MAX_LINK_RATE 125000000`.
pub const DUALQ_MAX_LINK_RATE: u64 = 125_000_000;

// ---------------------------------------------------------------------------
// Per-queue state.

/// One classified queue (L4S or Classic) inside a [`dualq_state_t`].
/// C: `dualq_queue_t`.
///
/// Pointer-shape choices:
///
/// * `queue_first` / `queue_last` stay raw pointers — same intrusive
///   list pattern as
///   [`crate::utils::test_sim_link_t`].
///   The queue does not own the node allocations on its own; the
///   parent [`dualq_state_t`] reaches them through these raw heads
///   and hands ownership back to callers via
///   [`dualq_dequeue_one`].
/// * `count` was a C `int` that only ever holds non-negative values;
///   widened to `i32` to match the underlying C ABI (the dualq
///   tests inspect it directly).
#[derive(Default)]
pub struct dualq_queue_t {
    pub queue_bytes: u64,
    /// Number of packets currently in the queue.
    pub count: i32,
    /// Running sum of drop probability — when it crosses 1.0 the
    /// `dualq_recur` helper "fires" and the head packet is
    /// dropped/marked.  C: `double sum_p`.
    pub sum_p: f64,
    pub queue_first: *mut test_sim_packet_t,
    pub queue_last: *mut test_sim_packet_t,
}

// ---------------------------------------------------------------------------
// AQM state.

/// DualQ Coupled AQM state attached to a sim link.  C:
/// `dualq_state_t`.
///
/// The C struct embedded a `test_aqm_t` vtable in its first
/// field (`super`); in Rust the equivalent is `impl
/// testAqmT for dualq_state_t`, so the explicit `super`
/// field is dropped.  All other fields mirror the C layout
/// one-for-one.
#[derive(Default)]
pub struct dualq_state_t {
    // -- Initialization parameters -----------------------------------
    /// PI2 queue-delay target for both L4S and Classic, in
    /// microseconds.
    pub target: u64,
    /// Coupling factor `k`.
    pub k: f64,
    /// Above this drop probability, classic queue uses drops
    /// instead of marks.
    pub p_Cmax: f64,
    /// Interval between PI2 parameter updates, in microseconds.
    pub Tupdate: u64,
    /// PI integral gain in Hz.
    pub pi2_alpha: f64,
    /// PI proportional gain in MHz (1 / microsecond).
    pub pi2_beta: f64,
    /// Above this queue-size threshold, the L queue behaves as
    /// classic.
    pub maxTh: u64,
    /// Queue size above which the L queue starts CE marking.
    pub minTh: u64,
    /// `maxTh - minTh`.
    pub range: u64,
    /// Above this drop probability, the L4S queue uses drops
    /// instead of marks.
    pub p_Lmax: f64,
    /// Maximum size of L4S + Classic queues, in bytes.
    pub limit: u64,
    /// Counter used for weighted fair queuing — 15 ticks for L4S,
    /// 1 tick for Classic.  C: `int schedule_tick`.
    pub schedule_tick: i32,

    // -- Queues ------------------------------------------------------
    /// L4S queue of pending packets.
    pub lq: dualq_queue_t,
    /// Classic queue of pending packets.
    pub cq: dualq_queue_t,

    // -- PI2 controller scratch space --------------------------------
    /// Current length of the classic queue, in microseconds.
    pub curq: i64,
    /// Previous length of the classic queue, in microseconds.
    pub prevq: i64,
    /// Time at which the PI2 parameters should next be updated.
    pub update_next: u64,
    pub lq_average_queue: u64,
    /// `p'` coefficient (RFC 9332): nominal mark rate of the L4S
    /// queue derived from the classic queue length.
    pub pprime: f64,
    /// `p'_L`: mark rate of the L4S queue computed from L4S queue
    /// length, before coupling.
    pub pprime_L: f64,
    /// Actual mark rate of the L4S queue, after combining with
    /// `p_CL`.
    pub p_L: f64,
    /// Coupled L4S probability: `pprime_L * k`.
    pub p_CL: f64,
    /// Nominal drop rate of the classic queue (`pprime_L^2`).
    pub p_C: f64,

    // -- quic NS data --------------------------------------------
    /// Time of the last `submit` call (microseconds).
    pub last_input_time: u64,
}

// ---------------------------------------------------------------------------
// Trait impl — replaces the C "embedded vtable + cast" inheritance.

impl testAqmT for dualq_state_t {
    /// C: `dualq_submit`.  Queues the packet, updates
    /// `last_input_time`, and runs the dequeue/PI2 update pass.
    fn submit(
        &mut self,
        _link: &mut test_sim_link_t,
        _packet: Box<test_sim_packet_t>,
        _current_time: u64,
    ) {
        todo!()
    }

    /// C: `dualq_reset` — runs the dequeue/PI2 update pass at
    /// `current_time`.
    fn reset(&mut self, _link: &mut test_sim_link_t, _current_time: u64) {
        todo!()
    }

    /// C: `dualq_release` — drains both queues onto the link as
    /// dropped packets.  The C body also `free(self)`s and nulls
    /// `link->aqm_state`; in Rust the caller drops the
    /// `Option<Box<dyn testAqmT>>` slot to do the same.
    fn release(&mut self, _link: &mut test_sim_link_t) {
        todo!()
    }

    /// C: `dualq_has_pending` — true iff either queue has at least
    /// one packet ready to admit.
    fn has_pending(&mut self) -> bool {
        todo!()
    }

    /// C: `dualq_admit_pending` — runs the dequeue/PI2 update pass
    /// at `current_time`.
    fn admit_pending(&mut self, _link: &mut test_sim_link_t, _current_time: u64) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Public configuration entry point.

/// Install (or reconfigure) DualQ on `link`.  C:
/// `int dualq_configure(test_sim_link_t* link, uint64_t l4s_max)`.
///
/// Returns `Err(())` for the C `ERROR_MEMORY` allocation
/// failure path.  TODO: once the crate-level [`crate::Error`] enum
/// lands, replace `Err(())` with `Error::Memory`.
pub fn dualq_configure(_link: &mut test_sim_link_t, _l4s_max: u64) -> Result<(), Error> {
    todo!()
}

// ---------------------------------------------------------------------------
// Internal helpers exposed for tests / monitoring.
//
// The header documents these as "for tests and monitoring".  Phase 2
// translates `test/dualq_aqm_test.c`; keeping the same
// public surface keeps that translation mechanical.

/// Append `packet` to the tail of `xq`'s intrusive list.  C:
/// `void dualq_enqueue_queue(dualq_queue_t* xq,
/// test_sim_packet_t* packet)`.
///
/// Pointer-shape choice: callers (`dualq_enqueue` and the
/// `dualq_enqueue_test` unit test) hand off ownership of the packet
/// to the queue, so the Rust signature takes `Box<...>`.  Phase 3
/// stores `Box::into_raw` into the intrusive `next_packet` chain
/// (matching the C raw-pointer storage).
pub fn dualq_enqueue_queue(_xq: &mut dualq_queue_t, _packet: Box<test_sim_packet_t>) {
    todo!()
}

/// Run the scheduler / mark / drop logic for one packet, returning
/// the chosen packet (if any) along with the should-drop flag.  C:
/// `test_sim_packet_t* dualq_dequeue_one(dualq_state_t*
/// dualq, uint64_t current_time, int* should_drop)`.
///
/// The C `should_drop` out-parameter folds into the tuple return.
/// `None` mirrors the C `NULL` "no packet ready" path; the C body
/// always sets `*should_drop = 0` before returning `NULL`, so no
/// flag survives that branch.
pub fn dualq_dequeue_one(
    _dualq: &mut dualq_state_t,
    _current_time: u64,
) -> Option<(Box<test_sim_packet_t>, bool)> {
    todo!()
}

#[cfg(test)]
mod test {}
