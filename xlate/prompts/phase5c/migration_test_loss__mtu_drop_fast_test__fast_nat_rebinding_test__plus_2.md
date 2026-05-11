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

## `picoquictest/tls_api_test.c:migration_test_loss`
* C test-table name: `migration_with_loss`
* C entry function: `migration_test_loss`
* Rust test: `migration_with_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4304-4306`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes loss mask 0x09, but uses an empty/default scenario instead of C's q_and_r 257/2000 case and omits the C migration checks for challenge renewal/verification and connection ID changes.
* Phase 5A fix note: Pass the q_and_r scenario and implement the C helper's migration assertions: propagate probe errors, apply loss during transfer, verify scenario completion, wait for path challenge validation, and check remote/local CID changes.
* Phase 5B analysis: Rust #[test] migration_with_loss is present, compiles, and calls migration_test_scenario(TEST_SCENARIO_Q_AND_R, 0x09, false), matching the C q_and_r/loss_mask/cid_zero API contract. Any failure from probe_new_path is Phase 5C runtime behavior.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t loss_mask = 0x09;

    return migration_test_scenario(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), loss_mask, 0);
}
```

### Current Rust test body
```rust
fn migration_with_loss() {
    migration_test_scenario(TEST_SCENARIO_Q_AND_R, 0x09, false).expect("migration_with_loss");
}
```

## `picoquictest/tls_api_test.c:mtu_drop_fast_test`
* C test-table name: `mtu_drop_fast`
* C entry function: `mtu_drop_fast_test`
* Rust test: `mtu_drop_fast`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4394-4396`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls the named helper with fast/11_500_000, but the helper ignores the CC algorithm, uses different link timing, does not assert MTU discovery, does not drop path MTU, and the verifier ignores the target time.
* Phase 5A fix note: Make Rust mtu_drop_cc_algotest select the requested algorithm, match the C link setup, wait for and assert MTU discovery, lower both path MTUs, then complete and verify the very-long transfer within the target time.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, cargo check --tests passes, and it calls mtu_drop_cc_algotest("fast", 11_500_000), matching C's mtu_drop_cc_algotest(picoquic_fastcc_algorithm, 11500000). The prior 100ms/1Mbps handshake failure is Phase 5C runtime implementation work, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_fastcc_algorithm, 11500000);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_drop_fast() {
    mtu_drop_cc_algotest("fast", 11_500_000).expect("mtu_drop_fast");
}
```

## `picoquictest/tls_api_test.c:fast_nat_rebinding_test`
* C test-table name: `nat_rebinding_fast`
* C entry function: `fast_nat_rebinding_test`
* Rust test: `nat_rebinding_fast`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4848-4850`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the generic no-loss TLS helper; it does not enable NAT, repeatedly change the client NAT port, require six rebinding switches, or verify sustained data completion after those switches.
* Phase 5A fix note: Implement a Rust fast NAT rebinding test/helper mirroring the C loop: initial CID, qlog setup, sustained scenario, NAT port increments after server observes each port, require at least 6 switches, then verify scenario completion.
* Phase 5B analysis: Reclassified ok: the Rust test is present under #[test], compiles/runs through the Rust harness, and mirrors the C API-level contract; the observed missing NAT peer tuple switch is a Phase 5C implementation failure.
* Phase 5B fix note: 

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
fn nat_rebinding_fast() {
    fast_nat_rebinding_test().expect("nat_rebinding_fast");
}
```

## `picoquictest/tls_api_test.c:optimistic_ack_test`
* C test-table name: `optimistic_ack`
* C entry function: `optimistic_ack_test`
* Rust test: `optimistic_ack`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5439-5441`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes true but optimistic_ack_test_one ignores the flag and just runs a normal scenario; it does not enable optimistic ACK hole insertion, spoof ACKs, count holes, reject unexpected retransmissions, or expect failure for the spoofed case.
* Phase 5A fix note: Port optimistic_ack_test_one behavior: initial CID variant, server optimistic-ACK policy, deterministic random seed, very_long scenario, hole/spoof loop, hole-count checks, and spoofed-transfer failure expectation.
* Phase 5B analysis: Rust test is present, compiles as a harness test, and calls optimistic_ack_test_one(true), matching the C entry's optimistic_ack_test_one(1). The helper already expresses the API-level contract: optimistic-ACK policy, deterministic seed, trap-hole scan, spoofed PN recording, retransmission rejection, hole counters, and spoofed-transfer failure expectation. The missing observed ACK-trap hole/runtime failure is a Phase 5C library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

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
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6108-6111`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs the generic tls_api_test_with_loss smoke path, while C runs pn_random_test_one twice, for initial-only and all packet-number spaces, setting random_initial and checking client/server packet-context sequence thresholds.
* Phase 5A fix note: Add a faithful Rust pn_random_test_one and have pn_random run both false and true cases: configure random_initial=1/2 on both endpoints, force the client sequence numbers as C does, verify client and server packet-number spaces against PICOQUIC_PN_RANDOM_MIN, then complete the q-and-r scenario.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract: it runs initial-only then all-PN-space randomization, checks client/server packet contexts, and executes the q-and-r scenario. The prior handshake/server-accept failure is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

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
