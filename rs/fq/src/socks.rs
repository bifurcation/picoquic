//! Translation of `picoquic/picosocks.h`.
//!
//! Thin wrappers over the platform UDP socket API used by
//! quic's default event loop.  All call sites are
//! Linux/macOS-targeted; the `_WINDOWS` paths in the C source —
//! including the entire `recvmsg_async_ctx_t` family
//! (overlapped I/O, `WSARecvMsg`/`WSASendMsg`, `WSAEVENT`) and the
//! UDP coalescing constants — are dropped per the v1 scope (see
//! `TRANSLATE_PLAN.md`).
//!
//! Phase 1 contract: signatures only — every function body is
//! `todo!()` and the empty `#[cfg(test)] mod test {}` lands at the
//! bottom for Phase 2 to fill.
//!
//! Pointer-shape decisions (read from `quic/socks.c`,
//! `quic/sockloop.c`):
//!
//! * `SOCKET_TYPE` (an OS file descriptor on Linux) → opaque
//!   newtype [`Socket`].  Phase 3 will gate the body behind the
//!   `std` feature and route it through `std::os::fd::OwnedFd` /
//!   `BorrowedFd`.  The C `INVALID_SOCKET = -1` sentinel becomes
//!   `Option<Socket>::None` — Rust call sites keep the "not yet
//!   open" state out of the [`Socket`] type itself.
//! * `struct sockaddr*` and `struct sockaddr_storage*` →
//!   [`core::net::SocketAddr`], same convention as
//!   [`crate::utils`].  Output sockaddr_storage slots fold into
//!   `Option<SocketAddr>` (`AF_UNSPEC` ↔ `None`).
//! * `void* vmsg` is the platform `struct msghdr*`; kept as an
//!   opaque [`MessageHeader`] for now.  Phase 3 will replace it with a
//!   libc `msghdr` wrapper under the `std` feature.
//! * Receive metadata that the C surface exposes through several
//!   nullable out-parameters folds into result structs
//!   ([`RecvInfo`], [`SelectInfo`], [`CmsgInfo`]).  C call sites
//!   pass `NULL` when uninterested; Rust call sites can simply
//!   ignore unused fields, so the Boolean "interested?" argument
//!   disappears.
//! * `int bytes_recv` returns become `Result<usize, Error>` —
//!   `Ok(n)` for the C non-negative count, `Err(_)` for the
//!   `-1` error path.
//! * `int* sock_err` out-parameters on [`Socket::sendmsg`] and
//!   the `send_through` helpers fold into the `Err` arm: the
//!   typed [`OsError`] newtype carries the OS errno on failure
//!   and exposes [`OsError::is_unreachable`] for the path-abandon
//!   decision the C `picoquic_socket_error_implies_unreachable`
//!   helper made.

use core::net::SocketAddr;

use crate::Error;

// ---------------------------------------------------------------------------
// Socket constants and types.

/// Number of UDP sockets a server context owns — one IPv4, one
/// IPv6.  C: `PICOQUIC_NB_SERVER_SOCKETS`.
pub const SERVER_SOCKET_COUNT: usize = 2;

/// OS socket descriptor.  C: `SOCKET_TYPE` (`int` on Linux; the
/// Windows `SOCKET` aliasing is dropped per the v1 scope).
///
/// Kept as an opaque newtype so the underlying integer doesn't
/// leak out of the module.  Phase 4 will swap the inner field for
/// `std::os::fd::OwnedFd` (under the `std` feature).  The "not
/// yet open" state is modelled by `Option<Socket>` rather than an
/// in-band sentinel — a [`Socket`] value always names a real fd.
///
/// REVIEW(open): Phase 2 (dependency abstraction) replaces this
/// with a `Socket` trait that the host platform implements
/// (libc on Unix, `WSARecv`/`WSASend` on Windows).
/// [`MessageHeader`] gets the same treatment, probably as an
/// associated type of `Socket` so each backend can pick its own
/// representation.  [`ServerSockets`] becomes generic over `S: Socket`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Socket {
    fd: i32,
}

/// Pair of UDP sockets owned by a server (one IPv4, one IPv6).
/// C: `picoquic_server_sockets_t`.  `repr(C)` is dropped — the
/// struct is internal scratch, never inspected through FFI.
#[derive(Debug, Default, Copy, Clone)]
pub struct ServerSockets {
    /// One UDP socket per address family.  `None` slots are
    /// uninitialised — see [`ServerSockets::open`] /
    /// [`ServerSockets::close`].
    pub sockets: [Option<Socket>; SERVER_SOCKET_COUNT],
}

/// Opaque platform `msghdr` wrapper.  C: `void* vmsg` cast to
/// `struct msghdr*` inside the cmsg helpers.  Phase 4 will replace
/// this placeholder with a libc-bound `msghdr` (under the `std`
/// feature) — keeping the type opaque here so the public surface
/// doesn't pin a concrete representation prematurely.
pub struct MessageHeader(());

/// OS error code surfaced by [`Socket::sendmsg`] and the
/// `send_through` helpers when the underlying syscall fails.
///
/// Wraps the raw Linux `errno` (or its Windows equivalent on
/// other targets) and exposes [`OsError::is_unreachable`] for the
/// classification the C source called
/// `picoquic_socket_error_implies_unreachable`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OsError(pub i32);

impl OsError {
    /// Whether this errno implies the destination is unreachable
    /// and the owning path should be abandoned.  C:
    /// `picoquic_socket_error_implies_unreachable`.
    pub fn is_unreachable(self) -> bool {
        todo!()
    }
}

impl Socket {
    /// Open a UDP client socket on address family `af`
    /// (`AF_INET` / `AF_INET6`).  Applies [`Socket::set_pkt_info`],
    /// [`Socket::set_ecn_options`], and [`Socket::set_pmtud_options`]
    /// before returning.  C: `picoquic_open_client_socket`.
    pub fn open_client(_af: i32) -> Result<Self, Error> {
        todo!()
    }

    /// Bind `self` to `port` on `af` (`AF_INET` / `AF_INET6`).
    /// `af` stays `i32` — callers pass libc `AF_*` constants directly.
    /// C: `picoquic_bind_to_port`.
    pub fn bind_to_port(self, _af: i32, _port: i32) -> Result<(), Error> {
        todo!()
    }

    /// Return the local address bound to `self` (`getsockname`).
    /// C: `picoquic_get_local_address`.
    pub fn local_address(self) -> Result<SocketAddr, Error> {
        todo!()
    }

    /// Enable per-packet destination-info delivery on `self`
    /// (`IP_PKTINFO` / `IPV6_RECVPKTINFO`, plus `IPV6_V6ONLY` for v6).
    /// C: `picoquic_socket_set_pkt_info`.
    pub fn set_pkt_info(self, _af: i32) -> Result<(), Error> {
        todo!()
    }

    /// Enable ECN reception (and request `ECN_ECT_1` outbound) on
    /// `self`.  Returns `(recv_set, send_set)` indicating whether the
    /// kernel honored each socket option.  C: `picoquic_socket_set_ecn_options`.
    pub fn set_ecn_options(self, _af: i32) -> Result<(bool, bool), Error> {
        todo!()
    }

    /// Like [`Socket::set_ecn_options`] but with an explicit outbound
    /// TOS / ECN value (`ecn_value` is one of the 2-bit `ECN_ECT_*`
    /// codes).  C: `picoquic_socket_set_ecn_options_ex`.
    pub fn set_ecn_options_ex(self, _af: i32, _ecn_value: u8) -> Result<(bool, bool), Error> {
        todo!()
    }

    /// Enable Path-MTU-Discovery probing on `self` (Linux-only; no-op
    /// on other platforms).  C: `picoquic_socket_set_pmtud_options`.
    pub fn set_pmtud_options(self, _af: i32) -> Result<(), Error> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// ServerSockets operations.

impl ServerSockets {
    /// Open the IPv4 + IPv6 server-side UDP socket pair listening on
    /// `port`.  C: `picoquic_open_server_sockets`.
    pub fn open(_port: i32) -> Result<Self, Error> {
        todo!()
    }

    /// Close every socket in the pair, leaving each slot `None`.
    /// C: `picoquic_close_server_sockets`.
    pub fn close(&mut self) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Receive / select.

/// Receive metadata returned by [`Socket::recvmsg`].  Mirrors
/// the C output parameters folded into a single struct.
#[derive(Debug, Default, Copy, Clone)]
pub struct RecvInfo {
    /// Source address; `None` matches the C `addr_from->ss_family
    /// = 0` written when `recvmsg` returned `<= 0`.
    pub addr_from: Option<SocketAddr>,
    /// Destination address parsed out of `IP_PKTINFO` /
    /// `IPV6_PKTINFO` cmsg; `None` when no pktinfo was attached.
    pub addr_dest: Option<SocketAddr>,
    /// Receiving interface index from the same cmsg.
    pub dest_if: i32,
    /// `IP_TOS` / `IPV6_TCLASS` ECN code-point byte.
    pub received_ecn: u8,
    /// Number of bytes written into the caller's buffer.
    pub bytes_recv: usize,
}

impl Socket {
    /// Receive one UDP datagram into `buffer`.  Output parameters
    /// (source/dest address, interface index, ECN, byte count) fold
    /// into [`RecvInfo`]; the slice length subsumes `buffer_max`.
    /// C: `picoquic_recvmsg`.
    pub fn recvmsg(self, _buffer: &mut [u8]) -> Result<RecvInfo, Error> {
        todo!()
    }
}

/// Result of [`select`].
///
/// Folds the seven nullable out-parameters of the C
/// `picoquic_select_ex` helper into one struct: source / destination
/// address, interface index, ECN code-point, byte count,
/// post-receive timestamp, and the index of the socket that fired.
/// The C library exposes both `picoquic_select` (rank discarded) and
/// `picoquic_select_ex` (rank reported); the Rust port keeps the rank
/// as a field so a single entry point covers both call patterns.
#[derive(Debug, Default, Copy, Clone)]
pub struct SelectInfo {
    /// Source address; `None` matches the C `addr_from->ss_family
    /// = 0` written when `recvmsg` returned `<= 0`.
    pub addr_from: Option<SocketAddr>,
    /// Destination address parsed out of `IP_PKTINFO` /
    /// `IPV6_PKTINFO` cmsg; `None` when no pktinfo was attached.
    pub addr_dest: Option<SocketAddr>,
    /// Receiving interface index from the same cmsg.
    pub dest_if: i32,
    /// `IP_TOS` / `IPV6_TCLASS` ECN code-point byte.
    pub received_ecn: u8,
    /// Number of bytes written into the caller's buffer.
    pub bytes_recv: usize,
    /// Snapshot of `current_time()` taken after the receive;
    /// mirrors the C `*current_time` out-parameter.
    pub current_time: u64,
    /// Index into the input socket slice identifying which socket
    /// received the datagram; mirrors the C `*socket_rank`
    /// out-parameter.
    pub socket_rank: usize,
}

/// Wait up to `delta_t` microseconds for a packet on any socket
/// in `sockets`, then read at most `buffer.len()` bytes.
///
/// The slice subsumes the C `(sockets, nb_sockets)` and
/// `(buffer, buffer_max)` pairs; the rest of the output folds
/// into [`SelectInfo`].  A negative or zero `delta_t` matches the
/// C "no wait" path.  C: `picoquic_select` and `picoquic_select_ex`
/// are merged here — the C `_ex` form is the more general one;
/// the rank just lives in [`SelectInfo`].
pub fn select(_sockets: &[Socket], _buffer: &mut [u8], _delta_t: i64) -> Result<SelectInfo, Error> {
    todo!()
}

impl Socket {
    /// Send `bytes` to `addr_dest` from `addr_from` with an optional
    /// GSO segmentation hint `send_msg_size` (`UDP_SEGMENT`).
    /// `addr_from = None` lets the kernel choose the source address.
    /// `Ok(n)` reports bytes sent; `Err` carries the OS error — see
    /// [`OsError::is_unreachable`] for the abandon-path
    /// classification.  C: `picoquic_sendmsg`.
    pub fn sendmsg(
        self,
        _addr_dest: &SocketAddr,
        _addr_from: Option<&SocketAddr>,
        _dest_if: i32,
        _bytes: &[u8],
        _send_msg_size: i32,
    ) -> Result<usize, OsError> {
        todo!()
    }

    /// Convenience wrapper: calls [`Socket::sendmsg`] with
    /// `send_msg_size = 0`.  C: `picoquic_send_through_socket`.
    pub fn send_through(
        self,
        _addr_dest: &SocketAddr,
        _addr_from: Option<&SocketAddr>,
        _from_if: i32,
        _bytes: &[u8],
    ) -> Result<usize, OsError> {
        todo!()
    }
}

impl ServerSockets {
    /// Pick the matching socket (v4 / v6, dispatched by
    /// `addr_dest`'s family) and send through it.
    /// C: `picoquic_send_through_server_sockets`.
    pub fn send_through(
        &self,
        _addr_dest: &SocketAddr,
        _addr_from: Option<&SocketAddr>,
        _from_if: i32,
        _bytes: &[u8],
    ) -> Result<usize, OsError> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Address resolution.

/// Resolved server address plus the "was the input a hostname?"
/// hint the TLS layer needs to choose an SNI value.
#[derive(Debug, Copy, Clone)]
pub struct ServerAddress {
    /// The resolved socket address.
    pub addr: SocketAddr,
    /// `true` when the input was a hostname rather than a numeric IP,
    /// signaling to the caller that the original text should be reused
    /// as the TLS SNI parameter.
    pub is_name: bool,
}

impl ServerAddress {
    /// Parse `ip_address_text` (numeric IPv4 / IPv6 or hostname) and
    /// combine with `server_port` into a [`ServerAddress`].
    /// C: `picoquic_get_server_address`.  `Err` matches the C `-1`
    /// (DNS lookup failure or unsupported family).
    pub fn resolve(_ip_address_text: &str, _server_port: i32) -> Result<Self, Error> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// MessageHeader control-message helpers.

/// Result of [`MessageHeader::parse_cmsg`].  Folds the four nullable
/// out-parameters of the C helper into one struct.
#[derive(Debug, Default, Copy, Clone)]
pub struct CmsgInfo {
    /// Destination address parsed from `IP_PKTINFO` / `IPV6_PKTINFO`;
    /// `None` when no pktinfo was attached (matches the C
    /// `AF_UNSPEC` sentinel).
    pub addr_dest: Option<SocketAddr>,
    /// Receiving interface index from the same cmsg.
    pub dest_if: Option<i32>,
    /// `IP_TOS` / `IPV6_TCLASS` ECN code-point, or `None` when the
    /// kernel didn't attach one.
    pub received_ecn: Option<EcnCodepoint>,
    /// `UDP_GRO` segment size; always `0` on Linux (the
    /// `UDP_COALESCED_INFO` cmsg the C source reads is
    /// Windows-only).
    pub udp_coalesced_size: usize,
}

/// IP/IPv6 ECN code-point reported by the kernel on receive.
/// Mirrors the two-bit `IP_TOS` / `IPV6_TCLASS` ECN field.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum EcnCodepoint {
    /// `Not-ECT` — not ECN-capable.
    #[default]
    NotEct = 0b00,
    /// `ECT(1)` — ECN-capable, L4S nominal mark.
    Ect1 = 0b01,
    /// `ECT(0)` — ECN-capable, classic.
    Ect0 = 0b10,
    /// `CE` — congestion experienced.
    Ce = 0b11,
}

impl MessageHeader {
    /// Parse the control-message ancillary data attached to a
    /// received `msghdr`.  C: `picoquic_socks_cmsg_parse`.
    pub fn parse_cmsg(&self) -> CmsgInfo {
        todo!()
    }

    /// Write `IP_PKTINFO` / `IPV6_PKTINFO` / `UDP_SEGMENT` control
    /// messages into `self` for an outgoing datagram.
    /// `addr_from = None` (or `AF_UNSPEC`) skips the source-address
    /// cmsg.  `send_msg_size = 0` disables the `UDP_SEGMENT` cmsg.
    /// C: `picoquic_socks_cmsg_format`.
    pub fn format_cmsg(
        &mut self,
        _message_length: usize,
        _send_msg_size: usize,
        _addr_from: Option<&SocketAddr>,
        _dest_if: i32,
    ) {
        todo!()
    }
}

#[cfg(test)]
mod test {}
