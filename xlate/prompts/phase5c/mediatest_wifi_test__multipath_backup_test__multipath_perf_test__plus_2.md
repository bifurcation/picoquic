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

## `picoquictest/mediatest.c:mediatest_wifi_test`
* C test-table name: `mediatest_wifi`
* C entry function: `mediatest_wifi_test`
* Rust test: `mediatest_wifi`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1506-1525`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust spec fields match the C entry, but the shared Rust mediatest driver misses the C media simulation/stat checks and does not assert the wifi-specific path-quality loss, spurious-loss, and timer-loss conditions.
* Phase 5A fix note: Repair Rust mediatest_one as above, reproduce the back-to-back suspension/drain semantics for suspension_up_time=0, and add the wifi-specific path quality assertions for lost, spurious_losses, and timer_losses.
* Phase 5B analysis: Rust test is present as a runnable #[test], matches the C spec fields and mediatest_one(MediatestId::Wifi) call, and the Rust media driver includes the Wi-Fi path-quality assertions. Prior handshake/startup failure is a Phase 5C library-runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.link_latency = 15000;
    spec.latency_average = 60000;
    spec.latency_max = 350000;
    spec.priority_limit_for_bypass = 5;
    spec.do_not_check_video2 = 1;
    spec.nb_suspensions = 20;
    spec.suspension_start_time = 4000000;
    spec.suspension_down_time = 150000;
    spec.suspension_up_time = 0;

    ret = mediatest_one(mediatest_wifi, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_wifi() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        link_latency: 15_000,
        latency_average: 60_000,
        latency_max: 350_000,
        priority_limit_for_bypass: 5,
        do_not_check_video2: true,
        nb_suspensions: 20,
        suspension_start_time: 4_000_000,
        suspension_down_time: 150_000,
        suspension_up_time: 0,
        ..Default::default()
    };
    mediatest_one(MediatestId::Wifi, &spec).expect("mediatest_wifi");
}
```

## `picoquictest/multipath_test.c:multipath_backup_test`
* C test-table name: `multipath_backup`
* C entry function: `multipath_backup_test`
* Rust test: `multipath_backup`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1566-1568`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper and Backup enum value match C, and Rust includes backup-path assertions, but it still relies on placeholder shared helpers that skip scenario completion and max-completion checks from C.
* Phase 5A fix note: Restore shared scenario verification so multipath_test_one checks completed transfers, payloads, close result, and max_completion_microsec in addition to backup-path assertions.
* Phase 5B analysis: Rust #[test] multipath_backup calls multipath_test_one(2_000_000, MultipathTestId::Backup), matching the C entry point and exercising the shared backup-path API-visible assertions. The previous multipath negotiation failure is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 2000000;

    return multipath_test_one(max_completion_microsec, multipath_test_backup);
}
```

### Current Rust test body
```rust
fn multipath_backup() {
    multipath_test_one(2_000_000, MultipathTestId::Backup);
}
```

## `picoquictest/multipath_test.c:multipath_perf_test`
* C test-table name: `multipath_perf`
* C entry function: `multipath_perf_test`
* Rust test: `multipath_perf`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1656-1658`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes Perf and sets perf links/BBR/long scenario, but it drops C's send_buffer_size=65536 setup and the 1.65s completion limit is ignored by the shared verifier.
* Phase 5A fix note: Preserve the perf send-buffer setup or an equivalent Rust harness behavior, and fix shared scenario completion/time verification.
* Phase 5B analysis: Rust #[test] multipath_perf is present and calls multipath_test_one(1_650_000, MultipathTestId::Perf), matching the C API-level contract multipath_test_one(1650000, multipath_test_perf). The shared Rust helper already covers the perf-specific API-visible setup: send_buffer_size 65536, perf link setup on both paths, BBR registration/default selection, long multipath scenario, wait_multipath_ready, and max_completion_microsec verification. The prior delayed-client/server-accept failure is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 1650000;

    return  multipath_test_one(max_completion_microsec, multipath_test_perf);
}
```

### Current Rust test body
```rust
fn multipath_perf() {
    multipath_test_one(1_650_000, MultipathTestId::Perf);
}
```

## `picoquictest/pacing_test.c:pacing_bbr_test`
* C test-table name: `pacing_bbr`
* C entry function: `pacing_bbr_test`
* Rust test: `pacing_bbr`
* Expected Rust file: `rs/fq/src/tests/pacing.rs`
* Current Rust span: `rs/fq/src/tests/pacing.rs:846-848`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The pacing helper and thresholds match, but Rust's registered bbr descriptor is backed by BASELINE_CC, so the test does not actually exercise BBRv3 behavior like the C test.
* Phase 5A fix note: Wire the Rust bbr congestion descriptor to the translated BBR CongestionControl, or block this test until that implementation is available; also mirror the C helper's algorithm-number CID byte when touching the helper.
* Phase 5B analysis: Reclassified ok: Rust pacing_bbr is a #[test] that calls pacing_cc_algotest(&PACING_BBR_ALGORITHM, 900_000, 160), matching the C API-level call and loss target. The helper mirrors the C-visible setup and assertions, including algorithm-number CID, selected congestion algorithm, scenario completion verification, and observed_loss <= 160. Any BBR completion/runtime failure is Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* BBRv3 includes a short term loop that detects losses and tune the
    * sending rate accordingly. The packet losses cause startup to 
    * give up too soon, but this is fixed by probing up "quickly"
    * after exiting startup. The packet losses occur during startup
    * and during the probing periods.
    */
    int ret = pacing_cc_algotest(picoquic_bbr_algorithm, 900000, 160);
    return ret;
}
```

### Current Rust test body
```rust
fn pacing_bbr() {
    pacing_cc_algotest(&PACING_BBR_ALGORITHM, 900_000, 160);
}
```

## `picoquictest/satellite_test.c:satellite_cubic_loss_test`
* C test-table name: `satellite_cubic_loss`
* C entry function: `satellite_cubic_loss_test`
* Rust test: `satellite_cubic_loss`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:446-461`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper parameters match C, but the Rust satellite helper passes data_size into an ignored helper parameter, uses an empty scenario, never sets stream0_target, and ignores the completion-time check, so it does not test the C 100MB lossy cubic transfer within 7.5s.
* Phase 5A fix note: Map data_size to stream0_target, drive and verify the stream0 transfer with the loss mask, and enforce max_completion_time in the Rust scenario helper/verification.
* Phase 5B analysis: Rust test is present, compiled by the test harness, and matches the C API-level contract: cubic congestion control, 100000000 bytes, 7500000 max completion time, 250/3 Mbps, no jitter, loss enabled, and no preemptive/seed/flow-control options. The stream0 active-send/PrepareToSend runtime failure is a Phase 5C implementation issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    /* Should be less than 10 sec per draft etosat, but cubic is a bit slower */
    return satellite_test_one(picoquic_cubic_algorithm, 100000000, 7500000, 250, 3, 0, 1, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_cubic_loss() {
    let cubic = satellite_ccalgo("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        7_500_000,
        250,
        3,
        0,
        true,
        false,
        false,
        false,
        false,
    );
}
```
