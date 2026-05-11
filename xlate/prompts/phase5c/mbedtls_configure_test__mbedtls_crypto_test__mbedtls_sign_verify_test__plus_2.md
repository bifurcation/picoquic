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

## `picoquictest/mbedtls_test.c:mbedtls_configure_test`
* C test-table name: `mbedtls_configure`
* C entry function: `mbedtls_configure_test`
* Rust test: `mbedtls_configure`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:789-805`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `fixed`
* Phase 5A analysis: The TLS registry now exposes a provider snapshot and registers mbedTLS verify/public-key provider slots, letting the Rust test check the same provider identity table shape as C.
* Phase 5A fix note: Added TlsProviderSnapshot/TlsProviderKind, registered mbedTLS verifier/public-key provider slots, and updated mbedtls_configure to assert mbedTLS cipher-suite, key-exchange, key-loader, random, and verifier providers.
* Phase 5B analysis: Still blocked under the corrected Phase 5B standard: the Rust test exists, but a faithful C-level registry test needs mbedTLS provider identity for cipher/key-exchange slots, random/private-key hooks, and certificate-verifier callbacks; that surface is private or absent from the owned Rust test file. This is API/test-harness surface, not a Phase 5C runtime failure.
* Phase 5B fix note: No Rust test changes; appended required command log entry.

### C test body
```c
{
    int ret = 0;
    int cipher_suite_match_low = 0;
    int cipher_suite_match_high = 0;
    int key_exchange_max = 0;
    ptls_cipher_suite_t* targets[3] = {
        &ptls_mbedtls_aes128gcmsha256,
        &ptls_mbedtls_aes256gcmsha384,
        &ptls_mbedtls_chacha20poly1305sha256
    };
    ptls_key_exchange_algorithm_t* exchange[3] = {
        &ptls_mbedtls_secp256r1, &ptls_mbedtls_x25519 };

    /* Cleanup previous initiation of the TLS API and do it cleanly. */
    picoquic_tls_api_reset(TLS_API_INIT_FLAGS_NO_OPENSSL |
        TLS_API_INIT_FLAGS_NO_FUSION);
    /* Verify that the negotiated parameters have the expected value */
    for (int i = 0; i < PICOQUIC_CIPHER_SUITES_NB_MAX; i++) {
        for (int j = 0; j < 3; j++) {
            if (targets[j] == picoquic_cipher_suites[i].high_memory_suite) {
                cipher_suite_match_high |= (1 << j);
            }
            if (targets[j] == picoquic_cipher_suites[i].low_memory_suite) {
                cipher_suite_match_low |= (1 << j);
            }
        }
        if (cipher_suite_match_low == 0x7 && cipher_suite_match_high == 0x7) {
            break;
        }
    }
    if (cipher_suite_match_low != 0x7 || cipher_suite_match_high != 0x7) {
        DBG_PRINTF("Suites registration test fails, expected 0x%x, 0x%x, got 0x%x, 0x%x",
            7, 7, cipher_suite_match_low, cipher_suite_match_high);
        ret = -1;
    }

    if (picoquic_key_exchange_secp256r1[0] != &ptls_mbedtls_secp256r1) {
        DBG_PRINTF("%s", "key_exchange_secp256r1 does not match");
        ret = -1;
    }

    for (int i = 0; i < PICOQUIC_KEY_EXCHANGES_NB_MAX; i++) {
        for (int j = 0; j < 2; j++) {
            if (exchange[j] == picoquic_key_exchanges[i]) {
                key_exchange_max |= (1 << j);
            }
            if (key_exchange_max == 0x3) {
                break;
            }
        }
    }

    if (key_exchange_max != 0x3) {
        DBG_PRINTF("Exchange registration test fails, expected 0x%x, got 0x%x",
            7, key_exchange_max);
        ret = -1;
    }

    if (picoquic_set_private_key_from_file_fn != ptls_mbedtls_load_private_key ||
        picoquic_dispose_sign_certificate_fn != ptls_mbedtls_dispose_sign_certificate ||
        picoquic_get_certs_from_file_fn != picoquic_mbedtls_get_certs_from_file) {
        DBG_PRINTF("%s", "At least one private key function does not match mbedtls");
        ret = -1;
    }

    if (picoquic_get_certificate_verifier_fn != picoquic_mbedtls_get_certificate_verifier ||
        picoquic_dispose_certificate_verifier_fn != ptls_mbedtls_dispose_verify_certificate) {
        DBG_PRINTF("%s", "At least one verify certs function does not match mbedtls");
        ret = -1;
    }

    if (picoquic_crypto_random_provider_fn != ptls_mbedtls_random_bytes) {
        DBG_PRINTF("%s", "Crypto random provider does not match mbedtls");
        ret = -1;
    }

    /* Reset configuration to default after test */
    picoquic_tls_api_reset(0);

    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls_configure() {
    // Bring up the TLS provider registry with only the mbedTLS backend.
    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL | TLS_API_INIT_FLAGS_NO_FUSION);
    assert_mbedtls_provider_snapshot();

    mbedtls_test_random().expect("registered random provider");
    mbedtls_test_load_one_der_key("certs/rsa/key.pem").expect("registered private key loader");
    mbedtls_test_sign_verify_one(
        "certs/rsa/key.pem",
        "certs/rsa/cert.pem",
        "certs/test-ca.crt",
        "rsa.test.example.com",
    )
    .expect("registered certificate verifier");

    reset_tls_api(0);
}
```

## `picoquictest/mbedtls_test.c:mbedtls_crypto_test`
* C test-table name: `mbedtls_crypto`
* C entry function: `mbedtls_crypto_test`
* Rust test: `mbedtls_crypto`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:694-704`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `fixed`
* Phase 5A analysis: The Rust test now has callable ptls_mbedtls_init/free equivalents and uses them around the crypto round-trip checks.
* Phase 5A fix note: Added ptls_mbedtls_init/free wrappers and used an mbedTLS runtime guard in mbedtls_crypto.
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
    let _runtime = MbedTlsRuntimeGuard::init();
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

## `picoquictest/mbedtls_test.c:mbedtls_sign_verify_test`
* C test-table name: `mbedtls_sign_verify`
* C entry function: `mbedtls_sign_verify_test`
* Rust test: `mbedtls_sign_verify`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:746-783`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `fixed`
* Phase 5A analysis: The Rust sign/verify test now has an explicit mbedTLS runtime surface and remains a compiling runnable test over the same key/cert/name cases as C.
* Phase 5A fix note: Added the mbedTLS runtime guard to mbedtls_sign_verify.
* Phase 5B analysis: Still blocked under the corrected Phase 5B standard: the Rust test is present and covers the five C fixture cases, but its helper only reads PEM fixtures and compares synthetic hashes. The crate exposes only registry-level mbedTLS stubs, with no mbedTLS signer, certificate verifier, server-name verification, or signature-verification callback surface needed to express the C API-level contract. This is an API-surface gap, not a Phase 5C runtime failure.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_RSA_KEY, ASSET_RSA_CERT, ASSET_TEST_CA, ASSET_RSA_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP256R1_KEY, ASSET_SECP256R1_CERT, ASSET_TEST_CA, ASSET_SECP256R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP384R1_KEY, ASSET_SECP384R1_CERT, ASSET_TEST_CA, ASSET_SECP384R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP521R1_KEY, ASSET_SECP521R1_CERT, ASSET_TEST_CA, ASSET_SECP521R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP256R1_PKCS8_KEY, ASSET_SECP256R1_PKCS8_CERT, ASSET_TEST_CA, ASSET_SECP256R1_PKCS8_NAME, 0, 0);
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }
    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls_sign_verify() {
    let _runtime = MbedTlsRuntimeGuard::init();
    mbedtls_test_sign_verify_one(
        "certs/rsa/key.pem",
        "certs/rsa/cert.pem",
        "certs/test-ca.crt",
        "rsa.test.example.com",
    )
    .expect("rsa sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1/key.pem",
        "certs/secp256r1/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp384r1/key.pem",
        "certs/secp384r1/cert.pem",
        "certs/test-ca.crt",
        "secp384r1.test.example.com",
    )
    .expect("secp384r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp521r1/key.pem",
        "certs/secp521r1/cert.pem",
        "certs/test-ca.crt",
        "secp521r1.test.example.com",
    )
    .expect("secp521r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1-pkcs8/key.pem",
        "certs/secp256r1-pkcs8/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1-pkcs8 sign_verify");
}
```

## `picoquictest/skip_frame_test.c:logger_test`
* C test-table name: `logger`
* C entry function: `logger_test`
* Rust test: `logger`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:4477-4479`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `fixed`
* Phase 5A analysis: Added a callable textlog_frames wrapper and extended logger_test to exercise frame corpus logging, random no-Unknown scanning, bad-frame logging, and fuzz logging.
* Phase 5A fix note: Exposed crate::textlog::textlog_frames and updated skip_frame logger_test with textlog frame, random packet, error-frame, and fuzz logging loops.
* Phase 5B analysis: Prior set_textlog/backend runtime failures are Phase 5C, but the C test directly calls picoquic_textlog_frames for the frame corpus and Rust has no faithful textlog frame API/harness surface to call from skip_frame.rs.
* Phase 5B fix note: 

### C test body
```c
{
    FILE* F = NULL;
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t fuzz_buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint64_t random_context = 0xF00BAB;
    struct sockaddr_in6 saddr = { 0 };
    picoquic_cnx_t * cnx = NULL;
    picoquic_quic_t * quic = NULL;
    uint64_t simulated_time = 123456789;
    uint64_t running_sum = 0;

    quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    saddr.sin6_family = AF_INET6;
    saddr.sin6_port = 443;
    memset(&saddr.sin6_addr, 0x20, 16);

    if (quic == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }
    else if ((cnx = picoquic_create_cnx(quic, logger_test_cid, logger_test_cid, (struct sockaddr*)&saddr,
        simulated_time, 0, "test-sni", "test-alpn", 1)) == NULL) {
        DBG_PRINTF("%s", "Cannot create CNX context\n");
        ret = -1;
    }
    else if (picoquic_set_textlog(quic, log_test_file) != 0) {
        DBG_PRINTF("failed to open file:%s\n", log_test_file);
        ret = -1;
    }
    else {
        for (size_t i = 0; i < nb_test_skip_list; i++) {
            picoquic_textlog_frames(quic->F_log, 0, test_skip_list[i].val, test_skip_list[i].len);
        }
        for (size_t i = 0; i < nb_test_frame_error_list; i++) {
            picoquic_textlog_frames(quic->F_log, 0, test_frame_error_list[i].val, test_frame_error_list[i].len);
        }
        fprintf(quic->F_log, "\n");
        picoquic_log_tls_ticket(cnx,
            log_test_ticket, (uint16_t) sizeof(log_test_ticket));

        picoquic_log_app_message(cnx, "%s.", "This is an app message test");
        picoquic_log_app_message(cnx, "This is app message test #%d, severity %d.", 1, 2);

        fprintf(quic->F_log, "\n");
        logger_test_packets(cnx);
        logger_test_pdus(quic, cnx);

        quic->F_log = picoquic_file_close(quic->F_log);
    }

    if (ret == 0) {
        char log_test_ref[512];

        ret = picoquic_get_input_path(log_test_ref, sizeof(log_test_ref), picoquic_solution_dir, LOG_TEST_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the log ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(log_test_file, log_test_ref);
        }
    }

    /* Create a set of randomized packets. Verify that they can be logged without
     * causing the dreaded "Unknown frame" message */

    for (size_t i = 0; ret == 0 && i < 100; i++) {
        char log_line[1024];
        size_t bytes_max = format_random_packet(buffer, sizeof(buffer), &random_context, -1);

        if (picoquic_set_textlog(quic, log_packet_test_file) != 0) {
            DBG_PRINTF("failed to open file:%s\n", log_packet_test_file);
            ret = -1;
        }
        else {
            ret &= fprintf(quic->F_log, "Log packet test #%d\n", (int)i);
            picoquic_textlog_frames(quic->F_log, 0, buffer, bytes_max);
            quic->F_log = picoquic_file_close(quic->F_log);
        }

        if ((F = picoquic_file_open(log_packet_test_file, "r")) == NULL) {
            DBG_PRINTF("failed to open file:%s\n", log_packet_test_file);
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        } else {
            while (fgets(log_line, (int)sizeof(log_line), F) != NULL) {
                /* skip blanks */
                size_t byte_index = 0;

                while (byte_index < sizeof(log_line) &&
                    (log_line[byte_index] == ' ' || log_line[byte_index] == '\t')) {
                    byte_index++;
                }

                if (byte_index + 7u < sizeof(log_line) &&
                    memcmp(&log_line[byte_index], "Unknown", 7) == 0)
                {
                    DBG_PRINTF("Packet log test #%d failed, unknown frame.\n", (int)i);
                    ret = -1;
                    break;
                }
            }
            (void)picoquic_file_close(F);
        }
    }

    /* Log a series of known bad packets  */
    for (size_t i = 0; ret == 0 && i < nb_test_frame_error_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            uint8_t extra_bytes[4] = { 0, 0, 0, 0 };
            size_t bytes_max = 0;

            if (picoquic_set_textlog(quic, log_error_test_file) != 0) {
                DBG_PRINTF("failed to open file:%s\n", log_error_test_file);
                ret = -1;
                break;
            }
            fprintf(quic->F_log, "Running_sum: %" PRIx64 "\n", running_sum);
            memcpy(buffer, test_frame_error_list[i].val, test_frame_error_list[i].len);
            bytes_max = test_frame_error_list[i].len;
            if (test_frame_error_list[i].must_be_last == 0 && sharp_end == 0) {
                /* add some padding to check that the end of frame is detected properly */
                memcpy(buffer + bytes_max, extra_bytes, sizeof(extra_bytes));
                bytes_max += sizeof(extra_bytes);
            }

            picoquic_textlog_frames(quic->F_log, 0, buffer, bytes_max);

            quic->F_log = picoquic_file_close(quic->F_log);
            running_sum += picoquic_sum_text_file(log_error_test_file);
        }
    }

    /* Do a minimal fuzz test */
    for (size_t i = 0; ret == 0 && i < 100; i++) {
        size_t bytes_max = format_random_packet(buffer, sizeof(buffer), &random_context, -1);

        if (picoquic_set_textlog(quic, log_fuzz_test_file) != 0) {
            DBG_PRINTF("failed to open file:%s\n", log_fuzz_test_file);
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        }

        ret &= (fprintf(quic->F_log, "Log fuzz test #%d, sum: %" PRIx64 "\n",
            (int)i, running_sum) > 0);
        picoquic_textlog_frames(quic->F_log, 0, buffer, bytes_max);

        /* Attempt to log fuzzed packets, and hope nothing crashes */
        for (size_t j = 0; j < 100; j++) {
            ret &= fprintf(quic->F_log, "Log fuzz test #%d, packet %d\n", (int)i, (int)j);
            fflush(quic->F_log);
            skip_test_fuzz_packet(fuzz_buffer, buffer, bytes_max, &random_context);
            picoquic_textlog_frames(quic->F_log, 0, fuzz_buffer, bytes_max);
        }
        quic->F_log = picoquic_file_close(quic->F_log);
        running_sum += picoquic_sum_text_file(log_fuzz_test_file);
    }

    if (quic != NULL) {
        picoquic_free(quic);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn logger() {
    run_logger_test().expect("logger_test");
}
```

## `picoquictest/spinbit_test.c:spinbit_bad_test`
* C test-table name: `spinbit_bad`
* C entry function: `spinbit_bad_test`
* Rust test: `spinbit_bad`
* Expected Rust file: `rs/fq/src/tests/spinbit.rs`
* Current Rust span: `rs/fq/src/tests/spinbit.rs:170-179`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `fixed`
* Phase 5A analysis: Added raw spinbit policy setters so out-of-range C enum values 123456 and 123455 can be represented and rejected by the Rust test.
* Phase 5A fix note: Added SpinbitVersion::from_raw, raw Quic/Connection spinbit setters, and updated spinbit_bad to call the raw invalid values.
* Phase 5B analysis: C checks raw out-of-range spinbit policy values 123456 and 123455 on the default and per-connection setter paths. The current Rust API uses closed SpinbitVersion values, so a faithful compiling runnable Rust test for those raw invalid inputs cannot be written without a raw conversion/setter API. Default SpinbitVersion::On rejection is a Phase 5C implementation note, not the blocker.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    if (spinbit_test_one(picoquic_spinbit_on, 123456) == 0 ||
        spinbit_test_one(123455, picoquic_spinbit_null) == 0) {
        ret = -1;
    }
    return ret;
}
```

### Current Rust test body
```rust
fn spinbit_bad() {
    assert!(
        spinbit_test_one_raw(SpinbitVersion::On as u64, 123_456).is_err(),
        "expected invalid server spinbit policy to be rejected"
    );
    assert!(
        spinbit_test_one_raw(123_455, SpinbitVersion::Null as u64).is_err(),
        "expected invalid client spinbit policy to be rejected"
    );
}
```
