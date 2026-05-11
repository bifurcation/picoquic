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

## `picoquictest/parseheadertest.c:incoming_initial_test`
* C test-table name: `incoming_initial`
* C entry function: `incoming_initial_test`
* Rust test: `incoming_initial`
* C source: `picoquictest/parseheadertest.c:975-980`
* Rust source: `rs/fq/src/tests/parseheadertest.rs:503-528`

### C test body
```c
{
    int ret = packet_initial_dec_one(packet_intel_bug, sizeof(packet_intel_bug));

    return ret;
}
```

### Rust test body
```rust
fn incoming_initial() {
    let current_time = Instant::from_ticks(0);
    let addr_c: SocketAddr = "0.0.0.0:12345".parse().unwrap();
    let addr_s: SocketAddr = "0.0.0.0:443".parse().unwrap();

    let mut quic = Quic::new(
        8,
        Some(util::TEST_FILE_SERVER_CERT),
        Some(util::TEST_FILE_SERVER_KEY),
        Some(util::TEST_FILE_CERT_STORE),
        Some("h3"),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("create server quic");

    let mut bytes = PACKET_INTEL_BUG.to_vec();
    let new_cnx = quic
        .incoming_packet_ex(&mut bytes, &addr_c, &addr_s, 0, 0, current_time)
        .expect("incoming_packet_ex");
    assert!(new_cnx.is_some(), "expected a new connection to be created");
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

## `picoquictest/satellite_test.c:satellite_small_up_test`
* C test-table name: `satellite_small_up`
* C entry function: `satellite_small_up_test`
* Rust test: `satellite_small_up`
* C source: `picoquictest/satellite_test.c:271-275`
* Rust source: `rs/fq/src/tests/satellite.rs:361-376`

### C test body
```c
{
    /* Should be less than 420 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 400000000, 2, 10, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_small_up() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        400_000_000,
        2,
        10,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/skip_frame_test.c:send_stream_blocked_test`
* C test-table name: `send_stream_blocked`
* C entry function: `send_stream_blocked_test`
* Rust test: `send_stream_blocked`
* C source: `picoquictest/skip_frame_test.c:3452-3463`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1016-1021`

### C test body
```c
{
    int ret = 0;

    for (size_t i = 0; ret == 0 && i < nb_stream_blocked_test; i++) {
        if ((ret = send_stream_blocked_test_one(&stream_blocked_test[i])) != 0) {
            DBG_PRINTF("Stream blocked test %d failed", (int)i);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn send_stream_blocked() {
    const NB_CASES: usize = 4; // representative count; full list in Phase 4
    for i in 0..NB_CASES {
        send_stream_blocked_test_one(i).expect("stream blocked case");
    }
}
```

## `picoquictest/sockloop_test.c:sockloop_ipv4_test`
* C test-table name: `sockloop_ipv4`
* C entry function: `sockloop_ipv4_test`
* Rust test: `sockloop_ipv4`
* C source: `picoquictest/sockloop_test.c:669-679`
* Rust source: `rs/fq/src/tests/sockloop.rs:584-590`

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 4);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);

    return(sockloop_test_one(&spec));
}
```

### Rust test body
```rust
fn sockloop_ipv4() {
    let mut spec = SockloopTestSpec::new(4);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    sockloop_test_one(&spec);
}
```
