//! Test-only utilities split out of [`crate::utils`].
//!
//! Contents:
//!
//! * **Deterministic test RNG** (`test_random`, `test_random_bytes`,
//!   `test_uniform_random`, `test_gauss_random`,
//!   `test_poisson_random`) — picoquic uses these to make
//!   simulator runs reproducible.  The production RNG
//!   (`crate::utils::uniform_random`) stays in `utils.rs`.
//!
//! * **Network simulator** (`TestSimPacket`, `TestSimLink`,
//!   `TestAqm`, `JitterMode`) — the in-process sim-link used by
//!   the test suite to exercise the QUIC stack without real sockets.
//!   The C bodies live in `picoquictest/sim_link.c`.
//!
//! * **TLS API test context** (`TestTlsApiCtx`, `TestApiStreamDesc`,
//!   `tls_api_one_scenario_init_ex`, etc.) — translation of the
//!   `picoquic_test_tls_api_ctx_t` infrastructure and the helpers
//!   declared in `picoquictest/picoquictest_internal.h`.
//!
//! * **Test-fixture certificate / SNI constants** — paths to PEM
//!   files baked into the test tree (`certs/...`).
//!
//! All `pub` items in this module ride on the parent's
//! `#[cfg(test)]` gate, so they don't bloat the production build.

use core::net::SocketAddr;

use crate::internal::{Connection, MAX_ACK_RANGE_REPEAT, SackList, Version, format_ack_frame};
use crate::tp::TransportParameters;
use crate::{ConnectionId, Instant, MAX_PACKET_SIZE, PacketContext, Quic};

// ---------------------------------------------------------------------------
// Deterministic test RNG.
//
// The production `uniform_random` reads from the platform RNG
// (and stays in `crate::utils`).  These deterministic helpers
// thread their state through the explicit `&mut u64` context
// so simulator runs reproduce bit-for-bit.

/// Deterministic test RNG: advance the 64-bit context and return
/// the new value.  C: `uint64_t test_random(uint64_t*
/// random_context)`.
pub fn test_random(_random_context: &mut u64) -> u64 {
    todo!()
}

/// Fill `bytes` with deterministic test RNG output.  C:
/// `void test_random_bytes(uint64_t* random_context,
/// uint8_t* bytes, size_t bytes_max)` — `bytes_max` folds into the
/// slice length.
pub fn test_random_bytes(_random_context: &mut u64, _bytes: &mut [u8]) {
    todo!()
}

/// Uniform test RNG in `[0, rnd_max)`.  C:
/// `test_uniform_random`.
pub fn test_uniform_random(_random_context: &mut u64, _rnd_max: u64) -> u64 {
    todo!()
}

/// Gaussian-distributed test RNG (variance 1, mean 0).  C:
/// `double test_gauss_random(uint64_t* random_context)`.
pub fn test_gauss_random(_random_context: &mut u64) -> f64 {
    todo!()
}

/// Poisson-distributed test RNG.  C:
/// `uint64_t test_poisson_random(uint64_t*, uint64_t)`
/// where the second argument is `(uint64_t)(exp(-lambda) * 0x40000000)`.
pub fn test_poisson_random(_random_context: &mut u64, _exp_minus_lambda_2_30: u64) -> u64 {
    todo!()
}

// ---------------------------------------------------------------------------
// Network simulator (sim_link).

/// One simulated packet flowing through a sim link.  C:
/// `picoquictest_sim_packet_t`.
///
/// Pointer-shape choices:
///
/// * Packets live in `TestSimLink.packets: VecDeque<TestSimPacket>`
///   (and `tests::dualq::DualqQueue.packets` for the AQM); the C
///   `next_packet` intrusive chain is gone.
/// * The two `sockaddr_storage` fields fold into
///   `Option<SocketAddr>` (the C zero-initialised storage maps to
///   `None`).
/// * The flexible-array-style `bytes` is a fixed
///   `[u8; MAX_PACKET_SIZE]` because the C struct
///   declares it inline at that exact size.
pub struct TestSimPacket {
    pub arrival_time: Instant,
    pub length: usize,
    pub addr_from: Option<SocketAddr>,
    pub addr_to: Option<SocketAddr>,
    pub ecn_mark: u8,
    pub bytes: [u8; MAX_PACKET_SIZE],
}

impl TestSimPacket {
    /// Allocate a fresh, empty packet.  C:
    /// `picoquictest_sim_link_create_packet`.
    pub fn create() -> Result<Self, crate::Error> {
        todo!()
    }
}

/// Active queue management vtable.  C: the `picoquictest_aqm_t`
/// struct of function pointers — folded into a single trait per
/// the Phase 1 rule on function pointers.  The `self` parameter
/// of each C method becomes the implicit `&mut self`; the
/// `picoquictest_sim_link_t*` link pointer stays explicit because
/// the AQM lives inside the link (taking the link by `&mut` in
/// each call would conflict with the `&mut self` borrow).  Phase 4
/// will resolve the borrow with a take-replace pattern.
pub trait TestAqm {
    /// Submit a packet to the AQM.  C: `submit`.
    fn submit(&mut self, link: &mut TestSimLink, packet: TestSimPacket, current_time: Instant);

    /// Reset the AQM state at `current_time`.  C: `reset`.
    fn reset(&mut self, link: &mut TestSimLink, current_time: Instant);

    /// Release any resources held by the AQM, e.g. when the link
    /// is being torn down.  C: `release`.
    fn release(&mut self, link: &mut TestSimLink);

    /// Whether the AQM has at least one pending packet ready to
    /// admit.  C: `has_pending` returning a 0/1 flag, mapped to
    /// `bool`.
    fn has_pending(&mut self) -> bool;

    /// Move any AQM-pending packets onto the link's main queue.
    /// C: `admit_pending`.
    fn admit_pending(&mut self, link: &mut TestSimLink, current_time: Instant);
}

/// Jitter model used by the sim link.  C: `picoquic_jitter_mode`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum JitterMode {
    /// Gaussian jitter.  C: `jitter_gauss`.
    #[default]
    Gauss = 0,
    /// Wi-Fi-style jitter.  C: `jitter_wifi`.
    Wifi = 1,
}

/// One simulated network link with an embedded queue plus AQM
/// hook.  C: `picoquictest_sim_link_t`.
///
/// Pointer-shape choices, derived from the bodies in
/// `picoquictest/sim_link.c`:
///
/// * `packets` replaces the C `first_packet` / `last_packet`
///   doubly-linked list head pair plus the per-`TestSimPacket`
///   `next_packet` chain.
/// * `loss_mask` — the C field was `*mut u64`, an externally-owned
///   error mask the link reads on every enqueue.  In the Rust port
///   the link owns its own copy: tests `&mut link.loss_mask` to
///   shift the mask between operations.  `None` matches the C
///   `NULL` sentinel.
/// * `aqm_state` becomes `Option<Box<dyn TestAqm>>` —
///   `None` matches the C `NULL` (no AQM installed).
/// * `is_switched_off` / `is_unreachable` / `is_suspended` were
///   `int` flags in C; promoted to `bool`.
pub struct TestSimLink {
    pub next_send_time: Instant,
    pub queue_time: Instant,
    pub resume_time: Instant,
    pub queue_delay_max: u64,
    pub picosec_per_byte: u64,
    pub microsec_latency: u64,
    pub packets_dropped: u64,
    pub packets_sent: u64,
    pub jitter: u64,
    pub jitter_mode: JitterMode,
    pub jitter_seed: u64,
    pub path_mtu: usize,
    /// Packets in flight on this link.  FIFO; the head is the
    /// next packet to deliver.
    pub packets: std::collections::VecDeque<TestSimPacket>,
    /// 64-bit error mask used in unit tests.  `None` ↔ "no mask".
    pub loss_mask: Option<u64>,
    pub nb_loss_in_burst: u64,
    pub packets_between_losses: u64,
    pub packets_sent_next_burst: u64,
    pub nb_losses_this_burst: u64,
    pub end_of_burst_time: Instant,
    pub aqm_state: Option<Box<dyn TestAqm>>,
    pub is_switched_off: bool,
    pub is_unreachable: bool,
    pub is_suspended: bool,
}

impl TestSimLink {
    /// Create a sim link at `current_time`, with the given data
    /// rate (in gigabits per second) and one-way latency (in
    /// microseconds).  C: `picoquictest_sim_link_create`.
    ///
    /// `loss_mask` is the test's 64-bit error mask; `None` matches
    /// the C `NULL` (no mask).
    pub fn create(
        _data_rate_in_gbps: f64,
        _microsec_latency: u64,
        _loss_mask: Option<u64>,
        _queue_delay_max: u64,
        _current_time: Instant,
    ) -> Result<Self, crate::Error> {
        todo!()
    }

    // C: `picoquictest_sim_link_delete`.  Dropped from the Rust
    // API: `Box<TestSimLink>` going out of scope will free the
    // link and its queued packets via Drop in Phase 4.

    /// Time at which the next packet will arrive (or `current_time`
    /// if the queue is empty).  C:
    /// `picoquictest_sim_link_next_arrival`.
    pub fn next_arrival(&mut self, _current_time: Instant) -> u64 {
        todo!()
    }

    /// Drain any AQM-pending packets onto the main queue at
    /// `current_time`.  C: `picoquictest_sim_link_admit_pending`.
    pub fn admit_pending(&mut self, _current_time: Instant) {
        todo!()
    }

    /// Time at which the AQM will admit its next packet (or
    /// `next_time` if nothing is pending).  C:
    /// `picoquictest_sim_link_next_admission`.
    pub fn next_admission(&mut self, _current_time: Instant, _next_time: Instant) -> u64 {
        todo!()
    }

    /// Pop the next-due packet, if any.  C:
    /// `picoquictest_sim_link_dequeue` returning `NULL` when
    /// nothing is ready, mapped to `Option<TestSimPacket>`.
    pub fn dequeue(&mut self, _current_time: Instant) -> Option<TestSimPacket> {
        todo!()
    }

    /// Submit a packet to the queue with normal AQM processing and
    /// length check.  C: `picoquictest_sim_link_submit`.  Takes
    /// ownership of the packet — the link is responsible for
    /// either freeing it (drop) or returning it via
    /// [`TestSimLink::dequeue`].
    pub fn submit(&mut self, _packet: TestSimPacket, _current_time: Instant) {
        todo!()
    }

    /// Submit a packet straight to the latency queue, bypassing the
    /// AQM.  When `should_drop` is `true` the packet is dropped
    /// instead of queued (and freed by the function).  C:
    /// `picoquictest_sim_link_enqueue` with the C `int
    /// should_drop` promoted to `bool`.
    pub fn enqueue(&mut self, _packet: TestSimPacket, _current_time: Instant, _should_drop: bool) {
        todo!()
    }

    /// Compute the transmission time of `packet` (a function of
    /// the link's data rate and the packet length).  C:
    /// `picoquictest_sim_link_transmit_time`.
    pub fn transmit_time(&mut self, _packet: &TestSimPacket) -> u64 {
        todo!()
    }

    /// Queueing delay of the next packet at `current_time`.  C:
    /// `picoquictest_sim_link_queue_delay`.
    pub fn queue_delay(&mut self, _current_time: Instant) -> u64 {
        todo!()
    }

    /// Simulate a transmission interruption until
    /// `time_end_of_interval`.  When `simulate_receive` is `true`
    /// the link suspends *reception* (pending packets are
    /// delivered at the end of the interval); when `false` it
    /// suspends transmission (packets are queued as if transmitted
    /// in sequence after the interval).  C:
    /// `picoquic_test_simlink_suspend` with the C `int
    /// simulate_receive` promoted to `bool`.
    pub fn suspend(&mut self, _time_end_of_interval: Instant, _simulate_receive: bool) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Sack / ACK-frame test helpers.

/// Verify that every per-range send counter in `sack_list` is consistent
/// with the summary stored in `sack_list.rc`.  C: `check_ack_ranges` in
/// `picoquictest/sacktest.c`.
pub fn check_ack_ranges(sack_list: &mut SackList) {
    for r in 0..2usize {
        let mut range_sum = [0i32; MAX_ACK_RANGE_REPEAT];
        let mut tok = sack_list.first_item();
        while let Some(t) = tok {
            let nb = sack_list
                .sack_items
                .get(t)
                .expect("sack item token valid")
                .nb_times_sent[r];
            assert!(nb >= 0, "nb_times_sent[{r}] < 0");
            let idx = nb as usize;
            if idx < MAX_ACK_RANGE_REPEAT {
                range_sum[idx] += 1;
            }
            tok = sack_list.sack_next_item(t);
        }
        for (i, &expected) in range_sum.iter().enumerate() {
            assert_eq!(
                sack_list.rc[r].range_counts[i], expected,
                "rc[{r}].range_counts[{i}] mismatch",
            );
        }
    }
}

/// Write an ACK frame into `bytes` and return the number of bytes written.
/// `None` maps to a test failure; `Some(0)` can't occur for a valid ACK.
/// Wraps [`crate::internal::format_ack_frame`] to avoid lifetime tangle.
pub fn format_ack_frame_written(
    connection: &mut Connection,
    bytes: &mut [u8],
    more_data: &mut i32,
    current_time: Instant,
    pc: PacketContext,
    is_opportunistic: i32,
) -> Option<usize> {
    let total = bytes.len();
    format_ack_frame(
        connection,
        bytes,
        more_data,
        current_time,
        pc,
        is_opportunistic,
    )
    .map(|r| total - r.len())
}

// ---------------------------------------------------------------------------
// TLS API test context — translation of `picoquic_test_tls_api_ctx_t`
// and the helpers from `picoquictest/picoquictest_internal.h`.

/// One stream scenario descriptor.  C: `test_api_stream_desc_t`.
pub struct TestApiStreamDesc {
    /// `stream_id` in C.
    pub stream_id: u64,
    /// `previous_stream_id` in C.
    pub previous_stream_id: u64,
    /// Query (client→server) payload length.
    pub q_len: usize,
    /// Response (server→client) payload length.
    pub r_len: usize,
}

/// Combined TLS-API test context holding two QUIC contexts, their
/// two sim-links, and the addresses/callbacks baked in by the C
/// initialiser helpers.  C: `picoquic_test_tls_api_ctx_t`.
///
/// Ownership notes:
/// * `qclient` and `qserver` own the QUIC contexts.
/// * `cnx_client` and `cnx_server` are connection handles that live
///   *inside* their respective QUIC contexts; they are surfaced via
///   the [`cnx_client`][`TestTlsApiCtx::cnx_client`] /
///   [`cnx_server`][`TestTlsApiCtx::cnx_server`] methods (Phase 4
///   will wire these into real accessor paths).
/// * The sim-links are owned here and mutably shared with the
///   simulation loop.
pub struct TestTlsApiCtx {
    pub qclient: Box<Quic>,
    pub qserver: Box<Quic>,
    pub c_to_s_link: Box<TestSimLink>,
    pub s_to_c_link: Box<TestSimLink>,
}

impl TestTlsApiCtx {
    /// Mutable reference to the client connection.
    /// C: `test_ctx->cnx_client`.
    pub fn cnx_client(&mut self) -> &mut Connection {
        todo!()
    }

    /// Mutable reference to the server-side connection that was
    /// accepted in response to the client.
    /// C: `test_ctx->cnx_server`.
    pub fn cnx_server(&mut self) -> &mut Connection {
        todo!()
    }
}

/// Initialise a TLS-API test context with the `_ex` variant that
/// accepts an explicit initial CID.
/// C: `tls_api_one_scenario_init_ex`.
pub fn tls_api_one_scenario_init_ex(
    _simulated_time: &mut Instant,
    _proposed_version: Version,
    _client_params: Option<&TransportParameters>,
    _server_params: Option<&TransportParameters>,
    _initial_cid: Option<&ConnectionId>,
) -> Option<Box<TestTlsApiCtx>> {
    todo!()
}

/// Drive the TLS handshake to completion.
/// C: `tls_api_connection_loop`.
pub fn tls_api_connection_loop(
    _test_ctx: &mut TestTlsApiCtx,
    _loss_mask: &mut u64,
    _queue_delay_max: u64,
    _simulated_time: &mut Instant,
) -> crate::Result<()> {
    todo!()
}

/// Spin the simulator until the client connection reaches the ready
/// state.  C: `wait_client_connection_ready`.
pub fn wait_client_connection_ready(
    _test_ctx: &mut TestTlsApiCtx,
    _simulated_time: &mut Instant,
) -> crate::Result<()> {
    todo!()
}

/// Register the stream scenario on the test context so that the
/// send/receive loop will drive those streams.
/// C: `test_api_init_send_recv_scenario`.
pub fn test_api_init_send_recv_scenario(
    _test_ctx: &mut TestTlsApiCtx,
    _scenario: &[TestApiStreamDesc],
) -> crate::Result<()> {
    todo!()
}

/// Drive data delivery until all streams in the scenario are done.
/// C: `tls_api_data_sending_loop`.
pub fn tls_api_data_sending_loop(
    _test_ctx: &mut TestTlsApiCtx,
    _loss_mask: &mut u64,
    _simulated_time: &mut Instant,
    _max_trials: i32,
) -> crate::Result<()> {
    todo!()
}

/// Assert that all scenario streams completed and that the wall-clock
/// time did not exceed `max_completion_microsec` (0 = unconstrained).
/// C: `tls_api_one_scenario_body_verify`.
pub fn tls_api_one_scenario_body_verify(
    _test_ctx: &mut TestTlsApiCtx,
    _simulated_time: &mut Instant,
    _max_completion_microsec: u64,
) -> crate::Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// SNI / certificate paths used by the test suite.  Linux/macOS
// layout only; `_WINDOWS` paths are dropped per the v1 scope.

/// Default SNI string for the test fixtures.
pub const TEST_SNI: &str = "test.example.com";

pub const TEST_FILE_SERVER_CERT: &str = "certs/cert.pem";
pub const TEST_FILE_SERVER_BAD_CERT: &str = "certs/badcert.pem";
pub const TEST_FILE_SERVER_KEY: &str = "certs/key.pem";
pub const TEST_FILE_CERT_STORE: &str = "certs/test-ca.crt";
pub const TEST_FILE_SERVER_CERT_ECDSA: &str = "certs/ecdsa/cert.pem";
pub const TEST_FILE_SERVER_KEY_ECDSA: &str = "certs/ecdsa/key.pem";
pub const TEST_ECH_PUB_KEY: &str = "certs/ech/public.pem";
pub const TEST_ECH_PRIVATE_KEY: &str = "certs/ech/private.pem";
pub const TEST_ECH_CONFIG: &str = "certs/ech/ech_config.txt";
pub const TEST_ECH_CERT: &str = "certs/ech/ech_cert.pem";
pub const TEST_ECH_RR_REF: &str = "certs/ech/ech_rr.txt";
pub const TEST_ECH_CONFIG_REF: &str = "certs/ech/ech_config.txt";
pub const TEST_FILE_SERVER_CERT_RSA: &str = "certs/rsa/cert.pem";
pub const TEST_FILE_SERVER_KEY_RSA: &str = "certs/rsa/key.pem";
pub const TEST_FILE_SERVER_CERT_ED25519: &str = "certs/mtls_ed25519/server.crt";

// ---------------------------------------------------------------------------
// Single-step simulator.

/// Advance the simulation by one round, respecting `time_out` as the earliest
/// wake-up.  `was_active` is set to `true` when at least one packet was
/// processed.  C: `tls_api_one_sim_round`.
pub fn tls_api_one_sim_round(
    _test_ctx: &mut TestTlsApiCtx,
    _simulated_time: &mut Instant,
    _time_out: Instant,
    _was_active: &mut bool,
) -> crate::Result<()> {
    todo!()
}
pub const TEST_FILE_SERVER_KEY_ED25519: &str = "certs/mtls_ed25519/server.key";
pub const TEST_FILE_CLIENT_CERT_ED25519: &str = "certs/mtls_ed25519/client.crt";
pub const TEST_FILE_CLIENT_KEY_ED25519: &str = "certs/mtls_ed25519/client.key";
pub const TEST_FILE_CERT_STORE_ED25519: &str = "certs/mtls_ed25519/ca.crt";
