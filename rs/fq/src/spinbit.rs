//! Spin-bit policy implementations.
//!
//! Concrete types implementing [`crate::internal::SpinBitPolicy`].
//! [`crate::internal::SPIN_FUNCTION_TABLE`] entries are indexed by
//! [`crate::SpinbitVersion`] discriminant:
//! * 0 (`Basic`) → [`BasicSpinBit`]
//! * 1 (`Random`) → [`RandomSpinBit`]
//! * 2 (`Null`) → [`NullSpinBit`]
//!
//! C: `picoquic/spinbit.c` and the `picoquic_spin_function_table[]` array.

use crate::internal::{Connection, PacketHeader, Path, SpinBitPolicy};

// ---------------------------------------------------------------------------
// Basic spin-bit policy.

/// Reflects the incoming spin bit XOR client_mode, echoes path[0]'s spin
/// on outgoing packets.
/// C: `picoquic_spinbit_basic_incoming` / `picoquic_spinbit_basic_outgoing`
/// in `picoquic/spinbit.c`.
pub struct BasicSpinBit;

impl SpinBitPolicy for BasicSpinBit {
    /// C: `picoquic/spinbit.c:picoquic_spinbit_basic_incoming` (line 32–35).
    fn incoming(&self, connection: &mut Connection, path_x: &mut Path, ph: &PacketHeader) {
        path_x.current_spin = ph.spin ^ connection.client_mode;
    }

    /// C: `picoquic/spinbit.c:picoquic_spinbit_basic_outgoing` (line 37–42).
    fn outgoing(&self, connection: &mut Connection) -> u8 {
        let spin = connection
            .paths
            .first()
            .map(|p| p.current_spin)
            .unwrap_or(false);
        (spin as u8) << 5
    }
}

// ---------------------------------------------------------------------------
// Null spin-bit policy.

/// No-op incoming; zero outgoing.
/// C: `picoquic_spinbit_null_incoming` / `picoquic_spinbit_null_outgoing`
/// in `picoquic/spinbit.c`.
pub struct NullSpinBit;

impl SpinBitPolicy for NullSpinBit {
    /// C: `picoquic/spinbit.c:picoquic_spinbit_null_incoming` (line 48–53).
    fn incoming(&self, _connection: &mut Connection, _path_x: &mut Path, _ph: &PacketHeader) {}

    /// C: `picoquic/spinbit.c:picoquic_spinbit_null_outgoing` (line 55–59).
    fn outgoing(&self, _connection: &mut Connection) -> u8 {
        0
    }
}

// ---------------------------------------------------------------------------
// Random spin-bit policy.

/// No-op incoming; random bit-5 value outgoing.
/// C: `picoquic_spinbit_random_incoming` / `picoquic_spinbit_random_outgoing`
/// in `picoquic/spinbit.c`.
pub struct RandomSpinBit;

impl SpinBitPolicy for RandomSpinBit {
    /// C: `picoquic/spinbit.c:picoquic_spinbit_random_incoming` (line 65–69).
    fn incoming(&self, _connection: &mut Connection, _path_x: &mut Path, _ph: &PacketHeader) {}

    /// C: `picoquic/spinbit.c:picoquic_spinbit_random_outgoing` (line 72-76).
    fn outgoing(&self, _connection: &mut Connection) -> u8 {
        // C: `(uint8_t)(picoquic_public_random_64() & 0x20)` — bit 5 is the
        // spin-bit position in the QUIC short header first byte.
        crate::public_random_64() as u8 & 0x20
    }
}

/// Return a random outgoing spin bit in the QUIC short-header spin position.
///
/// C: `picoquic/spinbit.c:picoquic_spinbit_random_outgoing` (line 72-76).
pub fn picoquic_spinbit_random_outgoing(connection: &mut Connection) -> u8 {
    RandomSpinBit.outgoing(connection)
}
