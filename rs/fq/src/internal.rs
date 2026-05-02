//! Translation of `quic/internal.h`.
//!
//! quic-core's grand internal header.  Anything that is not
//! part of the published `quic.h` API but is shared between
//! `.c` files of the core library lives here: connection /
//! path / stream / packet structures, frame and packet-type
//! enums, the giant `quic_t` and `cnx_t`
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
//! * `quic_t`, `cnx_t`, `path_t` were
//!   forward-declared as opaque stubs in
//!   [`crate`] (the public header).  Their
//!   real bodies live here; the public module re-exports the
//!   names so existing `use` paths in other modules keep
//!   working.
//! * Function pointer typedefs (`autoqlog_fn`,
//!   `performance_log_fn`, the spin-bit pair, the
//!   memlog hook on a connection, `mask_fns_t`) collapse to
//!   traits per the project rule.  When two function pointers
//!   are always installed together (`spinbit_def_t`,
//!   `mask_fns_t`) they share one trait.
//! * Conditional fields gated by `BBRExperiment` and
//!   `WITH_THREAD_CHECK` are dropped — they are not
//!   part of the v1 target build.
//! * The `PARSE_*`/`IS_*_STREAM_ID(_*)` macros that the C
//!   header inlines as preprocessor macros are translated to
//!   `pub const fn` helpers.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
// `*mut Self` chains in linked-list / splay nodes; clippy's
// `mutable_key_type` lint fires on `hash_table` lookups but
// the keys are Phase-3 opaque pointers and not actually keyed
// on interior mutability.

use core::ffi::c_void;
use core::net::SocketAddr;

use crate::hash::{hash_item, hash_table};
use crate::splay::{splay_node_t, splay_tree_t};
use crate::unified_log::UnifiedLogging;
use crate::{
    AlpnSelect, AlpnSelectV2, ConnectionIdCb, FreeVerifyCertificateCtx, Fuzz, RESET_SECRET_SIZE,
    StreamDataCb, StreamDirectReceive, congestion_algorithm_t, connection_id_t,
    lossbit_version_enum, packet_context_enum, path_status_enum, pmtud_policy_enum,
    ptls_verify_certificate_t, spinbit_version_enum, state_enum, tp_t,
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
pub enum pmtu_discovery_status_enum {
    #[default]
    pmtu_discovery_not_needed = 0,
    pmtu_discovery_optional,
    pmtu_discovery_required,
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
/// `version_parameters_t`.
///
/// `*aead_key` and `*retry_key` are static byte tables in the
/// C source — Rust models them as borrowed slices.  `upgrade_from`
/// is a `NULL`-terminated list in C; here it is a borrowed
/// slice, with an empty slice for "no upgrade path".
#[derive(Debug)]
pub struct version_parameters_t {
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
pub fn supported_versions() -> &'static [version_parameters_t] {
    todo!()
}

pub fn get_version_index(_proposed_version: u32) -> i32 {
    todo!()
}

// ---------------------------------------------------------------------------
// Crypto epochs and packet types.

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum epoch_enum {
    epoch_initial = 0,
    epoch_0rtt = 1,
    epoch_handshake = 2,
    epoch_1rtt = 3,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum packet_type_enum {
    packet_error = 0,
    packet_version_negotiation,
    packet_initial,
    packet_retry,
    packet_handshake,
    packet_0rtt_protected,
    packet_1rtt_protected,
    packet_type_max,
}

// ---------------------------------------------------------------------------
// Packet header.

/// Parsed long/short packet header.  C: `packet_header`.
///
/// The C struct uses a packed bitfield for eight single-bit
/// flags; Rust stores them as plain `bool` fields (one per flag).
pub struct packet_header {
    pub dest_cnx_id: connection_id_t,
    pub srce_cnx_id: connection_id_t,
    pub pn: u32,
    pub vn: u32,
    pub offset: usize,
    pub pn_offset: usize,
    pub ptype: packet_type_enum,
    pub pnmask: u64,
    pub pn64: u64,
    pub payload_length: usize,
    pub version_index: i32,
    pub epoch: epoch_enum,
    pub pc: packet_context_enum,

    pub key_phase: bool,
    pub spin: bool,
    pub has_spin_bit: bool,
    pub has_reserved_bit_set: bool,
    pub has_loss_bits: bool,
    pub loss_bit_Q: bool,
    pub loss_bit_L: bool,
    pub quic_bit_is_zero: bool,

    pub token_length: usize,
    pub token_bytes: *const u8,
    pub pl_val: usize,
    pub l_cid: *mut local_cnxid_t,
}

// ---------------------------------------------------------------------------
// Spin-bit policy vtable.

/// Spin-bit policy: how to update the spin bit on an incoming
/// packet, and what value to emit on outgoing packets.  In C the
/// policy is two function pointers grouped into
/// `spinbit_def_t`; the two are always installed
/// together so they share one Rust trait.
pub trait SpinBitPolicy {
    /// C: `spinbit_incoming_fn`.
    fn incoming(&self, cnx: &mut cnx_t, path_x: &mut path_t, ph: &packet_header);

    /// C: `spinbit_outgoing_fn`.
    fn outgoing(&self, cnx: &mut cnx_t) -> u8;
}

/// One row of the spin-bit policy dispatch table.  C:
/// `spinbit_def_t`.
pub struct spinbit_def_t {
    pub policy: &'static dyn SpinBitPolicy,
}

/// Replacement for `extern spinbit_def_t
/// spin_function_table[]`.  Returns the policy table as
/// a borrowed slice — length is implicit.
pub fn spin_function_table() -> &'static [spinbit_def_t] {
    todo!()
}

// ---------------------------------------------------------------------------
// Stateless packet, queued at the QUIC context until sendable.

pub struct stateless_packet_t {
    pub next_packet: *mut stateless_packet_t,
    pub addr_to: SocketAddr,
    pub addr_local: SocketAddr,
    pub if_index_local: i32,
    pub received_ecn: u8,
    pub length: usize,
    pub receive_time: u64,
    pub cnxid_log64: u64,
    pub initial_cid: connection_id_t,
    pub ptype: packet_type_enum,
    pub bytes: [u8; MAX_PACKET_SIZE],
}

pub fn create_stateless_packet(_quic: &mut quic_t) -> *mut stateless_packet_t {
    todo!()
}

pub fn queue_stateless_packet(_quic: &mut quic_t, _sp: *mut stateless_packet_t) {
    todo!()
}

pub fn dequeue_stateless_packet(_quic: &mut quic_t) -> *mut stateless_packet_t {
    todo!()
}

pub fn delete_stateless_packet(_sp: *mut stateless_packet_t) {
    todo!()
}

// ---------------------------------------------------------------------------
// Stream data nodes (received) and queue nodes (queued for send).

pub struct stream_data_node_t {
    pub stream_data_node: splay_node_t,
    pub quic: *mut quic_t,
    pub next_stream_data: *mut stream_data_node_t,
    pub offset: u64,
    pub length: usize,
    pub bytes: *const u8,
    pub data: [u8; MAX_PACKET_SIZE],
}

pub struct stream_queue_node_t {
    pub quic: *mut quic_t,
    pub next_stream_data: *mut stream_queue_node_t,
    pub offset: u64,
    pub length: usize,
    pub bytes: *mut u8,
}

// ---------------------------------------------------------------------------
// Sent packet (kept on retransmit queues until acked).

pub struct packet_t {
    pub packet_next: *mut packet_t,
    pub packet_previous: *mut packet_t,
    pub send_path: *mut path_t,
    pub queue_data_repeat_node: splay_node_t,
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
    pub ptype: packet_type_enum,
    pub pc: packet_context_enum,

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

pub fn create_packet(_quic: &mut quic_t) -> *mut packet_t {
    todo!()
}

pub fn recycle_packet(_quic: &mut quic_t, _packet: *mut packet_t) {
    todo!()
}

pub fn pad_to_policy(
    _cnx: &mut cnx_t,
    _bytes: &mut [u8],
    _length: usize,
    _max_length: u32,
) -> usize {
    todo!()
}

// ---------------------------------------------------------------------------
// Token register (replay protection for new tokens / retry tokens / tickets).

pub struct registered_token_t {
    pub registered_token_node: splay_node_t,
    pub token_time: u64,
    pub token_hash: u64,
    pub count: i32,
}

// ---------------------------------------------------------------------------
// 0-RTT remembered transport parameters.

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum tp_0rtt_enum {
    tp_0rtt_max_data = 0,
    tp_0rtt_max_stream_data_bidi_local = 1,
    tp_0rtt_max_stream_data_bidi_remote = 2,
    tp_0rtt_max_stream_data_uni = 3,
    tp_0rtt_max_streams_id_bidir = 4,
    tp_0rtt_max_streams_id_unidir = 5,
    tp_0rtt_rtt_local = 6,
    tp_0rtt_cwin_local = 7,
    tp_0rtt_rtt_remote = 8,
    tp_0rtt_cwin_remote = 9,
}

pub struct stored_ticket_t {
    pub next_ticket: *mut stored_ticket_t,
    pub sni: *mut core::ffi::c_char,
    pub alpn: *mut core::ffi::c_char,
    pub ip_addr: *mut u8,
    pub tp_0rtt: [u64; NB_TP_0RTT],
    pub ticket: *mut u8,
    pub time_valid_until: u64,
    pub sni_length: u16,
    pub alpn_length: u16,
    pub version: u32,
    pub ticket_length: u16,
    pub ip_addr_length: u8,
    pub ip_addr_client_length: u8,
    pub ip_addr_client: *mut u8,
    pub was_used: bool,
}

pub fn store_ticket(
    _quic: &mut quic_t,
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
    _tp: &tp_t,
) -> i32 {
    todo!()
}

pub fn get_stored_ticket(
    _quic: &mut quic_t,
    _sni: Option<&str>,
    _sni_length: u16,
    _alpn: Option<&str>,
    _alpn_length: u16,
    _version: u32,
    _need_unused: i32,
    _ticket_id: u64,
) -> *mut stored_ticket_t {
    todo!()
}

pub fn get_ticket(
    _quic: &mut quic_t,
    _sni: Option<&str>,
    _sni_length: u16,
    _alpn: Option<&str>,
    _alpn_length: u16,
    _version: u32,
    _ticket: &mut *mut u8,
    _ticket_length: &mut u16,
    _tp: &mut tp_t,
    _mark_used: i32,
) -> i32 {
    todo!()
}

pub fn get_ticket_and_version(
    _quic: &mut quic_t,
    _sni: Option<&str>,
    _sni_length: u16,
    _alpn: Option<&str>,
    _alpn_length: u16,
    _version: u32,
    _ticket_version: &mut u32,
    _ticket: &mut *mut u8,
    _ticket_length: &mut u16,
    _tp: &mut tp_t,
    _mark_used: i32,
) -> i32 {
    todo!()
}

pub fn save_tickets(
    _first_ticket: *const stored_ticket_t,
    _current_time: u64,
    _ticket_file_name: &str,
) -> i32 {
    todo!()
}

pub fn load_tickets(_quic: &mut quic_t, _ticket_file_name: &str) -> i32 {
    todo!()
}

pub fn free_tickets(_pp_first_ticket: &mut *mut stored_ticket_t) {
    todo!()
}

pub fn seed_ticket(_cnx: &mut cnx_t, _path_x: &mut path_t) {
    todo!()
}

// ---------------------------------------------------------------------------
// Stored retry-token (for client side, indexed by SNI + IP).

pub struct stored_token_t {
    pub next_token: *mut stored_token_t,
    pub sni: *const core::ffi::c_char,
    pub token: *const u8,
    pub ip_addr: *const u8,
    pub time_valid_until: u64,
    pub sni_length: u16,
    pub token_length: u16,
    pub ip_addr_length: u8,
    pub was_used: bool,
}

pub fn store_token(
    _quic: &mut quic_t,
    _sni: Option<&str>,
    _sni_length: u16,
    _ip_addr: &[u8],
    _ip_addr_length: u8,
    _token: &[u8],
    _token_length: u16,
) -> i32 {
    todo!()
}

pub fn get_token(
    _quic: &mut quic_t,
    _sni: Option<&str>,
    _sni_length: u16,
    _ip_addr: &[u8],
    _ip_addr_length: u8,
    _token: &mut *mut u8,
    _token_length: &mut u16,
    _mark_used: i32,
) -> i32 {
    todo!()
}

pub fn save_tokens(_quic: &mut quic_t, _token_file_name: &str) -> i32 {
    todo!()
}

pub fn load_tokens(_quic: &mut quic_t, _token_file_name: &str) -> i32 {
    todo!()
}

pub fn free_tokens(_pp_first_token: &mut *mut stored_token_t) {
    todo!()
}

// ---------------------------------------------------------------------------
// Issued-tickets bookkeeping (server side, per ticket-id index).

pub struct issued_ticket_t {
    pub next_ticket: *mut issued_ticket_t,
    pub previous_ticket: *mut issued_ticket_t,
    pub hash_item: hash_item,
    pub ticket_id: u64,
    pub creation_time: u64,
    pub rtt: u64,
    pub cwin: u64,
    pub ip_addr: [u8; 16],
    pub ip_addr_length: u8,
}

pub fn remember_issued_ticket(
    _quic: &mut quic_t,
    _ticket_id: u64,
    _rtt: u64,
    _cwin: u64,
    _ip_addr: &[u8],
    _ip_addr_length: u8,
) -> i32 {
    todo!()
}

pub fn retrieve_issued_ticket(_quic: &mut quic_t, _ticket_id: u64) -> *mut issued_ticket_t {
    todo!()
}

// ---------------------------------------------------------------------------
// Auto-qlog and performance log callbacks (function pointers → traits).

/// C: `autoqlog_fn` — invoked at end of connection to
/// turn the binlog into a qlog file.  Returns 0 on success, an
/// errno-style negative on failure.
pub trait AutoQlog {
    fn run(&mut self, cnx: &mut cnx_t) -> i32;
}

/// C: `performance_log_fn` — emit a per-connection
/// performance log row.  `should_delete` is `true` on connection
/// teardown.
pub trait PerformanceLog {
    fn emit(&mut self, quic: &mut quic_t, cnx: &mut cnx_t, should_delete: bool) -> i32;
}

// ---------------------------------------------------------------------------
// Memlog hook (per-connection memory-log callback on `cnx_t`).

/// C: `void (*memlog_call_back)(cnx_t*, path_t*,
/// void* v_memlog, int op_code, uint64_t current_time)` field on
/// `cnx_t`.
pub trait MemLogHook {
    fn callback(&mut self, cnx: &mut cnx_t, path: &mut path_t, op_code: i32, current_time: u64);
}

// ---------------------------------------------------------------------------
// QUIC context.

/// Top-level QUIC context.  C: `quic_t`.  Single-threaded
/// scope (per the translation plan): no `Send`/`Sync`.
pub struct quic_t {
    pub tls_master_ctx: *mut c_void,
    pub default_callback_fn: Option<Box<dyn StreamDataCb>>,
    pub default_callback_ctx: *mut c_void,
    pub mask_ctx: *mut c_void,
    pub mask_fns: Option<Box<dyn maskOps>>,
    pub default_alpn: *const core::ffi::c_char,
    pub alpn_select_fn: Option<Box<dyn AlpnSelect>>,
    pub alpn_select_fn_v2: Option<Box<dyn AlpnSelectV2>>,
    pub reset_seed: [u8; RESET_SECRET_SIZE],
    pub retry_seed: [u8; RETRY_SECRET_SIZE],
    pub p_simulated_time: *mut u64,
    pub hash_seed: [u8; 16],
    pub ticket_file_name: *const core::ffi::c_char,
    pub token_file_name: *const core::ffi::c_char,
    pub p_first_ticket: *mut stored_ticket_t,
    pub p_first_token: *mut stored_token_t,
    pub token_reuse_tree: splay_tree_t,
    pub local_cnxid_length: u8,
    pub default_stream_priority: u8,
    pub default_datagram_priority: u8,
    pub local_cnxid_ttl: u64,
    pub mtu_max: u32,
    pub padding_multiple_default: u32,
    pub padding_minsize_default: u32,
    pub sequence_hole_pseudo_period: u32,
    pub default_pmtud_policy: pmtud_policy_enum,
    pub default_spin_policy: spinbit_version_enum,
    pub default_lossbit_policy: lossbit_version_enum,
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

    pub pending_stateless_packet: *mut stateless_packet_t,

    pub default_congestion_alg: *const congestion_algorithm_t,
    pub default_congestion_alg_option_string: *const core::ffi::c_char,

    pub cnx_list: *mut cnx_t,
    pub cnx_last: *mut cnx_t,
    pub cnx_wake_tree: splay_tree_t,

    pub cnx_in_progress: *mut cnx_t,

    pub table_cnx_by_id: *mut hash_table,
    pub table_cnx_by_net: *mut hash_table,
    pub table_cnx_by_icid: *mut hash_table,
    pub table_cnx_by_secret: *mut hash_table,

    pub table_issued_tickets: *mut hash_table,
    pub table_issued_tickets_first: *mut issued_ticket_t,
    pub table_issued_tickets_last: *mut issued_ticket_t,
    pub table_issued_tickets_nb: usize,

    pub p_first_packet: *mut packet_t,
    pub nb_packets_in_pool: i32,
    pub nb_packets_allocated: i32,
    pub nb_packets_allocated_max: i32,

    pub p_first_data_node: *mut stream_data_node_t,
    pub nb_data_nodes_in_pool: i32,
    pub nb_data_nodes_allocated: i32,
    pub nb_data_nodes_allocated_max: i32,

    pub cnx_id_callback_fn: Option<Box<dyn ConnectionIdCb>>,
    pub cnx_id_callback_ctx: *mut c_void,

    pub aead_encrypt_ticket_ctx: *mut c_void,
    pub aead_decrypt_ticket_ctx: *mut c_void,
    pub retry_integrity_sign_ctx: *mut *mut c_void,
    pub retry_integrity_verify_ctx: *mut *mut c_void,

    pub verify_certificate_callback: *mut ptls_verify_certificate_t,
    pub free_verify_certificate_callback_fn: Option<Box<dyn FreeVerifyCertificateCtx>>,

    pub default_tp: tp_t,

    pub fuzz_fn: Option<Box<dyn Fuzz>>,
    pub fuzz_ctx: *mut c_void,
    pub wake_file: i32,
    pub wake_line: i32,

    pub max_data_limit: u64,

    pub rtt_update_delta: u64,
    pub pacing_rate_update_delta: u64,

    pub F_log: *mut c_void,
    pub binlog_dir: *mut core::ffi::c_char,
    pub qlog_dir: *mut core::ffi::c_char,
    pub autoqlog_fn: Option<Box<dyn AutoQlog>>,
    pub text_log_fns: Option<Box<dyn UnifiedLogging>>,
    pub bin_log_fns: Option<Box<dyn UnifiedLogging>>,
    pub qlog_fns: Option<Box<dyn UnifiedLogging>>,
    pub perflog_fn: Option<Box<dyn PerformanceLog>>,
    pub v_perflog_ctx: *mut c_void,
    pub v_thread_ctx: *mut c_void,
}

pub fn context_from_epoch(_epoch: i32) -> packet_context_enum {
    todo!()
}

pub fn registered_token_check_reuse(
    _quic: &mut quic_t,
    _token: &[u8],
    _token_length: usize,
    _expiry_time: u64,
) -> i32 {
    todo!()
}

pub fn registered_token_clear(_quic: &mut quic_t, _expiry_time_max: u64) {
    todo!()
}

// ---------------------------------------------------------------------------
// SACK list (used for both packet-number and stream-byte ranges).

pub struct sack_item_t {
    pub node: splay_node_t,
    pub start_of_sack_range: u64,
    pub end_of_sack_range: u64,
    pub time_created: u64,
    pub nb_times_sent: [i32; 2],
}

pub struct sack_range_count_t {
    pub range_counts: [i32; MAX_ACK_RANGE_REPEAT],
}

pub struct sack_list_t {
    pub ack_tree: splay_tree_t,
    pub ack_horizon: u64,
    pub horizon_delay: i64,
    pub rc: [sack_range_count_t; 2],
}

// ---------------------------------------------------------------------------
// Stream head.

pub struct stream_head_t {
    pub stream_node: splay_node_t,
    pub next_output_stream: *mut stream_head_t,
    pub previous_output_stream: *mut stream_head_t,
    pub cnx: *mut cnx_t,
    pub stream_id: u64,
    pub affinity_path: *mut path_t,
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
    pub stream_data_tree: splay_tree_t,
    pub sent_offset: u64,
    pub reliable_size: u64,
    pub send_queue: *mut stream_queue_node_t,
    pub app_stream_ctx: *mut c_void,
    pub direct_receive_fn: Option<Box<dyn StreamDirectReceive>>,
    pub direct_receive_ctx: *mut c_void,
    pub sack_list: sack_list_t,
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

#[inline]
pub const fn IS_CLIENT_STREAM_ID(id: u64) -> bool {
    (id & 1) == 0
}

#[inline]
pub const fn IS_BIDIR_STREAM_ID(id: u64) -> bool {
    (id & 2) == 0
}

#[inline]
pub const fn IS_LOCAL_STREAM_ID(id: u64, client_mode: u64) -> bool {
    ((id ^ client_mode) & 1) != 0
}

#[inline]
pub const fn STREAM_ID_FROM_RANK(rank: u64, client_mode: u64, is_unidir: u64) -> u64 {
    ((rank - 1) << 2) | (is_unidir << 1) | (client_mode ^ 1)
}

#[inline]
pub const fn STREAM_RANK_FROM_ID(id: u64) -> u64 {
    (id + 4) >> 2
}

#[inline]
pub const fn STREAM_TYPE_FROM_ID(id: u64) -> u64 {
    id & 3
}

#[inline]
pub const fn NEXT_STREAM_ID_FOR_TYPE(id: u64) -> u64 {
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
pub struct misc_frame_header_t {
    pub next_misc_frame: *mut misc_frame_header_t,
    pub previous_misc_frame: *mut misc_frame_header_t,
    pub length: usize,
    pub pc: packet_context_enum,
    pub is_pure_ack: i32,
}

// ---------------------------------------------------------------------------
// Per-epoch packet/ACK contexts.

pub struct packet_context_t {
    pub send_sequence: u64,
    pub next_sequence_hole: u64,
    pub retransmit_sequence: u64,
    pub highest_acknowledged: u64,
    pub latest_time_acknowledged: u64,
    pub highest_acknowledged_time: u64,
    pub pending_last: *mut packet_t,
    pub pending_first: *mut packet_t,
    pub retransmitted_newest: *mut packet_t,
    pub retransmitted_oldest: *mut packet_t,
    pub preemptive_repeat_ptr: *mut packet_t,
    pub retransmitted_queue_size: u64,
    pub ecn_ect0_total_remote: u64,
    pub ecn_ect1_total_remote: u64,
    pub ecn_ce_total_remote: u64,
    pub ack_of_ack_requested: bool,
}

pub struct ack_context_track_t {
    pub highest_ack_sent: u64,
    pub highest_ack_sent_time: u64,
    pub time_oldest_unack_packet_received: u64,

    pub ack_needed: bool,
    pub ack_after_fin: bool,
    pub out_of_order_received: bool,
    pub is_immediate_ack_required: bool,
}

pub struct ack_context_t {
    pub sack_list: sack_list_t,
    pub time_stamp_largest_received: u64,
    pub act: [ack_context_track_t; 2],
    pub crypto_rotation_sequence: u64,

    pub ecn_ect0_total_local: u64,
    pub ecn_ect1_total_local: u64,
    pub ecn_ce_total_local: u64,
    pub sending_ecn_ack: bool,
}

// ---------------------------------------------------------------------------
// CID state — local and remote.

pub struct local_cnxid_t {
    pub next: *mut local_cnxid_t,
    pub registered_cnx: *mut cnx_t,
    pub hash_item: hash_item,
    pub path_id: u64,
    pub sequence: u64,
    pub create_time: u64,
    pub cnx_id: connection_id_t,
    pub is_acked: bool,
}

pub struct local_cnxid_list_t {
    pub next_list: *mut local_cnxid_list_t,
    pub unique_path_id: u64,
    pub local_cnxid_sequence_next: u64,
    pub local_cnxid_retire_before: u64,
    pub local_cnxid_oldest_created: u64,
    pub nb_local_cnxid: i32,
    pub nb_local_cnxid_expired: i32,
    pub is_demoted: bool,
    pub demotion_time: u64,
    pub local_cnxid_first: *mut local_cnxid_t,
}

pub struct remote_cnxid_t {
    pub next: *mut remote_cnxid_t,
    pub sequence: u64,
    pub cnx_id: connection_id_t,
    pub reset_secret: [u8; RESET_SECRET_SIZE],
    pub nb_path_references: i32,
    pub needs_removal: bool,
    pub retire_sent: bool,
    pub retire_acked: bool,
    pub pkt_ctx: packet_context_t,
}

pub struct remote_cnxid_stash_t {
    pub next_stash: *mut remote_cnxid_stash_t,
    pub unique_path_id: u64,
    pub retire_cnxid_before: u64,
    pub cnxid_stash_first: *mut remote_cnxid_t,
    pub is_in_use: bool,
}

// ---------------------------------------------------------------------------
// Pacing and tuple/path state.

pub struct pacing_t {
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

pub struct tuple_t {
    pub unique_path_id: u64,
    pub next_tuple: *mut tuple_t,
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub if_index: core::ffi::c_ulong,
    pub observed_addr: SocketAddr,
    pub p_remote_cnxid: *mut remote_cnxid_t,
    pub p_local_cnxid: *mut local_cnxid_t,
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

pub struct path_t {
    pub registered_peer_addr: SocketAddr,
    pub net_id_hash_item: hash_item,
    pub cnx: *mut cnx_t,
    pub unique_path_id: u64,
    pub app_path_ctx: *mut c_void,
    pub ack_ctx: ack_context_t,
    pub pkt_ctx: packet_context_t,
    pub first_tuple: *mut tuple_t,
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
    pub congestion_alg_state: *mut c_void,
    pub pacing: pacing_t,

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

pub struct crypto_context_t {
    pub aead_encrypt: *mut c_void,
    pub aead_decrypt: *mut c_void,
    pub pn_enc: *mut c_void,
    pub pn_dec: *mut c_void,
}

// ---------------------------------------------------------------------------
// Connection context.

/// Per-connection state.  C: `cnx_t`.  This is the
/// largest and longest-lived structure in the library; almost
/// every internal function takes `cnx` as its first argument.
pub struct cnx_t {
    pub quic: *mut quic_t,

    pub next_in_table: *mut cnx_t,
    pub previous_in_table: *mut cnx_t,

    pub proposed_version: u32,
    pub rejected_version: u32,
    pub desired_version: u32,
    pub version_index: i32,

    pub is_0RTT_accepted: bool,
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

    pub pmtud_policy: pmtud_policy_enum,
    pub spin_policy: spinbit_version_enum,
    pub idle_timeout: u64,
    pub local_parameters: tp_t,
    pub remote_parameters: tp_t,
    pub padding_multiple: u32,
    pub padding_minsize: u32,
    pub seed_ip_addr: [u8; STORED_IP_MAX],
    pub seed_ip_addr_length: u8,
    pub seed_rtt_min: u64,
    pub seed_cwin: u64,

    pub issued_ticket_id: u64,
    pub resumed_ticket_id: u64,

    pub sni: *const core::ffi::c_char,
    pub alpn: *const core::ffi::c_char,
    pub max_early_data_size: usize,

    pub callback_fn: Option<Box<dyn StreamDataCb>>,
    pub callback_ctx: *mut c_void,

    pub cnx_state: state_enum,
    pub initial_cnxid: connection_id_t,
    pub original_cnxid: connection_id_t,
    pub registered_icid_addr: SocketAddr,
    pub registered_icid_item: hash_item,
    pub registered_secret_addr: SocketAddr,
    pub registered_reset_secret: [u8; RESET_SECRET_SIZE],
    pub registered_reset_secret_item: hash_item,

    pub start_time: u64,
    pub phase_delay: i64,
    pub application_error: u64,
    pub local_error: u64,
    pub local_error_reason: *const core::ffi::c_char,
    pub remote_application_error: u64,
    pub remote_error: u64,
    pub offending_frame_type: u64,
    pub remote_error_reason: *mut core::ffi::c_char,
    pub retry_token_length: u16,
    pub retry_token: *mut u8,

    pub next_wake_time: u64,
    pub cnx_wake_node: splay_node_t,
    pub app_wake_time: u64,

    pub tls_ctx: *mut c_void,
    pub crypto_epoch_length_max: u64,
    pub crypto_epoch_sequence: u64,
    pub crypto_rotation_time_guard: u64,
    pub tls_sendbuf: *mut c_void,
    pub psk_cipher_suite_id: u16,

    pub tls_stream: [stream_head_t; NUMBER_OF_EPOCHS],
    pub crypto_context: [crypto_context_t; NUMBER_OF_EPOCHS],
    pub crypto_context_old: crypto_context_t,
    pub crypto_context_new: crypto_context_t,
    pub crypto_failure_count: u64,

    pub latest_progress_time: u64,
    pub latest_receive_time: u64,
    pub last_close_sent: u64,
    pub pkt_ctx: [packet_context_t; 3], // nb_packet_context = 3
    pub ack_ctx: [ack_context_t; 3],
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

    pub congestion_alg: *const congestion_algorithm_t,
    pub congestion_alg_option_string: *const core::ffi::c_char,

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

    pub first_misc_frame: *mut misc_frame_header_t,
    pub last_misc_frame: *mut misc_frame_header_t,

    pub stream_tree: splay_tree_t,
    pub first_output_stream: *mut stream_head_t,
    pub last_output_stream: *mut stream_head_t,
    pub high_priority_stream_id: u64,
    pub next_stream_id: [u64; 4],
    pub priority_limit_for_bypass: u64,

    pub queue_data_repeat_tree: splay_tree_t,

    pub first_datagram: *mut misc_frame_header_t,
    pub last_datagram: *mut misc_frame_header_t,
    pub datagram_priority: u64,
    pub datagram_conflicts_count: i32,
    pub datagram_conflicts_max: i32,

    pub keep_alive_interval: u64,

    pub path: *mut *mut path_t,
    pub nb_paths: i32,
    pub nb_path_alloc: i32,
    pub last_path_polled: i32,
    pub unique_path_id_next: u64,
    pub nominal_path_for_ack: *mut path_t,
    pub status_sequence_to_send_next: u64,
    pub max_path_id_local: u64,
    pub max_path_id_acknowledged: u64,
    pub max_path_id_remote: u64,
    pub paths_blocked_acknowledged: u64,

    pub first_remote_cnxid_stash: *mut remote_cnxid_stash_t,

    pub nb_local_cnxid_lists: u64,
    pub next_path_id_in_lists: u64,
    pub max_path_id_in_cnxid_lists: u64,
    pub first_local_cnxid_list: *mut local_cnxid_list_t,

    pub ack_frequency_sequence_local: u64,
    pub ack_gap_local: u64,
    pub ack_frequency_delay_local: u64,
    pub ack_frequency_sequence_remote: u64,
    pub ack_gap_remote: u64,
    pub ack_delay_remote: u64,
    pub ack_reordering_threshold_remote: u64,

    pub first_sooner: *mut stateless_packet_t,
    pub last_sooner: *mut stateless_packet_t,

    pub log_unique: u16,
    pub f_binlog: *mut c_void,
    pub binlog_file_name: *mut core::ffi::c_char,
    pub memlog_call_back: Option<Box<dyn MemLogHook>>,
    pub memlog_ctx: *mut c_void,
    pub qlog_ctx: *mut c_void,
}

// `cnx_t` carries `Box<dyn Trait>` and raw pointers, so a
// derived `Debug` is impossible.  Hand-roll a marker impl so
// containers that store `cnx_t` references can still
// derive `Debug`.
impl core::fmt::Debug for cnx_t {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("cnx_t").finish_non_exhaustive()
    }
}

impl core::fmt::Debug for quic_t {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("quic_t").finish_non_exhaustive()
    }
}

impl core::fmt::Debug for path_t {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("path_t").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Per-incoming-packet ack accounting (filled in while processing a packet).

pub struct packet_data_path_ack_t {
    pub acked_path: *mut path_t,
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

pub struct packet_data_t {
    pub last_time_stamp_received: u64,
    pub last_ack_delay: u64,
    pub nb_path_ack: i32,
    pub path_ack: [packet_data_path_ack_t; NB_PATH_TARGET],
}

// ---------------------------------------------------------------------------
// Connection lifecycle / registration.

pub fn create_cnx_internal(
    _quic: &mut quic_t,
    _initial_cnx_id: connection_id_t,
    _remote_cnx_id: connection_id_t,
    _addr_to: Option<&SocketAddr>,
    _start_time: u64,
    _preferred_version: u32,
    _sni: Option<&str>,
    _alpn: Option<&str>,
    _client_mode: bool,
    _initial_aead_dec: *mut c_void,
    _initial_pn_dec: *mut c_void,
) -> *mut cnx_t {
    todo!()
}

pub fn load_token_file(_quic: &mut quic_t, _token_file_name: &str) -> i32 {
    todo!()
}

pub fn init_transport_parameters(_tp: &mut tp_t) {
    todo!()
}

pub fn register_cnx_id(_quic: &mut quic_t, _cnx: &mut cnx_t, _l_cid: &mut local_cnxid_t) -> i32 {
    todo!()
}

pub fn register_net_secret(_cnx: &mut cnx_t) -> i32 {
    todo!()
}

pub fn register_net_icid(_cnx: &mut cnx_t) -> i32 {
    todo!()
}

pub fn create_local_cnx_id(
    _quic: &mut quic_t,
    _cnx_id: &mut connection_id_t,
    _cnx_id_remote: connection_id_t,
) {
    todo!()
}

// ---------------------------------------------------------------------------
// Tuple/path management.

pub fn create_tuple(
    _path_x: &mut path_t,
    _local_addr: Option<&SocketAddr>,
    _peer_addr: Option<&SocketAddr>,
    _if_index: i32,
) -> *mut tuple_t {
    todo!()
}

pub fn delete_demoted_tuples(_cnx: &mut cnx_t, _current_time: u64, _next_wake_time: &mut u64) {
    todo!()
}

pub fn delete_tuple(_path_x: &mut path_t, _tuple: *mut tuple_t, _is_deleting_path: i32) {
    todo!()
}

pub fn set_first_tuple(_path_x: &mut path_t, _tuple: *mut tuple_t) {
    todo!()
}

pub fn create_path(
    _cnx: &mut cnx_t,
    _start_time: u64,
    _local_addr: Option<&SocketAddr>,
    _peer_addr: Option<&SocketAddr>,
    _if_index: i32,
    _unique_path_id: u64,
) -> i32 {
    todo!()
}

pub fn register_path(_cnx: &mut cnx_t, _path_x: &mut path_t) {
    todo!()
}

pub fn find_incoming_path(
    _cnx: &mut cnx_t,
    _ph: &mut packet_header,
    _addr_from: &mut SocketAddr,
    _addr_to: &mut SocketAddr,
    _if_index_to: i32,
    _current_time: u64,
    _p_path_id: &mut i32,
) -> i32 {
    todo!()
}

pub fn prepare_path_control_packet(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _tuple: &mut tuple_t,
    _packet: &mut packet_t,
    _current_time: u64,
    _send_buffer: &mut [u8],
    _send_buffer_max: usize,
    _send_length: &mut usize,
    _next_wake_time: &mut u64,
) -> i32 {
    todo!()
}

pub fn prepare_path_challenge_frames(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _bytes_next: &mut [u8],
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _is_challenge_padding_needed: &mut i32,
    _current_time: u64,
    _next_wake_time: &mut u64,
) -> *mut u8 {
    todo!()
}

pub fn select_next_path_tuple(
    _cnx: &mut cnx_t,
    _current_time: u64,
    _next_wake_time: &mut u64,
    _next_path: &mut *mut path_t,
    _next_tuple: &mut *mut tuple_t,
) {
    todo!()
}

pub fn renew_connection_id(_cnx: &mut cnx_t, _path_id: i32) -> i32 {
    todo!()
}

pub fn delete_path(_cnx: &mut cnx_t, _path_index: i32) {
    todo!()
}

pub fn demote_path(_cnx: &mut cnx_t, _path_index: i32, _current_time: u64, _reason: u64) {
    todo!()
}

pub fn retransmit_demoted_path(_cnx: &mut cnx_t, _path_x: &mut path_t, _current_time: u64) {
    todo!()
}

pub fn queue_retransmit_on_ack(_cnx: &mut cnx_t, _path_x: &mut path_t, _current_time: u64) {
    todo!()
}

pub fn delete_abandoned_paths(_cnx: &mut cnx_t, _current_time: u64, _next_wake_time: &mut u64) {
    todo!()
}

pub fn set_tuple_challenge(
    _tuple: &mut tuple_t,
    _current_time: u64,
    _use_constant_challenges: i32,
) {
    todo!()
}

pub fn set_path_challenge(_cnx: &mut cnx_t, _path_id: i32, _current_time: u64) {
    todo!()
}

pub fn find_path_by_address(
    _cnx: &mut cnx_t,
    _addr_local: Option<&SocketAddr>,
    _addr_peer: Option<&SocketAddr>,
    _partial_match: &mut i32,
) -> i32 {
    todo!()
}

pub fn find_path_by_unique_id(_cnx: &mut cnx_t, _unique_path_id: u64) -> i32 {
    todo!()
}

pub fn check_cid_for_new_tuple(_cnx: &mut cnx_t, _unique_path_id: u64) -> i32 {
    todo!()
}

pub fn assign_peer_cnxid_to_tuple(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _tuple: &mut tuple_t,
) -> i32 {
    todo!()
}

pub fn reset_path_mtu(_path_x: &mut path_t) {
    todo!()
}

pub fn get_path_id_from_unique(_cnx: &mut cnx_t, _unique_path_id: u64) -> i32 {
    todo!()
}

pub fn find_or_create_remote_cnxid_stash(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _do_create: i32,
) -> *mut remote_cnxid_stash_t {
    todo!()
}

// ---------------------------------------------------------------------------
// Remote CID stash management.

pub fn init_cnxid_stash(_cnx: &mut cnx_t) -> i32 {
    todo!()
}

pub fn add_remote_cnxid_to_stash(
    _cnx: &mut cnx_t,
    _remote_cnxid_stash: &mut remote_cnxid_stash_t,
    _retire_before_next: u64,
    _sequence: u64,
    _cid_length: u8,
    _cnxid_bytes: *const u8,
    _secret_bytes: *const u8,
    _pstashed: &mut *mut remote_cnxid_t,
) -> u64 {
    todo!()
}

pub fn stash_remote_cnxid(
    _cnx: &mut cnx_t,
    _retire_before_next: u64,
    _unique_path_id: u64,
    _sequence: u64,
    _cid_length: u8,
    _cnxid_bytes: *const u8,
    _secret_bytes: *const u8,
    _pstashed: &mut *mut remote_cnxid_t,
) -> u64 {
    todo!()
}

pub fn remove_cnxid_from_stash(
    _cnx: &mut cnx_t,
    _remote_cnxid_stash: &mut remote_cnxid_stash_t,
    _removed: *mut remote_cnxid_t,
    _previous: *mut remote_cnxid_t,
) -> *mut remote_cnxid_t {
    todo!()
}

pub fn remove_stashed_cnxid(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _removed: *mut remote_cnxid_t,
    _previous: *mut remote_cnxid_t,
) -> *mut remote_cnxid_t {
    todo!()
}

pub fn get_cnxid_from_stash(_stash: &mut remote_cnxid_stash_t) -> *mut remote_cnxid_t {
    todo!()
}

pub fn obtain_stashed_cnxid(_cnx: &mut cnx_t, _unique_path_id: u64) -> *mut remote_cnxid_t {
    todo!()
}

pub fn dereference_stashed_cnxid(_cnx: &mut cnx_t, _path_x: &mut path_t, _is_deleting_cnx: i32) {
    todo!()
}

pub fn dereference_stashed_cnxid_tuple(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _tuple: &mut tuple_t,
    _is_deleting_cnx: i32,
) {
    todo!()
}

pub fn remove_not_before_from_stash(
    _cnx: &mut cnx_t,
    _cnxid_stash: &mut remote_cnxid_stash_t,
    _not_before: u64,
    _current_time: u64,
) -> u64 {
    todo!()
}

pub fn delete_remote_cnxid_stash(_cnx: &mut cnx_t, _cnxid_stash: *mut remote_cnxid_stash_t) {
    todo!()
}

pub fn remove_not_before_cid(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _not_before: u64,
    _current_time: u64,
) -> u64 {
    todo!()
}

pub fn renew_path_connection_id(_cnx: &mut cnx_t, _path_x: &mut path_t) -> i32 {
    todo!()
}

// ---------------------------------------------------------------------------
// Retransmission queue management.

pub fn queue_for_retransmit(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _packet: &mut packet_t,
    _length: usize,
    _current_time: u64,
) {
    todo!()
}

pub fn dequeue_retransmit_packet(
    _cnx: &mut cnx_t,
    _pkt_ctx: &mut packet_context_t,
    _p: *mut packet_t,
    _should_free: i32,
    _add_to_data_repeat_queue: i32,
) -> *mut packet_t {
    todo!()
}

pub fn dequeue_retransmitted_packet(
    _cnx: &mut cnx_t,
    _pkt_ctx: &mut packet_context_t,
    _p: *mut packet_t,
) {
    todo!()
}

pub fn reset_cnx(_cnx: &mut cnx_t, _current_time: u64) -> i32 {
    todo!()
}

pub fn reset_packet_context(_cnx: &mut cnx_t, _pkt_ctx: &mut packet_context_t) {
    todo!()
}

pub fn connection_error(_cnx: &mut cnx_t, _local_error: u64, _frame_type: u64) -> i32 {
    todo!()
}

pub fn connection_error_ex(
    _cnx: &mut cnx_t,
    _local_error: u64,
    _frame_type: u64,
    _local_reason: Option<&str>,
) -> i32 {
    todo!()
}

pub fn connection_disconnect(_cnx: &mut cnx_t) {
    todo!()
}

// ---------------------------------------------------------------------------
// Connection lookup.

pub fn cnx_by_id(
    _quic: &mut quic_t,
    _cnx_id: connection_id_t,
    _l_cid_sequence: &mut *mut local_cnxid_t,
) -> *mut cnx_t {
    todo!()
}

pub fn cnx_by_net(_quic: &mut quic_t, _addr: Option<&SocketAddr>) -> *mut cnx_t {
    todo!()
}

pub fn cnx_by_icid(
    _quic: &mut quic_t,
    _icid: &connection_id_t,
    _addr: Option<&SocketAddr>,
) -> *mut cnx_t {
    todo!()
}

pub fn cnx_by_secret(
    _quic: &mut quic_t,
    _reset_secret: &[u8],
    _addr: Option<&SocketAddr>,
) -> *mut cnx_t {
    todo!()
}

// ---------------------------------------------------------------------------
// Pacing.

pub fn pacing_init(_pacing: &mut pacing_t, _current_time: u64) {
    todo!()
}

pub fn is_pacing_blocked(_pacing: &mut pacing_t) -> i32 {
    todo!()
}

pub fn is_authorized_by_pacing(
    _pacing: &mut pacing_t,
    _current_time: u64,
    _next_time: &mut u64,
    _packet_train_mode: bool,
    _quic: &mut quic_t,
) -> i32 {
    todo!()
}

pub fn update_pacing_parameters(
    _pacing: &mut pacing_t,
    _pacing_rate: f64,
    _quantum: u64,
    _send_mtu: usize,
    _smoothed_rtt: u64,
    _signalled_path: *mut path_t,
) {
    todo!()
}

pub fn update_pacing_window(
    _pacing: &mut pacing_t,
    _slow_start: i32,
    _cwin: u64,
    _send_mtu: usize,
    _smoothed_rtt: u64,
    _signalled_path: *mut path_t,
) {
    todo!()
}

pub fn update_pacing_data_after_send(
    _pacing: &mut pacing_t,
    _length: usize,
    _send_mtu: usize,
    _current_time: u64,
) {
    todo!()
}

pub fn update_pacing_data(_path_x: &mut path_t, _slow_start: i32) {
    todo!()
}

pub fn update_pacing_after_send(_path_x: &mut path_t, _length: usize, _current_time: u64) {
    todo!()
}

pub fn is_sending_authorized_by_pacing(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _current_time: u64,
    _next_time: &mut u64,
) -> i32 {
    todo!()
}

pub fn update_pacing_rate(_path_x: &mut path_t, _pacing_rate: f64, _quantum: u64) {
    todo!()
}

pub fn refresh_path_quality_thresholds(_path_x: &mut path_t) {
    todo!()
}

pub fn issue_path_quality_update(_cnx: &mut cnx_t, _path_x: &mut path_t) -> i32 {
    todo!()
}

pub fn reinsert_by_wake_time(_quic: &mut quic_t, _cnx: &mut cnx_t, _next_time: u64) {
    todo!()
}

// ---------------------------------------------------------------------------
// Integer parsing / formatting helpers (translated from `PARSE_*` and
// `format_*`).

#[inline]
pub const fn PARSE_16(b: &[u8]) -> u16 {
    ((b[0] as u16) << 8) | (b[1] as u16)
}

#[inline]
pub const fn PARSE_24(b: &[u8]) -> u32 {
    (PARSE_16(b) as u32) << 8 | (b[2] as u32)
}

#[inline]
pub const fn PARSE_32(b: &[u8]) -> u32 {
    ((PARSE_16(b) as u32) << 16) | PARSE_16(&[b[2], b[3]]) as u32
}

#[inline]
pub const fn PARSE_64(b: &[u8]) -> u64 {
    ((PARSE_32(b) as u64) << 32) | PARSE_32(&[b[4], b[5], b[6], b[7]]) as u64
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

pub fn frames_varint_decode(_bytes: &[u8], _bytes_max: *const u8, _n64: &mut u64) -> *const u8 {
    todo!()
}

pub fn frames_varint_skip(_bytes: &[u8], _bytes_max: *const u8) -> *const u8 {
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

pub fn parse_long_packet_type(_flags: u8, _version_index: i32) -> packet_type_enum {
    todo!()
}

pub fn parse_packet_header(
    _quic: &mut quic_t,
    _bytes: &[u8],
    _length: usize,
    _addr_from: Option<&SocketAddr>,
    _ph: &mut packet_header,
    _pcnx: &mut *mut cnx_t,
    _receiving: i32,
) -> i32 {
    todo!()
}

pub fn create_long_header(
    _packet_type: packet_type_enum,
    _dest_cnx_id: &connection_id_t,
    _srce_cnx_id: &connection_id_t,
    _do_grease_quic_bit: i32,
    _version: u32,
    _version_index: i32,
    _sequence_number: u64,
    _retry_token_length: usize,
    _retry_token: *mut u8,
    _bytes: &mut [u8],
    _pn_offset: &mut usize,
    _pn_length: &mut usize,
) -> usize {
    todo!()
}

pub fn create_packet_header(
    _cnx: &mut cnx_t,
    _packet_type: packet_type_enum,
    _sequence_number: u64,
    _path_x: &mut path_t,
    _tuple: &mut tuple_t,
    _header_length: usize,
    _bytes: &mut [u8],
    _pn_offset: &mut usize,
    _pn_length: &mut usize,
) -> usize {
    todo!()
}

pub fn predict_packet_header_length(
    _cnx: &mut cnx_t,
    _packet_type: packet_type_enum,
    _pkt_ctx: &mut packet_context_t,
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

pub fn get_checksum_length(_cnx: &mut cnx_t, _is_cleartext_mode: epoch_enum) -> usize {
    todo!()
}

pub fn protect_packet_header(
    _send_buffer: &mut [u8],
    _pn_offset: usize,
    _first_mask: u8,
    _pn_enc: *mut c_void,
) {
    todo!()
}

pub fn protect_packet(
    _cnx: &mut cnx_t,
    _ptype: packet_type_enum,
    _bytes: &mut [u8],
    _sequence_number: u64,
    _length: usize,
    _header_length: usize,
    _send_buffer: &mut [u8],
    _send_buffer_max: usize,
    _aead_context: *mut c_void,
    _pn_enc: *mut c_void,
    _path_x: &mut path_t,
    _tuple: &mut tuple_t,
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
    _ph: &mut packet_header,
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
    _cnx: &mut cnx_t,
    _packet: &mut packet_t,
    _ret: i32,
    _length: usize,
    _header_length: usize,
    _checksum_overhead: usize,
    _send_length: &mut usize,
    _send_buffer: &mut [u8],
    _send_buffer_max: usize,
    _path_x: &mut path_t,
    _current_time: u64,
    _tuple: &mut tuple_t,
) {
    todo!()
}

pub fn finalize_and_protect_packet(
    _cnx: &mut cnx_t,
    _packet: &mut packet_t,
    _ret: i32,
    _length: usize,
    _header_length: usize,
    _checksum_overhead: usize,
    _send_length: &mut usize,
    _send_buffer: &mut [u8],
    _send_buffer_max: usize,
    _path_x: &mut path_t,
    _current_time: u64,
) {
    todo!()
}

pub fn implicit_handshake_ack(_cnx: &mut cnx_t, _pc: packet_context_enum, _current_time: u64) {
    todo!()
}

pub fn false_start_transition(_cnx: &mut cnx_t, _current_time: u64) {
    todo!()
}

pub fn client_almost_ready_transition(_cnx: &mut cnx_t) {
    todo!()
}

pub fn ready_state_transition(_cnx: &mut cnx_t, _current_time: u64) {
    todo!()
}

pub fn parse_header_and_decrypt(
    _quic: &mut quic_t,
    _bytes: &[u8],
    _length: usize,
    _packet_length: usize,
    _addr_from: Option<&SocketAddr>,
    _current_time: u64,
    _decrypted_data: &mut stream_data_node_t,
    _ph: &mut packet_header,
    _pcnx: &mut *mut cnx_t,
    _consumed: &mut usize,
    _new_context_created: &mut i32,
) -> i32 {
    todo!()
}

// ---------------------------------------------------------------------------
// Packet number / ACK shortcuts.

pub fn get_sequence_number(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _pc: packet_context_enum,
) -> u64 {
    todo!()
}

pub fn get_ack_number(_cnx: &mut cnx_t, _path_x: &mut path_t, _pc: packet_context_enum) -> u64 {
    todo!()
}

pub fn get_last_packet(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _pc: packet_context_enum,
) -> *mut packet_t {
    todo!()
}

// ---------------------------------------------------------------------------
// ACK logic.

pub fn init_ack_ctx(_cnx: &mut cnx_t, _ack_ctx: &mut ack_context_t) {
    todo!()
}

pub fn is_ack_needed(
    _cnx: &mut cnx_t,
    _current_time: u64,
    _next_wake_time: &mut u64,
    _pc: packet_context_enum,
    _is_opportunistic: i32,
) -> i32 {
    todo!()
}

pub fn is_pn_already_received(
    _cnx: &mut cnx_t,
    _pc: packet_context_enum,
    _l_cid: *mut local_cnxid_t,
    _pn64: u64,
) -> i32 {
    todo!()
}

pub fn record_pn_received(
    _cnx: &mut cnx_t,
    _pc: packet_context_enum,
    _l_cid: *mut local_cnxid_t,
    _pn64: u64,
    _current_microsec: u64,
) -> i32 {
    todo!()
}

pub fn sack_select_ack_ranges(
    _sack_list: &mut sack_list_t,
    _first_sack: *mut sack_item_t,
    _max_ranges: i32,
    _is_opportunistic: i32,
    _nb_sent_max: &mut i32,
    _nb_sent_max_skip: &mut i32,
) {
    todo!()
}

pub fn update_sack_list(
    _sack: &mut sack_list_t,
    _pn64_min: u64,
    _pn64_max: u64,
    _current_time: u64,
) -> i32 {
    todo!()
}

pub fn check_sack_list(_sack: &mut sack_list_t, _pn64_min: u64, _pn64_max: u64) -> i32 {
    todo!()
}

pub fn process_ack_of_ack_range(
    _first_sack: &mut sack_list_t,
    _previous: *mut sack_item_t,
    _start_of_range: u64,
    _end_of_range: u64,
) -> *mut sack_item_t {
    todo!()
}

pub fn update_ack_horizon(_sack_list: &mut sack_list_t, _current_time: u64) {
    todo!()
}

pub fn sack_first_item(_sack_list: &mut sack_list_t) -> *mut sack_item_t {
    todo!()
}

pub fn sack_last_item(_sack_list: &mut sack_list_t) -> *mut sack_item_t {
    todo!()
}

pub fn sack_next_item(_sack: *mut sack_item_t) -> *mut sack_item_t {
    todo!()
}

pub fn sack_previous_item(_sack: *mut sack_item_t) -> *mut sack_item_t {
    todo!()
}

pub fn sack_insert_item(
    _sack_list: &mut sack_list_t,
    _range_min: u64,
    _range_max: u64,
    _current_time: u64,
) -> i32 {
    todo!()
}

pub fn sack_list_is_empty(_sack_list: &mut sack_list_t) -> i32 {
    todo!()
}

pub fn ack_ctx_from_cnx_context(
    _cnx: &mut cnx_t,
    _pc: packet_context_enum,
    _l_cid: *mut local_cnxid_t,
) -> *mut ack_context_t {
    todo!()
}

pub fn sack_list_from_cnx_context(
    _cnx: &mut cnx_t,
    _pc: packet_context_enum,
    _l_cid: *mut local_cnxid_t,
) -> *mut sack_list_t {
    todo!()
}

pub fn sack_list_first(_first_sack: &mut sack_list_t) -> u64 {
    todo!()
}

pub fn sack_list_last(_first_sack: &mut sack_list_t) -> u64 {
    todo!()
}

pub fn sack_list_first_range(_first_sack: &mut sack_list_t) -> *mut sack_item_t {
    todo!()
}

pub fn sack_list_init(_first_sack: &mut sack_list_t) {
    todo!()
}

pub fn sack_list_reset(
    _first_sack: &mut sack_list_t,
    _range_min: u64,
    _range_max: u64,
    _current_time: u64,
) -> i32 {
    todo!()
}

pub fn sack_list_free(_first_sack: &mut sack_list_t) {
    todo!()
}

pub fn sack_item_range_start(_sack_item: *mut sack_item_t) -> u64 {
    todo!()
}

pub fn sack_item_range_end(_sack_item: *mut sack_item_t) -> u64 {
    todo!()
}

pub fn sack_item_nb_times_sent(_sack_item: *mut sack_item_t, _is_opportunistic: i32) -> i32 {
    todo!()
}

pub fn sack_item_record_sent(
    _sack_list: &mut sack_list_t,
    _sack_item: *mut sack_item_t,
    _is_opportunistic: i32,
) {
    todo!()
}

pub fn sack_item_record_reset(_sack_list: &mut sack_list_t, _sack_item: *mut sack_item_t) {
    todo!()
}

pub fn sack_list_size(_first_sack: &mut sack_list_t) -> usize {
    todo!()
}

pub fn record_ack_packet_data(_packet_data: &mut packet_data_t, _acked_packet: &mut packet_t) {
    todo!()
}

pub fn init_packet_ctx(
    _cnx: &mut cnx_t,
    _pkt_ctx: &mut packet_context_t,
    _pc: packet_context_enum,
) {
    todo!()
}

pub fn process_ack_of_ack_frame(
    _first_sack: &mut sack_list_t,
    _bytes: &mut [u8],
    _bytes_max: usize,
    _consumed: &mut usize,
    _is_ecn: i32,
) -> i32 {
    todo!()
}

pub fn compute_ack_gap_and_delay(
    _cnx: &mut cnx_t,
    _rtt: u64,
    _remote_min_ack_delay: u64,
    _data_rate: u64,
    _ack_gap: &mut u64,
    _ack_delay_max: &mut u64,
) {
    todo!()
}

pub fn seed_bandwidth(
    _cnx: &mut cnx_t,
    _rtt_min: u64,
    _cwin: u64,
    _ip_addr: &[u8],
    _ip_addr_length: u8,
) {
    todo!()
}

pub fn current_retransmit_timer(_cnx: &mut cnx_t, _path_x: &mut path_t) -> u64 {
    todo!()
}

pub fn update_path_rtt(
    _cnx: &mut cnx_t,
    _old_path: &mut path_t,
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

pub fn create_stream(_cnx: &mut cnx_t, _stream_id: u64) -> *mut stream_head_t {
    todo!()
}

pub fn create_missing_streams(
    _cnx: &mut cnx_t,
    _stream_id: u64,
    _is_remote: i32,
) -> *mut stream_head_t {
    todo!()
}

pub fn is_stream_closed(_stream: &mut stream_head_t, _client_mode: i32) -> i32 {
    todo!()
}

pub fn delete_stream_if_closed(_cnx: &mut cnx_t, _stream: &mut stream_head_t) -> i32 {
    todo!()
}

pub fn update_stream_initial_remote(_cnx: &mut cnx_t) {
    todo!()
}

pub fn stream_from_node(_node: *mut splay_node_t) -> *mut stream_head_t {
    todo!()
}

pub fn insert_output_stream(_cnx: &mut cnx_t, _stream: &mut stream_head_t) {
    todo!()
}

pub fn remove_output_stream(_cnx: &mut cnx_t, _stream: &mut stream_head_t) {
    todo!()
}

pub fn reorder_output_stream(_cnx: &mut cnx_t, _stream: &mut stream_head_t) {
    todo!()
}

pub fn first_stream(_cnx: &mut cnx_t) -> *mut stream_head_t {
    todo!()
}

pub fn last_stream(_cnx: &mut cnx_t) -> *mut stream_head_t {
    todo!()
}

pub fn next_stream(_stream: *mut stream_head_t) -> *mut stream_head_t {
    todo!()
}

pub fn find_stream(_cnx: &mut cnx_t, _stream_id: u64) -> *mut stream_head_t {
    todo!()
}

pub fn add_output_streams(_cnx: &mut cnx_t, _old_limit: u64, _new_limit: u64, _is_bidir: bool) {
    todo!()
}

pub fn find_ready_stream_path(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _is_coalesced: i32,
) -> *mut stream_head_t {
    todo!()
}

pub fn find_ready_stream(_cnx: &mut cnx_t) -> *mut stream_head_t {
    todo!()
}

pub fn is_tls_stream_ready(_cnx: &mut cnx_t) -> i32 {
    todo!()
}

pub fn decode_stream_frame(
    _cnx: &mut cnx_t,
    _bytes: *const u8,
    _bytes_max: *const u8,
    _received_data: &mut stream_data_node_t,
    _current_time: u64,
) -> *const u8 {
    todo!()
}

pub fn format_stream_frame(
    _cnx: &mut cnx_t,
    _stream: &mut stream_head_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _is_still_active: &mut i32,
    _ret: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn update_max_stream_ID_local(_cnx: &mut cnx_t, _stream: &mut stream_head_t) {
    todo!()
}

// ---------------------------------------------------------------------------
// Frame retransmission.

pub fn check_frame_needs_repeat(
    _cnx: &mut cnx_t,
    _bytes: &[u8],
    _bytes_max: usize,
    _p_type: packet_type_enum,
    _no_need_to_repeat: &mut i32,
    _do_not_detect_spurious: &mut i32,
    _is_preemptive_needed: &mut i32,
) -> i32 {
    todo!()
}

pub fn format_available_stream_frames(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _bytes_next: *mut u8,
    _bytes_max: *mut u8,
    _current_priority: u64,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _stream_tried_and_failed: &mut i32,
    _ret: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn queue_data_repeat_init(_cnx: &mut cnx_t) {
    todo!()
}

pub fn queue_data_repeat_packet(_cnx: &mut cnx_t, _packet: &mut packet_t) {
    todo!()
}

pub fn dequeue_data_repeat_packet(_cnx: &mut cnx_t, _packet: &mut packet_t) {
    todo!()
}

pub fn first_data_repeat_packet(_cnx: &mut cnx_t) -> *mut packet_t {
    todo!()
}

pub fn copy_stream_frame_for_retransmit(
    _cnx: &mut cnx_t,
    _packet: &mut packet_t,
    _bytes_next: *mut u8,
    _bytes_max: *mut u8,
) -> *mut u8 {
    todo!()
}

pub fn copy_stream_frames_for_retransmit(
    _cnx: &mut cnx_t,
    _bytes_next: *mut u8,
    _bytes_max: *mut u8,
    _current_priority: u64,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn copy_before_retransmit(
    _old_p: &mut packet_t,
    _cnx: &mut cnx_t,
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
    _cnx: &mut cnx_t,
    _pc: packet_context_enum,
    _path_x: &mut path_t,
    _current_time: u64,
    _next_wake_time: &mut u64,
    _packet: &mut packet_t,
    _send_buffer_max: usize,
    _header_length: &mut usize,
) -> i32 {
    todo!()
}

pub fn set_ack_needed(
    _cnx: &mut cnx_t,
    _current_time: u64,
    _pc: packet_context_enum,
    _path_x: &mut path_t,
    _is_immediate_ack_required: i32,
) {
    todo!()
}

pub fn process_ack_of_frames(_cnx: &mut cnx_t, _p: &mut packet_t, _is_spurious: i32) {
    todo!()
}

// ---------------------------------------------------------------------------
// Stream data buffer (callback argument for "prepare to send").

pub struct stream_data_buffer_argument_t {
    pub bytes: *mut u8,
    pub byte_index: usize,
    pub byte_space: usize,
    pub allowed_space: usize,
    pub length: usize,
    pub is_fin: i32,
    pub is_still_active: i32,
    pub app_buffer: *mut u8,
}

pub fn is_stream_frame_unlimited(_bytes: &[u8]) -> i32 {
    todo!()
}

pub fn format_stream_frame_header(
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _stream_id: u64,
    _offset: u64,
) -> *mut u8 {
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

pub fn decode_crypto_hs_frame(
    _cnx: &mut cnx_t,
    _bytes: *const u8,
    _bytes_max: *const u8,
    _received_data: &mut stream_data_node_t,
    _epoch: i32,
) -> *const u8 {
    todo!()
}

pub fn format_crypto_hs_frame(
    _stream: &mut stream_head_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn format_ack_frame(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _current_time: u64,
    _pc: packet_context_enum,
    _is_opportunistic: i32,
) -> *mut u8 {
    todo!()
}

pub fn format_connection_close_frame(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn format_application_close_frame(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn format_required_max_stream_data_frames(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn format_max_data_frame(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _maxdata_increase: u64,
) -> *mut u8 {
    todo!()
}

pub fn format_max_stream_data_frame(
    _cnx: &mut cnx_t,
    _stream: &mut stream_head_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _new_max_data: u64,
) -> *mut u8 {
    todo!()
}

pub fn cc_increased_window(_cnx: &mut cnx_t, _previous_window: u64) -> u64 {
    todo!()
}

pub fn format_max_streams_frame_if_needed(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn stream_data_node_recycle(_stream_data: &mut stream_data_node_t) {
    todo!()
}

pub fn stream_data_node_alloc(_quic: &mut quic_t) -> *mut stream_data_node_t {
    todo!()
}

pub fn clear_stream(_stream: &mut stream_head_t) {
    todo!()
}

pub fn delete_stream(_cnx: &mut cnx_t, _stream: &mut stream_head_t) {
    todo!()
}

pub fn find_or_create_local_cnxid_list(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _do_create: i32,
) -> *mut local_cnxid_list_t {
    todo!()
}

pub fn create_local_cnxid(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _suggested_value: *const connection_id_t,
    _current_time: u64,
) -> *mut local_cnxid_t {
    todo!()
}

pub fn demote_local_cnxid_list(_cnx: &mut cnx_t, _unique_path_id: u64, _reason: u64) -> i32 {
    todo!()
}

pub fn delete_local_cnxid(_cnx: &mut cnx_t, _l_cid: *mut local_cnxid_t) {
    todo!()
}

pub fn delete_local_cnxid_list(_cnx: &mut cnx_t, _local_cnxid_list: *mut local_cnxid_list_t) {
    todo!()
}

pub fn delete_local_cnxid_lists(_cnx: &mut cnx_t) {
    todo!()
}

pub fn retire_local_cnxid(_cnx: &mut cnx_t, _unique_path_id: u64, _sequence: u64) {
    todo!()
}

pub fn check_local_cnxid_ttl(
    _cnx: &mut cnx_t,
    _local_cnxid_list: &mut local_cnxid_list_t,
    _current_time: u64,
    _next_wake_time: &mut u64,
) {
    todo!()
}

pub fn find_local_cnxid(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _cnxid: &connection_id_t,
) -> *mut local_cnxid_t {
    todo!()
}

pub fn format_path_challenge_frame(
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _challenge: u64,
) -> *mut u8 {
    todo!()
}

pub fn format_path_response_frame(
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _challenge: u64,
) -> *mut u8 {
    todo!()
}

pub fn should_repeat_path_response_frame(
    _cnx: &mut cnx_t,
    _bytes: &[u8],
    _bytes_max: usize,
) -> i32 {
    todo!()
}

pub fn format_new_connection_id_frame(
    _cnx: &mut cnx_t,
    _local_cnxid_list: &mut local_cnxid_list_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _l_cid: *mut local_cnxid_t,
) -> *mut u8 {
    todo!()
}

pub fn format_max_path_id_frame(
    _bytes: *mut u8,
    _bytes_max: *const u8,
    _max_path_id: u64,
    _more_data: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn format_blocked_frames(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn queue_retire_connection_id_frame(
    _cnx: &mut cnx_t,
    _unique_path_id: u64,
    _sequence: u64,
) -> i32 {
    todo!()
}

pub fn queue_new_token_frame(_cnx: &mut cnx_t, _token: *mut u8, _token_length: usize) -> i32 {
    todo!()
}

pub fn format_one_blocked_frame(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _stream: &mut stream_head_t,
) -> *mut u8 {
    todo!()
}

pub fn format_first_misc_or_dg_frame(
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _misc_frame: *mut misc_frame_header_t,
    _first: &mut *mut misc_frame_header_t,
    _last: &mut *mut misc_frame_header_t,
) -> *mut u8 {
    todo!()
}

pub fn find_first_misc_frame(
    _cnx: &mut cnx_t,
    _pc: packet_context_enum,
) -> *mut misc_frame_header_t {
    todo!()
}

pub fn format_misc_frames_in_context(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _pc: packet_context_enum,
) -> *mut u8 {
    todo!()
}

pub fn queue_misc_or_dg_frame(
    _cnx: &mut cnx_t,
    _first: &mut *mut misc_frame_header_t,
    _last: &mut *mut misc_frame_header_t,
    _bytes: &[u8],
    _length: usize,
    _is_pure_ack: i32,
    _pc: packet_context_enum,
) -> i32 {
    todo!()
}

pub fn purge_misc_frames_after_ready(_cnx: &mut cnx_t) {
    todo!()
}

pub fn delete_misc_or_dg(
    _first: &mut *mut misc_frame_header_t,
    _last: &mut *mut misc_frame_header_t,
    _frame: *mut misc_frame_header_t,
) {
    todo!()
}

pub fn clear_ack_ctx(_ack_ctx: &mut ack_context_t) {
    todo!()
}

pub fn reset_ack_context(_ack_ctx: &mut ack_context_t) {
    todo!()
}

pub fn queue_handshake_done_frame(_cnx: &mut cnx_t) -> i32 {
    todo!()
}

pub fn format_first_datagram_frame(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _is_first_in_packet: i32,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn format_ready_datagram_frame(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
    _ret: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn decode_datagram_frame_header(
    _bytes: *const u8,
    _bytes_max: *const u8,
    _frame_id: &mut u8,
    _length: &mut u64,
) -> *const u8 {
    todo!()
}

pub fn parse_ack_frequency_frame(
    _bytes: *const u8,
    _bytes_max: *const u8,
    _seq: &mut u64,
    _packets: &mut u64,
    _microsec: &mut u64,
    _ignore_order: &mut u8,
    _reordering_threshold: &mut u64,
) -> *const u8 {
    todo!()
}

pub fn format_ack_frequency_frame(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn format_immediate_ack_frame(
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn format_time_stamp_frame(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _current_time: u64,
) -> *mut u8 {
    todo!()
}

pub fn encode_time_stamp_length(_cnx: &mut cnx_t, _current_time: u64) -> usize {
    todo!()
}

pub fn format_bdp_frame(
    _cnx: &mut cnx_t,
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _path_x: &mut path_t,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn format_path_abandon_frame(
    _bytes: *mut u8,
    _bytes_max: *mut u8,
    _more_data: &mut i32,
    _path_id: u64,
    _reason: u64,
) -> *mut u8 {
    todo!()
}

pub fn queue_path_abandon_frame(_cnx: &mut cnx_t, _unique_path_id: u64, _reason: u64) -> i32 {
    todo!()
}

pub fn decode_frames(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _bytes: &[u8],
    _bytes_max: usize,
    _received_data: &mut stream_data_node_t,
    _epoch: i32,
    _addr_from: Option<&SocketAddr>,
    _addr_to: Option<&SocketAddr>,
    _pn64: u64,
    _path_is_not_allocated: i32,
    _current_time: u64,
) -> i32 {
    todo!()
}

pub fn parse_observed_address_frame(
    _bytes: *const u8,
    _bytes_max: *const u8,
    _ftype: u64,
    _sequence: &mut u64,
    _addr: &mut *const u8,
    _port: &mut u16,
) -> *const u8 {
    todo!()
}

pub fn format_observed_address_frame(
    _bytes: *mut u8,
    _bytes_max: *const u8,
    _ftype: u64,
    _sequence_number: u64,
    _addr: *mut u8,
    _port: u16,
    _more_data: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn prepare_observed_address_frame(
    _bytes: *mut u8,
    _bytes_max: *const u8,
    _path_x: &mut path_t,
    _tuple: &mut tuple_t,
    _current_time: u64,
    _next_wake_time: &mut u64,
    _more_data: &mut i32,
    _is_pure_ack: &mut i32,
) -> *mut u8 {
    todo!()
}

pub fn update_peer_addr(_path_x: &mut path_t, _peer_addr: Option<&SocketAddr>) {
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

pub fn skip_path_abandon_frame(_bytes: *const u8, _bytes_max: *const u8) -> *const u8 {
    todo!()
}

pub fn skip_path_available_or_backup_frame(_bytes: *const u8, _bytes_max: *const u8) -> *const u8 {
    todo!()
}

pub fn is_path_challenging_packet(_bytes: &[u8], _bytes_maxsize: usize) -> i32 {
    todo!()
}

pub fn queue_path_available_or_backup_frame(
    _cnx: &mut cnx_t,
    _path_x: &mut path_t,
    _status: path_status_enum,
) -> i32 {
    todo!()
}

pub fn test_and_signal_new_path_allowed(_cnx: &mut cnx_t) {
    todo!()
}

pub fn decode_closing_frames(
    _bytes: &mut [u8],
    _bytes_max: usize,
    _closing_received: &mut i32,
) -> i32 {
    todo!()
}

pub fn process_sooner_packets(_cnx: &mut cnx_t, _current_time: u64) {
    todo!()
}

pub fn delete_sooner_packets(_cnx: &mut cnx_t) {
    todo!()
}

// ---------------------------------------------------------------------------
// Transport extensions and version upgrade.

pub fn process_tp_version_negotiation(
    _bytes: *const u8,
    _bytes_max: *const u8,
    _extension_mode: i32,
    _envelop_vn: u32,
    _negotiated_vn: &mut u32,
    _negotiated_index: &mut i32,
    _vn_error: &mut u64,
) -> *const u8 {
    todo!()
}

pub fn prepare_transport_extensions(
    _cnx: &mut cnx_t,
    _extension_mode: i32,
    _bytes: &mut [u8],
    _bytes_max: usize,
    _consumed: &mut usize,
) -> i32 {
    todo!()
}

pub fn receive_transport_extensions(
    _cnx: &mut cnx_t,
    _extension_mode: i32,
    _bytes: &mut [u8],
    _bytes_max: usize,
    _consumed: &mut usize,
) -> i32 {
    todo!()
}

pub fn create_misc_frame(
    _bytes: &[u8],
    _length: usize,
    _is_pure_ack: i32,
    _pc: packet_context_enum,
) -> *mut misc_frame_header_t {
    todo!()
}

pub fn process_version_upgrade(
    _cnx: &mut cnx_t,
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
pub trait maskOps {
    /// C: `mask_intercept_fn`.
    fn intercept(
        &self,
        quic: &mut quic_t,
        mask_ctx: *mut c_void,
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
        mask_ctx: *mut c_void,
        bytes: &[u8],
        packet_length: usize,
        addr_from: Option<&SocketAddr>,
        consumed: &mut usize,
    ) -> i32;
}

#[cfg(test)]
mod test {}
