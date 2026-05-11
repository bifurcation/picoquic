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

## `picoquictest/tls_api_test.c:cnx_ddos_unit_test`
* C test-table name: `cnx_ddos`
* C entry function: `cnx_ddos_unit_test`
* Rust test: `cnx_ddos`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1574-1576`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test passes the same numeric arguments, but the Rust cnx_ddos_test_loop ignores them and only performs a generic connection and close. It misses the C DDoS Initial-packet injection loop, half-open connection threshold check, and post-attack normal scenario verification.
* Phase 5A fix note: In 5B, implement the Rust cnx_ddos_test_loop behavior: create 1000 attack Initial packets, inject them at 1000 usec intervals, fail if half-open connections exceed the server threshold, then run the normal q2-and-r2 scenario to confirm the server still works.
* Phase 5B analysis: Rust test is present, compiles, and matches the C API-level contract; any zero-length Initial/runtime failure is Phase 5C implementation behavior.
* Phase 5B fix note: 

### C test body
```c
{
    return cnx_ddos_test_loop(1000, 1000, NULL);
}
```

### Current Rust test body
```rust
fn cnx_ddos() {
    cnx_ddos_test_loop(1000, 1000).expect("cnx_ddos");
}
```

## `picoquictest/tls_api_test.c:ddos_amplification_test`
* C test-table name: `ddos_amplification`
* C entry function: `ddos_amplification_test`
* Rust test: `ddos_amplification`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1938-1940`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Although the wrapper passes (0,0), the Rust ddos_amplification_test_one is a placeholder-like normal handshake and close. It does not simulate a client that sends one Initial then disappears, count server bytes, or enforce the 3x amplification limit.
* Phase 5A fix note: Translate the DDoS amplification helper behavior: prepare/send only the first client packet, drive server-only sends until disconnect/inactive, count client/server bytes, and assert server output stays <= 3x client input.
* Phase 5B analysis: Rust #[test] matches the C entry call ddos_amplification_test_one(0, 0), and the shared helper already expresses the C API-level DDoS flow and 3x amplification assertion. Phase 5C note: any failure from the INTEROP_VERSION_LATEST Initial being dropped by the current parser is runtime/library behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return ddos_amplification_test_one(0, 0);
}
```

### Current Rust test body
```rust
fn ddos_amplification() {
    ddos_amplification_test_one(0, 0).expect("ddos_amplification");
}
```

## `picoquictest/tls_api_test.c:false_migration_test`
* C test-table name: `false_migration`
* C entry function: `false_migration_test`
* Rust test: `false_migration`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2616-2648`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses a generic TLS handshake/close helper and performs none of the C false-migration matrix: no spoofed source-address packet injection, no initial/handshake/application context coverage, and no target_client loop.
* Phase 5A fix note: Add a Rust false_migration_test_scenario/injection helper and run both target_client values over initial, handshake, and application seq 0..3 using the q2_and_r2 scenario.
* Phase 5B analysis: Rust test is present as #[test], cargo-checks, and matches the C API-level matrix and scenario helper contract. Runtime disconnect before spoofed Initial injection is a Phase 5C implementation behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    int target_client;

    for (target_client = 1; ret == 0 && target_client >= 0; target_client--) {
        ret = false_migration_test_scenario(test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2), target_client, picoquic_packet_context_initial, 0);
        
        if (ret == 0) {
            ret = false_migration_test_scenario(test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2), target_client, picoquic_packet_context_handshake, 0);
        }

        for (uint64_t seq = 0; ret == 0 && seq < 4; seq++) {
            ret = false_migration_test_scenario(test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2), target_client, picoquic_packet_context_application, seq);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn false_migration() {
    for target_client in [true, false] {
        false_migration_test_scenario(
            TEST_SCENARIO_Q2_AND_R2,
            target_client,
            PacketContext::Initial,
            0,
        )
        .unwrap_or_else(|e| panic!("false_migration initial target_client={target_client}: {e:?}"));

        false_migration_test_scenario(
            TEST_SCENARIO_Q2_AND_R2,
            target_client,
            PacketContext::Handshake,
            0,
        )
        .unwrap_or_else(|e| {
            panic!("false_migration handshake target_client={target_client}: {e:?}")
        });

        for seq in 0..4 {
            false_migration_test_scenario(
                TEST_SCENARIO_Q2_AND_R2,
                target_client,
                PacketContext::Application,
                seq,
            )
            .unwrap_or_else(|e| {
                panic!("false_migration application target_client={target_client} seq={seq}: {e:?}")
            });
        }
    }
}
```

## `picoquictest/tls_api_test.c:implicit_ack_test`
* C test-table name: `implicit_ack`
* C entry function: `implicit_ack_test`
* Rust test: `implicit_ack`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2999-3026`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the handshake loop; C also waits for ready and asserts Initial and Handshake pending/retransmitted queues are empty on both client and server.
* Phase 5A fix note: After tls_api_connection_loop, call wait_client_connection_ready and assert both endpoints' PacketContext::Initial and PacketContext::Handshake pending and retransmitted maps are empty.
* Phase 5B analysis: Rust test matches the C API-level contract: initializes with proposed_version 0, runs the connection loop, waits for client readiness, requires a server connection, and asserts Initial/Handshake pending and retransmitted queues are empty on both endpoints. Any early runtime failure from missing server acceptance or handshake behavior is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    picoquic_packet_context_enum pc[2] = { picoquic_packet_context_initial, picoquic_packet_context_handshake };

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    for (int i = 0; ret == 0 && i < 2; i++) {
        if (test_ctx->cnx_client->pkt_ctx[pc[i]].pending_first != NULL) {
            DBG_PRINTF("Retransmit queue type %d not empty on client", pc[i]);
            ret = -1;
        }
        else if (test_ctx->cnx_server->pkt_ctx[pc[i]].pending_first != NULL) {
            DBG_PRINTF("Retransmit queue type %d not empty on server", pc[i]);
            ret = -1;
        }
        else if (test_ctx->cnx_client->pkt_ctx[pc[i]].retransmitted_oldest != NULL) {
            DBG_PRINTF("Retransmitted queue type %d not empty on client", pc[i]);
            ret = -1;
        }
        else if (test_ctx->cnx_server->pkt_ctx[pc[i]].pending_first != NULL) {
            DBG_PRINTF("Retransmitted queue type %d not empty on server", pc[i]);
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
fn implicit_ack() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("implicit_ack");
    wait_client_connection_ready(&mut ctx, &mut t).expect("implicit_ack_ready");
    assert!(ctx.has_cnx_server(), "server connection not accepted");

    for pc in [PacketContext::Initial, PacketContext::Handshake] {
        let pc_index = pc as usize;
        assert!(
            ctx.cnx_client().pkt_ctx[pc_index].pending.is_empty(),
            "pending queue type {pc:?} not empty on client"
        );
        assert!(
            ctx.cnx_server().pkt_ctx[pc_index].pending.is_empty(),
            "pending queue type {pc:?} not empty on server"
        );
        assert!(
            ctx.cnx_client().pkt_ctx[pc_index].retransmitted.is_empty(),
            "retransmitted queue type {pc:?} not empty on client"
        );
        assert!(
            ctx.cnx_server().pkt_ctx[pc_index].retransmitted.is_empty(),
            "retransmitted queue type {pc:?} not empty on server"
        );
    }
}
```

## `picoquictest/tls_api_test.c:keylog_test`
* C test-table name: `keylog_test`
* C entry function: `keylog_test`
* Rust test: `keylog_test`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3545-3605`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is a generic handshake/close and does not exercise key logging. It misses file reset, explicit SSL keylog enablement, keylog file configuration, enabled-state assertions, the specific q_and_r scenario, and final file-size checks.
* Phase 5A fix note: Add a real keylog test: reset client/server keylog files, initialize with the C initial CID, enable key logging on both contexts, set both keylog files, assert enabled state, run the q_and_r scenario with matching parameters, then verify both files are at least 128 bytes.
* Phase 5B analysis: Rust keylog_test already matches the C API-level contract: reset both keylog files, init V1 with TEST_SNI/TEST_ALPN and explicit CID, enable/query SSL keylogging, set both keylog paths, run q_and_r with stream0_target=1000000 and C timing arguments, then assert both files are >=128 bytes. Runtime CannotSetActiveStream/key-emission failures are Phase 5C implementation issues, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x55, 0x17, 0xe9, 0x10, 0x90, 0x0, 0x0, 0x0}, 8 };
    int ret;

    /* Ensure that the log files are empty */
    keylog_reset_file(TEST_KEYLOG_FILE_SERVER);
    keylog_reset_file(TEST_KEYLOG_FILE_CLIENT);

    /* Create the contexts */
    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);

    if (ret == 0) {
        /* program key logging. */
        picoquic_enable_sslkeylog(test_ctx->qserver, 1);
        picoquic_enable_sslkeylog(test_ctx->qclient, 1);
        picoquic_set_key_log_file(test_ctx->qserver, TEST_KEYLOG_FILE_SERVER);
        picoquic_set_key_log_file(test_ctx->qclient, TEST_KEYLOG_FILE_CLIENT);
        if (!picoquic_is_sslkeylog_enabled(test_ctx->qserver) ||
            !picoquic_is_sslkeylog_enabled(test_ctx->qclient)) {
            ret = -1;
        }
    }
    if (ret == 0) {
        /* Execute a small scenario to force complete exchange of keys */
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 1000000, 0, 0, 20000,
            1200000);

        if (ret != 0)
        {
            DBG_PRINTF("Scenario body returns error %d\n", ret);
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    if (ret == 0) {
        /* Verify that data was properly written in log files. */
        if (keylog_file_size(TEST_KEYLOG_FILE_SERVER) < 128 ||
            keylog_file_size(TEST_KEYLOG_FILE_CLIENT) < 128) {
            ret = -1;
        }
    }
    return ret;
}
```

### Current Rust test body
```rust
fn keylog_test() {
    const TEST_KEYLOG_FILE_CLIENT: &str = "test_keylog_client.txt";
    const TEST_KEYLOG_FILE_SERVER: &str = "test_keylog_server.txt";
    const TEST_SCENARIO_Q_AND_R: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    std::fs::File::create(TEST_KEYLOG_FILE_SERVER).expect("reset server keylog file");
    std::fs::File::create(TEST_KEYLOG_FILE_CLIENT).expect("reset client keylog file");

    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x55, 0x17, 0xe9, 0x10, 0x90, 0x0, 0x0, 0x0])
            .expect("initial CID");
    let mut test_ctx =
        tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid)).expect("ctx");

    test_ctx.qserver.set_sslkeylog_enabled(true);
    test_ctx.qclient.set_sslkeylog_enabled(true);
    test_ctx
        .qserver
        .set_key_log_file(Some(TEST_KEYLOG_FILE_SERVER));
    test_ctx
        .qclient
        .set_key_log_file(Some(TEST_KEYLOG_FILE_CLIENT));
    assert!(test_ctx.qserver.is_sslkeylog_enabled());
    assert!(test_ctx.qclient.is_sslkeylog_enabled());

    tls_api_one_scenario_body_ex(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        1_000_000,
        0,
        0,
        20_000,
        1_200_000,
        &[],
    )
    .expect("keylog q_and_r scenario");

    drop(test_ctx);

    let server_size = std::fs::metadata(TEST_KEYLOG_FILE_SERVER)
        .expect("server keylog metadata")
        .len();
    let client_size = std::fs::metadata(TEST_KEYLOG_FILE_CLIENT)
        .expect("client keylog metadata")
        .len();
    assert!(
        server_size >= 128,
        "server keylog should contain TLS secrets, got {server_size} bytes"
    );
    assert!(
        client_size >= 128,
        "client keylog should contain TLS secrets, got {client_size} bytes"
    );
}
```
