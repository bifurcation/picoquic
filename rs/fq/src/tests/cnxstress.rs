//! Test cases for `picoquictest/cnxstress.c`.
//!
//! Exercises connection-level stress — multiple parallel client connections,
//! message passing, connection teardown — and connection-limit enforcement.
//!
//! The stress context creates a server `Quic`, a client `Quic`, and two
//! simulated links.  A loop drives connection setup, message exchange, and
//! teardown.  `cnx_limit` verifies that the stack sends `SERVER_BUSY` when
//! the connection table is full.

use std::cell::RefCell;
use std::rc::Rc;

use core::net::SocketAddr;

use crate::errors::TransportError;
use crate::tests::util::{
    TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_KEY, TestSimLink, TestSimPacket, test_random,
};
use crate::tp::TransportParameters;
use crate::{
    CallbackEvent, Connection, ConnectionId, Duration, Error, Instant, Quic, RESET_SECRET_SIZE,
    State, StreamDataCallback, current_time,
};

const CNX_STRESS_ALPN: &str = "cnxstress";

// ---------------------------------------------------------------------------
// Stream context.

/// Per-stream state for both sending and receiving sides.
/// C: `cnx_stress_stream_ctx_t`.
#[allow(dead_code)]
struct CnxStressStreamCtx {
    stream_id: u64,
    /// Timestamp embedded in the first 8 bytes of the message payload.
    send_time: u64,
    /// Total message size encoded in bytes 8–15.
    nb_bytes_expected: u64,
    nb_bytes_received: u64,
    nb_bytes_sent: u64,
}

// ---------------------------------------------------------------------------
// Shared state (callback ↔ outer loop).

/// Fields that both the connection callbacks and the outer simulation loop
/// read or write.  Wrapped in `Rc<RefCell<>>` so callbacks can hold a clone.
/// C: the callback-mutated subset of `cnx_stress_ctx_t`.
struct CnxStressShared {
    nb_clients: usize,
    nb_servers: usize,
    nb_clients_deleted: usize,
    /// `c_active[rank]` is `true` while the client connection at that slot
    /// is live.  Set to `false` in the close/application_close callback.
    c_active: Vec<bool>,
    /// Same for server-side connections.
    s_active: Vec<bool>,
    is_limit_test: bool,
    limit_test_got_server_busy: bool,
    nb_client_target: usize,
    nb_messages_target: usize,
    message_size: u64,
    nb_messages_sent: usize,
    nb_messages_errors: usize,
    nb_messages_received: usize,
    sum_message_delays: i64,
    sum_square_message_delays: f64,
    message_delay_min: i64,
    message_delay_max: i64,
    /// Mirrors `CnxStressCtx::simulated_time`; updated each loop iteration so
    /// callbacks can compute `time_now – send_time` without access to the
    /// outer context.  C: obtained via `picoquic_get_quic_time(cnx->quic)`.
    simulated_time: u64,
}

// ---------------------------------------------------------------------------
// Outer simulation loop context.

/// Full simulation state.  C: `cnx_stress_ctx_t`.
struct CnxStressCtx {
    shared: Rc<RefCell<CnxStressShared>>,
    qserver: Box<Quic>,
    qclient: Box<Quic>,
    server_addr: SocketAddr,
    client_addr: SocketAddr,
    link_to_clients: Box<TestSimLink>,
    link_to_server: Box<TestSimLink>,
    client_creation_interval: u64,
    next_client_creation_time: u64,
    client_deletion_interval: u64,
    next_client_deletion_time: u64,
    message_creation_interval: u64,
    next_message_creation_time: u64,
    /// Initial source CIDs of client connections, indexed by rank.
    /// `None` means the slot is vacant (never filled or connection closed).
    client_connections: Vec<Option<ConnectionId>>,
    /// Same for server-side connections.
    server_connections: Vec<Option<ConnectionId>>,
}

impl CnxStressCtx {
    fn simulated_time(&self) -> u64 {
        self.shared.borrow().simulated_time
    }
}

// ---------------------------------------------------------------------------
// Connection callback.

/// Per-connection callback context.  C: `cnx_stress_callback_ctx_t`.
///
/// `mode` values:
/// * `0` — client connection
/// * `1` — server connection (created after the default context fires)
/// * `2` — default server context (installed on `qserver`); on the first
///   non-close event it replaces itself with a mode-1 callback.
struct CnxStressCallback {
    shared: Rc<RefCell<CnxStressShared>>,
    mode: i32,
    rank: usize,
    streams: Vec<CnxStressStreamCtx>,
}

impl CnxStressCallback {
    fn find_stream_mut(&mut self, stream_id: u64) -> Option<&mut CnxStressStreamCtx> {
        self.streams.iter_mut().find(|s| s.stream_id == stream_id)
    }

    fn create_stream(&mut self, stream_id: u64) -> &mut CnxStressStreamCtx {
        self.streams.push(CnxStressStreamCtx {
            stream_id,
            send_time: 0,
            nb_bytes_expected: 0,
            nb_bytes_received: 0,
            nb_bytes_sent: 0,
        });
        self.streams.last_mut().unwrap()
    }

    fn delete_stream(&mut self, connection: &mut Connection, stream_id: u64) {
        connection.unlink_app_stream_ctx(stream_id);
        self.streams.retain(|s| s.stream_id != stream_id);
    }

    /// Fill the next chunk of an active message stream.
    /// C: `cnx_stress_callback_prepare_to_send`.
    fn prepare_to_send(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        length: usize,
        stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        fn build_payload(stream: &mut CnxStressStreamCtx, length: usize) -> (Vec<u8>, bool) {
            let remaining = stream
                .nb_bytes_expected
                .saturating_sub(stream.nb_bytes_sent) as usize;
            let data_length = length.min(remaining);
            let mut payload = vec![b'z'; data_length];

            for byte in &mut payload {
                if stream.nb_bytes_sent < 8 {
                    let shift = 8 * (7 - stream.nb_bytes_sent);
                    *byte = ((stream.send_time >> shift) & 0xff) as u8;
                } else if stream.nb_bytes_sent < 16 {
                    let shift = 8 * (15 - stream.nb_bytes_sent);
                    *byte = ((stream.nb_bytes_expected >> shift) & 0xff) as u8;
                }
                stream.nb_bytes_sent += 1;
            }

            (payload, stream.nb_bytes_sent >= stream.nb_bytes_expected)
        }

        let (payload, is_fin) = if let Some(any_ctx) = stream_ctx {
            let Some(stream) = any_ctx.downcast_mut::<CnxStressStreamCtx>() else {
                return -1;
            };
            build_payload(stream, length)
        } else {
            let Some(stream) = self.find_stream_mut(stream_id) else {
                return -1;
            };
            build_payload(stream, length)
        };

        if connection
            .add_to_stream(stream_id, &payload, is_fin)
            .is_err()
        {
            return -1;
        }
        if is_fin {
            self.delete_stream(connection, stream_id);
        }
        0
    }

    /// Handle incoming stream data / FIN.
    /// C: `cnx_stress_callback_data`.
    fn handle_data(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        is_fin: bool,
    ) -> i32 {
        if self.find_stream_mut(stream_id).is_none() {
            let stream = self.create_stream(stream_id);
            stream.stream_id = stream_id;
            if connection.set_app_stream_ctx(stream_id, None).is_err() {
                return -1;
            }
        }
        let stream = self.find_stream_mut(stream_id).unwrap();
        let mut pos = 0usize;
        let buf = bytes;
        while pos < buf.len() && stream.nb_bytes_received < 8 {
            stream.send_time = (stream.send_time << 8) | buf[pos] as u64;
            pos += 1;
            stream.nb_bytes_received += 1;
        }
        while pos < buf.len() && stream.nb_bytes_received < 16 {
            stream.nb_bytes_expected = (stream.nb_bytes_expected << 8) | buf[pos] as u64;
            pos += 1;
            stream.nb_bytes_received += 1;
        }
        stream.nb_bytes_received += (buf.len() - pos) as u64;

        if is_fin {
            let (recv, expected, send_time) = (
                stream.nb_bytes_received,
                stream.nb_bytes_expected,
                stream.send_time,
            );
            {
                let mut shared = self.shared.borrow_mut();
                if recv < 16 || recv < expected {
                    shared.nb_messages_errors += 1;
                } else {
                    let time_now = shared.simulated_time;
                    let delta_t = time_now as i64 - send_time as i64;
                    shared.nb_messages_received += 1;
                    shared.sum_message_delays += delta_t;
                    shared.sum_square_message_delays += (delta_t as f64) * (delta_t as f64);
                    if delta_t > shared.message_delay_max {
                        shared.message_delay_max = delta_t;
                    }
                    if delta_t < shared.message_delay_min {
                        shared.message_delay_min = delta_t;
                    }
                }
            }
            self.delete_stream(connection, stream_id);
        }
        0
    }

    /// Called when the connection is closed (any flavour).
    /// C: `cnx_stress_callback_delete_context`.
    fn handle_close(&mut self, connection: &mut Connection) {
        {
            let mut shared = self.shared.borrow_mut();
            if self.mode == 0 {
                if shared.is_limit_test
                    && self.rank + 1 == shared.nb_client_target
                    && connection.state() == State::Disconnected
                    && connection.remote_error() == TransportError::ServerBusy as u64
                {
                    shared.limit_test_got_server_busy = true;
                }
                if self.rank < shared.c_active.len() {
                    shared.c_active[self.rank] = false;
                }
            } else if self.mode == 1 && self.rank < shared.s_active.len() {
                shared.s_active[self.rank] = false;
            }
        }
        // Drain remaining stream contexts.
        let stream_ids: Vec<u64> = self.streams.iter().map(|s| s.stream_id).collect();
        for sid in stream_ids {
            connection.unlink_app_stream_ctx(sid);
        }
        self.streams.clear();
        connection.set_callback(None);
    }
}

impl StreamDataCallback for CnxStressCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        // Default context (mode == 2): on first real event, spawn a mode-1
        // server context and install it on the connection.
        if self.mode == 2 {
            match fin_or_event {
                CallbackEvent::StatelessReset
                | CallbackEvent::Close
                | CallbackEvent::ApplicationClose => return 0,
                _ => {
                    let new_rank = {
                        let mut shared = self.shared.borrow_mut();
                        let rank = shared.nb_servers;
                        if rank < shared.s_active.len() {
                            shared.s_active[rank] = true;
                        }
                        shared.nb_servers += 1;
                        rank
                    };
                    let new_cb = Box::new(CnxStressCallback {
                        shared: Rc::clone(&self.shared),
                        mode: 1,
                        rank: new_rank,
                        streams: Vec::new(),
                    });
                    connection.set_callback(Some(new_cb));
                    // The new callback will handle subsequent events; for the
                    // current delivery the stack will re-invoke after set_callback.
                    return 0;
                }
            }
        }

        match fin_or_event {
            CallbackEvent::StreamData => self.handle_data(connection, stream_id, bytes, false),
            CallbackEvent::StreamFin => self.handle_data(connection, stream_id, bytes, true),
            CallbackEvent::StreamReset => {
                self.shared.borrow_mut().nb_messages_errors += 1;
                self.delete_stream(connection, stream_id);
                0
            }
            CallbackEvent::StopSending => 0,
            CallbackEvent::StatelessReset
            | CallbackEvent::Close
            | CallbackEvent::ApplicationClose => {
                self.handle_close(connection);
                0
            }
            CallbackEvent::StreamGap => 0,
            CallbackEvent::PrepareToSend => {
                self.prepare_to_send(connection, stream_id, bytes.len(), stream_ctx)
            }
            CallbackEvent::AlmostReady | CallbackEvent::Ready => 0,
            CallbackEvent::Datagram => 0,
            CallbackEvent::VersionNegotiation => 0,
            CallbackEvent::RequestAlpnList | CallbackEvent::SetAlpn => 0,
            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Simulation helpers.

/// Find an active connection slot, cycling from `msg_num % nb_ctx`.
/// C: `cnx_stress_cnx_from_rank`.
fn cnx_stress_cnx_from_rank(
    msg_num: usize,
    nb_ctx: usize,
    active: &[bool],
    connections: &[Option<ConnectionId>],
) -> Option<(usize, ConnectionId)> {
    if nb_ctx == 0 {
        return None;
    }
    let start = msg_num % nb_ctx;
    for i in 0..nb_ctx {
        let rank = (start + i) % nb_ctx;
        if active.get(rank).copied().unwrap_or(false)
            && let Some(Some(cid)) = connections.get(rank)
        {
            return Some((rank, *cid));
        }
    }
    None
}

/// Initiate a new unidirectional message stream on an active connection.
/// C: `cnx_stress_initiate_message`.
fn cnx_stress_initiate_message(ctx: &mut CnxStressCtx) -> crate::Result<()> {
    let (is_client_turn, nb_ctx, message_size, msg_num) = {
        let mut shared = ctx.shared.borrow_mut();
        shared.nb_messages_sent += 1;
        let is_client = (shared.nb_messages_sent & 1) != 0;
        let nb = if is_client {
            shared.nb_clients
        } else {
            shared.nb_servers
        };
        (is_client, nb, shared.message_size, shared.nb_messages_sent)
    };

    let slot = {
        let shared = ctx.shared.borrow();
        let active = if is_client_turn {
            &shared.c_active
        } else {
            &shared.s_active
        };
        let conns = if is_client_turn {
            &ctx.client_connections
        } else {
            &ctx.server_connections
        };
        cnx_stress_cnx_from_rank(msg_num, nb_ctx, active, conns)
    };

    let (_rank, cid) = slot.ok_or(Error::Generic)?;
    let simulated_time = ctx.simulated_time();
    let quic = if is_client_turn {
        &mut ctx.qclient
    } else {
        &mut ctx.qserver
    };
    let connection = quic.connection_ref_by_id(cid).ok_or(Error::Generic)?;
    let stream_id = connection.get_next_local_stream_id(true);
    let stream_ctx = CnxStressStreamCtx {
        stream_id,
        send_time: simulated_time,
        nb_bytes_expected: message_size,
        nb_bytes_received: 0,
        nb_bytes_sent: 0,
    };
    connection.mark_active_stream(stream_id, true, Some(Box::new(stream_ctx)))
}

/// Create a new client QUIC connection and start it.
/// C: `cnx_stress_create_client_cnx`.
fn cnx_stress_create_client_cnx(ctx: &mut CnxStressCtx) -> crate::Result<()> {
    let simulated_time = ctx.simulated_time();
    let server_addr = ctx.server_addr;
    let rank = {
        let shared = ctx.shared.borrow();
        shared.nb_clients
    };

    // Allocate a slot for the ConnectionId.
    if rank >= ctx.client_connections.len() {
        return Err(Error::Generic);
    }

    // Create the connection; set the callback before starting.
    let cb = Box::new(CnxStressCallback {
        shared: Rc::clone(&ctx.shared),
        mode: 0,
        rank,
        streams: Vec::new(),
    });
    {
        let shared = ctx.shared.borrow_mut();
        let _ = shared; // just to keep borrow alive during build
    }
    let cnx_id_null = ConnectionId::with_size(0).ok_or(Error::Generic)?;
    let cnx = ctx
        .qclient
        .create_connection(
            cnx_id_null,
            cnx_id_null,
            Some(&server_addr),
            Instant::from_ticks(simulated_time),
            0,
            Some(crate::tests::util::TEST_SNI),
            Some(CNX_STRESS_ALPN),
            true,
        )
        .ok_or(Error::Memory)?;

    // Store the connection's initial CID for later lookup.
    let initial_cid = ConnectionId::with_size(0).ok_or(Error::Generic)?;
    cnx.set_callback(Some(cb));
    cnx.enable_keep_alive(Duration::from_ticks(0));
    cnx.start_client()?;

    ctx.client_connections[rank] = Some(initial_cid);
    ctx.shared.borrow_mut().c_active[rank] = true;
    ctx.shared.borrow_mut().nb_clients += 1;
    Ok(())
}

/// Close the next-in-line client connection.
/// C: `cnx_stress_close_one_connection`.
fn cnx_stress_close_one_connection(ctx: &mut CnxStressCtx) -> crate::Result<()> {
    ctx.shared.borrow_mut().nb_clients_deleted += 1;
    let rank = {
        let shared = ctx.shared.borrow();
        shared.nb_clients.saturating_sub(shared.nb_clients_deleted)
    };
    if let Some(Some(cid)) = ctx.client_connections.get(rank) {
        let cid = *cid;
        if let Some(cnx) = ctx.qclient.connection_ref_by_id(cid) {
            cnx.close(0)?;
        }
    }
    Ok(())
}

/// Dequeue one packet from `link` and deliver it to `quic`.
/// C: `cnx_stress_link_arrival`.
fn cnx_stress_link_arrival(
    quic: &mut Quic,
    link: &mut TestSimLink,
    current_time: Instant,
) -> crate::Result<()> {
    if let Some(mut packet) = link.dequeue(current_time) {
        quic.incoming_packet(
            &mut packet.bytes[..packet.length],
            &packet
                .addr_from
                .unwrap_or(SocketAddr::from(([0u8; 4], 0u16))),
            &packet.addr_to.unwrap_or(SocketAddr::from(([0u8; 4], 0u16))),
            0,
            0,
            current_time,
        )?;
    }
    Ok(())
}

/// Prepare and send one outbound packet from `quic` onto `link`.
/// C: `cnx_stress_prepare`.
fn cnx_stress_prepare(
    quic: &mut Quic,
    link: &mut TestSimLink,
    default_source: SocketAddr,
    current_time: Instant,
) -> crate::Result<()> {
    let mut packet = TestSimPacket::create()?;
    let prepared = quic.prepare_next_packet(current_time, &mut packet.bytes)?;
    if prepared.send_length > 0 {
        packet.length = prepared.send_length;
        packet.addr_to = Some(prepared.addr_to);
        let addr_from = if prepared.addr_from.ip().is_unspecified() {
            default_source
        } else {
            prepared.addr_from
        };
        packet.addr_from = Some(addr_from);
        link.submit(packet, current_time);
    }
    Ok(())
}

/// Events the simulation loop can dispatch.
/// C: `cnx_stress_event_enum`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum CnxStressEvent {
    None,
    NewMessage,
    ClientCreation,
    ClientRemoval,
    ClientArrival,
    ClientPrepare,
    ServerArrival,
    ServerPrepare,
}

/// Run one step of the simulation, advancing `simulated_time` to the next
/// scheduled event and executing it.  C: `cnx_stress_loop_step`.
fn cnx_stress_loop_step(ctx: &mut CnxStressCtx) -> crate::Result<()> {
    let simulated_time = ctx.simulated_time();

    let mut next_event = CnxStressEvent::None;
    let mut next_time = u64::MAX;

    macro_rules! maybe_event {
        ($event:expr, $time:expr) => {
            if $time < next_time {
                next_event = $event;
                next_time = $time;
            }
        };
    }

    maybe_event!(CnxStressEvent::NewMessage, ctx.next_message_creation_time);
    maybe_event!(
        CnxStressEvent::ClientCreation,
        ctx.next_client_creation_time
    );
    maybe_event!(CnxStressEvent::ClientRemoval, ctx.next_client_deletion_time);

    // Next packet arrival on the link to clients.
    if let Some(first) = ctx.link_to_clients.packets.front() {
        maybe_event!(CnxStressEvent::ClientArrival, first.arrival_time.ticks());
    }
    // Next time the client QUIC context wants to act.
    let client_wake = ctx
        .qclient
        .next_wake_time(Instant::from_ticks(simulated_time));
    maybe_event!(CnxStressEvent::ClientPrepare, client_wake);

    // Next packet arrival on the link to server.
    if let Some(first) = ctx.link_to_server.packets.front() {
        maybe_event!(CnxStressEvent::ServerArrival, first.arrival_time.ticks());
    }
    // Next time the server QUIC context wants to act.
    let server_wake = ctx
        .qserver
        .next_wake_time(Instant::from_ticks(simulated_time));
    maybe_event!(CnxStressEvent::ServerPrepare, server_wake);

    // Advance simulated time.
    if next_time > simulated_time {
        ctx.shared.borrow_mut().simulated_time = next_time;
    }
    let now = Instant::from_ticks(ctx.simulated_time());

    match next_event {
        CnxStressEvent::NewMessage => {
            cnx_stress_initiate_message(ctx)?;
            let target = ctx.shared.borrow().nb_messages_target;
            let sent = ctx.shared.borrow().nb_messages_sent;
            if sent >= target {
                ctx.next_message_creation_time = u64::MAX;
            } else {
                ctx.next_message_creation_time += ctx.message_creation_interval;
            }
        }
        CnxStressEvent::ClientCreation => {
            cnx_stress_create_client_cnx(ctx)?;
            let nb = ctx.shared.borrow().nb_clients;
            let target = ctx.shared.borrow().nb_client_target;
            if nb >= target {
                ctx.next_client_creation_time = u64::MAX;
            } else {
                ctx.next_client_creation_time += ctx.client_creation_interval;
            }
        }
        CnxStressEvent::ClientRemoval => {
            cnx_stress_close_one_connection(ctx)?;
            let deleted = ctx.shared.borrow().nb_clients_deleted;
            let nb = ctx.shared.borrow().nb_clients;
            if deleted >= nb {
                ctx.next_client_deletion_time = u64::MAX;
            } else {
                ctx.next_client_deletion_time += ctx.client_deletion_interval;
            }
        }
        CnxStressEvent::ClientArrival => {
            cnx_stress_link_arrival(&mut ctx.qclient, &mut ctx.link_to_clients, now)?;
        }
        CnxStressEvent::ClientPrepare => {
            cnx_stress_prepare(
                &mut ctx.qclient,
                &mut ctx.link_to_server,
                ctx.client_addr,
                now,
            )?;
        }
        CnxStressEvent::ServerArrival => {
            cnx_stress_link_arrival(&mut ctx.qserver, &mut ctx.link_to_server, now)?;
        }
        CnxStressEvent::ServerPrepare => {
            cnx_stress_prepare(
                &mut ctx.qserver,
                &mut ctx.link_to_clients,
                ctx.server_addr,
                now,
            )?;
        }
        CnxStressEvent::None => return Err(Error::Generic),
    }

    Ok(())
}

/// Set transport parameters suitable for the stress test.
/// C: `cnx_stress_set_default_tp`.
fn cnx_stress_set_default_tp(quic: &mut Quic) -> crate::Result<()> {
    let tp = TransportParameters {
        initial_max_stream_data_bidi_local: 0,
        initial_max_stream_data_bidi_remote: 0,
        initial_max_stream_id_bidir: 0,
        initial_max_stream_data_uni: 0x20000,
        initial_max_stream_id_unidir: 64,
        initial_max_data: 0x20000,
        max_idle_timeout: Duration::from_ticks(60_000_000), // 60 s in µs
        max_packet_size: crate::MAX_PACKET_SIZE as u32,
        max_ack_delay: 10_000,
        active_connection_id_limit: 3,
        ack_delay_exponent: 3,
        migration_disabled: false,
        ..Default::default()
    };
    quic.set_default_tp(&tp)
}

/// Build the full stress-test context.
/// C: `cnx_stress_create_ctx`.
fn cnx_stress_create_ctx(
    duration: u64,
    nb_clients: usize,
    limit_test: bool,
) -> Option<CnxStressCtx> {
    let limit_extra = limit_test as usize;
    let nb_slots = nb_clients + limit_extra;

    // Validate timing parameters.
    let client_creation_interval = 2_000u64;
    let client_deletion_interval = 100u64;
    if (client_creation_interval + client_deletion_interval) * nb_clients as u64 > duration {
        return None;
    }
    let next_client_deletion_time = duration - client_deletion_interval * nb_clients as u64;
    let nb_messages_target = nb_clients.min(20_000);
    let message_size = 1024u64;
    let message_creation_interval = next_client_deletion_time
        .checked_div(3 * nb_messages_target as u64)
        .filter(|&x| x > 0)?;
    let next_message_creation_time = next_client_deletion_time / 3;

    // Seed the deterministic RNG the same way as C.  C: `picoquic_test_random`.
    let mut random_ctx = 0xBABAC001BADDBAB1_u64 ^ duration ^ (nb_clients as u64);
    test_random(&mut random_ctx);
    test_random(&mut random_ctx);

    // Shared mutable state.
    let shared = Rc::new(RefCell::new(CnxStressShared {
        nb_clients: 0,
        nb_servers: 0,
        nb_clients_deleted: 0,
        c_active: vec![false; nb_slots],
        s_active: vec![false; nb_slots],
        is_limit_test: false,
        limit_test_got_server_busy: false,
        nb_client_target: nb_clients,
        nb_messages_target,
        message_size,
        nb_messages_sent: 0,
        nb_messages_errors: 0,
        nb_messages_received: 0,
        sum_message_delays: 0,
        sum_square_message_delays: 0.0,
        message_delay_min: i64::MAX,
        message_delay_max: 0,
        simulated_time: 0,
    }));

    let simulated_time = 0u64;

    // Default server callback (mode == 2).
    let default_cb = Box::new(CnxStressCallback {
        shared: Rc::clone(&shared),
        mode: 2,
        rank: 0,
        streams: Vec::new(),
    });

    let client_addr = SocketAddr::from(([8u8, 8, 8, 8], 12345u16));
    let server_addr = SocketAddr::from(([1u8, 1, 1, 1], 4433u16));

    let mut qclient = Quic::new(
        nb_slots as u32,
        None,
        None,
        None,
        Some(CNX_STRESS_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(simulated_time),
        None,
        None,
    )?;

    let mut qserver = Quic::new(
        nb_clients as u32,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        None,
        Some(CNX_STRESS_ALPN),
        Some(default_cb),
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(simulated_time),
        None,
        None,
    )?;

    let link_to_clients = Box::new(
        TestSimLink::create(
            1.0,
            10_000,
            None,
            20_000,
            Instant::from_ticks(simulated_time),
        )
        .ok()?,
    );
    let link_to_server = Box::new(
        TestSimLink::create(
            1.0,
            10_000,
            None,
            20_000,
            Instant::from_ticks(simulated_time),
        )
        .ok()?,
    );

    qclient.set_low_memory_mode(true).ok()?;
    qserver.set_low_memory_mode(true).ok()?;
    cnx_stress_set_default_tp(&mut qclient).ok()?;
    cnx_stress_set_default_tp(&mut qserver).ok()?;

    Some(CnxStressCtx {
        shared,
        qserver,
        qclient,
        server_addr,
        client_addr,
        link_to_clients,
        link_to_server,
        client_creation_interval,
        next_client_creation_time: 0,
        client_deletion_interval,
        next_client_deletion_time,
        message_creation_interval,
        next_message_creation_time,
        client_connections: vec![None; nb_slots],
        server_connections: vec![None; nb_slots],
    })
}

/// Run the full stress test for `duration` µs with `nb_clients` connections.
/// C: `cnx_stress_do_test`.
fn cnx_stress_do_test(duration: u64, nb_clients: usize, do_report: bool) -> crate::Result<()> {
    let mut ctx = cnx_stress_create_ctx(duration, nb_clients, false).ok_or(Error::Generic)?;

    let wall_time_start = current_time();

    while ctx.simulated_time() < duration {
        cnx_stress_loop_step(&mut ctx)?;
    }

    let wall_time_elapsed = current_time() - wall_time_start;
    let simulated = ctx.simulated_time();
    let shared = ctx.shared.borrow();

    // If real time exceeded simulated time, the implementation is too slow.
    assert!(
        wall_time_elapsed <= simulated,
        "wall time {wall_time_elapsed} µs exceeded simulated time {simulated} µs",
    );
    assert_eq!(
        shared.nb_clients, shared.nb_client_target,
        "client count mismatch",
    );
    assert_eq!(
        shared.nb_servers, shared.nb_client_target,
        "server count mismatch",
    );
    assert_eq!(
        shared.nb_messages_received, shared.nb_messages_target,
        "messages received mismatch: sent {}, received {}",
        shared.nb_messages_sent, shared.nb_messages_received,
    );

    if do_report {
        let msg_avg_delay = if shared.nb_messages_target > 0 {
            shared.sum_message_delays as f64 / shared.nb_messages_target as f64 / 1_000_000.0
        } else {
            0.0
        };
        println!("Many connection stress (cnx_stress) succeeds:");
        println!(
            "Processed {} connections for {}s (simulated) in {}s (wall time).",
            shared.nb_client_target,
            simulated as f64 / 1_000_000.0,
            wall_time_elapsed as f64 / 1_000_000.0,
        );
        println!(
            "Processed {} messages, delays min/avg/max= {}s, {}s, {}s.",
            shared.nb_messages_target,
            shared.message_delay_min as f64 / 1_000_000.0,
            msg_avg_delay,
            shared.message_delay_max as f64 / 1_000_000.0,
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test entry points.

/// C: `cnx_limit_test` in `picoquictest/cnxstress.c`.
///
/// Verifies that creating a connection when the server is at capacity causes
/// the connection to be rejected with `SERVER_BUSY`.
#[test]
fn cnx_limit() {
    let nb_clients = 4usize;
    let duration = 120_000_000u64;

    let mut ctx = cnx_stress_create_ctx(duration, nb_clients, true).expect("create stress context");

    // Run until all `nb_clients` connections are up on both sides and at least
    // at `ClientAlmostReady`.
    let mut is_done = false;
    while !is_done && ctx.simulated_time() < duration {
        cnx_stress_loop_step(&mut ctx).expect("loop step");

        let (nb_c, nb_s) = {
            let shared = ctx.shared.borrow();
            (shared.nb_clients, shared.nb_servers)
        };
        if nb_c == nb_clients && nb_s == nb_clients {
            // Check that every client slot has a connection at AlmostReady or beyond.
            is_done = true;
            for c in 0..nb_clients {
                let ok = if let Some(Some(cid)) = ctx.client_connections.get(c) {
                    let cid = *cid;
                    if let Some(cnx) = ctx.qclient.connection_ref_by_id(cid) {
                        cnx.state() >= State::ClientAlmostReady
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !ok {
                    is_done = false;
                    break;
                }
            }
        }
    }
    assert!(
        is_done,
        "failed to reach ClientAlmostReady for all connections"
    );

    // Now attempt one extra connection (beyond the server's limit).
    {
        let mut shared = ctx.shared.borrow_mut();
        shared.is_limit_test = true;
        shared.nb_client_target += 1;
    }
    ctx.next_client_creation_time = ctx.simulated_time();

    while ctx.simulated_time() < duration {
        cnx_stress_loop_step(&mut ctx).expect("limit loop step");
        if ctx.shared.borrow().limit_test_got_server_busy {
            break;
        }
    }
    assert!(
        ctx.shared.borrow().limit_test_got_server_busy,
        "server did not send SERVER_BUSY when at connection limit",
    );
}

/// C: `cnx_stress_unit_test` in `picoquictest/cnxstress.c`.
///
/// Stress-tests 100 simultaneous QUIC connections over 120 seconds of
/// simulated time, verifying that all connections establish and all messages
/// are delivered.
#[test]
fn cnx_stress() {
    cnx_stress_do_test(120_000_000, 100, false).expect("cnx_stress_do_test");
}
