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

## `picoquictest/satellite_test.c:satellite_jitter_test`
* C test-table name: `satellite_jitter`
* C entry function: `satellite_jitter_test`
* Rust test: `satellite_jitter`
* C source: `picoquictest/satellite_test.c:253-257`
* Rust source: `rs/fq/src/tests/satellite.rs:304-319`

### C test body
```c
{
    /* Should be less than 7 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 6700000, 250, 3, 3000, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_jitter() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        6_700_000,
        250,
        3,
        3_000,
        false,
        false,
        false,
        false,
        false,
    );
}
```

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

## `picoquictest/tls_api_test.c:bad_client_certificate_test`
* C test-table name: `bad_client_certificate`
* C entry function: `bad_client_certificate_test`
* Rust test: `bad_client_certificate`
* C source: `picoquictest/tls_api_test.c:5841-5930`
* Rust source: `rs/fq/src/tests/tls_api.rs:68-71`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_server_cert_store_file[512];
    int ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_BAD_CERT);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }
    else {
        ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    }

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Delete the client context, and recreate with a certificate */
    if (ret == 0)
    {
        if (test_ctx->qclient != NULL) {
            picoquic_free(test_ctx->qclient);
            test_ctx->cnx_client = NULL;
        }

        test_ctx->qclient = picoquic_create(8,
            test_server_cert_file, test_server_key_file, test_server_cert_store_file,
            NULL, test_api_callback, (void*)&test_ctx->client_callback, NULL, NULL, NULL,
            simulated_time, &simulated_time, NULL, NULL, 0);

        if (test_ctx->qclient == NULL) {
            ret = -1;
        }
    }

    /* recreate the client connection */
    if (ret == 0) {
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient, picoquic_null_connection_id,
                                                   picoquic_null_connection_id,
                                                   (struct sockaddr*)&test_ctx->server_addr, 0,
                                                   0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        } else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }

    if (ret == 0) {
        picoquic_set_client_authentication(test_ctx->qserver, 1);
        
        /* Proceed with the connection loop. It should fail */
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_local_error(test_ctx->cnx_server))) {
            ret = -1;
        }
        else if (!picoquic_is_handshake_error(picoquic_get_remote_error(test_ctx->cnx_client))) {
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
fn bad_client_certificate() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("bad_client_certificate");
}
```

## `picoquictest/tls_api_test.c:client_error_test`
* C test-table name: `client_error`
* C entry function: `client_error_test`
* Rust test: `client_error`
* C source: `picoquictest/tls_api_test.c:6383-6397`
* Rust source: `rs/fq/src/tests/tls_api.rs:160-164`

### C test body
```c
{
    int ret = 0;
    char const* mode_name[] = { "stream", "new_connection_id", "stop_sending" };
    int nb_modes = (int)(sizeof(mode_name) / sizeof(char const*));

    for (int mode = 0; mode < nb_modes; mode++) {
        if (client_error_test_modal(mode) != 0) {
            DBG_PRINTF("Client error test mode(%s) failed.\n", mode_name[mode]);
            ret = -1;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn client_error() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 4_000_000).expect("client_error");
}
```
