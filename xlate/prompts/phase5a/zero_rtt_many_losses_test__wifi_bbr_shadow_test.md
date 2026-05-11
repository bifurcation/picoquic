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

## `picoquictest/tls_api_test.c:zero_rtt_many_losses_test`
* C test-table name: `zero_rtt_many_losses`
* C entry function: `zero_rtt_many_losses_test`
* Rust test: `zero_rtt_many_losses`
* C source: `picoquictest/tls_api_test.c:4709-4734`
* Rust source: `rs/fq/src/tests/tls_api.rs:1618-1631`

### C test body
```c
{
    int ret = 0;
    uint64_t random_context = 0x1055ca45c001babaull;

    for (int i = 0; ret == 0 && i < 50; i++)
    {
        uint64_t loss_mask = 0;
        zero_rtt_test_t zrt = { 0 };

        for (int j = 0; j < 64; j++)
        {
            loss_mask <<= 1;

            if (picoquic_test_uniform_random(&random_context, 1000) < 300) {
                loss_mask |= 1;
            }
        }
        zrt.early_loss = loss_mask;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Handshake fails for mask %d, mask = %llx", i, (unsigned long long)loss_mask);
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn zero_rtt_many_losses() {
    let mut seed = 0xdead_beef_cafe_1234u64;
    for _ in 0..50 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mask = seed >> 56;
        zero_rtt_test_one(&ZeroRttTest {
            early_loss: mask,
            ..Default::default()
        })
        .expect("zero_rtt_many_losses");
    }
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
