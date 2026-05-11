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

## `picoquictest/tls_api_test.c:transmit_cnxid_disable_test`
* C test-table name: `cnxid_transmit_disable`
* C entry function: `transmit_cnxid_disable_test`
* Rust test: `cnxid_transmit_disable`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1673-1675`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust wrapper passes the same booleans, but the shared Rust helper ignores the disable flag and omits the C CID-count and stash-correspondence assertions.
* Phase 5A fix note: Implement transmit_cnxid_test_one behavior for disable_migration: set server default TP migration_disabled before handshake, wait for CID exchange, assert local CID counts and peer stash contents as in C.
* Phase 5B analysis: Rust #[test] cnxid_transmit_disable is present, compiles under the Rust test harness, and calls transmit_cnxid_test_one(false, true, false), matching C transmit_cnxid_test_one(0, 1, 0). The helper expresses the disable_migration setup and CID count/stash checks; the missing server connection at runtime is a Phase 5C implementation issue.
* Phase 5B fix note: 

### C test body
```c
{
    return transmit_cnxid_test_one(0, 1, 0);
}
```

### Current Rust test body
```rust
fn cnxid_transmit_disable() {
    transmit_cnxid_test_one(false, true, false).expect("cnxid_transmit_disable");
}
```

## `picoquictest/tls_api_test.c:tls_different_params_test`
* C test-table name: `different_params`
* C entry function: `tls_different_params_test`
* Rust test: `different_params`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1963-1988`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs an empty scenario with default context parameters; C runs test_scenario_very_long with client transport parameters initialized to C defaults and initial_max_stream_id_bidir forced to 0.
* Phase 5A fix note: Create C-equivalent initialized TransportParameters, set initial_max_stream_id_bidir to 0, initialize the scenario context with those client params, and run the very-long stream scenario {4,0,257,1000000}.
* Phase 5B analysis: Rust #[test] different_params matches the C API-level contract: initialized client transport parameters, initial_max_stream_id_bidir = 0, server params None/NULL, very-long scenario, zero loss/max-data/queue-delay/proposed-version fields, and 3_510_000 completion bound. Any remaining very-long stream transfer/verification runtime failure is a Phase 5C implementation issue.
* Phase 5B fix note: 

### C test body
```c
{
    picoquic_tp_t test_parameters;

    memset(&test_parameters, 0, sizeof(picoquic_tp_t));

    picoquic_init_transport_parameters(&test_parameters);

    test_parameters.initial_max_stream_id_bidir = 0;

    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 0, 0, 0, 3510000, &test_parameters, NULL);
}
```

### Current Rust test body
```rust
fn different_params() {
    let mut t = Instant::from_ticks(0);
    let mut client_params = TransportParameters::default();
    init_transport_parameters(&mut client_params);
    client_params.initial_max_stream_id_bidir = 0;

    let mut ctx = tls_api_one_scenario_init_ex(
        &mut t,
        Version::InternalTest1,
        Some(&client_params),
        None,
        None,
    )
    .expect("ctx");
    tls_api_one_scenario_body(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_VERY_LONG,
        0,
        0,
        0,
        0,
        3_510_000,
    )
    .expect("different_params");
}
```

## `picoquictest/tls_api_test.c:heavy_loss_test`
* C test-table name: `heavy_loss`
* C entry function: `heavy_loss_test`
* Rust test: `heavy_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2816-2818`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust helper is materially different: it uses a smaller one-stream scenario, injects simple link burst loss before handshake, ignores the C ramp/loss-mask window, does not set BBR/qlog/initial CID, and the completion target is not actually enforced by the current verify helper.
* Phase 5A fix note: Rework heavy_loss_test_one to match the C helper for mode 0: use the sustained 4-stream scenario, explicit initial CID, BBR setup where available, 0.1s ramp, fixed 50% loss mask for up to 20 seconds, then clear loss and verify completion within 23500000.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, compiles under the Rust test harness, and calls heavy_loss_test_one(0, 23_500_000), matching the C entry function's API-level contract. Any early Generic/runtime stream-completion failure is Phase 5C implementation work, not a Phase 5B block.
* Phase 5B fix note: No Rust test changes; command log bookkeeping only.

### C test body
```c
{
    return heavy_loss_test_one(0, 23500000);
}
```

### Current Rust test body
```rust
fn heavy_loss() {
    heavy_loss_test_one(0, 23_500_000).expect("heavy_loss");
}
```

## `picoquictest/tls_api_test.c:integrity_limit_test`
* C test-table name: `integrity_limit`
* C entry function: `integrity_limit_test`
* Rust test: `integrity_limit`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3352-3354`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the generic TLS handshake/close helper; it omits the long data loop, confidentiality-limit assertions, forced AEAD failure count, bad packet injection, disconnecting-state check, and AEAD_LIMIT_REACHED error check.
* Phase 5A fix note: Port the C test body: exact initial CID, run very_long until 1-RTT AEAD is active, assert crypto_epoch_length_max equals confidentiality limit on both endpoints, inject one bad packet after setting failure count to integrity limit, and assert disconnecting with AeadLimitReached.
* Phase 5B analysis: Rust test already matches the C API-level contract and compiles as a runnable Rust harness test. The prior crypto_epoch_length_max/long-loop runtime concerns are Phase 5C implementation/runtime notes, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    int nb_initial_loop = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x15, 0x4E, 0x98, 0x14, 0, 0, 0, 1}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    if (ret == 0) {
        picoquic_set_qlog(test_ctx->qserver, ".");
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    /* Perform a data sending loop for a few rounds after the ready state */
    while (ret == 0 && nb_initial_loop < 64) {
        if (test_ctx->cnx_server != NULL && 
            test_ctx->cnx_server->crypto_context[picoquic_epoch_1rtt].aead_decrypt != NULL) {
            nb_initial_loop++;
        }

        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 16);
    }

    /* Check the max length of an epoch is the expected value */
    if (ret == 0) {
        uint64_t limit = picoquic_aead_confidentiality_limit(test_ctx->cnx_server->crypto_context[picoquic_epoch_1rtt].aead_decrypt);

        if (test_ctx->cnx_server->crypto_epoch_length_max != limit) {
            DBG_PRINTF("Server confidentiality limit set to 0x%" PRIx64 ", insted of %" PRIx64,
                test_ctx->cnx_server->crypto_epoch_length_max, limit);
            ret = -1;
        } else if (test_ctx->cnx_client->crypto_epoch_length_max != limit) {
            DBG_PRINTF("Client confidentiality limit set to 0x%" PRIx64 ", insted of %" PRIx64,
                test_ctx->cnx_client->crypto_epoch_length_max, limit);
            ret = -1;
        }
    }


    /* Set the number of failed decryptions just below the limit and then send a bad packet  */
    if (ret == 0 && test_ctx->cnx_server != NULL) {
        uint8_t p[256];

        test_ctx->cnx_server->crypto_failure_count = picoquic_aead_integrity_limit(
            test_ctx->cnx_server->crypto_context[picoquic_epoch_1rtt].aead_decrypt);

        memset(p, 0, sizeof(p));
        memcpy(p + 1, test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id.id, test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len);
        p[0] |= 64;
        (void)picoquic_incoming_packet(test_ctx->qserver, p, sizeof(p), (struct sockaddr*) & test_ctx->cnx_server->path[0]->first_tuple->peer_addr,
            (struct sockaddr*) & test_ctx->cnx_server->path[0]->first_tuple->local_addr, 0, test_ctx->recv_ecn_server, simulated_time);

        if (test_ctx->cnx_server->cnx_state != picoquic_state_disconnecting) {
            DBG_PRINTF("Connection not disconnecting, limit 0x%" PRIx64 ", reached %" PRIx64,
                test_ctx->cnx_server->crypto_context[picoquic_epoch_1rtt].aead_decrypt,
                test_ctx->cnx_server->crypto_failure_count);
            ret = -1;
        }
        else if (test_ctx->cnx_server->local_error != PICOQUIC_TRANSPORT_AEAD_LIMIT_REACHED) {
            DBG_PRINTF("Wrong error code, 0x%x instead of 0x%x",
                test_ctx->cnx_server->local_error,
                PICOQUIC_TRANSPORT_AEAD_LIMIT_REACHED);
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

### Current Rust test body
```rust
fn integrity_limit() {
    integrity_limit_test_one().expect("integrity_limit");
}
```

## `picoquictest/tls_api_test.c:loss_bit_test`
* C test-table name: `loss_bit`
* C entry function: `loss_bit_test`
* Rust test: `loss_bit`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3688-3840`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C runs four client/server enable_loss_bit transport-parameter combinations over test_scenario_many_streams with a 250000us target. Rust runs only a generic handshake/close with default parameters and no many-streams scenario.
* Phase 5A fix note: Add the four transport-parameter combinations, set client/server enable_loss_bit as in C, and run the many-streams scenario with the C completion target.
* Phase 5B analysis: Rust loss_bit is present as a #[test], compiles under the Rust test harness, and faithfully expresses the C API-level contract: all four client/server enable_loss_bit transport-parameter combinations run the many-streams scenario with a 250000us completion target. Any early disconnect or behavior failure is Phase 5C implementation work, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    picoquic_tp_t client_parameters;
    picoquic_tp_t server_parameters;

    for (int i = 0; ret == 0 && i <= 3; i++) {
        memset(&client_parameters, 0, sizeof(picoquic_tp_t));
        memset(&server_parameters, 0, sizeof(picoquic_tp_t));
        picoquic_init_transport_parameters(&client_parameters);
        picoquic_init_transport_parameters(&server_parameters);

        client_parameters.enable_loss_bit = (i & 1);
        server_parameters.enable_loss_bit = ((i > 1) & 1);

        ret = tls_api_one_scenario_test(test_scenario_many_streams, sizeof(test_scenario_many_streams), 0, 0, 0, 0, 0, 250000, &client_parameters, &server_parameters);
        if (ret != 0) {
            DBG_PRINTF("Loss bit test fails for client: %d, server: %d, ret = %d", i & 1, i >> 1, ret);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn loss_bit() {
    const TEST_SCENARIO_MANY_STREAMS: &[TestApiStreamDesc] = &[
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 1000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 1000,
        },
        TestApiStreamDesc {
            stream_id: 12,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 1000,
        },
        TestApiStreamDesc {
            stream_id: 16,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 1000,
        },
        TestApiStreamDesc {
            stream_id: 20,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 350,
        },
        TestApiStreamDesc {
            stream_id: 24,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 225,
        },
        TestApiStreamDesc {
            stream_id: 28,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 700,
        },
        TestApiStreamDesc {
            stream_id: 32,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
        TestApiStreamDesc {
            stream_id: 36,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
        TestApiStreamDesc {
            stream_id: 40,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
        TestApiStreamDesc {
            stream_id: 44,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
        TestApiStreamDesc {
            stream_id: 48,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
    ];

    for i in 0..=3 {
        let mut client_parameters = TransportParameters::default();
        let mut server_parameters = TransportParameters::default();
        init_transport_parameters(&mut client_parameters);
        init_transport_parameters(&mut server_parameters);

        client_parameters.enable_loss_bit = i & 1;
        server_parameters.enable_loss_bit = ((i > 1) as i32) & 1;

        let mut simulated_time = Instant::from_ticks(0);
        let mut test_ctx = tls_api_one_scenario_init_ex(
            &mut simulated_time,
            Version::InternalTest1,
            Some(&client_parameters),
            Some(&server_parameters),
            None,
        )
        .expect("loss_bit context");

        let mut loss_mask = 0u64;
        tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
            .unwrap_or_else(|e| {
                panic!(
                    "loss_bit handshake client={} server={}: {e:?}",
                    client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
                )
            });
        wait_client_connection_ready(&mut test_ctx, &mut simulated_time).unwrap_or_else(|e| {
            panic!(
                "loss_bit wait-ready client={} server={}: {e:?}",
                client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
            )
        });
        assert!(
            test_ctx.client_ready() && test_ctx.server_ready(),
            "loss_bit handshake did not reach ready client={} server={} client_state={:?} server_state={:?} time={}",
            client_parameters.enable_loss_bit,
            server_parameters.enable_loss_bit,
            test_ctx.qclient.first_cnx_mut().map(|cnx| cnx.state()),
            test_ctx.qserver.first_cnx_mut().map(|cnx| cnx.state()),
            simulated_time.ticks()
        );
        test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MANY_STREAMS).unwrap_or_else(
            |e| {
                panic!(
                    "loss_bit scenario-init client={} server={}: {e:?}",
                    client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
                )
            },
        );
        tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
            .unwrap_or_else(|e| {
                panic!(
                    "loss_bit data-loop client={} server={}: {e:?}",
                    client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
                )
            });
        let client_state = test_ctx.qclient.first_cnx_mut().map(|cnx| cnx.state());
        let server_state = test_ctx.qserver.first_cnx_mut().map(|cnx| cnx.state());
        assert!(
            test_ctx.test_finished,
            "loss_bit did not finish many-streams scenario client={} server={} client_state={:?} server_state={:?} time={}",
            client_parameters.enable_loss_bit,
            server_parameters.enable_loss_bit,
            client_state,
            server_state,
            simulated_time.ticks()
        );
        tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 250_000)
            .unwrap_or_else(|e| {
                panic!(
                    "loss_bit verify client={} server={}: {e:?}",
                    client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
                )
            });
    }
}
```
