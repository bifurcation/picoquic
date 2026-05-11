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

## `picoquictest/qlog_frame_test.c:qlog_frames_test`
* C test-table name: `qlog_frames`
* C entry function: `qlog_frames_test`
* Rust test: `qlog_frames`
* C source: `picoquictest/qlog_frame_test.c:37-74`
* Rust source: `rs/fq/src/tests/qlog_frame.rs:632-655`

### C test body
```c
{
    int ret = 0;
    char qlog_frames_test_ref[512];
    char const * need_comma = "";
    FILE* F = picoquic_file_open(QLOG_FRAMES_TEST, "w");
    if (F == NULL) {
        return -1;
    }
    fprintf(F, "[\n");
    for (size_t i = 0; i < nb_test_skip_list; i++) {
        test_skip_frames_t* test = &test_skip_list[i];
        const uint8_t* bytes = test->val;
        const uint8_t* bytes_max = test->val + test->len;

        fprintf(F, "%s{ \"test\": \"%s\", \"frame\": ", need_comma, test->name);
        need_comma = ",\n";

        // Write one line per frame
        qlog_frames(F, bytes, bytes_max, 0);
        fprintf(F, "}");
    }
    fprintf(F, "\n]\n");
    (void)picoquic_file_close(F);


    ret = picoquic_get_input_path(qlog_frames_test_ref, sizeof(qlog_frames_test_ref), picoquic_solution_dir,
        QLOG_FRAMES_TEST_REF);

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the log ref file name.\n");
    }
    else {
        ret = picoquic_test_compare_text_files(qlog_frames_test_ref, QLOG_FRAMES_TEST);
    }

    return ret;
}
```

### Rust test body
```rust
fn qlog_frames() {
    let test_ref = get_input_path("picoquictest/qlog_frames_test_ref.txt")
        .expect("resolve qlog frames ref path");

    let mut out: Vec<u8> = Vec::new();
    let list = test_skip_list();

    let mut need_comma = "";
    out.extend_from_slice(b"[\n");
    for entry in &list {
        out.extend_from_slice(need_comma.as_bytes());
        out.extend_from_slice(format!("{{ \"test\": \"{}\", \"frame\": ", entry.name).as_bytes());
        need_comma = ",\n";

        qlog_frames_write(&mut out, entry.val, false);
        out.extend_from_slice(b"}");
    }
    out.extend_from_slice(b"\n]\n");

    let output_file = "qlog_frames_test.json";
    std::fs::write(output_file, &out).expect("write qlog frames test output");

    compare_text_files(&test_ref, output_file).expect("qlog frames output matches reference");
}
```

## `picoquictest/skip_frame_test.c:parse_frame_test`
* C test-table name: `frames_parse`
* C entry function: `parse_frame_test`
* Rust test: `frames_parse`
* C source: `picoquictest/skip_frame_test.c:1096-1265`
* Rust source: `rs/fq/src/tests/skip_frame.rs:956-962`

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

### Rust test body
```rust
fn frames_parse() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    let sample = &[0x01u8]; // PING frame
    parse_test_packet(&mut quic, sample, 3, false).expect("parse PING");
}
```

## `picoquictest/sockloop_test.c:sockloop_thread_name_test`
* C test-table name: `sockloop_thread_name`
* C entry function: `sockloop_thread_name_test`
* Rust test: `sockloop_thread_name`
* C source: `picoquictest/sockloop_test.c:722-733`
* Rust source: `rs/fq/src/tests/sockloop.rs:628-635`

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 8);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.use_background_thread = 1;
    spec.thread_name = "picoquic loop";

    return(sockloop_test_one(&spec));
}
```

### Rust test body
```rust
fn sockloop_thread_name() {
    let mut spec = SockloopTestSpec::new(8);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.use_background_thread = true;
    spec.thread_name = Some("picoquic loop");
    sockloop_test_one(&spec);
}
```

## `picoquictest/stream0_frame_test.c:TlsStreamFrameTest`
* C test-table name: `TlsStreamFrame`
* C entry function: `TlsStreamFrameTest`
* Rust test: `tlsstreamframe`
* C source: `picoquictest/stream0_frame_test.c:411-420`
* Rust source: `rs/fq/src/tests/stream0_frame.rs:694-698`

### C test body
```c
{
    int ret = 0;

    for (size_t i = 0; ret == 0 && i < nb_tls_test_cases; i++) {
        ret = TlsStreamFrameOneTest(&tls_test_case[i]);
    }

    return ret;
}
```

### Rust test body
```rust
fn tlsstreamframe() {
    for name in &["tlstest_v1", "tlstest_v2", "tlstest_v3", "tlstest_v4"] {
        tls_stream_frame_one_test(name).unwrap_or_else(|_| panic!("{name}"));
    }
}
```

## `picoquictest/ticket_store_test.c:token_reuse_api_test`
* C test-table name: `token_reuse_api`
* C entry function: `token_reuse_api_test`
* Rust test: `token_reuse_api`
* C source: `picoquictest/ticket_store_test.c:483-547`
* Rust source: `rs/fq/src/tests/ticket_store.rs:98-129`

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

### Rust test body
```rust
fn token_reuse_api() {
    let mut t = Instant::from_ticks(1_000_000);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    let tokens: &[&[u8]] = &[
        b"token_one",
        b"token_two",
        b"token_three",
        b"token_four",
        b"token_five",
        b"token_six",
        b"token_seven",
    ];

    for token in tokens {
        ctx.qserver
            .registered_token_check_reuse(token, token.len(), 2_000_000)
            .expect("check_reuse first");
        ctx.qserver
            .registered_token_check_reuse(token, token.len(), 2_000_000)
            .expect_err("duplicate should fail");
    }

    t = Instant::from_ticks(3_000_000);
    ctx.qserver.registered_token_clear(t);

    for token in tokens {
        ctx.qserver
            .registered_token_check_reuse(token, token.len(), 4_000_000)
            .expect("check_reuse after clear");
    }
}
```
