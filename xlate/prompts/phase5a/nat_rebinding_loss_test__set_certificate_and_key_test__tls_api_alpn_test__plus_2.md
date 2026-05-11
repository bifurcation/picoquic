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

## `picoquictest/tls_api_test.c:nat_rebinding_loss_test`
* C test-table name: `nat_rebinding_loss`
* C entry function: `nat_rebinding_loss_test`
* Rust test: `nat_rebinding_loss`
* C source: `picoquictest/tls_api_test.c:6079-6084`
* Rust source: `rs/fq/src/tests/tls_api.rs:763-765`

### C test body
```c
{
    uint64_t loss_mask = 0x2012;

    return nat_rebinding_test_one(loss_mask, 0, 0);
}
```

### Rust test body
```rust
fn nat_rebinding_loss() {
    nat_rebinding_test_one(0x2012, false, 0).expect("nat_rebinding_loss");
}
```

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

## `picoquictest/tls_api_test.c:tls_api_alpn_test`
* C test-table name: `tls_api_alpn`
* C entry function: `tls_api_alpn_test`
* Rust test: `tls_api_alpn`
* C source: `picoquictest/tls_api_test.c:2976-2991`
* Rust source: `rs/fq/src/tests/tls_api.rs:1329-1331`

### C test body
```c
{
    int ret = tls_api_test_with_loss(NULL, 0, PICOQUIC_TEST_SNI, NULL);

    if (ret == PICOQUIC_ERROR_NO_ALPN_PROVIDED) {
        ret = 0;
    } else if (ret == 0) {
        DBG_PRINTF("ALPN test succeeds while no ALPN is specified, ret = 0x%x", ret);
        ret = -1;
    }
    else {
        DBG_PRINTF("ALPN test does not return expected error code, ret = 0x%x", ret);
        ret = -1;
    }
    return ret;
}
```

### Rust test body
```rust
fn tls_api_alpn() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), None).expect("tls_api_alpn");
}
```

## `picoquictest/tls_api_test.c:tls_api_very_long_max_test`
* C test-table name: `tls_api_very_long_max`
* C entry function: `tls_api_very_long_max_test`
* Rust test: `tls_api_very_long_max`
* C source: `picoquictest/tls_api_test.c:3286-3289`
* Rust source: `rs/fq/src/tests/tls_api.rs:1414-1418`

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 128000, 0, 0, 1000000, NULL, NULL);
}
```

### Rust test body
```rust
fn tls_api_very_long_max() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 1_000_000).expect("very_long_max");
}
```

## `picoquictest/tls_api_test.c:tls_api_version_invariant_test`
* C test-table name: `version_invariant`
* C entry function: `tls_api_version_invariant_test`
* Rust test: `version_invariant`
* C source: `picoquictest/tls_api_test.c:2666-2729`
* Rust source: `rs/fq/src/tests/tls_api.rs:1498-1500`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret != 0)
    {
        DBG_PRINTF("%s", "Could not create the QUIC test contexts");
    }
    else {
        /* Fabricate a packet that stresses the invariants */
        uint8_t packet[PICOQUIC_MAX_PACKET_SIZE];
        /* Initialize first byte to long header value*/
        packet[0] = 0xF5;
        /* Set version to unexpected value */
        memset(packet + 1, 0xaa, 4);
        /* Set destination CID to 255 times "dd" */
        packet[6] = 255;
        memset(packet + 7, 0xdd, 255);
        /* Set source CID to 127 times "cc" */
        packet[262] = 127;
        memset(packet + 263, 0xcc, 127);
        /* Set reminder of packet to 0x55*/
        memset(packet + 290, 0x55, PICOQUIC_MAX_PACKET_SIZE - 290);

        /* Submit the packet to the server */
        ret = picoquic_incoming_packet(test_ctx->qserver, packet, PICOQUIC_MAX_PACKET_SIZE,
            (struct sockaddr*) & test_ctx->client_addr, (struct sockaddr*) & test_ctx->server_addr,
            0, 0, simulated_time);
        if (ret != 0) {
            DBG_PRINTF("incoming invariant test returns %d (0x%x)", ret, ret);
        }
        else {
            uint8_t response[PICOQUIC_MAX_PACKET_SIZE];
            struct sockaddr_storage addr_to;
            struct sockaddr_storage addr_from;
            size_t send_length = 0;
            int if_index = 0;
            picoquic_connection_id_t log_cid;
            picoquic_cnx_t* last_cnx = NULL;

            ret = picoquic_prepare_next_packet(test_ctx->qserver, simulated_time, response, PICOQUIC_MAX_PACKET_SIZE,
                &send_length, &addr_to, &addr_from, &if_index, &log_cid, &last_cnx);
            if (ret != 0) {
                DBG_PRINTF("Invariant response test returns %d (0x%x)", ret, ret);
            }
            else if (send_length == 0) {
                DBG_PRINTF("%s", "Invariant response test does not return any data");
                ret = -1;
            }
            else {
                ret = check_vn_invariant(packet, PICOQUIC_MAX_PACKET_SIZE, response, send_length);
            }
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
fn version_invariant() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("version_invariant");
}
```
