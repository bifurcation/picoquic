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

## `picoquictest/satellite_test.c:satellite_preemptive_test`
* C test-table name: `satellite_preemptive`
* C entry function: `satellite_preemptive_test`
* Rust test: `satellite_preemptive`
* C source: `picoquictest/satellite_test.c:246-251`
* Rust source: `rs/fq/src/tests/satellite.rs:285-300`

### C test body
```c
{
    /* Variation of the loss test, using preemptive repeat*/
    /* Should be less than 10 sec per draft etosat.  */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 7100000, 250, 3, 0, 1, 1, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_preemptive() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        7_100_000,
        250,
        3,
        0,
        true,
        true,
        false,
        false,
        false,
    );
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

## `picoquictest/sockloop_test.c:sockloop_migration_test`
* C test-table name: `sockloop_migration`
* C entry function: `sockloop_migration_test`
* Rust test: `sockloop_migration`
* C source: `picoquictest/sockloop_test.c:681-693`
* Rust source: `rs/fq/src/tests/sockloop.rs:594-601`

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 5);
    spec.af = AF_INET6;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.extra_socket_required = 1;
    spec.force_migration = 3;

    return(sockloop_test_one(&spec));
}
```

### Rust test body
```rust
fn sockloop_migration() {
    let mut spec = SockloopTestSpec::new(5);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.extra_socket_required = true;
    spec.force_migration = 3;
    sockloop_test_one(&spec);
}
```

## `picoquictest/spinbit_test.c:spinbit_random_test`
* C test-table name: `spinbit_random`
* C entry function: `spinbit_random_test`
* Rust test: `spinbit_random`
* C source: `picoquictest/spinbit_test.c:195-198`
* Rust source: `rs/fq/src/tests/spinbit.rs:144-146`

### C test body
```c
{
    return spinbit_test_one(picoquic_spinbit_basic, picoquic_spinbit_random);
}
```

### Rust test body
```rust
fn spinbit_random() {
    spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::Random).expect("spinbit_random");
}
```
