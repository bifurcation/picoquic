# Phase 5B repair Rust test mismatches

You are repairing Phase 5A `needs_fix` entries.  The goal
is to make the Rust tests faithfully check the same behavior
as the C tests.

Rules:

* Edit Rust tests, Rust test helpers, and Rust test fixtures
  only: `rs/fq/src/tests/` and `rs/fq/tests/fixtures/`.
* Do not edit C sources.
* Do not weaken assertions, skip important C cases, or replace
  tests with placeholders.
* If the test already matches after closer inspection, report
  `ok` and do not edit source.
* Phase 5B is about test/API correspondence, not test success.
  The Rust test must exist, compile as a test, and be runnable
  by the Rust test harness, but it may fail arbitrarily early
  because the Rust library implementation is incomplete.
* Do not report `blocked` merely because the implementation
  returns the wrong state, fails a handshake, lacks protocol
  behavior, or would fail the test. Those are Phase 5C issues.
* Report `blocked` only when the faithful test cannot be
  written, compiled, or exposed as a runnable Rust test because
  the necessary Rust API/test-harness surface is missing or
  ambiguous.
* Do not run full `cargo test` in this pass. Use source review
  and, if needed, `cargo check --tests` for compile validation.

Owned Rust test file(s): `rs/fq/src/tests/cleartext_aead.rs`, `rs/fq/src/tests/netperf.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/cleartext_aead_test.c:draft17_vector_test`
* C test-table name: `draft17_vector`
* C entry function: `draft17_vector_test`
* Rust test: `draft17_vector`
* Expected Rust file: `rs/fq/src/tests/cleartext_aead.rs`
* Rust span: `rs/fq/src/tests/cleartext_aead.rs:494-653`
* Phase 5A analysis: Rust matches the HKDF label-vector checks and conditional master-secret checks, but it always runs aead_vector_test_one. In C, that integration check is inside the salt-matches-draft17 branch and is skipped for the current INTEROP_VERSION_LATEST salt mismatch.
* Phase 5A fix note: Move the Rust aead_vector_test_one call inside the same salt-equality branch as the master/client/server secret checks.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: HKDF and conditional secret checks are present, but the AEAD integration vector is still called unconditionally after the salt branch; C only calls it when the draft-17 salt matches.
* Phase 5C fix note: Move `aead_vector_test_one(...)` inside the `version_aead_key == draft17_test_salt` branch.

### C test body
```c
{
    int ret = 0;
    int version_index = 0;
    ptls_iovec_t salt;
    uint8_t master_secret[256];
    uint8_t client_secret[256];
    uint8_t server_secret[256];
    ptls_cipher_suite_t* cipher = (ptls_cipher_suite_t*)picoquic_get_aes128gcm_sha256_v(0);

    if (cipher == NULL) {
        DBG_PRINTF("%s", "Could not find the default cipher suite.");
        ret = -1;
    }
    else {
        /* Check the label expansions */
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_KEY, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_server_initial_secret, sizeof(draft17_test_server_initial_secret),
            draft17_test_server_key, sizeof(draft17_test_server_key));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_IV, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_server_initial_secret, sizeof(draft17_test_server_initial_secret),
            draft17_test_server_iv, sizeof(draft17_test_server_iv));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_HP, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_server_initial_secret, sizeof(draft17_test_server_initial_secret),
            draft17_test_server_pn, sizeof(draft17_test_server_pn));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_KEY, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_client_initial_secret, sizeof(draft17_test_client_initial_secret),
            draft17_test_client_key, sizeof(draft17_test_client_key));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_IV, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_client_initial_secret, sizeof(draft17_test_client_initial_secret),
            draft17_test_client_iv, sizeof(draft17_test_client_iv));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_HP, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_client_initial_secret, sizeof(draft17_test_client_initial_secret),
            draft17_test_client_pn, sizeof(draft17_test_client_pn));
    }

    /* Check the salt */
    version_index = picoquic_get_version_index(draft17_test_vn);
    if (version_index < 0) {
        DBG_PRINTF("Test version (%x) is not supported.\n", draft17_test_vn);
        ret = -1;
    }
    else if (picoquic_supported_versions[version_index].version_aead_key == NULL) {
        DBG_PRINTF("Test version (%x) has no salt.\n", draft17_test_vn);
        ret = -1;
    }
    else if (picoquic_supported_versions[version_index].version_aead_key_length != sizeof(draft17_test_salt))
    {
        DBG_PRINTF("Test version (%x) has no salt[%d], expected [%d].\n", draft17_test_vn,
            (int)picoquic_supported_versions[version_index].version_aead_key_length, (int) sizeof(draft17_test_salt));
        ret = -1;
    }
    else if (memcmp(picoquic_supported_versions[version_index].version_aead_key, draft17_test_salt, sizeof(draft17_test_salt)) != 0) {
        /* TODO: this test means that the reminder of the code will not be executed for new versions */
        DBG_PRINTF("Test version (%x) does not have matching salt.\n", draft17_test_vn);
    }
    else {

        /* Check the master secret and then client and server secret */
        if (ret == 0) {
            salt.base = draft17_test_salt;
            salt.len = sizeof(draft17_test_salt);

            ret = picoquic_setup_initial_master_secret(cipher, salt, draft17_test_cnx_id, master_secret);

            if (ret != 0) {
                DBG_PRINTF("Cannot compute master secret, ret = %x\n", ret);
            }
            else {
                if (memcmp(master_secret, draft17_test_initial_secret, sizeof(draft17_test_initial_secret)) != 0) {
                    DBG_PRINTF("%s", "Initial master secret does not match expected value");
                    ret = -1;
                }
            }

            if (ret == 0) {
                ret = picoquic_setup_initial_secrets(cipher, master_secret, client_secret, server_secret);

                if (ret != 0) {
                    DBG_PRINTF("Cannot derive client and server secrets, ret = %x\n", ret);
                }
                else {
                    if (memcmp(client_secret, draft17_test_client_initial_secret, sizeof(draft17_test_client_initial_secret)) != 0) {
                        DBG_PRINTF("%s", "Initial client secret does not match expected value");
                        ret = -1;
                    }

                    if (memcmp(server_secret, draft17_test_server_initial_secret, sizeof(draft17_test_server_initial_secret)) != 0) {
                        DBG_PRINTF("%s", "Initial server secret does not match expected value");
                        ret = -1;
                    }
                }
            }
        }

        /* First integration test: verify that the aead keys are as expected */
        if (ret == 0) {
            ret = cleartext_aead_vector_test_one(draft17_test_cnx_id, draft17_test_vn,
                draft17_test_client_iv, sizeof(draft17_test_client_iv),
                draft17_test_server_iv, sizeof(draft17_test_server_iv), "draft17_vector");
        }

#if 0
        /* TODO: reset this test once we have draft-17 samples. */
        /* Final integration test: verify that the incoming packet can be decrypted */
        if (ret == 0) {
            ret = draft31_incoming_initial_test(void);
        }
#endif
    }
    return ret;
}
```

### Current Rust test body
```rust
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
```

## `picoquictest/cleartext_aead_test.c:pn_ctr_test`
* C test-table name: `pn_ctr`
* C entry function: `pn_ctr_test`
* Rust test: `pn_ctr`
* Expected Rust file: `rs/fq/src/tests/cleartext_aead.rs`
* Rust span: `rs/fq/src/tests/cleartext_aead.rs:271-374`
* Phase 5A analysis: Known-answer mask and packet vectors match, but the variable-length loop derives ciphertext from EXPECTED instead of exercising the Rust PN helper for lengths 1,2,4,8,16, and omits the C in-place check.
* Phase 5A fix note: Drive pn_encrypt/HeaderKey output for each tested length, verify prefix masks and round-trip XOR, and add an in-place-style second XOR check.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The current Rust test checks the known mask and packet vectors, but the variable-length loop derives ciphertext from EXPECTED instead of calling pn_encrypt/HeaderKey for each length and still omits the in-place second XOR check.
* Phase 5C fix note: Restore the Phase 5B repair: import/use pn_encrypt for lengths 1,2,4,8,16 and add the in-place-style roundtrip assertion.

### C test body
```c
{
    int ret = 0;

    static const uint8_t key[] = {
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
        0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c };
    static const uint8_t iv[] = {
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a };
    static const uint8_t expected[] = { 
        0x3a, 0xd7, 0x7b, 0xb4, 0x0d, 0x7a, 0x36, 0x60,
        0xa8, 0x9e, 0xca, 0xf3, 0x24, 0x66, 0xef, 0x97 };
    static const uint8_t packet_clear_pn[] = {
        0x5D,
        0xba, 0xba, 0xc0, 0x01,
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
        0x20, 0x3f, 0xbe, 0x2e, 0x32, 0x17, 0xfc, 0x5b, 
        0x88, 0x55
    };
    static const uint8_t packet_encrypted_pn[] = {
        0x5d,
        0x80, 0x6d, 0xbb, 0xb5,
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
        0x20, 0x3f, 0xbe, 0x2e, 0x32, 0x17, 0xfc, 0x5b,
        0x88, 0x55
    };

    uint8_t in_bytes[16];
    uint8_t out_bytes[16];
    uint8_t decoded[16];

    picoquic_tls_api_init();

    ptls_aead_algorithm_t* aead = (ptls_aead_algorithm_t*)picoquic_get_aes128gcm_v(0);
    ptls_cipher_context_t *pn_enc = (aead == NULL)?NULL:ptls_cipher_new(aead->ctr_cipher, 1, key);

    if (pn_enc == NULL) {
        ret = -1;
    } else {
        /* test against expected value, from PTLS test */
        ptls_cipher_init(pn_enc, iv);
        memset(in_bytes, 0, 16);
        ptls_cipher_encrypt(pn_enc, out_bytes, in_bytes, sizeof(in_bytes));
        if (memcmp(out_bytes, expected, 16) != 0) {
            ret = -1;
        }

        /* test for various values of the PN length */

        for (size_t i = 1; ret == 0 && i <= 16; i *= 2) {
            memset(in_bytes, (int)i, i);
            ptls_cipher_init(pn_enc, iv);
            ptls_cipher_encrypt(pn_enc, out_bytes, in_bytes, i);
            for (size_t j = 0; j < i; j++) {
                if (in_bytes[j] != (out_bytes[j] ^ expected[j])) {
                    ret = -1;
                    break;
                }
            }
            ptls_cipher_init(pn_enc, iv);
            ptls_cipher_encrypt(pn_enc, decoded, out_bytes, i);
            if (memcmp(in_bytes, decoded, i) != 0) {
                ret = -1;
            }

            ptls_cipher_init(pn_enc, iv);
            ptls_cipher_encrypt(pn_enc, out_bytes, out_bytes, i);
            if (memcmp(in_bytes, out_bytes, i) != 0) {
                ret = -1;
            }
        }

        /* Test with the encrypted value from the packet */
        if (ret == 0)
        {
            ptls_cipher_init(pn_enc, packet_clear_pn + 5);
            ptls_cipher_encrypt(pn_enc, out_bytes, packet_clear_pn + 1, 4);
            if (memcmp(out_bytes, packet_encrypted_pn + 1, 4) != 0)
            {
                ret = -1;
            } else {
                ptls_cipher_init(pn_enc, packet_encrypted_pn + 5);
                ptls_cipher_encrypt(pn_enc, out_bytes, packet_encrypted_pn + 1, 4);
                if (memcmp(out_bytes, packet_clear_pn + 1, 4) != 0)
                {
                    ret = -1;
                }
            }
        }
        // cleanup
        ptls_cipher_free(pn_enc);
    }

    return ret;
}
```

### Current Rust test body
```rust
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
```

## `picoquictest/cleartext_aead_test.c:retry_protection_vector_test`
* C test-table name: `retry_protection_vector`
* C entry function: `retry_protection_vector_test`
* Rust test: `retry_protection_vector`
* Expected Rust file: `rs/fq/src/tests/cleartext_aead.rs`
* Rust span: `rs/fq/src/tests/cleartext_aead.rs:657-815`
* Phase 5A analysis: Rust covers the same phases, but uses Version::V1 retry key while the C vector explicitly uses picoquic_retry_protection_key_25; that is a different test vector. Rust also omits the explicit IV comparison.
* Phase 5A fix note: Use the draft-25 retry-protection key bytes from picoquic_retry_protection_key_25 for this test, and add an equivalent expected-IV check if the Rust API exposes or can derive it.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Merged Rust test still uses Version::V1 retry key rather than C's picoquic_retry_protection_key_25 and does not perform the explicit expected-IV checks, so the Phase 5B repair appears lost.
* Phase 5C fix note: Use the draft-25 retry-protection key bytes and add protection/verification IV assertions equivalent to the C cleartext_iv_cmp checks.

### C test body
```c
{
    /* First, create a protection context to test the basic mechanisms */
    int ret = 0;
    void* protection_ctx = picoquic_create_retry_protection_context(1, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);

    if (protection_ctx == NULL) {
        DBG_PRINTF("%s", "Cannot create protection context!");
        ret = -1;
    }
    else if (0 != cleartext_iv_cmp(protection_ctx, retry_protection_test_iv, sizeof(retry_protection_test_iv))) {
        DBG_PRINTF("%s", "Clear protection IV does not match expected value.\n");
            ret = -1;
    } 
    else {
        uint8_t encoded[256];
        size_t encoded_length = picoquic_aead_encrypt_generic(encoded, encoded, 0, 0, retry_protection_pseudo_packet, sizeof(retry_protection_pseudo_packet), protection_ctx);

        if (encoded_length != 16) {
            DBG_PRINTF("Encoded length = %d instead of 16", (int)encoded_length);
            ret = -1;
        }
        else if (memcmp(encoded, retry_protection_test_checksum, 16) != 0) {
            DBG_PRINTF("%s", "Test vector does not match!");
            ret = -1;
        }

        picoquic_aead_free(protection_ctx);

        if (ret == 0) {
            void* verification_ctx = picoquic_create_retry_protection_context(0, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
            if (verification_ctx == NULL) {
                DBG_PRINTF("%s", "Cannot create verification context!");
                ret = -1;
            }
            else if (0 != cleartext_iv_cmp(verification_ctx, retry_protection_test_iv, sizeof(retry_protection_test_iv))) {
                DBG_PRINTF("%s", "Clear verification IV does not match expected value.\n");
                    ret = -1;
            }
            else {
                uint8_t decoded[256];
                size_t decoded_length = picoquic_aead_decrypt_generic(decoded, encoded, encoded_length, 0, retry_protection_pseudo_packet, sizeof(retry_protection_pseudo_packet), verification_ctx);

                if (decoded_length != 0) {
                    DBG_PRINTF("Decoded length = %d instead of 0", (int)decoded_length);
                    ret = -1;
                }
                else {
                    /* Positive test succeeded, now do a negative test */
                    encoded[0] ^= 1;
                    decoded_length = picoquic_aead_decrypt_generic(decoded, encoded, encoded_length, 0, retry_protection_pseudo_packet, sizeof(retry_protection_pseudo_packet), verification_ctx);
                    if (decoded_length == 0) {
                        DBG_PRINTF("Decoded length = 0 instead of expected error", (int)decoded_length);
                        ret = -1;
                    }
                }

                picoquic_aead_free(verification_ctx);
            }
        }
    }

    if (ret == 0) {
        /* Test the verification functions */
        void* protection_ctx = picoquic_create_retry_protection_context(1, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
        uint8_t packet[PICOQUIC_MAX_PACKET_SIZE];
        size_t packet_index = sizeof(retry_protection_test_input);

        if (protection_ctx == NULL) {
            DBG_PRINTF("%s", "Cannot create protection context!");
            ret = -1;
        }
        else {
            size_t length;
            memcpy(packet, retry_protection_test_input, packet_index);

            length = picoquic_encode_retry_protection(protection_ctx, packet, PICOQUIC_MAX_PACKET_SIZE, packet_index, &retry_protection_test_odcid);

            if (length != packet_index + sizeof(retry_protection_test_checksum)) {
                DBG_PRINTF("Packet length = %d instead of %d+16", (int)length, (int)packet_index);
                ret = -1;
            }
            else if (memcmp(packet + packet_index, retry_protection_test_checksum, sizeof(retry_protection_test_checksum)) != 0) {
                DBG_PRINTF("%s", "Packet checksum does not match!");
                ret = -1;
            }
            
            picoquic_aead_free(protection_ctx);

            if (ret == 0) {
                void* verification_ctx = picoquic_create_retry_protection_context(0, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
                if (verification_ctx == NULL) {
                    DBG_PRINTF("%s", "Cannot create verification context!");
                    ret = -1;
                }
                else {
                    size_t data_length = length;
                    size_t bytes_index = sizeof(retry_protection_test_input) - RETRY_PROTECTION_TEST_RETRY_TOKEN_LENGTH;

                    ret = picoquic_verify_retry_protection(verification_ctx, packet, &data_length, bytes_index, &retry_protection_test_odcid);

                    if (ret != 0) {
                        DBG_PRINTF("Verification returns %d (0x%d)!", ret, ret);
                    }
                    else if (data_length != sizeof(retry_protection_test_input)) {
                        DBG_PRINTF("Verification returns length %d instead of %d!", (int)data_length, (int)sizeof(retry_protection_test_input));
                        ret = -1;
                    }

                    if (ret == 0) {
                        /* Try verification with a different odcid. It should fail */
                        picoquic_connection_id_t bad_odcid = retry_protection_test_odcid;
                        bad_odcid.id[0] ^= 1;
                        data_length = length;
                        if (picoquic_verify_retry_protection(verification_ctx, packet, &data_length, bytes_index, &bad_odcid) == 0) {
                            DBG_PRINTF("%s", "Bad odcid not detected!");
                            ret = -1;
                        }
                    }


                    if (ret == 0) {
                        /* Verify that the draft 25 vector passes */
                        data_length = sizeof(retry_protection_packet_draft25);
                        memcpy(packet, retry_protection_packet_draft25, data_length);
                        bytes_index = data_length - RETRY_PROTECTION_TEST_RETRY_TOKEN_LENGTH;

                        ret = picoquic_verify_retry_protection(verification_ctx, packet, &data_length, bytes_index, &retry_protection_odcid_draft25);

                        if (ret != 0) {
                            DBG_PRINTF("Testing vector in draft 25 returns %d (0x%x)!", ret, ret);
                        }
                    }

                    picoquic_aead_free(verification_ctx);
                }
            }
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
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
```

## `picoquictest/netperf_test.c:netperf_basic_test`
* C test-table name: `netperf_basic`
* C entry function: `netperf_basic_test`
* Rust test: `netperf_basic`
* Expected Rust file: `rs/fq/src/tests/netperf.rs`
* Rust span: `rs/fq/src/tests/netperf.rs:246-254`
* Phase 5A analysis: Rust preserves the basic scenario inputs, zero loss, deadline argument, and packet-count assertions, but it does not actually use the 10-packet send buffer/coalesced-send path that the C netperf test exercises. The shared Rust verifier also ignores the 1,000,000 us max completion time.
* Phase 5A fix note: Implement the netperf large-send-buffer path in Rust, including coalesced packet preparation/splitting comparable to C, and enforce max_completion_microsec.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust wrapper inputs match C, but the final shared harness is merge-corrupted: TestTlsApiCtx duplicates send_buffer_size/use_udp_gso fields and set_send_buffer_size is not exposed as a valid impl method, so the large-send-buffer surface netperf_basic calls is not runnable.
* Phase 5C fix note: Restore the TestTlsApiCtx send-buffer fields once and expose set_send_buffer_size in impl so netperf_basic can compile and exercise the coalesced send-buffer path.

### C test body
```c
{
    int ret = netperf_one_scenario(netperf_scenario_basic, sizeof(netperf_scenario_basic),
        NULL,
        0, 0, 0, 0, 0, 1000000, NULL, NULL, 10 * PICOQUIC_MAX_PACKET_SIZE);

    return ret;
}
```

### Current Rust test body
```rust
fn netperf_basic() {
    netperf_one_scenario(
        NETPERF_SCENARIO_BASIC,
        None,
        0,
        1_000_000,
        10 * MAX_PACKET_SIZE,
    );
}
```

## `picoquictest/netperf_test.c:netperf_bbr_test`
* C test-table name: `netperf_bbr`
* C entry function: `netperf_bbr_test`
* Rust test: `netperf_bbr`
* Expected Rust file: `rs/fq/src/tests/netperf.rs`
* Rust span: `rs/fq/src/tests/netperf.rs:258-268`
* Phase 5A analysis: The wrapper uses the right basic scenario, BBR algorithm, loss mask, target, and packet-size constant, but Rust does not run the C netperf coalesced-send loop with the supplied large send buffer and also misses C completion verification.
* Phase 5A fix note: Port/use the netperf send-buffer/coalescing loop semantics and enforce scenario verification plus the 1000000 us completion bound.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The intended Rust netperf wrapper matches the C API contract, including BBR, deadline, and large send buffer, but the current merged harness no longer validly exposes the send-buffer surface and has duplicate TestTlsApiCtx fields.
* Phase 5C fix note: Restore a valid TestTlsApiCtx::set_send_buffer_size harness surface and deduplicate send_buffer_size/use_udp_gso fields so the coalesced-send test is runnable.

### C test body
```c
{
    int ret = netperf_one_scenario(netperf_scenario_basic, sizeof(netperf_scenario_basic),
        picoquic_bbr_algorithm,
        0, 0, 0, 0, 0, 1000000, NULL, NULL, 10 * PICOQUIC_MAX_PACKET_SIZE);

    return ret;
}
```

### Current Rust test body
```rust
fn netperf_bbr() {
    register_all_congestion_control_algorithms();
    let algo = get_congestion_algorithm("bbr").expect("bbr algorithm");
    netperf_one_scenario(
        NETPERF_SCENARIO_BASIC,
        Some(algo),
        0,
        1_000_000,
        10 * MAX_PACKET_SIZE,
    );
}
```
