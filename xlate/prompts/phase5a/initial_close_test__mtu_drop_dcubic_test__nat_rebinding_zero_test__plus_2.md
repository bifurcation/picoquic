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

## `picoquictest/tls_api_test.c:initial_close_test`
* C test-table name: `initial_close`
* C entry function: `initial_close_test`
* Rust test: `initial_close`
* C source: `picoquictest/tls_api_test.c:7480-7537`
* Rust source: `rs/fq/src/tests/tls_api.rs:437-439`

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    int was_active = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        /* Send the initial packet, but no more than that */
        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

        if (ret == 0) {
            test_ctx->cnx_client->cnx_state = picoquic_state_handshake_failure;
            test_ctx->cnx_client->local_error = 0xDEAD;
            picoquic_reinsert_by_wake_time(test_ctx->qclient, test_ctx->cnx_client, simulated_time);
        }
    }

    if (ret == 0) {
        for (int i = 0; i < 128; i++) {
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            if (test_ctx->cnx_server != NULL) {
                break;
            }
        }
        if (ret == 0) {
            ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
        }

        if (ret == 0) {
            if (test_ctx->cnx_server == NULL) {
                DBG_PRINTF("%s", "Server connection deleted, cannot verify error code.\n");
                ret = -1;
            }
            else if (test_ctx->cnx_server->cnx_state != picoquic_state_disconnected ||
                test_ctx->cnx_server->remote_error != 0xDEAD) {
                DBG_PRINTF("Server state: %d, remote error: %x\n", test_ctx->cnx_server->cnx_state, test_ctx->cnx_server->remote_error);
                ret = -1;
            }
            else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
                DBG_PRINTF("Client state: %d, local error: %x", test_ctx->cnx_client->cnx_state, test_ctx->cnx_client->local_error);
                ret = -1;
            }
            else if (simulated_time > 50000ull) {
                DBG_PRINTF("Simulated time: %llu", (unsigned long long)simulated_time);
                ret = -1;
            }
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
fn initial_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("initial_close");
}
```

## `picoquictest/tls_api_test.c:mtu_drop_dcubic_test`
* C test-table name: `mtu_drop_dcubic`
* C entry function: `mtu_drop_dcubic_test`
* Rust test: `mtu_drop_dcubic`
* C source: `picoquictest/tls_api_test.c:5105-5109`
* Rust source: `rs/fq/src/tests/tls_api.rs:666-668`

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_dcubic_algorithm, 9200000);
    return ret;
}
```

### Rust test body
```rust
fn mtu_drop_dcubic() {
    mtu_drop_cc_algotest("dcubic", 9_200_000).expect("mtu_drop_dcubic");
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

## `picoquictest/tls_api_test.c:padding_null_test`
* C test-table name: `padding_null`
* C entry function: `padding_null_test`
* Rust test: `padding_null`
* C source: `picoquictest/tls_api_test.c:8736-8739`
* Rust source: `rs/fq/src/tests/tls_api.rs:860-862`

### C test body
```c
{
    return padding_test_one(0, 0);
}
```

### Rust test body
```rust
fn padding_null() {
    padding_test_one(0, 0).expect("padding_null");
}
```

## `picoquictest/tls_api_test.c:preferred_address_dis_mig_test`
* C test-table name: `preferred_address_dis_mig`
* C entry function: `preferred_address_dis_mig_test`
* Rust test: `preferred_address_dis_mig`
* C source: `picoquictest/tls_api_test.c:10077-10080`
* Rust source: `rs/fq/src/tests/tls_api.rs:930-932`

### C test body
```c
{
    return preferred_address_test_one(1, 0);
}
```

### Rust test body
```rust
fn preferred_address_dis_mig() {
    preferred_address_test_one(true, false).expect("preferred_address_dis_mig");
}
```
