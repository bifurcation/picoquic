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

## `picoquictest/tls_api_test.c:mtu_drop_fast_test`
* C test-table name: `mtu_drop_fast`
* C entry function: `mtu_drop_fast_test`
* Rust test: `mtu_drop_fast`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:4346-4348`
* Phase 5A analysis: Rust calls the named helper with fast/11_500_000, but the helper ignores the CC algorithm, uses different link timing, does not assert MTU discovery, does not drop path MTU, and the verifier ignores the target time.
* Phase 5A fix note: Make Rust mtu_drop_cc_algotest select the requested algorithm, match the C link setup, wait for and assert MTU discovery, lower both path MTUs, then complete and verify the very-long transfer within the target time.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust calls mtu_drop_cc_algotest("fast", 11_500_000), and the helper mirrors C link setup, FastCC selection, MTU assertions, MTU drop, and target-time verification, but the final test tree is not compile-exposed.
* Phase 5C fix note: Repair merged tls_api.rs/util.rs compile-exposure damage; no mtu_drop_fast-specific semantic mismatch found.

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_fastcc_algorithm, 11500000);
    return ret;
}
```

### Current Rust test body
```rust

/// C: `mtu_drop_fast_test` in `picoquictest/tls_api_test.c`.
///
```

## `picoquictest/tls_api_test.c:nat_rebinding_test`
* C test-table name: `nat_rebinding`
* C entry function: `nat_rebinding_test`
* Rust test: `nat_rebinding`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:4658-4660`
* Phase 5A analysis: The Rust helper accepts the same arguments but does not simulate NAT rebinding: it never switches the client address/port, does not capture or verify path challenge renewal, ignores cid_zero behavior, and omits the post-transfer challenge/stash checks.
* Phase 5A fix note: Implement nat_rebinding_test_one with explicit initial CID variants, sync-to-empty with the path target, change the client NAT address before data, use the q_and_r scenario, then loop and assert the server challenge changed and was verified, including the zero-CID/loss/latency cases.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust nat_rebinding calls nat_rebinding_test_one(0,false,0), but the current shared helper does not simulate NAT rebinding, uses a different long scenario, ignores cid_zero, and omits scenario/challenge verification.
* Phase 5C fix note: Restore nat_rebinding_test_one to change client_addr_natted/client_use_nat before data, use test_scenario_q_and_r, apply loss only to data, and assert challenge renewal/verification.

### C test body
```c
{
    uint64_t loss_mask = 0;

    return nat_rebinding_test_one(loss_mask, 0, 0);
}
```

### Current Rust test body
```rust

/// C: `nat_rebinding_test` in `picoquictest/tls_api_test.c`.
///
```

## `picoquictest/tls_api_test.c:fast_nat_rebinding_test`
* C test-table name: `nat_rebinding_fast`
* C entry function: `fast_nat_rebinding_test`
* Rust test: `nat_rebinding_fast`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:4800-4802`
* Phase 5A analysis: Rust only runs the generic no-loss TLS helper; it does not enable NAT, repeatedly change the client NAT port, require six rebinding switches, or verify sustained data completion after those switches.
* Phase 5A fix note: Implement a Rust fast NAT rebinding test/helper mirroring the C loop: initial CID, qlog setup, sustained scenario, NAT port increments after server observes each port, require at least 6 switches, then verify scenario completion.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust fast_nat_rebinding_test mirrors C initial CID, qlog setup, sustained scenario, repeated NAT port switches, six-switch requirement, and final scenario verification, but the final tls_api.rs/util.rs test tree is not runnable.
* Phase 5C fix note: Repair duplicate tls_api.rs scenario constants and util.rs harness merge damage; no nat_rebinding_fast-specific semantic mismatch found.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    const int nb_switches_required = 6;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xfa, 0x57, 0x08, 0xa7, 0, 0, 0, 0}, 8 };

    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0 && test_ctx == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    }

    if (ret == 0) {
        /* Set up logging */ 
        picoquic_set_qlog(test_ctx->qserver, ".");
        picoquic_set_qlog(test_ctx->qclient, ".");
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_sustained, sizeof(test_scenario_sustained));
    }

    /* Perform a data sending loop */
    if (ret == 0) {
        uint64_t delta_t = 5 * (test_ctx->c_to_s_link->microsec_latency + test_ctx->s_to_c_link->microsec_latency);
        uint64_t next_time = simulated_time + 200000000;
        int nb_trials = 0;
        int nb_inactive = 0;
        int max_trials = 1000000;
        uint64_t switch_time = simulated_time;
        int switched = 0;
        int nb_switched = 0;

        test_ctx->client_use_nat = 1;
        test_ctx->client_addr_natted = test_ctx->client_addr;
        test_ctx->client_addr_natted.sin_port += 17;

        while (ret == 0 && nb_trials < max_trials && nb_inactive < 256 && simulated_time < next_time && TEST_CLIENT_READY && TEST_SERVER_READY) {
            int was_active = 0;

            nb_trials++;

            ret = tls_api_one_sim_round(test_ctx, &simulated_time, next_time, &was_active);

            if (ret < 0)
            {
                break;
            }

            if (ret == 0 && test_ctx->cnx_server != NULL && test_ctx->cnx_server->cnx_state == picoquic_state_ready) {
                if (((struct sockaddr_in*) & test_ctx->cnx_server->path[0]->first_tuple->peer_addr)->sin_port == 0) {
                    DBG_PRINTF("Client address out of sync, port: %d", ((struct sockaddr_in*) & test_ctx->cnx_server->path[0]->first_tuple->peer_addr)->sin_port);
                }
                else if (((struct sockaddr_in*) & test_ctx->cnx_server->path[0]->first_tuple->peer_addr)->sin_port == test_ctx->client_addr_natted.sin_port) {
                    if (switched) {
                        if (simulated_time > switch_time + delta_t &&
                            nb_switched < nb_switches_required) {
                            switched = 0;
                        }
                    }
                    else
                    {
                        /* Change the client address */
                        test_ctx->client_addr_natted.sin_port += 17;
                        switched = 1;
                        switch_time = simulated_time;
                        nb_switched++;
                    }
                }
            }

            if (was_active) {
                nb_inactive = 0;
            }
            else {
                nb_inactive++;

                if (nb_inactive == 254) {
                    DBG_PRINTF("Almost stalled after %d trials, %d inactive, %d switches", nb_trials, nb_inactive, nb_switched);
                }
            }

            if (test_ctx->test_finished) {
                if (picoquic_is_cnx_backlog_empty(test_ctx->cnx_client) && picoquic_is_cnx_backlog_empty(test_ctx->cnx_server)) {
                    break;
                }
            }
        }

        DBG_PRINTF("Exit after %d trials, %d inactive, %d switches", nb_trials, nb_inactive, nb_switched);

        /* Verify that the test was effective */
        if (ret == 0 && nb_switched < nb_switches_required) {
            ret = -1;
        }
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_verify(test_ctx);
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

    tls_api_one_scenario_verify(&test_ctx)
}
```

## `picoquictest/tls_api_test.c:nat_rebinding_latency_test`
* C test-table name: `nat_rebinding_latency`
* C entry function: `nat_rebinding_latency_test`
* Rust test: `nat_rebinding_latency`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:4808-4810`
* Phase 5A analysis: The wrapper arguments match, but the Rust helper does not perform NAT rebinding or verify challenge renewal/validation; it only runs a normal transfer with latency.
* Phase 5A fix note: Implement the NAT address switch, q_and_r transfer, completion check, post-rebinding wait, and server challenge renewal/verified assertions for latency=100000.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust wrapper passes the right arguments, but the current nat_rebinding_test_one helper is still not faithful: it ignores cid_zero, does not perform NAT rebinding, uses a different data scenario, lacks scenario verification, and omits challenge renewal/verification checks.
* Phase 5C fix note: Implement the C NAT rebinding flow in the Rust helper: zero-CID-aware context setup, q_and_r scenario, NAT port switch, data completion verification, post-rebinding wait, and server challenge renewed/verified assertions.

### C test body
```c
{
    /* Test of NAT rebinding with zero-length client CID */
    uint64_t loss_mask = 0;

    return nat_rebinding_test_one(loss_mask, 0, 100000);
}
```

### Current Rust test body
```rust

/// C: `nat_rebinding_latency_test` in `picoquictest/tls_api_test.c`.
///
```

## `picoquictest/tls_api_test.c:nat_rebinding_loss_test`
* C test-table name: `nat_rebinding_loss`
* C entry function: `nat_rebinding_loss_test`
* Rust test: `nat_rebinding_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:4816-4818`
* Phase 5A analysis: Rust passes the same top-level arguments, but the helper does not implement the C NAT rebinding behavior: it applies the loss mask during the handshake, never switches to a NAT address, ignores the zero-CID path, and omits path challenge/remote CID verification.
* Phase 5A fix note: Implement Rust nat_rebinding_test_one to match C: handshake with zero loss, switch client address/port for NAT rebinding, run data transfer with loss_mask_data, and verify scenario completion, challenge renewal/verification, path count, and remote CID expectations.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test entry is present, but the current nat_rebinding_test_one helper does not express the C helper: it applies data loss during handshake, never switches to the NAT address, uses a different data scenario, and omits challenge/path/remote-CID verification.
* Phase 5C fix note: Restore the Phase 5B NAT helper behavior: handshake with zero loss, set the C-style initial CID, enable NAT rebinding before data transfer, use test_scenario_q_and_r, apply loss_mask_data only to data, and assert scenario/challenge/path/remote-CID postconditions.

### C test body
```c
{
    uint64_t loss_mask = 0x2012;

    return nat_rebinding_test_one(loss_mask, 0, 0);
}
```

### Current Rust test body
```rust

/// C: `nat_rebinding_loss_test` in `picoquictest/tls_api_test.c`.
///
```
