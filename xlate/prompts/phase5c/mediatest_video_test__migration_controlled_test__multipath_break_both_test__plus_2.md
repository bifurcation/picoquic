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

## `picoquictest/mediatest.c:mediatest_video_test`
* C test-table name: `mediatest_video`
* C entry function: `mediatest_video_test`
* Rust test: `mediatest_video`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1413-1421`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper sets the same spec fields, but Rust mediatest_one reduces the media simulation to a generic send/receive scenario and does not check the C media frame pacing or video latency statistics.
* Phase 5A fix note: Implement/use a faithful media-test driver with media stream generation, custom loop, completion detection, and mediatest_check_stats-equivalent checks for the video stream.
* Phase 5B analysis: Rust test is present, compiles under the Rust test harness, and matches the C API-level contract: BBR, bandwidth 0.01, do_video=true, and mediatest_one(Video). Prior callback/STREAM/runtime failures are Phase 5C implementation notes, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    ret = mediatest_one(mediatest_video, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_video() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video, &spec).expect("mediatest_video");
}
```

## `picoquictest/multipath_test.c:migration_controlled_test`
* C test-table name: `migration_controlled`
* C entry function: `migration_controlled_test`
* Rust test: `migration_controlled`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1390-1392`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper and migration helper preserve the C inputs and migration flow, including CID, scenario, path probing, old-link removal, and address checks, but the shared Rust final verifier omits the C transfer-completion and deadline checks.
* Phase 5A fix note: Repair the shared Rust scenario/body verifier to check stream delivery, callback errors, accounting, and max completion time; migration_controlled otherwise matches the C test intent.
* Phase 5B analysis: Rust #[test] migration_controlled calls migration_test_one(false), matching C migration_controlled_test -> migration_test_one(0). The shared Rust helper expresses the same API-level migration flow and visible assertions; current probe_new_path runtime failure is Phase 5C implementation work, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return migration_test_one(0);
}
```

### Current Rust test body
```rust
fn migration_controlled() {
    migration_test_one(false);
}
```

## `picoquictest/multipath_test.c:multipath_break_both_test`
* C test-table name: `multipath_break_both`
* C entry function: `multipath_break_both_test`
* Rust test: `multipath_break_both`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1584-1586`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust selects BreakBoth with the right bound, but helper behavior is inverted from C: C expects transfer failure after both paths become unreachable and converts that failure to success; Rust still expects the data loop/verify to succeed and would also pass if it unexpectedly reaches the end.
* Phase 5A fix note: Phase 5B should special-case BreakBoth by capturing the final data/verify result, accepting the expected failure, and failing if the transfer completes successfully.
* Phase 5B analysis: Rust #[test] calls multipath_test_one(1_060_000, MultipathTestId::BreakBoth), matching the C wrapper's API-level contract; any early multipath negotiation/runtime failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 1060000;

    return multipath_test_one(max_completion_microsec, multipath_test_break_both);
}
```

### Current Rust test body
```rust
fn multipath_break_both() {
    multipath_test_one(1_060_000, MultipathTestId::BreakBoth);
}
```

## `picoquictest/multipath_test.c:multipath_sat_plus_test`
* C test-table name: `multipath_sat_plus`
* C entry function: `multipath_sat_plus_test`
* Rust test: `multipath_sat_plus`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1692-1694`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper passes the same 10,000,000us limit and SatPlus test id, but the Rust wait_multipath_ready helper only spins and returns Ok; the C helper fails unless both sides have the second path challenge-verified before transfer.
* Phase 5A fix note: Make rs/fq/src/tests/util.rs::wait_multipath_ready mirror the C readiness condition and final failure check for two verified paths on client and server.
* Phase 5B analysis: Rust test is present, compiles under the Rust test harness, and calls multipath_test_one with the C 10,000,000us limit and SatPlus id; any early server-handshake/runtime failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 10000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_sat_plus);
}
```

### Current Rust test body
```rust
fn multipath_sat_plus() {
    multipath_test_one(10_000_000, MultipathTestId::SatPlus);
}
```

## `picoquictest/pacing_test.c:pacing_fast_test`
* C test-table name: `pacing_fast`
* C entry function: `pacing_fast_test`
* Rust test: `pacing_fast`
* Expected Rust file: `rs/fq/src/tests/pacing.rs`
* Current Rust span: `rs/fq/src/tests/pacing.rs:864-866`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses fastcc, the same data size, and the same loss target, but the target_time check is weakened because the shared Rust verifier ignores the deadline. The Rust helper also leaves the algorithm-number byte in the initial CID as zero instead of setting it from the selected congestion algorithm as C does.
* Phase 5A fix note: Enforce target_time in the Rust verifier and set the pacing helper initial CID byte to the selected congestion algorithm number.
* Phase 5B analysis: Rust pacing_fast is present as a #[test], compiles under the Rust test harness, and calls pacing_cc_algotest with FastCC plus the same target_time=1000000 and loss_target=180 as C. The known early runtime failure is a Phase 5C library-behavior issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = pacing_cc_algotest(picoquic_fastcc_algorithm, 1000000, 180);
    return ret;
}
```

### Current Rust test body
```rust
fn pacing_fast() {
    pacing_cc_algotest(&PACING_FASTCC_ALGORITHM, 1_000_000, 180);
}
```
