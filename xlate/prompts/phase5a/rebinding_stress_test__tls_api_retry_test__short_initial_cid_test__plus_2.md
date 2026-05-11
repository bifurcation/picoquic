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

## `picoquictest/tls_api_test.c:rebinding_stress_test`
* C test-table name: `nat_rebinding_stress`
* C entry function: `rebinding_stress_test`
* Rust test: `nat_rebinding_stress`
* C source: `picoquictest/tls_api_test.c:6946-7111`
* Rust source: `rs/fq/src/tests/tls_api.rs:771-774`

### C test body
```c
{
    int nb_trials = 0;
    const int max_trials = 10000;
    int nb_inactive = 0;
    int client_rebinding_done = 0;
    struct sockaddr_in hack_address;
    struct sockaddr_in hack_address_random;
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t last_inject_time = 0;
    uint64_t random_context = 0xBABAC001CAFEull;
    picoquictest_sim_packet_t* last_client_packet_processed = NULL;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    }

    if (ret == 0) {
        memcpy(&hack_address, &test_ctx->client_addr, sizeof(struct sockaddr_in));
        memcpy(&hack_address_random, &test_ctx->client_addr, sizeof(struct sockaddr_in));

        hack_address.sin_port += 1023;

        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    /* Rewrite the sending loop, so we can add injection of packet copies */
    if (ret == 0) {
        test_ctx->client_use_multiple_addresses = 1;
    }

    while (ret == 0 && nb_trials < max_trials && nb_inactive < 256 && TEST_CLIENT_READY && TEST_SERVER_READY) {
        int was_active = 0;

        nb_trials++;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

        if (ret < 0)
        {
            break;
        }

        if (was_active) {
            nb_inactive = 0;
        }
        else {
            nb_inactive++;
        }

        if (test_ctx->test_finished) {
            if (picoquic_is_cnx_backlog_empty(test_ctx->cnx_client) && picoquic_is_cnx_backlog_empty(test_ctx->cnx_server)) {
                break;
            }
        }

        /* Packet injection at the server */
        if (test_ctx->c_to_s_link->last_packet != NULL) {
            uint64_t server_arrival = test_ctx->c_to_s_link->last_packet->arrival_time;

            if (server_arrival > last_inject_time) {
                /* 9% chance of packet injection, 5% chances of reusing test address */
                uint64_t rand100 = picoquic_test_uniform_random(&random_context, 100);
                last_inject_time = server_arrival;
                if (rand100 < 9) {
                    struct sockaddr * bad_address;
                    if (rand100 < 5) {
                        bad_address = (struct sockaddr *)&hack_address;
                    }
                    else {
                        hack_address_random.sin_port = (uint16_t)picoquic_test_uniform_random(&random_context, 0x10000);
                        bad_address = (struct sockaddr *)&hack_address_random;
                    }
                    ret = picoquic_incoming_packet(test_ctx->qserver,
                        test_ctx->c_to_s_link->last_packet->bytes,
                        (uint32_t)test_ctx->c_to_s_link->last_packet->length,
                        bad_address,
                        (struct sockaddr*)&test_ctx->c_to_s_link->last_packet->addr_to, 0, test_ctx->recv_ecn_server,
                        simulated_time);
                }
            }
        }

        /* Initially, the attacker relays packets to the client. Then, it gives up */
        if (test_ctx->cnx_server->pkt_ctx[picoquic_packet_context_application].send_sequence > 256) {
            test_ctx->client_use_multiple_addresses = 0;
        }

        if (test_ctx->client_use_multiple_addresses && test_ctx->s_to_c_link->last_packet != NULL &&
            test_ctx->s_to_c_link->last_packet != last_client_packet_processed){
            last_client_packet_processed = test_ctx->s_to_c_link->last_packet;
            /* Packet reinjection at the client if using the special address */
            if (picoquic_compare_addr((struct sockaddr *)&hack_address, (struct sockaddr *)&test_ctx->s_to_c_link->last_packet->addr_to) == 0)
            {
                picoquic_store_addr(&test_ctx->s_to_c_link->last_packet->addr_to, (struct sockaddr *)&test_ctx->client_addr);
            }
            else if (test_ctx->client_use_nat) {
                if (picoquic_compare_addr((struct sockaddr*) & test_ctx->client_addr_natted, (struct sockaddr*) & test_ctx->s_to_c_link->last_packet->addr_to) == 0) {
                    picoquic_store_addr(&test_ctx->s_to_c_link->last_packet->addr_to, (struct sockaddr*) & test_ctx->client_addr);
                }
                else {
                    /* This packet should be dropped on arrival */
                    test_ctx->s_to_c_link->last_packet->length = 1;
                }
            }
            else if (picoquic_compare_addr((struct sockaddr*) & test_ctx->client_addr, (struct sockaddr*) & test_ctx->s_to_c_link->last_packet->addr_to) != 0) {
                /* This packet should be dropped on arrival */
                test_ctx->s_to_c_link->last_packet->length = 1;
            }
        }

        /* At some point, the client does migrate to a new address */
        if (!client_rebinding_done && test_ctx->cnx_server->pkt_ctx[picoquic_packet_context_application].send_sequence > 128) {
            test_ctx->client_addr_natted = test_ctx->client_addr;
            test_ctx->client_addr_natted.sin_port += 17;
            test_ctx->client_use_nat = 1;
            client_rebinding_done = 1;
        }
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        if (test_ctx->server_callback.error_detected) {
            ret = -1;
        }
        else if (test_ctx->client_callback.error_detected) {
            ret = -1;
        }
        else {
            for (size_t i = 0; ret == 0 && i < test_ctx->nb_test_streams; i++) {
                if (test_ctx->test_stream[i].q_recv_nb != test_ctx->test_stream[i].q_len) {
                    ret = -1;
                }
                else if (test_ctx->test_stream[i].r_recv_nb != test_ctx->test_stream[i].r_len) {
                    ret = -1;
                }
                else if (test_ctx->test_stream[i].q_received == 0 || test_ctx->test_stream[i].r_received == 0) {
                    ret = -1;
                }
            }
        }
        if (ret != 0)
        {
            DBG_PRINTF("Test scenario verification returns %d\n", ret);
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
fn nat_rebinding_stress() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("nat_rebinding_stress");
}
```

## `picoquictest/tls_api_test.c:tls_api_retry_test`
* C test-table name: `retry`
* C entry function: `tls_api_retry_test`
* Rust test: `retry`
* C source: `picoquictest/tls_api_test.c:4053-4056`
* Rust source: `rs/fq/src/tests/tls_api.rs:1128-1130`

### C test body
```c
{
    return tls_api_retry_test_one(0);
}
```

### Rust test body
```rust
fn retry() {
    tls_api_retry_test_one(false).expect("retry");
}
```

## `picoquictest/tls_api_test.c:short_initial_cid_test`
* C test-table name: `short_initial_cid`
* C entry function: `short_initial_cid_test`
* Rust test: `short_initial_cid`
* C source: `picoquictest/tls_api_test.c:8532-8540`
* Rust source: `rs/fq/src/tests/tls_api.rs:1221-1226`

### C test body
```c
{
    int ret = 0;
    for (uint8_t i = 4; ret == 0 && i < 18; i++) {
        ret = short_initial_cid_test_one(i);
    }

    return ret;
}
```

### Rust test body
```rust
fn short_initial_cid() {
    for len in 4u32..=17 {
        short_initial_cid_test_one(len)
            .unwrap_or_else(|e| panic!("short_initial_cid({len}): {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:stop_sending_test`
* C test-table name: `stop_sending`
* C entry function: `stop_sending_test`
* Rust test: `stop_sending`
* C source: `picoquictest/tls_api_test.c:4901-4905`
* Rust source: `rs/fq/src/tests/tls_api.rs:1294-1296`

### C test body
```c
{
    int ret = stop_sending_test_one(0, 0);
    return ret;
}
```

### Rust test body
```rust
fn stop_sending() {
    stop_sending_test_one(false, false).expect("stop_sending");
}
```

## `picoquictest/tls_api_test.c:tls_api_q2_and_r2_stream_test`
* C test-table name: `tls_api_q2_and_r2_stream`
* C entry function: `tls_api_q2_and_r2_stream_test`
* Rust test: `tls_api_q2_and_r2_stream`
* C source: `picoquictest/tls_api_test.c:3276-3279`
* Rust source: `rs/fq/src/tests/tls_api.rs:1374-1378`

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2), 0, 0, 0, 0, 0, 86000, NULL, NULL);
}
```

### Rust test body
```rust
fn tls_api_q2_and_r2_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("q2_and_r2_stream");
}
```
