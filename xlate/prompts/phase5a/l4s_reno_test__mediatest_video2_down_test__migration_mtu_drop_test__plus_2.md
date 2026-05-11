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

## `picoquictest/l4s_test.c:l4s_reno_test`
* C test-table name: `l4s_reno`
* C entry function: `l4s_reno_test`
* Rust test: `l4s_reno`
* C source: `picoquictest/l4s_test.c:131-141`
* Rust source: `rs/fq/src/tests/l4s.rs:154-157`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgo = picoquic_newreno_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 5600000, 45, 3000, 0, NULL);

    return ret;
}
```

### Rust test body
```rust
fn l4s_reno() {
    let ccalgo = get_congestion_algorithm("newreno").expect("newreno cc algo");
    l4s_congestion_test(ccalgo, true, 5_600_000, 45, 3_000, &[]);
}
```

## `picoquictest/mediatest.c:mediatest_video2_down_test`
* C test-table name: `mediatest_video2_down`
* C entry function: `mediatest_video2_down_test`
* Rust test: `mediatest_video2_down`
* C source: `picoquictest/mediatest.c:1382-1398`
* Rust source: `rs/fq/src/tests/mediatest.rs:269-282`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.latency_average = 100000;
    spec.latency_max = 600000;
    spec.do_not_check_video2 = 1;
    ret = mediatest_one(mediatest_video2_down, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_video2_down() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 100_000,
        latency_max: 600_000,
        do_not_check_video2: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Down, &spec).expect("mediatest_video2_down");
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
