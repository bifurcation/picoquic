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

## `picoquictest/app_limited.c:app_limited_reno_test`
* C test-table name: `app_limited_reno`
* C entry function: `app_limited_reno_test`
* Rust test: `app_limited_reno`
* C source: `picoquictest/app_limited.c:580-587`
* Rust source: `rs/fq/src/tests/app_limited.rs:521-525`

### C test body
```c
{
    app_limited_test_config_t config;
    app_limited_config_set_default(&config, 1);
    config.ccalgo = picoquic_newreno_algorithm;

    return app_limited_test_one(&config);
}
```

### Rust test body
```rust
fn app_limited_reno() {
    let mut config = AppLimitedConfig::default_config(1);
    config.ccalgo = get_congestion_algorithm("newreno").expect("newreno cc algo");
    app_limited_test_one(config);
}
```

## `picoquictest/congestion_test.c:bbr_asym100_nodelay_test`
* C test-table name: `bbr_asym100_nodelay`
* C entry function: `bbr_asym100_nodelay_test`
* Rust test: `bbr_asym100_nodelay`
* C source: `picoquictest/congestion_test.c:408-429`
* Rust source: `rs/fq/src/tests/congestion.rs:857-862`

### C test body
```c
{
    uint64_t max_completion_time = 8500000;
    uint64_t latency = 1000;
    uint64_t jitter = 750;
    uint64_t buffer = 50000;
    uint64_t mbps = 10;
    uint64_t kbps = 100;
    picoquic_tp_t server_parameters;

    memset(&server_parameters, 0, sizeof(picoquic_tp_t));
    picoquic_init_transport_parameters(&server_parameters);
    server_parameters.min_ack_delay = 0;

    int ret = performance_test_one(max_completion_time, mbps, kbps, latency, jitter, buffer,
        &server_parameters);

    return ret;
}
```

### Rust test body
```rust
fn bbr_asym100_nodelay() {
    let mut server_params = TransportParameters::default();
    init_transport_parameters(&mut server_params);
    server_params.min_ack_delay = Duration::from_ticks(0);
    performance_test_one(8_500_000, 10, 100, 1_000, 750, 50_000, Some(&server_params));
}
```

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

## `picoquictest/congestion_test.c:blackhole_test`
* C test-table name: `blackhole`
* C entry function: `blackhole_test`
* Rust test: `blackhole`
* C source: `picoquictest/congestion_test.c:787-792`
* Rust source: `rs/fq/src/tests/congestion.rs:726-729`

### C test body
```c
{
    int ret = blackhole_test_one(picoquic_bbr_algorithm, 15000000, 0);

    return ret;
}
```

### Rust test body
```rust
fn blackhole() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    blackhole_test_one(ccalgo, 15_000_000, 0);
}
```

## `picoquictest/congestion_test.c:fastcc_jitter_test`
* C test-table name: `fastcc_jitter`
* C entry function: `fastcc_jitter_test`
* Rust test: `fastcc_jitter`
* C source: `picoquictest/congestion_test.c:135-138`
* Rust source: `rs/fq/src/tests/congestion.rs:768-771`

### C test body
```c
{
    return congestion_control_test(picoquic_fastcc_algorithm, 4050000, 5000, 5);
}
```

### Rust test body
```rust
fn fastcc_jitter() {
    let ccalgo = get_congestion_algorithm("fastcc").expect("fastcc cc algo");
    congestion_control_test(ccalgo, 4_050_000, 5_000, 5);
}
```
