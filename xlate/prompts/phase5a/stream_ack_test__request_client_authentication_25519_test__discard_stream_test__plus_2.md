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

## `picoquictest/skip_frame_test.c:stream_ack_test`
* C test-table name: `stream_ack`
* C entry function: `stream_ack_test`
* Rust test: `stream_ack`
* C source: `picoquictest/skip_frame_test.c:3740-3845`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1025-1029`

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    picoquic_cnx_t* cnx = NULL;
    struct sockaddr_storage addr;
    picoquic_quic_t* quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    if (quic == NULL) {
        ret = -1;
    }
    else {
        ret = picoquic_store_text_addr(&addr, "10.0.0.1", 1234);
        if (ret == 0) {
            cnx = picoquic_create_cnx(quic, picoquic_null_connection_id,
                picoquic_null_connection_id, (struct sockaddr*) & addr,
                simulated_time, 0, "test-sni", "test-alpn", 1);
            if (cnx == NULL) {
                ret = -1;
            }
        }
    }

    if (ret == 0) {
        /* Create the required streams */
        for (size_t i = 0; i < sizeof(stream_ack_stream_list) / sizeof(uint64_t); i++) {
            if (picoquic_create_stream(cnx, stream_ack_stream_list[i]) == NULL) {
                DBG_PRINTF("Cannot create stream %" PRIu64, stream_ack_stream_list[i]);
                ret = -1;
                break;
            }
        }
    }

    if (ret == 0) {
        /* Acknowledge the specified packets */
        for (size_t i = 0; ret == 0 && i < nb_stream_ack_case; i++) {
            uint8_t * bytes = stream_ack_case[i].bytes;
            uint8_t * bytes_max = bytes + stream_ack_case[i].length;
            while (bytes < bytes_max && stream_ack_case[i].should_ack) {
                size_t consumed = 0;

                ret = picoquic_process_ack_of_stream_frame(cnx,
                    bytes, bytes_max - bytes, &consumed);
                if (ret != 0) {
                    DBG_PRINTF("Case %zu, cannot process frame index %zu",
                        i, bytes - stream_ack_case[i].bytes);
                    ret = -1;
                    break;
                }
                else {
                    bytes += consumed;
                }
            }
        }
    }

    if (ret == 0) {
        /* verify the expected acks */
        for (size_t i = 0; i < nb_stream_ack_case; i++) {
            uint8_t * bytes = stream_ack_case[i].bytes;
            size_t byte_index = 0;
            size_t bytes_max = stream_ack_case[i].length;
            while (byte_index < stream_ack_case[i].length){
                size_t consumed = 0;
                int is_pure_ack = 0;
                int do_not_detect_spurious = 0;

                ret = picoquic_skip_frame(
                    bytes + byte_index, bytes_max - byte_index, &consumed, &is_pure_ack);
                if (ret != 0) {
                    DBG_PRINTF("Case %zu, cannot process frame index %zu",
                        i, byte_index);
                    ret = -1;
                    break;
                }
                else {
                    int no_need_to_repeat;

                    ret = picoquic_check_frame_needs_repeat(cnx,
                        bytes + byte_index, consumed, picoquic_packet_1rtt_protected, &no_need_to_repeat, &do_not_detect_spurious, 0);
                    if (no_need_to_repeat && !stream_ack_case[i].should_ack) {
                        DBG_PRINTF("Case %zu, failed to repeat frame index %zu",
                            i, byte_index);
                        ret = -1;
                        break;
                    } else if (!no_need_to_repeat && stream_ack_case[i].should_ack) {
                        DBG_PRINTF("Case %zu, unneeded repeat frame index %zu",
                            i, byte_index);
                        ret = -1;
                        break;
                    }
                    byte_index += consumed;
                }
            }
        }
    }

    if (quic != NULL) {
        picoquic_free(quic);
    }

    return ret;
}
```

### Rust test body
```rust
fn stream_ack() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    stream_ack_test_one(&mut quic).expect("stream_ack");
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

## `picoquictest/tls_api_test.c:discard_stream_test`
* C test-table name: `discard_stream`
* C entry function: `discard_stream_test`
* Rust test: `discard_stream`
* C source: `picoquictest/tls_api_test.c:4913-4917`
* Rust source: `rs/fq/src/tests/tls_api.rs:300-302`

### C test body
```c
{
    int ret = stop_sending_test_one(1, 0);
    return ret;
}
```

### Rust test body
```rust
fn discard_stream() {
    stop_sending_test_one(true, false).expect("discard_stream");
}
```

## `picoquictest/tls_api_test.c:grease_quic_bit_one_way_test`
* C test-table name: `grease_quic_bit_one_way`
* C entry function: `grease_quic_bit_one_way_test`
* Rust test: `grease_quic_bit_one_way`
* C source: `picoquictest/tls_api_test.c:11032-11035`
* Rust source: `rs/fq/src/tests/tls_api.rs:375-377`

### C test body
```c
{
    return  grease_quic_bit_test_one(1);
}
```

### Rust test body
```rust
fn grease_quic_bit_one_way() {
    grease_quic_bit_test_one(true).expect("grease_quic_bit_one_way");
}
```

## `picoquictest/tls_api_test.c:initial_race_test`
* C test-table name: `initial_race`
* C entry function: `initial_race_test`
* Rust test: `initial_race`
* C source: `picoquictest/tls_api_test.c:10772-10862`
* Rust source: `rs/fq/src/tests/tls_api.rs:446-448`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    }

    /* Run an initial loop to make to send the client's first packet, and then replicate it. */
    if (ret == 0) {
        int was_active = 0;
        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

        if (ret == 0) {
            /* Force a repeat of the first packet */
            simulated_time += 100;
            test_ctx->cnx_client->initial_repeat_needed = 1;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            test_ctx->cnx_client->initial_repeat_needed = 0;
            /* Verify that there are two packets in the initial queue */
            if (ret == 0) {
                if (test_ctx->c_to_s_link->first_packet == NULL) {
                    DBG_PRINTF("%s", "No packet queued");
                    ret = -1;
                }
                else if (test_ctx->c_to_s_link->last_packet == test_ctx->c_to_s_link->first_packet) {
                    DBG_PRINTF("%s", "Only one packet queued");
                    ret = -1;
                }
            }
        }

        while (ret == 0 && test_ctx->s_to_c_link->first_packet == NULL){
            /* run a couple of simulation round to process the first server packets,
             * but make sure the server sends only one packet */
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
        }

        if (ret == 0) {
            if (test_ctx->cnx_server == NULL) {
                DBG_PRINTF("%s", "No server connection");
                ret = -1;

            }
            else {
                /* Make sure that the server waits before sending the next packet. */
                test_ctx->cnx_server->next_wake_time += 2000;
            }
        }
    }

    /* Run a connection loop */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2));
    }

    /* Try send data */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* Check that the data was sent and received */
    if (ret == 0) {
        ret = tls_api_one_scenario_verify(test_ctx);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);

        if (ret != 0)
        {
            DBG_PRINTF("Connection close returns %d\n", ret);
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
fn initial_race() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("initial_race");
}
```
