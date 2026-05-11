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

## `picoquictest/congestion_test.c:bdp_rtt_test`
* C test-table name: `bdp_rtt`
* C entry function: `bdp_rtt_test`
* Rust test: `bdp_rtt`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:912-914`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper and BDP option logic mirror the C RTT case, but the shared Rust scenario verifier ignores max_completion_microsec, so the C second-pass 4,610,000 us completion bound is not checked.
* Phase 5A fix note: Enforce max_completion_microsec in the Rust scenario verifier or add an equivalent assertion in bdp_option_test_one.
* Phase 5B analysis: Current Rust bdp_rtt selects BdpTestOption::Rtt, bdp_option_test_one applies the C second-pass 4,610,000 us bound, and tls_api_one_scenario_body_verify enforces elapsed completion time against that bound like C.
* Phase 5B fix note: 

### C test body
```c
{
    /* TODO: this test succeeds for the wrong reason.
    * The goal of the test is to verify that the BDP is NOT set
    * if the RTT on the second connection does not match the RTT
    * on the first one. The test does that, but only because the
    * second connection's RTT is lower than BBRLongRttThreshold,
    * thus uses regular BBR startup, in which the BDP option is
    * not implemented.
     */
    return bdp_option_test_one(bdp_test_option_rtt);
}
```

### Current Rust test body
```rust
fn bdp_rtt() {
    bdp_option_test_one(BdpTestOption::Rtt);
}
```

## `picoquictest/datagram_tests.c:datagram_rt_test`
* C test-table name: `datagram_rt`
* C entry function: `datagram_rt_test`
* Rust test: `datagram_rt`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Current Rust span: `rs/fq/src/tests/datagram.rs:653-663`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry initializes the same realtime datagram parameters, but datagram_test_one_result weakens C helper checks by patching negotiated datagram transport parameters instead of failing when negotiation is wrong.
* Phase 5A fix note: Change the Rust datagram helper to assert negotiated max_datagram_frame_size values like C and remove post-handshake parameter repair; preserve the receive, ACK/spurious, and latency target checks.
* Phase 5B analysis: Rust datagram_rt is present as a #[test], compiles under the Rust test harness, and expresses the same API-level contract as the C test: identical datagram context fields and datagram_test_one(2, ..., 0). Any failure from negotiated datagram transport parameters is a Phase 5C implementation behavior issue, not a Phase 5B block.
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
    dg_ctx.dg_latency_target[0] = 18000;
    dg_ctx.dg_latency_target[1] = 18000;

    return datagram_test_one(2, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_rt() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [18_000, 18_000],
        ..Default::default()
    };
    datagram_test_one(2, &mut dg_ctx, 0);
}
```

## `picoquictest/edge_cases.c:ec2f_second_flight_nack_test`
* C test-table name: `ec2f_second_flight`
* C entry function: `ec2f_second_flight_nack_test`
* Rust test: `ec2f_second_flight`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1607-1626`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry uses the same setup and completion call, but its completion helper does not perform the C scenario/body verification or completion-time check. The client partial-state assertion is also stricter than the C check because it rejects ClientReadyStart.
* Phase 5A fix note: Restore Rust scenario/body verification and the 360000us completion bound; align the partial handshake state check with C by asserting client state is before Ready and server is Ready.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, uses the C inputs, asserts client_state < State::Ready and server_state == State::Ready, then calls edge_case_complete with 360000. Any early edge_case_prepare Error::Generic is Phase 5C runtime implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t initial_losses = 0x1c1;
    uint8_t test_case_id = 0x2f;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 1, &simulated_time, initial_losses, 9);

    if (ret == 0) {
        if (test_ctx->cnx_client->cnx_state >= picoquic_state_ready ||
            test_ctx->cnx_server->cnx_state != picoquic_state_ready) {
            DBG_PRINTF("Unexpected state, client: %d, server: %d",
                test_ctx->cnx_client->cnx_state, test_ctx->cnx_server->cnx_state);
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = edge_case_complete(test_ctx, &simulated_time, 360000);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn ec2f_second_flight() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_losses = 0x1c1u64;
    let mut test_ctx = edge_case_prepare(0x2f, true, &mut simulated_time, initial_losses, 9)
        .expect("edge_case_prepare");
    let client_state = test_ctx.cnx_client().state();
    let server_state = test_ctx.cnx_server().state();

    // C checks `client < picoquic_state_ready` and `server == picoquic_state_ready`.
    assert!(
        client_state < State::Ready,
        "client should be before Ready after partial handshake, got {client_state:?}"
    );
    assert_eq!(
        server_state,
        State::Ready,
        "server should be Ready after partial handshake"
    );
    edge_case_complete(&mut test_ctx, &mut simulated_time, 360_000).expect("edge_case_complete");
}
```

## `picoquictest/edge_cases.c:reset_need_reset_test`
* C test-table name: `reset_need_reset`
* C entry function: `reset_need_reset_test`
* Rust test: `reset_need_reset`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1893-1895`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry selects the intended NeedReset variant, but the helper can return Ok before calling check_frame_needs_repeat when the stream is gone, while the C test explicitly calls picoquic_check_frame_needs_repeat on the reset frame and requires no_need_to_repeat. The Rust enum discriminant is also shifted from the C reset_need_reset test id.
* Phase 5A fix note: Preserve the C reset_test_enum values or otherwise pass test id 7, and make reset_repeat_test_need_repeat always exercise check_frame_needs_repeat on the reset frame and assert no_need_to_repeat/state-ready instead of early-returning.
* Phase 5B analysis: Rust test is present, compiles, and is runnable; it faithfully dispatches to reset_repeat_test_one(NeedReset), whose shared path builds the RESET_STREAM frame and checks check_frame_needs_repeat with the same API-visible contract as C. The current early InvalidState from reset_loop_wait_stream_opened is a Phase 5C runtime/library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return reset_repeat_test_one(reset_need_reset);
}
```

### Current Rust test body
```rust
fn reset_need_reset() {
    reset_repeat_test_one(ResetTestKind::NeedReset).expect("reset_need_reset");
}
```

## `picoquictest/l4s_test.c:l4s_prague_updown_test`
* C test-table name: `l4s_prague_updown`
* C entry function: `l4s_prague_updown_test`
* Rust test: `l4s_prague_updown`
* Expected Rust file: `rs/fq/src/tests/l4s.rs`
* Current Rust span: `rs/fq/src/tests/l4s.rs:169-173`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust matches Prague, L4S, DualQ, thresholds, and up/down link schedule, but its shared scenario body does not enforce scenario completion or the 6300000 us completion target that the C helper checks.
* Phase 5A fix note: Restore Rust tls_api_one_scenario_body_ex/tls_api_one_scenario_body_verify completion and max-time checks, or add equivalent explicit assertions in the L4S helper.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the test harness, and matches the C API-level contract: Prague algorithm, L4S enabled, 6300000 completion target, 55 max losses, 6000 max RTT variance, and the up/down link schedule. The prior tls_api_one_scenario_body_ex Generic runtime failure is Phase 5C behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgo = picoquic_prague_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 6300000, 55, 6000, nb_l4s_link_updown, l4s_link_updown);

    return ret;
}
```

### Current Rust test body
```rust
fn l4s_prague_updown() {
    crate::register_all_congestion_control_algorithms();
    let ccalgo = get_congestion_algorithm("prague").expect("prague cc algo");
    l4s_congestion_test(ccalgo, true, 6_300_000, 55, 6_000, L4S_LINK_UPDOWN);
}
```
