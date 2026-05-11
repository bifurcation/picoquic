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

## `picoquictest/cpu_limited.c:limited_bbr_test`
* C test-table name: `limited_bbr`
* C entry function: `limited_bbr_test`
* Rust test: `limited_bbr`
* C source: `picoquictest/cpu_limited.c:230-238`
* Rust source: `rs/fq/src/tests/cpu_limited.rs:178-183`

### C test body
```c
{
    limited_test_config_t config;
    limited_config_set_default(&config, 3);
    config.ccalgo = picoquic_bbr_algorithm;
    config.max_completion_time = 4100000;

    return limited_client_test_one(&config);
}
```

### Rust test body
```rust
fn limited_bbr() {
    let mut config = limited_config_default(3);
    config.ccalgo = get_congestion_algorithm("bbr").expect("bbr algo");
    config.max_completion_time = 4_100_000;
    limited_client_test_one(config);
}
```

## `picoquictest/edge_cases.c:ecf1_final_loss_test`
* C test-table name: `ecf1_final_loss`
* C entry function: `ecf1_final_loss_test`
* Rust test: `ecf1_final_loss`
* C source: `picoquictest/edge_cases.c:445-489`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1155-1172`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t final_losses = 0xb10;
    uint8_t test_case_id = 0xf1;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 0, &simulated_time, 0, 20);
    uint64_t zero_loss_mask = 0;

    /* Finish the connection */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &zero_loss_mask, 0, &simulated_time);
        if (ret != 0)
        {
            DBG_PRINTF("Connect loop returns %d\n", ret);
        }
    }
    /* Finish sending data */
    test_ctx->immediate_exit = 1;

    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &zero_loss_mask, &simulated_time, 0);

        if (ret != 0)
        {
            DBG_PRINTF("Data sending loop returns %d\n", ret);
        }
    }
    /* Simulate losses during closing */
    if (ret == 0) {
        ret = tls_api_close_with_losses(test_ctx, &simulated_time, final_losses);
    }

    if (ret == 0 && simulated_time > 10000000) {
        DBG_PRINTF("Took %" PRIu64 "us to complete, too long", simulated_time);
        ret = -1;
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn ecf1_final_loss() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0xf1, false, &mut simulated_time, 0, 20).expect("edge_case_prepare");
    let mut zero_loss = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut zero_loss, 0, &mut simulated_time)
        .expect("connection loop");
    test_ctx.immediate_exit = true;
    tls_api_data_sending_loop(&mut test_ctx, &mut zero_loss, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0xb10)
        .expect("close with losses");
    assert!(
        simulated_time.ticks() <= 10_000_000,
        "connection close took too long: {} µs",
        simulated_time.ticks()
    );
}
```

## `picoquictest/edge_cases.c:reset_extra_stop_test`
* C test-table name: `reset_extra_stop`
* C entry function: `reset_extra_stop_test`
* Rust test: `reset_extra_stop`
* C source: `picoquictest/edge_cases.c:1213-1216`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1361-1363`

### C test body
```c
{
    return reset_repeat_test_one(reset_extra_stop_sending);
}
```

### Rust test body
```rust
fn reset_extra_stop() {
    reset_repeat_test_one(ResetTestKind::ExtraStop).expect("reset_extra_stop");
}
```

## `picoquictest/getter_test.c:getter_test`
* C test-table name: `getter`
* C entry function: `getter_test`
* Rust test: `getter`
* C source: `picoquictest/getter_test.c:39-426`
* Rust source: `rs/fq/src/tests/getter.rs:18-334`

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

### Rust test body
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
    let ttl = test_ctx.qserver.default_connection_id_ttl();
    assert_eq!(
        test_ctx.qserver.default_connection_id_ttl(),
        ttl,
        "default_connection_id_ttl getter is stable"
    );

    // default_tp getter.
    let _tp = test_ctx.qserver.default_tp();

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

    // queue_misc_frame: SIZE_MAX invalid (skip — usize::MAX slice impossible in safe Rust);
    // valid frame queues, then purge_misc_frames_after_ready empties the list.
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
    let _if_idx = test_ctx.cnx_client().local_if_index();

    // local_connection_id, remote_connection_id, initial_connection_id.
    let local_cid = test_ctx.cnx_client().local_connection_id();
    let remote_cid = test_ctx.cnx_client().remote_connection_id();
    let initial_cid2 = test_ctx.cnx_client().initial_connection_id();
    assert!(!local_cid.is_empty());
    assert!(!remote_cid.is_empty());
    assert!(!initial_cid2.is_empty());

    // client_connection_id matches on both sides.
    let client_cid = test_ctx.cnx_client().client_connection_id();
    let client_cid_s = test_ctx.cnx_server().client_connection_id();
    assert_eq!(client_cid.as_bytes(), client_cid_s.as_bytes());

    // server_connection_id matches on both sides.
    let server_cid = test_ctx.cnx_client().server_connection_id();
    let server_cid_s = test_ctx.cnx_server().server_connection_id();
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
    let _ssl = test_ctx.qclient.is_sslkeylog_enabled();

    // is_handshake_error.
    assert!(
        is_handshake_error(crate::errors::InternalError::AeadCheck as u64),
        "AEAD check is a handshake error"
    );
    // A transport-frame error is not a handshake error.
    assert!(
        !is_handshake_error(crate::errors::InternalError::InvalidFrame as u64),
        "frame-format error is not a handshake error"
    );

    // Congestion-algorithm registry.
    register_all_congestion_control_algorithms();
    {
        let alg_names = ["reno", "cubic", "dcubic", "fast", "bbr", "prague", "bbr1"];
        for name in alg_names {
            assert!(
                get_congestion_algorithm(name).is_some(),
                "algorithm '{name}' should be registered"
            );
        }
        assert!(
            get_congestion_algorithm("wuovipfwds").is_none(),
            "bogus name should not be registered"
        );
    }

    // set_default_congestion_algorithm_by_name.
    {
        let dcubic = get_congestion_algorithm("dcubic").expect("dcubic");
        let _ = test_ctx
            .qclient
            .set_default_congestion_algorithm_by_name("dcubic");
        let _ = dcubic;
    }

    // enable_keep_alive / disable_keep_alive.
    {
        let l_timer = 10_000_000u64;
        test_ctx.cnx_client().enable_keep_alive(
            crate::Duration::from_ticks(0), // 0 → auto-compute from retransmit_timer
        );
        test_ctx
            .cnx_client()
            .enable_keep_alive(crate::Duration::from_ticks(l_timer));
        test_ctx.cnx_client().disable_keep_alive();
    }

    // application_error getter.
    {
        let app_error = 0x12345678abcdefu64;
        test_ctx.cnx_client().set_stream_remote_error(u64::MAX, 0); // noop for nonexistent stream
        let _ = test_ctx.cnx_client().remote_stream_error(u32::MAX as u64);
        // Queue data on stream 0 then inject remote error.
        let data = [1u8, 2, 3, 4];
        let _ = test_ctx.cnx_client().add_to_stream(0, &data, false);
        test_ctx.cnx_client().set_stream_remote_error(0, app_error);
        assert_eq!(
            test_ctx.cnx_client().remote_stream_error(0),
            app_error,
            "remote_stream_error round-trip"
        );
    }

    // adjust_max_connections / current_number_connections.
    test_ctx
        .qserver
        .adjust_max_connections(4)
        .expect("adjust_max_connections");
    assert_eq!(test_ctx.qserver.current_number_connections(), 1);

    // default_crypto_epoch_length / set_crypto_epoch_length.
    let _epoch = test_ctx.qserver.default_crypto_epoch_length();
    test_ctx.cnx_client().set_crypto_epoch_length(0);
    let _ = test_ctx.cnx_client().crypto_epoch_length();

    // local_cid_length / is_local_cid.
    let _lcl = test_ctx.qserver.local_cid_length();
    {
        let fake = ConnectionId::clone_from_slice(&[1u8, 2, 3]).expect("fake cid");
        assert!(!test_ctx.qclient.is_local_cid(&fake));
    }

    // max_simultaneous_logs / set_max_simultaneous_logs.
    test_ctx.qserver.set_max_simultaneous_logs(17);
    assert_eq!(test_ctx.qserver.max_simultaneous_logs(), 17);

    // set_max_half_open_retry_threshold / max_half_open_retry_threshold.
    test_ctx.qserver.set_max_half_open_retry_threshold(17);
    assert_eq!(test_ctx.qserver.max_half_open_retry_threshold(), 17);

    // register_cnx_id: re-registering an already-registered CID should fail.
    {
        let local_cid_registered = test_ctx.cnx_client().local_connection_id();
        // Create a LocalConnectionId wrapper and attempt re-register.
        // In C: picoquic_register_cnx_id(qclient, cnx, cnx->path[0]->...) == 0 → failure expected.
        // In Rust: just assert that registering an existing CID returns an error.
        let _ = local_cid_registered; // Phase 4: wire register_cnx_id
    }

    // register_net_icid: re-registration should fail.
    {
        let ret = test_ctx.cnx_client().register_net_icid();
        // Second call should fail; first might succeed or fail depending on state.
        // C: first call to register_net_icid on a non-registered ICID succeeds.
        let _ = ret;
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
}
```

## `picoquictest/high_latency_test.c:high_latency_probeRTT_test`
* C test-table name: `high_latency_probeRTT`
* C entry function: `high_latency_probeRTT_test`
* Rust test: `high_latency_probertt`
* C source: `picoquictest/high_latency_test.c:330-339`
* Rust source: `rs/fq/src/tests/high_latency.rs:318-334`

### C test body
```c
{
    /* Simple test. */
    uint64_t latency = 5000000;
    uint64_t expected_completion = 839000000;

    return high_latency_one(0xf1, picoquic_bbr_algorithm,
        hilat_scenario_100mb, sizeof(hilat_scenario_100mb),
        expected_completion, latency, 1, 1, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn high_latency_probertt() {
    let latency = 5_000_000u64;
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    high_latency_one(
        0xf1,
        bbr,
        HILAT_SCENARIO_100MB,
        839_000_000,
        latency,
        1,
        1,
        0,
        false,
        false,
        false,
    );
}
```
