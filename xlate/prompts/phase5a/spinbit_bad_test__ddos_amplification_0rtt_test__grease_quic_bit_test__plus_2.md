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

## `picoquictest/spinbit_test.c:spinbit_bad_test`
* C test-table name: `spinbit_bad`
* C entry function: `spinbit_bad_test`
* Rust test: `spinbit_bad`
* C source: `picoquictest/spinbit_test.c:210-218`
* Rust source: `rs/fq/src/tests/spinbit.rs:168-175`

### C test body
```c
{
    int ret = 0;
    if (spinbit_test_one(picoquic_spinbit_on, 123456) == 0 ||
        spinbit_test_one(123455, picoquic_spinbit_null) == 0) {
        ret = -1;
    }
    return ret;
}
```

### Rust test body
```rust
fn spinbit_bad() {
    let r1 = spinbit_test_one(SpinbitVersion::On, SpinbitVersion::Basic);
    let r2 = spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::On);
    assert!(
        r1.is_err() || r2.is_err(),
        "expected at least one invalid per-connection policy to be rejected"
    );
}
```

## `picoquictest/tls_api_test.c:ddos_amplification_0rtt_test`
* C test-table name: `ddos_amplification_0rtt`
* C entry function: `ddos_amplification_0rtt_test`
* Rust test: `ddos_amplification_0rtt`
* C source: `picoquictest/tls_api_test.c:10415-10418`
* Rust source: `rs/fq/src/tests/tls_api.rs:262-264`

### C test body
```c
{
    return ddos_amplification_test_one(1, 0);
}
```

### Rust test body
```rust
fn ddos_amplification_0rtt() {
    ddos_amplification_test_one(1, 0).expect("ddos_amplification_0rtt");
}
```

## `picoquictest/tls_api_test.c:grease_quic_bit_test`
* C test-table name: `grease_quic_bit`
* C entry function: `grease_quic_bit_test`
* Rust test: `grease_quic_bit`
* C source: `picoquictest/tls_api_test.c:11027-11030`
* Rust source: `rs/fq/src/tests/tls_api.rs:367-369`

### C test body
```c
{
    return  grease_quic_bit_test_one(0);
}
```

### Rust test body
```rust
fn grease_quic_bit() {
    grease_quic_bit_test_one(false).expect("grease_quic_bit");
}
```

## `picoquictest/tls_api_test.c:initial_race_test`
* C test-table name: `initial_race`
* C entry function: `initial_race_test`
* Rust test: `initial_race`
* C source: `picoquictest/tls_api_test.c:10772-10862`
* Rust source: `rs/fq/src/tests/tls_api.rs:446-448`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    }

    /* Run an initial loop to make to send the client's first packet, and then replicate it. */
    if (ret == 0) {
        int was_active = 0;
        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

        if (ret == 0) {
            /* Force a repeat of the first packet */
            simulated_time += 100;
            test_ctx->cnx_client->initial_repeat_needed = 1;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            test_ctx->cnx_client->initial_repeat_needed = 0;
            /* Verify that there are two packets in the initial queue */
            if (ret == 0) {
                if (test_ctx->c_to_s_link->first_packet == NULL) {
                    DBG_PRINTF("%s", "No packet queued");
                    ret = -1;
                }
                else if (test_ctx->c_to_s_link->last_packet == test_ctx->c_to_s_link->first_packet) {
                    DBG_PRINTF("%s", "Only one packet queued");
                    ret = -1;
                }
            }
        }

        while (ret == 0 && test_ctx->s_to_c_link->first_packet == NULL){
            /* run a couple of simulation round to process the first server packets,
             * but make sure the server sends only one packet */
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
        }

        if (ret == 0) {
            if (test_ctx->cnx_server == NULL) {
                DBG_PRINTF("%s", "No server connection");
                ret = -1;

            }
            else {
                /* Make sure that the server waits before sending the next packet. */
                test_ctx->cnx_server->next_wake_time += 2000;
            }
        }
    }

    /* Run a connection loop */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2));
    }

    /* Try send data */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* Check that the data was sent and received */
    if (ret == 0) {
        ret = tls_api_one_scenario_verify(test_ctx);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);

        if (ret != 0)
        {
            DBG_PRINTF("Connection close returns %d\n", ret);
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
fn initial_race() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("initial_race");
}
```

## `picoquictest/tls_api_test.c:keylog_test`
* C test-table name: `keylog_test`
* C entry function: `keylog_test`
* Rust test: `keylog_test`
* C source: `picoquictest/tls_api_test.c:12818-12869`
* Rust source: `rs/fq/src/tests/tls_api.rs:515-517`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x55, 0x17, 0xe9, 0x10, 0x90, 0x0, 0x0, 0x0}, 8 };
    int ret;

    /* Ensure that the log files are empty */
    keylog_reset_file(TEST_KEYLOG_FILE_SERVER);
    keylog_reset_file(TEST_KEYLOG_FILE_CLIENT);

    /* Create the contexts */
    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);

    if (ret == 0) {
        /* program key logging. */
        picoquic_enable_sslkeylog(test_ctx->qserver, 1);
        picoquic_enable_sslkeylog(test_ctx->qclient, 1);
        picoquic_set_key_log_file(test_ctx->qserver, TEST_KEYLOG_FILE_SERVER);
        picoquic_set_key_log_file(test_ctx->qclient, TEST_KEYLOG_FILE_CLIENT);
        if (!picoquic_is_sslkeylog_enabled(test_ctx->qserver) ||
            !picoquic_is_sslkeylog_enabled(test_ctx->qclient)) {
            ret = -1;
        }
    }
    if (ret == 0) {
        /* Execute a small scenario to force complete exchange of keys */
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 1000000, 0, 0, 20000,
            1200000);

        if (ret != 0)
        {
            DBG_PRINTF("Scenario body returns error %d\n", ret);
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    if (ret == 0) {
        /* Verify that data was properly written in log files. */
        if (keylog_file_size(TEST_KEYLOG_FILE_SERVER) < 128 ||
            keylog_file_size(TEST_KEYLOG_FILE_CLIENT) < 128) {
            ret = -1;
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn keylog_test() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("keylog");
}
```
