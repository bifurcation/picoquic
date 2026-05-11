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

## `picoquictest/tls_api_test.c:zero_rtt_loss_test`
* C test-table name: `zero_rtt_loss`
* C entry function: `zero_rtt_loss_test`
* Rust test: `zero_rtt_loss`
* C source: `picoquictest/tls_api_test.c:4630-4645`
* Rust source: `rs/fq/src/tests/tls_api.rs:1603-1611`

### C test body
```c
{
    int ret = 0;

    for (unsigned int i = 1; ret == 0 && i < 16; i++) {
        zero_rtt_test_t zrt = { 0 };
        zrt.early_loss = 1ull << i;

        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Zero RTT test fails when packet #%d is lost.\n", i);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn zero_rtt_loss() {
    for i in 1u32..16 {
        zero_rtt_test_one(&ZeroRttTest {
            early_loss: 1u64 << i,
            ..Default::default()
        })
        .unwrap_or_else(|e| panic!("zero_rtt_loss i={i}: {e:?}"));
    }
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

## `picoquictest/wifitest.c:wifi_bbr1_long_test`
* C test-table name: `wifi_bbr1_long`
* C entry function: `wifi_bbr1_long_test`
* Rust test: `wifi_bbr1_long`
* C source: `picoquictest/wifitest.c:345-359`
* Rust source: `rs/fq/src/tests/wifitest.rs:134-145`

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_basic,
        50000,
        suspension_basic,
        picoquic_bbr1_algorithm,
        NULL,
        3400000,
        1,
        0 };
    int ret = wifi_test_one(wifi_test_bbr1_long, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_bbr1_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr1",
        cc_algo_option: None,
        target_time: 3_400_000,
        simulate_receive_block: true,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR1_LONG, &spec).expect("wifi_bbr1_long");
}
```

## `picoquictest/wifitest.c:wifi_reno_test`
* C test-table name: `wifi_reno`
* C entry function: `wifi_reno_test`
* Rust test: `wifi_reno`
* C source: `picoquictest/wifitest.c:245-252`
* Rust source: `rs/fq/src/tests/wifitest.rs:248-251`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_newreno_algorithm, 2800000);
    int ret = wifi_test_one(wifi_test_reno, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_reno() {
    let spec = default_spec("newreno", SUSPENSION_BASIC, 2_800_000);
    wifi_test_one(WIFI_TEST_RENO, &spec).expect("wifi_reno");
}
```
