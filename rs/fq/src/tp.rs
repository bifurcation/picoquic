//! QUIC transport-parameter shapes and helpers.
//!
//! Wire-format identifiers ([`TransportParameter`]), the value
//! structs that travel inside a TP block ([`PreferredAddress`],
//! [`VersionNegotiation`]), and the aggregate the handshake hands
//! to the application ([`TransportParameters`]).

use core::net::SocketAddr;

use crate::{ConnectionId, Duration};

// ---------------------------------------------------------------------------
// Wire identifiers.

/// QUIC transport-parameter identifiers.  Wire values exceed `u32`
/// for several extension parameters, hence `#[repr(u64)]`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u64)]
pub enum TransportParameter {
    OriginalConnectionId = 0,
    IdleTimeout = 1,
    StatelessResetToken = 2,
    MaxPacketSize = 3,
    InitialMaxData = 4,
    InitialMaxStreamDataBidiLocal = 5,
    InitialMaxStreamDataBidiRemote = 6,
    InitialMaxStreamDataUni = 7,
    InitialMaxStreamsBidi = 8,
    InitialMaxStreamsUni = 9,
    AckDelayExponent = 10,
    MaxAckDelay = 11,
    DisableMigration = 12,
    ServerPreferredAddress = 13,
    ActiveConnectionIdLimit = 14,
    HandshakeConnectionId = 15,
    RetryConnectionId = 16,
    /// Per `draft-quic-multipath 20`.
    InitialMaxPathId = 0x3e,
    VersionNegotiation = 0x11,
    /// Per `draft-pauly-quic-datagram-05`.
    MaxDatagramFrameSize = 32,
    TestLargeChello = 3127,
    EnableLossBit = 0x1057,
    /// `(x & 1)` ↔ "want timestamps", `(x & 2)` ↔ "can send timestamps".
    EnableTimeStamp = 0x7158,
    GreaseQuicBit = 0x2ab2,
    /// Per `draft-kuhn-quic-0rtt-bdp-09`.
    EnableBdpFrame = 0xebd9,
    MinAckDelay = 0xff04de1b,
    /// Per `draft-seemann-quic-address-discovery`.
    AddressDiscovery = 0x9f81a176,
    /// Per `draft-ietf-quic-reliable-stream-reset-07`.
    ResetStreamAt = 0x17f7586d2cb571,
}

impl TransportParameter {
    /// Textual name for `tp_number`, taking a raw `u64` so callers
    /// can name unknown / extension parameter IDs too.  C: `tp_name`.
    pub fn name(tp_number: u64) -> Option<&'static str> {
        match tp_number {
            0 => Some("original_connection_id"),
            1 => Some("idle_timeout"),
            2 => Some("stateless_reset_token"),
            3 => Some("max_packet_size"),
            4 => Some("initial_max_data"),
            5 => Some("initial_max_stream_data_bidi_local"),
            6 => Some("initial_max_stream_data_bidi_remote"),
            7 => Some("initial_max_stream_data_uni"),
            8 => Some("initial_max_streams_bidi"),
            9 => Some("initial_max_streams_uni"),
            10 => Some("ack_delay_exponent"),
            11 => Some("max_ack_delay"),
            12 => Some("disable_migration"),
            13 => Some("server_preferred_address"),
            14 => Some("active_connection_id_limit"),
            15 => Some("handshake_connection_id"),
            16 => Some("retry_connection_id"),
            0x11 => Some("version_negotiation"),
            32 => Some("max_datagram_frame_size"),
            3127 => Some("large_chello"),
            0x1057 => Some("enable_loss_bit"),
            0x7158 => Some("enable_time_stamp"),
            0x2ab2 => Some("grease_quic_bit"),
            0xebd9 => Some("enable_bdp_frame"),
            0x3e => Some("initial_max_path_id"),
            0xff04de1b => Some("min_ack_delay"),
            0x9f81a176 => Some("address_discovery"),
            0x17f7586d2cb571 => Some("reset_stream_at"),
            _ => None,
        }
    }
}

/// 0-RTT-remembered transport-parameter slots.  Indexes into the
/// `tp_0rtt: [u64; NB_TP_0RTT]` table on a stored ticket.  C:
/// `Tp0rttKind`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum TransportParameter0RttKind {
    MaxData = 0,
    MaxStreamDataBidiLocal = 1,
    MaxStreamDataBidiRemote = 2,
    MaxStreamDataUni = 3,
    MaxStreamsIdBidir = 4,
    MaxStreamsIdUnidir = 5,
    RttLocal = 6,
    CwinLocal = 7,
    RttRemote = 8,
    CwinRemote = 9,
}

/// C: `nb_tp_0rtt`.  Number of variants in
/// [`TransportParameter0RttKind`].
pub const NB_TP_0RTT: usize = 10;

// ---------------------------------------------------------------------------
// Preferred-address TP value.

/// Server's preferred address advertised in transport parameters.
/// At most one IPv4 and at most one IPv6 endpoint may be present;
/// each socket address (the C `ipvNAddress[]` + `ipvNPort` pair)
/// folds into an `Option<SocketAddr>`.  C: `TpPreferredAddress`.
#[derive(Debug, Default, Copy, Clone)]
pub struct PreferredAddress {
    /// IPv4 address + port, if the server advertised one.
    pub v4: Option<SocketAddr>,
    /// IPv6 address + port, if the server advertised one.
    pub v6: Option<SocketAddr>,
    pub connection_id: ConnectionId,
    pub stateless_reset_token: [u8; 16],
}

// ---------------------------------------------------------------------------
// Version-negotiation TP value.

/// Version-negotiation TP payload.  C: `TpVersionNegotiation`.
/// The C `nb_received` / `nb_supported` length fields disappear
/// (the `Vec`s carry their lengths).
#[derive(Debug, Default, Clone)]
pub struct VersionNegotiation {
    /// Version found in TP, should match envelope.
    pub current: u32,
    /// Version that triggered a previous version negotiation.
    pub previous: u32,
    /// Versions received in a prior VN packet (client side only).
    pub received: Vec<u32>,
    /// Compatible versions supported by the peer (client side only).
    pub supported: Vec<u32>,
}

// ---------------------------------------------------------------------------
// Aggregate.

/// Full set of QUIC transport parameters carried during the
/// handshake.  C: `TransportParameters`.
///
/// `migration_disabled`, `do_grease_quic_bit`, `enable_bdp_frame`,
/// `is_reset_stream_at_enabled` were `unsigned int` Booleans in C;
/// promoted to `bool`.  `enable_loss_bit`, `enable_time_stamp`,
/// `address_discovery_mode` are kept as integers because callers
/// inspect the low bits separately ("want / can" flags).
#[derive(Debug, Clone)]
pub struct TransportParameters {
    pub initial_max_stream_data_bidi_local: u64,
    pub initial_max_stream_data_bidi_remote: u64,
    pub initial_max_stream_data_uni: u64,
    pub initial_max_data: u64,
    pub initial_max_stream_id_bidir: u64,
    pub initial_max_stream_id_unidir: u64,
    pub max_idle_timeout: Duration,
    pub max_packet_size: u32,
    pub max_ack_delay: u32,
    pub active_connection_id_limit: u32,
    pub ack_delay_exponent: u8,
    pub migration_disabled: bool,
    pub preferred_address: PreferredAddress,
    pub max_datagram_frame_size: u32,
    pub enable_loss_bit: i32,
    /// `(x & 1)` want, `(x & 2)` can.
    pub enable_time_stamp: i32,
    pub min_ack_delay: Duration,
    pub do_grease_quic_bit: bool,
    pub version_negotiation: VersionNegotiation,
    pub enable_bdp_frame: bool,
    pub initial_max_path_id: u64,
    /// `0`=none, `1`=provide-only, `2`=receive-only, `3`=both.
    pub address_discovery_mode: i32,
    pub is_reset_stream_at_enabled: bool,
}

impl Default for TransportParameters {
    fn default() -> Self {
        Self {
            initial_max_stream_data_bidi_local: 0,
            initial_max_stream_data_bidi_remote: 0,
            initial_max_stream_data_uni: 0,
            initial_max_data: 0,
            initial_max_stream_id_bidir: 0,
            initial_max_stream_id_unidir: 0,
            max_idle_timeout: Duration::from_ticks(0),
            max_packet_size: 0,
            max_ack_delay: 0,
            active_connection_id_limit: 0,
            ack_delay_exponent: 0,
            migration_disabled: false,
            preferred_address: PreferredAddress::default(),
            max_datagram_frame_size: 0,
            enable_loss_bit: 0,
            enable_time_stamp: 0,
            min_ack_delay: Duration::from_ticks(0),
            do_grease_quic_bit: false,
            version_negotiation: VersionNegotiation::default(),
            enable_bdp_frame: false,
            initial_max_path_id: 0,
            address_discovery_mode: 0,
            is_reset_stream_at_enabled: false,
        }
    }
}

#[cfg(test)]
mod test {}
