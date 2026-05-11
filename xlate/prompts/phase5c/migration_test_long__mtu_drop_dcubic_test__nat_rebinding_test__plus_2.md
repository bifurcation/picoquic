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

## `picoquictest/tls_api_test.c:migration_test_long`
* C test-table name: `migration_long`
* C entry function: `migration_test_long`
* Rust test: `migration_long`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4296-4298`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes an empty scenario, whose helper fallback is not C's test_scenario_very_long, and the helper omits key C migration checks such as required probe success, challenge verification, scenario verification, and CID change assertions.
* Phase 5A fix note: Pass/define the C very-long scenario {stream_id:4, previous:0, q_len:257, r_len:1000000} and make migration_test_scenario enforce probe success, data completion, challenge renewal/verification, and expected CID changes.
* Phase 5B analysis: Rust `migration_long` is present as a runnable `#[test]`, compiles, and calls `migration_test_scenario(TEST_SCENARIO_VERY_LONG, 0, false)`, matching the C call with `test_scenario_very_long`, zero loss, and `cid_zero` false. The `probe_new_path` stub can cause runtime failure, but that is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return migration_test_scenario(test_scenario_very_long, sizeof(test_scenario_very_long), 0, 0);
}
```

### Current Rust test body
```rust
fn migration_long() {
    migration_test_scenario(TEST_SCENARIO_VERY_LONG, 0, false).expect("migration_long");
}
```

## `picoquictest/tls_api_test.c:mtu_drop_dcubic_test`
* C test-table name: `mtu_drop_dcubic`
* C entry function: `mtu_drop_dcubic_test`
* Rust test: `mtu_drop_dcubic`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4386-4388`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the dcubic label and target time, but the helper ignores the algorithm label, does not configure dcubic, does not set 100ms/1Mbps links, does not verify MTU discovery before dropping path MTU, does not drop path MTU, and the completion-time verifier currently ignores the target time.
* Phase 5A fix note: Implement mtu_drop_cc_algotest faithfully: resolve and set dcubic, use the C link characteristics and initial CID behavior, verify pre-drop MTUs, halve path MTUs, complete the long transfer, and enforce the 9200000us target.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, compiles, and calls the MTU-drop helper with the C dcubic target and 9_200_000 completion time. The helper expresses the API-visible setup, MTU prechecks, MTU drop, data loop, and completion verification. Any early failure from missing Rust TLS/Initial handshake behavior is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_dcubic_algorithm, 9200000);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_drop_dcubic() {
    mtu_drop_cc_algotest("dcubic", 9_200_000).expect("mtu_drop_dcubic");
}
```

## `picoquictest/tls_api_test.c:nat_rebinding_test`
* C test-table name: `nat_rebinding`
* C entry function: `nat_rebinding_test`
* Rust test: `nat_rebinding`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4706-4708`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust helper accepts the same arguments but does not simulate NAT rebinding: it never switches the client address/port, does not capture or verify path challenge renewal, ignores cid_zero behavior, and omits the post-transfer challenge/stash checks.
* Phase 5A fix note: Implement nat_rebinding_test_one with explicit initial CID variants, sync-to-empty with the path target, change the client NAT address before data, use the q_and_r scenario, then loop and assert the server challenge changed and was verified, including the zero-CID/loss/latency cases.
* Phase 5B analysis: Rust #[test] nat_rebinding calls nat_rebinding_test_one(0, false, 0), matching the C entry's loss_mask=0, zero_cid=0, latency=0. The helper expresses the API-visible setup, NAT port change, q-and-r transfer, scenario verification, post-rebinding challenge checks, and close path. Prior packet-preparation/path-challenge failure is Phase 5C runtime library behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t loss_mask = 0;

    return nat_rebinding_test_one(loss_mask, 0, 0);
}
```

### Current Rust test body
```rust
fn nat_rebinding() {
    nat_rebinding_test_one(0, false, 0).expect("nat_rebinding");
}
```

## `picoquictest/tls_api_test.c:not_before_cnxid_test`
* C test-table name: `not_before_cnxid`
* C entry function: `not_before_cnxid_test`
* Rust test: `not_before_cnxid`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5421-5423`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the generic handshake/close helper; it never fills CID stashes, calls remove_not_before_cid, waits for CID replacement/backlog empty, or checks local CID counts and stash correspondence.
* Phase 5A fix note: Add a not_before_cnxid test/helper that syncs to PICOQUIC_NB_PATH_TARGET/NB_PATH_TARGET CIDs, calls remove_not_before_cid with sequence_next-1, drives the replacement loop, verifies server local CID count, and compares both peers' stashed CIDs to the peer local list.
* Phase 5B analysis: Rust test already expresses the C API-level contract and compiles/runs under the Rust test harness. Any failure to reach server CID state is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t not_before;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* run a receive loop until no outstanding data */
    if (ret == 0) {
        ret = tls_api_synch_to_empty_loop(test_ctx, &simulated_time, 2048, PICOQUIC_NB_PATH_TARGET, 0);
    }

    /* find a plausible "not before" value, and apply it */
    if (ret == 0) {
        not_before = test_ctx->cnx_server->first_local_cnxid_list->local_cnxid_sequence_next - 1;
        uint64_t transport_error = picoquic_remove_not_before_cid(test_ctx->cnx_client, 0, not_before, simulated_time);
        if (transport_error != 0) {
            DBG_PRINTF("picoquic_remove_not_before_cid returns 0x%" PRIx64, transport_error);
            ret = -1;
        }
    }

    /* run the loop again until no outstanding data */
    if (ret == 0) {
        uint64_t time_out = simulated_time + 8000000;
        int nb_rounds = 0;
        int success = 0;

        while (ret == 0 && simulated_time < time_out &&
            nb_rounds < 2048 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
            int was_active = 0;

            ret = tls_api_one_sim_round(test_ctx, &simulated_time, time_out, &was_active);
            nb_rounds++;

            if (nb_rounds == 30) {
                ret = 0;
            }

            if (test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid >= PICOQUIC_NB_PATH_TARGET &&
                test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid >= PICOQUIC_NB_PATH_TARGET &&
                test_ctx->cnx_client->first_misc_frame == NULL &&
                test_cnxid_count_stash(test_ctx->cnx_client) >= (PICOQUIC_NB_PATH_TARGET - 1) &&
                test_cnxid_count_stash(test_ctx->cnx_server) >= (PICOQUIC_NB_PATH_TARGET - 1) &&
                picoquic_is_cnx_backlog_empty(test_ctx->cnx_client) &&
                picoquic_is_cnx_backlog_empty(test_ctx->cnx_server)) {
                success = 1;
                break;
            }
        }

        if (ret == 0 && success == 0) {
            DBG_PRINTF("Exit synch loop after %d rounds, backlog or not enough cid (%d & %d).\n",
                nb_rounds, test_ctx->cnx_client->first_local_cnxid_list->nb_local_cnxid, test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid);
        }
    }

    /* Check */

    if (ret == 0) {
        if (test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid != PICOQUIC_NB_PATH_TARGET) {
            DBG_PRINTF("Found %d cid active on server instead of %d.\n", test_ctx->cnx_server->first_local_cnxid_list->nb_local_cnxid, PICOQUIC_NB_PATH_TARGET + 1);
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = transmit_cnxid_test_stash(test_ctx->cnx_client, test_ctx->cnx_server, "client");
    }

    if (ret == 0) {
        ret = transmit_cnxid_test_stash(test_ctx->cnx_server, test_ctx->cnx_client, "server");
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
fn not_before_cnxid() {
    not_before_cnxid_impl().expect("not_before_cnxid");
}
```

## `picoquictest/tls_api_test.c:pn_enc_1rtt_test`
* C test-table name: `pn_enc_1rtt`
* C entry function: `pn_enc_1rtt_test`
* Rust test: `pn_enc_1rtt`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5918-5970`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic handshake/loss test; it never waits for 1-RTT AEAD readiness or checks client-to-server and server-to-client packet-number encryption/decryption with the C vectors.
* Phase 5A fix note: After handshake, wait for application AEAD readiness and test both 1-RTT pn_enc/pn_dec pairs using seq_num_1/sample_1 and seq_num_2/sample_2, analogous to test_one_pn_enc_pair.
* Phase 5B analysis: Rust test is present, compiles as a Rust test, and matches the C API-level contract: init context, run connection loop, wait for 1-RTT application AEAD, then exercise both client/server 1-RTT pn_enc/pn_dec round trips twice. Any failure to make server 1-RTT AEAD decrypt ready is a Phase 5C runtime implementation issue, not a Phase 5B block.
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

    if (ret == 0)
    {
        /* Try to encrypt a sequence number */
        uint8_t seq_num_1[4] = { 0xde, 0xad, 0xbe, 0xef };
        uint8_t sample_1[16] = {
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
            0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a };
        uint8_t seq_num_2[4] = { 0xba, 0xba, 0xc0, 0x0l };
        uint8_t sample_2[16] = {
            0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96 };

        for (int i = 1; i < 4; i *= 2)
        {
            ret = test_one_pn_enc_pair(seq_num_1, 4, test_ctx->cnx_client->crypto_context[3].pn_enc, test_ctx->cnx_server->crypto_context[3].pn_dec, sample_1);

            if (ret == 0)
            {
                ret = test_one_pn_enc_pair(seq_num_2, 4, test_ctx->cnx_server->crypto_context[3].pn_enc, test_ctx->cnx_client->crypto_context[3].pn_dec, sample_2);
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

### Current Rust test body
```rust
fn pn_enc_1rtt() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("pn_enc_1rtt connection");
    wait_application_aead_ready(&mut test_ctx, &mut simulated_time)
        .expect("pn_enc_1rtt application aead");

    let seq_num_1: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
    let sample_1: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17,
        0x2a,
    ];
    let seq_num_2: [u8; 4] = [0xba, 0xba, 0xc0, 0x00];
    let sample_2: [u8; 16] = [
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a, 0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f,
        0x96,
    ];

    let epoch = Epoch::OneRtt as usize;
    let (qclient, qserver) = (&mut test_ctx.qclient, &mut test_ctx.qserver);
    let client = qclient.first_cnx_mut().expect("client connection");
    let server = qserver.first_cnx_mut().expect("server connection");

    for _i in [1, 2] {
        test_one_pn_enc_pair(
            &seq_num_1,
            client.crypto_context[epoch]
                .pn_enc
                .as_deref()
                .expect("client 1-RTT pn_enc"),
            server.crypto_context[epoch]
                .pn_dec
                .as_deref()
                .expect("server 1-RTT pn_dec"),
            &sample_1,
        );
        test_one_pn_enc_pair(
            &seq_num_2,
            server.crypto_context[epoch]
                .pn_enc
                .as_deref()
                .expect("server 1-RTT pn_enc"),
            client.crypto_context[epoch]
                .pn_dec
                .as_deref()
                .expect("client 1-RTT pn_dec"),
            &sample_2,
        );
    }
}
```
