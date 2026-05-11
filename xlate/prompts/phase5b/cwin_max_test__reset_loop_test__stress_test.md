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

Owned Rust test file(s): `rs/fq/src/tests/congestion.rs`, `rs/fq/src/tests/edge_cases.rs`, `rs/fq/src/tests/stresstest.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/congestion_test.c:cwin_max_test`
* C test-table name: `cwin_max`
* C entry function: `cwin_max_test`
* Rust test: `cwin_max`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Rust span: `rs/fq/src/tests/congestion.rs:975-985`
* Phase 5A analysis: The Rust test uses the same algorithm list, limits, and qlog bytes-in-flight check, but cwin_max_test_one passes zeroed TransportParameters::default() where C initializes defaults, changing initial_max_data/max-data control. The shared Rust scenario verifier also ignores max_completion_time.
* Phase 5A fix note: Initialize client_params with init_transport_parameters before passing/using it, and fix shared scenario completion/max-time verification.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust wrapper and helper mostly match the C cwin_max contract, but the current congestion registry has no fastcc alias, so the FastCC case from the C algorithm table is not exercised correctly.
* Phase 5C fix note: Restore a fastcc registry alias or make the test resolve the existing fast/FASTCC Rust algorithm for the C picoquic_fastcc_algorithm case.

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
fn app_limit_cc() {
    let algo_names = ["newreno", "cubic", "dcubic", "bbr", "fastcc", "bbr1"];
    let max_completion_times: [u64; 6] = [
        22_000_000, 23_500_000, 22_000_000, 21_000_000, 25_000_000, 25_000_000,
    ];

    for (name, &max_time) in algo_names.iter().zip(max_completion_times.iter()) {
        let ccalgo = cc_algo(name);
        app_limit_cc_test_one(ccalgo, max_time);
    }
}
```

## `picoquictest/edge_cases.c:reset_loop_test`
* C test-table name: `reset_loop_test`
* C entry function: `reset_loop_test`
* Rust test: `reset_loop_test`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Rust span: `rs/fq/src/tests/edge_cases.rs:1908-2052`
* Phase 5A analysis: The Rust test omits the C reset-loop callback behavior and explicitly installs no callbacks, while active stream marking depends on callbacks and the C test relies on callback-driven streaming and reset handling.
* Phase 5A fix note: Implement and install a Rust equivalent of reset_loop_callback, including prepare-to-send/data/reset/stop-sending behavior and stream contexts, then keep the mid-transfer reset and post-reset rejection checks.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust now installs a reset-loop callback and covers the stream/reset sequence, but its prepare-to-send path only observes the event and does not provide stream data buffers like the C callback, so callback-driven streaming is not faithfully expressed.
* Phase 5C fix note: Override the Rust prepare_to_send callback to call provide_stream_data_buffer, fill the buffer, set fin/still-active behavior, and update counters before the existing reset checks.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t timeout;
    uint64_t test_stream = 8;
    reset_loop_callback_t cb = { 0 };
    picoquic_stream_head_t* stream = NULL;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        uint8_t bogus_data[4] = { 0 };
        picoquic_set_default_callback(test_ctx->qserver, reset_loop_callback, &cb);
        picoquic_set_callback(test_ctx->cnx_client, reset_loop_callback, &cb);
        picoquic_start_client_cnx(test_ctx->cnx_client);
        picoquic_add_to_stream(test_ctx->cnx_client, 4, bogus_data, 4, 0);
        picoquic_add_to_stream(test_ctx->cnx_client, 8, bogus_data, 4, 0);
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        picoquic_mark_active_stream(test_ctx->cnx_client, 4, 1, &cb);
        picoquic_mark_active_stream(test_ctx->cnx_client, 8, 1, &cb);
        /* set priorities */
        if (ret == 0) {
            ret = picoquic_set_stream_priority(test_ctx->cnx_client, 4, 8);
        }
        if (ret == 0) {
            ret = picoquic_set_stream_priority(test_ctx->cnx_client, 8, 8);
        }
    }

    /* Perform a few rounds of sending loop, but not enough to send all the data */
    if (ret == 0) {
        timeout = simulated_time + 100000;
        ret = tls_api_wait_for_timeout(test_ctx, &simulated_time, timeout);
    }

    /* trigger a reset of tst stream */
    if (ret == 0) {
        ret = picoquic_reset_stream(test_ctx->cnx_server, test_stream, 0);
    }

    /* set priorities */
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_client, 4, 9);
    }
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_client, 8, 7);
    }
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_server, 4, 9);
    }
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_server, 8, 7);
    }

    /* make sure that reset is sent */
    if (ret == 0) {
        timeout = simulated_time + 100000;
        for (int i = 0; ret == 0 && i < 16; i++) {
            int was_active = 0;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, timeout, &was_active);
            if (ret == 0) {
                stream = picoquic_find_stream(test_ctx->cnx_server, test_stream);
                if (stream == NULL) {
                    ret = -1;
                    break;
                }
                else if (stream->reset_sent) {
                    break;
                }
            }
        }
    }
    if (ret == 0 && (stream == NULL || !stream->reset_sent)) {
        DBG_PRINTF("Could not reset stream %" PRIu64, test_stream);
        ret = -1;
    }

    if (ret == 0) {
        /* add data to the stream to elicit some bad behavior */
        uint8_t bogus_data[4] = { 1, 2, 3, 4 }; 
        if (picoquic_add_to_stream(test_ctx->cnx_server, test_stream, bogus_data, 4, 1) == 0) {
            DBG_PRINTF("Adding on stream %" PRIu64 " after reset should be forbidden", test_stream);
            ret = -1;
        }
    }

    if (ret == 0) {
        /* add data to the stream to elicit some bad behavior */
        uint8_t bogus_context[4] = { 0, 0, 0, 0 };
        if (picoquic_mark_active_stream(test_ctx->cnx_server, test_stream, 1, bogus_context) == 0) {
            DBG_PRINTF("Marking stream %" PRIu64 " active after reset should be forbidden", test_stream);
            ret = -1;
        }
    }

    /* Do a loop to check the behavior */
    if (ret == 0) {
        timeout = simulated_time + 2000000;
        ret = tls_api_wait_for_timeout(test_ctx, &simulated_time, timeout);
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
fn reset_loop_test() {
    let mut simulated_time = Instant::from_ticks(0);
    let test_stream: u64 = 8;
    let cb_state = Rc::new(RefCell::new(ResetLoopState::default()));

    let mut test_ctx = tls_api_init_ctx(
        &mut simulated_time,
        crate::internal::Version::InternalTest1 as u32,
        None,
    )
    .expect("tls_api_init_ctx");

    test_ctx
        .qserver
        .set_default_callback(Some(Box::new(ResetLoopCallback {
            state: Rc::clone(&cb_state),
        })));
    test_ctx
        .cnx_client()
        .set_callback(Some(Box::new(ResetLoopCallback {
            state: Rc::clone(&cb_state),
        })));

    test_ctx.cnx_client().start_client().expect("start_client");

    // Queue initial data on streams 4 and 8; triggers server-side stream creation.
    let bogus = [0u8; 4];
    test_ctx
        .cnx_client()
        .add_to_stream(4, &bogus, false)
        .expect("add_to_stream 4");
    test_ctx
        .cnx_client()
        .add_to_stream(8, &bogus, false)
        .expect("add_to_stream 8");

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    // Enable streaming on both client streams with equal priority.
    test_ctx
        .cnx_client()
        .mark_active_stream(4, true, Some(Box::new(Rc::clone(&cb_state))))
        .expect("mark active 4");
    test_ctx
        .cnx_client()
        .mark_active_stream(8, true, Some(Box::new(Rc::clone(&cb_state))))
        .expect("mark active 8");
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 8)
        .expect("set client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 8)
        .expect("set client priority 8");

    // Allow stream data to begin flowing before the reset.
    let timeout = simulated_time.ticks() + 100_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout).expect("100ms wait");
    {
        let state = cb_state.borrow();
        assert!(
            state.prepare_to_send[1] > 0 && state.data_sent[1] > 0,
            "client stream 4 should have used callback-driven prepare-to-send before reset"
        );
        assert!(
            state.prepare_to_send[3] > 0 && state.data_sent[3] > 0,
            "client stream 8 should have used callback-driven prepare-to-send before reset"
        );
        assert!(
            state.data_sent[3] < RESET_LOOP_TARGET_BYTES,
            "stream {test_stream} should still be mid-transfer before reset"
        );
    }

    // Server resets stream 8 while the transfer is in progress.
    test_ctx
        .cnx_server()
        .reset_stream(test_stream, 0)
        .expect("reset_stream");

    // Adjust priorities to expose the bug (different priorities post-reset).
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 9)
        .expect("client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 7)
        .expect("client priority 8");
    test_ctx
        .cnx_server()
        .set_stream_priority(4, 9)
        .expect("server priority 4");
    test_ctx
        .cnx_server()
        .set_stream_priority(8, 7)
        .expect("server priority 8");

    // Poll until the server's RESET_STREAM frame has actually been sent.
    let deadline = Instant::from_ticks(simulated_time.ticks() + 100_000);
    for _ in 0..16 {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            deadline,
            &mut was_active,
        )
        .expect("sim round");
        if check_stream_reset_sent(&mut test_ctx, test_stream) {
            break;
        }
    }
    assert!(
        check_stream_reset_sent(&mut test_ctx, test_stream),
        "server did not send RESET_STREAM for stream {test_stream}"
    );

    // After reset, adding data or marking the stream active must be rejected.
    assert!(
        test_ctx
            .cnx_server()
            .add_to_stream(test_stream, &[1, 2, 3, 4], true)
            .is_err(),
        "add_to_stream after reset should be forbidden on stream {test_stream}"
    );
    assert!(
        test_ctx
            .cnx_server()
            .mark_active_stream(test_stream, true, None)
            .is_err(),
        "mark_active_stream after reset should be forbidden on stream {test_stream}"
    );

    // Final loop: verify the connection settles within 2 seconds.
    let timeout2 = simulated_time.ticks() + 2_000_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout2).expect("2s wait");
    assert!(
        cb_state.borrow().reset_received[3] > 0,
        "client callback should observe RESET_STREAM for stream {test_stream}"
    );
}
```

## `picoquictest/stresstest.c:stress_test`
* C test-table name: `stress`
* C entry function: `stress_test`
* Rust test: `stress`
* Expected Rust file: `rs/fq/src/tests/stresstest.rs`
* Rust span: `rs/fq/src/tests/stresstest.rs:1010-1014`
* Phase 5A analysis: Rust passes the same nominal duration and wall-time limit, but its stress_or_fuzz_test is a lightweight counter/random simulation, not the C QUIC stress harness with server/client contexts, sim links, packet polling, callbacks, and cleanup.
* Phase 5A fix note: Port or call a faithful Rust stress harness that creates the QUIC server and clients, drives stress_loop_poll_context-style simulation for the configured duration, enforces wall-time timeout, and verifies completion as C does.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust stress still uses a lightweight random/time client model, not the C QUIC stress harness with server/client contexts, sim links, packet polling, callbacks, and cleanup.
* Phase 5C fix note: Port or call a faithful Rust stress harness that creates the QUIC server and clients, drives stress_loop_poll_context-style simulation for the configured duration, enforces wall-time timeout, and verifies completion.

### C test body
```c
{
    return stress_or_fuzz_test(NULL, NULL, picoquic_stress_test_duration, 10*picoquic_stress_test_duration);
}
```

### Current Rust test body
```rust
fn stress() {
    let duration: u64 = 60_000_000; // 1 minute
    let wall_time_max: u64 = 10 * duration;
    stress_or_fuzz_test(duration, wall_time_max, None).expect("stress_test");
}
```
