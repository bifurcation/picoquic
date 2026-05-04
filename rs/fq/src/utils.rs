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
//! Phase 1 contract: signatures only — every function body is
//! `todo!()`.  Bodies and the empty test module land in later
//! phases.
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
//! * `file_open*` / `file_close` / `file_delete` are real OS file
//!   primitives.  They are stubbed here as opaque [`File`] handles;
//!   Phase 3 will wire them to `std::fs::File` behind the
//!   (not-yet-defined) `std` Cargo feature.
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
//! * Threading primitives ([`Thread`], [`Mutex`], [`Event`]) are
//!   explicitly out of scope for v1 ("threading dropped, revisit in
//!   v2" per `TRANSLATE_PLAN.md`).  The types are kept as opaque
//!   placeholders so signatures can land; the bodies stay `todo!()`
//!   and a Phase 3 review will decide whether to drop them outright
//!   or feature-gate.
//! * The C macro `SET_LAST_WAKE(quic, file_id)` and the
//!   `DBG_PRINTF` family are bodies-not-signatures: they expand at
//!   the call site rather than being declared in the header.  They
//!   are deferred to Phase 3 (where the C call sites get
//!   translated) — `quic_t` has no `wake_file` /
//!   `wake_line` fields yet, so the macro can't be expressed in
//!   Rust without redesigning that type.

use core::cmp::Ordering;
use core::net::SocketAddr;

use crate::Error;
use crate::{ConnectionId, PreferredAddress};

// ---------------------------------------------------------------------------
// Tracing / file-id constants.
//
// Used by the C `SET_LAST_WAKE(quic, file_id)` macro to record
// which translation unit last poked the QUIC context.  The macro
// itself is deferred — see the module docstring.

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

/// Install (or clear, with `None`) the global debug-output sink.
/// The C signature `void debug_set_stream(FILE *F)` takes a
/// borrowed `FILE*`; Rust keeps the sink alive across calls by
/// transferring ownership of a `Box<dyn Write>`.  Phase 3 will
/// decide whether to expose a borrowed-with-`'static`-lifetime
/// alternative.
pub fn debug_set_stream(_stream: Option<Box<dyn core::fmt::Write>>) {
    todo!()
}

/// `printf`-style write to the installed debug sink.  The C
/// variadic signature `void debug_printf(const char* fmt, ...)`
/// becomes a sink for already-formatted text — Rust call sites
/// will use `format_args!` / `write!` directly.  The string is
/// borrowed for the duration of the call.
pub fn debug_printf(_msg: &str) {
    todo!()
}

/// Push the current debug sink and install a new one.  Mirrors
/// `void debug_printf_push_stream(FILE* f)`.  The previously
/// installed sink (if any) is saved for [`debug_printf_pop_stream`]
/// to restore.
pub fn debug_printf_push_stream(_stream: Box<dyn core::fmt::Write>) {
    todo!()
}

/// Pop the most recently pushed debug sink and restore the saved
/// one.  Mirrors `void debug_printf_pop_stream(void)`.
pub fn debug_printf_pop_stream() {
    todo!()
}

/// Stop logging through the debug sink without tearing it down.
/// Mirrors `void debug_printf_suspend(void)`.
pub fn debug_printf_suspend() {
    todo!()
}

/// Resume logging through the previously installed debug sink.
/// Mirrors `void debug_printf_resume(void)`.
pub fn debug_printf_resume() {
    todo!()
}

/// Force the debug suspension flag to `suspended` and return the
/// previous value.  C: `int debug_printf_reset(int suspended)`.
/// The flag is a 0/1 indicator on both sides, so it maps to
/// `bool`.
pub fn debug_printf_reset(_suspended: bool) -> bool {
    todo!()
}

/// Hex-dump `bytes` to the debug sink.  C: `void debug_dump(const
/// void *x, int len)` — only declared when `_DEBUG` is defined,
/// so this entry point is informational in v1.  The C `int len`
/// is non-negative in every observed call; mapped to slice length.
pub fn debug_dump(_bytes: &[u8]) {
    todo!()
}

// ---------------------------------------------------------------------------
// String utilities.

// `picoquic_string_create`, `picoquic_string_duplicate`, and
// `picoquic_string_free` are subsumed by Rust's owned `String`:
// `String::from(s)` replaces the duplicate / create pair, and the
// `Drop` impl replaces the `free` shim.  Phase 3 callers should use
// owned `String` directly rather than going through helpers.

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
pub fn sprintf(_buf: &mut [u8], _msg: &str) -> Result<usize, Error> {
    todo!()
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
pub fn format_connection_id(_bytes: &mut [u8], _cnx_id: ConnectionId) -> u8 {
    todo!()
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
pub fn parse_connection_id(_bytes: &[u8]) -> Result<ConnectionId, Error> {
    todo!()
}

/// Test whether a connection id is the `null` sentinel.  C:
/// `int is_connection_id_null(const ConnectionId* connection_id)`
/// returning a 0/1 flag, mapped to `bool`.
pub fn is_connection_id_null(_cnx_id: &ConnectionId) -> bool {
    todo!()
}

/// Three-way compare two connection ids.  C:
/// `int compare_connection_id(const ConnectionId*,
/// const ConnectionId*)` returning negative / zero /
/// positive.  Mapped to [`Ordering`].
pub fn compare_connection_id(_cnx_id1: &ConnectionId, _cnx_id2: &ConnectionId) -> Ordering {
    todo!()
}

/// Hash a connection id with a 16-byte seed.  C: `uint64_t
/// connection_id_hash(const ConnectionId* cid,
/// const uint8_t* hash_seed)`.  The seed parameter is a fixed-size
/// 16-byte buffer everywhere it is called — same as
/// [`crate::iovec_t`]'s neighbour
/// `hash_bytes`.
pub fn connection_id_hash(_cid: &ConnectionId, _hash_seed: &[u8; 16]) -> u64 {
    todo!()
}

/// Fold the first up-to-8 bytes of a connection id into a `u64`.
/// C: `uint64_t val64_connection_id(ConnectionId)`.
pub fn val64_connection_id(_cnx_id: ConnectionId) -> u64 {
    todo!()
}

// ---------------------------------------------------------------------------
// Address helpers.

/// Pack an IP-and-port into a stable, byte-form key suitable for
/// hashing.  C: `size_t hash_addr_bytes(const struct
/// sockaddr* addr, uint8_t* bytes)`.  Returns the populated prefix
/// length; the C body writes at most 18 bytes (16 for the v6
/// address + 2 for the port).
pub fn hash_addr_bytes(_addr: &SocketAddr, _bytes: &mut [u8]) -> usize {
    todo!()
}

/// Hash an address with a 16-byte seed.  C: `uint64_t
/// hash_addr(const struct sockaddr* addr, const uint8_t*
/// hash_seed)`.
pub fn hash_addr(_addr: &SocketAddr, _hash_seed: &[u8; 16]) -> u64 {
    todo!()
}

/// Decode a hex-coded byte string and write the binary form into
/// `bin_output`.  C: `size_t parse_hexa(char const*
/// hex_input, size_t input_length, uint8_t* bin_output, size_t
/// output_max)`.  Returns the number of bytes decoded.
///
/// Both length parameters fold into the slice lengths.
pub fn parse_hexa(_hex_input: &str, _bin_output: &mut [u8]) -> usize {
    todo!()
}

/// Parse a hex-coded connection id.  C: `uint8_t
/// parse_connection_id_hexa(char const* hex_input, size_t
/// input_length, ConnectionId* connection_id)` returning the
/// number of bytes decoded; the output parameter folds into the
/// `Result` return.
pub fn parse_connection_id_hexa(_hex_input: &str) -> Result<ConnectionId, Error> {
    todo!()
}

/// Print a connection id to a hex string buffer.  C:
/// `int print_connection_id_hexa(char* buf, size_t buf_len,
/// const ConnectionId* connection_id)` returning 0 / -1.
///
/// The output buffer becomes a `&mut dyn core::fmt::Write` sink to
/// keep the helper `no_std`-friendly (same convention as the
/// logger module).  The 0 / -1 status maps to `Result<(), Error>`.
pub fn print_connection_id_hexa(
    _w: &mut dyn core::fmt::Write,
    _connection_id: &ConnectionId,
) -> Result<(), Error> {
    todo!()
}

/// Three-way compare two addresses (IP + port).  C:
/// `int compare_addr(const struct sockaddr* expected,
/// const struct sockaddr* actual)`.
pub fn compare_addr(_expected: &SocketAddr, _actual: &SocketAddr) -> Ordering {
    todo!()
}

/// Three-way compare just the IP component of two addresses.  C:
/// `int compare_ip_addr(const struct sockaddr*, const
/// struct sockaddr*)`.
pub fn compare_ip_addr(_expected: &SocketAddr, _actual: &SocketAddr) -> Ordering {
    todo!()
}

/// Read the port number out of an address.  C: `uint16_t
/// get_addr_port(const struct sockaddr* addr)`.
pub fn get_addr_port(_addr: &SocketAddr) -> u16 {
    todo!()
}

/// Replace the port number on an address.  C declares this with
/// `const struct sockaddr*` but the body casts away const and
/// writes through the pointer (`util.c:531`).  The Rust signature
/// takes a `&mut SocketAddr` to reflect the actual contract.
pub fn set_addr_port(_addr: &mut SocketAddr, _port: u16) {
    todo!()
}

/// Length of the platform sockaddr representation in bytes —
/// `sizeof(sockaddr_in)` for v4, `sizeof(sockaddr_in6)` for v6,
/// `0` for `AF_UNSPEC`.  C: `int addr_length(const struct
/// sockaddr* addr)`.  The return value is non-negative; mapped to
/// `usize`.
pub fn addr_length(_addr: &SocketAddr) -> usize {
    todo!()
}

/// Copy an address into a sockaddr_storage slot, treating
/// `addr == None` as the C "zero out the storage" path.  C:
/// `void store_addr(struct sockaddr_storage* stored_addr,
/// const struct sockaddr* addr)`.
///
/// The C output parameter becomes the function return:
/// `Option<SocketAddr>` represents stored / cleared respectively.
pub fn store_addr(_addr: Option<&SocketAddr>) -> Option<SocketAddr> {
    todo!()
}

/// Return the IP-bytes (4 or 16) inside a sockaddr.  C:
/// `void get_ip_addr(struct sockaddr* addr, uint8_t**
/// ip_addr, uint8_t* ip_addr_len)` — both output parameters fold
/// into the returned slice.  `None` matches the C path where
/// `*ip_addr = NULL; *ip_addr_len = 0;` for unsupported families.
pub fn get_ip_addr(_addr: &SocketAddr) -> Option<&[u8]> {
    todo!()
}

/// Parse `ip_address_text` (IPv4 or IPv6 textual form) and combine
/// with `port` into a [`SocketAddr`].  C:
/// `int store_text_addr(struct sockaddr_storage* stored_addr,
/// const char* ip_address_text, uint16_t port)` returning 0 / -1.
pub fn store_text_addr(_ip_address_text: &str, _port: u16) -> Result<SocketAddr, Error> {
    todo!()
}

/// Print an address (IP + port) to a `core::fmt::Write` sink.  C:
/// `char const* addr_text(const struct sockaddr* addr,
/// char* text, size_t text_size)` returns the populated prefix of
/// `text`; the Rust translation writes through a sink to stay
/// `no_std`-friendly.  The `Err` arm propagates a fmt error.
pub fn addr_text(
    _addr: &SocketAddr,
    _w: &mut dyn core::fmt::Write,
) -> Result<(), core::fmt::Error> {
    todo!()
}

/// Build the loopback address for the given family + port.  C:
/// `int store_loopback_addr(struct sockaddr_storage*
/// stored_addr, int addr_family, uint16_t port)` — the family
/// argument is one of `AF_INET` / `AF_INET6` and folds into the
/// `SocketAddr` variant.
///
/// `addr_family` stays an `i32` to keep parity with the C ABI;
/// callers in the codebase pass the `AF_*` constants directly.
pub fn store_loopback_addr(_addr_family: i32, _port: u16) -> Result<SocketAddr, Error> {
    todo!()
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
    _preferred: &mut PreferredAddress,
    _v4_text: Option<&str>,
    _v6_text: Option<&str>,
    _preferred_port: u16,
) -> Result<(), Error> {
    todo!()
}

// ---------------------------------------------------------------------------
// Solution-dir helpers (test fixtures).

/// Set the global solution-relative root used by the test
/// fixtures.  C: `void set_solution_dir(char const*
/// solution_dir)`.  Passing `None` clears the override.
pub fn set_solution_dir(_solution_dir: Option<&str>) {
    todo!()
}

/// Read the global solution-relative root.  C exposes a raw
/// `extern char const* solution_dir;` — translated as a
/// getter so the global stays behind a safe interface.
pub fn solution_dir() -> Option<&'static str> {
    todo!()
}

/// Compose `solution_path/file_name` (or `./file_name` when
/// `solution_path` is `None`) into `target_file_path`.  C:
/// `int get_input_path(char* target_file_path, size_t
/// file_path_max, const char* solution_path, const char*
/// file_name)`.  Buffer overflow returns `Err(())`.
pub fn get_input_path(
    _target_file_path: &mut dyn core::fmt::Write,
    _solution_path: Option<&str>,
    _file_name: &str,
) -> Result<(), Error> {
    todo!()
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
pub fn file_delete(_file_name: &(impl AsRef<std::path::Path> + ?Sized)) -> Result<(), i32> {
    todo!()
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
pub fn frames_fixed_skip(_bytes: &[u8], _size: u64) -> Option<&[u8]> {
    todo!()
}

/// Skip a varint-encoded field; returns the suffix.  C:
/// `frames_varint_skip`.
pub fn frames_varint_skip(_bytes: &[u8]) -> Option<&[u8]> {
    todo!()
}

/// Decode a varint and return the suffix plus the value.  C:
/// `frames_varint_decode` with the `*n64` out-parameter
/// folded into the tuple return.
pub fn frames_varint_decode(_bytes: &[u8]) -> Option<(&[u8], u64)> {
    todo!()
}

/// Decode a length-prefixed field's length component as a
/// `usize`.  C: `frames_varlen_decode`.
pub fn frames_varlen_decode(_bytes: &[u8]) -> Option<(&[u8], usize)> {
    todo!()
}

/// Decode a single byte.  C: `frames_uint8_decode`.
pub fn frames_uint8_decode(_bytes: &[u8]) -> Option<(&[u8], u8)> {
    todo!()
}

/// Decode a network-order `u16`.  C: `frames_uint16_decode`.
pub fn frames_uint16_decode(_bytes: &[u8]) -> Option<(&[u8], u16)> {
    todo!()
}

/// Decode a network-order `u32`.  C: `frames_uint32_decode`.
pub fn frames_uint32_decode(_bytes: &[u8]) -> Option<(&[u8], u32)> {
    todo!()
}

/// Decode a network-order `u64`.  C: `frames_uint64_decode`.
pub fn frames_uint64_decode(_bytes: &[u8]) -> Option<(&[u8], u64)> {
    todo!()
}

/// Skip a length-prefixed data field.  C:
/// `frames_length_data_skip`.
pub fn frames_length_data_skip(_bytes: &[u8]) -> Option<&[u8]> {
    todo!()
}

/// Decode a connection id and return the suffix.  C:
/// `frames_cid_decode`.  The output parameter folds into
/// the tuple return.
pub fn frames_cid_decode(_bytes: &[u8]) -> Option<(&[u8], ConnectionId)> {
    todo!()
}

/// Length of the varint encoding of `n64`.  C:
/// `size_t frames_varint_encode_length(uint64_t n64)`.
pub fn frames_varint_encode_length(_n64: u64) -> usize {
    todo!()
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
pub fn frames_varint_encode(_bytes: &mut [u8], _n64: u64) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a `usize` as a varint length prefix.  C:
/// `frames_varlen_encode`.
pub fn frames_varlen_encode(_bytes: &mut [u8], _n: usize) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a single byte.  C: `frames_uint8_encode`.
pub fn frames_uint8_encode(_bytes: &mut [u8], _n: u8) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a network-order `u16`.  C: `frames_uint16_encode`.
pub fn frames_uint16_encode(_bytes: &mut [u8], _n: u16) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a 24-bit value (low three bytes of `n`) in network
/// order.  C: `frames_uint24_encode`.
pub fn frames_uint24_encode(_bytes: &mut [u8], _n: u32) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a network-order `u32`.  C: `frames_uint32_encode`.
pub fn frames_uint32_encode(_bytes: &mut [u8], _n: u32) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a network-order `u64`.  C: `frames_uint64_encode`.
pub fn frames_uint64_encode(_bytes: &mut [u8], _n: u64) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a length prefix followed by the data bytes.  C:
/// `frames_length_data_encode`.  The C signature took
/// `(size_t l, const uint8_t* v)` — collapsed to `&[u8]` (slice
/// length subsumes `l`).
pub fn frames_length_data_encode<'a>(_bytes: &'a mut [u8], _v: &[u8]) -> Option<&'a mut [u8]> {
    todo!()
}

/// Encode a connection id as length-prefixed data.  C:
/// `frames_cid_encode`.
pub fn frames_cid_encode<'a>(_bytes: &'a mut [u8], _cid: &ConnectionId) -> Option<&'a mut [u8]> {
    todo!()
}

/// Encode a NUL-terminated C string `s` as length-prefixed data
/// (the NUL is *not* written).  C: `frames_charz_encode`.
pub fn frames_charz_encode<'a>(_bytes: &'a mut [u8], _s: &str) -> Option<&'a mut [u8]> {
    todo!()
}

// ---------------------------------------------------------------------------
// Constant-time memcmp (used for reset secrets).

/// Constant-time three-way compare of two byte buffers of equal
/// length.  C: `int constant_time_memcmp(const uint8_t*
/// x, const uint8_t* y, size_t l)` — returning negative / zero /
/// positive, mapped to [`Ordering`].  The two slice lengths are
/// the C `l` argument and must match.
pub fn constant_time_memcmp(_x: &[u8], _y: &[u8]) -> Ordering {
    todo!()
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
pub fn uint8_to_str<'a>(_text: &'a mut [u8], _data: &[u8]) -> &'a [u8] {
    todo!()
}

// `TestSimPacket`, `TestAqm`, `JitterMode`, and `TestSimLink`
// moved to `crate::tests::util` — they translate the
// `picoquictest/sim_link.c` test-only network simulator.
// `TEST_SNI` and the `TEST_FILE_*` / `TEST_ECH_*` cert-path
// constants likewise moved over.

#[cfg(test)]
mod test {}
