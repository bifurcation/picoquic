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

## `picoquictest/sacktest.c:sacktest`
* C test-table name: `ack_sack`
* C entry function: `sacktest`
* Rust test: `ack_sack`
* C source: `picoquictest/sacktest.c:147-240`
* Rust source: `rs/fq/src/tests/sacktest.rs:500-625`

### C test body
```c
{
    int ret = 0;
    picoquic_cnx_t *cnx;
    picoquic_quic_t* quic;
    uint64_t current_time = 0;
    uint64_t highest_seen = 0;
    uint64_t highest_seen_time = 0;
    picoquic_packet_context_enum pc = 0;

    if (picoquic_test_set_minimal_cnx(&quic, &cnx) != 0) {
        return -1;
    }

    if (picoquic_create_local_cnxid(cnx, 0, NULL, 0) == NULL) {
        return -1;
    }

    /* Do a basic test with packet zero */
    if (picoquic_is_pn_already_received(cnx, pc,
        cnx->first_local_cnxid_list->local_cnxid_first, 0) != 0) {
        ret = -1;
    }
    else if (picoquic_record_pn_received(cnx, pc, cnx->first_local_cnxid_list->local_cnxid_first,
        0, current_time) != 0) {
        ret = -1;
    }
    else if (picoquic_is_pn_already_received(cnx, pc,
        cnx->first_local_cnxid_list->local_cnxid_first, 0) == 0) {
        ret = -1;
    }
    else if (picoquic_sack_list_first(&cnx->ack_ctx[pc].sack_list) != 0 ||
        picoquic_sack_list_last(&cnx->ack_ctx[pc].sack_list) != 0 ||
        picoquic_sack_list_first_range(&cnx->ack_ctx[pc].sack_list) != NULL) {
        ret = -1;
    }
    else {
        /* reset for the next test */
        picoquic_test_reset_minimal_cnx(quic, &cnx);
    }

    if (ret == 0) {
        ret = check_ack_ranges(&cnx->ack_ctx[pc].sack_list);
    }

    for (size_t i = 0; ret == 0 && i < nb_test_pn64; i++) {
        current_time = ((uint64_t)i) * 100 + 1;

        if (test_pn64[i] > highest_seen) {
            highest_seen = test_pn64[i];
            highest_seen_time = current_time;
        }

        if (picoquic_record_pn_received(cnx, pc, cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[i], current_time) != 0) {
            ret = -1;
        }

        if (ret == 0) {
            ret = check_ack_ranges(&cnx->ack_ctx[pc].sack_list);
        }

        for (size_t j = 0; ret == 0 && j <= i; j++) {
            if (picoquic_is_pn_already_received(cnx, pc,
                cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[j]) == 0) {
                ret = -1;
            }

            if (picoquic_record_pn_received(cnx, pc, cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[j], current_time) != 1) {
                ret = -1;
            }
        }

        for (size_t j = i + 1; ret == 0 && j < nb_test_pn64; j++) {
            if (picoquic_is_pn_already_received(cnx, pc,
                cnx->first_local_cnxid_list->local_cnxid_first, test_pn64[j]) != 0) {
                ret = -1;
            }
        }
    }

    if (ret == 0) {
        if (picoquic_sack_list_last(&cnx->ack_ctx[pc].sack_list) != 21 ||
            picoquic_sack_list_first(&cnx->ack_ctx[pc].sack_list) != 0 ||
            cnx->ack_ctx[pc].time_stamp_largest_received != highest_seen_time ||
            picoquic_sack_list_first_range(&cnx->ack_ctx[pc].sack_list) != NULL) {
            ret = -1;
        }
    }

    /* Free the sack lists*/
    picoquic_test_delete_minimal_cnx(&quic, &cnx);

    return ret;
}
```

### Rust test body
```rust
fn ack_sack() {
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

    // Phase 1: basic pn=0 case (mirrors C before the reset).
    {
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

        let l_cid = cnx.create_local_connection_id(0, None, t0).expect("l_cid");

        assert!(
            !cnx.is_pn_already_received(pc, Some(l_cid), 0),
            "pn 0 should not be received yet"
        );
        assert_eq!(
            cnx.record_pn_received(pc, Some(l_cid), 0, t0),
            0,
            "first record of pn 0 should return 0"
        );
        assert!(
            cnx.is_pn_already_received(pc, Some(l_cid), 0),
            "pn 0 should now be received"
        );
        // C: picoquic_sack_list_first == 0 (min PN), == Rust last()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.last(), 0);
        // C: picoquic_sack_list_last == 0 (max PN), == Rust first()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.first(), 0);
        assert!(cnx.ack_ctx[pc as usize].sack_list.first_range().is_none());
    }

    // Phase 2: fresh connection (mirrors picoquic_test_reset_minimal_cnx).
    {
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
            .expect("cnx after reset");

        let l_cid = cnx
            .create_local_connection_id(0, None, t0)
            .expect("l_cid after reset");

        util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

        let mut highest_seen = 0u64;
        let mut highest_seen_time = t0;

        for (i, &pn) in TEST_PNS.iter().enumerate() {
            let current_time = Instant::from_ticks(i as u64 * 100 + 1);

            if pn > highest_seen {
                highest_seen = pn;
                highest_seen_time = current_time;
            }

            assert_eq!(
                cnx.record_pn_received(pc, Some(l_cid), pn, current_time),
                0,
                "record pn {pn} at step {i}"
            );
            util::check_ack_ranges(&mut cnx.ack_ctx[pc as usize].sack_list);

            for &pn_j in &TEST_PNS[..=i] {
                assert!(
                    cnx.is_pn_already_received(pc, Some(l_cid), pn_j),
                    "pn {pn_j} should be received at step {i}"
                );
                assert_eq!(
                    cnx.record_pn_received(pc, Some(l_cid), pn_j, current_time),
                    1,
                    "duplicate record of pn {pn_j} should return 1 at step {i}"
                );
            }

            for &pn_j in &TEST_PNS[i + 1..] {
                assert!(
                    !cnx.is_pn_already_received(pc, Some(l_cid), pn_j),
                    "pn {pn_j} should not yet be received at step {i}"
                );
            }
        }

        // Final state: [0..21] all received, contiguous.
        // C: picoquic_sack_list_last == 21 (max) → Rust first()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.first(), 21);
        // C: picoquic_sack_list_first == 0 (min) → Rust last()
        assert_eq!(cnx.ack_ctx[pc as usize].sack_list.last(), 0);
        assert_eq!(
            cnx.ack_ctx[pc as usize].time_stamp_largest_received,
            highest_seen_time
        );
        assert!(cnx.ack_ctx[pc as usize].sack_list.first_range().is_none());
    }
}
```

## `picoquictest/skip_frame_test.c:frames_format_test`
* C test-table name: `frames_format`
* C entry function: `frames_format_test`
* Rust test: `frames_format`
* C source: `picoquictest/skip_frame_test.c:1585-1664`
* Rust source: `rs/fq/src/tests/skip_frame.rs:966-968`

### C test body
```c
{
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t data[] = { 0xaa, 0xaa };
    uint8_t* bytes = NULL;
    uint8_t* bytes_max;
    int more_data;
    uint64_t current_time = 0;
    int is_pure_ack = 0;
    picoquic_stream_head_t* stream = NULL;
    int round;
    uint64_t simulated_time = 0;
    picoquic_quic_t* qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    struct sockaddr_in saddr = { 0 };
    uint8_t addr_bytes[4] = { 1, 2, 3, 4 };
    picoquic_cnx_t* cnx;
    picoquic_local_cnxid_list_t* local_cnxid_list = NULL;
    picoquic_local_cnxid_t* l_cid = NULL; 

    if (qclient == NULL) {
        ret = -1;
    }
    else {
        cnx = frames_format_test_get_cnx(qclient, (struct sockaddr *)&saddr, picoquic_epoch_1rtt, simulated_time, 1);
        if (cnx == NULL) {
            ret = -1;
        }
    }

    if (ret == 0)  {
        local_cnxid_list = cnx->first_local_cnxid_list;
        l_cid = picoquic_create_local_cnxid(cnx, local_cnxid_list->unique_path_id, NULL, current_time);
        picoquic_add_to_stream(cnx, 0, data, 2, 0);
        stream = picoquic_find_stream(cnx, 0);
        if (stream == NULL) {
            ret = -1;
        }
    }
    if (ret == 0) {
        stream->reset_requested = 1;
        FRAME_FORMAT_TEST_ONCE(picoquic_format_reset_stream_frame, 2, stream, bytes, bytes_max, &more_data, &is_pure_ack);
        stream->reset_requested = 0;
        FRAME_FORMAT_TEST(picoquic_format_new_connection_id_frame, cnx, local_cnxid_list, bytes, bytes_max, &more_data, &is_pure_ack, l_cid);
        FRAME_FORMAT_TEST(picoquic_format_retire_connection_id_frame, bytes, bytes_max, &more_data, &is_pure_ack, 1, 0, 17);
        FRAME_FORMAT_TEST(picoquic_format_new_token_frame, bytes, bytes_max, &more_data, &is_pure_ack, data, 2);
        stream->stop_sending_requested = 1;
        FRAME_FORMAT_TEST_ONCE(picoquic_format_stop_sending_frame, 2, stream, bytes, bytes_max, &more_data, &is_pure_ack);
        stream->stop_sending_requested = 0;
        stream->stop_sending_sent = 0;
        FRAME_FORMAT_TEST_ONCE(picoquic_format_data_blocked_frame, 1, cnx, bytes, bytes_max, &more_data, &is_pure_ack);
        FRAME_FORMAT_TEST(picoquic_format_stream_data_blocked_frame, bytes, bytes_max, &more_data, &is_pure_ack, stream);
        stream->stream_data_blocked_sent = 0;
        FRAME_FORMAT_TEST_ONCE(picoquic_format_stream_blocked_frame, 1, cnx, bytes, bytes_max, &more_data, &is_pure_ack, stream);
        cnx->stream_blocked_bidir_sent = 0;
        FRAME_FORMAT_TEST(picoquic_format_connection_close_frame, cnx, bytes, bytes_max, &more_data, &is_pure_ack);
        FRAME_FORMAT_TEST(picoquic_format_application_close_frame, cnx, bytes, bytes_max, &more_data, &is_pure_ack);
        FRAME_FORMAT_TEST(picoquic_format_max_stream_data_frame, cnx, stream, bytes, bytes_max, &more_data, &is_pure_ack, 100000000);
        FRAME_FORMAT_TEST(picoquic_format_path_challenge_frame, bytes, bytes_max, &more_data, &is_pure_ack, 0xaabbccddeeff0011ull);
        FRAME_FORMAT_TEST(picoquic_format_path_response_frame, bytes, bytes_max, &more_data, &is_pure_ack, 0xaabbccddeeff0011ull);
        FRAME_FORMAT_TEST(picoquic_format_datagram_frame, bytes, bytes_max, &more_data, &is_pure_ack, 2, data);
        FRAME_FORMAT_TEST_ONCE(picoquic_format_ack_frequency_frame, 2, cnx, bytes, bytes_max, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_immediate_ack_frame, bytes, bytes_max, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_time_stamp_frame, cnx, buffer, bytes_max, &more_data, simulated_time);
        FRAME_FORMAT_TEST(picoquic_format_path_abandon_frame, bytes, bytes_max, &more_data, 1, 3);
        FRAME_FORMAT_TEST(picoquic_format_path_available_or_backup_frame, bytes, bytes_max, picoquic_frame_type_path_available, 1, 17, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_max_path_id_frame, bytes, bytes_max, 123, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_paths_blocked_frame, bytes, bytes_max, 123, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_path_cid_blocked_frame, bytes, bytes_max, 123, 0, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_observed_address_frame, bytes, bytes_max, picoquic_frame_type_observed_address_v4, 13, addr_bytes, 4433, &more_data);
    }

    if (qclient != NULL) {
        picoquic_free(qclient);
    }

    return ret;
}
```

### Rust test body
```rust
fn frames_format() {
    run_frames_format_test().expect("frames_format");
}
```

## `picoquictest/sockloop_test.c:sockloop_basic_test`
* C test-table name: `sockloop_basic`
* C entry function: `sockloop_basic_test`
* Rust test: `sockloop_basic`
* C source: `picoquictest/sockloop_test.c:633-641`
* Rust source: `rs/fq/src/tests/sockloop.rs:557-562`

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 1);
    spec.ipv6_only = 1;
    spec.do_not_use_gso = 1;

    return(sockloop_test_one(&spec));
}
```

### Rust test body
```rust
fn sockloop_basic() {
    let mut spec = SockloopTestSpec::new(1);
    spec.ipv6_only = true;
    spec.do_not_use_gso = true;
    sockloop_test_one(&spec);
}
```

## `picoquictest/stream0_frame_test.c:provide_stream_buffer_test`
* C test-table name: `provide_stream_buffer`
* C entry function: `provide_stream_buffer_test`
* Rust test: `provide_stream_buffer`
* C source: `picoquictest/stream0_frame_test.c:1084-1105`
* Rust source: `rs/fq/src/tests/stream0_frame.rs:702-715`

### C test body
```c
{
    uint64_t stream_ids[4] = { 0, 7, 127, 0x10000 };
    uint64_t offsets[4] = { 0, 1, 65, 0x10000 };
    int ret = 0;

    for (int i_stream = 0; ret == 0 && i_stream < 4; i_stream++) {
        for (int i_offset = 0; ret == 0 &&  i_offset < 4; i_offset++) {
            for (int is_fin = 0; ret == 0 &&  is_fin < 2; is_fin++) {
                for (size_test_enum size_test = 0; ret == 0 &&  size_test < size_test_last; size_test++) {
                    ret = provide_stream_buffer_test_one(stream_ids[i_stream], offsets[i_offset], size_test, is_fin);
                    if (ret != 0) {
                        DBG_PRINTF("Fails for stream %" PRIu64 ", offset %" PRIu64 ", fin %d, test %d",
                            stream_ids[i_stream], offsets[i_offset], is_fin, size_test);
                        break;
                    }
                }
            }
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn provide_stream_buffer() {
    let stream_ids: [u64; 4] = [0, 7, 127, 0x10000];
    let offsets: [u64; 4] = [0, 1, 65, 0x10000];
    for &sid in &stream_ids {
        for &off in &offsets {
            for is_fin in [false, true] {
                for size_test in 0..7usize {
                    provide_stream_buffer_test_one(sid, off, size_test, is_fin)
                        .expect("provide_stream_buffer_test_one");
                }
            }
        }
    }
}
```

## `picoquictest/stresstest.c:fuzz_initial_test`
* C test-table name: `fuzz_initial`
* C entry function: `fuzz_initial_test`
* Rust test: `fuzz_initial`
* C source: `picoquictest/stresstest.c:1488-1500`
* Rust source: `rs/fq/src/tests/stresstest.rs:266-269`

### C test body
```c
{
    initial_fuzzer_ctx_t fuzz_ctx;
    int ret = 0;

    memset(&fuzz_ctx, 0, sizeof(initial_fuzzer_ctx_t));
    fuzz_ctx.random_context = 0x01234567DEADBEEFull;
    fuzz_ctx.random_context ^= picoquic_stress_test_duration;

    ret = stress_or_fuzz_test(initial_fuzzer, &fuzz_ctx, 2*picoquic_stress_test_duration, 4*picoquic_stress_test_duration);

    return ret;
}
```

### Rust test body
```rust
fn fuzz_initial() {
    let duration: u64 = 60_000_000;
    stress_or_fuzz_test(2 * duration, 4 * duration).expect("fuzz_initial_test");
}
```
