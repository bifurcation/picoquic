//! Port blocking for protection against reflection attacks.
//!
//! This module provides functions to check if a port is on the blocked list.
//! The blocked port list protects QUIC servers from being used in reflection
//! attacks by dropping packets from known amplification-vulnerable services.
//!
//! Translated from picoquic/port_blocking.c

/// List of UDP ports commonly used in reflection/amplification attacks.
///
/// The list is sorted in descending order for efficient lookup.
/// Sources: Cloudflare blog, Microsoft msquic implementation.
pub const BLOCKED_PORT_LIST: &[u16] = &[
    27015, // SRCDS
    20800, // Call Of Duty
    11211, // memcache
    5353,  // mDNS
    1900,  // SSDP
    520,   // RIP
    500,   // IKE
    389,   // CLDAP
    161,   // SNMP
    138,   // NETBIOS Datagram Service
    137,   // NETBIOS Name Service
    123,   // NTP
    111,   // Portmap -- used by SUN RPC
    53,    // DNS
    19,    // Chargen
    17,    // Quote of the Day
    7,     // Echo
    0,     // Unusable
];

/// Check if a port is on the blocked list.
///
/// Returns `true` if the port should be blocked, `false` otherwise.
/// The list is searched in descending order, so we can stop early
/// when we find a port smaller than the target.
pub fn check_port_blocked(port: u16) -> bool {
    for &blocked in BLOCKED_PORT_LIST {
        if port > blocked {
            // List is descending, so no match possible
            break;
        }
        if port == blocked {
            return true;
        }
    }
    false
}

// =============================================================================
// FFI exports
// =============================================================================

/// FFI export: Check if a port is blocked.
#[no_mangle]
pub extern "C" fn picoquic_check_port_blocked(port: u16) -> std::ffi::c_int {
    if check_port_blocked(port) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_ports() {
        // Known blocked ports
        assert!(check_port_blocked(0));
        assert!(check_port_blocked(53)); // DNS
        assert!(check_port_blocked(123)); // NTP
        assert!(check_port_blocked(1900)); // SSDP
        assert!(check_port_blocked(11211)); // memcache
    }

    #[test]
    fn test_allowed_ports() {
        // Common QUIC ports should be allowed
        assert!(!check_port_blocked(443));
        assert!(!check_port_blocked(8443));
        assert!(!check_port_blocked(4433));
        assert!(!check_port_blocked(6121));
    }

    #[test]
    fn test_edge_cases() {
        // Port just above a blocked port
        assert!(!check_port_blocked(54)); // Just above DNS (53)
        assert!(!check_port_blocked(124)); // Just above NTP (123)

        // High ports
        assert!(!check_port_blocked(65535));
        assert!(!check_port_blocked(30000));
    }

    #[test]
    fn test_list_is_descending() {
        // Verify our list is in descending order for the optimization
        for i in 1..BLOCKED_PORT_LIST.len() {
            assert!(
                BLOCKED_PORT_LIST[i - 1] > BLOCKED_PORT_LIST[i],
                "List must be descending: {} should be > {}",
                BLOCKED_PORT_LIST[i - 1],
                BLOCKED_PORT_LIST[i]
            );
        }
    }
}
