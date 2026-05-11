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

## `picoquictest/tls_api_test.c:transmit_cnxid_retire_early_test`
* C test-table name: `cnxid_transmit_r_early`
* C entry function: `transmit_cnxid_retire_early_test`
* Rust test: `cnxid_transmit_r_early`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1697-1699`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the same boolean tuple, but transmit_cnxid_test_one ignores all three flags and only performs a generic handshake, sync, timeout, and close. It misses the C early retire-before setup and CID/stash sequence assertions.
* Phase 5A fix note: Implement the transmit_cnxid_test_one scenarios: honor early retire_number_zero by creating the server connection early and setting retire_before=1, sync until path targets, assert path 0 uses a nonzero remote CID, verify local CID counts and client/server stash contents, and preserve retire_before/disable behavior for shared helper correctness.
* Phase 5B analysis: Rust test is present as a runnable #[test], compiles under the Rust test harness, and calls transmit_cnxid_test_one(false, false, true), matching C transmit_cnxid_test_one(0, 0, 1). The helper expresses the early-retire setup plus CID count/stash API-visible assertions; failures from incomplete NEW_CONNECTION_ID/RETIRE_CONNECTION_ID behavior are Phase 5C runtime notes, not Phase 5B blocks.
* Phase 5B fix note: 

### C test body
```c
{
    return transmit_cnxid_test_one(0, 0, 1);
}
```

### Current Rust test body
```rust
fn cnxid_transmit_r_early() {
    transmit_cnxid_test_one(false, false, true).expect("cnxid_transmit_r_early");
}
```

## `picoquictest/tls_api_test.c:document_addresses_test`
* C test-table name: `document_addresses`
* C entry function: `document_addresses_test`
* Rust test: `document_addresses`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2052-2101`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic handshake/close helper. It does not install the client/server address callbacks, run the q_and_r scenario with queue delay, or verify almost_ready/ready callback counts and local/remote addresses.
* Phase 5A fix note: Add a faithful Rust test that wraps callbacks on client and server, records almost_ready and ready addresses, runs q_and_r with queue_delay_max=20000 and max_completion=3600000, then checks both directions against client_addr/server_addr.
* Phase 5B analysis: Reclassified ok: the Rust #[test] compiles, is runnable by the Rust test harness, installs matching client/server address callback wrappers, runs the same q_and_r scenario parameters, and asserts the same API-visible almost-ready/ready address contract. Runtime Generic failure or callback propagation issues are Phase 5C behavior work.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0; 
    tls_api_address_are_documented_t client_address_callback_ctx, server_address_callback_ctx;

    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }
    if (ret == 0) {
        /* Set the call backs to intercept the almost ready and ready transitions */
        memset(&client_address_callback_ctx, 0, sizeof(tls_api_address_are_documented_t));
        client_address_callback_ctx.callback_fn = picoquic_get_callback_function(test_ctx->cnx_client);
        client_address_callback_ctx.callback_ctx = picoquic_get_callback_context(test_ctx->cnx_client);
        picoquic_set_callback(test_ctx->cnx_client, test_local_address_callback, &client_address_callback_ctx);

        memset(&server_address_callback_ctx, 0, sizeof(tls_api_address_are_documented_t));
        server_address_callback_ctx.callback_fn = picoquic_get_default_callback_function(test_ctx->qserver);
        server_address_callback_ctx.callback_ctx = picoquic_get_default_callback_context(test_ctx->qserver);
        picoquic_set_default_callback(test_ctx->qserver, test_local_address_callback, &server_address_callback_ctx);

        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 20000, 3600000);
    }

    /* Verify that the addresses and calls are what we expect */
    if (ret == 0) {
        ret = document_addresses_check(&client_address_callback_ctx,
            (struct sockaddr*)&test_ctx->client_addr, (struct sockaddr*)&test_ctx->server_addr);
        if (ret != 0) {
            DBG_PRINTF("%s", "Client addresses were not properly documented\n");
        }
    }

    if (ret == 0) {
        ret = document_addresses_check(&server_address_callback_ctx,
            (struct sockaddr*)&test_ctx->server_addr, (struct sockaddr*)&test_ctx->client_addr);
        if (ret != 0) {
            DBG_PRINTF("%s", "Server addresses were not properly documented\n");
        }
    }

    /* Free the resource */

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn document_addresses() {
    let mut simulated_time = Instant::from_ticks(0);
    let client_address_state =
        std::rc::Rc::new(std::cell::RefCell::new(DocumentAddressState::default()));
    let server_address_state =
        std::rc::Rc::new(std::cell::RefCell::new(DocumentAddressState::default()));
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    {
        let cnx = test_ctx.cnx_client();
        let inner = cnx.callback_fn.take();
        cnx.set_callback(Some(Box::new(DocumentAddressCallback {
            state: std::rc::Rc::clone(&client_address_state),
            inner,
        })));
    }

    {
        let inner = test_ctx.qserver.default_callback_fn.take();
        test_ctx
            .qserver
            .set_default_callback(Some(Box::new(DocumentAddressCallback {
                state: std::rc::Rc::clone(&server_address_state),
                inner,
            })));
    }

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        0,
        0,
        0,
        20_000,
        3_600_000,
    )
    .expect("document_addresses scenario");

    document_addresses_check(
        &client_address_state.borrow(),
        test_ctx.client_addr,
        test_ctx.server_addr,
    );
    document_addresses_check(
        &server_address_state.borrow(),
        test_ctx.server_addr,
        test_ctx.client_addr,
    );
}
```

## `picoquictest/tls_api_test.c:immediate_ack_test`
* C test-table name: `immediate_ack`
* C entry function: `immediate_ack_test`
* Rust test: `immediate_ack`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2929-2931`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic no-loss TLS API handshake/close. It does not use the fixed initial CID, set qlog, queue an IMMEDIATE_ACK misc frame, observe the server immediate-ack flag, verify it clears without time advancing, or check the client backlog becomes empty after the ACK.
* Phase 5A fix note: Add a dedicated immediate_ack Rust test mirroring the C flow: init with CID 1a..., complete handshake/readiness, queue FrameType::ImmediateAck on the client application context, drive bounded rounds, assert server ack_ctx immediate flag set then cleared at the same simulated time, then assert client backlog empties.
* Phase 5B analysis: Rust test is present, compiled by the test harness, and already expresses the C API-level flow and assertions. The immediate-ACK flag runtime failure is a Phase 5C library-behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x1a, 0x1a, 0x1a, 0x1a, 0x1a, 0x1a, 0x1a, 0x1a}, 8 };
    int ret;

    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0) {
        picoquic_set_qlog(test_ctx->qserver, ".");
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        int nb_trials = 0;
        int was_active;
        uint64_t immediate_received_at_server = 0;
        uint64_t immediate_cleared_at_server = 0;
        int all_acked = 0;
        uint8_t immediate_ack_frame[2] = { picoquic_frame_type_immediate_ack };
        /* Queue misc frame with "Immediate ACK" set */
        picoquic_queue_misc_frame(test_ctx->cnx_client, immediate_ack_frame, 2, 0,
            picoquic_packet_context_application);
        /* Do couple of rounds until the frame is received;
         * Check that it is received my verifying that the "immediate ACK" 
         * is set in the ACK context at the server. Check the time.
         */
        while (ret == 0 && nb_trials < 16) {
            nb_trials++;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            if (test_ctx->cnx_server != NULL &&
                test_ctx->cnx_server->ack_ctx[picoquic_packet_context_application].act[0].is_immediate_ack_required) {
                immediate_received_at_server = simulated_time;
                break;
            }
        }
        if (ret == 0 && immediate_received_at_server == 0) {
            DBG_PRINTF("Immediate ACK not received after %d rounds", nb_trials);
            ret = -1;
        }
        /* Do a couple rounds until the "immediate ACK" flag is not
         * set at the server anymore. Verify that no time is elapsed since
         * the end of the previous round */
        nb_trials = 0;
        while (ret == 0 && nb_trials < 16) {
            nb_trials++;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            if (test_ctx->cnx_server != NULL &&
                !test_ctx->cnx_server->ack_ctx[picoquic_packet_context_application].act[0].is_immediate_ack_required) {
                immediate_cleared_at_server = simulated_time;
                break;
            }
        }
        if (ret != 0){
            if (immediate_cleared_at_server == 0) {
                DBG_PRINTF("Immediate ACK not cleared after %d rounds", nb_trials);
                ret = -1;
            }
            else if (immediate_cleared_at_server != immediate_received_at_server) {
                DBG_PRINTF("ACK not quite immediate, set at: %" PRIu64 ", cleared at %" PRIu64,
                    immediate_received_at_server, immediate_cleared_at_server);
                ret = -1;
            }
        }
        /* Do couple rounds until the ACK is received at the client. 
         * This is verified by checking that the client ACK queue is
         * empty.
         */
        nb_trials = 0;
        while (ret == 0 && nb_trials < 32) {
            nb_trials++;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            if (test_ctx->cnx_client != NULL && 
                picoquic_is_cnx_backlog_empty(test_ctx->cnx_client)) {
                all_acked = 1;
                break;
            }
        }
        if (ret == 0 && !all_acked) {
            DBG_PRINTF("ACK was not received at: %" PRIu64, simulated_time);
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
fn immediate_ack() {
    immediate_ack_test_one().expect("immediate_ack");
}
```

## `picoquictest/tls_api_test.c:key_rotation_auto_client`
* C test-table name: `key_rotation_client`
* C entry function: `key_rotation_auto_client`
* Rust test: `key_rotation_client`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3381-3383`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls key_rotation_auto_one(400,true), but the helper uses a different one-stream scenario, only sets the client epoch length, and never verifies the peer's nb_crypto_key_rotations against the expected/max range computed from sent packet count.
* Phase 5A fix note: Implement key_rotation_auto_one with the C key_rotation scenario, client/server epoch-length selection, 2s scenario body, and rotation-count bounds check on the opposite endpoint; key_rotation_client should use epoch_length 400 and client_test=true.
* Phase 5B analysis: Current Rust source has runnable #[test] key_rotation_client calling key_rotation_auto_one(400, true). The helper matches the C API-level contract; missing automatic key-rotation behavior is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return key_rotation_auto_one(400, 1);
}
```

### Current Rust test body
```rust
fn key_rotation_client() {
    key_rotation_auto_one(400, true).expect("key_rotation_client");
}
```

## `picoquictest/tls_api_test.c:migration_test`
* C test-table name: `migration`
* C entry function: `migration_test`
* Rust test: `migration`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4229-4231`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust migration helper uses a different default scenario, does not initialize with the C cid_zero/8-CID parameters, and omits C assertions for selected remote CID, local CID renewal, challenge renewal/verification, and post-migration scenario verification.
* Phase 5A fix note: Translate migration_test_scenario faithfully for the basic migration test: use q_and_r {4,0,257,2000}, initialize like tls_api_init_ctx_ex2, probe after port change, capture target/previous CIDs, verify stream completion, challenge renewal/verification, and CID changes.
* Phase 5B analysis: Rust migration test is present as a harness-runnable #[test] and now calls migration_test_scenario(TEST_SCENARIO_Q_AND_R, 0, false), matching the C entry. The helper expresses the API-visible migration contract; any probe_new_path Error::Generic/runtime failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return migration_test_scenario(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0);
}
```

### Current Rust test body
```rust
fn migration() {
    migration_test_scenario(TEST_SCENARIO_Q_AND_R, 0, false).expect("migration");
}
```
