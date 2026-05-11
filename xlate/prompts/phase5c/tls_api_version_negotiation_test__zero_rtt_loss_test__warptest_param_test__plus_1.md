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

## `picoquictest/tls_api_test.c:tls_api_version_negotiation_test`
* C test-table name: `version_negotiation`
* C entry function: `tls_api_version_negotiation_test`
* Rust test: `version_negotiation`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8602-8647`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C proposes GREASE version 0x0aca4a0a, expects disconnect, and checks a version-negotiation notification; Rust uses supported V1 and only runs a normal loss-test path.
* Phase 5A fix note: Initialize with GREASE version 0x0aca4a0a, run the connection loop, assert the client disconnects, and record/assert the version-negotiation callback in the Rust test context if needed.
* Phase 5B analysis: Rust test is present, compiles as a harness test, uses GREASE version 0x0aca4a0a, runs the connection loop, and asserts the same API-visible contract as C: client disconnect plus version-negotiation callback notification. Missing runtime VN callback emission is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    const uint32_t version_grease = 0x0aca4a0a;
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, version_grease, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", version_grease);
    }

    if (ret == 0) {
        (void)tls_api_connection_loop(test_ctx, NULL, 0, &simulated_time);

        if (test_ctx->cnx_client->cnx_state == picoquic_state_disconnected) {
            ret = 0;
        }
        else {
            DBG_PRINTF("Unexpected state: %d\n", test_ctx->cnx_client->cnx_state);
            ret = -1;
        }
    }


    if (ret == 0) {
        if (!test_ctx->received_version_negotiation){
            DBG_PRINTF("%s", "No version negotiation notified\n");
            ret = -1;
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
fn version_negotiation() {
    const VERSION_GREASE: u32 = 0x0aca4a0a;

    struct VersionNegotiationTracker {
        received: Rc<Cell<bool>>,
    }

    impl StreamDataCallback for VersionNegotiationTracker {
        fn callback(
            &mut self,
            _connection: &mut Connection,
            _stream_id: u64,
            _bytes: &[u8],
            fin_or_event: CallbackEvent,
            _stream_ctx: Option<&mut dyn core::any::Any>,
        ) -> i32 {
            if fin_or_event == CallbackEvent::VersionNegotiation {
                self.received.set(true);
            }
            0
        }
    }

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, VERSION_GREASE, None).expect("ctx");
    let received_version_negotiation = Rc::new(Cell::new(false));
    test_ctx
        .cnx_client()
        .set_callback(Some(Box::new(VersionNegotiationTracker {
            received: Rc::clone(&received_version_negotiation),
        })));

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    assert_eq!(
        test_ctx.cnx_client().connection_state,
        State::Disconnected,
        "client did not disconnect after GREASE version negotiation"
    );
    assert!(
        received_version_negotiation.get(),
        "version negotiation was not notified"
    );
}
```

## `picoquictest/tls_api_test.c:zero_rtt_loss_test`
* C test-table name: `zero_rtt_loss`
* C entry function: `zero_rtt_loss_test`
* Rust test: `zero_rtt_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:9055-9063`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test loop matches the C loss-mask cases, but the Rust zero_rtt_test_one helper appears to synthesize 0-RTT counters, PSK state, and tickets, weakening the behavior checked by the C test.
* Phase 5A fix note: Make zero_rtt_test_one rely on real ticket issuance, PSK handshake state, 0-RTT send/ack counters, and data acceptance instead of synthetic assignments; keep the loss loop over packets 1..15.
* Phase 5B analysis: Rust #[test] zero_rtt_loss matches the C API-level contract: it iterates packet indices 1 through 15, sets early_loss = 1 << i, and requires zero_rtt_test_one to succeed. Any runtime failure from missing ticket/PSK/0-RTT behavior is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    for (unsigned int i = 1; ret == 0 && i < 16; i++) {
        zero_rtt_test_t zrt = { 0 };
        zrt.early_loss = 1ull << i;

        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Zero RTT test fails when packet #%d is lost.\n", i);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn zero_rtt_loss() {
    for i in 1u32..16 {
        zero_rtt_test_one(&ZeroRttTest {
            early_loss: 1u64 << i,
            ..Default::default()
        })
        .unwrap_or_else(|e| panic!("zero_rtt_loss i={i}: {e:?}"));
    }
}
```

## `picoquictest/warptest.c:warptest_param_test`
* C test-table name: `warptest_param`
* C entry function: `warptest_param_test`
* Rust test: `warptest_param`
* Expected Rust file: `rs/fq/src/tests/warptest.rs`
* Current Rust span: `rs/fq/src/tests/warptest.rs:11-22`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper fields match the C spec, but C warptest_one runs the WARP media simulation, while the Rust helper mostly advances counters/stats without exercising the actual stream media behavior or max-stream limits, making the test placeholder-like.
* Phase 5A fix note: Make Rust warptest_one exercise the translated WARP audio/video path over QUIC with the configured BBR, bandwidth, and max stream settings, then assert the resulting stats as C does.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, and matches the C API-level contract: BBR, bandwidth 0.01, audio/video enabled, max_streams_client/server 4, and warptest_one(5). Runtime handshake/server behavior remains Phase 5C.
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
    spec.max_streams_client = 4;
    spec.max_streams_server = 4;

    ret = warptest_one(5, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn warptest_param() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        max_streams_client: 4,
        max_streams_server: 4,
        ..Default::default()
    };
    warptest_one(5, &spec).expect("warptest_param");
}
```

## `picoquictest/wifitest.c:wifi_bbr1_long_test`
* C test-table name: `wifi_bbr1_long`
* C entry function: `wifi_bbr1_long_test`
* Rust test: `wifi_bbr1_long`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:135-146`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses the same BBR1, latency, suspension, receive-block, target-time, queue-delay, and test-id inputs as C, but the shared Rust verifier ignores stream completion and the 3_400_000us target bound, weakening the C test's core assertions.
* Phase 5A fix note: Keep the matching WifiTestSpec inputs, but make the Rust wifi/TLS scenario verification enforce q/r stream completion and the target completion time.
* Phase 5B analysis: Rust test is present, compiles as a harness test, and matches the C API-level contract: BBR1, basic suspension, 50_000us latency, 3_400_000us target, receive-block enabled, queue delay 0, and wifi_test_bbr1_long CID/test id. Any Generic runtime failure is Phase 5C implementation behavior.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_basic,
        50000,
        suspension_basic,
        picoquic_bbr1_algorithm,
        NULL,
        3400000,
        1,
        0 };
    int ret = wifi_test_one(wifi_test_bbr1_long, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_bbr1_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr1",
        cc_algo_option: None,
        target_time: 3_400_000,
        simulate_receive_block: true,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR1_LONG, &spec).expect("wifi_bbr1_long");
}
```
