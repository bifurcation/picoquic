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

## `picoquictest/tls_api_test.c:preferred_address_zero_test`
* C test-table name: `preferred_address_zero`
* C entry function: `preferred_address_zero_test`
* Rust test: `preferred_address_zero`
* C source: `picoquictest/tls_api_test.c:10082-10086`
* Rust source: `rs/fq/src/tests/tls_api.rs:938-940`

### C test body
```c
{
    /* test with zero length client cid */
    return preferred_address_test_one(0, 1);
}
```

### Rust test body
```rust
fn preferred_address_zero() {
    preferred_address_test_one(false, true).expect("preferred_address_zero");
}
```

## `picoquictest/tls_api_test.c:random_padding_test`
* C test-table name: `random_padding`
* C entry function: `random_padding_test`
* Rust test: `random_padding`
* C source: `picoquictest/tls_api_test.c:12390-12401`
* Rust source: `rs/fq/src/tests/tls_api.rs:1018-1020`

### C test body
```c
{
    uint64_t random_context = 0x1234567890abcdef;

    int ret = random_padding_test_one(128, &random_context, 0);

    if (ret == 0) {
        ret = random_padding_test_one(16, &random_context, 1);
    }

    return ret;
}
```

### Rust test body
```rust
fn random_padding() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("random_padding");
}
```

## `picoquictest/tls_api_test.c:red_dcubic_test`
* C test-table name: `red_dcubic`
* C entry function: `red_dcubic_test`
* Rust test: `red_dcubic`
* C source: `picoquictest/tls_api_test.c:11126-11130`
* Rust source: `rs/fq/src/tests/tls_api.rs:1086-1088`

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_dcubic_algorithm, 500000, 275);
    return ret;
}
```

### Rust test body
```rust
fn red_dcubic() {
    red_cc_algotest("dcubic", 500_000, 275).expect("red_dcubic");
}
```
