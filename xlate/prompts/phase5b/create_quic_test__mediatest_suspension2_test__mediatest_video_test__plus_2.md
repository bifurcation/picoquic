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

Owned Rust test file(s): `rs/fq/src/tests/cnx_creation.rs`, `rs/fq/src/tests/mediatest.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/cnx_creation_test.c:create_quic_test`
* C test-table name: `create_quic`
* C entry function: `create_quic_test`
* Rust test: `create_quic`
* Expected Rust file: `rs/fq/src/tests/cnx_creation.rs`
* Rust span: `rs/fq/src/tests/cnx_creation.rs:207-328`
* Phase 5A analysis: Most checks match, but the NULL transport-parameter reset is not faithfully tested: C calls picoquic_set_default_tp(quic, NULL), which reloads initialized defaults, while Rust passes TransportParameters::default(), the zeroed struct, and only checks success.
* Phase 5A fix note: Phase 5B should test the Rust equivalent of NULL/default reset, using initialized transport defaults via init_transport_parameters or an Option-style API, and assert the resulting default_tp matches initialized defaults.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The merged create_quic test lost the Phase 5B NULL-TP reset repair: it still calls set_default_tp(&TransportParameters::default()) and only checks success, while C calls picoquic_set_default_tp(quic, NULL) to restore initialized defaults. The Rust API already supports set_default_tp(None::<&TransportParameters>).
* Phase 5C fix note: Use set_default_tp(None::<&TransportParameters>) after perturbing default_tp, and assert the resulting default_tp matches init_transport_parameters defaults.

### C test body
```c
{
    int ret = 0;
    char const* bad_dir = "..";
    char const* bad_file = "no_such_file_should_exist.pem";
    picoquic_quic_t* quic = NULL;

    /* Check that 0 connection == 1 */
    if (ret == 0) {
        quic = picoquic_create(0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0);
        if (quic == NULL || quic->max_number_connections != 1) {
            ret = -1;
        }
        picoquic_free(quic);
        quic = NULL;
    }

    /* Check that bad context, bad key or bad store crashes connection */
    if (ret == 0) {
        char test_server_cert_file[512];
        char test_server_key_file[512];

        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir,
            PICOQUIC_TEST_FILE_SERVER_CERT);

        if (ret == 0) {
            ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir,
                PICOQUIC_TEST_FILE_SERVER_KEY);
        }

        if (ret == 0) {
            if ((quic = picoquic_create(8, bad_file, test_server_key_file, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0)) != NULL ||
                (quic = picoquic_create(8, test_server_cert_file, bad_file, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0)) != NULL) {
                ret = -1;
                picoquic_free(quic);
                quic = NULL;
            }
        }
    }

    /* Check that bad ticket store does not crash a client connection */
    if (ret == 0) {
        if ((quic = picoquic_create(0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, bad_file, NULL, 0)) == NULL) {
            ret = -1;
        }
        else {
            picoquic_free(quic);
            if ((quic = picoquic_create(0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, bad_dir, NULL, 0)) == NULL) {
                ret = -1;
            }
            else {
                picoquic_free(quic);
                quic = NULL;
            }
        }
    }

    /* Check loading of token file (always work) and not a valid file name (always fail).
    * However, this test is not very portable, because reading a bad directory only
    * fails on Windows.
     */
    if (ret == 0) {
        if ((quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0)) == NULL) {
            ret = -1;
        }
        else
        {
            int rbf = 0;
            int rbd = 0;
            if ((rbf = picoquic_load_token_file(quic, bad_file)) != 0 &&
                (rbd = picoquic_load_token_file(quic, bad_dir)) == 0) {
                ret = -1;
            }
            DBG_PRINTF("Load token %s %s",
                bad_file, (rbf == 0) ? "Succeeds" : "Fails");
            DBG_PRINTF("Load token %s %s",
                bad_dir, (rbd == 0) ? "Succeeds" : "Fails");
            picoquic_free(quic);
            quic = NULL;
        }
    }

    /* Check that loading a NULL TP loads the default */
    if (ret == 0) {
        if ((quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0)) == NULL) {
            ret = -1;
        }
        else
        {
            if (picoquic_set_default_tp(quic, NULL) != 0) {
                ret = -1;
            }
            picoquic_free(quic);
            quic = NULL;
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn create_quic() {
    let bad_file = "no_such_file_should_exist.pem";
    let bad_dir = "..";

    // 0 connections clamps to 1.
    // C: `quic->max_number_connections != 1` → fail.
    {
        let quic = Quic::new(
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .expect("0 max_nb_connections must still create a context (clamped to 1)");
        assert_eq!(
            quic.max_number_connections, 1,
            "0 max_nb_connections must clamp to 1"
        );
    }

    // Bad cert or key path must cause creation to fail.
    let cert_file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/cert.pem");
    let key_file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/key.pem");

    assert!(
        Quic::new(
            8,
            Some(bad_file),
            Some(key_file),
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .is_none(),
        "bad cert path must reject context creation"
    );
    assert!(
        Quic::new(
            8,
            Some(cert_file),
            Some(bad_file),
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .is_none(),
        "bad key path must reject context creation"
    );

    // Bad ticket-store path must NOT crash a client context.
    // C: the ticket file is position 13 in `picoquic_create` (→ Rust `ticket_file_name`).
    Quic::new(
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        Some(bad_file),
        None,
    )
    .expect("bad ticket-store file name should still create a context");

    Quic::new(
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        Some(bad_dir),
        None,
    )
    .expect("bad ticket-store directory should still create a context");

    // Load-token-file edge cases.
    // C: fails if bad_file fails AND bad_dir succeeds (Windows-specific).
    // On Linux/macOS, bad_dir usually fails too, so the condition is always false.
    {
        let mut quic = default_quic().expect("create quic");
        let rbf = quic.load_token_file(bad_file);
        if rbf.is_err() {
            // Only check bad_dir when bad_file failed.
            let rbd = quic.load_token_file(bad_dir);
            assert!(
                rbd.is_err(),
                "load_token_file: bad_dir succeeded where bad_file failed (platform-specific)"
            );
        }
    }

    // Resetting transport parameters to their defaults must succeed.
    // C: `picoquic_set_default_tp(quic, NULL)` — NULL resets to defaults.
    {
        let mut quic = default_quic().expect("create quic");
        quic.set_default_tp(&TransportParameters::default())
            .expect("set_default_tp with default params");
    }
}
```

## `picoquictest/mediatest.c:mediatest_suspension2_test`
* C test-table name: `mediatest_suspension2`
* C entry function: `mediatest_suspension2_test`
* Rust test: `mediatest_suspension2`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Rust span: `rs/fq/src/tests/mediatest.rs:1582-1596`
* Phase 5A analysis: The Rust entry spec fields match the C wrapper, but the shared Rust mediatest_one is materially different from the C media harness: it runs generic TLS stream descriptors and does not reproduce media frame generation, media latency stats, or meaningful video2/probe-up checks.
* Phase 5A fix note: Port or implement an equivalent mediatest harness for audio/video/video2 frame scheduling, probe-up behavior, completion, and latency-stat checks; then keep this entry's spec values.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust media harness and wrapper fields match the C suspension2 scenario, but the wrapper calls get_congestion_algorithm("bbr") without ensuring the registry is populated, so the test can run without the C-intended BBR algorithm when executed in isolation.
* Phase 5C fix note: Register congestion algorithms before looking up bbr, or use a helper that registers and unwraps/asserts BBR so the test always exercises the intended C algorithm.

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

## `picoquictest/mediatest.c:mediatest_video_test`
* C test-table name: `mediatest_video`
* C entry function: `mediatest_video_test`
* Rust test: `mediatest_video`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Rust span: `rs/fq/src/tests/mediatest.rs:1413-1421`
* Phase 5A analysis: The Rust wrapper sets the same spec fields, but Rust mediatest_one reduces the media simulation to a generic send/receive scenario and does not check the C media frame pacing or video latency statistics.
* Phase 5A fix note: Implement/use a faithful media-test driver with media stream generation, custom loop, completion detection, and mediatest_check_stats-equivalent checks for the video stream.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust wrapper has the right spec fields, but current mediatest_one does not route MediatestId::Video through the media-specific driver, so it still uses the generic send/receive scenario and skips video frame pacing/stat checks.
* Phase 5C fix note: Include MediatestId::Video in the mediatest_media_one path so the video-only test checks media frames and latency stats like C.

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

## `picoquictest/mediatest.c:mediatest_video2_back_test`
* C test-table name: `mediatest_video2_back`
* C entry function: `mediatest_video2_back_test`
* Rust test: `mediatest_video2_back`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Rust span: `rs/fq/src/tests/mediatest.rs:1471-1484`
* Phase 5A analysis: The Rust entry sets the same spec values, but its mediatest_one helper is a generic stream transfer with a permanent bandwidth change and completion-bound check, not the C media-frame simulator with staged down/back timing and audio/video latency statistics.
* Phase 5A fix note: Implement the C-style media harness for this path: run to 2,000,000 us, reduce bandwidth to 8,000,000 ps/byte until 4,000,000 us, restore link settings, finish by 30,000,000 us, require completion, check audio and video stats against average/max latency, and skip only video2 stats.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust entry and media harness match the C video2_back timing and stats intent, but the shared util test module is not currently compilable in the merged tree, so this test is not exposed as runnable.
* Phase 5C fix note: Repair rs/fq/src/tests/util.rs merge damage: duplicate send_buffer_size/use_udp_gso fields and misplaced helper methods. No mediatest-specific mismatch found.

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

## `picoquictest/mediatest.c:mediatest_video_audio_test`
* C test-table name: `mediatest_video_audio`
* C entry function: `mediatest_video_audio_test`
* Rust test: `mediatest_video_audio`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Rust span: `rs/fq/src/tests/mediatest.rs:1425-1434`
* Phase 5A analysis: Wrapper spec fields match, but Rust mediatest_one checks a generic bulk stream transfer, not the C media harness that generates timed audio/video frames and verifies frame counts plus latency average/sigma/max stats.
* Phase 5A fix note: Phase 5B should make mediatest_one faithful for VideoAudio: generate the C-style 10s audio/video frame streams and assert mediatest_check_stats-equivalent frame counts and latency bounds, instead of only generic scenario completion.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The C/Rust media-test correspondence is faithful, including audio/video sender setup and latency/frame-count stats, but the merged shared Rust test harness is currently malformed and not runnable.
* Phase 5C fix note: Repair rs/fq/src/tests/util.rs: deduplicate TestTlsApiCtx send_buffer_size/use_udp_gso fields and restore misplaced helper methods to valid scope.

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
