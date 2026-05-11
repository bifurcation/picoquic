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

## `picoquictest/skip_frame_test.c:frames_repeat_test`
* C test-table name: `frames_repeat`
* C entry function: `frames_repeat_test`
* Rust test: `frames_repeat`
* C source: `picoquictest/skip_frame_test.c:1320-1376`
* Rust source: `rs/fq/src/tests/skip_frame.rs:983-989`

### C test body
```c
{
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint64_t simulated_time = 0;
    picoquic_quic_t* qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    struct sockaddr_in saddr = { 0 };

    if (qclient == NULL) {
        ret = -1;
    }
    else {
        for (size_t i = 0; ret == 0 && i < nb_test_skip_list; i++) {
            size_t len = test_skip_list[i].len;
            uint64_t frame_type = 0;
            const uint8_t* type_byte = NULL;
            if ((type_byte = picoquic_frames_varint_decode(test_skip_list[i].val, test_skip_list[i].val + test_skip_list[i].len, &frame_type)) != NULL) {
                memcpy(buffer, test_skip_list[i].val, len);
                if (frame_repeat_error_packet(qclient, (struct sockaddr*)&saddr, simulated_time, buffer, len,
                    test_skip_list[i].epoch, test_skip_list[i].mpath, 0) != 0) {
                    ret = -1;
                }
                else if (len > 1 && !test_skip_list[i].is_pure_ack) {
                    switch (frame_type) {
                    case picoquic_frame_type_connection_close:
                    case picoquic_frame_type_application_close:
                    case picoquic_frame_type_new_token:
                    case picoquic_frame_type_path_abandon:
                    case picoquic_frame_type_bdp:
                    case picoquic_frame_type_observed_address_v4:
                    case picoquic_frame_type_observed_address_v6:
                        break;
                    default:
                        if (frame_repeat_error_packet(qclient, (struct sockaddr*)&saddr, simulated_time, buffer, len - 1,
                            test_skip_list[i].epoch, test_skip_list[i].mpath, 1) != 0) {
                            if (test_skip_list[i].nb_varints > 0) {
                                /* Try again with shorter length */
                                size_t type_len = type_byte - test_skip_list[i].val;
                                if (frame_repeat_error_packet(qclient, (struct sockaddr*)&saddr, simulated_time, buffer, type_len,
                                    test_skip_list[i].epoch, test_skip_list[i].mpath, 1) != 0) {
                                    ret = -1;
                                }
                            }
                            else {
                                ret = -1;
                            }
                        }
                    }
                }
            }
        }
        picoquic_free(qclient);
    }
    return ret;
}
```

### Rust test body
```rust
fn frames_repeat() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    let sample = &[0x01u8]; // PING
    frame_repeat_error_packet(&mut quic, sample, 3, false, false).expect("repeat PING");
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

## `picoquictest/stream0_frame_test.c:stream_state_local_reuse_test`
* C test-table name: `stream_state_local_reuse`
* C entry function: `stream_state_local_reuse_test`
* Rust test: `stream_state_local_reuse`
* C source: `picoquictest/stream0_frame_test.c:422-472`
* Rust source: `rs/fq/src/tests/stream0_frame.rs:798-800`

### C test body
```c
{
    picoquic_quic_t* quic = NULL;
    picoquic_cnx_t* cnx = NULL;
    uint64_t simulated_time = 0;
    int ret = 0;

    if (picoquic_test_set_minimal_cnx_with_time(&quic, &cnx, &simulated_time) != 0 || quic == NULL || cnx == NULL) {
        ret = -1;
    } else {
        /* Ensure client mode for local bidirectional stream IDs */
        cnx->client_mode = 1;

        uint64_t sid1 = picoquic_get_next_local_stream_id(cnx, 0);
        uint64_t sid2 = picoquic_get_next_local_stream_id(cnx, 0);

        if (sid1 != sid2) {
            ret = -1;
        } else {
            picoquic_stream_head_t* s = picoquic_create_stream(cnx, sid1);
            if (s == NULL) {
                ret = -1;
            } else {
                /* Remove the stream from tree so next_stream_id advanced, but stream object is gone */
                picoquic_delete_stream(cnx, s);

                if (!(cnx->next_stream_id[STREAM_TYPE_FROM_ID(sid1)] > sid1)) {
                    ret = -1;
                } else {
                    int aret = picoquic_set_app_stream_ctx(cnx, sid2, NULL);
                    if (aret != PICOQUIC_ERROR_STREAM_ALREADY_CLOSED) {
                        ret = -1;
                    } else if (cnx->cnx_state == picoquic_state_disconnecting ||
                        cnx->cnx_state == picoquic_state_disconnected ||
                        cnx->cnx_state == picoquic_state_handshake_failure ||
                        cnx->cnx_state == picoquic_state_handshake_failure_resend) {
                        ret = -1;
                    } else if (cnx->local_error == PICOQUIC_TRANSPORT_STREAM_STATE_ERROR) {
                        ret = -1;
                    }
                }
            }
        }
    }

    picoquic_test_delete_minimal_cnx(&quic, &cnx);
    return ret;
}
```

### Rust test body
```rust
fn stream_state_local_reuse() {
    stream_state_local_reuse_body().expect("stream_state_local_reuse_test");
}
```

## `picoquictest/ticket_store_test.c:ticket_seed_from_bdp_frame_test`
* C test-table name: `ticket_seed_from_bdp_frame`
* C entry function: `ticket_seed_from_bdp_frame_test`
* Rust test: `ticket_seed_from_bdp_frame`
* C source: `picoquictest/ticket_store_test.c:740-743`
* Rust source: `rs/fq/src/tests/ticket_store.rs:25-27`

### C test body
```c
int ticket_seed_from_bdp_frame_test(void) {
    
   return ticket_seed_test_one(2);
}
```

### Rust test body
```rust
fn ticket_seed_from_bdp_frame() {
    ticket_seed_test_one(2).expect("ticket_seed_from_bdp_frame");
}
```
