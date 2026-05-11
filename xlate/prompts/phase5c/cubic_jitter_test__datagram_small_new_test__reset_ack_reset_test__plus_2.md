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

## `picoquictest/congestion_test.c:cubic_jitter_test`
* C test-table name: `cubic_jitter`
* C entry function: `cubic_jitter_test`
* Rust test: `cubic_jitter`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:746-749`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test uses the same cubic algorithm, completion target, jitter, and jitter id as C, but the shared Rust scenario verifier drops the 3,550,000 us completion assertion.
* Phase 5A fix note: Enforce max_completion_microsec in tls_api_one_scenario_body_verify or assert the completion time in congestion_control_test.
* Phase 5B analysis: Rust #[test] is present and compile-checks; it selects cubic and calls congestion_control_test(ccalgo, 3_550_000, 5_000, 5), matching the C API-level contract. Any Cubic runtime failure is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    return congestion_control_test(picoquic_cubic_algorithm, 3550000, 5000, 5);
}
```

### Current Rust test body
```rust
fn cubic_jitter() {
    let ccalgo = cc_algo("cubic");
    congestion_control_test(ccalgo, 3_550_000, 5_000, 5);
}
```

## `picoquictest/datagram_tests.c:datagram_small_new_test`
* C test-table name: `datagram_small_new`
* C entry function: `datagram_small_new_test`
* Rust test: `datagram_small_new`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Current Rust span: `rs/fq/src/tests/datagram.rs:739-752`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The entry field values match, but the Rust shared datagram helper weakens the C test by normalizing negotiated datagram parameters and synthesizing datagram delivery/packet counts instead of verifying the real negotiated datagram path and connection packet count as the C helper does.
* Phase 5A fix note: Make the Rust datagram helper fail on bad max_datagram_frame_size negotiation and exercise the real datagram send/receive/ack/provider path, including the client packet-count assertion for max_packets_received=55, rather than patching parameters or using synthetic delivery counters.
* Phase 5B analysis: Rust test is present, compiles under the Rust test harness, and expresses the same API-level contract as C: identical context fields, extended provider flag, test id 7, and zero loss mask. Any remaining provider callback, negotiation, ack, or runtime behavior failures are Phase 5C implementation issues.
* Phase 5B fix note: 

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_small_size = 64;
    dg_ctx.batch_size[0] = 4;
    dg_ctx.batch_size[1] = 4;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 5000;
    dg_ctx.next_gen_time[0] = 50000;
    dg_ctx.next_gen_time[1] = 50000;
    dg_ctx.max_packets_received = 55;
    dg_ctx.use_extended_provider_api = 1;

    return datagram_test_one(7, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_small_new() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        use_extended_provider_api: true,
        ..Default::default()
    };
    datagram_test_one(7, &mut dg_ctx, 0);
}
```

## `picoquictest/edge_cases.c:reset_ack_reset_test`
* C test-table name: `reset_ack_reset`
* C entry function: `reset_ack_reset_test`
* Rust test: `reset_ack_reset`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1863-1865`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The entry maps to the right reset variant, but the Rust AckReset branch only calls generic skip_frame on the reset frame; the C test calls picoquic_process_ack_of_reset_stream_frame and checks the post-ACK state.
* Phase 5A fix note: Implement or call the Rust equivalent of picoquic_process_ack_of_reset_stream_frame for ResetTestKind::AckReset, then assert success and client state remains ready.
* Phase 5B analysis: Rust #[test] is present, compiles, and calls reset_repeat_test_one(ResetTestKind::AckReset), matching the C wrapper and RESET_STREAM ACK API-level contract. Any early failure while waiting for stream deletion is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return reset_repeat_test_one(reset_ack_reset);
}
```

### Current Rust test body
```rust
fn reset_ack_reset() {
    reset_repeat_test_one(ResetTestKind::AckReset).expect("reset_ack_reset");
}
```

## `picoquictest/high_latency_test.c:high_latency_basic_test`
* C test-table name: `high_latency_basic`
* C entry function: `high_latency_basic_test`
* Rust test: `high_latency_basic`
* Expected Rust file: `rs/fq/src/tests/high_latency.rs`
* Current Rust span: `rs/fq/src/tests/high_latency.rs:268-289`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Inputs match, but Rust relies on a placeholder-like scenario verifier that closes only and ignores C's stream-completion and max-completion checks.
* Phase 5A fix note: Implement Rust tls_api_one_scenario_body_verify/test_api scenario verification and completion-time bound; also mirror setting the client congestion algorithm in high_latency_one.
* Phase 5B analysis: Current Rust test is present as #[test], compiles under the Rust test harness, and matches the C API-level contract: NewReno, scenario {4,0,257,2000}, latency*7 completion budget, 5s latency, 10/10 Mbps, no jitter/loss/preemptive/seed. Any incomplete stream callback delivery is a Phase 5C runtime behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Simple test. */
    uint64_t latency = 5000000;
    uint64_t expected_completion = latency*7;

    return high_latency_one(0xba, picoquic_newreno_algorithm, 
        hilat_scenario_basic, sizeof(hilat_scenario_basic),
        expected_completion, latency, 10, 10, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn high_latency_basic() {
    let latency = 5_000_000u64;
    let newreno = high_latency_ccalgo("newreno");
    high_latency_one(
        0xba,
        newreno,
        &[TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 2_000,
        }],
        latency * 7,
        latency,
        10,
        10,
        0,
        false,
        false,
        false,
    );
}
```

## `picoquictest/mediatest.c:mediatest_suspension_test`
* C test-table name: `mediatest_suspension`
* C entry function: `mediatest_suspension_test`
* Rust test: `mediatest_suspension`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1560-1577`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust spec fields match the C entry, but the Rust mediatest driver is a generic stream-transfer approximation and does not implement the C media frame generation, suspension loop timing, finished check, or latency/stat verification.
* Phase 5A fix note: Repair Rust mediatest_one to mirror the C media simulation, including per-suspension timing, media callbacks/statistics, finished-state validation, and audio/video latency checks while honoring do_not_check_video2.
* Phase 5B analysis: Rust #[test] is present and matches the C API-level contract: same BBR selection, media flags, latency bounds, suspension parameters, and mediatest_one Suspension dispatch. Any early handshake/readiness failure is Phase 5C runtime behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.1;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.latency_average = 50000;
    spec.latency_max = 300000;
    spec.do_not_check_video2 = 1;
    spec.nb_suspensions = 1;
    spec.suspension_start_time = 4000000;
    spec.suspension_down_time = 150000;
    spec.suspension_up_time = 50000;
    ret = mediatest_one(mediatest_suspension, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_suspension() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 50_000,
        latency_max: 300_000,
        do_not_check_video2: true,
        nb_suspensions: 1,
        suspension_start_time: 4_000_000,
        suspension_down_time: 150_000,
        suspension_up_time: 50_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::Suspension, &spec).expect("mediatest_suspension");
}
```
