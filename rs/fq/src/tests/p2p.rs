//! Test case for `picoquictest/p2p_test.c`.
//!
//! Tests the address-discovery extension: verifies that a client
//! can learn its own public address from the server.

#![allow(non_snake_case)]

use super::util::{
    TEST_ALPN, TEST_SNI, TestApiStreamDesc, test_api_init_send_recv_scenario,
    tls_api_connection_loop, tls_api_data_sending_loop, tls_api_one_scenario_body_verify,
    tls_api_one_scenario_init_ex,
};
use crate::ConnectionId;
use crate::internal::Version;

const TEST_SCENARIO_ADDRESS_DISCOVERY: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 100_000,
}];

/// C: `address_discovery_test`.
#[test]
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
