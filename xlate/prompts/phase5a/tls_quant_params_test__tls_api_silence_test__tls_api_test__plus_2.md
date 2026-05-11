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

## `picoquictest/tls_api_test.c:tls_quant_params_test`
* C test-table name: `quant_params`
* C entry function: `tls_quant_params_test`
* Rust test: `quant_params`
* C source: `picoquictest/tls_api_test.c:5550-5566`
* Rust source: `rs/fq/src/tests/tls_api.rs:1007-1011`

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

### Rust test body
```rust
fn quant_params() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_510_000).expect("quant_params");
}
```

## `picoquictest/tls_api_test.c:tls_api_silence_test`
* C test-table name: `silence_test`
* C entry function: `tls_api_silence_test`
* Rust test: `silence_test`
* C source: `picoquictest/tls_api_test.c:2441-2480`
* Rust source: `rs/fq/src/tests/tls_api.rs:1233-1235`

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    uint64_t next_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* simulate 5 seconds of silence */
    next_time = simulated_time + 5000000;
    while (ret == 0 && simulated_time < next_time && TEST_CLIENT_READY && TEST_SERVER_READY) {
        int was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, next_time, &was_active);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        /* verify the absence of any spurious retransmission */
        if (test_ctx->cnx_client->nb_retransmission_total != 0) {
            ret = -1;
        } else if (test_ctx->cnx_server != NULL && test_ctx->cnx_server->nb_retransmission_total != 0) {
            ret = -1;
        }
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
fn silence_test() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("silence_test");
}
```

## `picoquictest/tls_api_test.c:tls_api_test`
* C test-table name: `tls_api`
* C entry function: `tls_api_test`
* Rust test: `tls_api`
* C source: `picoquictest/tls_api_test.c:2283-2286`
* Rust source: `rs/fq/src/tests/tls_api.rs:1321-1323`

### C test body
```c
{
    return tls_api_test_with_loss(NULL, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN);
}
```

### Rust test body
```rust
fn tls_api() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_api");
}
```

## `picoquictest/tls_api_test.c:tls_api_very_long_congestion_test`
* C test-table name: `tls_api_very_long_congestion`
* C entry function: `tls_api_very_long_congestion_test`
* Rust test: `tls_api_very_long_congestion`
* C source: `picoquictest/tls_api_test.c:3296-3299`
* Rust source: `rs/fq/src/tests/tls_api.rs:1403-1408`

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 128000, 20000, 0, 1000000, NULL, NULL);
}
```

### Rust test body
```rust
fn tls_api_very_long_congestion() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 20_000, 1_000_000)
        .expect("very_long_congestion");
}
```

## `picoquictest/tls_api_test.c:unidir_test`
* C test-table name: `unidir`
* C entry function: `unidir_test`
* Rust test: `unidir`
* C source: `picoquictest/tls_api_test.c:3301-3334`
* Rust source: `rs/fq/src/tests/tls_api.rs:1487-1491`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;

    int ret = tls_api_one_scenario_init(&test_ctx, &simulated_time,
        0, NULL, NULL);

    if (ret == 0) {
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_unidir, sizeof(test_scenario_unidir), 0, 0, 128000, 10000,
            100000);
    }

    /* Verify that the unidir streams are properly closed. */
    if (ret == 0 && test_ctx->cnx_client != NULL && test_ctx->cnx_client->stream_tree.size != 0) {
        DBG_PRINTF("There are %d streams left open on client at the end of test.",
            test_ctx->cnx_client->stream_tree.size);
        ret = -1;
    }

    if (ret == 0 && test_ctx->cnx_server != NULL && test_ctx->cnx_server->stream_tree.size != 0) {
        DBG_PRINTF("There are %d streams left open on client at the end of test.",
            test_ctx->cnx_server->stream_tree.size);
        ret = -1;
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
fn unidir() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 2_000_000).expect("unidir");
}
```
