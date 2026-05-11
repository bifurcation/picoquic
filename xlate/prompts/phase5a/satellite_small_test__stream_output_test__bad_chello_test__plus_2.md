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

## `picoquictest/satellite_test.c:satellite_small_test`
* C test-table name: `satellite_small`
* C entry function: `satellite_small_test`
* Rust test: `satellite_small`
* C source: `picoquictest/satellite_test.c:265-269`
* Rust source: `rs/fq/src/tests/satellite.rs:342-357`

### C test body
```c
{
    /* Should be less than 85 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 81500000, 10, 2, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_small() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        81_500_000,
        10,
        2,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/stream0_frame_test.c:stream_output_test`
* C test-table name: `stream_output`
* C entry function: `stream_output_test`
* Rust test: `stream_output`
* C source: `picoquictest/stream0_frame_test.c:800-907`
* Rust source: `rs/fq/src/tests/stream0_frame.rs:719-721`

### C test body
```c
{
    int ret = 0;
    picoquic_quic_t *quic = NULL;
    picoquic_cnx_t *cnx = NULL;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;
    uint64_t values[] = { 0, 3, 4, 1, 2, 8, 5, 7 };
    uint64_t output1[] = { 0, 1, 2, 4, 5 };
    uint64_t output2[] = { 0, 1, 2, 4, 5, 8 };
    uint64_t delete_order[] = { 1, 0, 4, 2, 5, 8 };
    picoquic_stream_head_t * stream = NULL;

    quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    memset(&saddr, 0, sizeof(struct sockaddr_in));
    saddr.sin_family = AF_INET;
    saddr.sin_port = 1000;

    if (quic == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }
    else {
        cnx = picoquic_create_cnx(quic,
            picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr *) &saddr,
            simulated_time, 0, "test-sni", "test-alpn", 1);

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create connection\n");
            ret = -1;
        }
        else {
            picoquic_set_callback(cnx, stream_output_test_callback, NULL);
            /* Set parameter data to a plausible value so tests can run */
            cnx->maxdata_remote = PICOQUIC_DEFAULT_0RTT_WINDOW;
            cnx->remote_parameters.initial_max_stream_data_bidi_remote = PICOQUIC_DEFAULT_0RTT_WINDOW;
            cnx->remote_parameters.initial_max_stream_data_uni = PICOQUIC_DEFAULT_0RTT_WINDOW;
            cnx->max_stream_id_bidir_remote = (cnx->client_mode) ? 4 : 0;
            cnx->max_stream_id_unidir_remote = (cnx->client_mode) ? 10 : 0;

            cnx->high_priority_stream_id = 1;

            /* Create the list of streams */
            for (int i = 0; i < 7; i++) {
                picoquic_create_stream(cnx, values[i]);
            }

            ret = stream_output_test_list(cnx, sizeof(output1) / sizeof(uint64_t), output1);

            if (ret == 0) {
                /* Relax the max stream id value and test order again */
                uint64_t old_limit = cnx->max_stream_id_bidir_remote;
                cnx->max_stream_id_bidir_remote = 8;
                picoquic_add_output_streams(cnx, old_limit, 8, 1);
                ret = stream_output_test_list(cnx, sizeof(output2) / sizeof(uint64_t), output2);
            }

            if (ret == 0) {
                /* Check that find ready stream returns NULL when no stream is ready */
                stream = picoquic_find_ready_stream(cnx);
                if (stream != NULL) {
                    DBG_PRINTF("Unexpected ready stream[%d]\n", (int)stream->stream_id);
                    ret = -1;
                }
            }

            if (ret == 0) {
                /* Mark all streams as active */
                stream = cnx->first_output_stream;

                while (stream != NULL) {
                    stream->maxdata_remote = 4096;
                    picoquic_mark_active_stream(cnx, stream->stream_id, 1, NULL);
                    stream = stream->next_output_stream;
                }

                /* Check that first stream is what we expect */
                stream = picoquic_find_ready_stream(cnx);
                if (stream == NULL) {
                    DBG_PRINTF("Expected stream[%d],got NULL\n", (int)output2[0]);
                    ret = -1;
                }
                else if (stream->stream_id != output2[0]) {
                    DBG_PRINTF("Expected stream[%d],got %d\n", (int)output2[0], (int)stream->stream_id);
                    ret = -1;
                }
            }

            if (ret == 0) {
                /* Check automated stream deletion */
                for (size_t i = 0; ret == 0 && i < (sizeof(delete_order) / sizeof(uint64_t)); i++) {
                    ret = stream_output_test_delete(cnx, delete_order[i], i & 1);
                }
            }

            picoquic_delete_cnx(cnx);
            cnx = NULL;
        }

        picoquic_free(quic);
        quic = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn stream_output() {
    stream_output_test_body().expect("stream_output_test");
}
```

## `picoquictest/tls_api_test.c:bad_chello_test`
* C test-table name: `bad_chello`
* C entry function: `bad_chello_test`
* Rust test: `bad_chello`
* C source: `picoquictest/tls_api_test.c:11973-12020`
* Rust source: `rs/fq/src/tests/tls_api.rs:56-61`

### C test body
```c
{
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t simulated_time = 0;
    picoquic_connection_id_t icid = { { 0xba, 0xdc, 0xe1, 0x10, 0, 0, 0, 0}, 8 };
    int ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0, &icid, 10000, 0, 0, 0);
    uint8_t buffer[PICOQUIC_ENFORCED_INITIAL_MTU];
    

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }
    else {
        picoquic_set_qlog(test_ctx->qserver, ".");
        /* Create an initial packet with a bad chello */
        ret = bad_chello_fill_initial(test_ctx->qserver, buffer, PICOQUIC_ENFORCED_INITIAL_MTU, chello_malformed, sizeof(chello_malformed));
    }

    /* Submit the packet to the server context */
    if (ret == 0) {
        picoquic_cnx_t* cnx_trial = NULL;
        ret = picoquic_incoming_packet_ex(test_ctx->qserver, buffer, PICOQUIC_ENFORCED_INITIAL_MTU,
            (struct sockaddr*)&test_ctx->client_addr, (struct sockaddr*)&test_ctx->server_addr, 0,
            0, &cnx_trial, simulated_time);
        if (cnx_trial != NULL) {
            DBG_PRINTF("Bad chello caused context creation at t=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* If not apparently broken, start the client connection. */
    if (ret == 0) {
        simulated_time += 10000;
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 20000, 2000000);

        if (ret == 0) {
            DBG_PRINTF("Post bad chello connection succeeds at t=%" PRIu64 , simulated_time);
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
    }

    return ret;
}
```

### Rust test body
```rust
fn bad_chello() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("bad_chello");
}
```

## `picoquictest/tls_api_test.c:client_only_test`
* C test-table name: `client_only`
* C entry function: `client_only_test`
* Rust test: `client_only`
* C source: `picoquictest/tls_api_test.c:6404-6440`
* Rust source: `rs/fq/src/tests/tls_api.rs:179-181`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xc1, 0x10, 0, 0, 0, 0, 0, 0}, 8 };
    int ret;

    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0) {
        /* First, try enforcement. We do set log on the server side, but it is expected to be empty */
        int connection_ret = 0;
        picoquic_enforce_client_only(test_ctx->qserver, 1);
        picoquic_set_qlog(test_ctx->qserver, ".");
        picoquic_set_qlog(test_ctx->qclient, ".");
        connection_ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
        if (connection_ret == 0 && test_ctx->cnx_client->cnx_state < picoquic_state_disconnected) {
            DBG_PRINTF("Connection unexpectedly succeeds, state=%d, ret=%d (0x%x)",
                test_ctx->cnx_client->cnx_state, connection_ret, connection_ret);
            ret = -1;
        }
        else if (test_ctx->cnx_server != NULL) {
            DBG_PRINTF("Connection context created on client-only note, ret=%d (0x%x)",
                connection_ret, connection_ret);
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
fn client_only() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("client_only");
}
```

## `picoquictest/tls_api_test.c:connection_drop_test`
* C test-table name: `connection_drop`
* C entry function: `connection_drop_test`
* Rust test: `connection_drop`
* C source: `picoquictest/tls_api_test.c:10548-10576`
* Rust source: `rs/fq/src/tests/tls_api.rs:245-247`

### C test body
```c
{
    int ret = 0;
    picoquic_state_enum target_state[9] = {
        picoquic_state_client_init_sent,
        picoquic_state_client_renegotiate,
        picoquic_state_client_init_resent,
        picoquic_state_server_init,
        picoquic_state_server_handshake,
        picoquic_state_client_handshake_start,
        picoquic_state_server_false_start,
        picoquic_state_server_almost_ready,
        picoquic_state_client_almost_ready
    };
    int target_is_client[9] = {
        1, 1, 1, 0, 0, 1, 0, 0, 1 };

    for (int i = 0; ret == 0 && i < 9; i++) {
        picoquic_state_enum c_state = (target_is_client[i]) ? target_state[i] : picoquic_state_ready;
        picoquic_state_enum s_state = (target_is_client[i]) ? picoquic_state_ready : target_state[i];

        ret = connection_drop_test_one(c_state, s_state, target_is_client[i]);
        if (ret == -1) {
            DBG_PRINTF("connection drop test %d fails", i);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn connection_drop() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("connection_drop");
}
```
