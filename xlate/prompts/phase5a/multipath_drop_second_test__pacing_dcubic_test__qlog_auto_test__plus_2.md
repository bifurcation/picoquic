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

## `picoquictest/multipath_test.c:multipath_drop_second_test`
* C test-table name: `multipath_drop_second`
* C entry function: `multipath_drop_second_test`
* Rust test: `multipath_drop_second`
* C source: `picoquictest/multipath_test.c:1337-1342`
* Rust source: `rs/fq/src/tests/multipath.rs:1546-1548`

### C test body
```c
{
    uint64_t max_completion_microsec = 1260000;

    return multipath_test_one(max_completion_microsec, multipath_test_drop_second);
}
```

### Rust test body
```rust
fn multipath_drop_second() {
    multipath_test_one(1_260_000, MultipathTestId::DropSecond);
}
```

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

## `picoquictest/satellite_test.c:satellite_dcubic_seeded_test`
* C test-table name: `satellite_dcubic_seeded`
* C entry function: `satellite_dcubic_seeded_test`
* Rust test: `satellite_dcubic_seeded`
* C source: `picoquictest/satellite_test.c:301-305`
* Rust source: `rs/fq/src/tests/satellite.rs:456-471`

### C test body
```c
{
    /* TODO check max_completion_time */
    return satellite_test_one(picoquic_dcubic_algorithm, 100000000, 5300000, 250, 3, 0, 0, 0, 1, 0, 0);
}
```

### Rust test body
```rust
fn satellite_dcubic_seeded() {
    let dcubic = get_congestion_algorithm("dcubic").expect("dcubic");
    satellite_test_one(
        dcubic,
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
