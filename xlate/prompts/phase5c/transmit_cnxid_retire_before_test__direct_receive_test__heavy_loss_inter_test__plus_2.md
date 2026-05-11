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

## `picoquictest/tls_api_test.c:transmit_cnxid_retire_before_test`
* C test-table name: `cnxid_transmit_r_before`
* C entry function: `transmit_cnxid_retire_before_test`
* Rust test: `cnxid_transmit_r_before`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1681-1683`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper passes matching boolean arguments, but the Rust helper ignores them and only performs a basic connection, sync, wait, and close. It omits the C retire-before setup and all connection-ID retirement/count/stash assertions.
* Phase 5A fix note: Implement the C transmit_cnxid_test_one behavior in Rust for retire_before=true: configure CID TTL, drive the sync/wait sequence, verify retire-before progress, validate CID counts and stashed remote IDs, and check stale retired IDs are absent.
* Phase 5B analysis: Rust #[test] cnxid_transmit_r_before is present, compiles under the test harness, and calls transmit_cnxid_test_one(true, false, false), matching C transmit_cnxid_test_one(1, 0, 0). The shared helper expresses the same retire-before CID API contract; the known early disconnect/server-CID absence is a Phase 5C implementation issue.
* Phase 5B fix note: 

### C test body
```c
{
    return transmit_cnxid_test_one(1, 0, 0);
}
```

### Current Rust test body
```rust
fn cnxid_transmit_r_before() {
    transmit_cnxid_test_one(true, false, false).expect("cnxid_transmit_r_before");
}
```

## `picoquictest/tls_api_test.c:direct_receive_test`
* C test-table name: `direct_receive`
* C entry function: `direct_receive_test`
* Rust test: `direct_receive`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1995-2036`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic TLS handshake/close; it does not mark stream 4 for direct receive, use loss_mask 8, drive the very-long stream scenario, or validate direct callback data/offsets.
* Phase 5A fix note: Implement a direct-receive Rust test/helper that connects, initializes the very-long stream scenario, installs a StreamDirectReceive callback on client stream 4, runs with loss_mask 8, and verifies data completion within 3500000 us.
* Phase 5B analysis: Current Rust test is present, compiles as a harness test, and matches the C API-level contract: scenario init/connect, very-long stream 4, loss_mask=8, direct-receive registration, validation of response bytes by offset, coverage, FIN offset, completion time, close, and pool recycling. Any failure from missing direct-receive callback delivery or unwired server response behavior is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t loss_mask = 8;
    uint64_t max_completion_microsec = 3500000;

    ret = tls_api_one_scenario_init(&test_ctx, &simulated_time,
        0, NULL, NULL);

    if (ret == 0) {
        ret = tls_api_one_scenario_body_connect(test_ctx, &simulated_time, 0, 0);

        /* Prepare to send data */
        if (ret == 0) {
            test_ctx->stream0_target = 0;
            ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));

            if (ret != 0)
            {
                DBG_PRINTF("Init send receive scenario returns %d\n", ret);
            }
        }

        /* Set the direct receive API for the stream number 4. */
        if (ret == 0) {
            ret = picoquic_mark_direct_receive_stream(test_ctx->cnx_client, 4, test_api_direct_receive_callback, 
                (void*)&test_ctx->client_callback);

            if (ret != 0)
            {
                DBG_PRINTF("Mark direct receive stream returns %d\n", ret);
            }
        }

        /* Perform a data sending loop */
        if (ret == 0) {
            ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);

            if (ret != 0)
            {
                DBG_PRINTF("Data sending loop returns %d\n", ret);
            }
        }

        if (ret == 0) {
            ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, max_completion_microsec);
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
fn direct_receive() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 8u64;
    let max_completion_microsec = 3_500_000;
    let direct_state = std::rc::Rc::new(std::cell::RefCell::new(DirectReceiveState::new(
        4,
        TEST_SCENARIO_VERY_LONG[0].r_len,
    )));
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_one_scenario_body_connect(&mut test_ctx, &mut simulated_time, 0, 0).expect("connect");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    test_ctx
        .cnx_client()
        .mark_direct_receive_stream(
            4,
            Box::new(DirectReceiveProbe {
                state: std::rc::Rc::clone(&direct_state),
            }),
        )
        .expect("mark direct receive stream");

    direct_receive_data_sending_loop(
        &mut test_ctx,
        &direct_state,
        &mut loss_mask,
        &mut simulated_time,
        max_completion_microsec,
    )
    .expect("direct receive data loop");

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("close");
    assert!(
        test_ctx.qclient.nb_data_nodes_allocated <= test_ctx.qclient.nb_data_nodes_in_pool(),
        "client data node pool did not fully recycle"
    );
    assert!(
        test_ctx.qserver.nb_data_nodes_allocated <= test_ctx.qserver.nb_data_nodes_in_pool(),
        "server data node pool did not fully recycle"
    );
}
```

## `picoquictest/tls_api_test.c:heavy_loss_inter_test`
* C test-table name: `heavy_loss_inter`
* C entry function: `heavy_loss_inter_test`
* Rust test: `heavy_loss_inter`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2824-2826`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the same visible parameters, but heavy_loss_test_one ignores mode and uses a single large default transfer with always-on burst loss; C mode 1 builds 100 chained 255-byte query/1KB response transactions, warms up, applies a deterministic loss mask for up to 20 seconds, clears loss, then verifies completion time.
* Phase 5A fix note: Make Rust heavy_loss_test_one honor mode 1 with the interactive 100-stream scenario, C initial CID/BBR/log setup where relevant, warmup, deterministic loss window, clear-loss completion, and real scenario/time verification.
* Phase 5B analysis: Rust #[test] is present and directly matches the C API-level contract: heavy_loss_test_one(1, 22000000). Any Generic/runtime stream-transfer failure is Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return heavy_loss_test_one(1, 22000000);
}
```

### Current Rust test body
```rust
fn heavy_loss_inter() {
    heavy_loss_test_one(1, 22_000_000).expect("heavy_loss_inter");
}
```

## `picoquictest/tls_api_test.c:keep_alive_test`
* C test-table name: `keep_alive`
* C entry function: `keep_alive_test`
* Rust test: `keep_alive`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3361-3364`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls both helper modes, but the helper does not assert the C outcomes: keep-alive enabled must remain ready past 2x silence max, and disabled must disconnect. It also uses 1ms instead of C's default keep-alive interval and returns Ok unconditionally after the loop.
* Phase 5A fix note: Repair keep_alive_test_impl to enable keep-alive with the default interval for the on case, assert ready/time threshold, assert disconnected for the off case, and close when still connected.
* Phase 5B analysis: Rust #[test] keep_alive is present, compiles under the Rust test harness, and calls keep_alive_test_impl(1) followed by keep_alive_test_impl(0), matching the C entry. The helper expresses the API-level enabled/disabled keep-alive assertions; any early disconnect/default keep-alive behavior failure is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = keep_alive_test_impl(1);

    if (ret == 0) {
        ret = keep_alive_test_impl(0);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn keep_alive() {
    keep_alive_test_impl(1).expect("keep_alive_on");
    keep_alive_test_impl(0).expect("keep_alive_off");
}
```

## `picoquictest/tls_api_test.c:tls_api_many_losses`
* C test-table name: `many_losses`
* C entry function: `tls_api_many_losses`
* Rust test: `many_losses`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4035-4064`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers only eight fixed masks and its tls_api_loss_test ignores the supplied mask; it omits the C preprogrammed mask matrix and the 50 deterministic 30% random-loss scenario runs.
* Phase 5A fix note: Recreate the C mask loops, deterministic random mask generation from 0x1055ca45c001baba, and ensure Rust loss/scenario helpers actually apply each loss mask including the q_and_r max_data=128000 random-loss scenario.
* Phase 5B analysis: Rust many_losses mirrors the C mask matrix, deterministic 0x1055ca45c001baba random-mask generation, q_and_r scenario, and max_data=128000. Any early failure from missing server connection/runtime behavior is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: No Rust test changes; updated COMMANDS.log only for command trace.

### C test body
```c
{
    uint64_t loss_mask = 0;
    int ret = 0;
    uint64_t random_context = 0x1055ca45c001babaull;

    /* We first test with a set of preprogrammed masks, checking consecutive drops */
    for (int i = 0; ret == 0 && i < 6; i++) {
        for (int j = 0; ret == 0 && j < 4; j++) {
            uint64_t j_mask = ~(UINT64_MAX << j);
            loss_mask = j_mask << i;
            ret = tls_api_loss_test(loss_mask);
            if (ret != 0) {
                DBG_PRINTF("Handshake fails for mask %d-%d = %llx", i, j, (unsigned long long)loss_mask);
            }
        }
        for (uint64_t j = 8; ret == 0 && j < 11; j++) {
            loss_mask = (j | (j << 4) | (j << 8))<<i;
            ret = tls_api_loss_test(loss_mask);
            if (ret != 0) {
                DBG_PRINTF("Handshake fails for mask %d, %" PRIu64" = %llx", i, j,  (unsigned long long)loss_mask);
            }
        }
    }

    /* Then we verify that we can establish 50 connections with packet drop rate=30% */
    for (int i = 0; ret == 0 &&  i < 50; i++)
    {
        uint64_t loss_mask = 0;
        for (int j = 0; j < 64; j++)
        {
            loss_mask <<= 1;

            if (picoquic_test_uniform_random(&random_context, 1000) < 300) {
                loss_mask |= 1;
            }
        }

        ret = tls_api_one_scenario_test(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, loss_mask, 128000, 0, 0, 0, NULL, NULL);
        if (ret != 0) {
            DBG_PRINTF("Handshake fails for random mask %d, mask = %llx", i, (unsigned long long)loss_mask);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn many_losses() {
    for i in 0..6u32 {
        for j in 0..4u32 {
            let j_mask = if j == 0 { 0 } else { (1u64 << j) - 1 };
            let loss_mask = j_mask << i;
            many_losses_loss_test(loss_mask)
                .unwrap_or_else(|e| panic!("many_losses mask {i}-{j}={loss_mask:#x}: {e:?}"));
        }

        for j in 8u64..11 {
            let loss_mask = (j | (j << 4) | (j << 8)) << i;
            many_losses_loss_test(loss_mask)
                .unwrap_or_else(|e| panic!("many_losses mask {i},{j}={loss_mask:#x}: {e:?}"));
        }
    }

    let mut random_context = 0x1055_ca45_c001_babau64;
    for i in 0..50 {
        let mut loss_mask = 0u64;
        for _ in 0..64 {
            loss_mask <<= 1;
            if test_uniform_random(&mut random_context, 1000) < 300 {
                loss_mask |= 1;
            }
        }

        many_losses_q_and_r_scenario(loss_mask)
            .unwrap_or_else(|e| panic!("many_losses random mask {i}={loss_mask:#x}: {e:?}"));
    }
}
```
