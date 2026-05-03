//! Test-only translations of `picoquictest/`.
//!
//! Modules here are gated on `cfg(test)` from [`crate`]; they
//! translate the internal test-support sources whose only callers
//! are the test suite (`picoquictest/dualq_aqm.c`, etc.).
//!
//! REVIEW(open): the simulator scaffolding currently in
//! `crate::utils` (`TestSimPacket`, `TestSimLink`, `TestAqm`,
//! `JitterMode`) belongs here too — pulling it across is part of
//! the same cleanup but is a bigger move because every reference
//! crosses module boundaries.  Land it once the rest of Phase 1B
//! has settled.

pub mod dualq;
