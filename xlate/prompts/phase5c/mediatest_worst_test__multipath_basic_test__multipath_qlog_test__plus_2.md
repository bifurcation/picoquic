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

## `picoquictest/mediatest.c:mediatest_worst_test`
* C test-table name: `mediatest_worst`
* C entry function: `mediatest_worst_test`
* Rust test: `mediatest_worst`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1529-1539`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper spec fields match, but the runner is not equivalent. C mediatest_worst runs the custom media simulation with disruption_clear=2500000, one normal second, one lossy second, then stats checks; Rust sets loss_mask=u64::MAX around a generic stream scenario and never checks media latencies.
* Phase 5A fix note: Implement the C worst-case media schedule and stats checks in the Rust mediatest harness instead of using generic stream transfer/loss_mask logic.
* Phase 5B analysis: Rust mediatest_worst mirrors the C spec fields and mediatest_one(MediatestId::Worst, &spec) call. The Worst harness path includes the API-visible disruption_clear/stat checks and 1s normal plus 1s lossy window; any early idle-out/runtime media failure is Phase 5C behavior, not a Phase 5B block.
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
    ret = mediatest_one(mediatest_worst, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_worst() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::Worst, &spec).expect("mediatest_worst");
}
```

## `picoquictest/multipath_test.c:multipath_basic_test`
* C test-table name: `multipath_basic`
* C entry function: `multipath_basic_test`
* Rust test: `multipath_basic`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1572-1574`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper and multipath_basic helper broadly match the C flow, including max completion value, CID, multipath negotiation, scenario, second-path probing, readiness, and CID availability checks, but the shared Rust final verifier skips C's stream/data completion and deadline assertions.
* Phase 5A fix note: Fix the shared Rust scenario/body verifier to implement the C body verification semantics; the multipath_basic-specific setup and assertions are otherwise aligned.
* Phase 5B analysis: Rust `multipath_basic` is a present, runnable `#[test]` that calls `multipath_test_one(1_060_000, MultipathTestId::Basic)`, matching the C wrapper's API-level contract. Any early runtime failure from incomplete multipath negotiation is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 1060000;

    return multipath_test_one(max_completion_microsec, multipath_test_basic);
}
```

### Current Rust test body
```rust
fn multipath_basic() {
    multipath_test_one(1_060_000, MultipathTestId::Basic);
}
```

## `picoquictest/multipath_test.c:multipath_qlog_test`
* C test-table name: `multipath_qlog`
* C entry function: `multipath_qlog_test`
* Rust test: `multipath_qlog`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1662-1672`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs the qlog trace scenario, but compares multipath_qlog_test.qlog while C compares 0807060504030201.server.qlog. The Rust trace helper also omits the final bad-packet injection that C performs before closing the context and comparing the qlog reference.
* Phase 5A fix note: Compare the generated 0807060504030201.server.qlog against the reference and add the missing bad-packet injection in the Rust trace helper if the reference expects that log event.
* Phase 5B analysis: Rust test is present, compiles as a harness test, deletes 0807060504030201.server.qlog, runs the qlog variant, injects the final malformed packet through the helper, and compares against picoquictest/multipath_qlog_ref.txt. probe_new_path still returns Generic, but that is a Phase 5C runtime implementation failure, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    (void)picoquic_file_delete(MULTIPATH_TRACE_QLOG, NULL);

    ret = multipath_trace_test_one(1);

    /* compare the log file to the expected value */
    if (ret == 0)
    {
        char qlog_trace_test_ref[512];

        ret = picoquic_get_input_path(qlog_trace_test_ref, sizeof(qlog_trace_test_ref),
            picoquic_solution_dir, MULTIPATH_QLOG_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the qlog trace test ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(MULTIPATH_TRACE_QLOG, qlog_trace_test_ref);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn multipath_qlog() {
    const MULTIPATH_TRACE_QLOG: &str = "0807060504030201.server.qlog";
    const MULTIPATH_QLOG_REF: &str = "picoquictest/multipath_qlog_ref.txt";

    // Delete any existing qlog file.
    let _ = std::fs::remove_file(MULTIPATH_TRACE_QLOG);

    multipath_trace_test_one(true);

    compare_text_files(MULTIPATH_TRACE_QLOG, MULTIPATH_QLOG_REF).expect("qlog matches reference");
}
```

## `picoquictest/pacing_test.c:pacing_cubic_test`
* C test-table name: `pacing_cubic`
* C entry function: `pacing_cubic_test`
* Rust test: `pacing_cubic`
* Expected Rust file: `rs/fq/src/tests/pacing.rs`
* Current Rust span: `rs/fq/src/tests/pacing.rs:852-854`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust pacing cubic test mirrors the congestion algorithm, target time, loss threshold, and scenario flow, but its final body verifier does not check completion/time. The helper also does not mirror the C assignment of the selected algorithm number into initial_cid.id[4].
* Phase 5A fix note: Restore scenario completion/max-time checks in the shared verifier; mirror the C initial CID algorithm-byte setup if that field is still used for pacing test identity.
* Phase 5B analysis: Rust #[test] pacing_cubic is present, compiles under the Rust test harness, and calls pacing_cc_algotest with the same API-visible contract as C: CUBIC algorithm, 900000 target time, and 210 loss target. The known unfinished runtime scenario is a Phase 5C implementation failure, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = pacing_cc_algotest(picoquic_cubic_algorithm, 900000, 210);
    return ret;
}
```

### Current Rust test body
```rust
fn pacing_cubic() {
    pacing_cc_algotest(&PACING_CUBIC_ALGORITHM, 900_000, 210);
}
```

## `picoquictest/satellite_test.c:satellite_cubic_seeded_test`
* C test-table name: `satellite_cubic_seeded`
* C entry function: `satellite_cubic_seeded_test`
* Rust test: `satellite_cubic_seeded`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:427-442`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Wrapper parameters match C, but the shared Rust satellite helper does not preserve C behavior: C passes data_size as stream0_target to tls_api_one_scenario_body, while Rust passes an empty scenario and puts data_size in an ignored helper argument, so the 100 MB stream-0 transfer is not tested.
* Phase 5A fix note: Make Rust satellite_test_one drive stream0_target=data_size, either by using/fixing a scenario-body helper with the C signature or by setting TestTlsApiCtx stream0_target before the data loop.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and calls satellite_test_one with the same cubic algorithm and API-visible arguments as the C test. The active-stream/PrepareToSend runtime failure is a Phase 5C implementation issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    return satellite_test_one(picoquic_cubic_algorithm, 100000000, 5000000, 250, 3, 0, 0, 0, 1, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_cubic_seeded() {
    let cubic = satellite_ccalgo("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        5_000_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}
```
