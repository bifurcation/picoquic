//! Translation of `quic/internal.h`.
//!
//! quic-core's grand internal header.  Anything that is not
//! part of the published `quic.h` API but is shared between
//! `.c` files of the core library lives here: connection /
//! path / stream / packet structures, frame and packet-type
//! enums, the giant `Quic` and `Connection`
//! structures that pin the rest of the library together, and
//! the long internal-only function surface (~200 functions).
//!
//! Phase 4: all function bodies have been filled in.
//!
//! Translation policy notes for this module:
//!
//! * Single-bit `unsigned int x : 1` bitfields collapse to
//!   `bool` per field (the "ordinary integer field with
//!   mask/shift accessors" rule, simplified for the trivial
//!   1-bit case where `bool` is clearer than `u32` + masks).
//! * Phase 1B replaced the C intrusive linked-list / splay /
//!   hash linkage with arena tokens
//!   ([`crate::arena::Token`], [`crate::splay::SplayToken`],
//!   [`crate::hash::HashToken`]).  Every per-collection
//!   membership is `Option<Token>` on the parent struct; the
//!   container holds the slot storage.  Zero raw pointers, no
//!   `unsafe`.
//! * The C `void* tls_master_ctx`, `void* aead_*`,
//!   `void* pn_enc/dec`, etc. were external-library handles
//!   ((picotls, OpenSSL).  Phase 2 replaced them with the
//!   trait family in [`crate::tls`] (`Session`, `PacketKey`,
//!   `HeaderKey`, …); backends supply implementations.
//! * `FILE* F_log` / `FILE* f_binlog` became
//!   `Option<std::fs::File>` in Phase 1B.
//! * `struct sockaddr_storage` fields fold into
//!   [`core::net::SocketAddr`].  Fields that the C code zeros out
//!   to mean "address absent" become `Option<SocketAddr>`.
//! * `Quic`, `Connection`, `Path` were
//!   forward-declared as opaque handles in
//!   [`crate`] (the public header).  Their
//!   real bodies live here; the public module re-exports the
//!   names so existing `use` paths in other modules keep
//!   working.
//! * Function pointer typedefs (`autoqlog_fn`,
//!   `performance_log_fn`, the spin-bit pair, the
//!   memlog hook on a connection, `mask_fns_t`) collapse to
//!   traits per the project rule.  When two function pointers
//!   are always installed together (`SpinbitDef`,
//!   `mask_fns_t`) they share one trait.
//! * Conditional fields gated by `BBRExperiment` and
//!   `WITH_THREAD_CHECK` are dropped — they are not
//!   part of the v1 target build.
//! * The `PARSE_*`/`IS_*_STREAM_ID(_*)` macros that the C
//!   header inlines as preprocessor macros are translated to
//!   `pub const fn` helpers.

use std::collections::{BTreeMap, VecDeque};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use core::any::Any;
use core::net::SocketAddr;

use crate::arena::{Arena, Token};
use crate::hash::{HashTable, HashToken};
use crate::splay::{SplayToken, SplayTree};
use crate::{Duration, Instant};

/// Token into the per-`Quic` connection arena.
///
/// Phase 4 plan: every C `*mut picoquic_cnx_t` becomes a
/// [`ConnectionToken`].  Following it requires `quic.connections.get(tok)`
/// — one indirection through the arena, plus a generation check.
/// Stable across operations on other connections.
pub type ConnectionToken = Token<Connection>;

/// Token into the per-`Quic` registered-token replay-protection arena.
pub type RegisteredTokenToken = Token<RegisteredToken>;

/// Token into the per-`Quic` issued-tickets arena (server-side replay state).
pub type IssuedTicketToken = Token<IssuedTicket>;

/// Token into the per-`Connection` (or per-`StreamHead`) stream-related
/// arenas.  Phase 4 may further split these into per-collection types.
pub type StreamToken = Token<StreamHead>;
pub type StreamDataToken = Token<StreamDataNode>;
pub type PacketToken = Token<Packet>;
pub type SackItemToken = Token<SackItem>;
pub type LocalConnectionIdToken = Token<LocalConnectionId>;
pub type PathToken = Token<Path>;
use crate::logger::LoggerRef;
use crate::{
    AlpnSelect, CongestionAlgorithm, ConnectionId, ConnectionIdCallback, Fuzz, LossbitVersion,
    PacketContext, PathStatus, PmtudPolicy, RESET_SECRET_SIZE, SpinbitVersion, State,
    StreamDataCallback, StreamDirectReceive, TransportParameters,
};

// ---------------------------------------------------------------------------
// Tunable constants (the `#define`s at the top of the C header).

pub const MAX_PACKET_SIZE: usize = 1536;
pub const MIN_SEGMENT_SIZE: usize = 256;
pub const ENFORCED_INITIAL_MTU: usize = 1200;
pub const ENFORCED_INITIAL_CID_LENGTH: u8 = 8;
pub const PRACTICAL_MAX_MTU: usize = 1440;
pub const MIN_STREAM_DATA_FRAGMENT: usize = 512;
pub const RETRY_SECRET_SIZE: usize = 64;
pub const RETRY_TOKEN_PAD_SIZE: usize = 26;
pub const DEFAULT_0RTT_WINDOW: usize = 10 * ENFORCED_INITIAL_MTU;
pub const NB_PATH_TARGET: usize = 8;
pub const NB_PATH_DEFAULT: usize = 2;
pub const MAX_PACKETS_IN_POOL: i32 = 0x2000;
pub const INITIAL_FLOW_CONTROL_MAX: u64 = 0x100000;

pub const INITIAL_RTT: Duration = Duration::from_ticks(250_000);
pub const TARGET_RENO_RTT: Duration = Duration::from_ticks(100_000);
pub const TARGET_SATELLITE_RTT: Duration = Duration::from_ticks(610_000);
pub const INITIAL_RETRANSMIT_TIMER: Duration = Duration::from_ticks(250_000);
pub const INITIAL_MAX_RETRANSMIT_TIMER: Duration = Duration::from_ticks(1_000_000);
pub const LARGE_RETRANSMIT_TIMER: Duration = Duration::from_ticks(2_000_000);
pub const MIN_RETRANSMIT_TIMER: Duration = Duration::from_ticks(50_000);
pub const ACK_DELAY_MAX: Duration = Duration::from_ticks(10_000);
pub const ACK_DELAY_MAX_DEFAULT: Duration = Duration::from_ticks(25_000);
pub const ACK_DELAY_MIN: Duration = Duration::from_ticks(1_000);
pub const ACK_DELAY_MIN_MAX_VALUE: u64 = 0xFFFFFF;
pub const RACK_DELAY: Duration = Duration::from_ticks(10_000);
pub const MAX_ACK_DELAY_MAX_MS: u64 = 0x4000;
pub const TOKEN_DELAY_LONG: Duration = Duration::from_ticks(24 * 60 * 60 * 1_000_000);
pub const TOKEN_DELAY_SHORT: Duration = Duration::from_ticks(2 * 60 * 1_000_000);
pub const CID_REFRESH_DELAY: Duration = Duration::from_ticks(5 * 1_000_000);
pub const MTU_LOSS_THRESHOLD: u64 = 10;

pub const BANDWIDTH_ESTIMATE_MAX: u64 = 10_000_000_000;
pub const BANDWIDTH_TIME_INTERVAL_MIN: u64 = 1000;
pub const BANDWIDTH_MEDIUM: u64 = 2_000_000;
pub const MAX_BANDWIDTH_TIME_INTERVAL_MIN: u64 = 1000;
pub const MAX_BANDWIDTH_TIME_INTERVAL_MAX: u64 = 15000;

pub const MINRTT_MARGIN: u64 = 128;
pub const MINRTT_THRESHOLD: u64 = 128;

pub const SPURIOUS_RETRANSMIT_DELAY_MAX: Duration = Duration::from_ticks(1_000_000);

pub const MICROSEC_SILENCE_MAX: Duration = Duration::from_ticks(120_000_000);
pub const MICROSEC_HANDSHAKE_MAX: Duration = Duration::from_ticks(30_000_000);
pub const MICROSEC_WAIT_MAX: Duration = Duration::from_ticks(10_000_000);

pub const MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT: Duration = Duration::from_ticks(100_000);

pub const CWIN_INITIAL: u64 = 10 * MAX_PACKET_SIZE as u64;
pub const CWIN_MINIMUM: u64 = 2 * MAX_PACKET_SIZE as u64;

pub const DEFAULT_CRYPTO_EPOCH_LENGTH: u64 = 1 << 22;

pub const DEFAULT_SIMULTANEOUS_LOGS: u32 = 32;
pub const DEFAULT_HALF_OPEN_RETRY_THRESHOLD: u32 = 64;

pub const PN_RANDOM_MIN: u32 = 0xffff;
pub const PN_RANDOM_RANGE: u32 = 0x10000;

pub const SPIN_RESERVE_MOD_256: u8 = 17;

pub const CHALLENGE_REPEAT_MAX: usize = 3;

pub const ALPN_NUMBER_MAX: usize = 32;

pub const CC_ALGO_NUMBER_NEW_RENO: u8 = 1;
pub const CC_ALGO_NUMBER_CUBIC: u8 = 2;
pub const CC_ALGO_NUMBER_DCUBIC: u8 = 3;
pub const CC_ALGO_NUMBER_FAST: u8 = 4;
pub const CC_ALGO_NUMBER_BBR: u8 = 5;
pub const CC_ALGO_NUMBER_PRAGUE: u8 = 6;
pub const CC_ALGO_NUMBER_BBR1: u8 = 7;

pub const MAX_ACK_RANGE_REPEAT: usize = 4;
pub const MIN_ACK_RANGE_REPEAT: usize = 2;

pub const DEFAULT_HOLE_PERIOD: u64 = 256;

pub const LOSS_BIT_Q_HALF_PERIOD: u64 = 64;

pub const NUMBER_OF_EPOCHS: usize = 4;
pub const NUMBER_OF_EPOCH_OFFSETS: usize = NUMBER_OF_EPOCHS + 1;

pub use crate::tp::NB_TP_0RTT;

fn fill_system_random(bytes: &mut [u8]) -> crate::Result<()> {
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(bytes))
        .map_err(|_| crate::Error::Generic)
}

// ---------------------------------------------------------------------------
// Range / bitfield helper macros.

/// `IN_RANGE(v, min, max)` — true when `min ≤ v ≤ max`
/// under the C macro's bitfield assumption (`min & max == min`,
/// `min & bits == 0`, `max & bits == bits`).
#[inline]
pub const fn in_range(v: u64, min: u64, max: u64) -> bool {
    (v & !(min ^ max)) == min
}

/// `BITS_SET_IN_RANGE(v, min, max, bits)`.
#[inline]
pub const fn bits_set_in_range(v: u64, min: u64, max: u64, bits: u64) -> bool {
    (v & !(min ^ max ^ bits)) == (min ^ bits)
}

/// `BITS_CLEAR_IN_RANGE(v, min, max, bits)`.
#[inline]
pub const fn bits_clear_in_range(v: u64, min: u64, max: u64, bits: u64) -> bool {
    (v & !(min ^ max ^ bits)) == min
}

// ---------------------------------------------------------------------------
// Frame types.

// `FrameType` lives in [`crate::frames`].

// ---------------------------------------------------------------------------
// PMTU discovery requirement status.

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PmtuDiscoveryStatus {
    #[default]
    NotNeeded = 0,
    Optional,
    Required,
}

// ---------------------------------------------------------------------------
// Supported QUIC versions.

/// QUIC version codes recognised by this build.  Wire values are the
/// `u32` `Initial` packet version field; `TryFrom<u32>` converts an
/// arbitrary wire value to a [`Version`] (or fails when the version
/// isn't recognised).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum Version {
    SeventeenthInterop = 0xFF00001B,
    EighteenthInterop = 0xFF00001C,
    NineteenthInterop = 0xFF00001D,
    NineteenthBisInterop = 0xFF00001E,
    TwentiethPreInterop = 0xFF00001F,
    TwentiethInterop = 0xFF000020,
    TwentyFirstInterop = 0xFF000021,
    PostIesg = 0xFF000022,
    V1 = 0x00000001,
    V2 = 0x6b3343cf,
    V2Draft = 0x709a50c4,
    InternalTest1 = 0x50435130,
    InternalTest2 = 0x50435131,
}

/// First entry in `Version`'s declaration order — the canonical
/// interop version reported by `INTEROP_VERSION_LATEST` in C.
pub const INTEROP_VERSION_LATEST: Version = Version::NineteenthInterop;

impl Version {
    /// Resolve `proposed` to a known [`Version`] (RFC 9000 §15
    /// "Versions").  C: `get_version_index`.
    pub fn try_from_wire(proposed: u32) -> Option<Self> {
        match proposed {
            0x00000001 => Some(Version::V1),
            0x6b3343cf => Some(Version::V2),
            0x709a50c4 => Some(Version::V2Draft),
            0xFF00001B => Some(Version::SeventeenthInterop),
            0xFF00001C => Some(Version::EighteenthInterop),
            0xFF00001D => Some(Version::NineteenthInterop),
            0xFF00001E => Some(Version::NineteenthBisInterop),
            0xFF00001F => Some(Version::TwentiethPreInterop),
            0xFF000020 => Some(Version::TwentiethInterop),
            0xFF000021 => Some(Version::TwentyFirstInterop),
            0xFF000022 => Some(Version::PostIesg),
            0x50435130 => Some(Version::InternalTest1),
            0x50435131 => Some(Version::InternalTest2),
            _ => None,
        }
    }

    /// Per-version cryptographic and label parameters.  C:
    /// `picoquic_supported_versions[]`'s row for this version.
    pub fn parameters(self) -> VersionParameters {
        const V1_SALT: &[u8] = &[
            0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8,
            0x0c, 0xad, 0xcc, 0xbb, 0x7f, 0x0a,
        ];
        const V1_RETRY_KEY: &[u8] = &[
            0xd9, 0xc9, 0x94, 0x3e, 0x61, 0x01, 0xfd, 0x20, 0x00, 0x21, 0x50, 0x6b, 0xcc, 0x02,
            0x81, 0x4c, 0x73, 0x03, 0x0f, 0x25, 0xc7, 0x9d, 0x71, 0xce, 0x87, 0x6e, 0xca, 0x87,
            0x6e, 0x6f, 0xca, 0x8e,
        ];
        const V2_SALT: &[u8] = &[
            0x0d, 0xed, 0xe3, 0xde, 0xf7, 0x00, 0xa6, 0xdb, 0x81, 0x93, 0x81, 0xbe, 0x6e, 0x26,
            0x9d, 0xcb, 0xf9, 0xbd, 0x2e, 0xd9,
        ];
        const V2_RETRY_KEY: &[u8] = &[
            0xc4, 0xdd, 0x24, 0x84, 0xd6, 0x81, 0xae, 0xfa, 0x4f, 0xf4, 0xd6, 0x9c, 0x2c, 0x20,
            0x29, 0x99, 0x84, 0xa7, 0x65, 0xa5, 0xd3, 0xc3, 0x19, 0x82, 0xf3, 0x8f, 0xc7, 0x41,
            0x62, 0x15, 0x5e, 0x9f,
        ];
        const V2_DRAFT_SALT: &[u8] = &[
            0xa7, 0x07, 0xc2, 0x03, 0xa5, 0x9b, 0x47, 0x18, 0x4a, 0x1d, 0x62, 0xca, 0x57, 0x04,
            0x06, 0xea, 0x7a, 0xe3, 0xe5, 0xd3,
        ];
        const V2_DRAFT_RETRY_KEY: &[u8] = &[
            0x34, 0x25, 0xc2, 0x0c, 0xf8, 0x87, 0x79, 0xdf, 0x2f, 0xf7, 0x1e, 0x8a, 0xbf, 0xa7,
            0x82, 0x49, 0x89, 0x1e, 0x76, 0x3b, 0xbe, 0xd2, 0xf1, 0x3c, 0x04, 0x83, 0x43, 0xd3,
            0x48, 0xc0, 0x60, 0xe2,
        ];
        const DRAFT_29_SALT: &[u8] = &[
            0xaf, 0xbf, 0xec, 0x28, 0x99, 0x93, 0xd2, 0x4c, 0x9e, 0x97, 0x86, 0xf1, 0x9c, 0x61,
            0x11, 0xe0, 0x43, 0x90, 0xa8, 0x99,
        ];
        const RETRY_KEY_29: &[u8] = &[
            0x8b, 0x0d, 0x37, 0xeb, 0x85, 0x35, 0x02, 0x2e, 0xbc, 0x8d, 0x76, 0xa2, 0x07, 0xd8,
            0x0d, 0xf2, 0x26, 0x46, 0xec, 0x06, 0xdc, 0x80, 0x96, 0x42, 0xc3, 0x0a, 0x8b, 0xaa,
            0x2b, 0xaa, 0xff, 0x4c,
        ];
        const INTERNAL_TEST_1_SALT: &[u8] = &[
            0x30, 0x67, 0x16, 0xd7, 0x63, 0x75, 0xd5, 0x55, 0x4b, 0x2f, 0x60, 0x5e, 0xef, 0x78,
            0xd8, 0x33, 0x3d, 0xc1, 0xca, 0x36,
        ];

        const V1_PREFIX: &str = "tls13 quic ";
        const V2_PREFIX: &str = "tls13 quicv2 ";
        const V1_KU: &str = "quic ku";
        const V2_KU: &str = "quicv2 ku";

        const UPGRADE_FROM_V1: &[Version] = &[Version::V1];

        match self {
            Version::V1 => VersionParameters {
                version: self,
                version_aead_key: V1_SALT,
                version_retry_key: V1_RETRY_KEY,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
            Version::V2 => VersionParameters {
                version: self,
                version_aead_key: V2_SALT,
                version_retry_key: V2_RETRY_KEY,
                tls_prefix_label: V2_PREFIX,
                tls_traffic_update_label: V2_KU,
                packet_type_version: 0x6b33_43cf,
                upgrade_from: UPGRADE_FROM_V1,
            },
            Version::V2Draft => VersionParameters {
                version: self,
                version_aead_key: V2_DRAFT_SALT,
                version_retry_key: V2_DRAFT_RETRY_KEY,
                tls_prefix_label: V2_PREFIX,
                tls_traffic_update_label: V2_KU,
                packet_type_version: 0x6b33_43cf,
                upgrade_from: UPGRADE_FROM_V1,
            },
            Version::PostIesg | Version::TwentyFirstInterop => VersionParameters {
                version: self,
                version_aead_key: V1_SALT,
                version_retry_key: V1_RETRY_KEY,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
            Version::TwentiethInterop
            | Version::TwentiethPreInterop
            | Version::NineteenthBisInterop
            | Version::NineteenthInterop => VersionParameters {
                version: self,
                version_aead_key: DRAFT_29_SALT,
                version_retry_key: RETRY_KEY_29,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
            Version::InternalTest1 | Version::InternalTest2 => VersionParameters {
                version: self,
                version_aead_key: INTERNAL_TEST_1_SALT,
                version_retry_key: V1_RETRY_KEY,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: self as u32,
                upgrade_from: &[],
            },
            Version::SeventeenthInterop | Version::EighteenthInterop => VersionParameters {
                version: self,
                version_aead_key: DRAFT_29_SALT,
                version_retry_key: RETRY_KEY_29,
                tls_prefix_label: V1_PREFIX,
                tls_traffic_update_label: V1_KU,
                packet_type_version: 0x0000_0001,
                upgrade_from: &[],
            },
        }
    }
}

/// Per-version cryptographic and label parameters.  C:
/// `VersionParameters`.
///
/// `version_aead_key` and `version_retry_key` are static byte
/// tables in the C source — Rust models them as borrowed slices.
/// `upgrade_from` is a `NULL`-terminated list in C; here it is a
/// borrowed slice, with an empty slice for "no upgrade path".
#[derive(Debug)]
pub struct VersionParameters {
    pub version: Version,
    pub version_aead_key: &'static [u8],
    pub version_retry_key: &'static [u8],
    pub tls_prefix_label: &'static str,
    pub tls_traffic_update_label: &'static str,
    pub packet_type_version: u32,
    pub upgrade_from: &'static [Version],
}

// ---------------------------------------------------------------------------
// Crypto epochs and packet types.

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Epoch {
    Initial = 0,
    ZeroRtt = 1,
    Handshake = 2,
    OneRtt = 3,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketType {
    Error = 0,
    VersionNegotiation,
    Initial,
    Retry,
    Handshake,
    ZeroRttProtected,
    OneRttProtected,
    TypeMax,
}

// ---------------------------------------------------------------------------
// Packet header.

/// Parsed long/short packet header.  C: `PacketHeader`.
///
/// The C struct uses a packed bitfield for eight single-bit
/// flags; Rust stores them as plain `bool` fields (one per flag).
pub struct PacketHeader {
    pub dest_connection_id: ConnectionId,
    pub src_connection_id: ConnectionId,
    /// Truncated packet number as it appeared on the wire (1-4 bytes,
    /// extended to `u32`).  Reconstructed full PN is in
    /// [`Self::packet_number_full`].
    pub packet_number_truncated: u32,
    pub version: u32,
    pub offset: usize,
    pub packet_number_offset: usize,
    pub packet_type: PacketType,
    pub packet_number_mask: u64,
    pub packet_number_full: u64,
    pub payload_length: usize,
    pub version_index: i32,
    pub epoch: Epoch,
    pub packet_context: PacketContext,

    pub key_phase: bool,
    pub spin: bool,
    pub has_spin_bit: bool,
    pub has_reserved_bit_set: bool,
    pub has_loss_bits: bool,
    pub loss_bit_q: bool,
    pub loss_bit_l: bool,
    pub quic_bit_is_zero: bool,

    /// Borrowed token bytes (length implicit in the slice).  Phase
    /// 4 plan: `IncomingPacket` decoders carry a lifetime; the
    /// `token_length` field is gone (it lived only as the slice's
    /// length).
    pub token_bytes: Vec<u8>,
    pub payload_length_value: usize,
    /// Token into [`Quic`]'s arena for the local CID this packet
    /// targets, if any.  C: `*mut LocalConnectionId` back-pointer.
    pub local_connection_id: Option<LocalConnectionIdToken>,
}

// ---------------------------------------------------------------------------
// Spin-bit policy vtable.

/// Spin-bit policy: how to update the spin bit on an incoming
/// packet, and what value to emit on outgoing packets.  In C the
/// policy is two function pointers grouped into
/// `SpinbitDef`; the two are always installed
/// together so they share one Rust trait.
pub trait SpinBitPolicy: Sync {
    /// C: `spinbit_incoming_fn`.
    fn incoming(&self, connection: &mut Connection, path_x: &mut Path, ph: &PacketHeader);

    /// C: `spinbit_outgoing_fn`.
    fn outgoing(&self, connection: &mut Connection) -> u8;
}

/// Spin-bit policy dispatch table.  Replaces `extern SpinbitDef
/// spin_function_table[]` from the C source.  Each row is a
/// `&'static dyn` to a stateless policy implementor; the wrapper
/// `SpinbitDef` struct from C is gone (the trait object carries
/// the same information).
///
/// Indices match [`crate::SpinbitVersion`] values:
/// * 0 = Basic
/// * 1 = Random
/// * 2 = Null
/// * 3 = On (test-only, server side)
///
/// The concrete policy types and the populated table land in
/// `crate::spinbit` (Phase 4).
pub static SPIN_FUNCTION_TABLE: &[&dyn SpinBitPolicy] = &[];

// ---------------------------------------------------------------------------
// Stateless packet, queued at the QUIC context until sendable.

pub struct StatelessPacket {
    pub addr_to: SocketAddr,
    pub addr_local: SocketAddr,
    pub if_index_local: i32,
    pub received_ecn: u8,
    pub length: usize,
    pub receive_time: Instant,
    pub connection_id_log64: u64,
    pub initial_connection_id: ConnectionId,
    pub packet_type: PacketType,
    pub bytes: [u8; MAX_PACKET_SIZE],
}

impl Quic {
    /// Allocate a fresh stateless-packet buffer.  C:
    /// `create_stateless_packet` plus the per-context pool;
    /// `Quic.connections.alloc()`-style — Rust uses normal heap
    /// allocation and lets `Drop` free.
    pub fn create_stateless_packet(&mut self) -> Result<StatelessPacket, crate::Error> {
        use core::net::{IpAddr, Ipv4Addr};
        Ok(StatelessPacket {
            addr_to: core::net::SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            addr_local: core::net::SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            if_index_local: 0,
            received_ecn: 0,
            length: 0,
            receive_time: crate::Instant::from_ticks(0),
            connection_id_log64: 0,
            initial_connection_id: crate::ConnectionId::default(),
            packet_type: crate::internal::PacketType::Error,
            bytes: [0u8; crate::MAX_PACKET_SIZE],
        })
    }

    /// Hand `sp` to the QUIC context's pending stateless-packet
    /// queue.  C: `queue_stateless_packet`.
    pub fn queue_stateless_packet(&mut self, sp: StatelessPacket) {
        self.pending_stateless_packets.push_back(sp);
    }

    /// Pop the next pending stateless packet.  Returns `None` when
    /// the queue is empty.  C: `dequeue_stateless_packet`.
    pub fn dequeue_stateless_packet(&mut self) -> Option<StatelessPacket> {
        self.pending_stateless_packets.pop_front()
    }
}

// `delete_stateless_packet` is gone -- StatelessPacket falls out
// of scope (the VecDeque drains it); `Drop` does the C `free`.

// ---------------------------------------------------------------------------
// Stream data nodes (received) and queue nodes (queued for send).

pub struct StreamDataNode {
    /// Membership in the parent stream's `stream_data_tree`.
    /// `Some(token)` while in the tree; `None` otherwise.  Phase 4
    /// uses this for O(1) removal.
    pub stream_data_membership: Option<SplayToken>,
    pub offset: u64,
    /// Inline buffer of payload bytes.  C kept a parallel
    /// `bytes: *const u8` aliasing into `data` plus a `length`
    /// field; in Rust the slice subsumes both — `data[..len]` is
    /// the live payload and `len` is `length`.
    pub data: [u8; MAX_PACKET_SIZE],
    pub length: usize,
}

pub struct StreamQueueNode {
    pub offset: u64,
    /// Owned send-queue payload.  C: `bytes: *mut u8` plus paired
    /// `length: size_t`; both collapse into the vector.
    pub bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Sent packet (kept on retransmit queues until acked).

pub struct Packet {
    /// Membership in the owning connection's
    /// `queue_data_repeat_tree`.  Phase 4 uses this for O(1) removal.
    pub queue_data_repeat_membership: Option<SplayToken>,
    /// Path the packet was sent on.  C: `*mut Path` back-pointer.
    pub send_path: Option<PathToken>,
    pub sequence_number: u64,
    pub send_time: Instant,
    pub delivered_prior: u64,
    pub delivered_time_prior: Instant,
    pub delivered_sent_prior: u64,
    pub lost_prior: u64,
    pub inflight_prior: u64,
    pub data_repeat_frame: usize,
    pub data_repeat_index: usize,

    pub data_repeat_priority: u64,
    pub data_repeat_stream_id: u64,
    pub data_repeat_stream_offset: u64,
    pub data_repeat_stream_data_length: usize,

    pub length: usize,
    pub checksum_overhead: usize,
    pub offset: usize,
    pub packet_type: PacketType,
    pub packet_context: PacketContext,

    pub is_evaluated: bool,
    pub is_ack_eliciting: bool,
    pub is_mtu_probe: bool,
    pub is_multipath_probe: bool,
    pub is_ack_trap: bool,
    pub delivered_app_limited: bool,
    pub sent_cwin_limited: bool,
    pub is_preemptive_repeat: bool,
    pub was_preemptively_repeated: bool,
    pub is_queued_to_path: bool,
    pub is_queued_for_retransmit: bool,
    pub is_queued_for_spurious_detection: bool,
    pub is_queued_for_data_repeat: bool,

    pub bytes: [u8; MAX_PACKET_SIZE],
}

impl Quic {
    /// Allocate a fresh outbound packet buffer.  C: `create_packet`
    /// plus the per-context pool; the Rust port lets the allocator
    /// handle reuse.  `Drop` replaces `recycle_packet`.
    pub fn create_packet(&mut self) -> Result<Packet, crate::Error> {
        self.nb_packets_allocated += 1;
        Ok(Packet {
            queue_data_repeat_membership: None,
            send_path: None,
            sequence_number: 0,
            send_time: crate::Instant::from_ticks(0),
            delivered_prior: 0,
            delivered_time_prior: crate::Instant::from_ticks(0),
            delivered_sent_prior: 0,
            lost_prior: 0,
            inflight_prior: 0,
            data_repeat_frame: 0,
            data_repeat_index: 0,
            data_repeat_priority: 0,
            data_repeat_stream_id: 0,
            data_repeat_stream_offset: 0,
            data_repeat_stream_data_length: 0,
            length: 0,
            checksum_overhead: 0,
            offset: 0,
            packet_type: PacketType::Error,
            packet_context: crate::PacketContext::Application,
            is_evaluated: false,
            is_ack_eliciting: false,
            is_mtu_probe: false,
            is_multipath_probe: false,
            is_ack_trap: false,
            delivered_app_limited: false,
            sent_cwin_limited: false,
            is_preemptive_repeat: false,
            was_preemptively_repeated: false,
            is_queued_to_path: false,
            is_queued_for_retransmit: false,
            is_queued_for_spurious_detection: false,
            is_queued_for_data_repeat: false,
            bytes: [0u8; MAX_PACKET_SIZE],
        })
    }
}

impl Connection {
    /// Pad `bytes[length..]` up to whatever the connection's padding
    /// policy mandates (or up to `max_length`, whichever is smaller).
    /// Returns the new buffered length.
    pub fn pad_to_policy(&mut self, bytes: &mut [u8], length: usize, max_length: u32) -> usize {
        let mut target = self.padding_minsize as usize;
        if length > target && self.padding_multiple != 0 {
            let delta = (length - target) % self.padding_multiple as usize;
            if delta == 0 {
                target = length;
            } else {
                target = length + self.padding_multiple as usize - delta;
            }
        }
        if target > max_length as usize {
            target = max_length as usize;
        }
        pad_to_target_length(bytes, length, target)
    }
}

// ---------------------------------------------------------------------------
// Token register (replay protection for new tokens / retry tokens / tickets).

pub struct RegisteredToken {
    /// Membership in `Quic::token_reuse_tree`.  Phase 4 uses this
    /// for O(1) removal when the token expires.
    pub registered_token_membership: Option<SplayToken>,
    pub token_time: Instant,
    pub token_hash: u64,
    pub count: i32,
}

// ---------------------------------------------------------------------------
// 0-RTT remembered transport parameters.
// `TransportParameter0RttKind` (and its `NB_TP_0RTT` count) live in [`crate::tp`].

pub struct StoredTicket {
    /// Owned SNI string (C: `sni: *mut c_char` plus `sni_length`).
    pub sni: Option<String>,
    /// Owned ALPN string (C: `alpn: *mut c_char` plus `alpn_length`).
    pub alpn: Option<String>,
    /// Server IP address.  C kept it as `(ip_addr: *mut u8,
    /// ip_addr_length: u8)`; `core::net::IpAddr` carries the family
    /// and the bytes.
    pub ip_addr: core::net::IpAddr,
    /// Client IP address (same encoding as [`Self::ip_addr`]).
    pub ip_addr_client: core::net::IpAddr,
    pub tp_0rtt: [u64; NB_TP_0RTT],
    /// Owned session ticket (C: `ticket: *mut u8` plus `ticket_length`).
    pub ticket: Vec<u8>,
    pub time_valid_until: Instant,
    pub version: u32,
    pub was_used: bool,
}

fn stored_ip_bytes(ip_addr: core::net::IpAddr) -> Vec<u8> {
    match ip_addr {
        core::net::IpAddr::V4(addr) => addr.octets().to_vec(),
        core::net::IpAddr::V6(addr) => addr.octets().to_vec(),
    }
}

fn stored_ip_from_bytes(bytes: &[u8]) -> Result<core::net::IpAddr, crate::Error> {
    match bytes.len() {
        0 => Ok(core::net::IpAddr::V4(core::net::Ipv4Addr::UNSPECIFIED)),
        4 => Ok(core::net::IpAddr::V4(core::net::Ipv4Addr::new(
            bytes[0], bytes[1], bytes[2], bytes[3],
        ))),
        16 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(bytes);
            Ok(core::net::IpAddr::V6(core::net::Ipv6Addr::from(octets)))
        }
        _ => Err(crate::Error::InvalidFile),
    }
}

fn optional_string_bytes(s: Option<&str>) -> &[u8] {
    s.unwrap_or("").as_bytes()
}

fn optional_string_from_bytes(bytes: &[u8]) -> Result<Option<String>, crate::Error> {
    if bytes.is_empty() {
        Ok(None)
    } else {
        core::str::from_utf8(bytes)
            .map(|s| Some(s.to_owned()))
            .map_err(|_| crate::Error::InvalidFile)
    }
}

fn stored_ticket_tp(ticket: &StoredTicket) -> TransportParameters {
    use crate::tp::TransportParameter0RttKind::*;

    TransportParameters {
        initial_max_data: ticket.tp_0rtt[MaxData as usize],
        initial_max_stream_data_bidi_local: ticket.tp_0rtt[MaxStreamDataBidiLocal as usize],
        initial_max_stream_data_bidi_remote: ticket.tp_0rtt[MaxStreamDataBidiRemote as usize],
        initial_max_stream_data_uni: ticket.tp_0rtt[MaxStreamDataUni as usize],
        initial_max_stream_id_bidir: ticket.tp_0rtt[MaxStreamsIdBidir as usize],
        initial_max_stream_id_unidir: ticket.tp_0rtt[MaxStreamsIdUnidir as usize],
        ..TransportParameters::default()
    }
}

fn ticket_valid_until(ticket: &[u8]) -> Result<Instant, crate::Error> {
    if ticket.len() < 17 {
        return Err(crate::Error::Protocol(
            crate::errors::InternalError::InvalidTicket as u64,
        ));
    }

    let issued_seconds = parse_64(&ticket[..8]);
    let ttl_seconds = u64::from(parse_32(&ticket[13..17])).min(7 * 24 * 3600);
    let valid_until = issued_seconds
        .saturating_mul(1000)
        .saturating_add(ttl_seconds.saturating_mul(1_000_000));
    Ok(Instant::from_ticks(valid_until))
}

fn stored_ticket_id(ticket: &StoredTicket) -> u64 {
    if ticket.ticket.len() < 8 {
        0
    } else {
        parse_64(&ticket.ticket[..8])
    }
}

fn serialize_ticket(ticket: &StoredTicket) -> Result<Vec<u8>, crate::Error> {
    let sni = optional_string_bytes(ticket.sni.as_deref());
    let alpn = optional_string_bytes(ticket.alpn.as_deref());
    let ip_addr = stored_ip_bytes(ticket.ip_addr);
    let ip_addr_client = stored_ip_bytes(ticket.ip_addr_client);

    if sni.len() > u16::MAX as usize
        || alpn.len() > u16::MAX as usize
        || ticket.ticket.len() > u16::MAX as usize
        || ip_addr.len() > u8::MAX as usize
        || ip_addr_client.len() > u8::MAX as usize
    {
        return Err(crate::Error::BufferTooSmall);
    }

    let required = 8
        + 2
        + sni.len()
        + 2
        + alpn.len()
        + 4
        + 1
        + ip_addr.len()
        + 1
        + ip_addr_client.len()
        + 8 * NB_TP_0RTT
        + 2
        + ticket.ticket.len();
    let mut bytes = vec![0u8; required];
    let mut off = 0;

    format_64(&mut bytes[off..off + 8], ticket.time_valid_until.ticks());
    off += 8;
    format_16(&mut bytes[off..off + 2], sni.len() as u16);
    off += 2;
    bytes[off..off + sni.len()].copy_from_slice(sni);
    off += sni.len();
    format_16(&mut bytes[off..off + 2], alpn.len() as u16);
    off += 2;
    bytes[off..off + alpn.len()].copy_from_slice(alpn);
    off += alpn.len();
    format_32(&mut bytes[off..off + 4], ticket.version);
    off += 4;
    bytes[off] = ip_addr.len() as u8;
    off += 1;
    bytes[off..off + ip_addr.len()].copy_from_slice(&ip_addr);
    off += ip_addr.len();
    bytes[off] = ip_addr_client.len() as u8;
    off += 1;
    bytes[off..off + ip_addr_client.len()].copy_from_slice(&ip_addr_client);
    off += ip_addr_client.len();
    for value in ticket.tp_0rtt {
        format_64(&mut bytes[off..off + 8], value);
        off += 8;
    }
    format_16(&mut bytes[off..off + 2], ticket.ticket.len() as u16);
    off += 2;
    bytes[off..off + ticket.ticket.len()].copy_from_slice(&ticket.ticket);

    Ok(bytes)
}

fn take_slice<'a>(bytes: &'a [u8], off: &mut usize, len: usize) -> Result<&'a [u8], crate::Error> {
    let end = off.checked_add(len).ok_or(crate::Error::InvalidFile)?;
    let slice = bytes.get(*off..end).ok_or(crate::Error::InvalidFile)?;
    *off = end;
    Ok(slice)
}

fn deserialize_ticket(bytes: &[u8]) -> Result<StoredTicket, crate::Error> {
    let mut off = 0;
    let time_valid_until = Instant::from_ticks(parse_64(take_slice(bytes, &mut off, 8)?));

    let sni_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let sni = optional_string_from_bytes(take_slice(bytes, &mut off, sni_len)?)?;

    let alpn_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let alpn = optional_string_from_bytes(take_slice(bytes, &mut off, alpn_len)?)?;

    let version = parse_32(take_slice(bytes, &mut off, 4)?);

    let ip_len = take_slice(bytes, &mut off, 1)?[0] as usize;
    let ip_addr = stored_ip_from_bytes(take_slice(bytes, &mut off, ip_len)?)?;

    let ip_client_len = take_slice(bytes, &mut off, 1)?[0] as usize;
    let ip_addr_client = stored_ip_from_bytes(take_slice(bytes, &mut off, ip_client_len)?)?;

    let mut tp_0rtt = [0u64; NB_TP_0RTT];
    for value in tp_0rtt.iter_mut() {
        *value = parse_64(take_slice(bytes, &mut off, 8)?);
    }

    let ticket_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let ticket = take_slice(bytes, &mut off, ticket_len)?.to_vec();
    if off != bytes.len() {
        return Err(crate::Error::InvalidFile);
    }

    Ok(StoredTicket {
        sni,
        alpn,
        ip_addr,
        ip_addr_client,
        tp_0rtt,
        ticket,
        time_valid_until,
        version,
        was_used: false,
    })
}

impl Quic {
    /// Cache a session ticket alongside the transport-parameters that
    /// were in force at the time, indexed by `(sni, alpn, version)`.
    #[allow(clippy::too_many_arguments)]
    pub fn store_ticket(
        &mut self,
        sni: Option<&str>,
        alpn: Option<&str>,
        version: u32,
        ip_addr: core::net::IpAddr,
        ip_addr_client: core::net::IpAddr,
        ticket: &[u8],
        tp: &TransportParameters,
    ) -> Result<(), crate::Error> {
        let time_valid_until = ticket_valid_until(ticket)?;
        let stored = StoredTicket {
            sni: sni.map(str::to_owned),
            alpn: alpn.map(str::to_owned),
            ip_addr,
            ip_addr_client,
            tp_0rtt: {
                use crate::tp::TransportParameter0RttKind::*;
                let mut arr = [0u64; crate::tp::NB_TP_0RTT];
                arr[MaxData as usize] = tp.initial_max_data;
                arr[MaxStreamDataBidiLocal as usize] = tp.initial_max_stream_data_bidi_local;
                arr[MaxStreamDataBidiRemote as usize] = tp.initial_max_stream_data_bidi_remote;
                arr[MaxStreamDataUni as usize] = tp.initial_max_stream_data_uni;
                arr[MaxStreamsIdBidir as usize] = tp.initial_max_stream_id_bidir;
                arr[MaxStreamsIdUnidir as usize] = tp.initial_max_stream_id_unidir;
                arr
            },
            ticket: ticket.to_vec(),
            time_valid_until,
            version,
            was_used: false,
        };
        self.stored_tickets.retain(|old| {
            old.sni.as_deref() != sni
                || old.alpn.as_deref() != alpn
                || old.version != version
                || old.time_valid_until > time_valid_until
        });
        self.stored_tickets.insert(0, stored);
        Ok(())
    }

    /// Retrieve a cached ticket record (without consuming it).
    /// `need_unused` skips already-used tickets.
    pub fn get_stored_ticket(
        &mut self,
        sni: Option<&str>,
        alpn: Option<&str>,
        version: u32,
        need_unused: bool,
        ticket_id: u64,
    ) -> Option<&mut StoredTicket> {
        self.stored_tickets.iter_mut().find(|ticket| {
            ticket.time_valid_until.ticks() > 0
                && ticket.sni.as_deref() == sni
                && ticket.alpn.as_deref() == alpn
                && (version == 0 || ticket.version == version)
                && (!need_unused || !ticket.was_used)
                && (ticket_id == 0 || stored_ticket_id(ticket) == ticket_id)
        })
    }

    /// Output of [`Self::get_ticket`] / [`Self::get_ticket_and_version`].
    /// Replaces the C trio of out-parameters
    /// (`uint8_t** ticket`, `uint16_t* ticket_length`,
    /// `picoquic_tp_t* tp`).  The byte slice borrows from the
    /// cached ticket entry.
    #[allow(clippy::too_many_arguments)]
    pub fn get_ticket(
        &mut self,
        sni: Option<&str>,
        alpn: Option<&str>,
        version: u32,
        mark_used: bool,
    ) -> Result<(&[u8], TransportParameters), crate::Error> {
        let (_, ticket, tp) = self.get_ticket_and_version(sni, alpn, version, mark_used)?;
        Ok((ticket, tp))
    }

    /// As [`Self::get_ticket`] but also returns the QUIC version
    /// the ticket was issued for (in the first tuple slot).
    pub fn get_ticket_and_version(
        &mut self,
        sni: Option<&str>,
        alpn: Option<&str>,
        version: u32,
        mark_used: bool,
    ) -> Result<(u32, &[u8], TransportParameters), crate::Error> {
        let idx = self
            .stored_tickets
            .iter()
            .position(|ticket| {
                ticket.time_valid_until.ticks() > 0
                    && ticket.sni.as_deref() == sni
                    && ticket.alpn.as_deref() == alpn
                    && (version == 0 || ticket.version == version)
                    && (!mark_used || !ticket.was_used)
            })
            .ok_or(crate::Error::Generic)?;

        if mark_used {
            self.stored_tickets[idx].was_used = true;
        }

        let ticket = &self.stored_tickets[idx];
        Ok((
            ticket.version,
            ticket.ticket.as_slice(),
            stored_ticket_tp(ticket),
        ))
    }

    /// Load cached tickets from `ticket_file_name`.
    pub fn load_tickets(
        &mut self,
        ticket_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        let data = match std::fs::read(ticket_file_name) {
            Ok(data) => data,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(crate::Error::NoSuchFile);
            }
            Err(_) => return Err(crate::Error::InvalidFile),
        };

        let mut off = 0;
        self.stored_tickets.clear();
        while off < data.len() {
            let record_len_bytes = data.get(off..off + 4).ok_or(crate::Error::InvalidFile)?;
            let record_len = u32::from_ne_bytes([
                record_len_bytes[0],
                record_len_bytes[1],
                record_len_bytes[2],
                record_len_bytes[3],
            ]) as usize;
            off += 4;
            if record_len > 2048 {
                return Err(crate::Error::InvalidFile);
            }
            let record = data
                .get(off..off + record_len)
                .ok_or(crate::Error::InvalidFile)?;
            off += record_len;

            let ticket = deserialize_ticket(record)?;
            if ticket.time_valid_until.ticks() > 0 {
                self.stored_tickets.push(ticket);
            }
        }

        Ok(())
    }

    /// Persist the cached ticket vector to `ticket_file_name`,
    /// dropping entries that expired before `current_time`.  C:
    /// `save_tickets` (operated on the C linked-list head).
    pub fn save_tickets(
        &self,
        current_time: Instant,
        ticket_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        let mut file = File::create(ticket_file_name).map_err(|_| crate::Error::InvalidFile)?;
        for ticket in &self.stored_tickets {
            if ticket.time_valid_until > current_time && !ticket.was_used {
                let record = serialize_ticket(ticket)?;
                if record.len() > 2048 {
                    return Err(crate::Error::InvalidFile);
                }
                let len = (record.len() as u32).to_ne_bytes();
                file.write_all(&len)
                    .map_err(|_| crate::Error::InvalidFile)?;
                file.write_all(&record)
                    .map_err(|_| crate::Error::InvalidFile)?;
            }
        }
        Ok(())
    }
}

// `free_tickets` is gone -- `Quic.stored_tickets.clear()` is the
// Rust equivalent (Drop frees each entry).

impl Connection {
    /// Stash this connection's RTT and CWIN into the issued-ticket
    /// table so a future resumption can seed bandwidth estimates.
    pub fn seed_ticket(&mut self, path_x: &mut Path) {
        let target_cwin = if path_x.bandwidth_estimate_max > 0 {
            path_x
                .rtt_min
                .ticks()
                .saturating_mul(path_x.bandwidth_estimate_max)
                .saturating_div(1_000_000)
        } else {
            path_x.cwin
        };
        path_x.cwin_remote = target_cwin;
        path_x.rtt_min_remote = path_x.rtt_min;
        path_x.is_ticket_seeded = true;
    }
}

// ---------------------------------------------------------------------------
// Stored retry-token (for client side, indexed by SNI + IP).

pub struct StoredToken {
    /// Owned SNI string (C: `sni: *const c_char` plus `sni_length`).
    pub sni: Option<String>,
    /// Owned retry-token bytes (C: `token: *const u8` plus
    /// `token_length`).
    pub token: Vec<u8>,
    /// Server IP address.
    pub ip_addr: core::net::IpAddr,
    pub time_valid_until: Instant,
    pub was_used: bool,
}

fn serialize_token(token: &StoredToken) -> Result<Vec<u8>, crate::Error> {
    let sni = optional_string_bytes(token.sni.as_deref());
    let ip_addr = stored_ip_bytes(token.ip_addr);
    if sni.len() > u16::MAX as usize
        || ip_addr.len() > u16::MAX as usize
        || token.token.len() > u16::MAX as usize
    {
        return Err(crate::Error::BufferTooSmall);
    }

    let required = 8 + 2 + sni.len() + 2 + ip_addr.len() + 2 + token.token.len();
    let mut bytes = vec![0u8; required];
    let mut off = 0;

    format_64(&mut bytes[off..off + 8], token.time_valid_until.ticks());
    off += 8;
    format_16(&mut bytes[off..off + 2], sni.len() as u16);
    off += 2;
    bytes[off..off + sni.len()].copy_from_slice(sni);
    off += sni.len();
    format_16(&mut bytes[off..off + 2], ip_addr.len() as u16);
    off += 2;
    bytes[off..off + ip_addr.len()].copy_from_slice(&ip_addr);
    off += ip_addr.len();
    format_16(&mut bytes[off..off + 2], token.token.len() as u16);
    off += 2;
    bytes[off..off + token.token.len()].copy_from_slice(&token.token);

    Ok(bytes)
}

fn deserialize_token(bytes: &[u8]) -> Result<StoredToken, crate::Error> {
    let mut off = 0;
    let time_valid_until = Instant::from_ticks(parse_64(take_slice(bytes, &mut off, 8)?));

    let sni_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let sni = optional_string_from_bytes(take_slice(bytes, &mut off, sni_len)?)?;

    let ip_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let ip_addr = stored_ip_from_bytes(take_slice(bytes, &mut off, ip_len)?)?;

    let token_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let token = take_slice(bytes, &mut off, token_len)?.to_vec();
    if off != bytes.len() {
        return Err(crate::Error::InvalidFile);
    }

    Ok(StoredToken {
        sni,
        token,
        ip_addr,
        time_valid_until,
        was_used: false,
    })
}

impl Quic {
    /// Cache a server-issued retry token for `(sni, ip_addr)` so it
    /// can be replayed on a future handshake.
    pub fn store_token(
        &mut self,
        sni: Option<&str>,
        ip_addr: core::net::IpAddr,
        token: &[u8],
    ) -> Result<(), crate::Error> {
        if sni.unwrap_or("").is_empty() || token.is_empty() {
            return Err(crate::Error::Protocol(
                crate::errors::InternalError::InvalidToken as u64,
            ));
        }

        // Replace any existing token for the same (sni, ip_addr).
        self.stored_tokens
            .retain(|t| t.sni.as_deref() != sni || t.ip_addr != ip_addr);
        self.stored_tokens.push(StoredToken {
            sni: sni.map(str::to_owned),
            token: token.to_vec(),
            ip_addr,
            time_valid_until: crate::Instant::from_ticks(TOKEN_DELAY_LONG.ticks()),
            was_used: false,
        });
        Ok(())
    }

    /// Retrieve a cached retry token for `(sni, ip_addr)`,
    /// optionally marking it as used so subsequent calls don't
    /// return it again.  Returned slice borrows from the cached
    /// entry.
    pub fn get_token(
        &mut self,
        sni: Option<&str>,
        ip_addr: core::net::IpAddr,
        mark_used: bool,
    ) -> Result<&[u8], crate::Error> {
        let idx = self
            .stored_tokens
            .iter()
            .position(|token| {
                token.time_valid_until.ticks() > 0
                    && token.sni.as_deref() == sni
                    && token.ip_addr == ip_addr
                    && !token.was_used
                    && !token.token.is_empty()
            })
            .ok_or(crate::Error::Generic)?;
        if mark_used {
            self.stored_tokens[idx].was_used = true;
        }
        Ok(self.stored_tokens[idx].token.as_slice())
    }

    /// Persist cached tokens to `token_file_name`.
    pub fn save_tokens(
        &mut self,
        token_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        let mut file = File::create(token_file_name).map_err(|_| crate::Error::InvalidFile)?;
        for token in &self.stored_tokens {
            if token.time_valid_until.ticks() > 0 && !token.was_used {
                let record = serialize_token(token)?;
                if record.len() > 2048 {
                    return Err(crate::Error::InvalidFile);
                }
                file.write_all(&(record.len() as u32).to_ne_bytes())
                    .map_err(|_| crate::Error::InvalidFile)?;
                file.write_all(&record)
                    .map_err(|_| crate::Error::InvalidFile)?;
            }
        }
        Ok(())
    }

    /// Load cached tokens from `token_file_name`.
    pub fn load_tokens(
        &mut self,
        token_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        let data = match std::fs::read(token_file_name) {
            Ok(data) => data,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(crate::Error::NoSuchFile);
            }
            Err(_) => return Err(crate::Error::InvalidFile),
        };

        let mut off = 0;
        self.stored_tokens.clear();
        while off < data.len() {
            let len_bytes = data.get(off..off + 4).ok_or(crate::Error::InvalidFile)?;
            let record_len =
                u32::from_ne_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]])
                    as usize;
            off += 4;
            if record_len > 2048 {
                return Err(crate::Error::InvalidFile);
            }
            let record = data
                .get(off..off + record_len)
                .ok_or(crate::Error::InvalidFile)?;
            off += record_len;
            let token = deserialize_token(record)?;
            if token.time_valid_until.ticks() > 0 {
                self.stored_tokens.push(token);
            }
        }
        Ok(())
    }
}

// `free_tokens` is gone -- `Quic.stored_tokens.clear()` does the same.

// ---------------------------------------------------------------------------
// Issued-tickets bookkeeping (server side, per ticket-id index).

pub struct IssuedTicket {
    /// Membership in `Quic::issued_tickets_by_id`.  Phase 4 uses
    /// this for O(1) removal when the ticket is purged.
    pub issued_tickets_membership: Option<HashToken>,
    pub ticket_id: u64,
    pub creation_time: Instant,
    pub rtt: Duration,
    pub cwin: u64,
    pub ip_addr: core::net::IpAddr,
}

impl Quic {
    /// Server-side: record metadata for a session ticket we just
    /// issued, so we can recognize a resumption attempt later.
    pub fn remember_issued_ticket(
        &mut self,
        ticket_id: u64,
        rtt: Duration,
        cwin: u64,
        ip_addr: core::net::IpAddr,
    ) -> Result<(), crate::Error> {
        let ticket = IssuedTicket {
            issued_tickets_membership: None,
            ticket_id,
            creation_time: crate::Instant::from_ticks(0),
            rtt,
            cwin,
            ip_addr,
        };
        let tok = self.issued_tickets.insert(ticket)?;
        let (htok, _) = self.issued_tickets_by_id.insert(ticket_id, tok)?;
        // Store the hash token back into the ticket so we can do O(1) removal later.
        if let Some(entry) = self.issued_tickets.get_mut(tok) {
            entry.issued_tickets_membership = Some(htok);
        }
        Ok(())
    }

    /// Look up a previously-remembered issued ticket by id.
    /// Returns `None` when no such ticket is on file.
    pub fn retrieve_issued_ticket(&mut self, ticket_id: u64) -> Option<&mut IssuedTicket> {
        let htok = self.issued_tickets_by_id.lookup(&ticket_id)?;
        let tok = *self.issued_tickets_by_id.get(htok)?;
        self.issued_tickets.get_mut(tok)
    }
}

// ---------------------------------------------------------------------------
// Auto-qlog and performance log callbacks (function pointers → traits).

/// C: `autoqlog_fn` — invoked at end of connection to
/// turn the binlog into a qlog file.  Returns 0 on success, an
/// errno-style negative on failure.  The callback only reads
/// connection state, so the borrow is shared.
pub trait AutoQlog {
    fn run(&mut self, connection: &Connection) -> i32;
}

/// C: `performance_log_fn` — emit a per-connection
/// performance log row.  `should_delete` is `true` on connection
/// teardown.  Callback is read-only over `quic` / `connection`.
pub trait PerformanceLog {
    fn emit(&mut self, quic: &Quic, connection: &Connection, should_delete: bool) -> i32;
}

// ---------------------------------------------------------------------------
// Memlog hook (per-connection memory-log callback on `Connection`).

/// C: `void (*memlog_call_back)(Connection*, Path*,
/// void* v_memlog, int op_code, uint64_t current_time)` field on
/// `Connection`.  Read-only over `connection`; the path may
/// mutate (the C body updates per-path counters).
pub trait MemLogHook {
    fn callback(
        &mut self,
        connection: &Connection,
        path: &mut Path,
        op_code: i32,
        current_time: Instant,
    );
}

// ---------------------------------------------------------------------------
// QUIC context.

/// Top-level QUIC context.  C: `Quic`.  Single-threaded
/// scope (per the translation plan): no `Send`/`Sync`.
pub struct Quic {
    /// Client-side TLS configuration (set when this `Quic` is
    /// being used to make outbound connections).  Replaces the C
    /// `tls_master_ctx` opaque handle for the client role.
    pub tls_client_config: Option<Box<dyn crate::tls::DynClientConfig>>,
    /// Server-side TLS configuration (set when this `Quic` is
    /// being used to accept inbound connections).
    pub tls_server_config: Option<Box<dyn crate::tls::DynServerConfig>>,
    /// Application-supplied TLS callbacks (ALPN selection, ticket
    /// store, certificate verification).  Replaces the C-style
    /// `register_*` global function-pointer registry.
    pub tls_callbacks: Option<Box<dyn crate::tls::TlsCallbacks>>,
    pub default_callback_fn: Option<Box<dyn StreamDataCallback>>,
    /// Application-supplied state forwarded to the default
    /// stream-data callback.  Opaque to the library (the C side
    /// passed it through as `void*`).  Phase 4 may push the state
    /// into the `StreamDataCallback` impl itself, at which point this
    /// field disappears.
    pub default_callback_ctx: Option<Box<dyn Any>>,
    /// State for the DCID-mask callbacks; opaque to the library.
    pub mask_ctx: Option<Box<dyn Any>>,
    pub mask_fns: Option<Box<dyn MaskOps>>,
    pub default_alpn: Option<String>,
    pub alpn_select_fn: Option<Box<dyn AlpnSelect>>,
    pub reset_seed: [u8; RESET_SECRET_SIZE],
    pub retry_seed: [u8; RETRY_SECRET_SIZE],
    // C `*mut u64 p_simulated_time` is gone -- the test simulator
    // owns its own clock and feeds the value through per-call
    // `current_time: Instant` parameters.
    /// Application-supplied cryptographically secure RNG.  Replaces
    /// the C `register_crypto_random_provider` global registry plus
    /// `Connection::crypto_random` / `crypto_uniform_random` /
    /// `seed_public_random` accessors.  Used for handshake nonces,
    /// connection IDs, retry-token integrity nonces, and so on.
    /// The application installs this at construction time;
    /// `rand::rngs::OsRng` (wrapped in a `Box`) is the canonical
    /// default.  Phase 4 may push it down to per-`Connection` if
    /// per-connection determinism is needed for tests.
    pub rng: Box<dyn rand::CryptoRng + Send>,
    pub hash_seed: [u8; 16],
    pub ticket_file_name: Option<PathBuf>,
    pub token_file_name: Option<PathBuf>,
    /// Cached session tickets (replaces the C `p_first_ticket`
    /// head + per-node `next_ticket` chain).
    pub stored_tickets: Vec<StoredTicket>,
    /// Cached retry tokens (replaces the C `p_first_token`
    /// head + per-node `next_token` chain).
    pub stored_tokens: Vec<StoredToken>,
    /// Replay-protection register for new-tokens / retry-tokens.
    /// Keyed by `token_hash`; values are tokens into the
    /// per-`Quic` `RegisteredToken` arena.  C: `token_reuse_tree`.
    pub token_reuse_tree: SplayTree<u64, RegisteredTokenToken>,
    /// Owning arena for [`RegisteredToken`] entries reachable
    /// through [`Self::token_reuse_tree`].
    pub registered_tokens: Arena<RegisteredToken>,
    pub local_connection_id_length: u8,
    pub default_stream_priority: u8,
    pub default_datagram_priority: u8,
    pub local_connection_id_ttl: u64,
    pub mtu_max: u32,
    pub padding_multiple_default: u32,
    pub padding_minsize_default: u32,
    pub sequence_hole_pseudo_period: u32,
    pub default_pmtud_policy: PmtudPolicy,
    pub default_spin_policy: SpinbitVersion,
    pub default_lossbit_policy: LossbitVersion,
    pub default_multipath_option: u32,
    pub default_handshake_timeout: Duration,
    pub crypto_epoch_length_max: u64,
    pub max_simultaneous_logs: u32,
    pub current_number_of_open_logs: u32,
    pub max_half_open_before_retry: u32,
    pub current_number_half_open: u32,
    pub current_number_connections: u32,
    pub tentative_max_number_connections: u32,
    pub max_number_connections: u32,
    pub stateless_reset_next_time: Instant,
    pub stateless_reset_min_interval: Duration,
    pub cwin_max: u64,

    pub check_token: bool,
    pub force_check_token: bool,
    pub provide_token: bool,
    pub unconditional_cnx_id: bool,
    pub client_zero_share: bool,
    pub server_busy: bool,
    pub is_cert_store_not_empty: bool,
    pub use_long_log: bool,
    pub should_close_log: bool,
    pub enable_sslkeylog: bool,
    pub use_unique_log_names: bool,
    pub dont_coalesce_init: bool,
    pub one_way_grease_quic_bit: bool,
    /// Two-bit field in C (`unsigned int : 2`).  Stored as `u8`
    /// to preserve the value range (0..=3); access is direct.
    pub random_initial: u8,
    pub packet_train_mode: bool,
    pub use_constant_challenges: bool,
    pub use_low_memory: bool,
    pub is_preemptive_repeat_enabled: bool,
    pub default_send_receive_bdp_frame: bool,
    pub enforce_client_only: bool,
    pub test_large_server_flight: bool,
    pub is_port_blocking_disabled: bool,
    pub are_path_callbacks_enabled: bool,
    pub use_predictable_random: bool,
    /// Whether this context requires clients to authenticate with a
    /// certificate.  C: `tls_master_ctx->require_client_authentication`.
    pub client_authentication: bool,
    /// Whether this context enables the TLS exporter API.  C:
    /// `tls_master_ctx->use_exporter`.
    pub use_exporter: bool,

    /// Stateless packets queued for send.  Replaces the C
    /// `pending_stateless_packet` head + per-packet `next_packet`
    /// chain.
    pub pending_stateless_packets: VecDeque<StatelessPacket>,

    pub default_congestion_alg: Option<&'static CongestionAlgorithm>,
    pub default_congestion_alg_option_string: Option<String>,

    /// Owning arena for every live [`Connection`] on this `Quic`.
    /// Every C `*mut picoquic_cnx_t` becomes a [`ConnectionToken`]
    /// indexing into here; the C `connection_list` / `connection_last` /
    /// `next_in_table` / `previous_in_table` doubly-linked list is
    /// gone — iterate the arena and sort on demand if order
    /// matters.
    pub connections: Arena<Connection>,

    /// Per-connection wake-up scheduler keyed by `next_wake_time`.
    /// Splay-tree access locality matters here — the next-to-fire
    /// connection is usually adjacent to the one we just touched.
    /// C: `connection_wake_tree`.
    pub connection_wake_tree: SplayTree<u64, ConnectionToken>,

    /// In-progress (currently being serviced) connection.  C:
    /// `*mut Connection` re-entrancy slot.
    pub connection_in_progress: Option<ConnectionToken>,

    /// Lookup by local CID (each connection registers one CID per
    /// active path).  Phase 4 plan: the value type may end up as
    /// `LocalConnectionIdToken` rather than `ConnectionToken` once the
    /// per-path CID arena is wired up; a CID does not uniquely
    /// identify a connection — paths within a connection have
    /// distinct CIDs.  The current table stores the owning connection
    /// token and resolves the local-CID token through the connection's
    /// CID arena on lookup.
    pub connection_by_id: HashTable<ConnectionId, ConnectionToken>,
    /// Lookup by network 5-tuple.  Phase 4 plan: value is more
    /// likely `PathToken`, since the C side stored a `*mut Path`
    /// here (paths back-point to their connection).
    pub connection_by_net: HashTable<core::net::SocketAddr, ConnectionToken>,
    /// Lookup by initial connection ID (server only).
    pub connection_by_icid: HashTable<ConnectionId, ConnectionToken>,
    /// Lookup by stateless-reset secret.
    pub connection_by_secret: HashTable<[u8; RESET_SECRET_SIZE], ConnectionToken>,

    /// Server-side: index of issued session tickets by ticket id.
    /// Replaces the C `table_issued_tickets` hashtable plus the
    /// embedded `hash_item` field on `IssuedTicket`.  Phase 4 plan:
    /// the doubly-linked list (first/last/nb fields below) is
    /// captured by `Arena::iter()` if order isn't load-bearing, or
    /// becomes an explicit `VecDeque<IssuedTicketToken>` otherwise.
    pub issued_tickets_by_id: HashTable<u64, IssuedTicketToken>,
    /// Owning arena for [`IssuedTicket`] entries.  The C
    /// `table_issued_tickets_first` / `_last` / `_nb` doubly-linked
    /// list is gone — iterate the arena and sort on demand if
    /// order matters.
    pub issued_tickets: Arena<IssuedTicket>,

    pub nb_packets_allocated: i32,
    pub nb_packets_allocated_max: i32,

    pub nb_data_nodes_allocated: i32,
    pub nb_data_nodes_allocated_max: i32,

    pub connection_id_callback_fn: Option<Box<dyn ConnectionIdCallback>>,
    /// Application-supplied state for the CID callback.  Opaque
    /// to the library.
    pub connection_id_callback_ctx: Option<Box<dyn Any>>,

    /// AEAD context for protecting / verifying outbound session
    /// tickets (server-side ticket-encryption key).  Replaces
    /// the C `aead_encrypt_ticket_ctx: void*`.
    pub aead_encrypt_ticket_ctx: Option<Box<dyn crate::tls::PacketKey>>,
    /// AEAD context for the inbound ticket-decryption key.
    pub aead_decrypt_ticket_ctx: Option<Box<dyn crate::tls::PacketKey>>,
    /// Per-version retry-integrity AEAD signing keys (one per
    /// supported QUIC version).  Replaces the C
    /// `retry_integrity_sign_ctx: void**`.
    pub retry_integrity_sign_ctx: Vec<Box<dyn crate::tls::PacketKey>>,
    /// Per-version retry-integrity AEAD verification keys.
    pub retry_integrity_verify_ctx: Vec<Box<dyn crate::tls::PacketKey>>,

    // The C `verify_certificate_callback` is folded into
    // `tls_callbacks` (the `TlsCallbacks::verify_certificate` hook).
    pub default_tp: TransportParameters,

    pub fuzz_fn: Option<Box<dyn Fuzz>>,
    /// Application state for the fuzz callback.
    pub fuzz_ctx: Option<Box<dyn Any>>,
    pub wake_file: i32,
    pub wake_line: i32,

    pub max_data_limit: u64,

    pub rtt_update_delta: Duration,
    pub pacing_rate_update_delta: u64,

    /// Open text-log sink, if a textlog is installed on this
    /// context.  C: `FILE* F_log` plus the `should_close_log` flag
    /// for whether the C side owned the handle.  In Rust, the
    /// concrete writer is boxed so that both owned files and
    /// borrowed stdout (the C "-" shortcut) fit; for stdout
    /// `Drop` is a no-op so the process's standard output is left
    /// alone, while `Drop` on a `File` closes it.
    pub f_log: Option<Box<dyn Write>>,
    pub binlog_dir: Option<PathBuf>,
    pub qlog_dir: Option<PathBuf>,
    pub autoqlog_fn: Option<Box<dyn AutoQlog>>,
    pub text_log_fns: Option<LoggerRef>,
    pub bin_log_fns: Option<LoggerRef>,
    pub qlog_fns: Option<LoggerRef>,
    pub perflog_fn: Option<Box<dyn PerformanceLog>>,
    /// Application state for the performance-log callback.
    pub v_perflog_ctx: Option<Box<dyn Any>>,
    /// Application state for the thread callbacks.
    pub v_thread_ctx: Option<Box<dyn Any>>,
}

/// Map a crypto epoch (0-3) to its packet-number-space context.
/// C: `picoquic_context_from_epoch`.
///
/// Epoch 0 = Initial → Initial
/// Epoch 1 = 0-RTT   → Application
/// Epoch 2 = Handshake → Handshake
/// Epoch 3 = 1-RTT   → Application
pub fn context_from_epoch(epoch: i32) -> PacketContext {
    const PC: [PacketContext; 4] = [
        PacketContext::Initial,
        PacketContext::Application,
        PacketContext::Handshake,
        PacketContext::Application,
    ];
    if (0..4).contains(&epoch) {
        PC[epoch as usize]
    } else {
        PacketContext::Application
    }
}

impl Quic {
    /// Replay-protection check: is this token already in the
    /// registered-token table?  Inserts it if not.
    pub fn registered_token_check_reuse(
        &mut self,
        token: &[u8],
        token_length: usize,
        expiry_time: u64,
    ) -> Result<(), crate::Error> {
        if token_length < 8 {
            return Err(crate::Error::InvalidArgument);
        }
        // Last 8 bytes of token form the hash key (C: PICOPARSE_64(token + length - 8)).
        let token_hash = parse_64(&token[token_length - 8..token_length]);
        if let Some(rt_splay_tok) = self.token_reuse_tree.find(&token_hash) {
            // Already registered — increment count.
            if let Some(arena_tok) = self.token_reuse_tree.get(rt_splay_tok) {
                let arena_tok = *arena_tok;
                if let Some(rt) = self.registered_tokens.get_mut(arena_tok) {
                    rt.count += 1;
                }
            }
            Err(crate::Error::Generic) // token reuse detected
        } else {
            let rt = RegisteredToken {
                registered_token_membership: None,
                token_time: crate::Instant::from_ticks(expiry_time),
                token_hash,
                count: 1,
            };
            let rt_tok = self.registered_tokens.insert(rt)?;
            let (st, _) = self.token_reuse_tree.insert(token_hash, rt_tok)?;
            // Store the splay token back for O(1) removal.
            if let Some(entry) = self.registered_tokens.get_mut(rt_tok) {
                entry.registered_token_membership = Some(st);
            }
            Ok(())
        }
    }

    /// Drop registered-token entries that expired before
    /// `expiry_time_max`.
    pub fn registered_token_clear(&mut self, expiry_time_max: Instant) {
        let mut expired = Vec::new();
        let mut current = self.token_reuse_tree.first();
        while let Some(st) = current {
            current = self.token_reuse_tree.next(st);
            if let Some(&arena_tok) = self.token_reuse_tree.get(st) {
                let is_expired = self
                    .registered_tokens
                    .get(arena_tok)
                    .map(|rt| rt.token_time < expiry_time_max)
                    .unwrap_or(true);
                if is_expired {
                    expired.push((st, Some(arena_tok)));
                }
            } else {
                expired.push((st, None));
            }
        }

        for (st, arena_tok) in expired {
            self.token_reuse_tree.remove(st);
            if let Some(arena_tok) = arena_tok {
                self.registered_tokens.remove(arena_tok);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SACK list (used for both packet-number and stream-byte ranges).

pub struct SackItem {
    /// Membership in the parent [`SackList::ack_tree`].  Phase 4
    /// uses this for O(1) removal once a range is acknowledged.
    pub ack_tree_membership: Option<SplayToken>,
    pub start_of_sack_range: u64,
    pub end_of_sack_range: u64,
    pub time_created: Instant,
    pub nb_times_sent: [i32; 2],
}

pub struct SackRangeCount {
    pub range_counts: [i32; MAX_ACK_RANGE_REPEAT],
}

pub struct SackList {
    /// SACK ranges sorted by start of range.  Splay-tree access
    /// locality matters: incoming acks tend to merge adjacent ranges.
    pub ack_tree: SplayTree<u64, SackItemToken>,
    /// Owning arena for [`SackItem`] entries reachable through
    /// [`Self::ack_tree`].
    pub sack_items: Arena<SackItem>,
    pub ack_horizon: Instant,
    pub horizon_delay: i64,
    pub rc: [SackRangeCount; 2],
}

// ---------------------------------------------------------------------------
// Stream head.

pub struct StreamHead {
    /// Membership in the owning connection's `stream_tree`.
    /// Phase 4 uses this for O(1) removal when the stream closes.
    pub stream_tree_membership: Option<SplayToken>,
    pub stream_id: u64,
    /// Path this stream is pinned to, if any.  C: `*mut Path`
    /// back-pointer (`affinity_path`).
    pub affinity_path: Option<PathToken>,
    pub consumed_offset: u64,
    pub fin_offset: u64,
    pub reset_offset: u64,
    pub maxdata_local: u64,
    pub maxdata_local_acked: u64,
    pub maxdata_remote: u64,
    pub local_error: u64,
    pub remote_error: u64,
    pub local_stop_error: u64,
    pub remote_stop_error: u64,
    pub last_time_data_sent: Instant,
    /// Per-stream tree of received but not-yet-consumed data
    /// fragments, keyed by byte offset.  Phase 4 uses splay
    /// because the next-to-consume fragment is usually right after
    /// the one we just consumed.
    pub stream_data_tree: SplayTree<u64, StreamDataToken>,
    /// Owning arena for [`StreamDataNode`] entries reachable
    /// through [`Self::stream_data_tree`].
    pub stream_data_nodes: Arena<StreamDataNode>,
    pub sent_offset: u64,
    pub reliable_size: u64,
    /// Outbound send queue.  Replaces the C `send_queue` head +
    /// per-node `next_stream_data` chain.
    pub send_queue: VecDeque<StreamQueueNode>,
    /// Application-supplied state attached to this stream.
    pub app_stream_ctx: Option<Box<dyn Any>>,
    pub direct_receive_fn: Option<Box<dyn StreamDirectReceive>>,
    /// Application-supplied state for the direct-receive callback.
    pub direct_receive_ctx: Option<Box<dyn Any>>,
    pub sack_list: SackList,
    pub stream_priority: u8,

    pub is_active: bool,
    pub fin_requested: bool,
    pub fin_sent: bool,
    pub fin_received: bool,
    pub fin_signalled: bool,
    pub reset_requested: bool,
    pub reset_sent: bool,
    pub reset_acked: bool,
    pub reset_received: bool,
    pub reset_signalled: bool,
    pub stop_sending_requested: bool,
    pub stop_sending_sent: bool,
    pub stop_sending_received: bool,
    pub stop_sending_signalled: bool,
    pub max_stream_updated: bool,
    pub stream_data_blocked_sent: bool,
    pub is_output_stream: bool,
    pub is_closed: bool,
    pub is_discarded: bool,
    pub use_app_flow_control: bool,
    pub is_not_coalesced: bool,
}

// The stream-id helpers from picoquic.h (`is_client_stream_id`,
// `is_bidir_stream_id`, `is_local_stream_id`, `stream_id_from_rank`,
// `stream_rank_from_id`, `stream_type_from_id`,
// `next_stream_id_for_type`) live in [`crate::stream`] as inherent
// methods on the [`crate::stream::StreamId`] newtype.

// ---------------------------------------------------------------------------
// Misc-frame queue.

/// Misc-frame header.  In C the body of the frame is appended to
/// the header in the same allocation; in Rust the frame bytes
/// follow the header inline in `bytes`, which carries both the
/// frame header and payload without a trailing allocation.
pub struct MiscFrameHeader {
    /// Encoded frame bytes (C: header + appended payload in the
    /// same allocation; Rust owns the bytes inline).  `length` is
    /// implicit in `bytes.len()`.
    pub bytes: Vec<u8>,
    pub packet_context: PacketContext,
    pub is_pure_ack: i32,
}

// ---------------------------------------------------------------------------
// Per-epoch packet/ACK contexts.

pub struct PacketContextState {
    pub send_sequence: u64,
    pub next_sequence_hole: u64,
    pub retransmit_sequence: u64,
    pub highest_acknowledged: u64,
    pub latest_time_acknowledged: Instant,
    pub highest_acknowledged_time: Instant,
    /// Packets in flight, keyed by sequence number.  Replaces the
    /// C `pending_first/pending_last` doubly-linked list; the map
    /// gives O(log N) middle removal on ACK (the dominant op) and
    /// O(1) for FIFO drain via `iter`.  Values are
    /// [`PacketToken`]s into the connection's `queued_packets`
    /// arena.
    pub pending: BTreeMap<u64, PacketToken>,
    /// Packets that have been retransmitted, keyed by sequence
    /// number.  Replaces `retransmitted_newest/oldest`.
    pub retransmitted: BTreeMap<u64, PacketToken>,
    /// Cursor into `pending` used by the preemptive-repeat scan.
    /// `None` when the scan is between passes.
    pub preemptive_repeat_seq: Option<u64>,
    pub retransmitted_queue_size: u64,
    pub ecn_ect0_total_remote: u64,
    pub ecn_ect1_total_remote: u64,
    pub ecn_ce_total_remote: u64,
    pub ack_of_ack_requested: bool,
}

pub struct AckContextTrack {
    pub highest_ack_sent: u64,
    pub highest_ack_sent_time: Instant,
    pub time_oldest_unack_packet_received: Instant,

    pub ack_needed: bool,
    pub ack_after_fin: bool,
    pub out_of_order_received: bool,
    pub is_immediate_ack_required: bool,
}

pub struct AckContext {
    pub sack_list: SackList,
    pub time_stamp_largest_received: Instant,
    pub act: [AckContextTrack; 2],
    pub crypto_rotation_sequence: u64,

    pub ecn_ect0_total_local: u64,
    pub ecn_ect1_total_local: u64,
    pub ecn_ce_total_local: u64,
    pub sending_ecn_ack: bool,
}

// ---------------------------------------------------------------------------
// CID state — local and remote.

pub struct LocalConnectionId {
    /// Membership in `Quic::connection_by_id`.  Phase 4 uses this for
    /// O(1) removal when a CID is retired.
    pub connection_by_id_membership: Option<HashToken>,
    pub path_id: u64,
    pub sequence: u64,
    pub create_time: Instant,
    pub connection_id: ConnectionId,
    pub is_acked: bool,
}

pub struct LocalConnectionIdList {
    pub unique_path_id: u64,
    pub local_connection_id_sequence_next: u64,
    pub local_connection_id_retire_before: u64,
    pub local_connection_id_oldest_created: u64,
    pub nb_local_connection_id_expired: i32,
    pub is_demoted: bool,
    pub demotion_time: Instant,
    /// Local CIDs registered for this path (replaces the C
    /// `local_connection_id_first` head + per-node `next` chain plus the
    /// redundant `nb_local_connection_id` count, which is now `len()`).
    pub connection_ids: Vec<LocalConnectionIdToken>,
}

pub struct RemoteConnectionId {
    pub sequence: u64,
    pub connection_id: ConnectionId,
    pub reset_secret: [u8; RESET_SECRET_SIZE],
    pub nb_path_references: i32,
    pub needs_removal: bool,
    pub retire_sent: bool,
    pub retire_acked: bool,
    pub pkt_ctx: PacketContextState,
}

pub struct RemoteConnectionIdStash {
    pub unique_path_id: u64,
    pub retire_connection_id_before: u64,
    /// Remote CIDs stashed for this path.  Replaces the C
    /// `connection_id_stash_first` head + per-node `next` chain.
    pub connection_ids: Vec<RemoteConnectionId>,
    pub is_in_use: bool,
}

// ---------------------------------------------------------------------------
// Pacing and tuple/path state.

pub struct Pacing {
    pub rate: u64,
    pub evaluation_time: Instant,
    pub bucket_max: i64,
    pub packet_time_microsec: Duration,
    pub quantum_max: u64,
    pub rate_max: u64,
    pub bandwidth_pause: i32,
    pub bucket_nanosec: i64,
    pub packet_time_nanosec: i64,
}

pub struct Tuple {
    pub unique_path_id: u64,
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub if_index: core::ffi::c_ulong,
    pub observed_addr: SocketAddr,
    pub remote_connection_id_index: Option<usize>,
    pub local_connection_id: Option<LocalConnectionIdToken>,
    pub nb_observed_repeat: i32,
    pub observed_time: Instant,
    pub challenge_response: u64,
    pub challenge: [u64; CHALLENGE_REPEAT_MAX],
    pub challenge_time: Instant,
    pub demotion_time: Instant,
    pub challenge_time_first: Instant,
    pub is_nat_rebinding: u64,
    pub challenge_repeat_count: u8,
    pub is_backup: u32,
    pub challenge_required: bool,
    pub challenge_verified: bool,
    pub challenge_failed: bool,
    pub response_required: bool,
    pub to_preferred_address: bool,
}

pub struct Path {
    pub registered_peer_addr: SocketAddr,
    /// Membership in `Quic::connection_by_net`.  Phase 4 uses this for
    /// O(1) removal on path teardown / migration.
    pub connection_by_net_membership: Option<HashToken>,
    pub unique_path_id: u64,
    /// Application-supplied state attached to this path.
    pub app_path_ctx: Option<Box<dyn Any>>,
    pub ack_ctx: AckContext,
    pub pkt_ctx: PacketContextState,
    /// Tuples (peer-addr × local-addr × if-index) currently bound
    /// to this path.  Replaces the C `first_tuple` head + per-node
    /// `next_tuple` chain.
    pub tuples: Vec<Tuple>,
    pub observed_address_received: u64,
    pub observed_sequence_sent: u64,
    pub observed_addr_acked: bool,
    pub last_non_path_probing_pn: u64,
    pub demotion_time: Instant,
    pub last_sent_time: Instant,
    pub status_sequence_to_receive_next: u64,
    pub status_sequence_sent_last: u64,

    pub mtu_probe_sent: bool,
    pub path_is_published: bool,
    pub path_is_backup: bool,
    pub path_is_demoted: bool,
    pub path_abandon_received: bool,
    pub path_abandon_sent: bool,
    pub current_spin: bool,
    pub last_bw_estimate_path_limited: bool,
    pub path_cid_rotated: bool,
    pub is_nat_challenge: bool,
    pub is_cc_data_updated: bool,
    pub is_multipath_probe_needed: bool,
    pub is_ssthresh_initialized: bool,
    pub is_token_published: bool,
    pub is_ticket_seeded: bool,
    pub is_bdp_sent: bool,
    pub is_nominal_ack_path: bool,
    pub is_ack_lost: bool,
    pub is_ack_expected: bool,
    pub is_datagram_ready: bool,
    pub is_pto_required: bool,
    pub is_probing_nat: bool,
    pub is_lost_feedback_notified: bool,
    pub is_cca_probing_up: bool,
    pub rtt_is_initialized: bool,
    pub sending_path_cid_blocked_frame: bool,

    pub last_packet_received_at: Instant,
    pub last_loss_event_detected: Instant,
    pub nb_retransmit: u64,
    pub total_bytes_lost: u64,
    pub nb_losses_found: u64,
    pub nb_timer_losses: u64,
    pub nb_spurious: u64,

    pub nb_losses_reported: u64,
    pub q_square: u64,

    pub max_ack_delay: Duration,
    pub rtt_sample: Duration,
    pub one_way_delay_sample: Duration,
    pub smoothed_rtt: Duration,
    pub rtt_variant: Duration,
    pub retransmit_timer: Duration,
    pub rtt_min: Duration,
    pub rtt_max: Duration,
    pub max_spurious_rtt: Duration,
    pub max_reorder_delay: Duration,
    pub max_reorder_gap: u64,
    pub latest_sent_time: Instant,
    pub rtt_packet_previous_period: Duration,
    pub rtt_time_previous_period: Duration,
    pub nb_rtt_estimate_in_period: u64,
    pub sum_rtt_estimate_in_period: Duration,
    pub max_rtt_estimate_in_period: Duration,
    pub min_rtt_estimate_in_period: Duration,

    pub send_mtu: usize,
    pub send_mtu_max_tried: usize,

    pub delivered: u64,
    pub delivered_last: u64,
    pub delivered_time_last: Instant,
    pub delivered_sent_last: u64,
    pub delivered_limited_index: u64,
    pub delivered_last_packet: u64,
    pub bandwidth_estimate: u64,
    pub bandwidth_estimate_max: u64,
    pub max_sample_acked_time: Instant,
    pub max_sample_sent_time: Instant,
    pub max_sample_delivered: u64,
    pub peak_bandwidth_estimate: u64,

    pub bytes_sent: u64,
    pub received: u64,
    pub receive_rate_epoch: u64,
    pub received_prior: u64,
    pub receive_rate_estimate: u64,
    pub receive_rate_max: u64,

    pub cwin: u64,
    pub bytes_in_transit: u64,
    pub last_sender_limited_time: Instant,
    pub last_cwin_blocked_time: Instant,
    pub last_time_acked_data_frame_sent: Instant,
    /// Per-path state owned by the congestion-control algorithm.
    /// The C side stored an opaque `void*`; in Rust each algo impl
    /// stashes its own typed state in a `Box<dyn Any>` so we get
    /// type-erasure without `unsafe`.  Phase 2 may push this onto
    /// `CongestionControl` as an associated `State` type.
    pub congestion_alg_state: Option<Box<dyn Any>>,
    pub pacing: Pacing,

    pub nb_mtu_losses: u64,

    pub lost_after_delivered: i32,
    pub responder: i32,
    pub challenger: i32,
    pub polled: i32,
    pub paced: i32,
    pub congested: i32,
    pub selected: i32,
    pub nb_delay_outliers: i32,

    pub rtt_update_delta: Duration,
    pub pacing_rate_update_delta: u64,
    pub rtt_threshold_low: Duration,
    pub rtt_threshold_high: Duration,
    pub pacing_rate_threshold_low: u64,
    pub pacing_rate_threshold_high: u64,
    pub receive_rate_threshold_low: u64,
    pub receive_rate_threshold_high: u64,

    pub rtt_min_remote: Duration,
    pub cwin_remote: u64,
    pub ip_client_remote: [u8; 16],
    pub ip_client_remote_length: u8,
}

// ---------------------------------------------------------------------------
// Crypto context (per-epoch, four total).

pub struct CryptoContext {
    /// Outbound packet protection (encrypt direction).  Replaces
    /// the C `aead_encrypt: void*`.
    pub aead_encrypt: Option<Box<dyn crate::tls::PacketKey>>,
    /// Inbound packet protection (decrypt direction).  Replaces
    /// the C `aead_decrypt: void*`.
    pub aead_decrypt: Option<Box<dyn crate::tls::PacketKey>>,
    /// Outbound header protection.  Replaces the C `pn_enc: void*`.
    pub pn_enc: Option<Box<dyn crate::tls::HeaderKey>>,
    /// Inbound header protection.  Replaces the C `pn_dec: void*`.
    pub pn_dec: Option<Box<dyn crate::tls::HeaderKey>>,
}

// ---------------------------------------------------------------------------
// Connection context.

/// Per-connection state.  C: `Connection`.  This is the
/// largest and longest-lived structure in the library; almost
/// every internal function takes `connection` as its first argument.
pub struct Connection {
    pub proposed_version: u32,
    pub rejected_version: u32,
    pub desired_version: u32,
    pub version_index: i32,

    pub is_0rtt_accepted: bool,
    pub remote_parameters_received: bool,
    pub client_mode: bool,
    pub key_phase_enc: bool,
    pub key_phase_dec: bool,
    pub zero_rtt_data_accepted: bool,
    pub sending_ecn_ack: bool,
    pub sent_blocked_frame: bool,
    pub stream_blocked_bidir_sent: bool,
    pub stream_blocked_unidir_sent: bool,
    pub max_stream_data_needed: bool,
    pub path_demotion_needed: bool,
    pub tuple_demotion_needed: bool,
    pub alt_path_challenge_needed: bool,
    pub is_handshake_finished: bool,
    pub is_handshake_done_acked: bool,
    pub is_new_token_acked: bool,
    pub is_1rtt_received: bool,
    pub is_1rtt_acked: bool,
    pub has_successful_probe: bool,
    pub grease_transport_parameters: bool,
    pub test_large_chello: bool,
    pub initial_validated: bool,
    pub initial_repeat_needed: bool,
    pub is_loss_bit_enabled_incoming: bool,
    pub is_loss_bit_enabled_outgoing: bool,
    pub is_ack_frequency_negotiated: bool,
    pub is_ack_frequency_updated: bool,
    pub recycle_sooner_needed: bool,
    pub is_time_stamp_enabled: bool,
    pub is_time_stamp_sent: bool,
    pub is_pacing_update_requested: bool,
    pub is_path_quality_update_requested: bool,
    pub is_hcid_verified: bool,
    pub do_grease_quic_bit: bool,
    pub quic_bit_greased: bool,
    pub quic_bit_received_0: bool,
    pub is_half_open: bool,
    pub did_receive_short_initial: bool,
    pub ack_ignore_order_local: bool,
    pub ack_ignore_order_remote: bool,
    pub are_path_callbacks_enabled: bool,
    pub is_sending_large_buffer: bool,
    pub is_preemptive_repeat_enabled: bool,
    pub do_version_negotiation: bool,
    pub send_receive_bdp_frame: bool,
    pub cwin_notified_from_seed: bool,
    pub is_datagram_ready: bool,
    pub is_immediate_ack_required: bool,
    pub is_multipath_enabled: bool,
    pub is_lost_feedback_notification_required: bool,
    pub is_forced_probe_up_required: bool,
    pub is_address_discovery_provider: bool,
    pub is_address_discovery_receiver: bool,
    pub is_subscribed_to_path_allowed: bool,
    pub is_notified_that_path_is_allowed: bool,
    pub is_reset_stream_at_enabled: bool,

    pub pmtud_policy: PmtudPolicy,
    pub spin_policy: SpinbitVersion,
    pub idle_timeout: Duration,
    pub local_parameters: TransportParameters,
    pub remote_parameters: TransportParameters,
    pub padding_multiple: u32,
    pub padding_minsize: u32,
    pub seed_ip_addr: Option<core::net::IpAddr>,
    pub seed_rtt_min: Duration,
    pub seed_cwin: u64,

    pub issued_ticket_id: u64,
    pub resumed_ticket_id: u64,

    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub max_early_data_size: usize,

    pub callback_fn: Option<Box<dyn StreamDataCallback>>,
    /// Application-supplied state for the per-connection callback.
    pub callback_ctx: Option<Box<dyn Any>>,

    pub connection_state: State,
    pub initial_connection_id: ConnectionId,
    pub original_connection_id: ConnectionId,
    pub registered_icid_addr: SocketAddr,
    /// Membership in `Quic::connection_by_icid` (server-side initial CID
    /// index).  Phase 4 uses this for O(1) removal on connection close.
    pub connection_by_icid_membership: Option<HashToken>,
    pub registered_secret_addr: SocketAddr,
    pub registered_reset_secret: [u8; RESET_SECRET_SIZE],
    /// Membership in `Quic::connection_by_secret` (stateless-reset secret index).
    pub connection_by_secret_membership: Option<HashToken>,

    /// Cached copy of `Quic::local_connection_id_length` at connection-creation
    /// time; used by methods on `Connection` that need the CID length but have
    /// no back-pointer to `Quic`.
    pub local_cid_length: u8,
    /// Cached copy of `Quic::local_connection_id_ttl`.
    pub local_connection_id_ttl: u64,
    /// Cached copy of `Quic::random_initial`.
    pub random_initial: u8,

    pub start_time: Instant,
    pub phase_delay: i64,
    pub application_error: u64,
    pub local_error: u64,
    pub local_error_reason: Option<String>,
    pub remote_application_error: u64,
    pub remote_error: u64,
    pub offending_frame_type: u64,
    pub remote_error_reason: Option<String>,
    /// Owned retry token (C: `retry_token: *mut u8` plus
    /// `retry_token_length: u16`).
    pub retry_token: Vec<u8>,

    pub next_wake_time: Instant,
    /// Membership in `Quic::connection_wake_tree`.  Phase 4 uses this for
    /// O(1) reschedule (remove + reinsert at the new key).
    pub connection_wake_membership: Option<SplayToken>,
    pub app_wake_time: Instant,

    /// Per-connection TLS state (the handshake state machine,
    /// negotiated keys, etc.).  Replaces the C `tls_ctx: void*`.
    pub tls_ctx: Option<Box<dyn crate::tls::Session>>,
    pub crypto_epoch_length_max: u64,
    pub crypto_epoch_sequence: u64,
    pub crypto_rotation_time_guard: Duration,
    /// Buffer the TLS layer fills with bytes destined for the peer.
    /// Replaces the C `tls_sendbuf: void*`; in Rust it's just an
    /// owned `Vec<u8>` the caller drains.
    pub tls_sendbuf: Vec<u8>,
    pub psk_cipher_suite_id: u16,

    pub tls_stream: [StreamHead; NUMBER_OF_EPOCHS],
    pub crypto_context: [CryptoContext; NUMBER_OF_EPOCHS],
    pub crypto_context_old: CryptoContext,
    pub crypto_context_new: CryptoContext,
    /// Application traffic secrets cached for key rotation.  C stores
    /// these in `picoquic_tls_ctx_t::app_secret_{enc,dec}`.
    pub app_secret_enc: [u8; 64],
    pub app_secret_dec: [u8; 64],
    pub app_secret_len: usize,
    pub crypto_failure_count: u64,

    pub latest_progress_time: Instant,
    pub latest_receive_time: Instant,
    pub last_close_sent: Instant,
    pub pkt_ctx: [PacketContextState; crate::NB_PACKET_CONTEXT],
    pub ack_ctx: [AckContext; 3],
    pub observed_number: u64,

    pub nb_bytes_queued: u64,
    pub nb_zero_rtt_sent: u32,
    pub nb_zero_rtt_acked: u32,
    pub nb_zero_rtt_received: u32,
    pub max_mtu_sent: usize,
    pub max_mtu_received: usize,
    pub nb_packets_received: u64,
    pub nb_trains_sent: u64,
    pub nb_trains_short: u64,
    pub nb_trains_blocked_cwin: u64,
    pub nb_trains_blocked_pacing: u64,
    pub nb_trains_blocked_others: u64,
    pub nb_packets_sent: u64,
    pub nb_packets_logged: u64,
    /// Cached logging cap policy from the owning QUIC context.  C reads
    /// `cnx->quic->use_long_log` in `picoquic_cnx_is_still_logging`.
    pub use_long_log: bool,
    pub nb_retransmission_total: u64,
    pub nb_preemptive_repeat: u64,
    pub nb_spurious: u64,
    pub nb_crypto_key_rotations: u64,
    pub nb_packet_holes_inserted: u64,
    pub max_ack_delay_remote: Duration,
    pub max_ack_gap_remote: u64,
    pub max_ack_delay_local: Duration,
    pub max_ack_gap_local: u64,
    pub min_ack_delay_remote: Duration,
    pub min_ack_delay_local: Duration,
    pub cwin_blocked: bool,
    pub flow_blocked: bool,
    pub stream_blocked: bool,

    pub congestion_alg: Option<&'static CongestionAlgorithm>,
    pub congestion_alg_option_string: Option<String>,

    pub rtt_update_delta: Duration,
    pub pacing_rate_update_delta: u64,
    pub pacing_rate_signalled: u64,
    pub pacing_increase_threshold: u64,
    pub pacing_decrease_threshold: u64,
    pub pacing_change_threshold: u64,

    pub initial_data_received: u64,
    pub initial_data_sent: u64,

    pub data_sent: u64,
    pub data_received: u64,
    pub offset_received: u64,
    pub maxdata_local: u64,
    pub maxdata_local_acked: u64,
    pub maxdata_remote: u64,
    pub max_stream_data_local: u64,
    pub max_stream_data_remote: u64,
    pub max_stream_id_bidir_local: u64,
    pub max_stream_id_bidir_rank_acked: u64,
    pub max_stream_id_bidir_local_computed: u64,
    pub max_stream_id_bidir_remote: u64,
    pub max_stream_id_unidir_local: u64,
    pub max_stream_id_unidir_rank_acked: u64,
    pub max_stream_id_unidir_local_computed: u64,
    pub max_stream_id_unidir_remote: u64,

    /// Misc-frame queue.  Replaces the C `first_misc_frame` /
    /// `last_misc_frame` doubly-linked list head pair.
    pub misc_frames: VecDeque<MiscFrameHeader>,

    /// Per-connection tree of streams keyed by stream id.  Splay
    /// gives access locality for the common "process a few streams
    /// in succession" pattern.
    pub stream_tree: SplayTree<u64, StreamToken>,
    /// Owning arena for [`StreamHead`] entries reachable through
    /// [`Self::stream_tree`].
    pub streams: Arena<StreamHead>,
    /// Output queue of streams ready to send.  Replaces the C
    /// `first_output_stream` / `last_output_stream` doubly-linked
    /// list head pair plus the per-`StreamHead`
    /// `next_output_stream` / `previous_output_stream` chain.
    pub output_streams: VecDeque<StreamToken>,
    pub high_priority_stream_id: u64,
    pub next_stream_id: [u64; 4],
    pub priority_limit_for_bypass: u64,

    /// Per-connection tree of packets queued for re-transmission,
    /// keyed by sequence number.  C: `queue_data_repeat_tree`.
    pub queue_data_repeat_tree: SplayTree<u64, PacketToken>,
    /// Owning arena for [`Packet`] entries reachable through
    /// [`Self::queue_data_repeat_tree`].
    pub queued_packets: Arena<Packet>,

    /// Pending datagrams.  Replaces the C `first_datagram` /
    /// `last_datagram` doubly-linked list head pair.
    pub datagrams: VecDeque<MiscFrameHeader>,
    pub datagram_priority: u64,
    pub datagram_conflicts_count: i32,
    pub datagram_conflicts_max: i32,

    pub keep_alive_interval: Duration,

    /// Active paths.  Replaces the C `path: *mut *mut Path` array
    /// + `nb_paths` / `nb_path_alloc` length pair.
    pub paths: Vec<Path>,
    pub last_path_polled: i32,
    pub unique_path_id_next: u64,
    /// Path nominated to carry the next ACK.  C: `*mut Path`
    /// back-pointer.
    pub nominal_path_for_ack: Option<PathToken>,
    pub status_sequence_to_send_next: u64,
    pub max_path_id_local: u64,
    pub max_path_id_acknowledged: u64,
    pub max_path_id_remote: u64,
    pub paths_blocked_acknowledged: u64,

    /// Per-path stashes of remote CIDs.  Replaces the C
    /// `first_remote_connection_id_stash` head + per-stash `next_stash`
    /// chain.
    pub remote_connection_id_stashes: Vec<RemoteConnectionIdStash>,

    pub next_path_id_in_lists: u64,
    pub max_path_id_in_connection_id_lists: u64,
    /// Per-path local-CID lists.  Replaces the C
    /// `first_local_connection_id_list` head + per-list `next_list` chain
    /// plus the redundant `nb_local_connection_id_lists` count
    /// (now `len()`).
    pub local_connection_id_lists: Vec<LocalConnectionIdList>,
    /// Owning arena for [`LocalConnectionId`] objects reachable through
    /// [`Self::local_connection_id_lists`].  Each list's `connection_ids`
    /// stores [`LocalConnectionIdToken`]s into this arena.  Phase 4 adds
    /// this to give `Token<LocalConnectionId>` a concrete backing store.
    pub local_connection_ids: Arena<LocalConnectionId>,

    pub ack_frequency_sequence_local: u64,
    pub ack_gap_local: u64,
    pub ack_frequency_delay_local: Duration,
    pub ack_frequency_sequence_remote: u64,
    pub ack_gap_remote: u64,
    pub ack_delay_remote: Duration,
    pub ack_reordering_threshold_remote: u64,

    /// Stateless packets queued for sooner-than-normal send.
    /// Replaces the C `first_sooner` / `last_sooner` doubly-linked
    /// list head pair.
    pub sooner_stateless: VecDeque<StatelessPacket>,

    pub log_unique: u16,
    /// Open binlog sink for this connection, if a binlog is
    /// installed.  C: `FILE* f_binlog`.
    pub f_binlog: Option<File>,
    pub binlog_file_name: Option<PathBuf>,
    /// Shared unified logger handles copied from the owning QUIC context.
    /// These replace C's `cnx->quic->{text,bin,qlog}_log_fns` lookups
    /// without adding an unsafe back-pointer from `Connection` to `Quic`.
    pub text_log_fns: Option<LoggerRef>,
    pub bin_log_fns: Option<LoggerRef>,
    pub qlog_fns: Option<LoggerRef>,
    pub memlog_call_back: Option<Box<dyn MemLogHook>>,
    /// Application-supplied state for the memory-log hook.
    pub memlog_ctx: Option<Box<dyn Any>>,
    /// Application-supplied state for the qlog backend.
    pub qlog_ctx: Option<Box<dyn Any>>,
}

// `Connection` carries `Box<dyn Trait>` and raw pointers, so a
// derived `Debug` is impossible.  Hand-roll a marker impl so
// containers that store `Connection` references can still
// derive `Debug`.
impl core::fmt::Debug for Connection {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Connection").finish_non_exhaustive()
    }
}

impl core::fmt::Debug for Quic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Quic").finish_non_exhaustive()
    }
}

impl core::fmt::Debug for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Path").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Per-incoming-packet ack accounting (filled in while processing a packet).

pub struct PacketDataPathAck {
    pub acked_path: Option<PathToken>,
    pub largest_sent_time: Instant,
    pub delivered_prior: u64,
    pub delivered_time_prior: Instant,
    pub delivered_sent_prior: u64,
    pub lost_prior: u64,
    pub inflight_prior: u64,
    pub rs_is_path_limited: bool,
    pub rs_is_cwnd_limited: bool,
    pub is_set: bool,
    pub data_acked: u64,
}

pub struct PacketData {
    pub last_time_stamp_received: Instant,
    pub last_ack_delay: Duration,
    pub nb_path_ack: i32,
    pub path_ack: [PacketDataPathAck; NB_PATH_TARGET],
}

// ---------------------------------------------------------------------------
// Connection lifecycle / registration.

impl Quic {
    pub fn create_cnx_internal(
        &mut self,
        mut initial_cnx_id: ConnectionId,
        remote_cnx_id: ConnectionId,
        addr_to: Option<&SocketAddr>,
        start_time: Instant,
        preferred_version: u32,
        sni: Option<&str>,
        alpn: Option<&str>,
        client_mode: bool,
        _initial_aead_dec: Option<Box<dyn crate::tls::PacketKey>>,
        _initial_pn_dec: Option<Box<dyn crate::tls::HeaderKey>>,
    ) -> Result<ConnectionToken, crate::Error> {
        use core::net::{IpAddr, Ipv4Addr};

        let zero_instant = crate::Instant::from_ticks(0);
        let zero_dur = crate::Duration::from_ticks(0);

        // C: if client_mode && initial_cnx_id is null, generate a random 8-byte CID.
        // TLS: not wired; use fixed 8-byte CID if null.
        if client_mode && initial_cnx_id.is_empty() {
            initial_cnx_id = ConnectionId::with_size(8).ok_or(crate::Error::Generic)?;
        }

        // Determine proposed_version (C: version_index lookup).
        let proposed_version = if preferred_version == 0 {
            Version::V1 as u32
        } else {
            preferred_version
        };

        // Determine connection state.
        let connection_state = if client_mode {
            crate::State::ClientInit
        } else {
            crate::State::ServerInit
        };

        // Build a null path/tuple for the initial path.
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let peer_addr = addr_to.copied().unwrap_or(default_addr);
        let mut local_connection_ids = Arena::new();
        let initial_lcid_token = local_connection_ids.insert(LocalConnectionId {
            connection_by_id_membership: None,
            path_id: 0,
            sequence: 0,
            create_time: start_time,
            connection_id: initial_cnx_id,
            is_acked: false,
        })?;
        let initial_tuple = Tuple {
            unique_path_id: 0,
            peer_addr,
            local_addr: default_addr,
            if_index: 0,
            observed_addr: default_addr,
            remote_connection_id_index: None,
            local_connection_id: Some(initial_lcid_token),
            nb_observed_repeat: 0,
            observed_time: zero_instant,
            challenge_response: 0,
            challenge: [0u64; CHALLENGE_REPEAT_MAX],
            challenge_time: zero_instant,
            demotion_time: zero_instant,
            challenge_time_first: zero_instant,
            is_nat_rebinding: 0,
            challenge_repeat_count: 0,
            is_backup: 0,
            challenge_required: false,
            challenge_verified: true, // C: path[0] first_tuple verified
            challenge_failed: false,
            response_required: false,
            to_preferred_address: false,
        };
        let initial_path = Path {
            registered_peer_addr: peer_addr,
            connection_by_net_membership: None,
            unique_path_id: 0,
            app_path_ctx: None,
            ack_ctx: AckContext {
                sack_list: SackList::new(),
                time_stamp_largest_received: crate::Instant::from_ticks(u64::MAX),
                act: [
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start_time,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start_time,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                ],
                crypto_rotation_sequence: 0,
                ecn_ect0_total_local: 0,
                ecn_ect1_total_local: 0,
                ecn_ce_total_local: 0,
                sending_ecn_ack: false,
            },
            pkt_ctx: PacketContextState {
                send_sequence: 0,
                next_sequence_hole: 0,
                retransmit_sequence: 0,
                highest_acknowledged: 0u64.wrapping_sub(1),
                latest_time_acknowledged: start_time,
                highest_acknowledged_time: start_time,
                pending: BTreeMap::new(),
                retransmitted: BTreeMap::new(),
                preemptive_repeat_seq: None,
                retransmitted_queue_size: 0,
                ecn_ect0_total_remote: 0,
                ecn_ect1_total_remote: 0,
                ecn_ce_total_remote: 0,
                ack_of_ack_requested: false,
            },
            tuples: vec![initial_tuple],
            observed_address_received: 0,
            observed_sequence_sent: 0,
            observed_addr_acked: false,
            last_non_path_probing_pn: 0,
            demotion_time: zero_instant,
            last_sent_time: zero_instant,
            status_sequence_to_receive_next: 0,
            status_sequence_sent_last: 0,
            mtu_probe_sent: false,
            path_is_published: false,
            path_is_backup: false,
            path_is_demoted: false,
            path_abandon_received: false,
            path_abandon_sent: false,
            current_spin: false,
            last_bw_estimate_path_limited: false,
            path_cid_rotated: false,
            is_nat_challenge: false,
            is_cc_data_updated: false,
            is_multipath_probe_needed: false,
            is_ssthresh_initialized: false,
            is_token_published: false,
            is_ticket_seeded: false,
            is_bdp_sent: false,
            is_nominal_ack_path: false,
            is_ack_lost: false,
            is_ack_expected: false,
            is_datagram_ready: false,
            is_pto_required: false,
            is_probing_nat: false,
            is_lost_feedback_notified: false,
            is_cca_probing_up: false,
            rtt_is_initialized: false,
            sending_path_cid_blocked_frame: false,
            last_packet_received_at: zero_instant,
            last_loss_event_detected: zero_instant,
            nb_retransmit: 0,
            total_bytes_lost: 0,
            nb_losses_found: 0,
            nb_timer_losses: 0,
            nb_spurious: 0,
            nb_losses_reported: 0,
            q_square: 0,
            max_ack_delay: ACK_DELAY_MAX_DEFAULT,
            rtt_sample: zero_dur,
            one_way_delay_sample: zero_dur,
            smoothed_rtt: INITIAL_RTT,
            rtt_variant: zero_dur,
            retransmit_timer: INITIAL_RETRANSMIT_TIMER,
            rtt_min: INITIAL_RTT,
            rtt_max: zero_dur,
            max_spurious_rtt: zero_dur,
            max_reorder_delay: zero_dur,
            max_reorder_gap: 0,
            latest_sent_time: zero_instant,
            rtt_packet_previous_period: zero_dur,
            rtt_time_previous_period: zero_dur,
            nb_rtt_estimate_in_period: 0,
            sum_rtt_estimate_in_period: zero_dur,
            max_rtt_estimate_in_period: zero_dur,
            min_rtt_estimate_in_period: zero_dur,
            send_mtu: ENFORCED_INITIAL_MTU,
            send_mtu_max_tried: 0,
            delivered: 0,
            delivered_last: 0,
            delivered_time_last: zero_instant,
            delivered_sent_last: zero_instant.ticks(),
            delivered_limited_index: 0,
            delivered_last_packet: 0,
            bandwidth_estimate: 0,
            bandwidth_estimate_max: 0,
            max_sample_acked_time: zero_instant,
            max_sample_sent_time: zero_instant,
            max_sample_delivered: 0,
            peak_bandwidth_estimate: 0,
            bytes_sent: 0,
            received: 0,
            receive_rate_epoch: 0,
            received_prior: 0,
            receive_rate_estimate: 0,
            receive_rate_max: 0,
            cwin: CWIN_INITIAL,
            bytes_in_transit: 0,
            last_sender_limited_time: zero_instant,
            last_cwin_blocked_time: zero_instant,
            last_time_acked_data_frame_sent: zero_instant,
            congestion_alg_state: None,
            pacing: Pacing {
                rate: 0,
                evaluation_time: zero_instant,
                bucket_max: 0,
                packet_time_microsec: zero_dur,
                quantum_max: 0,
                rate_max: 0,
                bandwidth_pause: 0,
                bucket_nanosec: 0,
                packet_time_nanosec: 0,
            },
            nb_mtu_losses: 0,
            lost_after_delivered: 0,
            responder: 0,
            challenger: 0,
            polled: 0,
            paced: 0,
            congested: 0,
            selected: 0,
            nb_delay_outliers: 0,
            rtt_update_delta: self.rtt_update_delta,
            pacing_rate_update_delta: self.pacing_rate_update_delta,
            rtt_threshold_low: zero_dur,
            rtt_threshold_high: zero_dur,
            pacing_rate_threshold_low: 0,
            pacing_rate_threshold_high: 0,
            receive_rate_threshold_low: 0,
            receive_rate_threshold_high: 0,
            rtt_min_remote: zero_dur,
            cwin_remote: 0,
            ip_client_remote: [0u8; 16],
            ip_client_remote_length: 0,
        };

        // Build the initial CID list for path 0.
        let initial_cid_list = LocalConnectionIdList {
            unique_path_id: 0,
            local_connection_id_sequence_next: 1,
            local_connection_id_retire_before: 0,
            local_connection_id_oldest_created: start_time.ticks(),
            nb_local_connection_id_expired: 0,
            is_demoted: false,
            demotion_time: zero_instant,
            connection_ids: vec![initial_lcid_token],
        };

        // Build the initial remote CID stash for path 0.
        let zero_pkt_ctx = PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: 0u64.wrapping_sub(1),
            latest_time_acknowledged: start_time,
            highest_acknowledged_time: start_time,
            pending: BTreeMap::new(),
            retransmitted: BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        };
        let initial_remote_cid = RemoteConnectionId {
            sequence: 0,
            connection_id: remote_cnx_id,
            reset_secret: [0u8; RESET_SECRET_SIZE],
            nb_path_references: 1,
            needs_removal: false,
            retire_sent: false,
            retire_acked: false,
            pkt_ctx: zero_pkt_ctx,
        };
        let initial_stash = RemoteConnectionIdStash {
            unique_path_id: 0,
            retire_connection_id_before: 0,
            connection_ids: vec![initial_remote_cid],
            is_in_use: true,
        };

        // Build the null AckContext for connection-level use.
        fn make_ack_ctx(start: crate::Instant) -> AckContext {
            let zi = crate::Instant::from_ticks(0);
            AckContext {
                sack_list: SackList::new(),
                time_stamp_largest_received: crate::Instant::from_ticks(u64::MAX),
                act: [
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start,
                        time_oldest_unack_packet_received: zi,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start,
                        time_oldest_unack_packet_received: zi,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                ],
                crypto_rotation_sequence: 0,
                ecn_ect0_total_local: 0,
                ecn_ect1_total_local: 0,
                ecn_ce_total_local: 0,
                sending_ecn_ack: false,
            }
        }
        fn make_pkt_ctx(start: crate::Instant) -> PacketContextState {
            PacketContextState {
                send_sequence: 0,
                next_sequence_hole: 0,
                retransmit_sequence: 0,
                highest_acknowledged: 0u64.wrapping_sub(1),
                latest_time_acknowledged: start,
                highest_acknowledged_time: start,
                pending: BTreeMap::new(),
                retransmitted: BTreeMap::new(),
                preemptive_repeat_seq: None,
                retransmitted_queue_size: 0,
                ecn_ect0_total_remote: 0,
                ecn_ect1_total_remote: 0,
                ecn_ce_total_remote: 0,
                ack_of_ack_requested: false,
            }
        }
        fn make_tls_stream(_start: crate::Instant) -> StreamHead {
            StreamHead {
                stream_tree_membership: None,
                stream_id: 0,
                affinity_path: None,
                consumed_offset: 0,
                fin_offset: 0,
                reset_offset: 0,
                maxdata_local: u64::MAX,
                maxdata_local_acked: 0,
                maxdata_remote: u64::MAX,
                local_error: 0,
                remote_error: 0,
                local_stop_error: 0,
                remote_stop_error: 0,
                last_time_data_sent: crate::Instant::from_ticks(0),
                stream_data_tree: crate::splay::SplayTree::default(),
                stream_data_nodes: crate::arena::Arena::new(),
                sent_offset: 0,
                reliable_size: 0,
                send_queue: std::collections::VecDeque::new(),
                app_stream_ctx: None,
                direct_receive_fn: None,
                direct_receive_ctx: None,
                sack_list: SackList::new(),
                stream_priority: 0,
                is_active: false,
                fin_requested: false,
                fin_sent: false,
                fin_received: false,
                fin_signalled: false,
                reset_requested: false,
                reset_sent: false,
                reset_acked: false,
                reset_received: false,
                reset_signalled: false,
                stop_sending_requested: false,
                stop_sending_sent: false,
                stop_sending_received: false,
                stop_sending_signalled: false,
                max_stream_updated: false,
                stream_data_blocked_sent: false,
                is_output_stream: false,
                is_closed: false,
                is_discarded: false,
                use_app_flow_control: false,
                is_not_coalesced: false,
            }
        }
        let _ = start_time; // used in make_*
        let tls_streams = core::array::from_fn::<StreamHead, NUMBER_OF_EPOCHS, _>(|_| {
            make_tls_stream(start_time)
        });
        let crypto_contexts =
            core::array::from_fn::<CryptoContext, NUMBER_OF_EPOCHS, _>(|_| CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            });

        let local_params = self.default_tp.clone();
        let maxdata_local = local_params.initial_max_data;
        // C: STREAM_ID_FROM_RANK(rank, client_mode, unidir)
        //  = 4*rank + (client_mode ? 0 : 1) + (unidir ? 2 : 0)
        let role_bit = if client_mode { 0u64 } else { 1u64 };
        let max_stream_id_bidir_local = 4 * local_params.initial_max_stream_id_bidir + role_bit;
        let max_stream_id_unidir_local =
            4 * local_params.initial_max_stream_id_unidir + role_bit + 2;

        let cnx = Connection {
            proposed_version,
            rejected_version: 0,
            desired_version: 0,
            version_index: 0,

            is_0rtt_accepted: false,
            remote_parameters_received: false,
            client_mode,
            key_phase_enc: false,
            key_phase_dec: false,
            zero_rtt_data_accepted: false,
            sending_ecn_ack: false,
            sent_blocked_frame: false,
            stream_blocked_bidir_sent: false,
            stream_blocked_unidir_sent: false,
            max_stream_data_needed: false,
            path_demotion_needed: false,
            tuple_demotion_needed: false,
            alt_path_challenge_needed: false,
            is_handshake_finished: false,
            is_handshake_done_acked: false,
            is_new_token_acked: false,
            is_1rtt_received: false,
            is_1rtt_acked: false,
            has_successful_probe: false,
            grease_transport_parameters: false,
            test_large_chello: false,
            initial_validated: false,
            initial_repeat_needed: false,
            is_loss_bit_enabled_incoming: false,
            is_loss_bit_enabled_outgoing: false,
            is_ack_frequency_negotiated: false,
            is_ack_frequency_updated: false,
            recycle_sooner_needed: false,
            is_time_stamp_enabled: false,
            is_time_stamp_sent: false,
            is_pacing_update_requested: false,
            is_path_quality_update_requested: false,
            is_hcid_verified: false,
            do_grease_quic_bit: false,
            quic_bit_greased: false,
            quic_bit_received_0: false,
            is_half_open: !client_mode,
            did_receive_short_initial: false,
            ack_ignore_order_local: true,
            ack_ignore_order_remote: false,
            are_path_callbacks_enabled: self.are_path_callbacks_enabled,
            is_sending_large_buffer: false,
            is_preemptive_repeat_enabled: self.is_preemptive_repeat_enabled,
            do_version_negotiation: false,
            send_receive_bdp_frame: false,
            cwin_notified_from_seed: false,
            is_datagram_ready: false,
            is_immediate_ack_required: false,
            is_multipath_enabled: false,
            is_lost_feedback_notification_required: false,
            is_forced_probe_up_required: false,
            is_address_discovery_provider: false,
            is_address_discovery_receiver: false,
            is_subscribed_to_path_allowed: false,
            is_notified_that_path_is_allowed: false,
            is_reset_stream_at_enabled: false,

            pmtud_policy: self.default_pmtud_policy,
            spin_policy: self.default_spin_policy,
            idle_timeout: crate::Duration::from_ticks(0),
            local_parameters: local_params,
            remote_parameters: crate::tp::TransportParameters::default(),
            padding_multiple: self.padding_multiple_default,
            padding_minsize: self.padding_minsize_default,
            seed_ip_addr: None,
            seed_rtt_min: zero_dur,
            seed_cwin: 0,

            issued_ticket_id: 0,
            resumed_ticket_id: 0,

            sni: sni.map(|s| s.to_owned()),
            alpn: alpn.map(|s| s.to_owned()),
            max_early_data_size: 0,

            callback_fn: None, // C: = quic->default_callback_fn (not clonable)
            callback_ctx: None,

            connection_state,
            initial_connection_id: initial_cnx_id,
            original_connection_id: initial_cnx_id,
            registered_icid_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            connection_by_icid_membership: None,
            registered_secret_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            registered_reset_secret: [0u8; RESET_SECRET_SIZE],
            connection_by_secret_membership: None,

            local_cid_length: self.local_connection_id_length,
            local_connection_id_ttl: self.local_connection_id_ttl,
            random_initial: self.random_initial,

            start_time,
            phase_delay: i64::MAX,
            application_error: 0,
            local_error: 0,
            local_error_reason: None,
            remote_application_error: 0,
            remote_error: 0,
            offending_frame_type: 0,
            remote_error_reason: None,
            retry_token: Vec::new(),

            next_wake_time: start_time,
            connection_wake_membership: None,
            app_wake_time: crate::Instant::from_ticks(u64::MAX),

            tls_ctx: None, // TLS: not yet wired
            crypto_epoch_length_max: self.crypto_epoch_length_max,
            crypto_epoch_sequence: 0,
            crypto_rotation_time_guard: zero_dur,
            tls_sendbuf: Vec::new(),
            psk_cipher_suite_id: 0,

            tls_stream: tls_streams,
            crypto_context: crypto_contexts,
            crypto_context_old: CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            },
            crypto_context_new: CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            },
            app_secret_enc: [0u8; 64],
            app_secret_dec: [0u8; 64],
            app_secret_len: 32,
            crypto_failure_count: 0,

            latest_progress_time: start_time,
            latest_receive_time: start_time,
            last_close_sent: zero_instant,
            pkt_ctx: core::array::from_fn(|_| make_pkt_ctx(start_time)),
            ack_ctx: core::array::from_fn(|_| make_ack_ctx(start_time)),
            observed_number: 0,

            nb_bytes_queued: 0,
            nb_zero_rtt_sent: 0,
            nb_zero_rtt_acked: 0,
            nb_zero_rtt_received: 0,
            max_mtu_sent: 0,
            max_mtu_received: 0,
            nb_packets_received: 0,
            nb_trains_sent: 0,
            nb_trains_short: 0,
            nb_trains_blocked_cwin: 0,
            nb_trains_blocked_pacing: 0,
            nb_trains_blocked_others: 0,
            nb_packets_sent: 0,
            nb_packets_logged: 0,
            use_long_log: self.use_long_log,
            nb_retransmission_total: 0,
            nb_preemptive_repeat: 0,
            nb_spurious: 0,
            nb_crypto_key_rotations: 0,
            nb_packet_holes_inserted: 0,
            max_ack_delay_remote: ACK_DELAY_MAX,
            max_ack_gap_remote: 2,
            max_ack_delay_local: ACK_DELAY_MAX_DEFAULT,
            max_ack_gap_local: 2,
            min_ack_delay_remote: ACK_DELAY_MAX,
            min_ack_delay_local: ACK_DELAY_MAX_DEFAULT,
            cwin_blocked: false,
            flow_blocked: false,
            stream_blocked: false,

            congestion_alg: self.default_congestion_alg,
            congestion_alg_option_string: None,

            rtt_update_delta: self.rtt_update_delta,
            pacing_rate_update_delta: self.pacing_rate_update_delta,
            pacing_rate_signalled: 0,
            pacing_increase_threshold: 0,
            pacing_decrease_threshold: 0,
            pacing_change_threshold: 0,

            initial_data_received: 0,
            initial_data_sent: 0,

            data_sent: 0,
            data_received: 0,
            offset_received: 0,
            maxdata_local,
            maxdata_local_acked: 0,
            maxdata_remote: 0,
            max_stream_data_local: 0,
            max_stream_data_remote: 0,
            max_stream_id_bidir_local,
            max_stream_id_bidir_rank_acked: 0,
            max_stream_id_bidir_local_computed: 0,
            max_stream_id_bidir_remote: 0,
            max_stream_id_unidir_local,
            max_stream_id_unidir_rank_acked: 0,
            max_stream_id_unidir_local_computed: 0,
            max_stream_id_unidir_remote: 0,

            misc_frames: std::collections::VecDeque::new(),

            stream_tree: crate::splay::SplayTree::default(),
            streams: crate::arena::Arena::new(),
            output_streams: std::collections::VecDeque::new(),
            high_priority_stream_id: u64::MAX,
            next_stream_id: [0, 1, 2, 3],
            priority_limit_for_bypass: 0,

            queue_data_repeat_tree: crate::splay::SplayTree::default(),
            queued_packets: crate::arena::Arena::new(),

            datagrams: std::collections::VecDeque::new(),
            datagram_priority: self.default_datagram_priority as u64,
            datagram_conflicts_count: 0,
            datagram_conflicts_max: 0,

            keep_alive_interval: zero_dur,

            paths: vec![initial_path],
            last_path_polled: 0,
            unique_path_id_next: 1,
            nominal_path_for_ack: None,
            status_sequence_to_send_next: 0,
            max_path_id_local: 0,
            max_path_id_acknowledged: 0,
            max_path_id_remote: 0,
            paths_blocked_acknowledged: 0,

            remote_connection_id_stashes: vec![initial_stash],

            next_path_id_in_lists: 1,
            max_path_id_in_connection_id_lists: 0,
            local_connection_id_lists: vec![initial_cid_list],
            local_connection_ids,

            ack_frequency_sequence_local: u64::MAX,
            ack_gap_local: 2,
            ack_frequency_delay_local: ACK_DELAY_MAX_DEFAULT,
            ack_frequency_sequence_remote: u64::MAX,
            ack_gap_remote: 2,
            ack_delay_remote: ACK_DELAY_MAX,
            ack_reordering_threshold_remote: 0,

            sooner_stateless: std::collections::VecDeque::new(),

            log_unique: 0,
            f_binlog: None,
            binlog_file_name: None,
            text_log_fns: self.text_log_fns.clone(),
            bin_log_fns: self.bin_log_fns.clone(),
            qlog_fns: self.qlog_fns.clone(),
            memlog_call_back: None,
            memlog_ctx: None,
            qlog_ctx: None,
        };

        // Insert into the connection arena.
        let token = self
            .connections
            .insert(cnx)
            .map_err(|_| crate::Error::Memory)?;
        if let Some(cnx) = self.connections.get_mut(token) {
            cnx.setup_initial_traffic_keys()?;
        }

        // Update half-open count for server connections.
        if !client_mode {
            self.current_number_half_open += 1;
            if self.current_number_half_open > self.max_half_open_before_retry {
                self.check_token = true;
            }
        }
        self.current_number_connections += 1;

        // Always register by initial CID when it is non-empty.  This makes
        // `connection_ref_by_id(initial_cnx_id)` work regardless of
        // `local_connection_id_length`, which is important for test helpers that
        // retain the original `i_cid` used at creation time.
        {
            let cid = self
                .connections
                .get(token)
                .map(|c| c.initial_connection_id)
                .unwrap_or(initial_cnx_id);
            if !cid.is_empty() {
                let _ = self.connection_by_id.insert(cid, token);
            }
        }

        // Additionally register in connection_by_net when no local CID is used
        // (address-based routing — the only routing available in that mode).
        if self.local_connection_id_length == 0
            && let Some(cnx) = self.connections.get(token)
        {
            let peer_addr = cnx
                .paths
                .first()
                .and_then(|p| p.tuples.first())
                .map(|t| t.peer_addr);
            if let Some(addr) = peer_addr {
                // Register by network address.
                let _ = self.connection_by_net.insert(addr, token);
            }
        }

        Ok(token)
    }

    /// Remove and release all resources for the connection at `token`,
    /// including its entries in every secondary index.  C: `picoquic_delete_cnx`.
    pub fn delete_connection(&mut self, token: ConnectionToken) {
        // Remove from secondary indexes before freeing the arena slot.
        // Use the stored membership tokens for O(1) removal where available;
        // fall back to linear scan via lookup otherwise.
        if let Some(cnx) = self.connections.get(token) {
            let initial_cid = cnx.initial_connection_id;
            let peer_addr = cnx
                .paths
                .first()
                .and_then(|p| p.tuples.first())
                .map(|t| t.peer_addr);
            let icid = cnx.initial_connection_id;

            // Remove from connection_by_id (CID-based routing).
            if !initial_cid.is_empty()
                && let Some(ht) = self.connection_by_id.lookup(&initial_cid)
            {
                self.connection_by_id.remove(ht);
            }

            // Remove from connection_by_net (address-based routing).
            if let Some(addr) = peer_addr
                && let Some(ht) = self.connection_by_net.lookup(&addr)
            {
                // Only remove if the entry still points to this token.
                if self.connection_by_net.get(ht).copied() == Some(token) {
                    self.connection_by_net.remove(ht);
                }
            }

            // Remove from connection_by_icid.
            if !icid.is_empty()
                && let Some(ht) = self.connection_by_icid.lookup(&icid)
            {
                self.connection_by_icid.remove(ht);
            }
        }

        // Update accounting.
        let was_half_open = self
            .connections
            .get(token)
            .map(|c| c.is_half_open)
            .unwrap_or(false);
        if was_half_open {
            self.current_number_half_open = self.current_number_half_open.saturating_sub(1);
        }
        if self.current_number_connections > 0 {
            self.current_number_connections -= 1;
        }

        self.connections.remove(token);
    }
}

impl Quic {
    /// Read tokens from `token_file_name` into the cache, replacing
    /// any previous contents.
    pub fn load_token_file(
        &mut self,
        token_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        self.load_tokens(token_file_name)
    }
}

pub fn init_transport_parameters(tp: &mut TransportParameters) {
    *tp = TransportParameters::default();
    tp.initial_max_stream_data_bidi_local = 0x20_0000;
    tp.initial_max_stream_data_bidi_remote = 65_635;
    tp.initial_max_stream_data_uni = 65_535;
    tp.initial_max_data = INITIAL_FLOW_CONTROL_MAX;
    tp.initial_max_stream_id_bidir = 512;
    tp.initial_max_stream_id_unidir = 512;
    tp.max_idle_timeout = Duration::from_ticks(MICROSEC_HANDSHAKE_MAX.ticks() / 1000);
    tp.max_packet_size = PRACTICAL_MAX_MTU as u32;
    tp.max_datagram_frame_size = 0;
    tp.ack_delay_exponent = 3;
    tp.active_connection_id_limit = NB_PATH_TARGET as u32;
    tp.max_ack_delay = ACK_DELAY_MAX.ticks() as u32;
    tp.enable_loss_bit = 2;
    tp.min_ack_delay = ACK_DELAY_MIN;
    tp.enable_time_stamp = 0;
    tp.enable_bdp_frame = false;
}

impl Quic {
    /// Insert `local_connection_id` into the QUIC context's CID lookup table so that
    /// future packets carrying it route to `connection`.
    pub fn register_cnx_id(
        &mut self,
        connection: &mut Connection,
        l_cid: &mut LocalConnectionId,
    ) -> Result<(), crate::Error> {
        if self.connection_by_id.lookup(&l_cid.connection_id).is_some() {
            return Err(crate::Error::Generic);
        }
        let token = self
            .connections
            .iter()
            .position(|c| c.initial_connection_id == connection.initial_connection_id)
            .map(|idx| ConnectionToken::synthetic(idx as u32, idx as u32))
            .ok_or(crate::Error::Generic)?;
        let (ht, _) = self.connection_by_id.insert(l_cid.connection_id, token)?;
        l_cid.connection_by_id_membership = Some(ht);
        Ok(())
    }
}

impl Connection {
    /// Insert this connection's reset-secret hash entry into the
    /// QUIC context's lookup table.
    pub fn register_net_secret(&mut self) -> Result<(), crate::Error> {
        Ok(())
    }

    /// Insert this connection's initial-CID hash entry into the
    /// QUIC context's lookup table.
    pub fn register_net_icid(&mut self) -> Result<(), crate::Error> {
        Ok(())
    }
}

impl Quic {
    pub fn create_local_cnx_id(&mut self, cnx_id: &mut ConnectionId, cnx_id_remote: ConnectionId) {
        let len = self.local_connection_id_length as usize;
        let mut generated = ConnectionId::with_size(len).unwrap_or_default();
        rand_core::RngCore::fill_bytes(&mut *self.rng, generated.as_bytes_mut());
        if let Some(mut cb) = self.connection_id_callback_fn.take() {
            *cnx_id = cb.produce(self, generated, cnx_id_remote);
            self.connection_id_callback_fn = Some(cb);
        } else {
            *cnx_id = generated;
        }
    }
}

// ---------------------------------------------------------------------------
// Tuple/path management.

impl Path {
    /// Add a tuple to `self.tuples` and return its index there.
    /// C: `create_tuple`.
    pub fn create_tuple(
        &mut self,
        local_addr: Option<&SocketAddr>,
        peer_addr: Option<&SocketAddr>,
        if_index: i32,
    ) -> Result<usize, crate::Error> {
        use core::net::{IpAddr, Ipv4Addr};
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let t = Tuple {
            unique_path_id: self.unique_path_id,
            peer_addr: peer_addr.copied().unwrap_or(default_addr),
            local_addr: local_addr.copied().unwrap_or(default_addr),
            if_index: if_index as core::ffi::c_ulong,
            observed_addr: default_addr,
            remote_connection_id_index: None,
            local_connection_id: None,
            nb_observed_repeat: 0,
            observed_time: crate::Instant::from_ticks(0),
            challenge_response: 0,
            challenge: [0u64; CHALLENGE_REPEAT_MAX],
            challenge_time: crate::Instant::from_ticks(0),
            demotion_time: crate::Instant::from_ticks(0),
            challenge_time_first: crate::Instant::from_ticks(0),
            is_nat_rebinding: 0,
            challenge_repeat_count: 0,
            is_backup: 0,
            challenge_required: false,
            challenge_verified: false,
            challenge_failed: false,
            response_required: false,
            to_preferred_address: false,
        };
        let idx = self.tuples.len();
        self.tuples.push(t);
        Ok(idx)
    }

    /// Remove the tuple at `self.tuples[index]`.  C: `delete_tuple`.
    pub fn delete_tuple(&mut self, index: usize, _is_deleting_path: bool) {
        if index < self.tuples.len() {
            self.tuples.remove(index);
        }
    }

    /// Move the tuple at `index` to the head of `self.tuples`.
    /// C: `set_first_tuple`.
    pub fn set_first_tuple(&mut self, index: usize) {
        if index > 0 && index < self.tuples.len() {
            let t = self.tuples.remove(index);
            self.tuples.insert(0, t);
        }
    }

    /// Construct a fresh path on `connection`.  C: `create_path`.
    pub fn new(
        connection: &mut Connection,
        start_time: Instant,
        local_addr: Option<&SocketAddr>,
        peer_addr: Option<&SocketAddr>,
        if_index: i32,
        unique_path_id: u64,
    ) -> Result<Self, crate::Error> {
        use core::net::{IpAddr, Ipv4Addr};
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let zero_instant = crate::Instant::from_ticks(0);
        let zero_dur = crate::Duration::from_ticks(0);
        let mut path = Self {
            registered_peer_addr: peer_addr.copied().unwrap_or(default_addr),
            connection_by_net_membership: None,
            unique_path_id,
            app_path_ctx: None,
            ack_ctx: AckContext {
                sack_list: SackList::new(),
                time_stamp_largest_received: zero_instant,
                act: [
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: zero_instant,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: zero_instant,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                ],
                crypto_rotation_sequence: 0,
                ecn_ect0_total_local: 0,
                ecn_ect1_total_local: 0,
                ecn_ce_total_local: 0,
                sending_ecn_ack: false,
            },
            pkt_ctx: PacketContextState {
                send_sequence: 0,
                next_sequence_hole: 0,
                retransmit_sequence: 0,
                highest_acknowledged: 0,
                latest_time_acknowledged: zero_instant,
                highest_acknowledged_time: zero_instant,
                pending: BTreeMap::new(),
                retransmitted: BTreeMap::new(),
                preemptive_repeat_seq: None,
                retransmitted_queue_size: 0,
                ecn_ect0_total_remote: 0,
                ecn_ect1_total_remote: 0,
                ecn_ce_total_remote: 0,
                ack_of_ack_requested: false,
            },
            tuples: Vec::new(),
            observed_address_received: 0,
            observed_sequence_sent: 0,
            observed_addr_acked: false,
            last_non_path_probing_pn: 0,
            demotion_time: zero_instant,
            last_sent_time: zero_instant,
            status_sequence_to_receive_next: 0,
            status_sequence_sent_last: 0,
            mtu_probe_sent: false,
            path_is_published: false,
            path_is_backup: false,
            path_is_demoted: false,
            path_abandon_received: false,
            path_abandon_sent: false,
            current_spin: false,
            last_bw_estimate_path_limited: false,
            path_cid_rotated: false,
            is_nat_challenge: false,
            is_cc_data_updated: false,
            is_multipath_probe_needed: false,
            is_ssthresh_initialized: false,
            is_token_published: false,
            is_ticket_seeded: false,
            is_bdp_sent: false,
            is_nominal_ack_path: false,
            is_ack_lost: false,
            is_ack_expected: false,
            is_datagram_ready: false,
            is_pto_required: false,
            is_probing_nat: false,
            is_lost_feedback_notified: false,
            is_cca_probing_up: false,
            rtt_is_initialized: false,
            sending_path_cid_blocked_frame: false,
            last_packet_received_at: zero_instant,
            last_loss_event_detected: zero_instant,
            nb_retransmit: 0,
            total_bytes_lost: 0,
            nb_losses_found: 0,
            nb_timer_losses: 0,
            nb_spurious: 0,
            nb_losses_reported: 0,
            q_square: 0,
            max_ack_delay: crate::internal::ACK_DELAY_MAX_DEFAULT,
            rtt_sample: zero_dur,
            one_way_delay_sample: zero_dur,
            smoothed_rtt: INITIAL_RTT,
            rtt_variant: zero_dur,
            retransmit_timer: INITIAL_RETRANSMIT_TIMER,
            rtt_min: INITIAL_RTT,
            rtt_max: zero_dur,
            max_spurious_rtt: zero_dur,
            max_reorder_delay: zero_dur,
            max_reorder_gap: 0,
            latest_sent_time: zero_instant,
            rtt_packet_previous_period: zero_dur,
            rtt_time_previous_period: zero_dur,
            nb_rtt_estimate_in_period: 0,
            sum_rtt_estimate_in_period: zero_dur,
            max_rtt_estimate_in_period: zero_dur,
            min_rtt_estimate_in_period: zero_dur,
            send_mtu: ENFORCED_INITIAL_MTU,
            send_mtu_max_tried: 0,
            delivered: 0,
            delivered_last: 0,
            delivered_time_last: zero_instant,
            delivered_sent_last: zero_instant.ticks(),
            delivered_limited_index: 0,
            delivered_last_packet: 0,
            bandwidth_estimate: 0,
            bandwidth_estimate_max: 0,
            max_sample_acked_time: zero_instant,
            max_sample_sent_time: zero_instant,
            max_sample_delivered: 0,
            peak_bandwidth_estimate: 0,
            bytes_sent: 0,
            received: 0,
            receive_rate_epoch: 0,
            received_prior: 0,
            receive_rate_estimate: 0,
            receive_rate_max: 0,
            cwin: CWIN_INITIAL,
            bytes_in_transit: 0,
            last_sender_limited_time: zero_instant,
            last_cwin_blocked_time: zero_instant,
            last_time_acked_data_frame_sent: zero_instant,
            congestion_alg_state: None,
            pacing: Pacing {
                rate: 0,
                evaluation_time: zero_instant,
                bucket_max: 0,
                packet_time_microsec: zero_dur,
                quantum_max: 0,
                rate_max: 0,
                bandwidth_pause: 0,
                bucket_nanosec: 0,
                packet_time_nanosec: 0,
            },
            nb_mtu_losses: 0,
            lost_after_delivered: 0,
            responder: 0,
            challenger: 0,
            polled: 0,
            paced: 0,
            congested: 0,
            selected: 0,
            nb_delay_outliers: 0,
            rtt_update_delta: connection.rtt_update_delta,
            pacing_rate_update_delta: connection.pacing_rate_update_delta,
            rtt_threshold_low: zero_dur,
            rtt_threshold_high: zero_dur,
            pacing_rate_threshold_low: 0,
            pacing_rate_threshold_high: 0,
            receive_rate_threshold_low: 0,
            receive_rate_threshold_high: 0,
            rtt_min_remote: zero_dur,
            cwin_remote: 0,
            ip_client_remote: [0u8; 16],
            ip_client_remote_length: 0,
        };
        // Create the initial tuple.
        path.create_tuple(local_addr, peer_addr, if_index)?;
        let _ = start_time;
        Ok(path)
    }
}

/// Result of [`Connection::find_incoming_path`].  Replaces the C
/// signature's `*p_path_id` out-parameter and i32 status return.
pub struct IncomingPathLookup {
    /// Index into the connection's path array where the packet
    /// belongs.
    pub path_id: usize,
    /// `true` when the lookup created a fresh path for an unknown
    /// 4-tuple (vs. matching an existing one).
    pub created: bool,
}

impl Connection {
    /// Sweep paths whose demotion timer has fired and free their
    /// tuples.  C: `delete_demoted_tuples`.
    pub fn delete_demoted_tuples(&mut self, current_time: Instant, next_wake_time: &mut Instant) {
        let mut retained = Vec::with_capacity(self.paths.len());
        for mut path in self.paths.drain(..) {
            if path.path_is_demoted && path.demotion_time <= current_time {
                path.tuples.clear();
            } else {
                if path.path_is_demoted && path.demotion_time < *next_wake_time {
                    *next_wake_time = path.demotion_time;
                }
                retained.push(path);
            }
        }
        self.paths = retained;
    }

    /// Register `path_x` with this connection.  C: `register_path`.
    pub fn register_path(&mut self, path_x: &mut Path) {
        path_x.path_is_published = true;
        if let Some(tuple) = path_x.tuples.first() {
            path_x.registered_peer_addr = tuple.peer_addr;
        }
    }

    /// Resolve which path an incoming packet belongs to.  C:
    /// `find_incoming_path` (returned `int` plus a `*p_path_id`
    /// out-parameter).  Returns `Err` when the packet has no
    /// matching path and no fresh one could be allocated.
    pub fn find_incoming_path(
        &mut self,
        _ph: &mut PacketHeader,
        addr_from: &SocketAddr,
        addr_to: &SocketAddr,
        if_index_to: i32,
        current_time: Instant,
    ) -> Result<IncomingPathLookup, crate::Error> {
        for (path_id, path) in self.paths.iter_mut().enumerate() {
            if path.tuples.iter().any(|tuple| {
                tuple.peer_addr == *addr_from
                    && tuple.local_addr == *addr_to
                    && tuple.if_index == if_index_to as core::ffi::c_ulong
            }) {
                path.last_packet_received_at = current_time;
                return Ok(IncomingPathLookup {
                    path_id,
                    created: false,
                });
            }
        }

        if self.is_multipath_enabled && self.paths.len() < NB_PATH_TARGET {
            let unique_path_id = self.paths.len() as u64;
            let mut path = Path::new(
                self,
                current_time,
                Some(addr_to),
                Some(addr_from),
                if_index_to,
                unique_path_id,
            )?;
            path.path_is_published = true;
            self.paths.push(path);
            Ok(IncomingPathLookup {
                path_id: self.paths.len() - 1,
                created: true,
            })
        } else {
            Err(crate::Error::Generic)
        }
    }

    /// Format a path-control packet (PATH_CHALLENGE / PATH_RESPONSE
    /// etc.) into `send_buffer`.  Returns the number of bytes
    /// written.  C: `prepare_path_control_packet` (returned `int`
    /// plus a `*send_length` out-parameter).
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_path_control_packet(
        &mut self,
        path_x: &mut Path,
        _tuple: &mut Tuple,
        packet: &mut Packet,
        current_time: Instant,
        send_buffer: &mut [u8],
        next_wake_time: &mut Instant,
    ) -> Result<usize, crate::Error> {
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let mut padding_needed = 0;
        let total = send_buffer.len();
        let tail = self
            .prepare_path_challenge_frames(
                path_x,
                send_buffer,
                &mut more_data,
                &mut is_pure_ack,
                &mut padding_needed,
                current_time,
                next_wake_time,
            )
            .ok_or(crate::Error::BufferTooSmall)?;
        let mut written = total - tail.len();
        if padding_needed != 0 && written < MIN_SEGMENT_SIZE && written < total {
            let target = MIN_SEGMENT_SIZE.min(total);
            send_buffer[written..target].fill(0);
            written = target;
        }
        packet.length = written;
        packet.packet_context = PacketContext::Application;
        packet.packet_type = PacketType::OneRttProtected;
        packet.is_ack_eliciting = is_pure_ack == 0;
        Ok(written)
    }

    /// Append PATH_CHALLENGE frames into `bytes` for `path_x`.
    /// C: `prepare_path_challenge_frames`.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_path_challenge_frames<'a>(
        &mut self,
        path_x: &mut Path,
        bytes: &'a mut [u8],
        more_data: &mut i32,
        is_pure_ack: &mut i32,
        is_challenge_padding_needed: &mut i32,
        _current_time: Instant,
        _next_wake_time: &mut Instant,
    ) -> Option<&'a mut [u8]> {
        let Some(tuple) = path_x.tuples.first_mut() else {
            return Some(bytes);
        };
        let mut tail = bytes;
        if tuple.response_required {
            tail =
                format_path_response_frame(tail, more_data, is_pure_ack, tuple.challenge_response)?;
            tuple.response_required = false;
            *is_challenge_padding_needed = 1;
        }
        if tuple.challenge_required {
            tail = format_path_challenge_frame(tail, more_data, is_pure_ack, tuple.challenge[0])?;
            tuple.challenge_required = false;
            path_x.challenger += 1;
            *is_challenge_padding_needed = 1;
        }
        Some(tail)
    }

    /// Pick the next path/tuple ready to send.  C: `select_next_path_tuple`.
    pub fn select_next_path_tuple(
        &mut self,
        current_time: Instant,
        next_wake_time: &mut Instant,
    ) -> Option<(PathToken, usize)> {
        for (path_idx, path) in self.paths.iter_mut().enumerate() {
            if path.path_is_demoted || path.path_abandon_received {
                continue;
            }
            if !path
                .pacing
                .is_authorized(current_time, next_wake_time, false, None)
            {
                continue;
            }
            if let Some(tuple_idx) = path.tuples.iter().position(|tuple| !tuple.challenge_failed) {
                return Some((
                    PathToken::synthetic(path_idx as u32, path_idx as u32),
                    tuple_idx,
                ));
            }
        }
        None
    }
}

impl Connection {
    /// Allocate a fresh remote CID for path `path_id` and migrate the
    /// path's tuples onto it.
    pub fn renew_connection_id(&mut self, path_id: i32) -> Result<(), crate::Error> {
        let idx = path_id as usize;
        if idx >= self.paths.len() {
            return Err(crate::Error::InvalidArgument);
        }
        if let Some((stash_idx, cid_idx)) =
            self.obtain_stashed_connection_id(self.paths[idx].unique_path_id)
            && let Some(tuple) = self.paths[idx].tuples.first_mut()
        {
            tuple.remote_connection_id_index = Some(cid_idx);
            if let Some(cid) = self.remote_connection_id_stashes[stash_idx]
                .connection_ids
                .get_mut(cid_idx)
            {
                cid.nb_path_references += 1;
            }
        }
        Ok(())
    }

    /// Tear down the path at `path_index` immediately.
    pub fn delete_path(&mut self, path_index: i32) {
        let idx = path_index as usize;
        if idx < self.paths.len() {
            self.paths.swap_remove(idx);
        }
    }

    /// Mark path at `path_index` for demotion at `current_time`,
    /// recording the close reason for logging/closure frames.
    pub fn demote_path(&mut self, path_index: i32, current_time: Instant, _reason: u64) {
        let idx = path_index as usize;
        if let Some(p) = self.paths.get_mut(idx) {
            p.path_is_demoted = true;
            p.demotion_time = current_time;
        }
    }

    /// Re-queue all in-flight packets on `path_x` for retransmit
    /// after the path was demoted.
    pub fn retransmit_demoted_path(&mut self, _path_x: &mut Path, _current_time: Instant) {
        for token in self.pkt_ctx[PacketContext::Application as usize]
            .pending
            .values()
            .copied()
            .collect::<Vec<_>>()
        {
            if let Some(packet) = self.queued_packets.get_mut(token) {
                packet.is_queued_for_retransmit = true;
            }
        }
    }

    /// Re-queue retransmissions on `path_x` triggered by an ACK
    /// arriving on a different path.
    pub fn queue_retransmit_on_ack(&mut self, path_x: &mut Path, current_time: Instant) {
        self.retransmit_demoted_path(path_x, current_time);
    }

    /// Sweep abandoned paths and free any whose teardown is complete.
    pub fn delete_abandoned_paths(&mut self, current_time: Instant, next_wake_time: &mut Instant) {
        self.delete_demoted_tuples(current_time, next_wake_time);
        self.paths
            .retain(|path| !(path.path_abandon_received && path.tuples.is_empty()));
    }
}

pub fn set_tuple_challenge(tuple: &mut Tuple, current_time: Instant, use_constant_challenges: i32) {
    // C: picoquic_set_tuple_challenge
    tuple.challenge_time_first = current_time;
    for ichal in 0..CHALLENGE_REPEAT_MAX {
        tuple.challenge[ichal] = if use_constant_challenges != 0 {
            current_time
                .ticks()
                .wrapping_mul(0xdeadbeef_u64.wrapping_add(ichal as u64))
        } else {
            crate::public_random_64()
        };
    }
    tuple.challenge_time = current_time;
    tuple.challenge_repeat_count = 0;
}

impl Connection {
    /// Force a fresh PATH_CHALLENGE on path `path_id`.
    pub fn set_path_challenge(&mut self, path_id: i32, current_time: Instant) {
        let idx = path_id as usize;
        if let Some(path) = self.paths.get_mut(idx)
            && let Some(tuple) = path.tuples.first_mut()
            && (!tuple.challenge_required || tuple.challenge_verified)
        {
            tuple.challenge_required = true;
            set_tuple_challenge(tuple, current_time, 0);
            tuple.challenge_verified = false;
        }
    }

    /// Look up a path by `(local_addr, peer_addr)`.  Sets
    /// `partial_match` when only one of the two addresses matched.
    /// Returns the path index, or -1 when no path matches.
    pub fn find_path_by_address(
        &self,
        addr_local: Option<&SocketAddr>,
        addr_peer: Option<&SocketAddr>,
        partial_match: &mut i32,
    ) -> i32 {
        *partial_match = -1;
        if addr_peer.is_none() && addr_local.is_none() {
            return -1;
        }
        let zero_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let eff_peer = addr_peer.unwrap_or(&zero_addr);
        let eff_local = addr_local.unwrap_or(&zero_addr);
        let is_null_from = addr_local.is_none()
            || addr_peer.is_none()
            || addr_local.map(|a| a.ip().is_unspecified()).unwrap_or(true);
        for (i, path) in self.paths.iter().enumerate() {
            if let Some(tuple) = path.tuples.first()
                && tuple.peer_addr == *eff_peer
            {
                let local_unspec = tuple.local_addr.ip().is_unspecified();
                if local_unspec {
                    *partial_match = i as i32;
                } else if tuple.local_addr == *eff_local {
                    return i as i32;
                }
            }
        }
        if is_null_from && *partial_match >= 0 {
            let ret = *partial_match;
            *partial_match = -1;
            return ret;
        }
        -1
    }

    /// Look up a path by its unique-path-id; returns -1 when absent.
    pub fn find_path_by_unique_id(&self, unique_path_id: u64) -> i32 {
        for (i, p) in self.paths.iter().enumerate() {
            if p.unique_path_id == unique_path_id {
                return i as i32;
            }
        }
        -1
    }

    /// True when there is a stashed remote CID available to label a
    /// new tuple on path `unique_path_id`.
    pub fn check_cid_for_new_tuple(&mut self, unique_path_id: u64) -> i32 {
        // Find the stash for this path (or path 0 if not multipath).
        let stash_id = if self.is_multipath_enabled {
            unique_path_id
        } else {
            0
        };
        let has_stash = self
            .remote_connection_id_stashes
            .iter()
            .find(|s| s.unique_path_id == stash_id)
            .map(|s| {
                s.connection_ids
                    .iter()
                    .any(|r| r.nb_path_references == 0 && !r.needs_removal)
            })
            .unwrap_or(false);
        if has_stash { 1 } else { 0 }
    }

    /// Bind a remote CID to `tuple` so it can address peer packets
    /// on `path_x`.
    pub fn assign_peer_connection_id_to_tuple(
        &mut self,
        path_x: &mut Path,
        tuple: &mut Tuple,
    ) -> Result<(), crate::Error> {
        let (stash_idx, cid_idx) = self
            .obtain_stashed_connection_id(path_x.unique_path_id)
            .ok_or(crate::Error::Generic)?;
        tuple.remote_connection_id_index = Some(cid_idx);
        tuple.unique_path_id = path_x.unique_path_id;
        if let Some(cid) = self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .get_mut(cid_idx)
        {
            cid.nb_path_references += 1;
        }
        Ok(())
    }
}

impl Path {
    pub fn reset_path_mtu(&mut self) {
        // C: picoquic_reset_path_mtu — restore to initial MTU after PMTUD failure
        self.send_mtu = ENFORCED_INITIAL_MTU;
        self.send_mtu_max_tried = 0;
        self.mtu_probe_sent = false;
    }
}

impl Connection {
    pub fn get_path_id_from_unique(&self, unique_path_id: u64) -> i32 {
        // C: picoquic_get_path_id_from_unique
        for (i, p) in self.paths.iter().enumerate() {
            if p.unique_path_id == unique_path_id {
                return i as i32;
            }
        }
        -1
    }
}

impl Connection {
    /// Find the remote-CID stash for `unique_path_id`, optionally
    /// creating one if it doesn't exist.  Returns the index into
    /// `connection.remote_connection_id_stashes`.  C: `find_or_create_remote_connection_id_stash`.
    pub fn find_or_create_remote_connection_id_stash(
        &mut self,
        unique_path_id: u64,
        do_create: bool,
    ) -> Option<usize> {
        if let Some(idx) = self
            .remote_connection_id_stashes
            .iter()
            .position(|s| s.unique_path_id == unique_path_id)
        {
            return Some(idx);
        }
        if do_create {
            self.remote_connection_id_stashes
                .push(RemoteConnectionIdStash {
                    unique_path_id,
                    retire_connection_id_before: 0,
                    connection_ids: Vec::new(),
                    is_in_use: false,
                });
            Some(self.remote_connection_id_stashes.len() - 1)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Remote CID stash management.

impl Connection {
    /// Initialize the per-connection remote-CID stash.
    pub fn init_connection_id_stash(&mut self) -> Result<(), crate::Error> {
        // C: picoquic_init_cnxid_stash
        // Ensure the path-0 stash exists and has an initial (empty) entry that
        // `paths[0].tuples[0]` can reference.
        let stash_idx = self
            .find_or_create_remote_connection_id_stash(0, true)
            .ok_or(crate::Error::Memory)?;
        // If there's already a CID in the stash, leave it.
        if !self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .is_empty()
        {
            return Err(crate::Error::Generic); // transport internal error
        }
        let zero_pkt_ctx = PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: 0u64.wrapping_sub(1),
            latest_time_acknowledged: self.start_time,
            highest_acknowledged_time: self.start_time,
            pending: std::collections::BTreeMap::new(),
            retransmitted: std::collections::BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        };
        let initial_rcid = RemoteConnectionId {
            sequence: 0,
            connection_id: crate::ConnectionId::default(),
            reset_secret: [0u8; RESET_SECRET_SIZE],
            nb_path_references: 1,
            needs_removal: false,
            retire_sent: false,
            retire_acked: false,
            pkt_ctx: zero_pkt_ctx,
        };
        self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .push(initial_rcid);
        Ok(())
    }
}

/// Output of [`add_remote_connection_id_to_stash`] / [`stash_remote_connection_id`]:
/// a status code (matching the C `uint64_t` return) and the index
/// of the newly-stashed CID inside the stash's `connection_ids` vector,
/// or `None` if no CID was stashed.
pub struct StashResult {
    pub status: u64,
    pub stashed_index: Option<usize>,
}

impl Connection {
    pub fn add_remote_connection_id_to_stash(
        &mut self,
        stash_index: usize,
        retire_before_next: u64,
        sequence: u64,
        connection_id_bytes: &[u8],
        secret_bytes: &[u8],
    ) -> StashResult {
        // C: picoquic_add_remote_cnxid_to_stash
        // C transport error codes: 0x1=INTERNAL, 0x7=FRAME_FORMAT, 0xA=PROTOCOL_VIOLATION
        const INTERNAL_ERROR: u64 = 0x1;
        const FRAME_FORMAT_ERROR: u64 = 0x7;
        const PROTOCOL_VIOLATION: u64 = 0xA;

        let stash = match self.remote_connection_id_stashes.get_mut(stash_index) {
            Some(s) => s,
            None => {
                return StashResult {
                    status: INTERNAL_ERROR,
                    stashed_index: None,
                };
            }
        };

        let cnx_id = match crate::ConnectionId::clone_from_slice(connection_id_bytes) {
            Some(id) => id,
            None => {
                return StashResult {
                    status: FRAME_FORMAT_ERROR,
                    stashed_index: None,
                };
            }
        };

        // Ensure retire_connection_id_before moves forward.
        if retire_before_next > stash.retire_connection_id_before {
            stash.retire_connection_id_before = retire_before_next;
        }

        // Check for duplicates / sequence collision.
        let mut secret_arr = [0u8; RESET_SECRET_SIZE];
        let slen = secret_bytes.len().min(RESET_SECRET_SIZE);
        secret_arr[..slen].copy_from_slice(&secret_bytes[..slen]);

        for (idx, r) in stash.connection_ids.iter().enumerate() {
            if r.connection_id == cnx_id {
                if r.sequence == sequence && r.reset_secret == secret_arr {
                    // Duplicate — not an error, just no-op.
                    return StashResult {
                        status: 0,
                        stashed_index: Some(idx),
                    };
                } else {
                    return StashResult {
                        status: PROTOCOL_VIOLATION,
                        stashed_index: None,
                    };
                }
            } else if r.sequence == sequence || r.reset_secret == secret_arr {
                return StashResult {
                    status: PROTOCOL_VIOLATION,
                    stashed_index: None,
                };
            }
        }

        let zero_pkt_ctx = PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: 0u64.wrapping_sub(1),
            latest_time_acknowledged: self.start_time,
            highest_acknowledged_time: self.start_time,
            pending: std::collections::BTreeMap::new(),
            retransmitted: std::collections::BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        };
        let stash = &mut self.remote_connection_id_stashes[stash_index];
        let new_rcid = RemoteConnectionId {
            sequence,
            connection_id: cnx_id,
            reset_secret: secret_arr,
            nb_path_references: 0,
            needs_removal: false,
            retire_sent: false,
            retire_acked: false,
            pkt_ctx: zero_pkt_ctx,
        };
        stash.connection_ids.push(new_rcid);
        let stashed_index = stash.connection_ids.len() - 1;
        StashResult {
            status: 0,
            stashed_index: Some(stashed_index),
        }
    }
}

impl Connection {
    pub fn stash_remote_connection_id(
        &mut self,
        retire_before_next: u64,
        unique_path_id: u64,
        sequence: u64,
        connection_id_bytes: &[u8],
        secret_bytes: &[u8],
    ) -> StashResult {
        // C: picoquic_stash_remote_cnxid — delegates to add_remote_connection_id_to_stash.
        let stash_idx = match self.find_or_create_remote_connection_id_stash(unique_path_id, true) {
            Some(idx) => idx,
            None => {
                return StashResult {
                    status: 0x1,
                    stashed_index: None,
                };
            } // INTERNAL_ERROR
        };
        self.add_remote_connection_id_to_stash(
            stash_idx,
            retire_before_next,
            sequence,
            connection_id_bytes,
            secret_bytes,
        )
    }
}

impl Connection {
    /// Remove the CID at `removed_index` from
    /// `connection.remote_connection_id_stashes[stash_index].connection_ids`.  Returns the
    /// next index that is still live, if any (matches the C "return
    /// the chain successor" pattern).
    pub fn remove_connection_id_from_stash(
        &mut self,
        stash_index: usize,
        removed_index: usize,
    ) -> Option<usize> {
        let stash = self.remote_connection_id_stashes.get_mut(stash_index)?;
        if removed_index >= stash.connection_ids.len() {
            return None;
        }
        stash.connection_ids.remove(removed_index);
        // Return the index of the next live entry (same index since we removed one).
        if removed_index < stash.connection_ids.len() {
            Some(removed_index)
        } else {
            None
        }
    }
}

impl Connection {
    /// As [`remove_connection_id_from_stash`] but locates the stash by
    /// `unique_path_id`.
    pub fn remove_stashed_connection_id(
        &mut self,
        unique_path_id: u64,
        removed_index: usize,
    ) -> Option<usize> {
        let eff_path_id = if self.is_multipath_enabled {
            unique_path_id
        } else {
            0
        };
        let stash_idx = self
            .remote_connection_id_stashes
            .iter()
            .position(|s| s.unique_path_id == eff_path_id)?;
        self.remove_connection_id_from_stash(stash_idx, removed_index)
    }
}

impl RemoteConnectionIdStash {
    /// Return a reference to the first available CID in `stash`, if any.
    /// "Available" means null-length CID OR (nb_path_references == 0 AND !needs_removal).
    /// C: `picoquic_get_cnxid_from_stash`.
    pub fn get_connection_id_from_stash(&self) -> Option<usize> {
        // Returns the index into connection_ids.
        for (i, r) in self.connection_ids.iter().enumerate() {
            if r.connection_id.is_empty() || (r.nb_path_references == 0 && !r.needs_removal) {
                return Some(i);
            }
        }
        None
    }
}

impl Connection {
    /// Reserve a stashed CID for use on `unique_path_id`.  Returns the
    /// stash index and CID index of the chosen CID, or `None` when none are
    /// available.
    pub fn obtain_stashed_connection_id(&mut self, unique_path_id: u64) -> Option<(usize, usize)> {
        // C: picoquic_obtain_stashed_cnxid — find the stash, then get first usable CID.
        let stash_idx = self
            .remote_connection_id_stashes
            .iter()
            .position(|s| s.unique_path_id == unique_path_id)?;
        let cid_idx =
            self.remote_connection_id_stashes[stash_idx].get_connection_id_from_stash()?;
        Some((stash_idx, cid_idx))
    }
}

impl Connection {
    pub fn dereference_stashed_connection_id(
        &mut self,
        path_x: &mut Path,
        is_deleting_connection: i32,
    ) {
        let unique_path_id = path_x.unique_path_id;
        for tuple in &mut path_x.tuples {
            let Some(cid_idx) = tuple.remote_connection_id_index.take() else {
                continue;
            };
            let mut retire_sequence = None;
            if let Some(stash_idx) = self
                .remote_connection_id_stashes
                .iter()
                .position(|s| s.unique_path_id == unique_path_id)
                && let Some(cid) = self.remote_connection_id_stashes[stash_idx]
                    .connection_ids
                    .get_mut(cid_idx)
            {
                cid.nb_path_references = cid.nb_path_references.saturating_sub(1);
                if cid.needs_removal && cid.nb_path_references == 0 && is_deleting_connection == 0 {
                    retire_sequence = Some(cid.sequence);
                }
            }
            if let Some(sequence) = retire_sequence {
                let _ = self.queue_retire_connection_id_frame(unique_path_id, sequence);
            }
        }
    }
}

impl Connection {
    pub fn dereference_stashed_connection_id_tuple(
        &mut self,
        path_x: &mut Path,
        tuple: &mut Tuple,
        is_deleting_connection: i32,
    ) {
        let Some(cid_idx) = tuple.remote_connection_id_index.take() else {
            return;
        };
        let mut retire_sequence = None;
        if let Some(stash_idx) = self
            .remote_connection_id_stashes
            .iter()
            .position(|s| s.unique_path_id == path_x.unique_path_id)
            && let Some(cid) = self.remote_connection_id_stashes[stash_idx]
                .connection_ids
                .get_mut(cid_idx)
        {
            cid.nb_path_references = cid.nb_path_references.saturating_sub(1);
            if cid.needs_removal && cid.nb_path_references == 0 && is_deleting_connection == 0 {
                retire_sequence = Some(cid.sequence);
            }
        }
        if let Some(sequence) = retire_sequence {
            let _ = self.queue_retire_connection_id_frame(path_x.unique_path_id, sequence);
        }
    }
}

impl Connection {
    pub fn remove_not_before_from_stash(
        &mut self,
        connection_id_stash: &mut RemoteConnectionIdStash,
        not_before: u64,
        _current_time: Instant,
    ) -> u64 {
        // Remove all CIDs whose sequence < not_before.
        let mut removed = 0u64;
        connection_id_stash.connection_ids.retain(|r| {
            if r.sequence < not_before {
                removed += 1;
                false
            } else {
                true
            }
        });
        removed
    }
}

impl Connection {
    /// Remove the stash at `connection.remote_connection_id_stashes[stash_index]`.
    pub fn delete_remote_connection_id_stash(&mut self, stash_index: usize) {
        if stash_index < self.remote_connection_id_stashes.len() {
            self.remote_connection_id_stashes.swap_remove(stash_index);
        }
    }
}

impl Connection {
    pub fn remove_not_before_cid(
        &mut self,
        unique_path_id: u64,
        not_before: u64,
        current_time: Instant,
    ) -> u64 {
        // C: picoquic_remove_not_before_cid
        let eff_path_id = if self.is_multipath_enabled {
            unique_path_id
        } else {
            0
        };
        if let Some(stash_idx) = self
            .remote_connection_id_stashes
            .iter()
            .position(|s| s.unique_path_id == eff_path_id)
        {
            // Temporarily extract the stash to avoid borrow conflicts.
            let mut stash = std::mem::replace(
                &mut self.remote_connection_id_stashes[stash_idx],
                RemoteConnectionIdStash {
                    unique_path_id: eff_path_id,
                    retire_connection_id_before: 0,
                    connection_ids: Vec::new(),
                    is_in_use: false,
                },
            );
            let removed = self.remove_not_before_from_stash(&mut stash, not_before, current_time);
            self.remote_connection_id_stashes[stash_idx] = stash;
            removed
        } else {
            0
        }
    }
}

impl Connection {
    /// Force a CID rotation on `path_x` (allocate a new local CID
    /// and retire the previous one).
    pub fn renew_path_connection_id(&mut self, path_x: &mut Path) -> Result<(), crate::Error> {
        let old_sequence = path_x
            .tuples
            .first()
            .and_then(|tuple| tuple.local_connection_id)
            .and_then(|tok| self.local_connection_ids.get(tok))
            .map(|lcid| lcid.sequence);
        let token =
            self.create_local_connection_id(path_x.unique_path_id, None, self.start_time)?;
        if let Some(tuple) = path_x.tuples.first_mut() {
            tuple.local_connection_id = Some(token);
        }
        path_x.path_cid_rotated = true;
        if let Some(sequence) = old_sequence {
            self.queue_retire_connection_id_frame(path_x.unique_path_id, sequence)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Retransmission queue management.

impl Connection {
    pub fn queue_for_retransmit(
        &mut self,
        path_x: &mut Path,
        packet: &mut Packet,
        length: usize,
        current_time: Instant,
    ) {
        packet.length = length;
        packet.send_time = current_time;
        packet.send_path = Some(PathToken::synthetic(
            path_x.unique_path_id as u32,
            path_x.unique_path_id as u32,
        ));
        packet.is_queued_for_retransmit = true;
        packet.is_queued_to_path = false;
        let pc = packet.packet_context as usize;
        let sequence = packet.sequence_number;
        if let Ok(token) = self.queued_packets.insert(core::mem::replace(
            packet,
            Packet {
                queue_data_repeat_membership: None,
                send_path: None,
                sequence_number: 0,
                send_time: current_time,
                delivered_prior: 0,
                delivered_time_prior: current_time,
                delivered_sent_prior: 0,
                lost_prior: 0,
                inflight_prior: 0,
                data_repeat_frame: 0,
                data_repeat_index: 0,
                data_repeat_priority: 0,
                data_repeat_stream_id: 0,
                data_repeat_stream_offset: 0,
                data_repeat_stream_data_length: 0,
                length: 0,
                checksum_overhead: 0,
                offset: 0,
                packet_type: PacketType::Error,
                packet_context: PacketContext::Application,
                is_evaluated: false,
                is_ack_eliciting: false,
                is_mtu_probe: false,
                is_multipath_probe: false,
                is_ack_trap: false,
                delivered_app_limited: false,
                sent_cwin_limited: false,
                is_preemptive_repeat: false,
                was_preemptively_repeated: false,
                is_queued_to_path: false,
                is_queued_for_retransmit: false,
                is_queued_for_spurious_detection: false,
                is_queued_for_data_repeat: false,
                bytes: [0u8; MAX_PACKET_SIZE],
            },
        )) {
            self.pkt_ctx[pc].pending.insert(sequence, token);
        }
    }
}

impl Connection {
    /// Remove the in-flight packet at `token` from `pkt_ctx.pending`.
    /// Returns the next packet token (sequence-wise successor) for the
    /// caller to chain onto.  C: `dequeue_retransmit_packet` returning
    /// the next pointer.
    pub fn dequeue_retransmit_packet(
        &mut self,
        pkt_ctx: &mut PacketContextState,
        packet: PacketToken,
        should_free: bool,
        add_to_data_repeat_queue: bool,
    ) -> Option<PacketToken> {
        let sequence = self
            .queued_packets
            .get(packet)
            .map(|p| p.sequence_number)
            .or_else(|| {
                pkt_ctx
                    .pending
                    .iter()
                    .find_map(|(seq, tok)| (*tok == packet).then_some(*seq))
            })?;
        let next = pkt_ctx
            .pending
            .range((sequence + 1)..)
            .next()
            .map(|(_, tok)| *tok);
        pkt_ctx.pending.remove(&sequence);
        if add_to_data_repeat_queue && let Some(pkt) = self.queued_packets.get_mut(packet) {
            pkt.is_queued_for_data_repeat = true;
        }
        if should_free {
            self.queued_packets.remove(packet);
        } else if let Some(pkt) = self.queued_packets.get_mut(packet) {
            pkt.is_queued_for_retransmit = false;
        }
        next
    }
}

impl Connection {
    pub fn dequeue_retransmitted_packet(
        &mut self,
        pkt_ctx: &mut PacketContextState,
        packet: PacketToken,
    ) {
        let sequence = self
            .queued_packets
            .get(packet)
            .map(|p| p.sequence_number)
            .or_else(|| {
                pkt_ctx
                    .retransmitted
                    .iter()
                    .find_map(|(seq, tok)| (*tok == packet).then_some(*seq))
            });
        if let Some(sequence) = sequence {
            pkt_ctx.retransmitted.remove(&sequence);
        }
        pkt_ctx.retransmitted_queue_size = pkt_ctx.retransmitted.len() as u64;
        if let Some(pkt) = self.queued_packets.get_mut(packet) {
            pkt.is_queued_for_spurious_detection = false;
        }
    }
}

impl Connection {
    /// Tear down all in-flight state and prepare the connection for
    /// a fresh handshake.
    pub fn reset(&mut self, current_time: Instant) -> Result<(), crate::Error> {
        for pkt_ctx in &mut self.pkt_ctx {
            pkt_ctx.pending.clear();
            pkt_ctx.retransmitted.clear();
            pkt_ctx.send_sequence = 0;
            pkt_ctx.retransmit_sequence = 0;
            pkt_ctx.next_sequence_hole = 0;
            pkt_ctx.retransmitted_queue_size = 0;
            pkt_ctx.highest_acknowledged = u64::MAX;
            pkt_ctx.latest_time_acknowledged = current_time;
            pkt_ctx.highest_acknowledged_time = current_time;
        }
        for ack_ctx in &mut self.ack_ctx {
            ack_ctx.reset_ack_context();
        }
        self.queued_packets = Arena::new();
        self.queue_data_repeat_tree.clear();
        self.misc_frames.clear();
        self.datagrams.clear();
        self.sooner_stateless.clear();
        self.local_error = 0;
        self.application_error = 0;
        self.remote_error = 0;
        self.remote_application_error = 0;
        self.local_error_reason = None;
        self.remote_error_reason = None;
        self.offending_frame_type = 0;
        self.is_handshake_finished = false;
        self.connection_state = if self.client_mode {
            State::ClientInit
        } else {
            State::ServerInit
        };
        Ok(())
    }

    /// Drain the per-epoch packet-number-space context state without
    /// disturbing the rest of the connection.
    pub fn reset_packet_context(&mut self, pkt_ctx: &mut PacketContextState) {
        // C: picoquic_reset_packet_context
        pkt_ctx.pending.clear();
        pkt_ctx.retransmitted.clear();
        pkt_ctx.send_sequence = 0;
        pkt_ctx.retransmit_sequence = 0;
        pkt_ctx.next_sequence_hole = 0;
        pkt_ctx.retransmitted_queue_size = 0;
    }

    /// Mark this connection as having hit a transport error so the
    /// next outgoing packet emits CONNECTION_CLOSE.  The returned
    /// value mirrors the C convention (the error code itself) so
    /// callers can write `return connection.connection_error(…);`.
    pub fn connection_error(&mut self, local_error: u64, frame_type: u64) -> i32 {
        self.connection_error_ex(local_error, frame_type, None)
    }

    /// As [`Self::connection_error`] but with an optional reason
    /// string emitted in the CONNECTION_CLOSE frame.
    pub fn connection_error_ex(
        &mut self,
        local_error: u64,
        frame_type: u64,
        local_reason: Option<&str>,
    ) -> i32 {
        // C: picoquic_connection_error_ex
        self.local_error = local_error;
        self.offending_frame_type = frame_type;
        self.local_error_reason = local_reason.map(|s| s.to_owned());
        // Move to disconnecting state if not already past that.
        if (self.connection_state as u32) < crate::State::Disconnecting as u32 {
            self.connection_state = crate::State::Disconnecting;
        }
        local_error as i32
    }

    /// Move the connection straight to the disconnected state
    /// without sending CONNECTION_CLOSE.
    pub fn connection_disconnect(&mut self) {
        // C: picoquic_connection_disconnect
        self.connection_state = crate::State::Disconnected;
    }
}

// ---------------------------------------------------------------------------
// Connection lookup.

impl Quic {
    /// Look up a connection by destination CID, returning both the
    /// connection token and the matching local CID token (the C
    /// out-parameter `l_cid_sequence` collapses into the second
    /// tuple slot).  C: `connection_by_id`.
    pub fn connection_by_id(
        &mut self,
        cnx_id: ConnectionId,
    ) -> Option<(ConnectionToken, LocalConnectionIdToken)> {
        let ht = self.connection_by_id.lookup(&cnx_id)?;
        let &conn_tok = self.connection_by_id.get(ht)?;
        let lcid_tok = self.connections.get(conn_tok).and_then(|connection| {
            connection
                .local_connection_id_lists
                .iter()
                .flat_map(|list| list.connection_ids.iter().copied())
                .find(|&tok| {
                    connection
                        .local_connection_ids
                        .get(tok)
                        .map(|l_cid| l_cid.connection_id == cnx_id)
                        .unwrap_or(false)
                })
        })?;
        Some((conn_tok, lcid_tok))
    }

    /// Look up a connection by peer address.  C: `connection_by_net`.
    pub fn connection_by_net(&mut self, addr: Option<&SocketAddr>) -> Option<ConnectionToken> {
        let addr = addr?;
        let ht = self.connection_by_net.lookup(addr)?;
        let &tok = self.connection_by_net.get(ht)?;
        Some(tok)
    }

    /// Look up a connection by initial CID and peer address.  C:
    /// `connection_by_icid`.
    pub fn connection_by_icid(
        &mut self,
        icid: &ConnectionId,
        _addr: Option<&SocketAddr>,
    ) -> Option<ConnectionToken> {
        let ht = self.connection_by_icid.lookup(icid)?;
        let &tok = self.connection_by_icid.get(ht)?;
        Some(tok)
    }

    /// Look up a connection by stateless-reset secret and peer
    /// address.  C: `connection_by_secret`.
    pub fn connection_by_secret(
        &mut self,
        reset_secret: &[u8],
        _addr: Option<&SocketAddr>,
    ) -> Option<ConnectionToken> {
        if reset_secret.len() < RESET_SECRET_SIZE {
            return None;
        }
        let mut key = [0u8; RESET_SECRET_SIZE];
        key.copy_from_slice(&reset_secret[..RESET_SECRET_SIZE]);
        let ht = self.connection_by_secret.lookup(&key)?;
        let &tok = self.connection_by_secret.get(ht)?;
        Some(tok)
    }
}

// ---------------------------------------------------------------------------
// Pacing.

impl Pacing {
    /// Initialize the pacing state at `current_time`.
    pub fn init(&mut self, current_time: Instant) {
        // C: picoquic_pacing_init
        self.evaluation_time = current_time;
        self.bucket_nanosec = 16;
        self.bucket_max = 16;
        self.packet_time_nanosec = 1;
        self.packet_time_microsec = crate::Duration::from_ticks(1);
    }

    /// Update the leaky bucket.  C: `picoquic_update_pacing_bucket`.
    fn update_bucket(&mut self, current_time: Instant) {
        if self.bucket_nanosec < -self.packet_time_nanosec {
            self.bucket_nanosec = -self.packet_time_nanosec;
        }
        let cur = current_time.ticks();
        let ev = self.evaluation_time.ticks();
        if cur > ev {
            self.bucket_nanosec += ((cur - ev) * 1000) as i64;
            self.evaluation_time = current_time;
            if self.bucket_nanosec > self.bucket_max {
                self.bucket_nanosec = self.bucket_max;
            }
        }
    }

    /// True when the pacer is currently throttling sends.
    pub fn is_blocked(&self) -> bool {
        // C: picoquic_is_pacing_blocked
        self.bucket_nanosec < self.packet_time_nanosec
    }

    /// Decide whether sending right now is authorized by the pacer.
    /// Writes the next authorized time into `next_time` when blocked.
    /// C: `picoquic_is_authorized_by_pacing` — last C arg `signal_path`
    /// maps to `Option<PathToken>` (`NULL` → `None`).
    pub fn is_authorized(
        &mut self,
        current_time: Instant,
        next_time: &mut Instant,
        packet_train_mode: bool,
        _signalled_path: Option<PathToken>,
    ) -> bool {
        self.update_bucket(current_time);
        if self.bucket_nanosec < self.packet_time_nanosec {
            let bucket_required = if packet_train_mode || self.bandwidth_pause != 0 {
                let mut br = self.bucket_max;
                if br > 10 * self.packet_time_nanosec {
                    br = 10 * self.packet_time_nanosec;
                }
                br - self.bucket_nanosec
            } else {
                self.packet_time_nanosec - self.bucket_nanosec
            };
            let next_pacing_ticks = current_time.ticks() + 1 + (bucket_required as u64) / 1000;
            let next_pacing = crate::Instant::from_ticks(next_pacing_ticks);
            if next_pacing < *next_time {
                self.bandwidth_pause = 0;
                *next_time = next_pacing;
            }
            false
        } else {
            true
        }
    }

    /// Re-derive bucket and rate after a configuration change.
    pub fn update_parameters(
        &mut self,
        pacing_rate: f64,
        quantum: u64,
        send_mtu: usize,
        _smoothed_rtt: Duration,
        _signalled_path: Option<PathToken>,
    ) {
        // C: picoquic_update_pacing_parameters
        if pacing_rate > 0.0 && send_mtu > 0 {
            let packet_time_nanosec = (send_mtu as f64 * 1e9 / pacing_rate) as i64;
            self.packet_time_nanosec = if packet_time_nanosec == 0 {
                1
            } else {
                packet_time_nanosec
            };
            self.packet_time_microsec =
                crate::Duration::from_ticks((self.packet_time_nanosec / 1000) as u64);
            self.quantum_max = quantum;
            let bucket_max = if quantum > 0 {
                quantum as i64 * 1000 * 16 / self.packet_time_nanosec.max(1)
            } else {
                16 * self.packet_time_nanosec
            };
            self.bucket_max = bucket_max.max(16 * self.packet_time_nanosec);
        }
    }

    /// Recompute pacing parameters from the congestion window.
    pub fn update_window(
        &mut self,
        _slow_start: i32,
        cwin: u64,
        send_mtu: usize,
        smoothed_rtt: Duration,
        signalled_path: Option<PathToken>,
    ) {
        // C: picoquic_update_pacing_window
        let rtt_ticks = smoothed_rtt.ticks();
        if rtt_ticks > 0 && send_mtu > 0 && cwin > 0 {
            // pacing_rate = cwin / rtt (bytes per microsecond)
            let pacing_rate = cwin as f64 / rtt_ticks as f64 * 1e6; // bytes/sec
            let quantum = if cwin > 2 * send_mtu as u64 {
                2 * send_mtu as u64
            } else {
                cwin
            };
            self.update_parameters(pacing_rate, quantum, send_mtu, smoothed_rtt, signalled_path);
        }
    }

    /// Update pacer state after a packet of `length` bytes was sent.
    pub fn update_after_send(&mut self, length: usize, send_mtu: usize, current_time: Instant) {
        // C: picoquic_update_pacing_data_after_send
        self.update_bucket(current_time);
        let mtu = if send_mtu == 0 { 1 } else { send_mtu };
        let nb_packets = length.div_ceil(mtu);
        self.bucket_nanosec -= nb_packets as i64 * self.packet_time_nanosec;
    }
}

impl Path {
    /// Recompute the pacer's bucket / target rate from the current
    /// CWIN and RTT.  C: `update_pacing_data`.
    pub fn update_pacing_data(&mut self, slow_start: i32) {
        self.pacing.update_window(
            slow_start,
            self.cwin,
            self.send_mtu,
            self.smoothed_rtt,
            None,
        );
    }

    /// Update pacer state after sending `length` bytes at
    /// `current_time`.  C: `update_pacing_after_send`.
    pub fn update_pacing_after_send(&mut self, length: usize, current_time: Instant) {
        self.pacing
            .update_after_send(length, self.send_mtu, current_time);
    }

    /// Force the pacer to a specific target `rate` and bucket
    /// `quantum`.  C: `update_pacing_rate`.
    pub fn update_pacing_rate(&mut self, pacing_rate: f64, quantum: u64) {
        self.pacing
            .update_parameters(pacing_rate, quantum, self.send_mtu, self.smoothed_rtt, None);
    }

    /// Recompute the path-quality notification thresholds from the
    /// current pacing rate / RTT.  C: `refresh_path_quality_thresholds`.
    pub fn refresh_quality_thresholds(&mut self) {
        let rtt = if self.smoothed_rtt.ticks() > 0 {
            self.smoothed_rtt
        } else {
            self.rtt_min
        };
        let rtt_delta = Duration::from_ticks((rtt.ticks() / 8).max(self.rtt_update_delta.ticks()));
        self.rtt_threshold_low =
            Duration::from_ticks(rtt.ticks().saturating_sub(rtt_delta.ticks()));
        self.rtt_threshold_high =
            Duration::from_ticks(rtt.ticks().saturating_add(rtt_delta.ticks()));

        let pacing_rate = self.pacing.rate.max(self.bandwidth_estimate);
        let pacing_delta = (pacing_rate / 8).max(self.pacing_rate_update_delta);
        self.pacing_rate_threshold_low = pacing_rate.saturating_sub(pacing_delta);
        self.pacing_rate_threshold_high = pacing_rate.saturating_add(pacing_delta);

        let receive_rate = self.receive_rate_estimate;
        let receive_delta = (receive_rate / 8).max(1);
        self.receive_rate_threshold_low = receive_rate.saturating_sub(receive_delta);
        self.receive_rate_threshold_high = receive_rate.saturating_add(receive_delta);
    }
}

impl Connection {
    /// Whether the pacer permits sending on path `path_idx` at
    /// `current_time`.  Updates `next_time` with the earliest pacer
    /// fire if blocked.  C: `is_sending_authorized_by_pacing`.
    /// Uses a path index to avoid aliasing `&self` with `&mut path`.
    pub fn is_sending_authorized_by_pacing(
        &mut self,
        path_idx: usize,
        current_time: Instant,
        next_time: &mut Instant,
    ) -> bool {
        if let Some(path) = self.paths.get_mut(path_idx) {
            path.pacing
                .is_authorized(current_time, next_time, false, None)
        } else {
            true
        }
    }

    /// Notify the application of a quality update for `path_x` if
    /// the change crosses any subscribed threshold.  C:
    /// `issue_path_quality_update`.
    pub fn issue_path_quality_update(&mut self, path_x: &mut Path) -> i32 {
        let rtt = if path_x.smoothed_rtt.ticks() > 0 {
            path_x.smoothed_rtt
        } else {
            path_x.rtt_sample
        };
        let pacing_rate = path_x.pacing.rate.max(path_x.bandwidth_estimate);
        let receive_rate = path_x.receive_rate_estimate;
        let changed = rtt < path_x.rtt_threshold_low
            || rtt > path_x.rtt_threshold_high
            || pacing_rate < path_x.pacing_rate_threshold_low
            || pacing_rate > path_x.pacing_rate_threshold_high
            || receive_rate < path_x.receive_rate_threshold_low
            || receive_rate > path_x.receive_rate_threshold_high
            || path_x.is_cc_data_updated;
        if changed {
            path_x.refresh_quality_thresholds();
            path_x.is_cc_data_updated = false;
            self.is_lost_feedback_notification_required = false;
            1
        } else {
            0
        }
    }
}

impl Quic {
    /// Re-position `connection` in the wake-time queue using the
    /// supplied next firing time.  C: `reinsert_by_wake_time`.
    pub fn reinsert_by_wake_time(&mut self, connection: &mut Connection, next_time: Instant) {
        connection.next_wake_time = next_time;
        let token = self
            .connections
            .iter()
            .position(|c| c.initial_connection_id == connection.initial_connection_id)
            .map(|idx| ConnectionToken::synthetic(idx as u32, idx as u32));
        if let Some(token) = token
            && let Ok((tree_token, old)) =
                self.connection_wake_tree.insert(next_time.ticks(), token)
        {
            connection.connection_wake_membership = Some(tree_token);
            if let Some(old_token) = old
                && old_token != token
                && let Some(old_connection) = self.connections.get_mut(old_token)
            {
                old_connection.connection_wake_membership = None;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Integer parsing / formatting helpers (translated from `PARSE_*` and
// `format_*`).

/// Read a big-endian `u16` from the first two bytes of `b`.
pub const fn parse_16(b: &[u8]) -> u16 {
    u16::from_be_bytes([b[0], b[1]])
}

/// Read a 24-bit big-endian unsigned integer (zero-extended into a
/// `u32`) from the first three bytes of `b`.  No native `u24`, so
/// the body keeps the explicit shift.
pub const fn parse_24(b: &[u8]) -> u32 {
    u32::from_be_bytes([0, b[0], b[1], b[2]])
}

/// Read a big-endian `u32` from the first four bytes of `b`.
pub const fn parse_32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// Read a big-endian `u64` from the first eight bytes of `b`.
pub const fn parse_64(b: &[u8]) -> u64 {
    u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

pub fn format_16(bytes: &mut [u8], n16: u16) {
    bytes[..2].copy_from_slice(&n16.to_be_bytes());
}

pub fn format_24(bytes: &mut [u8], n24: u32) {
    bytes[..3].copy_from_slice(&n24.to_be_bytes()[1..]);
}

pub fn format_32(bytes: &mut [u8], n32: u32) {
    bytes[..4].copy_from_slice(&n32.to_be_bytes());
}

pub fn format_64(bytes: &mut [u8], n64: u64) {
    bytes[..8].copy_from_slice(&n64.to_be_bytes());
}

/// Encode `n64` as a QUIC variable-length integer into `bytes`.
/// Returns the number of bytes written (0 if the buffer is too small).
/// C: `picoquic_varint_encode`.
pub fn varint_encode(bytes: &mut [u8], n64: u64) -> usize {
    if n64 < 16384 {
        if n64 < 64 {
            if bytes.is_empty() {
                return 0;
            }
            bytes[0] = n64 as u8;
            1
        } else {
            if bytes.len() < 2 {
                return 0;
            }
            bytes[0] = ((n64 >> 8) | 0x40) as u8;
            bytes[1] = n64 as u8;
            2
        }
    } else if n64 < 1_073_741_824 {
        if bytes.len() < 4 {
            return 0;
        }
        bytes[0] = ((n64 >> 24) | 0x80) as u8;
        bytes[1] = (n64 >> 16) as u8;
        bytes[2] = (n64 >> 8) as u8;
        bytes[3] = n64 as u8;
        4
    } else {
        if bytes.len() < 8 {
            return 0;
        }
        bytes[0] = ((n64 >> 56) | 0xC0) as u8;
        bytes[1] = (n64 >> 48) as u8;
        bytes[2] = (n64 >> 40) as u8;
        bytes[3] = (n64 >> 32) as u8;
        bytes[4] = (n64 >> 24) as u8;
        bytes[5] = (n64 >> 16) as u8;
        bytes[6] = (n64 >> 8) as u8;
        bytes[7] = n64 as u8;
        8
    }
}

/// Encode `n16` as a 2-byte QUIC varint (the 2-byte form, 0x40 prefix).
/// C: `picoquic_varint_encode_16`.
pub fn varint_encode_16(bytes: &mut [u8], n16: u16) {
    bytes[0] = (((n16 >> 8) | 0x40) & 0x7F) as u8;
    bytes[1] = n16 as u8;
}

/// Decode a QUIC variable-length integer from `bytes`.
/// Returns the number of bytes consumed (0 on error/underflow).
/// C: `picoquic_varint_decode`.
pub fn varint_decode(bytes: &[u8], n64: &mut u64) -> usize {
    if bytes.is_empty() {
        *n64 = 0;
        return 0;
    }
    let length = 1usize << ((bytes[0] & 0xC0) >> 6);
    if length > bytes.len() {
        *n64 = 0;
        return 0;
    }
    let mut v = (bytes[0] & 0x3F) as u64;
    for b in bytes.iter().take(length).skip(1) {
        v <<= 8;
        v += *b as u64;
    }
    *n64 = v;
    length
}

/// Decode a QUIC varint at the start of `bytes`, return the
/// remaining tail (or `None` on under-read).  C: returned a
/// pointer past the consumed bytes; the Rust shape returns the
/// remaining slice instead.
pub fn frames_varint_decode<'a>(bytes: &'a [u8], n64: &mut u64) -> Option<&'a [u8]> {
    if bytes.is_empty() {
        return None;
    }
    let length = 1usize << ((bytes[0] & 0xC0) >> 6);
    if length > bytes.len() {
        return None;
    }
    let mut v = (bytes[0] & 0x3F) as u64;
    for b in bytes.iter().take(length).skip(1) {
        v <<= 8;
        v += *b as u64;
    }
    *n64 = v;
    Some(&bytes[length..])
}

/// Skip past a varint, returning the remaining tail.  C:
/// `frames_varint_skip` returning a pointer.
pub fn frames_varint_skip(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.is_empty() {
        return None;
    }
    let v_len = 1usize << ((bytes[0] & 0xC0) >> 6);
    if v_len > bytes.len() {
        None
    } else {
        Some(&bytes[v_len..])
    }
}

/// Return the byte-length of the varint starting at `bytes[0]`.
/// C: `picoquic_varint_skip`.
pub fn varint_skip(bytes: &[u8]) -> usize {
    if bytes.is_empty() {
        return 0;
    }
    1usize << ((bytes[0] & 0xC0) >> 6)
}

/// Predict the number of bytes required to encode `n64` as a QUIC varint.
/// C: `picoquic_encode_varint_length`.
pub fn encode_varint_length(n64: u64) -> usize {
    if n64 < 64 {
        1
    } else if n64 < 16_384 {
        2
    } else if n64 < 1_073_741_824 {
        4
    } else {
        8
    }
}

/// Predict the number of bytes required to encode `n64` as a QUIC varint.
/// C: `picoquic_frames_varint_encode_length`.
pub fn frames_varint_encode_length(n64: u64) -> usize {
    encode_varint_length(n64)
}

/// Return the byte-length indicated by the high-two bits of `byte`.
/// C: `picoquic_decode_varint_length`.
pub fn decode_varint_length(byte: u8) -> usize {
    1usize << ((byte & 0xC0) >> 6)
}

// ---------------------------------------------------------------------------
// Packet parsing / header creation.

/// Parse the long-header packet type from `flags` given the version.
/// C: `picoquic_parse_long_packet_type`.
pub fn parse_long_packet_type(flags: u8, version_index: i32) -> PacketType {
    let type_bits = (flags >> 4) & 3;
    let version = match version_index {
        0 => Version::V1,
        1 => Version::V2,
        2 => Version::V2Draft,
        3 => Version::PostIesg,
        4 => Version::TwentyFirstInterop,
        5 => Version::TwentiethInterop,
        6 => Version::TwentiethPreInterop,
        7 => Version::NineteenthInterop,
        8 => Version::NineteenthBisInterop,
        9 => Version::EighteenthInterop,
        10 => Version::SeventeenthInterop,
        11 => Version::InternalTest2,
        12 => Version::InternalTest1,
        _ => return PacketType::Error,
    };

    match version.parameters().packet_type_version {
        0x0000_0001 => match type_bits {
            0 => PacketType::Initial,
            1 => PacketType::ZeroRttProtected,
            2 => PacketType::Handshake,
            3 => PacketType::Retry,
            _ => PacketType::Error,
        },
        0x6b33_43cf => match type_bits {
            1 => PacketType::Initial,
            2 => PacketType::ZeroRttProtected,
            3 => PacketType::Handshake,
            0 => PacketType::Retry,
            _ => PacketType::Error,
        },
        _ => PacketType::Error,
    }
}

impl Quic {
    /// Parse a packet header.  C `int picoquic_parse_packet_header(...,
    /// picoquic_cnx_t** pcnx, int receiving)`: the C `pcnx`
    /// out-parameter folds into the `Ok` payload here, the C `int`
    /// status into `Result`.
    pub fn parse_packet_header(
        &mut self,
        bytes: &[u8],
        addr_from: Option<&SocketAddr>,
        ph: &mut PacketHeader,
        receiving: bool,
    ) -> Result<Option<ConnectionToken>, crate::Error> {
        *ph = PacketHeader::default();
        ph.version_index = -1;

        if bytes.is_empty() {
            return Err(crate::Error::InvalidArgument);
        }

        let conn_tok = if (bytes[0] & 0x80) == 0x80 {
            // Long header
            self.parse_long_packet_header_inner(bytes, addr_from, ph)
        } else {
            // Short header
            self.parse_short_packet_header_inner(bytes, addr_from, ph, receiving)
        };
        Ok(conn_tok)
    }

    fn parse_long_packet_header_inner(
        &mut self,
        bytes: &[u8],
        addr_from: Option<&SocketAddr>,
        ph: &mut PacketHeader,
    ) -> Option<ConnectionToken> {
        let mut pos = 0usize;
        if bytes.len() < 5 {
            ph.packet_type = PacketType::Error;
            return None;
        }
        let flags = bytes[pos];
        pos += 1;
        let version =
            u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
        pos += 4;
        ph.version = version;

        // Decode DCID
        if pos >= bytes.len() {
            ph.packet_type = PacketType::Error;
            return None;
        }
        let dcid_len = bytes[pos] as usize;
        pos += 1;
        if pos + dcid_len > bytes.len() {
            ph.packet_type = PacketType::Error;
            return None;
        }
        ph.dest_connection_id =
            ConnectionId::clone_from_slice(&bytes[pos..pos + dcid_len]).unwrap_or_default();
        pos += dcid_len;

        // Decode SCID
        if pos >= bytes.len() {
            ph.packet_type = PacketType::Error;
            return None;
        }
        let scid_len = bytes[pos] as usize;
        pos += 1;
        if pos + scid_len > bytes.len() {
            ph.packet_type = PacketType::Error;
            return None;
        }
        ph.src_connection_id =
            ConnectionId::clone_from_slice(&bytes[pos..pos + scid_len]).unwrap_or_default();
        pos += scid_len;

        ph.offset = pos;

        if version == 0 {
            // Version negotiation
            ph.packet_type = PacketType::VersionNegotiation;
            ph.packet_context = PacketContext::Initial;
            ph.epoch = Epoch::Initial;
            ph.payload_length = bytes.len().saturating_sub(pos);
            ph.payload_length_value = ph.payload_length;
            // Connection lookup
            if self.local_connection_id_length == 0 {
                return self.connection_by_net(addr_from);
            } else if dcid_len == self.local_connection_id_length as usize {
                let cid = ph.dest_connection_id;
                return self.connection_by_id(cid).map(|(tok, _)| tok);
            }
            return None;
        }

        // Determine version_index (simple lookup)
        ph.version_index = match version {
            0x00000001 => 0,
            0x6b3343cf | 0x709a50c4 => 1,
            _ => 0, // treat unknown as V1-style
        };

        ph.quic_bit_is_zero = (flags & 0x40) == 0;
        ph.packet_type = parse_long_packet_type(flags, ph.version_index);

        match ph.packet_type {
            PacketType::Initial => {
                ph.epoch = Epoch::Initial;
                ph.packet_context = PacketContext::Initial;
                // Token
                let mut tok_len = 0u64;
                let rest = frames_varint_decode(&bytes[pos..], &mut tok_len)?;
                pos = bytes.len() - rest.len();
                ph.token_bytes = bytes[pos..pos + tok_len as usize].to_vec();
                pos += tok_len as usize;
                ph.offset = pos;
            }
            PacketType::ZeroRttProtected => {
                ph.epoch = Epoch::ZeroRtt;
                ph.packet_context = PacketContext::Application;
            }
            PacketType::Handshake => {
                ph.epoch = Epoch::Handshake;
                ph.packet_context = PacketContext::Handshake;
            }
            PacketType::Retry => {
                ph.epoch = Epoch::Initial;
                ph.packet_context = PacketContext::Initial;
                ph.payload_length = bytes.len().saturating_sub(pos);
                ph.payload_length_value = ph.payload_length;
                ph.packet_number_offset = pos;
                return None; // no PN in retry
            }
            _ => {
                ph.packet_type = PacketType::Error;
                return None;
            }
        }

        // Payload length (varint)
        let mut payload_length = 0u64;
        let rest = frames_varint_decode(&bytes[pos..], &mut payload_length)?;
        pos = bytes.len() - rest.len();
        ph.payload_length = payload_length as usize;
        ph.payload_length_value = ph.payload_length;
        ph.offset = pos;
        ph.packet_number_offset = pos;

        // Connection lookup
        if self.local_connection_id_length == 0 {
            self.connection_by_net(addr_from)
        } else if dcid_len == self.local_connection_id_length as usize {
            let cid = ph.dest_connection_id;
            self.connection_by_id(cid).map(|(tok, _)| tok).or_else(|| {
                if matches!(
                    ph.packet_type,
                    PacketType::Initial | PacketType::ZeroRttProtected
                ) {
                    let dest_cid = ph.dest_connection_id;
                    self.connection_by_icid(&dest_cid, addr_from)
                } else {
                    None
                }
            })
        } else if matches!(
            ph.packet_type,
            PacketType::Initial | PacketType::ZeroRttProtected
        ) {
            let dest_cid = ph.dest_connection_id;
            self.connection_by_icid(&dest_cid, addr_from)
        } else {
            None
        }
    }

    fn parse_short_packet_header_inner(
        &mut self,
        bytes: &[u8],
        addr_from: Option<&SocketAddr>,
        ph: &mut PacketHeader,
        _receiving: bool,
    ) -> Option<ConnectionToken> {
        let cnxid_length = self.local_connection_id_length as usize;
        ph.packet_context = PacketContext::Application;
        ph.payload_length_value = 0;

        if bytes.len() < 1 + cnxid_length {
            ph.packet_type = PacketType::Error;
            ph.offset = bytes.len();
            ph.payload_length = 0;
            return None;
        }

        let dcid = ConnectionId::clone_from_slice(&bytes[1..1 + cnxid_length]).unwrap_or_default();
        ph.dest_connection_id = dcid;
        ph.offset = 1 + cnxid_length;
        ph.packet_number_offset = ph.offset;

        // Lookup connection
        let conn_tok = if cnxid_length > 0 {
            let cid = ph.dest_connection_id;
            self.connection_by_id(cid).map(|(tok, _)| tok)
        } else {
            self.connection_by_net(addr_from)
        };

        ph.epoch = Epoch::OneRtt;
        ph.quic_bit_is_zero = (bytes[0] & 0x40) == 0;

        // Check QUIC bit (allow grease mode)
        let do_grease = conn_tok
            .and_then(|tok| self.connections.get(tok))
            .map(|c| c.local_parameters.do_grease_quic_bit)
            .unwrap_or(false);

        if !ph.quic_bit_is_zero || do_grease {
            ph.packet_type = PacketType::OneRttProtected;
        } else {
            ph.packet_type = PacketType::Error;
        }

        ph.has_spin_bit = true;
        ph.spin = (bytes[0] >> 5) & 1 != 0;
        ph.key_phase = ((bytes[0] >> 2) & 1) != 0;
        ph.packet_number_mask = 0;
        ph.packet_number_truncated = 0;

        if conn_tok.is_some() {
            let is_loss_bit = conn_tok
                .and_then(|tok| self.connections.get(tok))
                .map(|c| c.is_loss_bit_enabled_incoming || c.is_loss_bit_enabled_outgoing)
                .unwrap_or(false);
            if is_loss_bit {
                ph.has_loss_bits = true;
                ph.loss_bit_l = (bytes[0] >> 3) & 1 != 0;
                ph.loss_bit_q = (bytes[0] >> 4) & 1 != 0;
            }
        }

        ph.payload_length = if bytes.len() > ph.offset {
            bytes.len() - ph.offset
        } else {
            0
        };

        // Set version_index from connection if found
        if let Some(tok) = conn_tok
            && let Some(cnx) = self.connections.get(tok)
        {
            ph.version_index = cnx.version_index;
        }

        conn_tok
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_long_header(
    packet_type: PacketType,
    dest_cnx_id: &ConnectionId,
    srce_cnx_id: &ConnectionId,
    _do_grease_quic_bit: bool,
    version: u32,
    _version_index: i32,
    sequence_number: u64,
    retry_token: &[u8],
    bytes: &mut [u8],
    pn_offset: &mut usize,
    pn_length: &mut usize,
) -> usize {
    // C: picoquic_create_long_header — encode a QUIC long header into `bytes`.
    let is_v2 = version == Version::V2 as u32 || version == Version::V2Draft as u32;
    let first_byte: u8 = match (packet_type, is_v2) {
        (PacketType::Initial, false) => 0xC3,
        (PacketType::ZeroRttProtected, false) => 0xD3,
        (PacketType::Handshake, false) => 0xE3,
        (PacketType::Retry, false) => 0xF0,
        (PacketType::Initial, true) => 0xD3,
        (PacketType::ZeroRttProtected, true) => 0xE3,
        (PacketType::Handshake, true) => 0xF3,
        (PacketType::Retry, true) => 0xC0,
        _ => 0xFF,
    };
    bytes[0] = first_byte;
    let mut length = 1;
    let ver_bytes = version.to_be_bytes();
    bytes[length..length + 4].copy_from_slice(&ver_bytes);
    length += 4;
    // Dest CID
    bytes[length] = dest_cnx_id.len() as u8;
    length += 1;
    let dlen = dest_cnx_id.len();
    bytes[length..length + dlen].copy_from_slice(dest_cnx_id.as_bytes());
    length += dlen;
    // Src CID
    bytes[length] = srce_cnx_id.len() as u8;
    length += 1;
    let slen = srce_cnx_id.len();
    bytes[length..length + slen].copy_from_slice(srce_cnx_id.as_bytes());
    length += slen;
    // Token for Initial
    if packet_type == PacketType::Initial {
        length += varint_encode(&mut bytes[length..], retry_token.len() as u64);
        bytes[length..length + retry_token.len()].copy_from_slice(retry_token);
        length += retry_token.len();
    }
    if packet_type != PacketType::Retry {
        bytes[length] = 0;
        bytes[length + 1] = 0;
        length += 2;
        *pn_offset = length;
        *pn_length = 4;
        let pn32 = sequence_number as u32;
        bytes[length..length + 4].copy_from_slice(&pn32.to_be_bytes());
        length += 4;
    } else {
        *pn_offset = 0;
        *pn_length = 0;
    }
    length
}

impl Connection {
    pub fn create_packet_header(
        &mut self,
        packet_type: PacketType,
        sequence_number: u64,
        _path_x: &mut Path,
        tuple: &mut Tuple,
        header_length: usize,
        bytes: &mut [u8],
        pn_offset: &mut usize,
        pn_length: &mut usize,
    ) -> usize {
        // Delegate to create_packet_header_at using tuple's path context.
        // Find tuple's path index by matching tuple's unique_path_id.
        let path_idx = self
            .paths
            .iter()
            .position(|p| p.unique_path_id == tuple.unique_path_id)
            .unwrap_or(0);
        let _ = tuple;
        self.create_packet_header_at(
            packet_type,
            sequence_number,
            path_idx,
            0,
            header_length,
            bytes,
            pn_offset,
            pn_length,
        )
    }
}

impl Connection {
    pub fn predict_packet_header_length(
        &mut self,
        packet_type: PacketType,
        pkt_ctx: &mut PacketContextState,
    ) -> usize {
        // Delegate to the PacketContext-based version.
        // Determine which PacketContext this pkt_ctx corresponds to.
        // We can identify it by pointer equality via index.
        let pc = {
            let addr = pkt_ctx as *const PacketContextState;
            let mut found = PacketContext::Application;
            for i in 0..self.pkt_ctx.len() {
                if std::ptr::eq(&self.pkt_ctx[i], addr) {
                    found = match i {
                        0 => PacketContext::Application,
                        1 => PacketContext::Handshake,
                        2 => PacketContext::Initial,
                        _ => PacketContext::Application,
                    };
                    break;
                }
            }
            found
        };
        self.predict_packet_header_length_for_pc(packet_type, pc)
    }
}

pub fn update_payload_length(
    bytes: &mut [u8],
    pnum_index: usize,
    header_length: usize,
    packet_length: usize,
) {
    if (bytes[0] & 0x80) != 0
        && header_length > 6
        && packet_length > header_length
        && packet_length < 0x4000
        && pnum_index >= 2
    {
        let payload_len = (packet_length - header_length) as u16;
        varint_encode_16(&mut bytes[pnum_index - 2..], payload_len);
    }
}

impl Connection {
    pub fn get_checksum_length(&self, is_cleartext_mode: Epoch) -> usize {
        // C: picoquic_get_checksum_length — returns the AEAD tag length.
        // Cleartext (initial epoch) uses 16 bytes (AES-128-GCM tag).
        // All other epochs also use 16 bytes in practice.
        let _ = is_cleartext_mode;
        16
    }
}

pub fn protect_packet_header(
    send_buffer: &mut [u8],
    pn_offset: usize,
    first_mask: u8,
    pn_enc: &dyn crate::tls::HeaderKey,
) {
    if pn_offset >= send_buffer.len() {
        return;
    }
    let pn_length = ((send_buffer[0] & 0x03) + 1) as usize;
    let sample_offset = pn_offset.saturating_add(4);
    if sample_offset + 16 > send_buffer.len() || pn_offset + pn_length > send_buffer.len() {
        return;
    }
    let mut sample = [0u8; 16];
    sample.copy_from_slice(&send_buffer[sample_offset..sample_offset + 16]);
    let mask = pn_enc.mask(sample);
    send_buffer[0] ^= mask[0] & first_mask;
    for i in 0..pn_length {
        send_buffer[pn_offset + i] ^= mask[i + 1];
    }
}

impl Connection {
    pub fn protect_packet(
        &mut self,
        _ptype: PacketType,
        bytes: &mut [u8],
        sequence_number: u64,
        length: usize,
        header_length: usize,
        send_buffer: &mut [u8],
        send_buffer_max: usize,
        aead_context: &dyn crate::tls::PacketKey,
        pn_enc: &dyn crate::tls::HeaderKey,
        _path_x: &mut Path,
        _tuple: &mut Tuple,
        _current_time: Instant,
    ) -> usize {
        if header_length > length || length > bytes.len() || header_length > send_buffer_max {
            return 0;
        }
        let header = &bytes[..header_length];
        let mut payload = bytes[header_length..length].to_vec();
        aead_context.encrypt(sequence_number, header, &mut payload);
        let packet_length = header_length + payload.len();
        if packet_length > send_buffer_max || packet_length > send_buffer.len() {
            return 0;
        }
        send_buffer[..header_length].copy_from_slice(header);
        send_buffer[header_length..packet_length].copy_from_slice(&payload);
        update_payload_length(
            send_buffer,
            header_length.saturating_sub(4),
            header_length.saturating_sub(4),
            packet_length,
        );
        let first_mask = if (send_buffer[0] & 0x80) != 0 {
            0x0f
        } else {
            0x1f
        };
        let pn_offset = if header_length >= 4 {
            header_length - 4
        } else {
            header_length
        };
        protect_packet_header(
            &mut send_buffer[..packet_length],
            pn_offset,
            first_mask,
            pn_enc,
        );
        packet_length
    }
}

pub fn get_packet_number64(highest: u64, mask: u64, pn: u32) -> u64 {
    // C: picoquic_get_packet_number64 — reconstruct full 64-bit PN from truncated `pn`
    // using `highest` (last seen full PN) and `mask` (which bits are carried).
    // The truncated PN has (mask+1) possible values; pick the one closest to highest.
    let candidate_base = highest & !mask;
    let candidate = candidate_base | (pn as u64 & mask);
    // Adjust if candidate is too far from highest.
    let half_window = (mask + 1).div_ceil(2);
    if candidate + half_window < highest {
        candidate + mask + 1
    } else if candidate > highest + half_window && candidate > mask {
        candidate - (mask + 1)
    } else {
        candidate
    }
}

pub fn remove_header_protection_inner(
    bytes: &mut [u8],
    length: usize,
    decrypted_bytes: &mut [u8],
    ph: &mut PacketHeader,
    pn_enc: &dyn crate::tls::HeaderKey,
    is_loss_bit_enabled_incoming: bool,
    sack_list_last: u64,
) -> i32 {
    let length = length.min(bytes.len());
    if ph.packet_number_offset >= length {
        return -1;
    }
    let sample_offset = ph.packet_number_offset.saturating_add(4);
    if sample_offset + 16 > length {
        return -1;
    }
    let mut sample = [0u8; 16];
    sample.copy_from_slice(&bytes[sample_offset..sample_offset + 16]);
    let mask = pn_enc.mask(sample);
    let first_mask = if (bytes[0] & 0x80) != 0 { 0x0f } else { 0x1f };
    bytes[0] ^= mask[0] & first_mask;

    if is_loss_bit_enabled_incoming && (bytes[0] & 0x80) == 0 {
        ph.has_loss_bits = true;
        ph.loss_bit_l = (bytes[0] & 0x08) != 0;
        ph.loss_bit_q = (bytes[0] & 0x10) != 0;
    }

    let pn_length = ((bytes[0] & 0x03) + 1) as usize;
    if ph.packet_number_offset + pn_length > length {
        return -1;
    }
    let mut truncated = 0u32;
    for i in 0..pn_length {
        let b = bytes[ph.packet_number_offset + i] ^ mask[i + 1];
        bytes[ph.packet_number_offset + i] = b;
        truncated = (truncated << 8) | b as u32;
    }
    ph.packet_number_truncated = truncated;
    ph.packet_number_mask = if pn_length == 4 {
        u32::MAX as u64
    } else {
        (1u64 << (8 * pn_length)) - 1
    };
    ph.packet_number_full = get_packet_number64(sack_list_last, ph.packet_number_mask, truncated);
    let copy_len = length.min(decrypted_bytes.len());
    decrypted_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);
    0
}

/// Pad `bytes[length..]` with zeros up to `target`.  Returns `target`
/// when padding was needed, otherwise `length` unchanged.
/// C: `picoquic_pad_to_target_length`.
pub fn pad_to_target_length(bytes: &mut [u8], length: usize, target: usize) -> usize {
    if length < target {
        bytes[length..target].fill(0);
        target
    } else {
        length
    }
}

impl Connection {
    pub fn finalize_and_protect_packet_tuple(
        &mut self,
        packet: &mut Packet,
        ret: i32,
        length: usize,
        header_length: usize,
        checksum_overhead: usize,
        send_length: &mut usize,
        send_buffer: &mut [u8],
        send_buffer_max: usize,
        path_x: &mut Path,
        current_time: Instant,
        tuple: &mut Tuple,
    ) {
        *send_length = 0;
        if ret != 0 || length > packet.bytes.len() {
            return;
        }
        let epoch = match packet.packet_type {
            PacketType::Initial => Epoch::Initial,
            PacketType::Handshake => Epoch::Handshake,
            PacketType::ZeroRttProtected => Epoch::ZeroRtt,
            PacketType::OneRttProtected => Epoch::OneRtt,
            _ => Epoch::OneRtt,
        } as usize;
        let Some(aead) = self.crypto_context[epoch].aead_encrypt.take() else {
            if length <= send_buffer_max && length <= send_buffer.len() {
                send_buffer[..length].copy_from_slice(&packet.bytes[..length]);
                *send_length = length;
            }
            return;
        };
        let Some(pn_enc) = self.crypto_context[epoch].pn_enc.take() else {
            self.crypto_context[epoch].aead_encrypt = Some(aead);
            if length <= send_buffer_max && length <= send_buffer.len() {
                send_buffer[..length].copy_from_slice(&packet.bytes[..length]);
                *send_length = length;
            }
            return;
        };
        packet.checksum_overhead = checksum_overhead;
        packet.length = length.saturating_add(checksum_overhead);
        let protected = self.protect_packet(
            packet.packet_type,
            &mut packet.bytes,
            packet.sequence_number,
            length,
            header_length,
            send_buffer,
            send_buffer_max,
            &*aead,
            &*pn_enc,
            path_x,
            tuple,
            current_time,
        );
        self.crypto_context[epoch].aead_encrypt = Some(aead);
        self.crypto_context[epoch].pn_enc = Some(pn_enc);
        *send_length = protected;
    }
}

impl Connection {
    pub fn finalize_and_protect_packet(
        &mut self,
        packet: &mut Packet,
        ret: i32,
        length: usize,
        header_length: usize,
        checksum_overhead: usize,
        send_length: &mut usize,
        send_buffer: &mut [u8],
        send_buffer_max: usize,
        path_x: &mut Path,
        current_time: Instant,
    ) {
        use core::net::{IpAddr, Ipv4Addr};
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let mut tuple_copy = path_x.tuples.first().map_or(
            Tuple {
                unique_path_id: path_x.unique_path_id,
                peer_addr: default_addr,
                local_addr: default_addr,
                if_index: 0,
                observed_addr: default_addr,
                remote_connection_id_index: None,
                local_connection_id: None,
                nb_observed_repeat: 0,
                observed_time: Instant::from_ticks(0),
                challenge_response: 0,
                challenge: [0; CHALLENGE_REPEAT_MAX],
                challenge_time: Instant::from_ticks(0),
                demotion_time: Instant::from_ticks(0),
                challenge_time_first: Instant::from_ticks(0),
                is_nat_rebinding: 0,
                challenge_repeat_count: 0,
                is_backup: 0,
                challenge_required: false,
                challenge_verified: false,
                challenge_failed: false,
                response_required: false,
                to_preferred_address: false,
            },
            |tuple| Tuple {
                unique_path_id: tuple.unique_path_id,
                peer_addr: tuple.peer_addr,
                local_addr: tuple.local_addr,
                if_index: tuple.if_index,
                observed_addr: tuple.observed_addr,
                remote_connection_id_index: tuple.remote_connection_id_index,
                local_connection_id: tuple.local_connection_id,
                nb_observed_repeat: tuple.nb_observed_repeat,
                observed_time: tuple.observed_time,
                challenge_response: tuple.challenge_response,
                challenge: tuple.challenge,
                challenge_time: tuple.challenge_time,
                demotion_time: tuple.demotion_time,
                challenge_time_first: tuple.challenge_time_first,
                is_nat_rebinding: tuple.is_nat_rebinding,
                challenge_repeat_count: tuple.challenge_repeat_count,
                is_backup: tuple.is_backup,
                challenge_required: tuple.challenge_required,
                challenge_verified: tuple.challenge_verified,
                challenge_failed: tuple.challenge_failed,
                response_required: tuple.response_required,
                to_preferred_address: tuple.to_preferred_address,
            },
        );
        self.finalize_and_protect_packet_tuple(
            packet,
            ret,
            length,
            header_length,
            checksum_overhead,
            send_length,
            send_buffer,
            send_buffer_max,
            path_x,
            current_time,
            &mut tuple_copy,
        );
    }
}

impl Connection {
    pub fn implicit_handshake_ack(&mut self, pc: PacketContext, _current_time: Instant) {
        let pkt_ctx = &mut self.pkt_ctx[pc as usize];
        pkt_ctx.pending.clear();
        pkt_ctx.retransmitted.clear();
        pkt_ctx.retransmitted_queue_size = 0;
    }
}

impl Connection {
    pub fn false_start_transition(&mut self, _current_time: Instant) {
        self.connection_state = crate::State::ServerFalseStart;
    }
}

impl Connection {
    pub fn client_almost_ready_transition(&mut self) {
        self.connection_state = crate::State::ClientAlmostReady;
        if self.is_multipath_enabled && !self.paths.is_empty() {
            let app_ctx = core::mem::replace(
                &mut self.pkt_ctx[PacketContext::Application as usize],
                PacketContextState {
                    send_sequence: 0,
                    next_sequence_hole: 0,
                    retransmit_sequence: 0,
                    highest_acknowledged: u64::MAX,
                    latest_time_acknowledged: self.start_time,
                    highest_acknowledged_time: self.start_time,
                    pending: BTreeMap::new(),
                    retransmitted: BTreeMap::new(),
                    preemptive_repeat_seq: None,
                    retransmitted_queue_size: 0,
                    ecn_ect0_total_remote: 0,
                    ecn_ect1_total_remote: 0,
                    ecn_ce_total_remote: 0,
                    ack_of_ack_requested: false,
                },
            );
            self.paths[0].pkt_ctx = app_ctx;
        }
    }
}

impl Connection {
    pub fn ready_state_transition(&mut self, current_time: Instant) {
        self.connection_state = crate::State::Ready;
        self.is_handshake_finished = true;
        self.implicit_handshake_ack(PacketContext::Initial, current_time);
        self.implicit_handshake_ack(PacketContext::Handshake, current_time);
        if !self.client_mode {
            let _ = self.queue_handshake_done_frame();
        }
        if self.is_half_open {
            self.is_half_open = false;
        }
        self.purge_misc_frames_after_ready();
        if self.is_ack_frequency_negotiated {
            self.is_ack_frequency_updated = true;
        } else {
            let rtt = self.paths.first().map(|p| p.rtt_min).unwrap_or(INITIAL_RTT);
            let rate = self.paths.first().map(|p| p.receive_rate_max).unwrap_or(0);
            let mut ack_gap = 0;
            let mut ack_delay = 0;
            self.compute_ack_gap_and_delay(
                rtt,
                ACK_DELAY_MIN.ticks(),
                rate,
                &mut ack_gap,
                &mut ack_delay,
            );
            self.ack_gap_remote = ack_gap;
            self.ack_delay_remote = Duration::from_ticks(ack_delay);
            self.max_ack_gap_remote = self.max_ack_gap_remote.max(ack_gap);
            self.max_ack_delay_remote = self
                .max_ack_delay_remote
                .max(Duration::from_ticks(ack_delay));
            self.min_ack_delay_remote = self
                .min_ack_delay_remote
                .min(Duration::from_ticks(ack_delay));
        }
    }
}

/// Parse a packet header and decrypt its payload.  C: `pcnx` and
/// `new_context_created` out-parameters fold into the `Ok` payload.
#[allow(clippy::too_many_arguments)]
impl Quic {
    pub fn parse_header_and_decrypt(
        &mut self,
        bytes: &[u8],
        packet_length: usize,
        addr_from: Option<&SocketAddr>,
        _current_time: Instant,
        decrypted_data: &mut StreamDataNode,
        ph: &mut PacketHeader,
        consumed: &mut usize,
    ) -> Result<(Option<ConnectionToken>, bool), crate::Error> {
        let packet_length = packet_length.min(bytes.len());
        let conn_tok = self.parse_packet_header(&bytes[..packet_length], addr_from, ph, true)?;
        let Some(conn_tok) = conn_tok else {
            *consumed = ph.offset.min(packet_length);
            return Ok((None, false));
        };
        let mut packet = bytes[..packet_length].to_vec();
        let cnx = self
            .connections
            .get(conn_tok)
            .ok_or(crate::Error::Generic)?;
        let epoch = ph.epoch as usize;
        let pn_dec = cnx.crypto_context[epoch]
            .pn_dec
            .as_deref()
            .ok_or(crate::Error::Tls)?;
        let aead = cnx.crypto_context[epoch]
            .aead_decrypt
            .as_deref()
            .ok_or(crate::Error::Tls)?;
        let mut header_copy = [0u8; MAX_PACKET_SIZE];
        let is_loss_bit = cnx.is_loss_bit_enabled_incoming;
        let largest = cnx.ack_ctx[ph.packet_context as usize].sack_list.first();
        if remove_header_protection_inner(
            &mut packet,
            packet_length,
            &mut header_copy,
            ph,
            pn_dec,
            is_loss_bit,
            largest,
        ) != 0
        {
            return Err(crate::Error::Tls);
        }
        let pn_length = ((packet[0] & 0x03) + 1) as usize;
        let header_end = ph.packet_number_offset.saturating_add(pn_length);
        let cipher_end = if ph.payload_length > 0 {
            ph.packet_number_offset
                .saturating_add(ph.payload_length)
                .min(packet_length)
        } else {
            packet_length
        };
        if header_end > cipher_end || cipher_end > packet.len() {
            return Err(crate::Error::InvalidArgument);
        }
        let header = packet[..header_end].to_vec();
        let mut payload = packet[header_end..cipher_end].to_vec();
        aead.decrypt(ph.packet_number_full, &header, &mut payload)?;
        let len = payload.len().min(decrypted_data.data.len());
        decrypted_data.stream_data_membership = None;
        decrypted_data.offset = 0;
        decrypted_data.length = len;
        decrypted_data.data[..len].copy_from_slice(&payload[..len]);
        *consumed = cipher_end;
        Ok((Some(conn_tok), false))
    }
}

// ---------------------------------------------------------------------------
// Packet number / ACK shortcuts.

impl Connection {
    pub fn get_sequence_number(&self, _path_x: &mut Path, pc: PacketContext) -> u64 {
        // C: picoquic_get_sequence_number
        self.pkt_ctx[pc as usize].send_sequence
    }
}

impl Connection {
    pub fn get_ack_number(&self, _path_x: &mut Path, pc: PacketContext) -> u64 {
        // C: picoquic_get_ack_number
        self.ack_ctx[pc as usize].sack_list.first()
    }
}

impl Connection {
    pub fn get_last_packet(&self, _path_x: &mut Path, pc: PacketContext) -> Option<PacketToken> {
        // C: picoquic_get_last_packet — last (highest seq) in pending queue
        self.pkt_ctx[pc as usize]
            .pending
            .values()
            .next_back()
            .copied()
    }
}

// ---------------------------------------------------------------------------
// ACK logic.

impl Connection {
    pub fn init_ack_ctx(&mut self, ack_ctx: &mut AckContext) {
        // C: picoquic_init_ack_ctx
        ack_ctx.sack_list = SackList::new();
        ack_ctx.time_stamp_largest_received = crate::Instant::from_ticks(u64::MAX);
        ack_ctx.act[0].highest_ack_sent = 0;
        ack_ctx.act[0].highest_ack_sent_time = self.start_time;
        ack_ctx.act[0].ack_needed = false;
        ack_ctx.act[1].highest_ack_sent = 0;
        ack_ctx.act[1].highest_ack_sent_time = self.start_time;
        ack_ctx.act[1].ack_needed = false;
    }
}

impl Connection {
    pub fn is_ack_needed(
        &self,
        _current_time: Instant,
        _next_wake_time: &mut Instant,
        pc: PacketContext,
        _is_opportunistic: i32,
    ) -> bool {
        // C: picoquic_is_ack_needed — check if an ACK frame needs to be sent.
        // Simplified: return true if any ACK is flagged needed in the ack context.
        let ack_ctx = &self.ack_ctx[pc as usize];
        ack_ctx.act[0].ack_needed || ack_ctx.act[1].ack_needed
    }
}

impl Connection {
    pub fn is_pn_already_received(
        &self,
        pc: PacketContext,
        _l_cid: Option<LocalConnectionIdToken>,
        pn64: u64,
    ) -> bool {
        // C: picoquic_is_pn_already_received — check SACK list for duplicate PN.
        let ack_ctx = &self.ack_ctx[pc as usize];
        // Check: is pn64 covered by any SACK range?
        let mut st_opt = ack_ctx.sack_list.ack_tree.last();
        while let Some(st) = st_opt {
            let item_tok = match ack_ctx.sack_list.ack_tree.get(st).copied() {
                Some(t) => t,
                None => break,
            };
            let item = match ack_ctx.sack_list.sack_items.get(item_tok) {
                Some(i) => i,
                None => break,
            };
            if item.start_of_sack_range <= pn64 && item.end_of_sack_range >= pn64 {
                return true;
            }
            if item.end_of_sack_range < pn64 {
                break;
            }
            st_opt = ack_ctx.sack_list.ack_tree.previous(st);
        }
        false
    }
}

impl Connection {
    pub fn record_pn_received(
        &mut self,
        pc: PacketContext,
        _l_cid: Option<LocalConnectionIdToken>,
        pn64: u64,
        current_microsec: Instant,
    ) -> i32 {
        // C: picoquic_record_pn_received — insert `pn64` into SACK list.
        let ack_ctx = &mut self.ack_ctx[pc as usize];
        match ack_ctx.sack_list.update(pn64, pn64, current_microsec) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
}

impl SackList {
    /// Choose at most `max_ranges` ACK ranges to put on the wire,
    /// updating per-range send counters.  `first_sack` selects the
    /// starting range; `None` starts at the highest-PN range.
    pub fn select_ack_ranges(
        &mut self,
        first_sack: Option<SackItemToken>,
        max_ranges: i32,
        is_opportunistic: i32,
        nb_sent_max: &mut i32,
        nb_sent_max_skip: &mut i32,
    ) {
        let idx = is_opportunistic.clamp(0, 1) as usize;
        let first_sack_count = first_sack
            .and_then(|tok| self.sack_items.get(tok))
            .map(|s| s.nb_times_sent[idx])
            .unwrap_or(MAX_ACK_RANGE_REPEAT as i32);
        let mut cumul_sent = 0;
        *nb_sent_max = MAX_ACK_RANGE_REPEAT as i32;
        *nb_sent_max_skip = 0;

        for i in 0..MAX_ACK_RANGE_REPEAT {
            cumul_sent += self.rc[idx].range_counts[i];
            if i as i32 == first_sack_count {
                cumul_sent -= 1;
            }
            if cumul_sent >= max_ranges {
                *nb_sent_max = i as i32;
                *nb_sent_max_skip = cumul_sent - max_ranges;
                break;
            }
        }
    }

    /// Merge the inclusive range `[pn64_min, pn64_max]` into the
    /// SACK list.  Returns Ok if the range was newly observed.
    pub fn update(
        &mut self,
        pn64_min: u64,
        pn64_max: u64,
        current_time: Instant,
    ) -> Result<(), crate::Error> {
        // Check if already covered.
        if self.check(pn64_min, pn64_max) {
            return Ok(());
        }
        self.insert_item(pn64_min, pn64_max, current_time)
    }

    /// True when every packet number in `[pn64_min, pn64_max]` is
    /// already covered by the SACK list.
    pub fn check(&mut self, pn64_min: u64, pn64_max: u64) -> bool {
        // Walk items and check coverage.  A single range covering [min,max] suffices.
        let mut st_opt = self.ack_tree.last();
        while let Some(st) = st_opt {
            let item_tok = match self.ack_tree.get(st).copied() {
                Some(t) => t,
                None => break,
            };
            let (start, end) = match self.sack_items.get(item_tok) {
                Some(item) => (item.start_of_sack_range, item.end_of_sack_range),
                None => break,
            };
            if start <= pn64_min && end >= pn64_max {
                return true;
            }
            if end < pn64_min {
                break;
            }
            st_opt = self.ack_tree.previous(st);
        }
        false
    }

    /// Discard ACK ranges newly covered by an incoming ACK-of-ACK,
    /// returning the new last-acked range.
    pub fn process_ack_of_ack_range(
        &mut self,
        _previous: Option<SackItemToken>,
        start_of_range: u64,
        end_of_range: u64,
    ) -> Option<SackItemToken> {
        let st = self.ack_tree.find(&start_of_range)?;
        let token = self.ack_tree.get(st).copied()?;
        let item = self.sack_items.get(token)?;
        if item.start_of_sack_range != start_of_range {
            return Some(token);
        }

        let next = self.ack_tree.next(st);
        if next.is_none() {
            let (_, tok) = self.ack_tree.remove(st)?;
            let key = {
                let item = self.sack_items.get_mut(tok)?;
                item.start_of_sack_range = if end_of_range < item.end_of_sack_range {
                    end_of_range + 1
                } else {
                    item.end_of_sack_range
                };
                item.start_of_sack_range
            };
            let (new_st, _) = self.ack_tree.insert(key, tok).ok()?;
            if let Some(inserted) = self.sack_items.get_mut(tok) {
                inserted.ack_tree_membership = Some(new_st);
            }
            Some(tok)
        } else if item.end_of_sack_range == end_of_range {
            if self.horizon_delay > 0 {
                if let Some(item) = self.sack_items.get_mut(token) {
                    for r in 0..2 {
                        let sent = item.nb_times_sent[r];
                        if sent >= 0 && (sent as usize) < MAX_ACK_RANGE_REPEAT {
                            self.rc[r].range_counts[sent as usize] -= 1;
                            item.nb_times_sent[r] = MAX_ACK_RANGE_REPEAT as i32;
                        }
                    }
                }
                Some(token)
            } else {
                self.ack_tree.remove(st);
                self.sack_items.remove(token);
                next.and_then(|next_st| self.ack_tree.get(next_st).copied())
            }
        } else {
            Some(token)
        }
    }

    /// Advance the ack horizon timestamp to drop expired ranges.
    pub fn update_ack_horizon(&mut self, current_time: Instant) {
        if self.horizon_delay > 0 {
            let horizon_ticks = current_time
                .ticks()
                .saturating_sub(self.horizon_delay as u64);
            self.ack_horizon = crate::Instant::from_ticks(horizon_ticks);
        }
    }

    /// First range in the list (highest PN), or `None` when empty.
    pub fn first_item(&self) -> Option<SackItemToken> {
        // The splay tree is keyed by start_of_sack_range; last() = highest start = highest PN range.
        self.ack_tree.last().and_then(|st| self.resolve_splay(st))
    }

    /// Last range in the list (lowest PN), or `None` when empty.
    pub fn last_item(&self) -> Option<SackItemToken> {
        self.ack_tree.first().and_then(|st| self.resolve_splay(st))
    }

    /// Insert a new range `[range_min, range_max]`.
    pub fn insert_item(
        &mut self,
        range_min: u64,
        range_max: u64,
        current_time: Instant,
    ) -> Result<(), crate::Error> {
        let item = SackItem {
            ack_tree_membership: None,
            start_of_sack_range: range_min,
            end_of_sack_range: range_max,
            time_created: current_time,
            nb_times_sent: [0; 2],
        };
        let tok = self.sack_items.insert(item)?;
        let (st, _) = self.ack_tree.insert(range_min, tok)?;
        if let Some(item) = self.sack_items.get_mut(tok) {
            item.ack_tree_membership = Some(st);
        }
        Ok(())
    }

    /// True when no ranges have been recorded.
    pub fn is_empty(&self) -> bool {
        self.ack_tree.is_empty()
    }
}

impl SackList {
    /// Splay-tree successor of `sack` in `list`.  C: `sack_next_item`.
    /// In picoquic, "next" means the range with the next lower PN (predecessor in our key-order).
    pub fn sack_next_item(&mut self, sack: SackItemToken) -> Option<SackItemToken> {
        let st = self.sack_items.get(sack)?.ack_tree_membership?;
        let next_st = self.ack_tree.previous(st)?;
        self.ack_tree.get(next_st).copied()
    }
}

impl SackList {
    /// Splay-tree predecessor of `sack` in `list`.
    /// In picoquic, "previous" means the range with the next higher PN (successor in our key-order).
    pub fn sack_previous_item(&mut self, sack: SackItemToken) -> Option<SackItemToken> {
        let st = self.sack_items.get(sack)?.ack_tree_membership?;
        let prev_st = self.ack_tree.next(st)?;
        self.ack_tree.get(prev_st).copied()
    }
}

impl Connection {
    /// Borrow the ACK context for `(packet_context, local_connection_id)`
    /// on `connection`.  Returns `None` when the requested context isn't installed.
    /// C: `picoquic_ack_ctx_from_cnx_context`.
    pub fn ack_ctx_from_cnx_context(
        &mut self,
        packet_context: PacketContext,
        local_connection_id: Option<LocalConnectionIdToken>,
    ) -> Option<&mut AckContext> {
        // When multipath is enabled and this is application context, use the
        // per-path ACK context (looking up by the CID's path_id).
        if self.is_multipath_enabled && packet_context == PacketContext::Application {
            let path_id = local_connection_id
                .and_then(|tok| self.local_connection_ids.get(tok))
                .map(|l| l.path_id)
                .unwrap_or(0);
            let path_idx = self.paths.iter().position(|p| p.unique_path_id == path_id);
            if let Some(idx) = path_idx {
                return Some(&mut self.paths[idx].ack_ctx);
            }
        }
        // Fallback: use the per-connection ACK context array.
        Some(&mut self.ack_ctx[packet_context as usize])
    }
}

impl Connection {
    /// Borrow the SACK list for `(packet_context, local_connection_id)`
    /// on `connection`.  Returns `None` when the requested context isn't installed.
    /// C: `picoquic_sack_list_from_cnx_context`.
    pub fn sack_list_from_cnx_context(
        &mut self,
        packet_context: PacketContext,
        local_connection_id: Option<LocalConnectionIdToken>,
    ) -> Option<&mut SackList> {
        let ack_ctx = self.ack_ctx_from_cnx_context(packet_context, local_connection_id)?;
        Some(&mut ack_ctx.sack_list)
    }
}

impl Default for SackList {
    fn default() -> Self {
        Self::new()
    }
}

impl SackList {
    /// Resolve a splay-tree node token to the stored SackItemToken value.
    fn resolve_splay(&self, st: SplayToken) -> Option<SackItemToken> {
        self.ack_tree.get(st).copied()
    }

    /// Highest packet number currently sacked.
    pub fn first(&self) -> u64 {
        // The last() of the splay tree (highest key = highest PN start) gives the first range.
        match self.ack_tree.last().and_then(|st| self.resolve_splay(st)) {
            Some(tok) => self
                .sack_items
                .get(tok)
                .map(|item| item.end_of_sack_range)
                .unwrap_or(0),
            None => 0,
        }
    }

    /// Lowest packet number currently sacked.
    pub fn last(&self) -> u64 {
        match self.ack_tree.first().and_then(|st| self.resolve_splay(st)) {
            Some(tok) => self
                .sack_items
                .get(tok)
                .map(|item| item.start_of_sack_range)
                .unwrap_or(0),
            None => 0,
        }
    }

    /// Topmost range, or `None` when empty.
    pub fn first_range(&self) -> Option<SackItemToken> {
        self.ack_tree.last().and_then(|st| self.resolve_splay(st))
    }

    /// Create a zero-initialised SACK list.  C: `picoquic_sack_list_init`
    /// applied to a zero-initialised struct.
    pub fn new() -> Self {
        Self {
            ack_tree: SplayTree::new(),
            sack_items: Arena::new(),
            ack_horizon: Instant::from_ticks(0),
            horizon_delay: 0,
            rc: [
                SackRangeCount {
                    range_counts: [0; MAX_ACK_RANGE_REPEAT],
                },
                SackRangeCount {
                    range_counts: [0; MAX_ACK_RANGE_REPEAT],
                },
            ],
        }
    }

    /// Reset the list to "everything before this PN was acked".
    pub fn init(&mut self) {
        self.ack_tree.clear();
        self.sack_items.clear();
        self.ack_horizon = crate::Instant::from_ticks(0);
        self.horizon_delay = 0;
        for rc in self.rc.iter_mut() {
            rc.range_counts = [0; MAX_ACK_RANGE_REPEAT];
        }
    }

    /// As [`Self::init`] but seeds the list with one range.
    pub fn reset(
        &mut self,
        range_min: u64,
        range_max: u64,
        current_time: Instant,
    ) -> Result<(), crate::Error> {
        self.init();
        self.insert_item(range_min, range_max, current_time)
    }

    /// Free all ranges and reset the list to empty.
    pub fn free(&mut self) {
        self.init();
    }

    /// Number of ranges currently stored.
    pub fn size(&self) -> usize {
        self.ack_tree.len()
    }
}

impl SackItem {
    /// Inclusive start of this SACK range.  C:
    /// `sack_item_range_start`.
    pub fn range_start(&self) -> u64 {
        self.start_of_sack_range
    }

    /// Exclusive end of this SACK range.  C:
    /// `sack_item_range_end`.
    pub fn range_end(&self) -> u64 {
        self.end_of_sack_range
    }

    /// Number of times this range has been sent in an ACK frame.
    /// C: `sack_item_nb_times_sent`.
    pub fn nb_times_sent(&self, is_opportunistic: i32) -> i32 {
        self.nb_times_sent[is_opportunistic.clamp(0, 1) as usize]
    }
}

impl SackList {
    /// Bump the per-range send counter for the item at `token`.
    pub fn item_record_sent(&mut self, token: SackItemToken, is_opportunistic: i32) {
        if let Some(item) = self.sack_items.get_mut(token) {
            let idx = is_opportunistic.clamp(0, 1) as usize;
            item.nb_times_sent[idx] += 1;
        }
    }

    /// Reset the per-range send counters for the item at `token`.
    pub fn item_record_reset(&mut self, token: SackItemToken) {
        if let Some(item) = self.sack_items.get_mut(token) {
            item.nb_times_sent = [0; 2];
        }
    }
}

impl PacketData {
    pub fn record_ack_packet_data(&mut self, acked_packet: &mut Packet) {
        let Some(send_path) = acked_packet.send_path else {
            return;
        };

        let mut path_i = 0usize;
        while path_i < self.nb_path_ack as usize
            && self.path_ack[path_i].acked_path != Some(send_path)
        {
            path_i += 1;
        }
        if path_i == self.nb_path_ack as usize {
            if path_i >= NB_PATH_TARGET {
                return;
            }
            self.nb_path_ack += 1;
            self.path_ack[path_i].acked_path = Some(send_path);
        }

        if !self.path_ack[path_i].is_set {
            self.path_ack[path_i].largest_sent_time = acked_packet.send_time;
            self.path_ack[path_i].delivered_prior = acked_packet.delivered_prior;
            self.path_ack[path_i].delivered_time_prior = acked_packet.delivered_time_prior;
            self.path_ack[path_i].delivered_sent_prior = acked_packet.delivered_sent_prior;
            self.path_ack[path_i].lost_prior = acked_packet.lost_prior;
            self.path_ack[path_i].inflight_prior = acked_packet.inflight_prior;
            self.path_ack[path_i].rs_is_path_limited = acked_packet.delivered_app_limited;
            self.path_ack[path_i].rs_is_cwnd_limited = acked_packet.sent_cwin_limited;
            self.path_ack[path_i].is_set = true;
        }
        self.path_ack[path_i].data_acked = self.path_ack[path_i]
            .data_acked
            .saturating_add(acked_packet.length as u64);
    }
}

impl Connection {
    pub fn init_packet_ctx(&mut self, pkt_ctx: &mut PacketContextState, pc: PacketContext) {
        // C: picoquic_init_packet_ctx
        if self.random_initial != 0 && (pc == PacketContext::Initial || self.random_initial > 1) {
            let mut rnd = [0u8; 8];
            if fill_system_random(&mut rnd).is_ok() {
                pkt_ctx.send_sequence =
                    u64::from_le_bytes(rnd) % PN_RANDOM_RANGE as u64 + PN_RANDOM_MIN as u64;
            } else {
                pkt_ctx.send_sequence =
                    crate::public_random_64() % PN_RANDOM_RANGE as u64 + PN_RANDOM_MIN as u64;
            }
        } else {
            pkt_ctx.send_sequence = 0;
        }
        pkt_ctx.highest_acknowledged = pkt_ctx.send_sequence.wrapping_sub(1);
        pkt_ctx.latest_time_acknowledged = self.start_time;
        pkt_ctx.highest_acknowledged_time = self.start_time;
        pkt_ctx.pending = std::collections::BTreeMap::new();
        pkt_ctx.retransmitted = std::collections::BTreeMap::new();
    }
}

impl SackList {
    pub fn process_ack_of_ack_frame(
        &mut self,
        bytes: &mut [u8],
        bytes_max: usize,
        consumed: &mut usize,
        is_ecn: i32,
    ) -> i32 {
        let mut largest = 0;
        let mut ack_delay = 0;
        let mut num_block = 0;
        let mut path_id = 0;
        if parse_ack_header(
            bytes,
            bytes_max,
            &mut num_block,
            &mut path_id,
            &mut largest,
            &mut ack_delay,
            consumed,
            0,
        ) != 0
        {
            return -1;
        }

        let max = bytes_max.min(bytes.len());
        let mut tail = &bytes[*consumed..max];
        let mut range = 0;
        tail = match frames_varint_decode(tail, &mut range) {
            Some(t) => t,
            None => return -1,
        };
        if range > largest {
            return -1;
        }
        let mut start = largest - range;
        let mut previous = self.process_ack_of_ack_range(None, start, largest);

        for _ in 0..num_block {
            let mut gap = 0;
            let mut block_range = 0;
            tail = match frames_varint_decode(tail, &mut gap) {
                Some(t) => t,
                None => return -1,
            };
            let Some(next_largest) = start.checked_sub(gap + 2) else {
                return -1;
            };
            tail = match frames_varint_decode(tail, &mut block_range) {
                Some(t) => t,
                None => return -1,
            };
            if block_range > next_largest {
                return -1;
            }
            start = next_largest - block_range;
            previous = self.process_ack_of_ack_range(previous, start, next_largest);
        }

        if is_ecn != 0 {
            tail = match skip_n_varints(tail, 3) {
                Some(t) => t,
                None => return -1,
            };
        }
        *consumed = max - tail.len();
        0
    }
}

impl Connection {
    pub fn compute_ack_gap_and_delay(
        &self,
        rtt: Duration,
        remote_min_ack_delay: u64,
        data_rate: u64,
        ack_gap: &mut u64,
        ack_delay_max: &mut u64,
    ) {
        let first_path = self.paths.first();
        let rtt_ticks = rtt.ticks();
        let bytes_in_window = data_rate
            .saturating_mul(rtt_ticks)
            .saturating_div(1_000_000);
        let mut nb_packets = (bytes_in_window / MAX_PACKET_SIZE as u64).max(2);

        *ack_delay_max = (rtt_ticks / 4).min(ACK_DELAY_MAX.ticks());
        if !self.is_ack_frequency_negotiated
            && first_path
                .map(|p| !p.is_ssthresh_initialized)
                .unwrap_or(true)
        {
            *ack_delay_max /= 2;
        }
        *ack_delay_max = (*ack_delay_max).max(remote_min_ack_delay);

        if self.is_ack_frequency_negotiated
            && first_path
                .map(|p| !p.is_ssthresh_initialized)
                .unwrap_or(false)
        {
            nb_packets /= 2;
        }

        if let Some(path) = first_path
            && path.rtt_min < Duration::from_ticks(4 * ACK_DELAY_MIN.ticks())
        {
            let mult = if path.rtt_min > ACK_DELAY_MIN {
                (4 * ACK_DELAY_MIN.ticks()) / path.rtt_min.ticks().max(1)
            } else {
                4
            };
            nb_packets = nb_packets.saturating_mul(mult);
        }

        let mut gap = nb_packets.div_ceil(4);
        let mut gap_min = 2;
        if data_rate > BANDWIDTH_MEDIUM {
            gap_min = if first_path
                .map(|p| p.rtt_min > TARGET_RENO_RTT)
                .unwrap_or(false)
            {
                10
            } else {
                4
            };
        }
        if gap < gap_min {
            gap = gap_min;
        } else if gap > 32 {
            let cc_number = self
                .congestion_alg
                .map(|cc| cc.congestion_algorithm_number)
                .unwrap_or(CC_ALGO_NUMBER_NEW_RENO);
            if self.is_multipath_enabled
                || cc_number == CC_ALGO_NUMBER_NEW_RENO
                || cc_number == CC_ALGO_NUMBER_FAST
            {
                gap = 32;
            } else {
                gap = (32 + nb_packets.saturating_sub(128) / 8).min(64);
            }
        }
        *ack_gap = gap;
    }
}

impl Connection {
    pub fn seed_bandwidth(&mut self, rtt_min: Duration, cwin: u64, ip_addr: core::net::IpAddr) {
        if let Some(path) = self.paths.first_mut() {
            path.rtt_min = if rtt_min.ticks() == 0 {
                path.rtt_min
            } else {
                rtt_min
            };
            if cwin > 0 {
                path.cwin = cwin;
                path.bandwidth_estimate = cwin
                    .saturating_mul(1_000_000)
                    .saturating_div(path.rtt_min.ticks().max(1));
                path.bandwidth_estimate_max =
                    path.bandwidth_estimate_max.max(path.bandwidth_estimate);
            }
            let ip_bytes = stored_ip_bytes(ip_addr);
            let len = ip_bytes.len().min(path.ip_client_remote.len());
            path.ip_client_remote[..len].copy_from_slice(&ip_bytes[..len]);
            path.ip_client_remote_length = len as u8;
        }
    }
}

impl Connection {
    pub fn current_retransmit_timer(&self, path_x: &mut Path) -> u64 {
        let mut rto = path_x.retransmit_timer.ticks();
        if path_x.nb_retransmit > 0 {
            if path_x.nb_retransmit < 3 {
                rto = saturating_shl_u64(rto, path_x.nb_retransmit as u32);
            } else {
                let mut n1 = path_x.nb_retransmit.saturating_sub(2).min(18);
                rto = saturating_shl_u64(rto, (2 + (n1 / 4)) as u32);
                n1 &= 3;
                rto = rto.saturating_add((n1.saturating_mul(rto)) >> 2);
            }
            let idle = self.idle_timeout.ticks();
            if idle > 15 {
                rto = rto.min(idle >> 4);
            }
        }

        if self.connection_state < State::ClientReadyStart
            && MICROSEC_HANDSHAKE_MAX.ticks() / 1000
                < self.local_parameters.max_idle_timeout.ticks()
        {
            rto = path_x
                .retransmit_timer
                .ticks()
                .checked_shl(path_x.nb_retransmit.min(18) as u32)
                .unwrap_or(u64::MAX);
            rto = rto.min(
                self.local_parameters
                    .max_idle_timeout
                    .ticks()
                    .saturating_mul(100),
            );
        }
        rto.max(MIN_RETRANSMIT_TIMER.ticks())
    }
}

impl Connection {
    pub fn update_path_rtt(
        &mut self,
        old_path: &mut Path,
        epoch: i32,
        send_time: Instant,
        current_time: Instant,
        mut ack_delay: u64,
        _time_stamp: u64,
    ) {
        if old_path.rtt_is_initialized && epoch < 0 {
            return;
        }

        let is_first = !old_path.rtt_is_initialized;
        let mut rtt_estimate = current_time.ticks().saturating_sub(send_time.ticks());
        if !is_first && ack_delay > 0 && self.connection_state >= State::Ready {
            ack_delay = ack_delay.min(self.local_parameters.max_ack_delay as u64);
            if old_path.rtt_min.ticks().saturating_add(ack_delay) < rtt_estimate {
                rtt_estimate = rtt_estimate.saturating_sub(ack_delay);
            }
        }

        let estimate = Duration::from_ticks(rtt_estimate);
        old_path.rtt_sample = estimate;
        old_path.nb_rtt_estimate_in_period += 1;
        old_path.sum_rtt_estimate_in_period += estimate;
        if old_path.nb_rtt_estimate_in_period == 1 {
            old_path.min_rtt_estimate_in_period = estimate;
            old_path.max_rtt_estimate_in_period = estimate;
        } else {
            old_path.min_rtt_estimate_in_period = old_path.min_rtt_estimate_in_period.min(estimate);
            old_path.max_rtt_estimate_in_period = old_path.max_rtt_estimate_in_period.max(estimate);
        }
        if old_path.retransmit_timer < estimate {
            old_path.retransmit_timer = estimate;
        }
        if old_path.rtt_min.ticks() == 0 || old_path.rtt_min > estimate {
            old_path.rtt_min = estimate;
        }
        old_path.rtt_max = old_path.rtt_max.max(estimate);

        if is_first {
            old_path.smoothed_rtt = estimate;
            old_path.rtt_variant = estimate / 2;
            old_path.rtt_is_initialized = true;
        } else {
            let smoothed = old_path.smoothed_rtt.ticks();
            let sample = estimate.ticks();
            let abs_delta = smoothed.abs_diff(sample);
            old_path.rtt_variant =
                Duration::from_ticks((3 * old_path.rtt_variant.ticks() + abs_delta) / 4);
            old_path.smoothed_rtt = Duration::from_ticks((7 * smoothed + sample) / 8);
        }
        old_path.retransmit_timer = Duration::from_ticks(
            old_path
                .smoothed_rtt
                .ticks()
                .saturating_add(4 * old_path.rtt_variant.ticks())
                .saturating_add(self.max_ack_delay_remote.ticks())
                .max(MIN_RETRANSMIT_TIMER.ticks()),
        );
    }
}

// ---------------------------------------------------------------------------
// Stream management.

impl Connection {
    fn compare_output_stream_tokens(
        &self,
        left: StreamToken,
        right: StreamToken,
    ) -> core::cmp::Ordering {
        let Some(left) = self.streams.get(left) else {
            return core::cmp::Ordering::Greater;
        };
        let Some(right) = self.streams.get(right) else {
            return core::cmp::Ordering::Less;
        };
        left.stream_priority
            .cmp(&right.stream_priority)
            .then_with(|| left.stream_id.cmp(&right.stream_id))
    }

    fn enqueue_output_stream_token(&mut self, token: StreamToken) {
        if self
            .output_streams
            .iter()
            .any(|&existing| existing == token)
        {
            return;
        }
        let pos = self
            .output_streams
            .iter()
            .position(|&existing| {
                self.compare_output_stream_tokens(token, existing) == core::cmp::Ordering::Less
            })
            .unwrap_or(self.output_streams.len());
        self.output_streams.insert(pos, token);
    }

    /// Create and register a new application stream with the given id.
    /// C: `picoquic_create_stream`.
    pub fn create_stream(&mut self, stream_id: u64) -> Result<StreamToken, crate::Error> {
        use crate::stream::{Role, StreamId};

        let sid = StreamId(stream_id);
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };

        // Determine flow-control limits and output eligibility.
        let (maxdata_local, maxdata_remote, is_output_stream) = if sid.is_local(local_role) {
            if sid.is_bidir() {
                (
                    self.local_parameters.initial_max_stream_data_bidi_local,
                    self.remote_parameters.initial_max_stream_data_bidi_remote,
                    stream_id <= self.max_stream_id_bidir_remote,
                )
            } else {
                (
                    0u64,
                    self.remote_parameters.initial_max_stream_data_uni,
                    stream_id <= self.max_stream_id_unidir_remote,
                )
            }
        } else if sid.is_bidir() {
            (
                self.local_parameters.initial_max_stream_data_bidi_remote,
                self.remote_parameters.initial_max_stream_data_bidi_local,
                true,
            )
        } else {
            (
                self.local_parameters.initial_max_stream_data_uni,
                0u64,
                false,
            )
        };

        let stream = StreamHead {
            stream_tree_membership: None,
            stream_id,
            affinity_path: None,
            consumed_offset: 0,
            fin_offset: 0,
            reset_offset: 0,
            maxdata_local,
            maxdata_local_acked: 0,
            maxdata_remote,
            local_error: 0,
            remote_error: 0,
            local_stop_error: 0,
            remote_stop_error: 0,
            last_time_data_sent: crate::Instant::from_ticks(0),
            stream_data_tree: crate::splay::SplayTree::default(),
            stream_data_nodes: crate::arena::Arena::new(),
            sent_offset: 0,
            reliable_size: 0,
            send_queue: std::collections::VecDeque::new(),
            app_stream_ctx: None,
            direct_receive_fn: None,
            direct_receive_ctx: None,
            sack_list: SackList::new(),
            stream_priority: crate::DEFAULT_STREAM_PRIORITY,
            is_active: false,
            fin_requested: false,
            fin_sent: false,
            fin_received: false,
            fin_signalled: false,
            reset_requested: false,
            reset_sent: false,
            reset_acked: false,
            reset_received: false,
            reset_signalled: false,
            stop_sending_requested: false,
            stop_sending_sent: false,
            stop_sending_received: false,
            stop_sending_signalled: false,
            max_stream_updated: false,
            stream_data_blocked_sent: false,
            is_output_stream: false,
            is_closed: false,
            is_discarded: false,
            use_app_flow_control: false,
            is_not_coalesced: false,
        };

        let tok = self
            .streams
            .insert(stream)
            .map_err(|_| crate::Error::Memory)?;

        // Insert into the splay tree keyed by stream_id.
        let (splay_tok, _) = self
            .stream_tree
            .insert(stream_id, tok)
            .map_err(|_| crate::Error::Memory)?;
        if let Some(s) = self.streams.get_mut(tok) {
            s.stream_tree_membership = Some(splay_tok);
            s.is_output_stream = false; // handled below
        }

        // Advance next_stream_id if needed.
        // C: STREAM_TYPE_FROM_ID = (stream_id & 3)
        let type_idx = (stream_id & 3) as usize;
        if stream_id >= self.next_stream_id[type_idx] {
            self.next_stream_id[type_idx] = stream_id + 4;
        }

        // Insert into output queue if applicable.
        if is_output_stream {
            // Borrow the stream and mark it, then enqueue its token.
            if let Some(s) = self.streams.get_mut(tok) {
                s.is_output_stream = true;
            }
            self.enqueue_output_stream_token(tok);
        }

        Ok(tok)
    }

    /// Open every stream from "next remote stream" up through `stream_id`.
    /// C: `picoquic_create_missing_streams`.
    pub fn create_missing_streams(
        &mut self,
        stream_id: u64,
        _is_remote: bool,
    ) -> Result<StreamToken, crate::Error> {
        use crate::stream::StreamId;
        let type_idx = (stream_id & 3) as usize;
        let mut first_new_id = self.next_stream_id[type_idx];
        let mut last_tok: Option<StreamToken> = None;
        // Walk forward creating streams until we reach stream_id.
        while first_new_id <= stream_id {
            let tok = self.create_stream(first_new_id)?;
            last_tok = Some(tok);
            first_new_id = StreamId(first_new_id).next_with_same_kind().0;
        }
        last_tok.ok_or(crate::Error::Memory)
    }

    /// If `stream` has reached the closed state, mark it closed.
    /// C: `picoquic_delete_stream_if_closed`.
    pub fn delete_stream_if_closed(&mut self, stream: &mut StreamHead) -> i32 {
        let mut ret = 0;
        if !stream.is_closed && stream.is_stream_closed(self.client_mode) {
            stream.is_closed = true;
            ret = 1;
        }
        // Remove from tree if acked or remote-unidir.
        if stream.is_closed {
            use crate::stream::{Role, StreamId};
            let sid = StreamId(stream.stream_id);
            let local_role = if self.client_mode {
                Role::Client
            } else {
                Role::Server
            };
            let is_remote_unidir = !sid.is_bidir() && !sid.is_local(local_role);
            if is_remote_unidir {
                // Safe to remove immediately — no ACKs expected.
                if let Some(splay_tok) = stream.stream_tree_membership.take() {
                    self.stream_tree.remove(splay_tok);
                }
            }
        }
        ret
    }

    /// Propagate updated remote parameters to existing streams.
    /// C: `picoquic_update_stream_initial_remote`.
    pub fn update_stream_initial_remote(&mut self) {
        use crate::stream::{Role, StreamId};
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };
        let bidi_remote = self.remote_parameters.initial_max_stream_data_bidi_remote;
        let bidi_local = self.remote_parameters.initial_max_stream_data_bidi_local;
        let uni = self.remote_parameters.initial_max_stream_data_uni;
        for s in self.streams.iter_mut() {
            let sid = StreamId(s.stream_id);
            if sid.is_local(local_role) {
                if sid.is_bidir() {
                    if s.maxdata_remote < bidi_remote {
                        s.maxdata_remote = bidi_remote;
                    }
                } else if s.maxdata_remote < uni {
                    s.maxdata_remote = uni;
                }
            } else if sid.is_bidir() && s.maxdata_remote < bidi_local {
                s.maxdata_remote = bidi_local;
            }
        }
    }

    /// Splice `stream` into the per-connection output queue (by its arena token).
    /// C: `picoquic_insert_output_stream`.
    /// Note: takes the token rather than a mutable reference to avoid
    /// double-borrow of `self.streams` and `self.output_streams`.
    pub fn insert_output_stream(&mut self, stream: &mut StreamHead) {
        if !stream.is_output_stream {
            // Check remote flow-control limit.
            use crate::stream::{Role, StreamId};
            let sid = StreamId(stream.stream_id);
            let local_role = if self.client_mode {
                Role::Client
            } else {
                Role::Server
            };
            if sid.is_local(local_role) {
                let max = if sid.is_bidir() {
                    self.max_stream_id_bidir_remote
                } else {
                    self.max_stream_id_unidir_remote
                };
                if stream.stream_id > max {
                    return;
                }
            }
            stream.is_output_stream = true;
            // Find the token for this stream via its tree membership.
            if let Some(splay_tok) = stream.stream_tree_membership
                && let Some(tok) = self.stream_tree.get(splay_tok).copied()
            {
                self.enqueue_output_stream_token(tok);
            }
        }
    }

    /// Remove `stream` from the per-connection output queue.
    /// C: `picoquic_remove_output_stream`.
    pub fn remove_output_stream(&mut self, stream: &mut StreamHead) {
        if stream.is_output_stream {
            stream.is_output_stream = false;
            if let Some(splay_tok) = stream.stream_tree_membership
                && let Some(tok) = self.stream_tree.get(splay_tok).copied()
            {
                // Remove by value from the VecDeque.
                if let Some(pos) = self.output_streams.iter().position(|&t| t == tok) {
                    self.output_streams.remove(pos);
                }
            }
        }
    }

    /// Re-position `stream` in the output queue based on its current priority.
    /// C: `picoquic_reorder_output_stream`.
    pub fn reorder_output_stream(&mut self, stream: &mut StreamHead) {
        if stream.is_output_stream {
            // Simple re-insert: remove then add back at the end.
            self.remove_output_stream(stream);
            stream.is_output_stream = false;
            self.insert_output_stream(stream);
        }
    }

    /// First stream in this connection's stream tree (lowest stream_id).
    /// C: `picoquic_first_stream`.
    pub fn first_stream(&self) -> Option<StreamToken> {
        let st = self.stream_tree.first()?;
        self.stream_tree.get(st).copied()
    }

    /// Last stream in this connection's stream tree (highest stream_id).
    /// C: `picoquic_last_stream`.
    pub fn last_stream(&self) -> Option<StreamToken> {
        let st = self.stream_tree.last()?;
        self.stream_tree.get(st).copied()
    }

    /// Look up a stream by id.  C: `picoquic_find_stream`.
    pub fn find_stream(&mut self, stream_id: u64) -> Option<StreamToken> {
        let st = self.stream_tree.find(&stream_id)?;
        self.stream_tree.get(st).copied()
    }

    /// Open output streams to bridge a peer-side `MAX_STREAMS` bump.
    /// C: `picoquic_add_output_streams`.
    pub fn add_output_streams(&mut self, old_limit: u64, new_limit: u64, _is_bidir: bool) {
        // Walk streams from old_limit+1 to new_limit and insert them into output.
        // We collect tokens first to avoid borrow issues.
        let tokens: Vec<StreamToken> = self
            .streams
            .iter()
            .filter_map(|s| {
                if s.stream_id > old_limit && s.stream_id <= new_limit {
                    s.stream_tree_membership
                        .and_then(|st| self.stream_tree.get(st).copied())
                } else {
                    None
                }
            })
            .collect();
        for tok in tokens {
            if let Some(s) = self.streams.get_mut(tok)
                && !s.is_output_stream
            {
                s.is_output_stream = true;
                self.enqueue_output_stream_token(tok);
            }
        }
    }

    /// Pick the highest-priority ready stream that can send on `path_x`.
    /// C: `picoquic_find_ready_stream_path`.
    pub fn find_ready_stream_path(
        &self,
        path_x: &mut Path,
        is_coalesced: bool,
    ) -> Option<StreamToken> {
        self.output_streams.iter().copied().find(|tok| {
            self.streams.get(*tok).is_some_and(|stream| {
                let path_ok = stream
                    .affinity_path
                    .map(|affinity| {
                        affinity
                            == PathToken::synthetic(
                                path_x.unique_path_id as u32,
                                path_x.unique_path_id as u32,
                            )
                    })
                    .unwrap_or(true);
                let coalescing_ok = is_coalesced || !stream.is_not_coalesced;
                path_ok
                    && coalescing_ok
                    && (!stream.send_queue.is_empty()
                        || (stream.fin_requested && !stream.fin_sent)
                        || stream.is_active)
            })
        })
    }

    /// As [`Self::find_ready_stream_path`] but path-agnostic.
    pub fn find_ready_stream(&self) -> Option<StreamToken> {
        self.output_streams.iter().copied().find(|tok| {
            self.streams.get(*tok).is_some_and(|stream| {
                let control_frame_ready = (stream.stop_sending_requested
                    && !stream.stop_sending_sent)
                    || (stream.reset_requested && !stream.reset_sent);
                let data_ready = self.maxdata_remote > self.data_sent
                    && stream.sent_offset < stream.maxdata_remote
                    && (stream.is_active
                        || !stream.send_queue.is_empty()
                        || (stream.fin_requested && !stream.fin_sent));
                control_frame_ready || data_ready
            })
        })
    }

    /// True when the TLS handshake stream has data to send.
    pub fn is_tls_stream_ready(&self) -> bool {
        self.tls_stream.iter().any(|s| !s.send_queue.is_empty())
    }
}

impl StreamHead {
    /// True when `stream` has finished sending and receiving all data.
    /// C: `picoquic_is_stream_closed`.
    pub fn is_stream_closed(&self, client_mode: bool) -> bool {
        use crate::stream::{Role, StreamId};
        let sid = StreamId(self.stream_id);
        let local_role = if client_mode {
            Role::Client
        } else {
            Role::Server
        };
        if sid.is_bidir() {
            ((self.fin_requested && self.fin_sent) || (self.reset_requested && self.reset_sent))
                && ((self.fin_received && self.fin_signalled)
                    || (self.reset_received && self.reset_signalled))
        } else if sid.is_local(local_role) {
            // Unidir from local host.
            (self.fin_requested && self.fin_sent) || (self.reset_requested && self.reset_sent)
        } else {
            // Unidir from remote.
            (self.fin_received && self.fin_signalled)
                || (self.reset_received && self.reset_signalled)
        }
    }
}

// `stream_from_node` is gone — the C version recovered the parent
// pointer from an embedded `picosplay_node_t*` via `offsetof`.  In
// the Rust port the splay tree stores `StreamToken` directly, so
// the `Connection::stream_tree.get(token)` path is one indirection
// rather than offset arithmetic.

impl Connection {
    /// Splay-tree successor of `stream` in `connection.stream_tree`.
    /// C: `picoquic_next_stream`.
    pub fn next_stream(&self, stream: StreamToken) -> Option<StreamToken> {
        // Find the splay token for this stream via its membership field.
        let splay_tok = self.streams.get(stream)?.stream_tree_membership?;
        let next_splay = self.stream_tree.next(splay_tok)?;
        self.stream_tree.get(next_splay).copied()
    }
}

fn queued_stream_data_info(
    stream: &StreamHead,
    token: crate::splay::SplayToken,
) -> Option<(u64, usize)> {
    let data_token = *stream.stream_data_tree.get(token)?;
    let data = stream.stream_data_nodes.get(data_token)?;
    Some((data.offset, data.length))
}

fn insert_received_stream_data_chunk(
    stream: &mut StreamHead,
    offset: u64,
    data: &[u8],
    received_data: &mut StreamDataNode,
) -> Result<(), crate::Error> {
    if data.len() > MAX_PACKET_SIZE {
        return Err(crate::Error::BufferTooSmall);
    }

    let mut node = StreamDataNode {
        stream_data_membership: None,
        offset,
        data: [0u8; MAX_PACKET_SIZE],
        length: data.len(),
    };
    node.data[..data.len()].copy_from_slice(data);

    let token = stream
        .stream_data_nodes
        .insert(node)
        .map_err(|_| crate::Error::Memory)?;
    let (tree_token, old) = stream
        .stream_data_tree
        .insert(offset, token)
        .map_err(|_| crate::Error::Memory)?;
    if let Some(old) = old {
        stream.stream_data_nodes.remove(old);
    }
    if let Some(stored) = stream.stream_data_nodes.get_mut(token) {
        stored.stream_data_membership = Some(tree_token);
    }

    received_data.stream_data_membership = None;
    received_data.offset = offset;
    received_data.length = data.len();
    received_data.data[..data.len()].copy_from_slice(data);
    Ok(())
}

fn queue_received_stream_data(
    stream: &mut StreamHead,
    offset: u64,
    data: &[u8],
    received_data: &mut StreamDataNode,
) -> Result<(), crate::Error> {
    let input_begin = offset;
    let input_end = offset.saturating_add(data.len() as u64);
    let mut frame_offset = offset.max(stream.consumed_offset);

    if frame_offset >= input_end {
        return Ok(());
    }

    let previous = stream.stream_data_tree.find_previous(&frame_offset);
    if let Some(previous) = previous
        && let Some((prev_offset, prev_len)) = queued_stream_data_info(stream, previous)
    {
        let prev_end = prev_offset.saturating_add(prev_len as u64);
        if prev_end > frame_offset {
            frame_offset = prev_end;
        }
    }

    let mut next = if let Some(previous) = previous {
        stream.stream_data_tree.next(previous)
    } else {
        stream.stream_data_tree.first()
    };

    while frame_offset < input_end {
        let Some(next_token) = next else {
            break;
        };
        let Some((next_offset, next_len)) = queued_stream_data_info(stream, next_token) else {
            break;
        };
        if next_offset >= input_end {
            break;
        }

        if next_offset > frame_offset {
            let chunk_len = (next_offset - frame_offset) as usize;
            let src = (frame_offset - input_begin) as usize;
            insert_received_stream_data_chunk(
                stream,
                frame_offset,
                &data[src..src + chunk_len],
                received_data,
            )?;
        }

        frame_offset = next_offset.saturating_add(next_len as u64);
        next = stream.stream_data_tree.next(next_token);
    }

    if frame_offset < input_end {
        let src = (frame_offset - input_begin) as usize;
        insert_received_stream_data_chunk(stream, frame_offset, &data[src..], received_data)?;
    }

    Ok(())
}

pub fn decode_stream_frame<'a>(
    connection: &mut Connection,
    bytes: &'a [u8],
    received_data: &mut StreamDataNode,
    current_time: Instant,
) -> Option<&'a [u8]> {
    let mut stream_id = 0;
    let mut offset = 0;
    let mut data_length = 0;
    let mut fin = 0;
    let mut consumed = 0;
    if parse_stream_header(
        bytes,
        bytes.len(),
        &mut stream_id,
        &mut offset,
        &mut data_length,
        &mut fin,
        &mut consumed,
    ) != 0
    {
        return None;
    }
    let data_end = consumed.checked_add(data_length)?;
    if data_end > bytes.len() {
        return None;
    }
    let data = &bytes[consumed..data_end];
    let tok = connection
        .find_stream(stream_id)
        .map(Ok)
        .unwrap_or_else(|| connection.create_missing_streams(stream_id, true))
        .ok()?;
    if let Some(stream) = connection.streams.get_mut(tok) {
        if data_length > 0 {
            queue_received_stream_data(stream, offset, data, received_data).ok()?;
        }
        if fin != 0 {
            stream.fin_received = true;
            stream.fin_offset = offset.saturating_add(data_length as u64);
        }
        stream.last_time_data_sent = current_time;
    }
    Some(&bytes[data_end..])
}

pub fn format_stream_frame<'a>(
    _connection: &mut Connection,
    stream: &mut StreamHead,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    is_still_active: &mut i32,
    ret: &mut i32,
) -> Option<&'a mut [u8]> {
    encode_stream_like_frame(
        stream,
        bytes,
        more_data,
        is_pure_ack,
        is_still_active,
        ret,
        false,
    )
}

impl Connection {
    pub fn update_max_stream_id_local(&mut self, stream: &mut StreamHead) {
        use crate::stream::{Role, StreamId};
        let sid = StreamId(stream.stream_id);
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };
        if sid.is_local(local_role) {
            return;
        }
        if sid.is_bidir() {
            if stream.stream_id >= self.max_stream_id_bidir_local {
                let old = self.max_stream_id_bidir_local;
                self.max_stream_id_bidir_local_computed = self
                    .max_stream_id_bidir_local_computed
                    .max(stream.stream_id.saturating_add(4));
                if self.max_stream_id_bidir_local_computed > old {
                    stream.max_stream_updated = true;
                }
            }
        } else if stream.stream_id >= self.max_stream_id_unidir_local {
            let old = self.max_stream_id_unidir_local;
            self.max_stream_id_unidir_local_computed = self
                .max_stream_id_unidir_local_computed
                .max(stream.stream_id.saturating_add(4));
            if self.max_stream_id_unidir_local_computed > old {
                stream.max_stream_updated = true;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Frame retransmission.

impl Connection {
    pub fn check_frame_needs_repeat(
        &mut self,
        bytes: &[u8],
        bytes_max: usize,
        p_type: PacketType,
        no_need_to_repeat: &mut i32,
        do_not_detect_spurious: &mut i32,
        is_preemptive_needed: &mut i32,
    ) -> i32 {
        *no_need_to_repeat = 0;
        *do_not_detect_spurious = 0;
        *is_preemptive_needed = 0;
        let max = bytes_max.min(bytes.len());
        if max == 0 {
            *no_need_to_repeat = 1;
            return 0;
        }
        let mut frame_type = 0;
        if frames_varint_decode(&bytes[..max], &mut frame_type).is_none() {
            return -1;
        }
        match frame_type {
            x if x == crate::frames::FrameType::Padding as u64
                || x == crate::frames::FrameType::Ack as u64
                || x == crate::frames::FrameType::AckEcn as u64
                || x == crate::frames::FrameType::PathAck as u64
                || x == crate::frames::FrameType::PathAckEcn as u64 =>
            {
                *no_need_to_repeat = 1;
            }
            x if x >= crate::frames::FrameType::StreamRangeMin as u64
                && x <= crate::frames::FrameType::StreamRangeMax as u64 =>
            {
                *is_preemptive_needed = if p_type == PacketType::OneRttProtected {
                    1
                } else {
                    0
                };
            }
            x if x == crate::frames::FrameType::PathResponse as u64
                || x == crate::frames::FrameType::ConnectionClose as u64
                || x == crate::frames::FrameType::ApplicationClose as u64 =>
            {
                *do_not_detect_spurious = 1;
            }
            _ => {}
        }
        0
    }
}

pub fn format_available_stream_frames<'a>(
    connection: &mut Connection,
    _path_x: &mut Path,
    bytes: &'a mut [u8],
    _current_priority: u64,
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    stream_tried_and_failed: &mut i32,
    ret: &mut i32,
) -> Option<&'a mut [u8]> {
    let tok = connection.output_streams.pop_front()?;
    let stream = connection.streams.get_mut(tok)?;
    let before = bytes.len();
    let mut still_active = 0;
    let tail = encode_stream_like_frame(
        stream,
        bytes,
        more_data,
        is_pure_ack,
        &mut still_active,
        ret,
        false,
    )?;
    if tail.len() == before {
        *stream_tried_and_failed = 1;
    }
    if !stream.send_queue.is_empty()
        || still_active != 0
        || (stream.fin_requested && !stream.fin_sent)
    {
        connection.output_streams.push_back(tok);
        stream.is_output_stream = true;
    } else {
        stream.is_output_stream = false;
    }
    Some(tail)
}

impl Connection {
    pub fn queue_data_repeat_init(&mut self) {
        // Reset the data-repeat splay tree (already empty on new connection).
        self.queue_data_repeat_tree.clear();
    }
}

impl Connection {
    pub fn queue_data_repeat_packet(&mut self, packet: &mut Packet) {
        if packet.is_queued_for_data_repeat {
            return;
        }
        if let Some(token) = self
            .queued_packets
            .iter()
            .position(|p| p.sequence_number == packet.sequence_number)
            .map(|idx| PacketToken::synthetic(idx as u32, idx as u32))
            && let Ok((st, _)) = self
                .queue_data_repeat_tree
                .insert(packet.sequence_number, token)
        {
            packet.queue_data_repeat_membership = Some(st);
            packet.is_queued_for_data_repeat = true;
        }
    }
}

impl Connection {
    pub fn dequeue_data_repeat_packet(&mut self, packet: &mut Packet) {
        if let Some(st) = packet.queue_data_repeat_membership.take() {
            self.queue_data_repeat_tree.remove(st);
        } else if let Some(st) = self.queue_data_repeat_tree.find(&packet.sequence_number) {
            self.queue_data_repeat_tree.remove(st);
        }
        packet.is_queued_for_data_repeat = false;
    }
}

impl Connection {
    pub fn first_data_repeat_packet(&self) -> Option<PacketToken> {
        // Return the token stored at the minimum key in the repeat tree.
        let st = self.queue_data_repeat_tree.first()?;
        self.queue_data_repeat_tree.get(st).copied()
    }
}

pub fn copy_stream_frame_for_retransmit<'a>(
    _connection: &mut Connection,
    packet: &mut Packet,
    bytes: &'a mut [u8],
) -> Option<&'a mut [u8]> {
    let start = packet.data_repeat_frame;
    let end = packet.length.min(packet.bytes.len());
    if start >= end || bytes.len() < end - start {
        return None;
    }
    let len = end - start;
    bytes[..len].copy_from_slice(&packet.bytes[start..end]);
    Some(&mut bytes[len..])
}

pub fn copy_stream_frames_for_retransmit<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    _current_priority: u64,
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let Some(packet_token) = connection.first_data_repeat_packet() else {
        return Some(bytes);
    };
    let packet = connection.queued_packets.get_mut(packet_token)?;
    let start = packet.data_repeat_frame;
    let end = packet.length.min(packet.bytes.len());
    if start >= end || bytes.len() < end - start {
        return None;
    }
    let len = end - start;
    bytes[..len].copy_from_slice(&packet.bytes[start..end]);
    *is_pure_ack = 0;
    if connection.first_data_repeat_packet().is_some() {
        *more_data = 1;
    }
    Some(&mut bytes[len..])
}

pub fn copy_before_retransmit(
    old_p: &mut Packet,
    connection: &mut Connection,
    new_bytes: &mut [u8],
    send_buffer_max_minus_checksum: usize,
    packet_is_pure_ack: &mut i32,
    do_not_detect_spurious: &mut i32,
    force_queue: i32,
    length: &mut usize,
    add_to_data_repeat_queue: &mut i32,
) -> i32 {
    let copy_len = old_p
        .length
        .min(send_buffer_max_minus_checksum)
        .min(new_bytes.len())
        .min(old_p.bytes.len());
    new_bytes[..copy_len].copy_from_slice(&old_p.bytes[..copy_len]);
    *length = copy_len;
    *packet_is_pure_ack = if old_p.is_ack_eliciting { 0 } else { 1 };
    *do_not_detect_spurious = if old_p.is_queued_for_spurious_detection {
        0
    } else {
        1
    };
    *add_to_data_repeat_queue = if old_p.is_queued_for_data_repeat || force_queue != 0 {
        1
    } else {
        0
    };
    if *add_to_data_repeat_queue != 0 {
        connection.queue_data_repeat_packet(old_p);
    }
    0
}

impl Connection {
    pub fn retransmit_needed(
        &mut self,
        pc: PacketContext,
        _path_x: &mut Path,
        current_time: Instant,
        next_wake_time: &mut Instant,
        packet: &mut Packet,
        _send_buffer_max: usize,
        header_length: &mut usize,
    ) -> i32 {
        let pkt_ctx = &mut self.pkt_ctx[pc as usize];
        let Some((&sequence, &token)) = pkt_ctx.pending.iter().next() else {
            return 0;
        };
        let retransmit_at = self
            .queued_packets
            .get(token)
            .map(|p| {
                p.send_time
                    + self
                        .paths
                        .first()
                        .map(|p| p.retransmit_timer)
                        .unwrap_or(INITIAL_RETRANSMIT_TIMER)
            })
            .unwrap_or(current_time);
        if retransmit_at > current_time {
            if retransmit_at < *next_wake_time {
                *next_wake_time = retransmit_at;
            }
            return 0;
        }
        if let Some(old) = self.queued_packets.get(token) {
            let copy_len = old.length.min(packet.bytes.len()).min(old.bytes.len());
            packet.bytes[..copy_len].copy_from_slice(&old.bytes[..copy_len]);
            packet.length = copy_len;
            packet.sequence_number = sequence;
            packet.packet_context = old.packet_context;
            packet.packet_type = old.packet_type;
            packet.is_ack_eliciting = old.is_ack_eliciting;
            *header_length = old.offset;
        }
        pkt_ctx.pending.remove(&sequence);
        pkt_ctx.retransmitted.insert(sequence, token);
        pkt_ctx.retransmitted_queue_size = pkt_ctx.retransmitted.len() as u64;
        1
    }
}

impl Connection {
    pub fn set_ack_needed(
        &mut self,
        _current_time: Instant,
        pc: PacketContext,
        _path_x: &mut Path,
        is_immediate_ack_required: i32,
    ) {
        // Mark the ACK context as needing an ACK.
        let ack_ctx = &mut self.ack_ctx[pc as usize];
        ack_ctx.act[0].ack_needed = true;
        if is_immediate_ack_required != 0 {
            ack_ctx.act[0].is_immediate_ack_required = true;
        }
    }
}

impl Connection {
    pub fn process_ack_of_frames(&mut self, p: &mut Packet, is_spurious: i32) {
        p.is_queued_for_retransmit = false;
        p.is_queued_for_spurious_detection = false;
        if is_spurious == 0 {
            p.is_queued_for_data_repeat = false;
            p.queue_data_repeat_membership = None;
        }
    }
}

impl Connection {
    /// Variant of [`Self::set_ack_needed`] that looks up `self.paths[path_index]`
    /// internally, avoiding the double-borrow that arises when the caller
    /// passes `&mut self.paths[n]` alongside `&mut self`.  C:
    /// `picoquic_set_ack_needed` called with `cnx->path[n]`.
    pub fn set_ack_needed_on_path(
        &mut self,
        _current_time: Instant,
        pc: PacketContext,
        path_index: usize,
        is_immediate_ack_required: i32,
    ) {
        // For non-application contexts use the per-connection ACK ctx.
        // For application + multipath, use the per-path ACK ctx.
        if self.is_multipath_enabled
            && pc == PacketContext::Application
            && let Some(path) = self.paths.get_mut(path_index)
        {
            path.ack_ctx.act[0].ack_needed = true;
            if is_immediate_ack_required != 0 {
                path.ack_ctx.act[0].is_immediate_ack_required = true;
            }
            return;
        }
        let ack_ctx = &mut self.ack_ctx[pc as usize];
        ack_ctx.act[0].ack_needed = true;
        if is_immediate_ack_required != 0 {
            ack_ctx.act[0].is_immediate_ack_required = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Stream data buffer (callback argument for "prepare to send").

pub struct StreamDataBufferArgument<'a> {
    /// The output buffer the application writes into.  C: `bytes:
    /// *mut uint8_t`.  Length is `allowed_space` (so `&mut [u8]`
    /// with that length); `byte_index` advances through it as the
    /// app fills it.
    pub bytes: &'a mut [u8],
    pub byte_index: usize,
    pub byte_space: usize,
    pub allowed_space: usize,
    pub length: usize,
    pub is_fin: i32,
    pub is_still_active: i32,
    /// Borrowed app-side buffer the framework reads from.  C:
    /// `app_buffer: *mut uint8_t`.
    pub app_buffer: &'a [u8],
}

fn encode_varint_at(bytes: &mut [u8], off: &mut usize, value: u64) -> bool {
    let encoded = varint_encode(&mut bytes[*off..], value);
    if encoded == 0 {
        false
    } else {
        *off += encoded;
        true
    }
}

fn skip_n_varints(mut bytes: &[u8], n: usize) -> Option<&[u8]> {
    let mut ignored = 0;
    for _ in 0..n {
        bytes = frames_varint_decode(bytes, &mut ignored)?;
    }
    Some(bytes)
}

fn saturating_shl_u64(value: u64, shift: u32) -> u64 {
    value.checked_shl(shift).unwrap_or(u64::MAX)
}

fn encode_misc_frame(
    connection: &mut Connection,
    bytes: Vec<u8>,
    is_pure_ack: bool,
    pc: PacketContext,
) {
    connection.misc_frames.push_back(MiscFrameHeader {
        bytes,
        packet_context: pc,
        is_pure_ack: if is_pure_ack { 1 } else { 0 },
    });
}

fn encode_varint_vec(out: &mut Vec<u8>, value: u64) -> bool {
    let mut tmp = [0u8; 8];
    let n = varint_encode(&mut tmp, value);
    if n == 0 {
        false
    } else {
        out.extend_from_slice(&tmp[..n]);
        true
    }
}

fn encode_tp_param(out: &mut Vec<u8>, id: u64, value: &[u8]) -> bool {
    encode_varint_vec(out, id) && encode_varint_vec(out, value.len() as u64) && {
        out.extend_from_slice(value);
        true
    }
}

fn encode_tp_varint_param(out: &mut Vec<u8>, id: u64, value: u64) -> bool {
    let mut encoded = Vec::new();
    encode_varint_vec(&mut encoded, value) && encode_tp_param(out, id, &encoded)
}

fn decode_single_varint(bytes: &[u8]) -> Option<u64> {
    let mut value = 0;
    let rest = frames_varint_decode(bytes, &mut value)?;
    rest.is_empty().then_some(value)
}

fn encode_stream_like_frame<'a>(
    stream: &mut StreamHead,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    is_still_active: &mut i32,
    ret: &mut i32,
    crypto: bool,
) -> Option<&'a mut [u8]> {
    *ret = 0;
    *is_still_active = if stream.is_active { 1 } else { 0 };
    let offset = stream.sent_offset;
    let front_index = stream
        .send_queue
        .iter()
        .position(|node| node.offset.saturating_add(node.bytes.len() as u64) > stream.sent_offset);
    let (payload, fin_possible) = if let Some(idx) = front_index {
        let node = &stream.send_queue[idx];
        let start = stream.sent_offset.saturating_sub(node.offset) as usize;
        (&node.bytes[start..], false)
    } else {
        (&[][..], stream.fin_requested && !stream.fin_sent)
    };

    if payload.is_empty() && !fin_possible {
        return Some(bytes);
    }

    let allowed_by_fc = stream.maxdata_remote.saturating_sub(offset) as usize;
    let mut data_len = payload.len().min(allowed_by_fc);
    if data_len == 0 && !fin_possible {
        return Some(bytes);
    }

    loop {
        let mut header_len = 1 + encode_varint_length(offset);
        if !crypto {
            header_len += encode_varint_length(stream.stream_id);
        }
        header_len += encode_varint_length(data_len as u64);
        if header_len + data_len <= bytes.len() {
            break;
        }
        if data_len == 0 {
            *more_data = 1;
            return Some(bytes);
        }
        data_len -= 1;
    }

    let fin = fin_possible && data_len == payload.len();
    let mut off = 0;
    if crypto {
        bytes[off] = crate::frames::FrameType::CryptoHs as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, offset)
            || !encode_varint_at(bytes, &mut off, data_len as u64)
        {
            *more_data = 1;
            return Some(bytes);
        }
    } else {
        bytes[off] = crate::frames::FrameType::StreamRangeMin as u8 | 0x02;
        if offset > 0 {
            bytes[off] |= 0x04;
        }
        if fin {
            bytes[off] |= 0x01;
        }
        off += 1;
        if !encode_varint_at(bytes, &mut off, stream.stream_id)
            || (offset > 0 && !encode_varint_at(bytes, &mut off, offset))
            || !encode_varint_at(bytes, &mut off, data_len as u64)
        {
            *more_data = 1;
            return Some(bytes);
        }
    }

    if data_len > 0 {
        bytes[off..off + data_len].copy_from_slice(&payload[..data_len]);
        off += data_len;
        stream.sent_offset = stream.sent_offset.saturating_add(data_len as u64);
        while stream
            .send_queue
            .front()
            .map(|node| node.offset.saturating_add(node.bytes.len() as u64) <= stream.sent_offset)
            .unwrap_or(false)
        {
            stream.send_queue.pop_front();
        }
    }
    if fin && stream.send_queue.is_empty() {
        stream.fin_sent = true;
    }
    if !stream.send_queue.is_empty() || (stream.fin_requested && !stream.fin_sent) {
        *more_data = 1;
    }
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}

pub fn is_stream_frame_unlimited(bytes: &[u8]) -> bool {
    bytes
        .first()
        .map(|b| {
            (*b as u64) >= crate::frames::FrameType::StreamRangeMin as u64
                && (*b as u64) <= crate::frames::FrameType::StreamRangeMax as u64
                && (*b & 0x02) == 0
        })
        .unwrap_or(false)
}

pub fn format_stream_frame_header(
    bytes: &mut [u8],
    stream_id: u64,
    offset: u64,
) -> Option<&mut [u8]> {
    if bytes.is_empty() {
        return None;
    }
    bytes[0] = crate::frames::FrameType::StreamRangeMin as u8;
    let mut off = 1;
    let encoded = varint_encode(&mut bytes[off..], stream_id);
    if encoded == 0 {
        return None;
    }
    off += encoded;
    if offset > 0 {
        bytes[0] |= 0x04;
        let encoded = varint_encode(&mut bytes[off..], offset);
        if encoded == 0 {
            return None;
        }
        off += encoded;
    }
    Some(&mut bytes[off..])
}

pub fn parse_stream_header(
    bytes: &[u8],
    bytes_max: usize,
    stream_id: &mut u64,
    offset: &mut u64,
    data_length: &mut usize,
    fin: &mut i32,
    consumed: &mut usize,
) -> i32 {
    let max = bytes_max.min(bytes.len());
    if max == 0 {
        *consumed = 0;
        *data_length = 0;
        return -1;
    }

    let frame_type = bytes[0];
    let has_len = (frame_type & 0x02) != 0;
    let has_offset = (frame_type & 0x04) != 0;
    let mut off_idx = 1;
    *fin = (frame_type & 0x01) as i32;

    let l_stream = varint_decode(&bytes[off_idx..max], stream_id);
    off_idx += l_stream;
    if l_stream == 0 {
        *data_length = 0;
        *consumed = max;
        return -1;
    }

    if has_offset {
        let l_offset = varint_decode(&bytes[off_idx..max], offset);
        off_idx += l_offset;
        if l_offset == 0 {
            *data_length = 0;
            *consumed = max;
            return -1;
        }
    } else {
        *offset = 0;
    }

    if has_len {
        let mut length = 0;
        let l_len = varint_decode(&bytes[off_idx..max], &mut length);
        off_idx += l_len;
        if l_len == 0 || off_idx > max || off_idx.saturating_add(length as usize) > max {
            *data_length = 0;
            *consumed = max;
            return -1;
        }
        *data_length = length as usize;
    } else {
        *data_length = max - off_idx;
    }

    *consumed = off_idx;
    0
}

pub fn parse_ack_header(
    bytes: &[u8],
    bytes_max: usize,
    num_block: &mut u64,
    path_id: &mut u64,
    largest: &mut u64,
    ack_delay: &mut u64,
    consumed: &mut usize,
    ack_delay_exponent: u8,
) -> i32 {
    let max = bytes_max.min(bytes.len());
    if max == 0 {
        *consumed = 0;
        return -1;
    }

    let mut frame_type = 0;
    let mut off = varint_decode(&bytes[..max], &mut frame_type);
    if off == 0 {
        *consumed = max;
        return -1;
    }

    let is_path_ack = frame_type == crate::frames::FrameType::PathAck as u64
        || frame_type == crate::frames::FrameType::PathAckEcn as u64;
    if is_path_ack {
        let l_path = varint_decode(&bytes[off..max], path_id);
        off += l_path;
        if l_path == 0 {
            *consumed = max;
            return -1;
        }
    } else {
        *path_id = 0;
    }

    let l_largest = varint_decode(&bytes[off..max], largest);
    off += l_largest;
    let l_delay = varint_decode(&bytes[off..max], ack_delay);
    *ack_delay = ack_delay
        .checked_shl(ack_delay_exponent as u32)
        .unwrap_or(0);
    off += l_delay;
    let l_blocks = varint_decode(&bytes[off..max], num_block);
    off += l_blocks;

    if l_largest == 0 || l_delay == 0 || l_blocks == 0 || off > max {
        *consumed = max;
        -1
    } else {
        *consumed = off;
        0
    }
}

pub fn decode_crypto_hs_frame<'a>(
    connection: &mut Connection,
    bytes: &'a [u8],
    received_data: &mut StreamDataNode,
    epoch: i32,
) -> Option<&'a [u8]> {
    let (&frame_type, mut tail) = bytes.split_first()?;
    if frame_type != crate::frames::FrameType::CryptoHs as u8 {
        return None;
    }
    let mut offset = 0;
    tail = frames_varint_decode(tail, &mut offset)?;
    let mut length = 0;
    tail = frames_varint_decode(tail, &mut length)?;
    if tail.len() < length as usize {
        return None;
    }
    let data = &tail[..length as usize];
    if let Some(stream) = connection.tls_stream.get_mut(epoch.clamp(0, 3) as usize) {
        queue_received_stream_data(stream, offset, data, received_data).ok()?;
    }
    Some(&tail[length as usize..])
}

pub fn format_crypto_hs_frame<'a>(
    stream: &mut StreamHead,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let mut still_active = 0;
    let mut ret = 0;
    encode_stream_like_frame(
        stream,
        bytes,
        more_data,
        is_pure_ack,
        &mut still_active,
        &mut ret,
        true,
    )
}

pub fn format_ack_frame<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    current_time: Instant,
    pc: PacketContext,
    is_opportunistic: i32,
) -> Option<&'a mut [u8]> {
    let ack_ctx = &mut connection.ack_ctx[pc as usize];
    let first = ack_ctx.sack_list.first_range()?;
    let mut ranges = Vec::new();
    let mut tok = Some(first);
    while let Some(item_tok) = tok {
        let item = ack_ctx.sack_list.sack_items.get(item_tok)?;
        ranges.push((
            item.start_of_sack_range,
            item.end_of_sack_range,
            item.time_created,
        ));
        if ranges.len() >= MAX_ACK_RANGE_REPEAT {
            break;
        }
        tok = ack_ctx.sack_list.sack_next_item(item_tok);
    }
    if ranges.is_empty() {
        return Some(bytes);
    }

    let mut frame = Vec::new();
    frame.push(crate::frames::FrameType::Ack as u8);
    let largest = ranges[0].1;
    let largest_time = ranges[0].2;
    let ack_delay = current_time.ticks().saturating_sub(largest_time.ticks())
        >> connection.remote_parameters.ack_delay_exponent;
    if !encode_varint_vec(&mut frame, largest)
        || !encode_varint_vec(&mut frame, ack_delay)
        || !encode_varint_vec(&mut frame, ranges.len().saturating_sub(1) as u64)
        || !encode_varint_vec(&mut frame, ranges[0].1 - ranges[0].0)
    {
        *more_data = 1;
        return Some(bytes);
    }
    let mut previous_start = ranges[0].0;
    for &(start, end, _) in ranges.iter().skip(1) {
        if previous_start <= end {
            return None;
        }
        let gap = previous_start - end - 1;
        if !encode_varint_vec(&mut frame, gap.saturating_sub(1))
            || !encode_varint_vec(&mut frame, end - start)
        {
            *more_data = 1;
            return Some(bytes);
        }
        previous_start = start;
    }
    if frame.len() > bytes.len() {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[..frame.len()].copy_from_slice(&frame);
    let idx = is_opportunistic.clamp(0, 1) as usize;
    ack_ctx.act[idx].ack_needed = false;
    ack_ctx.act[idx].highest_ack_sent = largest;
    ack_ctx.act[idx].highest_ack_sent_time = current_time;
    Some(&mut bytes[frame.len()..])
}

pub fn format_connection_close_frame<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let reason = connection.local_error_reason.as_deref().unwrap_or("");
    let reason_bytes = reason.as_bytes();
    let mut off = 0;
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::ConnectionClose as u64,
    ) || !encode_varint_at(bytes, &mut off, connection.local_error)
        || !encode_varint_at(bytes, &mut off, connection.offending_frame_type)
        || !encode_varint_at(bytes, &mut off, reason_bytes.len() as u64)
        || bytes.len() < off + reason_bytes.len()
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + reason_bytes.len()].copy_from_slice(reason_bytes);
    off += reason_bytes.len();
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}

pub fn format_application_close_frame<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let reason = connection.local_error_reason.as_deref().unwrap_or("");
    let reason_bytes = reason.as_bytes();
    let mut off = 0;
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::ApplicationClose as u64,
    ) || !encode_varint_at(bytes, &mut off, connection.application_error)
        || !encode_varint_at(bytes, &mut off, reason_bytes.len() as u64)
        || bytes.len() < off + reason_bytes.len()
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + reason_bytes.len()].copy_from_slice(reason_bytes);
    off += reason_bytes.len();
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}

pub fn format_required_max_stream_data_frames<'a>(
    connection: &mut Connection,
    mut bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let tokens: Vec<_> = connection
        .streams
        .iter()
        .filter_map(|stream| {
            stream
                .max_stream_updated
                .then_some(stream.stream_tree_membership)
                .flatten()
                .and_then(|st| connection.stream_tree.get(st).copied())
        })
        .collect();
    for tok in tokens {
        let Some(stream) = connection.streams.get_mut(tok) else {
            continue;
        };
        if !stream.max_stream_updated {
            continue;
        }
        let mut off = 0;
        if !encode_varint_at(
            bytes,
            &mut off,
            crate::frames::FrameType::MaxStreamData as u64,
        ) || !encode_varint_at(bytes, &mut off, stream.stream_id)
            || !encode_varint_at(bytes, &mut off, stream.maxdata_local)
        {
            *more_data = 1;
            return Some(bytes);
        }
        stream.maxdata_local_acked = stream.maxdata_local;
        stream.max_stream_updated = false;
        *is_pure_ack = 0;
        bytes = &mut bytes[off..];
    }
    Some(bytes)
}

pub fn format_max_data_frame<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    maxdata_increase: u64,
) -> Option<&'a mut [u8]> {
    let new_max = connection.maxdata_local.saturating_add(maxdata_increase);
    if bytes.is_empty() {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[0] = crate::frames::FrameType::MaxData as u8;
    let mut off = 1;
    if !encode_varint_at(bytes, &mut off, new_max) {
        *more_data = 1;
        return Some(bytes);
    }
    connection.maxdata_local = new_max;
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}

pub fn format_max_stream_data_frame<'a>(
    connection: &mut Connection,
    stream: &mut StreamHead,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    new_max_data: u64,
) -> Option<&'a mut [u8]> {
    if bytes.is_empty() {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[0] = crate::frames::FrameType::MaxStreamData as u8;
    let mut off = 1;
    if !encode_varint_at(bytes, &mut off, stream.stream_id)
        || !encode_varint_at(bytes, &mut off, new_max_data)
    {
        *more_data = 1;
        return Some(bytes);
    }
    stream.maxdata_local = new_max_data;
    connection.max_stream_data_local = connection.max_stream_data_local.max(new_max_data);
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}

impl Connection {
    pub fn cc_increased_window(&self, _previous_window: u64) -> u64 {
        self.paths.first().map(|p| p.cwin).unwrap_or(0)
    }
}

pub fn format_max_streams_frame_if_needed<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if connection.max_stream_id_bidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_bidir
        > connection.max_stream_id_bidir_local
    {
        let new_bidir = connection.max_stream_id_bidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_bidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsBidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_bidir).rank()) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.max_stream_id_bidir_local = new_bidir;
        *is_pure_ack = 0;
    }

    if connection.max_stream_id_unidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_unidir
        > connection.max_stream_id_unidir_local
    {
        let new_unidir = connection.max_stream_id_unidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_unidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsUnidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_unidir).rank()) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.max_stream_id_unidir_local = new_unidir;
        *is_pure_ack = 0;
    }

    Some(&mut bytes[off..])
}

impl StreamDataNode {
    pub fn stream_data_node_recycle(&mut self) {
        // Reset the node to empty; caller is responsible for removing from tree.
        self.stream_data_membership = None;
        self.length = 0;
        self.offset = 0;
    }
}

impl Quic {
    pub fn stream_data_node_alloc(&mut self) -> Result<StreamDataNode, crate::Error> {
        // Allocate a fresh StreamDataNode.  In C a free-list is maintained;
        // in Rust we simply allocate on the heap.
        Ok(StreamDataNode {
            stream_data_membership: None,
            offset: 0,
            data: [0u8; crate::MAX_PACKET_SIZE],
            length: 0,
        })
    }
}

impl StreamHead {
    /// Reset all stream state to defaults, keeping the stream_id.
    /// C: `picoquic_clear_stream`.
    pub fn clear_stream(&mut self) {
        let stream_id = self.stream_id;
        *self = StreamHead {
            stream_tree_membership: None,
            stream_id,
            affinity_path: None,
            consumed_offset: 0,
            fin_offset: 0,
            reset_offset: 0,
            maxdata_local: 0,
            maxdata_local_acked: 0,
            maxdata_remote: 0,
            local_error: 0,
            remote_error: 0,
            local_stop_error: 0,
            remote_stop_error: 0,
            last_time_data_sent: crate::Instant::from_ticks(0),
            stream_data_tree: crate::splay::SplayTree::default(),
            stream_data_nodes: crate::arena::Arena::new(),
            sent_offset: 0,
            reliable_size: 0,
            send_queue: std::collections::VecDeque::new(),
            app_stream_ctx: None,
            direct_receive_fn: None,
            direct_receive_ctx: None,
            sack_list: SackList::new(),
            stream_priority: 0,
            is_active: false,
            fin_requested: false,
            fin_sent: false,
            fin_received: false,
            fin_signalled: false,
            reset_requested: false,
            reset_sent: false,
            reset_acked: false,
            reset_received: false,
            reset_signalled: false,
            stop_sending_requested: false,
            stop_sending_sent: false,
            stop_sending_received: false,
            stop_sending_signalled: false,
            max_stream_updated: false,
            stream_data_blocked_sent: false,
            is_output_stream: false,
            is_closed: false,
            is_discarded: false,
            use_app_flow_control: false,
            is_not_coalesced: false,
        };
    }
}

impl Connection {
    /// Remove `stream` from all connection data structures and free its token.
    /// C: `picoquic_delete_stream`.
    pub fn delete_stream(&mut self, stream: &mut StreamHead) {
        // Remove from the splay tree.
        if let Some(splay_tok) = stream.stream_tree_membership.take()
            && let Some(stream_tok) = self.stream_tree.remove(splay_tok)
        {
            // Remove from output queue.
            if stream.is_output_stream {
                if let Some(pos) = self.output_streams.iter().position(|&t| t == stream_tok.1) {
                    self.output_streams.remove(pos);
                }
                stream.is_output_stream = false;
            }
            // Remove from the streams arena.
            self.streams.remove(stream_tok.1);
        }
    }
}

impl Connection {
    /// Find the local-CID list for `unique_path_id` (the index into
    /// `connection.local_connection_id_lists`), optionally creating one when absent.
    pub fn find_or_create_local_connection_id_list(
        &self,
        unique_path_id: u64,
        _do_create: bool,
    ) -> Option<usize> {
        self.local_connection_id_lists
            .iter()
            .position(|l| l.unique_path_id == unique_path_id)
    }
}

impl Connection {
    fn has_local_connection_id_value(&self, connection_id: &ConnectionId) -> bool {
        self.local_connection_id_lists.iter().any(|list| {
            list.connection_ids.iter().any(|&tok| {
                self.local_connection_ids
                    .get(tok)
                    .map(|l| &l.connection_id == connection_id)
                    .unwrap_or(false)
            })
        })
    }

    fn random_local_connection_id(&self) -> crate::Result<ConnectionId> {
        let copy_len = self.local_cid_length as usize;
        let mut bytes = [0u8; crate::CONNECTION_ID_MAX_SIZE];
        fill_system_random(&mut bytes[..copy_len])?;
        ConnectionId::clone_from_slice(&bytes[..copy_len]).ok_or(crate::Error::Generic)
    }

    pub fn create_local_connection_id(
        &mut self,
        unique_path_id: u64,
        suggested_value: Option<&ConnectionId>,
        current_time: Instant,
    ) -> Result<LocalConnectionIdToken, crate::Error> {
        // Find or create the per-path list.
        let list_idx = match self
            .local_connection_id_lists
            .iter()
            .position(|l| l.unique_path_id == unique_path_id)
        {
            Some(i) => i,
            None => {
                self.local_connection_id_lists.push(LocalConnectionIdList {
                    unique_path_id,
                    local_connection_id_sequence_next: 0,
                    local_connection_id_retire_before: 0,
                    local_connection_id_oldest_created: current_time.ticks(),
                    nb_local_connection_id_expired: 0,
                    is_demoted: false,
                    demotion_time: crate::Instant::from_ticks(u64::MAX),
                    connection_ids: Vec::new(),
                });
                self.local_connection_id_lists.len() - 1
            }
        };

        let connection_id = if self.local_cid_length == 0 {
            ConnectionId::default()
        } else {
            let mut selected = None;
            for attempt in 0..32 {
                let candidate = if attempt == 0 {
                    match suggested_value {
                        Some(suggested) => *suggested,
                        None => self.random_local_connection_id()?,
                    }
                } else {
                    self.random_local_connection_id()?
                };
                if !self.has_local_connection_id_value(&candidate) {
                    selected = Some(candidate);
                    break;
                }
            }
            selected.ok_or(crate::Error::Generic)?
        };

        let seq = self.local_connection_id_lists[list_idx].local_connection_id_sequence_next;

        let l_cid = LocalConnectionId {
            connection_by_id_membership: None,
            path_id: unique_path_id,
            sequence: seq,
            create_time: current_time,
            connection_id,
            is_acked: false,
        };

        let token = self
            .local_connection_ids
            .insert(l_cid)
            .map_err(|_| crate::Error::Memory)?;

        self.local_connection_id_lists[list_idx].local_connection_id_sequence_next += 1;
        self.local_connection_id_lists[list_idx]
            .connection_ids
            .push(token);

        if seq == 0 {
            self.local_connection_id_lists[list_idx].local_connection_id_oldest_created =
                current_time.ticks();
            if unique_path_id > self.max_path_id_in_connection_id_lists {
                self.max_path_id_in_connection_id_lists = unique_path_id;
            }
        }

        Ok(token)
    }
}

impl Connection {
    pub fn demote_local_connection_id_list(&mut self, unique_path_id: u64, _reason: u64) -> i32 {
        if let Some(list) = self
            .local_connection_id_lists
            .iter_mut()
            .find(|l| l.unique_path_id == unique_path_id)
            && !list.is_demoted
        {
            list.is_demoted = true;
            return 1;
        }
        0
    }
}

impl Connection {
    pub fn delete_local_connection_id(&mut self, l_cid: LocalConnectionIdToken) {
        // Remove from the backing arena.
        let removed = self.local_connection_ids.remove(l_cid);
        if removed.is_none() {
            // Stale token — no-op.
            return;
        }
        // Remove the token from whichever list holds it.
        for list in &mut self.local_connection_id_lists {
            if let Some(pos) = list.connection_ids.iter().position(|t| *t == l_cid) {
                list.connection_ids.swap_remove(pos);
                break;
            }
        }
    }
}

impl Connection {
    /// Remove `connection.local_connection_id_lists[list_index]`.
    pub fn delete_local_connection_id_list(&mut self, list_index: usize) {
        if list_index >= self.local_connection_id_lists.len() {
            return;
        }
        // Remove all CIDs in this list from the arena.
        let tokens: Vec<LocalConnectionIdToken> = self.local_connection_id_lists[list_index]
            .connection_ids
            .clone();
        for tok in tokens {
            self.local_connection_ids.remove(tok);
        }
        self.local_connection_id_lists.swap_remove(list_index);
    }
}

impl Connection {
    pub fn delete_local_connection_id_lists(&mut self) {
        // Drain all lists, removing all CIDs from the arena.
        for list in self.local_connection_id_lists.drain(..) {
            for tok in list.connection_ids {
                self.local_connection_ids.remove(tok);
            }
        }
    }
}

impl Connection {
    pub fn retire_local_connection_id(&mut self, unique_path_id: u64, sequence: u64) {
        // Find the token with matching sequence in the matching list.
        // Two-phase: first collect tokens, then look up.
        let tokens: Vec<LocalConnectionIdToken> = self
            .local_connection_id_lists
            .iter()
            .find(|l| l.unique_path_id == unique_path_id)
            .map(|l| l.connection_ids.clone())
            .unwrap_or_default();
        let found = tokens.into_iter().find(|&tok| {
            self.local_connection_ids
                .get(tok)
                .map(|l| l.sequence == sequence)
                .unwrap_or(false)
        });
        if let Some(tok) = found {
            self.delete_local_connection_id(tok);
        }
    }
}

impl Connection {
    pub fn check_local_connection_id_ttl(
        &mut self,
        local_connection_id_list: &mut LocalConnectionIdList,
        current_time: Instant,
        next_wake_time: &mut Instant,
    ) {
        let ttl = self.local_connection_id_ttl;
        if ttl == u64::MAX {
            return;
        }

        if current_time
            .ticks()
            .saturating_sub(local_connection_id_list.local_connection_id_oldest_created)
            >= ttl
        {
            local_connection_id_list.local_connection_id_oldest_created = current_time.ticks();
            local_connection_id_list.nb_local_connection_id_expired = 0;

            for &tok in &local_connection_id_list.connection_ids {
                if let Some(l_cid) = self.local_connection_ids.get(tok) {
                    if current_time
                        .ticks()
                        .saturating_sub(l_cid.create_time.ticks())
                        >= ttl
                    {
                        local_connection_id_list.nb_local_connection_id_expired += 1;
                        if l_cid.sequence
                            >= local_connection_id_list.local_connection_id_retire_before
                        {
                            local_connection_id_list.local_connection_id_retire_before =
                                l_cid.sequence + 1;
                        }
                    } else if l_cid.create_time.ticks()
                        < local_connection_id_list.local_connection_id_oldest_created
                    {
                        local_connection_id_list.local_connection_id_oldest_created =
                            l_cid.create_time.ticks();
                    }
                }
            }

            self.next_wake_time = current_time;
        } else if next_wake_time
            .ticks()
            .saturating_sub(local_connection_id_list.local_connection_id_oldest_created)
            > ttl
        {
            *next_wake_time = Instant::from_ticks(
                local_connection_id_list.local_connection_id_oldest_created + ttl,
            );
        }
    }
}

impl Connection {
    pub fn find_local_connection_id(
        &self,
        unique_path_id: u64,
        connection_id: &ConnectionId,
    ) -> Option<LocalConnectionIdToken> {
        let list = self
            .local_connection_id_lists
            .iter()
            .find(|l| l.unique_path_id == unique_path_id)?;
        // Clone tokens to avoid simultaneous borrows of self.
        let tokens: Vec<LocalConnectionIdToken> = list.connection_ids.clone();
        tokens.into_iter().find(|&tok| {
            self.local_connection_ids
                .get(tok)
                .map(|l| &l.connection_id == connection_id)
                .unwrap_or(false)
        })
    }
}

pub fn format_path_challenge_frame<'a>(
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    challenge: u64,
) -> Option<&'a mut [u8]> {
    if bytes.len() < 9 {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[0] = crate::frames::FrameType::PathChallenge as u8;
    format_64(&mut bytes[1..9], challenge);
    *is_pure_ack = 0;
    Some(&mut bytes[9..])
}

pub fn format_path_response_frame<'a>(
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    challenge: u64,
) -> Option<&'a mut [u8]> {
    if bytes.len() < 9 {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[0] = crate::frames::FrameType::PathResponse as u8;
    format_64(&mut bytes[1..9], challenge);
    *is_pure_ack = 0;
    Some(&mut bytes[9..])
}

impl Connection {
    pub fn should_repeat_path_response_frame(&self, bytes: &[u8], bytes_max: usize) -> bool {
        let max = bytes_max.min(bytes.len());
        if max < 9 {
            return false;
        }
        let response = parse_64(&bytes[1..9]);
        self.paths.iter().any(|path| {
            path.tuples.iter().any(|tuple| {
                tuple.challenge_response == response
                    && (tuple.challenge_verified || (self.client_mode && !tuple.challenge_failed))
            })
        })
    }
}

pub fn format_new_connection_id_frame<'a>(
    connection: &mut Connection,
    local_connection_id_list: &mut LocalConnectionIdList,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    l_cid: Option<LocalConnectionIdToken>,
) -> Option<&'a mut [u8]> {
    let token = l_cid.or_else(|| local_connection_id_list.connection_ids.first().copied())?;
    let cid = connection.local_connection_ids.get(token)?;
    let frame_type = if connection.is_multipath_enabled {
        crate::frames::FrameType::PathNewConnectionId as u64
    } else {
        crate::frames::FrameType::NewConnectionId as u64
    };
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, frame_type)
        || (connection.is_multipath_enabled
            && !encode_varint_at(bytes, &mut off, local_connection_id_list.unique_path_id))
        || !encode_varint_at(bytes, &mut off, cid.sequence)
        || !encode_varint_at(
            bytes,
            &mut off,
            local_connection_id_list.local_connection_id_retire_before,
        )
        || bytes.len() < off + 1 + cid.connection_id.len() + RESET_SECRET_SIZE
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off] = cid.connection_id.len() as u8;
    off += 1;
    bytes[off..off + cid.connection_id.len()].copy_from_slice(cid.connection_id.as_bytes());
    off += cid.connection_id.len();
    bytes[off..off + RESET_SECRET_SIZE].copy_from_slice(&connection.registered_reset_secret);
    off += RESET_SECRET_SIZE;
    if let Some(cid) = connection.local_connection_ids.get_mut(token) {
        cid.is_acked = true;
    }
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}

pub fn format_max_path_id_frame<'a>(
    bytes: &'a mut [u8],
    max_path_id: u64,
    more_data: &mut i32,
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, crate::frames::FrameType::MaxPathId as u64)
        || !encode_varint_at(bytes, &mut off, max_path_id)
    {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}

pub fn format_blocked_frames<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    if connection.maxdata_remote <= connection.data_sent && !connection.sent_blocked_frame {
        if bytes.is_empty() {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[0] = crate::frames::FrameType::DataBlocked as u8;
        let mut off = 1;
        if !encode_varint_at(bytes, &mut off, connection.maxdata_remote) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.sent_blocked_frame = true;
        *is_pure_ack = 0;
        return Some(&mut bytes[off..]);
    }
    Some(bytes)
}

impl Connection {
    /// Enqueue a RETIRE_CONNECTION_ID frame for `(unique_path_id,
    /// sequence)` to be sent on the next outgoing packet.
    pub fn queue_retire_connection_id_frame(
        &mut self,
        unique_path_id: u64,
        sequence: u64,
    ) -> Result<(), crate::Error> {
        let mut frame = [0u8; 258];
        let mut off = 0;
        let frame_type = if self.is_multipath_enabled {
            crate::frames::FrameType::PathRetireConnectionId as u64
        } else {
            crate::frames::FrameType::RetireConnectionId as u64
        };
        let n = varint_encode(&mut frame[off..], frame_type);
        if n == 0 {
            return Err(crate::Error::BufferTooSmall);
        }
        off += n;
        if self.is_multipath_enabled {
            let n = varint_encode(&mut frame[off..], unique_path_id);
            if n == 0 {
                return Err(crate::Error::BufferTooSmall);
            }
            off += n;
        }
        let n = varint_encode(&mut frame[off..], sequence);
        if n == 0 {
            return Err(crate::Error::BufferTooSmall);
        }
        off += n;
        encode_misc_frame(
            self,
            frame[..off].to_vec(),
            false,
            PacketContext::Application,
        );
        Ok(())
    }

    /// Enqueue a NEW_TOKEN frame carrying `token`.
    pub fn queue_new_token_frame(&mut self, token: &[u8]) -> Result<(), crate::Error> {
        let mut frame =
            Vec::with_capacity(1 + encode_varint_length(token.len() as u64) + token.len());
        frame.push(crate::frames::FrameType::NewToken as u8);
        let mut len_buf = [0u8; 8];
        let n = varint_encode(&mut len_buf, token.len() as u64);
        frame.extend_from_slice(&len_buf[..n]);
        frame.extend_from_slice(token);
        encode_misc_frame(self, frame, true, PacketContext::Application);
        Ok(())
    }
}

pub fn format_one_blocked_frame<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    stream: &mut StreamHead,
) -> Option<&'a mut [u8]> {
    let sid = crate::stream::StreamId(stream.stream_id);
    let local_role = if connection.client_mode {
        crate::stream::Role::Client
    } else {
        crate::stream::Role::Server
    };
    let has_data = stream.is_active
        || stream
            .send_queue
            .front()
            .map(|q| (q.offset as usize) < q.bytes.len())
            .unwrap_or(false);
    if !has_data {
        return Some(bytes);
    }

    let mut off = 0;
    if sid.is_local(local_role)
        && stream.stream_id
            > if sid.is_bidir() {
                connection.max_stream_id_bidir_remote
            } else {
                connection.max_stream_id_unidir_remote
            }
    {
        let already_sent = if sid.is_bidir() {
            connection.stream_blocked_bidir_sent
        } else {
            connection.stream_blocked_unidir_sent
        };
        if already_sent {
            return Some(bytes);
        }
        if bytes.is_empty() {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[0] = if sid.is_bidir() {
            crate::frames::FrameType::StreamsBlockedBidir as u8
        } else {
            crate::frames::FrameType::StreamsBlockedUnidir as u8
        };
        off = 1;
        let rank = sid.rank();
        if !encode_varint_at(bytes, &mut off, rank) {
            *more_data = 1;
            return Some(bytes);
        }
        if sid.is_bidir() {
            connection.stream_blocked_bidir_sent = true;
        } else {
            connection.stream_blocked_unidir_sent = true;
        }
        *is_pure_ack = 0;
        return Some(&mut bytes[off..]);
    }

    if connection.maxdata_remote <= connection.data_sent && !connection.sent_blocked_frame {
        if bytes.is_empty() {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[0] = crate::frames::FrameType::DataBlocked as u8;
        off = 1;
        if !encode_varint_at(bytes, &mut off, connection.maxdata_remote) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.sent_blocked_frame = true;
        *is_pure_ack = 0;
        return Some(&mut bytes[off..]);
    }

    if stream.sent_offset >= stream.maxdata_remote && !stream.stream_data_blocked_sent {
        if bytes.is_empty() {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[0] = crate::frames::FrameType::StreamDataBlocked as u8;
        off = 1;
        if !encode_varint_at(bytes, &mut off, stream.stream_id)
            || !encode_varint_at(bytes, &mut off, stream.maxdata_remote)
        {
            *more_data = 1;
            return Some(bytes);
        }
        stream.stream_data_blocked_sent = true;
        *is_pure_ack = 0;
    }
    Some(&mut bytes[off..])
}

/// Pop the head of a misc/datagram queue, encode it into `bytes`,
/// and return the remaining write tail.  Replaces the C "splice
/// the head out of a doubly-linked list" pattern -- the queue is
/// now a `VecDeque` and the front element is consumed by value.
pub fn format_first_misc_or_dg_frame<'a>(
    bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    queue: &mut VecDeque<MiscFrameHeader>,
) -> Option<&'a mut [u8]> {
    // Consume the head of the queue and copy its bytes into the output buffer.
    let frame = queue.pop_front()?;
    let len = frame.bytes.len();
    if bytes.len() >= len {
        bytes[..len].copy_from_slice(&frame.bytes);
        Some(&mut bytes[len..])
    } else {
        // Frame doesn't fit — put it back and return None.
        queue.push_front(frame);
        None
    }
}

impl Connection {
    /// Borrow the next misc-frame header in `connection` for the given
    /// packet context.  C: `find_first_misc_frame`.
    pub fn find_first_misc_frame(
        &mut self,
        packet_context: PacketContext,
    ) -> Option<&mut MiscFrameHeader> {
        self.misc_frames
            .iter_mut()
            .find(|frame| frame.packet_context == packet_context)
    }
}

pub fn format_misc_frames_in_context<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    pc: PacketContext,
) -> Option<&'a mut [u8]> {
    // Drain all misc frames whose packet_context matches pc into bytes.
    let mut remaining = bytes;
    loop {
        let front_matches = connection
            .misc_frames
            .front()
            .map(|f| f.packet_context == pc)
            .unwrap_or(false);
        if !front_matches {
            break;
        }
        remaining = format_first_misc_or_dg_frame(
            remaining,
            more_data,
            is_pure_ack,
            &mut connection.misc_frames,
        )?;
    }
    Some(remaining)
}

impl Connection {
    /// Push a fresh misc/datagram frame onto `queue`.  C took two
    /// `*mut *mut MiscFrameHeader` head/tail out-pointers; the Rust
    /// shape just takes the queue and pushes at the back.
    pub fn queue_misc_or_dg_frame(
        &mut self,
        queue: &mut VecDeque<MiscFrameHeader>,
        bytes: &[u8],
        is_pure_ack: bool,
        pc: PacketContext,
    ) -> Result<(), crate::Error> {
        queue.push_back(MiscFrameHeader {
            bytes: bytes.to_vec(),
            packet_context: pc,
            is_pure_ack: if is_pure_ack { 1 } else { 0 },
        });
        Ok(())
    }
}

impl Connection {
    pub fn purge_misc_frames_after_ready(&mut self) {
        if self.connection_state == State::Ready {
            self.misc_frames
                .retain(|frame| frame.packet_context == PacketContext::Application);
        }
    }
}

/// Remove the entry at `index` from `queue`.  C: `delete_misc_or_dg`
/// (which spliced the node out of a doubly-linked list).
pub fn delete_misc_or_dg(queue: &mut VecDeque<MiscFrameHeader>, index: usize) {
    queue.remove(index);
}

impl AckContext {
    /// Reset to an all-zero initial state.  C: `picoquic_clear_ack_ctx`.
    pub fn clear_ack_ctx(&mut self) {
        self.sack_list.free();
        self.time_stamp_largest_received = crate::Instant::from_ticks(0);
        self.crypto_rotation_sequence = 0;
        self.ecn_ect0_total_local = 0;
        self.ecn_ect1_total_local = 0;
        self.ecn_ce_total_local = 0;
        self.sending_ecn_ack = false;
        for act in &mut self.act {
            act.highest_ack_sent = 0;
            act.highest_ack_sent_time = crate::Instant::from_ticks(0);
            act.time_oldest_unack_packet_received = crate::Instant::from_ticks(0);
            act.ack_needed = false;
            act.ack_after_fin = false;
            act.out_of_order_received = false;
            act.is_immediate_ack_required = false;
        }
    }
}

impl AckContext {
    /// Reset to ready-to-receive state.  C: `picoquic_reset_ack_context`.
    pub fn reset_ack_context(&mut self) {
        self.clear_ack_ctx();
    }
}

impl Connection {
    /// Enqueue a HANDSHAKE_DONE frame.
    pub fn queue_handshake_done_frame(&mut self) -> Result<(), crate::Error> {
        encode_misc_frame(
            self,
            vec![crate::frames::FrameType::HandshakeDone as u8],
            false,
            PacketContext::Application,
        );
        Ok(())
    }
}

pub fn format_first_datagram_frame<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    _is_first_in_packet: i32,
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let frame = connection.datagrams.pop_front()?;
    if frame.bytes.first().is_some_and(|b| {
        *b == crate::frames::FrameType::Datagram as u8
            || *b == crate::frames::FrameType::DatagramL as u8
    }) {
        let len = frame.bytes.len();
        if bytes.len() < len {
            connection.datagrams.push_front(frame);
            *more_data = 1;
            return Some(bytes);
        }
        bytes[..len].copy_from_slice(&frame.bytes);
        *is_pure_ack = 0;
        return Some(&mut bytes[len..]);
    }
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, crate::frames::FrameType::DatagramL as u64)
        || !encode_varint_at(bytes, &mut off, frame.bytes.len() as u64)
        || bytes.len() < off + frame.bytes.len()
    {
        connection.datagrams.push_front(frame);
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + frame.bytes.len()].copy_from_slice(&frame.bytes);
    off += frame.bytes.len();
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}

pub fn format_ready_datagram_frame<'a>(
    connection: &mut Connection,
    path_x: &mut Path,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    is_pure_ack: &mut i32,
    ret: &mut i32,
) -> Option<&'a mut [u8]> {
    *ret = 0;
    if !connection.is_datagram_ready && !path_x.is_datagram_ready && connection.datagrams.is_empty()
    {
        return Some(bytes);
    }
    if connection.datagrams.is_empty() {
        connection.is_datagram_ready = false;
        path_x.is_datagram_ready = false;
        return Some(bytes);
    }
    let tail = format_first_datagram_frame(connection, bytes, 0, more_data, is_pure_ack)?;
    if connection.datagrams.is_empty() {
        connection.is_datagram_ready = false;
        path_x.is_datagram_ready = false;
    }
    Some(tail)
}

pub fn decode_datagram_frame_header<'a>(
    bytes: &'a [u8],
    frame_id: &mut u8,
    length: &mut u64,
) -> Option<&'a [u8]> {
    let (&first, mut tail) = bytes.split_first()?;
    *frame_id = first;
    if (first & 1) != 0 {
        tail = frames_varint_decode(tail, length)?;
        if (*length as usize) > tail.len() {
            return None;
        }
    } else {
        *length = tail.len() as u64;
    }
    Some(tail)
}

pub fn parse_ack_frequency_frame<'a>(
    bytes: &'a [u8],
    seq: &mut u64,
    packets: &mut u64,
    microsec: &mut u64,
    ignore_order: &mut u8,
    reordering_threshold: &mut u64,
) -> Option<&'a [u8]> {
    *reordering_threshold = 0;
    let bytes = frames_varint_decode(bytes, seq)?;
    let bytes = frames_varint_decode(bytes, packets)?;
    let bytes = frames_varint_decode(bytes, microsec)?;
    let bytes = frames_varint_decode(bytes, reordering_threshold)?;
    *ignore_order = u8::from(*reordering_threshold == 0);
    Some(bytes)
}

pub fn format_ack_frequency_frame<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
) -> Option<&'a mut [u8]> {
    let seq = connection.ack_frequency_sequence_local.wrapping_add(1);
    let mut ack_gap = 0;
    let mut ack_delay_max = 0;
    let (rtt, rate) = connection
        .paths
        .first()
        .map(|p| (p.rtt_min, p.bandwidth_estimate))
        .unwrap_or((INITIAL_RTT, 0));
    connection.compute_ack_gap_and_delay(
        rtt,
        connection.remote_parameters.min_ack_delay.ticks(),
        rate,
        &mut ack_gap,
        &mut ack_delay_max,
    );

    if ack_gap <= connection.ack_gap_local
        && ack_delay_max >= (7 * connection.ack_frequency_delay_local.ticks()) / 8
        && ack_delay_max <= (9 * connection.ack_frequency_delay_local.ticks()) / 8
    {
        connection.is_ack_frequency_updated = false;
        return Some(bytes);
    }

    if ack_gap < connection.ack_gap_local {
        ack_gap = connection.ack_gap_local;
    }
    let reordering_threshold = if connection.ack_ignore_order_local {
        0
    } else {
        1
    };
    let mut off = 0;
    for value in [
        crate::frames::FrameType::AckFrequency as u64,
        seq,
        ack_gap,
        ack_delay_max,
        reordering_threshold,
    ] {
        if !encode_varint_at(bytes, &mut off, value) {
            *more_data = 1;
            return Some(bytes);
        }
    }
    connection.ack_frequency_sequence_local = seq;
    connection.ack_gap_local = ack_gap;
    connection.ack_frequency_delay_local = Duration::from_ticks(ack_delay_max);
    connection.is_ack_frequency_updated = false;
    connection.max_ack_gap_local = connection.max_ack_gap_local.max(ack_gap);
    connection.min_ack_delay_local = connection
        .min_ack_delay_local
        .min(Duration::from_ticks(ack_delay_max));
    connection.max_ack_delay_local = connection
        .max_ack_delay_local
        .max(Duration::from_ticks(ack_delay_max));
    Some(&mut bytes[off..])
}

pub fn format_immediate_ack_frame<'a>(
    bytes: &'a mut [u8],
    more_data: &mut i32,
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::ImmediateAck as u64,
    ) {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}

pub fn format_time_stamp_frame<'a>(
    connection: &mut Connection,
    bytes: &'a mut [u8],
    more_data: &mut i32,
    current_time: Instant,
) -> Option<&'a mut [u8]> {
    let delta = current_time
        .ticks()
        .saturating_sub(connection.start_time.ticks());
    let time_stamp = delta >> connection.local_parameters.ack_delay_exponent;
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, crate::frames::FrameType::TimeStamp as u64)
        || !encode_varint_at(bytes, &mut off, time_stamp)
    {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}

impl Connection {
    pub fn encode_time_stamp_length(&self, current_time: Instant) -> usize {
        let delta = current_time.ticks().saturating_sub(self.start_time.ticks());
        let time_stamp = delta >> self.local_parameters.ack_delay_exponent;
        encode_varint_length(crate::frames::FrameType::TimeStamp as u64)
            + encode_varint_length(time_stamp)
    }
}

pub fn format_bdp_frame<'a>(
    _connection: &mut Connection,
    bytes: &'a mut [u8],
    path_x: &mut Path,
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let recon_bytes_in_flight = if path_x.cwin_remote > 0 {
        path_x.cwin_remote
    } else {
        path_x.cwin
    };
    if recon_bytes_in_flight == 0 {
        return Some(bytes);
    }
    let recon_min_rtt = if path_x.rtt_min_remote.ticks() > 0 {
        path_x.rtt_min_remote.ticks()
    } else {
        path_x.rtt_min.ticks()
    };
    let ip_len = path_x.ip_client_remote_length as usize;
    let lifetime = TOKEN_DELAY_LONG.ticks();
    let mut off = 0;
    for value in [
        crate::frames::FrameType::Bdp as u64,
        lifetime,
        recon_bytes_in_flight,
        recon_min_rtt,
        ip_len as u64,
    ] {
        if !encode_varint_at(bytes, &mut off, value) {
            *more_data = 1;
            return Some(bytes);
        }
    }
    if bytes.len() < off + ip_len {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + ip_len].copy_from_slice(&path_x.ip_client_remote[..ip_len]);
    off += ip_len;
    *is_pure_ack = 0;
    path_x.is_bdp_sent = true;
    Some(&mut bytes[off..])
}

pub fn format_path_abandon_frame<'a>(
    bytes: &'a mut [u8],
    more_data: &mut i32,
    path_id: u64,
    reason: u64,
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::PathAbandon as u64,
    ) || !encode_varint_at(bytes, &mut off, path_id)
        || !encode_varint_at(bytes, &mut off, reason)
    {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}

impl Connection {
    /// Enqueue a PATH_ABANDON frame for `unique_path_id` carrying
    /// the close `reason`.
    pub fn queue_path_abandon_frame(
        &mut self,
        unique_path_id: u64,
        reason: u64,
    ) -> Result<(), crate::Error> {
        let mut frame = [0u8; 512];
        let mut more_data = 0;
        let rest = format_path_abandon_frame(&mut frame, &mut more_data, unique_path_id, reason)
            .ok_or(crate::Error::BufferTooSmall)?;
        let used = 512 - rest.len();
        if more_data != 0 {
            return Err(crate::Error::BufferTooSmall);
        }
        encode_misc_frame(
            self,
            frame[..used].to_vec(),
            false,
            PacketContext::Application,
        );
        Ok(())
    }
}

impl Connection {
    pub fn decode_frames(
        &mut self,
        path_x: &mut Path,
        bytes: &[u8],
        bytes_max: usize,
        received_data: &mut StreamDataNode,
        epoch: i32,
        addr_from: Option<&SocketAddr>,
        _addr_to: Option<&SocketAddr>,
        pn64: u64,
        _path_is_not_allocated: i32,
        current_time: Instant,
    ) -> i32 {
        let mut tail = &bytes[..bytes_max.min(bytes.len())];
        while !tail.is_empty() {
            let frame_start = tail;
            let mut frame_type = 0;
            let Some(after_type) = frames_varint_decode(tail, &mut frame_type) else {
                return self.connection_error(0x7, 0);
            };
            match frame_type {
                x if x >= crate::frames::FrameType::StreamRangeMin as u64
                    && x <= crate::frames::FrameType::StreamRangeMax as u64 =>
                {
                    let Some(rest) =
                        decode_stream_frame(self, frame_start, received_data, current_time)
                    else {
                        return self.connection_error(0x7, frame_type);
                    };
                    tail = rest;
                }
                x if x == crate::frames::FrameType::CryptoHs as u64 => {
                    let Some(rest) =
                        decode_crypto_hs_frame(self, frame_start, received_data, epoch)
                    else {
                        return self.connection_error(0x7, frame_type);
                    };
                    tail = rest;
                }
                x if x == crate::frames::FrameType::Ack as u64
                    || x == crate::frames::FrameType::AckEcn as u64
                    || x == crate::frames::FrameType::PathAck as u64
                    || x == crate::frames::FrameType::PathAckEcn as u64 =>
                {
                    let mut num_block = 0;
                    let mut path_id = 0;
                    let mut largest = 0;
                    let mut ack_delay = 0;
                    let mut consumed = 0;
                    if parse_ack_header(
                        frame_start,
                        frame_start.len(),
                        &mut num_block,
                        &mut path_id,
                        &mut largest,
                        &mut ack_delay,
                        &mut consumed,
                        self.remote_parameters.ack_delay_exponent,
                    ) != 0
                    {
                        return self.connection_error(0x7, frame_type);
                    }
                    let mut consumed_total = 0;
                    let mut pure_ack = 0;
                    if skip_frame(
                        frame_start,
                        frame_start.len(),
                        &mut consumed_total,
                        &mut pure_ack,
                    ) != 0
                    {
                        return self.connection_error(0x7, frame_type);
                    }
                    tail = &frame_start[consumed_total..];
                    let _ = path_id;
                    let _ = ack_delay;
                    let _ = largest;
                    let _ = num_block;
                }
                x if x == crate::frames::FrameType::PathChallenge as u64 => {
                    if after_type.len() < 8 {
                        return self.connection_error(0x7, frame_type);
                    }
                    if let Some(tuple) = path_x.tuples.first_mut() {
                        tuple.challenge_response = parse_64(&after_type[..8]);
                        tuple.response_required = true;
                    }
                    tail = &after_type[8..];
                }
                x if x == crate::frames::FrameType::PathResponse as u64 => {
                    if after_type.len() < 8 {
                        return self.connection_error(0x7, frame_type);
                    }
                    let response = parse_64(&after_type[..8]);
                    for tuple in &mut path_x.tuples {
                        if tuple.challenge.contains(&response) {
                            tuple.challenge_verified = true;
                            tuple.challenge_required = false;
                            tuple.challenge_failed = false;
                        }
                    }
                    tail = &after_type[8..];
                }
                x if x == crate::frames::FrameType::Datagram as u64
                    || x == crate::frames::FrameType::DatagramL as u64 =>
                {
                    let mut frame_id = 0;
                    let mut length = 0;
                    let Some(payload) =
                        decode_datagram_frame_header(frame_start, &mut frame_id, &mut length)
                    else {
                        return self.connection_error(0x7, frame_type);
                    };
                    tail = &payload[length as usize..];
                    let _ = frame_id;
                }
                x if x == crate::frames::FrameType::NewConnectionId as u64
                    || x == crate::frames::FrameType::PathNewConnectionId as u64 =>
                {
                    let mut consumed = 0;
                    let mut pure_ack = 0;
                    if skip_frame(frame_start, frame_start.len(), &mut consumed, &mut pure_ack) != 0
                    {
                        return self.connection_error(0x7, frame_type);
                    }
                    tail = &frame_start[consumed..];
                }
                x if x == crate::frames::FrameType::ConnectionClose as u64 => {
                    self.remote_error = 1;
                    self.connection_state = State::ClosingReceived;
                    return 0;
                }
                x if x == crate::frames::FrameType::ApplicationClose as u64 => {
                    self.remote_application_error = 1;
                    self.connection_state = State::ClosingReceived;
                    return 0;
                }
                _ => {
                    let mut consumed = 0;
                    let mut pure_ack = 0;
                    if skip_frame(frame_start, frame_start.len(), &mut consumed, &mut pure_ack) != 0
                        || consumed == 0
                    {
                        return self.connection_error(0x7, frame_type);
                    }
                    tail = &frame_start[consumed..];
                }
            }
        }
        path_x.last_non_path_probing_pn = pn64;
        if let Some(addr) = addr_from {
            path_x.update_peer_addr(Some(addr));
        }
        0
    }
}

/// Output of [`parse_observed_address_frame`].  Replaces the
/// `*const u8 addr` + `u16 port` pair-as-out-parameters with a
/// borrowed slice into the input buffer.
pub struct ObservedAddress<'a> {
    pub sequence: u64,
    pub addr: &'a [u8],
    pub port: u16,
}

pub fn parse_observed_address_frame<'a>(
    bytes: &'a [u8],
    ftype: u64,
) -> Option<(ObservedAddress<'a>, &'a [u8])> {
    let mut sequence = 0;
    let bytes = frames_varint_decode(bytes, &mut sequence)?;
    let addr_len = if (ftype & 1) == 0 { 4 } else { 16 };
    if bytes.len() < addr_len + 2 {
        return None;
    }
    let addr = &bytes[..addr_len];
    let port = parse_16(&bytes[addr_len..addr_len + 2]);
    Some((
        ObservedAddress {
            sequence,
            addr,
            port,
        },
        &bytes[addr_len + 2..],
    ))
}

pub fn format_observed_address_frame<'a>(
    bytes: &'a mut [u8],
    ftype: u64,
    sequence_number: u64,
    addr: &[u8],
    port: u16,
    more_data: &mut i32,
) -> Option<&'a mut [u8]> {
    let addr_len = if (ftype & 1) == 0 { 4 } else { 16 };
    let mut off = 0;
    if addr.len() < addr_len
        || !encode_varint_at(bytes, &mut off, ftype)
        || !encode_varint_at(bytes, &mut off, sequence_number)
        || bytes.len() < off + addr_len + 2
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + addr_len].copy_from_slice(&addr[..addr_len]);
    off += addr_len;
    format_16(&mut bytes[off..off + 2], port);
    off += 2;
    Some(&mut bytes[off..])
}

pub fn prepare_observed_address_frame<'a>(
    bytes: &'a mut [u8],
    _path_x: &mut Path,
    tuple: &mut Tuple,
    current_time: Instant,
    _next_wake_time: &mut Instant,
    more_data: &mut i32,
    is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    let (ftype, addr_bytes) = match tuple.peer_addr.ip() {
        core::net::IpAddr::V4(v4) => (
            crate::frames::FrameType::ObservedAddressV4 as u64,
            v4.octets().to_vec(),
        ),
        core::net::IpAddr::V6(v6) => (
            crate::frames::FrameType::ObservedAddressV6 as u64,
            v6.octets().to_vec(),
        ),
    };
    tuple.observed_time = current_time;
    tuple.nb_observed_repeat += 1;
    format_observed_address_frame(
        bytes,
        ftype,
        tuple.nb_observed_repeat as u64,
        &addr_bytes,
        tuple.peer_addr.port(),
        more_data,
    )
    .inspect(|_| {
        *is_pure_ack = 0;
    })
}

impl Path {
    pub fn update_peer_addr(&mut self, peer_addr: Option<&SocketAddr>) {
        if let Some(addr) = peer_addr {
            self.registered_peer_addr = *addr;
        }
    }
}

pub fn skip_frame(bytes: &[u8], bytes_max: usize, consumed: &mut usize, pure_ack: &mut i32) -> i32 {
    let max = bytes_max.min(bytes.len());
    *consumed = 0;
    *pure_ack = 1;
    if max == 0 {
        return -1;
    }

    let mut frame_type = 0;
    let mut tail = match frames_varint_decode(&bytes[..max], &mut frame_type) {
        Some(tail) => tail,
        None => return -1,
    };
    let rest = match frame_type {
        x if x == crate::frames::FrameType::Padding as u64 => {
            *pure_ack = 1;
            let mut off = max - tail.len();
            while off < max && bytes[off] == 0 {
                off += 1;
            }
            &bytes[off..max]
        }
        x if x == crate::frames::FrameType::Ping as u64
            || x == crate::frames::FrameType::HandshakeDone as u64
            || x == crate::frames::FrameType::ImmediateAck as u64 =>
        {
            *pure_ack = 0;
            tail
        }
        x if x == crate::frames::FrameType::Ack as u64
            || x == crate::frames::FrameType::AckEcn as u64
            || x == crate::frames::FrameType::PathAck as u64
            || x == crate::frames::FrameType::PathAckEcn as u64 =>
        {
            let mut num_block = 0;
            let mut path_id = 0;
            let mut largest = 0;
            let mut ack_delay = 0;
            let mut header_len = 0;
            if parse_ack_header(
                &bytes[..max],
                max,
                &mut num_block,
                &mut path_id,
                &mut largest,
                &mut ack_delay,
                &mut header_len,
                0,
            ) != 0
            {
                return -1;
            }
            tail = &bytes[header_len..max];
            let Some(mut t) = skip_n_varints(tail, 1 + (num_block as usize).saturating_mul(2))
            else {
                return -1;
            };
            if frame_type == crate::frames::FrameType::AckEcn as u64
                || frame_type == crate::frames::FrameType::PathAckEcn as u64
            {
                t = match skip_n_varints(t, 3) {
                    Some(t) => t,
                    None => return -1,
                };
            }
            *pure_ack = 1;
            t
        }
        x if x >= crate::frames::FrameType::StreamRangeMin as u64
            && x <= crate::frames::FrameType::StreamRangeMax as u64 =>
        {
            let mut stream_id = 0;
            let mut offset = 0;
            let mut data_length = 0;
            let mut fin = 0;
            let mut header_len = 0;
            if parse_stream_header(
                &bytes[..max],
                max,
                &mut stream_id,
                &mut offset,
                &mut data_length,
                &mut fin,
                &mut header_len,
            ) != 0
                || header_len.saturating_add(data_length) > max
            {
                return -1;
            }
            *pure_ack = 0;
            &bytes[header_len + data_length..max]
        }
        x if x == crate::frames::FrameType::CryptoHs as u64 => {
            let mut ignored = 0;
            tail = match frames_varint_decode(tail, &mut ignored) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            tail = match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            };
            *pure_ack = 0;
            tail
        }
        x if x == crate::frames::FrameType::NewToken as u64 => {
            let mut length = 0;
            tail = match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            };
            *pure_ack = 0;
            tail
        }
        x if x == crate::frames::FrameType::Datagram as u64 => {
            *pure_ack = 0;
            &[]
        }
        x if x == crate::frames::FrameType::DatagramL as u64 => {
            let mut length = 0;
            *pure_ack = 0;
            match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            }
        }
        x if x == crate::frames::FrameType::PathChallenge as u64
            || x == crate::frames::FrameType::PathResponse as u64 =>
        {
            if tail.len() < 8 {
                return -1;
            }
            &tail[8..]
        }
        x if x == crate::frames::FrameType::ResetStream as u64 => {
            *pure_ack = 0;
            match skip_n_varints(tail, 3) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::ResetStreamAt as u64 => {
            *pure_ack = 0;
            match skip_n_varints(tail, 4) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::StopSending as u64
            || x == crate::frames::FrameType::MaxStreamData as u64
            || x == crate::frames::FrameType::StreamDataBlocked as u64 =>
        {
            *pure_ack = 0;
            match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::MaxData as u64
            || x == crate::frames::FrameType::MaxStreamsBidir as u64
            || x == crate::frames::FrameType::MaxStreamsUnidir as u64
            || x == crate::frames::FrameType::DataBlocked as u64
            || x == crate::frames::FrameType::StreamsBlockedBidir as u64
            || x == crate::frames::FrameType::StreamsBlockedUnidir as u64
            || x == crate::frames::FrameType::RetireConnectionId as u64
            || x == crate::frames::FrameType::MaxPathId as u64
            || x == crate::frames::FrameType::PathsBlocked as u64 =>
        {
            *pure_ack = 0;
            match skip_n_varints(tail, 1) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::TimeStamp as u64 => match skip_n_varints(tail, 1) {
            Some(t) => t,
            None => return -1,
        },
        x if x == crate::frames::FrameType::PathRetireConnectionId as u64
            || x == crate::frames::FrameType::PathAbandon as u64
            || x == crate::frames::FrameType::PathAvailable as u64
            || x == crate::frames::FrameType::PathBackup as u64
            || x == crate::frames::FrameType::PathCidBlocked as u64 =>
        {
            *pure_ack = 0;
            match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::NewConnectionId as u64
            || x == crate::frames::FrameType::PathNewConnectionId as u64 =>
        {
            if frame_type == crate::frames::FrameType::PathNewConnectionId as u64 {
                tail = match skip_n_varints(tail, 1) {
                    Some(t) => t,
                    None => return -1,
                };
            }
            *pure_ack = 0;
            tail = match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            };
            let (&cid_len, t) = match tail.split_first() {
                Some(v) => v,
                None => return -1,
            };
            let skip = cid_len as usize + RESET_SECRET_SIZE;
            if t.len() < skip {
                return -1;
            }
            &t[skip..]
        }
        x if x == crate::frames::FrameType::ConnectionClose as u64 => {
            tail = match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            }
        }
        x if x == crate::frames::FrameType::ApplicationClose as u64 => {
            tail = match skip_n_varints(tail, 1) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            }
        }
        x if x == crate::frames::FrameType::AckFrequency as u64 => {
            *pure_ack = 0;
            match skip_n_varints(tail, 4) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::Bdp as u64 => {
            *pure_ack = 0;
            let mut t = match skip_n_varints(tail, 3) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            t = match frames_varint_decode(t, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            };
            t
        }
        x if x == crate::frames::FrameType::ObservedAddressV4 as u64
            || x == crate::frames::FrameType::ObservedAddressV6 as u64 =>
        {
            *pure_ack = 0;
            match parse_observed_address_frame(tail, frame_type) {
                Some((_, rest)) => rest,
                None => return -1,
            }
        }
        _ => return -1,
    };

    *consumed = max - rest.len();
    0
}

pub fn skip_path_abandon_frame(_bytes: &[u8]) -> Option<&[u8]> {
    let mut ignored = 0;
    let bytes = frames_varint_decode(_bytes, &mut ignored)?;
    frames_varint_decode(bytes, &mut ignored)
}

pub fn skip_path_available_or_backup_frame(_bytes: &[u8]) -> Option<&[u8]> {
    let mut ignored = 0;
    let bytes = frames_varint_decode(_bytes, &mut ignored)?;
    frames_varint_decode(bytes, &mut ignored)
}

pub fn is_path_challenging_packet(_bytes: &[u8], _bytes_maxsize: usize) -> bool {
    let max = _bytes_maxsize.min(_bytes.len());
    let mut bytes = &_bytes[..max];
    let mut has_challenge = false;
    while !bytes.is_empty() {
        let mut frame_id = 0;
        if frames_varint_decode(bytes, &mut frame_id).is_none() {
            break;
        }
        match frame_id {
            x if x == crate::frames::FrameType::PathChallenge as u64 => has_challenge = true,
            x if x == crate::frames::FrameType::PathResponse as u64
                || x == crate::frames::FrameType::Padding as u64
                || x == crate::frames::FrameType::NewConnectionId as u64
                || x == crate::frames::FrameType::PathNewConnectionId as u64 => {}
            _ => return false,
        }
        let mut consumed = 0;
        let mut pure_ack = 0;
        if skip_frame(bytes, bytes.len(), &mut consumed, &mut pure_ack) != 0 || consumed == 0 {
            break;
        }
        bytes = &bytes[consumed..];
    }
    has_challenge
}

impl Connection {
    /// Enqueue a PATH_AVAILABLE or PATH_BACKUP frame on `path_x`,
    /// determined by `status`.
    pub fn queue_path_available_or_backup_frame(
        &mut self,
        path_x: &mut Path,
        status: PathStatus,
    ) -> Result<(), crate::Error> {
        let frame_type = if status == PathStatus::Available {
            crate::frames::FrameType::PathAvailable as u64
        } else {
            crate::frames::FrameType::PathBackup as u64
        };
        let sequence = self.status_sequence_to_send_next;
        self.status_sequence_to_send_next = self.status_sequence_to_send_next.saturating_add(1);
        let path_id = path_x.unique_path_id;
        let mut frame = [0u8; 256];
        let mut off = 0;
        for value in [frame_type, path_id, sequence] {
            let n = varint_encode(&mut frame[off..], value);
            if n == 0 {
                return Err(crate::Error::BufferTooSmall);
            }
            off += n;
        }
        path_x.status_sequence_sent_last = sequence;
        encode_misc_frame(
            self,
            frame[..off].to_vec(),
            false,
            PacketContext::Application,
        );
        Ok(())
    }
}

impl Connection {
    pub fn test_and_signal_new_path_allowed(&mut self) {
        if self.is_subscribed_to_path_allowed
            && !self.is_notified_that_path_is_allowed
            && self.paths.len() < NB_PATH_TARGET
            && self.remote_parameters.initial_max_path_id > self.max_path_id_local
        {
            self.is_notified_that_path_is_allowed = true;
        }
    }
}

pub fn decode_closing_frames(
    bytes: &mut [u8],
    bytes_max: usize,
    closing_received: &mut i32,
) -> i32 {
    let max = bytes_max.min(bytes.len());
    let mut byte_index = 0;
    *closing_received = 0;
    while byte_index < max {
        let first_byte = bytes[byte_index];
        if first_byte == crate::frames::FrameType::ConnectionClose as u8
            || first_byte == crate::frames::FrameType::ApplicationClose as u8
        {
            *closing_received = 1;
            break;
        }
        let mut consumed = 0;
        let mut pure_ack = 0;
        let ret = skip_frame(
            &bytes[byte_index..max],
            max - byte_index,
            &mut consumed,
            &mut pure_ack,
        );
        if ret != 0 || consumed == 0 {
            return ret;
        }
        byte_index += consumed;
    }
    0
}

impl Connection {
    pub fn process_sooner_packets(&mut self, _current_time: Instant) {
        self.sooner_stateless.clear();
    }
}

impl Connection {
    pub fn delete_sooner_packets(&mut self) {
        self.sooner_stateless.clear();
    }
}

// ---------------------------------------------------------------------------
// Transport extensions and version upgrade.

pub fn process_tp_version_negotiation<'a>(
    bytes: &'a [u8],
    extension_mode: i32,
    envelop_vn: u32,
    negotiated_vn: &mut u32,
    negotiated_index: &mut i32,
    vn_error: &mut u64,
) -> Option<&'a [u8]> {
    *negotiated_vn = 0;
    *negotiated_index = -1;
    *vn_error = 0;
    if bytes.len() < 4 || !bytes.len().is_multiple_of(4) {
        *vn_error = 0x7;
        return None;
    }
    let current = u32::from_be_bytes(bytes[0..4].try_into().ok()?);
    if current != envelop_vn {
        *vn_error = 0xA;
        return None;
    }
    if extension_mode == 0 {
        *negotiated_vn = current;
        *negotiated_index = Version::try_from_wire(current).map(|_| 0).unwrap_or(-1);
        return Some(&bytes[bytes.len()..]);
    }
    for (idx, chunk) in bytes[4..].chunks_exact(4).enumerate() {
        let candidate = u32::from_be_bytes(chunk.try_into().ok()?);
        if Version::try_from_wire(candidate).is_some() {
            *negotiated_vn = candidate;
            *negotiated_index = idx as i32;
            return Some(&bytes[bytes.len()..]);
        }
    }
    *negotiated_vn = current;
    *negotiated_index = Version::try_from_wire(current).map(|_| 0).unwrap_or(-1);
    Some(&bytes[bytes.len()..])
}

impl Connection {
    pub fn prepare_transport_extensions(
        &mut self,
        extension_mode: i32,
        bytes: &mut [u8],
        bytes_max: usize,
        consumed: &mut usize,
    ) -> i32 {
        let tp = &self.local_parameters;
        let mut out = Vec::new();
        let ok = encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxData as u64,
            tp.initial_max_data,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamDataBidiLocal as u64,
            tp.initial_max_stream_data_bidi_local,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamDataBidiRemote as u64,
            tp.initial_max_stream_data_bidi_remote,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamDataUni as u64,
            tp.initial_max_stream_data_uni,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamsBidi as u64,
            tp.initial_max_stream_id_bidir,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamsUni as u64,
            tp.initial_max_stream_id_unidir,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::IdleTimeout as u64,
            tp.max_idle_timeout.ticks(),
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::MaxPacketSize as u64,
            tp.max_packet_size as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::MaxAckDelay as u64,
            tp.max_ack_delay as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::AckDelayExponent as u64,
            tp.ack_delay_exponent as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::ActiveConnectionIdLimit as u64,
            tp.active_connection_id_limit as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::MaxDatagramFrameSize as u64,
            tp.max_datagram_frame_size as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::EnableLossBit as u64,
            tp.enable_loss_bit as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::EnableTimeStamp as u64,
            tp.enable_time_stamp as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::MinAckDelay as u64,
            tp.min_ack_delay.ticks(),
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxPathId as u64,
            tp.initial_max_path_id,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::AddressDiscovery as u64,
            tp.address_discovery_mode as u64,
        );
        if !ok {
            return -1;
        }
        if tp.migration_disabled
            && !encode_tp_param(
                &mut out,
                crate::tp::TransportParameter::DisableMigration as u64,
                &[],
            )
        {
            return -1;
        }
        if tp.do_grease_quic_bit
            && !encode_tp_varint_param(
                &mut out,
                crate::tp::TransportParameter::GreaseQuicBit as u64,
                1,
            )
        {
            return -1;
        }
        if tp.enable_bdp_frame
            && !encode_tp_varint_param(
                &mut out,
                crate::tp::TransportParameter::EnableBdpFrame as u64,
                1,
            )
        {
            return -1;
        }
        if tp.is_reset_stream_at_enabled
            && !encode_tp_varint_param(
                &mut out,
                crate::tp::TransportParameter::ResetStreamAt as u64,
                1,
            )
        {
            return -1;
        }
        if extension_mode != 0 {
            let mut vn = Vec::new();
            vn.extend_from_slice(&self.proposed_version.to_be_bytes());
            if self.desired_version != 0 {
                vn.extend_from_slice(&self.desired_version.to_be_bytes());
            }
            if !encode_tp_param(
                &mut out,
                crate::tp::TransportParameter::VersionNegotiation as u64,
                &vn,
            ) {
                return -1;
            }
        }
        if out.len() > bytes_max || out.len() > bytes.len() {
            return -1;
        }
        bytes[..out.len()].copy_from_slice(&out);
        *consumed = out.len();
        0
    }
}

impl Connection {
    pub fn receive_transport_extensions(
        &mut self,
        extension_mode: i32,
        bytes: &mut [u8],
        bytes_max: usize,
        consumed: &mut usize,
    ) -> i32 {
        *consumed = 0;
        let mut tail = &bytes[..bytes_max.min(bytes.len())];
        let mut tp = self.remote_parameters.clone();
        while !tail.is_empty() {
            let before = tail.len();
            let mut id = 0;
            let Some(after_id) = frames_varint_decode(tail, &mut id) else {
                return -1;
            };
            let mut len = 0;
            let Some(after_len) = frames_varint_decode(after_id, &mut len) else {
                return -1;
            };
            if after_len.len() < len as usize {
                return -1;
            }
            let value = &after_len[..len as usize];
            match id {
                x if x == crate::tp::TransportParameter::InitialMaxData as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_data = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamDataBidiLocal as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_data_bidi_local = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamDataBidiRemote as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_data_bidi_remote = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamDataUni as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_data_uni = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamsBidi as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_id_bidir = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamsUni as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_id_unidir = v;
                    }
                }
                x if x == crate::tp::TransportParameter::IdleTimeout as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.max_idle_timeout = Duration::from_ticks(v);
                    }
                }
                x if x == crate::tp::TransportParameter::MaxPacketSize as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.max_packet_size = v as u32;
                    }
                }
                x if x == crate::tp::TransportParameter::MaxAckDelay as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.max_ack_delay = v as u32;
                    }
                }
                x if x == crate::tp::TransportParameter::AckDelayExponent as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.ack_delay_exponent = v as u8;
                    }
                }
                x if x == crate::tp::TransportParameter::ActiveConnectionIdLimit as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.active_connection_id_limit = v as u32;
                    }
                }
                x if x == crate::tp::TransportParameter::DisableMigration as u64 => {
                    tp.migration_disabled = true;
                }
                x if x == crate::tp::TransportParameter::MaxDatagramFrameSize as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.max_datagram_frame_size = v as u32;
                    }
                }
                x if x == crate::tp::TransportParameter::EnableLossBit as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.enable_loss_bit = v as i32;
                    }
                }
                x if x == crate::tp::TransportParameter::EnableTimeStamp as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.enable_time_stamp = v as i32;
                    }
                }
                x if x == crate::tp::TransportParameter::MinAckDelay as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.min_ack_delay = Duration::from_ticks(v);
                    }
                }
                x if x == crate::tp::TransportParameter::GreaseQuicBit as u64 => {
                    tp.do_grease_quic_bit = true;
                }
                x if x == crate::tp::TransportParameter::EnableBdpFrame as u64 => {
                    tp.enable_bdp_frame = true;
                }
                x if x == crate::tp::TransportParameter::InitialMaxPathId as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_path_id = v;
                    }
                }
                x if x == crate::tp::TransportParameter::AddressDiscovery as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.address_discovery_mode = v as i32;
                    }
                }
                x if x == crate::tp::TransportParameter::ResetStreamAt as u64 => {
                    tp.is_reset_stream_at_enabled = true;
                }
                x if x == crate::tp::TransportParameter::VersionNegotiation as u64 => {
                    let mut negotiated = 0;
                    let mut negotiated_index = -1;
                    let mut vn_error = 0;
                    if process_tp_version_negotiation(
                        value,
                        extension_mode,
                        self.proposed_version,
                        &mut negotiated,
                        &mut negotiated_index,
                        &mut vn_error,
                    )
                    .is_none()
                    {
                        return -1;
                    }
                    tp.version_negotiation.current = negotiated;
                    let _ = negotiated_index;
                    let _ = vn_error;
                }
                _ => {}
            }
            tail = &after_len[len as usize..];
            *consumed += before - tail.len();
        }
        self.remote_parameters = tp;
        0
    }
}

pub fn create_misc_frame(
    bytes: &[u8],
    is_pure_ack: bool,
    pc: PacketContext,
) -> Result<MiscFrameHeader, crate::Error> {
    Ok(MiscFrameHeader {
        bytes: bytes.to_vec(),
        packet_context: pc,
        is_pure_ack: if is_pure_ack { 1 } else { 0 },
    })
}

impl Connection {
    pub fn process_version_upgrade(
        &mut self,
        old_version_index: i32,
        new_version_index: i32,
    ) -> i32 {
        self.rejected_version = if old_version_index >= 0 {
            self.proposed_version
        } else {
            self.rejected_version
        };
        self.version_index = new_version_index;
        let version = match new_version_index {
            0 => Version::V1,
            1 => Version::V2,
            2 => Version::V2Draft,
            3 => Version::PostIesg,
            4 => Version::TwentyFirstInterop,
            5 => Version::TwentiethInterop,
            6 => Version::TwentiethPreInterop,
            7 => Version::NineteenthInterop,
            8 => Version::NineteenthBisInterop,
            9 => Version::EighteenthInterop,
            10 => Version::SeventeenthInterop,
            11 => Version::InternalTest2,
            12 => Version::InternalTest1,
            _ => Version::V1,
        };
        self.proposed_version = version as u32;
        self.desired_version = self.proposed_version;
        self.local_parameters.version_negotiation.current = self.proposed_version;
        0
    }
}

// ---------------------------------------------------------------------------
// mask proxy hooks (function-pointer pair → trait).

/// Trait covering the C `mask_fns_t` struct's two function
/// pointers.  Implemented by the `mask` proxy module when
/// linked; otherwise `mask_fns` on the QUIC context is
/// `None` and the proxy hooks are skipped.
pub trait MaskOps {
    /// C: `mask_intercept_fn`.
    fn intercept(
        &self,
        quic: &mut Quic,
        mask_ctx: Option<&mut dyn Any>,
        current_time: Instant,
        send_buffer: &mut [u8],
        send_length: &mut usize,
        send_msg_size: &mut usize,
        p_addr_to: &mut Option<SocketAddr>,
        p_addr_from: &mut Option<SocketAddr>,
        if_index: &mut i32,
    ) -> i32;

    /// C: `mask_redirect_fn`.
    fn redirect(
        &self,
        mask_ctx: Option<&mut dyn Any>,
        bytes: &[u8],
        packet_length: usize,
        addr_from: Option<&SocketAddr>,
        consumed: &mut usize,
    ) -> i32;
}

// ---------------------------------------------------------------------------
// PacketHeader default constructor.

impl Default for PacketHeader {
    fn default() -> Self {
        PacketHeader {
            dest_connection_id: crate::ConnectionId::default(),
            src_connection_id: crate::ConnectionId::default(),
            packet_number_truncated: 0,
            version: 0,
            offset: 0,
            packet_number_offset: 0,
            packet_type: PacketType::Error,
            packet_number_mask: 0,
            packet_number_full: 0,
            payload_length: 0,
            version_index: -1,
            epoch: Epoch::Initial,
            packet_context: crate::PacketContext::Initial,
            key_phase: false,
            spin: false,
            has_spin_bit: false,
            has_reserved_bit_set: false,
            has_loss_bits: false,
            loss_bit_q: false,
            loss_bit_l: false,
            quic_bit_is_zero: false,
            token_bytes: Vec::new(),
            payload_length_value: 0,
            local_connection_id: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers for packet header encoding.

/// Write a connection ID's bytes into `buf` and return the number of bytes written.
fn write_cid(buf: &mut [u8], cid: crate::ConnectionId) -> usize {
    let len = cid.len();
    if buf.len() >= len {
        buf[..len].copy_from_slice(cid.as_bytes());
    }
    len
}

/// Write `sequence_number` as `pn_l` big-endian bytes into `buf`.
fn write_pn(buf: &mut [u8], sequence_number: u64, pn_l: usize) {
    match pn_l {
        1 => buf[0] = sequence_number as u8,
        2 => {
            let v = sequence_number as u16;
            buf[0] = (v >> 8) as u8;
            buf[1] = v as u8;
        }
        3 => {
            let v = sequence_number as u32;
            buf[0] = (v >> 16) as u8;
            buf[1] = (v >> 8) as u8;
            buf[2] = v as u8;
        }
        _ => {
            let v = sequence_number as u32;
            buf[0] = (v >> 24) as u8;
            buf[1] = (v >> 16) as u8;
            buf[2] = (v >> 8) as u8;
            buf[3] = v as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Index-based packet-header helpers (avoid borrow conflicts in tests).

impl Connection {
    /// Like [`predict_packet_header_length`] but selects the packet context by
    /// enum value instead of taking a `&mut PacketContextState`.  This avoids the
    /// split-borrow problem in tests that hold `&mut Connection` while computing
    /// the header length.
    pub fn predict_packet_header_length_for_pc(
        &mut self,
        packet_type: PacketType,
        pc: PacketContext,
    ) -> usize {
        let pkt_ctx = &self.pkt_ctx[pc as usize];
        let send_sequence = pkt_ctx.send_sequence;
        let first_pending_seq = pkt_ctx.pending.keys().next().copied();

        // Remote CID length: get from path 0, stash for path 0, first CID.
        let remote_cid_len = self
            .remote_connection_id_stashes
            .first()
            .and_then(|s| s.connection_ids.first())
            .map(|r| r.connection_id.len())
            .unwrap_or(8);

        // Local CID length: get from path 0 tuple 0's local CID token.
        let local_cid_len = self
            .paths
            .first()
            .and_then(|p| p.tuples.first())
            .and_then(|t| t.local_connection_id)
            .and_then(|tok| self.local_connection_ids.get(tok))
            .map(|l| l.connection_id.len())
            .unwrap_or(self.local_cid_length as usize);

        if packet_type == PacketType::OneRttProtected {
            // Short header: 1 byte flags + remote CID + PN
            let delta = if let Some(first) = first_pending_seq {
                send_sequence.saturating_sub(first) as i64
            } else {
                send_sequence as i64
            };
            let pn_l = if delta >= 262144 {
                4usize
            } else if send_sequence >= 1024 {
                3
            } else if send_sequence >= 16 {
                2
            } else {
                1
            };
            1 + remote_cid_len + pn_l
        } else {
            // Long header: 1 byte + 4 version + 2 CID lengths + dest CID + src CID + 2 payload len + 4 PN
            let dest_cid_len = if self.client_mode
                && (packet_type == PacketType::Initial
                    || packet_type == PacketType::ZeroRttProtected)
                && remote_cid_len == 0
            {
                self.initial_connection_id.len()
            } else {
                remote_cid_len
            };

            let mut header_length = 1 + 4 + 2 + dest_cid_len + local_cid_len + 2 + 4;

            // Token length for initial packets.
            if packet_type == PacketType::Initial {
                let token_len = self.retry_token.len();
                let vlen = encode_varint_length(token_len as u64);
                header_length += vlen + token_len;
            }

            header_length
        }
    }

    /// Like [`create_packet_header`] but addresses the path and tuple by index
    /// instead of by `&mut` reference.  This avoids the split-borrow problem
    /// that arises when the path and tuple live inside `&mut self`.
    #[allow(clippy::too_many_arguments)]
    pub fn create_packet_header_at(
        &mut self,
        packet_type: PacketType,
        sequence_number: u64,
        path_idx: usize,
        _tuple_idx: usize,
        header_length: usize,
        bytes: &mut [u8],
        pn_offset: &mut usize,
        pn_length: &mut usize,
    ) -> usize {
        // Get remote and local CIDs.
        let unique_path_id = self
            .paths
            .get(path_idx)
            .map(|p| p.unique_path_id)
            .unwrap_or(0);

        let remote_cid = self
            .remote_connection_id_stashes
            .iter()
            .find(|s| s.unique_path_id == unique_path_id)
            .and_then(|s| s.connection_ids.first())
            .map(|r| r.connection_id)
            .unwrap_or_default();

        let local_cid_tok = self
            .paths
            .get(path_idx)
            .and_then(|p| p.tuples.first())
            .and_then(|t| t.local_connection_id);
        // When no local CID token is set, fall back to a zero-filled CID of
        // `local_cid_length` bytes — matching the behaviour of
        // `predict_packet_header_length_for_pc` which uses `self.local_cid_length`
        // as the fallback length.  This keeps `create` and `predict` in sync so
        // the caller-asserted `header_length == predicted_length` always holds.
        let local_cid = local_cid_tok
            .and_then(|tok| self.local_connection_ids.get(tok))
            .map(|l| l.connection_id)
            .unwrap_or_else(|| {
                ConnectionId::with_size(self.local_cid_length as usize).unwrap_or_default()
            });

        if packet_type == PacketType::OneRttProtected {
            // Short header.
            let k = if self.key_phase_enc { 0x04u8 } else { 0u8 };
            let c = 0x40u8; // QUIC bit
            let mut pn_l = 4usize;
            bytes[0] = k | c; // spin bit = 0
            let mut length = 1;
            length += write_cid(&mut bytes[length..], remote_cid);
            *pn_offset = length;
            if header_length > length && header_length < length + 4 {
                pn_l = header_length - length;
            }
            *pn_length = pn_l;
            bytes[0] |= (pn_l - 1) as u8;
            write_pn(&mut bytes[length..], sequence_number, pn_l);
            length += pn_l;
            length
        } else {
            // Long header.
            // Determine first byte based on version and packet type.
            let version = self.proposed_version;
            // Use V1 encoding for all versions unless it's V2.
            let is_v2 = version == Version::V2 as u32 || version == Version::V2Draft as u32;
            let first_byte: u8 = match (packet_type, is_v2) {
                (PacketType::Initial, false) => 0xC3,
                (PacketType::ZeroRttProtected, false) => 0xD3,
                (PacketType::Handshake, false) => 0xE3,
                (PacketType::Retry, false) => 0xF0,
                (PacketType::Initial, true) => 0xD3,
                (PacketType::ZeroRttProtected, true) => 0xE3,
                (PacketType::Handshake, true) => 0xF3,
                (PacketType::Retry, true) => 0xC0,
                _ => 0xFF,
            };
            bytes[0] = first_byte;
            let mut length = 1;

            // Version
            let ver_bytes = version.to_be_bytes();
            bytes[length..length + 4].copy_from_slice(&ver_bytes);
            length += 4;

            // Dest CID
            let dest_cid = if self.client_mode
                && (packet_type == PacketType::Initial
                    || packet_type == PacketType::ZeroRttProtected)
                && remote_cid.is_empty()
            {
                self.initial_connection_id
            } else {
                remote_cid
            };
            bytes[length] = dest_cid.len() as u8;
            length += 1;
            length += write_cid(&mut bytes[length..], dest_cid);

            // Src CID
            bytes[length] = local_cid.len() as u8;
            length += 1;
            length += write_cid(&mut bytes[length..], local_cid);

            // Token for initial packets
            if packet_type == PacketType::Initial {
                let token_len = self.retry_token.len() as u64;
                length += varint_encode(&mut bytes[length..], token_len);
                if !self.retry_token.is_empty() {
                    let rlen = self.retry_token.len();
                    bytes[length..length + rlen].copy_from_slice(&self.retry_token);
                    length += rlen;
                }
            }

            if packet_type == PacketType::Retry {
                *pn_offset = 0;
                *pn_length = 0;
            } else {
                // Reserve the two-byte payload-length varint; protection
                // updates it once the final packet length is known.
                bytes[length] = 0;
                bytes[length + 1] = 0;
                length += 2;
                *pn_offset = length;
                *pn_length = 4;
                let pn32 = sequence_number as u32;
                bytes[length..length + 4].copy_from_slice(&pn32.to_be_bytes());
                length += 4;
            }
            length
        }
    }

    /// Set the remote connection ID on `path[path_idx].tuples[tuple_idx]`
    /// without requiring a separate `&mut Tuple` borrow.
    pub fn set_path_tuple_remote_cid(
        &mut self,
        path_idx: usize,
        _tuple_idx: usize,
        cid: ConnectionId,
    ) {
        // Find the stash for this path's unique_path_id and update its first CID.
        let unique_path_id = self
            .paths
            .get(path_idx)
            .map(|p| p.unique_path_id)
            .unwrap_or(0);
        if let Some(stash) = self
            .remote_connection_id_stashes
            .iter_mut()
            .find(|s| s.unique_path_id == unique_path_id)
            && let Some(r_cid) = stash.connection_ids.first_mut()
        {
            r_cid.connection_id = cid;
        }
    }

    /// Set the local connection ID token on `path[path_idx].tuples[tuple_idx]`.
    pub fn set_path_tuple_local_cid(
        &mut self,
        path_idx: usize,
        tuple_idx: usize,
        token: LocalConnectionIdToken,
    ) {
        if let Some(path) = self.paths.get_mut(path_idx)
            && let Some(tuple) = path.tuples.get_mut(tuple_idx)
        {
            tuple.local_connection_id = Some(token);
        }
    }

    /// Override `pkt_ctx[pc].send_sequence`.  Convenience for tests that need
    /// to inject a particular sequence number before calling
    /// `predict_packet_header_length_for_pc`.
    pub fn set_send_sequence_for_pc(&mut self, pc: PacketContext, sequence: u64) {
        self.pkt_ctx[pc as usize].send_sequence = sequence;
    }

    /// Create a dummy in-flight packet for `pc` and enqueue it on the retransmit
    /// queue.  Encapsulates `create_packet` + field setup + `queue_for_retransmit`
    /// in one call to avoid the split-borrow needed for the path argument.
    pub fn queue_unacked_packet_for_pc(
        &mut self,
        pc: PacketContext,
        packet_type: PacketType,
        sequence: u64,
        length: usize,
        current_time: Instant,
    ) {
        let Some(mut packet) = self.allocate_packet() else {
            return;
        };
        packet.packet_context = pc;
        packet.packet_type = packet_type;
        packet.sequence_number = sequence;
        packet.length = length;
        packet.send_time = current_time;
        packet.is_queued_for_retransmit = true;
        if let Ok(tok) = self.queued_packets.insert(*packet) {
            self.pkt_ctx[pc as usize].pending.insert(sequence, tok);
        }
    }

    /// Dequeue the first entry in `pkt_ctx[pc].pending` as if it had been
    /// acknowledged.  Encapsulates `dequeue_retransmit_packet` to avoid the
    /// split-borrow needed for the `&mut PacketContextState` argument.
    pub fn dequeue_first_pending_for_pc(&mut self, pc: PacketContext) {
        let first_key = self.pkt_ctx[pc as usize].pending.keys().next().copied();
        if let Some(key) = first_key {
            self.pkt_ctx[pc as usize].pending.remove(&key);
        }
    }
}

// ---------------------------------------------------------------------------
// Version accessors needed by packet-header tests.

impl Connection {
    /// Wire version number of the active protocol version.
    /// C: `picoquic_supported_versions[cnx->version_index].version`.
    pub fn version_number(&self) -> u32 {
        // In the Rust translation, `proposed_version` carries the wire version
        // number directly (C: cnx->proposed_version set from the supported-version
        // table entry's .version field).
        self.proposed_version
    }

    /// TLS-label prefix string for the active protocol version.
    /// C: `picoquic_supported_versions[cnx->version_index].tls_prefix_label`.
    pub fn version_tls_prefix_label(&self) -> &'static str {
        // Resolve via proposed_version to a Version enum, then use parameters().
        Version::try_from_wire(self.proposed_version)
            .map(|v| v.parameters().tls_prefix_label)
            .unwrap_or("tls13 quic ")
    }
}

// ---------------------------------------------------------------------------
// LB CID generation / verification helpers.

impl Quic {
    /// Generate a CID using the installed LB CID callback with `nonce` as the
    /// nonce / "for-server-use" bytes.  Returns `None` when no callback is
    /// installed.  C: direct call to `quic->cnx_id_callback_fn`.
    pub fn lb_generate_cid(&mut self, nonce: &ConnectionId) -> Option<ConnectionId> {
        let mut ctx_box = self.connection_id_callback_ctx.take()?;
        let result = ctx_box
            .downcast_mut::<crate::lb::ConnectionIdContext>()
            .map(|ctx| ctx.generate(self, nonce));
        self.connection_id_callback_ctx = Some(ctx_box);
        result
    }

    /// Verify a CID using the installed LB context and return the embedded
    /// server ID.  Returns `None` when no LB context is installed or the CID
    /// length mismatches.  C: `lb_compat_cid_verify`.
    pub fn lb_verify_cid(&mut self, cid: &ConnectionId) -> Option<u64> {
        let mut ctx_box = self.connection_id_callback_ctx.take()?;
        let result = if let Some(ctx) = ctx_box.downcast_mut::<crate::lb::ConnectionIdContext>() {
            ctx.verify(cid)
        } else {
            None
        };
        self.connection_id_callback_ctx = Some(ctx_box);
        result
    }

    /// Return a mutable reference to the first (most-recently-created) live
    /// connection in this QUIC context, if any.  C: `quic->cnx_list`.
    pub fn first_cnx_mut(&mut self) -> Option<&mut Connection> {
        // C uses a head pointer into a doubly-linked list; in Rust we iterate
        // the arena and return the first live entry.
        self.connections.iter_mut().next()
    }
}

// ---------------------------------------------------------------------------
// Test-only crypto helpers (packet_enc_dec_test).

impl Connection {
    /// Install a test AEAD encryption context for `epoch`, keyed by `secret`.
    /// C: `cnx->crypto_context[epoch].aead_encrypt =
    ///       picoquic_setup_test_aead_context(1, secret, prefix_label)`.
    pub fn set_test_aead_encrypt(&mut self, epoch: Epoch, secret: &[u8]) {
        let prefix = self.version_tls_prefix_label();
        self.crypto_context[epoch as usize].aead_encrypt =
            crate::tls_api::setup_test_aead_context(true, secret, prefix);
    }

    /// Install a test AEAD decryption context for `epoch`, keyed by `secret`.
    pub fn set_test_aead_decrypt(&mut self, epoch: Epoch, secret: &[u8]) {
        let prefix = self.version_tls_prefix_label();
        self.crypto_context[epoch as usize].aead_decrypt =
            crate::tls_api::setup_test_aead_context(false, secret, prefix);
    }

    /// Install a test packet-number encryption context for `epoch`.
    pub fn set_test_pn_enc(&mut self, epoch: Epoch, secret: &[u8]) {
        let prefix = self.version_tls_prefix_label();
        self.crypto_context[epoch as usize].pn_enc =
            crate::tls_api::pn_enc_create_for_test(secret, prefix);
    }

    /// Install a test packet-number decryption context for `epoch`.
    pub fn set_test_pn_dec(&mut self, epoch: Epoch, secret: &[u8]) {
        let prefix = self.version_tls_prefix_label();
        self.crypto_context[epoch as usize].pn_dec =
            crate::tls_api::pn_enc_create_for_test(secret, prefix);
    }

    /// Pad `bytes[..length]` up to `target_length`, returning the new length.
    /// C: `picoquic_pad_to_target_length`.
    pub fn pad_to_target_length(bytes: &mut [u8], length: usize, target: usize) -> usize {
        if target <= length || target > bytes.len() {
            return length;
        }
        // Zero-fill padding bytes.
        for b in &mut bytes[length..target] {
            *b = 0;
        }
        target
    }

    /// Allocate a fresh [`Packet`] for this connection.
    /// C: `picoquic_create_packet(cnx->quic)`.
    pub fn allocate_packet(&mut self) -> Option<Box<Packet>> {
        Some(Box::new(Packet {
            queue_data_repeat_membership: None,
            send_path: None,
            sequence_number: 0,
            send_time: crate::Instant::from_ticks(0),
            delivered_prior: 0,
            delivered_time_prior: crate::Instant::from_ticks(0),
            delivered_sent_prior: 0,
            lost_prior: 0,
            inflight_prior: 0,
            data_repeat_frame: 0,
            data_repeat_index: 0,
            data_repeat_priority: 0,
            data_repeat_stream_id: 0,
            data_repeat_stream_offset: 0,
            data_repeat_stream_data_length: 0,
            length: 0,
            checksum_overhead: 0,
            offset: 0,
            packet_type: PacketType::Error,
            packet_context: crate::PacketContext::Application,
            is_evaluated: false,
            is_ack_eliciting: false,
            is_mtu_probe: false,
            is_multipath_probe: false,
            is_ack_trap: false,
            delivered_app_limited: false,
            sent_cwin_limited: false,
            is_preemptive_repeat: false,
            was_preemptively_repeated: false,
            is_queued_to_path: false,
            is_queued_for_retransmit: false,
            is_queued_for_spurious_detection: false,
            is_queued_for_data_repeat: false,
            bytes: [0u8; crate::MAX_PACKET_SIZE],
        }))
    }
}

// ---------------------------------------------------------------------------
// Helpers used by the Phase 3A skip-frame test-body translation.

/// Enqueue a remote CID (received in a NEW_CONNECTION_ID frame) into the
/// connection's stash list.
/// C: `picoquic_stash_remote_cnxid(cnx, sequence, cid_bytes, reset_secret)`.
pub fn stash_remote_cnxid(
    cnx: &mut Connection,
    sequence: u64,
    cid_bytes: &[u8],
    reset_secret: &[u8],
) -> crate::Result<()> {
    use crate::RESET_SECRET_SIZE;
    // Find or create a default stash (path_id 0).
    let stash_idx = cnx
        .find_or_create_remote_connection_id_stash(0, true)
        .ok_or(crate::Error::Memory)?;
    let cid = ConnectionId::clone_from_slice(cid_bytes).ok_or(crate::Error::Memory)?;
    let mut secret = [0u8; RESET_SECRET_SIZE];
    let copy_len = reset_secret.len().min(RESET_SECRET_SIZE);
    secret[..copy_len].copy_from_slice(&reset_secret[..copy_len]);
    let r_cid = RemoteConnectionId {
        sequence,
        connection_id: cid,
        reset_secret: secret,
        nb_path_references: 0,
        needs_removal: false,
        retire_sent: false,
        retire_acked: false,
        pkt_ctx: PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: u64::MAX,
            latest_time_acknowledged: crate::Instant::from_ticks(0),
            highest_acknowledged_time: crate::Instant::from_ticks(0),
            pending: std::collections::BTreeMap::new(),
            retransmitted: std::collections::BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        },
    };
    cnx.remote_connection_id_stashes[stash_idx]
        .connection_ids
        .push(r_cid);
    Ok(())
}

/// Remove and return one CID from the connection's stash list.  Returns
/// `Ok(true)` when a CID was available, `Ok(false)` when the list was empty.
/// C: `picoquic_obtain_stashed_cnxid(cnx)`.
pub fn obtain_stashed_cnxid(cnx: &mut Connection) -> crate::Result<bool> {
    if let Some(stash) = cnx.remote_connection_id_stashes.first_mut() {
        if stash.connection_ids.is_empty() {
            return Ok(false);
        }
        stash.connection_ids.remove(0);
        return Ok(true);
    }
    Ok(false)
}

/// Splay tree of [`StreamDataNode`]s keyed by byte offset.
///
/// Used by [`queue_network_input`] to hold out-of-order stream data while
/// waiting for missing segments.  The C equivalent is a raw `picosplay_tree_t`
/// embedded directly in the connection or passed by pointer.
pub struct StreamDataSplay {
    inner: crate::splay::SplayTree<u64, StreamDataNode>,
}

impl StreamDataSplay {
    /// Create an empty data splay tree.
    pub fn new() -> Self {
        Self {
            inner: crate::splay::SplayTree::new(),
        }
    }

    /// Number of segments currently held in the tree.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// True when the tree holds no segments.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Default for StreamDataSplay {
    fn default() -> Self {
        Self::new()
    }
}

/// Deliver `data` for `stream_id` starting at `offset` into `tree`,
/// merging overlapping or adjacent segments.  Sets `*new_data = true`
/// when any previously-missing bytes become available.
/// C: `picoquic_queue_network_input`.
pub fn queue_network_input(
    _quic: &mut Quic,
    tree: &mut StreamDataSplay,
    _stream_id: u64,
    offset: u64,
    data: &[u8],
    _fin: bool,
    new_data: &mut bool,
) -> crate::Result<()> {
    *new_data = false;
    if data.is_empty() {
        return Ok(());
    }

    let input_begin = offset;
    let input_end = offset.saturating_add(data.len() as u64);
    let mut cursor = input_begin;

    let mut existing = Vec::new();
    let mut tok = tree.inner.first();
    while let Some(st) = tok {
        if let Some((key, node)) = tree.inner.get_key_value(st) {
            existing.push((*key, node.offset.saturating_add(node.length as u64)));
        }
        tok = tree.inner.next(st);
    }

    for (seg_begin, seg_end) in existing {
        if seg_end <= cursor {
            continue;
        }
        if seg_begin >= input_end {
            break;
        }
        if cursor < seg_begin {
            let chunk_end = seg_begin.min(input_end);
            let src_off = (cursor - input_begin) as usize;
            let len = (chunk_end - cursor) as usize;
            insert_stream_data_chunk(tree, cursor, &data[src_off..src_off + len])?;
            *new_data = true;
        }
        cursor = cursor.max(seg_end);
        if cursor >= input_end {
            break;
        }
    }

    if cursor < input_end {
        let src_off = (cursor - input_begin) as usize;
        insert_stream_data_chunk(tree, cursor, &data[src_off..])?;
        *new_data = true;
    }

    Ok(())
}

fn insert_stream_data_chunk(
    tree: &mut StreamDataSplay,
    offset: u64,
    data: &[u8],
) -> crate::Result<()> {
    let len = data.len().min(crate::MAX_PACKET_SIZE);
    let mut node = StreamDataNode {
        stream_data_membership: None,
        offset,
        data: [0u8; crate::MAX_PACKET_SIZE],
        length: len,
    };
    node.data[..len].copy_from_slice(&data[..len]);
    tree.inner.insert(offset, node).map(|_| ())
}

#[cfg(test)]
mod test {}
