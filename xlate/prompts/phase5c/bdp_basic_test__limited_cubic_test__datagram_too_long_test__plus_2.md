# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/congestion_test.c:bdp_basic_test`
* C test-table name: `bdp_basic`
* C entry function: `bdp_basic_test`
* Rust test: `bdp_basic`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:894-896`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper and BDP option setup match the C test, but the shared Rust scenario verifier ignores the nonzero max-completion deadline used by the C helper, so the second-pass 5900000us timing assertion is not checked.
* Phase 5A fix note: In Phase 5B, make rs/fq/src/tests/util.rs::tls_api_one_scenario_body_verify enforce max_completion_microsec like the C helper, so bdp_basic retains the 5900000us bound.
* Phase 5B analysis: Current Rust already matches the C timing check: bdp_basic runs bdp_option_test_one(Basic), passes 5900000us on the second pass, and tls_api_one_scenario_body_verify enforces nonzero max_completion_microsec using close_time - client start_time.
* Phase 5B fix note: 

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_basic);
}
```

### Current Rust test body
```rust
fn bdp_basic() {
    bdp_option_test_one(BdpTestOption::Basic);
}
```

## `picoquictest/cpu_limited.c:limited_cubic_test`
* C test-table name: `limited_cubic`
* C entry function: `limited_cubic_test`
* Rust test: `limited_cubic`
* Expected Rust file: `rs/fq/src/tests/cpu_limited.rs`
* Current Rust span: `rs/fq/src/tests/cpu_limited.rs:171-176`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust limited-cubic config, default values, generated scenario, congestion algorithm, and helper setup correspond to C, but the Rust scenario verifier ignores the 4200000us max-completion assertion.
* Phase 5A fix note: In Phase 5B, enforce max_completion_microsec in the shared Rust scenario verifier so limited_cubic checks the 4200000us deadline.
* Phase 5B analysis: Rust #[test] limited_cubic matches the C API-level contract: default config id 2, Cubic congestion algorithm, 4_200_000us completion limit, and shared limited_client_test_one execution. Any scenario-body Generic/runtime failure is Phase 5C behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    limited_test_config_t config;
    limited_config_set_default(&config, 2);
    config.ccalgo = picoquic_cubic_algorithm;
    config.max_completion_time = 4200000;

    return limited_client_test_one(&config);
}
```

### Current Rust test body
```rust
fn limited_cubic() {
    let mut config = limited_config_default(2);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic algo");
    config.max_completion_time = 4_200_000;
    limited_client_test_one(config);
}
```

## `picoquictest/datagram_tests.c:datagram_too_long_test`
* C test-table name: `datagram_too_long_test`
* C entry function: `datagram_too_long_test`
* Rust test: `datagram_too_long_test`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Current Rust span: `rs/fq/src/tests/datagram.rs:777-785`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry uses the same dg_max_size, targets, test_too_long flag, test_id, and loss mask, and the too-long helper covers the same queue-size cases. However the Rust datagram_test_one_result silently overwrites negotiated max_datagram_frame_size values after the connection loop instead of failing when negotiation differs, weakening a C datagram_test_one assertion used by this test.
* Phase 5A fix note: In rs/fq/src/tests/datagram.rs, make datagram_test_one_result assert the client remote max_datagram_frame_size is MAX_PACKET_SIZE and the server remote max_datagram_frame_size is dg_ctx.dg_max_size, matching C, rather than normalizing those fields before continuing.
* Phase 5B analysis: Rust #[test] matches the C entry and compiles/runs under the Rust test harness. The prior max_datagram_frame_size negotiation failure is Phase 5C runtime library behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 5;
    dg_ctx.dg_target[1] = 5;
    dg_ctx.test_too_long = 1;

    return datagram_test_one(10, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_too_long_test() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [5, 5],
        test_too_long: true,
        ..Default::default()
    };
    datagram_test_one(10, &mut dg_ctx, 0);
}
```

## `picoquictest/edge_cases.c:reset_extra_stop_test`
* C test-table name: `reset_extra_stop`
* C entry function: `reset_extra_stop_test`
* Rust test: `reset_extra_stop`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1881-1883`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust takes the same extra STOP_SENDING frame path, but ResetTestKind omits the C enum's reset_ack_stop_sending slot, so ExtraStop uses test id/CID byte 4 instead of C's 5.
* Phase 5A fix note: Preserve the C reset_test_enum discriminants, for example by adding an unused AckStopSending = 2 or explicitly assigning ExtraStop = 5 and the following values.
* Phase 5B analysis: Rust test is present, compiles, and is runnable by the Rust harness. It calls reset_repeat_test_one(ResetTestKind::ExtraStop), and ExtraStop is explicitly discriminant 5, matching C reset_extra_stop_sending. Any InvalidState/reset-repeat runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return reset_repeat_test_one(reset_extra_stop_sending);
}
```

### Current Rust test body
```rust
fn reset_extra_stop() {
    reset_repeat_test_one(ResetTestKind::ExtraStop).expect("reset_extra_stop");
}
```

## `picoquictest/high_latency_test.c:high_latency_probeRTT_test`
* C test-table name: `high_latency_probeRTT`
* C entry function: `high_latency_probeRTT_test`
* Rust test: `high_latency_probertt`
* Expected Rust file: `rs/fq/src/tests/high_latency.rs`
* Current Rust span: `rs/fq/src/tests/high_latency.rs:333-349`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses the same scenario, latency, bandwidths, BBR handle, and target time, but the helper never applies BBR to the existing client connection and shared scenario verification does not enforce stream completion or the 839000000 us limit.
* Phase 5A fix note: Set the client connection congestion algorithm in high_latency_one, and restore scenario completion and max-completion-time verification in tls_api_one_scenario_body/tls_api_one_scenario_body_verify.
* Phase 5B analysis: Rust test is present, compiles under the Rust test harness, and matches the C API-level contract: BBR, 100MB scenario, 839000000 us deadline, 5000000 us latency, 1/1 Mbps, and no jitter/loss/preemptive/seed. Any stream completion/data-delivery runtime failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

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

### Current Rust test body
```rust
fn high_latency_probertt() {
    let latency = 5_000_000u64;
    let bbr = high_latency_ccalgo("bbr");
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
