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

## `picoquictest/spinbit_test.c:spinbit_bad_test`
* C test-table name: `spinbit_bad`
* C entry function: `spinbit_bad_test`
* Rust test: `spinbit_bad`
* Expected Rust file: `rs/fq/src/tests/spinbit.rs`
* Current Rust span: `rs/fq/src/tests/spinbit.rs:168-173`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust weakens the C test: C requires both invalid policy calls to fail, including raw out-of-range codes; Rust uses valid typed variants and only requires one call to error, while the helper ignores default-policy errors.
* Phase 5A fix note: Make the Rust test assert each invalid case independently. Propagate server default spinbit-policy errors in spinbit_test_one and add raw-code rejection coverage if the Rust API exposes a conversion path.
* Phase 5B analysis: C checks raw out-of-range spinbit policy values 123456 and 123455 on the default and per-connection setter paths. The current Rust API uses closed SpinbitVersion values, so a faithful compiling runnable Rust test for those raw invalid inputs cannot be written without a raw conversion/setter API. Default SpinbitVersion::On rejection is a Phase 5C implementation note, not the blocker.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    if (spinbit_test_one(picoquic_spinbit_on, 123456) == 0 ||
        spinbit_test_one(123455, picoquic_spinbit_null) == 0) {
        ret = -1;
    }
    return ret;
}
```

### Current Rust test body
```rust
fn spinbit_bad() {
    assert!(
        spinbit_test_one(SpinbitVersion::On, SpinbitVersion::Basic).is_err(),
        "expected server-only per-connection spinbit policy to be rejected"
    );
}
```

## `picoquictest/cnxstress.c:cnx_limit_test`
* C test-table name: `cnx_limit`
* C entry function: `cnx_limit_test`
* Rust test: `cnx_limit`
* Expected Rust file: `rs/fq/src/tests/cnxstress.rs`
* Current Rust span: `rs/fq/src/tests/cnxstress.rs:897-957`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test body mirrors the C phases, but the Rust stress helper treats limit_test as an initial extra client target and active limit-test mode. In C, limit_test only sizes capacity; is_limit_test is set and nb_client_target is incremented only after the first four clients are ready.
* Phase 5A fix note: Make cnx_stress_create_ctx keep nb_client_target at nb_clients and is_limit_test false initially, while still allocating the extra slot/max client capacity for limit tests.
* Phase 5B analysis: Rust now matches the C setup: limit_test allocates extra client-side capacity only, and limit mode/extra target are enabled later by cnx_limit after the first four clients are ready.
* Phase 5B fix note: Changed cnx_stress_create_ctx to initialize is_limit_test to false and nb_client_target to nb_clients while preserving the extra slot allocation for limit tests.

### C test body
```c
{
    int ret = 0;
    int nb_clients = 4;
    uint64_t duration = 120000000;
    cnx_stress_ctx_t* stress_ctx = cnx_stress_create_ctx(duration, nb_clients, 1);

    if (stress_ctx == NULL) {
        ret = -1;
    }

    if (stress_ctx != NULL) {
        int is_done = 0;

        /* loop until time exhausted or all created */
        while (ret == 0 && stress_ctx->simulated_time < duration && !is_done) {
            ret = cnx_stress_loop_step(stress_ctx);
            if (stress_ctx->nb_clients == nb_clients &&
                stress_ctx->nb_servers == nb_clients) {
                is_done = 1;
                for (int c = 0; c < nb_clients; c++) {
                    if (stress_ctx->c_ctx[c] == NULL ||
                        stress_ctx->c_ctx[c]->cnx == NULL ||
                        stress_ctx->c_ctx[c]->cnx->cnx_state <
                        picoquic_state_client_almost_ready) {
                        is_done = 0;
                        break;
                    }
                }
            }
        }
        if (!is_done) {
            ret = -1;
        }

        if (ret == 0) {
            /* Try creating one more client. Loop until time exhausted or
             * verify new client refused because server busy */

            stress_ctx->is_limit_test = 1;
            stress_ctx->next_client_creation_time = stress_ctx->simulated_time;
            stress_ctx->nb_client_target++;
            while (ret == 0 && stress_ctx->simulated_time < duration &&
                !stress_ctx->limit_test_got_server_busy) {
                ret = cnx_stress_loop_step(stress_ctx);
            }
            if (!stress_ctx->limit_test_got_server_busy) {
                ret = -1;
            }
        }

        cnx_stress_delete_ctx(stress_ctx);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn cnx_limit() {
    let nb_clients = 4usize;
    let duration = 120_000_000u64;

    let mut ctx = cnx_stress_create_ctx(duration, nb_clients, true).expect("create stress context");

    // Run until all `nb_clients` connections are up on both sides and at least
    // at `ClientAlmostReady`.
    let mut is_done = false;
    while !is_done && ctx.simulated_time() < duration {
        cnx_stress_loop_step(&mut ctx).expect("loop step");

        let (nb_c, nb_s) = {
            let shared = ctx.shared.borrow();
            (shared.nb_clients, shared.nb_servers)
        };
        if nb_c == nb_clients && nb_s == nb_clients {
            // Check that every client slot has a connection at AlmostReady or beyond.
            is_done = true;
            for c in 0..nb_clients {
                let ok = if let Some(Some(cid)) = ctx.client_connections.get(c) {
                    let cid = *cid;
                    if let Some(cnx) = ctx.qclient.connection_ref_by_id(cid) {
                        cnx.state() >= State::ClientAlmostReady
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !ok {
                    is_done = false;
                    break;
                }
            }
        }
    }
    assert!(
        is_done,
        "failed to reach ClientAlmostReady for all connections"
    );

    // Now attempt one extra connection (beyond the server's limit).
    {
        let mut shared = ctx.shared.borrow_mut();
        shared.is_limit_test = true;
        shared.nb_client_target += 1;
    }
    ctx.next_client_creation_time = ctx.simulated_time();

    while ctx.simulated_time() < duration {
        cnx_stress_loop_step(&mut ctx).expect("limit loop step");
        if ctx.shared.borrow().limit_test_got_server_busy {
            break;
        }
    }
    assert!(
        ctx.shared.borrow().limit_test_got_server_busy,
        "server did not send SERVER_BUSY when at connection limit",
    );
}
```

## `picoquictest/mbedtls_test.c:mbedtls_load_key_fail_test`
* C test-table name: `mbedtls_load_key_fail`
* C entry function: `mbedtls_load_key_fail_test`
* Rust test: `mbedtls_load_key_fail`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:657-660`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers the same four fixture paths and expects failure, but the helper is a textual/path-name stand-in rather than the C behavior of initializing mbedTLS, loading a private key, installing sign_certificate, and attempting a real signature.
* Phase 5A fix note: Replace the placeholder key-load/sign helper with the translated provider path, including init/free equivalent if required, and keep the four negative cases.
* Phase 5B analysis: Rust has sufficient test-harness surface for this Phase 5B contract. The test now mirrors the C init/free bracket and the four negative key-load assertions; incomplete real mbedTLS key-loading/signing behavior is a Phase 5C runtime issue, not a Phase 5B block.
* Phase 5B fix note: Added TlsApiResetGuard::mbedtls_only() to mbedtls_load_key_fail before running the negative load-key cases.

### C test body
```c
{
    int ret = 0;


    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0 && mbedtls_test_load_one_der_key(ASSET_NO_SUCH_FILE) == 0)
        {
            ret = -1;
        }

        if (ret == 0 && mbedtls_test_load_one_der_key(ASSET_NOT_A_PEM_FILE) == 0)
        {
            ret = -1;
        }

        if (ret == 0 && mbedtls_test_load_one_der_key(ASSET_RSA_CERT) == 0)
        {
            ret = -1;
        }

        if (ret == 0 && mbedtls_test_load_one_der_key(ASSET_ED25519_KEY) == 0)
        {
            ret = -1;
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }

    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls_load_key_fail() {
    let _tls_api_reset = TlsApiResetGuard::mbedtls_only();
    mbedtls_test_load_key_fail_cases().expect("load_key_fail_cases");
}
```

## `picoquictest/quic_tester.c:initial_ping_test`
* C test-table name: `initial_ping`
* C entry function: `initial_ping_test`
* Rust test: `initial_ping`
* Expected Rust file: `rs/fq/src/tests/quic_tester.rs`
* Current Rust span: `rs/fq/src/tests/quic_tester.rs:19-60`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust injects the same padded Initial PING before the handshake and runs the connection loop, but its tls_api_test_with_loss_final helper only closes the connection. The C final helper also verifies transport parameters, SNI, ALPN, and negotiated version.
* Phase 5A fix note: Make Rust tls_api_test_with_loss_final perform the C final checks for transport extension negotiation, SNI, ALPN, and version before closing.
* Phase 5B analysis: Rust finalization now checks the same transport-parameter, SNI, ALPN, and negotiated-version conditions as the C helper before closing; initial_ping now uses the C ALPN token.
* Phase 5B fix note: Added C-equivalent checks to tls_api_test_with_loss_final and corrected quic_tester ALPN from picoquic_test to picoquic-test.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint8_t ping_frame[1] = { 1 };
    picoquic_connection_id_t initial_cid = { {0x4e, 0x54, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", PICOQUIC_INTERNAL_TEST_VERSION_1);
    }
    else {
        picoquic_set_qlog(test_ctx->qserver, ".");
    }

    /*
    Insert a ping frame at the client, pass it to the server.
    */
    if (ret == 0) {
        ret = tester_push_frame_packet(test_ctx,
            picoquic_packet_initial,
            ping_frame, sizeof(ping_frame),
            1, 0, simulated_time);
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
fn initial_ping() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;

    let initial_cid =
        ConnectionId::clone_from_slice(&[0x4e, 0x54, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11])
            .expect("build initial CID");

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.set_qlog(".").ok();

    let ping_frame: &[u8] = &[1];
    tester_push_frame_packet(
        &mut test_ctx,
        PacketType::Initial,
        ping_frame,
        true,
        false,
        simulated_time,
    )
    .expect("push ping packet");

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

## `picoquictest/skip_frame_test.c:new_cnxid_test`
* C test-table name: `new_cnxid`
* C entry function: `new_cnxid_test`
* Rust test: `new_cnxid`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:3860-3862`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses a hard-coded NEW_CONNECTION_ID byte sequence and only checks skip length; C creates a QUIC connection, creates a second local CID, formats the frame with picoquic_format_new_connection_id_frame, verifies CID list state, and checks pure_ack is 0.
* Phase 5A fix note: Rework run_new_cnxid_test to create the context/connection/local CID, assert the local CID list has two entries, format the frame via the Rust formatter, then assert skip_frame consumes exactly that formatted length and pure_ack == 0.
* Phase 5B analysis: Rust test now builds a QUIC context/connection, creates a second local CID, verifies the CID list state, formats NEW_CONNECTION_ID through the Rust formatter, and checks skip_frame consumes the formatted length with pure_ack == 0.
* Phase 5B fix note: Replaced the hard-coded NEW_CONNECTION_ID byte fixture with formatter-driven setup matching the C test flow.

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;
    picoquic_quic_t * qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    picoquic_cnx_t * cnx = NULL;
    uint8_t frame_buffer[256];
    size_t consumed = 0;

    memset(&saddr, 0, sizeof(struct sockaddr_in));
    saddr.sin_family = AF_INET;
    saddr.sin_port = 1000;

    if (qclient == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    } else {
        cnx = picoquic_create_cnx(qclient,
            picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr *) &saddr,
            simulated_time, 0, "test-sni", "test-alpn", 1);

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create QUIC CNX context\n");
            ret = -1;
        }
        else {
            /* Create a new local CID */
            picoquic_local_cnxid_t* local_cid = picoquic_create_local_cnxid(cnx, 0, NULL, simulated_time);
            picoquic_local_cnxid_list_t* local_cid_list = cnx->first_local_cnxid_list;
            
            if (local_cid == NULL || local_cid_list == NULL) {
                DBG_PRINTF("%s", "Cannot create local cnxid\n");
                ret = -1;
            }

            if (local_cid_list->nb_local_cnxid != 2) {
                DBG_PRINTF("Expected 2 CID, got %d\n", local_cid_list->nb_local_cnxid);
                ret = -1;
            }
            else if (local_cid_list->local_cnxid_first == NULL || local_cid_list->local_cnxid_first->next == NULL) {
                DBG_PRINTF("%s", "Pointer to CID is NULL in cnx context\n");
                ret = -1;
            }

            if (ret == 0) {
                int more_data = 0;
                int is_pure_ack = 1;
                uint8_t* bytes_next = picoquic_format_new_connection_id_frame(cnx, local_cid_list, frame_buffer, frame_buffer + sizeof(frame_buffer),
                    &more_data, &is_pure_ack, local_cid);

                consumed = bytes_next - frame_buffer;

                if (consumed == 0) {
                    ret = -1;
                    DBG_PRINTF("Cannot encode new connection ID frame, ret = %x\n", ret);
                }
            }

            if (ret == 0) {
                size_t skipped = 0;
                int pure_ack = 0;

                ret = picoquic_skip_frame(frame_buffer, sizeof(frame_buffer), &skipped, &pure_ack);

                if (ret != 0) {
                    DBG_PRINTF("Cannot skip connection ID frame, ret = %x\n", ret);
                }
                else if (skipped != consumed) {
                    DBG_PRINTF("Skipped %d bytes instead of %d\n", (int)skipped, (int)consumed);
                    ret = -1;
                }
                else if (pure_ack != 0) {
                    DBG_PRINTF("Pure ACK = %d instead of 0\n", (int)pure_ack);
                    ret = -1;
                }
            }
            /* Delete the connecton and free the stash */
            picoquic_delete_cnx(cnx);
        }

        picoquic_free(qclient);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn new_cnxid() {
    run_new_cnxid_test().expect("new_cnxid");
}
```
