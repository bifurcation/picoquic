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

## `picoquictest/minicrypto_test.c:minicrypto_test`
* C test-table name: `minicrypto`
* C entry function: `minicrypto_test`
* Rust test: `minicrypto`
* C source: `picoquictest/minicrypto_test.c:63-110`
* Rust source: `rs/fq/src/tests/minicrypto.rs:31-65`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t target_time = 1000000;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x81, 0x81, 0xc8, 0x19, 0x40, 0, 6, 7}, 8 };
    int ret = 0;

    picoquic_tls_api_reset(TLS_API_INIT_FLAGS_NO_OPENSSL);
#ifndef PICOQUIC_WITH_MBEDTLS
    picoquic_mbedtls_load(0);
#endif
    ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid, 8, 0, 0, 1);
    if (ret == 0) {
        picoquic_set_binlog(test_ctx->qserver, ".");
        test_ctx->qserver->use_long_log = 1;
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 20000, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_minicrypto, sizeof(test_scenario_minicrypto));
    }

    /* Try to complete the data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, target_time);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    picoquic_tls_api_reset(0);

    return ret;
}
```

### Rust test body
```rust
fn minicrypto() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;
    let target_time: u64 = 1_000_000;

    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL);

    let initial_cid =
        crate::ConnectionId::clone_from_slice(&[0x81, 0x81, 0xc8, 0x19, 0x40, 0, 6, 7])
            .expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 20_000, &mut simulated_time)
        .expect("connection loop");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MINICRYPTO)
        .expect("init scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
        .expect("scenario verify");

    reset_tls_api(0);
}
```

## `picoquictest/parseheadertest.c:header_length_test`
* C test-table name: `header_length`
* C entry function: `header_length_test`
* Rust test: `header_length`
* C source: `picoquictest/parseheadertest.c:1205-1216`
* Rust source: `rs/fq/src/tests/parseheadertest.rs:916-1485`

### C test body
```c
{
    int ret = 0;
    for (size_t i = 0; i < nb_header_length_cases; i++) {
        ret = header_length_test_one(&header_length_case[i]);
        if (ret != 0) {
            DBG_PRINTF("Header length test %zu fails", i);
            break;
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn header_length() {
    const U64MAX: Option<u64> = None;
    let cases: &[HlCase] = &[
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 8,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 63,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 64,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 255,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xff_ffff_ffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: false,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Initial,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::ZeroRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 16,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 0,
            remote_cid_length: 0,
            ptype: PacketType::Handshake,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Handshake,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::Initial,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 8,
            local_cid_length: 8,
            remote_cid_length: 8,
            ptype: PacketType::OneRttProtected,
            sequence: 0,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 255,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xff_ffff_ffff,
            sequence_unack: U64MAX,
            sequence_unack_after: U64MAX,
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: Some(0xfffffffe),
        },
        HlCase {
            is_client: true,
            i_cid_length: 16,
            local_cid_length: 8,
            remote_cid_length: 4,
            ptype: PacketType::OneRttProtected,
            sequence: 0xffffffff,
            sequence_unack: Some(0xffffff00),
            sequence_unack_after: U64MAX,
        },
    ];

    for (i, hlc) in cases.iter().enumerate() {
        header_length_test_one(hlc);
        let _ = i; // used in assertions within header_length_test_one
    }
}
```

## `picoquictest/quic_tester.c:initial_ping_test`
* C test-table name: `initial_ping`
* C entry function: `initial_ping_test`
* Rust test: `initial_ping`
* C source: `picoquictest/quic_tester.c:206-255`
* Rust source: `rs/fq/src/tests/quic_tester.rs:21-62`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint8_t ping_frame[1] = { 1 };
    picoquic_connection_id_t initial_cid = { {0x4e, 0x54, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", PICOQUIC_INTERNAL_TEST_VERSION_1);
    }
    else {
        picoquic_set_qlog(test_ctx->qserver, ".");
    }

    /*
    Insert a ping frame at the client, pass it to the server.
    */
    if (ret == 0) {
        ret = tester_push_frame_packet(test_ctx,
            picoquic_packet_initial,
            ping_frame, sizeof(ping_frame),
            1, 0, simulated_time);
    }

    /*
    * Finish the test
    */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);

        if (ret != 0)
        {
            DBG_PRINTF("Connection loop returns %d\n", ret);
        }
    }

    if (ret == 0) {
        ret = tls_api_test_with_loss_final(test_ctx, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time);
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
fn initial_ping() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;

    let initial_cid =
        ConnectionId::clone_from_slice(&[0x4e, 0x54, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11])
            .expect("build initial CID");

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.set_qlog(".").ok();

    let ping_frame: &[u8] = &[1];
    tester_push_frame_packet(
        &mut test_ctx,
        PacketType::Initial,
        ping_frame,
        true,
        false,
        simulated_time,
    )
    .expect("push ping packet");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    tls_api_test_with_loss_final(
        &mut test_ctx,
        PICOQUIC_TEST_SNI,
        PICOQUIC_TEST_ALPN,
        &mut simulated_time,
    )
    .expect("test with loss final");

    delete_ctx(Some(test_ctx));
}
```

## `picoquictest/satellite_test.c:satellite_bbr1_test`
* C test-table name: `satellite_bbr1`
* C entry function: `satellite_bbr1_test`
* Rust test: `satellite_bbr1`
* C source: `picoquictest/satellite_test.c:278-282`
* Rust source: `rs/fq/src/tests/satellite.rs:380-395`

### C test body
```c
{
    /* Should be less than 7 sec per draft etosat */
    return satellite_test_one(picoquic_bbr1_algorithm, 100000000, 7000000, 250, 3, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_bbr1() {
    let bbr1 = get_congestion_algorithm("bbr1").expect("bbr1");
    satellite_test_one(
        bbr1,
        100_000_000,
        7_000_000,
        250,
        3,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/satellite_test.c:satellite_medium_test`
* C test-table name: `satellite_medium`
* C entry function: `satellite_medium_test`
* Rust test: `satellite_medium`
* C source: `picoquictest/satellite_test.c:259-263`
* Rust source: `rs/fq/src/tests/satellite.rs:323-338`

### C test body
```c
{
    /* Should be less than 20 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 18200000, 50, 10, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_medium() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        18_200_000,
        50,
        10,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```
