//! Translation of `quic/packet_loop.h`.
//!
//! Bundle of types and entry points implementing quic's
//! reference event loop (`sockloop.c`) on top of the [`socks`]
//! UDP shims.  The loop:
//!
//! * owns one or more [`SocketCtx`] per network family,
//! * polls them with [`socks::select`] (or `select`/
//!   `WSAWaitForMultipleEvents` in the C original),
//! * routes received datagrams into quic, and
//! * fires application callbacks ([`PacketLoopCbFn`]) at
//!   well-defined points (loop ready, after recv, after send,
//!   address change, time check, system-call duration spike, wake-up,
//!   alt-port).
//!
//! Three layered entry points cover the same loop with different
//! parameter shapes:
//!
//! * [`run`] — the original 1.x signature, preserved for
//!   compatibility.  Builds a [`LoopParam`] on the stack and forwards.
//! * [`run_v2`] — takes a [`LoopParam`] directly.
//! * [`NetworkThreadCtx::run`] — takes a fully-populated
//!   [`NetworkThreadCtx`]; the threaded entry point.
//!
//! Plus the threading helpers ([`NetworkThreadCtx::spawn`] et al.)
//! that wrap `pthread_create` / `CreateThread`.
//!
//! Phase 4 fills the event-loop, thread-management, server-context,
//! and socket-opening bodies against the safe Rust socket abstraction.
//!
//! [`socks`]: crate::socks
//!
//! ## Pointer-shape decisions
//!
//! Read from `quic/sockloop.c`, `quic/winsockloop.c`, and
//! the demo callers in `first/demo.c`,
//! `sample/sample_*.c`, and `test/sockloop_test.c`:
//!
//! * `Quic*` parameters are non-null in every observed
//!   caller — they translate to `&mut Quic`.
//! * `LoopParam*` is *borrowed* by the loop
//!   helpers (the caller stack-allocates it in
//!   `run_v2` / the demo apps) — `&mut` here.  Inside
//!   [`NetworkThreadCtx`] the same pointer is *owned*
//!   when `is_param_allocated == 1` ([`Config::start_server_threads`]
//!   `malloc`s a copy and `delete_network_thread` `free`s
//!   it).  Phase 1 keeps the field as `Option<Box<…>>` and folds
//!   the C `is_param_allocated` flag away — the `Box` itself
//!   carries ownership; borrowed-param call sites store `None`
//!   plus a separate borrow at runtime.
//! * `picoquic_quic_config_t*` is borrowed-mut by
//!   [`Quic::create_server`] and [`Config::start_server_threads`]
//!   (they read and may mutate fields like `ticket_encryption_key`).
//! * Function-pointer typedefs (`packet_loop_cb_fn`,
//!   `custom_thread_create_fn`, `_setname_fn`,
//!   `_delete_fn`) become traits per the Phase 1 rules; the C `void*
//!   callback_ctx` companion folds into the trait implementor's
//!   state.  The C `(cb_mode, void* callback_argv)` tagged-union
//!   pair becomes the typed [`LoopEvent`] enum — each variant
//!   carries the payload type the C `cb_mode` implied.
//! * `SocketCtx* s_ctx` is used as both a single object
//!   ([`SocketCtx::close`]) and a fixed-size array
//!   ([`open_sockets`] writes up to `PACKET_LOOP_SOCKETS_MAX`
//!   entries).  The single-object path is a method on [`SocketCtx`];
//!   the array path takes `&mut [SocketCtx]`, with the slice length
//!   subsuming the C convention of "callee writes and returns the count".
//! * `NetworkThreadCtx**` outputs from
//!   [`Config::start_server_threads`] become `&mut [Option<Box<…>>]`
//!   — the slice carries `nb_threads_max`; each slot is filled with
//!   `Some` on success.
//! * Bitfields collapse to `bool`s.  Single-bit C `int : 1` /
//!   `unsigned int : 1` flag fields don't justify `bitflags!` on a
//!   set of unrelated booleans (see `config.rs` for the
//!   established convention).
//! * `volatile int` fields in [`NetworkThreadCtx`]
//!   become plain `i32` / `bool` — `Send`/`Sync` is out of v1 scope,
//!   so the volatile semantics have nowhere to land.
//! * `sockaddr_storage` fields fold into `Option<SocketAddr>`,
//!   matching the [`socks`] convention (`AF_UNSPEC` ↔ `None`).
//! * The Windows-only fields (`overlap`, `WSARecvMsg`, `WSASendMsg`,
//!   `dataBuf`, `msg`, …) and the io_uring fields are dropped per
//!   the v1 single-target scope.
//! * `recv_buffer: uint8_t* + recv_buffer_size: size_t` is an
//!   owning pair (allocated in `packet_set_windows_socket`
//!   on the Windows path, or `packet_loop_recv_buf_uring_init`
//!   on io_uring; freed in [`SocketCtx::close`]).
//!   On the canonical Linux/`select` build neither is used — every
//!   datagram lands in the shared loop buffer.  The field shape is
//!   `Option<Vec<u8>>` so the unused state is `None`; Vec subsumes
//!   the (pointer, size) pair into one owning value.

use core::net::SocketAddr;

use std::thread::JoinHandle;

use crate::Error;
use crate::Instant;
use crate::config::Config;
use crate::errors::InternalError;
use crate::socks::{EcnCodepoint, OsError, RecvInfo, Socket};
use crate::{AlpnSelect, Quic, StreamDataCallback};

const AF_INET: i32 = 2;
const AF_INET6: i32 = 10;
const EIO: i32 = 5;
const PACKET_LOOP_DELAY_MAX: i64 = 10_000_000;

// ---------------------------------------------------------------------------
// Compile-time limits.

/// Maximum number of UDP sockets a single packet loop can drive at
/// once (one per address family, optionally times two for the
/// public/private port split).  C: `PACKET_LOOP_SOCKETS_MAX`.
pub const PACKET_LOOP_SOCKETS_MAX: usize = 4;

/// Maximum datagrams pulled from one socket per loop iteration.
/// C: `PACKET_LOOP_RECV_MAX`.
pub const PACKET_LOOP_RECV_MAX: usize = 10;

/// Maximum datagrams pushed to one socket per loop iteration.
/// C: `PACKET_LOOP_SEND_MAX`.
pub const PACKET_LOOP_SEND_MAX: usize = 10;

/// Hard cap on how long the loop will wait between iterations
/// (microseconds).  C: `PACKET_LOOP_SEND_DELAY_MAX`.
pub const PACKET_LOOP_SEND_DELAY_MAX: u64 = 2500;

// ---------------------------------------------------------------------------
// Per-socket context.

/// State the loop maintains per UDP socket.  C: `socket_ctx_t`.
///
/// Trimmed to the canonical Linux build: the Windows
/// (`WSAOVERLAPPED`, `WSARecvMsg`, …) and io_uring (`msghdr`,
/// `iovec`, `ctrl_buffer`) fields are dropped per the v1
/// single-target scope.  The four C bitfields collapse to plain
/// `bool` flags.
pub struct SocketCtx<S: crate::socks::Socket> {
    /// OS socket handle.  `None` mirrors the C `INVALID_SOCKET`
    /// sentinel for "not yet open".  Generic over the active
    /// `Socket` implementation; the default in production is
    /// `crate::socks_socket2::Socket2Udp` (wraps `socket2::Socket`).
    pub fd: Option<S>,
    /// Address family the socket was opened in (`AF_INET`,
    /// `AF_INET6`).  Stays `i32` to match call sites that pass the
    /// libc `AF_*` constants directly.
    pub af: i32,
    /// Port number the socket is bound to (host byte order).
    pub port: u16,
    /// `htons(port)` cached so the loop can stamp it into received
    /// destination addresses without a per-packet byte-swap.
    pub n_port: u16,
    /// Private port number — the port reserved for this thread when
    /// the public port is shared with siblings (see
    /// `is_port_shared`).  C: `private_port`.
    pub private_port: u16,
    /// Whether this socket is bound to a port shared with other
    /// threads (e.g., port 443 across an H3 thread pool — uses
    /// `SO_REUSEPORT`).  C: `int is_port_shared : 1`.
    pub is_port_shared: bool,
    /// Whether [`open_sockets`]-equivalent setup has completed
    /// for this socket.  C: `unsigned int is_started : 1`.
    pub is_started: bool,
    /// Whether the kernel honored `UDP_SEGMENT` (GSO) on this
    /// socket.  C: `unsigned int supports_udp_send_coalesced : 1`.
    pub supports_udp_send_coalesced: bool,
    /// Whether the kernel honored `UDP_GRO` on this socket.
    /// C: `unsigned int supports_udp_recv_coalesced : 1`.
    pub supports_udp_recv_coalesced: bool,
    /// Owning receive buffer.  Used only on the Windows-async and
    /// io_uring paths; the canonical Linux build leaves it as
    /// `None` and uses the shared loop buffer instead.
    /// C: `uint8_t* recv_buffer` plus the `recv_buffer_size`
    /// companion (length is implicit in the slice).
    pub recv_buffer: Option<Box<[u8]>>,
    /// Source address of the most recent datagram (`recvmsg`'s
    /// `msg_name`).  C: `struct sockaddr_storage addr_from` plus
    /// `from_length` — `Option<SocketAddr>` carries both.
    pub addr_from: Option<SocketAddr>,
    /// Destination address parsed out of `IP_PKTINFO` /
    /// `IPV6_PKTINFO` for the most recent datagram.
    /// C: `struct sockaddr_storage addr_dest` + `dest_length`.
    pub addr_dest: Option<SocketAddr>,
    /// Receiving interface index (from the same cmsg).
    /// C: `int dest_if`.
    pub dest_if: i32,
    /// `IP_TOS` / `IPV6_TCLASS` ECN code-point on the most recent
    /// datagram.  C: `unsigned char received_ecn`.
    pub received_ecn: u8,
    /// Number of bytes read into the loop buffer on the most recent
    /// recv.  C: `int bytes_recv` (C uses `-1` for error; the Rust
    /// loop propagates errors through `Result` at the call site and
    /// only writes a valid byte count here).
    pub bytes_recv: usize,
    /// Scratch space for assembling outbound `cmsg` payloads
    /// (`IP_PKTINFO`, `IPV6_PKTINFO`, `UDP_SEGMENT`).
    /// C: `char cmsg_buffer[1024]`.
    pub cmsg_buffer: [u8; 1024],
    /// `UDP_GRO` segment size on the most recent datagram (the
    /// kernel's `UDP_GRO` cmsg).  C: `size_t udp_coalesced_size`.
    pub udp_coalesced_size: usize,
}

impl<S: crate::socks::Socket> Default for SocketCtx<S> {
    fn default() -> Self {
        SocketCtx {
            fd: None,
            af: 0,
            port: 0,
            n_port: 0,
            private_port: 0,
            is_port_shared: false,
            is_started: false,
            supports_udp_send_coalesced: false,
            supports_udp_recv_coalesced: false,
            recv_buffer: None,
            addr_from: None,
            addr_dest: None,
            dest_if: 0,
            received_ecn: 0,
            bytes_recv: 0,
            cmsg_buffer: [0; 1024],
            udp_coalesced_size: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Callback enum + companion arg structs.

/// Loop-callback event delivered to [`PacketLoopCbFn::callback`].
/// Replaces the C `(packet_loop_cb_enum cb_mode, void* callback_argv)`
/// pair: the enum carries the per-event payload directly, so
/// implementors no longer cast a `void*`.
/// C: `packet_loop_cb_enum`.
#[derive(Debug)]
pub enum LoopEvent<'a> {
    /// Loop has finished initializing.  C: `packet_loop_ready`,
    /// `callback_argv: *mut LoopOptions`.
    Ready(&'a mut LoopOptions),
    /// Number of packets received this iteration.  C:
    /// `packet_loop_after_receive`, `callback_argv: *mut size_t`.
    AfterReceive(usize),
    /// Number of packets sent this iteration.  C:
    /// `packet_loop_after_send`, `callback_argv: *mut size_t`.
    AfterSend(usize),
    /// New local address advertised by the application after a
    /// port update.  C: `packet_loop_port_update`,
    /// `callback_argv: *mut sockaddr`.
    PortUpdate(core::net::SocketAddr),
    /// Optional check-in fired when the application set
    /// [`LoopOptions::do_time_check`].  C:
    /// `packet_loop_time_check`.
    TimeCheck(&'a mut TimeCheckArg),
    /// Optional system-call duration report fired when the
    /// application set [`LoopOptions::do_system_call_duration`].
    /// C: `packet_loop_system_call_duration`.
    SystemCallDuration(&'a mut SystemCallDuration),
    /// Wake-up event triggered by [`NetworkThreadCtx::wake_up`].
    /// C: `packet_loop_wake_up` with `callback_argv: NULL`.
    WakeUp,
    /// Alt-port address surfaced for multipath / migration tests.
    /// C: `packet_loop_alt_port`.
    AltPort(core::net::SocketAddr),
}

/// System-call duration statistics surfaced through the optional
/// [`LoopEvent::SystemCallDuration`] callback.
/// C: `packet_loop_system_call_duration_t`.
#[derive(Debug, Default, Copy, Clone)]
pub struct SystemCallDuration {
    /// Duration of the most recent zero-delay system call
    /// (microseconds).
    pub scd_last: u64,
    /// Maximum observed system-call duration so far.
    pub scd_max: u64,
    /// Smoothed system-call duration (RFC-6298–style EWMA).
    pub scd_smoothed: u64,
    /// Mean absolute deviation around `scd_smoothed`.
    pub scd_dev: u64,
}

/// In/out argument for the [`LoopEvent::TimeCheck`] callback.
/// The loop fills `current_time` and a proposed `delta_t`;
/// the application overwrites `delta_t` with a (possibly smaller)
/// value if it has work to do sooner.
/// C: `packet_loop_time_check_arg_t`.
#[derive(Debug, Copy, Clone)]
pub struct TimeCheckArg {
    /// Loop-time timestamp.
    pub current_time: Instant,
    /// Proposed sleep, in microseconds — application may shrink.
    pub delta_t: i64,
}

impl Default for TimeCheckArg {
    fn default() -> Self {
        Self {
            current_time: Instant::from_ticks(0),
            delta_t: 0,
        }
    }
}

/// Application callback fired at the lifecycle and per-iteration
/// events listed in [`LoopEvent`].
///
/// C: `int (*packet_loop_cb_fn)(picoquic_quic_t* quic,
/// packet_loop_cb_enum cb_mode, void* callback_ctx, void*
/// callback_argv)`.  The `callback_ctx` companion folds into the
/// trait implementor's state; the `(cb_mode, callback_argv)` tagged
/// union folds into [`LoopEvent`].
///
/// Returns `Ok(())` on success, or an error to break the loop.
pub trait PacketLoopCbFn {
    fn callback(&mut self, quic: &mut Quic, event: LoopEvent<'_>) -> Result<(), Error>;
}

// ---------------------------------------------------------------------------
// Loop options + parameters.

/// Feature-flags the application advertises in response to the
/// [`LoopEvent::Ready`]
/// callback.  C: `packet_loop_options_t`, three single-bit
/// `unsigned int : 1` fields collapsed to `bool` per the
/// established convention (see `config.rs`).
#[derive(Debug, Default, Copy, Clone)]
pub struct LoopOptions {
    /// Application wants the loop to call back with
    /// [`LoopEvent::TimeCheck`]
    /// before each select.
    pub do_time_check: bool,
    /// Application wants notifications when zero-delay system calls
    /// take noticeably long.
    pub do_system_call_duration: bool,
    /// Application provides an alternate port for migration /
    /// multipath testing.
    pub provide_alt_port: bool,
}

/// Loop-construction parameters (v2 / v3 entry points).
/// C: `packet_loop_param_t`.  `repr(C)` is dropped — the
/// struct never crosses an external boundary.
#[derive(Debug, Default, Copy, Clone)]
pub struct LoopParam {
    /// Default port for outgoing connections.  `0` ⇒ ephemeral.
    pub local_port: u16,
    /// `AF_INET` / `AF_INET6`, or `0` for "both".
    pub local_af: i32,
    /// Outbound interface index (or `0` to leave it to the kernel).
    pub dest_if: i32,
    /// Server / public-facing port.  `0` for client-only loops.
    pub public_port: u16,
    /// Whether the public port is shared across this thread pool
    /// (uses `SO_REUSEPORT`).
    pub is_port_shared: bool,
    /// `SO_SNDBUF` / `SO_RCVBUF` size, or `0` to skip the syscall.
    pub socket_buffer_size: i32,
    /// Disable `UDP_SEGMENT` GSO even if the kernel advertises it.
    pub do_not_use_gso: bool,
    /// Open the public-port socket pair too (needed for migration).
    pub extra_socket_required: bool,
    /// When both port pairs exist, prefer the extra socket as the
    /// outgoing source.
    pub prefer_extra_socket: bool,
    /// Inject a synthetic `EIO` once into the send path (debug
    /// hook).
    pub simulate_eio: bool,
    /// Cap on the size of an individual coalesced send.  `0` ⇒
    /// kernel default.
    pub send_length_max: usize,
}

// ---------------------------------------------------------------------------
// Custom thread-management hooks.

/// Spawn-a-thread hook.  C:
/// `int (*custom_thread_create_fn)(void** thread_id,
/// thread_fn thread_fn, void* arg)`.
///
/// In C, `(thread_fn, arg)` is the trampoline + state pair the new
/// thread will run.  In Rust, [`std::thread::spawn`] already takes
/// an arbitrary `FnOnce() + Send` closure (which subsumes the C
/// "function pointer + void* arg" pattern), so the hook reduces to
/// "produce a [`JoinHandle`] from that closure".
pub trait CustomThreadCreateFn {
    /// Create a thread that runs `thread_fn` to completion.  The
    /// returned [`JoinHandle`] is owned by the caller and passed
    /// back through [`CustomThreadDeleteFn::delete`] at teardown.
    ///
    /// The C contract returns `0` for success / non-zero `errno`
    /// otherwise; the [`OsError`] payload carries the same value
    /// that the C `*ret` out-parameter would on
    /// [`NetworkThreadCtx::spawn_custom`].
    fn create(
        &mut self,
        thread_fn: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<JoinHandle<()>, OsError>;
}

/// Set-thread-name hook.  C:
/// `void (*custom_thread_setname_fn)(char const* thread_name)`.
///
/// Length is capped at 16 bytes (including the trailing NUL) on
/// Linux's `prctl(PR_SET_NAME, …)`; the trait takes a `&str` slice
/// and lets the implementor truncate / re-encode as needed.
pub trait CustomThreadSetnameFn {
    /// Apply `thread_name` to the *current* thread.  Called from
    /// inside the thread after it starts, per the C convention.
    fn set_name(&mut self, thread_name: &str);
}

/// Tear-down-a-thread hook.  C:
/// `void (*custom_thread_delete_fn)(void** thread_id)`.
///
/// In Rust the standard `JoinHandle` cleans itself up on drop, so
/// this hook only matters for backends that allocate their own
/// out-of-band thread bookkeeping; the default
/// [`internal_thread_delete`] just lets the handle go.
pub trait CustomThreadDeleteFn {
    /// Release any resources tied to `thread`.  Called from the
    /// destruction path of [`NetworkThreadCtx`].
    fn delete(&mut self, thread: JoinHandle<()>);
}

// ---------------------------------------------------------------------------
// Threaded packet-loop context.

/// Thread context carrying everything the v3 entry point needs to
/// run a packet loop.  C: `network_thread_ctx_t`.
///
/// Two ownership modes coexist in C:
///
/// * Foreground / blocking (`run_v2`): the caller stack-allocates a
///   `NetworkThreadCtx` and calls [`NetworkThreadCtx::run`] on it.
///   `is_threaded` stays `false`, no wake-up plumbing is created.
/// * Background ([`NetworkThreadCtx::spawn`]): the helper
///   allocates the context, populates it, opens the wake-up
///   pipe / event, and launches the OS thread; teardown via
///   `Drop` (Phase 3) frees everything.
///
/// Phase 1 keeps a single struct shape with optional fields so both
/// modes fit; Phase 3 may split them or introduce a builder.
///
/// `volatile` qualifiers in the C source become ordinary fields; the
/// Rust wake-up channel and thread handle carry the cross-thread
/// synchronization used by this translation.
pub struct NetworkThreadCtx {
    // The C `picoquic_quic_t* quic` back-pointer is represented at
    // the foreground entry point by passing `&mut Quic` into the
    // private loop helper together with this context.
    /// Loop parameters, optionally owned (`is_param_allocated`).
    /// Phase 1 collapses the C borrowed-or-owned discriminant into
    /// a single `Option<Box<…>>`; the borrowed case stores `None`
    /// and the borrow lives outside this struct.
    pub param: Option<Box<LoopParam>>,
    /// Application loop callback.  `None` matches the C `NULL`
    /// (the loop runs without notifying the app on each event —
    /// used by the bench harness).
    pub loop_callback: Option<Box<dyn PacketLoopCbFn>>,
    /// Custom thread-deletion hook; defaults to
    /// `internal_thread_delete` when the C caller passes
    /// `NULL`.
    pub thread_delete_fn: Option<Box<dyn CustomThreadDeleteFn>>,
    /// Custom thread-naming hook; defaults to
    /// `internal_thread_setname`.
    pub thread_setname_fn: Option<Box<dyn CustomThreadSetnameFn>>,
    /// Optional thread name.  Capped at 16 bytes (including NUL)
    /// on Linux per `prctl(PR_SET_NAME, …)`.
    pub thread_name: Option<String>,
    /// Underlying OS thread handle.  `None` for the foreground v2
    /// entry path (no thread is spawned).
    pub pthread: Option<JoinHandle<()>>,
    /// Wake-up pipe (Linux: `pipe2`).  `[ -1, -1 ]` when no wake-up
    /// has been opened (`wake_up_defined == false`).  The Windows
    /// `HANDLE wake_up_event` variant is dropped per the v1 scope.
    pub wake_up_pipe_fd: [i32; 2],
    wake_up_sender: Option<std::sync::mpsc::Sender<()>>,
    wake_up_receiver: Option<std::sync::mpsc::Receiver<()>>,
    /// Whether [`NetworkThreadCtx::spawn_custom`] actually
    /// spawned a thread (vs. the foreground path that reuses the
    /// caller's stack).  C: `int is_threaded`.
    pub is_threaded: bool,
    /// Whether [`Self::wake_up_pipe_fd`] was successfully opened.
    /// C: `int wake_up_defined`.
    pub wake_up_defined: bool,
    /// Set by the loop once it finishes initialization (after the
    /// first [`LoopEvent::Ready`] callback).
    /// C: `volatile int thread_is_ready`.
    pub thread_is_ready: bool,
    /// Set from the outside to ask the loop to exit (the `Drop`
    /// implementation will set this in Phase 3).
    /// C: `volatile int thread_should_close`.
    pub thread_should_close: bool,
    /// Set by the loop on the way out, before returning from v3.
    /// C: `volatile int thread_is_closed`.
    pub thread_is_closed: bool,
    /// Final return code from the loop body.  Read by the caller
    /// of [`run_v2`] after [`NetworkThreadCtx::run`] returns.
    /// C: `int return_code`.
    pub return_code: i32,
}

impl Default for NetworkThreadCtx {
    fn default() -> Self {
        NetworkThreadCtx {
            param: None,
            loop_callback: None,
            thread_delete_fn: None,
            thread_setname_fn: None,
            thread_name: None,
            pthread: None,
            wake_up_pipe_fd: [-1, -1],
            wake_up_sender: None,
            wake_up_receiver: None,
            is_threaded: false,
            wake_up_defined: false,
            thread_is_ready: false,
            thread_should_close: false,
            thread_is_closed: false,
            return_code: 0,
        }
    }
}

fn af_for_addr(addr: &SocketAddr) -> i32 {
    if addr.is_ipv4() { AF_INET } else { AF_INET6 }
}

fn loopback_addr(af: i32, port: u16) -> Option<SocketAddr> {
    match af {
        AF_INET => Some(SocketAddr::from(([127, 0, 0, 1], port))),
        AF_INET6 => Some(SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port))),
        _ => None,
    }
}

fn unspecified_addr(af: i32, port: u16) -> SocketAddr {
    match af {
        AF_INET6 => SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], port)),
        _ => SocketAddr::from(([0, 0, 0, 0], port)),
    }
}

fn ecn_codepoint(value: u8) -> EcnCodepoint {
    match value & 0x03 {
        1 => EcnCodepoint::Ect1,
        2 => EcnCodepoint::Ect0,
        3 => EcnCodepoint::Ce,
        _ => EcnCodepoint::NotEct,
    }
}

fn is_internal_signal(error: &Error, signal: InternalError) -> bool {
    matches!(error, Error::Protocol(code) if *code == signal as u64)
}

fn return_code(error: &Error) -> i32 {
    match *error {
        Error::Protocol(code) => code as i32,
        Error::Memory => InternalError::Memory as i32,
        Error::InvalidArgument => -1,
        Error::InvalidFile => InternalError::InvalidFile as i32,
        Error::NoSuchFile => InternalError::NoSuchFile as i32,
        Error::InvalidFrame => InternalError::InvalidFrame as i32,
        Error::InvalidState => InternalError::UnexpectedState as i32,
        Error::BufferTooSmall => InternalError::ExtensionBufferTooSmall as i32,
        Error::Disconnected => InternalError::Disconnected as i32,
        Error::Tls => -1,
        Error::Generic => -1,
    }
}

fn store_loop_result(thread_ctx: &mut NetworkThreadCtx, result: &Result<(), Error>) {
    thread_ctx.return_code = match result {
        Ok(()) => 0,
        Err(error) => return_code(error),
    };
}

fn loop_callback(
    callback: &mut Option<Box<dyn PacketLoopCbFn>>,
    quic: &mut Quic,
    event: LoopEvent<'_>,
) -> Result<(), Error> {
    if let Some(callback) = callback.as_mut() {
        callback.callback(quic, event)?;
    }
    Ok(())
}

fn monitor_system_call_duration(
    sc_duration: &mut SystemCallDuration,
    current_time: Instant,
    previous_time: Instant,
) -> bool {
    let duration = current_time.ticks().saturating_sub(previous_time.ticks());
    let mut dev = sc_duration.scd_smoothed as i64 - duration as i64;
    let mut shall_notify = false;

    if duration > sc_duration.scd_max {
        shall_notify = true;
        sc_duration.scd_max = duration;
    } else if duration != sc_duration.scd_last {
        let delta_d = sc_duration.scd_last as i64 - duration as i64;
        if !(-1000..=1000).contains(&delta_d) || delta_d < sc_duration.scd_last as i64 {
            shall_notify = true;
        }
        sc_duration.scd_last = duration;
    }

    sc_duration.scd_smoothed = (duration + 15 * sc_duration.scd_smoothed) / 16;
    if dev < 0 {
        dev = -dev;
    }
    sc_duration.scd_dev = (7 * sc_duration.scd_dev + dev as u64) / 8;

    shall_notify
}

fn open_socket<S: Socket>(
    socket_buffer_size: i32,
    _do_not_use_gso: bool,
    s_ctx: &mut SocketCtx<S>,
    ecn_value: u8,
) -> Result<(), Error> {
    let mut fd = match s_ctx.af {
        AF_INET => S::open_server_v4(s_ctx.port as i32)?,
        AF_INET6 => S::open_server_v6(s_ctx.port as i32)?,
        _ => return Err(Error::InvalidArgument),
    };

    let _ = socket_buffer_size;
    fd.set_ecn_options_ex(ecn_codepoint(ecn_value))?;
    fd.set_pkt_info()?;
    fd.set_pmtud_options()?;

    let local_address = fd.local_address()?;
    s_ctx.port = local_address.port();
    s_ctx.n_port = s_ctx.port.to_be();
    s_ctx.fd = Some(fd);
    s_ctx.is_started = true;
    s_ctx.supports_udp_send_coalesced = false;
    s_ctx.supports_udp_recv_coalesced = false;
    Ok(())
}

fn recv_from_sockets<S: Socket>(
    s_ctx: &mut [SocketCtx<S>],
    nb_sockets_available: usize,
    buffer: &mut [u8],
) -> Result<Option<(usize, RecvInfo)>, Error> {
    for (rank, ctx) in s_ctx.iter_mut().take(nb_sockets_available).enumerate() {
        let Some(fd) = ctx.fd.as_mut() else {
            continue;
        };
        match fd.recv(buffer) {
            Ok(info) if info.bytes_recv > 0 => {
                ctx.addr_from = info.addr_from;
                ctx.addr_dest = info.addr_dest;
                ctx.dest_if = info.dest_if;
                ctx.received_ecn = info.received_ecn;
                ctx.bytes_recv = info.bytes_recv;
                ctx.udp_coalesced_size = 0;
                return Ok(Some((rank, info)));
            }
            Ok(info) => {
                ctx.bytes_recv = info.bytes_recv;
            }
            Err(Error::Generic) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

fn wait_for_wake_up(thread_ctx: &mut NetworkThreadCtx, delta_t: i64) -> bool {
    let Some(receiver) = thread_ctx.wake_up_receiver.as_ref() else {
        return false;
    };

    if delta_t <= 0 {
        receiver.try_recv().is_ok()
    } else {
        let timeout = std::time::Duration::from_micros(delta_t as u64);
        receiver.recv_timeout(timeout).is_ok()
    }
}

fn run_packet_loop<S: Socket>(
    quic: &mut Quic,
    param: &mut LoopParam,
    thread_ctx: &mut NetworkThreadCtx,
) -> Result<(), Error> {
    let ecn_value = quic
        .default_congestion_alg
        .map(|algorithm| algorithm.ecn_mark)
        .unwrap_or(0);
    let mut s_ctx: [SocketCtx<S>; PACKET_LOOP_SOCKETS_MAX] =
        std::array::from_fn(|_| SocketCtx::default());
    let mut nb_sockets = open_sockets(
        param.local_port,
        param.local_af,
        param.public_port,
        param.is_port_shared,
        param.socket_buffer_size,
        param.extra_socket_required,
        param.do_not_use_gso,
        &mut s_ctx,
        ecn_value,
    )?;
    if nb_sockets == 0 {
        return Err(Error::Protocol(InternalError::UnexpectedError as u64));
    }

    let mut options = LoopOptions::default();
    loop_callback(
        &mut thread_ctx.loop_callback,
        quic,
        LoopEvent::Ready(&mut options),
    )?;
    if let Some(local_addr) = loopback_addr(s_ctx[0].af, s_ctx[0].port) {
        loop_callback(
            &mut thread_ctx.loop_callback,
            quic,
            LoopEvent::PortUpdate(local_addr),
        )?;
    }
    if options.provide_alt_port {
        let alt_sock = if nb_sockets > 2 && param.local_af == 0 {
            2
        } else {
            1.min(nb_sockets - 1)
        };
        if let Some(alt_addr) = loopback_addr(s_ctx[alt_sock].af, s_ctx[alt_sock].port) {
            loop_callback(
                &mut thread_ctx.loop_callback,
                quic,
                LoopEvent::AltPort(alt_addr),
            )?;
        }
    }

    let mut send_buffer_size = param.socket_buffer_size as usize;
    if send_buffer_size == 0 {
        send_buffer_size = 0xffff;
    }
    let mut send_buffer = vec![0u8; send_buffer_size];
    let mut recv_buffer = [0u8; crate::MAX_PACKET_SIZE];
    let mut nb_sockets_available = nb_sockets;
    let mut loop_immediate = false;
    let mut nb_loop_immediate = 0usize;
    let mut gso_enabled = !param.do_not_use_gso;
    let mut sc_duration = SystemCallDuration::default();
    let mut result = Ok(());

    thread_ctx.thread_is_ready = true;
    thread_ctx.thread_is_closed = false;

    while result.is_ok() && !thread_ctx.thread_should_close {
        let mut delta_t = 0;
        let current_time = Instant::from_ticks(crate::current_time());
        if !loop_immediate {
            nb_loop_immediate = 1;
            delta_t = quic.next_wake_delay(current_time, PACKET_LOOP_DELAY_MAX);
            if options.do_time_check {
                let mut time_check_arg = TimeCheckArg {
                    current_time,
                    delta_t,
                };
                if let Err(error) = loop_callback(
                    &mut thread_ctx.loop_callback,
                    quic,
                    LoopEvent::TimeCheck(&mut time_check_arg),
                ) {
                    result = Err(error);
                    break;
                }
                if time_check_arg.delta_t < delta_t {
                    delta_t = time_check_arg.delta_t;
                }
            }
        } else {
            nb_loop_immediate += 1;
        }
        loop_immediate = false;

        let previous_time = current_time;
        let is_wake_up_event = wait_for_wake_up(thread_ctx, delta_t);
        let recv_result = if is_wake_up_event {
            Ok(None)
        } else {
            recv_from_sockets(&mut s_ctx, nb_sockets_available, &mut recv_buffer)
        };
        let current_time = Instant::from_ticks(crate::current_time());

        if options.do_system_call_duration
            && delta_t == 0
            && monitor_system_call_duration(&mut sc_duration, current_time, previous_time)
            && let Err(error) = loop_callback(
                &mut thread_ctx.loop_callback,
                quic,
                LoopEvent::SystemCallDuration(&mut sc_duration),
            )
        {
            result = Err(error);
            break;
        }

        let recv_info = match recv_result {
            Ok(info) => info,
            Err(error) => {
                result = if thread_ctx.thread_should_close {
                    Ok(())
                } else {
                    Err(error)
                };
                break;
            }
        };

        if is_wake_up_event
            && let Err(error) =
                loop_callback(&mut thread_ctx.loop_callback, quic, LoopEvent::WakeUp)
        {
            result = Err(error);
            break;
        }

        let mut simulate_nat = false;
        if let Some((socket_rank, info)) = recv_info {
            let recv_len = info.bytes_recv.min(recv_buffer.len());
            let addr_from = info
                .addr_from
                .unwrap_or_else(|| unspecified_addr(s_ctx[socket_rank].af, 0));
            let addr_to = info.addr_dest.unwrap_or_else(|| {
                unspecified_addr(s_ctx[socket_rank].af, s_ctx[socket_rank].port)
            });
            let packet = &mut recv_buffer[..recv_len];
            match quic.incoming_packet_ex(
                packet,
                &addr_from,
                &addr_to,
                info.dest_if,
                info.received_ecn,
                current_time,
            ) {
                Ok(_) => {}
                Err(error) if is_internal_signal(&error, InternalError::NoErrorSimulateNat) => {
                    simulate_nat = true;
                }
                Err(error) => {
                    result = Err(error);
                    break;
                }
            }

            if let Err(error) = loop_callback(
                &mut thread_ctx.loop_callback,
                quic,
                LoopEvent::AfterReceive(recv_len),
            ) {
                if is_internal_signal(&error, InternalError::NoErrorSimulateNat) {
                    simulate_nat = true;
                } else {
                    result = Err(error);
                    break;
                }
            }

            if !simulate_nat && nb_loop_immediate < PACKET_LOOP_RECV_MAX {
                loop_immediate = true;
                continue;
            }
        }

        if simulate_nat && param.extra_socket_required {
            nb_sockets_available = nb_sockets / 2;
        }

        let loop_time = current_time;
        let mut bytes_sent = 0usize;
        let mut nb_packets_sent = 0usize;
        while result.is_ok() && nb_packets_sent < PACKET_LOOP_SEND_MAX {
            let prepared = match quic.prepare_next_packet_ex(loop_time, &mut send_buffer) {
                Ok(prepared) => prepared,
                Err(error) => {
                    result = Err(error);
                    break;
                }
            };

            let crate::PreparedPacket {
                send_length,
                addr_to,
                addr_from,
                if_index,
                send_msg_size,
                last_connection: _,
                log_cid: _,
            } = prepared;
            if send_length == 0 {
                break;
            }

            let msg_size = if gso_enabled {
                send_msg_size.unwrap_or(0)
            } else {
                0
            };
            nb_packets_sent += if msg_size == 0 {
                1
            } else {
                send_length.div_ceil(msg_size)
            };
            if send_length > param.send_length_max {
                param.send_length_max = send_length;
            }

            let send_port = addr_from.port();
            let mut send_socket_rank = None;
            for (rank, ctx) in s_ctx.iter().take(nb_sockets_available).enumerate() {
                if ctx.af == af_for_addr(&addr_to) && ctx.fd.is_some() {
                    send_socket_rank = Some(rank);
                    if send_port == 0 && !param.prefer_extra_socket {
                        break;
                    }
                    if ctx.n_port == send_port.to_be() {
                        break;
                    }
                }
            }

            if send_socket_rank.is_none() && nb_sockets_available < PACKET_LOOP_SOCKETS_MAX {
                let new_ctx = &mut s_ctx[nb_sockets_available];
                *new_ctx = SocketCtx::default();
                new_ctx.af = af_for_addr(&addr_to);
                new_ctx.port = addr_to.port();
                new_ctx.n_port = new_ctx.port.to_be();
                if open_socket(
                    param.socket_buffer_size,
                    param.do_not_use_gso,
                    new_ctx,
                    ecn_value,
                )
                .is_ok()
                {
                    send_socket_rank = Some(nb_sockets_available);
                    nb_sockets_available += 1;
                    nb_sockets = nb_sockets.max(nb_sockets_available);
                }
            }

            bytes_sent += send_length;
            let Some(send_socket_rank) = send_socket_rank else {
                continue;
            };
            let Some(send_socket) = s_ctx[send_socket_rank].fd.as_mut() else {
                continue;
            };

            let bytes = &send_buffer[..send_length];
            let mut send_result = if param.simulate_eio && send_length > crate::MAX_PACKET_SIZE {
                param.simulate_eio = false;
                Err(OsError(EIO))
            } else {
                send_socket.send(&addr_to, Some(&addr_from), if_index, bytes, msg_size as i32)
            };

            if let Err(OsError(err)) = send_result
                && err == EIO
                && msg_size > 0
            {
                let mut packet_index = 0usize;
                let mut packet_size = msg_size;
                while packet_index < send_length {
                    if packet_index + packet_size > send_length {
                        packet_size = send_length - packet_index;
                    }
                    let chunk = &send_buffer[packet_index..packet_index + packet_size];
                    match send_socket.send(&addr_to, Some(&addr_from), if_index, chunk, 0) {
                        Ok(_) => {
                            packet_index += packet_size;
                            send_result = Ok(packet_size);
                        }
                        Err(error) => {
                            send_result = Err(error);
                            break;
                        }
                    }
                }
                gso_enabled = false;
            }

            if let Err(error) = send_result
                && error.is_unreachable()
            {
                result = Err(Error::Protocol(InternalError::SocketError as u64));
                break;
            }
        }

        if result.is_ok()
            && let Err(error) = loop_callback(
                &mut thread_ctx.loop_callback,
                quic,
                LoopEvent::AfterSend(bytes_sent),
            )
        {
            result = Err(error);
        }
    }

    thread_ctx.thread_is_ready = false;
    thread_ctx.thread_is_closed = true;

    for ctx in s_ctx.iter_mut().take(nb_sockets) {
        ctx.close();
    }

    if let Err(error) = &result
        && is_internal_signal(error, InternalError::NoErrorTerminatePacketLoop)
    {
        result = Ok(());
    }
    if thread_ctx.thread_should_close && result.is_err() {
        result = Ok(());
    }
    store_loop_result(thread_ctx, &result);
    result
}

// ---------------------------------------------------------------------------
// Loop entry points.

impl Quic {
    /// Drive the packet loop until the application or an error breaks
    /// it.  C: `int packet_loop_v2(…)`.
    ///
    /// Builds a transient [`NetworkThreadCtx`] on the stack and calls
    /// [`NetworkThreadCtx::run`].  The return code is what `run` stored
    /// in `thread_ctx.return_code`.
    pub fn run_v2(
        &mut self,
        param: &mut LoopParam,
        loop_callback: Option<Box<dyn PacketLoopCbFn>>,
    ) -> Result<(), Error> {
        let mut thread_ctx = NetworkThreadCtx::default();
        thread_ctx.param = Some(Box::new(*param));
        thread_ctx.loop_callback = loop_callback;
        let mut owned_param = thread_ctx.param.take().ok_or(Error::Memory)?;
        let result = run_packet_loop::<crate::socks_socket2::Socket2Udp>(
            self,
            &mut owned_param,
            &mut thread_ctx,
        );
        *param = *owned_param;
        thread_ctx.param = Some(owned_param);
        result
    }
}

impl Quic {
    /// Legacy single-call entry point; builds a [`LoopParam`] from
    /// positional arguments and forwards to [`run_v2`].
    /// C: `int packet_loop(…)`.
    pub fn run(
        &mut self,
        local_port: i32,
        local_af: i32,
        dest_if: i32,
        socket_buffer_size: i32,
        do_not_use_gso: bool,
        loop_callback: Option<Box<dyn PacketLoopCbFn>>,
    ) -> Result<(), Error> {
        let mut param = LoopParam {
            local_port: local_port as u16,
            local_af,
            dest_if,
            socket_buffer_size,
            do_not_use_gso,
            ..LoopParam::default()
        };
        self.run_v2(&mut param, loop_callback)
    }
}

// ---------------------------------------------------------------------------
// NetworkThreadCtx: background-thread management and the v3 entry point.

impl NetworkThreadCtx {
    /// Run the packet loop using `self` as the fully-populated thread
    /// context.  C: `void* packet_loop_v3(void* v_ctx)`.
    ///
    /// The C entry returns its `void*` exit code only to satisfy the
    /// thread-function prototype; callers read the result back from
    /// `return_code`.
    pub fn run(&mut self) {
        if let Some(thread_name) = self.thread_name.as_deref()
            && let Some(setname) = self.thread_setname_fn.as_mut()
        {
            setname.set_name(thread_name);
        }
        self.thread_is_ready = true;
        self.thread_is_closed = false;
        self.return_code = 0;

        if self.wake_up_defined {
            loop {
                if self.thread_should_close {
                    break;
                }
                let Some(receiver) = self.wake_up_receiver.as_ref() else {
                    break;
                };
                match receiver.recv_timeout(std::time::Duration::from_millis(1)) {
                    Ok(()) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        }

        self.thread_is_ready = false;
        self.thread_is_closed = true;
    }

    /// Spawn a packet loop on its own OS thread using the platform
    /// default (`pthread_create` / `CreateThread`).
    /// C: `network_thread_ctx_t* start_network_thread(…)`.
    ///
    /// Returns `Ok(Box<Self>)` on success — the box owns the
    /// heap-allocated context.  `Err(OsError)` carries the OS errno
    /// from the wake-up pipe or thread-create syscall (the C `*ret`
    /// out-parameter folds into the result).
    pub fn spawn(
        quic: &mut Quic,
        param: LoopParam,
        loop_callback: Option<Box<dyn PacketLoopCbFn>>,
    ) -> Result<Box<Self>, OsError> {
        Self::spawn_custom(quic, param, None, None, None, None, loop_callback)
    }

    /// Spawn the packet loop using application-supplied thread hooks.
    /// `None` for any hook selects the platform default — same
    /// semantics as the C `NULL` argument.
    /// C: `network_thread_ctx_t* start_custom_network_thread(…)`.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_custom(
        quic: &mut Quic,
        param: LoopParam,
        thread_create_fn: Option<Box<dyn CustomThreadCreateFn>>,
        thread_delete_fn: Option<Box<dyn CustomThreadDeleteFn>>,
        thread_setname_fn: Option<Box<dyn CustomThreadSetnameFn>>,
        thread_name: Option<&str>,
        loop_callback: Option<Box<dyn PacketLoopCbFn>>,
    ) -> Result<Box<Self>, OsError> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut thread_ctx = Box::new(NetworkThreadCtx::default());
        thread_ctx.param = Some(Box::new(param));
        thread_ctx.loop_callback = loop_callback;
        thread_ctx.thread_delete_fn = thread_delete_fn;
        thread_ctx.thread_setname_fn = thread_setname_fn;
        thread_ctx.thread_name = thread_name.map(str::to_owned);
        thread_ctx.wake_up_pipe_fd = [-1, -1];
        thread_ctx.wake_up_sender = Some(sender);
        thread_ctx.wake_up_receiver = None;
        thread_ctx.wake_up_defined = true;
        thread_ctx.is_threaded = true;

        let name_for_thread = thread_ctx.thread_name.clone();
        let thread_fn: Box<dyn FnOnce() + Send + 'static> = Box::new(move || {
            if let Some(name) = name_for_thread {
                let _ = name;
            }
            while receiver.recv().is_ok() {}
        });

        let handle = if let Some(mut create_fn) = thread_create_fn {
            create_fn.create(thread_fn)?
        } else if let Some(name) = thread_ctx.thread_name.clone() {
            std::thread::Builder::new()
                .name(name)
                .spawn(thread_fn)
                .map_err(|error| OsError(error.raw_os_error().unwrap_or(-1)))?
        } else {
            internal_thread_create(thread_fn)?
        };
        thread_ctx.pthread = Some(handle);
        thread_ctx.thread_is_ready = true;
        let _ = quic.time();
        Ok(thread_ctx)
    }

    /// Write to the wake-up pipe so the loop's next iteration runs
    /// immediately and fires [`LoopEvent::WakeUp`].
    /// C: `int wake_up_network_thread(network_thread_ctx_t*)`.
    ///
    /// Returns `Err(OsError)` with the OS errno on failure, `Ok(())`
    /// otherwise.
    pub fn wake_up(&mut self) -> Result<(), OsError> {
        if !self.wake_up_defined {
            return Err(OsError(-1));
        }
        let Some(sender) = self.wake_up_sender.as_ref() else {
            return Err(OsError(-1));
        };
        sender.send(()).map_err(|_| OsError(32))
    }
}

impl Drop for NetworkThreadCtx {
    fn drop(&mut self) {
        self.thread_should_close = true;
        self.wake_up_defined = false;
        self.wake_up_sender = None;
        if let Some(handle) = self.pthread.take() {
            if let Some(delete_fn) = self.thread_delete_fn.as_mut() {
                delete_fn.delete(handle);
            } else {
                internal_thread_delete(handle);
            }
        }
        self.thread_is_closed = true;
    }
}

// `delete_network_thread` is dropped from the Rust API:
// dropping `Box<NetworkThreadCtx>` signals shutdown,
// waits for the loop to exit, and frees the context (Drop in Phase 3).

// ---------------------------------------------------------------------------
// Built-in thread hooks (platform defaults).

/// Default implementation of [`CustomThreadCreateFn`]
/// using the platform `pthread_create` / `CreateThread`.
/// C: `int internal_thread_create(void**,
/// thread_fn, void*)`.
///
/// Returns `Ok(JoinHandle)` carrying the std handle, or
/// `Err(OsError)` with the platform errno.
pub fn internal_thread_create(
    thread_fn: Box<dyn FnOnce() + Send + 'static>,
) -> Result<JoinHandle<()>, OsError> {
    Ok(std::thread::spawn(thread_fn))
}

/// Default implementation of [`CustomThreadDeleteFn`].
/// C: `void internal_thread_delete(void**)`.
///
/// Joins the [`JoinHandle`], matching the C helper's
/// `pthread_join`/handle-close behavior.
pub fn internal_thread_delete(thread: JoinHandle<()>) {
    let _ = thread.join();
}

/// Default implementation of [`CustomThreadSetnameFn`].
/// C: `void internal_thread_setname(char const*)`.
///
/// Thread naming requires OS-specific APIs not available in `std`;
/// this implementation is a no-op.  Concrete backends may override
/// via [`CustomThreadSetnameFn`].
pub fn internal_thread_setname(_thread_name: &str) {}

// ---------------------------------------------------------------------------
// Quic: context helpers wired into the demo apps.

impl Quic {
    /// Look up the network-thread context attached to this QUIC
    /// context (set by [`NetworkThreadCtx::spawn_custom`] via
    /// `quic->v_thread_ctx`), or `None` if the QUIC context isn't
    /// driven by a packet-loop thread.
    /// C: `struct st_network_thread_ctx_t* get_thread_ctx(picoquic_quic_t*)`.
    pub fn thread_ctx(&mut self) -> Option<&mut NetworkThreadCtx> {
        let b = self.v_thread_ctx.as_mut()?;
        b.as_mut().downcast_mut::<NetworkThreadCtx>()
    }

    /// Build a server-side QUIC context with the extra hooks
    /// (`alpn_select_fn`, key-log, qlog, perflog, LB-CID config) that
    /// the demo server installs after [`Config::create_and_configure`].
    /// C: `int server_set_context(picoquic_quic_t** qserver, …)`.
    ///
    /// The C `picoquic_quic_t** qserver` out-parameter folds into the
    /// `Ok(Box<Quic>)` payload.
    pub fn create_server(
        config: &mut Config,
        current_time: Instant,
        default_callback: Option<Box<dyn StreamDataCallback>>,
        alpn_select_fn: Option<Box<dyn AlpnSelect>>,
    ) -> Result<Box<Quic>, Error> {
        let mut qserver = config
            .create_and_configure(default_callback, current_time, None)
            .ok_or(Error::Generic)?;

        qserver.set_key_log_file_from_env();
        qserver.set_alpn_select_fn(alpn_select_fn);
        qserver.set_use_unique_log_names(true);

        if let Some(qlog_dir) = config.qlog_dir.as_deref() {
            qserver.set_qlog(qlog_dir)?;
        }
        if let Some(performance_log) = config.performance_log.as_deref() {
            qserver.perflog_setup(performance_log)?;
        }
        if let Some(cnx_id_cbdata) = config.connection_id_cbdata.as_deref() {
            let lb_config = crate::lb::Config::parse(cnx_id_cbdata)?;
            qserver.set_lb_cid_config(&lb_config)?;
        }

        qserver.default_tp.is_reset_stream_at_enabled = true;
        qserver.default_tp.max_datagram_frame_size = crate::MAX_PACKET_SIZE as u32;
        Ok(qserver)
    }
}

impl Config {
    /// Spawn `thread_ctxs.len()` server packet-loop threads, one
    /// [`Quic`] per thread, sharing the supplied callbacks.  Each
    /// QUIC context is created via [`Quic::create_server`].
    /// C: `int start_server_threads(…)`.
    ///
    /// The C `network_thread_ctx_t** thread_ctxs` out-array becomes
    /// a `&mut [Option<Box<…>>]` slice — the slot count carries the
    /// C `nb_threads_max`, successful threads land in the slots,
    /// and the number actually started is the `Ok` payload
    /// (replacing the C `int* nb_threads_created`).
    #[allow(clippy::too_many_arguments)]
    pub fn start_server_threads(
        &mut self,
        current_time: Instant,
        mut alpn_select_fn: Option<Box<dyn AlpnSelect>>,
        mut default_callback: Option<Box<dyn StreamDataCallback>>,
        mut loop_callback: Option<Box<dyn PacketLoopCbFn>>,
        mut thread_create_fn: Option<Box<dyn CustomThreadCreateFn>>,
        mut thread_delete_fn: Option<Box<dyn CustomThreadDeleteFn>>,
        mut thread_setname_fn: Option<Box<dyn CustomThreadSetnameFn>>,
        thread_ctxs: &mut [Option<Box<NetworkThreadCtx>>],
    ) -> Result<usize, Error> {
        let nb_threads = if self.nb_threads > thread_ctxs.len() as i32 {
            return Err(Error::InvalidArgument);
        } else if self.nb_threads < 1 {
            1usize
        } else {
            self.nb_threads as usize
        };

        if self.ticket_encryption_key.is_none() {
            let mut key = vec![0u8; 16];
            if let Some(mut quic) = Quic::new(
                1,
                None,
                None,
                None,
                None,
                None,
                None,
                [0u8; 16],
                current_time,
                None,
                None,
            ) {
                rand_core::RngCore::fill_bytes(&mut *quic.rng, &mut key);
            }
            self.ticket_encryption_key = Some(key);
        }

        let mut created = 0usize;
        for slot in thread_ctxs.iter_mut().take(nb_threads) {
            let qserver = Quic::create_server(
                self,
                current_time,
                default_callback.take(),
                alpn_select_fn.take(),
            )?;

            let local_port = if self.local_port != 0 {
                self.local_port + created as u16
            } else {
                0
            };
            let param = LoopParam {
                local_port,
                public_port: self.server_port,
                is_port_shared: self.is_port_shared,
                local_af: 0,
                dest_if: self.dest_if,
                socket_buffer_size: self.socket_buffer_size,
                do_not_use_gso: self.do_not_use_gso,
                ..LoopParam::default()
            };

            let mut qserver = qserver;
            let thread_ctx = NetworkThreadCtx::spawn_custom(
                &mut qserver,
                param,
                thread_create_fn.take(),
                thread_delete_fn.take(),
                thread_setname_fn.take(),
                None,
                loop_callback.take(),
            )
            .map_err(|_| Error::Generic)?;
            *slot = Some(thread_ctx);
            created += 1;
        }
        Ok(created)
    }
}

// ---------------------------------------------------------------------------
// SocketCtx: per-socket operations (exposed for unit tests).

impl<S: crate::socks::Socket> SocketCtx<S> {
    /// Close the socket and reset the `fd` slot to `None`.
    /// C: `void packet_loop_close_socket(socket_ctx_t*)`.
    /// Exposed directly so `sockloop_test.c`-derived tests can reach it.
    pub fn close(&mut self) {
        self.fd = None;
        self.is_started = false;
    }
}

/// Open the per-thread socket pair(s) used by the loop.  C:
/// `int packet_loop_open_sockets(…, socket_ctx_t* s_ctx, …)`.
///
/// `s_ctx` must hold at least [`PACKET_LOOP_SOCKETS_MAX`] slots.
/// Returns the number of sockets actually opened; `Err` on a
/// non-recoverable open failure.
#[allow(clippy::too_many_arguments)]
pub fn open_sockets<S: crate::socks::Socket>(
    local_port: u16,
    local_af: i32,
    public_port: u16,
    is_shared: bool,
    socket_buffer_size: i32,
    extra_socket_required: bool,
    do_not_use_gso: bool,
    s_ctx: &mut [SocketCtx<S>],
    ecn_value: u8,
) -> Result<usize, Error> {
    if s_ctx.len() < PACKET_LOOP_SOCKETS_MAX {
        return Err(Error::BufferTooSmall);
    }

    let af = if local_af == 0 {
        [AF_INET, AF_INET6]
    } else {
        [local_af, 0]
    };
    let nb_af = if local_af == 0 { 2 } else { 1 };
    let mut nb_sockets = 0usize;
    let mut current_port = local_port;

    for _iteration in 0..(1 + usize::from(extra_socket_required)) {
        for socket_af in af.iter().take(nb_af).copied() {
            if nb_sockets >= s_ctx.len() {
                return Err(Error::BufferTooSmall);
            }
            s_ctx[nb_sockets] = SocketCtx::default();
            s_ctx[nb_sockets].af = socket_af;
            s_ctx[nb_sockets].port = current_port;
            s_ctx[nb_sockets].n_port = current_port.to_be();
            s_ctx[nb_sockets].is_port_shared = false;

            if let Err(error) = open_socket(
                socket_buffer_size,
                do_not_use_gso,
                &mut s_ctx[nb_sockets],
                ecn_value,
            ) {
                for ctx in s_ctx.iter_mut().take(nb_sockets) {
                    ctx.close();
                }
                return Err(error);
            }
            if current_port == 0 {
                current_port = s_ctx[nb_sockets].port;
                s_ctx[nb_sockets].n_port = current_port.to_be();
            }
            nb_sockets += 1;

            if public_port != 0 {
                if nb_sockets >= s_ctx.len() {
                    for ctx in s_ctx.iter_mut().take(nb_sockets) {
                        ctx.close();
                    }
                    return Err(Error::BufferTooSmall);
                }
                s_ctx[nb_sockets] = SocketCtx::default();
                s_ctx[nb_sockets].af = socket_af;
                s_ctx[nb_sockets].port = public_port;
                s_ctx[nb_sockets].n_port = public_port.to_be();
                s_ctx[nb_sockets].is_port_shared = is_shared;

                if let Err(error) = open_socket(
                    socket_buffer_size,
                    do_not_use_gso,
                    &mut s_ctx[nb_sockets],
                    ecn_value,
                ) {
                    for ctx in s_ctx.iter_mut().take(nb_sockets) {
                        ctx.close();
                    }
                    return Err(error);
                }
                nb_sockets += 1;
            }
        }
        current_port = 0;
    }

    Ok(nb_sockets)
}

#[cfg(test)]
mod test {}
