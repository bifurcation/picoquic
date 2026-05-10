//! Test cases for `picoquictest/pacing_test.c`.
//!
//! `pacing` verifies inter-packet gap math via a Quic + Connection setup.
//! `pacing_repeat` is a table-driven unit test for the `Pacing` struct directly.
//! `pacing_bbr` / `_cubic` / `_dcubic` / `_fast` / `_newreno` exercise the
//! pacer end-to-end under a leaky-bucket rate limit with each CC algorithm.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, rctl_configure, test_api_init_send_recv_scenario, tls_api_connection_loop,
    tls_api_data_sending_loop, tls_api_init_ctx_ex, tls_api_one_scenario_body_verify,
};
use crate::internal::{Connection, INITIAL_RTT, Pacing, Path, Version};
use crate::{
    CongestionAlgorithm, CongestionControl, CongestionNotification, ConnectionId, Duration,
    ECN_ECT_0, Instant, MAX_PACKET_SIZE, PerAckState, RESET_SECRET_SIZE, get_congestion_algorithm,
};

// ---------------------------------------------------------------------------
// Shared stream scenario for the CC algo tests.

const SCENARIO_PACING: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

// ---------------------------------------------------------------------------
// pacing_cc_algotest helper.  C: `pacing_cc_algotest`.

struct PacingBbrCongestionControl;

struct PacingBbrState {
    inner: crate::bbr::BbrState,
    option_string: Option<String>,
    init_time: Instant,
    needs_connection_init: bool,
}

impl PacingBbrCongestionControl {
    fn init_state(
        path_x: &mut Path,
        option_string: Option<&str>,
        current_time: Instant,
    ) -> PacingBbrState {
        let mut bbr_state = crate::bbr::BbrState {
            option_string: option_string.map(str::to_owned),
            ..Default::default()
        };

        // This pacing test installs BBR only on the server context, so mirror
        // server-side BBROnInit until alg_notify can replay the translated
        // on_init with the owning Connection.
        let mut random_context: u64 = 0xfedc_ba98_7654_3210;
        random_context ^= current_time.ticks();
        if path_x.unique_path_id > 0 && path_x.unique_path_id != u64::MAX {
            random_context = random_context.wrapping_mul(path_x.unique_path_id.wrapping_add(1));
        }
        bbr_state.random_context = random_context;

        if path_x.smoothed_rtt == INITIAL_RTT && path_x.rtt_variant.ticks() == 0 {
            bbr_state.min_rtt = u64::MAX;
        } else {
            bbr_state.min_rtt = path_x.smoothed_rtt.ticks();
        }

        let min_rtt = bbr_state.min_rtt;
        bbr_state.reset_rtt_jitter_buffer(min_rtt, current_time.ticks());
        bbr_state.probe_rtt_min_stamp = current_time.ticks();
        bbr_state.probe_rtt_min_delay = bbr_state.min_rtt;
        bbr_state.min_rtt_stamp = current_time.ticks();
        bbr_state.extra_acked_interval_start = current_time.ticks();
        bbr_state.extra_acked_delivered = 0;
        bbr_state.set_options();
        if bbr_state.quantum_ratio == 0.0 {
            bbr_state.quantum_ratio = 0.001;
        }
        bbr_state.reset_congestion_signals();
        bbr_state.reset_lower_bounds();
        bbr_state.next_round_delivered = 0;
        bbr_state.round_start = false;
        bbr_state.round_count = 0;
        bbr_state.round_start_pn = path_x.pkt_ctx.send_sequence;
        bbr_state.init_full_pipe();
        bbr_state.init_pacing_rate(path_x);
        bbr_state.enter_startup(path_x);

        PacingBbrState {
            inner: bbr_state,
            option_string: option_string.map(str::to_owned),
            init_time: current_time,
            needs_connection_init: true,
        }
    }
}

impl CongestionControl for PacingBbrCongestionControl {
    fn alg_init(&self, path_x: &mut Path, option_string: Option<&str>, current_time: Instant) {
        let state = Self::init_state(path_x, option_string, current_time);
        path_x.congestion_alg_state = Some(Box::new(state));
    }

    fn alg_notify(
        &self,
        connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
        current_time: Instant,
    ) {
        if path_x.congestion_alg_state.is_none() {
            let state = Self::init_state(path_x, None, current_time);
            path_x.congestion_alg_state = Some(Box::new(state));
        }

        let Some(boxed_state) = path_x.congestion_alg_state.take() else {
            return;
        };

        let mut bbr_state = match boxed_state.downcast::<PacingBbrState>() {
            Ok(state) => state,
            Err(boxed_state) => {
                path_x.congestion_alg_state = Some(boxed_state);
                return;
            }
        };

        if bbr_state.needs_connection_init {
            bbr_state.inner.on_init(
                connection,
                path_x,
                bbr_state.init_time.ticks(),
                bbr_state.option_string.clone(),
            );
            bbr_state.needs_connection_init = false;
        }

        bbr_state.inner.notify(
            connection,
            path_x,
            notification,
            ack_state,
            current_time.ticks(),
        );
        path_x.congestion_alg_state = Some(bbr_state);
    }

    fn alg_delete(&self, path_x: &mut Path) {
        path_x.congestion_alg_state = None;
    }

    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|state| state.downcast_ref::<PacingBbrState>())
            .map(|state| state.inner.observe())
    }
}

static PACING_BBR_CONTROL: PacingBbrCongestionControl = PacingBbrCongestionControl;
static PACING_BBR_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "bbr",
    congestion_algorithm_number: 5,
    ecn_mark: ECN_ECT_0,
    algorithm: &PACING_BBR_CONTROL,
};

struct PacingCubicCongestionControl;

impl CongestionControl for PacingCubicCongestionControl {
    fn alg_init(&self, path_x: &mut Path, option_string: Option<&str>, current_time: Instant) {
        crate::cubic::CubicState::init(path_x, option_string, current_time.ticks());
    }

    fn alg_notify(
        &self,
        connection: &mut Connection,
        path_x: &mut Path,
        notification: CongestionNotification,
        ack_state: &PerAckState,
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

    fn alg_delete(&self, path_x: &mut Path) {
        path_x.congestion_alg_state = None;
    }

    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|state| state.downcast_ref::<crate::cubic::CubicState>())
            .map(|state| state.observe())
    }
}

static PACING_CUBIC_CONTROL: PacingCubicCongestionControl = PacingCubicCongestionControl;
static PACING_CUBIC_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "cubic",
    congestion_algorithm_number: 2,
    ecn_mark: ECN_ECT_0,
    algorithm: &PACING_CUBIC_CONTROL,
};

static PACING_FASTCC_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "fast",
    congestion_algorithm_number: 4,
    ecn_mark: ECN_ECT_0,
    algorithm: &crate::fastcc::FASTCC_CONTROL,
};

static PACING_NEWRENO_ALGORITHM: CongestionAlgorithm = CongestionAlgorithm {
    congestion_algorithm_id: "newreno",
    congestion_algorithm_number: 1,
    ecn_mark: ECN_ECT_0,
    algorithm: &crate::newreno::NEWRENO_CONTROL,
};

fn registered_cc_algorithm(cc_algo_name: &str) -> &'static CongestionAlgorithm {
    get_congestion_algorithm(cc_algo_name).expect("cc algorithm")
}

fn pacing_cc_algotest(cc_algo: &'static CongestionAlgorithm, target_time: u64, loss_target: u64) {
    const LATENCY_TARGET: u64 = 7_500;
    const BUCKET_INCREASE_PER_MICROSEC: f64 = 1.25;
    const BUCKET_MAX: u64 = 16 * MAX_PACKET_SIZE as u64;
    const PICOSEC_PER_BYTE: u64 = (1_000_000u64 * 8) / 100;

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;

    let initial_cid = ConnectionId::clone_from_slice(&[
        0x9a,
        0xc1,
        0xcc,
        0xa1,
        cc_algo.congestion_algorithm_number,
        6,
        7,
        8,
    ])
    .expect("initial CID");

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.c_to_s_link.microsec_latency = LATENCY_TARGET;
    test_ctx.c_to_s_link.picosec_per_byte = PICOSEC_PER_BYTE;
    test_ctx.s_to_c_link.microsec_latency = LATENCY_TARGET;
    test_ctx.s_to_c_link.picosec_per_byte = PICOSEC_PER_BYTE;

    test_ctx.qserver.set_default_congestion_algorithm(cc_algo);
    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;

    rctl_configure(
        &mut test_ctx.c_to_s_link,
        BUCKET_INCREASE_PER_MICROSEC,
        BUCKET_MAX,
        simulated_time,
    )
    .expect("rctl_configure c_to_s");
    rctl_configure(
        &mut test_ctx.s_to_c_link,
        BUCKET_INCREASE_PER_MICROSEC,
        BUCKET_MAX,
        simulated_time,
    )
    .expect("rctl_configure s_to_c");

    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        LATENCY_TARGET,
        &mut simulated_time,
    )
    .expect("connection loop");

    test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_PACING)
        .expect("init send/recv scenario");

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");

    let observed_loss = if test_ctx.has_cnx_server() {
        test_ctx.cnx_server().nb_retransmission_total
    } else {
        u64::MAX
    };

    let scenario_finished = test_ctx.test_finished;
    let completion_time = simulated_time
        .ticks()
        .saturating_sub(test_ctx.cnx_client().start_time.ticks());
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
        .unwrap_or_else(|err| {
            panic!(
                "scenario body verify: {err:?}; finished={scenario_finished}, completion_time={completion_time}, target_time={target_time}"
            )
        });

    assert!(
        observed_loss <= loss_target,
        "pacing cc={}: expected <= {loss_target} losses, got {observed_loss}",
        cc_algo.congestion_algorithm_id
    );
}

// ---------------------------------------------------------------------------
// pacing_repeat event table.  C: `pacing_events[]` / `pacing_test_t`.

struct PacingTestEvent {
    current_time: u64,
    length: usize,
    send_mtu: usize,
    cwin: u64,
    slow_start: i32,
    rtt: u64,
    rate: u64,
    quantum: u64,
    expected_ok: bool,
    expected_packet_nanosec: i64,
    expected_bucket_nanosec: i64,
    expected_next_time: u64,
}

const PACING_EVENTS: &[PacingTestEvent] = &[
    // Setup (length == 0, cwin == 0) → update_parameters.
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1280,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 125000,
        quantum: 8096,
        expected_ok: false,
        expected_packet_nanosec: 10000000,
        expected_bucket_nanosec: 64768000,
        expected_next_time: 0,
    },
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1280,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 1250000,
        quantum: 8096,
        expected_ok: false,
        expected_packet_nanosec: 1024000,
        expected_bucket_nanosec: 6476800,
        expected_next_time: 0,
    },
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1280,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 12500000,
        quantum: 8096,
        expected_ok: false,
        expected_packet_nanosec: 102400,
        expected_bucket_nanosec: 647680,
        expected_next_time: 0,
    },
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1280,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 12500000,
        quantum: 16192,
        expected_ok: false,
        expected_packet_nanosec: 102400,
        expected_bucket_nanosec: 1295360,
        expected_next_time: 0,
    },
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1280,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 125000000,
        quantum: 16192,
        expected_ok: false,
        expected_packet_nanosec: 10240,
        expected_bucket_nanosec: 129536,
        expected_next_time: 0,
    },
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1280,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 1250000000,
        quantum: 16192,
        expected_ok: false,
        expected_packet_nanosec: 1024,
        expected_bucket_nanosec: 12953,
        expected_next_time: 0,
    },
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1280,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 12500000000,
        quantum: 16192,
        expected_ok: false,
        expected_packet_nanosec: 102,
        expected_bucket_nanosec: 1295,
        expected_next_time: 0,
    },
    // Setup (length == 0, cwin != 0) → update_window.
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1280,
        cwin: 16000,
        slow_start: 1,
        rtt: 10000,
        rate: 2000000,
        quantum: 0,
        expected_ok: false,
        expected_packet_nanosec: 640000,
        expected_bucket_nanosec: 2000000,
        expected_next_time: 0,
    },
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1536,
        cwin: 153600,
        slow_start: 1,
        rtt: 10000,
        rate: 19200000,
        quantum: 0,
        expected_ok: false,
        expected_packet_nanosec: 80000,
        expected_bucket_nanosec: 1280000,
        expected_next_time: 0,
    },
    PacingTestEvent {
        current_time: 0,
        length: 0,
        send_mtu: 1536,
        cwin: 153600,
        slow_start: 0,
        rtt: 10000,
        rate: 15360000,
        quantum: 0,
        expected_ok: false,
        expected_packet_nanosec: 100000,
        expected_bucket_nanosec: 1600000,
        expected_next_time: 0,
    },
    // Use events (length != 0) → is_authorized + update_after_send.
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 900000,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 800000,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 700000,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 600000,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 500000,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 400000,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 300000,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 200000,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 100000,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 0,
        expected_next_time: u64::MAX,
    },
    PacingTestEvent {
        current_time: 1000,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: false,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 0,
        expected_next_time: 1101,
    },
    PacingTestEvent {
        current_time: 1050,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: false,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 50000,
        expected_next_time: 1101,
    },
    PacingTestEvent {
        current_time: 1101,
        length: 1536,
        send_mtu: 1536,
        cwin: 0,
        slow_start: 0,
        rtt: 10000,
        rate: 0,
        quantum: 0,
        expected_ok: true,
        expected_packet_nanosec: 0,
        expected_bucket_nanosec: 1000,
        expected_next_time: u64::MAX,
    },
];

// ---------------------------------------------------------------------------
// Test entries.

/// C: `pacing_test`.
#[test]
fn pacing() {
    const TEST_BYTE_PER_SEC: u64 = 1_250_000;
    const TEST_QUANTUM: u64 = 0x4000;
    const NB_TARGET: i32 = 10_000;

    let mut current_time = Instant::from_ticks(0);

    let mut quic = crate::Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("quic");

    let saddr: std::net::SocketAddr = "127.0.0.1:1000".parse().unwrap();

    let cnx = quic
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            Some(&saddr),
            current_time,
            0,
            Some("test-sni"),
            Some("test-alpn"),
            true,
        )
        .expect("cnx");

    cnx.paths[0].update_pacing_rate(TEST_BYTE_PER_SEC as f64, TEST_QUANTUM);

    let mut nb_sent = 0i32;
    let mut nb_round = 0i32;

    while nb_sent < NB_TARGET {
        nb_round += 1;
        assert!(
            nb_round <= 4 * NB_TARGET,
            "pacing needs more than {nb_round} rounds for {NB_TARGET} packets"
        );
        let mut next_time = current_time + Duration::from_ticks(10_000_000);
        if cnx.is_sending_authorized_by_pacing(0, current_time, &mut next_time) {
            nb_sent += 1;
            let send_mtu = cnx.paths[0].send_mtu;
            cnx.paths[0].update_pacing_after_send(send_mtu, current_time);
        } else {
            assert!(
                current_time < next_time,
                "pacing next={next_time:?} <= current={current_time:?}"
            );
            current_time = next_time;
        }
    }

    let send_mtu = cnx.paths[0].send_mtu;
    let volume_sent = NB_TARGET as u64 * send_mtu as u64;
    let time_max = (volume_sent * 1_000_000) / TEST_BYTE_PER_SEC + 1;
    let time_min = (volume_sent.saturating_sub(TEST_QUANTUM) * 1_000_000) / TEST_BYTE_PER_SEC + 1;
    let current_us = current_time.ticks();

    assert!(
        current_us <= time_max,
        "pacing used = {current_us}, expected max = {time_max}"
    );
    assert!(
        current_us >= time_min,
        "pacing used = {current_us}, expected min = {time_min}"
    );
}

/// C: `pacing_repeat_test`.
#[test]
fn pacing_repeat() {
    let mut pacing = Pacing {
        rate: 0,
        evaluation_time: Instant::from_ticks(0),
        bucket_max: 0,
        packet_time_microsec: Duration::from_ticks(0),
        quantum_max: 0,
        rate_max: 0,
        bandwidth_pause: 0,
        bucket_nanosec: 0,
        packet_time_nanosec: 0,
    };

    for (i, ev) in PACING_EVENTS.iter().enumerate() {
        if ev.length == 0 {
            if ev.cwin == 0 {
                pacing.update_parameters(
                    ev.rate as f64,
                    ev.quantum,
                    ev.send_mtu,
                    Duration::from_ticks(ev.rtt),
                    None,
                );
            } else {
                pacing.update_window(
                    ev.slow_start,
                    ev.cwin,
                    ev.send_mtu,
                    Duration::from_ticks(ev.rtt),
                    None,
                );
            }
            assert_eq!(pacing.rate, ev.rate, "event {i}: rate");
            assert_eq!(
                pacing.packet_time_nanosec, ev.expected_packet_nanosec,
                "event {i}: packet_time_nanosec"
            );
            assert_eq!(
                pacing.bucket_max, ev.expected_bucket_nanosec,
                "event {i}: bucket_max"
            );
        } else {
            let mut next_time = Instant::from_ticks(u64::MAX);
            let is_ok = pacing.is_authorized(
                Instant::from_ticks(ev.current_time),
                &mut next_time,
                false,
                None,
            );
            assert_eq!(is_ok, ev.expected_ok, "event {i}: is_ok");
            if is_ok {
                pacing.update_after_send(
                    ev.length,
                    ev.send_mtu,
                    Instant::from_ticks(ev.current_time),
                );
            }
            assert_eq!(
                pacing.bucket_nanosec, ev.expected_bucket_nanosec,
                "event {i}: bucket_nanosec"
            );
            assert_eq!(
                next_time.ticks(),
                ev.expected_next_time,
                "event {i}: next_time"
            );
        }
    }
}

/// C: `pacing_bbr_test`.
#[test]
fn pacing_bbr() {
    pacing_cc_algotest(&PACING_BBR_ALGORITHM, 900_000, 160);
}

/// C: `pacing_cubic_test`.
#[test]
fn pacing_cubic() {
    pacing_cc_algotest(&PACING_CUBIC_ALGORITHM, 900_000, 210);
}

/// C: `pacing_dcubic_test`.
#[test]
fn pacing_dcubic() {
    pacing_cc_algotest(registered_cc_algorithm("dcubic"), 900_000, 240);
}

/// C: `pacing_fast_test`.
#[test]
fn pacing_fast() {
    pacing_cc_algotest(&PACING_FASTCC_ALGORITHM, 1_000_000, 180);
}

/// C: `pacing_newreno_test`.
#[test]
fn pacing_newreno() {
    pacing_cc_algotest(&PACING_NEWRENO_ALGORITHM, 900_000, 100);
}
