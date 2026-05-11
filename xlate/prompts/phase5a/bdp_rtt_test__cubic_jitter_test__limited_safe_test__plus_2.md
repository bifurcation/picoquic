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

## `picoquictest/congestion_test.c:bdp_rtt_test`
* C test-table name: `bdp_rtt`
* C entry function: `bdp_rtt_test`
* Rust test: `bdp_rtt`
* C source: `picoquictest/congestion_test.c:674-685`
* Rust source: `rs/fq/src/tests/congestion.rs:906-908`

### C test body
```c
{
    /* TODO: this test succeeds for the wrong reason.
    * The goal of the test is to verify that the BDP is NOT set
    * if the RTT on the second connection does not match the RTT
    * on the first one. The test does that, but only because the
    * second connection's RTT is lower than BBRLongRttThreshold,
    * thus uses regular BBR startup, in which the BDP option is
    * not implemented.
     */
    return bdp_option_test_one(bdp_test_option_rtt);
}
```

### Rust test body
```rust
fn bdp_rtt() {
    bdp_option_test_one(BdpTestOption::Rtt);
}
```

## `picoquictest/congestion_test.c:cubic_jitter_test`
* C test-table name: `cubic_jitter`
* C entry function: `cubic_jitter_test`
* Rust test: `cubic_jitter`
* C source: `picoquictest/congestion_test.c:115-118`
* Rust source: `rs/fq/src/tests/congestion.rs:740-743`

### C test body
```c
{
    return congestion_control_test(picoquic_cubic_algorithm, 3550000, 5000, 5);
}
```

### Rust test body
```rust
fn cubic_jitter() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    congestion_control_test(ccalgo, 3_550_000, 5_000, 5);
}
```

## `picoquictest/cpu_limited.c:limited_safe_test`
* C test-table name: `limited_safe`
* C entry function: `limited_safe_test`
* Rust test: `limited_safe`
* C source: `picoquictest/cpu_limited.c:251-262`
* Rust source: `rs/fq/src/tests/cpu_limited.rs:197-205`

### C test body
```c
{
    limited_test_config_t config;
    limited_config_set_default(&config, 5);
    config.ccalgo = picoquic_cubic_algorithm;
    config.max_completion_time = 5450000;
    /* Bug. Should investigate later -- there should be 0 or maybe 1 losses */
    config.nb_losses_max = 6;
    config.flow_control_max = 57344;

    return limited_client_test_one(&config);
}
```

### Rust test body
```rust
fn limited_safe() {
    let mut config = limited_config_default(5);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic algo");
    config.max_completion_time = 5_450_000;
    // Bug noted in original C: there should be 0 or maybe 1 losses.
    config.nb_losses_max = 6;
    config.flow_control_max = 57_344;
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
