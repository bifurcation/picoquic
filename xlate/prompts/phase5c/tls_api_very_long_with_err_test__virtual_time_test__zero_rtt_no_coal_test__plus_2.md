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

## `picoquictest/tls_api_test.c:tls_api_very_long_with_err_test`
* C test-table name: `tls_api_very_long_with_err`
* C entry function: `tls_api_very_long_with_err_test`
* Rust test: `tls_api_very_long_with_err`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8237-8265`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust omits the very-long scenario and max_data=128000. It passes an empty scenario and the helper signature currently cannot express the C max_data parameter, so it does not exercise the 1 MB transfer under loss.
* Phase 5A fix note: Run scenario [{stream_id:4, previous_stream_id:0, q_len:257, r_len:1000000}] with loss_mask=0x30000, max_data=128000, queue_delay=0, and completion bound 2210000; extend/use a helper that preserves max_data.
* Phase 5B analysis: Current Rust test is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract: clean handshake, max_data=128000 on client/server, TEST_SCENARIO_VERY_LONG, data loss mask 0x30000, and 2210000 us completion bound. Any early runtime failure from missing server acceptance or incomplete transfer behavior is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0x30000, 128000, 0, 0, 2210000, NULL, NULL);
}
```

### Current Rust test body
```rust
fn tls_api_very_long_with_err() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    let mut loss_mask = 0u64;

    tls_api_connection_loop(&mut ctx, &mut loss_mask, 0, &mut t)
        .expect("very_long_with_err connect");
    wait_client_connection_ready(&mut ctx, &mut t).expect("very_long_with_err ready");
    assert!(
        ctx.server_ready(),
        "very_long_with_err: server connection was not accepted"
    );
    {
        let client = ctx.cnx_client();
        client.maxdata_local = 128_000;
        client.maxdata_remote = 128_000;
    }
    {
        let server = ctx.cnx_server();
        server.maxdata_local = 128_000;
        server.maxdata_remote = 128_000;
    }
    test_api_init_send_recv_scenario(&mut ctx, TEST_SCENARIO_VERY_LONG)
        .expect("very_long_with_err scenario");
    loss_mask = 0x3_0000;
    tls_api_data_sending_loop(&mut ctx, &mut loss_mask, &mut t, 0)
        .expect("very_long_with_err data");
    tls_api_one_scenario_body_verify(&mut ctx, &mut t, 2_210_000).expect("very_long_with_err");
}
```

## `picoquictest/tls_api_test.c:virtual_time_test`
* C test-table name: `virtual_time`
* C entry function: `virtual_time_test`
* Rust test: `virtual_time`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8836-8906`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust test is only the generic TLS handshake/close smoke path. It does not create simulated vs direct QUIC contexts or assert quic_time/tls_time behavior, so it misses the C test's core intent.
* Phase 5A fix note: Replace with a dedicated virtual-time test: create a simulated-time context and assert repeated simulated_time increments are reflected by Quic::time/tls_time; create a direct wall-clock context and assert Quic::time/tls_time track current_time deltas within the C tolerance. This may also expose missing simulated-time support because Quic::new currently documents the simulated-time pointer as dropped and Quic::time returns current_time().
* Phase 5B analysis: Rust test is present, compiles, is runnable by the Rust harness, and expresses the C API-level contract for simulated and direct QUIC/TLS time checks. The known simulated-time runtime failure is a Phase 5C library implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    uint64_t test_time = 0;
    uint64_t simulated_time = 0;
    uint64_t current_time = picoquic_current_time();
    uint64_t ptls_time = 0;
    uint8_t callback_ctx[256];
    char test_server_cert_store_file[512];
    picoquic_quic_t * qsimul = NULL;
    picoquic_quic_t * qdirect = NULL;

    ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }
    else {
        qsimul = picoquic_create(8, NULL, NULL, test_server_cert_store_file,
            NULL, test_api_callback,
            (void*)callback_ctx, NULL, NULL, NULL, simulated_time,
            &simulated_time, ticket_file_name, NULL, 0);
        qdirect = picoquic_create(8, NULL, NULL, PICOQUIC_TEST_FILE_CERT_STORE,
            NULL, test_api_callback,
            (void*)callback_ctx, NULL, NULL, NULL, current_time,
            NULL, ticket_file_name, NULL, 0);

        if (qsimul == NULL || qdirect == NULL)
        {
            ret = -1;
        }
        else
        {
            /* Check that the simulated time follows the simulation */
            for (int i = 0; ret == 0 && i < 5; i++) {
                simulated_time += 12345678000;
                test_time = picoquic_get_quic_time(qsimul);
                ptls_time = picoquic_get_tls_time(qsimul);
                if (test_time != simulated_time) {
                    DBG_PRINTF("Test time: %llu != Simulated: %llu",
                        (unsigned long long)test_time,
                        (unsigned long long)simulated_time);
                    ret = -1;
                }
                else if (ptls_time < test_time || ptls_time > test_time + 1000) {
                    DBG_PRINTF("Test time: %llu does match ptls time: %llu",
                        (unsigned long long)test_time,
                        (unsigned long long)ptls_time);
                    ret = -1;
                }
            }
        }

        if (ret == 0) {
            int64_t delta, delta_low, delta_high;
            uint64_t current_previous = picoquic_current_time();
            uint64_t test_previous = picoquic_current_time();
            uint64_t ptls_previous = picoquic_get_tls_time(qdirect);

            /* Check that the non simulated time follows the current time */
            for (int i = 0; ret == 0 && i < 5; i++) {
#ifdef _WINDOWS
                Sleep(1);
#else
                usleep(1000);
#endif
                current_time = picoquic_current_time();
                test_time = picoquic_get_quic_time(qdirect);
                ptls_time = picoquic_get_tls_time(qdirect);

                delta = current_time - current_previous;
                delta_low = delta - 1000;
                delta_high = delta + 1000;
                if (test_time < test_previous + delta_low || test_time > test_previous + delta_high ) {
                    DBG_PRINTF("Test time: %" PRIu64 " does not match previous test time : %" PRIu64 " + delta : %" PRId64,
                        test_time, test_previous, delta);
                    ret = -1;
                }
                else if (ptls_time < ptls_previous + delta_low || ptls_time > ptls_previous + delta_high) {
                    DBG_PRINTF("Test time: %" PRIu64 " does not match previous test time : %" PRIu64 " + delta : %" PRId64,
                        ptls_time, ptls_previous, delta);
                    ret = -1;
                }
            }
        }
    }

    if (qsimul != NULL)
    {
        picoquic_free(qsimul);
        qsimul = NULL;
    }

    if (qdirect != NULL)
    {
        picoquic_free(qdirect);
        qdirect = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn virtual_time() {
    const SIMULATED_STEP: u64 = 12_345_678_000;
    const TLS_TIME_TOLERANCE: u64 = 1000;

    let mut simulated_time = 0u64;
    let mut simulated_config = crate::config::Config {
        nb_connections: 8,
        root_trust_file: Some(TEST_FILE_CERT_STORE.to_owned()),
        ..Default::default()
    };
    let qsimul = simulated_config
        .create_and_configure(
            None,
            Instant::from_ticks(simulated_time),
            Some(&mut simulated_time),
        )
        .expect("simulated-time quic");

    for i in 0..5 {
        simulated_time = simulated_time.saturating_add(SIMULATED_STEP);
        let test_time = qsimul.time();
        let tls_time = qsimul.tls_time();
        assert_eq!(
            test_time, simulated_time,
            "iteration {i}: QUIC time does not follow simulated time"
        );
        assert!(
            tls_time >= test_time && tls_time <= test_time.saturating_add(TLS_TIME_TOLERANCE),
            "iteration {i}: TLS time {tls_time} is not within {TLS_TIME_TOLERANCE}us of QUIC time {test_time}"
        );
    }

    let direct_start = crate::current_time();
    let qdirect = Quic::new(
        8,
        None,
        None,
        Some(TEST_FILE_CERT_STORE),
        None,
        None,
        None,
        [0u8; crate::RESET_SECRET_SIZE],
        Instant::from_ticks(direct_start),
        None,
        None,
    )
    .expect("direct quic");

    let current_previous = crate::current_time();
    let test_previous = crate::current_time();
    let tls_previous = qdirect.tls_time();

    for i in 0..5 {
        std::thread::sleep(std::time::Duration::from_micros(1000));

        let current_time = crate::current_time();
        let test_time = qdirect.time();
        let tls_time = qdirect.tls_time();
        let delta = current_time.saturating_sub(current_previous);

        assert_time_delta_matches(
            test_time,
            test_previous,
            delta,
            TLS_TIME_TOLERANCE,
            "QUIC",
            i,
        );
        assert_time_delta_matches(tls_time, tls_previous, delta, TLS_TIME_TOLERANCE, "TLS", i);
    }
}
```

## `picoquictest/tls_api_test.c:zero_rtt_no_coal_test`
* C test-table name: `zero_rtt_no_coal`
* C entry function: `zero_rtt_no_coal_test`
* Rust test: `zero_rtt_no_coal`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:9092-9098`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust sets no_coal=true, but the shared helper omits the C no_coal-specific assertion that server zero-RTT received count equals client zero-RTT sent count, and it manually seeds zero-RTT counters before the loop.
* Phase 5A fix note: Make zero_rtt_test_one observe real zero-RTT send/ack/receive counters and add the no_coal received-count assertion equivalent to the C helper.
* Phase 5B analysis: Rust test is present, compiles as a harness test, and calls zero_rtt_test_one with no_coal=true. The helper contains the C-equivalent API-visible zero-RTT sent/acked checks plus the no_coal server received-count check. Any empty-ticket or runtime behavior failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.no_coal = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
fn zero_rtt_no_coal() {
    zero_rtt_test_one(&ZeroRttTest {
        no_coal: true,
        ..Default::default()
    })
    .expect("zero_rtt_no_coal");
}
```

## `picoquictest/warptest.c:warptest_video_audio_test`
* C test-table name: `warptest_video_audio`
* C entry function: `warptest_video_audio_test`
* Rust test: `warptest_video_audio`
* Expected Rust file: `rs/fq/src/tests/warptest.rs`
* Current Rust span: `rs/fq/src/tests/warptest.rs:42-51`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test uses matching spec values, but the Rust warptest_one helper advances synthetic counters rather than running the WARP stream/callback simulation the C test checks.
* Phase 5A fix note: Replace the placeholder warptest_one behavior with a faithful WARP simulation: configure BBR/0.01/audio/video, generate media over streams, run steps to completion, and check audio/video stats.
* Phase 5B analysis: Rust #[test] is present, compiles under cargo check --tests, and matches the C API-level contract: BBR, bandwidth 0.01, video+audio enabled, and warptest_one(2, &spec). Runtime handshake/library failures are Phase 5C notes, not Phase 5B blockers.
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

## `picoquictest/wifitest.c:wifi_bbr_long_test`
* C test-table name: `wifi_bbr_long`
* C entry function: `wifi_bbr_long_test`
* Rust test: `wifi_bbr_long`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:165-176`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the same Wi-Fi spec values, but the shared Rust scenario verification only closes the connection and does not verify stream completion or target completion time like C.
* Phase 5A fix note: Restore tls_api_one_scenario_body_verify/test scenario checks for all stream byte counts/completion and max_completion_microsec, keeping the BBR long spec and rtt_max suspension check.
* Phase 5B analysis: Rust test is present as a runnable #[test] and already matches the C API-level contract: BBR long test id, basic suspension, 50000 latency, no CC option, target_time 3400000, receive-block simulation, queue_max_delay 0, and shared wifi_test_one verification. Prior stream callback/runtime failure is a Phase 5C library-behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_basic,
        50000,
        suspension_basic,
        picoquic_bbr_algorithm,
        NULL,
        3400000,
        1,
        0 };
    int ret = wifi_test_one(wifi_test_bbr_long, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_bbr_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr",
        cc_algo_option: None,
        target_time: 3_400_000,
        simulate_receive_block: true,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR_LONG, &spec).expect("wifi_bbr_long");
}
```
