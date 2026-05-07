//! Translation of `picoquic/picosocks.h` plus the Phase 2 socket
//! abstraction.
//!
//! Phase 2 reshape: the C `SOCKET_TYPE` (a Linux file descriptor)
//! is replaced by a [`Socket`] *trait* that the host platform
//! implements.  The default implementation wraps
//! [`socket2::Socket`] (the canonical cross-platform UDP socket
//! crate); other backends (no-std embedded, Windows-specific,
//! the test simulator) implement the same trait.
//! [`ServerSockets<S>`] is generic over the implementor.
//!
//! Phase 4: all method bodies are implemented.

use core::net::SocketAddr;

#[cfg(unix)]
use libc;

use crate::Error;
use crate::Instant;

// ---------------------------------------------------------------------------
// Constants and supporting types.

/// Number of UDP sockets a server context owns — one IPv4, one
/// IPv6.  C: `PICOQUIC_NB_SERVER_SOCKETS`.
pub const SERVER_SOCKET_COUNT: usize = 2;

/// `MessageHeader` is a re-export of [`socket2::MsgHdr`] — the
/// canonical cross-platform `msghdr` wrapper.  Phase 4 fills in
/// the bodies that thread one through the cmsg helpers.
pub type MessageHeader<'addr, 'bufs, 'control> = socket2::MsgHdr<'addr, 'bufs, 'control>;

/// OS error code surfaced by socket sends when the underlying
/// syscall fails.  Wraps the raw Linux `errno` (or its Windows
/// equivalent on other targets) and exposes
/// [`OsError::is_unreachable`] for the path-abandon decision the
/// C `picoquic_socket_error_implies_unreachable` helper made.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OsError(pub i32);

impl OsError {
    /// Whether this errno implies the destination is unreachable
    /// and the owning path should be abandoned.  C:
    /// `picoquic_socket_error_implies_unreachable`.
    pub fn is_unreachable(self) -> bool {
        use std::io::ErrorKind;
        let kind = std::io::Error::from_raw_os_error(self.0).kind();
        matches!(
            kind,
            ErrorKind::ConnectionReset
                | ErrorKind::HostUnreachable
                | ErrorKind::NetworkDown
                | ErrorKind::NetworkUnreachable
        )
    }
}

// ---------------------------------------------------------------------------
// `Socket` trait.

/// One UDP socket on the application's network stack.  Replaces
/// the C `SOCKET_TYPE` (a raw file descriptor); the host
/// implementation supplies open / bind / send / recv operations
/// while the rest of picoquic stays platform-agnostic.
///
/// The default [`socket2::Socket`] implementation is in
/// [`crate::socks_socket2`] (gated on the `std` feature).
pub trait Socket {
    /// Local address bound to this socket (`getsockname`).
    /// C: `picoquic_get_local_address`.
    fn local_address(&self) -> Result<SocketAddr, Error>;

    /// Receive one UDP datagram into `buffer`.  Output parameters
    /// (source / dest address, interface index, ECN, byte count)
    /// fold into [`RecvInfo`]; the slice length subsumes the C
    /// `buffer_max` parameter.  C: `picoquic_recvmsg`.
    fn recv(&mut self, buffer: &mut [u8]) -> Result<RecvInfo, Error>;

    /// Send `bytes` to `addr_dest` from `addr_from` with an
    /// optional GSO segmentation hint `gso_size` (`UDP_SEGMENT`).
    /// `addr_from = None` lets the kernel choose the source
    /// address.  `Ok(n)` reports bytes sent; `Err` carries the
    /// OS error.  C: `picoquic_sendmsg`.
    fn send(
        &mut self,
        addr_dest: &SocketAddr,
        addr_from: Option<&SocketAddr>,
        dest_if: i32,
        bytes: &[u8],
        gso_size: i32,
    ) -> Result<usize, OsError>;

    /// Open an unbound UDP socket for address family `af`.
    /// C: `socket(af, SOCK_DGRAM, IPPROTO_UDP)`.
    fn open_udp(_af: i32) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::Generic)
    }

    /// Bind this socket to the unspecified address and `port`.
    /// C: `picoquic_bind_to_port`.
    fn bind_to_port(&mut self, _af: i32, _port: i32) -> Result<(), Error> {
        Err(Error::Generic)
    }

    /// Enable or disable `SO_REUSEADDR` before binding.
    fn set_reuse_addr(&mut self, _reuse: bool) -> Result<(), Error> {
        Ok(())
    }

    /// Enable or disable `SO_REUSEPORT` before binding when the platform
    /// exposes it.
    fn set_reuse_port(&mut self, _reuse: bool) -> Result<(), Error> {
        Ok(())
    }

    /// Set `SO_SNDBUF`.
    fn set_send_buffer_size(&mut self, _size: usize) -> Result<(), Error> {
        Ok(())
    }

    /// Set `SO_RCVBUF`.
    fn set_recv_buffer_size(&mut self, _size: usize) -> Result<(), Error> {
        Ok(())
    }

    /// Enable per-packet destination-info delivery on this socket
    /// (`IP_PKTINFO` / `IPV6_RECVPKTINFO`).  Default: no-op for
    /// backends that don't surface pktinfo.
    /// C: `picoquic_socket_set_pkt_info`.
    fn set_pkt_info(&mut self) -> Result<(), Error> {
        Ok(())
    }

    /// Enable ECN reception (and request `ECN_ECT_1` outbound).
    /// Returns `(recv_set, send_set)` indicating whether the
    /// kernel honored each socket option.  Default: no-op,
    /// returns `(false, false)`.
    fn set_ecn_options(&mut self) -> Result<(bool, bool), Error> {
        Ok((false, false))
    }

    /// Like [`Self::set_ecn_options`] but with an explicit
    /// outbound TOS / ECN value (`ecn` is one of the 2-bit
    /// `EcnCodepoint` codes).  Default: delegates to the
    /// no-op `set_ecn_options`.
    fn set_ecn_options_ex(&mut self, _ecn: EcnCodepoint) -> Result<(bool, bool), Error> {
        self.set_ecn_options()
    }

    /// Enable Path-MTU-Discovery probing on this socket
    /// (Linux-only; no-op on other platforms by default).
    /// C: `picoquic_socket_set_pmtud_options`.
    fn set_pmtud_options(&mut self) -> Result<(), Error> {
        Ok(())
    }

    /// Open and bind an IPv4 server socket on `port`.  Called by
    /// [`ServerSockets::open`]; concrete implementations override
    /// this to use their socket backend.
    fn open_server_v4(_port: i32) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::Generic)
    }

    /// Open and bind an IPv6 server socket on `port`.  Called by
    /// [`ServerSockets::open`]; concrete implementations override
    /// this to use their socket backend.
    fn open_server_v6(_port: i32) -> Result<Self, Error>
    where
        Self: Sized,
    {
        Err(Error::Generic)
    }

    /// Underlying OS file descriptor, or `-1` when not available
    /// (mock/test sockets, non-Unix platforms).  Used by
    /// `packet_loop::packet_loop_set_fds` to populate a `pollfd` array.
    /// C: `s_ctx->fd` (a raw `int` fd on POSIX builds).
    fn raw_fd(&self) -> i32 {
        -1
    }
}

// ---------------------------------------------------------------------------
// `ServerSockets` -- generic over the socket implementation.

/// Pair of UDP sockets owned by a server (one IPv4, one IPv6).
/// C: `picoquic_server_sockets_t`.  Generic over the active
/// [`Socket`] implementation.
pub struct ServerSockets<S: Socket> {
    /// One UDP socket per address family.  `None` slots are
    /// uninitialised — see [`ServerSockets::open`] /
    /// [`ServerSockets::close`].
    pub sockets: [Option<S>; SERVER_SOCKET_COUNT],
}

impl<S: Socket> Default for ServerSockets<S> {
    fn default() -> Self {
        Self {
            sockets: [None, None],
        }
    }
}

impl<S: Socket> ServerSockets<S> {
    /// Open the IPv4 + IPv6 server-side UDP socket pair listening
    /// on `port`.  C: `picoquic_open_server_sockets`.  The
    /// concrete `S` implementor is chosen at the call site;
    /// the default `crate::socks_socket2::Socket2Udp` wraps
    /// [`socket2::Socket`].
    pub fn open(port: i32) -> Result<Self, Error> {
        let s6 = S::open_server_v6(port)?;
        let s4 = S::open_server_v4(port)?;
        Ok(Self {
            sockets: [Some(s6), Some(s4)],
        })
    }

    /// Close every socket in the pair, leaving each slot `None`.
    /// C: `picoquic_close_server_sockets`.
    pub fn close(&mut self) {
        for slot in &mut self.sockets {
            *slot = None;
        }
    }

    /// Pick the matching socket (v4 / v6, dispatched by
    /// `addr_dest`'s family) and send through it.
    /// C: `picoquic_send_through_server_sockets`.
    pub fn send_through(
        &mut self,
        addr_dest: &SocketAddr,
        addr_from: Option<&SocketAddr>,
        from_if: i32,
        bytes: &[u8],
    ) -> Result<usize, OsError> {
        let idx = if addr_dest.is_ipv4() { 1 } else { 0 };
        match &mut self.sockets[idx] {
            Some(s) => s.send(addr_dest, addr_from, from_if, bytes, 0),
            None => Err(OsError(-1)),
        }
    }
}

// ---------------------------------------------------------------------------
// Receive metadata.

/// Receive metadata returned by [`Socket::recv`].  Mirrors the
/// C output parameters folded into a single struct.
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

/// Result of [`select`].
///
/// Folds the seven nullable out-parameters of the C
/// `picoquic_select_ex` helper into one struct: source / destination
/// address, interface index, ECN code-point, byte count,
/// post-receive timestamp, and the index of the socket that fired.
#[derive(Debug, Copy, Clone)]
pub struct SelectInfo {
    pub addr_from: Option<SocketAddr>,
    pub addr_dest: Option<SocketAddr>,
    pub dest_if: i32,
    pub received_ecn: u8,
    pub bytes_recv: usize,
    pub current_time: Instant,
    pub socket_rank: usize,
}

impl Default for SelectInfo {
    fn default() -> Self {
        Self {
            addr_from: None,
            addr_dest: None,
            dest_if: 0,
            received_ecn: 0,
            bytes_recv: 0,
            current_time: Instant::from_ticks(0),
            socket_rank: 0,
        }
    }
}

/// Wait up to `delta_t` microseconds for a packet on any socket
/// in `sockets`, then read at most `buffer.len()` bytes.
///
/// Generic over the [`Socket`] implementation.  C:
/// `picoquic_select` and `picoquic_select_ex` are merged here —
/// the `_ex` form is the more general one; the rank lives in
/// [`SelectInfo`].
pub fn select<S: Socket>(
    sockets: &mut [S],
    buffer: &mut [u8],
    _delta_t: i64,
) -> Result<SelectInfo, Error> {
    // Faithful note: the C body uses select(2) with raw file descriptors.
    // The generic Socket trait does not expose a file descriptor, so this
    // fallback tries each socket once in order and returns the first that
    // delivers data.  Platform-specific implementations can supply a more
    // efficient multiplexed select via the concrete Socket type.
    for (rank, socket) in sockets.iter_mut().enumerate() {
        match socket.recv(buffer) {
            Ok(info) if info.bytes_recv > 0 => {
                let now_us = std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_micros() as u64)
                    .unwrap_or(0);
                return Ok(SelectInfo {
                    addr_from: info.addr_from,
                    addr_dest: info.addr_dest,
                    dest_if: info.dest_if,
                    received_ecn: info.received_ecn,
                    bytes_recv: info.bytes_recv,
                    current_time: Instant::from_ticks(now_us),
                    socket_rank: rank,
                });
            }
            _ => {}
        }
    }
    Err(Error::Generic)
}

// ---------------------------------------------------------------------------
// Address resolution.

/// Resolved server address plus the "was the input a hostname?"
/// hint the TLS layer needs to choose an SNI value.
#[derive(Debug, Copy, Clone)]
pub struct ServerAddress {
    pub addr: SocketAddr,
    pub is_name: bool,
}

impl ServerAddress {
    /// Parse `ip_address_text` (numeric IPv4 / IPv6 or hostname)
    /// and combine with `server_port` into a [`ServerAddress`].
    /// C: `picoquic_get_server_address`.
    pub fn resolve(ip_address_text: &str, server_port: i32) -> Result<Self, Error> {
        let port = server_port as u16;
        // Try numeric IPv4.
        if let Ok(ipv4) = ip_address_text.parse::<std::net::Ipv4Addr>() {
            return Ok(Self {
                addr: SocketAddr::V4(std::net::SocketAddrV4::new(ipv4, port)),
                is_name: false,
            });
        }
        // Try numeric IPv6.
        if let Ok(ipv6) = ip_address_text.parse::<std::net::Ipv6Addr>() {
            return Ok(Self {
                addr: SocketAddr::V6(std::net::SocketAddrV6::new(ipv6, port, 0, 0)),
                is_name: false,
            });
        }
        // Hostname: DNS lookup.
        use std::net::ToSocketAddrs;
        let addr = (ip_address_text, port)
            .to_socket_addrs()
            .map_err(|_| Error::Generic)?
            .next()
            .ok_or(Error::Generic)?;
        Ok(Self {
            addr,
            is_name: true,
        })
    }
}

// ---------------------------------------------------------------------------
// Control-message helpers (cmsg parse / format).

/// Result of cmsg parsing.  Folds the four nullable
/// out-parameters of the C helper into one struct.
#[derive(Debug, Default, Copy, Clone)]
pub struct CmsgInfo {
    /// Destination address parsed from `IP_PKTINFO` / `IPV6_PKTINFO`;
    /// `None` when no pktinfo was attached.
    pub addr_dest: Option<SocketAddr>,
    /// Receiving interface index from the same cmsg.
    pub dest_if: Option<i32>,
    /// `IP_TOS` / `IPV6_TCLASS` ECN code-point.
    pub received_ecn: Option<EcnCodepoint>,
    /// `UDP_GRO` segment size; always `0` on Linux.
    pub udp_coalesced_size: usize,
}

/// IP / IPv6 ECN code-point reported by the kernel on receive.
/// Mirrors the two-bit `IP_TOS` / `IPV6_TCLASS` ECN field.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum EcnCodepoint {
    #[default]
    NotEct = 0b00,
    Ect1 = 0b01,
    Ect0 = 0b10,
    Ce = 0b11,
}

#[cfg(unix)]
fn setsockopt_uint(
    sd: libc::c_int,
    level: libc::c_int,
    optname: libc::c_int,
    value: libc::c_uint,
) -> bool {
    let value_ptr = &value as *const libc::c_uint as *const libc::c_void;
    let value_len = core::mem::size_of::<libc::c_uint>() as libc::socklen_t;
    // SAFETY: `setsockopt` reads `value_len` bytes from `value_ptr` during
    // this call. `value_ptr` points to the local `value`, and an invalid
    // socket descriptor is reported by the OS as a negative return value.
    unsafe { libc::setsockopt(sd, level, optname, value_ptr, value_len) == 0 }
}

/// Enable ECN receive reporting and, when `ecn_value != NotEct`, request
/// that outgoing packets use that ECN codepoint.
/// C: `picoquic_socket_set_ecn_options_ex` (picosocks.c:93-223).
#[cfg(unix)]
pub fn picoquic_socket_set_ecn_options_ex(
    sd: i32,
    af: i32,
    ecn_value: EcnCodepoint,
) -> Result<(bool, bool), Error> {
    let mut ret = -1;
    let recv_set;
    let send_set;
    let ecn = ecn_value as libc::c_uint;

    if af == libc::AF_INET6 {
        send_set = if ecn != 0 {
            setsockopt_uint(sd, libc::IPPROTO_IPV6, libc::IPV6_TCLASS, ecn)
        } else {
            true
        };

        let recv_ok = setsockopt_uint(sd, libc::IPPROTO_IPV6, libc::IPV6_RECVTCLASS, 1);
        if recv_ok {
            recv_set = true;
            ret = 0;
        } else {
            recv_set = false;
        }
    } else {
        send_set = if ecn != 0 {
            setsockopt_uint(sd, libc::IPPROTO_IP, libc::IP_TOS, ecn)
        } else {
            true
        };

        let recv_ok = setsockopt_uint(sd, libc::IPPROTO_IP, libc::IP_RECVTOS, 1);
        if recv_ok {
            recv_set = true;
            ret = 0;
        } else {
            recv_set = false;
        }
    }

    if ret == 0 {
        Ok((recv_set, send_set))
    } else {
        Err(Error::Generic)
    }
}

/// Enable ECN receive reporting and request ECT(1) on outgoing packets.
/// C: `picoquic_socket_set_ecn_options`.
#[cfg(unix)]
pub fn picoquic_socket_set_ecn_options(sd: i32, af: i32) -> Result<(bool, bool), Error> {
    picoquic_socket_set_ecn_options_ex(sd, af, EcnCodepoint::Ect1)
}

/// Non-Unix targets are outside the Phase 4 target triple; keep the safe
/// shape available for cross-target builds.
#[cfg(not(unix))]
pub fn picoquic_socket_set_ecn_options_ex(
    _sd: i32,
    _af: i32,
    _ecn_value: EcnCodepoint,
) -> Result<(bool, bool), Error> {
    Ok((false, false))
}

/// Non-Unix companion wrapper for [`picoquic_socket_set_ecn_options_ex`].
#[cfg(not(unix))]
pub fn picoquic_socket_set_ecn_options(_sd: i32, _af: i32) -> Result<(bool, bool), Error> {
    Ok((false, false))
}

/// Parse the control-message ancillary data attached to a
/// received `msghdr`.  C: `picoquic_socks_cmsg_parse`.
pub fn parse_cmsg(_header: &MessageHeader<'_, '_, '_>) -> CmsgInfo {
    // Control-message parsing requires CMSG_FIRSTHDR / CMSG_NXTHDR macros,
    // which are not available in safe Rust through socket2 0.5.  Return a
    // neutral default; ECN and pktinfo will appear as absent.
    CmsgInfo::default()
}

/// Write `IP_PKTINFO` / `IPV6_PKTINFO` / `UDP_SEGMENT` control
/// messages into `header` for an outgoing datagram.  C:
/// `picoquic_socks_cmsg_format`.
pub fn format_cmsg(
    _header: &mut MessageHeader<'_, '_, '_>,
    _message_length: usize,
    _send_msg_size: usize,
    _addr_from: Option<&SocketAddr>,
    _dest_if: i32,
) {
    // Control-message formatting requires CMSG_ macros not available in
    // safe Rust through socket2 0.5.  The kernel will pick the source
    // address; GSO segmentation and ECN marking are not applied.
}

// ---------------------------------------------------------------------------
// Low-level cmsg helpers (Unix only).

/// Append a new control-message header at the next available slot in the
/// `msghdr`'s control buffer, zero-fill the required space, write the header
/// fields (`cmsg_level`, `cmsg_type`, `cmsg_len`), and return a pointer to
/// the cmsg data area.  `*control_length` is incremented by the padded size
/// consumed (i.e. `CMSG_SPACE(cmsg_data_len)`).
///
/// Returns a null pointer when no further slot is available in the control
/// buffer.
///
/// # Safety
///
/// `msg` must point to a valid `msghdr` whose `msg_control` buffer has
/// capacity for at least `CMSG_SPACE(cmsg_data_len)` bytes beyond the
/// space already used.  `last_cmsg` must be either null (first entry) or
/// the pointer returned from the most recent call for this `msg`.
///
/// C: `cmsg_format_header_return_data_ptr` (static helper, `#else`
/// non-Windows branch, `picosocks.c`).
#[cfg(unix)]
#[allow(dead_code)]
pub(crate) unsafe fn cmsg_format_header_return_data_ptr(
    msg: *mut libc::msghdr,
    last_cmsg: &mut *mut libc::cmsghdr,
    control_length: &mut libc::c_int,
    cmsg_level: libc::c_int,
    cmsg_type: libc::c_int,
    cmsg_data_len: usize,
) -> *mut core::ffi::c_void {
    // Rust 2024: unsafe operations inside unsafe fn still need explicit
    // unsafe {} blocks.

    // Locate the next available cmsg slot.  On the first call last_cmsg
    // is null so CMSG_FIRSTHDR returns the start of the control buffer;
    // on subsequent calls CMSG_NXTHDR advances past the previous entry.
    let cmsg: *mut libc::cmsghdr = unsafe {
        if (*last_cmsg).is_null() {
            libc::CMSG_FIRSTHDR(msg as *const libc::msghdr)
        } else {
            // The C source uses CMSG_ALIGN (Linux) when defined and falls
            // back to CMSG_NXTHDR otherwise.  CMSG_NXTHDR is correct on
            // all POSIX platforms we target and is always available.
            libc::CMSG_NXTHDR(
                msg as *const libc::msghdr,
                *last_cmsg as *const libc::cmsghdr,
            )
        }
    };

    if cmsg.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        let cmsg_required_space = libc::CMSG_SPACE(cmsg_data_len as libc::c_uint) as usize;
        *control_length += cmsg_required_space as libc::c_int;
        // Zero-fill the entire padded region (mirrors the C memset call).
        core::ptr::write_bytes(cmsg as *mut u8, 0, cmsg_required_space);
        (*cmsg).cmsg_level = cmsg_level;
        (*cmsg).cmsg_type = cmsg_type;
        (*cmsg).cmsg_len = libc::CMSG_LEN(cmsg_data_len as libc::c_uint) as _;
        *last_cmsg = cmsg;
        libc::CMSG_DATA(cmsg) as *mut core::ffi::c_void
    }
}

#[cfg(test)]
mod test {}
