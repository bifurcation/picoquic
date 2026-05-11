//! Test cases for `picoquictest/multipath_test.c`.
//!
//! Covers address migration, multipath QUIC, monopath-with-multipath-option,
//! and the multipath AEAD / qlog smoke tests.

#![allow(non_snake_case)]

use core::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use super::util::{
    TEST_ALPN, TEST_SNI, TestApiStreamDesc, TestDatagramCtx, ZeroRttTest, compare_text_files,
    multipath_init_params, multipath_test_add_links, multipath_test_kill_links,
    multipath_test_kill_server_links, multipath_test_perf_links, multipath_test_sat_links,
    multipath_test_set_reachable, multipath_test_set_unreachable, multipath_test_unkill_links,
    test_api_init_send_recv_scenario, test_datagram_check_ready, test_datagram_next_time_ready,
    tls_api_connection_loop, tls_api_data_sending_loop, tls_api_init_ctx, tls_api_init_ctx_ex2,
    tls_api_init_ctx_ex2_delayed, tls_api_one_scenario_body_connect,
    tls_api_one_scenario_body_verify, tls_api_one_scenario_init_ex, tls_api_one_sim_round,
    tls_api_wait_for_timeout, wait_client_connection_ready, wait_client_migration_done,
    wait_multipath_ready, zero_rtt_test_one,
};
use crate::internal::{DatagramBufferArgument, Version};
use crate::tls_api::{LABEL_QUIC_V1_KEY_BASE, setup_test_aead_context};
use crate::utils::{frames_uint64_decode, frames_uint64_encode};
use crate::{
    AES_128_GCM_SHA256, CallbackEvent, Connection, ConnectionId, ConnectionIdCallback,
    DatagramActive, GROUP_SECP256R1, Instant, MAX_PACKET_SIZE, PacketContext, PathStatus,
    RESET_SECRET_SIZE, StreamDataCallback, aead_decrypt_mp, aead_encrypt_mp, public_random_seed_64,
};

// ---------------------------------------------------------------------------
// Shared stream scenarios.

const SCENARIO_MULTIPATH: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 4,
        q_len: 257,
        r_len: 1_000_000,
    },
];

const SCENARIO_MULTIPATH_LONG: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 12,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 16,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 20,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 24,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 28,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 32,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 36,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
    TestApiStreamDesc {
        stream_id: 40,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    },
];

const SCENARIO_MULTIPATH_QLOG: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 10_000,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 4,
        q_len: 257,
        r_len: 10_000,
    },
];

// ---------------------------------------------------------------------------
// MultipathTestId enum — mirrors `multipath_test_enum_t` in C.

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MultipathTestId {
    Basic = 0,
    DropFirst = 1,
    DropSecond = 2,
    SatPlus = 3,
    Renew = 4,
    Rotation = 5,
    Nat = 6,
    NatChallenge = 7,
    Break1 = 8,
    Break2 = 9,
    Break3 = 10,
    Back0 = 11,
    Back1 = 12,
    Perf = 13,
    Callback = 14,
    Quality = 15,
    QualityServer = 16,
    StreamAf = 17,
    Abandon = 18,
    Datagram = 19,
    DgAf = 20,
    Backup = 21,
    Standup = 22,
    Tunnel = 23,
    Fail = 24,
    Ab1 = 25,
    Discovery = 26,
    KeepAlive = 27,
    JustOne = 28,
    BreakBoth = 29,
}

// ---------------------------------------------------------------------------
// MonopathTestId enum — mirrors `monopath_test_enum_t` in C.

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum MonopathTestId {
    Basic = 0,
    Hole = 1,
    Rotation = 2,
    KeepAlive = 3,
}

// ---------------------------------------------------------------------------
// migration_test_one — shared by migration_controlled and migration_mtu_drop.
// C: `migration_test_one`.

fn migration_test_one(mtu_drop: bool) {
    let max_completion_microsec = 2_100_000u64;
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;

    let mut cid_bytes = [0x1a_u8, 0x10, 0xc0, 4, 5, 6, 7, 8];
    if mtu_drop {
        cid_bytes[2] = 0xcd;
    }
    let initial_cid = ConnectionId::clone_from_slice(&cid_bytes).expect("migration CID");

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    if mtu_drop {
        loss_mask |= 1u64 << 31;
    }

    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_MULTIPATH)
        .expect("init send/recv scenario");

    wait_client_connection_ready(&mut test_ctx, &mut simulated_time).expect("client ready");

    multipath_test_add_links(&mut test_ctx, mtu_drop).expect("add links");

    let server_addr = test_ctx.server_addr;
    let client_addr_2 = test_ctx.client_addr_2;
    test_ctx
        .cnx_client()
        .probe_new_path(&server_addr, &client_addr_2, simulated_time)
        .expect("probe new path");

    wait_client_migration_done(&mut test_ctx, &mut simulated_time).expect("migration done");

    multipath_test_kill_links(&mut test_ctx, 0);

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, max_completion_microsec)
        .expect("scenario body verify");

    assert!(test_ctx.has_cnx_server(), "no server connection");

    let c_addr_at_server = test_ctx.cnx_server().get_peer_addr();
    let c_addr_at_client = test_ctx.cnx_client().get_local_addr();
    assert_eq!(
        c_addr_at_server, test_ctx.client_addr_2,
        "server peer addr should be client_addr_2"
    );
    assert_eq!(
        c_addr_at_client, test_ctx.client_addr_2,
        "client local addr should be client_addr_2"
    );
}

// ---------------------------------------------------------------------------
// multipath_init_callbacks / multipath_verify_callbacks.
// C: `multipath_init_callbacks` / `multipath_verify_callbacks`.

fn multipath_init_callbacks(
    test_ctx: &mut super::util::TestTlsApiCtx,
    test_id: MultipathTestId,
) -> crate::Result<()> {
    use MultipathTestId::*;
    if test_id == QualityServer {
        test_ctx.qserver.enable_path_callbacks_default(true);
        test_ctx
            .qserver
            .default_quality_update(50_000, crate::Duration::from_ticks(5_000));
    } else {
        test_ctx.cnx_client().enable_path_callbacks(true);
        if test_id == Quality {
            test_ctx
                .cnx_client()
                .subscribe_to_quality_update(50_000, crate::Duration::from_ticks(5_000));
            test_ctx.cnx_client().subscribe_to_quality_update_per_path(
                0,
                50_000,
                crate::Duration::from_ticks(5_000),
            )?;
        }
        if test_id == Callback {
            test_ctx.cnx_client().subscribe_new_path_allowed()?;
        }
    }
    Ok(())
}

fn multipath_verify_callbacks(test_id: MultipathTestId) -> crate::Result<()> {
    let (filename, reference) = match test_id {
        MultipathTestId::Callback => ("path_callback.csv", "picoquictest/path_callback_ref.txt"),
        MultipathTestId::Quality => ("path_quality.csv", "picoquictest/path_quality_ref.txt"),
        MultipathTestId::QualityServer => (
            "path_quality_server.csv",
            "picoquictest/path_quality_server_ref.txt",
        ),
        _ => return Err(crate::Error::InvalidArgument),
    };

    compare_text_files(filename, reference)
}

fn multipath_assert_ready(test_ctx: &mut super::util::TestTlsApiCtx, test_id: MultipathTestId) {
    assert!(
        matches!(test_ctx.cnx_client().connection_state, crate::State::Ready),
        "{test_id:?}: client connection should be ready after multipath wait"
    );

    for endpoint in ["client", "server"] {
        let cnx = if endpoint == "client" {
            test_ctx.cnx_client()
        } else {
            test_ctx.cnx_server()
        };
        assert_eq!(
            cnx.nb_paths(),
            2,
            "{test_id:?}: {endpoint} should have two paths after multipath wait"
        );
        assert!(
            cnx.paths
                .get(1)
                .and_then(|path| path.tuples.first())
                .is_some_and(|tuple| tuple.challenge_verified),
            "{test_id:?}: {endpoint} path 1 should have verified challenge"
        );
    }
}

// ---------------------------------------------------------------------------
// multipath_test_abandon_cycle.
// C: `multipath_test_abandon_cycle`.

fn multipath_test_abandon_cycle(
    test_ctx: &mut super::util::TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let deleted_id = test_ctx
        .cnx_client()
        .local_connection_id_lists
        .iter()
        .filter(|list| !list.is_demoted && list.unique_path_id != 0)
        .map(|list| list.unique_path_id)
        .min()
        .ok_or(crate::Error::Generic)?;

    test_ctx
        .cnx_client()
        .abandon_path(deleted_id, 0, *simulated_time)?;

    tls_api_wait_for_timeout(test_ctx, simulated_time, 250_000)?;

    if test_ctx
        .cnx_client()
        .local_connection_id_lists
        .iter()
        .any(|list| list.unique_path_id == deleted_id)
    {
        return Err(crate::Error::Generic);
    }

    let mut stash_count = 0usize;
    for i in 0..=10 {
        stash_count = test_ctx.cnx_client().remote_connection_id_stashes.len();
        if stash_count >= 2 {
            break;
        }
        if i < 10 {
            tls_api_wait_for_timeout(test_ctx, simulated_time, 100_000)?;
        }
    }

    if stash_count >= 2 {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

// ---------------------------------------------------------------------------
// multipath datagram helpers.
// C: `multipath_init_datagram_ctx`, `multipath_set_datagram_ready`,
//    `multipath_verify_datagram_sent`, `multipath_datagram_send_loop`.

#[derive(Clone)]
struct MultipathDatagramCallback {
    dg_ctx: Rc<RefCell<TestDatagramCtx>>,
}

impl MultipathDatagramCallback {
    fn new(dg_ctx: &Rc<RefCell<TestDatagramCtx>>) -> Self {
        Self {
            dg_ctx: Rc::clone(dg_ctx),
        }
    }
}

impl StreamDataCallback for MultipathDatagramCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        _stream_ctx: Option<&mut dyn Any>,
    ) -> i32 {
        let client_mode = usize::from(connection.client_mode);
        match fin_or_event {
            CallbackEvent::Datagram => multipath_datagram_recv(
                &mut self.dg_ctx.borrow_mut(),
                client_mode,
                stream_id,
                bytes,
                connection.latest_receive_time.ticks(),
            ),
            CallbackEvent::DatagramAcked
            | CallbackEvent::DatagramLost
            | CallbackEvent::DatagramSpurious => {
                multipath_datagram_ack(&mut self.dg_ctx.borrow_mut(), client_mode, fin_or_event)
            }
            _ => 0,
        }
    }

    fn prepare_datagram<'buf, 'cnx, 'path>(
        &mut self,
        context: &mut DatagramBufferArgument<'buf, 'cnx, 'path>,
        unique_path_id: u64,
        allowed_space: usize,
    ) -> i32 {
        multipath_datagram_prepare(
            &mut self.dg_ctx.borrow_mut(),
            context,
            unique_path_id,
            allowed_space,
        )
    }
}

fn multipath_install_datagram_callback(
    cnx: &mut Connection,
    dg_ctx: &Rc<RefCell<TestDatagramCtx>>,
) {
    cnx.set_callback(Some(Box::new(MultipathDatagramCallback::new(dg_ctx))));
}

fn multipath_init_datagram_ctx(
    test_ctx: &mut super::util::TestTlsApiCtx,
    dg_ctx: &Rc<RefCell<TestDatagramCtx>>,
) {
    *dg_ctx.borrow_mut() = TestDatagramCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 3_000,
        use_extended_provider_api: true,
        ..TestDatagramCtx::default()
    };
    test_ctx
        .qserver
        .set_default_callback(Some(Box::new(MultipathDatagramCallback::new(dg_ctx))));
    multipath_install_datagram_callback(test_ctx.cnx_client(), dg_ctx);
    test_ctx.qserver.enable_path_callbacks_default(true);
    test_ctx.cnx_client().enable_path_callbacks(true);
}

fn multipath_set_datagram_ready(
    test_ctx: &mut super::util::TestTlsApiCtx,
    dg_ctx: &Rc<RefCell<TestDatagramCtx>>,
    test_id: MultipathTestId,
) -> crate::Result<()> {
    if test_id == MultipathTestId::Datagram {
        test_ctx.cnx_client().mark_datagram_ready(true)?;
        test_ctx.cnx_server().mark_datagram_ready(true)?;
    } else {
        dg_ctx.borrow_mut().test_affinity = true;
        test_ctx.cnx_client().mark_datagram_ready_path(0, true)?;
        test_ctx.cnx_server().mark_datagram_ready_path(0, true)?;
    }
    Ok(())
}

fn multipath_verify_datagram_sent(
    dg_ctx: &TestDatagramCtx,
    test_id: MultipathTestId,
) -> crate::Result<()> {
    if 4 * dg_ctx.dg_recv[0] < 3 * dg_ctx.dg_target[1]
        || 4 * dg_ctx.dg_recv[1] < 3 * dg_ctx.dg_target[0]
    {
        return Err(crate::Error::Generic);
    }

    match test_id {
        MultipathTestId::Datagram => {
            for i in 0..2 {
                let min_expected = dg_ctx.dg_recv[i] / 8;
                if dg_ctx.nb_recv_path_0[i] < min_expected
                    || dg_ctx.nb_recv_path_other[i] < min_expected
                {
                    return Err(crate::Error::Generic);
                }
            }
            Ok(())
        }
        MultipathTestId::DgAf => {
            if dg_ctx.nb_recv_path_other.iter().any(|&n| n > 0) {
                Err(crate::Error::Generic)
            } else {
                Ok(())
            }
        }
        _ => Err(crate::Error::InvalidArgument),
    }
}

fn multipath_datagram_prepare(
    dg_ctx: &mut TestDatagramCtx,
    context: &mut DatagramBufferArgument<'_, '_, '_>,
    unique_path_id: u64,
    allowed_space: usize,
) -> i32 {
    let (client_mode, current_time) = {
        let connection = context.connection_mut();
        (
            usize::from(connection.client_mode),
            connection.latest_progress_time.ticks(),
        )
    };

    if client_mode >= 2 || (client_mode == 0 && allowed_space > dg_ctx.dg_max_size) {
        return -1;
    }

    if allowed_space < 24
        || !dg_ctx.is_ready[client_mode]
        || dg_ctx.dg_sent[client_mode] >= dg_ctx.dg_target[client_mode]
        || (dg_ctx.test_affinity && unique_path_id != 0)
    {
        let _ = crate::provide_datagram_buffer_ex(context, 0, DatagramActive::NotActive);
        return 0;
    }

    let sent_mod = (dg_ctx.dg_sent[client_mode] % 6) as usize;
    let available = allowed_space.saturating_sub(sent_mod + 8);
    if available < 16 {
        let _ = crate::provide_datagram_buffer_ex(context, 0, DatagramActive::NotActive);
        return -1;
    }

    let active = if dg_ctx.test_affinity {
        DatagramActive::ThisPathOnly
    } else {
        DatagramActive::AnyPath
    };
    let Some(payload) = crate::provide_datagram_buffer_ex(context, available, active) else {
        return -1;
    };

    let send_time = if dg_ctx.dg_sent[client_mode] == 0 {
        current_time
    } else {
        dg_ctx.dg_time_ready[client_mode]
    };
    dg_ctx.dg_sent[client_mode] += 1;

    let Some(rest) = frames_uint64_encode(payload, dg_ctx.dg_sent[client_mode]) else {
        return -1;
    };
    let Some(rest) = frames_uint64_encode(rest, send_time) else {
        return -1;
    };
    rest.fill(b'd');

    dg_ctx.next_gen_time[client_mode] =
        dg_ctx.next_gen_time[client_mode].saturating_add(dg_ctx.send_delay);
    dg_ctx.is_ready[client_mode] = false;
    0
}

fn multipath_datagram_recv(
    dg_ctx: &mut TestDatagramCtx,
    client_mode: usize,
    unique_path_id: u64,
    bytes: &[u8],
    current_time: u64,
) -> i32 {
    if client_mode >= 2 {
        return -1;
    }

    dg_ctx.dg_recv[client_mode] += 1;
    if bytes.len() > 16 {
        if unique_path_id == 0 {
            dg_ctx.nb_recv_path_0[client_mode] += 1;
        } else {
            dg_ctx.nb_recv_path_other[client_mode] += 1;
        }

        if let Some((tail, _number_sent)) = frames_uint64_decode(bytes)
            && let Some((_tail, time_sent)) = frames_uint64_decode(tail)
            && time_sent <= current_time
        {
            // Multipath verification only needs receive and path counters.
        }
    }
    0
}

fn multipath_datagram_ack(
    dg_ctx: &mut TestDatagramCtx,
    client_mode: usize,
    event: CallbackEvent,
) -> i32 {
    if client_mode >= 2 {
        return -1;
    }

    match event {
        CallbackEvent::DatagramAcked => dg_ctx.dg_acked[client_mode] += 1,
        CallbackEvent::DatagramLost => dg_ctx.dg_nacked[client_mode] += 1,
        CallbackEvent::DatagramSpurious => dg_ctx.dg_spurious[client_mode] += 1,
        _ => return -1,
    }
    0
}

fn multipath_datagram_send_loop(
    test_ctx: &mut super::util::TestTlsApiCtx,
    dg_ctx: &Rc<RefCell<TestDatagramCtx>>,
    loss_mask: &mut u64,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    test_ctx.c_to_s_link.loss_mask = Some(*loss_mask);
    test_ctx.s_to_c_link.loss_mask = Some(*loss_mask);

    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while nb_trials < 16_000
        && nb_inactive < 512
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let mut was_active = false;
        let time_out = test_datagram_next_time_ready(&dg_ctx.borrow());
        nb_trials += 1;

        tls_api_one_sim_round(test_ctx, simulated_time, time_out, &mut was_active)?;

        for dir in 0..2 {
            let (should_mark_ready, test_affinity) = {
                let mut dg_ctx = dg_ctx.borrow_mut();
                let should_mark_ready = !dg_ctx.is_ready[dir]
                    && test_datagram_check_ready(&mut dg_ctx, dir, simulated_time.ticks());
                (should_mark_ready, dg_ctx.test_affinity)
            };

            if should_mark_ready {
                if test_affinity {
                    if dir == 0 {
                        test_ctx.cnx_server().mark_datagram_ready_path(0, true)?;
                    } else {
                        test_ctx.cnx_client().mark_datagram_ready_path(0, true)?;
                    }
                } else if dir == 0 {
                    test_ctx.cnx_server().mark_datagram_ready(true)?;
                } else {
                    test_ctx.cnx_client().mark_datagram_ready(true)?;
                }
            }
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if test_ctx.test_finished {
            let client_empty = test_ctx
                .qclient
                .first_cnx_mut()
                .map(|c| c.is_backlog_empty())
                .unwrap_or(true);
            let server_empty = test_ctx
                .qserver
                .first_cnx_mut()
                .map(|c| c.is_backlog_empty())
                .unwrap_or(true);
            if test_ctx.immediate_exit || (client_empty && server_empty) {
                break;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// multipath_test_do_keep_alive.
// C: `multipath_test_do_keep_alive`.

fn multipath_test_do_keep_alive(
    test_ctx: &mut super::util::TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let ping_frame = [0x01u8]; // PING frame type
    let mut nat_test_needed = true;
    let mut rounds_without_nat = 0u32;

    while *simulated_time < Instant::from_ticks(200_000_000) {
        let previous_time = *simulated_time;

        if nat_test_needed {
            if rounds_without_nat > 3 {
                let mut natted = test_ctx.client_addr;
                let port = natted.port();
                natted.set_port(port + 7);
                test_ctx.client_addr_natted = natted;
                test_ctx.client_use_nat = true;
                nat_test_needed = false;
            } else {
                rounds_without_nat += 1;
            }
        }

        test_ctx
            .cnx_client()
            .queue_misc_frame(&ping_frame, false, PacketContext::Application)
            .expect("queue ping frame");

        tls_api_wait_for_timeout(test_ctx, simulated_time, 10_000_000).expect("wait 10s");

        assert!(
            test_ctx.client_ready()
                && test_ctx.server_ready()
                && *simulated_time >= previous_time + crate::Duration::from_ticks(1_000_000),
            "connection stalled at t={}",
            simulated_time.ticks()
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// multipath_verify_all_cid_available.
// C: `multipath_verify_all_cid_available`.

fn multipath_verify_all_cid_available(cnx: &mut crate::internal::Connection) -> crate::Result<()> {
    let unique_id_max = cnx
        .remote_connection_id_stashes
        .iter()
        .map(|stash| stash.unique_path_id)
        .max()
        .unwrap_or(0);

    if unique_id_max < cnx.max_path_id_local() && unique_id_max < cnx.max_path_id_remote() {
        Err(crate::Error::Generic)
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// multipath_trace_test_one.
// C: `multipath_trace_test_one`.

struct QlogTraceCid {
    data: ConnectionId,
}

impl ConnectionIdCallback for QlogTraceCid {
    fn produce(
        &mut self,
        _quic: &mut crate::Quic,
        connection_id_local: ConnectionId,
        connection_id_remote: ConnectionId,
    ) -> ConnectionId {
        let len = connection_id_local.len();
        let mut cid = ConnectionId::with_size(len).unwrap_or_default();
        for i in 0..len {
            cid.as_bytes_mut()[i] = connection_id_remote
                .as_bytes()
                .get(i)
                .copied()
                .unwrap_or(0)
                .wrapping_add(self.data.as_bytes().get(i).copied().unwrap_or(0));
        }

        for byte in self.data.as_bytes_mut().iter_mut().take(len) {
            *byte = byte.wrapping_add(1);
            if *byte != 0 {
                break;
            }
        }

        cid
    }
}

fn multipath_trace_test_one(use_qlog_streaming: bool) {
    const RANDOM_PUBLIC_TEST_SEED: u64 = 0xDEAD_BEEF_CAFE_C001;
    const QLOG_MULTIPATH_INITIAL_CID: [u8; 8] = [8, 7, 6, 5, 4, 3, 2, 1];
    let reset_seed_client: [u8; RESET_SECRET_SIZE] = [
        10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    ];
    let reset_seed_server: [u8; RESET_SECRET_SIZE] = [
        20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35,
    ];

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;

    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, Version::InternalTest1 as u32, None)
        .expect("tls_api_init_ctx");

    if use_qlog_streaming {
        test_ctx.qserver.set_qlog(".").ok();
    } else {
        test_ctx.qserver.set_binlog(Some(".")).ok();
    }
    test_ctx
        .qserver
        .set_default_spinbit_policy(crate::SpinbitVersion::On)
        .ok();
    test_ctx
        .qclient
        .set_default_spinbit_policy(crate::SpinbitVersion::On)
        .ok();
    test_ctx
        .qserver
        .set_default_lossbit_policy(crate::LossbitVersion::SendReceive);
    test_ctx
        .qclient
        .set_default_lossbit_policy(crate::LossbitVersion::SendReceive);

    test_ctx.qserver.connection_id_callback_fn = Some(Box::new(QlogTraceCid {
        data: ConnectionId::clone_from_slice(&[2; 8]).expect("qlog server cid data"),
    }));
    test_ctx.qclient.connection_id_callback_fn = Some(Box::new(QlogTraceCid {
        data: ConnectionId::clone_from_slice(&[1; 8]).expect("qlog client cid data"),
    }));

    test_ctx.qclient.reset_seed = reset_seed_client;
    test_ctx.qserver.reset_seed = reset_seed_server;
    test_ctx.qserver.use_constant_challenges = true;
    test_ctx.qclient.use_constant_challenges = true;

    let lat = test_ctx.c_to_s_link.microsec_latency;
    test_ctx.c_to_s_link.queue_delay_max = 2 * lat;
    let lat = test_ctx.s_to_c_link.microsec_latency;
    test_ctx.s_to_c_link.queue_delay_max = 2 * lat;

    let server_params = multipath_init_params(true);
    test_ctx.qserver.set_default_tp(&server_params).ok();
    let client_params = multipath_init_params(true);
    test_ctx.qclient.set_default_tp(&client_params).ok();

    test_ctx.qclient.use_predictable_random = true;
    test_ctx.qserver.use_predictable_random = true;

    test_ctx.qclient.set_cipher_suite(AES_128_GCM_SHA256).ok();
    test_ctx.qclient.set_key_exchange(GROUP_SECP256R1).ok();

    let old_initial_cid = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.initial_connection_id);
    if let Some(old_initial_cid) = old_initial_cid
        && let Some((token, _)) = test_ctx.qclient.connection_by_id(old_initial_cid)
    {
        test_ctx.qclient.delete_connection(token);
    }
    public_random_seed_64(RANDOM_PUBLIC_TEST_SEED, 1);
    let initial_cid =
        ConnectionId::clone_from_slice(&QLOG_MULTIPATH_INITIAL_CID).expect("qlog initial CID");
    test_ctx
        .qclient
        .create_connection(
            initial_cid,
            ConnectionId::default(),
            Some(&test_ctx.server_addr),
            simulated_time,
            Version::InternalTest1 as u32,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .expect("create qlog client connection");

    {
        test_ctx.cnx_client().start_client().expect("start client");

        let queue_delay = 2 * test_ctx.s_to_c_link.microsec_latency;
        tls_api_connection_loop(
            &mut test_ctx,
            &mut loss_mask,
            queue_delay,
            &mut simulated_time,
        )
        .expect("connection loop");

        wait_client_connection_ready(&mut test_ctx, &mut simulated_time).expect("client ready");

        test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_MULTIPATH_QLOG)
            .expect("init scenario");

        multipath_test_add_links(&mut test_ctx, false).expect("add links");

        let server_addr = test_ctx.server_addr;
        let client_addr_2 = test_ctx.client_addr_2;
        test_ctx
            .cnx_client()
            .probe_new_path(&server_addr, &client_addr_2, simulated_time)
            .expect("probe new path");

        wait_multipath_ready(&mut test_ctx, &mut simulated_time).expect("multipath ready");

        tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
            .expect("data sending loop");

        tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 2_000_000)
            .expect("scenario body verify");

        if test_ctx.has_cnx_server() {
            let (server_local_cid, peer_addr, local_addr) = {
                let cnx_server = test_ctx.cnx_server();
                (
                    cnx_server.local_cnxid(),
                    cnx_server.path_peer_addr_by_index(0),
                    cnx_server.path_local_addr_by_index(0),
                )
            };
            let mut packet = [0u8; 256];
            let cid = server_local_cid.as_bytes();
            packet[1..1 + cid.len()].copy_from_slice(cid);
            packet[0] |= 64;
            let _ = test_ctx.qserver.incoming_packet(
                &mut packet,
                &peer_addr,
                &local_addr,
                0,
                0,
                simulated_time,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// multipath_test_one — the large shared helper.
// C: `multipath_test_one`.

fn multipath_test_one(max_completion_microsec: u64, test_id: MultipathTestId) {
    use MultipathTestId::*;

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;

    let mut cid_bytes = [0x1b_u8, 0x11, 0xc0, 4, 5, 6, 7, 8];
    cid_bytes[2] = test_id as u8;
    let initial_cid = ConnectionId::clone_from_slice(&cid_bytes).expect("multipath CID");

    let mut test_ctx = tls_api_init_ctx_ex2_delayed(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    if test_id == Perf {
        test_ctx.set_send_buffer_size(65_536);
    }

    let is_sat_test = test_id == SatPlus;

    if is_sat_test || test_id == Break1 || test_id == Break2 || test_id == Back1 {
        multipath_test_sat_links(&mut test_ctx, 0);
    } else if test_id == Perf {
        multipath_test_perf_links(&mut test_ctx, 0);
        crate::register_all_congestion_control_algorithms();
        test_ctx
            .qserver
            .set_default_congestion_algorithm(crate::get_congestion_algorithm("bbr").expect("bbr"));
    }

    let lat = test_ctx.c_to_s_link.microsec_latency;
    test_ctx.c_to_s_link.queue_delay_max = 2 * lat;
    let lat = test_ctx.s_to_c_link.microsec_latency;
    test_ctx.s_to_c_link.queue_delay_max = 2 * lat;

    if test_id == Rotation {
        test_ctx.qserver.set_default_crypto_epoch_length(200);
    }

    if matches!(test_id, Callback | Quality | QualityServer | StreamAf) {
        multipath_init_callbacks(&mut test_ctx, test_id).expect("init callbacks");
    }

    let dg_ctx = Rc::new(RefCell::new(TestDatagramCtx::default()));
    if matches!(test_id, Datagram | DgAf) {
        multipath_init_datagram_ctx(&mut test_ctx, &dg_ctx);
    }

    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;
    test_ctx.qclient.set_binlog(Some(".")).ok();
    test_ctx.qclient.use_long_log = true;

    let mut server_params = multipath_init_params(is_sat_test);
    if matches!(test_id, Datagram | DgAf) {
        // server_params.max_datagram_frame_size = 1536 — through set_default_tp below
        test_ctx
            .cnx_client()
            .set_local_max_datagram_frame_size(1536);
    }
    if test_id == Discovery {
        test_ctx.cnx_client().set_local_address_discovery_mode(3);
        server_params.address_discovery_mode = 1;
    }
    if test_id == JustOne {
        server_params.initial_max_path_id = 1;
    }
    test_ctx.qserver.set_default_tp(&server_params).ok();

    test_ctx.cnx_client().set_local_enable_time_stamp(3);
    test_ctx.cnx_client().set_local_initial_max_path_id(3);

    // Establish the connection.
    test_ctx.cnx_client().start_client().expect("start client");

    let queue_delay = 2 * test_ctx.s_to_c_link.microsec_latency;
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        queue_delay,
        &mut simulated_time,
    )
    .expect("connection loop");

    assert!(
        test_ctx.has_cnx_server(),
        "server connection not accepted during multipath handshake"
    );
    if matches!(test_id, Datagram | DgAf) {
        multipath_install_datagram_callback(test_ctx.cnx_server(), &dg_ctx);
    }

    let client_multipath = test_ctx.cnx_client().is_multipath_enabled();
    let server_multipath = test_ctx.cnx_server().is_multipath_enabled();
    assert!(
        client_multipath && server_multipath,
        "multipath not fully negotiated (client={client_multipath}, server={server_multipath})"
    );
    assert_eq!(
        test_ctx.cnx_client().max_path_id_local(),
        test_ctx.cnx_server().max_path_id_remote(),
        "max_path_id_local/remote mismatch"
    );
    assert_eq!(
        test_ctx.cnx_client().max_path_id_remote(),
        test_ctx.cnx_server().max_path_id_local(),
        "max_path_id_remote/local mismatch"
    );

    wait_client_connection_ready(&mut test_ctx, &mut simulated_time).expect("client ready");

    if test_id == Ab1 {
        for _ in 0..7 {
            multipath_test_abandon_cycle(&mut test_ctx, &mut simulated_time)
                .expect("abandon cycle");
        }
    }

    let scenario = if matches!(test_id, SatPlus | Perf) {
        SCENARIO_MULTIPATH_LONG
    } else {
        SCENARIO_MULTIPATH
    };
    test_api_init_send_recv_scenario(&mut test_ctx, scenario).expect("init scenario");

    // Add second path.
    multipath_test_add_links(&mut test_ctx, false).expect("add links");
    if test_id == SatPlus {
        multipath_test_sat_links(&mut test_ctx, 1);
    } else if test_id == Perf {
        multipath_test_perf_links(&mut test_ctx, 1);
    } else if test_id == Fail {
        multipath_test_kill_server_links(&mut test_ctx, 1);
    }

    let server_addr = test_ctx.server_addr;
    let client_addr_2 = test_ctx.client_addr_2;
    test_ctx
        .cnx_client()
        .probe_new_path(&server_addr, &client_addr_2, simulated_time)
        .expect("probe new path");

    if test_id != Fail {
        wait_multipath_ready(&mut test_ctx, &mut simulated_time).expect("multipath ready");
        multipath_assert_ready(&mut test_ctx, test_id);
    }

    if test_id == StreamAf {
        test_ctx
            .cnx_server()
            .set_stream_path_affinity(4, 0)
            .expect("set stream affinity");
    } else if matches!(test_id, Backup | Standup) {
        test_ctx
            .cnx_client()
            .set_path_status(1, PathStatus::Backup)
            .expect("set backup");
    }

    // Mid-transfer link manipulation.
    if matches!(
        test_id,
        DropFirst
            | DropSecond
            | Renew
            | Nat
            | NatChallenge
            | Break1
            | Break2
            | Break3
            | Back0
            | Back1
            | Standup
            | Abandon
            | Tunnel
            | BreakBoth
    ) {
        tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, 640_000).expect("wait 640ms");

        match test_id {
            Renew => {
                test_ctx
                    .cnx_client()
                    .renew_connection_id(1)
                    .expect("renew CID");
            }
            Nat | NatChallenge => {
                let mut natted = test_ctx.client_addr;
                let port = natted.port();
                natted.set_port(port + 7);
                test_ctx.client_addr_natted = natted;
                test_ctx.client_use_nat = true;
                if test_id == NatChallenge {
                    test_ctx.cnx_client().set_path_challenge_required(0, true);
                }
            }
            Abandon => {
                test_ctx
                    .cnx_client()
                    .abandon_path(0, 0, simulated_time)
                    .expect("abandon path 0");
            }
            Break2 => {
                multipath_test_set_unreachable(&mut test_ctx, 1);
            }
            Back0 | Break3 => {
                multipath_test_set_unreachable(&mut test_ctx, 0);
            }
            Tunnel => {
                multipath_test_kill_links(&mut test_ctx, 0);
                multipath_test_kill_links(&mut test_ctx, 1);
            }
            BreakBoth => {
                multipath_test_set_unreachable(&mut test_ctx, 0);
                multipath_test_set_unreachable(&mut test_ctx, 1);
            }
            _ => {
                multipath_test_kill_links(
                    &mut test_ctx,
                    if matches!(test_id, DropFirst | Standup) {
                        0
                    } else {
                        1
                    },
                );
            }
        }
    }

    // Recovery for back/tunnel scenarios.
    if matches!(test_id, Back0 | Back1 | Tunnel) {
        let timeout = if test_id == Tunnel {
            5_000_000
        } else {
            1_000_000
        };
        tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout)
            .expect("wait for recovery");
        if test_id == Back0 {
            multipath_test_set_reachable(&mut test_ctx, 0);
            let client_addr = test_ctx.client_addr;
            test_ctx
                .cnx_client()
                .probe_new_path(&server_addr, &client_addr, simulated_time)
                .expect("probe path 0");
        } else {
            if test_id == Tunnel {
                multipath_test_unkill_links(&mut test_ctx, 0, simulated_time);
            }
            multipath_test_unkill_links(&mut test_ctx, 1, simulated_time);
        }
    }

    // Abandon cleanup delay.
    if test_id == Abandon {
        tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, 1_100_000)
            .expect("wait after abandon");
    }

    // Trigger datagram transmission.
    if matches!(test_id, Datagram | DgAf) {
        multipath_set_datagram_ready(&mut test_ctx, &dg_ctx, test_id).expect("set datagram ready");
    }

    // Final data loop.
    let final_data_result = if matches!(test_id, Datagram | DgAf) {
        multipath_datagram_send_loop(&mut test_ctx, &dg_ctx, &mut loss_mask, &mut simulated_time)
    } else {
        tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
    };

    if test_id == BreakBoth {
        let final_result = final_data_result.and_then(|_| {
            tls_api_one_scenario_body_verify(
                &mut test_ctx,
                &mut simulated_time,
                max_completion_microsec,
            )
        });
        assert!(
            final_result.is_err(),
            "break_both unexpectedly completed transfer and verification"
        );
        return;
    }

    if matches!(test_id, Datagram | DgAf) {
        final_data_result.expect("datagram send loop");
    } else {
        final_data_result.expect("data sending loop");
    }

    if test_id == KeepAlive {
        multipath_test_do_keep_alive(&mut test_ctx, &mut simulated_time).expect("keep alive");
    }

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, max_completion_microsec)
        .expect("scenario body verify");

    // --- Post-transfer assertions ---

    if matches!(test_id, Basic | JustOne) {
        multipath_verify_all_cid_available(test_ctx.cnx_client())
            .expect("all CIDs available on client");
        multipath_verify_all_cid_available(test_ctx.cnx_server())
            .expect("all CIDs available on server");
    }

    if test_id == Fail {
        assert_eq!(
            test_ctx.cnx_client().nb_paths(),
            1,
            "client should have 1 path after fail"
        );
        if test_ctx.has_cnx_server() {
            assert_eq!(
                test_ctx.cnx_server().nb_paths(),
                1,
                "server should have 1 path after fail"
            );
        }
    }

    if test_id == Renew {
        let original_r_cid_sequence = 0u64;
        assert_ne!(
            test_ctx.cnx_client().path_remote_cnxid_sequence(1),
            original_r_cid_sequence,
            "client path 1 remote CID not renewed"
        );
        assert_ne!(
            test_ctx.cnx_server().path_remote_cnxid_sequence(1),
            original_r_cid_sequence,
            "server path 1 remote CID not renewed"
        );
        assert_ne!(
            test_ctx.cnx_server().path_local_cnxid_sequence(1),
            original_r_cid_sequence,
            "server path 1 local CID not renewed"
        );
    }

    if test_id == Rotation {
        assert!(
            test_ctx.cnx_server().nb_crypto_key_rotations() > 0,
            "no key rotation observed"
        );
    }

    // Verify path IDs are unique and consistent.
    let nb_paths = test_ctx.cnx_client().nb_paths();
    let mut path_id_mask = 0u64;
    for i in 0..nb_paths {
        let uid = test_ctx.cnx_client().path_unique_id(i);
        assert!(uid <= 63, "path ID[{i}] = {uid}, too big");
        assert_eq!(
            path_id_mask & (1u64 << uid),
            0,
            "path ID[{i}] = {uid}, reuse"
        );
        path_id_mask |= 1u64 << uid;
        if test_ctx.cnx_client().path_local_cnxid_path_id(i) != u64::MAX {
            assert_eq!(
                uid,
                test_ctx.cnx_client().path_local_cnxid_path_id(i),
                "path ID mismatch at index {i}"
            );
        }
    }

    if matches!(test_id, Nat | NatChallenge) {
        assert!(
            test_ctx.cnx_server().nb_paths() >= 2,
            "server should have >= 2 paths"
        );
        assert!(
            test_ctx.cnx_client().nb_paths() >= 2,
            "client should have >= 2 paths"
        );
        assert_eq!(test_ctx.cnx_server().path_unique_id(0), 0);
        assert_eq!(test_ctx.cnx_client().path_unique_id(0), 0);
        assert_eq!(
            test_ctx.cnx_server().path_peer_addr_by_index(0),
            test_ctx.client_addr_natted,
            "NAT traversal: server path 0 peer addr wrong"
        );
    }

    if matches!(test_id, Break1 | Break2 | Break3 | Abandon) {
        assert_eq!(
            test_ctx.cnx_server().nb_paths(),
            1,
            "server should have 1 path after break"
        );
        assert_eq!(
            test_ctx.cnx_client().nb_paths(),
            1,
            "client should have 1 path after break"
        );
    }

    if matches!(test_id, Back0 | Back1) {
        assert_eq!(
            test_ctx.cnx_server().nb_paths(),
            2,
            "server should have 2 paths after back"
        );
        assert_eq!(
            test_ctx.cnx_client().nb_paths(),
            2,
            "client should have 2 paths after back"
        );
    }

    if test_id == StreamAf {
        assert_eq!(
            test_ctx.cnx_server().nb_paths(),
            2,
            "server should have 2 paths"
        );
        assert!(
            test_ctx.cnx_server().path_delivered(0) >= 1_400_000,
            "not enough data on server path 0"
        );
        assert!(
            test_ctx.cnx_server().path_delivered(1) <= 600_000,
            "too much data on server path 1"
        );
    }

    if matches!(test_id, Datagram | DgAf) {
        multipath_verify_datagram_sent(&dg_ctx.borrow(), test_id).expect("datagram verify");
    }

    if test_id == Backup {
        assert!(
            test_ctx.cnx_server().path_is_backup(1),
            "server path 1 should be backup"
        );
        assert!(
            test_ctx.cnx_server().path_delivered(1) <= 50_000,
            "too much data on backup path"
        );
    }

    if test_id == Discovery {
        assert!(
            test_ctx.nb_address_observed >= 2,
            "got {} addresses observed",
            test_ctx.nb_address_observed
        );
        for p in 0..2 {
            let local = test_ctx.cnx_client().path_local_addr_by_index(p);
            let observed = test_ctx.cnx_client().path_observed_addr_by_index(p);
            assert_eq!(
                local, observed,
                "path[{p}] local ({local}) != observed ({observed})"
            );
        }
    }

    // Verify callbacks after context teardown.
    if matches!(test_id, Callback | Quality | QualityServer) {
        multipath_verify_callbacks(test_id).expect("verify callbacks");
    }
}

// ---------------------------------------------------------------------------
// monopath_test_one — shared by all monopath_* tests.
// C: `monopath_test_one`.

fn monopath_test_one(test_case: MonopathTestId) {
    const LATENCY: u64 = 10_000;
    let mut simulated_time = Instant::from_ticks(0);

    let mut cid_bytes = [0xba_u8, 0xba, 1, 0, 0, 0, 0, 0];
    cid_bytes[7] = test_case as u8;
    let initial_cid = ConnectionId::clone_from_slice(&cid_bytes).expect("monopath CID");

    let client_params = multipath_init_params(false);
    let server_params = multipath_init_params(false);

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        Some(&client_params),
        Some(&server_params),
        Some(&initial_cid),
    )
    .expect("tls_api_one_scenario_init_ex");

    test_ctx.c_to_s_link.microsec_latency = LATENCY;
    test_ctx.s_to_c_link.microsec_latency = LATENCY;

    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;
    test_ctx.qclient.set_binlog(Some(".")).ok();
    test_ctx.qclient.use_long_log = true;

    if test_case == MonopathTestId::Hole {
        test_ctx.qserver.set_optimistic_ack_policy(256); // PICOQUIC_DEFAULT_HOLE_PERIOD
        // C: picoquic_public_random_seed_64(RANDOM_PUBLIC_TEST_SEED, 1)
        public_random_seed_64(0xDEAD_BEEF_CAFE_C001, 1);
    }
    if test_case == MonopathTestId::Rotation {
        test_ctx.qserver.set_default_crypto_epoch_length(200);
    }

    tls_api_one_scenario_body_connect(&mut test_ctx, &mut simulated_time, 0, 0)
        .expect("body connect");

    test_api_init_send_recv_scenario(&mut test_ctx, SCENARIO_MULTIPATH).expect("init scenario");

    let mut loss_mask = test_ctx.loss_mask_default;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");

    let max_completion_microsec = if test_case == MonopathTestId::KeepAlive {
        multipath_test_do_keep_alive(&mut test_ctx, &mut simulated_time).expect("keep alive");
        202_000_000u64
    } else {
        2_200_000u64
    };

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, max_completion_microsec)
        .expect("scenario body verify");

    if test_case == MonopathTestId::Hole {
        assert!(
            test_ctx.cnx_server().nb_packet_holes_inserted() > 0,
            "no holes inserted"
        );
    } else if test_case == MonopathTestId::Rotation {
        assert!(
            test_ctx.cnx_server().nb_crypto_key_rotations() > 0,
            "no key rotation observed"
        );
    }
}

// ===========================================================================
// Test entries.
// ===========================================================================

/// C: `migration_controlled_test`.
#[test]
fn migration_controlled() {
    migration_test_one(false);
}

/// C: `migration_mtu_drop_test`.
#[test]
fn migration_mtu_drop() {
    migration_test_one(true);
}

/// C: `monopath_0rtt_test` (first occurrence in test table).
#[test]
fn monopath_0rtt() {
    let zrt = ZeroRttTest {
        do_multipath: true,
        ..ZeroRttTest::default()
    };
    zero_rtt_test_one(&zrt).expect("monopath_0rtt");
}

/// C: `monopath_0rtt_test` (second occurrence in test table).
#[test]
fn monopath_0rtt_2() {
    let zrt = ZeroRttTest {
        do_multipath: true,
        ..ZeroRttTest::default()
    };
    zero_rtt_test_one(&zrt).expect("monopath_0rtt_2");
}

/// C: `monopath_0rtt_loss_test` (first occurrence in test table).
#[test]
fn monopath_0rtt_loss() {
    for i in 1u32..16 {
        let zrt = ZeroRttTest {
            early_loss: 1u64 << i,
            do_multipath: true,
            ..ZeroRttTest::default()
        };
        zero_rtt_test_one(&zrt)
            .unwrap_or_else(|_| panic!("monopath_0rtt_loss fails at packet #{i}"));
    }
}

/// C: `monopath_0rtt_loss_test` (second occurrence in test table).
#[test]
fn monopath_0rtt_loss_2() {
    for i in 1u32..16 {
        let zrt = ZeroRttTest {
            early_loss: 1u64 << i,
            do_multipath: true,
            ..ZeroRttTest::default()
        };
        zero_rtt_test_one(&zrt)
            .unwrap_or_else(|_| panic!("monopath_0rtt_loss_2 fails at packet #{i}"));
    }
}

/// C: `monopath_basic_test`.
#[test]
fn monopath_basic() {
    monopath_test_one(MonopathTestId::Basic);
}

/// C: `monopath_hole_test`.
#[test]
fn monopath_hole() {
    monopath_test_one(MonopathTestId::Hole);
}

/// C: `monopath_keep_alive_test`.
#[test]
fn monopath_keep_alive() {
    monopath_test_one(MonopathTestId::KeepAlive);
}

/// C: `monopath_rotation_test`.
#[test]
fn monopath_rotation() {
    monopath_test_one(MonopathTestId::Rotation);
}

/// C: `multipath_ab1_test`.
#[test]
fn multipath_ab1() {
    multipath_test_one(3_000_000, MultipathTestId::Ab1);
}

/// C: `multipath_abandon_test`.
#[test]
fn multipath_abandon() {
    multipath_test_one(3_800_000, MultipathTestId::Abandon);
}

/// C: `multipath_aead_test`.
#[test]
fn multipath_aead() {
    const MP_AEAD_SECRET: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        35, 26, 27, 28, 29, 30, 31,
    ];

    let aead_encrypt = setup_test_aead_context(true, &MP_AEAD_SECRET, LABEL_QUIC_V1_KEY_BASE)
        .expect("aead_encrypt context");
    let aead_decrypt = setup_test_aead_context(false, &MP_AEAD_SECRET, LABEL_QUIC_V1_KEY_BASE)
        .expect("aead_decrypt context");

    let path_id_test: &[u64] = &[0, 1, 2, 0x0123456789abcdef];
    let sequence = 12345u64;
    let aad = b"This is a test";
    let test_input = b"The quick brown fox jumps over the lazy dog";

    let mut encrypted = [0u8; 256];
    let mut decrypted = [0u8; 256];

    for (i, &enc_path) in path_id_test.iter().enumerate() {
        let encrypted_len = aead_encrypt_mp(
            &mut encrypted,
            test_input,
            enc_path,
            sequence,
            aad,
            aead_encrypt.as_ref(),
        );

        for (j, &dec_path) in path_id_test.iter().enumerate() {
            let result = aead_decrypt_mp(
                &mut decrypted,
                &encrypted[..encrypted_len],
                dec_path,
                sequence,
                aad,
                aead_decrypt.as_ref(),
            );

            if i != j {
                assert!(
                    result.is_none(),
                    "unexpected decrypt success: enc path 0x{:x}, dec path 0x{:x}",
                    enc_path,
                    dec_path
                );
            } else {
                let dec_len = result
                    .unwrap_or_else(|| panic!("unexpected decrypt error, path 0x{:x}", enc_path));
                assert_eq!(
                    dec_len,
                    test_input.len(),
                    "length mismatch at path 0x{:x}",
                    enc_path
                );
                assert_eq!(
                    &decrypted[..dec_len],
                    test_input,
                    "decoded mismatch at path 0x{:x}",
                    enc_path
                );
            }
        }
    }
}

/// C: `multipath_back0_test`.
#[test]
fn multipath_back0() {
    multipath_test_one(3_300_000, MultipathTestId::Back0);
}

/// C: `multipath_back1_test`.
#[test]
fn multipath_back1() {
    multipath_test_one(3_300_000, MultipathTestId::Back1);
}

/// C: `multipath_backup_test`.
#[test]
fn multipath_backup() {
    multipath_test_one(2_000_000, MultipathTestId::Backup);
}

/// C: `multipath_basic_test`.
#[test]
fn multipath_basic() {
    multipath_test_one(1_060_000, MultipathTestId::Basic);
}

/// C: `multipath_break1_test`.
#[test]
fn multipath_break1() {
    multipath_test_one(10_800_000, MultipathTestId::Break1);
}

/// C: `multipath_break_both_test`.
#[test]
fn multipath_break_both() {
    multipath_test_one(1_060_000, MultipathTestId::BreakBoth);
}

/// C: `multipath_callback_test`.
#[test]
fn multipath_callback() {
    multipath_test_one(1_000_000, MultipathTestId::Callback);
}

/// C: `multipath_datagram_test`.
#[test]
fn multipath_datagram() {
    multipath_test_one(1_150_000, MultipathTestId::Datagram);
}

/// C: `multipath_dg_af_test`.
#[test]
fn multipath_dg_af() {
    multipath_test_one(1_100_000, MultipathTestId::DgAf);
}

/// C: `multipath_discovery_test`.
#[test]
fn multipath_discovery() {
    multipath_test_one(2_000_000, MultipathTestId::Discovery);
}

/// C: `multipath_drop_first_test`.
#[test]
fn multipath_drop_first() {
    multipath_test_one(1_490_000, MultipathTestId::DropFirst);
}

/// C: `multipath_drop_second_test`.
#[test]
fn multipath_drop_second() {
    multipath_test_one(1_260_000, MultipathTestId::DropSecond);
}

/// C: `multipath_fail_test`.
#[test]
fn multipath_fail() {
    multipath_test_one(2_000_000, MultipathTestId::Fail);
}

/// C: `multipath_just_one_test`.
#[test]
fn multipath_just_one() {
    multipath_test_one(1_060_000, MultipathTestId::JustOne);
}

/// C: `multipath_keep_alive_test`.
#[test]
fn multipath_keep_alive() {
    multipath_test_one(210_000_000, MultipathTestId::KeepAlive);
}

/// C: `multipath_nat_test`.
#[test]
fn multipath_nat() {
    multipath_test_one(3_000_000, MultipathTestId::Nat);
}

/// C: `multipath_nat_challenge_test`.
#[test]
fn multipath_nat_challenge() {
    multipath_test_one(3_000_000, MultipathTestId::NatChallenge);
}

/// C: `multipath_perf_test`.
#[test]
fn multipath_perf() {
    multipath_test_one(1_650_000, MultipathTestId::Perf);
}

/// C: `multipath_qlog_test`.
#[test]
fn multipath_qlog() {
    const MULTIPATH_TRACE_QLOG: &str = "0807060504030201.server.qlog";
    const MULTIPATH_QLOG_REF: &str = "picoquictest/multipath_qlog_ref.txt";

    // Delete any existing qlog file.
    let _ = std::fs::remove_file(MULTIPATH_TRACE_QLOG);

    multipath_trace_test_one(true);

    compare_text_files(MULTIPATH_TRACE_QLOG, MULTIPATH_QLOG_REF).expect("qlog matches reference");
}

/// C: `multipath_quality_test`.
#[test]
fn multipath_quality() {
    multipath_test_one(1_000_000, MultipathTestId::Quality);
}

/// C: `multipath_renew_test`.
#[test]
fn multipath_renew() {
    multipath_test_one(3_000_000, MultipathTestId::Renew);
}

/// C: `multipath_rotation_test`.
#[test]
fn multipath_rotation() {
    multipath_test_one(3_000_000, MultipathTestId::Rotation);
}

/// C: `multipath_sat_plus_test`.
#[test]
fn multipath_sat_plus() {
    multipath_test_one(10_000_000, MultipathTestId::SatPlus);
}

/// C: `multipath_socket0_error_test`.
#[test]
fn multipath_socket0_error() {
    multipath_test_one(10_900_000, MultipathTestId::Break3);
}

/// C: `multipath_socket_error_test`.
#[test]
fn multipath_socket_error() {
    multipath_test_one(11_000_000, MultipathTestId::Break2);
}

/// C: `multipath_standup_test`.
#[test]
fn multipath_standup() {
    multipath_test_one(7_200_000, MultipathTestId::Standup);
}

/// C: `multipath_stream_af_test`.
#[test]
fn multipath_stream_af() {
    multipath_test_one(1_500_000, MultipathTestId::StreamAf);
}

/// C: `multipath_tunnel_test`.
#[test]
fn multipath_tunnel() {
    multipath_test_one(12_000_000, MultipathTestId::Tunnel);
}
