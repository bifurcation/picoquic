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

## `picoquictest/qlog_test.c:qlog_auto_test`
* C test-table name: `qlog_auto`
* C entry function: `qlog_auto_test`
* Rust test: `qlog_auto`
* C source: `picoquictest/qlog_test.c:134-151`
* Rust source: `rs/fq/src/tests/qlog.rs:73-78`

### C test body
```c
{
	int ret = autoqlog_bad_file();

	if (ret == 0) {
		ret = autoqlog_no_binlog();
	}

	if (ret == 0) {
		ret = autoqlog_longdir();
	}

	if (ret == 0) {
		ret = autoqlog_unique();
	}

	return ret;
}
```

### Rust test body
```rust
fn qlog_auto() {
    autoqlog_bad_file().expect("autoqlog_bad_file");
    autoqlog_no_binlog().expect("autoqlog_no_binlog");
    autoqlog_longdir().expect("autoqlog_longdir");
    autoqlog_unique().expect("autoqlog_unique");
}
```

## `picoquictest/satellite_test.c:satellite_seeded_bbr1_test`
* C test-table name: `satellite_seeded_bbr1`
* C entry function: `satellite_seeded_bbr1_test`
* Rust test: `satellite_seeded_bbr1`
* C source: `picoquictest/satellite_test.c:224-230`
* Rust source: `rs/fq/src/tests/satellite.rs:228-243`

### C test body
```c
{
    /* Simulate remembering RTT and BW from previous connection */
    /* TODO test changed, app limited, verify. */
    /* return satellite_test_one(picoquic_bbr1_algorithm, 100000000, 5300000, 250, 3, 0, 0, 0, 1, 0, 0); */
    return satellite_test_one(picoquic_bbr1_algorithm, 100000000, 5500000, 250, 3, 0, 0, 0, 1, 0, 0);
}
```

### Rust test body
```rust
fn satellite_seeded_bbr1() {
    let bbr1 = get_congestion_algorithm("bbr1").expect("bbr1");
    satellite_test_one(
        bbr1,
        100_000_000,
        5_500_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}
```

## `picoquictest/skip_frame_test.c:test_copy_for_retransmit`
* C test-table name: `stream_retransmit_copy`
* C entry function: `test_copy_for_retransmit`
* Rust test: `stream_retransmit_copy`
* C source: `picoquictest/skip_frame_test.c:2795-2927`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1033-1038`

### C test body
```c
{
    picoquic_quic_t * qtest = NULL;
    picoquic_cnx_t * cnx = NULL;
    int ret = 0;
    picoquic_packet_t old_p;
    uint8_t new_bytes[PICOQUIC_MAX_PACKET_SIZE];
    size_t length = 0;
    int packet_is_pure_ack = 0;
    int do_not_detect_spurious = 1;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;

    memset(&saddr, 0, sizeof(struct sockaddr_in));

    /* Initialize the connection context */
    qtest = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    if (qtest == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }

    /* Perform the tests */
    for (size_t i = 0; ret == 0 && i < nb_copy_retransmit_case; i++) {
        int add_to_data_repeat_queue = 0;

        cnx = picoquic_create_cnx(qtest,
            picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr *) &saddr,
            simulated_time, 0, "test-sni", "test-alpn", 1);

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create QUIC CNX context\n");
            ret = -1;
            break;
        }
        /* Initialize stream 0 */
        if ((ret = picoquic_add_to_stream(cnx, 0, ct_stream0_data, sizeof(ct_stream0_data), 0)) != 0) {
            DBG_PRINTF("%s", "Cannot initialize stream 0\n");
            ret = -1;
            break;
        }

        /* Initialize the old packet */
        memset(&old_p, 0, sizeof(picoquic_packet_t));
        if (copy_retransmit_case[i].packet_length > 0) {
            memcpy(old_p.bytes, copy_retransmit_case[i].packet, copy_retransmit_case[i].packet_length);
            old_p.length = copy_retransmit_case[i].packet_length;
        }
        old_p.offset = copy_retransmit_case[i].offset;
        old_p.is_mtu_probe = copy_retransmit_case[i].is_mtu_probe;
        old_p.is_ack_trap = copy_retransmit_case[i].is_ack_trap;
        old_p.send_path = cnx->path[0];

        length = copy_retransmit_case[i].b1_offset;

        ret = picoquic_copy_before_retransmit(&old_p, cnx, new_bytes,
            copy_retransmit_case[i].copy_max,
            &packet_is_pure_ack,
            &do_not_detect_spurious, 0,
            &length,
            &add_to_data_repeat_queue);

        if (ret != 0) {
            DBG_PRINTF("Cannot perform copy for test[%d]\n", i);
        } else if (packet_is_pure_ack != copy_retransmit_case[i].is_pure_ack_expected) {
            /* Check whether pure ack matches expectation */
            DBG_PRINTF("Is pure ack mismatch on test[%d], got %d\n", i,
                packet_is_pure_ack);
            ret = -1;
        }
        else if (!packet_is_pure_ack) {
            /* Compare bytes and length to expected */
            if (length != copy_retransmit_case[i].b1_length) {
                DBG_PRINTF("Length mismatch on test[%d], got %d vs %d\n", i,
                    packet_is_pure_ack, length,
                    copy_retransmit_case[i].b1_length);
                ret = -1;
            }
            else if (memcmp(new_bytes + copy_retransmit_case[i].b1_offset,
                copy_retransmit_case[i].b1_expected + copy_retransmit_case[i].b1_offset,
                length - copy_retransmit_case[i].b1_offset) != 0) {
                DBG_PRINTF("Value mismatch on test[%d]\n", i);
                ret = -1;
            }
            else {
                if (copy_retransmit_case[i].b2_expected == NULL) {
                    if (add_to_data_repeat_queue) {
                        DBG_PRINTF("Unexpected stream frame in test[%d]\n", i);
                        ret = -1;
                    }
                }
                else if (!add_to_data_repeat_queue) {
                    DBG_PRINTF("Missing stream frame in test[%d]\n", i);
                    ret = -1;
                }

                if (ret == 0) {
                    if (copy_retransmit_case[i].b3_expected == NULL) {
                        if (cnx->first_misc_frame != NULL) {
                            DBG_PRINTF("Unexpected misc frame in test[%d]\n", i);
                            ret = -1;
                        }
                    }
                    else if (cnx->first_misc_frame == NULL) {
                        DBG_PRINTF("Missing misc frame in test[%d]\n", i);
                        ret = -1;
                    }
                    else if (copy_retransmit_case[i].b3_length != cnx->first_misc_frame->length) {
                        DBG_PRINTF("Mismatching misc frame lenght in test[%d]\n", i);
                        ret = -1;
                    }
                    else if (memcmp(((uint8_t*)cnx->first_misc_frame) + sizeof(picoquic_misc_frame_header_t),
                        copy_retransmit_case[i].b3_expected, cnx->first_misc_frame->length) != 0) {
                        DBG_PRINTF("Mismatching misc frame in test[%d]\n", i);
                        ret = -1;
                    }
                }
            }
        }
        /* Free the extra frames */
        if (cnx != NULL) {
            picoquic_delete_cnx(cnx);
        }
    }

    /* Free the connection context */
    if (qtest != NULL) {
        picoquic_free(qtest);
    }
    return ret;
}
```

### Rust test body
```rust
fn stream_retransmit_copy() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    run_stream_retransmit_copy_test(&mut quic, &mut simulated_time)
        .expect("copy_before_retransmit");
}
```

## `picoquictest/spinbit_test.c:spinbit_test`
* C test-table name: `spinbit`
* C entry function: `spinbit_test`
* Rust test: `spinbit`
* C source: `picoquictest/spinbit_test.c:190-193`
* Rust source: `rs/fq/src/tests/spinbit.rs:138-140`

### C test body
```c
{
    return spinbit_test_one(picoquic_spinbit_basic, picoquic_spinbit_on);
}
```

### Rust test body
```rust
fn spinbit() {
    spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::On).expect("spinbit");
}
```

## `picoquictest/stream0_frame_test.c:stream_splay_test`
* C test-table name: `stream_splay`
* C entry function: `stream_splay_test`
* Rust test: `stream_splay`
* C source: `picoquictest/stream0_frame_test.c:515-651`
* Rust source: `rs/fq/src/tests/stream0_frame.rs:792-794`

### C test body
```c
{
    int ret = 0;
    int count = 0;
    picoquic_quic_t *quic = NULL;
    picoquic_cnx_t *cnx = NULL;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;
    uint64_t values[] = { 3, 4, 1, 2, 8, 5, 7 };
    uint64_t ordered[] = { 1, 2, 3, 4, 5, 7, 8 };
    uint64_t values_first[] = { 3, 3, 1, 1, 1, 1, 1 };
    uint64_t values_last[] = { 3, 4, 4, 4, 8, 8, 8 };
    uint64_t value2_first[] = { 1, 1, 2, 5, 5, 7, 0 };
    uint64_t value2_last[] = { 8, 8, 8, 8, 7, 7, 0 };


    quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    memset(&saddr, 0, sizeof(struct sockaddr_in));
    saddr.sin_family = AF_INET;
    saddr.sin_port = 1000;

    if (quic == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }
    else {
        cnx = picoquic_create_cnx(quic,
            picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr *) &saddr,
            simulated_time, 0, "test-sni", "test-alpn", 1);

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create connection\n");
            ret = -1;
        } else {
            picoquic_stream_head_t * stream;
            int rank = 0;

            /* test creation of streams */
            for (int i = 0; ret == 0 && i < 7; i++) {
                picoquic_create_stream(cnx, values[i]);
                /* Verify sanity and count after each insertion */
                count = check_stream_splay_node_sanity(cnx->stream_tree.root, NULL, NULL, cnx->stream_tree.comp);
                if (count != i + 1) {
                    DBG_PRINTF("Insert v[%d] = %d, expected %d nodes, got %d instead\n",
                        i, values[i], i + 1, count);
                    ret = -1;
                }
                else if (cnx->stream_tree.size != count) {
                    DBG_PRINTF("Insert v[%d] = %d, expected tree size %d, got %d instead\n",
                        i, values[i], count, cnx->stream_tree.size);
                    ret = -1;
                }
                else if (picoquic_first_stream(cnx)->stream_id != values_first[i]) {
                    DBG_PRINTF("Insert v[%d] = %d, expected first = %d, got %d instead\n",
                        i, values[i],
                        values_first[i], (int)picoquic_first_stream(cnx)->stream_id);
                    ret = -1;
                }
                else if (picoquic_last_stream(cnx)->stream_id != values_last[i]) {
                    DBG_PRINTF("Insert v[%d] = %d, expected first = %d, got %d instead\n",
                        i, values[i],
                        values_last[i], (int)picoquic_last_stream(cnx)->stream_id);
                    ret = -1;
                }
            }

            /* test order */
            stream = picoquic_first_stream(cnx);
            while (ret == 0 && rank < 7) {
                if (stream == NULL) {
                    DBG_PRINTF("Stream[%d] is NULL\n", rank);
                    ret = -1;
                }
                else if (stream->stream_id != ordered[rank]) {
                    DBG_PRINTF("Stream[%d].stream_id = %d, expected %d\n", rank, (int)stream->stream_id, (int)ordered[rank]);
                    ret = -1;
                }
                else {
                    stream = picoquic_next_stream(stream);
                    rank++;
                }
            }


            /* Test deletion of streams */
            for (int i = 0; ret == 0 && i < 7; i++) {
                stream = picoquic_find_stream(cnx, values[i]);
                if (stream == NULL) {
                    DBG_PRINTF("Cannot find stream %d\n", (int)values[i]);
                    ret = -1;
                    break;
                }
                picoquic_delete_stream(cnx, stream);
                /* Verify sanity and count after each deletion */
                count = check_stream_splay_node_sanity(cnx->stream_tree.root, NULL, NULL, cnx->stream_tree.comp);
                if (count != 6 - i) {
                    DBG_PRINTF("Delete v[%d] = %d, expected %d nodes, got %d instead\n",
                        i, values[i], 6 - i, count);
                    ret = -1;
                }
                else if (cnx->stream_tree.size != count) {
                    DBG_PRINTF("Insert v[%d] = %d, expected cnx->stream_tree size %d, got %d instead\n",
                        i, values[i], count, cnx->stream_tree.size);
                    ret = -1;
                }
                else if (i < 6) {
                    if (picoquic_first_stream(cnx)->stream_id != value2_first[i]) {
                        DBG_PRINTF("Delete v[%d] = %d, expected first = %d, got %d instead\n",
                            i, values[i], value2_first[i], (int)picoquic_first_stream(cnx)->stream_id);
                        ret = -1;
                    }
                    else if (picoquic_last_stream(cnx)->stream_id != value2_last[i]) {
                        DBG_PRINTF("Delete v[%d] = %d, expected first = %d, got %d instead\n",
                            i, values[i], value2_last[i], (int)picoquic_last_stream(cnx)->stream_id);
                        ret = -1;
                    }
                }
            }

            if (ret == 0 && cnx->stream_tree.root != NULL) {
                DBG_PRINTF("%s", "Final cnx->stream_tree root should be NULL, is not.\n");
                ret = -1;
            }

            picoquic_delete_cnx(cnx);
            cnx = NULL;
        }

        picoquic_free(quic);
        quic = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn stream_splay() {
    stream_splay_test_body().expect("stream_splay_test");
}
```
