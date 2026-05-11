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
