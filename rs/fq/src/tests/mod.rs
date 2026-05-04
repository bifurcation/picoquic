//! Test-only translations of `picoquictest/`.
//!
//! Modules here are gated on `cfg(test)` from [`crate`]; they
//! translate the internal test-support sources whose only callers
//! are the test suite (`picoquictest/dualq_aqm.c`,
//! `picoquictest/sim_link.c`, the deterministic test RNG, and the
//! TLS-fixture certificate paths).

pub mod dualq;
pub mod util;
