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

## `picoquictest/satellite_test.c:satellite_small_up_test`
* C test-table name: `satellite_small_up`
* C entry function: `satellite_small_up_test`
* Rust test: `satellite_small_up`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:370-385`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The visible call arguments match C, but the same Rust helper issue means the intended 100MB BBR transfer over the small upstream path is not actually queued or verified, and the 400s completion bound is ignored.
* Phase 5A fix note: Map data_size to stream0_target, ensure the stream0 transfer is initialized and verified, and enforce the max_completion_time bound for this no-loss BBR case.
* Phase 5B analysis: Rust test matches the C entry parameters and helper-level API contract; any early mark_active_stream/PrepareToSend failure is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Should be less than 420 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 400000000, 2, 10, 0, 0, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_small_up() {
    let bbr = satellite_ccalgo("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        400_000_000,
        2,
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

## `picoquictest/sockloop_test.c:sockloop_nat_test`
* C test-table name: `sockloop_nat`
* C entry function: `sockloop_nat_test`
* Rust test: `sockloop_nat`
* Expected Rust file: `rs/fq/src/tests/sockloop.rs`
* Current Rust span: `rs/fq/src/tests/sockloop.rs:1018-1027`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust sets the same spec fields, but sockloop_test_one_result never programs or verifies spec.scenario and its received-finished check only requires ready/established. The C test sends the 1M scenario, verifies completion, and validates NAT migration.
* Phase 5A fix note: Make the Rust sockloop driver initialize and verify the scenario streams like C, then keep the IPv4 extra-socket/prefer-extra-socket force_migration=1 NAT checks.
* Phase 5B analysis: Rust #[test] matches the C API-level contract for sockloop_nat: spec id 6, IPv4, 0xffff socket buffer, 1M scenario, extra/preferred extra socket, and force_migration=1. Any early socket-loop/runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 6);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.extra_socket_required = 1;
    spec.prefer_extra_socket = 1;
    spec.force_migration = 1;

    return(sockloop_test_one(&spec));
}
```

### Current Rust test body
```rust
fn sockloop_nat() {
    let mut spec = SockloopTestSpec::new(6);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.extra_socket_required = true;
    spec.prefer_extra_socket = true;
    spec.force_migration = 1;
    sockloop_test_one(&spec);
}
```

## `picoquictest/ticket_store_test.c:ticket_store_test`
* C test-table name: `ticket_store`
* C entry function: `ticket_store_test`
* Rust test: `ticket_store`
* Expected Rust file: `rs/fq/src/tests/ticket_store.rs`
* Current Rust span: `rs/fq/src/tests/ticket_store.rs:174-296`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is materially weaker and different: it skips empty-file load verification, uses a different ALPN set and a 3x3x3 version cartesian product, uses fixed 48-byte tickets/default TP/zero IPs, does not check expected ticket lengths or full saved-vs-loaded content, and does not verify expired reload produces no tickets.
* Phase 5A fix note: Mirror the C matrix and data: empty save/load check, three SNI by three ALPN/version pairs, create_test_ticket-style variable lengths and TTL, IPv4/IPv6 address variants, non-default TP comparison, retrieval length checks, full round-trip content comparison, and too-late reload expiry check.
* Phase 5B analysis: Current Rust test is registered, compiles under the Rust test harness, and mirrors the C API-level save/load/store/get matrix and assertions. Late reload expiry behavior and null-IP representation fidelity are Phase 5C/API implementation notes, not Phase 5B blockers.
* Phase 5B fix note: 

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

## `picoquictest/tls_api_test.c:bad_coalesce_test`
* C test-table name: `bad_coalesce`
* C entry function: `bad_coalesce_test`
* Rust test: `bad_coalesce`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1339-1345`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C sets do_bad_coalesce_test so client departure prepends a malformed coalesced packet, then runs test_scenario_q_and_r with a 250,000 us target. Rust does not set or implement that flag/path, passes an empty scenario, and uses a 2,000,000 us target.
* Phase 5A fix note: Add/use a Rust bad-coalesce test context flag and client-departure injection equivalent, run the q_and_r scenario {stream_id 4, q_len 257, r_len 2000}, and use the C completion target of 250,000 us.
* Phase 5B analysis: Rust test matches the C API-level contract: initialize context, set bad-coalesce mode, run Q_AND_R with zero options and 250000 us target. Runtime callback/accounting failures are Phase 5C, not Phase 5B blocks.
* Phase 5B fix note: 

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

### Current Rust test body
```rust
fn bad_coalesce() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    ctx.do_bad_coalesce_test = true;
    tls_api_one_scenario_body(&mut ctx, &mut t, TEST_SCENARIO_Q_AND_R, 0, 0, 0, 0, 250_000)
        .expect("bad_coalesce");
}
```

## `picoquictest/tls_api_test.c:tls_api_client_losses_test`
* C test-table name: `client_losses`
* C entry function: `tls_api_client_losses_test`
* Rust test: `client_losses`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1535-1537`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test passes mask 3, matching the C entry, but Rust tls_api_loss_test ignores its loss mask and always runs the handshake with zero loss.
* Phase 5A fix note: Thread the provided loss mask into tls_api_test_with_loss/tls_api_connection_loop so client_losses actually drops packets 1 and 2 and verifies recovery.
* Phase 5B analysis: Current Rust helper threads the loss mask from tls_api_loss_test into tls_api_connection_loop and simulated packet submit/admit paths, so client_losses now exercises mask 3 like the C test.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_loss_test(3ull);
}
```

### Current Rust test body
```rust
fn client_losses() {
    tls_api_loss_test(3).expect("client_losses");
}
```
