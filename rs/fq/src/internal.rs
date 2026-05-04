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
//! Phase 1 contract: signatures only — every function body is
//! `todo!()`.  Test module at the end is intentionally empty;
//! Phase 2 fills it.
//!
//! Translation policy notes for this module:
//!
//! * Single-bit `unsigned int x : 1` bitfields collapse to
//!   `bool` per field (the "ordinary integer field with
//!   mask/shift accessors" rule, simplified for the trivial
//!   1-bit case where `bool` is clearer than `u32` + masks).
//! * Intrusive list / splay-tree pointers stay as raw
//!   `*mut`/`*const` because the chains are not owned by their
//!   container — hash and splay already established
//!   this convention.  Phase 3 dereferences inside `unsafe { … }`
//!   with `// SAFETY:` notes.
//! * `void* tls_master_ctx`, `void* aead_*`, `void* pn_enc/dec`,
//!   `FILE* F_log`, `struct st_ptls_buffer_t*` etc. stay as
//!   `*mut c_void` — they reference state owned by external
//!   libraries (tls, OpenSSL) that has no Rust counterpart in
//!   v1.
//! * `struct sockaddr_storage` fields fold into
//!   [`core::net::SocketAddr`].  Fields that the C code zeros out
//!   to mean "address not yet set" become `Option<SocketAddr>`.
//! * `Quic`, `Connection`, `Path` were
//!   forward-declared as opaque stubs in
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
use std::path::PathBuf;

use core::any::Any;
use core::ffi::c_void;
use core::net::SocketAddr;

use crate::arena::{Arena, Token};
use crate::crypto_provider_api::VerifyCertificate;
use crate::hash::{HashTable, HashToken};
use crate::splay::{SplayToken, SplayTree};

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
pub type LocalCnxidToken = Token<LocalCnxid>;
pub type PathToken = Token<Path>;
use crate::logger::Logger;
use crate::{
    AlpnSelect, CongestionAlgorithm, ConnectionId, ConnectionIdCb, Fuzz, LossbitVersion,
    PacketContext, PathStatus, PmtudPolicy, RESET_SECRET_SIZE, SpinbitVersion, State, StreamDataCb,
    StreamDirectReceive, TransportParameters,
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
pub const STORED_IP_MAX: usize = 16;
pub const INITIAL_FLOW_CONTROL_MAX: u64 = 0x100000;

pub const INITIAL_RTT: u64 = 250_000;
pub const TARGET_RENO_RTT: u64 = 100_000;
pub const TARGET_SATELLITE_RTT: u64 = 610_000;
pub const INITIAL_RETRANSMIT_TIMER: u64 = 250_000;
pub const INITIAL_MAX_RETRANSMIT_TIMER: u64 = 1_000_000;
pub const LARGE_RETRANSMIT_TIMER: u64 = 2_000_000;
pub const MIN_RETRANSMIT_TIMER: u64 = 50_000;
pub const ACK_DELAY_MAX: u64 = 10_000;
pub const ACK_DELAY_MAX_DEFAULT: u64 = 25_000;
pub const ACK_DELAY_MIN: u64 = 1_000;
pub const ACK_DELAY_MIN_MAX_VALUE: u64 = 0xFFFFFF;
pub const RACK_DELAY: u64 = 10_000;
pub const MAX_ACK_DELAY_MAX_MS: u64 = 0x4000;
pub const TOKEN_DELAY_LONG: u64 = 24 * 60 * 60 * 1_000_000;
pub const TOKEN_DELAY_SHORT: u64 = 2 * 60 * 1_000_000;
pub const CID_REFRESH_DELAY: u64 = 5 * 1_000_000;
pub const MTU_LOSS_THRESHOLD: u64 = 10;

pub const BANDWIDTH_ESTIMATE_MAX: u64 = 10_000_000_000;
pub const BANDWIDTH_TIME_INTERVAL_MIN: u64 = 1000;
pub const BANDWIDTH_MEDIUM: u64 = 2_000_000;
pub const MAX_BANDWIDTH_TIME_INTERVAL_MIN: u64 = 1000;
pub const MAX_BANDWIDTH_TIME_INTERVAL_MAX: u64 = 15000;

pub const MINRTT_MARGIN: u64 = 128;
pub const MINRTT_THRESHOLD: u64 = 128;

pub const SPURIOUS_RETRANSMIT_DELAY_MAX: u64 = 1_000_000;

pub const MICROSEC_SILENCE_MAX: u64 = 120_000_000;
pub const MICROSEC_HANDSHAKE_MAX: u64 = 30_000_000;
pub const MICROSEC_WAIT_MAX: u64 = 10_000_000;

pub const MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT: u64 = 100_000;

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

pub const NB_TP_0RTT: usize = 10;

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

/// QUIC frame-type tags.
///
/// Wire values exceed `u32` for some extension frame types, hence
/// the `#[repr(u64)]`.  `StreamRangeMin`/`StreamRangeMax` mark the
/// inclusive bounds of the eight-variant STREAM frame block
/// (0x08-0x0f); the bit-flag-encoded variants in between aren't
/// individually named in the C source either.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u64)]
pub enum FrameType {
    Padding = 0,
    Ping = 1,
    Ack = 0x02,
    AckEcn = 0x03,
    ResetStream = 0x04,
    StopSending = 0x05,
    CryptoHs = 0x06,
    NewToken = 0x07,
    StreamRangeMin = 0x08,
    StreamRangeMax = 0x0f,
    MaxData = 0x10,
    MaxStreamData = 0x11,
    MaxStreamsBidir = 0x12,
    MaxStreamsUnidir = 0x13,
    DataBlocked = 0x14,
    StreamDataBlocked = 0x15,
    StreamsBlockedBidir = 0x16,
    StreamsBlockedUnidir = 0x17,
    NewConnectionId = 0x18,
    RetireConnectionId = 0x19,
    PathChallenge = 0x1a,
    PathResponse = 0x1b,
    ConnectionClose = 0x1c,
    ApplicationClose = 0x1d,
    HandshakeDone = 0x1e,
    ImmediateAck = 0x1F,
    ResetStreamAt = 0x24,
    Datagram = 0x30,
    DatagramL = 0x31,
    PathAck = 0x3e,
    PathAckEcn = 0x3f,
    AckFrequency = 0xAF,
    TimeStamp = 757,
    PathAbandon = 0x3e75,
    PathBackup = 0x3e76,
    PathAvailable = 0x3e77,
    PathNewConnectionId = 0x3e78,
    PathRetireConnectionId = 0x3e79,
    MaxPathId = 0x3e7a,
    PathsBlocked = 0x3e7b,
    PathCidBlocked = 0x3e7c,
    Bdp = 0xebd9,
    ObservedAddressV4 = 0x9f81a6,
    ObservedAddressV6 = 0x9f81a7,
}

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
// Supported versions.

pub const SEVENTEENTH_INTEROP_VERSION: u32 = 0xFF00001B;
pub const EIGHTEENTH_INTEROP_VERSION: u32 = 0xFF00001C;
pub const NINETEENTH_INTEROP_VERSION: u32 = 0xFF00001D;
pub const NINETEENTH_BIS_INTEROP_VERSION: u32 = 0xFF00001E;
pub const TWENTIETH_PRE_INTEROP_VERSION: u32 = 0xFF00001F;
pub const TWENTIETH_INTEROP_VERSION: u32 = 0xFF000020;
pub const TWENTYFIRST_INTEROP_VERSION: u32 = 0xFF000021;
pub const POST_IESG_VERSION: u32 = 0xFF000022;
pub const V1_VERSION: u32 = 0x00000001;
pub const V2_VERSION: u32 = 0x6b3343cf;
pub const V2_VERSION_DRAFT: u32 = 0x709a50c4;
pub const INTERNAL_TEST_VERSION_1: u32 = 0x50435130;
pub const INTERNAL_TEST_VERSION_2: u32 = 0x50435131;

pub const INTEROP_VERSION_INDEX: usize = 0;
pub const INTEROP_VERSION_LATEST: u32 = NINETEENTH_INTEROP_VERSION;

/// Per-version cryptographic and label parameters.  C:
/// `VersionParameters`.
///
/// `*aead_key` and `*retry_key` are static byte tables in the
/// C source — Rust models them as borrowed slices.  `upgrade_from`
/// is a `NULL`-terminated list in C; here it is a borrowed
/// slice, with an empty slice for "no upgrade path".
#[derive(Debug)]
pub struct VersionParameters {
    pub version: u32,
    pub version_aead_key: &'static [u8],
    pub version_retry_key: &'static [u8],
    pub tls_prefix_label: &'static str,
    pub tls_traffic_update_label: &'static str,
    pub packet_type_version: u32,
    pub upgrade_from: &'static [u32],
}

// `supported_versions[]` and `nb_supported_versions`
// in C — exposed here as a single accessor returning a borrowed
// slice (length implicit).
pub fn supported_versions() -> &'static [VersionParameters] {
    todo!()
}

pub fn get_version_index(_proposed_version: u32) -> i32 {
    todo!()
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
    pub dest_cnx_id: ConnectionId,
    pub srce_cnx_id: ConnectionId,
    pub pn: u32,
    pub vn: u32,
    pub offset: usize,
    pub pn_offset: usize,
    pub ptype: PacketType,
    pub pnmask: u64,
    pub pn64: u64,
    pub payload_length: usize,
    pub version_index: i32,
    pub epoch: Epoch,
    pub pc: PacketContext,

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
    pub pl_val: usize,
    /// Token into [`Quic`]'s arena for the local CID this packet
    /// targets, if any.  C: `*mut LocalCnxid` back-pointer.
    pub l_cid: Option<LocalCnxidToken>,
}

// ---------------------------------------------------------------------------
// Spin-bit policy vtable.

/// Spin-bit policy: how to update the spin bit on an incoming
/// packet, and what value to emit on outgoing packets.  In C the
/// policy is two function pointers grouped into
/// `SpinbitDef`; the two are always installed
/// together so they share one Rust trait.
pub trait SpinBitPolicy {
    /// C: `spinbit_incoming_fn`.
    fn incoming(&self, connection: &mut Connection, path_x: &mut Path, ph: &PacketHeader);

    /// C: `spinbit_outgoing_fn`.
    fn outgoing(&self, connection: &mut Connection) -> u8;
}

/// One row of the spin-bit policy dispatch table.  C:
/// `SpinbitDef`.
pub struct SpinbitDef {
    pub policy: &'static dyn SpinBitPolicy,
}

/// Replacement for `extern SpinbitDef
/// spin_function_table[]`.  Returns the policy table as
/// a borrowed slice — length is implicit.
pub fn spin_function_table() -> &'static [SpinbitDef] {
    todo!()
}

// ---------------------------------------------------------------------------
// Stateless packet, queued at the QUIC context until sendable.

pub struct StatelessPacket {
    pub addr_to: SocketAddr,
    pub addr_local: SocketAddr,
    pub if_index_local: i32,
    pub received_ecn: u8,
    pub length: usize,
    pub receive_time: u64,
    pub connection_id_log64: u64,
    pub initial_cid: ConnectionId,
    pub ptype: PacketType,
    pub bytes: [u8; MAX_PACKET_SIZE],
}

impl Quic {
    /// Allocate a fresh stateless-packet buffer.  C:
    /// `create_stateless_packet` plus the per-context pool;
    /// `Quic.connections.alloc()`-style — Rust uses normal heap
    /// allocation and lets `Drop` free.
    pub fn create_stateless_packet(&mut self) -> Result<StatelessPacket, crate::Error> {
        todo!()
    }

    /// Hand `sp` to the QUIC context's pending stateless-packet
    /// queue.  C: `queue_stateless_packet`.
    pub fn queue_stateless_packet(&mut self, _sp: StatelessPacket) {
        todo!()
    }

    /// Pop the next pending stateless packet.  Returns `None` when
    /// the queue is empty.  C: `dequeue_stateless_packet`.
    pub fn dequeue_stateless_packet(&mut self) -> Option<StatelessPacket> {
        todo!()
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
    pub send_time: u64,
    pub delivered_prior: u64,
    pub delivered_time_prior: u64,
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
    pub ptype: PacketType,
    pub pc: PacketContext,

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
        todo!()
    }
}

impl Connection {
    /// Pad `bytes[length..]` up to whatever the connection's padding
    /// policy mandates (or up to `max_length`, whichever is smaller).
    /// Returns the new buffered length.
    pub fn pad_to_policy(&mut self, _bytes: &mut [u8], _length: usize, _max_length: u32) -> usize {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Token register (replay protection for new tokens / retry tokens / tickets).

pub struct RegisteredToken {
    /// Membership in `Quic::token_reuse_tree`.  Phase 4 uses this
    /// for O(1) removal when the token expires.
    pub registered_token_membership: Option<SplayToken>,
    pub token_time: u64,
    pub token_hash: u64,
    pub count: i32,
}

// ---------------------------------------------------------------------------
// 0-RTT remembered transport parameters.

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Tp0rttKind {
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

pub struct StoredTicket {
    /// Owned SNI string (C: `sni: *mut c_char` plus `sni_length`).
    pub sni: Option<String>,
    /// Owned ALPN string (C: `alpn: *mut c_char` plus `alpn_length`).
    pub alpn: Option<String>,
    /// Owned server IP bytes (C: `ip_addr: *mut u8` plus `ip_addr_length`).
    pub ip_addr: Vec<u8>,
    /// Owned client IP bytes (C: `ip_addr_client: *mut u8` plus
    /// `ip_addr_client_length`).
    pub ip_addr_client: Vec<u8>,
    pub tp_0rtt: [u64; NB_TP_0RTT],
    /// Owned session ticket (C: `ticket: *mut u8` plus `ticket_length`).
    pub ticket: Vec<u8>,
    pub time_valid_until: u64,
    pub version: u32,
    pub was_used: bool,
}

impl Quic {
    /// Cache a session ticket alongside the transport-parameters that
    /// were in force at the time, indexed by `(sni, alpn, version)`.
    #[allow(clippy::too_many_arguments)]
    pub fn store_ticket(
        &mut self,
        _sni: Option<&str>,
        _sni_length: u16,
        _alpn: Option<&str>,
        _alpn_length: u16,
        _version: u32,
        _ip_addr: &[u8],
        _ip_addr_length: u8,
        _ip_addr_client: &[u8],
        _ip_addr_client_length: u8,
        _ticket: &[u8],
        _ticket_length: u16,
        _tp: &TransportParameters,
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// Retrieve a cached ticket record (without consuming it).
    /// `need_unused` skips already-used tickets.
    pub fn get_stored_ticket(
        &mut self,
        _sni: Option<&str>,
        _alpn: Option<&str>,
        _version: u32,
        _need_unused: bool,
        _ticket_id: u64,
    ) -> Option<&mut StoredTicket> {
        todo!()
    }

    /// Output of [`Self::get_ticket`] / [`Self::get_ticket_and_version`].
    /// Replaces the C trio of out-parameters
    /// (`uint8_t** ticket`, `uint16_t* ticket_length`,
    /// `picoquic_tp_t* tp`).  The byte slice borrows from the
    /// cached ticket entry.
    #[allow(clippy::too_many_arguments)]
    pub fn get_ticket(
        &mut self,
        _sni: Option<&str>,
        _alpn: Option<&str>,
        _version: u32,
        _mark_used: bool,
    ) -> Result<(&[u8], TransportParameters), crate::Error> {
        todo!()
    }

    /// As [`Self::get_ticket`] but also returns the QUIC version
    /// the ticket was issued for (in the first tuple slot).
    pub fn get_ticket_and_version(
        &mut self,
        _sni: Option<&str>,
        _alpn: Option<&str>,
        _version: u32,
        _mark_used: bool,
    ) -> Result<(u32, &[u8], TransportParameters), crate::Error> {
        todo!()
    }

    /// Load cached tickets from `ticket_file_name`.
    pub fn load_tickets(
        &mut self,
        _ticket_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// Persist the cached ticket vector to `ticket_file_name`,
    /// dropping entries that expired before `current_time`.  C:
    /// `save_tickets` (operated on the C linked-list head).
    pub fn save_tickets(
        &self,
        _current_time: u64,
        _ticket_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        todo!()
    }
}

// `free_tickets` is gone -- `Quic.stored_tickets.clear()` is the
// Rust equivalent (Drop frees each entry).

impl Connection {
    /// Stash this connection's RTT and CWIN into the issued-ticket
    /// table so a future resumption can seed bandwidth estimates.
    pub fn seed_ticket(&mut self, _path_x: &mut Path) {
        todo!()
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
    /// Owned server IP bytes (C: `ip_addr: *const u8` plus
    /// `ip_addr_length`).
    pub ip_addr: Vec<u8>,
    pub time_valid_until: u64,
    pub was_used: bool,
}

impl Quic {
    /// Cache a server-issued retry token for `(sni, ip_addr)` so it
    /// can be replayed on a future handshake.
    pub fn store_token(
        &mut self,
        _sni: Option<&str>,
        _ip_addr: &[u8],
        _token: &[u8],
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// Retrieve a cached retry token for `(sni, ip_addr)`,
    /// optionally marking it as used so subsequent calls don't
    /// return it again.  Returned slice borrows from the cached
    /// entry.
    pub fn get_token(
        &mut self,
        _sni: Option<&str>,
        _ip_addr: &[u8],
        _mark_used: bool,
    ) -> Result<&[u8], crate::Error> {
        todo!()
    }

    /// Persist cached tokens to `token_file_name`.
    pub fn save_tokens(
        &mut self,
        _token_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// Load cached tokens from `token_file_name`.
    pub fn load_tokens(
        &mut self,
        _token_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        todo!()
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
    pub creation_time: u64,
    pub rtt: u64,
    pub cwin: u64,
    /// 4 bytes for IPv4, 16 for IPv6.
    pub ip_addr: Vec<u8>,
}

impl Quic {
    /// Server-side: record metadata for a session ticket we just
    /// issued, so we can recognize a resumption attempt later.
    pub fn remember_issued_ticket(
        &mut self,
        _ticket_id: u64,
        _rtt: u64,
        _cwin: u64,
        _ip_addr: &[u8],
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// Look up a previously-remembered issued ticket by id.
    /// Returns `None` when no such ticket is on file.
    pub fn retrieve_issued_ticket(&mut self, _ticket_id: u64) -> Option<&mut IssuedTicket> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Auto-qlog and performance log callbacks (function pointers → traits).

/// C: `autoqlog_fn` — invoked at end of connection to
/// turn the binlog into a qlog file.  Returns 0 on success, an
/// errno-style negative on failure.
pub trait AutoQlog {
    fn run(&mut self, connection: &mut Connection) -> i32;
}

/// C: `performance_log_fn` — emit a per-connection
/// performance log row.  `should_delete` is `true` on connection
/// teardown.
pub trait PerformanceLog {
    fn emit(&mut self, quic: &mut Quic, connection: &mut Connection, should_delete: bool) -> i32;
}

// ---------------------------------------------------------------------------
// Memlog hook (per-connection memory-log callback on `Connection`).

/// C: `void (*memlog_call_back)(Connection*, Path*,
/// void* v_memlog, int op_code, uint64_t current_time)` field on
/// `Connection`.
pub trait MemLogHook {
    fn callback(
        &mut self,
        connection: &mut Connection,
        path: &mut Path,
        op_code: i32,
        current_time: u64,
    );
}

// ---------------------------------------------------------------------------
// QUIC context.

/// Top-level QUIC context.  C: `Quic`.  Single-threaded
/// scope (per the translation plan): no `Send`/`Sync`.
pub struct Quic {
    pub tls_master_ctx: *mut c_void,
    pub default_callback_fn: Option<Box<dyn StreamDataCb>>,
    /// Application-supplied state forwarded to the default
    /// stream-data callback.  Opaque to the library (the C side
    /// passed it through as `void*`).  Phase 4 may push the state
    /// into the `StreamDataCb` impl itself, at which point this
    /// field disappears.
    pub default_callback_ctx: Option<Box<dyn Any>>,
    /// State for the DCID-mask callbacks; opaque to the library.
    pub mask_ctx: Option<Box<dyn Any>>,
    pub mask_fns: Option<Box<dyn MaskOps>>,
    pub default_alpn: Option<String>,
    pub alpn_select_fn: Option<Box<dyn AlpnSelect>>,
    pub reset_seed: [u8; RESET_SECRET_SIZE],
    pub retry_seed: [u8; RETRY_SECRET_SIZE],
    pub p_simulated_time: *mut u64,
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
    pub default_handshake_timeout: u64,
    pub crypto_epoch_length_max: u64,
    pub max_simultaneous_logs: u32,
    pub current_number_of_open_logs: u32,
    pub max_half_open_before_retry: u32,
    pub current_number_half_open: u32,
    pub current_number_connections: u32,
    pub tentative_max_number_connections: u32,
    pub max_number_connections: u32,
    pub stateless_reset_next_time: u64,
    pub stateless_reset_min_interval: u64,
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
    /// `LocalCnxidToken` rather than `ConnectionToken` once the
    /// per-path CID arena is wired up; a CID does not uniquely
    /// identify a connection — paths within a connection have
    /// distinct CIDs.  Stub assumes the simpler shape for now.
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

    pub connection_id_callback_fn: Option<Box<dyn ConnectionIdCb>>,
    /// Application-supplied state for the CID callback.  Opaque
    /// to the library.
    pub connection_id_callback_ctx: Option<Box<dyn Any>>,

    pub aead_encrypt_ticket_ctx: *mut c_void,
    pub aead_decrypt_ticket_ctx: *mut c_void,
    pub retry_integrity_sign_ctx: *mut *mut c_void,
    pub retry_integrity_verify_ctx: *mut *mut c_void,

    pub verify_certificate_callback: Option<Box<dyn VerifyCertificate>>,

    pub default_tp: TransportParameters,

    pub fuzz_fn: Option<Box<dyn Fuzz>>,
    /// Application state for the fuzz callback.
    pub fuzz_ctx: Option<Box<dyn Any>>,
    pub wake_file: i32,
    pub wake_line: i32,

    pub max_data_limit: u64,

    pub rtt_update_delta: u64,
    pub pacing_rate_update_delta: u64,

    /// Open text-log sink, if a textlog is installed on this
    /// context.  C: `FILE* F_log` plus the `should_close_log` flag
    /// for whether the C side owned the handle; the Rust shape
    /// makes ownership unambiguous (`Some` -> we own and `Drop`
    /// closes; `None` -> no log).
    pub f_log: Option<File>,
    pub binlog_dir: Option<PathBuf>,
    pub qlog_dir: Option<PathBuf>,
    pub autoqlog_fn: Option<Box<dyn AutoQlog>>,
    pub text_log_fns: Option<Box<dyn Logger>>,
    pub bin_log_fns: Option<Box<dyn Logger>>,
    pub qlog_fns: Option<Box<dyn Logger>>,
    pub perflog_fn: Option<Box<dyn PerformanceLog>>,
    /// Application state for the performance-log callback.
    pub v_perflog_ctx: Option<Box<dyn Any>>,
    /// Application state for the thread callbacks.
    pub v_thread_ctx: Option<Box<dyn Any>>,
}

pub fn context_from_epoch(_epoch: i32) -> PacketContext {
    todo!()
}

impl Quic {
    /// Replay-protection check: is this token already in the
    /// registered-token table?  Inserts it if not.
    pub fn registered_token_check_reuse(
        &mut self,
        _token: &[u8],
        _token_length: usize,
        _expiry_time: u64,
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// Drop registered-token entries that expired before
    /// `expiry_time_max`.
    pub fn registered_token_clear(&mut self, _expiry_time_max: u64) {
        todo!()
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
    pub time_created: u64,
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
    pub ack_horizon: u64,
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
    pub last_time_data_sent: u64,
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

/// True if the stream ID belongs to the client side (client opens
/// even-numbered streams).
#[inline]
pub const fn is_client_stream_id(id: u64) -> bool {
    (id & 1) == 0
}

/// True if the stream ID identifies a bidirectional stream
/// (bit 1 cleared per RFC 9000 §2.1).
#[inline]
pub const fn is_bidir_stream_id(id: u64) -> bool {
    (id & 2) == 0
}

/// True if the stream ID was opened locally given the connection's
/// `client_mode` flag (1 ↔ client, 0 ↔ server).
#[inline]
pub const fn is_local_stream_id(id: u64, client_mode: u64) -> bool {
    ((id ^ client_mode) & 1) != 0
}

/// Build a stream ID from its 1-based rank, client/server role, and
/// uni/bidi flag.
#[inline]
pub const fn stream_id_from_rank(rank: u64, client_mode: u64, is_unidir: u64) -> u64 {
    ((rank - 1) << 2) | (is_unidir << 1) | (client_mode ^ 1)
}

/// Recover the 1-based rank from a stream ID.
#[inline]
pub const fn stream_rank_from_id(id: u64) -> u64 {
    (id + 4) >> 2
}

/// Extract the two type bits (bidi/unidir × client/server) from a
/// stream ID.
#[inline]
pub const fn stream_type_from_id(id: u64) -> u64 {
    id & 3
}

/// Next stream ID with the same type bits as `id`.
#[inline]
pub const fn next_stream_id_for_type(id: u64) -> u64 {
    id + 4
}

// ---------------------------------------------------------------------------
// Misc-frame queue.

/// Misc-frame header.  In C the body of the frame is appended to
/// the header in the same allocation; in Rust the frame bytes
/// follow the header out-of-line — Phase 3 will decide whether
/// to bake them in (`Box<[u8]>`) or keep them external.  For
/// Phase 1 the field is omitted; the layout question is part of
/// the body translation and out of scope here.
pub struct MiscFrameHeader {
    /// Encoded frame bytes (C: header + appended payload in the
    /// same allocation; Rust owns the bytes inline).  `length` is
    /// implicit in `bytes.len()`.
    pub bytes: Vec<u8>,
    pub pc: PacketContext,
    pub is_pure_ack: i32,
}

// ---------------------------------------------------------------------------
// Per-epoch packet/ACK contexts.

pub struct PacketContextState {
    pub send_sequence: u64,
    pub next_sequence_hole: u64,
    pub retransmit_sequence: u64,
    pub highest_acknowledged: u64,
    pub latest_time_acknowledged: u64,
    pub highest_acknowledged_time: u64,
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
    pub highest_ack_sent_time: u64,
    pub time_oldest_unack_packet_received: u64,

    pub ack_needed: bool,
    pub ack_after_fin: bool,
    pub out_of_order_received: bool,
    pub is_immediate_ack_required: bool,
}

pub struct AckContext {
    pub sack_list: SackList,
    pub time_stamp_largest_received: u64,
    pub act: [AckContextTrack; 2],
    pub crypto_rotation_sequence: u64,

    pub ecn_ect0_total_local: u64,
    pub ecn_ect1_total_local: u64,
    pub ecn_ce_total_local: u64,
    pub sending_ecn_ack: bool,
}

// ---------------------------------------------------------------------------
// CID state — local and remote.

pub struct LocalCnxid {
    /// Membership in `Quic::connection_by_id`.  Phase 4 uses this for
    /// O(1) removal when a CID is retired.
    pub connection_by_id_membership: Option<HashToken>,
    pub path_id: u64,
    pub sequence: u64,
    pub create_time: u64,
    pub connection_id: ConnectionId,
    pub is_acked: bool,
}

pub struct LocalCnxidList {
    pub unique_path_id: u64,
    pub local_connection_id_sequence_next: u64,
    pub local_connection_id_retire_before: u64,
    pub local_connection_id_oldest_created: u64,
    pub nb_local_connection_id_expired: i32,
    pub is_demoted: bool,
    pub demotion_time: u64,
    /// Local CIDs registered for this path (replaces the C
    /// `local_connection_id_first` head + per-node `next` chain plus the
    /// redundant `nb_local_connection_id` count, which is now `len()`).
    pub cnxids: Vec<LocalCnxidToken>,
}

pub struct RemoteCnxid {
    pub sequence: u64,
    pub connection_id: ConnectionId,
    pub reset_secret: [u8; RESET_SECRET_SIZE],
    pub nb_path_references: i32,
    pub needs_removal: bool,
    pub retire_sent: bool,
    pub retire_acked: bool,
    pub pkt_ctx: PacketContextState,
}

pub struct RemoteCnxidStash {
    pub unique_path_id: u64,
    pub retire_connection_id_before: u64,
    /// Remote CIDs stashed for this path.  Replaces the C
    /// `connection_id_stash_first` head + per-node `next` chain.
    pub cnxids: Vec<RemoteCnxid>,
    pub is_in_use: bool,
}

// ---------------------------------------------------------------------------
// Pacing and tuple/path state.

pub struct Pacing {
    pub rate: u64,
    pub evaluation_time: u64,
    pub bucket_max: i64,
    pub packet_time_microsec: u64,
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
    pub local_connection_id: Option<LocalCnxidToken>,
    pub nb_observed_repeat: i32,
    pub observed_time: u64,
    pub challenge_response: u64,
    pub challenge: [u64; CHALLENGE_REPEAT_MAX],
    pub challenge_time: u64,
    pub demotion_time: u64,
    pub challenge_time_first: u64,
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
    pub demotion_time: u64,
    pub last_sent_time: u64,
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

    pub last_packet_received_at: u64,
    pub last_loss_event_detected: u64,
    pub nb_retransmit: u64,
    pub total_bytes_lost: u64,
    pub nb_losses_found: u64,
    pub nb_timer_losses: u64,
    pub nb_spurious: u64,

    pub nb_losses_reported: u64,
    pub q_square: u64,

    pub max_ack_delay: u64,
    pub rtt_sample: u64,
    pub one_way_delay_sample: u64,
    pub smoothed_rtt: u64,
    pub rtt_variant: u64,
    pub retransmit_timer: u64,
    pub rtt_min: u64,
    pub rtt_max: u64,
    pub max_spurious_rtt: u64,
    pub max_reorder_delay: u64,
    pub max_reorder_gap: u64,
    pub latest_sent_time: u64,
    pub rtt_packet_previous_period: u64,
    pub rtt_time_previous_period: u64,
    pub nb_rtt_estimate_in_period: u64,
    pub sum_rtt_estimate_in_period: u64,
    pub max_rtt_estimate_in_period: u64,
    pub min_rtt_estimate_in_period: u64,

    pub send_mtu: usize,
    pub send_mtu_max_tried: usize,

    pub delivered: u64,
    pub delivered_last: u64,
    pub delivered_time_last: u64,
    pub delivered_sent_last: u64,
    pub delivered_limited_index: u64,
    pub delivered_last_packet: u64,
    pub bandwidth_estimate: u64,
    pub bandwidth_estimate_max: u64,
    pub max_sample_acked_time: u64,
    pub max_sample_sent_time: u64,
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
    pub last_sender_limited_time: u64,
    pub last_cwin_blocked_time: u64,
    pub last_time_acked_data_frame_sent: u64,
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

    pub rtt_update_delta: u64,
    pub pacing_rate_update_delta: u64,
    pub rtt_threshold_low: u64,
    pub rtt_threshold_high: u64,
    pub pacing_rate_threshold_low: u64,
    pub pacing_rate_threshold_high: u64,
    pub receive_rate_threshold_low: u64,
    pub receive_rate_threshold_high: u64,

    pub rtt_min_remote: u64,
    pub cwin_remote: u64,
    pub ip_client_remote: [u8; 16],
    pub ip_client_remote_length: u8,
}

// ---------------------------------------------------------------------------
// Crypto context (per-epoch, four total).

pub struct CryptoContext {
    pub aead_encrypt: *mut c_void,
    pub aead_decrypt: *mut c_void,
    pub pn_enc: *mut c_void,
    pub pn_dec: *mut c_void,
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
    pub idle_timeout: u64,
    pub local_parameters: TransportParameters,
    pub remote_parameters: TransportParameters,
    pub padding_multiple: u32,
    pub padding_minsize: u32,
    pub seed_ip_addr: [u8; STORED_IP_MAX],
    pub seed_ip_addr_length: u8,
    pub seed_rtt_min: u64,
    pub seed_cwin: u64,

    pub issued_ticket_id: u64,
    pub resumed_ticket_id: u64,

    pub sni: Option<String>,
    pub alpn: Option<String>,
    pub max_early_data_size: usize,

    pub callback_fn: Option<Box<dyn StreamDataCb>>,
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

    pub start_time: u64,
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

    pub next_wake_time: u64,
    /// Membership in `Quic::connection_wake_tree`.  Phase 4 uses this for
    /// O(1) reschedule (remove + reinsert at the new key).
    pub connection_wake_membership: Option<SplayToken>,
    pub app_wake_time: u64,

    pub tls_ctx: *mut c_void,
    pub crypto_epoch_length_max: u64,
    pub crypto_epoch_sequence: u64,
    pub crypto_rotation_time_guard: u64,
    pub tls_sendbuf: *mut c_void,
    pub psk_cipher_suite_id: u16,

    pub tls_stream: [StreamHead; NUMBER_OF_EPOCHS],
    pub crypto_context: [CryptoContext; NUMBER_OF_EPOCHS],
    pub crypto_context_old: CryptoContext,
    pub crypto_context_new: CryptoContext,
    pub crypto_failure_count: u64,

    pub latest_progress_time: u64,
    pub latest_receive_time: u64,
    pub last_close_sent: u64,
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
    pub nb_retransmission_total: u64,
    pub nb_preemptive_repeat: u64,
    pub nb_spurious: u64,
    pub nb_crypto_key_rotations: u64,
    pub nb_packet_holes_inserted: u64,
    pub max_ack_delay_remote: u64,
    pub max_ack_gap_remote: u64,
    pub max_ack_delay_local: u64,
    pub max_ack_gap_local: u64,
    pub min_ack_delay_remote: u64,
    pub min_ack_delay_local: u64,
    pub cwin_blocked: bool,
    pub flow_blocked: bool,
    pub stream_blocked: bool,

    pub congestion_alg: Option<&'static CongestionAlgorithm>,
    pub congestion_alg_option_string: Option<String>,

    pub rtt_update_delta: u64,
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

    pub keep_alive_interval: u64,

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
    pub remote_connection_id_stashes: Vec<RemoteCnxidStash>,

    pub next_path_id_in_lists: u64,
    pub max_path_id_in_connection_id_lists: u64,
    /// Per-path local-CID lists.  Replaces the C
    /// `first_local_connection_id_list` head + per-list `next_list` chain
    /// plus the redundant `nb_local_connection_id_lists` count
    /// (now `len()`).
    pub local_connection_id_lists: Vec<LocalCnxidList>,

    pub ack_frequency_sequence_local: u64,
    pub ack_gap_local: u64,
    pub ack_frequency_delay_local: u64,
    pub ack_frequency_sequence_remote: u64,
    pub ack_gap_remote: u64,
    pub ack_delay_remote: u64,
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
    pub largest_sent_time: u64,
    pub delivered_prior: u64,
    pub delivered_time_prior: u64,
    pub delivered_sent_prior: u64,
    pub lost_prior: u64,
    pub inflight_prior: u64,
    pub rs_is_path_limited: bool,
    pub rs_is_cwnd_limited: bool,
    pub is_set: bool,
    pub data_acked: u64,
}

pub struct PacketData {
    pub last_time_stamp_received: u64,
    pub last_ack_delay: u64,
    pub nb_path_ack: i32,
    pub path_ack: [PacketDataPathAck; NB_PATH_TARGET],
}

// ---------------------------------------------------------------------------
// Connection lifecycle / registration.

pub fn create_cnx_internal(
    _quic: &mut Quic,
    _initial_cnx_id: ConnectionId,
    _remote_cnx_id: ConnectionId,
    _addr_to: Option<&SocketAddr>,
    _start_time: u64,
    _preferred_version: u32,
    _sni: Option<&str>,
    _alpn: Option<&str>,
    _client_mode: bool,
    // Phase 2 plan: replace these *mut c_void crypto-handle
    // parameters with `Box<dyn AeadCipher>` / `Box<dyn PnEncrypt>`
    // once the crypto provider abstraction lands.
    _initial_aead_dec: *mut c_void,
    _initial_pn_dec: *mut c_void,
) -> Result<ConnectionToken, crate::Error> {
    todo!()
}

impl Quic {
    /// Read tokens from `token_file_name` into the cache, replacing
    /// any previous contents.
    pub fn load_token_file(
        &mut self,
        _token_file_name: &(impl AsRef<std::path::Path> + ?Sized),
    ) -> Result<(), crate::Error> {
        todo!()
    }
}

pub fn init_transport_parameters(_tp: &mut TransportParameters) {
    todo!()
}

/// Insert `l_cid` into the QUIC context's CID lookup table so that
/// future packets carrying it route to `connection`.
pub fn register_cnx_id(
    _quic: &mut Quic,
    _connection: &mut Connection,
    _l_cid: &mut LocalCnxid,
) -> Result<(), crate::Error> {
    todo!()
}

impl Connection {
    /// Insert this connection's reset-secret hash entry into the
    /// QUIC context's lookup table.
    pub fn register_net_secret(&mut self) -> Result<(), crate::Error> {
        todo!()
    }

    /// Insert this connection's initial-CID hash entry into the
    /// QUIC context's lookup table.
    pub fn register_net_icid(&mut self) -> Result<(), crate::Error> {
        todo!()
    }
}

pub fn create_local_cnx_id(
    _quic: &mut Quic,
    _cnx_id: &mut ConnectionId,
    _cnx_id_remote: ConnectionId,
) {
    todo!()
}

// ---------------------------------------------------------------------------
// Tuple/path management.

/// Add a tuple to `path_x.tuples` and return its index there.
pub fn create_tuple(
    _path_x: &mut Path,
    _local_addr: Option<&SocketAddr>,
    _peer_addr: Option<&SocketAddr>,
    _if_index: i32,
) -> Result<usize, crate::Error> {
    todo!()
}

pub fn delete_demoted_tuples(
    _connection: &mut Connection,
    _current_time: u64,
    _next_wake_time: &mut u64,
) {
    todo!()
}

/// Remove the tuple at `path_x.tuples[index]`.  C: `delete_tuple`.
pub fn delete_tuple(_path_x: &mut Path, _index: usize, _is_deleting_path: bool) {
    todo!()
}

/// Move the tuple at `index` to the head of `path_x.tuples`.  C:
/// `set_first_tuple`.
pub fn set_first_tuple(_path_x: &mut Path, _index: usize) {
    todo!()
}

pub fn create_path(
    _connection: &mut Connection,
    _start_time: u64,
    _local_addr: Option<&SocketAddr>,
    _peer_addr: Option<&SocketAddr>,
    _if_index: i32,
    _unique_path_id: u64,
) -> i32 {
    todo!()
}

pub fn register_path(_connection: &mut Connection, _path_x: &mut Path) {
    todo!()
}

pub fn find_incoming_path(
    _connection: &mut Connection,
    _ph: &mut PacketHeader,
    _addr_from: &mut SocketAddr,
    _addr_to: &mut SocketAddr,
    _if_index_to: i32,
    _current_time: u64,
    _p_path_id: &mut i32,
) -> i32 {
    todo!()
}

pub fn prepare_path_control_packet(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _tuple: &mut Tuple,
    _packet: &mut Packet,
    _current_time: u64,
    _send_buffer: &mut [u8],
    _send_buffer_max: usize,
    _send_length: &mut usize,
    _next_wake_time: &mut u64,
) -> i32 {
    todo!()
}

pub fn prepare_path_challenge_frames<'a>(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _is_challenge_padding_needed: &mut i32,
    _current_time: u64,
    _next_wake_time: &mut u64,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn select_next_path_tuple(
    _connection: &mut Connection,
    _current_time: u64,
    _next_wake_time: &mut u64,
) -> Option<(PathToken, usize)> {
    todo!()
}

impl Connection {
    /// Allocate a fresh remote CID for path `path_id` and migrate the
    /// path's tuples onto it.
    pub fn renew_connection_id(&mut self, _path_id: i32) -> Result<(), crate::Error> {
        todo!()
    }

    /// Tear down the path at `path_index` immediately.
    pub fn delete_path(&mut self, _path_index: i32) {
        todo!()
    }

    /// Mark path at `path_index` for demotion at `current_time`,
    /// recording the close reason for logging/closure frames.
    pub fn demote_path(&mut self, _path_index: i32, _current_time: u64, _reason: u64) {
        todo!()
    }

    /// Re-queue all in-flight packets on `path_x` for retransmit
    /// after the path was demoted.
    pub fn retransmit_demoted_path(&mut self, _path_x: &mut Path, _current_time: u64) {
        todo!()
    }

    /// Re-queue retransmissions on `path_x` triggered by an ACK
    /// arriving on a different path.
    pub fn queue_retransmit_on_ack(&mut self, _path_x: &mut Path, _current_time: u64) {
        todo!()
    }

    /// Sweep abandoned paths and free any whose teardown is complete.
    pub fn delete_abandoned_paths(&mut self, _current_time: u64, _next_wake_time: &mut u64) {
        todo!()
    }
}

pub fn set_tuple_challenge(_tuple: &mut Tuple, _current_time: u64, _use_constant_challenges: i32) {
    todo!()
}

impl Connection {
    /// Force a fresh PATH_CHALLENGE on path `path_id`.
    pub fn set_path_challenge(&mut self, _path_id: i32, _current_time: u64) {
        todo!()
    }

    /// Look up a path by `(local_addr, peer_addr)`.  Sets
    /// `partial_match` when only one of the two addresses matched.
    /// Returns the path index, or -1 when no path matches.
    pub fn find_path_by_address(
        &mut self,
        _addr_local: Option<&SocketAddr>,
        _addr_peer: Option<&SocketAddr>,
        _partial_match: &mut i32,
    ) -> i32 {
        todo!()
    }

    /// Look up a path by its unique-path-id; returns -1 when absent.
    pub fn find_path_by_unique_id(&mut self, _unique_path_id: u64) -> i32 {
        todo!()
    }

    /// True when there is a stashed remote CID available to label a
    /// new tuple on path `unique_path_id`.
    pub fn check_cid_for_new_tuple(&mut self, _unique_path_id: u64) -> i32 {
        todo!()
    }

    /// Bind a remote CID to `tuple` so it can address peer packets
    /// on `path_x`.
    pub fn assign_peer_connection_id_to_tuple(
        &mut self,
        _path_x: &mut Path,
        _tuple: &mut Tuple,
    ) -> Result<(), crate::Error> {
        todo!()
    }
}

pub fn reset_path_mtu(_path_x: &mut Path) {
    todo!()
}

pub fn get_path_id_from_unique(_connection: &mut Connection, _unique_path_id: u64) -> i32 {
    todo!()
}

/// Find the remote-CID stash for `unique_path_id`, optionally
/// creating one if it doesn't exist.  Returns the index into
/// `connection.remote_connection_id_stashes`.  C: `find_or_create_remote_connection_id_stash`.
pub fn find_or_create_remote_connection_id_stash(
    _connection: &mut Connection,
    _unique_path_id: u64,
    _do_create: bool,
) -> Option<usize> {
    todo!()
}

// ---------------------------------------------------------------------------
// Remote CID stash management.

impl Connection {
    /// Initialize the per-connection remote-CID stash.
    pub fn init_connection_id_stash(&mut self) -> Result<(), crate::Error> {
        todo!()
    }
}

/// Output of [`add_remote_connection_id_to_stash`] / [`stash_remote_connection_id`]:
/// a status code (matching the C `uint64_t` return) and the index
/// of the newly-stashed CID inside the stash's `cnxids` vector,
/// or `None` if no CID was stashed.
pub struct StashResult {
    pub status: u64,
    pub stashed_index: Option<usize>,
}

pub fn add_remote_connection_id_to_stash(
    _connection: &mut Connection,
    _stash_index: usize,
    _retire_before_next: u64,
    _sequence: u64,
    _connection_id_bytes: &[u8],
    _secret_bytes: &[u8],
) -> StashResult {
    todo!()
}

pub fn stash_remote_connection_id(
    _connection: &mut Connection,
    _retire_before_next: u64,
    _unique_path_id: u64,
    _sequence: u64,
    _connection_id_bytes: &[u8],
    _secret_bytes: &[u8],
) -> StashResult {
    todo!()
}

/// Remove the CID at `removed_index` from
/// `connection.remote_connection_id_stashes[stash_index].cnxids`.  Returns the
/// next index that is still live, if any (matches the C "return
/// the chain successor" pattern).
pub fn remove_connection_id_from_stash(
    _connection: &mut Connection,
    _stash_index: usize,
    _removed_index: usize,
) -> Option<usize> {
    todo!()
}

/// As [`remove_connection_id_from_stash`] but locates the stash by
/// `unique_path_id`.
pub fn remove_stashed_connection_id(
    _connection: &mut Connection,
    _unique_path_id: u64,
    _removed_index: usize,
) -> Option<usize> {
    todo!()
}

/// Return a reference to the first available CID in `stash`, if any.
pub fn get_connection_id_from_stash(_stash: &mut RemoteCnxidStash) -> Option<&mut RemoteCnxid> {
    todo!()
}

/// Reserve a stashed CID for use on `unique_path_id`.  Returns the
/// stash index of the chosen CID, or `None` when none are
/// available.
pub fn obtain_stashed_connection_id(
    _connection: &mut Connection,
    _unique_path_id: u64,
) -> Option<(usize, usize)> {
    todo!()
}

pub fn dereference_stashed_connection_id(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _is_deleting_connection: i32,
) {
    todo!()
}

pub fn dereference_stashed_connection_id_tuple(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _tuple: &mut Tuple,
    _is_deleting_connection: i32,
) {
    todo!()
}

pub fn remove_not_before_from_stash(
    _connection: &mut Connection,
    _connection_id_stash: &mut RemoteCnxidStash,
    _not_before: u64,
    _current_time: u64,
) -> u64 {
    todo!()
}

/// Remove the stash at `connection.remote_connection_id_stashes[stash_index]`.
pub fn delete_remote_connection_id_stash(_connection: &mut Connection, _stash_index: usize) {
    todo!()
}

pub fn remove_not_before_cid(
    _connection: &mut Connection,
    _unique_path_id: u64,
    _not_before: u64,
    _current_time: u64,
) -> u64 {
    todo!()
}

impl Connection {
    /// Force a CID rotation on `path_x` (allocate a new local CID
    /// and retire the previous one).
    pub fn renew_path_connection_id(&mut self, _path_x: &mut Path) -> Result<(), crate::Error> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Retransmission queue management.

pub fn queue_for_retransmit(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _packet: &mut Packet,
    _length: usize,
    _current_time: u64,
) {
    todo!()
}

/// Remove the in-flight packet at `token` from `pkt_ctx.pending`.
/// Returns the next packet token (sequence-wise successor) for the
/// caller to chain onto.  C: `dequeue_retransmit_packet` returning
/// the next pointer.
pub fn dequeue_retransmit_packet(
    _connection: &mut Connection,
    _pkt_ctx: &mut PacketContextState,
    _packet: PacketToken,
    _should_free: bool,
    _add_to_data_repeat_queue: bool,
) -> Option<PacketToken> {
    todo!()
}

pub fn dequeue_retransmitted_packet(
    _connection: &mut Connection,
    _pkt_ctx: &mut PacketContextState,
    _packet: PacketToken,
) {
    todo!()
}

impl Connection {
    /// Tear down all in-flight state and prepare the connection for
    /// a fresh handshake.
    pub fn reset(&mut self, _current_time: u64) -> Result<(), crate::Error> {
        todo!()
    }

    /// Drain the per-epoch packet-number-space context state without
    /// disturbing the rest of the connection.
    pub fn reset_packet_context(&mut self, _pkt_ctx: &mut PacketContextState) {
        todo!()
    }

    /// Mark this connection as having hit a transport error so the
    /// next outgoing packet emits CONNECTION_CLOSE.  The returned
    /// value mirrors the C convention (the error code itself) so
    /// callers can write `return connection.connection_error(…);`.
    pub fn connection_error(&mut self, _local_error: u64, _frame_type: u64) -> i32 {
        todo!()
    }

    /// As [`Self::connection_error`] but with an optional reason
    /// string emitted in the CONNECTION_CLOSE frame.
    pub fn connection_error_ex(
        &mut self,
        _local_error: u64,
        _frame_type: u64,
        _local_reason: Option<&str>,
    ) -> i32 {
        todo!()
    }

    /// Move the connection straight to the disconnected state
    /// without sending CONNECTION_CLOSE.
    pub fn connection_disconnect(&mut self) {
        todo!()
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
        _cnx_id: ConnectionId,
    ) -> Option<(ConnectionToken, LocalCnxidToken)> {
        todo!()
    }

    /// Look up a connection by peer address.  C: `connection_by_net`.
    pub fn connection_by_net(&mut self, _addr: Option<&SocketAddr>) -> Option<ConnectionToken> {
        todo!()
    }

    /// Look up a connection by initial CID and peer address.  C:
    /// `connection_by_icid`.
    pub fn connection_by_icid(
        &mut self,
        _icid: &ConnectionId,
        _addr: Option<&SocketAddr>,
    ) -> Option<ConnectionToken> {
        todo!()
    }

    /// Look up a connection by stateless-reset secret and peer
    /// address.  C: `connection_by_secret`.
    pub fn connection_by_secret(
        &mut self,
        _reset_secret: &[u8],
        _addr: Option<&SocketAddr>,
    ) -> Option<ConnectionToken> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// Pacing.

impl Pacing {
    /// Initialize the pacing state at `current_time`.
    pub fn init(&mut self, _current_time: u64) {
        todo!()
    }

    /// True when the pacer is currently throttling sends.
    pub fn is_blocked(&mut self) -> bool {
        todo!()
    }

    /// Decide whether sending right now is authorized by the pacer.
    /// Writes the next authorized time into `next_time` when blocked.
    pub fn is_authorized(
        &mut self,
        _current_time: u64,
        _next_time: &mut u64,
        _packet_train_mode: bool,
        _quic: &mut Quic,
    ) -> bool {
        todo!()
    }

    /// Re-derive bucket and rate after a configuration change.
    pub fn update_parameters(
        &mut self,
        _pacing_rate: f64,
        _quantum: u64,
        _send_mtu: usize,
        _smoothed_rtt: u64,
        _signalled_path: Option<PathToken>,
    ) {
        todo!()
    }

    /// Recompute pacing parameters from the congestion window.
    pub fn update_window(
        &mut self,
        _slow_start: i32,
        _cwin: u64,
        _send_mtu: usize,
        _smoothed_rtt: u64,
        _signalled_path: Option<PathToken>,
    ) {
        todo!()
    }

    /// Update pacer state after a packet of `length` bytes was sent.
    pub fn update_after_send(&mut self, _length: usize, _send_mtu: usize, _current_time: u64) {
        todo!()
    }
}

pub fn update_pacing_data(_path_x: &mut Path, _slow_start: i32) {
    todo!()
}

pub fn update_pacing_after_send(_path_x: &mut Path, _length: usize, _current_time: u64) {
    todo!()
}

pub fn is_sending_authorized_by_pacing(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _current_time: u64,
    _next_time: &mut u64,
) -> bool {
    todo!()
}

pub fn update_pacing_rate(_path_x: &mut Path, _pacing_rate: f64, _quantum: u64) {
    todo!()
}

pub fn refresh_path_quality_thresholds(_path_x: &mut Path) {
    todo!()
}

pub fn issue_path_quality_update(_connection: &mut Connection, _path_x: &mut Path) -> i32 {
    todo!()
}

pub fn reinsert_by_wake_time(_quic: &mut Quic, _connection: &mut Connection, _next_time: u64) {
    todo!()
}

// ---------------------------------------------------------------------------
// Integer parsing / formatting helpers (translated from `PARSE_*` and
// `format_*`).

/// Read a big-endian `u16` from the first two bytes of `b`.
#[inline]
pub const fn parse_16(b: &[u8]) -> u16 {
    ((b[0] as u16) << 8) | (b[1] as u16)
}

/// Read a 24-bit big-endian unsigned integer (zero-extended into a
/// `u32`) from the first three bytes of `b`.
#[inline]
pub const fn parse_24(b: &[u8]) -> u32 {
    (parse_16(b) as u32) << 8 | (b[2] as u32)
}

/// Read a big-endian `u32` from the first four bytes of `b`.
#[inline]
pub const fn parse_32(b: &[u8]) -> u32 {
    ((parse_16(b) as u32) << 16) | parse_16(&[b[2], b[3]]) as u32
}

/// Read a big-endian `u64` from the first eight bytes of `b`.
#[inline]
pub const fn parse_64(b: &[u8]) -> u64 {
    ((parse_32(b) as u64) << 32) | parse_32(&[b[4], b[5], b[6], b[7]]) as u64
}

pub fn format_16(_bytes: &mut [u8], _n16: u16) {
    todo!()
}

pub fn format_24(_bytes: &mut [u8], _n24: u32) {
    todo!()
}

pub fn format_32(_bytes: &mut [u8], _n32: u32) {
    todo!()
}

pub fn format_64(_bytes: &mut [u8], _n64: u64) {
    todo!()
}

pub fn varint_encode(_bytes: &mut [u8], _n64: u64) -> usize {
    todo!()
}

pub fn varint_encode_16(_bytes: &mut [u8], _n16: u16) {
    todo!()
}

pub fn varint_decode(_bytes: &[u8], _n64: &mut u64) -> usize {
    todo!()
}

/// Decode a QUIC varint at the start of `bytes`, return the
/// remaining tail (or `None` on under-read).  C: returned a
/// pointer past the consumed bytes; the Rust shape returns the
/// remaining slice instead.
pub fn frames_varint_decode<'a>(_bytes: &'a [u8], _n64: &mut u64) -> Option<&'a [u8]> {
    todo!()
}

/// Skip past a varint, returning the remaining tail.  C:
/// `frames_varint_skip` returning a pointer.
pub fn frames_varint_skip(_bytes: &[u8]) -> Option<&[u8]> {
    todo!()
}

pub fn varint_skip(_bytes: &[u8]) -> usize {
    todo!()
}

pub fn encode_varint_length(_n64: u64) -> usize {
    todo!()
}

pub fn decode_varint_length(_byte: u8) -> usize {
    todo!()
}

// ---------------------------------------------------------------------------
// Packet parsing / header creation.

pub fn parse_long_packet_type(_flags: u8, _version_index: i32) -> PacketType {
    todo!()
}

/// Parse a packet header.  C `int picoquic_parse_packet_header(...,
/// picoquic_cnx_t** pcnx, int receiving)`: the C `pcnx`
/// out-parameter folds into the `Ok` payload here, the C `int`
/// status into `Result`.
pub fn parse_packet_header(
    _quic: &mut Quic,
    _bytes: &[u8],
    _addr_from: Option<&SocketAddr>,
    _ph: &mut PacketHeader,
    _receiving: bool,
) -> Result<Option<ConnectionToken>, crate::Error> {
    todo!()
}

#[allow(clippy::too_many_arguments)]
pub fn create_long_header(
    _packet_type: PacketType,
    _dest_cnx_id: &ConnectionId,
    _srce_cnx_id: &ConnectionId,
    _do_grease_quic_bit: bool,
    _version: u32,
    _version_index: i32,
    _sequence_number: u64,
    _retry_token: &[u8],
    _bytes: &mut [u8],
    _pn_offset: &mut usize,
    _pn_length: &mut usize,
) -> usize {
    todo!()
}

pub fn create_packet_header(
    _connection: &mut Connection,
    _packet_type: PacketType,
    _sequence_number: u64,
    _path_x: &mut Path,
    _tuple: &mut Tuple,
    _header_length: usize,
    _bytes: &mut [u8],
    _pn_offset: &mut usize,
    _pn_length: &mut usize,
) -> usize {
    todo!()
}

pub fn predict_packet_header_length(
    _connection: &mut Connection,
    _packet_type: PacketType,
    _pkt_ctx: &mut PacketContextState,
) -> usize {
    todo!()
}

pub fn update_payload_length(
    _bytes: &mut [u8],
    _pnum_index: usize,
    _header_length: usize,
    _packet_length: usize,
) {
    todo!()
}

pub fn get_checksum_length(_connection: &mut Connection, _is_cleartext_mode: Epoch) -> usize {
    todo!()
}

pub fn protect_packet_header(
    _send_buffer: &mut [u8],
    _pn_offset: usize,
    _first_mask: u8,
    // Phase 2: replace with `&mut dyn PnEncrypt`.
    _pn_enc: *mut c_void,
) {
    todo!()
}

pub fn protect_packet(
    _connection: &mut Connection,
    _ptype: PacketType,
    _bytes: &mut [u8],
    _sequence_number: u64,
    _length: usize,
    _header_length: usize,
    _send_buffer: &mut [u8],
    _send_buffer_max: usize,
    // Phase 2: replace with `&mut dyn AeadCipher` + `&mut dyn PnEncrypt`.
    _aead_context: *mut c_void,
    _pn_enc: *mut c_void,
    _path_x: &mut Path,
    _tuple: &mut Tuple,
    _current_time: u64,
) -> usize {
    todo!()
}

pub fn get_packet_number64(_highest: u64, _mask: u64, _pn: u32) -> u64 {
    todo!()
}

pub fn remove_header_protection_inner(
    _bytes: &mut [u8],
    _length: usize,
    _decrypted_bytes: &mut [u8],
    _ph: &mut PacketHeader,
    // Phase 2: replace with `&mut dyn PnEncrypt`.
    _pn_enc: *mut c_void,
    _is_loss_bit_enabled_incoming: bool,
    _sack_list_last: u64,
) -> i32 {
    todo!()
}

pub fn pad_to_target_length(_bytes: &mut [u8], _length: usize, _target: usize) -> usize {
    todo!()
}

pub fn finalize_and_protect_packet_tuple(
    _connection: &mut Connection,
    _packet: &mut Packet,
    _ret: i32,
    _length: usize,
    _header_length: usize,
    _checksum_overhead: usize,
    _send_length: &mut usize,
    _send_buffer: &mut [u8],
    _send_buffer_max: usize,
    _path_x: &mut Path,
    _current_time: u64,
    _tuple: &mut Tuple,
) {
    todo!()
}

pub fn finalize_and_protect_packet(
    _connection: &mut Connection,
    _packet: &mut Packet,
    _ret: i32,
    _length: usize,
    _header_length: usize,
    _checksum_overhead: usize,
    _send_length: &mut usize,
    _send_buffer: &mut [u8],
    _send_buffer_max: usize,
    _path_x: &mut Path,
    _current_time: u64,
) {
    todo!()
}

pub fn implicit_handshake_ack(
    _connection: &mut Connection,
    _pc: PacketContext,
    _current_time: u64,
) {
    todo!()
}

pub fn false_start_transition(_connection: &mut Connection, _current_time: u64) {
    todo!()
}

pub fn client_almost_ready_transition(_connection: &mut Connection) {
    todo!()
}

pub fn ready_state_transition(_connection: &mut Connection, _current_time: u64) {
    todo!()
}

/// Parse a packet header and decrypt its payload.  C: `pcnx` and
/// `new_context_created` out-parameters fold into the `Ok` payload.
#[allow(clippy::too_many_arguments)]
pub fn parse_header_and_decrypt(
    _quic: &mut Quic,
    _bytes: &[u8],
    _packet_length: usize,
    _addr_from: Option<&SocketAddr>,
    _current_time: u64,
    _decrypted_data: &mut StreamDataNode,
    _ph: &mut PacketHeader,
    _consumed: &mut usize,
) -> Result<(Option<ConnectionToken>, bool), crate::Error> {
    todo!()
}

// ---------------------------------------------------------------------------
// Packet number / ACK shortcuts.

pub fn get_sequence_number(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _pc: PacketContext,
) -> u64 {
    todo!()
}

pub fn get_ack_number(_connection: &mut Connection, _path_x: &mut Path, _pc: PacketContext) -> u64 {
    todo!()
}

pub fn get_last_packet(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _pc: PacketContext,
) -> Option<PacketToken> {
    todo!()
}

// ---------------------------------------------------------------------------
// ACK logic.

pub fn init_ack_ctx(_connection: &mut Connection, _ack_ctx: &mut AckContext) {
    todo!()
}

pub fn is_ack_needed(
    _connection: &mut Connection,
    _current_time: u64,
    _next_wake_time: &mut u64,
    _pc: PacketContext,
    _is_opportunistic: i32,
) -> bool {
    todo!()
}

pub fn is_pn_already_received(
    _connection: &mut Connection,
    _pc: PacketContext,
    _l_cid: Option<LocalCnxidToken>,
    _pn64: u64,
) -> bool {
    todo!()
}

pub fn record_pn_received(
    _connection: &mut Connection,
    _pc: PacketContext,
    _l_cid: Option<LocalCnxidToken>,
    _pn64: u64,
    _current_microsec: u64,
) -> i32 {
    todo!()
}

impl SackList {
    /// Choose at most `max_ranges` ACK ranges to put on the wire,
    /// updating per-range send counters.  `first_sack` selects the
    /// starting range; `None` starts at the highest-PN range.
    pub fn select_ack_ranges(
        &mut self,
        _first_sack: Option<SackItemToken>,
        _max_ranges: i32,
        _is_opportunistic: i32,
        _nb_sent_max: &mut i32,
        _nb_sent_max_skip: &mut i32,
    ) {
        todo!()
    }

    /// Merge the inclusive range `[pn64_min, pn64_max]` into the
    /// SACK list.  Returns Ok if the range was newly observed.
    pub fn update(
        &mut self,
        _pn64_min: u64,
        _pn64_max: u64,
        _current_time: u64,
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// True when every packet number in `[pn64_min, pn64_max]` is
    /// already covered by the SACK list.
    pub fn check(&mut self, _pn64_min: u64, _pn64_max: u64) -> bool {
        todo!()
    }

    /// Discard ACK ranges newly covered by an incoming ACK-of-ACK,
    /// returning the new last-acked range.
    pub fn process_ack_of_ack_range(
        &mut self,
        _previous: Option<SackItemToken>,
        _start_of_range: u64,
        _end_of_range: u64,
    ) -> Option<SackItemToken> {
        todo!()
    }

    /// Advance the ack horizon timestamp to drop expired ranges.
    pub fn update_ack_horizon(&mut self, _current_time: u64) {
        todo!()
    }

    /// First range in the list (highest PN), or `None` when empty.
    pub fn first_item(&mut self) -> Option<SackItemToken> {
        todo!()
    }

    /// Last range in the list (lowest PN), or `None` when empty.
    pub fn last_item(&mut self) -> Option<SackItemToken> {
        todo!()
    }

    /// Insert a new range `[range_min, range_max]`.
    pub fn insert_item(
        &mut self,
        _range_min: u64,
        _range_max: u64,
        _current_time: u64,
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// True when no ranges have been recorded.
    pub fn is_empty(&mut self) -> bool {
        todo!()
    }
}

/// Splay-tree successor of `sack` in `list`.  C: `sack_next_item`.
pub fn sack_next_item(_list: &mut SackList, _sack: SackItemToken) -> Option<SackItemToken> {
    todo!()
}

/// Splay-tree predecessor of `sack` in `list`.
pub fn sack_previous_item(_list: &mut SackList, _sack: SackItemToken) -> Option<SackItemToken> {
    todo!()
}

/// Borrow the ACK context for `(pc, l_cid)` on `connection`.  Returns
/// `None` when the requested context isn't installed.
pub fn ack_ctx_from_cnx_context(
    _connection: &mut Connection,
    _pc: PacketContext,
    _l_cid: Option<LocalCnxidToken>,
) -> Option<&mut AckContext> {
    todo!()
}

/// Borrow the SACK list for `(pc, l_cid)` on `connection`.  Returns
/// `None` when the requested context isn't installed.
pub fn sack_list_from_cnx_context(
    _connection: &mut Connection,
    _pc: PacketContext,
    _l_cid: Option<LocalCnxidToken>,
) -> Option<&mut SackList> {
    todo!()
}

impl SackList {
    /// Highest packet number currently sacked.
    pub fn first(&mut self) -> u64 {
        todo!()
    }

    /// Lowest packet number currently sacked.
    pub fn last(&mut self) -> u64 {
        todo!()
    }

    /// Topmost range, or `None` when empty.
    pub fn first_range(&mut self) -> Option<SackItemToken> {
        todo!()
    }

    /// Reset the list to "everything before this PN was acked".
    pub fn init(&mut self) {
        todo!()
    }

    /// As [`Self::init`] but seeds the list with one range.
    pub fn reset(
        &mut self,
        _range_min: u64,
        _range_max: u64,
        _current_time: u64,
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// Free all ranges and reset the list to empty.
    pub fn free(&mut self) {
        todo!()
    }

    /// Number of ranges currently stored.
    pub fn size(&mut self) -> usize {
        todo!()
    }
}

impl SackItem {
    /// Inclusive start of this SACK range.  C:
    /// `sack_item_range_start`.
    pub fn range_start(&self) -> u64 {
        todo!()
    }

    /// Exclusive end of this SACK range.  C:
    /// `sack_item_range_end`.
    pub fn range_end(&self) -> u64 {
        todo!()
    }

    /// Number of times this range has been sent in an ACK frame.
    /// C: `sack_item_nb_times_sent`.
    pub fn nb_times_sent(&self, _is_opportunistic: i32) -> i32 {
        todo!()
    }
}

impl SackList {
    /// Bump the per-range send counter for the item at `token`.
    pub fn item_record_sent(&mut self, _token: SackItemToken, _is_opportunistic: i32) {
        todo!()
    }

    /// Reset the per-range send counters for the item at `token`.
    pub fn item_record_reset(&mut self, _token: SackItemToken) {
        todo!()
    }
}

pub fn record_ack_packet_data(_packet_data: &mut PacketData, _acked_packet: &mut Packet) {
    todo!()
}

pub fn init_packet_ctx(
    _connection: &mut Connection,
    _pkt_ctx: &mut PacketContextState,
    _pc: PacketContext,
) {
    todo!()
}

pub fn process_ack_of_ack_frame(
    _first_sack: &mut SackList,
    _bytes: &mut [u8],
    _bytes_max: usize,
    _consumed: &mut usize,
    _is_ecn: i32,
) -> i32 {
    todo!()
}

pub fn compute_ack_gap_and_delay(
    _connection: &mut Connection,
    _rtt: u64,
    _remote_min_ack_delay: u64,
    _data_rate: u64,
    _ack_gap: &mut u64,
    _ack_delay_max: &mut u64,
) {
    todo!()
}

pub fn seed_bandwidth(
    _connection: &mut Connection,
    _rtt_min: u64,
    _cwin: u64,
    _ip_addr: &[u8],
    _ip_addr_length: u8,
) {
    todo!()
}

pub fn current_retransmit_timer(_connection: &mut Connection, _path_x: &mut Path) -> u64 {
    todo!()
}

pub fn update_path_rtt(
    _connection: &mut Connection,
    _old_path: &mut Path,
    _epoch: i32,
    _send_time: u64,
    _current_time: u64,
    _ack_delay: u64,
    _time_stamp: u64,
) {
    todo!()
}

// ---------------------------------------------------------------------------
// Stream management.

impl Connection {
    /// Create and register a new application stream with the given
    /// id.  Returns the stream's token in `self.streams`.
    pub fn create_stream(&mut self, _stream_id: u64) -> Result<StreamToken, crate::Error> {
        todo!()
    }

    /// Open every stream from "next remote stream" up through
    /// `stream_id` so the receiver sees a contiguous prefix.  The
    /// token returned names the highest-id stream just created.
    pub fn create_missing_streams(
        &mut self,
        _stream_id: u64,
        _is_remote: bool,
    ) -> Result<StreamToken, crate::Error> {
        todo!()
    }

    /// If `stream` has reached the closed state, free it.  Returns
    /// non-zero when the stream was actually deleted.
    pub fn delete_stream_if_closed(&mut self, _stream: &mut StreamHead) -> i32 {
        todo!()
    }

    /// Notify the connection that the remote's initial transport
    /// parameters arrived; propagate them to existing streams.
    pub fn update_stream_initial_remote(&mut self) {
        todo!()
    }

    /// Splice `stream` into the per-connection output queue.
    pub fn insert_output_stream(&mut self, _stream: &mut StreamHead) {
        todo!()
    }

    /// Remove `stream` from the per-connection output queue.
    pub fn remove_output_stream(&mut self, _stream: &mut StreamHead) {
        todo!()
    }

    /// Re-position `stream` in the output queue based on its current
    /// priority.
    pub fn reorder_output_stream(&mut self, _stream: &mut StreamHead) {
        todo!()
    }

    /// First stream in this connection's stream tree.
    pub fn first_stream(&mut self) -> Option<StreamToken> {
        todo!()
    }

    /// Last stream in this connection's stream tree.
    pub fn last_stream(&mut self) -> Option<StreamToken> {
        todo!()
    }

    /// Look up a stream by id.
    pub fn find_stream(&mut self, _stream_id: u64) -> Option<StreamToken> {
        todo!()
    }

    /// Open output streams to bridge a peer-side `MAX_STREAMS` bump
    /// from `old_limit` to `new_limit`.
    pub fn add_output_streams(&mut self, _old_limit: u64, _new_limit: u64, _is_bidir: bool) {
        todo!()
    }

    /// Pick the highest-priority ready stream that can send on
    /// `path_x`.  `is_coalesced` selects whether to consider streams
    /// already partially placed in the current packet.
    pub fn find_ready_stream_path(
        &mut self,
        _path_x: &mut Path,
        _is_coalesced: bool,
    ) -> Option<StreamToken> {
        todo!()
    }

    /// As [`Self::find_ready_stream_path`] but path-agnostic.
    pub fn find_ready_stream(&mut self) -> Option<StreamToken> {
        todo!()
    }

    /// True when the TLS handshake stream has data to send.
    pub fn is_tls_stream_ready(&mut self) -> bool {
        todo!()
    }
}

/// True when `stream` has finished sending and receiving all data.
pub fn is_stream_closed(_stream: &mut StreamHead, _client_mode: bool) -> bool {
    todo!()
}

// `stream_from_node` is gone — the C version recovered the parent
// pointer from an embedded `picosplay_node_t*` via `offsetof`.  In
// the Rust port the splay tree stores `StreamToken` directly, so
// the `Connection::stream_tree.get(token)` path is one indirection
// rather than offset arithmetic.

/// Splay-tree successor of `stream` in `connection.stream_tree`.
pub fn next_stream(_connection: &mut Connection, _stream: StreamToken) -> Option<StreamToken> {
    todo!()
}

pub fn decode_stream_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a [u8],
    _received_data: &mut StreamDataNode,
    _current_time: u64,
) -> Option<&'a [u8]> {
    todo!()
}

pub fn format_stream_frame<'a>(
    _connection: &mut Connection,
    _stream: &mut StreamHead,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _is_still_active: &mut i32,
    _ret: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn update_max_stream_id_local(_connection: &mut Connection, _stream: &mut StreamHead) {
    todo!()
}

// ---------------------------------------------------------------------------
// Frame retransmission.

pub fn check_frame_needs_repeat(
    _connection: &mut Connection,
    _bytes: &[u8],
    _bytes_max: usize,
    _p_type: PacketType,
    _no_need_to_repeat: &mut i32,
    _do_not_detect_spurious: &mut i32,
    _is_preemptive_needed: &mut i32,
) -> i32 {
    todo!()
}

pub fn format_available_stream_frames<'a>(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _bytes: &'a mut [u8],
    _current_priority: u64,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _stream_tried_and_failed: &mut i32,
    _ret: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn queue_data_repeat_init(_connection: &mut Connection) {
    todo!()
}

pub fn queue_data_repeat_packet(_connection: &mut Connection, _packet: &mut Packet) {
    todo!()
}

pub fn dequeue_data_repeat_packet(_connection: &mut Connection, _packet: &mut Packet) {
    todo!()
}

pub fn first_data_repeat_packet(_connection: &mut Connection) -> Option<PacketToken> {
    todo!()
}

pub fn copy_stream_frame_for_retransmit<'a>(
    _connection: &mut Connection,
    _packet: &mut Packet,
    _bytes: &'a mut [u8],
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn copy_stream_frames_for_retransmit<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _current_priority: u64,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn copy_before_retransmit(
    _old_p: &mut Packet,
    _connection: &mut Connection,
    _new_bytes: &mut [u8],
    _send_buffer_max_minus_checksum: usize,
    _packet_is_pure_ack: &mut i32,
    _do_not_detect_spurious: &mut i32,
    _force_queue: i32,
    _length: &mut usize,
    _add_to_data_repeat_queue: &mut i32,
) -> i32 {
    todo!()
}

pub fn retransmit_needed(
    _connection: &mut Connection,
    _pc: PacketContext,
    _path_x: &mut Path,
    _current_time: u64,
    _next_wake_time: &mut u64,
    _packet: &mut Packet,
    _send_buffer_max: usize,
    _header_length: &mut usize,
) -> i32 {
    todo!()
}

pub fn set_ack_needed(
    _connection: &mut Connection,
    _current_time: u64,
    _pc: PacketContext,
    _path_x: &mut Path,
    _is_immediate_ack_required: i32,
) {
    todo!()
}

pub fn process_ack_of_frames(_connection: &mut Connection, _p: &mut Packet, _is_spurious: i32) {
    todo!()
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

pub fn is_stream_frame_unlimited(_bytes: &[u8]) -> bool {
    todo!()
}

pub fn format_stream_frame_header(
    _bytes: &mut [u8],
    _stream_id: u64,
    _offset: u64,
) -> Option<&mut [u8]> {
    todo!()
}

pub fn parse_stream_header(
    _bytes: &[u8],
    _bytes_max: usize,
    _stream_id: &mut u64,
    _offset: &mut u64,
    _data_length: &mut usize,
    _fin: &mut i32,
    _consumed: &mut usize,
) -> i32 {
    todo!()
}

pub fn parse_ack_header(
    _bytes: &[u8],
    _bytes_max: usize,
    _num_block: &mut u64,
    _path_id: &mut u64,
    _largest: &mut u64,
    _ack_delay: &mut u64,
    _consumed: &mut usize,
    _ack_delay_exponent: u8,
) -> i32 {
    todo!()
}

pub fn decode_crypto_hs_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a [u8],
    _received_data: &mut StreamDataNode,
    _epoch: i32,
) -> Option<&'a [u8]> {
    todo!()
}

pub fn format_crypto_hs_frame<'a>(
    _stream: &mut StreamHead,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_ack_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _current_time: u64,
    _pc: PacketContext,
    _is_opportunistic: i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_connection_close_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_application_close_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_required_max_stream_data_frames<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_max_data_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _maxdata_increase: u64,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_max_stream_data_frame<'a>(
    _connection: &mut Connection,
    _stream: &mut StreamHead,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _new_max_data: u64,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn cc_increased_window(_connection: &mut Connection, _previous_window: u64) -> u64 {
    todo!()
}

pub fn format_max_streams_frame_if_needed<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn stream_data_node_recycle(_stream_data: &mut StreamDataNode) {
    todo!()
}

pub fn stream_data_node_alloc(_quic: &mut Quic) -> Result<StreamDataNode, crate::Error> {
    todo!()
}

pub fn clear_stream(_stream: &mut StreamHead) {
    todo!()
}

pub fn delete_stream(_connection: &mut Connection, _stream: &mut StreamHead) {
    todo!()
}

/// Find the local-CID list for `unique_path_id` (the index into
/// `connection.local_connection_id_lists`), optionally creating one when absent.
pub fn find_or_create_local_connection_id_list(
    _connection: &mut Connection,
    _unique_path_id: u64,
    _do_create: bool,
) -> Option<usize> {
    todo!()
}

pub fn create_local_connection_id(
    _connection: &mut Connection,
    _unique_path_id: u64,
    _suggested_value: Option<&ConnectionId>,
    _current_time: u64,
) -> Result<LocalCnxidToken, crate::Error> {
    todo!()
}

pub fn demote_local_connection_id_list(
    _connection: &mut Connection,
    _unique_path_id: u64,
    _reason: u64,
) -> i32 {
    todo!()
}

pub fn delete_local_connection_id(_connection: &mut Connection, _l_cid: LocalCnxidToken) {
    todo!()
}

/// Remove `connection.local_connection_id_lists[list_index]`.
pub fn delete_local_connection_id_list(_connection: &mut Connection, _list_index: usize) {
    todo!()
}

pub fn delete_local_connection_id_lists(_connection: &mut Connection) {
    todo!()
}

pub fn retire_local_connection_id(
    _connection: &mut Connection,
    _unique_path_id: u64,
    _sequence: u64,
) {
    todo!()
}

pub fn check_local_connection_id_ttl(
    _connection: &mut Connection,
    _local_connection_id_list: &mut LocalCnxidList,
    _current_time: u64,
    _next_wake_time: &mut u64,
) {
    todo!()
}

pub fn find_local_connection_id(
    _connection: &mut Connection,
    _unique_path_id: u64,
    _connection_id: &ConnectionId,
) -> Option<LocalCnxidToken> {
    todo!()
}

pub fn format_path_challenge_frame<'a>(
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _challenge: u64,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_path_response_frame<'a>(
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _challenge: u64,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn should_repeat_path_response_frame(
    _connection: &mut Connection,
    _bytes: &[u8],
    _bytes_max: usize,
) -> bool {
    todo!()
}

pub fn format_new_connection_id_frame<'a>(
    _connection: &mut Connection,
    _local_connection_id_list: &mut LocalCnxidList,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _l_cid: Option<LocalCnxidToken>,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_max_path_id_frame<'a>(
    _bytes: &'a mut [u8],
    _max_path_id: u64,
    _more_data: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_blocked_frames<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

impl Connection {
    /// Enqueue a RETIRE_CONNECTION_ID frame for `(unique_path_id,
    /// sequence)` to be sent on the next outgoing packet.
    pub fn queue_retire_connection_id_frame(
        &mut self,
        _unique_path_id: u64,
        _sequence: u64,
    ) -> Result<(), crate::Error> {
        todo!()
    }

    /// Enqueue a NEW_TOKEN frame carrying `token`.
    pub fn queue_new_token_frame(&mut self, _token: &[u8]) -> Result<(), crate::Error> {
        todo!()
    }
}

pub fn format_one_blocked_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _stream: &mut StreamHead,
) -> Option<&'a mut [u8]> {
    todo!()
}

/// Pop the head of a misc/datagram queue, encode it into `bytes`,
/// and return the remaining write tail.  Replaces the C "splice
/// the head out of a doubly-linked list" pattern -- the queue is
/// now a `VecDeque` and the front element is consumed by value.
pub fn format_first_misc_or_dg_frame<'a>(
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _queue: &mut VecDeque<MiscFrameHeader>,
) -> Option<&'a mut [u8]> {
    todo!()
}

/// Borrow the next misc-frame header in `connection` for packet context
/// `pc`.  C: `find_first_misc_frame`.
pub fn find_first_misc_frame(
    _connection: &mut Connection,
    _pc: PacketContext,
) -> Option<&mut MiscFrameHeader> {
    todo!()
}

pub fn format_misc_frames_in_context<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _pc: PacketContext,
) -> Option<&'a mut [u8]> {
    todo!()
}

/// Push a fresh misc/datagram frame onto `queue`.  C took two
/// `*mut *mut MiscFrameHeader` head/tail out-pointers; the Rust
/// shape just takes the queue and pushes at the back.
pub fn queue_misc_or_dg_frame(
    _connection: &mut Connection,
    _queue: &mut VecDeque<MiscFrameHeader>,
    _bytes: &[u8],
    _is_pure_ack: bool,
    _pc: PacketContext,
) -> Result<(), crate::Error> {
    todo!()
}

pub fn purge_misc_frames_after_ready(_connection: &mut Connection) {
    todo!()
}

/// Remove the entry at `index` from `queue`.  C: `delete_misc_or_dg`
/// (which spliced the node out of a doubly-linked list).
pub fn delete_misc_or_dg(_queue: &mut VecDeque<MiscFrameHeader>, _index: usize) {
    todo!()
}

pub fn clear_ack_ctx(_ack_ctx: &mut AckContext) {
    todo!()
}

pub fn reset_ack_context(_ack_ctx: &mut AckContext) {
    todo!()
}

impl Connection {
    /// Enqueue a HANDSHAKE_DONE frame.
    pub fn queue_handshake_done_frame(&mut self) -> Result<(), crate::Error> {
        todo!()
    }
}

pub fn format_first_datagram_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _is_first_in_packet: i32,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_ready_datagram_frame<'a>(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _ret: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn decode_datagram_frame_header<'a>(
    _bytes: &'a [u8],
    _frame_id: &mut u8,
    _length: &mut u64,
) -> Option<&'a [u8]> {
    todo!()
}

pub fn parse_ack_frequency_frame<'a>(
    _bytes: &'a [u8],
    _seq: &mut u64,
    _packets: &mut u64,
    _microsec: &mut u64,
    _ignore_order: &mut u8,
    _reordering_threshold: &mut u64,
) -> Option<&'a [u8]> {
    todo!()
}

pub fn format_ack_frequency_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_immediate_ack_frame<'a>(
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_time_stamp_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _current_time: u64,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn encode_time_stamp_length(_connection: &mut Connection, _current_time: u64) -> usize {
    todo!()
}

pub fn format_bdp_frame<'a>(
    _connection: &mut Connection,
    _bytes: &'a mut [u8],
    _path_x: &mut Path,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn format_path_abandon_frame<'a>(
    _bytes: &'a mut [u8],
    _more_data: &mut i32,
    _path_id: u64,
    _reason: u64,
) -> Option<&'a mut [u8]> {
    todo!()
}

impl Connection {
    /// Enqueue a PATH_ABANDON frame for `unique_path_id` carrying
    /// the close `reason`.
    pub fn queue_path_abandon_frame(
        &mut self,
        _unique_path_id: u64,
        _reason: u64,
    ) -> Result<(), crate::Error> {
        todo!()
    }
}

pub fn decode_frames(
    _connection: &mut Connection,
    _path_x: &mut Path,
    _bytes: &[u8],
    _bytes_max: usize,
    _received_data: &mut StreamDataNode,
    _epoch: i32,
    _addr_from: Option<&SocketAddr>,
    _addr_to: Option<&SocketAddr>,
    _pn64: u64,
    _path_is_not_allocated: i32,
    _current_time: u64,
) -> i32 {
    todo!()
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
    _bytes: &'a [u8],
    _ftype: u64,
) -> Option<(ObservedAddress<'a>, &'a [u8])> {
    todo!()
}

pub fn format_observed_address_frame<'a>(
    _bytes: &'a mut [u8],
    _ftype: u64,
    _sequence_number: u64,
    _addr: &[u8],
    _port: u16,
    _more_data: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn prepare_observed_address_frame<'a>(
    _bytes: &'a mut [u8],
    _path_x: &mut Path,
    _tuple: &mut Tuple,
    _current_time: u64,
    _next_wake_time: &mut u64,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> Option<&'a mut [u8]> {
    todo!()
}

pub fn update_peer_addr(_path_x: &mut Path, _peer_addr: Option<&SocketAddr>) {
    todo!()
}

pub fn skip_frame(
    _bytes: &[u8],
    _bytes_max: usize,
    _consumed: &mut usize,
    _pure_ack: &mut i32,
) -> i32 {
    todo!()
}

pub fn skip_path_abandon_frame(_bytes: &[u8]) -> Option<&[u8]> {
    todo!()
}

pub fn skip_path_available_or_backup_frame(_bytes: &[u8]) -> Option<&[u8]> {
    todo!()
}

pub fn is_path_challenging_packet(_bytes: &[u8], _bytes_maxsize: usize) -> bool {
    todo!()
}

impl Connection {
    /// Enqueue a PATH_AVAILABLE or PATH_BACKUP frame on `path_x`,
    /// determined by `status`.
    pub fn queue_path_available_or_backup_frame(
        &mut self,
        _path_x: &mut Path,
        _status: PathStatus,
    ) -> Result<(), crate::Error> {
        todo!()
    }
}

pub fn test_and_signal_new_path_allowed(_connection: &mut Connection) {
    todo!()
}

pub fn decode_closing_frames(
    _bytes: &mut [u8],
    _bytes_max: usize,
    _closing_received: &mut i32,
) -> i32 {
    todo!()
}

pub fn process_sooner_packets(_connection: &mut Connection, _current_time: u64) {
    todo!()
}

pub fn delete_sooner_packets(_connection: &mut Connection) {
    todo!()
}

// ---------------------------------------------------------------------------
// Transport extensions and version upgrade.

pub fn process_tp_version_negotiation<'a>(
    _bytes: &'a [u8],
    _extension_mode: i32,
    _envelop_vn: u32,
    _negotiated_vn: &mut u32,
    _negotiated_index: &mut i32,
    _vn_error: &mut u64,
) -> Option<&'a [u8]> {
    todo!()
}

pub fn prepare_transport_extensions(
    _connection: &mut Connection,
    _extension_mode: i32,
    _bytes: &mut [u8],
    _bytes_max: usize,
    _consumed: &mut usize,
) -> i32 {
    todo!()
}

pub fn receive_transport_extensions(
    _connection: &mut Connection,
    _extension_mode: i32,
    _bytes: &mut [u8],
    _bytes_max: usize,
    _consumed: &mut usize,
) -> i32 {
    todo!()
}

pub fn create_misc_frame(
    _bytes: &[u8],
    _is_pure_ack: bool,
    _pc: PacketContext,
) -> Result<MiscFrameHeader, crate::Error> {
    todo!()
}

pub fn process_version_upgrade(
    _connection: &mut Connection,
    _old_version_index: i32,
    _new_version_index: i32,
) -> i32 {
    todo!()
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
        current_time: u64,
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

#[cfg(test)]
mod test {}
