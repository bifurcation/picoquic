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
        let uninit_buffer = unsafe {
            // SAFETY: `recv_from` writes at most `buffer.len()` initialized
            // bytes into the same allocation, and `u8` has no invalid bit
            // patterns. The initialized prefix remains readable through
            // `buffer` after the call returns.
            &mut *(buffer as *mut [u8] as *mut [core::mem::MaybeUninit<u8>])
        };
        let (bytes_recv, addr_from) = self
            .0
            .recv_from(uninit_buffer)
            .map_err(|_| Error::Generic)?;
        Ok(RecvInfo {
            addr_from: addr_from.as_socket(),
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
            let fd = self.0.as_raw_fd();
            let val: libc::c_int = 1;
            let vp = &val as *const libc::c_int as *const libc::c_void;
            let vl = core::mem::size_of::<libc::c_int>() as libc::socklen_t;
            // IPv6 path: IPV6_V6ONLY=1 then IPV6_RECVPKTINFO=1.
            // setsockopt returns -1 on an AF_INET socket (wrong protocol),
            // so the if-check naturally skips IPV6_RECVPKTINFO on IPv4 sockets.
            let r6only =
                unsafe { libc::setsockopt(fd, libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, vp, vl) };
            if r6only == 0 {
                unsafe { libc::setsockopt(fd, libc::IPPROTO_IPV6, libc::IPV6_RECVPKTINFO, vp, vl) };
            }
            // IPv4 path: IP_PKTINFO (Linux/Android) or IP_RECVDSTADDR (BSD/macOS).
            // setsockopt returns -1 on an AF_INET6 socket; error is ignored.
            #[cfg(any(target_os = "linux", target_os = "android"))]
            unsafe {
                libc::setsockopt(fd, libc::IPPROTO_IP, libc::IP_PKTINFO, vp, vl)
            };
            #[cfg(not(any(target_os = "linux", target_os = "android")))]
            unsafe {
                libc::setsockopt(fd, libc::IPPROTO_IP, libc::IP_RECVDSTADDR, vp, vl)
            };
        }
        Ok(())
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
            let fd = self.0.as_raw_fd();
            let val: libc::c_int = libc::IP_PMTUDISC_PROBE;
            let vp = &val as *const libc::c_int as *const libc::c_void;
            let vl = core::mem::size_of::<libc::c_int>() as libc::socklen_t;
            // Apply to both families; the one that doesn't match the socket's
            // AF will return -1 (ignored), matching the C af-branch behavior.
            unsafe { libc::setsockopt(fd, libc::IPPROTO_IPV6, libc::IPV6_MTU_DISCOVER, vp, vl) };
            unsafe { libc::setsockopt(fd, libc::IPPROTO_IP, libc::IP_MTU_DISCOVER, vp, vl) };
        }
        Ok(())
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
