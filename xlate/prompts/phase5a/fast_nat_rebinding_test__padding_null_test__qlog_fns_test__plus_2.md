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

## `picoquictest/tls_api_test.c:fast_nat_rebinding_test`
* C test-table name: `nat_rebinding_fast`
* C entry function: `fast_nat_rebinding_test`
* Rust test: `nat_rebinding_fast`
* C source: `picoquictest/tls_api_test.c:6110-6225`
* Rust source: `rs/fq/src/tests/tls_api.rs:747-749`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    const int nb_switches_required = 6;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xfa, 0x57, 0x08, 0xa7, 0, 0, 0, 0}, 8 };

    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0 && test_ctx == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    }

    if (ret == 0) {
        /* Set up logging */ 
        picoquic_set_qlog(test_ctx->qserver, ".");
        picoquic_set_qlog(test_ctx->qclient, ".");
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_sustained, sizeof(test_scenario_sustained));
    }

    /* Perform a data sending loop */
    if (ret == 0) {
        uint64_t delta_t = 5 * (test_ctx->c_to_s_link->microsec_latency + test_ctx->s_to_c_link->microsec_latency);
        uint64_t next_time = simulated_time + 200000000;
        int nb_trials = 0;
        int nb_inactive = 0;
        int max_trials = 1000000;
        uint64_t switch_time = simulated_time;
        int switched = 0;
        int nb_switched = 0;

        test_ctx->client_use_nat = 1;
        test_ctx->client_addr_natted = test_ctx->client_addr;
        test_ctx->client_addr_natted.sin_port += 17;

        while (ret == 0 && nb_trials < max_trials && nb_inactive < 256 && simulated_time < next_time && TEST_CLIENT_READY && TEST_SERVER_READY) {
            int was_active = 0;

            nb_trials++;

            ret = tls_api_one_sim_round(test_ctx, &simulated_time, next_time, &was_active);

            if (ret < 0)
            {
                break;
            }

            if (ret == 0 && test_ctx->cnx_server != NULL && test_ctx->cnx_server->cnx_state == picoquic_state_ready) {
                if (((struct sockaddr_in*) & test_ctx->cnx_server->path[0]->first_tuple->peer_addr)->sin_port == 0) {
                    DBG_PRINTF("Client address out of sync, port: %d", ((struct sockaddr_in*) & test_ctx->cnx_server->path[0]->first_tuple->peer_addr)->sin_port);
                }
                else if (((struct sockaddr_in*) & test_ctx->cnx_server->path[0]->first_tuple->peer_addr)->sin_port == test_ctx->client_addr_natted.sin_port) {
                    if (switched) {
                        if (simulated_time > switch_time + delta_t &&
                            nb_switched < nb_switches_required) {
                            switched = 0;
                        }
                    }
                    else
                    {
                        /* Change the client address */
                        test_ctx->client_addr_natted.sin_port += 17;
                        switched = 1;
                        switch_time = simulated_time;
                        nb_switched++;
                    }
                }
            }

            if (was_active) {
                nb_inactive = 0;
            }
            else {
                nb_inactive++;

                if (nb_inactive == 254) {
                    DBG_PRINTF("Almost stalled after %d trials, %d inactive, %d switches", nb_trials, nb_inactive, nb_switched);
                }
            }

            if (test_ctx->test_finished) {
                if (picoquic_is_cnx_backlog_empty(test_ctx->cnx_client) && picoquic_is_cnx_backlog_empty(test_ctx->cnx_server)) {
                    break;
                }
            }
        }

        DBG_PRINTF("Exit after %d trials, %d inactive, %d switches", nb_trials, nb_inactive, nb_switched);

        /* Verify that the test was effective */
        if (ret == 0 && nb_switched < nb_switches_required) {
            ret = -1;
        }
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_verify(test_ctx);
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
fn nat_rebinding_fast() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("nat_rebinding_fast");
}
```

## `picoquictest/tls_api_test.c:padding_null_test`
* C test-table name: `padding_null`
* C entry function: `padding_null_test`
* Rust test: `padding_null`
* C source: `picoquictest/tls_api_test.c:8736-8739`
* Rust source: `rs/fq/src/tests/tls_api.rs:860-862`

### C test body
```c
{
    return padding_test_one(0, 0);
}
```

### Rust test body
```rust
fn padding_null() {
    padding_test_one(0, 0).expect("padding_null");
}
```

## `picoquictest/tls_api_test.c:qlog_fns_test`
* C test-table name: `qlog_fns`
* C entry function: `qlog_fns_test`
* Rust test: `qlog_fns`
* C source: `picoquictest/tls_api_test.c:9200-9203`
* Rust source: `rs/fq/src/tests/tls_api.rs:956-958`

### C test body
```c
{
    return qlog_fns_test_one(0);
}
```

### Rust test body
```rust
fn qlog_fns() {
    qlog_fns_test_one(0).expect("qlog_fns");
}
```

## `picoquictest/tls_api_test.c:random_public_tester_test`
* C test-table name: `random_public_tester`
* C entry function: `random_public_tester_test`
* Rust test: `random_public_tester`
* C source: `picoquictest/tls_api_test.c:10088-10131`
* Rust source: `rs/fq/src/tests/tls_api.rs:1027-1032`

### C test body
```c
{
#define RANDOM_PUBLIC_TEST_CONST 11
#define RANDOM_PUBLIC_TEST_ROUNDS 100
#define RANDOM_PUBLIC_CHI_SQUARE 18.31 /* Fail if significance of bias < P = 0.05 */
    int ret = 0;
    int r_count[RANDOM_PUBLIC_TEST_CONST];

    picoquic_public_random_seed_64(RANDOM_PUBLIC_TEST_SEED, 1);

    memset(r_count, 0, sizeof(r_count));

    for (int i = 0; i < RANDOM_PUBLIC_TEST_CONST*RANDOM_PUBLIC_TEST_ROUNDS; i++) {
        uint64_t x = picoquic_public_uniform_random(RANDOM_PUBLIC_TEST_CONST);

        if (x >= RANDOM_PUBLIC_TEST_CONST) {
            DBG_PRINTF("Value %d >= %d\n", x, RANDOM_PUBLIC_TEST_CONST);
            ret = -1;
            break;
        }
        else {
            r_count[x] += 1;
        }
    }

    if (ret == 0) {
        double chi_squared = 0;

        for (int i = 0; i < RANDOM_PUBLIC_TEST_CONST; i++) {
            double delta = ((double)RANDOM_PUBLIC_TEST_ROUNDS - r_count[i]);
            double d2 = delta * delta;
            d2 /= ((double)RANDOM_PUBLIC_TEST_ROUNDS);
            chi_squared += d2;
        }

        if (chi_squared > RANDOM_PUBLIC_CHI_SQUARE) {
            DBG_PRINTF("Chi2 = %f, larger than %f\n", chi_squared, RANDOM_PUBLIC_CHI_SQUARE);
            ret = -1;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn random_public_tester() {
    for _ in 0..100 {
        tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
            .expect("random_public_tester");
    }
}
```

## `picoquictest/tls_api_test.c:red_fast_test`
* C test-table name: `red_fast`
* C entry function: `red_fast_test`
* Rust test: `red_fast`
* C source: `picoquictest/tls_api_test.c:11132-11136`
* Rust source: `rs/fq/src/tests/tls_api.rs:1094-1096`

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_fastcc_algorithm, 500000, 250);
    return ret;
}
```

### Rust test body
```rust
fn red_fast() {
    red_cc_algotest("fast", 500_000, 250).expect("red_fast");
}
```
