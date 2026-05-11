# Phase 5B repair Rust test mismatches

You are repairing Phase 5A `needs_fix` entries.  The goal
is to make the Rust tests faithfully check the same behavior
as the C tests.

Rules:

* Edit Rust tests, Rust test helpers, and Rust test fixtures
  only: `rs/fq/src/tests/` and `rs/fq/tests/fixtures/`.
* Do not edit C sources.
* Do not weaken assertions, skip important C cases, or replace
  tests with placeholders.
* If the test already matches after closer inspection, report
  `ok` and do not edit source.
* Phase 5B is about test/API correspondence, not test success.
  The Rust test must exist, compile as a test, and be runnable
  by the Rust test harness, but it may fail arbitrarily early
  because the Rust library implementation is incomplete.
* Do not report `blocked` merely because the implementation
  returns the wrong state, fails a handshake, lacks protocol
  behavior, or would fail the test. Those are Phase 5C issues.
* Report `blocked` only when the faithful test cannot be
  written, compiled, or exposed as a runnable Rust test because
  the necessary Rust API/test-harness surface is missing or
  ambiguous.
* Do not run full `cargo test` in this pass. Use source review
  and, if needed, `cargo check --tests` for compile validation.

Owned Rust test file(s): `rs/fq/src/tests/datagram.rs`, `rs/fq/src/tests/ticket_store.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/datagram_tests.c:datagram_small_packet_test`
* C test-table name: `datagram_small_packet`
* C entry function: `datagram_small_packet_test`
* Rust test: `datagram_small_packet`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Rust span: `rs/fq/src/tests/datagram.rs:756-773`
* Phase 5A analysis: The Rust wrapper preserves the C field values, but the underlying Rust datagram helper is synthetic: it bypasses actual QUIC DATAGRAM frames/callbacks, overwrites negotiation results, and ignores bandwidth-sensitive delivery behavior from picosec_per_byte, weakening duration, packet, and latency checks.
* Phase 5A fix note: Repair the shared Rust datagram helper to use the real simulator/datagram provider path, preserve the small-packet inputs, assert negotiated parameters, and base packet/duration/latency checks on actual connection behavior.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust entry preserves the C small-packet fields and calls datagram_test_one(9,...), but the shared Rust helper still uses synthetic datagram delivery/ack accounting instead of the real simulator DATAGRAM provider, receive, and ack harness surface used by C.
* Phase 5C fix note: Replace the synthetic datagram app round with the real QUIC DATAGRAM provider/callback path while preserving negotiation, packet, duration, and latency checks.

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_small_size = 64;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 20000;
    dg_ctx.send_delay = 100;
    dg_ctx.next_gen_time[0] = 50000;
    dg_ctx.next_gen_time[1] = 50000;
    dg_ctx.link_latency = 10000;
    dg_ctx.picosec_per_byte = 20000; /* 400 Mbps */
    dg_ctx.dg_latency_target[0] = 20000;
    dg_ctx.dg_latency_target[1] = 13500;
    dg_ctx.use_extended_provider_api = 1;
    dg_ctx.one_datagram_per_packet = 1;
    dg_ctx.nb_trials_max = 200000;
    dg_ctx.duration_max = 2060000;

    return datagram_test_one(9, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
        {
            return Err(Error::Generic);
        }
        let _ = final_ctx.dg_number_delta_max[i] > final_ctx.dg_number_delta_target[i];
    }

    *dg_ctx = final_ctx;

    Ok(())
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `datagram_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram() {
    let mut dg_ctx = DatagramSendRecvCtx {
```

## `picoquictest/datagram_tests.c:datagram_wifi_test`
* C test-table name: `datagram_wifi`
* C entry function: `datagram_wifi_test`
* Rust test: `datagram_wifi`
* Expected Rust file: `rs/fq/src/tests/datagram.rs`
* Rust span: `rs/fq/src/tests/datagram.rs:789-802`
* Phase 5A analysis: The Rust datagram_wifi initializer preserves C's WiFi parameters, but the shared helper masks datagram negotiation failures by assigning expected frame sizes instead of checking them.
* Phase 5A fix note: Use the same datagram negotiation assertion fix in Rust datagram_test_one_result; keep the WiFi timing, trial-limit, latency, and link-latency inputs unchanged.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The WiFi wrapper fields match C, but it uses the same shared helper that bypasses QUIC DATAGRAM provider/receive/ack callbacks with synthetic delivery and ACK accounting.
* Phase 5C fix note: Wire WiFi datagram execution through the real QUIC DATAGRAM callback/provider path instead of manual app-round delivery.

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 1000;
    dg_ctx.dg_target[1] = 1000;
    dg_ctx.send_delay = 2000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;
    dg_ctx.dg_latency_target[0] = 305000;
    dg_ctx.dg_latency_target[1] = 280000;
    dg_ctx.test_wifi = 1;
    dg_ctx.nb_trials_max = 64000;
    dg_ctx.link_latency = 25000;

    return datagram_test_one(8, &dg_ctx, 0);
}
```

### Current Rust test body
```rust
        dg_latency_target: [18_000, 18_000],
        ..Default::default()
    };
    datagram_test_one(2, &mut dg_ctx, 0);
}

/// C: `datagram_rt_skip_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_rt_skip() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 10],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
```

## `picoquictest/ticket_store_test.c:ticket_store_test`
* C test-table name: `ticket_store`
* C entry function: `ticket_store_test`
* Rust test: `ticket_store`
* Expected Rust file: `rs/fq/src/tests/ticket_store.rs`
* Rust span: `rs/fq/src/tests/ticket_store.rs:174-296`
* Phase 5A analysis: Rust is materially weaker and different: it skips empty-file load verification, uses a different ALPN set and a 3x3x3 version cartesian product, uses fixed 48-byte tickets/default TP/zero IPs, does not check expected ticket lengths or full saved-vs-loaded content, and does not verify expired reload produces no tickets.
* Phase 5A fix note: Mirror the C matrix and data: empty save/load check, three SNI by three ALPN/version pairs, create_test_ticket-style variable lengths and TTL, IPv4/IPv6 address variants, non-default TP comparison, retrieval length checks, full round-trip content comparison, and too-late reload expiry check.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust ticket_store mirrors the C save/load/store/get matrix and assertions, but the shared util harness it imports is not currently compile-clean.
* Phase 5C fix note: Repair rs/fq/src/tests/util.rs merge damage so ticket_store is exposed as a runnable Rust test.

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

### Current Rust test body
```rust
fn ticket_store() {
    let test_sni = ["example.com", "example.net", "test.example.com"];
    let test_alpn = ["hq05", "hq07", "hq09"];
    let test_version: [u32; 3] = [0x0000_0001, 0xFF00_0020, 0x0000_0002];
    const TICKET_TIME: u64 = 40_000_000_000;
    const CURRENT_TIME: u64 = 50_000_000_000;
    const RETRIEVE_TIME: u64 = 60_000_000_000;
    const TOO_LATE_TIME: u64 = 150_000_000_000;
    const TTL: u32 = 100_000;
    let ticket_file =
        std::env::temp_dir().join(format!("fq_ticket_store_test_{}.bin", std::process::id()));
    let ticket_file_name = ticket_file.to_string_lossy().into_owned();

    let mut t = Instant::from_ticks(CURRENT_TIME);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    ctx.qclient
        .save_tickets(t, &ticket_file)
        .expect("save_empty_tickets");

    let mut retrieve_time = Instant::from_ticks(RETRIEVE_TIME);
    let mut empty_ctx = tls_api_init_ctx(&mut retrieve_time, 0, None).expect("empty ctx");
    empty_ctx
        .qclient
        .load_tickets(&ticket_file)
        .expect("load_empty_tickets");
    assert!(empty_ctx.qclient.stored_tickets.is_empty());

    let tp = TransportParameters {
        initial_max_stream_data_bidi_local: 123,
        initial_max_stream_data_bidi_remote: 456,
        initial_max_stream_data_uni: 78,
        initial_max_data: 91_011,
        initial_max_stream_id_bidir: 1_234,
        initial_max_stream_id_unidir: 567,
        ..TransportParameters::default()
    };

    let mut expected_tickets = Vec::new();

    for (i, &sni) in test_sni.iter().enumerate() {
        for (j, &alpn) in test_alpn.iter().enumerate() {
            let ticket_length = 64 + j * test_sni.len() + i;
            let delta_factor = i * test_alpn.len() + j;
            let test_ticket_time = TICKET_TIME / 1000 + 1000 * delta_factor as u64;
            let ticket = create_test_ticket(test_ticket_time, TTL, ticket_length);
            let (ip_addr, ip_addr_client) = test_ticket_addrs(i);
            let version = test_version[j];

            ctx.qclient
                .store_ticket(
                    Some(sni),
                    Some(alpn),
                    version,
                    ip_addr,
                    ip_addr_client,
                    &ticket,
                    &tp,
                )
                .expect("store_ticket");

            expected_tickets.push(ExpectedTicket {
                sni,
                alpn,
                version,
                ticket,
                ip_addr,
                ip_addr_client,
            });
        }
    }

    assert_eq!(ctx.qclient.stored_tickets.len(), expected_tickets.len());

    for expected in &expected_tickets {
        let (ticket, stored_tp) = ctx
            .qclient
            .get_ticket(
                Some(expected.sni),
                Some(expected.alpn),
                expected.version,
                false,
            )
            .expect("get_ticket");
        assert_eq!(ticket.len(), expected.ticket.len());
        assert_eq!(ticket, expected.ticket.as_slice());
        assert_ticket_tp(&stored_tp, &tp);
    }

    for stored in &ctx.qclient.stored_tickets {
        let expected = expected_tickets
            .iter()
            .find(|expected| {
                Some(expected.sni) == stored.sni.as_deref()
                    && Some(expected.alpn) == stored.alpn.as_deref()
                    && expected.version == stored.version
            })
            .expect("stored ticket key");
        assert_eq!(stored.ticket, expected.ticket);
        assert_eq!(stored.ip_addr, expected.ip_addr);
        assert_eq!(stored.ip_addr_client, expected.ip_addr_client);
    }

    let before_save = ticket_snapshots(&ctx.qclient.stored_tickets);

    ctx.qclient
        .save_tickets(Instant::from_ticks(CURRENT_TIME), &ticket_file)
        .expect("save_tickets");

    let mut retrieve_time = Instant::from_ticks(RETRIEVE_TIME);
    let mut loaded_ctx = tls_api_init_ctx(&mut retrieve_time, 0, None).expect("loaded ctx");
    loaded_ctx
        .qclient
        .load_tickets(&ticket_file)
        .expect("load_tickets");
    let after_load = ticket_snapshots(&loaded_ctx.qclient.stored_tickets);
    assert_eq!(after_load, before_save);

    let mut too_late = Instant::from_ticks(TOO_LATE_TIME);
    let ctx_too_late =
        tls_api_init_ctx(&mut too_late, 0, Some(&ticket_file_name)).expect("ctx too late");
    assert!(ctx_too_late.qclient.stored_tickets.is_empty());
}
```
