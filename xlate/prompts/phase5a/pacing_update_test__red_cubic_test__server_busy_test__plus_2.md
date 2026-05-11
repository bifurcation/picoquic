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

## `picoquictest/tls_api_test.c:pacing_update_test`
* C test-table name: `pacing_update`
* C entry function: `pacing_update_test`
* Rust test: `pacing_update`
* C source: `picoquictest/tls_api_test.c:10590-10640`
* Rust source: `rs/fq/src/tests/tls_api.rs:843-845`

### C test body
```c
{
    uint64_t simulated_time = 0;

    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }
    if (ret == 0) {
        /* Open a file to log bandwidth updates and document it in context */
        test_ctx->bw_update = picoquic_file_open(PACING_RATE_CSV, "w");
        if (test_ctx->bw_update == NULL) {
            DBG_PRINTF("Could not write file <%s>", PACING_RATE_CSV);
            ret = -1;
        }
        else {
            fprintf(test_ctx->bw_update, "Time, Pacing_rate_CB, Pacing_rate, CWIN, RTT\n");
            /* Request bandwidth updates */
            picoquic_subscribe_pacing_rate_updates(test_ctx->cnx_client, 0x8000, 0x10000);

            /* Start a standard scenario, pushing 1MB from the client*/
            ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
                test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 1000000, 0, 0, 20000, 3600000);
        }
    }

    /* Free the test contex, which closes the trace file  */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    /* compare the trace to the expected value */
    if (ret == 0)
    {
        char pacing_rate_ref[512];

        ret = picoquic_get_input_path(pacing_rate_ref, sizeof(pacing_rate_ref), picoquic_solution_dir, PACING_RATE_TEST_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the pacing rate test ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(PACING_RATE_CSV, pacing_rate_ref);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn pacing_update() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("pacing_update");
}
```

## `picoquictest/tls_api_test.c:red_cubic_test`
* C test-table name: `red_cubic`
* C entry function: `red_cubic_test`
* Rust test: `red_cubic`
* C source: `picoquictest/tls_api_test.c:11120-11124`
* Rust source: `rs/fq/src/tests/tls_api.rs:1078-1080`

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_cubic_algorithm, 510000, 225);
    return ret;
}
```

### Rust test body
```rust
fn red_cubic() {
    red_cc_algotest("cubic", 510_000, 225).expect("red_cubic");
}
```

## `picoquictest/tls_api_test.c:server_busy_test`
* C test-table name: `server_busy`
* C entry function: `server_busy_test`
* Rust test: `server_busy`
* C source: `picoquictest/tls_api_test.c:7408-7474`
* Rust source: `rs/fq/src/tests/tls_api.rs:1172-1174`

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        test_ctx->qserver->server_busy = 1;
        (void) tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);

        if (test_ctx->cnx_server != NULL &&
            test_ctx->cnx_server->cnx_state != picoquic_state_disconnected) {
            DBG_PRINTF("Server state: %d, local error: %" PRIx64, test_ctx->cnx_server->cnx_state, test_ctx->cnx_server->local_error);
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected ||
            test_ctx->cnx_client->remote_error != PICOQUIC_TRANSPORT_SERVER_BUSY) {
            DBG_PRINTF("Client state: %d, remote error: %" PRIx64, test_ctx->cnx_client->cnx_state, test_ctx->cnx_client->remote_error);
            ret = -1;
        }
        else if (simulated_time > 500000ull) {
            DBG_PRINTF("Simulated time: %" PRIu64, (unsigned long long)simulated_time);
            ret = -1;
        }
    }

    if (ret == 0) {
        test_ctx->qserver->server_busy = 0;

        if (test_ctx->cnx_server != NULL) {
            picoquic_delete_cnx(test_ctx->cnx_server);
            test_ctx->cnx_server = NULL;
        }
        if (test_ctx->cnx_client != NULL) {
            picoquic_delete_cnx(test_ctx->cnx_client);
            test_ctx->cnx_client = NULL;
        }

        /* Create a new client connection */
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient,
            picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*)&test_ctx->server_addr, simulated_time,
            0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        } else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }

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
fn server_busy() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("server_busy");
}
```

## `picoquictest/tls_api_test.c:test_stateless_blowback`
* C test-table name: `stateless_blowback`
* C entry function: `test_stateless_blowback`
* Rust test: `stateless_blowback`
* C source: `picoquictest/tls_api_test.c:12182-12284`
* Rust source: `rs/fq/src/tests/tls_api.rs:1250-1252`

### C test body
```c
{
    int was_sent = 0;
    uint64_t new_interval = 2 * PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT;

    /* Create a context with the default timer. */
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;

    picoquic_connection_id_t initial_cid = { {0xb1, 0x08, 0xba, 0xcc, 0, 0, 0, 0}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN,
        &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);

    if (test_ctx->qserver->stateless_reset_min_interval != PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT) {
        DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
        ret = -1;

    }

    /* Format a random packet and submit it, verify that the stateless reset is queued */
    if (ret == 0) {
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("First stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Progress by 1/2 specified interval, retry, it should not work  */
    if (ret == 0) {
        simulated_time += test_ctx->qserver->stateless_reset_min_interval;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("Second stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }
    
    /* Progress by 1x specified interval, retry, it should work  */
    if (ret == 0) {
        simulated_time += test_ctx->qserver->stateless_reset_min_interval / 2;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && was_sent) {
            DBG_PRINTF("Third stateless reset was sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Reset the interval to twice the previous value */
    if (ret == 0) {
        picoquic_set_default_stateless_reset_min_interval(test_ctx->qserver, new_interval);
        if (test_ctx->qserver->stateless_reset_min_interval != new_interval) {
            DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
            ret = -1;
        }
    }
    
    /* Progress by 0.75x new interval, retry, it should work  */
    if (ret == 0) {
        simulated_time += (new_interval - new_interval / 4);
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After new interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Reset the interval to zero */
    if (ret == 0) {
        picoquic_set_default_stateless_reset_min_interval(test_ctx->qserver, 0);
        if (test_ctx->qserver->stateless_reset_min_interval != 0) {
            DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
            ret = -1;
        }
    }

    /* Try immediately, it should not work  */
    if (ret == 0) {
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After zero interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Add 1 microsec, it should work  */
    if (ret == 0) {
        simulated_time += 1;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After zero +1 interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Free the resurce and return */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn stateless_blowback() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_blowback");
}
```

## `picoquictest/tls_api_test.c:tls_api_test`
* C test-table name: `tls_api`
* C entry function: `tls_api_test`
* Rust test: `tls_api`
* C source: `picoquictest/tls_api_test.c:2283-2286`
* Rust source: `rs/fq/src/tests/tls_api.rs:1321-1323`

### C test body
```c
{
    return tls_api_test_with_loss(NULL, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN);
}
```

### Rust test body
```rust
fn tls_api() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_api");
}
```
