//! General utility functions.
//!
//! This module provides:
//! - String creation/duplication
//! - Hex encoding/decoding
//! - Connection ID handling
//! - Constant-time memory comparison
//! - Test random number generation
//! - Frame skip/decode/encode helpers
//!
//! Translated from picoquic/util.c

use crate::intformat::{self, varint};
use std::cmp::Ordering;
use std::ffi::c_int;

// =============================================================================
// Connection ID Constants
// =============================================================================

/// Minimum connection ID size (0 bytes).
pub const CONNECTION_ID_MIN_SIZE: usize = 0;

/// Maximum connection ID size (20 bytes).
pub const CONNECTION_ID_MAX_SIZE: usize = 20;

// =============================================================================
// Connection ID
// =============================================================================

/// A QUIC connection ID.
///
/// Connection IDs are variable-length identifiers (0-20 bytes) used to
/// identify QUIC connections at endpoints. They allow connection migration
/// and load balancing.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ConnectionId {
    /// Connection ID bytes (only first `id_len` bytes are valid).
    pub id: [u8; CONNECTION_ID_MAX_SIZE],
    /// Length of the connection ID (0-20).
    pub id_len: u8,
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::null()
    }
}

impl std::fmt::Debug for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectionId(")?;
        for i in 0..self.id_len as usize {
            write!(f, "{:02x}", self.id[i])?;
        }
        write!(f, ")")
    }
}

impl ConnectionId {
    /// Create a null (zero-length) connection ID.
    pub const fn null() -> Self {
        Self {
            id: [0; CONNECTION_ID_MAX_SIZE],
            id_len: 0,
        }
    }

    /// Create a connection ID from bytes.
    ///
    /// Returns None if `bytes` is longer than `CONNECTION_ID_MAX_SIZE`.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > CONNECTION_ID_MAX_SIZE {
            return None;
        }
        let mut cid = Self::null();
        cid.id[..bytes.len()].copy_from_slice(bytes);
        cid.id_len = bytes.len() as u8;
        Some(cid)
    }

    /// Get the connection ID bytes as a slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.id[..self.id_len as usize]
    }

    /// Check if this is a null (zero-length) connection ID.
    pub fn is_null(&self) -> bool {
        self.id_len == 0
    }

    /// Get the length of the connection ID.
    pub fn len(&self) -> usize {
        self.id_len as usize
    }

    /// Check if the connection ID is empty (same as is_null).
    pub fn is_empty(&self) -> bool {
        self.id_len == 0
    }

    /// Format the connection ID into a byte buffer.
    ///
    /// Returns the number of bytes written, or 0 if the buffer is too small
    /// or the connection ID is empty.
    pub fn format(&self, bytes: &mut [u8]) -> u8 {
        if self.id_len == 0 || bytes.len() < self.id_len as usize {
            return 0;
        }
        bytes[..self.id_len as usize].copy_from_slice(&self.id[..self.id_len as usize]);
        self.id_len
    }

    /// Parse a connection ID from bytes with known length.
    ///
    /// Returns the number of bytes consumed, or 0 on error.
    pub fn parse(&mut self, bytes: &[u8], len: u8) -> u8 {
        if len as usize > CONNECTION_ID_MAX_SIZE || bytes.len() < len as usize {
            self.id_len = 0;
            return 0;
        }
        self.id = [0; CONNECTION_ID_MAX_SIZE];
        self.id[..len as usize].copy_from_slice(&bytes[..len as usize]);
        self.id_len = len;
        len
    }

    /// Compare two connection IDs.
    ///
    /// Returns ordering suitable for sorting.
    pub fn compare(&self, other: &Self) -> Ordering {
        match self.id_len.cmp(&other.id_len) {
            Ordering::Equal => self.as_bytes().cmp(other.as_bytes()),
            other_ord => other_ord,
        }
    }

    /// Compute a simple hash of the connection ID.
    ///
    /// This is a fast, non-cryptographic hash suitable for hash tables.
    /// For security-sensitive hashing, use `hash_siphash`.
    pub fn hash(&self) -> u64 {
        let mut val64: u64 = 0;
        let mut i = 0usize;

        // First 8 bytes: simple shift and add
        while i < self.id_len as usize && i < 8 {
            val64 <<= 8;
            val64 += self.id[i] as u64;
            i += 1;
        }

        // Remaining bytes: fold in with multiplication
        while i < self.id_len as usize {
            let top = val64 >> 56;
            val64 <<= 8;
            val64 += self.id[i] as u64;
            val64 = val64.wrapping_add(top.wrapping_mul(0x10001));
            i += 1;
        }

        val64
    }

    /// Get a 64-bit value representation of the connection ID.
    ///
    /// For IDs shorter than 8 bytes, pads with zeros on the right.
    /// For IDs longer than 8 bytes, uses only the first 8 bytes.
    pub fn val64(&self) -> u64 {
        let mut val64: u64 = 0;

        if self.id_len < 8 {
            for i in 0..self.id_len as usize {
                val64 <<= 8;
                val64 |= self.id[i] as u64;
            }
            // Pad remaining with zeros
            for _ in self.id_len as usize..8 {
                val64 <<= 8;
            }
        } else {
            for i in 0..8 {
                val64 <<= 8;
                val64 |= self.id[i] as u64;
            }
        }

        val64
    }

    /// Format connection ID as a hex string.
    ///
    /// Returns the number of hex characters written (excluding null terminator),
    /// or None if buffer is too small.
    pub fn format_hexa(&self, buf: &mut [u8]) -> Option<usize> {
        let required = self.id_len as usize * 2 + 1;
        if buf.len() < required {
            return None;
        }

        for i in 0..self.id_len as usize {
            buf[i * 2] = HEX_CHARS[self.id[i] as usize >> 4];
            buf[i * 2 + 1] = HEX_CHARS[self.id[i] as usize & 0x0f];
        }
        buf[self.id_len as usize * 2] = 0;

        Some(self.id_len as usize * 2)
    }

    /// Parse connection ID from a hex string.
    ///
    /// Returns the number of bytes parsed, or 0 on error.
    pub fn parse_hexa(&mut self, hex_input: &[u8]) -> u8 {
        *self = Self::null();
        // Use the general hex parser, limiting to 18 bytes (36 hex chars)
        // which is the limit used in C code (slightly less than max 20)
        let max_bytes = 18.min(CONNECTION_ID_MAX_SIZE);
        let id_len = parse_hexa(hex_input, &mut self.id[..max_bytes]);
        if id_len == 0 {
            *self = Self::null();
            0
        } else {
            self.id_len = id_len as u8;
            self.id_len
        }
    }
}

// =============================================================================
// String utilities
// =============================================================================

/// Create a new string by copying `len` bytes from `original`.
/// Returns None on allocation failure or integer overflow.
pub fn string_create(original: Option<&[u8]>, len: usize) -> Option<Vec<u8>> {
    let allocated = len.checked_add(1)?;
    let mut str_vec = Vec::with_capacity(allocated);

    match original {
        None | Some(&[]) => {
            str_vec.push(0);
        }
        Some(data) if data.len() >= len => {
            str_vec.extend_from_slice(&data[..len]);
            str_vec.push(0);
        }
        _ => return None,
    }

    Some(str_vec)
}

/// Duplicate a null-terminated string.
pub fn string_duplicate(original: &[u8]) -> Option<Vec<u8>> {
    // Find null terminator or use full length
    let len = original
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(original.len());
    string_create(Some(original), len)
}

// =============================================================================
// Hex encoding/decoding
// =============================================================================

/// Parse a single hex digit. Returns None if not a valid hex character.
pub fn parse_hexa_digit(x: u8) -> Option<u8> {
    match x {
        b'0'..=b'9' => Some(x - b'0'),
        b'A'..=b'F' => Some(x - b'A' + 10),
        b'a'..=b'f' => Some(x - b'a' + 10),
        _ => None,
    }
}

/// Parse a hex string into bytes.
/// Returns the number of bytes written, or 0 on error.
pub fn parse_hexa(hex_input: &[u8], bin_output: &mut [u8]) -> usize {
    let input_length = hex_input.len();

    // Must have even length and fit in output
    if input_length == 0 || (input_length & 1) != 0 || bin_output.len() * 2 < input_length {
        return 0;
    }

    let mut ret = 0;
    let mut offset = 0;

    while offset < input_length {
        let a = match parse_hexa_digit(hex_input[offset]) {
            Some(v) => v,
            None => return 0,
        };
        let b = match parse_hexa_digit(hex_input[offset + 1]) {
            Some(v) => v,
            None => return 0,
        };
        bin_output[ret] = (a << 4) | b;
        ret += 1;
        offset += 2;
    }

    ret
}

/// Hex character lookup table.
const HEX_CHARS: [u8; 16] = *b"0123456789abcdef";

/// Encode bytes as hex string.
/// Returns the number of characters written, or 0 if buffer too small.
pub fn format_hexa(bytes: &[u8], output: &mut [u8]) -> usize {
    let required = bytes.len() * 2 + 1;
    if output.len() < required {
        return 0;
    }

    for (i, &byte) in bytes.iter().enumerate() {
        output[i * 2] = HEX_CHARS[(byte >> 4) as usize];
        output[i * 2 + 1] = HEX_CHARS[(byte & 0x0f) as usize];
    }
    output[bytes.len() * 2] = 0;

    bytes.len() * 2
}

// =============================================================================
// Constant-time memory comparison
// =============================================================================

/// Constant-time memory comparison.
/// Returns 0 if equal, -1 otherwise.
pub fn constant_time_memcmp(x: &[u8], y: &[u8]) -> i32 {
    if x.len() != y.len() {
        return -1;
    }

    let mut ret: u64 = 0;
    for (a, b) in x.iter().zip(y.iter()) {
        ret += (*a ^ *b) as u64;
    }

    if ret == 0 {
        0
    } else {
        -1
    }
}

// =============================================================================
// Test random number generation (deterministic PRNG for tests)
// =============================================================================

/// Deterministic PRNG for tests (splitmix64).
/// Given the same seed, produces the same sequence.
pub fn test_random(random_context: &mut u64) -> u64 {
    *random_context = random_context.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *random_context;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

/// Fill a buffer with deterministic random bytes.
pub fn test_random_bytes(random_context: &mut u64, bytes: &mut [u8]) {
    let mut byte_index = 0;

    while byte_index < bytes.len() {
        let mut v = test_random(random_context);

        for _ in 0..8 {
            if byte_index >= bytes.len() {
                break;
            }
            bytes[byte_index] = (v & 0xFF) as u8;
            v >>= 8;
            byte_index += 1;
        }
    }
}

/// Uniform random in [0, rnd_max).
pub fn test_uniform_random(random_context: &mut u64, rnd_max: u64) -> u64 {
    if rnd_max == 0 {
        return 0;
    }

    let rnd_min = u64::MAX % rnd_max;
    let mut rnd;

    loop {
        rnd = test_random(random_context);
        if rnd >= rnd_min {
            break;
        }
    }

    rnd % rnd_max
}

/// Gaussian random with mean=0, stdev=1.
/// Uses sum of 12 uniform randoms (Central Limit Theorem approximation).
pub fn test_gauss_random(random_context: &mut u64) -> f64 {
    let mut dx: f64 = 0.0;

    for _ in 0..12 {
        let mut r = test_random(random_context);
        r ^= r >> 17;
        r ^= r >> 34;
        let d = ((r & 0x1ffff) as f64 + 0.5) / (0x20000 as f64);
        dx += d;
    }

    dx - 6.0
}

/// Poisson random variable.
/// `exp_minus_lambda_2_30` is `(exp(-lambda) * 2^30)` as fixed-point.
pub fn test_poisson_random(random_context: &mut u64, exp_minus_lambda_2_30: u64) -> u64 {
    let mut k: u64 = 0;
    let mut p: u64 = 0x40000000;

    loop {
        let mut r = test_random(random_context);
        r ^= r >> 30;
        r &= 0x3fffffff;
        p = (p.wrapping_mul(r)) >> 30;
        k += 1;
        if p <= exp_minus_lambda_2_30 {
            break;
        }
    }

    k - 1
}

// =============================================================================
// Frame skip/decode helpers (pointer-advancing pattern)
// =============================================================================

/// Skip a fixed number of bytes.
/// Returns remaining slice, or None if not enough bytes.
pub fn frames_fixed_skip(bytes: &[u8], size: usize) -> Option<&[u8]> {
    if size <= bytes.len() {
        Some(&bytes[size..])
    } else {
        None
    }
}

/// Skip a varint.
/// Returns remaining slice, or None if invalid.
pub fn frames_varint_skip(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.is_empty() {
        return None;
    }
    let v_len = varint::decode_length(bytes[0]);
    frames_fixed_skip(bytes, v_len)
}

/// Decode a varint, returning (value, remaining slice).
/// Returns None on error.
pub fn frames_varint_decode(bytes: &[u8]) -> Option<(u64, &[u8])> {
    if bytes.is_empty() {
        return None;
    }
    let length = varint::decode_length(bytes[0]);
    if length > bytes.len() {
        return None;
    }

    let (value, _) = varint::decode(bytes);
    Some((value, &bytes[length..]))
}

/// Decode a varint as usize.
/// Returns None on error or if value doesn't fit in usize.
pub fn frames_varlen_decode(bytes: &[u8]) -> Option<(usize, &[u8])> {
    let (len, rest) = frames_varint_decode(bytes)?;
    let n = len as usize;
    if n as u64 != len {
        return None;
    }
    Some((n, rest))
}

/// Decode a u8.
pub fn frames_uint8_decode(bytes: &[u8]) -> Option<(u8, &[u8])> {
    if bytes.is_empty() {
        return None;
    }
    Some((bytes[0], &bytes[1..]))
}

/// Decode a u16 (big-endian).
pub fn frames_uint16_decode(bytes: &[u8]) -> Option<(u16, &[u8])> {
    if bytes.len() < 2 {
        return None;
    }
    let n = intformat::parse_16(bytes);
    Some((n, &bytes[2..]))
}

/// Decode a u32 (big-endian).
pub fn frames_uint32_decode(bytes: &[u8]) -> Option<(u32, &[u8])> {
    if bytes.len() < 4 {
        return None;
    }
    let n = intformat::parse_32(bytes);
    Some((n, &bytes[4..]))
}

/// Decode a u64 (big-endian).
pub fn frames_uint64_decode(bytes: &[u8]) -> Option<(u64, &[u8])> {
    if bytes.len() < 8 {
        return None;
    }
    let n = intformat::parse_64(bytes);
    Some((n, &bytes[8..]))
}

/// Skip length-prefixed data (varint length + data).
pub fn frames_length_data_skip(bytes: &[u8]) -> Option<&[u8]> {
    let (length, rest) = frames_varint_decode(bytes)?;
    frames_fixed_skip(rest, length as usize)
}

// =============================================================================
// Frame encode helpers (mutable slice pattern)
// =============================================================================

/// Encode a varint, returning remaining slice.
/// Returns None if buffer too small.
pub fn frames_varint_encode(bytes: &mut [u8], n64: u64) -> Option<&mut [u8]> {
    let len = varint::encode(bytes, n64);
    if len == 0 {
        return None;
    }
    Some(&mut bytes[len..])
}

/// Encode a u8, returning remaining slice.
pub fn frames_uint8_encode(bytes: &mut [u8], n: u8) -> Option<&mut [u8]> {
    if bytes.is_empty() {
        return None;
    }
    bytes[0] = n;
    Some(&mut bytes[1..])
}

/// Encode a u16 (big-endian), returning remaining slice.
pub fn frames_uint16_encode(bytes: &mut [u8], n: u16) -> Option<&mut [u8]> {
    if bytes.len() < 2 {
        return None;
    }
    intformat::format_16(bytes, n);
    Some(&mut bytes[2..])
}

/// Encode a u24 (big-endian), returning remaining slice.
pub fn frames_uint24_encode(bytes: &mut [u8], n: u32) -> Option<&mut [u8]> {
    if bytes.len() < 3 {
        return None;
    }
    intformat::format_24(bytes, n);
    Some(&mut bytes[3..])
}

/// Encode a u32 (big-endian), returning remaining slice.
pub fn frames_uint32_encode(bytes: &mut [u8], n: u32) -> Option<&mut [u8]> {
    if bytes.len() < 4 {
        return None;
    }
    intformat::format_32(bytes, n);
    Some(&mut bytes[4..])
}

/// Encode a u64 (big-endian), returning remaining slice.
pub fn frames_uint64_encode(bytes: &mut [u8], n: u64) -> Option<&mut [u8]> {
    if bytes.len() < 8 {
        return None;
    }
    intformat::format_64(bytes, n);
    Some(&mut bytes[8..])
}

/// Encode length-prefixed data, returning remaining slice.
pub fn frames_length_data_encode<'a>(bytes: &'a mut [u8], data: &[u8]) -> Option<&'a mut [u8]> {
    let bytes = frames_varint_encode(bytes, data.len() as u64)?;
    if bytes.len() < data.len() {
        return None;
    }
    bytes[..data.len()].copy_from_slice(data);
    Some(&mut bytes[data.len()..])
}

/// Predict the encoded length of a varint.
pub fn frames_varint_encode_length(n64: u64) -> usize {
    varint::encode_length(n64)
}

// =============================================================================
// Binary to string conversion
// =============================================================================

/// Convert binary data to printable string, replacing non-printable chars with '?'.
/// Truncates with "..." if too long.
pub fn uint8_to_str(data: &[u8], text: &mut [u8]) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut render_length = data.len();
    if render_length + 1 > text.len() {
        render_length = if text.len() > 4 { text.len() - 4 } else { 0 };
    }

    for (i, &byte) in data.iter().take(render_length).enumerate() {
        text[i] = if (b' '..127).contains(&byte) {
            byte
        } else {
            b'?'
        };
    }

    let mut rendered = render_length;
    if rendered < data.len() {
        for _ in 0..3 {
            if rendered + 1 >= text.len() {
                break;
            }
            text[rendered] = b'.';
            rendered += 1;
        }
    }
    text[rendered] = 0;

    rendered
}

// =============================================================================
// FFI exports
// =============================================================================

/// FFI export: Parse a hex digit.
#[no_mangle]
pub extern "C" fn picoquic_parse_hexa_digit(x: std::ffi::c_char) -> std::ffi::c_int {
    match parse_hexa_digit(x as u8) {
        Some(v) => v as std::ffi::c_int,
        None => -1,
    }
}

/// FFI export: Parse hex string to bytes.
///
/// # Safety
/// - `hex_input` must point to valid data of `input_length` bytes.
/// - `bin_output` must point to valid writable buffer of `output_max` bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_hexa(
    hex_input: *const std::ffi::c_char,
    input_length: usize,
    bin_output: *mut u8,
    output_max: usize,
) -> usize {
    let input = std::slice::from_raw_parts(hex_input as *const u8, input_length);
    let output = std::slice::from_raw_parts_mut(bin_output, output_max);
    parse_hexa(input, output)
}

/// FFI export: Constant-time memory comparison.
///
/// # Safety
/// - `x` must point to valid data of `l` bytes.
/// - `y` must point to valid data of `l` bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_constant_time_memcmp(
    x: *const u8,
    y: *const u8,
    l: usize,
) -> std::ffi::c_int {
    let x_slice = std::slice::from_raw_parts(x, l);
    let y_slice = std::slice::from_raw_parts(y, l);
    constant_time_memcmp(x_slice, y_slice)
}

/// FFI export: Test random (splitmix64).
///
/// # Safety
/// - `random_context` must point to a valid u64.
#[no_mangle]
pub unsafe extern "C" fn picoquic_test_random(random_context: *mut u64) -> u64 {
    test_random(&mut *random_context)
}

/// FFI export: Fill buffer with test random bytes.
///
/// # Safety
/// - `random_context` must point to a valid u64.
/// - `bytes` must point to valid writable buffer of `bytes_max` bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_test_random_bytes(
    random_context: *mut u64,
    bytes: *mut u8,
    bytes_max: usize,
) {
    let slice = std::slice::from_raw_parts_mut(bytes, bytes_max);
    test_random_bytes(&mut *random_context, slice);
}

/// FFI export: Uniform random in [0, rnd_max).
///
/// # Safety
/// - `random_context` must point to a valid u64.
#[no_mangle]
pub unsafe extern "C" fn picoquic_test_uniform_random(
    random_context: *mut u64,
    rnd_max: u64,
) -> u64 {
    test_uniform_random(&mut *random_context, rnd_max)
}

/// FFI export: Gaussian random.
///
/// # Safety
/// - `random_context` must point to a valid u64.
#[no_mangle]
pub unsafe extern "C" fn picoquic_test_gauss_random(random_context: *mut u64) -> f64 {
    test_gauss_random(&mut *random_context)
}

/// FFI export: Poisson random.
///
/// # Safety
/// - `random_context` must point to a valid u64.
#[no_mangle]
pub unsafe extern "C" fn picoquic_test_poisson_random(
    random_context: *mut u64,
    exp_minus_lambda_2_30: u64,
) -> u64 {
    test_poisson_random(&mut *random_context, exp_minus_lambda_2_30)
}

/// FFI export: Predict varint encode length.
#[no_mangle]
pub extern "C" fn picoquic_frames_varint_encode_length(n64: u64) -> usize {
    frames_varint_encode_length(n64)
}

/// Helper: Safely create a slice from C pointers, returning None if invalid.
/// Returns None if bytes is null or bytes_max <= bytes.
#[inline]
unsafe fn slice_from_c_ptrs(bytes: *const u8, bytes_max: *const u8) -> Option<&'static [u8]> {
    if bytes.is_null() || bytes_max <= bytes {
        return None;
    }
    let len = bytes_max.offset_from(bytes) as usize;
    Some(std::slice::from_raw_parts(bytes, len))
}

/// Helper: Safely create a mutable slice from C pointers, returning None if invalid.
#[inline]
unsafe fn slice_from_c_ptrs_mut(bytes: *mut u8, bytes_max: *const u8) -> Option<&'static mut [u8]> {
    if bytes.is_null() || bytes_max <= bytes as *const u8 {
        return None;
    }
    let len = bytes_max.offset_from(bytes) as usize;
    Some(std::slice::from_raw_parts_mut(bytes, len))
}

/// FFI export: Skip fixed bytes.
///
/// # Safety
/// - `bytes` must point to valid data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_fixed_skip(
    bytes: *const u8,
    bytes_max: *const u8,
    size: u64,
) -> *const u8 {
    let Some(slice) = slice_from_c_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    match frames_fixed_skip(slice, size as usize) {
        Some(rest) => rest.as_ptr(),
        None => std::ptr::null(),
    }
}

/// FFI export: Skip a varint.
///
/// # Safety
/// - `bytes` must point to valid data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_varint_skip(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_c_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    match frames_varint_skip(slice) {
        Some(rest) => rest.as_ptr(),
        None => std::ptr::null(),
    }
}

/// FFI export: Decode a varint.
///
/// # Safety
/// - `bytes` must point to valid data.
/// - `bytes_max` must be >= bytes.
/// - `n64` must point to a valid writable u64.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_varint_decode(
    bytes: *const u8,
    bytes_max: *const u8,
    n64: *mut u64,
) -> *const u8 {
    let Some(slice) = slice_from_c_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    match frames_varint_decode(slice) {
        Some((value, rest)) => {
            *n64 = value;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Decode a varlen (varint as size_t).
///
/// # Safety
/// - `bytes` must point to valid data.
/// - `bytes_max` must be >= bytes.
/// - `n` must point to a valid writable size_t.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_varlen_decode(
    bytes: *const u8,
    bytes_max: *const u8,
    n: *mut usize,
) -> *const u8 {
    let Some(slice) = slice_from_c_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    match frames_varlen_decode(slice) {
        Some((value, rest)) => {
            *n = value;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Decode a u8.
///
/// # Safety
/// - `bytes` must point to valid data.
/// - `bytes_max` must be >= bytes.
/// - `n` must point to a valid writable u8.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_uint8_decode(
    bytes: *const u8,
    bytes_max: *const u8,
    n: *mut u8,
) -> *const u8 {
    let Some(slice) = slice_from_c_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    match frames_uint8_decode(slice) {
        Some((value, rest)) => {
            *n = value;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Decode a u16.
///
/// # Safety
/// - `bytes` must point to valid data.
/// - `bytes_max` must be >= bytes.
/// - `n` must point to a valid writable u16.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_uint16_decode(
    bytes: *const u8,
    bytes_max: *const u8,
    n: *mut u16,
) -> *const u8 {
    let Some(slice) = slice_from_c_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    match frames_uint16_decode(slice) {
        Some((value, rest)) => {
            *n = value;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Decode a u32.
///
/// # Safety
/// - `bytes` must point to valid data.
/// - `bytes_max` must be >= bytes.
/// - `n` must point to a valid writable u32.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_uint32_decode(
    bytes: *const u8,
    bytes_max: *const u8,
    n: *mut u32,
) -> *const u8 {
    let Some(slice) = slice_from_c_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    match frames_uint32_decode(slice) {
        Some((value, rest)) => {
            *n = value;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Decode a u64.
///
/// # Safety
/// - `bytes` must point to valid data.
/// - `bytes_max` must be >= bytes.
/// - `n` must point to a valid writable u64.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_uint64_decode(
    bytes: *const u8,
    bytes_max: *const u8,
    n: *mut u64,
) -> *const u8 {
    let Some(slice) = slice_from_c_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    match frames_uint64_decode(slice) {
        Some((value, rest)) => {
            *n = value;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Skip length-prefixed data.
///
/// # Safety
/// - `bytes` must point to valid data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_length_data_skip(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_c_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    match frames_length_data_skip(slice) {
        Some(rest) => rest.as_ptr(),
        None => std::ptr::null(),
    }
}

/// FFI export: Encode a varint.
///
/// # Safety
/// - `bytes` must point to valid writable data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_varint_encode(
    bytes: *mut u8,
    bytes_max: *const u8,
    n64: u64,
) -> *mut u8 {
    let Some(slice) = slice_from_c_ptrs_mut(bytes, bytes_max) else {
        return std::ptr::null_mut();
    };
    match frames_varint_encode(slice, n64) {
        Some(rest) => rest.as_mut_ptr(),
        None => std::ptr::null_mut(),
    }
}

/// FFI export: Encode a varlen.
///
/// # Safety
/// - `bytes` must point to valid writable data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_varlen_encode(
    bytes: *mut u8,
    bytes_max: *const u8,
    n: usize,
) -> *mut u8 {
    picoquic_frames_varint_encode(bytes, bytes_max, n as u64)
}

/// FFI export: Encode a u8.
///
/// # Safety
/// - `bytes` must point to valid writable data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_uint8_encode(
    bytes: *mut u8,
    bytes_max: *const u8,
    n: u8,
) -> *mut u8 {
    let Some(slice) = slice_from_c_ptrs_mut(bytes, bytes_max) else {
        return std::ptr::null_mut();
    };
    match frames_uint8_encode(slice, n) {
        Some(rest) => rest.as_mut_ptr(),
        None => std::ptr::null_mut(),
    }
}

/// FFI export: Encode a u16.
///
/// # Safety
/// - `bytes` must point to valid writable data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_uint16_encode(
    bytes: *mut u8,
    bytes_max: *const u8,
    n: u16,
) -> *mut u8 {
    let Some(slice) = slice_from_c_ptrs_mut(bytes, bytes_max) else {
        return std::ptr::null_mut();
    };
    match frames_uint16_encode(slice, n) {
        Some(rest) => rest.as_mut_ptr(),
        None => std::ptr::null_mut(),
    }
}

/// FFI export: Encode a u24.
///
/// # Safety
/// - `bytes` must point to valid writable data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_uint24_encode(
    bytes: *mut u8,
    bytes_max: *const u8,
    n: u32,
) -> *mut u8 {
    let Some(slice) = slice_from_c_ptrs_mut(bytes, bytes_max) else {
        return std::ptr::null_mut();
    };
    match frames_uint24_encode(slice, n) {
        Some(rest) => rest.as_mut_ptr(),
        None => std::ptr::null_mut(),
    }
}

/// FFI export: Encode a u32.
///
/// # Safety
/// - `bytes` must point to valid writable data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_uint32_encode(
    bytes: *mut u8,
    bytes_max: *const u8,
    n: u32,
) -> *mut u8 {
    let Some(slice) = slice_from_c_ptrs_mut(bytes, bytes_max) else {
        return std::ptr::null_mut();
    };
    match frames_uint32_encode(slice, n) {
        Some(rest) => rest.as_mut_ptr(),
        None => std::ptr::null_mut(),
    }
}

/// FFI export: Encode a u64.
///
/// # Safety
/// - `bytes` must point to valid writable data.
/// - `bytes_max` must be >= bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_uint64_encode(
    bytes: *mut u8,
    bytes_max: *const u8,
    n: u64,
) -> *mut u8 {
    let Some(slice) = slice_from_c_ptrs_mut(bytes, bytes_max) else {
        return std::ptr::null_mut();
    };
    match frames_uint64_encode(slice, n) {
        Some(rest) => rest.as_mut_ptr(),
        None => std::ptr::null_mut(),
    }
}

/// FFI export: Encode length-prefixed data.
///
/// # Safety
/// - `bytes` must point to valid writable data.
/// - `bytes_max` must be >= bytes.
/// - `v` must point to valid data of `l` bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_length_data_encode(
    bytes: *mut u8,
    bytes_max: *const u8,
    l: usize,
    v: *const u8,
) -> *mut u8 {
    let Some(slice) = slice_from_c_ptrs_mut(bytes, bytes_max) else {
        return std::ptr::null_mut();
    };
    let data = if l == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(v, l)
    };
    match frames_length_data_encode(slice, data) {
        Some(rest) => rest.as_mut_ptr(),
        None => std::ptr::null_mut(),
    }
}

/// FFI export: Encode null-terminated string with length prefix.
///
/// # Safety
/// - `bytes` must point to valid writable data.
/// - `bytes_max` must be >= bytes.
/// - `s` must point to a valid null-terminated string (or be null).
#[no_mangle]
pub unsafe extern "C" fn picoquic_frames_charz_encode(
    bytes: *mut u8,
    bytes_max: *const u8,
    s: *const std::ffi::c_char,
) -> *mut u8 {
    if s.is_null() {
        return picoquic_frames_varlen_encode(bytes, bytes_max, 0);
    }
    let Some(slice) = slice_from_c_ptrs_mut(bytes, bytes_max) else {
        return std::ptr::null_mut();
    };
    let len = libc::strlen(s);
    let data = std::slice::from_raw_parts(s as *const u8, len);
    match frames_length_data_encode(slice, data) {
        Some(rest) => rest.as_mut_ptr(),
        None => std::ptr::null_mut(),
    }
}

/// FFI export: Convert binary to printable string.
///
/// # Safety
/// - `text` must point to valid writable buffer of `text_len` bytes.
/// - `data` must point to valid data of `data_len` bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_uint8_to_str(
    text: *mut std::ffi::c_char,
    text_len: usize,
    data: *const u8,
    data_len: usize,
) -> *mut std::ffi::c_char {
    let text_slice = std::slice::from_raw_parts_mut(text as *mut u8, text_len);
    let data_slice = std::slice::from_raw_parts(data, data_len);
    uint8_to_str(data_slice, text_slice);
    text
}

// =============================================================================
// Connection ID FFI exports
// =============================================================================

/// FFI export: Format a connection ID into a byte buffer.
///
/// # Safety
/// - `bytes` must point to valid writable buffer of `bytes_max` bytes.
/// - `cnx_id` is passed by value (C struct).
#[no_mangle]
pub unsafe extern "C" fn picoquic_format_connection_id(
    bytes: *mut u8,
    bytes_max: usize,
    cnx_id: ConnectionId,
) -> u8 {
    let slice = std::slice::from_raw_parts_mut(bytes, bytes_max);
    cnx_id.format(slice)
}

/// FFI export: Parse a connection ID from bytes.
///
/// # Safety
/// - `bytes` must point to valid data of at least `len` bytes.
/// - `cnx_id` must point to a valid ConnectionId struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_connection_id(
    bytes: *const u8,
    len: u8,
    cnx_id: *mut ConnectionId,
) -> u8 {
    let slice = std::slice::from_raw_parts(bytes, len as usize);
    (*cnx_id).parse(slice, len)
}

/// FFI export: Check if a connection ID is null (zero-length).
///
/// # Safety
/// - `cnx_id` must point to a valid ConnectionId struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_is_connection_id_null(cnx_id: *const ConnectionId) -> c_int {
    if (*cnx_id).is_null() {
        1
    } else {
        0
    }
}

/// FFI export: Compare two connection IDs.
///
/// Returns -1 if cnx_id1 < cnx_id2, 0 if equal, 1 if cnx_id1 > cnx_id2.
///
/// # Safety
/// - Both pointers must point to valid ConnectionId structs.
#[no_mangle]
pub unsafe extern "C" fn picoquic_compare_connection_id(
    cnx_id1: *const ConnectionId,
    cnx_id2: *const ConnectionId,
) -> c_int {
    match (*cnx_id1).compare(&*cnx_id2) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

/// FFI export: Compute a hash of a connection ID.
///
/// # Safety
/// - `cid` must point to a valid ConnectionId struct.
/// - `hash_seed` is unused but kept for API compatibility.
#[no_mangle]
pub unsafe extern "C" fn picoquic_connection_id_hash(
    cid: *const ConnectionId,
    _hash_seed: *const u8,
) -> u64 {
    (*cid).hash()
}

/// FFI export: Get 64-bit value representation of connection ID.
///
/// # Safety
/// - `cnx_id` is passed by value (C struct).
#[no_mangle]
pub extern "C" fn picoquic_val64_connection_id(cnx_id: ConnectionId) -> u64 {
    cnx_id.val64()
}

/// FFI export: Print connection ID as hex string.
///
/// # Safety
/// - `buf` must point to valid writable buffer of `buf_len` bytes.
/// - `cnxid` must point to a valid ConnectionId struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_print_connection_id_hexa(
    buf: *mut std::ffi::c_char,
    buf_len: usize,
    cnxid: *const ConnectionId,
) -> c_int {
    let slice = std::slice::from_raw_parts_mut(buf as *mut u8, buf_len);
    match (*cnxid).format_hexa(slice) {
        Some(_) => 0,
        None => -1,
    }
}

/// FFI export: Parse connection ID from hex string.
///
/// # Safety
/// - `hex_input` must point to valid data of `input_length` bytes.
/// - `cnx_id` must point to a valid ConnectionId struct.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_connection_id_hexa(
    hex_input: *const std::ffi::c_char,
    input_length: usize,
    cnx_id: *mut ConnectionId,
) -> u8 {
    let slice = std::slice::from_raw_parts(hex_input as *const u8, input_length);
    (*cnx_id).parse_hexa(slice)
}

/// Null connection ID constant (for FFI compatibility).
#[no_mangle]
pub static picoquic_null_connection_id: ConnectionId = ConnectionId::null();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hexa_digit() {
        assert_eq!(parse_hexa_digit(b'0'), Some(0));
        assert_eq!(parse_hexa_digit(b'9'), Some(9));
        assert_eq!(parse_hexa_digit(b'a'), Some(10));
        assert_eq!(parse_hexa_digit(b'f'), Some(15));
        assert_eq!(parse_hexa_digit(b'A'), Some(10));
        assert_eq!(parse_hexa_digit(b'F'), Some(15));
        assert_eq!(parse_hexa_digit(b'g'), None);
        assert_eq!(parse_hexa_digit(b'G'), None);
    }

    #[test]
    fn test_parse_hexa() {
        let mut output = [0u8; 4];
        assert_eq!(parse_hexa(b"deadbeef", &mut output), 4);
        assert_eq!(output, [0xde, 0xad, 0xbe, 0xef]);

        assert_eq!(parse_hexa(b"00ff", &mut output), 2);
        assert_eq!(&output[..2], &[0x00, 0xff]);

        // Odd length should fail
        assert_eq!(parse_hexa(b"abc", &mut output), 0);

        // Invalid char should fail
        assert_eq!(parse_hexa(b"ghij", &mut output), 0);
    }

    #[test]
    fn test_format_hexa() {
        let mut output = [0u8; 9];
        assert_eq!(format_hexa(&[0xde, 0xad, 0xbe, 0xef], &mut output), 8);
        assert_eq!(&output[..8], b"deadbeef");
    }

    #[test]
    fn test_constant_time_memcmp() {
        assert_eq!(constant_time_memcmp(b"hello", b"hello"), 0);
        assert_eq!(constant_time_memcmp(b"hello", b"world"), -1);
        assert_eq!(constant_time_memcmp(b"abc", b"abcd"), -1); // different lengths
    }

    #[test]
    fn test_random_deterministic() {
        let mut ctx1 = 12345u64;
        let mut ctx2 = 12345u64;

        // Same seed should produce same sequence
        for _ in 0..10 {
            assert_eq!(test_random(&mut ctx1), test_random(&mut ctx2));
        }
    }

    #[test]
    fn test_frames_varint_roundtrip() {
        let mut buf = [0u8; 8];

        for &val in &[0u64, 63, 64, 16383, 16384, 1073741823, 1073741824] {
            let rest = frames_varint_encode(&mut buf, val).unwrap();
            let encoded_len = 8 - rest.len();

            let (decoded, _) = frames_varint_decode(&buf[..encoded_len]).unwrap();
            assert_eq!(decoded, val);
        }
    }

    #[test]
    fn test_frames_uint_roundtrip() {
        let mut buf = [0u8; 8];

        // u8
        let rest = frames_uint8_encode(&mut buf, 0x42).unwrap();
        assert_eq!(rest.len(), 7);
        let (val, _) = frames_uint8_decode(&buf).unwrap();
        assert_eq!(val, 0x42);

        // u16
        let rest = frames_uint16_encode(&mut buf, 0x1234).unwrap();
        assert_eq!(rest.len(), 6);
        let (val, _) = frames_uint16_decode(&buf).unwrap();
        assert_eq!(val, 0x1234);

        // u32
        let rest = frames_uint32_encode(&mut buf, 0x12345678).unwrap();
        assert_eq!(rest.len(), 4);
        let (val, _) = frames_uint32_decode(&buf).unwrap();
        assert_eq!(val, 0x12345678);

        // u64
        let rest = frames_uint64_encode(&mut buf, 0x123456789abcdef0).unwrap();
        assert_eq!(rest.len(), 0);
        let (val, _) = frames_uint64_decode(&buf).unwrap();
        assert_eq!(val, 0x123456789abcdef0);
    }

    // =========================================================================
    // Connection ID Tests
    // =========================================================================

    #[test]
    fn test_connection_id_null() {
        let cid = ConnectionId::null();
        assert!(cid.is_null());
        assert_eq!(cid.len(), 0);
        assert!(cid.is_empty());
    }

    #[test]
    fn test_connection_id_from_bytes() {
        let cid = ConnectionId::from_bytes(&[0x01, 0x02, 0x03, 0x04]).unwrap();
        assert_eq!(cid.len(), 4);
        assert_eq!(cid.as_bytes(), &[0x01, 0x02, 0x03, 0x04]);
        assert!(!cid.is_null());
    }

    #[test]
    fn test_connection_id_from_bytes_max() {
        let bytes = [0xab; CONNECTION_ID_MAX_SIZE];
        let cid = ConnectionId::from_bytes(&bytes).unwrap();
        assert_eq!(cid.len(), CONNECTION_ID_MAX_SIZE);
    }

    #[test]
    fn test_connection_id_from_bytes_too_long() {
        let bytes = [0xab; CONNECTION_ID_MAX_SIZE + 1];
        assert!(ConnectionId::from_bytes(&bytes).is_none());
    }

    #[test]
    fn test_connection_id_format_parse() {
        let cid = ConnectionId::from_bytes(&[0xde, 0xad, 0xbe, 0xef]).unwrap();

        let mut buf = [0u8; 8];
        let written = cid.format(&mut buf);
        assert_eq!(written, 4);
        assert_eq!(&buf[..4], &[0xde, 0xad, 0xbe, 0xef]);

        let mut cid2 = ConnectionId::null();
        let parsed = cid2.parse(&buf, 4);
        assert_eq!(parsed, 4);
        assert_eq!(cid, cid2);
    }

    #[test]
    fn test_connection_id_compare() {
        let cid1 = ConnectionId::from_bytes(&[0x01, 0x02]).unwrap();
        let cid2 = ConnectionId::from_bytes(&[0x01, 0x02]).unwrap();
        let cid3 = ConnectionId::from_bytes(&[0x01, 0x03]).unwrap();
        let cid4 = ConnectionId::from_bytes(&[0x01]).unwrap();

        assert_eq!(cid1.compare(&cid2), Ordering::Equal);
        assert_eq!(cid1.compare(&cid3), Ordering::Less);
        assert_eq!(cid3.compare(&cid1), Ordering::Greater);
        assert_eq!(cid4.compare(&cid1), Ordering::Less); // shorter length
    }

    #[test]
    fn test_connection_id_hash() {
        let cid1 = ConnectionId::from_bytes(&[0x01, 0x02, 0x03, 0x04]).unwrap();
        let cid2 = ConnectionId::from_bytes(&[0x01, 0x02, 0x03, 0x04]).unwrap();
        let cid3 = ConnectionId::from_bytes(&[0x01, 0x02, 0x03, 0x05]).unwrap();

        // Same IDs should hash the same
        assert_eq!(cid1.hash(), cid2.hash());

        // Different IDs should (almost certainly) hash differently
        assert_ne!(cid1.hash(), cid3.hash());
    }

    #[test]
    fn test_connection_id_val64() {
        // Short ID (< 8 bytes)
        let cid = ConnectionId::from_bytes(&[0x01, 0x02]).unwrap();
        // 0x0102 shifted left by 48 bits
        assert_eq!(cid.val64(), 0x0102_0000_0000_0000);

        // Exact 8 bytes
        let cid =
            ConnectionId::from_bytes(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]).unwrap();
        assert_eq!(cid.val64(), 0x0102_0304_0506_0708);

        // Longer than 8 bytes (only first 8 used)
        let cid =
            ConnectionId::from_bytes(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a])
                .unwrap();
        assert_eq!(cid.val64(), 0x0102_0304_0506_0708);
    }

    #[test]
    fn test_connection_id_format_hexa() {
        let cid = ConnectionId::from_bytes(&[0xde, 0xad, 0xbe, 0xef]).unwrap();
        let mut buf = [0u8; 16];
        let len = cid.format_hexa(&mut buf).unwrap();
        assert_eq!(len, 8);
        assert_eq!(&buf[..8], b"deadbeef");
    }

    #[test]
    fn test_connection_id_parse_hexa() {
        let mut cid = ConnectionId::null();
        let parsed = cid.parse_hexa(b"deadbeef");
        assert_eq!(parsed, 4);
        assert_eq!(cid.as_bytes(), &[0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_connection_id_debug() {
        let cid = ConnectionId::from_bytes(&[0xab, 0xcd]).unwrap();
        let debug_str = format!("{:?}", cid);
        assert_eq!(debug_str, "ConnectionId(abcd)");
    }
}
