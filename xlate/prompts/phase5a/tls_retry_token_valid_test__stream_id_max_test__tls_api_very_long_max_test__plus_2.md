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

## `picoquictest/tls_api_test.c:tls_retry_token_valid_test`
* C test-table name: `retry_token_valid`
* C entry function: `tls_retry_token_valid_test`
* Rust test: `retry_token_valid`
* C source: `picoquictest/tls_api_test.c:3865-4010`
* Rust source: `rs/fq/src/tests/tls_api.rs:1155-1157`

### C test body
```c
{
    int ret = 0;
    int is_new_token = 0;
    struct sockaddr_in addr1, addr2, addr3;
    struct sockaddr* addr[3];
    picoquic_connection_id_t n_cid = picoquic_null_connection_id;
    picoquic_connection_id_t cid1 = { { 1,1,1,1,1,1,1,1}, 8 };
    picoquic_connection_id_t cid2 = { { 2,2,2,2,2,2,2,2,2}, 9 };
    picoquic_connection_id_t* cid[3];
    picoquic_connection_id_t cid_o = { { 3,3,3,3,3,3,3,3}, 8 };
    picoquic_connection_id_t* odcid[2];
    picoquic_connection_id_t odcid_found;
    uint64_t time_base = 10000;
    uint64_t time_delta[4] = { 0, 0, PICOQUIC_TOKEN_DELAY_SHORT + 1,
        PICOQUIC_TOKEN_DELAY_LONG + 1000000 };
    uint32_t pn[3] = { 0, 1, 2 };
    uint8_t token_buffer[128];
    size_t token_size;
    int verified;
    picoquic_quic_t * quic = picoquic_create(8, NULL, NULL, NULL, PICOQUIC_TEST_ALPN, NULL, NULL, NULL, NULL, NULL,
        time_base*1000000, NULL, NULL, test_ticket_encrypt_key, sizeof(test_ticket_encrypt_key));

    if (quic == NULL) {
        return -1;
    }

    picoquic_set_test_address(&addr1, 0x01010101, 1234);
    picoquic_set_test_address(&addr2, 0x01010101, 3456);
    picoquic_set_test_address(&addr3, 0x03030303, 1234);
    addr[0] = (struct sockaddr*) & addr1;
    addr[1] = (struct sockaddr*) & addr2;
    addr[2] = (struct sockaddr*) & addr3;
    cid[0] = &cid1;
    cid[1] = &n_cid;
    cid[2] = &cid2;
    odcid[0] = &cid_o;
    odcid[1] = &n_cid;

    /* Test of a connection token
     * - Create a token with test address, time1, cid1, pn1.
     * - Check with time-0 (valid), time1(valid), time2(invalid)
     * - Check with addr1,port1 (valid), addr1,port2 (valid), addr2,port1(invalid)
     * - Check with pn2 (valid), pn1(invalid)
     * - check with cid1 (valid), cid2(invalid)
     */

     /* Test of a new token
      * - Create a token with test address, time1, n_cid, pn1.
      * - Check with time-0 (valid), time1(valid), time2(invalid)
      * - Check with addr1,port1 (valid), addr1,port2 (valid), addr2,port1(invalid)
      * - Check with pn2 (valid), pn1(valid)
      * - check with cid1 (valid), cid2(valid)
      */

    /* Test of an invalid token: valid token with changed bytes */

    for (int token_mode = 0; ret == 0 && token_mode < 2; token_mode++) {
        if (picoquic_prepare_retry_token(quic, addr[0], time_base * 1000000 + time_delta[1], odcid[token_mode],
            cid[token_mode], pn[1],
            token_buffer, sizeof(token_buffer), &token_size) != 0) {
            ret = PICOQUIC_ERROR_MEMORY;
        }

        if (ret == 0) {
            verified = picoquic_verify_retry_token(quic, addr[0], time_base * 1000000 + time_delta[0],
                &is_new_token, &odcid_found, cid[0], pn[2],
                token_buffer, token_size, 0);
            if (verified != 0) {
                DBG_PRINTF("%s", "Token validation fails for normal parameters\n");
                ret = -1;
            }
            else if (token_mode == 0 && picoquic_compare_connection_id(odcid[0], &odcid_found) != 0) {
                DBG_PRINTF("%s", "ODCID validation fails\n");
                ret = -1;
            }
            else if (token_mode == 1 && odcid_found.id_len > 0) {
                DBG_PRINTF("%s", "Spurious ODCID\n");
                ret = -1;
            }
            else if (!is_new_token && odcid[token_mode]->id_len == 0) {
                DBG_PRINTF("%s", "Wrongly recognized as new token\n");
                ret = -1;
            }
            else if (is_new_token && odcid[token_mode]->id_len > 0) {
                DBG_PRINTF("%s", "Wrongly recognized as retry token\n");
                ret = -1;
            }
        }

        if (ret == 0 && picoquic_verify_retry_token(quic, addr[0], time_base * 1000000 + time_delta[2 + token_mode],
            &is_new_token, &odcid_found, cid[0], pn[2],
            token_buffer, token_size, 0) == 0) {
            DBG_PRINTF("%s", "Token validation does not detect elapsed time.\n");
            ret = -1;
        }

        if (ret == 0) {
            verified = picoquic_verify_retry_token(quic, addr[0], time_base * 1000000 + time_delta[0],
                &is_new_token, &odcid_found, cid[2], pn[2],
                token_buffer, token_size, 0);
            if (token_mode == 0 && verified == 0) {
                DBG_PRINTF("%s", "RCID invalidation fails\n");
                ret = -1;
            }
            else if (token_mode == 1 && verified != 0) {
                DBG_PRINTF("%s", "Spurious RCID invalidation\n");
                ret = -1;
            }
        }

        for (int pn_id = 0; ret == 0 && pn_id < 2; pn_id++) {
            verified = picoquic_verify_retry_token(quic, addr[0], time_base * 1000000 + time_delta[0],
                &is_new_token, &odcid_found, cid[0], pn[pn_id],
                token_buffer, token_size, 0);
            if (token_mode == 0 && verified == 0) {
                DBG_PRINTF("%s", "PN invalidation fails\n");
                ret = -1;
            }
            else if (token_mode == 1 && verified != 0) {
                DBG_PRINTF("%s", "Spurious PN invalidation\n");
                ret = -1;
            }
        }

        /* test of an invalid token, overly long */
        if (ret == 0 && token_mode == 0) {
            uint8_t big_token[PICOQUIC_MAX_PACKET_SIZE];
            if (token_size < sizeof(big_token)) {
                memcpy(big_token, token_buffer, token_size);
                memset(big_token + token_size, 0xa5, sizeof(big_token) - token_size);
                verified = picoquic_verify_retry_token(quic, addr[0], time_base * 1000000 + time_delta[0],
                    &is_new_token, &odcid_found, cid[0], pn[2],
                    token_buffer, sizeof(big_token), 0);
                if (verified == 0) {
                    DBG_PRINTF("%s", "Bad length check fails\n");
                    ret = -1;
                }
            }
        }
    }

    picoquic_free(quic);
    return ret;
}
```

### Rust test body
```rust
fn retry_token_valid() {
    tls_retry_token_test_one(2, false).expect("retry_token_valid");
}
```

## `picoquictest/tls_api_test.c:stream_id_max_test`
* C test-table name: `stream_id_max`
* C entry function: `stream_id_max_test`
* Rust test: `stream_id_max`
* C source: `picoquictest/tls_api_test.c:8546-8556`
* Rust source: `rs/fq/src/tests/tls_api.rs:1311-1315`

### C test body
```c
{
    picoquic_tp_t test_parameters;

    memset(&test_parameters, 0, sizeof(picoquic_tp_t));

    picoquic_init_transport_parameters(&test_parameters);
    test_parameters.initial_max_stream_id_bidir = 4;

    return tls_api_one_scenario_test(test_scenario_many_streams, sizeof(test_scenario_many_streams), 0, 0, 0, 0, 0, 250000, NULL, &test_parameters);
}
```

### Rust test body
```rust
fn stream_id_max() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_000_000).expect("stream_id_max");
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

## `picoquictest/tls_api_test.c:tls_api_version_negotiation_test`
* C test-table name: `version_negotiation`
* C entry function: `tls_api_version_negotiation_test`
* Rust test: `version_negotiation`
* C source: `picoquictest/tls_api_test.c:2541-2579`
* Rust source: `rs/fq/src/tests/tls_api.rs:1507-1509`

### C test body
```c
{
    const uint32_t version_grease = 0x0aca4a0a;
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, version_grease, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", version_grease);
    }

    if (ret == 0) {
        (void)tls_api_connection_loop(test_ctx, NULL, 0, &simulated_time);

        if (test_ctx->cnx_client->cnx_state == picoquic_state_disconnected) {
            ret = 0;
        }
        else {
            DBG_PRINTF("Unexpected state: %d\n", test_ctx->cnx_client->cnx_state);
            ret = -1;
        }
    }


    if (ret == 0) {
        if (!test_ctx->received_version_negotiation){
            DBG_PRINTF("%s", "No version negotiation notified\n");
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
fn version_negotiation() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("version_negotiation");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_long_test`
* C test-table name: `zero_rtt_long`
* C entry function: `zero_rtt_long_test`
* Rust test: `zero_rtt_long`
* C source: `picoquictest/tls_api_test.c:4740-4745`
* Rust source: `rs/fq/src/tests/tls_api.rs:1590-1596`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.long_data = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt_long() {
    zero_rtt_test_one(&ZeroRttTest {
        long_data: true,
        ..Default::default()
    })
    .expect("zero_rtt_long");
}
```
