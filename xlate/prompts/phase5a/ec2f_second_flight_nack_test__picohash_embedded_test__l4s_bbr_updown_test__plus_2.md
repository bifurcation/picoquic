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

## `picoquictest/edge_cases.c:ec2f_second_flight_nack_test`
* C test-table name: `ec2f_second_flight`
* C entry function: `ec2f_second_flight_nack_test`
* Rust test: `ec2f_second_flight`
* C source: `picoquictest/edge_cases.c:334-361`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1098-1110`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t initial_losses = 0x1c1;
    uint8_t test_case_id = 0x2f;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 1, &simulated_time, initial_losses, 9);

    if (ret == 0) {
        if (test_ctx->cnx_client->cnx_state >= picoquic_state_ready ||
            test_ctx->cnx_server->cnx_state != picoquic_state_ready) {
            DBG_PRINTF("Unexpected state, client: %d, server: %d",
                test_ctx->cnx_client->cnx_state, test_ctx->cnx_server->cnx_state);
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = edge_case_complete(test_ctx, &simulated_time, 360000);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn ec2f_second_flight() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_losses = 0x1c1u64;
    let mut test_ctx = edge_case_prepare(0x2f, true, &mut simulated_time, initial_losses, 9)
        .expect("edge_case_prepare");
    // After 9 rounds: client must not yet be Ready; server must be Ready.
    assert!(
        !test_ctx.client_ready(),
        "client should not be ready yet after partial handshake"
    );
    assert!(test_ctx.server_ready(), "server should be ready");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 360_000).expect("edge_case_complete");
}
```

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

## `picoquictest/mediatest.c:mediatest_video2_back_test`
* C test-table name: `mediatest_video2_back`
* C entry function: `mediatest_video2_back_test`
* Rust test: `mediatest_video2_back`
* C source: `picoquictest/mediatest.c:1400-1416`
* Rust source: `rs/fq/src/tests/mediatest.rs:287-300`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.latency_average = 80000;
    spec.latency_max = 500000;
    spec.do_not_check_video2 = 1;
    ret = mediatest_one(mediatest_video2_back, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_video2_back() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 80_000,
        latency_max: 500_000,
        do_not_check_video2: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Back, &spec).expect("mediatest_video2_back");
}
```
