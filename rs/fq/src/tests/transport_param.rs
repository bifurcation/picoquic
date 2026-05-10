//! Test cases for `picoquictest/transport_param_test.c`.

#![allow(non_snake_case)]

use crate::internal::{SUPPORTED_VERSIONS, Version, process_tp_version_negotiation};
use crate::tests::util::{
    TEST_ALPN, TEST_FILE_CERT_STORE, TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_KEY,
    compare_text_files, transport_param_log_test_one,
};
use crate::tp::{PreferredAddress, TransportParameters};
use crate::{ConnectionId, Duration, Instant, Quic, RESET_SECRET_SIZE, State, TransportError};
use core::net::SocketAddr;

/// C: `transport_param_test` in `picoquictest/transport_param_test.c`.
///
/// Encodes 11+ predefined `TransportParameters` structures, decodes each,
/// and verifies the round-trip produces bit-for-bit identical output.
/// Also exercises fuzz vectors (truncated / malformed encodings) to confirm
/// they are rejected.
#[test]
fn transport_param() {
    let version_default = Version::V1 as u32;

    transport_param_one(
        "TP1/CP1",
        0,
        false,
        version_default,
        version_default,
        tp1(),
        CLIENT_PARAM1,
    );
    transport_param_one(
        "TP2/CP2",
        0,
        false,
        version_default,
        0x0a1a_0a1a,
        tp2(),
        CLIENT_PARAM2,
    );
    transport_param_decode(
        "TP3/CP3",
        0,
        version_default,
        0x0a1a_0a1a,
        tp3(),
        CLIENT_PARAM3,
    );
    transport_param_one(
        "TP4/SP1",
        1,
        false,
        version_default,
        version_default,
        tp4(),
        SERVER_PARAM1,
    );
    transport_param_one(
        "TP5/SP2",
        1,
        false,
        version_default,
        0x0a1a_0a1a,
        tp5(),
        SERVER_PARAM2,
    );
    transport_param_decode(
        "TP6/CP4",
        0,
        version_default,
        0x0a1a_0a1a,
        tp6(),
        CLIENT_PARAM4,
    );
    transport_param_decode(
        "TP7/CP5",
        0,
        version_default,
        0xbaba_baba,
        tp7(),
        CLIENT_PARAM5,
    );
    transport_param_decode(
        "TP8/CP8",
        0,
        version_default,
        0x0a1a_0a1a,
        tp8(),
        CLIENT_PARAM8,
    );
    transport_param_decode(
        "TP9/SP3",
        1,
        version_default,
        0x0a1a_0a1a,
        tp9(),
        SERVER_PARAM3,
    );
    transport_param_one(
        "TP10/CP9",
        0,
        false,
        version_default,
        version_default,
        tp10(),
        CLIENT_PARAM9,
    );
    transport_param_decode(
        "TP8/CP10",
        0,
        version_default,
        0x0a1a_0a1a,
        tp8(),
        CLIENT_PARAM10,
    );
    transport_param_one(
        "TP1/CP11-grease",
        0,
        true,
        version_default,
        version_default,
        tp1(),
        CLIENT_PARAM11,
    );
    transport_param_one(
        "TP11/CP12",
        0,
        false,
        version_default,
        version_default,
        tp11(),
        CLIENT_PARAM12,
    );

    for (i, target) in TRANSPORT_PARAM_ERROR_CASES.iter().enumerate() {
        transport_param_error(&format!("error[{i}]"), 0, target);
    }

    transport_param_fuzz(
        "client fuzz CP2",
        0,
        version_default,
        0x0a1a_0a1a,
        tp2(),
        CLIENT_PARAM2,
    );
    transport_param_fuzz(
        "server fuzz SP2",
        1,
        version_default,
        0x0a1a_0a1a,
        tp2(),
        SERVER_PARAM2,
    );
}

const LOCAL_CONNECTION_ID: [u8; 8] = [2, 3, 4, 5, 6, 7, 8, 9];
const INITIAL_CONNECTION_ID: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
const RESET_TOKEN: [u8; RESET_SECRET_SIZE] =
    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
const ACK_DELAY_MAX_DEFAULT: u32 = 25_000;
const NB_PATH_TARGET: u32 = 8;

const CLIENT_PARAM1: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e, 3, 2,
    0x45, 0xc8, 9, 4, 0x80, 0, 0x40, 0, 14, 1, 8, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9, 0xc0, 0, 0, 0,
    0x9f, 0x81, 0xa1, 0x76, 1, 2,
];
const CLIENT_PARAM2: &[u8] = &[
    5, 4, 0x81, 0, 0, 0, 4, 4, 0x81, 0, 0, 0, 8, 1, 1, 1, 2, 0x40, 0xff, 3, 2, 0x45, 0xc8, 15, 8,
    2, 3, 4, 5, 6, 7, 8, 9, 32, 2, 0x45, 0xc8, 0x50, 0x57, 1, 1, 0x80, 0, 0x71, 0x58, 1, 3, 0x6a,
    0xb2, 0, 0xc0, 0x17, 0xf7, 0x58, 0x6d, 0x2c, 0xb5, 0x71, 0,
];
const CLIENT_PARAM3: &[u8] = &[
    5, 4, 0x81, 0, 0, 0, 4, 4, 0x81, 0, 0, 0, 8, 1, 1, 1, 2, 0x40, 0xff, 15, 8, 2, 3, 4, 5, 6, 7,
    8, 9, 0xc0, 0, 0, 0, 0xff, 4, 0xde, 0x1b, 2, 0x43, 0xe8, 0x80, 0, 0x71, 0x58, 1, 3,
];
const CLIENT_PARAM4: &[u8] = &[
    5, 4, 0x80, 1, 0, 0, 4, 8, 0xc0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 1, 1, 0x1e, 3, 2, 0x45, 0xc8,
    15, 8, 2, 3, 4, 5, 6, 7, 8, 9, 0x3e, 1, 4,
];
const CLIENT_PARAM5: &[u8] = &[
    1, 2, 0x40, 0x0a, 8, 1, 2, 5, 4, 0x80, 0, 0x20, 0, 4, 4, 0x80, 0, 0x40, 0, 3, 2, 0x45, 0xc0,
    10, 1, 0x11, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
];
const SERVER_PARAM1: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e, 3, 2,
    0x45, 0xc8, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 2, 16, 1, 2, 3, 4, 5,
    6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
const SERVER_PARAM2: &[u8] = &[
    5, 4, 0x81, 0, 0, 0, 4, 4, 0x81, 0, 0, 0, 8, 1, 2, 1, 2, 0x40, 0xff, 3, 2, 0x45, 0xc8, 15, 8,
    2, 3, 4, 5, 6, 7, 8, 9, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 2, 16, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
const CLIENT_PARAM8: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 3, 2, 0x45, 0xc8, 15, 8, 2, 3,
    4, 5, 6, 7, 8, 9,
];
const SERVER_PARAM3: &[u8] = &[
    5, 4, 0x81, 0, 0, 0, 4, 4, 0x81, 0, 0, 0, 8, 1, 2, 1, 2, 0x40, 0xff, 3, 2, 0x45, 0xc8, 15, 8,
    2, 3, 4, 5, 6, 7, 8, 9, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 2, 16, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16, 13, 45, 10, 0, 0, 1, 0x11, 0x51, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 4, 1, 2, 3, 4, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
const CLIENT_PARAM9: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e, 3, 2,
    0x45, 0xc8, 9, 4, 0x80, 0, 0x40, 0, 12, 0, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
];
const CLIENT_PARAM10: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 3, 2, 0x45, 0xc8, 15, 8, 2, 3,
    4, 5, 6, 7, 8, 9, 0x4c, 0x56, 4, 0xde, 0xad, 0xbe, 0xef,
];
const CLIENT_PARAM11: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e, 3, 2,
    0x45, 0xc8, 9, 4, 0x80, 0, 0x40, 0, 14, 1, 8, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9, 0x40, 0x59, 2,
    0x42, 0x03, 0xc0, 0, 0, 0, 0x9f, 0x81, 0xa1, 0x76, 1, 2,
];
const CLIENT_PARAM12: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e, 3, 2,
    0x45, 0xc8, 9, 4, 0x80, 0, 0x40, 0, 14, 1, 8, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9, 0x6a, 0xb2, 0,
];

const CLIENT_PARAM_ERR2: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e, 3, 2,
    0x45, 0xc8, 9, 2, 0x80, 0, 0x40, 0, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
];
const CLIENT_PARAM_ERR3: &[u8] = CLIENT_PARAM_ERR2;
const CLIENT_PARAM_ERR4: &[u8] = &[
    5, 2, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e, 3, 2, 0x45, 0xc8,
    9, 2, 0x80, 0, 0x40, 0, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
];
const CLIENT_PARAM_ERR5: &[u8] = &[
    5, 4, 0x80, 0x40, 0, 0, 4, 2, 0xff, 0xff, 8, 4, 0x80, 0, 0x40, 0, 1, 1, 0x1e, 3, 2, 0x45, 0xc8,
    9, 2, 0x80, 0, 0x40, 0, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
];
const CLIENT_PARAM_ERR6: &[u8] = &[
    5, 4, 0x80, 0x40, 0, 0, 4, 4, 0x40, 0x40, 0, 0, 8, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 3, 2, 0x45,
    0xc8, 9, 4, 0x80, 0, 0x40, 0, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
];
const CLIENT_PARAM_ERR7: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 8, 4, 0, 0, 0x40, 0, 1, 4, 0, 0, 0, 0x1e, 3,
    2, 0x45, 0xc8, 9, 4, 0x80, 0, 0x40, 0, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
];
const CLIENT_PARAM_ERR8: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 3, 2, 0x44, 0xaf, 15, 8, 2, 3,
    4, 5, 6, 7, 8, 9,
];
const CLIENT_PARAM_ERR9: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 3, 4, 0x80, 0, 0xff, 0xf8, 15,
    8, 2, 3, 4, 5, 6, 7, 8, 9,
];
const CLIENT_PARAM_ERR10: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
    14, 1, 1,
];
const CLIENT_PARAM_ERR11: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
    10, 1, 21,
];
const CLIENT_PARAM_ERR12: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
    11, 4, 0x80, 0, 0x40, 0x01,
];
const CLIENT_PARAM_ERR13: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
    8, 8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];
const CLIENT_PARAM_ERR14: &[u8] = &[
    5, 4, 0x80, 0, 0xff, 0xff, 4, 4, 0x80, 0x40, 0, 0, 1, 1, 0x1e, 15, 8, 2, 3, 4, 5, 6, 7, 8, 9,
    9, 8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];
const TRANSPORT_PARAM_ERROR_CASES: &[&[u8]] = &[
    CLIENT_PARAM_ERR2,
    CLIENT_PARAM_ERR3,
    CLIENT_PARAM_ERR4,
    CLIENT_PARAM_ERR5,
    CLIENT_PARAM_ERR6,
    CLIENT_PARAM_ERR7,
    CLIENT_PARAM_ERR8,
    CLIENT_PARAM_ERR9,
    CLIENT_PARAM_ERR10,
    CLIENT_PARAM_ERR11,
    CLIENT_PARAM_ERR12,
    CLIENT_PARAM_ERR13,
    CLIENT_PARAM_ERR14,
];

fn cid(bytes: &[u8]) -> ConnectionId {
    ConnectionId::clone_from_slice(bytes).expect("connection id")
}

fn version_index(version: u32) -> i32 {
    SUPPORTED_VERSIONS
        .iter()
        .position(|v| *v as u32 == version)
        .expect("supported version") as i32
}

fn transport_param_context(mode: i32, preferred_version: u32) -> Box<Quic> {
    let mut quic = Quic::new(
        8,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        None,
        None,
    )
    .expect("quic context");

    let initial_cid = cid(&INITIAL_CONNECTION_ID);
    let local_cid = cid(&LOCAL_CONNECTION_ID);
    let addr = SocketAddr::from(([0u8; 4], 0u16));
    let cnx = quic
        .create_connection(
            initial_cid,
            local_cid,
            Some(&addr),
            Instant::from_ticks(0),
            preferred_version,
            Some("sni"),
            Some("alpn"),
            mode == 0,
        )
        .expect("transport parameter connection");

    if let Some(token) = cnx
        .paths
        .first()
        .and_then(|path| path.tuples.first())
        .and_then(|tuple| tuple.local_connection_id)
    {
        cnx.local_connection_ids
            .get_mut(token)
            .expect("local cid")
            .connection_id = local_cid;
    }
    if let Some(remote_cid) = cnx
        .remote_connection_id_stashes
        .first_mut()
        .and_then(|stash| stash.connection_ids.first_mut())
    {
        remote_cid.connection_id = local_cid;
    }

    quic
}

fn transport_param_one(
    name: &str,
    mode: i32,
    grease: bool,
    version: u32,
    proposed_version: u32,
    params: TransportParameters,
    target: &[u8],
) {
    let mut quic = transport_param_context(mode, proposed_version);
    let mut buffer = [0u8; 256];
    let mut encoded = 0usize;
    let local_cid;

    {
        let cnx = quic.first_cnx_mut().expect("connection");
        cnx.local_parameters = params.clone();
        cnx.version_index = version_index(version);
        cnx.proposed_version = proposed_version;
        cnx.grease_transport_parameters = grease;
        if mode == 1 {
            cnx.is_hcid_verified = true;
        }
        assert_eq!(
            cnx.prepare_transport_extensions(mode, &mut buffer, 256, &mut encoded),
            0,
            "{name}: prepare_transport_extensions"
        );
        local_cid = cnx.local_cnxid();
    }

    assert_eq!(encoded, target.len(), "{name}: encoded length");
    if mode == 0 {
        assert_eq!(&buffer[..encoded], target, "{name}: encoded bytes");
    } else {
        let secret_offset = target.len() - RESET_SECRET_SIZE;
        assert_eq!(
            &buffer[..secret_offset],
            &target[..secret_offset],
            "{name}: encoded bytes before reset secret"
        );
        let mut reset_secret = [0u8; RESET_SECRET_SIZE];
        quic.create_connection_id_reset_secret(&local_cid, &mut reset_secret)
            .expect("reset secret");
        assert_eq!(
            &buffer[secret_offset..encoded],
            &reset_secret,
            "{name}: generated reset secret"
        );
    }

    let cnx = quic.first_cnx_mut().expect("connection");
    let mut decoded = 0usize;
    assert_eq!(
        cnx.receive_transport_extensions(mode, &mut buffer, encoded, &mut decoded),
        0,
        "{name}: receive_transport_extensions"
    );
    assert_transport_parameters_match(name, &cnx.remote_parameters, &params);
}

fn transport_param_decode(
    name: &str,
    mode: i32,
    version: u32,
    proposed_version: u32,
    expected: TransportParameters,
    target: &[u8],
) {
    let mut quic = transport_param_context(mode, version);
    let cnx = quic.first_cnx_mut().expect("connection");
    cnx.proposed_version = proposed_version;
    let mut bytes = target.to_vec();
    let mut decoded = 0usize;

    assert_eq!(
        cnx.receive_transport_extensions(mode, &mut bytes, target.len(), &mut decoded),
        0,
        "{name}: receive_transport_extensions"
    );
    assert_eq!(decoded, target.len(), "{name}: decoded length");
    assert_transport_parameters_match(name, &cnx.remote_parameters, &expected);
}

fn transport_param_error(name: &str, mode: i32, target: &[u8]) {
    let mut quic = transport_param_context(mode, 0);
    let cnx = quic.first_cnx_mut().expect("connection");
    let mut bytes = target.to_vec();
    let mut decoded = 0usize;
    let ret = cnx.receive_transport_extensions(mode, &mut bytes, target.len(), &mut decoded);

    assert_ne!(ret, 0, "{name}: malformed transport parameters accepted");
    assert!(
        matches!(
            cnx.connection_state,
            State::Disconnecting | State::HandshakeFailure
        ),
        "{name}: unexpected connection state {:?}",
        cnx.connection_state
    );
    assert_eq!(
        cnx.local_error,
        TransportError::ParameterError as u64,
        "{name}: local error"
    );
}

fn transport_param_fuzz(
    name: &str,
    mode: i32,
    version: u32,
    proposed_version: u32,
    params: TransportParameters,
    target: &[u8],
) {
    assert!(
        target.len() >= 8 && target.len() <= 256,
        "{name}: invalid fuzz target length"
    );

    let mut quic = transport_param_context(mode, 0);
    {
        let cnx = quic.first_cnx_mut().expect("connection");
        cnx.local_parameters = params;
        cnx.version_index = version_index(version);
        cnx.proposed_version = proposed_version;
    }

    let cnx = quic.first_cnx_mut().expect("connection");
    let mut proof = 0u64;
    let mut fuzz_byte = 1u8;

    for l in 1..=8usize {
        for i in l..=target.len() {
            let mut buffer = target.to_vec();
            for b in &mut buffer[i - l..i] {
                *b ^= fuzz_byte;
                fuzz_byte = fuzz_byte.wrapping_add(1);
            }
            for dl in (0..target.len()).step_by(l + 6) {
                let input_len = target.len() - dl;
                let mut decoded = 0usize;
                let fuzz_ret =
                    cnx.receive_transport_extensions(mode, &mut buffer, input_len, &mut decoded);
                if fuzz_ret != 0 {
                    proof = proof.wrapping_add(fuzz_ret as i64 as u64);
                } else {
                    proof = proof
                        .wrapping_add(cnx.remote_parameters.initial_max_stream_data_bidi_local);
                    assert!(
                        decoded <= input_len,
                        "{name}: decoded {decoded} bytes from {input_len}-byte input"
                    );
                }
            }
        }
    }

    core::hint::black_box(proof);
}

fn assert_transport_parameters_match(
    name: &str,
    actual: &TransportParameters,
    expected: &TransportParameters,
) {
    macro_rules! assert_tp_field_eq {
        ($field:ident) => {
            assert_eq!(
                actual.$field,
                expected.$field,
                "{}: {}",
                name,
                stringify!($field)
            );
        };
    }

    assert_tp_field_eq!(initial_max_stream_data_bidi_local);
    assert_tp_field_eq!(initial_max_stream_data_bidi_remote);
    assert_tp_field_eq!(initial_max_stream_data_uni);
    assert_tp_field_eq!(initial_max_data);
    assert_tp_field_eq!(initial_max_stream_id_bidir);
    assert_tp_field_eq!(initial_max_stream_id_unidir);
    assert_tp_field_eq!(max_idle_timeout);
    assert_eq!(
        actual.preferred_address.v4, expected.preferred_address.v4,
        "{name}: preferred_address.v4"
    );
    assert_eq!(
        actual.preferred_address.v6, expected.preferred_address.v6,
        "{name}: preferred_address.v6"
    );
    assert_eq!(
        actual.preferred_address.connection_id, expected.preferred_address.connection_id,
        "{name}: preferred_address.connection_id"
    );
    assert_eq!(
        actual.preferred_address.stateless_reset_token,
        expected.preferred_address.stateless_reset_token,
        "{name}: preferred_address.stateless_reset_token"
    );
    assert_tp_field_eq!(max_datagram_frame_size);
    assert_tp_field_eq!(enable_loss_bit);
    assert_tp_field_eq!(enable_time_stamp);
    assert_tp_field_eq!(min_ack_delay);
    assert_tp_field_eq!(do_grease_quic_bit);
    assert_tp_field_eq!(enable_bdp_frame);
    assert_tp_field_eq!(initial_max_path_id);
    assert_tp_field_eq!(address_discovery_mode);
    assert_tp_field_eq!(is_reset_stream_at_enabled);
}

fn tp1() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 65_535,
        initial_max_data: 0x40_0000,
        initial_max_stream_id_bidir: 16_384,
        initial_max_stream_id_unidir: 16_384,
        max_idle_timeout: Duration::from_ticks(30),
        max_packet_size: 1480,
        max_ack_delay: ACK_DELAY_MAX_DEFAULT,
        active_connection_id_limit: NB_PATH_TARGET,
        ack_delay_exponent: 3,
        address_discovery_mode: 3,
        ..Default::default()
    }
}

fn tp2() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 0x100_0000,
        initial_max_data: 0x100_0000,
        initial_max_stream_id_bidir: 1,
        max_idle_timeout: Duration::from_ticks(255),
        max_packet_size: 1480,
        max_ack_delay: ACK_DELAY_MAX_DEFAULT,
        ack_delay_exponent: 3,
        max_datagram_frame_size: 1480,
        enable_loss_bit: 2,
        enable_time_stamp: 3,
        do_grease_quic_bit: true,
        is_reset_stream_at_enabled: true,
        ..Default::default()
    }
}

fn tp3() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 0x100_0000,
        initial_max_data: 0x100_0000,
        initial_max_stream_id_bidir: 1,
        max_idle_timeout: Duration::from_ticks(255),
        max_ack_delay: ACK_DELAY_MAX_DEFAULT,
        ack_delay_exponent: 3,
        enable_time_stamp: 3,
        min_ack_delay: Duration::from_ticks(1000),
        ..Default::default()
    }
}

fn tp4() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 65_535,
        initial_max_data: 0x40_0000,
        initial_max_stream_id_bidir: 16_384,
        max_idle_timeout: Duration::from_ticks(30),
        max_packet_size: 1480,
        max_ack_delay: ACK_DELAY_MAX_DEFAULT,
        ack_delay_exponent: 3,
        ..Default::default()
    }
}

fn tp5() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 0x100_0000,
        initial_max_data: 0x100_0000,
        initial_max_stream_id_bidir: 2,
        max_idle_timeout: Duration::from_ticks(255),
        max_packet_size: 1480,
        max_ack_delay: ACK_DELAY_MAX_DEFAULT,
        ack_delay_exponent: 3,
        ..Default::default()
    }
}

fn tp6() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 0x1_0000,
        initial_max_data: 0xffff_ffff,
        max_idle_timeout: Duration::from_ticks(30),
        max_packet_size: 1480,
        max_ack_delay: ACK_DELAY_MAX_DEFAULT,
        ack_delay_exponent: 3,
        initial_max_path_id: 4,
        ..Default::default()
    }
}

fn tp7() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 8192,
        initial_max_data: 16_384,
        initial_max_stream_id_bidir: 2,
        max_idle_timeout: Duration::from_ticks(10),
        max_packet_size: 1472,
        max_ack_delay: ACK_DELAY_MAX_DEFAULT,
        ack_delay_exponent: 17,
        ..Default::default()
    }
}

fn tp8() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 65_535,
        initial_max_data: 0x40_0000,
        max_idle_timeout: Duration::from_ticks(30),
        max_packet_size: 1480,
        max_ack_delay: ACK_DELAY_MAX_DEFAULT,
        ack_delay_exponent: 3,
        ..Default::default()
    }
}

fn tp9() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 0x100_0000,
        initial_max_data: 0x100_0000,
        initial_max_stream_id_bidir: 2,
        max_idle_timeout: Duration::from_ticks(255),
        max_packet_size: 1480,
        max_ack_delay: ACK_DELAY_MAX_DEFAULT,
        ack_delay_exponent: 3,
        preferred_address: PreferredAddress {
            v4: Some(SocketAddr::from(([10, 0, 0, 1], 4433u16))),
            v6: None,
            connection_id: cid(&[1, 2, 3, 4]),
            stateless_reset_token: RESET_TOKEN,
        },
        ..Default::default()
    }
}

fn tp10() -> TransportParameters {
    TransportParameters {
        migration_disabled: true,
        active_connection_id_limit: 0,
        address_discovery_mode: 0,
        ..tp1()
    }
}

fn tp11() -> TransportParameters {
    TransportParameters {
        do_grease_quic_bit: true,
        address_discovery_mode: 0,
        ..tp1()
    }
}

/// C: `transport_param_default_test` in `picoquictest/transport_param_test.c`.
///
/// Calls `Quic::set_default_tp_value` for the C `tp_default_test_case`
/// table, checking both expected success/failure and the updated field.
#[test]
fn transport_param_default() {
    for (i, case) in DEFAULT_TP_TEST_CASES.iter().enumerate() {
        let mut quic = default_tp_test_context();
        let result = quic.set_default_tp_value(case.tp_id, case.tp_value);

        match (result, case.expected) {
            (Ok(()), DefaultTpExpected::Ok(field)) => {
                default_tp_value_check(i, case, quic.default_tp(), field);
            }
            (Err(_), DefaultTpExpected::Err) => {}
            (Ok(()), DefaultTpExpected::Err) => {
                panic!(
                    "default_tp[{i}] {}: set_default_tp_value({:#x}, {}) succeeded, expected error",
                    case.name, case.tp_id, case.tp_value
                );
            }
            (Err(err), DefaultTpExpected::Ok(_)) => {
                panic!(
                    "default_tp[{i}] {}: set_default_tp_value({:#x}, {}) failed: {err:?}",
                    case.name, case.tp_id, case.tp_value
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
struct DefaultTpCase {
    name: &'static str,
    tp_id: u64,
    tp_value: u64,
    expected: DefaultTpExpected,
}

#[derive(Clone, Copy)]
enum DefaultTpExpected {
    Ok(DefaultTpField),
    Err,
}

#[derive(Clone, Copy)]
enum DefaultTpField {
    MaxIdleTimeout,
    MaxPacketSize,
    InitialMaxData,
    InitialMaxStreamDataBidiLocal,
    InitialMaxStreamDataBidiRemote,
    InitialMaxStreamDataUni,
    InitialMaxStreamsBidi,
    InitialMaxStreamsUni,
    AckDelayExponent,
    MaxAckDelay,
    DisableMigration,
    ActiveConnectionIdLimit,
    MaxDatagramFrameSize,
    EnableLossBit,
    MinAckDelay,
    EnableTimeStamp,
    EnableBdpFrame,
    InitialMaxPathId,
    ResetStreamAt,
}

const fn default_tp_ok(
    name: &'static str,
    tp_id: u64,
    tp_value: u64,
    field: DefaultTpField,
) -> DefaultTpCase {
    DefaultTpCase {
        name,
        tp_id,
        tp_value,
        expected: DefaultTpExpected::Ok(field),
    }
}

const fn default_tp_err(name: &'static str, tp_id: u64, tp_value: u64) -> DefaultTpCase {
    DefaultTpCase {
        name,
        tp_id,
        tp_value,
        expected: DefaultTpExpected::Err,
    }
}

const DEFAULT_TP_TEST_CASES: &[DefaultTpCase] = &[
    default_tp_err("original_connection_id", 0, 0),
    default_tp_ok("idle_timeout", 1, 12345, DefaultTpField::MaxIdleTimeout),
    default_tp_err("stateless_reset_token", 2, 0),
    default_tp_ok("max_packet_size", 3, 1234, DefaultTpField::MaxPacketSize),
    default_tp_ok("initial_max_data", 4, 12345, DefaultTpField::InitialMaxData),
    default_tp_ok(
        "initial_max_stream_data_bidi_local",
        5,
        12345,
        DefaultTpField::InitialMaxStreamDataBidiLocal,
    ),
    default_tp_ok(
        "initial_max_stream_data_bidi_remote",
        6,
        12345,
        DefaultTpField::InitialMaxStreamDataBidiRemote,
    ),
    default_tp_ok(
        "initial_max_stream_data_uni",
        7,
        12345,
        DefaultTpField::InitialMaxStreamDataUni,
    ),
    default_tp_ok(
        "initial_max_streams_bidi",
        8,
        12345,
        DefaultTpField::InitialMaxStreamsBidi,
    ),
    default_tp_ok(
        "initial_max_streams_uni",
        9,
        12345,
        DefaultTpField::InitialMaxStreamsUni,
    ),
    default_tp_ok(
        "ack_delay_exponent",
        10,
        5,
        DefaultTpField::AckDelayExponent,
    ),
    default_tp_ok("max_ack_delay", 11, 12345, DefaultTpField::MaxAckDelay),
    default_tp_ok("disable_migration", 12, 1, DefaultTpField::DisableMigration),
    default_tp_err("server_preferred_address", 13, 0),
    default_tp_ok(
        "active_connection_id_limit",
        14,
        12345,
        DefaultTpField::ActiveConnectionIdLimit,
    ),
    default_tp_err("handshake_connection_id", 15, 0),
    default_tp_err("retry_connection_id", 16, 0),
    default_tp_ok(
        "max_datagram_frame_size",
        32,
        12345,
        DefaultTpField::MaxDatagramFrameSize,
    ),
    default_tp_err("test_large_chello", 3127, 0),
    default_tp_ok("enable_loss_bit", 0x1057, 1, DefaultTpField::EnableLossBit),
    default_tp_ok(
        "min_ack_delay",
        0xff04_de1b,
        12345,
        DefaultTpField::MinAckDelay,
    ),
    default_tp_ok(
        "enable_time_stamp",
        0x7158,
        1,
        DefaultTpField::EnableTimeStamp,
    ),
    // C `picoquic_set_tp_value_by_type` stores this in max_idle_timeout.
    default_tp_ok("grease_quic_bit", 0x2ab2, 1, DefaultTpField::MaxIdleTimeout),
    default_tp_err("version_negotiation", 0x11, 0),
    default_tp_ok(
        "enable_bdp_frame",
        0xebd9,
        1,
        DefaultTpField::EnableBdpFrame,
    ),
    default_tp_ok(
        "initial_max_path_id",
        0x3e,
        12345,
        DefaultTpField::InitialMaxPathId,
    ),
    // C `picoquic_set_tp_value_by_type` stores this in max_idle_timeout.
    default_tp_ok(
        "address_discovery",
        0x9f81_a176,
        1,
        DefaultTpField::MaxIdleTimeout,
    ),
    default_tp_ok(
        "reset_stream_at",
        0x17f7586d2cb571,
        1,
        DefaultTpField::ResetStreamAt,
    ),
];

fn default_tp_test_context() -> Box<Quic> {
    let mut quic = Quic::new(
        8,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        None,
        None,
    )
    .expect("quic context");
    quic.set_default_tp(&TransportParameters::default())
        .expect("zero default transport parameters");
    quic
}

fn default_tp_value_check(
    index: usize,
    case: &DefaultTpCase,
    tp: &TransportParameters,
    field: DefaultTpField,
) {
    match field {
        DefaultTpField::MaxIdleTimeout => assert_eq!(
            tp.max_idle_timeout,
            Duration::from_ticks(case.tp_value),
            "default_tp[{index}] {}: max_idle_timeout",
            case.name
        ),
        DefaultTpField::MaxPacketSize => assert_eq!(
            u64::from(tp.max_packet_size),
            case.tp_value,
            "default_tp[{index}] {}: max_packet_size",
            case.name
        ),
        DefaultTpField::InitialMaxData => assert_eq!(
            tp.initial_max_data, case.tp_value,
            "default_tp[{index}] {}: initial_max_data",
            case.name
        ),
        DefaultTpField::InitialMaxStreamDataBidiLocal => assert_eq!(
            tp.initial_max_stream_data_bidi_local, case.tp_value,
            "default_tp[{index}] {}: initial_max_stream_data_bidi_local",
            case.name
        ),
        DefaultTpField::InitialMaxStreamDataBidiRemote => assert_eq!(
            tp.initial_max_stream_data_bidi_remote, case.tp_value,
            "default_tp[{index}] {}: initial_max_stream_data_bidi_remote",
            case.name
        ),
        DefaultTpField::InitialMaxStreamDataUni => assert_eq!(
            tp.initial_max_stream_data_uni, case.tp_value,
            "default_tp[{index}] {}: initial_max_stream_data_uni",
            case.name
        ),
        DefaultTpField::InitialMaxStreamsBidi => assert_eq!(
            tp.initial_max_stream_id_bidir, case.tp_value,
            "default_tp[{index}] {}: initial_max_stream_id_bidir",
            case.name
        ),
        DefaultTpField::InitialMaxStreamsUni => assert_eq!(
            tp.initial_max_stream_id_unidir, case.tp_value,
            "default_tp[{index}] {}: initial_max_stream_id_unidir",
            case.name
        ),
        DefaultTpField::AckDelayExponent => assert_eq!(
            u64::from(tp.ack_delay_exponent),
            case.tp_value,
            "default_tp[{index}] {}: ack_delay_exponent",
            case.name
        ),
        DefaultTpField::MaxAckDelay => assert_eq!(
            u64::from(tp.max_ack_delay),
            case.tp_value,
            "default_tp[{index}] {}: max_ack_delay",
            case.name
        ),
        DefaultTpField::DisableMigration => assert_eq!(
            tp.migration_disabled as u64, case.tp_value,
            "default_tp[{index}] {}: migration_disabled",
            case.name
        ),
        DefaultTpField::ActiveConnectionIdLimit => assert_eq!(
            u64::from(tp.active_connection_id_limit),
            case.tp_value,
            "default_tp[{index}] {}: active_connection_id_limit",
            case.name
        ),
        DefaultTpField::MaxDatagramFrameSize => assert_eq!(
            u64::from(tp.max_datagram_frame_size),
            case.tp_value,
            "default_tp[{index}] {}: max_datagram_frame_size",
            case.name
        ),
        DefaultTpField::EnableLossBit => assert_eq!(
            tp.enable_loss_bit as u64, case.tp_value,
            "default_tp[{index}] {}: enable_loss_bit",
            case.name
        ),
        DefaultTpField::MinAckDelay => assert_eq!(
            tp.min_ack_delay,
            Duration::from_ticks(case.tp_value),
            "default_tp[{index}] {}: min_ack_delay",
            case.name
        ),
        DefaultTpField::EnableTimeStamp => assert_eq!(
            tp.enable_time_stamp as u64, case.tp_value,
            "default_tp[{index}] {}: enable_time_stamp",
            case.name
        ),
        DefaultTpField::EnableBdpFrame => assert_eq!(
            tp.enable_bdp_frame as u64, case.tp_value,
            "default_tp[{index}] {}: enable_bdp_frame",
            case.name
        ),
        DefaultTpField::InitialMaxPathId => assert_eq!(
            tp.initial_max_path_id, case.tp_value,
            "default_tp[{index}] {}: initial_max_path_id",
            case.name
        ),
        DefaultTpField::ResetStreamAt => assert_eq!(
            tp.is_reset_stream_at_enabled as u64, case.tp_value,
            "default_tp[{index}] {}: is_reset_stream_at_enabled",
            case.name
        ),
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
/// Runs the exact C `vn_tp_test_case` table: 9 client-mode cases and
/// 7 server-mode cases, including malformed encodings.
#[test]
fn vn_tp() {
    for (i, case) in VN_TP_TEST_CASES.iter().enumerate() {
        vn_tp_test_case(i, case);
    }
}

#[derive(Clone, Copy)]
struct VnTpCase {
    name: &'static str,
    mode: i32,
    vn_envelop: u32,
    vn_expected: u32,
    error_expected: u64,
    vn_tp: &'static [u8],
}

const VN_TP_V1: u32 = Version::V1 as u32;
const VN_TP_V2: u32 = Version::V2 as u32;
const VN_TP_VERSION_NEGOTIATION_ERROR: u64 = TransportError::VersionNegotiationError as u64;
const VN_TP_PARAMETER_ERROR: u64 = TransportError::ParameterError as u64;

const VN_TP_CLIENT_0: &[u8] = &[
    0x00, 0x00, 0x00, 0x01, 0x6b, 0x33, 0x43, 0xcf, 0x00, 0x00, 0x00, 0x01,
];
const VN_TP_CLIENT_1: &[u8] = &[
    0x00, 0x00, 0x00, 0x01, 0x0a, 0x0a, 0x0a, 0x0a, 0x6b, 0x33, 0x43, 0xcf,
];
const VN_TP_CLIENT_2: &[u8] = &[
    0x00, 0x00, 0x00, 0x01, 0x0a, 0x0a, 0x0a, 0x0a, 0x00, 0x00, 0x00, 0x01, 0x6b, 0x33, 0x43, 0xcf,
    0xfa, 0x0a, 0x0a, 0x0a,
];
const VN_TP_CLIENT_3: &[u8] = &[0x00, 0x00, 0x00, 0x01];
const VN_TP_CLIENT_BAD_1: &[u8] = &[0x00, 0x00, 0x00];
const VN_TP_CLIENT_BAD_2: &[u8] = &[0x00, 0x00, 0x00, 0x01, 0x0a, 0x0a, 0x0a];
const VN_TP_CLIENT_BAD_3: &[u8] = &[0x00];
const VN_TP_CLIENT_BAD_4: &[u8] = &[
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x50, 0x43,
];
const VN_TP_SERVER_0: &[u8] = &[0x6b, 0x33, 0x43, 0xcf];
const VN_TP_SERVER_1: &[u8] = &[
    0x6b, 0x33, 0x43, 0xcf, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03,
];
const VN_TP_SERVER_BAD_1: &[u8] = &[
    0x6b, 0x33, 0x43, 0xcf, 0x00, 0x00, 0x00, 0x01, 0x6b, 0x33, 0x43, 0xcf, 0x50, 0x43,
];
const VN_TP_SERVER_BAD_2: &[u8] = &[0x6b, 0x33, 0x43, 0xcf, 0x00];
const VN_TP_SERVER_BAD_3: &[u8] = &[0x6b, 0x33, 0x43, 0xcf, 0x00, 0x00];
const VN_TP_SERVER_BAD_4: &[u8] = &[
    0x6b, 0x33, 0x43, 0xcf, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
];

const VN_TP_TEST_CASES: &[VnTpCase] = &[
    VnTpCase {
        name: "client_0",
        mode: 0,
        vn_envelop: VN_TP_V1,
        vn_expected: VN_TP_V2,
        error_expected: 0,
        vn_tp: VN_TP_CLIENT_0,
    },
    VnTpCase {
        name: "client_1",
        mode: 0,
        vn_envelop: VN_TP_V1,
        vn_expected: VN_TP_V2,
        error_expected: 0,
        vn_tp: VN_TP_CLIENT_1,
    },
    VnTpCase {
        name: "client_2",
        mode: 0,
        vn_envelop: VN_TP_V1,
        vn_expected: VN_TP_V1,
        error_expected: 0,
        vn_tp: VN_TP_CLIENT_2,
    },
    VnTpCase {
        name: "client_3",
        mode: 0,
        vn_envelop: VN_TP_V1,
        vn_expected: 0,
        error_expected: 0,
        vn_tp: VN_TP_CLIENT_3,
    },
    VnTpCase {
        name: "client_3_version_error",
        mode: 0,
        vn_envelop: VN_TP_V2,
        vn_expected: 0,
        error_expected: VN_TP_VERSION_NEGOTIATION_ERROR,
        vn_tp: VN_TP_CLIENT_3,
    },
    VnTpCase {
        name: "client_bad_1",
        mode: 0,
        vn_envelop: VN_TP_V1,
        vn_expected: 0,
        error_expected: VN_TP_PARAMETER_ERROR,
        vn_tp: VN_TP_CLIENT_BAD_1,
    },
    VnTpCase {
        name: "client_bad_2",
        mode: 0,
        vn_envelop: VN_TP_V1,
        vn_expected: 0,
        error_expected: VN_TP_PARAMETER_ERROR,
        vn_tp: VN_TP_CLIENT_BAD_2,
    },
    VnTpCase {
        name: "client_bad_3",
        mode: 0,
        vn_envelop: VN_TP_V1,
        vn_expected: 0,
        error_expected: VN_TP_PARAMETER_ERROR,
        vn_tp: VN_TP_CLIENT_BAD_3,
    },
    VnTpCase {
        name: "client_bad_4",
        mode: 0,
        vn_envelop: VN_TP_V1,
        vn_expected: 0,
        error_expected: VN_TP_PARAMETER_ERROR,
        vn_tp: VN_TP_CLIENT_BAD_4,
    },
    VnTpCase {
        name: "server_0",
        mode: 1,
        vn_envelop: VN_TP_V2,
        vn_expected: 0,
        error_expected: 0,
        vn_tp: VN_TP_SERVER_0,
    },
    VnTpCase {
        name: "server_1",
        mode: 1,
        vn_envelop: VN_TP_V2,
        vn_expected: 0,
        error_expected: 0,
        vn_tp: VN_TP_SERVER_1,
    },
    VnTpCase {
        name: "server_0_version_error",
        mode: 1,
        vn_envelop: VN_TP_V1,
        vn_expected: 0,
        error_expected: VN_TP_VERSION_NEGOTIATION_ERROR,
        vn_tp: VN_TP_SERVER_0,
    },
    VnTpCase {
        name: "server_bad_1",
        mode: 1,
        vn_envelop: VN_TP_V2,
        vn_expected: 0,
        error_expected: VN_TP_PARAMETER_ERROR,
        vn_tp: VN_TP_SERVER_BAD_1,
    },
    VnTpCase {
        name: "server_bad_2",
        mode: 1,
        vn_envelop: VN_TP_V2,
        vn_expected: 0,
        error_expected: VN_TP_PARAMETER_ERROR,
        vn_tp: VN_TP_SERVER_BAD_2,
    },
    VnTpCase {
        name: "server_bad_3",
        mode: 1,
        vn_envelop: VN_TP_V2,
        vn_expected: 0,
        error_expected: VN_TP_PARAMETER_ERROR,
        vn_tp: VN_TP_SERVER_BAD_3,
    },
    VnTpCase {
        name: "server_bad_4",
        mode: 1,
        vn_envelop: VN_TP_V2,
        vn_expected: 0,
        error_expected: VN_TP_PARAMETER_ERROR,
        vn_tp: VN_TP_SERVER_BAD_4,
    },
];

fn vn_tp_test_case(index: usize, case: &VnTpCase) {
    let mut negotiated_vn = 0u32;
    let mut negotiated_index = -1i32;
    let mut error_found = 0u64;

    let final_bytes = process_tp_version_negotiation(
        case.vn_tp,
        case.mode,
        case.vn_envelop,
        &mut negotiated_vn,
        &mut negotiated_index,
        &mut error_found,
    );

    if final_bytes.is_none() {
        assert_ne!(
            case.error_expected, 0,
            "vn_tp[{index}] {}: unexpected parsing error 0x{error_found:x}",
            case.name
        );
        assert_eq!(
            error_found, case.error_expected,
            "vn_tp[{index}] {}: parsing error",
            case.name
        );
    } else {
        assert_eq!(
            case.error_expected, 0,
            "vn_tp[{index}] {}: expected error 0x{:x}, got success",
            case.name, case.error_expected
        );
        assert_eq!(
            negotiated_vn, case.vn_expected,
            "vn_tp[{index}] {}: negotiated version",
            case.name
        );
        if negotiated_vn != 0 {
            assert!(
                negotiated_index >= 0,
                "vn_tp[{index}] {}: negotiated index {negotiated_index}",
                case.name
            );
            assert_eq!(
                SUPPORTED_VERSIONS
                    .get(negotiated_index as usize)
                    .map(|version| *version as u32),
                Some(negotiated_vn),
                "vn_tp[{index}] {}: negotiated index maps to version",
                case.name
            );
        }
    }
}
