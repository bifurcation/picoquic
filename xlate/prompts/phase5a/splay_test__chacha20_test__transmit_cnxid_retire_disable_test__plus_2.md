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

## `picoquictest/splay_test.c:splay_test`
* C test-table name: `splay`
* C entry function: `splay_test`
* Rust test: `splay`
* C source: `picoquictest/splay_test.c:106-253`
* Rust source: `rs/fq/src/tests/splay.rs:16-94`

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

### Rust test body
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
    assert!(tree.first().is_none(), "tree not empty after all deletes");
    assert!(tree.last().is_none(), "tree not empty after all deletes");
}
```

## `picoquictest/tls_api_test.c:chacha20_test`
* C test-table name: `chacha20`
* C entry function: `chacha20_test`
* Rust test: `chacha20`
* C source: `picoquictest/tls_api_test.c:10868-10904`
* Rust source: `rs/fq/src/tests/tls_api.rs:100-102`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int has_chacha_poly = 0;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Set the cipher suite to chacha20
     */
    if (ret == 0) {
        has_chacha_poly = (picoquic_set_cipher_suite(test_ctx->qclient, PICOQUIC_CHACHA20_POLY1305_SHA256) == 0);

        if (has_chacha_poly) {

            /* Run a basic test scenario */
            ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
                test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 250000);
        }
        else {
            DBG_PRINTF("%s", "Could not test CHACHA20, not supported on this platform.");
        }
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
fn chacha20() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("chacha20");
}
```

## `picoquictest/tls_api_test.c:transmit_cnxid_retire_disable_test`
* C test-table name: `cnxid_transmit_r_disable`
* C entry function: `transmit_cnxid_retire_disable_test`
* Rust test: `cnxid_transmit_r_disable`
* C source: `picoquictest/tls_api_test.c:6626-6629`
* Rust source: `rs/fq/src/tests/tls_api.rs:229-231`

### C test body
```c
{
    return transmit_cnxid_test_one(1, 1, 0);
}
```

### Rust test body
```rust
fn cnxid_transmit_r_disable() {
    transmit_cnxid_test_one(true, true, false).expect("cnxid_transmit_r_disable");
}
```

## `picoquictest/tls_api_test.c:error_reason_test`
* C test-table name: `error_reason`
* C entry function: `error_reason_test`
* Rust test: `error_reason`
* C source: `picoquictest/tls_api_test.c:12408-12490`
* Rust source: `rs/fq/src/tests/tls_api.rs:318-320`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xe8, 0x80, 0x88, 0xea, 0x50, 0, 0, 0}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN,
        &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    if (ret == 0) {
        /* Request the logs on the server side, so manual inspection can verify that
         * the error reason is properly displayed. */
        picoquic_set_textlog(test_ctx->qserver, error_reason_text_log);
        test_ctx->qserver->use_long_log = 1;
        picoquic_set_qlog(test_ctx->qserver, ".");
        /* Now, start the client connection */
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }

    if (ret == 0) {
        /* Perform a connection loop to verify it goes OK */
        ret = tls_api_connection_loop(test_ctx, &loss_mask,
            2 * test_ctx->c_to_s_link->microsec_latency, &simulated_time);

        if (ret != 0)
        {
            DBG_PRINTF("Connection loop returns %d\n", ret);
        }
    }

    if (ret == 0) {
        /* force closure of the client connection with an internal error */
        int local_error_ret = picoquic_connection_error_ex(test_ctx->cnx_client, PICOQUIC_TRANSPORT_INTERNAL_ERROR,
            0, "error reason test");
        if (local_error_ret != PICOQUIC_ERROR_DETECTED) {
            DBG_PRINTF("picoquic_connection_error_ex returns %d\n", ret);
            ret = -1;
        }
    }

    if (ret == 0) {
        /* verify that the connection will be closed */
        int nb_trials = 0;
        int nb_inactive = 0;
        while (ret == 0 && nb_trials < 1024 && nb_inactive < 512 ) {
            int was_active = 0;
            nb_trials++;

            ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);

            if (test_ctx->cnx_client->cnx_state == picoquic_state_disconnected &&
                (test_ctx->cnx_server == NULL || test_ctx->cnx_server->cnx_state == picoquic_state_disconnected)) {
                break;
            }

            if (nb_trials == 512) {
                DBG_PRINTF("After %d trials, client state = %d, server state = %d",
                    nb_trials, (int)test_ctx->cnx_client->cnx_state,
                    (test_ctx->cnx_server == NULL) ? -1 : test_ctx->cnx_server->cnx_state);
            }

            if (was_active) {
                nb_inactive = 0;
            }
            else {
                nb_inactive++;
            }
        }
    }
    /* Close the contexts, which will close the logs.
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
fn error_reason() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("error_reason");
}
```

## `picoquictest/tls_api_test.c:heavy_loss_test`
* C test-table name: `heavy_loss`
* C entry function: `heavy_loss_test`
* Rust test: `heavy_loss`
* C source: `picoquictest/tls_api_test.c:11359-11362`
* Rust source: `rs/fq/src/tests/tls_api.rs:384-386`

### C test body
```c
{
    return heavy_loss_test_one(0, 23500000);
}
```

### Rust test body
```rust
fn heavy_loss() {
    heavy_loss_test_one(0, 23_500_000).expect("heavy_loss");
}
```
