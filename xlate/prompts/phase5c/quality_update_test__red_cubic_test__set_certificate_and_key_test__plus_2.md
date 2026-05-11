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

## `picoquictest/tls_api_test.c:quality_update_test`
* C test-table name: `quality_update`
* C entry function: `quality_update_test`
* Rust test: `quality_update`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6594-6633`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs a generic TLS smoke test; it does not subscribe to quality updates, write the CSV, run the 1MB scenario, or compare against the reference file.
* Phase 5A fix note: Create the quality-update context, subscribe with deltas 0x10000/0x1000, run the q_and_r stream0_target=1000000 scenario with queue_delay=20000 and max_completion=3600000, then compare quality_update.csv to picoquictest/quality_update_ref.txt.
* Phase 5B analysis: Rust #[test] is present, compiles under the test harness, and expresses the C API-level contract: init TLS API context, create quality CSV/header, subscribe with 0x10000/0x1000, run q_and_r with stream0_target=1000000, queue_delay=20000, max_completion=3600000, then compare against the reference. Prior Protocol(1060)/CannotSetActiveStream and missing quality callback/logging behavior are Phase 5C implementation notes, not Phase 5B blockers.
* Phase 5B fix note: 

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
        test_ctx->default_path_update = picoquic_file_open(QUALITY_UPDATE_CSV, "w");
        if (test_ctx->default_path_update == NULL) {
            DBG_PRINTF("Could not write file <%s>", QUALITY_UPDATE_CSV);
            ret = -1;
        }
        else {
            fprintf(test_ctx->default_path_update, "Time, Path_id, Sending_rate_CB, Pacing_rate, Receive_Rate, CWIN, RTT\n");
            /* Request bandwidth updates */
            picoquic_subscribe_to_quality_update(test_ctx->cnx_client, 0x10000, 0x1000);

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
        char quality_update_ref[512];

        ret = picoquic_get_input_path(quality_update_ref, sizeof(quality_update_ref), picoquic_solution_dir, QUALITY_UPDATE_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the quality update ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(QUALITY_UPDATE_CSV, quality_update_ref);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn quality_update() {
    use std::io::Write as _;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");
    let quality_update_scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    {
        let mut file = std::fs::File::create(QUALITY_UPDATE_CSV).expect("quality_update.csv");
        writeln!(
            file,
            "Time, Path_id, Sending_rate_CB, Pacing_rate, Receive_Rate, CWIN, RTT"
        )
        .expect("quality_update header");
    }

    test_ctx
        .cnx_client()
        .subscribe_to_quality_update(0x10000, crate::Duration::from_ticks(0x1000));

    tls_api_one_scenario_body_ex(
        &mut test_ctx,
        &mut simulated_time,
        &quality_update_scenario,
        1_000_000,
        0,
        0,
        20_000,
        3_600_000,
        &[],
    )
    .expect("quality_update scenario");

    compare_text_files(QUALITY_UPDATE_CSV, QUALITY_UPDATE_REF).expect("quality_update reference");
}
```

## `picoquictest/tls_api_test.c:red_cubic_test`
* C test-table name: `red_cubic`
* C entry function: `red_cubic_test`
* Rust test: `red_cubic`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6848-6850`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper arguments resemble the C call, but the Rust helper does not preserve the C RED test: it treats the C loss_target argument as MTU, ignores the algorithm id, omits RED AQM setup, uses a different scenario, and does not check observed retransmission loss.
* Phase 5A fix note: Repair red_cc_algotest to select Cubic, configure RED AQM/latency/bandwidth like C, run test_scenario_sustained, enforce target completion time, and assert observed server retransmissions are <= loss_target 225.
* Phase 5B analysis: Rust red_cubic is present as a #[test], compiles under the test harness, and calls red_cc_algotest("cubic", 510_000, 225), matching the C entry's API-level contract. Any runtime failure from cubic being wired to baseline congestion behavior is Phase 5C, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_cubic_algorithm, 510000, 225);
    return ret;
}
```

### Current Rust test body
```rust
fn red_cubic() {
    red_cc_algotest("cubic", 510_000, 225).expect("red_cubic");
}
```

## `picoquictest/tls_api_test.c:set_certificate_and_key_test`
* C test-table name: `set_certificate_and_key`
* C entry function: `set_certificate_and_key_test`
* Rust test: `set_certificate_and_key`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7384-7445`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a default handshake. The C test recreates the server without cert/key/root files, installs them through setter APIs, then checks the connection reaches ready state.
* Phase 5A fix note: Add a Rust test/helper that recreates qserver without certificate inputs, calls private-key/certificate-chain/root-certificate setter paths, runs the connection loop, and asserts client/server ready.
* Phase 5B analysis: Rust test is present as a runnable #[test], compiles, and matches the C API-visible contract: init context, recreate qserver without cert inputs, set private key, set certificate chain, set root certificates, re-enable server mode, run connection loop, and assert client/server ready. The prior handshake failure is a Phase 5C library-behavior issue, not a Phase 5B block.
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

    /* Delete the server context, and recreate it. */
    if (ret == 0)
    {
        if (test_ctx->qserver != NULL) {
            picoquic_free(test_ctx->qserver);
        }

        test_ctx->qserver = picoquic_create(8,
            NULL, NULL, NULL,
            PICOQUIC_TEST_ALPN, test_api_callback, (void*)&test_ctx->server_callback, NULL, NULL, NULL,
            simulated_time, &simulated_time, NULL,
            test_ticket_encrypt_key, sizeof(test_ticket_encrypt_key));

        if (test_ctx->qserver == NULL) {
            ret = -1;
        }

        if (ret == 0) {
            ret = picoquic_set_private_key_from_file(test_ctx->qserver, test_server_key_file);
        }

        if (ret == 0) {
            size_t count = 0;
            ptls_iovec_t* chain = picoquic_get_certs_from_file(test_server_cert_file, &count);
            if (chain == NULL) {
                ret = -1;
            } else {
                picoquic_set_tls_certificate_chain(test_ctx->qserver, chain, count);
            }
        }

        if (ret == 0) {
            size_t count = 0;
            ptls_iovec_t* chain = picoquic_get_certs_from_file(test_server_cert_store_file, &count);

            if (chain == NULL) {
                ret = -1;
            } else {
                picoquic_set_tls_root_certificates(test_ctx->qserver, chain, count);
                for (size_t i = 0; i < count; i++) {
                    free(chain[i].base);
                }
                free(chain);
            }
        }
    }

    /* Proceed with the connection loop. */
    if (ret == 0) {
        picoquic_enforce_client_only(test_ctx->qserver, 0);
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0 && (!TEST_CLIENT_READY || !TEST_SERVER_READY)) {
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
fn set_certificate_and_key() {
    const SERVER_KEY: [u8; crate::RESET_SECRET_SIZE] = {
        let mut k = [0u8; crate::RESET_SECRET_SIZE];
        let mut i = 0usize;
        while i < crate::RESET_SECRET_SIZE {
            k[i] = i as u8;
            i += 1;
        }
        k
    };

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    test_ctx.qserver = crate::Quic::new(
        8,
        None,
        None,
        None,
        Some(TEST_ALPN),
        None,
        None,
        [0u8; crate::RESET_SECRET_SIZE],
        simulated_time,
        None,
        Some(&SERVER_KEY),
    )
    .expect("recreate qserver without certificate inputs");

    test_ctx
        .qserver
        .set_private_key_from_file(TEST_FILE_SERVER_KEY)
        .expect("set private key");

    let server_certs =
        crate::tls_api::get_certs_from_file(TEST_FILE_SERVER_CERT).expect("server cert chain");
    test_ctx.qserver.set_tls_certificate_chain(server_certs);

    let root_certs =
        crate::tls_api::get_certs_from_file(TEST_FILE_CERT_STORE).expect("root cert chain");
    test_ctx
        .qserver
        .set_tls_root_certificates(root_certs)
        .expect("set root certificates");

    test_ctx.qserver.enforce_client_only(false);

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    let client_state = test_ctx.qclient.first_cnx_mut().map(|cnx| cnx.state());
    let server_state = test_ctx.qserver.first_cnx_mut().map(|cnx| cnx.state());
    assert!(
        test_ctx.client_ready(),
        "client did not reach ready state: client={client_state:?} server={server_state:?}",
    );
    assert!(
        test_ctx.server_ready(),
        "server did not reach ready state: client={client_state:?} server={server_state:?}",
    );
}
```

## `picoquictest/tls_api_test.c:stop_sending_loss_test`
* C test-table name: `stop_sending_loss`
* C entry function: `stop_sending_loss_test`
* Rust test: `stop_sending_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7947-7949`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls the matching helper flags, but the helper does not preserve the C scenario: wrong stream set, missing pre-STOP partial-receive loop, different loss masks/latency, ignored stop_sending result, and missing final stream/data-node assertions.
* Phase 5A fix note: Rework Rust stop_sending_test_one to use the C two-stream scenario, 100ms latency, calibrated loss masks including RESET loss, partial first-stream receive before STOP_SENDING, and the same completion/leak checks.
* Phase 5B analysis: Rust #[test] is present and calls stop_sending_test_one(false, true), matching the C stop_sending_test_one(0, 1). The helper expresses the same API-level STOP_SENDING/loss scenario and assertions; any early handshake/runtime failure is Phase 5C behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = stop_sending_test_one(0, 1);
    return ret;
}
```

### Current Rust test body
```rust
fn stop_sending_loss() {
    stop_sending_test_one(false, true).expect("stop_sending_loss");
}
```

## `picoquictest/tls_api_test.c:tls_api_very_long_congestion_test`
* C test-table name: `tls_api_very_long_congestion`
* C entry function: `tls_api_very_long_congestion_test`
* Rust test: `tls_api_very_long_congestion`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8162-8189`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust omits test_scenario_very_long, max_data=128000, queue_delay_max=20000 behavior, and effective completion verification; the 20000 argument lands in an ignored Rust parameter.
* Phase 5A fix note: Run the 1 MB very-long scenario with max_data 128000, queue delay 20000 us, and a real 1000000 us completion assertion.
* Phase 5B analysis: Rust test is present as a runnable #[test], compiles under the Rust test harness, and matches the C API-level contract: very-long scenario, loss_mask=0, max_data=128000, queue_delay_max=20000, and 1000000 us completion bound. Any early failure because the server connection is not accepted is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 128000, 20000, 0, 1000000, NULL, NULL);
}
```

### Current Rust test body
```rust
fn tls_api_very_long_congestion() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    let mut loss_mask = 0u64;

    tls_api_connection_loop(&mut ctx, &mut loss_mask, 20_000, &mut t)
        .expect("very_long_congestion connect");
    wait_client_connection_ready(&mut ctx, &mut t).expect("very_long_congestion ready");
    assert!(
        ctx.server_ready(),
        "very_long_congestion: server connection was not accepted"
    );
    {
        let client = ctx.cnx_client();
        client.maxdata_local = 128_000;
        client.maxdata_remote = 128_000;
    }
    {
        let server = ctx.cnx_server();
        server.maxdata_local = 128_000;
        server.maxdata_remote = 128_000;
    }
    test_api_init_send_recv_scenario(&mut ctx, TEST_SCENARIO_VERY_LONG)
        .expect("very_long_congestion scenario");
    tls_api_data_sending_loop(&mut ctx, &mut loss_mask, &mut t, 0)
        .expect("very_long_congestion data");
    tls_api_one_scenario_body_verify(&mut ctx, &mut t, 1_000_000).expect("very_long_congestion");
}
```
