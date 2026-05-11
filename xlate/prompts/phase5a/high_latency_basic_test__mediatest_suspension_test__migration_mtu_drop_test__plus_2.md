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

## `picoquictest/high_latency_test.c:high_latency_basic_test`
* C test-table name: `high_latency_basic`
* C entry function: `high_latency_basic_test`
* Rust test: `high_latency_basic`
* C source: `picoquictest/high_latency_test.c:157-166`
* Rust source: `rs/fq/src/tests/high_latency.rs:253-274`

### C test body
```c
{
    /* Simple test. */
    uint64_t latency = 5000000;
    uint64_t expected_completion = latency*7;

    return high_latency_one(0xba, picoquic_newreno_algorithm, 
        hilat_scenario_basic, sizeof(hilat_scenario_basic),
        expected_completion, latency, 10, 10, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn high_latency_basic() {
    let latency = 5_000_000u64;
    let newreno = get_congestion_algorithm("reno").expect("newreno");
    high_latency_one(
        0xba,
        newreno,
        &[TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 2_000,
        }],
        latency * 7,
        latency,
        10,
        10,
        0,
        false,
        false,
        false,
    );
}
```

## `picoquictest/mediatest.c:mediatest_suspension_test`
* C test-table name: `mediatest_suspension`
* C entry function: `mediatest_suspension_test`
* Rust test: `mediatest_suspension`
* C source: `picoquictest/mediatest.c:1490-1510`
* Rust source: `rs/fq/src/tests/mediatest.rs:376-393`

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
    spec.latency_average = 50000;
    spec.latency_max = 300000;
    spec.do_not_check_video2 = 1;
    spec.nb_suspensions = 1;
    spec.suspension_start_time = 4000000;
    spec.suspension_down_time = 150000;
    spec.suspension_up_time = 50000;
    ret = mediatest_one(mediatest_suspension, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_suspension() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 50_000,
        latency_max: 300_000,
        do_not_check_video2: true,
        nb_suspensions: 1,
        suspension_start_time: 4_000_000,
        suspension_down_time: 150_000,
        suspension_up_time: 50_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::Suspension, &spec).expect("mediatest_suspension");
}
```

## `picoquictest/multipath_test.c:migration_mtu_drop_test`
* C test-table name: `migration_mtu_drop`
* C entry function: `migration_mtu_drop_test`
* Rust test: `migration_mtu_drop`
* C source: `picoquictest/multipath_test.c:374-377`
* Rust source: `rs/fq/src/tests/multipath.rs:1322-1324`

### C test body
```c
{
    return migration_test_one(1);
}
```

### Rust test body
```rust
fn migration_mtu_drop() {
    migration_test_one(true);
}
```

## `picoquictest/multipath_test.c:multipath_abandon_test`
* C test-table name: `multipath_abandon`
* C entry function: `multipath_abandon_test`
* Rust test: `multipath_abandon`
* C source: `picoquictest/multipath_test.c:1415-1422`
* Rust source: `rs/fq/src/tests/multipath.rs:1406-1408`

### C test body
```c
{
    uint64_t max_completion_microsec = 3800000;

    return  multipath_test_one(max_completion_microsec, multipath_test_abandon);
}
```

### Rust test body
```rust
fn multipath_abandon() {
    multipath_test_one(3_800_000, MultipathTestId::Abandon);
}
```

## `picoquictest/multipath_test.c:multipath_callback_test`
* C test-table name: `multipath_callback`
* C entry function: `multipath_callback_test`
* Rust test: `multipath_callback`
* C source: `picoquictest/multipath_test.c:1452-1457`
* Rust source: `rs/fq/src/tests/multipath.rs:1516-1518`

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Rust test body
```rust
fn multipath_callback() {
    multipath_test_one(1_000_000, MultipathTestId::Callback);
}
```
