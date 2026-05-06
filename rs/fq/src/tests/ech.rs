//! Test cases for `picoquictest/ech_test.c`.
//!
//! Exercises Encrypted Client Hello (ECH):
//!
//! * `ech_config` / `ech_config_p` — build an ECH config list from a
//!   public-key or private-key file and verify it matches the reference.
//! * `ech_e2e` — end-to-end handshake with ECH enabled, verify success.
//! * `ech_e2e_0rtt` — ECH with GREASE (no config), complete the
//!   connection, obtain a session ticket, and verify 0-RTT on the second
//!   connection.
//! * `ech_grease` — client sends ECH GREASE (no valid config); server
//!   should *not* report `is_ech_handshake`.
//! * `ech_no_ech` — server has no ECH key; client should fall back and
//!   complete the connection; second connection reuses the ticket.

#![allow(non_snake_case)]

use super::util::{
    TEST_ALPN, TEST_ECH_CONFIG, TEST_ECH_CONFIG_REF, TEST_ECH_PRIVATE_KEY, TEST_ECH_PUB_KEY,
    TEST_SNI, TestApiStreamDesc, save_empty_tickets, session_resume_wait_for_ticket,
    test_api_init_send_recv_scenario, tls_api_connection_loop, tls_api_data_sending_loop,
    tls_api_init_ctx_ex, tls_api_one_scenario_body_verify, tls_api_synch_to_empty_loop,
};
use crate::internal::Version;
use crate::{
    ConnectionId, Error, Instant, ech_create_config_from_private_key,
    ech_create_config_from_public_key, ech_read_config, ech_save_config, tls_api_init,
};

// ---------------------------------------------------------------------------
// Constants.

const ECH_CONFIG_FILE: &str = "ech_config.txt";
#[allow(dead_code)]
const ECH_TICKET_FILE: &str = "ech_ticket_store.bin";
const ECH_PUBLIC_NAME: &str = "test.example.com";
const ECH_SCENARIO_SMALL: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 256,
    r_len: 1000,
}];

// ---------------------------------------------------------------------------
// Local helper.

/// Read the ECH config from `ref_file` and assert it byte-matches `config`.
/// C: `ech_test_check_buf` in `picoquictest/ech_test.c`.
fn ech_test_check_buf(config: &[u8], ref_file: &str) -> crate::Result<()> {
    let ref_config = ech_read_config(ref_file)?;
    assert_eq!(
        config,
        ref_config.as_slice(),
        "ECH config does not match reference file {ref_file}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// ECH e2e spec type.  C: `ech_e2e_spec_t`.

/// Parameters for one ECH end-to-end scenario.
/// C: `ech_e2e_spec_t` in `picoquictest/ech_test.c`.
#[derive(Default)]
#[allow(dead_code)]
struct EchE2eSpec {
    /// The handshake should complete with ECH active.
    expect_success: bool,
    /// The client should send ECH GREASE (no valid config supplied).
    expect_grease: bool,
    /// Do not configure the server with an ECH key.
    no_ech_server: bool,
    /// After the handshake, run a small data exchange and save a ticket.
    complete_cnx: bool,
    /// After `complete_cnx`, open a second connection reusing the ticket.
    try_twice: bool,
}

/// Run one ECH end-to-end scenario.
/// C: `ech_e2e_test_one` in `picoquictest/ech_test.c`.
fn ech_e2e_test_one(spec: &EchE2eSpec) {
    ech_e2e_test_one_inner(spec).expect("ech_e2e_test_one");
}

/// Run the post-handshake data exchange, wait for a ticket, verify
/// completion, and persist the ticket store.
/// C: `ech_test_complete_cnx` in `picoquictest/ech_test.c`.
fn ech_test_complete_cnx(
    test_ctx: &mut super::util::TestTlsApiCtx,
    loss_mask: &mut u64,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    test_api_init_send_recv_scenario(test_ctx, ECH_SCENARIO_SMALL)?;
    tls_api_data_sending_loop(test_ctx, loss_mask, simulated_time, 0)?;
    session_resume_wait_for_ticket(test_ctx, simulated_time)?;
    tls_api_one_scenario_body_verify(test_ctx, simulated_time, 1_000_000)?;

    if test_ctx.qclient.stored_tickets.is_empty() {
        return Err(Error::Generic);
    }
    test_ctx
        .qclient
        .save_tickets(*simulated_time, ECH_TICKET_FILE)
}

/// Recreate the client connection, attempt ticket-based resumption,
/// verify that the second handshake used PSK, and drain the simulator.
/// C: `ech_e2e_second` in `picoquictest/ech_test.c`.
fn ech_e2e_second(
    test_ctx: &mut super::util::TestTlsApiCtx,
    ech_config_buf: &[u8],
    loss_mask: &mut u64,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    test_ctx.qclient.connections.clear();
    test_ctx.qserver.connections.clear();

    {
        let cnx = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).ok_or(Error::Generic)?,
                ConnectionId::with_size(0).ok_or(Error::Generic)?,
                Some(&test_ctx.server_addr),
                *simulated_time,
                Version::InternalTest1 as u32,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .ok_or(Error::Generic)?;

        if !ech_config_buf.is_empty() {
            cnx.ech_configure_client(ech_config_buf)?;
        }

        cnx.start_client()?;
    }

    tls_api_connection_loop(test_ctx, loss_mask, 0, simulated_time)?;

    if !test_ctx.cnx_client().tls_is_psk_handshake() {
        return Err(Error::Generic);
    }

    tls_api_synch_to_empty_loop(test_ctx, simulated_time, 2048, 0, 0)
}

fn ech_e2e_test_one_inner(spec: &EchE2eSpec) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut initial_cid_bytes = [0xec, 0x8e, 0x2e, 0, 0, 0, 0, 0];
    initial_cid_bytes[3] = u8::from(spec.expect_success);
    initial_cid_bytes[4] = u8::from(spec.expect_grease);
    let initial_cid = ConnectionId::clone_from_slice(&initial_cid_bytes).ok_or(Error::Generic)?;
    let mut ech_config_buf = Vec::new();

    save_empty_tickets(ECH_TICKET_FILE, simulated_time)?;

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(ECH_TICKET_FILE),
        Some(&initial_cid),
    )
    .ok_or(Error::Generic)?;

    test_ctx.qserver.set_qlog(".")?;
    test_ctx.qserver.use_long_log = true;
    if !spec.no_ech_server {
        test_ctx
            .qserver
            .ech_configure(Some(TEST_ECH_PRIVATE_KEY), Some(TEST_ECH_CONFIG))?;
    }

    if !spec.no_ech_server || spec.expect_grease {
        test_ctx.qclient.ech_configure(None, None)?;
    }

    if !spec.no_ech_server {
        ech_config_buf = ech_read_config(TEST_ECH_CONFIG)?;
        if spec.expect_success {
            test_ctx
                .cnx_client()
                .ech_configure_client(&ech_config_buf)?;
        } else if spec.expect_grease {
            test_ctx.cnx_client().ech_configure_client(&[])?;
        } else {
            return Err(Error::Generic);
        }
    }

    test_ctx.cnx_client().start_client()?;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        return Err(Error::Generic);
    }

    if spec.expect_success {
        if !test_ctx.cnx_client().is_ech_handshake() {
            return Err(Error::Generic);
        }
    } else if test_ctx.cnx_client().is_ech_handshake() {
        return Err(Error::Generic);
    } else {
        let retry_matches = {
            let retry_config = test_ctx.cnx_client().ech_retry_config();
            !ech_config_buf.is_empty() && retry_config == ech_config_buf.as_slice()
        };
        if retry_matches && spec.no_ech_server {
            return Err(Error::Generic);
        }
    }

    if spec.complete_cnx {
        ech_test_complete_cnx(&mut test_ctx, &mut loss_mask, &mut simulated_time)?;
    }

    if spec.try_twice {
        ech_e2e_second(
            &mut test_ctx,
            &ech_config_buf,
            &mut loss_mask,
            &mut simulated_time,
        )?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test entries.

/// Create an ECH config from the test public key, save it, and verify it
/// matches the reference file.
/// C: `ech_config_test` in `picoquictest/ech_test.c`.
#[test]
fn ech_config() {
    tls_api_init();
    let config = ech_create_config_from_public_key(TEST_ECH_PUB_KEY, ECH_PUBLIC_NAME)
        .expect("create ECH config from public key");
    ech_save_config(&config, ECH_CONFIG_FILE).expect("save ECH config");
    ech_test_check_buf(&config, TEST_ECH_CONFIG_REF).expect("config matches reference");
}

/// Create an ECH config from the test private key, save it, and verify it
/// matches the reference file.
/// C: `ech_config_p_test` in `picoquictest/ech_test.c`.
#[test]
fn ech_config_p() {
    tls_api_init();
    let config = ech_create_config_from_private_key(TEST_ECH_PRIVATE_KEY, ECH_PUBLIC_NAME)
        .expect("create ECH config from private key");
    ech_save_config(&config, ECH_CONFIG_FILE).expect("save ECH config");
    ech_test_check_buf(&config, TEST_ECH_CONFIG_REF).expect("config matches reference");
}

/// Verify a full ECH handshake succeeds.
/// C: `ech_e2e_test` in `picoquictest/ech_test.c`.
#[test]
fn ech_e2e() {
    let spec = EchE2eSpec {
        expect_success: true,
        ..Default::default()
    };
    ech_e2e_test_one(&spec);
}

/// Verify ECH with GREASE, then 0-RTT on a second connection.
/// C: `ech_e2e_0rtt_test` in `picoquictest/ech_test.c`.
#[test]
fn ech_e2e_0rtt() {
    let spec = EchE2eSpec {
        expect_grease: true,
        complete_cnx: true,
        try_twice: true,
        ..Default::default()
    };
    ech_e2e_test_one(&spec);
}

/// Verify ECH GREASE (client sends GREASE, ECH should not be reported).
/// C: `ech_grease_test` in `picoquictest/ech_test.c`.
#[test]
fn ech_grease() {
    let spec = EchE2eSpec {
        expect_grease: true,
        ..Default::default()
    };
    ech_e2e_test_one(&spec);
}

/// Verify fallback when the server has no ECH key; second connection reuses ticket.
/// C: `ech_no_ech_test` in `picoquictest/ech_test.c`.
#[test]
fn ech_no_ech() {
    let spec = EchE2eSpec {
        no_ech_server: true,
        complete_cnx: true,
        try_twice: true,
        ..Default::default()
    };
    ech_e2e_test_one(&spec);
}
