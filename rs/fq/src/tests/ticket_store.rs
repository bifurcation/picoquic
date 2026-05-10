//! Test cases for `picoquictest/ticket_store_test.c`.

#![allow(non_snake_case)]

use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::Instant;
use crate::tests::util::{ticket_seed_test_one, tls_api_init_ctx};
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

#[derive(Debug, PartialEq, Eq)]
struct TicketSnapshot {
    time_valid_until: u64,
    sni: Option<String>,
    alpn: Option<String>,
    version: u32,
    ip_addr: IpAddr,
    ip_addr_client: IpAddr,
    tp_0rtt: [u64; crate::tp::NB_TP_0RTT],
    ticket: Vec<u8>,
}

fn ticket_snapshots(tickets: &[crate::internal::StoredTicket]) -> Vec<TicketSnapshot> {
    tickets
        .iter()
        .map(|ticket| TicketSnapshot {
            time_valid_until: ticket.time_valid_until.ticks(),
            sni: ticket.sni.clone(),
            alpn: ticket.alpn.clone(),
            version: ticket.version,
            ip_addr: ticket.ip_addr,
            ip_addr_client: ticket.ip_addr_client,
            tp_0rtt: ticket.tp_0rtt,
            ticket: ticket.ticket.clone(),
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
struct TokenSnapshot {
    time_valid_until: u64,
    sni: Option<String>,
    ip_addr: IpAddr,
    token: Vec<u8>,
}

fn token_snapshots(tokens: &[crate::internal::StoredToken]) -> Vec<TokenSnapshot> {
    tokens
        .iter()
        .map(|token| TokenSnapshot {
            time_valid_until: token.time_valid_until.ticks(),
            sni: token.sni.clone(),
            ip_addr: token.ip_addr,
            token: token.token.clone(),
        })
        .collect()
}

fn create_test_ticket(current_time: u64, ttl: u32, len: usize) -> Vec<u8> {
    assert!(len >= 35);

    let mut ticket = vec![0u8; len];
    let t_length = (len - 31) as u16;

    ticket[..8].copy_from_slice(&current_time.to_be_bytes());
    ticket[8] = 0;
    ticket[9] = 1;
    ticket[10] = 0;
    ticket[11..13].copy_from_slice(&t_length.to_be_bytes());
    ticket[13..17].copy_from_slice(&ttl.to_be_bytes());
    ticket[17..].fill(0xcc);
    ticket[len - 18] = 0;
    ticket[len - 17] = 16;

    ticket
}

fn create_test_token(current_time: u64, ttl: u32, len: usize) -> Vec<u8> {
    create_test_ticket(current_time, ttl, len)
}

fn test_ticket_addrs(sni_index: usize) -> (IpAddr, IpAddr) {
    let ipv4_test = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    let ipv6_test = IpAddr::V6(Ipv6Addr::from([
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    ]));

    if (sni_index & 7) == 0 {
        (
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        )
    } else {
        let ip_addr = if (sni_index & 1) != 0 {
            ipv6_test
        } else {
            ipv4_test
        };
        let ip_addr_client = if (sni_index & 2) != 0 {
            ipv6_test
        } else {
            ipv4_test
        };
        (ip_addr, ip_addr_client)
    }
}

fn assert_ticket_tp(actual: &TransportParameters, expected: &TransportParameters) {
    assert_eq!(
        actual.initial_max_data, expected.initial_max_data,
        "initial_max_data"
    );
    assert_eq!(
        actual.initial_max_stream_data_bidi_local, expected.initial_max_stream_data_bidi_local,
        "initial_max_stream_data_bidi_local"
    );
    assert_eq!(
        actual.initial_max_stream_data_bidi_remote, expected.initial_max_stream_data_bidi_remote,
        "initial_max_stream_data_bidi_remote"
    );
    assert_eq!(
        actual.initial_max_stream_data_uni, expected.initial_max_stream_data_uni,
        "initial_max_stream_data_uni"
    );
    assert_eq!(
        actual.initial_max_stream_id_bidir, expected.initial_max_stream_id_bidir,
        "initial_max_stream_id_bidir"
    );
    assert_eq!(
        actual.initial_max_stream_id_unidir, expected.initial_max_stream_id_unidir,
        "initial_max_stream_id_unidir"
    );
}

struct ExpectedTicket {
    sni: &'static str,
    alpn: &'static str,
    version: u32,
    ticket: Vec<u8>,
    ip_addr: IpAddr,
    ip_addr_client: IpAddr,
}

struct ExpectedToken {
    sni: &'static str,
    ip_addr: IpAddr,
    token: Vec<u8>,
}

/// C: `ticket_store_test` in `picoquictest/ticket_store_test.c`.
///
/// Creates a QUIC context, verifies empty-file save/load, stores synthetic
/// tickets for the C test's 3 SNI × 3 ALPN/version matrix, verifies retrieval
/// lengths and 0-RTT transport parameters, and checks full save/load content.
#[test]
fn ticket_store() {
    let test_sni = ["example.com", "example.net", "test.example.com"];
    let test_alpn = ["hq05", "hq07", "hq09"];
    let test_version: [u32; 3] = [0x0000_0001, 0xFF00_0020, 0x0000_0002];
    const TICKET_TIME: u64 = 40_000_000_000;
    const CURRENT_TIME: u64 = 50_000_000_000;
    const RETRIEVE_TIME: u64 = 60_000_000_000;
    const TOO_LATE_TIME: u64 = 150_000_000_000;
    const TTL: u32 = 100_000;
    let ticket_file =
        std::env::temp_dir().join(format!("fq_ticket_store_test_{}.bin", std::process::id()));
    let ticket_file_name = ticket_file.to_string_lossy().into_owned();

    let mut t = Instant::from_ticks(CURRENT_TIME);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    ctx.qclient
        .save_tickets(t, &ticket_file)
        .expect("save_empty_tickets");

    let mut retrieve_time = Instant::from_ticks(RETRIEVE_TIME);
    let mut empty_ctx = tls_api_init_ctx(&mut retrieve_time, 0, None).expect("empty ctx");
    empty_ctx
        .qclient
        .load_tickets(&ticket_file)
        .expect("load_empty_tickets");
    assert!(empty_ctx.qclient.stored_tickets.is_empty());

    let tp = TransportParameters {
        initial_max_stream_data_bidi_local: 123,
        initial_max_stream_data_bidi_remote: 456,
        initial_max_stream_data_uni: 78,
        initial_max_data: 91_011,
        initial_max_stream_id_bidir: 1_234,
        initial_max_stream_id_unidir: 567,
        ..TransportParameters::default()
    };

    let mut expected_tickets = Vec::new();

    for (i, &sni) in test_sni.iter().enumerate() {
        for (j, &alpn) in test_alpn.iter().enumerate() {
            let ticket_length = 64 + j * test_sni.len() + i;
            let delta_factor = i * test_alpn.len() + j;
            let test_ticket_time = TICKET_TIME / 1000 + 1000 * delta_factor as u64;
            let ticket = create_test_ticket(test_ticket_time, TTL, ticket_length);
            let (ip_addr, ip_addr_client) = test_ticket_addrs(i);
            let version = test_version[j];

            ctx.qclient
                .store_ticket(
                    Some(sni),
                    Some(alpn),
                    version,
                    ip_addr,
                    ip_addr_client,
                    &ticket,
                    &tp,
                )
                .expect("store_ticket");

            expected_tickets.push(ExpectedTicket {
                sni,
                alpn,
                version,
                ticket,
                ip_addr,
                ip_addr_client,
            });
        }
    }

    assert_eq!(ctx.qclient.stored_tickets.len(), expected_tickets.len());

    for expected in &expected_tickets {
        let (ticket, stored_tp) = ctx
            .qclient
            .get_ticket(
                Some(expected.sni),
                Some(expected.alpn),
                expected.version,
                false,
            )
            .expect("get_ticket");
        assert_eq!(ticket.len(), expected.ticket.len());
        assert_eq!(ticket, expected.ticket.as_slice());
        assert_ticket_tp(&stored_tp, &tp);
    }

    for stored in &ctx.qclient.stored_tickets {
        let expected = expected_tickets
            .iter()
            .find(|expected| {
                Some(expected.sni) == stored.sni.as_deref()
                    && Some(expected.alpn) == stored.alpn.as_deref()
                    && expected.version == stored.version
            })
            .expect("stored ticket key");
        assert_eq!(stored.ticket, expected.ticket);
        assert_eq!(stored.ip_addr, expected.ip_addr);
        assert_eq!(stored.ip_addr_client, expected.ip_addr_client);
    }

    let before_save = ticket_snapshots(&ctx.qclient.stored_tickets);

    ctx.qclient
        .save_tickets(Instant::from_ticks(CURRENT_TIME), &ticket_file)
        .expect("save_tickets");

    let mut retrieve_time = Instant::from_ticks(RETRIEVE_TIME);
    let mut loaded_ctx = tls_api_init_ctx(&mut retrieve_time, 0, None).expect("loaded ctx");
    loaded_ctx
        .qclient
        .load_tickets(&ticket_file)
        .expect("load_tickets");
    let after_load = ticket_snapshots(&loaded_ctx.qclient.stored_tickets);
    assert_eq!(after_load, before_save);

    let mut too_late = Instant::from_ticks(TOO_LATE_TIME);
    let ctx_too_late =
        tls_api_init_ctx(&mut too_late, 0, Some(&ticket_file_name)).expect("ctx too late");
    assert!(ctx_too_late.qclient.stored_tickets.is_empty());
}

/// C: `token_reuse_api_test` in `picoquictest/ticket_store_test.c`.
///
/// Exercises the replay-protection token API with the C test's token table:
/// repeated token hashes with different expiry dates are distinct entries,
/// duplicates are rejected, expired entries are cleared, and short tokens fail.
#[test]
fn token_reuse_api() {
    let mut t = Instant::from_ticks(1_000_000);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    struct TokenReuseApiCase {
        expiry_date: u64,
        token: [u8; 16],
        token_length: usize,
    }

    let cases = [
        TokenReuseApiCase {
            expiry_date: 2,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 3,
            token: [3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 3,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 3,
            token: [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 5,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 7,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0],
            token_length: 12,
        },
        TokenReuseApiCase {
            expiry_date: 1,
            token: [1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0],
            token_length: 8,
        },
    ];

    for (i, case) in cases.iter().enumerate() {
        ctx.qserver
            .registered_token_check_reuse(&case.token, case.token_length, case.expiry_date)
            .unwrap_or_else(|err| panic!("Token[{i}] already used? {err:?}"));
    }

    for (i, case) in cases.iter().enumerate() {
        let result = ctx
            .qserver
            .registered_token_check_reuse(&case.token, case.token_length, case.expiry_date)
            .is_err();
        assert!(result, "Token[{i}] not already used?");
    }

    t = Instant::from_ticks(4);
    ctx.qserver.registered_token_clear(t);

    for (i, case) in cases.iter().enumerate() {
        let result = ctx.qserver.registered_token_check_reuse(
            &case.token,
            case.token_length,
            case.expiry_date,
        );
        if case.expiry_date >= 4 {
            assert!(result.is_err(), "Token[{i}] not already used after clear?");
        } else {
            assert!(result.is_ok(), "Token[{i}] already used after clear?");
        }
    }

    for length in 0..8 {
        let result =
            ctx.qserver
                .registered_token_check_reuse(&cases[0].token, length, cases[0].expiry_date);
        assert!(result.is_err(), "Token[1] length {length} accepted?");
    }
}

/// C: `token_store_test` in `picoquictest/ticket_store_test.c`.
///
/// Verifies empty token-file save/load, stores the C test's 3 SNI × 4 IP
/// retry-token matrix, verifies retrieved lengths and bytes, compares the
/// full saved/reloaded token list, and checks late reload expiration.
#[test]
fn token_store() {
    let test_sni = ["example.com", "example.net", "test.example.com"];
    let test_ips: [IpAddr; 4] = [
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(128, 12, 34, 56)),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
        IpAddr::V6(Ipv6Addr::from([
            0x20, 1, 2, 3, 4, 5, 6, 7, 0, 0, 0, 0, 0, 0, 0, 1,
        ])),
    ];
    const TOKEN_TIME: u64 = 40_000_000_000;
    const CURRENT_TIME: u64 = 50_000_000_000;
    const RETRIEVE_TIME: u64 = 60_000_000_000;
    const TOO_LATE_TIME: u64 = 150_000_000_000;
    const TTL: u32 = 100_000;
    let token_file =
        std::env::temp_dir().join(format!("fq_token_store_test_{}.bin", std::process::id()));

    let mut t = Instant::from_ticks(CURRENT_TIME);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    ctx.qclient
        .save_tokens(&token_file)
        .expect("save_empty_tokens");

    let mut retrieve_time = Instant::from_ticks(RETRIEVE_TIME);
    let mut empty_ctx = tls_api_init_ctx(&mut retrieve_time, 0, None).expect("empty ctx");
    empty_ctx
        .qclient
        .load_tokens(&token_file)
        .expect("load_empty_tokens");
    assert!(empty_ctx.qclient.stored_tokens.is_empty());

    let mut expected_tokens = Vec::new();

    for (i, &sni) in test_sni.iter().enumerate() {
        for (j, &ip_addr) in test_ips.iter().enumerate() {
            let token_length = 64 + j * test_sni.len() + i;
            let delta_factor = i * test_ips.len() + j;
            let test_token_time = TOKEN_TIME / 1000 + 1000 * delta_factor as u64;
            let token = create_test_token(test_token_time, TTL, token_length);

            ctx.qclient
                .store_token(Some(sni), ip_addr, &token)
                .expect("store_token");

            expected_tokens.push(ExpectedToken {
                sni,
                ip_addr,
                token,
            });
        }
    }

    assert_eq!(ctx.qclient.stored_tokens.len(), expected_tokens.len());

    for expected in &expected_tokens {
        let token = ctx
            .qclient
            .get_token(Some(expected.sni), expected.ip_addr, false)
            .unwrap_or_else(|err| {
                panic!(
                    "get_token sni={} ip={:?}: {err:?}",
                    expected.sni, expected.ip_addr
                )
            });
        assert_eq!(token.len(), expected.token.len());
        assert_eq!(token, expected.token.as_slice());
    }

    for stored in &ctx.qclient.stored_tokens {
        let expected = expected_tokens
            .iter()
            .find(|expected| {
                Some(expected.sni) == stored.sni.as_deref() && expected.ip_addr == stored.ip_addr
            })
            .expect("stored token key");
        assert_eq!(stored.token, expected.token);
    }

    let before_save = token_snapshots(&ctx.qclient.stored_tokens);

    ctx.qclient.save_tokens(&token_file).expect("save_tokens");

    let mut retrieve_time = Instant::from_ticks(RETRIEVE_TIME);
    let mut loaded_ctx = tls_api_init_ctx(&mut retrieve_time, 0, None).expect("loaded ctx");
    loaded_ctx
        .qclient
        .load_tokens(&token_file)
        .expect("load_tokens");
    let after_load = token_snapshots(&loaded_ctx.qclient.stored_tokens);
    assert_eq!(after_load, before_save);

    let mut too_late = Instant::from_ticks(TOO_LATE_TIME);
    let mut late_ctx = tls_api_init_ctx(&mut too_late, 0, None).expect("late ctx");
    late_ctx
        .qclient
        .load_tokens(&token_file)
        .expect("load_tokens_too_late");
    assert!(
        late_ctx.qclient.stored_tokens.is_empty(),
        "expired tokens should be dropped during late reload"
    );
}
