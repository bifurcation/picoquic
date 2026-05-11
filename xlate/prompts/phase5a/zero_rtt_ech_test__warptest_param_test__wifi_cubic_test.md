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

## `picoquictest/tls_api_test.c:zero_rtt_ech_test`
* C test-table name: `zero_rtt_ech`
* C entry function: `zero_rtt_ech_test`
* Rust test: `zero_rtt_ech`
* C source: `picoquictest/tls_api_test.c:4781-4786`
* Rust source: `rs/fq/src/tests/tls_api.rs:1578-1584`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.propose_ech = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt_ech() {
    zero_rtt_test_one(&ZeroRttTest {
        propose_ech: true,
        ..Default::default()
    })
    .expect("zero_rtt_ech");
}
```

## `picoquictest/warptest.c:warptest_param_test`
* C test-table name: `warptest_param`
* C entry function: `warptest_param_test`
* Rust test: `warptest_param`
* C source: `picoquictest/warptest.c:1552-1566`
* Rust source: `rs/fq/src/tests/warptest.rs:11-22`

### C test body
```c
{
    int ret;
    warptest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    spec.max_streams_client = 4;
    spec.max_streams_server = 4;

    ret = warptest_one(5, &spec);

    return ret;
}
```

### Rust test body
```rust
fn warptest_param() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        max_streams_client: 4,
        max_streams_server: 4,
        ..Default::default()
    };
    warptest_one(5, &spec).expect("warptest_param");
}
```

## `picoquictest/wifitest.c:wifi_cubic_test`
* C test-table name: `wifi_cubic`
* C entry function: `wifi_cubic_test`
* Rust test: `wifi_cubic`
* C source: `picoquictest/wifitest.c:235-243`
* Rust source: `rs/fq/src/tests/wifitest.rs:211-214`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_cubic_algorithm, 2870000);

    int ret = wifi_test_one(wifi_test_cubic, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_cubic() {
    let spec = default_spec("cubic", SUSPENSION_BASIC, 2_870_000);
    wifi_test_one(WIFI_TEST_CUBIC, &spec).expect("wifi_cubic");
}
```
