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

## `picoquictest/tls_api_test.c:cnxid_renewal_test`
* C test-table name: `cnxid_renewal`
* C entry function: `cnxid_renewal_test`
* Rust test: `cnxid_renewal`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1583-1659`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the generic TLS handshake/close helper; it never renews CID 0, sends the q_and_r scenario, waits 7 seconds, or checks demotion and CID rotation invariants.
* Phase 5A fix note: Implement the C sequence: handshake, sync empty, renew_connection_id(0), save target/previous CIDs, run q_and_r data, 7s sim loop, assert path[0] not demoted and remote/local CIDs match expectations.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and mirrors the C API sequence and API-visible assertions. The known local-CID rotation failure is a Phase 5C implementation issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t next_time = 0;
    uint64_t loss_mask = 0;
    picoquic_connection_id_t target_id = picoquic_null_connection_id;
    picoquic_connection_id_t previous_local_id = picoquic_null_connection_id;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* run a receive loop until no outstanding data */
    if (ret == 0) {
        ret = tls_api_synch_to_empty_loop(test_ctx, &simulated_time, 2048, PICOQUIC_NB_PATH_TARGET, 1);
    }

    /* Renew the connection ID */
    if (ret == 0) {
        ret = picoquic_renew_connection_id(test_ctx->cnx_client, 0);
        if (ret == 0) {
            target_id = test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id;
            previous_local_id = test_ctx->cnx_client->path[0]->first_tuple->p_local_cnxid->cnx_id;
        }
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_q_and_r, sizeof(test_scenario_q_and_r));
    }

    /* Perform a data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* Add a time loop of 7 seconds to give some time for the probes to be repeated,
     * and to ensure that the demotion timers expire. */
    next_time = simulated_time + 7000000;
    loss_mask = 0;
    while (ret == 0 && simulated_time < next_time && TEST_CLIENT_READY
        && TEST_SERVER_READY) {
        int was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, next_time, &was_active);
    }

    /* verify that path[0] was not demoted */
    if (ret == 0) {
        if (test_ctx->cnx_client->path[0]->path_is_demoted) {
            DBG_PRINTF("%s", "The default client path is demoted");
            ret = -1;
        }
        if (test_ctx->cnx_server != NULL && test_ctx->cnx_server->path[0]->path_is_demoted) {
            DBG_PRINTF("%s", "The default server path is demoted");
            ret = -1;
        }
    }

    /* Verify that the connection ID are what we expect */
    if (ret == 0) {
        if (picoquic_compare_connection_id(&test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id, &target_id) != 0) {
            DBG_PRINTF("%s", "The remote CNX ID migrated from the selected value");
            ret = -1;
        }
        else if (picoquic_compare_connection_id(&test_ctx->cnx_client->path[0]->first_tuple->p_local_cnxid->cnx_id, &previous_local_id) == 0) {
            DBG_PRINTF("%s", "The local CNX ID did not change to a new value");
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
fn cnxid_renewal() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        NB_PATH_TARGET as i32,
        1,
    )
    .expect("synch to empty");

    test_ctx
        .cnx_client()
        .renew_connection_id(0)
        .expect("renew connection ID");
    let target_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("target remote CID after renewal");
    let previous_local_id =
        first_path_local_cid(test_ctx.cnx_client()).expect("local CID after renewal");

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_Q_AND_R)
        .expect("init q_and_r scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");

    let next_time = Instant::from_ticks(simulated_time.ticks() + 7_000_000);
    while simulated_time.ticks() < next_time.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            next_time,
            &mut was_active,
        )
        .expect("renewal grace loop");
    }

    let client_path_demoted = test_ctx
        .cnx_client()
        .paths
        .first()
        .map(|path| path.path_is_demoted)
        .unwrap_or(true);
    assert!(!client_path_demoted, "default client path is demoted");

    if test_ctx.has_cnx_server() {
        let server_path_demoted = test_ctx
            .cnx_server()
            .paths
            .first()
            .map(|path| path.path_is_demoted)
            .unwrap_or(true);
        assert!(!server_path_demoted, "default server path is demoted");
    }

    let final_remote_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID after renewal loop");
    assert_eq!(
        final_remote_id, target_id,
        "remote CNX ID migrated from the selected value"
    );

    let final_local_id =
        first_path_local_cid(test_ctx.cnx_client()).expect("local CID after renewal loop");
    assert_ne!(
        final_local_id, previous_local_id,
        "local CNX ID did not change to a new value"
    );
}
```

## `picoquictest/tls_api_test.c:ddos_amplification_0rtt_test`
* C test-table name: `ddos_amplification_0rtt`
* C entry function: `ddos_amplification_0rtt_test`
* Rust test: `ddos_amplification_0rtt`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1946-1948`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test calls ddos_amplification_test_one(1,0), but the Rust helper ignores the mode and only performs a normal handshake and close. It does not obtain a 0-RTT ticket/token, send one client packet then disappear, count client/server bytes, or enforce the 3x amplification limit.
* Phase 5A fix note: Implement the 0-RTT ddos amplification path: first connection for ticket/token, recreate 0-RTT client connection, deliver only the first client packet to the server, prepare server packets until termination, and assert server bytes <= 3 * client bytes.
* Phase 5B analysis: Rust is a `#[test]` and calls `ddos_amplification_test_one(1, 0)`, matching the C entry's API-level contract. Runtime 0-RTT/ticket failures are Phase 5C implementation issues, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return ddos_amplification_test_one(1, 0);
}
```

### Current Rust test body
```rust
fn ddos_amplification_0rtt() {
    ddos_amplification_test_one(1, 0).expect("ddos_amplification_0rtt");
}
```

## `picoquictest/tls_api_test.c:get_tls_errors_test`
* C test-table name: `get_tls_errors`
* C entry function: `get_tls_errors_test`
* Rust test: `get_tls_errors`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2707-2793`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is just a generic successful TLS handshake; C checks specific TLS API error surfaces for invalid ECB/cipher/key-exchange IDs, unloaded TLS API behavior, and bad key/cert files.
* Phase 5A fix note: Replace the smoke test with direct assertions against the translated TLS API: invalid ECB name returns None, invalid cipher suite/key exchange are rejected, AES_128 lookup succeeds as in C, unload makes connection creation fail, bad key/cert filenames fail, then reinitialize TLS API.
* Phase 5B analysis: Rust test is present, compiles as a harness test, and matches the C API-visible contract. The TLS-unload connection-creation failure is a Phase 5C implementation/runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    uint8_t data[128];
    char const* invalid_stuff = "no_such_stuff_nada_niente";
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t simulated_time = 0;
    picoquic_connection_id_t initial_cid = { {0x9e, 0x71, 0x5e, 0, 0, 0, 0, 0}, 8 };
    int invalid_id = 0xFFFE8808;

    ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid, 8, 0, 0, 0);

    memset(data, 0xaa, sizeof(data));
    if (ret == 0 && picoquic_ecb_create_by_name(0, data, invalid_stuff) != NULL) {
        ret = -1;
    }
    if (ret == 0 && picoquic_get_cipher_suite_by_id_v(invalid_id, 0) != NULL) {
        ret = -1;
    }
    if (ret == 0 && picoquic_set_cipher_suite(test_ctx->qserver, invalid_id) == 0) {
        ret = -1;
    }
    if (ret == 0 && picoquic_set_key_exchange(test_ctx->qserver, invalid_id) == 0) {
        ret = -1;
    }
    if (ret == 0 &&
        picoquic_get_cipher_suite_by_id_v(PICOQUIC_AES_128_GCM_SHA256, 1) == NULL &&
        picoquic_get_cipher_suite_by_id_v(PICOQUIC_AES_128_GCM_SHA256, 0) == NULL) {
        ret = -1;
    }
    if (ret == 0 &&
        picoquic_get_cipher_suite_by_id_v(invalid_id, 1) != NULL) {
        ret = -1;
    }
    if (ret == 0) {

    }
    if (ret == 0) {
        /* Unload the TLS API to force internal errors. */
        picoquic_cnx_t* cnx;
        picoquic_tls_api_unload();

        cnx = picoquic_create_cnx(test_ctx->qclient, picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*)&test_ctx->server_addr, simulated_time, PICOQUIC_INTERNAL_TEST_VERSION_1,
            PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 0);
        if (cnx != NULL) {
            ret = -1;
            picoquic_delete_cnx(cnx);
        }
        if (ret == 0) {
            if (picoquic_set_private_key_from_file(test_ctx->qclient, "some bad file name.not") == 0) {
                ret = -1;
            }
        }
        if (ret == 0) {
            size_t count = 0;
            ptls_iovec_t* certs = picoquic_get_certs_from_file("some bad file name.not", &count);
            if (certs != NULL) {
                ret = -1;
                for (size_t i = 0; i < count; i++) {
                    free(certs[i].base);
                }
                free(certs);
            }
        }

        ptls_iovec_t* picoquic_get_certs_from_file(char const* file_name, size_t * count);
        /* Reinit the TLS API */
        picoquic_tls_api_init();
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
fn get_tls_errors() {
    struct TlsApiResetGuard;

    impl Drop for TlsApiResetGuard {
        fn drop(&mut self) {
            tls_api_init();
        }
    }

    let _guard = TlsApiResetGuard;
    tls_api_init();

    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x9e, 0x71, 0x5e, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut test_ctx =
        tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid)).expect("ctx");

    let invalid_stuff = "no_such_stuff_nada_niente";
    let invalid_id = 0xFFFE_8808u32 as i32;
    let ecb_key = [0xaau8; 16];

    assert!(
        ecb_create_by_name(false, &ecb_key, invalid_stuff).is_none(),
        "invalid ECB cipher name must be rejected"
    );
    assert!(
        picoquic_get_cipher_suite_by_id_v(invalid_id, false).is_none(),
        "invalid high-memory cipher suite ID must not resolve"
    );
    assert!(
        test_ctx
            .qserver
            .set_cipher_suite(invalid_id as u16)
            .is_err(),
        "invalid cipher suite ID must be rejected"
    );
    assert!(
        test_ctx
            .qserver
            .set_key_exchange(invalid_id as u16)
            .is_err(),
        "invalid key exchange ID must be rejected"
    );
    assert!(
        picoquic_get_cipher_suite_by_id_v(i32::from(AES_128_GCM_SHA256), true).is_some()
            || picoquic_get_cipher_suite_by_id_v(i32::from(AES_128_GCM_SHA256), false).is_some(),
        "AES_128_GCM_SHA256 must resolve in at least one memory mode"
    );
    assert!(
        picoquic_get_cipher_suite_by_id_v(invalid_id, true).is_none(),
        "invalid low-memory cipher suite ID must not resolve"
    );

    tls_api_unload();

    let cnx_created = test_ctx
        .qclient
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            Some(&test_ctx.server_addr),
            simulated_time,
            V1,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .is_some();
    assert!(
        !cnx_created,
        "connection creation must fail after TLS API unload"
    );
    assert!(
        test_ctx
            .qclient
            .set_private_key_from_file("some bad file name.not")
            .is_err(),
        "bad private-key filename must fail"
    );
    assert!(
        get_certs_from_file("some bad file name.not").is_none(),
        "bad certificate filename must fail"
    );

    tls_api_init();
}
```

## `picoquictest/tls_api_test.c:initial_close_test`
* C test-table name: `initial_close`
* C entry function: `initial_close_test`
* Rust test: `initial_close`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3096-3098`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the generic successful TLS API handshake/close helper. It never sends only the Initial packet, forces client handshake_failure with local error 0xDEAD, or checks server/client disconnected states, server remote_error, and the 50000us bound.
* Phase 5A fix note: Add a dedicated Rust initial_close test path that mirrors the C early client close sequence and asserts server remote_error == 0xDEAD, both endpoints disconnected, and simulated time <= 50000.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and its helper mirrors the C Initial-close API contract. The known early server deletion and remote_error decoding failures are Phase 5C runtime behavior issues, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    int was_active = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        /* Send the initial packet, but no more than that */
        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

        if (ret == 0) {
            test_ctx->cnx_client->cnx_state = picoquic_state_handshake_failure;
            test_ctx->cnx_client->local_error = 0xDEAD;
            picoquic_reinsert_by_wake_time(test_ctx->qclient, test_ctx->cnx_client, simulated_time);
        }
    }

    if (ret == 0) {
        for (int i = 0; i < 128; i++) {
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            if (test_ctx->cnx_server != NULL) {
                break;
            }
        }
        if (ret == 0) {
            ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
        }

        if (ret == 0) {
            if (test_ctx->cnx_server == NULL) {
                DBG_PRINTF("%s", "Server connection deleted, cannot verify error code.\n");
                ret = -1;
            }
            else if (test_ctx->cnx_server->cnx_state != picoquic_state_disconnected ||
                test_ctx->cnx_server->remote_error != 0xDEAD) {
                DBG_PRINTF("Server state: %d, remote error: %x\n", test_ctx->cnx_server->cnx_state, test_ctx->cnx_server->remote_error);
                ret = -1;
            }
            else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
                DBG_PRINTF("Client state: %d, local error: %x", test_ctx->cnx_client->cnx_state, test_ctx->cnx_client->local_error);
                ret = -1;
            }
            else if (simulated_time > 50000ull) {
                DBG_PRINTF("Simulated time: %llu", (unsigned long long)simulated_time);
                ret = -1;
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
fn initial_close() {
    initial_close_test_one().expect("initial_close");
}
```

## `picoquictest/tls_api_test.c:large_client_hello_test`
* C test-table name: `large_client_hello`
* C entry function: `large_client_hello_test`
* Rust test: `large_client_hello`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3612-3647`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls the retry-cookie helper, which ignores the large-client-hello flag, does not run the q_and_r data scenario, and does not assert zero client/server retransmissions.
* Phase 5A fix note: Implement this as the standalone C test: init V1 context with SNI/ALPN, set client test_large_chello, run q_and_r with 250000 completion target, then assert server exists and both retransmission counters are zero.
* Phase 5B analysis: Rust test already expresses the C API-level contract: V1 TLS API context, client test_large_chello flag, q_and_r scenario with 250000 completion target, server-exists assertion, and zero client/server retransmission assertions. Any Error::Generic or missing large ClientHello behavior is Phase 5C implementation work, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Set the test large hello flag in the client connection
     */
    if (ret == 0) {
        test_ctx->cnx_client->test_large_chello = 1;

        /* Run a basic test scenario
         */

        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 250000);
    }

    /* Verify that there is no spurious retransmission */
    if (ret == 0) {
        if (test_ctx->cnx_server == NULL || test_ctx->cnx_server->nb_retransmission_total > 0) {
            DBG_PRINTF("Unexpected, server retransmitted %" PRIu64 " packets", 
                (test_ctx->cnx_server == NULL)? UINT64_MAX:test_ctx->cnx_server->nb_retransmission_total);
            ret = -1;
        }
        else if (test_ctx->cnx_client->nb_retransmission_total > 0) {
            DBG_PRINTF("Unexpected, client retransmitted %" PRIu64 " packets", test_ctx->cnx_client->nb_retransmission_total);
            ret = -1;
        }
    }

    /* And then free the resource
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
fn large_client_hello() {
    const TEST_SCENARIO_Q_AND_R: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    test_ctx.cnx_client().test_large_chello = true;
    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        0,
        0,
        0,
        0,
        250_000,
    )
    .expect("large_client_hello q_and_r scenario");

    assert!(test_ctx.has_cnx_server(), "server connection not accepted");
    assert_eq!(
        test_ctx.cnx_server().nb_retransmission_total,
        0,
        "server retransmitted during large ClientHello scenario"
    );
    assert_eq!(
        test_ctx.cnx_client().nb_retransmission_total,
        0,
        "client retransmitted during large ClientHello scenario"
    );
}
```
