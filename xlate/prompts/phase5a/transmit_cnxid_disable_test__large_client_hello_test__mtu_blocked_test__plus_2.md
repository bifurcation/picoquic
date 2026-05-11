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

## `picoquictest/tls_api_test.c:transmit_cnxid_disable_test`
* C test-table name: `cnxid_transmit_disable`
* C entry function: `transmit_cnxid_disable_test`
* Rust test: `cnxid_transmit_disable`
* C source: `picoquictest/tls_api_test.c:6616-6619`
* Rust source: `rs/fq/src/tests/tls_api.rs:213-215`

### C test body
```c
{
    return transmit_cnxid_test_one(0, 1, 0);
}
```

### Rust test body
```rust
fn cnxid_transmit_disable() {
    transmit_cnxid_test_one(false, true, false).expect("cnxid_transmit_disable");
}
```

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

## `picoquictest/tls_api_test.c:mtu_blocked_test`
* C test-table name: `mtu_blocked`
* C entry function: `mtu_blocked_test`
* Rust test: `mtu_blocked`
* C source: `picoquictest/tls_api_test.c:4981-4986`
* Rust source: `rs/fq/src/tests/tls_api.rs:625-627`

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_blocked, 1252, 1252, 
        test_scenario_mtu_discovery, sizeof(test_scenario_mtu_discovery), 0);
    return ret;
}
```

### Rust test body
```rust
fn mtu_blocked() {
    mtu_discovery_test_one(1, 1252, 1252, 10_000_000, 0).expect("mtu_blocked");
}
```

## `picoquictest/tls_api_test.c:mtu_required_test`
* C test-table name: `mtu_required`
* C entry function: `mtu_required_test`
* Rust test: `mtu_required`
* C source: `picoquictest/tls_api_test.c:4995-5000`
* Rust source: `rs/fq/src/tests/tls_api.rs:699-701`

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_required, 1440, 1440, 
        test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0);
    return ret;
}
```

### Rust test body
```rust
fn mtu_required() {
    mtu_discovery_test_one(3, 1440, 1440, 2_500_000, 0).expect("mtu_required");
}
```

## `picoquictest/tls_api_test.c:nat_rebinding_zero_test`
* C test-table name: `nat_rebinding_zero`
* C entry function: `nat_rebinding_zero_test`
* Rust test: `nat_rebinding_zero`
* C source: `picoquictest/tls_api_test.c:6086-6092`
* Rust source: `rs/fq/src/tests/tls_api.rs:780-782`

### C test body
```c
{
    /* Test of NAT rebinding with zero-length client CID */
    uint64_t loss_mask = 0;

    return nat_rebinding_test_one(loss_mask, 1, 0);
}
```

### Rust test body
```rust
fn nat_rebinding_zero() {
    nat_rebinding_test_one(0, true, 0).expect("nat_rebinding_zero");
}
```
