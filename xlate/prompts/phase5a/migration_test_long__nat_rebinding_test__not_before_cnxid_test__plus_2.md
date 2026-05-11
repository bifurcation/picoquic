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

## `picoquictest/tls_api_test.c:migration_test_long`
* C test-table name: `migration_long`
* C entry function: `migration_test_long`
* Rust test: `migration_long`
* C source: `picoquictest/tls_api_test.c:6854-6857`
* Rust source: `rs/fq/src/tests/tls_api.rs:600-602`

### C test body
```c
{
    return migration_test_scenario(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0);
}
```

### Rust test body
```rust
fn migration_long() {
    migration_test_scenario(&[], 0, false).expect("migration_long");
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

## `picoquictest/tls_api_test.c:padding_zero_min_test`
* C test-table name: `padding_zero_min`
* C entry function: `padding_zero_min_test`
* Rust test: `padding_zero_min`
* C source: `picoquictest/tls_api_test.c:8741-8744`
* Rust source: `rs/fq/src/tests/tls_api.rs:876-878`

### C test body
```c
{
    return padding_test_one(128, 0);
}
```

### Rust test body
```rust
fn padding_zero_min() {
    padding_test_one(128, 0).expect("padding_zero_min");
}
```

## `picoquictest/tls_api_test.c:probe_api_test`
* C test-table name: `probe_api`
* C entry function: `probe_api_test`
* Rust test: `probe_api`
* C source: `picoquictest/tls_api_test.c:6643-6733`
* Rust source: `rs/fq/src/tests/tls_api.rs:947-949`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    struct sockaddr_in t4[PICOQUIC_NB_PATH_TARGET];
    struct sockaddr_in6 t6[PICOQUIC_NB_PATH_TARGET];
    int nb_trials;

    /* Initialize the test addresses to synthetic values */
    for (int i = 0; i < PICOQUIC_NB_PATH_TARGET; i++) {
        memset(&t4[i], 0, sizeof(struct sockaddr_in));
        t4[i].sin_family = AF_INET;
        t4[i].sin_port = 1000+i;
        memset(&t4[i].sin_addr, i, 4);
        memset(&t6[i], 0, sizeof(struct sockaddr_in6));
        t6[i].sin6_family = AF_INET6;
        t6[i].sin6_port = 2000 + i;
        memset(&t6[i].sin6_addr, i, 16);
    }

    /* Set a test connection between client and server */
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
            DBG_PRINTF("Only %d CID created on client.\n", test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid);
            ret = -1;
        }
        else if (test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid < PICOQUIC_NB_PATH_TARGET) {
            DBG_PRINTF("Only %d CID created on server.\n", test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid);
        }
    }

    /* Now, create a series of probes.
     * There are only PICOQUIC_NB_PATH_TARGET - 1 paths available. 
     * The last trial should fail.
     */
    nb_trials = 0;

    for (int i = 1; ret == 0 && test_ctx->cnx_client->nb_paths < PICOQUIC_NB_PATH_TARGET && i < PICOQUIC_NB_PATH_TARGET; i++) {
        for (int j = 0; ret == 0 && j < 2; j++) {
            int ret_probe;
            if (j == 0) {
                ret_probe = picoquic_probe_new_path(test_ctx->cnx_client, (struct sockaddr *) &t4[0], 
                    (struct sockaddr *) &t4[i], simulated_time);
            } else {
                ret_probe = picoquic_probe_new_path(test_ctx->cnx_client, (struct sockaddr *) &t6[0],
                    (struct sockaddr *) &t6[i], simulated_time);
            }

            nb_trials++;

            if (nb_trials < PICOQUIC_NB_PATH_TARGET) {
                if (ret_probe != 0) {
                    DBG_PRINTF("Trial %d (%d, %d) fails with ret = %x\n", nb_trials, i, j, ret_probe);
                    ret = -1;
                }
            }
            else if (ret_probe == 0) {
                DBG_PRINTF("Trial %d (%d, %d) succeeds (unexpected)\n", nb_trials, i, j);
                ret = -1;
            }

            if (ret == 0 && ret_probe == 0) {
                int path_id = test_ctx->cnx_client->nb_paths - 1;
                for (int ichal = 0; ichal < PICOQUIC_CHALLENGE_REPEAT_MAX; ichal++) {
                    test_ctx->cnx_client->path[path_id]->first_tuple->challenge[ichal] = (uint64_t)10000 + (uint64_t)10 * i + j + (uint64_t)1000*ichal;
                }
            }
        }
    }

    /* Releasing the context will test the delete functions. */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn probe_api() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("probe_api");
}
```
