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

## `picoquictest/sacktest.c:sendack_loop_test`
* C test-table name: `ack_loop`
* C entry function: `sendack_loop_test`
* Rust test: `ack_loop`
* C source: `picoquictest/sacktest.c:539-551`
* Rust source: `rs/fq/src/tests/sacktest.rs:703-708`

### C test body
```c
{
    int ret;
    uint64_t ack_gap[3] = { 0, 2, 10000 };
    uint64_t ack_delay[3] = { 0, 1000, 25 };

    for (int i = 0; i < 3; i++) {
        if ((ret = sendack_loop_test_one(ack_gap[i], ack_delay[i])) != 0) {
            DBG_PRINTF("ack loop test (%" PRIu64", %" PRIu64") fails", ack_gap[i], ack_delay[i]);
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn ack_loop() {
    let combinations: [(u64, u64); 3] = [(0, 0), (2, 1000), (10000, 25)];
    for (ack_gap, ack_delay) in combinations {
        ack_loop_one(ack_gap, ack_delay);
    }
}
```

## `picoquictest/socket_test.c:socket_ecn_test`
* C test-table name: `socket_ecn`
* C entry function: `socket_ecn_test`
* Rust test: `socket_ecn`
* C source: `picoquictest/socket_test.c:234-245`
* Rust source: `rs/fq/src/tests/socket.rs:181-184`

### C test body
```c
{
    int ret;

    ret = socket_ecn_test_one(AF_INET);

    if (ret == 0) {
        ret = socket_ecn_test_one(AF_INET6);
    }

    return ret;
}
```

### Rust test body
```rust
fn socket_ecn() {
    socket_ecn_test_one(AF_INET).expect("ECN on IPv4");
    socket_ecn_test_one(AF_INET6).expect("ECN on IPv6");
}
```

## `picoquictest/stream0_frame_test.c:stream_rank_test`
* C test-table name: `stream_rank`
* C entry function: `stream_rank_test`
* Rust test: `stream_rank`
* C source: `picoquictest/stream0_frame_test.c:937-953`
* Rust source: `rs/fq/src/tests/stream0_frame.rs:729-788`

### C test body
```c
{
    uint64_t stream_rank[] = { 1, 2, 3, 1000, 10000 };
    uint64_t stream_client_bidir[] = { 0, 4, 8, 3996, 39996 };
    uint64_t stream_client_unidir[] = { 2, 6, 10, 3998, 39998 };
    uint64_t stream_server_bidir[] = { 1, 5, 9, 3997, 39997 };
    uint64_t stream_server_unidir[] = { 3, 7, 11, 3999, 39999 };
    size_t n = sizeof(stream_rank) / sizeof(uint64_t);
    int ret = 0;

    ret |= stream_rank_test_one(n, stream_rank, stream_client_bidir, 0, 1);
    ret |= stream_rank_test_one(n, stream_rank, stream_client_unidir, 1, 1);
    ret |= stream_rank_test_one(n, stream_rank, stream_server_bidir, 0, 0);
    ret |= stream_rank_test_one(n, stream_rank, stream_server_unidir, 1, 0);

    return ret;
}
```

### Rust test body
```rust
fn stream_rank() {
    let ranks: [u64; 5] = [1, 2, 3, 1000, 10000];
    let client_bidir: [u64; 5] = [0, 4, 8, 3996, 39996];
    let client_unidir: [u64; 5] = [2, 6, 10, 3998, 39998];
    let server_bidir: [u64; 5] = [1, 5, 9, 3997, 39997];
    let server_unidir: [u64; 5] = [3, 7, 11, 3999, 39999];

    let rank_from_id = |id: u64| -> u64 { (id >> 2) + 1 };
    let id_from_rank = |rank: u64, client_mode: bool, is_unidir: bool| -> u64 {
        ((rank - 1) << 2) | (u64::from(is_unidir) << 1) | u64::from(!client_mode)
    };

    for i in 0..5 {
        let r = ranks[i];

        assert_eq!(
            rank_from_id(client_bidir[i]),
            r,
            "rank_from_id client_bidir[{i}]"
        );
        assert_eq!(
            id_from_rank(r, true, false),
            client_bidir[i],
            "id_from_rank client_bidir[{i}]"
        );

        assert_eq!(
            rank_from_id(client_unidir[i]),
            r,
            "rank_from_id client_unidir[{i}]"
        );
        assert_eq!(
            id_from_rank(r, true, true),
            client_unidir[i],
            "id_from_rank client_unidir[{i}]"
        );

        assert_eq!(
            rank_from_id(server_bidir[i]),
            r,
            "rank_from_id server_bidir[{i}]"
        );
        assert_eq!(
            id_from_rank(r, false, false),
            server_bidir[i],
            "id_from_rank server_bidir[{i}]"
        );

        assert_eq!(
            rank_from_id(server_unidir[i]),
            r,
            "rank_from_id server_unidir[{i}]"
        );
        assert_eq!(
            id_from_rank(r, false, true),
            server_unidir[i],
            "id_from_rank server_unidir[{i}]"
        );
    }
}
```

## `picoquictest/ticket_store_test.c:token_store_test`
* C test-table name: `token_store`
* C entry function: `token_store_test`
* Rust test: `token_store`
* C source: `picoquictest/ticket_store_test.c:342-462`
* Rust source: `rs/fq/src/tests/ticket_store.rs:136-170`

### C test body
```c
{
    int ret = 0;
    picoquic_stored_token_t* p_first_token = NULL;
    picoquic_stored_token_t* p_first_token_bis = NULL;
    picoquic_stored_token_t* p_first_token_ter = NULL;

    uint64_t token_time = 40000000000ull;
    uint64_t current_time = 50000000000ull;
    uint64_t retrieve_time = 60000000000ull;
    uint64_t too_late_time = 150000000000ull;
    uint32_t ttl = 100000;
    uint8_t token[128];
    uint64_t simulated_time = current_time;
    picoquic_quic_t * quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, 0, &simulated_time, NULL, NULL, 0);

    /* Writing an empty file */
    ret = picoquic_save_tokens(quic, test_token_file_name);

    /* Load the empty file again */
    if (ret == 0) {
        simulated_time = retrieve_time;
        ret = picoquic_load_tokens(quic, test_token_file_name);

        /* Verify that the content is empty */
        if (quic->p_first_token != NULL) {
            if (ret == 0) {
                ret = -1;
            }
            picoquic_free_tokens(&quic->p_first_token);
        }
    }

    /* Generate a set of tokens */
    for (size_t i = 0; ret == 0 && i < nb_test_sni; i++) {
        for (size_t j = 0; ret == 0 && j < nb_test_ip_addr; j++) {
            uint16_t token_length = (uint16_t)(64 + j * nb_test_sni + i);
            uint64_t test_ticket_time = token_time / 1000;
            size_t delta_factor = (i * nb_test_ip_addr) + j;
            uint64_t delta_time = ((uint64_t)1000) * delta_factor;
            test_ticket_time += delta_time;
            ret = create_test_token(test_ticket_time, ttl, token, token_length);

            if (ret != 0) {
                break;
            }
            ret = picoquic_store_token(quic,
                test_sni[i], (uint16_t)strlen(test_sni[i]),
                test_ip_addr[j].ip_addr, test_ip_addr[j].ip_addr_length,
                token, token_length);
            if (ret != 0) {
                break;
            }
        }
    }

    /* Verify that they can be retrieved */
    for (size_t i = 0; ret == 0 && i < nb_test_sni; i++) {
        for (size_t j = 0; ret == 0 && j < nb_test_alpn; j++) {
            uint16_t token_length = 0;
            uint16_t expected_length = (uint16_t)(64 + j * nb_test_sni + i);
            uint8_t* token = NULL;
            ret = picoquic_get_token(quic,
                test_sni[i], (uint16_t)strlen(test_sni[i]),
                test_ip_addr[j].ip_addr, test_ip_addr[j].ip_addr_length,
                &token, &token_length, 0);
            if (ret != 0) {
                break;
            }
            if (token_length != expected_length) {
                ret = -1;
                break;
            }

            if (token != NULL) {
                free(token);
                token = NULL;
            }
        }
    }
    /* Store them on a file */
    if (ret == 0) {
        ret = picoquic_save_tokens(quic, test_token_file_name);
        p_first_token = quic->p_first_token;
        quic->p_first_token = NULL;
    }
    /* Load the file again */
    if (ret == 0) {
        simulated_time = retrieve_time;
        ret = picoquic_load_tokens(quic, test_token_file_name);
        p_first_token_bis = quic->p_first_token;
        quic->p_first_token = NULL;
    }

    /* Verify that the two contents match */
    if (ret == 0) {
        ret = token_store_compare(p_first_token, p_first_token_bis);
    }

    /* Reload after a long time */
    if (ret == 0) {
        simulated_time = too_late_time;
        ret = picoquic_load_tokens(quic, test_token_file_name);

        p_first_token_ter = quic->p_first_token;
        quic->p_first_token = NULL;
        if (ret == 0 && p_first_token_ter != NULL) {
            ret = -1;
        }
    }
    /* Free what needs be */
    picoquic_free_tokens(&p_first_token);
    picoquic_free_tokens(&p_first_token_bis);
    picoquic_free_tokens(&p_first_token_ter);

    if (quic != NULL) {
        picoquic_free(quic);
    }
    return ret;
}
```

### Rust test body
```rust
fn token_store() {
    let test_ips: &[IpAddr] = &[
        "192.168.0.1".parse().unwrap(),
        "10.0.0.1".parse().unwrap(),
        "172.16.0.1".parse().unwrap(),
        "::1".parse().unwrap(),
    ];
    let test_sni = [TEST_SNI, "example.com", "example.net"];
    const TOKEN_FILE: &str = "token_store_test.bin";

    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    for ip in test_ips {
        for sni in &test_sni {
            let token = [0xab_u8; 32];
            ctx.qclient
                .store_token(Some(sni), *ip, &token)
                .expect("store_token");
        }
    }

    ctx.qclient.save_tokens(TOKEN_FILE).expect("save_tokens");

    let mut ctx2 = tls_api_init_ctx(&mut t, 0, None).expect("ctx2");
    ctx2.qclient.load_tokens(TOKEN_FILE).expect("load_tokens");

    for ip in test_ips {
        for sni in &test_sni {
            ctx2.qclient
                .get_token(Some(sni), *ip, false)
                .expect("get_token");
        }
    }
}
```

## `picoquictest/tls_api_test.c:cid_length_test`
* C test-table name: `cid_length`
* C entry function: `cid_length_test`
* Rust test: `cid_length`
* C source: `picoquictest/tls_api_test.c:9538-9552`
* Rust source: `rs/fq/src/tests/tls_api.rs:109-113`

### C test body
```c
{
    int ret = 0;
    const uint8_t tested_length[] = { 0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20};

    for (size_t i = 0; i < sizeof(tested_length); i++) {
        ret = cid_length_test_one(tested_length[i]);
        if (ret != 0) {
            DBG_PRINTF("Test fails for cid_length = %d\n", tested_length[i]);
            break;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn cid_length() {
    for len in 0u32..20 {
        cid_length_test_one(len).unwrap_or_else(|e| panic!("cid_length({len}): {e:?}"));
    }
}
```
