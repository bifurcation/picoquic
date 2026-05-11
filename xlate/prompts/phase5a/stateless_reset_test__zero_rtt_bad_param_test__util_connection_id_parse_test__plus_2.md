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

## `picoquictest/tls_api_test.c:stateless_reset_test`
* C test-table name: `stateless_reset`
* C entry function: `stateless_reset_test`
* Rust test: `stateless_reset`
* C source: `picoquictest/tls_api_test.c:3397-3457`
* Rust source: `rs/fq/src/tests/tls_api.rs:1258-1260`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    uint8_t buffer[128];
    int was_active = 0;

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    /* verify that client and server have the same reset secret */
    if (ret == 0) {
        uint8_t ref_secret[PICOQUIC_RESET_SECRET_SIZE];

        (void)picoquic_create_cnxid_reset_secret(test_ctx->qserver,
            &test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id, ref_secret);
        if (memcmp(test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->reset_secret, ref_secret,
            PICOQUIC_RESET_SECRET_SIZE) != 0) {
            ret = -1;
        }
    }

    /* Prepare to reset */
    if (ret == 0) {
        picoquic_delete_cnx(test_ctx->cnx_server);
        test_ctx->cnx_server = NULL;

        memset(buffer, 0xaa, sizeof(buffer));
        ret = picoquic_add_to_stream(test_ctx->cnx_client, 4,
            buffer, sizeof(buffer), 1);
    }

    /* Perform a couple rounds of sending data */
    for (int i = 0; ret == 0 && i < 64 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected; i++) {
        was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
    }

    /* Client should now be in state disconnected */
    if (ret == 0 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
        ret = -1;
    }

    if (ret == 0 && test_ctx->reset_received == 0) {
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
fn stateless_reset() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_reset");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_bad_param_test`
* C test-table name: `zero_rtt_bad_param`
* C entry function: `zero_rtt_bad_param_test`
* Rust test: `zero_rtt_bad_param`
* C source: `picoquictest/tls_api_test.c:4669-4674`
* Rust source: `rs/fq/src/tests/tls_api.rs:1552-1558`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.change_params = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt_bad_param() {
    zero_rtt_test_one(&ZeroRttTest {
        change_params: true,
        ..Default::default()
    })
    .expect("zero_rtt_bad_param");
}
```

## `picoquictest/util_test.c:util_connection_id_parse_test`
* C test-table name: `connection_id_parse`
* C entry function: `util_connection_id_parse_test`
* Rust test: `connection_id_parse`
* C source: `picoquictest/util_test.c:73-89`
* Rust source: `rs/fq/src/tests/util_test.rs:44-55`

### C test body
```c
{
    int ret = 0;  
    for (size_t i = 0; i < test_cases; ++i) {
        picoquic_connection_id_t cnxid;
        uint8_t id_len = picoquic_parse_connection_id_hexa(expected_str[i], strlen(expected_str[i]), &cnxid);
        if (id_len != expected_cnxid[i].id_len) {
            DBG_PRINTF("Wrong length returned. result: %d, expected: %d\n", id_len, expected_cnxid[i].id_len);
            ret = -1;
        }
        if (picoquic_compare_connection_id(&cnxid, &expected_cnxid[i]) != 0) {
            DBG_PRINTF("%s", "the returned connection id is different than expected.\n");
            ret = -1;
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn connection_id_parse() {
    for (bytes, hex) in EXPECTED_CIDS {
        let mut decoded = [0u8; 20];
        let n = hex.len() / 2;
        for i in 0..n {
            decoded[i] = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("hex byte");
        }
        let parsed = ConnectionId::clone_from_slice(&decoded[..n]).expect("CID under cap");
        let expected = ConnectionId::clone_from_slice(bytes).expect("CID under cap");
        assert_eq!(parsed, expected, "CID parse roundtrip for {hex}");
    }
}
```

## `picoquictest/warptest.c:warptest_video_audio_test`
* C test-table name: `warptest_video_audio`
* C entry function: `warptest_video_audio_test`
* Rust test: `warptest_video_audio`
* C source: `picoquictest/warptest.c:1511-1522`
* Rust source: `rs/fq/src/tests/warptest.rs:42-51`

### C test body
```c
{
    int ret;
    warptest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    ret = warptest_one(2, &spec);

    return ret;
}
```

### Rust test body
```rust
fn warptest_video_audio() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        ..Default::default()
    };
    warptest_one(2, &spec).expect("warptest_video_audio");
}
```

## `picoquictest/wifitest.c:wifi_bbr_long_test`
* C test-table name: `wifi_bbr_long`
* C entry function: `wifi_bbr_long_test`
* Rust test: `wifi_bbr_long`
* C source: `picoquictest/wifitest.c:329-343`
* Rust source: `rs/fq/src/tests/wifitest.rs:164-175`

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_basic,
        50000,
        suspension_basic,
        picoquic_bbr_algorithm,
        NULL,
        3400000,
        1,
        0 };
    int ret = wifi_test_one(wifi_test_bbr_long, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_bbr_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr",
        cc_algo_option: None,
        target_time: 3_400_000,
        simulate_receive_block: true,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR_LONG, &spec).expect("wifi_bbr_long");
}
```
