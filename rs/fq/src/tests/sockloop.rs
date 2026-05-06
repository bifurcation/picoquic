//! Test cases for `picoquictest/sockloop_test.c`.
//!
//! Integration tests for the packet-loop entry points
//! ([`Quic::run`], [`Quic::run_v2`], [`NetworkThreadCtx::spawn`]).
//! They bind real OS sockets, loop a QUIC client+server through the
//! loopback interface, and verify that all scenario streams complete.

#![allow(non_snake_case, dead_code)]

use core::net::SocketAddr;

use super::util::TestApiStreamDesc;
use crate::packet_loop::{LoopEvent, LoopParam, PacketLoopCbFn};
use crate::{ConnectionId, Error, Quic};

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
    notified_ready: bool,
    established: bool,
    force_migration: i32,
    migration_started: bool,
    address_updated: bool,
    param: LoopParam,
    alt_port: u16,
    server_address: Option<SocketAddr>,
    client_address: Option<SocketAddr>,
}

impl SockloopTestCb {
    fn new(force_migration: i32, param: LoopParam) -> Self {
        Self {
            notified_ready: false,
            established: false,
            force_migration,
            migration_started: false,
            address_updated: false,
            param,
            alt_port: 0,
            server_address: None,
            client_address: None,
        }
    }
}

impl PacketLoopCbFn for SockloopTestCb {
    fn callback(&mut self, quic: &mut Quic, event: LoopEvent<'_>) -> Result<(), Error> {
        todo!(
            "sockloop callback: quic={:p}, event={:?}, notified={}",
            quic as *mut Quic,
            event,
            self.notified_ready
        )
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

// ---------------------------------------------------------------------------
// Core test driver.
// C: `sockloop_test_one`.

fn sockloop_test_one(_spec: &SockloopTestSpec) {
    todo!("sockloop_test_one")
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
