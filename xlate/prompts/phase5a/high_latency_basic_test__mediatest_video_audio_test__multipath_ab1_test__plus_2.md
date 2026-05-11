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

## `picoquictest/mediatest.c:mediatest_video_audio_test`
* C test-table name: `mediatest_video_audio`
* C entry function: `mediatest_video_audio_test`
* Rust test: `mediatest_video_audio`
* C source: `picoquictest/mediatest.c:1355-1366`
* Rust source: `rs/fq/src/tests/mediatest.rs:241-250`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    ret = mediatest_one(mediatest_video_audio, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_video_audio() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::VideoAudio, &spec).expect("mediatest_video_audio");
}
```

## `picoquictest/multipath_test.c:multipath_ab1_test`
* C test-table name: `multipath_ab1`
* C entry function: `multipath_ab1_test`
* Rust test: `multipath_ab1`
* C source: `picoquictest/multipath_test.c:1315-1320`
* Rust source: `rs/fq/src/tests/multipath.rs:1400-1402`

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return multipath_test_one(max_completion_microsec, multipath_test_ab1);
}
```

### Rust test body
```rust
fn multipath_ab1() {
    multipath_test_one(3_000_000, MultipathTestId::Ab1);
}
```

## `picoquictest/multipath_test.c:multipath_break_both_test`
* C test-table name: `multipath_break_both`
* C entry function: `multipath_break_both_test`
* Rust test: `multipath_break_both`
* C source: `picoquictest/multipath_test.c:1540-1547`
* Rust source: `rs/fq/src/tests/multipath.rs:1510-1512`

### C test body
```c
{
    uint64_t max_completion_microsec = 1060000;

    return multipath_test_one(max_completion_microsec, multipath_test_break_both);
}
```

### Rust test body
```rust
fn multipath_break_both() {
    multipath_test_one(1_060_000, MultipathTestId::BreakBoth);
}
```

## `picoquictest/multipath_test.c:multipath_just_one_test`
* C test-table name: `multipath_just_one`
* C entry function: `multipath_just_one_test`
* Rust test: `multipath_just_one`
* C source: `picoquictest/multipath_test.c:1532-1538`
* Rust source: `rs/fq/src/tests/multipath.rs:1558-1560`

### C test body
```c
{
    uint64_t max_completion_microsec = 1060000;

    return multipath_test_one(max_completion_microsec, multipath_test_just_one);
}
```

### Rust test body
```rust
fn multipath_just_one() {
    multipath_test_one(1_060_000, MultipathTestId::JustOne);
}
```
