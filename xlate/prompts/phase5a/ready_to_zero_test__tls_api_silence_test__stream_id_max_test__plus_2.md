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

## `picoquictest/tls_api_test.c:ready_to_zero_test`
* C test-table name: `ready_to_zero`
* C entry function: `ready_to_zero_test`
* Rust test: `ready_to_zero`
* C source: `picoquictest/tls_api_test.c:9460-9464`
* Rust source: `rs/fq/src/tests/tls_api.rs:1054-1056`

### C test body
```c
{
    int ret = ready_to_send_test_one(4);
    return ret;
}
```

### Rust test body
```rust
fn ready_to_zero() {
    ready_to_send_test_one(4).expect("ready_to_zero");
}
```

## `picoquictest/tls_api_test.c:tls_api_silence_test`
* C test-table name: `silence_test`
* C entry function: `tls_api_silence_test`
* Rust test: `silence_test`
* C source: `picoquictest/tls_api_test.c:2441-2480`
* Rust source: `rs/fq/src/tests/tls_api.rs:1233-1235`

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    uint64_t next_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* simulate 5 seconds of silence */
    next_time = simulated_time + 5000000;
    while (ret == 0 && simulated_time < next_time && TEST_CLIENT_READY && TEST_SERVER_READY) {
        int was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, next_time, &was_active);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        /* verify the absence of any spurious retransmission */
        if (test_ctx->cnx_client->nb_retransmission_total != 0) {
            ret = -1;
        } else if (test_ctx->cnx_server != NULL && test_ctx->cnx_server->nb_retransmission_total != 0) {
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
fn silence_test() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("silence_test");
}
```

## `picoquictest/tls_api_test.c:stream_id_max_test`
* C test-table name: `stream_id_max`
* C entry function: `stream_id_max_test`
* Rust test: `stream_id_max`
* C source: `picoquictest/tls_api_test.c:8546-8556`
* Rust source: `rs/fq/src/tests/tls_api.rs:1311-1315`

### C test body
```c
{
    picoquic_tp_t test_parameters;

    memset(&test_parameters, 0, sizeof(picoquic_tp_t));

    picoquic_init_transport_parameters(&test_parameters);
    test_parameters.initial_max_stream_id_bidir = 4;

    return tls_api_one_scenario_test(test_scenario_many_streams, sizeof(test_scenario_many_streams), 0, 0, 0, 0, 0, 250000, NULL, &test_parameters);
}
```

### Rust test body
```rust
fn stream_id_max() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_000_000).expect("stream_id_max");
}
```

## `picoquictest/tls_api_test.c:tls_api_sni_test`
* C test-table name: `tls_api_sni`
* C entry function: `tls_api_sni_test`
* Rust test: `tls_api_sni`
* C source: `picoquictest/tls_api_test.c:2968-2971`
* Rust source: `rs/fq/src/tests/tls_api.rs:1394-1396`

### C test body
```c
{
    return tls_api_test_with_loss(NULL, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN);
}
```

### Rust test body
```rust
fn tls_api_sni() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_api_sni");
}
```

## `picoquictest/tls_api_test.c:tls_api_two_connections_test`
* C test-table name: `two_connections`
* C entry function: `tls_api_two_connections_test`
* Rust test: `two_connections`
* C source: `picoquictest/tls_api_test.c:4095-4153`
* Rust source: `rs/fq/src/tests/tls_api.rs:1478-1480`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        /* Verify that the connection is fully established */
        uint64_t target_time = simulated_time + 2000000;

        while (ret == 0 && TEST_CLIENT_READY && TEST_SERVER_READY && simulated_time < target_time) {
            int was_active = 0;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, target_time, &was_active);
        }

        /* Delete the client connection from the client context,
         * without sending notification to the server */
        while (test_ctx->qclient->cnx_list != NULL) {
            picoquic_delete_cnx(test_ctx->qclient->cnx_list);
        }

        /* Erase the server connection reference */
        test_ctx->cnx_server = NULL;

        /* Create a new connection in the client context */

        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient,
            picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*)&test_ctx->server_addr, simulated_time, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
        else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }

    /* Now, restart a connection in the same context */
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

### Rust test body
```rust
fn two_connections() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("two_connections");
}
```
