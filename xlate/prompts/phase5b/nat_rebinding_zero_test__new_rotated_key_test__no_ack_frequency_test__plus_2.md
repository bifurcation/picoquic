# Phase 5B repair Rust test mismatches

You are repairing Phase 5A `needs_fix` entries.  The goal
is to make the Rust tests faithfully check the same behavior
as the C tests.

Rules:

* Edit Rust tests, Rust test helpers, and Rust test fixtures
  only: `rs/fq/src/tests/` and `rs/fq/tests/fixtures/`.
* Do not edit C sources.
* Do not weaken assertions, skip important C cases, or replace
  tests with placeholders.
* If the test already matches after closer inspection, report
  `ok` and do not edit source.
* Phase 5B is about test/API correspondence, not test success.
  The Rust test must exist, compile as a test, and be runnable
  by the Rust test harness, but it may fail arbitrarily early
  because the Rust library implementation is incomplete.
* Do not report `blocked` merely because the implementation
  returns the wrong state, fails a handshake, lacks protocol
  behavior, or would fail the test. Those are Phase 5C issues.
* Report `blocked` only when the faithful test cannot be
  written, compiled, or exposed as a runnable Rust test because
  the necessary Rust API/test-harness surface is missing or
  ambiguous.
* Do not run full `cargo test` in this pass. Use source review
  and, if needed, `cargo check --tests` for compile validation.

Owned Rust test file(s): `rs/fq/src/tests/tls_api.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/tls_api_test.c:nat_rebinding_zero_test`
* C test-table name: `nat_rebinding_zero`
* C entry function: `nat_rebinding_zero_test`
* Rust test: `nat_rebinding_zero`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:4984-4986`
* Phase 5A analysis: agent response did not include this test
* Phase 5A fix note: 
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The current merged Rust wrapper calls nat_rebinding_test_one(0, true, 0), but the shared helper in util.rs ignores cid_zero, uses normal context initialization, never switches client_use_nat/client_addr_natted, uses a different long scenario, and does not verify challenge renewal/verification.
* Phase 5C fix note: Restore the NAT rebinding helper repair: honor zero-CID initialization, apply NAT port rebinding, run q_and_r with loss_mask_data, verify stream completion, then assert server challenge was renewed and verified.

### C test body
```c
{
    /* Test of NAT rebinding with zero-length client CID */
    uint64_t loss_mask = 0;

    return nat_rebinding_test_one(loss_mask, 1, 0);
}
```

### Current Rust test body
```rust
fn nat_rebinding_zero() {
    nat_rebinding_test_one(0, true, 0).expect("nat_rebinding_zero");
}
```

## `picoquictest/tls_api_test.c:new_rotated_key_test`
* C test-table name: `new_rotated_key`
* C entry function: `new_rotated_key_test`
* Rust test: `new_rotated_key`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:5154-5156`
* Phase 5A analysis: agent response did not include this test
* Phase 5A fix note: 
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The repaired key-rotation logic matches the C intent, but the final tree defines wait_application_aead_ready twice in rs/fq/src/tests/tls_api.rs, so this #[test] module is not currently compile-runnable.
* Phase 5C fix note: Deduplicate or rename the overlapping wait_application_aead_ready helper; keep the existing rotated-secret and AEAD checks.

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

### Current Rust test body
```rust
fn new_rotated_key() {
    new_rotated_key_impl().expect("new_rotated_key");
}
```

## `picoquictest/tls_api_test.c:no_ack_frequency_test`
* C test-table name: `no_ack_frequency`
* C entry function: `no_ack_frequency_test`
* Rust test: `no_ack_frequency`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:5163-5214`
* Phase 5A analysis: agent response did not include this test
* Phase 5A fix note: 
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust covers the three parameter combinations and scenario, but initializes with tls_api_init_ctx_ex, which starts the client before client_parameters are installed. The C path uses delayed init, sets client/server transport parameters, then starts the client in the scenario body.
* Phase 5C fix note: Install client/server transport parameters before client start/TLS transport-parameter serialization, then run the same very-long scenario with init loss mask 128 and 2000000 target.

### C test body
```c
{
    int ret = 0;
    picoquic_tp_t client_parameters;
    picoquic_tp_t server_parameters;

    for (int i = 1; ret == 0 && i <= 3; i++) {
        memset(&client_parameters, 0, sizeof(picoquic_tp_t));
        memset(&server_parameters, 0, sizeof(picoquic_tp_t));
        picoquic_init_transport_parameters(&client_parameters);
        picoquic_init_transport_parameters(&server_parameters);

        client_parameters.min_ack_delay = (i & 1) ? 0 : 1000;
        server_parameters.enable_loss_bit = (1 - ((i > 1) & 1));

        ret = tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 128, 0, 0, 0, 2000000, &client_parameters, &server_parameters);
        if (ret != 0) {
            DBG_PRINTF("No min ack delay test fails for client: %d, server: %d, ret = %d", i & 1, i >> 1, ret);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn no_ack_frequency() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    for i in 1..=3 {
        let mut client_parameters = TransportParameters::default();
        let mut server_parameters = TransportParameters::default();
        crate::internal::init_transport_parameters(&mut client_parameters);
        crate::internal::init_transport_parameters(&mut server_parameters);

        client_parameters.min_ack_delay = Duration::from_ticks(if i & 1 == 1 { 0 } else { 1000 });
        server_parameters.enable_loss_bit = if i > 1 { 0 } else { 1 };

        let mut simulated_time = Instant::from_ticks(0);
        let mut test_ctx = tls_api_init_ctx_ex2_delayed(
            &mut simulated_time,
            0,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            None,
            None,
        )
        .unwrap_or_else(|| panic!("no_ack_frequency({i}): ctx"));
        test_ctx
            .cnx_client()
            .set_transport_parameters(&client_parameters);
        test_ctx
            .qserver
            .set_default_tp(&server_parameters)
            .unwrap_or_else(|e| panic!("no_ack_frequency({i}): server tp: {e:?}"));
        test_ctx
            .cnx_client()
            .start_client()
            .unwrap_or_else(|e| panic!("no_ack_frequency({i}): start client: {e:?}"));

        tls_api_one_scenario_body(
            &mut test_ctx,
            &mut simulated_time,
            &scenario,
            128,
            0,
            0,
            0,
            2_000_000,
        )
        .unwrap_or_else(|e| panic!("no_ack_frequency({i}): {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:optimistic_ack_test`
* C test-table name: `optimistic_ack`
* C entry function: `optimistic_ack_test`
* Rust test: `optimistic_ack`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:5407-5409`
* Phase 5A analysis: agent response did not include this test
* Phase 5A fix note: 
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust calls optimistic_ack_test_one(true), and the helper covers C optimistic-ACK policy, deterministic seed, trap-hole scan, spoof recording, retransmission rejection, hole counters, and spoofed-transfer failure expectation, but final-tree compile exposure is broken.
* Phase 5C fix note: Repair shared tls_api.rs/util.rs merge damage; no optimistic_ack-specific semantic mismatch found.

### C test body
```c
{
    int ret = optimistic_ack_test_one(1);

    return ret;
}
```

### Current Rust test body
```rust
fn optimistic_ack() {
    optimistic_ack_test_one(true).expect("optimistic_ack");
}
```

## `picoquictest/tls_api_test.c:pn_random_test`
* C test-table name: `pn_random`
* C entry function: `pn_random_test`
* Rust test: `pn_random`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6076-6079`
* Phase 5A analysis: agent response did not include this test
* Phase 5A fix note: 
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust runs both pn_random_test_one(false) and true, sets random_initial 1/2, forces/checks packet contexts, and completes q_and_r like C, but the final merged test module/harness is not compile-exposed.
* Phase 5C fix note: Repair duplicate tls_api.rs scenario constants and malformed util.rs TestTlsApiCtx harness items; no pn_random-specific semantic mismatch found.

### C test body
```c
{

    int ret = pn_random_test_one(0);

    if (ret != 0) {
        DBG_PRINTF("Randomize initials fails, ret = %d", ret);
    } else{
        ret = pn_random_test_one(1);
        if (ret != 0) {
            DBG_PRINTF("Randomize all fails, ret = %d", ret);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn pn_random() {
    pn_random_test_one(false).expect("pn_random initial-only");
    pn_random_test_one(true).expect("pn_random all packet number spaces");
}
```
