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

## `picoquictest/tls_api_test.c:migration_test_loss`
* C test-table name: `migration_with_loss`
* C entry function: `migration_test_loss`
* Rust test: `migration_with_loss`
* C source: `picoquictest/tls_api_test.c:6859-6864`
* Rust source: `rs/fq/src/tests/tls_api.rs:608-610`

### C test body
```c
{
    uint64_t loss_mask = 0x09;

    return migration_test_scenario(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), loss_mask, 0);
}
```

### Rust test body
```rust
fn migration_with_loss() {
    migration_test_scenario(&[], 0x09, false).expect("migration_with_loss");
}
```

## `picoquictest/tls_api_test.c:optimistic_hole_test`
* C test-table name: `optimistic_hole`
* C entry function: `optimistic_hole_test`
* Rust test: `optimistic_hole`
* C source: `picoquictest/tls_api_test.c:9769-9774`
* Rust source: `rs/fq/src/tests/tls_api.rs:834-836`

### C test body
```c
{
    int ret = optimistic_ack_test_one(0);

    return ret;
}
```

### Rust test body
```rust
fn optimistic_hole() {
    optimistic_ack_test_one(false).expect("optimistic_hole");
}
```

## `picoquictest/tls_api_test.c:qlog_trace_test`
* C test-table name: `qlog_trace`
* C entry function: `qlog_trace_test`
* Rust test: `qlog_trace`
* C source: `picoquictest/tls_api_test.c:9087-9090`
* Rust source: `rs/fq/src/tests/tls_api.rs:973-975`

### C test body
```c
{
    return qlog_trace_test_one(0, 0);
}
```

### Rust test body
```rust
fn qlog_trace() {
    qlog_trace_test_one(0, false).expect("qlog_trace");
}
```

## `picoquictest/tls_api_test.c:ready_to_skip_test`
* C test-table name: `ready_to_skip`
* C entry function: `ready_to_skip_test`
* Rust test: `ready_to_skip`
* C source: `picoquictest/tls_api_test.c:9454-9458`
* Rust source: `rs/fq/src/tests/tls_api.rs:1046-1048`

### C test body
```c
{
    int ret = ready_to_send_test_one(3);
    return ret;
}
```

### Rust test body
```rust
fn ready_to_skip() {
    ready_to_send_test_one(3).expect("ready_to_skip");
}
```

## `picoquictest/tls_api_test.c:retire_cnxid_test`
* C test-table name: `retire_cnxid`
* C entry function: `retire_cnxid_test`
* Rust test: `retire_cnxid`
* C source: `picoquictest/tls_api_test.c:7207-7307`
* Rust source: `rs/fq/src/tests/tls_api.rs:1120-1122`

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

### Rust test body
```rust
fn retire_cnxid() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("retire_cnxid");
}
```
