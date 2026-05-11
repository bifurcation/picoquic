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

## `picoquictest/congestion_test.c:bdp_short_lo_test`
* C test-table name: `bdp_short_lo`
* C entry function: `bdp_short_lo_test`
* Rust test: `bdp_short_lo`
* C source: `picoquictest/congestion_test.c:712-715`
* Rust source: `rs/fq/src/tests/congestion.rs:936-938`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_short_lo);
}
```

### Rust test body
```rust
fn bdp_short_lo() {
    bdp_option_test_one(BdpTestOption::ShortLo);
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

## `picoquictest/datagram_tests.c:datagram_small_packet_test`
* C test-table name: `datagram_small_packet`
* C entry function: `datagram_small_packet_test`
* Rust test: `datagram_small_packet`
* C source: `picoquictest/datagram_tests.c:712-732`
* Rust source: `rs/fq/src/tests/datagram.rs:770-787`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_small_size = 64;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 20000;
    dg_ctx.send_delay = 100;
    dg_ctx.next_gen_time[0] = 50000;
    dg_ctx.next_gen_time[1] = 50000;
    dg_ctx.link_latency = 10000;
    dg_ctx.picosec_per_byte = 20000; /* 400 Mbps */
    dg_ctx.dg_latency_target[0] = 20000;
    dg_ctx.dg_latency_target[1] = 13500;
    dg_ctx.use_extended_provider_api = 1;
    dg_ctx.one_datagram_per_packet = 1;
    dg_ctx.nb_trials_max = 200000;
    dg_ctx.duration_max = 2060000;

    return datagram_test_one(9, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_small_packet() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        dg_target: [100, 20_000],
        send_delay: 100,
        next_gen_time: [50_000, 50_000],
        link_latency: 10_000,
        picosec_per_byte: 20_000, // 400 Mbps
        dg_latency_target: [20_000, 13_500],
        use_extended_provider_api: true,
        one_datagram_per_packet: true,
        nb_trials_max: 200_000,
        duration_max: 2_060_000,
        ..Default::default()
    };
    datagram_test_one(9, &mut dg_ctx, 0);
}
```

## `picoquictest/ech_test.c:ech_e2e_test`
* C test-table name: `ech_e2e`
* C entry function: `ech_e2e_test`
* Rust test: `ech_e2e`
* C source: `picoquictest/ech_test.c:442-447`
* Rust source: `rs/fq/src/tests/ech.rs:262-268`

### C test body
```c
{
    ech_e2e_spec_t spec = { 0 };
    spec.expect_success = 1;
    return ech_e2e_test_one(&spec);
}
```

### Rust test body
```rust
fn ech_e2e() {
    let spec = EchE2eSpec {
        expect_success: true,
        ..Default::default()
    };
    ech_e2e_test_one(&spec);
}
```

## `picoquictest/edge_cases.c:eca1_amplification_loss_test`
* C test-table name: `eca1_amplification_loss`
* C entry function: `eca1_amplification_loss_test`
* Rust test: `eca1_amplification_loss`
* C source: `picoquictest/edge_cases.c:418-439`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1145-1150`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t initial_losses = 0x0FF4;
    uint8_t test_case_id = 0xa1;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 0, &simulated_time, initial_losses, 16);

    if (ret == 0) {
        ret = edge_case_complete(test_ctx, &simulated_time, 15000000);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn eca1_amplification_loss() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0xa1, false, &mut simulated_time, 0x0FF4, 16).expect("edge_case_prepare");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 15_000_000).expect("edge_case_complete");
}
```
