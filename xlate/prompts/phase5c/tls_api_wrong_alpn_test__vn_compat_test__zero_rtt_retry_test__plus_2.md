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

## `picoquictest/tls_api_test.c:tls_api_wrong_alpn_test`
* C test-table name: `tls_api_wrong_alpn`
* C entry function: `tls_api_wrong_alpn_test`
* Rust test: `tls_api_wrong_alpn`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8272-8304`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls the generic helper and expects success; that helper ignores the supplied SNI/ALPN for context creation, so it does not create the client/server ALPN mismatch or check the wrong-ALPN alert/disconnected state.
* Phase 5A fix note: Create the context with client ALPN wrong and server default ALPN TEST_ALPN, run the connection loop, then assert the client is disconnected with remote_error == TransportError::TlsAlertWrongAlpn.
* Phase 5B analysis: Rust test already matches the C API-level contract: wrong client ALPN setup, server ALPN reset, connection loop, disconnected assertion, and wrong-ALPN remote error assertion. The current remote_error mismatch is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_WRONG_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        /* By default, client and servers are using the same ALPN. Correct that on the server side 
         * so we can test the wrong ALPN condition */
        free((void*)test_ctx->qserver->default_alpn);
        test_ctx->qserver->default_alpn = picoquic_string_duplicate(PICOQUIC_TEST_ALPN);
    }

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", 0);
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, 0, 0, &simulated_time);

        if (ret == 0)
        {
            if (test_ctx->cnx_client != NULL) {
                if (test_ctx->cnx_client->cnx_state == picoquic_state_disconnected &&
                    test_ctx->cnx_client->remote_error == PICOQUIC_TLS_ALERT_WRONG_ALPN) {
                    ret = 0;
                }
                else {
                    DBG_PRINTF("Connection loop returns 0x%" PRIx64, test_ctx->cnx_client->remote_error);
                    ret = -1;
                }
            }
            else {
                DBG_PRINTF("%s", "Could not establish a client connection");
                ret = -1;
            }
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn tls_api_wrong_alpn() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        0,
        Some(TEST_SNI),
        Some("wrong-alpn"),
        None,
        None,
    )
    .expect("wrong_alpn ctx");

    test_ctx.qserver.default_alpn = Some(TEST_ALPN.to_owned());

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("wrong_alpn connection loop");

    let client = test_ctx
        .qclient
        .first_cnx_mut()
        .expect("client connection not initialized");
    assert_eq!(
        client.connection_state,
        State::Disconnected,
        "wrong_alpn: client did not disconnect"
    );
    assert_eq!(
        client.remote_error,
        crate::TransportError::TlsAlertWrongAlpn as u64,
        "wrong_alpn: client remote error"
    );
}
```

## `picoquictest/tls_api_test.c:vn_compat_test`
* C test-table name: `vn_compat`
* C entry function: `vn_compat_test`
* Rust test: `vn_compat`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8930-8938`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic InternalTest1 handshake. C runs three compatibility cases: V1 to V2 must negotiate V2, V1 to V2 draft must negotiate V2 draft, and V1 to InternalTest1 must fail.
* Phase 5A fix note: Implement vn_compat_test_one-style Rust coverage: create a V1 connection, set desired_version to V2 and V2Draft and assert both endpoints negotiate the target, then assert desired InternalTest1 is rejected.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract for V1->V2, V1->V2Draft, and rejected V1->InternalTest1 using endpoint version_index checks through SUPPORTED_VERSIONS. The known compatible-version runtime failure is Phase 5C implementation work, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    if (vn_compat_test_one(PICOQUIC_V1_VERSION, PICOQUIC_V2_VERSION) != 0) {
        ret = -1;
    }
    else if (vn_compat_test_one(PICOQUIC_V1_VERSION, PICOQUIC_V2_VERSION_DRAFT) != 0) {
        ret = -1;
    }
    else if (vn_compat_test_one(PICOQUIC_V1_VERSION, PICOQUIC_INTERNAL_TEST_VERSION_1) == 0) {
        ret = -1;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn vn_compat() {
    vn_compat_test_one(Version::V1 as u32, Version::V2 as u32).expect("vn_compat V1 -> V2");
    vn_compat_test_one(Version::V1 as u32, Version::V2Draft as u32)
        .expect("vn_compat V1 -> V2 draft");
    assert!(
        vn_compat_test_one(Version::V1 as u32, Version::InternalTest1 as u32).is_err(),
        "vn_compat V1 -> InternalTest1 unexpectedly succeeded"
    );
}
```

## `picoquictest/tls_api_test.c:zero_rtt_retry_test`
* C test-table name: `zero_rtt_retry`
* C entry function: `zero_rtt_retry_test`
* Rust test: `zero_rtt_retry`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:9105-9111`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The hardreset flag is mapped, but the shared Rust helper still pre-sets 0-RTT counters and skips important C retry-path checks such as real data receipt under rejection/retry behavior.
* Phase 5A fix note: After fixing zero_rtt_test_one, ensure the hardreset path actually exercises server retry/cookie mode and checks the C rejection/data-receipt conditions without synthetic counters.
* Phase 5B analysis: Rust #[test] matches the C entry by calling zero_rtt_test_one with only hardreset=true. The shared helper expresses the C API-visible retry contract, including pass-2 cookie mode, queued 0-RTT data, sent/data-delivered checks, and short-initial assertion. Runtime failures from missing real 0-RTT keys or early-data acceptance are Phase 5C, not Phase 5B.
* Phase 5B fix note: 

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.hardreset = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
fn zero_rtt_retry() {
    zero_rtt_test_one(&ZeroRttTest {
        hardreset: true,
        ..Default::default()
    })
    .expect("zero_rtt_retry");
}
```

## `picoquictest/warptest.c:warptest_video_data_audio_test`
* C test-table name: `warptest_video_data_audio`
* C entry function: `warptest_video_data_audio_test`
* Rust test: `warptest_video_data_audio`
* Expected Rust file: `rs/fq/src/tests/warptest.rs`
* Current Rust span: `rs/fq/src/tests/warptest.rs:57-67`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test body mirrors the C spec and calls warptest_one(3), but the Rust warptest_one helper is synthetic: it increments counters and fabricates media delays instead of running the C-style WARP QUIC simulation with bulk data, media streams, callbacks, packet loop, and received-frame stats. This weakens the test and does not check the intended audio/video behavior under 10 MB data contention.
* Phase 5A fix note: Phase 5B should replace the placeholder-like Rust warptest_one path with a faithful WARP simulation: create/send the bulk data stream and audio/video unidirectional frames, run the simulated packet loop until completion, and verify audio/video frame counts and delay stats for the BBR + bandwidth 0.01 + data_size 10000000 scenario.
* Phase 5B analysis: Rust #[test] is present, compile-runnable, and matches the C API-level contract: BBR, bandwidth 0.01, video/audio enabled, data_size 10000000, and warptest_one(3, &spec). Any early WARP/TLS readiness, bulk-data delivery, or media-stat failure is Phase 5C runtime behavior, not a Phase 5B block.
* Phase 5B fix note: 

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

## `picoquictest/wifitest.c:wifi_cubic_test`
* C test-table name: `wifi_cubic`
* C entry function: `wifi_cubic_test`
* Rust test: `wifi_cubic`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:212-215`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses the matching Cubic algorithm, suspension pattern, test id, and target time, but the shared Rust scenario verification path ignores the target-time/completion checks that the C wifi_test_one relies on.
* Phase 5A fix note: Restore scenario verification for wifi tests so completion and target_time are checked, then keep the Cubic spec values and RTT-max suspension assertion.
* Phase 5B analysis: Rust wifi_cubic is a compiled #[test] and matches the C API-level contract: Cubic default spec, basic suspension, target_time 2_870_000, and WIFI_TEST_CUBIC through the shared wifi_test_one checks. Any Generic runtime failure is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_cubic_algorithm, 2870000);

    int ret = wifi_test_one(wifi_test_cubic, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_cubic() {
    let spec = default_spec("cubic", SUSPENSION_BASIC, 2_870_000);
    wifi_test_one(WIFI_TEST_CUBIC, &spec).expect("wifi_cubic");
}
```
