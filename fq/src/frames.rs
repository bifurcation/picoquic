//! Frame skip and parse functions.
//!
//! This module provides functions for skipping over and parsing QUIC frames
//! without requiring the full connection context. These are used for:
//! - Fast packet parsing
//! - Frame validation
//! - Loss detection (checking if frames need retransmission)
//!
//! Translated from picoquic/frames.c

use crate::connection::CConnectionView;
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

/// Parse a MAX_DATA frame.
///
/// Expects bytes to start with the frame type (0x10).
/// Returns (maxdata_value, remaining_bytes) or None on error.
pub fn parse_max_data_frame(bytes: &[u8]) -> Option<(u64, &[u8])> {
    if bytes.is_empty() {
        return None;
    }
    // Skip the 1-byte frame type (0x10 = MAX_DATA)
    frames_varint_decode(&bytes[1..])
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

/// Parse a NEW_CONNECTION_ID frame.
///
/// Returns (path_id, sequence, retire_before, cid_bytes, secret_bytes, remaining_bytes).
/// path_id is 0 if not multipath.
/// cid_bytes and secret_bytes are slices into the original bytes.
#[allow(clippy::type_complexity)]
pub fn parse_new_connection_id_frame(
    bytes: &[u8],
    is_mp: bool,
) -> Option<(u64, u64, u64, &[u8], &[u8], &[u8])> {
    let mut rest = bytes;

    // Skip frame type
    rest = frames_varint_skip(rest)?;

    // Path ID if multipath
    let mut path_id = 0u64;
    if is_mp {
        let (pid, r) = frames_varint_decode(rest)?;
        path_id = pid;
        rest = r;
    }

    // Sequence and retire_before
    let (sequence, rest2) = frames_varint_decode(rest)?;
    let (retire_before, rest3) = frames_varint_decode(rest2)?;

    // CID length
    let (cid_length, rest4) = frames_uint8_decode(rest3)?;
    let cid_len = cid_length as usize;

    // Check we have enough bytes for CID + reset secret
    if rest4.len() < cid_len + RESET_SECRET_SIZE {
        return None;
    }

    let cid_bytes = &rest4[..cid_len];
    let secret_bytes = &rest4[cid_len..cid_len + RESET_SECRET_SIZE];
    let remaining = &rest4[cid_len + RESET_SECRET_SIZE..];

    Some((
        path_id,
        sequence,
        retire_before,
        cid_bytes,
        secret_bytes,
        remaining,
    ))
}

/// Parse a BDP frame.
///
/// Returns (lifetime, bytes_in_flight, min_rtt, ip_addr, remaining_bytes).
/// ip_addr is a slice into the original bytes (4 or 16 bytes).
#[allow(clippy::type_complexity)]
pub fn parse_bdp_frame(bytes: &[u8]) -> Option<(u64, u64, u64, &[u8], &[u8])> {
    let (lifetime, rest) = frames_varint_decode(bytes)?;
    let (bytes_in_flight, rest) = frames_varint_decode(rest)?;
    let (min_rtt, rest) = frames_varint_decode(rest)?;
    let (ip_length, rest) = frames_varint_decode(rest)?;

    // IP address must be 4 (IPv4) or 16 (IPv6) bytes
    if ip_length != 4 && ip_length != 16 {
        return None;
    }

    let ip_len = ip_length as usize;
    if rest.len() < ip_len {
        return None;
    }

    let ip_addr = &rest[..ip_len];
    let remaining = &rest[ip_len..];

    Some((lifetime, bytes_in_flight, min_rtt, ip_addr, remaining))
}

// =============================================================================
// Stream Frame Functions
// =============================================================================

/// Stream frame type range (0x08-0x0f).
pub const STREAM_FRAME_TYPE_MIN: u8 = 0x08;
pub const STREAM_FRAME_TYPE_MAX: u8 = 0x0f;

/// Stream frame flag bits.
pub const STREAM_FLAG_FIN: u8 = 0x01;
pub const STREAM_FLAG_LEN: u8 = 0x02;
pub const STREAM_FLAG_OFF: u8 = 0x04;

/// Check if a byte is a stream frame type.
#[inline]
pub fn is_stream_frame_type(byte: u8) -> bool {
    (STREAM_FRAME_TYPE_MIN..=STREAM_FRAME_TYPE_MAX).contains(&byte)
}

/// Check if a stream frame has no length field (unlimited).
///
/// A stream frame is "unlimited" when the LEN bit (0x02) is not set,
/// meaning the data extends to the end of the packet.
#[inline]
pub fn is_stream_frame_unlimited(first_byte: u8) -> bool {
    (first_byte & STREAM_FLAG_LEN) == 0
}

/// Parse a stream frame header.
///
/// Returns (stream_id, offset, data_length, fin, consumed_bytes) on success.
/// Returns None on parse error.
///
/// Note: For frames without the LEN bit set, data_length will be the remaining
/// bytes in the input (bytes_max - consumed).
pub fn parse_stream_header(bytes: &[u8]) -> Option<(u64, u64, usize, bool, usize)> {
    if bytes.is_empty() {
        return None;
    }

    let first_byte = bytes[0];
    let has_len = (first_byte & STREAM_FLAG_LEN) != 0;
    let has_off = (first_byte & STREAM_FLAG_OFF) != 0;
    let fin = (first_byte & STREAM_FLAG_FIN) != 0;

    let mut index = 1usize;

    // Parse stream ID
    if index >= bytes.len() {
        return None;
    }
    let (stream_id, rest) = frames_varint_decode(&bytes[index..])?;
    index += bytes.len() - index - rest.len();

    // Parse offset (if present)
    let offset = if has_off {
        if index >= bytes.len() {
            return None;
        }
        let (off, rest2) = frames_varint_decode(&bytes[index..])?;
        index += bytes.len() - index - rest2.len();
        off
    } else {
        0
    };

    // Parse length (if present)
    let data_length = if has_len {
        if index >= bytes.len() {
            return None;
        }
        let (len, rest3) = frames_varint_decode(&bytes[index..])?;
        let len_size = bytes.len() - index - rest3.len();
        index += len_size;

        // Validate that data fits
        if index + len as usize > bytes.len() {
            return None;
        }
        len as usize
    } else {
        // No length field - data extends to end of packet
        bytes.len() - index
    };

    Some((stream_id, offset, data_length, fin, index))
}

// =============================================================================
// Packet Number Functions
// =============================================================================

/// Reconstruct the full 64-bit packet number from a truncated packet number.
///
/// QUIC packet numbers are transmitted with a variable number of least-significant
/// bits. This function reconstructs the full packet number using the highest
/// packet number seen so far.
///
/// # Arguments
/// * `highest` - The highest packet number seen so far in this space
/// * `mask` - The mask for the truncated packet number (e.g., 0xFF for 1 byte,
///   0xFFFF for 2 bytes, 0xFFFFFF for 3 bytes, 0xFFFFFFFF for 4 bytes)
/// * `pn` - The truncated packet number from the packet
///
/// # Returns
/// The reconstructed full 64-bit packet number.
pub fn get_packet_number64(highest: u64, mask: u64, pn: u32) -> u64 {
    let expected = highest.wrapping_add(1);
    let not_mask_plus_one = (!mask).wrapping_add(1);
    let mut pn64 = (expected & mask) | u64::from(pn);

    if pn64 < expected {
        let delta1 = expected - pn64;
        let delta2 = not_mask_plus_one.wrapping_sub(delta1);
        if delta2 < delta1 {
            pn64 = pn64.wrapping_add(not_mask_plus_one);
        }
    } else {
        let delta1 = pn64 - expected;
        let delta2 = not_mask_plus_one.wrapping_sub(delta1);

        if delta2 <= delta1 && (pn64 & mask) > 0 {
            // Out of sequence packet from previous roll
            pn64 = pn64.wrapping_sub(not_mask_plus_one);
        }
    }

    pn64
}

// =============================================================================
// ACK Frame Functions
// =============================================================================

/// Result of parsing an ACK frame header.
#[derive(Debug, Clone, Copy)]
pub struct AckHeader {
    /// Number of additional ACK blocks after the first range.
    pub num_block: u64,
    /// Path ID (for multipath ACK frames).
    pub path_id: Option<u64>,
    /// Largest acknowledged packet number.
    pub largest: u64,
    /// ACK delay (already scaled by exponent, in microseconds).
    pub ack_delay: u64,
    /// Number of bytes consumed from input.
    pub consumed: usize,
}

/// Parse an ACK frame header.
///
/// This parses the fixed header portion of an ACK frame:
/// - Frame type (already skipped by caller, or handled here)
/// - [Path ID] (if is_multipath is true)
/// - Largest Acknowledged
/// - ACK Delay (scaled by ack_delay_exponent)
/// - ACK Range Count
///
/// Returns None on parse error.
///
/// # Arguments
/// * `bytes` - The frame bytes starting at the frame type
/// * `is_multipath` - Whether this is a multipath ACK frame (has path_id)
/// * `ack_delay_exponent` - Exponent to scale the ack delay value
pub fn parse_ack_header(
    bytes: &[u8],
    is_multipath: bool,
    ack_delay_exponent: u8,
) -> Option<AckHeader> {
    if bytes.is_empty() {
        return None;
    }

    // Skip frame type - use varint_decode_length to determine frame type size
    let frame_type_len = varint_decode_length(bytes[0]);
    let mut rest = &bytes[frame_type_len..];

    // Parse path_id if multipath
    let path_id = if is_multipath {
        let (pid, remaining) = frames_varint_decode(rest)?;
        rest = remaining;
        Some(pid)
    } else {
        None
    };

    // Parse largest
    let (largest, remaining) = frames_varint_decode(rest)?;
    rest = remaining;

    // Parse ack_delay and scale by exponent
    let (raw_delay, remaining) = frames_varint_decode(rest)?;
    let ack_delay = raw_delay << ack_delay_exponent;
    rest = remaining;

    // Parse num_block
    let (num_block, remaining) = frames_varint_decode(rest)?;
    rest = remaining;

    let consumed = bytes.len() - rest.len();

    Some(AckHeader {
        num_block,
        path_id,
        largest,
        ack_delay,
        consumed,
    })
}

/// Get the length of a varint based on its first byte.
#[inline]
fn varint_decode_length(first_byte: u8) -> usize {
    1 << (first_byte >> 6)
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
// Parse Function FFI Exports
// =============================================================================

/// FFI export: Parse NEW_CONNECTION_ID frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_new_connection_id_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    is_mp: c_int,
    path_id: *mut u64,
    sequence: *mut u64,
    retire_before: *mut u64,
    cid_length: *mut u8,
    cnxid_bytes: *mut *const u8,
    secret_bytes: *mut *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_new_connection_id_frame(slice, is_mp != 0) {
        Some((pid, seq, retire, cid, secret, rest)) => {
            *path_id = pid;
            *sequence = seq;
            *retire_before = retire;
            *cid_length = cid.len() as u8;
            *cnxid_bytes = cid.as_ptr();
            *secret_bytes = secret.as_ptr();
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse RETIRE_CONNECTION_ID frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_retire_connection_id_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    unique_path_id: *mut u64,
    sequence: *mut u64,
    is_mp: c_int,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_retire_connection_id_frame(slice, is_mp != 0) {
        Some((pid, seq, rest)) => {
            *unique_path_id = pid;
            *sequence = seq;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse ACK_FREQUENCY frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_ack_frequency_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    seq: *mut u64,
    packets: *mut u64,
    microsec: *mut u64,
    ignore_order: *mut u8,
    reordering_threshold: *mut u64,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_ack_frequency_frame(slice) {
        Some((s, p, m, r, rest)) => {
            *seq = s;
            *packets = p;
            *microsec = m;
            *reordering_threshold = r;
            *ignore_order = if r == 0 { 1 } else { 0 };
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse TIME_STAMP frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_time_stamp_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    time_stamp: *mut u64,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_time_stamp_frame(slice) {
        Some((ts, rest)) => {
            *time_stamp = ts;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse PATH_ABANDON frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_path_abandon_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    path_id: *mut u64,
    reason: *mut u64,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_path_abandon_frame(slice) {
        Some((pid, r, rest)) => {
            *path_id = pid;
            *reason = r;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse PATH_AVAILABLE or PATH_BACKUP frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_path_available_or_backup_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    path_id: *mut u64,
    sequence: *mut u64,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_path_available_or_backup_frame(slice) {
        Some((pid, seq, rest)) => {
            *path_id = pid;
            *sequence = seq;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse MAX_PATH_ID frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_max_path_id_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    max_path_id: *mut u64,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_max_path_id_frame(slice) {
        Some((mpid, rest)) => {
            *max_path_id = mpid;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse PATHS_BLOCKED frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_paths_blocked_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    max_path_id: *mut u64,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_paths_blocked_frame(slice) {
        Some((mpid, rest)) => {
            *max_path_id = mpid;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse PATH_CID_BLOCKED frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_path_cid_blocked_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    unique_path_id: *mut u64,
    next_sequence_number: *mut u64,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_path_cid_blocked_frame(slice) {
        Some((pid, nsn, rest)) => {
            *unique_path_id = pid;
            *next_sequence_number = nsn;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse OBSERVED_ADDRESS frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_observed_address_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    ftype: u64,
    sequence: *mut u64,
    addr: *mut *const u8,
    port: *mut u16,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_observed_address_frame(slice, ftype) {
        Some((seq, a, p, rest)) => {
            *sequence = seq;
            *addr = a.as_ptr();
            *port = p;
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Parse BDP frame.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_bdp_frame(
    bytes: *const u8,
    bytes_max: *const u8,
    lifetime: *mut u64,
    recon_bytes_in_flight: *mut u64,
    recon_min_rtt: *mut u64,
    saved_ip_length: *mut u64,
    saved_ip: *mut *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_bdp_frame(slice) {
        Some((lt, bif, mrtt, ip, rest)) => {
            *lifetime = lt;
            *recon_bytes_in_flight = bif;
            *recon_min_rtt = mrtt;
            *saved_ip_length = ip.len() as u64;
            *saved_ip = ip.as_ptr();
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// FFI export: Check if stream frame has no length field.
///
/// # Safety
/// `bytes` must point to a valid stream frame first byte.
#[no_mangle]
pub unsafe extern "C" fn picoquic_is_stream_frame_unlimited(bytes: *const u8) -> c_int {
    if bytes.is_null() {
        return 0;
    }
    if is_stream_frame_unlimited(*bytes) {
        1
    } else {
        0
    }
}

/// FFI export: Parse stream frame header.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_stream_header(
    bytes: *const u8,
    bytes_max: usize,
    stream_id: *mut u64,
    offset: *mut u64,
    data_length: *mut usize,
    fin: *mut c_int,
    consumed: *mut usize,
) -> c_int {
    if bytes.is_null() || bytes_max == 0 {
        return -1;
    }
    let slice = std::slice::from_raw_parts(bytes, bytes_max);

    match parse_stream_header(slice) {
        Some((sid, off, len, f, cons)) => {
            *stream_id = sid;
            *offset = off;
            *data_length = len;
            *fin = if f { 1 } else { 0 };
            *consumed = cons;
            0
        }
        None => -1,
    }
}

/// FFI export: Reconstruct full 64-bit packet number.
///
/// # Safety
/// This function is always safe to call.
#[no_mangle]
pub extern "C" fn picoquic_get_packet_number64(highest: u64, mask: u64, pn: u32) -> u64 {
    get_packet_number64(highest, mask, pn)
}

/// FFI export: Parse ACK frame header.
///
/// # Safety
/// All pointers must be valid. Output pointers must be non-null.
/// `path_id` can be null if this is not a multipath ACK frame.
#[no_mangle]
pub unsafe extern "C" fn picoquic_parse_ack_header(
    bytes: *const u8,
    bytes_max: usize,
    num_block: *mut u64,
    path_id: *mut u64,
    largest: *mut u64,
    ack_delay: *mut u64,
    consumed: *mut usize,
    ack_delay_exponent: u8,
) -> c_int {
    if bytes.is_null() || bytes_max == 0 {
        return -1;
    }
    let slice = std::slice::from_raw_parts(bytes, bytes_max);
    let is_multipath = !path_id.is_null();

    match parse_ack_header(slice, is_multipath, ack_delay_exponent) {
        Some(header) => {
            *num_block = header.num_block;
            if !path_id.is_null() {
                *path_id = header.path_id.unwrap_or(0);
            }
            *largest = header.largest;
            *ack_delay = header.ack_delay;
            *consumed = header.consumed;
            0
        }
        None => -1,
    }
}

/// FFI export: Decode MAX_DATA frame and apply to connection.
///
/// Parses a MAX_DATA frame starting at `bytes` and updates the connection's
/// maxdata_remote and sent_blocked_frame fields if the new value is larger.
///
/// # Safety
/// - `cnx` must be a valid pointer to a CConnectionView struct.
/// - `bytes` and `bytes_max` must form a valid memory range.
/// - The caller is responsible for calling `picoquic_connection_error` on
///   parse failure (when this function returns null).
#[no_mangle]
pub unsafe extern "C" fn picoquic_decode_max_data_frame_ffi(
    cnx: *mut CConnectionView,
    bytes: *const u8,
    bytes_max: *const u8,
) -> *const u8 {
    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        return std::ptr::null();
    };

    match parse_max_data_frame(slice) {
        Some((maxdata, rest)) => {
            // Convert to Rust, apply, convert back
            let mut conn = (*cnx).to_rust();
            conn.apply_max_data(maxdata);
            (*cnx).from_rust(&conn);
            rest.as_ptr()
        }
        None => std::ptr::null(),
    }
}

/// Decode result for MAX_PATH_ID frame.
///
/// Used to communicate the outcome to the C caller so it can call
/// the appropriate error function if needed.
#[repr(C)]
pub enum MaxPathIdResult {
    /// Success - max_path_id_remote was updated if needed.
    Success = 0,
    /// Multipath not enabled - caller should call connection_error_ex.
    MultipathNotEnabled = 1,
    /// Parse error - caller should call connection_error_ex.
    ParseError = 2,
}

/// FFI export: Decode MAX_PATH_ID frame and apply to connection.
///
/// Parses a MAX_PATH_ID frame and updates max_path_id_remote if larger.
/// The frame type is assumed to be already skipped by the caller.
///
/// Returns the result via `result_out` and the new byte pointer.
/// Returns null on error (check result_out for the error type).
///
/// # Safety
/// - `cnx` must be a valid pointer to a CConnectionView struct.
/// - `bytes` and `bytes_max` must form a valid memory range.
/// - `result_out` must be a valid pointer.
/// - The caller handles error reporting via picoquic_connection_error_ex.
#[no_mangle]
pub unsafe extern "C" fn picoquic_decode_max_path_id_frame_ffi(
    bytes: *const u8,
    bytes_max: *const u8,
    cnx: *mut CConnectionView,
    result_out: *mut MaxPathIdResult,
) -> *const u8 {
    // Check multipath enabled
    if (*cnx).is_multipath_enabled == 0 {
        *result_out = MaxPathIdResult::MultipathNotEnabled;
        return std::ptr::null();
    }

    let Some(slice) = slice_from_ptrs(bytes, bytes_max) else {
        *result_out = MaxPathIdResult::ParseError;
        return std::ptr::null();
    };

    match parse_max_path_id_frame(slice) {
        Some((max_path_id, rest)) => {
            let mut conn = (*cnx).to_rust();
            conn.apply_max_path_id(max_path_id);
            (*cnx).from_rust(&conn);
            *result_out = MaxPathIdResult::Success;
            rest.as_ptr()
        }
        None => {
            *result_out = MaxPathIdResult::ParseError;
            std::ptr::null()
        }
    }
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
    fn test_parse_max_data_frame() {
        // MAX_DATA frame: type (0x10) + maxdata varint (1000 = 0x43, 0xE8)
        let data = [0x10, 0x43, 0xE8, 0xAB];
        let (maxdata, rest) = parse_max_data_frame(&data).unwrap();
        assert_eq!(maxdata, 1000);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], 0xAB);
    }

    #[test]
    fn test_parse_max_data_frame_large_value() {
        // MAX_DATA with 4-byte varint: 0x80000064 = 100 with 4-byte prefix
        let data = [0x10, 0x80, 0x00, 0x00, 0x64, 0xAB];
        let (maxdata, rest) = parse_max_data_frame(&data).unwrap();
        assert_eq!(maxdata, 100);
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

    #[test]
    fn test_is_stream_frame_unlimited() {
        // Stream frame 0x08 (no FIN, no LEN, no OFF)
        assert!(is_stream_frame_unlimited(0x08));
        // Stream frame 0x09 (FIN, no LEN, no OFF)
        assert!(is_stream_frame_unlimited(0x09));
        // Stream frame 0x0A (no FIN, LEN, no OFF)
        assert!(!is_stream_frame_unlimited(0x0A));
        // Stream frame 0x0F (FIN, LEN, OFF)
        assert!(!is_stream_frame_unlimited(0x0F));
    }

    #[test]
    fn test_parse_stream_header_basic() {
        // Stream frame type 0x08 (no offset, no length, no fin)
        // Frame type (0x08) + Stream ID (varint 5) + data to end
        let data = [0x08, 0x05, 0x01, 0x02, 0x03];
        let (stream_id, offset, data_length, fin, consumed) = parse_stream_header(&data).unwrap();
        assert_eq!(stream_id, 5);
        assert_eq!(offset, 0);
        assert_eq!(data_length, 3); // Remaining bytes
        assert!(!fin);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_parse_stream_header_with_offset_and_length() {
        // Stream frame type 0x0E (no fin, LEN, OFF)
        // Frame type + Stream ID (5) + Offset (100) + Length (3) + data
        let data = [0x0E, 0x05, 0x40, 0x64, 0x03, 0x01, 0x02, 0x03, 0xAB];
        let (stream_id, offset, data_length, fin, consumed) = parse_stream_header(&data).unwrap();
        assert_eq!(stream_id, 5);
        assert_eq!(offset, 100);
        assert_eq!(data_length, 3);
        assert!(!fin);
        assert_eq!(consumed, 5);
    }

    #[test]
    fn test_parse_stream_header_with_fin() {
        // Stream frame type 0x0F (FIN, LEN, OFF)
        let data = [0x0F, 0x05, 0x00, 0x02, 0xAA, 0xBB];
        let (stream_id, offset, data_length, fin, consumed) = parse_stream_header(&data).unwrap();
        assert_eq!(stream_id, 5);
        assert_eq!(offset, 0);
        assert_eq!(data_length, 2);
        assert!(fin);
        assert_eq!(consumed, 4);
    }

    #[test]
    fn test_get_packet_number64_one_byte() {
        // 1-byte PN: mask = 0xFFFFFFFFFFFFFF00 (high bits preserved)
        // highest=0xDEADBEEF, pn=0xF0 -> expected=0xDEADBEF0
        let pn64 = get_packet_number64(0xDEADBEEF, 0xFFFFFFFFFFFFFF00, 0xF0);
        assert_eq!(pn64, 0xDEADBEF0);
    }

    #[test]
    fn test_get_packet_number64_one_byte_same() {
        // 1-byte PN: exact match
        let pn64 = get_packet_number64(0xDEADBEEF, 0xFFFFFFFFFFFFFF00, 0xEF);
        assert_eq!(pn64, 0xDEADBEEF);
    }

    #[test]
    fn test_get_packet_number64_two_byte() {
        // 2-byte PN: mask = 0xFFFFFFFFFFFF0000
        // highest=0x10000, pn=0x8000 -> expected=0x18000
        let pn64 = get_packet_number64(0x10000, 0xFFFFFFFFFFFF0000, 0x8000);
        assert_eq!(pn64, 0x18000);
    }

    #[test]
    fn test_get_packet_number64_four_byte() {
        // 4-byte PN: mask = 0xFFFFFFFF00000000
        // highest=0xDEADBEEF, pn=0xDEADBEF0 -> expected=0xDEADBEF0
        let pn64 = get_packet_number64(0xDEADBEEF, 0xFFFFFFFF00000000, 0xDEADBEF0);
        assert_eq!(pn64, 0xDEADBEF0);
    }

    #[test]
    fn test_get_packet_number64_rollover() {
        // 4-byte PN with rollover: highest=0xDEADBEEF, pn=0 -> expected=0x100000000
        let pn64 = get_packet_number64(0xDEADBEEF, 0xFFFFFFFF00000000, 0);
        assert_eq!(pn64, 0x100000000);
    }

    #[test]
    fn test_stream_frame_type_checks() {
        assert!(is_stream_frame_type(0x08));
        assert!(is_stream_frame_type(0x0F));
        assert!(!is_stream_frame_type(0x07));
        assert!(!is_stream_frame_type(0x10));
    }

    #[test]
    fn test_parse_ack_header_basic() {
        // ACK frame: type (0x02) + largest (100) + ack_delay (5) + num_blocks (0)
        // With ack_delay_exponent=0, ack_delay stays as 5
        let data = [0x02, 0x40, 0x64, 0x05, 0x00, 0xAB];
        let header = parse_ack_header(&data, false, 0).unwrap();
        assert_eq!(header.largest, 100);
        assert_eq!(header.ack_delay, 5);
        assert_eq!(header.num_block, 0);
        assert!(header.path_id.is_none());
        assert_eq!(header.consumed, 5);
    }

    #[test]
    fn test_parse_ack_header_with_exponent() {
        // ACK frame with ack_delay_exponent=3 (multiply by 8)
        let data = [0x02, 0x0A, 0x04, 0x00, 0xAB]; // largest=10, delay=4*8=32, blocks=0
        let header = parse_ack_header(&data, false, 3).unwrap();
        assert_eq!(header.largest, 10);
        assert_eq!(header.ack_delay, 32); // 4 << 3
        assert_eq!(header.num_block, 0);
    }

    #[test]
    fn test_parse_ack_header_multipath() {
        // MP ACK frame: type (2-byte) + path_id + largest + delay + blocks
        let data = [0x40, 0x02, 0x05, 0x0A, 0x02, 0x01, 0xAB];
        // type=0x02, path_id=5, largest=10, delay=2, blocks=1
        let header = parse_ack_header(&data, true, 0).unwrap();
        assert_eq!(header.path_id, Some(5));
        assert_eq!(header.largest, 10);
        assert_eq!(header.ack_delay, 2);
        assert_eq!(header.num_block, 1);
    }
}
