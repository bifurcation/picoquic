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

## `picoquictest/tls_api_test.c:mtu_max_test`
* C test-table name: `mtu_max`
* C entry function: `mtu_max_test`
* Rust test: `mtu_max`
* C source: `picoquictest/tls_api_test.c:5002-5007`
* Rust source: `rs/fq/src/tests/tls_api.rs:690-692`

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_basic, 1420, 1392,
        test_scenario_mtu_discovery, sizeof(test_scenario_mtu_discovery), 1420);
    return ret;
}
```

### Rust test body
```rust
fn mtu_max() {
    mtu_discovery_test_one(0, 1420, 1392, 2_500_000, 1420).expect("mtu_max");
}
```

## `picoquictest/tls_api_test.c:perflog_test`
* C test-table name: `perflog`
* C entry function: `perflog_test`
* Rust test: `perflog`
* C source: `picoquictest/tls_api_test.c:9324-9329`
* Rust source: `rs/fq/src/tests/tls_api.rs:885-887`

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Rust test body
```rust
fn perflog() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("perflog");
}
```

## `picoquictest/tls_api_test.c:qlog_trace_parallel_test`
* C test-table name: `qlog_trace_parallel`
* C entry function: `qlog_trace_parallel_test`
* Rust test: `qlog_trace_parallel`
* C source: `picoquictest/tls_api_test.c:9097-9100`
* Rust source: `rs/fq/src/tests/tls_api.rs:989-991`

### C test body
```c
{
    return qlog_trace_test_one(0, 1);
}
```

### Rust test body
```rust
fn qlog_trace_parallel() {
    qlog_trace_test_one(0, true).expect("qlog_trace_parallel");
}
```

## `picoquictest/tls_api_test.c:ready_to_zfin_test`
* C test-table name: `ready_to_zfin`
* C entry function: `ready_to_zfin_test`
* Rust test: `ready_to_zfin`
* C source: `picoquictest/tls_api_test.c:9466-9470`
* Rust source: `rs/fq/src/tests/tls_api.rs:1062-1064`

### C test body
```c
{
    int ret = ready_to_send_test_one(2);
    return ret;
}
```

### Rust test body
```rust
fn ready_to_zfin() {
    ready_to_send_test_one(2).expect("ready_to_zfin");
}
```

## `picoquictest/tls_api_test.c:tls_api_retry_large_test`
* C test-table name: `retry_large`
* C entry function: `tls_api_retry_large_test`
* Rust test: `retry_large`
* C source: `picoquictest/tls_api_test.c:4058-4061`
* Rust source: `rs/fq/src/tests/tls_api.rs:1137-1139`

### C test body
```c
{
    return tls_api_retry_test_one(1);
}
```

### Rust test body
```rust
fn retry_large() {
    tls_api_retry_test_one(true).expect("retry_large");
}
```
