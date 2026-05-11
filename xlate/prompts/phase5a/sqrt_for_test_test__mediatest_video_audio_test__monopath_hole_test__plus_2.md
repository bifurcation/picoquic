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

## `picoquictest/intformattest.c:sqrt_for_test_test`
* C test-table name: `sqrt_for_test`
* C entry function: `sqrt_for_test_test`
* Rust test: `sqrt_for_test`
* C source: `picoquictest/intformattest.c:384-401`
* Rust source: `rs/fq/src/tests/intformattest.rs:260-270`

### C test body
```c
{
    int ret = 0;
    uint64_t x_base = 0;

    while (x_base < 0xffffffff && ret == 0) {
        for (uint64_t i = 0; i < 2; i++) {
            uint64_t y = x_base * (x_base+i);
            uint64_t x = picoquic_sqrt_for_tests(y);
            if (x != x_base) {
                ret = -1;
                break;
            }
        }
        x_base = 2 * x_base + 1;
    }
    return ret;
}
```

### Rust test body
```rust
fn sqrt_for_test() {
    let mut x_base: u64 = 0;
    while x_base < 0xffff_ffff {
        for i in 0u64..2 {
            let y = x_base * (x_base + i);
            let x = sqrt_for_tests(y);
            assert_eq!(x, x_base, "sqrt_for_tests({x_base}*({x_base}+{i})) = {x}");
        }
        x_base = 2 * x_base + 1;
    }
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

## `picoquictest/multipath_test.c:monopath_hole_test`
* C test-table name: `monopath_hole`
* C entry function: `monopath_hole_test`
* Rust test: `monopath_hole`
* C source: `picoquictest/multipath_test.c:1678-1682`
* Rust source: `rs/fq/src/tests/multipath.rs:1382-1384`

### C test body
```c
{
    return monopath_test_one(monopath_test_hole);
}
```

### Rust test body
```rust
fn monopath_hole() {
    monopath_test_one(MonopathTestId::Hole);
}
```

## `picoquictest/multipath_test.c:multipath_backup_test`
* C test-table name: `multipath_backup`
* C entry function: `multipath_backup_test`
* Rust test: `multipath_backup`
* C source: `picoquictest/multipath_test.c:1504-1509`
* Rust source: `rs/fq/src/tests/multipath.rs:1492-1494`

### C test body
```c
{
    uint64_t max_completion_microsec = 2000000;

    return multipath_test_one(max_completion_microsec, multipath_test_backup);
}
```

### Rust test body
```rust
fn multipath_backup() {
    multipath_test_one(2_000_000, MultipathTestId::Backup);
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
