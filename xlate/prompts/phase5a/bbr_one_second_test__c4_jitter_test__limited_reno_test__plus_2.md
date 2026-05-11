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

## `picoquictest/congestion_test.c:bbr_one_second_test`
* C test-table name: `bbr_one_second`
* C entry function: `bbr_one_second_test`
* Rust test: `bbr_one_second`
* C source: `picoquictest/congestion_test.c:359-370`
* Rust source: `rs/fq/src/tests/congestion.rs:827-832`

### C test body
```c
{
    uint64_t max_completion_time = 90000000;
    uint64_t latency = 1000000;
    uint64_t jitter = 3000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 1;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Rust test body
```rust
fn bbr_one_second() {
    let latency = 1_000_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(90_000_000, 1, latency, jitter, buffer);
}
```

## `picoquictest/congestion_test.c:c4_jitter_test`
* C test-table name: `c4_jitter`
* C entry function: `c4_jitter_test`
* Rust test: `c4_jitter`
* C source: `picoquictest/congestion_test.c:125-128`
* Rust source: `rs/fq/src/tests/congestion.rs:754-757`

### C test body
```c
{
    return congestion_control_test(c4_algorithm, 3650000, 5000, 5);
}
```

### Rust test body
```rust
fn c4_jitter() {
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    congestion_control_test(ccalgo, 3_650_000, 5_000, 5);
}
```

## `picoquictest/cpu_limited.c:limited_reno_test`
* C test-table name: `limited_reno`
* C entry function: `limited_reno_test`
* Rust test: `limited_reno`
* C source: `picoquictest/cpu_limited.c:210-218`
* Rust source: `rs/fq/src/tests/cpu_limited.rs:160-165`

### C test body
```c
{
    limited_test_config_t config;
    limited_config_set_default(&config, 1);
    config.ccalgo = picoquic_newreno_algorithm;
    config.max_completion_time = 4600000;

    return limited_client_test_one(&config);
}
```

### Rust test body
```rust
fn limited_reno() {
    let mut config = limited_config_default(1);
    config.ccalgo = get_congestion_algorithm("newreno").expect("newreno algo");
    config.max_completion_time = 4_600_000;
    limited_client_test_one(config);
}
```

## `picoquictest/datagram_tests.c:datagram_small_new_test`
* C test-table name: `datagram_small_new`
* C entry function: `datagram_small_new_test`
* Rust test: `datagram_small_new`
* C source: `picoquictest/datagram_tests.c:676-692`
* Rust source: `rs/fq/src/tests/datagram.rs:753-766`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_small_size = 64;
    dg_ctx.batch_size[0] = 4;
    dg_ctx.batch_size[1] = 4;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 5000;
    dg_ctx.next_gen_time[0] = 50000;
    dg_ctx.next_gen_time[1] = 50000;
    dg_ctx.max_packets_received = 55;
    dg_ctx.use_extended_provider_api = 1;

    return datagram_test_one(7, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_small_new() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        use_extended_provider_api: true,
        ..Default::default()
    };
    datagram_test_one(7, &mut dg_ctx, 0);
}
```

## `picoquictest/dualq_aqm_test.c:dualq_aqm_test`
* C test-table name: `dualq_aqm`
* C entry function: `dualq_aqm_test`
* Rust test: `dualq_aqm`
* C source: `picoquictest/dualq_aqm_test.c:415-436`
* Rust source: `rs/fq/src/tests/dualq_aqm.rs:312-318`

### C test body
```c
{
    int ret = dualq_test_ctx_test();

    if (ret == 0) {
        ret = dualq_enqueue_test();
    }

    if (ret == 0) {
        ret = dualq_dequeue_test();
    }

    if (ret == 0) {
        ret = dualq_submit_test();
    }

    if (ret == 0) {
        ret = dualq_sustain_test();
    }

    return ret;
}
```

### Rust test body
```rust
fn dualq_aqm() {
    dualq_ctx_test().expect("dualq_ctx_test");
    dualq_enqueue().expect("dualq_enqueue");
    dualq_dequeue().expect("dualq_dequeue");
    dualq_submit().expect("dualq_submit");
    dualq_sustain().expect("dualq_sustain");
}
```
