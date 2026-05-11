# Phase 5A C/Rust test correspondence audit

Review each C/Rust test pair and decide whether the Rust
`#[test]` checks the same behavior as the C test.  This is
a read-only pass: do not edit files.

You may inspect directly relevant context when needed:
Rust test helpers, fixtures, translated implementation under
`rs/fq/`, C helper functions in `picoquictest/`, constants,
and nearby tests.  Do not require byte-for-byte structure;
idiomatic Rust is fine when it preserves the C test intent,
inputs, expected results, and important edge cases.

Classify each entry as:

* `ok` when the Rust test is an acceptable translation.
* `needs_fix` when the Rust test is missing checks, checks
  materially different behavior, weakens assertions, skips
  cases the C test covers, or has placeholder-like logic.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"reviews":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short conclusion","fix_summary":"what 5B should change, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/tls_api_test.c:cid_quiescence_test`
* C test-table name: `cid_quiescence`
* C entry function: `cid_quiescence_test`
* Rust test: `cid_quiescence`
* C source: `picoquictest/tls_api_test.c:10906-10961`
* Rust source: `rs/fq/src/tests/tls_api.rs:119-121`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t previous_remote_id = picoquic_null_connection_id;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    /* Set up the connection */
    if (ret == 0) {
        /* establish the connection */
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        previous_remote_id = test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id;
        /* Prepare to send data */
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        previous_remote_id = test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id;
        simulated_time += PICOQUIC_CID_REFRESH_DELAY;
    }

    /* Perform a data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, 0);
    }
    
    /* Verify that the CID has rotated */
    if (ret == 0 &&
        picoquic_compare_connection_id(&previous_remote_id, &test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id) == 0) {
        ret = -1;
    }
    
    /* And then free the resource  */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn cid_quiescence() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("cid_quiescence");
}
```

## `picoquictest/tls_api_test.c:initial_server_close_test`
* C test-table name: `initial_server_close`
* C entry function: `initial_server_close_test`
* Rust test: `initial_server_close`
* C source: `picoquictest/tls_api_test.c:7544-7604`
* Rust source: `rs/fq/src/tests/tls_api.rs:454-457`

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    int was_active = 0;
    int nb_trials = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    /* Set the connection on the server side, but not on the client side */
    while (ret == 0 && nb_trials < 32 ) {
        nb_trials++;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

        if (test_ctx->cnx_server != NULL && test_ctx->cnx_server->cnx_state == picoquic_state_server_almost_ready) {
            break;
        }
    }

    if (test_ctx->cnx_server == NULL || test_ctx->cnx_server->cnx_state != picoquic_state_server_almost_ready) {
        DBG_PRINTF("Server state: %d\n", (test_ctx->cnx_server == NULL) ?
            -1 : test_ctx->cnx_server->cnx_state);
        ret = -1;
    }

    if (ret == 0) {
        test_ctx->cnx_server->cnx_state = picoquic_state_handshake_failure;
        test_ctx->cnx_server->local_error = 0xDEAD;
        picoquic_reinsert_by_wake_time(test_ctx->qserver, test_ctx->cnx_server, simulated_time);
    }


    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        if (test_ctx->cnx_server != NULL &&
            test_ctx->cnx_server->cnx_state != picoquic_state_disconnected) {
            DBG_PRINTF("Server state: %d, remote error: %x\n", test_ctx->cnx_server->cnx_state, test_ctx->cnx_server->remote_error);
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected ||
            test_ctx->cnx_client->remote_error != 0xDEAD) {
            DBG_PRINTF("Client state: %d, local error: %x", test_ctx->cnx_client->cnx_state, test_ctx->cnx_client->local_error);
            ret = -1;
        }
        else if (simulated_time > 50000ull) {
            DBG_PRINTF("Simulated time: %llu", (unsigned long long)simulated_time);
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

### Rust test body
```rust
fn initial_server_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("initial_server_close");
}
```

## `picoquictest/tls_api_test.c:migration_disabled_test`
* C test-table name: `migration_disabled`
* C entry function: `migration_disabled_test`
* Rust test: `migration_disabled`
* C source: `picoquictest/tls_api_test.c:10138-10173`
* Rust source: `rs/fq/src/tests/tls_api.rs:583-585`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Set the migration_disabled flag in the server parameter
     */
    if (ret == 0) {
        test_ctx->qserver->default_tp.migration_disabled = 1;

        /* Run a basic test scenario
         */

        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 250000);
    }

    /* verify that the migration was properly noticed as disabled. */
    if (ret == 0 && test_ctx->cnx_client != NULL &&
        !test_ctx->cnx_client->remote_parameters.migration_disabled) {
        DBG_PRINTF("%s", "Migration not disabled on client\n");
        ret = -1;
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn migration_disabled() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("migration_disabled");
}
```

## `picoquictest/tls_api_test.c:mtu_drop_bbr_test`
* C test-table name: `mtu_drop_bbr`
* C entry function: `mtu_drop_bbr_test`
* Rust test: `mtu_drop_bbr`
* C source: `picoquictest/tls_api_test.c:5090-5097`
* Rust source: `rs/fq/src/tests/tls_api.rs:650-652`

### C test body
```c
{
    /* TODO: the time with BBR v1 was 10300000. The current value is
     * a slight regression. Investigate whether some performance
     * for BBR3 "recover from PTO" could be improved. */
    int ret = mtu_drop_cc_algotest(picoquic_bbr_algorithm, 10700000);
    return ret;
}
```

### Rust test body
```rust
fn mtu_drop_bbr() {
    mtu_drop_cc_algotest("bbr", 10_700_000).expect("mtu_drop_bbr");
}
```

## `picoquictest/tls_api_test.c:tls_api_multiple_versions_test`
* C test-table name: `multiple_versions`
* C entry function: `tls_api_multiple_versions_test`
* Rust test: `multiple_versions`
* C source: `picoquictest/tls_api_test.c:4180-4193`
* Rust source: `rs/fq/src/tests/tls_api.rs:719-724`

### C test body
```c
{
    int ret = 0;

    for (size_t i = 1; ret == 0 && i < picoquic_nb_supported_versions; i++) {
        ret = tls_api_one_scenario_test(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0,
            picoquic_supported_versions[i].version, 0, NULL, NULL);
    }

    return ret;
}
```

### Rust test body
```rust
fn multiple_versions() {
    for ver in [V1, 0xFF00_0020u32, 0xFF00_0013u32] {
        tls_api_test_with_loss(None, ver, Some(TEST_SNI), Some(TEST_ALPN))
            .unwrap_or_else(|e| panic!("multiple_versions ver={ver:#x}: {e:?}"));
    }
}
```
