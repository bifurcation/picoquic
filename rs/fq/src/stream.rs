//! QUIC stream identifiers.
//!
//! RFC 9000 §2.1 packs four pieces of information into the
//! 62-bit stream-id integer: the role of the endpoint that opened
//! the stream (client / server, low bit), the directionality (bidi /
//! uni, second-low bit), and a 1-based rank within that
//! `(role, direction)` quadrant (the remaining bits).  This module
//! exposes those decompositions as inherent methods on a
//! [`StreamId`] newtype, replacing the C bit-twiddling helpers.

/// Endpoint role responsible for opening a stream.  Maps to the
/// low bit of a stream id (`0` ↔ client-initiated, `1` ↔
/// server-initiated).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Role {
    Client,
    Server,
}

/// Stream directionality.  Maps to the second-low bit of a stream
/// id (`0` ↔ bidirectional, `1` ↔ unidirectional).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Bidir,
    Unidir,
}

/// QUIC stream identifier (62-bit).  Wraps `u64` as a newtype so
/// the bit-packed layout (RFC 9000 §2.1) is reachable through
/// well-named methods rather than C macros.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamId(pub u64);

impl StreamId {
    /// `true` when this stream id was opened by the client.
    /// C: `IS_CLIENT_STREAM_ID`.
    pub const fn is_client(self) -> bool {
        (self.0 & 1) == 0
    }

    /// `true` when this stream id identifies a bidirectional stream.
    /// C: `IS_BIDIR_STREAM_ID`.
    pub const fn is_bidir(self) -> bool {
        (self.0 & 2) == 0
    }

    /// Whether this stream id was opened locally by an endpoint
    /// playing `local_role`.  C: `IS_LOCAL_STREAM_ID`.
    pub const fn is_local(self, local_role: Role) -> bool {
        let local_bit = match local_role {
            Role::Client => 0,
            Role::Server => 1,
        };
        (self.0 & 1) == local_bit
    }

    /// Build a stream id from its 1-based `rank`, opening `role`,
    /// and direction.  C: `STREAM_ID_FROM_RANK`.
    pub const fn from_parts(rank: u64, role: Role, direction: Direction) -> Self {
        let role_bit = match role {
            Role::Client => 0,
            Role::Server => 1,
        };
        let dir_bit = match direction {
            Direction::Bidir => 0,
            Direction::Unidir => 2,
        };
        Self(((rank - 1) << 2) | dir_bit | role_bit)
    }

    /// Recover the 1-based rank within the `(role, direction)` quadrant.
    /// C: `STREAM_RANK_FROM_ID` — note the off-by-four arithmetic;
    /// preserved here for parity.
    pub const fn rank(self) -> u64 {
        (self.0 + 4) >> 2
    }

    /// Decompose a stream id into its `(direction, role)` quadrant.
    /// C: `STREAM_TYPE_FROM_ID`.
    pub const fn kind(self) -> (Direction, Role) {
        let role = if (self.0 & 1) == 0 {
            Role::Client
        } else {
            Role::Server
        };
        let direction = if (self.0 & 2) == 0 {
            Direction::Bidir
        } else {
            Direction::Unidir
        };
        (direction, role)
    }

    /// Next stream id with the same `(direction, role)` quadrant.
    /// C: `NEXT_STREAM_ID_FOR_TYPE`.
    pub const fn next_with_same_kind(self) -> Self {
        Self(self.0 + 4)
    }
}

#[cfg(test)]
mod test {}
