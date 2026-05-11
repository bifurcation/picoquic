//! Test cases for `picoquictest/edge_cases.c`.
//!
//! A collection of regression tests for edge cases typically discovered
//! via the QUIC interop runner.  Many involve specific loss patterns during
//! 0-RTT handshakes, idle-timeout negotiation, stream-reset interaction,
//! initial-PTO behaviour, and crypto-handshake frame errors.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, TestTlsApiCtx, format_ack_frame_written, session_resume_wait_for_ticket,
    test_api_init_send_recv_scenario, test_api_queue_initial_queries, test_random,
    tls_api_close_with_losses, tls_api_connection_loop, tls_api_data_sending_loop,
    tls_api_init_ctx, tls_api_init_ctx_ex, tls_api_one_scenario_body_connect,
    tls_api_one_scenario_body_verify, tls_api_one_sim_round, tls_api_wait_for_timeout,
};
use crate::errors::{InternalError, TransportError};
use crate::frames::FrameType;
use crate::internal::{
    Epoch, MICROSEC_HANDSHAKE_MAX, PacketType, StreamDataBufferArgument, StreamDataNode, Version,
    connection_wake_key, pad_to_target_length, protect_packet_header, skip_frame,
    update_payload_length,
};
use crate::{
    CallbackEvent, Connection, ConnectionId, Duration, Error, INITIAL_MTU_IPV6, Instant,
    MAX_PACKET_SIZE, PacketContext, PmtudPolicy, State, StreamDataCallback,
    provide_stream_data_buffer,
};
use std::{cell::RefCell, rc::Rc};

// ---------------------------------------------------------------------------
// Shared scenarios.  Passed to helpers once those are implemented.

#[allow(dead_code)]
static SCENARIO_EDGE_CASE: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 128,
    r_len: 1_000,
}];

#[allow(dead_code)]
static SCENARIO_EDGE_RESET: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 1_000_000,
    r_len: 1_000_000,
}];

#[allow(dead_code)]
static SCENARIO_RESET_AT: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

/// Reset the test-harness scenario counters between the first 1-RTT pass and
/// the second 0-RTT connection.
/// C: `edge_case_reset_scenario` in `picoquictest/edge_cases.c`.
fn edge_case_reset_scenario(test_ctx: &mut TestTlsApiCtx) {
    test_ctx.test_finished = false;
    test_ctx.immediate_exit = false;
}

/// Set up a test context with the standard edge-case configuration: a
/// CID of the form `{0xed, 0x9e, 0xca, 0x5e, edge_case_id, zero_rtt, 0, 0}`,
/// optional 0-RTT first pass, and `nb_init_rounds` rounds of simulation
/// with the given `loss_mask`.
/// C: `edge_case_prepare` in `picoquictest/edge_cases.c`.
fn edge_case_prepare(
    edge_case_id: u8,
    zero_rtt: bool,
    simulated_time: &mut Instant,
    mut loss_mask: u64,
    nb_init_rounds: i32,
) -> crate::Result<Box<TestTlsApiCtx>> {
    const EDGE_CASE_TICKET_FILE: &str = "edge_case_ticket_file.bin";
    const EDGE_CASE_TOKEN_FILE: &str = "edge_case_token_file.bin";

    let mut initial_cid = ConnectionId::clone_from_slice(&[0xed, 0x9e, 0xca, 0x5e, 0, 0, 0, 0])
        .ok_or(Error::InvalidArgument)?;
    initial_cid.id[4] = edge_case_id;
    initial_cid.id[5] = zero_rtt as u8;

    std::fs::File::create(EDGE_CASE_TICKET_FILE).map_err(|_| Error::Generic)?;
    std::fs::File::create(EDGE_CASE_TOKEN_FILE).map_err(|_| Error::Generic)?;

    let mut test_ctx = tls_api_init_ctx_ex(
        simulated_time,
        Version::InternalTest1 as u32,
        Some(EDGE_CASE_TICKET_FILE),
        Some(&initial_cid),
    )
    .ok_or(Error::Generic)?;

    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.qserver.set_qlog(".").ok();

    let latency = 17_000;
    test_ctx.c_to_s_link.microsec_latency = latency;
    test_ctx.s_to_c_link.microsec_latency = latency;

    match edge_case_id {
        0xcf | 0xf1 | 0x5c => {
            test_ctx.cnx_client().local_parameters.min_ack_delay = Duration::from_ticks(0);
            test_ctx.cnx_client().set_pmtud_policy(PmtudPolicy::Blocked);
        }
        0xa1 => {
            test_ctx.cnx_client().local_parameters.min_ack_delay = Duration::from_ticks(0);
            test_ctx.qserver.test_large_server_flight = true;
            test_ctx.cnx_client().set_pmtud_policy(PmtudPolicy::Blocked);
        }
        _ => {
            test_ctx.cnx_client().set_pmtud_policy(PmtudPolicy::Delayed);
            test_ctx
                .qclient
                .set_default_pmtud_policy(PmtudPolicy::Delayed);
        }
    }

    test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_EDGE_CASE)?;

    if !zero_rtt {
        test_ctx.cnx_client().start_client()?;
    } else {
        tls_api_one_scenario_body_connect(&mut test_ctx, simulated_time, 0, 0)?;
        let mut zero_loss = 0u64;
        tls_api_data_sending_loop(&mut test_ctx, &mut zero_loss, simulated_time, 0)?;
        session_resume_wait_for_ticket(&mut test_ctx, simulated_time)?;
        tls_api_one_scenario_body_verify(&mut test_ctx, simulated_time, 1_000_000)?;

        let ticket_version = test_ctx
            .qclient
            .stored_tickets
            .first()
            .map(|ticket| ticket.version)
            .ok_or(Error::InvalidState)?;

        edge_case_reset_scenario(&mut test_ctx);
        initial_cid.id[5] = 0;
        let null_cid = ConnectionId::with_size(0).ok_or(Error::InvalidArgument)?;
        let server_addr = test_ctx.server_addr;
        let cnx = test_ctx
            .qclient
            .create_connection(
                initial_cid,
                null_cid,
                Some(&server_addr),
                *simulated_time,
                ticket_version,
                Some(super::util::TEST_SNI),
                Some(super::util::TEST_ALPN),
                true,
            )
            .ok_or(Error::Memory)?;
        cnx.start_client()?;
        test_api_queue_initial_queries(&mut test_ctx, 0)?;
    }

    let nb_init_rounds = nb_init_rounds.max(0) as u32;
    let loss_target = if nb_init_rounds == 0 {
        loss_mask
    } else {
        loss_mask.rotate_right(nb_init_rounds)
    };

    test_ctx.c_to_s_link.loss_mask = Some(loss_mask);
    test_ctx.s_to_c_link.loss_mask = Some(loss_mask);

    if edge_case_id != 0xf1 && edge_case_id != 0x5c {
        test_ctx.qserver.set_preemptive_repeat_policy(true);
    }

    let mut nb_trials = 0;
    let mut nb_inactive = 0;
    while nb_trials < 4 * nb_init_rounds as i32 && nb_inactive < 256 && loss_mask != loss_target {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round(
            &mut test_ctx,
            simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
        if test_ctx.test_finished {
            break;
        }
        if let Some(mask) = test_ctx.c_to_s_link.loss_mask {
            loss_mask = mask;
        }
    }

    Ok(test_ctx)
}

/// Finish an edge-case test: run the connection loop, enable
/// `immediate_exit`, drive data sending, and verify completion within
/// `duration_max` µs.
/// C: `edge_case_complete` in `picoquictest/edge_cases.c`.
fn edge_case_complete(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    duration_max: u64,
) -> crate::Result<()> {
    let mut loss_mask = 0u64;
    tls_api_connection_loop(test_ctx, &mut loss_mask, 0, simulated_time)?;
    test_ctx.immediate_exit = true;
    tls_api_data_sending_loop(test_ctx, &mut loss_mask, simulated_time, 0)?;
    tls_api_one_scenario_body_verify(test_ctx, simulated_time, duration_max)
}

// ---------------------------------------------------------------------------
// Reset-repeat test.

/// Variant of the reset-repeat test: which frame event to exercise.
/// C: `reset_test_enum` in `picoquictest/edge_cases.c`.
#[derive(Copy, Clone)]
enum ResetTestKind {
    AckMaxStream = 0,
    AckReset = 1,
    // C value 2 is reset_ack_stop_sending; that case is intentionally
    // not tested because C has no ACK processing for STOP_SENDING frames.
    ExtraMaxStream = 3,
    ExtraReset = 4,
    ExtraStop = 5,
    NeedMaxStream = 6,
    NeedReset = 7,
    NeedStop = 8,
}

fn stream_sent_offset(connection: &mut Connection, stream_id: u64) -> Option<u64> {
    let token = connection.find_stream(stream_id)?;
    connection
        .streams
        .get(token)
        .map(|stream| stream.sent_offset)
}

fn stream_fin_sent(connection: &mut Connection, stream_id: u64) -> Option<bool> {
    let token = connection.find_stream(stream_id)?;
    connection.streams.get(token).map(|stream| stream.fin_sent)
}

fn reset_loop_check_stream_opened(test_ctx: &mut TestTlsApiCtx, data_stream_id: u64) -> bool {
    if !test_ctx.has_cnx_server() {
        return false;
    }
    let client_sent = stream_sent_offset(test_ctx.cnx_client(), data_stream_id).unwrap_or(0);
    let server_has_stream = test_ctx.cnx_server().find_stream(data_stream_id).is_some();
    client_sent > 10_000 && server_has_stream
}

fn reset_loop_wait_stream_opened(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    data_stream_id: u64,
    loop1_time: u64,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + loop1_time);
    let mut nb_inactive = 0;
    while simulated_time.ticks() < time_out.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
        && nb_inactive < 64
    {
        if reset_loop_check_stream_opened(test_ctx, data_stream_id) {
            return Ok(());
        }
        let mut was_active = false;
        tls_api_one_sim_round(
            test_ctx,
            simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }
    Err(Error::InvalidState)
}

const RESET_LOOP_TARGET_BYTES: u64 = 1_000_000;

#[derive(Default)]
struct ResetLoopState {
    data_sent: [u64; 4],
    data_received: [u64; 4],
    fin_received: [i32; 4],
    reset_received: [i32; 4],
    prepare_to_send: [u64; 4],
}

struct ResetLoopCallback {
    state: Rc<RefCell<ResetLoopState>>,
}

fn reset_loop_stream_rank(connection: &Connection, stream_id: u64) -> Option<usize> {
    let base = (stream_id / 2).checked_sub(2)?;
    let rank = base + u64::from(connection.is_client());
    (rank < 4).then_some(rank as usize)
}

fn reset_loop_prepare_to_send(
    state: &mut ResetLoopState,
    stream_rank: usize,
    context: &mut StreamDataBufferArgument<'_>,
) -> i32 {
    if stream_rank >= state.data_sent.len() {
        return -1;
    }

    let space = context.allowed_space;
    let is_fin =
        state.data_sent[stream_rank].saturating_add(space as u64) > RESET_LOOP_TARGET_BYTES;
    let Some(buffer) = provide_stream_data_buffer(context, space, is_fin, !is_fin) else {
        return -1;
    };
    buffer.fill(b'a'.saturating_add(stream_rank as u8));
    state.prepare_to_send[stream_rank] = state.prepare_to_send[stream_rank].saturating_add(1);
    state.data_sent[stream_rank] = state.data_sent[stream_rank].saturating_add(space as u64);
    0
}

impl StreamDataCallback for ResetLoopCallback {
    fn prepare_to_send<'a>(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        context: &mut StreamDataBufferArgument<'a>,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        let Some(stream_rank) = reset_loop_stream_rank(connection, stream_id) else {
            return -1;
        };
        reset_loop_prepare_to_send(&mut self.state.borrow_mut(), stream_rank, context)
    }

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
                let Some(stream_rank) = reset_loop_stream_rank(connection, stream_id) else {
                    return -1;
                };
                let should_mark_active = {
                    let mut state = self.state.borrow_mut();
                    state.data_received[stream_rank] =
                        state.data_received[stream_rank].saturating_add(bytes.len() as u64);
                    if fin_or_event == CallbackEvent::StreamFin {
                        state.fin_received[stream_rank] += 1;
                    }
                    !connection.is_client() && state.data_sent[stream_rank] == 0
                };
                if should_mark_active
                    && connection
                        .mark_active_stream(stream_id, true, Some(Box::new(Rc::clone(&self.state))))
                        .is_err()
                {
                    return -1;
                }
                0
            }
            CallbackEvent::PrepareToSend => -1,
            CallbackEvent::StreamReset => {
                let Some(stream_rank) = reset_loop_stream_rank(connection, stream_id) else {
                    return -1;
                };
                self.state.borrow_mut().reset_received[stream_rank] += 1;
                0
            }
            CallbackEvent::StopSending => connection.reset_stream(stream_id, 0).map_or(-1, |_| 0),
            CallbackEvent::PrepareDatagram => -1,
            CallbackEvent::StatelessReset
            | CallbackEvent::Close
            | CallbackEvent::ApplicationClose => {
                connection.set_callback(None);
                0
            }
            _ => 0,
        }
    }
}

fn reset_repeat_test_receive_frame(
    cnx: &mut Connection,
    frame: &[u8],
    simulated_time: Instant,
    stream_id: u64,
    do_not_create: bool,
) -> crate::Result<()> {
    let mut received_data = StreamDataNode {
        stream_data_membership: None,
        offset: 0,
        data: [0u8; crate::internal::MAX_PACKET_SIZE],
        length: 0,
    };

    let ret = if cnx.paths.is_empty() {
        let mut consumed = 0usize;
        let mut pure_ack = 0i32;
        skip_frame(frame, frame.len(), &mut consumed, &mut pure_ack)
    } else {
        let mut path = cnx.paths.remove(0);
        let ret = cnx.decode_frames(
            &mut path,
            frame,
            frame.len(),
            &mut received_data,
            crate::internal::Epoch::OneRtt as i32,
            None,
            None,
            123,
            0,
            simulated_time,
        );
        cnx.paths.insert(0, path);
        ret
    };

    if ret != 0 || cnx.state() > State::Ready {
        return Err(Error::InvalidFrame);
    }
    if stream_id != u64::MAX && do_not_create && cnx.find_stream(stream_id).is_some() {
        return Err(Error::InvalidState);
    }
    Ok(())
}

fn reset_repeat_test_need_repeat(cnx: &mut Connection, frame: &[u8]) -> crate::Result<()> {
    let mut no_need_to_repeat = 0;
    let mut do_not_detect_spurious = 0;
    let mut is_preemptive_needed = 0;
    let ret = cnx.check_frame_needs_repeat(
        frame,
        frame.len(),
        PacketType::OneRttProtected,
        &mut no_need_to_repeat,
        &mut do_not_detect_spurious,
        &mut is_preemptive_needed,
    );
    if ret != 0 || cnx.state() > State::Ready || no_need_to_repeat == 0 {
        return Err(Error::InvalidFrame);
    }
    Ok(())
}

fn reset_repeat_process_ack_of_max_stream_data_frame(
    cnx: &mut Connection,
    bytes: &[u8],
    consumed: &mut usize,
) -> i32 {
    if bytes.is_empty() {
        *consumed = bytes.len();
        return -1;
    }
    let Some((tail, stream_id)) = crate::utils::frames_varint_decode(&bytes[1..]) else {
        *consumed = bytes.len();
        return -1;
    };
    let Some((tail, maxdata)) = crate::utils::frames_varint_decode(tail) else {
        *consumed = bytes.len();
        return -1;
    };
    *consumed = bytes.len() - tail.len();

    if let Some(stream_token) = cnx.find_stream(stream_id)
        && let Some(stream) = cnx.streams.get_mut(stream_token)
        && maxdata > stream.maxdata_local_acked
    {
        stream.maxdata_local_acked = maxdata;
    }

    0
}

fn reset_repeat_delete_stream_if_closed(
    cnx: &mut Connection,
    stream_token: crate::internal::StreamToken,
) -> i32 {
    use crate::stream::{Role, StreamId};

    let client_mode = cnx.client_mode;
    let local_role = if client_mode {
        Role::Client
    } else {
        Role::Server
    };

    let mut ret = 0;
    let mut stream_id = 0;
    let mut remove_tree_entry = None;
    let mut remove_stream = false;
    let mut mark_max_stream_updated = false;

    if let Some(stream) = cnx.streams.get_mut(stream_token) {
        stream_id = stream.stream_id;
        let sid = StreamId(stream_id);
        if !stream.is_closed && stream.is_stream_closed(client_mode) {
            stream.is_closed = true;
            ret = 1;
            if !sid.is_local(local_role) {
                if sid.is_bidir() && stream_id >= cnx.max_stream_id_bidir_local {
                    let old = cnx.max_stream_id_bidir_local;
                    cnx.max_stream_id_bidir_local_computed = cnx
                        .max_stream_id_bidir_local_computed
                        .max(stream_id.saturating_add(4));
                    mark_max_stream_updated = cnx.max_stream_id_bidir_local_computed > old;
                } else if !sid.is_bidir() && stream_id >= cnx.max_stream_id_unidir_local {
                    let old = cnx.max_stream_id_unidir_local;
                    cnx.max_stream_id_unidir_local_computed = cnx
                        .max_stream_id_unidir_local_computed
                        .max(stream_id.saturating_add(4));
                    mark_max_stream_updated = cnx.max_stream_id_unidir_local_computed > old;
                }
            }
        }

        let sid = StreamId(stream_id);
        let is_remote_unidir = !sid.is_bidir() && !sid.is_local(local_role);
        let stream_acked = if stream.reset_sent {
            stream.reset_acked
        } else {
            stream.sack_list.check(0, stream.sent_offset)
        };
        if stream.is_closed && (is_remote_unidir || stream_acked) {
            remove_tree_entry = stream.stream_tree_membership.take();
            stream.is_output_stream = false;
            remove_stream = true;
        } else if mark_max_stream_updated {
            stream.max_stream_updated = true;
        }
    }

    if remove_stream {
        if let Some(pos) = cnx.output_streams.iter().position(|&t| t == stream_token) {
            cnx.output_streams.remove(pos);
        }
        if remove_tree_entry.is_none() {
            remove_tree_entry = cnx.stream_tree.find(&stream_id);
        }
        if let Some(st) = remove_tree_entry {
            cnx.stream_tree.remove(st);
        }
        cnx.streams.remove(stream_token);
    }

    ret
}

fn reset_repeat_process_ack_of_reset_stream_frame(
    cnx: &mut Connection,
    bytes: &[u8],
    consumed: &mut usize,
) -> i32 {
    if bytes.is_empty() {
        *consumed = bytes.len();
        return -1;
    }
    let Some((tail, stream_id)) = crate::utils::frames_varint_decode(&bytes[1..]) else {
        *consumed = bytes.len();
        return -1;
    };
    let Some((tail, _error_code)) = crate::utils::frames_varint_decode(tail) else {
        *consumed = bytes.len();
        return -1;
    };
    let Some((tail, _final_offset)) = crate::utils::frames_varint_decode(tail) else {
        *consumed = bytes.len();
        return -1;
    };
    *consumed = bytes.len() - tail.len();

    if let Some(stream_token) = cnx.find_stream(stream_id) {
        if let Some(stream) = cnx.streams.get_mut(stream_token) {
            stream.reset_acked = true;
        }
        reset_repeat_delete_stream_if_closed(cnx, stream_token);
    }

    0
}

/// Establish a connection, start a large stream transfer, reset the stream
/// on both sides, wait for the stream context to be freed, then exercise
/// the specified post-reset frame event.
/// C: `reset_repeat_test_one` in `picoquictest/edge_cases.c`.
fn reset_repeat_test_one(kind: ResetTestKind) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let data_stream_id = 4u64;
    let loop1_time = 50_000;
    let loop2_time = 1_000_000;
    let mut initial_cid = ConnectionId::clone_from_slice(&[0x8e, 0x5e, 0x48, 0xe9, 0, 0, 0, 0])
        .ok_or(Error::InvalidArgument)?;
    initial_cid.id[4] = kind as u8;

    let stop_sending_frame = [FrameType::StopSending as u8, data_stream_id as u8, 0x17];
    let max_stream_data_frame = [FrameType::MaxStreamData as u8, data_stream_id as u8, 63];
    let reset_frame = [FrameType::ResetStream as u8, data_stream_id as u8, 1, 1];

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .ok_or(Error::Generic)?;
    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.cnx_client().start_client()?;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_EDGE_RESET)?;
    reset_loop_wait_stream_opened(
        &mut test_ctx,
        &mut simulated_time,
        data_stream_id,
        loop1_time,
    )?;

    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        return Err(Error::InvalidState);
    }
    if stream_fin_sent(test_ctx.cnx_client(), data_stream_id).unwrap_or(true) {
        return Err(Error::InvalidState);
    }

    test_ctx.cnx_client().reset_stream(data_stream_id, 0)?;
    test_ctx.cnx_server().reset_stream(data_stream_id, 0)?;

    let mut nb_inactive = 0;
    let time_out = Instant::from_ticks(simulated_time.ticks() + loop2_time);
    let mut client_stream_gone = false;
    while simulated_time.ticks() < time_out.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
        && nb_inactive < 64
    {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            time_out,
            &mut was_active,
        )?;
        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
        if test_ctx.cnx_client().find_stream(data_stream_id).is_none() {
            client_stream_gone = true;
            break;
        }
    }
    if !client_stream_gone {
        return Err(Error::InvalidState);
    }

    match kind {
        ResetTestKind::AckMaxStream => {
            let mut consumed = 0usize;
            if reset_repeat_process_ack_of_max_stream_data_frame(
                test_ctx.cnx_client(),
                &reset_frame,
                &mut consumed,
            ) != 0
                || test_ctx.cnx_client().state() > State::Ready
            {
                return Err(Error::InvalidFrame);
            }
        }
        ResetTestKind::AckReset => {
            let mut consumed = 0usize;
            if reset_repeat_process_ack_of_reset_stream_frame(
                test_ctx.cnx_client(),
                &reset_frame,
                &mut consumed,
            ) != 0
                || test_ctx.cnx_client().state() > State::Ready
            {
                return Err(Error::InvalidFrame);
            }
        }
        ResetTestKind::ExtraMaxStream => reset_repeat_test_receive_frame(
            test_ctx.cnx_client(),
            &max_stream_data_frame,
            simulated_time,
            data_stream_id,
            true,
        )?,
        ResetTestKind::ExtraReset => reset_repeat_test_receive_frame(
            test_ctx.cnx_client(),
            &reset_frame,
            simulated_time,
            data_stream_id,
            true,
        )?,
        ResetTestKind::ExtraStop => reset_repeat_test_receive_frame(
            test_ctx.cnx_client(),
            &stop_sending_frame,
            simulated_time,
            data_stream_id,
            true,
        )?,
        ResetTestKind::NeedMaxStream => {
            reset_repeat_test_need_repeat(test_ctx.cnx_client(), &max_stream_data_frame)?
        }
        ResetTestKind::NeedReset => {
            reset_repeat_test_need_repeat(test_ctx.cnx_client(), &reset_frame)?
        }
        ResetTestKind::NeedStop => {
            reset_repeat_test_need_repeat(test_ctx.cnx_client(), &stop_sending_frame)?
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Idle-timeout test helpers.

/// Verify idle-timeout negotiation: `client_timeout` and `server_timeout`
/// (in ms) must negotiate to `expected_timeout` (in µs); assert the
/// connection remains alive at half-time and drops by full-time.
/// C: `idle_timeout_test_one` in `picoquictest/edge_cases.c`.
fn idle_timeout_test_one(
    test_id: u8,
    client_timeout: u64,
    server_timeout: u64,
    expected_timeout: u64,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut initial_cid = ConnectionId::clone_from_slice(&[0x41, 0x9e, 0x00, 0x94, 0, 0, 0, 0])
        .ok_or(Error::InvalidArgument)?;
    initial_cid.id[4] = test_id;
    let half_time = if expected_timeout == u64::MAX {
        20_000_000
    } else {
        expected_timeout / 2
    };
    let full_time = if expected_timeout == u64::MAX {
        600_000_000
    } else {
        half_time + 100_000
    };

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .ok_or(Error::Generic)?;
    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx
        .qclient
        .set_default_idle_timeout(Duration::from_ticks(client_timeout));
    test_ctx
        .qserver
        .set_default_idle_timeout(Duration::from_ticks(server_timeout));
    test_ctx.cnx_client().local_parameters.max_idle_timeout = Duration::from_ticks(client_timeout);
    test_ctx.cnx_client().max_early_data_size = 0;
    test_ctx.cnx_client().start_client()?;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    if test_ctx
        .cnx_client()
        .local_parameters
        .max_idle_timeout
        .ticks()
        != client_timeout
    {
        return Err(Error::InvalidState);
    }
    if test_ctx
        .cnx_server()
        .local_parameters
        .max_idle_timeout
        .ticks()
        != server_timeout
    {
        return Err(Error::InvalidState);
    }
    if test_ctx.cnx_client().idle_timeout.ticks() != expected_timeout {
        return Err(Error::InvalidState);
    }
    if test_ctx.cnx_server().idle_timeout.ticks() != expected_timeout {
        return Err(Error::InvalidState);
    }

    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, half_time)?;
    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        return Err(Error::InvalidState);
    }

    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, full_time)?;
    if test_ctx.client_ready() && test_ctx.server_ready() {
        if expected_timeout != u64::MAX {
            return Err(Error::InvalidState);
        }
    } else if expected_timeout == u64::MAX {
        return Err(Error::InvalidState);
    }
    Ok(())
}

/// Verify that a client connecting to a non-responding server disconnects
/// after `expected_timeout` µs (derived from `client_timeout` ms or
/// `handshake_timeout` µs).
/// C: `idle_server_test_one` in `picoquictest/edge_cases.c`.
fn idle_server_test_one(
    test_id: u8,
    client_timeout: u64,
    handshake_timeout: u64,
    expected_timeout: u64,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut initial_cid = ConnectionId::clone_from_slice(&[0x41, 0x9e, 0xc0, 0x99, 0, 0, 0, 0])
        .ok_or(Error::InvalidArgument)?;
    initial_cid.id[4] = test_id;
    let target_timeout = if handshake_timeout != 0 {
        handshake_timeout
    } else if client_timeout != 0 {
        client_timeout * 1000
    } else {
        MICROSEC_HANDSHAKE_MAX.ticks()
    };

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .ok_or(Error::Generic)?;
    test_ctx.qclient.set_qlog(".").ok();
    test_ctx
        .qclient
        .set_default_idle_timeout(Duration::from_ticks(client_timeout));
    if handshake_timeout > 0 {
        test_ctx
            .qclient
            .set_default_handshake_timeout(Duration::from_ticks(handshake_timeout));
    }
    test_ctx.cnx_client().local_parameters.max_idle_timeout = Duration::from_ticks(client_timeout);
    test_ctx.cnx_client().start_client()?;

    let mut send_buffer = [0u8; MAX_PACKET_SIZE];
    let mut nb_trials = 0;
    let mut disconnected = false;
    while simulated_time.ticks() < expected_timeout {
        let result = test_ctx
            .cnx_client()
            .prepare_packet_ex(simulated_time, &mut send_buffer);
        match result {
            Ok(_) => {}
            Err(Error::Disconnected) => {
                disconnected = true;
                break;
            }
            Err(e) => return Err(e),
        }
        if test_ctx.cnx_client().state() == State::Disconnected {
            disconnected = true;
            break;
        }
        let next_wake = test_ctx.cnx_client().next_wake_time;
        if simulated_time > next_wake {
            return Err(Error::InvalidState);
        }
        if nb_trials >= 512 {
            return Err(Error::InvalidState);
        }
        nb_trials += 1;
        simulated_time = next_wake;
    }

    if disconnected || test_ctx.cnx_client().state() == State::Disconnected {
        if simulated_time.ticks() < target_timeout {
            Err(Error::InvalidState)
        } else {
            Ok(())
        }
    } else {
        Err(Error::InvalidState)
    }
}

// ---------------------------------------------------------------------------
// Initial-PTO helpers.

/// Prepare the client's initial packet and submit it to the server to
/// establish a crypto context on both sides.
/// C: `initial_pto_prepare` in `picoquictest/edge_cases.c`.
fn initial_pto_prepare(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<usize> {
    let mut buf = [0u8; MAX_PACKET_SIZE];
    let prepared = test_ctx
        .cnx_client()
        .prepare_packet(*simulated_time, &mut buf)?;
    let length = prepared.send_length;
    let addr_from = if prepared.addr_from.ip().is_unspecified() {
        test_ctx.client_addr
    } else {
        prepared.addr_from
    };
    let addr_to = if prepared.addr_to.ip().is_unspecified() {
        test_ctx.server_addr
    } else {
        prepared.addr_to
    };
    let accepted = test_ctx
        .qserver
        .incoming_packet_ex(
            &mut buf[..length],
            &addr_from,
            &addr_to,
            0,
            0,
            *simulated_time,
        )?
        .is_some();
    if !accepted && !test_ctx.has_cnx_server() {
        return Err(Error::InvalidState);
    }
    Ok(length)
}

/// Advance time until the client's next scheduled send, returning the packet
/// length.  Stops early if the client enters a failure state.
/// C: `initial_pto_wait` in `picoquictest/edge_cases.c`.
fn initial_pto_wait(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    max_wait: u64,
) -> crate::Result<usize> {
    while simulated_time.ticks() < max_wait {
        let client_departure = test_ctx.cnx_client().next_wake_time.ticks();
        if client_departure >= max_wait {
            *simulated_time = Instant::from_ticks(max_wait);
            break;
        }
        if simulated_time.ticks() < client_departure {
            *simulated_time = Instant::from_ticks(client_departure);
        }
        let length = initial_pto_prepare(test_ctx, simulated_time)?;
        if length > 0 || test_ctx.cnx_client().state() >= State::HandshakeFailure {
            return Ok(length);
        }
    }
    Ok(0)
}

/// Craft a synthetic server ACK (Initial epoch) and deliver it to the client.
/// C: `initial_pto_ack` in `picoquictest/edge_cases.c`.
fn initial_pto_ack(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let mut packet = [0u8; INITIAL_MTU_IPV6];
    let mut send_buffer = [0u8; MAX_PACKET_SIZE];
    let send_length = {
        let server = test_ctx.cnx_server();
        let checksum_overhead = server.get_checksum_length(Epoch::Initial);
        let bytes_max = INITIAL_MTU_IPV6
            .checked_sub(checksum_overhead)
            .ok_or(Error::BufferTooSmall)?;
        let sequence_number = 0;
        let header_length =
            server.predict_packet_header_length_for_pc(PacketType::Initial, PacketContext::Initial);
        let mut pn_offset = 0usize;
        let mut pn_length = 0usize;
        let written = server.create_packet_header_at(
            PacketType::Initial,
            sequence_number,
            0,
            0,
            header_length,
            &mut packet,
            &mut pn_offset,
            &mut pn_length,
        );
        if written != header_length || header_length >= bytes_max {
            return Err(Error::BufferTooSmall);
        }
        let mut more_data = 0;
        let ack_length = format_ack_frame_written(
            server,
            &mut packet[header_length..bytes_max],
            &mut more_data,
            *simulated_time,
            PacketContext::Initial,
            0,
        )
        .ok_or(Error::InvalidFrame)?;
        let packet_length =
            pad_to_target_length(&mut packet, header_length + ack_length, bytes_max);
        let protected_length = packet_length
            .checked_add(checksum_overhead)
            .ok_or(Error::BufferTooSmall)?;
        if protected_length > send_buffer.len() {
            return Err(Error::BufferTooSmall);
        }

        let aead = server.crypto_context[Epoch::Initial as usize]
            .aead_encrypt
            .as_deref()
            .ok_or(Error::Tls)?;
        let pn_enc = server.crypto_context[Epoch::Initial as usize]
            .pn_enc
            .as_deref()
            .ok_or(Error::Tls)?;

        update_payload_length(
            &mut packet,
            pn_offset,
            header_length.saturating_sub(pn_length),
            protected_length,
        );
        let header = packet[..header_length].to_vec();
        let mut payload = packet[header_length..packet_length].to_vec();
        aead.encrypt(sequence_number, &header, &mut payload);
        let send_length = header_length + payload.len();
        if send_length > send_buffer.len() {
            return Err(Error::BufferTooSmall);
        }
        send_buffer[..header_length].copy_from_slice(&header);
        send_buffer[header_length..send_length].copy_from_slice(&payload);
        protect_packet_header(&mut send_buffer[..send_length], pn_offset, 0x0f, pn_enc);
        send_length
    };
    if send_length == 0 {
        return Err(Error::InvalidState);
    }
    let _ = test_ctx.qclient.incoming_packet_ex(
        &mut send_buffer[..send_length],
        &test_ctx.server_addr,
        &test_ctx.client_addr,
        0,
        0,
        *simulated_time,
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Crypto-HS-offset helper.

/// Inject a crypto handshake frame with a 64 KB offset into packet context
/// `pc`, attempt the connection, and verify the client receives
/// CRYPTO_BUFFER_EXCEEDED or IDLE_TIMEOUT.
/// C: `crypto_hs_offset_test_one` in `picoquictest/edge_cases.c`.
fn wait_client_connection_timeout(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    timeout_value: u64,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + timeout_value);
    let mut nb_trials = 0;
    let mut nb_inactive = 0;
    while simulated_time.ticks() < time_out.ticks()
        && test_ctx.cnx_client().state() < State::Ready
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
    if test_ctx.cnx_client().state() == State::Ready {
        Ok(())
    } else {
        Err(Error::InvalidState)
    }
}

fn crypto_hs_offset_one(pc: PacketContext) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut initial_cid = ConnectionId::clone_from_slice(&[0xc0, 0xff, 0x5e, 0x40, 0, 0, 0, 0])
        .ok_or(Error::InvalidArgument)?;
    initial_cid.id[4] = pc as u8;
    let bad_crypto_hs = [FrameType::CryptoHs as u8, 0x80, 0x01, 0, 0, 4, 1, 2, 3, 4];

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .ok_or(Error::Generic)?;
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.cnx_client().start_client()?;
    test_ctx
        .cnx_client()
        .queue_misc_frame(&bad_crypto_hs, true, pc)?;

    let _ = wait_client_connection_timeout(&mut test_ctx, &mut simulated_time, 300_000_000);
    let server_ok = !test_ctx.has_cnx_server()
        || test_ctx.cnx_server().state() == State::HandshakeFailure
        || test_ctx.cnx_server().state() >= State::Disconnecting;
    if !server_ok {
        return Err(Error::InvalidState);
    }
    let client_remote = test_ctx.cnx_client().remote_error();
    let client_local = test_ctx.cnx_client().local_error();
    if client_remote == TransportError::CryptoBufferExceeded as u64
        || client_local == InternalError::IdleTimeout as u64
    {
        Ok(())
    } else {
        Err(Error::InvalidState)
    }
}

// ---------------------------------------------------------------------------
// Reset-stream-at helper.

/// Variant for `reset_stream_at_test_one`.
/// C: `reset_stream_at_test_enum` in `picoquictest/edge_cases.c`.
#[derive(Copy, Clone)]
enum ResetStreamAtSpec {
    /// Reset just past what was already received.
    Basic,
    /// Reset exactly at the sent offset.
    Limit,
    /// Reset past what was received with a lossy link.
    Loss,
}

struct ResetStreamAtState {
    stream_id: u64,
    q_len: usize,
    r_len: usize,
    q_recv_nb: usize,
    r_recv_nb: usize,
    q_received: bool,
    r_received: bool,
    q_src: Vec<u8>,
    q_rcv: Vec<u8>,
    r_src: Vec<u8>,
    r_rcv: Vec<u8>,
    error_detected: bool,
}

impl ResetStreamAtState {
    fn new(stream_id: u64, q_len: usize, r_len: usize) -> Self {
        fn source_bytes(len: usize) -> Vec<u8> {
            (0..len).map(|i| i as u8).collect()
        }

        Self {
            stream_id,
            q_len,
            r_len,
            q_recv_nb: 0,
            r_recv_nb: 0,
            q_received: false,
            r_received: false,
            q_src: source_bytes(q_len),
            q_rcv: vec![0; q_len],
            r_src: source_bytes(r_len),
            r_rcv: vec![0; r_len],
            error_detected: false,
        }
    }

    fn receive_stream_data(
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        buffer: &mut [u8],
        max_len: usize,
        reference: &[u8],
        nb_received: &mut usize,
        received: &mut bool,
        error_detected: &mut bool,
    ) {
        if nb_received.saturating_add(bytes.len()) > max_len {
            *error_detected = true;
        } else {
            let start = *nb_received;
            let end = start + bytes.len();
            buffer[start..end].copy_from_slice(bytes);
            if reference[start..end] != *bytes {
                *error_detected = true;
            }
        }

        *nb_received = nb_received.saturating_add(bytes.len());

        if fin_or_event != CallbackEvent::StreamData {
            if *received {
                *error_detected = true;
            }
            *received = true;
        }
    }

    fn receive_query(&mut self, bytes: &[u8], fin_or_event: CallbackEvent) {
        Self::receive_stream_data(
            bytes,
            fin_or_event,
            &mut self.q_rcv,
            self.q_len,
            &self.q_src,
            &mut self.q_recv_nb,
            &mut self.q_received,
            &mut self.error_detected,
        );
    }

    fn receive_response(&mut self, bytes: &[u8], fin_or_event: CallbackEvent) {
        Self::receive_stream_data(
            bytes,
            fin_or_event,
            &mut self.r_rcv,
            self.r_len,
            &self.r_src,
            &mut self.r_recv_nb,
            &mut self.r_received,
            &mut self.error_detected,
        );
    }

    fn scenario_finished(&self) -> bool {
        !self.error_detected && self.q_received && self.r_received && self.q_recv_nb == self.q_len
    }
}

struct ResetStreamAtCallback {
    state: Rc<RefCell<ResetStreamAtState>>,
    client_mode: bool,
}

impl StreamDataCallback for ResetStreamAtCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        if matches!(
            fin_or_event,
            CallbackEvent::Close
                | CallbackEvent::ApplicationClose
                | CallbackEvent::AlmostReady
                | CallbackEvent::Ready
        ) {
            return 0;
        }

        let expected_stream_id = self.state.borrow().stream_id;
        if stream_id != expected_stream_id {
            self.state.borrow_mut().error_detected = true;
            return 0;
        }

        match fin_or_event {
            CallbackEvent::StopSending => {
                if connection.reset_stream(stream_id, 0).is_err() {
                    self.state.borrow_mut().error_detected = true;
                }
            }
            CallbackEvent::StreamData | CallbackEvent::StreamFin | CallbackEvent::StreamReset => {
                if self.client_mode {
                    self.state
                        .borrow_mut()
                        .receive_response(bytes, fin_or_event);
                } else {
                    let should_send_response = {
                        let mut state = self.state.borrow_mut();
                        state.receive_query(bytes, fin_or_event);
                        fin_or_event != CallbackEvent::StreamData
                            && !state.error_detected
                            && state.r_len > 0
                            && fin_or_event != CallbackEvent::StreamReset
                    };

                    if should_send_response {
                        let response = self.state.borrow().r_src.clone();
                        if connection
                            .add_to_stream(stream_id, &response, true)
                            .is_err()
                        {
                            self.state.borrow_mut().error_detected = true;
                        }
                    } else if fin_or_event == CallbackEvent::StreamReset {
                        self.state.borrow_mut().r_received = true;
                    }
                }
            }
            _ => {
                self.state.borrow_mut().error_detected = true;
            }
        }

        0
    }
}

/// Establish a connection, start sending a 1 MB stream, wait a short
/// time, then call `reset_stream_at` with the appropriate `reliable_size`.
/// Verify that the scenario completes and that `reliable_size` bytes were
/// delivered.
/// C: `reset_stream_at_test_one` in `picoquictest/edge_cases.c`.
fn reset_stream_at_test_one(spec: ResetStreamAtSpec) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut initial_cid = ConnectionId::clone_from_slice(&[0x8e, 0x5e, 0x57, 0xa7, 0, 0, 0, 0])
        .ok_or(Error::InvalidArgument)?;
    initial_cid.id[4] = spec as u8;
    let reset_at_state = Rc::new(RefCell::new(ResetStreamAtState::new(
        SCENARIO_RESET_AT[0].stream_id,
        SCENARIO_RESET_AT[0].q_len,
        SCENARIO_RESET_AT[0].r_len,
    )));

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .ok_or(Error::Generic)?;
    test_ctx.qserver.default_tp.is_reset_stream_at_enabled = true;
    test_ctx
        .cnx_client()
        .local_parameters
        .is_reset_stream_at_enabled = true;
    test_ctx.qserver.use_long_log = true;
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qclient.use_long_log = true;
    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.cnx_client().start_client()?;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    let client_reset_at_enabled = test_ctx.cnx_client().is_reset_stream_at_enabled;
    let server_reset_at_enabled = test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| cnx.is_reset_stream_at_enabled)
        .unwrap_or(false);
    if !client_reset_at_enabled || !server_reset_at_enabled {
        return Err(Error::InvalidState);
    }

    if matches!(spec, ResetStreamAtSpec::Loss) {
        loss_mask = 0xf0a0_5007_030c_9042;
    }

    test_ctx
        .cnx_server()
        .set_callback(Some(Box::new(ResetStreamAtCallback {
            state: Rc::clone(&reset_at_state),
            client_mode: false,
        })));
    test_ctx
        .cnx_client()
        .set_callback(Some(Box::new(ResetStreamAtCallback {
            state: Rc::clone(&reset_at_state),
            client_mode: true,
        })));

    test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_RESET_AT)?;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, 30_000)?;

    let received = reset_at_state.borrow().r_recv_nb as u64;
    let sent_limit = stream_sent_offset(test_ctx.cnx_server(), 4).ok_or(Error::InvalidState)?;
    let mut reliable_size = match spec {
        ResetStreamAtSpec::Limit => sent_limit,
        ResetStreamAtSpec::Loss => received + 5_000,
        ResetStreamAtSpec::Basic => received + 1,
    };
    if reliable_size > sent_limit {
        reliable_size = sent_limit;
    }
    if reliable_size > 0 {
        test_ctx.cnx_server().reset_stream_at(4, 0, reliable_size)?;
    }

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 1000)?;
    {
        let state = reset_at_state.borrow();
        if !state.scenario_finished() || state.r_recv_nb < reliable_size as usize {
            return Err(Error::InvalidState);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ec9a helper (unique body, not a shared helper in C).

fn ec9a_server_application_pending(test_ctx: &mut TestTlsApiCtx) -> bool {
    test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| {
            !cnx.pkt_ctx[PacketContext::Application as usize]
                .pending
                .is_empty()
        })
        .unwrap_or(false)
}

fn ec9a_reinsert_server_by_wake_time(
    test_ctx: &mut TestTlsApiCtx,
    next_time: Instant,
) -> crate::Result<()> {
    let token = test_ctx
        .qserver
        .first_cnx_mut()
        .and_then(|cnx| cnx.own_token)
        .ok_or(Error::InvalidState)?;

    test_ctx.qserver.remove_cnx_from_wake_list(token);
    {
        let cnx = test_ctx
            .qserver
            .connections
            .get_mut(token)
            .ok_or(Error::InvalidState)?;
        cnx.next_wake_time = next_time;
    }

    let (tree_token, old_token) = test_ctx
        .qserver
        .connection_wake_tree
        .insert(connection_wake_key(next_time, token), token)?;
    let cnx = test_ctx
        .qserver
        .connections
        .get_mut(token)
        .ok_or(Error::InvalidState)?;
    cnx.connection_wake_membership = Some(tree_token);
    if let Some(old_token) = old_token
        && old_token != token
        && let Some(old_connection) = test_ctx.qserver.connections.get_mut(old_token)
    {
        old_connection.connection_wake_membership = None;
    }
    Ok(())
}

/// Run the server-only prepare loop for `ec9a_preemptive_amok`:
/// advance simulated time through server wakes, counting sent packets,
/// until the server's connection count drops to zero.  Returns
/// `(send_count, repeat_duration_µs)`.
fn ec9a_server_loop(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<(i32, u64)> {
    let cnx_server_idle_timeout = test_ctx.cnx_server().idle_timeout.ticks();
    let mut cnx_server_nb_preemptive_repeat = test_ctx.cnx_server().preemptive_repeat_count();
    let repeat_begin = simulated_time.ticks();
    let mut loop_count = 0;
    let mut send_count = 0;
    let mut buffer = [0u8; MAX_PACKET_SIZE];

    ec9a_reinsert_server_by_wake_time(test_ctx, *simulated_time)?;

    while test_ctx.qserver.current_number_connections > 0
        && test_ctx
            .qserver
            .first_cnx_mut()
            .map(|cnx| cnx.state() == State::Ready)
            .unwrap_or(false)
        && loop_count < 10_000
    {
        loop_count += 1;
        cnx_server_nb_preemptive_repeat = test_ctx
            .qserver
            .first_cnx_mut()
            .map(|cnx| cnx.preemptive_repeat_count())
            .unwrap_or(cnx_server_nb_preemptive_repeat);
        let next = test_ctx.qserver.next_wake_time(*simulated_time);
        *simulated_time = Instant::from_ticks(next);
        let prepared = test_ctx
            .qserver
            .prepare_next_packet_ex(*simulated_time, &mut buffer)?;
        if prepared.send_length > 0 {
            send_count += 1;
        }
    }

    if loop_count >= 10_000 {
        return Err(Error::InvalidState);
    }
    let repeat_duration = simulated_time.ticks().saturating_sub(repeat_begin);
    if send_count > 50
        || repeat_duration > cnx_server_idle_timeout
        || cnx_server_nb_preemptive_repeat == 0
    {
        return Err(Error::InvalidState);
    }
    Ok((send_count, repeat_duration))
}

/// Check whether the server's stream `stream_id` has `reset_sent` set.
/// C: `picoquic_find_stream(cnx_server, id)->reset_sent`.
fn check_stream_reset_sent(test_ctx: &mut TestTlsApiCtx, stream_id: u64) -> bool {
    let cnx = test_ctx.cnx_server();
    let Some(token) = cnx.find_stream(stream_id) else {
        return false;
    };
    cnx.streams
        .get(token)
        .map(|stream| stream.reset_sent)
        .unwrap_or(false)
}

fn v1_header_skip(bytes: &[u8]) -> Option<(&[u8], PacketType)> {
    let (&first_byte, mut rest) = bytes.split_first()?;
    if (first_byte & 0x80) == 0 {
        return Some((&[], PacketType::OneRttProtected));
    }
    rest = rest.get(4..)?;
    let (&dcid_len, after_dcid_len) = rest.split_first()?;
    rest = after_dcid_len.get(dcid_len as usize..)?;
    let (&scid_len, after_scid_len) = rest.split_first()?;
    rest = after_scid_len.get(scid_len as usize..)?;

    match (first_byte >> 4) & 3 {
        0 => {
            let (after_token_len, token_length) = crate::utils::frames_varint_decode(rest)?;
            let after_token = after_token_len.get(token_length as usize..)?;
            let (after_length, length) = crate::utils::frames_varint_decode(after_token)?;
            Some((after_length.get(length as usize..)?, PacketType::Initial))
        }
        2 => {
            let (after_length, length) = crate::utils::frames_varint_decode(rest)?;
            Some((after_length.get(length as usize..)?, PacketType::Handshake))
        }
        _ => None,
    }
}

fn pto_server_prepare(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: Instant,
    has_initial: &mut bool,
    has_handshake: &mut bool,
) -> crate::Result<bool> {
    let mut buf = [0u8; MAX_PACKET_SIZE];
    let prepared = test_ctx
        .cnx_server()
        .prepare_packet(simulated_time, &mut buf)?;
    if prepared.send_length == 0 {
        return Ok(false);
    }

    let mut tail = &buf[..prepared.send_length];
    while !tail.is_empty() {
        let Some((next, packet_type)) = v1_header_skip(tail) else {
            break;
        };
        match packet_type {
            PacketType::Initial => *has_initial = true,
            PacketType::Handshake => *has_handshake = true,
            _ => {}
        }
        if next.len() == tail.len() {
            break;
        }
        tail = next;
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `crypto_hs_offset_test` in `picoquictest/edge_cases.c`.
#[test]
fn crypto_hs_offset() {
    for pc in [
        PacketContext::Initial,
        PacketContext::Handshake,
        PacketContext::Application,
    ] {
        crypto_hs_offset_one(pc).expect("crypto_hs_offset_one");
    }
}

/// Edge case zero: verify the common 0-RTT infrastructure works.
/// C: `ec00_zero_test` in `picoquictest/edge_cases.c`.
#[test]
fn ec00_zero() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0x00, true, &mut simulated_time, 0, 4).expect("edge_case_prepare");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 100_000).expect("edge_case_complete");
    assert!(
        test_ctx.cnx_client().nb_zero_rtt_acked > 0,
        "expected at least one 0-RTT packet acked"
    );
}

/// Second-flight NACK: client should recover even when the handshake ACK
/// and HandshakeDone are lost.
/// C: `ec2f_second_flight_nack_test` in `picoquictest/edge_cases.c`.
#[test]
fn ec2f_second_flight() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_losses = 0x1c1u64;
    let mut test_ctx = edge_case_prepare(0x2f, true, &mut simulated_time, initial_losses, 9)
        .expect("edge_case_prepare");
    let client_state = test_ctx.cnx_client().state();
    let server_state = test_ctx.cnx_server().state();

    // C checks `client < picoquic_state_ready` and `server == picoquic_state_ready`.
    assert!(
        client_state < State::Ready,
        "client should be before Ready after partial handshake, got {client_state:?}"
    );
    assert_eq!(
        server_state,
        State::Ready,
        "server should be Ready after partial handshake"
    );
    edge_case_complete(&mut test_ctx, &mut simulated_time, 360_000).expect("edge_case_complete");
}

/// Fuzz 50 random loss patterns to look for corrupted retransmissions.
/// C: `eccf_corrupted_file_fuzz_test` in `picoquictest/edge_cases.c`.
#[test]
fn eccf_corrupted_fuzz() {
    use std::io::Write as _;

    let mut report = std::fs::File::create("ECCF_Fuzz_report.csv").expect("fuzz report");
    writeln!(report, "Seed_hex, Seed, Ret, Elapsed").expect("fuzz header");
    let mut random_context = 0x1234_5678_8765_4321u64;

    for _ in 0..50 {
        let mut simulated_time = Instant::from_ticks(0);
        let initial_losses = random_context & 0x00ff_ffff_ff86;
        let _ = test_random(&mut random_context);

        let result = edge_case_prepare(0xcf, false, &mut simulated_time, initial_losses, 40)
            .and_then(|mut test_ctx| {
                edge_case_complete(&mut test_ctx, &mut simulated_time, 15_000_000)
            });
        if let Err(err) = result {
            writeln!(
                report,
                "0x{initial_losses:x}, {initial_losses}, {err:?}, {}",
                simulated_time.ticks()
            )
            .expect("fuzz row");
        }
    }
}

/// Amplification-limited handshake with a large server hello and losses.
/// C: `eca1_amplification_loss_test` in `picoquictest/edge_cases.c`.
#[test]
fn eca1_amplification_loss() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0xa1, false, &mut simulated_time, 0x0FF4, 16).expect("edge_case_prepare");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 15_000_000).expect("edge_case_complete");
}

/// Loss of the final closing packet: connection must close within 10 s.
/// C: `ecf1_final_loss_test` in `picoquictest/edge_cases.c`.
#[test]
fn ecf1_final_loss() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0xf1, false, &mut simulated_time, 0, 20).expect("edge_case_prepare");
    let mut zero_loss = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut zero_loss, 0, &mut simulated_time)
        .expect("connection loop");
    test_ctx.immediate_exit = true;
    tls_api_data_sending_loop(&mut test_ctx, &mut zero_loss, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0xb10)
        .expect("close with losses");
    assert!(
        simulated_time.ticks() <= 10_000_000,
        "connection close took too long: {} µs",
        simulated_time.ticks()
    );
}

/// Silly CID: server sends a new CID frame that is dropped and later
/// repeated in a bogus way.  Both sides must reach Ready.
/// C: `ec5c_silly_cid_test` in `picoquictest/edge_cases.c`.
#[test]
fn ec5c_silly_cid() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = edge_case_prepare(0x5c, false, &mut simulated_time, 0x01e084, 48)
        .expect("edge_case_prepare");
    assert!(test_ctx.has_cnx_server(), "server connection must exist");
    assert!(test_ctx.client_ready(), "client must be ready");
    assert!(test_ctx.server_ready(), "server must be ready");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 3_000_000).expect("edge_case_complete");
}

/// After the client closes, the server must not send more than ~50 preemptive
/// repeats and must stop within its idle timeout.
/// C: `ec9a_preemptive_amok_test` in `picoquictest/edge_cases.c`.
#[test]
fn ec9a_preemptive_amok() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0x9a, false, &mut simulated_time, 0x800, 12).expect("edge_case_prepare");
    assert!(test_ctx.has_cnx_server(), "server connection must exist");
    assert!(test_ctx.server_ready(), "server must be in ready state");
    assert!(test_ctx.test_finished, "data transfer must have completed");
    assert!(
        ec9a_server_application_pending(&mut test_ctx),
        "server application pending queue must be nonempty before repeat loop"
    );
    let (send_count, repeat_duration) =
        ec9a_server_loop(&mut test_ctx, &mut simulated_time).expect("server loop");
    assert!(
        send_count <= 50,
        "server sent too many repeat packets: {send_count}"
    );
    // repeat_duration must be <= idle_timeout (checked inside ec9a_server_loop)
    let _ = repeat_duration;
}

/// C: `idle_timeout_test` in `picoquictest/edge_cases.c`.
#[test]
fn idle_timeout() {
    idle_timeout_test_one(1, 30_000, 30_000, 30_000_000).expect("case 1");
    idle_timeout_test_one(2, 60_000, 20_000, 20_000_000).expect("case 2");
    idle_timeout_test_one(3, 20_000, 60_000, 20_000_000).expect("case 3");
    idle_timeout_test_one(4, 5_000, 300_000, 5_000_000).expect("case 4");
    idle_timeout_test_one(5, 300_000, 5_000, 5_000_000).expect("case 5");
    idle_timeout_test_one(6, 0, 5_000, 5_000_000).expect("case 6");
    idle_timeout_test_one(7, 0, 60_000, 60_000_000).expect("case 7");
    idle_timeout_test_one(8, 5_000, 0, 5_000_000).expect("case 8");
    idle_timeout_test_one(9, 60_000, 0, 60_000_000).expect("case 9");
    idle_timeout_test_one(10, 0, 0, u64::MAX).expect("case 10");
}

/// C: `idle_server_test` in `picoquictest/edge_cases.c`.
#[test]
fn idle_server() {
    idle_server_test_one(1, 30_000, 0, 30_100_000).expect("case 1");
    idle_server_test_one(2, 60_000, 0, 60_100_000).expect("case 2");
    idle_server_test_one(3, 5_000, 0, 5_100_000).expect("case 3");
    idle_server_test_one(4, 0, 0, 30_100_000).expect("case 4");
    idle_server_test_one(5, 0, 10_000, 10_100_000).expect("case 5");
    idle_server_test_one(6, 20_000, 60_000, 60_100_000).expect("case 6");
    idle_server_test_one(7, 60_000, 5_000, 5_100_000).expect("case 7");
}

/// C: `initial_pto_test` in `picoquictest/edge_cases.c`.
#[test]
fn initial_pto() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x94, 0x01, 0x41, 0, 0, 0, 0, 0]).expect("8-byte CID");
    let simulated_rtt = 20_000u64;
    let simulated_pto = 4 * simulated_rtt;

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.cnx_client().start_client().expect("start_client");

    // Send the initial packet to the server.
    let length =
        initial_pto_prepare(&mut test_ctx, &mut simulated_time).expect("initial_pto_prepare");
    assert!(length >= 1200, "initial packet too short: {length}");

    // Wait until the ACK time, then send a synthetic ACK to the client.
    if simulated_time.ticks() < simulated_rtt {
        let _length = initial_pto_wait(&mut test_ctx, &mut simulated_time, simulated_rtt)
            .expect("initial_pto_wait");
    }
    initial_pto_ack(&mut test_ctx, &mut simulated_time).expect("initial_pto_ack");

    // The client should fire a PTO and send at least 1200 bytes.
    if simulated_time.ticks() < simulated_pto {
        let length = initial_pto_wait(&mut test_ctx, &mut simulated_time, simulated_pto)
            .expect("initial_pto_wait (PTO)");
        assert!(length >= 1200, "PTO packet not sent (length = {length})");
    }
}

/// C: `initial_pto_srv_test` in `picoquictest/edge_cases.c`.
#[test]
fn initial_pto_srv() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x94, 0x01, 0x85, 0, 0, 0, 0, 0]).expect("8-byte CID");
    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.cnx_client().start_client().expect("start_client");

    let length =
        initial_pto_prepare(&mut test_ctx, &mut simulated_time).expect("initial_pto_prepare");
    assert!(length >= 1200, "initial packet too short: {length}");

    test_ctx.cnx_server().initial_validated = true;
    simulated_time = Instant::from_ticks(test_ctx.qserver.next_wake_time(simulated_time));
    let mut has_initial = false;
    let mut has_handshake = false;
    loop {
        let has_packet = pto_server_prepare(
            &mut test_ctx,
            simulated_time,
            &mut has_initial,
            &mut has_handshake,
        )
        .expect("pto_server_prepare first flight");
        if !has_packet {
            break;
        }
    }

    simulated_time = Instant::from_ticks(test_ctx.qserver.next_wake_time(simulated_time));
    has_initial = false;
    has_handshake = false;
    loop {
        let has_packet = pto_server_prepare(
            &mut test_ctx,
            simulated_time,
            &mut has_initial,
            &mut has_handshake,
        )
        .expect("pto_server_prepare PTO");
        if !has_packet {
            break;
        }
    }

    assert!(
        has_initial && has_handshake,
        "server PTO did not include both Initial and Handshake packets"
    );
}

/// C: `reset_ack_max_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_ack_max() {
    reset_repeat_test_one(ResetTestKind::AckMaxStream).expect("reset_ack_max");
}

/// C: `reset_ack_reset_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_ack_reset() {
    reset_repeat_test_one(ResetTestKind::AckReset).expect("reset_ack_reset");
}

/// C: `reset_extra_max_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_extra_max() {
    reset_repeat_test_one(ResetTestKind::ExtraMaxStream).expect("reset_extra_max");
}

/// C: `reset_extra_reset_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_extra_reset() {
    reset_repeat_test_one(ResetTestKind::ExtraReset).expect("reset_extra_reset");
}

/// C: `reset_extra_stop_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_extra_stop() {
    reset_repeat_test_one(ResetTestKind::ExtraStop).expect("reset_extra_stop");
}

/// C: `reset_need_max_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_need_max() {
    reset_repeat_test_one(ResetTestKind::NeedMaxStream).expect("reset_need_max");
}

/// C: `reset_need_reset_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_need_reset() {
    reset_repeat_test_one(ResetTestKind::NeedReset).expect("reset_need_reset");
}

/// C: `reset_need_stop_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_need_stop() {
    reset_repeat_test_one(ResetTestKind::NeedStop).expect("reset_need_stop");
}

/// Reset a stream on the server side while it is still mid-transfer, then
/// verify that the client correctly forbids adding data or marking it active
/// after the reset.
/// C: `reset_loop_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_loop_test() {
    let mut simulated_time = Instant::from_ticks(0);
    let test_stream: u64 = 8;
    let cb_state = Rc::new(RefCell::new(ResetLoopState::default()));

    let mut test_ctx = tls_api_init_ctx(
        &mut simulated_time,
        crate::internal::Version::InternalTest1 as u32,
        None,
    )
    .expect("tls_api_init_ctx");

    test_ctx
        .qserver
        .set_default_callback(Some(Box::new(ResetLoopCallback {
            state: Rc::clone(&cb_state),
        })));
    test_ctx
        .cnx_client()
        .set_callback(Some(Box::new(ResetLoopCallback {
            state: Rc::clone(&cb_state),
        })));

    test_ctx.cnx_client().start_client().expect("start_client");

    // Queue initial data on streams 4 and 8; triggers server-side stream creation.
    let bogus = [0u8; 4];
    test_ctx
        .cnx_client()
        .add_to_stream(4, &bogus, false)
        .expect("add_to_stream 4");
    test_ctx
        .cnx_client()
        .add_to_stream(8, &bogus, false)
        .expect("add_to_stream 8");

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    // Enable streaming on both client streams with equal priority.
    test_ctx
        .cnx_client()
        .mark_active_stream(4, true, Some(Box::new(Rc::clone(&cb_state))))
        .expect("mark active 4");
    test_ctx
        .cnx_client()
        .mark_active_stream(8, true, Some(Box::new(Rc::clone(&cb_state))))
        .expect("mark active 8");
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 8)
        .expect("set client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 8)
        .expect("set client priority 8");

    // Allow stream data to begin flowing before the reset.
    let timeout = simulated_time.ticks() + 100_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout).expect("100ms wait");
    {
        let state = cb_state.borrow();
        assert!(
            state.prepare_to_send[1] > 0 && state.data_sent[1] > 0,
            "client stream 4 should have used callback-driven prepare-to-send before reset"
        );
        assert!(
            state.prepare_to_send[3] > 0 && state.data_sent[3] > 0,
            "client stream 8 should have used callback-driven prepare-to-send before reset"
        );
        assert!(
            state.data_sent[3] < RESET_LOOP_TARGET_BYTES,
            "stream {test_stream} should still be mid-transfer before reset"
        );
    }

    // Server resets stream 8 while the transfer is in progress.
    test_ctx
        .cnx_server()
        .reset_stream(test_stream, 0)
        .expect("reset_stream");

    // Adjust priorities to expose the bug (different priorities post-reset).
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 9)
        .expect("client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 7)
        .expect("client priority 8");
    test_ctx
        .cnx_server()
        .set_stream_priority(4, 9)
        .expect("server priority 4");
    test_ctx
        .cnx_server()
        .set_stream_priority(8, 7)
        .expect("server priority 8");

    // Poll until the server's RESET_STREAM frame has actually been sent.
    let deadline = Instant::from_ticks(simulated_time.ticks() + 100_000);
    for _ in 0..16 {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            deadline,
            &mut was_active,
        )
        .expect("sim round");
        if check_stream_reset_sent(&mut test_ctx, test_stream) {
            break;
        }
    }
    assert!(
        check_stream_reset_sent(&mut test_ctx, test_stream),
        "server did not send RESET_STREAM for stream {test_stream}"
    );

    // After reset, adding data or marking the stream active must be rejected.
    assert!(
        test_ctx
            .cnx_server()
            .add_to_stream(test_stream, &[1, 2, 3, 4], true)
            .is_err(),
        "add_to_stream after reset should be forbidden on stream {test_stream}"
    );
    assert!(
        test_ctx
            .cnx_server()
            .mark_active_stream(test_stream, true, None)
            .is_err(),
        "mark_active_stream after reset should be forbidden on stream {test_stream}"
    );

    // Final loop: verify the connection settles within 2 seconds.
    let timeout2 = simulated_time.ticks() + 2_000_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout2).expect("2s wait");
    assert!(
        cb_state.borrow().reset_received[3] > 0,
        "client callback should observe RESET_STREAM for stream {test_stream}"
    );
}

/// C: `reset_stream_at_basic_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_stream_at_basic() {
    reset_stream_at_test_one(ResetStreamAtSpec::Basic).expect("reset_stream_at_basic");
}

/// C: `reset_stream_at_limit_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_stream_at_limit_test() {
    reset_stream_at_test_one(ResetStreamAtSpec::Limit).expect("reset_stream_at_limit");
}

/// C: `reset_stream_at_loss_test` in `picoquictest/edge_cases.c`.
#[test]
fn reset_stream_at_loss() {
    reset_stream_at_test_one(ResetStreamAtSpec::Loss).expect("reset_stream_at_loss");
}

/// Table-driven check that every named [`crate::errors::InternalError`]
/// and [`crate::errors::TransportError`] code resolves to its
/// short textual name, plus the four "default-branch" buckets
/// (`crypto error alert`, `unknown picoquic error`, `unknown`).
#[test]
fn error_name() {
    use crate::errors::InternalError;

    let cases: &[(u64, &str)] = &[
        // Protocol (transport / TLS) errors.
        (0x1, "internal"),
        (0x2, "server busy"),
        (0x3, "flow control"),
        (0x4, "stream limit"),
        (0x5, "stream state"),
        (0x6, "final offset"),
        (0x7, "frame format"),
        (0x8, "parameter"),
        (0x9, "connection_id limit"),
        (0xA, "protocol violation"),
        (0xB, "invalid token"),
        (0xC, "application"),
        (0xD, "crypto buffer exceeded"),
        (0xE, "key update"),
        (0xF, "aead limit"),
        (0x178, "wrong alpn"),
        (0x201, "tls handshake failed"),
        (0x11, "version negotiation"),
        (0x3e, "application abandon"),
        (0x3e75, "resource limit reached"),
        (0x3e76, "unstable interface"),
        (0x3e77, "no CID available"),
        // Picoquic-internal codes (0x400+ range).
        (0x401, "duplicate"),
        (0x403, "payload_decrypt_error"),
        (0x404, "unexpected packet"),
        (0x405, "memory"),
        (0x407, "connection ID check"),
        (0x408, ""),
        (0x409, "version negotation spoofed"),
        (0x40A, "malformed transport extension"),
        (0x40B, "extension buffer too small"),
        (0x40C, "illegal transport extension"),
        (0x40D, "cannot reset the crypto stream"),
        (0x40E, "invalid stream id"),
        (0x40F, "stream already closed"),
        (0x410, "frame buffer too small"),
        (0x411, "invalid frame"),
        (0x412, "cannot control the crypto stream"),
        (0x413, "retry"),
        (0x414, "disconnected"),
        (0x415, "error detected"),
        (0x417, "invalid ticket"),
        (0x418, "invalid file"),
        (0x419, "send buffer too small"),
        (0x41A, "unexpected state"),
        (0x41B, "unexpected error"),
        (0x41C, "server configuration without cert"),
        (0x41D, "no such file"),
        (0x41E, "stateless reset"),
        (0x41F, "connection deleted"),
        (0x420, "connection ID segment error"),
        (0x421, "connection ID not available"),
        (0x422, "migration disabled"),
        (0x423, "cannot compute key"),
        (0x424, "cannot set active stream"),
        (0x425, "cannot change active context"),
        (0x426, "invalid token"),
        (0x427, "initial CID too short"),
        (0x428, "key rotation not ready"),
        (0x429, "aead not ready"),
        (0x42A, "no ALPN provided"),
        (0x42B, "no callback provided"),
        (0x42C, "stream receive complete"),
        (0x42D, "packet header parsing"),
        (0x42E, "QUIC bit missing"),
        (0x42F, "terminate packet loop (not an error)"),
        (0x430, "simulate NAT (not an error)"),
        (0x431, "simulate migration (not an error)"),
        (0x432, "version not supported"),
        (0x433, "idle timeout"),
        (0x434, "repeat timeout"),
        (0x435, "handshake timeout"),
        (0x436, "socket"),
        (0x437, "version negotiation"),
        (0x438, "packet too long"),
        (0x439, "wrong version"),
        (0x43A, "port blocked"),
        (0x43B, "datagram too long"),
        (0x43C, "invalid path ID"),
        (0x43D, "retry needed"),
        (0x43E, "server busy"),
        (0x43F, "duplicate path"),
        (0x440, "blocked by lack of path ID"),
        (0x441, "blocked by lack of CID"),
        (0x442, "path address family"),
        (0x443, "path not ready"),
        (0x444, "path limit exceeded"),
        (0x445, "redirected to proxy (not an error)"),
        (0x446, "padding_packet"),
        // CRYPTO_ERROR alert range (default branch).
        (0x101, "crypto error alert"),
        (0x150, "crypto error alert"),
        (0x1FF, "crypto error alert"),
        // Unknown picoquic error range (default branch).
        (0x450, "unknown picoquic error"),
        (0x4FF, "unknown picoquic error"),
        // Truly unknown.
        (0x0, "unknown"),
        (0x200, "unknown"),
        (0x500, "unknown"),
        (0xFFFF_FFFF_FFFF_FFFF, "unknown"),
    ];

    for &(code, expected) in cases {
        let got = InternalError::name(code).unwrap_or("<none>");
        assert_eq!(
            got, expected,
            "error_code=0x{code:x} got={got:?} expected={expected:?}",
        );
    }
}
