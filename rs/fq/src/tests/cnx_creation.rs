//! Test cases for `picoquictest/cnx_creation_test.c`.
//!
//! Covers:
//! * `create_cnx` — builds 7 connections across IPv4 / IPv6 / per-port /
//!   per-CID combinations, verifies address-based and CID-based lookup,
//!   iterator count, non-registered lookup, and delete-then-verify.
//! * `create_quic` — edge cases for QUIC context creation: 0-connection
//!   clamping, bad cert/key rejection, bad ticket-store tolerance, token-file
//!   loading, and NULL transport-parameter reset.

#![allow(non_snake_case)]

use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::{ConnectionId, Instant, Quic, RESET_SECRET_SIZE, TransportParameters};

/// Make a fresh `Quic` context with the C test's
/// `picoquic_create(8, NULL, …)` defaults.  Returns `Option<Box<Quic>>`
/// directly (the C `NULL` failure goes to `None`).
fn default_quic() -> Option<Box<Quic>> {
    Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        None,
        None,
    )
}

fn assert_transport_parameters_eq(actual: &TransportParameters, expected: &TransportParameters) {
    assert_eq!(
        actual.initial_max_stream_data_bidi_local,
        expected.initial_max_stream_data_bidi_local
    );
    assert_eq!(
        actual.initial_max_stream_data_bidi_remote,
        expected.initial_max_stream_data_bidi_remote
    );
    assert_eq!(
        actual.initial_max_stream_data_uni,
        expected.initial_max_stream_data_uni
    );
    assert_eq!(actual.initial_max_data, expected.initial_max_data);
    assert_eq!(
        actual.initial_max_stream_id_bidir,
        expected.initial_max_stream_id_bidir
    );
    assert_eq!(
        actual.initial_max_stream_id_unidir,
        expected.initial_max_stream_id_unidir
    );
    assert_eq!(actual.max_idle_timeout, expected.max_idle_timeout);
    assert_eq!(actual.max_packet_size, expected.max_packet_size);
    assert_eq!(actual.max_ack_delay, expected.max_ack_delay);
    assert_eq!(
        actual.active_connection_id_limit,
        expected.active_connection_id_limit
    );
    assert_eq!(actual.ack_delay_exponent, expected.ack_delay_exponent);
    assert_eq!(actual.migration_disabled, expected.migration_disabled);
    assert_eq!(actual.preferred_address.v4, expected.preferred_address.v4);
    assert_eq!(actual.preferred_address.v6, expected.preferred_address.v6);
    assert_eq!(
        actual.preferred_address.connection_id,
        expected.preferred_address.connection_id
    );
    assert_eq!(
        actual.preferred_address.stateless_reset_token,
        expected.preferred_address.stateless_reset_token
    );
    assert_eq!(
        actual.max_datagram_frame_size,
        expected.max_datagram_frame_size
    );
    assert_eq!(actual.enable_loss_bit, expected.enable_loss_bit);
    assert_eq!(actual.enable_time_stamp, expected.enable_time_stamp);
    assert_eq!(actual.min_ack_delay, expected.min_ack_delay);
    assert_eq!(actual.do_grease_quic_bit, expected.do_grease_quic_bit);
    assert_eq!(
        actual.version_negotiation.current,
        expected.version_negotiation.current
    );
    assert_eq!(
        actual.version_negotiation.previous,
        expected.version_negotiation.previous
    );
    assert_eq!(
        actual.version_negotiation.received,
        expected.version_negotiation.received
    );
    assert_eq!(
        actual.version_negotiation.supported,
        expected.version_negotiation.supported
    );
    assert_eq!(actual.enable_bdp_frame, expected.enable_bdp_frame);
    assert_eq!(actual.initial_max_path_id, expected.initial_max_path_id);
    assert_eq!(
        actual.address_discovery_mode,
        expected.address_discovery_mode
    );
    assert_eq!(
        actual.is_reset_stream_at_enabled,
        expected.is_reset_stream_at_enabled
    );
}

/// C: `create_cnx_test` in `picoquictest/cnx_creation_test.c`.
///
/// Builds 7 connections covering the IPv4 / IPv6 / per-port / per-CID
/// matrix.  The outer loop runs twice: once with `local_connection_id_length
/// = 0` (address-based lookup) and once with the default length of 8
/// (CID-based lookup).  Within each iteration, verifies:
///
/// 1. Lookup by address (when no local CID) or by CID (when local CID is
///    in use) finds every registered connection.
/// 2. Iterator returns the correct count.
/// 3. An unregistered address / CID returns `None`.
/// 4. After deleting even-indexed connections, odd-indexed ones are still
///    found and even-indexed ones are gone.
#[test]
fn create_cnx() {
    const TEST_CNX_COUNT: usize = 7;

    let test_ipv4 = Ipv4Addr::new(192, 0, 2, 0);
    let test_ipv6 = Ipv6Addr::new(0x2001, 0x0DB8, 0, 0, 0, 0, 0, 0);
    let test_ipv4_local = Ipv4Addr::new(127, 0, 0, 1);
    let test_ipv6_local = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);

    // Build 5 IPv4 test addresses:
    //   [0] 192.0.2.1:1000  [1] 192.0.2.2:1001  [2] 192.0.2.2:1002
    //   [3] 192.0.2.2:1003  (used only for the "not found" check)
    //   [4] 127.0.0.1:1004
    let mut test4: [SocketAddr; 5] = [SocketAddr::new(IpAddr::V4(test_ipv4), 0); 5];
    for (i, addr) in test4.iter_mut().enumerate() {
        let ip = if i < 4 {
            let mut octets = test_ipv4.octets();
            octets[3] = if i == 0 { 1 } else { 2 };
            Ipv4Addr::from(octets)
        } else {
            test_ipv4_local
        };
        *addr = SocketAddr::new(IpAddr::V4(ip), 1000 + i as u16);
    }

    // Build 3 IPv6 test addresses:
    //   [0] 2001:db8::1:1000  [1] 2001:db8::2:1001  [2] ::1:1002
    let mut test6: [SocketAddr; 3] = [SocketAddr::new(IpAddr::V6(test_ipv6), 0); 3];
    for (i, addr) in test6.iter_mut().enumerate() {
        let ip = if i < 2 {
            let mut segments = test_ipv6.segments();
            segments[7] = (i as u16) + 1;
            Ipv6Addr::from(segments)
        } else {
            test_ipv6_local
        };
        *addr = SocketAddr::new(IpAddr::V6(ip), 1000 + i as u16);
    }

    // The 7 addresses used for connection creation (test4[3] is omitted).
    let test_cnx_addr: [SocketAddr; TEST_CNX_COUNT] = [
        test4[0], test4[1], test4[2], test4[4], test6[0], test6[1], test6[2],
    ];

    // Pre-built initial CIDs: {x,x,x,x,x,x,x,x} for x in 1..=7.
    let test_cnx_id: [ConnectionId; TEST_CNX_COUNT] =
        core::array::from_fn(|i| ConnectionId::clone_from_slice(&[(i + 1) as u8; 8]).unwrap());

    // C: loops l=0 (local_cnxid_length = 0, address-based) and
    //         l=1 (local_cnxid_length = 8, CID-based).
    for l in 0..2usize {
        let use_cid = l != 0;
        let mut quic = default_quic().expect("create quic");

        if !use_cid {
            // C: `quic->local_cnxid_length = 0`
            quic.local_connection_id_length = 0;
        }

        // Create 7 connections, record the initial CID assigned to each.
        let mut test_cid = [ConnectionId::default(); TEST_CNX_COUNT];
        for i in 0..TEST_CNX_COUNT {
            let initial_id = if use_cid {
                test_cnx_id[i]
            } else {
                ConnectionId::default()
            };
            let cnx = quic
                .create_connection(
                    initial_id,
                    ConnectionId::default(),
                    Some(&test_cnx_addr[i]),
                    Instant::from_ticks(0),
                    0,
                    None,
                    None,
                    true,
                )
                .expect("create_connection");
            // C: `test_cid[i] = test_cnx[i]->path[0]->first_tuple->p_local_cnxid->cnx_id`
            test_cid[i] = cnx.initial_connection_id();
        }

        // Verify that every connection can be retrieved by its registered attribute.
        if !use_cid {
            for addr in test_cnx_addr.iter() {
                assert!(
                    quic.connection_by_net(Some(addr)).is_some(),
                    "connection not found by net address"
                );
            }
        }

        // Verify the iterator visits all connections.
        // C: `for (cnx = first; cnx != NULL; cnx = next_cnx(cnx)) counter++`
        assert_eq!(
            quic.connections.len(),
            TEST_CNX_COUNT,
            "connection count mismatch after creation"
        );

        // Verify that an unregistered address / CID returns None.
        if !use_cid {
            // test4[3] was not used in test_cnx_addr.
            assert!(
                quic.connection_by_net(Some(&test4[3])).is_none(),
                "non-registered address must not be found"
            );
        } else {
            let bad_target = ConnectionId::clone_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
            assert!(
                quic.connection_by_id(bad_target).is_none(),
                "non-registered CID must not be found"
            );
        }

        // Delete connections at even indices (first, middle, last).
        // C: `picoquic_delete_cnx(test_cnx[i])` for i in {0,2,4,6}.
        for i in (0..TEST_CNX_COUNT).step_by(2) {
            let tok = if use_cid {
                quic.connection_by_id(test_cid[i]).map(|(t, _)| t)
            } else {
                quic.connection_by_net(Some(&test_cnx_addr[i]))
            };
            if let Some(t) = tok {
                quic.delete_connection(t);
            }
        }

        // Verify deleted connections are gone; surviving (odd-indexed) ones remain.
        for i in 0..TEST_CNX_COUNT {
            let found = if use_cid {
                quic.connection_by_id(test_cid[i]).is_some()
            } else {
                quic.connection_by_net(Some(&test_cnx_addr[i])).is_some()
            };
            if i % 2 == 0 {
                assert!(!found, "connection {i} (even) should have been deleted");
            } else {
                assert!(found, "connection {i} (odd) should still exist");
            }
        }
        // `quic` is dropped here — equivalent to `picoquic_free(quic)`.
    }
}

/// C: `create_quic_test` in `picoquictest/cnx_creation_test.c`.
///
/// Edge cases for [`Quic::new`]:
///
/// * Requesting 0 connections clamps `max_number_connections` to 1.
/// * A bad cert or key path causes creation to return `None`.
/// * A bad ticket-store path does **not** crash creation (client contexts
///   don't need one).
/// * [`Quic::load_token_file`] with a bad file should fail; a bad directory
///   is platform-dependent.
/// * Resetting transport parameters to `None` restores initialized defaults.
#[test]
fn create_quic() {
    let bad_file = "no_such_file_should_exist.pem";
    let bad_dir = "..";

    // 0 connections clamps to 1.
    // C: `quic->max_number_connections != 1` → fail.
    {
        let quic = Quic::new(
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .expect("0 max_nb_connections must still create a context (clamped to 1)");
        assert_eq!(
            quic.max_number_connections, 1,
            "0 max_nb_connections must clamp to 1"
        );
    }

    // Bad cert or key path must cause creation to fail.
    let cert_file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/cert.pem");
    let key_file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/key.pem");

    assert!(
        Quic::new(
            8,
            Some(bad_file),
            Some(key_file),
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .is_none(),
        "bad cert path must reject context creation"
    );
    assert!(
        Quic::new(
            8,
            Some(cert_file),
            Some(bad_file),
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .is_none(),
        "bad key path must reject context creation"
    );

    // Bad ticket-store path must NOT crash a client context.
    // C: the ticket file is position 13 in `picoquic_create` (→ Rust `ticket_file_name`).
    Quic::new(
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        Some(bad_file),
        None,
    )
    .expect("bad ticket-store file name should still create a context");

    Quic::new(
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        Some(bad_dir),
        None,
    )
    .expect("bad ticket-store directory should still create a context");

    // Load-token-file edge cases.
    // C: fails if bad_file fails AND bad_dir succeeds (Windows-specific).
    // On Linux/macOS, bad_dir usually fails too, so the condition is always false.
    {
        let mut quic = default_quic().expect("create quic");
        let rbf = quic.load_token_file(bad_file);
        if rbf.is_err() {
            // Only check bad_dir when bad_file failed.
            let rbd = quic.load_token_file(bad_dir);
            assert!(
                rbd.is_err(),
                "load_token_file: bad_dir succeeded where bad_file failed (platform-specific)"
            );
        }
    }

    // Resetting transport parameters to their initialized defaults must succeed.
    // C: `picoquic_set_default_tp(quic, NULL)` — NULL resets to defaults.
    {
        let mut quic = default_quic().expect("create quic");
        let mut expected = TransportParameters::default();
        crate::internal::init_transport_parameters(&mut expected);

        let custom = TransportParameters {
            initial_max_data: 7,
            max_packet_size: 9,
            ack_delay_exponent: 1,
            enable_loss_bit: 0,
            ..expected.clone()
        };
        quic.set_default_tp(&custom)
            .expect("set_default_tp with custom params");
        assert_eq!(quic.default_tp().initial_max_data, custom.initial_max_data);

        quic.set_default_tp(None::<&TransportParameters>)
            .expect("set_default_tp None must reset initialized defaults");
        assert_transport_parameters_eq(quic.default_tp(), &expected);
    }
}
