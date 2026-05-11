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

## `picoquictest/stresstest.c:stress_test`
* C test-table name: `stress`
* C entry function: `stress_test`
* Rust test: `stress`
* C source: `picoquictest/stresstest.c:1156-1159`
* Rust source: `rs/fq/src/tests/stresstest.rs:251-255`

### C test body
```c
{
    return stress_or_fuzz_test(NULL, NULL, picoquic_stress_test_duration, 10*picoquic_stress_test_duration);
}
```

### Rust test body
```rust
fn stress() {
    let duration: u64 = 60_000_000; // 1 minute
    let wall_time_max: u64 = 10 * duration;
    stress_or_fuzz_test(duration, wall_time_max).expect("stress_test");
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

## `picoquictest/tls_api_test.c:tls_api_client_first_loss_test`
* C test-table name: `first_loss`
* C entry function: `tls_api_client_first_loss_test`
* Rust test: `first_loss`
* C source: `picoquictest/tls_api_test.c:4155-4158`
* Rust source: `rs/fq/src/tests/tls_api.rs:343-345`

### C test body
```c
{
    return tls_api_loss_test(1ull);
}
```

### Rust test body
```rust
fn first_loss() {
    tls_api_loss_test(1).expect("first_loss");
}
```

## `picoquictest/tls_api_test.c:immediate_ack_test`
* C test-table name: `immediate_ack`
* C entry function: `immediate_ack_test`
* Rust test: `immediate_ack`
* C source: `picoquictest/tls_api_test.c:12689-12791`
* Rust source: `rs/fq/src/tests/tls_api.rs:408-410`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x1a, 0x1a, 0x1a, 0x1a, 0x1a, 0x1a, 0x1a, 0x1a}, 8 };
    int ret;

    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0) {
        picoquic_set_qlog(test_ctx->qserver, ".");
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        int nb_trials = 0;
        int was_active;
        uint64_t immediate_received_at_server = 0;
        uint64_t immediate_cleared_at_server = 0;
        int all_acked = 0;
        uint8_t immediate_ack_frame[2] = { picoquic_frame_type_immediate_ack };
        /* Queue misc frame with "Immediate ACK" set */
        picoquic_queue_misc_frame(test_ctx->cnx_client, immediate_ack_frame, 2, 0,
            picoquic_packet_context_application);
        /* Do couple of rounds until the frame is received;
         * Check that it is received my verifying that the "immediate ACK" 
         * is set in the ACK context at the server. Check the time.
         */
        while (ret == 0 && nb_trials < 16) {
            nb_trials++;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            if (test_ctx->cnx_server != NULL &&
                test_ctx->cnx_server->ack_ctx[picoquic_packet_context_application].act[0].is_immediate_ack_required) {
                immediate_received_at_server = simulated_time;
                break;
            }
        }
        if (ret == 0 && immediate_received_at_server == 0) {
            DBG_PRINTF("Immediate ACK not received after %d rounds", nb_trials);
            ret = -1;
        }
        /* Do a couple rounds until the "immediate ACK" flag is not
         * set at the server anymore. Verify that no time is elapsed since
         * the end of the previous round */
        nb_trials = 0;
        while (ret == 0 && nb_trials < 16) {
            nb_trials++;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            if (test_ctx->cnx_server != NULL &&
                !test_ctx->cnx_server->ack_ctx[picoquic_packet_context_application].act[0].is_immediate_ack_required) {
                immediate_cleared_at_server = simulated_time;
                break;
            }
        }
        if (ret != 0){
            if (immediate_cleared_at_server == 0) {
                DBG_PRINTF("Immediate ACK not cleared after %d rounds", nb_trials);
                ret = -1;
            }
            else if (immediate_cleared_at_server != immediate_received_at_server) {
                DBG_PRINTF("ACK not quite immediate, set at: %" PRIu64 ", cleared at %" PRIu64,
                    immediate_received_at_server, immediate_cleared_at_server);
                ret = -1;
            }
        }
        /* Do couple rounds until the ACK is received at the client. 
         * This is verified by checking that the client ACK queue is
         * empty.
         */
        nb_trials = 0;
        while (ret == 0 && nb_trials < 32) {
            nb_trials++;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
            if (test_ctx->cnx_client != NULL && 
                picoquic_is_cnx_backlog_empty(test_ctx->cnx_client)) {
                all_acked = 1;
                break;
            }
        }
        if (ret == 0 && !all_acked) {
            DBG_PRINTF("ACK was not received at: %" PRIu64, simulated_time);
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
fn immediate_ack() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("immediate_ack");
}
```

## `picoquictest/tls_api_test.c:key_rotation_test`
* C test-table name: `key_rotation`
* C entry function: `key_rotation_test`
* Rust test: `key_rotation`
* C source: `picoquictest/tls_api_test.c:7899-7920`
* Rust source: `rs/fq/src/tests/tls_api.rs:482-484`

### C test body
```c
{
    int ret = key_rotation_test_one(0);

    if (ret == 0) {
        /* test rotation with injection of bad packets on client */
        ret = key_rotation_test_one(2);
        if (ret != 0) {
            DBG_PRINTF("%s", "Packet injection on client defeats rotation.\n", ret);
        }
    }

    if (ret == 0) {
        /* test rotation with injection of bad packets on server */
        ret = key_rotation_test_one(1);
        if (ret != 0) {
            DBG_PRINTF("%s", "Packet injection on server defeats rotation.\n", ret);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn key_rotation() {
    key_rotation_test_one(false).expect("key_rotation");
}
```
