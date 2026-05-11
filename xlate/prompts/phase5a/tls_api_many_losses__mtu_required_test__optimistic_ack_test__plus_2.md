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

## `picoquictest/tls_api_test.c:optimistic_ack_test`
* C test-table name: `optimistic_ack`
* C entry function: `optimistic_ack_test`
* Rust test: `optimistic_ack`
* C source: `picoquictest/tls_api_test.c:9762-9767`
* Rust source: `rs/fq/src/tests/tls_api.rs:825-827`

### C test body
```c
{
    int ret = optimistic_ack_test_one(1);

    return ret;
}
```

### Rust test body
```rust
fn optimistic_ack() {
    optimistic_ack_test_one(true).expect("optimistic_ack");
}
```

## `picoquictest/tls_api_test.c:pn_enc_1rtt_test`
* C test-table name: `pn_enc_1rtt`
* C entry function: `pn_enc_1rtt_test`
* Rust test: `pn_enc_1rtt`
* C source: `picoquictest/tls_api_test.c:5178-5222`
* Rust source: `rs/fq/src/tests/tls_api.rs:894-896`

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_application_aead_ready(test_ctx, &simulated_time);
    }

    if (ret == 0)
    {
        /* Try to encrypt a sequence number */
        uint8_t seq_num_1[4] = { 0xde, 0xad, 0xbe, 0xef };
        uint8_t sample_1[16] = {
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
            0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a };
        uint8_t seq_num_2[4] = { 0xba, 0xba, 0xc0, 0x0l };
        uint8_t sample_2[16] = {
            0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96 };

        for (int i = 1; i < 4; i *= 2)
        {
            ret = test_one_pn_enc_pair(seq_num_1, 4, test_ctx->cnx_client->crypto_context[3].pn_enc, test_ctx->cnx_server->crypto_context[3].pn_dec, sample_1);

            if (ret == 0)
            {
                ret = test_one_pn_enc_pair(seq_num_2, 4, test_ctx->cnx_server->crypto_context[3].pn_enc, test_ctx->cnx_client->crypto_context[3].pn_dec, sample_2);
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
fn pn_enc_1rtt() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("pn_enc_1rtt");
}
```

## `picoquictest/tls_api_test.c:qlog_fns_ecn_test`
* C test-table name: `qlog_fns_ecn`
* C entry function: `qlog_fns_ecn_test`
* Rust test: `qlog_fns_ecn`
* C source: `picoquictest/tls_api_test.c:9205-9208`
* Rust source: `rs/fq/src/tests/tls_api.rs:964-966`

### C test body
```c
{
    return qlog_fns_test_one(0x02);
}
```

### Rust test body
```rust
fn qlog_fns_ecn() {
    qlog_fns_test_one(0x02).expect("qlog_fns_ecn");
}
```
