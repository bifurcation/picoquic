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

## `picoquictest/edge_cases.c:ec00_zero_test`
* C test-table name: `ec00_zero`
* C entry function: `ec00_zero_test`
* Rust test: `ec00_zero`
* C source: `picoquictest/edge_cases.c:295-321`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1083-1092`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = edge_case_prepare(&test_ctx, 0, 1, &simulated_time, 0, 4);

    if (ret == 0) {
        ret = edge_case_complete(test_ctx, &simulated_time, 100000);
    }

    if (ret == 0) {
        if (test_ctx->cnx_client->nb_zero_rtt_acked == 0) {
            DBG_PRINTF("Nb 0RTT acked = %d", test_ctx->cnx_client->nb_zero_rtt_acked);
            ret = -1;
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn ec00_zero() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0x00, true, &mut simulated_time, 0, 4).expect("edge_case_prepare");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 100_000).expect("edge_case_complete");
    assert!(
        test_ctx.cnx_client().nb_zero_rtt_acked > 0,
        "expected at least one 0-RTT packet acked"
    );
}
```

## `picoquictest/edge_cases.c:reset_extra_max_test`
* C test-table name: `reset_extra_max`
* C entry function: `reset_extra_max_test`
* Rust test: `reset_extra_max`
* C source: `picoquictest/edge_cases.c:1203-1206`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1349-1351`

### C test body
```c
{
    return reset_repeat_test_one(reset_extra_max_stream);
}
```

### Rust test body
```rust
fn reset_extra_max() {
    reset_repeat_test_one(ResetTestKind::ExtraMaxStream).expect("reset_extra_max");
}
```

## `picoquictest/flow_control_test.c:flow_control_test`
* C test-table name: `flow_control`
* C entry function: `flow_control_test`
* Rust test: `flow_control`
* C source: `picoquictest/flow_control_test.c:361-374`
* Rust source: `rs/fq/src/tests/flow_control.rs:368-373`

### C test body
```c
{
	fctest_spec_t spec = { 0 };
	spec.test_id = 1;
	spec.transfer_size = 1000000;
	spec.microsecs_per_byte = 10;
	spec.credit_quantum = 0x4000;
	spec.initial_credit = 0x10000;
	spec.bytes_buffered_max = 0x4000;
	spec.completion_target = 11000000;
	spec.ccalgo = picoquic_bbr_algorithm;

	return fctest_one(&spec);
}
```

### Rust test body
```rust
fn flow_control() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr algorithm");
    fctest_one(
        1, bbr, 0, 1_000_000, 10, 0x4000, 0x1_0000, 0x4000, 11_000_000,
    );
}
```

## `picoquictest/high_latency_test.c:high_latency_probeRTT_test`
* C test-table name: `high_latency_probeRTT`
* C entry function: `high_latency_probeRTT_test`
* Rust test: `high_latency_probertt`
* C source: `picoquictest/high_latency_test.c:330-339`
* Rust source: `rs/fq/src/tests/high_latency.rs:318-334`

### C test body
```c
{
    /* Simple test. */
    uint64_t latency = 5000000;
    uint64_t expected_completion = 839000000;

    return high_latency_one(0xf1, picoquic_bbr_algorithm,
        hilat_scenario_100mb, sizeof(hilat_scenario_100mb),
        expected_completion, latency, 1, 1, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn high_latency_probertt() {
    let latency = 5_000_000u64;
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    high_latency_one(
        0xf1,
        bbr,
        HILAT_SCENARIO_100MB,
        839_000_000,
        latency,
        1,
        1,
        0,
        false,
        false,
        false,
    );
}
```

## `picoquictest/l4s_test.c:l4s_prague_updown_test`
* C test-table name: `l4s_prague_updown`
* C entry function: `l4s_prague_updown_test`
* Rust test: `l4s_prague_updown`
* C source: `picoquictest/l4s_test.c:179-186`
* Rust source: `rs/fq/src/tests/l4s.rs:169-172`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgo = picoquic_prague_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 6300000, 55, 6000, nb_l4s_link_updown, l4s_link_updown);

    return ret;
}
```

### Rust test body
```rust
fn l4s_prague_updown() {
    let ccalgo = get_congestion_algorithm("prague").expect("prague cc algo");
    l4s_congestion_test(ccalgo, true, 6_300_000, 55, 6_000, L4S_LINK_UPDOWN);
}
```
