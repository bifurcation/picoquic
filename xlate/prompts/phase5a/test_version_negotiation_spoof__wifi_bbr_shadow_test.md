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

## `picoquictest/tls_api_test.c:test_version_negotiation_spoof`
* C test-table name: `version_negotiation_spoof`
* C entry function: `test_version_negotiation_spoof`
* Rust test: `version_negotiation_spoof`
* C source: `picoquictest/tls_api_test.c:2879-2897`
* Rust source: `rs/fq/src/tests/tls_api.rs:1516-1519`

### C test body
```c
{
    int ret = 0;

    if (test_version_negotiation_spoof_one(0) == 0) {
        DBG_PRINTF("%s", "VN spoof mode 0 has no effect");
        ret = -1;
    }

    for (int spoof_mode = 1; ret == 0 && spoof_mode < 8; spoof_mode++) {
        ret = test_version_negotiation_spoof_one(spoof_mode);
        if (ret != 0) {
            DBG_PRINTF("VN spoof mode %d caused failure", spoof_mode);
            ret = -1;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn version_negotiation_spoof() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("version_negotiation_spoof");
}
```

## `picoquictest/wifitest.c:wifi_bbr_shadow_test`
* C test-table name: `wifi_bbr_shadow`
* C entry function: `wifi_bbr_shadow_test`
* Rust test: `wifi_bbr_shadow`
* C source: `picoquictest/wifitest.c:384-395`
* Rust source: `rs/fq/src/tests/wifitest.rs:196-207`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_bbr_algorithm, 2750000);
    spec.cc_algo_option = "T250000";
    spec.queue_max_delay = 600000;
    spec.simulate_receive_block = 1;

    int ret = wifi_test_one(wifi_test_bbr_shadow, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_bbr_shadow() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr",
        cc_algo_option: Some("T250000"),
        target_time: 2_750_000,
        simulate_receive_block: true,
        queue_max_delay: 600_000,
    };
    wifi_test_one(WIFI_TEST_BBR_SHADOW, &spec).expect("wifi_bbr_shadow");
}
```
