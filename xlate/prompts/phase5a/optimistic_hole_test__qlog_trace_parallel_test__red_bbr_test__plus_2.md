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

## `picoquictest/tls_api_test.c:red_bbr_test`
* C test-table name: `red_bbr`
* C entry function: `red_bbr_test`
* Rust test: `red_bbr`
* C source: `picoquictest/tls_api_test.c:11138-11142`
* Rust source: `rs/fq/src/tests/tls_api.rs:1070-1072`

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_bbr_algorithm, 500000, 170);
    return ret;
}
```

### Rust test body
```rust
fn red_bbr() {
    red_cc_algotest("bbr", 500_000, 170).expect("red_bbr");
}
```

## `picoquictest/tls_api_test.c:tls_retry_token_test`
* C test-table name: `retry_token`
* C entry function: `tls_retry_token_test`
* Rust test: `retry_token`
* C source: `picoquictest/tls_api_test.c:3841-3863`
* Rust source: `rs/fq/src/tests/tls_api.rs:1146-1148`

### C test body
```c
{
    int ret = tls_retry_token_test_one(1,0);

    if (ret != 0) {
        DBG_PRINTF("Retry token test returns %d", ret);
    }
    else {
        ret = tls_retry_token_test_one(2,0);

        if (ret != 0) {
            DBG_PRINTF("Provide token test returns %d", ret);
        }
        else {
            ret = tls_retry_token_test_one(1, 1);
            if (ret != 0){
                DBG_PRINTF("Duplicate token test returns %d", ret);
            }
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn retry_token() {
    tls_retry_token_test_one(1, false).expect("retry_token");
}
```

## `picoquictest/tls_api_test.c:short_initial_cid_test`
* C test-table name: `short_initial_cid`
* C entry function: `short_initial_cid_test`
* Rust test: `short_initial_cid`
* C source: `picoquictest/tls_api_test.c:8532-8540`
* Rust source: `rs/fq/src/tests/tls_api.rs:1221-1226`

### C test body
```c
{
    int ret = 0;
    for (uint8_t i = 4; ret == 0 && i < 18; i++) {
        ret = short_initial_cid_test_one(i);
    }

    return ret;
}
```

### Rust test body
```rust
fn short_initial_cid() {
    for len in 4u32..=17 {
        short_initial_cid_test_one(len)
            .unwrap_or_else(|e| panic!("short_initial_cid({len}): {e:?}"));
    }
}
```
