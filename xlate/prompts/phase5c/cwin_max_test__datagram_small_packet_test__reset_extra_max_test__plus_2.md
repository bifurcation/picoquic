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

## `picoquictest/congestion_test.c:cwin_max_test`
* C test-table name: `cwin_max`
* C entry function: `cwin_max_test`
* Rust test: `cwin_max`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:972-982`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test uses the same algorithm list, limits, and qlog bytes-in-flight check, but cwin_max_test_one passes zeroed TransportParameters::default() where C initializes defaults, changing initial_max_data/max-data control. The shared Rust scenario verifier also ignores max_completion_time.
* Phase 5A fix note: Initialize client_params with init_transport_parameters before passing/using it, and fix shared scenario completion/max-time verification.
* Phase 5B analysis: Rust cwin_max iterates the same six congestion algorithms with the same max-completion limits and delegates to a helper that matches the C API-level setup and bytes-in-flight assertion. The prior early Generic runtime failure is a Phase 5C implementation/harness behavior note, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgos[] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_bbr_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr1_algorithm
    };
    uint64_t max_completion_times[] = {
        11000000,
        11000000,
        11000000,
        11000000,
        12100000,
        11000000
    };
    int ret = 0;

    for (size_t i = 0; i < sizeof(ccalgos) / sizeof(picoquic_congestion_algorithm_t*); i++) {
        ret = cwin_max_test_one(ccalgos[i], 68000, max_completion_times[i]);
        if (ret != 0) {
            DBG_PRINTF("CWIN Max test fails for <%s>", ccalgos[i]->congestion_algorithm_id);
            break;
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn cwin_max() {
    let algo_names = ["newreno", "cubic", "dcubic", "bbr", "fastcc", "bbr1"];
    let max_completion_times: [u64; 6] = [
        11_000_000, 11_000_000, 11_000_000, 11_000_000, 12_100_000, 11_000_000,
    ];

    for (name, &max_time) in algo_names.iter().zip(max_completion_times.iter()) {
        let ccalgo = cc_algo(name);
        cwin_max_test_one(ccalgo, 68_000, max_time);
    }
}
```

## `picoquictest/datagram_tests.c:datagram_small_packet_test`
* C test-table name: `datagram_small_packet`
* C entry function: `datagram_small_packet_test`
* Rust test: `datagram_small_packet`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Current Rust span: `rs/fq/src/tests/datagram.rs:756-773`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper preserves the C field values, but the underlying Rust datagram helper is synthetic: it bypasses actual QUIC DATAGRAM frames/callbacks, overwrites negotiation results, and ignores bandwidth-sensitive delivery behavior from picosec_per_byte, weakening duration, packet, and latency checks.
* Phase 5A fix note: Repair the shared Rust datagram helper to use the real simulator/datagram provider path, preserve the small-packet inputs, assert negotiated parameters, and base packet/duration/latency checks on actual connection behavior.
* Phase 5B analysis: Rust #[test] is present, compiles under the test harness, uses the same C scenario fields, and calls datagram_test_one(9, &mut dg_ctx, 0). Any runtime failures from incomplete DATAGRAM negotiation/provider/recv/ack behavior are Phase 5C implementation issues, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_small_size = 64;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 20000;
    dg_ctx.send_delay = 100;
    dg_ctx.next_gen_time[0] = 50000;
    dg_ctx.next_gen_time[1] = 50000;
    dg_ctx.link_latency = 10000;
    dg_ctx.picosec_per_byte = 20000; /* 400 Mbps */
    dg_ctx.dg_latency_target[0] = 20000;
    dg_ctx.dg_latency_target[1] = 13500;
    dg_ctx.use_extended_provider_api = 1;
    dg_ctx.one_datagram_per_packet = 1;
    dg_ctx.nb_trials_max = 200000;
    dg_ctx.duration_max = 2060000;

    return datagram_test_one(9, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_small_packet() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        dg_target: [100, 20_000],
        send_delay: 100,
        next_gen_time: [50_000, 50_000],
        link_latency: 10_000,
        picosec_per_byte: 20_000, // 400 Mbps
        dg_latency_target: [20_000, 13_500],
        use_extended_provider_api: true,
        one_datagram_per_packet: true,
        nb_trials_max: 200_000,
        duration_max: 2_060_000,
        ..Default::default()
    };
    datagram_test_one(9, &mut dg_ctx, 0);
}
```

## `picoquictest/edge_cases.c:reset_extra_max_test`
* C test-table name: `reset_extra_max`
* C entry function: `reset_extra_max_test`
* Rust test: `reset_extra_max`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1869-1871`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust exercises the extra MAX_STREAM_DATA-after-reset path and no-stream-recreation assertion, but ResetTestKind discriminants are shifted because the C enum's AckStopSending slot is omitted, so the initial CID test id differs from C for ExtraMaxStream.
* Phase 5A fix note: Give ResetTestKind explicit C-compatible discriminants or add the missing AckStopSending slot so ExtraMaxStream uses test id 3.
* Phase 5B analysis: Rust now matches the C API-level contract: reset_extra_max is a #[test] that calls reset_repeat_test_one with ExtraMaxStream value 3, decodes the extra MAX_STREAM_DATA frame, and asserts stream 4 is not recreated. Any current InvalidState/no-stream-recreation runtime failure is Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return reset_repeat_test_one(reset_extra_max_stream);
}
```

### Current Rust test body
```rust
fn reset_extra_max() {
    reset_repeat_test_one(ResetTestKind::ExtraMaxStream).expect("reset_extra_max");
}
```

## `picoquictest/high_latency_test.c:high_latency_bbr_test`
* C test-table name: `high_latency_bbr`
* C entry function: `high_latency_bbr_test`
* Rust test: `high_latency_bbr`
* Expected Rust file: `rs/fq/src/tests/high_latency.rs`
* Current Rust span: `rs/fq/src/tests/high_latency.rs:293-309`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper passes the same visible parameters, but Rust high_latency_one does not set the existing client connection to the requested congestion algorithm as C does, so the test does not actually exercise the BBR client-transfer behavior.
* Phase 5A fix note: Update Rust high_latency_one to call cnx_client().set_congestion_algorithm(ccalgo) in addition to setting the server default; ensure the bbr registry entry is not a placeholder when this test is meant to validate BBR behavior.
* Phase 5B analysis: Current Rust test is present as a runnable #[test] and matches the C API-level contract: it selects "bbr", calls high_latency_one with the same test id, scenario, completion target, latency, bandwidth, jitter/loss/preemptive/seed settings, and the helper installs the selected congestion algorithm on both server default and client connection. The current registry mapping of "bbr" to BASELINE_CC is a Phase 5C implementation gap, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t latency = 5000000;
    uint64_t expected_completion = 145000000;

    return high_latency_one(0xbb, picoquic_bbr_algorithm,
        hilat_scenario_100mb, sizeof(hilat_scenario_100mb),
        expected_completion, latency, 10, 10, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn high_latency_bbr() {
    let latency = 5_000_000u64;
    let bbr = high_latency_ccalgo("bbr");
    high_latency_one(
        0xbb,
        bbr,
        HILAT_SCENARIO_100MB,
        145_000_000,
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

## `picoquictest/mediatest.c:mediatest_suspension2_test`
* C test-table name: `mediatest_suspension2`
* C entry function: `mediatest_suspension2_test`
* Rust test: `mediatest_suspension2`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1582-1596`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry spec fields match the C wrapper, but the shared Rust mediatest_one is materially different from the C media harness: it runs generic TLS stream descriptors and does not reproduce media frame generation, media latency stats, or meaningful video2/probe-up checks.
* Phase 5A fix note: Port or implement an equivalent mediatest harness for audio/video/video2 frame scheduling, probe-up behavior, completion, and latency-stat checks; then keep this entry's spec values.
* Phase 5B analysis: Rust #[test] is present, compiles under the test harness, and mirrors the C wrapper's API-visible contract: BBR, bandwidth 0.1, video/video2/audio enabled, latency bounds, video2 check suppression, probe-up, and MediatestId::Suspension2. Any TLS/simulation readiness failure is Phase 5C runtime behavior, not a Phase 5B block.
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
    spec.do_probe_up = 1;
    ret = mediatest_one(mediatest_suspension2, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_suspension2() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 50_000,
        latency_max: 300_000,
        do_not_check_video2: true,
        do_probe_up: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Suspension2, &spec).expect("mediatest_suspension2");
}
```
