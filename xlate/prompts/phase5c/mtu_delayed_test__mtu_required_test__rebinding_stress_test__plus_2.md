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

## `picoquictest/tls_api_test.c:mtu_delayed_test`
* C test-table name: `mtu_delayed`
* C entry function: `mtu_delayed_test`
* Rust test: `mtu_delayed`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4342-4350`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses delayed policy and a very-long scenario, but it does not set the server PMTUD policy and never checks the expected client/server MTUs.
* Phase 5A fix note: Set delayed PMTUD on both server default and client, run `test_scenario_very_long`, then assert client `send_mtu == 1252` and server `send_mtu == 1440`.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract: delayed PMTUD, very-long scenario {4,0,257,1000000}, and expected client/server MTUs 1252/1440. Any runtime failure from incomplete PMTUD send_mtu promotion is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_delayed, 1252, 1440, 
        test_scenario_very_long, sizeof(test_scenario_very_long), 0);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_delayed() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];
    mtu_discovery_test_one(PmtudPolicy::Delayed, 1252, 1440, &scenario, 0).expect("mtu_delayed");
}
```

## `picoquictest/tls_api_test.c:mtu_required_test`
* C test-table name: `mtu_required`
* C entry function: `mtu_required_test`
* Rust test: `mtu_required`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4425-4433`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes policy 3, which maps to Blocked, while C uses Required. The Rust helper also uses the wrong stream scenario and omits the C MTU equality checks.
* Phase 5A fix note: Use Required policy, the q_and_r scenario with 257-byte query and 2000-byte response, and assert client/server send_mtu are both 1440.
* Phase 5B analysis: Current Rust test already matches C: Required PMTUD policy, q_and_r scenario {stream_id:4, previous:0, q_len:257, r_len:2000}, and helper checks client/server send_mtu are both 1440.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_required, 1440, 1440, 
        test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_required() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2_000,
    }];
    mtu_discovery_test_one(PmtudPolicy::Required, 1440, 1440, &scenario, 0).expect("mtu_required");
}
```

## `picoquictest/tls_api_test.c:rebinding_stress_test`
* C test-table name: `nat_rebinding_stress`
* C entry function: `rebinding_stress_test`
* Rust test: `nat_rebinding_stress`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5019-5021`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the generic handshake/close helper. It omits the C stress loop, fake-address packet injection, NAT rebinding trigger, multi-address relay/drop logic, long stream scenario, and final stream-completion checks.
* Phase 5A fix note: Implement a Rust nat rebinding stress helper mirroring the C loop: very_long scenario, 10000-trial bounded sim loop, deterministic random injection from bad addresses, client NAT rebinding after server app packet sequence >128, attacker relay disable after >256, and final callback/stream verification.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust harness, and the helper mirrors the C API-level contract: init, connection loop, very-long scenario setup, multiple-address stress loop with packet reinjection/drop behavior, NAT rebinding, close, and scenario verification. Any early runtime failure from incomplete handshake/no packets is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int nb_trials = 0;
    const int max_trials = 10000;
    int nb_inactive = 0;
    int client_rebinding_done = 0;
    struct sockaddr_in hack_address;
    struct sockaddr_in hack_address_random;
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t last_inject_time = 0;
    uint64_t random_context = 0xBABAC001CAFEull;
    picoquictest_sim_packet_t* last_client_packet_processed = NULL;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    }

    if (ret == 0) {
        memcpy(&hack_address, &test_ctx->client_addr, sizeof(struct sockaddr_in));
        memcpy(&hack_address_random, &test_ctx->client_addr, sizeof(struct sockaddr_in));

        hack_address.sin_port += 1023;

        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    /* Rewrite the sending loop, so we can add injection of packet copies */
    if (ret == 0) {
        test_ctx->client_use_multiple_addresses = 1;
    }

    while (ret == 0 && nb_trials < max_trials && nb_inactive < 256 && TEST_CLIENT_READY && TEST_SERVER_READY) {
        int was_active = 0;

        nb_trials++;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

        if (ret < 0)
        {
            break;
        }

        if (was_active) {
            nb_inactive = 0;
        }
        else {
            nb_inactive++;
        }

        if (test_ctx->test_finished) {
            if (picoquic_is_cnx_backlog_empty(test_ctx->cnx_client) && picoquic_is_cnx_backlog_empty(test_ctx->cnx_server)) {
                break;
            }
        }

        /* Packet injection at the server */
        if (test_ctx->c_to_s_link->last_packet != NULL) {
            uint64_t server_arrival = test_ctx->c_to_s_link->last_packet->arrival_time;

            if (server_arrival > last_inject_time) {
                /* 9% chance of packet injection, 5% chances of reusing test address */
                uint64_t rand100 = picoquic_test_uniform_random(&random_context, 100);
                last_inject_time = server_arrival;
                if (rand100 < 9) {
                    struct sockaddr * bad_address;
                    if (rand100 < 5) {
                        bad_address = (struct sockaddr *)&hack_address;
                    }
                    else {
                        hack_address_random.sin_port = (uint16_t)picoquic_test_uniform_random(&random_context, 0x10000);
                        bad_address = (struct sockaddr *)&hack_address_random;
                    }
                    ret = picoquic_incoming_packet(test_ctx->qserver,
                        test_ctx->c_to_s_link->last_packet->bytes,
                        (uint32_t)test_ctx->c_to_s_link->last_packet->length,
                        bad_address,
                        (struct sockaddr*)&test_ctx->c_to_s_link->last_packet->addr_to, 0, test_ctx->recv_ecn_server,
                        simulated_time);
                }
            }
        }

        /* Initially, the attacker relays packets to the client. Then, it gives up */
        if (test_ctx->cnx_server->pkt_ctx[picoquic_packet_context_application].send_sequence > 256) {
            test_ctx->client_use_multiple_addresses = 0;
        }

        if (test_ctx->client_use_multiple_addresses && test_ctx->s_to_c_link->last_packet != NULL &&
            test_ctx->s_to_c_link->last_packet != last_client_packet_processed){
            last_client_packet_processed = test_ctx->s_to_c_link->last_packet;
            /* Packet reinjection at the client if using the special address */
            if (picoquic_compare_addr((struct sockaddr *)&hack_address, (struct sockaddr *)&test_ctx->s_to_c_link->last_packet->addr_to) == 0)
            {
                picoquic_store_addr(&test_ctx->s_to_c_link->last_packet->addr_to, (struct sockaddr *)&test_ctx->client_addr);
            }
            else if (test_ctx->client_use_nat) {
                if (picoquic_compare_addr((struct sockaddr*) & test_ctx->client_addr_natted, (struct sockaddr*) & test_ctx->s_to_c_link->last_packet->addr_to) == 0) {
                    picoquic_store_addr(&test_ctx->s_to_c_link->last_packet->addr_to, (struct sockaddr*) & test_ctx->client_addr);
                }
                else {
                    /* This packet should be dropped on arrival */
                    test_ctx->s_to_c_link->last_packet->length = 1;
                }
            }
            else if (picoquic_compare_addr((struct sockaddr*) & test_ctx->client_addr, (struct sockaddr*) & test_ctx->s_to_c_link->last_packet->addr_to) != 0) {
                /* This packet should be dropped on arrival */
                test_ctx->s_to_c_link->last_packet->length = 1;
            }
        }

        /* At some point, the client does migrate to a new address */
        if (!client_rebinding_done && test_ctx->cnx_server->pkt_ctx[picoquic_packet_context_application].send_sequence > 128) {
            test_ctx->client_addr_natted = test_ctx->client_addr;
            test_ctx->client_addr_natted.sin_port += 17;
            test_ctx->client_use_nat = 1;
            client_rebinding_done = 1;
        }
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        if (test_ctx->server_callback.error_detected) {
            ret = -1;
        }
        else if (test_ctx->client_callback.error_detected) {
            ret = -1;
        }
        else {
            for (size_t i = 0; ret == 0 && i < test_ctx->nb_test_streams; i++) {
                if (test_ctx->test_stream[i].q_recv_nb != test_ctx->test_stream[i].q_len) {
                    ret = -1;
                }
                else if (test_ctx->test_stream[i].r_recv_nb != test_ctx->test_stream[i].r_len) {
                    ret = -1;
                }
                else if (test_ctx->test_stream[i].q_received == 0 || test_ctx->test_stream[i].r_received == 0) {
                    ret = -1;
                }
            }
        }
        if (ret != 0)
        {
            DBG_PRINTF("Test scenario verification returns %d\n", ret);
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
fn nat_rebinding_stress() {
    nat_rebinding_stress_test().expect("nat_rebinding_stress");
}
```

## `picoquictest/tls_api_test.c:packet_trace_test`
* C test-table name: `packet_trace`
* C entry function: `packet_trace_test`
* Rust test: `packet_trace`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5717-5719`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic TLS-with-loss helper. It does not set the fixed initial CID, enable server binlog/lossbit/long-log policy, run the very_long scenario, convert the binlog to CSV, or compare against packet_trace_ref.txt.
* Phase 5A fix note: Mirror the C packet trace workflow: fixed initial CID ace1020304050607, server binlog '.', lossbit SendReceive on both contexts, server long log, very_long scenario with queue delay 20000 and 1000000 target, then CSV conversion and reference-file comparison.
* Phase 5B analysis: Rust test is present, compiles, and is harness-runnable. It matches the C API-level contract: fixed initial CID, server binlog setup, SendReceive lossbit policy on both contexts, server long-log flag, very-long scenario, CC binlog-to-CSV conversion, and reference comparison. Missing server binlog output is a Phase 5C runtime/library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xac, 0xe1, 2, 3, 4, 5, 6, 7}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Set the logging policy on the server side, to store data in the
     * current working directory, and run a basic test scenario */
    if (ret == 0) {
        picoquic_set_binlog(test_ctx->qserver, ".");
        picoquic_set_default_lossbit_policy(test_ctx->qserver, picoquic_lossbit_send_receive);
        picoquic_set_default_lossbit_policy(test_ctx->qclient, picoquic_lossbit_send_receive);
        test_ctx->qserver->use_long_log = 1;
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 0, 20000, 1000000);
    }

    /* Free the resource, which will close the log file.
     */

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    /* Create a CSV file from the .bin log file */
    if (ret == 0) {
        ret = picoquic_cc_log_file_to_csv(PACKET_TRACE_BIN, PACKET_TRACE_CSV);
    }

    /* compare the log file to the expected value */
    if (ret == 0)
    {
        char packet_trace_test_ref[512];

        ret = picoquic_get_input_path(packet_trace_test_ref, sizeof(packet_trace_test_ref), picoquic_solution_dir, PACKET_TRACE_TEST_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the packet trace test ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(PACKET_TRACE_CSV, packet_trace_test_ref);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn packet_trace() {
    packet_trace_impl().expect("packet_trace");
}
```

## `picoquictest/tls_api_test.c:preferred_address_dis_mig_test`
* C test-table name: `preferred_address_dis_mig`
* C entry function: `preferred_address_dis_mig_test`
* Rust test: `preferred_address_dis_mig`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6371-6373`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust configures a preferred address and migration_disabled=true, but then only runs the generic scenario. It does not verify client/server migration to the preferred address, CID updates, or that migration-disabled flags are cleared/authorized as the C helper requires.
* Phase 5A fix note: Extend preferred_address_test_one to assert the preferred server address is promoted on both endpoints, CIDs are updated when cid_zero is false, and migration_disabled is no longer set after preferred-address migration.
* Phase 5B analysis: Rust #[test] is present, compiles, and calls preferred_address_test_one(true, false), matching C preferred_address_test_one(1, 0). The helper asserts the same preferred-address path, CID, and migration-flag contract; any early Generic runtime failure is Phase 5C implementation behavior.
* Phase 5B fix note: 

### C test body
```c
{
    return preferred_address_test_one(1, 0);
}
```

### Current Rust test body
```rust
fn preferred_address_dis_mig() {
    preferred_address_test_one(true, false).expect("preferred_address_dis_mig");
}
```
