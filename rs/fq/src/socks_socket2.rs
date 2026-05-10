//! Default [`crate::socks::Socket`] implementation backed by
//! [`socket2::Socket`].
//!
//! This is the canonical UDP-socket implementation for
//! Linux / macOS / Windows.  Other backends (no-std embedded,
//! the test simulator) implement the same trait against their
//! own platform abstractions.
//!
//! Phase 4 wires the safe parts through `socket2`'s
//! receive/send/option-setting calls.

use core::net::SocketAddr;

use crate::Error;
use crate::socks::{OsError, RecvInfo, Socket};

/// Newtype around [`socket2::Socket`] so we can implement
/// the [`crate::socks::Socket`] trait on it locally (the
/// orphan-rules forbid `impl crate::Socket for socket2::Socket`
/// directly, since neither type belongs to this crate before
/// the trait moves in).
pub struct Socket2Udp(pub socket2::Socket, i32);

impl Socket2Udp {
    /// Open a UDP client socket on address family `af`
    /// (`AF_INET` / `AF_INET6`).  Applies pktinfo / ECN /
    /// PMTUD options before returning.  C:
    /// `picoquic_open_client_socket`.
    pub fn open_client(af: i32) -> Result<Self, Error> {
        let domain = socket2::Domain::from(af);
        let sock = socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))
            .map_err(|_| Error::Generic)?;
        let mut udp = Socket2Udp(sock, af);
        let _ = udp.set_pkt_info();
        let _ = udp.set_ecn_options();
        let _ = udp.set_pmtud_options();
        Ok(udp)
    }

    /// Bind this socket to `port` on `af`.
    /// C: `picoquic_bind_to_port`.
    pub fn bind_to_port(&mut self, af: i32, port: i32) -> Result<(), Error> {
        use core::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
        self.1 = af;
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
        let af = if domain == socket2::Domain::IPV6 {
            libc::AF_INET6
        } else {
            libc::AF_INET
        };
        let mut udp = Socket2Udp(sock, af);
        let _ = udp.set_ecn_options();
        udp.set_pkt_info()?;
        let addr = if domain == socket2::Domain::IPV6 {
            SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port as u16, 0, 0))
        } else {
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port as u16))
        };
        udp.0
            .bind(&socket2::SockAddr::from(addr))
            .map_err(|_| Error::Generic)?;
        udp.set_pmtud_options()?;
        Ok(udp)
    }

    fn open_unbound(af: i32) -> Result<Self, Error> {
        let domain = socket2::Domain::from(af);
        let sock = socket2::Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))
            .map_err(|_| Error::Generic)?;
        Ok(Socket2Udp(sock, af))
    }
}

impl Socket for Socket2Udp {
    fn local_address(&self) -> Result<SocketAddr, Error> {
        let sa = self.0.local_addr().map_err(|_| Error::Generic)?;
        sa.as_socket().ok_or(Error::Generic)
    }

    fn recv(&mut self, buffer: &mut [u8]) -> Result<RecvInfo, Error> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;

            crate::socks::picoquic_recvmsg(self.0.as_raw_fd(), buffer)
        }

        #[cfg(not(unix))]
        {
            use std::io::Read;

            let bytes_recv = self.0.read(buffer).map_err(|_| Error::Generic)?;
            Ok(RecvInfo {
                bytes_recv,
                ..RecvInfo::default()
            })
        }
    }

    fn send(
        &mut self,
        addr_dest: &SocketAddr,
        addr_from: Option<&SocketAddr>,
        dest_if: i32,
        bytes: &[u8],
        gso_size: i32,
    ) -> Result<usize, OsError> {
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::io::AsRawFd;

            crate::socks::picoquic_sendmsg(
                self.0.as_raw_fd(),
                addr_dest,
                addr_from,
                dest_if,
                bytes,
                gso_size,
            )
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = addr_from;
            let _ = dest_if;
            let _ = gso_size;
            let sa = socket2::SockAddr::from(*addr_dest);
            self.0
                .send_to(bytes, &sa)
                .map_err(|e| OsError(e.raw_os_error().unwrap_or(-1)))
        }
    }

    fn open_udp(af: i32) -> Result<Self, Error> {
        Self::open_unbound(af)
    }

    fn bind_to_port(&mut self, af: i32, port: i32) -> Result<(), Error> {
        Socket2Udp::bind_to_port(self, af, port)
    }

    fn set_reuse_addr(&mut self, reuse: bool) -> Result<(), Error> {
        self.0.set_reuse_address(reuse).map_err(|_| Error::Generic)
    }

    fn set_reuse_port(&mut self, reuse: bool) -> Result<(), Error> {
        self.0.set_reuse_port(reuse).map_err(|_| Error::Generic)
    }

    fn set_send_buffer_size(&mut self, size: usize) -> Result<(), Error> {
        self.0
            .set_send_buffer_size(size)
            .map_err(|_| Error::Generic)
    }

    fn set_recv_buffer_size(&mut self, size: usize) -> Result<(), Error> {
        self.0
            .set_recv_buffer_size(size)
            .map_err(|_| Error::Generic)
    }

    fn set_pkt_info(&mut self) -> Result<(), Error> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;

            crate::socks::picoquic_socket_set_pkt_info(self.0.as_raw_fd(), self.1)
        }

        #[cfg(not(unix))]
        {
            Ok(())
        }
    }

    fn set_ecn_options(&mut self) -> Result<(bool, bool), Error> {
        self.set_ecn_options_ex(crate::socks::EcnCodepoint::Ect1)
    }

    fn set_ecn_options_ex(
        &mut self,
        ecn: crate::socks::EcnCodepoint,
    ) -> Result<(bool, bool), Error> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            crate::socks::picoquic_socket_set_ecn_options_ex(self.0.as_raw_fd(), self.1, ecn)
        }
        #[cfg(not(unix))]
        {
            let _ = ecn;
            Ok((false, false))
        }
    }

    fn set_pmtud_options(&mut self) -> Result<(), Error> {
        // Linux only: IP_MTU_DISCOVER / IPV6_MTU_DISCOVER with IP_PMTUDISC_PROBE.
        // No-op on other platforms, matching the C #ifdef __linux guard.
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::io::AsRawFd;

            crate::socks::picoquic_socket_set_pmtud_options(self.0.as_raw_fd(), self.1)
        }

        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    fn open_server_v4(port: i32) -> Result<Self, Error> {
        Self::open_bound(socket2::Domain::IPV4, port)
    }

    fn open_server_v6(port: i32) -> Result<Self, Error> {
        Self::open_bound(socket2::Domain::IPV6, port)
    }

    #[cfg(unix)]
    fn raw_fd(&self) -> i32 {
        use std::os::unix::io::AsRawFd;
        self.0.as_raw_fd()
    }
}

#[cfg(test)]
mod test {}
