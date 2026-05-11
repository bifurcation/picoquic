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

## `picoquictest/congestion_test.c:bdp_short_hi_test`
* C test-table name: `bdp_short_hi`
* C entry function: `bdp_short_hi_test`
* Rust test: `bdp_short_hi`
* C source: `picoquictest/congestion_test.c:707-710`
* Rust source: `rs/fq/src/tests/congestion.rs:930-932`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_short_hi);
}
```

### Rust test body
```rust
fn bdp_short_hi() {
    bdp_option_test_one(BdpTestOption::ShortHi);
}
```

## `picoquictest/cpu_limited.c:limited_cubic_test`
* C test-table name: `limited_cubic`
* C entry function: `limited_cubic_test`
* Rust test: `limited_cubic`
* C source: `picoquictest/cpu_limited.c:220-228`
* Rust source: `rs/fq/src/tests/cpu_limited.rs:169-174`

### C test body
```c
{
    limited_test_config_t config;
    limited_config_set_default(&config, 2);
    config.ccalgo = picoquic_cubic_algorithm;
    config.max_completion_time = 4200000;

    return limited_client_test_one(&config);
}
```

### Rust test body
```rust
fn limited_cubic() {
    let mut config = limited_config_default(2);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic algo");
    config.max_completion_time = 4_200_000;
    limited_client_test_one(config);
}
```

## `picoquictest/datagram_tests.c:datagram_small_new_test`
* C test-table name: `datagram_small_new`
* C entry function: `datagram_small_new_test`
* Rust test: `datagram_small_new`
* C source: `picoquictest/datagram_tests.c:676-692`
* Rust source: `rs/fq/src/tests/datagram.rs:753-766`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_small_size = 64;
    dg_ctx.batch_size[0] = 4;
    dg_ctx.batch_size[1] = 4;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 5000;
    dg_ctx.next_gen_time[0] = 50000;
    dg_ctx.next_gen_time[1] = 50000;
    dg_ctx.max_packets_received = 55;
    dg_ctx.use_extended_provider_api = 1;

    return datagram_test_one(7, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_small_new() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        use_extended_provider_api: true,
        ..Default::default()
    };
    datagram_test_one(7, &mut dg_ctx, 0);
}
```

## `picoquictest/ech_test.c:ech_config_test`
* C test-table name: `ech_config`
* C entry function: `ech_config_test`
* Rust test: `ech_config`
* C source: `picoquictest/ech_test.c:89-122`
* Rust source: `rs/fq/src/tests/ech.rs:239-245`

### C test body
```c
{
    int ret = 0;
    char test_server_pub_key_file[512];
    const char* public_name = "test.example.com";
    uint8_t* config = NULL;
    size_t config_len = 0;

    if (picoquic_hpke_kems[0] == NULL) {
        picoquic_tls_api_init();
    }

    ret = picoquic_get_input_path(test_server_pub_key_file, sizeof(test_server_pub_key_file), picoquic_solution_dir,
        PICOQUIC_TEST_ECH_PUB_KEY);
    if (ret != 0) {
        DBG_PRINTF("Cannot find pub_key file in <%s>, err: %d (0x%x)", picoquic_solution_dir, ret, ret);
    }
    else if ((ret = picoquic_ech_create_config_from_public_key(&config, &config_len, test_server_pub_key_file, public_name)) != 0) {
        DBG_PRINTF("Cannot create ECH record from <%s>, err: %d (0x%x)", test_server_pub_key_file, ret, ret);
    }
    /* Save a config representation in ech_config.txt */
    if (ret == 0) {
        ret = picoquic_ech_save_config(config, config_len, ECH_CONFIG_FILE_TXT);
        if (ret == 0) {
            ret = ech_test_check_buf(config, config_len, PICOQUIC_TEST_ECH_CONFIG_REF);
        }
    }

    if (config != NULL) {
        free(config);
    }

    return ret;
}
```

### Rust test body
```rust
fn ech_config() {
    tls_api_init();
    let config = ech_create_config_from_public_key(TEST_ECH_PUB_KEY, ECH_PUBLIC_NAME)
        .expect("create ECH config from public key");
    ech_save_config(&config, ECH_CONFIG_FILE).expect("save ECH config");
    ech_test_check_buf(&config, TEST_ECH_CONFIG_REF).expect("config matches reference");
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
