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

## `picoquictest/stream0_frame_test.c:StreamZeroFrameTest`
* C test-table name: `StreamZeroFrame`
* C entry function: `StreamZeroFrameTest`
* Rust test: `streamzeroframe`
* C source: `picoquictest/stream0_frame_test.c:220-229`
* Rust source: `rs/fq/src/tests/stream0_frame.rs:686-690`

### C test body
```c
{
    int ret = 0;

    for (size_t i = 0; ret == 0 && i < nb_test_cases; i++) {
        ret = StreamZeroFrameOneTest(&test_case[i]);
    }

    return ret;
}
```

### Rust test body
```rust
fn streamzeroframe() {
    for name in &["test_v1", "test_v2", "test_v3"] {
        stream_zero_frame_one_test(name).unwrap_or_else(|_| panic!("{name}"));
    }
}
```

## `picoquictest/tls_api_test.c:transmit_cnxid_retire_disable_test`
* C test-table name: `cnxid_transmit_r_disable`
* C entry function: `transmit_cnxid_retire_disable_test`
* Rust test: `cnxid_transmit_r_disable`
* C source: `picoquictest/tls_api_test.c:6626-6629`
* Rust source: `rs/fq/src/tests/tls_api.rs:229-231`

### C test body
```c
{
    return transmit_cnxid_test_one(1, 1, 0);
}
```

### Rust test body
```rust
fn cnxid_transmit_r_disable() {
    transmit_cnxid_test_one(true, true, false).expect("cnxid_transmit_r_disable");
}
```

## `picoquictest/tls_api_test.c:get_hash_test`
* C test-table name: `get_hash`
* C entry function: `get_hash_test`
* Rust test: `get_hash`
* C source: `picoquictest/tls_api_test.c:12905-12925`
* Rust source: `rs/fq/src/tests/tls_api.rs:351-353`

### C test body
```c
{
    int ret = 0;
    char const* valid_hash = "sha256";
    char const* invalid_hash = "no_such_hash_nada_niente";

    picoquic_tls_api_init();
    if (get_hash_length_test(valid_hash) != 0) {
        ret = -1;
    }
    else if (get_hash_length_test(invalid_hash) == 0) {
        ret = -1;
    }
    else if (get_hash_algo_test(valid_hash) != 0) {
        ret = -1;
    }
    else if (get_hash_algo_test(invalid_hash) == 0) {
        ret = -1;
    }
    return (ret);
}
```

### Rust test body
```rust
fn get_hash() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("get_hash");
}
```

## `picoquictest/tls_api_test.c:immediate_close_test`
* C test-table name: `immediate_close`
* C entry function: `immediate_close_test`
* Rust test: `immediate_close`
* C source: `picoquictest/tls_api_test.c:3627-3690`
* Rust source: `rs/fq/src/tests/tls_api.rs:416-418`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t nb_packet_sent_before_close = 0;
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

    /* Immediate close */
    if (ret == 0) {
        nb_packet_sent_before_close = test_ctx->cnx_server->nb_packets_sent;
        picoquic_close_immediate(test_ctx->cnx_server);
    }
    /* Client sends some data, in order to test the connection */
    if (ret == 0) {
        memset(buffer, 0xaa, sizeof(buffer));
        ret = picoquic_add_to_stream(test_ctx->cnx_client, 4,
            buffer, sizeof(buffer), 1);
    }

    /* Perform a couple rounds of sending data */
    for (int i = 0; ret == 0 && i < 256 ; i++) {
        was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
        if (test_ctx->cnx_client->cnx_state >= picoquic_state_disconnected) {
            /* Client has noticed the disconnect */
            ret = 0;
            break;
        }
    }

    /* Client and server should now be in state disconnected */
    if (ret == 0 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
        ret = -1;
    }

    if (ret == 0 && test_ctx->cnx_server != NULL){
        if (test_ctx->cnx_server->cnx_state != picoquic_state_disconnected) {
            ret = -1;
        }
        else if (nb_packet_sent_before_close != test_ctx->cnx_server->nb_packets_sent)
        {
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
fn immediate_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("immediate_close");
}
```

## `picoquictest/tls_api_test.c:key_rotation_auto_client`
* C test-table name: `key_rotation_client`
* C entry function: `key_rotation_auto_client`
* Rust test: `key_rotation_client`
* C source: `picoquictest/tls_api_test.c:7981-7984`
* Rust source: `rs/fq/src/tests/tls_api.rs:490-492`

### C test body
```c
{
    return key_rotation_auto_one(400, 1);
}
```

### Rust test body
```rust
fn key_rotation_client() {
    key_rotation_auto_one(400, true).expect("key_rotation_client");
}
```
