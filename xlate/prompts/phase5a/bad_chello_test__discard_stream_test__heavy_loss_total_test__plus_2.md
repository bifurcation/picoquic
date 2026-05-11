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

## `picoquictest/tls_api_test.c:bad_chello_test`
* C test-table name: `bad_chello`
* C entry function: `bad_chello_test`
* Rust test: `bad_chello`
* C source: `picoquictest/tls_api_test.c:11973-12020`
* Rust source: `rs/fq/src/tests/tls_api.rs:56-61`

### C test body
```c
{
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t simulated_time = 0;
    picoquic_connection_id_t icid = { { 0xba, 0xdc, 0xe1, 0x10, 0, 0, 0, 0}, 8 };
    int ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0, &icid, 10000, 0, 0, 0);
    uint8_t buffer[PICOQUIC_ENFORCED_INITIAL_MTU];
    

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }
    else {
        picoquic_set_qlog(test_ctx->qserver, ".");
        /* Create an initial packet with a bad chello */
        ret = bad_chello_fill_initial(test_ctx->qserver, buffer, PICOQUIC_ENFORCED_INITIAL_MTU, chello_malformed, sizeof(chello_malformed));
    }

    /* Submit the packet to the server context */
    if (ret == 0) {
        picoquic_cnx_t* cnx_trial = NULL;
        ret = picoquic_incoming_packet_ex(test_ctx->qserver, buffer, PICOQUIC_ENFORCED_INITIAL_MTU,
            (struct sockaddr*)&test_ctx->client_addr, (struct sockaddr*)&test_ctx->server_addr, 0,
            0, &cnx_trial, simulated_time);
        if (cnx_trial != NULL) {
            DBG_PRINTF("Bad chello caused context creation at t=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* If not apparently broken, start the client connection. */
    if (ret == 0) {
        simulated_time += 10000;
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 20000, 2000000);

        if (ret == 0) {
            DBG_PRINTF("Post bad chello connection succeeds at t=%" PRIu64 , simulated_time);
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
    }

    return ret;
}
```

### Rust test body
```rust
fn bad_chello() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("bad_chello");
}
```

## `picoquictest/tls_api_test.c:discard_stream_test`
* C test-table name: `discard_stream`
* C entry function: `discard_stream_test`
* Rust test: `discard_stream`
* C source: `picoquictest/tls_api_test.c:4913-4917`
* Rust source: `rs/fq/src/tests/tls_api.rs:300-302`

### C test body
```c
{
    int ret = stop_sending_test_one(1, 0);
    return ret;
}
```

### Rust test body
```rust
fn discard_stream() {
    stop_sending_test_one(true, false).expect("discard_stream");
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

## `picoquictest/tls_api_test.c:keep_alive_test`
* C test-table name: `keep_alive`
* C entry function: `keep_alive_test`
* Rust test: `keep_alive`
* C source: `picoquictest/tls_api_test.c:4269-4278`
* Rust source: `rs/fq/src/tests/tls_api.rs:473-476`

### C test body
```c
{
    int ret = keep_alive_test_impl(1);

    if (ret == 0) {
        ret = keep_alive_test_impl(0);
    }

    return ret;
}
```

### Rust test body
```rust
fn keep_alive() {
    keep_alive_test_impl(1).expect("keep_alive_on");
    keep_alive_test_impl(0).expect("keep_alive_off");
}
```

## `picoquictest/tls_api_test.c:loss_bit_test`
* C test-table name: `loss_bit`
* C entry function: `loss_bit_test`
* Rust test: `loss_bit`
* C source: `picoquictest/tls_api_test.c:6231-6253`
* Rust source: `rs/fq/src/tests/tls_api.rs:546-548`

### C test body
```c
{
    int ret = 0;
    picoquic_tp_t client_parameters;
    picoquic_tp_t server_parameters;

    for (int i = 0; ret == 0 && i <= 3; i++) {
        memset(&client_parameters, 0, sizeof(picoquic_tp_t));
        memset(&server_parameters, 0, sizeof(picoquic_tp_t));
        picoquic_init_transport_parameters(&client_parameters);
        picoquic_init_transport_parameters(&server_parameters);

        client_parameters.enable_loss_bit = (i & 1);
        server_parameters.enable_loss_bit = ((i > 1) & 1);

        ret = tls_api_one_scenario_test(test_scenario_many_streams, sizeof(test_scenario_many_streams), 0, 0, 0, 0, 0, 250000, &client_parameters, &server_parameters);
        if (ret != 0) {
            DBG_PRINTF("Loss bit test fails for client: %d, server: %d, ret = %d", i & 1, i >> 1, ret);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn loss_bit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("loss_bit");
}
```
