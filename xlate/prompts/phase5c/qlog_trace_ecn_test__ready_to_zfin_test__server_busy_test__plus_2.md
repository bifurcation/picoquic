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

## `picoquictest/tls_api_test.c:qlog_trace_ecn_test`
* C test-table name: `qlog_trace_ecn`
* C entry function: `qlog_trace_ecn_test`
* Rust test: `qlog_trace_ecn`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6577-6579`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the same top-level arguments, but the Rust helper is only a lightweight connection scenario and does not configure qlog/CID/reset seeds, inject the bad packet, or compare the ECN qlog reference file that the C test checks.
* Phase 5A fix note: Expand the Rust qlog trace helper/test to generate the ECN qlog under deterministic settings, run the C q2_and_r2-style scenario with ECN 0x02, inject the bad packet, and compare against the ECN reference output.
* Phase 5B analysis: Rust #[test] qlog_trace_ecn calls qlog_trace_test_one(0x02, false), matching the C wrapper qlog_trace_test_one(0x02, 0). The shared helper expresses the C API-level qlog/ECN contract; missing qlog output is a Phase 5C library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return qlog_trace_test_one(0x02, 0);
}
```

### Current Rust test body
```rust
fn qlog_trace_ecn() {
    qlog_trace_test_one(0x02, false).expect("qlog_trace_ecn");
}
```

## `picoquictest/tls_api_test.c:ready_to_zfin_test`
* C test-table name: `ready_to_zfin`
* C entry function: `ready_to_zfin_test`
* Rust test: `ready_to_zfin`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6832-6834`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes option 2, but the helper ignores the option and does not exercise the C direct stream-0 prepare-to-send FIN behavior. It also uses a different small stream scenario and does not pass/use the C stream0_target path.
* Phase 5A fix note: Add stream0_test_option/prepare-to-send behavior for option 2, run the q_and_r scenario with stream0_target=1000000, and verify the zero-length FIN-after-data path completes like the C helper.
* Phase 5B analysis: Rust #[test] ready_to_zfin is present and calls ready_to_send_test_one(2), matching the C entry's API-level call; prior prepare-to-send/Protocol(1060) failure is a Phase 5C implementation note, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = ready_to_send_test_one(2);
    return ret;
}
```

### Current Rust test body
```rust
fn ready_to_zfin() {
    ready_to_send_test_one(2).expect("ready_to_zfin");
}
```

## `picoquictest/tls_api_test.c:server_busy_test`
* C test-table name: `server_busy`
* C entry function: `server_busy_test`
* Rust test: `server_busy`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7252-7316`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs a normal TLS handshake; it never sets qserver.server_busy, never checks client SERVER_BUSY remote error/disconnect timing, and never retries after clearing busy state.
* Phase 5A fix note: Implement the two-phase server-busy test: busy rejection with disconnected client/server and SERVER_BUSY error within 500000 us, then clear busy, recreate/start a client connection, complete handshake, and close.
* Phase 5B analysis: Rust #[test] matches the C API-level sequence and assertions; the SERVER_BUSY remote_error mismatch is a Phase 5C implementation behavior failure, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        test_ctx->qserver->server_busy = 1;
        (void) tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);

        if (test_ctx->cnx_server != NULL &&
            test_ctx->cnx_server->cnx_state != picoquic_state_disconnected) {
            DBG_PRINTF("Server state: %d, local error: %" PRIx64, test_ctx->cnx_server->cnx_state, test_ctx->cnx_server->local_error);
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected ||
            test_ctx->cnx_client->remote_error != PICOQUIC_TRANSPORT_SERVER_BUSY) {
            DBG_PRINTF("Client state: %d, remote error: %" PRIx64, test_ctx->cnx_client->cnx_state, test_ctx->cnx_client->remote_error);
            ret = -1;
        }
        else if (simulated_time > 500000ull) {
            DBG_PRINTF("Simulated time: %" PRIu64, (unsigned long long)simulated_time);
            ret = -1;
        }
    }

    if (ret == 0) {
        test_ctx->qserver->server_busy = 0;

        if (test_ctx->cnx_server != NULL) {
            picoquic_delete_cnx(test_ctx->cnx_server);
            test_ctx->cnx_server = NULL;
        }
        if (test_ctx->cnx_client != NULL) {
            picoquic_delete_cnx(test_ctx->cnx_client);
            test_ctx->cnx_client = NULL;
        }

        /* Create a new client connection */
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient,
            picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*)&test_ctx->server_addr, simulated_time,
            0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        } else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
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
fn server_busy() {
    let mut loss_mask = 0u64;
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("server_busy ctx");

    test_ctx.qserver.server_busy = true;
    let _ = tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time);

    if let Some(server) = test_ctx.qserver.first_cnx_mut() {
        assert_eq!(
            server.state(),
            crate::State::Disconnected,
            "server state {:?}, local error {:x}",
            server.state(),
            server.local_error()
        );
    }

    let client = test_ctx.cnx_client();
    assert_eq!(
        client.state(),
        crate::State::Disconnected,
        "client state {:?}, remote error {:x}",
        client.state(),
        client.remote_error()
    );
    assert_eq!(
        client.remote_error(),
        crate::TransportError::ServerBusy as u64,
        "client remote error {:x}",
        client.remote_error()
    );
    assert!(
        simulated_time.ticks() <= 500_000,
        "simulated time {}",
        simulated_time.ticks()
    );

    test_ctx.qserver.server_busy = false;
    delete_tls_api_test_connections(&mut test_ctx.qserver);
    delete_tls_api_test_connections(&mut test_ctx.qclient);

    test_ctx
        .qclient
        .create_connection(
            ConnectionId::with_size(0).expect("null initial CID"),
            ConnectionId::with_size(0).expect("null remote CID"),
            Some(&test_ctx.server_addr),
            simulated_time,
            0,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .expect("new client connection")
        .start_client()
        .expect("start new client connection");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("server_busy retry connection");
    assert!(test_ctx.client_ready(), "client did not reach ready state");
    assert!(test_ctx.server_ready(), "server did not reach ready state");

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("server_busy close");
}
```

## `picoquictest/tls_api_test.c:stateless_reset_handshake_test`
* C test-table name: `stateless_reset_handshake`
* C entry function: `stateless_reset_handshake_test`
* Rust test: `stateless_reset_handshake`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7858-7932`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only performs a normal TLS API handshake. It omits the bogus long-header packet construction, mutation of the client CID byte, picoquic_incoming_packet injection into the server, and checks that the server remains up with no pending stateless packet.
* Phase 5A fix note: Translate the C test body directly: establish the connection, build and inject the bogus long-header packet using the server path CIDs, then assert server state is not past ready and no stateless packet is queued.
* Phase 5B analysis: Rust test already expresses the C API-level contract and compiles/runs under the Rust test harness; any early failure from incomplete handshake/server-connection behavior is a Phase 5C library issue, not a Phase 5B block.
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
        buffer[byte_index++] = 0xff;
        buffer[byte_index++] = test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len;
        buffer[byte_index++] = test_ctx->cnx_server->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len;
        /* Copy the client ID */
        byte_index += picoquic_format_connection_id(&buffer[byte_index], sizeof(buffer) - byte_index, test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id);
        /* Change one byte of the client ID */
        if (byte_index > 5) {
            buffer[5] ^= 0xff;
        }
        else {
            buffer[1] ^= 0xff;
        }
        /* Copy the server ID */
        byte_index += picoquic_format_connection_id(&buffer[byte_index], sizeof(buffer) - byte_index, test_ctx->cnx_server->path[0]->first_tuple->p_remote_cnxid->cnx_id);
        /* rest of packet is null */
        memset(buffer + byte_index, 0xcc, sizeof(buffer) - byte_index);

        /* Submit bogus request to server */
        ret = picoquic_incoming_packet(test_ctx->qserver, buffer, sizeof(buffer),
            (struct sockaddr*)(&test_ctx->client_addr),
            (struct sockaddr*)(&test_ctx->server_addr), 0, test_ctx->recv_ecn_server,
            simulated_time);
    }

    /* check that the server is still up */
    if (ret == 0 && test_ctx->cnx_server != NULL && test_ctx->cnx_server->cnx_state > picoquic_state_ready) {
        ret = -1;
    }
    /* Check that no stateless packet is queued */
    if (ret == 0 && test_ctx->qserver->pending_stateless_packet != NULL) {
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
fn stateless_reset_handshake() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    let (local_cid, remote_cid) = {
        let server = test_ctx.cnx_server();
        let path = server.paths.first().expect("server path");
        let tuple = path.tuples.first().expect("server tuple");
        let local_cid = tuple
            .local_connection_id
            .and_then(|token| server.local_connection_ids.get(token))
            .map(|cid| cid.connection_id)
            .expect("server local connection id");
        let remote_index = tuple.remote_connection_id_index.unwrap_or(0);
        let remote_cid = server
            .remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == path.unique_path_id)
            .and_then(|stash| stash.connection_ids.get(remote_index))
            .map(|cid| cid.connection_id)
            .expect("server remote connection id");
        (local_cid, remote_cid)
    };

    let mut buffer = [0u8; 256];
    let mut byte_index = 0usize;
    buffer[byte_index] = 0xff;
    byte_index += 1;
    buffer[byte_index] = local_cid.len() as u8;
    byte_index += 1;
    buffer[byte_index] = remote_cid.len() as u8;
    byte_index += 1;
    byte_index += crate::utils::format_connection_id(&mut buffer[byte_index..], local_cid) as usize;
    if byte_index > 5 {
        buffer[5] ^= 0xff;
    } else {
        buffer[1] ^= 0xff;
    }
    byte_index +=
        crate::utils::format_connection_id(&mut buffer[byte_index..], remote_cid) as usize;
    buffer[byte_index..].fill(0xcc);

    let client_addr = test_ctx.client_addr;
    let server_addr = test_ctx.server_addr;
    test_ctx
        .qserver
        .incoming_packet(
            &mut buffer,
            &client_addr,
            &server_addr,
            0,
            0,
            simulated_time,
        )
        .expect("incoming bogus long-header packet");

    let server_state = test_ctx.qserver.first_cnx_mut().map(|cnx| cnx.state());
    assert!(
        server_state.is_some(),
        "server connection disappeared after bogus long-header packet"
    );
    let server_state = server_state.expect("server connection state");
    assert!(
        server_state <= State::Ready,
        "server connection advanced past ready: {server_state:?}"
    );
    assert!(
        test_ctx.qserver.pending_stateless_packets.is_empty(),
        "server queued a stateless packet for bogus long-header packet"
    );
}
```

## `picoquictest/tls_api_test.c:tls_api_q_and_r_stream_test`
* C test-table name: `tls_api_q_and_r_stream`
* C entry function: `tls_api_q_and_r_stream_test`
* Rust test: `tls_api_q_and_r_stream`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8117-8122`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs the generic scenario helper with an empty scenario, while C uses test_scenario_q_and_r with stream 4, q_len 257, r_len 2000, and the Rust helper's verify path only closes the connection.
* Phase 5A fix note: Add/pass the q_and_r TestApiStreamDesc and make the helper verify stream completion and the 75000 us completion target.
* Phase 5B analysis: Rust test already expresses the C API-level contract: it is a #[test], initializes the TLS API context with proposed version 0, uses TEST_SCENARIO_Q_AND_R {stream 4, q_len 257, r_len 2000}, passes zero stream0/loss/max_data/queue_delay, and uses the 75_000 us completion target. Any early runtime failure from missing q/r stream completion is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: No Rust test changes needed; appended command-log entry only.

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 0, 75000, NULL, NULL);
}
```

### Current Rust test body
```rust
fn tls_api_q_and_r_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, TEST_SCENARIO_Q_AND_R, 0, 0, 0, 0, 75_000)
        .expect("q_and_r_stream");
}
```
