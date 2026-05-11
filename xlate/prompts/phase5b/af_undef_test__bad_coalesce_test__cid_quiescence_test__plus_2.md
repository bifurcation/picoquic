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

Owned Rust test file(s): `rs/fq/src/tests/tls_api.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/tls_api_test.c:af_undef_test`
* C test-table name: `af_undef`
* C entry function: `af_undef_test`
* Rust test: `af_undef`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:1034-1054`
* Phase 5A analysis: Rust runs a generic TLS handshake/close helper. It does not set client_endpoint.addr_to_unspec, does not use the AF-specific initial CID, and does not run or verify the very_long data scenario.
* Phase 5A fix note: Implement af_undef with the explicit initial CID, addr_to_unspec=true, server qlog/long-log setup, connection loop, very_long scenario send loop, and 1,000,000us completion verification.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The af_undef body faithfully mirrors the C flow, but current tls_api.rs has duplicate top-level TEST_SCENARIO_* constants, including TEST_SCENARIO_VERY_LONG, so this test is not compile-exposed in the final tree.
* Phase 5C fix note: Remove the duplicate tls_api.rs scenario-constant merge artifacts; no af_undef body/API mismatch remains.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xaf, 0x0d, 0xef, 0, 0, 0, 0, 0}, 8 };
    uint64_t target_time = 1000000;
    int ret;

    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0) {
        test_ctx->client_endpoint.addr_to_unspec = 1;
        picoquic_set_qlog(test_ctx->qserver, ".");
        test_ctx->qserver->use_long_log = 1;
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    /* Try to complete the data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, target_time);
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
fn af_undef() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xaf, 0x0d, 0xef, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .expect("tls_api_init_ctx_ex");

    test_ctx.client_endpoint.addr_to_unspec = true;
    test_ctx.qserver.set_qlog(".").expect("server qlog");
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 1_000_000)
        .expect("scenario verify");
}
```

## `picoquictest/tls_api_test.c:bad_coalesce_test`
* C test-table name: `bad_coalesce`
* C entry function: `bad_coalesce_test`
* Rust test: `bad_coalesce`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:1324-1330`
* Phase 5A analysis: C sets do_bad_coalesce_test so client departure prepends a malformed coalesced packet, then runs test_scenario_q_and_r with a 250,000 us target. Rust does not set or implement that flag/path, passes an empty scenario, and uses a 2,000,000 us target.
* Phase 5A fix note: Add/use a Rust bad-coalesce test context flag and client-departure injection equivalent, run the q_and_r scenario {stream_id 4, q_len 257, r_len 2000}, and use the C completion target of 250,000 us.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust bad_coalesce matches the C flag, Q_AND_R scenario, and 250000 us target, but tls_api.rs has duplicate TEST_SCENARIO_* constants and util.rs has merge damage.
* Phase 5C fix note: Deduplicate tls_api.rs scenario constants and repair util.rs harness damage; the bad_coalesce body/API choice is otherwise faithful.

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

## `picoquictest/tls_api_test.c:cid_quiescence_test`
* C test-table name: `cid_quiescence`
* C entry function: `cid_quiescence_test`
* Rust test: `cid_quiescence`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:1378-1407`
* Phase 5A analysis: Rust only runs the generic handshake/close helper; it does not send the very-long scenario, advance time by CID refresh delay, or assert the client remote CID rotated.
* Phase 5A fix note: Add a dedicated Rust test mirroring the C flow: establish connection, initialize very-long stream scenario, wait ready, record remote CID, advance by CID_REFRESH_DELAY, send/verify data, then assert the remote CID changed.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The cid_quiescence body faithfully mirrors the C flow and CID-rotation assertion, but the same duplicate tls_api.rs scenario constants prevent the test file from compiling as a runnable Rust test.
* Phase 5C fix note: Remove the duplicate tls_api.rs scenario-constant merge artifacts; no cid_quiescence body/API mismatch remains.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t previous_remote_id = picoquic_null_connection_id;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    /* Set up the connection */
    if (ret == 0) {
        /* establish the connection */
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        previous_remote_id = test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id;
        /* Prepare to send data */
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_very_long, sizeof(test_scenario_very_long));
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        previous_remote_id = test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id;
        simulated_time += PICOQUIC_CID_REFRESH_DELAY;
    }

    /* Perform a data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, 0);
    }
    
    /* Verify that the CID has rotated */
    if (ret == 0 &&
        picoquic_compare_connection_id(&previous_remote_id, &test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id) == 0) {
        ret = -1;
    }
    
    /* And then free the resource  */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn cid_quiescence() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    let _remote_after_handshake =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID after handshake");

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time).expect("client ready");

    let previous_remote_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID before quiescence");
    simulated_time += CID_REFRESH_DELAY;

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)
        .expect("scenario verify");

    let refreshed_remote_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID after quiescence");
    assert_ne!(
        previous_remote_id, refreshed_remote_id,
        "client remote CID did not rotate after CID refresh delay"
    );
}
```

## `picoquictest/tls_api_test.c:tls_api_client_losses_test`
* C test-table name: `client_losses`
* C entry function: `tls_api_client_losses_test`
* Rust test: `client_losses`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:1520-1522`
* Phase 5A analysis: The Rust test passes mask 3, matching the C entry, but Rust tls_api_loss_test ignores its loss mask and always runs the handshake with zero loss.
* Phase 5A fix note: Thread the provided loss mask into tls_api_test_with_loss/tls_api_connection_loop so client_losses actually drops packets 1 and 2 and verifies recovery.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust client_losses calls tls_api_loss_test(3), and the helper threads the loss mask, but the current TLS test module/harness is not runnable after merge damage.
* Phase 5C fix note: Deduplicate tls_api.rs scenario constants and repair util.rs harness damage; the client_losses body/API choice is otherwise faithful.

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

## `picoquictest/tls_api_test.c:excess_repeat_test`
* C test-table name: `excess_repeat`
* C entry function: `excess_repeat_test`
* Rust test: `excess_repeat`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:2274-2287`
* Phase 5A analysis: C iterates six congestion algorithms and runs excess_repeat_test_one with repeat limits after killing the client. Rust only runs the generic TLS success helper, with no CC loop, long transfer, client disappearance, repeat counting, or thresholds.
* Phase 5A fix note: Add/translate excess_repeat_test_one and make excess_repeat iterate newreno, cubic, dcubic, fastcc, bbr, and prague with the C repeat limits and disconnect/repeat assertions.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust helper matches the C long-transfer/repeat-count contract, but the final merged registry no longer exposes the Phase 5B fastcc alias; get_congestion_algorithm("fastcc") is not a valid final Rust API surface.
* Phase 5C fix note: Restore the fastcc registry alias or change the test case to the final Rust fastcc algorithm id, "fast", while preserving the six C algorithms and repeat thresholds.

### C test body
```c
{
    const int nb_repeat_max = 128;

    picoquic_congestion_algorithm_t* algo_list[6] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr_algorithm,
        picoquic_prague_algorithm
    };
    int ret = 0;

    for (int i = 0; i < 6 && ret == 0; i++) {
        ret = excess_repeat_test_one(algo_list[i], nb_repeat_max);
        if (ret != 0) {
            DBG_PRINTF("Excess repeat test fails for CC=%s", algo_list[i]->congestion_algorithm_id);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn excess_repeat() {
    const NB_REPEAT_MAX: usize = 128;
    const ALGORITHMS: &[&str] = &["newreno", "cubic", "dcubic", "fastcc", "bbr", "prague"];

    register_all_congestion_control_algorithms();

    for algo_id in ALGORITHMS {
        let cc_algo = get_congestion_algorithm(algo_id).unwrap_or_else(|| {
            panic!("congestion algorithm {algo_id} must be registered");
        });
        excess_repeat_test_one(cc_algo, NB_REPEAT_MAX)
            .unwrap_or_else(|e| panic!("excess_repeat({algo_id}): {e:?}"));
    }
}
```
