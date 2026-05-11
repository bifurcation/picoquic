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

## `picoquictest/l4s_test.c:l4s_c4_test`
* C test-table name: `l4s_c4`
* C entry function: `l4s_c4_test`
* Rust test: `l4s_c4`
* C source: `picoquictest/l4s_test.c:162-169`
* Rust source: `rs/fq/src/tests/l4s.rs:183-186`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgo = c4_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 3600000, 30, 3000, 0, NULL);

    return ret;
}
```

### Rust test body
```rust
fn l4s_c4() {
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    l4s_congestion_test(ccalgo, true, 3_600_000, 30, 3_000, &[]);
}
```

## `picoquictest/multipath_test.c:multipath_qlog_test`
* C test-table name: `multipath_qlog`
* C entry function: `multipath_qlog_test`
* Rust test: `multipath_qlog`
* C source: `picoquictest/multipath_test.c:1963-1988`
* Rust source: `rs/fq/src/tests/multipath.rs:1588-1599`

### C test body
```c
{
    int ret = 0;

    (void)picoquic_file_delete(MULTIPATH_TRACE_QLOG, NULL);

    ret = multipath_trace_test_one(1);

    /* compare the log file to the expected value */
    if (ret == 0)
    {
        char qlog_trace_test_ref[512];

        ret = picoquic_get_input_path(qlog_trace_test_ref, sizeof(qlog_trace_test_ref),
            picoquic_solution_dir, MULTIPATH_QLOG_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the qlog trace test ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(MULTIPATH_TRACE_QLOG, qlog_trace_test_ref);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn multipath_qlog() {
    const MULTIPATH_TRACE_QLOG: &str = "0807060504030201.server.qlog";
    const MULTIPATH_QLOG: &str = "multipath_qlog_test.qlog";
    const MULTIPATH_QLOG_REF: &str = "picoquictest/multipath_qlog_ref.txt";

    // Delete any existing qlog file.
    let _ = std::fs::remove_file(MULTIPATH_TRACE_QLOG);

    multipath_trace_test_one(true);

    compare_text_files(MULTIPATH_QLOG, MULTIPATH_QLOG_REF).expect("qlog matches reference");
}
```

## `picoquictest/netperf_test.c:netperf_basic_test`
* C test-table name: `netperf_basic`
* C entry function: `netperf_basic_test`
* Rust test: `netperf_basic`
* C source: `picoquictest/netperf_test.c:484-491`
* Rust source: `rs/fq/src/tests/netperf.rs:225-233`

### C test body
```c
{
    int ret = netperf_one_scenario(netperf_scenario_basic, sizeof(netperf_scenario_basic),
        NULL,
        0, 0, 0, 0, 0, 1000000, NULL, NULL, 10 * PICOQUIC_MAX_PACKET_SIZE);

    return ret;
}
```

### Rust test body
```rust
fn netperf_basic() {
    netperf_one_scenario(
        NETPERF_SCENARIO_BASIC,
        None,
        0,
        1_000_000,
        10 * MAX_PACKET_SIZE,
    );
}
```

## `picoquictest/pacing_test.c:pacing_fast_test`
* C test-table name: `pacing_fast`
* C entry function: `pacing_fast_test`
* Rust test: `pacing_fast`
* C source: `picoquictest/pacing_test.c:238-242`
* Rust source: `rs/fq/src/tests/pacing.rs:628-630`

### C test body
```c
{
    int ret = pacing_cc_algotest(picoquic_fastcc_algorithm, 1000000, 180);
    return ret;
}
```

### Rust test body
```rust
fn pacing_fast() {
    pacing_cc_algotest("fastcc", 1_000_000, 180);
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
