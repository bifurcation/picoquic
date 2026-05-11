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

## `picoquictest/sockloop_test.c:sockloop_thread_test`
* C test-table name: `sockloop_thread`
* C entry function: `sockloop_thread_test`
* Rust test: `sockloop_thread`
* C source: `picoquictest/sockloop_test.c:710-720`
* Rust source: `rs/fq/src/tests/sockloop.rs:618-624`

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 7);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.use_background_thread = 1;

    return(sockloop_test_one(&spec));
}
```

### Rust test body
```rust
fn sockloop_thread() {
    let mut spec = SockloopTestSpec::new(7);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.use_background_thread = true;
    sockloop_test_one(&spec);
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

## `picoquictest/tls_api_test.c:cnx_ddos_unit_test`
* C test-table name: `cnx_ddos`
* C entry function: `cnx_ddos_unit_test`
* Rust test: `cnx_ddos`
* C source: `picoquictest/tls_api_test.c:11806-11809`
* Rust source: `rs/fq/src/tests/tls_api.rs:188-190`

### C test body
```c
{
    return cnx_ddos_test_loop(1000, 1000, NULL);
}
```

### Rust test body
```rust
fn cnx_ddos() {
    cnx_ddos_test_loop(1000, 1000).expect("cnx_ddos");
}
```

## `picoquictest/tls_api_test.c:ddos_amplification_8k_test`
* C test-table name: `ddos_amplification_8k`
* C entry function: `ddos_amplification_8k_test`
* Rust test: `ddos_amplification_8k`
* C source: `picoquictest/tls_api_test.c:10420-10423`
* Rust source: `rs/fq/src/tests/tls_api.rs:270-272`

### C test body
```c
{
    return ddos_amplification_test_one(0, 1);
}
```

### Rust test body
```rust
fn ddos_amplification_8k() {
    ddos_amplification_test_one(2, 0).expect("ddos_amplification_8k");
}
```

## `picoquictest/tls_api_test.c:tls_api_client_first_loss_test`
* C test-table name: `first_loss`
* C entry function: `tls_api_client_first_loss_test`
* Rust test: `first_loss`
* C source: `picoquictest/tls_api_test.c:4155-4158`
* Rust source: `rs/fq/src/tests/tls_api.rs:343-345`

### C test body
```c
{
    return tls_api_loss_test(1ull);
}
```

### Rust test body
```rust
fn first_loss() {
    tls_api_loss_test(1).expect("first_loss");
}
```
