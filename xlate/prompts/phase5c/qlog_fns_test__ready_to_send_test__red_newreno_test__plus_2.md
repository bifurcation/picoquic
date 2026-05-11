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

## `picoquictest/tls_api_test.c:qlog_fns_test`
* C test-table name: `qlog_fns`
* C entry function: `qlog_fns_test`
* Rust test: `qlog_fns`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6552-6554`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls qlog_fns_test_one(0), but that helper just delegates to a simplified qlog_trace_test_one; it omits custom CID callbacks, reset seeds, deterministic initial CID/cipher settings, bad-packet injection, qlog generation, and reference-file comparison.
* Phase 5A fix note: Implement qlog_fns_test_one with the C setup: qlog enabled, spin/loss bits, CID callback data, reset seeds, deterministic client connection, q2/r2 scenario, bad packet injection, teardown, and qlog reference comparison.
* Phase 5B analysis: Reclassified ok: Rust #[test] qlog_fns delegates to qlog_fns_test_one(0), matching the C entry body, and the helper expresses the C API-level setup/assertions. Phase 5C note: runtime may still fail from incomplete q2/r2 TLS and qlog backend behavior.
* Phase 5B fix note: 

### C test body
```c
{
    return qlog_fns_test_one(0);
}
```

### Current Rust test body
```rust
fn qlog_fns() {
    qlog_fns_test_one(0).expect("qlog_fns");
}
```

## `picoquictest/tls_api_test.c:ready_to_send_test`
* C test-table name: `ready_to_send`
* C entry function: `ready_to_send_test`
* Rust test: `ready_to_send`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6808-6810`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls ready_to_send_test_one(1), but the helper ignores the option, does not model stream0_test_option, uses different scenario sizes, and relies on weaker scenario/final verification than the C ready-to-send callback test.
* Phase 5A fix note: Implement the stream-0 prepare-to-send behavior and option handling, use the C test_scenario_q_and_r inputs, set stream0_target=1000000, and verify stream0 plus scenario completion as C does.
* Phase 5B analysis: Rust #[test] ready_to_send is present, compiles under the test harness, and calls ready_to_send_test_one(1), matching the C entry. Earlier PrepareToSend/StreamData/StreamFin behavior gaps are Phase 5C runtime/library notes, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = ready_to_send_test_one(1);
    return ret;
}
```

### Current Rust test body
```rust
fn ready_to_send() {
    ready_to_send_test_one(1).expect("ready_to_send");
}
```

## `picoquictest/tls_api_test.c:red_newreno_test`
* C test-table name: `red_newreno`
* C entry function: `red_newreno_test`
* Rust test: `red_newreno`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6872-6874`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the same visible arguments but the helper checks materially different behavior: it does not select NewReno, configure RED AQM/link parameters, use the sustained multi-stream scenario, or assert observed server retransmissions stay under the loss target.
* Phase 5A fix note: Make red_cc_algotest configure the requested congestion algorithm, RED queues, latency/bandwidth, sustained scenario, and observed-loss threshold, then keep red_newreno using newreno, 500000, 150.
* Phase 5B analysis: Rust red_newreno is a #[test], compiles under cargo check --tests, and calls red_cc_algotest("newreno", 500_000, 150). The helper matches the C RED setup and API-visible assertions; the prior server-connection/observed_loss failure is a Phase 5C runtime-library issue.
* Phase 5B fix note: No Rust test repair needed; COMMANDS.log updated for required command logging.

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_newreno_algorithm, 500000, 150);
    return ret;
}
```

### Current Rust test body
```rust
fn red_newreno() {
    red_cc_algotest("newreno", 500_000, 150).expect("red_newreno");
}
```

## `picoquictest/tls_api_test.c:stateless_reset_test`
* C test-table name: `stateless_reset`
* C entry function: `stateless_reset_test`
* Rust test: `stateless_reset`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7661-7756`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the generic TLS handshake/close helper; it does not exercise stateless reset generation or handling.
* Phase 5A fix note: Implement the C scenario: verify reset secret, delete server connection, send stream 4 data, run sim rounds, assert client disconnected and stateless-reset callback observed.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, and expresses the C API-level contract. The reset-secret/runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    uint8_t buffer[128];
    int was_active = 0;

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    /* verify that client and server have the same reset secret */
    if (ret == 0) {
        uint8_t ref_secret[PICOQUIC_RESET_SECRET_SIZE];

        (void)picoquic_create_cnxid_reset_secret(test_ctx->qserver,
            &test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id, ref_secret);
        if (memcmp(test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->reset_secret, ref_secret,
            PICOQUIC_RESET_SECRET_SIZE) != 0) {
            ret = -1;
        }
    }

    /* Prepare to reset */
    if (ret == 0) {
        picoquic_delete_cnx(test_ctx->cnx_server);
        test_ctx->cnx_server = NULL;

        memset(buffer, 0xaa, sizeof(buffer));
        ret = picoquic_add_to_stream(test_ctx->cnx_client, 4,
            buffer, sizeof(buffer), 1);
    }

    /* Perform a couple rounds of sending data */
    for (int i = 0; ret == 0 && i < 64 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected; i++) {
        was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
    }

    /* Client should now be in state disconnected */
    if (ret == 0 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
        ret = -1;
    }

    if (ret == 0 && test_ctx->reset_received == 0) {
        ret = -1;
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
fn stateless_reset() {
    struct StatelessResetTracker {
        reset_received: Rc<Cell<bool>>,
    }

    impl StreamDataCallback for StatelessResetTracker {
        fn callback(
            &mut self,
            _connection: &mut Connection,
            _stream_id: u64,
            _bytes: &[u8],
            fin_or_event: CallbackEvent,
            _stream_ctx: Option<&mut dyn core::any::Any>,
        ) -> i32 {
            if fin_or_event == CallbackEvent::StatelessReset {
                self.reset_received.set(true);
            }
            0
        }
    }

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time).expect("client ready");

    let (remote_cid, client_reset_secret) = {
        let client = test_ctx.cnx_client();
        let path = client.paths.first().expect("client path");
        let tuple = path.tuples.first().expect("client tuple");
        let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
        let remote_cid = client
            .remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == path.unique_path_id)
            .and_then(|stash| stash.connection_ids.get(cid_index))
            .expect("client remote connection id");
        (remote_cid.connection_id, remote_cid.reset_secret)
    };
    let mut ref_secret = [0u8; crate::RESET_SECRET_SIZE];
    test_ctx
        .qserver
        .create_connection_id_reset_secret(&remote_cid, &mut ref_secret)
        .expect("reference reset secret");
    assert_eq!(
        client_reset_secret, ref_secret,
        "client and server reset secrets differ"
    );

    let reset_received = Rc::new(Cell::new(false));
    test_ctx
        .cnx_client()
        .set_callback(Some(Box::new(StatelessResetTracker {
            reset_received: Rc::clone(&reset_received),
        })));

    let server_token = test_ctx.cnx_server().own_token.expect("server token");
    test_ctx.qserver.delete_connection(server_token);
    assert!(
        !test_ctx.has_cnx_server(),
        "server connection was not deleted"
    );

    let buffer = [0xaa; 128];
    test_ctx
        .cnx_client()
        .add_to_stream(4, &buffer, true)
        .expect("queue stream 4 data");

    for _ in 0..64 {
        if test_ctx.cnx_client().state() == State::Disconnected {
            break;
        }
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )
        .expect("stateless reset sim round");
    }

    assert_eq!(
        test_ctx.cnx_client().state(),
        State::Disconnected,
        "client did not disconnect after stateless reset"
    );
    assert!(
        reset_received.get(),
        "client callback did not observe stateless reset"
    );
}
```

## `picoquictest/tls_api_test.c:tls_api_alpn_test`
* C test-table name: `tls_api_alpn`
* C entry function: `tls_api_alpn_test`
* Rust test: `tls_api_alpn`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8020-8028`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C expects the no-ALPN connection attempt to fail specifically with PICOQUIC_ERROR_NO_ALPN_PROVIDED. Rust calls the helper with None but expects success, and the helper weakens ALPN checks.
* Phase 5A fix note: Change the Rust test to assert the no-ALPN error, and ensure the helper models the C setup: client ALPN omitted, server default ALPN present, specific error propagated.
* Phase 5B analysis: Rust test is present, compiles, is runnable by the Rust test harness, calls the matching helper with missing client ALPN, and asserts Protocol(NoAlpnProvided). Any runtime success instead is a Phase 5C library behavior issue.
* Phase 5B fix note: No test source changes; appended COMMANDS.log per repo instruction.

### C test body
```c
{
    int ret = tls_api_test_with_loss(NULL, 0, PICOQUIC_TEST_SNI, NULL);

    if (ret == PICOQUIC_ERROR_NO_ALPN_PROVIDED) {
        ret = 0;
    } else if (ret == 0) {
        DBG_PRINTF("ALPN test succeeds while no ALPN is specified, ret = 0x%x", ret);
        ret = -1;
    }
    else {
        DBG_PRINTF("ALPN test does not return expected error code, ret = 0x%x", ret);
        ret = -1;
    }
    return ret;
}
```

### Current Rust test body
```rust
fn tls_api_alpn() {
    let err = tls_api_test_with_loss(None, 0, Some(TEST_SNI), None)
        .expect_err("tls_api_alpn should reject a missing client ALPN");
    assert_eq!(
        err,
        crate::Error::Protocol(crate::InternalError::NoAlpnProvided as u64),
        "tls_api_alpn returned the wrong error for missing ALPN"
    );
}
```
