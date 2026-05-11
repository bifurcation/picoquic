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

## `picoquictest/datagram_tests.c:datagram_loss_test`
* C test-table name: `datagram_loss`
* C entry function: `datagram_loss_test`
* Rust test: `datagram_loss`
* C source: `picoquictest/datagram_tests.c:635-646`
* Rust source: `rs/fq/src/tests/datagram.rs:712-721`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 20000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;

    return datagram_test_one(4, &dg_ctx, 0x040080100200400ull);
}
```

### Rust test body
```rust
fn datagram_loss() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        ..Default::default()
    };
    datagram_test_one(4, &mut dg_ctx, 0x040080100200400);
}
```

## `picoquictest/edge_cases.c:idle_server_test`
* C test-table name: `idle_server`
* C entry function: `idle_server_test`
* Rust test: `idle_server`
* C source: `picoquictest/edge_cases.c:846-860`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1226-1234`

### C test body
```c
{
    int ret = 0;

    if ((ret = idle_server_test_one(1, 30000, 0, 30100000)) == 0 &&
        (ret = idle_server_test_one(2, 60000, 0, 60100000)) == 0 &&
        (ret = idle_server_test_one(3, 5000, 0, 5100000)) == 0 &&
        (ret = idle_server_test_one(4, 0, 0, 30100000)) == 0 &&
        (ret = idle_server_test_one(5, 0, 10000, 10100000)) == 0 &&
        (ret = idle_server_test_one(6, 20000, 60000, 60100000)) == 0 &&
        (ret = idle_server_test_one(7, 60000, 5000, 5100000)) == 0){
        DBG_PRINTF("%s", "All idle timeout tests pass.\n");
    }
    return ret;
}
```

### Rust test body
```rust
fn idle_server() {
    idle_server_test_one(1, 30_000, 0, 30_100_000).expect("case 1");
    idle_server_test_one(2, 60_000, 0, 60_100_000).expect("case 2");
    idle_server_test_one(3, 5_000, 0, 5_100_000).expect("case 3");
    idle_server_test_one(4, 0, 0, 30_100_000).expect("case 4");
    idle_server_test_one(5, 0, 10_000, 10_100_000).expect("case 5");
    idle_server_test_one(6, 20_000, 60_000, 60_100_000).expect("case 6");
    idle_server_test_one(7, 60_000, 5_000, 5_100_000).expect("case 7");
}
```

## `picoquictest/edge_cases.c:reset_loop_test`
* C test-table name: `reset_loop_test`
* C entry function: `reset_loop_test`
* Rust test: `reset_loop_test`
* C source: `picoquictest/edge_cases.c:1757-1873`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1388-1506`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t timeout;
    uint64_t test_stream = 8;
    reset_loop_callback_t cb = { 0 };
    picoquic_stream_head_t* stream = NULL;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        uint8_t bogus_data[4] = { 0 };
        picoquic_set_default_callback(test_ctx->qserver, reset_loop_callback, &cb);
        picoquic_set_callback(test_ctx->cnx_client, reset_loop_callback, &cb);
        picoquic_start_client_cnx(test_ctx->cnx_client);
        picoquic_add_to_stream(test_ctx->cnx_client, 4, bogus_data, 4, 0);
        picoquic_add_to_stream(test_ctx->cnx_client, 8, bogus_data, 4, 0);
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        picoquic_mark_active_stream(test_ctx->cnx_client, 4, 1, &cb);
        picoquic_mark_active_stream(test_ctx->cnx_client, 8, 1, &cb);
        /* set priorities */
        if (ret == 0) {
            ret = picoquic_set_stream_priority(test_ctx->cnx_client, 4, 8);
        }
        if (ret == 0) {
            ret = picoquic_set_stream_priority(test_ctx->cnx_client, 8, 8);
        }
    }

    /* Perform a few rounds of sending loop, but not enough to send all the data */
    if (ret == 0) {
        timeout = simulated_time + 100000;
        ret = tls_api_wait_for_timeout(test_ctx, &simulated_time, timeout);
    }

    /* trigger a reset of tst stream */
    if (ret == 0) {
        ret = picoquic_reset_stream(test_ctx->cnx_server, test_stream, 0);
    }

    /* set priorities */
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_client, 4, 9);
    }
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_client, 8, 7);
    }
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_server, 4, 9);
    }
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_server, 8, 7);
    }

    /* make sure that reset is sent */
    if (ret == 0) {
        timeout = simulated_time + 100000;
        for (int i = 0; ret == 0 && i < 16; i++) {
            int was_active = 0;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, timeout, &was_active);
            if (ret == 0) {
                stream = picoquic_find_stream(test_ctx->cnx_server, test_stream);
                if (stream == NULL) {
                    ret = -1;
                    break;
                }
                else if (stream->reset_sent) {
                    break;
                }
            }
        }
    }
    if (ret == 0 && (stream == NULL || !stream->reset_sent)) {
        DBG_PRINTF("Could not reset stream %" PRIu64, test_stream);
        ret = -1;
    }

    if (ret == 0) {
        /* add data to the stream to elicit some bad behavior */
        uint8_t bogus_data[4] = { 1, 2, 3, 4 }; 
        if (picoquic_add_to_stream(test_ctx->cnx_server, test_stream, bogus_data, 4, 1) == 0) {
            DBG_PRINTF("Adding on stream %" PRIu64 " after reset should be forbidden", test_stream);
            ret = -1;
        }
    }

    if (ret == 0) {
        /* add data to the stream to elicit some bad behavior */
        uint8_t bogus_context[4] = { 0, 0, 0, 0 };
        if (picoquic_mark_active_stream(test_ctx->cnx_server, test_stream, 1, bogus_context) == 0) {
            DBG_PRINTF("Marking stream %" PRIu64 " active after reset should be forbidden", test_stream);
            ret = -1;
        }
    }

    /* Do a loop to check the behavior */
    if (ret == 0) {
        timeout = simulated_time + 2000000;
        ret = tls_api_wait_for_timeout(test_ctx, &simulated_time, timeout);
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
fn reset_loop_test() {
    let mut simulated_time = Instant::from_ticks(0);
    let test_stream: u64 = 8;

    let mut test_ctx = tls_api_init_ctx(
        &mut simulated_time,
        crate::internal::Version::InternalTest1 as u32,
        None,
    )
    .expect("tls_api_init_ctx");

    // The reset-loop callback state used by C is represented here by direct
    // stream inspection after the simulator has delivered the reset.
    test_ctx.qserver.set_default_callback(None);
    test_ctx.cnx_client().set_callback(None);

    test_ctx.cnx_client().start_client().expect("start_client");

    // Queue initial data on streams 4 and 8; triggers server-side stream creation.
    let bogus = [0u8; 4];
    test_ctx
        .cnx_client()
        .add_to_stream(4, &bogus, false)
        .expect("add_to_stream 4");
    test_ctx
        .cnx_client()
        .add_to_stream(8, &bogus, false)
        .expect("add_to_stream 8");

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    // Enable streaming on both client streams with equal priority.
    test_ctx
        .cnx_client()
        .mark_active_stream(4, true, None)
        .expect("mark active 4");
    test_ctx
        .cnx_client()
        .mark_active_stream(8, true, None)
        .expect("mark active 8");
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 8)
        .expect("set client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 8)
        .expect("set client priority 8");

    // Allow stream data to begin flowing before the reset.
    let timeout = simulated_time.ticks() + 100_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout).expect("100ms wait");

    // Server resets stream 8 while the transfer is in progress.
    test_ctx
        .cnx_server()
        .reset_stream(test_stream, 0)
        .expect("reset_stream");

    // Adjust priorities to expose the bug (different priorities post-reset).
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 9)
        .expect("client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 7)
        .expect("client priority 8");
    test_ctx
        .cnx_server()
        .set_stream_priority(4, 9)
        .expect("server priority 4");
    test_ctx
        .cnx_server()
        .set_stream_priority(8, 7)
        .expect("server priority 8");

    // Poll until the server's RESET_STREAM frame has actually been sent.
    let deadline = Instant::from_ticks(simulated_time.ticks() + 100_000);
    for _ in 0..16 {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            deadline,
            &mut was_active,
        )
        .expect("sim round");
        if check_stream_reset_sent(&mut test_ctx, test_stream) {
            break;
        }
    }
    assert!(
        check_stream_reset_sent(&mut test_ctx, test_stream),
        "server did not send RESET_STREAM for stream {test_stream}"
    );

    // After reset, adding data or marking the stream active must be rejected.
    assert!(
        test_ctx
            .cnx_server()
            .add_to_stream(test_stream, &[1, 2, 3, 4], true)
            .is_err(),
        "add_to_stream after reset should be forbidden on stream {test_stream}"
    );
    assert!(
        test_ctx
            .cnx_server()
            .mark_active_stream(test_stream, true, None)
            .is_err(),
        "mark_active_stream after reset should be forbidden on stream {test_stream}"
    );

    // Final loop: verify the connection settles within 2 seconds.
    let timeout2 = simulated_time.ticks() + 2_000_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout2).expect("2s wait");
}
```

## `picoquictest/hashtest.c:picohash_test`
* C test-table name: `picohash`
* C entry function: `picohash_test`
* Rust test: `picohash`
* C source: `picoquictest/hashtest.c:197-200`
* Rust source: `rs/fq/src/tests/hashtest.rs:26-78`

### C test body
```c
{
    return(picohash_test_one(0));
}
```

### Rust test body
```rust
fn picohash() {
    use crate::hash::HashTable;

    let mut t: HashTable<u64, ()> = HashTable::new(32).expect("create hash table");

    assert_eq!(t.len(), 0);

    // Insert odd values 1, 3, 5, 7, 9.
    for i in (1u64..10).step_by(2) {
        assert!(t.insert(i, ()).is_ok(), "insert({i}) failed");
    }
    assert_eq!(t.len(), 5);

    // Every inserted value is retrievable.
    for i in (1u64..10).step_by(2) {
        assert!(t.lookup(&i).is_some(), "lookup({i}) failed");
    }

    // Create collisions: for k in {1, 5}, insert k + 32*j for j in 1..=k.
    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.insert(key, ()).is_ok(), "insert({key}) failed");
        }
    }
    // Original 5 + 1 + 5 = 11.
    assert_eq!(t.len(), 11);

    // Collision entries are retrievable.
    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.lookup(&key).is_some(), "lookup({key}) failed");
        }
    }

    // Even values 0, 2, 4, 6, 8, 10 were never inserted.
    for i in (0u64..=10).step_by(2) {
        assert!(t.lookup(&i).is_none(), "lookup({i}) returned invalid item");
    }

    // Delete values 1 and 9 (first and near-last of the originals).
    for i in (1u64..10).step_by(4) {
        let tok = t.lookup(&i).expect("pre-delete lookup");
        t.remove(tok);
    }
    assert_eq!(t.len(), 8);

    // Deleted values are gone.
    for i in (1u64..10).step_by(4) {
        assert!(t.lookup(&i).is_none(), "deleted value {i} still found");
    }
}
```

## `picoquictest/intformattest.c:intformattest`
* C test-table name: `intformat`
* C entry function: `intformattest`
* Rust test: `intformat`
* C source: `picoquictest/intformattest.c:48-161`
* Rust source: `rs/fq/src/tests/intformattest.rs:35-58`

### C test body
```c
{
    /* Test the formating routines */
    int ret = 0;
    uint8_t bytes[8];
    uint64_t decoded;
    uint64_t parsed;
    uint32_t test32;
    uint32_t test24;
    uint16_t test16;
    uint64_t test64;

    for (int new_encoding = 0; new_encoding < 2; new_encoding++) {
        /* First test with 16 bits macros */
        for (size_t i = 0; ret == 0 && i < nb_test_numbers; i++) {
            test16 = (uint16_t)test_number[i];
            if (new_encoding == 0) {
                picoformat_16(bytes, test16);
            }
            else {
                uint8_t* next_byte = picoquic_frames_uint16_encode(bytes, bytes + sizeof(bytes), test16);
                if ((next_byte - bytes) != 2) {
                    ret = -1;
                }
            }
            decoded = decode_number(bytes, 2);
            if (decoded != test16) {
                ret = -1;
            }
            else {
                parsed = PICOPARSE_16(bytes);
                if (parsed != test16) {
                    ret = -1;
                }
            }
        }

        /* Next test with 24 bits macros */
        for (size_t i = 0; ret == 0 && i < nb_test_numbers; i++) {

            test24 = (uint32_t)(test_number[i]&0xFFFFFF);
            if (new_encoding == 0) {
                picoformat_24(bytes, test24);
            }
            else {
                uint8_t* next_byte = picoquic_frames_uint24_encode(bytes, bytes + sizeof(bytes), test24);
                if ((next_byte - bytes) != 3) {
                    ret = -1;
                }
            }
            decoded = decode_number(bytes, 3);
            if (decoded != test24) {
                ret = -1;
            }
            else {
                parsed = PICOPARSE_24(bytes);
                if (parsed != test24) {
                    ret = -1;
                }
            }
        }

        /* Next test with 32 bits macros */
        for (size_t i = 0; ret == 0 && i < nb_test_numbers; i++) {
            test32 = (uint32_t)test_number[i];
            if (new_encoding == 0) {
                picoformat_32(bytes, test32);
            }
            else {
                uint8_t* next_byte = picoquic_frames_uint32_encode(bytes, bytes + sizeof(bytes), test32);
                if ((next_byte - bytes) != 4) {
                    ret = -1;
                }
            }
            decoded = decode_number(bytes, 4);
            if (decoded != test32) {
                ret = -1;
            }
            else {
                parsed = PICOPARSE_32(bytes);
                if (parsed != test32) {
                    ret = -1;
                }
            }
        }

        /* Test with 64 bits macros */
        for (size_t i = 0; ret == 0 && i < nb_test_numbers; i++) {
            test64 = test_number[i];
            picoformat_64(bytes, test64);
            if (new_encoding == 0) {
                picoformat_64(bytes, test64);
            }
            else {
                uint8_t* next_byte = picoquic_frames_uint64_encode(bytes, bytes + sizeof(bytes), test64);
                if ((next_byte - bytes) != 8) {
                    ret = -1;
                }
            }
            decoded = decode_number(bytes, 8);
            if (decoded != test64) {
                ret = -1;
            }
            else {
                parsed = PICOPARSE_64(bytes);
                if (parsed != test64) {
                    ret = -1;
                }
            }
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn intformat() {
    let mut buf = [0u8; 8];

    for &n in TEST_NUMBERS {
        let n16 = n as u16;
        format_16(&mut buf, n16);
        assert_eq!(decode_number(&buf, 2), n16 as u64, "u16 BE bytes mismatch");
        assert_eq!(parse_16(&buf), n16, "parse_16 roundtrip");

        let n24 = (n & 0xFF_FFFF) as u32;
        format_24(&mut buf, n24);
        assert_eq!(decode_number(&buf, 3), n24 as u64, "u24 BE bytes mismatch");
        assert_eq!(parse_24(&buf), n24, "parse_24 roundtrip");

        let n32 = n as u32;
        format_32(&mut buf, n32);
        assert_eq!(decode_number(&buf, 4), n32 as u64, "u32 BE bytes mismatch");
        assert_eq!(parse_32(&buf), n32, "parse_32 roundtrip");

        format_64(&mut buf, n);
        assert_eq!(decode_number(&buf, 8), n, "u64 BE bytes mismatch");
        assert_eq!(parse_64(&buf), n, "parse_64 roundtrip");
    }
}
```
