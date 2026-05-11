# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/sockloop_test.c:sockloop_basic_test`
* C test-table name: `sockloop_basic`
* C entry function: `sockloop_basic_test`
* Rust test: `sockloop_basic`
* Expected Rust file: `rs/fq/src/tests/sockloop.rs`
* Current Rust span: `rs/fq/src/tests/sockloop.rs:970-975`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry sets the same ipv6_only and do_not_use_gso flags, but SockloopTestSpec::new uses socket_buffer_size 1280 while C sockloop_test_set_spec uses PICOQUIC_MAX_PACKET_SIZE, which is 1536. That changes the test input.
* Phase 5A fix note: Set SockloopTestSpec::new socket_buffer_size to crate::MAX_PACKET_SIZE as i32, or explicitly set it in sockloop_basic, so it matches C PICOQUIC_MAX_PACKET_SIZE.
* Phase 5B analysis: Rust helper default now matches C sockloop_test_set_spec: socket_buffer_size uses PICOQUIC_MAX_PACKET_SIZE via crate::MAX_PACKET_SIZE; sockloop_basic already matched the remaining C field overrides.
* Phase 5B fix note: Changed SockloopTestSpec::new default socket_buffer_size from 1280 to crate::MAX_PACKET_SIZE as i32.

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 1);
    spec.ipv6_only = 1;
    spec.do_not_use_gso = 1;

    return(sockloop_test_one(&spec));
}
```

### Current Rust test body
```rust
fn sockloop_basic() {
    let mut spec = SockloopTestSpec::new(1);
    spec.ipv6_only = true;
    spec.do_not_use_gso = true;
    sockloop_test_one(&spec);
}
```

## `picoquictest/tls_api_test.c:get_hash_test`
* C test-table name: `get_hash`
* C entry function: `get_hash_test`
* Rust test: `get_hash`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2662-2700`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs tls_api_test_with_loss, a generic handshake/close helper; it never exercises hash length lookup, valid/invalid algorithm lookup, hash creation, update/finalize, or the nonzero digest check from C.
* Phase 5A fix note: Replace with direct hash API checks for sha256 and no_such_hash_nada_niente using hash_get_length and hash_create/get_hash_algorithm_by_name, including invalid-name failure and nonzero digest validation.
* Phase 5B analysis: Rust test was checking a generic handshake instead of the C hash API behavior.
* Phase 5B fix note: Replaced get_hash with direct sha256 and invalid-name checks for hash_get_length, get_hash_algorithm_by_name, hash_create, digest update/finalize, and nonzero digest prefix validation.

### C test body
```c
{
    int ret = 0;
    char const* valid_hash = "sha256";
    char const* invalid_hash = "no_such_hash_nada_niente";

    picoquic_tls_api_init();
    if (get_hash_length_test(valid_hash) != 0) {
        ret = -1;
    }
    else if (get_hash_length_test(invalid_hash) == 0) {
        ret = -1;
    }
    else if (get_hash_algo_test(valid_hash) != 0) {
        ret = -1;
    }
    else if (get_hash_algo_test(invalid_hash) == 0) {
        ret = -1;
    }
    return (ret);
}
```

### Current Rust test body
```rust
fn get_hash() {
    let valid_hash = "sha256";
    let invalid_hash = "no_such_hash_nada_niente";

    let hash_length = hash_get_length(valid_hash);
    assert!(
        hash_length > 0 && hash_length <= 1024,
        "valid hash length should fit the C test scratch buffer"
    );
    assert_eq!(hash_length, 32, "sha256 digest length");
    assert_eq!(
        hash_get_length(invalid_hash),
        0,
        "invalid hash names should not resolve to a digest length"
    );

    let algorithm = get_hash_algorithm_by_name(valid_hash).expect("valid hash algorithm lookup");
    assert_eq!(algorithm.output_size(), hash_length);
    assert!(
        get_hash_algorithm_by_name(invalid_hash).is_none(),
        "invalid hash algorithm lookup should fail"
    );

    let mut hash = hash_create(valid_hash).expect("valid hash creation");
    assert_eq!(hash.output_size(), hash_length);
    assert!(
        hash_create(invalid_hash).is_none(),
        "invalid hash creation should fail"
    );

    let mut outbuf = [0u8; 1024];
    hash.update(&[1, 2, 3, 4]);
    hash.finalize_into_reset(&mut outbuf[..hash_length])
        .expect("finalize sha256 digest");
    assert!(
        outbuf[..16].iter().any(|&b| b != 0),
        "digest prefix should not remain all zero"
    );
}
```

## `picoquictest/tls_api_test.c:tls_retry_token_valid_test`
* C test-table name: `retry_token_valid`
* C entry function: `tls_retry_token_valid_test`
* Rust test: `retry_token_valid`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7071-7224`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a cookie-mode connection helper and does not directly validate retry/new token contents, ODCID/is_new_token classification, expiry, RCID/PN mismatch behavior, or oversized-token rejection from the C matrix.
* Phase 5A fix note: Add a direct token validation test using prepare_retry_token/verify_retry_token that covers both retry and new-token modes, ODCID and is_new_token results, expiry, RCID and PN mismatch semantics, and oversized-token rejection.
* Phase 5B analysis: Rust now directly validates retry and new-token behavior matching the C token matrix.
* Phase 5B fix note: Replaced the connection-helper-only test with prepare_retry_token/verify_retry_token checks for token classification, ODCID, same-IP port handling, address mismatch, expiry, RCID mismatch, PN mismatch, and oversized retry token rejection.

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

### Current Rust test body
```rust
fn retry_token_valid() {
    const TIME_BASE: u64 = 10_000 * 1_000_000;

    let mut simulated_time = Instant::from_ticks(TIME_BASE);
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, None).expect("ctx");
    let quic = &mut test_ctx.qserver;

    let addr = [
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 1234),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 3456),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(3, 3, 3, 3)), 1234),
    ];
    let n_cid = ConnectionId::default();
    let cid = [
        ConnectionId::clone_from_slice(&[1, 1, 1, 1, 1, 1, 1, 1]).expect("cid1"),
        n_cid,
        ConnectionId::clone_from_slice(&[2, 2, 2, 2, 2, 2, 2, 2, 2]).expect("cid2"),
    ];
    let odcid = [
        ConnectionId::clone_from_slice(&[3, 3, 3, 3, 3, 3, 3, 3]).expect("odcid"),
        n_cid,
    ];
    let pn = [0u32, 1, 2];

    for token_mode in 0..2 {
        let expected_new_token = odcid[token_mode].is_empty();
        let mut token_buffer = [0u8; 128];
        let token_size = quic
            .prepare_retry_token(
                &addr[0],
                Instant::from_ticks(TIME_BASE),
                &odcid[token_mode],
                &cid[token_mode],
                pn[1],
                &mut token_buffer,
            )
            .expect("prepare_retry_token");
        let token = &token_buffer[..token_size];

        let verified = quic
            .verify_retry_token(
                &addr[0],
                Instant::from_ticks(TIME_BASE),
                &cid[0],
                pn[2],
                token,
                false,
            )
            .expect("valid token");
        assert_retry_token_verified(
            verified,
            expected_new_token,
            &odcid[token_mode],
            "normal parameters",
        );

        let verified = quic
            .verify_retry_token(
                &addr[1],
                Instant::from_ticks(TIME_BASE),
                &cid[0],
                pn[2],
                token,
                false,
            )
            .expect("same-IP token");
        assert_retry_token_verified(
            verified,
            expected_new_token,
            &odcid[token_mode],
            "same IP, different port",
        );

        assert!(
            quic.verify_retry_token(
                &addr[2],
                Instant::from_ticks(TIME_BASE),
                &cid[0],
                pn[2],
                token,
                false,
            )
            .is_err(),
            "Token validation does not detect an address change."
        );

        let expired_delta = if token_mode == 0 {
            TOKEN_DELAY_SHORT.ticks() + 1
        } else {
            TOKEN_DELAY_LONG.ticks() + 1_000_000
        };
        assert!(
            quic.verify_retry_token(
                &addr[0],
                Instant::from_ticks(TIME_BASE + expired_delta),
                &cid[0],
                pn[2],
                token,
                false,
            )
            .is_err(),
            "Token validation does not detect elapsed time."
        );

        let rcid_mismatch = quic.verify_retry_token(
            &addr[0],
            Instant::from_ticks(TIME_BASE),
            &cid[2],
            pn[2],
            token,
            false,
        );
        if token_mode == 0 {
            assert!(rcid_mismatch.is_err(), "RCID invalidation fails");
        } else {
            let verified = rcid_mismatch.expect("new token ignores RCID");
            assert_retry_token_verified(verified, true, &n_cid, "new token RCID mismatch");
        }

        for &initial_pn in pn.iter().take(2) {
            let pn_mismatch = quic.verify_retry_token(
                &addr[0],
                Instant::from_ticks(TIME_BASE),
                &cid[0],
                initial_pn,
                token,
                false,
            );
            if token_mode == 0 {
                assert!(pn_mismatch.is_err(), "PN invalidation fails");
            } else {
                let verified = pn_mismatch.expect("new token ignores PN");
                assert_retry_token_verified(verified, true, &n_cid, "new token PN mismatch");
            }
        }

        if token_mode == 0 {
            let mut big_token = vec![0xa5; MAX_PACKET_SIZE];
            big_token[..token_size].copy_from_slice(token);
            assert!(
                quic.verify_retry_token(
                    &addr[0],
                    Instant::from_ticks(TIME_BASE),
                    &cid[0],
                    pn[2],
                    &big_token,
                    false,
                )
                .is_err(),
                "Bad length check fails"
            );
        }
    }
}
```

## `picoquictest/util_test.c:util_connection_id_parse_test`
* C test-table name: `connection_id_parse`
* C entry function: `util_connection_id_parse_test`
* Rust test: `connection_id_parse`
* Expected Rust file: `rs/fq/src/tests/util_test.rs`
* Current Rust span: `rs/fq/src/tests/util_test.rs:56-67`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust manually decodes hex with from_str_radix and constructs a ConnectionId; it does not test the translated parse_connection_id_hexa behavior or returned length.
* Phase 5A fix note: Call crate::utils::parse_connection_id_hexa for each expected string and assert parsed length/bytes equal the expected ConnectionId.
* Phase 5B analysis: Rust test now exercises the translated parser and checks the decoded length and bytes against the C table expectations.
* Phase 5B fix note: Updated connection_id_parse to call crate::utils::parse_connection_id_hexa and assert parsed ConnectionId length and value.

### C test body
```c
{
    int ret = 0;  
    for (size_t i = 0; i < test_cases; ++i) {
        picoquic_connection_id_t cnxid;
        uint8_t id_len = picoquic_parse_connection_id_hexa(expected_str[i], strlen(expected_str[i]), &cnxid);
        if (id_len != expected_cnxid[i].id_len) {
            DBG_PRINTF("Wrong length returned. result: %d, expected: %d\n", id_len, expected_cnxid[i].id_len);
            ret = -1;
        }
        if (picoquic_compare_connection_id(&cnxid, &expected_cnxid[i]) != 0) {
            DBG_PRINTF("%s", "the returned connection id is different than expected.\n");
            ret = -1;
        }
    }
    return ret;
}
```

### Current Rust test body
```rust
fn connection_id_parse() {
    for (bytes, hex) in EXPECTED_CIDS {
        let parsed = crate::utils::parse_connection_id_hexa(hex).expect("CID hex parse");
        let expected = ConnectionId::clone_from_slice(bytes).expect("CID under cap");
        assert_eq!(
            parsed.as_bytes().len(),
            expected.as_bytes().len(),
            "CID parsed length for {hex}",
        );
        assert_eq!(parsed, expected, "CID parse roundtrip for {hex}");
    }
}
```

## `picoquictest/congestion_test.c:bbr1_long_test`
* C test-table name: `bbr1_long`
* C entry function: `bbr1_long_test`
* Rust test: `bbr1_long`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:887-890`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper and long-test flow mostly match, but the shared Rust scenario verifier only closes the connection and omits C's stream completion, data-node pool, and 15s completion-time checks.
* Phase 5A fix note: Implement the missing tls_api_one_scenario_body_verify assertions or add equivalent checks in congestion_long_test before accepting bbr1_long.
* Phase 5B analysis: Current Rust already matches the C long-test flow and verifier: stream completion, stream0 counts, data-node pool checks, close, and the 15s completion bound are present.
* Phase 5B fix note: 

### C test body
```c
{
    return congestion_long_test(picoquic_bbr1_algorithm);
}
```

### Current Rust test body
```rust
fn bbr1_long() {
    let ccalgo = cc_algo("bbr1");
    congestion_long_test(ccalgo);
}
```
