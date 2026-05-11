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

## `picoquictest/minicrypto_test.c:minicrypto_test`
* C test-table name: `minicrypto`
* C entry function: `minicrypto_test`
* Rust test: `minicrypto`
* C source: `picoquictest/minicrypto_test.c:63-110`
* Rust source: `rs/fq/src/tests/minicrypto.rs:31-65`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t target_time = 1000000;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x81, 0x81, 0xc8, 0x19, 0x40, 0, 6, 7}, 8 };
    int ret = 0;

    picoquic_tls_api_reset(TLS_API_INIT_FLAGS_NO_OPENSSL);
#ifndef PICOQUIC_WITH_MBEDTLS
    picoquic_mbedtls_load(0);
#endif
    ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid, 8, 0, 0, 1);
    if (ret == 0) {
        picoquic_set_binlog(test_ctx->qserver, ".");
        test_ctx->qserver->use_long_log = 1;
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 20000, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_minicrypto, sizeof(test_scenario_minicrypto));
    }

    /* Try to complete the data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, target_time);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    picoquic_tls_api_reset(0);

    return ret;
}
```

### Rust test body
```rust
fn minicrypto() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;
    let target_time: u64 = 1_000_000;

    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL);

    let initial_cid =
        crate::ConnectionId::clone_from_slice(&[0x81, 0x81, 0xc8, 0x19, 0x40, 0, 6, 7])
            .expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 20_000, &mut simulated_time)
        .expect("connection loop");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MINICRYPTO)
        .expect("init scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
        .expect("scenario verify");

    reset_tls_api(0);
}
```

## `picoquictest/multipath_test.c:multipath_standup_test`
* C test-table name: `multipath_standup`
* C entry function: `multipath_standup_test`
* Rust test: `multipath_standup`
* C source: `picoquictest/multipath_test.c:1511-1516`
* Rust source: `rs/fq/src/tests/multipath.rs:1639-1641`

### C test body
```c
{
    uint64_t max_completion_microsec = 7200000;

    return multipath_test_one(max_completion_microsec, multipath_test_standup);
}
```

### Rust test body
```rust
fn multipath_standup() {
    multipath_test_one(7_200_000, MultipathTestId::Standup);
}
```

## `picoquictest/pacing_test.c:pacing_cubic_test`
* C test-table name: `pacing_cubic`
* C entry function: `pacing_cubic_test`
* Rust test: `pacing_cubic`
* C source: `picoquictest/pacing_test.c:226-230`
* Rust source: `rs/fq/src/tests/pacing.rs:616-618`

### C test body
```c
{
    int ret = pacing_cc_algotest(picoquic_cubic_algorithm, 900000, 210);
    return ret;
}
```

### Rust test body
```rust
fn pacing_cubic() {
    pacing_cc_algotest("cubic", 900_000, 210);
}
```

## `picoquictest/picolog_test.c:picolog_basic_test`
* C test-table name: `picolog_basic`
* C entry function: `picolog_basic_test`
* Rust test: `picolog_basic`
* C source: `picoquictest/picolog_test.c:81-165`
* Rust source: `rs/fq/src/tests/picolog.rs:390-426`

### C test body
```c
{
    int ret = 0;
    /* find the test input file */
    char log_test_input[512];
    char svg_template[512];
    app_conversion_context_t appctx = { 0 };
    picohash_table* cids = NULL;

    appctx.binlog_name = "?";
    appctx.out_dir = ".";
    ret = picoquic_get_input_path(log_test_input, sizeof(log_test_input), picoquic_solution_dir, PICOLOG_BIN_INPUT);
    if (ret == 0) {
        appctx.binlog_name = log_test_input;
        if ((appctx.f_binlog = picoquic_file_open(log_test_input, "rb")) == NULL) {
            ret = -1;
        }
    }
    if (ret == 0) {
        ret = picoquic_get_input_path(svg_template, sizeof(svg_template), picoquic_solution_dir, PICOLOG_SVG_TEMPLATE);
        if (ret == 0) {
            if ((appctx.f_template = picoquic_file_open(svg_template, "r")) == NULL) {
                ret = -1;
            }
        }
    }

    if (ret == 0) {
        cids = cidset_create();
        if (cids == NULL) {
            ret = -1;
        }
        else {
            binlog_list_cids(appctx.f_binlog, cids);
            if (cids->count == 0) {
                ret = -1;
            }
        }
    }
    
    if (ret == 0) {
        FILE* cid_prints;
        if ((cid_prints = picoquic_file_open(CIDSET_OUTPUT, "w")) == NULL) {
            ret = -1;
        }
        else {
            cidset_print(cid_prints, cids);
            (void)picoquic_file_close(cid_prints);
        }
    }

    if (ret == 0) {
        picoquic_connection_id_t cid_test = { {11, 12, 13, 14, 15, 16, 17, 18}, 8 };

        if (cidset_has_cid(cids, &cid_test)) {
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = cidset_iterate(cids, test_convert_svg, &appctx);
    }

    (void)picoquic_file_close(appctx.f_binlog);
    (void)picoquic_file_close(appctx.f_template);
    if (cids != NULL) {
        (void)cidset_delete(cids);
    }
    /* compare the log file to the expected value */
    if (ret == 0)
    {
        char svglog_ref[512];

        ret = picoquic_get_input_path(svglog_ref, sizeof(svglog_ref), picoquic_solution_dir, SVG_LOG_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the svglog test ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(SVG_LOG_OUTPUT, svglog_ref);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn picolog_basic() {
    let mut _simulated_time = Instant::from_ticks(0);

    let log_input =
        get_input_path("picoquictest/picolog_test_input.log").expect("get log input path");
    let svg_template_path = get_input_path("loglib/template.svg").expect("get svg template path");

    let mut f_binlog = std::fs::File::open(&log_input).expect("open binlog");
    let f_template = std::fs::File::open(&svg_template_path).expect("open svg template");

    let mut cids = cidset_create().expect("cidset_create");

    binlog_list_cids(&mut f_binlog, &mut cids);
    assert!(!cids.is_empty(), "cids must not be empty");

    {
        let mut cid_prints = std::fs::File::create("./cidset.txt").expect("create cidset output");
        cidset_print(&mut cid_prints, &cids);
    }

    let cid_test = crate::ConnectionId::clone_from_slice(&[11, 12, 13, 14, 15, 16, 17, 18])
        .expect("build test CID");
    assert!(!cidset_has_cid(&cids, &cid_test), "unexpected CID in set");

    let mut ctx = SvgConvertCtx {
        binlog_name: log_input.clone(),
        out_dir: ".",
        f_binlog: Some(std::fs::File::open(&log_input).expect("reopen binlog")),
        f_template: Some(f_template),
    };
    let ret = cidset_iterate(&cids, svg_convert, &mut ctx);
    assert_eq!(ret, 0, "cidset_iterate failed");

    let svglog_ref = get_input_path("picoquictest/svglog_ref.svg").expect("get svglog ref path");
    compare_text_files("./0102030405060708.svg", &svglog_ref)
        .expect("svg output matches reference");
}
```

## `picoquictest/quic_tester.c:initial_ping_ack_test`
* C test-table name: `initial_ping_ack`
* C entry function: `initial_ping_ack_test`
* Rust test: `initial_ping_ack`
* C source: `picoquictest/quic_tester.c:267-345`
* Rust source: `rs/fq/src/tests/quic_tester.rs:66-132`

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

### Rust test body
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
