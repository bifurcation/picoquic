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

## `picoquictest/edge_cases.c:eccf_corrupted_file_fuzz_test`
* C test-table name: `eccf_corrupted_fuzz`
* C entry function: `eccf_corrupted_file_fuzz_test`
* Rust test: `eccf_corrupted_fuzz`
* C source: `picoquictest/edge_cases.c:403-416`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1115-1140`

### C test body
```c
{
    int ret = 0;
    FILE* F = picoquic_file_open("ECCF_Fuzz_report.csv", "w");
    if (F == NULL) {
        ret = -1;
    }
    else {
        (void)fprintf(F, "Seed_hex, Seed, Ret, Elapsed\n");
        eccf_corrupted_file_fuzz(50, 0, F);
        picoquic_file_close(F);
    }
    return ret;
}
```

### Rust test body
```rust
fn eccf_corrupted_fuzz() {
    use std::io::Write as _;

    let mut report = std::fs::File::create("ECCF_Fuzz_report.csv").expect("fuzz report");
    writeln!(report, "Seed_hex, Seed, Ret, Elapsed").expect("fuzz header");
    let mut random_context = 0x1234_5678_8765_4321u64;

    for _ in 0..50 {
        let mut simulated_time = Instant::from_ticks(0);
        let initial_losses = random_context & 0x00ff_ffff_ff86;
        let _ = test_random(&mut random_context);

        let result = edge_case_prepare(0xcf, false, &mut simulated_time, initial_losses, 40)
            .and_then(|mut test_ctx| {
                edge_case_complete(&mut test_ctx, &mut simulated_time, 15_000_000)
            });
        if let Err(err) = result {
            writeln!(
                report,
                "0x{initial_losses:x}, {initial_losses}, {err:?}, {}",
                simulated_time.ticks()
            )
            .expect("fuzz row");
        }
    }
}
```

## `picoquictest/edge_cases.c:reset_stream_at_basic_test`
* C test-table name: `reset_stream_at_basic`
* C entry function: `reset_stream_at_basic_test`
* Rust test: `reset_stream_at_basic`
* C source: `picoquictest/edge_cases.c:2013-2016`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1510-1512`

### C test body
```c
{
    return reset_stream_at_test_one(rsat_basic);
}
```

### Rust test body
```rust
fn reset_stream_at_basic() {
    reset_stream_at_test_one(ResetStreamAtSpec::Basic).expect("reset_stream_at_basic");
}
```

## `picoquictest/high_latency_test.c:high_latency_cubic_test`
* C test-table name: `high_latency_cubic`
* C entry function: `high_latency_cubic_test`
* Rust test: `high_latency_cubic`
* C source: `picoquictest/high_latency_test.c:306-311`
* Rust source: `rs/fq/src/tests/high_latency.rs:298-314`

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Rust test body
```rust
fn high_latency_cubic() {
    let latency = 5_000_000u64;
    let cubic = get_congestion_algorithm("cubic").expect("cubic");
    high_latency_one(
        0xcb,
        cubic,
        HILAT_SCENARIO_100MB,
        200_000_000,
        latency,
        10,
        10,
        0,
        false,
        false,
        false,
    );
}
```

## `picoquictest/l4s_test.c:l4s_prague_test`
* C test-table name: `l4s_prague`
* C entry function: `l4s_prague_test`
* Rust test: `l4s_prague`
* C source: `picoquictest/l4s_test.c:143-150`
* Rust source: `rs/fq/src/tests/l4s.rs:161-164`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgo = picoquic_prague_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 4100000, 9, 4500, 0, NULL);

    return ret;
}
```

### Rust test body
```rust
fn l4s_prague() {
    let ccalgo = get_congestion_algorithm("prague").expect("prague cc algo");
    l4s_congestion_test(ccalgo, true, 4_100_000, 9, 4_500, &[]);
}
```

## `picoquictest/mbedtls_test.c:mbedtls_retrieve_pubkey_test`
* C test-table name: `mbedtls_retrieve_pubkey`
* C entry function: `mbedtls_retrieve_pubkey_test`
* Rust test: `mbedtls_retrieve_pubkey`
* C source: `picoquictest/mbedtls_test.c:870-898`
* Rust source: `rs/fq/src/tests/mbedtls.rs:447-456`

### C test body
```c
{
    int ret = 0;
    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = test_retrieve_pubkey_one(ASSET_RSA_KEY, ASSET_RSA_CERT);
        }

        if (ret == 0) {
            ret = test_retrieve_pubkey_one(ASSET_SECP256R1_KEY, ASSET_SECP256R1_CERT);
        }

        if (ret == 0) {
            ret = test_retrieve_pubkey_one(ASSET_SECP384R1_KEY, ASSET_SECP384R1_CERT);
        }

        if (ret == 0) {
            ret = test_retrieve_pubkey_one(ASSET_SECP521R1_KEY, ASSET_SECP521R1_CERT);
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }

    return ret;
}
```

### Rust test body
```rust
fn mbedtls_retrieve_pubkey() {
    mbedtls_test_retrieve_pubkey_one("certs/rsa/key.pem", "certs/rsa/cert.pem")
        .expect("rsa pubkey");
    mbedtls_test_retrieve_pubkey_one("certs/secp256r1/key.pem", "certs/secp256r1/cert.pem")
        .expect("secp256r1 pubkey");
    mbedtls_test_retrieve_pubkey_one("certs/secp384r1/key.pem", "certs/secp384r1/cert.pem")
        .expect("secp384r1 pubkey");
    mbedtls_test_retrieve_pubkey_one("certs/secp521r1/key.pem", "certs/secp521r1/cert.pem")
        .expect("secp521r1 pubkey");
}
```
