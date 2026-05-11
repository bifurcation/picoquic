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

## `picoquictest/tls_api_test.c:tls_zero_share_test`
* C test-table name: `tls_zero_share`
* C entry function: `tls_zero_share_test`
* Rust test: `tls_zero_share`
* C source: `picoquictest/tls_api_test.c:4067-4088`
* Rust source: `rs/fq/src/tests/tls_api.rs:1469-1471`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 1, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
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
fn tls_zero_share() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_zero_share");
}
```

## `picoquictest/wifitest.c:wifi_reno_test`
* C test-table name: `wifi_reno`
* C entry function: `wifi_reno_test`
* Rust test: `wifi_reno`
* C source: `picoquictest/wifitest.c:245-252`
* Rust source: `rs/fq/src/tests/wifitest.rs:248-251`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_newreno_algorithm, 2800000);
    int ret = wifi_test_one(wifi_test_reno, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_reno() {
    let spec = default_spec("newreno", SUSPENSION_BASIC, 2_800_000);
    wifi_test_one(WIFI_TEST_RENO, &spec).expect("wifi_reno");
}
```
