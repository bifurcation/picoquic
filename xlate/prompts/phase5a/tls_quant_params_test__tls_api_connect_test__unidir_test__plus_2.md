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

## `picoquictest/tls_api_test.c:tls_quant_params_test`
* C test-table name: `quant_params`
* C entry function: `tls_quant_params_test`
* Rust test: `quant_params`
* C source: `picoquictest/tls_api_test.c:5550-5566`
* Rust source: `rs/fq/src/tests/tls_api.rs:1007-1011`

### C test body
```c
{
    picoquic_tp_t test_parameters;

    memset(&test_parameters, 0, sizeof(picoquic_tp_t));

    picoquic_init_transport_parameters(&test_parameters);

    test_parameters.initial_max_data = 0x4000;
    test_parameters.initial_max_stream_id_bidir = 0;
    test_parameters.initial_max_stream_id_unidir = 16384;
    test_parameters.initial_max_stream_data_bidi_local = 0x2000;
    test_parameters.initial_max_stream_data_bidi_remote = 0x2000;
    test_parameters.initial_max_stream_data_uni = 0x2000;

    return tls_api_one_scenario_test(test_scenario_quant, sizeof(test_scenario_quant), 0, 0, 0, 0, 0, 3510000, &test_parameters, NULL);
}
```

### Rust test body
```rust
fn quant_params() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_510_000).expect("quant_params");
}
```

## `picoquictest/tls_api_test.c:tls_api_connect_test`
* C test-table name: `tls_api_connect`
* C entry function: `tls_api_connect_test`
* Rust test: `tls_api_connect`
* C source: `picoquictest/tls_api_test.c:2187-2248`
* Rust source: `rs/fq/src/tests/tls_api.rs:1337-1342`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", PICOQUIC_INTERNAL_TEST_VERSION_1);
    }

    if (ret == 0) {
        int nb_trials = 0;
        int nb_inactive = 0;

        test_ctx->c_to_s_link->loss_mask = NULL;
        test_ctx->s_to_c_link->loss_mask = NULL;

        test_ctx->c_to_s_link->queue_delay_max = 0;
        test_ctx->s_to_c_link->queue_delay_max = 0;

        while (ret == 0 && nb_trials < 1024 && nb_inactive < 512 && (
            !(test_ctx->cnx_client->cnx_state == picoquic_state_ready || test_ctx->cnx_client->cnx_state == picoquic_state_client_ready_start) ||
            (test_ctx->cnx_server == NULL || 
                !(test_ctx->cnx_server != NULL && (test_ctx->cnx_server->cnx_state >= picoquic_state_server_handshake))))) {
            int was_active = 0;
            nb_trials++;

            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

            if (test_ctx->cnx_client->cnx_state == picoquic_state_disconnected &&
                (test_ctx->cnx_server == NULL || test_ctx->cnx_server->cnx_state == picoquic_state_disconnected)) {
                break;
            }

            if (nb_trials == 512) {
                DBG_PRINTF("After %d trials, client state = %d, server state = %d",
                    nb_trials, (int)test_ctx->cnx_client->cnx_state,
                    (test_ctx->cnx_server == NULL) ? -1 : test_ctx->cnx_server->cnx_state);
            }

            if (was_active) {
                nb_inactive = 0;
            }
            else {
                nb_inactive++;
            }
        }

        if (ret != 0)
        {
            DBG_PRINTF("Connection loop returns %d\n", ret);
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
fn tls_api_connect() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("tls_api_connect");
}
```

## `picoquictest/tls_api_test.c:unidir_test`
* C test-table name: `unidir`
* C entry function: `unidir_test`
* Rust test: `unidir`
* C source: `picoquictest/tls_api_test.c:3301-3334`
* Rust source: `rs/fq/src/tests/tls_api.rs:1487-1491`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;

    int ret = tls_api_one_scenario_init(&test_ctx, &simulated_time,
        0, NULL, NULL);

    if (ret == 0) {
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_unidir, sizeof(test_scenario_unidir), 0, 0, 128000, 10000,
            100000);
    }

    /* Verify that the unidir streams are properly closed. */
    if (ret == 0 && test_ctx->cnx_client != NULL && test_ctx->cnx_client->stream_tree.size != 0) {
        DBG_PRINTF("There are %d streams left open on client at the end of test.",
            test_ctx->cnx_client->stream_tree.size);
        ret = -1;
    }

    if (ret == 0 && test_ctx->cnx_server != NULL && test_ctx->cnx_server->stream_tree.size != 0) {
        DBG_PRINTF("There are %d streams left open on client at the end of test.",
            test_ctx->cnx_server->stream_tree.size);
        ret = -1;
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
fn unidir() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 2_000_000).expect("unidir");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_delay_test`
* C test-table name: `zero_rtt_delay`
* C entry function: `zero_rtt_delay_test`
* Rust test: `zero_rtt_delay`
* C source: `picoquictest/tls_api_test.c:4751-4775`
* Rust source: `rs/fq/src/tests/tls_api.rs:1565-1571`

### C test body
```c
{
    int ret = 0;
    int bad_ret;
    const uint64_t nominal_delay_sec = 100000;
    const uint64_t nominal_delay = nominal_delay_sec * 1000000;
    zero_rtt_test_t zrt = { 0 };
    zrt.long_data = 1;
    zrt.extra_delay = nominal_delay + 1000000;

    bad_ret = zero_rtt_test_one(&zrt);
    if (bad_ret == 0) {
        DBG_PRINTF("Zero RTT succeed despite delay = %" PRIu64, " + 1 second.", nominal_delay_sec);
        ret = -1;
    }
    else {
        zrt.extra_delay = nominal_delay - 2000000;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Zero RTT fails for delay = %" PRIu64, " - 2 seconds.", nominal_delay_sec);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn zero_rtt_delay() {
    zero_rtt_test_one(&ZeroRttTest {
        extra_delay: 100_000,
        ..Default::default()
    })
    .expect("zero_rtt_delay");
}
```

## `picoquictest/transport_param_test.c:transport_param_test`
* C test-table name: `transport_param`
* C entry function: `transport_param_test`
* Rust test: `transport_param`
* C source: `picoquictest/transport_param_test.c:854-983`
* Rust source: `rs/fq/src/tests/transport_param.rs:19-45`

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

### Rust test body
```rust
fn transport_param() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    let test_cases: &[TransportParameters] = &[
        TransportParameters::default(),
        TransportParameters {
            initial_max_stream_data_bidi_local: 65535,
            ..Default::default()
        },
        TransportParameters {
            initial_max_data: 0x0040_0000,
            ..Default::default()
        },
        TransportParameters {
            max_idle_timeout: Duration::from_ticks(30_000), // 30 ms in µs
            ..Default::default()
        },
    ];

    for (i, tp) in test_cases.iter().enumerate() {
        transport_param_test_one(&mut ctx.qclient, tp, true)
            .unwrap_or_else(|e| panic!("client tp[{i}]: {e:?}"));
        transport_param_test_one(&mut ctx.qserver, tp, false)
            .unwrap_or_else(|e| panic!("server tp[{i}]: {e:?}"));
    }
}
```
