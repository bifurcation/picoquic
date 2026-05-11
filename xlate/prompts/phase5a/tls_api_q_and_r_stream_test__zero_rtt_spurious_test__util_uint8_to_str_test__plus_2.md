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

## `picoquictest/tls_api_test.c:tls_api_q_and_r_stream_test`
* C test-table name: `tls_api_q_and_r_stream`
* C entry function: `tls_api_q_and_r_stream_test`
* Rust test: `tls_api_q_and_r_stream`
* C source: `picoquictest/tls_api_test.c:3271-3274`
* Rust source: `rs/fq/src/tests/tls_api.rs:1384-1388`

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 0, 75000, NULL, NULL);
}
```

### Rust test body
```rust
fn tls_api_q_and_r_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("q_and_r_stream");
}
```

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

## `picoquictest/util_test.c:util_uint8_to_str_test`
* C test-table name: `util_uint8_to_str`
* C entry function: `util_uint8_to_str_test`
* Rust test: `util_uint8_to_str`
* C source: `picoquictest/util_test.c:133-144`
* Rust source: `rs/fq/src/tests/util_test.rs:65-79`

### C test body
```c
{
    int ret = 0;
    char text[16];

    if (strcmp(picoquic_uint8_to_str(text, 16, util_uint8_to_str_input, sizeof(util_uint8_to_str_input)), util_uint8_to_str_out) != 0 ||
        strcmp(picoquic_uint8_to_str(text, 7, util_uint8_to_str_input, sizeof(util_uint8_to_str_input)), util_uint8_to_str_out7) != 0 ||
        strcmp(picoquic_uint8_to_str(text, 2, util_uint8_to_str_input, sizeof(util_uint8_to_str_input)), util_uint8_to_str_out2) != 0) {
        ret = -1;
    }
    return ret;
}
```

### Rust test body
```rust
fn util_uint8_to_str() {
    let input: &[u8] = b"azAZ09.\xff";

    let mut buf = [0u8; 16];
    let out = crate::utils::uint8_to_str(&mut buf, input);
    assert_eq!(out, b"azAZ09.?", "full-width output");

    let mut buf7 = [0u8; 7];
    let out = crate::utils::uint8_to_str(&mut buf7, input);
    assert_eq!(out, b"azA...", "7-byte output");

    let mut buf2 = [0u8; 2];
    let out = crate::utils::uint8_to_str(&mut buf2, input);
    assert_eq!(out, b".", "2-byte output");
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

## `picoquictest/wifitest.c:wifi_cubic_long_test`
* C test-table name: `wifi_cubic_long`
* C entry function: `wifi_cubic_long_test`
* Rust test: `wifi_cubic_long`
* C source: `picoquictest/wifitest.c:361-370`
* Rust source: `rs/fq/src/tests/wifitest.rs:233-244`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_cubic_algorithm, 3100000);
    spec.latency = 50000;
    spec.simulate_receive_block = 1;
    int ret = wifi_test_one(wifi_test_cubic_long, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_cubic_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "cubic",
        cc_algo_option: None,
        target_time: 3_100_000,
        simulate_receive_block: true,
        queue_max_delay: 260_000,
    };
    wifi_test_one(WIFI_TEST_CUBIC_LONG, &spec).expect("wifi_cubic_long");
}
```
