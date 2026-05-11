# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/satellite_test.c:satellite_preemptive_test`
* C test-table name: `satellite_preemptive`
* C entry function: `satellite_preemptive_test`
* Rust test: `satellite_preemptive`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:294-309`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The top-level BBR, loss, preemptive, and timing parameters match, and Rust keeps the preemptive-repeat count checks. However, the same helper issue means the 100 MB lossy satellite stream transfer and max completion time are not actually exercised, so the Rust test is materially weaker than the C test.
* Phase 5A fix note: Fix the satellite helper/call path to perform the real 100 MB stream0 transfer under loss and enforce completion time, then keep the existing preemptive-repeat assertions after that transfer.
* Phase 5B analysis: Rust #[test] is present, compiles under the Rust test harness, and matches the C API-level contract/parameters. The known early Protocol(1060) runtime failure is a Phase 5C library-behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Variation of the loss test, using preemptive repeat*/
    /* Should be less than 10 sec per draft etosat.  */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 7100000, 250, 3, 0, 1, 1, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_preemptive() {
    let bbr = satellite_ccalgo("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        7_100_000,
        250,
        3,
        0,
        true,
        true,
        false,
        false,
        false,
    );
}
```

## `picoquictest/skip_frame_test.c:frames_repeat_test`
* C test-table name: `frames_repeat`
* C entry function: `frames_repeat_test`
* Rust test: `frames_repeat`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:3802-3856`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only smoke-tests one full PING frame. The C test iterates the full test_skip_list, preserves each case's epoch/mpath/pure-ACK metadata, and checks both valid full frames and important truncated/error-repeat cases with frame-type exclusions.
* Phase 5A fix note: Extend the Rust test/fixture to cover every skip-frame case with epoch, mpath, is_pure_ack, and nb_varints metadata; for each case run the full-frame check and the same len-1/type-only truncated checks with the C exclusion list.
* Phase 5B analysis: Rust `frames_repeat` already matches the C API-level contract: full-frame repeat checks plus non-pure-ACK truncation checks, C exclusions, and type-only fallback when varints exist. Any `check_frame_needs_repeat` runtime mismatch is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint64_t simulated_time = 0;
    picoquic_quic_t* qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    struct sockaddr_in saddr = { 0 };

    if (qclient == NULL) {
        ret = -1;
    }
    else {
        for (size_t i = 0; ret == 0 && i < nb_test_skip_list; i++) {
            size_t len = test_skip_list[i].len;
            uint64_t frame_type = 0;
            const uint8_t* type_byte = NULL;
            if ((type_byte = picoquic_frames_varint_decode(test_skip_list[i].val, test_skip_list[i].val + test_skip_list[i].len, &frame_type)) != NULL) {
                memcpy(buffer, test_skip_list[i].val, len);
                if (frame_repeat_error_packet(qclient, (struct sockaddr*)&saddr, simulated_time, buffer, len,
                    test_skip_list[i].epoch, test_skip_list[i].mpath, 0) != 0) {
                    ret = -1;
                }
                else if (len > 1 && !test_skip_list[i].is_pure_ack) {
                    switch (frame_type) {
                    case picoquic_frame_type_connection_close:
                    case picoquic_frame_type_application_close:
                    case picoquic_frame_type_new_token:
                    case picoquic_frame_type_path_abandon:
                    case picoquic_frame_type_bdp:
                    case picoquic_frame_type_observed_address_v4:
                    case picoquic_frame_type_observed_address_v6:
                        break;
                    default:
                        if (frame_repeat_error_packet(qclient, (struct sockaddr*)&saddr, simulated_time, buffer, len - 1,
                            test_skip_list[i].epoch, test_skip_list[i].mpath, 1) != 0) {
                            if (test_skip_list[i].nb_varints > 0) {
                                /* Try again with shorter length */
                                size_t type_len = type_byte - test_skip_list[i].val;
                                if (frame_repeat_error_packet(qclient, (struct sockaddr*)&saddr, simulated_time, buffer, type_len,
                                    test_skip_list[i].epoch, test_skip_list[i].mpath, 1) != 0) {
                                    ret = -1;
                                }
                            }
                            else {
                                ret = -1;
                            }
                        }
                    }
                }
            }
        }
        picoquic_free(qclient);
    }
    return ret;
}
```

### Current Rust test body
```rust
fn frames_repeat() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let frames = test_skip_frames();

    assert_eq!(frames.len(), TEST_SKIP_FRAME_VARINT_COUNTS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_EPOCHS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_MPATH.len());

    for (i, case) in frames.iter().enumerate() {
        let Some((ftype, type_len)) = frame_type_and_len(&case.bytes) else {
            continue;
        };
        frame_repeat_error_packet(
            &mut quic,
            &case.bytes,
            TEST_SKIP_FRAME_EPOCHS[i],
            TEST_SKIP_FRAME_MPATH[i],
            false,
        )
        .unwrap_or_else(|err| panic!("repeat full frame <{}> failed: {:?}", case.name, err));

        if case.bytes.len() > 1 && case.pure_ack == 0 && !frame_repeat_truncation_excluded(ftype) {
            let truncated = &case.bytes[..case.bytes.len() - 1];
            if frame_repeat_error_packet(
                &mut quic,
                truncated,
                TEST_SKIP_FRAME_EPOCHS[i],
                TEST_SKIP_FRAME_MPATH[i],
                true,
            )
            .is_err()
            {
                assert!(
                    TEST_SKIP_FRAME_VARINT_COUNTS[i] > 0,
                    "repeat truncated frame <{}> failed without C type-only fallback",
                    case.name
                );
                frame_repeat_error_packet(
                    &mut quic,
                    &case.bytes[..type_len],
                    TEST_SKIP_FRAME_EPOCHS[i],
                    TEST_SKIP_FRAME_MPATH[i],
                    true,
                )
                .unwrap_or_else(|err| {
                    panic!(
                        "repeat type-only frame <{}> failed after truncated fallback: {:?}",
                        case.name, err
                    )
                });
            }
        }
    }
}
```

## `picoquictest/stream0_frame_test.c:stream_output_test`
* C test-table name: `stream_output`
* C entry function: `stream_output_test`
* Rust test: `stream_output`
* Expected Rust file: `rs/fq/src/tests/stream0_frame.rs`
* Current Rust span: `rs/fq/src/tests/stream0_frame.rs:745-747`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Ordering and readiness checks mostly match, but the Rust deletion helper directly removes streams from the arena/output list instead of exercising the translated remove-output/delete-if-closed behavior that the C test explicitly verifies.
* Phase 5A fix note: Change the Rust deletion path to use the translated output-stream removal and delete-if-closed logic, preserving the C flags, deletion order, and postcondition checks. Optionally align stream creation with the C loop that excludes stream 7.
* Phase 5B analysis: Rust test compiles as a harness test and already expresses the C API-level contract, including output-list ordering, ready-stream checks, translated removal/delete-if-closed calls, and deletion postconditions. Any failure from delete_stream_if_closed not deleting closed streams is a Phase 5C implementation issue.
* Phase 5B fix note: 

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

### Current Rust test body
```rust
fn stream_output() {
    stream_output_test_body().expect("stream_output_test");
}
```

## `picoquictest/tls_api_test.c:bad_certificate_test`
* C test-table name: `bad_certificate`
* C entry function: `bad_certificate_test`
* Rust test: `bad_certificate`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1076-1119`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs the generic successful TLS helper with a different SNI string; it does not recreate the server with the bad certificate and does not assert disconnection or handshake-error states.
* Phase 5A fix note: Recreate the Rust server context with TEST_FILE_SERVER_BAD_CERT, run the connection loop expecting failure, and assert the client is disconnected with handshake errors on both client and server sides.
* Phase 5B analysis: Rust test is present, compiles/runs under the Rust test harness, and matches the C API-level contract: bad-cert server context recreation, ignored connection-loop result, client disconnected, client local handshake error, and server remote handshake error. Runtime failure to create the server-side connection is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

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

### Current Rust test body
```rust
fn bad_certificate() {
    const TEST_TICKET_ENCRYPT_KEY: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    test_ctx.qserver = Quic::new(
        8,
        Some(TEST_FILE_SERVER_BAD_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        Some(&TEST_TICKET_ENCRYPT_KEY),
    )
    .expect("bad-cert server context");

    let _ = tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time);

    assert!(test_ctx.has_cnx_server(), "server connection should exist");
    let (client_state, client_local_error) = {
        let client = test_ctx.cnx_client();
        (client.state(), client.local_error())
    };
    assert_eq!(client_state, State::Disconnected);
    assert!(
        is_handshake_error(client_local_error),
        "client local error should be a handshake error, got {client_local_error:#x}"
    );

    let server_remote_error = test_ctx.cnx_server().remote_error();
    assert!(
        is_handshake_error(server_remote_error),
        "server remote error should be a handshake error, got {server_remote_error:#x}"
    );
}
```

## `picoquictest/tls_api_test.c:request_client_authentication_test`
* C test-table name: `client_auth`
* C entry function: `request_client_authentication_test`
* Rust test: `client_auth`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1429-1438`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test passes only two paths and uses the default server certificate/key as client credentials, while the C test uses separate RSA client cert/key, default server cert/key, and CA store. The Rust helper also lacks the C test's explicit client/server ready checks.
* Phase 5A fix note: Expand the Rust helper to accept client cert/key, server cert/key, and CA store; pass the RSA client credentials plus default server credentials/CA store; assert both client and server connections reach ready state.
* Phase 5B analysis: Rust test and helper already match the C API-level contract: same RSA client credentials, default server credentials, CA store, recreated client/server contexts, client-only client, server client-auth toggle, client start, connection loop, and client/server ready assertions. Any failure to reach ready is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

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
                                  picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_CERT_RSA);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_client_key_file, sizeof(test_client_key_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY_RSA);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_CERT);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_ca_cert_store_file, sizeof(test_ca_cert_store_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
    }

    if (ret == 0) {
        ret = request_client_authentication_test_one(test_client_cert_file, test_client_key_file,
                                                     test_server_cert_file, test_server_key_file,
                                                     test_ca_cert_store_file);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "mTLS client-auth test failed RSA\n");
    }

    return ret;
}
```

### Current Rust test body
```rust
fn client_auth() {
    request_client_authentication_test_one(
        TEST_FILE_SERVER_CERT_RSA,
        TEST_FILE_SERVER_KEY_RSA,
        TEST_FILE_SERVER_CERT,
        TEST_FILE_SERVER_KEY,
        TEST_FILE_CERT_STORE,
    )
    .expect("client_auth");
}
```
