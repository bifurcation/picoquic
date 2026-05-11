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

## `picoquictest/cert_verify_test.c:cert_verify_bad_cert_test`
* C test-table name: `cert_verify_bad_cert`
* C entry function: `cert_verify_bad_cert_test`
* Rust test: `cert_verify_bad_cert`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Current Rust span: `rs/fq/src/tests/cert_verify.rs:42-50`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The bad cert/key/CA/SNI inputs match, but Rust compares only tls_api_connection_loop Result. C treats success as loop OK plus both endpoints ready, so rejection also includes loop OK with not-ready endpoints.
* Phase 5A fix note: In cert_verify_test_one, compute success as result.is_ok() && client_ready && server_ready, then compare that boolean to expect_success.
* Phase 5B analysis: Rust now matches the C helper success semantics: loop OK plus client and server ready. Focused bad-cert test passes; the full cert_verify filter exposes a valid-cert implementation readiness gap.
* Phase 5B fix note: Changed cert_verify_test_one to compute success from result.is_ok() && client_ready && server_ready, then compare to expect_success.

### C test body
```c
{
    int ret = cert_verify_test_one(0, CERT_VERIFY_RSA_BAD_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_bad_cert() {
    cert_verify_test_one(
        false,
        Some(TEST_FILE_SERVER_BAD_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_SNI),
    );
}
```

## `picoquictest/getter_test.c:getter_test`
* C test-table name: `getter`
* C entry function: `getter_test`
* Rust test: `getter`
* Expected Rust file: `rs/fq/src/tests/getter.rs`
* Current Rust span: `rs/fq/src/tests/getter.rs:40-536`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is only a partial/weak translation: several C checks are tautologies, no-op reads, skipped cases, or placeholders, and some assertions use different error codes or omit expected field comparisons.
* Phase 5A fix note: Restore C-equivalent assertions for default TTL/TP, local interface and CID getters, sslkeylog, exact handshake error codes, congestion algorithm identity/default update, keep-alive intervals, application error getter, remote stream missing-stream result, max-connection tentative value, crypto epoch defaults, local CID true case, register_cnx_id, register_net_icid first-call behavior, get_quic_ctx(NULL), and the invalid queue_misc_frame length case where an API/test hook is needed.
* Phase 5B analysis: Rust test is present, compiles, and is runnable. Remaining misc-frame/handshake behavior failures are Phase 5C notes, not Phase 5B blockers; SIZE_MAX queue length is unrepresentable in the safe slice API.
* Phase 5B fix note: Added a runnable duplicate register_cnx_id assertion and the Rust Option equivalent of get_quic_ctx(NULL).

### C test body
```c
{
    /* Create a connection context so we can test the various API */
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_cnx_t* cnx = NULL;
    picoquic_cnx_t* cnx_s = NULL;
    picoquic_connection_id_t initial_cid = { {0x9e, 0x77, 0xe8, 0, 0, 0, 0, 0}, 8 };
    int ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN,
        &simulated_time, NULL, NULL,
        0, 1, 0, &initial_cid, 8, 0, 0, 0);

    if (ret == 0) {
        if (picoquic_set_default_connection_id_length(test_ctx->qserver, (uint8_t)255) != PICOQUIC_ERROR_CNXID_CHECK ||
            picoquic_set_default_connection_id_length(test_ctx->qclient, (uint8_t)5) != PICOQUIC_ERROR_CANNOT_CHANGE_ACTIVE_CONTEXT) {
            ret = -1;
        }
    }

    if (ret == 0) {
        if (picoquic_get_default_connection_id_ttl(test_ctx->qserver) != test_ctx->qserver->local_cnxid_ttl) {
            ret = -1;
        }
    }

    if (ret == 0) {
        const picoquic_tp_t* tp = picoquic_get_default_tp(test_ctx->qserver);
        if (tp != &test_ctx->qserver->default_tp) {
            ret = -1;
        }
    }

    if (ret == 0) {
        uint64_t old_max = test_ctx->qserver->cwin_max;
        picoquic_set_cwin_max(test_ctx->qserver, 0);
        if (test_ctx->qserver->cwin_max != UINT64_MAX) {
            ret = -1;
        }
        test_ctx->qserver->cwin_max = old_max;
    }

    if (ret == 0) {
        int partial_match = 0;
        int path_id = picoquic_find_path_by_address(test_ctx->cnx_client, NULL,
            (struct sockaddr*)&test_ctx->cnx_client->path[0]->first_tuple->peer_addr, &partial_match);
        if (path_id != 0 || partial_match == 0) {
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = picoquic_set_local_addr(test_ctx->cnx_client, (struct sockaddr*) &test_ctx->client_addr);
        if (ret == 0 && picoquic_set_local_addr(test_ctx->cnx_client, (struct sockaddr*)&test_ctx->client_addr) == 0) {
            /* Second call should fail because the address is already set */
            ret = -1;
        }
        memset(&test_ctx->cnx_client->path[0]->first_tuple->local_addr, 0, sizeof(struct sockaddr_storage));
    }

    if (ret == 0) {
        uint8_t mf[] = { picoquic_frame_type_max_streams_bidir, 0x41, 0 };
        cnx = test_ctx->cnx_client;
        if (picoquic_queue_misc_frame(cnx, mf, SIZE_MAX, 0, picoquic_packet_context_initial) == 0) {
            ret = -1;
        }
        else if (picoquic_queue_misc_frame(cnx, mf, sizeof(mf), 0, picoquic_packet_context_initial) != 0) {
            ret = -1;
        }
        else {
            picoquic_purge_misc_frames_after_ready(cnx);
            if (cnx->first_misc_frame != NULL) {
                ret = -1;
            }
        }
        if (ret == 0) {
            if (picoquic_queue_misc_frame(cnx, mf, sizeof(mf), 0, picoquic_packet_context_initial) != 0 ||
                picoquic_queue_misc_frame(cnx, mf, sizeof(mf), 0, picoquic_packet_context_initial) != 0) {
                ret = -1;
            }
            else {
                picoquic_delete_misc_or_dg(&cnx->first_misc_frame, &cnx->last_misc_frame, cnx->last_misc_frame);
                if (cnx->first_misc_frame == NULL || cnx->first_misc_frame->next_misc_frame != NULL) {
                    ret = -1;
                }
                picoquic_purge_misc_frames_after_ready(cnx);
            }
        }
    }

    /* Activate a connection so all data are properly initialized */
    if (ret == 0) {
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }
    if (ret == 0) {
        cnx_s = test_ctx->cnx_server;
    }
    /* Test a series of getter interfaces */

    if (ret == 0 &&
        picoquic_get_local_if_index(cnx) != cnx->path[0]->first_tuple->if_index) {
        ret = -1;
    }

    if (ret == 0) {
        picoquic_connection_id_t cid = picoquic_get_local_cnxid(cnx);

        if (cid.id_len != cnx->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len ||
            memcmp(cid.id, cnx->path[0]->first_tuple->p_local_cnxid->cnx_id.id, cid.id_len) != 0) {
            ret = -1;
        }
    }

    if (ret == 0) {
        picoquic_connection_id_t cid = picoquic_get_remote_cnxid(cnx);

        if (cid.id_len != cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len ||
            memcmp(cid.id, cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id, cid.id_len) != 0) {
            ret = -1;
        }
    }

    if (ret == 0) {
        picoquic_connection_id_t cid = picoquic_get_initial_cnxid(cnx);

        if (cid.id_len != cnx->initial_cnxid.id_len ||
            memcmp(cid.id, cnx->initial_cnxid.id, cid.id_len) != 0) {
            ret = -1;
        }
    }

    if (ret == 0) {
        picoquic_connection_id_t cid = picoquic_get_client_cnxid(cnx);
        if (cid.id_len != cnx->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len ||
            memcmp(cid.id, cnx->path[0]->first_tuple->p_local_cnxid->cnx_id.id, cid.id_len) != 0) {
            ret = -1;
        }
        else {
            picoquic_connection_id_t cid_s = picoquic_get_client_cnxid(cnx_s);
            if (cid.id_len != cid_s.id_len ||
                memcmp(cid.id, cid_s.id, cid.id_len) != 0) {
                ret = -1;
            }
        }
    }

    if (ret == 0) {
        picoquic_connection_id_t cid = picoquic_get_server_cnxid(cnx);
        if (cid.id_len != cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len ||
            memcmp(cid.id, cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id, cid.id_len) != 0) {
            ret = -1;
        }
        else {
            picoquic_connection_id_t cid_s = picoquic_get_server_cnxid(cnx_s);
            if (cid.id_len != cid_s.id_len ||
                memcmp(cid.id, cid_s.id, cid.id_len) != 0) {
                ret = -1;
            }
        }
    }

    if (ret == 0) {
        uint32_t r_padding_multiple = 256;
        uint32_t r_padding_minsize = 55;
        uint32_t padding_multiple = 0;
        uint32_t padding_minsize = 0;

        picoquic_cnx_set_padding_policy(cnx, r_padding_multiple, r_padding_minsize);
        picoquic_cnx_get_padding_policy(cnx, &padding_multiple, &padding_minsize);
        if (padding_multiple != r_padding_multiple ||
            padding_minsize != r_padding_minsize) {
            ret = -1;
        }
    }

    if (ret == 0) {
        picoquic_spinbit_version_enum spin = picoquic_spinbit_random;
        picoquic_cnx_set_spinbit_policy(cnx, spin);
        if (cnx->spin_policy != spin) {
            ret = -1;
        }
    }

    if (ret == 0) {
        if (picoquic_is_sslkeylog_enabled(test_ctx->qclient) != test_ctx->qclient->enable_sslkeylog) {
            ret = -1;
        }
    }

    if (ret == 0) {
        if (!picoquic_is_handshake_error(PICOQUIC_TLS_HANDSHAKE_FAILED) ||
            !picoquic_is_handshake_error(PICOQUIC_TRANSPORT_CRYPTO_ERROR(123)) ||
            picoquic_is_handshake_error(PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR)) {
            ret = -1;
        }
    }
    /* set the algorithm list to the complete value before the alogorithm set/get test
    * Hopefully, nobody is going to call their algorithm "wuovipfwds".
     */
    picoquic_register_all_congestion_control_algorithms();
    if (ret == 0) {
        char const* alg_name[] = {
            "reno", "cubic", "dcubic", "fast", "bbr", "prague", "bbr1", "wuovipfwds", NULL
        };
        picoquic_congestion_algorithm_t const* alg[] = {
            picoquic_newreno_algorithm, picoquic_cubic_algorithm, picoquic_dcubic_algorithm,
            picoquic_fastcc_algorithm, picoquic_bbr_algorithm, picoquic_prague_algorithm,
            picoquic_bbr1_algorithm, NULL, NULL
        };
        size_t nb_alg = sizeof(alg_name) / sizeof(char const*);

        for (size_t i = 0; i < nb_alg && ret == 0; i++) {
            if (picoquic_get_congestion_algorithm(alg_name[i]) != alg[i]) {
                ret = -1;
            }
        }
    }

    if (ret == 0) {
        picoquic_set_default_congestion_algorithm_by_name(test_ctx->qclient, "dcubic");
        if (test_ctx->qclient->default_congestion_alg != picoquic_dcubic_algorithm) {
            ret = -1;
        }
    }

    if (ret == 0) {
        uint64_t l_timer = 10000000;
        uint64_t r_timer = cnx->path[0]->retransmit_timer;
        cnx->idle_timeout = 0;
        cnx->local_parameters.max_idle_timeout = r_timer / 500;
        picoquic_enable_keep_alive(cnx, 0);
        if (cnx->keep_alive_interval == 0 ||
            cnx->keep_alive_interval >= 3 * cnx->path[0]->retransmit_timer) {
            ret = -1;
        }
        else {
            picoquic_enable_keep_alive(cnx, l_timer);
            if (cnx->keep_alive_interval != l_timer) {
                ret = -1;
            }
            else {
                picoquic_disable_keep_alive(cnx);
                if (cnx->keep_alive_interval != 0) {
                    ret = -1;
                }
            }
        }
    }

    if (ret == 0) {
        uint64_t app_error = 0x12345678abcdefull;
        cnx->remote_application_error = app_error;
        if (picoquic_get_application_error(cnx) != app_error) {
            ret = -1;
        }
        cnx->remote_application_error = 0;
    }

    if (ret == 0) {
        if (picoquic_get_remote_stream_error(cnx, UINT32_MAX) != 0) {
            ret = -1;
        }
        else {
            uint8_t data[] = { 1, 2, 3, 4 };
            ret = picoquic_add_to_stream(cnx, 0, data, sizeof(data), 0);
            if (ret == 0) {
                uint64_t app_error = 0x12345678abcdefull;
                picoquic_stream_head_t* stream = picoquic_find_stream(cnx, 0);
                if (stream == NULL) {
                    ret = -1;
                }
                else {
                    stream->remote_error = app_error;
                    if (picoquic_get_remote_stream_error(cnx, 0) != app_error) {
                        ret = -1;
                    }
                }
            }
        }
    }

    if (ret == 0 &&
        (ret = picoquic_adjust_max_connections(test_ctx->qserver, 4)) == 0) {
        if (test_ctx->qserver->tentative_max_number_connections != 4) {
            ret = -1;
        }
    }

    if (ret == 0 &&
        picoquic_current_number_connections(test_ctx->qserver) != 1) {
        ret = -1;
    }

    if (ret == 0 &&
        picoquic_get_default_crypto_epoch_length(test_ctx->qserver) !=
        test_ctx->qserver->crypto_epoch_length_max) {
        ret = -1;
    }

    if (ret == 0) {
        picoquic_set_crypto_epoch_length(cnx, 0);
        if (cnx->crypto_epoch_length_max != PICOQUIC_DEFAULT_CRYPTO_EPOCH_LENGTH ||
            picoquic_get_crypto_epoch_length(cnx) != PICOQUIC_DEFAULT_CRYPTO_EPOCH_LENGTH) {
            ret = -1;
        }
    }

    if (ret == 0 &&
        picoquic_get_local_cid_length(test_ctx->qserver) !=
        test_ctx->qserver->local_cnxid_length) {
        ret = -1;
    }

    if (ret == 0) {
        picoquic_connection_id_t cid = { { 1, 2, 3}, 3 };
        if (picoquic_is_local_cid(test_ctx->qclient, &cid) ||
            !picoquic_is_local_cid(test_ctx->qclient, &cnx->path[0]->first_tuple->p_local_cnxid->cnx_id)) {
            ret = -1;
        }
    }

    if (ret == 0) {
        uint32_t max_simul_log = 17;
        picoquic_set_max_simultaneous_logs(test_ctx->qserver, max_simul_log);
        if (picoquic_get_max_simultaneous_logs(test_ctx->qserver) != max_simul_log) {
            ret = -1;
        }
    }


    if (ret == 0) {
        uint32_t max_half_open = 17;
        picoquic_set_max_half_open_retry_threshold(test_ctx->qserver, max_half_open);
        if (picoquic_get_max_half_open_retry_threshold(test_ctx->qserver) != max_half_open) {
            ret = -1;
        }
    }

    if (ret == 0 && picoquic_register_cnx_id(test_ctx->qclient, cnx, cnx->path[0]->first_tuple->p_local_cnxid) == 0) {
        /* Should be already registered ! */
        ret = -1;
    }

    if (ret == 0){
        if (cnx->registered_icid_addr.ss_family == 0 &&
            picoquic_register_net_icid(cnx) != 0) {
            ret = -1;
        }
        else if (picoquic_register_net_icid(cnx) == 0) {
            /* Should be already registered ! */
            ret = -1;
        }
    }

    if (ret == 0 && picoquic_get_quic_ctx(NULL) != NULL) {
        ret = -1;
    }

    if (ret == 0) {
        uint64_t wake_time = picoquic_get_next_wake_time(test_ctx->qclient, UINT64_MAX);

        if (wake_time > 2 && picoquic_get_earliest_cnx_to_wake(test_ctx->qclient, wake_time / 2) != NULL) {
            ret = -1;
        }
    }

    if (ret == 0) {
        int64_t delay = 1000;
        uint64_t test_time = cnx->next_wake_time - delay;
        int64_t sooner = delay / 2;

        if (picoquic_get_wake_delay(cnx, test_time, INT64_MAX) != delay) {
            ret = -1;
        }
        else if (picoquic_get_wake_delay(cnx, test_time, sooner) != sooner) {
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
fn getter() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;

    let mut cid_bytes = [0u8; 8];
    cid_bytes[0] = 0x9e;
    cid_bytes[1] = 0x77;
    cid_bytes[2] = 0xe8;
    let initial_cid = ConnectionId::clone_from_slice(&cid_bytes).expect("initial CID");

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        Some(&initial_cid),
    )
    .expect("test context");

    // set_default_connection_id_length: 255 → CnxidCheck error;
    //                                     5 → CannotChangeActiveContext error.
    assert_eq!(
        test_ctx.qserver.set_default_connection_id_length(255),
        Err(Error::Protocol(InternalError::CnxidCheck as u64)),
        "255 should be CNXID_CHECK error"
    );
    assert_eq!(
        test_ctx.qclient.set_default_connection_id_length(5),
        Err(Error::Protocol(
            InternalError::CannotChangeActiveContext as u64
        )),
        "5 on live ctx should be CANNOT_CHANGE_ACTIVE_CONTEXT"
    );

    // default_connection_id_ttl getter.
    assert_eq!(
        test_ctx.qserver.default_connection_id_ttl(),
        test_ctx.qserver.local_connection_id_ttl,
        "default_connection_id_ttl should expose quic.local_connection_id_ttl"
    );

    // default_tp getter.
    assert!(
        core::ptr::eq(test_ctx.qserver.default_tp(), &test_ctx.qserver.default_tp),
        "default_tp should borrow quic.default_tp"
    );

    // set_cwin_max(0) should saturate to u64::MAX; restore afterwards.
    {
        let old_max = test_ctx.qserver.cwin_max();
        test_ctx.qserver.set_cwin_max(0);
        assert_eq!(
            test_ctx.qserver.cwin_max(),
            u64::MAX,
            "cwin_max 0 → u64::MAX"
        );
        test_ctx.qserver.set_cwin_max(old_max);
    }

    // find_path_by_address: look up the client's own peer address.
    {
        let peer = test_ctx.cnx_client().peer_addr();
        let mut partial = 0i32;
        let path_id = test_ctx
            .cnx_client()
            .find_path_by_address(None, Some(&peer), &mut partial);
        assert_eq!(path_id, 0, "path 0 should match peer addr");
        assert_ne!(partial, 0, "partial_match should be set");
    }

    // set_local_addr: first call succeeds, second fails.
    {
        let client_addr = test_ctx.client_addr;
        assert!(
            test_ctx.cnx_client().set_local_addr(&client_addr).is_ok(),
            "first set_local_addr should succeed"
        );
        assert!(
            test_ctx.cnx_client().set_local_addr(&client_addr).is_err(),
            "second set_local_addr should fail (already set)"
        );
        // Reset via set_local_addr with zeroed address (matches C memset).
        let zero: core::net::SocketAddr = "0.0.0.0:0".parse().unwrap();
        let _ = test_ctx.cnx_client().set_local_addr(&zero);
    }

    // queue_misc_frame: the Rust slice API cannot express the C SIZE_MAX
    // invalid-length case without an explicit test hook. The valid queue,
    // purge, delete-last, and singleton cases still mirror the C test.
    {
        let mf = [crate::frames::FrameType::MaxStreamsBidir as u8, 0x41, 0];
        assert!(
            test_ctx
                .cnx_client()
                .queue_misc_frame(&mf, false, PacketContext::Initial)
                .is_ok(),
            "queue_misc_frame should succeed"
        );
        test_ctx.cnx_client().purge_misc_frames_after_ready();
        assert!(
            !test_ctx.cnx_client().has_misc_frames(),
            "misc frames should be empty after purge"
        );

        // Queue two, delete last, one remains as singleton.
        assert!(
            test_ctx
                .cnx_client()
                .queue_misc_frame(&mf, false, PacketContext::Initial)
                .is_ok()
        );
        assert!(
            test_ctx
                .cnx_client()
                .queue_misc_frame(&mf, false, PacketContext::Initial)
                .is_ok()
        );
        test_ctx.cnx_client().delete_last_misc_frame();
        assert!(
            test_ctx.cnx_client().has_misc_frames(),
            "one frame should remain"
        );
        assert!(
            test_ctx.cnx_client().misc_frames_is_singleton(),
            "exactly one frame should remain"
        );
        test_ctx.cnx_client().purge_misc_frames_after_ready();
    }

    // Start connection and run handshake.
    test_ctx.cnx_client().start_client().expect("start client");
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    // local_if_index.
    {
        let cnx = test_ctx.cnx_client();
        let expected_if_index = cnx.paths[0].tuples[0].if_index as u32;
        assert_eq!(
            cnx.local_if_index(),
            expected_if_index,
            "local_if_index should expose path[0].tuple[0].if_index"
        );
    }

    // local_connection_id, remote_connection_id, initial_connection_id.
    let (client_local_cid, client_remote_cid) = {
        let cnx = test_ctx.cnx_client();
        let local_cid = first_path_local_cid(cnx);
        let remote_cid = first_path_remote_cid(cnx);
        assert_eq!(
            cnx.local_cnxid().as_bytes(),
            local_cid.as_bytes(),
            "local_cnxid should match path[0].tuple[0].local_connection_id"
        );
        assert_eq!(
            cnx.remote_connection_id().as_bytes(),
            remote_cid.as_bytes(),
            "remote_connection_id should match path[0].tuple[0].remote_connection_id"
        );
        assert_eq!(
            cnx.initial_connection_id().as_bytes(),
            cnx.initial_connection_id.as_bytes(),
            "initial_connection_id should expose cnx.initial_connection_id"
        );
        (local_cid, remote_cid)
    };

    // client_connection_id matches the client CID on both sides.
    let client_cid = {
        let cnx = test_ctx.cnx_client();
        let cid = cnx.client_connection_id();
        assert_eq!(
            cid.as_bytes(),
            client_local_cid.as_bytes(),
            "client_connection_id on client should match its local path CID"
        );
        cid
    };
    let client_cid_s = {
        let cnx_s = test_ctx.cnx_server();
        let server_remote_cid = first_path_remote_cid(cnx_s);
        let cid = cnx_s.client_connection_id();
        assert_eq!(
            cid.as_bytes(),
            server_remote_cid.as_bytes(),
            "client_connection_id on server should match its remote path CID"
        );
        cid
    };
    assert_eq!(client_cid.as_bytes(), client_cid_s.as_bytes());

    // server_connection_id matches the server CID on both sides.
    let server_cid = {
        let cnx = test_ctx.cnx_client();
        let cid = cnx.server_connection_id();
        assert_eq!(
            cid.as_bytes(),
            client_remote_cid.as_bytes(),
            "server_connection_id on client should match its remote path CID"
        );
        cid
    };
    let server_cid_s = {
        let cnx_s = test_ctx.cnx_server();
        let server_local_cid = first_path_local_cid(cnx_s);
        let cid = cnx_s.server_connection_id();
        assert_eq!(
            cid.as_bytes(),
            server_local_cid.as_bytes(),
            "server_connection_id on server should match its local path CID"
        );
        cid
    };
    assert_eq!(server_cid.as_bytes(), server_cid_s.as_bytes());

    // set/get padding policy.
    {
        let (r_mult, r_min) = (256u32, 55u32);
        test_ctx.cnx_client().set_padding_policy(r_mult, r_min);
        let (got_mult, got_min) = test_ctx.cnx_client().padding_policy();
        assert_eq!(got_mult, r_mult, "padding_multiple");
        assert_eq!(got_min, r_min, "padding_minsize");
    }

    // set/get spinbit policy.
    {
        test_ctx
            .cnx_client()
            .set_cnx_spinbit_policy(SpinbitVersion::Random);
        assert_eq!(
            test_ctx.cnx_client().cnx_spinbit_policy(),
            SpinbitVersion::Random
        );
    }

    // is_sslkeylog_enabled getter.
    assert_eq!(
        test_ctx.qclient.is_sslkeylog_enabled(),
        test_ctx.qclient.enable_sslkeylog,
        "is_sslkeylog_enabled should expose qclient.enable_sslkeylog"
    );

    // is_handshake_error.
    assert!(
        !is_handshake_error(InternalError::AeadCheck as u64),
        "AEAD check is not a handshake error"
    );
    assert!(
        is_handshake_error(transport_crypto_error(0) as u64),
        "CRYPTO_ERROR codes are handshake errors"
    );
    assert!(
        is_handshake_error(TransportError::TlsHandshakeFailed as u64),
        "TLS handshake failure is a handshake error"
    );
    assert!(
        is_handshake_error(transport_crypto_error(123) as u64),
        "CRYPTO_ERROR alert is a handshake error"
    );
    assert!(
        !is_handshake_error(TransportError::FrameFormatError as u64),
        "frame-format error is not a handshake error"
    );

    // Congestion-algorithm registry.
    register_all_congestion_control_algorithms();
    {
        let alg_cases = [
            ("reno", Some(("reno", 1))),
            ("cubic", Some(("cubic", 2))),
            ("dcubic", Some(("dcubic", 3))),
            ("fast", Some(("fast", 4))),
            ("bbr", Some(("bbr", 5))),
            ("prague", Some(("prague", 6))),
            ("bbr1", Some(("bbr1", 7))),
            ("wuovipfwds", None),
        ];
        for (name, expected) in alg_cases {
            match expected {
                Some((expected_id, expected_number)) => {
                    let alg = get_congestion_algorithm(name).expect("registered algorithm");
                    assert_eq!(alg.congestion_algorithm_id, expected_id);
                    assert_eq!(alg.congestion_algorithm_number, expected_number);
                }
                None => {
                    assert!(
                        get_congestion_algorithm(name).is_none(),
                        "bogus name should not be registered"
                    );
                }
            }
        }
    }

    // set_default_congestion_algorithm_by_name.
    {
        let dcubic = get_congestion_algorithm("dcubic").expect("dcubic");
        test_ctx
            .qclient
            .set_default_congestion_algorithm_by_name("dcubic")
            .expect("set dcubic as default congestion algorithm");
        let selected = test_ctx
            .qclient
            .default_congestion_alg
            .expect("default congestion algorithm");
        assert!(
            core::ptr::eq(selected, dcubic),
            "default congestion algorithm should be dcubic"
        );
    }

    // enable_keep_alive / disable_keep_alive.
    {
        let l_timer = 10_000_000u64;
        let cnx = test_ctx.cnx_client();
        let r_timer = cnx.paths[0].retransmit_timer;
        cnx.idle_timeout = Duration::from_ticks(0);
        cnx.local_parameters.max_idle_timeout = Duration::from_ticks(r_timer.ticks() / 500);
        cnx.enable_keep_alive(Duration::from_ticks(0));
        assert_ne!(
            cnx.keep_alive_interval.ticks(),
            0,
            "zero keep-alive should auto-compute a nonzero interval"
        );
        assert!(
            cnx.keep_alive_interval.ticks() < 3 * r_timer.ticks(),
            "auto-computed keep-alive should be below 3*rto"
        );
        cnx.enable_keep_alive(Duration::from_ticks(l_timer));
        assert_eq!(cnx.keep_alive_interval.ticks(), l_timer);
        cnx.disable_keep_alive();
        assert_eq!(cnx.keep_alive_interval.ticks(), 0);
    }

    // application_error getter.
    {
        let app_error = 0x12345678abcdefu64;
        let cnx = test_ctx.cnx_client();
        cnx.remote_application_error = app_error;
        assert_eq!(
            cnx.remote_application_error(),
            app_error,
            "remote_application_error should expose cnx.remote_application_error"
        );
        cnx.remote_application_error = 0;
        assert_eq!(
            cnx.remote_stream_error(u32::MAX as u64),
            0,
            "missing stream should report no remote stream error"
        );

        let data = [1u8, 2, 3, 4];
        cnx.add_to_stream(0, &data, false).expect("add stream data");
        cnx.set_stream_remote_error(0, app_error);
        assert_eq!(
            cnx.remote_stream_error(0),
            app_error,
            "remote_stream_error round-trip"
        );
    }

    // adjust_max_connections / current_number_connections.
    test_ctx
        .qserver
        .adjust_max_connections(4)
        .expect("adjust_max_connections");
    assert_eq!(test_ctx.qserver.tentative_max_number_connections, 4);
    assert_eq!(test_ctx.qserver.current_number_connections(), 1);

    // default_crypto_epoch_length / set_crypto_epoch_length.
    assert_eq!(
        test_ctx.qserver.default_crypto_epoch_length(),
        test_ctx.qserver.crypto_epoch_length_max
    );
    {
        let cnx = test_ctx.cnx_client();
        cnx.set_crypto_epoch_length(0);
        assert_eq!(
            cnx.crypto_epoch_length_max, DEFAULT_CRYPTO_EPOCH_LENGTH,
            "zero crypto epoch length should reset to the default"
        );
        assert_eq!(
            cnx.crypto_epoch_length(),
            DEFAULT_CRYPTO_EPOCH_LENGTH,
            "crypto_epoch_length getter should expose the reset default"
        );
    }

    // local_cid_length / is_local_cid.
    assert_eq!(
        test_ctx.qserver.local_cid_length(),
        test_ctx.qserver.local_connection_id_length
    );
    {
        let fake = ConnectionId::clone_from_slice(&[1u8, 2, 3]).expect("fake cid");
        let real = first_path_local_cid(test_ctx.cnx_client());
        assert!(!test_ctx.qclient.is_local_cid(&fake));
        assert!(
            test_ctx.qclient.is_local_cid(&real),
            "client path local CID should be registered in qclient"
        );
    }

    // max_simultaneous_logs / set_max_simultaneous_logs.
    test_ctx.qserver.set_max_simultaneous_logs(17);
    assert_eq!(test_ctx.qserver.max_simultaneous_logs(), 17);

    // set_max_half_open_retry_threshold / max_half_open_retry_threshold.
    test_ctx.qserver.set_max_half_open_retry_threshold(17);
    assert_eq!(test_ctx.qserver.max_half_open_retry_threshold(), 17);

    // register_net_icid: re-registration should fail.
    {
        let was_unregistered =
            crate::socket_addr_is_unspecified(&test_ctx.cnx_client().registered_icid_addr);
        if was_unregistered {
            assert!(
                test_ctx.cnx_client().register_net_icid().is_ok(),
                "first register_net_icid should succeed when the ICID is unregistered"
            );
        }
        assert!(
            test_ctx.cnx_client().register_net_icid().is_err(),
            "second register_net_icid should fail"
        );
    }

    // next_wake_time / earliest_cnx_to_wake.
    {
        let wake = test_ctx.qclient.next_wake_time(simulated_time);
        if wake > 2 {
            let half = Instant::from_ticks(wake / 2);
            assert!(
                test_ctx.qclient.earliest_cnx_to_wake(half).is_none(),
                "no cnx should wake before wake/2"
            );
        }
    }

    // wake_delay.  The `next_wake_time` on Connection is a public field.
    {
        let delay = 1000i64;
        let next_wake_ticks = test_ctx.cnx_client().next_wake_time.ticks();
        let test_time = Instant::from_ticks(next_wake_ticks.saturating_sub(delay as u64));
        assert_eq!(
            test_ctx.cnx_client().wake_delay(test_time, i64::MAX),
            delay,
            "wake_delay full"
        );
        assert_eq!(
            test_ctx.cnx_client().wake_delay(test_time, delay / 2),
            delay / 2,
            "wake_delay capped"
        );
    }

    // register_cnx_id: re-registering an already-registered CID should fail.
    {
        let token = test_ctx.cnx_client().own_token.expect("client token");
        let mut cnx = test_ctx
            .qclient
            .connections
            .remove(token)
            .expect("client connection");
        let local_cid_token = cnx.paths[0].tuples[0]
            .local_connection_id
            .expect("path 0 local CID token");
        let local_cid = cnx
            .local_connection_ids
            .get(local_cid_token)
            .expect("path 0 local CID");
        let mut duplicate_lcid = LocalConnectionId {
            connection_by_id_membership: local_cid.connection_by_id_membership,
            path_id: local_cid.path_id,
            sequence: local_cid.sequence,
            create_time: local_cid.create_time,
            connection_id: local_cid.connection_id,
            is_acked: local_cid.is_acked,
        };
        assert!(
            test_ctx
                .qclient
                .register_cnx_id(&mut cnx, &mut duplicate_lcid)
                .is_err(),
            "register_cnx_id should fail for an already registered local CID"
        );
    }

    // get_quic_ctx(NULL): Rust maps the nullable C connection pointer to Option.
    let null_cnx: Option<&mut Connection> = None;
    assert!(
        null_cnx.is_none(),
        "get_quic_ctx(NULL) maps to None in the Rust API"
    );
}
```

## `picoquictest/multipath_test.c:multipath_quality_test`
* C test-table name: `multipath_quality`
* C entry function: `multipath_quality_test`
* Rust test: `multipath_quality`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1676-1676`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The no-op C body shown is Win32-only; the in-scope non-Win32 C branch runs Quality with 1000000 us and callback-log comparison, which Rust maps. It still inherits the weak Rust scenario verifier that omits C completion and timing checks.
* Phase 5A fix note: Keep the non-Win32 Quality scenario, but repair the shared Rust verifier to check scenario completion and max_completion_microsec.
* Phase 5B analysis: C test is a no-op returning success, so the Rust test should be a harness-runnable no-op; prior handshake failure was runtime/library behavior, not a Phase 5B block.
* Phase 5B fix note: Changed multipath_quality to an empty #[test] body instead of invoking multipath_test_one(...Quality).

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Current Rust test body
```rust
fn multipath_quality() {}
```

## `picoquictest/skip_frame_test.c:app_message_overflow_test`
* C test-table name: `app_message_overflow`
* C entry function: `app_message_overflow_test`
* Rust test: `app_message_overflow`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:4226-4272`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only logs oversized app messages and does not reproduce the C overflow fixture or validation. The C test uses a BYTESTREAM_MAX_BUFFER_SIZE buffer with a terminal '!', logs suffixes at offsets 15..0, closes the connection, converts the binlog to qlog, and compares against the reference qlog. Rust uses a much larger 65533-byte string, omits the '!' pattern, does not convert to qlog, and does not compare the generated output to the reference.
* Phase 5A fix note: Use the same BYTESTREAM_MAX_BUFFER_SIZE message pattern and offsets as C, ensure the binlog is actually emitted and flushed, convert the expected binlog to qlog, and compare with app_msg_overflow_ref.qlog.
* Phase 5B analysis: Reclassified as a Phase 5B test-correspondence issue: runtime binlog wiring/timestamp divergence is Phase 5C, while the Rust test now exercises the C-level logging/delete contract and keeps the qlog reference assertion.
* Phase 5B fix note: Changed app_message_overflow to use cnx.log_new_connection(), cnx.log_app_message(...) for the 16 overflow suffixes, and cnx.delete()/drop before qlog conversion instead of direct Binlog helper calls.

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    picoquic_cnx_t* cnx = NULL;
    struct sockaddr_storage addr;
    const picoquic_connection_id_t initial_cid = { { 8, 9, 0, 1, 2, 3, 4, 5 }, 8 };
    const picoquic_connection_id_t dest_cid = { { 16, 17, 18, 19, 20, 21, 22, 23 }, 8 };
    char qlog_test_ref[512];
    int ret_qlog = picoquic_get_input_path(qlog_test_ref, sizeof(qlog_test_ref),
        picoquic_solution_dir, QLOG_OVERFLOW_REF);
    picoquic_quic_t* quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    if (quic == NULL || ret_qlog != 0) {
        ret = -1;
    }
    else {
        picoquic_set_binlog(quic, ".");
        ret = picoquic_store_text_addr(&addr, "10.0.0.1", 1234);
        if (ret == 0) {
            cnx = picoquic_create_cnx(quic, initial_cid, dest_cid, (struct sockaddr*) & addr,
                simulated_time, 0, "test-sni", "test-alpn", 1);
            if (cnx == NULL) {
                ret = -1;
            }
            else {
                picoquic_log_new_connection(cnx);
            }
        }
    }
    if (ret == 0) {
        char test[BYTESTREAM_MAX_BUFFER_SIZE];

        memset(test, 'x', sizeof(test) - 3);
        test[sizeof(test) - 3] = '!';
        test[sizeof(test) - 2] = 0;

        for (int i = 0; i < 16; i++) {
            picoquic_log_app_message(cnx, "s:%s", &test[15 - i]);
        }

        picoquic_delete_cnx(cnx);
    }

    picoquic_free(quic);

    if (ret == 0) {
        /* Convert to QLOG and verify */
        uint64_t log_time = 0;
        uint16_t flags = 0;
        FILE* f_binlog = picoquic_open_cc_log_file_for_read(qlog_overflow_bin, &flags, &log_time);

        if (f_binlog == NULL) {
            DBG_PRINTF("Cannot open binlog file: %s.", qlog_overflow_bin);
            ret = -1;
        }
        else {
            ret = qlog_convert(&initial_cid, f_binlog, qlog_overflow_file, NULL, ".", flags);
            if (ret != 0) {
                DBG_PRINTF("%s", "Cannot convert the binary log into QLOG.\n");
            }
            else {
                ret = picoquic_test_compare_text_files(qlog_overflow_file, qlog_test_ref);
                if (ret != 0) {
                    DBG_PRINTF("%s", "Unexpected content in QLOG log file.\n");
                }
            }
            picoquic_file_close(f_binlog);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn app_message_overflow() {
    use crate::ConnectionId as InternalCid;

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    quic.set_binlog(Some(".")).ok();
    let _ = std::fs::remove_file(QLOG_OVERFLOW_BIN);
    let _ = std::fs::remove_file(QLOG_OVERFLOW_FILE);

    let initial_cid =
        InternalCid::clone_from_slice(&[8, 9, 0, 1, 2, 3, 4, 5]).expect("initial CID");
    let dest_cid =
        InternalCid::clone_from_slice(&[16, 17, 18, 19, 20, 21, 22, 23]).expect("dest CID");
    let addr: core::net::SocketAddr = "10.0.0.1:1234".parse().unwrap();

    let mut cnx = quic
        .create_connection_with_cids(
            initial_cid,
            dest_cid,
            Some(&addr),
            simulated_time,
            "test-sni",
            "test-alpn",
        )
        .expect("cnx");

    cnx.log_new_connection();

    let mut test = [0u8; BYTESTREAM_MAX_BUFFER_SIZE];
    test[..BYTESTREAM_MAX_BUFFER_SIZE - 3].fill(b'x');
    test[BYTESTREAM_MAX_BUFFER_SIZE - 3] = b'!';
    test[BYTESTREAM_MAX_BUFFER_SIZE - 2] = 0;
    for i in 0..16usize {
        let suffix = core::str::from_utf8(&test[15 - i..BYTESTREAM_MAX_BUFFER_SIZE - 2])
            .expect("overflow message is ascii");
        cnx.log_app_message(&format!("s:{}", suffix));
    }
    cnx.delete();
    drop(cnx);
    drop(quic);

    convert_overflow_binlog_to_qlog(QLOG_OVERFLOW_BIN, QLOG_OVERFLOW_FILE, &initial_cid)
        .expect("convert overflow binlog to qlog");
    super::util::compare_text_files(QLOG_OVERFLOW_FILE, QLOG_OVERFLOW_REF)
        .expect("app message overflow qlog matches reference");
}
```

## `picoquictest/skip_frame_test.c:send_stream_blocked_test`
* C test-table name: `send_stream_blocked`
* C entry function: `send_stream_blocked_test`
* Rust test: `send_stream_blocked`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:3993-3999`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust helper contains the same three case values, but the test hardcodes NB_CASES to 4 and the helper indexes with modulo, so it reruns the first case instead of faithfully iterating the C table once; the comment is placeholder-like.
* Phase 5A fix note: Loop over the actual Rust case table length exactly once, remove modulo cycling and the hardcoded representative count, and keep the per-case blocked-frame assertions.
* Phase 5B analysis: Rust now matches the C table loop: each stream-blocked case is run exactly once with no modulo cycling.
* Phase 5B fix note: Promoted the three stream-blocked cases to a shared Rust table, changed the helper to take a case reference, and iterated that table in the test.

### C test body
```c
{
    int ret = 0;

    for (size_t i = 0; ret == 0 && i < nb_stream_blocked_test; i++) {
        if ((ret = send_stream_blocked_test_one(&stream_blocked_test[i])) != 0) {
            DBG_PRINTF("Stream blocked test %d failed", (int)i);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn send_stream_blocked() {
    for (i, case) in STREAM_BLOCKED_TEST_CASES.iter().enumerate() {
        send_stream_blocked_test_one(case).unwrap_or_else(|err| {
            panic!("stream blocked case {i} failed: {err:?}");
        });
    }
}
```
