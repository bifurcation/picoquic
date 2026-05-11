# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/satellite_test.c:satellite_loss_test`
* C test-table name: `satellite_loss`
* C entry function: `satellite_loss_test`
* Rust test: `satellite_loss`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:256-271`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper passes the same BBR, bandwidth, latency, loss, and flag parameters, but the shared Rust scenario verifier currently ignores max_completion_microsec, so it does not enforce the C test's completion-time requirement for the lossy satellite transfer.
* Phase 5A fix note: In 5B, make the Rust scenario verification enforce the C completion bound, ideally in tls_api_one_scenario_body_verify, and verify the stream scenario before close so satellite_loss checks the 10MB lossy transfer under 8_000_000 usec.
* Phase 5B analysis: Rust satellite_loss is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract: BBR, 100000000 bytes, 8000000 usec completion cap, 250/3 Mbps satellite link, loss enabled, other toggles disabled. The mark_active_stream/PrepareToSend failure is a Phase 5C implementation/runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Should be less than 10 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 8000000, 250, 3, 0, 1, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_loss() {
    let bbr = satellite_ccalgo("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        8_000_000,
        250,
        3,
        0,
        true,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/skip_frame_test.c:dataqueue_packet_test`
* C test-table name: `dataqueue_packet`
* C entry function: `dataqueue_packet_test`
* Rust test: `dataqueue_packet`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:4032-4062`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only checks queue/dequeue flag toggling on one packet. C initializes stream 0, queues a 256-byte repeat packet, calls copy_stream_frames_for_retransmit four times with buffer sizes 2/128/1024/1024 and expected data/more_data/pure_ack states, then exercises dequeue on another prepared packet.
* Phase 5A fix note: Expand Rust dataqueue_packet to initialize stream 0, queue the prepared packet, run the four buffer-size iterations with the C expected flags, and keep the final dequeue coverage.
* Phase 5B analysis: Rust #[test] is present and expresses the C API-level contract: stream 0 initialization, prepared 256-byte repeat packet queueing, four copy_stream_frames_for_retransmit buffer/flag cases, and dequeue coverage. The current undersized-buffer None result is a Phase 5C implementation failure, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    picoquic_quic_t* qtest = NULL;
    picoquic_cnx_t* cnx = NULL;
    int ret = 0;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;

    memset(&saddr, 0, sizeof(struct sockaddr_in));

    /* Initialize the connection context */
    if (ret == 0) {
        qtest = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
            NULL, NULL, NULL, NULL, simulated_time,
            &simulated_time, NULL, NULL, 0);
        if (qtest == NULL) {
            DBG_PRINTF("%s", "Cannot create QUIC context\n");
            ret = -1;
        }
        else {
            cnx = picoquic_create_cnx(qtest,
                picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr*) & saddr,
                simulated_time, 0, "test-sni", "test-alpn", 1);

            if (cnx == NULL) {
                ret = -1;
            }
            else {
                /* Create stream 0 so the later tests succeed */
                uint8_t new_bytes[256];
                memset(new_bytes, 0, sizeof(new_bytes));
                if ((ret = picoquic_add_to_stream(cnx, 0, new_bytes, sizeof(new_bytes), 0)) != 0) {
                    DBG_PRINTF("%s", "Cannot initialize stream 0\n");
                    ret = -1;
                }
            }
        }
    }

    if (ret == 0) {
        /* Create a packet and chain it to the data queue */
        picoquic_packet_t* packet = picoquic_create_packet(qtest);
        if (packet == NULL) {
            ret = -1;
        }
        else {
            (void)dataqueue_prepare_packet(packet, 1, 0, 0, 0, 256);
            picoquic_queue_data_repeat_packet(cnx, packet);
        }
    }

    if (ret == 0 &&
        (ret = dataqueue_packet_test_iterate(1, cnx, 2, 0, 1, 1)) == 0 &&
        (ret = dataqueue_packet_test_iterate(2, cnx, 128, 0, 1, 1)) == 0 &&
        (ret = dataqueue_packet_test_iterate(3, cnx, 1024, 1, 0, 0)) == 0) {
        ret = dataqueue_packet_test_iterate(4, cnx, 1024, 0, 0, 1);
    }

    if (ret == 0) {
        /* Create a packet and chain it to the data queue */
        picoquic_packet_t* packet = picoquic_create_packet(qtest);
        if (packet == NULL) {
            ret = -1;
        }
        else {
            (void)dataqueue_prepare_packet(packet, 1, 0, 0, 0, 256);
            picoquic_dequeue_data_repeat_packet(cnx, packet);
        }
    }

    /* Free the connection context */
    if (cnx != NULL) {
        picoquic_delete_cnx(cnx);
    }

    if (qtest != NULL) {
        picoquic_free(qtest);
    }
    return ret;
}
```

### Current Rust test body
```rust
fn dataqueue_packet() {
    const DATAQUEUE_PACKET_SEQUENCE: u64 = 0x5a5a_0000;

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut cnx = quic.create_test_cnx(&mut simulated_time).expect("cnx");
    let new_bytes = [0u8; 256];

    cnx.add_to_stream(0, &new_bytes, false)
        .expect("initialize stream 0");
    let _packet = dataqueue_queue_prepared_packet(&mut quic, &mut cnx, DATAQUEUE_PACKET_SEQUENCE)
        .expect("queue data-repeat packet");

    dataqueue_packet_test_iterate(&mut cnx, 1, 2, false, true, true)
        .expect("dataqueue_packet iteration 1");
    dataqueue_packet_test_iterate(&mut cnx, 2, 128, false, true, true)
        .expect("dataqueue_packet iteration 2");
    dataqueue_packet_test_iterate(&mut cnx, 3, 1024, true, false, false)
        .expect("dataqueue_packet iteration 3");
    dataqueue_packet_test_iterate(&mut cnx, 4, 1024, false, false, true)
        .expect("dataqueue_packet iteration 4");

    let mut packet =
        dataqueue_queue_prepared_packet(&mut quic, &mut cnx, DATAQUEUE_PACKET_SEQUENCE + 1)
            .expect("queue packet for dequeue coverage");
    cnx.dequeue_data_repeat_packet(&mut packet);
    assert!(
        !packet.is_queued_for_data_repeat,
        "dequeued data-repeat packet remains queued"
    );
}
```

## `picoquictest/sockloop_test.c:sockloop_thread_name_test`
* C test-table name: `sockloop_thread_name`
* C entry function: `sockloop_thread_name_test`
* Rust test: `sockloop_thread_name`
* Expected Rust file: `rs/fq/src/tests/sockloop.rs`
* Current Rust span: `rs/fq/src/tests/sockloop.rs:1041-1048`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test sets the same visible spec fields and thread name, but the shared Rust sockloop helper ignores spec.scenario, lacks the C scenario initialization/verification, and the background-thread branch can return Ok without proving the 1MB transfer completed.
* Phase 5A fix note: Make sockloop_test_one_result faithfully initialize and verify the scenario, and make the background-thread path run the packet loop and fail if the ready/transfer completion conditions are not met while preserving thread_name propagation.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, and matches the C API-level contract: spec id 8, 0xffff socket buffer, 1M scenario, background thread enabled, thread name set to "picoquic loop", and sockloop_test_one invoked. Any failure from incomplete background-thread packet-loop or wake-up behavior is Phase 5C runtime work, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 8);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.use_background_thread = 1;
    spec.thread_name = "picoquic loop";

    return(sockloop_test_one(&spec));
}
```

### Current Rust test body
```rust
fn sockloop_thread_name() {
    let mut spec = SockloopTestSpec::new(8);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.use_background_thread = true;
    spec.thread_name = Some("picoquic loop");
    sockloop_test_one(&spec);
}
```

## `picoquictest/ticket_store_test.c:token_store_test`
* C test-table name: `token_store`
* C entry function: `token_store_test`
* Rust test: `token_store`
* Expected Rust file: `rs/fq/src/tests/ticket_store.rs`
* Current Rust span: `rs/fq/src/tests/ticket_store.rs:396-499`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is only a smoke test for storing/loading fixed 32-byte tokens. It omits the empty-file reload check, variable token lengths and TTL-bearing token bodies, length verification, full stored-list comparison after reload, and expiration check at the late time.
* Phase 5A fix note: Rebuild the Rust test around the C flow: save/load an empty token file and assert empty state, create the 3 SNI x 4 IP token matrix with C-equivalent lengths/content/times, verify retrieved lengths/bytes, compare saved and reloaded stored-token contents, and assert late reload drops expired tokens.
* Phase 5B analysis: Rust token_store is present as a #[test], compiles under the Rust test harness, and expresses the C API-level contract: empty token save/load, 3 SNI x 4 IP token storage, retrieval checks, saved/reloaded token snapshot comparison, and late-expiry assertion. The known late reload failure is an implementation/runtime issue for Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

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

### Current Rust test body
```rust
fn token_store() {
    let test_sni = ["example.com", "example.net", "test.example.com"];
    let test_ips: [IpAddr; 4] = [
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(128, 12, 34, 56)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        IpAddr::V6(Ipv6Addr::from([
            0x20, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 1,
        ])),
    ];
    const TOKEN_TIME: u64 = 40_000_000_000;
    const CURRENT_TIME: u64 = 50_000_000_000;
    const RETRIEVE_TIME: u64 = 60_000_000_000;
    const TOO_LATE_TIME: u64 = 150_000_000_000;
    const TTL: u32 = 100_000;
    let token_file =
        std::env::temp_dir().join(format!("fq_token_store_test_{}.bin", std::process::id()));

    let mut t = Instant::from_ticks(CURRENT_TIME);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    ctx.qclient
        .save_tokens(&token_file)
        .expect("save_empty_tokens");

    let mut retrieve_time = Instant::from_ticks(RETRIEVE_TIME);
    let mut empty_ctx = tls_api_init_ctx(&mut retrieve_time, 0, None).expect("empty ctx");
    empty_ctx
        .qclient
        .load_tokens(&token_file)
        .expect("load_empty_tokens");
    assert!(empty_ctx.qclient.stored_tokens.is_empty());

    let mut expected_tokens = Vec::new();

    for (i, &sni) in test_sni.iter().enumerate() {
        for (j, &ip_addr) in test_ips.iter().enumerate() {
            let token_length = 64 + j * test_sni.len() + i;
            let delta_factor = i * test_ips.len() + j;
            let test_token_time = TOKEN_TIME / 1000 + 1000 * delta_factor as u64;
            let token = create_test_token(test_token_time, TTL, token_length);

            ctx.qclient
                .store_token(Some(sni), ip_addr, &token)
                .expect("store_token");

            expected_tokens.push(ExpectedToken {
                sni,
                ip_addr,
                token,
            });
        }
    }

    assert_eq!(ctx.qclient.stored_tokens.len(), expected_tokens.len());

    for expected in &expected_tokens {
        let token = ctx
            .qclient
            .get_token(Some(expected.sni), expected.ip_addr, false)
            .unwrap_or_else(|err| {
                panic!(
                    "get_token sni={} ip={:?}: {err:?}",
                    expected.sni, expected.ip_addr
                )
            });
        assert_eq!(token.len(), expected.token.len());
        assert_eq!(token, expected.token.as_slice());
    }

    for stored in &ctx.qclient.stored_tokens {
        let expected = expected_tokens
            .iter()
            .find(|expected| {
                Some(expected.sni) == stored.sni.as_deref() && expected.ip_addr == stored.ip_addr
            })
            .expect("stored token key");
        assert_eq!(stored.token, expected.token);
    }

    let before_save = token_snapshots(&ctx.qclient.stored_tokens);

    ctx.qclient.save_tokens(&token_file).expect("save_tokens");

    let mut retrieve_time = Instant::from_ticks(RETRIEVE_TIME);
    let mut loaded_ctx = tls_api_init_ctx(&mut retrieve_time, 0, None).expect("loaded ctx");
    loaded_ctx
        .qclient
        .load_tokens(&token_file)
        .expect("load_tokens");
    let after_load = token_snapshots(&loaded_ctx.qclient.stored_tokens);
    assert_eq!(after_load, before_save);

    let mut too_late = Instant::from_ticks(TOO_LATE_TIME);
    let mut late_ctx = tls_api_init_ctx(&mut too_late, 0, None).expect("late ctx");
    late_ctx
        .qclient
        .load_tokens(&token_file)
        .expect("load_tokens_too_late");
    assert!(
        late_ctx.qclient.stored_tokens.is_empty(),
        "expired tokens should be dropped during late reload"
    );
}
```

## `picoquictest/tls_api_test.c:cid_length_test`
* C test-table name: `cid_length`
* C entry function: `cid_length_test`
* Rust test: `cid_length`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1379-1387`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust iterates 0..20, which includes 8 and misses the C case 20. The helper also does not mirror C's recreate-after-setting-length flow or assert the client/server CID lengths after handshake.
* Phase 5A fix note: Use the exact C length table [0,1,2,3,4,5,6,7,9,10,11,12,13,14,15,16,17,18,19,20], and update cid_length_test_one to set the length before the tested connection starts/recreate it as C does, then verify both endpoint CID lengths.
* Phase 5B analysis: Rust test and helper already express the C API-level contract: exact CID-length table, client connection recreation after setting default CID length, q-and-r scenario execution, and client-local/server-remote CID length assertions. The length-0 runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

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

### Current Rust test body
```rust
fn cid_length() {
    const TESTED_LENGTHS: &[u32] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ];

    for &len in TESTED_LENGTHS {
        cid_length_test_one(len).unwrap_or_else(|e| panic!("cid_length({len}): {e:?}"));
    }
}
```
