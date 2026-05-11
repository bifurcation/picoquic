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

## `picoquictest/tls_api_test.c:mtu_drop_bbr_test`
* C test-table name: `mtu_drop_bbr`
* C entry function: `mtu_drop_bbr_test`
* Rust test: `mtu_drop_bbr`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4370-4372`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper passes the same nominal target, but its helper ignores the congestion algorithm/target timing and omits the C test's long-latency 1Mbps setup, MTU-discovery check, path-MTU drop, and completion-time assertion.
* Phase 5A fix note: Repair mtu_drop_cc_algotest for BBR: apply the selected CC algorithm, match the C initial CID/link latency/bandwidth setup, wait for MTU discovery, assert expected MTUs, lower both link path MTUs, complete the very-long transfer, and enforce target_time.
* Phase 5B analysis: Rust test is present as a #[test], compiles under cargo check --tests, and calls the MTU-drop helper with BBR and target_time 10700000 matching the C entry. Any early handshake/server-creation failure is a Phase 5C runtime-library issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* TODO: the time with BBR v1 was 10300000. The current value is
     * a slight regression. Investigate whether some performance
     * for BBR3 "recover from PTO" could be improved. */
    int ret = mtu_drop_cc_algotest(picoquic_bbr_algorithm, 10700000);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_drop_bbr() {
    mtu_drop_cc_algotest("bbr", 10_700_000).expect("mtu_drop_bbr");
}
```

## `picoquictest/tls_api_test.c:tls_api_multiple_versions_test`
* C test-table name: `multiple_versions`
* C entry function: `tls_api_multiple_versions_test`
* Rust test: `multiple_versions`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4560-4576`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C iterates every supported version except index 0 and runs q_and_r data scenario for each; Rust hard-codes three versions, includes V1 which C skips, includes 0xFF000013 which is not in the current supported list, omits most supported versions, and only runs generic handshake/close.
* Phase 5A fix note: Iterate Rust SUPPORTED_VERSIONS from index 1 onward and run the q_and_r scenario body for each version, matching tls_api_one_scenario_test semantics rather than tls_api_test_with_loss.
* Phase 5B analysis: Rust multiple_versions is a #[test] that iterates SUPPORTED_VERSIONS from index 1, uses the q-and-r scenario {4,0,257,2000}, initializes a context with each version code, and calls tls_api_one_scenario_body with matching zero arguments. Any QUIC v2/q_and_r runtime failure is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    for (size_t i = 1; ret == 0 && i < picoquic_nb_supported_versions; i++) {
        ret = tls_api_one_scenario_test(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0,
            picoquic_supported_versions[i].version, 0, NULL, NULL);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn multiple_versions() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2_000,
    }];

    for version in SUPPORTED_VERSIONS.iter().skip(1).copied() {
        let version_code = version as u32;
        let mut t = Instant::from_ticks(0);
        let mut ctx = tls_api_init_ctx(&mut t, version_code, None)
            .unwrap_or_else(|| panic!("multiple_versions ver={version_code:#x}: ctx"));
        tls_api_one_scenario_body(&mut ctx, &mut t, &scenario, 0, 0, 0, 0, 0)
            .unwrap_or_else(|e| panic!("multiple_versions ver={version_code:#x}: {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:new_rotated_key_test`
* C test-table name: `new_rotated_key`
* C entry function: `new_rotated_key_test`
* Rust test: `new_rotated_key`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5197-5199`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic handshake/close. C waits for application AEAD readiness, computes new rotated keys on server and client for three rounds, compares secret sizes and cross-direction app secrets, checks AEAD compatibility hooks, and frees the new crypto contexts each round.
* Phase 5A fix note: Add a Rust key-rotation test that drives handshake and AEAD readiness, loops three rotations on both endpoints, compares server enc to client dec and server dec to client enc secrets, checks new AEAD availability/compatibility where exposed, and clears crypto_context_new between rounds.
* Phase 5B analysis: Rust test already matches the C API-level contract: it initializes the TLS API context, runs the connection loop, waits for application AEAD readiness, performs three server/client rotated-key rounds, compares app-secret sizes and cross-direction secrets, checks AEAD encrypt/decrypt compatibility, and clears crypto_context_new each round. The known missing accepted server connection after the loop is a Phase 5C runtime/library issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_application_aead_ready(test_ctx, &simulated_time);
    }


    for (int i = 1; ret == 0 && i <= 3; i++) {
        
        /* Try to compute rotated keys on server */
        ret = picoquic_compute_new_rotated_keys(test_ctx->cnx_server);
        if (ret != 0) {
            DBG_PRINTF("Could not rotate server key, ret: %x\n", ret);
        } else {
            /* Try to compute rotated keys on client */
            ret = picoquic_compute_new_rotated_keys(test_ctx->cnx_client);
            if (ret != 0) {
                DBG_PRINTF("Could not rotate client key, round %d, ret: %x\n", i, ret);
            }
        }

        if (ret == 0)
        {
            /* Compare server encryption and client decryption */
            size_t key_size = picoquic_get_app_secret_size(test_ctx->cnx_client);

            if (key_size != picoquic_get_app_secret_size(test_ctx->cnx_server)) {
                DBG_PRINTF("Round %d. Key sizes dont match, client: %d, server: %d\n", i, key_size, picoquic_get_app_secret_size(test_ctx->cnx_server));
                ret = -1;
            }
            else if (memcmp(picoquic_get_app_secret(test_ctx->cnx_server, 1), picoquic_get_app_secret(test_ctx->cnx_client, 0), key_size) != 0) {
                DBG_PRINTF("Round %d. Server encryption secret does not match client decryption secret\n", i);
                ret = -1;
            }
            else if (memcmp(picoquic_get_app_secret(test_ctx->cnx_server, 0), picoquic_get_app_secret(test_ctx->cnx_client, 1), key_size) != 0) {
                DBG_PRINTF("Round %d. Server decryption secret does not match client encryption secret\n", i);
                ret = -1;
            }
            else if (aead_iv_check(test_ctx->cnx_server->crypto_context_new.aead_encrypt, test_ctx->cnx_client->crypto_context_new.aead_decrypt) != 0) {
                DBG_PRINTF("Round %d. Client AEAD decryption does not match server AEAD encryption.\n", i);
                ret = -1;
            }
            else if (aead_iv_check(test_ctx->cnx_client->crypto_context_new.aead_encrypt, test_ctx->cnx_server->crypto_context_new.aead_decrypt) != 0) {
                DBG_PRINTF("Round %d. Server AEAD decryption does not match cliens AEAD encryption.\n", i);
                ret = -1;
            }
#if 0
            else if (pn_enc_check(test_ctx->cnx_server->crypto_context_new.pn_enc, test_ctx->cnx_client->crypto_context_new.pn_dec) != 0) {
                DBG_PRINTF("Round %d. Client PN decryption does not match server PN encryption.\n", i);
                ret = -1;
            }
            else if (pn_enc_check(test_ctx->cnx_client->crypto_context_new.pn_enc, test_ctx->cnx_server->crypto_context_new.pn_dec) != 0) {
                DBG_PRINTF("Round %d. Server PN decryption does not match client PN encryption.\n", i);
                ret = -1;
            }
#endif
        }

        picoquic_crypto_context_free(&test_ctx->cnx_server->crypto_context_new);
        picoquic_crypto_context_free(&test_ctx->cnx_client->crypto_context_new);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn new_rotated_key() {
    new_rotated_key_impl().expect("new_rotated_key");
}
```

## `picoquictest/tls_api_test.c:padding_test`
* C test-table name: `padding_test`
* C entry function: `padding_test`
* Rust test: `padding_test`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5894-5896`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust forwards the same numeric parameters but the helper ignores them and only performs a generic handshake/close; it omits the packet-size padding checks across the C test sizes.
* Phase 5A fix note: Implement `padding_test_one` to apply server/client padding policy, queue padding+PING frames for the C size table, inspect outbound packet lengths, enforce min-size/multiple/no-overpadding rules, and require each queued packet to complete.
* Phase 5B analysis: Rust #[test] directly calls padding_test_one(128, 64), matching the C entry body. The helper expresses the C API-level contract: server/client padding policy, same probe-size table, PADDING+PING misc frames, packet-length checks, and queued-packet completion. The 1023-byte drain failure is a Phase 5C runtime/library issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return padding_test_one(128, 64);
}
```

### Current Rust test body
```rust
fn padding_test() {
    padding_test_one(128, 64).expect("padding_test");
}
```

## `picoquictest/tls_api_test.c:probe_api_test`
* C test-table name: `probe_api`
* C entry function: `probe_api_test`
* Rust test: `probe_api`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6388-6460`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is only a generic handshake/close test. It does not create synthetic IPv4/IPv6 probe addresses, require enough CIDs, exercise probe_new_path repeatedly, assert the final probe fails at capacity, or seed path challenges.
* Phase 5A fix note: Implement probe_api to mirror C: build synthetic v4/v6 address arrays, synchronize to NB_PATH_TARGET CIDs, call probe_new_path for alternating address families until path capacity, assert trials before the limit succeed and the limit trial fails, and populate challenge values for successful paths.
* Phase 5B analysis: Rust #[test] is present, compiles, and mirrors the C probe_api API contract: setup/sync, CID capacity checks, alternating IPv4/IPv6 probe_new_path calls, expected success until path capacity, final failure, and challenge seeding. Current CID/probe_new_path runtime failures are Phase 5C implementation issues, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    struct sockaddr_in t4[PICOQUIC_NB_PATH_TARGET];
    struct sockaddr_in6 t6[PICOQUIC_NB_PATH_TARGET];
    int nb_trials;

    /* Initialize the test addresses to synthetic values */
    for (int i = 0; i < PICOQUIC_NB_PATH_TARGET; i++) {
        memset(&t4[i], 0, sizeof(struct sockaddr_in));
        t4[i].sin_family = AF_INET;
        t4[i].sin_port = 1000+i;
        memset(&t4[i].sin_addr, i, 4);
        memset(&t6[i], 0, sizeof(struct sockaddr_in6));
        t6[i].sin6_family = AF_INET6;
        t6[i].sin6_port = 2000 + i;
        memset(&t6[i].sin6_addr, i, 16);
    }

    /* Set a test connection between client and server */
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* run a receive loop until no outstanding data */
    if (ret == 0) {
        ret = tls_api_synch_to_empty_loop(test_ctx, &simulated_time, 2048, PICOQUIC_NB_PATH_TARGET, 0);
    }

    if (ret == 0) {
        if (test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid < PICOQUIC_NB_PATH_TARGET) {
            DBG_PRINTF("Only %d CID created on client.\n", test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid);
            ret = -1;
        }
        else if (test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid < PICOQUIC_NB_PATH_TARGET) {
            DBG_PRINTF("Only %d CID created on server.\n", test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid);
        }
    }

    /* Now, create a series of probes.
     * There are only PICOQUIC_NB_PATH_TARGET - 1 paths available. 
     * The last trial should fail.
     */
    nb_trials = 0;

    for (int i = 1; ret == 0 && test_ctx->cnx_client->nb_paths < PICOQUIC_NB_PATH_TARGET && i < PICOQUIC_NB_PATH_TARGET; i++) {
        for (int j = 0; ret == 0 && j < 2; j++) {
            int ret_probe;
            if (j == 0) {
                ret_probe = picoquic_probe_new_path(test_ctx->cnx_client, (struct sockaddr *) &t4[0], 
                    (struct sockaddr *) &t4[i], simulated_time);
            } else {
                ret_probe = picoquic_probe_new_path(test_ctx->cnx_client, (struct sockaddr *) &t6[0],
                    (struct sockaddr *) &t6[i], simulated_time);
            }

            nb_trials++;

            if (nb_trials < PICOQUIC_NB_PATH_TARGET) {
                if (ret_probe != 0) {
                    DBG_PRINTF("Trial %d (%d, %d) fails with ret = %x\n", nb_trials, i, j, ret_probe);
                    ret = -1;
                }
            }
            else if (ret_probe == 0) {
                DBG_PRINTF("Trial %d (%d, %d) succeeds (unexpected)\n", nb_trials, i, j);
                ret = -1;
            }

            if (ret == 0 && ret_probe == 0) {
                int path_id = test_ctx->cnx_client->nb_paths - 1;
                for (int ichal = 0; ichal < PICOQUIC_CHALLENGE_REPEAT_MAX; ichal++) {
                    test_ctx->cnx_client->path[path_id]->first_tuple->challenge[ichal] = (uint64_t)10000 + (uint64_t)10 * i + j + (uint64_t)1000*ichal;
                }
            }
        }
    }

    /* Releasing the context will test the delete functions. */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn probe_api() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let t4: [SocketAddr; NB_PATH_TARGET] = core::array::from_fn(|i| {
        SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(i as u8, i as u8, i as u8, i as u8)),
            1000u16 + i as u16,
        )
    });
    let t6: [SocketAddr; NB_PATH_TARGET] = core::array::from_fn(|i| {
        SocketAddr::new(
            IpAddr::V6(Ipv6Addr::from([i as u8; 16])),
            2000u16 + i as u16,
        )
    });
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("probe_api ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("probe_api connection");
    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        NB_PATH_TARGET as i32,
        0,
    )
    .expect("probe_api sync");

    let client_local_cid_count = first_local_cnxid_count(test_ctx.cnx_client());
    assert!(
        client_local_cid_count >= NB_PATH_TARGET,
        "Only {client_local_cid_count} CID created on client."
    );
    let server_local_cid_count = first_local_cnxid_count(test_ctx.cnx_server());
    assert!(
        server_local_cid_count >= NB_PATH_TARGET,
        "Only {server_local_cid_count} CID created on server."
    );

    let client = test_ctx.cnx_client();
    let mut nb_trials = 0usize;

    for i in 1..NB_PATH_TARGET {
        if client.nb_paths() >= NB_PATH_TARGET {
            break;
        }

        for j in 0..2 {
            let ret_probe = if j == 0 {
                client.probe_new_path(&t4[0], &t4[i], simulated_time)
            } else {
                client.probe_new_path(&t6[0], &t6[i], simulated_time)
            };
            nb_trials += 1;

            if nb_trials < NB_PATH_TARGET {
                assert!(
                    ret_probe.is_ok(),
                    "Trial {nb_trials} ({i}, {j}) fails with ret = {ret_probe:?}"
                );
            } else {
                assert!(
                    ret_probe.is_err(),
                    "Trial {nb_trials} ({i}, {j}) succeeds unexpectedly"
                );
            }

            if ret_probe.is_ok() {
                seed_probe_path_challenges(client, i, j);
            }
        }
    }
}
```
