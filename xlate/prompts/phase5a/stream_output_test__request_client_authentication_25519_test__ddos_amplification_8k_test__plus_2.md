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

## `picoquictest/tls_api_test.c:request_client_authentication_25519_test`
* C test-table name: `client_auth_25519`
* C entry function: `request_client_authentication_25519_test`
* Rust test: `client_auth_25519`
* C source: `picoquictest/tls_api_test.c:5795-5839`
* Rust source: `rs/fq/src/tests/tls_api.rs:137-143`

### C test body
```c
{
    char test_client_cert_file[512];
    char test_client_key_file[512];
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_ca_cert_store_file[512];
    int ret = 0;

    ret = picoquic_get_input_path(test_client_cert_file, sizeof(test_client_cert_file),
                                  picoquic_solution_dir, PICOQUIC_TEST_FILE_CLIENT_CERT_ED25519);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_client_key_file, sizeof(test_client_key_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_CLIENT_KEY_ED25519);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file),
                                      picoquic_solution_dir,
                                      PICOQUIC_TEST_FILE_SERVER_CERT_ED25519);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY_ED25519);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_ca_cert_store_file, sizeof(test_ca_cert_store_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE_ED25519);
    }

    if (ret == 0) {
        ret = request_client_authentication_test_one(test_client_cert_file, test_client_key_file,
                                                     test_server_cert_file, test_server_key_file,
                                                     test_ca_cert_store_file);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "mTLS client-auth test failed ED25519\n");
    }

    return ret;
}
```

### Rust test body
```rust
fn client_auth_25519() {
    request_client_authentication_test_one(
        TEST_FILE_CLIENT_CERT_ED25519,
        TEST_FILE_CLIENT_KEY_ED25519,
    )
    .expect("client_auth_25519");
}
```

## `picoquictest/tls_api_test.c:ddos_amplification_8k_test`
* C test-table name: `ddos_amplification_8k`
* C entry function: `ddos_amplification_8k_test`
* Rust test: `ddos_amplification_8k`
* C source: `picoquictest/tls_api_test.c:10420-10423`
* Rust source: `rs/fq/src/tests/tls_api.rs:270-272`

### C test body
```c
{
    return ddos_amplification_test_one(0, 1);
}
```

### Rust test body
```rust
fn ddos_amplification_8k() {
    ddos_amplification_test_one(2, 0).expect("ddos_amplification_8k");
}
```

## `picoquictest/tls_api_test.c:get_hash_test`
* C test-table name: `get_hash`
* C entry function: `get_hash_test`
* Rust test: `get_hash`
* C source: `picoquictest/tls_api_test.c:12905-12925`
* Rust source: `rs/fq/src/tests/tls_api.rs:351-353`

### C test body
```c
{
    int ret = 0;
    char const* valid_hash = "sha256";
    char const* invalid_hash = "no_such_hash_nada_niente";

    picoquic_tls_api_init();
    if (get_hash_length_test(valid_hash) != 0) {
        ret = -1;
    }
    else if (get_hash_length_test(invalid_hash) == 0) {
        ret = -1;
    }
    else if (get_hash_algo_test(valid_hash) != 0) {
        ret = -1;
    }
    else if (get_hash_algo_test(invalid_hash) == 0) {
        ret = -1;
    }
    return (ret);
}
```

### Rust test body
```rust
fn get_hash() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("get_hash");
}
```

## `picoquictest/tls_api_test.c:immediate_close_test`
* C test-table name: `immediate_close`
* C entry function: `immediate_close_test`
* Rust test: `immediate_close`
* C source: `picoquictest/tls_api_test.c:3627-3690`
* Rust source: `rs/fq/src/tests/tls_api.rs:416-418`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t nb_packet_sent_before_close = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    uint8_t buffer[128];
    int was_active = 0;

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    /* Immediate close */
    if (ret == 0) {
        nb_packet_sent_before_close = test_ctx->cnx_server->nb_packets_sent;
        picoquic_close_immediate(test_ctx->cnx_server);
    }
    /* Client sends some data, in order to test the connection */
    if (ret == 0) {
        memset(buffer, 0xaa, sizeof(buffer));
        ret = picoquic_add_to_stream(test_ctx->cnx_client, 4,
            buffer, sizeof(buffer), 1);
    }

    /* Perform a couple rounds of sending data */
    for (int i = 0; ret == 0 && i < 256 ; i++) {
        was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
        if (test_ctx->cnx_client->cnx_state >= picoquic_state_disconnected) {
            /* Client has noticed the disconnect */
            ret = 0;
            break;
        }
    }

    /* Client and server should now be in state disconnected */
    if (ret == 0 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
        ret = -1;
    }

    if (ret == 0 && test_ctx->cnx_server != NULL){
        if (test_ctx->cnx_server->cnx_state != picoquic_state_disconnected) {
            ret = -1;
        }
        else if (nb_packet_sent_before_close != test_ctx->cnx_server->nb_packets_sent)
        {
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
fn immediate_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("immediate_close");
}
```
