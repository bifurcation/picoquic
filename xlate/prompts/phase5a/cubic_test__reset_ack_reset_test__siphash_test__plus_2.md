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

## `picoquictest/congestion_test.c:cubic_test`
* C test-table name: `cubic`
* C entry function: `cubic_test`
* Rust test: `cubic`
* C source: `picoquictest/congestion_test.c:110-113`
* Rust source: `rs/fq/src/tests/congestion.rs:733-736`

### C test body
```c
{
    return congestion_control_test(picoquic_cubic_algorithm, 3500000, 0, 0);
}
```

### Rust test body
```rust
fn cubic() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    congestion_control_test(ccalgo, 3_500_000, 0, 0);
}
```

## `picoquictest/edge_cases.c:reset_ack_reset_test`
* C test-table name: `reset_ack_reset`
* C entry function: `reset_ack_reset_test`
* Rust test: `reset_ack_reset`
* C source: `picoquictest/edge_cases.c:1198-1201`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1343-1345`

### C test body
```c
{
    return reset_repeat_test_one(reset_ack_reset);
}
```

### Rust test body
```rust
fn reset_ack_reset() {
    reset_repeat_test_one(ResetTestKind::AckReset).expect("reset_ack_reset");
}
```

## `picoquictest/hashtest.c:siphash_test`
* C test-table name: `siphash`
* C entry function: `siphash_test`
* Rust test: `siphash`
* C source: `picoquictest/hashtest.c:264-335`
* Rust source: `rs/fq/src/tests/hashtest.rs:174-201`

### C test body
```c
{
    uint8_t test[1024];
    uint8_t k[16];
    size_t test_lengths[12] = { 1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024 };
    uint64_t href[12] = {
        0xa9b786935f98d6b8,
        0x3fb64f2d81ebf107,
        0xcd34491a7b437e1b,
        0x5fbe917709286bc4,
        0xb2cc76e0f81d6e2f,
        0x09e69c0f70753651,
        0xc615b5349acc0cc2,
        0x965379fb0e26e150,
        0x85a286cfc4a62574,
        0x5f774367aeea9f83,
        0xd04ee1d420e9bc22,
        0x0a7ad6655680779e
    };
    int ret = 0;

    hash_test_init(test, sizeof(test), k, sizeof(k));
    /* Compute or check the reference siphash value */
    for (size_t i = 0; i < sizeof(test_lengths) / sizeof(size_t); i++) {
        uint64_t h = picohash_siphash(test, test_lengths[i], k);
        if (h != href[i]) {
            DBG_PRINTF("H[%zu] = %" PRIu64 "instead of %"PRIu64, i, h, href[i]);
#ifdef COMPUTING_REFERENCE_SIPASH_VALUE
            href[i] = h;
#else
            ret = -1;
            break;
#endif
        }
    }
#ifdef COMPARING_TIMES
    /* Compare execution time */
    uint64_t h;
    double sip_t[48];
    double basic_t[48];
    for (size_t lt=1; lt <= 48; lt++) {
        uint64_t start_siphash = picoquic_current_time();
        uint64_t siphash_sum = 0;
        uint64_t basic_sum = 0;
        size_t n = 0;

        for (size_t i = 0; i + lt < sizeof(test); i++) {
            h = picohash_siphash(test, lt, k);
            siphash_sum += h;
            n++;
        }
        uint64_t start_basic = picoquic_current_time();
        for (size_t i = 0; i + lt < sizeof(test); i++) {
            h = picohash_bytes(test, (uint32_t)lt, k);
            basic_sum += h;
        }
        uint64_t end_basic = picoquic_current_time();
        uint64_t siphash_time = start_basic - start_siphash;
        uint64_t basic_time = end_basic - start_basic;
        double siphash_one = ((double)siphash_time) / n;
        double basic_one = ((double)basic_time) / n;
        sip_t[lt - 1] = siphash_one;
        basic_t[lt - 1] = basic_one;
        printf("Sip hash time, %zu: %" PRIu64 ", sum: %" PRIu64", n = % zu, us=%f\n", lt, siphash_time, siphash_sum, n, siphash_one);
        printf("Basic hash time, %zu: %" PRIu64 ", sum: %" PRIu64", n = % zu, us=%f\n", lt, basic_time, basic_sum, n, basic_one);
    }
    for (int i = 0; i < 48; i++) {
        printf("%d, %f, %f\n", i + 1, basic_t[i], sip_t[i]);
    }
#endif /* COMPARING TIMES */
    return ret;
}
```

### Rust test body
```rust
fn siphash() {
    use crate::siphash::siphash;

    let mut test = [0u8; 1024];
    let mut k = [0u8; 16];
    hash_test_init(&mut test, &mut k);

    let lengths = [1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024];
    let expected: [u64; 12] = [
        0xa9b7_8693_5f98_d6b8,
        0x3fb6_4f2d_81eb_f107,
        0xcd34_491a_7b43_7e1b,
        0x5fbe_9177_0928_6bc4,
        0xb2cc_76e0_f81d_6e2f,
        0x09e6_9c0f_7075_3651,
        0xc615_b534_9acc_0cc2,
        0x9653_79fb_0e26_e150,
        0x85a2_86cf_c4a6_2574,
        0x5f77_4367_aeea_9f83,
        0xd04e_e1d4_20e9_bc22,
        0x0a7a_d665_5680_779e,
    ];

    for (i, &len) in lengths.iter().enumerate() {
        let h = siphash(&test[..len], &k);
        assert_eq!(h, expected[i], "siphash[{i}] for len={len}");
    }
}
```

## `picoquictest/l4s_test.c:l4s_bbr_updown_test`
* C test-table name: `l4s_bbr_updown`
* C entry function: `l4s_bbr_updown_test`
* Rust test: `l4s_bbr_updown`
* C source: `picoquictest/l4s_test.c:188-199`
* Rust source: `rs/fq/src/tests/l4s.rs:191-194`

### C test body
```c
{
#if defined(_WINDOWS) && !defined(_WINDOWS64)
    return 0;
#else
    picoquic_congestion_algorithm_t* ccalgo = picoquic_bbr_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 5800000, 69, 3000, nb_l4s_link_updown, l4s_link_updown);

    return ret;
#endif
}
```

### Rust test body
```rust
fn l4s_bbr_updown() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    l4s_congestion_test(ccalgo, true, 5_800_000, 69, 3_000, L4S_LINK_UPDOWN);
}
```

## `picoquictest/mbedtls_test.c:mbedtls_load_key_test`
* C test-table name: `mbedtls_load_key`
* C entry function: `mbedtls_load_key_test`
* Rust test: `mbedtls_load_key`
* C source: `picoquictest/mbedtls_test.c:691-735`
* Rust source: `rs/fq/src/tests/mbedtls.rs:428-435`

### C test body
```c
{
    int ret = 0;


    /* Initialize the PSA crypto library. */
    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_RSA_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP256R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP384R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP521R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP256R1_PKCS8_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_RSA_PKCS8_KEY);
        }
#if 0
        /* Commenting out ED25519 for now, probably not supported yet in MBEDTLS/PSA */
        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_ED25519_KEY);
        }
#endif
        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }

    return ret;
}
```

### Rust test body
```rust
fn mbedtls_load_key() {
    mbedtls_test_load_one_der_key("certs/rsa/key.pem").expect("rsa key");
    mbedtls_test_load_one_der_key("certs/secp256r1/key.pem").expect("secp256r1 key");
    mbedtls_test_load_one_der_key("certs/secp384r1/key.pem").expect("secp384r1 key");
    mbedtls_test_load_one_der_key("certs/secp521r1/key.pem").expect("secp521r1 key");
    mbedtls_test_load_one_der_key("certs/secp256r1-pkcs8/key.pem").expect("secp256r1 pkcs8 key");
    mbedtls_test_load_one_der_key("certs/rsa-pkcs8/key.pem").expect("rsa pkcs8 key");
}
```
