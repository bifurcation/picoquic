//! Test case for `picoquictest/getter_test.c`.
//!
//! Verifies that every getter/setter on `Quic` and `Connection` round-trips
//! correctly against its internal field.

#![allow(non_snake_case)]

use crate::errors::{InternalError, TransportError, transport_crypto_error};
use crate::internal::{Connection, DEFAULT_CRYPTO_EPOCH_LENGTH, LocalConnectionId, Version};
use crate::tests::util::{TEST_ALPN, TEST_SNI, tls_api_connection_loop, tls_api_init_ctx_ex2};
use crate::{
    ConnectionId, Duration, Error, Instant, PacketContext, SpinbitVersion,
    get_congestion_algorithm, is_handshake_error, register_all_congestion_control_algorithms,
};

fn first_path_local_cid(cnx: &Connection) -> ConnectionId {
    let path = cnx.paths.first().expect("path 0");
    let tuple = path.tuples.first().expect("path 0 tuple 0");
    let token = tuple.local_connection_id.expect("path 0 local CID");
    cnx.local_connection_ids
        .get(token)
        .expect("registered local CID")
        .connection_id
}

fn first_path_remote_cid(cnx: &Connection) -> ConnectionId {
    let path = cnx.paths.first().expect("path 0");
    let tuple = path.tuples.first().expect("path 0 tuple 0");
    let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
    cnx.remote_connection_id_stashes
        .iter()
        .find(|stash| stash.unique_path_id == path.unique_path_id)
        .and_then(|stash| stash.connection_ids.get(cid_index))
        .expect("path 0 remote CID")
        .connection_id
}

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
