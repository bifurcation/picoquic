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

## `picoquictest/tls_api_test.c:stateless_reset_client_test`
* C test-table name: `stateless_reset_client`
* C entry function: `stateless_reset_client_test`
* Rust test: `stateless_reset_client`
* C source: `picoquictest/tls_api_test.c:3505-3559`
* Rust source: `rs/fq/src/tests/tls_api.rs:1275-1278`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    uint8_t buffer[256];

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare the bogus reset */
    if (ret == 0) {
        size_t byte_index = 0;
        buffer[byte_index++] = 0x41;
        /* Copy the client ID */
        byte_index += picoquic_format_connection_id(&buffer[byte_index], sizeof(buffer) - byte_index, test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id);
        /* Change one byte of the client ID */
        if (byte_index > 5) {
            buffer[5] ^= 0xff;
        }
        else {
            buffer[1] ^= 0xff;
        }
        /* rest of packet is null */
        memset(buffer + byte_index, 0xcc, sizeof(buffer) - byte_index);

        /* Submit bogus request to server */
        ret = picoquic_incoming_packet(test_ctx->qserver, buffer, sizeof(buffer),
            (struct sockaddr*)(&test_ctx->client_addr),
            (struct sockaddr*)(&test_ctx->server_addr), 0, test_ctx->recv_ecn_server,
            simulated_time);
    }

    /* check that the server is still up */
    if (ret == 0 && test_ctx->cnx_server != NULL && test_ctx->cnx_server->cnx_state > picoquic_state_ready) {
        ret = -1;
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
fn stateless_reset_client() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("stateless_reset_client");
}
```

## `picoquictest/tls_api_test.c:tls_api_q2_and_r2_stream_test`
* C test-table name: `tls_api_q2_and_r2_stream`
* C entry function: `tls_api_q2_and_r2_stream_test`
* Rust test: `tls_api_q2_and_r2_stream`
* C source: `picoquictest/tls_api_test.c:3276-3279`
* Rust source: `rs/fq/src/tests/tls_api.rs:1374-1378`

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2), 0, 0, 0, 0, 0, 86000, NULL, NULL);
}
```

### Rust test body
```rust
fn tls_api_q2_and_r2_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("q2_and_r2_stream");
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

## `picoquictest/tls_api_test.c:vn_compat_test`
* C test-table name: `vn_compat`
* C entry function: `vn_compat_test`
* Rust test: `vn_compat`
* C source: `picoquictest/tls_api_test.c:2948-2963`
* Rust source: `rs/fq/src/tests/tls_api.rs:1535-1537`

### C test body
```c
{
    int ret = 0;

    if (vn_compat_test_one(PICOQUIC_V1_VERSION, PICOQUIC_V2_VERSION) != 0) {
        ret = -1;
    }
    else if (vn_compat_test_one(PICOQUIC_V1_VERSION, PICOQUIC_V2_VERSION_DRAFT) != 0) {
        ret = -1;
    }
    else if (vn_compat_test_one(PICOQUIC_V1_VERSION, PICOQUIC_INTERNAL_TEST_VERSION_1) == 0) {
        ret = -1;
    }

    return ret;
}
```

### Rust test body
```rust
fn vn_compat() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("vn_compat");
}
```
