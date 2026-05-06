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

// ---------------------------------------------------------------------------
// Sub-test helpers.  Each mirrors one C helper function; all are `todo!()`
// until Phase 4 implements the simulation infrastructure.

/// Verify that `test_set_minimal_cnx_with_time` + `TestSimLink::create` +
/// `Dualq::install` complete without error.
/// C: `dualq_test_ctx_test`.
fn dualq_ctx_test() -> crate::Result<()> {
    todo!("dualq_ctx_test: minimal-cnx and link creation not yet implemented")
}

/// Enqueue one packet per ECN value into the matching queue and verify
/// that `queue_bytes`, head, and tail pointers update correctly.
/// C: `dualq_enqueue_test`.
fn dualq_enqueue() -> crate::Result<()> {
    todo!("dualq_enqueue: queue field inspection not yet implemented")
}

/// Load five packets, dequeue until both queues are empty, and verify
/// all five packets are delivered within 100 ms simulated time.
/// C: `dualq_dequeue_test`.
fn dualq_dequeue() -> crate::Result<()> {
    todo!("dualq_dequeue: dequeue loop not yet implemented")
}

/// Submit 50 packets and verify that: the first packet goes directly to
/// the link when the link queue is idle; subsequent packets go to the AQM
/// queues; and at least one packet is dropped when the combined queue
/// exceeds `dqs.limit`.
/// C: `dualq_submit_test`.
fn dualq_submit() -> crate::Result<()> {
    todo!("dualq_submit: link submit routing not yet implemented")
}

/// Drive 100 mixed-ECN packets through the full submit→dequeue pipeline,
/// interleaving arrivals, admissions, and submissions; verify all 100 are
/// accounted for (received + dropped) within 125 ms simulated time.
/// C: `dualq_sustain_test`.
fn dualq_sustain() -> crate::Result<()> {
    todo!("dualq_sustain: end-to-end loop not yet implemented")
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
