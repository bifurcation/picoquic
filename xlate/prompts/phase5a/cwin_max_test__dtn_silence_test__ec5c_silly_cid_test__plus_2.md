# Phase 5A C/Rust test correspondence audit

Review each C/Rust test pair and decide whether the Rust
`#[test]` checks the same behavior as the C test.  This is
a read-only pass: do not edit files.

You may inspect directly relevant context when needed:
Rust test helpers, fixtures, translated implementation under
`rs/fq/`, C helper functions in `picoquictest/`, constants,
and nearby tests.  Do not require byte-for-byte structure;
idiomatic Rust is fine when it preserves the C test intent,
inputs, expected results, and important edge cases.

Classify each entry as:

* `ok` when the Rust test is an acceptable translation.
* `needs_fix` when the Rust test is missing checks, checks
  materially different behavior, weakens assertions, skips
  cases the C test covers, or has placeholder-like logic.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"reviews":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short conclusion","fix_summary":"what 5B should change, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/congestion_test.c:cwin_max_test`
* C test-table name: `cwin_max`
* C entry function: `cwin_max_test`
* Rust test: `cwin_max`
* C source: `picoquictest/congestion_test.c:1016-1045`
* Rust source: `rs/fq/src/tests/congestion.rs:967-978`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgos[] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_bbr_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr1_algorithm
    };
    uint64_t max_completion_times[] = {
        11000000,
        11000000,
        11000000,
        11000000,
        12100000,
        11000000
    };
    int ret = 0;

    for (size_t i = 0; i < sizeof(ccalgos) / sizeof(picoquic_congestion_algorithm_t*); i++) {
        ret = cwin_max_test_one(ccalgos[i], 68000, max_completion_times[i]);
        if (ret != 0) {
            DBG_PRINTF("CWIN Max test fails for <%s>", ccalgos[i]->congestion_algorithm_id);
            break;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn cwin_max() {
    let algo_names = ["newreno", "cubic", "dcubic", "bbr", "fastcc", "bbr1"];
    let max_completion_times: [u64; 6] = [
        11_000_000, 11_000_000, 11_000_000, 11_000_000, 12_100_000, 11_000_000,
    ];

    for (name, &max_time) in algo_names.iter().zip(max_completion_times.iter()) {
        let ccalgo =
            get_congestion_algorithm(name).unwrap_or_else(|| panic!("cc algo not found: {name}"));
        cwin_max_test_one(ccalgo, 68_000, max_time);
    }
}
```

## `picoquictest/delay_tolerant_test.c:dtn_silence_test`
* C test-table name: `dtn_silence`
* C entry function: `dtn_silence_test`
* Rust test: `dtn_silence`
* C source: `picoquictest/delay_tolerant_test.c:216-226`
* Rust source: `rs/fq/src/tests/delay_tolerant.rs:202-208`

### C test body
```c
{
    /* Simple test. */
    dtn_test_spec_t spec;
    dtn_set_basic_test_spec(&spec);
    spec.scenario = dtn_scenario_silence;
    spec.sizeof_scenario = sizeof(dtn_scenario_silence);
    spec.max_number_of_packets = 120; /* Check that the number of packets does not increase wildly */
    spec.max_completion_time = 481000000; /* 8 minutes: 2 for handshake, plus 2 per transaction */
    return dtn_test_one(0x51, &spec);
}
```

### Rust test body
```rust
fn dtn_silence() {
    let mut spec = dtn_basic_spec();
    spec.scenario = DTN_SCENARIO_SILENCE;
    spec.max_number_of_packets = 120;
    spec.max_completion_time = 481_000_000; // 8 min: 2 handshake + 2 per tx
    dtn_test_one(0x51, &spec);
}
```

## `picoquictest/edge_cases.c:ec5c_silly_cid_test`
* C test-table name: `ec5c_silly_cid`
* C entry function: `ec5c_silly_cid_test`
* Rust test: `ec5c_silly_cid`
* C source: `picoquictest/edge_cases.c:496-527`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1178-1186`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t initial_losses = 0x01e084;
    uint8_t test_case_id = 0x5c;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 0, &simulated_time, initial_losses, 48);

    if (ret == 0) {
        if (test_ctx->cnx_server == NULL) {
            DBG_PRINTF("Unexpected state, client: %d, server: NULL",
                test_ctx->cnx_client->cnx_state);

        } else if (test_ctx->cnx_client->cnx_state != picoquic_state_ready ||
            test_ctx->cnx_server->cnx_state != picoquic_state_ready) {
            DBG_PRINTF("Unexpected state, client: %d, server: %d",
                test_ctx->cnx_client->cnx_state, test_ctx->cnx_server->cnx_state);
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = edge_case_complete(test_ctx, &simulated_time, 3000000);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn ec5c_silly_cid() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = edge_case_prepare(0x5c, false, &mut simulated_time, 0x01e084, 48)
        .expect("edge_case_prepare");
    assert!(test_ctx.has_cnx_server(), "server connection must exist");
    assert!(test_ctx.client_ready(), "client must be ready");
    assert!(test_ctx.server_ready(), "server must be ready");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 3_000_000).expect("edge_case_complete");
}
```

## `picoquictest/edge_cases.c:initial_pto_test`
* C test-table name: `initial_pto`
* C entry function: `initial_pto_test`
* Rust test: `initial_pto`
* C source: `picoquictest/edge_cases.c:1341-1394`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1238-1274`

### C test body
```c
{
    int ret = 0;
    picoquic_test_tls_api_ctx_t *test_ctx = NULL;
    size_t length = 0;
    uint64_t simulated_time = 0;
    uint64_t simulated_rtt = 20000;
    uint64_t simulated_pto = 4*simulated_rtt;
    picoquic_connection_id_t initial_cid = { { 0x94, 0x01, 0x41, 0, 0, 0, 0, 0}, 8 };

    /* Create a client. */
    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
            PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);
    if (ret != 0) {
        DBG_PRINTF("Cannot initialize context, ret = 0x%x", ret);
    }
    else {
        /* Set the binlog */
        picoquic_set_qlog(test_ctx->qclient, ".");
        /* start the client connection */
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }
    /* Send the initial packet */
    if (ret == 0) {
        ret = initial_pto_prepare(test_ctx, &simulated_time, &length);
        if (ret == 0 && length < 1200) {
            length = -1;
        }
    }
    /* get the initial message, wait until next client time >= time of ACK */
    if (ret == 0 && simulated_time < 20000) {
        ret = initial_pto_wait(test_ctx, &simulated_time, simulated_rtt, &length);
    }
    /* format an ACK packet, apply initial protection, submit ACK to client */
    if (ret == 0) {
        ret = initial_pto_ack(test_ctx, &simulated_time);
    }
    /* Wait until next client time >= expected response, or
     * client is ready to send and does send. */
    if (ret == 0 && simulated_time < simulated_pto) {
        ret = initial_pto_wait(test_ctx, &simulated_time, simulated_pto, &length);
        if (ret == 0 && length < 1200) {
            /* Did not send the PTO */
            ret = -1;
        }
    }
    /* Clean up */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn initial_pto() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x94, 0x01, 0x41, 0, 0, 0, 0, 0]).expect("8-byte CID");
    let simulated_rtt = 20_000u64;
    let simulated_pto = 4 * simulated_rtt;

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.cnx_client().start_client().expect("start_client");

    // Send the initial packet to the server.
    let length =
        initial_pto_prepare(&mut test_ctx, &mut simulated_time).expect("initial_pto_prepare");
    assert!(length >= 1200, "initial packet too short: {length}");

    // Wait until the ACK time, then send a synthetic ACK to the client.
    if simulated_time.ticks() < simulated_rtt {
        let _length = initial_pto_wait(&mut test_ctx, &mut simulated_time, simulated_rtt)
            .expect("initial_pto_wait");
    }
    initial_pto_ack(&mut test_ctx, &mut simulated_time).expect("initial_pto_ack");

    // The client should fire a PTO and send at least 1200 bytes.
    if simulated_time.ticks() < simulated_pto {
        let length = initial_pto_wait(&mut test_ctx, &mut simulated_time, simulated_pto)
            .expect("initial_pto_wait (PTO)");
        assert!(length >= 1200, "PTO packet not sent (length = {length})");
    }
}
```

## `picoquictest/edge_cases.c:reset_need_max_test`
* C test-table name: `reset_need_max`
* C entry function: `reset_need_max_test`
* Rust test: `reset_need_max`
* C source: `picoquictest/edge_cases.c:1218-1221`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1367-1369`

### C test body
```c
{
    return reset_repeat_test_one(reset_need_max_stream);
}
```

### Rust test body
```rust
fn reset_need_max() {
    reset_repeat_test_one(ResetTestKind::NeedMaxStream).expect("reset_need_max");
}
```
