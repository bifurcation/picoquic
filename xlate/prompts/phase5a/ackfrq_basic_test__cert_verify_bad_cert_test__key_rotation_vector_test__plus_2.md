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

## `picoquictest/ack_frequency_test.c:ackfrq_basic_test`
* C test-table name: `ackfrq_basic`
* C entry function: `ackfrq_basic_test`
* Rust test: `ackfrq_basic`
* C source: `picoquictest/ack_frequency_test.c:178-192`
* Rust source: `rs/fq/src/tests/ack_frequency.rs:174-187`

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

### Rust test body
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

## `picoquictest/cert_verify_test.c:cert_verify_bad_cert_test`
* C test-table name: `cert_verify_bad_cert`
* C entry function: `cert_verify_bad_cert_test`
* Rust test: `cert_verify_bad_cert`
* C source: `picoquictest/cert_verify_test.c:228-235`
* Rust source: `rs/fq/src/tests/cert_verify.rs:42-50`

### C test body
```c
{
    int ret = cert_verify_test_one(0, CERT_VERIFY_RSA_BAD_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Rust test body
```rust
fn cert_verify_bad_cert() {
    cert_verify_test_one(
        false,
        Some(TEST_FILE_SERVER_BAD_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_SNI),
    );
}
```

## `picoquictest/cleartext_aead_test.c:key_rotation_vector_test`
* C test-table name: `key_rotation_vector`
* C entry function: `key_rotation_vector_test`
* Rust test: `key_rotation_vector`
* C source: `picoquictest/cleartext_aead_test.c:1081-1115`
* Rust source: `rs/fq/src/tests/cleartext_aead.rs:866-921`

### C test body
```c
{
    int ret = 0;
    ptls_cipher_suite_t* key_rotation_test_suites[4] = {
        NULL, NULL, NULL, NULL };
    uint8_t new_secret[PTLS_MAX_DIGEST_SIZE];

    key_rotation_test_suites[0] = (ptls_cipher_suite_t*)picoquic_get_cipher_suite_by_id_v(PICOQUIC_AES_256_GCM_SHA384, 0);
    key_rotation_test_suites[1] = (ptls_cipher_suite_t*)picoquic_get_cipher_suite_by_id_v(PICOQUIC_AES_128_GCM_SHA256, 0);
    key_rotation_test_suites[2] = (ptls_cipher_suite_t*)picoquic_get_cipher_suite_by_id_v(PICOQUIC_CHACHA20_POLY1305_SHA256, 0);
    key_rotation_test_suites[3] = NULL;

    memcpy(new_secret, key_rotation_test_init, PTLS_MAX_DIGEST_SIZE);

    for (int i = 0; ret == 0 && key_rotation_test_suites[i] != NULL; i++) {
        memset(new_secret, 0, sizeof(new_secret));
        memcpy(new_secret, key_rotation_test_init, key_rotation_test_suites[i]->hash->digest_size);
        /* TODO: update to use the test vector of draft 25 and up */
        ret = picoquic_rotate_app_secret(key_rotation_test_suites[i], new_secret, PICOQUIC_LABEL_V1_TRAFFIC_UPDATE);
        if (ret != 0) {
            DBG_PRINTF("Cannot rotate secret[%d], ret=%x\n", i, ret);
        }
        else if (key_rotation_test_suites[i]->hash->digest_size != key_rotation_test_target_size[i]) {
            DBG_PRINTF("Wrong size for secret[%d], %d vs %d\n", i, (int) key_rotation_test_suites[i]->hash->digest_size, 
                (int) key_rotation_test_target_size[i]);
            ret = -1;
        }
        else if (memcmp(new_secret, key_rotation_test_target[i], key_rotation_test_target_size[i]) != 0) {
            DBG_PRINTF("Values don't match for secret[%d]\n", i);
            ret = -1;
        }
    }

    return ret;
}
```

### Rust test body
```rust
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
```

## `picoquictest/cnxstress.c:cnx_stress_unit_test`
* C test-table name: `cnx_stress`
* C entry function: `cnx_stress_unit_test`
* Rust test: `cnx_stress`
* C source: `picoquictest/cnxstress.c:967-973`
* Rust source: `rs/fq/src/tests/cnxstress.rs:965-967`

### C test body
```c
{
    return cnx_stress_do_test(120000000, 100, 0);
}
```

### Rust test body
```rust
fn cnx_stress() {
    cnx_stress_do_test(120_000_000, 100, false).expect("cnx_stress_do_test");
}
```

## `picoquictest/congestion_test.c:app_limit_cc_test`
* C test-table name: `app_limit_cc`
* C entry function: `app_limit_cc_test`
* Rust test: `app_limit_cc`
* C source: `picoquictest/congestion_test.c:869-898`
* Rust source: `rs/fq/src/tests/congestion.rs:950-961`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgos[] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_bbr_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr1_algorithm
    };
    uint64_t max_completion_times[] = {
        22000000,
        23500000,
        22000000,
        21000000,
        25000000,
        25000000
    };
    int ret = 0;

    for (size_t i = 0; i < sizeof(ccalgos) / sizeof(picoquic_congestion_algorithm_t*); i++) {
        ret = app_limit_cc_test_one(ccalgos[i], max_completion_times[i]);
        if (ret != 0) {
            DBG_PRINTF("Appplication limited congestion test fails for <%s>", ccalgos[i]->congestion_algorithm_id);
            break;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn app_limit_cc() {
    let algo_names = ["newreno", "cubic", "dcubic", "bbr", "fastcc", "bbr1"];
    let max_completion_times: [u64; 6] = [
        22_000_000, 23_500_000, 22_000_000, 21_000_000, 25_000_000, 25_000_000,
    ];

    for (name, &max_time) in algo_names.iter().zip(max_completion_times.iter()) {
        let ccalgo =
            get_congestion_algorithm(name).unwrap_or_else(|| panic!("cc algo not found: {name}"));
        app_limit_cc_test_one(ccalgo, max_time);
    }
}
```
