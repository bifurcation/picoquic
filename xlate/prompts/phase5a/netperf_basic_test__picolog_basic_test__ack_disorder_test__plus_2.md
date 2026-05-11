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

## `picoquictest/picolog_test.c:picolog_basic_test`
* C test-table name: `picolog_basic`
* C entry function: `picolog_basic_test`
* Rust test: `picolog_basic`
* C source: `picoquictest/picolog_test.c:81-165`
* Rust source: `rs/fq/src/tests/picolog.rs:390-426`

### C test body
```c
{
    int ret = 0;
    /* find the test input file */
    char log_test_input[512];
    char svg_template[512];
    app_conversion_context_t appctx = { 0 };
    picohash_table* cids = NULL;

    appctx.binlog_name = "?";
    appctx.out_dir = ".";
    ret = picoquic_get_input_path(log_test_input, sizeof(log_test_input), picoquic_solution_dir, PICOLOG_BIN_INPUT);
    if (ret == 0) {
        appctx.binlog_name = log_test_input;
        if ((appctx.f_binlog = picoquic_file_open(log_test_input, "rb")) == NULL) {
            ret = -1;
        }
    }
    if (ret == 0) {
        ret = picoquic_get_input_path(svg_template, sizeof(svg_template), picoquic_solution_dir, PICOLOG_SVG_TEMPLATE);
        if (ret == 0) {
            if ((appctx.f_template = picoquic_file_open(svg_template, "r")) == NULL) {
                ret = -1;
            }
        }
    }

    if (ret == 0) {
        cids = cidset_create();
        if (cids == NULL) {
            ret = -1;
        }
        else {
            binlog_list_cids(appctx.f_binlog, cids);
            if (cids->count == 0) {
                ret = -1;
            }
        }
    }
    
    if (ret == 0) {
        FILE* cid_prints;
        if ((cid_prints = picoquic_file_open(CIDSET_OUTPUT, "w")) == NULL) {
            ret = -1;
        }
        else {
            cidset_print(cid_prints, cids);
            (void)picoquic_file_close(cid_prints);
        }
    }

    if (ret == 0) {
        picoquic_connection_id_t cid_test = { {11, 12, 13, 14, 15, 16, 17, 18}, 8 };

        if (cidset_has_cid(cids, &cid_test)) {
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = cidset_iterate(cids, test_convert_svg, &appctx);
    }

    (void)picoquic_file_close(appctx.f_binlog);
    (void)picoquic_file_close(appctx.f_template);
    if (cids != NULL) {
        (void)cidset_delete(cids);
    }
    /* compare the log file to the expected value */
    if (ret == 0)
    {
        char svglog_ref[512];

        ret = picoquic_get_input_path(svglog_ref, sizeof(svglog_ref), picoquic_solution_dir, SVG_LOG_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the svglog test ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(SVG_LOG_OUTPUT, svglog_ref);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn picolog_basic() {
    let mut _simulated_time = Instant::from_ticks(0);

    let log_input =
        get_input_path("picoquictest/picolog_test_input.log").expect("get log input path");
    let svg_template_path = get_input_path("loglib/template.svg").expect("get svg template path");

    let mut f_binlog = std::fs::File::open(&log_input).expect("open binlog");
    let f_template = std::fs::File::open(&svg_template_path).expect("open svg template");

    let mut cids = cidset_create().expect("cidset_create");

    binlog_list_cids(&mut f_binlog, &mut cids);
    assert!(!cids.is_empty(), "cids must not be empty");

    {
        let mut cid_prints = std::fs::File::create("./cidset.txt").expect("create cidset output");
        cidset_print(&mut cid_prints, &cids);
    }

    let cid_test = crate::ConnectionId::clone_from_slice(&[11, 12, 13, 14, 15, 16, 17, 18])
        .expect("build test CID");
    assert!(!cidset_has_cid(&cids, &cid_test), "unexpected CID in set");

    let mut ctx = SvgConvertCtx {
        binlog_name: log_input.clone(),
        out_dir: ".",
        f_binlog: Some(std::fs::File::open(&log_input).expect("reopen binlog")),
        f_template: Some(f_template),
    };
    let ret = cidset_iterate(&cids, svg_convert, &mut ctx);
    assert_eq!(ret, 0, "cidset_iterate failed");

    let svglog_ref = get_input_path("picoquictest/svglog_ref.svg").expect("get svglog ref path");
    compare_text_files("./0102030405060708.svg", &svglog_ref)
        .expect("svg output matches reference");
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

## `picoquictest/satellite_test.c:satellite_cubic_loss_test`
* C test-table name: `satellite_cubic_loss`
* C entry function: `satellite_cubic_loss_test`
* Rust test: `satellite_cubic_loss`
* C source: `picoquictest/satellite_test.c:295-299`
* Rust source: `rs/fq/src/tests/satellite.rs:437-452`

### C test body
```c
{
    /* Should be less than 10 sec per draft etosat, but cubic is a bit slower */
    return satellite_test_one(picoquic_cubic_algorithm, 100000000, 7500000, 250, 3, 0, 1, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_cubic_loss() {
    let cubic = get_congestion_algorithm("cubic").expect("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        7_500_000,
        250,
        3,
        0,
        true,
        false,
        false,
        false,
        false,
    );
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
