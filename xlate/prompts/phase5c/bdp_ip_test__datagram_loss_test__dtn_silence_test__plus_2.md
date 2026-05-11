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

## `picoquictest/congestion_test.c:bdp_ip_test`
* C test-table name: `bdp_ip`
* C entry function: `bdp_ip_test`
* Rust test: `bdp_ip`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:906-908`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The IP-specific BDP setup and BDP assertions mostly match, including changed client address and cwin seed rejection, but the shared Rust scenario verifier drops the C max-completion and full scenario-completion checks used by this case.
* Phase 5A fix note: Fix the shared Rust scenario verification helper to enforce the C scenario checks and the 9000000 microsecond bound for the IP second pass; keep the existing BDP-specific assertions.
* Phase 5B analysis: Current Rust already calls BdpTestOption::Ip, applies the IP change and 9000000us second-pass bound, and the shared verifier checks full scenario completion plus max completion time like C. No test repair needed.
* Phase 5B fix note: 

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_ip);
}
```

### Current Rust test body
```rust
fn bdp_ip() {
    bdp_option_test_one(BdpTestOption::Ip);
}
```

## `picoquictest/datagram_tests.c:datagram_loss_test`
* C test-table name: `datagram_loss`
* C entry function: `datagram_loss_test`
* Rust test: `datagram_loss`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Current Rust span: `rs/fq/src/tests/datagram.rs:698-707`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper uses the same parameters, but the Rust helper manually simulates datagram sends, losses, receives, and ACK/loss callbacks instead of driving QUIC datagram frames through the simulator as the C helper does.
* Phase 5A fix note: Phase 5B should make datagram_test_one exercise the Rust QUIC datagram transport/callback path under the supplied loss mask, then assert the same ACK/NACK/spurious and delivery accounting as C.
* Phase 5B analysis: Rust datagram_loss is present as a #[test], compiles under the Rust test harness, and matches the C entry's API-level setup and datagram_test_one call. Callback/transport behavior gaps are Phase 5C runtime implementation issues, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 20000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;

    return datagram_test_one(4, &dg_ctx, 0x040080100200400ull);
}
```

### Current Rust test body
```rust
fn datagram_loss() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        ..Default::default()
    };
    datagram_test_one(4, &mut dg_ctx, 0x040080100200400);
}
```

## `picoquictest/delay_tolerant_test.c:dtn_silence_test`
* C test-table name: `dtn_silence`
* C entry function: `dtn_silence_test`
* Rust test: `dtn_silence`
* Expected Rust file: `rs/fq/src/tests/delay_tolerant.rs`
* Current Rust span: `rs/fq/src/tests/delay_tolerant.rs:203-209`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust DTN silence scenario, basic spec overrides, packet cap, test id, and transport setup match C, but the Rust scenario verifier ignores the C test's 481000000us completion-time limit.
* Phase 5A fix note: In Phase 5B, enforce max_completion_microsec in the shared Rust scenario verifier so dtn_silence checks the 481000000us deadline while keeping the packet-count assertion.
* Phase 5B analysis: Current Rust #[test] faithfully mirrors the C API-level contract: silence scenario, packet cap 120, 481000000us deadline, and dtn_test_one(0x51). Any early tls_api_one_scenario_body Error::Generic/runtime behavior is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Simple test. */
    dtn_test_spec_t spec;
    dtn_set_basic_test_spec(&spec);
    spec.scenario = dtn_scenario_silence;
    spec.sizeof_scenario = sizeof(dtn_scenario_silence);
    spec.max_number_of_packets = 120; /* Check that the number of packets does not increase wildly */
    spec.max_completion_time = 481000000; /* 8 minutes: 2 for handshake, plus 2 per transaction */
    return dtn_test_one(0x51, &spec);
}
```

### Current Rust test body
```rust
fn dtn_silence() {
    let mut spec = dtn_basic_spec();
    spec.scenario = DTN_SCENARIO_SILENCE;
    spec.max_number_of_packets = 120;
    spec.max_completion_time = 481_000_000; // 8 min: 2 handshake + 2 per tx
    dtn_test_one(0x51, &spec);
}
```

## `picoquictest/edge_cases.c:reset_need_max_test`
* C test-table name: `reset_need_max`
* C entry function: `reset_need_max_test`
* Rust test: `reset_need_max`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1887-1889`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper selects the matching NeedMaxStream case, but Rust returns success when the stream is absent, while the C helper still calls check_frame_needs_repeat and asserts no_need_to_repeat for that frame.
* Phase 5A fix note: Remove the early success path in reset_repeat_test_need_repeat; always call check_frame_needs_repeat and require success, state <= Ready, and no_need_to_repeat, matching C.
* Phase 5B analysis: Rust #[test] is present, compiles, and calls reset_repeat_test_one(NeedMaxStream); the helper now calls check_frame_needs_repeat and checks ret/state/no_need_to_repeat like C. Any early runtime failure is Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return reset_repeat_test_one(reset_need_max_stream);
}
```

### Current Rust test body
```rust
fn reset_need_max() {
    reset_repeat_test_one(ResetTestKind::NeedMaxStream).expect("reset_need_max");
}
```

## `picoquictest/l4s_test.c:l4s_c4_test`
* C test-table name: `l4s_c4`
* C entry function: `l4s_c4_test`
* Rust test: `l4s_c4`
* Expected Rust file: `rs/fq/src/tests/l4s.rs`
* Current Rust span: `rs/fq/src/tests/l4s.rs:184-188`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses the same C4/L4S inputs and keeps the loss and RTT-variance assertions, but the shared Rust verifier ignores the max completion time, so the C 3,600,000 us deadline is not checked.
* Phase 5A fix note: Make the Rust scenario verifier or L4S helper enforce max_completion_time before accepting the test.
* Phase 5B analysis: Current Rust test matches the C API-level contract: it resolves C4, enables L4S, passes the 3,600,000 us deadline, loss cap 30, RTT-variance cap 3000, and no link schedule through the shared L4S helper. Prior early runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgo = c4_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 3600000, 30, 3000, 0, NULL);

    return ret;
}
```

### Current Rust test body
```rust
fn l4s_c4() {
    crate::register_all_congestion_control_algorithms();
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    l4s_congestion_test(ccalgo, true, 3_600_000, 30, 3_000, &[]);
}
```
