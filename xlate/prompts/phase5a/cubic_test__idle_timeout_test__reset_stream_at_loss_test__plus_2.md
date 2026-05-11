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

## `picoquictest/edge_cases.c:idle_timeout_test`
* C test-table name: `idle_timeout`
* C entry function: `idle_timeout_test`
* Rust test: `idle_timeout`
* C source: `picoquictest/edge_cases.c:723-740`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1211-1222`

### C test body
```c
{
    int ret = 0;

    if ((ret = idle_timeout_test_one(1, 30000, 30000, 30000000)) == 0 &&
        (ret = idle_timeout_test_one(2, 60000, 20000, 20000000)) == 0 &&
        (ret = idle_timeout_test_one(3, 20000, 60000, 20000000)) == 0 &&
        (ret = idle_timeout_test_one(4, 5000, 300000, 5000000)) == 0 &&
        (ret = idle_timeout_test_one(5, 300000, 5000, 5000000)) == 0 &&
        (ret = idle_timeout_test_one(6, 0, 5000, 5000000)) == 0 &&
        (ret = idle_timeout_test_one(7, 0, 60000, 60000000)) == 0 &&
        (ret = idle_timeout_test_one(8, 5000, 0, 5000000)) == 0 &&
        (ret = idle_timeout_test_one(9, 60000, 0, 60000000)) == 0 &&
        (ret = idle_timeout_test_one(10, 0, 0, UINT64_MAX)) == 0) {
        DBG_PRINTF("%s", "All idle timeout tests pass.\n");
    }
    return ret;
}
```

### Rust test body
```rust
fn idle_timeout() {
    idle_timeout_test_one(1, 30_000, 30_000, 30_000_000).expect("case 1");
    idle_timeout_test_one(2, 60_000, 20_000, 20_000_000).expect("case 2");
    idle_timeout_test_one(3, 20_000, 60_000, 20_000_000).expect("case 3");
    idle_timeout_test_one(4, 5_000, 300_000, 5_000_000).expect("case 4");
    idle_timeout_test_one(5, 300_000, 5_000, 5_000_000).expect("case 5");
    idle_timeout_test_one(6, 0, 5_000, 5_000_000).expect("case 6");
    idle_timeout_test_one(7, 0, 60_000, 60_000_000).expect("case 7");
    idle_timeout_test_one(8, 5_000, 0, 5_000_000).expect("case 8");
    idle_timeout_test_one(9, 60_000, 0, 60_000_000).expect("case 9");
    idle_timeout_test_one(10, 0, 0, u64::MAX).expect("case 10");
}
```

## `picoquictest/edge_cases.c:reset_stream_at_loss_test`
* C test-table name: `reset_stream_at_loss`
* C entry function: `reset_stream_at_loss_test`
* Rust test: `reset_stream_at_loss`
* C source: `picoquictest/edge_cases.c:2023-2026`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1522-1524`

### C test body
```c
{
    return reset_stream_at_test_one(rsat_loss);
}
```

### Rust test body
```rust
fn reset_stream_at_loss() {
    reset_stream_at_test_one(ResetStreamAtSpec::Loss).expect("reset_stream_at_loss");
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
