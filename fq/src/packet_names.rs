//! Packet type names and constants.
//!
//! Translated from picoquic/packet_names.c.
//!
//! This module provides string names for QUIC packet types as defined
//! in RFC 9000.

use std::ffi::CStr;

// =============================================================================
// Packet Type Constants (from picoquic_internal.h)
// =============================================================================

/// QUIC packet types.
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// Error/invalid packet type.
    Error = 0,
    /// Version negotiation packet.
    VersionNegotiation = 1,
    /// Initial packet (contains CRYPTO frames for handshake).
    Initial = 2,
    /// Retry packet (server requests address validation).
    Retry = 3,
    /// Handshake packet (contains CRYPTO frames).
    Handshake = 4,
    /// 0-RTT protected packet (early data).
    ZeroRtt = 5,
    /// 1-RTT protected packet (application data).
    OneRtt = 6,
    /// Maximum packet type value (sentinel).
    Max = 7,
}

impl PacketType {
    /// Convert from u64 to PacketType.
    pub fn from_u64(value: u64) -> Option<Self> {
        match value {
            0 => Some(PacketType::Error),
            1 => Some(PacketType::VersionNegotiation),
            2 => Some(PacketType::Initial),
            3 => Some(PacketType::Retry),
            4 => Some(PacketType::Handshake),
            5 => Some(PacketType::ZeroRtt),
            6 => Some(PacketType::OneRtt),
            7 => Some(PacketType::Max),
            _ => None,
        }
    }

    /// Get the string name of this packet type.
    pub fn name(&self) -> &'static str {
        packet_type_name((*self) as u64)
    }
}

// =============================================================================
// Packet Type Name Lookup (Core Logic)
// =============================================================================

/// Get the name of a packet type as a C string.
///
/// This is the core lookup function - all logic lives here.
/// Returns "unknown" if the packet type is not recognized.
///
/// # Arguments
/// * `ptype` - The packet type number
///
/// # Returns
/// A static CStr with the packet type name
pub fn packet_type_name_cstr(ptype: u64) -> &'static CStr {
    match ptype {
        0 => c"error",
        1 => c"version_negotiation",
        2 => c"initial",
        3 => c"retry",
        4 => c"handshake",
        5 => c"0RTT",
        6 => c"1RTT",
        _ => c"unknown",
    }
}

/// Get the name of a packet type.
///
/// Convenience wrapper that returns `&str` for Rust callers.
///
/// # Arguments
/// * `ptype` - The packet type number
///
/// # Returns
/// A static string with the packet type name
pub fn packet_type_name(ptype: u64) -> &'static str {
    // Safe: all our CStr literals are valid UTF-8
    packet_type_name_cstr(ptype)
        .to_str()
        .expect("packet type names are ASCII")
}

// =============================================================================
// FFI Export (Thin Wrapper Only)
// =============================================================================

/// Get packet type name (FFI export).
///
/// Returns a pointer to a null-terminated static string.
#[no_mangle]
pub extern "C" fn picoquic_packet_type_name(ptype: u64) -> *const std::ffi::c_char {
    packet_type_name_cstr(ptype).as_ptr()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_type_enum_values() {
        assert_eq!(PacketType::Error as u64, 0);
        assert_eq!(PacketType::VersionNegotiation as u64, 1);
        assert_eq!(PacketType::Initial as u64, 2);
        assert_eq!(PacketType::Retry as u64, 3);
        assert_eq!(PacketType::Handshake as u64, 4);
        assert_eq!(PacketType::ZeroRtt as u64, 5);
        assert_eq!(PacketType::OneRtt as u64, 6);
        assert_eq!(PacketType::Max as u64, 7);
    }

    #[test]
    fn test_packet_type_from_u64() {
        assert_eq!(PacketType::from_u64(0), Some(PacketType::Error));
        assert_eq!(PacketType::from_u64(2), Some(PacketType::Initial));
        assert_eq!(PacketType::from_u64(6), Some(PacketType::OneRtt));
        assert_eq!(PacketType::from_u64(100), None);
    }

    #[test]
    fn test_packet_type_name_method() {
        assert_eq!(PacketType::Error.name(), "error");
        assert_eq!(PacketType::VersionNegotiation.name(), "version_negotiation");
        assert_eq!(PacketType::Initial.name(), "initial");
        assert_eq!(PacketType::Retry.name(), "retry");
        assert_eq!(PacketType::Handshake.name(), "handshake");
        assert_eq!(PacketType::ZeroRtt.name(), "0RTT");
        assert_eq!(PacketType::OneRtt.name(), "1RTT");
        assert_eq!(PacketType::Max.name(), "unknown");
    }

    #[test]
    fn test_packet_type_name_function() {
        assert_eq!(packet_type_name(0), "error");
        assert_eq!(packet_type_name(1), "version_negotiation");
        assert_eq!(packet_type_name(2), "initial");
        assert_eq!(packet_type_name(3), "retry");
        assert_eq!(packet_type_name(4), "handshake");
        assert_eq!(packet_type_name(5), "0RTT");
        assert_eq!(packet_type_name(6), "1RTT");
        assert_eq!(packet_type_name(7), "unknown");
        assert_eq!(packet_type_name(100), "unknown");
    }

    #[test]
    fn test_packet_type_name_cstr() {
        assert_eq!(packet_type_name_cstr(0), c"error");
        assert_eq!(packet_type_name_cstr(2), c"initial");
        assert_eq!(packet_type_name_cstr(999), c"unknown");
    }

    #[test]
    fn test_packet_type_name_ffi() {
        unsafe {
            let name_ptr = picoquic_packet_type_name(2);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "initial");

            let name_ptr = picoquic_packet_type_name(5);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "0RTT");

            let name_ptr = picoquic_packet_type_name(999);
            let name = CStr::from_ptr(name_ptr);
            assert_eq!(name.to_str().unwrap(), "unknown");
        }
    }
}
