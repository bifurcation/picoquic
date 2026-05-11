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

## `picoquictest/congestion_test.c:bdp_delay_test`
* C test-table name: `bdp_delay`
* C entry function: `bdp_delay_test`
* Rust test: `bdp_delay`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:900-902`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The BDP delay option setup and BDP-specific checks match, but the Rust path still drops the C helper's scenario completion/max-time assertion, including the 8,000,000us second-pass bound.
* Phase 5A fix note: Restore max completion and completion verification in the shared scenario verifier so `bdp_option_test_one(BdpTestOption::Delay)` enforces the C timing result.
* Phase 5B analysis: Current Rust already matches C: bdp_delay uses BdpTestOption::Delay, the second pass sets the 8,000,000us bound, and tls_api_one_scenario_body_verify enforces completion and max time.
* Phase 5B fix note: 

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_delay);
}
```

### Current Rust test body
```rust
fn bdp_delay() {
    bdp_option_test_one(BdpTestOption::Delay);
}
```

## `picoquictest/datagram_tests.c:datagram_test`
* C test-table name: `datagram`
* C entry function: `datagram_test`
* Rust test: `datagram`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Current Rust span: `rs/fq/src/tests/datagram.rs:642-649`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper fields match, but the Rust helper forces negotiated datagram transport parameters to expected values and manually delivers/acks datagrams outside the QUIC DATAGRAM provider/callback path, so it does not check the same behavior as C.
* Phase 5A fix note: Wire the Rust datagram helper through actual QUIC datagram send/receive/ack callbacks, assert negotiated max_datagram_frame_size values instead of overwriting them, and remove direct synthetic delivery.
* Phase 5B analysis: Rust #[test] datagram matches the C entry's API-level setup: default context, MAX_PACKET_SIZE, targets [5,5], and datagram_test_one(1, ..., 0). It compiles as a runnable Rust test. Phase 5C note: incomplete real DATAGRAM provider/receive/ack behavior may still cause runtime failure, but that is not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 5;
    dg_ctx.dg_target[1] = 5;

    return datagram_test_one(1, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [5, 5],
        ..Default::default()
    };
    datagram_test_one(1, &mut dg_ctx, 0);
}
```

## `picoquictest/datagram_tests.c:datagram_wifi_test`
* C test-table name: `datagram_wifi`
* C entry function: `datagram_wifi_test`
* Rust test: `datagram_wifi`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Current Rust span: `rs/fq/src/tests/datagram.rs:789-802`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust datagram_wifi initializer preserves C's WiFi parameters, but the shared helper masks datagram negotiation failures by assigning expected frame sizes instead of checking them.
* Phase 5A fix note: Use the same datagram negotiation assertion fix in Rust datagram_test_one_result; keep the WiFi timing, trial-limit, latency, and link-latency inputs unchanged.
* Phase 5B analysis: Rust #[test] is present, compiles, and calls datagram_test_one(8, ...) with the same API-visible ctx values as C. Any failure from missing negotiated remote max_datagram_frame_size population is Phase 5C runtime implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 1000;
    dg_ctx.dg_target[1] = 1000;
    dg_ctx.send_delay = 2000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;
    dg_ctx.dg_latency_target[0] = 305000;
    dg_ctx.dg_latency_target[1] = 280000;
    dg_ctx.test_wifi = 1;
    dg_ctx.nb_trials_max = 64000;
    dg_ctx.link_latency = 25000;

    return datagram_test_one(8, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_wifi() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [1_000, 1_000],
        send_delay: 2_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [305_000, 280_000],
        test_wifi: true,
        nb_trials_max: 64_000,
        link_latency: 25_000,
        ..Default::default()
    };
    datagram_test_one(8, &mut dg_ctx, 0);
}
```

## `picoquictest/edge_cases.c:reset_loop_test`
* C test-table name: `reset_loop_test`
* C entry function: `reset_loop_test`
* Rust test: `reset_loop_test`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1908-2052`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test omits the C reset-loop callback behavior and explicitly installs no callbacks, while active stream marking depends on callbacks and the C test relies on callback-driven streaming and reset handling.
* Phase 5A fix note: Implement and install a Rust equivalent of reset_loop_callback, including prepare-to-send/data/reset/stop-sending behavior and stream contexts, then keep the mid-transfer reset and post-reset rejection checks.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, compiles under the Rust test harness, and matches the C test's API-level setup, stream operations, reset, priority changes, reset-sent check, post-reset rejection checks, and final wait. Any early failure from missing PrepareToSend delivery, callback counters, reset side effects, or other library behavior is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

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

## `picoquictest/l4s_test.c:l4s_bbr_updown_test`
* C test-table name: `l4s_bbr_updown`
* C entry function: `l4s_bbr_updown_test`
* Rust test: `l4s_bbr_updown`
* Expected Rust file: `rs/fq/src/tests/l4s.rs`
* Current Rust span: `rs/fq/src/tests/l4s.rs:193-197`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Entry inputs and L4S_LINK_UPDOWN constants match, but the Rust tls_api_one_scenario_body_ex ignores link_states, so the up/down bandwidth schedule that defines this C test is not exercised; it also relies on weakened shared completion verification.
* Phase 5A fix note: Make the Rust scenario body_ex/data loop apply VaryLinkSpec transitions like C tls_api_data_sending_loop_ex and enforce scenario/completion checks.
* Phase 5B analysis: Rust #[test] is present, compiles, and calls the L4S helper with the C-visible BBR algorithm, do_l4s=true, max_completion_time=5_800_000, max_losses=69, max_rttvar=3_000, and the up/down link schedule. The prior client-disconnect/no-server-connection failure is a Phase 5C runtime-library issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
#if defined(_WINDOWS) && !defined(_WINDOWS64)
    return 0;
#else
    picoquic_congestion_algorithm_t* ccalgo = picoquic_bbr_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 5800000, 69, 3000, nb_l4s_link_updown, l4s_link_updown);

    return ret;
#endif
}
```

### Current Rust test body
```rust
fn l4s_bbr_updown() {
    crate::register_all_congestion_control_algorithms();
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    l4s_congestion_test(ccalgo, true, 5_800_000, 69, 3_000, L4S_LINK_UPDOWN);
}
```
