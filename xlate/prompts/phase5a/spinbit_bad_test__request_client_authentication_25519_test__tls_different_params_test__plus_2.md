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

## `picoquictest/spinbit_test.c:spinbit_bad_test`
* C test-table name: `spinbit_bad`
* C entry function: `spinbit_bad_test`
* Rust test: `spinbit_bad`
* C source: `picoquictest/spinbit_test.c:210-218`
* Rust source: `rs/fq/src/tests/spinbit.rs:168-175`

### C test body
```c
{
    int ret = 0;
    if (spinbit_test_one(picoquic_spinbit_on, 123456) == 0 ||
        spinbit_test_one(123455, picoquic_spinbit_null) == 0) {
        ret = -1;
    }
    return ret;
}
```

### Rust test body
```rust
fn spinbit_bad() {
    let r1 = spinbit_test_one(SpinbitVersion::On, SpinbitVersion::Basic);
    let r2 = spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::On);
    assert!(
        r1.is_err() || r2.is_err(),
        "expected at least one invalid per-connection policy to be rejected"
    );
}
```

## `picoquictest/tls_api_test.c:request_client_authentication_25519_test`
* C test-table name: `client_auth_25519`
* C entry function: `request_client_authentication_25519_test`
* Rust test: `client_auth_25519`
* C source: `picoquictest/tls_api_test.c:5795-5839`
* Rust source: `rs/fq/src/tests/tls_api.rs:137-143`

### C test body
```c
{
    char test_client_cert_file[512];
    char test_client_key_file[512];
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_ca_cert_store_file[512];
    int ret = 0;

    ret = picoquic_get_input_path(test_client_cert_file, sizeof(test_client_cert_file),
                                  picoquic_solution_dir, PICOQUIC_TEST_FILE_CLIENT_CERT_ED25519);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_client_key_file, sizeof(test_client_key_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_CLIENT_KEY_ED25519);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file),
                                      picoquic_solution_dir,
                                      PICOQUIC_TEST_FILE_SERVER_CERT_ED25519);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY_ED25519);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_ca_cert_store_file, sizeof(test_ca_cert_store_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE_ED25519);
    }

    if (ret == 0) {
        ret = request_client_authentication_test_one(test_client_cert_file, test_client_key_file,
                                                     test_server_cert_file, test_server_key_file,
                                                     test_ca_cert_store_file);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "mTLS client-auth test failed ED25519\n");
    }

    return ret;
}
```

### Rust test body
```rust
fn client_auth_25519() {
    request_client_authentication_test_one(
        TEST_FILE_CLIENT_CERT_ED25519,
        TEST_FILE_CLIENT_KEY_ED25519,
    )
    .expect("client_auth_25519");
}
```

## `picoquictest/tls_api_test.c:tls_different_params_test`
* C test-table name: `different_params`
* C entry function: `tls_different_params_test`
* Rust test: `different_params`
* C source: `picoquictest/tls_api_test.c:5537-5548`
* Rust source: `rs/fq/src/tests/tls_api.rs:279-284`

### C test body
```c
{
    picoquic_tp_t test_parameters;

    memset(&test_parameters, 0, sizeof(picoquic_tp_t));

    picoquic_init_transport_parameters(&test_parameters);

    test_parameters.initial_max_stream_id_bidir = 0;

    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 0, 0, 0, 3510000, &test_parameters, NULL);
}
```

### Rust test body
```rust
fn different_params() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_510_000)
        .expect("different_params");
}
```

## `picoquictest/tls_api_test.c:get_tls_errors_test`
* C test-table name: `get_tls_errors`
* C entry function: `get_tls_errors_test`
* Rust test: `get_tls_errors`
* C source: `picoquictest/tls_api_test.c:12927-13005`
* Rust source: `rs/fq/src/tests/tls_api.rs:359-361`

### C test body
```c
{
    int ret = 0;
    uint8_t data[128];
    char const* invalid_stuff = "no_such_stuff_nada_niente";
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t simulated_time = 0;
    picoquic_connection_id_t initial_cid = { {0x9e, 0x71, 0x5e, 0, 0, 0, 0, 0}, 8 };
    int invalid_id = 0xFFFE8808;

    ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid, 8, 0, 0, 0);

    memset(data, 0xaa, sizeof(data));
    if (ret == 0 && picoquic_ecb_create_by_name(0, data, invalid_stuff) != NULL) {
        ret = -1;
    }
    if (ret == 0 && picoquic_get_cipher_suite_by_id_v(invalid_id, 0) != NULL) {
        ret = -1;
    }
    if (ret == 0 && picoquic_set_cipher_suite(test_ctx->qserver, invalid_id) == 0) {
        ret = -1;
    }
    if (ret == 0 && picoquic_set_key_exchange(test_ctx->qserver, invalid_id) == 0) {
        ret = -1;
    }
    if (ret == 0 &&
        picoquic_get_cipher_suite_by_id_v(PICOQUIC_AES_128_GCM_SHA256, 1) == NULL &&
        picoquic_get_cipher_suite_by_id_v(PICOQUIC_AES_128_GCM_SHA256, 0) == NULL) {
        ret = -1;
    }
    if (ret == 0 &&
        picoquic_get_cipher_suite_by_id_v(invalid_id, 1) != NULL) {
        ret = -1;
    }
    if (ret == 0) {

    }
    if (ret == 0) {
        /* Unload the TLS API to force internal errors. */
        picoquic_cnx_t* cnx;
        picoquic_tls_api_unload();

        cnx = picoquic_create_cnx(test_ctx->qclient, picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*)&test_ctx->server_addr, simulated_time, PICOQUIC_INTERNAL_TEST_VERSION_1,
            PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 0);
        if (cnx != NULL) {
            ret = -1;
            picoquic_delete_cnx(cnx);
        }
        if (ret == 0) {
            if (picoquic_set_private_key_from_file(test_ctx->qclient, "some bad file name.not") == 0) {
                ret = -1;
            }
        }
        if (ret == 0) {
            size_t count = 0;
            ptls_iovec_t* certs = picoquic_get_certs_from_file("some bad file name.not", &count);
            if (certs != NULL) {
                ret = -1;
                for (size_t i = 0; i < count; i++) {
                    free(certs[i].base);
                }
                free(certs);
            }
        }

        ptls_iovec_t* picoquic_get_certs_from_file(char const* file_name, size_t * count);
        /* Reinit the TLS API */
        picoquic_tls_api_init();
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
fn get_tls_errors() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("get_tls_errors");
}
```

## `picoquictest/tls_api_test.c:implicit_ack_test`
* C test-table name: `implicit_ack`
* C entry function: `implicit_ack_test`
* Rust test: `implicit_ack`
* C source: `picoquictest/tls_api_test.c:3345-3386`
* Rust source: `rs/fq/src/tests/tls_api.rs:425-430`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    picoquic_packet_context_enum pc[2] = { picoquic_packet_context_initial, picoquic_packet_context_handshake };

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    for (int i = 0; ret == 0 && i < 2; i++) {
        if (test_ctx->cnx_client->pkt_ctx[pc[i]].pending_first != NULL) {
            DBG_PRINTF("Retransmit queue type %d not empty on client", pc[i]);
            ret = -1;
        }
        else if (test_ctx->cnx_server->pkt_ctx[pc[i]].pending_first != NULL) {
            DBG_PRINTF("Retransmit queue type %d not empty on server", pc[i]);
            ret = -1;
        }
        else if (test_ctx->cnx_client->pkt_ctx[pc[i]].retransmitted_oldest != NULL) {
            DBG_PRINTF("Retransmitted queue type %d not empty on client", pc[i]);
            ret = -1;
        }
        else if (test_ctx->cnx_server->pkt_ctx[pc[i]].pending_first != NULL) {
            DBG_PRINTF("Retransmitted queue type %d not empty on server", pc[i]);
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
fn implicit_ack() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("implicit_ack");
}
```
