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

## `picoquictest/edge_cases.c:ec9a_preemptive_amok_test`
* C test-table name: `ec9a_preemptive_amok`
* C entry function: `ec9a_preemptive_amok_test`
* Rust test: `ec9a_preemptive_amok`
* C source: `picoquictest/edge_cases.c:534-620`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1192-1207`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t initial_losses = 0x800;
    uint8_t test_case_id = 0x9a;
    uint64_t cnx_server_idle_timeout = 0;
    uint64_t cnx_server_nb_preemptive_repeat = 0;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 0, &simulated_time, initial_losses, 12);

    if (ret == 0) {
        if (test_ctx->cnx_server == NULL) {
            DBG_PRINTF("Unexpected state, client: %d, server: NULL",
                test_ctx->cnx_client->cnx_state);
            ret = -1;
        }
        else if ( test_ctx->cnx_server->cnx_state != picoquic_state_ready ||
            !test_ctx->test_finished || 
            test_ctx->cnx_server->pkt_ctx[picoquic_packet_context_application].pending_first == NULL){
            DBG_PRINTF("Unexpected state, server: %d, test finished: %d, queue for repeat %s",
                test_ctx->cnx_server->cnx_state, test_ctx->test_finished, 
                (test_ctx->cnx_server->pkt_ctx[picoquic_packet_context_application].pending_first == NULL)?"empty":"full");
            ret = -1;
        }
    }
    /* Do a loop involving only the server */
    if (ret == 0) {
        uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
        size_t send_length;
        size_t send_msg_size;
        struct sockaddr_storage addr_to;
        struct sockaddr_storage addr_from;
        int if_index;
        picoquic_connection_id_t log_id;
        picoquic_cnx_t * last_cnx;
        int loop_count = 0;
        int send_count = 0;
        const int send_count_max = 50;
        uint64_t repeat_begin = simulated_time;
        uint64_t repeat_duration = 0;

        cnx_server_idle_timeout = test_ctx->cnx_server->idle_timeout;
        cnx_server_nb_preemptive_repeat = test_ctx->cnx_server->nb_preemptive_repeat;

        picoquic_reinsert_by_wake_time(test_ctx->qserver, test_ctx->cnx_server, simulated_time);

        while (test_ctx->qserver->current_number_connections > 0 && test_ctx->cnx_server->cnx_state == picoquic_state_ready && loop_count < 10000 && ret == 0) {
            loop_count++;
            cnx_server_nb_preemptive_repeat = test_ctx->cnx_server->nb_preemptive_repeat;
            simulated_time = picoquic_get_next_wake_time(test_ctx->qserver, simulated_time);
            ret = picoquic_prepare_next_packet_ex(test_ctx->qserver, simulated_time, buffer,
                sizeof(buffer), &send_length, &addr_to, &addr_from, &if_index, &log_id,
                &last_cnx, &send_msg_size);
            if (ret != 0) {
                DBG_PRINTF("Prepare next returns an error: %d (0x%x)", ret, ret);
            }
            else if (send_length > 0) {
                send_count++;
            }
        }

        if (ret == 0) {
            repeat_duration = simulated_time - repeat_begin;
            if (send_count > send_count_max) {
                DBG_PRINTF("Repeated %d packets, more that the %d expected",
                    send_count, send_count_max);
                ret = -1;
            }
            else if (repeat_duration > cnx_server_idle_timeout) {
                DBG_PRINTF("End at t=%" PRIu64 ", later than %" PRIu64,
                    simulated_time, cnx_server_idle_timeout);
                ret = -1;
            }
            else if (cnx_server_nb_preemptive_repeat == 0) {
                DBG_PRINTF("%s", "No preemptive repeat");
                ret = -1;
            }
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
fn ec9a_preemptive_amok() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0x9a, false, &mut simulated_time, 0x800, 12).expect("edge_case_prepare");
    assert!(test_ctx.has_cnx_server(), "server connection must exist");
    assert!(test_ctx.server_ready(), "server must be in ready state");
    assert!(test_ctx.test_finished, "data transfer must have completed");
    let (send_count, repeat_duration) =
        ec9a_server_loop(&mut test_ctx, &mut simulated_time).expect("server loop");
    assert!(
        send_count <= 50,
        "server sent too many repeat packets: {send_count}"
    );
    // repeat_duration must be <= idle_timeout (checked inside ec9a_server_loop)
    let _ = repeat_duration;
}
```

## `picoquictest/edge_cases.c:initial_pto_srv_test`
* C test-table name: `initial_pto_srv`
* C entry function: `initial_pto_srv_test`
* Rust test: `initial_pto_srv`
* C source: `picoquictest/edge_cases.c:1493-1561`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1278-1333`

### C test body
```c
{
    int ret = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    size_t length = 0;
    int has_packet;
    int has_initial;
    int has_handshake;
    uint64_t simulated_time = 0;
    picoquic_connection_id_t initial_cid = { { 0x94, 0x01, 0x85, 0, 0, 0, 0, 0}, 8 };

    /* Create a client. */
    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);
    if (ret != 0) {
        DBG_PRINTF("Cannot initialize context, ret = 0x%x", ret);
    }
    else {
        /* Set the binlog */
        picoquic_set_qlog(test_ctx->qserver, ".");
        /* start the client connection */
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }
    /* Send the initial packet */
    if (ret == 0) {
        ret = initial_pto_prepare(test_ctx, &simulated_time, &length);
        if (ret == 0 && length < 1200) {
            length = -1;
        }
    }
    if (ret == 0) {
        /* pretend that the address is validated, so the server sends ACKS, etc. */
        test_ctx->cnx_server->initial_validated = 1;
        /* Set the time to server next wake. */
        simulated_time = picoquic_get_next_wake_time(test_ctx->qserver, simulated_time);
        has_initial = 0;
        has_handshake = 0;
        /* Get the first flight. */
        do {
            has_packet = 0;
            ret = pto_server_prepare(test_ctx, simulated_time, &has_packet, &has_initial, &has_handshake);
        } while (ret == 0 && has_packet);
    }

    /* verify that the packet has Initial and Handshake */
    if (ret == 0) {
        /* Set the time to server next wake. */
        has_initial = 0;
        has_handshake = 0;
        simulated_time = picoquic_get_next_wake_time(test_ctx->qserver, simulated_time);
        /* Get the PTO. */
        do {
            has_packet = 0;
            ret = pto_server_prepare(test_ctx, simulated_time, &has_packet, &has_initial, &has_handshake);
        } while (ret == 0 && has_packet);

        if (ret == 0 && !(has_initial && has_handshake)) {
            /* Bug. The server out to repeat both the initial packet and some handshake packet */
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
fn initial_pto_srv() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x94, 0x01, 0x85, 0, 0, 0, 0, 0]).expect("8-byte CID");
    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.cnx_client().start_client().expect("start_client");

    let length =
        initial_pto_prepare(&mut test_ctx, &mut simulated_time).expect("initial_pto_prepare");
    assert!(length >= 1200, "initial packet too short: {length}");

    test_ctx.cnx_server().initial_validated = true;
    simulated_time = Instant::from_ticks(test_ctx.qserver.next_wake_time(simulated_time));
    let mut has_initial = false;
    let mut has_handshake = false;
    loop {
        let has_packet = pto_server_prepare(
            &mut test_ctx,
            simulated_time,
            &mut has_initial,
            &mut has_handshake,
        )
        .expect("pto_server_prepare first flight");
        if !has_packet {
            break;
        }
    }

    simulated_time = Instant::from_ticks(test_ctx.qserver.next_wake_time(simulated_time));
    has_initial = false;
    has_handshake = false;
    loop {
        let has_packet = pto_server_prepare(
            &mut test_ctx,
            simulated_time,
            &mut has_initial,
            &mut has_handshake,
        )
        .expect("pto_server_prepare PTO");
        if !has_packet {
            break;
        }
    }

    assert!(
        has_initial && has_handshake,
        "server PTO did not include both Initial and Handshake packets"
    );
}
```

## `picoquictest/edge_cases.c:reset_need_reset_test`
* C test-table name: `reset_need_reset`
* C entry function: `reset_need_reset_test`
* Rust test: `reset_need_reset`
* C source: `picoquictest/edge_cases.c:1223-1226`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1373-1375`

### C test body
```c
{
    return reset_repeat_test_one(reset_need_reset);
}
```

### Rust test body
```rust
fn reset_need_reset() {
    reset_repeat_test_one(ResetTestKind::NeedReset).expect("reset_need_reset");
}
```

## `picoquictest/hashtest.c:picohash_bytes_test`
* C test-table name: `picohash_bytes`
* C entry function: `picohash_bytes_test`
* Rust test: `picohash_bytes`
* C source: `picoquictest/hashtest.c:222-259`
* Rust source: `rs/fq/src/tests/hashtest.rs:140-167`

### C test body
```c
{
    uint8_t test[1024];
    uint8_t k[16];
    int ret = 0;
    size_t test_lengths[12] = { 1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024 };
    uint64_t href[12] = {
        0x03016721e32d7aa7,
        0x64208401ad85bed5,
        0x44587b0209479519,
        0x14a481748ee6d77e,
        0x9a44370fd1b8c1ee,
        0x27081725c4164c1a,
        0x2f1f325da756df85,
        0x2aa4fda796f9ffff,
        0x8ded0692d7038037,
        0x7893f9399f507284,
        0x47a065dbeea77343,
        0xb543a5b3c675127d
    };

    hash_test_init(test, sizeof(test), k, sizeof(k));

    /* Compute or check the reference siphash value */
    for (size_t i = 0; i < sizeof(test_lengths) / sizeof(size_t); i++) {
        uint64_t h = picohash_bytes(test, (uint32_t)test_lengths[i], k);
        if (h != href[i]) {
            DBG_PRINTF("H[%zu] = %" PRIx64 " instead of %"PRIx64, i, h, href[i]);
#if 1//def COMPUTING_REFERENCE_BASIC_VALUE
            href[i] = h;
#else
            ret = -1;
            break;
#endif
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn picohash_bytes() {
    use crate::hash::hash_bytes;

    let mut test = [0u8; 1024];
    let mut k = [0u8; 16];
    hash_test_init(&mut test, &mut k);

    let lengths: [usize; 12] = [1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024];
    let expected: [u64; 12] = [
        0x0301_6721_e32d_7aa7,
        0x6420_8401_ad85_bed5,
        0x4458_7b02_0947_9519,
        0x14a4_8174_8ee6_d77e,
        0x9a44_370f_d1b8_c1ee,
        0x2708_1725_c416_4c1a,
        0x2f1f_325d_a756_df85,
        0x2aa4_fda7_96f9_ffff,
        0x8ded_0692_d703_8037,
        0x7893_f939_9f50_7284,
        0x47a0_65db_eea7_7343,
        0xb543_a5b3_c675_127d,
    ];

    for (i, &len) in lengths.iter().enumerate() {
        let h = hash_bytes(&test[..len], &k);
        assert_eq!(h, expected[i], "picohash_bytes[{i}] for len={len}");
    }
}
```
