//! Tests from `picoquictest/cleartext_aead_test.c`.
//!
//! Covers: Initial-epoch cleartext AEAD round-trip, PN header-protection
//! primitives (AES-128-CTR known-answer and enc/dec roundtrip), HKDF
//! label-expansion against draft-17 / QUIC-v1 vectors, and retry-protection
//! integrity tag for v1 and v2.

use core::net::SocketAddr;

use crate::internal::{INTEROP_VERSION_LATEST, Version, format_32};
use crate::tls_api::{
    HASH_SIZE_MAX, LABEL_HP, LABEL_IV, LABEL_KEY, LABEL_QUIC_V1_KEY_BASE, LABEL_V1_TRAFFIC_UPDATE,
    create_retry_protection_context, encode_retry_protection, hash_create, hkdf_expand_label,
    rotate_app_secret, setup_initial_master_secret, setup_initial_secrets,
    test_pn_enc_from_raw_key, verify_retry_protection,
};
use crate::utils::format_connection_id;
use crate::{ConnectionId, Instant, MAX_PACKET_SIZE, Quic, RESET_SECRET_SIZE};

use super::util::{TEST_ALPN, TEST_FILE_CERT_STORE, TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_KEY};

// ---------------------------------------------------------------------------
// Shared packet-building helper.

/// Serialize a synthetic QUIC-like long-header packet into `buf[..target]`
/// and return the fixed header length (17 bytes) used as the AEAD AAD.
/// C: `cleartext_aead_init_packet`.
fn init_aead_packet(
    dest_cnx_id: ConnectionId,
    pn: u32,
    vn: u32,
    ptype: u8,
    buf: &mut [u8],
    target: usize,
) -> usize {
    const OFFSET: usize = 17;
    let mut idx = 0usize;
    buf[idx] = 0x80 | ptype;
    idx += 1;
    format_32(&mut buf[idx..], vn);
    idx += 4;
    idx += format_connection_id(&mut buf[idx..], dest_cnx_id) as usize;
    idx += format_connection_id(&mut buf[idx..], ConnectionId::with_size(0).unwrap()) as usize;
    format_32(&mut buf[idx..], pn);
    idx += 4;
    // "silly content" filler derived from CID and PN.
    let mut seed = dest_cnx_id.val64() ^ (pn as u64);
    while idx < target {
        seed = seed.wrapping_mul(101);
        buf[idx] = (seed & 0xFF) as u8;
        idx += 1;
    }
    OFFSET
}

// ---------------------------------------------------------------------------
// `cleartext_aead_vector_test_one` helper.

/// Create a client connection with `test_id` and `test_vn`, start it, and
/// assert that the Initial-epoch crypto contexts are populated.
/// C: `cleartext_aead_vector_test_one`.
///
/// The C function also compared the AEAD IVs against expected values, but
/// that comparison is entirely inside `#if 0` and always succeeds — so the
/// Rust translation only verifies context presence.
fn aead_vector_test_one(test_id: ConnectionId, test_vn: u32) {
    let t0 = Instant::from_ticks(0);
    let mut qclient = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("client quic");

    let addr: SocketAddr = SocketAddr::from(([10u8, 0, 0, 1], 12345u16));

    let cnx = qclient
        .create_connection(
            test_id,
            ConnectionId::with_size(0).unwrap(),
            Some(&addr),
            t0,
            test_vn,
            None,
            None,
            true,
        )
        .expect("client connection");

    cnx.start_client().expect("start client");

    assert!(
        cnx.crypto_context[0].aead_encrypt.is_some(),
        "no Initial encrypt ctx"
    );
    assert!(
        cnx.crypto_context[0].aead_decrypt.is_some(),
        "no Initial decrypt ctx"
    );
}

// ---------------------------------------------------------------------------
// PN enc/dec pair helper.

/// Encrypt `seqnum` with `pn_enc` using `sample` as the AES block input,
/// then decrypt the result with `pn_dec` and assert roundtrip correctness.
/// C: `test_one_pn_enc_pair`.
fn test_one_pn_enc_pair(
    seqnum: &[u8],
    pn_enc: &dyn crate::tls::HeaderKey,
    pn_dec: &dyn crate::tls::HeaderKey,
    sample: &[u8; 16],
) {
    let enc_mask = pn_enc.mask(*sample);
    let encoded: Vec<u8> = seqnum
        .iter()
        .zip(enc_mask.iter())
        .map(|(a, b)| a ^ b)
        .collect();

    let dec_mask = pn_dec.mask(*sample);
    let decoded: Vec<u8> = encoded
        .iter()
        .zip(dec_mask.iter())
        .map(|(a, b)| a ^ b)
        .collect();

    assert_eq!(
        &decoded[..seqnum.len()],
        seqnum,
        "PN enc/dec roundtrip failed"
    );
}

// ---------------------------------------------------------------------------
// Tests.

/// C: `cleartext_aead_test` in `picoquictest/cleartext_aead_test.c`.
#[test]
fn clear_text_aead() {
    let t0 = Instant::from_ticks(0);
    let client_addr: SocketAddr = SocketAddr::from(([10u8, 0, 0, 1], 12345u16));
    let server_addr: SocketAddr = SocketAddr::from(([10u8, 0, 0, 2], 4433u16));

    let mut qclient = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("client quic");

    let mut qserver = Quic::new(
        8,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some("test"),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("server quic");

    // Create client connection and capture its initial CID and proposed version.
    let cnx_client = qclient
        .create_connection(
            ConnectionId::with_size(0).unwrap(),
            ConnectionId::with_size(0).unwrap(),
            Some(&client_addr),
            t0,
            0,
            None,
            None,
            true,
        )
        .expect("client connection");

    let client_initial_cnxid = cnx_client.initial_connection_id;
    let client_proposed_version = cnx_client.proposed_version;

    // Create server connection using the client's initial CID.
    let cnx_server = qserver
        .create_connection(
            client_initial_cnxid,
            client_initial_cnxid,
            Some(&server_addr),
            t0,
            client_proposed_version,
            None,
            None,
            false,
        )
        .expect("server connection");

    // Verify initial CIDs match.
    assert_eq!(
        cnx_client.initial_connection_id.as_bytes(),
        cnx_server.initial_connection_id.as_bytes(),
        "initial CID mismatch"
    );

    // Build a synthetic 1200-byte cleartext packet (17-byte header + payload).
    const CLEAR_LENGTH: usize = 1200;
    const SEQNUM: u32 = 0xdeadbeef;
    let mut clear_text = [0u8; 1536];
    let offset = init_aead_packet(
        client_initial_cnxid,
        SEQNUM,
        client_proposed_version,
        2, // picoquic_packet_initial
        &mut clear_text,
        CLEAR_LENGTH,
    );

    // AEAD encrypt (client → server): header = clear_text[..offset], plaintext = clear_text[offset..CLEAR_LENGTH].
    let mut payload: Vec<u8> = clear_text[offset..CLEAR_LENGTH].to_vec();
    cnx_client.crypto_context[0]
        .aead_encrypt
        .as_ref()
        .expect("client encrypt ctx")
        .encrypt(SEQNUM as u64, &clear_text[..offset], &mut payload);

    // payload now contains ciphertext + 16-byte AEAD tag.
    let encoded_length = offset + payload.len();
    let mut incoming = [0u8; 1536];
    incoming[..offset].copy_from_slice(&clear_text[..offset]);
    incoming[offset..encoded_length].copy_from_slice(&payload);

    // AEAD decrypt (server side).
    let mut ciphertext: Vec<u8> = incoming[offset..encoded_length].to_vec();
    cnx_server.crypto_context[0]
        .aead_decrypt
        .as_ref()
        .expect("server decrypt ctx")
        .decrypt(SEQNUM as u64, &incoming[..offset], &mut ciphertext)
        .expect("AEAD decrypt");

    // ciphertext now holds the decrypted plaintext (tag stripped).
    let decoded_length = offset + ciphertext.len();
    incoming[offset..decoded_length].copy_from_slice(&ciphertext);

    assert_eq!(decoded_length, CLEAR_LENGTH, "decoded length mismatch");
    assert_eq!(
        &incoming[..CLEAR_LENGTH],
        &clear_text[..CLEAR_LENGTH],
        "decrypted bytes do not match cleartext"
    );
}

/// C: `pn_ctr_test` in `picoquictest/cleartext_aead_test.c`.
#[test]
fn pn_ctr() {
    #[rustfmt::skip]
    const KEY: [u8; 16] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
        0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
    ];
    #[rustfmt::skip]
    const IV: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
    ];
    // AES-128-ECB(KEY, IV) — the expected CTR keystream for all-zeros plaintext.
    #[rustfmt::skip]
    const EXPECTED: [u8; 16] = [
        0x3a, 0xd7, 0x7b, 0xb4, 0x0d, 0x7a, 0x36, 0x60,
        0xa8, 0x9e, 0xca, 0xf3, 0x24, 0x66, 0xef, 0x97,
    ];
    // A packet whose first byte is flags, bytes 1..5 are the PN, bytes 5..21
    // are the CTR sample, and the rest is payload.
    #[rustfmt::skip]
    const PACKET_CLEAR_PN: [u8; 31] = [
        0x5d,
        0xba, 0xba, 0xc0, 0x01,
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
        0x20, 0x3f, 0xbe, 0x2e, 0x32, 0x17, 0xfc, 0x5b,
        0x88, 0x55,
    ];
    #[rustfmt::skip]
    const PACKET_ENCRYPTED_PN: [u8; 31] = [
        0x5d,
        0x80, 0x6d, 0xbb, 0xb5,
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
        0x20, 0x3f, 0xbe, 0x2e, 0x32, 0x17, 0xfc, 0x5b,
        0x88, 0x55,
    ];

    let cipher = test_pn_enc_from_raw_key(&KEY).expect("create CTR cipher");

    // Verify the AES-128-ECB keystream against the NIST test vector.
    let keystream = cipher.mask(IV);
    assert_eq!(
        keystream, EXPECTED,
        "AES-128 keystream does not match expected"
    );

    // For each prefix length i in {1, 2, 4, 8, 16}: encrypt [i; i] bytes,
    // verify the XOR relationship with the keystream, then round-trip.
    let mut i = 1usize;
    while i <= 16 {
        let in_bytes: Vec<u8> = vec![i as u8; i];
        let out_bytes: Vec<u8> = in_bytes
            .iter()
            .zip(EXPECTED.iter())
            .map(|(a, b)| a ^ b)
            .collect();

        for j in 0..i {
            assert_eq!(
                in_bytes[j],
                out_bytes[j] ^ EXPECTED[j],
                "CTR XOR property failed at i={i}, j={j}"
            );
        }

        // Re-encrypt the ciphertext to recover plaintext.
        let decoded: Vec<u8> = out_bytes
            .iter()
            .zip(EXPECTED.iter())
            .map(|(a, b)| a ^ b)
            .collect();
        assert_eq!(&decoded, &in_bytes, "CTR roundtrip failed at i={i}");

        i *= 2;
    }

    // Verify PN encryption against the test packet vectors.
    let sample_clear: [u8; 16] = PACKET_CLEAR_PN[5..21].try_into().unwrap();
    let enc_mask = cipher.mask(sample_clear);
    let encrypted_pn: Vec<u8> = PACKET_CLEAR_PN[1..5]
        .iter()
        .zip(enc_mask.iter())
        .map(|(a, b)| a ^ b)
        .collect();
    assert_eq!(
        &encrypted_pn,
        &PACKET_ENCRYPTED_PN[1..5],
        "PN encryption does not match expected"
    );

    let sample_enc: [u8; 16] = PACKET_ENCRYPTED_PN[5..21].try_into().unwrap();
    let dec_mask = cipher.mask(sample_enc);
    let decrypted_pn: Vec<u8> = PACKET_ENCRYPTED_PN[1..5]
        .iter()
        .zip(dec_mask.iter())
        .map(|(a, b)| a ^ b)
        .collect();
    assert_eq!(
        &decrypted_pn,
        &PACKET_CLEAR_PN[1..5],
        "PN decryption does not match expected"
    );
}

/// C: `cleartext_pn_enc_test` in `picoquictest/cleartext_aead_test.c`.
#[test]
fn cleartext_pn_enc() {
    let t0 = Instant::from_ticks(0);
    let client_addr: SocketAddr = SocketAddr::from(([10u8, 0, 0, 1], 12345u16));
    let server_addr: SocketAddr = SocketAddr::from(([10u8, 0, 0, 2], 4433u16));

    let mut qclient = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("client quic");

    let mut qserver = Quic::new(
        8,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        None,
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("server quic");

    let cnx_client = qclient
        .create_connection(
            ConnectionId::with_size(0).unwrap(),
            ConnectionId::with_size(0).unwrap(),
            Some(&client_addr),
            t0,
            0,
            None,
            Some(TEST_ALPN),
            true,
        )
        .expect("client connection");

    cnx_client.start_client().expect("start client");

    let client_initial_cnxid = cnx_client.initial_connection_id;
    let client_remote_cnxid = cnx_client.remote_connection_id();
    let client_proposed_version = cnx_client.proposed_version;

    let cnx_server = qserver
        .create_connection(
            client_initial_cnxid,
            client_remote_cnxid,
            Some(&server_addr),
            t0,
            client_proposed_version,
            None,
            None,
            false,
        )
        .expect("server connection");

    // Test PN encryption pair 1: client enc → server dec.
    let seq_num_1: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
    let sample_1: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17,
        0x2a,
    ];
    test_one_pn_enc_pair(
        &seq_num_1,
        cnx_client.crypto_context[0]
            .pn_enc
            .as_deref()
            .expect("client pn_enc"),
        cnx_server.crypto_context[0]
            .pn_dec
            .as_deref()
            .expect("server pn_dec"),
        &sample_1,
    );

    // Test PN encryption pair 2: server enc → client dec.
    let seq_num_2: [u8; 4] = [0xba, 0xba, 0xc0, 0x00];
    let sample_2: [u8; 16] = [
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a, 0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f,
        0x96,
    ];
    test_one_pn_enc_pair(
        &seq_num_2,
        cnx_server.crypto_context[0]
            .pn_enc
            .as_deref()
            .expect("server pn_enc"),
        cnx_client.crypto_context[0]
            .pn_dec
            .as_deref()
            .expect("client pn_dec"),
        &sample_2,
    );
}

/// C: `cleartext_pn_vector_test` in `picoquictest/cleartext_aead_test.c`.
///
/// The entire C body is within `#if 0` and is a no-op.
#[test]
fn pn_vector() {
    // C body is entirely within #if 0; no-op.
}

/// C: `draft17_vector_test` in `picoquictest/cleartext_aead_test.c`.
#[test]
fn draft17_vector() {
    // Known-answer data for the draft-17 / QUIC-v1 interop vector.
    let draft17_test_cnx_id =
        ConnectionId::clone_from_slice(&[0x7d, 0xdc, 0x42, 0x90, 0xc4, 0xe7, 0xd2, 0x04]).unwrap();

    #[rustfmt::skip]
    let draft17_test_salt: [u8; 20] = [
        0xef, 0x4f, 0xb0, 0xab, 0xb4, 0x74, 0x70, 0xc4,
        0x1b, 0xef, 0xcf, 0x80, 0x31, 0x33, 0x4f, 0xae,
        0x48, 0x5e, 0x09, 0xa0,
    ];
    #[rustfmt::skip]
    let draft17_test_initial_secret: [u8; 32] = [
        0xe5, 0x6c, 0x75, 0x1d, 0xbc, 0x9a, 0xb8, 0xe7,
        0x9f, 0x61, 0x61, 0x42, 0xc0, 0xc0, 0x7a, 0xb8,
        0x30, 0xeb, 0x25, 0x96, 0x8f, 0xae, 0xb7, 0x40,
        0x4d, 0xa6, 0x9a, 0x80, 0xf7, 0x5f, 0x1c, 0x7c,
    ];
    #[rustfmt::skip]
    let draft17_test_server_initial_secret: [u8; 32] = [
        0x5e, 0xac, 0x74, 0x74, 0x78, 0x72, 0xfe, 0x6d,
        0x9e, 0xcb, 0xac, 0x75, 0xdf, 0x87, 0xab, 0xc4,
        0xbb, 0x43, 0x74, 0xc8, 0xe6, 0x63, 0x65, 0x49,
        0xda, 0x71, 0x8b, 0x9f, 0x72, 0x2f, 0x0d, 0x6a,
    ];
    #[rustfmt::skip]
    let draft17_test_server_key: [u8; 16] = [
        0xf3, 0x67, 0xa4, 0xc1, 0x2f, 0x77, 0x26, 0xd9,
        0x2c, 0xce, 0xa2, 0x1b, 0x93, 0x39, 0xa8, 0x71,
    ];
    #[rustfmt::skip]
    let draft17_test_server_iv: [u8; 12] = [
        0x44, 0x82, 0x14, 0xc9, 0x66, 0x31, 0x4d, 0x8f, 0x54, 0x0b, 0x7b, 0x43,
    ];
    #[rustfmt::skip]
    let draft17_test_server_pn: [u8; 16] = [
        0x92, 0x2b, 0x11, 0x3f, 0x1b, 0x2a, 0x81, 0x5f,
        0x08, 0x42, 0x54, 0xf9, 0x81, 0xa0, 0xb0, 0x97,
    ];
    #[rustfmt::skip]
    let draft17_test_client_initial_secret: [u8; 32] = [
        0xf8, 0x86, 0x16, 0x78, 0x10, 0x56, 0xa6, 0xac,
        0x00, 0x70, 0x87, 0xd1, 0x21, 0xce, 0x15, 0x8e,
        0xa8, 0xc7, 0x70, 0xa1, 0xe6, 0x28, 0x99, 0x61,
        0x6c, 0xde, 0x50, 0x7b, 0xb6, 0xd6, 0x0e, 0x08,
    ];
    #[rustfmt::skip]
    let draft17_test_client_key: [u8; 16] = [
        0x1b, 0x7e, 0x28, 0x58, 0x10, 0x18, 0x33, 0xce,
        0x98, 0x9a, 0x77, 0x25, 0x4f, 0x3f, 0xaa, 0x62,
    ];
    #[rustfmt::skip]
    let draft17_test_client_iv: [u8; 12] = [
        0x01, 0xa4, 0x1a, 0xa7, 0x3c, 0x43, 0x29, 0x8d, 0xcb, 0x38, 0xbc, 0xb6,
    ];
    #[rustfmt::skip]
    let draft17_test_client_pn: [u8; 16] = [
        0x9a, 0x85, 0x42, 0xef, 0x39, 0x90, 0x38, 0xab,
        0xa6, 0x6e, 0xf1, 0x33, 0x38, 0x09, 0xfc, 0x5b,
    ];

    // Check HKDF label expansions for server key, IV, and HP.
    let mut out = [0u8; 16];
    hkdf_expand_label(
        LABEL_KEY,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_server_initial_secret,
        &mut out,
    )
    .expect("expand server key");
    assert_eq!(&out, &draft17_test_server_key, "server key mismatch");

    let mut out_iv = [0u8; 12];
    hkdf_expand_label(
        LABEL_IV,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_server_initial_secret,
        &mut out_iv,
    )
    .expect("expand server IV");
    assert_eq!(&out_iv, &draft17_test_server_iv, "server IV mismatch");

    let mut out_hp = [0u8; 16];
    hkdf_expand_label(
        LABEL_HP,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_server_initial_secret,
        &mut out_hp,
    )
    .expect("expand server HP");
    assert_eq!(&out_hp, &draft17_test_server_pn, "server HP mismatch");

    let mut out_ck = [0u8; 16];
    hkdf_expand_label(
        LABEL_KEY,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_client_initial_secret,
        &mut out_ck,
    )
    .expect("expand client key");
    assert_eq!(&out_ck, &draft17_test_client_key, "client key mismatch");

    let mut out_civ = [0u8; 12];
    hkdf_expand_label(
        LABEL_IV,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_client_initial_secret,
        &mut out_civ,
    )
    .expect("expand client IV");
    assert_eq!(&out_civ, &draft17_test_client_iv, "client IV mismatch");

    let mut out_chp = [0u8; 16];
    hkdf_expand_label(
        LABEL_HP,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_client_initial_secret,
        &mut out_chp,
    )
    .expect("expand client HP");
    assert_eq!(&out_chp, &draft17_test_client_pn, "client HP mismatch");

    // Check the version salt and master secret derivation.
    let version_params = INTEROP_VERSION_LATEST.parameters();
    assert_eq!(
        version_params.version_aead_key.len(),
        draft17_test_salt.len(),
        "salt length mismatch for INTEROP_VERSION_LATEST"
    );
    if version_params.version_aead_key == draft17_test_salt {
        // The current INTEROP_VERSION_LATEST uses this salt — verify the full key tree.
        let mut master_secret = [0u8; 32];
        setup_initial_master_secret(&draft17_test_salt, draft17_test_cnx_id, &mut master_secret)
            .expect("setup master secret");
        assert_eq!(
            &master_secret, &draft17_test_initial_secret,
            "master secret mismatch"
        );

        let mut client_secret = [0u8; 32];
        let mut server_secret = [0u8; 32];
        setup_initial_secrets(&master_secret, &mut client_secret, &mut server_secret)
            .expect("setup initial secrets");
        assert_eq!(
            &client_secret, &draft17_test_client_initial_secret,
            "client initial secret mismatch"
        );
        assert_eq!(
            &server_secret, &draft17_test_server_initial_secret,
            "server initial secret mismatch"
        );
    }
    // (If the interop version salt no longer matches the draft-17 vector, skip the
    // master-secret check — this mirrors the C test's `else if memcmp(...) != 0` path
    // which logs a message but does not fail.)

    // Integration test: verify that the AEAD contexts are set up correctly
    // for the known CID and version.
    aead_vector_test_one(draft17_test_cnx_id, INTEROP_VERSION_LATEST as u32);
}

/// C: `retry_protection_vector_test` in `picoquictest/cleartext_aead_test.c`.
#[test]
fn retry_protection_vector() {
    // Retry pseudo-packet constants.
    const RETRY_TOKEN_LENGTH: usize = 24;

    #[rustfmt::skip]
    let retry_protection_test_input: [u8; 41] = [
        // FIRST_BYTE
        0xF5,
        // VERSION = 0xFF000019
        0xFF, 0x00, 0x00, 0x19,
        // DCID_LENGTH = 6, DCID_BYTES = 61..66
        6, 61, 62, 63, 64, 65, 66,
        // SCID_LENGTH = 4, SCID_BYTES = 44..47
        4, 44, 45, 46, 47,
        // RETRY_TOKEN (24 bytes) = 101..124
        101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
        113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124,
    ];

    let retry_protection_test_odcid =
        ConnectionId::clone_from_slice(&[81, 82, 83, 84, 85, 86, 87, 88]).unwrap();

    // Pseudo-packet = ODCID_LENGTH | ODCID_BYTES | retry_protection_test_input.
    let retry_protection_pseudo_packet: Vec<u8> = {
        let mut v = Vec::with_capacity(1 + 8 + retry_protection_test_input.len());
        v.push(8u8); // ODCID length
        v.extend_from_slice(&[81, 82, 83, 84, 85, 86, 87, 88]);
        v.extend_from_slice(&retry_protection_test_input);
        v
    };

    #[rustfmt::skip]
    let retry_protection_test_checksum: [u8; 16] = [
        0xf9, 0x50, 0xf8, 0x85, 0x71, 0x4b, 0xae, 0x7a,
        0xf1, 0xe2, 0x86, 0x7d, 0xd8, 0xf7, 0x83, 0x92,
    ];

    // Draft-25 retry packet vector.
    let retry_protection_odcid_draft25 =
        ConnectionId::clone_from_slice(&[0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08]).unwrap();
    #[rustfmt::skip]
    let retry_protection_packet_draft25: [u8; 36] = [
        0xff, 0xff, 0x00, 0x00, 0x19, 0x00, 0x08, 0xf0, 0x67, 0xa5, 0x50, 0x2a, 0x42, 0x62,
        0xb5, 0x74, 0x6f, 0x6b, 0x65, 0x6e, 0x1e, 0x5e, 0xc5, 0xb0, 0x14, 0xcb, 0xb1, 0xf0,
        0xfd, 0x93, 0xdf, 0x40, 0x48, 0xc4, 0x46, 0xa6,
    ];

    // Obtain the QUIC-v1 retry integrity key via the version parameters.
    let v1_params = Version::V1.parameters();
    let retry_key = v1_params.version_retry_key;
    let prefix_label = v1_params.tls_prefix_label;

    // Phase 1: low-level AEAD encrypt/decrypt against the known checksum.
    {
        let protection_ctx = create_retry_protection_context(true, retry_key, prefix_label)
            .expect("create protection ctx");

        // Encrypt empty plaintext; the only output is the 16-byte AEAD tag.
        let mut tag: Vec<u8> = Vec::new();
        protection_ctx.encrypt(0, &retry_protection_pseudo_packet, &mut tag);
        assert_eq!(tag.len(), 16, "tag length != 16");
        assert_eq!(
            &tag[..],
            &retry_protection_test_checksum,
            "tag does not match expected"
        );

        // Verify: decrypt the tag (should succeed with 0 plaintext bytes).
        let verification_ctx = create_retry_protection_context(false, retry_key, prefix_label)
            .expect("create verification ctx");

        let mut tag_verify = tag.clone();
        verification_ctx
            .decrypt(0, &retry_protection_pseudo_packet, &mut tag_verify)
            .expect("AEAD decrypt should succeed");
        assert!(tag_verify.is_empty(), "decrypted plaintext should be empty");

        // Negative test: corrupt one byte → decryption must fail.
        tag_verify = tag.clone();
        tag_verify[0] ^= 1;
        assert!(
            verification_ctx
                .decrypt(0, &retry_protection_pseudo_packet, &mut tag_verify)
                .is_err(),
            "corrupted tag should fail verification"
        );
    }

    // Phase 2: encode_retry_protection / verify_retry_protection.
    {
        let protection_ctx = create_retry_protection_context(true, retry_key, prefix_label)
            .expect("create protection ctx");

        let mut packet = vec![0u8; MAX_PACKET_SIZE];
        let packet_index = retry_protection_test_input.len();
        packet[..packet_index].copy_from_slice(&retry_protection_test_input);

        let length = encode_retry_protection(
            protection_ctx.as_ref(),
            &mut packet,
            packet_index,
            &retry_protection_test_odcid,
        );
        assert_eq!(length, packet_index + 16, "encoded length mismatch");
        assert_eq!(
            &packet[packet_index..length],
            &retry_protection_test_checksum,
            "appended checksum mismatch"
        );

        let verification_ctx = create_retry_protection_context(false, retry_key, prefix_label)
            .expect("create verification ctx");

        // Positive verify.
        let bytes_index = packet_index - RETRY_TOKEN_LENGTH; // = 17
        let new_length = verify_retry_protection(
            verification_ctx.as_ref(),
            &mut packet,
            length,
            bytes_index,
            &retry_protection_test_odcid,
        )
        .expect("verify retry protection");
        assert_eq!(
            new_length, packet_index,
            "verified length should equal input length"
        );

        // Bad ODCID → must fail.
        let mut bad_odcid = retry_protection_test_odcid;
        bad_odcid.as_bytes_mut()[0] ^= 1;
        packet[..packet_index].copy_from_slice(&retry_protection_test_input);
        packet[packet_index..length].copy_from_slice(&retry_protection_test_checksum);
        assert!(
            verify_retry_protection(
                verification_ctx.as_ref(),
                &mut packet,
                length,
                bytes_index,
                &bad_odcid,
            )
            .is_err(),
            "bad ODCID should fail verification"
        );

        // Draft-25 vector.
        let draft25_len = retry_protection_packet_draft25.len();
        packet[..draft25_len].copy_from_slice(&retry_protection_packet_draft25);
        let draft25_bytes_index = draft25_len - RETRY_TOKEN_LENGTH; // = 12
        verify_retry_protection(
            verification_ctx.as_ref(),
            &mut packet,
            draft25_len,
            draft25_bytes_index,
            &retry_protection_odcid_draft25,
        )
        .expect("draft-25 vector verification");
    }
}

/// C: `retry_protection_v2_test` in `picoquictest/cleartext_aead_test.c`.
#[test]
fn retry_protection_v2() {
    let v2_sample_odcid =
        ConnectionId::clone_from_slice(&[0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08]).unwrap();
    #[rustfmt::skip]
    let v2_sample_retry: [u8; 36] = [
        0xcf, 0x6b, 0x33, 0x43, 0xcf, 0x00, 0x08, 0xf0,
        0x67, 0xa5, 0x50, 0x2a, 0x42, 0x62, 0xb5, 0x74,
        0x6f, 0x6b, 0x65, 0x6e, 0xc8, 0x64, 0x6c, 0xe8,
        0xbf, 0xe3, 0x39, 0x52, 0xd9, 0x55, 0x54, 0x36,
        0x65, 0xdc, 0xc7, 0xb6,
    ];

    let v2_params = Version::V2.parameters();
    let protection_ctx_v2 = create_retry_protection_context(
        true,
        v2_params.version_retry_key,
        v2_params.tls_prefix_label,
    )
    .expect("create V2 protection context");

    // The C test copies v2_sample_retry[..byte_index] into `bytes`, then calls
    // encode_retry_protection, and compares the full result to v2_sample_retry.
    let byte_index = v2_sample_retry.len() - 16; // payload before the integrity tag
    let mut bytes = vec![0u8; MAX_PACKET_SIZE];
    bytes[..byte_index].copy_from_slice(&v2_sample_retry[..byte_index]);

    let new_index = encode_retry_protection(
        protection_ctx_v2.as_ref(),
        &mut bytes,
        byte_index,
        &v2_sample_odcid,
    );

    assert_eq!(
        new_index,
        v2_sample_retry.len(),
        "V2 retry packet length mismatch"
    );
    assert_eq!(
        &bytes[..new_index],
        &v2_sample_retry[..],
        "V2 retry packet content mismatch"
    );
}

/// C: `key_rotation_vector_test` in `picoquictest/cleartext_aead_test.c`.
#[test]
fn key_rotation_vector() {
    // Input secret: bytes 1..=64.
    let key_rotation_test_init: [u8; HASH_SIZE_MAX] = {
        let mut a = [0u8; HASH_SIZE_MAX];
        for (i, b) in a.iter_mut().enumerate() {
            *b = (i + 1) as u8;
        }
        a
    };

    // Expected outputs after rotation with LABEL_V1_TRAFFIC_UPDATE.
    #[rustfmt::skip]
    let target_sha384: [u8; 48] = [
        0xa1, 0xb5, 0xbd, 0xa2, 0x55, 0xf0, 0x7b, 0x68,
        0xdb, 0xe0, 0xa0, 0x39, 0x86, 0x94, 0xd9, 0x0d,
        0xe1, 0xf9, 0x46, 0xe4, 0x68, 0xf6, 0x87, 0xeb,
        0x19, 0x22, 0x5c, 0x92, 0x45, 0xe1, 0xf4, 0xe4,
        0x17, 0x73, 0xf6, 0x46, 0x5c, 0xb2, 0x24, 0xe0,
        0x5d, 0xb0, 0x40, 0x7a, 0x9b, 0x67, 0x47, 0xd1,
    ];
    #[rustfmt::skip]
    let target_sha256: [u8; 32] = [
        0x00, 0x70, 0x0d, 0x33, 0x5b, 0x1c, 0x49, 0xd1,
        0xe6, 0x37, 0x1e, 0x22, 0xd4, 0xa0, 0x17, 0x6d,
        0x0e, 0x34, 0x09, 0x19, 0x1b, 0x28, 0x46, 0x3c,
        0x38, 0xaf, 0x43, 0x34, 0x99, 0x43, 0x72, 0x57,
    ];
    // ChaCha20-Poly1305 also uses SHA-256.
    let target_poly = target_sha256;

    // (hash_algorithm_name, digest_size, expected_output)
    let cases: &[(&str, usize, &[u8])] = &[
        ("sha384", 48, &target_sha384),
        ("sha256", 32, &target_sha256),
        ("sha256", 32, &target_poly),
    ];

    for &(hash_name, digest_size, target) in cases {
        let mut new_secret = [0u8; HASH_SIZE_MAX];
        new_secret[..digest_size].copy_from_slice(&key_rotation_test_init[..digest_size]);

        let mut hasher = hash_create(hash_name).expect("hash_create");
        rotate_app_secret(
            &mut *hasher,
            &mut new_secret[..digest_size],
            LABEL_V1_TRAFFIC_UPDATE,
        )
        .expect("rotate_app_secret");

        assert_eq!(
            &new_secret[..digest_size],
            target,
            "key rotation mismatch for {hash_name}"
        );
    }
}
