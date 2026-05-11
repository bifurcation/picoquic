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

## `picoquictest/wifitest.c:wifi_cubic_hard_test`
* C test-table name: `wifi_cubic_hard`
* C entry function: `wifi_cubic_hard_test`
* Rust test: `wifi_cubic_hard`
* C source: `picoquictest/wifitest.c:297-311`
* Rust source: `rs/fq/src/tests/wifitest.rs:218-229`

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_hard,
        3000,
        suspension_hard,
        picoquic_cubic_algorithm,
        NULL,
        4700000,
        0,
        0 };
    int ret = wifi_test_one(wifi_test_cubic_hard, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_cubic_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "cubic",
        cc_algo_option: None,
        target_time: 4_700_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_CUBIC_HARD, &spec).expect("wifi_cubic_hard");
}
```
