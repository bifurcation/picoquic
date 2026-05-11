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

## `picoquictest/multipath_test.c:multipath_rotation_test`
* C test-table name: `multipath_rotation`
* C entry function: `multipath_rotation_test`
* Rust test: `multipath_rotation`
* C source: `picoquictest/multipath_test.c:1363-1370`
* Rust source: `rs/fq/src/tests/multipath.rs:1615-1617`

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_rotation);
}
```

### Rust test body
```rust
fn multipath_rotation() {
    multipath_test_one(3_000_000, MultipathTestId::Rotation);
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

## `picoquictest/satellite_test.c:satellite_small_up_test`
* C test-table name: `satellite_small_up`
* C entry function: `satellite_small_up_test`
* Rust test: `satellite_small_up`
* C source: `picoquictest/satellite_test.c:271-275`
* Rust source: `rs/fq/src/tests/satellite.rs:361-376`

### C test body
```c
{
    /* Should be less than 420 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 400000000, 2, 10, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_small_up() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        400_000_000,
        2,
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
