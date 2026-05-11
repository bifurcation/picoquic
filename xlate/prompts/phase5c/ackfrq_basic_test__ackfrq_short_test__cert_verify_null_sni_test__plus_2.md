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

## `picoquictest/ack_frequency_test.c:ackfrq_basic_test`
* C test-table name: `ackfrq_basic`
* C entry function: `ackfrq_basic_test`
* Rust test: `ackfrq_basic`
* Expected Rust file: `rs/fq/src/tests/ack_frequency.rs`
* Current Rust span: `rs/fq/src/tests/ack_frequency.rs:174-187`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Entry parameters match C, but the Rust scenario verifier only closes the connection and does not perform C's stream/body completion checks before the ACK-frequency assertions.
* Phase 5A fix note: Make the Rust scenario verify helper check stream byte counts/received flags and callback errors like C tls_api_one_scenario_verify; target_time is 0 here, so no time-limit check is needed for this entry.
* Phase 5B analysis: Blocked: the Rust verifier cannot faithfully add C tls_api_one_scenario_verify checks because TestTlsApiCtx has no client/server callback error state, normal stream-data callbacks are not dispatched to populate q/r receive counts or received flags, and accepted server connections do not inherit default callbacks. The test also currently fails before scenario setup because the cubic congestion registry is not initialized.
* Phase 5B fix note: 

### C test body
```c
{
    ackfrq_test_spec_t spec = { 0 };
    spec.test_id = ackfrq_test_basic;
    spec.latency = 10000;
    spec.picosec_per_byte_up = 80000;
    spec.picosec_per_byte_down = 80000;
    spec.ccalgo = picoquic_cubic_algorithm;
    spec.max_ack_delay_remote = 6000;
    spec.max_ack_gap_remote = 40;
    spec.min_ack_delay_remote = 1000;
    spec.target_interval = 4000;

    return ackfrq_test_one(&spec);
}
```

### Current Rust test body
```rust
fn ackfrq_basic() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    ackfrq_test_one(&AckfrqTestSpec {
        latency: 10_000,
        picosec_per_byte_up: 80_000,
        picosec_per_byte_down: 80_000,
        ccalgo,
        target_time: 0,
        max_ack_delay_remote: Duration::from_ticks(6_000),
        max_ack_gap_remote: 40,
        min_ack_delay_remote: Duration::from_ticks(1_000),
        target_interval: Duration::from_ticks(4_000),
    });
}
```

## `picoquictest/ack_frequency_test.c:ackfrq_short_test`
* C test-table name: `ackfrq_short`
* C entry function: `ackfrq_short_test`
* Rust test: `ackfrq_short`
* Expected Rust file: `rs/fq/src/tests/ack_frequency.rs`
* Current Rust span: `rs/fq/src/tests/ack_frequency.rs:195-208`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Entry parameters and ACK-frequency assertions match C, but the Rust scenario verifier only closes connections and does not perform the C stream/error completion checks.
* Phase 5A fix note: Make the Rust scenario verifier check stream completion/callback errors like C tls_api_one_scenario_verify, then close; preserve target_time handling.
* Phase 5B analysis: Rust verifier now checks C-style scenario completion before close, but the Rust TLS test harness does not yet implement/update the application callback stream counters and callback error flags, so a faithful ackfrq_short fails at scenario verification.
* Phase 5B fix note: Added C-equivalent scenario verifier checks in the Rust test helper, restored completion-time handling, made close verify disconnection, and initialized the congestion-control registry for the ACK-frequency tests.

### C test body
```c
{
    ackfrq_test_spec_t spec = { 0 };
    spec.test_id = ackfrq_test_basic;
    spec.latency = 10;
    spec.picosec_per_byte_up = 80000;
    spec.picosec_per_byte_down = 80000;
    spec.ccalgo = picoquic_cubic_algorithm;
    spec.max_ack_delay_remote = 1000;
    spec.max_ack_gap_remote = 32;
    spec.min_ack_delay_remote = 1000;
    spec.target_interval = 1000;

    return ackfrq_test_one(&spec);
}
```

### Current Rust test body
```rust
fn ackfrq_short() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    ackfrq_test_one(&AckfrqTestSpec {
        latency: 10,
        picosec_per_byte_up: 80_000,
        picosec_per_byte_down: 80_000,
        ccalgo,
        target_time: 0,
        max_ack_delay_remote: Duration::from_ticks(1_000),
        max_ack_gap_remote: 32,
        min_ack_delay_remote: Duration::from_ticks(1_000),
        target_interval: Duration::from_ticks(1_000),
    });
}
```

## `picoquictest/cert_verify_test.c:cert_verify_null_sni_test`
* C test-table name: `cert_verify_null_sni`
* C entry function: `cert_verify_null_sni_test`
* Rust test: `cert_verify_null_sni`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Current Rust span: `rs/fq/src/tests/cert_verify.rs:80-88`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Inputs match the C null-SNI success case, but the Rust helper only checks tls_api_connection_loop().is_ok(); C also treats success as failure unless both client and server are ready.
* Phase 5A fix note: Make cert_verify_test_one compute success as loop_ok && client_ready && server_ready, then compare that with expect_success.
* Phase 5B analysis: Current Rust helper faithfully matches the C readiness check, but the faithful null-SNI success test fails because the Rust connection loop returns Ok while both client_ready and server_ready are false. Blocked on Rust QUIC/TLS handshake readiness behavior reaching the C-equivalent ready states.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = cert_verify_test_one(1, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, NULL);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_null_sni() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        None,
    );
}
```

## `picoquictest/cert_verify_test.c:cert_verify_rsa_test`
* C test-table name: `cert_verify_rsa`
* C entry function: `cert_verify_rsa_test`
* Rust test: `cert_verify_rsa`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Current Rust span: `rs/fq/src/tests/cert_verify.rs:93-101`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Fixture inputs match, but Rust treats tls_api_connection_loop Ok as success; the C helper also requires TEST_CLIENT_READY and TEST_SERVER_READY for positive cases.
* Phase 5A fix note: Make cert_verify_test_one fold client_ready() && server_ready() into the success condition when expect_success is true.
* Phase 5B analysis: The Rust helper now faithfully matches the C helper by requiring tls_api_connection_loop success plus client_ready() and server_ready() for the positive RSA case. The faithful test exposes an implementation-side gap: the loop returns Ok while both endpoints remain not ready.
* Phase 5B fix note: No additional source edit in this pass; rs/fq/src/tests/cert_verify.rs already contains the readiness-folded success check.

### C test body
```c
{
    int ret = cert_verify_test_one(1, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_rsa() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_SNI),
    );
}
```

## `picoquictest/mbedtls_test.c:mbedtls_configure_test`
* C test-table name: `mbedtls_configure`
* C entry function: `mbedtls_configure_test`
* Rust test: `mbedtls_configure`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:722-752`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust does not check the mbedTLS provider registry; it checks constants, local arrays, and direct helper behavior that can pass without proving mbedTLS registration.
* Phase 5A fix note: Add registry-level assertions for mbedTLS high/low cipher suites, secp256r1/x25519 key exchanges, secp256r1 special slot, private-key/cert/random provider hooks; gate or assert mbedTLS availability as appropriate.
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

    let cipher_suites = [
        crate::AES_128_GCM_SHA256,
        crate::AES_256_GCM_SHA384,
        crate::CHACHA20_POLY1305_SHA256,
    ];
    assert_eq!(
        cipher_suites,
        [0x1301, 0x1302, 0x1303],
        "cipher suite registration values"
    );

    let key_exchanges = [crate::GROUP_SECP256R1, 29u16];
    assert!(key_exchanges.contains(&crate::GROUP_SECP256R1));
    assert!(key_exchanges.contains(&29u16));

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
