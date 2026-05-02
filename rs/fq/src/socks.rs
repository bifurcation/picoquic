//! Translation of `quic/socks.h`.
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
//!   newtype [`socket_t`].  Phase 3 will gate the body
//!   behind the `std` feature and route it through
//!   `std::os::fd::OwnedFd` / `BorrowedFd`.  The sentinel
//!   [`INVALID_SOCKET`] mirrors the C `-1`.
//! * `struct sockaddr*` and `struct sockaddr_storage*` →
//!   [`core::net::SocketAddr`], same convention as
//!   [`crate::utils`].  Output sockaddr_storage
//!   slots fold into `Option<SocketAddr>` (`AF_UNSPEC` ↔ `None`).
//! * `void* vmsg` is the platform `struct msghdr*`; kept as an
//!   opaque [`msghdr_t`] for now.  Phase 3 will replace
//!   it with a libc `msghdr` wrapper under the `std` feature.
//! * Receive metadata that the C surface exposes through several
//!   nullable out-parameters folds into result structs
//!   ([`RecvInfo`], [`SelectInfo`],
//!   [`SelectExInfo`]).  C call sites pass `NULL` when
//!   uninterested; Rust call sites can simply ignore unused
//!   fields, so the Boolean "interested?" argument disappears.
//! * `int bytes_recv` returns become `Result<usize, Error>` —
//!   `Ok(n)` for the C non-negative count, `Err(())` for the
//!   `-1` error path.
//! * `int* sock_err` out-parameters on [`sendmsg`] and
//!   the `_send_through_*` helpers fold into the `Err` arm:
//!   `Result<usize, i32>` carries the OS errno on failure.

// Stand-in for the not-yet-defined crate-level `Error` enum.

use core::net::SocketAddr;

use crate::Error;
use crate::quic_t;

// ---------------------------------------------------------------------------
// Socket constants and types.

/// Number of UDP sockets a server context owns — one IPv4, one
/// IPv6.  C: `NB_SERVER_SOCKETS`.
pub const NB_SERVER_SOCKETS: usize = 2;

/// OS socket descriptor.  C: `SOCKET_TYPE` (`int` on Linux; the
/// Windows `SOCKET` aliasing is dropped per the v1 scope).
///
/// Kept as an opaque newtype so the underlying integer doesn't
/// leak out of the module.  Phase 3 will swap the inner field for
/// `std::os::fd::OwnedFd` (under the `std` feature) and tighten
/// the sentinel handling to `Option<socket_t>`.
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct socket_t {
    /// Raw file descriptor.  `-1` is the [`INVALID_SOCKET`]
    /// sentinel; compare against [`INVALID_SOCKET`] rather than
    /// reading this field directly.
    fd: i32,
}

/// Sentinel for "no socket".  C: `INVALID_SOCKET = -1` on Linux.
pub const INVALID_SOCKET: socket_t = socket_t { fd: -1 };

/// Pair of UDP sockets owned by a server (one IPv4, one IPv6).
/// C: `server_sockets_t`.  `repr(C)` is dropped — the
/// struct is internal scratch, never inspected through FFI.
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
pub struct server_sockets_t {
    pub s_socket: [socket_t; NB_SERVER_SOCKETS],
}

impl Default for server_sockets_t {
    fn default() -> Self {
        server_sockets_t {
            s_socket: [INVALID_SOCKET; NB_SERVER_SOCKETS],
        }
    }
}

/// Opaque platform `msghdr` wrapper.  C: `void* vmsg` cast to
/// `struct msghdr*` inside the cmsg helpers.  Phase 3 will replace
/// this placeholder with a libc-bound `msghdr` (under the `std`
/// feature) — keeping the type opaque here so the public surface
/// doesn't pin a concrete representation prematurely.
#[allow(non_camel_case_types)]
pub struct msghdr_t {
    _opaque: [u8; 0],
}

// ---------------------------------------------------------------------------
// Socket setup.

/// Bind `fd` to `port` on the given address family
/// (`AF_INET` / `AF_INET6`).  C:
/// `int bind_to_port(SOCKET_TYPE fd, int af, int port)`
/// returning 0 / -1.
///
/// `af` stays an `i32` — call sites pass the `AF_*` constants from
/// `<sys/socket.h>` directly.
pub fn bind_to_port(_fd: socket_t, _af: i32, _port: i32) -> Result<(), Error> {
    todo!()
}

/// Read the local address bound to `sd` (`getsockname`).  C:
/// `int get_local_address(SOCKET_TYPE sd, struct
/// sockaddr_storage* addr)`.  The sockaddr out-parameter folds
/// into the `Ok` payload; `Err(())` matches the C `-1`.
pub fn get_local_address(_sd: socket_t) -> Result<SocketAddr, Error> {
    todo!()
}

/// Open a UDP client socket on `af`.  C:
/// `SOCKET_TYPE open_client_socket(int af)`.  The C body
/// returns `INVALID_SOCKET` on failure; the Rust signature keeps
/// the C-style sentinel surface (callers compare `!=
/// INVALID_SOCKET`).  Tightening to `Result<socket_t,
/// ()>` is a candidate Phase 3 follow-up once call sites are
/// translated.
pub fn open_client_socket(_af: i32) -> socket_t {
    todo!()
}

/// Open the IPv4 + IPv6 server-side UDP socket pair listening on
/// `port`.  C: `int open_server_sockets
/// (server_sockets_t* sockets, int port)`.  The
/// `sockets` output parameter folds into the `Ok` payload —
/// callers in `sockloop.c` declare the storage on the stack and
/// pass `&sockets`, so returning by value matches the lifetime
/// expectation.
pub fn open_server_sockets(_port: i32) -> Result<server_sockets_t, Error> {
    todo!()
}

/// Close every socket in the pair and stamp [`INVALID_SOCKET`]
/// over each slot.  C:
/// `void close_server_sockets(server_sockets_t* sockets)`.
pub fn close_server_sockets(_sockets: &mut server_sockets_t) {
    todo!()
}

/// Enable per-packet destination-info delivery on `sd`
/// (`IP_PKTINFO` / `IPV6_RECVPKTINFO`, plus `IPV6_V6ONLY` for v6).
/// C: `int socket_set_pkt_info(SOCKET_TYPE sd, int af)`
/// returning 0 / -1.
pub fn socket_set_pkt_info(_sd: socket_t, _af: i32) -> Result<(), Error> {
    todo!()
}

/// Enable ECN reception (and request `ECN_ECT_1` outbound) on
/// `sd`.  C: `int socket_set_ecn_options(SOCKET_TYPE sd,
/// int af, int* recv_set, int* send_set)` — the `recv_set` /
/// `send_set` out-params fold into the tuple return as `bool`
/// flags (the C uses 0/1).  `Err(())` matches the C `-1`
/// "neither could be configured" path.
pub fn socket_set_ecn_options(_sd: socket_t, _af: i32) -> Result<(bool, bool), Error> {
    todo!()
}

/// Like [`socket_set_ecn_options`] but with an explicit
/// outbound TOS / ECN value.  C:
/// `socket_set_ecn_options_ex` — `ecn_value` is one of
/// the 2-bit `ECN_ECT_*` codes.
pub fn socket_set_ecn_options_ex(
    _sd: socket_t,
    _af: i32,
    _ecn_value: u8,
) -> Result<(bool, bool), Error> {
    todo!()
}

/// Enable Path-MTU-Discovery probing on `sd` (Linux-only — a
/// no-op on other platforms).  C:
/// `int socket_set_pmtud_options(SOCKET_TYPE sd, int af)`
/// returning 0 / -1.
pub fn socket_set_pmtud_options(_sd: socket_t, _af: i32) -> Result<(), Error> {
    todo!()
}

// ---------------------------------------------------------------------------
// Receive / select.

/// Receive metadata returned by [`recvmsg`].  Mirrors
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

/// Receive one UDP datagram on `fd` into `buffer`.  C:
/// `int recvmsg(SOCKET_TYPE fd, struct sockaddr_storage*
/// addr_from, struct sockaddr_storage* addr_dest, int* dest_if,
/// unsigned char* received_ecn, uint8_t* buffer, int buffer_max)`
/// returning bytes received or `-1`.  Output parameters fold into
/// [`RecvInfo`]; the slice length subsumes
/// `buffer_max`.
pub fn recvmsg(_fd: socket_t, _buffer: &mut [u8]) -> Result<RecvInfo, Error> {
    todo!()
}

/// [`select`] result, mirroring [`RecvInfo`]
/// plus the `current_time()` snapshot the C body writes
/// through `*current_time`.
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
    /// Snapshot of `current_time()` taken after the
    /// receive; mirrors the C `*current_time` out-parameter.
    pub current_time: u64,
}

/// [`select_ex`] result — additionally reports which
/// socket fired (the C `*socket_rank` out-parameter, here a
/// `usize` index into the input slice).
#[derive(Debug, Default, Copy, Clone)]
pub struct SelectExInfo {
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
    /// Snapshot of `current_time()` taken after the
    /// receive; mirrors the C `*current_time` out-parameter.
    pub current_time: u64,
    /// Index into the input socket slice identifying which socket
    /// received the datagram; mirrors the C `*socket_rank`
    /// out-parameter.
    pub socket_rank: usize,
}

/// Wait up to `delta_t` microseconds for a packet on any socket
/// in `sockets`, then read at most `buffer.len()` bytes.  C:
/// `int select(SOCKET_TYPE* sockets, int nb_sockets,
/// struct sockaddr_storage* addr_from, struct sockaddr_storage*
/// addr_dest, int* dest_if, unsigned char* received_ecn, uint8_t*
/// buffer, int buffer_max, int64_t delta_t, uint64_t*
/// current_time)`.
///
/// The slice subsumes the C `(sockets, nb_sockets)` and
/// `(buffer, buffer_max)` pairs; the rest of the output folds
/// into [`SelectInfo`].  `Err(())` matches the C `-1`
/// from `select` / `recvmsg`.
pub fn select(
    _sockets: &[socket_t],
    _buffer: &mut [u8],
    _delta_t: i64,
) -> Result<SelectInfo, Error> {
    todo!()
}

/// Like [`select`] but also reports which socket fired.
/// C: `select_ex`.
pub fn select_ex(
    _sockets: &[socket_t],
    _buffer: &mut [u8],
    _delta_t: i64,
) -> Result<SelectExInfo, Error> {
    todo!()
}

// ---------------------------------------------------------------------------
// Send.

/// Send `bytes` from `addr_from` to `addr_dest` over `fd` with an
/// optional segmentation hint `send_msg_size` (`UDP_SEGMENT`).
/// C: `int sendmsg(SOCKET_TYPE fd, struct sockaddr*
/// addr_dest, struct sockaddr* addr_from, int dest_if, const
/// char* bytes, int length, int send_msg_size, int* sock_err)`.
///
/// `addr_from == NULL` (the "let the kernel pick the source"
/// path) maps to `None`.  The C `*sock_err` out-parameter folds
/// into the `Err` arm: `Result<usize, i32>` carries the `errno`
/// value on failure; `Ok(n)` reports the byte count actually
/// sent.  The C `int length` is the slice length.
pub fn sendmsg(
    _fd: socket_t,
    _addr_dest: &SocketAddr,
    _addr_from: Option<&SocketAddr>,
    _dest_if: i32,
    _bytes: &[u8],
    _send_msg_size: i32,
) -> Result<usize, i32> {
    todo!()
}

/// Convenience wrapper that calls [`sendmsg`] with
/// `send_msg_size = 0`.  C: `send_through_socket`.
pub fn send_through_socket(
    _fd: socket_t,
    _addr_dest: &SocketAddr,
    _addr_from: Option<&SocketAddr>,
    _from_if: i32,
    _bytes: &[u8],
) -> Result<usize, i32> {
    todo!()
}

/// Pick the matching socket from `sockets` (v4 / v6, dispatched
/// by `addr_dest`'s family) and send through it.  C:
/// `send_through_server_sockets`.
pub fn send_through_server_sockets(
    _sockets: &server_sockets_t,
    _addr_dest: &SocketAddr,
    _addr_from: Option<&SocketAddr>,
    _from_if: i32,
    _bytes: &[u8],
) -> Result<usize, i32> {
    todo!()
}

// ---------------------------------------------------------------------------
// Address resolution.

/// Result of [`get_server_address`]: the resolved
/// address paired with the C `is_name` flag (`true` when the
/// input was a hostname rather than a numeric IP, signaling to
/// the caller that the original text should be reused as the SNI
/// parameter).
#[derive(Debug, Copy, Clone)]
pub struct ServerAddress {
    pub server_address: SocketAddr,
    pub is_name: bool,
}

/// Parse `ip_address_text` (numeric IPv4 / IPv6 or hostname) and
/// combine with `server_port` into a [`SocketAddr`].  C:
/// `int get_server_address(const char* ip_address_text,
/// int server_port, struct sockaddr_storage* server_address, int*
/// is_name)`.  `Err(())` matches the C `-1` (DNS lookup failure
/// or unsupported family).
pub fn get_server_address(
    _ip_address_text: &str,
    _server_port: i32,
) -> Result<ServerAddress, Error> {
    todo!()
}

// ---------------------------------------------------------------------------
// Misc helpers.

/// Read `SSLKEYLOGFILE` from the process environment (gated
/// behind the build-time `WITHOUT_SSLKEYLOG` opt-out and
/// the per-context `is_sslkeylog_enabled` flag) and install it on
/// `quic`.  C: `void set_key_log_file_from_env
/// (quic_t* quic)`.
///
/// The environment lookup is a `std`-only operation (it goes
/// through `getenv` / `_dupenv_s`); Phase 3 will feature-gate the
/// body.
pub fn set_key_log_file_from_env(_quic: &mut quic_t) {
    todo!()
}

/// Whether `sock_err` (a Linux `errno` value or its Windows
/// equivalent) implies the destination is unreachable and the
/// owning path should be abandoned.  C:
/// `int socket_error_implies_unreachable(int sock_err)`
/// returning a 0/1 flag, mapped to `bool`.
pub fn socket_error_implies_unreachable(_sock_err: i32) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------
// cmsg helpers.

/// Parse the control-message data attached to a received
/// `msghdr`.  C: `void socks_cmsg_parse(void* vmsg,
/// struct sockaddr_storage* addr_dest, int* dest_if, unsigned
/// char* received_ecn, size_t* udp_coalesced_size)`.
///
/// Each C output parameter accepts NULL and is skipped when
/// absent — translated as `Option<&mut _>` so callers preserve
/// the "I don't care" choice.  `addr_dest` becomes `Option<&mut
/// Option<SocketAddr>>`: the outer `Option` is caller-presence,
/// the inner is the family-unspecified sentinel.
///
/// `udp_coalesced_size` is only ever populated on Windows
/// (`UDP_COALESCED_INFO` cmsg); the parameter is kept on the
/// signature for parity with the C declaration even though the
/// Linux body always leaves it untouched.
pub fn socks_cmsg_parse(
    _msg: &msghdr_t,
    _addr_dest: Option<&mut Option<SocketAddr>>,
    _dest_if: Option<&mut i32>,
    _received_ecn: Option<&mut u8>,
    _udp_coalesced_size: Option<&mut usize>,
) {
    todo!()
}

/// Format `IP_PKTINFO` / `IPV6_PKTINFO` / `UDP_SEGMENT`
/// control messages on an outgoing `msghdr`.  C:
/// `void socks_cmsg_format(void* vmsg, size_t
/// message_length, size_t send_msg_size, struct sockaddr*
/// addr_from, int dest_if)`.
///
/// `addr_from == NULL` (or family `AF_UNSPEC`) maps to `None` —
/// the C body short-circuits without writing the source-address
/// cmsg in that case.  `send_msg_size == 0` disables the
/// `UDP_SEGMENT` cmsg.
pub fn socks_cmsg_format(
    _msg: &mut msghdr_t,
    _message_length: usize,
    _send_msg_size: usize,
    _addr_from: Option<&SocketAddr>,
    _dest_if: i32,
) {
    todo!()
}

#[cfg(test)]
mod test {}
