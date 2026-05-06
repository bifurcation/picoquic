//! Test cases for `picoquictest/socket_test.c`.
//!
//! Exercises OS UDP socket open/send/receive and ECN option
//! configuration using the [`crate::socks`] / [`crate::socks_socket2`]
//! abstractions.

#![allow(non_snake_case)]

use core::net::SocketAddr;

use crate::socks::ServerSockets;
use crate::socks_socket2::Socket2Udp;

// AF_INET / AF_INET6 raw values (POSIX; matches Linux and macOS).
const AF_INET: i32 = 2;
const AF_INET6: i32 = 10;

// ---------------------------------------------------------------------------
// Helper: resolve a text address into a SocketAddr.
// C: `picoquic_get_server_address`.

fn get_server_address(addr_text: &str, port: u16) -> crate::Result<(SocketAddr, bool)> {
    let sa = crate::socks::ServerAddress::resolve(addr_text, port as i32)?;
    Ok((sa.addr, sa.is_name))
}

// ---------------------------------------------------------------------------
// Ping-pong over a single port.
// C: `socket_ping_pong` + `socket_test_one` + `socket_test_port`.

fn socket_ping_pong(
    _client: &mut Socket2Udp,
    _server_addr: SocketAddr,
    _server_sockets: &mut ServerSockets<Socket2Udp>,
) -> crate::Result<()> {
    todo!("socket_ping_pong")
}

fn socket_test_one(
    addr_text: &str,
    port: u16,
    should_be_name: bool,
    server_sockets: &mut ServerSockets<Socket2Udp>,
) -> crate::Result<()> {
    let (server_addr, is_name) = get_server_address(addr_text, port)?;
    assert_eq!(is_name, should_be_name, "is_name mismatch for {addr_text}");

    let af = match server_addr {
        SocketAddr::V4(_) => AF_INET,
        SocketAddr::V6(_) => AF_INET6,
    };
    let mut client = Socket2Udp::open_client(af)?;
    socket_ping_pong(&mut client, server_addr, server_sockets)
}

fn socket_test_port(
    server_sockets: &mut ServerSockets<Socket2Udp>,
    test_port: u16,
) -> crate::Result<()> {
    socket_test_one("127.0.0.1", test_port, false, server_sockets)?;
    socket_test_one("::1", test_port, false, server_sockets)?;
    socket_test_one("localhost", test_port, true, server_sockets)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ECN test helper.
// C: `socket_ecn_test_one`.

fn socket_ecn_test_one(_af: i32) -> crate::Result<()> {
    todo!("socket_ecn_test_one")
}

// ---------------------------------------------------------------------------
// Exported tests.

/// C: `socket_test` in `picoquictest/socket_test.c`.
#[test]
fn sockets() {
    let test_port: u16 = 12345;
    let test_port2: u16 = 1234;

    let mut server_sockets =
        ServerSockets::<Socket2Udp>::open(test_port as i32).expect("open server sockets");

    socket_test_port(&mut server_sockets, test_port).expect("ping-pong on port 12345");

    let mut server_sockets2 =
        ServerSockets::<Socket2Udp>::open(test_port2 as i32).expect("open server sockets 2");

    socket_test_port(&mut server_sockets2, test_port2).expect("ping-pong on port 1234");

    server_sockets2.close();
    server_sockets.close();
}

/// C: `socket_ecn_test` in `picoquictest/socket_test.c`.
#[test]
fn socket_ecn() {
    socket_ecn_test_one(AF_INET).expect("ECN on IPv4");
    socket_ecn_test_one(AF_INET6).expect("ECN on IPv6");
}
