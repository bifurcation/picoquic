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

## `picoquictest/ticket_store_test.c:ticket_store_test`
* C test-table name: `ticket_store`
* C entry function: `ticket_store_test`
* Rust test: `ticket_store`
* C source: `picoquictest/ticket_store_test.c:109-261`
* Rust source: `rs/fq/src/tests/ticket_store.rs:36-90`

### C test body
```c
{
    int ret = 0;
    picoquic_stored_ticket_t* p_first_ticket = NULL;
    picoquic_stored_ticket_t* p_first_ticket_bis = NULL;
    picoquic_stored_ticket_t* p_first_ticket_ter = NULL;
    uint8_t ipv4_test[4] = { 10, 0, 0, 1 };
    uint8_t ipv6_test[16] = { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16 };

    uint64_t ticket_time = 40000000000ull;
    uint64_t current_time = 50000000000ull;
    uint64_t retrieve_time = 60000000000ull;
    uint64_t too_late_time = 150000000000ull;
    uint32_t ttl = 100000;
    uint8_t ticket[128];
    uint64_t simulated_time = current_time;
    picoquic_quic_t * quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, 0, &simulated_time, NULL, NULL, 0);

    if (quic == NULL) {
        ret = -1;
    }

    if (ret == 0) {
        /* Writing an empty file */
        ret = picoquic_save_tickets(p_first_ticket, current_time, test_ticket_file_name);
    }

    /* Load the empty file again */
    if (ret == 0) {
        simulated_time = retrieve_time;
        ret = picoquic_load_tickets(quic, test_ticket_file_name);

        /* Verify that the content is empty */
        if (quic->p_first_ticket != NULL) {
            if (ret == 0) {
                ret = -1;
            }
            picoquic_free_tickets(&quic->p_first_ticket);
        }
    }

    /* Generate a set of tickets */
    for (size_t i = 0; ret == 0 && i < nb_test_sni; i++) {
        for (size_t j = 0; ret == 0 && j < nb_test_alpn; j++) {
            uint16_t ticket_length = (uint16_t)(64 + j * nb_test_sni + i);
            uint64_t test_ticket_time = ticket_time / 1000;
            size_t delta_factor = (i * nb_test_alpn) + j;
            uint64_t delta_time = ((uint64_t)1000) * delta_factor;
            uint8_t ip_addr_length = 0;
            uint8_t* ip_addr = NULL;
            uint8_t ip_addr_client_length = 0;
            uint8_t* ip_addr_client = NULL;

            test_ticket_time += delta_time;
            ret = create_test_ticket(test_ticket_time, ttl, ticket, ticket_length);

            if (ret != 0) {
                break;
            }

            if ((i & 7) != 0) {
                if ((i & 1) != 0) {
                    ip_addr_length = 16;
                    ip_addr = ipv6_test;
                }
                else {
                    ip_addr_length = 4;
                    ip_addr = ipv4_test;
                }
                if ((i & 2) != 0) {
                    ip_addr_client_length = 16;
                    ip_addr_client = ipv6_test;
                }
                else {
                    ip_addr_client_length = 4;
                    ip_addr_client = ipv4_test;
                }
            }
            ret = picoquic_store_ticket(quic,
                test_sni[i], (uint16_t)strlen(test_sni[i]),
                test_alpn[j], (uint16_t)strlen(test_alpn[j]),
                test_version[j], ip_addr, ip_addr_length,
                ip_addr_client, ip_addr_client_length,
                ticket, ticket_length, &test_tp);
            if (ret != 0) {
                break;
            }
        }
        p_first_ticket = quic->p_first_ticket;
    }

    /* Verify that they can be retrieved */
    for (size_t i = 0; ret == 0 && i < nb_test_sni; i++) {
        for (size_t j = 0; ret == 0 && j < nb_test_alpn; j++) {
            uint16_t ticket_length = 0;
            uint16_t expected_length = (uint16_t)(64 + j * nb_test_sni + i);
            uint8_t* ticket = NULL;
            ret = picoquic_get_ticket(quic,
                test_sni[i], (uint16_t)strlen(test_sni[i]),
                test_alpn[j], (uint16_t)strlen(test_alpn[j]),
                test_version[j],
                &ticket, &ticket_length, NULL, 0);
            if (ret != 0) {
                break;
            }
            if (ticket_length != expected_length) {
                ret = -1;
                break;
            }
        }
    }
    /* Store them on a file */
    if (ret == 0) {
        ret = picoquic_save_tickets(quic->p_first_ticket, current_time, test_ticket_file_name);
    }
    /* Load the file again */
    if (ret == 0) {
        p_first_ticket = quic->p_first_ticket;
        quic->p_first_ticket = NULL;

        simulated_time = retrieve_time;
        ret = picoquic_load_tickets(quic, test_ticket_file_name);
    }

    /* Verify that the two contents match */
    if (ret == 0) {
        p_first_ticket_bis = quic->p_first_ticket;
        ret = ticket_store_compare(p_first_ticket, p_first_ticket_bis);
    }

    /* Reload after a long time */
    if (ret == 0) {
        quic->p_first_ticket = NULL;
        simulated_time = too_late_time;
        ret = picoquic_load_tickets(quic, test_ticket_file_name);
        p_first_ticket_ter = quic->p_first_ticket;
        quic->p_first_ticket = NULL;
        if (ret == 0 && p_first_ticket_ter != NULL) {
            ret = -1;
        }
    }
    /* Free what needs be */
    picoquic_free_tickets(&p_first_ticket);
    picoquic_free_tickets(&p_first_ticket_bis);
    picoquic_free_tickets(&p_first_ticket_ter);

    if (quic != NULL) {
        picoquic_free(quic);
    }

    return ret;
}
```

### Rust test body
```rust
fn ticket_store() {
    let test_sni = [TEST_SNI, "example.com", "example.net"];
    let test_alpn = [TEST_ALPN, "hq05", "hq07"];
    let test_version: [u32; 3] = [0x0000_0001, 0xFF00_0020, 0x0000_0002];
    const TICKET_FILE: &str = "ticket_store_test.bin";

    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    let ip_zero: IpAddr = "0.0.0.0".parse().unwrap();
    let tp = TransportParameters::default();

    for sni in &test_sni {
        for alpn in &test_alpn {
            for &version in &test_version {
                let ticket = [0xcc_u8; 48];
                ctx.qclient
                    .store_ticket(
                        Some(sni),
                        Some(alpn),
                        version,
                        ip_zero,
                        ip_zero,
                        &ticket,
                        &tp,
                    )
                    .expect("store_ticket");
            }
        }
    }

    ctx.qclient
        .save_tickets(t, TICKET_FILE)
        .expect("save_tickets");

    let mut ctx2 = tls_api_init_ctx(&mut t, 0, None).expect("ctx2");
    ctx2.qclient
        .load_tickets(TICKET_FILE)
        .expect("load_tickets");

    for sni in &test_sni {
        for alpn in &test_alpn {
            for &version in &test_version {
                ctx2.qclient
                    .get_ticket(Some(sni), Some(alpn), version, false)
                    .expect("get_ticket");
            }
        }
    }

    t = Instant::from_ticks(u64::MAX);
    ctx.qclient
        .save_tickets(t, TICKET_FILE)
        .expect("save_expired");
}
```

## `picoquictest/tls_api_test.c:bad_coalesce_test`
* C test-table name: `bad_coalesce`
* C entry function: `bad_coalesce_test`
* Rust test: `bad_coalesce`
* C source: `picoquictest/tls_api_test.c:8751-8782`
* Rust source: `rs/fq/src/tests/tls_api.rs:90-94`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Set the coalescing policy in the test context
     */
    if (ret == 0) {
        test_ctx->do_bad_coalesce_test = 1;

        /* Run a basic test scenario
         */

        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 250000);
    }

    /* And then free the resource
     */

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn bad_coalesce() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 2_000_000).expect("bad_coalesce");
}
```
