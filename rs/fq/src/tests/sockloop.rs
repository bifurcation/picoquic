//! Test cases for `picoquictest/sockloop_test.c`.
//!
//! Integration tests for the packet-loop entry points
//! ([`Quic::run`], [`Quic::run_v2`], [`NetworkThreadCtx::spawn`]).
//! They bind real OS sockets, loop a QUIC client+server through the
//! loopback interface, and verify that all scenario streams complete.

#![allow(non_snake_case, dead_code)]

use core::net::SocketAddr;
use std::cell::RefCell;
use std::rc::Rc;

use super::util::{
    TEST_ALPN, TEST_FILE_CERT_STORE, TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_KEY, TEST_SNI,
    TestApiStreamDesc,
};
use crate::errors::InternalError;
use crate::internal::Connection;
use crate::packet_loop::{
    LoopEvent, LoopParam, NetworkThreadCtx, PACKET_LOOP_SOCKETS_MAX, PacketLoopCbFn, SocketCtx,
    open_sockets,
};
use crate::socks_socket2::Socket2Udp;
use crate::stream::StreamId;
use crate::{ConnectionId, Error, Instant, Quic, RESET_SECRET_SIZE, State};

// ---------------------------------------------------------------------------
// Scenario used by the basic and EIO variants: two small streams.
// C: `sockloop_test_scenario_basic[]`.

const SOCKLOOP_SCENARIO_BASIC: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2_000,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 0,
        q_len: 531,
        r_len: 11_000,
    },
];

// Two sequential 1 MB streams.  C: `sockloop_test_scenario_1M[]`.
const SOCKLOOP_SCENARIO_1M: &[TestApiStreamDesc] = &[
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
];

// ---------------------------------------------------------------------------
// Test specification.
// C: `sockloop_test_spec_t`.

#[derive(Clone)]
struct SockloopTestSpec {
    test_id: u8,
    af: i32,
    port: u16,
    socket_buffer_size: i32,
    scenario: &'static [TestApiStreamDesc],
    thread_name: Option<&'static str>,
    use_background_thread: bool,
    ipv6_only: bool,
    do_not_use_gso: bool,
    simulate_eio: bool,
    double_bind: bool,
    extra_socket_required: bool,
    prefer_extra_socket: bool,
    force_migration: i32,
}

impl SockloopTestSpec {
    /// Build defaults matching `sockloop_test_set_spec`.
    fn new(test_id: u8) -> Self {
        Self {
            test_id,
            af: AF_INET6,
            port: 3456,
            socket_buffer_size: crate::MAX_PACKET_SIZE as i32, // PICOQUIC_MAX_PACKET_SIZE
            scenario: SOCKLOOP_SCENARIO_BASIC,
            thread_name: None,
            use_background_thread: false,
            ipv6_only: false,
            do_not_use_gso: false,
            simulate_eio: false,
            double_bind: false,
            extra_socket_required: false,
            prefer_extra_socket: false,
            force_migration: 0,
        }
    }
}

// AF_INET / AF_INET6 raw values (POSIX).
const AF_INET: i32 = 2;
const AF_INET6: i32 = 10;

// ---------------------------------------------------------------------------
// Packet-loop callback.
// C: `sockloop_test_cb_t` + `sockloop_test_cb`.

struct SockloopTestCb {
    test_id: u8,
    notified_ready: bool,
    established: bool,
    force_migration: i32,
    migration_started: bool,
    address_updated: bool,
    param: LoopParam,
    alt_port: u16,
    server_address: Option<SocketAddr>,
    client_address: Option<SocketAddr>,
    client_alt_address: Option<SocketAddr>,
    client_cid_before_migration: Option<ConnectionId>,
    server_cid_before_migration: Option<ConnectionId>,
    scenario: SockloopScenarioState,
}

impl SockloopTestCb {
    fn new(
        test_id: u8,
        force_migration: i32,
        param: LoopParam,
        scenario: &[TestApiStreamDesc],
    ) -> Self {
        Self {
            test_id,
            notified_ready: false,
            established: false,
            force_migration,
            migration_started: false,
            address_updated: false,
            param,
            alt_port: 0,
            server_address: None,
            client_address: None,
            client_alt_address: None,
            client_cid_before_migration: None,
            server_cid_before_migration: None,
            scenario: SockloopScenarioState::new(scenario),
        }
    }

    fn callback_impl(&mut self, quic: &mut Quic, event: LoopEvent<'_>) -> Result<(), Error> {
        match event {
            LoopEvent::Ready(options) => {
                if self.test_id > 1 {
                    options.do_time_check = true;
                    if self.param.extra_socket_required {
                        options.provide_alt_port = true;
                    }
                }
                Ok(())
            }
            LoopEvent::AfterReceive(_) => {
                let current_time = Instant::from_ticks(quic.time());
                let mut process_scenario = false;
                {
                    let Some(cnx) = quic.first_cnx_mut() else {
                        return Ok(());
                    };
                    match cnx.connection_state {
                        State::Disconnected => {
                            return Err(Error::Protocol(
                                InternalError::NoErrorTerminatePacketLoop as u64,
                            ));
                        }
                        State::ClientAlmostReady if !self.notified_ready => {
                            self.notified_ready = true;
                            self.client_address = connection_local_addr(cnx);
                            self.server_address = connection_peer_addr(cnx);
                            self.client_cid_before_migration = Some(cnx.local_connection_id());
                            self.server_cid_before_migration = Some(cnx.remote_connection_id());
                        }
                        State::Ready => {
                            self.handle_ready_receive(cnx, current_time)?;
                            process_scenario = true;
                        }
                        _ => {}
                    }
                }
                if process_scenario {
                    self.scenario.process_received_streams(quic);
                    if sockloop_test_received_finished(self) {
                        return Err(Error::Protocol(
                            InternalError::NoErrorTerminatePacketLoop as u64,
                        ));
                    }
                }
                Ok(())
            }
            LoopEvent::AfterSend(_) => {
                let Some(cnx) = quic.first_cnx_mut() else {
                    return Ok(());
                };
                if cnx.connection_state == State::Disconnected {
                    Err(Error::Protocol(
                        InternalError::NoErrorTerminatePacketLoop as u64,
                    ))
                } else {
                    if !self.established
                        && matches!(cnx.connection_state, State::Ready | State::ClientReadyStart)
                    {
                        self.established = true;
                    }
                    Ok(())
                }
            }
            LoopEvent::PortUpdate(_) => Ok(()),
            LoopEvent::TimeCheck(time_check_arg) => {
                if time_check_arg.delta_t > 5000 {
                    time_check_arg.delta_t = 5000;
                }
                Ok(())
            }
            LoopEvent::SystemCallDuration(_) => Ok(()),
            LoopEvent::WakeUp => {
                if let Some(cnx) = quic.first_cnx_mut() {
                    cnx.start_client()?;
                }
                Ok(())
            }
            LoopEvent::AltPort(addr) => {
                self.alt_port = addr.port();
                if self.force_migration == 1
                    && let Some(cnx) = quic.first_cnx_mut()
                    && connection_local_addr(cnx)
                        .map(|a| a.ip().is_unspecified())
                        .unwrap_or(false)
                {
                    let mut alt_addr = connection_peer_addr(cnx).unwrap_or(addr);
                    set_addr_port(&mut alt_addr, self.alt_port);
                    cnx.set_local_addr(&alt_addr)?;
                }
                Ok(())
            }
        }
    }

    fn handle_ready_receive(
        &mut self,
        cnx: &mut Connection,
        current_time: Instant,
    ) -> Result<(), Error> {
        if self.force_migration == 0 {
            return Ok(());
        }

        if !self.migration_started {
            let has_remote_cid = cnx
                .remote_connection_id_stashes
                .first()
                .map(|stash| !stash.connection_ids.is_empty())
                .unwrap_or(false);

            if has_remote_cid
                && self.alt_port != 0
                && let (Some(client_address), Some(server_address)) =
                    (self.client_address, self.server_address)
            {
                let mut client_alt_address = client_address;
                set_addr_port(&mut client_alt_address, self.alt_port);
                self.client_alt_address = Some(client_alt_address);

                if self.force_migration == 3 {
                    self.migration_started = true;
                    cnx.probe_new_path(&server_address, &client_alt_address, current_time)?;
                } else if self.force_migration == 1 {
                    self.migration_started = true;
                    let mut local_addr = connection_local_addr(cnx).unwrap_or(client_alt_address);
                    set_addr_port(&mut local_addr, self.param.local_port);
                    cnx.set_local_addr(&local_addr)?;
                    return Err(Error::Protocol(InternalError::NoErrorSimulateNat as u64));
                }
            }
        } else if !self.address_updated
            && let Some(server_address) = self.server_address
            && connection_local_addr(cnx) == Some(server_address)
        {
            self.address_updated = true;
        }

        Ok(())
    }
}

impl PacketLoopCbFn for SockloopTestCb {
    fn callback(&mut self, quic: &mut Quic, event: LoopEvent<'_>) -> Result<(), Error> {
        self.callback_impl(quic, event)
    }
}

#[derive(Clone)]
struct SharedSockloopTestCb(Rc<RefCell<SockloopTestCb>>);

impl PacketLoopCbFn for SharedSockloopTestCb {
    fn callback(&mut self, quic: &mut Quic, event: LoopEvent<'_>) -> Result<(), Error> {
        self.0.borrow_mut().callback_impl(quic, event)
    }
}

// ---------------------------------------------------------------------------
// Socket-loop scenario state.
// C: the `picoquic_test_tls_api_ctx_t` stream fields used by
// `test_api_init_send_recv_scenario`, `sockloop_test_received_finished`,
// and `tls_api_one_scenario_verify`.

struct SockloopStream {
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

impl SockloopStream {
    fn new(desc: &TestApiStreamDesc) -> Self {
        fn source_bytes(len: usize) -> Vec<u8> {
            (0..len).map(|i| i as u8).collect()
        }

        Self {
            stream_id: desc.stream_id,
            previous_stream_id: desc.previous_stream_id,
            q_sent: false,
            q_received: false,
            r_received: false,
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
        self.r_received && self.r_recv_nb == self.r_len
    }
}

struct SockloopStreamEvent {
    client_mode: bool,
    stream_id: u64,
    bytes: Vec<u8>,
    fin: bool,
}

struct SockloopScenarioState {
    streams: Vec<SockloopStream>,
    client_callback_error_detected: bool,
    server_callback_error_detected: bool,
    stream0_target: usize,
    stream0_sent: usize,
    stream0_received: usize,
    streams_finished: bool,
    test_finished: bool,
}

impl SockloopScenarioState {
    fn new(scenario: &[TestApiStreamDesc]) -> Self {
        Self {
            streams: scenario.iter().map(SockloopStream::new).collect(),
            client_callback_error_detected: false,
            server_callback_error_detected: false,
            stream0_target: 0,
            stream0_sent: 0,
            stream0_received: 0,
            streams_finished: false,
            test_finished: false,
        }
    }

    fn set_callback_error(&mut self, client_mode: bool) {
        if client_mode {
            self.client_callback_error_detected = true;
        } else {
            self.server_callback_error_detected = true;
        }
    }

    fn queue_initial_queries(&mut self, quic: &mut Quic, initial_data_stream_id: u64) {
        let mut more_stream = false;

        for i in 0..self.streams.len() {
            if self.streams[i].previous_stream_id != initial_data_stream_id {
                continue;
            }

            let stream_id = self.streams[i].stream_id;
            let data = self.streams[i].q_src[..self.streams[i].q_len].to_vec();
            let client_mode = StreamId(stream_id).is_client();
            if queue_on_connection(quic, client_mode, stream_id, &data, true).is_err() {
                self.set_callback_error(client_mode);
            } else {
                self.streams[i].q_sent = true;
            }
            more_stream = true;
        }

        if !more_stream {
            more_stream = self.streams.iter().any(|s| !s.response_complete());
        }

        if more_stream {
            self.test_finished = false;
            self.streams_finished = false;
        } else {
            self.streams_finished = true;
            self.test_finished = self.stream0_received >= self.stream0_target;
        }
    }

    fn process_received_streams(&mut self, quic: &mut Quic) {
        let mut events = Vec::new();

        for cnx in quic.connections.iter_mut() {
            events.extend(collect_received_stream_events(cnx, cnx.client_mode));
        }

        for event in events {
            self.handle_stream_event(quic, event);
        }
    }

    fn handle_stream_event(&mut self, quic: &mut Quic, event: SockloopStreamEvent) {
        if event.stream_id == 0 && !event.client_mode {
            if event.bytes.iter().any(|b| *b != 0xa5) {
                self.set_callback_error(event.client_mode);
                return;
            }
            self.stream0_received = self.stream0_received.saturating_add(event.bytes.len());
            if self.streams_finished && self.stream0_received >= self.stream0_target {
                self.test_finished = true;
            }
            return;
        }

        let Some(stream_index) = self
            .streams
            .iter()
            .position(|stream| stream.stream_id == event.stream_id)
        else {
            self.set_callback_error(event.client_mode);
            return;
        };

        let is_client_stream = StreamId(event.stream_id).is_client();
        let mut stream_finished = false;
        let mut response_target = None;
        let mut response = Vec::new();
        let callback_ok;

        {
            let stream = &mut self.streams[stream_index];

            if is_client_stream {
                if event.client_mode {
                    callback_ok = receive_stream_data(stream, true, &event.bytes, event.fin);
                    stream_finished = event.fin;
                } else {
                    callback_ok = receive_stream_data(stream, false, &event.bytes, event.fin);
                    if event.fin && callback_ok {
                        if stream.r_len == 0 {
                            stream.r_received = true;
                            stream_finished = true;
                        } else {
                            response_target = Some(false);
                            response = stream.r_src.clone();
                        }
                    }
                }
            } else if event.client_mode {
                callback_ok = receive_stream_data(stream, false, &event.bytes, event.fin);
                if event.fin && callback_ok {
                    if stream.r_len == 0 {
                        stream.r_received = true;
                        stream_finished = true;
                    } else {
                        response_target = Some(true);
                        response = stream.r_src.clone();
                    }
                }
            } else {
                callback_ok = receive_stream_data(stream, true, &event.bytes, event.fin);
                stream_finished = event.fin;
            }
        }

        if !callback_ok {
            self.set_callback_error(event.client_mode);
            return;
        }

        if let Some(client_mode) = response_target
            && queue_on_connection(quic, client_mode, event.stream_id, &response, true).is_err()
        {
            self.set_callback_error(event.client_mode);
            return;
        }

        if stream_finished {
            self.queue_initial_queries(quic, event.stream_id);
        }
    }

    fn received_finished_or_error(&self) -> bool {
        if self.server_callback_error_detected || self.client_callback_error_detected {
            return true;
        }

        if self.streams.is_empty() {
            return false;
        }

        if self
            .streams
            .iter()
            .any(|stream| stream.q_recv_nb != stream.q_len || stream.r_recv_nb != stream.r_len)
        {
            return false;
        }

        self.stream0_sent == self.stream0_target && self.stream0_sent == self.stream0_received
    }

    fn verify(&self, quic: &Quic) -> Result<(), Error> {
        if self.server_callback_error_detected || self.client_callback_error_detected {
            return Err(Error::Generic);
        }

        for stream in &self.streams {
            if stream.q_recv_nb != stream.q_len
                || stream.r_recv_nb != stream.r_len
                || !stream.q_received
                || !stream.r_received
                || stream.q_rcv != stream.q_src
                || stream.r_rcv != stream.r_src
            {
                return Err(Error::Generic);
            }
        }

        if self.stream0_sent != self.stream0_target || self.stream0_sent != self.stream0_received {
            return Err(Error::Generic);
        }

        if quic.nb_data_nodes_allocated > quic.nb_data_nodes_in_pool() {
            return Err(Error::Generic);
        }

        Ok(())
    }
}

fn receive_stream_data(
    stream: &mut SockloopStream,
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
) -> Vec<SockloopStreamEvent> {
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
                events.push(SockloopStreamEvent {
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
            events.push(SockloopStreamEvent {
                client_mode,
                stream_id,
                bytes: Vec::new(),
                fin: true,
            });
        }
    }

    events
}

fn queue_on_connection(
    quic: &mut Quic,
    client_mode: bool,
    stream_id: u64,
    data: &[u8],
    fin: bool,
) -> Result<(), Error> {
    let Some(cnx) = quic
        .connections
        .iter_mut()
        .find(|cnx| cnx.client_mode == client_mode)
    else {
        return Err(Error::Generic);
    };

    cnx.add_to_stream(stream_id, data, fin)
}

// ---------------------------------------------------------------------------
// Initial CID builder.
// C: `sockloop_test_set_icid`.

fn sockloop_icid(test_id: u8) -> ConnectionId {
    let mut bytes = [0u8; 8];
    bytes[0] = 0x50;
    bytes[1] = 0xcc;
    bytes[2] = 0x10;
    bytes[3] = 0x09;
    bytes[4] = test_id;
    ConnectionId::clone_from_slice(&bytes).expect("ICID")
}

fn sockloop_addr_config(af: i32, port: u16) -> Result<SocketAddr, Error> {
    match af {
        AF_INET6 => Ok(SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port))),
        AF_INET => Ok(SocketAddr::from(([127, 0, 0, 1], port))),
        _ => Err(Error::InvalidArgument),
    }
}

fn sockloop_quic_config(current_time: Instant) -> Result<Box<Quic>, Error> {
    const TEST_TICKET_ENCRYPT_KEY: [u8; 16] =
        [16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

    let mut quic = Quic::new(
        8,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        Some(&TEST_TICKET_ENCRYPT_KEY),
    )
    .ok_or(Error::Generic)?;
    quic.set_random_initial(0);
    quic.set_optimistic_ack_policy(0);
    Ok(quic)
}

fn sockloop_cnx_config(
    quic: &mut Quic,
    addr: &SocketAddr,
    icid: ConnectionId,
    current_time: Instant,
) -> Result<(), Error> {
    quic.create_connection(
        icid,
        ConnectionId::with_size(0).ok_or(Error::Generic)?,
        Some(addr),
        current_time,
        0,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        true,
    )
    .ok_or(Error::Generic)?;
    Ok(())
}

fn connection_local_addr(cnx: &Connection) -> Option<SocketAddr> {
    cnx.paths
        .first()
        .and_then(|path| path.tuples.first())
        .map(|tuple| tuple.local_addr)
}

fn connection_peer_addr(cnx: &Connection) -> Option<SocketAddr> {
    cnx.paths
        .first()
        .and_then(|path| path.tuples.first())
        .map(|tuple| tuple.peer_addr)
}

fn set_addr_port(addr: &mut SocketAddr, port: u16) {
    match addr {
        SocketAddr::V4(v4) => v4.set_port(port),
        SocketAddr::V6(v6) => v6.set_port(port),
    }
}

fn sockloop_test_received_finished(loop_cb: &SockloopTestCb) -> bool {
    loop_cb.established && loop_cb.notified_ready && loop_cb.scenario.received_finished_or_error()
}

fn sockloop_test_verify_migration(
    loop_cb: &SockloopTestCb,
    cnx_client: &Connection,
) -> Result<(), Error> {
    if !loop_cb.migration_started {
        return Err(Error::Generic);
    }

    if loop_cb.force_migration == 1 || loop_cb.force_migration == 3 {
        if connection_local_addr(cnx_client) == loop_cb.client_address {
            return Err(Error::Generic);
        }

        if loop_cb.force_migration == 3 {
            if Some(cnx_client.remote_connection_id()) == loop_cb.server_cid_before_migration {
                return Err(Error::Generic);
            }
            if Some(cnx_client.local_connection_id()) == loop_cb.client_cid_before_migration {
                return Err(Error::Generic);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Core test driver.
// C: `sockloop_test_one`.

fn sockloop_test_one(spec: &SockloopTestSpec) {
    sockloop_test_one_result(spec).expect("sockloop_test_one")
}

fn sockloop_test_one_result(spec: &SockloopTestSpec) -> Result<(), Error> {
    let current_time = Instant::from_ticks(crate::current_time());
    let mut quic = sockloop_quic_config(current_time)?;
    let _ = quic.set_qlog(".");
    let server_address = sockloop_addr_config(spec.af, spec.port)?;
    let icid = sockloop_icid(spec.test_id);
    sockloop_cnx_config(&mut quic, &server_address, icid, current_time)?;

    let mut double_bind: [SocketCtx<Socket2Udp>; PACKET_LOOP_SOCKETS_MAX] =
        std::array::from_fn(|_| SocketCtx::default());
    if spec.double_bind {
        let opened = open_sockets(
            spec.port,
            AF_INET6,
            0,
            false,
            crate::MAX_PACKET_SIZE as i32,
            false,
            true,
            &mut double_bind,
            quic.default_congestion_alg
                .map(|algorithm| algorithm.ecn_mark)
                .unwrap_or(0),
        )?;
        if opened == 0 {
            return Err(Error::Protocol(InternalError::UnexpectedError as u64));
        }
    }

    let mut param = LoopParam {
        local_port: spec.port,
        local_af: if spec.ipv6_only { AF_INET6 } else { 0 },
        socket_buffer_size: spec.socket_buffer_size,
        do_not_use_gso: spec.do_not_use_gso,
        simulate_eio: spec.simulate_eio,
        extra_socket_required: spec.extra_socket_required,
        prefer_extra_socket: spec.prefer_extra_socket,
        ..LoopParam::default()
    };
    let loop_cb = Rc::new(RefCell::new(SockloopTestCb::new(
        spec.test_id,
        spec.force_migration,
        param,
        spec.scenario,
    )));

    loop_cb
        .borrow_mut()
        .scenario
        .queue_initial_queries(&mut quic, 0);

    if !spec.use_background_thread
        && let Some(cnx) = quic.first_cnx_mut()
    {
        cnx.start_client()?;
    }

    let run_result = if spec.test_id == 1 {
        quic.run(
            spec.port as i32,
            if spec.ipv6_only { AF_INET6 } else { 0 },
            0,
            spec.socket_buffer_size,
            spec.do_not_use_gso,
            Some(Box::new(SharedSockloopTestCb(loop_cb.clone()))),
        )
    } else if spec.use_background_thread {
        let mut thread_ctx = NetworkThreadCtx::spawn_custom(
            &mut quic,
            param,
            None,
            None,
            None,
            spec.thread_name,
            Some(Box::new(SharedSockloopTestCb(loop_cb.clone()))),
        )
        .map_err(|_| Error::Generic)?;

        for _ in 0..2000 {
            if thread_ctx.thread_is_ready {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        if !thread_ctx.thread_is_ready {
            Err(Error::Generic)
        } else {
            thread_ctx.wake_up().map_err(|_| Error::Generic)?;
            let mut transfer_finished = false;
            for _ in 0..50 {
                if sockloop_test_received_finished(&loop_cb.borrow()) {
                    transfer_finished = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            drop(thread_ctx);
            if transfer_finished {
                Ok(())
            } else {
                Err(Error::Generic)
            }
        }
    } else {
        quic.run_v2(
            &mut param,
            Some(Box::new(SharedSockloopTestCb(loop_cb.clone()))),
        )
    };

    for socket in &mut double_bind {
        socket.close();
    }

    match run_result {
        Ok(()) => {
            if spec.double_bind || (spec.force_migration != 0 && !loop_cb.borrow().address_updated)
            {
                Err(Error::Generic)
            } else if spec.force_migration != 0 {
                let cb = loop_cb.borrow();
                let cnx = quic.first_cnx_mut().ok_or(Error::Generic)?;
                sockloop_test_verify_migration(&cb, cnx)?;
                cb.scenario.verify(&quic)
            } else {
                loop_cb.borrow().scenario.verify(&quic)
            }
        }
        Err(error) => {
            if spec.double_bind {
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Exported tests.

/// C: `sockloop_basic_test` in `picoquictest/sockloop_test.c`.
#[test]
fn sockloop_basic() {
    let mut spec = SockloopTestSpec::new(1);
    spec.ipv6_only = true;
    spec.do_not_use_gso = true;
    sockloop_test_one(&spec);
}

/// C: `sockloop_eio_test` in `picoquictest/sockloop_test.c`.
#[test]
fn sockloop_eio() {
    let mut spec = SockloopTestSpec::new(2);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.simulate_eio = true;
    sockloop_test_one(&spec);
}

/// C: `sockloop_errsock_test` in `picoquictest/sockloop_test.c`.
#[test]
fn sockloop_errsock() {
    let mut spec = SockloopTestSpec::new(3);
    spec.double_bind = true;
    sockloop_test_one(&spec);
}

/// C: `sockloop_ipv4_test` in `picoquictest/sockloop_test.c`.
#[test]
fn sockloop_ipv4() {
    let mut spec = SockloopTestSpec::new(4);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    sockloop_test_one(&spec);
}

/// C: `sockloop_migration_test` in `picoquictest/sockloop_test.c`.
#[test]
fn sockloop_migration() {
    let mut spec = SockloopTestSpec::new(5);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.extra_socket_required = true;
    spec.force_migration = 3;
    sockloop_test_one(&spec);
}

/// C: `sockloop_nat_test` in `picoquictest/sockloop_test.c`.
#[test]
fn sockloop_nat() {
    let mut spec = SockloopTestSpec::new(6);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.extra_socket_required = true;
    spec.prefer_extra_socket = true;
    spec.force_migration = 1;
    sockloop_test_one(&spec);
}

/// C: `sockloop_thread_test` in `picoquictest/sockloop_test.c`.
#[test]
fn sockloop_thread() {
    let mut spec = SockloopTestSpec::new(7);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.use_background_thread = true;
    sockloop_test_one(&spec);
}

/// C: `sockloop_thread_name_test` in `picoquictest/sockloop_test.c`.
#[test]
fn sockloop_thread_name() {
    let mut spec = SockloopTestSpec::new(8);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.use_background_thread = true;
    spec.thread_name = Some("picoquic loop");
    sockloop_test_one(&spec);
}
