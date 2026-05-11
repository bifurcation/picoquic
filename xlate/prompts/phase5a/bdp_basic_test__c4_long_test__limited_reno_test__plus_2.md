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

## `picoquictest/congestion_test.c:bdp_basic_test`
* C test-table name: `bdp_basic`
* C entry function: `bdp_basic_test`
* Rust test: `bdp_basic`
* C source: `picoquictest/congestion_test.c:669-672`
* Rust source: `rs/fq/src/tests/congestion.rs:888-890`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_basic);
}
```

### Rust test body
```rust
fn bdp_basic() {
    bdp_option_test_one(BdpTestOption::Basic);
}
```

## `picoquictest/congestion_test.c:c4_long_test`
* C test-table name: `c4_long`
* C entry function: `c4_long_test`
* Rust test: `c4_long`
* C source: `picoquictest/congestion_test.c:241-244`
* Rust source: `rs/fq/src/tests/congestion.rs:796-799`

### C test body
```c
{
    return congestion_long_test(c4_algorithm);
}
```

### Rust test body
```rust
fn c4_long() {
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    congestion_long_test(ccalgo);
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

## `picoquictest/datagram_tests.c:datagram_small_test`
* C test-table name: `datagram_small`
* C entry function: `datagram_small_test`
* Rust test: `datagram_small`
* C source: `picoquictest/datagram_tests.c:659-674`
* Rust source: `rs/fq/src/tests/datagram.rs:737-749`

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

    return datagram_test_one(6, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_small() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        ..Default::default()
    };
    datagram_test_one(6, &mut dg_ctx, 0);
}
```

## `picoquictest/delay_tolerant_test.c:dtn_twenty_test`
* C test-table name: `dtn_twenty`
* C entry function: `dtn_twenty_test`
* Rust test: `dtn_twenty`
* C source: `picoquictest/delay_tolerant_test.c:228-239`
* Rust source: `rs/fq/src/tests/delay_tolerant.rs:212-218`

### C test body
```c
{
    /* Simple test. */
    dtn_test_spec_t spec;
    dtn_set_basic_test_spec(&spec);
    spec.latency = 20 * 60000000;
    spec.max_completion_time = 8* spec.latency;

    spec.max_number_of_packets = 190;

    return dtn_test_one(0x20, &spec);
}
```

### Rust test body
```rust
fn dtn_twenty() {
    let mut spec = dtn_basic_spec();
    spec.latency = 20 * 60_000_000; // 20 minutes in µs
    spec.max_completion_time = 8 * spec.latency;
    spec.max_number_of_packets = 190;
    dtn_test_one(0x20, &spec);
}
```
