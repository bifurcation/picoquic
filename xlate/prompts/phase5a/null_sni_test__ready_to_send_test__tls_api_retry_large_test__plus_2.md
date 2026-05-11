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

## `picoquictest/tls_api_test.c:null_sni_test`
* C test-table name: `null_sni`
* C entry function: `null_sni_test`
* Rust test: `null_sni`
* C source: `picoquictest/tls_api_test.c:9972-9975`
* Rust source: `rs/fq/src/tests/tls_api.rs:816-818`

### C test body
```c
{
    return tls_api_test_with_loss(NULL, PICOQUIC_INTERNAL_TEST_VERSION_1, NULL, PICOQUIC_TEST_ALPN);
}
```

### Rust test body
```rust
fn null_sni() {
    tls_api_test_with_loss(None, V1, None, Some(TEST_ALPN)).expect("null_sni");
}
```

## `picoquictest/tls_api_test.c:ready_to_send_test`
* C test-table name: `ready_to_send`
* C entry function: `ready_to_send_test`
* Rust test: `ready_to_send`
* C source: `picoquictest/tls_api_test.c:9448-9452`
* Rust source: `rs/fq/src/tests/tls_api.rs:1038-1040`

### C test body
```c
{
    int ret = ready_to_send_test_one(1);
    return ret;
}
```

### Rust test body
```rust
fn ready_to_send() {
    ready_to_send_test_one(1).expect("ready_to_send");
}
```

## `picoquictest/tls_api_test.c:tls_api_retry_large_test`
* C test-table name: `retry_large`
* C entry function: `tls_api_retry_large_test`
* Rust test: `retry_large`
* C source: `picoquictest/tls_api_test.c:4058-4061`
* Rust source: `rs/fq/src/tests/tls_api.rs:1137-1139`

### C test body
```c
{
    return tls_api_retry_test_one(1);
}
```

### Rust test body
```rust
fn retry_large() {
    tls_api_retry_test_one(true).expect("retry_large");
}
```

## `picoquictest/tls_api_test.c:tls_api_server_first_loss_test`
* C test-table name: `SH_loss`
* C entry function: `tls_api_server_first_loss_test`
* Rust test: `sh_loss`
* C source: `picoquictest/tls_api_test.c:4165-4168`
* Rust source: `rs/fq/src/tests/tls_api.rs:30-32`

### C test body
```c
{
    return tls_api_loss_test(14ull);
}
```

### Rust test body
```rust
fn sh_loss() {
    tls_api_loss_test(14).expect("sh_loss");
}
```

## `picoquictest/tls_api_test.c:stateless_reset_handshake_test`
* C test-table name: `stateless_reset_handshake`
* C entry function: `stateless_reset_handshake_test`
* Rust test: `stateless_reset_handshake`
* C source: `picoquictest/tls_api_test.c:3562-3621`
* Rust source: `rs/fq/src/tests/tls_api.rs:1284-1287`

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
        buffer[byte_index++] = 0xff;
        buffer[byte_index++] = test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len;
        buffer[byte_index++] = test_ctx->cnx_server->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len;
        /* Copy the client ID */
        byte_index += picoquic_format_connection_id(&buffer[byte_index], sizeof(buffer) - byte_index, test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id);
        /* Change one byte of the client ID */
        if (byte_index > 5) {
            buffer[5] ^= 0xff;
        }
        else {
            buffer[1] ^= 0xff;
        }
        /* Copy the server ID */
        byte_index += picoquic_format_connection_id(&buffer[byte_index], sizeof(buffer) - byte_index, test_ctx->cnx_server->path[0]->first_tuple->p_remote_cnxid->cnx_id);
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
    /* Check that no stateless packet is queued */
    if (ret == 0 && test_ctx->qserver->pending_stateless_packet != NULL) {
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
fn stateless_reset_handshake() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("stateless_reset_handshake");
}
```
