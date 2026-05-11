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

## `picoquictest/congestion_test.c:bbr_performance_test`
* C test-table name: `bbr_performance`
* C entry function: `bbr_performance_test`
* Rust test: `bbr_performance`
* C source: `picoquictest/congestion_test.c:325-336`
* Rust source: `rs/fq/src/tests/congestion.rs:805-810`

### C test body
```c
{
    uint64_t max_completion_time = 1050000;
    uint64_t latency = 10000;
    uint64_t jitter = 3000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 100;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Rust test body
```rust
fn bbr_performance() {
    let latency = 10_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(1_050_000, 100, latency, jitter, buffer);
}
```

## `picoquictest/ech_test.c:ech_e2e_test`
* C test-table name: `ech_e2e`
* C entry function: `ech_e2e_test`
* Rust test: `ech_e2e`
* C source: `picoquictest/ech_test.c:442-447`
* Rust source: `rs/fq/src/tests/ech.rs:262-268`

### C test body
```c
{
    ech_e2e_spec_t spec = { 0 };
    spec.expect_success = 1;
    return ech_e2e_test_one(&spec);
}
```

### Rust test body
```rust
fn ech_e2e() {
    let spec = EchE2eSpec {
        expect_success: true,
        ..Default::default()
    };
    ech_e2e_test_one(&spec);
}
```

## `picoquictest/edge_cases.c:ecf1_final_loss_test`
* C test-table name: `ecf1_final_loss`
* C entry function: `ecf1_final_loss_test`
* Rust test: `ecf1_final_loss`
* C source: `picoquictest/edge_cases.c:445-489`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1155-1172`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t final_losses = 0xb10;
    uint8_t test_case_id = 0xf1;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 0, &simulated_time, 0, 20);
    uint64_t zero_loss_mask = 0;

    /* Finish the connection */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &zero_loss_mask, 0, &simulated_time);
        if (ret != 0)
        {
            DBG_PRINTF("Connect loop returns %d\n", ret);
        }
    }
    /* Finish sending data */
    test_ctx->immediate_exit = 1;

    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &zero_loss_mask, &simulated_time, 0);

        if (ret != 0)
        {
            DBG_PRINTF("Data sending loop returns %d\n", ret);
        }
    }
    /* Simulate losses during closing */
    if (ret == 0) {
        ret = tls_api_close_with_losses(test_ctx, &simulated_time, final_losses);
    }

    if (ret == 0 && simulated_time > 10000000) {
        DBG_PRINTF("Took %" PRIu64 "us to complete, too long", simulated_time);
        ret = -1;
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
fn ecf1_final_loss() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0xf1, false, &mut simulated_time, 0, 20).expect("edge_case_prepare");
    let mut zero_loss = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut zero_loss, 0, &mut simulated_time)
        .expect("connection loop");
    test_ctx.immediate_exit = true;
    tls_api_data_sending_loop(&mut test_ctx, &mut zero_loss, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0xb10)
        .expect("close with losses");
    assert!(
        simulated_time.ticks() <= 10_000_000,
        "connection close took too long: {} µs",
        simulated_time.ticks()
    );
}
```

## `picoquictest/edge_cases.c:reset_extra_max_test`
* C test-table name: `reset_extra_max`
* C entry function: `reset_extra_max_test`
* Rust test: `reset_extra_max`
* C source: `picoquictest/edge_cases.c:1203-1206`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1349-1351`

### C test body
```c
{
    return reset_repeat_test_one(reset_extra_max_stream);
}
```

### Rust test body
```rust
fn reset_extra_max() {
    reset_repeat_test_one(ResetTestKind::ExtraMaxStream).expect("reset_extra_max");
}
```

## `picoquictest/edge_cases.c:reset_stream_at_limit_test`
* C test-table name: `reset_stream_at_limit_test`
* C entry function: `reset_stream_at_limit_test`
* Rust test: `reset_stream_at_limit_test`
* C source: `picoquictest/edge_cases.c:2018-2021`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1516-1518`

### C test body
```c
{
    return reset_stream_at_test_one(rsat_limit);
}
```

### Rust test body
```rust
fn reset_stream_at_limit_test() {
    reset_stream_at_test_one(ResetStreamAtSpec::Limit).expect("reset_stream_at_limit");
}
```
