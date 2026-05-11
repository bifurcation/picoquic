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

## `picoquictest/tls_api_test.c:zero_rtt_spurious_test`
* C test-table name: `zero_rtt_spurious`
* C entry function: `zero_rtt_spurious_test`
* Rust test: `zero_rtt_spurious`
* C source: `picoquictest/tls_api_test.c:4654-4659`
* Rust source: `rs/fq/src/tests/tls_api.rs:1662-1668`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.use_badcrypt = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt_spurious() {
    zero_rtt_test_one(&ZeroRttTest {
        use_badcrypt: true,
        ..Default::default()
    })
    .expect("zero_rtt_spurious");
}
```

## `picoquictest/wifitest.c:wifi_bbr1_hard_test`
* C test-table name: `wifi_bbr1_hard`
* C entry function: `wifi_bbr1_hard_test`
* Rust test: `wifi_bbr1_hard`
* C source: `picoquictest/wifitest.c:281-295`
* Rust source: `rs/fq/src/tests/wifitest.rs:119-130`

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_hard,
        3000,
        suspension_hard,
        picoquic_bbr1_algorithm,
        NULL,
        4060000,
        0,
        0 };
    int ret = wifi_test_one(wifi_test_bbr1_hard, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_bbr1_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "bbr1",
        cc_algo_option: None,
        target_time: 4_060_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR1_HARD, &spec).expect("wifi_bbr1_hard");
}
```

## `picoquictest/wifitest.c:wifi_reno_long_test`
* C test-table name: `wifi_reno_long`
* C entry function: `wifi_reno_long_test`
* Rust test: `wifi_reno_long`
* C source: `picoquictest/wifitest.c:372-382`
* Rust source: `rs/fq/src/tests/wifitest.rs:270-281`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_newreno_algorithm, 3000000);
    spec.latency = 50000;
    spec.simulate_receive_block = 1;

    int ret = wifi_test_one(wifi_test_reno_long, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_reno_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "newreno",
        cc_algo_option: None,
        target_time: 3_000_000,
        simulate_receive_block: true,
        queue_max_delay: 260_000,
    };
    wifi_test_one(WIFI_TEST_RENO_LONG, &spec).expect("wifi_reno_long");
}
```
