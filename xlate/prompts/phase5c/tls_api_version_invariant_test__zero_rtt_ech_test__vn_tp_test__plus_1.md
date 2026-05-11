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

## `picoquictest/tls_api_test.c:tls_api_version_invariant_test`
* C test-table name: `version_invariant`
* C entry function: `tls_api_version_invariant_test`
* Rust test: `version_invariant`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8567-8594`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C fabricates an invalid-version long-header packet with oversized CID lengths, submits it to the server, prepares a VN response, and checks invariant fields. Rust only runs a normal handshake.
* Phase 5A fix note: Implement the fabricated packet submission and response validation in Rust, including nonzero VN response, long header, zero version, and swapped CID checks.
* Phase 5B analysis: Rust test is present as a runnable #[test], builds under the Rust test harness, and expresses the C API contract: initialize TLS API context, submit the fabricated invalid-version long-header packet, prepare the server response, and assert nonzero VN response with long header, zero version, and swapped CID fields. Any failure to queue the VN response is Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret != 0)
    {
        DBG_PRINTF("%s", "Could not create the QUIC test contexts");
    }
    else {
        /* Fabricate a packet that stresses the invariants */
        uint8_t packet[PICOQUIC_MAX_PACKET_SIZE];
        /* Initialize first byte to long header value*/
        packet[0] = 0xF5;
        /* Set version to unexpected value */
        memset(packet + 1, 0xaa, 4);
        /* Set destination CID to 255 times "dd" */
        packet[6] = 255;
        memset(packet + 7, 0xdd, 255);
        /* Set source CID to 127 times "cc" */
        packet[262] = 127;
        memset(packet + 263, 0xcc, 127);
        /* Set reminder of packet to 0x55*/
        memset(packet + 290, 0x55, PICOQUIC_MAX_PACKET_SIZE - 290);

        /* Submit the packet to the server */
        ret = picoquic_incoming_packet(test_ctx->qserver, packet, PICOQUIC_MAX_PACKET_SIZE,
            (struct sockaddr*) & test_ctx->client_addr, (struct sockaddr*) & test_ctx->server_addr,
            0, 0, simulated_time);
        if (ret != 0) {
            DBG_PRINTF("incoming invariant test returns %d (0x%x)", ret, ret);
        }
        else {
            uint8_t response[PICOQUIC_MAX_PACKET_SIZE];
            struct sockaddr_storage addr_to;
            struct sockaddr_storage addr_from;
            size_t send_length = 0;
            int if_index = 0;
            picoquic_connection_id_t log_cid;
            picoquic_cnx_t* last_cnx = NULL;

            ret = picoquic_prepare_next_packet(test_ctx->qserver, simulated_time, response, PICOQUIC_MAX_PACKET_SIZE,
                &send_length, &addr_to, &addr_from, &if_index, &log_cid, &last_cnx);
            if (ret != 0) {
                DBG_PRINTF("Invariant response test returns %d (0x%x)", ret, ret);
            }
            else if (send_length == 0) {
                DBG_PRINTF("%s", "Invariant response test does not return any data");
                ret = -1;
            }
            else {
                ret = check_vn_invariant(packet, PICOQUIC_MAX_PACKET_SIZE, response, send_length);
            }
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
fn version_invariant() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");
    let client_addr = test_ctx.client_addr;
    let server_addr = test_ctx.server_addr;
    let mut packet = version_invariant_packet();

    test_ctx
        .qserver
        .incoming_packet(
            &mut packet,
            &client_addr,
            &server_addr,
            0,
            0,
            simulated_time,
        )
        .expect("incoming invariant packet");

    let mut response = [0u8; crate::MAX_PACKET_SIZE];
    let prepared = test_ctx
        .qserver
        .prepare_next_packet(simulated_time, &mut response)
        .expect("prepare invariant response");
    let send_length = prepared.send_length;
    assert!(send_length > 0, "server did not return a VN response");
    assert_vn_invariant(&packet, &response[..send_length]);
}
```

## `picoquictest/tls_api_test.c:zero_rtt_ech_test`
* C test-table name: `zero_rtt_ech`
* C entry function: `zero_rtt_ech_test`
* Rust test: `zero_rtt_ech`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:9030-9036`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C sets propose_ech and configures client ECH before verifying 0-RTT/PSK behavior; Rust sets the flag but the helper only toggles client_zero_share and synthesizes 0-RTT/PSK state, so ECH proposal behavior is not actually checked.
* Phase 5A fix note: Implement/use the Rust equivalent of picoquic_ech_configure_quic_ctx for propose_ech and verify real 0-RTT plus ECH behavior without forcing counters/state.
* Phase 5B analysis: Rust test matches the C API-level contract: it sets only propose_ech and calls zero_rtt_test_one; the shared helper uses qclient.ech_configure(None, None) and retains the zero-RTT/ticket/PSK assertions. Remaining resumption, early-data, PSK, or ECH GREASE runtime failures are Phase 5C implementation issues.
* Phase 5B fix note: 

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.propose_ech = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
fn zero_rtt_ech() {
    zero_rtt_test_one(&ZeroRttTest {
        propose_ech: true,
        ..Default::default()
    })
    .expect("zero_rtt_ech");
}
```

## `picoquictest/transport_param_test.c:vn_tp_test`
* C test-table name: `vn_tp`
* C entry function: `vn_tp_test`
* Rust test: `vn_tp`
* Expected Rust file: `rs/fq/src/tests/transport_param.rs`
* Current Rust span: `rs/fq/src/tests/transport_param.rs:1057-1061`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust does not run the same VN TP table. C has 9 client and 7 server cases; Rust reshapes this into 8+8, drops client_bad_4 and server_bad_4, adds duplicate server OK cases, shifts the server VN-error case, and does not check negotiated_index consistency.
* Phase 5A fix note: Translate the exact C vn_tp_test_case table, including client_bad_4 and server_bad_4, preserve mode/envelope/expected version/error per case, and assert negotiated_index maps to the negotiated version on success.
* Phase 5B analysis: Rust vn_tp already mirrors the C 16-case vn_tp_test_case table and checks the same API-visible contract: parse success/error, expected negotiated version, and negotiated_index mapping. Remaining VN behavior failures are Phase 5C implementation issues. cargo check --tests currently fails in unrelated shared util.rs textlog_transport_extension_content surface, not in this vn_tp test contract.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    for (size_t i = 0; i < nb_vn_tp_test_case; i++) {
        if (vn_tp_test_one(vn_tp_test_case[i].vn_tp_len, vn_tp_test_case[i].vn_tp, vn_tp_test_case[i].mode,
            vn_tp_test_case[i].vn_envelop, vn_tp_test_case[i].vn_expected, vn_tp_test_case[i].error_expected) != 0) {
            DBG_PRINTF("Vn test case[%zu] fails", i);
            ret = -1;
            break;
        }
    }
    return ret;
}
```

### Current Rust test body
```rust
fn vn_tp() {
    for (i, case) in VN_TP_TEST_CASES.iter().enumerate() {
        vn_tp_test_case(i, case);
    }
}
```

## `picoquictest/wifitest.c:wifi_bbr1_hard_test`
* C test-table name: `wifi_bbr1_hard`
* C entry function: `wifi_bbr1_hard_test`
* Rust test: `wifi_bbr1_hard`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:120-131`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust spec fields match C for bbr1 hard, including the six hard suspensions. However, Rust wifi_test_one relies on tls_api_one_scenario_body_verify, which ignores target_time and does not perform the C scenario-completion verification.
* Phase 5A fix note: Keep the test spec, but repair the shared Rust scenario verification used by wifi_test_one so it checks full stream completion and enforces the 4060000us target.
* Phase 5B analysis: Rust test is present, compiles as a test, and matches the C API-level contract: BBR1, hard suspensions, latency 3000, target_time 4060000, no receive block, queue_max_delay 0, and wifi_test_bbr1_hard ID. Any failure to reach Ready is a Phase 5C runtime implementation issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_hard,
        3000,
        suspension_hard,
        picoquic_bbr1_algorithm,
        NULL,
        4060000,
        0,
        0 };
    int ret = wifi_test_one(wifi_test_bbr1_hard, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_bbr1_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "bbr1",
        cc_algo_option: None,
        target_time: 4_060_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR1_HARD, &spec).expect("wifi_bbr1_hard");
}
```
