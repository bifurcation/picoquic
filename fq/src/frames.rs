//! Frame skip and parse functions.
//!
//! This module provides functions for skipping over and parsing QUIC frames
//! without requiring the full connection context. These are used for:
//! - Fast packet parsing
//! - Frame validation
//! - Loss detection (checking if frames need retransmission)
//!
//! Translated from picoquic/frames.c

use crate::util::{
    frames_fixed_skip, frames_length_data_skip, frames_uint16_decode, frames_uint8_decode,
    frames_varint_decode, frames_varint_skip,
};

// =============================================================================
// Constants
// =============================================================================

/// Size of the stateless reset secret (16 bytes).
pub const RESET_SECRET_SIZE: usize = 16;

// =============================================================================
// Frame Skip Functions
// =============================================================================

/// Skip a NEW_CONNECTION_ID frame.
///
/// Frame format:
/// - Frame type (varint)
/// - Sequence Number (varint)
/// - [Path ID (varint)] - if is_mp
/// - Retire Prior To (varint)
/// - Connection ID Length (u8)
/// - Connection ID (variable)
/// - Stateless Reset Token (16 bytes)
pub fn skip_new_connection_id_frame(bytes: &[u8], is_mp: bool) -> Option<&[u8]> {
    let mut rest = bytes;

    // Skip frame type
    rest = frames_varint_skip(rest)?;

    // Skip path ID if multipath
    if is_mp {
        rest = frames_varint_skip(rest)?;
    }

    // Skip sequence number
    rest = frames_varint_skip(rest)?;

    // Skip retire prior to
    rest = frames_varint_skip(rest)?;

    // Get CID length and skip CID + reset token
    let (cid_length, rest2) = frames_uint8_decode(rest)?;
    frames_fixed_skip(rest2, cid_length as usize + RESET_SECRET_SIZE)
}

/// Skip a RETIRE_CONNECTION_ID frame.
///
/// Frame format (non-mp):
/// - Frame type (1 byte, 0x19)
/// - Sequence Number (varint)
///
/// Frame format (multipath):
/// - Frame type (varint)
/// - Path ID (varint)
/// - Sequence Number (varint)
pub fn skip_retire_connection_id_frame(bytes: &[u8], is_mp: bool) -> Option<&[u8]> {
    if is_mp {
        // Skip frame type, path ID, and sequence (3 varints)
        let rest = frames_varint_skip(bytes)?;
        let rest = frames_varint_skip(rest)?;
        frames_varint_skip(rest)
    } else {
        // Skip 1-byte frame type, then sequence varint
        if bytes.is_empty() {
            return None;
        }
        frames_varint_skip(&bytes[1..])
    }
}

/// Skip a NEW_TOKEN frame.
///
/// Frame format:
/// - Frame type (1 byte, 0x07)
/// - Token Length (varint)
/// - Token (variable)
pub fn skip_new_token_frame(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.is_empty() {
        return None;
    }
    // Skip 1-byte frame type, then length-prefixed data
    frames_length_data_skip(&bytes[1..])
}

/// Skip a STOP_SENDING frame.
///
/// Frame format:
/// - Frame type (1 byte, 0x05)
/// - Stream ID (varint)
/// - Application Error Code (varint)
pub fn skip_stop_sending_frame(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.is_empty() {
        return None;
    }
    // Skip 1-byte frame type, then stream ID and error code varints
    let rest = frames_varint_skip(&bytes[1..])?;
    frames_varint_skip(rest)
}

/// Skip a DATAGRAM frame.
///
/// Frame format (type 0x30 or 0x31):
/// - Frame type (1 byte) - must be included in bytes
/// - [Length (varint)] - if frame_id & 1
/// - Data (variable)
pub fn skip_datagram_frame(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.is_empty() {
        return None;
    }

    let frame_id = bytes[0];
    let has_length = (frame_id & 1) != 0;
    let rest = &bytes[1..];

    if has_length {
        let (length, rest2) = frames_varint_decode(rest)?;
        frames_fixed_skip(rest2, length as usize)
    } else {
        // No length field means data extends to end of packet
        Some(&rest[rest.len()..])
    }
}

/// Skip an ACK_FREQUENCY frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - Sequence Number (varint)
/// - Ack-Eliciting Threshold (varint)
/// - Request Max Ack Delay (varint)
/// - Reordering Threshold (varint)
pub fn skip_ack_frequency_frame(bytes: &[u8]) -> Option<&[u8]> {
    let rest = frames_varint_skip(bytes)?;
    let rest = frames_varint_skip(rest)?;
    let rest = frames_varint_skip(rest)?;
    frames_varint_skip(rest)
}

/// Skip an IMMEDIATE_ACK frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - (no additional fields)
pub fn skip_immediate_ack_frame(bytes: &[u8]) -> Option<&[u8]> {
    // No additional data after frame type
    Some(bytes)
}

/// Skip a TIME_STAMP frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - Timestamp (varint)
pub fn skip_time_stamp_frame(bytes: &[u8]) -> Option<&[u8]> {
    frames_varint_skip(bytes)
}

/// Skip a PATH_ABANDON frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - Path ID (varint)
/// - Error Code (varint)
pub fn skip_path_abandon_frame(bytes: &[u8]) -> Option<&[u8]> {
    let rest = frames_varint_skip(bytes)?;
    frames_varint_skip(rest)
}

/// Skip a PATH_AVAILABLE or PATH_BACKUP frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - Path ID (varint)
/// - Sequence (varint)
pub fn skip_path_available_or_backup_frame(bytes: &[u8]) -> Option<&[u8]> {
    let rest = frames_varint_skip(bytes)?;
    frames_varint_skip(rest)
}

/// Skip a MAX_PATH_ID frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - Maximum Path ID (varint)
pub fn skip_max_path_id_frame(bytes: &[u8]) -> Option<&[u8]> {
    frames_varint_skip(bytes)
}

/// Skip a PATHS_BLOCKED frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - Maximum Path ID (varint)
pub fn skip_paths_blocked_frame(bytes: &[u8]) -> Option<&[u8]> {
    frames_varint_skip(bytes)
}

/// Skip a PATH_CID_BLOCKED frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - Path ID (varint)
/// - Next Sequence Number (varint)
pub fn skip_path_cid_blocked_frame(bytes: &[u8]) -> Option<&[u8]> {
    let rest = frames_varint_skip(bytes)?;
    frames_varint_skip(rest)
}

/// Skip an OBSERVED_ADDRESS frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - Sequence Number (varint)
/// - IP Address (4 bytes for IPv4, 16 bytes for IPv6)
/// - Port (2 bytes)
///
/// The `ftype` parameter is the frame type, used to determine IPv4 vs IPv6:
/// - Even type = IPv4 (4 bytes)
/// - Odd type = IPv6 (16 bytes)
pub fn skip_observed_address_frame(bytes: &[u8], ftype: u64) -> Option<&[u8]> {
    let rest = frames_varint_skip(bytes)?;

    let addr_len = if (ftype & 1) == 0 { 4 } else { 16 };
    let frame_len = addr_len + 2; // addr + port

    frames_fixed_skip(rest, frame_len)
}

/// Skip a BDP frame.
///
/// Frame format:
/// - Frame type (varint) - already consumed
/// - Lifetime (varint)
/// - Bytes in Flight (varint)
/// - Min RTT (varint)
/// - IP Address (length-prefixed)
pub fn skip_bdp_frame(bytes: &[u8]) -> Option<&[u8]> {
    let rest = frames_varint_skip(bytes)?; // lifetime
    let rest = frames_varint_skip(rest)?; // bytes in flight
    let rest = frames_varint_skip(rest)?; // min rtt
    frames_length_data_skip(rest) // IP address
}

// =============================================================================
// Frame Parse Functions (standalone, no context required)
// =============================================================================

/// Parse a RETIRE_CONNECTION_ID frame.
///
/// Returns (path_id, sequence, remaining_bytes).
/// path_id is 0 if not multipath.
pub fn parse_retire_connection_id_frame(bytes: &[u8], is_mp: bool) -> Option<(u64, u64, &[u8])> {
    let mut path_id = 0u64;
    let rest;

    if is_mp {
        let (pid, r) = frames_varint_decode(bytes)?;
        path_id = pid;
        rest = r;
    } else {
        rest = bytes;
    }

    let (sequence, rest) = frames_varint_decode(rest)?;
    Some((path_id, sequence, rest))
}

/// Parse an ACK_FREQUENCY frame.
///
/// Returns (seq, packets, microsec, reordering_threshold, remaining_bytes).
pub fn parse_ack_frequency_frame(bytes: &[u8]) -> Option<(u64, u64, u64, u64, &[u8])> {
    let (seq, rest) = frames_varint_decode(bytes)?;
    let (packets, rest) = frames_varint_decode(rest)?;
    let (microsec, rest) = frames_varint_decode(rest)?;
    let (reordering_threshold, rest) = frames_varint_decode(rest)?;
    Some((seq, packets, microsec, reordering_threshold, rest))
}

/// Parse a TIME_STAMP frame.
///
/// Returns (timestamp, remaining_bytes).
pub fn parse_time_stamp_frame(bytes: &[u8]) -> Option<(u64, &[u8])> {
    frames_varint_decode(bytes)
}

/// Parse a PATH_ABANDON frame.
///
/// Returns (path_id, reason, remaining_bytes).
pub fn parse_path_abandon_frame(bytes: &[u8]) -> Option<(u64, u64, &[u8])> {
    let (path_id, rest) = frames_varint_decode(bytes)?;
    let (reason, rest) = frames_varint_decode(rest)?;
    Some((path_id, reason, rest))
}

/// Parse a PATH_AVAILABLE or PATH_BACKUP frame.
///
/// Returns (path_id, sequence, remaining_bytes).
pub fn parse_path_available_or_backup_frame(bytes: &[u8]) -> Option<(u64, u64, &[u8])> {
    let (path_id, rest) = frames_varint_decode(bytes)?;
    let (sequence, rest) = frames_varint_decode(rest)?;
    Some((path_id, sequence, rest))
}

/// Parse a MAX_PATH_ID frame.
///
/// Returns (max_path_id, remaining_bytes).
pub fn parse_max_path_id_frame(bytes: &[u8]) -> Option<(u64, &[u8])> {
    frames_varint_decode(bytes)
}

/// Parse a PATHS_BLOCKED frame.
///
/// Returns (max_path_id, remaining_bytes).
pub fn parse_paths_blocked_frame(bytes: &[u8]) -> Option<(u64, &[u8])> {
    frames_varint_decode(bytes)
}

/// Parse a PATH_CID_BLOCKED frame.
///
/// Returns (path_id, next_sequence_number, remaining_bytes).
pub fn parse_path_cid_blocked_frame(bytes: &[u8]) -> Option<(u64, u64, &[u8])> {
    let (path_id, rest) = frames_varint_decode(bytes)?;
    let (next_seq, rest) = frames_varint_decode(rest)?;
    Some((path_id, next_seq, rest))
}

/// Parse an OBSERVED_ADDRESS frame.
///
/// Returns (sequence, addr_slice, port, remaining_bytes).
/// The addr_slice points into the original bytes.
pub fn parse_observed_address_frame(bytes: &[u8], ftype: u64) -> Option<(u64, &[u8], u16, &[u8])> {
    let (sequence, rest) = frames_varint_decode(bytes)?;

    let addr_len = if (ftype & 1) == 0 { 4 } else { 16 };
    if rest.len() < addr_len + 2 {
        return None;
    }

    let addr = &rest[..addr_len];
    let rest = &rest[addr_len..];

    let (port, rest) = frames_uint16_decode(rest)?;
    Some((sequence, addr, port, rest))
}

// =============================================================================
// FFI Exports
// =============================================================================

use std::ffi::c_int;

/// Helper to convert Option<&[u8]> to C pointer.
#[inline]
fn result_to_ptr(result: Option<&[u8]>) -> *const u8 {
    match result {
        Some(slice) => slice.as_ptr(),
        None => std::ptr::null(),
    }
}

/// Helper to create slice from C pointers.
#[inline]
unsafe fn slice_from_ptrs(bytes: *const u8, bytes_max: *const u8) -> Option<&'static [u8]> {
    if bytes.is_null() || bytes_max <= bytes {
        return None;
    }
    let len = bytes_max.offset_from(bytes) as usize;
    Some(std::slice::from_raw_parts(bytes, len))
}

/// FFI export: Skip NEW_CONNECTION_ID frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_new_connection_id_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    is_mp: c_int,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_new_connection_id_frame(slice, is_mp != 0))
}

/// FFI export: Skip RETIRE_CONNECTION_ID frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_retire_connection_id_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    is_mp: c_int,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_retire_connection_id_frame(slice, is_mp != 0))
}

/// FFI export: Skip NEW_TOKEN frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_new_token_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_new_token_frame(slice))
}

/// FFI export: Skip STOP_SENDING frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_stop_sending_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_stop_sending_frame(slice))
}

/// FFI export: Skip DATAGRAM frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_datagram_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_datagram_frame(slice))
}

/// FFI export: Skip ACK_FREQUENCY frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_ack_frequency_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_ack_frequency_frame(slice))
}

/// FFI export: Skip IMMEDIATE_ACK frame.
///
/// # Safety
/// `bytes` must be a valid pointer (or null, which returns null).
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_immediate_ack_frame(
    bytes: *const u8,
    _bytes_max: *const u8,
) -> *const u8 {
    // No additional data after frame type
    bytes
}

/// FFI export: Skip TIME_STAMP frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_time_stamp_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_time_stamp_frame(slice))
}

/// FFI export: Skip PATH_ABANDON frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_path_abandon_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_path_abandon_frame(slice))
}

/// FFI export: Skip PATH_AVAILABLE or PATH_BACKUP frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_path_available_or_backup_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_path_available_or_backup_frame(slice))
}

/// FFI export: Skip MAX_PATH_ID frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_max_path_id_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_max_path_id_frame(slice))
}

/// FFI export: Skip PATHS_BLOCKED frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_paths_blocked_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_paths_blocked_frame(slice))
}

/// FFI export: Skip PATH_CID_BLOCKED frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_path_cid_blocked_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_path_cid_blocked_frame(slice))
}

/// FFI export: Skip OBSERVED_ADDRESS frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_observed_address_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    ftype: u64,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_observed_address_frame(slice, ftype))
}

/// FFI export: Skip BDP frame.
///
/// # Safety
/// `bytes` and `bytes_max` must form a valid memory range.
#[no_mangle]
pub unsafe extern "C" fn picoquic_skip_bdp_frame(
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };
    result_to_ptr(skip_bdp_frame(slice))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_new_token_frame() {
        // Frame type (0x07) + Length (varint 5) + 5 bytes of data
        let data = [0x07, 0x05, 0x01, 0x02, 0x03, 0x04, 0x05, 0xAB];
        let rest = skip_new_token_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], 0xAB);
    }

    #[test]
    fn test_skip_stop_sending_frame() {
        // Frame type (0x05) + Stream ID (varint 10) + Error code (varint 20)
        let data = [0x05, 0x0a, 0x14, 0xAB];
        let rest = skip_stop_sending_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], 0xAB);
    }

    #[test]
    fn test_skip_datagram_frame_with_length() {
        // Frame type 0x31 (with length), length 3, 3 bytes data
        let data = [0x31, 0x03, 0x01, 0x02, 0x03, 0xAB];
        let rest = skip_datagram_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], 0xAB);
    }

    #[test]
    fn test_skip_datagram_frame_without_length() {
        // Frame type 0x30 (no length), data extends to end
        let data = [0x30, 0x01, 0x02, 0x03];
        let rest = skip_datagram_frame(&data).unwrap();
        assert_eq!(rest.len(), 0);
    }

    #[test]
    fn test_skip_ack_frequency_frame() {
        // 4 varints
        let data = [0x01, 0x02, 0x03, 0x04, 0xAB];
        let rest = skip_ack_frequency_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], 0xAB);
    }

    #[test]
    fn test_skip_immediate_ack_frame() {
        // No data after frame type
        let data = [0xAB, 0xCD];
        let rest = skip_immediate_ack_frame(&data).unwrap();
        assert_eq!(rest.len(), 2);
    }

    #[test]
    fn test_skip_time_stamp_frame() {
        // Single varint
        let data = [0x42, 0x00, 0xAB]; // 2-byte varint (0x200)
        let rest = skip_time_stamp_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], 0xAB);
    }

    #[test]
    fn test_skip_path_abandon_frame() {
        // Path ID + Error code
        let data = [0x05, 0x10, 0xAB];
        let rest = skip_path_abandon_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_skip_path_available_or_backup_frame() {
        // Path ID + Sequence
        let data = [0x05, 0x10, 0xAB];
        let rest = skip_path_available_or_backup_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_skip_max_path_id_frame() {
        // Single varint
        let data = [0x10, 0xAB];
        let rest = skip_max_path_id_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_skip_paths_blocked_frame() {
        // Single varint
        let data = [0x10, 0xAB];
        let rest = skip_paths_blocked_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_skip_path_cid_blocked_frame() {
        // Path ID + Next sequence
        let data = [0x05, 0x10, 0xAB];
        let rest = skip_path_cid_blocked_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_skip_observed_address_frame_ipv4() {
        // Sequence (1 byte) + IPv4 addr (4 bytes) + port (2 bytes)
        let data = [0x01, 192, 168, 1, 1, 0x11, 0x51, 0xAB]; // port 4433
        let rest = skip_observed_address_frame(&data, 0).unwrap(); // even = IPv4
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], 0xAB);
    }

    #[test]
    fn test_skip_observed_address_frame_ipv6() {
        // Sequence (1 byte) + IPv6 addr (16 bytes) + port (2 bytes)
        let mut data = vec![0x01];
        data.extend_from_slice(&[0u8; 16]); // IPv6 address
        data.extend_from_slice(&[0x11, 0x51]); // port
        data.push(0xAB);
        let rest = skip_observed_address_frame(&data, 1).unwrap(); // odd = IPv6
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], 0xAB);
    }

    #[test]
    fn test_skip_bdp_frame() {
        // lifetime + bytes_in_flight + min_rtt + length-prefixed IP
        let data = [0x01, 0x02, 0x03, 0x04, 0x0a, 0x0b, 0x0c, 0x0d, 0xAB];
        let rest = skip_bdp_frame(&data).unwrap();
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_skip_retire_connection_id_frame_non_mp() {
        // Frame type (0x19) + Sequence (varint 5)
        let data = [0x19, 0x05, 0xAB];
        let rest = skip_retire_connection_id_frame(&data, false).unwrap();
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_skip_retire_connection_id_frame_mp() {
        // Frame type (varint) + Path ID (varint) + Sequence (varint)
        let data = [0x40, 0x06, 0x05, 0x10, 0xAB]; // 2-byte varint frame type + path_id + seq
        let rest = skip_retire_connection_id_frame(&data, true).unwrap();
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_parse_path_abandon_frame() {
        let data = [0x05, 0x10, 0xAB];
        let (path_id, reason, rest) = parse_path_abandon_frame(&data).unwrap();
        assert_eq!(path_id, 5);
        assert_eq!(reason, 16);
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_parse_ack_frequency_frame() {
        let data = [0x01, 0x02, 0x03, 0x04, 0xAB];
        let (seq, packets, microsec, reorder, rest) = parse_ack_frequency_frame(&data).unwrap();
        assert_eq!(seq, 1);
        assert_eq!(packets, 2);
        assert_eq!(microsec, 3);
        assert_eq!(reorder, 4);
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_parse_observed_address_frame_ipv4() {
        let data = [0x05, 192, 168, 1, 1, 0x11, 0x51, 0xAB]; // seq=5, addr, port=4433
        let (seq, addr, port, rest) = parse_observed_address_frame(&data, 0).unwrap();
        assert_eq!(seq, 5);
        assert_eq!(addr, &[192, 168, 1, 1]);
        assert_eq!(port, 0x1151);
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn test_skip_new_connection_id_frame_non_mp() {
        // Frame type (0x18) + Sequence (1) + Retire prior (1) + CID len (1) + CID (4) + reset token (16)
        let mut data = vec![0x18, 0x01, 0x00, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
        data.extend_from_slice(&[0u8; 16]); // reset token
        data.push(0xAB);
        let rest = skip_new_connection_id_frame(&data, false).unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], 0xAB);
    }
}
