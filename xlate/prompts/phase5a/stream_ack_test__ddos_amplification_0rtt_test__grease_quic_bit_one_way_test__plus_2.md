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

## `picoquictest/tls_api_test.c:ddos_amplification_0rtt_test`
* C test-table name: `ddos_amplification_0rtt`
* C entry function: `ddos_amplification_0rtt_test`
* Rust test: `ddos_amplification_0rtt`
* C source: `picoquictest/tls_api_test.c:10415-10418`
* Rust source: `rs/fq/src/tests/tls_api.rs:262-264`

### C test body
```c
{
    return ddos_amplification_test_one(1, 0);
}
```

### Rust test body
```rust
fn ddos_amplification_0rtt() {
    ddos_amplification_test_one(1, 0).expect("ddos_amplification_0rtt");
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

## `picoquictest/tls_api_test.c:long_rtt_test`
* C test-table name: `long_rtt`
* C entry function: `long_rtt_test`
* Rust test: `long_rtt`
* C source: `picoquictest/tls_api_test.c:9557-9593`
* Rust source: `rs/fq/src/tests/tls_api.rs:533-539`

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    uint64_t latency = 300000ull; /* assume that each direction is 300 ms, e.g. satellite link */
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x10, 0x10, 30, 0, 0, 0, 0, 0}, 8 };

    ret = tls_api_one_scenario_init_ex(&test_ctx, &simulated_time,
        0, NULL, NULL, &initial_cid);

    if (ret == 0) {
        /* set the delay estimate, then launch the test */
        test_ctx->c_to_s_link->microsec_latency = latency;
        test_ctx->s_to_c_link->microsec_latency = latency;

        picoquic_set_qlog(test_ctx->qserver, ".");

        /* The transmission delay cannot be less than 2.6 sec:
         * 3 handshakes at 1 RTT each = 1.8 sec, plus
         * 1MB over a 10Mbps link = 0.8 sec. We observe
         * 3.31 seconds instead, i.e. 1.51 sec for the
         * data transmission. This is due to the slow start
         * phase of the congestion control, which we accelerated
         * but could not completely fix. */
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 0, 2*latency,
            3600000);
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
fn long_rtt() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    ctx.c_to_s_link.microsec_latency = 300_000;
    ctx.s_to_c_link.microsec_latency = 300_000;
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 10_000_000).expect("long_rtt");
}
```
