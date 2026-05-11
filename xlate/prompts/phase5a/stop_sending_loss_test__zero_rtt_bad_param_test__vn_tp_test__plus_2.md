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

## `picoquictest/tls_api_test.c:stop_sending_loss_test`
* C test-table name: `stop_sending_loss`
* C entry function: `stop_sending_loss_test`
* Rust test: `stop_sending_loss`
* C source: `picoquictest/tls_api_test.c:4907-4911`
* Rust source: `rs/fq/src/tests/tls_api.rs:1302-1304`

### C test body
```c
{
    int ret = stop_sending_test_one(0, 1);
    return ret;
}
```

### Rust test body
```rust
fn stop_sending_loss() {
    stop_sending_test_one(false, true).expect("stop_sending_loss");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_bad_param_test`
* C test-table name: `zero_rtt_bad_param`
* C entry function: `zero_rtt_bad_param_test`
* Rust test: `zero_rtt_bad_param`
* C source: `picoquictest/tls_api_test.c:4669-4674`
* Rust source: `rs/fq/src/tests/tls_api.rs:1552-1558`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.change_params = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt_bad_param() {
    zero_rtt_test_one(&ZeroRttTest {
        change_params: true,
        ..Default::default()
    })
    .expect("zero_rtt_bad_param");
}
```

## `picoquictest/transport_param_test.c:vn_tp_test`
* C test-table name: `vn_tp`
* C entry function: `vn_tp_test`
* Rust test: `vn_tp`
* C source: `picoquictest/transport_param_test.c:1257-1270`
* Rust source: `rs/fq/src/tests/transport_param.rs:104-111`

### C test body
```c
{
    int ret = 0;

    for (size_t i = 0; i < nb_vn_tp_test_case; i++) {
        if (vn_tp_test_one(vn_tp_test_case[i].vn_tp_len, vn_tp_test_case[i].vn_tp, vn_tp_test_case[i].mode,
            vn_tp_test_case[i].vn_envelop, vn_tp_test_case[i].vn_expected, vn_tp_test_case[i].error_expected) != 0) {
            DBG_PRINTF("Vn test case[%zu] fails", i);
            ret = -1;
            break;
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn vn_tp() {
    for test_id in 0..8 {
        vn_tp_test_one(test_id, true, test_id < 4)
            .unwrap_or_else(|e| panic!("client vn_tp[{test_id}]: {e:?}"));
        vn_tp_test_one(test_id, false, test_id < 4)
            .unwrap_or_else(|e| panic!("server vn_tp[{test_id}]: {e:?}"));
    }
}
```

## `picoquictest/warptest.c:warptest_video_test`
* C test-table name: `warptest_video`
* C entry function: `warptest_video_test`
* Rust test: `warptest_video`
* C source: `picoquictest/warptest.c:1498-1509`
* Rust source: `rs/fq/src/tests/warptest.rs:28-36`

### C test body
```c
{
    int ret;
    warptest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    ret = warptest_one(1, &spec);

    return ret;
}
```

### Rust test body
```rust
fn warptest_video() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        ..Default::default()
    };
    warptest_one(1, &spec).expect("warptest_video");
}
```

## `picoquictest/wifitest.c:wifi_bbr_hard_test`
* C test-table name: `wifi_bbr_hard`
* C entry function: `wifi_bbr_hard_test`
* Rust test: `wifi_bbr_hard`
* C source: `picoquictest/wifitest.c:265-279`
* Rust source: `rs/fq/src/tests/wifitest.rs:149-160`

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_hard,
        3000,
        suspension_hard,
        picoquic_bbr_algorithm,
        NULL,
        4060000,
        0,
        0 };
    int ret = wifi_test_one(wifi_test_bbr_hard, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_bbr_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "bbr",
        cc_algo_option: None,
        target_time: 4_060_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR_HARD, &spec).expect("wifi_bbr_hard");
}
```
