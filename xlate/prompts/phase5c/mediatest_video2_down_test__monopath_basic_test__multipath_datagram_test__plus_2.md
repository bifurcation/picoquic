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

## `picoquictest/mediatest.c:mediatest_video2_down_test`
* C test-table name: `mediatest_video2_down`
* C entry function: `mediatest_video2_down_test`
* Rust test: `mediatest_video2_down`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1453-1466`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust spec fields match the C test body, but Rust mediatest_one does not check the same behavior: it does not run the staged 4s normal/24s down/restore simulation, does not collect/check media latency stats, and does not use do_not_check_video2 for stats selection.
* Phase 5A fix note: Repair mediatest_one or this test helper to reproduce the C media loop, bandwidth drop/restore timing, completion check, and audio/video stats checks while skipping video2 stats for this case.
* Phase 5B analysis: Rust test matches the C API-level contract and compiles/runs under the Rust test harness. Prior Generic runtime failure is Phase 5C library behavior, not a Phase 5B block.
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
    spec.latency_average = 100000;
    spec.latency_max = 600000;
    spec.do_not_check_video2 = 1;
    ret = mediatest_one(mediatest_video2_down, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_video2_down() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 100_000,
        latency_max: 600_000,
        do_not_check_video2: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Down, &spec).expect("mediatest_video2_down");
}
```

## `picoquictest/multipath_test.c:monopath_basic_test`
* C test-table name: `monopath_basic`
* C entry function: `monopath_basic_test`
* Rust test: `monopath_basic`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1450-1452`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Wrapper maps Basic correctly, but the Rust shared verifier ignores max_completion_microsec and omits the C stream/callback completion checks, weakening the test.
* Phase 5A fix note: Implement Rust scenario verification equivalent to tls_api_one_scenario_verify plus completion-time enforcement in tls_api_one_scenario_body_verify.
* Phase 5B analysis: Reclassified as ok: Rust monopath_basic is a #[test] that calls monopath_test_one(MonopathTestId::Basic), and the shared helper expresses the C API-level setup, scenario/data-loop, completion-time, stream0, and data-node-pool checks. Any early runtime failure from incomplete callback/stream behavior is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    return monopath_test_one(monopath_test_basic);
}
```

### Current Rust test body
```rust
fn monopath_basic() {
    monopath_test_one(MonopathTestId::Basic);
}
```

## `picoquictest/multipath_test.c:multipath_datagram_test`
* C test-table name: `multipath_datagram`
* C entry function: `multipath_datagram_test`
* Rust test: `multipath_datagram`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1596-1598`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The entry constants match, but Rust helper behavior is weaker: common scenario verification ignores max_completion_microsec, and datagram receive/path counters are updated when queueing datagrams rather than by real receive callbacks as in C.
* Phase 5A fix note: Make multipath datagram helpers use real datagram send/receive/ack callbacks and real path accounting, and make scenario verification enforce completion and the 1,150,000 us bound.
* Phase 5B analysis: Rust test is present as a runnable #[test], calls multipath_test_one(1_150_000, MultipathTestId::Datagram), and the shared helper expresses the C API-level datagram setup, ready marking, send loop, completion bound, and path-balance assertions. Prior callback-plumbing concerns are Phase 5C runtime/library behavior notes, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    /* TODO: investigate why 1.15 instead of 1.12 with prior implementation of multipath */
    uint64_t max_completion_microsec = 1150000;

    return multipath_test_one(max_completion_microsec, multipath_test_datagram);
}
```

### Current Rust test body
```rust
fn multipath_datagram() {
    multipath_test_one(1_150_000, MultipathTestId::Datagram);
}
```

## `picoquictest/multipath_test.c:multipath_standup_test`
* C test-table name: `multipath_standup`
* C entry function: `multipath_standup_test`
* Rust test: `multipath_standup`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1710-1712`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper and Standup helper path match the C intent, including max time and Standup test id, but the final Rust scenario-body verifier omits the C stream-completion and max-time checks.
* Phase 5A fix note: Fix the shared Rust scenario body verifier so Standup verifies transfer completion and the 7,200,000 usec bound.
* Phase 5B analysis: Rust Standup already uses the C max time and test id, and the shared verifier checks stream completion plus the 7,200,000 usec bound.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 7200000;

    return multipath_test_one(max_completion_microsec, multipath_test_standup);
}
```

### Current Rust test body
```rust
fn multipath_standup() {
    multipath_test_one(7_200_000, MultipathTestId::Standup);
}
```

## `picoquictest/qlog_test.c:qlog_auto_test`
* C test-table name: `qlog_auto`
* C entry function: `qlog_auto_test`
* Rust test: `qlog_auto`
* Expected Rust file: `rs/fq/src/tests/qlog.rs`
* Current Rust span: `rs/fq/src/tests/qlog.rs:73-78`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust preserves the four helper calls but does not exercise the same qlog/binlog new-connection logging behavior as C; the translated set_qlog/start_client path only stores paths and does not open or dispatch qlog logging. autoqlog_no_binlog also changes binlog_file_name before start_client instead of after it.
* Phase 5A fix note: Make the Rust helpers exercise the translated qlog/binlog new-connection path like C, preserve autoqlog_no_binlog ordering, and cover bad qlog path, missing binlog, long directory, and unique-name cases with the intended no-error behavior.
* Phase 5B analysis: Rust qlog_auto is present, is a #[test], compiles under the Rust test harness, and calls the same four autoqlog helpers in C order. The prior qlog/binlog backend/open-path concern is a Phase 5C runtime-library note, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
	int ret = autoqlog_bad_file();

	if (ret == 0) {
		ret = autoqlog_no_binlog();
	}

	if (ret == 0) {
		ret = autoqlog_longdir();
	}

	if (ret == 0) {
		ret = autoqlog_unique();
	}

	return ret;
}
```

### Current Rust test body
```rust
fn qlog_auto() {
    autoqlog_bad_file().expect("autoqlog_bad_file");
    autoqlog_no_binlog().expect("autoqlog_no_binlog");
    autoqlog_longdir().expect("autoqlog_longdir");
    autoqlog_unique().expect("autoqlog_unique");
}
```
