//! Default [`crate::socks::Socket`] implementation backed by
//! [`socket2::Socket`].
//!
//! This is the canonical UDP-socket implementation for
//! Linux / macOS / Windows.  Other backends (no-std embedded,
//! the test simulator) implement the same trait against their
//! own platform abstractions.
//!
//! Phase 1 contract: signatures only — every method body is
//! stubs.  Phase 4 wires them through `socket2`'s
//! `recv_from`/`send_to`/`set_*` calls.

use core::net::SocketAddr;

use crate::Error;
use crate::socks::{OsError, RecvInfo, Socket};

/// Newtype around [`socket2::Socket`] so we can implement
/// the [`crate::socks::Socket`] trait on it locally (the
/// orphan-rules forbid `impl crate::Socket for socket2::Socket`
/// directly, since neither type belongs to this crate before
/// the trait moves in).
pub struct Socket2Udp(pub socket2::Socket);

impl Socket2Udp {
    /// Open a UDP client socket on address family `af`
    /// (`AF_INET` / `AF_INET6`).  Applies pktinfo / ECN /
    /// PMTUD options before returning.  C:
    /// `picoquic_open_client_socket`.
    pub fn open_client(af: i32) -> Result<Self, Error> {
        let domain = socket2::Domain::from(af);
        let sock = socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))
            .map_err(|_| Error::Generic)?;
        let mut udp = Socket2Udp(sock);
        let _ = udp.set_pkt_info();
        let _ = udp.set_ecn_options();
        let _ = udp.set_pmtud_options();
        Ok(udp)
    }

    /// Bind this socket to `port` on `af`.
    /// C: `picoquic_bind_to_port`.
    pub fn bind_to_port(&mut self, af: i32, port: i32) -> Result<(), Error> {
        use core::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
        let addr = if socket2::Domain::from(af) == socket2::Domain::IPV6 {
            SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port as u16, 0, 0))
        } else {
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port as u16))
        };
        self.0
            .bind(&socket2::SockAddr::from(addr))
            .map_err(|_| Error::Generic)
    }

    fn open_bound(domain: socket2::Domain, port: i32) -> Result<Self, Error> {
        use core::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
        let sock = socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))
            .map_err(|_| Error::Generic)?;
        let mut udp = Socket2Udp(sock);
        let _ = udp.set_pkt_info();
        let _ = udp.set_ecn_options();
        let _ = udp.set_pmtud_options();
        let addr = if domain == socket2::Domain::IPV6 {
            SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port as u16, 0, 0))
        } else {
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port as u16))
        };
        udp.0
            .bind(&socket2::SockAddr::from(addr))
            .map_err(|_| Error::Generic)?;
        Ok(udp)
    }
}

impl Socket for Socket2Udp {
    fn local_address(&self) -> Result<SocketAddr, Error> {
        let sa = self.0.local_addr().map_err(|_| Error::Generic)?;
        sa.as_socket().ok_or(Error::Generic)
    }

    fn recv(&mut self, buffer: &mut [u8]) -> Result<RecvInfo, Error> {
        use std::io::Read;
        let bytes_recv = self.0.read(buffer).map_err(|_| Error::Generic)?;
        Ok(RecvInfo {
            bytes_recv,
            ..RecvInfo::default()
        })
    }

    fn send(
        &mut self,
        addr_dest: &SocketAddr,
        _addr_from: Option<&SocketAddr>,
        _dest_if: i32,
        bytes: &[u8],
        _gso_size: i32,
    ) -> Result<usize, OsError> {
        let sa = socket2::SockAddr::from(*addr_dest);
        self.0
            .send_to(bytes, &sa)
            .map_err(|e| OsError(e.raw_os_error().unwrap_or(-1)))
    }

    fn set_pkt_info(&mut self) -> Result<(), Error> {
        // IPV6_V6ONLY is the part socket2 exposes; IP_PKTINFO /
        // IPV6_RECVPKTINFO require direct setsockopt (not in socket2 0.5).
        let _ = self.0.set_only_v6(true);
        Ok(())
    }

    fn set_ecn_options(&mut self) -> Result<(bool, bool), Error> {
        // Full ECN option setting (IP_TOS/IP_RECVTOS, IPV6_TCLASS/
        // IPV6_RECVTCLASS) is platform-specific and not uniformly
        // exposed by socket2 0.5.  Report unsupported for now.
        Ok((false, false))
    }

    fn set_pmtud_options(&mut self) -> Result<(), Error> {
        // IP_MTU_DISCOVER / IPV6_MTU_DISCOVER are Linux-only and not
        // exposed by socket2 0.5; no-op on other platforms.
        Ok(())
    }

    fn open_server_v4(port: i32) -> Result<Self, Error> {
        Self::open_bound(socket2::Domain::IPV4, port)
    }

    fn open_server_v6(port: i32) -> Result<Self, Error> {
        Self::open_bound(socket2::Domain::IPV6, port)
    }
}

#[cfg(test)]
mod test {}
