//! Test cases for `picoquictest/qlog_test.c`.

#![allow(non_snake_case)]

use std::path::PathBuf;

use super::util::test_set_minimal_cnx_with_time;
use crate::Instant;
use crate::binlog::Binlog as _;
use crate::qlog::{
    qlog_chars, qlog_preferred_address, qlog_string, qlog_tp_version_negotiation,
    qlog_transport_extensions,
};

// ---------------------------------------------------------------------------
// autoqlog helpers.

const AUTOQLOG_BAD_QLOG: &str = "no_such_folder/bad\\folder";

fn autoqlog_bad_file() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = test_set_minimal_cnx_with_time(&mut simulated_time)?;
    quic.set_binlog(Some("."))?;
    quic.set_qlog(AUTOQLOG_BAD_QLOG)?;
    quic.first_cnx_mut()
        .expect("connection exists")
        .start_client()?;
    Ok(())
}

fn autoqlog_no_binlog() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = test_set_minimal_cnx_with_time(&mut simulated_time)?;
    quic.set_binlog(Some("."))?;
    quic.set_qlog(".")?;
    {
        let cnx = quic.first_cnx_mut().expect("connection exists");
        cnx.binlog_file_name = Some(PathBuf::from(AUTOQLOG_BAD_QLOG));
        cnx.start_client()?;
    }
    Ok(())
}

fn autoqlog_longdir() -> crate::Result<()> {
    let long_qlog = "x".repeat(511);
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = test_set_minimal_cnx_with_time(&mut simulated_time)?;
    quic.set_binlog(Some("."))?;
    quic.set_qlog(long_qlog.as_str())?;
    quic.first_cnx_mut()
        .expect("connection exists")
        .start_client()?;
    Ok(())
}

fn autoqlog_unique() -> crate::Result<()> {
    let long_qlog = "x".repeat(511);
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = test_set_minimal_cnx_with_time(&mut simulated_time)?;
    quic.set_use_unique_log_names(true);
    quic.set_binlog(Some("."))?;
    quic.set_qlog(long_qlog.as_str())?;
    {
        let cnx = quic.first_cnx_mut().expect("connection exists");
        cnx.new_connection();
        cnx.start_client()?;
    }
    Ok(())
}

/// C: `qlog_auto_test` in `picoquictest/qlog_test.c`.
#[test]
fn qlog_auto() {
    autoqlog_bad_file().expect("autoqlog_bad_file");
    autoqlog_no_binlog().expect("autoqlog_no_binlog");
    autoqlog_longdir().expect("autoqlog_longdir");
    autoqlog_unique().expect("autoqlog_unique");
}

// ---------------------------------------------------------------------------
// qlog_error helpers.

const QLOG_ERROR_FILE: &str = "qlog_error_test.txt";

fn qlog_error_string(out: &mut Vec<u8>) -> crate::Result<()> {
    // 15 bytes of data (C: data[16] with bs.size = sizeof(data)-1 = 15).
    let data = b"xxxxxxxxxxxxxxx";
    // Request 2*16=32 bytes, only 15 available → Err (truncation).
    assert!(
        qlog_string(out, data, 32).is_err(),
        "qlog_string truncation should return Err"
    );

    out.push(b'\n');

    let char_data: &[u8] = &[b'"', b'\\', 0xFF, b' ', b'z', 127];
    // Request 2*6=12 bytes, only 6 available → Err.
    assert!(
        qlog_chars(out, char_data, 12).is_err(),
        "qlog_chars truncation should return Err"
    );

    out.push(b'\n');

    // Request exactly 6 bytes from 6-byte slice → Ok.
    qlog_chars(out, char_data, 6).expect("qlog_chars full-size should succeed");

    Ok(())
}

static QLOG_PREF_ADDR: &[u8] = &[
    /* IPv4 address */ 10, 0, 0, 1, /* IPv4 port */ 1, 4, /* IPv6 address */ 2, 1,
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, /* IPv6 port */ 2, 8,
    /* CID len */ 4, /* CID value */ 15, 14, 13, 12, /* Reset token */ 0, 1, 2, 3,
    4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, /* 4 extra bytes */ 16, 17, 18, 19,
];

fn qlog_pref_addr_test(out: &mut Vec<u8>) -> crate::Result<()> {
    let size_before = out.len();
    out.push(b'\n');
    qlog_preferred_address(out, QLOG_PREF_ADDR, QLOG_PREF_ADDR.len());
    assert!(
        out.len() >= size_before + 32,
        "preferred-address output too short"
    );
    Ok(())
}

static QLOG_VNEGO_TP_INPUT: &[u8] = &[0, 0, 0, 2, 0, 0, 0, 1, 1, 2, 3, 4, 5, 6, 7, 8];

fn qlog_pref_vnego_test(out: &mut Vec<u8>) -> crate::Result<()> {
    let size_before = out.len();
    let test_lens: &[usize] = &[4, 8, 12, 16, 20, 0, 15];
    for &len in test_lens {
        out.push(b'\n');
        qlog_tp_version_negotiation(out, QLOG_VNEGO_TP_INPUT, len);
    }
    assert!(
        out.len() >= size_before + 32,
        "tp_version_negotiation output too short"
    );
    Ok(())
}

// Transport parameter IDs used in qlog_test.c:
//   picoquic_tp_ack_delay_exponent = 10 = 0x0a
//   picoquic_tp_server_preferred_address = 13 = 0x0d
//   picoquic_tp_disable_migration = 12 = 0x0c
//   picoquic_tp_retry_connection_id = 16 = 0x10
//   picoquic_tp_grease_quic_bit = 0x2ab2  (QUIC 2-byte varint: 0x6a, 0xb2)
//   picoquic_tp_version_negotiation = 0x11
//   picoquic_tp_enable_bdp_frame = 0xebd9  (QUIC 4-byte varint: 0x80, 0x00, 0xeb, 0xd9)
static QLOG_TP_EXTENSION_INPUT: &[u8] = &[
    0x0a, 1, 3, 0x0d, 45, 10, 0, 0, 1, 1, 4, 2, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    2, 8, 4, 15, 14, 13, 12, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0x0c, 0, 0x10,
    5, 10, 11, 12, 13, 14, 0x6a, 0xb2, 0, 0x11, 8, 0, 0, 0, 2, 0, 0, 0, 1, 0x80, 0x00, 0xeb, 0xd9,
    1, 1, 0xc0, 0, 0, 0xab, 0xba, 0xca, 0xda, 0xba, 5, 0xab, 0xba, 0xca, 0xda, 0xba,
];

fn qlog_tp_extension_test(out: &mut Vec<u8>) -> crate::Result<()> {
    let size_before = out.len();
    let test_lens: &[usize] = &[
        QLOG_TP_EXTENSION_INPUT.len(),
        2 * QLOG_TP_EXTENSION_INPUT.len(),
        1,
        2,
        QLOG_TP_EXTENSION_INPUT.len() - 1,
    ];
    for &len in test_lens {
        out.push(b'\n');
        qlog_transport_extensions(out, QLOG_TP_EXTENSION_INPUT, len)?;
    }
    assert!(
        out.len() >= size_before + 32,
        "transport_extensions output too short"
    );
    Ok(())
}

/// C: `qlog_error_test` in `picoquictest/qlog_test.c`.
#[test]
fn qlog_error() {
    let mut out: Vec<u8> = Vec::new();

    out.push(b'\n');
    qlog_error_string(&mut out).expect("qlog_error_string");
    out.push(b'\n');

    qlog_pref_addr_test(&mut out).expect("qlog_pref_addr_test");
    out.push(b'\n');

    qlog_pref_vnego_test(&mut out).expect("qlog_pref_vnego_test");
    out.push(b'\n');

    qlog_tp_extension_test(&mut out).expect("qlog_tp_extension_test");
    out.push(b'\n');

    std::fs::write(QLOG_ERROR_FILE, &out).expect("write qlog_error_test.txt");
}
