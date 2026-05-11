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

## `picoquictest/satellite_test.c:satellite_jitter_test`
* C test-table name: `satellite_jitter`
* C entry function: `satellite_jitter_test`
* Rust test: `satellite_jitter`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:313-328`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The entry arguments match, but the Rust helper does not preserve the C behavior: C sends 100 MB on stream 0 and enforces the 6,700,000 us completion target; Rust passes data_size into an ignored helper parameter and the completion limit is not checked.
* Phase 5A fix note: Make the Rust satellite helper drive stream0_target=data_size, preserve the C tls_api_one_scenario_body argument semantics, and assert max_completion_time while keeping the jitter/link settings.
* Phase 5B analysis: Rust test is present as a runnable #[test], compiles under cargo check --tests, and matches the C API-level call tuple. The CannotSetActiveStream/callback_fn runtime failure is a Phase 5C library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Should be less than 7 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 6700000, 250, 3, 3000, 0, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_jitter() {
    let bbr = satellite_ccalgo("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        6_700_000,
        250,
        3,
        3_000,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/skip_frame_test.c:binlog_test`
* C test-table name: `binlog`
* C entry function: `binlog_test`
* Rust test: `binlog`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:4358-4360`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is only a minimal smoke test that creates a connection and logs one app message. It omits the C test's good-frame and bad-frame packet logging, binary log reference comparison, qlog conversion and reference comparison, sharp-end bad-frame logging cases, and 100x100 fuzz logging crash check.
* Phase 5A fix note: Expand run_binlog_test to generate and compare the same binlog output, convert and compare qlog output, and add the bad-frame sharp_end and fuzz logging loops from the C test or faithful Rust equivalents.
* Phase 5B analysis: Rust #[test] binlog is present and drives the C-shaped binlog scenario, reference comparisons, qlog conversion, bad-frame logging, and 100x100 fuzz logging. The binlog byte divergence is Phase 5C runtime logger behavior, not a Phase 5B block. cargo check --tests currently fails in unrelated rs/fq/src/tests/util.rs on missing textlog_transport_extension_content.
* Phase 5B fix note: 

### C test body
```c
{
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t fuzz_buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint64_t random_context = 0xF00BAB;
    int ret = 0;

    const picoquic_connection_id_t initial_cid = {
        { 1, 2, 3, 4 }, 4
    };

    const picoquic_connection_id_t dest_cid = {
        { 5, 6, 7, 8 }, 4
    };

    char log_test_ref[512];
    int ret_bin = picoquic_get_input_path(log_test_ref, sizeof(log_test_ref), picoquic_solution_dir, BINLOG_TEST_REF);

    char qlog_test_ref[512];
    int ret_qlog = picoquic_get_input_path(qlog_test_ref, sizeof(qlog_test_ref), picoquic_solution_dir, QLOG_TEST_REF);

    uint64_t simulated_time = 0;
    picoquic_quic_t* quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    if (quic == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    } else if (ret_bin != 0 || ret_qlog != 0) {
        DBG_PRINTF("%s", "Cannot set the log ref file name.\n");
        ret = -1;
    }
    else {
        picoquic_set_binlog(quic, ".");        
        (void)picoquic_set_default_spinbit_policy(quic, picoquic_spinbit_null);

        struct sockaddr_in saddr;
        memset(&saddr, 0, sizeof(struct sockaddr_in));
        picoquic_cnx_t* cnx = picoquic_create_cnx(quic, initial_cid, dest_cid, (struct sockaddr*) & saddr,
            simulated_time, 0, "test-sni", "test-alpn", 1);

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create QUIC CNX context\n");
            ret = -1;
        }
        else {
            picoquic_log_new_connection(cnx);
            /* Log of good packets */
            for (size_t i = 0; i < nb_test_skip_list; i++) {

                picoquic_packet_header ph;
                memset(&ph, 0, sizeof(ph));

                ph.ptype = picoquic_packet_1rtt_protected;
                ph.pn64 = i;
                ph.dest_cnx_id = initial_cid;
                ph.srce_cnx_id = dest_cid;

                ph.offset = 0;
                ph.payload_length = test_skip_list[i].len;

                binlog_packet(cnx->f_binlog, &initial_cid, 0, 0, 0, &ph, test_skip_list[i].val, test_skip_list[i].len);
            }
            /* Log of bad backets */
            for (size_t i = 0; i < nb_test_frame_error_list; i++) {
                picoquic_packet_header ph;
                memset(&ph, 0, sizeof(ph));

                ph.ptype = picoquic_packet_1rtt_protected;
                ph.pn64 = i;
                ph.dest_cnx_id = initial_cid;
                ph.srce_cnx_id = dest_cid;

                ph.offset = 0;
                ph.payload_length = test_frame_error_list[i].len;

                binlog_packet(cnx->f_binlog, &initial_cid, 0, 0, 0, &ph, test_frame_error_list[i].val, test_frame_error_list[i].len);
            }
            picoquic_delete_cnx(cnx);
        }
    }

    picoquic_free(quic);

    if (ret == 0) {
        ret_bin = picoquic_test_compare_binary_files(binlog_test_file, log_test_ref);
        if (ret_bin != 0) {
            DBG_PRINTF("%s", "Unexpected content in binary log file.\n");
        }

        /* Convert to QLOG and verify */
        uint64_t log_time = 0;
        uint16_t flags;
        FILE* f_binlog = picoquic_open_cc_log_file_for_read(binlog_test_file, &flags, &log_time);
        
        ret = qlog_convert(&initial_cid, f_binlog, binlog_test_file, NULL, ".", flags);
        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot convert the binary log into QLOG.\n");
        } else {
            /* When changing the reference QLOG file please verify the new file at:
                https://qvis.edm.uhasselt.be/#/files */
            ret_qlog = picoquic_test_compare_text_files(qlog_test_file, qlog_test_ref);
            if (ret_qlog != 0) {
                DBG_PRINTF("%s", "Unexpected content in QLOG log file.\n");
            }
        }

        if (ret_bin != 0 || ret_qlog != 0) {
            ret = -1;
        }
    }


    /* Log a series of known bad packets  */
    for (size_t i = 0; ret == 0 && i < nb_test_frame_error_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            uint8_t extra_bytes[4] = { 0, 0, 0, 0 };
            size_t bytes_max = 0;
            FILE* F = NULL;

            if ((F = picoquic_file_open(binlog_error_test_file, "wb")) == NULL) {
                DBG_PRINTF("failed to open file:%s\n", binlog_error_test_file);
                ret = PICOQUIC_ERROR_INVALID_FILE;
                break;
            }

            memcpy(buffer, test_frame_error_list[i].val, test_frame_error_list[i].len);
            bytes_max = test_frame_error_list[i].len;
            if (test_frame_error_list[i].must_be_last == 0 && sharp_end == 0) {
                /* add some padding to check that the end of frame is detected properly */
                memcpy(buffer + bytes_max, extra_bytes, sizeof(extra_bytes));
                bytes_max += sizeof(extra_bytes);
            }

            picoquic_binlog_frames(F, buffer, bytes_max);

            (void)picoquic_file_close(F);
        }
    }

    /* Do a minimal fuzz test */
    for (size_t i = 0; ret == 0 && i < 100; i++) {
        size_t bytes_max = format_random_packet(buffer, sizeof(buffer), &random_context, -1);
        FILE* F;

        if ((F = picoquic_file_open(binlog_fuzz_test_file, "wb")) == NULL) {
            DBG_PRINTF("failed to open file:%s\n", log_fuzz_test_file);
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        }

        picoquic_binlog_frames(F, buffer, bytes_max);

        /* Attempt to log fuzzed packets, and hope nothing crashes */
        for (size_t j = 0; j < 100; j++) {
            fflush(F);
            skip_test_fuzz_packet(fuzz_buffer, buffer, bytes_max, &random_context);
            picoquic_binlog_frames(F, fuzz_buffer, bytes_max);
        }
        (void)picoquic_file_close(F);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn binlog() {
    run_binlog_test().expect("binlog_test");
}
```

## `picoquictest/sockloop_test.c:sockloop_thread_test`
* C test-table name: `sockloop_thread`
* C entry function: `sockloop_thread_test`
* Rust test: `sockloop_thread`
* Expected Rust file: `rs/fq/src/tests/sockloop.rs`
* Current Rust span: `rs/fq/src/tests/sockloop.rs:1031-1037`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test sets the same visible spec fields and enables the background thread, but SockloopTestSpec.scenario is never consumed by the Rust helper, so the 1MB stream scenario and final scenario verification from the C test are missing.
* Phase 5A fix note: In 5B, make sockloop_test_one_result initialize and verify spec.scenario, equivalent to C test_api_init_send_recv_scenario plus tls_api_one_scenario_verify, while preserving the background-thread readiness and wake path.
* Phase 5B analysis: Rust test matches the C API-level contract and compiles as a harness test. The background-thread loop stub may still fail at runtime, but that is Phase 5C implementation work.
* Phase 5B fix note: 

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 7);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.use_background_thread = 1;

    return(sockloop_test_one(&spec));
}
```

### Current Rust test body
```rust
fn sockloop_thread() {
    let mut spec = SockloopTestSpec::new(7);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.use_background_thread = true;
    sockloop_test_one(&spec);
}
```

## `picoquictest/ticket_store_test.c:token_reuse_api_test`
* C test-table name: `token_reuse_api`
* C entry function: `token_reuse_api_test`
* Rust test: `token_reuse_api`
* Expected Rust file: `rs/fq/src/tests/ticket_store.rs`
* Current Rust span: `rs/fq/src/tests/ticket_store.rs:304-388`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers simple duplicate rejection and clearing, but it uses different token data, all with the same expiry, clears all tokens, and misses C's same-hash/different-expiry cases, selective retention after clear(test_time=4), and length<8 rejection loop.
* Phase 5A fix note: Use the exact seven C token_reuse_api_cases with their expiry dates and lengths, test first insertion of all cases before duplicate detection, clear at time 4 and assert expired vs retained behavior, and add the l=0..7 rejection checks.
* Phase 5B analysis: Rust test is present, compiles as a harness test, and mirrors the C token table, duplicate checks, clear-at-4 checks, and short-token rejection. Any runtime failure from token reuse keying is Phase 5C implementation work.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    uint64_t test_time = 4;
    uint64_t simulated_time = 0;
    picoquic_quic_t * quic = picoquic_create(4, NULL, NULL, NULL, "test", NULL, NULL, NULL, NULL,
        NULL, 0, &simulated_time, NULL, NULL, 0);

    if (quic == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context");
        ret = -1;
    }
    else {
        /* Test that all tokens can be created */
        for (size_t i = 0; ret == 0 && i < nb_token_reuse_api_cases; i++) {
            if (picoquic_registered_token_check_reuse(quic,
                token_reuse_api_cases[i].token,
                token_reuse_api_cases[i].token_length,
                token_reuse_api_cases[i].expiry_date) != 0) {
                DBG_PRINTF("Token[%z] already used?", i);
                ret = -1;
            }
        }
        /* Test that all tokens can be detected as in use */
        for (size_t i = 0; ret == 0 && i < nb_token_reuse_api_cases; i++) {
            if (picoquic_registered_token_check_reuse(quic,
                token_reuse_api_cases[i].token,
                token_reuse_api_cases[i].token_length,
                token_reuse_api_cases[i].expiry_date) == 0) {
                DBG_PRINTF("Token[%z] not already used?", i);
                ret = -1;
            }
        }
        /* Remove tokens with t <= test_time */
        picoquic_registered_token_clear(quic, test_time);
        /* Test that all deleted tokens are absent and others are not */
        for (size_t i = 0; ret == 0 && i < nb_token_reuse_api_cases; i++) {
            int x = picoquic_registered_token_check_reuse(quic,
                token_reuse_api_cases[i].token,
                token_reuse_api_cases[i].token_length,
                token_reuse_api_cases[i].expiry_date);
            if (x == 0 && token_reuse_api_cases[i].expiry_date >= test_time){
                DBG_PRINTF("Token[%z], time %" PRIu64 " not already used?", i, token_reuse_api_cases[i].expiry_date);
                ret = -1;
            }
            if (x != 0 && token_reuse_api_cases[i].expiry_date < test_time) {
                DBG_PRINTF("Token[%z], time %" PRIu64 " already used?", i, token_reuse_api_cases[i].expiry_date);
                ret = -1;
            }
        }
        /* Check refusal with length < 8 */
        for (size_t l = 0; ret == 0 && l < 8; l++) {
            if (picoquic_registered_token_check_reuse(quic,
                token_reuse_api_cases[0].token, l,
                token_reuse_api_cases[0].expiry_date) == 0) {
                DBG_PRINTF("Token[1] length %z accepted?", l);
                ret = -1;
            }
        }

        picoquic_free(quic);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn token_reuse_api() {
    let mut t = Instant::from_ticks(1_000_000);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    struct TokenReuseApiCase {
        expiry_date: u64,
        token: [u8; 16],
        token_length: usize,
    }

    let cases = [
        TokenReuseApiCase {
            expiry_date: 2,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 3,
            token: [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 3,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 3,
            token: [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 5,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 7,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 1,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0],
            token_length: 8,
        },
    ];

    for (i, case) in cases.iter().enumerate() {
        ctx.qserver
            .registered_token_check_reuse(&case.token, case.token_length, case.expiry_date)
            .unwrap_or_else(|err| panic!("Token[{i}] already used? {err:?}"));
    }

    for (i, case) in cases.iter().enumerate() {
        let result = ctx
            .qserver
            .registered_token_check_reuse(&case.token, case.token_length, case.expiry_date)
            .is_err();
        assert!(result, "Token[{i}] not already used?");
    }

    t = Instant::from_ticks(4);
    ctx.qserver.registered_token_clear(t);

    for (i, case) in cases.iter().enumerate() {
        let result = ctx.qserver.registered_token_check_reuse(
            &case.token,
            case.token_length,
            case.expiry_date,
        );
        if case.expiry_date >= 4 {
            assert!(result.is_err(), "Token[{i}] not already used after clear?");
        } else {
            assert!(result.is_ok(), "Token[{i}] already used after clear?");
        }
    }

    for length in 0..8 {
        let result =
            ctx.qserver
                .registered_token_check_reuse(&cases[0].token, length, cases[0].expiry_date);
        assert!(result.is_err(), "Token[1] length {length} accepted?");
    }
}
```

## `picoquictest/tls_api_test.c:chacha20_test`
* C test-table name: `chacha20`
* C entry function: `chacha20_test`
* Rust test: `chacha20`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1351-1372`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test runs a generic TLS API no-loss connection using V1/SNI/ALPN, but it never selects PICOQUIC_CHACHA20_POLY1305_SHA256 and does not run the C q-and-r data scenario with the 250000 usec completion bound.
* Phase 5A fix note: In 5B, initialize the TLS API context, set the client cipher suite to CHACHA20_POLY1305_SHA256, pass if unsupported as C does, and when supported run the q-and-r scenario with the same completion bound.
* Phase 5B analysis: Rust test is present under #[test], compiles/runs under the Rust harness, and matches the C API-level contract: initialize TLS API context, call set_cipher_suite(CHACHA20_POLY1305_SHA256), run TEST_SCENARIO_Q_AND_R with 250000 usec only when the selector succeeds, otherwise skip. Any failure from incomplete ChaCha20 cipher persistence/enforcement or AEAD implementation is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int has_chacha_poly = 0;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Set the cipher suite to chacha20
     */
    if (ret == 0) {
        has_chacha_poly = (picoquic_set_cipher_suite(test_ctx->qclient, PICOQUIC_CHACHA20_POLY1305_SHA256) == 0);

        if (has_chacha_poly) {

            /* Run a basic test scenario */
            ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
                test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 250000);
        }
        else {
            DBG_PRINTF("%s", "Could not test CHACHA20, not supported on this platform.");
        }
    }

    /* And then free the resource
     */

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn chacha20() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    if test_ctx
        .qclient
        .set_cipher_suite(CHACHA20_POLY1305_SHA256)
        .is_ok()
    {
        tls_api_one_scenario_body(
            &mut test_ctx,
            &mut simulated_time,
            TEST_SCENARIO_Q_AND_R,
            0,
            0,
            0,
            0,
            250_000,
        )
        .expect("chacha20");
    }
}
```
