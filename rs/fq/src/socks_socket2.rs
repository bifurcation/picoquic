//! Default [`crate::socks::Socket`] implementation backed by
//! [`socket2::Socket`].
//!
//! This is the canonical UDP-socket implementation for
//! Linux / macOS / Windows.  Other backends (no-std embedded,
//! the test simulator) implement the same trait against their
//! own platform abstractions.
//!
//! Phase 1 contract: signatures only — every method body is
//! `todo!()`.  Phase 4 wires them through `socket2`'s
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
    pub fn open_client(_af: i32) -> Result<Self, Error> {
        todo!()
    }

    /// Bind this socket to `port` on `af`.
    /// C: `picoquic_bind_to_port`.
    pub fn bind_to_port(&mut self, _af: i32, _port: i32) -> Result<(), Error> {
        todo!()
    }
}

impl Socket for Socket2Udp {
    fn local_address(&self) -> Result<SocketAddr, Error> {
        todo!()
    }

    fn recv(&mut self, _buffer: &mut [u8]) -> Result<RecvInfo, Error> {
        todo!()
    }

    fn send(
        &mut self,
        _addr_dest: &SocketAddr,
        _addr_from: Option<&SocketAddr>,
        _dest_if: i32,
        _bytes: &[u8],
        _gso_size: i32,
    ) -> Result<usize, OsError> {
        todo!()
    }

    fn set_pkt_info(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn set_ecn_options(&mut self) -> Result<(bool, bool), Error> {
        todo!()
    }

    fn set_pmtud_options(&mut self) -> Result<(), Error> {
        todo!()
    }
}

#[cfg(test)]
mod test {}
