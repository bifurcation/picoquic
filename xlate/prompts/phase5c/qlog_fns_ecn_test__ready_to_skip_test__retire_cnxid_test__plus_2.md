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

## `picoquictest/tls_api_test.c:qlog_fns_ecn_test`
* C test-table name: `qlog_fns_ecn`
* C entry function: `qlog_fns_ecn_test`
* Rust test: `qlog_fns_ecn`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6560-6562`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust wrapper passes 0x02, but qlog_fns_test_one is just qlog_trace_test_one; it omits fixed CID callback setup, reset seeds, AES/key-exchange forcing, q2_and_r2 scenario with loss/queue-delay parameters, bad-packet injection, and qlog reference comparison.
* Phase 5A fix note: Implement qlog_fns_test_one faithfully for recv_ecn=0x02, including qlog setup, CID callback behavior, deterministic seeds/cipher settings, expected scenario parameters, bad-packet log exercise, and comparison against the ECN qlog reference.
* Phase 5B analysis: Rust #[test] is present and calls qlog_fns_test_one(0x02), matching the C entry function. The shared Rust helper expresses the C API-level qlog-fns contract; the missing qlog file/write behavior is a Phase 5C implementation-runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return qlog_fns_test_one(0x02);
}
```

### Current Rust test body
```rust
fn qlog_fns_ecn() {
    qlog_fns_test_one(0x02).expect("qlog_fns_ecn");
}
```

## `picoquictest/tls_api_test.c:ready_to_skip_test`
* C test-table name: `ready_to_skip`
* C entry function: `ready_to_skip_test`
* Rust test: `ready_to_skip`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6816-6818`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes option 3 but the helper ignores it and runs unrelated small stream transfers, so it does not exercise the prepare-to-send skip behavior covered by the C test.
* Phase 5A fix note: Add Rust support for the stream0 prepare-to-send test option, implement option 3 skip/no-data behavior, and run the C-equivalent q_and_r scenario with stream0_target 1000000 and matching timing parameters.
* Phase 5B analysis: Rust #[test] ready_to_skip is present, compiles under the test harness, and calls ready_to_send_test_one(3), matching the C entry. Missing PrepareToSend/provide_stream_data_buffer behavior is a Phase 5C runtime-library note, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = ready_to_send_test_one(3);
    return ret;
}
```

### Current Rust test body
```rust
fn ready_to_skip() {
    ready_to_send_test_one(3).expect("ready_to_skip");
}
```

## `picoquictest/tls_api_test.c:retire_cnxid_test`
* C test-table name: `retire_cnxid`
* C entry function: `retire_cnxid_test`
* Rust test: `retire_cnxid`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6896-6984`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs only the generic TLS API loss helper, so it never retires stashed connection IDs or checks that the peer refills and matches CID stashes.
* Phase 5A fix note: Implement the retire-CID test flow: handshake, synch to empty with PICOQUIC_NB_PATH_TARGET, verify local CID counts, obtain stashed client CIDs, queue RETIRE_CONNECTION_ID frames and remove them from stash, run until refill/backlogs empty, assert server CID count, and compare both remote stashes to peer local CID lists.
* Phase 5B analysis: Rust test is present, compiled by the Rust test harness, and expresses the same API-visible contract as C: handshake/sync, CID supply assertions, stashed CID retirement, refill wait predicates, backlog checks, server CID count, and peer stash comparisons. Any early failure from incomplete CID population/refill behavior is Phase 5C runtime implementation work, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* run a receive loop until no outstanding data */
    if (ret == 0) {
        ret = tls_api_synch_to_empty_loop(test_ctx, &simulated_time, 2048, PICOQUIC_NB_PATH_TARGET, 0);
    }

    if (ret == 0) {
        if (test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid < PICOQUIC_NB_PATH_TARGET) {
            DBG_PRINTF("Only %d cids created on client.\n", test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid);
            ret = -1;
        }
        else if (test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid < PICOQUIC_NB_PATH_TARGET) {
            DBG_PRINTF("Only %d cids created on server.\n", test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid);
            ret = -1;
        }
    }

    /* Delete several connection ID */
    for (int i = 2; ret == 0 && i < PICOQUIC_NB_PATH_TARGET; i++) {
        picoquic_remote_cnxid_t * stashed = picoquic_obtain_stashed_cnxid(test_ctx->cnx_client, 0);

        if (stashed == NULL) {
            DBG_PRINTF("Could not retrieve cnx ID #%d.\n", i-1);
            ret = -1;
        } else {
            ret = picoquic_queue_retire_connection_id_frame(test_ctx->cnx_client, 0, stashed->sequence);
            (void)picoquic_remove_stashed_cnxid(test_ctx->cnx_client, 0, stashed, NULL);
        }
    }

    /* run the loop again until no outstanding data */
    if (ret == 0) {
        uint64_t time_out = simulated_time + 8000000;
        int nb_rounds = 0;
        int success = 0;

        while (ret == 0 && simulated_time < time_out &&
            nb_rounds < 2048 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
            int was_active = 0; 

            ret = tls_api_one_sim_round(test_ctx, &simulated_time, time_out, &was_active);
            nb_rounds++;

            if (test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid >= PICOQUIC_NB_PATH_TARGET &&
                test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid >= PICOQUIC_NB_PATH_TARGET &&
                test_ctx->cnx_client->first_misc_frame == NULL &&
                test_cnxid_count_stash(test_ctx->cnx_client) >= (PICOQUIC_NB_PATH_TARGET - 1) &&
                picoquic_is_cnx_backlog_empty(test_ctx->cnx_client) &&
                picoquic_is_cnx_backlog_empty(test_ctx->cnx_server)) {
                success = 1;
                break;
            }
        }

        if (ret == 0 && success == 0) {
            DBG_PRINTF("Exit synch loop after %d rounds, backlog or not enough cids (%d & %d).\n",
                nb_rounds, test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid, test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid);
        }
    }

    /* Check */

    if (ret == 0) {
        if (test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid != PICOQUIC_NB_PATH_TARGET) {
            DBG_PRINTF("Found %d cids active on server instead of %d.\n", test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid, PICOQUIC_NB_PATH_TARGET);
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = transmit_cnxid_test_stash(test_ctx->cnx_client, test_ctx->cnx_server, "client");
    }

    if (ret == 0) {
        ret = transmit_cnxid_test_stash(test_ctx->cnx_server, test_ctx->cnx_client, "server");
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
fn retire_cnxid() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("retire_cnxid ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("retire_cnxid connection");
    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        NB_PATH_TARGET as i32,
        0,
    )
    .expect("retire_cnxid initial sync");

    let client_local_cid_count = first_local_cnxid_count(test_ctx.cnx_client());
    assert!(
        client_local_cid_count >= NB_PATH_TARGET,
        "Only {client_local_cid_count} cids created on client."
    );
    let server_local_cid_count = first_local_cnxid_count(test_ctx.cnx_server());
    assert!(
        server_local_cid_count >= NB_PATH_TARGET,
        "Only {server_local_cid_count} cids created on server."
    );

    for i in 2..NB_PATH_TARGET {
        let client = test_ctx.cnx_client();
        let (stash_index, cid_index) = client
            .obtain_stashed_connection_id(0)
            .unwrap_or_else(|| panic!("Could not retrieve cnx ID #{}.", i - 1));
        let sequence =
            client.remote_connection_id_stashes[stash_index].connection_ids[cid_index].sequence;
        client
            .queue_retire_connection_id_frame(0, sequence)
            .expect("queue RETIRE_CONNECTION_ID");
        let _ = client.remove_stashed_cnxid(0, cid_index, None);
    }

    let time_out = Instant::from_ticks(simulated_time.ticks() + 8_000_000);
    let mut nb_rounds = 0;
    let mut success = false;

    while simulated_time.ticks() < time_out.ticks()
        && nb_rounds < 2048
        && test_ctx
            .qclient
            .first_cnx_mut()
            .map(|c| c.connection_state != crate::State::Disconnected)
            .unwrap_or(false)
    {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            time_out,
            &mut was_active,
        )
        .expect("retire_cnxid refill round");
        nb_rounds += 1;

        if retire_cnxid_refill_ready(&mut test_ctx) {
            success = true;
            break;
        }
    }

    assert!(
        success,
        "Exit synch loop after {nb_rounds} rounds, backlog or not enough cids ({} & {}).",
        first_local_cnxid_count(test_ctx.cnx_client()),
        first_local_cnxid_count(test_ctx.cnx_server())
    );

    let server_local_cid_count = first_local_cnxid_count(test_ctx.cnx_server());
    assert_eq!(
        server_local_cid_count, NB_PATH_TARGET,
        "Found {server_local_cid_count} cids active on server instead of {NB_PATH_TARGET}."
    );

    {
        let (client, server) = (&test_ctx.qclient, &test_ctx.qserver);
        let client_cnx = client.connections.iter().next().expect("client connection");
        let server_cnx = server.connections.iter().next().expect("server connection");
        assert_cnxid_stash_matches_peer(client_cnx, server_cnx, "client");
        assert_cnxid_stash_matches_peer(server_cnx, client_cnx, "server");
    }
}
```

## `picoquictest/tls_api_test.c:stateless_reset_bad_test`
* C test-table name: `stateless_reset_bad`
* C entry function: `stateless_reset_bad_test`
* Rust test: `stateless_reset_bad`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7762-7796`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a normal handshake/close; it never constructs the bogus stateless-reset packet or verifies that the client remains ready after receiving it.
* Phase 5A fix note: Add the bogus reset injection after handshake: build the 256-byte packet with header 0x41, the client's local CID, and 0xcc filler, feed it to qclient as an incoming packet, then assert the client is still ready.
* Phase 5B analysis: Rust test is present, compiles under the Rust test harness, and matches the C API-level contract: initialize context, complete connection loop, build/inject the 256-byte bogus reset through qclient.incoming_packet, and assert the client remains ready. Any failure where the client leaves ready state is a Phase 5C implementation issue.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    uint8_t buffer[256];

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare the bogus reset */
    if (ret == 0) {
        size_t byte_index = 0;
        buffer[byte_index++] = 0x41;
        byte_index += picoquic_format_connection_id(&buffer[byte_index], sizeof(buffer) - byte_index, test_ctx->cnx_client->path[0]->first_tuple->p_local_cnxid->cnx_id);
        memset(buffer + byte_index, 0xcc, sizeof(buffer) - byte_index);
        
        /* Submit bogus request to client */
        ret = picoquic_incoming_packet(test_ctx->qclient, buffer, sizeof(buffer),
            (struct sockaddr*)(&test_ctx->server_addr),
            (struct sockaddr*)(&test_ctx->client_addr), 0, test_ctx->recv_ecn_client,
            simulated_time);
    }

    /* check that the client is still up */
    if (ret == 0 && !TEST_CLIENT_READY) {
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
fn stateless_reset_bad() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    let local_cid = test_ctx.cnx_client().local_cnxid();
    let mut buffer = [0u8; 256];
    let mut byte_index = 0usize;
    buffer[byte_index] = 0x41;
    byte_index += 1;
    byte_index += crate::utils::format_connection_id(&mut buffer[byte_index..], local_cid) as usize;
    buffer[byte_index..].fill(0xcc);

    let server_addr = test_ctx.server_addr;
    let client_addr = test_ctx.client_addr;
    test_ctx
        .qclient
        .incoming_packet(
            &mut buffer,
            &server_addr,
            &client_addr,
            0,
            0,
            simulated_time,
        )
        .expect("incoming bogus stateless reset");

    assert!(
        test_ctx.client_ready(),
        "client did not remain ready after bogus stateless reset"
    );
}
```

## `picoquictest/tls_api_test.c:tls_api_oneway_stream_test`
* C test-table name: `tls_api_oneway_stream`
* C entry function: `tls_api_oneway_stream_test`
* Rust test: `tls_api_oneway_stream`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8086-8091`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C uses test_scenario_oneway {stream_id 4, previous 0, q_len 257, r_len 0} and verifies completion within 75000 usec. Rust passes an empty scenario, and the Rust body verification currently only closes without stream or timing checks.
* Phase 5A fix note: Add/pass the one-way scenario and make scenario verification check stream completion and max_completion_microsec as the C helper does.
* Phase 5B analysis: Rust #[test] is present, compiles/runs under the test harness, and matches the C API-level contract: same one-way scenario descriptor {4,0,257,0}, zero option arguments, default context/version behavior, and 75000 usec bound. Prior StreamData/StreamFin callback failure is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_oneway, sizeof(test_scenario_oneway), 0, 0, 0, 0, 0, 75000, NULL, NULL);
}
```

### Current Rust test body
```rust
fn tls_api_oneway_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, TEST_SCENARIO_ONEWAY, 0, 0, 0, 0, 75_000)
        .expect("oneway_stream");
}
```
