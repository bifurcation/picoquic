//! Test cases for `picoquictest/socket_test.c`.
//!
//! Exercises OS UDP socket open/send/receive and ECN option
//! configuration using the [`crate::socks`] / [`crate::socks_socket2`]
//! abstractions.

#![allow(non_snake_case)]

use core::net::SocketAddr;

use crate::socks::{ServerSockets, Socket};
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
    client: &mut Socket2Udp,
    server_addr: SocketAddr,
    server_sockets: &mut ServerSockets<Socket2Udp>,
) -> crate::Result<()> {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0);
    let mut message = [0u8; 1440];
    let mut buffer = [0u8; 1536];

    let mut i = 0usize;
    while i < message.len() {
        let mut j = 0;
        while j < 64 && i < message.len() {
            message[i] = (current_time >> j) as u8;
            i += 1;
            j += 8;
            i += 1;
        }
    }

    let bytes_sent = client
        .send(&server_addr, None, 0, &message, 0)
        .map_err(|_| crate::Error::Generic)?;
    if bytes_sent != message.len() {
        return Err(crate::Error::Generic);
    }

    let client_port = client.local_address()?.port();
    let client_addr = match server_addr {
        SocketAddr::V4(addr) => {
            SocketAddr::V4(core::net::SocketAddrV4::new(*addr.ip(), client_port))
        }
        SocketAddr::V6(addr) => {
            SocketAddr::V6(core::net::SocketAddrV6::new(*addr.ip(), client_port, 0, 0))
        }
    };

    let server_index = if server_addr.is_ipv4() { 1 } else { 0 };
    let bytes_recv = {
        let server = server_sockets.sockets[server_index]
            .as_mut()
            .ok_or(crate::Error::Generic)?;
        let _ = server
            .0
            .set_read_timeout(Some(std::time::Duration::from_secs(1)));
        let recv = server.recv(&mut buffer)?;
        if recv.bytes_recv != bytes_sent {
            return Err(crate::Error::Generic);
        }
        recv.bytes_recv
    };

    for b in &mut buffer[..bytes_recv] {
        *b ^= 0xff;
    }

    let bytes_back = server_sockets
        .send_through(&client_addr, Some(&server_addr), 0, &buffer[..bytes_recv])
        .map_err(|_| crate::Error::Generic)?;
    if bytes_back != bytes_recv {
        return Err(crate::Error::Generic);
    }

    buffer.fill(0);
    let _ = client
        .0
        .set_read_timeout(Some(std::time::Duration::from_secs(1)));
    let recv = client.recv(&mut buffer)?;
    if recv.bytes_recv != bytes_sent {
        return Err(crate::Error::Generic);
    }
    for (expected, actual) in message.iter().zip(&buffer[..recv.bytes_recv]) {
        if *expected != (*actual ^ 0xff) {
            return Err(crate::Error::Generic);
        }
    }
    Ok(())
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

fn socket_ecn_test_one(af: i32) -> crate::Result<()> {
    let mut fd = Socket2Udp::open_client(af)?;
    let (recv_set, send_set) = fd.set_ecn_options()?;
    if !recv_set {
        return Err(crate::Error::Generic);
    }
    if !send_set && !cfg!(windows) {
        return Err(crate::Error::Generic);
    }
    Ok(())
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
