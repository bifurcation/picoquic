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

## `picoquictest/tls_api_test.c:document_addresses_test`
* C test-table name: `document_addresses`
* C entry function: `document_addresses_test`
* Rust test: `document_addresses`
* C source: `picoquictest/tls_api_test.c:9914-9966`
* Rust source: `rs/fq/src/tests/tls_api.rs:309-311`

### C test body
```c
{
    uint64_t simulated_time = 0; 
    tls_api_address_are_documented_t client_address_callback_ctx, server_address_callback_ctx;

    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }
    if (ret == 0) {
        /* Set the call backs to intercept the almost ready and ready transitions */
        memset(&client_address_callback_ctx, 0, sizeof(tls_api_address_are_documented_t));
        client_address_callback_ctx.callback_fn = picoquic_get_callback_function(test_ctx->cnx_client);
        client_address_callback_ctx.callback_ctx = picoquic_get_callback_context(test_ctx->cnx_client);
        picoquic_set_callback(test_ctx->cnx_client, test_local_address_callback, &client_address_callback_ctx);

        memset(&server_address_callback_ctx, 0, sizeof(tls_api_address_are_documented_t));
        server_address_callback_ctx.callback_fn = picoquic_get_default_callback_function(test_ctx->qserver);
        server_address_callback_ctx.callback_ctx = picoquic_get_default_callback_context(test_ctx->qserver);
        picoquic_set_default_callback(test_ctx->qserver, test_local_address_callback, &server_address_callback_ctx);

        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 20000, 3600000);
    }

    /* Verify that the addresses and calls are what we expect */
    if (ret == 0) {
        ret = document_addresses_check(&client_address_callback_ctx,
            (struct sockaddr*)&test_ctx->client_addr, (struct sockaddr*)&test_ctx->server_addr);
        if (ret != 0) {
            DBG_PRINTF("%s", "Client addresses were not properly documented\n");
        }
    }

    if (ret == 0) {
        ret = document_addresses_check(&server_address_callback_ctx,
            (struct sockaddr*)&test_ctx->server_addr, (struct sockaddr*)&test_ctx->client_addr);
        if (ret != 0) {
            DBG_PRINTF("%s", "Server addresses were not properly documented\n");
        }
    }

    /* Free the resource */

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn document_addresses() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("document_addresses");
}
```

## `picoquictest/tls_api_test.c:many_short_loss_test`
* C test-table name: `many_short_loss`
* C entry function: `many_short_loss_test`
* Rust test: `many_short_loss`
* C source: `picoquictest/tls_api_test.c:3336-3339`
* Rust source: `rs/fq/src/tests/tls_api.rs:566-568`

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_more_streams, sizeof(test_scenario_more_streams), 0, 0x882818A881288848ull, 16000, 2000, 0, 0, NULL, NULL);
}
```

### Rust test body
```rust
fn many_short_loss() {
    tls_api_loss_test(0x882818A881288848u64).expect("many_short_loss");
}
```

## `picoquictest/tls_api_test.c:mtu_drop_bbr_test`
* C test-table name: `mtu_drop_bbr`
* C entry function: `mtu_drop_bbr_test`
* Rust test: `mtu_drop_bbr`
* C source: `picoquictest/tls_api_test.c:5090-5097`
* Rust source: `rs/fq/src/tests/tls_api.rs:650-652`

### C test body
```c
{
    /* TODO: the time with BBR v1 was 10300000. The current value is
     * a slight regression. Investigate whether some performance
     * for BBR3 "recover from PTO" could be improved. */
    int ret = mtu_drop_cc_algotest(picoquic_bbr_algorithm, 10700000);
    return ret;
}
```

### Rust test body
```rust
fn mtu_drop_bbr() {
    mtu_drop_cc_algotest("bbr", 10_700_000).expect("mtu_drop_bbr");
}
```

## `picoquictest/tls_api_test.c:tls_api_multiple_versions_test`
* C test-table name: `multiple_versions`
* C entry function: `tls_api_multiple_versions_test`
* Rust test: `multiple_versions`
* C source: `picoquictest/tls_api_test.c:4180-4193`
* Rust source: `rs/fq/src/tests/tls_api.rs:719-724`

### C test body
```c
{
    int ret = 0;

    for (size_t i = 1; ret == 0 && i < picoquic_nb_supported_versions; i++) {
        ret = tls_api_one_scenario_test(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0,
            picoquic_supported_versions[i].version, 0, NULL, NULL);
    }

    return ret;
}
```

### Rust test body
```rust
fn multiple_versions() {
    for ver in [V1, 0xFF00_0020u32, 0xFF00_0013u32] {
        tls_api_test_with_loss(None, ver, Some(TEST_SNI), Some(TEST_ALPN))
            .unwrap_or_else(|e| panic!("multiple_versions ver={ver:#x}: {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:new_rotated_key_test`
* C test-table name: `new_rotated_key`
* C entry function: `new_rotated_key_test`
* Rust test: `new_rotated_key`
* C source: `picoquictest/tls_api_test.c:7650-7727`
* Rust source: `rs/fq/src/tests/tls_api.rs:789-791`

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


    for (int i = 1; ret == 0 && i <= 3; i++) {
        
        /* Try to compute rotated keys on server */
        ret = picoquic_compute_new_rotated_keys(test_ctx->cnx_server);
        if (ret != 0) {
            DBG_PRINTF("Could not rotate server key, ret: %x\n", ret);
        } else {
            /* Try to compute rotated keys on client */
            ret = picoquic_compute_new_rotated_keys(test_ctx->cnx_client);
            if (ret != 0) {
                DBG_PRINTF("Could not rotate client key, round %d, ret: %x\n", i, ret);
            }
        }

        if (ret == 0)
        {
            /* Compare server encryption and client decryption */
            size_t key_size = picoquic_get_app_secret_size(test_ctx->cnx_client);

            if (key_size != picoquic_get_app_secret_size(test_ctx->cnx_server)) {
                DBG_PRINTF("Round %d. Key sizes dont match, client: %d, server: %d\n", i, key_size, picoquic_get_app_secret_size(test_ctx->cnx_server));
                ret = -1;
            }
            else if (memcmp(picoquic_get_app_secret(test_ctx->cnx_server, 1), picoquic_get_app_secret(test_ctx->cnx_client, 0), key_size) != 0) {
                DBG_PRINTF("Round %d. Server encryption secret does not match client decryption secret\n", i);
                ret = -1;
            }
            else if (memcmp(picoquic_get_app_secret(test_ctx->cnx_server, 0), picoquic_get_app_secret(test_ctx->cnx_client, 1), key_size) != 0) {
                DBG_PRINTF("Round %d. Server decryption secret does not match client encryption secret\n", i);
                ret = -1;
            }
            else if (aead_iv_check(test_ctx->cnx_server->crypto_context_new.aead_encrypt, test_ctx->cnx_client->crypto_context_new.aead_decrypt) != 0) {
                DBG_PRINTF("Round %d. Client AEAD decryption does not match server AEAD encryption.\n", i);
                ret = -1;
            }
            else if (aead_iv_check(test_ctx->cnx_client->crypto_context_new.aead_encrypt, test_ctx->cnx_server->crypto_context_new.aead_decrypt) != 0) {
                DBG_PRINTF("Round %d. Server AEAD decryption does not match cliens AEAD encryption.\n", i);
                ret = -1;
            }
#if 0
            else if (pn_enc_check(test_ctx->cnx_server->crypto_context_new.pn_enc, test_ctx->cnx_client->crypto_context_new.pn_dec) != 0) {
                DBG_PRINTF("Round %d. Client PN decryption does not match server PN encryption.\n", i);
                ret = -1;
            }
            else if (pn_enc_check(test_ctx->cnx_client->crypto_context_new.pn_enc, test_ctx->cnx_server->crypto_context_new.pn_dec) != 0) {
                DBG_PRINTF("Round %d. Server PN decryption does not match client PN encryption.\n", i);
                ret = -1;
            }
#endif
        }

        picoquic_crypto_context_free(&test_ctx->cnx_server->crypto_context_new);
        picoquic_crypto_context_free(&test_ctx->cnx_client->crypto_context_new);
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
fn new_rotated_key() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("new_rotated_key");
}
```
