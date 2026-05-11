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

## `picoquictest/satellite_test.c:satellite_seeded_test`
* C test-table name: `satellite_seeded`
* C entry function: `satellite_seeded_test`
* Rust test: `satellite_seeded`
* C source: `picoquictest/satellite_test.c:218-222`
* Rust source: `rs/fq/src/tests/satellite.rs:209-224`

### C test body
```c
{
    /* Simulate remembering RTT and BW from previous connection */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 4900000, 250, 3, 0, 0, 0, 1, 0, 0);
}
```

### Rust test body
```rust
fn satellite_seeded() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        4_900_000,
        250,
        3,
        0,
        false,
        false,
        true,
        false,
        false,
    );
}
```

## `picoquictest/tls_api_test.c:af_undef_test`
* C test-table name: `af_undef`
* C entry function: `af_undef_test`
* Rust test: `af_undef`
* C source: `picoquictest/tls_api_test.c:13012-13055`
* Rust source: `rs/fq/src/tests/tls_api.rs:38-40`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xaf, 0x0d, 0xef, 0, 0, 0, 0, 0}, 8 };
    uint64_t target_time = 1000000;
    int ret;

    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0) {
        test_ctx->client_endpoint.addr_to_unspec = 1;
        picoquic_set_qlog(test_ctx->qserver, ".");
        test_ctx->qserver->use_long_log = 1;
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    /* Try to complete the data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, target_time);
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
fn af_undef() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("af_undef");
}
```

## `picoquictest/tls_api_test.c:transmit_cnxid_retire_disable_test`
* C test-table name: `cnxid_transmit_r_disable`
* C entry function: `transmit_cnxid_retire_disable_test`
* Rust test: `cnxid_transmit_r_disable`
* C source: `picoquictest/tls_api_test.c:6626-6629`
* Rust source: `rs/fq/src/tests/tls_api.rs:229-231`

### C test body
```c
{
    return transmit_cnxid_test_one(1, 1, 0);
}
```

### Rust test body
```rust
fn cnxid_transmit_r_disable() {
    transmit_cnxid_test_one(true, true, false).expect("cnxid_transmit_r_disable");
}
```

## `picoquictest/tls_api_test.c:false_migration_test`
* C test-table name: `false_migration`
* C entry function: `false_migration_test`
* Rust test: `false_migration`
* C source: `picoquictest/tls_api_test.c:8327-8345`
* Rust source: `rs/fq/src/tests/tls_api.rs:335-337`

### C test body
```c
{
    int ret = 0;
    int target_client;

    for (target_client = 1; ret == 0 && target_client >= 0; target_client--) {
        ret = false_migration_test_scenario(test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2), target_client, picoquic_packet_context_initial, 0);
        
        if (ret == 0) {
            ret = false_migration_test_scenario(test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2), target_client, picoquic_packet_context_handshake, 0);
        }

        for (uint64_t seq = 0; ret == 0 && seq < 4; seq++) {
            ret = false_migration_test_scenario(test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2), target_client, picoquic_packet_context_application, seq);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn false_migration() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("false_migration");
}
```

## `picoquictest/tls_api_test.c:heavy_loss_total_test`
* C test-table name: `heavy_loss_total`
* C entry function: `heavy_loss_total_test`
* Rust test: `heavy_loss_total`
* C source: `picoquictest/tls_api_test.c:11369-11372`
* Rust source: `rs/fq/src/tests/tls_api.rs:400-402`

### C test body
```c
{
    return heavy_loss_test_one(2, 25000000);
}
```

### Rust test body
```rust
fn heavy_loss_total() {
    heavy_loss_test_one(2, 25_000_000).expect("heavy_loss_total");
}
```
