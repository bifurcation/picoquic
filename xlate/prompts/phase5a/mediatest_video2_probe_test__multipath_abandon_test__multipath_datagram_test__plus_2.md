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

## `picoquictest/multipath_test.c:multipath_datagram_test`
* C test-table name: `multipath_datagram`
* C entry function: `multipath_datagram_test`
* Rust test: `multipath_datagram`
* C source: `picoquictest/multipath_test.c:1489-1495`
* Rust source: `rs/fq/src/tests/multipath.rs:1522-1524`

### C test body
```c
{
    /* TODO: investigate why 1.15 instead of 1.12 with prior implementation of multipath */
    uint64_t max_completion_microsec = 1150000;

    return multipath_test_one(max_completion_microsec, multipath_test_datagram);
}
```

### Rust test body
```rust
fn multipath_datagram() {
    multipath_test_one(1_150_000, MultipathTestId::Datagram);
}
```

## `picoquictest/multipath_test.c:multipath_nat_test`
* C test-table name: `multipath_nat`
* C entry function: `multipath_nat_test`
* Rust test: `multipath_nat`
* C source: `picoquictest/multipath_test.c:1372-1378`
* Rust source: `rs/fq/src/tests/multipath.rs:1570-1572`

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_nat);
}
```

### Rust test body
```rust
fn multipath_nat() {
    multipath_test_one(3_000_000, MultipathTestId::Nat);
}
```

## `picoquictest/multipath_test.c:multipath_socket0_error_test`
* C test-table name: `multipath_socket0_error`
* C entry function: `multipath_socket0_error_test`
* Rust test: `multipath_socket0_error`
* C source: `picoquictest/multipath_test.c:1406-1413`
* Rust source: `rs/fq/src/tests/multipath.rs:1627-1629`

### C test body
```c
{
    uint64_t max_completion_microsec = 10900000;

    return  multipath_test_one(max_completion_microsec, multipath_test_break3);
}
```

### Rust test body
```rust
fn multipath_socket0_error() {
    multipath_test_one(10_900_000, MultipathTestId::Break3);
}
```
