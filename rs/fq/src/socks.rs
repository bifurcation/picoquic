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

use core::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

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

#[cfg(unix)]
const UNREACHABLE_SOCKET_ERRORS: [i32; 6] = [
    libc::EAFNOSUPPORT,
    libc::ECONNRESET,
    libc::EHOSTUNREACH,
    libc::ENETDOWN,
    libc::ENETUNREACH,
    -1,
];

#[cfg(windows)]
const UNREACHABLE_SOCKET_ERRORS: [i32; 11] = [
    10013, // WSAEACCES
    10049, // WSAEADDRNOTAVAIL
    10047, // WSAEAFNOSUPPORT
    10054, // WSAECONNRESET
    10039, // WSAEDESTADDRREQ
    10065, // WSAEHOSTUNREACH
    10050, // WSAENETDOWN
    10052, // WSAENETRESET
    10051, // WSAENETUNREACH
    10058, // WSAESHUTDOWN
    -1,
];

impl OsError {
    /// Whether this errno implies the destination is unreachable
    /// and the owning path should be abandoned.  C:
    /// `picoquic_socket_error_implies_unreachable`.
    pub fn is_unreachable(self) -> bool {
        #[cfg(any(unix, windows))]
        {
            UNREACHABLE_SOCKET_ERRORS.contains(&self.0)
        }

        #[cfg(not(any(unix, windows)))]
        {
            false
        }
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

/// C: `picoquic_send_through_socket` (picoquic/picosocks.c:1262)
///
/// Send one datagram through a single socket, with no GSO segmentation hint.
/// The OS error out-parameter is represented by `sock_err`.
pub fn picoquic_send_through_socket<S: Socket>(
    socket: &mut S,
    addr_dest: &SocketAddr,
    addr_from: Option<&SocketAddr>,
    from_if: i32,
    bytes: &[u8],
    sock_err: &mut Option<OsError>,
) -> Result<usize, OsError> {
    match socket.send(addr_dest, addr_from, from_if, bytes, 0) {
        Ok(sent) => {
            *sock_err = None;
            Ok(sent)
        }
        Err(err) => {
            *sock_err = Some(err);
            Err(err)
        }
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

#[cfg(unix)]
#[derive(Debug, Default, Copy, Clone)]
struct RecvAncillaryInfo {
    addr_dest: Option<SocketAddr>,
    dest_if: Option<i32>,
    received_ecn: Option<u8>,
}

#[cfg(unix)]
#[repr(C, align(8))]
struct ControlBuffer([u8; 1024]);

#[cfg(unix)]
fn ipv4_addr_from_in_addr(addr: libc::in_addr) -> Ipv4Addr {
    Ipv4Addr::from(addr.s_addr.to_ne_bytes())
}

#[cfg(unix)]
fn ipv6_addr_from_in6_addr(addr: libc::in6_addr) -> Ipv6Addr {
    Ipv6Addr::from(addr.s6_addr)
}

#[cfg(unix)]
fn socket_addr_from_sockaddr_storage(
    storage: &libc::sockaddr_storage,
    len: libc::socklen_t,
) -> Option<SocketAddr> {
    match storage.ss_family as libc::c_int {
        libc::AF_INET if len as usize >= core::mem::size_of::<libc::sockaddr_in>() => {
            // SAFETY: `storage` was filled by the kernel as an AF_INET
            // sockaddr, and `len` proves the sockaddr_in prefix is present.
            let sin = unsafe {
                core::ptr::read_unaligned(storage as *const _ as *const libc::sockaddr_in)
            };
            Some(SocketAddr::V4(SocketAddrV4::new(
                ipv4_addr_from_in_addr(sin.sin_addr),
                u16::from_be(sin.sin_port),
            )))
        }
        libc::AF_INET6 if len as usize >= core::mem::size_of::<libc::sockaddr_in6>() => {
            // SAFETY: `storage` was filled by the kernel as an AF_INET6
            // sockaddr, and `len` proves the sockaddr_in6 prefix is present.
            let sin6 = unsafe {
                core::ptr::read_unaligned(storage as *const _ as *const libc::sockaddr_in6)
            };
            Some(SocketAddr::V6(SocketAddrV6::new(
                ipv6_addr_from_in6_addr(sin6.sin6_addr),
                u16::from_be(sin6.sin6_port),
                sin6.sin6_flowinfo,
                sin6.sin6_scope_id,
            )))
        }
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn ipv4_addr_to_in_addr(addr: &Ipv4Addr) -> libc::in_addr {
    libc::in_addr {
        s_addr: u32::from_ne_bytes(addr.octets()),
    }
}

#[cfg(target_os = "linux")]
fn ipv6_addr_to_in6_addr(addr: &Ipv6Addr) -> libc::in6_addr {
    libc::in6_addr {
        s6_addr: addr.octets(),
    }
}

#[cfg(target_os = "linux")]
fn socket_addr_to_sockaddr_storage(addr: &SocketAddr) -> (libc::sockaddr_storage, libc::socklen_t) {
    // SAFETY: all-zero is a valid starting byte pattern for sockaddr_storage;
    // the address-family-specific sockaddr is written over the prefix below.
    let mut storage: libc::sockaddr_storage = unsafe { core::mem::zeroed() };

    match addr {
        SocketAddr::V4(addr4) => {
            // SAFETY: zero is a valid baseline for sockaddr_in; all fields
            // relevant to sendmsg are initialized immediately below.
            let mut sin: libc::sockaddr_in = unsafe { core::mem::zeroed() };
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "freebsd",
                target_os = "openbsd",
                target_os = "netbsd",
                target_os = "dragonfly"
            ))]
            {
                sin.sin_len = core::mem::size_of::<libc::sockaddr_in>() as u8;
            }
            sin.sin_family = libc::AF_INET as libc::sa_family_t;
            sin.sin_port = addr4.port().to_be();
            sin.sin_addr = ipv4_addr_to_in_addr(addr4.ip());
            // SAFETY: sockaddr_storage is large enough for sockaddr_in.  The
            // write only initializes the sockaddr_in prefix used by sendmsg.
            unsafe {
                core::ptr::write_unaligned(
                    (&mut storage as *mut libc::sockaddr_storage).cast::<libc::sockaddr_in>(),
                    sin,
                );
            }
            (
                storage,
                core::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
            )
        }
        SocketAddr::V6(addr6) => {
            // SAFETY: zero is a valid baseline for sockaddr_in6; all fields
            // relevant to sendmsg are initialized immediately below.
            let mut sin6: libc::sockaddr_in6 = unsafe { core::mem::zeroed() };
            #[cfg(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "freebsd",
                target_os = "openbsd",
                target_os = "netbsd",
                target_os = "dragonfly"
            ))]
            {
                sin6.sin6_len = core::mem::size_of::<libc::sockaddr_in6>() as u8;
            }
            sin6.sin6_family = libc::AF_INET6 as libc::sa_family_t;
            sin6.sin6_port = addr6.port().to_be();
            sin6.sin6_flowinfo = addr6.flowinfo();
            sin6.sin6_addr = ipv6_addr_to_in6_addr(addr6.ip());
            sin6.sin6_scope_id = addr6.scope_id();
            // SAFETY: sockaddr_storage is large enough for sockaddr_in6.  The
            // write only initializes the sockaddr_in6 prefix used by sendmsg.
            unsafe {
                core::ptr::write_unaligned(
                    (&mut storage as *mut libc::sockaddr_storage).cast::<libc::sockaddr_in6>(),
                    sin6,
                );
            }
            (
                storage,
                core::mem::size_of::<libc::sockaddr_in6>() as libc::socklen_t,
            )
        }
    }
}

#[cfg(unix)]
fn cmsg_has_data(cmsg: *const libc::cmsghdr, data_len: usize) -> bool {
    // SAFETY: `cmsg` comes from CMSG_FIRSTHDR / CMSG_NXTHDR for the live
    // msghdr. Reading the header length is valid for those pointers.
    // `cmsg_len`'s native type varies by platform (`usize` on Linux,
    // `socklen_t` on macOS), so the cast is needed on some targets and
    // a no-op on others.
    #[allow(clippy::unnecessary_cast)]
    let cmsg_len = unsafe { (*cmsg).cmsg_len as usize };
    // SAFETY: `data_len` is the payload length being checked; CMSG_LEN only
    // computes the platform header-plus-payload size.
    let required_len = unsafe { libc::CMSG_LEN(data_len as libc::c_uint) as usize };
    cmsg_len >= required_len
}

#[cfg(unix)]
unsafe fn cmsg_data_as<T: Copy>(cmsg: *const libc::cmsghdr) -> Option<T> {
    if !cmsg_has_data(cmsg, core::mem::size_of::<T>()) {
        return None;
    }

    // SAFETY: the caller supplied a cmsg pointer produced by the CMSG macros,
    // and `cmsg_has_data` verified that the payload contains a full `T`.
    let data = unsafe { libc::CMSG_DATA(cmsg as *mut libc::cmsghdr) as *const T };
    // SAFETY: ancillary payload alignment is platform-defined; use
    // read_unaligned so the Rust side does not assume stronger alignment.
    Some(unsafe { core::ptr::read_unaligned(data) })
}

#[cfg(unix)]
unsafe fn parse_ipv4_cmsg(cmsg: *const libc::cmsghdr, info: &mut RecvAncillaryInfo) -> bool {
    // SAFETY: `cmsg` comes from CMSG_FIRSTHDR / CMSG_NXTHDR.
    let cmsg_type = unsafe { (*cmsg).cmsg_type };

    #[cfg(any(target_os = "linux", target_os = "android"))]
    if cmsg_type == libc::IP_PKTINFO {
        // SAFETY: `cmsg` is a valid IPv4 pktinfo control message.
        if let Some(pktinfo) = unsafe { cmsg_data_as::<libc::in_pktinfo>(cmsg) } {
            info.addr_dest = Some(SocketAddr::V4(SocketAddrV4::new(
                ipv4_addr_from_in_addr(pktinfo.ipi_addr),
                0,
            )));
            // `ipi_ifindex`'s native type varies by libc release/platform
            // (`u32` on some, `i32` on others); the cast is needed where
            // they disagree and a no-op where they don't.
            #[allow(clippy::unnecessary_cast)]
            let if_index = pktinfo.ipi_ifindex as i32;
            info.dest_if = Some(if_index);
        }
        return true;
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    if cmsg_type == libc::IP_RECVDSTADDR {
        // SAFETY: `cmsg` is a valid IPv4 destination-address control message.
        if let Some(addr) = unsafe { cmsg_data_as::<libc::in_addr>(cmsg) } {
            info.addr_dest = Some(SocketAddr::V4(SocketAddrV4::new(
                ipv4_addr_from_in_addr(addr),
                0,
            )));
            info.dest_if = Some(0);
        }
        return true;
    }

    if cmsg_type == libc::IP_TOS || cmsg_type == libc::IP_RECVTOS {
        // SAFETY: `cmsg` is an IPv4 TOS/ECN control message.
        if let Some(ecn) = unsafe { cmsg_data_as::<u8>(cmsg) } {
            info.received_ecn = Some(ecn);
        }
        return true;
    }

    false
}

#[cfg(unix)]
unsafe fn parse_ipv6_cmsg(cmsg: *const libc::cmsghdr, info: &mut RecvAncillaryInfo) -> bool {
    // SAFETY: `cmsg` comes from CMSG_FIRSTHDR / CMSG_NXTHDR.
    let cmsg_type = unsafe { (*cmsg).cmsg_type };

    if cmsg_type == libc::IPV6_PKTINFO {
        // SAFETY: `cmsg` is a valid IPv6 pktinfo control message.
        if let Some(pktinfo) = unsafe { cmsg_data_as::<libc::in6_pktinfo>(cmsg) } {
            info.addr_dest = Some(SocketAddr::V6(SocketAddrV6::new(
                ipv6_addr_from_in6_addr(pktinfo.ipi6_addr),
                0,
                0,
                0,
            )));
            info.dest_if = Some(pktinfo.ipi6_ifindex as i32);
        }
        return true;
    }

    if cmsg_type == libc::IPV6_TCLASS {
        // SAFETY: `cmsg` is an IPv6 traffic-class/ECN control message.
        if let Some(ecn) = unsafe { cmsg_data_as::<u8>(cmsg) } {
            info.received_ecn = Some(ecn);
        }
        return true;
    }

    false
}

#[cfg(unix)]
fn parse_recv_cmsgs(msg: &libc::msghdr) -> RecvAncillaryInfo {
    let mut info = RecvAncillaryInfo::default();

    // SAFETY: `msg` is the initialized msghdr just returned by recvmsg.
    let mut cmsg = unsafe { libc::CMSG_FIRSTHDR(msg as *const libc::msghdr) };
    while !cmsg.is_null() {
        // SAFETY: `cmsg` is produced by the CMSG iteration macros for `msg`.
        let level = unsafe { (*cmsg).cmsg_level };
        match level {
            libc::IPPROTO_IP => {
                // SAFETY: `cmsg` points at an IPv4-level ancillary header.
                let _ = unsafe { parse_ipv4_cmsg(cmsg, &mut info) };
            }
            libc::IPPROTO_IPV6 => {
                // SAFETY: `cmsg` points at an IPv6-level ancillary header.
                let _ = unsafe { parse_ipv6_cmsg(cmsg, &mut info) };
            }
            _ => {}
        }

        // SAFETY: `cmsg` is the current header for `msg`; CMSG_NXTHDR either
        // returns the next valid header or null at the end of the control data.
        cmsg = unsafe { libc::CMSG_NXTHDR(msg as *const libc::msghdr, cmsg as *const _) };
    }

    info
}

/// Receive one datagram with POSIX `recvmsg`, including source address,
/// destination pktinfo, interface index, and ECN ancillary data.
/// C: `picoquic_recvmsg` (non-Windows branch).
#[cfg(unix)]
pub(crate) fn picoquic_recvmsg(sd: i32, buffer: &mut [u8]) -> Result<RecvInfo, Error> {
    // SAFETY: all-zero is a valid initial value for POSIX sockaddr_storage and
    // msghdr before the kernel fills their fields.
    let mut addr_from: libc::sockaddr_storage = unsafe { core::mem::zeroed() };
    let mut msg: libc::msghdr = unsafe { core::mem::zeroed() };
    let mut data_buf = libc::iovec {
        iov_base: buffer.as_mut_ptr() as *mut libc::c_void,
        iov_len: buffer.len(),
    };
    let mut cmsg_buffer = ControlBuffer([0u8; 1024]);

    msg.msg_name = (&mut addr_from as *mut libc::sockaddr_storage).cast();
    msg.msg_namelen = core::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    msg.msg_iov = &mut data_buf;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buffer.0.as_mut_ptr().cast();
    msg.msg_controllen = cmsg_buffer.0.len() as _;
    msg.msg_flags = 0;

    // SAFETY: `msg` points to live stack storage; its name, iovec, and control
    // buffers all remain valid and writable for the duration of the syscall.
    let bytes_recv = unsafe { libc::recvmsg(sd, &mut msg, 0) };
    if bytes_recv < 0 {
        return Err(Error::Generic);
    }
    if bytes_recv == 0 {
        return Ok(RecvInfo::default());
    }

    let cmsg_info = parse_recv_cmsgs(&msg);
    Ok(RecvInfo {
        addr_from: socket_addr_from_sockaddr_storage(&addr_from, msg.msg_namelen),
        addr_dest: cmsg_info.addr_dest,
        dest_if: cmsg_info.dest_if.unwrap_or(0),
        received_ecn: cmsg_info.received_ecn.unwrap_or(0),
        bytes_recv: bytes_recv as usize,
    })
}

#[cfg(target_os = "linux")]
fn write_send_cmsg<T: Copy>(
    msg: &mut libc::msghdr,
    last_cmsg: &mut *mut libc::cmsghdr,
    control_length: &mut libc::c_int,
    cmsg_level: libc::c_int,
    cmsg_type: libc::c_int,
    value: &T,
) -> bool {
    // SAFETY: `msg` owns a live control buffer.  The helper returns a pointer
    // inside that buffer or null if there is no room for this cmsg.
    let data = unsafe {
        cmsg_format_header_return_data_ptr(
            msg,
            last_cmsg,
            control_length,
            cmsg_level,
            cmsg_type,
            core::mem::size_of::<T>(),
        )
    };
    if data.is_null() {
        return false;
    }

    // SAFETY: the helper reserved at least size_of::<T>() bytes for this cmsg
    // payload. Ancillary data alignment is platform-defined, so write without
    // imposing a stronger Rust alignment requirement.
    unsafe {
        core::ptr::write_unaligned(data.cast::<T>(), *value);
    }
    true
}

#[cfg(target_os = "linux")]
fn write_ipv4_pktinfo_cmsg(
    msg: &mut libc::msghdr,
    last_cmsg: &mut *mut libc::cmsghdr,
    control_length: &mut libc::c_int,
    addr_from: &SocketAddrV4,
    dest_if: i32,
) -> bool {
    let pktinfo = libc::in_pktinfo {
        ipi_ifindex: dest_if as libc::c_int,
        ipi_spec_dst: ipv4_addr_to_in_addr(addr_from.ip()),
        ipi_addr: libc::in_addr { s_addr: 0 },
    };
    write_send_cmsg(
        msg,
        last_cmsg,
        control_length,
        libc::IPPROTO_IP,
        libc::IP_PKTINFO,
        &pktinfo,
    )
}

#[cfg(target_os = "linux")]
fn write_ipv6_pktinfo_cmsg(
    msg: &mut libc::msghdr,
    last_cmsg: &mut *mut libc::cmsghdr,
    control_length: &mut libc::c_int,
    addr_from: &SocketAddrV6,
    dest_if: i32,
) -> bool {
    let pktinfo = libc::in6_pktinfo {
        ipi6_addr: ipv6_addr_to_in6_addr(addr_from.ip()),
        ipi6_ifindex: dest_if as libc::c_uint,
    };
    write_send_cmsg(
        msg,
        last_cmsg,
        control_length,
        libc::IPPROTO_IPV6,
        libc::IPV6_PKTINFO,
        &pktinfo,
    )
}

#[cfg(target_os = "linux")]
fn format_send_cmsgs(
    msg: &mut libc::msghdr,
    message_length: usize,
    send_msg_size: usize,
    addr_from: Option<&SocketAddr>,
    dest_if: i32,
) {
    let mut control_length: libc::c_int = 0;

    #[cfg(target_os = "linux")]
    {
        let mut last_cmsg: *mut libc::cmsghdr = core::ptr::null_mut();
        let mut is_null = false;

        if let Some(addr_from) = addr_from {
            match addr_from {
                SocketAddr::V4(addr4) => {
                    if !write_ipv4_pktinfo_cmsg(
                        msg,
                        &mut last_cmsg,
                        &mut control_length,
                        addr4,
                        dest_if,
                    ) {
                        is_null = true;
                    }
                }
                SocketAddr::V6(addr6) => {
                    if !write_ipv6_pktinfo_cmsg(
                        msg,
                        &mut last_cmsg,
                        &mut control_length,
                        addr6,
                        dest_if,
                    ) {
                        is_null = true;
                    }

                    if !is_null {
                        let dontfrag: libc::c_int = 1;
                        if !write_send_cmsg(
                            msg,
                            &mut last_cmsg,
                            &mut control_length,
                            libc::SOL_IPV6,
                            libc::IPV6_DONTFRAG,
                            &dontfrag,
                        ) {
                            is_null = true;
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        if !is_null && send_msg_size > 0 && send_msg_size < message_length {
            let segment_size = send_msg_size as u16;
            if !write_send_cmsg(
                msg,
                &mut last_cmsg,
                &mut control_length,
                libc::SOL_UDP,
                libc::UDP_SEGMENT,
                &segment_size,
            ) {
                is_null = true;
            }
        }

        let _ = is_null;
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (addr_from, dest_if);

    let _ = (message_length, send_msg_size);
    msg.msg_controllen = control_length as _;
    if control_length == 0 {
        msg.msg_control = core::ptr::null_mut();
    }
}

/// Send one UDP datagram with POSIX `sendmsg`, including source-address /
/// interface pktinfo and Linux `UDP_SEGMENT` when `send_msg_size` requests
/// segmentation. C: `picoquic_sendmsg` (non-Windows branch).
#[cfg(target_os = "linux")]
pub(crate) fn picoquic_sendmsg(
    sd: i32,
    addr_dest: &SocketAddr,
    addr_from: Option<&SocketAddr>,
    dest_if: i32,
    bytes: &[u8],
    send_msg_size: i32,
) -> Result<usize, OsError> {
    let (mut addr_dest_storage, addr_dest_len) = socket_addr_to_sockaddr_storage(addr_dest);
    // SAFETY: all-zero is a valid initial value for POSIX msghdr before fields
    // are populated below.
    let mut msg: libc::msghdr = unsafe { core::mem::zeroed() };
    let mut data_buf = libc::iovec {
        iov_base: bytes.as_ptr() as *mut libc::c_void,
        iov_len: bytes.len(),
    };
    let mut cmsg_buffer = ControlBuffer([0u8; 1024]);
    let send_msg_size = usize::try_from(send_msg_size).unwrap_or(0);

    msg.msg_name = (&mut addr_dest_storage as *mut libc::sockaddr_storage).cast();
    msg.msg_namelen = addr_dest_len;
    msg.msg_iov = &mut data_buf;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buffer.0.as_mut_ptr().cast();
    msg.msg_controllen = cmsg_buffer.0.len() as _;
    msg.msg_flags = 0;

    format_send_cmsgs(&mut msg, bytes.len(), send_msg_size, addr_from, dest_if);

    // SAFETY: `msg` points to live stack storage; its destination sockaddr,
    // iovec, and control buffer remain valid for the duration of the syscall.
    let bytes_sent = unsafe { libc::sendmsg(sd, &msg, 0) };
    if bytes_sent <= 0 {
        let raw_error = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
        Err(OsError(raw_error))
    } else {
        Ok(bytes_sent as usize)
    }
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
    pub bytes_recv: isize,
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
    delta_t: i64,
) -> Result<SelectInfo, Error> {
    select_impl(sockets, buffer, delta_t)
}

fn select_current_time() -> Instant {
    Instant::from_ticks(crate::current_time())
}

#[cfg(unix)]
fn select_timeval(delta_t: i64) -> libc::timeval {
    let delta_t = if delta_t <= 0 {
        0
    } else {
        delta_t.min(10_000_000)
    };

    libc::timeval {
        tv_sec: (delta_t / 1_000_000) as libc::time_t,
        tv_usec: (delta_t % 1_000_000) as libc::suseconds_t,
    }
}

#[cfg(unix)]
fn select_impl<S: Socket>(
    sockets: &mut [S],
    buffer: &mut [u8],
    delta_t: i64,
) -> Result<SelectInfo, Error> {
    // SAFETY: all-zero is a valid fd_set initialization; FD_ZERO is called
    // immediately below before the set is passed to select(2).
    let mut readfds: libc::fd_set = unsafe { core::mem::zeroed() };
    let mut tv = select_timeval(delta_t);
    let mut sockmax: libc::c_int = 0;
    let mut socket_fds = Vec::with_capacity(sockets.len());

    // SAFETY: `readfds` points to live storage initialized for the platform's
    // fd_set representation.
    unsafe {
        libc::FD_ZERO(&mut readfds);
    }

    for socket in sockets.iter() {
        let fd = socket.raw_fd();
        if fd < 0 {
            return Ok(SelectInfo {
                bytes_recv: -1,
                current_time: select_current_time(),
                ..SelectInfo::default()
            });
        }
        if sockmax < fd {
            sockmax = fd;
        }
        socket_fds.push(fd);

        // SAFETY: `fd` is the raw descriptor exposed by the socket backend,
        // and `readfds` is the fd_set being prepared for select(2).
        unsafe {
            libc::FD_SET(fd, &mut readfds);
        }
    }

    // SAFETY: all pointers either reference initialized local fd/timeval
    // storage or are null for the write/except sets that the C code omits.
    let ret_select = unsafe {
        libc::select(
            sockmax + 1,
            &mut readfds,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            &mut tv,
        )
    };

    if ret_select < 0 {
        return Ok(SelectInfo {
            bytes_recv: -1,
            current_time: select_current_time(),
            ..SelectInfo::default()
        });
    }

    if ret_select == 0 {
        return Ok(SelectInfo {
            current_time: select_current_time(),
            ..SelectInfo::default()
        });
    }

    for (rank, socket) in sockets.iter_mut().enumerate() {
        // SAFETY: `readfds` was returned by select(2), and each descriptor was
        // inserted into that same set before the call.
        let is_ready = unsafe { libc::FD_ISSET(socket_fds[rank], &readfds) };
        if !is_ready {
            continue;
        }

        return match socket.recv(buffer) {
            Ok(info) => Ok(SelectInfo {
                addr_from: info.addr_from,
                addr_dest: info.addr_dest,
                dest_if: info.dest_if,
                received_ecn: info.received_ecn,
                bytes_recv: info.bytes_recv as isize,
                current_time: select_current_time(),
                socket_rank: rank,
            }),
            Err(_) => Ok(SelectInfo {
                bytes_recv: -1,
                current_time: select_current_time(),
                socket_rank: rank,
                ..SelectInfo::default()
            }),
        };
    }

    Ok(SelectInfo {
        current_time: select_current_time(),
        ..SelectInfo::default()
    })
}

#[cfg(not(unix))]
fn select_impl<S: Socket>(
    sockets: &mut [S],
    buffer: &mut [u8],
    delta_t: i64,
) -> Result<SelectInfo, Error> {
    if delta_t > 0 {
        let timeout = core::time::Duration::from_micros(delta_t.min(10_000_000) as u64);
        std::thread::sleep(timeout);
    }

    for (rank, socket) in sockets.iter_mut().enumerate() {
        match socket.recv(buffer) {
            Ok(info) => {
                return Ok(SelectInfo {
                    addr_from: info.addr_from,
                    addr_dest: info.addr_dest,
                    dest_if: info.dest_if,
                    received_ecn: info.received_ecn,
                    bytes_recv: info.bytes_recv as isize,
                    current_time: select_current_time(),
                    socket_rank: rank,
                });
            }
            Err(_) => {
                return Ok(SelectInfo {
                    bytes_recv: -1,
                    current_time: select_current_time(),
                    socket_rank: rank,
                    ..SelectInfo::default()
                });
            }
        }
    }

    Ok(SelectInfo {
        current_time: select_current_time(),
        ..SelectInfo::default()
    })
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

fn ecn_codepoint_from_byte(value: u8) -> EcnCodepoint {
    match value & 0x03 {
        1 => EcnCodepoint::Ect1,
        2 => EcnCodepoint::Ect0,
        3 => EcnCodepoint::Ce,
        _ => EcnCodepoint::NotEct,
    }
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

#[cfg(unix)]
fn setsockopt_int(
    sd: libc::c_int,
    level: libc::c_int,
    optname: libc::c_int,
    value: libc::c_int,
) -> Result<(), Error> {
    let value_ptr = &value as *const libc::c_int as *const libc::c_void;
    let value_len = core::mem::size_of::<libc::c_int>() as libc::socklen_t;
    // SAFETY: `setsockopt` reads `value_len` bytes from `value_ptr` during
    // this call. `value_ptr` points to the local `value`, and any OS-level
    // failure is reported by the negative syscall result.
    let ret = unsafe { libc::setsockopt(sd, level, optname, value_ptr, value_len) };
    if ret == 0 {
        Ok(())
    } else {
        Err(Error::Generic)
    }
}

/// Enable per-packet destination-info delivery on a UDP socket.
/// C: `picoquic_socket_set_pkt_info` (picosocks.c:58-91).
#[cfg(unix)]
pub fn picoquic_socket_set_pkt_info(sd: i32, af: i32) -> Result<(), Error> {
    let val: libc::c_int = 1;

    if af == libc::AF_INET6 {
        setsockopt_int(sd, libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, val)?;
        setsockopt_int(sd, libc::IPPROTO_IPV6, libc::IPV6_RECVPKTINFO, val)
    } else {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        {
            setsockopt_int(sd, libc::IPPROTO_IP, libc::IP_PKTINFO, val)
        }

        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        {
            setsockopt_int(sd, libc::IPPROTO_IP, libc::IP_RECVDSTADDR, val)
        }
    }
}

/// Non-Unix companion wrapper for [`picoquic_socket_set_pkt_info`].
#[cfg(not(unix))]
pub fn picoquic_socket_set_pkt_info(_sd: i32, _af: i32) -> Result<(), Error> {
    Ok(())
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

/// Enable Path-MTU-Discovery probing on a UDP socket.
/// C: `picoquic_socket_set_pmtud_options` (picosocks.c:230-248).
#[cfg(target_os = "linux")]
pub fn picoquic_socket_set_pmtud_options(sd: i32, af: i32) -> Result<(), Error> {
    let val: libc::c_int = libc::IP_PMTUDISC_PROBE;

    if af == libc::AF_INET6 {
        setsockopt_int(sd, libc::IPPROTO_IPV6, libc::IPV6_MTU_DISCOVER, val)
    } else {
        setsockopt_int(sd, libc::IPPROTO_IP, libc::IP_MTU_DISCOVER, val)
    }
}

/// Non-Linux targets match the C build's `#ifdef __linux` no-op path.
#[cfg(not(target_os = "linux"))]
pub fn picoquic_socket_set_pmtud_options(_sd: i32, _af: i32) -> Result<(), Error> {
    Ok(())
}

/// Parse the control-message ancillary data attached to a
/// received `msghdr`.  C: `picoquic_socks_cmsg_parse`.
pub fn parse_cmsg(header: &MessageHeader<'_, '_, '_>) -> CmsgInfo {
    #[cfg(unix)]
    {
        debug_assert_eq!(
            core::mem::size_of_val(header),
            core::mem::size_of::<libc::msghdr>()
        );

        // SAFETY: socket2 0.5 `MsgHdr` wraps the platform `msghdr` as its
        // only non-zero-sized field on Unix. `parse_cmsg` only reads header
        // fields and ancillary bytes that the caller provided through that
        // wrapper, matching the C helper's "already filled by recvmsg"
        // contract.
        let msg = unsafe { &*(header as *const _ as *const libc::msghdr) };
        let parsed = parse_recv_cmsgs(msg);
        CmsgInfo {
            addr_dest: parsed.addr_dest,
            dest_if: parsed.dest_if,
            received_ecn: parsed.received_ecn.map(ecn_codepoint_from_byte),
            udp_coalesced_size: 0,
        }
    }

    #[cfg(not(unix))]
    {
        let _ = header;
        CmsgInfo::default()
    }
}

/// Write `IP_PKTINFO` / `IPV6_PKTINFO` / `UDP_SEGMENT` control
/// messages into `header` for an outgoing datagram.  C:
/// `picoquic_socks_cmsg_format`.
pub fn format_cmsg(
    header: &mut MessageHeader<'_, '_, '_>,
    message_length: usize,
    send_msg_size: usize,
    addr_from: Option<&SocketAddr>,
    dest_if: i32,
) {
    #[cfg(target_os = "linux")]
    {
        debug_assert_eq!(
            core::mem::size_of_val(header),
            core::mem::size_of::<libc::msghdr>()
        );

        // SAFETY: socket2 0.5 `MsgHdr` wraps the platform `msghdr` as its
        // only non-zero-sized field on Unix. The caller provides a header
        // whose control buffer is the outgoing ancillary-data scratch space;
        // `format_send_cmsgs` only writes cmsg records inside that buffer and
        // then shrinks `msg_controllen` to the used length, matching the C
        // helper's contract.
        let msg = unsafe { &mut *(header as *mut _ as *mut libc::msghdr) };
        format_send_cmsgs(msg, message_length, send_msg_size, addr_from, dest_if);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (header, message_length, send_msg_size, addr_from, dest_if);
    }
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
mod test {
    use super::{EcnCodepoint, MessageHeader, OsError, RecvInfo, Socket};
    use crate::Error;
    use core::net::{Ipv6Addr, SocketAddr, SocketAddrV6};

    #[cfg(unix)]
    struct FdGuard(libc::c_int);

    #[cfg(unix)]
    impl Drop for FdGuard {
        fn drop(&mut self) {
            if self.0 >= 0 {
                // SAFETY: the guard owns this file descriptor.
                unsafe {
                    libc::close(self.0);
                }
                self.0 = -1;
            }
        }
    }

    #[cfg(unix)]
    struct TestSocket {
        fd: libc::c_int,
        recv_calls: usize,
        recv_error: bool,
    }

    #[cfg(unix)]
    impl Drop for TestSocket {
        fn drop(&mut self) {
            if self.fd >= 0 {
                // SAFETY: the test socket owns this file descriptor.
                unsafe {
                    libc::close(self.fd);
                }
                self.fd = -1;
            }
        }
    }

    #[cfg(unix)]
    impl Socket for TestSocket {
        fn local_address(&self) -> Result<SocketAddr, Error> {
            Err(Error::Generic)
        }

        fn recv(&mut self, buffer: &mut [u8]) -> Result<RecvInfo, Error> {
            self.recv_calls += 1;
            if self.recv_error {
                return Err(Error::Generic);
            }

            // SAFETY: `self.fd` is a live pipe read end owned by this test
            // socket, and `buffer` is writable for `buffer.len()` bytes.
            let ret = unsafe {
                libc::read(
                    self.fd,
                    buffer.as_mut_ptr().cast::<libc::c_void>(),
                    buffer.len(),
                )
            };
            if ret < 0 {
                Err(Error::Generic)
            } else {
                Ok(RecvInfo {
                    bytes_recv: ret as usize,
                    ..RecvInfo::default()
                })
            }
        }

        fn send(
            &mut self,
            _addr_dest: &SocketAddr,
            _addr_from: Option<&SocketAddr>,
            _dest_if: i32,
            _bytes: &[u8],
            _gso_size: i32,
        ) -> Result<usize, OsError> {
            Err(OsError(-1))
        }

        fn raw_fd(&self) -> i32 {
            self.fd
        }
    }

    #[cfg(unix)]
    fn pipe_socket() -> (TestSocket, FdGuard) {
        let mut fds = [0; 2];
        // SAFETY: `fds` points to two writable ints for pipe(2) to fill.
        let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(ret, 0);
        (
            TestSocket {
                fd: fds[0],
                recv_calls: 0,
                recv_error: false,
            },
            FdGuard(fds[1]),
        )
    }

    #[cfg(unix)]
    fn write_fd(fd: libc::c_int, bytes: &[u8]) {
        // SAFETY: `fd` is a live pipe write end and `bytes` is readable for
        // `bytes.len()` bytes.
        let ret = unsafe { libc::write(fd, bytes.as_ptr().cast::<libc::c_void>(), bytes.len()) };
        assert_eq!(ret, bytes.len() as isize);
    }

    #[cfg(unix)]
    fn append_test_cmsg<T: Copy>(
        msg: &mut libc::msghdr,
        last_cmsg: &mut *mut libc::cmsghdr,
        control_length: &mut libc::c_int,
        cmsg_level: libc::c_int,
        cmsg_type: libc::c_int,
        value: T,
    ) {
        // SAFETY: the test msghdr owns a live control buffer with enough room
        // for all cmsgs appended in this test.
        let data = unsafe {
            super::cmsg_format_header_return_data_ptr(
                msg,
                last_cmsg,
                control_length,
                cmsg_level,
                cmsg_type,
                core::mem::size_of::<T>(),
            )
        };
        assert!(!data.is_null());

        // SAFETY: the cmsg helper reserved size_of::<T>() bytes at `data`.
        unsafe {
            core::ptr::write_unaligned(data.cast::<T>(), value);
        }
    }

    #[cfg(target_os = "linux")]
    fn send_msg_with_control(cmsg_buffer: &mut super::ControlBuffer) -> libc::msghdr {
        // SAFETY: all-zero is a valid initial POSIX msghdr value before the
        // test fills the control buffer fields used by cmsg formatting.
        let mut msg: libc::msghdr = unsafe { core::mem::zeroed() };
        msg.msg_control = cmsg_buffer.0.as_mut_ptr().cast();
        msg.msg_controllen = cmsg_buffer.0.len() as _;
        msg
    }

    #[cfg(target_os = "linux")]
    fn header_from_msghdr(msg: &mut libc::msghdr) -> &mut MessageHeader<'_, '_, '_> {
        // SAFETY: socket2 0.5 `MsgHdr` wraps the platform `msghdr` with only
        // zero-sized lifetime metadata; this is the same representation
        // assumption used by `format_cmsg`.
        unsafe { &mut *(msg as *mut libc::msghdr as *mut MessageHeader<'_, '_, '_>) }
    }

    #[cfg(unix)]
    #[test]
    fn parse_cmsg_extracts_ipv6_pktinfo_and_ecn() {
        let mut cmsg_buffer = super::ControlBuffer([0u8; 1024]);
        let mut msg: libc::msghdr = unsafe { core::mem::zeroed() };
        let mut last_cmsg: *mut libc::cmsghdr = core::ptr::null_mut();
        let mut control_length: libc::c_int = 0;
        let dest_ip = [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

        msg.msg_control = cmsg_buffer.0.as_mut_ptr().cast();
        msg.msg_controllen = cmsg_buffer.0.len() as _;

        append_test_cmsg(
            &mut msg,
            &mut last_cmsg,
            &mut control_length,
            libc::IPPROTO_IPV6,
            libc::IPV6_PKTINFO,
            libc::in6_pktinfo {
                ipi6_addr: libc::in6_addr { s6_addr: dest_ip },
                ipi6_ifindex: 17,
            },
        );
        append_test_cmsg(
            &mut msg,
            &mut last_cmsg,
            &mut control_length,
            libc::IPPROTO_IPV6,
            libc::IPV6_TCLASS,
            0x03u8,
        );

        let header = MessageHeader::new().with_control(&cmsg_buffer.0[..control_length as usize]);
        let info = super::parse_cmsg(&header);

        assert_eq!(
            info.addr_dest,
            Some(SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::from(dest_ip),
                0,
                0,
                0
            )))
        );
        assert_eq!(info.dest_if, Some(17));
        assert_eq!(info.received_ecn, Some(EcnCodepoint::Ce));
        assert_eq!(info.udp_coalesced_size, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn format_cmsg_writes_ipv4_pktinfo_and_udp_segment() {
        let mut cmsg_buffer = super::ControlBuffer([0u8; 1024]);
        let mut msg = send_msg_with_control(&mut cmsg_buffer);
        let addr = SocketAddr::V4(core::net::SocketAddrV4::new(
            core::net::Ipv4Addr::new(192, 0, 2, 1),
            4433,
        ));

        super::format_cmsg(header_from_msghdr(&mut msg), 1400, 1200, Some(&addr), 12);

        // SAFETY: `msg` owns a live control buffer that `format_cmsg` has just
        // populated with well-formed cmsg headers.
        let first = unsafe { libc::CMSG_FIRSTHDR(&msg as *const libc::msghdr) };
        assert!(!first.is_null());
        // SAFETY: `first` is the first cmsg returned for `msg`.
        unsafe {
            assert_eq!((*first).cmsg_level, libc::IPPROTO_IP);
            assert_eq!((*first).cmsg_type, libc::IP_PKTINFO);
        }
        // SAFETY: the first cmsg is the IPv4 pktinfo cmsg just checked above.
        let pktinfo = unsafe { super::cmsg_data_as::<libc::in_pktinfo>(first) }
            .expect("IPv4 pktinfo payload");
        let SocketAddr::V4(addr4) = addr else {
            unreachable!();
        };
        assert_eq!(
            pktinfo.ipi_spec_dst.s_addr,
            super::ipv4_addr_to_in_addr(addr4.ip()).s_addr
        );
        assert_eq!(pktinfo.ipi_ifindex, 12);

        // SAFETY: `first` belongs to `msg`; CMSG_NXTHDR returns the next valid
        // cmsg or null at the end.
        let second = unsafe { libc::CMSG_NXTHDR(&msg as *const libc::msghdr, first as *const _) };
        assert!(!second.is_null());
        // SAFETY: `second` is a cmsg returned for `msg`.
        unsafe {
            assert_eq!((*second).cmsg_level, libc::SOL_UDP);
            assert_eq!((*second).cmsg_type, libc::UDP_SEGMENT);
        }
        // SAFETY: the second cmsg is the UDP_SEGMENT cmsg just checked above.
        let segment = unsafe { super::cmsg_data_as::<u16>(second) }.expect("segment payload");
        assert_eq!(segment, 1200);

        // SAFETY: `second` belongs to `msg`; this verifies the formatter wrote
        // exactly the two expected cmsgs.
        let third = unsafe { libc::CMSG_NXTHDR(&msg as *const libc::msghdr, second as *const _) };
        assert!(third.is_null());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn format_cmsg_writes_ipv6_pktinfo_dontfrag_and_udp_segment() {
        let mut cmsg_buffer = super::ControlBuffer([0u8; 1024]);
        let mut msg = send_msg_with_control(&mut cmsg_buffer);
        let addr = SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
            4433,
            0,
            0,
        ));

        super::format_cmsg(header_from_msghdr(&mut msg), 1500, 1200, Some(&addr), 17);

        // SAFETY: `msg` owns a live control buffer that `format_cmsg` has just
        // populated with well-formed cmsg headers.
        let first = unsafe { libc::CMSG_FIRSTHDR(&msg as *const libc::msghdr) };
        assert!(!first.is_null());
        // SAFETY: `first` is the first cmsg returned for `msg`.
        unsafe {
            assert_eq!((*first).cmsg_level, libc::IPPROTO_IPV6);
            assert_eq!((*first).cmsg_type, libc::IPV6_PKTINFO);
        }
        // SAFETY: the first cmsg is the IPv6 pktinfo cmsg just checked above.
        let pktinfo = unsafe { super::cmsg_data_as::<libc::in6_pktinfo>(first) }
            .expect("IPv6 pktinfo payload");
        let SocketAddr::V6(addr6) = addr else {
            unreachable!();
        };
        assert_eq!(pktinfo.ipi6_addr.s6_addr, addr6.ip().octets());
        assert_eq!(pktinfo.ipi6_ifindex, 17);

        // SAFETY: `first` belongs to `msg`; CMSG_NXTHDR returns the next valid
        // cmsg or null at the end.
        let second = unsafe { libc::CMSG_NXTHDR(&msg as *const libc::msghdr, first as *const _) };
        assert!(!second.is_null());
        // SAFETY: `second` is a cmsg returned for `msg`.
        unsafe {
            assert_eq!((*second).cmsg_level, libc::SOL_IPV6);
            assert_eq!((*second).cmsg_type, libc::IPV6_DONTFRAG);
        }
        // SAFETY: the second cmsg is the IPV6_DONTFRAG cmsg just checked.
        let dontfrag =
            unsafe { super::cmsg_data_as::<libc::c_int>(second) }.expect("dontfrag payload");
        assert_eq!(dontfrag, 1);

        // SAFETY: `second` belongs to `msg`; CMSG_NXTHDR returns the next valid
        // cmsg or null at the end.
        let third = unsafe { libc::CMSG_NXTHDR(&msg as *const libc::msghdr, second as *const _) };
        assert!(!third.is_null());
        // SAFETY: `third` is a cmsg returned for `msg`.
        unsafe {
            assert_eq!((*third).cmsg_level, libc::SOL_UDP);
            assert_eq!((*third).cmsg_type, libc::UDP_SEGMENT);
        }
        // SAFETY: the third cmsg is the UDP_SEGMENT cmsg just checked above.
        let segment = unsafe { super::cmsg_data_as::<u16>(third) }.expect("segment payload");
        assert_eq!(segment, 1200);

        // SAFETY: `third` belongs to `msg`; this verifies the formatter wrote
        // exactly the three expected cmsgs.
        let fourth = unsafe { libc::CMSG_NXTHDR(&msg as *const libc::msghdr, third as *const _) };
        assert!(fourth.is_null());
    }

    #[cfg(unix)]
    #[test]
    fn is_unreachable_matches_unix_c_list() {
        for code in [
            libc::EAFNOSUPPORT,
            libc::ECONNRESET,
            libc::EHOSTUNREACH,
            libc::ENETDOWN,
            libc::ENETUNREACH,
            -1,
        ] {
            assert!(OsError(code).is_unreachable());
        }

        for code in [0, libc::EIO, libc::EACCES, libc::EADDRNOTAVAIL] {
            assert!(!OsError(code).is_unreachable());
        }
    }

    #[cfg(unix)]
    #[test]
    fn socket_set_pkt_info_reports_setsockopt_failure() {
        assert!(super::picoquic_socket_set_pkt_info(-1, libc::AF_INET).is_err());
        assert!(super::picoquic_socket_set_pkt_info(-1, libc::AF_INET6).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn socket_set_pmtud_reports_setsockopt_failure() {
        assert!(super::picoquic_socket_set_pmtud_options(-1, libc::AF_INET).is_err());
        assert!(super::picoquic_socket_set_pmtud_options(-1, libc::AF_INET6).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn select_timeout_returns_zero_without_recv() {
        let (socket, _writer) = pipe_socket();
        let mut sockets = [socket];
        let mut buffer = [0u8; 16];

        let info = super::select(&mut sockets, &mut buffer, 0).expect("select timeout");

        assert_eq!(info.bytes_recv, 0);
        assert_eq!(info.socket_rank, 0);
        assert_eq!(sockets[0].recv_calls, 0);
        assert!(info.current_time.ticks() > 0);
    }

    #[cfg(unix)]
    #[test]
    fn select_receives_only_from_ready_socket() {
        let (socket0, _writer0) = pipe_socket();
        let (socket1, writer1) = pipe_socket();
        let mut sockets = [socket0, socket1];
        let mut buffer = [0u8; 16];

        write_fd(writer1.0, b"abc");
        let info = super::select(&mut sockets, &mut buffer, 1_000_000).expect("select ready");

        assert_eq!(info.bytes_recv, 3);
        assert_eq!(info.socket_rank, 1);
        assert_eq!(&buffer[..3], b"abc");
        assert_eq!(sockets[0].recv_calls, 0);
        assert_eq!(sockets[1].recv_calls, 1);
        assert!(info.current_time.ticks() > 0);
    }

    #[cfg(unix)]
    #[test]
    fn select_ready_recv_error_returns_c_failure_value() {
        let (mut socket, writer) = pipe_socket();
        socket.recv_error = true;
        let mut sockets = [socket];
        let mut buffer = [0u8; 16];

        write_fd(writer.0, b"x");
        let info = super::select(&mut sockets, &mut buffer, 1_000_000).expect("select error");

        assert_eq!(info.bytes_recv, -1);
        assert_eq!(info.socket_rank, 0);
        assert_eq!(sockets[0].recv_calls, 1);
        assert!(info.current_time.ticks() > 0);
    }
}
