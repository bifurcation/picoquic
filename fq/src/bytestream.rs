//! Bytestream utilities for reading and writing binary data.
//!
//! This module provides a cursor-like abstraction for serializing and
//! deserializing binary protocols, with support for QUIC variable-length
//! integers.
//!
//! Translated from picoquic/bytestream.c

use crate::intformat::{self, varint};
use crate::util::{ConnectionId, CONNECTION_ID_MAX_SIZE};

/// Maximum size for stack-allocated bytestream buffers.
pub const MAX_BUFFER_SIZE: usize = 2560;

/// Error type for bytestream operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamError;

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bytestream error: buffer overflow or underflow")
    }
}

impl std::error::Error for StreamError {}

/// A bytestream wraps a byte buffer with a read/write position.
///
/// This is similar to `std::io::Cursor` but designed for QUIC protocol
/// serialization with explicit error handling.
#[derive(Debug)]
pub struct ByteStream<'a> {
    data: &'a mut [u8],
    pos: usize,
}

/// A read-only bytestream for parsing.
#[derive(Debug)]
pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

/// A bytestream with an owned buffer.
#[derive(Debug)]
pub struct ByteStreamBuf {
    buf: Vec<u8>,
    pos: usize,
}

impl<'a> ByteStream<'a> {
    /// Create a new bytestream wrapping a mutable buffer.
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Create a bytestream wrapping a mutable buffer with initial position.
    pub fn with_position(data: &'a mut [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    /// Wrap a C bytestream struct, borrowing its buffer.
    ///
    /// # Safety
    /// The CBytestream must have a valid data pointer and size.
    pub unsafe fn from_c(c: &'a mut CBytestream) -> Self {
        let data = std::slice::from_raw_parts_mut(c.data, c.size);
        Self { data, pos: c.ptr }
    }

    /// Sync position back to a C bytestream struct.
    pub fn sync_to_c(&self, c: &mut CBytestream) {
        c.ptr = self.pos;
    }

    /// Get the underlying data slice.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Get a slice at the current position.
    pub fn ptr(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    /// Get the total buffer size.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get the current position (bytes written/read).
    pub fn length(&self) -> usize {
        self.pos
    }

    /// Get remaining bytes available.
    pub fn remain(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Reset position to the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Clear buffer and reset position.
    pub fn clear(&mut self) {
        self.pos = 0;
        self.data.fill(0);
    }

    /// Check if the stream is at the end.
    pub fn finished(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Skip forward by `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<(), StreamError> {
        if self.remain() < n {
            self.pos = self.data.len();
            Err(StreamError)
        } else {
            self.pos += n;
            Ok(())
        }
    }

    /// Write a u8.
    pub fn write_u8(&mut self, value: u8) -> Result<(), StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.data[self.pos] = value;
        self.pos += 1;
        Ok(())
    }

    /// Write a u16 in big-endian format.
    pub fn write_u16(&mut self, value: u16) -> Result<(), StreamError> {
        if self.remain() < 2 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        intformat::format_16(&mut self.data[self.pos..], value);
        self.pos += 2;
        Ok(())
    }

    /// Write a u32 in big-endian format.
    pub fn write_u32(&mut self, value: u32) -> Result<(), StreamError> {
        if self.remain() < 4 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        intformat::format_32(&mut self.data[self.pos..], value);
        self.pos += 4;
        Ok(())
    }

    /// Write a u64 in big-endian format.
    pub fn write_u64(&mut self, value: u64) -> Result<(), StreamError> {
        if self.remain() < 8 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        intformat::format_64(&mut self.data[self.pos..], value);
        self.pos += 8;
        Ok(())
    }

    /// Write a QUIC variable-length integer.
    pub fn write_vint(&mut self, value: u64) -> Result<(), StreamError> {
        let len = varint::encode(&mut self.data[self.pos..], value);
        if len == 0 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.pos += len;
        Ok(())
    }

    /// Write a byte buffer.
    pub fn write_buffer(&mut self, buffer: &[u8]) -> Result<(), StreamError> {
        if self.remain() < buffer.len() {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.data[self.pos..self.pos + buffer.len()].copy_from_slice(buffer);
        self.pos += buffer.len();
        Ok(())
    }

    /// Write a length-prefixed string (varint length + bytes).
    pub fn write_cstr(&mut self, s: &str) -> Result<(), StreamError> {
        self.write_vint(s.len() as u64)?;
        self.write_buffer(s.as_bytes())
    }

    /// Write a connection ID (length byte + id bytes).
    pub fn write_cid(&mut self, cid: &ConnectionId) -> Result<(), StreamError> {
        self.write_u8(cid.id_len)?;
        self.write_buffer(cid.as_bytes())
    }

    /// Read a connection ID (length byte + id bytes).
    pub fn read_cid(&mut self) -> Result<ConnectionId, StreamError> {
        let id_len = self.read_u8()?;
        if id_len as usize > CONNECTION_ID_MAX_SIZE {
            return Err(StreamError);
        }
        let mut cid = ConnectionId::null();
        self.read_buffer(&mut cid.id[..id_len as usize])?;
        cid.id_len = id_len;
        Ok(cid)
    }

    /// Read a u8.
    pub fn read_u8(&mut self) -> Result<u8, StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = self.data[self.pos];
        self.pos += 1;
        Ok(value)
    }

    /// Peek a u8 without advancing position.
    pub fn peek_u8(&self) -> Result<u8, StreamError> {
        if self.remain() < 1 {
            return Err(StreamError);
        }
        Ok(self.data[self.pos])
    }

    /// Read a u16 in big-endian format.
    pub fn read_u16(&mut self) -> Result<u16, StreamError> {
        if self.remain() < 2 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_16(&self.data[self.pos..]);
        self.pos += 2;
        Ok(value)
    }

    /// Read a u32 in big-endian format.
    pub fn read_u32(&mut self) -> Result<u32, StreamError> {
        if self.remain() < 4 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_32(&self.data[self.pos..]);
        self.pos += 4;
        Ok(value)
    }

    /// Read a u64 in big-endian format.
    pub fn read_u64(&mut self) -> Result<u64, StreamError> {
        if self.remain() < 8 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_64(&self.data[self.pos..]);
        self.pos += 8;
        Ok(value)
    }

    /// Read a QUIC variable-length integer.
    pub fn read_vint(&mut self) -> Result<u64, StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let (value, len) = varint::decode(&self.data[self.pos..]);
        if len == 0 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.pos += len;
        Ok(value)
    }

    /// Skip a QUIC variable-length integer.
    pub fn skip_vint(&mut self) -> Result<(), StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let len = varint::decode_length(self.data[self.pos]);
        self.skip(len)
    }

    /// Read bytes into a buffer.
    pub fn read_buffer(&mut self, buffer: &mut [u8]) -> Result<(), StreamError> {
        if self.remain() < buffer.len() {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        buffer.copy_from_slice(&self.data[self.pos..self.pos + buffer.len()]);
        self.pos += buffer.len();
        Ok(())
    }

    /// Read a length-prefixed string into a buffer.
    /// Returns the number of bytes read (excluding null terminator).
    pub fn read_cstr(&mut self, buffer: &mut [u8]) -> Result<usize, StreamError> {
        let len = self.read_vint()? as usize;
        if len + 1 > buffer.len() {
            return Err(StreamError);
        }
        self.read_buffer(&mut buffer[..len])?;
        buffer[len] = 0; // null terminator
        Ok(len)
    }

    /// Skip a length-prefixed string.
    pub fn skip_cstr(&mut self) -> Result<(), StreamError> {
        let len = self.read_vint()? as usize;
        self.skip(len)
    }

    /// Read a varint as usize, checking for truncation.
    pub fn read_vlen(&mut self) -> Result<usize, StreamError> {
        let val = self.read_vint()?;
        let as_usize = val as usize;
        if as_usize as u64 != val {
            return Err(StreamError);
        }
        Ok(as_usize)
    }
}

impl<'a> ByteReader<'a> {
    /// Create a new read-only bytestream.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Get the underlying data slice.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Get a slice at the current position.
    pub fn ptr(&self) -> &[u8] {
        &self.data[self.pos..]
    }

    /// Get the total buffer size.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get the current position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Get remaining bytes available.
    pub fn remain(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Reset position to the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Check if at end of stream.
    pub fn finished(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Skip forward by `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<(), StreamError> {
        if self.remain() < n {
            self.pos = self.data.len();
            Err(StreamError)
        } else {
            self.pos += n;
            Ok(())
        }
    }

    /// Read a u8.
    pub fn read_u8(&mut self) -> Result<u8, StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = self.data[self.pos];
        self.pos += 1;
        Ok(value)
    }

    /// Peek a u8 without advancing.
    pub fn peek_u8(&self) -> Result<u8, StreamError> {
        if self.remain() < 1 {
            return Err(StreamError);
        }
        Ok(self.data[self.pos])
    }

    /// Read a u16 in big-endian format.
    pub fn read_u16(&mut self) -> Result<u16, StreamError> {
        if self.remain() < 2 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_16(&self.data[self.pos..]);
        self.pos += 2;
        Ok(value)
    }

    /// Read a u32 in big-endian format.
    pub fn read_u32(&mut self) -> Result<u32, StreamError> {
        if self.remain() < 4 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_32(&self.data[self.pos..]);
        self.pos += 4;
        Ok(value)
    }

    /// Read a u64 in big-endian format.
    pub fn read_u64(&mut self) -> Result<u64, StreamError> {
        if self.remain() < 8 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let value = intformat::parse_64(&self.data[self.pos..]);
        self.pos += 8;
        Ok(value)
    }

    /// Read a QUIC variable-length integer.
    pub fn read_vint(&mut self) -> Result<u64, StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let (value, len) = varint::decode(&self.data[self.pos..]);
        if len == 0 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        self.pos += len;
        Ok(value)
    }

    /// Skip a QUIC variable-length integer.
    pub fn skip_vint(&mut self) -> Result<(), StreamError> {
        if self.remain() < 1 {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        let len = varint::decode_length(self.data[self.pos]);
        self.skip(len)
    }

    /// Read bytes into a buffer.
    pub fn read_buffer(&mut self, buffer: &mut [u8]) -> Result<(), StreamError> {
        if self.remain() < buffer.len() {
            self.pos = self.data.len();
            return Err(StreamError);
        }
        buffer.copy_from_slice(&self.data[self.pos..self.pos + buffer.len()]);
        self.pos += buffer.len();
        Ok(())
    }

    /// Read a length-prefixed string.
    pub fn read_cstr(&mut self, max_len: usize) -> Result<String, StreamError> {
        let len = self.read_vint()? as usize;
        if len > max_len {
            return Err(StreamError);
        }
        let mut buf = vec![0u8; len];
        self.read_buffer(&mut buf)?;
        String::from_utf8(buf).map_err(|_| StreamError)
    }

    /// Skip a length-prefixed string.
    pub fn skip_cstr(&mut self) -> Result<(), StreamError> {
        let len = self.read_vint()? as usize;
        self.skip(len)
    }

    /// Read a connection ID (length byte + id bytes).
    pub fn read_cid(&mut self) -> Result<ConnectionId, StreamError> {
        let id_len = self.read_u8()?;
        if id_len as usize > CONNECTION_ID_MAX_SIZE {
            return Err(StreamError);
        }
        let mut cid = ConnectionId::null();
        self.read_buffer(&mut cid.id[..id_len as usize])?;
        cid.id_len = id_len;
        Ok(cid)
    }
}

impl ByteStreamBuf {
    /// Create a new bytestream with owned buffer.
    pub fn new(size: usize) -> Option<Self> {
        if size > MAX_BUFFER_SIZE {
            return None;
        }
        Some(Self {
            buf: vec![0u8; size],
            pos: 0,
        })
    }

    /// Get the underlying buffer.
    pub fn data(&self) -> &[u8] {
        &self.buf
    }

    /// Get written data (up to current position).
    pub fn written(&self) -> &[u8] {
        &self.buf[..self.pos]
    }

    /// Get a mutable bytestream view.
    pub fn as_stream(&mut self) -> ByteStream<'_> {
        ByteStream {
            data: &mut self.buf,
            pos: self.pos,
        }
    }

    /// Get the current position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Set the position (used after writing via as_stream).
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }
}

/// Get the encoded length of a QUIC varint.
#[inline]
pub fn vint_len(value: u64) -> usize {
    varint::encode_length(value)
}

// =============================================================================
// FFI exports - these replace the C implementations when FQ_USE_RUST is defined
// =============================================================================
//
// IMPORTANT: FFI functions are thin wrappers around the safe Rust API.
// All logic lives in ByteStream methods. FFI functions:
// 1. Convert C pointers to safe Rust types via ByteStream::from_c
// 2. Call the safe method
// 3. Sync state back via sync_to_c
// 4. Convert Result to C return code

/// Maximum size for stack-allocated bytestream buffers (matches C).
pub const BYTESTREAM_MAX_BUFFER_SIZE: usize = 2560;

/// C-compatible bytestream struct.
///
/// This must match the memory layout of the C `bytestream` typedef.
#[repr(C)]
pub struct CBytestream {
    pub data: *mut u8,
    pub size: usize,
    pub ptr: usize,
}

impl CBytestream {
    /// Initialize to reference existing bytes.
    ///
    /// # Safety
    /// `data` must point to a valid buffer of at least `size` bytes, or be null if size is 0.
    pub unsafe fn init_ref(&mut self, data: *mut u8, size: usize) {
        self.data = data;
        self.size = size;
        self.ptr = 0;
    }

    /// Allocate a buffer of the given size.
    /// Returns true on success, false on allocation failure.
    pub fn alloc(&mut self, size: usize) -> bool {
        let layout = match std::alloc::Layout::from_size_align(size, 1) {
            Ok(l) => l,
            Err(_) => return false,
        };
        // SAFETY: layout is valid (size, align=1)
        let data = unsafe { std::alloc::alloc(layout) };
        if data.is_null() {
            return false;
        }
        self.data = data;
        self.size = size;
        self.ptr = 0;
        true
    }

    /// Free the allocated buffer.
    ///
    /// # Safety
    /// If `data` is non-null, it must have been allocated by `alloc` with the same `size`.
    pub unsafe fn free(&mut self) {
        if !self.data.is_null() {
            let layout = std::alloc::Layout::from_size_align(self.size, 1).unwrap();
            std::alloc::dealloc(self.data, layout);
            self.data = std::ptr::null_mut();
        }
    }
}

/// C-compatible bytestream_buf struct.
///
/// This must match the memory layout of the C `bytestream_buf` typedef.
#[repr(C)]
pub struct CBytestreamBuf {
    pub s: CBytestream,
    pub buf: [u8; BYTESTREAM_MAX_BUFFER_SIZE],
}

impl CBytestreamBuf {
    /// Initialize to use the embedded buffer.
    /// Returns None if size exceeds BYTESTREAM_MAX_BUFFER_SIZE.
    pub fn init(&mut self, size: usize) -> Option<&mut CBytestream> {
        if size > BYTESTREAM_MAX_BUFFER_SIZE {
            return None;
        }
        self.s.data = self.buf.as_mut_ptr();
        self.s.size = size;
        self.s.ptr = 0;
        Some(&mut self.s)
    }
}

// =============================================================================
// FFI exports for initialization/allocation (thin wrappers)
// =============================================================================

/// FFI export: Initialize a bytestream to reference existing bytes.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct.
/// - `bytes` must point to a valid buffer of at least `nb_bytes` bytes (or be null if nb_bytes is 0).
#[no_mangle]
pub unsafe extern "C" fn bytestream_ref_init(
    s: *mut CBytestream,
    bytes: *const std::ffi::c_void,
    nb_bytes: usize,
) -> *mut CBytestream {
    (*s).init_ref(bytes as *mut u8, nb_bytes);
    s
}

/// FFI export: Initialize a bytestream_buf with its embedded buffer.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream_buf` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_buf_init(
    s: *mut CBytestreamBuf,
    nb_bytes: usize,
) -> *mut CBytestream {
    match (*s).init(nb_bytes) {
        Some(bs) => bs,
        None => std::ptr::null_mut(),
    }
}

/// FFI export: Allocate a bytestream with malloc'd buffer.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_alloc(
    s: *mut CBytestream,
    nb_bytes: usize,
) -> *mut CBytestream {
    if (*s).alloc(nb_bytes) {
        s
    } else {
        std::ptr::null_mut()
    }
}

/// FFI export: Free a bytestream's allocated buffer.
///
/// # Safety
/// - `s` must point to a valid `bytestream` struct.
/// - If `s.data` is non-null, it must have been allocated by `bytestream_alloc`.
#[no_mangle]
pub unsafe extern "C" fn bytestream_delete(s: *mut CBytestream) {
    (*s).free();
}

// =============================================================================
// Accessor functions (direct field access, trivial wrappers)
// =============================================================================

/// FFI export: Get pointer to start of bytestream data.
///
/// # Safety
/// - `s` must point to a valid `bytestream` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_data(s: *const CBytestream) -> *const u8 {
    (*s).data
}

/// FFI export: Get pointer to current position in bytestream.
///
/// # Safety
/// - `s` must point to a valid `bytestream` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_ptr(s: *const CBytestream) -> *const u8 {
    (*s).data.add((*s).ptr)
}

/// FFI export: Get total size of bytestream buffer.
///
/// # Safety
/// - `s` must point to a valid `bytestream` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_size(s: *const CBytestream) -> usize {
    (*s).size
}

/// FFI export: Get current position (bytes written/read).
///
/// # Safety
/// - `s` must point to a valid `bytestream` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_length(s: *const CBytestream) -> usize {
    (*s).ptr
}

/// FFI export: Get remaining bytes available.
///
/// # Safety
/// - `s` must point to a valid `bytestream` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_remain(s: *const CBytestream) -> usize {
    (*s).size - (*s).ptr
}

/// FFI export: Reset position to beginning.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_reset(s: *mut CBytestream) {
    let mut stream = ByteStream::from_c(&mut *s);
    stream.reset();
    stream.sync_to_c(&mut *s);
}

/// FFI export: Clear buffer and reset position.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct.
/// - `s.data` must point to a writable buffer of at least `s.size` bytes.
#[no_mangle]
pub unsafe extern "C" fn bytestream_clear(s: *mut CBytestream) {
    let mut stream = ByteStream::from_c(&mut *s);
    stream.clear();
    stream.sync_to_c(&mut *s);
}

/// FFI export: Check if stream is at end.
///
/// # Safety
/// - `s` must point to a valid `bytestream` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_finished(s: *const CBytestream) -> std::ffi::c_int {
    // Note: can't use from_c here because s is const, but finished() doesn't modify
    if (*s).ptr >= (*s).size {
        1
    } else {
        0
    }
}

/// FFI export: Get encoded length of a varint.
#[no_mangle]
pub extern "C" fn bytestream_vint_len(value: u64) -> usize {
    vint_len(value)
}

// =============================================================================
// Stream operations (wrap safe ByteStream methods)
// =============================================================================

/// FFI export: Skip forward by n bytes.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct.
#[no_mangle]
pub unsafe extern "C" fn bytestream_skip(s: *mut CBytestream, nb_bytes: usize) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.skip(nb_bytes) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Skip a varint without decoding its value.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
#[no_mangle]
pub unsafe extern "C" fn byteread_skip_vint(s: *mut CBytestream) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.skip_vint() {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Write a varint.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data buffer.
#[no_mangle]
pub unsafe extern "C" fn bytewrite_vint(s: *mut CBytestream, value: u64) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.write_vint(value) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Read a varint.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `value` must point to a valid, writable u64.
#[no_mangle]
pub unsafe extern "C" fn byteread_vint(s: *mut CBytestream, value: *mut u64) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.read_vint() {
        Ok(v) => {
            *value = v;
            0
        }
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Read a varint as size_t, checking for truncation.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `value` must point to a valid, writable size_t.
#[no_mangle]
pub unsafe extern "C" fn byteread_vlen(s: *mut CBytestream, value: *mut usize) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.read_vlen() {
        Ok(v) => {
            *value = v;
            0
        }
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Write a u8.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data buffer.
#[no_mangle]
pub unsafe extern "C" fn bytewrite_int8(s: *mut CBytestream, value: u8) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.write_u8(value) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Read a u8.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `value` must point to a valid, writable u8.
#[no_mangle]
pub unsafe extern "C" fn byteread_int8(s: *mut CBytestream, value: *mut u8) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.read_u8() {
        Ok(v) => {
            *value = v;
            0
        }
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Peek a u8 without advancing position.
///
/// # Safety
/// - `s` must point to a valid `bytestream` struct with valid data.
/// - `value` must point to a valid, writable u8.
#[no_mangle]
pub unsafe extern "C" fn byteshow_int8(s: *mut CBytestream, value: *mut u8) -> std::ffi::c_int {
    let stream = ByteStream::from_c(&mut *s);
    match stream.peek_u8() {
        Ok(v) => {
            *value = v;
            0
        }
        Err(_) => -1,
    }
    // Note: no sync needed, peek doesn't modify position
}

/// FFI export: Write a u16 in big-endian format.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data buffer.
#[no_mangle]
pub unsafe extern "C" fn bytewrite_int16(s: *mut CBytestream, value: u16) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.write_u16(value) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Read a u16 in big-endian format.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `value` must point to a valid, writable u16.
#[no_mangle]
pub unsafe extern "C" fn byteread_int16(s: *mut CBytestream, value: *mut u16) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.read_u16() {
        Ok(v) => {
            *value = v;
            0
        }
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Write a u32 in big-endian format.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data buffer.
#[no_mangle]
pub unsafe extern "C" fn bytewrite_int32(s: *mut CBytestream, value: u32) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.write_u32(value) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Read a u32 in big-endian format.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `value` must point to a valid, writable u32.
#[no_mangle]
pub unsafe extern "C" fn byteread_int32(s: *mut CBytestream, value: *mut u32) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.read_u32() {
        Ok(v) => {
            *value = v;
            0
        }
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Write a u64 in big-endian format.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data buffer.
#[no_mangle]
pub unsafe extern "C" fn bytewrite_int64(s: *mut CBytestream, value: u64) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.write_u64(value) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Read a u64 in big-endian format.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `value` must point to a valid, writable u64.
#[no_mangle]
pub unsafe extern "C" fn byteread_int64(s: *mut CBytestream, value: *mut u64) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.read_u64() {
        Ok(v) => {
            *value = v;
            0
        }
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Write a buffer.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data buffer.
/// - `buffer` must point to a valid buffer of at least `length` bytes.
#[no_mangle]
pub unsafe extern "C" fn bytewrite_buffer(
    s: *mut CBytestream,
    buffer: *const std::ffi::c_void,
    length: usize,
) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let buf_slice = std::slice::from_raw_parts(buffer as *const u8, length);
    let result = match stream.write_buffer(buf_slice) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Read into a buffer.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `buffer` must point to a valid, writable buffer of at least `length` bytes.
#[no_mangle]
pub unsafe extern "C" fn byteread_buffer(
    s: *mut CBytestream,
    buffer: *mut std::ffi::c_void,
    length: usize,
) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let buf_slice = std::slice::from_raw_parts_mut(buffer as *mut u8, length);
    let result = match stream.read_buffer(buf_slice) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Write a length-prefixed C string.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data buffer.
/// - `cstr` must point to a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn bytewrite_cstr(
    s: *mut CBytestream,
    cstr: *const std::ffi::c_char,
) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let len = libc::strlen(cstr);
    let str_slice = std::slice::from_raw_parts(cstr as *const u8, len);
    // Convert to &str for write_cstr (it expects valid UTF-8, but we just need bytes)
    // Use write_vint + write_buffer directly to avoid UTF-8 requirement
    let result = match stream
        .write_vint(len as u64)
        .and_then(|()| stream.write_buffer(str_slice))
    {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Read a length-prefixed string into a C string buffer.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `cstr` must point to a valid, writable buffer of at least `max_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn byteread_cstr(
    s: *mut CBytestream,
    cstr: *mut std::ffi::c_char,
    max_len: usize,
) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let buf_slice = std::slice::from_raw_parts_mut(cstr as *mut u8, max_len);
    let result = match stream.read_cstr(buf_slice) {
        Ok(_len) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Skip a length-prefixed string.
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
#[no_mangle]
pub unsafe extern "C" fn byteskip_cstr(s: *mut CBytestream) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.skip_cstr() {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Skip a connection ID (length byte + id bytes).
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
#[no_mangle]
pub unsafe extern "C" fn byteskip_cid(s: *mut CBytestream) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.read_u8().and_then(|len| stream.skip(len as usize)) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Write a connection ID (length byte + id bytes).
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `cid` must point to a valid ConnectionId struct.
#[no_mangle]
pub unsafe extern "C" fn bytewrite_cid(
    s: *mut CBytestream,
    cid: *const crate::util::ConnectionId,
) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.write_cid(&*cid) {
        Ok(()) => 0,
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

/// FFI export: Read a connection ID (length byte + id bytes).
///
/// # Safety
/// - `s` must point to a valid, writable `bytestream` struct with valid data.
/// - `cid` must point to a valid, writable ConnectionId struct.
#[no_mangle]
pub unsafe extern "C" fn byteread_cid(
    s: *mut CBytestream,
    cid: *mut crate::util::ConnectionId,
) -> std::ffi::c_int {
    let mut stream = ByteStream::from_c(&mut *s);
    let result = match stream.read_cid() {
        Ok(read_cid) => {
            *cid = read_cid;
            0
        }
        Err(_) => -1,
    };
    stream.sync_to_c(&mut *s);
    result
}

// Note: bytewrite_addr, byteread_addr, byteskip_addr are NOT implemented here
// because they depend on platform-specific types (struct sockaddr, struct sockaddr_storage).
// These remain in the C implementation.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_read_u8() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.write_u8(0x42).is_ok());
        assert_eq!(stream.length(), 1);

        stream.reset();
        assert_eq!(stream.read_u8(), Ok(0x42));
    }

    #[test]
    fn test_write_read_u16() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.write_u16(0x1234).is_ok());
        assert_eq!(stream.length(), 2);

        stream.reset();
        assert_eq!(stream.read_u16(), Ok(0x1234));
    }

    #[test]
    fn test_write_read_u32() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.write_u32(0x12345678).is_ok());
        assert_eq!(stream.length(), 4);

        stream.reset();
        assert_eq!(stream.read_u32(), Ok(0x12345678));
    }

    #[test]
    fn test_write_read_u64() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.write_u64(0x123456789ABCDEF0).is_ok());
        assert_eq!(stream.length(), 8);

        stream.reset();
        assert_eq!(stream.read_u64(), Ok(0x123456789ABCDEF0));
    }

    #[test]
    fn test_write_read_vint() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);

        // 1-byte varint
        assert!(stream.write_vint(37).is_ok());
        assert_eq!(stream.length(), 1);

        // 2-byte varint
        assert!(stream.write_vint(15293).is_ok());
        assert_eq!(stream.length(), 3);

        stream.reset();
        assert_eq!(stream.read_vint(), Ok(37));
        assert_eq!(stream.read_vint(), Ok(15293));
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buf = [0u8; 2];
        let mut stream = ByteStream::new(&mut buf);

        assert!(stream.write_u32(0x12345678).is_err());
        assert!(stream.finished());
    }

    #[test]
    fn test_skip() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        stream.write_u8(1).unwrap();
        stream.write_u8(2).unwrap();
        stream.write_u8(3).unwrap();

        stream.reset();
        assert!(stream.skip(2).is_ok());
        assert_eq!(stream.read_u8(), Ok(3));
    }

    #[test]
    fn test_skip_overflow() {
        let mut buf = [0u8; 5];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.skip(10).is_err());
        assert!(stream.finished());
    }

    #[test]
    fn test_write_read_buffer() {
        let mut buf = [0u8; 20];
        let mut stream = ByteStream::new(&mut buf);

        let data = b"hello";
        assert!(stream.write_buffer(data).is_ok());
        assert_eq!(stream.length(), 5);

        stream.reset();
        let mut out = [0u8; 5];
        assert!(stream.read_buffer(&mut out).is_ok());
        assert_eq!(&out, b"hello");
    }

    #[test]
    fn test_peek_u8() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        stream.write_u8(0x42).unwrap();

        stream.reset();
        assert_eq!(stream.peek_u8(), Ok(0x42));
        assert_eq!(stream.position(), 0); // position unchanged
        assert_eq!(stream.read_u8(), Ok(0x42));
        assert_eq!(stream.position(), 1); // position advanced
    }

    #[test]
    fn test_bytereader() {
        let data = [0x12, 0x34, 0x56, 0x78];
        let mut reader = ByteReader::new(&data);

        assert_eq!(reader.read_u16(), Ok(0x1234));
        assert_eq!(reader.read_u16(), Ok(0x5678));
        assert!(reader.finished());
    }

    #[test]
    fn test_bytereader_cstr() {
        // varint length (5) followed by "hello"
        let data = [0x05, b'h', b'e', b'l', b'l', b'o'];
        let mut reader = ByteReader::new(&data);

        assert_eq!(reader.read_cstr(100), Ok("hello".to_string()));
    }

    #[test]
    fn test_bytestream_buf() {
        let mut buf = ByteStreamBuf::new(100).unwrap();
        let len = {
            let mut stream = buf.as_stream();
            stream.write_u32(0xDEADBEEF).unwrap();
            stream.length()
        };
        buf.set_position(len);
        assert_eq!(buf.position(), 4);
        assert_eq!(&buf.written()[..4], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_bytestream_buf_too_large() {
        assert!(ByteStreamBuf::new(MAX_BUFFER_SIZE + 1).is_none());
    }

    #[test]
    fn test_remain() {
        let mut buf = [0u8; 10];
        let mut stream = ByteStream::new(&mut buf);
        assert_eq!(stream.remain(), 10);
        stream.write_u32(0).unwrap();
        assert_eq!(stream.remain(), 6);
    }

    #[test]
    fn test_write_read_cid() {
        use crate::util::ConnectionId;

        let cid = ConnectionId::from_bytes(&[0xde, 0xad, 0xbe, 0xef]).unwrap();

        let mut buf = [0u8; 20];
        let mut stream = ByteStream::new(&mut buf);
        assert!(stream.write_cid(&cid).is_ok());
        assert_eq!(stream.length(), 5); // 1 byte len + 4 bytes id

        stream.reset();
        let read_cid = stream.read_cid().unwrap();
        assert_eq!(read_cid, cid);
    }

    #[test]
    fn test_read_cid_invalid_length() {
        // Length byte > CONNECTION_ID_MAX_SIZE (20)
        let data = [25u8, 0, 0, 0, 0]; // length 25, invalid
        let mut reader = ByteReader::new(&data);
        assert!(reader.read_cid().is_err());
    }

    impl ByteStream<'_> {
        fn position(&self) -> usize {
            self.pos
        }
    }
}
