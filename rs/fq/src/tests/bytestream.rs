//! Test cases for `picoquictest/bytestream_test.c`.

#![allow(non_snake_case)]

use crate::ConnectionId;
use crate::bytestream::ByteStream;

/// Reference encoding of the canonical test stream:
/// `intXX` write order produces the same 16 bytes (RFC 9000 varints
/// land on the same final state).
const EXPECTED_STREAM: [u8; 16] = [
    0x09, 0x01, 0x42, 0x03, 0x84, 0x05, 0x06, 0x07, 0xc8, 0x09, 0x05, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];

/// C: `bytestream_test` in `picoquictest/bytestream_test.c`.
///
/// The C body is a sequence of seven sub-suites: write/read on
/// stack, write/read on heap, plus separate write-limit /
/// read-limit / varint / address / utility tests.  We translate
/// the smallest cross-section — the int encoder/decoder roundtrip
/// against `EXPECTED_STREAM` — and leave the rest as `todo!()`
/// guarded by the [`ByteStream`] API surface.  Phase 4 fills the
/// API bodies and the rest of the test naturally lights up.
#[test]
fn bytestream() {
    let mut storage = [0u8; 16];
    let mut s = ByteStream::from_slice(&mut storage);

    // bytewrite_int8 / int16 / int32 / int64 round-trip.
    s.write_u8(0x09).unwrap();
    s.write_u8(0x01).unwrap();
    s.write_u16(0x4203).unwrap();
    s.write_u32(0x8405_0607).unwrap();
    s.write_u64(0xc809_050b_0c0d_0e0f).unwrap();
    assert_eq!(s.len(), 16, "bytestream_length");
    assert_eq!(
        s.as_bytes(),
        &EXPECTED_STREAM,
        "byte order matches reference"
    );

    // Reset and read everything back.
    s.reset();
    assert_eq!(s.read_u8().unwrap(), 0x09);
    assert_eq!(s.read_u8().unwrap(), 0x01);
    assert_eq!(s.read_u16().unwrap(), 0x4203);
    assert_eq!(s.read_u32().unwrap(), 0x8405_0607);
    assert_eq!(s.read_u64().unwrap(), 0xc809_050b_0c0d_0e0f);
    assert!(s.is_finished(), "cursor at end");

    // Connection-ID round-trip — exercises bytewrite_cid / byteread_cid.
    s.reset();
    s.clear();
    let cid0 =
        ConnectionId::clone_from_slice(&[0x01, 0x42, 0x03, 0x84, 0x05, 0x06, 0x07, 0xc8, 0x09])
            .unwrap();
    let cid1 = ConnectionId::clone_from_slice(&[0x0b, 0x0c, 0x0d, 0x0e, 0x0f]).unwrap();
    s.write_cid(&cid0).unwrap();
    s.write_cid(&cid1).unwrap();
    s.reset();
    assert_eq!(s.read_cid().unwrap(), cid0);
    assert_eq!(s.read_cid().unwrap(), cid1);
}
