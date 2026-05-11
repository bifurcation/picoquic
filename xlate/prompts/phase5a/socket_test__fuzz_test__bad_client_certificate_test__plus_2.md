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

## `picoquictest/socket_test.c:socket_test`
* C test-table name: `sockets`
* C entry function: `socket_test`
* Rust test: `sockets`
* C source: `picoquictest/socket_test.c:165-196`
* Rust source: `rs/fq/src/tests/socket.rs:161-177`

### C test body
```c
{
    int ret = 0;
    int test_port = 12345;
    int test_port2 = 1234;

    /* Open server sockets */
    picoquic_server_sockets_t server_sockets;
    ret = picoquic_open_server_sockets(&server_sockets, test_port);

    if (ret == 0) {

        /* Test with one server socket */
        ret = socket_test_port(&server_sockets, test_port);

        if (ret == 0) {
            /* Test with two server sockets */
            picoquic_server_sockets_t server_sockets2;
            ret = picoquic_open_server_sockets(&server_sockets2, test_port2);

            if (ret == 0) {
                ret = socket_test_port(&server_sockets2, test_port2);
                picoquic_close_server_sockets(&server_sockets2);
            }
        }

        /* Close the sockets */
        picoquic_close_server_sockets(&server_sockets);
    }

    return ret;
}
```

### Rust test body
```rust
fn sockets() {
    let test_port: u16 = 12345;
    let test_port2: u16 = 1234;

    let mut server_sockets =
        ServerSockets::<Socket2Udp>::open(test_port as i32).expect("open server sockets");

    socket_test_port(&mut server_sockets, test_port).expect("ping-pong on port 12345");

    let mut server_sockets2 =
        ServerSockets::<Socket2Udp>::open(test_port2 as i32).expect("open server sockets 2");

    socket_test_port(&mut server_sockets2, test_port2).expect("ping-pong on port 1234");

    server_sockets2.close();
    server_sockets.close();
}
```

## `picoquictest/stresstest.c:fuzz_test`
* C test-table name: `fuzz`
* C entry function: `fuzz_test`
* Rust test: `fuzz`
* C source: `picoquictest/stresstest.c:1230-1250`
* Rust source: `rs/fq/src/tests/stresstest.rs:259-262`

### C test body
```c
{
    basic_fuzzer_ctx_t fuzz_ctx;
    int ret = 0;

    fuzz_ctx.nb_packets = 0;
    fuzz_ctx.nb_fuzzed = 0;
    fuzz_ctx.nb_fuzzed_length = 0;
    fuzz_ctx.highest_state_fuzzed = 0;
    /* Random seed depends on duration, so different durations do not all start 
     * with exactly the same message sequences. */
    fuzz_ctx.random_context = 0xDEADBEEFBABACAFEull;
    fuzz_ctx.random_context ^= picoquic_stress_test_duration;

    ret = stress_or_fuzz_test(basic_fuzzer, &fuzz_ctx, picoquic_stress_test_duration, picoquic_stress_test_duration);

    DBG_PRINTF("Fuzzed %d packets out of %d, changed %d lengths, ret = %d\n",
        fuzz_ctx.nb_fuzzed, fuzz_ctx.nb_packets, fuzz_ctx.nb_fuzzed_length, ret);

    return ret;
}
```

### Rust test body
```rust
fn fuzz() {
    let duration: u64 = 60_000_000;
    stress_or_fuzz_test(duration, duration).expect("fuzz_test");
}
```

## `picoquictest/tls_api_test.c:bad_client_certificate_test`
* C test-table name: `bad_client_certificate`
* C entry function: `bad_client_certificate_test`
* Rust test: `bad_client_certificate`
* C source: `picoquictest/tls_api_test.c:5841-5930`
* Rust source: `rs/fq/src/tests/tls_api.rs:68-71`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_server_cert_store_file[512];
    int ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_BAD_CERT);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }
    else {
        ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    }

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Delete the client context, and recreate with a certificate */
    if (ret == 0)
    {
        if (test_ctx->qclient != NULL) {
            picoquic_free(test_ctx->qclient);
            test_ctx->cnx_client = NULL;
        }

        test_ctx->qclient = picoquic_create(8,
            test_server_cert_file, test_server_key_file, test_server_cert_store_file,
            NULL, test_api_callback, (void*)&test_ctx->client_callback, NULL, NULL, NULL,
            simulated_time, &simulated_time, NULL, NULL, 0);

        if (test_ctx->qclient == NULL) {
            ret = -1;
        }
    }

    /* recreate the client connection */
    if (ret == 0) {
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient, picoquic_null_connection_id,
                                                   picoquic_null_connection_id,
                                                   (struct sockaddr*)&test_ctx->server_addr, 0,
                                                   0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        } else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }

    if (ret == 0) {
        picoquic_set_client_authentication(test_ctx->qserver, 1);
        
        /* Proceed with the connection loop. It should fail */
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_local_error(test_ctx->cnx_server))) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_remote_error(test_ctx->cnx_client))) {
            ret = -1;
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
fn bad_client_certificate() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("bad_client_certificate");
}
```

## `picoquictest/tls_api_test.c:tls_api_client_losses_test`
* C test-table name: `client_losses`
* C entry function: `tls_api_client_losses_test`
* Rust test: `client_losses`
* C source: `picoquictest/tls_api_test.c:4170-4173`
* Rust source: `rs/fq/src/tests/tls_api.rs:170-172`

### C test body
```c
{
    return tls_api_loss_test(3ull);
}
```

### Rust test body
```rust
fn client_losses() {
    tls_api_loss_test(3).expect("client_losses");
}
```

## `picoquictest/tls_api_test.c:transmit_cnxid_retire_early_test`
* C test-table name: `cnxid_transmit_r_early`
* C entry function: `transmit_cnxid_retire_early_test`
* Rust test: `cnxid_transmit_r_early`
* C source: `picoquictest/tls_api_test.c:6631-6634`
* Rust source: `rs/fq/src/tests/tls_api.rs:237-239`

### C test body
```c
{
    return transmit_cnxid_test_one(0, 0, 1);
}
```

### Rust test body
```rust
fn cnxid_transmit_r_early() {
    transmit_cnxid_test_one(false, false, true).expect("cnxid_transmit_r_early");
}
```
