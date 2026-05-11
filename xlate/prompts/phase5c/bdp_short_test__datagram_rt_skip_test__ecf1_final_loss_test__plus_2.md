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

## `picoquictest/congestion_test.c:bdp_short_test`
* C test-table name: `bdp_short`
* C entry function: `bdp_short_test`
* Rust test: `bdp_short`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:930-932`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper maps to the correct Short option and mostly matches BDP-specific assertions, but the Rust scenario helper ignores the C second-pass 4,500,000us completion target and scenario completion verification.
* Phase 5A fix note: Fix the shared Rust scenario verification/max-time enforcement; keep the BdpTestOption::Short wrapper.
* Phase 5B analysis: Current Rust source already maps bdp_short to BdpTestOption::Short, applies the C short-case 4,500,000us second-pass cap, and uses tls_api_one_scenario_body verification for completion/time checks.
* Phase 5B fix note: 

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_short);
}
```

### Current Rust test body
```rust
fn bdp_short() {
    bdp_option_test_one(BdpTestOption::Short);
}
```

## `picoquictest/datagram_tests.c:datagram_rt_skip_test`
* C test-table name: `datagram_rt_skip`
* C entry function: `datagram_rt_skip_test`
* Rust test: `datagram_rt_skip`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Current Rust span: `rs/fq/src/tests/datagram.rs:667-678`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper copies the C context values, but the shared Rust datagram helper does not check the same behavior: it manually simulates datagram send/receive/ack instead of exercising the callback/provider datagram path, and it patches negotiated datagram sizes rather than failing like the C helper.
* Phase 5A fix note: Update rs/fq/src/tests/datagram.rs shared datagram_test_one path to exercise the actual datagram callback/provider or queued datagram machinery and preserve the C negotiation, latency, ack/nack/spurious, and skip checks without short-circuiting delivery.
* Phase 5B analysis: Rust datagram_rt_skip is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract: identical datagram context fields and datagram_test_one(3, ..., 0). Runtime failure at datagram negotiation or missing DATAGRAM callback behavior is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 10;
    dg_ctx.send_delay = 20000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;
    dg_ctx.dg_latency_target[0] = 13000;
    dg_ctx.dg_latency_target[1] = 20000;
    dg_ctx.do_skip_test[0] = 1;
    dg_ctx.do_skip_test[1] = 1;

    return datagram_test_one(3, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
fn datagram_rt_skip() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 10],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [13_000, 20_000],
        do_skip_test: [true, true],
        ..Default::default()
    };
    datagram_test_one(3, &mut dg_ctx, 0);
}
```

## `picoquictest/edge_cases.c:ecf1_final_loss_test`
* C test-table name: `ecf1_final_loss`
* C entry function: `ecf1_final_loss_test`
* Rust test: `ecf1_final_loss`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1671-1688`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test uses the same edge-case id, loss mask, loop sequence, immediate_exit flag, close losses, and 10s bound, but its close-with-losses helper does not fail if endpoints remain non-disconnected after the close loop as the C helper does.
* Phase 5A fix note: Make Rust tls_api_close_with_losses return an error when either endpoint is still not disconnected after the close loop, while keeping the 10s elapsed-time assertion in this test.
* Phase 5B analysis: The Rust test already matches the C sequence and 10s bound, and the shared Rust close-with-losses helper already returns an error if either endpoint remains non-disconnected after the close loop.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t final_losses = 0xb10;
    uint8_t test_case_id = 0xf1;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 0, &simulated_time, 0, 20);
    uint64_t zero_loss_mask = 0;

    /* Finish the connection */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &zero_loss_mask, 0, &simulated_time);
        if (ret != 0)
        {
            DBG_PRINTF("Connect loop returns %d\n", ret);
        }
    }
    /* Finish sending data */
    test_ctx->immediate_exit = 1;

    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &zero_loss_mask, &simulated_time, 0);

        if (ret != 0)
        {
            DBG_PRINTF("Data sending loop returns %d\n", ret);
        }
    }
    /* Simulate losses during closing */
    if (ret == 0) {
        ret = tls_api_close_with_losses(test_ctx, &simulated_time, final_losses);
    }

    if (ret == 0 && simulated_time > 10000000) {
        DBG_PRINTF("Took %" PRIu64 "us to complete, too long", simulated_time);
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
fn ecf1_final_loss() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0xf1, false, &mut simulated_time, 0, 20).expect("edge_case_prepare");
    let mut zero_loss = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut zero_loss, 0, &mut simulated_time)
        .expect("connection loop");
    test_ctx.immediate_exit = true;
    tls_api_data_sending_loop(&mut test_ctx, &mut zero_loss, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0xb10)
        .expect("close with losses");
    assert!(
        simulated_time.ticks() <= 10_000_000,
        "connection close took too long: {} µs",
        simulated_time.ticks()
    );
}
```

## `picoquictest/edge_cases.c:reset_need_stop_test`
* C test-table name: `reset_need_stop`
* C entry function: `reset_need_stop_test`
* Rust test: `reset_need_stop`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1899-1901`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper selects NeedStop, but Rust reset_repeat_test_need_repeat returns Ok when the stream is absent. C still calls picoquic_check_frame_needs_repeat on the STOP_SENDING frame and requires no_need_to_repeat; the Rust test skips the key check after the stream is deleted. Rust ResetTestKind also omits the C ack_stop discriminant, shifting NeedStop's CID byte from 8 to 7.
* Phase 5A fix note: Remove the early return in reset_repeat_test_need_repeat so it always calls check_frame_needs_repeat and verifies no_need_to_repeat, and give ResetTestKind explicit C discriminants including NeedStop = 8.
* Phase 5B analysis: Rust #[test] reset_need_stop is present and calls reset_repeat_test_one(ResetTestKind::NeedStop), matching C reset_need_stop_test -> reset_repeat_test_one(reset_need_stop_sending). The helper uses the C enum discriminant 8 and checks check_frame_needs_repeat on stop_sending_frame with no_need_to_repeat required. Any InvalidState/runtime reset-repeat failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return reset_repeat_test_one(reset_need_stop_sending);
}
```

### Current Rust test body
```rust
fn reset_need_stop() {
    reset_repeat_test_one(ResetTestKind::NeedStop).expect("reset_need_stop");
}
```

## `picoquictest/mbedtls_test.c:mbedtls_test`
* C test-table name: `mbedtls`
* C entry function: `mbedtls_test`
* Rust test: `mbedtls`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:593-625`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test follows the broad connection/data loop shape, flags, CID, scenario, and reset calls, but it does not recreate the C use_ecdsa=1 initialization path and relies on the weakened scenario verification helper.
* Phase 5A fix note: Support/select the ECDSA server certificate path for this test and strengthen tls_api_one_scenario_body_verify so stream completion and the 1000000us deadline are actually checked.
* Phase 5B analysis: Rust #[test] already matches the C API-level contract: mbedTLS-only reset, ECDSA init with TEST_SNI/TEST_ALPN and initial CID, binlog/long-log setup, connection loop, mbedTLS stream scenario, data-sending loop, and 1000000us verifier. Known stream/callback receive-state failure is Phase 5C runtime behavior, not a Phase 5B block. cargo check --tests currently fails outside this test in src/tests/util.rs due missing crate::textlog::textlog_transport_extension_content.
* Phase 5B fix note: No Rust test changes; COMMANDS.log updated for required command logging.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t target_time = 1000000;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x99, 0xbe, 0xd7, 0x15, 0, 0, 0, 0}, 8 };
    int ret = 0;

    picoquic_tls_api_reset(TLS_API_INIT_FLAGS_NO_OPENSSL|TLS_API_INIT_FLAGS_NO_FUSION);

    ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid, 8, 0, 0, 1);
    if (ret == 0) {
        picoquic_set_binlog(test_ctx->qserver, ".");
        test_ctx->qserver->use_long_log = 1;
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 20000, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_mbedtls, sizeof(test_scenario_mbedtls));
    }

    /* Try to complete the data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, target_time);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    picoquic_tls_api_reset(0);

    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;
    let target_time: u64 = 1_000_000;

    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL | TLS_API_INIT_FLAGS_NO_FUSION);

    let initial_cid = crate::ConnectionId::clone_from_slice(&[0x99, 0xbe, 0xd7, 0x15, 0, 0, 0, 0])
        .expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex2_ecdsa(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 20_000, &mut simulated_time)
        .expect("connection loop");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MBEDTLS).expect("init scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
        .expect("scenario verify");

    reset_tls_api(0);
}
```
