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

## `picoquictest/satellite_test.c:satellite_seeded_test`
* C test-table name: `satellite_seeded`
* C entry function: `satellite_seeded_test`
* Rust test: `satellite_seeded`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:218-233`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust wrapper passes the same satellite_seeded parameters, but the helper path no longer checks the C behavior: the 100MB stream0 transfer and max-completion assertion are effectively lost through the Rust tls_api_one_scenario_body helper.
* Phase 5A fix note: Make the Rust satellite helper drive the C-equivalent stream0 transfer with stream0_target=100_000_000, seed_bw=true, satellite link settings, loss/max-time handling, and completion verification.
* Phase 5B analysis: Rust #[test] satellite_seeded matches the C satellite_seeded_test API-level contract: bbr, 100MB stream0 transfer, 4_900_000 completion bound, 250/3 Mbps links, no jitter/loss/preemptive/flow-control, and seed_bw enabled. Prior Protocol(1060)/CannotSetActiveStream runtime failure is a Phase 5C implementation/harness behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Simulate remembering RTT and BW from previous connection */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 4900000, 250, 3, 0, 0, 0, 1, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_seeded() {
    let bbr = satellite_ccalgo("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        4_900_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}
```

## `picoquictest/sockloop_test.c:sockloop_ipv4_test`
* C test-table name: `sockloop_ipv4`
* C entry function: `sockloop_ipv4_test`
* Rust test: `sockloop_ipv4`
* Expected Rust file: `rs/fq/src/tests/sockloop.rs`
* Current Rust span: `rs/fq/src/tests/sockloop.rs:997-1003`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust sets the same IPv4, buffer, and 1M scenario spec fields, but its sockloop driver does not initialize the scenario on the test context, uses a readiness-only finished predicate, and skips the C post-loop scenario verification, so the 1M IPv4 transfer is not checked.
* Phase 5A fix note: Initialize the configured scenario before the packet loop, make completion depend on actual stream receive counts, and add the Rust equivalent of tls_api_one_scenario_verify after the loop.
* Phase 5B analysis: Rust test is present with #[test], compiles under cargo check --tests, and matches the C API-level setup: test id 4, AF_INET, socket buffer 0xffff, 1M scenario, and sockloop_test_one. The packet-loop Generic/runtime failure is a Phase 5C library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 4);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);

    return(sockloop_test_one(&spec));
}
```

### Current Rust test body
```rust
fn sockloop_ipv4() {
    let mut spec = SockloopTestSpec::new(4);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    sockloop_test_one(&spec);
}
```

## `picoquictest/ticket_store_test.c:ticket_seed_test`
* C test-table name: `ticket_seed`
* C entry function: `ticket_seed_test`
* Rust test: `ticket_seed`
* Expected Rust file: `rs/fq/src/tests/ticket_store.rs`
* Current Rust span: `rs/fq/src/tests/ticket_store.rs:16-18`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper argument matches, but the Rust helper is placeholder-like: it manually writes ticket IDs, 0-RTT parameters, and seed values instead of verifying real ticket issuance, storage, deletion/recreation, and actual resumed seeding as the C test does.
* Phase 5A fix note: Make the Rust helper rely on real ticket issuance/storage, inspect actual client and server ticket RTT/CWIN values, delete and recreate the client connection, then verify actual resumed ticket IDs and seeded RTT/CWIN with bdp_option 1.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and delegates to ticket_seed_test_one(1), whose reviewed helper covers the same API-visible lifecycle and assertions as the C ticket_seed_test_one path. Any failure to store/issue/resume tickets or seed RTT/CWIN is a Phase 5C implementation behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
int ticket_seed_test(void) {
    
   return ticket_seed_test_one(1);
}
```

### Current Rust test body
```rust
fn ticket_seed() {
    ticket_seed_test_one(1).expect("ticket_seed");
}
```

## `picoquictest/tls_api_test.c:bad_client_certificate_test`
* C test-table name: `bad_client_certificate`
* C entry function: `bad_client_certificate_test`
* Rust test: `bad_client_certificate`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1186-1245`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs a generic successful tls_api_test_with_loss path with the normal client context. It does not recreate the client with badcert.pem, require server client authentication, expect handshake failure, or assert client/server handshake errors.
* Phase 5A fix note: Implement the bad-client-certificate scenario: recreate qclient with TEST_FILE_SERVER_BAD_CERT/TEST_FILE_SERVER_KEY/TEST_FILE_CERT_STORE, enable qserver client authentication, run the loop expecting disconnect, and assert handshake errors on server local and client remote error.
* Phase 5B analysis: Rust #[test] already matches the C API-level contract and compiles under the Rust test harness. Any remaining missing server connection retention or handshake-error behavior is Phase 5C runtime implementation work, not a Phase 5B block.
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
    int ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_BAD_CERT);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }
    else {
        ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    }

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Delete the client context, and recreate with a certificate */
    if (ret == 0)
    {
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

    if (ret == 0) {
        picoquic_set_client_authentication(test_ctx->qserver, 1);
        
        /* Proceed with the connection loop. It should fail */
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_local_error(test_ctx->cnx_server))) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_remote_error(test_ctx->cnx_client))) {
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

### Current Rust test body
```rust
fn bad_client_certificate() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");
    let server_addr = test_ctx.server_addr;

    test_ctx.qclient = Quic::new(
        8,
        Some(TEST_FILE_SERVER_BAD_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        None,
    )
    .expect("bad-client-cert client context");
    test_ctx.qclient.enforce_client_only(true);

    {
        let client = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).expect("initial CID"),
                ConnectionId::with_size(0).expect("remote CID"),
                Some(&server_addr),
                simulated_time,
                0,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .expect("client connection");
        client.start_client().expect("start client");
    }

    test_ctx.qserver.set_client_authentication(true);
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    let (client_state, client_remote_error) = {
        let client = test_ctx.cnx_client();
        (client.state(), client.remote_error())
    };
    assert_eq!(client_state, State::Disconnected);

    assert!(test_ctx.has_cnx_server(), "server connection should exist");
    let server_local_error = test_ctx.cnx_server().local_error();
    assert!(
        is_handshake_error(server_local_error),
        "server local error should be a handshake error, got {server_local_error:#x}"
    );
    assert!(
        is_handshake_error(client_remote_error),
        "client remote error should be a handshake error, got {client_remote_error:#x}"
    );
}
```

## `picoquictest/tls_api_test.c:set_verify_certificate_callback_test`
* C test-table name: `client_cert_callback`
* C entry function: `set_verify_certificate_callback_test`
* Rust test: `client_cert_callback`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1460-1518`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs the generic successful TLS helper. It does not install certificate verification callbacks, recreate the client with a certificate, enable client authentication, or verify the callback count of 2.
* Phase 5A fix note: Implement a counting Rust certificate verify callback, install it on client and server, recreate the client with cert/key/store, enable server client authentication, run the handshake, and assert two callback invocations.
* Phase 5B analysis: Rust test is present, compiles as a harness test, and expresses the C API-level contract: recreate client with cert/key/store, install verify callbacks on client and server, enable server client auth, run the loop, and assert two callback invocations. Any callback not firing at runtime is Phase 5C implementation behavior, not a Phase 5B block.
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

### Current Rust test body
```rust
fn client_cert_callback() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");
    let verify_callcount = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let server_addr = test_ctx.server_addr;

    test_ctx.qclient = Quic::new(
        8,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        None,
    )
    .expect("client context with certificate");

    {
        let client = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).expect("initial CID"),
                ConnectionId::with_size(0).expect("remote CID"),
                Some(&server_addr),
                simulated_time,
                0,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .expect("client connection");
        client.start_client().expect("start client");
    }

    test_ctx
        .qclient
        .set_verify_certificate_callback(Some(Box::new(CountingVerifyCertificateCallback {
            callcount: std::sync::Arc::clone(&verify_callcount),
        })));
    test_ctx
        .qserver
        .set_verify_certificate_callback(Some(Box::new(CountingVerifyCertificateCallback {
            callcount: std::sync::Arc::clone(&verify_callcount),
        })));
    test_ctx.qserver.set_client_authentication(true);

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    assert_eq!(
        verify_callcount.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "certificate verification callback count"
    );
}
```
