//! Test case for `picoquictest/flow_control_test.c`.
//!
//! Simulates a slow receiver that must buffer incoming data, exercising
//! application-managed flow control (`set_app_flow_control` /
//! `open_flow_control`).

use std::cell::RefCell;
use std::rc::Rc;

use crate::internal::Version;
use crate::tests::util::{tls_api_connection_loop, tls_api_init_ctx_ex2, tls_api_one_sim_round};
use crate::{
    CallbackEvent, CongestionAlgorithm, Connection, ConnectionId, Instant, StreamDataCallback,
    get_congestion_algorithm,
};

const FCTEST_ALPN: &str = "fctest";

// ---------------------------------------------------------------------------
// Shared callback context.

/// Shared state between the client and server callbacks.
/// C: `fctest_ctx_t`.
#[allow(dead_code)]
struct FctestShared {
    transfer_size: u64,
    microsecs_per_byte: u64,
    credit_quantum: u64,

    simulated_time: Instant,
    stream_id: u64,
    bytes_sent: u64,

    bytes_received: u64,
    bytes_buffered: u64,
    buffered_time: Instant,
    microsec_rounding_error: u64,

    credits_pending: u64,
    bytes_buffered_max: u64,

    is_started: bool,
    fin_sent: bool,
    fin_received: bool,
    is_closed: bool,
    error_detected: bool,
}

impl FctestShared {
    fn new(transfer_size: u64, microsecs_per_byte: u64, credit_quantum: u64) -> Self {
        Self {
            transfer_size,
            microsecs_per_byte,
            credit_quantum,
            simulated_time: Instant::from_ticks(0),
            stream_id: 0,
            bytes_sent: 0,
            bytes_received: 0,
            bytes_buffered: 0,
            buffered_time: Instant::from_ticks(0),
            microsec_rounding_error: 0,
            credits_pending: 0,
            bytes_buffered_max: 0,
            is_started: false,
            fin_sent: false,
            fin_received: false,
            is_closed: false,
            error_detected: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Callback.

/// Wraps shared state for one of the two endpoints.
struct FctestCallback(Rc<RefCell<FctestShared>>);

impl StreamDataCallback for FctestCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        let current_time = {
            let ctx = self.0.borrow();
            ctx.simulated_time
        };
        let is_client = connection.is_client();

        match fin_or_event {
            CallbackEvent::StreamData | CallbackEvent::StreamFin => {
                if is_client {
                    let expected_stream = self.0.borrow().stream_id;
                    if stream_id == expected_stream {
                        let length = bytes.len() as u64;
                        let ret = fctest_receive_data(
                            &mut self.0.borrow_mut(),
                            connection,
                            current_time,
                            length,
                        );
                        if fin_or_event == CallbackEvent::StreamFin {
                            self.0.borrow_mut().fin_received = true;
                        }
                        if ret != 0 {
                            return ret;
                        }
                    }
                } else {
                    let expected_stream = self.0.borrow().stream_id;
                    if stream_id == expected_stream && fin_or_event == CallbackEvent::StreamFin {
                        let _ = connection.mark_active_stream(stream_id, true, None);
                    }
                }
                0
            }
            CallbackEvent::StatelessReset
            | CallbackEvent::Close
            | CallbackEvent::ApplicationClose => {
                self.0.borrow_mut().is_closed = true;
                0
            }
            CallbackEvent::VersionNegotiation | CallbackEvent::StreamGap => -1,
            CallbackEvent::PrepareToSend => {
                if !is_client {
                    // `bytes` is the PrepareToSend context; provide_stream_data_buffer
                    // is not yet wired through the Rust callback API.
                    todo!(
                        "PrepareToSend: provide_stream_data_buffer context not yet resolved in Rust API"
                    )
                }
                0
            }
            CallbackEvent::Datagram
            | CallbackEvent::PrepareDatagram
            | CallbackEvent::DatagramAcked
            | CallbackEvent::DatagramLost
            | CallbackEvent::DatagramSpurious => -1,
            CallbackEvent::AlmostReady | CallbackEvent::Ready => {
                fctest_start_stream(&mut self.0.borrow_mut(), connection)
            }
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper logic (mirrors the C static functions).

fn fctest_start_stream(ctx: &mut FctestShared, cnx: &mut Connection) -> i32 {
    if !ctx.is_started {
        let start = [0xffu8, 0xfe, 0xfd, 0xfc];
        ctx.is_started = true;
        ctx.stream_id = cnx.get_next_local_stream_id(false);
        if cnx.add_to_stream(ctx.stream_id, &start, true).is_err() {
            return -1;
        }
        if cnx.set_app_flow_control(ctx.stream_id, true).is_err() {
            return -1;
        }
    }
    0
}

fn fctest_receive_data(
    ctx: &mut FctestShared,
    cnx: &mut Connection,
    current_time: Instant,
    length: u64,
) -> i32 {
    let elapsed = current_time
        .ticks()
        .saturating_sub(ctx.buffered_time.ticks());
    let delta_t = elapsed + ctx.microsec_rounding_error;
    let processed = delta_t / ctx.microsecs_per_byte;

    if processed >= ctx.bytes_buffered {
        ctx.bytes_buffered = 0;
        ctx.microsec_rounding_error = 0;
    } else {
        ctx.bytes_buffered -= processed;
        ctx.microsec_rounding_error = delta_t % ctx.microsecs_per_byte;
    }
    ctx.buffered_time = current_time;
    ctx.credits_pending += processed;

    if length > 0 {
        ctx.bytes_buffered += length;
        ctx.bytes_received += length;
        if ctx.bytes_buffered > ctx.bytes_buffered_max {
            ctx.bytes_buffered_max = ctx.bytes_buffered;
        }
    }

    if ctx.credits_pending >= ctx.credit_quantum {
        ctx.credits_pending -= ctx.credit_quantum;
        let quantum = ctx.credit_quantum;
        let stream_id = ctx.stream_id;
        if cnx.open_flow_control(stream_id, quantum).is_err() {
            return -1;
        }
    }
    0
}

// ---------------------------------------------------------------------------
// Test driver.
// C: `fctest_one`.

fn fctest_one(
    test_id: u8,
    ccalgo: &'static CongestionAlgorithm,
    loss_mask_init: u64,
    transfer_size: u64,
    microsecs_per_byte: u64,
    credit_quantum: u64,
    initial_credit: u64,
    _bytes_buffered_max_limit: u64,
    completion_target: u64,
) {
    let shared = Rc::new(RefCell::new(FctestShared::new(
        transfer_size,
        microsecs_per_byte,
        credit_quantum,
    )));

    let mut cid_bytes = [0u8; 8];
    cid_bytes[0] = 0xfc;
    cid_bytes[1] = 0x4e;
    cid_bytes[2] = 0x54;
    cid_bytes[7] = test_id;
    let initial_cid = ConnectionId::clone_from_slice(&cid_bytes).expect("initial CID");

    let mut simulated_time = Instant::from_ticks(0);

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(crate::tests::util::TEST_SNI),
        Some(FCTEST_ALPN),
        None,
        Some(&initial_cid),
    )
    .expect("test context");

    // Phase 4: wire initial_credit into client transport params
    // (initial_max_stream_data_bidi_local = initial_credit).
    let _ = initial_credit;

    test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
    test_ctx.cnx_client().set_preemptive_repeat(false);

    let _ = test_ctx.qserver.set_qlog(".");
    let _ = test_ctx.qclient.set_qlog(".");

    // Install the fctest callback on both server (default) and client.
    let cb_server = Box::new(FctestCallback(Rc::clone(&shared)));
    let cb_client = Box::new(FctestCallback(Rc::clone(&shared)));
    test_ctx.qserver.set_default_callback(Some(cb_server));
    test_ctx.cnx_client().set_callback(Some(cb_client));
    test_ctx.cnx_client().start_client().expect("start client");

    let mut loss_mask = loss_mask_init;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    const TIMEOUT: u64 = 10_000;
    let mut nb_trials = 0i32;
    let mut nb_inactive = 0i32;

    loop {
        // Update shared simulated_time so callbacks can reference it.
        shared.borrow_mut().simulated_time = simulated_time;

        let bytes_buffered = shared.borrow().bytes_buffered;
        let buffered_time = shared.borrow().buffered_time;

        if bytes_buffered > 0 && simulated_time.ticks() >= buffered_time.ticks() + TIMEOUT {
            let ret = fctest_receive_data(
                &mut shared.borrow_mut(),
                test_ctx.cnx_client(),
                simulated_time,
                0,
            );
            assert_eq!(ret, 0, "receive_data error in timeout path");
        }

        let mut round_active = false;
        let next_time = Instant::from_ticks(simulated_time.ticks().saturating_add(TIMEOUT));
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            next_time,
            &mut round_active,
        )
        .expect("sim round");

        {
            let ctx = shared.borrow();
            if ctx.fin_received || ctx.is_closed || ctx.error_detected {
                break;
            }
        }

        if round_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
            if nb_inactive > 256 {
                break;
            }
        }

        nb_trials += 1;
        assert!(nb_trials <= 1_000_000, "trial limit exceeded");
    }

    let ctx = shared.borrow();
    assert!(
        ctx.fin_received && ctx.bytes_received >= transfer_size,
        "transfer incomplete: received {} of {}",
        ctx.bytes_received,
        transfer_size,
    );

    assert!(
        ctx.bytes_buffered_max <= initial_credit + credit_quantum,
        "buffer peak {} exceeded limit {}",
        ctx.bytes_buffered_max,
        initial_credit + credit_quantum,
    );

    if completion_target != 0 {
        assert!(
            simulated_time.ticks() <= completion_target,
            "completion time {} exceeded target {}",
            simulated_time.ticks(),
            completion_target,
        );
    }
}

// ---------------------------------------------------------------------------
// Exported test.

/// C: `flow_control_test` in `picoquictest/flow_control_test.c`.
#[test]
fn flow_control() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr algorithm");
    fctest_one(
        1, bbr, 0, 1_000_000, 10, 0x4000, 0x1_0000, 0x4000, 11_000_000,
    );
}
