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

## `picoquictest/tls_api_test.c:tls_api_many_losses`
* C test-table name: `many_losses`
* C entry function: `tls_api_many_losses`
* Rust test: `many_losses`
* C source: `picoquictest/tls_api_test.c:2489-2535`
* Rust source: `rs/fq/src/tests/tls_api.rs:555-559`

### C test body
```c
{
    uint64_t loss_mask = 0;
    int ret = 0;
    uint64_t random_context = 0x1055ca45c001babaull;

    /* We first test with a set of preprogrammed masks, checking consecutive drops */
    for (int i = 0; ret == 0 && i < 6; i++) {
        for (int j = 0; ret == 0 && j < 4; j++) {
            uint64_t j_mask = ~(UINT64_MAX << j);
            loss_mask = j_mask << i;
            ret = tls_api_loss_test(loss_mask);
            if (ret != 0) {
                DBG_PRINTF("Handshake fails for mask %d-%d = %llx", i, j, (unsigned long long)loss_mask);
            }
        }
        for (uint64_t j = 8; ret == 0 && j < 11; j++) {
            loss_mask = (j | (j << 4) | (j << 8))<<i;
            ret = tls_api_loss_test(loss_mask);
            if (ret != 0) {
                DBG_PRINTF("Handshake fails for mask %d, %" PRIu64" = %llx", i, j,  (unsigned long long)loss_mask);
            }
        }
    }

    /* Then we verify that we can establish 50 connections with packet drop rate=30% */
    for (int i = 0; ret == 0 &&  i < 50; i++)
    {
        uint64_t loss_mask = 0;
        for (int j = 0; j < 64; j++)
        {
            loss_mask <<= 1;

            if (picoquic_test_uniform_random(&random_context, 1000) < 300) {
                loss_mask |= 1;
            }
        }

        ret = tls_api_one_scenario_test(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, loss_mask, 128000, 0, 0, 0, NULL, NULL);
        if (ret != 0) {
            DBG_PRINTF("Handshake fails for random mask %d, mask = %llx", i, (unsigned long long)loss_mask);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn many_losses() {
    for mask in [1u64, 2, 3, 6, 14, 0x55, 0xAA, 0xFF] {
        tls_api_loss_test(mask).unwrap_or_else(|e| panic!("many_losses mask={mask:#x}: {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:nat_rebinding_latency_test`
* C test-table name: `nat_rebinding_latency`
* C entry function: `nat_rebinding_latency_test`
* Rust test: `nat_rebinding_latency`
* C source: `picoquictest/tls_api_test.c:6094-6100`
* Rust source: `rs/fq/src/tests/tls_api.rs:755-757`

### C test body
```c
{
    /* Test of NAT rebinding with zero-length client CID */
    uint64_t loss_mask = 0;

    return nat_rebinding_test_one(loss_mask, 0, 100000);
}
```

### Rust test body
```rust
fn nat_rebinding_latency() {
    nat_rebinding_test_one(0, false, 100_000).expect("nat_rebinding_latency");
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

## `picoquictest/tls_api_test.c:quality_update_test`
* C test-table name: `quality_update`
* C entry function: `quality_update_test`
* Rust test: `quality_update`
* C source: `picoquictest/tls_api_test.c:10653-10703`
* Rust source: `rs/fq/src/tests/tls_api.rs:998-1000`

### C test body
```c
{
    uint64_t simulated_time = 0;

    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }
    if (ret == 0) {
        /* Open a file to log bandwidth updates and document it in context */
        test_ctx->default_path_update = picoquic_file_open(QUALITY_UPDATE_CSV, "w");
        if (test_ctx->default_path_update == NULL) {
            DBG_PRINTF("Could not write file <%s>", QUALITY_UPDATE_CSV);
            ret = -1;
        }
        else {
            fprintf(test_ctx->default_path_update, "Time, Path_id, Sending_rate_CB, Pacing_rate, Receive_Rate, CWIN, RTT\n");
            /* Request bandwidth updates */
            picoquic_subscribe_to_quality_update(test_ctx->cnx_client, 0x10000, 0x1000);

            /* Start a standard scenario, pushing 1MB from the client*/
            ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
                test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 1000000, 0, 0, 20000, 3600000);
        }
    }

    /* Free the test contex, which closes the trace file  */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    /* compare the trace to the expected value */
    if (ret == 0)
    {
        char quality_update_ref[512];

        ret = picoquic_get_input_path(quality_update_ref, sizeof(quality_update_ref), picoquic_solution_dir, QUALITY_UPDATE_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the quality update ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(QUALITY_UPDATE_CSV, quality_update_ref);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn quality_update() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("quality_update");
}
```
