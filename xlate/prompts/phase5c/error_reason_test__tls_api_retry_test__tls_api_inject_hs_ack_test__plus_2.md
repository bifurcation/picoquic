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

## `picoquictest/tls_api_test.c:error_reason_test`
* C test-table name: `error_reason`
* C entry function: `error_reason_test`
* Rust test: `error_reason`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2108-2185`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is only a generic TLS test. It does not use the explicit initial CID, enable server logs, force connection_error_ex with the reason string, assert the error return, or simulate until both endpoints disconnect.
* Phase 5A fix note: Port the C sequence: init with the specified CID, complete the connection loop, call connection_error_ex(INTERNAL_ERROR, 0, "error reason test"), verify the expected detected-error return/recorded reason, and drive rounds until client and server are disconnected or the C trial limits are hit.
* Phase 5B analysis: Rust test is runnable and now matches the C API-level contract. The known connection_error_ex return-code mismatch is Phase 5C library behavior, not a Phase 5B block.
* Phase 5B fix note: Added the missing server text-log setup using error_reason_log.txt before the qlog/long-log setup.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xe8, 0x80, 0x88, 0xea, 0x50, 0, 0, 0}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN,
        &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    if (ret == 0) {
        /* Request the logs on the server side, so manual inspection can verify that
         * the error reason is properly displayed. */
        picoquic_set_textlog(test_ctx->qserver, error_reason_text_log);
        test_ctx->qserver->use_long_log = 1;
        picoquic_set_qlog(test_ctx->qserver, ".");
        /* Now, start the client connection */
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }

    if (ret == 0) {
        /* Perform a connection loop to verify it goes OK */
        ret = tls_api_connection_loop(test_ctx, &loss_mask,
            2 * test_ctx->c_to_s_link->microsec_latency, &simulated_time);

        if (ret != 0)
        {
            DBG_PRINTF("Connection loop returns %d\n", ret);
        }
    }

    if (ret == 0) {
        /* force closure of the client connection with an internal error */
        int local_error_ret = picoquic_connection_error_ex(test_ctx->cnx_client, PICOQUIC_TRANSPORT_INTERNAL_ERROR,
            0, "error reason test");
        if (local_error_ret != PICOQUIC_ERROR_DETECTED) {
            DBG_PRINTF("picoquic_connection_error_ex returns %d\n", ret);
            ret = -1;
        }
    }

    if (ret == 0) {
        /* verify that the connection will be closed */
        int nb_trials = 0;
        int nb_inactive = 0;
        while (ret == 0 && nb_trials < 1024 && nb_inactive < 512 ) {
            int was_active = 0;
            nb_trials++;

            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

            if (test_ctx->cnx_client->cnx_state == picoquic_state_disconnected &&
                (test_ctx->cnx_server == NULL || test_ctx->cnx_server->cnx_state == picoquic_state_disconnected)) {
                break;
            }

            if (nb_trials == 512) {
                DBG_PRINTF("After %d trials, client state = %d, server state = %d",
                    nb_trials, (int)test_ctx->cnx_client->cnx_state,
                    (test_ctx->cnx_server == NULL) ? -1 : test_ctx->cnx_server->cnx_state);
            }

            if (was_active) {
                nb_inactive = 0;
            }
            else {
                nb_inactive++;
            }
        }
    }
    /* Close the contexts, which will close the logs.
     */

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn error_reason() {
    const ERROR_REASON: &str = "error reason test";
    const ERROR_REASON_TEXT_LOG: &str = "error_reason_log.txt";

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid = ConnectionId::clone_from_slice(&[0xe8, 0x80, 0x88, 0xea, 0x50, 0, 0, 0])
        .expect("initial CID");
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .expect("tls_api_init_ctx_ex");

    test_ctx
        .qserver
        .set_textlog(Some(ERROR_REASON_TEXT_LOG))
        .expect("server textlog");
    test_ctx.qserver.set_qlog(".").expect("server qlog");
    test_ctx.qserver.use_long_log = true;

    let queue_delay_max = 2 * test_ctx.c_to_s_link.microsec_latency;
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        queue_delay_max,
        &mut simulated_time,
    )
    .expect("connection loop");

    let local_error_ret = test_ctx.cnx_client().connection_error_ex(
        crate::errors::TransportError::InternalError as u64,
        0,
        Some(ERROR_REASON),
    );
    assert_eq!(
        local_error_ret,
        crate::errors::InternalError::Detected as i32,
        "connection_error_ex should report PICOQUIC_ERROR_DETECTED"
    );
    {
        let client = test_ctx.cnx_client();
        assert_eq!(
            client.local_error(),
            crate::errors::TransportError::InternalError as u64
        );
        assert_eq!(client.offending_frame_type, 0);
        assert_eq!(client.local_error_reason.as_deref(), Some(ERROR_REASON));
    }

    let mut nb_trials = 0;
    let mut nb_inactive = 0;
    while nb_trials < 1024 && nb_inactive < 512 {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )
        .expect("close simulation round");

        let client_disconnected = test_ctx.cnx_client().state() == State::Disconnected;
        let server_disconnected = !test_ctx.has_cnx_server()
            || test_ctx
                .qserver
                .first_cnx_mut()
                .map(|cnx| cnx.state() == State::Disconnected)
                .unwrap_or(true);
        if client_disconnected && server_disconnected {
            break;
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }
}
```

## `picoquictest/tls_api_test.c:tls_api_retry_test`
* C test-table name: `retry`
* C entry function: `tls_api_retry_test`
* Rust test: `retry`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6990-6992`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust exercises a retry-style handshake with cookie_mode=1, but C also enforces completion within 230000 usec. The Rust helper omits that target-time assertion and ignores the large_client_hello parameter.
* Phase 5A fix note: Add the C target-time check after close, and make tls_api_retry_test_one honor large_client_hello for the shared helper.
* Phase 5B analysis: Rust helper now checks the C retry completion bound and honors the shared large_client_hello flag. The faithful retry test now exposes an implementation timing mismatch: simulated completion is 30000000 usec, above the C limit of 230000 usec.
* Phase 5B fix note: Updated tls_api_retry_test_one to set test_large_chello when requested, enable client qlog like C, close the retry connection, and assert the 230000 usec post-close target time.

### C test body
```c
{
    return tls_api_retry_test_one(0);
}
```

### Current Rust test body
```rust
fn retry() {
    tls_api_retry_test_one(false).expect("retry");
}
```

## `picoquictest/tls_api_test.c:tls_api_inject_hs_ack_test`
* C test-table name: `tls_api_inject_hs_ack`
* C entry function: `tls_api_inject_hs_ack_test`
* Rust test: `tls_api_inject_hs_ack`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8046-8080`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust has the rough shape, but the injected ACK is effectively empty because tester_simple_ack_frame returns Vec::new(); it injects directly instead of queueing like the C helper and skips the C final verification/close path.
* Phase 5A fix note: Encode the same Handshake ACK payload, require the handshake key wait to actually succeed, queue the forged packet on the client-to-server sim link, continue to ready, then run the final SNI/ALPN/version/close verification.
* Phase 5B analysis: Rust test is present, compiles, and now matches the C API-level setup/injection/final-check contract; any failure to derive the Handshake AEAD key before injection is a Phase 5C runtime-library issue.
* Phase 5B fix note: Changed tls_api_inject_hs_ack to initialize with V1/PICOQUIC_INTERNAL_TEST_VERSION_1 instead of negotiated version 0.

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", PICOQUIC_INTERNAL_TEST_VERSION_1);
    }

    if (ret == 0) {
        int ret = 0;
        int nb_trials = 0;
        int nb_inactive = 0;
        int injected = 0;

        while (ret == 0 && nb_trials < 1024 && nb_inactive < 512 && (!TEST_CLIENT_READY || (test_ctx->cnx_server == NULL || !TEST_SERVER_READY))) {
            int was_active = 0;
            nb_trials++;

            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

            if (test_ctx->cnx_client->cnx_state == picoquic_state_disconnected &&
                (test_ctx->cnx_server == NULL || test_ctx->cnx_server->cnx_state == picoquic_state_disconnected)) {
                break;
            }

            if (!injected && test_ctx->cnx_client->crypto_context[2].aead_encrypt != NULL) {
                const uint8_t ack_only[] = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    picoquic_frame_type_ack, 0, 0, 0, 0 };

                ret = tls_api_inject_packet(test_ctx, 1, 2, ack_only, sizeof(ack_only), 0, simulated_time);

                injected = 1;
            }

            if (was_active) {
                nb_inactive = 0;
            }
            else {
                nb_inactive++;
            }
        }

        if (ret != 0)
        {
            DBG_PRINTF("Connection loop returns %d\n", ret);
        }
    }

    if (ret == 0) {
        ret = tls_api_test_with_loss_final(test_ctx, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time);
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
fn tls_api_inject_hs_ack() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tester_wait_handshake_key(&mut ctx, &mut t).expect("handshake_key");
    {
        let client = ctx.cnx_client();
        assert!(
            client.connection_state >= State::ClientHandshakeStart
                && client.crypto_context[crate::internal::Epoch::Handshake as usize]
                    .aead_encrypt
                    .is_some(),
            "client did not derive handshake epoch keys before ACK injection"
        );
    }

    let mut ack_frame = vec![crate::frames::FrameType::Padding as u8; 15];
    ack_frame.extend_from_slice(&tester_simple_ack_frame(0));
    tester_push_frame_packet(&mut ctx, PacketType::Handshake, &ack_frame, false, true, t)
        .expect("queue handshake ack");

    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("inject_hs_ack");
    {
        let client = ctx
            .qclient
            .first_cnx_mut()
            .expect("client connection not initialized");
        let server = ctx
            .qserver
            .first_cnx_mut()
            .expect("server connection not accepted");
        assert_tls_api_final_negotiation(client, server, Some(TEST_SNI), Some(TEST_ALPN));
    }
    tls_api_close_with_losses(&mut ctx, &mut t, 0).expect("inject_hs_ack close");
}
```

## `picoquictest/congestion_test.c:bbr_test`
* C test-table name: `bbr`
* C entry function: `bbr_test`
* Rust test: `bbr`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:781-784`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper passes bbr, 3500000, 0, 0 like C, but the Rust shared verifier ignores max_completion_time and stream/error checks; the registered bbr entry also points at BASELINE_CC.
* Phase 5A fix note: Make scenario verification enforce stream completion/errors and completion_time <= max_completion_time; ensure bbr maps to the real translated BBR behavior before relying on this test.
* Phase 5B analysis: Rust bbr is present as a #[test], compiles under the Rust test harness, selects the registered bbr algorithm, and calls congestion_control_test with the same API-level inputs as C: max_completion_time 3500000, jitter 0, jitter_id 0. The bbr registry still using BASELINE_CC is a Phase 5C implementation/runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return congestion_control_test(picoquic_bbr_algorithm, 3500000, 0, 0);
}
```

### Current Rust test body
```rust
fn bbr() {
    let ccalgo = cc_algo("bbr");
    congestion_control_test(ccalgo, 3_500_000, 0, 0);
}
```

## `picoquictest/congestion_test.c:bbr_slow_long_test`
* C test-table name: `bbr_slow_long`
* C entry function: `bbr_slow_long_test`
* Rust test: `bbr_slow_long`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:822-827`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The test passes the same BBR slow-long parameters, but the shared Rust scenario verifier ignores max_completion_microsec and omits the C scenario completion/stream verification, so the performance bound is not checked.
* Phase 5A fix note: Implement Rust tls_api_one_scenario_body_verify to enforce scenario completion/stream results and max completion time, then this test can rely on the same checks as C.
* Phase 5B analysis: Rust #[test] is present/runnable and matches the C API-level contract: same latency, jitter, buffer derivation, Mbps, max_completion_time, performance_test call, and shared stream-completion/max-completion verifier. Prior callback/harness runtime failures are Phase 5C notes, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_time = 81000000;
    uint64_t latency = 300000;
    uint64_t jitter = 3000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 1;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Current Rust test body
```rust
fn bbr_slow_long() {
    let latency = 300_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(81_000_000, 1, latency, jitter, buffer);
}
```
