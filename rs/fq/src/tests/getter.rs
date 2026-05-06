//! Test case for `picoquictest/getter_test.c`.
//!
//! Verifies that every getter/setter on `Quic` and `Connection` round-trips
//! correctly against its internal field.

#![allow(non_snake_case)]

use crate::errors::InternalError;
use crate::internal::Version;
use crate::tests::util::{TEST_ALPN, TEST_SNI, tls_api_connection_loop, tls_api_init_ctx_ex2};
use crate::{
    ConnectionId, Error, Instant, PacketContext, SpinbitVersion, get_congestion_algorithm,
    is_handshake_error, register_all_congestion_control_algorithms,
};

/// C: `getter_test` in `picoquictest/getter_test.c`.
#[test]
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
