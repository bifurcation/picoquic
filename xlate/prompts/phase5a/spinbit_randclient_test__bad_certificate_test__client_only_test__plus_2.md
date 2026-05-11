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

## `picoquictest/spinbit_test.c:spinbit_randclient_test`
* C test-table name: `spinbit_randclient`
* C entry function: `spinbit_randclient_test`
* Rust test: `spinbit_randclient`
* C source: `picoquictest/spinbit_test.c:200-203`
* Rust source: `rs/fq/src/tests/spinbit.rs:150-152`

### C test body
```c
{
    return spinbit_test_one(picoquic_spinbit_random, picoquic_spinbit_basic);
}
```

### Rust test body
```rust
fn spinbit_randclient() {
    spinbit_test_one(SpinbitVersion::Random, SpinbitVersion::Basic).expect("spinbit_randclient");
}
```

## `picoquictest/tls_api_test.c:bad_certificate_test`
* C test-table name: `bad_certificate`
* C entry function: `bad_certificate_test`
* Rust test: `bad_certificate`
* C source: `picoquictest/tls_api_test.c:5224-5296`
* Rust source: `rs/fq/src/tests/tls_api.rs:47-50`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_server_cert_store_file[512];
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_BAD_CERT);

        if (ret == 0) {
            ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
        }

        if (ret == 0) {
            ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
        }

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
        }
    }

    /* Delete the server context, and recreate it with the bad certificate */

    if (ret == 0)
    {
        if (test_ctx->qserver != NULL) {
            picoquic_free(test_ctx->qserver);
        }

        test_ctx->qserver = picoquic_create(8,
            test_server_cert_file, test_server_key_file, test_server_cert_store_file,
            PICOQUIC_TEST_ALPN, test_api_callback, (void*)&test_ctx->server_callback, NULL, NULL, NULL,
            simulated_time, &simulated_time, NULL,
            test_ticket_encrypt_key, sizeof(test_ticket_encrypt_key));

        if (test_ctx->qserver == NULL) {
            ret = -1;
        }
    }

    /* Proceed with the connection loop. It should fail, and thus we don't test the return code */
    if (ret == 0) {
        (void)tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_local_error(test_ctx->cnx_client))) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_remote_error(test_ctx->cnx_server))) {
            ret = -1;
        }
        else {
            ret = 0;
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
fn bad_certificate() {
    tls_api_test_with_loss(None, V1, Some("bad.example.com"), Some(TEST_ALPN))
        .expect("bad_certificate");
}
```

## `picoquictest/tls_api_test.c:client_only_test`
* C test-table name: `client_only`
* C entry function: `client_only_test`
* Rust test: `client_only`
* C source: `picoquictest/tls_api_test.c:6404-6440`
* Rust source: `rs/fq/src/tests/tls_api.rs:179-181`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xc1, 0x10, 0, 0, 0, 0, 0, 0}, 8 };
    int ret;

    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0) {
        /* First, try enforcement. We do set log on the server side, but it is expected to be empty */
        int connection_ret = 0;
        picoquic_enforce_client_only(test_ctx->qserver, 1);
        picoquic_set_qlog(test_ctx->qserver, ".");
        picoquic_set_qlog(test_ctx->qclient, ".");
        connection_ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
        if (connection_ret == 0 && test_ctx->cnx_client->cnx_state < picoquic_state_disconnected) {
            DBG_PRINTF("Connection unexpectedly succeeds, state=%d, ret=%d (0x%x)",
                test_ctx->cnx_client->cnx_state, connection_ret, connection_ret);
            ret = -1;
        }
        else if (test_ctx->cnx_server != NULL) {
            DBG_PRINTF("Connection context created on client-only note, ret=%d (0x%x)",
                connection_ret, connection_ret);
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
fn client_only() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("client_only");
}
```

## `picoquictest/tls_api_test.c:ddos_amplification_test`
* C test-table name: `ddos_amplification`
* C entry function: `ddos_amplification_test`
* Rust test: `ddos_amplification`
* C source: `picoquictest/tls_api_test.c:10410-10413`
* Rust source: `rs/fq/src/tests/tls_api.rs:254-256`

### C test body
```c
{
    return ddos_amplification_test_one(0, 0);
}
```

### Rust test body
```rust
fn ddos_amplification() {
    ddos_amplification_test_one(0, 0).expect("ddos_amplification");
}
```

## `picoquictest/tls_api_test.c:excess_repeat_test`
* C test-table name: `excess_repeat`
* C entry function: `excess_repeat_test`
* Rust test: `excess_repeat`
* C source: `picoquictest/tls_api_test.c:11593-11615`
* Rust source: `rs/fq/src/tests/tls_api.rs:326-328`

### C test body
```c
{
    const int nb_repeat_max = 128;

    picoquic_congestion_algorithm_t* algo_list[6] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr_algorithm,
        picoquic_prague_algorithm
    };
    int ret = 0;

    for (int i = 0; i < 6 && ret == 0; i++) {
        ret = excess_repeat_test_one(algo_list[i], nb_repeat_max);
        if (ret != 0) {
            DBG_PRINTF("Excess repeat test fails for CC=%s", algo_list[i]->congestion_algorithm_id);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn excess_repeat() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("excess_repeat");
}
```
