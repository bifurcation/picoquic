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

## `picoquictest/mediatest.c:mediatest_video_audio_test`
* C test-table name: `mediatest_video_audio`
* C entry function: `mediatest_video_audio_test`
* Rust test: `mediatest_video_audio`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1425-1434`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Wrapper spec fields match, but Rust mediatest_one checks a generic bulk stream transfer, not the C media harness that generates timed audio/video frames and verifies frame counts plus latency average/sigma/max stats.
* Phase 5A fix note: Phase 5B should make mediatest_one faithful for VideoAudio: generate the C-style 10s audio/video frame streams and assert mediatest_check_stats-equivalent frame counts and latency bounds, instead of only generic scenario completion.
* Phase 5B analysis: Rust #[test] is present, compiles under the test harness, and matches the C API-level contract: BBR congestion control, bandwidth 0.01, video/audio enabled, and MediatestId::VideoAudio passed to mediatest_one. Prior TLS simulator early-return/runtime failure is Phase 5C behavior, not a Phase 5B block.
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
    ret = mediatest_one(mediatest_video_audio, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_video_audio() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::VideoAudio, &spec).expect("mediatest_video_audio");
}
```

## `picoquictest/multipath_test.c:monopath_keep_alive_test`
* C test-table name: `monopath_keep_alive`
* C entry function: `monopath_keep_alive_test`
* Rust test: `monopath_keep_alive`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1462-1464`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper and helper follow the C keep-alive scenario, including parameters, scenario transfer, NAT change, PING loop, readiness checks, and extended completion bound, but the final shared verifier does not enforce the C stream completion or time-bound verification.
* Phase 5A fix note: Fix the shared Rust scenario/body verifier; the monopath keep-alive-specific flow appears acceptable once that verifier checks the same completion conditions as C.
* Phase 5B analysis: Rust `monopath_keep_alive` is a compiled `#[test]` and calls `monopath_test_one(MonopathTestId::KeepAlive)`, matching C `monopath_keep_alive_test` -> `monopath_test_one(monopath_keep_alive)`. Any keep-alive PING-loop stall is a Phase 5C runtime-library issue, not a Phase 5B block.
* Phase 5B fix note: No Rust test changes; reclassified only.

### C test body
```c
{
    return monopath_test_one(monopath_keep_alive);
}
```

### Current Rust test body
```rust
fn monopath_keep_alive() {
    monopath_test_one(MonopathTestId::KeepAlive);
}
```

## `picoquictest/multipath_test.c:multipath_discovery_test`
* C test-table name: `multipath_discovery`
* C entry function: `multipath_discovery_test`
* Rust test: `multipath_discovery`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1608-1610`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Discovery setup and address-observation assertions are present, but the shared final verifier still skips C's stream completion and 2.0s deadline checks.
* Phase 5A fix note: Fix the shared Rust scenario verifier; keep the existing discovery-specific nb_address_observed and local-vs-observed address checks.
* Phase 5B analysis: Rust #[test] is present, compiles under the Rust test harness, and calls multipath_test_one(2_000_000, MultipathTestId::Discovery), matching the C entry function's API-level contract. Any early multipath negotiation/runtime failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 2000000;

    return multipath_test_one(max_completion_microsec, multipath_test_discovery);
}
```

### Current Rust test body
```rust
fn multipath_discovery() {
    multipath_test_one(2_000_000, MultipathTestId::Discovery);
}
```

## `picoquictest/netperf_test.c:netperf_bbr_test`
* C test-table name: `netperf_bbr`
* C entry function: `netperf_bbr_test`
* Rust test: `netperf_bbr`
* Expected Rust file: `rs/fq/src/tests/netperf.rs`
* Current Rust span: `rs/fq/src/tests/netperf.rs:258-268`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper uses the right basic scenario, BBR algorithm, loss mask, target, and packet-size constant, but Rust does not run the C netperf coalesced-send loop with the supplied large send buffer and also misses C completion verification.
* Phase 5A fix note: Port/use the netperf send-buffer/coalescing loop semantics and enforce scenario verification plus the 1000000 us completion bound.
* Phase 5B analysis: Rust test matches the C API-level contract and compiles/runs under the Rust test harness. Any large-send-buffer or placeholder BBR runtime failure is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = netperf_one_scenario(netperf_scenario_basic, sizeof(netperf_scenario_basic),
        picoquic_bbr_algorithm,
        0, 0, 0, 0, 0, 1000000, NULL, NULL, 10 * PICOQUIC_MAX_PACKET_SIZE);

    return ret;
}
```

### Current Rust test body
```rust
fn netperf_bbr() {
    register_all_congestion_control_algorithms();
    let algo = get_congestion_algorithm("bbr").expect("bbr algorithm");
    netperf_one_scenario(
        NETPERF_SCENARIO_BASIC,
        Some(algo),
        0,
        1_000_000,
        10 * MAX_PACKET_SIZE,
    );
}
```

## `picoquictest/satellite_test.c:satellite_bbr1_test`
* C test-table name: `satellite_bbr1`
* C entry function: `satellite_bbr1_test`
* Rust test: `satellite_bbr1`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:389-404`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper passes the same bbr1/data/link parameters, but satellite_test_one calls a Rust tls_api_one_scenario_body variant that treats data_size as an ignored compatibility argument instead of C's stream0_target, so it does not perform the 100 MB stream-0 transfer; final completion verification is also close-only.
* Phase 5A fix note: Update satellite_test_one to use a helper/signature that sets stream0_target=data_size, preserves init_loss_mask and queue_delay_max=2*latency, and verifies stream0 completion plus max_completion_time.
* Phase 5B analysis: Rust satellite_bbr1 is present as a #[test], compiles under the Rust test harness, and calls satellite_test_one with the same API-level contract as C: bbr1, 100000000 bytes, 7000000 max completion time, 250/3 Mbps, and all option flags disabled. The known early CannotSetActiveStream/runtime stream0 callback failure is a Phase 5C implementation/harness behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Should be less than 7 sec per draft etosat */
    return satellite_test_one(picoquic_bbr1_algorithm, 100000000, 7000000, 250, 3, 0, 0, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_bbr1() {
    let bbr1 = satellite_ccalgo("bbr1");
    satellite_test_one(
        bbr1,
        100_000_000,
        7_000_000,
        250,
        3,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```
