//! Translation of `quic/bytestream.h`.
//!
//! A small cursor-over-buffer helper used by `logwriter.c` /
//! `logreader.c` (binlog) and the connection-id encoding paths.
//! Supports three storage modes:
//!
//! * **Borrowed reference** — caller-owned buffer, wrapped via
//!   [`bytestream_ref_init`].  C: `bytestream_ref_init`.
//! * **Inline buffer** — a stack-allocated [`bytestream_buf`] holds
//!   2560 bytes of inline storage; [`bytestream_buf_init`] hands
//!   out a `bytestream<'_>` borrowing from it.  C: `bytestream_buf`
//!   + `bytestream_buf_init`.
//! * **Owned heap allocation** — [`bytestream_alloc`] returns a
//!   `bytestream<'static>` whose data is a `Box<[u8]>` it owns.
//!   C: `bytestream_alloc` + `bytestream_delete`.
//!
//! ## Pointer-shape choices
//!
//! C: `bytestream { uint8_t* data; size_t size; size_t ptr; }`.
//! Rust: `bytestream<'a> { data: bytestream_data<'a>, ptr: usize }`.
//! `data` collapses the C `(uint8_t*, size_t)` pair into one slice
//! whose `.len()` subsumes `size`.  `data` is an enum so that the
//! same struct expresses all three storage modes — a single
//! `&'a mut [u8]` would force `bytestream_alloc` into a separate
//! type, and a single `Box<[u8]>` would force every borrowed
//! caller to allocate.
//!
//! C: `bytestream_buf { bytestream s; uint8_t buf[2560]; }`, where
//! `s.data` aliased `buf` after init.  The Rust shape drops the
//! self-referential aliasing — [`bytestream_buf`] is just storage,
//! and [`bytestream_buf_init`] returns a `bytestream<'_>` borrowing
//! from the buf.
//!
//! C `int` 0/-1 returns become `Result<T, ()>`; the `()` error type
//! is a placeholder — Phase 3 swaps it for the crate's top-level
//! `Error` enum once that lands.  Out-parameters fold into the
//! `Ok` variant (e.g., C `int byteread_int8(bytestream*, uint8_t*)`
//! becomes `fn byteread_int8(&mut bytestream) -> Result<u8, ()>`).
//!
//! Phase 1 contract: signatures only; every body is `todo!()`.

#![allow(non_camel_case_types)]
#![allow(clippy::result_unit_err)]

extern crate alloc;

use alloc::boxed::Box;
use core::net::SocketAddr;

use crate::connection_id_t;

// ---------------------------------------------------------------------------
// Tunable constant.

/// Inline-buffer capacity carried by [`bytestream_buf`].  C:
/// `BYTESTREAM_MAX_BUFFER_SIZE`.
pub const BYTESTREAM_MAX_BUFFER_SIZE: usize = 2560;

// ---------------------------------------------------------------------------
// Bytestream storage and cursor.

/// Backing storage for a [`bytestream`].
///
/// Phase-1 shape choice: an enum keeps all three C constructors
/// (ref / buf / alloc) producing a single [`bytestream`] type
/// rather than splitting into "borrowed" and "owned" variants.
/// The match on this enum lives inside the accessor bodies in
/// Phase 3.
pub enum bytestream_data<'a> {
    /// Slice borrowed from external storage.  Used by
    /// [`bytestream_ref_init`] (caller-supplied buffer) and
    /// [`bytestream_buf_init`] (a [`bytestream_buf`]'s inline
    /// buffer).
    Borrowed(&'a mut [u8]),
    /// Heap-allocated buffer owned by the bytestream itself.  Used
    /// by [`bytestream_alloc`].  Freed when the bytestream is
    /// dropped — replaces the explicit `bytestream_delete(s)` call
    /// in C.
    Owned(Box<[u8]>),
}

/// Cursor over a byte buffer.  C: `bytestream`.
///
/// The cursor `ptr` advances monotonically as bytes are read or
/// written.  `data.len()` (under whichever variant) corresponds to
/// the C `size` field; `data` itself replaces the C `uint8_t*`.
pub struct bytestream<'a> {
    pub data: bytestream_data<'a>,
    pub ptr: usize,
}

/// Inline 2560-byte storage for a stack-allocated bytestream.
/// C: `bytestream_buf`.
///
/// Pair with [`bytestream_buf_init`] to get a usable cursor:
///
/// ```ignore
/// let mut buf = bytestream_buf::default();
/// let s = bytestream_buf_init(&mut buf, 128).unwrap();
/// // …read/write through `s`…
/// ```
///
/// Unlike the C struct this does not embed a `bytestream` field;
/// the cursor lives in the `bytestream<'_>` returned by init,
/// borrowing from the inline buffer.
pub struct bytestream_buf {
    pub buf: [u8; BYTESTREAM_MAX_BUFFER_SIZE],
}

impl Default for bytestream_buf {
    fn default() -> Self {
        Self {
            buf: [0; BYTESTREAM_MAX_BUFFER_SIZE],
        }
    }
}

// ---------------------------------------------------------------------------
// Constructors.

/// Wrap a caller-owned mutable byte slice as a bytestream cursor.
/// C: `bytestream_ref_init`.
///
/// The C function took a `const void*` and cast away the const,
/// because the same `bytestream` shape served both read and write
/// callers.  In Rust the parameter is `&mut [u8]` — read-only
/// callers can still pass `&mut` to a buffer they otherwise treat
/// as read-only.  A read-only-only refactor is deferred to Phase 3
/// once the read/write split is clearer.
pub fn bytestream_ref_init(_bytes: &mut [u8]) -> bytestream<'_> {
    todo!()
}

/// Initialize a bytestream backed by `s`'s inline buffer, with
/// active capacity `nb_bytes`.  Returns `None` when `nb_bytes`
/// exceeds [`BYTESTREAM_MAX_BUFFER_SIZE`] (matching the C `NULL`
/// return).  C: `bytestream_buf_init`.
pub fn bytestream_buf_init(_s: &mut bytestream_buf, _nb_bytes: usize) -> Option<bytestream<'_>> {
    todo!()
}

/// Allocate a fresh `nb_bytes`-byte buffer and return a bytestream
/// owning it.  Returns `None` on allocation failure.  C:
/// `bytestream_alloc`.
///
/// The C signature took a pre-existing `bytestream*` and only
/// allocated `s->data`; the Rust version bundles construction and
/// allocation, since the bytestream now owns the buffer through
/// [`bytestream_data::Owned`].
pub fn bytestream_alloc(_nb_bytes: usize) -> Option<bytestream<'static>> {
    todo!()
}

/// Free the bytestream's owned data buffer, if any.  C:
/// `bytestream_delete`.
///
/// In safe Rust the buffer is freed by the `Drop` impl on
/// [`bytestream_data::Owned`] when the bytestream goes out of
/// scope.  This function is kept for source parity with the C
/// call sites — taking ownership of the bytestream forces it to
/// drop here, matching the early-free intent.
pub fn bytestream_delete(_s: bytestream<'_>) {
    // dropped here
}

// ---------------------------------------------------------------------------
// Accessors.

/// Pointer to the start of the underlying buffer.  C:
/// `bytestream_data` returning `const uint8_t*`.
pub fn bytestream_data<'b>(_s: &'b bytestream<'_>) -> &'b [u8] {
    todo!()
}

/// Pointer to the unread/unwritten tail (offset `ptr`).  C:
/// `bytestream_ptr` returning `const uint8_t*`.
pub fn bytestream_ptr<'b>(_s: &'b bytestream<'_>) -> &'b [u8] {
    todo!()
}

/// Total active capacity (the C `size` field).  C:
/// `bytestream_size`.
pub fn bytestream_size(_s: &bytestream<'_>) -> usize {
    todo!()
}

/// Bytes consumed so far (the C `ptr` field).  C:
/// `bytestream_length`.
pub fn bytestream_length(_s: &bytestream<'_>) -> usize {
    todo!()
}

/// Bytes left between the cursor and the end of the buffer.  C:
/// `bytestream_remain`.
pub fn bytestream_remain(_s: &bytestream<'_>) -> usize {
    todo!()
}

/// Rewind the cursor to offset 0 without touching the buffer.  C:
/// `bytestream_reset`.
pub fn bytestream_reset(_s: &mut bytestream<'_>) {
    todo!()
}

/// Rewind and zero the entire buffer.  C: `bytestream_clear` —
/// `memset(s->data, 0, s->size)` then `s->ptr = 0`.
pub fn bytestream_clear(_s: &mut bytestream<'_>) {
    todo!()
}

/// `true` when the cursor has reached the end of the buffer.
/// C: `bytestream_finished` returning `int` (0/1).
pub fn bytestream_finished(_s: &bytestream<'_>) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------
// Cursor advance.

/// Advance the cursor by `nb_bytes` without reading/writing.  On
/// underflow, the C version sets `ptr = size` and returns -1; here
/// the same side effect occurs and `Err(())` is returned.  C:
/// `bytestream_skip`.
pub fn bytestream_skip(_s: &mut bytestream<'_>, _nb_bytes: usize) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Fixed-width integer I/O (big-endian on the wire).

/// Write one byte.  C: `bytewrite_int8`.
pub fn bytewrite_int8(_s: &mut bytestream<'_>, _value: u8) -> Result<(), ()> {
    todo!()
}

/// Read one byte, advancing the cursor.  C: `byteread_int8`.
pub fn byteread_int8(_s: &mut bytestream<'_>) -> Result<u8, ()> {
    todo!()
}

/// Peek one byte without advancing.  C: `byteshow_int8`.
///
/// The C version takes a non-`const` `bytestream*` because every
/// other function does, but only reads.  Rust takes `&bytestream`.
pub fn byteshow_int8(_s: &bytestream<'_>) -> Result<u8, ()> {
    todo!()
}

/// Write a big-endian `u16`.  C: `bytewrite_int16`.
pub fn bytewrite_int16(_s: &mut bytestream<'_>, _value: u16) -> Result<(), ()> {
    todo!()
}

/// Read a big-endian `u16`.  C: `byteread_int16`.
pub fn byteread_int16(_s: &mut bytestream<'_>) -> Result<u16, ()> {
    todo!()
}

/// Write a big-endian `u32`.  C: `bytewrite_int32`.
pub fn bytewrite_int32(_s: &mut bytestream<'_>, _value: u32) -> Result<(), ()> {
    todo!()
}

/// Read a big-endian `u32`.  C: `byteread_int32`.
pub fn byteread_int32(_s: &mut bytestream<'_>) -> Result<u32, ()> {
    todo!()
}

/// Write a big-endian `u64`.  C: `bytewrite_int64`.
pub fn bytewrite_int64(_s: &mut bytestream<'_>, _value: u64) -> Result<(), ()> {
    todo!()
}

/// Read a big-endian `u64`.  C: `byteread_int64`.
pub fn byteread_int64(_s: &mut bytestream<'_>) -> Result<u64, ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Variable-length integer I/O (RFC 9000 §16).

/// Write a QUIC variable-length integer.  C: `bytewrite_vint`.
pub fn bytewrite_vint(_s: &mut bytestream<'_>, _value: u64) -> Result<(), ()> {
    todo!()
}

/// Read a QUIC variable-length integer.  C: `byteread_vint`.
pub fn byteread_vint(_s: &mut bytestream<'_>) -> Result<u64, ()> {
    todo!()
}

/// Skip past a QUIC variable-length integer without decoding it.
/// C: `byteread_skip_vint`.
pub fn byteread_skip_vint(_s: &mut bytestream<'_>) -> Result<(), ()> {
    todo!()
}

/// Encoded byte-length of `value` as a QUIC varint (1, 2, 4, or 8
/// bytes).  C: `bytestream_vint_len`.
pub fn bytestream_vint_len(_value: u64) -> usize {
    todo!()
}

/// Read a varint and downcast to `usize`.  C: `byteread_vlen`.
/// Returns `Err(())` when the value doesn't fit in a `usize` on
/// the target.  Mirrors the C check `*value != val_read`.
pub fn byteread_vlen(_s: &mut bytestream<'_>) -> Result<usize, ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Raw buffer I/O.

/// Copy `buffer.len()` bytes into the stream.  C:
/// `bytewrite_buffer(s, buffer, length)` — the explicit `length`
/// is taken from the slice.
pub fn bytewrite_buffer(_s: &mut bytestream<'_>, _buffer: &[u8]) -> Result<(), ()> {
    todo!()
}

/// Copy `buffer.len()` bytes out of the stream.  C:
/// `byteread_buffer`.
pub fn byteread_buffer(_s: &mut bytestream<'_>, _buffer: &mut [u8]) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Connection-id helpers.

/// Encode a connection id as `u8` length + `id_len` raw bytes.  C:
/// `bytewrite_cid`.
///
/// `cid` is `&connection_id_t` — the C param was
/// `const connection_id_t*`, never NULL at any caller
/// (`logwriter.c:683`, `:757`, `:908`).
pub fn bytewrite_cid(_s: &mut bytestream<'_>, _cid: &connection_id_t) -> Result<(), ()> {
    todo!()
}

/// Decode a connection id.  C `byteread_cid` filled in an out
/// parameter; the Rust version returns the value through `Ok`.
/// Returns `Err(())` when the encoded length exceeds
/// `CONNECTION_ID_MAX_SIZE`.
pub fn byteread_cid(_s: &mut bytestream<'_>) -> Result<connection_id_t, ()> {
    todo!()
}

/// Skip past an encoded connection id.  C: `byteskip_cid`.
pub fn byteskip_cid(_s: &mut bytestream<'_>) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// C-string helpers.

/// Encode a string as varint-length + raw bytes (no NUL on the
/// wire).  C: `bytewrite_cstr` — took a `const char*` and called
/// `strlen` to find the length; in Rust the length comes from the
/// `&str`.
pub fn bytewrite_cstr(_s: &mut bytestream<'_>, _cstr: &str) -> Result<(), ()> {
    todo!()
}

/// Decode a length-prefixed string into `dst`, returning the
/// number of bytes written (excluding any NUL terminator).
/// C: `byteread_cstr(s, char* cstr, size_t max_len)` — wrote a
/// NUL-terminated string into `cstr` and required
/// `length + 1 <= max_len`; the Rust version writes raw bytes
/// (callers reconstruct a `&str`/`&CStr` themselves) and returns
/// `Err(())` when `length > dst.len()`.
pub fn byteread_cstr(_s: &mut bytestream<'_>, _dst: &mut [u8]) -> Result<usize, ()> {
    todo!()
}

/// Skip past a length-prefixed string.  C: `byteskip_cstr`.
pub fn byteskip_cstr(_s: &mut bytestream<'_>) -> Result<(), ()> {
    todo!()
}

// ---------------------------------------------------------------------------
// Socket-address helpers.
//
// The C versions take `struct sockaddr*` / `struct sockaddr_storage*`
// and branch on `sa_family`.  Rust uses [`core::net::SocketAddr`] —
// the same convention as `unified_log` and `socks`.

/// Encode a socket address as varint(family) + 4 or 16 raw bytes
/// (address) + big-endian `u16` (port).  C: `bytewrite_addr`.
pub fn bytewrite_addr(_s: &mut bytestream<'_>, _addr: &SocketAddr) -> Result<(), ()> {
    todo!()
}

/// Decode a socket address.  C: `byteread_addr` filled a
/// `sockaddr_storage` out parameter; the Rust version returns the
/// value.
pub fn byteread_addr(_s: &mut bytestream<'_>) -> Result<SocketAddr, ()> {
    todo!()
}

/// Skip past an encoded socket address.  C: `byteskip_addr`.
pub fn byteskip_addr(_s: &mut bytestream<'_>) -> Result<(), ()> {
    todo!()
}

#[cfg(test)]
mod test {}
