//! Integer formatting and QUIC variable-length integer encoding.
//!
//! This module provides:
//! - Big-endian integer serialization/deserialization (16, 24, 32, 64 bit)
//! - QUIC variable-length integer encoding per RFC 9000 Section 16
//!
//! Translated from picoquic/intformat.c

/// Write a 16-bit integer in big-endian format.
#[inline]
pub fn format_16(bytes: &mut [u8], n: u16) {
    bytes[0] = (n >> 8) as u8;
    bytes[1] = n as u8;
}

/// Write a 24-bit integer in big-endian format.
#[inline]
pub fn format_24(bytes: &mut [u8], n: u32) {
    bytes[0] = (n >> 16) as u8;
    bytes[1] = (n >> 8) as u8;
    bytes[2] = n as u8;
}

/// Write a 32-bit integer in big-endian format.
#[inline]
pub fn format_32(bytes: &mut [u8], n: u32) {
    bytes[0] = (n >> 24) as u8;
    bytes[1] = (n >> 16) as u8;
    bytes[2] = (n >> 8) as u8;
    bytes[3] = n as u8;
}

/// Write a 64-bit integer in big-endian format.
#[inline]
pub fn format_64(bytes: &mut [u8], n: u64) {
    bytes[0] = (n >> 56) as u8;
    bytes[1] = (n >> 48) as u8;
    bytes[2] = (n >> 40) as u8;
    bytes[3] = (n >> 32) as u8;
    bytes[4] = (n >> 24) as u8;
    bytes[5] = (n >> 16) as u8;
    bytes[6] = (n >> 8) as u8;
    bytes[7] = n as u8;
}

/// Parse a 16-bit big-endian integer.
#[inline]
pub const fn parse_16(bytes: &[u8]) -> u16 {
    ((bytes[0] as u16) << 8) | (bytes[1] as u16)
}

/// Parse a 24-bit big-endian integer.
#[inline]
pub const fn parse_24(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | (bytes[2] as u32)
}

/// Parse a 32-bit big-endian integer.
#[inline]
pub const fn parse_32(bytes: &[u8]) -> u32 {
    ((bytes[0] as u32) << 24)
        | ((bytes[1] as u32) << 16)
        | ((bytes[2] as u32) << 8)
        | (bytes[3] as u32)
}

/// Parse a 64-bit big-endian integer.
#[inline]
pub const fn parse_64(bytes: &[u8]) -> u64 {
    ((bytes[0] as u64) << 56)
        | ((bytes[1] as u64) << 48)
        | ((bytes[2] as u64) << 40)
        | ((bytes[3] as u64) << 32)
        | ((bytes[4] as u64) << 24)
        | ((bytes[5] as u64) << 16)
        | ((bytes[6] as u64) << 8)
        | (bytes[7] as u64)
}

/// QUIC variable-length integer encoding (RFC 9000 Section 16).
///
/// ```text
/// 2MSB  Length  Usable Bits  Range
/// 00    1       6            0-63
/// 01    2       14           0-16383
/// 10    4       30           0-1073741823
/// 11    8       62           0-4611686018427387903
/// ```
pub mod varint {
    /// Maximum value encodable as a QUIC varint.
    pub const MAX: u64 = 4611686018427387903;

    /// Return the encoded length for a value.
    #[inline]
    pub const fn encode_length(n: u64) -> usize {
        if n < 64 {
            1
        } else if n < 16384 {
            2
        } else if n < 1073741824 {
            4
        } else {
            8
        }
    }

    /// Return the encoded length from the first byte.
    #[inline]
    pub const fn decode_length(byte: u8) -> usize {
        1 << ((byte & 0xC0) >> 6)
    }

    /// Encode a varint into the buffer. Returns bytes written, or 0 if buffer too small.
    #[inline]
    pub fn encode(bytes: &mut [u8], n: u64) -> usize {
        let len = bytes.len();
        if n < 64 {
            if len >= 1 {
                bytes[0] = n as u8;
                return 1;
            }
        } else if n < 16384 {
            if len >= 2 {
                bytes[0] = ((n >> 8) as u8) | 0x40;
                bytes[1] = n as u8;
                return 2;
            }
        } else if n < 1073741824 {
            if len >= 4 {
                bytes[0] = ((n >> 24) as u8) | 0x80;
                bytes[1] = (n >> 16) as u8;
                bytes[2] = (n >> 8) as u8;
                bytes[3] = n as u8;
                return 4;
            }
        } else if len >= 8 {
            bytes[0] = ((n >> 56) as u8) | 0xC0;
            bytes[1] = (n >> 48) as u8;
            bytes[2] = (n >> 40) as u8;
            bytes[3] = (n >> 32) as u8;
            bytes[4] = (n >> 24) as u8;
            bytes[5] = (n >> 16) as u8;
            bytes[6] = (n >> 8) as u8;
            bytes[7] = n as u8;
            return 8;
        }
        0
    }

    /// Encode a value that fits in 2 bytes (0-16383) using the 2-byte format.
    /// Note: Always uses 2 bytes even for values < 64.
    #[inline]
    pub fn encode_16(bytes: &mut [u8], n: u16) {
        bytes[0] = ((n >> 8) as u8 | 0x40) & 0x7F;
        bytes[1] = n as u8;
    }

    /// Decode a varint from the buffer.
    /// Returns (value, bytes_consumed), or (0, 0) on error.
    #[inline]
    pub fn decode(bytes: &[u8]) -> (u64, usize) {
        if bytes.is_empty() {
            return (0, 0);
        }

        let length = decode_length(bytes[0]);
        if length > bytes.len() {
            return (0, 0);
        }

        let mut v = (bytes[0] & 0x3F) as u64;
        for &byte in &bytes[1..length] {
            v = (v << 8) | (byte as u64);
        }

        (v, length)
    }

    /// Return the length of a varint without fully decoding it.
    #[inline]
    pub fn skip(bytes: &[u8]) -> usize {
        decode_length(bytes[0])
    }
}

// =============================================================================
// FFI exports - these replace the C implementations when FQ_USE_RUST is defined
// =============================================================================
//
// # Safety
//
// All FFI functions below require:
// - `bytes` must be a valid pointer to a buffer of at least the required size
// - For write functions: the buffer must be writable
// - For read functions: the buffer must contain valid data
//
// These match the C API contracts from picoquic.

/// FFI export: Write a 16-bit integer in big-endian format.
///
/// # Safety
/// `bytes` must point to a valid, writable buffer of at least 2 bytes.
#[no_mangle]
pub unsafe extern "C" fn picoformat_16(bytes: *mut u8, n16: u16) {
    let slice = std::slice::from_raw_parts_mut(bytes, 2);
    format_16(slice, n16);
}

/// FFI export: Write a 24-bit integer in big-endian format.
///
/// # Safety
/// `bytes` must point to a valid, writable buffer of at least 3 bytes.
#[no_mangle]
pub unsafe extern "C" fn picoformat_24(bytes: *mut u8, n24: u32) {
    let slice = std::slice::from_raw_parts_mut(bytes, 3);
    format_24(slice, n24);
}

/// FFI export: Write a 32-bit integer in big-endian format.
///
/// # Safety
/// `bytes` must point to a valid, writable buffer of at least 4 bytes.
#[no_mangle]
pub unsafe extern "C" fn picoformat_32(bytes: *mut u8, n32: u32) {
    let slice = std::slice::from_raw_parts_mut(bytes, 4);
    format_32(slice, n32);
}

/// FFI export: Write a 64-bit integer in big-endian format.
///
/// # Safety
/// `bytes` must point to a valid, writable buffer of at least 8 bytes.
#[no_mangle]
pub unsafe extern "C" fn picoformat_64(bytes: *mut u8, n64: u64) {
    let slice = std::slice::from_raw_parts_mut(bytes, 8);
    format_64(slice, n64);
}

/// FFI export: Return the encoded length for a varint value.
#[no_mangle]
pub extern "C" fn picoquic_encode_varint_length(n64: u64) -> usize {
    varint::encode_length(n64)
}

/// FFI export: Return the encoded length from the first byte.
#[no_mangle]
pub extern "C" fn picoquic_decode_varint_length(byte: u8) -> usize {
    varint::decode_length(byte)
}

/// FFI export: Encode a varint into the buffer.
///
/// # Safety
/// `bytes` must point to a valid, writable buffer of at least `max_bytes` bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_varint_encode(
    bytes: *mut u8,
    max_bytes: usize,
    n64: u64,
) -> usize {
    let slice = std::slice::from_raw_parts_mut(bytes, max_bytes);
    varint::encode(slice, n64)
}

/// FFI export: Encode a value using 2-byte format.
///
/// # Safety
/// `bytes` must point to a valid, writable buffer of at least 2 bytes.
#[no_mangle]
pub unsafe extern "C" fn picoquic_varint_encode_16(bytes: *mut u8, n16: u16) {
    let slice = std::slice::from_raw_parts_mut(bytes, 2);
    varint::encode_16(slice, n16);
}

/// FFI export: Decode a varint from the buffer.
///
/// # Safety
/// - `bytes` must point to a valid buffer of at least `max_bytes` bytes.
/// - `n64` must point to a valid, writable u64.
#[no_mangle]
pub unsafe extern "C" fn picoquic_varint_decode(
    bytes: *const u8,
    max_bytes: usize,
    n64: *mut u64,
) -> usize {
    let slice = std::slice::from_raw_parts(bytes, max_bytes);
    let (value, len) = varint::decode(slice);
    *n64 = value;
    len
}

/// FFI export: Return the length of a varint without decoding.
///
/// # Safety
/// `bytes` must point to a valid buffer of at least 1 byte.
#[no_mangle]
pub unsafe extern "C" fn picoquic_varint_skip(bytes: *const u8) -> usize {
    varint::decode_length(*bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_parse_16() {
        let mut buf = [0u8; 2];
        format_16(&mut buf, 0x1234);
        assert_eq!(buf, [0x12, 0x34]);
        assert_eq!(parse_16(&buf), 0x1234);
    }

    #[test]
    fn test_format_parse_24() {
        let mut buf = [0u8; 3];
        format_24(&mut buf, 0x123456);
        assert_eq!(buf, [0x12, 0x34, 0x56]);
        assert_eq!(parse_24(&buf), 0x123456);
    }

    #[test]
    fn test_format_parse_32() {
        let mut buf = [0u8; 4];
        format_32(&mut buf, 0x12345678);
        assert_eq!(buf, [0x12, 0x34, 0x56, 0x78]);
        assert_eq!(parse_32(&buf), 0x12345678);
    }

    #[test]
    fn test_format_parse_64() {
        let mut buf = [0u8; 8];
        format_64(&mut buf, 0x123456789ABCDEF0);
        assert_eq!(buf, [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
        assert_eq!(parse_64(&buf), 0x123456789ABCDEF0);
    }

    #[test]
    fn test_varint_1_byte() {
        // Values 0-63 encode to 1 byte
        let mut buf = [0u8; 8];

        assert_eq!(varint::encode_length(0), 1);
        assert_eq!(varint::encode(&mut buf, 0), 1);
        assert_eq!(buf[0], 0x00);
        assert_eq!(varint::decode(&buf), (0, 1));

        assert_eq!(varint::encode_length(37), 1);
        assert_eq!(varint::encode(&mut buf, 37), 1);
        assert_eq!(buf[0], 0x25);
        assert_eq!(varint::decode(&buf), (37, 1));

        assert_eq!(varint::encode_length(63), 1);
        assert_eq!(varint::encode(&mut buf, 63), 1);
        assert_eq!(buf[0], 0x3F);
        assert_eq!(varint::decode(&buf), (63, 1));
    }

    #[test]
    fn test_varint_2_byte() {
        // Values 64-16383 encode to 2 bytes
        let mut buf = [0u8; 8];

        assert_eq!(varint::encode_length(64), 2);
        assert_eq!(varint::encode(&mut buf, 64), 2);
        assert_eq!(&buf[..2], &[0x40, 0x40]);
        assert_eq!(varint::decode(&buf), (64, 2));

        assert_eq!(varint::encode_length(15293), 2);
        assert_eq!(varint::encode(&mut buf, 15293), 2);
        assert_eq!(&buf[..2], &[0x7B, 0xBD]); // Example from RFC 9000
        assert_eq!(varint::decode(&buf), (15293, 2));

        assert_eq!(varint::encode_length(16383), 2);
        assert_eq!(varint::encode(&mut buf, 16383), 2);
        assert_eq!(&buf[..2], &[0x7F, 0xFF]);
        assert_eq!(varint::decode(&buf), (16383, 2));
    }

    #[test]
    fn test_varint_4_byte() {
        // Values 16384-1073741823 encode to 4 bytes
        let mut buf = [0u8; 8];

        assert_eq!(varint::encode_length(16384), 4);
        assert_eq!(varint::encode(&mut buf, 16384), 4);
        assert_eq!(&buf[..4], &[0x80, 0x00, 0x40, 0x00]);
        assert_eq!(varint::decode(&buf), (16384, 4));

        assert_eq!(varint::encode_length(494878333), 4);
        assert_eq!(varint::encode(&mut buf, 494878333), 4);
        assert_eq!(&buf[..4], &[0x9D, 0x7F, 0x3E, 0x7D]); // Example from RFC 9000
        assert_eq!(varint::decode(&buf), (494878333, 4));
    }

    #[test]
    fn test_varint_8_byte() {
        // Values >= 1073741824 encode to 8 bytes
        let mut buf = [0u8; 8];

        assert_eq!(varint::encode_length(1073741824), 8);
        assert_eq!(varint::encode(&mut buf, 1073741824), 8);
        assert_eq!(varint::decode(&buf), (1073741824, 8));

        assert_eq!(varint::encode_length(151288809941952652), 8);
        assert_eq!(varint::encode(&mut buf, 151288809941952652), 8);
        assert_eq!(buf, [0xC2, 0x19, 0x7C, 0x5E, 0xFF, 0x14, 0xE8, 0x8C]); // Example from RFC 9000
        assert_eq!(varint::decode(&buf), (151288809941952652, 8));
    }

    #[test]
    fn test_varint_decode_length() {
        assert_eq!(varint::decode_length(0x00), 1); // 00xxxxxx
        assert_eq!(varint::decode_length(0x3F), 1);
        assert_eq!(varint::decode_length(0x40), 2); // 01xxxxxx
        assert_eq!(varint::decode_length(0x7F), 2);
        assert_eq!(varint::decode_length(0x80), 4); // 10xxxxxx
        assert_eq!(varint::decode_length(0xBF), 4);
        assert_eq!(varint::decode_length(0xC0), 8); // 11xxxxxx
        assert_eq!(varint::decode_length(0xFF), 8);
    }

    #[test]
    fn test_varint_buffer_too_small() {
        let mut buf = [0u8; 1];
        assert_eq!(varint::encode(&mut buf, 64), 0); // needs 2 bytes
        assert_eq!(varint::encode(&mut buf, 16384), 0); // needs 4 bytes
    }

    #[test]
    fn test_varint_decode_truncated() {
        let buf = [0x40]; // 2-byte encoding but only 1 byte available
        assert_eq!(varint::decode(&buf), (0, 0));

        let buf = [0x80, 0x00]; // 4-byte encoding but only 2 bytes
        assert_eq!(varint::decode(&buf), (0, 0));
    }

    #[test]
    fn test_varint_encode_16() {
        let mut buf = [0u8; 2];
        varint::encode_16(&mut buf, 100);
        // Forces 2-byte encoding even for small values
        assert_eq!(varint::decode_length(buf[0]), 2);
        assert_eq!(varint::decode(&buf), (100, 2));
    }
}
