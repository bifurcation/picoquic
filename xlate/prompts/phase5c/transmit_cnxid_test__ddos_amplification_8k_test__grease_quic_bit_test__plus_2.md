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

## `picoquictest/tls_api_test.c:transmit_cnxid_test`
* C test-table name: `cnxid_transmit`
* C entry function: `transmit_cnxid_test`
* Rust test: `cnxid_transmit`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1665-1667`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper maps the three false flags correctly, but the Rust helper only runs a basic connection/sync/wait/close path. It does not verify CID counts, PICOQUIC_NB_PATH_TARGET behavior, or local/remote CID stash correspondence, and it ignores the variant flags.
* Phase 5A fix note: Implement the C helper's checks in Rust: honor retire-before, disable-migration, and early-retire flags; sync until enough CIDs are issued; verify CID counts; and compare each side's remote CID stash with the peer local CID list.
* Phase 5B analysis: Rust #[test] cnxid_transmit is present, compiles under the Rust test harness, and calls transmit_cnxid_test_one(false, false, false), matching the C wrapper transmit_cnxid_test_one(0, 0, 0). The helper expresses the C API-visible contract; remaining NEW_CONNECTION_ID/PATH_NEW_CONNECTION_ID runtime behavior is Phase 5C, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    return transmit_cnxid_test_one(0, 0, 0);
}
```

### Current Rust test body
```rust
fn cnxid_transmit() {
    transmit_cnxid_test_one(false, false, false).expect("cnxid_transmit");
}
```

## `picoquictest/tls_api_test.c:ddos_amplification_8k_test`
* C test-table name: `ddos_amplification_8k`
* C entry function: `ddos_amplification_8k_test`
* Rust test: `ddos_amplification_8k`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1954-1956`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes (2, 0) instead of the C (0, 1), and the Rust helper ignores its parameters and runs a normal handshake/close rather than the amplification-limit simulation.
* Phase 5A fix note: Change the Rust test to the 8k case parameters, e.g. (0, 1) or booleans false/true, and implement the helper to set large server flight, send only the first client packet, count bytes, require server <= 3x client, and require server disconnect.
* Phase 5B analysis: Rust #[test] is present and calls ddos_amplification_test_one(0, 1), matching the C entry. The helper expresses the same API-level contract, including the 8k server-flight flag, one client packet, server-only send loop, disconnect check, and 3x amplification assertion. The known invalid Initial/prepare_packet runtime failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return ddos_amplification_test_one(0, 1);
}
```

### Current Rust test body
```rust
fn ddos_amplification_8k() {
    ddos_amplification_test_one(0, 1).expect("ddos_amplification_8k");
}
```

## `picoquictest/tls_api_test.c:grease_quic_bit_test`
* C test-table name: `grease_quic_bit`
* C entry function: `grease_quic_bit_test`
* Rust test: `grease_quic_bit`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2799-2801`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper passes the correct false flag and checks the main GREASE bit flags, but the helper changes the C very_long scenario input and relies on a Rust scenario verifier that only closes instead of verifying stream completion.
* Phase 5A fix note: Use the C-equivalent very_long stream input (q_len 257, r_len 1000000) and ensure scenario completion is asserted before the existing quic_bit_greased/quic_bit_received_0 checks.
* Phase 5B analysis: Rust test is present, #[test]-runnable, compiles, and calls grease_quic_bit_test_one(false), matching C grease_quic_bit_test_one(0). The helper already expresses the C API-visible contract: GREASE transport parameter setup, very-long scenario completion verification, and symmetric quic_bit_greased/quic_bit_received_0 assertions. Any early runtime failure is Phase 5C library behavior.
* Phase 5B fix note: 

### C test body
```c
{
    return  grease_quic_bit_test_one(0);
}
```

### Current Rust test body
```rust
fn grease_quic_bit() {
    grease_quic_bit_test_one(false).expect("grease_quic_bit");
}
```

## `picoquictest/tls_api_test.c:initial_race_test`
* C test-table name: `initial_race`
* C entry function: `initial_race_test`
* Rust test: `initial_race`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3168-3170`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is just a generic handshake/close. It omits the forced repeated Initial packet, the two-packet initial-queue check, the delayed server wake, the q2/r2 data scenario, and scenario verification.
* Phase 5A fix note: Translate the full initial_race sequence: run one sim round, set initial_repeat_needed, verify two queued client Initial packets, wait for first server packet, delay server next_wake_time, complete connection, run q2/r2 transfer, verify, and close.
* Phase 5B analysis: Current Rust test is present, compiles as a Rust test, and matches the C API-level flow: init, first sim round, forced Initial repeat with two-packet queue assertion, wait for server packet, delay server wake, connection loop, q2/r2 scenario, verify, and close. Any failure to queue packets or complete handshake is Phase 5C runtime behavior.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    }

    /* Run an initial loop to make to send the client's first packet, and then replicate it. */
    if (ret == 0) {
        int was_active = 0;
        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

        if (ret == 0) {
            /* Force a repeat of the first packet */
            simulated_time += 100;
            test_ctx->cnx_client->initial_repeat_needed = 1;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            test_ctx->cnx_client->initial_repeat_needed = 0;
            /* Verify that there are two packets in the initial queue */
            if (ret == 0) {
                if (test_ctx->c_to_s_link->first_packet == NULL) {
                    DBG_PRINTF("%s", "No packet queued");
                    ret = -1;
                }
                else if (test_ctx->c_to_s_link->last_packet == test_ctx->c_to_s_link->first_packet) {
                    DBG_PRINTF("%s", "Only one packet queued");
                    ret = -1;
                }
            }
        }

        while (ret == 0 && test_ctx->s_to_c_link->first_packet == NULL){
            /* run a couple of simulation round to process the first server packets,
             * but make sure the server sends only one packet */
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
        }

        if (ret == 0) {
            if (test_ctx->cnx_server == NULL) {
                DBG_PRINTF("%s", "No server connection");
                ret = -1;

            }
            else {
                /* Make sure that the server waits before sending the next packet. */
                test_ctx->cnx_server->next_wake_time += 2000;
            }
        }
    }

    /* Run a connection loop */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2));
    }

    /* Try send data */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* Check that the data was sent and received */
    if (ret == 0) {
        ret = tls_api_one_scenario_verify(test_ctx);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);

        if (ret != 0)
        {
            DBG_PRINTF("Connection close returns %d\n", ret);
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
fn initial_race() {
    initial_race_test_one().expect("initial_race");
}
```

## `picoquictest/tls_api_test.c:long_rtt_test`
* C test-table name: `long_rtt`
* C entry function: `long_rtt_test`
* Rust test: `long_rtt`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3654-3681`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust sets 300 ms link latency but runs an empty scenario, omits the C very-long 1 MB transfer, omits queue_delay_max=2*latency, and uses a much looser completion target.
* Phase 5A fix note: Run the very-long scenario {stream 4, q_len 257, r_len 1000000}, preserve 300 ms each-way latency, pass queue_delay_max=600000, and verify completion against the C 3600000 usec target.
* Phase 5B analysis: Rust long_rtt already matches the C API-level contract and compiles/runs under the Rust test harness; the known very-long transfer failure is a Phase 5C implementation/runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    uint64_t latency = 300000ull; /* assume that each direction is 300 ms, e.g. satellite link */
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x10, 0x10, 30, 0, 0, 0, 0, 0}, 8 };

    ret = tls_api_one_scenario_init_ex(&test_ctx, &simulated_time,
        0, NULL, NULL, &initial_cid);

    if (ret == 0) {
        /* set the delay estimate, then launch the test */
        test_ctx->c_to_s_link->microsec_latency = latency;
        test_ctx->s_to_c_link->microsec_latency = latency;

        picoquic_set_qlog(test_ctx->qserver, ".");

        /* The transmission delay cannot be less than 2.6 sec:
         * 3 handshakes at 1 RTT each = 1.8 sec, plus
         * 1MB over a 10Mbps link = 0.8 sec. We observe
         * 3.31 seconds instead, i.e. 1.51 sec for the
         * data transmission. This is due to the slow start
         * phase of the congestion control, which we accelerated
         * but could not completely fix. */
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 0, 2*latency,
            3600000);
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
fn long_rtt() {
    const LATENCY: u64 = 300_000;
    const TEST_SCENARIO_VERY_LONG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    let mut t = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x10, 0x10, 30, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut ctx = tls_api_init_ctx_ex(&mut t, 0, None, Some(&initial_cid)).expect("ctx");
    ctx.c_to_s_link.microsec_latency = LATENCY;
    ctx.s_to_c_link.microsec_latency = LATENCY;
    ctx.qserver.set_qlog(".").ok();
    tls_api_one_scenario_body(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_VERY_LONG,
        0,
        0,
        0,
        2 * LATENCY,
        3_600_000,
    )
    .expect("long_rtt");
}
```
