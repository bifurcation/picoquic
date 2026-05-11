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

## `picoquictest/mediatest.c:mediatest_video2_probe_test`
* C test-table name: `mediatest_video2_probe`
* C entry function: `mediatest_video2_probe_test`
* Rust test: `mediatest_video2_probe`
* C source: `picoquictest/mediatest.c:1418-1434`
* Rust source: `rs/fq/src/tests/mediatest.rs:304-317`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.1;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.latency_average = 25000;
    spec.latency_max = 150000;
    spec.do_probe_up = 1;
    ret = mediatest_one(mediatest_video2_probe, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_video2_probe() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 25_000,
        latency_max: 150_000,
        do_probe_up: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Probe, &spec).expect("mediatest_video2_probe");
}
```

## `picoquictest/multipath_test.c:multipath_back0_test`
* C test-table name: `multipath_back0`
* C entry function: `multipath_back0_test`
* Rust test: `multipath_back0`
* C source: `picoquictest/multipath_test.c:1424-1432`
* Rust source: `rs/fq/src/tests/multipath.rs:1480-1482`

### C test body
```c
{
    uint64_t max_completion_microsec = 3300000;

    return  multipath_test_one(max_completion_microsec, multipath_test_back0);
}
```

### Rust test body
```rust
fn multipath_back0() {
    multipath_test_one(3_300_000, MultipathTestId::Back0);
}
```

## `picoquictest/multipath_test.c:multipath_drop_first_test`
* C test-table name: `multipath_drop_first`
* C entry function: `multipath_drop_first_test`
* Rust test: `multipath_drop_first`
* C source: `picoquictest/multipath_test.c:1326-1331`
* Rust source: `rs/fq/src/tests/multipath.rs:1540-1542`

### C test body
```c
{
    uint64_t max_completion_microsec = 1490000;

    return multipath_test_one(max_completion_microsec, multipath_test_drop_first);
}
```

### Rust test body
```rust
fn multipath_drop_first() {
    multipath_test_one(1_490_000, MultipathTestId::DropFirst);
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

## `picoquictest/multipath_test.c:multipath_stream_af_test`
* C test-table name: `multipath_stream_af`
* C entry function: `multipath_stream_af_test`
* Rust test: `multipath_stream_af`
* C source: `picoquictest/multipath_test.c:1482-1487`
* Rust source: `rs/fq/src/tests/multipath.rs:1645-1647`

### C test body
```c
{
    uint64_t max_completion_microsec = 1500000;

    return multipath_test_one(max_completion_microsec, multipath_test_stream_af);
}
```

### Rust test body
```rust
fn multipath_stream_af() {
    multipath_test_one(1_500_000, MultipathTestId::StreamAf);
}
```
