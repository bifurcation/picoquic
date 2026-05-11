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

## `picoquictest/tls_api_test.c:tls_api_two_connections_test`
* C test-table name: `two_connections`
* C entry function: `tls_api_two_connections_test`
* Rust test: `two_connections`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8373-8455`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only performs one generic connection; C establishes one connection, waits, deletes the client connection without notifying the server, creates a second client connection in the same context, then handshakes and closes it.
* Phase 5A fix note: Translate the two-connection sequence explicitly: first handshake and idle wait, remove client-side connections, clear server reference as appropriate, create/start a new client connection in the same context, then run the second handshake and close.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, compiles, and matches the C API-level sequence for two successive client connections. Any early readiness/close failure is incomplete Rust library behavior for Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        /* Verify that the connection is fully established */
        uint64_t target_time = simulated_time + 2000000;

        while (ret == 0 && TEST_CLIENT_READY && TEST_SERVER_READY && simulated_time < target_time) {
            int was_active = 0;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, target_time, &was_active);
        }

        /* Delete the client connection from the client context,
         * without sending notification to the server */
        while (test_ctx->qclient->cnx_list != NULL) {
            picoquic_delete_cnx(test_ctx->qclient->cnx_list);
        }

        /* Erase the server connection reference */
        test_ctx->cnx_server = NULL;

        /* Create a new connection in the client context */

        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient,
            picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*)&test_ctx->server_addr, simulated_time, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
        else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }

    /* Now, restart a connection in the same context */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
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
fn two_connections() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("first connection loop");
    assert!(
        test_ctx.client_ready(),
        "first client connection is not ready"
    );
    assert!(
        test_ctx.server_ready(),
        "first server connection is not ready"
    );

    let target_time = Instant::from_ticks(simulated_time.ticks() + 2_000_000);
    while test_ctx.client_ready() && test_ctx.server_ready() && simulated_time < target_time {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            target_time,
            &mut was_active,
        )
        .expect("idle round before second connection");
    }

    loop {
        let token = test_ctx
            .qclient
            .first_cnx_mut()
            .and_then(|cnx| cnx.own_token);
        let Some(token) = token else { break };
        test_ctx.qclient.delete_connection(token);
    }
    assert!(
        test_ctx.qclient.connections.is_empty(),
        "client connections were not deleted before restart"
    );

    test_ctx.clear_cnx_server_ref();
    assert!(
        !test_ctx.has_cnx_server(),
        "server reference was not cleared before restart"
    );

    let server_addr = test_ctx.server_addr;
    let cnx = test_ctx
        .qclient
        .create_connection(
            ConnectionId::with_size(0).expect("null initial cid"),
            ConnectionId::with_size(0).expect("null remote cid"),
            Some(&server_addr),
            simulated_time,
            0,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .expect("create second client connection");
    cnx.start_client().expect("start second client connection");

    loss_mask = 0;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("second connection loop");
    assert!(
        test_ctx.client_ready(),
        "second client connection is not ready"
    );
    assert!(
        test_ctx.server_ready(),
        "second server connection is not ready"
    );
    assert_eq!(
        test_ctx.qserver.connections.len(),
        2,
        "server should retain the first connection and accept a second"
    );

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
        .expect("close second connection");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_bad_param_test`
* C test-table name: `zero_rtt_bad_param`
* C entry function: `zero_rtt_bad_param_test`
* Rust test: `zero_rtt_bad_param`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8989-8995`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper passes change_params, but zero_rtt_test_one synthesizes 0-RTT counters instead of observing real behavior and omits the C server-data-received check.
* Phase 5A fix note: Make zero_rtt_test_one drive real 0-RTT state for changed transport parameters; assert 0-RTT was sent, not acked/accepted, data was eventually received, no short Initial, and tickets were saved.
* Phase 5B analysis: Rust test matches the C entry by setting change_params and calling zero_rtt_test_one; the helper already expresses the API-visible changed-transport-parameter 0-RTT rejection checks. Any failure from missing ticket/0-RTT library behavior is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.change_params = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
fn zero_rtt_bad_param() {
    zero_rtt_test_one(&ZeroRttTest {
        change_params: true,
        ..Default::default()
    })
    .expect("zero_rtt_bad_param");
}
```

## `picoquictest/transport_param_test.c:transport_param_test`
* C test-table name: `transport_param`
* C entry function: `transport_param_test`
* Rust test: `transport_param`
* Expected Rust file: `rs/fq/src/tests/transport_param.rs`
* Current Rust span: `rs/fq/src/tests/transport_param.rs:21-156`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C exercises many fixed encoded vectors, decode-only vectors, grease/version variants, 13 malformed error cases, and client/server fuzz truncation/mutation. Rust only round-trips four ad hoc TransportParameters and does not compare exact encodings, decode the C byte fixtures, or test error/fuzz cases.
* Phase 5A fix note: Port the C transport_param_test vectors and helpers: exact encode comparisons, decode-only fixtures, grease/version cases, malformed error cases, and client/server fuzz mutation/truncation coverage.
* Phase 5B analysis: Rust test mirrors the C API-level encode/decode/error/fuzz contract; TP1/CP1 encoded-byte mismatch is a Phase 5C implementation failure, not a Phase 5B block. cargo check currently fails earlier on unrelated missing textlog helper in util.rs.
* Phase 5B fix note: No Rust test changes; COMMANDS.log updated for command logging.

### C test body
```c
{
    int ret = 0;
    uint64_t proof = 0;
    uint32_t version_default = picoquic_supported_versions[0].version;

    ret = transport_param_one_test(0, 0, version_default, version_default,
        &transport_param_test1, client_param1, sizeof(client_param1));
    if (ret != 0) {
        DBG_PRINTF("Param test TP1, CP1 returns %x\n", ret);
    } else {
        ret = transport_param_one_test(0, 0, version_default, 0x0A1A0A1A,
            &transport_param_test2, client_param2, sizeof(client_param2));
        if (ret != 0) {
            DBG_PRINTF("Param test TP2, CP2 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test3, client_param3, sizeof(client_param3));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP3, CP3 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(1, 0, version_default, version_default,
            &transport_param_test4, server_param1, sizeof(server_param1));
        if (ret != 0) {
            DBG_PRINTF("Param test TP4, SP1 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(1, 0, version_default, 0x0A1A0A1A,
            &transport_param_test5, server_param2, sizeof(server_param2));
        if (ret != 0) {
            DBG_PRINTF("Param test TP5, SP2 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test6, client_param4, sizeof(client_param4));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP6, CP4 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0xBABABABA,
            &transport_param_test7, client_param5, sizeof(client_param5));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP7, CP5 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test8, client_param8, sizeof(client_param8));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP8, CP8 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(1, version_default, 0x0A1A0A1A,
            &transport_param_test9, server_param3, sizeof(server_param3));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP9, SP3 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(0, 0, version_default, version_default,
            &transport_param_test10, client_param9, sizeof(client_param9));
        if (ret != 0) {
            DBG_PRINTF("Param test TP10, CP9 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test8, client_param10, sizeof(client_param10));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP8, CP10 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(0, 1, version_default, version_default,
            &transport_param_test1, client_param11, sizeof(client_param11));
        if (ret != 0) {
            DBG_PRINTF("Param test TP1, CP1 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(0, 0, version_default, version_default,
            &transport_param_test11, client_param12, sizeof(client_param12));
        if (ret != 0) {
            DBG_PRINTF("Param test TP11, CP12 returns %x\n", ret);
        }
    }

    for (size_t i = 0; ret == 0 && i < nb_transport_param_error_case; i++) {
        ret = transport_param_error_test(transport_param_error_case[i].mode, transport_param_error_case[i].target, 
            transport_param_error_case[i].target_length, transport_param_error_case[i].local_error);
        if (ret != 0) {
            DBG_PRINTF("Param error test %d fails\n", (int)i);
        }
    }

    if (ret == 0)
    {
        DBG_PRINTF("%s", "Starting transport parameters fuzz test.\n");
        
        ret = transport_param_fuzz_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test2, client_param2, sizeof(client_param2), &proof);

        if (ret == 0) {
            ret = transport_param_fuzz_test(1, version_default, 0x0A1A0A1A,
                &transport_param_test2, server_param2, sizeof(server_param2), &proof);
        }

        DBG_PRINTF("%s", "End of transport parameters fuzz test.\n");
    }
    return ret;
}
```

### Current Rust test body
```rust
fn transport_param() {
    let version_default = Version::V1 as u32;

    transport_param_one(
        "TP1/CP1",
        0,
        false,
        version_default,
        version_default,
        tp1(),
        CLIENT_PARAM1,
    );
    transport_param_one(
        "TP2/CP2",
        0,
        false,
        version_default,
        0x0a1a_0a1a,
        tp2(),
        CLIENT_PARAM2,
    );
    transport_param_decode(
        "TP3/CP3",
        0,
        version_default,
        0x0a1a_0a1a,
        tp3(),
        CLIENT_PARAM3,
    );
    transport_param_one(
        "TP4/SP1",
        1,
        false,
        version_default,
        version_default,
        tp4(),
        SERVER_PARAM1,
    );
    transport_param_one(
        "TP5/SP2",
        1,
        false,
        version_default,
        0x0a1a_0a1a,
        tp5(),
        SERVER_PARAM2,
    );
    transport_param_decode(
        "TP6/CP4",
        0,
        version_default,
        0x0a1a_0a1a,
        tp6(),
        CLIENT_PARAM4,
    );
    transport_param_decode(
        "TP7/CP5",
        0,
        version_default,
        0xbaba_baba,
        tp7(),
        CLIENT_PARAM5,
    );
    transport_param_decode(
        "TP8/CP8",
        0,
        version_default,
        0x0a1a_0a1a,
        tp8(),
        CLIENT_PARAM8,
    );
    transport_param_decode(
        "TP9/SP3",
        1,
        version_default,
        0x0a1a_0a1a,
        tp9(),
        SERVER_PARAM3,
    );
    transport_param_one(
        "TP10/CP9",
        0,
        false,
        version_default,
        version_default,
        tp10(),
        CLIENT_PARAM9,
    );
    transport_param_decode(
        "TP8/CP10",
        0,
        version_default,
        0x0a1a_0a1a,
        tp8(),
        CLIENT_PARAM10,
    );
    transport_param_one(
        "TP1/CP11-grease",
        0,
        true,
        version_default,
        version_default,
        tp1(),
        CLIENT_PARAM11,
    );
    transport_param_one(
        "TP11/CP12",
        0,
        false,
        version_default,
        version_default,
        tp11(),
        CLIENT_PARAM12,
    );

    for (i, target) in TRANSPORT_PARAM_ERROR_CASES.iter().enumerate() {
        transport_param_error(&format!("error[{i}]"), 0, target);
    }

    transport_param_fuzz(
        "client fuzz CP2",
        0,
        version_default,
        0x0a1a_0a1a,
        tp2(),
        CLIENT_PARAM2,
    );
    transport_param_fuzz(
        "server fuzz SP2",
        1,
        version_default,
        0x0a1a_0a1a,
        tp2(),
        SERVER_PARAM2,
    );
}
```

## `picoquictest/wifitest.c:wifi_bbr_test`
* C test-table name: `wifi_bbr`
* C entry function: `wifi_bbr_test`
* Rust test: `wifi_bbr`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:104-107`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses the matching BBR algorithm, suspension pattern, test id, and target time, but the shared Rust scenario verification path ignores the target-time/completion checks that the C wifi_test_one relies on.
* Phase 5A fix note: Restore scenario verification for wifi tests so completion and target_time are checked, then keep the BBR spec values and RTT-max suspension assertion.
* Phase 5B analysis: Current Rust helper already matches the C wifi_test_one verification path: it calls tls_api_one_scenario_body_verify with spec.target_time, which checks scenario completion and completion time, then checks server rtt_max against the suspension interval. The wifi_bbr test keeps the C BBR algorithm, test id, suspension pattern, and 2800000 target time.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_bbr_algorithm, 2800000);
    int ret = wifi_test_one(wifi_test_bbr, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_bbr() {
    let spec = default_spec("bbr", SUSPENSION_BASIC, 2_800_000);
    wifi_test_one(WIFI_TEST_BBR, &spec).expect("wifi_bbr");
}
```

## `picoquictest/wifitest.c:wifi_cubic_long_test`
* C test-table name: `wifi_cubic_long`
* C entry function: `wifi_cubic_long_test`
* Rust test: `wifi_cubic_long`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:234-245`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust spec matches C defaults plus the latency and receive-block overrides. The remaining mismatch is helper-level: the Rust path passes target_time but the shared verify helper ignores it and does not assert full scenario completion like C.
* Phase 5A fix note: Keep the spec values, but repair the shared Rust scenario verification used by wifi_test_one so it checks full stream completion and enforces the 3100000us target.
* Phase 5B analysis: Rust wifi_cubic_long is a compiled #[test] in the test harness and matches the C API-level contract: Cubic default spec, latency 50000, receive-block suspension, target_time 3100000, default queue_max_delay 260000, WIFI_TEST_CUBIC_LONG, shared stream-completion verification, and RTT max check. Any Generic runtime failure is Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_cubic_algorithm, 3100000);
    spec.latency = 50000;
    spec.simulate_receive_block = 1;
    int ret = wifi_test_one(wifi_test_cubic_long, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_cubic_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "cubic",
        cc_algo_option: None,
        target_time: 3_100_000,
        simulate_receive_block: true,
        queue_max_delay: 260_000,
    };
    wifi_test_one(WIFI_TEST_CUBIC_LONG, &spec).expect("wifi_cubic_long");
}
```
