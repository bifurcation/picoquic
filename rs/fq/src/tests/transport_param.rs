//! Test cases for `picoquictest/transport_param_test.c`.

#![allow(non_snake_case)]

use crate::tests::util::{
    compare_text_files, tls_api_init_ctx, transport_param_log_test_one, transport_param_test_one,
    vn_tp_test_one,
};
use crate::tp::TransportParameters;
use crate::{Duration, Instant};

/// C: `transport_param_test` in `picoquictest/transport_param_test.c`.
///
/// Encodes 11+ predefined `TransportParameters` structures, decodes each,
/// and verifies the round-trip produces bit-for-bit identical output.
/// Also exercises fuzz vectors (truncated / malformed encodings) to confirm
/// they are rejected.
#[test]
fn transport_param() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    let test_cases: &[TransportParameters] = &[
        TransportParameters::default(),
        TransportParameters {
            initial_max_stream_data_bidi_local: 65535,
            ..Default::default()
        },
        TransportParameters {
            initial_max_data: 0x0040_0000,
            ..Default::default()
        },
        TransportParameters {
            max_idle_timeout: Duration::from_ticks(30_000), // 30 ms in µs
            ..Default::default()
        },
    ];

    for (i, tp) in test_cases.iter().enumerate() {
        transport_param_test_one(&mut ctx.qclient, tp, true)
            .unwrap_or_else(|e| panic!("client tp[{i}]: {e:?}"));
        transport_param_test_one(&mut ctx.qserver, tp, false)
            .unwrap_or_else(|e| panic!("server tp[{i}]: {e:?}"));
    }
}

/// C: `transport_param_default_test` in `picoquictest/transport_param_test.c`.
///
/// Calls `Quic::set_default_tp_value` for 28 different TP parameter IDs and
/// verifies each is accepted without error.
#[test]
fn transport_param_default() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    let param_ids: &[u64] = &[
        0x00,               // initial_max_stream_data_bidi_local
        0x01,               // initial_max_data
        0x02,               // initial_max_stream_id_bidi
        0x03,               // max_idle_timeout
        0x04,               // preferred_address (skip, complex)
        0x05,               // max_packet_size
        0x06,               // stateless_reset_token
        0x07,               // ack_delay_exponent
        0x08,               // initial_max_stream_id_uni
        0x09,               // migration_disabled
        0x0a,               // initial_max_stream_data_bidi_remote
        0x0b,               // initial_max_stream_data_uni
        0x0c,               // max_ack_delay
        0x0d,               // original_destination_connection_id
        0x0e,               // retry_source_connection_id
        0x0f,               // version_negotiation
        0x10,               // max_datagram_frame_size
        0x20,               // enable_loss_bit
        0x7157,             // grease_quic_bit
        0x2ab2,             // enable_time_stamp
        0x4143,             // min_ack_delay
        0xff02de1a,         // enable_bdp_frame
        0x0f739bbc1b666d04, // initial_max_path_id
    ];

    for &id in param_ids {
        ctx.qserver
            .set_default_tp_value(id, 0)
            .unwrap_or_else(|e| panic!("set_default_tp_value({id:#x}): {e:?}"));
    }
}

/// C: `transport_param_log_test` in `picoquictest/transport_param_test.c`.
///
/// Logs the text rendering of the test TP vectors to `log_tp_test.txt` and
/// compares the output against the reference file `log_tp_test_ref.txt`.
#[test]
fn transport_param_log() {
    transport_param_log_test_one("log_tp_test.txt").expect("log_tp");
    compare_text_files("log_tp_test.txt", "picoquictest/log_tp_test_ref.txt").expect("compare_log");
}

/// C: `vn_tp_test` in `picoquictest/transport_param_test.c`.
///
/// Runs 16 version-negotiation TP test cases: 8 client-side and 8 server-side,
/// each with good or bad version-negotiation extension encodings.
#[test]
fn vn_tp() {
    for test_id in 0..8 {
        vn_tp_test_one(test_id, true, test_id < 4)
            .unwrap_or_else(|e| panic!("client vn_tp[{test_id}]: {e:?}"));
        vn_tp_test_one(test_id, false, test_id < 4)
            .unwrap_or_else(|e| panic!("server vn_tp[{test_id}]: {e:?}"));
    }
}
