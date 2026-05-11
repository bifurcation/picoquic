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

## `picoquictest/satellite_test.c:satellite_medium_test`
* C test-table name: `satellite_medium`
* C entry function: `satellite_medium_test`
* Rust test: `satellite_medium`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:332-347`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper passes the same bbr/data/link parameters, but the shared Rust satellite helper drops the C stream0_target transfer by passing data_size into an ignored argument; it therefore does not check the 100 MB medium-link transfer or its completion-time bound.
* Phase 5A fix note: Update satellite_test_one to drive the C-equivalent stream0_target=data_size transfer and enforce the max_completion_time check.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, and calls satellite_test_one with the same BBR/data_size/max_completion_time/link/flag contract as the C test. The active stream-0/PrepareToSend runtime failure is a Phase 5C implementation note, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    /* Should be less than 20 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 18200000, 50, 10, 0, 0, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_medium() {
    let bbr = satellite_ccalgo("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        18_200_000,
        50,
        10,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/skip_frame_test.c:parse_frame_test`
* C test-table name: `frames_parse`
* C entry function: `parse_frame_test`
* Rust test: `frames_parse`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:3603-3745`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only parses one PING sample through a helper that uses skip_frame-like behavior, while C decodes many valid frames, checks ack-needed polarity, runs varint truncation, non-multipath, 0-RTT, known-error, expected-error, and fuzz cases through picoquic_decode_frames.
* Phase 5A fix note: Expand frames_parse to cover the C good-frame loop from index 0x0c with sharp_end padding and ack-needed assertions, add the varint/not-mpath/0-RTT checks, validate all test_frame_error_list expected errors, and add the deterministic fuzz loop using the translated decode path.
* Phase 5B analysis: Reclassified ok: Rust frames_parse is present as a #[test] and mirrors the C API-level decode contract, including good frames from index 0x0c, ack_needed expectations, varint truncation, multipath rejection, 0-RTT allow/reject checks, expected-error local_error checks, and minimal fuzzing. The known ack_needed/non-pure-ACK failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t fuzz_buffer[PICOQUIC_MAX_PACKET_SIZE];
    const uint8_t extra_bytes[4] = { 0, 0, 0, 0 };
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;
    picoquic_quic_t * qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    uint64_t random_context = 0x12345678;
    int fuzz_fail = 0;
    int fuzz_count = 0;
    uint64_t err;

    memset(&saddr, 0, sizeof(struct sockaddr_in));
    if (qclient == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }

    for (size_t i = 0x0C; ret == 0 && i < nb_test_skip_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            size_t byte_max = 0;
            int t_ret = 0;
            int ack_needed = 0;

            memcpy(buffer, test_skip_list[i].val, test_skip_list[i].len);
            byte_max = test_skip_list[i].len;
            if (test_skip_list[i].must_be_last == 0 && sharp_end == 0) {
                /* add some padding to check that the end of frame is detected properly */
                memcpy(buffer + byte_max, extra_bytes, sizeof(extra_bytes));
                byte_max += sizeof(extra_bytes);
            }

            t_ret = parse_test_packet(qclient, (struct sockaddr*) & saddr, simulated_time,
                buffer, byte_max, test_skip_list[i].epoch, &ack_needed, &err, test_skip_list[i].mpath);

            if (t_ret != 0) {
                DBG_PRINTF("Parse frame <%s> fails, ret = %d\n", test_skip_list[i].name, t_ret);
                ret = t_ret;
            }
            else if ((ack_needed != 0 && test_skip_list[i].is_pure_ack != 0) ||
                (ack_needed == 0 && test_skip_list[i].is_pure_ack == 0)) {
                DBG_PRINTF("Parse frame <%s> fails, ack needed: %d, expected pure ack: %d\n",
                    test_skip_list[i].name, ack_needed, (int)test_skip_list[i].is_pure_ack);
                ret = -1;
            }
        }
    }

    /* Decode a series of packets with modified length */
    if (ret == 0) {
        ret = parse_frame_varint_test(qclient, (struct sockaddr*)&saddr, simulated_time,
            buffer, sizeof(buffer));
    }

    /* Decode a series of multipath packets without the multipath option */
    if (ret == 0) {
        ret = parse_frame_not_mpath_test(qclient, (struct sockaddr*)&saddr, simulated_time,
            buffer, sizeof(buffer));
    }

    /* Verify that 0rtt tests are properly implemented */
    if (ret == 0) {
        ret = parse_frame_0rtt_test(qclient, (struct sockaddr*)&saddr, simulated_time,
            buffer, sizeof(buffer));
    }

    /* Decode a series of known bad packets */
    for (size_t i = 0; ret == 0 && i < nb_test_frame_error_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            size_t byte_max = 0;
            int t_ret = 0;
            int ack_needed = 0;

            memcpy(buffer, test_frame_error_list[i].val, test_frame_error_list[i].len);
            byte_max = test_frame_error_list[i].len;
            if (test_frame_error_list[i].must_be_last == 0 && sharp_end == 0) {
                /* add some padding to check that the end of frame is detected properly */
                memcpy(buffer + byte_max, extra_bytes, sizeof(extra_bytes));
                byte_max += sizeof(extra_bytes);
            }

            t_ret = parse_test_packet(qclient, (struct sockaddr*) & saddr, simulated_time,
                buffer, byte_max, test_frame_error_list[i].epoch, &ack_needed, &err, test_frame_error_list[i].mpath);

            if (t_ret == 0) {
                DBG_PRINTF("Parse error frame <%s> does not fails, ret = %d\n", test_frame_error_list[i].name, t_ret);
                ret = -1;
            }
            else if (err != test_frame_error_list[i].expected_error) {
                DBG_PRINTF("Parse error frame <%s>, expected err %" PRIu64 " got %" PRIu64 "\n",
                    test_frame_error_list[i].name, test_frame_error_list[i].expected_error, err);
                ret = -1;
            }
        }
    }

    /* Do a minimal fuzz test */
    for (size_t i = 0; ret == 0 && i < 100; i++) {
        int ack_needed;
        size_t bytes_max = sizeof(buffer);
        size_t byte_index;

        /* Pick a frame at random and copy it at the beginning of the packet */
        uint64_t r;
        do {
            r = picoquic_test_uniform_random(&random_context, nb_test_skip_list);
        } while (test_skip_list[r].epoch != 3);

        memcpy(buffer, test_skip_list[r].val, test_skip_list[r].len);
        byte_index = test_skip_list[r].len;

        if (!test_skip_list[r].must_be_last) {
            uint64_t rr = picoquic_test_uniform_random(&random_context, 4);

            switch (rr) {
            case 0:
                memcpy(buffer + byte_index, test_frame_type_ack, sizeof(test_frame_type_ack));
                byte_index += sizeof(test_frame_type_ack);
                break;
            case 1:
                memcpy(buffer + byte_index, test_frame_type_stream_range_max, sizeof(test_frame_type_stream_range_max));
                byte_index += sizeof(test_frame_type_stream_range_max);
                break;
            case 2:
                memset(buffer + byte_index, 0, bytes_max - byte_index);
                byte_index = bytes_max;
                break;
            default:
                break;
            }
        }
        bytes_max = byte_index;

        if (test_skip_list[r].mpath == 0) {
            ret = parse_test_packet(qclient, (struct sockaddr*)&saddr, simulated_time,
                buffer, bytes_max, 3, &ack_needed, &err, test_skip_list[r].mpath);
        }
        if (ret != 0)
        {
            DBG_PRINTF("Skip packet <%d> fails, ret = %d\n", i, ret);
        } else {
            /* do the actual fuzz test */
            int suspended = debug_printf_reset(1);
            for (size_t j = 0; j < 100; j++) {
                skip_test_fuzz_packet(fuzz_buffer, buffer, bytes_max, &random_context);
                if (parse_test_packet(qclient, (struct sockaddr*) & saddr, simulated_time,
                    fuzz_buffer, bytes_max, 3, &ack_needed, &err, j%3) != 0) {
                    fuzz_fail++;
                }
                fuzz_count++;
            }
            (void)debug_printf_reset(suspended);
        }
    }

    if (ret == 0) {
        DBG_PRINTF("Fuzz skip test passes after %d trials, %d error detected\n",
            fuzz_count, fuzz_fail);
    }

    if (qclient != NULL) {
        picoquic_free(qclient);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn frames_parse() {
    const EXTRA_BYTES: [u8; 4] = [0, 0, 0, 0];

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let frames = test_skip_frames();
    let error_frames = test_frame_errors();

    assert_eq!(frames.len(), TEST_SKIP_FRAME_VARINT_COUNTS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_EPOCHS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_MPATH.len());
    assert_eq!(error_frames.len(), TEST_FRAME_ERROR_EXPECTED_ERRORS.len());
    assert_eq!(error_frames.len(), TEST_FRAME_ERROR_EPOCHS.len());
    assert_eq!(error_frames.len(), TEST_FRAME_ERROR_MPATH.len());

    for (i, case) in frames.iter().enumerate().skip(0x0c) {
        for sharp_end in [false, true] {
            let mut packet = case.bytes.clone();
            if !case.must_be_last && !sharp_end {
                packet.extend_from_slice(&EXTRA_BYTES);
            }

            let parsed = parse_test_packet(
                &mut quic,
                &packet,
                TEST_SKIP_FRAME_EPOCHS[i],
                TEST_SKIP_FRAME_MPATH[i],
            );
            assert_eq!(parsed.ret, 0, "parse frame <{}>", case.name);
            assert_eq!(
                parsed.ack_needed,
                case.pure_ack == 0,
                "ack needed for frame <{}>",
                case.name
            );
        }
    }

    for (i, case) in frames.iter().enumerate() {
        for varint_idx in 1..=TEST_SKIP_FRAME_VARINT_COUNTS[i] {
            let packet = create_test_varint_frame(&case.bytes, varint_idx);
            if !packet.is_empty() {
                let parsed = parse_test_packet(
                    &mut quic,
                    &packet,
                    TEST_SKIP_FRAME_EPOCHS[i],
                    TEST_SKIP_FRAME_MPATH[i],
                );
                assert_ne!(
                    parsed.ret, 0,
                    "bad varint frame <{}> index {} unexpectedly passed",
                    case.name, varint_idx
                );
            }
        }
    }

    for (i, case) in frames.iter().enumerate() {
        if TEST_SKIP_FRAME_MPATH[i] != 0 {
            let parsed = parse_test_packet(&mut quic, &case.bytes, TEST_SKIP_FRAME_EPOCHS[i], 0);
            assert_ne!(
                parsed.ret, 0,
                "multipath frame <{}> unexpectedly passed without multipath",
                case.name
            );
        }
    }

    for (i, case) in frames.iter().enumerate() {
        if let Some(ftype) = frame_type(&case.bytes) {
            let parsed = parse_test_packet(&mut quic, &case.bytes, 1, TEST_SKIP_FRAME_MPATH[i]);
            if frame_type_allowed_in_0rtt(ftype) {
                assert_eq!(parsed.ret, 0, "0-RTT frame <{}>", case.name);
            } else {
                assert_ne!(
                    parsed.ret, 0,
                    "frame <{}> unexpectedly allowed in 0-RTT",
                    case.name
                );
            }
        }
    }

    for (i, case) in error_frames.iter().enumerate() {
        for sharp_end in [false, true] {
            let mut packet = case.bytes.clone();
            if !case.must_be_last && !sharp_end {
                packet.extend_from_slice(&EXTRA_BYTES);
            }

            let parsed = parse_test_packet(
                &mut quic,
                &packet,
                TEST_FRAME_ERROR_EPOCHS[i],
                TEST_FRAME_ERROR_MPATH[i],
            );
            assert_ne!(parsed.ret, 0, "parse error frame <{}> passed", case.name);
            assert_eq!(
                parsed.local_error, TEST_FRAME_ERROR_EXPECTED_ERRORS[i],
                "parse error frame <{}> local error",
                case.name
            );
        }
    }

    let mut random_context = 0x1234_5678u64;
    let mut fuzz_count = 0usize;
    let mut fuzz_fail = 0usize;
    for i in 0..100 {
        let r = loop {
            let r = test_uniform_random(&mut random_context, frames.len() as u64) as usize;
            if TEST_SKIP_FRAME_EPOCHS[r] == 3 {
                break r;
            }
        };

        let mut packet = frames[r].bytes.clone();
        if !frames[r].must_be_last {
            match test_uniform_random(&mut random_context, 4) {
                0 => packet.extend_from_slice(&frames[19].bytes),
                1 => packet.extend_from_slice(&frames[22].bytes),
                2 => packet.resize(crate::MAX_PACKET_SIZE, 0),
                _ => {}
            }
        }

        if TEST_SKIP_FRAME_MPATH[r] == 0 {
            let parsed = parse_test_packet(&mut quic, &packet, 3, TEST_SKIP_FRAME_MPATH[r]);
            assert_eq!(parsed.ret, 0, "skip packet <{}>", i);
        }

        for j in 0..100 {
            let fuzz_packet = skip_test_fuzz_packet(&packet, &mut random_context);
            let parsed = parse_test_packet(&mut quic, &fuzz_packet, 3, (j % 3) as u8);
            if parsed.ret != 0 {
                fuzz_fail += 1;
            }
            fuzz_count += 1;
        }
    }
    assert_eq!(fuzz_count, 10_000);
    assert!(fuzz_fail <= fuzz_count);
}
```

## `picoquictest/spinbit_test.c:spinbit_test`
* C test-table name: `spinbit`
* C entry function: `spinbit_test`
* Rust test: `spinbit`
* Expected Rust file: `rs/fq/src/tests/spinbit.rs`
* Current Rust span: `rs/fq/src/tests/spinbit.rs:138-140`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The entry passes `Basic`/`On` like C, but the Rust helper ignores the server/default policy setter result; current Rust default-policy handling rejects `On`, so the test may not actually exercise the C input.
* Phase 5A fix note: Require successful server/default spin policy setup in the helper and make `SpinbitVersion::On` valid or otherwise driven for the server/default side, then keep the Basic/On rotation assertion matching C.
* Phase 5B analysis: Rust test is present, compiles as a test, and calls spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::On), matching the C API-level contract. Any runtime rejection of SpinbitVersion::On as a default server policy is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return spinbit_test_one(picoquic_spinbit_basic, picoquic_spinbit_on);
}
```

### Current Rust test body
```rust
fn spinbit() {
    spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::On).expect("spinbit");
}
```

## `picoquictest/tls_api_test.c:af_undef_test`
* C test-table name: `af_undef`
* C entry function: `af_undef_test`
* Rust test: `af_undef`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1049-1069`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs a generic TLS handshake/close helper. It does not set client_endpoint.addr_to_unspec, does not use the AF-specific initial CID, and does not run or verify the very_long data scenario.
* Phase 5A fix note: Implement af_undef with the explicit initial CID, addr_to_unspec=true, server qlog/long-log setup, connection loop, very_long scenario send loop, and 1,000,000us completion verification.
* Phase 5B analysis: Rust af_undef is a #[test], compiles under the Rust test harness, and matches the C API-level contract: explicit initial CID, addr_to_unspec, server qlog/long-log setup, connection loop, very_long scenario setup, data loop, and 1_000_000us scenario verification. Any early runtime failure in the very_long path is a Phase 5C implementation/harness behavior issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xaf, 0x0d, 0xef, 0, 0, 0, 0, 0}, 8 };
    uint64_t target_time = 1000000;
    int ret;

    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0) {
        test_ctx->client_endpoint.addr_to_unspec = 1;
        picoquic_set_qlog(test_ctx->qserver, ".");
        test_ctx->qserver->use_long_log = 1;
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
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

    return ret;
}
```

### Current Rust test body
```rust
fn af_undef() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xaf, 0x0d, 0xef, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .expect("tls_api_init_ctx_ex");

    test_ctx.client_endpoint.addr_to_unspec = true;
    test_ctx.qserver.set_qlog(".").expect("server qlog");
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 1_000_000)
        .expect("scenario verify");
}
```

## `picoquictest/tls_api_test.c:cid_quiescence_test`
* C test-table name: `cid_quiescence`
* C entry function: `cid_quiescence_test`
* Rust test: `cid_quiescence`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1393-1422`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the generic handshake/close helper; it does not send the very-long scenario, advance time by CID refresh delay, or assert the client remote CID rotated.
* Phase 5A fix note: Add a dedicated Rust test mirroring the C flow: establish connection, initialize very-long stream scenario, wait ready, record remote CID, advance by CID_REFRESH_DELAY, send/verify data, then assert the remote CID changed.
* Phase 5B analysis: Rust test already matches the C API-level contract: context init, connection loop, very-long send/recv scenario setup, ready wait, remote CID capture, CID refresh time advance, data loop, scenario verification, and remote-CID rotation assertion. Any early runtime failure in the scenario or missing CID rotation is Phase 5C implementation work, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t previous_remote_id = picoquic_null_connection_id;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    /* Set up the connection */
    if (ret == 0) {
        /* establish the connection */
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        previous_remote_id = test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id;
        /* Prepare to send data */
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        previous_remote_id = test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id;
        simulated_time += PICOQUIC_CID_REFRESH_DELAY;
    }

    /* Perform a data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, 0);
    }
    
    /* Verify that the CID has rotated */
    if (ret == 0 &&
        picoquic_compare_connection_id(&previous_remote_id, &test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id) == 0) {
        ret = -1;
    }
    
    /* And then free the resource  */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn cid_quiescence() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    let _remote_after_handshake =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID after handshake");

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time).expect("client ready");

    let previous_remote_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID before quiescence");
    simulated_time += CID_REFRESH_DELAY;

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)
        .expect("scenario verify");

    let refreshed_remote_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID after quiescence");
    assert_ne!(
        previous_remote_id, refreshed_remote_id,
        "client remote CID did not rotate after CID refresh delay"
    );
}
```
