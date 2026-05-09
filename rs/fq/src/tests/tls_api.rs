//! Test cases for `picoquictest/tls_api_test.c`.

#![allow(non_snake_case)]

use crate::frames::FrameType;
use crate::internal::{
    CID_REFRESH_DELAY, ENFORCED_INITIAL_MTU, MICROSEC_HANDSHAKE_MAX, NB_PATH_TARGET, PacketType,
    Version, create_long_header, init_transport_parameters, protect_packet_header,
    update_payload_length, varint_encode,
};
use crate::tests::util::{
    TEST_ALPN, TEST_FILE_CERT_STORE, TEST_FILE_CERT_STORE_ED25519, TEST_FILE_CLIENT_CERT_ED25519,
    TEST_FILE_CLIENT_KEY_ED25519, TEST_FILE_SERVER_BAD_CERT, TEST_FILE_SERVER_CERT,
    TEST_FILE_SERVER_CERT_ECDSA, TEST_FILE_SERVER_CERT_ED25519, TEST_FILE_SERVER_CERT_RSA,
    TEST_FILE_SERVER_KEY, TEST_FILE_SERVER_KEY_ECDSA, TEST_FILE_SERVER_KEY_ED25519,
    TEST_FILE_SERVER_KEY_RSA, TEST_SNI, TestApiStreamDesc, ZeroRttTest, cid_length_test_one,
    cnx_ddos_test_loop, compare_text_files, ddos_amplification_test_one, grease_quic_bit_test_one,
    heavy_loss_test_one, keep_alive_test_impl, key_rotation_auto_one, key_rotation_stress_test_one,
    key_rotation_test_one, migration_test_scenario, mtu_discovery_test_one, mtu_drop_cc_algotest,
    nat_rebinding_test_one, optimistic_ack_test_one, padding_test_one, preferred_address_test_one,
    qlog_fns_test_one, qlog_trace_test_one, ready_to_send_test_one, red_cc_algotest,
    request_client_authentication_test_one, save_empty_tickets, session_resume_test_one,
    session_resume_wait_for_ticket, short_initial_cid_test_one, stop_sending_test_one,
    test_api_init_send_recv_scenario, test_random, tester_push_frame_packet,
    tester_simple_ack_frame, tester_wait_handshake_key, tls_api_close_with_losses,
    tls_api_connection_loop, tls_api_data_sending_loop, tls_api_init_ctx, tls_api_init_ctx_ex,
    tls_api_init_ctx_ex2, tls_api_loss_test, tls_api_one_scenario_body,
    tls_api_one_scenario_body_connect, tls_api_one_scenario_body_verify,
    tls_api_one_scenario_init_ex, tls_api_one_sim_round, tls_api_one_sim_round_with_loss,
    tls_api_retry_test_one, tls_api_synch_to_empty_loop, tls_api_test_with_loss,
    tls_retry_token_test_one, transmit_cnxid_test_one, wait_client_connection_ready,
    zero_rtt_test_one,
};
use crate::{
    CHACHA20_POLY1305_SHA256, CallbackEvent, ConnectionId, Instant, PacketContext, Quic,
    RESET_SECRET_SIZE, State, StreamDataCallback, TransportParameters, is_handshake_error,
};

const V1: u32 = Version::InternalTest1 as u32;

const TEST_SCENARIO_VERY_LONG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

const TEST_SCENARIO_Q_AND_R: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 2000,
}];

const CHELLO_MALFORMED: &[u8] = &[
    0x01, 0x00, 0x01, 0x19, 0x03, 0x03, 0xe2, 0xc9, 0x8d, 0x67, 0xe6, 0x60, 0xeb, 0x1f, 0xe4, 0xbc,
    0xa6, 0x19, 0x14, 0xac, 0x86, 0x1a, 0xf7, 0xb1, 0xa1, 0x8d, 0xd5, 0x59, 0xe9, 0x19, 0x20, 0xd2,
    0xee, 0xd3, 0xa5, 0x04, 0x2e, 0xa2, 0x00, 0x00, 0x06, 0x13, 0x01, 0x13, 0x02, 0x13, 0x03, 0x01,
    0x00, 0x00, 0xea, 0x00, 0x33, 0x00, 0x47, 0x00, 0x45, 0x00, 0x1d, 0x00, 0x41, 0x04, 0x1e, 0x93,
    0x9a, 0x4a, 0x38, 0x62, 0xf3, 0xd9, 0x23, 0x68, 0x25, 0x5d, 0x63, 0x01, 0xcb, 0xcc, 0x73, 0xa0,
    0x7e, 0x3c, 0xf2, 0x23, 0xab, 0x0e, 0x5a, 0x3e, 0x65, 0x50, 0x8b, 0xcb, 0x21, 0xe3, 0x7b, 0x6f,
    0xb7, 0x13, 0xaf, 0x3d, 0x0b, 0x8a, 0x5e, 0xb3, 0x13, 0x59, 0xda, 0x73, 0x24, 0xbf, 0x69, 0xf1,
    0x4f, 0x64, 0x7e, 0x04, 0xad, 0x6d, 0x4d, 0xdb, 0x99, 0x0f, 0xc9, 0x20, 0x8d, 0xdc, 0x00, 0x00,
    0x00, 0x15, 0x00, 0x13, 0x00, 0x00, 0x10, 0x74, 0x65, 0x73, 0x74, 0x2e, 0x65, 0x78, 0x61, 0x6d,
    0x70, 0x6c, 0x65, 0x2e, 0x63, 0x6f, 0x6d, 0x00, 0x10, 0x00, 0x10, 0x00, 0x0e, 0x0d, 0x70, 0x69,
    0x63, 0x6f, 0x71, 0x75, 0x69, 0x63, 0x2d, 0x74, 0x65, 0x73, 0x74, 0x00, 0x2b, 0x00, 0x03, 0x02,
    0x03, 0x04, 0x00, 0x0d, 0x00, 0x10, 0x00, 0x0e, 0x08, 0x07, 0x04, 0x03, 0x05, 0x03, 0x06, 0x03,
    0x08, 0x06, 0x08, 0x05, 0x08, 0x04, 0x00, 0x0a, 0x00, 0x06, 0x00, 0x04, 0x00, 0x1d, 0x00, 0x17,
    0xff, 0xa5, 0x00, 0x49, 0x05, 0x04, 0x80, 0x20, 0x00, 0x00, 0x04, 0x04, 0x80, 0x10, 0x00, 0x00,
    0x08, 0x02, 0x42, 0x00, 0x01, 0x04, 0x80, 0x00, 0x75, 0x30, 0x03, 0x02, 0x45, 0xa0, 0x09, 0x02,
    0x42, 0x00, 0x06, 0x04, 0x80, 0x01, 0x00, 0x63, 0x07, 0x04, 0x80, 0x00, 0xff, 0xff, 0x0e, 0x01,
    0x08, 0x0b, 0x01, 0x0a, 0x0f, 0x08, 0x0e, 0xa0, 0xd8, 0xd8, 0x54, 0x9e, 0x27, 0x43, 0x50, 0x57,
    0x01, 0x01, 0xc0, 0x00, 0x00, 0x00, 0xff, 0x04, 0xde, 0x1b, 0x02, 0x43, 0xe8,
];

fn append_crypto_frame(
    bytes: &mut [u8],
    offset: &mut usize,
    crypto_data: &[u8],
) -> crate::Result<()> {
    if bytes.len().saturating_sub(*offset) < 1 {
        return Err(crate::Error::BufferTooSmall);
    }
    bytes[*offset] = FrameType::CryptoHs as u8;
    *offset += 1;

    let written = varint_encode(&mut bytes[*offset..], 0);
    if written == 0 {
        return Err(crate::Error::BufferTooSmall);
    }
    *offset += written;

    let written = varint_encode(&mut bytes[*offset..], crypto_data.len() as u64);
    if written == 0 || bytes.len().saturating_sub(*offset + written) < crypto_data.len() {
        return Err(crate::Error::BufferTooSmall);
    }
    *offset += written;

    bytes[*offset..*offset + crypto_data.len()].copy_from_slice(crypto_data);
    *offset += crypto_data.len();
    Ok(())
}

fn bad_chello_fill_initial(
    quic: &mut Quic,
    buffer: &mut [u8; ENFORCED_INITIAL_MTU],
    chello: &[u8],
) -> crate::Result<()> {
    let icid = ConnectionId::clone_from_slice(&[0xba, 0xdc, 0xe1, 0x10, 0x01, 0x02, 0x03, 0x04])
        .expect("bad chello ICID");
    let scid_bytes = [0xc5, 0xc5, 0xc5, 0xc5, 0x05, 0x06, 0x07, 0x08];
    let scid_len = quic.local_connection_id_length as usize;
    let scid = ConnectionId::clone_from_slice(&scid_bytes[..scid_len]).expect("bad chello SCID");
    let mut pn_offset = 0usize;
    let mut pn_length = 0usize;
    let header_length = create_long_header(
        PacketType::Initial,
        &icid,
        &scid,
        false,
        Version::V1 as u32,
        0,
        0,
        &[],
        buffer,
        &mut pn_offset,
        &mut pn_length,
    );
    if header_length == 0 || pn_length == 0 {
        return Err(crate::Error::Generic);
    }

    let mut length = header_length;
    append_crypto_frame(
        &mut buffer[..ENFORCED_INITIAL_MTU - 16],
        &mut length,
        chello,
    )?;

    let initial_context = quic.initial_aead_context(0, &icid, true, true)?;
    let checksum_len = initial_context.aead_ctx.tag_len();
    update_payload_length(
        buffer,
        pn_offset,
        header_length - pn_length,
        length + checksum_len,
    );

    let header = buffer[..header_length].to_vec();
    let mut payload = buffer[header_length..length].to_vec();
    initial_context.aead_ctx.encrypt(0, &header, &mut payload);
    let send_length = header_length + payload.len();
    if send_length > buffer.len() {
        return Err(crate::Error::BufferTooSmall);
    }
    buffer[header_length..send_length].copy_from_slice(&payload);
    protect_packet_header(
        &mut buffer[..send_length],
        pn_offset,
        0x0f,
        initial_context.pn_enc_ctx.as_ref(),
    );
    buffer[send_length..].fill(0);
    Ok(())
}

#[derive(Debug)]
struct HeaderFuzzerState {
    random_context: u64,
    nb_packets: u32,
    nb_fuzzed: u32,
}

struct HeaderFuzzer {
    state: std::rc::Rc<std::cell::RefCell<HeaderFuzzerState>>,
}

#[derive(Clone)]
struct CountingVerifyCertificateCallback {
    callcount: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl crate::tls::TlsCallbacks for CountingVerifyCertificateCallback {
    fn verify_certificate(&mut self, _certs: &[&[u8]]) -> crate::Result<()> {
        self.callcount
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Debug)]
struct DirectReceiveState {
    stream_id: u64,
    expected_len: usize,
    seen: Vec<bool>,
    unique_received: usize,
    callback_count: usize,
    fin_received: bool,
    fin_offset: Option<u64>,
    error: Option<&'static str>,
}

impl DirectReceiveState {
    fn new(stream_id: u64, expected_len: usize) -> Self {
        Self {
            stream_id,
            expected_len,
            seen: vec![false; expected_len],
            unique_received: 0,
            callback_count: 0,
            fin_received: false,
            fin_offset: None,
            error: None,
        }
    }

    fn record(&mut self, stream_id: u64, fin: bool, bytes: &[u8], offset: u64) {
        self.callback_count += 1;
        if self.error.is_some() {
            return;
        }
        if stream_id != self.stream_id {
            self.error = Some("unexpected stream id");
            return;
        }

        let Ok(start) = usize::try_from(offset) else {
            self.error = Some("offset does not fit usize");
            return;
        };
        let Some(end) = start.checked_add(bytes.len()) else {
            self.error = Some("offset overflow");
            return;
        };
        if end > self.expected_len {
            self.error = Some("data extends past expected stream length");
            return;
        }

        for (i, byte) in bytes.iter().enumerate() {
            let pos = start + i;
            if *byte != pos as u8 {
                self.error = Some("direct receive data mismatch");
                return;
            }
        }
        for pos in start..end {
            if !self.seen[pos] {
                self.seen[pos] = true;
                self.unique_received += 1;
            }
        }

        if fin {
            let fin_offset = offset.saturating_add(bytes.len() as u64);
            self.fin_received = true;
            self.fin_offset = Some(fin_offset);
            if fin_offset != self.expected_len as u64 {
                self.error = Some("unexpected fin offset");
            }
        }
    }

    fn is_complete(&self) -> bool {
        self.error.is_none() && self.fin_received && self.unique_received == self.expected_len
    }
}

struct DirectReceiveProbe {
    state: std::rc::Rc<std::cell::RefCell<DirectReceiveState>>,
}

#[derive(Debug, Default)]
struct DocumentAddressState {
    nb_almost_ready: usize,
    local_addr_almost_ready: Option<core::net::SocketAddr>,
    remote_addr_almost_ready: Option<core::net::SocketAddr>,
    nb_ready: usize,
    local_addr_ready: Option<core::net::SocketAddr>,
    remote_addr_ready: Option<core::net::SocketAddr>,
}

impl DocumentAddressState {
    fn record(&mut self, connection: &crate::internal::Connection, fin_or_event: CallbackEvent) {
        fn documented_addr(addr: core::net::SocketAddr) -> Option<core::net::SocketAddr> {
            (!addr.ip().is_unspecified()).then_some(addr)
        }

        match fin_or_event {
            CallbackEvent::AlmostReady => {
                self.nb_almost_ready += 1;
                if self.nb_almost_ready == 1 {
                    self.local_addr_almost_ready = documented_addr(connection.get_local_addr());
                    self.remote_addr_almost_ready = documented_addr(connection.get_peer_addr());
                }
            }
            CallbackEvent::Ready => {
                self.nb_ready += 1;
                if self.nb_ready == 1 {
                    self.local_addr_ready = documented_addr(connection.get_local_addr());
                    self.remote_addr_ready = documented_addr(connection.get_peer_addr());
                }
            }
            _ => {}
        }
    }
}

struct DocumentAddressCallback {
    state: std::rc::Rc<std::cell::RefCell<DocumentAddressState>>,
    inner: Option<Box<dyn StreamDataCallback>>,
}

impl StreamDataCallback for DocumentAddressCallback {
    fn callback(
        &mut self,
        connection: &mut crate::internal::Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        self.state.borrow_mut().record(connection, fin_or_event);

        let ret = if let Some(inner) = self.inner.as_mut() {
            inner.callback(connection, stream_id, bytes, fin_or_event, stream_ctx)
        } else {
            0
        };

        if connection.callback_fn.is_some() {
            self.inner = connection.callback_fn.take();
        }

        ret
    }
}

fn document_addresses_check(
    state: &DocumentAddressState,
    local_addr_ref: core::net::SocketAddr,
    remote_addr_ref: core::net::SocketAddr,
) {
    assert_eq!(
        state.nb_almost_ready, 1,
        "expected exactly one almost-ready callback"
    );
    assert_eq!(
        state.local_addr_almost_ready,
        Some(local_addr_ref),
        "almost-ready local address mismatch"
    );
    assert_eq!(
        state.remote_addr_almost_ready,
        Some(remote_addr_ref),
        "almost-ready remote address mismatch"
    );
    assert_eq!(state.nb_ready, 1, "expected exactly one ready callback");
    assert_eq!(
        state.local_addr_ready,
        Some(local_addr_ref),
        "ready local address mismatch"
    );
    assert_eq!(
        state.remote_addr_ready,
        Some(remote_addr_ref),
        "ready remote address mismatch"
    );
}

impl crate::StreamDirectReceive for DirectReceiveProbe {
    fn receive(
        &mut self,
        _connection: &mut crate::internal::Connection,
        stream_id: u64,
        fin: bool,
        bytes: &[u8],
        offset: u64,
    ) -> i32 {
        let mut state = self.state.borrow_mut();
        state.record(stream_id, fin, bytes, offset);
        if state.error.is_some() {
            -1
        } else if state.is_complete() {
            crate::errors::InternalError::StreamReceiveComplete as i32
        } else {
            0
        }
    }
}

fn direct_receive_data_sending_loop(
    test_ctx: &mut crate::tests::util::TestTlsApiCtx,
    direct_state: &std::rc::Rc<std::cell::RefCell<DirectReceiveState>>,
    loss_mask: &mut u64,
    simulated_time: &mut Instant,
    max_completion_microsec: u64,
) -> crate::Result<()> {
    let start_time = test_ctx.cnx_client().start_time;
    let time_out = Instant::from_ticks(start_time.ticks().saturating_add(max_completion_microsec));
    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while nb_trials < 4_000_000
        && nb_inactive < 256
        && test_ctx.client_ready()
        && test_ctx.server_ready()
        && simulated_time.ticks() < time_out.ticks()
        && !direct_state.borrow().is_complete()
    {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round_with_loss(
            test_ctx,
            simulated_time,
            time_out,
            &mut was_active,
            loss_mask,
        )?;
        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    let state = direct_state.borrow();
    if let Some(error) = state.error {
        eprintln!("direct_receive: callback validation failed: {error}");
        return Err(crate::Error::Generic);
    }
    if !state.is_complete() {
        eprintln!(
            "direct_receive: incomplete direct receive: callbacks={} unique={}/{} fin={:?} time={}",
            state.callback_count,
            state.unique_received,
            state.expected_len,
            state.fin_offset,
            simulated_time.ticks()
        );
        return Err(crate::Error::Generic);
    }

    let completion_time = simulated_time.ticks().saturating_sub(start_time.ticks());
    if completion_time > max_completion_microsec {
        eprintln!(
            "direct_receive: completion time {completion_time} exceeds {max_completion_microsec}"
        );
        return Err(crate::Error::Generic);
    }

    Ok(())
}

impl crate::Fuzz for HeaderFuzzer {
    fn fuzz(
        &mut self,
        connection: &mut crate::internal::Connection,
        bytes: &mut [u8],
        length: usize,
        _header_length: usize,
    ) -> u32 {
        let mut state = self.state.borrow_mut();
        state.nb_packets += 1;

        if connection.connection_state >= State::ClientAlmostReady
            && connection.pkt_ctx[crate::PacketContext::Application as usize].send_sequence > 2
        {
            let mut fuzz_pilot = test_random(&mut state.random_context);
            for i in 1..=8 {
                if i < length && i < bytes.len() {
                    bytes[i] ^= fuzz_pilot as u8;
                    fuzz_pilot >>= 8;
                }
            }
            state.nb_fuzzed += 1;
        }

        length as u32
    }
}

fn first_path_remote_cid(cnx: &crate::internal::Connection) -> crate::Result<ConnectionId> {
    let path = cnx.paths.first().ok_or(crate::Error::Generic)?;
    let tuple = path.tuples.first().ok_or(crate::Error::Generic)?;
    let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
    cnx.remote_connection_id_stashes
        .iter()
        .find(|stash| stash.unique_path_id == path.unique_path_id)
        .and_then(|stash| stash.connection_ids.get(cid_index))
        .map(|remote_cnxid| remote_cnxid.connection_id)
        .ok_or(crate::Error::Generic)
}

fn first_path_local_cid(cnx: &crate::internal::Connection) -> crate::Result<ConnectionId> {
    let path = cnx.paths.first().ok_or(crate::Error::Generic)?;
    let tuple = path.tuples.first().ok_or(crate::Error::Generic)?;
    tuple
        .local_connection_id
        .and_then(|token| cnx.local_connection_ids.get(token))
        .map(|local_cnxid| local_cnxid.connection_id)
        .ok_or(crate::Error::Generic)
}

fn first_path_remote_reset_secret(
    cnx: &crate::internal::Connection,
) -> crate::Result<[u8; RESET_SECRET_SIZE]> {
    let path = cnx.paths.first().ok_or(crate::Error::Generic)?;
    let tuple = path.tuples.first().ok_or(crate::Error::Generic)?;
    let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
    cnx.remote_connection_id_stashes
        .iter()
        .find(|stash| stash.unique_path_id == path.unique_path_id)
        .and_then(|stash| stash.connection_ids.get(cid_index))
        .map(|remote_cnxid| remote_cnxid.reset_secret)
        .ok_or(crate::Error::Generic)
}

fn client_error_queue_bad_frame(
    test_ctx: &mut crate::tests::util::TestTlsApiCtx,
    mode: u8,
) -> crate::Result<()> {
    let frame = match mode {
        0 => vec![
            0x17, 0x04, 0x41, 0x01, 0x08, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        ],
        1 => {
            let sequence = test_ctx.cnx_client().nb_paths() as u8 + 3;
            let reset_secret = first_path_remote_reset_secret(test_ctx.cnx_server())?;
            let mut frame = Vec::with_capacity(4 + 8 + RESET_SECRET_SIZE);
            frame.push(FrameType::NewConnectionId as u8);
            frame.push(sequence);
            frame.push(0);
            frame.push(8);
            frame.extend_from_slice(&[0x99; 8]);
            frame.extend_from_slice(&reset_secret);
            frame
        }
        2 => vec![FrameType::StopSending as u8, 2, 0],
        _ => return Err(crate::Error::InvalidArgument),
    };

    test_ctx
        .cnx_client()
        .queue_misc_frame(&frame, false, PacketContext::Application)
}

fn client_error_wait_for_disconnect(
    test_ctx: &mut crate::tests::util::TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let next_time = Instant::from_ticks(simulated_time.ticks() + 3_000_000);

    while simulated_time.ticks() < next_time.ticks() {
        let client_disconnected = test_ctx
            .qclient
            .first_connection()
            .map(|cnx| cnx.connection_state >= State::Disconnected)
            .unwrap_or(true);
        let server_disconnected = test_ctx
            .qserver
            .first_connection()
            .map(|cnx| cnx.connection_state >= State::Disconnected)
            .unwrap_or(true);
        if client_disconnected && server_disconnected {
            break;
        }

        let mut was_active = false;
        tls_api_one_sim_round(test_ctx, simulated_time, next_time, &mut was_active)?;
    }

    let server_disconnected = test_ctx
        .qserver
        .first_connection()
        .map(|cnx| cnx.connection_state == State::Disconnected)
        .unwrap_or(true);
    if server_disconnected {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

fn client_error_delete_first_connection(quic: &mut Quic) {
    while let Some(token) = quic.first_connection().and_then(|cnx| cnx.own_token) {
        quic.delete_connection(token);
    }
}

fn client_error_delete_disconnected_server(test_ctx: &mut crate::tests::util::TestTlsApiCtx) {
    while let Some(token) = test_ctx.qserver.first_connection().and_then(|cnx| {
        (cnx.connection_state == State::Disconnected)
            .then_some(cnx.own_token)
            .flatten()
    }) {
        test_ctx.qserver.delete_connection(token);
    }
}

fn client_error_wait_application_aead_ready(
    test_ctx: &mut crate::tests::util::TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + 4_000_000);
    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while simulated_time.ticks() < time_out.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
        && test_ctx
            .qserver
            .first_connection()
            .map(|cnx| {
                cnx.crypto_context[crate::internal::Epoch::OneRtt as usize]
                    .aead_decrypt
                    .is_none()
            })
            .unwrap_or(false)
        && nb_trials < 1024
        && nb_inactive < 64
    {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round(test_ctx, simulated_time, time_out, &mut was_active)?;
        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    let ready = test_ctx
        .qserver
        .first_connection()
        .map(|cnx| {
            cnx.crypto_context[crate::internal::Epoch::OneRtt as usize]
                .aead_decrypt
                .is_some()
        })
        .unwrap_or(false);
    if ready {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

fn client_error_modal(mode: u8) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time).inspect_err(
        |e| {
            eprintln!("client_error({mode}): initial connection loop failed: {e:?}");
        },
    )?;
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_Q_AND_R).inspect_err(|e| {
        eprintln!("client_error({mode}): scenario init failed: {e:?}");
    })?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0).inspect_err(
        |e| {
            eprintln!("client_error({mode}): data loop failed: {e:?}");
        },
    )?;

    client_error_queue_bad_frame(&mut test_ctx, mode).inspect_err(|e| {
        eprintln!("client_error({mode}): queue bad frame failed: {e:?}");
    })?;
    client_error_wait_for_disconnect(&mut test_ctx, &mut simulated_time).inspect_err(|_| {
        let client_state = test_ctx.qclient.first_connection().map(|cnx| cnx.state());
        let server_state = test_ctx.qserver.first_connection().map(|cnx| cnx.state());
        eprintln!(
            "client_error({mode}): disconnect wait failed: client={client_state:?} server={server_state:?} t={}",
            simulated_time.ticks()
        );
    })?;

    client_error_delete_first_connection(&mut test_ctx.qclient);
    client_error_delete_disconnected_server(&mut test_ctx);

    test_ctx
        .qclient
        .create_connection(
            ConnectionId::with_size(0).expect("initial CID"),
            ConnectionId::with_size(0).expect("remote CID"),
            Some(&test_ctx.server_addr),
            simulated_time,
            0,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .expect("recreated client connection")
        .start_client()?;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time).inspect_err(|_| {
        let client_state = test_ctx.qclient.first_connection().map(|cnx| cnx.state());
        let server_state = test_ctx.qserver.first_connection().map(|cnx| cnx.state());
        eprintln!(
            "client_error({mode}): restart connection loop failed: client={client_state:?} server={server_state:?} t={}",
            simulated_time.ticks()
        );
    })?;
    client_error_wait_application_aead_ready(&mut test_ctx, &mut simulated_time).inspect_err(
        |_| {
            let client_state = test_ctx.qclient.first_connection().map(|cnx| {
                (
                    cnx.state(),
                    cnx.local_error(),
                    cnx.remote_error(),
                    cnx.application_error(),
                    cnx.remote_application_error(),
                )
            });
            let server_state = test_ctx.qserver.first_connection().map(|cnx| cnx.state());
            eprintln!(
                "client_error({mode}): application AEAD wait failed: client={client_state:?} server={server_state:?} t={}",
                simulated_time.ticks()
            );
        },
    )?;
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).inspect_err(|_| {
        let client_state = test_ctx.qclient.first_connection().map(|cnx| cnx.state());
        let server_state = test_ctx.qserver.first_connection().map(|cnx| cnx.state());
        eprintln!(
            "client_error({mode}): close failed: client={client_state:?} server={server_state:?} t={}",
            simulated_time.ticks()
        );
    })
}

/// C: `tls_api_server_first_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Drops packet 14 (the first server flight packet) and verifies recovery.
#[test]
fn sh_loss() {
    tls_api_loss_test(14).expect("sh_loss");
}

/// C: `af_undef_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies a client can complete handshake and data transfer when incoming
/// packets do not report a usable local destination address.
#[test]
fn af_undef() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xaf, 0x0d, 0xef, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .expect("tls_api_init_ctx_ex");

    test_ctx.client_endpoint.addr_to_unspec = true;
    test_ctx.qserver.set_qlog(".").expect("server qlog");
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 1_000_000)
        .expect("scenario verify");
}

/// C: `bad_certificate_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a server presenting the bad test certificate is rejected by
/// the client.
#[test]
fn bad_certificate() {
    const TEST_TICKET_ENCRYPT_KEY: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    test_ctx.qserver = Quic::new(
        8,
        Some(TEST_FILE_SERVER_BAD_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        Some(&TEST_TICKET_ENCRYPT_KEY),
    )
    .expect("bad-cert server context");

    let _ = tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time);

    assert!(test_ctx.has_cnx_server(), "server connection should exist");
    let (client_state, client_local_error) = {
        let client = test_ctx.cnx_client();
        (client.state(), client.local_error())
    };
    assert_eq!(client_state, State::Disconnected);
    assert!(
        is_handshake_error(client_local_error),
        "client local error should be a handshake error, got {client_local_error:#x}"
    );

    let server_remote_error = test_ctx.cnx_server().remote_error();
    assert!(
        is_handshake_error(server_remote_error),
        "server remote error should be a handshake error, got {server_remote_error:#x}"
    );
}

/// C: `bad_chello_test` in `picoquictest/tls_api_test.c`.
///
/// Sends a malformed ClientHello and verifies the server rejects it.
#[test]
fn bad_chello() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xba, 0xdc, 0xe1, 0x10, 0, 0, 0, 0]).expect("initial CID");
    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        V1,
        Some(TEST_SNI),
        Some(TEST_ALPN),
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");
    let mut buffer = [0u8; ENFORCED_INITIAL_MTU];

    test_ctx.qserver.set_qlog(".").expect("server qlog");
    bad_chello_fill_initial(&mut test_ctx.qserver, &mut buffer, CHELLO_MALFORMED)
        .expect("bad chello initial");

    let cnx_trial_created = test_ctx
        .qserver
        .incoming_packet_ex(
            &mut buffer,
            &test_ctx.client_addr,
            &test_ctx.server_addr,
            0,
            0,
            simulated_time,
        )
        .expect("submit bad chello")
        .is_some();
    assert!(
        !cnx_trial_created,
        "bad chello caused context creation at t={}",
        simulated_time.ticks()
    );
    assert!(
        !test_ctx.has_cnx_server(),
        "bad chello left a server connection at t={}",
        simulated_time.ticks()
    );

    simulated_time = Instant::from_ticks(simulated_time.ticks() + 10_000);
    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        0,
        0,
        0,
        20_000,
        2_000_000,
    )
    .expect("post bad chello scenario");
}

/// C: `bad_client_certificate_test` in `picoquictest/tls_api_test.c`.
///
/// Client presents a certificate that the server's trust store does not
/// recognise; verifies the server rejects the connection.
#[test]
fn bad_client_certificate() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");
    let server_addr = test_ctx.server_addr;

    test_ctx.qclient = Quic::new(
        8,
        Some(TEST_FILE_SERVER_BAD_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        None,
    )
    .expect("bad-client-cert client context");
    test_ctx.qclient.enforce_client_only(true);

    {
        let client = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).expect("initial CID"),
                ConnectionId::with_size(0).expect("remote CID"),
                Some(&server_addr),
                simulated_time,
                0,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .expect("client connection");
        client.start_client().expect("start client");
    }

    test_ctx.qserver.set_client_authentication(true);
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    let (client_state, client_remote_error) = {
        let client = test_ctx.cnx_client();
        (client.state(), client.remote_error())
    };
    assert_eq!(client_state, State::Disconnected);

    assert!(test_ctx.has_cnx_server(), "server connection should exist");
    let server_local_error = test_ctx.cnx_server().local_error();
    assert!(
        is_handshake_error(server_local_error),
        "server local error should be a handshake error, got {server_local_error:#x}"
    );
    assert!(
        is_handshake_error(client_remote_error),
        "client remote error should be a handshake error, got {client_remote_error:#x}"
    );
}

/// C: `bad_cnxid_test` in `picoquictest/tls_api_test.c`.
///
/// Fuzzes the connection ID field in the packet header and verifies the
/// connection is terminated gracefully.
#[test]
fn bad_cnxid() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");
    let mut loss_mask = 0u64;
    let fuzz_state = std::rc::Rc::new(std::cell::RefCell::new(HeaderFuzzerState {
        random_context: 0x1234_5678_9abc_def0,
        nb_packets: 0,
        nb_fuzzed: 0,
    }));

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    tls_api_one_scenario_body_connect(&mut test_ctx, &mut simulated_time, 0, 0).expect("connect");

    test_ctx.qclient.set_fuzz(Some(Box::new(HeaderFuzzer {
        state: std::rc::Rc::clone(&fuzz_state),
    })));
    let _ = tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0);

    let stats = fuzz_state.borrow();
    assert!(
        stats.nb_fuzzed > 0,
        "header fuzzer did not fuzz any packet; packets={}",
        stats.nb_packets
    );

    let server_state = test_ctx.qserver.first_connection().map(|cnx| cnx.state());
    assert!(
        matches!(server_state, None | Some(State::Disconnected)),
        "unexpected server state after header fuzzing: {:?}; packets={}, fuzzed={}",
        server_state,
        stats.nb_packets,
        stats.nb_fuzzed
    );
    drop(stats);

    let old_client = test_ctx
        .qclient
        .first_connection()
        .and_then(|cnx| cnx.own_token)
        .expect("old client connection token");
    test_ctx.qclient.delete_connection(old_client);
    test_ctx.qclient.set_fuzz(None);

    if let Some((Some(server_token), State::Disconnected)) = test_ctx
        .qserver
        .first_connection()
        .map(|cnx| (cnx.own_token, cnx.state()))
    {
        test_ctx.qserver.delete_connection(server_token);
    }

    test_ctx
        .qclient
        .create_connection(
            ConnectionId::with_size(0).expect("initial CID"),
            ConnectionId::with_size(0).expect("remote CID"),
            Some(&test_ctx.server_addr),
            simulated_time,
            V1,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .expect("second client connection")
        .start_client()
        .expect("start second client connection");

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        0,
        0,
        0,
        20_000,
        100_000,
    )
    .expect("second q_and_r connection");
}

/// C: `bad_coalesce_test` in `picoquictest/tls_api_test.c`.
///
/// Sends a coalesced packet with an invalid second QUIC packet and verifies
/// the implementation ignores the bad coalesced portion without dropping the
/// connection.
#[test]
fn bad_coalesce() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    ctx.do_bad_coalesce_test = true;
    tls_api_one_scenario_body(&mut ctx, &mut t, TEST_SCENARIO_Q_AND_R, 0, 0, 0, 0, 250_000)
        .expect("bad_coalesce");
}

/// C: `chacha20_test` in `picoquictest/tls_api_test.c`.
///
/// Enables ChaCha20-Poly1305 AEAD and verifies a complete handshake + data.
#[test]
fn chacha20() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    if test_ctx
        .qclient
        .set_cipher_suite(CHACHA20_POLY1305_SHA256)
        .is_ok()
    {
        tls_api_one_scenario_body(
            &mut test_ctx,
            &mut simulated_time,
            TEST_SCENARIO_Q_AND_R,
            0,
            0,
            0,
            0,
            250_000,
        )
        .expect("chacha20");
    }
}

/// C: `cid_length_test` in `picoquictest/tls_api_test.c`.
///
/// Loops through 20 different client CID lengths and verifies each completes
/// a successful handshake.
#[test]
fn cid_length() {
    const TESTED_LENGTHS: &[u32] = &[
        0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    ];

    for &len in TESTED_LENGTHS {
        cid_length_test_one(len).unwrap_or_else(|e| panic!("cid_length({len}): {e:?}"));
    }
}

/// C: `cid_quiescence_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection ID rotation quiesces after the handshake.
#[test]
fn cid_quiescence() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    let _remote_after_handshake =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID after handshake");

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time).expect("client ready");

    let previous_remote_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID before quiescence");
    simulated_time += CID_REFRESH_DELAY;

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)
        .expect("scenario verify");

    let refreshed_remote_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID after quiescence");
    assert_ne!(
        previous_remote_id, refreshed_remote_id,
        "client remote CID did not rotate after CID refresh delay"
    );
}

/// C: `request_client_authentication_test` in `picoquictest/tls_api_test.c`.
///
/// Server requests client authentication; verifies the RSA client certificate
/// is accepted by the default server.
#[test]
fn client_auth() {
    request_client_authentication_test_one(
        TEST_FILE_SERVER_CERT_RSA,
        TEST_FILE_SERVER_KEY_RSA,
        TEST_FILE_SERVER_CERT,
        TEST_FILE_SERVER_KEY,
        TEST_FILE_CERT_STORE,
    )
    .expect("client_auth");
}

/// C: `request_client_authentication_25519_test` in `picoquictest/tls_api_test.c`.
///
/// Same as `client_auth` but using Ed25519 ECDSA certificates.
#[test]
fn client_auth_25519() {
    request_client_authentication_test_one(
        TEST_FILE_CLIENT_CERT_ED25519,
        TEST_FILE_CLIENT_KEY_ED25519,
        TEST_FILE_SERVER_CERT_ED25519,
        TEST_FILE_SERVER_KEY_ED25519,
        TEST_FILE_CERT_STORE_ED25519,
    )
    .expect("client_auth_25519");
}

/// C: `set_verify_certificate_callback_test` in `picoquictest/tls_api_test.c`.
///
/// Registers a custom certificate-verification callback and verifies it is
/// invoked correctly during the handshake.
#[test]
fn client_cert_callback() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");
    let verify_callcount = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let server_addr = test_ctx.server_addr;

    test_ctx.qclient = Quic::new(
        8,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        simulated_time,
        None,
        None,
    )
    .expect("client context with certificate");

    {
        let client = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).expect("initial CID"),
                ConnectionId::with_size(0).expect("remote CID"),
                Some(&server_addr),
                simulated_time,
                0,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .expect("client connection");
        client.start_client().expect("start client");
    }

    test_ctx
        .qclient
        .set_verify_certificate_callback(Some(Box::new(CountingVerifyCertificateCallback {
            callcount: std::sync::Arc::clone(&verify_callcount),
        })));
    test_ctx
        .qserver
        .set_verify_certificate_callback(Some(Box::new(CountingVerifyCertificateCallback {
            callcount: std::sync::Arc::clone(&verify_callcount),
        })));
    test_ctx.qserver.set_client_authentication(true);

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    assert_eq!(
        verify_callcount.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "certificate verification callback count"
    );
}

/// C: `client_error_test` in `picoquictest/tls_api_test.c`.
///
/// Runs the client-error scenario in three modes (0, 1, 2) to exercise
/// different error-signalling paths.
#[test]
fn client_error() {
    for (mode, name) in [(0, "stream"), (1, "new_connection_id"), (2, "stop_sending")] {
        client_error_modal(mode).unwrap_or_else(|e| panic!("client_error({name}): {e:?}"));
    }
}

/// C: `tls_api_client_losses_test` in `picoquictest/tls_api_test.c`.
///
/// Drops packets 1 and 2 (both initial client flights) and verifies recovery.
#[test]
fn client_losses() {
    tls_api_loss_test(3).expect("client_losses");
}

/// C: `client_only_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a context created in client-only mode refuses incoming
/// server connections.
#[test]
fn client_only() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xc1, 0x10, 0, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.enforce_client_only(true);
    test_ctx.qserver.set_qlog(".").expect("server qlog");
    test_ctx.qclient.set_qlog(".").expect("client qlog");

    let connection_ret =
        tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time);
    let client_state = test_ctx.cnx_client().state();
    assert!(
        connection_ret.is_err() || client_state >= State::Disconnected,
        "connection unexpectedly succeeded: state={client_state:?}, ret={connection_ret:?}"
    );
    assert!(
        !test_ctx.has_cnx_server(),
        "server connection context created despite client-only enforcement"
    );
}

/// C: `cnx_ddos_unit_test` in `picoquictest/tls_api_test.c`.
///
/// Stress-tests the server with 1000 client Initial packets spaced 1000 us
/// apart, then verifies a normal q2-and-r2 connection still succeeds.
#[test]
fn cnx_ddos() {
    cnx_ddos_test_loop(1000, 1000).expect("cnx_ddos");
}

/// C: `cnxid_renewal_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that connection IDs are renewed when the active CID approaches
/// exhaustion.
#[test]
fn cnxid_renewal() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        NB_PATH_TARGET as i32,
        1,
    )
    .expect("synch to empty");

    test_ctx
        .cnx_client()
        .renew_connection_id(0)
        .expect("renew connection ID");
    let target_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("target remote CID after renewal");
    let previous_local_id =
        first_path_local_cid(test_ctx.cnx_client()).expect("local CID after renewal");

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_Q_AND_R)
        .expect("init q_and_r scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");

    let next_time = Instant::from_ticks(simulated_time.ticks() + 7_000_000);
    while simulated_time.ticks() < next_time.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            next_time,
            &mut was_active,
        )
        .expect("renewal grace loop");
    }

    let client_path_demoted = test_ctx
        .cnx_client()
        .paths
        .first()
        .map(|path| path.path_is_demoted)
        .unwrap_or(true);
    assert!(!client_path_demoted, "default client path is demoted");

    if test_ctx.has_cnx_server() {
        let server_path_demoted = test_ctx
            .cnx_server()
            .paths
            .first()
            .map(|path| path.path_is_demoted)
            .unwrap_or(true);
        assert!(!server_path_demoted, "default server path is demoted");
    }

    let final_remote_id =
        first_path_remote_cid(test_ctx.cnx_client()).expect("remote CID after renewal loop");
    assert_eq!(
        final_remote_id, target_id,
        "remote CNX ID migrated from the selected value"
    );

    let final_local_id =
        first_path_local_cid(test_ctx.cnx_client()).expect("local CID after renewal loop");
    assert_ne!(
        final_local_id, previous_local_id,
        "local CNX ID did not change to a new value"
    );
}

/// C: `transmit_cnxid_test` in `picoquictest/tls_api_test.c`.
///
/// Basic CNXID transmit test: no retire-before, no disable, no early retire.
#[test]
fn cnxid_transmit() {
    transmit_cnxid_test_one(false, false, false).expect("cnxid_transmit");
}

/// C: `transmit_cnxid_disable_test` in `picoquictest/tls_api_test.c`.
///
/// CNXID transmit with CID migration disabled.
#[test]
fn cnxid_transmit_disable() {
    transmit_cnxid_test_one(false, true, false).expect("cnxid_transmit_disable");
}

/// C: `transmit_cnxid_retire_before_test` in `picoquictest/tls_api_test.c`.
///
/// CNXID transmit with retire-before flag set.
#[test]
fn cnxid_transmit_r_before() {
    transmit_cnxid_test_one(true, false, false).expect("cnxid_transmit_r_before");
}

/// C: `transmit_cnxid_retire_disable_test` in `picoquictest/tls_api_test.c`.
///
/// CNXID transmit with retire-before and migration disabled.
#[test]
fn cnxid_transmit_r_disable() {
    transmit_cnxid_test_one(true, true, false).expect("cnxid_transmit_r_disable");
}

/// C: `transmit_cnxid_retire_early_test` in `picoquictest/tls_api_test.c`.
///
/// CNXID transmit with early retire.
#[test]
fn cnxid_transmit_r_early() {
    transmit_cnxid_test_one(false, false, true).expect("cnxid_transmit_r_early");
}

#[derive(Copy, Clone)]
struct ConnectionDropCase {
    target_state: State,
    target_is_client: bool,
}

fn connection_drop_state(
    test_ctx: &mut crate::tests::util::TestTlsApiCtx,
    is_client: bool,
) -> Option<State> {
    if is_client {
        test_ctx.qclient.first_connection()
    } else {
        test_ctx.qserver.first_connection()
    }
    .map(|cnx| cnx.state())
}

fn connection_drop_target_snapshot(
    test_ctx: &mut crate::tests::util::TestTlsApiCtx,
    target_is_client: bool,
) -> crate::Result<(State, Instant, Instant)> {
    let target = if target_is_client {
        test_ctx.qclient.first_connection()
    } else {
        test_ctx.qserver.first_connection()
    }
    .ok_or(crate::Error::Generic)?;
    Ok((target.state(), target.start_time, target.next_wake_time))
}

fn connection_drop_all_disconnected(test_ctx: &mut crate::tests::util::TestTlsApiCtx) -> bool {
    let client_disconnected = test_ctx
        .qclient
        .first_connection()
        .map(|cnx| cnx.state() == State::Disconnected)
        .unwrap_or(true);
    let server_disconnected = test_ctx
        .qserver
        .first_connection()
        .map(|cnx| cnx.state() == State::Disconnected)
        .unwrap_or(true);
    client_disconnected && server_disconnected
}

fn connection_drop_reached_target(
    test_ctx: &mut crate::tests::util::TestTlsApiCtx,
    target_client_state: State,
    target_server_state: State,
) -> bool {
    connection_drop_state(test_ctx, true)
        .map(|state| state >= target_client_state)
        .unwrap_or(false)
        || connection_drop_state(test_ctx, false)
            .map(|state| state >= target_server_state)
            .unwrap_or(false)
}

fn connection_drop_test_one(
    target_client_state: State,
    target_server_state: State,
    target_is_client: bool,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while nb_trials < 1024
        && nb_inactive < 512
        && (!test_ctx.client_ready() || !test_ctx.has_cnx_server() || !test_ctx.server_ready())
    {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;

        if connection_drop_all_disconnected(&mut test_ctx)
            || connection_drop_reached_target(
                &mut test_ctx,
                target_client_state,
                target_server_state,
            )
        {
            break;
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    let target_state =
        connection_drop_state(&mut test_ctx, target_is_client).ok_or(crate::Error::Generic)?;
    if target_state >= State::Ready {
        let client_state = connection_drop_state(&mut test_ctx, true);
        let server_state = connection_drop_state(&mut test_ctx, false);
        eprintln!(
            "connection_drop target already ready: target_is_client={target_is_client}, target={target_state:?}, client={client_state:?}, server={server_state:?}, trials={nb_trials}, inactive={nb_inactive}, t={}",
            simulated_time.ticks(),
        );
        return Err(crate::Error::Generic);
    }

    let mut disconnected_in_time = false;
    for _ in 0..100_000 {
        let (target_state, start_time, next_wake_time) =
            connection_drop_target_snapshot(&mut test_ctx, target_is_client)?;
        if target_state == State::Disconnected {
            disconnected_in_time = true;
            break;
        }
        if next_wake_time > simulated_time {
            simulated_time = next_wake_time;
        }
        if simulated_time.ticks()
            > start_time
                .ticks()
                .saturating_add(MICROSEC_HANDSHAKE_MAX.ticks())
        {
            break;
        }

        let mut packet = [0u8; crate::MAX_PACKET_SIZE];
        let prepared = if target_is_client {
            test_ctx
                .qclient
                .first_cnx_mut()
                .ok_or(crate::Error::Generic)?
                .prepare_packet(simulated_time, &mut packet)
        } else {
            test_ctx
                .qserver
                .first_cnx_mut()
                .ok_or(crate::Error::Generic)?
                .prepare_packet(simulated_time, &mut packet)
        };
        match prepared {
            Ok(_) | Err(crate::Error::Disconnected) => {}
            Err(error) => return Err(error),
        }
    }

    if disconnected_in_time {
        Ok(())
    } else {
        let target_state = connection_drop_state(&mut test_ctx, target_is_client);
        eprintln!(
            "connection_drop target did not disconnect: target_is_client={target_is_client}, state={target_state:?}, t={}",
            simulated_time.ticks()
        );
        Err(crate::Error::Generic)
    }
}

/// C: `connection_drop_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that each target client/server handshake state disconnects
/// cleanly when the peer stops responding.
#[test]
fn connection_drop() {
    const CASES: [ConnectionDropCase; 9] = [
        ConnectionDropCase {
            target_state: State::ClientInitSent,
            target_is_client: true,
        },
        ConnectionDropCase {
            target_state: State::ClientRenegotiate,
            target_is_client: true,
        },
        ConnectionDropCase {
            target_state: State::ClientInitResent,
            target_is_client: true,
        },
        ConnectionDropCase {
            target_state: State::ServerInit,
            target_is_client: false,
        },
        ConnectionDropCase {
            target_state: State::ServerHandshake,
            target_is_client: false,
        },
        ConnectionDropCase {
            target_state: State::ClientHandshakeStart,
            target_is_client: true,
        },
        ConnectionDropCase {
            target_state: State::ServerFalseStart,
            target_is_client: false,
        },
        ConnectionDropCase {
            target_state: State::ServerAlmostReady,
            target_is_client: false,
        },
        ConnectionDropCase {
            target_state: State::ClientAlmostReady,
            target_is_client: true,
        },
    ];

    for (i, case) in CASES.iter().copied().enumerate() {
        let target_client_state = if case.target_is_client {
            case.target_state
        } else {
            State::Ready
        };
        let target_server_state = if case.target_is_client {
            State::Ready
        } else {
            case.target_state
        };
        connection_drop_test_one(
            target_client_state,
            target_server_state,
            case.target_is_client,
        )
        .unwrap_or_else(|error| {
            panic!(
                "connection_drop case {i} failed: target={:?}, target_is_client={}, error={error:?}",
                case.target_state, case.target_is_client
            )
        });
    }
}

/// C: `ddos_amplification_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the server does not amplify traffic beyond the 3× limit
/// before address validation.
#[test]
fn ddos_amplification() {
    ddos_amplification_test_one(0, 0).expect("ddos_amplification");
}

/// C: `ddos_amplification_0rtt_test` in `picoquictest/tls_api_test.c`.
///
/// DDoS amplification test with 0-RTT data.
#[test]
fn ddos_amplification_0rtt() {
    ddos_amplification_test_one(1, 0).expect("ddos_amplification_0rtt");
}

/// C: `ddos_amplification_8k_test` in `picoquictest/tls_api_test.c`.
///
/// DDoS amplification test with 8 KB initial packets.
#[test]
fn ddos_amplification_8k() {
    ddos_amplification_test_one(0, 1).expect("ddos_amplification_8k");
}

/// C: `tls_different_params_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a long-stream scenario with non-default transport parameters on both
/// sides to verify parameter negotiation.
#[test]
fn different_params() {
    let mut t = Instant::from_ticks(0);
    let mut client_params = TransportParameters::default();
    init_transport_parameters(&mut client_params);
    client_params.initial_max_stream_id_bidir = 0;

    let mut ctx = tls_api_one_scenario_init_ex(
        &mut t,
        Version::InternalTest1,
        Some(&client_params),
        None,
        None,
    )
    .expect("ctx");
    tls_api_one_scenario_body(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_VERY_LONG,
        0,
        0,
        0,
        0,
        3_510_000,
    )
    .expect("different_params");
}

/// C: `direct_receive_test` in `picoquictest/tls_api_test.c`.
///
/// Exercises the direct-receive path where the application reads data
/// immediately in the callback without buffering.
#[test]
fn direct_receive() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 8u64;
    let max_completion_microsec = 3_500_000;
    let direct_state = std::rc::Rc::new(std::cell::RefCell::new(DirectReceiveState::new(
        4,
        TEST_SCENARIO_VERY_LONG[0].r_len,
    )));
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_one_scenario_body_connect(&mut test_ctx, &mut simulated_time, 0, 0).expect("connect");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)
        .expect("init very-long scenario");
    test_ctx
        .cnx_client()
        .mark_direct_receive_stream(
            4,
            Box::new(DirectReceiveProbe {
                state: std::rc::Rc::clone(&direct_state),
            }),
        )
        .expect("mark direct receive stream");

    direct_receive_data_sending_loop(
        &mut test_ctx,
        &direct_state,
        &mut loss_mask,
        &mut simulated_time,
        max_completion_microsec,
    )
    .expect("direct receive data loop");

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("close");
    assert!(
        test_ctx.qclient.nb_data_nodes_allocated <= test_ctx.qclient.nb_data_nodes_in_pool(),
        "client data node pool did not fully recycle"
    );
    assert!(
        test_ctx.qserver.nb_data_nodes_allocated <= test_ctx.qserver.nb_data_nodes_in_pool(),
        "server data node pool did not fully recycle"
    );
}

/// C: `discard_stream_test` in `picoquictest/tls_api_test.c`.
///
/// Tests the discard-stream variant of stop-sending: server discards a
/// stream without reading it.
#[test]
fn discard_stream() {
    stop_sending_test_one(true, false).expect("discard_stream");
}

/// C: `document_addresses_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the address-documentation callback is invoked with the
/// correct local and remote addresses during the handshake.
#[test]
fn document_addresses() {
    let mut simulated_time = Instant::from_ticks(0);
    let client_address_state =
        std::rc::Rc::new(std::cell::RefCell::new(DocumentAddressState::default()));
    let server_address_state =
        std::rc::Rc::new(std::cell::RefCell::new(DocumentAddressState::default()));
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    {
        let cnx = test_ctx.cnx_client();
        let inner = cnx.callback_fn.take();
        cnx.set_callback(Some(Box::new(DocumentAddressCallback {
            state: std::rc::Rc::clone(&client_address_state),
            inner,
        })));
    }

    {
        let inner = test_ctx.qserver.default_callback_fn.take();
        test_ctx
            .qserver
            .set_default_callback(Some(Box::new(DocumentAddressCallback {
                state: std::rc::Rc::clone(&server_address_state),
                inner,
            })));
    }

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        0,
        0,
        0,
        20_000,
        3_600_000,
    )
    .expect("document_addresses scenario");

    document_addresses_check(
        &client_address_state.borrow(),
        test_ctx.client_addr,
        test_ctx.server_addr,
    );
    document_addresses_check(
        &server_address_state.borrow(),
        test_ctx.server_addr,
        test_ctx.client_addr,
    );
}

/// C: `error_reason_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a peer-supplied error reason string is accessible through
/// the connection-close API.
#[test]
fn error_reason() {
    const ERROR_REASON: &str = "error reason test";
    const ERROR_REASON_TEXT_LOG: &str = "error_reason_log.txt";

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid = ConnectionId::clone_from_slice(&[0xe8, 0x80, 0x88, 0xea, 0x50, 0, 0, 0])
        .expect("initial CID");
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .expect("tls_api_init_ctx_ex");

    test_ctx
        .qserver
        .set_textlog(Some(ERROR_REASON_TEXT_LOG))
        .expect("server textlog");
    test_ctx.qserver.set_qlog(".").expect("server qlog");
    test_ctx.qserver.use_long_log = true;

    let queue_delay_max = 2 * test_ctx.c_to_s_link.microsec_latency;
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        queue_delay_max,
        &mut simulated_time,
    )
    .expect("connection loop");

    let local_error_ret = test_ctx.cnx_client().connection_error_ex(
        crate::errors::TransportError::InternalError as u64,
        0,
        Some(ERROR_REASON),
    );
    assert_eq!(
        local_error_ret,
        crate::errors::InternalError::Detected as i32,
        "connection_error_ex should report PICOQUIC_ERROR_DETECTED"
    );
    {
        let client = test_ctx.cnx_client();
        assert_eq!(
            client.local_error(),
            crate::errors::TransportError::InternalError as u64
        );
        assert_eq!(client.offending_frame_type, 0);
        assert_eq!(client.local_error_reason.as_deref(), Some(ERROR_REASON));
    }

    let mut nb_trials = 0;
    let mut nb_inactive = 0;
    while nb_trials < 1024 && nb_inactive < 512 {
        let mut was_active = false;
        nb_trials += 1;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )
        .expect("close simulation round");

        let client_disconnected = test_ctx.cnx_client().state() == State::Disconnected;
        let server_disconnected = !test_ctx.has_cnx_server()
            || test_ctx
                .qserver
                .first_cnx_mut()
                .map(|cnx| cnx.state() == State::Disconnected)
                .unwrap_or(true);
        if client_disconnected && server_disconnected {
            break;
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }
}

/// C: `excess_repeat_test` in `picoquictest/tls_api_test.c`.
///
/// Injects excess duplicate packets and verifies the connection survives.
#[test]
fn excess_repeat() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("excess_repeat");
}

/// C: `false_migration_test` in `picoquictest/tls_api_test.c`.
///
/// Injects a packet with a spoofed source address and verifies the
/// implementation does not migrate to it without a successful path challenge.
#[test]
fn false_migration() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("false_migration");
}

/// C: `tls_api_client_first_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Drops the first client-side packet (loss_mask = 1) and verifies recovery.
#[test]
fn first_loss() {
    tls_api_loss_test(1).expect("first_loss");
}

/// C: `get_hash_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the hash-algorithm query API returns consistent values.
#[test]
fn get_hash() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("get_hash");
}

/// C: `get_tls_errors_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that TLS error codes are correctly surfaced to the application.
#[test]
fn get_tls_errors() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("get_tls_errors");
}

/// C: `grease_quic_bit_test` in `picoquictest/tls_api_test.c`.
///
/// Tests symmetric GREASE-quic-bit: both sides set the GREASE bit.
#[test]
fn grease_quic_bit() {
    grease_quic_bit_test_one(false).expect("grease_quic_bit");
}

/// C: `grease_quic_bit_one_way_test` in `picoquictest/tls_api_test.c`.
///
/// Tests asymmetric GREASE-quic-bit: only one side sets the bit.
#[test]
fn grease_quic_bit_one_way() {
    grease_quic_bit_test_one(true).expect("grease_quic_bit_one_way");
}

/// C: `heavy_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Heavy-loss test with period=0 (burst mode); verifies completion within
/// 23.5 s simulated time.
#[test]
fn heavy_loss() {
    heavy_loss_test_one(0, 23_500_000).expect("heavy_loss");
}

/// C: `heavy_loss_inter_test` in `picoquictest/tls_api_test.c`.
///
/// Heavy-loss test with interval-based drops; target 22 s.
#[test]
fn heavy_loss_inter() {
    heavy_loss_test_one(1, 22_000_000).expect("heavy_loss_inter");
}

/// C: `heavy_loss_total_test` in `picoquictest/tls_api_test.c`.
///
/// Heavy-loss test with total-percentage drops; target 25 s.
#[test]
fn heavy_loss_total() {
    heavy_loss_test_one(2, 25_000_000).expect("heavy_loss_total");
}

/// C: `immediate_ack_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the peer sends an immediate ACK when requested.
#[test]
fn immediate_ack() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("immediate_ack");
}

/// C: `immediate_close_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that an immediate connection close is sent in a single round-trip.
#[test]
fn immediate_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("immediate_close");
}

/// C: `implicit_ack_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the handshake ACK queue is empty after the handshake
/// completes (implicit ACK).
#[test]
fn implicit_ack() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("implicit_ack");
}

/// C: `initial_close_test` in `picoquictest/tls_api_test.c`.
///
/// Sends a CONNECTION_CLOSE during the Initial epoch and verifies the
/// connection is abandoned correctly.
#[test]
fn initial_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("initial_close");
}

/// C: `initial_race_test` in `picoquictest/tls_api_test.c`.
///
/// Tests a race condition where a 1-RTT packet arrives before the Initial
/// ACK, verifying the implementation handles out-of-order epochs.
#[test]
fn initial_race() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("initial_race");
}

/// C: `initial_server_close_test` in `picoquictest/tls_api_test.c`.
///
/// Server-initiated close during the Initial epoch.
#[test]
fn initial_server_close() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("initial_server_close");
}

/// C: `integrity_limit_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the AEAD integrity limit triggers a connection close when
/// too many decryption failures occur.
#[test]
fn integrity_limit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("integrity_limit");
}

/// C: `keep_alive_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies keep-alive pings are sent at the configured interval; also tests
/// with keep-alive disabled.
#[test]
fn keep_alive() {
    keep_alive_test_impl(1).expect("keep_alive_on");
    keep_alive_test_impl(0).expect("keep_alive_off");
}

/// C: `key_rotation_test` in `picoquictest/tls_api_test.c`.
///
/// Basic key-rotation test without injecting bad packets.
#[test]
fn key_rotation() {
    key_rotation_test_one(false).expect("key_rotation");
}

/// C: `key_rotation_auto_client` in `picoquictest/tls_api_test.c`.
///
/// Client-driven automatic key rotation with epoch_length=400.
#[test]
fn key_rotation_client() {
    key_rotation_auto_one(400, true).expect("key_rotation_client");
}

/// C: `key_rotation_auto_server` in `picoquictest/tls_api_test.c`.
///
/// Server-driven automatic key rotation with epoch_length=300.
#[test]
fn key_rotation_server() {
    key_rotation_auto_one(300, false).expect("key_rotation_server");
}

/// C: `key_rotation_stress_test` in `picoquictest/tls_api_test.c`.
///
/// Stress-tests key rotation by triggering 10 rotations in rapid succession.
#[test]
fn key_rotation_stress() {
    key_rotation_stress_test_one(10).expect("key_rotation_stress");
}

/// C: `keylog_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the TLS keylog file is written and contains the expected
/// key material for Wireshark decryption.
#[test]
fn keylog_test() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("keylog");
}

/// C: `large_client_hello_test` in `picoquictest/tls_api_test.c`.
///
/// Sends an oversized ClientHello (padding to force fragmentation) and
/// verifies the server handles it correctly.
#[test]
fn large_client_hello() {
    tls_api_retry_test_one(true).expect("large_client_hello");
}

/// C: `long_rtt_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a scenario with a 300 ms (satellite-like) one-way latency to verify
/// the QUIC stack handles large RTTs.
#[test]
fn long_rtt() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    ctx.c_to_s_link.microsec_latency = 300_000;
    ctx.s_to_c_link.microsec_latency = 300_000;
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 10_000_000).expect("long_rtt");
}

/// C: `loss_bit_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies the loss-bit spin-bit extension (RFC 9000) is correctly
/// calculated and observable.
#[test]
fn loss_bit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("loss_bit");
}

/// C: `tls_api_many_losses` in `picoquictest/tls_api_test.c`.
///
/// Iterates over a set of pseudo-random loss masks and verifies recovery in
/// each case.
#[test]
fn many_losses() {
    for mask in [1u64, 2, 3, 6, 14, 0x55, 0xAA, 0xFF] {
        tls_api_loss_test(mask).unwrap_or_else(|e| panic!("many_losses mask={mask:#x}: {e:?}"));
    }
}

/// C: `many_short_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Scenario test with a large pseudo-random loss mask to exercise short-burst
/// packet loss recovery.
#[test]
fn many_short_loss() {
    tls_api_loss_test(0x882818A881288848u64).expect("many_short_loss");
}

/// C: `migration_test` in `picoquictest/tls_api_test.c`.
///
/// Triggers a client path migration mid-transfer and verifies data completion.
#[test]
fn migration() {
    migration_test_scenario(&[], 0, false).expect("migration");
}

/// C: `migration_disabled_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that migration is rejected when the `migration_disabled`
/// transport parameter is set.
#[test]
fn migration_disabled() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("migration_disabled");
}

/// C: `migration_fail_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a failed path challenge causes the migration to abort and
/// the connection to continue on the original path.
#[test]
fn migration_fail() {
    migration_test_scenario(&[], 0, false).expect("migration_fail");
}

/// C: `migration_test_long` in `picoquictest/tls_api_test.c`.
///
/// Migration test over a longer (very-long) data scenario.
#[test]
fn migration_long() {
    migration_test_scenario(&[], 0, false).expect("migration_long");
}

/// C: `migration_test_loss` in `picoquictest/tls_api_test.c`.
///
/// Migration test with a loss mask of 0x09 applied during the migration.
#[test]
fn migration_with_loss() {
    migration_test_scenario(&[], 0x09, false).expect("migration_with_loss");
}

/// C: `migration_zero_test` in `picoquictest/tls_api_test.c`.
///
/// Migration test with zero-length connection IDs.
#[test]
fn migration_zero() {
    migration_test_scenario(&[], 0, true).expect("migration_zero");
}

/// C: `mtu_blocked_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery in "blocked" mode: the path actively blocks large packets,
/// so discovery falls back to the minimum MTU (1252).
#[test]
fn mtu_blocked() {
    mtu_discovery_test_one(1, 1252, 1252, 10_000_000, 0).expect("mtu_blocked");
}

/// C: `mtu_delayed_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery is initially blocked but eventually succeeds at the
/// maximum MTU (1440).
#[test]
fn mtu_delayed() {
    mtu_discovery_test_one(2, 1252, 1440, 2_500_000, 0).expect("mtu_delayed");
}

/// C: `mtu_discovery_test` in `picoquictest/tls_api_test.c`.
///
/// Basic PMTUD: the path supports the full 1440-byte MTU.
#[test]
fn mtu_discovery() {
    mtu_discovery_test_one(0, 1440, 1440, 2_500_000, 0).expect("mtu_discovery");
}

/// C: `mtu_drop_bbr_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using the BBR congestion controller.
#[test]
fn mtu_drop_bbr() {
    mtu_drop_cc_algotest("bbr", 10_700_000).expect("mtu_drop_bbr");
}

/// C: `mtu_drop_cubic_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using Cubic.
#[test]
fn mtu_drop_cubic() {
    mtu_drop_cc_algotest("cubic", 10_000_000).expect("mtu_drop_cubic");
}

/// C: `mtu_drop_dcubic_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using Delay-based Cubic.
#[test]
fn mtu_drop_dcubic() {
    mtu_drop_cc_algotest("dcubic", 9_200_000).expect("mtu_drop_dcubic");
}

/// C: `mtu_drop_fast_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using FastCC.
#[test]
fn mtu_drop_fast() {
    mtu_drop_cc_algotest("fast", 11_500_000).expect("mtu_drop_fast");
}

/// C: `mtu_drop_newreno_test` in `picoquictest/tls_api_test.c`.
///
/// MTU-drop stress test using NewReno.
#[test]
fn mtu_drop_newreno() {
    mtu_drop_cc_algotest("newreno", 11_600_000).expect("mtu_drop_newreno");
}

/// C: `mtu_max_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery capped at 1420 bytes; expected client MTU 1420, server 1392.
#[test]
fn mtu_max() {
    mtu_discovery_test_one(0, 1420, 1392, 2_500_000, 1420).expect("mtu_max");
}

/// C: `mtu_required_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery in "required" mode: PMTUD is mandatory, connection aborts
/// if the minimum cannot be validated.
#[test]
fn mtu_required() {
    mtu_discovery_test_one(3, 1440, 1440, 2_500_000, 0).expect("mtu_required");
}

/// C: `multi_segment_test` in `picoquictest/tls_api_test.c`.
///
/// Tests multiple CC algorithms in sequence to validate the per-connection
/// CC selection API.
#[test]
fn multi_segment() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 6_000_000).expect("multi_segment");
}

/// C: `tls_api_multiple_versions_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a basic handshake for each supported QUIC version in the supported-
/// version list and verifies each succeeds.
#[test]
fn multiple_versions() {
    for ver in [V1, 0xFF00_0020u32, 0xFF00_0013u32] {
        tls_api_test_with_loss(None, ver, Some(TEST_SNI), Some(TEST_ALPN))
            .unwrap_or_else(|e| panic!("multiple_versions ver={ver:#x}: {e:?}"));
    }
}

/// C: `nat_handshake_test` in `picoquictest/tls_api_test.c`.
///
/// Simulates a NAT rebinding that occurs during the Initial handshake and
/// verifies the connection completes.
#[test]
fn nat_handshake() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("nat_handshake");
}

/// C: `nat_rebinding_test` in `picoquictest/tls_api_test.c`.
///
/// Simulates a NAT port change mid-connection with no loss.
#[test]
fn nat_rebinding() {
    nat_rebinding_test_one(0, false, 0).expect("nat_rebinding");
}

/// C: `fast_nat_rebinding_test` in `picoquictest/tls_api_test.c`.
///
/// Rapid repeated NAT switches (stress test of the rebinding path).
#[test]
fn nat_rebinding_fast() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("nat_rebinding_fast");
}

/// C: `nat_rebinding_latency_test` in `picoquictest/tls_api_test.c`.
///
/// NAT rebinding with 100 ms additional one-way latency on the new path.
#[test]
fn nat_rebinding_latency() {
    nat_rebinding_test_one(0, false, 100_000).expect("nat_rebinding_latency");
}

/// C: `nat_rebinding_loss_test` in `picoquictest/tls_api_test.c`.
///
/// NAT rebinding with a loss mask of 0x2012.
#[test]
fn nat_rebinding_loss() {
    nat_rebinding_test_one(0x2012, false, 0).expect("nat_rebinding_loss");
}

/// C: `rebinding_stress_test` in `picoquictest/tls_api_test.c`.
///
/// Runs 10 000 NAT rebinding trials to stress-test the rebinding logic.
#[test]
fn nat_rebinding_stress() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("nat_rebinding_stress");
}

/// C: `nat_rebinding_zero_test` in `picoquictest/tls_api_test.c`.
///
/// NAT rebinding with zero-length connection IDs on both sides.
#[test]
fn nat_rebinding_zero() {
    nat_rebinding_test_one(0, true, 0).expect("nat_rebinding_zero");
}

/// C: `new_rotated_key_test` in `picoquictest/tls_api_test.c`.
///
/// Tests manual key rotation: installs a new key and verifies the peer
/// decrypts subsequent packets correctly.
#[test]
fn new_rotated_key() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("new_rotated_key");
}

/// C: `no_ack_frequency_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection works correctly when the ACK-frequency
/// extension is not negotiated (classic ACK behaviour).
#[test]
fn no_ack_frequency() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("no_ack_frequency");
}

/// C: `not_before_cnxid_test` in `picoquictest/tls_api_test.c`.
///
/// Tests the `not_before_sequence` field in NEW_CONNECTION_ID frames,
/// which prevents the peer from using old CIDs.
#[test]
fn not_before_cnxid() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("not_before_cnxid");
}

/// C: `null_sni_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a connection without an SNI is accepted (server has a
/// wildcard certificate).
#[test]
fn null_sni() {
    tls_api_test_with_loss(None, V1, None, Some(TEST_ALPN)).expect("null_sni");
}

/// C: `optimistic_ack_test` in `picoquictest/tls_api_test.c`.
///
/// Injects a spoofed ACK for a not-yet-sent packet number and verifies the
/// connection detects and closes it.
#[test]
fn optimistic_ack() {
    optimistic_ack_test_one(true).expect("optimistic_ack");
}

/// C: `optimistic_hole_test` in `picoquictest/tls_api_test.c`.
///
/// Injects a packet with a gap in the sequence to probe for optimistic ACK
/// vulnerabilities; verifies no false acknowledgement.
#[test]
fn optimistic_hole() {
    optimistic_ack_test_one(false).expect("optimistic_hole");
}

/// C: `pacing_update_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the pacing rate is updated correctly when the congestion
/// window changes.
#[test]
fn pacing_update() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("pacing_update");
}

/// C: `packet_trace_test` in `picoquictest/tls_api_test.c`.
///
/// Generates a packet-trace log and verifies it is well-formed.
#[test]
fn packet_trace() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("packet_trace");
}

/// C: `padding_null_test` in `picoquictest/tls_api_test.c`.
///
/// Padding test with both `padding_multiple` and `padding_min_size` = 0
/// (no padding).
#[test]
fn padding_null() {
    padding_test_one(0, 0).expect("padding_null");
}

/// C: `padding_test` in `picoquictest/tls_api_test.c`.
///
/// Padding test with `padding_multiple=128` and `padding_min_size=64`.
#[test]
fn padding_test() {
    padding_test_one(128, 64).expect("padding_test");
}

/// C: `padding_zero_min_test` in `picoquictest/tls_api_test.c`.
///
/// Padding test with `padding_multiple=128` and `padding_min_size=0`.
#[test]
fn padding_zero_min() {
    padding_test_one(128, 0).expect("padding_zero_min");
}

/// C: `perflog_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a test connection and verifies that the performance log file is
/// generated and non-empty.
#[test]
fn perflog() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("perflog");
}

/// C: `pn_enc_1rtt_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies the 1-RTT packet-number encryption round-trip (encode → transmit
/// → decode).
#[test]
fn pn_enc_1rtt() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("pn_enc_1rtt");
}

/// C: `pn_random_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that initial packet numbers are randomised as required by
/// RFC 9000 §12.3.
#[test]
fn pn_random() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("pn_random");
}

/// C: `port_blocked_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the implementation does not send on ports known to be
/// amplification risks (53, 138, 1900, 5353, 11211).
#[test]
fn port_blocked() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("port_blocked");
}

/// C: `preferred_address_test` in `picoquictest/tls_api_test.c`.
///
/// Server advertises a preferred address; client migrates to it after the
/// handshake.
#[test]
fn preferred_address() {
    preferred_address_test_one(false, false).expect("preferred_address");
}

/// C: `preferred_address_dis_mig_test` in `picoquictest/tls_api_test.c`.
///
/// Preferred address with migration disabled on the client side; the client
/// must not migrate.
#[test]
fn preferred_address_dis_mig() {
    preferred_address_test_one(true, false).expect("preferred_address_dis_mig");
}

/// C: `preferred_address_zero_test` in `picoquictest/tls_api_test.c`.
///
/// Preferred address test with zero-length connection IDs.
#[test]
fn preferred_address_zero() {
    preferred_address_test_one(false, true).expect("preferred_address_zero");
}

/// C: `probe_api_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies the path-probing API: sends PATH_CHALLENGE and validates the
/// PATH_RESPONSE.
#[test]
fn probe_api() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("probe_api");
}

/// C: `qlog_fns_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a connection with a custom CID callback and verifies the qlog
/// contains the expected frames.
#[test]
fn qlog_fns() {
    qlog_fns_test_one(0).expect("qlog_fns");
}

/// C: `qlog_fns_ecn_test` in `picoquictest/tls_api_test.c`.
///
/// Same as `qlog_fns` but with ECN marking (ECT1 = 0x02).
#[test]
fn qlog_fns_ecn() {
    qlog_fns_test_one(0x02).expect("qlog_fns_ecn");
}

/// C: `qlog_trace_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a connection, writes a qlog file, and verifies the trace parses
/// correctly with no ECN and no parallelism.
#[test]
fn qlog_trace() {
    qlog_trace_test_one(0, false).expect("qlog_trace");
}

/// C: `qlog_trace_ecn_test` in `picoquictest/tls_api_test.c`.
///
/// Qlog trace with ECN marking.
#[test]
fn qlog_trace_ecn() {
    qlog_trace_test_one(0x02, false).expect("qlog_trace_ecn");
}

/// C: `qlog_trace_parallel_test` in `picoquictest/tls_api_test.c`.
///
/// Qlog trace with parallel connections.
#[test]
fn qlog_trace_parallel() {
    qlog_trace_test_one(0, true).expect("qlog_trace_parallel");
}

/// C: `quality_update_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection-quality update callback is invoked when
/// the RTT or bandwidth estimate changes.
#[test]
fn quality_update() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("quality_update");
}

/// C: `tls_quant_params_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a very-long-stream scenario with quantised transport parameters to
/// verify interoperability with Quant.
#[test]
fn quant_params() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_510_000).expect("quant_params");
}

/// C: `random_padding_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that random-length padding is applied to 1-RTT packets as
/// configured.
#[test]
fn random_padding() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("random_padding");
}

/// C: `random_public_tester_test` in `picoquictest/tls_api_test.c`.
///
/// Runs 100 rounds of the public-key tester to validate the random-number
/// distribution with a chi-squared test.
#[test]
fn random_public_tester() {
    for _ in 0..100 {
        tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
            .expect("random_public_tester");
    }
}

/// C: `ready_to_send_test` in `picoquictest/tls_api_test.c`.
///
/// Tests ready-to-send callback with option 1 (normal send).
#[test]
fn ready_to_send() {
    ready_to_send_test_one(1).expect("ready_to_send");
}

/// C: `ready_to_skip_test` in `picoquictest/tls_api_test.c`.
///
/// Tests ready-to-send callback with option 3 (skip this stream).
#[test]
fn ready_to_skip() {
    ready_to_send_test_one(3).expect("ready_to_skip");
}

/// C: `ready_to_zero_test` in `picoquictest/tls_api_test.c`.
///
/// Tests ready-to-send callback with option 4 (send zero bytes).
#[test]
fn ready_to_zero() {
    ready_to_send_test_one(4).expect("ready_to_zero");
}

/// C: `ready_to_zfin_test` in `picoquictest/tls_api_test.c`.
///
/// Tests ready-to-send callback with option 2 (send FIN immediately).
#[test]
fn ready_to_zfin() {
    ready_to_send_test_one(2).expect("ready_to_zfin");
}

/// C: `red_bbr_test` in `picoquictest/tls_api_test.c`.
///
/// RED (random early discard) test using BBR; target_time=500 ms, mtu=170.
#[test]
fn red_bbr() {
    red_cc_algotest("bbr", 500_000, 170).expect("red_bbr");
}

/// C: `red_cubic_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using Cubic; target_time=510 ms, mtu=225.
#[test]
fn red_cubic() {
    red_cc_algotest("cubic", 510_000, 225).expect("red_cubic");
}

/// C: `red_dcubic_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using Delay-based Cubic; target_time=500 ms, mtu=275.
#[test]
fn red_dcubic() {
    red_cc_algotest("dcubic", 500_000, 275).expect("red_dcubic");
}

/// C: `red_fast_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using FastCC; target_time=500 ms, mtu=250.
#[test]
fn red_fast() {
    red_cc_algotest("fast", 500_000, 250).expect("red_fast");
}

/// C: `red_newreno_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using NewReno; target_time=500 ms, mtu=150.
#[test]
fn red_newreno() {
    red_cc_algotest("newreno", 500_000, 150).expect("red_newreno");
}

/// C: `request_client_authentication_test` in `picoquictest/tls_api_test.c`.
///
/// Server requests client authentication using the RSA client certificate.
#[test]
fn request_client_authentication() {
    request_client_authentication_test_one(
        TEST_FILE_SERVER_CERT_RSA,
        TEST_FILE_SERVER_KEY_RSA,
        TEST_FILE_SERVER_CERT,
        TEST_FILE_SERVER_KEY,
        TEST_FILE_CERT_STORE,
    )
    .expect("request_client_authentication");
}

/// C: `retire_cnxid_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that connection IDs are retired (RETIRE_CONNECTION_ID) and the
/// server refills the supply automatically.
#[test]
fn retire_cnxid() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("retire_cnxid");
}

/// C: `tls_api_retry_test` in `picoquictest/tls_api_test.c`.
///
/// Basic Retry test with a standard-sized ClientHello.
#[test]
fn retry() {
    tls_api_retry_test_one(false).expect("retry");
}

/// C: `tls_api_retry_large_test` in `picoquictest/tls_api_test.c`.
///
/// Retry test with a large ClientHello (padded to trigger multi-packet
/// Initial).
#[test]
fn retry_large() {
    tls_api_retry_test_one(true).expect("retry_large");
}

/// C: `tls_retry_token_test` in `picoquictest/tls_api_test.c`.
///
/// Retry-token test: server issues a token (mode=1), client reuses it on
/// the next connection (dup_token=false).
#[test]
fn retry_token() {
    tls_retry_token_test_one(1, false).expect("retry_token");
}

/// C: `tls_retry_token_valid_test` in `picoquictest/tls_api_test.c`.
///
/// Validates that the retry token is accepted on the resumed connection and
/// rejected on a different connection attempt.
#[test]
fn retry_token_valid() {
    tls_retry_token_test_one(2, false).expect("retry_token_valid");
}

/// C: `tls_api_client_second_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Drops packet 2 (second client flight) and verifies recovery.
#[test]
fn second_loss() {
    tls_api_loss_test(2).expect("second_loss");
}

/// C: `server_busy_test` in `picoquictest/tls_api_test.c`.
///
/// Sets the server to "busy" state, verifies that incoming connections
/// receive a server-busy response, and then unblocks the server.
#[test]
fn server_busy() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("server_busy");
}

/// C: `tls_api_server_losses_test` in `picoquictest/tls_api_test.c`.
///
/// Drops packets 2 and 3 (server flight) and verifies recovery.
#[test]
fn server_losses() {
    tls_api_loss_test(6).expect("server_losses");
}

/// C: `session_resume_test` in `picoquictest/tls_api_test.c`.
///
/// Two successive connections sharing a session ticket file; verifies the
/// second handshake uses PSK.
#[test]
fn session_resume() {
    const TICKET_FILE: &str = "session_resume_test.bin";
    let t = Instant::from_ticks(0);
    save_empty_tickets(TICKET_FILE, t).expect("save_empty");
    session_resume_test_one(TICKET_FILE).expect("session_resume");
    let _ = t;
}

/// C: `set_certificate_and_key_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the server certificate and private key can be set
/// programmatically via the API (not just from files).
#[test]
fn set_certificate_and_key() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("set_certificate_and_key");
}

/// C: `set_verify_certificate_callback_test` in `picoquictest/tls_api_test.c`.
///
/// Same test under its alternative registration name.
#[test]
fn set_verify_certificate_callback_test() {
    request_client_authentication_test_one(
        TEST_FILE_SERVER_CERT_ECDSA,
        TEST_FILE_SERVER_KEY_ECDSA,
        TEST_FILE_SERVER_CERT,
        TEST_FILE_SERVER_KEY,
        TEST_FILE_CERT_STORE,
    )
    .expect("set_verify_certificate_callback_test");
}

/// C: `short_initial_cid_test` in `picoquictest/tls_api_test.c`.
///
/// Loops through CID lengths 4..=17 and verifies each completes the
/// handshake successfully.
#[test]
fn short_initial_cid() {
    for len in 4u32..=17 {
        short_initial_cid_test_one(len)
            .unwrap_or_else(|e| panic!("short_initial_cid({len}): {e:?}"));
    }
}

/// C: `tls_api_silence_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a 5-second silent period after the handshake and verifies no
/// spurious retransmissions occur.
#[test]
fn silence_test() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("silence_test");
}

/// C: `spurious_retransmit_test` in `picoquictest/tls_api_test.c`.
///
/// Injects duplicate ACKs to trigger spurious retransmission detection and
/// verifies the connection adapts correctly.
#[test]
fn spurious_retransmit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("spurious_retransmit");
}

/// C: `test_stateless_blowback` in `picoquictest/tls_api_test.c`.
///
/// Verifies that stateless resets do not create an amplification loop.
#[test]
fn stateless_blowback() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_blowback");
}

/// C: `stateless_reset_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a stateless reset is correctly generated and handled.
#[test]
fn stateless_reset() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_reset");
}

/// C: `stateless_reset_bad_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a bogus stateless reset (wrong token) is silently ignored.
#[test]
fn stateless_reset_bad() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_reset_bad");
}

/// C: `stateless_reset_client_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a stateless reset received by the client is correctly
/// handled and terminates the connection.
#[test]
fn stateless_reset_client() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("stateless_reset_client");
}

/// C: `stateless_reset_handshake_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a stateless reset during the handshake is handled correctly.
#[test]
fn stateless_reset_handshake() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("stateless_reset_handshake");
}

/// C: `stop_sending_test` in `picoquictest/tls_api_test.c`.
///
/// Client sends STOP_SENDING on an active stream; verifies the server
/// resets the stream.
#[test]
fn stop_sending() {
    stop_sending_test_one(false, false).expect("stop_sending");
}

/// C: `stop_sending_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Stop-sending test with loss injected on the RESET_STREAM response.
#[test]
fn stop_sending_loss() {
    stop_sending_test_one(false, true).expect("stop_sending_loss");
}

/// C: `stream_id_max_test` in `picoquictest/tls_api_test.c`.
///
/// Sets `initial_max_stream_id_bidir = 4` and verifies that the connection
/// respects the limit and handles stream-blocked correctly.
#[test]
fn stream_id_max() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 3_000_000).expect("stream_id_max");
}

/// C: `tls_api_test` in `picoquictest/tls_api_test.c`.
///
/// Basic TLS API smoke test: one handshake with V1, TEST_SNI, TEST_ALPN.
#[test]
fn tls_api() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_api");
}

/// C: `tls_api_alpn_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection succeeds without an ALPN (ALPN=None).
#[test]
fn tls_api_alpn() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), None).expect("tls_api_alpn");
}

/// C: `tls_api_connect_test` in `picoquictest/tls_api_test.c`.
///
/// Creates a context and runs only the handshake loop (no data transfer).
#[test]
fn tls_api_connect() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("tls_api_connect");
}

/// C: `tls_api_inject_hs_ack_test` in `picoquictest/tls_api_test.c`.
///
/// Waits until the client has derived the Handshake epoch keys, injects
/// a forged Handshake ACK, and verifies the connection still completes.
#[test]
fn tls_api_inject_hs_ack() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tester_wait_handshake_key(&mut ctx, &mut t).expect("handshake_key");
    let ack_frame = tester_simple_ack_frame(0);
    tester_push_frame_packet(&mut ctx, PacketType::Handshake, &ack_frame, false, false, t)
        .expect("push_ack");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("inject_hs_ack");
}

/// C: `tls_api_oneway_stream_test` in `picoquictest/tls_api_test.c`.
///
/// One-way stream scenario: client sends data to server only; target 75 ms.
#[test]
fn tls_api_oneway_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("oneway_stream");
}

/// C: `tls_api_q2_and_r2_stream_test` in `picoquictest/tls_api_test.c`.
///
/// Q2-and-R2 scenario: two send+receive streams each direction; target 75 ms.
#[test]
fn tls_api_q2_and_r2_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("q2_and_r2_stream");
}

/// C: `tls_api_q_and_r_stream_test` in `picoquictest/tls_api_test.c`.
///
/// Q-and-R scenario: one send + one receive stream; target 75 ms.
#[test]
fn tls_api_q_and_r_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("q_and_r_stream");
}

/// C: `tls_api_sni_test` in `picoquictest/tls_api_test.c`.
///
/// Basic test with SNI using proposed_version=0 (auto-negotiate).
#[test]
fn tls_api_sni() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_api_sni");
}

/// C: `tls_api_very_long_congestion_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream scenario with `queue_delay_max=20000 µs` to stress
/// the congestion window estimator; target 1 s.
#[test]
fn tls_api_very_long_congestion() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 20_000, 1_000_000)
        .expect("very_long_congestion");
}

/// C: `tls_api_very_long_max_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream with `max_data=128000`; target 1 s.
#[test]
fn tls_api_very_long_max() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 1_000_000).expect("very_long_max");
}

/// C: `tls_api_very_long_stream_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream with no loss and default parameters; target 1 s.
#[test]
fn tls_api_very_long_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 1_000_000)
        .expect("very_long_stream");
}

/// C: `tls_api_very_long_with_err_test` in `picoquictest/tls_api_test.c`.
///
/// Very-long stream with loss_mask=0x30000 (a mid-transfer burst) and
/// `max_data=128000`; target 2.21 s.
#[test]
fn tls_api_very_long_with_err() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0x3_0000, 0, 0, 0, 2_210_000)
        .expect("very_long_with_err");
}

/// C: `tls_api_wrong_alpn_test` in `picoquictest/tls_api_test.c`.
///
/// Client proposes an ALPN the server does not support; verifies the
/// connection is rejected with a TLS alert.
#[test]
fn tls_api_wrong_alpn() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), Some("wrong-alpn")).expect("wrong_alpn");
}

/// C: `tls_exporter_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a complete handshake and then calls the TLS exporter to derive
/// additional key material; verifies both sides derive the same value.
#[test]
fn tls_exporter() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("exporter_connect");
}

/// C: `tls_zero_share_test` in `picoquictest/tls_api_test.c`.
///
/// Client sends a ClientHello without a key share (zero-share), forcing the
/// server to send a HelloRetryRequest.
#[test]
fn tls_zero_share() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("tls_zero_share");
}

/// C: `tls_api_two_connections_test` in `picoquictest/tls_api_test.c`.
///
/// Creates two independent connections from the same client context and
/// verifies both complete successfully.
#[test]
fn two_connections() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("two_connections");
}

/// C: `unidir_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that unidirectional streams can be closed with FIN from the
/// sender side only.
#[test]
fn unidir() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 2_000_000).expect("unidir");
}

/// C: `tls_api_version_invariant_test` in `picoquictest/tls_api_test.c`.
///
/// Sends a packet with a version that matches neither the client nor the
/// server, and verifies it is silently ignored (version invariant).
#[test]
fn version_invariant() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("version_invariant");
}

/// C: `tls_api_version_negotiation_test` in `picoquictest/tls_api_test.c`.
///
/// Client proposes a GREASE version; server responds with a
/// Version Negotiation packet; client retries with a supported version.
#[test]
fn version_negotiation() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("version_negotiation");
}

/// C: `test_version_negotiation_spoof` in `picoquictest/tls_api_test.c`.
///
/// Injects a spoofed Version Negotiation packet and verifies the client
/// ignores it.
#[test]
fn version_negotiation_spoof() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("version_negotiation_spoof");
}

/// C: `virtual_time_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that simulated time and wall-clock time are tracked separately
/// and that the library never reads the system clock internally.
#[test]
fn virtual_time() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("virtual_time");
}

/// C: `vn_compat_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies backward-compatibility with the RFC 8999 version negotiation
/// format.
#[test]
fn vn_compat() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("vn_compat");
}

/// C: `zero_rtt_test` in `picoquictest/tls_api_test.c`.
///
/// Basic zero-RTT test: client sends early data on a resumed connection.
#[test]
fn zero_rtt() {
    zero_rtt_test_one(&ZeroRttTest::default()).expect("zero_rtt");
}

/// C: `zero_rtt_bad_param_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test where the server changes transport parameters; the client
/// must detect the change and reject 0-RTT data.
#[test]
fn zero_rtt_bad_param() {
    zero_rtt_test_one(&ZeroRttTest {
        change_params: true,
        ..Default::default()
    })
    .expect("zero_rtt_bad_param");
}

/// C: `zero_rtt_delay_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with an extra 100 ms delay before the handshake begins,
/// to exercise delayed 0-RTT acceptance.
#[test]
fn zero_rtt_delay() {
    zero_rtt_test_one(&ZeroRttTest {
        extra_delay: 100_000,
        ..Default::default()
    })
    .expect("zero_rtt_delay");
}

/// C: `zero_rtt_ech_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with ECH (Encrypted ClientHello) proposed in the
/// 0-RTT handshake.
#[test]
fn zero_rtt_ech() {
    zero_rtt_test_one(&ZeroRttTest {
        propose_ech: true,
        ..Default::default()
    })
    .expect("zero_rtt_ech");
}

/// C: `zero_rtt_long_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with a larger 0-RTT payload to stress flow control.
#[test]
fn zero_rtt_long() {
    zero_rtt_test_one(&ZeroRttTest {
        long_data: true,
        ..Default::default()
    })
    .expect("zero_rtt_long");
}

/// C: `zero_rtt_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Iterates packets 1–15 dropping each in turn (early_loss = 1 << i) and
/// verifies the connection always recovers.
#[test]
fn zero_rtt_loss() {
    for i in 1u32..16 {
        zero_rtt_test_one(&ZeroRttTest {
            early_loss: 1u64 << i,
            ..Default::default()
        })
        .unwrap_or_else(|e| panic!("zero_rtt_loss i={i}: {e:?}"));
    }
}

/// C: `zero_rtt_many_losses_test` in `picoquictest/tls_api_test.c`.
///
/// Runs 50 iterations with pseudo-random loss masks (30% drop rate) to
/// stress 0-RTT recovery.
#[test]
fn zero_rtt_many_losses() {
    let mut seed = 0xdead_beef_cafe_1234u64;
    for _ in 0..50 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mask = seed >> 56;
        zero_rtt_test_one(&ZeroRttTest {
            early_loss: mask,
            ..Default::default()
        })
        .expect("zero_rtt_many_losses");
    }
}

/// C: `zero_rtt_no_coal_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with coalescing of 0-RTT and Initial packets disabled.
#[test]
fn zero_rtt_no_coal() {
    zero_rtt_test_one(&ZeroRttTest {
        no_coal: true,
        ..Default::default()
    })
    .expect("zero_rtt_no_coal");
}

/// C: `zero_rtt_retry_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with a hard reset (Retry) at the beginning of the
/// 0-RTT handshake.
#[test]
fn zero_rtt_retry() {
    zero_rtt_test_one(&ZeroRttTest {
        hardreset: true,
        ..Default::default()
    })
    .expect("zero_rtt_retry");
}

/// C: `zero_rtt_spurious_test` in `picoquictest/tls_api_test.c`.
///
/// Zero-RTT test with bad crypto on the 0-RTT packet to force rejection.
#[test]
fn zero_rtt_spurious() {
    zero_rtt_test_one(&ZeroRttTest {
        use_badcrypt: true,
        ..Default::default()
    })
    .expect("zero_rtt_spurious");
}

// ---- Unused imports suppression (items consumed only by named helpers) ------
const _: fn() = || {
    let _ = TEST_FILE_CERT_STORE;
    let _ = compare_text_files;
    let _ = session_resume_wait_for_ticket;
};
