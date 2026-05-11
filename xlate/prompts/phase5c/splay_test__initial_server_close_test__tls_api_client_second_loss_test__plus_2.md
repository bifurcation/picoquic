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

## `picoquictest/splay_test.c:splay_test`
* C test-table name: `splay`
* C entry function: `splay_test`
* Rust test: `splay`
* Expected Rust file: `rs/fq/src/tests/splay.rs`
* Current Rust span: `rs/fq/src/tests/splay.rs:55-137`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers the same inserted values, min/max, find, find_previous, delete order, and empty-tree checks, but it omits the C test's per-insert/per-delete count, tree-size, and structural sanity assertions.
* Phase 5A fix note: Add Rust checks corresponding to C count/tree->size after each insert/delete, at least using len()/is_empty() and preferably an order/count walk via first/next if available.
* Phase 5B analysis: Rust now covers the C test's per-insert/per-delete count, tree-size, and traversal sanity checks.
* Phase 5B fix note: Added an in-order first/next traversal helper that verifies len(), is_empty(), strict key order, and traversal count after each insert/delete; added final len/is_empty empty-tree assertions.

### C test body
```c
int splay_test(void) {
    int ret = 0;
    int count = 0;
    picosplay_tree_t *tree = picosplay_new_tree(&compare_int, create_int_node, delete_int_node, int_node_value);
    int values[] = {5, 7, 1, 3, 13, 9, 11};
    int values_first[] = { 5, 5, 1, 1, 1, 1, 1 };
    int values_last[] = { 5, 7, 7, 7, 13, 13, 13 };
    int previous_test[] = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14 };
    int previous_value[] = { -1, 1, 1, 3, 3, 5, 5, 7, 7, 9, 9, 11, 11, 13, 13 };
    int value2_first[] = { 1, 1, 3, 9, 9, 11, 0 };
    int value2_last[] = { 13, 13, 13, 13, 11, 11, 0 };

    if (tree == NULL) {
        DBG_PRINTF("%s", "Cannot create tree.\n");
        ret = -1;
    }
    else {
        for (int i = 0; ret == 0 && i < 7; i++) {
            picosplay_insert(tree, new_int_node(values[i]));
            /* Verify sanity and count after each insertion */
            count = check_node_sanity(tree->root, NULL, NULL, &compare_int);
            if (count != i + 1) {
                DBG_PRINTF("Insert v[%d] = %d, expected %d nodes, got %d instead\n",
                    i, values[i], i + 1, count);
                ret = -1;
            }
            else if (tree->size != count) {
                DBG_PRINTF("Insert v[%d] = %d, expected tree size %d, got %d instead\n",
                    i, values[i], count, tree->size);
                ret = -1;
            }
            else if (((int_node_t*)int_node_value(picosplay_first(tree)))->v != values_first[i]) {
                DBG_PRINTF("Insert v[%d] = %d, expected first = %d, got %d instead\n",
                    i, values[i],
                    values_first[i], ((int_node_t*)int_node_value(picosplay_first(tree)))->v);
                ret = -1;
            }
            else if (((int_node_t*)int_node_value(picosplay_last(tree)))->v != values_last[i]) {
                DBG_PRINTF("Insert v[%d] = %d, expected first = %d, got %d instead\n",
                    i, values[i],
                    values_last[i], ((int_node_t*)int_node_value(picosplay_last(tree)))->v);
                ret = -1;
            }
        }

        for (int i = 0; ret == 0 && i < 15; i++) {
            int_node_t x;
            picosplay_node_t* y;
            
            x.v = previous_test[i];
            y = picosplay_find(tree, (void*)&x);

            if (previous_value[i] == previous_test[i]) {
                if (y == NULL) {
                    DBG_PRINTF("Find v[%d] = %d, expected = %d, got NULL instead\n",
                        i, previous_test[i], previous_value[i]);
                    ret = -1;
                }
                else {
                    int v = ((int_node_t*)int_node_value(y))->v;
                    if (v != previous_value[i]) {
                        DBG_PRINTF("Find v[%d] = %d, expected = %d, got %d instead\n",
                            i, previous_test[i], previous_value[i], v);
                        ret = -1;
                    }
                }
            }
            else {
                if (y != NULL) {
                    DBG_PRINTF("Find v[%d], expected NULL, got %d instead\n",
                        i, ((int_node_t*)int_node_value(y))->v);
                    ret = -1;
                }
            }
        }

        for (int i = 0; ret == 0 && i < 15; i++) {
            int_node_t x;
            picosplay_node_t* y;

            x.v = previous_test[i];
            y = picosplay_find_previous(tree, (void*)&x);

            if (previous_value[i] >= 0) {
                if (y == NULL) {
                    DBG_PRINTF("Next v[%d] = %d, expected = %d, got NULL instead\n",
                        i, previous_test[i], previous_value[i]);
                    ret = -1;
                }
                else {
                    int v = ((int_node_t*)int_node_value(y))->v;
                    if (v != previous_value[i]) {
                        DBG_PRINTF("next v[%d] = %d, expected = %d, got %d instead\n",
                            i, previous_test[i], previous_value[i], v);
                        ret = -1;
                    }
                }
            }
            else {
                if (y != NULL) {
                    DBG_PRINTF("Next v[%d], expected NULL, got %d instead\n",
                        i, ((int_node_t*)int_node_value(y))->v);
                    ret = -1;
                }
            }
        }

        for (int i = 0; ret == 0 && i < 7; i++) {
            int_node_t to_delete = { 0 };
            to_delete.v = values[i];
            picosplay_delete(tree, &to_delete);
            /* Verify sanity and count after each deletion */
            count = check_node_sanity(tree->root, NULL, NULL, &compare_int);
            if (count != 6 - i) {
                DBG_PRINTF("Delete v[%d] = %d, expected %d nodes, got %d instead\n",
                    i, values[i], 6 - i, count);
                ret = -1;
            }
            else if (tree->size != count) {
                DBG_PRINTF("Insert v[%d] = %d, expected tree size %d, got %d instead\n",
                    i, values[i], count, tree->size);
                ret = -1;
            }
            else if (i < 6) {
                if (((int_node_t*)int_node_value(picosplay_first(tree)))->v != value2_first[i]) {
                    DBG_PRINTF("Delete v[%d] = %d, expected first = %d, got %d instead\n",
                        i, values[i], value2_first[i], ((int_node_t*)int_node_value(picosplay_first(tree)))->v);
                    ret = -1;
                }
                else if (((int_node_t*)int_node_value(picosplay_last(tree)))->v != value2_last[i]) {
                    DBG_PRINTF("Delete v[%d] = %d, expected first = %d, got %d instead\n",
                        i, values[i], value2_last[i], ((int_node_t*)int_node_value(picosplay_last(tree)))->v);
                    ret = -1;
                }
            }
        }

        if (ret == 0 && tree->root != NULL) {
            DBG_PRINTF("%s", "Final tree root should be NULL, is not.\n");
            ret = -1;
        }
        picosplay_empty_tree(tree);
        free(tree);
        tree = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn splay() {
    let values = [5, 7, 1, 3, 13, 9, 11];
    let values_first = [5, 5, 1, 1, 1, 1, 1];
    let values_last = [5, 7, 7, 7, 13, 13, 13];
    // (key probed, expected previous-or-equal value, -1 means none).
    let previous_test: [i32; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
    let previous_value: [i32; 15] = [-1, 1, 1, 3, 3, 5, 5, 7, 7, 9, 9, 11, 11, 13, 13];
    let value2_first = [1, 1, 3, 9, 9, 11, 0];
    let value2_last = [13, 13, 13, 13, 11, 11, 0];

    let mut tree: SplayTree<i32, ()> = SplayTree::new();

    // Insertion + running min / max after each insert.
    for (i, &v) in values.iter().enumerate() {
        let (_token, prev) = tree.insert(v, ()).expect("insert under cap");
        assert!(prev.is_none(), "duplicate insert at i={i}, v={v}");
        assert_tree_sanity(&tree, i + 1, "insert", i, v);
        let first = tree
            .first()
            .and_then(|t| tree.get_key_value(t))
            .map(|(k, _)| *k);
        let last = tree
            .last()
            .and_then(|t| tree.get_key_value(t))
            .map(|(k, _)| *k);
        assert_eq!(first, Some(values_first[i]), "first after insert i={i}");
        assert_eq!(last, Some(values_last[i]), "last after insert i={i}");
    }

    // Direct lookups: succeed when the key is in `values`, fail otherwise.
    for (i, &probe) in previous_test.iter().enumerate() {
        let token = tree.find(&probe);
        if previous_value[i] == probe {
            let token = token.expect("hit on present key");
            let (k, _) = tree.get_key_value(token).expect("token live");
            assert_eq!(*k, previous_value[i], "find at i={i}, probe={probe}");
        } else {
            assert!(token.is_none(), "false hit at i={i}, probe={probe}");
        }
    }

    // Previous-or-equal lookups: succeed when there's any key ≤ probe.
    for (i, &probe) in previous_test.iter().enumerate() {
        let token = tree.find_previous(&probe);
        if previous_value[i] >= 0 {
            let token = token.expect("previous hit");
            let (k, _) = tree.get_key_value(token).expect("token live");
            assert_eq!(
                *k, previous_value[i],
                "find_previous at i={i}, probe={probe}"
            );
        } else {
            assert!(
                token.is_none(),
                "false previous-hit at i={i}, probe={probe}"
            );
        }
    }

    // Deletion + running min / max after each delete.
    for (i, &v) in values.iter().enumerate() {
        tree.remove_by_key(&v).expect("delete present key");
        assert_tree_sanity(&tree, 6 - i, "delete", i, v);
        if i < 6 {
            let first = tree
                .first()
                .and_then(|t| tree.get_key_value(t))
                .map(|(k, _)| *k);
            let last = tree
                .last()
                .and_then(|t| tree.get_key_value(t))
                .map(|(k, _)| *k);
            assert_eq!(first, Some(value2_first[i]), "first after delete i={i}");
            assert_eq!(last, Some(value2_last[i]), "last after delete i={i}");
        }
    }

    // Tree should be empty.
    assert_eq!(tree.len(), 0, "tree not empty after all deletes");
    assert!(tree.is_empty(), "tree not empty after all deletes");
    assert!(tree.first().is_none(), "tree not empty after all deletes");
    assert!(tree.last().is_none(), "tree not empty after all deletes");
}
```

## `picoquictest/tls_api_test.c:initial_server_close_test`
* C test-table name: `initial_server_close`
* C entry function: `initial_server_close_test`
* Rust test: `initial_server_close`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3245-3247`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs a normal successful handshake/close; it never stops at server_almost_ready, forces server handshake_failure with local_error 0xDEAD, or checks client remote_error and the 50000 us time bound.
* Phase 5A fix note: Recreate the C loop to reach State::ServerAlmostReady, force server State::HandshakeFailure and local_error 0xDEAD, reinsert/wake it, run the connection loop, then assert both sides disconnect, client remote_error is 0xDEAD, and simulated_time <= 50000.
* Phase 5B analysis: Rust test now matches the C API-level contract: it drives the same setup, forces server HandshakeFailure/local_error 0xDEAD, runs the connection loop, checks the client disconnect/error/time, and treats any early runtime failure as Phase 5C behavior.
* Phase 5B fix note: Relaxed the final server-side assertion to match C: if the server connection still exists it must be Disconnected, but deletion is allowed.

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    int was_active = 0;
    int nb_trials = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    /* Set the connection on the server side, but not on the client side */
    while (ret == 0 && nb_trials < 32 ) {
        nb_trials++;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

        if (test_ctx->cnx_server != NULL && test_ctx->cnx_server->cnx_state == picoquic_state_server_almost_ready) {
            break;
        }
    }

    if (test_ctx->cnx_server == NULL || test_ctx->cnx_server->cnx_state != picoquic_state_server_almost_ready) {
        DBG_PRINTF("Server state: %d\n", (test_ctx->cnx_server == NULL) ?
            -1 : test_ctx->cnx_server->cnx_state);
        ret = -1;
    }

    if (ret == 0) {
        test_ctx->cnx_server->cnx_state = picoquic_state_handshake_failure;
        test_ctx->cnx_server->local_error = 0xDEAD;
        picoquic_reinsert_by_wake_time(test_ctx->qserver, test_ctx->cnx_server, simulated_time);
    }


    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        if (test_ctx->cnx_server != NULL &&
            test_ctx->cnx_server->cnx_state != picoquic_state_disconnected) {
            DBG_PRINTF("Server state: %d, remote error: %x\n", test_ctx->cnx_server->cnx_state, test_ctx->cnx_server->remote_error);
            ret = -1;
        }
        else if (test_ctx->cnx_client->cnx_state != picoquic_state_disconnected ||
            test_ctx->cnx_client->remote_error != 0xDEAD) {
            DBG_PRINTF("Client state: %d, local error: %x", test_ctx->cnx_client->cnx_state, test_ctx->cnx_client->local_error);
            ret = -1;
        }
        else if (simulated_time > 50000ull) {
            DBG_PRINTF("Simulated time: %llu", (unsigned long long)simulated_time);
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

### Current Rust test body
```rust
fn initial_server_close() {
    initial_server_close_test_one().expect("initial_server_close");
}
```

## `picoquictest/tls_api_test.c:tls_api_client_second_loss_test`
* C test-table name: `second_loss`
* C entry function: `tls_api_client_second_loss_test`
* Rust test: `second_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7243-7245`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C passes loss mask 2 into tls_api_test_with_loss, so the connection loop runs with that loss pattern. Rust tls_api_loss_test accepts the mask but ignores it, and tls_api_test_with_loss always uses a zero loss mask.
* Phase 5A fix note: Thread the loss mask through Rust tls_api_loss_test/tls_api_test_with_loss into tls_api_connection_loop, preserving the C second-packet-loss case and final handshake verification.
* Phase 5B analysis: Rust second_loss already passed mask 2, but tls_api_loss_test discarded it before the simulator loop. The helper now preserves the mask so the connection loop runs the second-packet-loss case and still performs final close verification.
* Phase 5B fix note: Threaded the loss mask through tls_api_loss_test via an internal tls_api_test_with_loss_mask helper; existing no-loss callers keep the same tls_api_test_with_loss API.

### C test body
```c
{
    return tls_api_loss_test(2ull);
}
```

### Current Rust test body
```rust
fn second_loss() {
    tls_api_loss_test(2).expect("second_loss");
}
```

## `picoquictest/util_test.c:util_connection_id_print_test`
* C test-table name: `connection_id_print`
* C entry function: `util_connection_id_print_test`
* Rust test: `connection_id_print`
* Expected Rust file: `rs/fq/src/tests/util_test.rs`
* Current Rust span: `rs/fq/src/tests/util_test.rs:26-48`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C tests picoquic_print_connection_id_hexa success return, exact lowercase hex output, and failure on too-small buffer. Rust manually formats bytes in the test and never calls print_connection_id_hexa or checks the error path.
* Phase 5A fix note: Call crate::utils::print_connection_id_hexa for each expected CID and add a failing fmt::Write sink to assert the Rust error path.
* Phase 5B analysis: Rust test now exercises the translated print_connection_id_hexa helper for success output and error propagation, matching the C test intent.
* Phase 5B fix note: Replaced manual byte formatting with crate::utils::print_connection_id_hexa and added a failing fmt::Write sink assertion for the error path.

### C test body
```c
{
    int ret = 0;  
    char cnxid_str[2 * PICOQUIC_CONNECTION_ID_MAX_SIZE + 1];
  
    for (size_t i = 0; i < test_cases; ++i)
    {
        int result = picoquic_print_connection_id_hexa(cnxid_str, sizeof(cnxid_str), &expected_cnxid[i]);
        if (result != 0) {
            DBG_PRINTF("picoquic_print_connection_id_hexa failed with: %d\n", result);
            ret = -1;
        }
        if (strcmp(cnxid_str, expected_str[i]) != 0) {
            DBG_PRINTF("result: %s, expected: %s\n", cnxid_str, expected_str[i]);
            ret = -1;
        }
    }

    // Test invalid call
    if (picoquic_print_connection_id_hexa("", 0, &expected_cnxid[0]) == 0) {
        DBG_PRINTF("%s", "picoquic_print_connection_id_hexa did not fail\n");
        ret = -1;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn connection_id_print() {
    for (bytes, expected) in EXPECTED_CIDS {
        let cid = ConnectionId::clone_from_slice(bytes).expect("CID under cap");
        let mut got = String::new();
        crate::utils::print_connection_id_hexa(&mut got, &cid).expect("CID hex print");
        assert_eq!(got, *expected, "CID hex round-trip");
    }

    struct FailingWrite;

    impl core::fmt::Write for FailingWrite {
        fn write_str(&mut self, _s: &str) -> core::fmt::Result {
            Err(core::fmt::Error)
        }
    }

    let cid = ConnectionId::clone_from_slice(EXPECTED_CIDS[0].0).expect("CID under cap");
    let mut failing = FailingWrite;
    assert!(
        crate::utils::print_connection_id_hexa(&mut failing, &cid).is_err(),
        "CID hex print should fail when the sink rejects output",
    );
}
```

## `picoquictest/congestion_test.c:bbr_asym100_test`
* C test-table name: `bbr_asym100`
* C entry function: `bbr_asym100_test`
* Rust test: `bbr_asym100`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:855-857`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Entry parameters match, but the Rust scenario verification helper ignores max_completion_microsec, so the 8.5s performance bound from the C test is not enforced.
* Phase 5A fix note: Update the Rust performance path, preferably tls_api_one_scenario_body_verify, to verify scenario completion and enforce completion_time <= max_completion_microsec before closing.
* Phase 5B analysis: Rust #[test] bbr_asym100 is present and calls performance_test_one with the same API-level parameters as C; runtime Generic failure is a Phase 5C implementation/runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_time = 8500000;
    uint64_t latency = 1000;
    uint64_t jitter = 750;
    uint64_t buffer = 50000;
    uint64_t mbps = 10;
    uint64_t kbps = 100;

    int ret = performance_test_one(max_completion_time, mbps, kbps, latency, jitter, buffer, NULL);

    return ret;
}
```

### Current Rust test body
```rust
fn bbr_asym100() {
    performance_test_one(8_500_000, 10, 100, 1_000, 750, 50_000, None);
}
```
