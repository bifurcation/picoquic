//! Translation of `picoquic/picoquic_utils.h`.
//!
//! Grab-bag of helpers used throughout the C library: debug
//! tracing, sprintf-style formatting, connection-id and address
//! manipulation, file open/close wrappers, varint and fixed-width
//! frame field codecs, constant-time memcmp, threading primitives,
//! a deterministic test RNG, and the test-suite's network simulator
//! (sim_link).  No matching `.c` file lives next to the header —
//! the bodies are in `picoquic/util.c` (most of the API), with a
//! handful of helpers in `picoquic/picoquic_utils.c`-style
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
//! * `picoquic_file_open*` / `picoquic_file_close` /
//!   `picoquic_file_delete` are real OS file primitives.  They are
//!   stubbed here as opaque [`picoquic_file_t`] handles; Phase 3
//!   will wire them to `std::fs::File` behind the (not-yet-defined)
//!   `std` Cargo feature.
//! * `struct sockaddr*` parameters and fields use
//!   [`core::net::SocketAddr`] — same convention as
//!   [`crate`].  `struct sockaddr_storage*`
//!   output parameters fold into `Option<SocketAddr>` (the C
//!   `AF_UNSPEC` sentinel becomes `None`).
//! * `int`-valued comparators (`picoquic_compare_addr`,
//!   `picoquic_compare_ip_addr`, `picoquic_compare_connection_id`,
//!   `picoquic_constant_time_memcmp`) return [`core::cmp::Ordering`]
//!   when they are real three-way comparators; the `Boolean-ish`
//!   ones (`picoquic_is_connection_id_null`) become `bool`.
//! * `char*` strings: input strings borrow as `&str`; owned outputs
//!   return `String`.  Buffer-supplying helpers (e.g.
//!   [`picoquic_addr_text`]) take a `&mut dyn core::fmt::Write`.
//! * Threading primitives (`picoquic_thread_t`, `picoquic_mutex_t`,
//!   `picoquic_event_t`) are explicitly out of scope for v1
//!   ("threading dropped, revisit in v2" per `TRANSLATE_PLAN.md`).
//!   The types are kept as opaque placeholders so signatures can
//!   land; the bodies stay `todo!()` and a Phase 3 review will
//!   decide whether to drop them outright or feature-gate.
//! * The C macro `SET_LAST_WAKE(quic, file_id)` and the
//!   `DBG_PRINTF` family are bodies-not-signatures: they expand at
//!   the call site rather than being declared in the header.  They
//!   are deferred to Phase 3 (where the C call sites get
//!   translated) — `picoquic_quic_t` has no `wake_file` /
//!   `wake_line` fields yet, so the macro can't be expressed in
//!   Rust without redesigning that type.

// C-origin struct and opaque-type names are kept verbatim (snake_case);
// covers `picoquic_file_t`, the threading stubs, and the sim-link structs.
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
// Three-way comparator helpers return `Ordering`; result-as-`Result`
// stand-ins are flagged for the missing top-level `Error` enum.
#![allow(clippy::result_unit_err)]

use core::cmp::Ordering;
use core::net::SocketAddr;

use crate::{PICOQUIC_MAX_PACKET_SIZE, picoquic_connection_id_t, picoquic_tp_preferred_address_t};

// ---------------------------------------------------------------------------
// Tracing / file-id constants.
//
// Used by the C `SET_LAST_WAKE(quic, file_id)` macro to record
// which translation unit last poked the QUIC context.  The macro
// itself is deferred — see the module docstring.

/// File-id constant for `sender.c`.
pub const PICOQUIC_SENDER: u32 = 1;
/// File-id constant for `packet.c`.
pub const PICOQUIC_PACKET: u32 = 2;
/// File-id constant for `quicctx.c`.
pub const PICOQUIC_QUICCTX: u32 = 3;
/// File-id constant for `frames.c`.
pub const PICOQUIC_FRAME: u32 = 4;
/// File-id constant for `loss_recovery.c`.
pub const PICOQUIC_LOSS_RECOVERY: u32 = 5;

// ---------------------------------------------------------------------------
// Rate / time / byte conversion (function-like macros in C).

/// Bytes that fit on a link of `bps` bytes per second over
/// `microseconds`.  C: `PICOQUIC_BYTES_FROM_RATE`.
pub const fn picoquic_bytes_from_rate(microseconds: u64, bps: u64) -> u64 {
    microseconds.wrapping_mul(bps) / 1_000_000
}

/// Effective rate (bytes per second) implied by sending `bytes`
/// in `microseconds`.  C: `PICOQUIC_RATE_FROM_BYTES`.
pub const fn picoquic_rate_from_bytes(bytes: u64, microseconds: u64) -> u64 {
    bytes.wrapping_mul(1_000_000) / microseconds
}

// ---------------------------------------------------------------------------
// Filesystem path constants — Linux/macOS layout (the only target
// supported in v1; `_WINDOWS` paths are dropped per
// `TRANSLATE_PLAN.md`).

/// Default solution-relative path used by the test fixtures.
pub const PICOQUIC_DEFAULT_SOLUTION_DIR: &str = "./";
/// Path separator used to assemble fixture paths.
pub const PICOQUIC_FILE_SEPARATOR: &str = "/";

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

/// Allocate a new heap string with up to `len` bytes copied from
/// `original` (or zero-filled for `len` bytes when `original` is
/// `None`).  Returns `None` on allocation failure to match the C
/// `NULL` return.
///
/// Pointer-shape choice: the C `original` is `const char*` and may be
/// `NULL` (zero-fill `len` bytes) or a non-NUL-terminated buffer
/// (copy up to `len` bytes).  `Option<&[u8]>` represents both cases;
/// the slice carries the source length.  `len` is the desired output
/// length independently of the source slice.  The C-style trailing NUL
/// byte is dropped — Rust strings carry their length explicitly.
pub fn picoquic_string_create(_original: Option<&[u8]>, _len: usize) -> Option<String> {
    todo!()
}

/// Duplicate a NUL-terminated C string.  Returns `None` if the
/// input was `NULL` (mapped to `Option<&str>`) or if allocation
/// failed.  C: `picoquic_string_duplicate`.
pub fn picoquic_string_duplicate(_original: Option<&str>) -> Option<String> {
    todo!()
}

/// Free the C string and return `NULL`.  In the C source this
/// sentinel return makes `str = picoquic_string_free(str)`
/// idiomatic.  Rust uses ownership transfer instead — passing
/// `String` by value drops it at end of scope, so the function
/// is a no-op shim kept for API parity.  The unit return matches
/// the "always-NULL" C semantics.
#[allow(clippy::needless_pass_by_value)]
pub fn picoquic_string_free(_str: Option<String>) {
    todo!()
}

/// Format `args` into the head of `buf` and report bytes written
/// via `nb_chars`; truncation returns an `Err`.  C:
/// `int picoquic_sprintf(char* buf, size_t buf_len, size_t* nb_chars,
/// const char* fmt, ...)`.
///
/// The C variadic signature collapses to a single
/// pre-formatted `&str` argument, mirroring the convention used
/// by [`debug_printf`].  Buffer-overflow returns `Err(())`
/// (matching the C `-1`); on success the `Ok(usize)` is the byte
/// count written, replacing the `nb_chars` out-parameter.
pub fn picoquic_sprintf(_buf: &mut [u8], _msg: &str) -> Result<usize, ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Connection-id helpers.

/// All-zero connection id used as a sentinel for "unset" / "none".
/// C: `extern const picoquic_connection_id_t picoquic_null_connection_id`.
/// Uses `Default` because `picoquic_connection_id_t` derives it as
/// the all-zero, zero-length form.
pub const picoquic_null_connection_id: picoquic_connection_id_t = picoquic_connection_id_t {
    id: [0; 20],
    id_len: 0,
};

/// Format the connection id into `bytes` and return the number of
/// bytes written.  C: `uint8_t picoquic_format_connection_id(uint8_t* bytes,
/// size_t bytes_max, picoquic_connection_id_t cnx_id)`.
///
/// Pointer-shape choice: the C body writes through `bytes` for
/// `bytes_max` bytes and reports the populated prefix length, so
/// `&mut [u8]` carries both pieces of information.  The connection
/// id is `Copy`, kept by value as in C.
pub fn picoquic_format_connection_id(_bytes: &mut [u8], _cnx_id: picoquic_connection_id_t) -> u8 {
    todo!()
}

/// Parse a connection id of `len` bytes from `bytes` into `cnx_id`,
/// returning the number of bytes consumed.  C:
/// `uint8_t picoquic_parse_connection_id(const uint8_t* bytes, uint8_t len,
/// picoquic_connection_id_t* cnx_id)`.
///
/// The C output parameter becomes the function return:
/// `Result<picoquic_connection_id_t, ()>` reports the parsed id on
/// success, `Err(())` on truncation.  The `len` parameter and the
/// slice length are redundant in safe Rust; the slice carries it.
pub fn picoquic_parse_connection_id(_bytes: &[u8]) -> Result<picoquic_connection_id_t, ()> {
    todo!()
}

/// Test whether a connection id is the `null` sentinel.  C:
/// `int picoquic_is_connection_id_null(const picoquic_connection_id_t* cnx_id)`
/// returning a 0/1 flag, mapped to `bool`.
pub fn picoquic_is_connection_id_null(_cnx_id: &picoquic_connection_id_t) -> bool {
    todo!()
}

/// Three-way compare two connection ids.  C:
/// `int picoquic_compare_connection_id(const picoquic_connection_id_t*,
/// const picoquic_connection_id_t*)` returning negative / zero /
/// positive.  Mapped to [`Ordering`].
pub fn picoquic_compare_connection_id(
    _cnx_id1: &picoquic_connection_id_t,
    _cnx_id2: &picoquic_connection_id_t,
) -> Ordering {
    todo!()
}

/// Hash a connection id with a 16-byte seed.  C: `uint64_t
/// picoquic_connection_id_hash(const picoquic_connection_id_t* cid,
/// const uint8_t* hash_seed)`.  The seed parameter is a fixed-size
/// 16-byte buffer everywhere it is called — same as
/// [`crate::picoquic_iovec_t`]'s neighbour
/// `picohash_bytes`.
pub fn picoquic_connection_id_hash(_cid: &picoquic_connection_id_t, _hash_seed: &[u8; 16]) -> u64 {
    todo!()
}

/// Fold the first up-to-8 bytes of a connection id into a `u64`.
/// C: `uint64_t picoquic_val64_connection_id(picoquic_connection_id_t)`.
pub fn picoquic_val64_connection_id(_cnx_id: picoquic_connection_id_t) -> u64 {
    todo!()
}

// ---------------------------------------------------------------------------
// Address helpers.

/// Pack an IP-and-port into a stable, byte-form key suitable for
/// hashing.  C: `size_t picoquic_hash_addr_bytes(const struct
/// sockaddr* addr, uint8_t* bytes)`.  Returns the populated prefix
/// length; the C body writes at most 18 bytes (16 for the v6
/// address + 2 for the port).
pub fn picoquic_hash_addr_bytes(_addr: &SocketAddr, _bytes: &mut [u8]) -> usize {
    todo!()
}

/// Hash an address with a 16-byte seed.  C: `uint64_t
/// picoquic_hash_addr(const struct sockaddr* addr, const uint8_t*
/// hash_seed)`.
pub fn picoquic_hash_addr(_addr: &SocketAddr, _hash_seed: &[u8; 16]) -> u64 {
    todo!()
}

/// Decode a hex-coded byte string and write the binary form into
/// `bin_output`.  C: `size_t picoquic_parse_hexa(char const*
/// hex_input, size_t input_length, uint8_t* bin_output, size_t
/// output_max)`.  Returns the number of bytes decoded.
///
/// Both length parameters fold into the slice lengths.
pub fn picoquic_parse_hexa(_hex_input: &str, _bin_output: &mut [u8]) -> usize {
    todo!()
}

/// Parse a hex-coded connection id.  C: `uint8_t
/// picoquic_parse_connection_id_hexa(char const* hex_input, size_t
/// input_length, picoquic_connection_id_t* cnx_id)` returning the
/// number of bytes decoded; the output parameter folds into the
/// `Result` return.
pub fn picoquic_parse_connection_id_hexa(_hex_input: &str) -> Result<picoquic_connection_id_t, ()> {
    todo!()
}

/// Print a connection id to a hex string buffer.  C:
/// `int picoquic_print_connection_id_hexa(char* buf, size_t buf_len,
/// const picoquic_connection_id_t* cnxid)` returning 0 / -1.
///
/// The output buffer becomes a `&mut dyn core::fmt::Write` sink to
/// keep the helper `no_std`-friendly (same convention as the
/// logger module).  The 0 / -1 status maps to `Result<(), ()>`.
pub fn picoquic_print_connection_id_hexa(
    _w: &mut dyn core::fmt::Write,
    _cnxid: &picoquic_connection_id_t,
) -> Result<(), ()> {
    todo!()
}

/// Three-way compare two addresses (IP + port).  C:
/// `int picoquic_compare_addr(const struct sockaddr* expected,
/// const struct sockaddr* actual)`.
pub fn picoquic_compare_addr(_expected: &SocketAddr, _actual: &SocketAddr) -> Ordering {
    todo!()
}

/// Three-way compare just the IP component of two addresses.  C:
/// `int picoquic_compare_ip_addr(const struct sockaddr*, const
/// struct sockaddr*)`.
pub fn picoquic_compare_ip_addr(_expected: &SocketAddr, _actual: &SocketAddr) -> Ordering {
    todo!()
}

/// Read the port number out of an address.  C: `uint16_t
/// picoquic_get_addr_port(const struct sockaddr* addr)`.
pub fn picoquic_get_addr_port(_addr: &SocketAddr) -> u16 {
    todo!()
}

/// Replace the port number on an address.  C declares this with
/// `const struct sockaddr*` but the body casts away const and
/// writes through the pointer (`util.c:531`).  The Rust signature
/// takes a `&mut SocketAddr` to reflect the actual contract.
pub fn picoquic_set_addr_port(_addr: &mut SocketAddr, _port: u16) {
    todo!()
}

/// Length of the platform sockaddr representation in bytes —
/// `sizeof(sockaddr_in)` for v4, `sizeof(sockaddr_in6)` for v6,
/// `0` for `AF_UNSPEC`.  C: `int picoquic_addr_length(const struct
/// sockaddr* addr)`.  The return value is non-negative; mapped to
/// `usize`.
pub fn picoquic_addr_length(_addr: &SocketAddr) -> usize {
    todo!()
}

/// Copy an address into a sockaddr_storage slot, treating
/// `addr == None` as the C "zero out the storage" path.  C:
/// `void picoquic_store_addr(struct sockaddr_storage* stored_addr,
/// const struct sockaddr* addr)`.
///
/// The C output parameter becomes the function return:
/// `Option<SocketAddr>` represents stored / cleared respectively.
pub fn picoquic_store_addr(_addr: Option<&SocketAddr>) -> Option<SocketAddr> {
    todo!()
}

/// Return the IP-bytes (4 or 16) inside a sockaddr.  C:
/// `void picoquic_get_ip_addr(struct sockaddr* addr, uint8_t**
/// ip_addr, uint8_t* ip_addr_len)` — both output parameters fold
/// into the returned slice.  `None` matches the C path where
/// `*ip_addr = NULL; *ip_addr_len = 0;` for unsupported families.
pub fn picoquic_get_ip_addr(_addr: &SocketAddr) -> Option<&[u8]> {
    todo!()
}

/// Parse `ip_address_text` (IPv4 or IPv6 textual form) and combine
/// with `port` into a [`SocketAddr`].  C:
/// `int picoquic_store_text_addr(struct sockaddr_storage* stored_addr,
/// const char* ip_address_text, uint16_t port)` returning 0 / -1.
pub fn picoquic_store_text_addr(_ip_address_text: &str, _port: u16) -> Result<SocketAddr, ()> {
    todo!()
}

/// Print an address (IP + port) to a `core::fmt::Write` sink.  C:
/// `char const* picoquic_addr_text(const struct sockaddr* addr,
/// char* text, size_t text_size)` returns the populated prefix of
/// `text`; the Rust translation writes through a sink to stay
/// `no_std`-friendly.  The `Err` arm propagates a fmt error.
pub fn picoquic_addr_text(
    _addr: &SocketAddr,
    _w: &mut dyn core::fmt::Write,
) -> Result<(), core::fmt::Error> {
    todo!()
}

/// Build the loopback address for the given family + port.  C:
/// `int picoquic_store_loopback_addr(struct sockaddr_storage*
/// stored_addr, int addr_family, uint16_t port)` — the family
/// argument is one of `AF_INET` / `AF_INET6` and folds into the
/// `SocketAddr` variant.
///
/// `addr_family` stays an `i32` to keep parity with the C ABI;
/// callers in the codebase pass the `AF_*` constants directly.
pub fn picoquic_store_loopback_addr(_addr_family: i32, _port: u16) -> Result<SocketAddr, ()> {
    todo!()
}

/// Fill a preferred-address transport parameter from textual IPv4
/// and/or IPv6 addresses.  C: `int picoquic_set_preferred_address(
/// picoquic_tp_preferred_address_t* preferred, char const* v4_text,
/// char const* v6_text, uint16_t preferred_port)` returning 0 / -1.
///
/// Either text argument may be `None` (the C `NULL` selector for
/// "skip this family").  The function mutates `preferred` in
/// place; on `Err(())` the partial state is unspecified, matching
/// the C behaviour.
pub fn picoquic_set_preferred_address(
    _preferred: &mut picoquic_tp_preferred_address_t,
    _v4_text: Option<&str>,
    _v6_text: Option<&str>,
    _preferred_port: u16,
) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Solution-dir helpers (test fixtures).

/// Set the global solution-relative root used by the test
/// fixtures.  C: `void picoquic_set_solution_dir(char const*
/// solution_dir)`.  Passing `None` clears the override.
pub fn picoquic_set_solution_dir(_solution_dir: Option<&str>) {
    todo!()
}

/// Read the global solution-relative root.  C exposes a raw
/// `extern char const* picoquic_solution_dir;` — translated as a
/// getter so the global stays behind a safe interface.
pub fn picoquic_solution_dir() -> Option<&'static str> {
    todo!()
}

/// Compose `solution_path/file_name` (or `./file_name` when
/// `solution_path` is `None`) into `target_file_path`.  C:
/// `int picoquic_get_input_path(char* target_file_path, size_t
/// file_path_max, const char* solution_path, const char*
/// file_name)`.  Buffer overflow returns `Err(())`.
pub fn picoquic_get_input_path(
    _target_file_path: &mut dyn core::fmt::Write,
    _solution_path: Option<&str>,
    _file_name: &str,
) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Portable file open / close.
//
// The C wrappers paper over Windows's `fopen_s` quirk; the Rust
// translation will eventually delegate to `std::fs::File`.  Phase 1
// keeps the file handle as an opaque [`picoquic_file_t`] so call
// sites can compile against the right shape; the real
// representation lands in Phase 3 once the `std` Cargo feature is
// wired up.

/// Opaque OS file handle.  Phase 3 will replace the body with
/// `std::fs::File` (under the `std` feature) or an `embedded-io`
/// equivalent for `no_std`.
pub struct picoquic_file_t {
    _opaque: [u8; 0],
}

/// Open a file with a `last_err` out-parameter.  C: `FILE*
/// picoquic_file_open_ex(char const* file_name, char const*
/// flags, int* last_err)`.  Returns `Err(errno)` on failure
/// (folding the C return-`NULL`-and-set-`*last_err` pattern into
/// a `Result`); `flags` is the `fopen` mode string.
pub fn picoquic_file_open_ex(_file_name: &str, _flags: &str) -> Result<Box<picoquic_file_t>, i32> {
    todo!()
}

/// Open a file, discarding the OS error code on failure.  C:
/// `FILE* picoquic_file_open(char const* file_name, char const*
/// flags)`.  Returns `None` on failure to match the C `NULL`.
pub fn picoquic_file_open(_file_name: &str, _flags: &str) -> Option<Box<picoquic_file_t>> {
    todo!()
}

/// Close a file and return `None`.  C: `FILE*
/// picoquic_file_close(FILE* F)` always returns `NULL` so callers
/// can write `F = picoquic_file_close(F)`; ownership transfer is
/// the natural Rust mirror.
#[allow(clippy::boxed_local)]
pub fn picoquic_file_close(_f: Box<picoquic_file_t>) {
    todo!()
}

/// Delete a file by name with a `last_err` out-parameter.  C:
/// `int picoquic_file_delete(char const* file_name, int*
/// last_err)`.  Returns `Err(errno)` on failure.
pub fn picoquic_file_delete(_file_name: &str) -> Result<(), i32> {
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
/// if `bytes` is too short.  C: `picoquic_frames_fixed_skip`.
pub fn picoquic_frames_fixed_skip(_bytes: &[u8], _size: u64) -> Option<&[u8]> {
    todo!()
}

/// Skip a varint-encoded field; returns the suffix.  C:
/// `picoquic_frames_varint_skip`.
pub fn picoquic_frames_varint_skip(_bytes: &[u8]) -> Option<&[u8]> {
    todo!()
}

/// Decode a varint and return the suffix plus the value.  C:
/// `picoquic_frames_varint_decode` with the `*n64` out-parameter
/// folded into the tuple return.
pub fn picoquic_frames_varint_decode(_bytes: &[u8]) -> Option<(&[u8], u64)> {
    todo!()
}

/// Decode a length-prefixed field's length component as a
/// `usize`.  C: `picoquic_frames_varlen_decode`.
pub fn picoquic_frames_varlen_decode(_bytes: &[u8]) -> Option<(&[u8], usize)> {
    todo!()
}

/// Decode a single byte.  C: `picoquic_frames_uint8_decode`.
pub fn picoquic_frames_uint8_decode(_bytes: &[u8]) -> Option<(&[u8], u8)> {
    todo!()
}

/// Decode a network-order `u16`.  C: `picoquic_frames_uint16_decode`.
pub fn picoquic_frames_uint16_decode(_bytes: &[u8]) -> Option<(&[u8], u16)> {
    todo!()
}

/// Decode a network-order `u32`.  C: `picoquic_frames_uint32_decode`.
pub fn picoquic_frames_uint32_decode(_bytes: &[u8]) -> Option<(&[u8], u32)> {
    todo!()
}

/// Decode a network-order `u64`.  C: `picoquic_frames_uint64_decode`.
pub fn picoquic_frames_uint64_decode(_bytes: &[u8]) -> Option<(&[u8], u64)> {
    todo!()
}

/// Skip a length-prefixed data field.  C:
/// `picoquic_frames_length_data_skip`.
pub fn picoquic_frames_length_data_skip(_bytes: &[u8]) -> Option<&[u8]> {
    todo!()
}

/// Decode a connection id and return the suffix.  C:
/// `picoquic_frames_cid_decode`.  The output parameter folds into
/// the tuple return.
pub fn picoquic_frames_cid_decode(_bytes: &[u8]) -> Option<(&[u8], picoquic_connection_id_t)> {
    todo!()
}

/// Length of the varint encoding of `n64`.  C:
/// `size_t picoquic_frames_varint_encode_length(uint64_t n64)`.
pub fn picoquic_frames_varint_encode_length(_n64: u64) -> usize {
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
/// if the buffer was too small.  C: `picoquic_frames_varint_encode`.
pub fn picoquic_frames_varint_encode(_bytes: &mut [u8], _n64: u64) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a `usize` as a varint length prefix.  C:
/// `picoquic_frames_varlen_encode`.
pub fn picoquic_frames_varlen_encode(_bytes: &mut [u8], _n: usize) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a single byte.  C: `picoquic_frames_uint8_encode`.
pub fn picoquic_frames_uint8_encode(_bytes: &mut [u8], _n: u8) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a network-order `u16`.  C: `picoquic_frames_uint16_encode`.
pub fn picoquic_frames_uint16_encode(_bytes: &mut [u8], _n: u16) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a 24-bit value (low three bytes of `n`) in network
/// order.  C: `picoquic_frames_uint24_encode`.
pub fn picoquic_frames_uint24_encode(_bytes: &mut [u8], _n: u32) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a network-order `u32`.  C: `picoquic_frames_uint32_encode`.
pub fn picoquic_frames_uint32_encode(_bytes: &mut [u8], _n: u32) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a network-order `u64`.  C: `picoquic_frames_uint64_encode`.
pub fn picoquic_frames_uint64_encode(_bytes: &mut [u8], _n: u64) -> Option<&mut [u8]> {
    todo!()
}

/// Encode a length prefix followed by the data bytes.  C:
/// `picoquic_frames_length_data_encode`.  The C signature took
/// `(size_t l, const uint8_t* v)` — collapsed to `&[u8]` (slice
/// length subsumes `l`).
pub fn picoquic_frames_length_data_encode<'a>(
    _bytes: &'a mut [u8],
    _v: &[u8],
) -> Option<&'a mut [u8]> {
    todo!()
}

/// Encode a connection id as length-prefixed data.  C:
/// `picoquic_frames_cid_encode`.
pub fn picoquic_frames_cid_encode<'a>(
    _bytes: &'a mut [u8],
    _cid: &picoquic_connection_id_t,
) -> Option<&'a mut [u8]> {
    todo!()
}

/// Encode a NUL-terminated C string `s` as length-prefixed data
/// (the NUL is *not* written).  C: `picoquic_frames_charz_encode`.
pub fn picoquic_frames_charz_encode<'a>(_bytes: &'a mut [u8], _s: &str) -> Option<&'a mut [u8]> {
    todo!()
}

// ---------------------------------------------------------------------------
// Constant-time memcmp (used for reset secrets).

/// Constant-time three-way compare of two byte buffers of equal
/// length.  C: `int picoquic_constant_time_memcmp(const uint8_t*
/// x, const uint8_t* y, size_t l)` — returning negative / zero /
/// positive, mapped to [`Ordering`].  The two slice lengths are
/// the C `l` argument and must match.
pub fn picoquic_constant_time_memcmp(_x: &[u8], _y: &[u8]) -> Ordering {
    todo!()
}

// ---------------------------------------------------------------------------
// Threading primitives — out-of-scope-for-v1 placeholders.
//
// `TRANSLATE_PLAN.md` explicitly drops threading from v1; these
// types and functions exist purely so call sites compile.  Phase 3
// will either remove them entirely (single-threaded scope) or
// route them through `std::thread` / `parking_lot` once v2 lands.

/// Opaque thread handle.  Out-of-scope-for-v1 placeholder.
pub struct picoquic_thread_t {
    _opaque: [u8; 0],
}

/// Opaque mutex.  Out-of-scope-for-v1 placeholder.
pub struct picoquic_mutex_t {
    _opaque: [u8; 0],
}

/// Opaque condition-variable wrapper.  Out-of-scope-for-v1
/// placeholder.  C: `typedef struct st_picoquic_event_t { ... }
/// picoquic_event_t;` — a `pthread_mutex_t` + `pthread_cond_t`
/// pair on Linux.
pub struct picoquic_event_t {
    _opaque: [u8; 0],
}

/// Trait counterpart of the C `picoquic_thread_fn` typedef.  v1
/// keeps the trait shape so signatures land; the bodies are
/// `todo!()` and the trait is unused in the single-threaded scope.
pub trait PicoquicThreadFn {
    /// Thread entry point.  C: `void* (*)(void* lpParam)`.
    fn run(&mut self);
}

/// Spawn a thread.  Out-of-scope-for-v1 — see module docstring.
pub fn picoquic_create_thread(
    _thread: &mut picoquic_thread_t,
    _thread_fn: Box<dyn PicoquicThreadFn>,
) -> Result<(), ()> {
    todo!()
}

/// Wait for a thread to exit.  Out-of-scope-for-v1.
pub fn picoquic_wait_thread(_thread: picoquic_thread_t) -> Result<(), ()> {
    todo!()
}

/// Detach / release a thread handle.  Out-of-scope-for-v1.
pub fn picoquic_delete_thread(_thread: &mut picoquic_thread_t) {
    todo!()
}

/// Initialize a mutex.  Out-of-scope-for-v1.
pub fn picoquic_create_mutex(_mutex: &mut picoquic_mutex_t) -> Result<(), ()> {
    todo!()
}

/// Tear down a mutex.  Out-of-scope-for-v1.
pub fn picoquic_delete_mutex(_mutex: &mut picoquic_mutex_t) -> Result<(), ()> {
    todo!()
}

/// Lock a mutex.  Out-of-scope-for-v1.
pub fn picoquic_lock_mutex(_mutex: &mut picoquic_mutex_t) -> Result<(), ()> {
    todo!()
}

/// Unlock a mutex.  Out-of-scope-for-v1.
pub fn picoquic_unlock_mutex(_mutex: &mut picoquic_mutex_t) -> Result<(), ()> {
    todo!()
}

/// Initialize an event.  Out-of-scope-for-v1.
pub fn picoquic_create_event(_event: &mut picoquic_event_t) -> Result<(), ()> {
    todo!()
}

/// Tear down an event.  Out-of-scope-for-v1.
pub fn picoquic_delete_event(_event: &mut picoquic_event_t) {
    todo!()
}

/// Signal an event.  Out-of-scope-for-v1.
pub fn picoquic_signal_event(_event: &mut picoquic_event_t) -> Result<(), ()> {
    todo!()
}

/// Wait for an event to be signalled, with a timeout in
/// microseconds.  Out-of-scope-for-v1.
pub fn picoquic_wait_for_event(
    _event: &mut picoquic_event_t,
    _microsec_wait: u64,
) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Random-number helpers.

/// Uniform random in `[0, rnd_max)` from the platform RNG.  C:
/// `uint64_t picoquic_uniform_random(uint64_t rnd_max)`.  Phase 3
/// will gate the platform-RNG body behind the `std` feature (the
/// C body reads `/dev/urandom` on Linux).
pub fn picoquic_uniform_random(_rnd_max: u64) -> u64 {
    todo!()
}

/// Deterministic test RNG: advance the 64-bit context and return
/// the new value.  C: `uint64_t picoquic_test_random(uint64_t*
/// random_context)`.
pub fn picoquic_test_random(_random_context: &mut u64) -> u64 {
    todo!()
}

/// Fill `bytes` with deterministic test RNG output.  C:
/// `void picoquic_test_random_bytes(uint64_t* random_context,
/// uint8_t* bytes, size_t bytes_max)` — `bytes_max` folds into the
/// slice length.
pub fn picoquic_test_random_bytes(_random_context: &mut u64, _bytes: &mut [u8]) {
    todo!()
}

/// Uniform test RNG in `[0, rnd_max)`.  C:
/// `picoquic_test_uniform_random`.
pub fn picoquic_test_uniform_random(_random_context: &mut u64, _rnd_max: u64) -> u64 {
    todo!()
}

/// Gaussian-distributed test RNG (variance 1, mean 0).  C:
/// `double picoquic_test_gauss_random(uint64_t* random_context)`.
pub fn picoquic_test_gauss_random(_random_context: &mut u64) -> f64 {
    todo!()
}

/// Poisson-distributed test RNG.  C:
/// `uint64_t picoquic_test_poisson_random(uint64_t*, uint64_t)`
/// where the second argument is `(uint64_t)(exp(-lambda) * 0x40000000)`.
pub fn picoquic_test_poisson_random(_random_context: &mut u64, _exp_minus_lambda_2_30: u64) -> u64 {
    todo!()
}

// ---------------------------------------------------------------------------
// Byte-array → text conversion (logging helper).

/// Translate a byte buffer to a printable text form.  C:
/// `char* picoquic_uint8_to_str(char* text, size_t text_len, const
/// uint8_t* data, size_t data_len)` — both length parameters fold
/// into the slice lengths.  The returned slice is the populated
/// prefix of `text`.
pub fn picoquic_uint8_to_str<'a>(_text: &'a mut [u8], _data: &[u8]) -> &'a [u8] {
    todo!()
}

// ---------------------------------------------------------------------------
// Network simulator (sim_link) — used by the test suite.

/// One simulated packet flowing through a sim link.  C:
/// `picoquictest_sim_packet_t`.
///
/// Pointer-shape choices:
///
/// * `next_packet` stays a raw pointer — same intrusive-list
///   pattern as `picohash_item.next_in_bin` in
///   [`crate::hash`].  Phase 3 dereferences in
///   `unsafe` blocks; refactoring to `VecDeque<Box<...>>` on the
///   link side is a candidate follow-up.
/// * The two `sockaddr_storage` fields fold into
///   `Option<SocketAddr>` (the C zero-initialised storage maps to
///   `None`).
/// * The flexible-array-style `bytes` is a fixed
///   `[u8; PICOQUIC_MAX_PACKET_SIZE]` because the C struct
///   declares it inline at that exact size.
pub struct picoquictest_sim_packet_t {
    pub next_packet: *mut picoquictest_sim_packet_t,
    pub arrival_time: u64,
    pub length: usize,
    pub addr_from: Option<SocketAddr>,
    pub addr_to: Option<SocketAddr>,
    pub ecn_mark: u8,
    pub bytes: [u8; PICOQUIC_MAX_PACKET_SIZE],
}

/// Active queue management vtable.  C: the `picoquictest_aqm_t`
/// struct of function pointers — folded into a single trait per
/// the Phase 1 rule on function pointers.  The `self` parameter
/// of each C method becomes the implicit `&mut self`; the
/// `picoquictest_sim_link_t*` link pointer stays explicit because
/// the AQM lives inside the link (taking the link by `&mut` in
/// each call would conflict with the `&mut self` borrow).  Phase 3
/// will resolve the borrow with a take-replace pattern or an
/// `unsafe` raw-pointer access.
pub trait PicoquictestAqmT {
    /// Submit a packet to the AQM.  C: `submit`.
    fn submit(
        &mut self,
        link: &mut picoquictest_sim_link_t,
        packet: Box<picoquictest_sim_packet_t>,
        current_time: u64,
    );

    /// Reset the AQM state at `current_time`.  C: `reset`.
    fn reset(&mut self, link: &mut picoquictest_sim_link_t, current_time: u64);

    /// Release any resources held by the AQM, e.g. when the link
    /// is being torn down.  C: `release`.
    fn release(&mut self, link: &mut picoquictest_sim_link_t);

    /// Whether the AQM has at least one pending packet ready to
    /// admit.  C: `has_pending` returning a 0/1 flag, mapped to
    /// `bool`.
    fn has_pending(&mut self) -> bool;

    /// Move any AQM-pending packets onto the link's main queue.
    /// C: `admit_pending`.
    fn admit_pending(&mut self, link: &mut picoquictest_sim_link_t, current_time: u64);
}

/// Jitter model used by the sim link.  C: `picoquic_jitter_mode`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum picoquic_jitter_mode {
    /// Gaussian jitter.  C: `jitter_gauss`.
    #[default]
    JitterGauss = 0,
    /// Wi-Fi-style jitter.  C: `jitter_wifi`.
    JitterWifi = 1,
}

/// One simulated network link with an embedded queue plus AQM
/// hook.  C: `picoquictest_sim_link_t`.
///
/// Pointer-shape choices, derived from the bodies in
/// `picoquictest/sim_link.c`:
///
/// * `first_packet` / `last_packet` stay raw pointers — head/tail
///   of the intrusive linked list whose nodes are
///   [`picoquictest_sim_packet_t`].  Phase 3 dereferences in
///   `unsafe` blocks (or refactors to `VecDeque`).
/// * `loss_mask: *mut u64` — the test owns a 64-bit error mask
///   and passes its address; staying raw mirrors the C contract
///   without forcing a struct lifetime.  `None`-equivalent is the
///   null pointer (the C "no mask" sentinel).
/// * `aqm_state` becomes `Option<Box<dyn PicoquictestAqmT>>` —
///   `None` matches the C `NULL` (no AQM installed).
/// * `is_switched_off` / `is_unreachable` / `is_suspended` were
///   `int` flags in C; promoted to `bool`.
pub struct picoquictest_sim_link_t {
    pub next_send_time: u64,
    pub queue_time: u64,
    pub resume_time: u64,
    pub queue_delay_max: u64,
    pub picosec_per_byte: u64,
    pub microsec_latency: u64,
    pub packets_dropped: u64,
    pub packets_sent: u64,
    pub jitter: u64,
    pub jitter_mode: picoquic_jitter_mode,
    pub jitter_seed: u64,
    pub path_mtu: usize,
    pub first_packet: *mut picoquictest_sim_packet_t,
    pub last_packet: *mut picoquictest_sim_packet_t,
    /// 64-bit error mask used in unit tests.  `null` ↔ "no mask".
    pub loss_mask: *mut u64,
    pub nb_loss_in_burst: u64,
    pub packets_between_losses: u64,
    pub packets_sent_next_burst: u64,
    pub nb_losses_this_burst: u64,
    pub end_of_burst_time: u64,
    pub aqm_state: Option<Box<dyn PicoquictestAqmT>>,
    pub is_switched_off: bool,
    pub is_unreachable: bool,
    pub is_suspended: bool,
}

/// Create a sim link at `current_time`, with the given data rate
/// (in gigabits per second) and one-way latency (in microseconds).
/// C: `picoquictest_sim_link_create`.
///
/// `loss_mask` is the test's 64-bit error mask; `None` matches the
/// C `NULL`.  The pointer is stored as-is in the link and read on
/// every packet enqueue, so the caller must keep its `u64`
/// allocation alive for the life of the link (C contract).
pub fn picoquictest_sim_link_create(
    _data_rate_in_gbps: f64,
    _microsec_latency: u64,
    _loss_mask: Option<*mut u64>,
    _queue_delay_max: u64,
    _current_time: u64,
) -> Option<Box<picoquictest_sim_link_t>> {
    todo!()
}

/// Tear down a sim link, freeing any queued packets.  C:
/// `picoquictest_sim_link_delete`.  Takes ownership so end of
/// scope does the freeing.
#[allow(clippy::boxed_local)]
pub fn picoquictest_sim_link_delete(_link: Box<picoquictest_sim_link_t>) {
    todo!()
}

/// Allocate a fresh, empty packet.  C:
/// `picoquictest_sim_link_create_packet`.  Returns `None` on
/// allocation failure.
pub fn picoquictest_sim_link_create_packet() -> Option<Box<picoquictest_sim_packet_t>> {
    todo!()
}

/// Time at which the next packet will arrive (or `current_time`
/// if the queue is empty).  C: `picoquictest_sim_link_next_arrival`.
pub fn picoquictest_sim_link_next_arrival(
    _link: &mut picoquictest_sim_link_t,
    _current_time: u64,
) -> u64 {
    todo!()
}

/// Drain any AQM-pending packets onto the link's main queue at
/// `current_time`.  C: `picoquictest_sim_link_admit_pending`.
pub fn picoquictest_sim_link_admit_pending(
    _link: &mut picoquictest_sim_link_t,
    _current_time: u64,
) {
    todo!()
}

/// Time at which the AQM will admit its next packet (or
/// `next_time` if nothing is pending).  C:
/// `picoquictest_sim_link_next_admission`.
pub fn picoquictest_sim_link_next_admission(
    _link: &mut picoquictest_sim_link_t,
    _current_time: u64,
    _next_time: u64,
) -> u64 {
    todo!()
}

/// Pop the next-due packet, if any.  C:
/// `picoquictest_sim_link_dequeue` returning `NULL` when nothing
/// is ready, mapped to `Option<Box<...>>`.
pub fn picoquictest_sim_link_dequeue(
    _link: &mut picoquictest_sim_link_t,
    _current_time: u64,
) -> Option<Box<picoquictest_sim_packet_t>> {
    todo!()
}

/// Submit a packet to the link's queue with normal AQM processing
/// and length check.  C: `picoquictest_sim_link_submit`.  Takes
/// ownership of the packet — the link is responsible for
/// either freeing it (drop) or returning it via
/// [`picoquictest_sim_link_dequeue`].
pub fn picoquictest_sim_link_submit(
    _link: &mut picoquictest_sim_link_t,
    _packet: Box<picoquictest_sim_packet_t>,
    _current_time: u64,
) {
    todo!()
}

/// Submit a packet straight to the link's "latency queue",
/// bypassing the AQM.  When `should_drop` is `true` the packet is
/// dropped instead of queued (and freed by the function).  C:
/// `picoquictest_sim_link_enqueue` with the C `int should_drop`
/// promoted to `bool`.
pub fn picoquictest_sim_link_enqueue(
    _link: &mut picoquictest_sim_link_t,
    _packet: Box<picoquictest_sim_packet_t>,
    _current_time: u64,
    _should_drop: bool,
) {
    todo!()
}

/// Compute the transmission time of `packet` on `link` (a function
/// of the link's data rate and the packet length).  C:
/// `picoquictest_sim_link_transmit_time`.
pub fn picoquictest_sim_link_transmit_time(
    _link: &mut picoquictest_sim_link_t,
    _packet: &picoquictest_sim_packet_t,
) -> u64 {
    todo!()
}

/// Queueing delay of the next packet on `link` at `current_time`.
/// C: `picoquictest_sim_link_queue_delay`.
pub fn picoquictest_sim_link_queue_delay(
    _link: &mut picoquictest_sim_link_t,
    _current_time: u64,
) -> u64 {
    todo!()
}

/// Simulate a transmission interruption on `link` until
/// `time_end_of_interval`.  When `simulate_receive` is `true` the
/// link suspends *reception* (pending packets are delivered at
/// the end of the interval); when `false` it suspends transmission
/// (packets are queued as if transmitted in sequence after the
/// interval).  C: `picoquic_test_simlink_suspend` with the C `int
/// simulate_receive` promoted to `bool`.
pub fn picoquic_test_simlink_suspend(
    _link: &mut picoquictest_sim_link_t,
    _time_end_of_interval: u64,
    _simulate_receive: bool,
) {
    todo!()
}

// ---------------------------------------------------------------------------
// SNI / certificate paths used by the test suite.  Linux/macOS
// layout only; `_WINDOWS` paths are dropped per the v1 scope.

/// Default SNI string for the test fixtures.
pub const PICOQUIC_TEST_SNI: &str = "test.example.com";

pub const PICOQUIC_TEST_FILE_SERVER_CERT: &str = "certs/cert.pem";
pub const PICOQUIC_TEST_FILE_SERVER_BAD_CERT: &str = "certs/badcert.pem";
pub const PICOQUIC_TEST_FILE_SERVER_KEY: &str = "certs/key.pem";
pub const PICOQUIC_TEST_FILE_CERT_STORE: &str = "certs/test-ca.crt";
pub const PICOQUIC_TEST_FILE_SERVER_CERT_ECDSA: &str = "certs/ecdsa/cert.pem";
pub const PICOQUIC_TEST_FILE_SERVER_KEY_ECDSA: &str = "certs/ecdsa/key.pem";
pub const PICOQUIC_TEST_ECH_PUB_KEY: &str = "certs/ech/public.pem";
pub const PICOQUIC_TEST_ECH_PRIVATE_KEY: &str = "certs/ech/private.pem";
pub const PICOQUIC_TEST_ECH_CONFIG: &str = "certs/ech/ech_config.txt";
pub const PICOQUIC_TEST_ECH_CERT: &str = "certs/ech/ech_cert.pem";
pub const PICOQUIC_TEST_ECH_RR_REF: &str = "certs/ech/ech_rr.txt";
pub const PICOQUIC_TEST_ECH_CONFIG_REF: &str = "certs/ech/ech_config.txt";
pub const PICOQUIC_TEST_FILE_SERVER_CERT_RSA: &str = "certs/rsa/cert.pem";
pub const PICOQUIC_TEST_FILE_SERVER_KEY_RSA: &str = "certs/rsa/key.pem";
pub const PICOQUIC_TEST_FILE_SERVER_CERT_ED25519: &str = "certs/mtls_ed25519/server.crt";
pub const PICOQUIC_TEST_FILE_SERVER_KEY_ED25519: &str = "certs/mtls_ed25519/server.key";
pub const PICOQUIC_TEST_FILE_CLIENT_CERT_ED25519: &str = "certs/mtls_ed25519/client.crt";
pub const PICOQUIC_TEST_FILE_CLIENT_KEY_ED25519: &str = "certs/mtls_ed25519/client.key";
pub const PICOQUIC_TEST_FILE_CERT_STORE_ED25519: &str = "certs/mtls_ed25519/ca.crt";

#[cfg(test)]
mod test {}
