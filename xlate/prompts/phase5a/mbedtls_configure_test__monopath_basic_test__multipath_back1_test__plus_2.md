# Phase 5A C/Rust test correspondence audit

Review each C/Rust test pair and decide whether the Rust
`#[test]` checks the same behavior as the C test.  This is
a read-only pass: do not edit files.

You may inspect directly relevant context when needed:
Rust test helpers, fixtures, translated implementation under
`rs/fq/`, C helper functions in `picoquictest/`, constants,
and nearby tests.  Do not require byte-for-byte structure;
idiomatic Rust is fine when it preserves the C test intent,
inputs, expected results, and important edge cases.

Classify each entry as:

* `ok` when the Rust test is an acceptable translation.
* `needs_fix` when the Rust test is missing checks, checks
  materially different behavior, weakens assertions, skips
  cases the C test covers, or has placeholder-like logic.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"reviews":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short conclusion","fix_summary":"what 5B should change, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/mbedtls_test.c:mbedtls_configure_test`
* C test-table name: `mbedtls_configure`
* C entry function: `mbedtls_configure_test`
* Rust test: `mbedtls_configure`
* C source: `picoquictest/mbedtls_test.c:1199-1280`
* Rust source: `rs/fq/src/tests/mbedtls.rs:503-533`

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

### Rust test body
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

## `picoquictest/multipath_test.c:monopath_basic_test`
* C test-table name: `monopath_basic`
* C entry function: `monopath_basic_test`
* Rust test: `monopath_basic`
* C source: `picoquictest/multipath_test.c:1672-1676`
* Rust source: `rs/fq/src/tests/multipath.rs:1376-1378`

### C test body
```c
{
    return monopath_test_one(monopath_test_basic);
}
```

### Rust test body
```rust
fn monopath_basic() {
    monopath_test_one(MonopathTestId::Basic);
}
```

## `picoquictest/multipath_test.c:multipath_back1_test`
* C test-table name: `multipath_back1`
* C entry function: `multipath_back1_test`
* Rust test: `multipath_back1`
* C source: `picoquictest/multipath_test.c:1434-1442`
* Rust source: `rs/fq/src/tests/multipath.rs:1486-1488`

### C test body
```c
{
    /* TODO: investigate why 3.3 instead of 3.05 with prior implementation of multipath */
    uint64_t max_completion_microsec = 3300000;

    return  multipath_test_one(max_completion_microsec, multipath_test_back1);
}
```

### Rust test body
```rust
fn multipath_back1() {
    multipath_test_one(3_300_000, MultipathTestId::Back1);
}
```

## `picoquictest/multipath_test.c:multipath_discovery_test`
* C test-table name: `multipath_discovery`
* C entry function: `multipath_discovery_test`
* Rust test: `multipath_discovery`
* C source: `picoquictest/multipath_test.c:1518-1523`
* Rust source: `rs/fq/src/tests/multipath.rs:1534-1536`

### C test body
```c
{
    uint64_t max_completion_microsec = 2000000;

    return multipath_test_one(max_completion_microsec, multipath_test_discovery);
}
```

### Rust test body
```rust
fn multipath_discovery() {
    multipath_test_one(2_000_000, MultipathTestId::Discovery);
}
```

## `picoquictest/multipath_test.c:multipath_perf_test`
* C test-table name: `multipath_perf`
* C entry function: `multipath_perf_test`
* Rust test: `multipath_perf`
* C source: `picoquictest/multipath_test.c:1444-1450`
* Rust source: `rs/fq/src/tests/multipath.rs:1582-1584`

### C test body
```c
{
    uint64_t max_completion_microsec = 1650000;

    return  multipath_test_one(max_completion_microsec, multipath_test_perf);
}
```

### Rust test body
```rust
fn multipath_perf() {
    multipath_test_one(1_650_000, MultipathTestId::Perf);
}
```
