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

## `picoquictest/cpu_limited.c:limited_batch_test`
* C test-table name: `limited_batch`
* C entry function: `limited_batch_test`
* Rust test: `limited_batch`
* C source: `picoquictest/cpu_limited.c:240-249`
* Rust source: `rs/fq/src/tests/cpu_limited.rs:187-193`

### C test body
```c
{
    limited_test_config_t config;
    limited_config_set_default(&config, 4);
    config.ccalgo = picoquic_bbr_algorithm;
    config.max_completion_time = 6200000;
    config.nb_initial_steps = 10;

    return limited_client_test_one(&config);
}
```

### Rust test body
```rust
fn limited_batch() {
    let mut config = limited_config_default(4);
    config.ccalgo = get_congestion_algorithm("bbr").expect("bbr algo");
    config.max_completion_time = 6_200_000;
    config.nb_initial_steps = 10;
    limited_client_test_one(config);
}
```

## `picoquictest/datagram_tests.c:datagram_rtnew_skip_test`
* C test-table name: `datagram_rtnew_skip`
* C entry function: `datagram_rtnew_skip_test`
* Rust test: `datagram_rtnew_skip`
* C source: `picoquictest/datagram_tests.c:616-632`
* Rust source: `rs/fq/src/tests/datagram.rs:696-708`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 10;
    dg_ctx.send_delay = 20000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;
    dg_ctx.dg_latency_target[0] = 13000;
    dg_ctx.dg_latency_target[1] = 20000;
    dg_ctx.do_skip_test[0] = 1;
    dg_ctx.do_skip_test[1] = 1;
    dg_ctx.use_extended_provider_api = 1;

    return datagram_test_one(7, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_rtnew_skip() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 10],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [13_000, 20_000],
        do_skip_test: [true, true],
        use_extended_provider_api: true,
        ..Default::default()
    };
    datagram_test_one(7, &mut dg_ctx, 0);
}
```

## `picoquictest/delay_tolerant_test.c:dtn_data_test`
* C test-table name: `dtn_data`
* C entry function: `dtn_data_test`
* Rust test: `dtn_data`
* C source: `picoquictest/delay_tolerant_test.c:197-207`
* Rust source: `rs/fq/src/tests/delay_tolerant.rs:192-198`

### C test body
```c
{
    /* Simple test. */
    dtn_test_spec_t spec;
    dtn_set_basic_test_spec(&spec);
    spec.scenario = dtn_scenario_data;
    spec.sizeof_scenario = sizeof(dtn_scenario_data);
    spec.initial_flow_control_credit = 100000000; /* 100 MB, same as data size in scenario */
    spec.max_completion_time = 500000000; /* 8 minutes and 20 sec, including 2 minutes handshae, 2 minutes req/resp, 2 minutes chirp... */
    return dtn_test_one(0xda, &spec);
}
```

### Rust test body
```rust
fn dtn_data() {
    let mut spec = dtn_basic_spec();
    spec.scenario = DTN_SCENARIO_DATA;
    spec.initial_flow_control_credit = 100_000_000; // 100 MB
    spec.max_completion_time = 500_000_000; // ~8 min 20 sec
    dtn_test_one(0xda, &spec);
}
```

## `picoquictest/ech_test.c:ech_grease_test`
* C test-table name: `ech_grease`
* C entry function: `ech_grease_test`
* Rust test: `ech_grease`
* C source: `picoquictest/ech_test.c:458-463`
* Rust source: `rs/fq/src/tests/ech.rs:286-292`

### C test body
```c
{
    ech_e2e_spec_t spec = { 0 };
    spec.expect_grease = 1;
    return ech_e2e_test_one(&spec);
}
```

### Rust test body
```rust
fn ech_grease() {
    let spec = EchE2eSpec {
        expect_grease: true,
        ..Default::default()
    };
    ech_e2e_test_one(&spec);
}
```
