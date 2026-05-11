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

## `picoquictest/pacing_test.c:pacing_dcubic_test`
* C test-table name: `pacing_dcubic`
* C entry function: `pacing_dcubic_test`
* Rust test: `pacing_dcubic`
* C source: `picoquictest/pacing_test.c:232-236`
* Rust source: `rs/fq/src/tests/pacing.rs:622-624`

### C test body
```c
{
    int ret = pacing_cc_algotest(picoquic_dcubic_algorithm, 900000, 240);
    return ret;
}
```

### Rust test body
```rust
fn pacing_dcubic() {
    pacing_cc_algotest("dcubic", 900_000, 240);
}
```

## `picoquictest/sacktest.c:ackrange_test`
* C test-table name: `ack_range`
* C entry function: `ackrange_test`
* Rust test: `ack_range`
* C source: `picoquictest/sacktest.c:572-621`
* Rust source: `rs/fq/src/tests/sacktest.rs:715-744`

### C test body
```c
{
    int ret = 0;
    picoquic_sack_list_t sack0;

    picoquic_sack_list_init(&sack0);

    for (size_t i = 0; ret == 0 && i < nb_ack_range; i++) {
        ret = picoquic_check_sack_list(&sack0,
            ack_range[i].range_min, ack_range[i].range_max);

        if (ret == 0) {
            ret = picoquic_update_sack_list(&sack0,
                ack_range[i].range_min, ack_range[i].range_max, 0);
        }

        if (ret == 0) {
            ret = check_ack_ranges(&sack0);
        }

        for (size_t j = 0; j < i; j++) {
            if (picoquic_check_sack_list(&sack0,
                    ack_range[j].range_min, ack_range[j].range_max)
                == 0) {
                ret = -1;
                break;
            }
        }

        if (ret != 0) {
            break;
        }
    }

    if (ret == 0 && picoquic_sack_list_first(&sack0) != 0) {
        ret = -1;
    }

    if (ret == 0 && picoquic_sack_list_last(&sack0)!= 7500) {
        ret = -1;
    }

    if (ret == 0 && picoquic_sack_list_first_range(&sack0) != NULL) {
        ret = -1;
    }

    picoquic_sack_list_free(&sack0);

    return ret;
}
```

### Rust test body
```rust
fn ack_range() {
    let t0 = Instant::from_ticks(0);
    let mut sack0 = SackList::new();

    for (i, &(rmin, rmax)) in ACK_RANGES.iter().enumerate() {
        // C: picoquic_check_sack_list == 0 means not yet in sack.
        assert!(
            !sack0.check(rmin, rmax),
            "range ({rmin},{rmax}) already in sack at step {i}"
        );
        sack0.update(rmin, rmax, t0).expect("update sack");
        util::check_ack_ranges(&mut sack0);

        for &(jmin, jmax) in ACK_RANGES.iter().take(i) {
            assert!(
                sack0.check(jmin, jmax),
                "range ({jmin},{jmax}) not in sack at step {i}"
            );
        }
    }

    // Final state: [0..7500] fully covered (no gaps).
    // C: picoquic_sack_list_first == 0 (min) → Rust last()
    assert_eq!(sack0.last(), 0);
    // C: picoquic_sack_list_last == 7500 (max) → Rust first()
    assert_eq!(sack0.first(), 7500);
    assert!(sack0.first_range().is_none());

    sack0.free();
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

## `picoquictest/skip_frame_test.c:binlog_test`
* C test-table name: `binlog`
* C entry function: `binlog_test`
* Rust test: `binlog`
* C source: `picoquictest/skip_frame_test.c:2146-2310`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1130-1132`

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

### Rust test body
```rust
fn binlog() {
    run_binlog_test().expect("binlog_test");
}
```

## `picoquictest/skip_frame_test.c:new_cnxid_test`
* C test-table name: `new_cnxid`
* C entry function: `new_cnxid_test`
* Rust test: `new_cnxid`
* C source: `picoquictest/skip_frame_test.c:2442-2529`
* Rust source: `rs/fq/src/tests/skip_frame.rs:993-997`

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;
    picoquic_quic_t * qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    picoquic_cnx_t * cnx = NULL;
    uint8_t frame_buffer[256];
    size_t consumed = 0;

    memset(&saddr, 0, sizeof(struct sockaddr_in));
    saddr.sin_family = AF_INET;
    saddr.sin_port = 1000;

    if (qclient == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    } else {
        cnx = picoquic_create_cnx(qclient,
            picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr *) &saddr,
            simulated_time, 0, "test-sni", "test-alpn", 1);

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create QUIC CNX context\n");
            ret = -1;
        }
        else {
            /* Create a new local CID */
            picoquic_local_cnxid_t* local_cid = picoquic_create_local_cnxid(cnx, 0, NULL, simulated_time);
            picoquic_local_cnxid_list_t* local_cid_list = cnx->first_local_cnxid_list;
            
            if (local_cid == NULL || local_cid_list == NULL) {
                DBG_PRINTF("%s", "Cannot create local cnxid\n");
                ret = -1;
            }

            if (local_cid_list->nb_local_cnxid != 2) {
                DBG_PRINTF("Expected 2 CID, got %d\n", local_cid_list->nb_local_cnxid);
                ret = -1;
            }
            else if (local_cid_list->local_cnxid_first == NULL || local_cid_list->local_cnxid_first->next == NULL) {
                DBG_PRINTF("%s", "Pointer to CID is NULL in cnx context\n");
                ret = -1;
            }

            if (ret == 0) {
                int more_data = 0;
                int is_pure_ack = 1;
                uint8_t* bytes_next = picoquic_format_new_connection_id_frame(cnx, local_cid_list, frame_buffer, frame_buffer + sizeof(frame_buffer),
                    &more_data, &is_pure_ack, local_cid);

                consumed = bytes_next - frame_buffer;

                if (consumed == 0) {
                    ret = -1;
                    DBG_PRINTF("Cannot encode new connection ID frame, ret = %x\n", ret);
                }
            }

            if (ret == 0) {
                size_t skipped = 0;
                int pure_ack = 0;

                ret = picoquic_skip_frame(frame_buffer, sizeof(frame_buffer), &skipped, &pure_ack);

                if (ret != 0) {
                    DBG_PRINTF("Cannot skip connection ID frame, ret = %x\n", ret);
                }
                else if (skipped != consumed) {
                    DBG_PRINTF("Skipped %d bytes instead of %d\n", (int)skipped, (int)consumed);
                    ret = -1;
                }
                else if (pure_ack != 0) {
                    DBG_PRINTF("Pure ACK = %d instead of 0\n", (int)pure_ack);
                    ret = -1;
                }
            }
            /* Delete the connecton and free the stash */
            picoquic_delete_cnx(cnx);
        }

        picoquic_free(qclient);
    }

    return ret;
}
```

### Rust test body
```rust
fn new_cnxid() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    run_new_cnxid_test(&mut quic, &mut simulated_time).expect("new_cnxid");
}
```
