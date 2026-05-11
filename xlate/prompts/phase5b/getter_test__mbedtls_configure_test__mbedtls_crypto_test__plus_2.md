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

Owned Rust test file(s): `rs/fq/src/tests/getter.rs`, `rs/fq/src/tests/mbedtls.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/getter_test.c:getter_test`
* C test-table name: `getter`
* C entry function: `getter_test`
* Rust test: `getter`
* Expected Rust file: `rs/fq/src/tests/getter.rs`
* Rust span: `rs/fq/src/tests/getter.rs:40-536`
* Phase 5A analysis: Rust is only a partial/weak translation: several C checks are tautologies, no-op reads, skipped cases, or placeholders, and some assertions use different error codes or omit expected field comparisons.
* Phase 5A fix note: Restore C-equivalent assertions for default TTL/TP, local interface and CID getters, sslkeylog, exact handshake error codes, congestion algorithm identity/default update, keep-alive intervals, application error getter, remote stream missing-stream result, max-connection tentative value, crypto epoch defaults, local CID true case, register_cnx_id, register_net_icid first-call behavior, get_quic_ctx(NULL), and the invalid queue_misc_frame length case where an API/test hook is needed.
* Phase 5C outcome: blocked
* Phase 5C analysis: Most getter checks are present, but the C SIZE_MAX queue_misc_frame overflow case still has no Rust API/test hook equivalent; the congestion alias check also expects id "reno" instead of the C-equivalent newreno algorithm identity.
* Phase 5C fix note: Expose a test-only or low-level misc-frame API that can pass an explicit oversized length and assert failure; change the reno alias expectation to the newreno identity/id.

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

## `picoquictest/mbedtls_test.c:mbedtls_configure_test`
* C test-table name: `mbedtls_configure`
* C entry function: `mbedtls_configure_test`
* Rust test: `mbedtls_configure`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Rust span: `rs/fq/src/tests/mbedtls.rs:722-752`
* Phase 5A analysis: Rust does not check the mbedTLS provider registry; it checks constants, local arrays, and direct helper behavior that can pass without proving mbedTLS registration.
* Phase 5A fix note: Add registry-level assertions for mbedTLS high/low cipher suites, secp256r1/x25519 key exchanges, secp256r1 special slot, private-key/cert/random provider hooks; gate or assert mbedTLS availability as appropriate.
* Phase 5C outcome: blocked
* Phase 5C analysis: Current Rust test remains weaker than C: it checks constants and helper behavior, not mbedTLS provider identity in the TLS registry. The needed provider-identity surface is private or absent from mbedtls.rs.
* Phase 5C fix note: Expose or relocate test-only registry checks for mbedTLS cipher suites, key exchanges, secp256r1 slot, private-key/cert/random/verifier hooks.

### C test body
```c
{
    int ret = 0;
    int cipher_suite_match_low = 0;
    int cipher_suite_match_high = 0;
    int key_exchange_max = 0;
    ptls_cipher_suite_t* targets[3] = {
        &ptls_mbedtls_aes128gcmsha256,
        &ptls_mbedtls_aes256gcmsha384,
        &ptls_mbedtls_chacha20poly1305sha256
    };
    ptls_key_exchange_algorithm_t* exchange[3] = {
        &ptls_mbedtls_secp256r1, &ptls_mbedtls_x25519 };

    /* Cleanup previous initiation of the TLS API and do it cleanly. */
    picoquic_tls_api_reset(TLS_API_INIT_FLAGS_NO_OPENSSL |
        TLS_API_INIT_FLAGS_NO_FUSION);
    /* Verify that the negotiated parameters have the expected value */
    for (int i = 0; i < PICOQUIC_CIPHER_SUITES_NB_MAX; i++) {
        for (int j = 0; j < 3; j++) {
            if (targets[j] == picoquic_cipher_suites[i].high_memory_suite) {
                cipher_suite_match_high |= (1 << j);
            }
            if (targets[j] == picoquic_cipher_suites[i].low_memory_suite) {
                cipher_suite_match_low |= (1 << j);
            }
        }
        if (cipher_suite_match_low == 0x7 && cipher_suite_match_high == 0x7) {
            break;
        }
    }
    if (cipher_suite_match_low != 0x7 || cipher_suite_match_high != 0x7) {
        DBG_PRINTF("Suites registration test fails, expected 0x%x, 0x%x, got 0x%x, 0x%x",
            7, 7, cipher_suite_match_low, cipher_suite_match_high);
        ret = -1;
    }

    if (picoquic_key_exchange_secp256r1[0] != &ptls_mbedtls_secp256r1) {
        DBG_PRINTF("%s", "key_exchange_secp256r1 does not match");
        ret = -1;
    }

    for (int i = 0; i < PICOQUIC_KEY_EXCHANGES_NB_MAX; i++) {
        for (int j = 0; j < 2; j++) {
            if (exchange[j] == picoquic_key_exchanges[i]) {
                key_exchange_max |= (1 << j);
            }
            if (key_exchange_max == 0x3) {
                break;
            }
        }
    }

    if (key_exchange_max != 0x3) {
        DBG_PRINTF("Exchange registration test fails, expected 0x%x, got 0x%x",
            7, key_exchange_max);
        ret = -1;
    }

    if (picoquic_set_private_key_from_file_fn != ptls_mbedtls_load_private_key ||
        picoquic_dispose_sign_certificate_fn != ptls_mbedtls_dispose_sign_certificate ||
        picoquic_get_certs_from_file_fn != picoquic_mbedtls_get_certs_from_file) {
        DBG_PRINTF("%s", "At least one private key function does not match mbedtls");
        ret = -1;
    }

    if (picoquic_get_certificate_verifier_fn != picoquic_mbedtls_get_certificate_verifier ||
        picoquic_dispose_certificate_verifier_fn != ptls_mbedtls_dispose_verify_certificate) {
        DBG_PRINTF("%s", "At least one verify certs function does not match mbedtls");
        ret = -1;
    }

    if (picoquic_crypto_random_provider_fn != ptls_mbedtls_random_bytes) {
        DBG_PRINTF("%s", "Crypto random provider does not match mbedtls");
        ret = -1;
    }

    /* Reset configuration to default after test */
    picoquic_tls_api_reset(0);

    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls_configure() {
    // Bring up the TLS provider registry with only the mbedTLS backend.
    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL | TLS_API_INIT_FLAGS_NO_FUSION);

    let cipher_suites = [
        crate::AES_128_GCM_SHA256,
        crate::AES_256_GCM_SHA384,
        crate::CHACHA20_POLY1305_SHA256,
    ];
    assert_eq!(
        cipher_suites,
        [0x1301, 0x1302, 0x1303],
        "cipher suite registration values"
    );

    let key_exchanges = [crate::GROUP_SECP256R1, 29u16];
    assert!(key_exchanges.contains(&crate::GROUP_SECP256R1));
    assert!(key_exchanges.contains(&29u16));

    mbedtls_test_random().expect("registered random provider");
    mbedtls_test_load_one_der_key("certs/rsa/key.pem").expect("registered private key loader");
    mbedtls_test_sign_verify_one(
        "certs/rsa/key.pem",
        "certs/rsa/cert.pem",
        "certs/test-ca.crt",
        "rsa.test.example.com",
    )
    .expect("registered certificate verifier");

    reset_tls_api(0);
}
```

## `picoquictest/mbedtls_test.c:mbedtls_crypto_test`
* C test-table name: `mbedtls_crypto`
* C entry function: `mbedtls_crypto_test`
* Rust test: `mbedtls_crypto`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Rust span: `rs/fq/src/tests/mbedtls.rs:630-639`
* Phase 5A analysis: Rust top-level shape matches, but the helpers mostly self-check Rust crypto or deterministic stand-ins instead of initializing mbedTLS and comparing mbedTLS against minicrypto for hash, label, ciphers, AEAD, and key exchange.
* Phase 5A fix note: Replace placeholder/self-comparison helpers with real provider comparison semantics, including PSA/mbedTLS init/free, cipher/AEAD output comparisons, hash reset cases, and key-exchange failure/abort checks.
* Phase 5C outcome: blocked
* Phase 5C analysis: Final tree still lacks real Rust mbedTLS provider/test-harness surface for PSA init/free/random, raw hash/cipher/AEAD provider comparison, and provider key exchange; current Rust helpers remain deterministic stand-ins.
* Phase 5C fix note: Expose/implement the real mbedTLS provider surfaces, then replace stand-in helpers with mbedTLS-vs-minicrypto checks.

### C test body
```c
{
    ptls_cipher_algorithm_t* cipher_test[5] = {
        &ptls_mbedtls_aes128ecb,
        &ptls_mbedtls_aes128ctr,
        &ptls_mbedtls_aes256ecb,
        &ptls_mbedtls_aes256ctr,
        &ptls_mbedtls_chacha20
    };
    ptls_cipher_algorithm_t* cipher_ref[5] = {
        &ptls_minicrypto_aes128ecb,
        &ptls_minicrypto_aes128ctr,
        &ptls_minicrypto_aes256ecb,
        &ptls_minicrypto_aes256ctr,
        &ptls_minicrypto_chacha20
    };
    int ret = 0;

    /* Initialize the PSA crypto library. */
    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        ret = test_random();
        DBG_PRINTF("test random returns: %d\n", ret);

        if (ret == 0) {
            ret = test_hash(&ptls_mbedtls_sha256, &ptls_minicrypto_sha256);
            DBG_PRINTF("test hash returns: %d\n", ret);
        }

        if (ret == 0) {
            ret = test_label(&ptls_mbedtls_sha256, &ptls_minicrypto_sha256);
            DBG_PRINTF("test label returns: %d\n", ret);
        }

        if (ret == 0) {
            for (int i = 0; i < 5; i++) {
                if (test_cipher(cipher_test[i], cipher_ref[i]) != 0) {
                    DBG_PRINTF("test cipher %d fails\n", i);
                    ret = -1;
                }
            }
            DBG_PRINTF("test ciphers returns: %d\n", ret);
        }

        if (ret == 0) {
            ret = test_aead(&ptls_mbedtls_aes128gcm, &ptls_mbedtls_sha256, &ptls_minicrypto_aes128gcm, &ptls_minicrypto_sha256);
            DBG_PRINTF("test aeads returns: %d\n", ret);
        }

        if (ret == 0) {
            ret = test_key_exchange(&ptls_mbedtls_secp256r1, &ptls_minicrypto_secp256r1);
            if (ret != 0) {
                DBG_PRINTF("%s", "test key exchange secp256r1 mbedtls to minicrypto fails\n");
            }
            else {
                ret = test_key_exchange(&ptls_minicrypto_secp256r1, &ptls_mbedtls_secp256r1);
                if (ret != 0) {
                    DBG_PRINTF("%s", "test key exchange secp256r1 minicrypto to mbedtls fails\n");
                }
            }
            ret = test_key_exchange(&ptls_mbedtls_x25519, &ptls_minicrypto_x25519);
            if (ret != 0) {
                DBG_PRINTF("%s", "test key exchange x25519 mbedtls to minicrypto fails\n");
            }
            else {
                ret = test_key_exchange(&ptls_minicrypto_x25519, &ptls_mbedtls_x25519);
                if (ret != 0) {
                    DBG_PRINTF("%s", "test key exchange x25519 minicrypto to mbedtls fails\n");
                }
            }
            DBG_PRINTF("test key exchange returns: %d\n", ret);
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }
    return (ret == 0) ? 0 : -1;
}
```

### Current Rust test body
```rust
fn mbedtls_crypto() {
    // Initialise the PSA/mbedTLS library.
    mbedtls_test_random().expect("test_random");
    mbedtls_test_hash().expect("test_hash");
    mbedtls_test_label().expect("test_label");
    mbedtls_test_ciphers().expect("test_ciphers");
    mbedtls_test_aead().expect("test_aead");
    mbedtls_test_key_exchange().expect("test_key_exchange");
    // PSA/mbedTLS library is de-initialised by the helper internals.
}
```

## `picoquictest/mbedtls_test.c:mbedtls_load_key_test`
* C test-table name: `mbedtls_load_key`
* C entry function: `mbedtls_load_key_test`
* Rust test: `mbedtls_load_key`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Rust span: `rs/fq/src/tests/mbedtls.rs:644-652`
* Phase 5A analysis: Rust names the same six key fixtures, but the helper is placeholder-like: it checks PEM text and hashes bytes instead of initializing mbedTLS/PSA, loading the private key into a ptls context, requiring sign_certificate, and invoking the mbedTLS signer.
* Phase 5A fix note: Replace the Rust helper with a real mbedTLS-backed load-and-sign check, including init/free behavior or the Rust provider equivalent, while keeping the six positive key cases.
* Phase 5C outcome: blocked
* Phase 5C analysis: The Rust test has the mbedTLS-only reset guard and the six key fixtures, but mbedtls_test_load_one_der_key is still a PEM-text/SHA256 placeholder. The current Rust tree has no faithful mbedTLS load-and-sign surface: MbedTls private-key loading is registered but install_private_key_from_active_provider returns Err for MbedTls, and no sign_certificate-style harness is exposed.
* Phase 5C fix note: Provide a real Rust mbedTLS private-key/sign harness or provider equivalent, then make the helper load each key and perform the signing check instead of hashing PEM bytes.

### C test body
```c
{
    int ret = 0;


    /* Initialize the PSA crypto library. */
    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_RSA_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP256R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP384R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP521R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP256R1_PKCS8_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_RSA_PKCS8_KEY);
        }
#if 0
        /* Commenting out ED25519 for now, probably not supported yet in MBEDTLS/PSA */
        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_ED25519_KEY);
        }
#endif
        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }

    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls_load_key() {
    let _tls_api_reset = TlsApiResetGuard::mbedtls_only();
    mbedtls_test_load_one_der_key("certs/rsa/key.pem").expect("rsa key");
    mbedtls_test_load_one_der_key("certs/secp256r1/key.pem").expect("secp256r1 key");
    mbedtls_test_load_one_der_key("certs/secp384r1/key.pem").expect("secp384r1 key");
    mbedtls_test_load_one_der_key("certs/secp521r1/key.pem").expect("secp521r1 key");
    mbedtls_test_load_one_der_key("certs/secp256r1-pkcs8/key.pem").expect("secp256r1 pkcs8 key");
    mbedtls_test_load_one_der_key("certs/rsa-pkcs8/key.pem").expect("rsa pkcs8 key");
}
```

## `picoquictest/mbedtls_test.c:mbedtls_sign_verify_test`
* C test-table name: `mbedtls_sign_verify`
* C entry function: `mbedtls_sign_verify_test`
* Rust test: `mbedtls_sign_verify`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Rust span: `rs/fq/src/tests/mbedtls.rs:680-716`
* Phase 5A analysis: Rust covers the same five fixture paths and names, but the helper is placeholder-like: it only checks PEM markers and compares a synthetic hash to itself, not real mbedTLS init, key loading, certificate verification, selected signature algorithm, or signature verification.
* Phase 5A fix note: Make the Rust helper exercise the real translated mbedTLS/picotls sign-and-verify path for all five fixtures, including init/free, key/cert/CA loading, server-name verification, signing the test message, and verifying the signature.
* Phase 5C outcome: blocked
* Phase 5C analysis: Rust covers the five fixture rows, but the helper still only checks PEM text and compares a synthetic hash; the tree has no mbedTLS signer/verifier/server-name signature-verification harness to express the C contract.
* Phase 5C fix note: Expose faithful mbedTLS sign-certificate and certificate-verifier APIs/harness, then replace the synthetic helper.

### C test body
```c
{
    int ret = 0;

    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_RSA_KEY, ASSET_RSA_CERT, ASSET_TEST_CA, ASSET_RSA_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP256R1_KEY, ASSET_SECP256R1_CERT, ASSET_TEST_CA, ASSET_SECP256R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP384R1_KEY, ASSET_SECP384R1_CERT, ASSET_TEST_CA, ASSET_SECP384R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP521R1_KEY, ASSET_SECP521R1_CERT, ASSET_TEST_CA, ASSET_SECP521R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP256R1_PKCS8_KEY, ASSET_SECP256R1_PKCS8_CERT, ASSET_TEST_CA, ASSET_SECP256R1_PKCS8_NAME, 0, 0);
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }
    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls_sign_verify() {
    mbedtls_test_sign_verify_one(
        "certs/rsa/key.pem",
        "certs/rsa/cert.pem",
        "certs/test-ca.crt",
        "rsa.test.example.com",
    )
    .expect("rsa sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1/key.pem",
        "certs/secp256r1/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp384r1/key.pem",
        "certs/secp384r1/cert.pem",
        "certs/test-ca.crt",
        "secp384r1.test.example.com",
    )
    .expect("secp384r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp521r1/key.pem",
        "certs/secp521r1/cert.pem",
        "certs/test-ca.crt",
        "secp521r1.test.example.com",
    )
    .expect("secp521r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1-pkcs8/key.pem",
        "certs/secp256r1-pkcs8/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1-pkcs8 sign_verify");
}
```
