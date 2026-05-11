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

Owned Rust test file(s): `rs/fq/src/tests/cert_verify.rs`, `rs/fq/src/tests/cnxstress.rs`, `rs/fq/src/tests/satellite.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/cert_verify_test.c:cert_verify_null_test`
* C test-table name: `cert_verify_null`
* C entry function: `cert_verify_null_test`
* Rust test: `cert_verify_null`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Rust span: `rs/fq/src/tests/cert_verify.rs:70-78`
* Phase 5A analysis: Inputs match, but Rust only asserts tls_api_connection_loop success; C also fails if client/server are not both ready after the loop.
* Phase 5A fix note: Update cert_verify_test_one to treat Ok-but-not-ready as failure, matching C TEST_CLIENT_READY and TEST_SERVER_READY logic.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Final tree lost the helper readiness check: Rust only compares tls_api_connection_loop Ok/Err, while C treats Ok as success only if client and server are both ready.
* Phase 5C fix note: Restore cert_verify_test_one success calculation to require loop_ok && client_ready() && server_ready() before comparing with expect_success.

### C test body
```c
{
    int ret = cert_verify_test_one(1, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        NULL, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_null() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        None,
        Some(TEST_SNI),
    );
}
```

## `picoquictest/cert_verify_test.c:cert_verify_null_sni_test`
* C test-table name: `cert_verify_null_sni`
* C entry function: `cert_verify_null_sni_test`
* Rust test: `cert_verify_null_sni`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Rust span: `rs/fq/src/tests/cert_verify.rs:83-91`
* Phase 5A analysis: Inputs match the C null-SNI success case, but the Rust helper only checks tls_api_connection_loop().is_ok(); C also treats success as failure unless both client and server are ready.
* Phase 5A fix note: Make cert_verify_test_one compute success as loop_ok && client_ready && server_ready, then compare that with expect_success.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Inputs match the C null-SNI success case, but the Rust helper only treats tls_api_connection_loop Ok as success; C also requires client_ready and server_ready.
* Phase 5C fix note: Fold test_ctx.client_ready() && test_ctx.server_ready() into the positive success condition before comparing with expect_success.

### C test body
```c
{
    int ret = cert_verify_test_one(1, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, NULL);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_null_sni() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        None,
    );
}
```

## `picoquictest/cert_verify_test.c:cert_verify_rsa_test`
* C test-table name: `cert_verify_rsa`
* C entry function: `cert_verify_rsa_test`
* Rust test: `cert_verify_rsa`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Rust span: `rs/fq/src/tests/cert_verify.rs:96-104`
* Phase 5A analysis: Fixture inputs match, but Rust treats tls_api_connection_loop Ok as success; the C helper also requires TEST_CLIENT_READY and TEST_SERVER_READY for positive cases.
* Phase 5A fix note: Make cert_verify_test_one fold client_ready() && server_ready() into the success condition when expect_success is true.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Fixture inputs match C, but the final Rust helper again omits the C TEST_CLIENT_READY and TEST_SERVER_READY success requirement.
* Phase 5C fix note: Make cert_verify_test_one compute success as loop_ok && client_ready && server_ready for expected-success cases.

### C test body
```c
{
    int ret = cert_verify_test_one(1, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_rsa() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_SNI),
    );
}
```

## `picoquictest/cnxstress.c:cnx_limit_test`
* C test-table name: `cnx_limit`
* C entry function: `cnx_limit_test`
* Rust test: `cnx_limit`
* Expected Rust file: `rs/fq/src/tests/cnxstress.rs`
* Rust span: `rs/fq/src/tests/cnxstress.rs:897-957`
* Phase 5A analysis: The Rust test body mirrors the C phases, but the Rust stress helper treats limit_test as an initial extra client target and active limit-test mode. In C, limit_test only sizes capacity; is_limit_test is set and nb_client_target is incremented only after the first four clients are ready.
* Phase 5A fix note: Make cnx_stress_create_ctx keep nb_client_target at nb_clients and is_limit_test false initially, while still allocating the extra slot/max client capacity for limit tests.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust test body mirrors the C phases, but cnx_stress_create_ctx currently starts limit_test contexts with is_limit_test=true and nb_client_target=nb_clients+1, unlike C which only allocates extra capacity initially.
* Phase 5C fix note: Initialize is_limit_test=false and nb_client_target=nb_clients while keeping the extra allocation slot; let cnx_limit enable limit mode and increment target after the first four clients are ready.

### C test body
```c
{
    int ret = 0;
    int nb_clients = 4;
    uint64_t duration = 120000000;
    cnx_stress_ctx_t* stress_ctx = cnx_stress_create_ctx(duration, nb_clients, 1);

    if (stress_ctx == NULL) {
        ret = -1;
    }

    if (stress_ctx != NULL) {
        int is_done = 0;

        /* loop until time exhausted or all created */
        while (ret == 0 && stress_ctx->simulated_time < duration && !is_done) {
            ret = cnx_stress_loop_step(stress_ctx);
            if (stress_ctx->nb_clients == nb_clients &&
                stress_ctx->nb_servers == nb_clients) {
                is_done = 1;
                for (int c = 0; c < nb_clients; c++) {
                    if (stress_ctx->c_ctx[c] == NULL ||
                        stress_ctx->c_ctx[c]->cnx == NULL ||
                        stress_ctx->c_ctx[c]->cnx->cnx_state <
                        picoquic_state_client_almost_ready) {
                        is_done = 0;
                        break;
                    }
                }
            }
        }
        if (!is_done) {
            ret = -1;
        }

        if (ret == 0) {
            /* Try creating one more client. Loop until time exhausted or
             * verify new client refused because server busy */

            stress_ctx->is_limit_test = 1;
            stress_ctx->next_client_creation_time = stress_ctx->simulated_time;
            stress_ctx->nb_client_target++;
            while (ret == 0 && stress_ctx->simulated_time < duration &&
                !stress_ctx->limit_test_got_server_busy) {
                ret = cnx_stress_loop_step(stress_ctx);
            }
            if (!stress_ctx->limit_test_got_server_busy) {
                ret = -1;
            }
        }

        cnx_stress_delete_ctx(stress_ctx);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn cnx_limit() {
    let nb_clients = 4usize;
    let duration = 120_000_000u64;

    let mut ctx = cnx_stress_create_ctx(duration, nb_clients, true).expect("create stress context");

    // Run until all `nb_clients` connections are up on both sides and at least
    // at `ClientAlmostReady`.
    let mut is_done = false;
    while !is_done && ctx.simulated_time() < duration {
        cnx_stress_loop_step(&mut ctx).expect("loop step");

        let (nb_c, nb_s) = {
            let shared = ctx.shared.borrow();
            (shared.nb_clients, shared.nb_servers)
        };
        if nb_c == nb_clients && nb_s == nb_clients {
            // Check that every client slot has a connection at AlmostReady or beyond.
            is_done = true;
            for c in 0..nb_clients {
                let ok = if let Some(Some(cid)) = ctx.client_connections.get(c) {
                    let cid = *cid;
                    if let Some(cnx) = ctx.qclient.connection_ref_by_id(cid) {
                        cnx.state() >= State::ClientAlmostReady
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !ok {
                    is_done = false;
                    break;
                }
            }
        }
    }
    assert!(
        is_done,
        "failed to reach ClientAlmostReady for all connections"
    );

    // Now attempt one extra connection (beyond the server's limit).
    {
        let mut shared = ctx.shared.borrow_mut();
        shared.is_limit_test = true;
        shared.nb_client_target += 1;
    }
    ctx.next_client_creation_time = ctx.simulated_time();

    while ctx.simulated_time() < duration {
        cnx_stress_loop_step(&mut ctx).expect("limit loop step");
        if ctx.shared.borrow().limit_test_got_server_busy {
            break;
        }
    }
    assert!(
        ctx.shared.borrow().limit_test_got_server_busy,
        "server did not send SERVER_BUSY when at connection limit",
    );
}
```

## `picoquictest/satellite_test.c:satellite_bbr1_test`
* C test-table name: `satellite_bbr1`
* C entry function: `satellite_bbr1_test`
* Rust test: `satellite_bbr1`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Rust span: `rs/fq/src/tests/satellite.rs:389-404`
* Phase 5A analysis: The Rust wrapper passes the same bbr1/data/link parameters, but satellite_test_one calls a Rust tls_api_one_scenario_body variant that treats data_size as an ignored compatibility argument instead of C's stream0_target, so it does not perform the 100 MB stream-0 transfer; final completion verification is also close-only.
* Phase 5A fix note: Update satellite_test_one to use a helper/signature that sets stream0_target=data_size, preserves init_loss_mask and queue_delay_max=2*latency, and verifies stream0 completion plus max_completion_time.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust satellite test now faithfully passes the 100 MB stream target and C-equivalent link/BBR1 parameters, but the merged shared Rust test harness is currently malformed and not runnable.
* Phase 5C fix note: Repair rs/fq/src/tests/util.rs shared harness scope/duplicate-field regression; no pair-specific C/Rust intent mismatch found.

### C test body
```c
{
    /* Should be less than 7 sec per draft etosat */
    return satellite_test_one(picoquic_bbr1_algorithm, 100000000, 7000000, 250, 3, 0, 0, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_bbr1() {
    let bbr1 = satellite_ccalgo("bbr1");
    satellite_test_one(
        bbr1,
        100_000_000,
        7_000_000,
        250,
        3,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```
