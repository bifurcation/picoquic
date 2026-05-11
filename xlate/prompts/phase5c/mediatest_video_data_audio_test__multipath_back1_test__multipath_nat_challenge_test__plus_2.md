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

## `picoquictest/mediatest.c:mediatest_video_data_audio_test`
* C test-table name: `mediatest_video_data_audio`
* C entry function: `mediatest_video_data_audio_test`
* Rust test: `mediatest_video_data_audio`
* Expected Rust file: `rs/fq/src/tests/mediatest.rs`
* Current Rust span: `rs/fq/src/tests/mediatest.rs:1438-1448`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry matches the top-level spec values, but the helper sends static generic streams and only verifies scenario completion; it omits C's periodic audio/video frame generation, media-specific stream priorities, finished-frame accounting, and latency/sigma/max checks.
* Phase 5A fix note: Port the C media-test driver behavior for data plus audio/video frames, including media stats checks for audio and video under the default latency thresholds.
* Phase 5B analysis: Rust test matches the C API-level contract: same BBR algorithm, bandwidth, video/audio flags, data size, and mediatest_one VideoDataAudio dispatch; runtime simulator/handshake failures are Phase 5C, not a 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    spec.data_size = 10000000;
    ret = mediatest_one(mediatest_video_data_audio, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn mediatest_video_data_audio() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::VideoDataAudio, &spec).expect("mediatest_video_data_audio");
}
```

## `picoquictest/multipath_test.c:multipath_back1_test`
* C test-table name: `multipath_back1`
* C entry function: `multipath_back1_test`
* Rust test: `multipath_back1`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1560-1562`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Entry arguments and Back1-specific setup mostly match, including slow path, break/recovery, and final path-count checks, but final stream completion and 3.3s deadline are not actually verified in Rust.
* Phase 5A fix note: Fix the shared Rust scenario verifier to check stream completion/callback errors and enforce max_completion_microsec.
* Phase 5B analysis: Rust #[test] multipath_back1 is present, compiles under the Rust test harness, and matches the C API-level contract by calling multipath_test_one with the 3.3s deadline and Back1 id. Any early multipath negotiation/runtime failure is Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* TODO: investigate why 3.3 instead of 3.05 with prior implementation of multipath */
    uint64_t max_completion_microsec = 3300000;

    return  multipath_test_one(max_completion_microsec, multipath_test_back1);
}
```

### Current Rust test body
```rust
fn multipath_back1() {
    multipath_test_one(3_300_000, MultipathTestId::Back1);
}
```

## `picoquictest/multipath_test.c:multipath_nat_challenge_test`
* C test-table name: `multipath_nat_challenge`
* C entry function: `multipath_nat_challenge_test`
* Rust test: `multipath_nat_challenge`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1650-1652`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper maps to the right timeout and NatChallenge enum, and it sets NAT rebinding plus challenge_required, but it still relies on the weakened shared verifier that ignores the C completion bound and stream-body verification.
* Phase 5A fix note: Repair shared scenario verification/path readiness while preserving the NAT challenge setup and final path/address assertions.
* Phase 5B analysis: Rust #[test] is present, compiles, and calls multipath_test_one(3_000_000, MultipathTestId::NatChallenge), matching the C wrapper. Delayed-start/server-accept runtime failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_nat_challenge);
}
```

### Current Rust test body
```rust
fn multipath_nat_challenge() {
    multipath_test_one(3_000_000, MultipathTestId::NatChallenge);
}
```

## `picoquictest/p2p_test.c:address_discovery_test`
* C test-table name: `address_discovery`
* C entry function: `address_discovery_test`
* Rust test: `address_discovery`
* Expected Rust file: `rs/fq/src/tests/p2p.rs`
* Current Rust span: `rs/fq/src/tests/p2p.rs:25-138`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers the negotiated-role, data-transfer, callback, and observed-address checks, but it does not recreate the client connection after changing default address-discovery transport parameters. The C test explicitly deletes and recreates the client connection so the new defaults are picked up before starting.
* Phase 5A fix note: Recreate the client connection after setting qclient/qserver default address discovery modes, or otherwise set the client connection local transport parameters before start so the setup matches the C test.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and mirrors the C API-level setup and assertions. The known address-discovery role/callback/observed-address failures are Phase 5C implementation behavior, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_connection_id_t initial_cid = { {0xad, 0xd8, 0xd1, 0x5c, 0, 0, 0, 0}, 8 };
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t loss_mask = 0;
    int ret = 0;

    ret = tls_api_one_scenario_init_ex(&test_ctx, &simulated_time, PICOQUIC_INTERNAL_TEST_VERSION_1, NULL, NULL, &initial_cid);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    if (ret == 0) {
        /* set the qlogs on both sides */
        picoquic_set_qlog(test_ctx->qclient, ".");
        picoquic_set_qlog(test_ctx->qserver, ".");
        picoquic_set_log_level(test_ctx->qserver, 1);
        picoquic_set_log_level(test_ctx->qclient, 1);
        test_ctx->qclient->use_long_log = 1;
        test_ctx->qserver->use_long_log = 1;
        /* Set the address discovery option */
        picoquic_set_default_address_discovery_mode(test_ctx->qclient, 3);
        picoquic_set_default_address_discovery_mode(test_ctx->qserver, 1);
        /* Delete the client connection and create a new one,
        * so it picks the parameters.
         */
        picoquic_delete_cnx(test_ctx->cnx_client);
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient,
            initial_cid, picoquic_null_connection_id,
            (struct sockaddr*)&test_ctx->server_addr, simulated_time,
            0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
        else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }

    /* establish the connection */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* wait until the client (and thus the server) is ready */
    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    /* Check that the address discovery option is negotiated */
    if (ret == 0) {
        if (test_ctx->cnx_client->is_address_discovery_provider ||
            !test_ctx->cnx_client->is_address_discovery_receiver ||
            !test_ctx->cnx_server->is_address_discovery_provider ||
            test_ctx->cnx_server->is_address_discovery_receiver) {
            DBG_PRINTF("Address discovery not properly negotiated, C:(%u,%u), S:(%u,%u)",
                test_ctx->cnx_client->is_address_discovery_provider,
                test_ctx->cnx_client->is_address_discovery_receiver,
                test_ctx->cnx_server->is_address_discovery_provider,
                test_ctx->cnx_server->is_address_discovery_receiver);
            ret = -1;
        }
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_address_discovery, sizeof(test_scenario_address_discovery));

        if (ret != 0)
        {
            DBG_PRINTF("Init send receive scenario returns %d\n", ret);
        }
    }

    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* Check that the transmission succeeded */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, 1000000);
    }

    /* Check that the address discovery callback was called.
     */
    if (ret == 0) {
        if (test_ctx->nb_address_observed == 0) {
            DBG_PRINTF("Got % addresses observed", test_ctx->nb_address_observed);
            ret = -1;
        }
    }

    /* Check that the observed address was set on the client connection */
    if (ret == 0 && picoquic_compare_addr(
        (struct sockaddr*)&test_ctx->cnx_client->path[0]->first_tuple->local_addr,
        (struct sockaddr*)&test_ctx->cnx_client->path[0]->first_tuple->observed_addr) != 0) {
        char text1[256];
        char text2[256];

        DBG_PRINTF("Local: %s, observed: %s",
            picoquic_addr_text((struct sockaddr*)&test_ctx->cnx_client->path[0]->first_tuple->local_addr, text1, sizeof(text1)),
            picoquic_addr_text((struct sockaddr*)&test_ctx->cnx_client->path[0]->first_tuple->observed_addr, text2, sizeof(text2)));
        ret = -1;
    }

    /* Delete the context */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn address_discovery() {
    let mut simulated_time = crate::Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xad, 0xd8, 0xd1, 0x5c, 0, 0, 0, 0]).expect("initial CID");
    let mut loss_mask = 0u64;

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_one_scenario_init_ex");

    // Set QLOG on both sides.
    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qserver.set_log_level(1);
    test_ctx.qclient.set_log_level(1);
    test_ctx.qclient.use_long_log = true;
    test_ctx.qserver.use_long_log = true;

    // Enable address discovery: client receives, server provides.
    test_ctx.qclient.set_default_address_discovery_mode(3);
    test_ctx.qserver.set_default_address_discovery_mode(1);

    // Delete the initial client connection and re-create it so the new
    // transport parameters are picked up.
    let client_token = test_ctx
        .cnx_client()
        .own_token
        .expect("client connection token");
    test_ctx.qclient.delete_connection(client_token);
    {
        let server_addr = test_ctx.server_addr;
        let cnx = test_ctx
            .qclient
            .create_connection(
                initial_cid,
                ConnectionId::default(),
                Some(&server_addr),
                simulated_time,
                0,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .expect("create client connection");
        cnx.start_client().expect("start client");
    }

    // Establish the connection.
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    // Wait until the client (and server) are ready.
    super::util::wait_client_connection_ready(&mut test_ctx, &mut simulated_time)
        .expect("wait client ready");

    // Check that address discovery was negotiated.
    {
        let cnx_c = test_ctx.cnx_client();
        assert!(
            !cnx_c.is_address_discovery_provider,
            "client should not be a provider"
        );
        assert!(
            cnx_c.is_address_discovery_receiver,
            "client should be a receiver"
        );
    }
    {
        let cnx_s = test_ctx.cnx_server();
        assert!(
            cnx_s.is_address_discovery_provider,
            "server should be a provider"
        );
        assert!(
            !cnx_s.is_address_discovery_receiver,
            "server should not be a receiver"
        );
    }

    // Send test data.
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_ADDRESS_DISCOVERY)
        .expect("init send/recv scenario");

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 1_000_000)
        .expect("scenario body verify");

    // Verify the address-discovery callback fired.
    assert!(
        test_ctx.nb_address_observed > 0,
        "Got {} addresses observed",
        test_ctx.nb_address_observed
    );

    // Verify that the observed address matches the local address on path 0.
    {
        let cnx_c = test_ctx.cnx_client();
        let path0 = &cnx_c.paths[0];
        // The first tuple's local_addr and observed_addr should agree.
        let local = path0.tuples[0].local_addr;
        let observed = path0.tuples[0].observed_addr;
        assert_eq!(
            local, observed,
            "path[0] local addr ({local}) != observed addr ({observed})"
        );
    }
}
```

## `picoquictest/satellite_test.c:satellite_cubic_test`
* C test-table name: `satellite_cubic`
* C entry function: `satellite_cubic_test`
* Rust test: `satellite_cubic`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Current Rust span: `rs/fq/src/tests/satellite.rs:408-423`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Wrapper arguments match, but the Rust helper does not preserve the C stream0_target=data_size behavior, so it does not drive the 100MB transfer that the C test times.
* Phase 5A fix note: Update the Rust satellite helper to call a scenario helper that supports stream0_target=data_size, preserving the empty scenario, loss mask, queue delay, and completion target used by C.
* Phase 5B analysis: Reclassified as ok: the Rust #[test] is present, compiles under the Rust test harness, and preserves the C API-level contract for cubic, data_size=100000000, max_completion_time=6500000, 250/3 Mbps link, no jitter, no loss, no preemptive repeat, no seeding, no low flow, and no flow control. Any early runtime failure such as CannotSetActiveStream is a Phase 5C implementation/harness behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Should be less than 7 sec per draft etosat */
    return satellite_test_one(picoquic_cubic_algorithm, 100000000, 6500000, 250, 3, 0, 0, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_cubic() {
    let cubic = satellite_ccalgo("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        6_500_000,
        250,
        3,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```
