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

## `picoquictest/transport_param_test.c:transport_param_log_test`
* C test-table name: `transport_param_log`
* C entry function: `transport_param_log_test`
* Rust test: `transport_param_log`
* Expected Rust file: `rs/fq/src/tests/transport_param.rs`
* Current Rust span: `rs/fq/src/tests/transport_param.rs:1047-1050`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust helper includes the eight fixed TP vectors, reference-file comparison, and client/server fuzz loops, but it logs via a nested test-only textlog_transport_extension_content implementation. The C test exercises the production picoquic_textlog_transport_extension_content, so the Rust test can pass without checking the translated textlog logger behavior.
* Phase 5A fix note: Have the Rust test/helper call the production Rust textlog transport-extension logger, or add/expose that translated implementation in rs/fq/src/textlog.rs and route the same fixed vectors and fuzz cases through it. Keep the reference comparison and client_param2/server_param2 fuzz coverage.
* Phase 5B analysis: Rust test/helper already expresses the C API-level contract, including fixed-vector logging, reference-file comparison, and client_param2/server_param2 fuzz logging. It cannot compile as a runnable Rust test because the required production API crate::textlog::textlog_transport_extension_content is missing/exposed nowhere; this is an API-surface blocker, not a Phase 5C runtime failure.
* Phase 5B fix note: 

### C test body
```c
{
    FILE* F = NULL;
    int ret = 0;

    if ((F = picoquic_file_open(log_tp_test_file, "w")) == NULL) {
        fprintf(stderr, "failed to open file:%s\n", log_tp_test_file);
        ret = PICOQUIC_ERROR_INVALID_FILE;
    }

    if (F != NULL) {
        char log_tp_test_ref[512];

        transport_param_log_test_one(F, client_param1, sizeof(client_param1));
        transport_param_log_test_one(F, client_param2, sizeof(client_param2));
        transport_param_log_test_one(F, client_param3, sizeof(client_param3));
        transport_param_log_test_one(F, server_param1, sizeof(server_param1));
        transport_param_log_test_one(F, server_param2, sizeof(server_param2));
        transport_param_log_test_one(F, client_param4, sizeof(client_param4));
        transport_param_log_test_one(F, client_param5, sizeof(client_param5));
        transport_param_log_test_one(F, server_param3, sizeof(server_param3));

        fclose(F);

        ret = picoquic_get_input_path(log_tp_test_ref, sizeof(log_tp_test_ref), picoquic_solution_dir, LOG_TP_TEST_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the log TP ref file name.\n");
        } else {
            ret = picoquic_test_compare_text_files(log_tp_test_file, log_tp_test_ref);
        }
    }

    if (ret == 0)
    {
        DBG_PRINTF("Doing fuzz test of transport parameter logging into %s\n", log_tp_fuzz_file);

        ret = transport_param_log_fuzz_test(client_param2, sizeof(client_param2));

        if (ret == 0) {
            ret = transport_param_log_fuzz_test(server_param2, sizeof(server_param2));
        }

        DBG_PRINTF("Fuzz test of transport parameter was successful.\n", log_tp_fuzz_file);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn transport_param_log() {
    transport_param_log_test_one("log_tp_test.txt").expect("log_tp");
    compare_text_files("log_tp_test.txt", "picoquictest/log_tp_test_ref.txt").expect("compare_log");
}
```

## `picoquictest/edge_cases.c:ec9a_preemptive_amok_test`
* C test-table name: `ec9a_preemptive_amok`
* C entry function: `ec9a_preemptive_amok_test`
* Rust test: `ec9a_preemptive_amok`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1708-1727`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust checks server existence, ready state, test_finished, send_count cap, idle-time cap, and nonzero preemptive repeats, but it omits the C precondition that the application pending queue is nonempty before the server-only loop and does not mirror the explicit wake-list reinsertion before that loop.
* Phase 5A fix note: Add the pre-loop application pending-queue assertion and mirror/recreate picoquic_reinsert_by_wake_time behavior before running the server-only loop.
* Phase 5B analysis: Rust test now mirrors the C pending application-packet precondition and explicit server wake reinsertion before the server-only repeat loop. Narrow execution still fails earlier because edge_case_prepare currently leaves no server connection for this case.
* Phase 5B fix note: Added an application pending-queue assertion and a test helper that removes, retimes, and reinserts the server connection in the wake list before running the ec9a server-only loop.

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t initial_losses = 0x800;
    uint8_t test_case_id = 0x9a;
    uint64_t cnx_server_idle_timeout = 0;
    uint64_t cnx_server_nb_preemptive_repeat = 0;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 0, &simulated_time, initial_losses, 12);

    if (ret == 0) {
        if (test_ctx->cnx_server == NULL) {
            DBG_PRINTF("Unexpected state, client: %d, server: NULL",
                test_ctx->cnx_client->cnx_state);
            ret = -1;
        }
        else if ( test_ctx->cnx_server->cnx_state != picoquic_state_ready ||
            !test_ctx->test_finished || 
            test_ctx->cnx_server->pkt_ctx[picoquic_packet_context_application].pending_first == NULL){
            DBG_PRINTF("Unexpected state, server: %d, test finished: %d, queue for repeat %s",
                test_ctx->cnx_server->cnx_state, test_ctx->test_finished, 
                (test_ctx->cnx_server->pkt_ctx[picoquic_packet_context_application].pending_first == NULL)?"empty":"full");
            ret = -1;
        }
    }
    /* Do a loop involving only the server */
    if (ret == 0) {
        uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
        size_t send_length;
        size_t send_msg_size;
        struct sockaddr_storage addr_to;
        struct sockaddr_storage addr_from;
        int if_index;
        picoquic_connection_id_t log_id;
        picoquic_cnx_t * last_cnx;
        int loop_count = 0;
        int send_count = 0;
        const int send_count_max = 50;
        uint64_t repeat_begin = simulated_time;
        uint64_t repeat_duration = 0;

        cnx_server_idle_timeout = test_ctx->cnx_server->idle_timeout;
        cnx_server_nb_preemptive_repeat = test_ctx->cnx_server->nb_preemptive_repeat;

        picoquic_reinsert_by_wake_time(test_ctx->qserver, test_ctx->cnx_server, simulated_time);

        while (test_ctx->qserver->current_number_connections > 0 && test_ctx->cnx_server->cnx_state == picoquic_state_ready && loop_count < 10000 && ret == 0) {
            loop_count++;
            cnx_server_nb_preemptive_repeat = test_ctx->cnx_server->nb_preemptive_repeat;
            simulated_time = picoquic_get_next_wake_time(test_ctx->qserver, simulated_time);
            ret = picoquic_prepare_next_packet_ex(test_ctx->qserver, simulated_time, buffer,
                sizeof(buffer), &send_length, &addr_to, &addr_from, &if_index, &log_id,
                &last_cnx, &send_msg_size);
            if (ret != 0) {
                DBG_PRINTF("Prepare next returns an error: %d (0x%x)", ret, ret);
            }
            else if (send_length > 0) {
                send_count++;
            }
        }

        if (ret == 0) {
            repeat_duration = simulated_time - repeat_begin;
            if (send_count > send_count_max) {
                DBG_PRINTF("Repeated %d packets, more that the %d expected",
                    send_count, send_count_max);
                ret = -1;
            }
            else if (repeat_duration > cnx_server_idle_timeout) {
                DBG_PRINTF("End at t=%" PRIu64 ", later than %" PRIu64,
                    simulated_time, cnx_server_idle_timeout);
                ret = -1;
            }
            else if (cnx_server_nb_preemptive_repeat == 0) {
                DBG_PRINTF("%s", "No preemptive repeat");
                ret = -1;
            }
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
fn ec9a_preemptive_amok() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0x9a, false, &mut simulated_time, 0x800, 12).expect("edge_case_prepare");
    assert!(test_ctx.has_cnx_server(), "server connection must exist");
    assert!(test_ctx.server_ready(), "server must be in ready state");
    assert!(test_ctx.test_finished, "data transfer must have completed");
    assert!(
        ec9a_server_application_pending(&mut test_ctx),
        "server application pending queue must be nonempty before repeat loop"
    );
    let (send_count, repeat_duration) =
        ec9a_server_loop(&mut test_ctx, &mut simulated_time).expect("server loop");
    assert!(
        send_count <= 50,
        "server sent too many repeat packets: {send_count}"
    );
    // repeat_duration must be <= idle_timeout (checked inside ec9a_server_loop)
    let _ = repeat_duration;
}
```

## `picoquictest/mbedtls_test.c:mbedtls_retrieve_pubkey_test`
* C test-table name: `mbedtls_retrieve_pubkey`
* C entry function: `mbedtls_retrieve_pubkey_test`
* Rust test: `mbedtls_retrieve_pubkey`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:665-675`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers the same four key/certificate fixture pairs, but the helper is placeholder-like: it checks PEM markers and matching path families instead of initializing mbedTLS, loading the private key, extracting certificate public-key bits, and byte-comparing them to the exported private-key public key.
* Phase 5A fix note: Replace mbedtls_test_retrieve_pubkey_one with a faithful public-key extraction comparison, including mbedTLS init/free semantics or appropriate sys-mbedtls gating, and keep all four RSA/secp fixture cases.
* Phase 5B analysis: Rust helper was weaker than C; it now compares actual private-key public bytes against certificate SubjectPublicKeyInfo bytes for all four RSA/secp fixture pairs.
* Phase 5B fix note: Added PEM/DER parsing helpers, extracted RSA/EC public-key bytes from private keys and certificates, replaced marker/path-family checks with byte equality, and bracketed the test with TLS provider reset cleanup.

### C test body
```c
{
    int ret = 0;
    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = test_retrieve_pubkey_one(ASSET_RSA_KEY, ASSET_RSA_CERT);
        }

        if (ret == 0) {
            ret = test_retrieve_pubkey_one(ASSET_SECP256R1_KEY, ASSET_SECP256R1_CERT);
        }

        if (ret == 0) {
            ret = test_retrieve_pubkey_one(ASSET_SECP384R1_KEY, ASSET_SECP384R1_CERT);
        }

        if (ret == 0) {
            ret = test_retrieve_pubkey_one(ASSET_SECP521R1_KEY, ASSET_SECP521R1_CERT);
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }

    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls_retrieve_pubkey() {
    let _tls_api_reset = TlsApiResetGuard::mbedtls_only();
    mbedtls_test_retrieve_pubkey_one("certs/rsa/key.pem", "certs/rsa/cert.pem")
        .expect("rsa pubkey");
    mbedtls_test_retrieve_pubkey_one("certs/secp256r1/key.pem", "certs/secp256r1/cert.pem")
        .expect("secp256r1 pubkey");
    mbedtls_test_retrieve_pubkey_one("certs/secp384r1/key.pem", "certs/secp384r1/cert.pem")
        .expect("secp384r1 pubkey");
    mbedtls_test_retrieve_pubkey_one("certs/secp521r1/key.pem", "certs/secp521r1/cert.pem")
        .expect("secp521r1 pubkey");
}
```

## `picoquictest/quic_tester.c:initial_ping_ack_test`
* C test-table name: `initial_ping_ack`
* C entry function: `initial_ping_ack_test`
* Rust test: `initial_ping_ack`
* Expected Rust file: `rs/fq/src/tests/quic_tester.rs`
* Current Rust span: `rs/fq/src/tests/quic_tester.rs:64-131`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The high-level sequence matches, but Rust tester_simple_ack_frame returns an empty frame, so the Initial and Handshake ACK injections do not test the C behavior. The Rust final-loss verifier also skips the C SNI/ALPN/version/transport checks, and the local ALPN constant differs from C.
* Phase 5A fix note: Encode the simple ACK frame in Rust, fix/use the correct ALPN value, and implement the final transport/SNI/ALPN/version checks in tls_api_test_with_loss_final.
* Phase 5B analysis: ACK injections now exercise the C behavior with a real ACK frame; SNI/ALPN/version/transport final checks are present in the current Rust final helper, and the test uses the shared C-equivalent SNI/ALPN constants.
* Phase 5B fix note: Implemented tester_simple_ack_frame as the C five-varint ACK frame and added an explicit ACK byte check in initial_ping_ack; quic_tester now aliases shared TEST_SNI/TEST_ALPN instead of local drift-prone constants.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint8_t ping_frame[1] = { 1 };
    picoquic_connection_id_t initial_cid = { {0x4e, 0x54, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", PICOQUIC_INTERNAL_TEST_VERSION_1);
    }
    else {
        picoquic_set_qlog(test_ctx->qserver, ".");
        test_ctx->s_to_c_link->microsec_latency = 1;
        test_ctx->c_to_s_link->microsec_latency = 1;
    }

    /*
    Insert a ping frame at the client, pass it to the server.
    */
    if (ret == 0) {
        ret = tester_push_frame_packet(test_ctx,
            picoquic_packet_initial,
            ping_frame, sizeof(ping_frame),
            1, 1, simulated_time);
    }

    /*
    * Wait until the server hello has been received and the handshake key has
    * been computed.
    */
    if (ret == 0) {
        ret = tester_wait_handshake_key(test_ctx, &simulated_time);
    }

    /* Insert Initial ACK packet as specified in issue report. */

    if (ret == 0) {
        uint8_t ack_frame[128];
        size_t ack_frame_length = tester_simple_ack_frame(ack_frame, sizeof(ack_frame), 1);

        ret = tester_push_frame_packet(test_ctx, picoquic_packet_initial,
            ack_frame, ack_frame_length, 1, 0, simulated_time);
    }

    /* Insert Handshake ack packets, as specified.
     */
    if (ret == 0) {
        uint8_t ack_frame[128];
        size_t ack_frame_length = tester_simple_ack_frame(ack_frame, sizeof(ack_frame), 1);
        ret = tester_push_frame_packet(test_ctx, picoquic_packet_handshake,
            ack_frame, ack_frame_length, 0, 0, simulated_time);
    }

    /*
    * Finish the test
    */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);

        if (ret != 0)
        {
            DBG_PRINTF("Connection loop returns %d\n", ret);
        }
    }

    if (ret == 0) {
        ret = tls_api_test_with_loss_final(test_ctx, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time);
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
fn initial_ping_ack() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;

    let initial_cid =
        ConnectionId::clone_from_slice(&[0x4e, 0x54, 0x12, 0x12, 0x12, 0x12, 0x12, 0x12])
            .expect("build initial CID");

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.s_to_c_link.microsec_latency = 1;
    test_ctx.c_to_s_link.microsec_latency = 1;

    let ping_frame: &[u8] = &[1];
    tester_push_frame_packet(
        &mut test_ctx,
        PacketType::Initial,
        ping_frame,
        true,
        true,
        simulated_time,
    )
    .expect("push queued ping packet");

    tester_wait_handshake_key(&mut test_ctx, &mut simulated_time).expect("wait for handshake key");

    let ack_frame = tester_simple_ack_frame(1);
    assert_eq!(ack_frame.as_slice(), &[0x02, 0x01, 0x00, 0x00, 0x00]);
    tester_push_frame_packet(
        &mut test_ctx,
        PacketType::Initial,
        &ack_frame,
        true,
        false,
        simulated_time,
    )
    .expect("push initial ACK packet");

    tester_push_frame_packet(
        &mut test_ctx,
        PacketType::Handshake,
        &ack_frame,
        false,
        false,
        simulated_time,
    )
    .expect("push handshake ACK packet");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    tls_api_test_with_loss_final(
        &mut test_ctx,
        PICOQUIC_TEST_SNI,
        PICOQUIC_TEST_ALPN,
        &mut simulated_time,
    )
    .expect("test with loss final");

    delete_ctx(Some(test_ctx));
}
```

## `picoquictest/skip_frame_test.c:cnxid_stash_test`
* C test-table name: `new_cnxid_stash`
* C entry function: `cnxid_stash_test`
* Rust test: `new_cnxid_stash`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:3946-3989`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only stashes and obtains one CID as a boolean check, while C covers three fixture CIDs, exact sequence/CID/reset-secret comparisons, immediate dequeue, FIFO dequeue after enqueue-all, empty-queue checks, and delete-with-stash cleanup.
* Phase 5A fix note: Add the three C stash fixtures and all three modes. Validate obtained sequence, CID, and reset secret, verify the queue is empty after modes 0 and 1, and cover deleting/dropping a connection with queued stashed CIDs. Use or expose a helper that returns the obtained CID record, not only a boolean.
* Phase 5B analysis: Rust now exercises the three C stash fixtures across immediate dequeue, FIFO dequeue, empty-queue checks, and queued-drop mode using actual stashed CID records.
* Phase 5B fix note: Added C fixture CIDs/secrets, initialized the connection’s local/remote CIDs to the C test values, and replaced the boolean stash check with sequence/CID/reset-secret assertions on obtained records.

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;
    picoquic_quic_t * qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);


    memset(&saddr, 0, sizeof(struct sockaddr_in));
    if (qclient == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }

    /* First test: enqueue and dequeue immediately */
    /* Second test: enqueue all and then dequeue - verify order */
    /* Third test: enqueue all and then delete the connection */
    for (int test_mode = 0; ret == 0 && test_mode < 3; test_mode++) {
        picoquic_cnx_t * cnx = picoquic_create_cnx(qclient,
            picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr *) &saddr,
            simulated_time, 0, "test-sni", "test-alpn", 1);

        picoquic_remote_cnxid_t * stashed = NULL;

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create QUIC CNX context\n");
            ret = -1;
        } else {
            /* init the various connection id to a length compatible with test */
            cnx->path[0]->first_tuple->p_local_cnxid->cnx_id = stash_test_init_local;
            cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id = stash_test_init_remote;
        }

        for (size_t i = 0; ret == 0 && i < nb_stash_test_case; i++) {
            uint64_t transport_error = picoquic_stash_remote_cnxid(cnx, 0, 0,
                stash_test_case[i].sequence, stash_test_case[i].cnx_id.id_len,
                stash_test_case[i].cnx_id.id, stash_test_case[i].reset_secret, &stashed);
            if (transport_error != 0) {
                DBG_PRINTF("Test %d, cannot stash cnxid %d, err 0x%" PRIx64 ".\n", test_mode, i, transport_error);
                ret = -1;
            } else {
                if (stashed == NULL) {
                    DBG_PRINTF("Test %d, cannot stash cnxid %d (duplicate).\n", test_mode, i);
                    ret = -1;
                }
                else if (test_mode == 0) {
                    stashed = picoquic_obtain_stashed_cnxid(cnx, 0);
                    stashed->nb_path_references++;
                    ret = cnxid_stash_compare(test_mode, stashed, i);
                }
            }
        }

        /* Dequeue all in mode 1, verify order */
        if (test_mode == 1) {
            for (size_t i = 0; ret == 0 && i < nb_stash_test_case; i++) {
                stashed = picoquic_obtain_stashed_cnxid(cnx, 0);
                stashed->nb_path_references++;
                ret = cnxid_stash_compare(test_mode, stashed, i);
            }
        }

        /* Verify nothing left in queue in mode 0, 1 */
        if (test_mode < 2) {
            stashed = picoquic_obtain_stashed_cnxid(cnx, 0);
            if (stashed != NULL) {
                DBG_PRINTF("Test %d, unexpected cnxid left, #%d.\n", test_mode, (int)stashed->sequence);
                ret = -1;
            }
        }

        /* Delete the connecton and free the stash */
        picoquic_delete_cnx(cnx);
    }

    if (qclient != NULL) {
        picoquic_free(qclient);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn new_cnxid_stash() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    for test_mode in 0..3 {
        let mut cnx = quic.create_test_cnx(&mut simulated_time).expect("cnx");
        init_cnxid_stash_test_connection(&mut cnx);

        for (case_index, case) in CNXID_STASH_CASES.iter().enumerate() {
            let result = cnx.stash_remote_connection_id(
                0,
                0,
                case.sequence,
                case.connection_id,
                &case.reset_secret,
            );
            assert_eq!(
                result.status, 0,
                "Test {test_mode}, cannot stash cnxid {case_index}."
            );
            assert!(
                result.stashed_index.is_some(),
                "Test {test_mode}, cannot stash cnxid {case_index} (duplicate)."
            );

            if test_mode == 0 {
                assert_obtained_cnxid_stash_case(&mut cnx, test_mode, case_index);
            }
        }

        if test_mode == 1 {
            for case_index in 0..CNXID_STASH_CASES.len() {
                assert_obtained_cnxid_stash_case(&mut cnx, test_mode, case_index);
            }
        }

        if test_mode < 2 {
            assert!(
                cnx.obtain_stashed_connection_id(0).is_none(),
                "Test {test_mode}, unexpected cnxid left."
            );
        }
    }
}
```
