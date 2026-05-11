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

## `picoquictest/multipath_test.c:multipath_quality_test`
* C test-table name: `multipath_quality`
* C entry function: `multipath_quality_test`
* Rust test: `multipath_quality`
* C source: `picoquictest/multipath_test.c:1467-1472`
* Rust source: `rs/fq/src/tests/multipath.rs:1603-1605`

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Rust test body
```rust
fn multipath_quality() {
    multipath_test_one(1_000_000, MultipathTestId::Quality);
}
```

## `picoquictest/pacing_test.c:pacing_cubic_test`
* C test-table name: `pacing_cubic`
* C entry function: `pacing_cubic_test`
* Rust test: `pacing_cubic`
* C source: `picoquictest/pacing_test.c:226-230`
* Rust source: `rs/fq/src/tests/pacing.rs:616-618`

### C test body
```c
{
    int ret = pacing_cc_algotest(picoquic_cubic_algorithm, 900000, 210);
    return ret;
}
```

### Rust test body
```rust
fn pacing_cubic() {
    pacing_cc_algotest("cubic", 900_000, 210);
}
```

## `picoquictest/picoquic_lb_test.c:cid_for_lb_test`
* C test-table name: `cid_for_lb`
* C entry function: `cid_for_lb_test`
* Rust test: `cid_for_lb`
* C source: `picoquictest/picoquic_lb_test.c:966-987`
* Rust source: `rs/fq/src/tests/picoquic_lb.rs:775-824`

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    picoquic_quic_t* quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    if (quic == NULL) {
        DBG_PRINTF("%s", "Could not create the quic context.");
    }
    else {
        for (int i = 0; i < NB_LB_CONFIG_TEST && ret == 0; i++) {
            ret = cid_for_lb_test_one(quic, i, &cid_for_lb_test_config[i], &cid_for_lb_test_init[i], &cid_for_lb_test_ref[i]);
        }

        if (quic != NULL) {
            picoquic_free(quic);
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn cid_for_lb() {
    let simulated_time = Instant::from_ticks(0);
    let mut quic = crate::Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        None,
    )
    .expect("create quic context");

    let configs = cid_for_lb_test_config();
    let inits = cid_for_lb_test_init();
    let refs = cid_for_lb_test_ref();

    assert_eq!(configs.len(), NB_LB_CONFIG_TEST);
    assert_eq!(inits.len(), NB_LB_CONFIG_TEST);
    assert_eq!(refs.len(), NB_LB_CONFIG_TEST);

    for i in 0..NB_LB_CONFIG_TEST {
        quic.set_lb_cid_config(&configs[i])
            .unwrap_or_else(|_| panic!("test #{i}: set_lb_cid_config failed"));

        let result = quic
            .lb_generate_cid(&inits[i])
            .unwrap_or_else(|| panic!("test #{i}: lb_generate_cid returned None"));

        assert_eq!(
            result, refs[i],
            "test #{i}: generated CID does not match reference"
        );

        let server_id = quic
            .lb_verify_cid(&result)
            .unwrap_or_else(|| panic!("test #{i}: lb_verify_cid returned None"));

        assert_eq!(
            server_id, configs[i].server_id,
            "test #{i}: decoded server ID does not match config"
        );

        quic.clear_lb_cid_config();
    }
}
```

## `picoquictest/sacktest.c:ack_disorder_test`
* C test-table name: `ack_disorder`
* C entry function: `ack_disorder_test`
* Rust test: `ack_disorder`
* C source: `picoquictest/sacktest.c:784-788`
* Rust source: `rs/fq/src/tests/sacktest.rs:751-753`

### C test body
```c
{
    int ret = ack_disorder_test_one(ACK_DISORDER_LOG, 0, 133.0);
    return ret;
}
```

### Rust test body
```rust
fn ack_disorder() {
    ack_disorder_one("ack_disorder_test.csv", 0, 133.0);
}
```

## `picoquictest/satellite_test.c:satellite_cubic_test`
* C test-table name: `satellite_cubic`
* C entry function: `satellite_cubic_test`
* Rust test: `satellite_cubic`
* C source: `picoquictest/satellite_test.c:284-288`
* Rust source: `rs/fq/src/tests/satellite.rs:399-414`

### C test body
```c
{
    /* Should be less than 7 sec per draft etosat */
    return satellite_test_one(picoquic_cubic_algorithm, 100000000, 6500000, 250, 3, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_cubic() {
    let cubic = get_congestion_algorithm("cubic").expect("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        6_500_000,
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
