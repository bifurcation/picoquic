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

## `picoquictest/tls_api_test.c:random_padding_test`
* C test-table name: `random_padding`
* C entry function: `random_padding_test`
* Rust test: `random_padding`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6675-6680`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs a generic TLS API handshake/loss helper. C runs two random-padding injections with pad lengths 128 and 16, shared deterministic RNG state, distinct test IDs, direct first-packet mutation, and then verifies the connection loop.
* Phase 5A fix note: Add a faithful random_padding_test_one Rust helper and call it twice with 128/test_id 0 and 16/test_id 1 using the same RNG context.
* Phase 5B analysis: Rust #[test] is present, compiles under the test harness, and matches the C API-level contract: shared RNG context, pad lengths 128 and 16, test IDs 0 and 1, initial CID mutation, first packet padding mutation, server ingest, connection loop, and ready-state assertion. The zero-length prepare_packet failure is a Phase 5C runtime-library issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t random_context = 0x1234567890abcdef;

    int ret = random_padding_test_one(128, &random_context, 0);

    if (ret == 0) {
        ret = random_padding_test_one(16, &random_context, 1);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn random_padding() {
    let mut random_context = 0x1234_5678_90ab_cdef;

    random_padding_test_one(128, &mut random_context, 0).expect("random_padding_128");
    random_padding_test_one(16, &mut random_context, 1).expect("random_padding_16");
}
```

## `picoquictest/tls_api_test.c:red_fast_test`
* C test-table name: `red_fast`
* C entry function: `red_fast_test`
* Rust test: `red_fast`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6864-6866`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the visible scalar parameters, but red_cc_algotest ignores the algorithm id and does not configure RED AQM, latency/bandwidth, server FastCC, qlog/long log, or the retransmission loss threshold checked by C.
* Phase 5A fix note: Make Rust red_cc_algotest select the requested congestion algorithm, configure RED on both simulated links with the C latency/queue parameters, run the sustained scenario, verify target completion time, and assert server retransmissions do not exceed the loss target.
* Phase 5B analysis: Rust #[test] is present, compiles, and delegates to red_cc_algotest("fast", 500_000, 250), matching the C API-level call and assertions. Any early Generic/runtime transfer failure is a Phase 5C library-behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_fastcc_algorithm, 500000, 250);
    return ret;
}
```

### Current Rust test body
```rust
fn red_fast() {
    red_cc_algotest("fast", 500_000, 250).expect("red_fast");
}
```

## `picoquictest/tls_api_test.c:test_stateless_blowback`
* C test-table name: `stateless_blowback`
* C entry function: `test_stateless_blowback`
* Rust test: `stateless_blowback`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7573-7655`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only performs a generic handshake/close; it does not exercise stateless reset blowback throttling, default interval checks, interval updates, or sent/not-sent outcomes for random 1-RTT packets.
* Phase 5A fix note: Add a Rust blowback helper that submits unknown-CID 1-RTT packets, drains prepare_next_packet, checks the C sequence of sent/not-sent outcomes, verifies the default interval, updates it to 2x default and then zero, and tests the immediate/+1 us cases.
* Phase 5B analysis: Rust test is present as a normal #[test], compiles under the Rust test harness, and matches the C API-level contract: initialize context with initial CID, verify default interval, inject unknown short-header packets through incoming_packet, drain prepare_next_packet, assert sent/not-sent timing, update interval, then verify zero-interval behavior. Current runtime stateless-reset blowback failures are Phase 5C implementation issues, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    int was_sent = 0;
    uint64_t new_interval = 2 * PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT;

    /* Create a context with the default timer. */
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;

    picoquic_connection_id_t initial_cid = { {0xb1, 0x08, 0xba, 0xcc, 0, 0, 0, 0}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN,
        &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);

    if (test_ctx->qserver->stateless_reset_min_interval != PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT) {
        DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
        ret = -1;

    }

    /* Format a random packet and submit it, verify that the stateless reset is queued */
    if (ret == 0) {
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("First stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Progress by 1/2 specified interval, retry, it should not work  */
    if (ret == 0) {
        simulated_time += test_ctx->qserver->stateless_reset_min_interval;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("Second stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }
    
    /* Progress by 1x specified interval, retry, it should work  */
    if (ret == 0) {
        simulated_time += test_ctx->qserver->stateless_reset_min_interval / 2;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && was_sent) {
            DBG_PRINTF("Third stateless reset was sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Reset the interval to twice the previous value */
    if (ret == 0) {
        picoquic_set_default_stateless_reset_min_interval(test_ctx->qserver, new_interval);
        if (test_ctx->qserver->stateless_reset_min_interval != new_interval) {
            DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
            ret = -1;
        }
    }
    
    /* Progress by 0.75x new interval, retry, it should work  */
    if (ret == 0) {
        simulated_time += (new_interval - new_interval / 4);
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After new interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Reset the interval to zero */
    if (ret == 0) {
        picoquic_set_default_stateless_reset_min_interval(test_ctx->qserver, 0);
        if (test_ctx->qserver->stateless_reset_min_interval != 0) {
            DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
            ret = -1;
        }
    }

    /* Try immediately, it should not work  */
    if (ret == 0) {
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After zero interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Add 1 microsec, it should work  */
    if (ret == 0) {
        simulated_time += 1;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After zero +1 interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Free the resurce and return */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn stateless_blowback() {
    let mut simulated_time = Instant::from_ticks(0);
    let new_interval = Duration::from_ticks(2 * MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT.ticks());
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xb1, 0x08, 0xba, 0xcc, 0, 0, 0, 0]).expect("initial cid");
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .expect("stateless_blowback ctx");

    assert_eq!(
        test_ctx.qserver.stateless_reset_min_interval, MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT,
        "default stateless reset interval"
    );

    assert!(
        stateless_blowback_one(&mut test_ctx.qserver, simulated_time).expect("first packet"),
        "first stateless reset was not sent at T={}",
        simulated_time.ticks()
    );

    simulated_time = Instant::from_ticks(
        simulated_time
            .ticks()
            .saturating_add(test_ctx.qserver.stateless_reset_min_interval.ticks()),
    );
    assert!(
        stateless_blowback_one(&mut test_ctx.qserver, simulated_time).expect("second packet"),
        "second stateless reset was not sent at T={}",
        simulated_time.ticks()
    );

    simulated_time = Instant::from_ticks(
        simulated_time
            .ticks()
            .saturating_add(test_ctx.qserver.stateless_reset_min_interval.ticks() / 2),
    );
    assert!(
        !stateless_blowback_one(&mut test_ctx.qserver, simulated_time).expect("third packet"),
        "third stateless reset was sent at T={}",
        simulated_time.ticks()
    );

    test_ctx
        .qserver
        .set_default_stateless_reset_min_interval(new_interval);
    assert_eq!(
        test_ctx.qserver.stateless_reset_min_interval, new_interval,
        "updated stateless reset interval"
    );

    simulated_time = Instant::from_ticks(
        simulated_time
            .ticks()
            .saturating_add(new_interval.ticks() - new_interval.ticks() / 4),
    );
    assert!(
        stateless_blowback_one(&mut test_ctx.qserver, simulated_time).expect("after new interval"),
        "after new interval, stateless reset was not sent at T={}",
        simulated_time.ticks()
    );

    test_ctx
        .qserver
        .set_default_stateless_reset_min_interval(Duration::from_ticks(0));
    assert_eq!(
        test_ctx.qserver.stateless_reset_min_interval,
        Duration::from_ticks(0),
        "zero stateless reset interval"
    );

    assert!(
        stateless_blowback_one(&mut test_ctx.qserver, simulated_time).expect("after zero interval"),
        "after zero interval, stateless reset was not sent at T={}",
        simulated_time.ticks()
    );

    simulated_time = Instant::from_ticks(simulated_time.ticks().saturating_add(1));
    assert!(
        stateless_blowback_one(&mut test_ctx.qserver, simulated_time)
            .expect("after zero +1 interval"),
        "after zero +1 interval, stateless reset was not sent at T={}",
        simulated_time.ticks()
    );
}
```

## `picoquictest/tls_api_test.c:tls_api_test`
* C test-table name: `tls_api`
* C entry function: `tls_api_test`
* Rust test: `tls_api`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7987-8014`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers basic connection setup and close, but its tls_api_test_with_loss_final omits the C final assertions for transport parameters, SNI, ALPN, and negotiated version.
* Phase 5A fix note: Extend the Rust TLS final helper or this test to verify client/server transport parameters, SNI, ALPN, and negotiated version before closing, matching tls_api_test_with_loss_final.
* Phase 5B analysis: Current Rust #[test] mirrors the C API-level path: V1/SNI/ALPN init, connection loop, final transport-parameter/SNI/ALPN/version assertions, then close. Any early handshake/server-acceptance or transport-parameter exchange failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_test_with_loss(NULL, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN);
}
```

### Current Rust test body
```rust
fn tls_api() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        V1,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        None,
    )
    .expect("tls_api ctx");
    let mut loss_mask = 0u64;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("tls_api connection loop");
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
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("tls_api close");
}
```

## `picoquictest/tls_api_test.c:tls_api_very_long_stream_test`
* C test-table name: `tls_api_very_long_stream`
* C entry function: `tls_api_very_long_stream_test`
* Rust test: `tls_api_very_long_stream`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8216-8230`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C runs test_scenario_very_long with stream 4 query length 257 and response length 1000000, then verifies scenario completion and the 1000000 us target. Rust passes an empty scenario, so it does not exercise the very-long stream transfer.
* Phase 5A fix note: Use the Rust equivalent of test_scenario_very_long and ensure the helper verifies stream byte counts/completion and the max completion threshold.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, and expresses the C API-level contract: very-long scenario {stream 4, previous 0, q_len 257, r_len 1000000}, no loss, default params/proposed version, and 1_000_000us completion target. Any runtime Generic failure from incomplete callback/byte-count behavior is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0, 0, 0, 0, 1000000, NULL, NULL);
}
```

### Current Rust test body
```rust
fn tls_api_very_long_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_VERY_LONG,
        0,
        0,
        0,
        0,
        1_000_000,
    )
    .expect("very_long_stream");
}
```
