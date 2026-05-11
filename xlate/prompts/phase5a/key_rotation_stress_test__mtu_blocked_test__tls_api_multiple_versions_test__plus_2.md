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

## `picoquictest/tls_api_test.c:key_rotation_stress_test`
* C test-table name: `key_rotation_stress`
* C entry function: `key_rotation_stress_test`
* Rust test: `key_rotation_stress`
* C source: `picoquictest/tls_api_test.c:8099-8102`
* Rust source: `rs/fq/src/tests/tls_api.rs:506-508`

### C test body
```c
{
    return key_rotation_stress_test_one(10);
}
```

### Rust test body
```rust
fn key_rotation_stress() {
    key_rotation_stress_test_one(10).expect("key_rotation_stress");
}
```

## `picoquictest/tls_api_test.c:mtu_blocked_test`
* C test-table name: `mtu_blocked`
* C entry function: `mtu_blocked_test`
* Rust test: `mtu_blocked`
* C source: `picoquictest/tls_api_test.c:4981-4986`
* Rust source: `rs/fq/src/tests/tls_api.rs:625-627`

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_blocked, 1252, 1252, 
        test_scenario_mtu_discovery, sizeof(test_scenario_mtu_discovery), 0);
    return ret;
}
```

### Rust test body
```rust
fn mtu_blocked() {
    mtu_discovery_test_one(1, 1252, 1252, 10_000_000, 0).expect("mtu_blocked");
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

## `picoquictest/tls_api_test.c:padding_null_test`
* C test-table name: `padding_null`
* C entry function: `padding_null_test`
* Rust test: `padding_null`
* C source: `picoquictest/tls_api_test.c:8736-8739`
* Rust source: `rs/fq/src/tests/tls_api.rs:860-862`

### C test body
```c
{
    return padding_test_one(0, 0);
}
```

### Rust test body
```rust
fn padding_null() {
    padding_test_one(0, 0).expect("padding_null");
}
```
