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

## `picoquictest/tls_api_test.c:direct_receive_test`
* C test-table name: `direct_receive`
* C entry function: `direct_receive_test`
* Rust test: `direct_receive`
* C source: `picoquictest/tls_api_test.c:10709-10766`
* Rust source: `rs/fq/src/tests/tls_api.rs:291-293`

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t loss_mask = 8;
    uint64_t max_completion_microsec = 3500000;

    ret = tls_api_one_scenario_init(&test_ctx, &simulated_time,
        0, NULL, NULL);

    if (ret == 0) {
        ret = tls_api_one_scenario_body_connect(test_ctx, &simulated_time, 0, 0);

        /* Prepare to send data */
        if (ret == 0) {
            test_ctx->stream0_target = 0;
            ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));

            if (ret != 0)
            {
                DBG_PRINTF("Init send receive scenario returns %d\n", ret);
            }
        }

        /* Set the direct receive API for the stream number 4. */
        if (ret == 0) {
            ret = picoquic_mark_direct_receive_stream(test_ctx->cnx_client, 4, test_api_direct_receive_callback, 
                (void*)&test_ctx->client_callback);

            if (ret != 0)
            {
                DBG_PRINTF("Mark direct receive stream returns %d\n", ret);
            }
        }

        /* Perform a data sending loop */
        if (ret == 0) {
            ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);

            if (ret != 0)
            {
                DBG_PRINTF("Data sending loop returns %d\n", ret);
            }
        }

        if (ret == 0) {
            ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, max_completion_microsec);
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
fn direct_receive() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("direct_receive");
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

## `picoquictest/tls_api_test.c:mtu_drop_cubic_test`
* C test-table name: `mtu_drop_cubic`
* C entry function: `mtu_drop_cubic_test`
* Rust test: `mtu_drop_cubic`
* C source: `picoquictest/tls_api_test.c:5099-5103`
* Rust source: `rs/fq/src/tests/tls_api.rs:658-660`

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_cubic_algorithm, 10000000);
    return ret;
}
```

### Rust test body
```rust
fn mtu_drop_cubic() {
    mtu_drop_cc_algotest("cubic", 10_000_000).expect("mtu_drop_cubic");
}
```

## `picoquictest/tls_api_test.c:nat_rebinding_test`
* C test-table name: `nat_rebinding`
* C entry function: `nat_rebinding_test`
* Rust test: `nat_rebinding`
* C source: `picoquictest/tls_api_test.c:6072-6077`
* Rust source: `rs/fq/src/tests/tls_api.rs:739-741`

### C test body
```c
{
    uint64_t loss_mask = 0;

    return nat_rebinding_test_one(loss_mask, 0, 0);
}
```

### Rust test body
```rust
fn nat_rebinding() {
    nat_rebinding_test_one(0, false, 0).expect("nat_rebinding");
}
```

## `picoquictest/tls_api_test.c:not_before_cnxid_test`
* C test-table name: `not_before_cnxid`
* C entry function: `not_before_cnxid_test`
* Rust test: `not_before_cnxid`
* C source: `picoquictest/tls_api_test.c:7310-7402`
* Rust source: `rs/fq/src/tests/tls_api.rs:807-809`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t not_before;
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

    /* find a plausible "not before" value, and apply it */
    if (ret == 0) {
        not_before = test_ctx->cnx_server->first_local_cnxid_list->local_cnxid_sequence_next - 1;
        uint64_t transport_error = picoquic_remove_not_before_cid(test_ctx->cnx_client, 0, not_before, simulated_time);
        if (transport_error != 0) {
            DBG_PRINTF("picoquic_remove_not_before_cid returns 0x%" PRIx64, transport_error);
            ret = -1;
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

            if (nb_rounds == 30) {
                ret = 0;
            }

            if (test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid >= PICOQUIC_NB_PATH_TARGET &&
                test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid >= PICOQUIC_NB_PATH_TARGET &&
                test_ctx->cnx_client->first_misc_frame == NULL &&
                test_cnxid_count_stash(test_ctx->cnx_client) >= (PICOQUIC_NB_PATH_TARGET - 1) &&
                test_cnxid_count_stash(test_ctx->cnx_server) >= (PICOQUIC_NB_PATH_TARGET - 1) &&
                picoquic_is_cnx_backlog_empty(test_ctx->cnx_client) &&
                picoquic_is_cnx_backlog_empty(test_ctx->cnx_server)) {
                success = 1;
                break;
            }
        }

        if (ret == 0 && success == 0) {
            DBG_PRINTF("Exit synch loop after %d rounds, backlog or not enough cid (%d & %d).\n",
                nb_rounds, test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid, test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid);
        }
    }

    /* Check */

    if (ret == 0) {
        if (test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid != PICOQUIC_NB_PATH_TARGET) {
            DBG_PRINTF("Found %d cid active on server instead of %d.\n", test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid, PICOQUIC_NB_PATH_TARGET + 1);
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

### Rust test body
```rust
fn not_before_cnxid() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("not_before_cnxid");
}
```
