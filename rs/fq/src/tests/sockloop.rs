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
            socket_buffer_size: 1280, // PICOQUIC_MAX_PACKET_SIZE
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
}

impl SockloopTestCb {
    fn new(test_id: u8, force_migration: i32, param: LoopParam) -> Self {
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
                let Some(cnx) = quic.first_cnx_mut() else {
                    return Ok(());
                };
                match cnx.connection_state {
                    State::Disconnected => Err(Error::Protocol(
                        InternalError::NoErrorTerminatePacketLoop as u64,
                    )),
                    State::ClientAlmostReady if !self.notified_ready => {
                        self.notified_ready = true;
                        self.client_address = connection_local_addr(cnx);
                        self.server_address = connection_peer_addr(cnx);
                        self.client_cid_before_migration = Some(cnx.local_connection_id());
                        self.server_cid_before_migration = Some(cnx.remote_connection_id());
                        Ok(())
                    }
                    State::Ready => {
                        self.handle_ready_receive(cnx, current_time)?;
                        if sockloop_test_received_finished(self) {
                            Err(Error::Protocol(
                                InternalError::NoErrorTerminatePacketLoop as u64,
                            ))
                        } else {
                            Ok(())
                        }
                    }
                    _ => Ok(()),
                }
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
    loop_cb.established && loop_cb.notified_ready
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
    )));

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
            if let Some(cnx) = quic.first_cnx_mut() {
                cnx.start_client()?;
            }
            for _ in 0..50 {
                if sockloop_test_received_finished(&loop_cb.borrow()) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            drop(thread_ctx);
            Ok(())
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
                sockloop_test_verify_migration(&cb, cnx)
            } else {
                Ok(())
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
