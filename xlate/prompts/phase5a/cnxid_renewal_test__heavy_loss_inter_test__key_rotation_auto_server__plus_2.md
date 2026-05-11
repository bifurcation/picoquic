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

## `picoquictest/tls_api_test.c:cnxid_renewal_test`
* C test-table name: `cnxid_renewal`
* C entry function: `cnxid_renewal_test`
* Rust test: `cnxid_renewal`
* C source: `picoquictest/tls_api_test.c:7120-7204`
* Rust source: `rs/fq/src/tests/tls_api.rs:197-199`

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

### Rust test body
```rust
fn cnxid_renewal() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("cnxid_renewal");
}
```

## `picoquictest/tls_api_test.c:heavy_loss_inter_test`
* C test-table name: `heavy_loss_inter`
* C entry function: `heavy_loss_inter_test`
* Rust test: `heavy_loss_inter`
* C source: `picoquictest/tls_api_test.c:11364-11367`
* Rust source: `rs/fq/src/tests/tls_api.rs:392-394`

### C test body
```c
{
    return heavy_loss_test_one(1, 22000000);
}
```

### Rust test body
```rust
fn heavy_loss_inter() {
    heavy_loss_test_one(1, 22_000_000).expect("heavy_loss_inter");
}
```

## `picoquictest/tls_api_test.c:key_rotation_auto_server`
* C test-table name: `key_rotation_server`
* C entry function: `key_rotation_auto_server`
* Rust test: `key_rotation_server`
* C source: `picoquictest/tls_api_test.c:7976-7979`
* Rust source: `rs/fq/src/tests/tls_api.rs:498-500`

### C test body
```c
{
    return key_rotation_auto_one(300, 0);
}
```

### Rust test body
```rust
fn key_rotation_server() {
    key_rotation_auto_one(300, false).expect("key_rotation_server");
}
```

## `picoquictest/tls_api_test.c:migration_test`
* C test-table name: `migration`
* C entry function: `migration_test`
* Rust test: `migration`
* C source: `picoquictest/tls_api_test.c:6849-6852`
* Rust source: `rs/fq/src/tests/tls_api.rs:574-576`

### C test body
```c
{
    return migration_test_scenario(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0);
}
```

### Rust test body
```rust
fn migration() {
    migration_test_scenario(&[], 0, false).expect("migration");
}
```

## `picoquictest/tls_api_test.c:mtu_discovery_test`
* C test-table name: `mtu_discovery`
* C entry function: `mtu_discovery_test`
* Rust test: `mtu_discovery`
* C source: `picoquictest/tls_api_test.c:4974-4979`
* Rust source: `rs/fq/src/tests/tls_api.rs:642-644`

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_basic, 1440, 1440, 
        test_scenario_mtu_discovery, sizeof(test_scenario_mtu_discovery), 0);
    return ret;
}
```

### Rust test body
```rust
fn mtu_discovery() {
    mtu_discovery_test_one(0, 1440, 1440, 2_500_000, 0).expect("mtu_discovery");
}
```
