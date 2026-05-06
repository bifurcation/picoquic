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

use super::util::{TEST_ECH_CONFIG_REF, TEST_ECH_PRIVATE_KEY, TEST_ECH_PUB_KEY};
use crate::{
    ech_create_config_from_private_key, ech_create_config_from_public_key, ech_read_config,
    ech_save_config, tls_api_init,
};

// ---------------------------------------------------------------------------
// Constants.

const ECH_CONFIG_FILE: &str = "ech_config.txt";
#[allow(dead_code)]
const ECH_TICKET_FILE: &str = "ech_ticket_store.bin";
const ECH_PUBLIC_NAME: &str = "test.example.com";

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
fn ech_e2e_test_one(_spec: &EchE2eSpec) {
    todo!("ech_e2e_test_one: ECH connection loop not yet implemented")
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
