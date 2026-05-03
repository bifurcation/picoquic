//! Translation of `picoquic/bytestream.h`.
//!
//! A small cursor-over-buffer helper used by `logwriter.c` /
//! `logreader.c` (binlog) and the connection-id encoding paths.
//! Three storage modes share a single [`ByteStream`] type:
//!
//! * **Borrowed** — wrap a caller-owned slice via
//!   [`ByteStream::from_slice`].  C: `bytestream_ref_init`.
//! * **Inline** — a stack-allocated [`ByteStreamBuf`] holds 2560
//!   bytes of inline storage; [`ByteStreamBuf::stream`] hands out a
//!   [`ByteStream`] borrowing from it.  C: `bytestream_buf` +
//!   `bytestream_buf_init`.
//! * **Owned** — [`ByteStream::with_capacity`] returns a
//!   `ByteStream<'static>` that owns its heap buffer.  C:
//!   `bytestream_alloc` + `bytestream_delete` (the latter folds
//!   into `Drop`).
//!
//! ## Shape choices
//!
//! C: `bytestream { uint8_t* data; size_t size; size_t ptr; }`.
//! Rust: `ByteStream<'a> { data: ByteStreamData<'a>, ptr: usize }`.
//! `data` collapses the C `(uint8_t*, size_t)` pair into one slice
//! whose `.len()` subsumes `size`; the enum variant distinguishes
//! borrowed from owned storage so all three constructors produce
//! the same type.
//!
//! C `bytestream_buf { bytestream s; uint8_t buf[2560]; }` aliased
//! `s.data` against `buf` after init — a self-referential pair.
//! The Rust shape avoids that: [`ByteStreamBuf`] is just storage,
//! and [`ByteStreamBuf::stream`] returns a `ByteStream<'_>`
//! borrowing from the inline buffer for the duration of one I/O
//! pass.
//!
//! Operations are methods on [`ByteStream`].  C `int` 0/-1 returns
//! become `Result<T, Error>`; out-parameters fold into the `Ok`
//! variant (e.g., C `int byteread_int8(bytestream*, uint8_t*)`
//! becomes `fn read_u8(&mut self) -> Result<u8, Error>`).
//!
//! Phase 1 contract: signatures only; every body is `todo!()`.

use core::net::SocketAddr;

use crate::ConnectionId;
use crate::Error;

/// Inline-buffer capacity carried by [`ByteStreamBuf`].  C:
/// `BYTESTREAM_MAX_BUFFER_SIZE`.
pub const BYTESTREAM_MAX_BUFFER_SIZE: usize = 2560;

/// Backing storage for a [`ByteStream`].
///
/// An enum so that all three C constructors (ref / buf / alloc)
/// produce a single [`ByteStream`] type rather than splitting into
/// "borrowed" and "owned" variants.
pub enum ByteStreamData<'a> {
    /// Slice borrowed from external storage — caller-supplied
    /// buffer ([`ByteStream::from_slice`]) or the inline buffer of
    /// a [`ByteStreamBuf`] ([`ByteStreamBuf::stream`]).
    Borrowed(&'a mut [u8]),
    /// Heap-allocated buffer owned by the bytestream.  Used by
    /// [`ByteStream::with_capacity`].  Freed by the [`Drop`] impl
    /// on the vector — replaces the explicit `bytestream_delete`
    /// call in C.
    Owned(Vec<u8>),
}

/// Cursor over a byte buffer.  C: `bytestream`.
///
/// The cursor advances monotonically as bytes are read or written.
/// `data.len()` (under whichever variant) corresponds to the C
/// `size` field; the cursor offset corresponds to the C `ptr`
/// field.
// Phase 1: fields are private (encapsulated) but every method body
// is `todo!()`, so nothing reads them yet.  The allow drops in
// Phase 3 once bodies land.
#[allow(dead_code)]
pub struct ByteStream<'a> {
    data: ByteStreamData<'a>,
    ptr: usize,
}

/// Inline 2560-byte storage for a stack-allocated bytestream.
/// C: `bytestream_buf`.
///
/// Pair with [`ByteStreamBuf::stream`] to get a usable cursor:
///
/// ```ignore
/// let mut buf = ByteStreamBuf::default();
/// let s = buf.stream(128).unwrap();
/// // …read/write through `s`…
/// ```
///
/// Unlike the C struct this does not embed a `bytestream` field;
/// the cursor lives in the [`ByteStream`] returned by
/// [`stream`](Self::stream), borrowing from this inline buffer.
// See `ByteStream` for the rationale on the dead-code allow.
#[allow(dead_code)]
pub struct ByteStreamBuf {
    buf: [u8; BYTESTREAM_MAX_BUFFER_SIZE],
}

impl Default for ByteStreamBuf {
    fn default() -> Self {
        Self {
            buf: [0; BYTESTREAM_MAX_BUFFER_SIZE],
        }
    }
}

impl ByteStreamBuf {
    /// Initialize a [`ByteStream`] backed by this inline buffer,
    /// with active capacity `nb_bytes`.  Returns `None` when
    /// `nb_bytes` exceeds [`BYTESTREAM_MAX_BUFFER_SIZE`] (matching
    /// the C `NULL` return).  C: `bytestream_buf_init`.
    pub fn stream(&mut self, _nb_bytes: usize) -> Option<ByteStream<'_>> {
        todo!()
    }
}

impl<'a> ByteStream<'a> {
    /// Wrap a caller-owned mutable byte slice as a bytestream
    /// cursor.  C: `bytestream_ref_init`.
    ///
    /// The C function took `const void*` and cast away the const
    /// because the same struct served read and write callers.
    /// Rust takes `&mut [u8]` for both — read-only callers can
    /// still pass `&mut` to a buffer they treat as read-only.  A
    /// read-only-only refactor is deferred until the read/write
    /// split is clearer.
    pub fn from_slice(_bytes: &'a mut [u8]) -> Self {
        todo!()
    }

    /// Full underlying buffer (offsets `0..size`).  C:
    /// `bytestream_data` returning `const uint8_t*`.
    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }

    /// Slice from the cursor to the end of the buffer (offsets
    /// `len()..capacity()`).  C: `bytestream_ptr` returning
    /// `const uint8_t*`.
    ///
    /// REVIEW(open): only the encrypt/decrypt helpers in
    /// `quic/quicctx.c` use this — they want a writable tail to
    /// hand to the AEAD layer.  Keep until those callers are
    /// translated; if Phase 4 finds no users, drop it.
    pub fn tail(&self) -> &[u8] {
        todo!()
    }

    /// Total active capacity (the C `size` field).  C:
    /// `bytestream_size`.
    pub fn capacity(&self) -> usize {
        todo!()
    }

    /// Bytes consumed so far (the C `ptr` field).  C:
    /// `bytestream_length`.
    pub fn len(&self) -> usize {
        todo!()
    }

    /// `true` when no bytes have been consumed yet.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Bytes left between the cursor and the end of the buffer.
    /// C: `bytestream_remain`.
    pub fn remaining(&self) -> usize {
        todo!()
    }

    /// Rewind the cursor to offset 0 without touching the buffer.
    /// C: `bytestream_reset`.
    pub fn reset(&mut self) {
        todo!()
    }

    /// Rewind and zero the entire buffer.  C: `bytestream_clear` —
    /// `memset(s->data, 0, s->size)` then `s->ptr = 0`.
    pub fn clear(&mut self) {
        todo!()
    }

    /// `true` when the cursor has reached the end of the buffer.
    /// C: `bytestream_finished` returning `int` (0/1).
    pub fn is_finished(&self) -> bool {
        todo!()
    }

    /// Advance the cursor by `nb_bytes` without reading or
    /// writing.  On underflow, the C version sets `ptr = size`
    /// (forcing all subsequent operations to fail) and returns
    /// `-1`; the Rust version preserves that side effect and
    /// returns `Err`.  C: `bytestream_skip`.
    pub fn skip(&mut self, _nb_bytes: usize) -> Result<(), Error> {
        todo!()
    }

    /// Write one byte.  C: `bytewrite_int8`.
    pub fn write_u8(&mut self, _value: u8) -> Result<(), Error> {
        todo!()
    }

    /// Read one byte, advancing the cursor.  C: `byteread_int8`.
    pub fn read_u8(&mut self) -> Result<u8, Error> {
        todo!()
    }

    /// Read one byte without advancing the cursor.  C:
    /// `byteshow_int8`.
    ///
    /// The C version took a non-`const` pointer because every
    /// other function did; the Rust version takes `&self`.
    pub fn peek_u8(&self) -> Result<u8, Error> {
        todo!()
    }

    /// Write a big-endian `u16`.  C: `bytewrite_int16`.
    pub fn write_u16(&mut self, _value: u16) -> Result<(), Error> {
        todo!()
    }

    /// Read a big-endian `u16`.  C: `byteread_int16`.
    pub fn read_u16(&mut self) -> Result<u16, Error> {
        todo!()
    }

    /// Write a big-endian `u32`.  C: `bytewrite_int32`.
    pub fn write_u32(&mut self, _value: u32) -> Result<(), Error> {
        todo!()
    }

    /// Read a big-endian `u32`.  C: `byteread_int32`.
    pub fn read_u32(&mut self) -> Result<u32, Error> {
        todo!()
    }

    /// Write a big-endian `u64`.  C: `bytewrite_int64`.
    pub fn write_u64(&mut self, _value: u64) -> Result<(), Error> {
        todo!()
    }

    /// Read a big-endian `u64`.  C: `byteread_int64`.
    pub fn read_u64(&mut self) -> Result<u64, Error> {
        todo!()
    }

    /// Write a QUIC variable-length integer (RFC 9000 §16).  C:
    /// `bytewrite_vint`.
    pub fn write_varint(&mut self, _value: u64) -> Result<(), Error> {
        todo!()
    }

    /// Read a QUIC variable-length integer.  C: `byteread_varint`.
    pub fn read_varint(&mut self) -> Result<u64, Error> {
        todo!()
    }

    /// Skip past a QUIC variable-length integer without decoding
    /// it.  C: `byteread_skip_varint`.
    pub fn skip_varint(&mut self) -> Result<(), Error> {
        todo!()
    }

    /// Read a varint and downcast to `usize`.  C: `byteread_vlen`.
    /// Returns `Err` when the value doesn't fit in a `usize` on
    /// the target.  Mirrors the C check `*value != val_read`.
    pub fn read_vlen(&mut self) -> Result<usize, Error> {
        todo!()
    }

    /// Copy `buffer.len()` bytes into the stream.  C:
    /// `bytewrite_buffer(s, buffer, length)` — the explicit
    /// `length` is taken from the slice.
    pub fn write_bytes(&mut self, _buffer: &[u8]) -> Result<(), Error> {
        todo!()
    }

    /// Copy `buffer.len()` bytes out of the stream.  C:
    /// `byteread_buffer`.
    pub fn read_bytes(&mut self, _buffer: &mut [u8]) -> Result<(), Error> {
        todo!()
    }

    /// Encode a connection id as `u8` length + `id_len` raw bytes.
    /// C: `bytewrite_cid`.
    pub fn write_cid(&mut self, _cid: &ConnectionId) -> Result<(), Error> {
        todo!()
    }

    /// Decode a connection id.  C: `byteread_cid` (the C version
    /// filled an out parameter; the Rust version returns the
    /// value).  Returns `Err` when the encoded length exceeds
    /// [`crate::CONNECTION_ID_MAX_SIZE`].
    pub fn read_cid(&mut self) -> Result<ConnectionId, Error> {
        todo!()
    }

    /// Skip past an encoded connection id.  C: `byteskip_cid`.
    pub fn skip_cid(&mut self) -> Result<(), Error> {
        todo!()
    }

    /// Encode a string as varint-length + raw bytes (no NUL on the
    /// wire).  C: `bytewrite_cstr` — took a `const char*` and
    /// called `strlen`; the Rust version takes a `&str` and uses
    /// its length directly.
    pub fn write_str(&mut self, _s: &str) -> Result<(), Error> {
        todo!()
    }

    /// Decode a length-prefixed string into `dst`, returning the
    /// number of bytes written.  C: `byteread_cstr(s, char* cstr,
    /// size_t max_len)` wrote a NUL-terminated string and required
    /// `length + 1 <= max_len`; the Rust version writes raw bytes
    /// (callers reconstruct a `&str`/`&CStr` themselves) and
    /// returns `Err` when `length > dst.len()`.
    pub fn read_str(&mut self, _dst: &mut [u8]) -> Result<usize, Error> {
        todo!()
    }

    /// Skip past a length-prefixed string.  C: `byteskip_cstr`.
    pub fn skip_str(&mut self) -> Result<(), Error> {
        todo!()
    }

    /// Encode a socket address as varint(family) + 4 or 16 raw
    /// bytes (address) + big-endian `u16` (port).  C:
    /// `bytewrite_addr`, which branched on `sa_family`; the Rust
    /// version uses [`SocketAddr`].
    pub fn write_addr(&mut self, _addr: &SocketAddr) -> Result<(), Error> {
        todo!()
    }

    /// Decode a socket address.  C: `byteread_addr` (the C version
    /// filled a `sockaddr_storage` out parameter; the Rust version
    /// returns the value).
    pub fn read_addr(&mut self) -> Result<SocketAddr, Error> {
        todo!()
    }

    /// Skip past an encoded socket address.  C: `byteskip_addr`.
    pub fn skip_addr(&mut self) -> Result<(), Error> {
        todo!()
    }
}

impl ByteStream<'static> {
    /// Allocate a fresh `nb_bytes`-byte heap buffer and return a
    /// bytestream that owns it.  Returns `None` on allocation
    /// failure.  C: `bytestream_alloc`.
    ///
    /// The C signature took a pre-existing `bytestream*` and only
    /// allocated `s->data`; the Rust version bundles construction
    /// and allocation, since the bytestream now owns the buffer
    /// through [`ByteStreamData::Owned`].  The matching C
    /// `bytestream_delete` has no Rust equivalent — drop the
    /// `ByteStream` to free.
    pub fn with_capacity(_nb_bytes: usize) -> Option<Self> {
        todo!()
    }
}

impl ByteStream<'_> {
    /// Encoded byte-length of `value` as a QUIC varint (1, 2, 4,
    /// or 8 bytes).  C: `bytestream_varint_len`.
    pub fn varint_encoded_len(_value: u64) -> usize {
        todo!()
    }
}

#[cfg(test)]
mod test {}
