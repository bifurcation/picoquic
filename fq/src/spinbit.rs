//! Spin bit handling for passive RTT measurement.
//!
//! Translated from picoquic/spinbit.c.
//!
//! The spin bit is a mechanism defined in RFC 9000 that allows network
//! observers to passively measure round-trip time. The bit alternates
//! with each round trip, enabling RTT estimation without active probing.
//!
//! Three variants are provided:
//! - Basic: Standard spin bit behavior per RFC 9000
//! - Null: Spin bit always zero (disables RTT measurement)
//! - Random: Spin bit randomized (prevents RTT measurement, adds privacy)

use crate::util::test_random;

// =============================================================================
// Spin Bit Constants
// =============================================================================

/// Bit position of spin bit in packet header (bit 5).
pub const SPIN_BIT_POSITION: u8 = 5;

/// Mask for spin bit in packet header.
pub const SPIN_BIT_MASK: u8 = 1 << SPIN_BIT_POSITION;

// =============================================================================
// Spin Bit Variant Enum
// =============================================================================

/// Spin bit behavior variants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinBitVariant {
    /// Standard spin bit per RFC 9000.
    #[default]
    Basic = 0,
    /// Randomized spin bit (privacy mode).
    Random = 1,
    /// Null spin bit (always zero).
    Null = 2,
}

impl SpinBitVariant {
    /// Get variant from index.
    pub fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(SpinBitVariant::Basic),
            1 => Some(SpinBitVariant::Random),
            2 => Some(SpinBitVariant::Null),
            _ => None,
        }
    }
}

// =============================================================================
// Spin Bit State
// =============================================================================

/// Spin bit state for a connection path.
#[derive(Debug, Clone, Copy, Default)]
pub struct SpinBitState {
    /// Current spin bit value (0 or 1).
    pub current_spin: u8,
}

impl SpinBitState {
    /// Create a new spin bit state.
    pub fn new() -> Self {
        Self { current_spin: 0 }
    }

    /// Process incoming packet spin bit (basic variant).
    ///
    /// The spin bit is XORed with client_mode to handle the direction.
    /// Client and server see opposite values, so XOR normalizes them.
    ///
    /// # Arguments
    /// * `received_spin` - Spin bit value from received packet
    /// * `is_client` - Whether this endpoint is the client
    pub fn process_incoming_basic(&mut self, received_spin: u8, is_client: bool) {
        self.current_spin = received_spin ^ (is_client as u8);
    }

    /// Get outgoing spin bit value (basic variant).
    ///
    /// Returns the spin bit shifted to its position in the packet header.
    pub fn get_outgoing_basic(&self) -> u8 {
        (self.current_spin & 1) << SPIN_BIT_POSITION
    }

    /// Process incoming packet spin bit (null variant).
    ///
    /// Does nothing - spin bit is ignored.
    pub fn process_incoming_null(&mut self, _received_spin: u8, _is_client: bool) {
        // No-op
    }

    /// Get outgoing spin bit value (null variant).
    ///
    /// Always returns zero.
    pub fn get_outgoing_null(&self) -> u8 {
        0
    }

    /// Process incoming packet spin bit (random variant).
    ///
    /// Does nothing - incoming spin bit is ignored when using random.
    pub fn process_incoming_random(&mut self, _received_spin: u8, _is_client: bool) {
        // No-op
    }

    /// Get outgoing spin bit value (random variant).
    ///
    /// Returns a random spin bit value.
    ///
    /// # Arguments
    /// * `random_context` - Random number generator state
    pub fn get_outgoing_random(&self, random_context: &mut u64) -> u8 {
        (test_random(random_context) as u8) & SPIN_BIT_MASK
    }

    /// Process incoming packet based on variant.
    pub fn process_incoming(
        &mut self,
        variant: SpinBitVariant,
        received_spin: u8,
        is_client: bool,
    ) {
        match variant {
            SpinBitVariant::Basic => self.process_incoming_basic(received_spin, is_client),
            SpinBitVariant::Random => self.process_incoming_random(received_spin, is_client),
            SpinBitVariant::Null => self.process_incoming_null(received_spin, is_client),
        }
    }

    /// Get outgoing spin bit based on variant.
    ///
    /// # Arguments
    /// * `variant` - Which spin bit variant to use
    /// * `random_context` - Random number generator state (only used for Random variant)
    pub fn get_outgoing(&self, variant: SpinBitVariant, random_context: &mut u64) -> u8 {
        match variant {
            SpinBitVariant::Basic => self.get_outgoing_basic(),
            SpinBitVariant::Random => self.get_outgoing_random(random_context),
            SpinBitVariant::Null => self.get_outgoing_null(),
        }
    }
}

// =============================================================================
// Spin Bit Functions (Standalone)
// =============================================================================

/// Process incoming spin bit using basic variant.
///
/// # Arguments
/// * `current_spin` - Current spin state (will be updated)
/// * `received_spin` - Spin bit from received packet header
/// * `is_client` - Whether this endpoint is the client
///
/// # Returns
/// New spin bit value
pub fn spinbit_basic_incoming(received_spin: u8, is_client: bool) -> u8 {
    received_spin ^ (is_client as u8)
}

/// Get outgoing spin bit using basic variant.
///
/// # Arguments
/// * `current_spin` - Current spin state
///
/// # Returns
/// Spin bit value positioned for packet header
pub fn spinbit_basic_outgoing(current_spin: u8) -> u8 {
    (current_spin & 1) << SPIN_BIT_POSITION
}

/// Get outgoing spin bit using null variant.
///
/// Always returns zero.
pub fn spinbit_null_outgoing() -> u8 {
    0
}

/// Get outgoing spin bit using random variant.
///
/// Returns a random value in the spin bit position.
///
/// # Arguments
/// * `random_context` - Random number generator state
pub fn spinbit_random_outgoing(random_context: &mut u64) -> u8 {
    (test_random(random_context) as u8) & SPIN_BIT_MASK
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spin_bit_constants() {
        assert_eq!(SPIN_BIT_POSITION, 5);
        assert_eq!(SPIN_BIT_MASK, 0x20);
    }

    #[test]
    fn test_spin_bit_variant() {
        assert_eq!(SpinBitVariant::Basic as u8, 0);
        assert_eq!(SpinBitVariant::Random as u8, 1);
        assert_eq!(SpinBitVariant::Null as u8, 2);

        assert_eq!(SpinBitVariant::from_index(0), Some(SpinBitVariant::Basic));
        assert_eq!(SpinBitVariant::from_index(1), Some(SpinBitVariant::Random));
        assert_eq!(SpinBitVariant::from_index(2), Some(SpinBitVariant::Null));
        assert_eq!(SpinBitVariant::from_index(3), None);
    }

    #[test]
    fn test_spin_bit_state_default() {
        let state = SpinBitState::default();
        assert_eq!(state.current_spin, 0);
    }

    #[test]
    fn test_spin_bit_basic_client() {
        let mut state = SpinBitState::new();

        // Client receives spin=0 from server -> current = 0 ^ 1 = 1
        state.process_incoming_basic(0, true);
        assert_eq!(state.current_spin, 1);

        // Client receives spin=1 from server -> current = 1 ^ 1 = 0
        state.process_incoming_basic(1, true);
        assert_eq!(state.current_spin, 0);
    }

    #[test]
    fn test_spin_bit_basic_server() {
        let mut state = SpinBitState::new();

        // Server receives spin=0 from client -> current = 0 ^ 0 = 0
        state.process_incoming_basic(0, false);
        assert_eq!(state.current_spin, 0);

        // Server receives spin=1 from client -> current = 1 ^ 0 = 1
        state.process_incoming_basic(1, false);
        assert_eq!(state.current_spin, 1);
    }

    #[test]
    fn test_spin_bit_basic_outgoing() {
        let mut state = SpinBitState::new();

        state.current_spin = 0;
        assert_eq!(state.get_outgoing_basic(), 0x00);

        state.current_spin = 1;
        assert_eq!(state.get_outgoing_basic(), 0x20);
    }

    #[test]
    fn test_spin_bit_null() {
        let mut state = SpinBitState::new();
        state.current_spin = 1;

        // Null incoming does nothing
        state.process_incoming_null(0, true);
        assert_eq!(state.current_spin, 1); // Unchanged

        // Null outgoing always returns 0
        assert_eq!(state.get_outgoing_null(), 0);
    }

    #[test]
    fn test_spin_bit_random_incoming() {
        let mut state = SpinBitState::new();
        state.current_spin = 1;

        // Random incoming does nothing
        state.process_incoming_random(0, true);
        assert_eq!(state.current_spin, 1); // Unchanged
    }

    #[test]
    fn test_spin_bit_random_outgoing() {
        let state = SpinBitState::new();
        let mut random_ctx: u64 = 12345;

        // Random outgoing returns value in spin bit position
        let out = state.get_outgoing_random(&mut random_ctx);
        // Should be either 0x00 or 0x20
        assert!(out == 0x00 || out == 0x20);
    }

    #[test]
    fn test_spin_bit_process_incoming_dispatch() {
        let mut state = SpinBitState::new();

        state.process_incoming(SpinBitVariant::Basic, 1, true);
        assert_eq!(state.current_spin, 0); // 1 ^ 1 = 0

        state.current_spin = 5; // Set to something else
        state.process_incoming(SpinBitVariant::Null, 1, true);
        assert_eq!(state.current_spin, 5); // Unchanged

        state.process_incoming(SpinBitVariant::Random, 1, true);
        assert_eq!(state.current_spin, 5); // Unchanged
    }

    #[test]
    fn test_spin_bit_get_outgoing_dispatch() {
        let mut state = SpinBitState::new();
        state.current_spin = 1;
        let mut random_ctx: u64 = 12345;

        assert_eq!(
            state.get_outgoing(SpinBitVariant::Basic, &mut random_ctx),
            0x20
        );
        assert_eq!(
            state.get_outgoing(SpinBitVariant::Null, &mut random_ctx),
            0x00
        );
        // Random could be 0x00 or 0x20
        let random_out = state.get_outgoing(SpinBitVariant::Random, &mut random_ctx);
        assert!(random_out == 0x00 || random_out == 0x20);
    }

    #[test]
    fn test_standalone_functions() {
        // Basic incoming
        assert_eq!(spinbit_basic_incoming(0, true), 1);
        assert_eq!(spinbit_basic_incoming(1, true), 0);
        assert_eq!(spinbit_basic_incoming(0, false), 0);
        assert_eq!(spinbit_basic_incoming(1, false), 1);

        // Basic outgoing
        assert_eq!(spinbit_basic_outgoing(0), 0x00);
        assert_eq!(spinbit_basic_outgoing(1), 0x20);

        // Null outgoing
        assert_eq!(spinbit_null_outgoing(), 0);

        // Random outgoing
        let mut random_ctx: u64 = 54321;
        let out = spinbit_random_outgoing(&mut random_ctx);
        assert!(out == 0x00 || out == 0x20);
    }

    #[test]
    fn test_spin_bit_round_trip() {
        // Simulate a full round trip
        let mut client_state = SpinBitState::new();
        let mut server_state = SpinBitState::new();

        // Initial state: both at 0
        assert_eq!(client_state.current_spin, 0);
        assert_eq!(server_state.current_spin, 0);

        // Client sends packet with spin=0, server receives
        let client_out = client_state.get_outgoing_basic() >> SPIN_BIT_POSITION;
        server_state.process_incoming_basic(client_out, false);
        assert_eq!(server_state.current_spin, 0);

        // Server sends packet with spin=0, client receives
        let server_out = server_state.get_outgoing_basic() >> SPIN_BIT_POSITION;
        client_state.process_incoming_basic(server_out, true);
        // Client: 0 ^ 1 = 1 (spin flips on client side)
        assert_eq!(client_state.current_spin, 1);

        // Client sends packet with spin=1, server receives
        let client_out = client_state.get_outgoing_basic() >> SPIN_BIT_POSITION;
        server_state.process_incoming_basic(client_out, false);
        assert_eq!(server_state.current_spin, 1);

        // Server sends packet with spin=1, client receives
        let server_out = server_state.get_outgoing_basic() >> SPIN_BIT_POSITION;
        client_state.process_incoming_basic(server_out, true);
        // Client: 1 ^ 1 = 0 (spin flips again)
        assert_eq!(client_state.current_spin, 0);
    }
}
