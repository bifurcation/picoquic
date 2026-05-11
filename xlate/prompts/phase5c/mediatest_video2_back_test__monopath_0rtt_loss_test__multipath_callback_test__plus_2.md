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

## `picoquictest/mediatest.c:mediatest_video2_back_test`
* C test-table name: `mediatest_video2_back`
* C entry function: `mediatest_video2_back_test`
* Rust test: `mediatest_video2_back`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1471-1484`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry sets the same spec values, but its mediatest_one helper is a generic stream transfer with a permanent bandwidth change and completion-bound check, not the C media-frame simulator with staged down/back timing and audio/video latency statistics.
* Phase 5A fix note: Implement the C-style media harness for this path: run to 2,000,000 us, reduce bandwidth to 8,000,000 ps/byte until 4,000,000 us, restore link settings, finish by 30,000,000 us, require completion, check audio and video stats against average/max latency, and skip only video2 stats.
* Phase 5B analysis: Rust #[test] is present, compiles under the Rust test harness, builds the same API-visible MediatestSpec fields as C, and calls mediatest_one(MediatestId::Video2Back, &spec). Any runtime disconnect/no-server-connection failure is a Phase 5C implementation/harness issue, not a Phase 5B block.
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
    spec.latency_average = 80000;
    spec.latency_max = 500000;
    spec.do_not_check_video2 = 1;
    ret = mediatest_one(mediatest_video2_back, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_video2_back() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 80_000,
        latency_max: 500_000,
        do_not_check_video2: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Back, &spec).expect("mediatest_video2_back");
}
```

## `picoquictest/multipath_test.c:monopath_0rtt_loss_test`
* C test-table name: `monopath_0rtt_loss`
* C entry function: `monopath_0rtt_loss_test`
* Rust test: `monopath_0rtt_loss_2`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1436-1446`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The outer Rust loop matches i=1..15, early_loss, and do_multipath, but the Rust zero_rtt_test_one weakens the C behavior: it starts the client before setting some parameters and manually seeds 0RTT/PSK and multipath flags, so packet-loss recovery and multipath negotiation are not actually observed.
* Phase 5A fix note: Repair zero_rtt_test_one to use delayed init like C, inject early_loss through the simulator, and assert actual 0RTT counters, PSK handshake, ticket handling, and multipath negotiation without manual state seeding.
* Phase 5B analysis: Rust #[test] monopath_0rtt_loss_2 matches the C API-level contract: it iterates packets 1..15, sets early_loss=1<<i and do_multipath=true, and calls zero_rtt_test_one. Runtime failures in ticket/PSK/0-RTT counters or multipath negotiation are Phase 5C implementation issues, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    for (unsigned int i = 1; ret == 0 && i < 16; i++) {
        zero_rtt_test_t zrt = { 0 };
        zrt.early_loss = 1ull << i;
        zrt.do_multipath = 1;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Monopath 0 RTT test fails when packet #%d is lost.\n", i);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn monopath_0rtt_loss_2() {
    for i in 1u32..16 {
        let zrt = ZeroRttTest {
            early_loss: 1u64 << i,
            do_multipath: true,
            ..ZeroRttTest::default()
        };
        zero_rtt_test_one(&zrt)
            .unwrap_or_else(|_| panic!("monopath_0rtt_loss_2 fails at packet #{i}"));
    }
}
```

## `picoquictest/multipath_test.c:multipath_callback_test`
* C test-table name: `multipath_callback`
* C entry function: `multipath_callback_test`
* Rust test: `multipath_callback`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1590-1592`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The mapped C body is the Win32 no-op branch returning 0, while the Rust test runs the non-Win callback scenario and log comparison.
* Phase 5A fix note: Fix the mapping/source span to the active non-Win C branch for the v1 target, or gate/change the Rust test to no-op if the Win32 branch is truly intended.
* Phase 5B analysis: For the v1 Linux target, the active C branch calls multipath_test_one(1000000, multipath_test_callback); the Win32 no-op branch is non-target. The Rust multipath_callback test already calls multipath_test_one(1_000_000, MultipathTestId::Callback) and uses the shared callback log comparison path.
* Phase 5B fix note: 

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Current Rust test body
```rust
fn multipath_callback() {
    multipath_test_one(1_000_000, MultipathTestId::Callback);
}
```

## `picoquictest/multipath_test.c:multipath_socket_error_test`
* C test-table name: `multipath_socket_error`
* C entry function: `multipath_socket_error_test`
* Rust test: `multipath_socket_error`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1704-1706`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper maps to Break2 with the correct 11,000,000 us timeout and does assert one remaining path after the socket error, but the shared verifier omits C's stream completion and timeout checks.
* Phase 5A fix note: Repair shared scenario verification/path readiness so the socket-error case checks both recovery/removal and successful transfer within the C timeout.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, compiles, and calls multipath_test_one(11_000_000, MultipathTestId::Break2); the shared helper covers the Break2 socket-error setup, final scenario verification, and one-path recovery assertions. Earlier handshake/path-probing runtime failures are Phase 5C implementation issues, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 11000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_break2);
}
```

### Current Rust test body
```rust
fn multipath_socket_error() {
    multipath_test_one(11_000_000, MultipathTestId::Break2);
}
```

## `picoquictest/pacing_test.c:pacing_newreno_test`
* C test-table name: `pacing_newreno`
* C entry function: `pacing_newreno_test`
* Rust test: `pacing_newreno`
* Expected Rust file: `rs/fq/src/tests/pacing.rs`
* Current Rust span: `rs/fq/src/tests/pacing.rs:870-872`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses the intended newreno algorithm and the same 900000 us and loss_target 100 parameters, but the shared Rust verifier ignores the C target-time and scenario-completion checks.
* Phase 5A fix note: Repair tls_api_one_scenario_body_verify to verify delivered scenario data and completion_time <= target_time; preserve the newreno pacing parameters.
* Phase 5B analysis: Rust #[test] pacing_newreno matches the C API-level contract: it calls pacing_cc_algotest with the NewReno algorithm, target_time 900000, and loss_target 100. Any runtime failure to complete by the target time is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = pacing_cc_algotest(picoquic_newreno_algorithm, 900000, 100);
    return ret;
}
```

### Current Rust test body
```rust
fn pacing_newreno() {
    pacing_cc_algotest(&PACING_NEWRENO_ALGORITHM, 900_000, 100);
}
```
