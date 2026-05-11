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

## `picoquictest/sockloop_test.c:sockloop_nat_test`
* C test-table name: `sockloop_nat`
* C entry function: `sockloop_nat_test`
* Rust test: `sockloop_nat`
* C source: `picoquictest/sockloop_test.c:695-708`
* Rust source: `rs/fq/src/tests/sockloop.rs:605-614`

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 6);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.extra_socket_required = 1;
    spec.prefer_extra_socket = 1;
    spec.force_migration = 1;

    return(sockloop_test_one(&spec));
}
```

### Rust test body
```rust
fn sockloop_nat() {
    let mut spec = SockloopTestSpec::new(6);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.extra_socket_required = true;
    spec.prefer_extra_socket = true;
    spec.force_migration = 1;
    sockloop_test_one(&spec);
}
```

## `picoquictest/stresstest.c:random_gauss_test`
* C test-table name: `random_gauss`
* C entry function: `random_gauss_test`
* Rust test: `random_gauss`
* C source: `picoquictest/stresstest.c:1363-1390`
* Rust source: `rs/fq/src/tests/stresstest.rs:224-247`

### C test body
```c
{
    uint64_t t_seed = 0xDEADBEEFBABAC001ull;
    int ret = 0;
    double x2 = 0;
    double x_sum = 0;
    double a;
    double v;

    for (int i = 0; i < RANDOM_GAUSS_NB_TESTS; i++) {
        double x = picoquic_test_gauss_random(&t_seed);
        x_sum += x;
        x2 += x * x;
    }

    a = x_sum / RANDOM_GAUSS_NB_TESTS;
    v = x2 / RANDOM_GAUSS_NB_TESTS;

    if (a < -0.02 || a > 0.02) {
        ret = -1;
    }
    else if (v < 0.97 || v > 1.03) {
        ret = -1;
    }

    return ret;
}
```

### Rust test body
```rust
fn random_gauss() {
    const NB_TESTS: usize = 255;
    let mut t_seed: u64 = 0xDEADBEEFBABAC001u64;
    let mut x_sum: f64 = 0.0;
    let mut x2: f64 = 0.0;

    for _ in 0..NB_TESTS {
        let x = test_gauss_random(&mut t_seed);
        x_sum += x;
        x2 += x * x;
    }

    let mean = x_sum / NB_TESTS as f64;
    let var = x2 / NB_TESTS as f64;

    assert!(
        (-0.02..=0.02).contains(&mean),
        "Gaussian mean {mean} out of range [-0.02, 0.02]"
    );
    assert!(
        (0.97..=1.03).contains(&var),
        "Gaussian variance {var} out of range [0.97, 1.03]"
    );
}
```

## `picoquictest/tls_api_test.c:bad_cnxid_test`
* C test-table name: `bad_cnxid`
* C entry function: `bad_cnxid_test`
* Rust test: `bad_cnxid`
* C source: `picoquictest/tls_api_test.c:8816-8889`
* Rust source: `rs/fq/src/tests/tls_api.rs:78-82`

### C test body
```c
{
    uint64_t simulated_time = 0;
    header_fuzzer_ctx_t fuzz_ctx;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    memset(&fuzz_ctx, 0, sizeof(fuzz_ctx));
    fuzz_ctx.random_context = 0x123456789ABCDEF0ull;

    if (ret == 0) {
        /* Prepare to send data */
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    if (ret == 0) {
        /* establish the connection */
        ret = tls_api_one_scenario_body_connect(test_ctx, &simulated_time, 0, 0);
    }

    /* Set fuzzer, then perform a data sending loop */
    if (ret == 0) {
        picoquic_set_fuzz(test_ctx->qclient, header_fuzzer, &fuzz_ctx);

        (void) tls_api_data_sending_loop(test_ctx, NULL, &simulated_time, 0);


        /* verify that the server connection has disappeared */
        if (fuzz_ctx.nb_fuzzed > 0 && (test_ctx->cnx_server == NULL || test_ctx->cnx_server->cnx_state == picoquic_state_disconnected)) {
            ret = 0;
        }
        else {
            DBG_PRINTF("Unexpected server state: %d, packet: %d, fuzzed: %d\n", test_ctx->cnx_server->cnx_state, 
                fuzz_ctx.nb_packets, fuzz_ctx.nb_fuzzed);
            ret = -1;
        }
    }

    if (ret == 0) {
        /* Remove the reference to the old server connection */
        test_ctx->cnx_server = NULL;
        /* Delete the old client connection */
        picoquic_delete_cnx(test_ctx->cnx_client);
        test_ctx->cnx_client = NULL;
        /* Remove the fuzzer */
        picoquic_set_fuzz(test_ctx->qclient, NULL, NULL);
        /* re-create a client connection */
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient,
            picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*) & test_ctx->server_addr, simulated_time,
            PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);
        if (test_ctx->cnx_client == NULL) {
            DBG_PRINTF("%s", "Could not create second client connection\n");    
            ret = -1;
        }
        else {
            for (size_t i = 0; i < test_ctx->nb_test_streams; i++) {
                test_api_delete_test_stream(&test_ctx->test_stream[i]);
            }
            ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
                test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 20000, 100000);
            if (ret != 0) {
                DBG_PRINTF("Second connection fails, ret=%d (x%x)\n", ret, ret);
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

### Rust test body
```rust
fn bad_cnxid() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 2_000_000).expect("bad_cnxid");
}
```

## `picoquictest/tls_api_test.c:client_only_test`
* C test-table name: `client_only`
* C entry function: `client_only_test`
* Rust test: `client_only`
* C source: `picoquictest/tls_api_test.c:6404-6440`
* Rust source: `rs/fq/src/tests/tls_api.rs:179-181`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xc1, 0x10, 0, 0, 0, 0, 0, 0}, 8 };
    int ret;

    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0) {
        /* First, try enforcement. We do set log on the server side, but it is expected to be empty */
        int connection_ret = 0;
        picoquic_enforce_client_only(test_ctx->qserver, 1);
        picoquic_set_qlog(test_ctx->qserver, ".");
        picoquic_set_qlog(test_ctx->qclient, ".");
        connection_ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
        if (connection_ret == 0 && test_ctx->cnx_client->cnx_state < picoquic_state_disconnected) {
            DBG_PRINTF("Connection unexpectedly succeeds, state=%d, ret=%d (0x%x)",
                test_ctx->cnx_client->cnx_state, connection_ret, connection_ret);
            ret = -1;
        }
        else if (test_ctx->cnx_server != NULL) {
            DBG_PRINTF("Connection context created on client-only note, ret=%d (0x%x)",
                connection_ret, connection_ret);
            ret = -1;
        }
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
fn client_only() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("client_only");
}
```

## `picoquictest/tls_api_test.c:connection_drop_test`
* C test-table name: `connection_drop`
* C entry function: `connection_drop_test`
* Rust test: `connection_drop`
* C source: `picoquictest/tls_api_test.c:10548-10576`
* Rust source: `rs/fq/src/tests/tls_api.rs:245-247`

### C test body
```c
{
    int ret = 0;
    picoquic_state_enum target_state[9] = {
        picoquic_state_client_init_sent,
        picoquic_state_client_renegotiate,
        picoquic_state_client_init_resent,
        picoquic_state_server_init,
        picoquic_state_server_handshake,
        picoquic_state_client_handshake_start,
        picoquic_state_server_false_start,
        picoquic_state_server_almost_ready,
        picoquic_state_client_almost_ready
    };
    int target_is_client[9] = {
        1, 1, 1, 0, 0, 1, 0, 0, 1 };

    for (int i = 0; ret == 0 && i < 9; i++) {
        picoquic_state_enum c_state = (target_is_client[i]) ? target_state[i] : picoquic_state_ready;
        picoquic_state_enum s_state = (target_is_client[i]) ? picoquic_state_ready : target_state[i];

        ret = connection_drop_test_one(c_state, s_state, target_is_client[i]);
        if (ret == -1) {
            DBG_PRINTF("connection drop test %d fails", i);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn connection_drop() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("connection_drop");
}
```
