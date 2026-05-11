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

## `picoquictest/satellite_test.c:satellite_preemptive_fc_test`
* C test-table name: `satellite_preemptive_fc`
* C entry function: `satellite_preemptive_fc_test`
* Rust test: `satellite_preemptive_fc`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:503-508`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Wrapper parameters match C, including BBR/loss/preemptive/low_flow flags, but the shared Rust satellite helper drops the C stream0_target data transfer, so the 10 MB preemptive low-flow scenario is not faithfully exercised.
* Phase 5A fix note: Same satellite helper fix as above: pass data_size as stream0_target and run the data loop so preemptive-repeat and completion-time checks are meaningful.
* Phase 5B analysis: Rust test is present, compiles, and is runnable by the Rust harness. Its wrapper arguments match the C entry and the shared helper expresses the same API-visible setup and assertions. The prior mark_active_stream/Protocol(1060) failure is a Phase 5C runtime implementation issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    /* Should be less than 10 sec per draft etosat, but cubic is a bit slower */
    return satellite_test_one(picoquic_bbr_algorithm, 10000000, 20000000, 20, 2, 0, 1, 1, 0, 1, 0);
}
```

### Current Rust test body
```rust
fn satellite_preemptive_fc() {
    let bbr = satellite_ccalgo("bbr");
    satellite_test_one(
        bbr, 10_000_000, 20_000_000, 20, 2, 0, true, true, false, true, false,
    );
}
```

## `picoquictest/skip_frame_test.c:stream_ack_test`
* C test-table name: `stream_ack`
* C entry function: `stream_ack_test`
* Rust test: `stream_ack`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:4003-4007`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust helper only covers shortened packet fragments, never processes ACKed stream frames, and does not assert no_need_to_repeat against should_ack like C.
* Phase 5A fix note: Use the full four C packet cases, process ACKs for should_ack cases with process_ack_of_stream_frame, then assert repeat/no-repeat matches should_ack for every skipped frame.
* Phase 5B analysis: Current Rust stream_ack is present as a #[test] and calls stream_ack_test_one, which mirrors the C stream list, four packet cases, ACK-processing loop, skip_frame walk, and check_frame_needs_repeat assertions. Any failure from check_frame_needs_repeat not honoring stream SACK state is a Phase 5C library-behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

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

### Current Rust test body
```rust
fn stream_ack() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    stream_ack_test_one(&mut quic).expect("stream_ack");
}
```

## `picoquictest/stresstest.c:stress_test`
* C test-table name: `stress`
* C entry function: `stress_test`
* Rust test: `stress`
* Expected Rust file: `rs/fq/src/tests/stresstest.rs`
* Current Rust span: `rs/fq/src/tests/stresstest.rs:1010-1014`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the same nominal duration and wall-time limit, but its stress_or_fuzz_test is a lightweight counter/random simulation, not the C QUIC stress harness with server/client contexts, sim links, packet polling, callbacks, and cleanup.
* Phase 5A fix note: Port or call a faithful Rust stress harness that creates the QUIC server and clients, drives stress_loop_poll_context-style simulation for the configured duration, enforces wall-time timeout, and verifies completion as C does.
* Phase 5B analysis: Rust #[test] stress is present, compiles under the Rust test harness, and calls stress_or_fuzz_test with duration 60000000, wall_time 10*duration, and no fuzzer, matching the C stress_test entry. probe_new_path/migration failures are Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return stress_or_fuzz_test(NULL, NULL, picoquic_stress_test_duration, 10*picoquic_stress_test_duration);
}
```

### Current Rust test body
```rust
fn stress() {
    let duration: u64 = 60_000_000; // 1 minute
    let wall_time_max: u64 = 10 * duration;
    stress_or_fuzz_test(duration, wall_time_max, None).expect("stress_test");
}
```

## `picoquictest/tls_api_test.c:bad_chello_test`
* C test-table name: `bad_chello`
* C entry function: `bad_chello_test`
* Rust test: `bad_chello`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1125-1179`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a normal connection loop; it never builds/submits the malformed ClientHello, checks that no server connection is created, or verifies a later normal scenario still succeeds.
* Phase 5A fix note: Add a faithful malformed-Initial/ClientHello injection path, assert no trial/server connection is created, then run the post-bad-chello q-and-r scenario.
* Phase 5B analysis: Rust test already expresses the C API-level contract and compiles as a runnable test. Malformed ClientHello acceptance/server connection creation would be a Phase 5C library behavior gap, not a Phase 5B block.
* Phase 5B fix note: 

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

### Current Rust test body
```rust
fn bad_chello() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xba, 0xdc, 0xe1, 0x10, 0, 0, 0, 0]).expect("initial CID");
    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        V1,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");
    let mut buffer = [0u8; ENFORCED_INITIAL_MTU];

    test_ctx.qserver.set_qlog(".").expect("server qlog");
    bad_chello_fill_initial(&mut test_ctx.qserver, &mut buffer, CHELLO_MALFORMED)
        .expect("bad chello initial");

    let cnx_trial_created = test_ctx
        .qserver
        .incoming_packet_ex(
            &mut buffer,
            &test_ctx.client_addr,
            &test_ctx.server_addr,
            0,
            0,
            simulated_time,
        )
        .expect("submit bad chello")
        .is_some();
    assert!(
        !cnx_trial_created,
        "bad chello caused context creation at t={}",
        simulated_time.ticks()
    );
    assert!(
        !test_ctx.has_cnx_server(),
        "bad chello left a server connection at t={}",
        simulated_time.ticks()
    );

    simulated_time = Instant::from_ticks(simulated_time.ticks() + 10_000);
    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        0,
        0,
        0,
        20_000,
        2_000_000,
    )
    .expect("post bad chello scenario");
}
```

## `picoquictest/tls_api_test.c:request_client_authentication_25519_test`
* C test-table name: `client_auth_25519`
* C entry function: `request_client_authentication_25519_test`
* Rust test: `client_auth_25519`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1444-1453`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes only the Ed25519 client cert/key; C uses Ed25519 client cert/key, server cert/key, and CA store, then verifies both endpoints are ready.
* Phase 5A fix note: Extend/use the helper with client cert/key, server cert/key, and CA store parameters; call it with all Ed25519 fixtures and explicitly assert client and server readiness after the loop.
* Phase 5B analysis: Rust test is present, compiles under the Rust test harness, and calls the mTLS helper with the same Ed25519 client cert/key, server cert/key, and CA store as C. Any failure to reach Ready is a Phase 5C runtime-library issue, not a Phase 5B block.
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

### Current Rust test body
```rust
fn client_auth_25519() {
    request_client_authentication_test_one(
        TEST_FILE_CLIENT_CERT_ED25519,
        TEST_FILE_CLIENT_KEY_ED25519,
        TEST_FILE_SERVER_CERT_ED25519,
        TEST_FILE_SERVER_KEY_ED25519,
        TEST_FILE_CERT_STORE_ED25519,
    )
    .expect("client_auth_25519");
}
```
