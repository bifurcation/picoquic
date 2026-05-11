# Phase 5B repair Rust test mismatches

You are repairing Phase 5A `needs_fix` entries.  The goal
is to make the Rust tests faithfully check the same behavior
as the C tests.

Rules:

* Edit Rust tests, Rust test helpers, and Rust test fixtures
  only: `rs/fq/src/tests/` and `rs/fq/tests/fixtures/`.
* Do not edit C sources.
* Do not weaken assertions, skip important C cases, or replace
  tests with placeholders.
* If the test already matches after closer inspection, report
  `ok` and do not edit source.
* Phase 5B is about test/API correspondence, not test success.
  The Rust test must exist, compile as a test, and be runnable
  by the Rust test harness, but it may fail arbitrarily early
  because the Rust library implementation is incomplete.
* Do not report `blocked` merely because the implementation
  returns the wrong state, fails a handshake, lacks protocol
  behavior, or would fail the test. Those are Phase 5C issues.
* Report `blocked` only when the faithful test cannot be
  written, compiled, or exposed as a runnable Rust test because
  the necessary Rust API/test-harness surface is missing or
  ambiguous.
* Do not run full `cargo test` in this pass. Use source review
  and, if needed, `cargo check --tests` for compile validation.

Owned Rust test file(s): `rs/fq/src/tests/mediatest.rs`, `rs/fq/src/tests/skip_frame.rs`, `rs/fq/src/tests/wifitest.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/mediatest.c:mediatest_wifi_test`
* C test-table name: `mediatest_wifi`
* C entry function: `mediatest_wifi_test`
* Rust test: `mediatest_wifi`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Rust span: `rs/fq/src/tests/mediatest.rs:1506-1525`
* Phase 5A analysis: The Rust spec fields match the C entry, but the shared Rust mediatest driver misses the C media simulation/stat checks and does not assert the wifi-specific path-quality loss, spurious-loss, and timer-loss conditions.
* Phase 5A fix note: Repair Rust mediatest_one as above, reproduce the back-to-back suspension/drain semantics for suspension_up_time=0, and add the wifi-specific path quality assertions for lost, spurious_losses, and timer_losses.
* Phase 5C outcome: blocked
* Phase 5C analysis: Rust wrapper and mediatest driver match the C spec, stats checks, back-to-back suspension drain, and Wi-Fi path-quality assertions, but shared util.rs is merge-corrupted so the test tree is not exposed as runnable.
* Phase 5C fix note: Repair shared TestTlsApiCtx harness corruption: duplicate send_buffer_size/use_udp_gso fields and misplaced helper methods outside the impl; no mediatest semantic mismatch observed.

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.link_latency = 15000;
    spec.latency_average = 60000;
    spec.latency_max = 350000;
    spec.priority_limit_for_bypass = 5;
    spec.do_not_check_video2 = 1;
    spec.nb_suspensions = 20;
    spec.suspension_start_time = 4000000;
    spec.suspension_down_time = 150000;
    spec.suspension_up_time = 0;

    ret = mediatest_one(mediatest_wifi, &spec);

    return ret;
}
```

### Current Rust test body
```rust
    mediatest_one(MediatestId::Video2Probe, &spec).expect("mediatest_video2_probe");
}

/// Wi-Fi jitter test with periodic suspension intervals.
/// C: `mediatest_wifi_test`.
#[test]
fn mediatest_wifi() {
    let spec = MediatestSpec {
        ccalgo: Some(mediatest_bbr_algorithm()),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        link_latency: 15_000,
        latency_average: 60_000,
        latency_max: 350_000,
        priority_limit_for_bypass: 5,
        do_not_check_video2: true,
        nb_suspensions: 20,
        suspension_start_time: 4_000_000,
```

## `picoquictest/skip_frame_test.c:logger_test`
* C test-table name: `logger`
* C entry function: `logger_test`
* Rust test: `logger`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Rust span: `rs/fq/src/tests/skip_frame.rs:4364-4366`
* Phase 5A analysis: Rust only opens a text log, creates one connection, logs a new connection and one app message. It omits the C test's frame corpus logging, reference log comparison, TLS ticket, packet/PDU logging, randomized no-Unknown checks, known bad-frame logging, and fuzz logging loops.
* Phase 5A fix note: Expand run_logger_test to cover the C logger_test scenarios: skip/error frame tables, reference text comparison, TLS ticket/app messages, packet/PDU logs, random packet Unknown-frame scan, bad-frame padding cases, and fuzz loops.
* Phase 5C outcome: blocked
* Phase 5C analysis: Rust logger test still only checks set_textlog/app messages. C requires direct textlog frame-corpus, bad-frame, random, and fuzz logging via picoquic_textlog_frames; the Rust equivalent write_textlog_frames exists only as private textlog.rs internals, not a callable skip_frame.rs/API harness surface.
* Phase 5C fix note: Expose a Rust textlog frame logging test/API surface, then expand run_logger_test to cover the C frame corpus, reference comparison, TLS ticket, packet/PDU logging, random Unknown scan, bad-frame padding, and fuzz loops.

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

### Current Rust test body
```rust
fn logger() {
    run_logger_test().expect("logger_test");
}
```

## `picoquictest/wifitest.c:wifi_bbr_long_test`
* C test-table name: `wifi_bbr_long`
* C entry function: `wifi_bbr_long_test`
* Rust test: `wifi_bbr_long`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Rust span: `rs/fq/src/tests/wifitest.rs:165-176`
* Phase 5A analysis: Rust passes the same Wi-Fi spec values, but the shared Rust scenario verification only closes the connection and does not verify stream completion or target completion time like C.
* Phase 5A fix note: Restore tls_api_one_scenario_body_verify/test scenario checks for all stream byte counts/completion and max_completion_microsec, keeping the BBR long spec and rtt_max suspension check.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test and wifi_test_one faithfully mirror the C BBR-long spec, suspensions, receive-block simulation, completion bound, stream verification, and rtt_max check, but shared util.rs merge artifacts prevent clean harness compilation.
* Phase 5C fix note: Deduplicate TestTlsApiCtx fields/initializers and leave one correctly scoped set_send_buffer_size in rs/fq/src/tests/util.rs; keep the current wifi_bbr_long logic.

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_basic,
        50000,
        suspension_basic,
        picoquic_bbr_algorithm,
        NULL,
        3400000,
        1,
        0 };
    int ret = wifi_test_one(wifi_test_bbr_long, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_bbr_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr",
        cc_algo_option: None,
        target_time: 3_400_000,
        simulate_receive_block: true,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR_LONG, &spec).expect("wifi_bbr_long");
}
```
