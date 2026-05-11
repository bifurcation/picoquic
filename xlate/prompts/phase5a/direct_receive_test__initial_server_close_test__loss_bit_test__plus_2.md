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

## `picoquictest/tls_api_test.c:loss_bit_test`
* C test-table name: `loss_bit`
* C entry function: `loss_bit_test`
* Rust test: `loss_bit`
* C source: `picoquictest/tls_api_test.c:6231-6253`
* Rust source: `rs/fq/src/tests/tls_api.rs:546-548`

### C test body
```c
{
    int ret = 0;
    picoquic_tp_t client_parameters;
    picoquic_tp_t server_parameters;

    for (int i = 0; ret == 0 && i <= 3; i++) {
        memset(&client_parameters, 0, sizeof(picoquic_tp_t));
        memset(&server_parameters, 0, sizeof(picoquic_tp_t));
        picoquic_init_transport_parameters(&client_parameters);
        picoquic_init_transport_parameters(&server_parameters);

        client_parameters.enable_loss_bit = (i & 1);
        server_parameters.enable_loss_bit = ((i > 1) & 1);

        ret = tls_api_one_scenario_test(test_scenario_many_streams, sizeof(test_scenario_many_streams), 0, 0, 0, 0, 0, 250000, &client_parameters, &server_parameters);
        if (ret != 0) {
            DBG_PRINTF("Loss bit test fails for client: %d, server: %d, ret = %d", i & 1, i >> 1, ret);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn loss_bit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("loss_bit");
}
```

## `picoquictest/tls_api_test.c:migration_zero_test`
* C test-table name: `migration_zero`
* C entry function: `migration_zero_test`
* Rust test: `migration_zero`
* C source: `picoquictest/tls_api_test.c:6866-6869`
* Rust source: `rs/fq/src/tests/tls_api.rs:616-618`

### C test body
```c
{
    return migration_test_scenario(test_scenario_very_long, sizeof(test_scenario_q_and_r), 0, 1);
}
```

### Rust test body
```rust
fn migration_zero() {
    migration_test_scenario(&[], 0, true).expect("migration_zero");
}
```

## `picoquictest/tls_api_test.c:mtu_drop_newreno_test`
* C test-table name: `mtu_drop_newreno`
* C entry function: `mtu_drop_newreno_test`
* Rust test: `mtu_drop_newreno`
* C source: `picoquictest/tls_api_test.c:5117-5121`
* Rust source: `rs/fq/src/tests/tls_api.rs:682-684`

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_newreno_algorithm, 11600000);
    return ret;
}
```

### Rust test body
```rust
fn mtu_drop_newreno() {
    mtu_drop_cc_algotest("newreno", 11_600_000).expect("mtu_drop_newreno");
}
```
