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

## `picoquictest/tls_api_test.c:large_client_hello_test`
* C test-table name: `large_client_hello`
* C entry function: `large_client_hello_test`
* Rust test: `large_client_hello`
* C source: `picoquictest/tls_api_test.c:10180-10224`
* Rust source: `rs/fq/src/tests/tls_api.rs:524-526`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Set the test large hello flag in the client connection
     */
    if (ret == 0) {
        test_ctx->cnx_client->test_large_chello = 1;

        /* Run a basic test scenario
         */

        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 250000);
    }

    /* Verify that there is no spurious retransmission */
    if (ret == 0) {
        if (test_ctx->cnx_server == NULL || test_ctx->cnx_server->nb_retransmission_total > 0) {
            DBG_PRINTF("Unexpected, server retransmitted %" PRIu64 " packets", 
                (test_ctx->cnx_server == NULL)? UINT64_MAX:test_ctx->cnx_server->nb_retransmission_total);
            ret = -1;
        }
        else if (test_ctx->cnx_client->nb_retransmission_total > 0) {
            DBG_PRINTF("Unexpected, client retransmitted %" PRIu64 " packets", test_ctx->cnx_client->nb_retransmission_total);
            ret = -1;
        }
    }

    /* And then free the resource
     */

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn large_client_hello() {
    tls_api_retry_test_one(true).expect("large_client_hello");
}
```

## `picoquictest/tls_api_test.c:mtu_delayed_test`
* C test-table name: `mtu_delayed`
* C entry function: `mtu_delayed_test`
* Rust test: `mtu_delayed`
* C source: `picoquictest/tls_api_test.c:4988-4993`
* Rust source: `rs/fq/src/tests/tls_api.rs:634-636`

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_delayed, 1252, 1440, 
        test_scenario_very_long, sizeof(test_scenario_very_long), 0);
    return ret;
}
```

### Rust test body
```rust
fn mtu_delayed() {
    mtu_discovery_test_one(2, 1252, 1440, 2_500_000, 0).expect("mtu_delayed");
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

## `picoquictest/tls_api_test.c:no_ack_frequency_test`
* C test-table name: `no_ack_frequency`
* C entry function: `no_ack_frequency_test`
* Rust test: `no_ack_frequency`
* C source: `picoquictest/tls_api_test.c:10428-10450`
* Rust source: `rs/fq/src/tests/tls_api.rs:798-800`

### C test body
```c
{
    int ret = 0;
    picoquic_tp_t client_parameters;
    picoquic_tp_t server_parameters;

    for (int i = 1; ret == 0 && i <= 3; i++) {
        memset(&client_parameters, 0, sizeof(picoquic_tp_t));
        memset(&server_parameters, 0, sizeof(picoquic_tp_t));
        picoquic_init_transport_parameters(&client_parameters);
        picoquic_init_transport_parameters(&server_parameters);

        client_parameters.min_ack_delay = (i & 1) ? 0 : 1000;
        server_parameters.enable_loss_bit = (1 - ((i > 1) & 1));

        ret = tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 128, 0, 0, 0, 2000000, &client_parameters, &server_parameters);
        if (ret != 0) {
            DBG_PRINTF("No min ack delay test fails for client: %d, server: %d, ret = %d", i & 1, i >> 1, ret);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn no_ack_frequency() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("no_ack_frequency");
}
```

## `picoquictest/tls_api_test.c:padding_test`
* C test-table name: `padding_test`
* C entry function: `padding_test`
* Rust test: `padding_test`
* C source: `picoquictest/tls_api_test.c:8731-8734`
* Rust source: `rs/fq/src/tests/tls_api.rs:868-870`

### C test body
```c
{
    return padding_test_one(128, 64);
}
```

### Rust test body
```rust
fn padding_test() {
    padding_test_one(128, 64).expect("padding_test");
}
```
