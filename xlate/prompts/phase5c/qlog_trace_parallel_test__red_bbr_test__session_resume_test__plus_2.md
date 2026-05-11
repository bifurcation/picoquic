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

## `picoquictest/tls_api_test.c:qlog_trace_parallel_test`
* C test-table name: `qlog_trace_parallel`
* C entry function: `qlog_trace_parallel_test`
* Rust test: `qlog_trace_parallel`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6585-6587`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes parallel=true but the helper ignores it and only runs a tiny scenario. It omits qlog/binlog setup, spin/loss-bit policies, CID callback/seeds, fixed crypto choices, recreated initial CID, bad-packet injection, and comparison with the qlog reference file.
* Phase 5A fix note: Implement qlog_trace_test_one with the C qlog setup and reference-file comparison; for parallel=true also enable binlog before running the q2/r2 scenario and bad-packet log check.
* Phase 5B analysis: Rust test is present as a runnable #[test] and calls qlog_trace_test_one(0, true), matching the C entry qlog_trace_test_one(0, 1). Helper source expresses the same API-level setup and reference-file assertion; any missing qlog/binlog file generation is a Phase 5C runtime/library issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return qlog_trace_test_one(0, 1);
}
```

### Current Rust test body
```rust
fn qlog_trace_parallel() {
    qlog_trace_test_one(0, true).expect("qlog_trace_parallel");
}
```

## `picoquictest/tls_api_test.c:red_bbr_test`
* C test-table name: `red_bbr`
* C entry function: `red_bbr_test`
* Rust test: `red_bbr`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6840-6842`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls red_cc_algotest("bbr", 500000, 170), but the helper treats 170 as an MTU, does not select BBR, does not configure RED AQM/link parameters, and does not assert observed loss <= 170.
* Phase 5A fix note: Use the third parameter as loss_target, select the requested congestion algorithm, configure RED on both links, run the sustained scenario, verify target time, and check retransmissions against the loss target.
* Phase 5B analysis: Rust red_bbr is present, compiles under the Rust test harness, and calls red_cc_algotest("bbr", 500_000, 170), matching the C API-level contract. Any Generic/BBR runtime failure is a Phase 5C implementation issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_bbr_algorithm, 500000, 170);
    return ret;
}
```

### Current Rust test body
```rust
fn red_bbr() {
    red_cc_algotest("bbr", 500_000, 170).expect("red_bbr");
}
```

## `picoquictest/tls_api_test.c:session_resume_test`
* C test-table name: `session_resume`
* C entry function: `session_resume_test`
* Rust test: `session_resume`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7337-7377`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust has a two-pass ticket-file helper, but it omits the C test's explicit second-handshake PSK assertions and does not fail when no ticket was received before saving.
* Phase 5A fix note: Update session_resume_test_one to preserve the two-connection flow while asserting nonempty client tickets before save and asserting client and server tls_is_psk_handshake on the second pass.
* Phase 5B analysis: Current Rust #[test] mirrors the C two-pass ticket-file flow, including max_early_data_size=0, first-pass ticket wait/save, close, nonempty ticket assertion, and second-pass client/server PSK assertions. Any missing ticket issuance or PSK behavior is a Phase 5C implementation failure, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    char const* sni = PICOQUIC_TEST_SNI;
    char const* alpn = PICOQUIC_TEST_ALPN;
    uint64_t loss_mask = 0;
    int ret = 0;

    /* Initialize an empty ticket store */
    ret = picoquic_save_tickets(NULL, simulated_time, ticket_file_name);

    for (int i = 0; i < 2; i++) {
        /* Set up the context, while setting the ticket store parameter for the client */
        if (ret == 0) {
            ret = tls_api_init_ctx(&test_ctx, 0, sni, alpn, &simulated_time, ticket_file_name, NULL, 0, 0, 0);
        }

        if (ret == 0) {
            test_ctx->cnx_client->max_early_data_size = 0;

            ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
        }

        if (ret == 0 && i == 1) {
            /* If resume succeeded, the second connection will have a type "PSK" */
            if (picoquic_tls_is_psk_handshake(test_ctx->cnx_server) == 0 || picoquic_tls_is_psk_handshake(test_ctx->cnx_client) == 0) {
                ret = -1;
            }
        }

        if (ret == 0 && i == 0) {
            /* Before closing, wait for the session ticket to arrive */
            ret = session_resume_wait_for_ticket(test_ctx, &simulated_time);
        }

        if (ret == 0) {
            ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
        }

        /* Verify that the session ticket has been received correctly */
        if (ret == 0) {
            if (test_ctx->qclient->p_first_ticket == NULL) {
                ret = -1;
            } else {
                ret = picoquic_save_tickets(test_ctx->qclient->p_first_ticket, simulated_time, ticket_file_name);
            }
        }
        /* Tear down and free everything */

        if (test_ctx != NULL) {
            tls_api_delete_ctx(test_ctx);
            test_ctx = NULL;
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn session_resume() {
    const TICKET_FILE: &str = "session_resume_test.bin";
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;

    save_empty_tickets(TICKET_FILE, simulated_time).expect("save_empty");

    for i in 0..2 {
        let mut test_ctx =
            tls_api_init_ctx(&mut simulated_time, 0, Some(TICKET_FILE)).expect("ctx");
        test_ctx.cnx_client().max_early_data_size = 0;

        tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
            .unwrap_or_else(|e| panic!("session_resume pass {i}: connection loop: {e:?}"));

        if i == 1 {
            let server_psk = test_ctx.cnx_server().tls_is_psk_handshake();
            let client_psk = test_ctx.cnx_client().tls_is_psk_handshake();
            assert!(
                server_psk && client_psk,
                "session_resume pass {i}: expected PSK handshake, client={client_psk} server={server_psk}",
            );
        }

        if i == 0 {
            session_resume_wait_for_ticket(&mut test_ctx, &mut simulated_time)
                .expect("wait_for_ticket");
        }

        tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("close");

        assert!(
            !test_ctx.qclient.stored_tickets.is_empty(),
            "session_resume pass {i}: no ticket received",
        );
        test_ctx
            .qclient
            .save_tickets(simulated_time, TICKET_FILE)
            .expect("save_tickets");
    }
}
```

## `picoquictest/tls_api_test.c:stop_sending_test`
* C test-table name: `stop_sending`
* C entry function: `stop_sending_test`
* Rust test: `stop_sending`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7939-7941`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses a different one-stream scenario, sends STOP_SENDING immediately, skips the C long-latency and initial partial-transfer loop, omits the C loss-mask setup, and does not verify the required postconditions: first response incomplete, later stream complete, callbacks clean, and data-node pool reclaimed.
* Phase 5A fix note: Port stop_sending_test_one more closely: use test_scenario_stop_sending with streams 4 and 8, set 100000 us link latency and initial loss mask, drive until first stream has response bytes before STOP_SENDING(error 1), then verify all C stream/error/memory postconditions before close.
* Phase 5B analysis: Current Rust #[test] is present and calls stop_sending_test_one(false, false), matching C stop_sending_test_one(0, 0). The helper expresses the C API-visible contract; any early failure from handshake readiness or STOP_SENDING/stream callback behavior is Phase 5C implementation work, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = stop_sending_test_one(0, 0);
    return ret;
}
```

### Current Rust test body
```rust
fn stop_sending() {
    stop_sending_test_one(false, false).expect("stop_sending");
}
```

## `picoquictest/tls_api_test.c:tls_api_sni_test`
* C test-table name: `tls_api_sni`
* C entry function: `tls_api_sni_test`
* Rust test: `tls_api_sni`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8128-8155`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls the similarly named helper, but that helper ignores SNI/ALPN parameters and its final check only closes; C verifies transport parameters, SNI, ALPN, and version.
* Phase 5A fix note: Make Rust tls_api_test_with_loss honor SNI/ALPN inputs and implement the final verification checks, especially SNI on client and server.
* Phase 5B analysis: Reclassified as ok: the Rust #[test] is present, compiles under the Rust test harness, initializes proposed_version=0 with TEST_SNI/TEST_ALPN, runs the handshake loop, checks API-visible transport parameters, SNI, ALPN, and negotiated version, then closes. The prior missing server-side connection at runtime is a Phase 5C implementation failure, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_test_with_loss(NULL, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN);
}
```

### Current Rust test body
```rust
fn tls_api_sni() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        0,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        None,
    )
    .expect("tls_api_sni ctx");
    let mut loss_mask = 0u64;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("tls_api_sni connection loop");
    {
        let client = test_ctx
            .qclient
            .first_cnx_mut()
            .expect("client connection not initialized");
        let server = test_ctx
            .qserver
            .first_cnx_mut()
            .expect("server connection not accepted");
        assert_tls_api_final_negotiation(client, server, Some(TEST_SNI), Some(TEST_ALPN));
    }
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("tls_api_sni close");
}
```
