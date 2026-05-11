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

## `picoquictest/satellite_test.c:satellite_small_test`
* C test-table name: `satellite_small`
* C entry function: `satellite_small_test`
* Rust test: `satellite_small`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:351-366`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper arguments match, but the Rust helper does not actually configure the C test's stream0_target/data_size transfer; data_size is passed into an ignored parameter, so the 100MB transfer/time-bound behavior is not checked.
* Phase 5A fix note: Wire data_size to stream0_target, for example by using or fixing tls_api_one_scenario_body_ex, so satellite_small sends 100MB over the 10/2 Mbps simulated link and enforces the 81,500,000 us completion bound.
* Phase 5B analysis: Reclassified ok: current Rust `satellite_small` is a `#[test]`, selects BBR, and calls `satellite_test_one` with the same API-level parameters as C, including data_size as stream0 target and the 81,500,000 us bound. The `CannotSetActiveStream`/stream callback failure is a Phase 5C runtime-library note, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Should be less than 85 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 81500000, 10, 2, 0, 0, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_small() {
    let bbr = satellite_ccalgo("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        81_500_000,
        10,
        2,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/sockloop_test.c:sockloop_migration_test`
* C test-table name: `sockloop_migration`
* C entry function: `sockloop_migration_test`
* Rust test: `sockloop_migration`
* Expected Rust file: `rs/fq/src/tests/sockloop.rs`
* Current Rust span: `rs/fq/src/tests/sockloop.rs:1007-1014`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Wrapper parameters match the C setup, but the Rust sockloop driver never consumes spec.scenario and does not perform the C scenario completion verification after migration, so the 1M stream transfer behavior is not checked.
* Phase 5A fix note: In sockloop_test_one_result, register spec.scenario with the test scenario helper and verify stream completion after successful packet loop and migration checks, matching C test_api_init_send_recv_scenario plus tls_api_one_scenario_verify behavior.
* Phase 5B analysis: Rust #[test] is present, compiles under the Rust test harness, and expresses the same API-level contract: test id 5, IPv6 via SockloopTestSpec::new default, socket buffer 0xffff, 1M scenario, extra socket required, force_migration=3, and sockloop_test_one. Runtime UDP/socket Generic failures are Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 5);
    spec.af = AF_INET6;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.extra_socket_required = 1;
    spec.force_migration = 3;

    return(sockloop_test_one(&spec));
}
```

### Current Rust test body
```rust
fn sockloop_migration() {
    let mut spec = SockloopTestSpec::new(5);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.extra_socket_required = true;
    spec.force_migration = 3;
    sockloop_test_one(&spec);
}
```

## `picoquictest/ticket_store_test.c:ticket_seed_from_bdp_frame_test`
* C test-table name: `ticket_seed_from_bdp_frame`
* C entry function: `ticket_seed_from_bdp_frame_test`
* Rust test: `ticket_seed_from_bdp_frame`
* Expected Rust file: `rs/fq/src/tests/ticket_store.rs`
* Current Rust span: `rs/fq/src/tests/ticket_store.rs:25-27`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test calls a helper with mode 2, but that helper manually seeds ticket IDs, RTT/CWIN values, stored-ticket fields, issued tickets, and resumed IDs. The C helper relies on real connection/data loops, actual ticket creation, deletion/recreation, resumption, and BDP option 2 to prove seeding behavior.
* Phase 5A fix note: Rework Rust ticket_seed_test_one(2) to avoid manual seeding, set the BDP option to the mode-2 behavior, inspect actual client stored tickets and server issued tickets after the first transfer, recreate/resume the connection, and assert resumed ticket IDs plus nonzero seed RTT/CWIN come from the real ticket path.
* Phase 5B analysis: Rust #[test] calls ticket_seed_test_one(2), matching the C entry function's ticket_seed_test_one(2). The test compiles under the Rust harness; incomplete ticket storage/resume behavior is a Phase 5C runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
int ticket_seed_from_bdp_frame_test(void) {
    
   return ticket_seed_test_one(2);
}
```

### Current Rust test body
```rust
fn ticket_seed_from_bdp_frame() {
    ticket_seed_test_one(2).expect("ticket_seed_from_bdp_frame");
}
```

## `picoquictest/tls_api_test.c:bad_cnxid_test`
* C test-table name: `bad_cnxid`
* C entry function: `bad_cnxid_test`
* Rust test: `bad_cnxid`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1252-1331`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic empty scenario; it does not install the header fuzzer, require fuzzing, verify server connection disappearance, or prove a second connection succeeds after clearing the fuzzer.
* Phase 5A fix note: Implement the header fuzzer path, run the very-long scenario through connect plus data loop, assert fuzzed packets and server disconnect/removal, then recreate the client and run the q_and_r second connection scenario.
* Phase 5B analysis: Reclassified as ok: the Rust test is present, compiles, is a runnable #[test], and expresses the C API-level flow and assertions. The fuzzer packet count remaining zero is a Rust library behavior gap for Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    header_fuzzer_ctx_t fuzz_ctx;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    memset(&fuzz_ctx, 0, sizeof(fuzz_ctx));
    fuzz_ctx.random_context = 0x123456789ABCDEF0ull;

    if (ret == 0) {
        /* Prepare to send data */
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    if (ret == 0) {
        /* establish the connection */
        ret = tls_api_one_scenario_body_connect(test_ctx, &simulated_time, 0, 0);
    }

    /* Set fuzzer, then perform a data sending loop */
    if (ret == 0) {
        picoquic_set_fuzz(test_ctx->qclient, header_fuzzer, &fuzz_ctx);

        (void) tls_api_data_sending_loop(test_ctx, NULL, &simulated_time, 0);


        /* verify that the server connection has disappeared */
        if (fuzz_ctx.nb_fuzzed > 0 && (test_ctx->cnx_server == NULL || test_ctx->cnx_server->cnx_state == picoquic_state_disconnected)) {
            ret = 0;
        }
        else {
            DBG_PRINTF("Unexpected server state: %d, packet: %d, fuzzed: %d\n", test_ctx->cnx_server->cnx_state, 
                fuzz_ctx.nb_packets, fuzz_ctx.nb_fuzzed);
            ret = -1;
        }
    }

    if (ret == 0) {
        /* Remove the reference to the old server connection */
        test_ctx->cnx_server = NULL;
        /* Delete the old client connection */
        picoquic_delete_cnx(test_ctx->cnx_client);
        test_ctx->cnx_client = NULL;
        /* Remove the fuzzer */
        picoquic_set_fuzz(test_ctx->qclient, NULL, NULL);
        /* re-create a client connection */
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient,
            picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*) & test_ctx->server_addr, simulated_time,
            PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);
        if (test_ctx->cnx_client == NULL) {
            DBG_PRINTF("%s", "Could not create second client connection\n");    
            ret = -1;
        }
        else {
            for (size_t i = 0; i < test_ctx->nb_test_streams; i++) {
                test_api_delete_test_stream(&test_ctx->test_stream[i]);
            }
            ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
                test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 20000, 100000);
            if (ret != 0) {
                DBG_PRINTF("Second connection fails, ret=%d (x%x)\n", ret, ret);
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
fn bad_cnxid() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");
    let mut loss_mask = 0u64;
    let fuzz_state = std::rc::Rc::new(std::cell::RefCell::new(HeaderFuzzerState {
        random_context: 0x1234_5678_9abc_def0,
        nb_packets: 0,
        nb_fuzzed: 0,
    }));

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    tls_api_one_scenario_body_connect(&mut test_ctx, &mut simulated_time, 0, 0).expect("connect");

    test_ctx.qclient.set_fuzz(Some(Box::new(HeaderFuzzer {
        state: std::rc::Rc::clone(&fuzz_state),
    })));
    let _ = tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0);

    let stats = fuzz_state.borrow();
    assert!(
        stats.nb_fuzzed > 0,
        "header fuzzer did not fuzz any packet; packets={}",
        stats.nb_packets
    );

    let server_state = test_ctx.qserver.first_connection().map(|cnx| cnx.state());
    assert!(
        matches!(server_state, None | Some(State::Disconnected)),
        "unexpected server state after header fuzzing: {:?}; packets={}, fuzzed={}",
        server_state,
        stats.nb_packets,
        stats.nb_fuzzed
    );
    drop(stats);

    let old_client = test_ctx
        .qclient
        .first_connection()
        .and_then(|cnx| cnx.own_token)
        .expect("old client connection token");
    test_ctx.qclient.delete_connection(old_client);
    test_ctx.qclient.set_fuzz(None);

    if let Some((Some(server_token), State::Disconnected)) = test_ctx
        .qserver
        .first_connection()
        .map(|cnx| (cnx.own_token, cnx.state()))
    {
        test_ctx.qserver.delete_connection(server_token);
    }

    test_ctx
        .qclient
        .create_connection(
            ConnectionId::with_size(0).expect("initial CID"),
            ConnectionId::with_size(0).expect("remote CID"),
            Some(&test_ctx.server_addr),
            simulated_time,
            V1,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .expect("second client connection")
        .start_client()
        .expect("start second client connection");

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        0,
        0,
        0,
        20_000,
        100_000,
    )
    .expect("second q_and_r connection");
}
```

## `picoquictest/tls_api_test.c:client_error_test`
* C test-table name: `client_error`
* C entry function: `client_error_test`
* Rust test: `client_error`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1525-1529`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs one generic successful scenario. The C test loops three modes, injects invalid stream, NEW_CONNECTION_ID, and STOP_SENDING frames, requires server disconnect, then creates a fresh connection in the same context and closes it cleanly.
* Phase 5A fix note: Add a Rust modal helper for modes 0, 1, and 2 that runs the q_and_r scenario, queues the corresponding malformed application frame, verifies server disconnect, recreates the client connection, completes handshake/application AEAD readiness, and closes.
* Phase 5B analysis: Current Rust #[test] iterates the same three C modes, and client_error_modal mirrors the C API-level flow. The known disconnect/AEAD runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    char const* mode_name[] = { "stream", "new_connection_id", "stop_sending" };
    int nb_modes = (int)(sizeof(mode_name) / sizeof(char const*));

    for (int mode = 0; mode < nb_modes; mode++) {
        if (client_error_test_modal(mode) != 0) {
            DBG_PRINTF("Client error test mode(%s) failed.\n", mode_name[mode]);
            ret = -1;
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn client_error() {
    for (mode, name) in [(0, "stream"), (1, "new_connection_id"), (2, "stop_sending")] {
        client_error_modal(mode).unwrap_or_else(|e| panic!("client_error({name}): {e:?}"));
    }
}
```
