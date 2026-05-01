//! Translation of `picoquic/picoquic_packet_loop.h`.
//!
//! Bundle of types and entry points implementing picoquic's
//! reference event loop (`sockloop.c`) on top of the [`picosocks`]
//! UDP shims.  The loop:
//!
//! * owns one or more [`picoquic_socket_ctx_t`] per network family,
//! * polls them with [`picosocks::picoquic_select`] (or `select`/
//!   `WSAWaitForMultipleEvents` in the C original),
//! * routes received datagrams into picoquic, and
//! * fires application callbacks ([`PicoquicPacketLoopCbFn`]) at
//!   well-defined points (loop ready, after recv, after send,
//!   address change, time check, system-call duration spike, wake-up,
//!   alt-port).
//!
//! Three layered entry points cover the same loop with different
//! parameter shapes:
//!
//! * [`picoquic_packet_loop`] — the original 1.x signature,
//!   preserved for compatibility.  Builds a [`picoquic_packet_loop_param_t`]
//!   on the stack and forwards.
//! * [`picoquic_packet_loop_v2`] — takes a [`picoquic_packet_loop_param_t`]
//!   directly.
//! * [`picoquic_packet_loop_v3`] — takes a fully-populated
//!   [`picoquic_network_thread_ctx_t`]; the threaded entry point.
//!
//! Plus the threading helpers ([`picoquic_start_network_thread`] et
//! al.) that wrap `pthread_create` / `CreateThread`.
//!
//! Phase 1 contract: signatures only — every function body is
//! `todo!()` and the empty `#[cfg(test)] mod test {}` lands at the
//! bottom for Phase 2 to fill.
//!
//! [`picosocks`]: crate::picoquic::picosocks
//!
//! ## Pointer-shape decisions
//!
//! Read from `picoquic/sockloop.c`, `picoquic/winsockloop.c`, and
//! the demo callers in `picoquicfirst/picoquicdemo.c`,
//! `sample/sample_*.c`, and `picoquictest/sockloop_test.c`:
//!
//! * `picoquic_quic_t*` parameters are non-null in every observed
//!   caller — they translate to `&mut picoquic_quic_t`.
//! * `picoquic_packet_loop_param_t*` is *borrowed* by the loop
//!   helpers (the caller stack-allocates it in
//!   `picoquic_packet_loop_v2` / the demo apps) — `&mut` here.  Inside
//!   [`picoquic_network_thread_ctx_t`] the same pointer is *owned*
//!   when `is_param_allocated == 1` (`picoquic_start_server_threads`
//!   `malloc`s a copy and `picoquic_delete_network_thread` `free`s
//!   it).  Phase 1 keeps the field as `Option<Box<…>>` and folds
//!   the C `is_param_allocated` flag away — the `Box` itself
//!   carries ownership; borrowed-param call sites store `None`
//!   plus a separate borrow at runtime.
//! * `picoquic_quic_config_t*` is borrowed-mut by
//!   [`picoquic_server_set_context`] and [`picoquic_start_server_threads`]
//!   (they read and may mutate fields like `ticket_encryption_key`).
//! * Function-pointer typedefs (`picoquic_packet_loop_cb_fn`,
//!   `picoquic_custom_thread_create_fn`, `_setname_fn`,
//!   `_delete_fn`) become traits per the Phase 1 rules; the C `void*
//!   callback_ctx` companion folds into the trait implementor's
//!   state.  Per-event `void* callback_argv` stays a raw pointer —
//!   its concrete type depends on `cb_mode` (see
//!   [`picoquic_packet_loop_cb_enum`]) and a tagged enum would
//!   diverge from C source structure; Phase 3 may revisit.
//! * `picoquic_socket_ctx_t* s_ctx` is used as both a single object
//!   ([`picoquic_packet_loop_close_socket`]) and a fixed-size array
//!   ([`picoquic_packet_loop_open_sockets`] writes up to
//!   `PICOQUIC_PACKET_LOOP_SOCKETS_MAX` entries).  The single-object
//!   path takes `&mut picoquic_socket_ctx_t`; the array path takes
//!   `&mut [picoquic_socket_ctx_t]`, with the slice length subsuming
//!   the C convention of "callee writes and returns the count".
//! * `picoquic_network_thread_ctx_t**` outputs from
//!   [`picoquic_start_server_threads`] become `&mut [Option<Box<…>>]`
//!   — the slice carries `nb_threads_max`; each slot is filled with
//!   `Some` on success.
//! * Bitfields collapse to `bool`s.  Single-bit C `int : 1` /
//!   `unsigned int : 1` flag fields don't justify `bitflags!` on a
//!   set of unrelated booleans (see `picoquic_config.rs` for the
//!   established convention).
//! * `volatile int` fields in [`picoquic_network_thread_ctx_t`]
//!   become plain `i32` / `bool` — `Send`/`Sync` is out of v1 scope,
//!   so the volatile semantics have nowhere to land.
//! * `sockaddr_storage` fields fold into `Option<SocketAddr>`,
//!   matching the [`picosocks`] convention (`AF_UNSPEC` ↔ `None`).
//! * The Windows-only fields (`overlap`, `WSARecvMsg`, `WSASendMsg`,
//!   `dataBuf`, `msg`, …) and the io_uring fields are dropped per
//!   the v1 single-target scope.
//! * `recv_buffer: uint8_t* + recv_buffer_size: size_t` is an
//!   owning pair (allocated in `picoquic_packet_set_windows_socket`
//!   on the Windows path, or `picoquic_packet_loop_recv_buf_uring_init`
//!   on io_uring; freed in [`picoquic_packet_loop_close_socket`]).
//!   On the canonical Linux/`select` build neither is used — every
//!   datagram lands in the shared loop buffer.  Phase 1 keeps the
//!   field as `Option<Box<[u8]>>` so the unused state is just
//!   `None`; Phase 3 will tighten once the loop is implemented.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
// Stand-in for the not-yet-defined crate-level `Error` enum.
#![allow(clippy::result_unit_err)]
// Mirroring C parameter lists for the threaded entry points.
#![allow(clippy::too_many_arguments)]
// `Box<T>` parameters look local-only to clippy because the Phase
// 1 bodies are `todo!()`; the owning shape is real once Phase 3
// fills the bodies (`Box` lands inside the thread ctx, the loop
// param is owned-or-borrowed depending on `is_param_allocated`).
#![allow(clippy::boxed_local)]

use core::ffi::c_void;
use core::net::SocketAddr;

use crate::picoquic::picoquic::{
    picoquic_alpn_select_fn_v2, picoquic_quic_t, picoquic_stream_data_cb_fn,
};
use crate::picoquic::picoquic_config::picoquic_quic_config_t;
use crate::picoquic::picoquic_utils::{PicoquicThreadFn, picoquic_thread_t};

// ---------------------------------------------------------------------------
// Compile-time limits.

/// Maximum number of UDP sockets a single packet loop can drive at
/// once (one per address family, optionally times two for the
/// public/private port split).  C: `PICOQUIC_PACKET_LOOP_SOCKETS_MAX`.
pub const PICOQUIC_PACKET_LOOP_SOCKETS_MAX: usize = 4;

/// Maximum datagrams pulled from one socket per loop iteration.
/// C: `PICOQUIC_PACKET_LOOP_RECV_MAX`.
pub const PICOQUIC_PACKET_LOOP_RECV_MAX: usize = 10;

/// Maximum datagrams pushed to one socket per loop iteration.
/// C: `PICOQUIC_PACKET_LOOP_SEND_MAX`.
pub const PICOQUIC_PACKET_LOOP_SEND_MAX: usize = 10;

/// Hard cap on how long the loop will wait between iterations
/// (microseconds).  C: `PICOQUIC_PACKET_LOOP_SEND_DELAY_MAX`.
pub const PICOQUIC_PACKET_LOOP_SEND_DELAY_MAX: u64 = 2500;

// ---------------------------------------------------------------------------
// Per-socket context.

/// State the loop maintains per UDP socket.  C:
/// `picoquic_socket_ctx_t`.
///
/// Trimmed to the canonical Linux build: the Windows
/// (`WSAOVERLAPPED`, `WSARecvMsg`, …) and io_uring (`msghdr`,
/// `iovec`, `ctrl_buffer`) fields are dropped per the v1
/// single-target scope.  The four C bitfields collapse to plain
/// `bool` flags.
pub struct picoquic_socket_ctx_t {
    /// OS file descriptor.  C: `SOCKET_TYPE fd`.
    pub fd: crate::picoquic::picosocks::picoquic_socket_t,
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
    /// Whether [`picoquic_packet_loop_open_socket`]-equivalent setup
    /// has completed for this socket.  C: `unsigned int is_started : 1`.
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
    /// Number of bytes read into the loop buffer.
    /// C: `int bytes_recv` (`-1` on error; the Rust loop uses a
    /// `Result` at the call site, so this stays a plain `i32`).
    pub bytes_recv: i32,
    /// Scratch space for assembling outbound `cmsg` payloads
    /// (`IP_PKTINFO`, `IPV6_PKTINFO`, `UDP_SEGMENT`).
    /// C: `char cmsg_buffer[1024]`.
    pub cmsg_buffer: [u8; 1024],
    /// `UDP_GRO` segment size on the most recent datagram (the
    /// kernel's `UDP_GRO` cmsg).  C: `size_t udp_coalesced_size`.
    pub udp_coalesced_size: usize,
}

impl Default for picoquic_socket_ctx_t {
    fn default() -> Self {
        picoquic_socket_ctx_t {
            fd: crate::picoquic::picosocks::INVALID_SOCKET,
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

/// Tag identifying which event fired the
/// [`PicoquicPacketLoopCbFn`] callback.  The expected type of
/// the `callback_argv` payload is documented per-variant.
/// C: `picoquic_packet_loop_cb_enum`.
///
/// `repr(C)` is dropped — variants are inspected only through Rust
/// pattern matching, never through FFI.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum picoquic_packet_loop_cb_enum {
    /// Loop has finished initializing.  `callback_argv:
    /// *mut picoquic_packet_loop_options_t`.
    /// C: `picoquic_packet_loop_ready`.
    Ready = 0,
    /// `callback_argv: *mut size_t` — number of packets received this
    /// iteration.  C: `picoquic_packet_loop_after_receive`.
    AfterReceive,
    /// `callback_argv: *mut size_t` — number of packets sent this
    /// iteration.  C: `picoquic_packet_loop_after_send`.
    AfterSend,
    /// `callback_argv: *mut sockaddr` — new local address advertised
    /// by the application after a port update.
    /// C: `picoquic_packet_loop_port_update`.
    PortUpdate,
    /// `callback_argv: *mut packet_loop_time_check_arg_t`.  Optional;
    /// only fires when the application set
    /// [`picoquic_packet_loop_options_t::do_time_check`].
    /// C: `picoquic_packet_loop_time_check`.
    TimeCheck,
    /// `callback_argv: *mut packet_loop_system_call_duration_t`.
    /// Optional; only fires when the application set
    /// [`picoquic_packet_loop_options_t::do_system_call_duration`].
    /// C: `picoquic_packet_loop_system_call_duration`.
    SystemCallDuration,
    /// Wake-up event triggered by [`picoquic_wake_up_network_thread`].
    /// `callback_argv: NULL`.
    /// C: `picoquic_packet_loop_wake_up`.
    WakeUp,
    /// `callback_argv: *mut sockaddr` — alt-port address surfaced for
    /// multipath / migration tests.
    /// C: `picoquic_packet_loop_alt_port`.
    AltPort,
}

/// System-call duration statistics surfaced through the optional
/// [`picoquic_packet_loop_cb_enum::picoquic_packet_loop_system_call_duration`]
/// callback.  C: `packet_loop_system_call_duration_t`.
#[derive(Debug, Default, Copy, Clone)]
pub struct packet_loop_system_call_duration_t {
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

/// In/out argument for the
/// [`picoquic_packet_loop_cb_enum::picoquic_packet_loop_time_check`]
/// callback.  The loop fills `current_time` and a proposed
/// `delta_t`; the application overwrites `delta_t` with a
/// (possibly smaller) value if it has work to do sooner.
/// C: `packet_loop_time_check_arg_t`.
#[derive(Debug, Default, Copy, Clone)]
pub struct packet_loop_time_check_arg_t {
    /// Loop-time timestamp (microseconds since process start).
    pub current_time: u64,
    /// Proposed sleep, in microseconds — application may shrink.
    pub delta_t: i64,
}

/// Application callback fired at the lifecycle and per-iteration
/// events listed in [`picoquic_packet_loop_cb_enum`].
///
/// C: `int (*picoquic_packet_loop_cb_fn)(picoquic_quic_t* quic,
/// picoquic_packet_loop_cb_enum cb_mode, void* callback_ctx, void*
/// callback_argv)`.  The `void* callback_ctx` companion folds into
/// the trait implementor's state per the Phase 1 rules.
///
/// `callback_argv` keeps `*mut c_void` because its concrete type is
/// selected by `cb_mode` at runtime; safer reinterpretation lands
/// in Phase 3.  Implementors must consult [`picoquic_packet_loop_cb_enum`]
/// to know what to cast it to.
///
/// The return value is the C `int`: `0` for success, non-zero to
/// signal an error and break out of the loop.  Phase 3 may refine
/// to a `Result` once the crate-level `Error` enum lands.
pub trait PicoquicPacketLoopCbFn {
    /// Loop-event callback.  See trait docs for `callback_argv`
    /// dispatch.
    ///
    /// # Safety
    ///
    /// The caller (the loop) must pass an `argv` pointer that is
    /// either null or points to a value of the type associated with
    /// `cb_mode` per [`picoquic_packet_loop_cb_enum`].
    unsafe fn callback(
        &mut self,
        quic: &mut picoquic_quic_t,
        cb_mode: picoquic_packet_loop_cb_enum,
        argv: *mut c_void,
    ) -> i32;
}

// ---------------------------------------------------------------------------
// Loop options + parameters.

/// Feature-flags the application advertises in response to the
/// [`picoquic_packet_loop_cb_enum::Ready`]
/// callback.  C: `picoquic_packet_loop_options_t`, three single-bit
/// `unsigned int : 1` fields collapsed to `bool` per the
/// established convention (see `picoquic_config.rs`).
#[derive(Debug, Default, Copy, Clone)]
pub struct picoquic_packet_loop_options_t {
    /// Application wants the loop to call back with
    /// [`picoquic_packet_loop_cb_enum::TimeCheck`]
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
/// C: `picoquic_packet_loop_param_t`.  `repr(C)` is dropped — the
/// struct never crosses an external boundary.
#[derive(Debug, Default, Copy, Clone)]
pub struct picoquic_packet_loop_param_t {
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
/// `int (*picoquic_custom_thread_create_fn)(void** thread_id,
/// picoquic_thread_fn thread_fn, void* arg)`.
///
/// In C, `(thread_fn, arg)` is the trampoline + state pair the new
/// thread will run.  In Rust, the existing
/// [`PicoquicThreadFn`](crate::picoquic::picoquic_utils::PicoquicThreadFn)
/// trait already bundles both into a single trait object, so the
/// hook reduces to "produce a thread handle from a `Box<dyn …>`".
pub trait PicoquicCustomThreadCreateFn {
    /// Create a thread that runs `thread_fn` to completion.  The
    /// returned [`picoquic_thread_t`] is opaque to picoquic and is
    /// passed back through
    /// [`PicoquicCustomThreadDeleteFn::delete`] at teardown.
    ///
    /// The C contract returns `0` for success / non-zero `errno`
    /// otherwise; we keep the `i32` so call sites can surface the
    /// raw OS error via the `*ret` out-parameter on
    /// [`picoquic_start_custom_network_thread`].
    fn create(&mut self, thread_fn: Box<dyn PicoquicThreadFn>) -> Result<picoquic_thread_t, i32>;
}

/// Set-thread-name hook.  C:
/// `void (*picoquic_custom_thread_setname_fn)(char const* thread_name)`.
///
/// Length is capped at 16 bytes (including the trailing NUL) on
/// Linux's `prctl(PR_SET_NAME, …)`; the trait takes a `&str` slice
/// and lets the implementor truncate / re-encode as needed.
pub trait PicoquicCustomThreadSetnameFn {
    /// Apply `thread_name` to the *current* thread.  Called from
    /// inside the thread after it starts, per the C convention.
    fn set_name(&mut self, thread_name: &str);
}

/// Tear-down-a-thread hook.  C:
/// `void (*picoquic_custom_thread_delete_fn)(void** thread_id)`.
pub trait PicoquicCustomThreadDeleteFn {
    /// Release any resources tied to `thread`.  Called from the
    /// destruction path of [`picoquic_network_thread_ctx_t`].
    fn delete(&mut self, thread: picoquic_thread_t);
}

// ---------------------------------------------------------------------------
// Threaded packet-loop context.

/// Thread context carrying everything the v3 entry point needs to
/// run a packet loop.  C: `picoquic_network_thread_ctx_t`.
///
/// Two ownership modes coexist in C:
///
/// * Foreground / blocking (`picoquic_packet_loop_v2`): the caller
///   stack-allocates a `picoquic_network_thread_ctx_t = { 0 }` and
///   passes its address to [`picoquic_packet_loop_v3`].
///   `is_threaded` stays `0`, no wake-up plumbing is created.
/// * Background (`picoquic_start_network_thread`): the helper
///   `malloc`s the context, populates it, opens the wake-up
///   pipe / event, and launches the OS thread; teardown via
///   [`picoquic_delete_network_thread`] frees everything.
///
/// Phase 1 keeps a single struct shape with optional fields so both
/// modes fit; Phase 3 may split them or introduce a builder.
///
/// `volatile` qualifiers in the C source are dropped — `Send`/`Sync`
/// is out of v1 scope and the thread-shutdown handshake uses these
/// flags through the same single-threaded code path the rest of the
/// crate assumes.
pub struct picoquic_network_thread_ctx_t {
    /// QUIC context the loop drives.  Always set; `&mut` lifetime
    /// is expressed by the caller passing `&mut` into v3 — the
    /// owning case (background thread) keeps the QUIC context
    /// alive externally.
    pub quic: *mut picoquic_quic_t,
    /// Loop parameters, optionally owned (`is_param_allocated`).
    /// Phase 1 collapses the C borrowed-or-owned discriminant into
    /// a single `Option<Box<…>>`; the borrowed case stores `None`
    /// and the borrow lives outside this struct.
    pub param: Option<Box<picoquic_packet_loop_param_t>>,
    /// Application loop callback.  `None` matches the C `NULL`
    /// (the loop runs without notifying the app on each event —
    /// used by the bench harness).
    pub loop_callback: Option<Box<dyn PicoquicPacketLoopCbFn>>,
    /// Custom thread-deletion hook; defaults to
    /// `picoquic_internal_thread_delete` when the C caller passes
    /// `NULL`.
    pub thread_delete_fn: Option<Box<dyn PicoquicCustomThreadDeleteFn>>,
    /// Custom thread-naming hook; defaults to
    /// `picoquic_internal_thread_setname`.
    pub thread_setname_fn: Option<Box<dyn PicoquicCustomThreadSetnameFn>>,
    /// Optional thread name.  Capped at 16 bytes (including NUL)
    /// on Linux per `prctl(PR_SET_NAME, …)`.
    pub thread_name: Option<String>,
    /// Underlying OS thread handle.  `None` for the foreground v2
    /// entry path (no thread is spawned).
    pub pthread: Option<picoquic_thread_t>,
    /// Wake-up pipe (Linux: `pipe2`).  `[ -1, -1 ]` when no wake-up
    /// has been opened (`wake_up_defined == false`).  The Windows
    /// `HANDLE wake_up_event` variant is dropped per the v1 scope.
    pub wake_up_pipe_fd: [i32; 2],
    /// Whether [`picoquic_start_custom_network_thread`] actually
    /// spawned a thread (vs. the foreground path that reuses the
    /// caller's stack).  C: `int is_threaded`.
    pub is_threaded: bool,
    /// Whether [`Self::wake_up_pipe_fd`] was successfully opened.
    /// C: `int wake_up_defined`.
    pub wake_up_defined: bool,
    /// Set by the loop once it finishes initialization (after the
    /// first `picoquic_packet_loop_ready` callback).
    /// C: `volatile int thread_is_ready`.
    pub thread_is_ready: bool,
    /// Set from the outside (typically by
    /// [`picoquic_delete_network_thread`]) to ask the loop to
    /// exit.  C: `volatile int thread_should_close`.
    pub thread_should_close: bool,
    /// Set by the loop on the way out, before returning from v3.
    /// C: `volatile int thread_is_closed`.
    pub thread_is_closed: bool,
    /// Final return code from the loop body.  Read by the caller
    /// of [`picoquic_packet_loop_v2`] after v3 returns.
    /// C: `int return_code`.
    pub return_code: i32,
}

impl Default for picoquic_network_thread_ctx_t {
    fn default() -> Self {
        picoquic_network_thread_ctx_t {
            quic: core::ptr::null_mut(),
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

/// Drive the packet loop until the application or an error breaks
/// it.  C: `int picoquic_packet_loop_v2(…)`.
///
/// Builds a transient [`picoquic_network_thread_ctx_t`] on the stack
/// and forwards to [`picoquic_packet_loop_v3`].  The return code is
/// what v3 stored in `thread_ctx.return_code`.
pub fn picoquic_packet_loop_v2(
    _quic: &mut picoquic_quic_t,
    _param: &mut picoquic_packet_loop_param_t,
    _loop_callback: Option<Box<dyn PicoquicPacketLoopCbFn>>,
) -> Result<(), ()> {
    todo!()
}

/// Run the packet loop using a fully-populated thread context.
/// C: `void* picoquic_packet_loop_v3(void* v_ctx)` (Linux) /
/// `DWORD WINAPI picoquic_packet_loop_v3(LPVOID v_ctx)` (Windows).
///
/// The C entry returns its `void*` exit code only to satisfy the
/// thread-function prototype; callers always read the result back
/// from `thread_ctx.return_code`.  We therefore drop the return and
/// leave the result in [`picoquic_network_thread_ctx_t::return_code`].
pub fn picoquic_packet_loop_v3(_thread_ctx: &mut picoquic_network_thread_ctx_t) {
    todo!()
}

/// Legacy single-call entry point; builds a
/// [`picoquic_packet_loop_param_t`] from positional arguments and
/// forwards to [`picoquic_packet_loop_v2`].  C:
/// `int picoquic_packet_loop(…)`.
pub fn picoquic_packet_loop(
    _quic: &mut picoquic_quic_t,
    _local_port: i32,
    _local_af: i32,
    _dest_if: i32,
    _socket_buffer_size: i32,
    _do_not_use_gso: bool,
    _loop_callback: Option<Box<dyn PicoquicPacketLoopCbFn>>,
) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Background-thread management.

/// Spawn a packet loop on its own OS thread using the platform
/// default (`pthread_create` / `CreateThread`).  C:
/// `picoquic_network_thread_ctx_t* picoquic_start_network_thread(…)`.
///
/// Returns `Ok(Box<…>)` on success — the box owns the heap-allocated
/// thread context that C `malloc`'d.  `Err(i32)` carries the OS
/// error from the wake-up pipe or thread-create syscall, matching
/// the C `*ret` out-parameter (the `int* ret` argument folds into
/// the result).
pub fn picoquic_start_network_thread(
    _quic: &mut picoquic_quic_t,
    _param: Box<picoquic_packet_loop_param_t>,
    _loop_callback: Option<Box<dyn PicoquicPacketLoopCbFn>>,
) -> Result<Box<picoquic_network_thread_ctx_t>, i32> {
    todo!()
}

/// Spawn the packet loop using application-supplied thread hooks.
/// `None` for any of the hooks selects the platform default — same
/// semantics as the C `NULL` argument.  C:
/// `picoquic_network_thread_ctx_t* picoquic_start_custom_network_thread(…)`.
pub fn picoquic_start_custom_network_thread(
    _quic: &mut picoquic_quic_t,
    _param: Box<picoquic_packet_loop_param_t>,
    _thread_create_fn: Option<Box<dyn PicoquicCustomThreadCreateFn>>,
    _thread_delete_fn: Option<Box<dyn PicoquicCustomThreadDeleteFn>>,
    _thread_setname_fn: Option<Box<dyn PicoquicCustomThreadSetnameFn>>,
    _thread_name: Option<&str>,
    _loop_callback: Option<Box<dyn PicoquicPacketLoopCbFn>>,
) -> Result<Box<picoquic_network_thread_ctx_t>, i32> {
    todo!()
}

/// Wake the loop running in `thread_ctx` so its next iteration runs
/// immediately and fires
/// [`picoquic_packet_loop_cb_enum::WakeUp`].
/// C: `int picoquic_wake_up_network_thread(picoquic_network_thread_ctx_t*)`.
///
/// Returns the OS error on failure (the `errno` / `GetLastError`
/// the C body propagates), `Ok(())` otherwise.
pub fn picoquic_wake_up_network_thread(
    _thread_ctx: &mut picoquic_network_thread_ctx_t,
) -> Result<(), i32> {
    todo!()
}

/// Tear down the background thread: signals shutdown, waits for the
/// loop to exit, and `free`s the heap-allocated context.  C:
/// `void picoquic_delete_network_thread(picoquic_network_thread_ctx_t*)`.
///
/// The Rust signature consumes the owning [`Box`] —
/// the C `free` call at the end of the body is implicit in the
/// `Box` drop.
pub fn picoquic_delete_network_thread(_thread_ctx: Box<picoquic_network_thread_ctx_t>) {
    todo!()
}

// ---------------------------------------------------------------------------
// Built-in thread hooks (platform defaults).

/// Default implementation of [`PicoquicCustomThreadCreateFn`]
/// using the platform `pthread_create` / `CreateThread`.
/// C: `int picoquic_internal_thread_create(void**,
/// picoquic_thread_fn, void*)`.
///
/// Returns `Ok(picoquic_thread_t)` carrying the OS handle, or
/// `Err(i32)` with the OS error code.  Threading is dropped from
/// v1 (`TRANSLATE_PLAN.md`) so the body is a `todo!()` placeholder
/// that lands when v2 multi-threading work begins.
pub fn picoquic_internal_thread_create(
    _thread_fn: Box<dyn PicoquicThreadFn>,
) -> Result<picoquic_thread_t, i32> {
    todo!()
}

/// Default implementation of [`PicoquicCustomThreadDeleteFn`].
/// C: `void picoquic_internal_thread_delete(void**)`.
pub fn picoquic_internal_thread_delete(_thread: picoquic_thread_t) {
    todo!()
}

/// Default implementation of [`PicoquicCustomThreadSetnameFn`].
/// C: `void picoquic_internal_thread_setname(char const*)`.
pub fn picoquic_internal_thread_setname(_thread_name: &str) {
    todo!()
}

// ---------------------------------------------------------------------------
// QUIC-context helpers wired into the demo apps.

/// Look up the network-thread context attached to `quic` (set by
/// [`picoquic_start_custom_network_thread`] via
/// `quic->v_thread_ctx`), or `None` if the QUIC context isn't
/// driven by a packet-loop thread.  C:
/// `struct st_picoquic_network_thread_ctx_t* picoquic_get_thread_ctx(picoquic_quic_t*)`.
pub fn picoquic_get_thread_ctx(
    _quic: &mut picoquic_quic_t,
) -> Option<&mut picoquic_network_thread_ctx_t> {
    todo!()
}

/// Build a server-side QUIC context with the extra hooks
/// (`alpn_select_fn`, key-log, qlog, perflog, LB-CID config) that
/// the demo `picoquicdemo` server installs after
/// `picoquic_create_and_configure`.  C:
/// `int picoquic_server_set_context(picoquic_quic_t** qserver, …)`.
///
/// The C `picoquic_quic_t** qserver` out-parameter folds into the
/// `Ok(Box<…>)` payload.
pub fn picoquic_server_set_context(
    _config: &mut picoquic_quic_config_t,
    _current_time: u64,
    _default_callback: Option<Box<dyn picoquic_stream_data_cb_fn>>,
    _alpn_select_fn: Option<Box<dyn picoquic_alpn_select_fn_v2>>,
) -> Result<Box<picoquic_quic_t>, ()> {
    todo!()
}

/// Spawn `nb_threads_max` server packet-loop threads, one QUIC
/// context per thread, sharing the supplied callbacks.  C:
/// `int picoquic_start_server_threads(…)`.
///
/// The C `picoquic_network_thread_ctx_t** thread_ctxs` out-array
/// becomes a `&mut [Option<Box<…>>]` slice — the slot count carries
/// `nb_threads_max`, and successful threads land in the slots; the
/// number actually started is the `Ok` payload (replacing the C
/// `int* nb_threads_created`).
pub fn picoquic_start_server_threads(
    _config: &mut picoquic_quic_config_t,
    _current_time: u64,
    _alpn_select_fn: Option<Box<dyn picoquic_alpn_select_fn_v2>>,
    _default_callback: Option<Box<dyn picoquic_stream_data_cb_fn>>,
    _loop_callback: Option<Box<dyn PicoquicPacketLoopCbFn>>,
    _thread_create_fn: Option<Box<dyn PicoquicCustomThreadCreateFn>>,
    _thread_delete_fn: Option<Box<dyn PicoquicCustomThreadDeleteFn>>,
    _thread_setname_fn: Option<Box<dyn PicoquicCustomThreadSetnameFn>>,
    _thread_ctxs: &mut [Option<Box<picoquic_network_thread_ctx_t>>],
) -> Result<usize, ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Exposed for unit tests.

/// Close one socket and stamp [`picosocks::INVALID_SOCKET`][inv]
/// over its slot.  C: `void picoquic_packet_loop_close_socket
/// (picoquic_socket_ctx_t*)`.  Exposed so the unit tests in
/// `sockloop_test.c` can reach it directly.
///
/// [inv]: crate::picoquic::picosocks::INVALID_SOCKET
pub fn picoquic_packet_loop_close_socket(_s_ctx: &mut picoquic_socket_ctx_t) {
    todo!()
}

/// Open the per-thread socket pair(s) used by the loop.  C:
/// `int picoquic_packet_loop_open_sockets(uint16_t local_port, int
/// local_af, uint16_t public_port, int is_shared, int
/// socket_buffer_size, int extra_socket_required, int
/// do_not_use_gso, picoquic_socket_ctx_t* s_ctx, uint8_t ecn_value)`.
///
/// The slice subsumes the C "callee writes up to N entries"
/// convention — `s_ctx` must hold at least
/// [`PICOQUIC_PACKET_LOOP_SOCKETS_MAX`] elements.  Returns the number
/// of sockets actually opened (the C return); `Err(())` matches a
/// non-recoverable open failure.
pub fn picoquic_packet_loop_open_sockets(
    _local_port: u16,
    _local_af: i32,
    _public_port: u16,
    _is_shared: bool,
    _socket_buffer_size: i32,
    _extra_socket_required: bool,
    _do_not_use_gso: bool,
    _s_ctx: &mut [picoquic_socket_ctx_t],
    _ecn_value: u8,
) -> Result<usize, ()> {
    todo!()
}

#[cfg(test)]
mod test {}
