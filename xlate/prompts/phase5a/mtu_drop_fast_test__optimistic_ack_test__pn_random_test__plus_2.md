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

## `picoquictest/tls_api_test.c:optimistic_ack_test`
* C test-table name: `optimistic_ack`
* C entry function: `optimistic_ack_test`
* Rust test: `optimistic_ack`
* C source: `picoquictest/tls_api_test.c:9762-9767`
* Rust source: `rs/fq/src/tests/tls_api.rs:825-827`

### C test body
```c
{
    int ret = optimistic_ack_test_one(1);

    return ret;
}
```

### Rust test body
```rust
fn optimistic_ack() {
    optimistic_ack_test_one(true).expect("optimistic_ack");
}
```

## `picoquictest/tls_api_test.c:pn_random_test`
* C test-table name: `pn_random`
* C entry function: `pn_random_test`
* Rust test: `pn_random`
* C source: `picoquictest/tls_api_test.c:12119-12134`
* Rust source: `rs/fq/src/tests/tls_api.rs:903-905`

### C test body
```c
{

    int ret = pn_random_test_one(0);

    if (ret != 0) {
        DBG_PRINTF("Randomize initials fails, ret = %d", ret);
    } else{
        ret = pn_random_test_one(1);
        if (ret != 0) {
            DBG_PRINTF("Randomize all fails, ret = %d", ret);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn pn_random() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("pn_random");
}
```

## `picoquictest/tls_api_test.c:qlog_trace_test`
* C test-table name: `qlog_trace`
* C entry function: `qlog_trace_test`
* Rust test: `qlog_trace`
* C source: `picoquictest/tls_api_test.c:9087-9090`
* Rust source: `rs/fq/src/tests/tls_api.rs:973-975`

### C test body
```c
{
    return qlog_trace_test_one(0, 0);
}
```

### Rust test body
```rust
fn qlog_trace() {
    qlog_trace_test_one(0, false).expect("qlog_trace");
}
```

## `picoquictest/tls_api_test.c:ready_to_skip_test`
* C test-table name: `ready_to_skip`
* C entry function: `ready_to_skip_test`
* Rust test: `ready_to_skip`
* C source: `picoquictest/tls_api_test.c:9454-9458`
* Rust source: `rs/fq/src/tests/tls_api.rs:1046-1048`

### C test body
```c
{
    int ret = ready_to_send_test_one(3);
    return ret;
}
```

### Rust test body
```rust
fn ready_to_skip() {
    ready_to_send_test_one(3).expect("ready_to_skip");
}
```
