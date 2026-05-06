//! Tests for `picoquictest/parseheadertest.c`.
//!
//! Tests covered:
//! * [`header_length`]    — C `header_length_test`
//! * [`incoming_initial`] — C `incoming_initial_test`
//! * [`packet_enc_dec`]   — C `packet_enc_dec_test`
//! * [`parseheader`]      — C `parseheadertest`

#![allow(non_snake_case)]

use core::net::SocketAddr;

use crate::internal::{Epoch, PacketHeader, PacketType, Version, update_payload_length};
use crate::{ConnectionId, Instant, PacketContext, Quic, RESET_SECRET_SIZE};

use super::util;

// ---------------------------------------------------------------------------
// Shared test CIDs.

fn ini_id() -> ConnectionId {
    ConnectionId::clone_from_slice(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]).unwrap()
}
fn rem_id() -> ConnectionId {
    ConnectionId::clone_from_slice(&[0x04, 0x05, 0x06, 0x07]).unwrap()
}
fn local_id() -> ConnectionId {
    ConnectionId::clone_from_slice(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]).unwrap()
}
fn r10_id() -> ConnectionId {
    ConnectionId::clone_from_slice(&[0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09]).unwrap()
}

// ---------------------------------------------------------------------------
// Test packet bytes and expected headers (see C test_entries[]).

#[allow(dead_code)]
struct TestEntry {
    packet: &'static [u8],
    ph: ExpectedPh,
    decode_test_only: bool,
    local_cid_length: u8,
}

#[allow(dead_code)]
struct ExpectedPh {
    dest_connection_id: fn() -> ConnectionId,
    src_connection_id: fn() -> ConnectionId,
    pn_truncated: u32,
    version: u32,
    offset: usize,
    pn_offset: usize,
    packet_type: PacketType,
    packet_number_full: u64,
    payload_length: usize,
    epoch: Epoch,
    packet_context: PacketContext,
    key_phase: bool,
    spin: bool,
    has_loss_bits: bool,
}

// pinitial10
static PINITIAL10: &[u8] = &[
    0xC3, 0x50, 0x43, 0x51, 0x30, 0x08, // DCID len = 8
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x04, // SCID len = 4
    0x04, 0x05, 0x06, 0x07, 0x00, // token len = 0
    0x44, 0x00, // payload length = 0x400
    0xDE, 0xAD, 0xBE, 0xEF, // PN
];

// pinitial10_l (local CID in SCID)
static PINITIAL10_L: &[u8] = &[
    0xC3, 0x50, 0x43, 0x51, 0x30, 0x08, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x08, // SCID len = 8 (local CID)
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x44, 0x00, 0xDE, 0xAD, 0xBE, 0xEF,
];

// pvnego10
static PVNEGO10: &[u8] = &[
    0xFF, 0x00, 0x00, 0x00, 0x00, 0x08, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x04, 0x04,
    0x05, 0x06, 0x07, 0x50, 0x43, 0x51, 0x30, 0xFF, 0x00, 0x00, 0x07,
];

// pvnegobis10 (first byte 0xAA instead of 0xFF)
static PVNEGOBIS10: &[u8] = &[
    0xAA, 0x00, 0x00, 0x00, 0x00, 0x08, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x04, 0x04,
    0x05, 0x06, 0x07, 0x50, 0x43, 0x51, 0x30, 0xFF, 0x00, 0x00, 0x07,
];

// phandshake
static PHANDSHAKE: &[u8] = &[
    0xE3, 0x50, 0x43, 0x51, 0x30, 0x08, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x04, 0x04,
    0x05, 0x06, 0x07, 0x44, 0x00, 0xDE, 0xAD, 0xBE, 0xEF,
];

// packet_short_phi0_c_32
static PACKET_SHORT_PHI0_C_32: &[u8] = &[
    0x43, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0xDE, 0xAD, 0xBE, 0xEF,
];

// packet_short_phi0_c_32_spin
static PACKET_SHORT_PHI0_C_32_SPIN: &[u8] = &[
    0x63, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0xDE, 0xAD, 0xBE, 0xEF,
];

// packet_short_phi1_noc_32
static PACKET_SHORT_PHI1_NOC_32: &[u8] = &[0x47, 0xDE, 0xAD, 0xBE, 0xEF];

// packet_intel_bug (640 bytes)
static PACKET_INTEL_BUG: &[u8] = &[
    0xc4, 0x00, 0x00, 0x00, 0x01, 0x08, 0xbb, 0xba, 0xda, 0x0e, 0xf9, 0x26, 0x00, 0xc8, 0x00, 0x00,
    0x44, 0x9e, 0x19, 0x55, 0xc0, 0x25, 0x6e, 0x96, 0xd8, 0x1d, 0x8d, 0x85, 0xed, 0xe9, 0x3e, 0x4b,
    0x01, 0xfa, 0x6b, 0xc1, 0xf3, 0x5a, 0x67, 0xf3, 0xbf, 0xc5, 0x92, 0x21, 0xc4, 0xce, 0x1d, 0x46,
    0x63, 0x5c, 0x36, 0xff, 0x59, 0x15, 0x06, 0xd0, 0x8f, 0xe1, 0xd6, 0x7c, 0x15, 0x9d, 0x7e, 0xe9,
    0x20, 0xed, 0xca, 0x35, 0x83, 0x7c, 0x22, 0xa7, 0xd6, 0x5b, 0x2e, 0x5b, 0x52, 0x9d, 0xe3, 0xf2,
    0x7f, 0x72, 0xc4, 0x57, 0xc5, 0xc3, 0x3c, 0x09, 0x03, 0x47, 0x95, 0xe7, 0x12, 0x59, 0x9c, 0xa5,
    0x0b, 0xeb, 0xd6, 0x7c, 0x1a, 0xf4, 0xfa, 0x58, 0x67, 0x3f, 0xb2, 0x2e, 0x8a, 0xde, 0xce, 0x28,
    0xea, 0x1f, 0x44, 0x4d, 0x4e, 0xda, 0xfb, 0x98, 0xf8, 0x3a, 0x1d, 0x7c, 0x02, 0x05, 0x70, 0xfe,
    0xc5, 0x97, 0x97, 0x9d, 0x7e, 0x7f, 0xea, 0x9a, 0x03, 0x5e, 0x6a, 0x7b, 0xa6, 0x2c, 0x16, 0xd7,
    0xf9, 0xef, 0x98, 0x75, 0x05, 0xaa, 0xf6, 0x9e, 0xf7, 0x43, 0x4b, 0xd9, 0x0e, 0xd0, 0x5a, 0x6d,
    0x7e, 0x0c, 0xe2, 0xb9, 0x7c, 0x48, 0x3d, 0x00, 0xe8, 0xc6, 0x3f, 0x93, 0xb7, 0xc2, 0x44, 0xbf,
    0x44, 0xc3, 0x2c, 0x51, 0xef, 0x99, 0xaa, 0x10, 0x25, 0x42, 0xa3, 0x53, 0x88, 0xa7, 0x86, 0x39,
    0x7f, 0x1f, 0x62, 0x6c, 0x31, 0xb7, 0xca, 0xa9, 0x6a, 0x8b, 0x44, 0x31, 0x58, 0x2c, 0x20, 0x6c,
    0x94, 0xa9, 0x6b, 0x4e, 0x45, 0x38, 0xfb, 0xc0, 0x96, 0xbd, 0x06, 0x52, 0x71, 0x5a, 0x05, 0x88,
    0x3c, 0x96, 0x6f, 0x72, 0x79, 0x08, 0x05, 0xd9, 0xab, 0xb7, 0xcd, 0xe6, 0x70, 0xf7, 0x95, 0xd1,
    0x5d, 0x9b, 0x86, 0xc6, 0xc0, 0x4b, 0x89, 0x47, 0x15, 0x30, 0xb2, 0x6b, 0xab, 0x38, 0x6b, 0x60,
    0x6c, 0x19, 0x4d, 0xaa, 0x28, 0x6d, 0xf7, 0xf9, 0x17, 0x4a, 0xc7, 0x29, 0xbd, 0x85, 0x33, 0xc3,
    0xce, 0x38, 0x2f, 0x0a, 0x0f, 0x11, 0x9f, 0x60, 0x3d, 0xbd, 0x33, 0x0c, 0xcc, 0xf2, 0x9b, 0x3c,
    0x88, 0x77, 0x43, 0x20, 0x6c, 0xe5, 0xcb, 0x81, 0x4a, 0x50, 0x2d, 0x22, 0x00, 0x9c, 0xa1, 0x75,
    0xa8, 0xc9, 0x86, 0x5a, 0x2c, 0xea, 0x90, 0x90, 0xeb, 0x70, 0x0a, 0x82, 0x90, 0x4c, 0xd4, 0x12,
    0x2a, 0x97, 0x21, 0x1b, 0x7e, 0x46, 0x56, 0x5e, 0x5f, 0x0f, 0x23, 0xf0, 0x50, 0x18, 0x11, 0xcf,
    0xaf, 0x7b, 0x80, 0xe8, 0x31, 0x1f, 0x0c, 0x13, 0x66, 0x97, 0x1b, 0x41, 0x79, 0x53, 0x53, 0xd9,
    0xb9, 0xba, 0xcc, 0x0f, 0xcd, 0x31, 0x76, 0x7e, 0x48, 0x09, 0x1f, 0x94, 0xa2, 0x4e, 0x59, 0xe0,
    0x91, 0x40, 0xf2, 0x01, 0x97, 0x08, 0x07, 0xcf, 0x77, 0x3c, 0x3a, 0x7c, 0x7a, 0xf2, 0x12, 0xda,
    0x8a, 0x5c, 0xf3, 0xae, 0xf5, 0x4c, 0x2a, 0x92, 0x24, 0xe9, 0x06, 0x8d, 0x7b, 0x25, 0x69, 0xd0,
    0xa6, 0xd6, 0xf8, 0xa2, 0x1c, 0xcb, 0x2c, 0xa9, 0xa6, 0xc8, 0xa2, 0x95, 0xac, 0x07, 0x93, 0xf7,
    0x1b, 0x0e, 0x08, 0x5c, 0x8a, 0xd0, 0xeb, 0x59, 0x10, 0x5f, 0x15, 0xca, 0xed, 0x16, 0xf7, 0xb5,
    0x17, 0x1f, 0x42, 0x6e, 0x9b, 0xe6, 0x25, 0x38, 0xa2, 0xcd, 0xf0, 0x32, 0x56, 0x53, 0x48, 0xe9,
    0x77, 0xe2, 0xd1, 0xdf, 0x25, 0x03, 0xb0, 0x53, 0x72, 0xc8, 0x77, 0x85, 0x1f, 0xa1, 0x8f, 0x60,
    0x20, 0x42, 0xc7, 0xe1, 0x51, 0x57, 0x1e, 0x5f, 0xa7, 0xdc, 0xa4, 0xb7, 0xc1, 0xb8, 0x2e, 0x90,
    0x15, 0xea, 0x8b, 0x3c, 0x91, 0x44, 0x3a, 0x4d, 0x7d, 0x0c, 0xc2, 0x43, 0x54, 0x05, 0xfc, 0xff,
    0xbe, 0x66, 0x83, 0xfc, 0x5d, 0xf3, 0xf3, 0x92, 0xe7, 0xab, 0xf6, 0x6a, 0x5c, 0xb0, 0x93, 0x0a,
    0xe7, 0x81, 0x24, 0x5a, 0xed, 0x37, 0x69, 0xcf, 0x5a, 0xfe, 0x06, 0x12, 0xb7, 0x00, 0x0f, 0x56,
    0x71, 0xe5, 0xac, 0x54, 0x86, 0x3e, 0xfb, 0x48, 0x00, 0x84, 0x3b, 0x53, 0x10, 0xcd, 0x05, 0x92,
    0x03, 0xc0, 0x79, 0xc5, 0x59, 0x9f, 0xed, 0x3e, 0xa6, 0x7c, 0xfb, 0x60, 0xf3, 0xed, 0x7d, 0x1f,
    0x47, 0xa4, 0x38, 0x2f, 0x6d, 0xc5, 0x62, 0x26, 0x8d, 0x9e, 0x50, 0x38, 0xc6, 0x5c, 0x83, 0x22,
    0x19, 0xfb, 0x64, 0xff, 0x48, 0x55, 0x86, 0x84, 0xc8, 0x53, 0xaa, 0xaf, 0x7b, 0x9c, 0x6c, 0xb0,
    0xca, 0x60, 0xf2, 0xb8, 0x76, 0xeb, 0x68, 0x41, 0x94, 0x8c, 0x46, 0x55, 0x02, 0x16, 0xdb, 0xae,
    0xc7, 0x44, 0xd2, 0x37, 0xa0, 0xa3, 0x1d, 0x27, 0x37, 0xb8, 0xc7, 0x01, 0xe0, 0x21, 0x33, 0xf3,
    0xca, 0xe6, 0x8f, 0xb5, 0x49, 0x4b, 0x4a, 0xf5, 0x95, 0x54, 0x0b, 0xcb, 0x6b, 0x7f, 0xfa, 0x2a,
    0xc5, 0x11, 0xa8, 0x72, 0x2d, 0x58, 0x6c, 0x15, 0x11, 0xab, 0x8b, 0xd2, 0xb4, 0x59, 0x75, 0xa5,
    0xe8, 0x74, 0x9f, 0x58, 0x7a, 0x97, 0x57, 0x73, 0xb6, 0xb1, 0x34, 0x30, 0xfe, 0x6f, 0x70, 0x1d,
    0xf7, 0x0b, 0x67, 0x6b, 0x93, 0x6d, 0x9b, 0x5e, 0xc6, 0x64, 0xa3, 0xae, 0xe2, 0x54, 0x18, 0x34,
    0xda, 0xde, 0x09, 0xab, 0x2f, 0x64, 0xd4, 0xf2, 0xa0, 0x09, 0x81, 0xaa, 0x52, 0x2e, 0xba, 0x87,
    0xcf, 0x35, 0x36, 0x49, 0x57, 0x22, 0x95, 0x5b, 0x86, 0x2b, 0x07, 0xa0, 0x31, 0x08, 0xfe, 0x56,
    0x5f, 0x9a, 0x19, 0x63, 0xc6, 0x01, 0x63, 0x25, 0xc1, 0xf8, 0xd1, 0x1c, 0xbd, 0xd9, 0xf4, 0x5e,
    0x12, 0x79, 0xef, 0xaa, 0x1e, 0xca, 0xea, 0xb8, 0x25, 0xf1, 0x26, 0x12, 0x6a, 0x12, 0x44, 0xb7,
    0x1d, 0xcb, 0x45, 0xa6, 0x8c, 0xe6, 0x1e, 0x38, 0x77, 0x0b, 0x02, 0x03, 0x4d, 0xcc, 0x38, 0x17,
    0x58, 0x21, 0xd5, 0xd7, 0x00, 0x9c, 0x58, 0xea, 0xa7, 0x4f, 0xa6, 0xc0, 0xe7, 0x50, 0xc8, 0xdd,
    0xa9, 0x47, 0xf2, 0x56, 0x56, 0xaa, 0x9e, 0x91, 0x75, 0x61, 0xb0, 0x60, 0x1f, 0x2a, 0x2d, 0xd8,
    0x81, 0xcc, 0x22, 0x82, 0xc3, 0xf6, 0x14, 0xa1, 0xa4, 0xa5, 0x89, 0x89, 0xe1, 0xa3, 0x57, 0xe3,
    0xec, 0x38, 0xe4, 0x9a, 0x51, 0x00, 0xe7, 0xbf, 0x86, 0xe3, 0x46, 0x7d, 0x65, 0x81, 0xba, 0x40,
    0x54, 0xdc, 0xd8, 0xc3, 0x26, 0x86, 0xe3, 0x89, 0xb7, 0x05, 0x61, 0xd4, 0xa9, 0xed, 0x78, 0x26,
    0xd3, 0x8c, 0xa2, 0xc2, 0x5a, 0xd6, 0xc5, 0xc1, 0xb3, 0x47, 0x1c, 0xd8, 0x93, 0xa8, 0x02, 0xc6,
    0x87, 0xb2, 0x87, 0x60, 0x39, 0x63, 0xf6, 0x88, 0xc8, 0xf4, 0x62, 0xfc, 0x17, 0xc8, 0x0f, 0xbc,
    0x00, 0x8d, 0x98, 0x08, 0x6f, 0xb8, 0x9a, 0x88, 0x05, 0xb1, 0xd8, 0x55, 0x26, 0xd5, 0x14, 0xfb,
    0xef, 0x59, 0x29, 0x9e, 0x20, 0x87, 0x28, 0xcc, 0x29, 0x48, 0xd2, 0x95, 0x38, 0x66, 0xcb, 0xb2,
    0x92, 0x50, 0x84, 0xab, 0xd4, 0x6f, 0x91, 0x67, 0x70, 0x80, 0x2b, 0x5f, 0xab, 0x9a, 0xda, 0xad,
    0xe8, 0xd0, 0x62, 0x0f, 0xec, 0x34, 0x1a, 0x64, 0xc0, 0x5d, 0xe3, 0xa5, 0x2f, 0xef, 0x6f, 0x97,
    0x94, 0x43, 0xba, 0x69, 0x8c, 0x15, 0x73, 0x5c, 0x0b, 0x4c, 0x0f, 0xc1, 0x69, 0xcc, 0x11, 0x5c,
    0xcc, 0x43, 0x37, 0xff, 0x1a, 0x5d, 0xbf, 0x5c, 0xb1, 0x05, 0x2d, 0xee, 0x81, 0xf1, 0x22, 0x5c,
    0x82, 0xdd, 0xed, 0x65, 0x95, 0xbe, 0xa8, 0x8b, 0x64, 0xdb, 0xbb, 0x82, 0xf2, 0x01, 0xeb, 0xcb,
    0xb1, 0x31, 0x59, 0x2c, 0x1e, 0x53, 0xc7, 0x22, 0x5e, 0x1c, 0x82, 0xfd, 0x8f, 0xe2, 0x74, 0xc4,
    0x54, 0xf4, 0x3e, 0x72, 0xbe, 0x93, 0x1b, 0x17, 0x90, 0xc8, 0x61, 0xa5, 0xbb, 0x97, 0xf6, 0x85,
    0xf3, 0x88, 0x1b, 0xa2, 0xbf, 0xe3, 0x2b, 0x5c, 0x47, 0x28, 0xbd, 0x0f, 0xad, 0x27, 0x4f, 0xe2,
    0x89, 0x8d, 0x1b, 0xaa, 0x23, 0x5c, 0x03, 0xa3, 0xd8, 0x82, 0x5c, 0x6c, 0x2d, 0xfd, 0x76, 0xf4,
    0xe8, 0xdc, 0xc6, 0xdc, 0x1f, 0x32, 0x94, 0x9e, 0x9d, 0x6e, 0x28, 0xbd, 0x48, 0xa3, 0x18, 0x03,
    0x66, 0x61, 0x03, 0x9f, 0x40, 0x44, 0x69, 0xa0, 0x4e, 0x2d, 0xea, 0x96, 0xc3, 0xb1, 0x23, 0x11,
    0xdc, 0xf0, 0xe2, 0xde, 0x4e, 0xfd, 0x18, 0xbf, 0xdb, 0x86, 0x4c, 0xae, 0xd5, 0xaa, 0x17, 0x13,
    0x0c, 0x0a, 0xce, 0x61, 0xee, 0x9d, 0x75, 0xfa, 0xc9, 0x58, 0xa5, 0xa7, 0x14, 0x8c, 0x4e, 0x94,
    0xc2, 0xb0, 0xc8, 0x4e, 0x8d, 0x58, 0xc9, 0xe6, 0x2c, 0x9f, 0x37, 0x12, 0x6f, 0xd3, 0x68, 0x8c,
    0xbe, 0xdb, 0x83, 0x11, 0x14, 0xcd, 0x44, 0xeb, 0x84, 0xb3, 0xce, 0x36, 0x6e, 0xa1, 0x70, 0x42,
    0x7e, 0xb0, 0x90, 0x94, 0x56, 0x45, 0x06, 0x8d, 0x62, 0x76, 0x65, 0x59, 0xa9, 0x46, 0xef, 0xde,
    0xa3, 0xb8, 0x2f, 0x33, 0x01, 0x1d, 0xd7, 0x4a, 0xb9, 0x25, 0xae, 0xe9, 0x5e, 0x40, 0xf9, 0xf7,
    0x1e, 0x64, 0x40, 0xbe, 0x66, 0xbf, 0xb9, 0xfb, 0xe8, 0x25, 0x5b, 0x36, 0x3f, 0x05, 0x0a, 0x57,
];

// ---------------------------------------------------------------------------
// parseheader test

/// C: `parseheadertest` in `picoquictest/parseheadertest.c`.
#[test]
fn parseheader() {
    let current_time = Instant::from_ticks(0);
    let addr: SocketAddr = "10.0.0.2:4434".parse().unwrap();

    let mut quic = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("create quic");

    // Create connection with predictable CIDs.
    // C: picoquic_create_cnx(..., is_client=1) — client mode.
    quic.create_connection(
        ini_id(),
        rem_id(),
        Some(&addr),
        current_time,
        Version::InternalTest1 as u32,
        None,
        None,
        true,
    )
    .expect("create connection");

    // Replace default local CID with a predictable one
    {
        let cnx = quic.connection_ref_by_id(ini_id()).expect("lookup cnx");
        let orig = cnx
            .create_local_connection_id(0, None, current_time)
            .expect("create orig cid");
        cnx.delete_local_connection_id(orig);
        let new_tok = cnx
            .create_local_connection_id(0, Some(&local_id()), current_time)
            .expect("create local cid");
        cnx.set_path_tuple_local_cid(0, 0, new_tok);
    }

    struct Entry {
        packet: &'static [u8],
        dest: fn() -> ConnectionId,
        src: fn() -> ConnectionId,
        version: u32,
        offset: usize,
        pn_offset: usize,
        payload_length: usize,
        ptype: PacketType,
        spin: bool,
        epoch: Epoch,
        pc: PacketContext,
        key_phase: bool,
        decode_only: bool,
        local_cid_len: u8,
    }
    let null = ConnectionId::default;
    let entries: &[Entry] = &[
        Entry {
            packet: PINITIAL10,
            dest: ini_id,
            src: rem_id,
            version: 0x50435130,
            offset: 22,
            pn_offset: 22,
            payload_length: 0x400,
            ptype: PacketType::Initial,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PINITIAL10_L,
            dest: ini_id,
            src: local_id,
            version: 0x50435130,
            offset: 26,
            pn_offset: 26,
            payload_length: 0x400,
            ptype: PacketType::Initial,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: false,
            local_cid_len: 8,
        },
        Entry {
            packet: PVNEGO10,
            dest: r10_id,
            src: rem_id,
            version: 0,
            offset: 19,
            pn_offset: 0,
            payload_length: crate::MAX_PACKET_SIZE - 19,
            ptype: PacketType::VersionNegotiation,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PVNEGOBIS10,
            dest: r10_id,
            src: rem_id,
            version: 0,
            offset: 19,
            pn_offset: 0,
            payload_length: crate::MAX_PACKET_SIZE - 19,
            ptype: PacketType::VersionNegotiation,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PHANDSHAKE,
            dest: local_id,
            src: rem_id,
            version: 0x50435130,
            offset: 21,
            pn_offset: 21,
            payload_length: 0x400,
            ptype: PacketType::Handshake,
            spin: false,
            epoch: Epoch::Handshake,
            pc: PacketContext::Handshake,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PACKET_SHORT_PHI0_C_32,
            dest: r10_id,
            src: null,
            version: 0,
            offset: 9,
            pn_offset: 9,
            payload_length: crate::MAX_PACKET_SIZE - 9,
            ptype: PacketType::OneRttProtected,
            spin: false,
            epoch: Epoch::OneRtt,
            pc: PacketContext::Application,
            key_phase: false,
            decode_only: false,
            local_cid_len: 8,
        },
        Entry {
            packet: PACKET_SHORT_PHI0_C_32_SPIN,
            dest: r10_id,
            src: null,
            version: 0,
            offset: 9,
            pn_offset: 9,
            payload_length: crate::MAX_PACKET_SIZE - 9,
            ptype: PacketType::OneRttProtected,
            spin: true,
            epoch: Epoch::OneRtt,
            pc: PacketContext::Application,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
        Entry {
            packet: PACKET_SHORT_PHI1_NOC_32,
            dest: null,
            src: null,
            version: 0,
            offset: 1,
            pn_offset: 1,
            payload_length: crate::MAX_PACKET_SIZE - 1,
            ptype: PacketType::OneRttProtected,
            spin: false,
            epoch: Epoch::OneRtt,
            pc: PacketContext::Application,
            key_phase: true,
            decode_only: true,
            local_cid_len: 0,
        },
        Entry {
            packet: PACKET_INTEL_BUG,
            dest: || {
                ConnectionId::clone_from_slice(&[0xbb, 0xba, 0xda, 0x0e, 0xf9, 0x26, 0x00, 0xc8])
                    .unwrap()
            },
            src: null,
            version: 1,
            offset: 18,
            pn_offset: 18,
            payload_length: 1182,
            ptype: PacketType::Initial,
            spin: false,
            epoch: Epoch::Initial,
            pc: PacketContext::Initial,
            key_phase: false,
            decode_only: true,
            local_cid_len: 8,
        },
    ];

    let mut packet = [0u8; crate::MAX_PACKET_SIZE];

    // First loop: decode test
    for (i, e) in entries.iter().enumerate() {
        quic.local_connection_id_length = e.local_cid_len;
        packet.fill(0xcc);
        packet[..e.packet.len()].copy_from_slice(e.packet);
        let mut ph = PacketHeader::default();
        quic.parse_packet_header(&packet, Some(&addr), &mut ph, true)
            .unwrap_or_else(|_| panic!("parse_packet_header failed at entry {i}"));
        assert_eq!(
            ph.dest_connection_id,
            (e.dest)(),
            "dest CID mismatch at {i}"
        );
        assert_eq!(ph.src_connection_id, (e.src)(), "src CID mismatch at {i}");
        assert_eq!(ph.version, e.version, "version mismatch at {i}");
        assert_eq!(ph.offset, e.offset, "offset mismatch at {i}");
        assert_eq!(
            ph.packet_number_offset, e.pn_offset,
            "pn_offset mismatch at {i}"
        );
        assert_eq!(
            ph.payload_length, e.payload_length,
            "payload_length mismatch at {i}"
        );
        assert_eq!(ph.packet_type, e.ptype, "ptype mismatch at {i}");
        assert_eq!(ph.spin, e.spin, "spin mismatch at {i}");
        assert_eq!(ph.epoch, e.epoch, "epoch mismatch at {i}");
        assert_eq!(ph.packet_context, e.pc, "pc mismatch at {i}");
        assert_eq!(ph.key_phase, e.key_phase, "key_phase mismatch at {i}");
    }

    quic.local_connection_id_length = 8;

    // Second loop: encode + verify (decode_test_only == false entries only)
    for (i, e) in entries.iter().enumerate() {
        if e.decode_only {
            continue;
        }

        {
            let cnx = quic
                .connection_ref_by_id(ini_id())
                .expect("lookup cnx 2nd loop");
            if i < 2 {
                cnx.set_path_tuple_remote_cid(0, 0, ConnectionId::default());
            } else {
                cnx.set_path_tuple_remote_cid(0, 0, r10_id());
            }
        }

        packet.fill(0xcc);
        let mut pn_offset = 0usize;
        let mut pn_length = 0usize;

        let header_length = {
            let cnx = quic
                .connection_ref_by_id(ini_id())
                .expect("lookup cnx create_hdr");
            cnx.create_packet_header_at(
                e.ptype,
                0xDEADBEEF,
                0,
                0,
                0,
                &mut packet,
                &mut pn_offset,
                &mut pn_length,
            )
        };

        update_payload_length(
            &mut packet,
            pn_offset,
            pn_offset,
            pn_offset + e.payload_length,
        );

        assert_eq!(pn_offset, e.pn_offset, "create: pn_offset mismatch at {i}");
        assert_eq!(
            &packet[..header_length],
            &e.packet[..header_length],
            "header bytes mismatch at {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// incoming_initial test

/// C: `incoming_initial_test` in `picoquictest/parseheadertest.c`.
#[test]
fn incoming_initial() {
    let current_time = Instant::from_ticks(0);
    let addr_c: SocketAddr = "0.0.0.0:12345".parse().unwrap();
    let addr_s: SocketAddr = "0.0.0.0:443".parse().unwrap();

    let mut quic = Quic::new(
        8,
        Some(util::TEST_FILE_SERVER_CERT),
        Some(util::TEST_FILE_SERVER_KEY),
        Some(util::TEST_FILE_CERT_STORE),
        Some("h3"),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("create server quic");

    let mut bytes = PACKET_INTEL_BUG.to_vec();
    let new_cnx = quic
        .incoming_packet_ex(&mut bytes, &addr_c, &addr_s, 0, 0, current_time)
        .expect("incoming_packet_ex");
    assert!(new_cnx.is_some(), "expected a new connection to be created");
}

// ---------------------------------------------------------------------------
// packet_enc_dec test

/// Helper: encrypt `length` bytes of payload using `cnx_client`, then
/// decrypt the result using `q_server` and verify the packet header.
/// C: `test_packet_encrypt_one` in `picoquictest/parseheadertest.c`.
fn test_packet_encrypt_one(
    _addr_from: &SocketAddr,
    _cnx_client: &mut crate::internal::Connection,
    _q_server: &mut Quic,
    _ptype: PacketType,
    _length: usize,
) -> crate::Result<()> {
    todo!("test_packet_encrypt_one")
}

/// C: `packet_enc_dec_test` in `picoquictest/parseheadertest.c`.
#[test]
fn packet_enc_dec() {
    let current_time = Instant::from_ticks(0);
    let addr: SocketAddr = "10.0.0.1:12345".parse().unwrap();

    let mut qclient = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("create qclient");
    let mut qserver = Quic::new(
        8,
        Some(util::TEST_FILE_SERVER_CERT),
        Some(util::TEST_FILE_SERVER_KEY),
        Some(util::TEST_FILE_CERT_STORE),
        Some("test"),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("create qserver");

    qclient
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            Some(&addr),
            current_time,
            0,
            None,
            Some("picoquic-test"),
            true,
        )
        .expect("create client cnx");

    {
        let cnx = qclient.first_cnx_mut().expect("client cnx");
        cnx.start_client().expect("start client");

        // Initial packet
        test_packet_encrypt_one(&addr, cnx, &mut qserver, PacketType::Initial, 1256)
            .expect("initial enc_dec");
    }

    // Handshake packet
    {
        let prefix_label = {
            let cnx = qclient.first_cnx_mut().unwrap();
            cnx.version_tls_prefix_label()
        };
        let hs_secret: &[u8] = &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 1, 2, 3,
            4, 5, 6, 7, 8, 9, 10,
        ];
        {
            let cnx = qclient.first_cnx_mut().unwrap();
            cnx.set_test_aead_encrypt(Epoch::Handshake, hs_secret);
            cnx.set_test_pn_enc(Epoch::Handshake, hs_secret);
            let _ = prefix_label; // used in C to derive the test context
        }
        let cnx_server = qserver.first_cnx_mut().expect("server cnx");
        cnx_server.set_test_aead_decrypt(Epoch::Handshake, hs_secret);
        cnx_server.set_test_pn_dec(Epoch::Handshake, hs_secret);

        let cnx = qclient.first_cnx_mut().unwrap();
        test_packet_encrypt_one(&addr, cnx, &mut qserver, PacketType::Handshake, 1256)
            .expect("handshake enc_dec");
    }
}

// ---------------------------------------------------------------------------
// header_length test

/// Header-length test case (mirrors C `header_length_case_t`).
struct HlCase {
    is_client: bool,
    i_cid_length: usize,
    local_cid_length: usize,
    remote_cid_length: usize,
    ptype: PacketType,
    sequence: u64,
    sequence_unack: Option<u64>,
    sequence_unack_after: Option<u64>,
}

fn header_length_test_one(hlc: &HlCase) {
    let current_time = Instant::from_ticks(0);
    let addr: SocketAddr = "0.0.0.254:12345".parse().unwrap();

    let mut i_cid_bytes = [0x11u8; 20];
    let mut r_cid_bytes = [0xddu8; 20];
    let i_cid = ConnectionId::clone_from_slice(&i_cid_bytes[..hlc.i_cid_length]).unwrap();
    let r_cid = ConnectionId::clone_from_slice(&r_cid_bytes[..hlc.remote_cid_length]).unwrap();
    let _ = (&mut i_cid_bytes, &mut r_cid_bytes);

    let mut quic = Quic::new(
        8,
        Some(util::TEST_FILE_SERVER_CERT),
        Some(util::TEST_FILE_SERVER_KEY),
        Some(util::TEST_FILE_CERT_STORE),
        Some("h3"),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("create quic");
    quic.local_connection_id_length = hlc.local_cid_length as u8;

    quic.create_connection(
        i_cid,
        r_cid,
        Some(&addr),
        current_time,
        0,
        Some("test"),
        Some("h3"),
        hlc.is_client,
    )
    .expect("create connection");

    let pc = match hlc.ptype {
        PacketType::Initial => PacketContext::Initial,
        PacketType::Handshake => PacketContext::Handshake,
        _ => PacketContext::Application,
    };

    {
        let cnx = quic.connection_ref_by_id(i_cid).unwrap();
        cnx.set_send_sequence_for_pc(pc, hlc.sequence);
        if let Some(unack) = hlc.sequence_unack {
            cnx.queue_unacked_packet_for_pc(
                pc,
                hlc.ptype,
                unack,
                crate::MAX_PACKET_SIZE,
                current_time,
            );
        }
    }

    let predicted_length = {
        let cnx = quic.connection_ref_by_id(i_cid).unwrap();
        cnx.predict_packet_header_length_for_pc(hlc.ptype, pc)
    };

    // Dequeue / re-queue for the after-state
    {
        let cnx = quic.connection_ref_by_id(i_cid).unwrap();
        if hlc.sequence_unack != hlc.sequence_unack_after {
            if hlc.sequence_unack.is_some() {
                cnx.dequeue_first_pending_for_pc(pc);
            }
            if let Some(unack_after) = hlc.sequence_unack_after {
                cnx.queue_unacked_packet_for_pc(
                    pc,
                    hlc.ptype,
                    unack_after,
                    crate::MAX_PACKET_SIZE,
                    current_time,
                );
            }
        }
    }

    let mut buffer = [0u8; crate::MAX_PACKET_SIZE];
    let mut pn_offset = 0usize;
    let mut pn_length = 0usize;
    let header_length = {
        let cnx = quic.connection_ref_by_id(i_cid).unwrap();
        cnx.create_packet_header_at(
            hlc.ptype,
            hlc.sequence,
            0,
            0,
            predicted_length,
            &mut buffer,
            &mut pn_offset,
            &mut pn_length,
        )
    };

    assert_eq!(
        header_length, predicted_length,
        "predicted={predicted_length} actual={header_length}"
    );
    assert!(
        pn_length > 0 && pn_length <= 4,
        "invalid pn_length {pn_length}"
    );
    assert_eq!(
        pn_length + pn_offset,
        header_length,
        "pn_offset={pn_offset} + pn_length={pn_length} != header_length={header_length}"
    );
}

/// C: `header_length_test` in `picoquictest/parseheadertest.c`.
#[test]
fn header_length() {
    const U64MAX: Option<u64> = None;
    let cases: &[HlCase] = &[
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 63,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 64,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 255,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xff_ffff_ffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 255,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xff_ffff_ffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
    ];

    for (i, hlc) in cases.iter().enumerate() {
        header_length_test_one(hlc);
        let _ = i; // used in assertions within header_length_test_one
    }
}
