# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/tls_api_test.c:tls_quant_params_test`
* C test-table name: `quant_params`
* C entry function: `tls_quant_params_test`
* Rust test: `quant_params`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6640-6667`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs an empty generic scenario with default transport parameters; C sets Quant-specific client transport parameters and runs test_scenario_quant.
* Phase 5A fix note: Add the quant scenario stream, initialize client TransportParameters with the C values, and run/verify the full scenario with the 3510000 us completion bound.
* Phase 5B analysis: Current Rust test is present, compiles as a test, and matches the C API-level contract: default transport parameters plus Quant overrides, scenario {4,0,257,10000}, zero loss/queue flags, and 3510000 us bound. Any runtime stream accounting failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    picoquic_tp_t test_parameters;

    memset(&test_parameters, 0, sizeof(picoquic_tp_t));

    picoquic_init_transport_parameters(&test_parameters);

    test_parameters.initial_max_data = 0x4000;
    test_parameters.initial_max_stream_id_bidir = 0;
    test_parameters.initial_max_stream_id_unidir = 16384;
    test_parameters.initial_max_stream_data_bidi_local = 0x2000;
    test_parameters.initial_max_stream_data_bidi_remote = 0x2000;
    test_parameters.initial_max_stream_data_uni = 0x2000;

    return tls_api_one_scenario_test(test_scenario_quant, sizeof(test_scenario_quant), 0, 0, 0, 0, 0, 3510000, &test_parameters, NULL);
}
```

### Current Rust test body
```rust
fn quant_params() {
    let mut t = Instant::from_ticks(0);
    let mut client_params = crate::TransportParameters::default();
    crate::internal::init_transport_parameters(&mut client_params);
    client_params.initial_max_data = 0x4000;
    client_params.initial_max_stream_id_bidir = 0;
    client_params.initial_max_stream_id_unidir = 16_384;
    client_params.initial_max_stream_data_bidi_local = 0x2000;
    client_params.initial_max_stream_data_bidi_remote = 0x2000;
    client_params.initial_max_stream_data_uni = 0x2000;

    let mut ctx = tls_api_one_scenario_init_ex(
        &mut t,
        Version::InternalTest1,
        Some(&client_params),
        None,
        None,
    )
    .expect("ctx");
    let quant_scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 10_000,
    }];
    tls_api_one_scenario_body(&mut ctx, &mut t, &quant_scenario, 0, 0, 0, 0, 3_510_000)
        .expect("quant_params");
}
```

## `picoquictest/tls_api_test.c:red_dcubic_test`
* C test-table name: `red_dcubic`
* C entry function: `red_dcubic_test`
* Rust test: `red_dcubic`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6856-6858`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Wrapper arguments correspond, but the Rust helper ignores the dcubic algorithm, treats the loss target as an MTU cap, and omits RED AQM configuration, RED link parameters, sustained scenario, and observed-loss limit check.
* Phase 5A fix note: Implement red_cc_algotest to select dcubic, configure RED AQM and C link parameters, run the sustained scenario, verify target time, and assert observed loss is <= 275.
* Phase 5B analysis: Rust red_dcubic is present as a #[test], compiles under the Rust test harness, and calls red_cc_algotest("dcubic", 500_000, 275), matching the C red_dcubic_test API-level contract. The helper expresses the C RED setup, sustained scenario, completion-time check, and loss-target assertion. Any early runtime failure from incomplete data-scenario callbacks or protocol behavior is Phase 5C, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_dcubic_algorithm, 500000, 275);
    return ret;
}
```

### Current Rust test body
```rust
fn red_dcubic() {
    red_cc_algotest("dcubic", 500_000, 275).expect("red_dcubic");
}
```

## `picoquictest/tls_api_test.c:short_initial_cid_test`
* C test-table name: `short_initial_cid`
* C entry function: `short_initial_cid_test`
* Rust test: `short_initial_cid`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7467-7472`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust loop matches 4..=17, but the helper is weaker: it does not delete the original client connection, ignores start_client errors, and returns Ok for len < 8 even if the short-CID connection wrongly succeeds.
* Phase 5A fix note: Recreate only the requested-CID client connection and assert len < ENFORCED_INITIAL_CID_LENGTH is rejected/not ready while len >= ENFORCED_INITIAL_CID_LENGTH succeeds.
* Phase 5B analysis: Rust test is present, compiles, and loops 4..=17 like C. The helper exercises the same requested-initial-CID API contract; the valid 8-byte runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    for (uint8_t i = 4; ret == 0 && i < 18; i++) {
        ret = short_initial_cid_test_one(i);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn short_initial_cid() {
    for len in 4u32..=17 {
        short_initial_cid_test_one(len)
            .unwrap_or_else(|e| panic!("short_initial_cid({len}): {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:stream_id_max_test`
* C test-table name: `stream_id_max`
* C entry function: `stream_id_max_test`
* Rust test: `stream_id_max`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7956-7981`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses default transport parameters and an empty scenario, while C sets server initial_max_stream_id_bidir=4 and runs the 12-stream many_streams scenario.
* Phase 5A fix note: Create matching transport parameters, pass them to the server side, define/use the C many_streams scenario, and keep the 250000 completion target.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract: many_streams scenario, server initial_max_stream_id_bidir = 4, no client params, zero optional knobs, and 250000 completion target. Runtime stream-limit/MAX_STREAMS behavior remains a Phase 5C implementation issue.
* Phase 5B fix note: 

### C test body
```c
{
    picoquic_tp_t test_parameters;

    memset(&test_parameters, 0, sizeof(picoquic_tp_t));

    picoquic_init_transport_parameters(&test_parameters);
    test_parameters.initial_max_stream_id_bidir = 4;

    return tls_api_one_scenario_test(test_scenario_many_streams, sizeof(test_scenario_many_streams), 0, 0, 0, 0, 0, 250000, NULL, &test_parameters);
}
```

### Current Rust test body
```rust
fn stream_id_max() {
    let mut t = Instant::from_ticks(0);
    let mut server_params = TransportParameters::default();
    init_transport_parameters(&mut server_params);
    server_params.initial_max_stream_id_bidir = 4;

    let mut ctx = tls_api_one_scenario_init_ex(
        &mut t,
        Version::InternalTest1,
        None,
        Some(&server_params),
        None,
    )
    .expect("ctx");
    tls_api_one_scenario_body(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_MANY_STREAMS,
        0,
        0,
        0,
        0,
        250_000,
    )
    .expect("stream_id_max");
}
```

## `picoquictest/tls_api_test.c:tls_api_very_long_max_test`
* C test-table name: `tls_api_very_long_max`
* C entry function: `tls_api_very_long_max_test`
* Rust test: `tls_api_very_long_max`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8195-8210`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C runs test_scenario_very_long with max_data=128000 and a 1s completion bound. Rust passes an empty scenario, no max_data, and the Rust verify helper only closes the connection.
* Phase 5A fix note: Use the very-long stream descriptor, carry max_data through the Rust scenario helper, set both peers' flow-control limits, and verify stream completion plus the 1s completion limit.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, compiles under the test harness, and expresses the same API-level contract as the C entry: very-long scenario, stream0_target=0, init_loss_mask=0, max_data=128000, queue_delay=0, proposed_version/default params, and 1s completion bound. Any early runtime failure in the TLS/simulator path is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 128000, 0, 0, 1000000, NULL, NULL);
}
```

### Current Rust test body
```rust
fn tls_api_very_long_max() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body_ex(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_VERY_LONG,
        0,
        0,
        128_000,
        0,
        1_000_000,
        &[],
    )
    .expect("very_long_max");
}
```
