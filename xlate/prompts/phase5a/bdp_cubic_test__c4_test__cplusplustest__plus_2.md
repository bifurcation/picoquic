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

## `picoquictest/congestion_test.c:bdp_cubic_test`
* C test-table name: `bdp_cubic`
* C entry function: `bdp_cubic_test`
* Rust test: `bdp_cubic`
* C source: `picoquictest/congestion_test.c:717-722`
* Rust source: `rs/fq/src/tests/congestion.rs:918-920`

### C test body
```c
{
    /* We do not run this test in Win32 builds. */
    return 0;
}
```

### Rust test body
```rust
fn bdp_cubic() {
    bdp_option_test_one(BdpTestOption::Cubic);
}
```

## `picoquictest/congestion_test.c:c4_test`
* C test-table name: `c4`
* C entry function: `c4_test`
* Rust test: `c4`
* C source: `picoquictest/congestion_test.c:120-123`
* Rust source: `rs/fq/src/tests/congestion.rs:747-750`

### C test body
```c
{
    return congestion_control_test(c4_algorithm, 3600000, 0, 0);
}
```

### Rust test body
```rust
fn c4() {
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    congestion_control_test(ccalgo, 3_600_000, 0, 0);
}
```

## `picoquictest/cplusplus.cpp:cplusplustest`
* C test-table name: `cplusplus`
* C entry function: `cplusplustest`
* Rust test: `cplusplus`
* C source: `picoquictest/cplusplus.cpp:42-45`
* Rust source: `rs/fq/src/tests/cplusplus.rs:8-10`

### C test body
```c
    int cplusplustest(void) {
        return 0;
    }
```

### Rust test body
```rust
fn cplusplus() {
    // C: `cplusplustest` — no-op; proves C++ compilation succeeds.
}
```

## `picoquictest/datagram_tests.c:datagram_rt_test`
* C test-table name: `datagram_rt`
* C entry function: `datagram_rt_test`
* Rust test: `datagram_rt`
* C source: `picoquictest/datagram_tests.c:584-597`
* Rust source: `rs/fq/src/tests/datagram.rs:667-677`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 20000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;
    dg_ctx.dg_latency_target[0] = 18000;
    dg_ctx.dg_latency_target[1] = 18000;

    return datagram_test_one(2, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_rt() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [18_000, 18_000],
        ..Default::default()
    };
    datagram_test_one(2, &mut dg_ctx, 0);
}
```

## `picoquictest/datagram_tests.c:datagram_wifi_test`
* C test-table name: `datagram_wifi`
* C entry function: `datagram_wifi_test`
* Rust test: `datagram_wifi`
* C source: `picoquictest/datagram_tests.c:694-710`
* Rust source: `rs/fq/src/tests/datagram.rs:803-816`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 1000;
    dg_ctx.dg_target[1] = 1000;
    dg_ctx.send_delay = 2000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;
    dg_ctx.dg_latency_target[0] = 305000;
    dg_ctx.dg_latency_target[1] = 280000;
    dg_ctx.test_wifi = 1;
    dg_ctx.nb_trials_max = 64000;
    dg_ctx.link_latency = 25000;

    return datagram_test_one(8, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_wifi() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [1_000, 1_000],
        send_delay: 2_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [305_000, 280_000],
        test_wifi: true,
        nb_trials_max: 64_000,
        link_latency: 25_000,
        ..Default::default()
    };
    datagram_test_one(8, &mut dg_ctx, 0);
}
```
