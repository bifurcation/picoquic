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

## `picoquictest/tls_api_test.c:connection_drop_test`
* C test-table name: `connection_drop`
* C entry function: `connection_drop_test`
* Rust test: `connection_drop`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1868-1931`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test is a normal successful connection helper call. It does not iterate the 9 target client/server handshake states, stop at each partial state, prepare packets on the abandoned side, or verify timely disconnect.
* Phase 5A fix note: Port connection_drop_test_one and loop over the C target_state/target_is_client table, checking disconnect behavior for each target state.
* Phase 5B analysis: Rust test is present under #[test], compiles under the Rust test harness, and mirrors the C target-state table plus API-visible drop helper contract. Prior ClientRenegotiate timeout is a Phase 5C runtime/library behavior issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    picoquic_state_enum target_state[9] = {
        picoquic_state_client_init_sent,
        picoquic_state_client_renegotiate,
        picoquic_state_client_init_resent,
        picoquic_state_server_init,
        picoquic_state_server_handshake,
        picoquic_state_client_handshake_start,
        picoquic_state_server_false_start,
        picoquic_state_server_almost_ready,
        picoquic_state_client_almost_ready
    };
    int target_is_client[9] = {
        1, 1, 1, 0, 0, 1, 0, 0, 1 };

    for (int i = 0; ret == 0 && i < 9; i++) {
        picoquic_state_enum c_state = (target_is_client[i]) ? target_state[i] : picoquic_state_ready;
        picoquic_state_enum s_state = (target_is_client[i]) ? picoquic_state_ready : target_state[i];

        ret = connection_drop_test_one(c_state, s_state, target_is_client[i]);
        if (ret == -1) {
            DBG_PRINTF("connection drop test %d fails", i);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn connection_drop() {
    const CASES: [ConnectionDropCase; 9] = [
        ConnectionDropCase {
            target_state: State::ClientInitSent,
            target_is_client: true,
        },
        ConnectionDropCase {
            target_state: State::ClientRenegotiate,
            target_is_client: true,
        },
        ConnectionDropCase {
            target_state: State::ClientInitResent,
            target_is_client: true,
        },
        ConnectionDropCase {
            target_state: State::ServerInit,
            target_is_client: false,
        },
        ConnectionDropCase {
            target_state: State::ServerHandshake,
            target_is_client: false,
        },
        ConnectionDropCase {
            target_state: State::ClientHandshakeStart,
            target_is_client: true,
        },
        ConnectionDropCase {
            target_state: State::ServerFalseStart,
            target_is_client: false,
        },
        ConnectionDropCase {
            target_state: State::ServerAlmostReady,
            target_is_client: false,
        },
        ConnectionDropCase {
            target_state: State::ClientAlmostReady,
            target_is_client: true,
        },
    ];

    for (i, case) in CASES.iter().copied().enumerate() {
        let target_client_state = if case.target_is_client {
            case.target_state
        } else {
            State::Ready
        };
        let target_server_state = if case.target_is_client {
            State::Ready
        } else {
            case.target_state
        };
        connection_drop_test_one(
            target_client_state,
            target_server_state,
            case.target_is_client,
        )
        .unwrap_or_else(|error| {
            panic!(
                "connection_drop case {i} failed: target={:?}, target_is_client={}, error={error:?}",
                case.target_state, case.target_is_client
            )
        });
    }
}
```

## `picoquictest/tls_api_test.c:excess_repeat_test`
* C test-table name: `excess_repeat`
* C entry function: `excess_repeat_test`
* Rust test: `excess_repeat`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2289-2302`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C iterates six congestion algorithms and runs excess_repeat_test_one with repeat limits after killing the client. Rust only runs the generic TLS success helper, with no CC loop, long transfer, client disappearance, repeat counting, or thresholds.
* Phase 5A fix note: Add/translate excess_repeat_test_one and make excess_repeat iterate newreno, cubic, dcubic, fastcc, bbr, and prague with the C repeat limits and disconnect/repeat assertions.
* Phase 5B analysis: Rust test is present as a runnable #[test], compiles, and matches the C API-level contract: six CC algorithms, repeat target 128, and the translated long-transfer/repeat-count helper. Phase 5C note: the early newreno readiness failure is runtime library behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    const int nb_repeat_max = 128;

    picoquic_congestion_algorithm_t* algo_list[6] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr_algorithm,
        picoquic_prague_algorithm
    };
    int ret = 0;

    for (int i = 0; i < 6 && ret == 0; i++) {
        ret = excess_repeat_test_one(algo_list[i], nb_repeat_max);
        if (ret != 0) {
            DBG_PRINTF("Excess repeat test fails for CC=%s", algo_list[i]->congestion_algorithm_id);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn excess_repeat() {
    const NB_REPEAT_MAX: usize = 128;
    const ALGORITHMS: &[&str] = &["newreno", "cubic", "dcubic", "fastcc", "bbr", "prague"];

    register_all_congestion_control_algorithms();

    for algo_id in ALGORITHMS {
        let cc_algo = get_congestion_algorithm(algo_id).unwrap_or_else(|| {
            panic!("congestion algorithm {algo_id} must be registered");
        });
        excess_repeat_test_one(cc_algo, NB_REPEAT_MAX)
            .unwrap_or_else(|e| panic!("excess_repeat({algo_id}): {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:immediate_close_test`
* C test-table name: `immediate_close`
* C entry function: `immediate_close_test`
* Rust test: `immediate_close`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2990-2992`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust again runs the generic handshake/close helper. It does not close the server immediately, send client data afterward, loop until the client observes disconnect, assert both endpoints disconnected, or assert the server packet count did not increase.
* Phase 5A fix note: Add a Rust immediate-close helper mirroring C: connect, wait ready, record server nb_packets_sent, call server close_immediate, client add_to_stream stream 4 with 128 bytes and FIN, run up to 256 rounds, then assert both disconnected and packet count unchanged.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, compiles under the Rust test harness, and its helper mirrors the C API-level contract: connect, wait ready, snapshot server packet count, close server immediately, send 128 bytes plus FIN on client stream 4, run up to 256 sim rounds, then assert client/server disconnected and unchanged server packet count. Any failure to establish a ready accepted server connection is Phase 5C runtime behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t nb_packet_sent_before_close = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    uint8_t buffer[128];
    int was_active = 0;

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    /* Immediate close */
    if (ret == 0) {
        nb_packet_sent_before_close = test_ctx->cnx_server->nb_packets_sent;
        picoquic_close_immediate(test_ctx->cnx_server);
    }
    /* Client sends some data, in order to test the connection */
    if (ret == 0) {
        memset(buffer, 0xaa, sizeof(buffer));
        ret = picoquic_add_to_stream(test_ctx->cnx_client, 4,
            buffer, sizeof(buffer), 1);
    }

    /* Perform a couple rounds of sending data */
    for (int i = 0; ret == 0 && i < 256 ; i++) {
        was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
        if (test_ctx->cnx_client->cnx_state >= picoquic_state_disconnected) {
            /* Client has noticed the disconnect */
            ret = 0;
            break;
        }
    }

    /* Client and server should now be in state disconnected */
    if (ret == 0 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
        ret = -1;
    }

    if (ret == 0 && test_ctx->cnx_server != NULL){
        if (test_ctx->cnx_server->cnx_state != picoquic_state_disconnected) {
            ret = -1;
        }
        else if (nb_packet_sent_before_close != test_ctx->cnx_server->nb_packets_sent)
        {
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
fn immediate_close() {
    immediate_close_test_one().expect("immediate_close");
}
```

## `picoquictest/tls_api_test.c:key_rotation_auto_server`
* C test-table name: `key_rotation_server`
* C entry function: `key_rotation_auto_server`
* Rust test: `key_rotation_server`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3389-3391`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust helper ignores the server/client distinction for this case by setting the client epoch length, uses a different one-stream scenario, and does not check the expected key-rotation count range. C server mode sets qserver default epoch length and verifies rotations against packet count bounds.
* Phase 5A fix note: For client_test=false, set the server default crypto epoch length before connection setup, use test_scenario_key_rotation, then assert nb_crypto_key_rotations is within the C min/max bounds computed from application send_sequence and epoch_length.
* Phase 5B analysis: Rust #[test] key_rotation_server is present, compiles under the test harness, and calls key_rotation_auto_one(300, false). The helper matches the C API-level contract: server default epoch length, two-stream key-rotation scenario, peer packet-count based nb_crypto_key_rotations bounds. Phase 5C note: incomplete automatic key-rotation runtime behavior is not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return key_rotation_auto_one(300, 0);
}
```

### Current Rust test body
```rust
fn key_rotation_server() {
    key_rotation_auto_one(300, false).expect("key_rotation_server");
}
```

## `picoquictest/tls_api_test.c:migration_disabled_test`
* C test-table name: `migration_disabled`
* C entry function: `migration_disabled_test`
* Rust test: `migration_disabled`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4238-4260`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust does not set the server migration_disabled transport parameter and does not assert the client received it; it only runs the generic handshake/close helper.
* Phase 5A fix note: Use a dedicated context with server default transport parameters setting migration_disabled=true, run the q_and_r scenario, then assert client remote transport parameters have migration_disabled=true.
* Phase 5B analysis: Rust test already matches the C API-level contract: initialize TLS API context, set server default migration_disabled, run q_and_r scenario, then assert the client-visible remote transport parameter. The q_and_r runtime failure is a Phase 5C library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Set the migration_disabled flag in the server parameter
     */
    if (ret == 0) {
        test_ctx->qserver->default_tp.migration_disabled = 1;

        /* Run a basic test scenario
         */

        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 250000);
    }

    /* verify that the migration was properly noticed as disabled. */
    if (ret == 0 && test_ctx->cnx_client != NULL &&
        !test_ctx->cnx_client->remote_parameters.migration_disabled) {
        DBG_PRINTF("%s", "Migration not disabled on client\n");
        ret = -1;
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
fn migration_disabled() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).expect("migration_disabled context");
    test_ctx.qserver.default_tp.migration_disabled = true;

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        0,
        0,
        0,
        0,
        250_000,
    )
    .expect("migration_disabled q_and_r scenario");

    assert!(
        test_ctx.cnx_client().remote_parameters.migration_disabled,
        "client did not receive migration_disabled from server transport parameters"
    );
}
```
