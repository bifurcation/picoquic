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

## `picoquictest/tls_api_test.c:tls_api_alpn_test`
* C test-table name: `tls_api_alpn`
* C entry function: `tls_api_alpn_test`
* Rust test: `tls_api_alpn`
* C source: `picoquictest/tls_api_test.c:2976-2991`
* Rust source: `rs/fq/src/tests/tls_api.rs:1329-1331`

### C test body
```c
{
    int ret = tls_api_test_with_loss(NULL, 0, PICOQUIC_TEST_SNI, NULL);

    if (ret == PICOQUIC_ERROR_NO_ALPN_PROVIDED) {
        ret = 0;
    } else if (ret == 0) {
        DBG_PRINTF("ALPN test succeeds while no ALPN is specified, ret = 0x%x", ret);
        ret = -1;
    }
    else {
        DBG_PRINTF("ALPN test does not return expected error code, ret = 0x%x", ret);
        ret = -1;
    }
    return ret;
}
```

### Rust test body
```rust
fn tls_api_alpn() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), None).expect("tls_api_alpn");
}
```

## `picoquictest/tls_api_test.c:tls_exporter_test`
* C test-table name: `tls_exporter`
* C entry function: `tls_exporter_test`
* Rust test: `tls_exporter`
* C source: `picoquictest/tls_api_test.c:2351-2439`
* Rust source: `rs/fq/src/tests/tls_api.rs:1457-1462`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t *test_ctx = NULL;

    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI,
                               PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        if (test_ctx->qclient != NULL) {
            picoquic_free(test_ctx->qclient);
            test_ctx->qclient = NULL;
            test_ctx->cnx_client = NULL;
        }

        test_ctx->qclient = picoquic_create(8, NULL, NULL, NULL, NULL, test_api_callback,
                                            (void *)&test_ctx->client_callback, NULL, NULL, NULL,
                                            simulated_time, &simulated_time, NULL, NULL, 0);

        if (test_ctx->qclient == NULL) {
            ret = -1;
        }
    }

    if (ret == 0) {
        picoquic_set_use_exporter(test_ctx->qclient, 1);
        picoquic_set_use_exporter(test_ctx->qserver, 1);
    }

    if (ret == 0) {
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient, picoquic_null_connection_id,
                                                   picoquic_null_connection_id,
                                                   (struct sockaddr *)&test_ctx->server_addr, 0, 0,
                                                   PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        const char *label = "tls api test";
        const size_t export_key_len = 16;
        unsigned char client_export_key[16] = { 0 };
        unsigned char server_export_key[16] = { 0 };

        picoquic_cnx_t *client_cnx = test_ctx->cnx_client;
        picoquic_cnx_t *server_cnx = test_ctx->cnx_server;

        if (client_cnx == NULL || server_cnx == NULL) {
            ret = -1;
        }

        if (ret == 0) {
            int r = picoquic_export_secret(client_cnx, label, client_export_key, export_key_len);
            if (r != 0) {
                ret = -1;
            }
        }

        if (ret == 0) {
            int r = picoquic_export_secret(server_cnx, label, server_export_key, export_key_len);
            if (r != 0) {
                ret = -1;
            }
        }

        if (ret == 0) {
            if (memcmp(client_export_key, server_export_key, export_key_len) != 0) {
                ret = -1;
            }
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
fn tls_exporter() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("exporter_connect");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_test`
* C test-table name: `zero_rtt`
* C entry function: `zero_rtt_test`
* Rust test: `zero_rtt`
* C source: `picoquictest/tls_api_test.c:4614-4618`
* Rust source: `rs/fq/src/tests/tls_api.rs:1543-1545`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt() {
    zero_rtt_test_one(&ZeroRttTest::default()).expect("zero_rtt");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_retry_test`
* C test-table name: `zero_rtt_retry`
* C entry function: `zero_rtt_retry_test`
* Rust test: `zero_rtt_retry`
* C source: `picoquictest/tls_api_test.c:4683-4688`
* Rust source: `rs/fq/src/tests/tls_api.rs:1650-1656`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.hardreset = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt_retry() {
    zero_rtt_test_one(&ZeroRttTest {
        hardreset: true,
        ..Default::default()
    })
    .expect("zero_rtt_retry");
}
```
