# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/mbedtls_test.c:mbedtls_crypto_test`
* C test-table name: `mbedtls_crypto`
* C entry function: `mbedtls_crypto_test`
* Rust test: `mbedtls_crypto`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:630-639`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust top-level shape matches, but the helpers mostly self-check Rust crypto or deterministic stand-ins instead of initializing mbedTLS and comparing mbedTLS against minicrypto for hash, label, ciphers, AEAD, and key exchange.
* Phase 5A fix note: Replace placeholder/self-comparison helpers with real provider comparison semantics, including PSA/mbedTLS init/free, cipher/AEAD output comparisons, hash reset cases, and key-exchange failure/abort checks.
* Phase 5B analysis: Still blocked under the corrected Phase 5B standard because the faithful Rust test would need mbedTLS provider API/test-harness surface for init/free/random, raw hash/cipher/AEAD comparison against minicrypto, and provider key exchange. The current Rust helper uses stand-ins rather than the same API-level contract.
* Phase 5B fix note: 

### C test body
```c
{
    ptls_cipher_algorithm_t* cipher_test[5] = {
        &ptls_mbedtls_aes128ecb,
        &ptls_mbedtls_aes128ctr,
        &ptls_mbedtls_aes256ecb,
        &ptls_mbedtls_aes256ctr,
        &ptls_mbedtls_chacha20
    };
    ptls_cipher_algorithm_t* cipher_ref[5] = {
        &ptls_minicrypto_aes128ecb,
        &ptls_minicrypto_aes128ctr,
        &ptls_minicrypto_aes256ecb,
        &ptls_minicrypto_aes256ctr,
        &ptls_minicrypto_chacha20
    };
    int ret = 0;

    /* Initialize the PSA crypto library. */
    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        ret = test_random();
        DBG_PRINTF("test random returns: %d\n", ret);

        if (ret == 0) {
            ret = test_hash(&ptls_mbedtls_sha256, &ptls_minicrypto_sha256);
            DBG_PRINTF("test hash returns: %d\n", ret);
        }

        if (ret == 0) {
            ret = test_label(&ptls_mbedtls_sha256, &ptls_minicrypto_sha256);
            DBG_PRINTF("test label returns: %d\n", ret);
        }

        if (ret == 0) {
            for (int i = 0; i < 5; i++) {
                if (test_cipher(cipher_test[i], cipher_ref[i]) != 0) {
                    DBG_PRINTF("test cipher %d fails\n", i);
                    ret = -1;
                }
            }
            DBG_PRINTF("test ciphers returns: %d\n", ret);
        }

        if (ret == 0) {
            ret = test_aead(&ptls_mbedtls_aes128gcm, &ptls_mbedtls_sha256, &ptls_minicrypto_aes128gcm, &ptls_minicrypto_sha256);
            DBG_PRINTF("test aeads returns: %d\n", ret);
        }

        if (ret == 0) {
            ret = test_key_exchange(&ptls_mbedtls_secp256r1, &ptls_minicrypto_secp256r1);
            if (ret != 0) {
                DBG_PRINTF("%s", "test key exchange secp256r1 mbedtls to minicrypto fails\n");
            }
            else {
                ret = test_key_exchange(&ptls_minicrypto_secp256r1, &ptls_mbedtls_secp256r1);
                if (ret != 0) {
                    DBG_PRINTF("%s", "test key exchange secp256r1 minicrypto to mbedtls fails\n");
                }
            }
            ret = test_key_exchange(&ptls_mbedtls_x25519, &ptls_minicrypto_x25519);
            if (ret != 0) {
                DBG_PRINTF("%s", "test key exchange x25519 mbedtls to minicrypto fails\n");
            }
            else {
                ret = test_key_exchange(&ptls_minicrypto_x25519, &ptls_mbedtls_x25519);
                if (ret != 0) {
                    DBG_PRINTF("%s", "test key exchange x25519 minicrypto to mbedtls fails\n");
                }
            }
            DBG_PRINTF("test key exchange returns: %d\n", ret);
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }
    return (ret == 0) ? 0 : -1;
}
```

### Current Rust test body
```rust
fn mbedtls_crypto() {
    // Initialise the PSA/mbedTLS library.
    mbedtls_test_random().expect("test_random");
    mbedtls_test_hash().expect("test_hash");
    mbedtls_test_label().expect("test_label");
    mbedtls_test_ciphers().expect("test_ciphers");
    mbedtls_test_aead().expect("test_aead");
    mbedtls_test_key_exchange().expect("test_key_exchange");
    // PSA/mbedTLS library is de-initialised by the helper internals.
}
```

## `picoquictest/cleartext_aead_test.c:draft17_vector_test`
* C test-table name: `draft17_vector`
* C entry function: `draft17_vector_test`
* Rust test: `draft17_vector`
* Expected Rust file: `rs/fq/src/tests/cleartext_aead.rs`
* Current Rust span: `rs/fq/src/tests/cleartext_aead.rs:494-653`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust matches the HKDF label-vector checks and conditional master-secret checks, but it always runs aead_vector_test_one. In C, that integration check is inside the salt-matches-draft17 branch and is skipped for the current INTEROP_VERSION_LATEST salt mismatch.
* Phase 5A fix note: Move the Rust aead_vector_test_one call inside the same salt-equality branch as the master/client/server secret checks.
* Phase 5B analysis: Rust now mirrors the C salt-match branch: the AEAD integration vector is skipped when INTEROP_VERSION_LATEST salt differs from the draft-17 vector.
* Phase 5B fix note: Moved aead_vector_test_one into the version_aead_key == draft17_test_salt branch and updated the skip comment.

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

## `picoquictest/hashtest.c:picohash_test`
* C test-table name: `picohash`
* C entry function: `picohash_test`
* Rust test: `picohash`
* Expected Rust file: `rs/fq/src/tests/hashtest.rs`
* Current Rust span: `rs/fq/src/tests/hashtest.rs:36-100`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test preserves most insert/lookup/delete/count checks, but it does not guarantee the collision-chain coverage that the C custom hash function creates for k + 32*j keys.
* Phase 5A fix note: Add collision-guaranteed coverage in Rust, such as a test-only key/hash path or other setup that forces multiple distinct keys into one bin, while preserving the same retrieval and deletion checks.
* Phase 5B analysis: Rust now guarantees the same collision-chain coverage as the C custom hash for k + 32*j keys.
* Phase 5B fix note: Added a test-only HashTestKey that hashes by the C 32-bin collision pattern and updated picohash to use it while preserving insert, lookup, missing-key, delete, and count checks.

### C test body
```c
{
    return(picohash_test_one(0));
}
```

### Current Rust test body
```rust
fn picohash() {
    use crate::hash::HashTable;

    let mut t: HashTable<HashTestKey, ()> = HashTable::new(32).expect("create hash table");

    assert_eq!(t.len(), 0);

    // Insert odd values 1, 3, 5, 7, 9.
    for i in (1u64..10).step_by(2) {
        assert!(t.insert(HashTestKey(i), ()).is_ok(), "insert({i}) failed");
    }
    assert_eq!(t.len(), 5);

    // Every inserted value is retrievable.
    for i in (1u64..10).step_by(2) {
        assert!(t.lookup(&HashTestKey(i)).is_some(), "lookup({i}) failed");
    }

    // Create collisions: for k in {1, 5}, insert k + 32*j for j in 1..=k.
    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(
                t.insert(HashTestKey(key), ()).is_ok(),
                "insert({key}) failed"
            );
        }
    }
    // Original 5 + 1 + 5 = 11.
    assert_eq!(t.len(), 11);

    // Collision entries are retrievable.
    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(
                t.lookup(&HashTestKey(key)).is_some(),
                "lookup({key}) failed"
            );
        }
    }

    // Even values 0, 2, 4, 6, 8, 10 were never inserted.
    for i in (0u64..=10).step_by(2) {
        assert!(
            t.lookup(&HashTestKey(i)).is_none(),
            "lookup({i}) returned invalid item"
        );
    }

    // Delete values 1, 5, and 9 (first, middle, and last of the originals).
    for i in (1u64..10).step_by(4) {
        let tok = t.lookup(&HashTestKey(i)).expect("pre-delete lookup");
        t.remove(tok);
    }
    assert_eq!(t.len(), 8);

    // Deleted values are gone.
    for i in (1u64..10).step_by(4) {
        assert!(
            t.lookup(&HashTestKey(i)).is_none(),
            "deleted value {i} still found"
        );
    }
}
```

## `picoquictest/netperf_test.c:nat_attack_test`
* C test-table name: `nat_attack`
* C entry function: `nat_attack_test`
* Rust test: `nat_attack`
* Expected Rust file: `rs/fq/src/tests/netperf.rs`
* Current Rust span: `rs/fq/src/tests/netperf.rs:273-295`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses the same stream scenario and attack loop shape, but the final verification is not faithful. C conditionally calls tls_api_one_scenario_verify only if the client is still Ready; Rust has a no-op Ready check and then calls tls_api_one_scenario_body_verify unconditionally, whose current Rust helper only closes and does not check stream delivery or callback errors.
* Phase 5A fix note: Add/use a Rust equivalent of tls_api_one_scenario_verify that checks callback errors, q/r byte counts and received flags, stream0 accounting, and data-node pool state; call it only when the client remains Ready after nat_attack_loop. Avoid replacing that with the close-only body_verify path.
* Phase 5B analysis: Rust now matches the C post-attack behavior: stream verification runs only when the client remains exactly Ready, and it uses the direct scenario verifier instead of the close/body path.
* Phase 5B fix note: Made tls_api_one_scenario_verify public to tests and updated nat_attack to call it conditionally after nat_attack_loop; removed the unconditional body_verify call.

### C test body
```c
{
    /* Create a connection context */
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint8_t* send_buffer = NULL;
    size_t send_buffer_size = PICOQUIC_MAX_PACKET_SIZE;
    int ret = tls_api_one_scenario_init(&test_ctx, &simulated_time, PICOQUIC_INTERNAL_TEST_VERSION_1, NULL, NULL);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    if (ret == 0 && send_buffer_size > 0) {
        send_buffer = (uint8_t*)malloc(send_buffer_size);
        if (send_buffer == 0) {
            ret = -1;
        }
    }

    if (ret == 0)
    {
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }

    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, nat_attack_scenario, sizeof(nat_attack_scenario));
    }

    /* Run a simplified simulation */
    if (ret == 0) {
        ret = nat_attack_loop(test_ctx, &simulated_time, send_buffer, send_buffer_size, 1);
    }

    /* If the client connection is still up, verify that data was properly received. */
    if (ret == 0 && test_ctx->cnx_client->cnx_state == picoquic_state_ready) {
        ret = tls_api_one_scenario_verify(test_ctx);
    }

    if (ret == 0) {
        DBG_PRINTF("Exit attack loop at time %" PRIu64 ", received %" PRIu64 " packets at client.",
            simulated_time, test_ctx->cnx_client->nb_packets_received);
    }

    if (send_buffer != NULL) {
        free(send_buffer);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn nat_attack() {
    let mut simulated_time = Instant::from_ticks(0);

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        None,
    )
    .expect("tls_api_one_scenario_init_ex");

    test_ctx.cnx_client().start_client().expect("start client");

    test_api_init_send_recv_scenario(&mut test_ctx, NAT_ATTACK_SCENARIO)
        .expect("init send/recv scenario");

    nat_attack_loop(&mut test_ctx, &mut simulated_time, true).expect("nat attack loop");

    if test_ctx.cnx_client().state() == State::Ready {
        tls_api_one_scenario_verify(&test_ctx).expect("scenario verify");
    }
}
```

## `picoquictest/skip_frame_test.c:dataqueue_copy_test`
* C test-table name: `dataqueue_copy`
* C entry function: `dataqueue_copy_test`
* Rust test: `dataqueue_copy`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:4020-4028`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust loops over the same option matrix, but the helper collapses the six C basic cases into one simplified packet shape, uses a full-size output buffer, and misses the C buffer-size, partial-fragment, next-frame, next-index, and small-fragment exception checks.
* Phase 5A fix note: Port dataqueue_prepare_test and dataqueue_verify_test behavior: per-case stream/offset/data/buffer sizing, constrained output buffers, expected cursor updates, output bytes, and the C small-fragment zero-output exception.
* Phase 5B analysis: Rust now mirrors the C dataqueue_prepare_test/dataqueue_verify_test cases, including constrained buffers and the small-fragment zero-output exception.
* Phase 5B fix note: Added per-case preparation/expected-output helper and changed dataqueue_copy to verify cursor updates, output length, output bytes, and constrained buffer behavior.

### C test body
```c
{
    int ret = 0;
    picoquic_packet_t packet;
    uint8_t data[1536];
    uint8_t output[1536];
    size_t length_max = 1536;

    for (int case_opt = 0; ret == 0 && case_opt < 5; case_opt++) {
        int has_length = (case_opt & 1) == 0;
        int has_fin = (case_opt & 2) == 2;

        for (int basic_case = 1; ret == 0 && basic_case <= 6; basic_case++) {
            size_t next_frame = 0;
            size_t next_index = 0;
            size_t buffer_size = 0;
            size_t frame_length = 0;

            ret = dataqueue_prepare_test(basic_case, has_length, has_fin, &packet,
                &next_frame, &next_index, &buffer_size, &frame_length, data, length_max);

            if (ret != 0) {
                DBG_PRINTF("Prepare test fails for case %d, option &x", basic_case, case_opt);
                ret = -1;
            }
            else {
                uint8_t* next_byte = picoquic_copy_stream_frame_for_retransmit(
                    NULL, &packet, output, output + buffer_size);
                if (next_byte == NULL) {
                    DBG_PRINTF("Copy stream frame fails for case %d, option &x", basic_case, case_opt);
                    ret = -1;
                }
                else
                {
                    size_t output_length = next_byte - output;
                    if (dataqueue_verify_test(&packet, next_frame, next_index, frame_length, data, output, output_length) != 0) {
                        if (frame_length >= PICOQUIC_MIN_STREAM_DATA_FRAGMENT || output_length != 0) {
                            DBG_PRINTF("Verify data fails for case %d, option &x", basic_case, case_opt);
                            ret = -1;
                        }
                    }
                }
            }
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn dataqueue_copy() {
    for case_opt in 0..5i32 {
        let has_length = (case_opt & 1) == 0;
        let has_fin = (case_opt & 2) == 2;
        for basic_case in 1..=6i32 {
            dataqueue_copy_test_one(basic_case, has_length, has_fin).expect("dataqueue_copy");
        }
    }
}
```
