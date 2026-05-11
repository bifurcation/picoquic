//! Test-only translations of `picoquictest/`.
//!
//! Two kinds of submodule live here:
//!
//! * **Test infrastructure** — translations of the C support
//!   sources whose only callers are the test suite ([`dualq`],
//!   [`util`]: the DualQ AQM impl, the deterministic test RNG,
//!   the simulator helpers, the TLS-fixture certificate paths).
//!
//! * **Test cases** — one Rust submodule per `picoquictest/<X>.c`,
//!   carrying the `#[test] fn` translations of every entry in
//!   `picoquic_t/picoquic_t.c`'s `test_table[]`.  The modules are
//!   filled in incrementally as their matching C bodies are ported.

// Test infrastructure.
pub mod dualq;
pub mod util;

// Generated test-case modules (one per picoquictest/*.c).
mod ack_frequency;
mod ack_of_ack;
mod app_limited;
mod bytestream;
mod cert_verify;
mod cleartext_aead;
mod cnx_creation;
mod cnxstress;
mod code_version;
mod config;
mod congestion;
mod cplusplus;
mod cpu_limited;
mod datagram;
mod delay_tolerant;
mod dualq_aqm;
mod ech;
mod edge_cases;
mod flow_control;
mod getter;
mod harness;
mod hashtest;
mod high_latency;
mod intformattest;
mod l4s;
#[cfg(feature = "sys-mbedtls")]
mod mbedtls;
mod mediatest;
mod memlog;
mod minicrypto;
mod multipath;
mod netperf;
mod openssl;
mod p2p;
mod pacing;
mod parseheadertest;
mod picolog;
mod picoquic_lb;
mod pn2pn64test;
mod qlog;
mod qlog_frame;
mod quic_tester;
mod sacktest;
mod satellite;
mod skip_frame;
mod socket;
mod sockloop;
mod spinbit;
mod splay;
mod stream0_frame;
mod stresstest;
mod ticket_store;
mod tls_api;
mod transport_param;
mod util_test;
mod warptest;
mod wifitest;
