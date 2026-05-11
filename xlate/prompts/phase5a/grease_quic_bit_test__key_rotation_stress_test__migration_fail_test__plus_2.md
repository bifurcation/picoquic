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

## `picoquictest/tls_api_test.c:grease_quic_bit_test`
* C test-table name: `grease_quic_bit`
* C entry function: `grease_quic_bit_test`
* Rust test: `grease_quic_bit`
* C source: `picoquictest/tls_api_test.c:11027-11030`
* Rust source: `rs/fq/src/tests/tls_api.rs:367-369`

### C test body
```c
{
    return  grease_quic_bit_test_one(0);
}
```

### Rust test body
```rust
fn grease_quic_bit() {
    grease_quic_bit_test_one(false).expect("grease_quic_bit");
}
```

## `picoquictest/tls_api_test.c:key_rotation_stress_test`
* C test-table name: `key_rotation_stress`
* C entry function: `key_rotation_stress_test`
* Rust test: `key_rotation_stress`
* C source: `picoquictest/tls_api_test.c:8099-8102`
* Rust source: `rs/fq/src/tests/tls_api.rs:506-508`

### C test body
```c
{
    return key_rotation_stress_test_one(10);
}
```

### Rust test body
```rust
fn key_rotation_stress() {
    key_rotation_stress_test_one(10).expect("key_rotation_stress");
}
```

## `picoquictest/tls_api_test.c:migration_fail_test`
* C test-table name: `migration_fail`
* C entry function: `migration_fail_test`
* Rust test: `migration_fail`
* C source: `picoquictest/tls_api_test.c:6875-6935`
* Rust source: `rs/fq/src/tests/tls_api.rs:592-594`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    /* establish the connection*/
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));

        if (ret != 0)
        {
            DBG_PRINTF("Init send receive scenario returns %d\n", ret);
        }
    }

    /* Perform a loop until the connection is in ready state */
    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    /* Start migration to bogus address */
    if (ret == 0) {
        struct sockaddr_in bogus_addr = test_ctx->client_addr;
        bogus_addr.sin_port += 1;

        ret = picoquic_probe_new_path(test_ctx->cnx_client,
            (struct sockaddr*) & test_ctx->server_addr, (struct sockaddr*) & bogus_addr, simulated_time);
        if (ret != 0) {
            DBG_PRINTF("Probe new path returns %d\n", ret);
        }
    }

    /* Perform a data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);

        if (ret != 0)
        {
            DBG_PRINTF("Data sending loop returns %d\n", ret);
        }
    }

    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, 1100000);
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
fn migration_fail() {
    migration_test_scenario(&[], 0, false).expect("migration_fail");
}
```

## `picoquictest/tls_api_test.c:mtu_drop_cubic_test`
* C test-table name: `mtu_drop_cubic`
* C entry function: `mtu_drop_cubic_test`
* Rust test: `mtu_drop_cubic`
* C source: `picoquictest/tls_api_test.c:5099-5103`
* Rust source: `rs/fq/src/tests/tls_api.rs:658-660`

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_cubic_algorithm, 10000000);
    return ret;
}
```

### Rust test body
```rust
fn mtu_drop_cubic() {
    mtu_drop_cc_algotest("cubic", 10_000_000).expect("mtu_drop_cubic");
}
```

## `picoquictest/tls_api_test.c:nat_handshake_test`
* C test-table name: `nat_handshake`
* C entry function: `nat_handshake_test`
* Rust test: `nat_handshake`
* C source: `picoquictest/tls_api_test.c:8447-8456`
* Rust source: `rs/fq/src/tests/tls_api.rs:731-733`

### C test body
```c
{
    int ret = 0;

    for (int test_rank = 0; ret == 0 && test_rank < 2; test_rank++) {
        ret = nat_handshake_test_one(test_rank);
    }

    return ret;
}
```

### Rust test body
```rust
fn nat_handshake() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("nat_handshake");
}
```
