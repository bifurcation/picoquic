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

## `picoquictest/mbedtls_test.c:mbedtls_sign_verify_test`
* C test-table name: `mbedtls_sign_verify`
* C entry function: `mbedtls_sign_verify_test`
* Rust test: `mbedtls_sign_verify`
* C source: `picoquictest/mbedtls_test.c:1165-1197`
* Rust source: `rs/fq/src/tests/mbedtls.rs:461-497`

### C test body
```c
{
    int ret = 0;

    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_RSA_KEY, ASSET_RSA_CERT, ASSET_TEST_CA, ASSET_RSA_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP256R1_KEY, ASSET_SECP256R1_CERT, ASSET_TEST_CA, ASSET_SECP256R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP384R1_KEY, ASSET_SECP384R1_CERT, ASSET_TEST_CA, ASSET_SECP384R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP521R1_KEY, ASSET_SECP521R1_CERT, ASSET_TEST_CA, ASSET_SECP521R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP256R1_PKCS8_KEY, ASSET_SECP256R1_PKCS8_CERT, ASSET_TEST_CA, ASSET_SECP256R1_PKCS8_NAME, 0, 0);
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }
    return ret;
}
```

### Rust test body
```rust
fn mbedtls_sign_verify() {
    mbedtls_test_sign_verify_one(
        "certs/rsa/key.pem",
        "certs/rsa/cert.pem",
        "certs/test-ca.crt",
        "rsa.test.example.com",
    )
    .expect("rsa sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1/key.pem",
        "certs/secp256r1/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp384r1/key.pem",
        "certs/secp384r1/cert.pem",
        "certs/test-ca.crt",
        "secp384r1.test.example.com",
    )
    .expect("secp384r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp521r1/key.pem",
        "certs/secp521r1/cert.pem",
        "certs/test-ca.crt",
        "secp521r1.test.example.com",
    )
    .expect("secp521r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1-pkcs8/key.pem",
        "certs/secp256r1-pkcs8/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1-pkcs8 sign_verify");
}
```

## `picoquictest/multipath_test.c:multipath_back1_test`
* C test-table name: `multipath_back1`
* C entry function: `multipath_back1_test`
* Rust test: `multipath_back1`
* C source: `picoquictest/multipath_test.c:1434-1442`
* Rust source: `rs/fq/src/tests/multipath.rs:1486-1488`

### C test body
```c
{
    /* TODO: investigate why 3.3 instead of 3.05 with prior implementation of multipath */
    uint64_t max_completion_microsec = 3300000;

    return  multipath_test_one(max_completion_microsec, multipath_test_back1);
}
```

### Rust test body
```rust
fn multipath_back1() {
    multipath_test_one(3_300_000, MultipathTestId::Back1);
}
```

## `picoquictest/multipath_test.c:multipath_fail_test`
* C test-table name: `multipath_fail`
* C entry function: `multipath_fail_test`
* Rust test: `multipath_fail`
* C source: `picoquictest/multipath_test.c:1302-1307`
* Rust source: `rs/fq/src/tests/multipath.rs:1552-1554`

### C test body
```c
{
    uint64_t max_completion_microsec = 2000000;

    return multipath_test_one(max_completion_microsec, multipath_test_fail);
}
```

### Rust test body
```rust
fn multipath_fail() {
    multipath_test_one(2_000_000, MultipathTestId::Fail);
}
```

## `picoquictest/multipath_test.c:multipath_renew_test`
* C test-table name: `multipath_renew`
* C entry function: `multipath_renew_test`
* Rust test: `multipath_renew`
* C source: `picoquictest/multipath_test.c:1354-1361`
* Rust source: `rs/fq/src/tests/multipath.rs:1609-1611`

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_renew);
}
```

### Rust test body
```rust
fn multipath_renew() {
    multipath_test_one(3_000_000, MultipathTestId::Renew);
}
```

## `picoquictest/netperf_test.c:nat_attack_test`
* C test-table name: `nat_attack`
* C entry function: `nat_attack_test`
* Rust test: `nat_attack`
* C source: `picoquictest/netperf_test.c:652-706`
* Rust source: `rs/fq/src/tests/netperf.rs:251-283`

### C test body
```c
{
    /* Create a connection context */
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint8_t* send_buffer = NULL;
    size_t send_buffer_size = PICOQUIC_MAX_PACKET_SIZE;
    int ret = tls_api_one_scenario_init(&test_ctx, &simulated_time, PICOQUIC_INTERNAL_TEST_VERSION_1, NULL, NULL);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    if (ret == 0 && send_buffer_size > 0) {
        send_buffer = (uint8_t*)malloc(send_buffer_size);
        if (send_buffer == 0) {
            ret = -1;
        }
    }

    if (ret == 0)
    {
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }

    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, nat_attack_scenario, sizeof(nat_attack_scenario));
    }

    /* Run a simplified simulation */
    if (ret == 0) {
        ret = nat_attack_loop(test_ctx, &simulated_time, send_buffer, send_buffer_size, 1);
    }

    /* If the client connection is still up, verify that data was properly received. */
    if (ret == 0 && test_ctx->cnx_client->cnx_state == picoquic_state_ready) {
        ret = tls_api_one_scenario_verify(test_ctx);
    }

    if (ret == 0) {
        DBG_PRINTF("Exit attack loop at time %" PRIu64 ", received %" PRIu64 " packets at client.",
            simulated_time, test_ctx->cnx_client->nb_packets_received);
    }

    if (send_buffer != NULL) {
        free(send_buffer);
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
fn nat_attack() {
    let mut simulated_time = Instant::from_ticks(0);
    let send_buffer_size = MAX_PACKET_SIZE;

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        None,
    )
    .expect("tls_api_one_scenario_init_ex");

    test_ctx.cnx_client().start_client().expect("start client");

    test_api_init_send_recv_scenario(&mut test_ctx, NAT_ATTACK_SCENARIO)
        .expect("init send/recv scenario");

    nat_attack_loop(&mut test_ctx, &mut simulated_time, true).expect("nat attack loop");

    // If the client is still connected, verify data delivery.
    {
        let cnx_c = test_ctx.cnx_client();
        if cnx_c.state() == crate::State::Ready {
            // tls_api_one_scenario_body_verify checks completion metrics.
        }
    }

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)
        .expect("scenario body verify");

    let _ = send_buffer_size; // used only to size the buffer in the C version
}
```
