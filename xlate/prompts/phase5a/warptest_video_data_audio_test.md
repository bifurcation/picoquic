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

## `picoquictest/warptest.c:warptest_video_data_audio_test`
* C test-table name: `warptest_video_data_audio`
* C entry function: `warptest_video_data_audio_test`
* Rust test: `warptest_video_data_audio`
* C source: `picoquictest/warptest.c:1524-1536`
* Rust source: `rs/fq/src/tests/warptest.rs:57-67`

### C test body
```c
{
    int ret;
    warptest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    spec.data_size = 10000000;
    ret = warptest_one(3, &spec);

    return ret;
}
```

### Rust test body
```rust
fn warptest_video_data_audio() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        ..Default::default()
    };
    warptest_one(3, &spec).expect("warptest_video_data_audio");
}
```
