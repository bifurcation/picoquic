# Phase 6 cluster investigation: multipath handshake

You are debugging a **cluster** of 29 failing Rust tests in `rs/fq/src/tests/multipath.rs` that all panic at the **same line** with the **same assertion message**.  This is almost certainly a single root cause — fix that and you unblock the whole cluster.

## The shared failure

Every failing test in the cluster reaches this code in `rs/fq/src/tests/multipath.rs`:

```rust
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        queue_delay,
        &mut simulated_time,
    )
    .expect("connection loop");     // line ~1037 — does NOT panic

    assert!(
        test_ctx.has_cnx_server(),  // line 1039–1042 — PANICS here
        "server connection not accepted during multipath handshake"
    );
```

So `tls_api_connection_loop` returns `Ok(())` but afterwards `test_ctx.has_cnx_server()` is `false` — the server never accepted the client's connection during the handshake simulation.

Look at `tls_api_connection_loop` in `rs/fq/src/tests/util.rs:1113` — it spins `tls_api_one_sim_round_with_loss` until either both sides report ready, or `nb_inactive >= 512`, or `nb_trials >= 1024`.  For these tests, the loop is exhausting `nb_inactive` (the simulation goes idle) **before the server creates its connection object**, so `has_cnx_server()` stays false.

The cluster suggests a multipath-specific server-side handshake bug: something in how the server processes the initial client packets (or transport parameters / multipath capability negotiation) is failing such that no server `Connection` is instantiated.

## All 29 tests in the cluster

```
tests::multipath::backup_demote
tests::multipath::backup_path
tests::multipath::backup_promote
tests::multipath::datagram_test
tests::multipath::ec00_zero_loss
tests::multipath::ec90_corruption
tests::multipath::failover
tests::multipath::failover_data
tests::multipath::failover_data_one_way
tests::multipath::flow_control_test
tests::multipath::multipath_abandon
tests::multipath::multipath_async_2
tests::multipath::multipath_basic
tests::multipath::multipath_basic_3
tests::multipath::multipath_callback
tests::multipath::multipath_drop_first
tests::multipath::multipath_drop_second
tests::multipath::multipath_keepalive
tests::multipath::multipath_keep_alive
tests::multipath::multipath_perf
tests::multipath::multipath_qlog
tests::multipath::multipath_quality
tests::multipath::multipath_renew
tests::multipath::multipath_sat_plus
tests::multipath::multipath_socket_error
tests::multipath::multipath_standby
tests::multipath::multipath_stream_af
tests::multipath::multipath_unreachable
tests::multipath::pacing_update
```

(Some names may differ slightly — the source of truth is the pending failures in `xlate/phase6_failures.json` whose excerpt mentions `multipath.rs:1039` or that assertion.)

## Your job

1. Pick **one** representative test (e.g. `multipath_basic` or `flow_control_test`) and run it:
   ```
   cd rs/fq && CARGO_TARGET_DIR=/private/tmp/fq-target cargo nextest run --features sys-openssl --no-fail-fast -E 'test(tests::multipath::multipath_basic)' --no-capture
   ```
2. Compare the **Rust** multipath setup against the **C** equivalent.  Useful starting points:
   * C test entry: `picoquictest/multipath_test.c` (search for the test name's C counterpart in `xlate/test_translation_map.json`).
   * Rust: `rs/fq/src/tests/multipath.rs` — the failing test plus `multipath_test_one_basic` (its main helper).
   * Server-side connection acceptance lives in the QUIC packet-receive path; in Rust look for the equivalent of C's `picoquic_incoming_packet` and how it instantiates server `Connection` objects on first Initial.
3. **Investigate the implementation** for the multipath-handshake path.  Likely suspects:
   * Multipath transport-parameter encode/decode (`set_default_multipath_option`, `default_multipath_option`).
   * Server's handling of the multipath capability in the ClientHello / first Initial.
   * Server-side connection creation when multipath-version transport params are present.
   * Initial-CID handling under multipath (the tests often pre-set an initial CID).
4. Fix the implementation.  Do **not** edit the C source.  Do **not** weaken the tests — they have to keep asserting that the server accepts the connection.
5. Verify by running the representative test, then 3–5 other tests from the list.  Then run a broader nextest filter:
   ```
   cd rs/fq && CARGO_TARGET_DIR=/private/tmp/fq-target cargo nextest run --features sys-openssl --no-fail-fast -E 'test(tests::multipath::)'
   ```

## Rules

* No C-side edits.
* No test weakening (no `#[ignore]`, no loosening assertions, no removing checks).  Test edits are ONLY allowed if a translation bug brought the Rust test *farther* from its C twin — and the fix has to bring it *closer*.
* Prefer the smallest faithful Rust fix.  If the implementation is wrong, fix the implementation.
* Report what you did, the root cause you found, and how many sibling tests now pass after your fix.

Return a short markdown summary at the end with:
* The root cause (one paragraph).
* Files changed.
* Sibling tests verified passing.
