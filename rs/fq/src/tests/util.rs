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

use crate::internal::{
    Connection, ConnectionToken, DEFAULT_HOLE_PERIOD, Epoch, INTEROP_VERSION_LATEST,
    MAX_ACK_RANGE_REPEAT, NB_PATH_TARGET, PacketType, SackList, Version, format_ack_frame,
    init_transport_parameters, public_random,
};
use crate::tp::TransportParameters;
use crate::{
    AES_128_GCM_SHA256, ConnectionId, ConnectionIdCallback, GROUP_SECP256R1, INITIAL_MTU_IPV4,
    Instant, LossbitVersion, MAX_PACKET_SIZE, PacketContext, Quic, RESET_SECRET_SIZE,
    SpinbitVersion, State, public_random_seed_64,
};

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
pub fn test_random(random_context: &mut u64) -> u64 {
    *random_context = random_context.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *random_context;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9u64);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111ebu64);
    z ^ (z >> 31)
}

/// Fill `bytes` with deterministic test RNG output.  C:
/// `void test_random_bytes(uint64_t* random_context,
/// uint8_t* bytes, size_t bytes_max)` — `bytes_max` folds into the
/// slice length.
pub fn test_random_bytes(random_context: &mut u64, bytes: &mut [u8]) {
    let mut byte_index = 0;
    while byte_index < bytes.len() {
        let mut v = test_random(random_context);
        for _ in 0..8 {
            if byte_index >= bytes.len() {
                break;
            }
            bytes[byte_index] = (v & 0xFF) as u8;
            byte_index += 1;
            v >>= 8;
        }
    }
}

/// Uniform test RNG in `[0, rnd_max)`.  C:
/// `test_uniform_random`.
pub fn test_uniform_random(random_context: &mut u64, rnd_max: u64) -> u64 {
    if rnd_max == 0 {
        return 0;
    }
    let rnd_min = u64::MAX % rnd_max;
    loop {
        let rnd = test_random(random_context);
        if rnd >= rnd_min {
            return rnd % rnd_max;
        }
    }
}

/// Gaussian-distributed test RNG (variance 1, mean 0).  C:
/// `double test_gauss_random(uint64_t* random_context)`.
pub fn test_gauss_random(random_context: &mut u64) -> f64 {
    let mut dx = 0.0f64;
    for _ in 0..12 {
        let mut r = test_random(random_context);
        r ^= r >> 17;
        r ^= r >> 34;
        let d = (r & 0x1ffff) as f64 + 0.5;
        dx += d / (0x20000u64 as f64);
    }
    dx - 6.0
}

/// Poisson-distributed test RNG.  C:
/// `uint64_t test_poisson_random(uint64_t*, uint64_t)`
/// where the second argument is `(uint64_t)(exp(-lambda) * 0x40000000)`.
pub fn test_poisson_random(random_context: &mut u64, exp_minus_lambda_2_30: u64) -> u64 {
    let mut k = 0u64;
    let mut p = 0x40000000u64;
    loop {
        let r = test_random(random_context);
        let r = (r ^ (r >> 30)) & 0x3fffffff;
        p = (p * r) >> 30;
        k += 1;
        if p <= exp_minus_lambda_2_30 {
            break;
        }
    }
    k - 1
}

// ---------------------------------------------------------------------------
use core::any::Any;

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
        Ok(Self {
            arrival_time: Instant::from_ticks(0),
            length: 0,
            addr_from: None,
            addr_to: None,
            ecn_mark: 0,
            bytes: [0u8; MAX_PACKET_SIZE],
        })
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
pub trait TestAqm: Any {
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

    /// Return this AQM as [`Any`] for test-only inspection of concrete state.
    fn as_any_mut(&mut self) -> &mut dyn Any;
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
/// * `loss_mask` — the C field is `*mut u64`, an externally-owned
///   error mask the link reads on every enqueue.  The Rust link stores
///   the current value locally; the TLS API loop helpers synchronize
///   both directions around each simulator round so they consume the
///   same logical mask.  `None` matches the C `NULL` sentinel.
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
    // -----------------------------------------------------------------------
    // Private helpers.

    /// C: `picoquic/sim_link.c:picoquictest_sim_link_testloss`.
    fn sim_testloss(&mut self) -> bool {
        if let Some(mask) = self.loss_mask.as_mut() {
            let loss_bit = *mask & 1;
            *mask = (*mask >> 1) | (loss_bit << 63);
            loss_bit != 0
        } else {
            false
        }
    }

    /// C: `picoquic/sim_link.c:picoquictest_sim_link_simloss`.
    fn sim_simloss(&mut self, current_time: Instant) -> bool {
        if self.nb_loss_in_burst == 0 {
            return false;
        }
        let ct = current_time.ticks();
        if self.packets_sent > self.packets_sent_next_burst {
            let picosec_wait = self.nb_loss_in_burst * self.picosec_per_byte * 1536;
            self.packets_sent_next_burst = self.packets_sent + self.packets_between_losses;
            self.nb_losses_this_burst = self.nb_loss_in_burst - 1;
            self.end_of_burst_time = Instant::from_ticks(ct + picosec_wait / 1_000_000);
            true
        } else if self.nb_losses_this_burst > 0 {
            if ct > self.end_of_burst_time.ticks() {
                self.nb_losses_this_burst = 0;
                false
            } else {
                self.nb_losses_this_burst -= 1;
                true
            }
        } else {
            false
        }
    }

    fn sim_wifi_jitter(&mut self) -> u64 {
        const EXP_MINUS_1: u64 = 395007542;
        const PRIMARY: u64 = 1000;
        let n1 = test_poisson_random(&mut self.jitter_seed, EXP_MINUS_1);
        let mut jitter = n1 * PRIMARY;
        if n1 > 0 {
            jitter = jitter.saturating_sub(test_uniform_random(&mut self.jitter_seed, PRIMARY));
        }
        if self.jitter > 1000 {
            let r = test_random(&mut self.jitter_seed);
            let r = (r ^ (r >> 30)) & 0x3fffffff;
            let r_scaled = r.wrapping_mul(84000);
            if r_scaled < ((self.jitter - 1000) << 30) {
                const EXP_MINUS_12: u64 = 6597;
                const SECONDARY: u64 = 7500;
                let n2 = test_poisson_random(&mut self.jitter_seed, EXP_MINUS_12);
                jitter += n2 * SECONDARY;
                if n2 > 1 {
                    jitter = jitter
                        .saturating_sub(test_uniform_random(&mut self.jitter_seed, SECONDARY));
                }
            }
        }
        jitter
    }

    fn sim_jitter(&mut self) -> u64 {
        if self.jitter_mode == JitterMode::Wifi {
            self.sim_wifi_jitter()
        } else {
            let x = test_gauss_random(&mut self.jitter_seed).max(-3.0) / 3.0;
            let j = self.jitter as i64;
            (j + (x * j as f64) as i64).max(0) as u64
        }
    }

    // -----------------------------------------------------------------------
    // Public API.

    /// Create a sim link at `current_time`, with the given data
    /// rate (in gigabits per second) and one-way latency (in
    /// microseconds).  C: `picoquictest_sim_link_create`.
    ///
    /// `loss_mask` is the test's 64-bit error mask; `None` matches
    /// the C `NULL` (no mask).
    pub fn create(
        data_rate_in_gbps: f64,
        microsec_latency: u64,
        loss_mask: Option<u64>,
        queue_delay_max: u64,
        current_time: Instant,
    ) -> Result<Self, crate::Error> {
        let pico_d = if data_rate_in_gbps <= 0.0 {
            0.0
        } else {
            8000.0 / data_rate_in_gbps
        };
        let pico_d = pico_d * 1.024 * 1.024;
        Ok(Self {
            next_send_time: current_time,
            queue_time: current_time,
            resume_time: Instant::from_ticks(0),
            queue_delay_max,
            picosec_per_byte: pico_d as u64,
            microsec_latency,
            packets_dropped: 0,
            packets_sent: 0,
            jitter: 0,
            jitter_mode: JitterMode::Gauss,
            jitter_seed: 0xDEAD_BEEF_BABA_C001u64,
            path_mtu: MAX_PACKET_SIZE,
            packets: std::collections::VecDeque::new(),
            loss_mask,
            nb_loss_in_burst: 0,
            packets_between_losses: 0,
            packets_sent_next_burst: 0,
            nb_losses_this_burst: 0,
            end_of_burst_time: Instant::from_ticks(0),
            aqm_state: None,
            is_switched_off: false,
            is_unreachable: false,
            is_suspended: false,
        })
    }

    // C: `picoquictest_sim_link_delete`.  Rust lets
    // `Box<TestSimLink>` fall out of ownership; `Drop` releases the
    // link and its queued packets.

    /// Time at which the next packet will arrive (or `current_time`
    /// if the queue is empty).  C:
    /// `picoquictest_sim_link_next_arrival`.
    pub fn next_arrival(&mut self, current_time: Instant) -> u64 {
        let ct = current_time.ticks();
        if let Some(front) = self.packets.front() {
            let at = front.arrival_time.ticks();
            if at < ct {
                return at;
            }
        }
        ct
    }

    /// Drain any AQM-pending packets onto the main queue at
    /// `current_time`.  C: `picoquictest_sim_link_admit_pending`.
    pub fn admit_pending(&mut self, current_time: Instant) {
        if let Some(mut aqm) = self.aqm_state.take() {
            aqm.admit_pending(self, current_time);
            self.aqm_state = Some(aqm);
        }
    }

    /// Time at which the AQM will admit its next packet (or
    /// `next_time` if nothing is pending).  C:
    /// `picoquictest_sim_link_next_admission`.
    pub fn next_admission(&mut self, current_time: Instant, next_time: Instant) -> u64 {
        let mut nt = next_time.ticks();
        if let Some(mut aqm) = self.aqm_state.take() {
            if aqm.has_pending() {
                let candidate = if self.packets.is_empty() {
                    current_time.ticks()
                } else {
                    self.queue_time.ticks()
                };
                if candidate < nt {
                    nt = candidate;
                }
            }
            self.aqm_state = Some(aqm);
        }
        nt
    }

    /// Pop the next-due packet, if any.  C:
    /// `picoquictest_sim_link_dequeue` returning `NULL` when
    /// nothing is ready, mapped to `Option<TestSimPacket>`.
    pub fn dequeue(&mut self, current_time: Instant) -> Option<TestSimPacket> {
        if let Some(front) = self.packets.front()
            && front.arrival_time.ticks() <= current_time.ticks()
        {
            return self.packets.pop_front();
        }
        None
    }

    /// Submit a packet to the queue with normal AQM processing and
    /// length check.  C: `picoquictest_sim_link_submit`.  Takes
    /// ownership of the packet — the link is responsible for
    /// either freeing it (drop) or returning it via
    /// [`TestSimLink::dequeue`].
    pub fn submit(&mut self, packet: TestSimPacket, current_time: Instant) {
        if self.is_suspended {
            let mut p = packet;
            p.arrival_time = Instant::from_ticks(u64::MAX);
            self.packets.push_back(p);
            return;
        }
        if let Some(mut aqm) = self.aqm_state.take() {
            aqm.submit(self, packet, current_time);
            self.aqm_state = Some(aqm);
        } else {
            let queue_delay = self.queue_delay(current_time);
            let should_drop = self.queue_delay_max > 0 && queue_delay >= self.queue_delay_max;
            self.enqueue(packet, current_time, should_drop);
        }
    }

    /// Submit a packet straight to the latency queue, bypassing the
    /// AQM.  When `should_drop` is `true` the packet is dropped
    /// instead of queued (and freed by the function).  C:
    /// `picoquictest_sim_link_enqueue` with the C `int
    /// should_drop` promoted to `bool`.
    pub fn enqueue(&mut self, packet: TestSimPacket, current_time: Instant, should_drop: bool) {
        if should_drop {
            self.packets_dropped += 1;
            return;
        }
        let transmit_time = self.transmit_time(&packet);
        let queue_delay = self.queue_delay(current_time);
        let transmit_time = if transmit_time == 0 { 1 } else { transmit_time };
        self.queue_time = Instant::from_ticks(current_time.ticks() + queue_delay + transmit_time);

        if packet.length > self.path_mtu
            || self.sim_testloss()
            || self.is_switched_off
            || self.sim_simloss(current_time)
        {
            self.packets_dropped += 1;
            return;
        }

        self.packets_sent += 1;
        let arrival_ticks = self.queue_time.ticks() + self.microsec_latency;
        let arrival_ticks = if self.jitter != 0 {
            arrival_ticks + self.sim_jitter()
        } else {
            arrival_ticks
        };
        let arrival_ticks = arrival_ticks.max(self.resume_time.ticks());
        let mut p = packet;
        p.arrival_time = Instant::from_ticks(arrival_ticks);
        self.packets.push_back(p);
    }

    /// Compute the transmission time of `packet` (a function of
    /// the link's data rate and the packet length).  C:
    /// `picoquictest_sim_link_transmit_time`.
    pub fn transmit_time(&mut self, packet: &TestSimPacket) -> u64 {
        (self.picosec_per_byte * packet.length as u64) >> 20
    }

    /// Queueing delay of the next packet at `current_time`.  C:
    /// `picoquictest_sim_link_queue_delay`.
    pub fn queue_delay(&mut self, current_time: Instant) -> u64 {
        let qt = self.queue_time.ticks();
        let ct = current_time.ticks();
        qt.saturating_sub(ct)
    }

    /// Simulate a transmission interruption until
    /// `time_end_of_interval`.  When `simulate_receive` is `true`
    /// the link suspends *reception* (pending packets are
    /// delivered at the end of the interval); when `false` it
    /// suspends transmission (packets are queued as if transmitted
    /// in sequence after the interval).  C:
    /// `picoquic_test_simlink_suspend` with the C `int
    /// simulate_receive` promoted to `bool`.
    pub fn suspend(&mut self, time_end_of_interval: Instant, simulate_receive: bool) {
        if simulate_receive {
            self.resume_time = time_end_of_interval;
            let end = time_end_of_interval.ticks();
            for p in &mut self.packets {
                if p.arrival_time.ticks() < end {
                    p.arrival_time = time_end_of_interval;
                }
            }
        } else {
            self.queue_time = time_end_of_interval;
            let old: Vec<TestSimPacket> = self.packets.drain(..).collect();
            for p in old {
                self.submit(p, time_end_of_interval);
            }
        }
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

const TEST_MAX_TEST_STREAMS: usize = 100;

struct TestApiStream {
    stream_id: u64,
    previous_stream_id: u64,
    q_sent: bool,
    q_received: bool,
    r_received: bool,
    q_len: usize,
    r_len: usize,
    q_recv_nb: usize,
    r_recv_nb: usize,
    q_src: Vec<u8>,
    q_rcv: Vec<u8>,
    r_src: Vec<u8>,
    r_rcv: Vec<u8>,
}

impl TestApiStream {
    fn new(desc: &TestApiStreamDesc) -> Self {
        fn source_bytes(len: usize) -> Vec<u8> {
            (0..len).map(|i| i as u8).collect()
        }

        Self {
            stream_id: desc.stream_id,
            previous_stream_id: desc.previous_stream_id,
            q_sent: false,
            q_received: false,
            r_received: desc.r_len == 0,
            q_len: desc.q_len,
            r_len: desc.r_len,
            q_recv_nb: 0,
            r_recv_nb: 0,
            q_src: source_bytes(desc.q_len),
            q_rcv: vec![0; desc.q_len],
            r_src: source_bytes(desc.r_len),
            r_rcv: vec![0; desc.r_len],
        }
    }

    fn response_complete(&self) -> bool {
        self.r_received && self.r_src.len() == self.r_len && self.r_rcv.len() == self.r_len
    }
}

struct TestApiStreamEvent {
    client_mode: bool,
    stream_id: u64,
    bytes: Vec<u8>,
    fin: bool,
}

fn test_api_receive_stream_data(
    stream: &mut TestApiStream,
    response: bool,
    bytes: &[u8],
    fin: bool,
) -> bool {
    let (max_len, source, received, received_count, received_fin) = if response {
        (
            stream.r_len,
            &stream.r_src,
            &mut stream.r_rcv,
            &mut stream.r_recv_nb,
            &mut stream.r_received,
        )
    } else {
        (
            stream.q_len,
            &stream.q_src,
            &mut stream.q_rcv,
            &mut stream.q_recv_nb,
            &mut stream.q_received,
        )
    };

    if received_count.saturating_add(bytes.len()) > max_len {
        return false;
    }

    let start = *received_count;
    let end = start + bytes.len();
    received[start..end].copy_from_slice(bytes);
    if source[start..end] != bytes[..] {
        return false;
    }
    *received_count = end;

    if fin {
        if *received_fin {
            return false;
        }
        *received_fin = true;
    }

    true
}

fn collect_received_stream_events(
    cnx: &mut Connection,
    client_mode: bool,
) -> Vec<TestApiStreamEvent> {
    let mut events = Vec::new();

    for stream in cnx.streams.iter_mut() {
        let stream_id = stream.stream_id;
        while let Some(tree_token) = stream.stream_data_tree.first() {
            let Some(data_token) = stream.stream_data_tree.get(tree_token).copied() else {
                break;
            };
            let Some(data_node) = stream.stream_data_nodes.get(data_token) else {
                stream.stream_data_tree.remove(tree_token);
                continue;
            };

            let data_end = data_node.offset.saturating_add(data_node.length as u64);
            if data_end <= stream.consumed_offset {
                stream.stream_data_tree.remove(tree_token);
                stream.stream_data_nodes.remove(data_token);
                continue;
            }
            if data_node.offset > stream.consumed_offset {
                break;
            }

            let start = stream.consumed_offset.saturating_sub(data_node.offset) as usize;
            let bytes = data_node.data[start..data_node.length].to_vec();
            stream.consumed_offset = stream.consumed_offset.saturating_add(bytes.len() as u64);
            stream.stream_data_tree.remove(tree_token);
            stream.stream_data_nodes.remove(data_token);

            if !bytes.is_empty() {
                events.push(TestApiStreamEvent {
                    client_mode,
                    stream_id,
                    bytes,
                    fin: false,
                });
            }
        }

        if stream.fin_received
            && !stream.fin_signalled
            && stream.consumed_offset >= stream.fin_offset
        {
            stream.fin_signalled = true;
            events.push(TestApiStreamEvent {
                client_mode,
                stream_id,
                bytes: Vec::new(),
                fin: true,
            });
        }
    }

    events
}

fn set_test_api_callback_error(test_ctx: &mut TestTlsApiCtx, client_mode: bool) {
    if client_mode {
        test_ctx.client_callback_error_detected = true;
    } else {
        test_ctx.server_callback_error_detected = true;
    }
}

/// CPU-limiting parameters for a simulated endpoint in the test
/// network simulator.  C: `picoquictest_endpoint_t`.
#[derive(Default)]
pub struct TestClientEndpoint {
    /// Time the endpoint is unavailable after processing one incoming
    /// packet (µs).  C: `incoming_cpu_time`.
    pub incoming_cpu_time: u64,
    /// Time the endpoint is unavailable after successfully preparing
    /// one outbound packet (µs).  C: `prepare_cpu_time`.
    pub prepare_cpu_time: u64,
    /// Maximum backlog of not-yet-processed incoming packets.
    /// C: `packet_queue_max`.
    pub packet_queue_max: usize,
    /// Pass an undefined local destination address to incoming packets.
    /// C: `addr_to_unspec`.
    pub addr_to_unspec: bool,
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
    cnx_server_token: Option<ConnectionToken>,
    ignored_server_tokens: Vec<ConnectionToken>,
    pub c_to_s_link: Box<TestSimLink>,
    pub s_to_c_link: Box<TestSimLink>,
    /// Second client-to-server sim link for multipath tests.
    /// C: `test_ctx->c_to_s_link_2`.
    pub c_to_s_link_2: Option<Box<TestSimLink>>,
    /// Second server-to-client sim link for multipath tests.
    /// C: `test_ctx->s_to_c_link_2`.
    pub s_to_c_link_2: Option<Box<TestSimLink>>,
    /// Simulated client socket address (used by bdp_ip and similar tests).
    /// C: `test_ctx->client_addr`.
    pub client_addr: SocketAddr,
    /// Simulated server socket address.  C: `test_ctx->server_addr`.
    pub server_addr: SocketAddr,
    /// Second client socket address for multipath tests.
    /// C: `test_ctx->client_addr_2`.
    pub client_addr_2: SocketAddr,
    /// NAT-rebinding replacement address (multipath/NAT tests).
    /// C: `test_ctx->client_addr_natted`.
    pub client_addr_natted: SocketAddr,
    /// When `true`, substitute `client_addr_natted` for outgoing packets.
    /// C: `test_ctx->client_use_nat`.
    pub client_use_nat: bool,
    /// Prepend a malformed coalesced packet on client departures after the
    /// handshake starts. C: `test_ctx->do_bad_coalesce_test`.
    pub do_bad_coalesce_test: bool,
    /// When `true`, queue server packets sent to alternate client addresses
    /// so the test can explicitly relay or drop them.
    /// C: `test_ctx->client_use_multiple_addresses`.
    pub client_use_multiple_addresses: bool,
    /// Count of address-discovery observations received by the test.
    /// C: `test_ctx->nb_address_observed`.
    pub nb_address_observed: u64,
    /// Default per-test loss mask (used by monopath loop).
    /// C: `test_ctx->loss_mask_default`.
    pub loss_mask_default: u64,
    /// Timestamp (µs) at which the blackhole begins.  C: `blackhole_start`.
    pub blackhole_start: u64,
    /// Timestamp (µs) at which the blackhole ends.  C: `blackhole_end`.
    pub blackhole_end: u64,
    /// Size of the reusable packet-preparation buffer.
    /// C: `test_ctx->send_buffer_size`.
    pub send_buffer_size: usize,
    /// Whether the simulator should split UDP GSO trains into segments.
    /// C: `test_ctx->use_udp_gso`.
    pub use_udp_gso: bool,
    /// CPU-limiting configuration for the simulated client endpoint.
    /// C: `test_ctx->client_endpoint`.
    pub client_endpoint: TestClientEndpoint,
    /// Release flow control on stream 0 immediately.
    /// C: `test_ctx->stream0_flow_release`.
    pub stream0_flow_release: bool,
    /// Exit data loop as soon as connection is established.
    /// C: `test_ctx->immediate_exit`.
    pub immediate_exit: bool,
    /// Set when all scenario streams have completed.
    /// C: `test_ctx->test_finished`.
    pub test_finished: bool,
    /// ECN simulation mode: 0 = no ECN, 1 = ECN enabled.
    /// C: `test_ctx->ecn_support`.
    pub ecn_support: u8,
    /// Default ECN mark applied to outgoing packets.
    /// C: `test_ctx->packet_ecn_default`.
    pub packet_ecn_default: u8,
    /// Total stream bytes received by the server callback.
    /// C: `test_ctx->sum_data_received_at_server`.
    sum_data_received_at_server: usize,
    /// Whether the client-side test callback saw inconsistent stream data.
    /// C: `test_ctx->client_callback_error_detected`.
    pub client_callback_error_detected: bool,
    /// Whether the server-side test callback saw inconsistent stream data.
    /// C: `test_ctx->server_callback_error_detected`.
    pub server_callback_error_detected: bool,
    test_streams: Vec<TestApiStream>,
    stream0_target: usize,
    stream0_sent: usize,
    stream0_received: usize,
    streams_finished: bool,
}

impl TestTlsApiCtx {
    /// Push the simulator's virtual clock down to both Quic
    /// contexts.  Library calls that internally consult "now" via
    /// `Quic::time()` (`start_client`, `reinsert_self_by_wake_time`,
    /// path-quality probes, …) will then see the simulator's value
    /// instead of the wall clock.  Mirrors C's `p_simulated_time`
    /// pointer: tests own the clock, the library reads it.
    pub fn install_simulated_time(&self, t: Instant) {
        self.qclient.set_simulated_time(t.ticks());
        self.qserver.set_simulated_time(t.ticks());
    }

    /// Mutable reference to the client connection.
    /// C: `test_ctx->cnx_client`.
    pub fn cnx_client(&mut self) -> &mut Connection {
        self.qclient
            .first_cnx_mut()
            .expect("client connection not initialized")
    }

    /// Mutable reference to the server-side connection that was
    /// accepted in response to the client.
    /// C: `test_ctx->cnx_server`.
    pub fn cnx_server(&mut self) -> &mut Connection {
        let token = self
            .active_server_token()
            .expect("server connection not yet accepted");
        self.qserver
            .connections
            .get_mut(token)
            .expect("server connection token went stale")
    }

    fn active_server_token(&mut self) -> Option<ConnectionToken> {
        if let Some(token) = self.cnx_server_token
            && self.qserver.connections.contains(token)
            && !self.ignored_server_tokens.contains(&token)
        {
            return Some(token);
        }

        let token = self.qserver.connections.iter().find_map(|cnx| {
            let token = cnx.own_token?;
            (!self.ignored_server_tokens.contains(&token)).then_some(token)
        });
        self.cnx_server_token = token;
        token
    }

    fn active_server_connection(&mut self) -> Option<&mut Connection> {
        let token = self.active_server_token()?;
        self.qserver.connections.get_mut(token)
    }

    fn active_server_disconnected(&mut self) -> bool {
        self.active_server_connection()
            .map(|c| c.connection_state == crate::State::Disconnected)
            .unwrap_or(true)
    }

    /// Clear the active server connection reference without deleting the
    /// underlying server-side connections.
    /// C: `test_ctx->cnx_server = NULL`.
    pub fn clear_cnx_server_ref(&mut self) {
        let tokens: Vec<_> = self
            .qserver
            .connections
            .iter()
            .filter_map(|cnx| cnx.own_token)
            .collect();
        for token in tokens {
            if !self.ignored_server_tokens.contains(&token) {
                self.ignored_server_tokens.push(token);
            }
        }
        self.cnx_server_token = None;
    }

    /// Overwrite the simulated client socket address.
    /// C: `picoquic_set_test_address(&test_ctx->client_addr, addr_be, port)`.
    pub fn set_client_addr(&mut self, addr_be: u32, port: u16) {
        use core::net::Ipv4Addr;
        self.client_addr = SocketAddr::from((Ipv4Addr::from(addr_be.to_ne_bytes()), port.to_be()));
    }

    /// True when the client connection is in the Ready state.
    /// C: `TEST_CLIENT_READY` macro.
    pub fn client_ready(&mut self) -> bool {
        self.qclient
            .first_cnx_mut()
            .map(|c| {
                matches!(
                    c.connection_state,
                    crate::State::Ready | crate::State::ClientReadyStart
                )
            })
            .unwrap_or(false)
    }

    /// True when the server connection exists and is in the Ready state.
    /// C: `TEST_SERVER_READY` macro.
    pub fn server_ready(&mut self) -> bool {
        self.active_server_connection()
            .map(|c| {
                matches!(
                    c.connection_state,
                    crate::State::Ready | crate::State::ServerFalseStart
                )
            })
            .unwrap_or(false)
    }

    /// True when a server connection has been accepted.
    /// C: `test_ctx->cnx_server != NULL`.
    pub fn has_cnx_server(&self) -> bool {
        if let Some(token) = self.cnx_server_token {
            return self.qserver.connections.contains(token)
                && !self.ignored_server_tokens.contains(&token);
        }

        self.qserver.connections.iter().any(|cnx| {
            cnx.own_token
                .map(|token| !self.ignored_server_tokens.contains(&token))
                .unwrap_or(false)
        })
    }

    /// Configure the simulator send buffer. A non-zero value enables the C
    /// test harness' UDP-GSO style packet splitting.
    pub fn set_send_buffer_size(&mut self, send_buffer_size: usize) {
        if send_buffer_size == 0 {
            self.send_buffer_size = MAX_PACKET_SIZE;
            self.use_udp_gso = false;
        } else {
            self.send_buffer_size = send_buffer_size;
            self.use_udp_gso = true;
        }
    }

    /// True once the server-side callback has received stream bytes.
    /// C: `test_ctx->sum_data_received_at_server != 0`.
    pub fn server_received_stream_data(&self) -> bool {
        self.sum_data_received_at_server != 0
    }
}

#[allow(dead_code)]
fn tls_api_set_link_loss_mask(test_ctx: &mut TestTlsApiCtx, loss_mask: u64) {
    test_ctx.c_to_s_link.loss_mask = Some(loss_mask);
    test_ctx.s_to_c_link.loss_mask = Some(loss_mask);
    if let Some(link) = test_ctx.c_to_s_link_2.as_mut() {
        link.loss_mask = Some(loss_mask);
    }
    if let Some(link) = test_ctx.s_to_c_link_2.as_mut() {
        link.loss_mask = Some(loss_mask);
    }
}

#[allow(dead_code)]
fn tls_api_sync_link_loss_mask(test_ctx: &TestTlsApiCtx, loss_mask: &mut u64) {
    let before = *loss_mask;
    if let Some(mask) = test_ctx.c_to_s_link.loss_mask
        && mask != before
    {
        *loss_mask = mask;
        return;
    }
    if let Some(mask) = test_ctx.s_to_c_link.loss_mask
        && mask != before
    {
        *loss_mask = mask;
        return;
    }
    if let Some(link) = test_ctx.c_to_s_link_2.as_ref()
        && let Some(mask) = link.loss_mask
        && mask != before
    {
        *loss_mask = mask;
        return;
    }
    if let Some(link) = test_ctx.s_to_c_link_2.as_ref()
        && let Some(mask) = link.loss_mask
        && mask != before
    {
        *loss_mask = mask;
    }
}

#[allow(dead_code)]
fn tls_api_one_sim_round_with_loss_mask(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    time_out: Instant,
    was_active: &mut bool,
    loss_mask: &mut u64,
) -> crate::Result<()> {
    tls_api_set_link_loss_mask(test_ctx, *loss_mask);
    let ret = tls_api_one_sim_round(test_ctx, simulated_time, time_out, was_active);
    tls_api_sync_link_loss_mask(test_ctx, loss_mask);
    ret
}

/// Initialise a TLS-API test context with the `_ex` variant that
/// accepts an explicit initial CID.
/// C: `tls_api_one_scenario_init_ex`.
pub fn tls_api_one_scenario_init_ex(
    simulated_time: &mut Instant,
    proposed_version: Version,
    client_params: Option<&TransportParameters>,
    server_params: Option<&TransportParameters>,
    initial_cid: Option<&ConnectionId>,
) -> Option<Box<TestTlsApiCtx>> {
    let mut test_ctx =
        tls_api_init_ctx_ex(simulated_time, proposed_version as u32, None, initial_cid)?;

    if let Some(cp) = client_params {
        test_ctx.cnx_client().set_transport_parameters(cp);
    }
    if let Some(sp) = server_params {
        test_ctx.qserver.set_default_tp(sp).ok()?;
    }

    Some(test_ctx)
}

/// Drive the TLS handshake to completion.
/// C: `tls_api_connection_loop`.
pub fn tls_api_connection_loop(
    test_ctx: &mut TestTlsApiCtx,
    loss_mask: &mut u64,
    queue_delay_max: u64,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    test_ctx.c_to_s_link.queue_delay_max = queue_delay_max;
    test_ctx.s_to_c_link.queue_delay_max = queue_delay_max;

    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while nb_trials < 1024
        && nb_inactive < 512
        && (!test_ctx.client_ready() || !test_ctx.server_ready())
    {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round_with_loss(
            test_ctx,
            simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
            loss_mask,
        )?;

        let client_disc = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|c| c.connection_state == crate::State::Disconnected)
            .unwrap_or(true);
        let server_disc = test_ctx.active_server_disconnected();
        if client_disc && server_disc {
            break;
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }
    Ok(())
}

/// Spin the simulator until the client connection reaches the ready
/// state.  C: `wait_client_connection_ready`.
pub fn wait_client_connection_ready(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + 4_000_000);
    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while simulated_time.ticks() < time_out.ticks()
        && !matches!(
            test_ctx.qclient.first_cnx_mut().map(|c| c.connection_state),
            Some(crate::State::Ready)
        )
        && nb_trials < 1024
        && nb_inactive < 64
    {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round(test_ctx, simulated_time, time_out, &mut was_active)?;
        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    if matches!(
        test_ctx.qclient.first_cnx_mut().map(|c| c.connection_state),
        Some(crate::State::Ready)
    ) {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Register the stream scenario on the test context so that the
/// send/receive loop will drive those streams.
/// C: `test_api_init_send_recv_scenario`.
pub fn test_api_init_send_recv_scenario(
    test_ctx: &mut TestTlsApiCtx,
    scenario: &[TestApiStreamDesc],
) -> crate::Result<()> {
    if scenario.len() > TEST_MAX_TEST_STREAMS {
        return Err(crate::Error::Generic);
    }

    test_ctx.test_streams.clear();
    test_ctx
        .test_streams
        .extend(scenario.iter().map(TestApiStream::new));
    test_ctx.test_finished = false;
    test_ctx.streams_finished = false;

    test_api_queue_initial_queries(test_ctx, 0)
}

/// Drive data delivery until all streams in the scenario are done.
/// C: `tls_api_data_sending_loop`.
pub fn tls_api_data_sending_loop(
    test_ctx: &mut TestTlsApiCtx,
    loss_mask: &mut u64,
    simulated_time: &mut Instant,
    max_trials: i32,
) -> crate::Result<()> {
    let max = if max_trials <= 0 {
        4_000_000
    } else {
        max_trials
    };
    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while nb_trials < max && nb_inactive < 256 && test_ctx.client_ready() && test_ctx.server_ready()
    {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round_with_loss(
            test_ctx,
            simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
            loss_mask,
        )?;

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if test_ctx.test_finished {
            let client_empty = test_ctx
                .qclient
                .first_cnx_mut()
                .map(|c| c.is_backlog_empty())
                .unwrap_or(true);
            let server_empty = test_ctx
                .qserver
                .first_cnx_mut()
                .map(|c| c.is_backlog_empty())
                .unwrap_or(true);
            if test_ctx.immediate_exit || (client_empty && server_empty) {
                break;
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn submit_simulated_send_buffer(
    link: &mut TestSimLink,
    addr_from: SocketAddr,
    addr_to: SocketAddr,
    ecn_mark: u8,
    send_buffer: &[u8],
    segment_size: usize,
    simulated_time: Instant,
) -> crate::Result<()> {
    let segment_size = segment_size.clamp(1, MAX_PACKET_SIZE);
    let mut offset = 0usize;
    while offset < send_buffer.len() {
        let packet_len = (send_buffer.len() - offset).min(segment_size);
        let mut pkt = TestSimPacket::create()?;
        pkt.addr_from = Some(addr_from);
        pkt.addr_to = Some(addr_to);
        pkt.ecn_mark = ecn_mark;
        pkt.length = packet_len;
        pkt.bytes[..packet_len].copy_from_slice(&send_buffer[offset..offset + packet_len]);
        link.submit(pkt, simulated_time);
        offset = offset.saturating_add(segment_size);
    }
    Ok(())
}

/// Assert that all scenario streams completed and that the wall-clock
/// time did not exceed `max_completion_microsec` (0 = unconstrained).
/// C: `tls_api_one_scenario_body_verify`.
pub fn tls_api_one_scenario_body_verify(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    max_completion_microsec: u64,
) -> crate::Result<()> {
    tls_api_one_scenario_verify(test_ctx)?;

    let close_time = *simulated_time;
    tls_api_close_with_losses(test_ctx, simulated_time, 0)?;

    if max_completion_microsec != 0 {
        let completion_time = close_time
            .ticks()
            .saturating_sub(test_ctx.cnx_client().start_time.ticks());
        if completion_time > max_completion_microsec {
            return Err(crate::Error::Generic);
        }
    }

    Ok(())
}

/// Verify stream delivery and callback state for a completed scenario.
/// C: `tls_api_one_scenario_verify`.
pub fn tls_api_one_scenario_verify(test_ctx: &TestTlsApiCtx) -> crate::Result<()> {
    if test_ctx.server_callback_error_detected || test_ctx.client_callback_error_detected {
        return Err(crate::Error::Generic);
    }

    if !test_ctx.test_finished {
        return Err(crate::Error::Generic);
    }

    for stream in &test_ctx.test_streams {
        if stream.q_recv_nb != stream.q_len
            || stream.r_recv_nb != stream.r_len
            || !stream.q_received
            || !stream.r_received
            || stream.q_rcv != stream.q_src
            || stream.r_rcv != stream.r_src
        {
            return Err(crate::Error::Generic);
        }
    }

    if test_ctx.stream0_sent != test_ctx.stream0_target
        || test_ctx.stream0_sent != test_ctx.stream0_received
    {
        return Err(crate::Error::Generic);
    }

    if test_ctx.qclient.nb_data_nodes_allocated > test_ctx.qclient.nb_data_nodes_in_pool()
        || test_ctx.qserver.nb_data_nodes_allocated > test_ctx.qserver.nb_data_nodes_in_pool()
    {
        return Err(crate::Error::Generic);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// SNI / certificate paths used by the test suite.  Linux/macOS
// layout only; `_WINDOWS` paths are dropped per the v1 scope.

/// Default SNI string for the test fixtures.
pub const TEST_SNI: &str = "test.example.com";

/// SNI that does not match any test certificate (used by cert-verify tests).
pub const TEST_BAD_SNI: &str = "bad.example.com";

/// ALPN token used across the test suite.  C: `PICOQUIC_TEST_ALPN`.
pub const TEST_ALPN: &str = "picoquic-test";

pub const TEST_FILE_SERVER_CERT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/cert.pem");
pub const TEST_FILE_SERVER_BAD_CERT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/badcert.pem");
pub const TEST_FILE_SERVER_KEY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/key.pem");
pub const TEST_FILE_CERT_STORE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/test-ca.crt");
pub const TEST_FILE_SERVER_CERT_ECDSA: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/ecdsa/cert.pem");
pub const TEST_FILE_SERVER_KEY_ECDSA: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/ecdsa/key.pem");
pub const TEST_ECH_PUB_KEY: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/ech/public.pem");
pub const TEST_ECH_PRIVATE_KEY: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/ech/private.pem");
pub const TEST_ECH_CONFIG: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../certs/ech/ech_config.txt"
);
pub const TEST_ECH_CERT: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/ech/ech_cert.pem");
pub const TEST_ECH_RR_REF: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/ech/ech_rr.txt");
pub const TEST_ECH_CONFIG_REF: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../certs/ech/ech_config.txt"
);
pub const TEST_FILE_SERVER_CERT_RSA: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/rsa/cert.pem");
pub const TEST_FILE_SERVER_KEY_RSA: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/rsa/key.pem");
pub const TEST_FILE_SERVER_CERT_ED25519: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../certs/mtls_ed25519/server.crt"
);

// ---------------------------------------------------------------------------
// Single-step simulator.

fn submit_prepared_sim_packets(
    link: &mut TestSimLink,
    send_buffer: &[u8],
    send_length: usize,
    send_msg_size: Option<usize>,
    addr_from: SocketAddr,
    addr_to: SocketAddr,
    ecn_mark: u8,
    simulated_time: Instant,
    loss_mask: &mut Option<&mut u64>,
) -> crate::Result<()> {
    if send_length > send_buffer.len() {
        return Err(crate::Error::Generic);
    }

    let chunk_size = send_msg_size.filter(|s| *s > 0).unwrap_or(send_length);
    let mut offset = 0usize;
    while offset < send_length {
        let next = offset.saturating_add(chunk_size).min(send_length);
        let chunk = &send_buffer[offset..next];
        if chunk.len() > MAX_PACKET_SIZE {
            return Err(crate::Error::Generic);
        }

        let mut pkt = TestSimPacket::create()?;
        pkt.addr_from = Some(addr_from);
        pkt.addr_to = Some(addr_to);
        pkt.ecn_mark = ecn_mark;
        pkt.length = chunk.len();
        pkt.bytes[..chunk.len()].copy_from_slice(chunk);
        sim_link_submit_with_loss(link, pkt, simulated_time, loss_mask);

        offset = next;
    }

    Ok(())
}

/// Advance the simulation by one round, respecting `time_out` as the earliest
/// wake-up.  `was_active` is set to `true` when at least one packet was
/// processed.  C: `tls_api_one_sim_round`.
pub fn tls_api_one_sim_round(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    time_out: Instant,
    was_active: &mut bool,
) -> crate::Result<()> {
    tls_api_one_sim_round_inner(test_ctx, simulated_time, time_out, was_active, None)
}

/// Advance the simulation by one round with the same shared loss-mask
/// semantics used by `tls_api_connection_loop` and `tls_api_data_sending_loop`.
pub fn tls_api_one_sim_round_with_loss(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    time_out: Instant,
    was_active: &mut bool,
    loss_mask: &mut u64,
) -> crate::Result<()> {
    tls_api_one_sim_round_inner(
        test_ctx,
        simulated_time,
        time_out,
        was_active,
        Some(loss_mask),
    )
}

fn sim_link_submit_with_loss(
    link: &mut TestSimLink,
    packet: TestSimPacket,
    current_time: Instant,
    loss_mask: &mut Option<&mut u64>,
) {
    if let Some(mask) = loss_mask.as_deref_mut() {
        link.loss_mask = Some(*mask);
        link.submit(packet, current_time);
        if let Some(link_mask) = link.loss_mask {
            *mask = link_mask;
        }
    } else {
        link.submit(packet, current_time);
    }
}

fn sim_link_admit_pending_with_loss(
    link: &mut TestSimLink,
    current_time: Instant,
    loss_mask: &mut Option<&mut u64>,
) {
    if let Some(mask) = loss_mask.as_deref_mut() {
        link.loss_mask = Some(*mask);
        link.admit_pending(current_time);
        if let Some(link_mask) = link.loss_mask {
            *mask = link_mask;
        }
    } else {
        link.admit_pending(current_time);
    }
}

fn tls_api_prepare_bad_coalesce_packet(cnx: &Connection, bytes: &mut [u8]) -> crate::Result<usize> {
    const BAD_PAYLOAD_LENGTH: usize = 21;

    let path = cnx.paths.first().ok_or(crate::Error::Generic)?;
    let tuple = path.tuples.first().ok_or(crate::Error::Generic)?;
    let remote_cnxid = tuple
        .remote_connection_id_index
        .and_then(|cid_index| {
            cnx.remote_connection_id_stashes
                .iter()
                .find(|stash| stash.unique_path_id == path.unique_path_id)
                .and_then(|stash| stash.connection_ids.get(cid_index))
        })
        .or_else(|| {
            cnx.remote_connection_id_stashes
                .iter()
                .find(|stash| stash.unique_path_id == path.unique_path_id)
                .and_then(|stash| stash.connection_ids.first())
        })
        .map(|remote_cnxid| remote_cnxid.connection_id)
        .ok_or(crate::Error::Generic)?;
    let local_cnxid = tuple
        .local_connection_id
        .and_then(|token| cnx.local_connection_ids.get(token))
        .map(|local_cnxid| local_cnxid.connection_id)
        .ok_or(crate::Error::Generic)?;

    let required = 1 + 4 + 1 + remote_cnxid.len() + 1 + local_cnxid.len() + 1 + BAD_PAYLOAD_LENGTH;
    if bytes.len() < required {
        return Err(crate::Error::BufferTooSmall);
    }

    let mut offset = 0usize;
    bytes[offset] = 0xE0;
    offset += 1;
    bytes[offset..offset + 4].copy_from_slice(&cnx.version_number().to_be_bytes());
    offset += 4;
    bytes[offset] = remote_cnxid.len() as u8;
    offset += 1;
    bytes[offset..offset + remote_cnxid.len()].copy_from_slice(remote_cnxid.as_bytes());
    offset += remote_cnxid.len();
    bytes[offset] = local_cnxid.len() as u8;
    offset += 1;
    bytes[offset..offset + local_cnxid.len()].copy_from_slice(local_cnxid.as_bytes());
    offset += local_cnxid.len();
    bytes[offset] = BAD_PAYLOAD_LENGTH as u8;
    offset += 1;
    public_random(&mut bytes[offset..offset + BAD_PAYLOAD_LENGTH]);
    Ok(offset + BAD_PAYLOAD_LENGTH)
}

fn tls_api_one_sim_round_inner(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    time_out: Instant,
    was_active: &mut bool,
    mut loss_mask: Option<&mut u64>,
) -> crate::Result<()> {
    // Stateless packet takes priority.
    if test_ctx.qserver.pending_stateless_packets.front().is_some() {
        if let Some(sp) = test_ctx.qserver.dequeue_stateless_packet()
            && sp.length > 0
        {
            *was_active = true;
            if let Ok(mut pkt) = TestSimPacket::create() {
                pkt.addr_from = Some(sp.addr_local);
                pkt.addr_to = Some(sp.addr_to);
                pkt.ecn_mark = test_ctx.packet_ecn_default;
                pkt.length = sp.length;
                pkt.bytes[..sp.length].copy_from_slice(&sp.bytes[..sp.length]);
                let use_link2 =
                    test_ctx.s_to_c_link_2.is_some() && pkt.addr_to == Some(test_ctx.client_addr_2);
                if use_link2 {
                    if let Some(l) = test_ctx.s_to_c_link_2.as_mut() {
                        l.submit(pkt, *simulated_time);
                    }
                } else {
                    sim_link_submit_with_loss(
                        &mut test_ctx.s_to_c_link,
                        pkt,
                        *simulated_time,
                        &mut loss_mask,
                    );
                }
            }
        }
        return Ok(());
    }

    // Action enum (local, avoids allocating a string).
    #[derive(Copy, Clone, PartialEq, Eq)]
    enum Act {
        None,
        ClientDep,
        ServerDep,
        ClientArr,
        ServerArr,
        ClientArr2,
        ServerArr2,
        ClientAdm,
        ServerAdm,
        ClientAdm2,
        ServerAdm2,
    }

    let base = simulated_time.ticks().saturating_add(120_000_000u64);
    let mut next_time;
    let mut next_action;

    // Inner loop: drain arrivals/admissions.
    loop {
        next_time = base;
        next_action = Act::None;

        // Client departure
        {
            let v = test_ctx
                .qclient
                .first_cnx_mut()
                .filter(|c| c.connection_state != crate::State::Disconnected)
                .map(|c| c.next_wake_time.ticks());
            if let Some(t) = v
                && t < next_time
            {
                next_time = t;
                next_action = Act::ClientDep;
            }
        }
        // Server departure
        {
            let v = test_ctx
                .active_server_connection()
                .filter(|c| c.connection_state != crate::State::Disconnected)
                .map(|c| c.next_wake_time.ticks());
            if let Some(t) = v
                && t < next_time
            {
                next_time = t;
                next_action = Act::ServerDep;
            }
        }
        // Client arrival (s_to_c)
        {
            let t = test_ctx
                .s_to_c_link
                .next_arrival(Instant::from_ticks(next_time));
            if t < next_time {
                next_time = t;
                next_action = Act::ClientArr;
            }
        }
        {
            let t = test_ctx
                .s_to_c_link
                .next_admission(*simulated_time, Instant::from_ticks(next_time));
            if t < next_time {
                next_time = t;
                next_action = Act::ClientAdm;
            }
        }
        // Server arrival (c_to_s)
        {
            let t = test_ctx
                .c_to_s_link
                .next_arrival(Instant::from_ticks(next_time));
            if t < next_time {
                next_time = t;
                next_action = Act::ServerArr;
            }
        }
        {
            let t = test_ctx
                .c_to_s_link
                .next_admission(*simulated_time, Instant::from_ticks(next_time));
            if t < next_time {
                next_time = t;
                next_action = Act::ServerAdm;
            }
        }
        // Secondary links
        if let Some(link) = test_ctx.s_to_c_link_2.as_mut() {
            let t = link.next_arrival(Instant::from_ticks(next_time));
            if t < next_time {
                next_time = t;
                next_action = Act::ClientArr2;
            }
            let t = link.next_admission(*simulated_time, Instant::from_ticks(next_time));
            if t < next_time {
                next_time = t;
                next_action = Act::ClientAdm2;
            }
        }
        if let Some(link) = test_ctx.c_to_s_link_2.as_mut() {
            let t = link.next_arrival(Instant::from_ticks(next_time));
            if t < next_time {
                next_time = t;
                next_action = Act::ServerArr2;
            }
            let t = link.next_admission(*simulated_time, Instant::from_ticks(next_time));
            if t < next_time {
                next_time = t;
                next_action = Act::ServerAdm2;
            }
        }

        // Handle arrivals/admissions immediately (continue_dequeue).
        match next_action {
            Act::ClientArr => {
                let t = Instant::from_ticks(next_time);
                if let Some(mut pkt) = test_ctx.s_to_c_link.dequeue(t) {
                    let addr_from = pkt.addr_from.unwrap_or(test_ctx.server_addr);
                    let addr_to_ok = pkt
                        .addr_to
                        .map(|a| a == test_ctx.client_addr)
                        .unwrap_or(false);
                    if addr_to_ok && pkt.length > 0 {
                        let addr_to = if test_ctx.client_endpoint.addr_to_unspec {
                            crate::unspecified_socket_addr()
                        } else {
                            pkt.addr_to.unwrap_or(test_ctx.client_addr)
                        };
                        let ecn = pkt.ecn_mark;
                        let _ = test_ctx.qclient.incoming_packet(
                            &mut pkt.bytes[..pkt.length],
                            &addr_from,
                            &addr_to,
                            0,
                            ecn,
                            t,
                        );
                        tls_api_process_received_streams(test_ctx);
                        *was_active = true;
                    }
                }
                continue;
            }
            Act::ServerArr => {
                let t = Instant::from_ticks(next_time);
                if let Some(mut pkt) = test_ctx.c_to_s_link.dequeue(t) {
                    let addr_from = pkt.addr_from.unwrap_or(test_ctx.client_addr);
                    let addr_to = pkt.addr_to.unwrap_or(test_ctx.server_addr);
                    let ecn = pkt.ecn_mark;
                    let _ = test_ctx.qserver.incoming_packet(
                        &mut pkt.bytes[..pkt.length],
                        &addr_from,
                        &addr_to,
                        0,
                        ecn,
                        t,
                    );
                    tls_api_process_received_streams(test_ctx);
                    *was_active = true;
                }
                continue;
            }
            Act::ClientArr2 => {
                let t = Instant::from_ticks(next_time);
                if let Some(link) = test_ctx.s_to_c_link_2.as_mut()
                    && let Some(mut pkt) = link.dequeue(t)
                {
                    let addr_from = pkt.addr_from.unwrap_or(test_ctx.server_addr);
                    let addr_to = if test_ctx.client_endpoint.addr_to_unspec {
                        crate::unspecified_socket_addr()
                    } else {
                        pkt.addr_to.unwrap_or(test_ctx.client_addr_2)
                    };
                    let ecn = pkt.ecn_mark;
                    let _ = test_ctx.qclient.incoming_packet(
                        &mut pkt.bytes[..pkt.length],
                        &addr_from,
                        &addr_to,
                        0,
                        ecn,
                        t,
                    );
                    tls_api_process_received_streams(test_ctx);
                    *was_active = true;
                }
                continue;
            }
            Act::ServerArr2 => {
                let t = Instant::from_ticks(next_time);
                if let Some(link) = test_ctx.c_to_s_link_2.as_mut()
                    && let Some(mut pkt) = link.dequeue(t)
                {
                    let addr_from = pkt.addr_from.unwrap_or(test_ctx.client_addr_2);
                    let addr_to = pkt.addr_to.unwrap_or(test_ctx.server_addr);
                    let ecn = pkt.ecn_mark;
                    let _ = test_ctx.qserver.incoming_packet(
                        &mut pkt.bytes[..pkt.length],
                        &addr_from,
                        &addr_to,
                        0,
                        ecn,
                        t,
                    );
                    tls_api_process_received_streams(test_ctx);
                    *was_active = true;
                }
                continue;
            }
            Act::ClientAdm => {
                sim_link_admit_pending_with_loss(
                    &mut test_ctx.s_to_c_link,
                    Instant::from_ticks(next_time),
                    &mut loss_mask,
                );
                continue;
            }
            Act::ServerAdm => {
                sim_link_admit_pending_with_loss(
                    &mut test_ctx.c_to_s_link,
                    Instant::from_ticks(next_time),
                    &mut loss_mask,
                );
                continue;
            }
            Act::ClientAdm2 => {
                if let Some(l) = test_ctx.s_to_c_link_2.as_mut() {
                    l.admit_pending(Instant::from_ticks(next_time));
                }
                continue;
            }
            Act::ServerAdm2 => {
                if let Some(l) = test_ctx.c_to_s_link_2.as_mut() {
                    l.admit_pending(Instant::from_ticks(next_time));
                }
                continue;
            }
            _ => break,
        }
    }

    // Apply time bounds.
    let timeout = time_out.ticks();
    if timeout > 0 && next_time > timeout {
        *simulated_time = time_out;
        test_ctx.install_simulated_time(*simulated_time);
        return Ok(());
    } else if next_time > simulated_time.ticks() {
        *simulated_time = Instant::from_ticks(next_time);
    }
    test_ctx.install_simulated_time(*simulated_time);

    // Execute departure.
    match next_action {
        Act::ClientDep => {
            let mut buf = vec![0u8; test_ctx.send_buffer_size.max(MAX_PACKET_SIZE)];
            let mut coalesced_length = 0usize;
            let use_udp_gso = test_ctx.use_udp_gso && !test_ctx.do_bad_coalesce_test;
            let prep = {
                let do_bad_coalesce_test = test_ctx.do_bad_coalesce_test;
                if let Some(cnx) = test_ctx.qclient.first_cnx_mut() {
                    if do_bad_coalesce_test && cnx.state() > State::ServerHandshake {
                        coalesced_length = tls_api_prepare_bad_coalesce_packet(cnx, &mut buf)?;
                    }
                    if use_udp_gso {
                        cnx.prepare_packet_ex(*simulated_time, &mut buf[coalesced_length..])
                            .ok()
                    } else {
                        cnx.prepare_packet(*simulated_time, &mut buf[coalesced_length..])
                            .ok()
                    }
                } else {
                    None
                }
            };
            if let Some(pp) = prep
                && pp.send_length > 0
            {
                *was_active = true;
                let t = simulated_time.ticks();
                let in_blackhole = test_ctx.blackhole_start > 0
                    && t >= test_ctx.blackhole_start
                    && t < test_ctx.blackhole_end;
                if !in_blackhole {
                    let addr_from = if pp.addr_from.ip().is_unspecified() {
                        test_ctx.client_addr
                    } else {
                        pp.addr_from
                    };
                    let use_link2 =
                        test_ctx.c_to_s_link_2.is_some() && addr_from == test_ctx.client_addr_2;
                    let is_unreach = if use_link2 {
                        test_ctx
                            .c_to_s_link_2
                            .as_ref()
                            .map(|l| l.is_unreachable)
                            .unwrap_or(false)
                    } else {
                        test_ctx.c_to_s_link.is_unreachable
                    };
                    if !is_unreach {
                        let addr_from = if test_ctx.client_use_nat && !use_link2 {
                            test_ctx.client_addr_natted
                        } else {
                            addr_from
                        };
                        if use_link2 {
                            if let Some(l) = test_ctx.c_to_s_link_2.as_mut() {
                                submit_prepared_sim_packets(
                                    l,
                                    &buf,
                                    pp.send_length,
                                    pp.send_msg_size,
                                    addr_from,
                                    pp.addr_to,
                                    test_ctx.packet_ecn_default,
                                    *simulated_time,
                                    &mut loss_mask,
                                )?;
                            }
                        } else if coalesced_length > 0 {
                            let send_length = coalesced_length + pp.send_length;
                            let mut pkt = TestSimPacket::create()?;
                            pkt.addr_from = Some(addr_from);
                            pkt.addr_to = Some(pp.addr_to);
                            pkt.ecn_mark = test_ctx.packet_ecn_default;
                            pkt.length = send_length;
                            pkt.bytes[..send_length].copy_from_slice(&buf[..send_length]);
                            sim_link_submit_with_loss(
                                &mut test_ctx.c_to_s_link,
                                pkt,
                                *simulated_time,
                                &mut loss_mask,
                            );
                        } else {
                            submit_prepared_sim_packets(
                                &mut test_ctx.c_to_s_link,
                                &buf,
                                pp.send_length,
                                pp.send_msg_size,
                                addr_from,
                                pp.addr_to,
                                test_ctx.packet_ecn_default,
                                *simulated_time,
                                &mut loss_mask,
                            )?;
                        }
                    }
                }
            }
        }
        Act::ServerDep => {
            let mut buf = vec![0u8; test_ctx.send_buffer_size.max(MAX_PACKET_SIZE)];
            let use_udp_gso = test_ctx.use_udp_gso;
            let prep = test_ctx.active_server_connection().and_then(|c| {
                if use_udp_gso {
                    c.prepare_packet_ex(*simulated_time, &mut buf).ok()
                } else {
                    c.prepare_packet(*simulated_time, &mut buf).ok()
                }
            });
            if let Some(pp) = prep
                && pp.send_length > 0
            {
                *was_active = true;
                let t = simulated_time.ticks();
                let in_blackhole = test_ctx.blackhole_start > 0
                    && t >= test_ctx.blackhole_start
                    && t < test_ctx.blackhole_end;
                if !in_blackhole {
                    let addr_from = if pp.addr_from.ip().is_unspecified() {
                        test_ctx.server_addr
                    } else {
                        pp.addr_from
                    };
                    let use_link2 =
                        test_ctx.s_to_c_link_2.is_some() && pp.addr_to == test_ctx.client_addr_2;
                    let is_unreach = if use_link2 {
                        test_ctx
                            .s_to_c_link_2
                            .as_ref()
                            .map(|l| l.is_unreachable)
                            .unwrap_or(false)
                    } else {
                        test_ctx.s_to_c_link.is_unreachable
                    };
                    let mut addr_to = pp.addr_to;
                    let mut simulate_loss = is_unreach;
                    if !use_link2 && !test_ctx.client_use_multiple_addresses {
                        if test_ctx.client_use_nat {
                            if addr_to == test_ctx.client_addr_natted {
                                addr_to = test_ctx.client_addr;
                            } else {
                                simulate_loss = true;
                            }
                        } else if addr_to != test_ctx.client_addr {
                            simulate_loss = true;
                        }
                    }
                    if !simulate_loss {
                        if use_link2 {
                            if let Some(l) = test_ctx.s_to_c_link_2.as_mut() {
                                submit_prepared_sim_packets(
                                    l,
                                    &buf,
                                    pp.send_length,
                                    pp.send_msg_size,
                                    addr_from,
                                    addr_to,
                                    test_ctx.packet_ecn_default,
                                    *simulated_time,
                                    &mut loss_mask,
                                )?;
                            }
                        } else {
                            submit_prepared_sim_packets(
                                &mut test_ctx.s_to_c_link,
                                &buf,
                                pp.send_length,
                                pp.send_msg_size,
                                addr_from,
                                addr_to,
                                test_ctx.packet_ecn_default,
                                *simulated_time,
                                &mut loss_mask,
                            )?;
                        }
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}
/// Create a TLS-API test context with caller-supplied cert/key/root files and
/// SNI.  Used by the cert-verify test suite to exercise different certificate
/// combinations.  C: `cert_verify_set_ctx` in
/// `picoquictest/cert_verify_test.c`.
///
/// The server `Quic` is created with `cert_file` / `key_file` /
/// `root_certs_file` and ALPN `TEST_ALPN`.  The client `Quic` is created
/// with only `root_certs_file`.  A single client connection is opened with
/// the given `sni` and started before the context is returned.
pub fn cert_verify_set_ctx(
    simulated_time: &mut Instant,
    cert_file: Option<&str>,
    key_file: Option<&str>,
    root_certs_file: Option<&str>,
    sni: Option<&str>,
) -> Option<Box<TestTlsApiCtx>> {
    const VERIFIER_ENCRYPT_KEY: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];
    let server_addr = SocketAddr::from(([10u8, 0, 0, 1], 4321u16));

    let mut qclient = Quic::new(
        8,
        None,
        None,
        root_certs_file,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        *simulated_time,
        None,
        None,
    )?;

    let qserver = Quic::new(
        8,
        cert_file,
        key_file,
        root_certs_file,
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        *simulated_time,
        None,
        Some(&VERIFIER_ENCRYPT_KEY),
    )?;

    {
        let cnx = qclient.create_connection(
            ConnectionId::with_size(0)?,
            ConnectionId::with_size(0)?,
            Some(&server_addr),
            *simulated_time,
            Version::InternalTest1 as u32,
            sni,
            Some(TEST_ALPN),
            true,
        )?;
        cnx.start_client().ok()?;
    }

    let c_to_s_link = Box::new(TestSimLink::create(0.01, 10_000, None, 0, *simulated_time).ok()?);
    let s_to_c_link = Box::new(TestSimLink::create(0.01, 10_000, None, 0, *simulated_time).ok()?);

    Some(Box::new(TestTlsApiCtx {
        qclient,
        qserver,
        cnx_server_token: None,
        ignored_server_tokens: Vec::new(),
        c_to_s_link,
        s_to_c_link,
        c_to_s_link_2: None,
        s_to_c_link_2: None,
        client_addr: SocketAddr::from(([0u8; 4], 0u16)),
        server_addr,
        client_addr_2: SocketAddr::from(([0u8; 4], 0u16)),
        client_addr_natted: SocketAddr::from(([0u8; 4], 0u16)),
        client_use_nat: false,
        do_bad_coalesce_test: false,
        client_use_multiple_addresses: false,
        nb_address_observed: 0,
        loss_mask_default: 0,
        blackhole_start: 0,
        blackhole_end: 0,
        send_buffer_size: MAX_PACKET_SIZE,
        use_udp_gso: false,
        client_endpoint: TestClientEndpoint::default(),
        stream0_flow_release: false,
        immediate_exit: false,
        test_finished: false,
        ecn_support: 0,
        packet_ecn_default: 0,
        sum_data_received_at_server: 0,
        client_callback_error_detected: false,
        server_callback_error_detected: false,
        test_streams: Vec::new(),
        stream0_target: 0,
        stream0_sent: 0,
        stream0_received: 0,
        streams_finished: false,
    }))
}

pub const TEST_FILE_SERVER_KEY_ED25519: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../certs/mtls_ed25519/server.key"
);
pub const TEST_FILE_CLIENT_CERT_ED25519: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../certs/mtls_ed25519/client.crt"
);
pub const TEST_FILE_CLIENT_KEY_ED25519: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../certs/mtls_ed25519/client.key"
);
pub const TEST_FILE_CERT_STORE_ED25519: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../certs/mtls_ed25519/ca.crt"
);

// ---------------------------------------------------------------------------
// Composite scenario helpers used by congestion / BDP / blackhole tests.

/// Run a full one-shot scenario on an already-created test context:
/// handshake, scenario init, data loop, completion check.
/// C: `tls_api_one_scenario_body`.
///
/// `_cwin_blocked` and `_proposed_version` are carried for API
/// compatibility; the current implementation ignores them.
pub fn tls_api_one_scenario_body(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    scenario: &[TestApiStreamDesc],
    init_loss_mask: u64,
    _cwin_blocked: i32,
    _proposed_version: u32,
    queue_delay_max: u64,
    max_completion_microsec: u64,
) -> crate::Result<()> {
    let mut loss_mask = init_loss_mask;
    tls_api_connection_loop(test_ctx, &mut loss_mask, queue_delay_max, simulated_time)?;
    wait_client_connection_ready(test_ctx, simulated_time)?;
    test_api_init_send_recv_scenario(test_ctx, scenario)?;
    tls_api_data_sending_loop(test_ctx, &mut loss_mask, simulated_time, 0)?;
    tls_api_one_scenario_body_verify(test_ctx, simulated_time, max_completion_microsec)
}

/// Run only the connection-setup phase (handshake + wait-ready) on an
/// already-created test context, leaving the stream scenario for the
/// caller to drive separately.  C: `tls_api_one_scenario_body_connect`.
pub fn tls_api_one_scenario_body_connect(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    init_loss_mask: u64,
    queue_delay_max: u64,
) -> crate::Result<()> {
    let mut loss_mask = init_loss_mask;
    tls_api_connection_loop(test_ctx, &mut loss_mask, queue_delay_max, simulated_time)?;
    wait_client_connection_ready(test_ctx, simulated_time)
}

const TEST_TICKET_ENCRYPT_KEY: [u8; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];

const TEST_TICKET_BADCRYPT_KEY: [u8; 32] = [
    255, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];

/// Create a TLS-API test context from an explicit initial CID and
/// optional ticket file.  Uses `TEST_SNI` / `TEST_ALPN` internally.
/// C: `tls_api_init_ctx_ex`.
///
/// `proposed_version` is a raw wire version number (`0` = negotiated,
/// `Version::InternalTest1 as u32` = fixed test version).
pub fn tls_api_init_ctx_ex(
    simulated_time: &mut Instant,
    proposed_version: u32,
    ticket_file: Option<&str>,
    initial_cid: Option<&ConnectionId>,
) -> Option<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex_named(
        simulated_time,
        proposed_version,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        ticket_file,
        initial_cid,
        false,
    )
    .ok()
}

fn tls_api_init_ctx_ex_named(
    simulated_time: &mut Instant,
    proposed_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
    ticket_file: Option<&str>,
    initial_cid: Option<&ConnectionId>,
    cid_zero: bool,
) -> crate::Result<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex_named_with_flags(
        simulated_time,
        proposed_version,
        sni,
        alpn,
        ticket_file,
        initial_cid,
        cid_zero,
        false,
        false,
        true,
        false,
        Some(TEST_TICKET_ENCRYPT_KEY.as_slice()),
    )
}

fn tls_api_init_ctx_ex_named_with_start(
    simulated_time: &mut Instant,
    proposed_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
    ticket_file: Option<&str>,
    initial_cid: Option<&ConnectionId>,
    start_client: bool,
) -> Option<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex_named_with_flags(
        simulated_time,
        proposed_version,
        sni,
        alpn,
        ticket_file,
        initial_cid,
        false,
        false,
        false,
        start_client,
        false,
        Some(TEST_TICKET_ENCRYPT_KEY.as_slice()),
    )
    .ok()
}

fn tls_api_init_ctx_ex_named_with_ticket_key(
    simulated_time: &mut Instant,
    proposed_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
    ticket_file: Option<&str>,
    initial_cid: Option<&ConnectionId>,
    ticket_encryption_key: Option<&[u8]>,
) -> Option<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex_named_with_flags(
        simulated_time,
        proposed_version,
        sni,
        alpn,
        ticket_file,
        initial_cid,
        false,
        false,
        false,
        false,
        false,
        ticket_encryption_key,
    )
    .ok()
}

#[allow(clippy::too_many_arguments)]
fn tls_api_init_ctx_ex_named_with_flags(
    simulated_time: &mut Instant,
    proposed_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
    ticket_file: Option<&str>,
    initial_cid: Option<&ConnectionId>,
    cid_zero: bool,
    force_zero_share: bool,
    preserve_zero_version: bool,
    start_client: bool,
    use_ecdsa: bool,
    ticket_encryption_key: Option<&[u8]>,
) -> crate::Result<Box<TestTlsApiCtx>> {
    let version = if proposed_version == 0 && !preserve_zero_version {
        Version::InternalTest1 as u32
    } else {
        proposed_version
    };

    let client_addr = SocketAddr::from(([10u8, 0, 0, 2], 1234u16));
    let server_addr = SocketAddr::from(([10u8, 0, 0, 1], 4321u16));
    let (server_cert, server_key) = if use_ecdsa {
        (TEST_FILE_SERVER_CERT_ECDSA, TEST_FILE_SERVER_KEY_ECDSA)
    } else {
        (TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_KEY)
    };

    let mut qclient = Quic::new(
        8,
        None,
        None,
        Some(TEST_FILE_CERT_STORE),
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        *simulated_time,
        ticket_file,
        None,
    )
    .ok_or_else(|| {
        eprintln!(
            "DBG: qclient Quic::new failed, ticket_file={:?}",
            ticket_file
        );
        crate::Error::Generic
    })?;
    if cid_zero {
        qclient
            .set_default_connection_id_length(0)
            .map_err(|_| crate::Error::Generic)?;
    }

    if force_zero_share {
        qclient.client_zero_share = true;
    }

    let mut qserver = Quic::new(
        8,
        Some(server_cert),
        Some(server_key),
        Some(TEST_FILE_CERT_STORE),
        alpn.or(Some(TEST_ALPN)),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        *simulated_time,
        None,
        ticket_encryption_key,
    )
    .ok_or_else(|| {
        eprintln!(
            "DBG: qserver Quic::new failed, cert={} key={}",
            server_cert, server_key
        );
        crate::Error::Generic
    })?;
    qclient.set_random_initial(0);
    qserver.set_random_initial(0);
    qclient.set_simulated_time(simulated_time.ticks());
    qserver.set_simulated_time(simulated_time.ticks());

    {
        let icid = initial_cid
            .copied()
            .unwrap_or_else(|| ConnectionId::with_size(0).unwrap());
        let cnx = qclient
            .create_connection(
                icid,
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                Some(&server_addr),
                *simulated_time,
                version,
                sni,
                alpn,
                true,
            )
            .ok_or_else(|| {
                eprintln!(
                    "DBG: qclient create_connection failed, version={:#x}",
                    version
                );
                crate::Error::Generic
            })?;
        if start_client {
            cnx.start_client()?;
        }
    }

    let c_to_s_link = Box::new(TestSimLink::create(0.01, 10_000, None, 0, *simulated_time)?);
    let s_to_c_link = Box::new(TestSimLink::create(0.01, 10_000, None, 0, *simulated_time)?);

    Ok(Box::new(TestTlsApiCtx {
        qclient,
        qserver,
        cnx_server_token: None,
        ignored_server_tokens: Vec::new(),
        c_to_s_link,
        s_to_c_link,
        c_to_s_link_2: None,
        s_to_c_link_2: None,
        client_addr,
        server_addr,
        client_addr_2: SocketAddr::from(([10u8, 0, 0, 3], 1234u16)),
        client_addr_natted: SocketAddr::from(([0u8; 4], 0u16)),
        client_use_nat: false,
        do_bad_coalesce_test: false,
        client_use_multiple_addresses: false,
        nb_address_observed: 0,
        loss_mask_default: 0,
        blackhole_start: 0,
        blackhole_end: 0,
        send_buffer_size: MAX_PACKET_SIZE,
        use_udp_gso: false,
        client_endpoint: TestClientEndpoint::default(),
        stream0_flow_release: false,
        immediate_exit: false,
        test_finished: false,
        ecn_support: 0,
        packet_ecn_default: 0,
        sum_data_received_at_server: 0,
        client_callback_error_detected: false,
        server_callback_error_detected: false,
        test_streams: Vec::new(),
        stream0_target: 0,
        stream0_sent: 0,
        stream0_received: 0,
        streams_finished: false,
    }))
}

#[allow(dead_code)]
fn tls_api_init_ctx_delayed(
    simulated_time: &mut Instant,
    proposed_version: u32,
    ticket_file: Option<&str>,
) -> Option<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex_named_with_start(
        simulated_time,
        proposed_version,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        ticket_file,
        None,
        false,
    )
}

pub fn tls_api_init_ctx_ex2_delayed(
    simulated_time: &mut Instant,
    proposed_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
    ticket_file: Option<&str>,
    initial_cid: Option<&ConnectionId>,
) -> Option<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex_named_with_start(
        simulated_time,
        proposed_version,
        sni.or(Some(TEST_SNI)),
        alpn.or(Some(TEST_ALPN)),
        ticket_file,
        initial_cid,
        false,
    )
}

/// Read a QLOG file and return the highest `bytes_in_flight` value
/// seen across all logged events.  C: `picoquic_check_bytes_in_flight`
/// (defined inline in `picoquictest/congestion_test.c`).
pub fn check_bytes_in_flight(qlog_file: &str) -> crate::Result<u64> {
    use std::io::BufRead as _;
    let f = std::fs::File::open(qlog_file).map_err(|_| crate::Error::Generic)?;
    let reader = std::io::BufReader::new(f);
    let needle = "\"bytes_in_flight\":";
    let mut max_bif = 0u64;
    for line in reader.lines() {
        let line = line.map_err(|_| crate::Error::Generic)?;
        if let Some(idx) = line.find(needle) {
            let tail = &line[idx + needle.len()..];
            let start = tail
                .find(|c: char| c.is_ascii_digit())
                .unwrap_or(tail.len());
            let digits = &tail[start..];
            let end = digits
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(digits.len());
            if end > 0
                && let Ok(v) = digits[..end].parse::<u64>()
            {
                max_bif = max_bif.max(v);
            }
        }
    }
    Ok(max_bif)
}

/// Write an empty session-ticket store to `filename`.  Equivalent to
/// `picoquic_save_tickets(NULL, simulated_time, filename)` — used to
/// initialise a clean ticket file before a two-pass BDP test.
pub fn save_empty_tickets(filename: &str, _simulated_time: Instant) -> crate::Result<()> {
    std::fs::File::create(filename)
        .map(|_| ())
        .map_err(|_| crate::Error::Generic)
}

// ---------------------------------------------------------------------------
// Additional simulation helpers used by Phase 3A test bodies.

/// Wait until the session ticket has been received by the client.
/// C: `session_resume_wait_for_ticket`.
pub fn session_resume_wait_for_ticket(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + 4_000_000);
    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while simulated_time.ticks() < time_out.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
        && test_ctx.qclient.stored_tickets.is_empty()
        && nb_trials < 1024
        && nb_inactive < 64
    {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round(test_ctx, simulated_time, time_out, &mut was_active)?;
        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }
    Ok(())
}

/// Close both connections, injecting the given 64-bit loss mask.
/// C: `tls_api_close_with_losses`.
pub fn tls_api_close_with_losses(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    losses: u64,
) -> crate::Result<()> {
    let _ = test_ctx.qclient.first_cnx_mut().map(|c| c.close(0));

    test_ctx.c_to_s_link.loss_mask = Some(losses);
    test_ctx.s_to_c_link.loss_mask = Some(losses);

    let mut nb_rounds = 0;
    while nb_rounds < 100_000 {
        let client_disc = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|c| c.connection_state == crate::State::Disconnected)
            .unwrap_or(true);
        let server_disc = test_ctx.active_server_disconnected();
        if client_disc && server_disc {
            break;
        }

        let mut was_active = false;
        tls_api_one_sim_round(
            test_ctx,
            simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        nb_rounds += 1;
    }

    let client_disc = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|c| c.connection_state == crate::State::Disconnected)
        .unwrap_or(true);
    let server_disc = test_ctx.active_server_disconnected();
    if client_disc && server_disc {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Advance the simulation until `timeout` (µs) without doing anything.
/// C: `tls_api_wait_for_timeout`.
pub fn tls_api_wait_for_timeout(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    timeout: u64,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + timeout);
    let mut nb_inactive = 0;

    while simulated_time.ticks() < time_out.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
        && nb_inactive < 64
    {
        let mut was_active = false;
        tls_api_one_sim_round(test_ctx, simulated_time, time_out, &mut was_active)?;
        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }
    Ok(())
}

/// Spin until neither endpoint has pending data, or until `max_rounds`
/// iterations.  C: `tls_api_synch_to_empty_loop`.
pub fn tls_api_synch_to_empty_loop(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    max_rounds: i32,
    flag1: i32,
    flag2: i32,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + 4_000_000);
    let mut nb_rounds = 0;
    let path_target = flag1 as usize;

    while simulated_time.ticks() < time_out.ticks()
        && nb_rounds < max_rounds
        && test_ctx
            .qclient
            .first_cnx_mut()
            .map(|c| c.connection_state != crate::State::Disconnected)
            .unwrap_or(false)
    {
        let mut was_active = false;
        tls_api_one_sim_round(test_ctx, simulated_time, time_out, &mut was_active)?;
        nb_rounds += 1;

        if !test_ctx.has_cnx_server() {
            break;
        }

        let client_paths = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|c| c.nb_paths())
            .unwrap_or(0);
        let server_paths = test_ctx
            .qserver
            .first_cnx_mut()
            .map(|c| c.nb_paths())
            .unwrap_or(0);
        let client_empty = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|c| c.is_backlog_empty())
            .unwrap_or(true);
        let server_empty = test_ctx
            .qserver
            .first_cnx_mut()
            .map(|c| c.is_backlog_empty())
            .unwrap_or(true);
        let client_ready = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|c| c.connection_state == crate::State::Ready)
            .unwrap_or(false);
        let server_ready = test_ctx
            .qserver
            .first_cnx_mut()
            .map(|c| c.connection_state == crate::State::Ready)
            .unwrap_or(false);

        if client_paths >= path_target
            && server_paths >= path_target
            && client_empty
            && server_empty
            && (flag2 == 0 || (client_ready && server_ready))
        {
            break;
        }
    }
    Ok(())
}

/// Initialise a TLS-API test context without an explicit initial CID.
/// C: `tls_api_init_ctx` (the non-`_ex` variant).
pub fn tls_api_init_ctx(
    simulated_time: &mut Instant,
    proposed_version: u32,
    ticket_file: Option<&str>,
) -> Option<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex(simulated_time, proposed_version, ticket_file, None)
}

/// Initialise a TLS-API context with the client zero-share flag set before
/// the client connection is created and started.
/// C: `tls_api_init_ctx(..., force_zero_share=1, delayed_init=0)`.
pub fn tls_api_init_ctx_zero_share(simulated_time: &mut Instant) -> Option<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex_named_with_flags(
        simulated_time,
        0,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        None,
        false,
        true,
        true,
        true,
        false,
        Some(TEST_TICKET_ENCRYPT_KEY.as_slice()),
    )
    .ok()
}

/// Re-queue the initial data queries on a recycled test connection.
/// C: `test_api_queue_initial_queries`.
pub fn test_api_queue_initial_queries(
    test_ctx: &mut TestTlsApiCtx,
    initial_data_stream_id: u64,
) -> crate::Result<()> {
    let mut more_stream = false;

    for i in 0..test_ctx.test_streams.len() {
        if test_ctx.test_streams[i].previous_stream_id != initial_data_stream_id {
            continue;
        }

        let stream_id = test_ctx.test_streams[i].stream_id;
        let q_len = test_ctx.test_streams[i]
            .q_len
            .min(test_ctx.test_streams[i].q_src.len());
        let data = test_ctx.test_streams[i].q_src[..q_len].to_vec();
        debug_assert_eq!(test_ctx.test_streams[i].q_rcv.len(), q_len);
        if crate::stream::StreamId(stream_id).is_client() {
            test_ctx
                .cnx_client()
                .add_to_stream(stream_id, &data, true)?;
        } else {
            test_ctx
                .cnx_server()
                .add_to_stream(stream_id, &data, true)?;
        }
        test_ctx.test_streams[i].q_sent = true;
        more_stream = true;
    }

    if test_ctx.stream0_target > 0 {
        test_ctx.cnx_client().mark_active_stream(0, true, None)?;
    }

    if !more_stream {
        more_stream = test_ctx.test_streams.iter().any(|s| !s.response_complete());
    }

    if more_stream {
        test_ctx.test_finished = false;
        test_ctx.streams_finished = false;
    } else {
        test_ctx.streams_finished = true;
        test_ctx.test_finished = test_ctx.stream0_received >= test_ctx.stream0_target;
    }

    Ok(())
}

fn test_api_handle_stream_event(test_ctx: &mut TestTlsApiCtx, event: TestApiStreamEvent) {
    if !event.client_mode {
        test_ctx.sum_data_received_at_server = test_ctx
            .sum_data_received_at_server
            .saturating_add(event.bytes.len());
    }

    if event.stream_id == 0 && !event.client_mode {
        if event.bytes.iter().any(|b| *b != 0xa5) {
            set_test_api_callback_error(test_ctx, event.client_mode);
            return;
        }
        test_ctx.stream0_received = test_ctx.stream0_received.saturating_add(event.bytes.len());
        if test_ctx.streams_finished && test_ctx.stream0_received >= test_ctx.stream0_target {
            test_ctx.test_finished = true;
        }
        return;
    }

    let Some(stream_index) = test_ctx
        .test_streams
        .iter()
        .position(|stream| stream.stream_id == event.stream_id)
    else {
        set_test_api_callback_error(test_ctx, event.client_mode);
        return;
    };

    let is_client_stream = crate::stream::StreamId(event.stream_id).is_client();
    let mut stream_finished = false;
    let mut queue_response_on_client = false;
    let mut response = Vec::new();
    let callback_ok;

    {
        let stream = &mut test_ctx.test_streams[stream_index];

        if is_client_stream {
            if event.client_mode {
                callback_ok = test_api_receive_stream_data(stream, true, &event.bytes, event.fin);
                stream_finished = event.fin;
            } else {
                callback_ok = test_api_receive_stream_data(stream, false, &event.bytes, event.fin);
                if event.fin && callback_ok {
                    if stream.r_len == 0 {
                        stream.r_received = true;
                        stream_finished = true;
                    } else {
                        response = stream.r_src.clone();
                    }
                }
            }
        } else if event.client_mode {
            callback_ok = test_api_receive_stream_data(stream, false, &event.bytes, event.fin);
            if event.fin && callback_ok {
                if stream.r_len == 0 {
                    stream.r_received = true;
                    stream_finished = true;
                } else {
                    queue_response_on_client = true;
                    response = stream.r_src.clone();
                }
            }
        } else {
            callback_ok = test_api_receive_stream_data(stream, true, &event.bytes, event.fin);
            stream_finished = event.fin;
        }
    }

    if !callback_ok {
        set_test_api_callback_error(test_ctx, event.client_mode);
        return;
    }

    if !response.is_empty() {
        let add_result = if queue_response_on_client {
            test_ctx
                .cnx_client()
                .add_to_stream(event.stream_id, &response, true)
        } else {
            test_ctx
                .cnx_server()
                .add_to_stream(event.stream_id, &response, true)
        };
        if add_result.is_err() {
            set_test_api_callback_error(test_ctx, event.client_mode);
            return;
        }
    }

    if stream_finished && test_api_queue_initial_queries(test_ctx, event.stream_id).is_err() {
        set_test_api_callback_error(test_ctx, event.client_mode);
    }
}

fn tls_api_process_received_streams(test_ctx: &mut TestTlsApiCtx) {
    let mut events = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| collect_received_stream_events(cnx, true))
        .unwrap_or_default();
    if let Some(cnx) = test_ctx.qserver.first_cnx_mut() {
        events.extend(collect_received_stream_events(cnx, false));
    }

    for event in events {
        test_api_handle_stream_event(test_ctx, event);
    }
}

/// Create a TLS-API test context with an explicit SNI, ALPN, and
/// additional flag parameters.  This Rust helper exposes the subset
/// exercised by the translated tests: version, SNI, ALPN, ticket file,
/// and initial CID.
/// C: `tls_api_init_ctx_ex2`.
#[allow(clippy::too_many_arguments)]
pub fn tls_api_init_ctx_ex2(
    simulated_time: &mut Instant,
    proposed_version: u32,
    _sni: Option<&str>,
    _alpn: Option<&str>,
    ticket_file: Option<&str>,
    initial_cid: Option<&ConnectionId>,
) -> Option<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex_named(
        simulated_time,
        proposed_version,
        _sni.or(Some(TEST_SNI)),
        _alpn.or(Some(TEST_ALPN)),
        ticket_file,
        initial_cid,
        false,
    )
    .ok()
}

/// Create a TLS-API test context using the ECDSA server certificate/key pair.
/// This maps the final `use_ecdsa` argument of C `tls_api_init_ctx_ex2`.
pub fn tls_api_init_ctx_ex2_ecdsa(
    simulated_time: &mut Instant,
    proposed_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
    ticket_file: Option<&str>,
    initial_cid: Option<&ConnectionId>,
) -> Option<Box<TestTlsApiCtx>> {
    tls_api_init_ctx_ex_named_with_flags(
        simulated_time,
        proposed_version,
        sni.or(Some(TEST_SNI)),
        alpn.or(Some(TEST_ALPN)),
        ticket_file,
        initial_cid,
        false,
        false,
        false,
        true,
        true,
        Some(TEST_TICKET_ENCRYPT_KEY.as_slice()),
    )
    .ok()
}

/// One segment in a time-varying link scenario.
/// C: `test_vary_link_spec_t` in `picoquictest/picoquictest_internal.h`.
#[derive(Debug, Clone, Copy)]
pub struct VaryLinkSpec {
    /// Duration of this link-state segment in microseconds.  C: `duration`.
    pub duration: u64,
    /// Uplink bandwidth (bits per second).  C: `bits_per_second_up`.
    pub bits_per_second_up: u64,
    /// Downlink bandwidth (bits per second).  C: `bits_per_second_down`.
    pub bits_per_second_down: u64,
    /// One-way latency (microseconds).  C: `microsec_latency`.
    pub microsec_latency: u64,
}

/// Apply one time-varying link segment and return the next transition time.
/// C: `test_vary_link`.
fn test_vary_link(
    test_ctx: &mut TestTlsApiCtx,
    transition_time: u64,
    link_state: &VaryLinkSpec,
) -> u64 {
    const TEN_TWELVE: u64 = 1_000_000_000_000;
    let picosec_per_byte_up = (TEN_TWELVE * 8) / link_state.bits_per_second_up;
    let picosec_per_byte_down = (TEN_TWELVE * 8) / link_state.bits_per_second_down;

    test_ctx.c_to_s_link.microsec_latency = link_state.microsec_latency;
    test_ctx.c_to_s_link.picosec_per_byte = picosec_per_byte_up;
    test_ctx.s_to_c_link.microsec_latency = link_state.microsec_latency;
    test_ctx.s_to_c_link.picosec_per_byte = picosec_per_byte_down;

    transition_time + link_state.duration
}

/// Drive data delivery with optional time-varying link states.
/// C: `tls_api_data_sending_loop_ex`.
pub fn tls_api_data_sending_loop_ex(
    test_ctx: &mut TestTlsApiCtx,
    loss_mask: &mut u64,
    simulated_time: &mut Instant,
    max_trials: i32,
    link_states: &[VaryLinkSpec],
) -> crate::Result<()> {
    test_ctx.c_to_s_link.loss_mask = Some(*loss_mask);
    test_ctx.s_to_c_link.loss_mask = Some(*loss_mask);

    let max = if max_trials <= 0 {
        4_000_000
    } else {
        max_trials
    };
    let mut nb_trials = 0;
    let mut nb_inactive = 0;
    let mut next_state_change = 0;
    let mut next_link_state = 0;

    if let Some(first_link_state) = link_states.first() {
        next_state_change = test_vary_link(test_ctx, simulated_time.ticks(), first_link_state);
    }

    while nb_trials < max && nb_inactive < 256 && test_ctx.client_ready() && test_ctx.server_ready()
    {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round(
            test_ctx,
            simulated_time,
            Instant::from_ticks(next_state_change),
            &mut was_active,
        )?;
        if !link_states.is_empty() && simulated_time.ticks() >= next_state_change {
            next_link_state += 1;
            if next_link_state >= link_states.len() {
                next_link_state = 0;
            }
            next_state_change = test_vary_link(
                test_ctx,
                simulated_time.ticks(),
                &link_states[next_link_state],
            );
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if test_ctx.test_finished {
            let client_empty = test_ctx
                .qclient
                .first_cnx_mut()
                .map(|c| c.is_backlog_empty())
                .unwrap_or(true);
            let server_empty = test_ctx
                .qserver
                .first_cnx_mut()
                .map(|c| c.is_backlog_empty())
                .unwrap_or(true);
            if test_ctx.immediate_exit || (client_empty && server_empty) {
                break;
            }
        }
    }
    Ok(())
}

/// Run a full scenario with optional time-varying link states.
/// Extends [`tls_api_one_scenario_body`] with a link-state list for
/// bandwidth/latency variation during the test.
/// C: `tls_api_one_scenario_body_ex`.
#[allow(clippy::too_many_arguments)]
pub fn tls_api_one_scenario_body_ex(
    _test_ctx: &mut TestTlsApiCtx,
    _simulated_time: &mut Instant,
    _scenario: &[TestApiStreamDesc],
    stream0_target: usize,
    _init_loss_mask: u64,
    max_data: u64,
    _queue_delay_max: u64,
    _max_completion_microsec: u64,
    _link_states: &[VaryLinkSpec],
) -> crate::Result<()> {
    tls_api_one_scenario_body_connect(_test_ctx, _simulated_time, 0, _queue_delay_max)?;
    if max_data != 0 {
        if !_test_ctx.has_cnx_server() {
            return Err(crate::Error::Generic);
        }
        let client = _test_ctx.cnx_client();
        client.maxdata_local = max_data;
        client.maxdata_remote = max_data;
        let server = _test_ctx.cnx_server();
        server.maxdata_local = max_data;
        server.maxdata_remote = max_data;
    }
    _test_ctx.loss_mask_default = _init_loss_mask;
    _test_ctx.stream0_target = stream0_target;
    _test_ctx.stream0_sent = 0;
    _test_ctx.stream0_received = 0;
    test_api_init_send_recv_scenario(_test_ctx, _scenario)?;
    let mut loss_mask = _init_loss_mask;
    tls_api_data_sending_loop_ex(_test_ctx, &mut loss_mask, _simulated_time, 0, _link_states)?;
    tls_api_one_scenario_body_verify(_test_ctx, _simulated_time, _max_completion_microsec)
}

/// Compare two text files byte-for-byte; return `Err` if they differ.
/// C: `picoquic_test_compare_text_files`.
pub fn compare_text_files(file1: &str, file2: &str) -> crate::Result<()> {
    fn read_text(path: &str) -> crate::Result<String> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(s),
            Err(_) => {
                let fallback = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(path);
                std::fs::read_to_string(fallback).map_err(|_| crate::Error::Generic)
            }
        }
    }

    let c1 = read_text(file1)?;
    let c2 = read_text(file2)?;
    if c1 == c2 {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Create a minimal QUIC context + connection pair for unit testing,
/// seeding `simulated_time`.
/// C: `picoquic_test_set_minimal_cnx_with_time`.
///
/// Returns the quic context; the connection is owned by and reachable
/// through the context.
pub fn test_set_minimal_cnx_with_time(_simulated_time: &mut Instant) -> crate::Result<Box<Quic>> {
    let mut quic = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        *_simulated_time,
        None,
        None,
    )
    .ok_or_else(|| {
        eprintln!("cnx_ddos: qddos context creation failed");
        crate::Error::Generic
    })?;

    quic.create_connection(
        ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
        ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
        None,
        *_simulated_time,
        0,
        Some(TEST_SNI),
        Some("minimal"),
        true,
    )
    .ok_or_else(|| {
        eprintln!("cnx_ddos: test context creation failed");
        crate::Error::Generic
    })?;

    Ok(quic)
}

// ---------------------------------------------------------------------------
// Zero-RTT test helpers.

/// Parameters for a zero-RTT connection test.
/// C: `zero_rtt_test_t` in `picoquictest/picoquictest_internal.h`.
#[derive(Default)]
pub struct ZeroRttTest {
    /// Use wrong crypto to force a 0-RTT rejection.  C: `use_badcrypt`.
    pub use_badcrypt: bool,
    /// Trigger a hard reset (retry) during the 0-RTT handshake.  C: `hardreset`.
    pub hardreset: bool,
    /// Bitmask of packets to drop during the early handshake.  C: `early_loss`.
    pub early_loss: u64,
    /// Disable coalescing of 0-RTT and Initial packets.  C: `no_coal`.
    pub no_coal: bool,
    /// Use a larger payload to stress 0-RTT flow control.  C: `long_data`.
    pub long_data: bool,
    /// Delay (µs) to insert before the handshake begins.  C: `extra_delay`.
    pub extra_delay: u64,
    /// Enable multipath transport parameter.  C: `do_multipath`.
    pub do_multipath: bool,
    /// Propose ECH in the 0-RTT handshake.  C: `propose_ech`.
    pub propose_ech: bool,
    /// Change transport parameters between the first and resumed connection.
    /// C: `change_params`.
    pub change_params: bool,
}

/// Run a zero-RTT connection scenario.
/// C: `zero_rtt_test_one` in `picoquictest/tls_api.c`.
pub fn zero_rtt_test_one(_zrt: &ZeroRttTest) -> crate::Result<()> {
    const TICKET_FILE_NAME: &str = "resume_tests_tickets.bin";

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let proposed_version = 0u32;

    save_empty_tickets(TICKET_FILE_NAME, simulated_time)?;

    for pass in 0..2usize {
        if pass == 1 {
            simulated_time =
                Instant::from_ticks(simulated_time.ticks().saturating_add(_zrt.extra_delay));
        }

        let ticket_encryption_key = if pass == 1 && _zrt.use_badcrypt {
            TEST_TICKET_BADCRYPT_KEY.as_slice()
        } else {
            TEST_TICKET_ENCRYPT_KEY.as_slice()
        };
        let mut test_ctx = tls_api_init_ctx_ex_named_with_ticket_key(
            &mut simulated_time,
            proposed_version,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            Some(TICKET_FILE_NAME),
            None,
            Some(ticket_encryption_key),
        )
        .ok_or(crate::Error::Generic)?;

        if _zrt.no_coal {
            test_ctx.qserver.dont_coalesce_init = true;
        }
        if _zrt.hardreset && pass == 1 {
            test_ctx.qserver.set_cookie_mode(1);
        }
        if _zrt.do_multipath {
            let server_parameters = multipath_init_params(false);
            test_ctx.qserver.set_default_tp(&server_parameters)?;
            let cnx = test_ctx.cnx_client();
            cnx.local_parameters.initial_max_path_id = 3;
            cnx.local_parameters.enable_time_stamp = 0;
        }
        if pass > 0 && _zrt.change_params {
            test_ctx.qserver.default_tp.initial_max_data = test_ctx
                .qserver
                .default_tp
                .initial_max_data
                .saturating_sub(1);
            test_ctx.qserver.default_tp.initial_max_stream_id_bidir = test_ctx
                .qserver
                .default_tp
                .initial_max_stream_id_bidir
                .saturating_add(1);
        }
        if _zrt.propose_ech {
            test_ctx.qclient.ech_configure(None, None)?;
            if !test_ctx.qclient.ech_client_enabled || test_ctx.qclient.client_zero_share {
                return Err(crate::Error::Generic);
            }
        }

        test_ctx.cnx_client().start_client()?;

        if pass == 1 {
            test_ctx.c_to_s_link.microsec_latency = 50_000;
            test_ctx.s_to_c_link.microsec_latency = 50_000;

            let zero_rtt_packets = if _zrt.long_data { 17 } else { 1 };
            for x in 0..zero_rtt_packets {
                let stream_id = if _zrt.long_data {
                    4u64 * x as u64 + 4
                } else {
                    0
                };
                let payload = if _zrt.long_data {
                    vec![x as u8; 256]
                } else {
                    b"test0rtt".to_vec()
                };
                test_ctx
                    .cnx_client()
                    .add_to_stream(stream_id, &payload, true)?;
            }

            if _zrt.early_loss > 0 {
                loss_mask = _zrt.early_loss;
            }
        }

        tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

        if !_zrt.use_badcrypt && !_zrt.hardreset && !_zrt.change_params {
            let rtt_is_available = test_ctx.cnx_client().is_0rtt_available();
            if (rtt_is_available && pass == 0) || (!rtt_is_available && pass != 0) {
                return Err(crate::Error::Generic);
            }
        }

        if pass == 1 {
            if !_zrt.use_badcrypt && !_zrt.hardreset && !_zrt.change_params {
                let client_psk = test_ctx.cnx_client().tls_is_psk_handshake();
                let server_psk = test_ctx
                    .qserver
                    .first_cnx_mut()
                    .map(|c| c.tls_is_psk_handshake())
                    .unwrap_or(true);
                if !client_psk || !server_psk {
                    return Err(crate::Error::Generic);
                }
            }
            tls_api_synch_to_empty_loop(&mut test_ctx, &mut simulated_time, 2048, 0, 0)?;
        }

        if pass == 1 && _zrt.do_multipath {
            let server_mp = test_ctx
                .qserver
                .first_cnx_mut()
                .map(|c| c.is_multipath_enabled)
                .unwrap_or(false);
            if !test_ctx.cnx_client().is_multipath_enabled || !server_mp {
                return Err(crate::Error::Generic);
            }
        }

        if pass == 0 {
            session_resume_wait_for_ticket(&mut test_ctx, &mut simulated_time)?;
        } else {
            tls_api_synch_to_empty_loop(&mut test_ctx, &mut simulated_time, 2048, 0, 1)?;
        }

        tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;

        if pass == 1 {
            let server_received_stream_data = test_ctx.server_received_stream_data();
            let server_zero_rtt_received = test_ctx
                .qserver
                .first_cnx_mut()
                .map(|cnx| cnx.nb_zero_rtt_received);
            let cnx = test_ctx.cnx_client();
            if !_zrt.use_badcrypt && !_zrt.hardreset && !_zrt.change_params {
                if cnx.nb_zero_rtt_sent == 0 {
                    return Err(crate::Error::Generic);
                }
                if _zrt.early_loss == 0 && cnx.nb_zero_rtt_acked != cnx.nb_zero_rtt_sent {
                    return Err(crate::Error::Generic);
                }
                if _zrt.early_loss == 0
                    && _zrt.no_coal
                    && server_zero_rtt_received
                        .is_some_and(|received| cnx.nb_zero_rtt_sent != received)
                {
                    return Err(crate::Error::Generic);
                }
                if _zrt.long_data && cnx.nb_zero_rtt_sent < 3 {
                    return Err(crate::Error::Generic);
                }
            } else if cnx.nb_zero_rtt_sent == 0
                || ((_zrt.early_loss > 0 || _zrt.change_params) && cnx.nb_zero_rtt_acked != 0)
                || !server_received_stream_data
                || cnx.did_receive_short_initial
            {
                return Err(crate::Error::Generic);
            }
        }

        if test_ctx.qclient.stored_tickets.is_empty() {
            return Err(crate::Error::Generic);
        }
        test_ctx
            .qclient
            .save_tickets(simulated_time, TICKET_FILE_NAME)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Rate-control (leaky bucket) helpers for pacing tests.

struct RctlState {
    bucket_increase_per_microsec: f64,
    bucket_max: u64,
    bucket_current: f64,
    bucket_arrival_last: u64,
}

impl TestAqm for RctlState {
    fn submit(&mut self, link: &mut TestSimLink, packet: TestSimPacket, current_time: Instant) {
        let should_drop = if self.bucket_increase_per_microsec > 0.0 {
            let delta = current_time
                .ticks()
                .saturating_sub(self.bucket_arrival_last) as f64;
            self.bucket_arrival_last = current_time.ticks();
            self.bucket_current += delta * self.bucket_increase_per_microsec;
            if self.bucket_current > self.bucket_max as f64 {
                self.bucket_current = self.bucket_max as f64;
            }
            if self.bucket_current > packet.length as f64 {
                self.bucket_current -= packet.length as f64;
                false
            } else {
                true
            }
        } else {
            false
        };
        link.enqueue(packet, current_time, should_drop);
    }

    fn reset(&mut self, _link: &mut TestSimLink, current_time: Instant) {
        self.bucket_arrival_last = current_time.ticks();
        self.bucket_current = self.bucket_max as f64;
    }

    fn release(&mut self, _link: &mut TestSimLink) {}

    fn has_pending(&mut self) -> bool {
        false
    }

    fn admit_pending(&mut self, _link: &mut TestSimLink, _current_time: Instant) {}

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Install a leaky-bucket rate controller on `link`.
/// C: `rctl_configure` in `picoquictest/picoquictest_rctl.h`.
pub fn rctl_configure(
    _link: &mut TestSimLink,
    _bucket_increase_per_microsec: f64,
    _bucket_max: u64,
    _current_time: Instant,
) -> crate::Result<()> {
    _link.aqm_state = Some(Box::new(RctlState {
        bucket_increase_per_microsec: _bucket_increase_per_microsec,
        bucket_max: _bucket_max,
        bucket_current: _bucket_max as f64,
        bucket_arrival_last: _current_time.ticks(),
    }));
    Ok(())
}

struct RedAqmState {
    red_threshold: u64,
    red_queue_max: u64,
    red_average_queue: u64,
    drop_total: f64,
}

impl RedAqmState {
    fn recur(&mut self, drop_rate: f64) -> bool {
        self.drop_total += drop_rate;
        if self.drop_total > 1.0 {
            self.drop_total -= 1.0;
            true
        } else {
            false
        }
    }

    fn drop_rate(&self, queue_delay: u64) -> f64 {
        if queue_delay < self.red_threshold {
            0.0
        } else if queue_delay >= self.red_queue_max {
            1.0
        } else {
            (queue_delay - self.red_threshold) as f64
                / (self.red_queue_max - self.red_threshold) as f64
        }
    }
}

impl TestAqm for RedAqmState {
    fn submit(&mut self, link: &mut TestSimLink, packet: TestSimPacket, current_time: Instant) {
        let queue_delay = link.queue_delay(current_time);
        self.red_average_queue = (3 * self.red_average_queue + queue_delay) / 4;
        let drop_rate = self.drop_rate(self.red_average_queue);
        let should_drop = self.recur(drop_rate);
        link.enqueue(packet, current_time, should_drop);
    }

    fn reset(&mut self, _link: &mut TestSimLink, _current_time: Instant) {}

    fn release(&mut self, _link: &mut TestSimLink) {}

    fn has_pending(&mut self) -> bool {
        false
    }

    fn admit_pending(&mut self, _link: &mut TestSimLink, _current_time: Instant) {}

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn red_aqm_configure(
    link: &mut TestSimLink,
    red_threshold: u64,
    red_queue_max: u64,
) -> crate::Result<()> {
    link.aqm_state = Some(Box::new(RedAqmState {
        red_threshold,
        red_queue_max,
        red_average_queue: 0,
        drop_total: 0.0,
    }));
    Ok(())
}

// ---------------------------------------------------------------------------
// Multipath datagram context.

/// Per-test state for multipath datagram send/receive callbacks.
/// C: `test_datagram_send_recv_ctx_t` (multipath-relevant fields).
#[derive(Default)]
pub struct TestDatagramCtx {
    /// Maximum datagram payload size.  C: `dg_max_size`.
    pub dg_max_size: usize,
    /// Number of datagrams to send in each direction [client, server].
    /// C: `dg_target`.
    pub dg_target: [u64; 2],
    /// Number of datagrams sent.  C: `dg_sent`.
    pub dg_sent: [u64; 2],
    /// Number of datagrams received.  C: `dg_recv`.
    pub dg_recv: [u64; 2],
    /// Number of acknowledged datagrams.  C: `dg_acked`.
    pub dg_acked: [u64; 2],
    /// Number of lost datagrams.  C: `dg_nacked`.
    pub dg_nacked: [u64; 2],
    /// Number of spuriously lost datagrams.  C: `dg_spurious`.
    pub dg_spurious: [u64; 2],
    /// Interval between consecutive datagrams (µs).  C: `send_delay`.
    pub send_delay: u64,
    /// Time at which the next datagram generation is scheduled.
    /// C: `next_gen_time`.
    pub next_gen_time: [u64; 2],
    /// Time at which the current datagram became ready.
    /// C: `dg_time_ready`.
    pub dg_time_ready: [u64; 2],
    /// Whether the test is currently in ready state.  C: `is_ready`.
    pub is_ready: [bool; 2],
    /// Bind datagrams to path 0 only.  C: `test_affinity`.
    pub test_affinity: bool,
    /// Use the extended `provide_datagram_buffer_ex` API.  C: `use_extended_provider_api`.
    pub use_extended_provider_api: bool,
    /// Datagrams received on path 0 per direction.  C: `nb_recv_path_0`.
    pub nb_recv_path_0: [u64; 2],
    /// Datagrams received on other paths per direction.  C: `nb_recv_path_other`.
    pub nb_recv_path_other: [u64; 2],
}

/// Return the next scheduled datagram generation time across both directions.
/// C: `test_datagram_next_time_ready`.
pub fn test_datagram_next_time_ready(dg_ctx: &TestDatagramCtx) -> Instant {
    let mut next_time = 0u64;

    for dir in 0..2 {
        if !dg_ctx.is_ready[dir]
            && dg_ctx.dg_sent[dir] < dg_ctx.dg_target[dir]
            && (dg_ctx.next_gen_time[dir] < next_time || next_time == 0)
        {
            next_time = dg_ctx.next_gen_time[dir];
        }
    }

    Instant::from_ticks(next_time)
}

/// Check whether a datagram is ready for direction `dir` at `current_time`.
/// C: `test_datagram_check_ready`.
pub fn test_datagram_check_ready(
    dg_ctx: &mut TestDatagramCtx,
    dir: usize,
    current_time: u64,
) -> bool {
    if dir < 2
        && !dg_ctx.is_ready[dir]
        && dg_ctx.dg_sent[dir] < dg_ctx.dg_target[dir]
        && current_time >= dg_ctx.next_gen_time[dir]
    {
        dg_ctx.is_ready[dir] = true;
        dg_ctx.dg_time_ready[dir] = current_time;
    }

    dir < 2 && dg_ctx.is_ready[dir]
}

// ---------------------------------------------------------------------------
// Multipath-scenario link helpers.

/// Initialise multipath transport parameters with `initial_max_path_id = 2`.
/// When `enable_time_stamp` is true, sets `enable_time_stamp = 3`.
/// C: `multipath_init_params` in `picoquictest/multipath_test.c`.
pub fn multipath_init_params(enable_time_stamp: bool) -> TransportParameters {
    TransportParameters {
        initial_max_path_id: 2,
        enable_time_stamp: if enable_time_stamp { 3 } else { 0 },
        ..TransportParameters::default()
    }
}

/// Spin the simulator until the client has migrated to a new path.
/// C: `wait_client_migration_done`.
pub fn wait_client_migration_done(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + 4_000_000);
    let mut nb = 0;
    let mut nb_inactive = 0;
    while simulated_time.ticks() < time_out.ticks() && nb < 1024 && nb_inactive < 64 {
        let mut was = false;
        tls_api_one_sim_round(test_ctx, simulated_time, time_out, &mut was)?;
        nb += 1;
        if was {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }
    Ok(())
}

/// Spin the simulator until both sides have two paths with verified challenges.
/// C: `wait_multipath_ready`.
pub fn wait_multipath_ready(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + 4_000_000);
    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    fn path_one_ready(cnx: &Connection) -> bool {
        cnx.nb_paths() == 2
            && cnx
                .paths
                .get(1)
                .and_then(|path| path.tuples.first())
                .is_some_and(|tuple| tuple.challenge_verified)
    }

    fn client_connection_ready(test_ctx: &mut TestTlsApiCtx) -> bool {
        test_ctx
            .qclient
            .first_cnx_mut()
            .is_some_and(|cnx| cnx.connection_state == crate::State::Ready)
    }

    fn multipath_ready(test_ctx: &mut TestTlsApiCtx) -> bool {
        let client_ready = test_ctx
            .qclient
            .first_cnx_mut()
            .is_some_and(|cnx| cnx.connection_state == crate::State::Ready && path_one_ready(cnx));
        let server_ready = test_ctx
            .qserver
            .first_cnx_mut()
            .is_some_and(|cnx| path_one_ready(cnx));

        client_ready && server_ready
    }

    while simulated_time.ticks() < time_out.ticks()
        && nb_trials < 5000
        && nb_inactive < 64
        && client_connection_ready(test_ctx)
        && !multipath_ready(test_ctx)
    {
        let mut was = false;
        tls_api_one_sim_round(test_ctx, simulated_time, time_out, &mut was)?;
        nb_trials += 1;
        if was {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    if multipath_ready(test_ctx) {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// Attach a second pair of sim links to the test context for multipath tests.
/// When `mtu_drop` is true, the second links get a reduced path MTU.
/// C: `multipath_test_add_links`.
pub fn multipath_test_add_links(test_ctx: &mut TestTlsApiCtx, mtu_drop: bool) -> crate::Result<()> {
    let port_2 = test_ctx.client_addr.port() + 17;
    test_ctx.client_addr_2 = SocketAddr::from((test_ctx.client_addr.ip(), port_2));
    let mut c2s = TestSimLink::create(0.01, 10_000, None, 20_000, Instant::from_ticks(0))
        .map_err(|_| crate::Error::Generic)?;
    let mut s2c = TestSimLink::create(0.01, 10_000, None, 20_000, Instant::from_ticks(0))
        .map_err(|_| crate::Error::Generic)?;
    if mtu_drop {
        const INITIAL_MTU_IPV4: usize = 1252;
        c2s.path_mtu = (INITIAL_MTU_IPV4 + test_ctx.c_to_s_link.path_mtu) / 2;
        s2c.path_mtu = (INITIAL_MTU_IPV4 + test_ctx.s_to_c_link.path_mtu) / 2;
    }
    test_ctx.c_to_s_link_2 = Some(Box::new(c2s));
    test_ctx.s_to_c_link_2 = Some(Box::new(s2c));
    Ok(())
}

/// Disable a sim-link pair by setting `is_switched_off`.
/// `link_id = 0` → primary links; `link_id = 1` → secondary links.
/// C: `multipath_test_kill_links`.
pub fn multipath_test_kill_links(test_ctx: &mut TestTlsApiCtx, link_id: usize) {
    if link_id == 0 {
        test_ctx.c_to_s_link.next_send_time = Instant::from_ticks(u64::MAX);
        test_ctx.c_to_s_link.is_switched_off = true;
        test_ctx.s_to_c_link.next_send_time = Instant::from_ticks(u64::MAX);
        test_ctx.s_to_c_link.is_switched_off = true;
    } else {
        for l in [&mut test_ctx.c_to_s_link_2, &mut test_ctx.s_to_c_link_2]
            .into_iter()
            .flatten()
        {
            l.next_send_time = Instant::from_ticks(u64::MAX);
            l.is_switched_off = true;
        }
    }
}

/// Disable only the server-to-client side of one link pair.
/// C: `multipath_test_kill_server_links`.
pub fn multipath_test_kill_server_links(test_ctx: &mut TestTlsApiCtx, link_id: usize) {
    if link_id == 0 {
        test_ctx.s_to_c_link.next_send_time = Instant::from_ticks(u64::MAX);
        test_ctx.s_to_c_link.is_switched_off = true;
    } else if let Some(l) = test_ctx.s_to_c_link_2.as_mut() {
        l.next_send_time = Instant::from_ticks(u64::MAX);
        l.is_switched_off = true;
    }
}

/// Mark a link pair as "destination unreachable" (triggers socket errors).
/// C: `multipath_test_set_unreachable`.
pub fn multipath_test_set_unreachable(test_ctx: &mut TestTlsApiCtx, link_id: usize) {
    if link_id == 0 {
        test_ctx.c_to_s_link.is_unreachable = true;
        test_ctx.s_to_c_link.is_unreachable = true;
    } else {
        for l in [&mut test_ctx.c_to_s_link_2, &mut test_ctx.s_to_c_link_2]
            .into_iter()
            .flatten()
        {
            l.is_unreachable = true;
        }
    }
}

/// Clear the "unreachable" flag on a link pair.
/// C: `multipath_test_set_reachable`.
pub fn multipath_test_set_reachable(test_ctx: &mut TestTlsApiCtx, link_id: usize) {
    if link_id == 0 {
        test_ctx.c_to_s_link.is_unreachable = false;
        test_ctx.s_to_c_link.is_unreachable = false;
    } else {
        for l in [&mut test_ctx.c_to_s_link_2, &mut test_ctx.s_to_c_link_2]
            .into_iter()
            .flatten()
        {
            l.is_unreachable = false;
        }
    }
}

/// Re-enable a previously disabled link pair.
/// C: `multipath_test_unkill_links`.
pub fn multipath_test_unkill_links(
    test_ctx: &mut TestTlsApiCtx,
    link_id: usize,
    current_time: Instant,
) {
    if link_id == 0 {
        test_ctx.c_to_s_link.next_send_time = current_time;
        test_ctx.c_to_s_link.is_switched_off = false;
        test_ctx.s_to_c_link.next_send_time = current_time;
        // Faithful to C (apparent bug in C source — s_to_c stays switched_off).
        test_ctx.s_to_c_link.is_switched_off = true;
    } else {
        for l in [&mut test_ctx.c_to_s_link_2, &mut test_ctx.s_to_c_link_2]
            .into_iter()
            .flatten()
        {
            l.next_send_time = current_time;
            l.is_switched_off = false;
        }
    }
}

/// Configure satellite link parameters (1 Mbps or 300 ms latency).
/// C: `multipath_test_sat_links`.
pub fn multipath_test_sat_links(test_ctx: &mut TestTlsApiCtx, link_id: usize) {
    if link_id == 0 {
        test_ctx.c_to_s_link.picosec_per_byte = 8_000_000;
        test_ctx.s_to_c_link.picosec_per_byte = 8_000_000;
    } else {
        for l in [&mut test_ctx.c_to_s_link_2, &mut test_ctx.s_to_c_link_2]
            .into_iter()
            .flatten()
        {
            l.microsec_latency = 300_000;
            l.queue_delay_max = 600_000;
        }
    }
}

/// Configure performance (Wi-Fi / LTE) link parameters.
/// C: `multipath_test_perf_links`.
pub fn multipath_test_perf_links(test_ctx: &mut TestTlsApiCtx, link_id: usize) {
    if link_id == 0 {
        test_ctx.c_to_s_link.microsec_latency = 15_000;
        test_ctx.s_to_c_link.microsec_latency = 15_000;
        test_ctx.c_to_s_link.queue_delay_max = 30_000;
        test_ctx.s_to_c_link.queue_delay_max = 30_000;
        test_ctx.c_to_s_link.picosec_per_byte = 8_000_000 / 50;
        test_ctx.s_to_c_link.picosec_per_byte = 8_000_000 / 50;
    } else {
        for l in [&mut test_ctx.c_to_s_link_2, &mut test_ctx.s_to_c_link_2]
            .into_iter()
            .flatten()
        {
            l.microsec_latency = 30_000;
            l.queue_delay_max = 60_000;
            l.picosec_per_byte = 8_000_000 / 40;
        }
    }
}

// ---------------------------------------------------------------------------
// quic_tester helpers.

/// Run a final loss-tolerant check on a completed test connection.
/// C: `tls_api_test_with_loss_final` (defined in `picoquictest/tls_api.c`).
pub fn tls_api_test_with_loss_final<'s, 'a, S, A>(
    test_ctx: &mut TestTlsApiCtx,
    sni: S,
    alpn: A,
    simulated_time: &mut Instant,
) -> crate::Result<()>
where
    S: Into<Option<&'s str>>,
    A: Into<Option<&'a str>>,
{
    let sni = sni.into();
    let alpn = alpn.into();
    if test_ctx.qserver.first_cnx_mut().is_some() {
        verify_tls_api_transport_extension(test_ctx)?;
        verify_tls_api_sni(test_ctx, sni)?;
        verify_tls_api_alpn(test_ctx, alpn)?;
        verify_tls_api_version(test_ctx)?;
    }
    tls_api_close_with_losses(test_ctx, simulated_time, 0)
}

fn verify_tls_api_transport_extension(test_ctx: &mut TestTlsApiCtx) -> crate::Result<()> {
    let (client_local, client_remote) = {
        let client = test_ctx.cnx_client();
        (
            client.local_parameters.clone(),
            client.remote_parameters.clone(),
        )
    };
    let (server_local, server_remote) = {
        let server = test_ctx.cnx_server();
        (
            server.local_parameters.clone(),
            server.remote_parameters.clone(),
        )
    };

    if client_local.max_idle_timeout.ticks() == 0
        || client_local.initial_max_data == 0
        || client_local.initial_max_stream_data_bidi_local == 0
        || client_local.max_packet_size == 0
    {
        return Err(crate::Error::Generic);
    }
    if server_local.max_idle_timeout.ticks() == 0
        || server_local.initial_max_data == 0
        || server_local.initial_max_stream_data_bidi_remote == 0
        || server_local.max_packet_size == 0
    {
        return Err(crate::Error::Generic);
    }

    if !transport_parameters_equal(&client_local, &server_remote)
        || !transport_parameters_equal(&server_local, &client_remote)
    {
        return Err(crate::Error::Generic);
    }

    Ok(())
}

fn transport_parameters_equal(left: &TransportParameters, right: &TransportParameters) -> bool {
    left.initial_max_stream_data_bidi_local == right.initial_max_stream_data_bidi_local
        && left.initial_max_stream_data_bidi_remote == right.initial_max_stream_data_bidi_remote
        && left.initial_max_stream_data_uni == right.initial_max_stream_data_uni
        && left.initial_max_data == right.initial_max_data
        && left.initial_max_stream_id_bidir == right.initial_max_stream_id_bidir
        && left.initial_max_stream_id_unidir == right.initial_max_stream_id_unidir
        && left.max_idle_timeout == right.max_idle_timeout
        && left.max_packet_size == right.max_packet_size
        && left.max_ack_delay == right.max_ack_delay
        && left.active_connection_id_limit == right.active_connection_id_limit
        && left.ack_delay_exponent == right.ack_delay_exponent
        && left.migration_disabled == right.migration_disabled
        && left.preferred_address.v4 == right.preferred_address.v4
        && left.preferred_address.v6 == right.preferred_address.v6
        && left.preferred_address.connection_id == right.preferred_address.connection_id
        && left.preferred_address.stateless_reset_token
            == right.preferred_address.stateless_reset_token
        && left.max_datagram_frame_size == right.max_datagram_frame_size
        && left.enable_loss_bit == right.enable_loss_bit
        && left.enable_time_stamp == right.enable_time_stamp
        && left.min_ack_delay == right.min_ack_delay
        && left.do_grease_quic_bit == right.do_grease_quic_bit
        && left.version_negotiation.current == right.version_negotiation.current
        && left.version_negotiation.previous == right.version_negotiation.previous
        && left.version_negotiation.received == right.version_negotiation.received
        && left.version_negotiation.supported == right.version_negotiation.supported
        && left.enable_bdp_frame == right.enable_bdp_frame
        && left.initial_max_path_id == right.initial_max_path_id
        && left.address_discovery_mode == right.address_discovery_mode
        && left.is_reset_stream_at_enabled == right.is_reset_stream_at_enabled
}

fn verify_tls_api_sni(test_ctx: &mut TestTlsApiCtx, expected: Option<&str>) -> crate::Result<()> {
    let client_matches = {
        let client = test_ctx.cnx_client();
        client.sni.as_deref() == expected && client.tls_get_sni() == expected
    };
    if !client_matches {
        return Err(crate::Error::Generic);
    }

    if test_ctx.cnx_server().tls_get_sni() != expected {
        return Err(crate::Error::Generic);
    }

    Ok(())
}

fn verify_tls_api_alpn(test_ctx: &mut TestTlsApiCtx, expected: Option<&str>) -> crate::Result<()> {
    let client_matches = {
        let client = test_ctx.cnx_client();
        client.alpn.as_deref() == expected && client.tls_get_negotiated_alpn() == expected
    };
    if !client_matches {
        return Err(crate::Error::Generic);
    }

    if test_ctx.cnx_server().tls_get_negotiated_alpn() != expected {
        return Err(crate::Error::Generic);
    }

    Ok(())
}

fn verify_tls_api_version(test_ctx: &mut TestTlsApiCtx) -> crate::Result<()> {
    let (client_version_index, proposed_version) = {
        let client = test_ctx.cnx_client();
        (client.version_index, client.proposed_version)
    };
    let server_version_index = test_ctx.cnx_server().version_index;
    if client_version_index != server_version_index {
        return Err(crate::Error::Generic);
    }

    let Ok(version_index) = usize::try_from(client_version_index) else {
        return Err(crate::Error::Generic);
    };
    let Some(negotiated_version) = crate::internal::SUPPORTED_VERSIONS.get(version_index) else {
        return Err(crate::Error::Generic);
    };

    if crate::internal::SUPPORTED_VERSIONS
        .iter()
        .any(|version| *version as u32 == proposed_version)
        && *negotiated_version as u32 != proposed_version
    {
        return Err(crate::Error::Generic);
    }

    Ok(())
}

/// Advance the sim loop until the client has derived handshake-epoch keys.
/// C: `tester_wait_handshake_key` in `picoquictest/quic_tester.c`.
pub fn tester_wait_handshake_key(
    _test_ctx: &mut TestTlsApiCtx,
    _simulated_time: &mut Instant,
) -> crate::Result<()> {
    use crate::internal::Epoch;
    let time_out = Instant::from_ticks(_simulated_time.ticks() + 4_000_000);
    let mut nb_trials = 0u32;
    let mut nb_inactive = 0u32;
    while _simulated_time.ticks() < time_out.ticks() && nb_trials < 1024 && nb_inactive < 64 {
        nb_trials += 1;
        let has_key = {
            _test_ctx
                .qclient
                .first_cnx_mut()
                .map(|c| {
                    c.connection_state >= crate::State::ClientHandshakeStart
                        && c.crypto_context[Epoch::Handshake as usize]
                            .aead_encrypt
                            .is_some()
                })
                .unwrap_or(false)
        };
        if has_key {
            break;
        }
        let mut was_active = false;
        tls_api_one_sim_round(_test_ctx, _simulated_time, time_out, &mut was_active)?;
        if was_active {
            nb_inactive = 0;
            *_simulated_time = Instant::from_ticks(_simulated_time.ticks() + 1000);
        } else {
            nb_inactive += 1;
        }
    }
    Ok(())
}

/// Encode a minimal ACK frame acknowledging `last_packet_number`.
/// C: `tester_simple_ack_frame` in `picoquictest/quic_tester.c`.
pub fn tester_simple_ack_frame(last_packet_number: u64) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(5);
    for value in [
        crate::frames::FrameType::Ack as u64,
        last_packet_number,
        0,
        0,
        0,
    ] {
        let mut encoded = [0u8; 8];
        let written = crate::internal::varint_encode(&mut encoded, value);
        debug_assert!(written > 0);
        bytes.extend_from_slice(&encoded[..written]);
    }
    bytes
}

/// Build a packet containing `frame`, encrypt it as `ptype`, and either
/// inject it directly into the server or queue it on the sim link.
/// C: `tester_push_frame_packet` in `picoquictest/quic_tester.c`.
pub fn tester_push_frame_packet(
    test_ctx: &mut TestTlsApiCtx,
    ptype: crate::internal::PacketType,
    frame: &[u8],
    shall_pad: bool,
    shall_queue: bool,
    current_time: Instant,
) -> crate::Result<()> {
    let mut send_buffer = [0u8; MAX_PACKET_SIZE];
    let mut send_length = 0usize;

    {
        let cnx = test_ctx.cnx_client();
        let pc = match ptype {
            crate::internal::PacketType::Initial => PacketContext::Initial,
            crate::internal::PacketType::Handshake => PacketContext::Handshake,
            crate::internal::PacketType::ZeroRttProtected
            | crate::internal::PacketType::OneRttProtected => PacketContext::Application,
            _ => return Err(crate::Error::Generic),
        };

        let mut packet = cnx.allocate_packet().ok_or(crate::Error::Memory)?;
        packet.checksum_overhead = 16;
        packet.packet_type = ptype;
        packet.packet_context = pc;
        packet.offset = cnx.predict_packet_header_length_for_pc(ptype, pc);
        packet.length = packet.offset;
        packet.send_path = Some(crate::internal::PathToken::synthetic(0, 0));

        let sequence = cnx.pkt_ctx[pc as usize].send_sequence;
        let mut pn_offset = 0usize;
        let mut pn_length = 0usize;
        let header_len = cnx.create_packet_header_at(
            ptype,
            sequence,
            0,
            0,
            packet.offset,
            &mut packet.bytes,
            &mut pn_offset,
            &mut pn_length,
        );
        packet.offset = header_len;
        packet.length = header_len;

        if packet.length + frame.len() >= packet.bytes.len() {
            return Err(crate::Error::Memory);
        }
        packet.bytes[packet.length..packet.length + frame.len()].copy_from_slice(frame);
        packet.length += frame.len();

        if shall_pad {
            let target = cnx
                .paths
                .first()
                .map(|p| p.send_mtu.saturating_sub(packet.checksum_overhead))
                .unwrap_or(packet.length);
            packet.length =
                Connection::pad_to_target_length(&mut packet.bytes, packet.length, target);
        }

        if cnx.paths.is_empty() {
            return Err(crate::Error::Generic);
        }
        let mut path = cnx.paths.remove(0);
        let packet_length = packet.length;
        let packet_offset = packet.offset;
        let checksum_overhead = packet.checksum_overhead;
        cnx.finalize_and_protect_packet(
            &mut packet,
            0,
            packet_length,
            packet_offset,
            checksum_overhead,
            &mut send_length,
            &mut send_buffer,
            MAX_PACKET_SIZE,
            &mut path,
            current_time,
        );
        cnx.paths.insert(0, path);
    }

    let mut sim_packet = TestSimPacket::create()?;
    if send_length > sim_packet.bytes.len() {
        return Err(crate::Error::Memory);
    }
    sim_packet.bytes[..send_length].copy_from_slice(&send_buffer[..send_length]);
    sim_packet.length = send_length;
    sim_packet.addr_from = Some(test_ctx.client_addr);
    sim_packet.addr_to = Some(test_ctx.server_addr);
    sim_packet.ecn_mark = test_ctx.packet_ecn_default;
    if shall_queue {
        test_ctx.c_to_s_link.submit(sim_packet, current_time);
    } else {
        let len = sim_packet.length;
        let addr_from = test_ctx.client_addr;
        let addr_to = test_ctx.server_addr;
        let ecn = sim_packet.ecn_mark;
        let _ = test_ctx.qserver.incoming_packet(
            &mut sim_packet.bytes[..len],
            &addr_from,
            &addr_to,
            0,
            ecn,
            current_time,
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// TLS-API named helpers used by translated test bodies.

/// Run a TLS API handshake with a specific packet-loss bitmask.
/// C: `tls_api_loss_test` in `picoquictest/tls_api_test.c`.
pub fn tls_api_loss_test(loss_mask: u64) -> crate::Result<()> {
    tls_api_test_with_initial_loss(loss_mask, None, 0, Some(TEST_SNI), Some(TEST_ALPN))
}

/// Run a complete TLS API scenario with optional handshake loss, version, SNI, and ALPN.
/// C: `tls_api_test_with_loss` in `picoquictest/tls_api_test.c`.
pub fn tls_api_test_with_loss(
    init_loss_mask: Option<u64>,
    proposed_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
) -> crate::Result<()> {
    tls_api_test_with_initial_loss(
        init_loss_mask.unwrap_or(0),
        None,
        proposed_version,
        sni,
        alpn,
    )
}

fn tls_api_test_with_initial_loss(
    initial_loss_mask: u64,
    ticket_file: Option<&str>,
    proposed_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
) -> crate::Result<()> {
    let mut loss_mask = initial_loss_mask;
    tls_api_test_with_loss_mask(
        Some(&mut loss_mask),
        ticket_file,
        proposed_version,
        sni,
        alpn,
    )
}

fn tls_api_test_with_loss_mask(
    loss_mask: Option<&mut u64>,
    ticket_file: Option<&str>,
    proposed_version: u32,
    sni: Option<&str>,
    alpn: Option<&str>,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx_ex_named(
        &mut simulated_time,
        proposed_version,
        sni,
        alpn,
        ticket_file,
        None,
        false,
    )?;
    let mut no_loss = 0u64;
    let loss_mask = loss_mask.unwrap_or(&mut no_loss);
    tls_api_connection_loop(&mut test_ctx, loss_mask, 0, &mut simulated_time)?;
    tls_api_test_with_loss_final(&mut test_ctx, sni, alpn, &mut simulated_time)
}

/// Session-resume test helper (two successive connections sharing a ticket file).
/// C: `session_resume_test_one` (inlined in `session_resume_test`).
pub fn session_resume_test_one(ticket_file: &str) -> crate::Result<()> {
    save_empty_tickets(ticket_file, Instant::from_ticks(0))?;
    let mut loss_mask = 0u64;
    for i in 0..2usize {
        let mut simulated_time = Instant::from_ticks(0);
        let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, Some(ticket_file))
            .ok_or(crate::Error::Generic)?;
        test_ctx.cnx_client().max_early_data_size = 0;
        tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
        if i == 0 {
            session_resume_wait_for_ticket(&mut test_ctx, &mut simulated_time)?;
        }
        test_ctx.qclient.save_tickets(simulated_time, ticket_file)?;
        tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;
    }
    Ok(())
}

/// Run one MTU-discovery test.
/// C: `mtu_discovery_test_one` in `picoquictest/tls_api_test.c`.
pub fn mtu_discovery_test_one(
    policy: crate::PmtudPolicy,
    mtu_expected_client: u64,
    mtu_expected_server: u64,
    scenario: &[TestApiStreamDesc],
    mtu_max: u32,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    test_ctx.qserver.set_default_pmtud_policy(policy);
    test_ctx.cnx_client().set_pmtud_policy(policy);
    if mtu_max > 0 {
        test_ctx.qserver.set_mtu_max(mtu_max);
    }
    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, scenario)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    let mtu_client = test_ctx.cnx_client().primary_path_send_mtu();
    if mtu_client != mtu_expected_client {
        return Err(crate::Error::Generic);
    }
    if !test_ctx.has_cnx_server() {
        return Err(crate::Error::Generic);
    }
    let mtu_server = test_ctx.cnx_server().primary_path_send_mtu();
    if mtu_server != mtu_expected_server {
        return Err(crate::Error::Generic);
    }
    Ok(())
}

/// Run one MTU-drop congestion-control test.
/// C: `mtu_drop_cc_algotest` in `picoquictest/tls_api_test.c`.
pub fn mtu_drop_cc_algotest(algo_id: &'static str, target_time: u64) -> crate::Result<()> {
    const MTU_DROP_LATENCY: u64 = 100_000;
    const PICOSEC_1MBPS: u64 = 8_000_000;

    crate::register_all_congestion_control_algorithms();
    let cc_algo = crate::get_congestion_algorithm(algo_id).ok_or(crate::Error::Generic)?;

    let mut simulated_time = Instant::from_ticks(0);
    let mut initial_cid_bytes = [0xa1, 0x10, 0xcc, 0xa1, 0x90, 6, 7, 8];
    initial_cid_bytes[4] = cc_algo.congestion_algorithm_number;
    let initial_cid =
        ConnectionId::clone_from_slice(&initial_cid_bytes).ok_or(crate::Error::Protocol(9))?;
    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .ok_or(crate::Error::Protocol(10))?;

    test_ctx.c_to_s_link.microsec_latency = MTU_DROP_LATENCY;
    test_ctx.c_to_s_link.picosec_per_byte = PICOSEC_1MBPS;
    test_ctx.s_to_c_link.microsec_latency = MTU_DROP_LATENCY;
    test_ctx.s_to_c_link.picosec_per_byte = PICOSEC_1MBPS;
    test_ctx.qserver.set_default_congestion_algorithm(cc_algo);
    test_ctx.qserver.set_log_level(1);

    let mut loss_mask = 0u64;
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        2 * MTU_DROP_LATENCY,
        &mut simulated_time,
    )?;
    if !test_ctx.has_cnx_server() {
        return Err(crate::Error::Generic);
    }
    let server_cc_number = test_ctx
        .cnx_server()
        .congestion_alg
        .map(|alg| alg.congestion_algorithm_number)
        .ok_or(crate::Error::Generic)?;
    if server_cc_number != cc_algo.congestion_algorithm_number {
        return Err(crate::Error::Generic);
    }

    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario)?;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, 1_000_000)?;

    let client_send_mtu = test_ctx.cnx_client().primary_path_send_mtu();
    let server_local_max = test_ctx.cnx_server().local_parameters.max_packet_size as u64;
    if client_send_mtu != server_local_max {
        return Err(crate::Error::Generic);
    }

    let server_send_mtu = test_ctx.cnx_server().primary_path_send_mtu();
    let client_local_max = test_ctx.cnx_client().local_parameters.max_packet_size as u64;
    if server_send_mtu != client_local_max {
        return Err(crate::Error::Generic);
    }

    test_ctx.c_to_s_link.path_mtu = (INITIAL_MTU_IPV4 + test_ctx.c_to_s_link.path_mtu) / 2;
    test_ctx.s_to_c_link.path_mtu = (INITIAL_MTU_IPV4 + test_ctx.s_to_c_link.path_mtu) / 2;

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
}

const TEST_SCENARIO_STOP_SENDING: [TestApiStreamDesc; 2] = [
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 4,
        q_len: 531,
        r_len: 11_000,
    },
];

struct StopSendingCallback {
    state: std::rc::Rc<std::cell::RefCell<StopSendingState>>,
    client_mode: bool,
}

impl crate::StreamDataCallback for StopSendingCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: crate::CallbackEvent,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        self.state.borrow_mut().handle_event(
            self.client_mode,
            connection,
            stream_id,
            bytes,
            fin_or_event,
        )
    }
}

struct StopSendingState {
    streams: Vec<TestApiStream>,
    client_error_detected: bool,
    server_error_detected: bool,
    test_finished: bool,
}

impl StopSendingState {
    fn new(scenario: &[TestApiStreamDesc]) -> Self {
        Self {
            streams: scenario.iter().map(TestApiStream::new).collect(),
            client_error_detected: false,
            server_error_detected: false,
            test_finished: false,
        }
    }

    fn first_response_started(&self) -> bool {
        self.streams
            .first()
            .map(|stream| stream.r_recv_nb > 0)
            .unwrap_or(false)
    }

    fn mark_callback_error(&mut self, client_mode: bool) {
        if client_mode {
            self.client_error_detected = true;
        } else {
            self.server_error_detected = true;
        }
    }

    fn queue_initial_queries(
        &mut self,
        connection: &mut Connection,
        initial_data_stream_id: u64,
    ) -> crate::Result<()> {
        let mut more_stream = false;

        for stream in &mut self.streams {
            if stream.previous_stream_id != initial_data_stream_id {
                continue;
            }

            let stream_is_client = crate::stream::StreamId(stream.stream_id).is_client();
            if stream_is_client != connection.client_mode {
                continue;
            }

            let q_len = stream.q_len.min(stream.q_src.len());
            let data = stream.q_src[..q_len].to_vec();
            connection.add_to_stream(stream.stream_id, &data, true)?;
            stream.q_sent = true;
            more_stream = true;
        }

        if !more_stream {
            more_stream = self
                .streams
                .iter()
                .any(|stream| !stream.response_complete());
        }

        self.test_finished = !more_stream;
        Ok(())
    }

    fn handle_event(
        &mut self,
        client_mode: bool,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: crate::CallbackEvent,
    ) -> i32 {
        use crate::CallbackEvent;

        match fin_or_event {
            CallbackEvent::Close
            | CallbackEvent::ApplicationClose
            | CallbackEvent::AlmostReady
            | CallbackEvent::Ready
            | CallbackEvent::AppWakeup => return 0,
            CallbackEvent::StopSending => {
                if connection.reset_stream(stream_id, 0).is_err() {
                    self.mark_callback_error(client_mode);
                    return -1;
                }
                return 0;
            }
            CallbackEvent::StreamData | CallbackEvent::StreamFin | CallbackEvent::StreamReset => {}
            _ => {
                self.mark_callback_error(client_mode);
                return -1;
            }
        }

        let stream_index = match self
            .streams
            .iter()
            .position(|stream| stream.stream_id == stream_id)
        {
            Some(stream_index) => stream_index,
            None => {
                self.mark_callback_error(client_mode);
                return -1;
            }
        };

        let stream_is_client = crate::stream::StreamId(stream_id).is_client();
        let receiving_response = stream_is_client == client_mode;
        let mut response_to_send = None;
        let mut stream_finished = false;
        let mut callback_error;

        {
            let stream = &mut self.streams[stream_index];
            if receiving_response {
                callback_error =
                    !stop_sending_receive_stream_data(stream, true, bytes, fin_or_event);
                stream_finished = fin_or_event != CallbackEvent::StreamData;
            } else {
                callback_error =
                    !stop_sending_receive_stream_data(stream, false, bytes, fin_or_event);
                if !callback_error && fin_or_event != CallbackEvent::StreamData {
                    stream_finished = true;
                    if stream.r_len == 0 || fin_or_event == CallbackEvent::StreamReset {
                        if stream.r_received {
                            callback_error = true;
                        } else {
                            stream.r_received = true;
                        }
                    } else {
                        response_to_send = Some((stream_id, stream.r_src[..stream.r_len].to_vec()));
                    }
                }
            }
        }

        if callback_error {
            self.mark_callback_error(client_mode);
            return -1;
        }

        if let Some((response_stream_id, response)) = response_to_send
            && connection
                .add_to_stream(response_stream_id, &response, true)
                .is_err()
        {
            self.mark_callback_error(client_mode);
            return -1;
        }

        if stream_finished && self.queue_initial_queries(connection, stream_id).is_err() {
            self.mark_callback_error(client_mode);
            return -1;
        }

        0
    }

    fn verify(&self, test_ctx: &TestTlsApiCtx) -> crate::Result<()> {
        if self.server_error_detected || self.client_error_detected {
            return Err(crate::Error::Generic);
        }

        for (index, stream) in self.streams.iter().enumerate() {
            if stream.q_recv_nb != stream.q_len
                || !stream.q_received
                || stream.q_rcv != stream.q_src
                || !stream.r_received
            {
                return Err(crate::Error::Generic);
            }

            if index == 0 {
                if stream.r_recv_nb == 0 || stream.r_recv_nb >= stream.r_len {
                    return Err(crate::Error::Generic);
                }
                if stream.r_rcv[..stream.r_recv_nb] != stream.r_src[..stream.r_recv_nb] {
                    return Err(crate::Error::Generic);
                }
            } else if stream.r_recv_nb != stream.r_len || stream.r_rcv != stream.r_src {
                return Err(crate::Error::Generic);
            }
        }

        if test_ctx.qclient.nb_data_nodes_allocated > test_ctx.qclient.nb_data_nodes_in_pool()
            || test_ctx.qserver.nb_data_nodes_allocated > test_ctx.qserver.nb_data_nodes_in_pool()
        {
            return Err(crate::Error::Generic);
        }

        Ok(())
    }
}

fn stop_sending_receive_stream_data(
    stream: &mut TestApiStream,
    is_response: bool,
    bytes: &[u8],
    fin_or_event: crate::CallbackEvent,
) -> bool {
    let is_data_event = fin_or_event == crate::CallbackEvent::StreamData;

    if is_response {
        let end = match stream.r_recv_nb.checked_add(bytes.len()) {
            Some(end) => end,
            None => return false,
        };
        if end > stream.r_len || bytes != &stream.r_src[stream.r_recv_nb..end] {
            return false;
        }
        stream.r_rcv[stream.r_recv_nb..end].copy_from_slice(bytes);
        stream.r_recv_nb = end;
        if !is_data_event {
            if stream.r_received {
                return false;
            }
            stream.r_received = true;
        }
    } else {
        let end = match stream.q_recv_nb.checked_add(bytes.len()) {
            Some(end) => end,
            None => return false,
        };
        if end > stream.q_len || bytes != &stream.q_src[stream.q_recv_nb..end] {
            return false;
        }
        stream.q_rcv[stream.q_recv_nb..end].copy_from_slice(bytes);
        stream.q_recv_nb = end;
        if !is_data_event {
            if stream.q_received {
                return false;
            }
            stream.q_received = true;
        }
    }

    true
}

/// Run one stop-sending test.  `discard=true` → discard stream variant.
/// C: `stop_sending_test_one` in `picoquictest/tls_api_test.c`.
pub fn stop_sending_test_one(discard: bool, reset_loss: bool) -> crate::Result<()> {
    const STOP_SENDING_LATENCY: u64 = 100_000;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, Version::InternalTest1 as u32, None)
        .ok_or(crate::Error::Generic)?;
    test_ctx.c_to_s_link.microsec_latency = STOP_SENDING_LATENCY;
    test_ctx.s_to_c_link.microsec_latency = STOP_SENDING_LATENCY;

    let mut loss_mask = 0x0F0F0F0F0F000000u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        return Err(crate::Error::Generic);
    }

    let stop_sending_state = std::rc::Rc::new(std::cell::RefCell::new(StopSendingState::new(
        &TEST_SCENARIO_STOP_SENDING,
    )));
    test_ctx
        .cnx_client()
        .set_callback(Some(Box::new(StopSendingCallback {
            state: std::rc::Rc::clone(&stop_sending_state),
            client_mode: true,
        })));
    test_ctx
        .cnx_server()
        .set_callback(Some(Box::new(StopSendingCallback {
            state: std::rc::Rc::clone(&stop_sending_state),
            client_mode: false,
        })));

    {
        let mut state = stop_sending_state.borrow_mut();
        state.queue_initial_queries(test_ctx.cnx_client(), 0)?;
    }

    for _ in 0..64 {
        tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 16)?;
        if stop_sending_state.borrow().first_response_started() {
            break;
        }
    }
    if !stop_sending_state.borrow().first_response_started() {
        return Err(crate::Error::Generic);
    }

    if discard {
        test_ctx.cnx_client().discard_stream(4, 1)?;
        let reset_callback_result = stop_sending_state.borrow_mut().handle_event(
            true,
            test_ctx.cnx_client(),
            4,
            &[],
            crate::CallbackEvent::StreamReset,
        );
        if reset_callback_result != 0 {
            return Err(crate::Error::Generic);
        }
    } else {
        test_ctx.cnx_client().stop_sending(4, 1)?;
    }

    if reset_loss {
        loss_mask = 0x00FC0000000u64;
    }

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    {
        let state = stop_sending_state.borrow();
        state.verify(&test_ctx)?;
    }
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

/// Run one CNXID-transmit test.
/// C: `transmit_cnxid_test_one` in `picoquictest/tls_api_test.c`.
pub fn transmit_cnxid_test_one(
    retire_before: bool,
    disable_migration: bool,
    retire_number_zero: bool,
) -> crate::Result<()> {
    const SYNC_EMPTY_LOOP_TIMEOUT: u64 = 4_000_000;
    const DEFAULT_CONNECTION_ID_TTL: u64 = 5_000_000;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;

    if disable_migration {
        let mut test_parameters = TransportParameters::default();
        init_transport_parameters(&mut test_parameters);
        test_parameters.migration_disabled = true;
        test_ctx.qserver.set_default_tp(&test_parameters)?;
    }

    if retire_before {
        test_ctx
            .qserver
            .set_default_connection_id_ttl(DEFAULT_CONNECTION_ID_TTL);
    }

    if retire_number_zero {
        let mut nb_trials = 0;
        while nb_trials < 16 && !test_ctx.has_cnx_server() {
            let mut was_active = false;
            nb_trials += 1;
            tls_api_one_sim_round(
                &mut test_ctx,
                &mut simulated_time,
                Instant::from_ticks(0),
                &mut was_active,
            )?;
        }

        if !test_ctx.has_cnx_server() {
            return Err(crate::Error::Generic);
        }

        let server_list = first_test_connection_mut(&mut test_ctx.qserver)?
            .local_connection_id_lists
            .first_mut()
            .ok_or(crate::Error::Generic)?;
        server_list.local_connection_id_retire_before = 1;
    }

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        NB_PATH_TARGET as i32,
        0,
    )?;

    if retire_number_zero
        && first_path_remote_cid_sequence(first_test_connection(&test_ctx.qclient)?)? == 0
    {
        return Err(crate::Error::Generic);
    }

    if retire_before {
        tls_api_wait_for_timeout(
            &mut test_ctx,
            &mut simulated_time,
            DEFAULT_CONNECTION_ID_TTL - SYNC_EMPTY_LOOP_TIMEOUT,
        )?;

        if first_local_cid_retire_before(first_test_connection(&test_ctx.qserver)?)? == 0 {
            return Err(crate::Error::Generic);
        }

        tls_api_synch_to_empty_loop(
            &mut test_ctx,
            &mut simulated_time,
            2048,
            NB_PATH_TARGET as i32,
            0,
        )?;
    }

    if first_local_cid_count(first_test_connection(&test_ctx.qclient)?)? < NB_PATH_TARGET
        || first_local_cid_count(first_test_connection(&test_ctx.qserver)?)? < NB_PATH_TARGET
    {
        return Err(crate::Error::Generic);
    }

    {
        let client_cnx = first_test_connection(&test_ctx.qclient)?;
        let server_cnx = first_test_connection(&test_ctx.qserver)?;
        transmit_cnxid_test_stash(client_cnx, server_cnx)?;
        transmit_cnxid_test_stash(server_cnx, client_cnx)?;
    }

    if retire_before {
        let retire_before_next =
            first_local_cid_retire_before(first_test_connection(&test_ctx.qserver)?)?;
        let old_cid_still_stashed =
            first_remote_cid_stash(first_test_connection(&test_ctx.qclient)?)?
                .connection_ids
                .iter()
                .any(|stashed| stashed.sequence < retire_before_next);
        if old_cid_still_stashed {
            return Err(crate::Error::Generic);
        }
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

fn first_test_connection(quic: &Quic) -> crate::Result<&Connection> {
    quic.connections.iter().next().ok_or(crate::Error::Generic)
}

fn first_test_connection_mut(quic: &mut Quic) -> crate::Result<&mut Connection> {
    quic.connections
        .iter_mut()
        .next()
        .ok_or(crate::Error::Generic)
}

fn first_local_cid_count(cnx: &Connection) -> crate::Result<usize> {
    cnx.local_connection_id_lists
        .first()
        .map(|list| list.connection_ids.len())
        .ok_or(crate::Error::Generic)
}

fn first_local_cid_retire_before(cnx: &Connection) -> crate::Result<u64> {
    cnx.local_connection_id_lists
        .first()
        .map(|list| list.local_connection_id_retire_before)
        .ok_or(crate::Error::Generic)
}

fn first_remote_cid_stash(
    cnx: &Connection,
) -> crate::Result<&crate::internal::RemoteConnectionIdStash> {
    cnx.remote_connection_id_stashes
        .first()
        .ok_or(crate::Error::Generic)
}

fn first_path_remote_cid_sequence(cnx: &Connection) -> crate::Result<u64> {
    let path = cnx.paths.first().ok_or(crate::Error::Generic)?;
    let tuple = path.tuples.first().ok_or(crate::Error::Generic)?;
    let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
    cnx.remote_connection_id_stashes
        .iter()
        .find(|stash| stash.unique_path_id == path.unique_path_id)
        .and_then(|stash| stash.connection_ids.get(cid_index))
        .map(|remote_cid| remote_cid.sequence)
        .ok_or(crate::Error::Generic)
}

fn transmit_cnxid_test_stash(cnx1: &Connection, cnx2: &Connection) -> crate::Result<()> {
    let stash = first_remote_cid_stash(cnx1)?;
    let cid_list = cnx2
        .local_connection_id_lists
        .first()
        .ok_or(crate::Error::Generic)?;

    if stash.connection_ids.len() != cid_list.connection_ids.len() {
        return Err(crate::Error::Generic);
    }

    for (remote_cid, local_token) in stash.connection_ids.iter().zip(&cid_list.connection_ids) {
        let local_cid = cnx2
            .local_connection_ids
            .get(*local_token)
            .ok_or(crate::Error::Generic)?;
        if remote_cid.connection_id != local_cid.connection_id {
            return Err(crate::Error::Generic);
        }
    }

    Ok(())
}

/// Run one NAT-rebinding test.
/// C: `nat_rebinding_test_one` in `picoquictest/tls_api_test.c`.
pub fn nat_rebinding_test_one(
    loss_mask_data: u64,
    cid_zero: bool,
    latency: u64,
) -> crate::Result<()> {
    const TEST_SCENARIO_Q_AND_R: [TestApiStreamDesc; 1] = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    fn server_challenge_state(cnx: &Connection) -> crate::Result<(u64, bool)> {
        let tuple = cnx
            .paths
            .first()
            .and_then(|path| path.tuples.first())
            .ok_or(crate::Error::Generic)?;
        Ok((tuple.challenge[0], tuple.challenge_verified))
    }

    fn server_remote_cid_postcheck(cnx: &Connection) -> crate::Result<()> {
        let stash = first_remote_cid_stash(cnx)?;
        let first_sequence = stash
            .connection_ids
            .first()
            .map(|cid| cid.sequence)
            .ok_or(crate::Error::Generic)?;

        if cnx.nb_paths() > 1 || first_sequence == 0 || stash.connection_ids.len() < 8 {
            Err(crate::Error::Generic)
        } else {
            Ok(())
        }
    }

    let mut simulated_time = Instant::from_ticks(0);
    let mut initial_cid_bytes = [0x19, 0x8a, 0, 0, 0, 0, 0, 0];
    if loss_mask_data != 0 {
        initial_cid_bytes[2] = 0x10;
    }
    if cid_zero {
        initial_cid_bytes[3] = 0xc1;
    }
    let initial_cid =
        ConnectionId::clone_from_slice(&initial_cid_bytes).ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex_named(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        Some(&initial_cid),
        cid_zero,
    )?;

    if latency > 0 {
        test_ctx.c_to_s_link.microsec_latency = latency;
        test_ctx.s_to_c_link.microsec_latency = latency;
    }
    test_ctx.qserver.set_log_level(1);
    test_ctx.qserver.set_qlog(".")?;
    test_ctx.qclient.set_log_level(1);
    test_ctx.qclient.set_qlog(".")?;

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        NB_PATH_TARGET as i32,
        1,
    )?;

    let initial_challenge = server_challenge_state(test_ctx.cnx_server())?.0;
    loss_mask = loss_mask_data;

    let mut natted_addr = test_ctx.client_addr;
    natted_addr.set_port(natted_addr.port().wrapping_add(17));
    test_ctx.client_addr_natted = natted_addr;
    test_ctx.client_use_nat = true;

    test_api_init_send_recv_scenario(&mut test_ctx, &TEST_SCENARIO_Q_AND_R)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_verify(&test_ctx)?;

    let next_time = Instant::from_ticks(simulated_time.ticks().saturating_add(3_000_000));
    let mut nb_inactive = 0usize;
    loss_mask = 0;
    while simulated_time.ticks() < next_time.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
        && !cid_zero
    {
        let (_, challenge_verified) = server_challenge_state(test_ctx.cnx_server())?;
        let remote_cid_pending = server_remote_cid_postcheck(test_ctx.cnx_server()).is_err();
        if challenge_verified && !remote_cid_pending {
            break;
        }

        let mut was_active = false;
        tls_api_one_sim_round_with_loss(
            &mut test_ctx,
            &mut simulated_time,
            next_time,
            &mut was_active,
            &mut loss_mask,
        )?;

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
            if nb_inactive > 256 {
                break;
            }
        }
    }

    if !test_ctx.has_cnx_server() {
        return Err(crate::Error::Generic);
    }

    let (challenge, challenge_verified) = server_challenge_state(test_ctx.cnx_server())?;
    if challenge == initial_challenge || !challenge_verified {
        return Err(crate::Error::Generic);
    }

    if !cid_zero {
        server_remote_cid_postcheck(test_ctx.cnx_server())?;
    }

    Ok(())
}

/// Run a migration scenario test over the given stream scenario.
/// C: `migration_test_scenario` in `picoquictest/tls_api_test.c`.
fn migration_tuple_remote_cid(
    cnx: &Connection,
    path_index: usize,
    tuple_index: usize,
) -> crate::Result<ConnectionId> {
    let tuple = cnx
        .paths
        .get(path_index)
        .and_then(|path| path.tuples.get(tuple_index))
        .ok_or(crate::Error::Generic)?;
    let remote_index = tuple
        .remote_connection_id_index
        .ok_or(crate::Error::Generic)?;

    cnx.remote_connection_id_stashes
        .iter()
        .find(|stash| stash.unique_path_id == tuple.unique_path_id)
        .and_then(|stash| stash.connection_ids.get(remote_index))
        .map(|remote_cid| remote_cid.connection_id)
        .ok_or(crate::Error::Generic)
}

fn migration_tuple_local_cid(
    cnx: &Connection,
    path_index: usize,
    tuple_index: usize,
) -> crate::Result<ConnectionId> {
    let local_token = cnx
        .paths
        .get(path_index)
        .and_then(|path| path.tuples.get(tuple_index))
        .and_then(|tuple| tuple.local_connection_id)
        .ok_or(crate::Error::Generic)?;

    cnx.local_connection_ids
        .get(local_token)
        .map(|local_cid| local_cid.connection_id)
        .ok_or(crate::Error::Generic)
}

fn migration_server_challenge_state(cnx: &Connection) -> crate::Result<(u64, bool, bool)> {
    let path = cnx.paths.first().ok_or(crate::Error::Generic)?;
    let tuple = path.tuples.first().ok_or(crate::Error::Generic)?;
    Ok((
        tuple.challenge[0],
        tuple.challenge_verified,
        path.path_is_demoted,
    ))
}

pub fn migration_test_scenario(
    scenario: &[TestApiStreamDesc],
    loss_target: u64,
    cid_zero: bool,
) -> crate::Result<()> {
    let initial_challenge = 0u64;
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx_ex_named(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        None,
        cid_zero,
    )?;
    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    tls_api_synch_to_empty_loop(&mut test_ctx, &mut simulated_time, 2048, 4, 1)?;

    test_ctx
        .client_addr
        .set_port(test_ctx.client_addr.port() + 17);
    let server_addr = test_ctx.server_addr;
    let client_addr = test_ctx.client_addr;
    test_ctx
        .cnx_client()
        .probe_new_path(&server_addr, &client_addr, simulated_time)?;

    let (target_id, previous_local_id) = {
        let cnx = test_ctx.cnx_client();
        let target_id = migration_tuple_remote_cid(cnx, 0, 1)?;
        let previous_local_id = if cid_zero {
            None
        } else {
            Some(migration_tuple_local_cid(cnx, 0, 0)?)
        };
        (target_id, previous_local_id)
    };

    test_api_init_send_recv_scenario(&mut test_ctx, scenario)?;
    loss_mask = loss_target;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_verify(&test_ctx)?;

    let next_time = Instant::from_ticks(simulated_time.ticks() + 4_000_000);
    test_ctx.c_to_s_link.loss_mask = Some(0);
    test_ctx.s_to_c_link.loss_mask = Some(0);
    while simulated_time.ticks() < next_time.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let (challenge, challenge_verified, path_is_demoted) =
            migration_server_challenge_state(test_ctx.cnx_server())?;
        if challenge_verified && !path_is_demoted && challenge != initial_challenge {
            break;
        }

        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            next_time,
            &mut was_active,
        )?;
    }

    if !test_ctx.has_cnx_server() {
        return Err(crate::Error::Generic);
    }
    let (challenge, challenge_verified, _) =
        migration_server_challenge_state(test_ctx.cnx_server())?;
    if challenge == initial_challenge || !challenge_verified {
        return Err(crate::Error::Generic);
    }

    let current_remote_id = migration_tuple_remote_cid(test_ctx.cnx_client(), 0, 0)?;
    if current_remote_id != target_id {
        return Err(crate::Error::Generic);
    }
    if let Some(previous_local_id) = previous_local_id {
        let current_local_id = migration_tuple_local_cid(test_ctx.cnx_client(), 0, 0)?;
        if current_local_id == previous_local_id {
            return Err(crate::Error::Generic);
        }
    }

    if test_ctx.client_ready() {
        tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
    } else {
        Ok(())
    }
}

/// Run one migration-controlled test (mode 0=client, 1=server, 2=both).
/// C: `migration_controlled_test_one` in `picoquictest/tls_api_test.c`.
pub fn migration_controlled_test_one(_mode: u32) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

fn padding_test_error(code: u64) -> crate::Error {
    crate::Error::Protocol(0x5B02_1000 | code)
}

fn padding_test_predict_pn_length(cnx: &Connection) -> usize {
    let pkt_ctx = &cnx.pkt_ctx[PacketContext::Application as usize];
    let mut pn_l = 4usize;
    let mut delta = if pkt_ctx.send_sequence == 0 {
        0i128
    } else {
        pkt_ctx.send_sequence.saturating_sub(1) as i128
    };

    if let Some(first_pending) = pkt_ctx.pending.keys().next().copied() {
        delta -= first_pending as i128;
    }

    if delta < 262_144 {
        pn_l = 3;
        if pkt_ctx.send_sequence < 1024 {
            pn_l = 2;
            if pkt_ctx.send_sequence < 16 {
                pn_l = 1;
            }
        }
    }

    pn_l
}

fn padding_test_check_packet_length(
    test_ctx: &mut TestTlsApiCtx,
    padding_multiple: u32,
    padding_min_size: u32,
    test_size: usize,
    length: usize,
) -> crate::Result<()> {
    let (checksum_length, pn_iv_length, pn_length, pn_offset, raw_length, send_mtu) = {
        let client = test_ctx.cnx_client();
        let checksum_length = client.get_checksum_length(Epoch::OneRtt);
        let pn_iv_length = client.crypto_context[Epoch::OneRtt as usize]
            .pn_enc
            .as_deref()
            .map(crate::tls_api::pn_iv_size)
            .ok_or_else(|| padding_test_error(1))?;
        let pn_length = padding_test_predict_pn_length(client);
        let header_length = client.predict_packet_header_length_for_pc(
            PacketType::OneRttProtected,
            PacketContext::Application,
        );
        let pn_offset = header_length
            .checked_sub(pn_length)
            .ok_or_else(|| padding_test_error(2))?;
        let raw_length = header_length
            .checked_add(test_size)
            .ok_or_else(|| padding_test_error(3))?;
        let send_mtu = usize::try_from(client.primary_path_send_mtu()).unwrap_or(usize::MAX);
        (
            checksum_length,
            pn_iv_length,
            pn_length,
            pn_offset,
            raw_length,
            send_mtu,
        )
    };

    let padding_multiple = padding_multiple as usize;
    let padding_min_size = padding_min_size as usize;
    let size_code = test_size as u64;

    if pn_length == 1 && raw_length.saturating_add(checksum_length) > length {
        return Err(padding_test_error(0x1000 | size_code));
    }
    if pn_offset.saturating_add(4).saturating_add(pn_iv_length) > length {
        return Err(padding_test_error(0x2000 | size_code));
    }

    if padding_multiple == 0 && padding_min_size == 0 {
        let natural_ack_slack = raw_length.saturating_add(checksum_length).saturating_add(6);
        if natural_ack_slack < length
            && pn_offset.saturating_add(4).saturating_add(pn_iv_length) != length
        {
            return Err(padding_test_error(0x3000 | size_code));
        }
    } else if padding_min_size != 0 && length < padding_min_size {
        return Err(padding_test_error(0x4000 | size_code));
    } else if padding_min_size != 0 && raw_length < padding_min_size {
        let min_with_checksum = padding_min_size
            .checked_add(checksum_length)
            .ok_or_else(|| padding_test_error(4))?;
        if length != min_with_checksum {
            return Err(padding_test_error(0x5000 | size_code));
        }
    } else if padding_multiple != 0 {
        let formula_length = length
            .checked_sub(padding_min_size)
            .and_then(|v| v.checked_sub(checksum_length))
            .ok_or_else(|| padding_test_error(0x6000 | size_code))?;
        if formula_length % padding_multiple != 0 && length != send_mtu {
            return Err(padding_test_error(0x7000 | size_code));
        }

        if raw_length > padding_min_size {
            let padding_over_raw = length
                .checked_sub(checksum_length)
                .and_then(|v| v.checked_sub(raw_length))
                .ok_or_else(|| padding_test_error(0x8000 | size_code))?;
            if padding_over_raw >= padding_multiple {
                return Err(padding_test_error(0x9000 | size_code));
            }
        }
    }

    Ok(())
}

/// Run one qlog-trace test.
/// C: `qlog_trace_test_one` in `picoquictest/tls_api_test.c`.
pub fn qlog_trace_test_one(recv_ecn: u8, parallel: bool) -> crate::Result<()> {
    qlog_trace_like_test_one(recv_ecn, parallel)
}

/// Run one qlog-fns test.
/// C: `qlog_fns_test_one` in `picoquictest/tls_api_test.c`.
pub fn qlog_fns_test_one(recv_ecn: u8) -> crate::Result<()> {
    qlog_trace_like_test_one(recv_ecn, false)
}

fn qlog_trace_like_test_one(recv_ecn: u8, parallel: bool) -> crate::Result<()> {
    const QLOG_TRACE_AUTO_QLOG: &str = "0102030405060708.server.qlog";
    const QLOG_TRACE_TEST_REF: &str = "picoquictest/qlog_trace_ref.txt";
    const QLOG_TRACE_ECN_TEST_REF: &str = "picoquictest/qlog_trace_ecn_ref.txt";
    const INITIAL_CID: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    const RESET_SEED_CLIENT: [u8; RESET_SECRET_SIZE] = [
        10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    ];
    const RESET_SEED_SERVER: [u8; RESET_SECRET_SIZE] = [
        20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35,
    ];
    static QLOG_TRACE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    let _guard = QLOG_TRACE_LOCK.lock().map_err(|_| crate::Error::Generic)?;
    let _ = std::fs::remove_file(QLOG_TRACE_AUTO_QLOG);

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, Version::InternalTest1 as u32, None)
        .ok_or(crate::Error::Generic)?;

    test_ctx.ecn_support = recv_ecn;
    test_ctx.packet_ecn_default = recv_ecn;
    if parallel {
        test_ctx.qserver.set_binlog(Some("."))?;
    }
    test_ctx.qserver.set_qlog(".")?;
    let _ = test_ctx
        .qserver
        .set_default_spinbit_policy(SpinbitVersion::On);
    let _ = test_ctx
        .qclient
        .set_default_spinbit_policy(SpinbitVersion::On);
    test_ctx
        .qserver
        .set_default_lossbit_policy(LossbitVersion::SendReceive);
    test_ctx
        .qclient
        .set_default_lossbit_policy(LossbitVersion::SendReceive);
    test_ctx.qserver.connection_id_callback_fn = Some(Box::new(QlogTraceCid {
        data: ConnectionId::clone_from_slice(&[2; 8]).ok_or(crate::Error::Generic)?,
    }));
    test_ctx.qclient.connection_id_callback_fn = Some(Box::new(QlogTraceCid {
        data: ConnectionId::clone_from_slice(&[1; 8]).ok_or(crate::Error::Generic)?,
    }));
    test_ctx.qclient.reset_seed = RESET_SEED_CLIENT;
    test_ctx.qserver.reset_seed = RESET_SEED_SERVER;

    let _ = test_ctx.qclient.set_cipher_suite(AES_128_GCM_SHA256);
    let _ = test_ctx.qclient.set_key_exchange(GROUP_SECP256R1);

    let old_initial_cid = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.initial_connection_id);
    if let Some(old_initial_cid) = old_initial_cid
        && let Some((token, _)) = test_ctx.qclient.connection_by_id(old_initial_cid)
    {
        test_ctx.qclient.delete_connection(token);
    }

    let initial_cid = ConnectionId::clone_from_slice(&INITIAL_CID).ok_or(crate::Error::Generic)?;
    test_ctx
        .qclient
        .create_connection(
            initial_cid,
            ConnectionId::default(),
            Some(&test_ctx.server_addr),
            simulated_time,
            Version::InternalTest1 as u32,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .ok_or(crate::Error::Generic)?
        .start_client()?;

    let scenario_q2_and_r2 = [
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 2000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 0,
            q_len: 531,
            r_len: 11000,
        },
    ];
    let mut loss_mask = 0;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 20_000, &mut simulated_time)?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario_q2_and_r2)?;
    loss_mask = 0x0001_0a04;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 2_000_000)?;

    qlog_fns_inject_bad_packet(&mut test_ctx, simulated_time, recv_ecn)?;
    drop(test_ctx);

    let reference = if recv_ecn == 0 {
        QLOG_TRACE_TEST_REF
    } else {
        QLOG_TRACE_ECN_TEST_REF
    };
    compare_text_files(QLOG_TRACE_AUTO_QLOG, reference)
}

struct QlogTraceCid {
    data: ConnectionId,
}

impl ConnectionIdCallback for QlogTraceCid {
    fn produce(
        &mut self,
        _quic: &mut Quic,
        connection_id_local: ConnectionId,
        connection_id_remote: ConnectionId,
    ) -> ConnectionId {
        let len = connection_id_local.len();
        let mut cid = ConnectionId::with_size(len).unwrap_or_default();
        for i in 0..len {
            cid.as_bytes_mut()[i] = connection_id_remote
                .as_bytes()
                .get(i)
                .copied()
                .unwrap_or(0)
                .wrapping_add(self.data.as_bytes().get(i).copied().unwrap_or(0));
        }

        for byte in self.data.as_bytes_mut().iter_mut().take(len) {
            *byte = byte.wrapping_add(1);
            if *byte != 0 {
                break;
            }
        }

        cid
    }
}

fn qlog_fns_inject_bad_packet(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: Instant,
    recv_ecn: u8,
) -> crate::Result<()> {
    if !test_ctx.has_cnx_server() {
        return Ok(());
    }

    let (local_cid, peer_addr, local_addr) =
        qlog_fns_bad_packet_route(test_ctx).ok_or(crate::Error::Generic)?;

    let mut packet = [0u8; 256];
    let cid_len = local_cid.len().min(packet.len() - 1);
    packet[1..1 + cid_len].copy_from_slice(&local_cid.as_bytes()[..cid_len]);
    packet[0] |= 64;
    let _ = test_ctx.qserver.incoming_packet(
        &mut packet,
        &peer_addr,
        &local_addr,
        0,
        recv_ecn,
        simulated_time,
    );
    Ok(())
}

fn qlog_fns_bad_packet_route(
    test_ctx: &mut TestTlsApiCtx,
) -> Option<(ConnectionId, SocketAddr, SocketAddr)> {
    let server = test_ctx.qserver.first_cnx_mut()?;
    let tuple = server.paths.first()?.tuples.first()?;
    let local_cid = tuple
        .local_connection_id
        .and_then(|token| server.local_connection_ids.get(token))
        .map(|cid| cid.connection_id)?;

    Some((local_cid, tuple.peer_addr, tuple.local_addr))
}

/// Run one padding test.
/// C: `padding_test_one` in `picoquictest/tls_api_test.c`.
pub fn padding_test_one(padding_multiple: u32, padding_min_size: u32) -> crate::Result<()> {
    const TEST_SIZES: [usize; 15] = [1, 2, 3, 5, 8, 13, 21, 44, 65, 109, 174, 283, 457, 740, 1023];

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, Version::InternalTest1 as u32, None)
        .ok_or_else(|| padding_test_error(5))?;
    test_ctx
        .qserver
        .set_default_padding(padding_multiple, padding_min_size);
    test_ctx
        .cnx_client()
        .set_padding_policy(padding_multiple, padding_min_size);

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        let client_state = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|cnx| cnx.connection_state as u64)
            .unwrap_or(0xff);
        let server_state = test_ctx
            .qserver
            .first_cnx_mut()
            .map(|cnx| cnx.connection_state as u64)
            .unwrap_or(0xff);
        return Err(padding_test_error(
            0xA000 | (client_state << 8) | server_state,
        ));
    }

    for test_size in TEST_SIZES {
        let mut data = vec![crate::frames::FrameType::Padding as u8; test_size];
        data[test_size - 1] = crate::frames::FrameType::Ping as u8;

        let mut nb_trials = 0;
        let mut nb_inactive = 0;
        let mut is_queued = false;
        let mut is_success = false;

        while nb_trials < 256
            && nb_inactive < 256
            && test_ctx.client_ready()
            && test_ctx.server_ready()
        {
            let mut was_active = false;
            nb_trials += 1;

            let client_empty = test_ctx.cnx_client().is_cnx_backlog_empty();
            let server_empty = test_ctx.cnx_server().is_cnx_backlog_empty();
            let links_empty =
                test_ctx.c_to_s_link.packets.is_empty() && test_ctx.s_to_c_link.packets.is_empty();

            if client_empty && server_empty && links_empty {
                if !is_queued {
                    test_ctx.cnx_client().queue_misc_frame(
                        &data,
                        false,
                        PacketContext::Application,
                    )?;
                    is_queued = true;
                } else {
                    is_success = true;
                    break;
                }
            }

            tls_api_one_sim_round(
                &mut test_ctx,
                &mut simulated_time,
                Instant::from_ticks(0),
                &mut was_active,
            )?;

            if is_queued
                && let Some(length) = test_ctx
                    .c_to_s_link
                    .packets
                    .front()
                    .map(|packet| packet.length)
            {
                padding_test_check_packet_length(
                    &mut test_ctx,
                    padding_multiple,
                    padding_min_size,
                    test_size,
                    length,
                )?;
            }

            if was_active {
                nb_inactive = 0;
            } else {
                nb_inactive += 1;
            }
        }

        if !is_success {
            return Err(padding_test_error(0xB000 | test_size as u64));
        }
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

/// Run one optimistic-ACK injection test.
/// C: `optimistic_ack_test_one` in `picoquictest/tls_api_test.c`.
fn optimistic_ack_new_hole(test_ctx: &mut TestTlsApiCtx, hole_number: u64) -> Option<u64> {
    let server = test_ctx.qserver.first_cnx_mut()?;
    let pending = &server.pkt_ctx[PacketContext::Application as usize].pending;

    for (&sequence_number, &packet_token) in pending.range(hole_number.saturating_add(1)..) {
        let Some(packet) = server.queued_packets.get(packet_token) else {
            continue;
        };
        if packet.is_ack_trap {
            return Some(sequence_number);
        }
    }

    None
}

pub fn optimistic_ack_test_one(shall_spoof: bool) -> crate::Result<()> {
    const RANDOM_PUBLIC_TEST_SEED: u64 = 0xDEAD_BEEF_CAFE_C001;

    let mut simulated_time = Instant::from_ticks(0);
    let mut initial_cid_bytes = [0x0a, 0x0a, 0x0a, 0x0a, 0, 0, 0, 0];
    if shall_spoof {
        initial_cid_bytes[7] = 0xff;
    }
    let initial_cid =
        ConnectionId::clone_from_slice(&initial_cid_bytes).ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        Some(&initial_cid),
    )
    .ok_or(crate::Error::Generic)?;

    test_ctx
        .qserver
        .set_optimistic_ack_policy(DEFAULT_HOLE_PERIOD as u32);
    test_ctx.qserver.set_qlog(".").ok();
    public_random_seed_64(RANDOM_PUBLIC_TEST_SEED, 1);

    tls_api_one_scenario_body_connect(&mut test_ctx, &mut simulated_time, 0, 0)?;
    test_ctx.stream0_target = 0;
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario)?;

    test_ctx.c_to_s_link.loss_mask = None;
    test_ctx.s_to_c_link.loss_mask = None;

    let mut nb_trials = 0;
    let mut nb_inactive = 0;
    let mut nb_holes = 0;
    let mut hole_number = 0;
    let mut nb_packet_holes_inserted = 0;
    let mut loop_error = Ok(());

    while nb_trials < 64_000
        && nb_inactive < 1024
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let mut was_active = false;
        nb_trials += 1;

        if let Err(err) = tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        ) {
            loop_error = Err(err);
            break;
        }

        if let Some(server) = test_ctx.qserver.first_cnx_mut() {
            nb_packet_holes_inserted = server.nb_packet_holes_inserted();
            if server.nb_retransmission_total > 0 {
                loop_error = Err(crate::Error::Protocol(1));
                break;
            }
        } else {
            loop_error = Err(crate::Error::Protocol(2));
            break;
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if let Some(new_hole) = optimistic_ack_new_hole(&mut test_ctx, hole_number) {
            hole_number = new_hole;
            if shall_spoof {
                let local_cid = {
                    let client = test_ctx.cnx_client();
                    client
                        .local_connection_id_lists
                        .first()
                        .and_then(|list| list.connection_ids.first())
                        .copied()
                };
                if test_ctx.cnx_client().record_pn_received(
                    PacketContext::Application,
                    local_cid,
                    hole_number,
                    simulated_time,
                ) != 0
                {
                    loop_error = Err(crate::Error::Protocol(3));
                    break;
                }
            }
            nb_holes += 1;
        }

        if test_ctx.test_finished {
            let client_empty = test_ctx
                .qclient
                .first_cnx_mut()
                .is_none_or(|cnx| cnx.is_backlog_empty());
            let server_empty = test_ctx
                .qserver
                .first_cnx_mut()
                .is_none_or(|cnx| cnx.is_backlog_empty());
            if client_empty && server_empty {
                break;
            }
        }
    }

    if shall_spoof {
        if nb_holes == 0 && nb_packet_holes_inserted == 0 {
            Err(crate::Error::Protocol(4))
        } else if nb_holes == 0 {
            Err(crate::Error::Protocol(5))
        } else if nb_packet_holes_inserted == 0 {
            Err(crate::Error::Protocol(6))
        } else if loop_error.is_ok() && test_ctx.test_finished {
            Err(crate::Error::Protocol(7))
        } else {
            Ok(())
        }
    } else {
        if nb_holes == 0 && nb_packet_holes_inserted == 0 {
            return Err(crate::Error::Protocol(7));
        }
        if nb_holes == 0 {
            return Err(crate::Error::Protocol(8));
        }
        if nb_packet_holes_inserted == 0 {
            return Err(crate::Error::Protocol(9));
        }
        loop_error?;
        tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)
            .map_err(|_| crate::Error::Protocol(10))
    }
}

/// Run one preferred-address test.
/// C: `preferred_address_test_one` in `picoquictest/tls_api_test.c`.
pub fn preferred_address_test_one(_migration_disabled: bool, _cid_zero: bool) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let preferred_v4 = SocketAddr::from(([10u8, 0, 0, 11], 5678u16));
    let mut server_params = TransportParameters::default();
    server_params.preferred_address.v4 = Some(preferred_v4);
    server_params.migration_disabled = _migration_disabled;
    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        Some(&server_params),
        None,
    )
    .ok_or(crate::Error::Generic)?;
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];
    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        &scenario,
        0,
        0,
        0,
        0,
        1_500_000,
    )?;

    {
        let client = test_ctx.cnx_client();
        assert_eq!(
            client.path_peer_addr_by_index(0),
            preferred_v4,
            "Server address at client not updated"
        );
        assert!(
            _cid_zero || client.path_local_cnxid_sequence(0) != 0,
            "Client CID not updated"
        );
        assert!(
            !client.remote_parameters.migration_disabled,
            "Migration blocked on client"
        );
    }

    if test_ctx.has_cnx_server() {
        let server = test_ctx.cnx_server();
        assert_eq!(
            server.path_local_addr_by_index(0),
            preferred_v4,
            "Server address not promoted"
        );
        assert!(
            server.path_local_cnxid_sequence(0) != 0,
            "Server CID not updated"
        );
        assert!(
            !server.local_parameters.migration_disabled,
            "Migration blocked on server"
        );
    }

    Ok(())
}

/// Run one ready-to-send test.  `option`: 1=send, 2=zfin, 3=skip, 4=zero.
/// C: `ready_to_send_test_one` in `picoquictest/tls_api_test.c`.
pub fn ready_to_send_test_one(option: u32) -> crate::Result<()> {
    if !(1..=4).contains(&option) {
        return Err(crate::Error::Generic);
    }

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx_ex2_delayed(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        None,
    )
    .ok_or(crate::Error::Generic)?;
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];
    let state = std::rc::Rc::new(std::cell::RefCell::new(ReadyToSendState {
        option,
        target: 1_000_000,
        sent: 0,
    }));

    test_ctx
        .cnx_client()
        .set_callback(Some(Box::new(ReadyToSendCallback {
            state: std::rc::Rc::clone(&state),
        })));
    test_ctx.cnx_client().start_client()?;

    let mut loss_mask = 0;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 20_000, &mut simulated_time)?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;

    test_ctx.stream0_target = 1_000_000;
    test_ctx.stream0_sent = 0;
    test_ctx.stream0_received = 0;
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    test_ctx.stream0_sent = state.borrow().sent;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 1_200_000)
}

struct ReadyToSendState {
    option: u32,
    target: usize,
    sent: usize,
}

struct ReadyToSendCallback {
    state: std::rc::Rc<std::cell::RefCell<ReadyToSendState>>,
}

impl crate::StreamDataCallback for ReadyToSendCallback {
    fn callback(
        &mut self,
        _connection: &mut Connection,
        _stream_id: u64,
        _bytes: &[u8],
        _fin_or_event: crate::CallbackEvent,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        0
    }

    fn prepare_to_send<'a>(
        &mut self,
        _connection: &mut Connection,
        stream_id: u64,
        context: &mut crate::internal::StreamDataBufferArgument<'a>,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        if stream_id != 0 {
            return -1;
        }

        let mut state = self.state.borrow_mut();
        ready_to_send_stream0_prepare(&mut state, context)
    }
}

fn ready_to_send_stream0_prepare(
    state: &mut ReadyToSendState,
    context: &mut crate::internal::StreamDataBufferArgument<'_>,
) -> i32 {
    if state.option == 3 && state.sent > 5000 {
        state.option = 1;
        return 0;
    }

    if state.sent < state.target {
        let space = context.allowed_space;
        let mut available = state.target - state.sent;
        let mut is_fin = true;

        if state.option == 4 && state.sent > 5000 {
            available = 0;
            state.option = 1;
            is_fin = false;
        } else if available > space {
            available = space;
            if state.option == 1 && space > 1 {
                available -= 1;
            }
            is_fin = false;
        } else if state.option == 2 {
            is_fin = false;
        }

        let Some(buffer) = crate::provide_stream_data_buffer(context, available, is_fin, !is_fin)
        else {
            return -1;
        };
        buffer.fill(0xa5);
        state.sent = state.sent.saturating_add(available);
        return 0;
    }

    if state.option == 2
        && state.sent == state.target
        && crate::provide_stream_data_buffer(context, 0, true, false).is_some()
    {
        return 0;
    }

    -1
}

/// Run one key-rotation test.
/// C: `key_rotation_test_one` in `picoquictest/tls_api_test.c`.
fn key_rotation_server_send_sequence(test_ctx: &mut TestTlsApiCtx) -> Option<u64> {
    test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| cnx.pkt_ctx[PacketContext::Application as usize].send_sequence)
}

fn key_rotation_ready_to_rotate(test_ctx: &mut TestTlsApiCtx, rotation_sequence: u64) -> bool {
    let app = PacketContext::Application as usize;
    let server_ready = test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| {
            cnx.pkt_ctx[app].send_sequence > rotation_sequence
                && cnx.ack_ctx[app].sack_list.last() > cnx.crypto_epoch_sequence
                && cnx.key_phase_enc == cnx.key_phase_dec
        })
        .unwrap_or(false);
    let client_ready = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| {
            cnx.ack_ctx[app].sack_list.last() > cnx.crypto_epoch_sequence
                && cnx.key_phase_enc == cnx.key_phase_dec
        })
        .unwrap_or(false);

    server_ready && client_ready
}

fn key_rotation_backlog_empty(test_ctx: &mut TestTlsApiCtx) -> bool {
    let client_empty = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.is_cnx_backlog_empty())
        .unwrap_or(true);
    let server_empty = test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| cnx.is_cnx_backlog_empty())
        .unwrap_or(true);

    client_empty && server_empty
}

fn key_rotation_inject_false_rotation(
    test_ctx: &mut TestTlsApiCtx,
    target_client: bool,
    simulated_time: Instant,
) -> crate::Result<()> {
    let (send_sequence, key_phase_dec, local_cid) = {
        let cnx = if target_client {
            test_ctx.cnx_client()
        } else {
            if !test_ctx.has_cnx_server() {
                return Err(crate::Error::Generic);
            }
            test_ctx.cnx_server()
        };
        (
            cnx.pkt_ctx[PacketContext::Application as usize].send_sequence,
            cnx.key_phase_dec,
            cnx.local_cnxid(),
        )
    };

    let mut packet = TestSimPacket::create()?;
    let mut byte_index = 1usize;
    let local_cid = local_cid.as_bytes();
    if byte_index + local_cid.len() > 128 || 128 > packet.bytes.len() {
        return Err(crate::Error::BufferTooSmall);
    }

    packet.bytes[0] = 0x3f | if key_phase_dec { 0 } else { 0x40 };
    packet.bytes[byte_index..byte_index + local_cid.len()].copy_from_slice(local_cid);
    byte_index += local_cid.len();

    let mut random_context = 0x1234_5678_9abc_def0u64 | send_sequence;
    test_random_bytes(&mut random_context, &mut packet.bytes[byte_index..128]);
    packet.length = 128;
    packet.ecn_mark = test_ctx.packet_ecn_default;

    if target_client {
        packet.addr_from = Some(test_ctx.server_addr);
        packet.addr_to = Some(test_ctx.client_addr);
        test_ctx.s_to_c_link.submit(packet, simulated_time);
    } else {
        packet.addr_from = Some(test_ctx.client_addr);
        packet.addr_to = Some(test_ctx.server_addr);
        test_ctx.c_to_s_link.submit(packet, simulated_time);
    }

    Ok(())
}

pub fn key_rotation_test_one(inject_bad_packet: u32) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, Version::InternalTest1 as u32, None)
        .ok_or(crate::Error::Generic)?;
    let mut loss_mask = 0u64;
    let mut nb_trials = 0usize;
    let mut nb_inactive = 0usize;
    let max_trials = 100_000usize;
    let mut nb_rotation = 0usize;
    let mut rotation_sequence = 100u64;
    let mut injection_sequence = 50u64;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    let scenario = [
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 4,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 12,
            previous_stream_id: 8,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 16,
            previous_stream_id: 12,
            q_len: 257,
            r_len: 1_000_000,
        },
    ];
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario)?;

    while nb_trials < max_trials
        && nb_inactive < 256
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let mut was_active = false;
        nb_trials += 1;

        if inject_bad_packet != 0
            && key_rotation_server_send_sequence(&mut test_ctx).unwrap_or(0) > injection_sequence
        {
            key_rotation_inject_false_rotation(
                &mut test_ctx,
                (inject_bad_packet >> 1) != 0,
                simulated_time,
            )?;
            injection_sequence += 50;
        }

        if key_rotation_ready_to_rotate(&mut test_ctx, rotation_sequence) {
            let send_sequence =
                key_rotation_server_send_sequence(&mut test_ctx).ok_or(crate::Error::Generic)?;
            rotation_sequence = send_sequence + 100;
            injection_sequence = send_sequence + 50;
            nb_rotation += 1;

            match nb_rotation {
                1 => test_ctx.cnx_client().start_key_rotation()?,
                2 => test_ctx.cnx_server().start_key_rotation()?,
                3 => {
                    rotation_sequence += 1_000_000_000;
                    test_ctx.cnx_client().start_key_rotation()?;
                    test_ctx.cnx_server().start_key_rotation()?;
                }
                _ => {}
            }
        }

        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if test_ctx.test_finished && key_rotation_backlog_empty(&mut test_ctx) {
            break;
        }
    }

    if nb_rotation < 3 {
        return Err(crate::Error::Generic);
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

/// Run one automatic key-rotation test.
/// C: `key_rotation_auto_one` in `picoquictest/tls_api_test.c`.
pub fn key_rotation_auto_one(epoch_length: u64, client_test: bool) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, Version::InternalTest1 as u32, None)
        .ok_or(crate::Error::Generic)?;

    if client_test {
        test_ctx.cnx_client().set_crypto_epoch_length(epoch_length);
    } else {
        test_ctx
            .qserver
            .set_default_crypto_epoch_length(epoch_length);
    }

    let scenario = [
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 4,
            q_len: 1_000_000,
            r_len: 257,
        },
    ];
    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        &scenario,
        0,
        0,
        0,
        0,
        2_000_000,
    )?;

    let (nb_packets, nb_crypto_key_rotations) = {
        let cnx = if client_test {
            test_ctx.cnx_server()
        } else {
            test_ctx.cnx_client()
        };
        (
            cnx.pkt_ctx[PacketContext::Application as usize].send_sequence,
            cnx.nb_crypto_key_rotations(),
        )
    };
    let nb_rotation_expected = nb_packets / (epoch_length + 10);
    let nb_rotation_max = nb_packets / (epoch_length - 10);

    if nb_crypto_key_rotations < nb_rotation_expected || nb_crypto_key_rotations > nb_rotation_max {
        return Err(crate::Error::Generic);
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

/// Run one heavy-loss test.  `mode`: 0=sustained, 1=interval, 2=total.
/// C: `heavy_loss_test_one` in `picoquictest/tls_api_test.c`.
pub fn heavy_loss_test_one(_mode: u32, _target_time: u64) -> crate::Result<()> {
    const HEAVY_LOSS_MASK: u64 = 0x1359_6ac7_7ca6_9531;
    const TOTAL_LOSS_MASK: u64 = u64::MAX;
    const RAMP_UP_MICROSEC: u64 = 100_000;
    const LOSS_INTERVAL_MICROSEC: u64 = 1_000_000;
    const NB_LOSS_INTERVALS: usize = 20;

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid = ConnectionId::clone_from_slice(&[0x8e, 0xfe, 0x10, 0x55, 0, 0, 0, 0])
        .ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .ok_or(crate::Error::Generic)?;

    crate::register_all_congestion_control_algorithms();
    let bbr = crate::get_congestion_algorithm("bbr").ok_or(crate::Error::Generic)?;
    test_ctx.qserver.set_default_congestion_algorithm(bbr);
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qserver.use_long_log = true;

    let sustained_scenario = [
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 4,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 12,
            previous_stream_id: 8,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 16,
            previous_stream_id: 12,
            q_len: 257,
            r_len: 1_000_000,
        },
    ];
    let interval_scenario = if _mode == 1 {
        let mut scenario = Vec::with_capacity(100);
        let mut previous_stream_id = 0u64;
        let mut stream_id = 4u64;
        for _ in 0..100 {
            scenario.push(TestApiStreamDesc {
                stream_id,
                previous_stream_id,
                q_len: 255,
                r_len: 1000,
            });
            previous_stream_id = stream_id;
            stream_id += 4;
        }
        Some(scenario)
    } else {
        None
    };
    let scenario = interval_scenario.as_deref().unwrap_or(&sustained_scenario);

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, scenario)?;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, RAMP_UP_MICROSEC)?;

    loss_mask = if _mode == 2 {
        TOTAL_LOSS_MASK
    } else {
        HEAVY_LOSS_MASK
    };
    test_ctx.c_to_s_link.loss_mask = Some(loss_mask);
    test_ctx.s_to_c_link.loss_mask = Some(loss_mask);
    for _ in 0..NB_LOSS_INTERVALS {
        if test_ctx.test_finished {
            break;
        }
        tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, LOSS_INTERVAL_MICROSEC)?;
    }

    loss_mask = 0;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, _target_time)
}

/// Run one RED (random early discard) congestion-control test.
/// C: `red_cc_algotest` in `picoquictest/tls_api_test.c`.
pub fn red_cc_algotest(
    algo_id: &'static str,
    target_time: u64,
    loss_target: u64,
) -> crate::Result<()> {
    const LATENCY_TARGET: u64 = 7_500;
    const QUEUE_MAX_RED: u64 = 40_000;
    const PICOSEC_PER_BYTE: u64 = (1_000_000u64 * 8) / 100;

    crate::register_all_congestion_control_algorithms();
    let cc_algo = crate::get_congestion_algorithm(algo_id).ok_or(crate::Error::Generic)?;
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;

    let mut initial_cid_bytes = [0x8e, 0xd0, 0xcc, 0xa1, 0x90, 6, 7, 8];
    initial_cid_bytes[4] = cc_algo.congestion_algorithm_number;
    let initial_cid =
        ConnectionId::clone_from_slice(&initial_cid_bytes).ok_or(crate::Error::Generic)?;

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .ok_or(crate::Error::Generic)?;

    test_ctx.c_to_s_link.microsec_latency = LATENCY_TARGET;
    test_ctx.c_to_s_link.picosec_per_byte = PICOSEC_PER_BYTE;
    test_ctx.s_to_c_link.microsec_latency = LATENCY_TARGET;
    test_ctx.s_to_c_link.picosec_per_byte = PICOSEC_PER_BYTE;
    test_ctx.qserver.set_default_congestion_algorithm(cc_algo);
    test_ctx.qserver.set_qlog(".")?;
    test_ctx.qserver.use_long_log = true;
    red_aqm_configure(&mut test_ctx.c_to_s_link, LATENCY_TARGET, QUEUE_MAX_RED)?;
    red_aqm_configure(&mut test_ctx.s_to_c_link, LATENCY_TARGET, QUEUE_MAX_RED)?;

    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        LATENCY_TARGET,
        &mut simulated_time,
    )?;

    let scenario = [
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 4,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 12,
            previous_stream_id: 8,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 16,
            previous_stream_id: 12,
            q_len: 257,
            r_len: 1_000_000,
        },
    ];
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;

    let observed_loss = if test_ctx.has_cnx_server() {
        test_ctx.cnx_server().nb_retransmission_total
    } else {
        u64::MAX
    };

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)?;

    assert!(
        observed_loss <= loss_target,
        "RED cc={algo_id}: expected <= {loss_target} losses, got {observed_loss}"
    );
    Ok(())
}

/// Run one TLS-API retry test.
/// C: `tls_api_retry_test_one` in `picoquictest/tls_api_test.c`.
pub fn tls_api_retry_test_one(large_client_hello: bool) -> crate::Result<()> {
    const TARGET_TIME: u64 = 230_000;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    if large_client_hello {
        test_ctx.cnx_client().test_large_chello = true;
    }
    test_ctx.qclient.set_qlog(".")?;
    test_ctx.qserver.set_cookie_mode(1);
    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;
    assert!(
        simulated_time.ticks() <= TARGET_TIME,
        "Retry test completes in {} microsec, more than {}",
        simulated_time.ticks(),
        TARGET_TIME
    );
    Ok(())
}

/// Run one TLS-retry-token test.
/// C: `tls_retry_token_test_one` in `picoquictest/tls_api_test.c`.
pub fn tls_retry_token_test_one(token_mode: u32, dup_token: bool) -> crate::Result<()> {
    const TOKEN_FILE: &str = "retry_tests_tokens.bin";

    std::fs::File::create(TOKEN_FILE).map_err(|_| crate::Error::InvalidFile)?;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    test_ctx.qclient.load_token_file(TOKEN_FILE)?;
    test_ctx.qserver.set_cookie_mode(token_mode as i32);

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        crate::internal::NB_PATH_TARGET as i32,
        1,
    )?;
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;

    if dup_token {
        let token = retry_token_stored_token(&mut test_ctx.qclient, false)?;
        let mut text = [0u8; 256];
        let decrypted = test_ctx.qserver.server_decrypt_retry_token(
            &test_ctx.client_addr,
            &token,
            &mut text,
        )?;
        assert!(
            decrypted.text_length >= 8,
            "Retry token too short, len={}",
            decrypted.text_length
        );
        let valid_until = u64::from_be_bytes(text[..8].try_into().unwrap());
        test_ctx
            .qserver
            .registered_token_check_reuse(&token, token.len(), valid_until)?;
    }

    retry_token_delete_all_connections(&mut test_ctx.qclient);
    retry_token_delete_all_connections(&mut test_ctx.qserver);
    test_ctx.qserver.check_token = true;

    test_ctx
        .qclient
        .create_connection(
            ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
            ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
            Some(&test_ctx.server_addr),
            simulated_time,
            0,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .ok_or(crate::Error::Generic)?
        .start_client()?;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    let second_original_cid_len = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.original_connection_id.len())
        .ok_or(crate::Error::Generic)?;
    let second_disconnected = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.connection_state == crate::State::Disconnected)
        .unwrap_or(true);

    if dup_token {
        if !second_disconnected {
            assert!(
                second_original_cid_len > 0,
                "Connection succeeded despite duplicate token"
            );
            tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;
        }
    } else {
        tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;
        assert_eq!(
            second_original_cid_len, 0,
            "Second retry did not use the stored token, odcil len={second_original_cid_len}"
        );
        assert!(
            retry_token_has_used_token(&test_ctx.qclient),
            "Second connection did not consume a stored retry token"
        );
    }

    test_ctx.qclient.save_tokens(TOKEN_FILE)
}

fn retry_token_delete_all_connections(quic: &mut Quic) {
    while let Some(token) = quic.first_connection().and_then(|cnx| cnx.own_token) {
        quic.delete_connection(token);
    }
}

fn retry_token_stored_token(quic: &mut Quic, mark_used: bool) -> crate::Result<Vec<u8>> {
    let idx = quic
        .stored_tokens
        .iter()
        .position(|token| {
            token.time_valid_until.ticks() > 0
                && token.sni.as_deref() == Some(TEST_SNI)
                && !token.was_used
                && !token.token.is_empty()
        })
        .ok_or(crate::Error::Generic)?;
    let token = quic.stored_tokens[idx].token.clone();
    if mark_used {
        quic.stored_tokens[idx].was_used = true;
    }
    Ok(token)
}

fn retry_token_has_used_token(quic: &Quic) -> bool {
    quic.stored_tokens
        .iter()
        .any(|token| token.sni.as_deref() == Some(TEST_SNI) && token.was_used)
}

/// Run one GREASE-quic-bit test.  `one_way=true` → asymmetric GREASE.
/// C: `grease_quic_bit_test_one` in `picoquictest/tls_api_test.c`.
pub fn grease_quic_bit_test_one(_one_way: bool) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut client_params = TransportParameters::default();
    init_transport_parameters(&mut client_params);
    client_params.do_grease_quic_bit = true;
    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        Some(&client_params),
        None,
        None,
    )
    .ok_or(crate::Error::Generic)?;
    if _one_way {
        test_ctx.qserver.one_way_grease_quic_bit = true;
    }
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];
    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)?;
    let client_greased = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|c| c.quic_bit_greased)
        .unwrap_or(false);
    let server_greased = test_ctx
        .qserver
        .first_cnx_mut()
        .map(|c| c.quic_bit_greased)
        .unwrap_or(false);
    let client_recv_0 = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|c| c.quic_bit_received_0)
        .unwrap_or(false);
    let server_recv_0 = test_ctx
        .qserver
        .first_cnx_mut()
        .map(|c| c.quic_bit_received_0)
        .unwrap_or(false);
    if _one_way {
        if client_greased {
            return Err(crate::Error::Generic);
        }
    } else if !client_greased || !server_recv_0 {
        return Err(crate::Error::Generic);
    }
    if !server_greased || !client_recv_0 {
        return Err(crate::Error::Generic);
    }
    Ok(())
}

/// Run one DDoS-amplification test.
/// C: `ddos_amplification_test_one` in `picoquictest/tls_api_test.c`.
pub fn ddos_amplification_test_one(use_0rtt: u32, do_8k: u32) -> crate::Result<()> {
    const TICKET_FILE_NAME: &str = "resume_tests_tickets.bin";
    const SCENARIO_Q_AND_R5000: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 5000,
    }];

    fn server_connection_mut(
        qserver: &mut Quic,
        initial_cid: ConnectionId,
    ) -> Option<&mut Connection> {
        qserver
            .connections
            .iter_mut()
            .find(|cnx| cnx.initial_connection_id() == initial_cid)
    }

    fn delete_all_connections(quic: &mut Quic) {
        while let Some(token) = quic.first_connection().and_then(|cnx| cnx.own_token) {
            quic.delete_connection(token);
        }
    }

    fn ensure_client_initial_queued(
        cnx: &mut Connection,
        simulated_time: Instant,
    ) -> crate::Result<()> {
        if cnx.tls_stream[0].send_queue.is_empty() {
            cnx.initialize_tls_stream(simulated_time)?;
        }
        cnx.tls_stream[0].sent_offset = 0;
        Ok(())
    }

    fn prepare_first_client_packet(
        qclient: &mut Quic,
        simulated_time: Instant,
        buffer: &mut [u8],
    ) -> crate::Result<(usize, SocketAddr, ConnectionId)> {
        let cnx = qclient.first_cnx_mut().ok_or(crate::Error::Generic)?;
        ensure_client_initial_queued(cnx, simulated_time)?;
        let initial_cid = cnx.initial_connection_id();
        let prepared = cnx.prepare_packet(simulated_time, buffer)?;
        if prepared.send_length == 0 {
            return Err(crate::Error::Generic);
        }

        Ok((prepared.send_length, prepared.addr_to, initial_cid))
    }

    let mut simulated_time = Instant::from_ticks(0);
    let proposed_version = INTEROP_VERSION_LATEST as u32;

    save_empty_tickets(TICKET_FILE_NAME, simulated_time)?;
    let mut test_ctx = tls_api_init_ctx(
        &mut simulated_time,
        proposed_version,
        Some(TICKET_FILE_NAME),
    )
    .ok_or(crate::Error::Generic)?;

    if do_8k != 0 {
        test_ctx.qserver.test_large_server_flight = true;
    }

    if use_0rtt != 0 {
        test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_Q_AND_R5000)?;

        let mut loss_mask = 0u64;
        tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
        session_resume_wait_for_ticket(&mut test_ctx, &mut simulated_time)?;
        if test_ctx.qclient.stored_tickets.is_empty() {
            return Err(crate::Error::Generic);
        }

        delete_all_connections(&mut test_ctx.qserver);
        delete_all_connections(&mut test_ctx.qclient);
        test_ctx.test_streams.clear();
        test_ctx.test_finished = false;
        test_ctx.streams_finished = false;

        let server_addr = test_ctx.server_addr;
        let cnx = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                Some(&server_addr),
                simulated_time,
                proposed_version,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .ok_or(crate::Error::Generic)?;
        cnx.start_client()?;

        test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_Q_AND_R5000)?;
    }

    let mut packet = TestSimPacket::create()?;
    let (client_packet_length, addr_to, client_initial_cid) =
        prepare_first_client_packet(&mut test_ctx.qclient, simulated_time, &mut packet.bytes)?;

    packet.length = client_packet_length;
    let data_sent_by_client = client_packet_length;
    let addr_from = test_ctx.client_addr;
    let addr_to = if addr_to.ip().is_unspecified() {
        test_ctx.server_addr
    } else {
        addr_to
    };
    let ecn = test_ctx.packet_ecn_default;
    test_ctx.qserver.incoming_packet(
        &mut packet.bytes[..packet.length],
        &addr_from,
        &addr_to,
        0,
        ecn,
        simulated_time,
    )?;

    if server_connection_mut(&mut test_ctx.qserver, client_initial_cid).is_none() {
        eprintln!(
            "ddos amplification: server connection was not created, qserver connections={}, stateless={}, head={:02x?}",
            test_ctx.qserver.current_number_connections(),
            test_ctx.qserver.pending_stateless_packets.len(),
            &packet.bytes[..packet.length.min(16)]
        );
        return Err(crate::Error::Generic);
    }

    let mut data_sent_by_server = 0usize;
    let mut nb_loops = 0u32;
    let mut nb_inactive = 0u32;

    while let Some((server_state, next_wake_time)) =
        server_connection_mut(&mut test_ctx.qserver, client_initial_cid)
            .map(|cnx| (cnx.connection_state, cnx.next_wake_time))
    {
        if server_state == State::Disconnected || nb_loops >= 1024 || nb_inactive >= 256 {
            break;
        }

        simulated_time = next_wake_time;
        packet.length = 0;
        nb_loops += 1;

        let prepared = server_connection_mut(&mut test_ctx.qserver, client_initial_cid)
            .ok_or(crate::Error::Generic)?
            .prepare_packet(simulated_time, &mut packet.bytes);

        match prepared {
            Ok(prepared) if prepared.send_length > 0 => {
                packet.length = prepared.send_length;
                data_sent_by_server = data_sent_by_server.saturating_add(prepared.send_length);
                nb_inactive = 0;
            }
            Ok(_) => {
                nb_inactive += 1;
            }
            Err(crate::Error::Disconnected) => {
                break;
            }
            Err(error) => return Err(error),
        }
    }

    let server_still_live = server_connection_mut(&mut test_ctx.qserver, client_initial_cid)
        .map(|cnx| cnx.connection_state != State::Disconnected)
        .unwrap_or(false);
    if server_still_live {
        let server_state = server_connection_mut(&mut test_ctx.qserver, client_initial_cid)
            .map(|cnx| cnx.connection_state);
        eprintln!(
            "ddos amplification: server still live state={server_state:?}, loops={nb_loops}, inactive={nb_inactive}, client_bytes={data_sent_by_client}, server_bytes={data_sent_by_server}"
        );
        return Err(crate::Error::Generic);
    }

    if data_sent_by_server > 3 * data_sent_by_client {
        eprintln!(
            "ddos amplification: server amplified {data_sent_by_server} > 3 * {data_sent_by_client}"
        );
        return Err(crate::Error::Generic);
    }

    Ok(())
}

/// Run a CNX-DDOS unit test loop.
/// C: `cnx_ddos_test_loop` in `picoquictest/tls_api_test.c`.
pub fn cnx_ddos_test_loop(nb_connections: u32, ddos_interval: u64) -> crate::Result<()> {
    const SCENARIO_Q2_AND_R2: &[TestApiStreamDesc] = &[
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 2000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 0,
            q_len: 531,
            r_len: 11000,
        },
    ];

    let mut simulated_time = Instant::from_ticks(0);
    let mut qddos = Quic::new(
        100,
        None,
        None,
        None,
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        None,
    )
    .ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        None,
    )
    .ok_or(crate::Error::Generic)?;

    let mut ddos_packets = Vec::with_capacity(nb_connections as usize);
    for i in 0..nb_connections {
        let mut ddos_packet = TestSimPacket::create()?;
        let (token, start_ret) = {
            let ddos_cnx = qddos
                .create_connection(
                    ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                    ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                    Some(&test_ctx.server_addr),
                    simulated_time,
                    0,
                    Some(TEST_SNI),
                    Some(TEST_ALPN),
                    true,
                )
                .ok_or(crate::Error::Generic)?;
            let token = ddos_cnx.own_token.ok_or(crate::Error::Generic)?;
            (token, ddos_cnx.start_client())
        };
        if let Err(error) = start_ret {
            qddos.delete_connection(token);
            return Err(error);
        }

        let (send_length, addr_to) = {
            let prepared = qddos
                .first_cnx_mut()
                .ok_or(crate::Error::Generic)?
                .prepare_packet(simulated_time, &mut ddos_packet.bytes)?;
            (prepared.send_length, prepared.addr_to)
        };
        if send_length == 0 {
            qddos.delete_connection(token);
            return Err(crate::Error::Generic);
        }
        qddos.delete_connection(token);

        let addr_be = 0x0a01_0000u32.wrapping_add(i);
        ddos_packet.length = send_length;
        ddos_packet.addr_from = Some(SocketAddr::from((
            core::net::Ipv4Addr::from(addr_be.to_be_bytes()),
            0x8421u16,
        )));
        ddos_packet.addr_to = Some(addr_to);
        ddos_packets.push(ddos_packet);
    }

    let mut ddos_time = 0u64;
    let mut nb_ddos_done = 0u32;
    let mut still_sending = false;
    let mut max_number_cnx_ctx = 0u32;
    let mut server_send_buffer = [0u8; MAX_PACKET_SIZE];

    while !ddos_packets.is_empty() || still_sending {
        let next_time = test_ctx.qserver.next_wake_time(simulated_time);
        still_sending = false;

        max_number_cnx_ctx = max_number_cnx_ctx.max(test_ctx.qserver.current_number_connections());

        if next_time < ddos_time {
            simulated_time = Instant::from_ticks(next_time);
            let send_length = {
                let prepared = test_ctx
                    .qserver
                    .prepare_next_packet(simulated_time, &mut server_send_buffer)?;
                prepared.send_length
            };
            if send_length > 0 {
                still_sending = true;
            }
        } else if let Some(mut ddos_packet) = ddos_packets.pop() {
            simulated_time = Instant::from_ticks(ddos_time);
            nb_ddos_done += 1;

            ddos_time = if nb_ddos_done >= nb_connections {
                u64::MAX
            } else {
                ddos_time.saturating_add(ddos_interval)
            };

            let addr_from = ddos_packet.addr_from.unwrap_or(test_ctx.client_addr);
            let addr_to = ddos_packet.addr_to.unwrap_or(test_ctx.server_addr);
            test_ctx.qserver.incoming_packet(
                &mut ddos_packet.bytes[..ddos_packet.length],
                &addr_from,
                &addr_to,
                0,
                0,
                simulated_time,
            )?;
            still_sending = true;
        }
    }

    let half_open_limit = test_ctx.qserver.max_half_open_retry_threshold();
    if max_number_cnx_ctx > half_open_limit {
        return Err(crate::Error::Generic);
    }

    while let Some(token) = test_ctx
        .qclient
        .first_connection()
        .and_then(|cnx| cnx.own_token)
    {
        test_ctx.qclient.delete_connection(token);
    }
    {
        let cnx = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                Some(&test_ctx.server_addr),
                simulated_time,
                Version::InternalTest1 as u32,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .ok_or(crate::Error::Generic)?;
        cnx.start_client()?;
    }

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        SCENARIO_Q2_AND_R2,
        0,
        0x0000_4281,
        0,
        20_000,
        2_000_000,
    )
}

/// Run one request-client-authentication test.
/// C: `request_client_authentication_test_one` in `picoquictest/tls_api_test.c`.
pub fn request_client_authentication_test_one(
    client_cert_file: &str,
    client_key_file: &str,
    server_cert_file: &str,
    server_key_file: &str,
    ca_cert_store_file: &str,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    let server_addr = test_ctx.server_addr;
    // Recreate qclient with client certificate
    test_ctx.qclient = Quic::new(
        8,
        Some(client_cert_file),
        Some(client_key_file),
        Some(ca_cert_store_file),
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        None,
    )
    .ok_or(crate::Error::Generic)?;
    test_ctx.qclient.enforce_client_only(true);
    // Recreate qserver requiring client authentication
    const SERVER_KEY: [u8; RESET_SECRET_SIZE] = {
        let mut k = [0u8; RESET_SECRET_SIZE];
        let mut i = 0usize;
        while i < RESET_SECRET_SIZE {
            k[i] = i as u8;
            i += 1;
        }
        k
    };
    test_ctx.qserver = Quic::new(
        8,
        Some(server_cert_file),
        Some(server_key_file),
        Some(ca_cert_store_file),
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        Some(&SERVER_KEY),
    )
    .ok_or(crate::Error::Generic)?;
    test_ctx.qserver.set_client_authentication(true);
    // Create new client connection with the rebuilt context
    {
        let cnx = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                Some(&server_addr),
                simulated_time,
                0,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .ok_or(crate::Error::Generic)?;
        cnx.start_client()?;
    }
    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    let client_ready = test_ctx.client_ready();
    let server_ready = test_ctx.server_ready();
    assert!(
        client_ready,
        "client was not ready after mTLS client-auth loop"
    );
    assert!(
        server_ready,
        "server was not ready after mTLS client-auth loop"
    );

    Ok(())
}

/// Keep-alive test implementation.
/// C: `keep_alive_test_impl` in `picoquictest/tls_api_test.c`.
fn keep_alive_default_interval(cnx: &Connection) -> crate::Duration {
    let mut idle_timeout = cnx.idle_timeout.ticks();
    if idle_timeout == 0 {
        idle_timeout = cnx
            .local_parameters
            .max_idle_timeout
            .ticks()
            .saturating_mul(1000);
    }
    let min_idle_timeout = cnx
        .paths
        .first()
        .map(|path| path.retransmit_timer.ticks().saturating_mul(3))
        .unwrap_or(0);
    if idle_timeout < min_idle_timeout {
        idle_timeout = min_idle_timeout;
    }
    crate::Duration::from_ticks(idle_timeout / 2)
}

fn keep_alive_client_disconnected(test_ctx: &mut TestTlsApiCtx) -> bool {
    test_ctx
        .qclient
        .first_cnx_mut()
        .map(|c| c.connection_state == crate::State::Disconnected)
        .unwrap_or(true)
}

pub fn keep_alive_test_impl(keep_alive: u32) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    if keep_alive != 0 {
        let interval = keep_alive_default_interval(test_ctx.cnx_client());
        test_ctx.cnx_client().enable_keep_alive(interval);
    }

    let silence_limit = crate::internal::MICROSEC_SILENCE_MAX
        .ticks()
        .saturating_mul(2);
    for _ in 0..0x10000 {
        if keep_alive_client_disconnected(&mut test_ctx) {
            break;
        }
        let mut was_active = false;
        match tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        ) {
            Ok(()) => {}
            Err(crate::Error::Disconnected)
                if keep_alive == 0 && keep_alive_client_disconnected(&mut test_ctx) => {}
            Err(e) => return Err(e),
        }
        if simulated_time.ticks() > silence_limit {
            break;
        }
    }

    if keep_alive != 0 {
        if !test_ctx.client_ready() || simulated_time.ticks() < silence_limit {
            return Err(crate::Error::Generic);
        }
    } else if !keep_alive_client_disconnected(&mut test_ctx) {
        return Err(crate::Error::Generic);
    }

    if !keep_alive_client_disconnected(&mut test_ctx) {
        tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;
    }

    Ok(())
}

/// Run one short-initial-CID test.
/// C: `short_initial_cid_test_one` in `picoquictest/tls_api_test.c`.
pub fn short_initial_cid_test_one(_cid_length: u32) -> crate::Result<()> {
    use crate::internal::ENFORCED_INITIAL_CID_LENGTH;

    let cid_len = usize::try_from(_cid_length).map_err(|_| crate::Error::Generic)?;
    if cid_len > crate::CONNECTION_ID_MAX_SIZE {
        return Err(crate::Error::Generic);
    }
    let enforced = ENFORCED_INITIAL_CID_LENGTH as usize;

    let mut cid_bytes = [0u8; crate::CONNECTION_ID_MAX_SIZE];
    for (i, byte) in cid_bytes[..cid_len].iter_mut().enumerate() {
        *byte = (i + 1) as u8;
    }
    let init_cid =
        ConnectionId::clone_from_slice(&cid_bytes[..cid_len]).ok_or(crate::Error::Generic)?;

    let mut simulated_time = Instant::from_ticks(0);
    let Some(mut test_ctx) = tls_api_init_ctx_ex(&mut simulated_time, 0, None, Some(&init_cid))
    else {
        return if cid_len < enforced {
            Ok(())
        } else {
            Err(crate::Error::Generic)
        };
    };

    let mut loss_mask = 0u64;
    let res = tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time);
    let client_ready = test_ctx.client_ready();
    let server_ready = test_ctx.server_ready();

    if cid_len < enforced {
        if client_ready || server_ready {
            Err(crate::Error::Generic)
        } else {
            Ok(())
        }
    } else {
        res?;
        if client_ready && server_ready {
            Ok(())
        } else {
            Err(crate::Error::Generic)
        }
    }
}

/// Run one CID-length test.
/// C: `cid_length_test_one` in `picoquictest/tls_api_test.c`.
pub fn cid_length_test_one(client_cid_length: u32) -> crate::Result<()> {
    const SCENARIO_Q_AND_R: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    fn remote_cid_len_on_first_path(cnx: &Connection) -> crate::Result<usize> {
        let path = cnx.paths.first().ok_or(crate::Error::Generic)?;
        let tuple = path.tuples.first().ok_or(crate::Error::Generic)?;
        let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
        cnx.remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == path.unique_path_id)
            .and_then(|stash| stash.connection_ids.get(cid_index))
            .map(|cid| cid.connection_id.len())
            .ok_or(crate::Error::Generic)
    }

    let cid_length = u8::try_from(client_cid_length).map_err(|_| crate::Error::Generic)?;
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, Version::InternalTest1 as u32, None)
        .ok_or(crate::Error::Generic)?;

    if let Some(token) = test_ctx
        .qclient
        .first_cnx_mut()
        .and_then(|cnx| cnx.own_token)
    {
        test_ctx.qclient.delete_connection(token);
    }

    test_ctx
        .qclient
        .set_default_connection_id_length(cid_length)
        .map_err(|_| crate::Error::Generic)?;

    {
        let cnx = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                Some(&test_ctx.server_addr),
                simulated_time,
                Version::InternalTest1 as u32,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .ok_or(crate::Error::Generic)?;
        cnx.start_client()?;
    }

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_Q_AND_R)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;

    assert_eq!(
        test_ctx.cnx_client().local_cnxid().len(),
        cid_length as usize,
        "client local CID length mismatch"
    );
    assert_eq!(
        remote_cid_len_on_first_path(test_ctx.cnx_server())?,
        cid_length as usize,
        "server remote CID length mismatch"
    );

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 100_000)
}

// ---------------------------------------------------------------------------
// Warptest helpers.

/// Warptest scenario parameters.
/// C: `warptest_spec_t` in `picoquictest/warptest.c`.
#[derive(Default)]
pub struct WarptestSpec {
    /// Congestion-control algorithm id string, or `None` for default.
    pub ccalgo_id: Option<&'static str>,
    /// Enable audio streams.  C: `do_audio`.
    pub do_audio: bool,
    /// Enable video streams.  C: `do_video`.
    pub do_video: bool,
    /// Bulk data size (bytes).  C: `data_size`.
    pub data_size: usize,
    /// Datagram data size (bytes).  C: `datagram_data_size`.
    pub datagram_data_size: usize,
    /// Maximum number of client-initiated streams.  C: `max_streams_client`.
    pub max_streams_client: u64,
    /// Maximum number of server-initiated streams.  C: `max_streams_server`.
    pub max_streams_server: u64,
    /// Per-stream flow-control window (bytes).  C: `max_stream_data`.
    pub max_stream_data: u64,
    /// Simulated link bandwidth (Mbps).  C: `bandwidth`.
    pub bandwidth: f64,
}

/// Run one warptest scenario.
/// C: `warptest_one` in `picoquictest/warptest.c`.
pub fn warptest_one(_warptest_id: u32, _spec: &WarptestSpec) -> crate::Result<()> {
    const WARPTEST_DURATION: u64 = 10_000_000;
    const WARPTEST_AUDIO_PERIOD: u64 = 20_000;
    const WARPTEST_VIDEO_PERIOD: u64 = 33_333;
    const WARPTEST_HEADER_SIZE: usize = 21;
    const WARPTEST_DATA_FRAME_SIZE: usize = 0x4000;
    const WARPTEST_TYPE_DATA: u8 = 0;
    const WARPTEST_TYPE_AUDIO: u8 = 1;
    const WARPTEST_TYPE_VIDEO: u8 = 2;

    #[derive(Default, Clone, Copy)]
    struct MediaStats {
        nb_frames: u64,
        sum_delays: u64,
        sum_square_delays: u64,
        max_delay: u64,
    }

    fn check_stats(stats: MediaStats, expected: u64) -> crate::Result<()> {
        if stats.nb_frames != expected {
            return Err(crate::Error::Generic);
        }
        if stats.nb_frames > 0 {
            let average = stats
                .sum_delays
                .checked_div(stats.nb_frames)
                .ok_or(crate::Error::Generic)?;
            let variance = stats
                .sum_square_delays
                .checked_div(stats.nb_frames)
                .unwrap_or(0)
                .saturating_sub(average * average);
            let sigma = (variance as f64).sqrt() as u64;
            if average > 25_000 || sigma > 12_500 || stats.max_delay > 100_000 {
                return Err(crate::Error::Generic);
            }
        }
        Ok(())
    }

    fn format_media_frame(
        message_type: u8,
        message_size: usize,
        frame_number: u64,
        sent_time: u64,
    ) -> Vec<u8> {
        let mut frame = vec![message_type; message_size];
        frame[0] = message_type;
        frame[1..5].copy_from_slice(&(message_size as u32).to_be_bytes());
        frame[5..13].copy_from_slice(&frame_number.to_be_bytes());
        frame[13..21].copy_from_slice(&sent_time.to_be_bytes());
        frame
    }

    fn queue_media_frame(
        test_ctx: &mut TestTlsApiCtx,
        message_type: u8,
        message_size: usize,
        frame_number: u64,
        sent_time: u64,
        priority: u8,
    ) -> crate::Result<()> {
        if message_size < WARPTEST_HEADER_SIZE {
            return Err(crate::Error::Generic);
        }
        let stream_id = test_ctx.cnx_client().get_next_local_stream_id(true);
        let frame = format_media_frame(message_type, message_size, frame_number, sent_time);
        {
            let cnx = test_ctx.cnx_client();
            cnx.add_to_stream(stream_id, &frame, true)?;
            cnx.set_stream_priority(stream_id, priority)?;
            cnx.next_wake_time = Instant::from_ticks(sent_time);
        }
        Ok(())
    }

    fn complete_stream_bytes(
        stream: &crate::internal::StreamHead,
    ) -> crate::Result<Option<Vec<u8>>> {
        if !stream.fin_received {
            return Ok(None);
        }

        let mut bytes = Vec::new();
        let mut next_offset = 0u64;
        let mut node_token = stream.stream_data_tree.first();
        while let Some(token) = node_token {
            let data_token = *stream
                .stream_data_tree
                .get(token)
                .ok_or(crate::Error::Generic)?;
            let data = stream
                .stream_data_nodes
                .get(data_token)
                .ok_or(crate::Error::Generic)?;
            if data.offset > next_offset {
                return Ok(None);
            }
            let start = next_offset.saturating_sub(data.offset) as usize;
            if start < data.length {
                bytes.extend_from_slice(&data.data[start..data.length]);
                next_offset = data.offset + data.length as u64;
            }
            node_token = stream.stream_data_tree.next(token);
        }

        let fin_offset = usize::try_from(stream.fin_offset).map_err(|_| crate::Error::Generic)?;
        if bytes.len() < fin_offset {
            return Ok(None);
        }
        bytes.truncate(fin_offset);
        Ok(Some(bytes))
    }

    fn complete_media_frame(
        stream: &crate::internal::StreamHead,
    ) -> crate::Result<Option<Vec<u8>>> {
        let Some(mut bytes) = complete_stream_bytes(stream)? else {
            return Ok(None);
        };

        if bytes.len() < WARPTEST_HEADER_SIZE {
            return Ok(None);
        }
        let message_size = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
        if message_size < WARPTEST_HEADER_SIZE || stream.fin_offset != message_size as u64 {
            return Err(crate::Error::Generic);
        }
        if bytes.len() < message_size {
            return Ok(None);
        }
        bytes.truncate(message_size);
        Ok(Some(bytes))
    }

    fn collect_media_stats(
        test_ctx: &mut TestTlsApiCtx,
        current_time: u64,
        processed_streams: &mut Vec<u64>,
        audio_stats: &mut MediaStats,
        video_stats: &mut MediaStats,
    ) -> crate::Result<()> {
        if !test_ctx.has_cnx_server() {
            return Ok(());
        }

        let cnx = test_ctx.cnx_server();
        for stream in cnx.streams.iter() {
            let stream_id = stream.stream_id;
            if processed_streams.contains(&stream_id)
                || crate::stream::StreamId(stream_id).is_bidir()
            {
                continue;
            }
            let Some(frame) = complete_media_frame(stream)? else {
                continue;
            };
            let message_type = frame[0];
            let frame_number = u64::from_be_bytes([
                frame[5], frame[6], frame[7], frame[8], frame[9], frame[10], frame[11], frame[12],
            ]);
            let sent_time = u64::from_be_bytes([
                frame[13], frame[14], frame[15], frame[16], frame[17], frame[18], frame[19],
                frame[20],
            ]);
            let stats = if message_type == WARPTEST_TYPE_AUDIO {
                &mut *audio_stats
            } else if message_type == WARPTEST_TYPE_VIDEO {
                &mut *video_stats
            } else {
                return Err(crate::Error::Generic);
            };
            if frame_number != stats.nb_frames {
                return Err(crate::Error::Generic);
            }
            let delay = current_time.saturating_sub(sent_time);
            stats.nb_frames += 1;
            stats.sum_delays += delay;
            stats.sum_square_delays += delay * delay;
            stats.max_delay = stats.max_delay.max(delay);
            processed_streams.push(stream_id);
        }
        Ok(())
    }

    fn queue_bulk_data_stream(
        test_ctx: &mut TestTlsApiCtx,
        requested: usize,
        sent_time: u64,
    ) -> crate::Result<Option<(u64, u64)>> {
        if requested == 0 {
            return Ok(None);
        }

        let stream_id = test_ctx.cnx_client().get_next_local_stream_id(false);
        let mut bytes = Vec::with_capacity(requested);
        let mut bytes_queued = 0usize;
        let mut frame_number = 0u64;
        while bytes_queued < requested {
            let remaining = requested - bytes_queued;
            #[allow(clippy::manual_clamp)]
            let message_size = remaining
                .min(WARPTEST_DATA_FRAME_SIZE)
                .max(WARPTEST_HEADER_SIZE);
            let frame =
                format_media_frame(WARPTEST_TYPE_DATA, message_size, frame_number, sent_time);
            bytes.extend_from_slice(&frame);
            bytes_queued += message_size;
            frame_number += 1;
        }

        let cnx = test_ctx.cnx_client();
        cnx.add_to_stream(stream_id, &bytes, true)?;
        cnx.set_stream_priority(stream_id, 7)?;
        cnx.next_wake_time = Instant::from_ticks(sent_time);
        Ok(Some((stream_id, bytes_queued as u64)))
    }

    fn bulk_data_stream_sent(test_ctx: &mut TestTlsApiCtx, stream_id: u64) -> bool {
        test_ctx
            .cnx_client()
            .streams
            .iter()
            .find(|stream| stream.stream_id == stream_id)
            .map(|stream| stream.fin_sent && stream.send_queue.is_empty())
            .unwrap_or(false)
    }

    fn verify_complete_bulk_data_stream(
        test_ctx: &mut TestTlsApiCtx,
        stream_id: u64,
        expected_bytes: u64,
    ) -> crate::Result<()> {
        if !test_ctx.has_cnx_server() {
            return Ok(());
        }

        let cnx = test_ctx.cnx_server();
        let Some(stream) = cnx
            .streams
            .iter()
            .find(|stream| stream.stream_id == stream_id)
        else {
            return Ok(());
        };
        let Some(bytes) = complete_stream_bytes(stream)? else {
            return Ok(());
        };

        let mut offset = 0usize;
        let mut frame_number = 0u64;
        let mut bytes_received = 0u64;
        while bytes_received < expected_bytes {
            if bytes.len().saturating_sub(offset) < WARPTEST_HEADER_SIZE {
                return Err(crate::Error::Generic);
            }
            if bytes[offset] != WARPTEST_TYPE_DATA {
                return Err(crate::Error::Generic);
            }
            let message_size = u32::from_be_bytes([
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
            ]) as usize;
            if message_size < WARPTEST_HEADER_SIZE {
                return Err(crate::Error::Generic);
            }
            let end = offset
                .checked_add(message_size)
                .ok_or(crate::Error::Generic)?;
            if end > bytes.len() {
                return Err(crate::Error::Generic);
            }
            let received_frame_number = u64::from_be_bytes([
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
                bytes[offset + 12],
            ]);
            if received_frame_number != frame_number
                || bytes[offset + WARPTEST_HEADER_SIZE..end]
                    .iter()
                    .any(|byte| *byte != WARPTEST_TYPE_DATA)
            {
                return Err(crate::Error::Generic);
            }
            bytes_received = bytes_received.saturating_add(message_size as u64);
            offset = end;
            frame_number += 1;
        }

        if offset != bytes.len() {
            return Err(crate::Error::Generic);
        }
        Ok(())
    }

    fn rewind_tls_send_queues(cnx: &mut crate::Connection) {
        for stream in &mut cnx.tls_stream {
            if let Some(front) = stream.send_queue.front() {
                let front_end = front.offset.saturating_add(front.bytes.len() as u64);
                if stream.sent_offset >= front_end {
                    stream.sent_offset = front.offset;
                }
            }
        }
    }

    #[derive(Copy, Clone, PartialEq, Eq)]
    enum WarptestAction {
        ClientDeparture,
        ServerDeparture,
        ClientArrival,
        ServerArrival,
        ClientAdmission,
        ServerAdmission,
        MediaFrame,
    }

    fn run_media_step(
        test_ctx: &mut TestTlsApiCtx,
        simulated_time: &mut Instant,
        next_audio_time: &mut u64,
        next_video_time: &mut u64,
        frames_sent_audio: &mut u64,
        frames_sent_video: &mut u64,
        frames_to_send_audio: u64,
        frames_to_send_video: u64,
        audio_stats: &mut MediaStats,
        video_stats: &mut MediaStats,
        processed_streams: &mut Vec<u64>,
        is_active: &mut bool,
    ) -> crate::Result<()> {
        let mut next_time = simulated_time.ticks().saturating_add(120_000_000);
        let mut next_action: Option<WarptestAction> = None;

        let c_arrival = test_ctx
            .s_to_c_link
            .next_arrival(Instant::from_ticks(next_time));
        if c_arrival < next_time {
            next_time = c_arrival;
            next_action = Some(WarptestAction::ClientArrival);
        }
        let s_arrival = test_ctx
            .c_to_s_link
            .next_arrival(Instant::from_ticks(next_time));
        if s_arrival < next_time {
            next_time = s_arrival;
            next_action = Some(WarptestAction::ServerArrival);
        }
        let c_admission = test_ctx
            .s_to_c_link
            .next_admission(*simulated_time, Instant::from_ticks(next_time));
        if c_admission < next_time {
            next_time = c_admission;
            next_action = Some(WarptestAction::ClientAdmission);
        }
        let s_admission = test_ctx
            .c_to_s_link
            .next_admission(*simulated_time, Instant::from_ticks(next_time));
        if s_admission < next_time {
            next_time = s_admission;
            next_action = Some(WarptestAction::ServerAdmission);
        }
        if let Some(t) = test_ctx
            .qclient
            .first_cnx_mut()
            .filter(|c| c.connection_state != crate::State::Disconnected)
            .map(|c| c.next_wake_time.ticks())
            && t < next_time
        {
            next_time = t;
            next_action = Some(WarptestAction::ClientDeparture);
        }
        if let Some(t) = test_ctx
            .qserver
            .first_cnx_mut()
            .filter(|c| c.connection_state != crate::State::Disconnected)
            .map(|c| c.next_wake_time.ticks())
            && t < next_time
        {
            next_time = t;
            next_action = Some(WarptestAction::ServerDeparture);
        }

        if test_ctx.client_ready() {
            let media_time = (*next_audio_time).min(*next_video_time);
            if media_time < next_time {
                next_time = media_time;
                next_action = Some(WarptestAction::MediaFrame);
            }
        }

        if next_time > simulated_time.ticks() {
            *simulated_time = Instant::from_ticks(next_time);
        }

        match next_action {
            Some(WarptestAction::MediaFrame) => {
                let now = simulated_time.ticks();
                if *frames_sent_audio < frames_to_send_audio && *next_audio_time <= now {
                    queue_media_frame(
                        test_ctx,
                        WARPTEST_TYPE_AUDIO,
                        32,
                        *frames_sent_audio,
                        *next_audio_time,
                        3,
                    )?;
                    *frames_sent_audio += 1;
                    *next_audio_time = if *frames_sent_audio >= frames_to_send_audio {
                        u64::MAX
                    } else {
                        next_audio_time.saturating_add(WARPTEST_AUDIO_PERIOD)
                    };
                    *is_active = true;
                }
                if *frames_sent_video < frames_to_send_video && *next_video_time <= now {
                    let message_size = if (*frames_sent_video).is_multiple_of(100) {
                        0x8000
                    } else {
                        0x800
                    };
                    queue_media_frame(
                        test_ctx,
                        WARPTEST_TYPE_VIDEO,
                        message_size,
                        *frames_sent_video,
                        *next_video_time,
                        5,
                    )?;
                    *frames_sent_video += 1;
                    *next_video_time = if *frames_sent_video >= frames_to_send_video {
                        u64::MAX
                    } else {
                        next_video_time.saturating_add(WARPTEST_VIDEO_PERIOD)
                    };
                    *is_active = true;
                }
            }
            Some(WarptestAction::ClientArrival) => {
                let t = *simulated_time;
                if let Some(mut pkt) = test_ctx.s_to_c_link.dequeue(t) {
                    let addr_from = pkt.addr_from.unwrap_or(test_ctx.server_addr);
                    let addr_to = pkt.addr_to.unwrap_or(test_ctx.client_addr);
                    let ecn = pkt.ecn_mark;
                    test_ctx.qclient.incoming_packet(
                        &mut pkt.bytes[..pkt.length],
                        &addr_from,
                        &addr_to,
                        0,
                        ecn,
                        t,
                    )?;
                    *is_active = true;
                }
            }
            Some(WarptestAction::ServerArrival) => {
                let t = *simulated_time;
                if let Some(mut pkt) = test_ctx.c_to_s_link.dequeue(t) {
                    let addr_from = pkt.addr_from.unwrap_or(test_ctx.client_addr);
                    let addr_to = pkt.addr_to.unwrap_or(test_ctx.server_addr);
                    let ecn = pkt.ecn_mark;
                    test_ctx.qserver.incoming_packet(
                        &mut pkt.bytes[..pkt.length],
                        &addr_from,
                        &addr_to,
                        0,
                        ecn,
                        t,
                    )?;
                    collect_media_stats(
                        test_ctx,
                        t.ticks(),
                        processed_streams,
                        audio_stats,
                        video_stats,
                    )?;
                    *is_active = true;
                }
            }
            Some(WarptestAction::ClientAdmission) => {
                test_ctx.s_to_c_link.admit_pending(*simulated_time);
            }
            Some(WarptestAction::ServerAdmission) => {
                test_ctx.c_to_s_link.admit_pending(*simulated_time);
            }
            Some(WarptestAction::ClientDeparture) => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let prep = test_ctx.qclient.first_cnx_mut().and_then(|c| {
                    rewind_tls_send_queues(c);
                    c.prepare_packet(*simulated_time, &mut buf).ok()
                });
                if let Some(pp) = prep
                    && pp.send_length > 0
                {
                    let mut pkt = TestSimPacket::create()?;
                    pkt.addr_from = Some(if pp.addr_from.ip().is_unspecified() {
                        test_ctx.client_addr
                    } else {
                        pp.addr_from
                    });
                    pkt.addr_to = Some(pp.addr_to);
                    pkt.ecn_mark = test_ctx.packet_ecn_default;
                    pkt.length = pp.send_length;
                    pkt.bytes[..pp.send_length].copy_from_slice(&buf[..pp.send_length]);
                    test_ctx.c_to_s_link.submit(pkt, *simulated_time);
                    *is_active = true;
                }
            }
            Some(WarptestAction::ServerDeparture) => {
                let mut buf = [0u8; MAX_PACKET_SIZE];
                let prep = test_ctx.qserver.first_cnx_mut().and_then(|c| {
                    rewind_tls_send_queues(c);
                    c.prepare_packet(*simulated_time, &mut buf).ok()
                });
                if let Some(pp) = prep
                    && pp.send_length > 0
                {
                    let mut pkt = TestSimPacket::create()?;
                    pkt.addr_from = Some(if pp.addr_from.ip().is_unspecified() {
                        test_ctx.server_addr
                    } else {
                        pp.addr_from
                    });
                    pkt.addr_to = Some(pp.addr_to);
                    pkt.ecn_mark = test_ctx.packet_ecn_default;
                    pkt.length = pp.send_length;
                    pkt.bytes[..pp.send_length].copy_from_slice(&buf[..pp.send_length]);
                    test_ctx.s_to_c_link.submit(pkt, *simulated_time);
                    *is_active = true;
                }
            }
            None => {}
        }

        Ok(())
    }

    fn queue_datagram_load(
        test_ctx: &mut TestTlsApiCtx,
        simulated_time: Instant,
        requested: usize,
        sent: &mut usize,
    ) -> crate::Result<()> {
        while *sent < requested {
            let max_payload = test_ctx
                .cnx_client()
                .paths
                .first()
                .map(|path| path.send_mtu.saturating_sub(64).max(1))
                .unwrap_or(MAX_PACKET_SIZE.saturating_sub(64).max(1));
            let chunk = (requested - *sent).min(max_payload);
            let payload = vec![b'd'; chunk];
            test_ctx.cnx_client().queue_datagram_frame(&payload)?;
            *sent += chunk;
        }
        let cnx = test_ctx.cnx_client();
        cnx.mark_datagram_ready(!cnx.datagrams.is_empty())?;
        cnx.next_wake_time = simulated_time;
        Ok(())
    }

    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xed, 0x1a, 0x1d, 0x18, _warptest_id as u8, 0, 0, 0])
            .ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex_named(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some("picoquic_mediatest"),
        None,
        Some(&initial_cid),
        false,
    )?;
    crate::register_all_congestion_control_algorithms();

    if let Some(algo_id) = _spec.ccalgo_id {
        let algo = crate::get_congestion_algorithm(algo_id).ok_or(crate::Error::Generic)?;
        test_ctx
            .qclient
            .set_default_congestion_algorithm_ex(algo, None);
        test_ctx
            .qserver
            .set_default_congestion_algorithm_ex(algo, None);
        test_ctx
            .cnx_client()
            .set_congestion_algorithm_ex(algo, None);
    }

    let mut tp = TransportParameters::default();
    crate::internal::init_transport_parameters(&mut tp);
    tp.max_idle_timeout = crate::Duration::from_ticks(30_000);
    tp.max_packet_size = MAX_PACKET_SIZE as u32;
    tp.active_connection_id_limit = 4;
    tp.max_ack_delay = 10_000;
    tp.min_ack_delay = crate::Duration::from_ticks(1000);
    tp.max_datagram_frame_size = MAX_PACKET_SIZE as u32;
    tp.initial_max_stream_id_bidir = 512;
    tp.initial_max_stream_id_unidir = if _spec.max_streams_client == 0 {
        16
    } else {
        _spec.max_streams_client
    };
    tp.initial_max_stream_data_uni = if _spec.max_stream_data == 0 {
        65_535
    } else {
        _spec.max_stream_data
    };
    test_ctx.qclient.set_default_tp(&tp)?;
    test_ctx.qserver.set_default_tp(&tp)?;
    test_ctx.cnx_client().set_transport_parameters(&tp);
    test_ctx
        .cnx_client()
        .initialize_tls_stream(simulated_time)?;

    let link_rate = if _spec.bandwidth > 0.0 {
        _spec.bandwidth
    } else {
        0.01
    };
    test_ctx.c_to_s_link.picosec_per_byte = (8000.0 / link_rate * 1.024 * 1.024) as u64;
    test_ctx.s_to_c_link.picosec_per_byte = test_ctx.c_to_s_link.picosec_per_byte;

    let mut handshake_steps = 0;
    let mut handshake_inactive = 0;
    let mut handshake_audio_time = u64::MAX;
    let mut handshake_video_time = u64::MAX;
    let mut handshake_audio_sent = 0u64;
    let mut handshake_video_sent = 0u64;
    let mut handshake_audio_stats = MediaStats::default();
    let mut handshake_video_stats = MediaStats::default();
    let mut handshake_processed_streams = Vec::new();
    while handshake_steps < 100_000
        && handshake_inactive < 512
        && simulated_time.ticks() < 30_000_000
        && (!test_ctx.client_ready() || !test_ctx.server_ready())
    {
        handshake_steps += 1;
        let mut is_active = false;
        run_media_step(
            &mut test_ctx,
            &mut simulated_time,
            &mut handshake_audio_time,
            &mut handshake_video_time,
            &mut handshake_audio_sent,
            &mut handshake_video_sent,
            0,
            0,
            &mut handshake_audio_stats,
            &mut handshake_video_stats,
            &mut handshake_processed_streams,
            &mut is_active,
        )?;

        if is_active {
            handshake_inactive = 0;
        } else {
            handshake_inactive += 1;
        }
    }
    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        return Err(crate::Error::Generic);
    }

    let bulk_data_stream =
        queue_bulk_data_stream(&mut test_ctx, _spec.data_size, simulated_time.ticks())?;
    let frames_to_send_data = bulk_data_stream.map(|(_, bytes)| bytes).unwrap_or(0);
    let frames_to_send_audio = if _spec.do_audio {
        WARPTEST_DURATION / WARPTEST_AUDIO_PERIOD
    } else {
        0
    };
    let frames_to_send_video = if _spec.do_video {
        WARPTEST_DURATION / WARPTEST_VIDEO_PERIOD
    } else {
        0
    };

    let mut frames_sent_audio = 0u64;
    let mut frames_sent_video = 0u64;
    let mut datagram_sent = 0usize;
    let mut audio_stats = MediaStats::default();
    let mut video_stats = MediaStats::default();
    let mut nb_steps = 0;
    let mut nb_inactive = 0;
    let mut next_audio_time = if frames_to_send_audio == 0 {
        u64::MAX
    } else {
        simulated_time.ticks()
    };
    let mut next_video_time = if frames_to_send_video == 0 {
        u64::MAX
    } else {
        simulated_time.ticks()
    };
    let mut processed_streams = Vec::new();
    let mut bulk_data_sent = bulk_data_stream.is_none();

    if _spec.datagram_data_size > 0 {
        queue_datagram_load(
            &mut test_ctx,
            simulated_time,
            _spec.datagram_data_size,
            &mut datagram_sent,
        )?;
    }

    while nb_steps < 100_000 && nb_inactive < 512 && simulated_time.ticks() < 30_000_000 {
        nb_steps += 1;
        let mut is_active = false;
        run_media_step(
            &mut test_ctx,
            &mut simulated_time,
            &mut next_audio_time,
            &mut next_video_time,
            &mut frames_sent_audio,
            &mut frames_sent_video,
            frames_to_send_audio,
            frames_to_send_video,
            &mut audio_stats,
            &mut video_stats,
            &mut processed_streams,
            &mut is_active,
        )?;

        if let Some((stream_id, _)) = bulk_data_stream
            && !bulk_data_sent
            && bulk_data_stream_sent(&mut test_ctx, stream_id)
        {
            bulk_data_sent = true;
        }

        let datagram_done = if _spec.datagram_data_size == 0 {
            true
        } else {
            datagram_sent >= _spec.datagram_data_size && test_ctx.cnx_client().datagrams.is_empty()
        };
        let done = bulk_data_sent
            && frames_sent_audio == frames_to_send_audio
            && frames_sent_video == frames_to_send_video
            && audio_stats.nb_frames == frames_to_send_audio
            && video_stats.nb_frames == frames_to_send_video
            && datagram_done;
        if done {
            break;
        }

        if is_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    if !bulk_data_sent
        || frames_sent_audio != frames_to_send_audio
        || frames_sent_video != frames_to_send_video
        || audio_stats.nb_frames != frames_to_send_audio
        || video_stats.nb_frames != frames_to_send_video
        || datagram_sent < _spec.datagram_data_size
        || (_spec.datagram_data_size > 0 && !test_ctx.cnx_client().datagrams.is_empty())
    {
        return Err(crate::Error::Generic);
    }
    if let Some((stream_id, _)) = bulk_data_stream {
        verify_complete_bulk_data_stream(&mut test_ctx, stream_id, frames_to_send_data)?;
    }
    if _spec.do_audio {
        check_stats(audio_stats, frames_to_send_audio)?;
    }
    if _spec.do_video {
        check_stats(video_stats, frames_to_send_video)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Wifitest helpers.

/// One suspend/resume event in a wifi-test scenario.
/// C: `wifi_test_suspension_t` in `picoquictest/wifitest.c`.
#[derive(Debug, Clone, Copy)]
pub struct WifiTestSuspension {
    /// Simulation time (µs) at which to suspend the link.  C: `suspend_time`.
    pub suspend_time: u64,
    /// Duration of the suspension (µs).  C: `suspend_interval`.
    pub suspend_interval: u64,
}

/// Parameters for a wifi-test run.
/// C: `wifi_test_spec_t` in `picoquictest/wifitest.c`.
pub struct WifiTestSpec {
    /// One-way latency (µs) on both directions.  C: `latency`.
    pub latency: u64,
    /// Ordered suspend/resume events.  C: `suspension` + `nb_suspend`.
    pub suspension: &'static [WifiTestSuspension],
    /// Congestion-control algorithm id string.  C: `ccalgo`.
    pub ccalgo_id: &'static str,
    /// Optional CC algorithm option string.  C: `cc_algo_option`.
    pub cc_algo_option: Option<&'static str>,
    /// Expected upper bound on simulated time at test completion (µs).  C: `target_time`.
    pub target_time: u64,
    /// Simulate a receive-path block during suspension.  C: `simulate_receive_block`.
    pub simulate_receive_block: bool,
    /// Maximum queue delay on the sim link (µs).  C: `queue_max_delay`.
    pub queue_max_delay: u64,
}

struct WifiCubicCongestionControl;

impl crate::CongestionControl for WifiCubicCongestionControl {
    fn alg_init(
        &self,
        _connection: &mut Connection,
        path_x: &mut crate::internal::Path,
        option_string: Option<&str>,
        current_time: Instant,
    ) {
        crate::cubic::CubicState::init(path_x, option_string, current_time.ticks());
    }

    fn alg_notify(
        &self,
        connection: &mut Connection,
        path_x: &mut crate::internal::Path,
        notification: crate::CongestionNotification,
        ack_state: &crate::PerAckState,
        current_time: Instant,
    ) {
        if path_x.congestion_alg_state.is_none() {
            crate::cubic::CubicState::init(path_x, None, current_time.ticks());
        }

        let Some(boxed_state) = path_x.congestion_alg_state.take() else {
            return;
        };

        let mut cubic_state = match boxed_state.downcast::<crate::cubic::CubicState>() {
            Ok(state) => state,
            Err(boxed_state) => {
                path_x.congestion_alg_state = Some(boxed_state);
                return;
            }
        };

        cubic_state.notify(
            connection,
            path_x,
            notification,
            ack_state,
            current_time.ticks(),
        );
        path_x.congestion_alg_state = Some(cubic_state);
    }

    fn alg_delete(&self, path_x: &mut crate::internal::Path) {
        path_x.congestion_alg_state = None;
    }

    fn alg_observe(&self, path_x: &crate::internal::Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|state| state.downcast_ref::<crate::cubic::CubicState>())
            .map(|state| state.observe())
    }
}

static WIFI_CUBIC_CONTROL: WifiCubicCongestionControl = WifiCubicCongestionControl;
static WIFI_CUBIC_ALGORITHM: crate::CongestionAlgorithm = crate::CongestionAlgorithm {
    congestion_algorithm_id: "cubic",
    congestion_algorithm_number: 2,
    ecn_mark: crate::ECN_ECT_0,
    algorithm: &WIFI_CUBIC_CONTROL,
};

fn wifi_congestion_algorithm(
    ccalgo_id: &str,
) -> crate::Result<&'static crate::CongestionAlgorithm> {
    if ccalgo_id == "cubic" {
        Ok(&WIFI_CUBIC_ALGORITHM)
    } else {
        crate::register_all_congestion_control_algorithms();
        crate::get_congestion_algorithm(ccalgo_id).ok_or(crate::Error::Generic)
    }
}

fn wifi_set_connection_congestion_algorithm(
    connection: &mut Connection,
    algo: &'static crate::CongestionAlgorithm,
    option_string: Option<&str>,
    current_time: Instant,
) {
    connection.set_congestion_algorithm_ex(algo, option_string);
    let _ = current_time;
}

/// Run one wifi-test scenario.  `test_id` is the C enum discriminant used to
/// seed the initial connection ID.
/// C: `wifi_test_one` in `picoquictest/wifitest.c`.
pub fn wifi_test_one(_test_id: u32, _spec: &WifiTestSpec) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;

    // initial_cid = { {0x81, 0xf1, test_id, 0, 0, 0, 0, 0}, 8 }
    let cid_bytes = [0x81u8, 0xf1, _test_id as u8, 0, 0, 0, 0, 0];
    let initial_cid = ConnectionId::clone_from_slice(&cid_bytes).ok_or(crate::Error::Generic)?;

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        Some(&initial_cid),
    )
    .ok_or(crate::Error::Generic)?;

    let algo = wifi_congestion_algorithm(_spec.ccalgo_id)?;
    test_ctx
        .qserver
        .set_default_congestion_algorithm_ex(algo, _spec.cc_algo_option);
    wifi_set_connection_congestion_algorithm(
        test_ctx.cnx_client(),
        algo,
        _spec.cc_algo_option,
        simulated_time,
    );

    test_ctx.c_to_s_link.microsec_latency = _spec.latency;
    test_ctx.s_to_c_link.microsec_latency = _spec.latency;
    test_ctx.immediate_exit = true;
    test_ctx.cnx_client().set_pmtud_required(true);
    // picoquic_set_qlog is loglib (out of v1 scope) — skip
    test_ctx.qserver.set_log_level(1);
    test_ctx.qclient.set_log_level(1);
    test_ctx.qclient.use_long_log = true;
    test_ctx.qserver.use_long_log = true;
    // picoquic_start_client_cnx already called inside tls_api_init_ctx_ex

    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        _spec.queue_max_delay,
        &mut simulated_time,
    )?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;

    let scenario = [
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 0,
            q_len: 4,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 12,
            previous_stream_id: 0,
            q_len: 8,
            r_len: 1_000_000,
        },
    ];
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario)?;

    for susp in _spec.suspension {
        let wait_us = susp.suspend_time.saturating_sub(simulated_time.ticks());
        tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, wait_us)?;
        let resume_time = Instant::from_ticks(susp.suspend_time + susp.suspend_interval);
        test_ctx.c_to_s_link.suspend(resume_time, false);
        test_ctx
            .s_to_c_link
            .suspend(resume_time, _spec.simulate_receive_block);
    }

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, _spec.target_time)?;

    // Verify rtt_max >= suspension interval on the server path.
    if let Some(susp) = _spec.suspension.first()
        && test_ctx.has_cnx_server()
    {
        let rtt_max = test_ctx.cnx_server().primary_path_rtt_max();
        if rtt_max < susp.suspend_interval {
            return Err(crate::Error::Generic);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Ticket-store helpers.

/// Run one ticket-seed test.  `mode=1` → RTT seeding, `mode=2` → BDP-frame seeding.
/// C: `ticket_seed_test_one` in `picoquictest/ticket_store_test.c`.
pub fn ticket_seed_test_one(bdp_option: u32) -> crate::Result<()> {
    use crate::tp::TransportParameter0RttKind::{CwinLocal, RttLocal};

    fn run_ticket_seed_scenario(
        test_ctx: &mut TestTlsApiCtx,
        loss_mask: &mut u64,
        simulated_time: &mut Instant,
        scenario: &[TestApiStreamDesc],
        max_completion_microsec: u64,
    ) -> crate::Result<()> {
        tls_api_connection_loop(test_ctx, loss_mask, 0, simulated_time)?;
        test_api_init_send_recv_scenario(test_ctx, scenario)?;
        tls_api_data_sending_loop(test_ctx, loss_mask, simulated_time, 0)?;
        tls_api_one_scenario_body_verify(test_ctx, simulated_time, max_completion_microsec)
    }

    fn clear_ticket_seed_streams(test_ctx: &mut TestTlsApiCtx) {
        test_ctx.test_streams.clear();
        test_ctx.stream0_target = 0;
        test_ctx.stream0_sent = 0;
        test_ctx.stream0_received = 0;
        test_ctx.streams_finished = false;
        test_ctx.test_finished = false;
    }

    fn delete_and_recreate_ticket_seed_client(
        test_ctx: &mut TestTlsApiCtx,
        simulated_time: Instant,
    ) -> crate::Result<()> {
        let client_token = test_ctx
            .cnx_client()
            .own_token
            .ok_or(crate::Error::Generic)?;
        test_ctx.qclient.delete_connection(client_token);

        let server_token = if test_ctx.has_cnx_server() {
            test_ctx.cnx_server().own_token
        } else {
            None
        };
        if let Some(server_token) = server_token {
            test_ctx.qserver.delete_connection(server_token);
        }
        clear_ticket_seed_streams(test_ctx);

        let server_addr = test_ctx.server_addr;
        let cnx_client = test_ctx
            .qclient
            .create_connection(
                crate::ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                crate::ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                Some(&server_addr),
                simulated_time,
                Version::InternalTest1 as u32,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .ok_or(crate::Error::Generic)?;
        cnx_client.start_client()
    }

    const TICKET_SEED_STORE: &str = "ticket_seed_store.bin";

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let max_completion_microsec = 1_000_000;
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    save_empty_tickets(TICKET_SEED_STORE, simulated_time)?;

    let mut test_ctx = tls_api_init_ctx(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TICKET_SEED_STORE),
    )
    .ok_or(crate::Error::Generic)?;
    let enable_bdp = bdp_option != 0;
    test_ctx.qclient.set_default_bdp_frame_option(enable_bdp);
    test_ctx.qserver.set_default_bdp_frame_option(enable_bdp);

    run_ticket_seed_scenario(
        &mut test_ctx,
        &mut loss_mask,
        &mut simulated_time,
        &scenario,
        max_completion_microsec,
    )?;

    let client_ticket_id = test_ctx.cnx_client().issued_ticket_id;
    let client_ticket = test_ctx.qclient.get_stored_ticket(
        Some(TEST_SNI),
        Some(TEST_ALPN),
        0,
        false,
        client_ticket_id,
    );
    let Some(client_ticket) = client_ticket else {
        return Err(crate::Error::Generic);
    };
    if client_ticket.tp_0rtt[RttLocal as usize] == 0
        || client_ticket.tp_0rtt[CwinLocal as usize] == 0
    {
        return Err(crate::Error::Generic);
    }

    let server_ticket_id = if test_ctx.has_cnx_server() {
        let server_issued_ticket_id = test_ctx.cnx_server().issued_ticket_id;
        let Some(server_ticket) = test_ctx
            .qserver
            .retrieve_issued_ticket(server_issued_ticket_id)
        else {
            return Err(crate::Error::Generic);
        };
        if server_ticket.rtt.ticks() == 0 || server_ticket.cwin == 0 {
            return Err(crate::Error::Generic);
        }
        server_ticket.ticket_id
    } else {
        let Some(server_ticket) = test_ctx.qserver.issued_tickets.iter_mut().next() else {
            return Err(crate::Error::Generic);
        };
        if server_ticket.rtt.ticks() == 0 || server_ticket.cwin == 0 {
            return Err(crate::Error::Generic);
        }
        server_ticket.ticket_id
    };

    delete_and_recreate_ticket_seed_client(&mut test_ctx, simulated_time)?;
    run_ticket_seed_scenario(
        &mut test_ctx,
        &mut loss_mask,
        &mut simulated_time,
        &scenario,
        max_completion_microsec,
    )?;

    if test_ctx.cnx_client().resumed_ticket_id != client_ticket_id
        || test_ctx.cnx_client().seed_rtt_min.ticks() == 0
        || test_ctx.cnx_client().seed_cwin == 0
    {
        return Err(crate::Error::Generic);
    }
    if test_ctx.has_cnx_server() {
        if test_ctx.cnx_server().resumed_ticket_id != server_ticket_id {
            return Err(crate::Error::Generic);
        }
        if test_ctx.cnx_client().seed_rtt_min.ticks() == 0 || test_ctx.cnx_client().seed_cwin == 0 {
            return Err(crate::Error::Generic);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Transport-parameter test helpers.

/// Encode then decode a set of transport parameters and verify round-trip equality.
/// C: `transport_param_test_one` (inline in `transport_param_test`).
pub fn transport_param_test_one(
    _test_ctx: &mut crate::Quic,
    _tp: &crate::tp::TransportParameters,
    _is_client: bool,
) -> crate::Result<()> {
    let mode = if _is_client { 0i32 } else { 1i32 };
    let mut buffer = [0u8; 1500];
    let buf_len = buffer.len();
    let mut consumed = 0usize;
    let mut decoded = 0usize;

    let cnx = _test_ctx.first_cnx_mut().ok_or(crate::Error::Generic)?;
    cnx.set_transport_parameters(_tp);
    if cnx.prepare_transport_extensions(mode, &mut buffer, buf_len, &mut consumed) != 0 {
        return Err(crate::Error::Generic);
    }
    if cnx.receive_transport_extensions(mode, &mut buffer, consumed, &mut decoded) != 0 {
        return Err(crate::Error::Generic);
    }
    Ok(())
}

/// Write a log of all transport-parameter test vectors to `filename`.
/// C: body of `transport_param_log_test`.
pub fn transport_param_log_test_one(_filename: &str) -> crate::Result<()> {
    const LOCAL_CONNECTION_ID: [u8; 8] = [2, 3, 4, 5, 6, 7, 8, 9];
    const INITIAL_CONNECTION_ID: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    const RESET_TOKEN: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    fn append_connection_id(v: &mut Vec<u8>, bytes: &[u8; 8]) {
        v.extend_from_slice(bytes);
    }

    fn append_reset_token(v: &mut Vec<u8>) {
        v.extend_from_slice(&RESET_TOKEN);
    }

    fn client_param1() -> Vec<u8> {
        let mut v = vec![
            5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e,
            3, 2, 0x45, 0xc8, 9, 4, 0x80, 0, 0x40, 0, 14, 1, 8, 15, 8,
        ];
        append_connection_id(&mut v, &LOCAL_CONNECTION_ID);
        v.extend_from_slice(&[0xc0, 0, 0, 0, 0x9f, 0x81, 0xa1, 0x76, 1, 2]);
        v
    }

    fn client_param2() -> Vec<u8> {
        let mut v = vec![
            5, 4, 0x81, 0, 0, 0, 4, 4, 0x81, 0, 0, 0, 8, 1, 1, 1, 2, 0x40, 0xff, 3, 2, 0x45, 0xc8,
            15, 8,
        ];
        append_connection_id(&mut v, &LOCAL_CONNECTION_ID);
        v.extend_from_slice(&[
            32, 2, 0x45, 0xc8, 0x50, 0x57, 1, 1, 0x80, 0, 0x71, 0x58, 1, 3, 0x6a, 0xb2, 0, 0xc0,
            0x17, 0xf7, 0x58, 0x6d, 0x2c, 0xb5, 0x71, 0,
        ]);
        v
    }

    fn client_param3() -> Vec<u8> {
        let mut v = vec![
            5, 4, 0x81, 0, 0, 0, 4, 4, 0x81, 0, 0, 0, 8, 1, 1, 1, 2, 0x40, 0xff, 15, 8,
        ];
        append_connection_id(&mut v, &LOCAL_CONNECTION_ID);
        v.extend_from_slice(&[
            0xc0, 0, 0, 0, 0xff, 4, 0xde, 0x1b, 2, 0x43, 0xe8, 0x80, 0, 0x71, 0x58, 1, 3,
        ]);
        v
    }

    fn client_param4() -> Vec<u8> {
        let mut v = vec![
            5, 4, 0x80, 1, 0, 0, 4, 8, 0xc0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 1, 1, 0x1e, 3, 2,
            0x45, 0xc8, 15, 8,
        ];
        append_connection_id(&mut v, &LOCAL_CONNECTION_ID);
        v.extend_from_slice(&[0x3e, 1, 4]);
        v
    }

    fn client_param5() -> Vec<u8> {
        let mut v = vec![
            1, 2, 0x40, 0x0a, 8, 1, 2, 5, 4, 0x80, 0, 0x20, 0, 4, 4, 0x80, 0, 0x40, 0, 3, 2, 0x45,
            0xc0, 10, 1, 0x11, 15, 8,
        ];
        append_connection_id(&mut v, &LOCAL_CONNECTION_ID);
        v
    }

    fn server_param1() -> Vec<u8> {
        let mut v = vec![
            5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e,
            3, 2, 0x45, 0xc8, 15, 8,
        ];
        append_connection_id(&mut v, &LOCAL_CONNECTION_ID);
        v.extend_from_slice(&[0, 8]);
        append_connection_id(&mut v, &INITIAL_CONNECTION_ID);
        v.extend_from_slice(&[2, 16]);
        append_reset_token(&mut v);
        v
    }

    fn server_param2() -> Vec<u8> {
        let mut v = vec![
            5, 4, 0x81, 0, 0, 0, 4, 4, 0x81, 0, 0, 0, 8, 1, 2, 1, 2, 0x40, 0xff, 3, 2, 0x45, 0xc8,
            15, 8,
        ];
        append_connection_id(&mut v, &LOCAL_CONNECTION_ID);
        v.extend_from_slice(&[0, 8]);
        append_connection_id(&mut v, &INITIAL_CONNECTION_ID);
        v.extend_from_slice(&[2, 16]);
        append_reset_token(&mut v);
        v
    }

    fn server_param3() -> Vec<u8> {
        let mut v = server_param2();
        v.extend_from_slice(&[
            13, 45, 10, 0, 0, 1, 0x11, 0x51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            4, 1, 2, 3, 4,
        ]);
        append_reset_token(&mut v);
        v
    }

    fn log_one<W: std::io::Write>(out: &mut W, bytes: &[u8]) -> crate::Result<()> {
        crate::textlog::textlog_transport_extension_content(
            out,
            true,
            0x0102_0304_0506_0708,
            bytes,
        )?;
        writeln!(out).map_err(|_| crate::Error::Generic)
    }

    fn transport_param_log_fuzz_test(target: &[u8]) -> crate::Result<()> {
        use std::io::Write as _;

        if target.len() < 8 || target.len() > 256 {
            return Err(crate::Error::Generic);
        }

        let mut fuzz_byte = 1u8;
        for l in 1..=8usize {
            for i in l..=target.len() {
                let mut buffer = target.to_vec();
                for b in &mut buffer[i - l..i] {
                    *b ^= fuzz_byte;
                    fuzz_byte = fuzz_byte.wrapping_add(1);
                }

                let file = std::fs::File::create("log_tp_fuzz_test.txt")
                    .map_err(|_| crate::Error::InvalidFile)?;
                let mut file = std::io::BufWriter::new(file);
                let mut dl = 0usize;
                while dl < target.len() {
                    crate::textlog::textlog_transport_extension_content(
                        &mut file,
                        true,
                        0x0102_0304_0506_0708,
                        &buffer[..target.len() - dl],
                    )?;
                    writeln!(file).map_err(|_| crate::Error::Generic)?;
                    dl += l + 6;
                }
            }
        }
        Ok(())
    }

    let client_param1 = client_param1();
    let client_param2 = client_param2();
    let client_param3 = client_param3();
    let server_param1 = server_param1();
    let server_param2 = server_param2();
    let client_param4 = client_param4();
    let client_param5 = client_param5();
    let server_param3 = server_param3();

    let mut file = std::fs::File::create(_filename).map_err(|_| crate::Error::InvalidFile)?;
    for params in [
        client_param1.as_slice(),
        client_param2.as_slice(),
        client_param3.as_slice(),
        server_param1.as_slice(),
        server_param2.as_slice(),
        client_param4.as_slice(),
        client_param5.as_slice(),
        server_param3.as_slice(),
    ] {
        log_one(&mut file, params)?;
    }

    transport_param_log_fuzz_test(&client_param2)?;
    transport_param_log_fuzz_test(&server_param2)
}

/// Apply one version-negotiation transport-parameter test case.
/// C: `vn_tp_test_one` (inline in `vn_tp_test`).
pub fn vn_tp_test_one(_test_id: u32, _is_client: bool, _expect_ok: bool) -> crate::Result<()> {
    use crate::internal::process_tp_version_negotiation;
    const V1: u32 = 0x0000_0001;
    const V2: u32 = 0x6b33_43cf;
    // TransportError::VersionNegotiationError = 0x11, ParameterError = 0x8
    const VN_ERR: u64 = 0x11;
    const TP_ERR: u64 = 0x08;

    // (bytes, mode, envelop_vn, expected_vn, expected_error)
    // 8 client cases: indices 0-3 ok, 4-7 fail.
    static CLIENT: &[(&[u8], i32, u32, u32, u64)] = &[
        // client_0: [V1, V2, V1] with envelop=V1 → negotiate V2
        (
            &[0, 0, 0, 1, 0x6b, 0x33, 0x43, 0xcf, 0, 0, 0, 1],
            0,
            V1,
            V2,
            0,
        ),
        // client_1: [V1, 0x0a0a0a0a, V2] with envelop=V1 → negotiate V2
        (
            &[0, 0, 0, 1, 0x0a, 0x0a, 0x0a, 0x0a, 0x6b, 0x33, 0x43, 0xcf],
            0,
            V1,
            V2,
            0,
        ),
        // client_2: [V1, 0x0a0a0a0a, V1, V2, 0xfa0a0a0a] with envelop=V1 → negotiate V1
        (
            &[
                0, 0, 0, 1, 0x0a, 0x0a, 0x0a, 0x0a, 0, 0, 0, 1, 0x6b, 0x33, 0x43, 0xcf, 0xfa, 0x0a,
                0x0a, 0x0a,
            ],
            0,
            V1,
            V1,
            0,
        ),
        // client_3: [V1] with envelop=V1 → no negotiation (vn=0)
        (&[0, 0, 0, 1], 0, V1, 0, 0),
        // client_3 with envelop=V2 → version negotiation error
        (&[0, 0, 0, 1], 0, V2, 0, VN_ERR),
        // client_bad_1: truncated (3 bytes) → TP error
        (&[0, 0, 0], 0, V1, 0, TP_ERR),
        // client_bad_2: [V1, partial] → TP error
        (&[0, 0, 0, 1, 0x0a, 0x0a, 0x0a], 0, V1, 0, TP_ERR),
        // client_bad_3: [0x0] → TP error
        (&[0x0], 0, V1, 0, TP_ERR),
    ];

    // 8 server cases: indices 0-3 ok (repeated server_0/server_1), 4-7 fail.
    static SERVER: &[(&[u8], i32, u32, u32, u64)] = &[
        // server_0: [V2] with envelop=V2 → ok (server doesn't negotiate client version)
        (&[0x6b, 0x33, 0x43, 0xcf], 1, V2, 0, 0),
        // server_1: [V2, V1, 0x2, 0x3] with envelop=V2 → ok
        (
            &[0x6b, 0x33, 0x43, 0xcf, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3],
            1,
            V2,
            0,
            0,
        ),
        // repeat server_0
        (&[0x6b, 0x33, 0x43, 0xcf], 1, V2, 0, 0),
        // repeat server_1
        (
            &[0x6b, 0x33, 0x43, 0xcf, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3],
            1,
            V2,
            0,
            0,
        ),
        // server_0 with envelop=V1 → version negotiation error
        (&[0x6b, 0x33, 0x43, 0xcf], 1, V1, 0, VN_ERR),
        // server_bad_1: [V2, V1, V2, 0x5043] → TP error
        (
            &[
                0x6b, 0x33, 0x43, 0xcf, 0, 0, 0, 1, 0x6b, 0x33, 0x43, 0xcf, 0x50, 0x43,
            ],
            1,
            V2,
            0,
            TP_ERR,
        ),
        // server_bad_2: [V2, 0x0] → TP error
        (&[0x6b, 0x33, 0x43, 0xcf, 0x0], 1, V2, 0, TP_ERR),
        // server_bad_3: [V2, 0x0, 0x0] → TP error
        (&[0x6b, 0x33, 0x43, 0xcf, 0x0, 0x0], 1, V2, 0, TP_ERR),
    ];

    let cases = if _is_client { CLIENT } else { SERVER };
    let &(data, mode, envelop_vn, expected_vn, expected_error) =
        cases.get(_test_id as usize).ok_or(crate::Error::Generic)?;

    let mut negotiated_vn = 0u32;
    let mut negotiated_index = 0i32;
    let mut error_found = 0u64;
    let result = process_tp_version_negotiation(
        data,
        mode,
        envelop_vn,
        &mut negotiated_vn,
        &mut negotiated_index,
        &mut error_found,
    );

    let actual_ok = match result {
        // parse error: ok iff we expected an error and got the right code
        None => expected_error != 0 && error_found == expected_error,
        // parse success: ok iff no error expected and version matches
        Some(_) => expected_error == 0 && expected_vn == negotiated_vn,
    };

    if actual_ok == _expect_ok {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}
