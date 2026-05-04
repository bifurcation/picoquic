//! Test-only translations of `picoquictest/`.
//!
//! Modules here are gated on `cfg(test)` from [`crate`].  They
//! translate (a) the internal test-support sources whose only
//! callers are the test suite (`dualq_aqm.c`, `sim_link.c`, the
//! deterministic test RNG, the TLS-fixture certificate paths) and
//! (b) the test cases themselves, registered in `picoquic_ct`.
//!
//! ## Phase 3 stubs
//!
//! `phase3_*.rs` files contain auto-generated `#[test]` stubs —
//! one per row in `picoquic_t/picoquic_t.c`'s `test_table[]`.
//! Bodies are `todo!()` until Phase 3's body-translation pass
//! lands.  The stubs are grouped by destination Rust module to
//! make the eventual move into per-module `mod test {}` blocks
//! mechanical.  Files live here for now (Phase 3 acceptance gate
//! is "every test panics on todo!()", which is satisfied
//! regardless of physical placement).

pub mod dualq;
pub mod util;

mod phase3_bytestream;
mod phase3_config;
mod phase3_dualq;
mod phase3_hash;
mod phase3_lib;
mod phase3_packet_loop;
mod phase3_qlog;
mod phase3_splay;
mod phase3_tests;
mod phase3_utils;
