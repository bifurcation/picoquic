//! Tests for `picoquictest/app_limited.c`.
//!
//! Verifies behaviour of rate-limited clients — sources that send at a
//! sustained rate below the link capacity.  Tests cover initial window
//! growth, congestion-window growth in avoidance phase, pacing rate,
//! and preemptive repeat behaviour with a loss mask.
//!
//! Tests covered:
//! * [`app_limited_bbr`] — C `app_limited_bbr_test`: BBR congestion control.
//! * [`app_limited_cubic`] — C `app_limited_cubic_test`: Cubic.
//! * [`app_limited_reno`] — C `app_limited_reno_test`: New Reno.
//! * [`app_limited_rpr`] — C `app_limited_rpr_test`: Cubic + preemptive repeat + loss.

use std::cell::RefCell;
use std::rc::Rc;

use crate::internal::{Connection, Version};
use crate::{
    CallbackEvent, CongestionAlgorithm, ConnectionId, Instant, State, StreamDataCallback,
    get_congestion_algorithm,
};

use super::util::{
    TestTlsApiCtx, tls_api_connection_loop, tls_api_one_scenario_init_ex, tls_api_one_sim_round,
};

// ---------------------------------------------------------------------------
// Configuration bundle.  C: `app_limited_test_config_t`.

struct AppLimitedConfig {
    test_id: u8,
    ccalgo: &'static CongestionAlgorithm,
    do_preemptive_repeat: bool,
    stream_0_packet_size: usize,
    stream_0_packet_interval: u64,
    data_stream_size: u64,
    time_to_stream: [u64; 3],
    loss_mask: u64,
    completion_target: u64,
    rtt_max: u64,
    cwin_max: u64,
    data_rate_max: u64,
    nb_losses_max: u64,
}

impl AppLimitedConfig {
    fn default_config(test_id: u8) -> Self {
        let reno = get_congestion_algorithm("newreno").expect("newreno cc algo");
        Self {
            test_id,
            ccalgo: reno,
            do_preemptive_repeat: false,
            stream_0_packet_size: 511,
            stream_0_packet_interval: 800,
            data_stream_size: 1_000_000,
            time_to_stream: [0, 2_500_000, 7_500_000],
            loss_mask: 0,
            completion_target: 12_000_000,
            rtt_max: 62_000,
            cwin_max: 100_000,
            data_rate_max: 4_000_000,
            nb_losses_max: 10,
        }
    }
}

// ---------------------------------------------------------------------------
// Per-stream state.  C: `app_limited_stream_ctx_t`.

#[allow(dead_code)]
struct AppLimitedStreamCtx {
    stream_id: Option<u64>,
    data_size: usize,
    octets_sent: usize,
    octets_recv: usize,
    fin_received: bool,
    is_fin_sent: bool,
    rank: usize,
}

impl AppLimitedStreamCtx {
    fn new(rank: usize, data_size: usize) -> Self {
        Self {
            stream_id: None,
            data_size,
            octets_sent: 0,
            octets_recv: 0,
            fin_received: false,
            is_fin_sent: false,
            rank,
        }
    }

    fn find_or_create(streams: &mut [Self; 3], stream_id: u64) -> Option<usize> {
        for (i, s) in streams.iter_mut().enumerate() {
            if s.stream_id == Some(stream_id) || s.stream_id.is_none() {
                s.stream_id = Some(stream_id);
                s.rank = i;
                return Some(i);
            }
        }
        None
    }

    fn receive(&mut self, bytes: &[u8], is_fin: bool) -> bool {
        if is_fin && self.fin_received {
            return false;
        }
        for &b in bytes {
            if b == (self.octets_recv & 0xff) as u8 {
                self.octets_recv += 1;
            } else {
                return false;
            }
        }
        if self.octets_recv > self.data_size {
            return false;
        }
        if is_fin {
            self.fin_received = true;
            self.octets_recv == self.data_size
        } else {
            true
        }
    }
}

// ---------------------------------------------------------------------------
// Shared mutable context; held behind Rc<RefCell<>> so both callbacks
// can reach it.  C: `app_limited_ctx_t` (minus the cnx pointer pairs,
// which come from TestTlsApiCtx in the test body).

#[allow(dead_code)]
struct AppLimitedInner {
    stream0_next_time: u64,
    stream0_bytes_sent_this_packet: usize,
    nb_client_streams_completed: i32,
    rtt_max: u64,
    cwin_max: u64,
    data_rate_max: u64,
    server_streams: [AppLimitedStreamCtx; 3],
    client_streams: [AppLimitedStreamCtx; 3],
    config: AppLimitedConfig,
}

impl AppLimitedInner {
    fn new(config: AppLimitedConfig) -> Self {
        let data_stream_size = config.data_stream_size as usize;
        let stream0_size = {
            let t2 = config.time_to_stream[2];
            let size = config.stream_0_packet_size as u64;
            let interval = config.stream_0_packet_interval;
            (((t2 + 1_000_000) * size) / interval) as usize
        };
        let server_streams = core::array::from_fn(|i| {
            let data_size = if i == 0 {
                stream0_size
            } else {
                data_stream_size
            };
            AppLimitedStreamCtx::new(i, data_size)
        });
        let client_streams = core::array::from_fn(|i| {
            let data_size = if i == 0 {
                stream0_size
            } else {
                data_stream_size
            };
            AppLimitedStreamCtx::new(i, data_size)
        });
        Self {
            stream0_next_time: 0,
            stream0_bytes_sent_this_packet: 0,
            nb_client_streams_completed: 0,
            rtt_max: 0,
            cwin_max: 0,
            data_rate_max: 0,
            server_streams,
            client_streams,
            config,
        }
    }
}

// ---------------------------------------------------------------------------
// Client callback.  C: `app_limited_callback` with `is_server == 0`.

struct AppLimitedClientCallback {
    inner: Rc<RefCell<AppLimitedInner>>,
}

impl StreamDataCallback for AppLimitedClientCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        match fin_or_event {
            CallbackEvent::StreamData | CallbackEvent::StreamFin => {
                let is_fin = fin_or_event == CallbackEvent::StreamFin;
                let mut inner = self.inner.borrow_mut();
                let idx = AppLimitedStreamCtx::find_or_create(&mut inner.client_streams, stream_id);
                if let Some(i) = idx {
                    if inner.client_streams[i].receive(bytes, is_fin) {
                        if is_fin {
                            inner.nb_client_streams_completed += 1;
                        }
                        0
                    } else {
                        -1
                    }
                } else {
                    connection.reset_stream(stream_id, 1).ok();
                    -1
                }
            }
            CallbackEvent::PrepareToSend => {
                // C: picoquic_provide_stream_data_buffer(context, available, is_fin, is_still_active)
                // Phase 4 will wire this through the Rust PrepareToSend API once the
                // mutable-buffer-context mechanism is finalised.
                let _ = bytes;
                0
            }
            CallbackEvent::StatelessReset
            | CallbackEvent::Close
            | CallbackEvent::ApplicationClose => 0,
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Server default callback.  C: `app_limited_callback` with `is_server == 1`
// (invoked via the `picoquic_set_default_callback` path).

struct AppLimitedServerCallback {
    inner: Rc<RefCell<AppLimitedInner>>,
}

impl StreamDataCallback for AppLimitedServerCallback {
    fn callback(
        &mut self,
        _connection: &mut Connection,
        _stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        match fin_or_event {
            CallbackEvent::PrepareToSend => {
                // C: app_limited_prepare_to_send_on_stream -> picoquic_provide_stream_data_buffer
                // Phase 4 will wire this through the Rust PrepareToSend API.
                let _ = bytes;
                0
            }
            CallbackEvent::StatelessReset
            | CallbackEvent::Close
            | CallbackEvent::ApplicationClose => 0,
            _ => {
                let _ = &self.inner;
                0
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Simulation helpers.

/// Decide whether any server streams need activation and compute the
/// next wake-up time.  C: `app_limited_get_timeout`.
fn app_limited_get_timeout(
    test_ctx: &mut TestTlsApiCtx,
    inner: &mut AppLimitedInner,
    simulated_time: u64,
) -> u64 {
    let mut timeout: u64 = 0;

    for i in 0..3usize {
        if inner.server_streams[i].stream_id.is_none() {
            if simulated_time >= inner.config.time_to_stream[i] {
                let stream_id = test_ctx.cnx_server().get_next_local_stream_id(true);
                inner.server_streams[i].stream_id = Some(stream_id);
                test_ctx
                    .cnx_server()
                    .mark_active_stream(stream_id, true, Some(Box::new(i)))
                    .ok();
                if i == 0 {
                    inner.stream0_next_time = simulated_time;
                }
            } else {
                timeout = inner.config.time_to_stream[i];
            }
            break;
        }
    }

    if let Some(s0_id) = inner.server_streams[0].stream_id {
        if simulated_time < inner.stream0_next_time {
            if timeout == 0 || inner.stream0_next_time < timeout {
                timeout = inner.stream0_next_time;
            }
        } else if !inner.server_streams[0].is_fin_sent {
            test_ctx
                .cnx_server()
                .mark_active_stream(s0_id, true, Some(Box::new(0usize)))
                .ok();
        }
    }

    timeout
}

/// Sample the primary path's rtt_max, cwin, and pacing rate and keep
/// running maxima.  C: `app_limited_monitor`.
fn app_limited_monitor(test_ctx: &mut TestTlsApiCtx, inner: &mut AppLimitedInner) {
    let rtt = test_ctx.cnx_server().primary_path_rtt_max();
    let cwin = test_ctx.cnx_server().primary_path_cwin();
    let rate = test_ctx.cnx_server().primary_path_pacing_rate();
    if rtt > inner.rtt_max {
        inner.rtt_max = rtt;
    }
    if cwin > inner.cwin_max {
        inner.cwin_max = cwin;
    }
    if rate > inner.data_rate_max {
        inner.data_rate_max = rate;
    }
}

fn is_before_disconnecting(state: State) -> bool {
    !matches!(
        state,
        State::Disconnecting
            | State::ClosingReceived
            | State::Closing
            | State::Draining
            | State::Disconnected
    )
}

// ---------------------------------------------------------------------------
// Shared test body.  C: `app_limited_test_one`.

fn app_limited_test_one(config: AppLimitedConfig) {
    let picosec_per_byte = (1_000_000u64 * 8) / 10;
    let queue_delay_max: u64 = 40_000;

    let initial_cid =
        ConnectionId::clone_from_slice(&[0xab, 0xb1, 0x1b, 0x17, 0xed, 0, 0, config.test_id])
            .expect("8-byte initial CID");

    let mut simulated_time = Instant::from_ticks(0);
    let inner = Rc::new(RefCell::new(AppLimitedInner::new(config)));

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_one_scenario_init_ex");

    {
        let borrow = inner.borrow();
        let cfg = &borrow.config;
        test_ctx
            .qserver
            .set_default_congestion_algorithm(cfg.ccalgo);
        test_ctx.cnx_client().set_congestion_algorithm(cfg.ccalgo);
        test_ctx.qserver.set_qlog(".").ok();
        test_ctx.qclient.set_qlog(".").ok();
        test_ctx.qserver.use_long_log = true;
        if cfg.do_preemptive_repeat {
            test_ctx.qserver.set_preemptive_repeat_policy(true);
            test_ctx.cnx_client().set_preemptive_repeat(true);
        }
        test_ctx.s_to_c_link.picosec_per_byte = picosec_per_byte;
        test_ctx.c_to_s_link.picosec_per_byte = picosec_per_byte;
    }

    test_ctx
        .qserver
        .set_default_callback(Some(Box::new(AppLimitedServerCallback {
            inner: Rc::clone(&inner),
        })));
    test_ctx
        .cnx_client()
        .set_callback(Some(Box::new(AppLimitedClientCallback {
            inner: Rc::clone(&inner),
        })));

    test_ctx.cnx_client().start_client().ok();

    let mut loss_mask = inner.borrow().config.loss_mask;
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        queue_delay_max,
        &mut simulated_time,
    )
    .expect("connection loop");

    let mut nb_trials = 0i32;
    loop {
        if test_ctx.cnx_client().state() == State::Disconnected {
            break;
        }

        let timeout = {
            let mut borrow = inner.borrow_mut();
            app_limited_get_timeout(&mut test_ctx, &mut borrow, simulated_time.ticks())
        };

        let timeout_instant = Instant::from_ticks(timeout);
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            timeout_instant,
            &mut was_active,
        )
        .expect("sim round");

        let backlog_empty = test_ctx.cnx_client().is_cnx_backlog_empty();
        let client_state = test_ctx.cnx_client().state();
        {
            let borrow = inner.borrow();
            if backlog_empty
                && borrow.nb_client_streams_completed >= 3
                && is_before_disconnecting(client_state)
            {
                drop(borrow);
                test_ctx.cnx_client().close(0).ok();
            }
        }

        {
            let mut borrow = inner.borrow_mut();
            app_limited_monitor(&mut test_ctx, &mut borrow);
        }

        nb_trials += 1;
        assert!(nb_trials <= 1_000_000, "simulation did not converge");
    }

    let borrow = inner.borrow();
    let cfg = &borrow.config;

    if cfg.completion_target != 0 {
        assert!(
            simulated_time.ticks() <= cfg.completion_target,
            "completion time {} > target {}",
            simulated_time.ticks(),
            cfg.completion_target,
        );
    }

    assert!(
        test_ctx.qclient.nb_data_nodes_in_pool() >= test_ctx.qclient.nb_data_nodes_allocated,
        "client data node pool exhausted",
    );
    assert!(
        test_ctx.qserver.nb_data_nodes_in_pool() >= test_ctx.qserver.nb_data_nodes_allocated,
        "server data node pool exhausted",
    );

    assert!(
        borrow.rtt_max <= cfg.rtt_max,
        "rtt_max {} > {}",
        borrow.rtt_max,
        cfg.rtt_max,
    );
    assert!(
        borrow.cwin_max <= cfg.cwin_max,
        "cwin_max {} > {}",
        borrow.cwin_max,
        cfg.cwin_max,
    );
    assert!(
        borrow.data_rate_max <= cfg.data_rate_max,
        "data_rate_max {} > {}",
        borrow.data_rate_max,
        cfg.data_rate_max,
    );
    assert!(
        test_ctx.cnx_server().nb_retransmission_total <= cfg.nb_losses_max,
        "nb_retransmission {} > {}",
        test_ctx.cnx_server().nb_retransmission_total,
        cfg.nb_losses_max,
    );
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `app_limited_bbr_test` in `picoquictest/app_limited.c`.
#[test]
fn app_limited_bbr() {
    let mut config = AppLimitedConfig::default_config(3);
    config.ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    app_limited_test_one(config);
}

/// C: `app_limited_cubic_test` in `picoquictest/app_limited.c`.
#[test]
fn app_limited_cubic() {
    let mut config = AppLimitedConfig::default_config(2);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    config.nb_losses_max = 64;
    config.data_rate_max = 4_013_000;
    app_limited_test_one(config);
}

/// C: `app_limited_reno_test` in `picoquictest/app_limited.c`.
#[test]
fn app_limited_reno() {
    let mut config = AppLimitedConfig::default_config(1);
    config.ccalgo = get_congestion_algorithm("newreno").expect("newreno cc algo");
    app_limited_test_one(config);
}

/// C: `app_limited_rpr_test` in `picoquictest/app_limited.c`.
#[test]
fn app_limited_rpr() {
    let mut config = AppLimitedConfig::default_config(4);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    config.do_preemptive_repeat = true;
    config.loss_mask = 0x1482_4812_2481_8214_u64;
    config.completion_target = 47_200_000;
    config.nb_losses_max = 1_980;
    config.rtt_max = 275_000;
    app_limited_test_one(config);
}
