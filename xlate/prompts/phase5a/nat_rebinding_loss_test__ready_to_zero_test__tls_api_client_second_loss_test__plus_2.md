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

## `picoquictest/tls_api_test.c:nat_rebinding_loss_test`
* C test-table name: `nat_rebinding_loss`
* C entry function: `nat_rebinding_loss_test`
* Rust test: `nat_rebinding_loss`
* C source: `picoquictest/tls_api_test.c:6079-6084`
* Rust source: `rs/fq/src/tests/tls_api.rs:763-765`

### C test body
```c
{
    uint64_t loss_mask = 0x2012;

    return nat_rebinding_test_one(loss_mask, 0, 0);
}
```

### Rust test body
```rust
fn nat_rebinding_loss() {
    nat_rebinding_test_one(0x2012, false, 0).expect("nat_rebinding_loss");
}
```

## `picoquictest/tls_api_test.c:ready_to_zero_test`
* C test-table name: `ready_to_zero`
* C entry function: `ready_to_zero_test`
* Rust test: `ready_to_zero`
* C source: `picoquictest/tls_api_test.c:9460-9464`
* Rust source: `rs/fq/src/tests/tls_api.rs:1054-1056`

### C test body
```c
{
    int ret = ready_to_send_test_one(4);
    return ret;
}
```

### Rust test body
```rust
fn ready_to_zero() {
    ready_to_send_test_one(4).expect("ready_to_zero");
}
```

## `picoquictest/tls_api_test.c:tls_api_client_second_loss_test`
* C test-table name: `second_loss`
* C entry function: `tls_api_client_second_loss_test`
* Rust test: `second_loss`
* C source: `picoquictest/tls_api_test.c:4160-4163`
* Rust source: `rs/fq/src/tests/tls_api.rs:1163-1165`

### C test body
```c
{
    return tls_api_loss_test(2ull);
}
```

### Rust test body
```rust
fn second_loss() {
    tls_api_loss_test(2).expect("second_loss");
}
```

## `picoquictest/tls_api_test.c:test_stateless_blowback`
* C test-table name: `stateless_blowback`
* C entry function: `test_stateless_blowback`
* Rust test: `stateless_blowback`
* C source: `picoquictest/tls_api_test.c:12182-12284`
* Rust source: `rs/fq/src/tests/tls_api.rs:1250-1252`

### C test body
```c
{
    int was_sent = 0;
    uint64_t new_interval = 2 * PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT;

    /* Create a context with the default timer. */
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;

    picoquic_connection_id_t initial_cid = { {0xb1, 0x08, 0xba, 0xcc, 0, 0, 0, 0}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN,
        &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);

    if (test_ctx->qserver->stateless_reset_min_interval != PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT) {
        DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
        ret = -1;

    }

    /* Format a random packet and submit it, verify that the stateless reset is queued */
    if (ret == 0) {
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("First stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Progress by 1/2 specified interval, retry, it should not work  */
    if (ret == 0) {
        simulated_time += test_ctx->qserver->stateless_reset_min_interval;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("Second stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }
    
    /* Progress by 1x specified interval, retry, it should work  */
    if (ret == 0) {
        simulated_time += test_ctx->qserver->stateless_reset_min_interval / 2;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && was_sent) {
            DBG_PRINTF("Third stateless reset was sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Reset the interval to twice the previous value */
    if (ret == 0) {
        picoquic_set_default_stateless_reset_min_interval(test_ctx->qserver, new_interval);
        if (test_ctx->qserver->stateless_reset_min_interval != new_interval) {
            DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
            ret = -1;
        }
    }
    
    /* Progress by 0.75x new interval, retry, it should work  */
    if (ret == 0) {
        simulated_time += (new_interval - new_interval / 4);
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After new interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Reset the interval to zero */
    if (ret == 0) {
        picoquic_set_default_stateless_reset_min_interval(test_ctx->qserver, 0);
        if (test_ctx->qserver->stateless_reset_min_interval != 0) {
            DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
            ret = -1;
        }
    }

    /* Try immediately, it should not work  */
    if (ret == 0) {
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After zero interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Add 1 microsec, it should work  */
    if (ret == 0) {
        simulated_time += 1;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After zero +1 interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Free the resurce and return */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn stateless_blowback() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_blowback");
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
