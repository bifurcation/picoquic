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

## `picoquictest/tls_api_test.c:set_certificate_and_key_test`
* C test-table name: `set_certificate_and_key`
* C entry function: `set_certificate_and_key_test`
* Rust test: `set_certificate_and_key`
* C source: `picoquictest/tls_api_test.c:5568-5657`
* Rust source: `rs/fq/src/tests/tls_api.rs:1202-1205`

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
        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_CERT);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }

    /* Delete the server context, and recreate it. */
    if (ret == 0)
    {
        if (test_ctx->qserver != NULL) {
            picoquic_free(test_ctx->qserver);
        }

        test_ctx->qserver = picoquic_create(8,
            NULL, NULL, NULL,
            PICOQUIC_TEST_ALPN, test_api_callback, (void*)&test_ctx->server_callback, NULL, NULL, NULL,
            simulated_time, &simulated_time, NULL,
            test_ticket_encrypt_key, sizeof(test_ticket_encrypt_key));

        if (test_ctx->qserver == NULL) {
            ret = -1;
        }

        if (ret == 0) {
            ret = picoquic_set_private_key_from_file(test_ctx->qserver, test_server_key_file);
        }

        if (ret == 0) {
            size_t count = 0;
            ptls_iovec_t* chain = picoquic_get_certs_from_file(test_server_cert_file, &count);
            if (chain == NULL) {
                ret = -1;
            } else {
                picoquic_set_tls_certificate_chain(test_ctx->qserver, chain, count);
            }
        }

        if (ret == 0) {
            size_t count = 0;
            ptls_iovec_t* chain = picoquic_get_certs_from_file(test_server_cert_store_file, &count);

            if (chain == NULL) {
                ret = -1;
            } else {
                picoquic_set_tls_root_certificates(test_ctx->qserver, chain, count);
                for (size_t i = 0; i < count; i++) {
                    free(chain[i].base);
                }
                free(chain);
            }
        }
    }

    /* Proceed with the connection loop. */
    if (ret == 0) {
        picoquic_enforce_client_only(test_ctx->qserver, 0);
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0 && (!TEST_CLIENT_READY || !TEST_SERVER_READY)) {
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
fn set_certificate_and_key() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("set_certificate_and_key");
}
```

## `picoquictest/tls_api_test.c:tls_api_very_long_stream_test`
* C test-table name: `tls_api_very_long_stream`
* C entry function: `tls_api_very_long_stream_test`
* Rust test: `tls_api_very_long_stream`
* C source: `picoquictest/tls_api_test.c:3281-3284`
* Rust source: `rs/fq/src/tests/tls_api.rs:1424-1429`

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 0, 0, 0, 1000000, NULL, NULL);
}
```

### Rust test body
```rust
fn tls_api_very_long_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 1_000_000)
        .expect("very_long_stream");
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

## `picoquictest/tls_api_test.c:zero_rtt_no_coal_test`
* C test-table name: `zero_rtt_no_coal`
* C entry function: `zero_rtt_no_coal_test`
* Rust test: `zero_rtt_no_coal`
* C source: `picoquictest/tls_api_test.c:4697-4702`
* Rust source: `rs/fq/src/tests/tls_api.rs:1637-1643`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.no_coal = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt_no_coal() {
    zero_rtt_test_one(&ZeroRttTest {
        no_coal: true,
        ..Default::default()
    })
    .expect("zero_rtt_no_coal");
}
```

## `picoquictest/util_test.c:util_connection_id_print_test`
* C test-table name: `connection_id_print`
* C entry function: `util_connection_id_print_test`
* Rust test: `connection_id_print`
* C source: `picoquictest/util_test.c:46-71`
* Rust source: `rs/fq/src/tests/util_test.rs:26-36`

### C test body
```c
{
    int ret = 0;  
    char cnxid_str[2 * PICOQUIC_CONNECTION_ID_MAX_SIZE + 1];
  
    for (size_t i = 0; i < test_cases; ++i)
    {
        int result = picoquic_print_connection_id_hexa(cnxid_str, sizeof(cnxid_str), &expected_cnxid[i]);
        if (result != 0) {
            DBG_PRINTF("picoquic_print_connection_id_hexa failed with: %d\n", result);
            ret = -1;
        }
        if (strcmp(cnxid_str, expected_str[i]) != 0) {
            DBG_PRINTF("result: %s, expected: %s\n", cnxid_str, expected_str[i]);
            ret = -1;
        }
    }

    // Test invalid call
    if (picoquic_print_connection_id_hexa("", 0, &expected_cnxid[0]) == 0) {
        DBG_PRINTF("%s", "picoquic_print_connection_id_hexa did not fail\n");
        ret = -1;
    }

    return ret;
}
```

### Rust test body
```rust
fn connection_id_print() {
    use core::fmt::Write;
    for (bytes, expected) in EXPECTED_CIDS {
        let cid = ConnectionId::clone_from_slice(bytes).expect("CID under cap");
        let mut got = String::new();
        for b in cid.as_bytes() {
            write!(&mut got, "{b:02x}").unwrap();
        }
        assert_eq!(got, *expected, "CID hex round-trip");
    }
}
```
