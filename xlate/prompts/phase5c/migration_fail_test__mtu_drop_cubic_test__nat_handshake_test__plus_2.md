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

## `picoquictest/tls_api_test.c:migration_fail_test`
* C test-table name: `migration_fail`
* C entry function: `migration_fail_test`
* Rust test: `migration_fail`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4288-4290`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls the successful migration helper with an empty/default scenario. The C test starts the very-long transfer, waits for ready, probes a bogus client port, then verifies the transfer still completes within 1100000us.
* Phase 5A fix note: Add a Rust failed-migration path: initialize the C very-long scenario, wait ready, probe a bogus address/port, run the data loop, and verify completion with the 1100000us limit.
* Phase 5B analysis: Existing `migration_fail` is a runnable Rust `#[test]`; its helper mirrors the C init, connection loop, very-long scenario init, wait-ready, bogus-port `probe_new_path`, data loop, and 1100000us verify sequence. Any early failure from incomplete `probe_new_path` behavior is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    /* establish the connection*/
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));

        if (ret != 0)
        {
            DBG_PRINTF("Init send receive scenario returns %d\n", ret);
        }
    }

    /* Perform a loop until the connection is in ready state */
    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    /* Start migration to bogus address */
    if (ret == 0) {
        struct sockaddr_in bogus_addr = test_ctx->client_addr;
        bogus_addr.sin_port += 1;

        ret = picoquic_probe_new_path(test_ctx->cnx_client,
            (struct sockaddr*) & test_ctx->server_addr, (struct sockaddr*) & bogus_addr, simulated_time);
        if (ret != 0) {
            DBG_PRINTF("Probe new path returns %d\n", ret);
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
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, 1100000);
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
fn migration_fail() {
    migration_fail_scenario().expect("migration_fail");
}
```

## `picoquictest/tls_api_test.c:mtu_drop_cubic_test`
* C test-table name: `mtu_drop_cubic`
* C entry function: `mtu_drop_cubic_test`
* Rust test: `mtu_drop_cubic`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4378-4380`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper passes the cubic name and target time, but the Rust mtu_drop_cc_algotest helper ignores the algorithm, uses different link settings, skips the initial one-second PMTUD wait and MTU assertions, never lowers path_mtu, and only runs a normal transfer.
* Phase 5A fix note: Repair mtu_drop_cc_algotest to mirror the C helper: select the requested congestion algorithm, use the 100ms/1Mbps links and algorithm-coded initial CID, wait 1s, assert discovered MTUs, lower both path MTUs, then complete and verify against the target time.
* Phase 5B analysis: Rust test is present, compiled by the Rust test harness, and calls mtu_drop_cc_algotest("cubic", 10_000_000), matching the C entry's mtu_drop_cc_algotest(picoquic_cubic_algorithm, 10000000). The shared helper already expresses the C API-visible setup and assertions; any early handshake/connection failure is Phase 5C runtime implementation work.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_cubic_algorithm, 10000000);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_drop_cubic() {
    mtu_drop_cc_algotest("cubic", 10_000_000).expect("mtu_drop_cubic");
}
```

## `picoquictest/tls_api_test.c:nat_handshake_test`
* C test-table name: `nat_handshake`
* C entry function: `nat_handshake_test`
* Rust test: `nat_handshake`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4695-4700`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs a plain tls_api_test_with_loss handshake/close. The C test runs two NAT-handshake ranks, rebinding during handshake at two trigger points, then sends q2/r2 data, closes, and asserts NAT actually occurred.
* Phase 5A fix note: Implement nat_handshake_test_one in Rust and have nat_handshake loop ranks 0 and 1, applying client_use_nat/client_addr_natted at the same handshake triggers and checking natted.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, and now matches the C API-level contract: it runs ranks 0 and 1, triggers NAT on the same API-visible conditions, drives q2/r2 data, closes, and asserts that NAT occurred. Any rank-0 remote-CID or handshake/runtime failure is Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    for (int test_rank = 0; ret == 0 && test_rank < 2; test_rank++) {
        ret = nat_handshake_test_one(test_rank);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn nat_handshake() {
    for test_rank in 0..2 {
        nat_handshake_test_one(test_rank)
            .unwrap_or_else(|e| panic!("nat_handshake({test_rank}): {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:no_ack_frequency_test`
* C test-table name: `no_ack_frequency`
* C entry function: `no_ack_frequency_test`
* Rust test: `no_ack_frequency`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5206-5246`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs a generic handshake/close and does not cover the three transport-parameter combinations or the very-long scenario with loss mask 128.
* Phase 5A fix note: Loop i=1..=3, initialize client/server transport parameters, set client `min_ack_delay` to 0 or 1000 and server `enable_loss_bit` per C, then run the very-long scenario with init loss mask 128 and 2000000 target.
* Phase 5B analysis: Rust test is present, compiled by the Rust test harness, and mirrors the C API-level contract: three TP combinations, initialized client/server parameters, very-long scenario, init loss mask 128, and 2000000 target. Any current Generic/data-transfer runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    picoquic_tp_t client_parameters;
    picoquic_tp_t server_parameters;

    for (int i = 1; ret == 0 && i <= 3; i++) {
        memset(&client_parameters, 0, sizeof(picoquic_tp_t));
        memset(&server_parameters, 0, sizeof(picoquic_tp_t));
        picoquic_init_transport_parameters(&client_parameters);
        picoquic_init_transport_parameters(&server_parameters);

        client_parameters.min_ack_delay = (i & 1) ? 0 : 1000;
        server_parameters.enable_loss_bit = (1 - ((i > 1) & 1));

        ret = tls_api_one_scenario_test(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 128, 0, 0, 0, 2000000, &client_parameters, &server_parameters);
        if (ret != 0) {
            DBG_PRINTF("No min ack delay test fails for client: %d, server: %d, ret = %d", i & 1, i >> 1, ret);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn no_ack_frequency() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    for i in 1..=3 {
        let mut client_parameters = TransportParameters::default();
        let mut server_parameters = TransportParameters::default();
        crate::internal::init_transport_parameters(&mut client_parameters);
        crate::internal::init_transport_parameters(&mut server_parameters);

        client_parameters.min_ack_delay = Duration::from_ticks(if i & 1 == 1 { 0 } else { 1000 });
        server_parameters.enable_loss_bit = if i > 1 { 0 } else { 1 };

        let mut simulated_time = Instant::from_ticks(0);
        let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, 0, None, None)
            .unwrap_or_else(|| panic!("no_ack_frequency({i}): ctx"));
        test_ctx
            .cnx_client()
            .set_transport_parameters(&client_parameters);
        test_ctx
            .qserver
            .set_default_tp(&server_parameters)
            .unwrap_or_else(|e| panic!("no_ack_frequency({i}): server tp: {e:?}"));

        tls_api_one_scenario_body(
            &mut test_ctx,
            &mut simulated_time,
            &scenario,
            128,
            0,
            0,
            0,
            2_000_000,
        )
        .unwrap_or_else(|e| panic!("no_ack_frequency({i}): {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:padding_zero_min_test`
* C test-table name: `padding_zero_min`
* C entry function: `padding_zero_min_test`
* Rust test: `padding_zero_min`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5902-5904`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls padding_test_one(128, 0), but that helper ignores both padding parameters and only handshakes/closes; it does not set padding policy or verify packet lengths across the C test sizes.
* Phase 5A fix note: Implement padding_test_one to set server/client padding policy, queue the C ping/padding frame sizes, observe outgoing packet lengths, and assert the padding_multiple=128, padding_min_size=0 formula and success conditions.
* Phase 5B analysis: Rust #[test] padding_zero_min is present, compiles under the Rust test harness, and calls padding_test_one(128, 0), matching the C entry. Any early handshake disconnect or padding-policy runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return padding_test_one(128, 0);
}
```

### Current Rust test body
```rust
fn padding_zero_min() {
    padding_test_one(128, 0).expect("padding_zero_min");
}
```
