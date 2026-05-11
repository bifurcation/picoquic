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

## `picoquictest/multipath_test.c:multipath_keep_alive_test`
* C test-table name: `multipath_keep_alive`
* C entry function: `multipath_keep_alive_test`
* Rust test: `multipath_keep_alive`
* C source: `picoquictest/multipath_test.c:1525-1530`
* Rust source: `rs/fq/src/tests/multipath.rs:1564-1566`

### C test body
```c
{
    uint64_t max_completion_microsec = 210000000;

    return multipath_test_one(max_completion_microsec, multipath_test_keep_alive);
}
```

### Rust test body
```rust
fn multipath_keep_alive() {
    multipath_test_one(210_000_000, MultipathTestId::KeepAlive);
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

## `picoquictest/openssl_test.c:openssl_cert_test`
* C test-table name: `openssl_cert`
* C entry function: `openssl_cert_test`
* Rust test: `openssl_cert`
* C source: `picoquictest/openssl_test.c:45-50`
* Rust source: `rs/fq/src/tests/openssl.rs:26-30`

### C test body
```c
{
    /* Nothing to do, as the module is not loaded. */
    return 0;
}
```

### Rust test body
```rust
fn openssl_cert() {
    openssl_cert_test_one("cert.pem", 1);
    openssl_cert_test_one("chain.pem", 1);
    openssl_cert_test_one("fullchain.pem", 2);
}
```

## `picoquictest/pacing_test.c:pacing_repeat_test`
* C test-table name: `pacing_repeat`
* C entry function: `pacing_repeat_test`
* Rust test: `pacing_repeat`
* C source: `picoquictest/pacing_test.c:297-348`
* Rust source: `rs/fq/src/tests/pacing.rs:538-606`

### C test body
```c
{
    int ret = 0;
    picoquic_pacing_t pacing = { 0 };

    /* set either CWIN or data rate to expected value */
    for (size_t i = 0; ret == 0 && i < nb_pacing_events; i++) {
        if (pacing_events[i].length == 0) {
            /* This is a set up event */
            if (pacing_events[i].cwin == 0) {
                /* directly set the quantum and rate */
                picoquic_update_pacing_parameters(&pacing, (double)pacing_events[i].rate, 
                    pacing_events[i].quantum, pacing_events[i].send_mtu, pacing_events[i].rtt,
                    NULL);
            }
            else {
                /* Set control based on CWIN and RTT */
                picoquic_update_pacing_window(&pacing, pacing_events[i].slow_start,
                    pacing_events[i].cwin, pacing_events[i].send_mtu, pacing_events[i].rtt, NULL);
            }
            /* Check that the value are as expected */
            if (pacing.rate != pacing_events[i].rate ||
                (uint64_t)pacing.packet_time_nanosec != pacing_events[i].expected_packet_nanosec ||
                pacing.bucket_max != pacing_events[i].expected_bucket_nanosec) {
                DBG_PRINTF("Event %d, expected rate: " PRIu64 ", Packet_n: " PRIu64 ", Bucket: " PRIu64,
                    i, pacing.rate, pacing.packet_time_nanosec, pacing.bucket_max);
                ret = -1;
            }
        }
        else {
            /* Set using CWIN and RTT */
            uint64_t next_time = UINT64_MAX;
            int is_ok = picoquic_is_authorized_by_pacing(&pacing, pacing_events[i].current_time, &next_time, 0, NULL);
            if (is_ok != pacing_events[i].expected_ok) {
                DBG_PRINTF("Event %d, expected OK: %d", i, is_ok);
                ret = -1;
            }
            else {
                if (is_ok) {
                    picoquic_update_pacing_data_after_send(&pacing, pacing_events[i].length, pacing_events[i].send_mtu, pacing_events[i].current_time);
                }
            }
            if (pacing.bucket_nanosec != pacing_events[i].expected_bucket_nanosec ||
                next_time != pacing_events[i].expected_next_time) {
                DBG_PRINTF("Event %d, expected bucket: " PRIu64,
                    i, pacing.rate, pacing.bucket_nanosec);
                ret = -1;
            }
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn pacing_repeat() {
    let mut pacing = Pacing {
        rate: 0,
        evaluation_time: Instant::from_ticks(0),
        bucket_max: 0,
        packet_time_microsec: Duration::from_ticks(0),
        quantum_max: 0,
        rate_max: 0,
        bandwidth_pause: 0,
        bucket_nanosec: 0,
        packet_time_nanosec: 0,
    };

    for (i, ev) in PACING_EVENTS.iter().enumerate() {
        if ev.length == 0 {
            if ev.cwin == 0 {
                pacing.update_parameters(
                    ev.rate as f64,
                    ev.quantum,
                    ev.send_mtu,
                    Duration::from_ticks(ev.rtt),
                    None,
                );
            } else {
                pacing.update_window(
                    ev.slow_start,
                    ev.cwin,
                    ev.send_mtu,
                    Duration::from_ticks(ev.rtt),
                    None,
                );
            }
            assert_eq!(pacing.rate, ev.rate, "event {i}: rate");
            assert_eq!(
                pacing.packet_time_nanosec, ev.expected_packet_nanosec,
                "event {i}: packet_time_nanosec"
            );
            assert_eq!(
                pacing.bucket_max, ev.expected_bucket_nanosec,
                "event {i}: bucket_max"
            );
        } else {
            let mut next_time = Instant::from_ticks(u64::MAX);
            let is_ok = pacing.is_authorized(
                Instant::from_ticks(ev.current_time),
                &mut next_time,
                false,
                None,
            );
            assert_eq!(is_ok, ev.expected_ok, "event {i}: is_ok");
            if is_ok {
                pacing.update_after_send(
                    ev.length,
                    ev.send_mtu,
                    Instant::from_ticks(ev.current_time),
                );
            }
            assert_eq!(
                pacing.bucket_nanosec, ev.expected_bucket_nanosec,
                "event {i}: bucket_nanosec"
            );
            assert_eq!(
                next_time.ticks(),
                ev.expected_next_time,
                "event {i}: next_time"
            );
        }
    }
}
```
