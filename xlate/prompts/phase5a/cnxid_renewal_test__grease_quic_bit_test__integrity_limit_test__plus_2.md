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

## `picoquictest/tls_api_test.c:grease_quic_bit_test`
* C test-table name: `grease_quic_bit`
* C entry function: `grease_quic_bit_test`
* Rust test: `grease_quic_bit`
* C source: `picoquictest/tls_api_test.c:11027-11030`
* Rust source: `rs/fq/src/tests/tls_api.rs:367-369`

### C test body
```c
{
    return  grease_quic_bit_test_one(0);
}
```

### Rust test body
```rust
fn grease_quic_bit() {
    grease_quic_bit_test_one(false).expect("grease_quic_bit");
}
```

## `picoquictest/tls_api_test.c:integrity_limit_test`
* C test-table name: `integrity_limit`
* C entry function: `integrity_limit_test`
* Rust test: `integrity_limit`
* C source: `picoquictest/tls_api_test.c:11376-11459`
* Rust source: `rs/fq/src/tests/tls_api.rs:464-466`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    int nb_initial_loop = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x15, 0x4E, 0x98, 0x14, 0, 0, 0, 1}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    if (ret == 0) {
        picoquic_set_qlog(test_ctx->qserver, ".");
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    /* Perform a data sending loop for a few rounds after the ready state */
    while (ret == 0 && nb_initial_loop < 64) {
        if (test_ctx->cnx_server != NULL && 
            test_ctx->cnx_server->crypto_context[picoquic_epoch_1rtt].aead_decrypt != NULL) {
            nb_initial_loop++;
        }

        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 16);
    }

    /* Check the max length of an epoch is the expected value */
    if (ret == 0) {
        uint64_t limit = picoquic_aead_confidentiality_limit(test_ctx->cnx_server->crypto_context[picoquic_epoch_1rtt].aead_decrypt);

        if (test_ctx->cnx_server->crypto_epoch_length_max != limit) {
            DBG_PRINTF("Server confidentiality limit set to 0x%" PRIx64 ", insted of %" PRIx64,
                test_ctx->cnx_server->crypto_epoch_length_max, limit);
            ret = -1;
        } else if (test_ctx->cnx_client->crypto_epoch_length_max != limit) {
            DBG_PRINTF("Client confidentiality limit set to 0x%" PRIx64 ", insted of %" PRIx64,
                test_ctx->cnx_client->crypto_epoch_length_max, limit);
            ret = -1;
        }
    }


    /* Set the number of failed decryptions just below the limit and then send a bad packet  */
    if (ret == 0 && test_ctx->cnx_server != NULL) {
        uint8_t p[256];

        test_ctx->cnx_server->crypto_failure_count = picoquic_aead_integrity_limit(
            test_ctx->cnx_server->crypto_context[picoquic_epoch_1rtt].aead_decrypt);

        memset(p, 0, sizeof(p));
        memcpy(p + 1, test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id.id, test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len);
        p[0] |= 64;
        (void)picoquic_incoming_packet(test_ctx->qserver, p, sizeof(p), (struct sockaddr*) & test_ctx->cnx_server->path[0]->first_tuple->peer_addr,
            (struct sockaddr*) & test_ctx->cnx_server->path[0]->first_tuple->local_addr, 0, test_ctx->recv_ecn_server, simulated_time);

        if (test_ctx->cnx_server->cnx_state != picoquic_state_disconnecting) {
            DBG_PRINTF("Connection not disconnecting, limit 0x%" PRIx64 ", reached %" PRIx64,
                test_ctx->cnx_server->crypto_context[picoquic_epoch_1rtt].aead_decrypt,
                test_ctx->cnx_server->crypto_failure_count);
            ret = -1;
        }
        else if (test_ctx->cnx_server->local_error != PICOQUIC_TRANSPORT_AEAD_LIMIT_REACHED) {
            DBG_PRINTF("Wrong error code, 0x%x instead of 0x%x",
                test_ctx->cnx_server->local_error,
                PICOQUIC_TRANSPORT_AEAD_LIMIT_REACHED);
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
fn integrity_limit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("integrity_limit");
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
