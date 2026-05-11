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

## `picoquictest/hashtest.c:picohash_embedded_test`
* C test-table name: `picohash_embedded`
* C entry function: `picohash_embedded_test`
* Rust test: `picohash_embedded`
* C source: `picoquictest/hashtest.c:202-205`
* Rust source: `rs/fq/src/tests/hashtest.rs:86-132`

### C test body
```c
{
    return(picohash_test_one(1));
}
```

### Rust test body
```rust
fn picohash_embedded() {
    use crate::hash::HashTable;

    let hash_seed: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut t: HashTable<u64, ()> =
        HashTable::with_seed(32, &hash_seed).expect("create hash table with seed");

    assert_eq!(t.len(), 0);

    for i in (1u64..10).step_by(2) {
        assert!(t.insert(i, ()).is_ok(), "insert({i}) failed");
    }
    assert_eq!(t.len(), 5);

    for i in (1u64..10).step_by(2) {
        assert!(t.lookup(&i).is_some(), "lookup({i}) failed");
    }

    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.insert(key, ()).is_ok(), "insert({key}) failed");
        }
    }
    assert_eq!(t.len(), 11);

    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.lookup(&key).is_some(), "lookup({key}) failed");
        }
    }

    for i in (0u64..=10).step_by(2) {
        assert!(t.lookup(&i).is_none(), "lookup({i}) returned invalid item");
    }

    for i in (1u64..10).step_by(4) {
        let tok = t.lookup(&i).expect("pre-delete lookup");
        t.remove(tok);
    }
    assert_eq!(t.len(), 8);

    for i in (1u64..10).step_by(4) {
        assert!(t.lookup(&i).is_none(), "deleted value {i} still found");
    }
}
```

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

## `picoquictest/mediatest.c:mediatest_video_test`
* C test-table name: `mediatest_video`
* C entry function: `mediatest_video_test`
* Rust test: `mediatest_video`
* C source: `picoquictest/mediatest.c:1342-1353`
* Rust source: `rs/fq/src/tests/mediatest.rs:229-237`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    ret = mediatest_one(mediatest_video, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_video() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video, &spec).expect("mediatest_video");
}
```

## `picoquictest/memlog_test.c:memlog_test`
* C test-table name: `memlog`
* C entry function: `memlog_test`
* Rust test: `memlog`
* C source: `picoquictest/memlog_test.c:159-171`
* Rust source: `rs/fq/src/tests/memlog.rs:137-141`

### C test body
```c
{
    int ret = memlog_test_one(0, MEMLOG_FILE, 0);

    if (ret == 0) {
        ret = memlog_test_one(1, MEMLOG_FILE_MP, 0);
    }

    if (ret == 0) {
        ret = memlog_test_one(1, MEMLOG_FILE_BAD, 1);
    }
    return ret;
}
```

### Rust test body
```rust
fn memlog() {
    memlog_test_one(false, MEMLOG_FILE, false);
    memlog_test_one(true, MEMLOG_FILE_MP, false);
    memlog_test_one(true, MEMLOG_FILE_BAD, true);
}
```

## `picoquictest/multipath_test.c:monopath_0rtt_loss_test`
* C test-table name: `monopath_0rtt_loss`
* C entry function: `monopath_0rtt_loss_test`
* Rust test: `monopath_0rtt_loss_2`
* C source: `picoquictest/multipath_test.c:1708-1723`
* Rust source: `rs/fq/src/tests/multipath.rs:1362-1372`

### C test body
```c
{
    int ret = 0;

    for (unsigned int i = 1; ret == 0 && i < 16; i++) {
        zero_rtt_test_t zrt = { 0 };
        zrt.early_loss = 1ull << i;
        zrt.do_multipath = 1;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Monopath 0 RTT test fails when packet #%d is lost.\n", i);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn monopath_0rtt_loss_2() {
    for i in 1u32..16 {
        let zrt = ZeroRttTest {
            early_loss: 1u64 << i,
            do_multipath: true,
            ..ZeroRttTest::default()
        };
        zero_rtt_test_one(&zrt)
            .unwrap_or_else(|_| panic!("monopath_0rtt_loss_2 fails at packet #{i}"));
    }
}
```
