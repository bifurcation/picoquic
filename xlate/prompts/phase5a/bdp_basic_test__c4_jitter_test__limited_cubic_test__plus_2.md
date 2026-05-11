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

## `picoquictest/cpu_limited.c:limited_cubic_test`
* C test-table name: `limited_cubic`
* C entry function: `limited_cubic_test`
* Rust test: `limited_cubic`
* C source: `picoquictest/cpu_limited.c:220-228`
* Rust source: `rs/fq/src/tests/cpu_limited.rs:169-174`

### C test body
```c
{
    limited_test_config_t config;
    limited_config_set_default(&config, 2);
    config.ccalgo = picoquic_cubic_algorithm;
    config.max_completion_time = 4200000;

    return limited_client_test_one(&config);
}
```

### Rust test body
```rust
fn limited_cubic() {
    let mut config = limited_config_default(2);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic algo");
    config.max_completion_time = 4_200_000;
    limited_client_test_one(config);
}
```

## `picoquictest/datagram_tests.c:datagram_size_test`
* C test-table name: `datagram_size`
* C entry function: `datagram_size_test`
* Rust test: `datagram_size`
* C source: `picoquictest/datagram_tests.c:648-657`
* Rust source: `rs/fq/src/tests/datagram.rs:725-733`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 5000;

    return datagram_test_one(5, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_size() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_target: [100, 100],
        send_delay: 5_000,
        ..Default::default()
    };
    datagram_test_one(5, &mut dg_ctx, 0);
}
```

## `picoquictest/delay_tolerant_test.c:dtn_silence_test`
* C test-table name: `dtn_silence`
* C entry function: `dtn_silence_test`
* Rust test: `dtn_silence`
* C source: `picoquictest/delay_tolerant_test.c:216-226`
* Rust source: `rs/fq/src/tests/delay_tolerant.rs:202-208`

### C test body
```c
{
    /* Simple test. */
    dtn_test_spec_t spec;
    dtn_set_basic_test_spec(&spec);
    spec.scenario = dtn_scenario_silence;
    spec.sizeof_scenario = sizeof(dtn_scenario_silence);
    spec.max_number_of_packets = 120; /* Check that the number of packets does not increase wildly */
    spec.max_completion_time = 481000000; /* 8 minutes: 2 for handshake, plus 2 per transaction */
    return dtn_test_one(0x51, &spec);
}
```

### Rust test body
```rust
fn dtn_silence() {
    let mut spec = dtn_basic_spec();
    spec.scenario = DTN_SCENARIO_SILENCE;
    spec.max_number_of_packets = 120;
    spec.max_completion_time = 481_000_000; // 8 min: 2 handshake + 2 per tx
    dtn_test_one(0x51, &spec);
}
```
