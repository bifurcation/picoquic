# Phase 5B repair Rust test mismatches

You are repairing Phase 5A `needs_fix` entries.  The goal
is to make the Rust tests faithfully check the same behavior
as the C tests.

Rules:

* Edit Rust tests, Rust test helpers, and Rust test fixtures
  only: `rs/fq/src/tests/` and `rs/fq/tests/fixtures/`.
* Do not edit C sources.
* Do not weaken assertions, skip important C cases, or replace
  tests with placeholders.
* If the test already matches after closer inspection, report
  `ok` and do not edit source.
* Phase 5B is about test/API correspondence, not test success.
  The Rust test must exist, compile as a test, and be runnable
  by the Rust test harness, but it may fail arbitrarily early
  because the Rust library implementation is incomplete.
* Do not report `blocked` merely because the implementation
  returns the wrong state, fails a handshake, lacks protocol
  behavior, or would fail the test. Those are Phase 5C issues.
* Report `blocked` only when the faithful test cannot be
  written, compiled, or exposed as a runnable Rust test because
  the necessary Rust API/test-harness surface is missing or
  ambiguous.
* Do not run full `cargo test` in this pass. Use source review
  and, if needed, `cargo check --tests` for compile validation.

Owned Rust test file(s): `rs/fq/src/tests/spinbit.rs`, `rs/fq/src/tests/warptest.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/spinbit_test.c:spinbit_bad_test`
* C test-table name: `spinbit_bad`
* C entry function: `spinbit_bad_test`
* Rust test: `spinbit_bad`
* Expected Rust file: `rs/fq/src/tests/spinbit.rs`
* Rust span: `rs/fq/src/tests/spinbit.rs:168-173`
* Phase 5A analysis: Rust weakens the C test: C requires both invalid policy calls to fail, including raw out-of-range codes; Rust uses valid typed variants and only requires one call to error, while the helper ignores default-policy errors.
* Phase 5A fix note: Make the Rust test assert each invalid case independently. Propagate server default spinbit-policy errors in spinbit_test_one and add raw-code rejection coverage if the Rust API exposes a conversion path.
* Phase 5C outcome: blocked
* Phase 5C analysis: Current Rust test cannot express the C raw invalid spinbit values 123456 and 123455 because the Rust API exposes only the closed SpinbitVersion enum and typed setters.
* Phase 5C fix note: Expose a raw-code default/per-connection spinbit policy path, then assert both C invalid cases fail independently.

### C test body
```c
{
    int ret = 0;
    if (spinbit_test_one(picoquic_spinbit_on, 123456) == 0 ||
        spinbit_test_one(123455, picoquic_spinbit_null) == 0) {
        ret = -1;
    }
    return ret;
}
```

### Current Rust test body
```rust
fn spinbit_bad() {
    assert!(
        spinbit_test_one(SpinbitVersion::On, SpinbitVersion::Basic).is_err(),
        "expected server-only per-connection spinbit policy to be rejected"
    );
}
```

## `picoquictest/warptest.c:warptest_video_audio_test`
* C test-table name: `warptest_video_audio`
* C entry function: `warptest_video_audio_test`
* Rust test: `warptest_video_audio`
* Expected Rust file: `rs/fq/src/tests/warptest.rs`
* Rust span: `rs/fq/src/tests/warptest.rs:42-51`
* Phase 5A analysis: The Rust test uses matching spec values, but the Rust warptest_one helper advances synthetic counters rather than running the WARP stream/callback simulation the C test checks.
* Phase 5A fix note: Replace the placeholder warptest_one behavior with a faithful WARP simulation: configure BBR/0.01/audio/video, generate media over streams, run steps to completion, and check audio/video stats.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test and warptest_one faithfully configure BBR, bandwidth 0.01, video and audio, and use the real media simulation path, but shared util.rs contains duplicate TestTlsApiCtx fields/initializers and a duplicate set_send_buffer_size merge artifact.
* Phase 5C fix note: Deduplicate TestTlsApiCtx fields/initializers and leave one correctly scoped set_send_buffer_size in rs/fq/src/tests/util.rs; keep the current warptest logic.

### C test body
```c
{
    int ret;
    warptest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    ret = warptest_one(2, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn warptest_video_audio() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        ..Default::default()
    };
    warptest_one(2, &spec).expect("warptest_video_audio");
}
```

## `picoquictest/warptest.c:warptest_video_data_audio_test`
* C test-table name: `warptest_video_data_audio`
* C entry function: `warptest_video_data_audio_test`
* Rust test: `warptest_video_data_audio`
* Expected Rust file: `rs/fq/src/tests/warptest.rs`
* Rust span: `rs/fq/src/tests/warptest.rs:57-67`
* Phase 5A analysis: The Rust test body mirrors the C spec and calls warptest_one(3), but the Rust warptest_one helper is synthetic: it increments counters and fabricates media delays instead of running the C-style WARP QUIC simulation with bulk data, media streams, callbacks, packet loop, and received-frame stats. This weakens the test and does not check the intended audio/video behavior under 10 MB data contention.
* Phase 5A fix note: Phase 5B should replace the placeholder-like Rust warptest_one path with a faithful WARP simulation: create/send the bulk data stream and audio/video unidirectional frames, run the simulated packet loop until completion, and verify audio/video frame counts and delay stats for the BBR + bandwidth 0.01 + data_size 10000000 scenario.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust wrapper has the right id and spec, but warptest_one's data_size path fabricates bulk-data progress and media delay stats instead of running the 10 MB data-contention WARP simulation.
* Phase 5C fix note: Run the data_size path through real bulk stream/media frame queuing, packet simulation, received-frame stats, and delay checks.

### C test body
```c
{
    int ret;
    warptest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    spec.data_size = 10000000;
    ret = warptest_one(3, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn warptest_video_data_audio() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        ..Default::default()
    };
    warptest_one(3, &spec).expect("warptest_video_data_audio");
}
```
