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

## `picoquictest/tls_api_test.c:mtu_drop_fast_test`
* C test-table name: `mtu_drop_fast`
* C entry function: `mtu_drop_fast_test`
* Rust test: `mtu_drop_fast`
* C source: `picoquictest/tls_api_test.c:5111-5115`
* Rust source: `rs/fq/src/tests/tls_api.rs:674-676`

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_fastcc_algorithm, 11500000);
    return ret;
}
```

### Rust test body
```rust
fn mtu_drop_fast() {
    mtu_drop_cc_algotest("fast", 11_500_000).expect("mtu_drop_fast");
}
```

## `picoquictest/tls_api_test.c:optimistic_hole_test`
* C test-table name: `optimistic_hole`
* C entry function: `optimistic_hole_test`
* Rust test: `optimistic_hole`
* C source: `picoquictest/tls_api_test.c:9769-9774`
* Rust source: `rs/fq/src/tests/tls_api.rs:834-836`

### C test body
```c
{
    int ret = optimistic_ack_test_one(0);

    return ret;
}
```

### Rust test body
```rust
fn optimistic_hole() {
    optimistic_ack_test_one(false).expect("optimistic_hole");
}
```

## `picoquictest/tls_api_test.c:preferred_address_test`
* C test-table name: `preferred_address`
* C entry function: `preferred_address_test`
* Rust test: `preferred_address`
* C source: `picoquictest/tls_api_test.c:10072-10075`
* Rust source: `rs/fq/src/tests/tls_api.rs:921-923`

### C test body
```c
{
    return preferred_address_test_one(0, 0);
}
```

### Rust test body
```rust
fn preferred_address() {
    preferred_address_test_one(false, false).expect("preferred_address");
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
