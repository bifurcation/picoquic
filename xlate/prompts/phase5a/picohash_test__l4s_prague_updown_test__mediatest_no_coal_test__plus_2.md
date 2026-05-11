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

## `picoquictest/hashtest.c:picohash_test`
* C test-table name: `picohash`
* C entry function: `picohash_test`
* Rust test: `picohash`
* C source: `picoquictest/hashtest.c:197-200`
* Rust source: `rs/fq/src/tests/hashtest.rs:26-78`

### C test body
```c
{
    return(picohash_test_one(0));
}
```

### Rust test body
```rust
fn picohash() {
    use crate::hash::HashTable;

    let mut t: HashTable<u64, ()> = HashTable::new(32).expect("create hash table");

    assert_eq!(t.len(), 0);

    // Insert odd values 1, 3, 5, 7, 9.
    for i in (1u64..10).step_by(2) {
        assert!(t.insert(i, ()).is_ok(), "insert({i}) failed");
    }
    assert_eq!(t.len(), 5);

    // Every inserted value is retrievable.
    for i in (1u64..10).step_by(2) {
        assert!(t.lookup(&i).is_some(), "lookup({i}) failed");
    }

    // Create collisions: for k in {1, 5}, insert k + 32*j for j in 1..=k.
    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.insert(key, ()).is_ok(), "insert({key}) failed");
        }
    }
    // Original 5 + 1 + 5 = 11.
    assert_eq!(t.len(), 11);

    // Collision entries are retrievable.
    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.lookup(&key).is_some(), "lookup({key}) failed");
        }
    }

    // Even values 0, 2, 4, 6, 8, 10 were never inserted.
    for i in (0u64..=10).step_by(2) {
        assert!(t.lookup(&i).is_none(), "lookup({i}) returned invalid item");
    }

    // Delete values 1 and 9 (first and near-last of the originals).
    for i in (1u64..10).step_by(4) {
        let tok = t.lookup(&i).expect("pre-delete lookup");
        t.remove(tok);
    }
    assert_eq!(t.len(), 8);

    // Deleted values are gone.
    for i in (1u64..10).step_by(4) {
        assert!(t.lookup(&i).is_none(), "deleted value {i} still found");
    }
}
```

## `picoquictest/l4s_test.c:l4s_prague_updown_test`
* C test-table name: `l4s_prague_updown`
* C entry function: `l4s_prague_updown_test`
* Rust test: `l4s_prague_updown`
* C source: `picoquictest/l4s_test.c:179-186`
* Rust source: `rs/fq/src/tests/l4s.rs:169-172`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgo = picoquic_prague_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 6300000, 55, 6000, nb_l4s_link_updown, l4s_link_updown);

    return ret;
}
```

### Rust test body
```rust
fn l4s_prague_updown() {
    let ccalgo = get_congestion_algorithm("prague").expect("prague cc algo");
    l4s_congestion_test(ccalgo, true, 6_300_000, 55, 6_000, L4S_LINK_UPDOWN);
}
```

## `picoquictest/mediatest.c:mediatest_no_coal_test`
* C test-table name: `mediatest_no_coal`
* C entry function: `mediatest_no_coal_test`
* Rust test: `mediatest_no_coal`
* C source: `picoquictest/mediatest.c:1450-1463`
* Rust source: `rs/fq/src/tests/mediatest.rs:360-371`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    spec.data_size = 10000000;
    spec.no_coal = 1;
    ret = mediatest_one(mediatest_no_coal, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_no_coal() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        no_coal: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::NoCoal, &spec).expect("mediatest_no_coal");
}
```

## `picoquictest/mediatest.c:mediatest_video_data_audio_test`
* C test-table name: `mediatest_video_data_audio`
* C entry function: `mediatest_video_data_audio_test`
* Rust test: `mediatest_video_data_audio`
* C source: `picoquictest/mediatest.c:1368-1380`
* Rust source: `rs/fq/src/tests/mediatest.rs:254-264`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    spec.data_size = 10000000;
    ret = mediatest_one(mediatest_video_data_audio, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_video_data_audio() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::VideoDataAudio, &spec).expect("mediatest_video_data_audio");
}
```

## `picoquictest/multipath_test.c:monopath_0rtt_test`
* C test-table name: `monopath_0rtt`
* C entry function: `monopath_0rtt_test`
* Rust test: `monopath_0rtt`
* C source: `picoquictest/multipath_test.c:1701-1706`
* Rust source: `rs/fq/src/tests/multipath.rs:1328-1334`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.do_multipath = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn monopath_0rtt() {
    let zrt = ZeroRttTest {
        do_multipath: true,
        ..ZeroRttTest::default()
    };
    zero_rtt_test_one(&zrt).expect("monopath_0rtt");
}
```
