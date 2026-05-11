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

## `picoquictest/pn2pn64test.c:pn2pn64test`
* C test-table name: `pn2pn64`
* C entry function: `pn2pn64test`
* Rust test: `pn2pn64`
* C source: `picoquictest/pn2pn64test.c:73-89`
* Rust source: `rs/fq/src/tests/pn2pn64test.rs:239-248`

### C test body
```c
{
    int ret = 0;

    for (size_t i = 0; i < nb_test_entries; i++) {
        uint64_t pn64 = picoquic_get_packet_number64(
            test_entries[i].highest,
            test_entries[i].mask,
            test_entries[i].pn);

        if (pn64 != test_entries[i].expected) {
            ret = -1;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn pn2pn64() {
    for (i, case) in CASES.iter().enumerate() {
        let got = get_packet_number64(case.highest, case.mask, case.pn);
        assert_eq!(
            got, case.expected,
            "case {i}: highest={:#x} mask={:#x} pn={:#x}",
            case.highest, case.mask, case.pn,
        );
    }
}
```

## `picoquictest/sacktest.c:sendacktest`
* C test-table name: `ack_send`
* C entry function: `sendacktest`
* Rust test: `ack_send`
* C source: `picoquictest/sacktest.c:365-419`
* Rust source: `rs/fq/src/tests/sacktest.rs:632-695`

### C test body
```c
{
    int ret = 0;
    picoquic_quic_t* quic;
    picoquic_cnx_t * cnx;
    uint64_t current_time;
    uint64_t received_mask = 0;
    uint64_t previous_mask = 0;
    uint8_t bytes[256];
    picoquic_packet_context_enum pc = 0;

    if (picoquic_test_set_minimal_cnx(&quic, &cnx) != 0) {
        return -1;
    }
    cnx->sending_ecn_ack = 0; /* don't write an ack_ecn frame */
    
    if (check_ack_ranges(&cnx->ack_ctx[pc].sack_list) != 0) {
        ret = -1;
    }

    for (size_t i = 0; ret == 0 && i < nb_test_pn64; i++) {
        current_time = ((uint64_t)i) * 100;

        if (picoquic_record_pn_received(cnx, pc, cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[i], current_time) != 0) {
            ret = -1;
        }

        if (check_ack_ranges(&cnx->ack_ctx[pc].sack_list) != 0) {
            ret = -1;
        }

        if (ret == 0) {
            int more_data = 0;
            uint8_t* bytes_next = picoquic_format_ack_frame(cnx, bytes, bytes + sizeof(bytes), &more_data, 0, pc, 0);

            received_mask |= 1ull << (test_pn64[i] & 63);

            if (check_ack_ranges(&cnx->ack_ctx[pc].sack_list) != 0) {
                ret = -1;
            }

            if (ret == 0) {
                ret = basic_ack_parse(bytes, bytes_next - bytes, &expected_ack[i], &previous_mask, received_mask);
            }

            if (ret != 0) {
                ret = -1; /* useless code, but helps with checkpointing */
            }
        }
    }

    picoquic_test_delete_minimal_cnx(&quic, &cnx);

    return ret;
}
```

### Rust test body
```rust
fn ack_send() {
    let pc = PacketContext::Application;
    let t0 = Instant::from_ticks(0);
    let mut quic = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("quic");
    let cnx = quic
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            None,
            t0,
            0,
            Some(util::TEST_SNI),
            Some("minimal"),
            true,
        )
        .expect("cnx");

    cnx.sending_ecn_ack = false;
    let l_cid = cnx.create_local_connection_id(0, None, t0).expect("l_cid");

    util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

    let mut received_mask = 0u64;
    let mut previous_mask = 0u64;
    let mut bytes = [0u8; 256];

    for (i, &pn) in TEST_PNS.iter().enumerate() {
        let current_time = Instant::from_ticks(i as u64 * 100);

        assert_eq!(
            cnx.record_pn_received(pc, Some(l_cid), pn, current_time),
            0,
            "record pn {pn} at step {i}"
        );
        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        let mut more_data = 0i32;
        let written = util::format_ack_frame_written(cnx, &mut bytes, &mut more_data, t0, pc, 0)
            .expect("format_ack_frame at step {i}");

        received_mask |= 1u64 << (pn & 63);
        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        basic_ack_parse(
            &bytes[..written],
            &EXPECTED_ACKS[i],
            &mut previous_mask,
            received_mask,
        );
    }
}
```

## `picoquictest/satellite_test.c:satellite_loss_fc_test`
* C test-table name: `satellite_loss_fc`
* C entry function: `satellite_loss_fc_test`
* Rust test: `satellite_loss_fc`
* C source: `picoquictest/satellite_test.c:238-244`
* Rust source: `rs/fq/src/tests/satellite.rs:266-281`

### C test body
```c
{
    /* Should be less than 10 sec per draft etosat.
     * The flow control option sets the "max data" to 2 BDP, with the effect
     * of reducing the memory consumption, while the transmission is slowed. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 12500000, 250, 3, 0, 1, 0, 0, 0, 1);
}
```

### Rust test body
```rust
fn satellite_loss_fc() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        12_500_000,
        250,
        3,
        0,
        true,
        false,
        false,
        false,
        true,
    );
}
```

## `picoquictest/skip_frame_test.c:app_message_overflow_test`
* C test-table name: `app_message_overflow`
* C entry function: `app_message_overflow_test`
* Rust test: `app_message_overflow`
* C source: `picoquictest/skip_frame_test.c:3586-3660`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1062-1094`

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    picoquic_cnx_t* cnx = NULL;
    struct sockaddr_storage addr;
    const picoquic_connection_id_t initial_cid = { { 8, 9, 0, 1, 2, 3, 4, 5 }, 8 };
    const picoquic_connection_id_t dest_cid = { { 16, 17, 18, 19, 20, 21, 22, 23 }, 8 };
    char qlog_test_ref[512];
    int ret_qlog = picoquic_get_input_path(qlog_test_ref, sizeof(qlog_test_ref),
        picoquic_solution_dir, QLOG_OVERFLOW_REF);
    picoquic_quic_t* quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    if (quic == NULL || ret_qlog != 0) {
        ret = -1;
    }
    else {
        picoquic_set_binlog(quic, ".");
        ret = picoquic_store_text_addr(&addr, "10.0.0.1", 1234);
        if (ret == 0) {
            cnx = picoquic_create_cnx(quic, initial_cid, dest_cid, (struct sockaddr*) & addr,
                simulated_time, 0, "test-sni", "test-alpn", 1);
            if (cnx == NULL) {
                ret = -1;
            }
            else {
                picoquic_log_new_connection(cnx);
            }
        }
    }
    if (ret == 0) {
        char test[BYTESTREAM_MAX_BUFFER_SIZE];

        memset(test, 'x', sizeof(test) - 3);
        test[sizeof(test) - 3] = '!';
        test[sizeof(test) - 2] = 0;

        for (int i = 0; i < 16; i++) {
            picoquic_log_app_message(cnx, "s:%s", &test[15 - i]);
        }

        picoquic_delete_cnx(cnx);
    }

    picoquic_free(quic);

    if (ret == 0) {
        /* Convert to QLOG and verify */
        uint64_t log_time = 0;
        uint16_t flags = 0;
        FILE* f_binlog = picoquic_open_cc_log_file_for_read(qlog_overflow_bin, &flags, &log_time);

        if (f_binlog == NULL) {
            DBG_PRINTF("Cannot open binlog file: %s.", qlog_overflow_bin);
            ret = -1;
        }
        else {
            ret = qlog_convert(&initial_cid, f_binlog, qlog_overflow_file, NULL, ".", flags);
            if (ret != 0) {
                DBG_PRINTF("%s", "Cannot convert the binary log into QLOG.\n");
            }
            else {
                ret = picoquic_test_compare_text_files(qlog_overflow_file, qlog_test_ref);
                if (ret != 0) {
                    DBG_PRINTF("%s", "Unexpected content in QLOG log file.\n");
                }
            }
            picoquic_file_close(f_binlog);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn app_message_overflow() {
    use crate::ConnectionId as InternalCid;

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    quic.set_binlog(Some(".")).ok();

    let initial_cid =
        InternalCid::clone_from_slice(&[8, 9, 0, 1, 2, 3, 4, 5]).expect("initial CID");
    let dest_cid =
        InternalCid::clone_from_slice(&[16, 17, 18, 19, 20, 21, 22, 23]).expect("dest CID");
    let addr: core::net::SocketAddr = "10.0.0.1:1234".parse().unwrap();

    let mut cnx = quic
        .create_connection_with_cids(
            initial_cid,
            dest_cid,
            Some(&addr),
            simulated_time,
            "test-sni",
            "test-alpn",
        )
        .expect("cnx");

    cnx.log_new_connection();

    // Fill a large string and log it 16 times to exercise overflow handling.
    let fill = "x".repeat(65533);
    for i in 0..16usize {
        cnx.log_app_message(&format!("s:{}", &fill[15 - i.min(15)..]));
    }
}
```

## `picoquictest/skip_frame_test.c:skip_frame_test`
* C test-table name: `frames_skip`
* C entry function: `skip_frame_test`
* Rust test: `frames_skip`
* C source: `picoquictest/skip_frame_test.c:803-904`
* Rust source: `rs/fq/src/tests/skip_frame.rs:929-952`

### C test body
```c
{
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t fuzz_buffer[PICOQUIC_MAX_PACKET_SIZE];
    const uint8_t extra_bytes[4] = { 0xFF, 0, 0, 0 };
    uint64_t random_context = 0xBABED011;
    int fuzz_count = 0;
    int fuzz_fail = 0;
    picoquic_cnx_t cnx;

    memset(&cnx, 0, sizeof(cnx)); /* Null value gets default test version */

    for (size_t i = 0; ret == 0 && i < nb_test_skip_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            size_t consumed = 0;
            size_t byte_max = 0;
            int pure_ack;
            int t_ret = 0;

            memcpy(buffer, test_skip_list[i].val, test_skip_list[i].len);
            byte_max = test_skip_list[i].len;
            if (test_skip_list[i].must_be_last == 0 && sharp_end == 0) {
                memcpy(buffer + byte_max, extra_bytes, sizeof(extra_bytes));
                byte_max += sizeof(extra_bytes);
            }

            t_ret = picoquic_skip_frame(buffer, byte_max, &consumed, &pure_ack);

            if (t_ret != 0) {
                DBG_PRINTF("Skip frame <%s> fails, ret = %d\n", test_skip_list[i].name, t_ret);
                ret = t_ret;
            }
            else if (consumed != test_skip_list[i].len) {
                DBG_PRINTF("Skip frame <%s> fails, wrong length, %d instead of %d\n",
                    test_skip_list[i].name, (int)consumed, (int)test_skip_list[i].len);
                ret = -1;
            }
            else if (pure_ack != test_skip_list[i].is_pure_ack) {
                DBG_PRINTF("Skip frame <%s> fails, wrong pure ack, %d instead of %d\n",
                    test_skip_list[i].name, (int)pure_ack, (int)test_skip_list[i].is_pure_ack);
                ret = -1;
            }
        }
    }

    /* Check a series of known bad packets. We are checking that an error is
     * detected and no adverse code issue happens. */
    for (size_t i = 0; ret == 0 && i < nb_test_frame_error_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            size_t consumed = 0;
            size_t byte_max = 0;
            int pure_ack;
            int t_ret = 0;
            memcpy(buffer, test_frame_error_list[i].val, test_frame_error_list[i].len);
            byte_max = test_frame_error_list[i].len;
            if (test_frame_error_list[i].must_be_last == 0 && sharp_end == 0) {
                memcpy(buffer + byte_max, extra_bytes, sizeof(extra_bytes));
                byte_max += sizeof(extra_bytes);
            }

            t_ret = picoquic_skip_frame(buffer, byte_max, &consumed, &pure_ack);

            if (t_ret == 0 && test_frame_error_list[i].skip_fails) {
                DBG_PRINTF("Skip error frame <%s> does not fails, ret = %d\n", test_frame_error_list[i].name, t_ret);
                ret = -1;
            }
        }
    }
    /* Derive and test a series of packets with bad varint encodings */
    if (ret == 0) {
        ret = skip_frame_varint_test(buffer, PICOQUIC_MAX_PACKET_SIZE);
    }

    /* Do a minimal fuzz test */
    for (size_t i = 0; ret == 0 && i < 100; i++) {
        size_t bytes_max = format_random_packet(buffer, sizeof(buffer), &random_context, -1);

        ret = skip_test_packet(buffer, bytes_max);
        if (ret != 0) {
            DBG_PRINTF("Skip packet <%d> fails, ret = %d\n", i, ret);
        } else {
            /* do the actual fuzz test */
            int suspended = debug_printf_reset(1);
            for (size_t j = 0; j < 100; j++) {
                skip_test_fuzz_packet(fuzz_buffer, buffer, bytes_max, &random_context);
                if (skip_test_packet(fuzz_buffer, bytes_max) != 0) {
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

    return ret;
}
```

### Rust test body
```rust
fn frames_skip() {
    // Iterate over the internal `test_skip_list` via the public `skip_frame`
    // entry point.  Each frame is tested both with and without a trailing
    // guard region (the C "sharp_end" loop).

    let extra = [0xff, 0, 0, 0];
    for case in test_skip_frames() {
        for sharp_end in [false, true] {
            let mut frame = case.bytes.clone();
            let byte_max = if !case.must_be_last && !sharp_end {
                frame.extend_from_slice(&extra);
                frame.len()
            } else {
                case.bytes.len()
            };
            let mut consumed = 0usize;
            let mut pure_ack = 0i32;
            let ret = skip_frame(&frame, byte_max, &mut consumed, &mut pure_ack);
            assert_eq!(ret, 0, "skip_frame({})", case.name);
            assert_eq!(consumed, case.bytes.len(), "consumed({})", case.name);
            assert_eq!(pure_ack, case.pure_ack, "pure_ack({})", case.name);
        }
    }
}
```
