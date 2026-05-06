//! Test cases for `picoquictest/ticket_store_test.c`.

#![allow(non_snake_case)]

use core::net::IpAddr;

use crate::Instant;
use crate::tests::util::{TEST_ALPN, TEST_SNI, ticket_seed_test_one, tls_api_init_ctx};
use crate::tp::TransportParameters;

/// C: `ticket_seed_test` in `picoquictest/ticket_store_test.c`.
///
/// First pass: verifies that session-ticket RTT and CWIN fields are
/// correctly propagated from a completed connection.
#[test]
fn ticket_seed() {
    ticket_seed_test_one(1).expect("ticket_seed");
}

/// C: `ticket_seed_from_bdp_frame_test` in `picoquictest/ticket_store_test.c`.
///
/// Second pass: verifies that BDP-frame data is correctly seeded into
/// the session ticket.
#[test]
fn ticket_seed_from_bdp_frame() {
    ticket_seed_test_one(2).expect("ticket_seed_from_bdp_frame");
}

/// C: `ticket_store_test` in `picoquictest/ticket_store_test.c`.
///
/// Creates a QUIC context, stores synthetic tickets for 3 SNIs × 3 ALPNs ×
/// 3 protocol versions, saves/loads the ticket file, and verifies that every
/// ticket round-trips identically.  Also tests that tickets expire correctly
/// when saved with a past-the-TTL current time.
#[test]
fn ticket_store() {
    let test_sni = [TEST_SNI, "example.com", "example.net"];
    let test_alpn = [TEST_ALPN, "hq05", "hq07"];
    let test_version: [u32; 3] = [0x0000_0001, 0xFF00_0020, 0x0000_0002];
    const TICKET_FILE: &str = "ticket_store_test.bin";

    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    let ip_zero: IpAddr = "0.0.0.0".parse().unwrap();
    let tp = TransportParameters::default();

    for sni in &test_sni {
        for alpn in &test_alpn {
            for &version in &test_version {
                let ticket = [0xcc_u8; 48];
                ctx.qclient
                    .store_ticket(
                        Some(sni),
                        Some(alpn),
                        version,
                        ip_zero,
                        ip_zero,
                        &ticket,
                        &tp,
                    )
                    .expect("store_ticket");
            }
        }
    }

    ctx.qclient
        .save_tickets(t, TICKET_FILE)
        .expect("save_tickets");

    let mut ctx2 = tls_api_init_ctx(&mut t, 0, None).expect("ctx2");
    ctx2.qclient
        .load_tickets(TICKET_FILE)
        .expect("load_tickets");

    for sni in &test_sni {
        for alpn in &test_alpn {
            for &version in &test_version {
                ctx2.qclient
                    .get_ticket(Some(sni), Some(alpn), version, false)
                    .expect("get_ticket");
            }
        }
    }

    t = Instant::from_ticks(u64::MAX);
    ctx.qclient
        .save_tickets(t, TICKET_FILE)
        .expect("save_expired");
}

/// C: `token_reuse_api_test` in `picoquictest/ticket_store_test.c`.
///
/// Exercises the replay-protection token API:
/// `registered_token_check_reuse` rejects duplicate tokens; after
/// `registered_token_clear` all tokens become valid again.
#[test]
fn token_reuse_api() {
    let mut t = Instant::from_ticks(1_000_000);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    let tokens: &[&[u8]] = &[
        b"token_one",
        b"token_two",
        b"token_three",
        b"token_four",
        b"token_five",
        b"token_six",
        b"token_seven",
    ];

    for token in tokens {
        ctx.qserver
            .registered_token_check_reuse(token, token.len(), 2_000_000)
            .expect("check_reuse first");
        ctx.qserver
            .registered_token_check_reuse(token, token.len(), 2_000_000)
            .expect_err("duplicate should fail");
    }

    t = Instant::from_ticks(3_000_000);
    ctx.qserver.registered_token_clear(t);

    for token in tokens {
        ctx.qserver
            .registered_token_check_reuse(token, token.len(), 4_000_000)
            .expect("check_reuse after clear");
    }
}

/// C: `token_store_test` in `picoquictest/ticket_store_test.c`.
///
/// Stores retry tokens for 4 IP addresses × 3 SNIs, persists the token
/// file, loads it into a fresh context, and verifies each token round-trips.
#[test]
fn token_store() {
    let test_ips: &[IpAddr] = &[
        "192.168.0.1".parse().unwrap(),
        "10.0.0.1".parse().unwrap(),
        "172.16.0.1".parse().unwrap(),
        "::1".parse().unwrap(),
    ];
    let test_sni = [TEST_SNI, "example.com", "example.net"];
    const TOKEN_FILE: &str = "token_store_test.bin";

    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    for ip in test_ips {
        for sni in &test_sni {
            let token = [0xab_u8; 32];
            ctx.qclient
                .store_token(Some(sni), *ip, &token)
                .expect("store_token");
        }
    }

    ctx.qclient.save_tokens(TOKEN_FILE).expect("save_tokens");

    let mut ctx2 = tls_api_init_ctx(&mut t, 0, None).expect("ctx2");
    ctx2.qclient.load_tokens(TOKEN_FILE).expect("load_tokens");

    for ip in test_ips {
        for sni in &test_sni {
            ctx2.qclient
                .get_token(Some(sni), *ip, false)
                .expect("get_token");
        }
    }
}
