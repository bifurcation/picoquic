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

## `picoquictest/congestion_test.c:bdp_bbr1_test`
* C test-table name: `bdp_bbr1`
* C entry function: `bdp_bbr1_test`
* Rust test: `bdp_bbr1`
* C source: `picoquictest/congestion_test.c:730-733`
* Rust source: `rs/fq/src/tests/congestion.rs:942-944`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_bbr1);
}
```

### Rust test body
```rust
fn bdp_bbr1() {
    bdp_option_test_one(BdpTestOption::Bbr1);
}
```

## `picoquictest/cpu_limited.c:limited_safe_test`
* C test-table name: `limited_safe`
* C entry function: `limited_safe_test`
* Rust test: `limited_safe`
* C source: `picoquictest/cpu_limited.c:251-262`
* Rust source: `rs/fq/src/tests/cpu_limited.rs:197-205`

### C test body
```c
{
    limited_test_config_t config;
    limited_config_set_default(&config, 5);
    config.ccalgo = picoquic_cubic_algorithm;
    config.max_completion_time = 5450000;
    /* Bug. Should investigate later -- there should be 0 or maybe 1 losses */
    config.nb_losses_max = 6;
    config.flow_control_max = 57344;

    return limited_client_test_one(&config);
}
```

### Rust test body
```rust
fn limited_safe() {
    let mut config = limited_config_default(5);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic algo");
    config.max_completion_time = 5_450_000;
    // Bug noted in original C: there should be 0 or maybe 1 losses.
    config.nb_losses_max = 6;
    config.flow_control_max = 57_344;
    limited_client_test_one(config);
}
```

## `picoquictest/dualq_aqm_test.c:dualq_aqm_test`
* C test-table name: `dualq_aqm`
* C entry function: `dualq_aqm_test`
* Rust test: `dualq_aqm`
* C source: `picoquictest/dualq_aqm_test.c:415-436`
* Rust source: `rs/fq/src/tests/dualq_aqm.rs:312-318`

### C test body
```c
{
    int ret = dualq_test_ctx_test();

    if (ret == 0) {
        ret = dualq_enqueue_test();
    }

    if (ret == 0) {
        ret = dualq_dequeue_test();
    }

    if (ret == 0) {
        ret = dualq_submit_test();
    }

    if (ret == 0) {
        ret = dualq_sustain_test();
    }

    return ret;
}
```

### Rust test body
```rust
fn dualq_aqm() {
    dualq_ctx_test().expect("dualq_ctx_test");
    dualq_enqueue().expect("dualq_enqueue");
    dualq_dequeue().expect("dualq_dequeue");
    dualq_submit().expect("dualq_submit");
    dualq_sustain().expect("dualq_sustain");
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
