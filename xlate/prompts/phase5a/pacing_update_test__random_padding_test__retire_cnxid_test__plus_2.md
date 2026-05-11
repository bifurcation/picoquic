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

## `picoquictest/tls_api_test.c:random_padding_test`
* C test-table name: `random_padding`
* C entry function: `random_padding_test`
* Rust test: `random_padding`
* C source: `picoquictest/tls_api_test.c:12390-12401`
* Rust source: `rs/fq/src/tests/tls_api.rs:1018-1020`

### C test body
```c
{
    uint64_t random_context = 0x1234567890abcdef;

    int ret = random_padding_test_one(128, &random_context, 0);

    if (ret == 0) {
        ret = random_padding_test_one(16, &random_context, 1);
    }

    return ret;
}
```

### Rust test body
```rust
fn random_padding() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("random_padding");
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

## `picoquictest/tls_api_test.c:session_resume_test`
* C test-table name: `session_resume`
* C entry function: `session_resume_test`
* Rust test: `session_resume`
* C source: `picoquictest/tls_api_test.c:4316-4373`
* Rust source: `rs/fq/src/tests/tls_api.rs:1189-1195`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    char const* sni = PICOQUIC_TEST_SNI;
    char const* alpn = PICOQUIC_TEST_ALPN;
    uint64_t loss_mask = 0;
    int ret = 0;

    /* Initialize an empty ticket store */
    ret = picoquic_save_tickets(NULL, simulated_time, ticket_file_name);

    for (int i = 0; i < 2; i++) {
        /* Set up the context, while setting the ticket store parameter for the client */
        if (ret == 0) {
            ret = tls_api_init_ctx(&test_ctx, 0, sni, alpn, &simulated_time, ticket_file_name, NULL, 0, 0, 0);
        }

        if (ret == 0) {
            test_ctx->cnx_client->max_early_data_size = 0;

            ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
        }

        if (ret == 0 && i == 1) {
            /* If resume succeeded, the second connection will have a type "PSK" */
            if (picoquic_tls_is_psk_handshake(test_ctx->cnx_server) == 0 || picoquic_tls_is_psk_handshake(test_ctx->cnx_client) == 0) {
                ret = -1;
            }
        }

        if (ret == 0 && i == 0) {
            /* Before closing, wait for the session ticket to arrive */
            ret = session_resume_wait_for_ticket(test_ctx, &simulated_time);
        }

        if (ret == 0) {
            ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
        }

        /* Verify that the session ticket has been received correctly */
        if (ret == 0) {
            if (test_ctx->qclient->p_first_ticket == NULL) {
                ret = -1;
            } else {
                ret = picoquic_save_tickets(test_ctx->qclient->p_first_ticket, simulated_time, ticket_file_name);
            }
        }
        /* Tear down and free everything */

        if (test_ctx != NULL) {
            tls_api_delete_ctx(test_ctx);
            test_ctx = NULL;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn session_resume() {
    const TICKET_FILE: &str = "session_resume_test.bin";
    let t = Instant::from_ticks(0);
    save_empty_tickets(TICKET_FILE, t).expect("save_empty");
    session_resume_test_one(TICKET_FILE).expect("session_resume");
    let _ = t;
}
```

## `picoquictest/tls_api_test.c:stateless_reset_bad_test`
* C test-table name: `stateless_reset_bad`
* C entry function: `stateless_reset_bad_test`
* Rust test: `stateless_reset_bad`
* C source: `picoquictest/tls_api_test.c:3459-3503`
* Rust source: `rs/fq/src/tests/tls_api.rs:1266-1268`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    uint8_t buffer[256];

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare the bogus reset */
    if (ret == 0) {
        size_t byte_index = 0;
        buffer[byte_index++] = 0x41;
        byte_index += picoquic_format_connection_id(&buffer[byte_index], sizeof(buffer) - byte_index, test_ctx->cnx_client->path[0]->first_tuple->p_local_cnxid->cnx_id);
        memset(buffer + byte_index, 0xcc, sizeof(buffer) - byte_index);
        
        /* Submit bogus request to client */
        ret = picoquic_incoming_packet(test_ctx->qclient, buffer, sizeof(buffer),
            (struct sockaddr*)(&test_ctx->server_addr),
            (struct sockaddr*)(&test_ctx->client_addr), 0, test_ctx->recv_ecn_client,
            simulated_time);
    }

    /* check that the client is still up */
    if (ret == 0 && !TEST_CLIENT_READY) {
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
fn stateless_reset_bad() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_reset_bad");
}
```
