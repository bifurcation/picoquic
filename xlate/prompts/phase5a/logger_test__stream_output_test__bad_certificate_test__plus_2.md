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

## `picoquictest/skip_frame_test.c:logger_test`
* C test-table name: `logger`
* C entry function: `logger_test`
* Rust test: `logger`
* C source: `picoquictest/skip_frame_test.c:1969-2137`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1136-1138`

### C test body
```c
{
    FILE* F = NULL;
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t fuzz_buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint64_t random_context = 0xF00BAB;
    struct sockaddr_in6 saddr = { 0 };
    picoquic_cnx_t * cnx = NULL;
    picoquic_quic_t * quic = NULL;
    uint64_t simulated_time = 123456789;
    uint64_t running_sum = 0;

    quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    saddr.sin6_family = AF_INET6;
    saddr.sin6_port = 443;
    memset(&saddr.sin6_addr, 0x20, 16);

    if (quic == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }
    else if ((cnx = picoquic_create_cnx(quic, logger_test_cid, logger_test_cid, (struct sockaddr*)&saddr,
        simulated_time, 0, "test-sni", "test-alpn", 1)) == NULL) {
        DBG_PRINTF("%s", "Cannot create CNX context\n");
        ret = -1;
    }
    else if (picoquic_set_textlog(quic, log_test_file) != 0) {
        DBG_PRINTF("failed to open file:%s\n", log_test_file);
        ret = -1;
    }
    else {
        for (size_t i = 0; i < nb_test_skip_list; i++) {
            picoquic_textlog_frames(quic->F_log, 0, test_skip_list[i].val, test_skip_list[i].len);
        }
        for (size_t i = 0; i < nb_test_frame_error_list; i++) {
            picoquic_textlog_frames(quic->F_log, 0, test_frame_error_list[i].val, test_frame_error_list[i].len);
        }
        fprintf(quic->F_log, "\n");
        picoquic_log_tls_ticket(cnx,
            log_test_ticket, (uint16_t) sizeof(log_test_ticket));

        picoquic_log_app_message(cnx, "%s.", "This is an app message test");
        picoquic_log_app_message(cnx, "This is app message test #%d, severity %d.", 1, 2);

        fprintf(quic->F_log, "\n");
        logger_test_packets(cnx);
        logger_test_pdus(quic, cnx);

        quic->F_log = picoquic_file_close(quic->F_log);
    }

    if (ret == 0) {
        char log_test_ref[512];

        ret = picoquic_get_input_path(log_test_ref, sizeof(log_test_ref), picoquic_solution_dir, LOG_TEST_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the log ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(log_test_file, log_test_ref);
        }
    }

    /* Create a set of randomized packets. Verify that they can be logged without
     * causing the dreaded "Unknown frame" message */

    for (size_t i = 0; ret == 0 && i < 100; i++) {
        char log_line[1024];
        size_t bytes_max = format_random_packet(buffer, sizeof(buffer), &random_context, -1);

        if (picoquic_set_textlog(quic, log_packet_test_file) != 0) {
            DBG_PRINTF("failed to open file:%s\n", log_packet_test_file);
            ret = -1;
        }
        else {
            ret &= fprintf(quic->F_log, "Log packet test #%d\n", (int)i);
            picoquic_textlog_frames(quic->F_log, 0, buffer, bytes_max);
            quic->F_log = picoquic_file_close(quic->F_log);
        }

        if ((F = picoquic_file_open(log_packet_test_file, "r")) == NULL) {
            DBG_PRINTF("failed to open file:%s\n", log_packet_test_file);
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        } else {
            while (fgets(log_line, (int)sizeof(log_line), F) != NULL) {
                /* skip blanks */
                size_t byte_index = 0;

                while (byte_index < sizeof(log_line) &&
                    (log_line[byte_index] == ' ' || log_line[byte_index] == '\t')) {
                    byte_index++;
                }

                if (byte_index + 7u < sizeof(log_line) &&
                    memcmp(&log_line[byte_index], "Unknown", 7) == 0)
                {
                    DBG_PRINTF("Packet log test #%d failed, unknown frame.\n", (int)i);
                    ret = -1;
                    break;
                }
            }
            (void)picoquic_file_close(F);
        }
    }

    /* Log a series of known bad packets  */
    for (size_t i = 0; ret == 0 && i < nb_test_frame_error_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            uint8_t extra_bytes[4] = { 0, 0, 0, 0 };
            size_t bytes_max = 0;

            if (picoquic_set_textlog(quic, log_error_test_file) != 0) {
                DBG_PRINTF("failed to open file:%s\n", log_error_test_file);
                ret = -1;
                break;
            }
            fprintf(quic->F_log, "Running_sum: %" PRIx64 "\n", running_sum);
            memcpy(buffer, test_frame_error_list[i].val, test_frame_error_list[i].len);
            bytes_max = test_frame_error_list[i].len;
            if (test_frame_error_list[i].must_be_last == 0 && sharp_end == 0) {
                /* add some padding to check that the end of frame is detected properly */
                memcpy(buffer + bytes_max, extra_bytes, sizeof(extra_bytes));
                bytes_max += sizeof(extra_bytes);
            }

            picoquic_textlog_frames(quic->F_log, 0, buffer, bytes_max);

            quic->F_log = picoquic_file_close(quic->F_log);
            running_sum += picoquic_sum_text_file(log_error_test_file);
        }
    }

    /* Do a minimal fuzz test */
    for (size_t i = 0; ret == 0 && i < 100; i++) {
        size_t bytes_max = format_random_packet(buffer, sizeof(buffer), &random_context, -1);

        if (picoquic_set_textlog(quic, log_fuzz_test_file) != 0) {
            DBG_PRINTF("failed to open file:%s\n", log_fuzz_test_file);
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        }

        ret &= (fprintf(quic->F_log, "Log fuzz test #%d, sum: %" PRIx64 "\n",
            (int)i, running_sum) > 0);
        picoquic_textlog_frames(quic->F_log, 0, buffer, bytes_max);

        /* Attempt to log fuzzed packets, and hope nothing crashes */
        for (size_t j = 0; j < 100; j++) {
            ret &= fprintf(quic->F_log, "Log fuzz test #%d, packet %d\n", (int)i, (int)j);
            fflush(quic->F_log);
            skip_test_fuzz_packet(fuzz_buffer, buffer, bytes_max, &random_context);
            picoquic_textlog_frames(quic->F_log, 0, fuzz_buffer, bytes_max);
        }
        quic->F_log = picoquic_file_close(quic->F_log);
        running_sum += picoquic_sum_text_file(log_fuzz_test_file);
    }

    if (quic != NULL) {
        picoquic_free(quic);
    }

    return ret;
}
```

### Rust test body
```rust
fn logger() {
    run_logger_test().expect("logger_test");
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

## `picoquictest/tls_api_test.c:bad_certificate_test`
* C test-table name: `bad_certificate`
* C entry function: `bad_certificate_test`
* Rust test: `bad_certificate`
* C source: `picoquictest/tls_api_test.c:5224-5296`
* Rust source: `rs/fq/src/tests/tls_api.rs:47-50`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_server_cert_store_file[512];
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_BAD_CERT);

        if (ret == 0) {
            ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
        }

        if (ret == 0) {
            ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
        }

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
        }
    }

    /* Delete the server context, and recreate it with the bad certificate */

    if (ret == 0)
    {
        if (test_ctx->qserver != NULL) {
            picoquic_free(test_ctx->qserver);
        }

        test_ctx->qserver = picoquic_create(8,
            test_server_cert_file, test_server_key_file, test_server_cert_store_file,
            PICOQUIC_TEST_ALPN, test_api_callback, (void*)&test_ctx->server_callback, NULL, NULL, NULL,
            simulated_time, &simulated_time, NULL,
            test_ticket_encrypt_key, sizeof(test_ticket_encrypt_key));

        if (test_ctx->qserver == NULL) {
            ret = -1;
        }
    }

    /* Proceed with the connection loop. It should fail, and thus we don't test the return code */
    if (ret == 0) {
        (void)tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_local_error(test_ctx->cnx_client))) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_remote_error(test_ctx->cnx_server))) {
            ret = -1;
        }
        else {
            ret = 0;
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
fn bad_certificate() {
    tls_api_test_with_loss(None, V1, Some("bad.example.com"), Some(TEST_ALPN))
        .expect("bad_certificate");
}
```

## `picoquictest/tls_api_test.c:set_verify_certificate_callback_test`
* C test-table name: `client_cert_callback`
* C entry function: `set_verify_certificate_callback_test`
* Rust test: `client_cert_callback`
* C source: `picoquictest/tls_api_test.c:5334-5425`
* Rust source: `rs/fq/src/tests/tls_api.rs:150-153`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_server_cert_store_file[512];
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    static const uint16_t default_algos[] = {
        PTLS_SIGNATURE_ED25519, PTLS_SIGNATURE_RSA_PSS_RSAE_SHA256,
        PTLS_SIGNATURE_ECDSA_SECP256R1_SHA256, PTLS_SIGNATURE_RSA_PKCS1_SHA256, 
        PTLS_SIGNATURE_RSA_PKCS1_SHA1, UINT16_MAX };

    verify_certificate_test_cb_t verify_cb = { 0 };

    verify_cb.super.cb = verify_certificate_test_cb;
    verify_cb.super.algos = default_algos;

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_CERT);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }

    /* Delete the client context, and recreate with a certificate */
    if (ret == 0) {
        if (test_ctx->qclient != NULL) {
            picoquic_free(test_ctx->qclient);
            test_ctx->cnx_client = NULL;
        }

        test_ctx->qclient = picoquic_create(8,
            test_server_cert_file, test_server_key_file, test_server_cert_store_file,
            NULL, test_api_callback, (void*)&test_ctx->client_callback, NULL, NULL, NULL,
            simulated_time, &simulated_time, NULL, NULL, 0);

        if (test_ctx->qclient == NULL) {
            ret = -1;
        }
    }
    /* recreate the client connection */
    if (ret == 0) {
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient, picoquic_null_connection_id,
                                                   picoquic_null_connection_id,
                                                   (struct sockaddr*)&test_ctx->server_addr, 0,
                                                   0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        } else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }
    /* Set the verify callback for the client */
    if (ret == 0) {
        picoquic_set_verify_certificate_callback(test_ctx->qclient, &verify_cb.super, NULL);
    }

    /* Set the verify callback for the server */
    if (ret == 0) {
        picoquic_set_verify_certificate_callback(test_ctx->qserver, &verify_cb.super, NULL);
    }
    /* Activate client authentication */
    if (ret == 0) {
        picoquic_set_client_authentication(test_ctx->qserver, 1);

        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0 && verify_cb.callcount != 2) {
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
fn client_cert_callback() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("client_cert_callback");
}
```

## `picoquictest/tls_api_test.c:transmit_cnxid_retire_before_test`
* C test-table name: `cnxid_transmit_r_before`
* C entry function: `transmit_cnxid_retire_before_test`
* Rust test: `cnxid_transmit_r_before`
* C source: `picoquictest/tls_api_test.c:6621-6624`
* Rust source: `rs/fq/src/tests/tls_api.rs:221-223`

### C test body
```c
{
    return transmit_cnxid_test_one(1, 0, 0);
}
```

### Rust test body
```rust
fn cnxid_transmit_r_before() {
    transmit_cnxid_test_one(true, false, false).expect("cnxid_transmit_r_before");
}
```
