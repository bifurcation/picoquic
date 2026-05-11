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

## `picoquictest/multipath_test.c:monopath_rotation_test`
* C test-table name: `monopath_rotation`
* C entry function: `monopath_rotation_test`
* Rust test: `monopath_rotation`
* C source: `picoquictest/multipath_test.c:1690-1694`
* Rust source: `rs/fq/src/tests/multipath.rs:1394-1396`

### C test body
```c
{
    return monopath_test_one(monopath_test_rotation);
}
```

### Rust test body
```rust
fn monopath_rotation() {
    monopath_test_one(MonopathTestId::Rotation);
}
```

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

## `picoquictest/satellite_test.c:satellite_basic_test`
* C test-table name: `satellite_basic`
* C entry function: `satellite_basic_test`
* Rust test: `satellite_basic`
* C source: `picoquictest/satellite_test.c:210-216`
* Rust source: `rs/fq/src/tests/satellite.rs:190-205`

### C test body
```c
{
    /* Should be less than 7 sec per draft etosat. */
    /* TODO test changed, app limited, verify. */
    /* return satellite_test_one(picoquic_bbr_algorithm, 100000000, 5300000, 250, 3, 0, 0, 0, 0, 0, 0); */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 5500000, 250, 3, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_basic() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        5_500_000,
        250,
        3,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/satellite_test.c:satellite_prague_seeded_test`
* C test-table name: `satellite_prague_seeded`
* C entry function: `satellite_prague_seeded_test`
* Rust test: `satellite_prague_seeded`
* C source: `picoquictest/satellite_test.c:307-311`
* Rust source: `rs/fq/src/tests/satellite.rs:475-490`

### C test body
```c
{
    /* TODO check max_completion_time */
    return satellite_test_one(picoquic_prague_algorithm, 100000000, 5300000, 250, 3, 0, 0, 0, 1, 0, 0);
}
```

### Rust test body
```rust
fn satellite_prague_seeded() {
    let prague = get_congestion_algorithm("prague").expect("prague");
    satellite_test_one(
        prague,
        100_000_000,
        5_300_000,
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
