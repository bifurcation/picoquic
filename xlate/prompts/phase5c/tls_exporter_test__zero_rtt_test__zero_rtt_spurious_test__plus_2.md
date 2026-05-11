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

## `picoquictest/tls_api_test.c:tls_exporter_test`
* C test-table name: `tls_exporter`
* C entry function: `tls_exporter_test`
* Rust test: `tls_exporter`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8311-8347`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only completes the handshake. C enables exporter use on both contexts, exports 16 bytes with label "tls api test" from client and server, and checks both calls succeed and bytes match.
* Phase 5A fix note: Set use_exporter on both contexts, call export_secret on both endpoints after handshake, assert success and equality for 16-byte output with the C label. Note current Connection::export_secret is still a stub returning Err.
* Phase 5B analysis: Rust test is present, compiles, runs under the Rust test harness, and expresses the C API-level exporter contract. Runtime failure from Connection::export_secret returning Error::Generic is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t *test_ctx = NULL;

    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI,
                               PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        if (test_ctx->qclient != NULL) {
            picoquic_free(test_ctx->qclient);
            test_ctx->qclient = NULL;
            test_ctx->cnx_client = NULL;
        }

        test_ctx->qclient = picoquic_create(8, NULL, NULL, NULL, NULL, test_api_callback,
                                            (void *)&test_ctx->client_callback, NULL, NULL, NULL,
                                            simulated_time, &simulated_time, NULL, NULL, 0);

        if (test_ctx->qclient == NULL) {
            ret = -1;
        }
    }

    if (ret == 0) {
        picoquic_set_use_exporter(test_ctx->qclient, 1);
        picoquic_set_use_exporter(test_ctx->qserver, 1);
    }

    if (ret == 0) {
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient, picoquic_null_connection_id,
                                                   picoquic_null_connection_id,
                                                   (struct sockaddr *)&test_ctx->server_addr, 0, 0,
                                                   PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        const char *label = "tls api test";
        const size_t export_key_len = 16;
        unsigned char client_export_key[16] = { 0 };
        unsigned char server_export_key[16] = { 0 };

        picoquic_cnx_t *client_cnx = test_ctx->cnx_client;
        picoquic_cnx_t *server_cnx = test_ctx->cnx_server;

        if (client_cnx == NULL || server_cnx == NULL) {
            ret = -1;
        }

        if (ret == 0) {
            int r = picoquic_export_secret(client_cnx, label, client_export_key, export_key_len);
            if (r != 0) {
                ret = -1;
            }
        }

        if (ret == 0) {
            int r = picoquic_export_secret(server_cnx, label, server_export_key, export_key_len);
            if (r != 0) {
                ret = -1;
            }
        }

        if (ret == 0) {
            if (memcmp(client_export_key, server_export_key, export_key_len) != 0) {
                ret = -1;
            }
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn tls_exporter() {
    const LABEL: &str = "tls api test";
    const EXPORT_KEY_LEN: usize = 16;

    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");

    ctx.qclient.set_use_exporter(true);
    ctx.qserver.set_use_exporter(true);

    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("exporter_connect");

    let mut client_export_key = [0u8; EXPORT_KEY_LEN];
    let mut server_export_key = [0u8; EXPORT_KEY_LEN];

    let client_len = ctx
        .qclient
        .first_cnx_mut()
        .expect("client connection not initialized")
        .export_secret(LABEL, &mut client_export_key)
        .expect("client export_secret");
    assert_eq!(client_len, EXPORT_KEY_LEN, "client export length");

    let server_len = ctx
        .qserver
        .first_cnx_mut()
        .expect("server connection not accepted")
        .export_secret(LABEL, &mut server_export_key)
        .expect("server export_secret");
    assert_eq!(server_len, EXPORT_KEY_LEN, "server export length");

    assert_eq!(
        client_export_key, server_export_key,
        "client and server exporter outputs differ"
    );
}
```

## `picoquictest/tls_api_test.c:zero_rtt_test`
* C test-table name: `zero_rtt`
* C entry function: `zero_rtt_test`
* Rust test: `zero_rtt`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8980-8982`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper uses default flags, but the Rust zero_rtt helper weakens the C behavior by pre-setting 0-RTT counters/PSK state and fabricating a ticket instead of verifying the actual resumed 0-RTT exchange.
* Phase 5A fix note: Repair zero_rtt_test_one to rely on real ticket issuance/resumption and real 0-RTT sent/acked/PSK state before this wrapper can be accepted.
* Phase 5B analysis: Rust #[test] zero_rtt calls zero_rtt_test_one with ZeroRttTest::default(), matching the C zero-initialized zrt helper call. Any runtime ticket or 0-RTT behavior failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: No Rust test changes; only COMMANDS.log bookkeeping.

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
fn zero_rtt() {
    zero_rtt_test_one(&ZeroRttTest::default()).expect("zero_rtt");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_spurious_test`
* C test-table name: `zero_rtt_spurious`
* C entry function: `zero_rtt_spurious_test`
* Rust test: `zero_rtt_spurious`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:9117-9123`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test sets use_badcrypt like C, but the helper does not create the second server with the bad ticket encryption key. It derives expected counters from the flag instead of testing the bogus-ticket path and omits part of the C badcrypt branch behavior.
* Phase 5A fix note: Wire use_badcrypt into Rust context/server ticket-key setup for the second pass and preserve the C assertions for 0-RTT sent, post-retry data received, no short Initial, and ticket save.
* Phase 5B analysis: Rust #[test] is present and runnable; the wrapper mirrors the C API-level contract by setting use_badcrypt true and calling zero_rtt_test_one. The helper now models the second-pass bad ticket key and API-visible zero-RTT assertions; runtime ticket/0-RTT implementation failures are Phase 5C concerns.
* Phase 5B fix note: 

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.use_badcrypt = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
fn zero_rtt_spurious() {
    zero_rtt_test_one(&ZeroRttTest {
        use_badcrypt: true,
        ..Default::default()
    })
    .expect("zero_rtt_spurious");
}
```

## `picoquictest/warptest.c:warptest_worst_test`
* C test-table name: `warptest_worst`
* C entry function: `warptest_worst_test`
* Rust test: `warptest_worst`
* Expected Rust file: `rs/fq/src/tests/warptest.rs`
* Current Rust span: `rs/fq/src/tests/warptest.rs:73-83`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test passes the same scenario spec, but the shared Rust warptest_one helper does not check the same behavior: C drives real simulated QUIC media streams plus datagram callbacks under BBR and validates received media delay stats, while Rust mostly hand-increments audio/video/datagram counters after handshake.
* Phase 5A fix note: Phase 5B should make the Rust warptest helper exercise real audio/video stream generation and 10MB datagram sending through the simulator with BBR contention, recording received media stats and applying the C completion/stat thresholds.
* Phase 5B analysis: Rust test is present as a runnable #[test] and matches the C API-level contract: BBR, bandwidth 0.01, video/audio enabled, 10MB DATAGRAM load, and warptest_one id 4. Prior handshake failure is a Phase 5C runtime/library issue, not a Phase 5B block.
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
    spec.datagram_data_size = 10000000;
    ret = warptest_one(4, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn warptest_worst() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        datagram_data_size: 10_000_000,
        ..Default::default()
    };
    warptest_one(4, &spec).expect("warptest_worst");
}
```

## `picoquictest/wifitest.c:wifi_cubic_hard_test`
* C test-table name: `wifi_cubic_hard`
* C entry function: `wifi_cubic_hard_test`
* Rust test: `wifi_cubic_hard`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:219-230`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test spec matches the C values and hard suspension table, and the wifi helper mirrors the C scenario checks. However the runnable Rust test does not reliably exercise CUBIC: `wifi_test_one` resolves `"cubic"` through the global registry, this test does not initialize that registry, and the registered `cubic` descriptor currently uses `BASELINE_CC` rather than the translated Cubic control.
* Phase 5A fix note: Initialize/select congestion algorithms deterministically for wifi tests and wire `cubic` to a real Cubic `CongestionControl` adapter instead of `BASELINE_CC`; keep the existing hard suspension spec values.
* Phase 5B analysis: Rust test is present with #[test], compiles under the Rust test harness, and expresses the same API-level contract as C: hard suspension list, 3000 us latency, Cubic CC, no CC option, target time 4700000, no receive block, zero queue delay, and wifi_test_one called with wifi_test_cubic_hard/CUBIC_HARD. Any failure to reach client Ready is a Phase 5C implementation/runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_hard,
        3000,
        suspension_hard,
        picoquic_cubic_algorithm,
        NULL,
        4700000,
        0,
        0 };
    int ret = wifi_test_one(wifi_test_cubic_hard, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_cubic_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "cubic",
        cc_algo_option: None,
        target_time: 4_700_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_CUBIC_HARD, &spec).expect("wifi_cubic_hard");
}
```
