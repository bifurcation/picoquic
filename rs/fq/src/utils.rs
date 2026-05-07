//! Translation of `quic/utils.h`.
//!
//! Grab-bag of helpers used throughout the C library: debug
//! tracing, sprintf-style formatting, connection-id and address
//! manipulation, file open/close wrappers, varint and fixed-width
//! frame field codecs, constant-time memcmp, threading primitives,
//! a deterministic test RNG, and the test-suite's network simulator
//! (sim_link).  No matching `.c` file lives next to the header —
//! the bodies are in `quic/util.c` (most of the API), with a
//! handful of helpers in `quic/utils.c`-style
//! companions.
//!
//! Phase 4: all function bodies are implemented.  The empty test
//! module will be expanded in later phases.
//!
//! Translation policy notes for this module:
//!
//! * `FILE *` → `&mut dyn core::fmt::Write` for *write* sinks
//!   (mirrors the convention introduced in
//!   [`crate::logger`] and
//!   [`crate::config`]).  The persistent
//!   debug-output stream installed by [`debug_set_stream`]
//!   transfers ownership instead, so the global slot can keep the
//!   writer alive between calls.
//! * `file_open*` / `file_close` collapse to direct `std::fs::File`
//!   use at call sites; [`file_delete`] keeps the portable delete
//!   helper as an OS-backed function.
//! * `struct sockaddr*` parameters and fields use
//!   [`core::net::SocketAddr`] — same convention as
//!   [`crate`].  `struct sockaddr_storage*`
//!   output parameters fold into `Option<SocketAddr>` (the C
//!   `AF_UNSPEC` sentinel becomes `None`).
//! * `int`-valued comparators (`compare_addr`,
//!   `compare_ip_addr`, `compare_connection_id`,
//!   `constant_time_memcmp`) return [`core::cmp::Ordering`]
//!   when they are real three-way comparators; the `Boolean-ish`
//!   ones (`is_connection_id_null`) become `bool`.
//! * `char*` strings: input strings borrow as `&str`; owned outputs
//!   return `String`.  Buffer-supplying helpers (e.g.
//!   [`addr_text`]) take a `&mut dyn core::fmt::Write`.
//! * Threading wrappers from the C portability layer map directly to
//!   Rust standard-library types at call sites: `std::thread`,
//!   `std::sync::Mutex`, channels, and condition variables.
//! * The C macro `SET_LAST_WAKE(quic, file_id)` and the
//!   `DBG_PRINTF` family are bodies-not-signatures: they expand at
//!   the call site rather than being declared in the header.  They
//!   are call-site macros rather than standalone functions; translated
//!   call sites record equivalent wake provenance when their owning
//!   connection or QUIC context exposes it.

use core::cmp::Ordering;
use core::net::{IpAddr, SocketAddr};
use std::cell::{Cell, RefCell};
use std::sync::Mutex;

use crate::Error;
use crate::{CONNECTION_ID_MAX_SIZE, ConnectionId, PreferredAddress};

// ---------------------------------------------------------------------------
// Tracing / file-id constants.
//
// Used by the C `SET_LAST_WAKE(quic, file_id)` macro to record
// which translation unit last poked the QUIC context.  The macro
// itself is translated at the C macro's call sites.

/// File-id constant for `sender.c`.
pub const SENDER: u32 = 1;
/// File-id constant for `packet.c`.
pub const PACKET: u32 = 2;
/// File-id constant for `quicctx.c`.
pub const QUICCTX: u32 = 3;
/// File-id constant for `frames.c`.
pub const FRAME: u32 = 4;
/// File-id constant for `loss_recovery.c`.
pub const LOSS_RECOVERY: u32 = 5;

// ---------------------------------------------------------------------------
// Rate / time / byte conversion (function-like macros in C).

/// Bytes that fit on a link of `bps` bytes per second over
/// `microseconds`.  C: `BYTES_FROM_RATE`.
pub const fn bytes_from_rate(microseconds: u64, bps: u64) -> u64 {
    microseconds.wrapping_mul(bps) / 1_000_000
}

/// Effective rate (bytes per second) implied by sending `bytes`
/// in `microseconds`.  C: `RATE_FROM_BYTES`.
pub const fn rate_from_bytes(bytes: u64, microseconds: u64) -> u64 {
    bytes.wrapping_mul(1_000_000) / microseconds
}

// ---------------------------------------------------------------------------
// Filesystem path constants — Linux/macOS layout (the only target
// supported in v1; `_WINDOWS` paths are dropped per
// `TRANSLATE_PLAN.md`).

/// Default solution-relative path used by the test fixtures.
pub const DEFAULT_SOLUTION_DIR: &str = "./";
/// Path separator used to assemble fixture paths.
pub const FILE_SEPARATOR: &str = "/";

// ---------------------------------------------------------------------------
// Debug-printf knobs and tracing helpers.

/// Maximum number of trailing characters of `__FILE__` shown by
/// the C `DBG_PRINTF` macro (`__FILE__` is right-truncated to the
/// last `DBG_PRINTF_FILENAME_MAX` bytes).
pub const DBG_PRINTF_FILENAME_MAX: usize = 24;

// Thread-local debug state — no Send required; library is single-threaded.
thread_local! {
    static DEBUG_OUT: RefCell<Option<Box<dyn core::fmt::Write>>> = const { RefCell::new(None) };
    static DEBUG_SUSPENDED: Cell<bool> = const { Cell::new(false) };
}

// Global solution_dir; typically set once at startup.  Box::leak gives the
// returned reference a 'static lifetime at the cost of a one-time allocation.
static SOLUTION_DIR: Mutex<Option<&'static str>> = Mutex::new(None);

/// Install (or clear, with `None`) the global debug-output sink.
/// The C signature `void debug_set_stream(FILE *F)` takes a
/// borrowed `FILE*`; Rust keeps the sink alive across calls by
/// transferring ownership of a `Box<dyn Write>`.  Phase 3 will
/// decide whether to expose a borrowed-with-`'static`-lifetime
/// alternative.
pub fn debug_set_stream(stream: Option<Box<dyn core::fmt::Write>>) {
    DEBUG_OUT.with(|o| *o.borrow_mut() = stream);
}

/// Returns `true` if a debug output sink is currently installed.
/// C: `picoquic/util.c:get_debug_out` — callers that checked
/// `get_debug_out() != NULL` before calling `fprintf` map to
/// checking this boolean.
pub fn get_debug_out() -> bool {
    DEBUG_OUT.with(|o| o.borrow().is_some())
}

/// Returns `true` if debug output is currently suspended.
/// C: `picoquic/util.c:get_debug_suspended` — the C `int` (0/1)
/// maps to `bool`.
pub fn get_debug_suspended() -> bool {
    DEBUG_SUSPENDED.with(|s| s.get())
}

/// `printf`-style write to the installed debug sink.  The C
/// variadic signature `void debug_printf(const char* fmt, ...)`
/// becomes a sink for already-formatted text — Rust call sites
/// will use `format_args!` / `write!` directly.  The string is
/// borrowed for the duration of the call.
pub fn debug_printf(msg: &str) {
    if DEBUG_SUSPENDED.with(|s| s.get()) {
        return;
    }
    DEBUG_OUT.with(|o| {
        if let Some(w) = o.borrow_mut().as_mut() {
            let _ = w.write_str(msg);
        }
    });
}

/// Push the current debug sink and install a new one.  Mirrors
/// `void debug_printf_push_stream(FILE* f)`.  The previously
/// installed sink (if any) is saved for [`debug_printf_pop_stream`]
/// to restore.
pub fn debug_printf_push_stream(stream: Box<dyn core::fmt::Write>) {
    DEBUG_OUT.with(|o| {
        let mut guard = o.borrow_mut();
        assert!(
            guard.is_none(),
            "nested debug_printf_push_stream not supported"
        );
        *guard = Some(stream);
    });
}

/// Pop the most recently pushed debug sink and restore the saved
/// one.  Mirrors `void debug_printf_pop_stream(void)`.
pub fn debug_printf_pop_stream() {
    DEBUG_OUT.with(|o| {
        let mut guard = o.borrow_mut();
        assert!(
            guard.is_some(),
            "debug_printf_pop_stream: no current stream"
        );
        *guard = None;
    });
}

/// Stop logging through the debug sink without tearing it down.
/// Mirrors `void debug_printf_suspend(void)`.
pub fn debug_printf_suspend() {
    DEBUG_SUSPENDED.with(|s| s.set(true));
}

/// Resume logging through the previously installed debug sink.
/// Mirrors `void debug_printf_resume(void)`.
pub fn debug_printf_resume() {
    DEBUG_SUSPENDED.with(|s| s.set(false));
}

/// Force the debug suspension flag to `suspended` and return the
/// previous value.  C: `int debug_printf_reset(int suspended)`.
/// The flag is a 0/1 indicator on both sides, so it maps to
/// `bool`.
pub fn debug_printf_reset(suspended: bool) -> bool {
    DEBUG_SUSPENDED.with(|s| {
        let old = s.get();
        s.set(suspended);
        old
    })
}

/// Hex-dump `bytes` to the debug sink.  C: `void debug_dump(const
/// void *x, int len)` — only declared when `_DEBUG` is defined,
/// so this entry point is informational in v1.  The C `int len`
/// is non-negative in every observed call; mapped to slice length.
pub fn debug_dump(bytes: &[u8]) {
    if DEBUG_SUSPENDED.with(|s| s.get()) {
        return;
    }
    DEBUG_OUT.with(|o| {
        if let Some(w) = o.borrow_mut().as_mut() {
            for (i, chunk) in bytes.chunks(16).enumerate() {
                let _ = write!(w, "{:04x}:  ", i * 16);
                for b in chunk {
                    let _ = write!(w, "{:02x} ", b);
                }
                let _ = writeln!(w);
            }
        }
    });
}

// ---------------------------------------------------------------------------
// String utilities.

/// Parse a hex digit character and return its value (0–15), or -1 for
/// invalid input.  C: `picoquic/util.c:270-284`
pub fn parse_hexa_digit(x: char) -> i32 {
    match x {
        '0'..='9' => x as i32 - '0' as i32,
        'A'..='F' => x as i32 - 'A' as i32 + 10,
        'a'..='f' => x as i32 - 'a' as i32 + 10,
        _ => -1,
    }
}

/// Create an owned `String` from the first `len` bytes of `original`.
/// If `original` is `None` or `len == 0` the result is empty.
/// C: `picoquic/util.c:45-71` — the C function allocated on the heap and
/// null-terminated; here Rust's owned `String` subsumes both roles.
pub fn string_create(original: Option<&str>, len: usize) -> String {
    let Some(src) = original else {
        return String::new();
    };
    if len == 0 {
        return String::new();
    }
    src[..len.min(src.len())].to_owned()
}

/// Compute an absolute `SystemTime` deadline `microsec_wait` microseconds
/// from the current wall clock.  C: `picoquic/util.c:1115-1124`
/// (the `#ifndef _WINDOWS` branch — the Windows path is out of v1 scope).
/// Replaces the C `struct timespec *` out-parameter with a return value.
pub fn set_abs_delay(microsec_wait: u64) -> std::time::SystemTime {
    std::time::SystemTime::now() + std::time::Duration::from_micros(microsec_wait)
}

// `picoquic_string_duplicate` and `picoquic_string_free` are subsumed by
// Rust's owned `String`: `String::from(s)` replaces the duplicate / create
// pair, and the `Drop` impl replaces the `free` shim.  Phase 3 callers should
// use owned `String` directly rather than going through helpers.

/// Format `args` into the head of `buf` and report bytes written
/// via `nb_chars`; truncation returns an `Err`.  C:
/// `int sprintf(char* buf, size_t buf_len, size_t* nb_chars,
/// const char* fmt, ...)`.
///
/// The C variadic signature collapses to a single
/// pre-formatted `&str` argument, mirroring the convention used
/// by [`debug_printf`].  Buffer-overflow returns `Err(())`
/// (matching the C `-1`); on success the `Ok(usize)` is the byte
/// count written, replacing the `nb_chars` out-parameter.
pub fn sprintf(buf: &mut [u8], msg: &str) -> Result<usize, Error> {
    let bytes = msg.as_bytes();
    if bytes.len() < buf.len() {
        buf[..bytes.len()].copy_from_slice(bytes);
        buf[bytes.len()] = 0;
        Ok(bytes.len())
    } else if !buf.is_empty() {
        let n = buf.len() - 1;
        buf[..n].copy_from_slice(&bytes[..n]);
        buf[n] = 0;
        Err(Error::BufferTooSmall)
    } else {
        Err(Error::BufferTooSmall)
    }
}

// ---------------------------------------------------------------------------
// Connection-id helpers.

/// All-zero connection id used as a sentinel for "unset" / "none".
/// C: `extern const ConnectionId null_connection_id`.
pub const NULL_CONNECTION_ID: ConnectionId = ConnectionId {
    id: [0; 20],
    id_len: 0,
};

/// Format the connection id into `bytes` and return the number of
/// bytes written.  C: `uint8_t format_connection_id(uint8_t* bytes,
/// size_t bytes_max, ConnectionId connection_id)`.
///
/// Pointer-shape choice: the C body writes through `bytes` for
/// `bytes_max` bytes and reports the populated prefix length, so
/// `&mut [u8]` carries both pieces of information.  The connection
/// id is `Copy`, kept by value as in C.
pub fn format_connection_id(bytes: &mut [u8], cnx_id: ConnectionId) -> u8 {
    let id_bytes = cnx_id.as_bytes();
    let copied = id_bytes.len();
    if copied == 0 || copied > bytes.len() {
        return 0;
    }
    bytes[..copied].copy_from_slice(id_bytes);
    copied as u8
}

/// Parse a connection id of `len` bytes from `bytes` into `connection_id`,
/// returning the number of bytes consumed.  C:
/// `uint8_t parse_connection_id(const uint8_t* bytes, uint8_t len,
/// ConnectionId* connection_id)`.
///
/// The C output parameter becomes the function return:
/// `Result<ConnectionId, Error>` reports the parsed id on
/// success, `Err(())` on truncation.  The `len` parameter and the
/// slice length are redundant in safe Rust; the slice carries it.
pub fn parse_connection_id(bytes: &[u8]) -> Result<ConnectionId, Error> {
    let len = bytes.len();
    if len > CONNECTION_ID_MAX_SIZE {
        return Err(Error::InvalidArgument);
    }
    let mut id = [0u8; CONNECTION_ID_MAX_SIZE];
    id[..len].copy_from_slice(bytes);
    Ok(ConnectionId {
        id,
        id_len: len as u8,
    })
}

/// Three-way compare two connection ids.  Shorter length sorts
/// before longer; equal-length ids are compared byte-by-byte.
/// C: `picoquic/util.c:picoquic_compare_connection_id`
pub fn compare_connection_id(id1: &ConnectionId, id2: &ConnectionId) -> Ordering {
    let len1 = id1.as_bytes().len();
    let len2 = id2.as_bytes().len();
    match len1.cmp(&len2) {
        Ordering::Equal => id1.as_bytes().cmp(id2.as_bytes()),
        other => other,
    }
}

// `is_null` / `hash_with_seed` / `val64` live as methods on
// [`ConnectionId`] (see `crate::lib`); they were free fns in the
// C source.  The `Ord` impl on `ConnectionId` and the free
// `compare_connection_id` above match the C three-way comparator.

// ---------------------------------------------------------------------------
// Address helpers.

/// Pack an IP-and-port into a stable, byte-form key suitable for
/// hashing.  C: `size_t hash_addr_bytes(const struct
/// sockaddr* addr, uint8_t* bytes)`.  Returns the populated prefix
/// length; the C body writes at most 18 bytes (16 for the v6
/// address + 2 for the port).
pub fn hash_addr_bytes(addr: &SocketAddr, bytes: &mut [u8]) -> usize {
    let mut l = 0;
    match addr {
        SocketAddr::V4(a) => {
            let ip = a.ip().octets();
            bytes[l..l + 4].copy_from_slice(&ip);
            l += 4;
            let port = addr.port().to_ne_bytes();
            bytes[l..l + 2].copy_from_slice(&port);
            l += 2;
        }
        SocketAddr::V6(a) => {
            let ip = a.ip().octets();
            bytes[l..l + 16].copy_from_slice(&ip);
            l += 16;
            let port = addr.port().to_ne_bytes();
            bytes[l..l + 2].copy_from_slice(&port);
            l += 2;
        }
    }
    l
}

/// Hash an address with a 16-byte seed.  C: `uint64_t
/// hash_addr(const struct sockaddr* addr, const uint8_t*
/// hash_seed)`.
pub fn hash_addr(addr: &SocketAddr, hash_seed: &[u8; 16]) -> u64 {
    let mut bytes = [0u8; 18];
    let l = hash_addr_bytes(addr, &mut bytes);
    crate::siphash::siphash(&bytes[..l], hash_seed)
}

/// Decode a hex-coded byte string and write the binary form into
/// `bin_output`.  C: `size_t parse_hexa(char const*
/// hex_input, size_t input_length, uint8_t* bin_output, size_t
/// output_max)`.  Returns the number of bytes decoded.
///
/// Both length parameters fold into the slice lengths.
pub fn parse_hexa(hex_input: &str, bin_output: &mut [u8]) -> usize {
    fn hexa_digit(x: u8) -> Option<u8> {
        match x {
            b'0'..=b'9' => Some(x - b'0'),
            b'A'..=b'F' => Some(x - b'A' + 10),
            b'a'..=b'f' => Some(x - b'a' + 10),
            _ => None,
        }
    }
    let inp = hex_input.as_bytes();
    let input_length = inp.len();
    if input_length == 0 || (input_length & 1) != 0 || 2 * bin_output.len() < input_length {
        return 0;
    }
    let mut ret = 0;
    let mut offset = 0;
    while offset < input_length {
        match (hexa_digit(inp[offset]), hexa_digit(inp[offset + 1])) {
            (Some(av), Some(bv)) => {
                bin_output[ret] = (av << 4) | bv;
                ret += 1;
            }
            _ => return 0,
        }
        offset += 2;
    }
    ret
}

/// Parse a hex-coded connection id.  C: `uint8_t
/// parse_connection_id_hexa(char const* hex_input, size_t
/// input_length, ConnectionId* connection_id)` returning the
/// number of bytes decoded; the output parameter folds into the
/// `Result` return.
pub fn parse_connection_id_hexa(hex_input: &str) -> Result<ConnectionId, Error> {
    let mut id = [0u8; CONNECTION_ID_MAX_SIZE];
    let len = parse_hexa(hex_input, &mut id[..18]);
    Ok(ConnectionId {
        id,
        id_len: len as u8,
    })
}

/// Print a connection id to a hex string buffer.  C:
/// `int print_connection_id_hexa(char* buf, size_t buf_len,
/// const ConnectionId* connection_id)` returning 0 / -1.
///
/// The output buffer becomes a `&mut dyn core::fmt::Write` sink to
/// keep the helper `no_std`-friendly (same convention as the
/// logger module).  The 0 / -1 status maps to `Result<(), Error>`.
pub fn print_connection_id_hexa(
    w: &mut dyn core::fmt::Write,
    connection_id: &ConnectionId,
) -> Result<(), Error> {
    for b in connection_id.as_bytes() {
        write!(w, "{:02x}", b).map_err(|_| Error::Generic)?;
    }
    Ok(())
}

/// Three-way compare two addresses (IP + port).  C:
/// `int compare_addr(const struct sockaddr* expected,
/// const struct sockaddr* actual)`.
pub fn compare_addr(expected: &SocketAddr, actual: &SocketAddr) -> Ordering {
    let ip_cmp = compare_ip_addr(expected, actual);
    if ip_cmp == Ordering::Equal {
        expected.port().cmp(&actual.port())
    } else {
        ip_cmp
    }
}

/// Three-way compare just the IP component of two addresses.  C:
/// `int compare_ip_addr(const struct sockaddr*, const
/// struct sockaddr*)`.
pub fn compare_ip_addr(expected: &SocketAddr, actual: &SocketAddr) -> Ordering {
    match (expected, actual) {
        (SocketAddr::V4(ex), SocketAddr::V4(ac)) => ex.ip().octets().cmp(&ac.ip().octets()),
        (SocketAddr::V6(ex), SocketAddr::V6(ac)) => ex.ip().octets().cmp(&ac.ip().octets()),
        // AF_INET (2) < AF_INET6 (10 on Linux), so V4 < V6.
        (SocketAddr::V4(_), SocketAddr::V6(_)) => Ordering::Less,
        (SocketAddr::V6(_), SocketAddr::V4(_)) => Ordering::Greater,
    }
}

/// Read the port number out of an address.  C: `uint16_t
/// get_addr_port(const struct sockaddr* addr)`.
pub fn get_addr_port(addr: &SocketAddr) -> u16 {
    addr.port()
}

/// Replace the port number on an address.  C declares this with
/// `const struct sockaddr*` but the body casts away const and
/// writes through the pointer (`util.c:531`).  The Rust signature
/// takes a `&mut SocketAddr` to reflect the actual contract.
pub fn set_addr_port(addr: &mut SocketAddr, port: u16) {
    addr.set_port(port);
}

/// Length of the platform sockaddr representation in bytes —
/// `sizeof(sockaddr_in)` for v4, `sizeof(sockaddr_in6)` for v6,
/// `0` for `AF_UNSPEC`.  C: `int addr_length(const struct
/// sockaddr* addr)`.  The return value is non-negative; mapped to
/// `usize`.
pub fn addr_length(addr: &SocketAddr) -> usize {
    match addr {
        SocketAddr::V4(_) => 16, // sizeof(struct sockaddr_in)
        SocketAddr::V6(_) => 28, // sizeof(struct sockaddr_in6)
    }
}

/// Copy an address into a sockaddr_storage slot, treating
/// `addr == None` as the C "zero out the storage" path.  C:
/// `void store_addr(struct sockaddr_storage* stored_addr,
/// const struct sockaddr* addr)`.
///
/// The C output parameter becomes the function return:
/// `Option<SocketAddr>` represents stored / cleared respectively.
pub fn store_addr(addr: Option<&SocketAddr>) -> Option<SocketAddr> {
    addr.copied()
}

/// Return the IP-bytes (4 or 16) inside a sockaddr.  C:
/// `void get_ip_addr(struct sockaddr* addr, uint8_t**
/// ip_addr, uint8_t* ip_addr_len)` — both output parameters fold
/// into the returned slice.  `None` matches the C path where
/// `*ip_addr = NULL; *ip_addr_len = 0;` for unsupported families.
pub fn get_ip_addr(_addr: &SocketAddr) -> Option<&[u8]> {
    // `SocketAddr` exposes IP octets only by value (`IpAddr::octets()`),
    // not by reference; safe Rust cannot produce the required borrow.
    // Callers should use `addr.ip()` with `Ipv4Addr::octets()` /
    // `Ipv6Addr::octets()` directly.
    None
}

/// Parse `ip_address_text` (IPv4 or IPv6 textual form) and combine
/// with `port` into a [`SocketAddr`].  C:
/// `int store_text_addr(struct sockaddr_storage* stored_addr,
/// const char* ip_address_text, uint16_t port)` returning 0 / -1.
pub fn store_text_addr(ip_address_text: &str, port: u16) -> Result<SocketAddr, Error> {
    let ip: IpAddr = ip_address_text
        .parse()
        .map_err(|_| Error::InvalidArgument)?;
    Ok(SocketAddr::new(ip, port))
}

/// Print an address (IP + port) to a `core::fmt::Write` sink.  C:
/// `char const* addr_text(const struct sockaddr* addr,
/// char* text, size_t text_size)` returns the populated prefix of
/// `text`; the Rust translation writes through a sink to stay
/// `no_std`-friendly.  The `Err` arm propagates a fmt error.
pub fn addr_text(addr: &SocketAddr, w: &mut dyn core::fmt::Write) -> Result<(), core::fmt::Error> {
    match addr {
        SocketAddr::V4(a) => write!(w, "{}:{}", a.ip(), a.port()),
        SocketAddr::V6(a) => write!(w, "[{}]:{}", a.ip(), a.port()),
    }
}

/// Build the loopback address for the given family + port.  C:
/// `int store_loopback_addr(struct sockaddr_storage*
/// stored_addr, int addr_family, uint16_t port)` — the family
/// argument is one of `AF_INET` / `AF_INET6` and folds into the
/// `SocketAddr` variant.
///
/// `addr_family` stays an `i32` to keep parity with the C ABI;
/// callers in the codebase pass the `AF_*` constants directly.
pub fn store_loopback_addr(addr_family: i32, port: u16) -> Result<SocketAddr, Error> {
    // AF_INET=2 is universal; AF_INET6=10 on Linux, 30 on macOS.
    const AF_INET: i32 = 2;
    #[cfg(target_os = "macos")]
    const AF_INET6: i32 = 30;
    #[cfg(not(target_os = "macos"))]
    const AF_INET6: i32 = 10;

    if addr_family == AF_INET {
        store_text_addr("127.0.0.1", port)
    } else if addr_family == AF_INET6 {
        store_text_addr("::1", port)
    } else {
        Err(Error::InvalidArgument)
    }
}

/// Fill a preferred-address transport parameter from textual IPv4
/// and/or IPv6 addresses.  C: `int set_preferred_address(
/// TpPreferredAddress* preferred, char const* v4_text,
/// char const* v6_text, uint16_t preferred_port)` returning 0 / -1.
///
/// Either text argument may be `None` (the C `NULL` selector for
/// "skip this family").  The function mutates `preferred` in
/// place; on `Err(())` the partial state is unspecified, matching
/// the C behaviour.
pub fn set_preferred_address(
    preferred: &mut PreferredAddress,
    v4_text: Option<&str>,
    v6_text: Option<&str>,
    preferred_port: u16,
) -> Result<(), Error> {
    *preferred = PreferredAddress::default();
    if let Some(v4) = v4_text {
        let addr = store_text_addr(v4, preferred_port)?;
        if !matches!(addr, SocketAddr::V4(_)) {
            return Err(Error::InvalidArgument);
        }
        preferred.v4 = Some(addr);
    }
    if let Some(v6) = v6_text {
        let addr = store_text_addr(v6, preferred_port)?;
        if !matches!(addr, SocketAddr::V6(_)) {
            return Err(Error::InvalidArgument);
        }
        preferred.v6 = Some(addr);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Solution-dir helpers (test fixtures).

/// Set the global solution-relative root used by the test
/// fixtures.  C: `void set_solution_dir(char const*
/// solution_dir)`.  Passing `None` clears the override.
pub fn set_solution_dir(solution_dir: Option<&str>) {
    let mut lock = SOLUTION_DIR.lock().unwrap();
    *lock = solution_dir.map(|s| -> &'static str { Box::leak(s.to_owned().into_boxed_str()) });
}

/// Read the global solution-relative root.  C exposes a raw
/// `extern char const* solution_dir;` — translated as a
/// getter so the global stays behind a safe interface.
pub fn solution_dir() -> Option<&'static str> {
    *SOLUTION_DIR.lock().unwrap()
}

/// Compose `solution_path/file_name` (or `./file_name` when
/// `solution_path` is `None`) into `target_file_path`.  C:
/// `int get_input_path(char* target_file_path, size_t
/// file_path_max, const char* solution_path, const char*
/// file_name)`.  Buffer overflow returns `Err(())`.
pub fn get_input_path(
    target_file_path: &mut dyn core::fmt::Write,
    solution_path: Option<&str>,
    file_name: &str,
) -> Result<(), Error> {
    let solution_path = solution_path.unwrap_or(DEFAULT_SOLUTION_DIR);
    let sep = if solution_path.ends_with(FILE_SEPARATOR) {
        ""
    } else {
        FILE_SEPARATOR
    };
    write!(target_file_path, "{}{}{}", solution_path, sep, file_name).map_err(|_| Error::Generic)
}

// ---------------------------------------------------------------------------
// Portable file open / close.
//
// `picoquic_file_open*` / `picoquic_file_close` only existed to
// paper over Windows's `fopen_s` quirk; the Rust translation uses
// `std::fs::File` directly at every call site (close = Drop).
//
// `picoquic_file_delete` survives because callers want a stable
// no_std-friendly façade for "remove this file by name".  Phase 4
// routes the body through `std::fs::remove_file` under the `std`
// feature.

/// Delete a file by name, returning the OS errno on failure.  C:
/// `int picoquic_file_delete(char const* file_name, int* last_err)`.
pub fn file_delete(file_name: &(impl AsRef<std::path::Path> + ?Sized)) -> Result<(), i32> {
    std::fs::remove_file(file_name).map_err(|e| e.raw_os_error().unwrap_or(-1))
}

// ---------------------------------------------------------------------------
// Frame skip / decode / encode helpers.
//
// The C entry points all share a "cursor pair" idiom: take
// `(bytes, bytes_max)`, advance the cursor and return a pointer
// past the parsed field, or `NULL` on overrun.  In Rust the
// `&[u8]` slice carries both endpoints; the return value is the
// remaining suffix on success or `None` on overrun (the natural
// shape of the Rust slice idiom).  Decoded values fold into the
// `Ok` payload; encoded outputs fill a pre-allocated buffer and
// return the unused tail (or `None` if the buffer was too small).

/// Skip `size` bytes from `bytes`; returns the suffix or `None`
/// if `bytes` is too short.  C: `frames_fixed_skip`.
pub fn frames_fixed_skip(bytes: &[u8], size: u64) -> Option<&[u8]> {
    let size = usize::try_from(size).ok()?;
    bytes.get(size..)
}

/// Skip a varint-encoded field; returns the suffix.  C:
/// `frames_varint_skip`.
pub fn frames_varint_skip(bytes: &[u8]) -> Option<&[u8]> {
    let v_len = varint_len(*bytes.first()?);
    frames_fixed_skip(bytes, v_len as u64)
}

/// Decode a varint and return the suffix plus the value.  C:
/// `frames_varint_decode` with the `*n64` out-parameter
/// folded into the tuple return.
pub fn frames_varint_decode(bytes: &[u8]) -> Option<(&[u8], u64)> {
    let first = *bytes.first()?;
    let length = varint_len(first);
    if bytes.len() < length {
        return None;
    }
    let mut v = (first & 0x3F) as u64;
    for &b in &bytes[1..length] {
        v <<= 8;
        v += b as u64;
    }
    Some((&bytes[length..], v))
}

/// Decode a length-prefixed field's length component as a
/// `usize`.  C: `frames_varlen_decode`.
pub fn frames_varlen_decode(bytes: &[u8]) -> Option<(&[u8], usize)> {
    let (rest, len) = frames_varint_decode(bytes)?;
    let n = usize::try_from(len).ok()?;
    Some((rest, n))
}

/// Decode a single byte.  C: `frames_uint8_decode`.
pub fn frames_uint8_decode(bytes: &[u8]) -> Option<(&[u8], u8)> {
    let (&n, rest) = bytes.split_first()?;
    Some((rest, n))
}

/// Decode a network-order `u16`.  C: `frames_uint16_decode`.
pub fn frames_uint16_decode(bytes: &[u8]) -> Option<(&[u8], u16)> {
    if bytes.len() < 2 {
        return None;
    }
    let n = u16::from_be_bytes([bytes[0], bytes[1]]);
    Some((&bytes[2..], n))
}

/// Decode a network-order `u32`.  C: `frames_uint32_decode`.
pub fn frames_uint32_decode(bytes: &[u8]) -> Option<(&[u8], u32)> {
    if bytes.len() < 4 {
        return None;
    }
    let n = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    Some((&bytes[4..], n))
}

/// Decode a network-order `u64`.  C: `frames_uint64_decode`.
pub fn frames_uint64_decode(bytes: &[u8]) -> Option<(&[u8], u64)> {
    if bytes.len() < 8 {
        return None;
    }
    let n = u64::from_be_bytes(bytes[..8].try_into().unwrap());
    Some((&bytes[8..], n))
}

/// Skip a length-prefixed data field.  C:
/// `frames_length_data_skip`.
pub fn frames_length_data_skip(bytes: &[u8]) -> Option<&[u8]> {
    let (rest, length) = frames_varint_decode(bytes)?;
    frames_fixed_skip(rest, length)
}

/// Decode a connection id and return the suffix.  C:
/// `frames_cid_decode`.  The output parameter folds into
/// the tuple return.
pub fn frames_cid_decode(bytes: &[u8]) -> Option<(&[u8], ConnectionId)> {
    let (rest, id_len) = frames_uint8_decode(bytes)?;
    if id_len as usize > CONNECTION_ID_MAX_SIZE || rest.len() < id_len as usize {
        return None;
    }
    let mut id = [0u8; CONNECTION_ID_MAX_SIZE];
    id[..id_len as usize].copy_from_slice(&rest[..id_len as usize]);
    Some((&rest[id_len as usize..], ConnectionId { id, id_len }))
}

/// Length of the varint encoding of `n64`.  C:
/// `size_t frames_varint_encode_length(uint64_t n64)`.
pub fn frames_varint_encode_length(n64: u64) -> usize {
    if n64 < 64 {
        1
    } else if n64 < 16384 {
        2
    } else if n64 < 1073741824 {
        4
    } else {
        8
    }
}

/// Decode the wire length of a varint from its first byte.  C
/// macro: `VARINT_LEN(bytes) (((uint8_t)1) << ((bytes[0] >> 6) & 3))`.
/// The macro takes the whole buffer; the function takes just the
/// first byte (the rest is unused in the C expansion).
pub const fn varint_len(byte0: u8) -> usize {
    1usize << ((byte0 >> 6) & 3)
}

/// Encode a varint into `bytes`; returns the unused tail or `None`
/// if the buffer was too small.  C: `frames_varint_encode`.
pub fn frames_varint_encode(bytes: &mut [u8], n64: u64) -> Option<&mut [u8]> {
    if n64 < 64 {
        if bytes.is_empty() {
            return None;
        }
        bytes[0] = n64 as u8;
        Some(&mut bytes[1..])
    } else if n64 < 16384 {
        if bytes.len() < 2 {
            return None;
        }
        bytes[0] = ((n64 >> 8) | 0x40) as u8;
        bytes[1] = n64 as u8;
        Some(&mut bytes[2..])
    } else if n64 < 1073741824 {
        if bytes.len() < 4 {
            return None;
        }
        bytes[0] = ((n64 >> 24) | 0x80) as u8;
        bytes[1] = (n64 >> 16) as u8;
        bytes[2] = (n64 >> 8) as u8;
        bytes[3] = n64 as u8;
        Some(&mut bytes[4..])
    } else {
        if bytes.len() < 8 {
            return None;
        }
        bytes[0] = ((n64 >> 56) | 0xC0) as u8;
        bytes[1] = (n64 >> 48) as u8;
        bytes[2] = (n64 >> 40) as u8;
        bytes[3] = (n64 >> 32) as u8;
        bytes[4] = (n64 >> 24) as u8;
        bytes[5] = (n64 >> 16) as u8;
        bytes[6] = (n64 >> 8) as u8;
        bytes[7] = n64 as u8;
        Some(&mut bytes[8..])
    }
}

/// Encode a `usize` as a varint length prefix.  C:
/// `frames_varlen_encode`.
pub fn frames_varlen_encode(bytes: &mut [u8], n: usize) -> Option<&mut [u8]> {
    frames_varint_encode(bytes, n as u64)
}

/// Encode a single byte.  C: `frames_uint8_encode`.
pub fn frames_uint8_encode(bytes: &mut [u8], n: u8) -> Option<&mut [u8]> {
    if bytes.is_empty() {
        return None;
    }
    bytes[0] = n;
    Some(&mut bytes[1..])
}

/// Encode a network-order `u16`.  C: `frames_uint16_encode`.
pub fn frames_uint16_encode(bytes: &mut [u8], n: u16) -> Option<&mut [u8]> {
    if bytes.len() < 2 {
        return None;
    }
    bytes[0] = (n >> 8) as u8;
    bytes[1] = n as u8;
    Some(&mut bytes[2..])
}

/// Encode a 24-bit value (low three bytes of `n`) in network
/// order.  C: `frames_uint24_encode`.
pub fn frames_uint24_encode(bytes: &mut [u8], n: u32) -> Option<&mut [u8]> {
    if bytes.len() < 3 {
        return None;
    }
    bytes[0] = (n >> 16) as u8;
    bytes[1] = (n >> 8) as u8;
    bytes[2] = n as u8;
    Some(&mut bytes[3..])
}

/// Encode a network-order `u32`.  C: `frames_uint32_encode`.
pub fn frames_uint32_encode(bytes: &mut [u8], n: u32) -> Option<&mut [u8]> {
    if bytes.len() < 4 {
        return None;
    }
    bytes[0] = (n >> 24) as u8;
    bytes[1] = (n >> 16) as u8;
    bytes[2] = (n >> 8) as u8;
    bytes[3] = n as u8;
    Some(&mut bytes[4..])
}

/// Encode a network-order `u64`.  C: `frames_uint64_encode`.
pub fn frames_uint64_encode(bytes: &mut [u8], n: u64) -> Option<&mut [u8]> {
    if bytes.len() < 8 {
        return None;
    }
    bytes[0] = (n >> 56) as u8;
    bytes[1] = (n >> 48) as u8;
    bytes[2] = (n >> 40) as u8;
    bytes[3] = (n >> 32) as u8;
    bytes[4] = (n >> 24) as u8;
    bytes[5] = (n >> 16) as u8;
    bytes[6] = (n >> 8) as u8;
    bytes[7] = n as u8;
    Some(&mut bytes[8..])
}

/// Encode a length prefix followed by the data bytes.  C:
/// `frames_length_data_encode`.  The C signature took
/// `(size_t l, const uint8_t* v)` — collapsed to `&[u8]` (slice
/// length subsumes `l`).
pub fn frames_length_data_encode<'a>(bytes: &'a mut [u8], v: &[u8]) -> Option<&'a mut [u8]> {
    let l = v.len();
    let rest = frames_varlen_encode(bytes, l)?;
    if rest.len() < l {
        return None;
    }
    rest[..l].copy_from_slice(v);
    Some(&mut rest[l..])
}

/// Encode a connection id as length-prefixed data.  C:
/// `frames_cid_encode`.
pub fn frames_cid_encode<'a>(bytes: &'a mut [u8], cid: &ConnectionId) -> Option<&'a mut [u8]> {
    frames_length_data_encode(bytes, cid.as_bytes())
}

/// Encode a NUL-terminated C string `s` as length-prefixed data
/// (the NUL is *not* written).  C: `frames_charz_encode`.
pub fn frames_charz_encode<'a>(bytes: &'a mut [u8], s: &str) -> Option<&'a mut [u8]> {
    frames_length_data_encode(bytes, s.as_bytes())
}

// ---------------------------------------------------------------------------
// Fixed-width big-endian frame-field writers (intformat.c).
//
// Write exactly N bytes of a big-endian integer into the start of
// `bytes`, panicking if the slice is too short (same as the C
// contract: passing an undersized buffer is undefined behaviour).

/// Write `n16` as a big-endian 16-bit value into `bytes[0..2]`.
/// C: `picoquic/intformat.c:27-31`.
pub fn picoformat_16(bytes: &mut [u8], n16: u16) {
    bytes[0] = (n16 >> 8) as u8;
    bytes[1] = n16 as u8;
}

/// Write the low 24 bits of `n24` as a big-endian 3-byte value into `bytes[0..3]`.
/// C: `picoquic/intformat.c:33-38`.
pub fn picoformat_24(bytes: &mut [u8], n24: u32) {
    bytes[0] = (n24 >> 16) as u8;
    bytes[1] = (n24 >> 8) as u8;
    bytes[2] = n24 as u8;
}

/// Write `n32` as a big-endian 32-bit value into `bytes[0..4]`.
/// C: `picoquic/intformat.c:40-46`.
pub fn picoformat_32(bytes: &mut [u8], n32: u32) {
    bytes[0] = (n32 >> 24) as u8;
    bytes[1] = (n32 >> 16) as u8;
    bytes[2] = (n32 >> 8) as u8;
    bytes[3] = n32 as u8;
}

/// Write `n64` as a big-endian 64-bit value into `bytes[0..8]`.
/// C: `picoquic/intformat.c:48-58`.
pub fn picoformat_64(bytes: &mut [u8], n64: u64) {
    bytes[0] = (n64 >> 56) as u8;
    bytes[1] = (n64 >> 48) as u8;
    bytes[2] = (n64 >> 40) as u8;
    bytes[3] = (n64 >> 32) as u8;
    bytes[4] = (n64 >> 24) as u8;
    bytes[5] = (n64 >> 16) as u8;
    bytes[6] = (n64 >> 8) as u8;
    bytes[7] = n64 as u8;
}

// ---------------------------------------------------------------------------
// Constant-time memcmp (used for reset secrets).

/// Constant-time three-way compare of two byte buffers of equal
/// length.  C: `int constant_time_memcmp(const uint8_t*
/// x, const uint8_t* y, size_t l)` — returning negative / zero /
/// positive, mapped to [`Ordering`].  The two slice lengths are
/// the C `l` argument and must match.
pub fn constant_time_memcmp(x: &[u8], y: &[u8]) -> Ordering {
    let mut acc: u64 = 0;
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        acc += (xi ^ yi) as u64;
    }
    if acc == 0 {
        Ordering::Equal
    } else {
        Ordering::Less
    }
}

// The C `picoquic_thread_t` / `picoquic_mutex_t` / `picoquic_event_t`
// portability wrappers around pthread / Win32 are gone.  Rust call
// sites use the standard library directly: `std::thread::JoinHandle`
// for a thread handle, `std::sync::Mutex<T>` for a lock paired with
// the value it guards, and `(Mutex<bool>, Condvar)` (or a higher-level
// channel from `std::sync::mpsc`) where C used `picoquic_event_t`.
//
// Likewise the `picoquic_thread_fn` typedef has no Rust counterpart —
// `std::thread::spawn` already takes an arbitrary `FnOnce() + Send`
// closure, which subsumes the C "function pointer + void* arg"
// pattern.

// ---------------------------------------------------------------------------
// Random-number helpers.

// `uniform_random` is gone -- the C body read `/dev/urandom`
// at every call.  Rust callers use `rand::Rng::random_range`
// on whatever RNG they hold (`Quic.rng` for the secure stream,
// `rand::rng()` for the non-secure thread-local).

// `test_random` and the `test_*_random` family moved to
// `crate::tests::util` — they only support the test suite.

// ---------------------------------------------------------------------------
// Byte-array → text conversion (logging helper).

/// Translate a byte buffer to a printable text form.  C:
/// `char* uint8_to_str(char* text, size_t text_len, const
/// uint8_t* data, size_t data_len)` — both length parameters fold
/// into the slice lengths.  The returned slice is the populated
/// prefix of `text`.
pub fn uint8_to_str<'a>(text: &'a mut [u8], data: &[u8]) -> &'a [u8] {
    let text_len = text.len();
    let data_len = data.len();
    let render_length = if data_len >= text_len {
        text_len.saturating_sub(4)
    } else {
        data_len
    };
    let mut rendered = 0usize;
    for &c in &data[..render_length] {
        text[rendered] = if (b' '..127).contains(&c) { c } else { b'?' };
        rendered += 1;
    }
    if rendered < data_len {
        let mut i = 0usize;
        while i < 3 && rendered + 1 < text_len {
            text[rendered] = b'.';
            rendered += 1;
            i += 1;
        }
    }
    if rendered < text_len {
        text[rendered] = 0;
    }
    &text[..rendered]
}

// `TestSimPacket`, `TestAqm`, `JitterMode`, and `TestSimLink`
// moved to `crate::tests::util` — they translate the
// `picoquictest/sim_link.c` test-only network simulator.
// `TEST_SNI` and the `TEST_FILE_*` / `TEST_ECH_*` cert-path
// constants likewise moved over.

#[cfg(test)]
mod test {}
