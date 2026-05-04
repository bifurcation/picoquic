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
//! Phase 1 contract: signatures only — every function body is
//! `todo!()` and the empty `#[cfg(test)] mod test {}` lands at the
//! bottom for Phase 2 to fill.
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
use crate::socks::OsError;
use crate::{AlpnSelect, Quic, StreamDataCallback};

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
/// `volatile` qualifiers in the C source are dropped — `Send`/`Sync`
/// is out of v1 scope and the thread-shutdown handshake uses these
/// flags through the same single-threaded code path the rest of the
/// crate assumes.
pub struct NetworkThreadCtx {
    // The C field `picoquic_quic_t* quic` is gone.  v1 is
    // single-threaded (per `TRANSLATE_PLAN.md`), so the loop's
    // entry points (`Quic::run_loop` etc.) take `&mut Quic`
    // explicitly rather than threading a back-pointer through
    // this struct.  v2 will reintroduce a typed cross-thread
    // handle (`Arc<Mutex<Quic>>` or whatever the threading model
    // ends up using); the placeholder here is intentionally
    // absent.
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
            is_threaded: false,
            wake_up_defined: false,
            thread_is_ready: false,
            thread_should_close: false,
            thread_is_closed: false,
            return_code: 0,
        }
    }
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
        _param: &mut LoopParam,
        _loop_callback: Option<Box<dyn PacketLoopCbFn>>,
    ) -> Result<(), Error> {
        todo!()
    }
}

impl Quic {
    /// Legacy single-call entry point; builds a [`LoopParam`] from
    /// positional arguments and forwards to [`run_v2`].
    /// C: `int packet_loop(…)`.
    pub fn run(
        &mut self,
        _local_port: i32,
        _local_af: i32,
        _dest_if: i32,
        _socket_buffer_size: i32,
        _do_not_use_gso: bool,
        _loop_callback: Option<Box<dyn PacketLoopCbFn>>,
    ) -> Result<(), Error> {
        todo!()
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
        todo!()
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
        _quic: &mut Quic,
        _param: LoopParam,
        _loop_callback: Option<Box<dyn PacketLoopCbFn>>,
    ) -> Result<Box<Self>, OsError> {
        todo!()
    }

    /// Spawn the packet loop using application-supplied thread hooks.
    /// `None` for any hook selects the platform default — same
    /// semantics as the C `NULL` argument.
    /// C: `network_thread_ctx_t* start_custom_network_thread(…)`.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_custom(
        _quic: &mut Quic,
        _param: LoopParam,
        _thread_create_fn: Option<Box<dyn CustomThreadCreateFn>>,
        _thread_delete_fn: Option<Box<dyn CustomThreadDeleteFn>>,
        _thread_setname_fn: Option<Box<dyn CustomThreadSetnameFn>>,
        _thread_name: Option<&str>,
        _loop_callback: Option<Box<dyn PacketLoopCbFn>>,
    ) -> Result<Box<Self>, OsError> {
        todo!()
    }

    /// Write to the wake-up pipe so the loop's next iteration runs
    /// immediately and fires [`LoopEvent::WakeUp`].
    /// C: `int wake_up_network_thread(network_thread_ctx_t*)`.
    ///
    /// Returns `Err(OsError)` with the OS errno on failure, `Ok(())`
    /// otherwise.
    pub fn wake_up(&mut self) -> Result<(), OsError> {
        todo!()
    }
}

// `delete_network_thread` is dropped from the Rust API:
// `Box<NetworkThreadCtx>` going out of scope signals shutdown,
// waits for the loop to exit, and frees the context (Drop in Phase 3).

// ---------------------------------------------------------------------------
// Built-in thread hooks (platform defaults).

/// Default implementation of [`CustomThreadCreateFn`]
/// using the platform `pthread_create` / `CreateThread`.
/// C: `int internal_thread_create(void**,
/// thread_fn, void*)`.
///
/// Returns `Ok(JoinHandle)` carrying the std handle, or
/// `Err(OsError)` with the platform errno.  Threading is dropped
/// from v1 (`TRANSLATE_PLAN.md`) so the body is a `todo!()`
/// placeholder that lands when v2 multi-threading work begins.
pub fn internal_thread_create(
    _thread_fn: Box<dyn FnOnce() + Send + 'static>,
) -> Result<JoinHandle<()>, OsError> {
    todo!()
}

/// Default implementation of [`CustomThreadDeleteFn`].
/// C: `void internal_thread_delete(void**)`.
///
/// In Rust this is a no-op — dropping the [`JoinHandle`] detaches
/// the thread; if the caller wants to wait for completion they
/// call `join()` themselves first.
pub fn internal_thread_delete(_thread: JoinHandle<()>) {
    todo!()
}

/// Default implementation of [`CustomThreadSetnameFn`].
/// C: `void internal_thread_setname(char const*)`.
pub fn internal_thread_setname(_thread_name: &str) {
    todo!()
}

// ---------------------------------------------------------------------------
// Quic: context helpers wired into the demo apps.

impl Quic {
    /// Look up the network-thread context attached to this QUIC
    /// context (set by [`NetworkThreadCtx::spawn_custom`] via
    /// `quic->v_thread_ctx`), or `None` if the QUIC context isn't
    /// driven by a packet-loop thread.
    /// C: `struct st_network_thread_ctx_t* get_thread_ctx(picoquic_quic_t*)`.
    pub fn thread_ctx(&mut self) -> Option<&mut NetworkThreadCtx> {
        todo!()
    }

    /// Build a server-side QUIC context with the extra hooks
    /// (`alpn_select_fn`, key-log, qlog, perflog, LB-CID config) that
    /// the demo server installs after [`Config::create_and_configure`].
    /// C: `int server_set_context(picoquic_quic_t** qserver, …)`.
    ///
    /// The C `picoquic_quic_t** qserver` out-parameter folds into the
    /// `Ok(Box<Quic>)` payload.
    pub fn create_server(
        _config: &mut Config,
        _current_time: Instant,
        _default_callback: Option<Box<dyn StreamDataCallback>>,
        _alpn_select_fn: Option<Box<dyn AlpnSelect>>,
    ) -> Result<Box<Quic>, Error> {
        todo!()
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
        _current_time: Instant,
        _alpn_select_fn: Option<Box<dyn AlpnSelect>>,
        _default_callback: Option<Box<dyn StreamDataCallback>>,
        _loop_callback: Option<Box<dyn PacketLoopCbFn>>,
        _thread_create_fn: Option<Box<dyn CustomThreadCreateFn>>,
        _thread_delete_fn: Option<Box<dyn CustomThreadDeleteFn>>,
        _thread_setname_fn: Option<Box<dyn CustomThreadSetnameFn>>,
        _thread_ctxs: &mut [Option<Box<NetworkThreadCtx>>],
    ) -> Result<usize, Error> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// SocketCtx: per-socket operations (exposed for unit tests).

impl<S: crate::socks::Socket> SocketCtx<S> {
    /// Close the socket and reset the `fd` slot to `None`.
    /// C: `void packet_loop_close_socket(socket_ctx_t*)`.
    /// Exposed directly so `sockloop_test.c`-derived tests can reach it.
    pub fn close(&mut self) {
        todo!()
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
    _local_port: u16,
    _local_af: i32,
    _public_port: u16,
    _is_shared: bool,
    _socket_buffer_size: i32,
    _extra_socket_required: bool,
    _do_not_use_gso: bool,
    _s_ctx: &mut [SocketCtx<S>],
    _ecn_value: u8,
) -> Result<usize, Error> {
    todo!()
}

#[cfg(test)]
mod test {}
