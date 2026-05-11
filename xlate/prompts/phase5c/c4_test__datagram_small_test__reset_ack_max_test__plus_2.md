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

## `picoquictest/congestion_test.c:c4_test`
* C test-table name: `c4`
* C entry function: `c4_test`
* Rust test: `c4`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:753-756`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Entry parameters and C4 lookup match, but the Rust shared scenario verifier drops the C max-completion assertion, so the test does not enforce the 3600000 us completion bound.
* Phase 5A fix note: Implement the missing Rust scenario verifier checks for stream completion and max_completion_microsec, or assert the bound in congestion_control_test.
* Phase 5B analysis: Rust c4 is a runnable #[test] that selects c4 and calls the shared congestion_control_test with the C parameters: 3600000 us, zero jitter, zero jitter_id. The shared helper carries the C scenario and max_completion_time contract; any Generic/runtime scenario failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return congestion_control_test(c4_algorithm, 3600000, 0, 0);
}
```

### Current Rust test body
```rust
fn c4() {
    let ccalgo = cc_algo("c4");
    congestion_control_test(ccalgo, 3_600_000, 0, 0);
}
```

## `picoquictest/datagram_tests.c:datagram_small_test`
* C test-table name: `datagram_small`
* C entry function: `datagram_small_test`
* Rust test: `datagram_small`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Current Rust span: `rs/fq/src/tests/datagram.rs:723-735`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper copies the C datagram_small field values, but the helper does not check the same behavior: it normalizes negotiated datagram parameters and synthesizes datagram delivery/packet counting instead of exercising the live callback-driven QUIC datagram path and cnx_client->nb_packets_received assertion.
* Phase 5A fix note: Make datagram_test_one assert negotiated max_datagram_frame_size, drive real datagram send/recv/ack callbacks through the simulator, and check the real client packet count against max_packets_received.
* Phase 5B analysis: Rust datagram_small is a #[test], mirrors the C initializer fields and datagram_test_one(6, ..., 0) call, and compiles under the Rust test harness. Live DATAGRAM callback/runtime gaps are Phase 5C notes, not Phase 5B blockers.
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

    return datagram_test_one(6, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_small() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        ..Default::default()
    };
    datagram_test_one(6, &mut dg_ctx, 0);
}
```

## `picoquictest/edge_cases.c:reset_ack_max_test`
* C test-table name: `reset_ack_max`
* C entry function: `reset_ack_max_test`
* Rust test: `reset_ack_max`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1857-1859`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper selects the right reset variant, but the Rust helper handles AckMaxStream by skip_frame on reset_frame. C calls picoquic_process_ack_of_max_stream_data_frame and verifies no error/state regression, so Rust is not exercising the ACK-processing behavior under test.
* Phase 5A fix note: In reset_repeat_test_one, make AckMaxStream call the Rust equivalent of picoquic_process_ack_of_max_stream_data_frame with the C frame bytes and assert ret/state as C does.
* Phase 5B analysis: Rust `reset_ack_max` is present as a harness test and calls `reset_repeat_test_one(ResetTestKind::AckMaxStream)`, matching the C entry's API-level contract. The prior stream-deletion/runtime failure is a Phase 5C implementation-behavior note, not a Phase 5B block.
* Phase 5B fix note: No Rust test changes; appended command log entry.

### C test body
```c
{
    return reset_repeat_test_one(reset_ack_max_stream);
}
```

### Current Rust test body
```rust
fn reset_ack_max() {
    reset_repeat_test_one(ResetTestKind::AckMaxStream).expect("reset_ack_max");
}
```

## `picoquictest/edge_cases.c:reset_stream_at_basic_test`
* C test-table name: `reset_stream_at_basic`
* C entry function: `reset_stream_at_basic_test`
* Rust test: `reset_stream_at_basic`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:2056-2058`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The top-level Basic variant and scenario mostly match, but Rust computes and verifies delivery using internal stream consumed offsets instead of the C fixture's test_stream[0].r_recv_nb receive counter, weakening the test's application-level correspondence.
* Phase 5A fix note: Add or use Rust fixture receive-byte counters equivalent to C test_stream[0].r_recv_nb for the initial reliable_size calculation and final delivery assertion; ensure the scenario callback/data verification mirrors the C helper behavior.
* Phase 5B analysis: Rust test is present, compiles, and is runnable by the Rust harness. It calls the shared reset_stream_at helper with the Basic variant, matching the C entry function. Any failure from reset_stream_at negotiation/state is Phase 5C library behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return reset_stream_at_test_one(rsat_basic);
}
```

### Current Rust test body
```rust
fn reset_stream_at_basic() {
    reset_stream_at_test_one(ResetStreamAtSpec::Basic).expect("reset_stream_at_basic");
}
```

## `picoquictest/mediatest.c:mediatest_no_coal_test`
* C test-table name: `mediatest_no_coal`
* C entry function: `mediatest_no_coal_test`
* Rust test: `mediatest_no_coal`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1544-1555`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry sets the same spec fields, but mediatest_one never uses spec.no_coal and replaces the C media-frame timing/statistics loop with a generic stream scenario, so the no-coalescing media behavior is not checked.
* Phase 5A fix note: Implement/use the media stream configuration that marks audio/video streams not coalesced and verifies the C media frame completion and latency statistics.
* Phase 5B analysis: Rust test is present as a harness-runnable #[test], builds the same BBR/bandwidth/video/audio/data_size/no_coal spec, calls MediatestId::NoCoal, and the media helper applies no_coal to audio/video with completion and stats checks. Any early runtime failure from incomplete QUIC readiness is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    spec.data_size = 10000000;
    spec.no_coal = 1;
    ret = mediatest_one(mediatest_no_coal, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_no_coal() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        no_coal: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::NoCoal, &spec).expect("mediatest_no_coal");
}
```
