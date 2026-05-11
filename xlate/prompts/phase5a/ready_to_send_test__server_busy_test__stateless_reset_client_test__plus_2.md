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

## `picoquictest/tls_api_test.c:server_busy_test`
* C test-table name: `server_busy`
* C entry function: `server_busy_test`
* Rust test: `server_busy`
* C source: `picoquictest/tls_api_test.c:7408-7474`
* Rust source: `rs/fq/src/tests/tls_api.rs:1172-1174`

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        test_ctx->qserver->server_busy = 1;
        (void) tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);

        if (test_ctx->cnx_server != NULL &&
            test_ctx->cnx_server->cnx_state != picoquic_state_disconnected) {
            DBG_PRINTF("Server state: %d, local error: %" PRIx64, test_ctx->cnx_server->cnx_state, test_ctx->cnx_server->local_error);
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected ||
            test_ctx->cnx_client->remote_error != PICOQUIC_TRANSPORT_SERVER_BUSY) {
            DBG_PRINTF("Client state: %d, remote error: %" PRIx64, test_ctx->cnx_client->cnx_state, test_ctx->cnx_client->remote_error);
            ret = -1;
        }
        else if (simulated_time > 500000ull) {
            DBG_PRINTF("Simulated time: %" PRIu64, (unsigned long long)simulated_time);
            ret = -1;
        }
    }

    if (ret == 0) {
        test_ctx->qserver->server_busy = 0;

        if (test_ctx->cnx_server != NULL) {
            picoquic_delete_cnx(test_ctx->cnx_server);
            test_ctx->cnx_server = NULL;
        }
        if (test_ctx->cnx_client != NULL) {
            picoquic_delete_cnx(test_ctx->cnx_client);
            test_ctx->cnx_client = NULL;
        }

        /* Create a new client connection */
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient,
            picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*)&test_ctx->server_addr, simulated_time,
            0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        } else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
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
fn server_busy() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("server_busy");
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

## `picoquictest/tls_api_test.c:tls_api_inject_hs_ack_test`
* C test-table name: `tls_api_inject_hs_ack`
* C entry function: `tls_api_inject_hs_ack_test`
* Rust test: `tls_api_inject_hs_ack`
* C source: `picoquictest/tls_api_test.c:2288-2349`
* Rust source: `rs/fq/src/tests/tls_api.rs:1349-1358`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", PICOQUIC_INTERNAL_TEST_VERSION_1);
    }

    if (ret == 0) {
        int ret = 0;
        int nb_trials = 0;
        int nb_inactive = 0;
        int injected = 0;

        while (ret == 0 && nb_trials < 1024 && nb_inactive < 512 && (!TEST_CLIENT_READY || (test_ctx->cnx_server == NULL || !TEST_SERVER_READY))) {
            int was_active = 0;
            nb_trials++;

            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

            if (test_ctx->cnx_client->cnx_state == picoquic_state_disconnected &&
                (test_ctx->cnx_server == NULL || test_ctx->cnx_server->cnx_state == picoquic_state_disconnected)) {
                break;
            }

            if (!injected && test_ctx->cnx_client->crypto_context[2].aead_encrypt != NULL) {
                const uint8_t ack_only[] = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    picoquic_frame_type_ack, 0, 0, 0, 0 };

                ret = tls_api_inject_packet(test_ctx, 1, 2, ack_only, sizeof(ack_only), 0, simulated_time);

                injected = 1;
            }

            if (was_active) {
                nb_inactive = 0;
            }
            else {
                nb_inactive++;
            }
        }

        if (ret != 0)
        {
            DBG_PRINTF("Connection loop returns %d\n", ret);
        }
    }

    if (ret == 0) {
        ret = tls_api_test_with_loss_final(test_ctx, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time);
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
fn tls_api_inject_hs_ack() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tester_wait_handshake_key(&mut ctx, &mut t).expect("handshake_key");
    let ack_frame = tester_simple_ack_frame(0);
    tester_push_frame_packet(&mut ctx, PacketType::Handshake, &ack_frame, false, false, t)
        .expect("push_ack");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("inject_hs_ack");
}
```

## `picoquictest/tls_api_test.c:tls_api_very_long_with_err_test`
* C test-table name: `tls_api_very_long_with_err`
* C entry function: `tls_api_very_long_with_err_test`
* Rust test: `tls_api_very_long_with_err`
* C source: `picoquictest/tls_api_test.c:3291-3294`
* Rust source: `rs/fq/src/tests/tls_api.rs:1436-1441`

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0x30000, 128000, 0, 0, 2210000, NULL, NULL);
}
```

### Rust test body
```rust
fn tls_api_very_long_with_err() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0x3_0000, 0, 0, 0, 2_210_000)
        .expect("very_long_with_err");
}
```
