//! Test cases for `picoquictest/bytestream_test.c`.
//!
//! Covers: integer read/write (fixed-width and varint), buffer
//! read/write, connection-id encode/decode, length-prefixed string
//! encode/decode, socket-address encode/decode, boundary-condition
//! failures, varint-length-encoding table, and stream-metadata
//! helpers (size / length / remaining / finished).

#![allow(non_snake_case)]

use core::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use crate::CONNECTION_ID_MAX_SIZE;
use crate::ConnectionId;
use crate::bytestream::{ByteStream, ByteStreamBuf};

/// Reference 16-byte encoding shared by all write sub-tests.
const EXPECTED_STREAM: [u8; 16] = [
    0x09, 0x01, 0x42, 0x03, 0x84, 0x05, 0x06, 0x07, 0xc8, 0x09, 0x05, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];

// ── Private write helpers ─────────────────────────────────────────────────────

/// C: `verify_bytestream_write_intXX`.
fn write_intxx(s: &mut ByteStream<'_>) {
    s.write_u8(0x09).unwrap();
    s.write_u8(0x01).unwrap();
    s.write_u16(0x4203).unwrap();
    s.write_u32(0x8405_0607).unwrap();
    s.write_u64(0xc809_050b_0c0d_0e0f).unwrap();
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "bytewrite_intXX: length");
    assert_eq!(s.as_bytes(), &EXPECTED_STREAM, "bytewrite_intXX: content");
}

/// C: `verify_bytestream_write_int` (varints).
fn write_vint(s: &mut ByteStream<'_>) {
    s.write_varint(0x09).unwrap();
    s.write_varint(0x01).unwrap();
    s.write_varint(0x0203).unwrap();
    s.write_varint(0x0405_0607).unwrap();
    s.write_varint(0x0809_050b_0c0d_0e0f).unwrap();
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "bytewrite_vint: length");
    assert_eq!(s.as_bytes(), &EXPECTED_STREAM, "bytewrite_vint: content");
}

/// C: `verify_bytestream_write_buffer`.
fn write_buf(s: &mut ByteStream<'_>) {
    s.write_bytes(&EXPECTED_STREAM[..2]).unwrap();
    s.write_bytes(&EXPECTED_STREAM[2..]).unwrap();
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "bytewrite_buffer: length");
    assert_eq!(s.as_bytes(), &EXPECTED_STREAM, "bytewrite_buffer: content");
}

/// C: `verify_bytestream_write_cid`.
fn write_cid(s: &mut ByteStream<'_>) {
    let cid0 =
        ConnectionId::clone_from_slice(&[0x01, 0x42, 0x03, 0x84, 0x05, 0x06, 0x07, 0xc8, 0x09])
            .unwrap();
    let cid1 = ConnectionId::clone_from_slice(&[0x0b, 0x0c, 0x0d, 0x0e, 0x0f]).unwrap();
    s.write_cid(&cid0).unwrap();
    s.write_cid(&cid1).unwrap();
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "bytewrite_cid: length");
    assert_eq!(s.as_bytes(), &EXPECTED_STREAM, "bytewrite_cid: content");
}

/// C: `verify_bytestream_write_cstr`.
///
/// C `bytewrite_cstr` encodes as `varint(strlen(str)) + raw bytes`.
/// `write_str` takes `&str` (UTF-8); the C strings are binary data
/// so we decompose into `write_varint + write_bytes` to achieve the
/// identical on-wire encoding.
fn write_cstr(s: &mut ByteStream<'_>) {
    let str0: &[u8] = b"\x01\x42\x03\x84\x05\x06\x07\xc8\x09";
    let str1: &[u8] = b"\x0b\x0c\x0d\x0e\x0f";
    s.write_varint(str0.len() as u64).unwrap();
    s.write_bytes(str0).unwrap();
    s.write_varint(str1.len() as u64).unwrap();
    s.write_bytes(str1).unwrap();
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "bytewrite_cstr: length");
    assert_eq!(s.as_bytes(), &EXPECTED_STREAM, "bytewrite_cstr: content");
}

/// C: `verify_bytestream_write` — runs all write sub-tests with `clear()` between each.
fn verify_write(s: &mut ByteStream<'_>) {
    s.clear();
    write_intxx(s);
    s.clear();
    write_vint(s);
    s.clear();
    write_buf(s);
    s.clear();
    write_cid(s);
    s.clear();
    write_cstr(s);
}

// ── Private read helpers ──────────────────────────────────────────────────────

/// C: `verify_bytestream_read_intXX`.
fn read_intxx(s: &mut ByteStream<'_>) {
    assert_eq!(s.peek_u8().unwrap(), 0x09, "byteshow_int8 first");
    assert_eq!(s.read_u8().unwrap(), 0x09, "byteread_int8 first");
    assert_eq!(s.peek_u8().unwrap(), 0x01, "byteshow_int8 second");
    assert_eq!(s.read_u8().unwrap(), 0x01, "byteread_int8 second");
    assert_eq!(s.read_u16().unwrap(), 0x4203, "byteread_int16");
    assert_eq!(s.read_u32().unwrap(), 0x8405_0607, "byteread_int32");
    assert_eq!(
        s.read_u64().unwrap(),
        0xc809_050b_0c0d_0e0f,
        "byteread_int64"
    );
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "byteread_intXX: length");
}

/// C: `verify_bytestream_read_int` (varints).
fn read_vint(s: &mut ByteStream<'_>) {
    assert_eq!(s.read_varint().unwrap(), 0x09, "byteread_vint 1");
    assert_eq!(s.read_varint().unwrap(), 0x01, "byteread_vint 2");
    assert_eq!(s.read_varint().unwrap(), 0x0203, "byteread_vint 3");
    assert_eq!(s.read_varint().unwrap(), 0x0405_0607, "byteread_vint 4");
    assert_eq!(
        s.read_varint().unwrap(),
        0x0809_050b_0c0d_0e0f,
        "byteread_vint 5"
    );
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "byteread_vint: length");
}

/// C: `verify_bytestream_skip_int`.
fn skip_vint(s: &mut ByteStream<'_>) {
    s.skip_varint().unwrap();
    assert_eq!(s.len(), 1, "skip_varint after 1st");
    s.skip_varint().unwrap();
    assert_eq!(s.len(), 2, "skip_varint after 2nd");
    s.skip_varint().unwrap();
    assert_eq!(s.len(), 4, "skip_varint after 3rd");
    s.skip_varint().unwrap();
    assert_eq!(s.len(), 8, "skip_varint after 4th");
    s.skip_varint().unwrap();
    assert_eq!(s.len(), 16, "skip_varint after 5th");
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "skip_varint: final length");
}

/// C: `verify_bytestream_read_buffer`.
fn read_buf(s: &mut ByteStream<'_>) {
    let mut buf = [0u8; 16];
    s.read_bytes(&mut buf[..2]).unwrap();
    assert_eq!(&buf[..2], &EXPECTED_STREAM[..2], "byteread_buffer first 2");
    s.read_bytes(&mut buf[..14]).unwrap();
    assert_eq!(&buf[..14], &EXPECTED_STREAM[2..], "byteread_buffer last 14");
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "byteread_buffer: length");
}

/// C: `verify_bytestream_read_cid`.
fn read_cid(s: &mut ByteStream<'_>) {
    let cid0 =
        ConnectionId::clone_from_slice(&[0x01, 0x42, 0x03, 0x84, 0x05, 0x06, 0x07, 0xc8, 0x09])
            .unwrap();
    let cid1 = ConnectionId::clone_from_slice(&[0x0b, 0x0c, 0x0d, 0x0e, 0x0f]).unwrap();
    assert_eq!(s.read_cid().unwrap(), cid0, "byteread_cid cid0");
    assert_eq!(s.read_cid().unwrap(), cid1, "byteread_cid cid1");
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "byteread_cid: length");
}

/// C: `verify_bytestream_skip_cid`.
fn skip_cid(s: &mut ByteStream<'_>) {
    s.skip_cid().unwrap();
    assert_eq!(s.len(), 10, "skip_cid after cid0");
    s.skip_cid().unwrap();
    assert_eq!(s.len(), 16, "skip_cid after cid1");
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "skip_cid: final length");
}

/// C: `verify_bytestream_read_cstr`.
fn read_cstr(s: &mut ByteStream<'_>) {
    let str0: &[u8] = b"\x01\x42\x03\x84\x05\x06\x07\xc8\x09";
    let str1: &[u8] = b"\x0b\x0c\x0d\x0e\x0f";
    let mut buf = [0u8; 16];
    let n0 = s.read_str(&mut buf).unwrap();
    assert_eq!(&buf[..n0], str0, "byteread_cstr str0");
    let n1 = s.read_str(&mut buf).unwrap();
    assert_eq!(&buf[..n1], str1, "byteread_cstr str1");
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "byteread_cstr: length");
}

/// C: `verify_bytestream_skip_cstr`.
fn skip_cstr(s: &mut ByteStream<'_>) {
    s.skip_str().unwrap();
    assert_eq!(s.len(), 10, "skip_cstr after str0");
    s.skip_str().unwrap();
    assert_eq!(s.len(), 16, "skip_cstr after str1");
    assert_eq!(s.len(), EXPECTED_STREAM.len(), "skip_cstr: final length");
}

/// C: `verify_bytestream_read` — runs all read sub-tests with `reset()` before each.
fn verify_read(s: &mut ByteStream<'_>) {
    s.reset();
    read_intxx(s);
    s.reset();
    read_vint(s);
    s.reset();
    read_buf(s);
    s.reset();
    read_cid(s);
    s.reset();
    read_cstr(s);
}

// ── Test entry ────────────────────────────────────────────────────────────────

/// C: `bytestream_test` in `picoquictest/bytestream_test.c`.
///
/// Exercises the full [`ByteStream`] surface: fixed-width integer
/// encode/decode, varint encode/decode/skip, raw-buffer copy,
/// connection-id encode/decode/skip, length-prefixed string
/// encode/decode/skip, socket-address encode/decode/skip, write/read
/// boundary failures, varint-length-encoding table, and stream
/// metadata helpers.
#[test]
fn bytestream() {
    // ── verify_bytestream_on_stack ────────────────────────────────────────────

    // Write via ByteStreamBuf inline storage.
    {
        let mut wbuf = ByteStreamBuf::default();
        let mut ws = wbuf.stream(EXPECTED_STREAM.len()).unwrap();
        verify_write(&mut ws);
    }

    // Read via a reference stream over EXPECTED_STREAM.
    {
        let mut data = EXPECTED_STREAM;
        let mut rs = ByteStream::from_slice(&mut data);
        verify_read(&mut rs);
    }

    // Skip sub-tests (each resets to position 0).
    {
        let mut data = EXPECTED_STREAM;
        let mut rs = ByteStream::from_slice(&mut data);

        rs.reset();
        skip_vint(&mut rs);

        rs.reset();
        skip_cid(&mut rs);

        rs.reset();
        skip_cstr(&mut rs);
    }

    // ── verify_bytestream_on_heap ─────────────────────────────────────────────

    {
        let mut s = ByteStream::with_capacity(EXPECTED_STREAM.len()).unwrap();

        s.reset();
        verify_write(&mut s);

        // Pre-fill with the reference stream for the read sub-tests.
        s.reset();
        s.write_bytes(&EXPECTED_STREAM).unwrap();
        s.reset();
        verify_read(&mut s);
        // Drop frees the heap buffer; no explicit delete needed.
    }

    // ── bytestream_test_write_limits ──────────────────────────────────────────

    {
        let buf8 = [0x08u8, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f];
        let mut s9 = ByteStream::with_capacity(9).unwrap();

        // write_u8: last byte succeeds, one past the end fails.
        s9.reset();
        s9.skip(8).unwrap();
        assert!(s9.write_u8(0x0e).is_ok(), "bytewrite_int8 at pos 8");
        assert!(s9.write_u8(0x0f).is_err(), "bytewrite_int8 past end");

        // write_u16: fits in last 2 bytes, second write fails.
        s9.reset();
        s9.skip(6).unwrap();
        assert!(s9.write_u16(0x0c0d).is_ok(), "bytewrite_int16 at pos 6");
        assert!(s9.write_u16(0x0e0f).is_err(), "bytewrite_int16 past end");

        // write_u32: first fits (4 bytes from pos 4), second fails.
        s9.reset();
        s9.skip(4).unwrap();
        assert!(s9.write_u32(0x0809_0a0b).is_ok(), "first bytewrite_int32");
        assert!(s9.write_u32(0x0c0d_0e0f).is_err(), "second bytewrite_int32");

        // write_u64: first fits (8 bytes from pos 0), second fails.
        s9.reset();
        assert!(
            s9.write_u64(0x0102_0304_0506_0708).is_ok(),
            "first bytewrite_int64"
        );
        assert!(
            s9.write_u64(0x0809_0a0b_0c0d_0e0f).is_err(),
            "second bytewrite_int64"
        );

        // write_varint: 8-byte varint fits, second fails.
        s9.reset();
        assert!(
            s9.write_varint(0x0102_0304_0506_0708).is_ok(),
            "first bytewrite_vint"
        );
        assert!(
            s9.write_varint(0x0809_0a0b_0c0d_0e0f).is_err(),
            "second bytewrite_vint"
        );

        // write_bytes: 8-byte write fits, second fails.
        s9.reset();
        assert!(s9.write_bytes(&buf8).is_ok(), "bytewrite_buffer of 8 bytes");
        assert!(s9.write_bytes(&buf8).is_err(), "second bytewrite_buffer");

        // After a write failure all further writes must also fail.
        assert!(s9.write_u8(0x01).is_err(), "write after failure");
    }

    // ── bytestream_test_read_limits ───────────────────────────────────────────

    {
        let buf9 = [0xc8u8, 0x00, 0x01, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0xc8];
        let mut heap = buf9;
        let mut s9 = ByteStream::from_slice(&mut heap);

        // read_u8: last byte succeeds, then fails.
        s9.reset();
        s9.skip(8).unwrap();
        assert!(s9.read_u8().is_ok(), "byteread_int8 at pos 8");
        assert!(s9.read_u8().is_err(), "byteread_int8 past end");

        // peek_u8: peek at pos 8 succeeds, then skip and peek at pos 9 fails.
        s9.reset();
        s9.skip(8).unwrap();
        assert!(s9.peek_u8().is_ok(), "byteshow_int8 at pos 8");
        s9.skip(1).unwrap();
        assert!(s9.peek_u8().is_err(), "byteshow_int8 past end");

        // read_u16: last 2 bytes fit, then fails.
        s9.reset();
        s9.skip(6).unwrap();
        assert!(s9.read_u16().is_ok(), "byteread_int16 at pos 6");
        assert!(s9.read_u16().is_err(), "byteread_int16 past end");

        // read_u32: first fits (4 bytes from pos 4), second fails.
        s9.reset();
        s9.skip(4).unwrap();
        assert!(s9.read_u32().is_ok(), "first byteread_int32");
        assert!(s9.read_u32().is_err(), "second byteread_int32");

        // read_u64: 8-byte read from pos 0 fits, second fails.
        s9.reset();
        assert!(s9.read_u64().is_ok(), "first byteread_int64");
        assert!(s9.read_u64().is_err(), "second byteread_int64");

        // read_varint: first (8-byte varint) fits, subsequent reads fail.
        // buf9[0] = 0xc8 encodes an 8-byte varint; only 1 byte then remains.
        s9.reset();
        assert!(s9.read_varint().is_ok(), "first byteread_vint");
        assert!(s9.read_varint().is_err(), "second byteread_vint");
        assert!(s9.read_varint().is_err(), "third byteread_vint (sticky)");

        // skip_varint: same sticky-failure behavior.
        s9.reset();
        assert!(s9.skip_varint().is_ok(), "first byteread_skip_vint");
        assert!(s9.skip_varint().is_err(), "second byteread_skip_vint");
        assert!(
            s9.skip_varint().is_err(),
            "third byteread_skip_vint (sticky)"
        );

        // read_bytes: 8-byte read fits, second fails.
        s9.reset();
        let mut tmp = [0u8; 8];
        assert!(
            s9.read_bytes(&mut tmp).is_ok(),
            "byteread_buffer of 8 bytes"
        );
        assert!(s9.read_bytes(&mut tmp).is_err(), "second byteread_buffer");

        // After a read failure all further reads must fail.
        assert!(s9.read_u8().is_err(), "read after failure");

        // skip_cid: length byte = 0xc8 = 200 → tries to skip 200 bytes → fails.
        s9.reset();
        assert!(s9.skip_cid().is_err(), "byteskip_cid with length 200");

        // read_cid: same.
        s9.reset();
        assert!(s9.read_cid().is_err(), "byteread_cid with length 200");

        // read_cid with encoded length > CONNECTION_ID_MAX_SIZE.
        let mut long_cid_buf = [0u8; CONNECTION_ID_MAX_SIZE + 2];
        long_cid_buf[0] = (CONNECTION_ID_MAX_SIZE + 1) as u8;
        let mut scid = ByteStream::from_slice(&mut long_cid_buf);
        assert!(scid.read_cid().is_err(), "byteread_cid oversized");

        // skip_cstr: varint length = huge → skip fails after reading varint.
        s9.reset();
        assert!(s9.skip_str().is_err(), "byteskip_cstr past end");

        // read_cstr with huge length → fails.
        s9.reset();
        let mut strbuf = [0u8; 0x100];
        assert!(
            s9.read_str(&mut strbuf).is_err(),
            "byteread_cstr(len=0x100) huge length"
        );

        // read_cstr with decoded length > dst capacity.
        let mut short_str = [0x02u8, 0x00, 0x00]; // varint 2, two bytes
        let mut sstr = ByteStream::from_slice(&mut short_str);
        assert!(
            sstr.read_str(&mut strbuf[..1]).is_err(),
            "byteread_cstr(len=1) for 2-byte str"
        );

        // read_cstr of an empty string (varint 0):
        // Rust API: "Err when length > dst.len()"; 0 > 0 is false → succeeds
        // with a 0-byte dst. C fails here due to NUL requirement.
        let mut empty_str = [0x00u8]; // varint 0
        let mut s0 = ByteStream::from_slice(&mut empty_str);

        s0.reset();
        // Rust semantics: empty string fits in a 0-byte slice (no NUL needed).
        assert!(
            s0.read_str(&mut strbuf[..0]).is_ok(),
            "byteread_cstr(len=0) empty into 0 bytes"
        );

        s0.reset();
        assert!(
            s0.read_str(&mut strbuf[..1]).is_ok(),
            "byteread_cstr(len=1) empty string"
        );
    }

    // ── bytestream_test_vint ──────────────────────────────────────────────────

    {
        struct Case {
            val: u64,
            len: usize,
        }
        let cases = [
            Case { val: 0, len: 1 },
            Case { val: 10, len: 1 },
            Case { val: 100, len: 2 },
            Case { val: 1000, len: 2 },
            Case { val: 10000, len: 2 },
            Case {
                val: 100000,
                len: 4,
            },
            Case {
                val: 10000000,
                len: 4,
            },
            Case {
                val: 1000000000,
                len: 4,
            },
            Case {
                val: 100000000000,
                len: 8,
            },
            Case {
                val: 1000000000000000,
                len: 8,
            },
            Case { val: 0x3f, len: 1 },
            Case { val: 0x40, len: 2 },
            Case { val: 0xff, len: 2 },
            Case { val: 0x100, len: 2 },
            Case {
                val: 0x3fff,
                len: 2,
            },
            Case {
                val: 0x4000,
                len: 4,
            },
            Case {
                val: 0x3fff_ffff,
                len: 4,
            },
            Case {
                val: 0x4000_0000,
                len: 8,
            },
        ];
        for case in &cases {
            assert_eq!(
                ByteStream::varint_encoded_len(case.val),
                case.len,
                "bytestream_varint_len({})",
                case.val,
            );
        }
    }

    // ── bytestream_test_addr ──────────────────────────────────────────────────

    {
        // IPv4: write twice, reset, skip once, read once, check roundtrip.
        let addr_v4 = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(0x0102_0304u32), 1234));
        let ipv4_encoded_size = 1 + 4 + 2; // family(varint) + 4-byte addr + 2-byte port
        let mut v4buf = ByteStreamBuf::default();
        let mut sv4 = v4buf.stream(2 * ipv4_encoded_size).unwrap();
        sv4.write_addr(&addr_v4).unwrap();
        sv4.write_addr(&addr_v4).unwrap();
        sv4.reset();
        sv4.skip_addr().unwrap();
        let out_v4 = sv4.read_addr().unwrap();
        assert_eq!(addr_v4, out_v4, "sockaddr_in roundtrip");

        // IPv6: same pattern.
        let mut ipv6_bytes = [0u8; 16];
        ipv6_bytes[0] = 12;
        let addr_v6 = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from(ipv6_bytes), 1234, 0, 0));
        let ipv6_encoded_size = 1 + 16 + 2; // family(varint) + 16-byte addr + 2-byte port
        let mut v6buf = ByteStreamBuf::default();
        let mut sv6 = v6buf.stream(2 * ipv6_encoded_size).unwrap();
        sv6.write_addr(&addr_v6).unwrap();
        sv6.write_addr(&addr_v6).unwrap();
        sv6.reset();
        sv6.skip_addr().unwrap();
        let out_v6 = sv6.read_addr().unwrap();
        assert_eq!(addr_v6, out_v6, "sockaddr_in6 roundtrip");
    }

    // ── bytestream_test_utils ─────────────────────────────────────────────────

    {
        let buf9 = [0xc8u8, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0xc8];
        let mut data = buf9;
        let mut s9 = ByteStream::from_slice(&mut data);

        s9.reset();
        assert_eq!(s9.capacity(), buf9.len(), "bytestream_size");
        assert_eq!(s9.len(), 0, "bytestream_length at reset");
        assert_eq!(s9.remaining(), buf9.len(), "bytestream_remain at start");

        s9.skip(8).unwrap();
        assert_eq!(s9.len(), 8, "bytestream_length after skip 8");
        assert_eq!(s9.remaining(), 1, "bytestream_remain after skip 8");
        assert!(!s9.is_finished(), "not finished at 8/9");

        s9.skip(1).unwrap();
        assert_eq!(s9.len(), buf9.len(), "bytestream_length at end");
        assert_eq!(s9.remaining(), 0, "bytestream_remain at end");
        assert!(s9.is_finished(), "finished at 9/9");
    }
}
