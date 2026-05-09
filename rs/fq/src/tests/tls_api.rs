//! Test cases for `picoquictest/tls_api_test.c`.

#![allow(non_snake_case)]

use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::cell::RefCell;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::rc::Rc;

use crate::bytestream::{BYTESTREAM_MAX_BUFFER_SIZE, ByteStream};
use crate::frames::FrameType;
use crate::internal::{
    CHALLENGE_REPEAT_MAX, CID_REFRESH_DELAY, Connection, ENFORCED_INITIAL_MTU, Epoch,
    INTEROP_VERSION_LATEST, MICROSEC_HANDSHAKE_MAX, NB_PATH_TARGET, PN_RANDOM_MIN, PacketType,
    SUPPORTED_VERSIONS, TOKEN_DELAY_LONG, TOKEN_DELAY_SHORT, Version, create_long_header,
    init_transport_parameters, parse_long_packet_type, protect_packet_header,
    update_payload_length, varint_encode,
};
use crate::tests::util::{
    TEST_ALPN, TEST_FILE_CERT_STORE, TEST_FILE_CERT_STORE_ED25519, TEST_FILE_CLIENT_CERT_ED25519,
    TEST_FILE_CLIENT_KEY_ED25519, TEST_FILE_SERVER_BAD_CERT, TEST_FILE_SERVER_CERT,
    TEST_FILE_SERVER_CERT_ECDSA, TEST_FILE_SERVER_CERT_ED25519, TEST_FILE_SERVER_CERT_RSA,
    TEST_FILE_SERVER_KEY, TEST_FILE_SERVER_KEY_ECDSA, TEST_FILE_SERVER_KEY_ED25519,
    TEST_FILE_SERVER_KEY_RSA, TEST_SNI, TestApiStreamDesc, TestSimPacket, TestTlsApiCtx,
    ZeroRttTest, cid_length_test_one, cnx_ddos_test_loop, compare_text_files,
    ddos_amplification_test_one, grease_quic_bit_test_one, heavy_loss_test_one,
    keep_alive_test_impl, key_rotation_auto_one, key_rotation_stress_test_one,
    key_rotation_test_one, migration_test_scenario, mtu_discovery_test_one, mtu_drop_cc_algotest,
    nat_rebinding_test_one, optimistic_ack_test_one, padding_test_one, preferred_address_test_one,
    qlog_fns_test_one, qlog_trace_test_one, ready_to_send_test_one, red_cc_algotest,
    request_client_authentication_test_one, save_empty_tickets, session_resume_test_one,
    session_resume_wait_for_ticket, short_initial_cid_test_one, stop_sending_test_one,
    test_api_init_send_recv_scenario, test_random, test_random_bytes, test_uniform_random,
    tester_push_frame_packet, tester_simple_ack_frame, tester_wait_handshake_key,
    tls_api_close_with_losses, tls_api_connection_loop, tls_api_data_sending_loop,
    tls_api_init_ctx, tls_api_init_ctx_ex, tls_api_init_ctx_ex2, tls_api_loss_test,
    tls_api_one_scenario_body, tls_api_one_scenario_body_connect, tls_api_one_scenario_body_ex,
    tls_api_one_scenario_body_verify, tls_api_one_scenario_init_ex, tls_api_one_scenario_verify,
    tls_api_one_sim_round, tls_api_one_sim_round_with_loss, tls_api_retry_test_one,
    tls_api_synch_to_empty_loop, tls_api_test_with_loss, tls_retry_token_test_one,
    transmit_cnxid_test_one, wait_client_connection_ready, zero_rtt_test_one,
};
use crate::tls_api::{
    aead_confidentiality_limit, aead_integrity_limit, ecb_create_by_name, get_certs_from_file,
    get_hash_algorithm_by_name, hash_create, hash_get_length, picoquic_get_cipher_suite_by_id_v,
    tls_api_init, tls_api_unload,
};
use crate::{
    AES_128_GCM_SHA256, CHACHA20_POLY1305_SHA256, CallbackEvent, CongestionAlgorithm, ConnectionId,
    Duration, Instant, LossbitVersion, MAX_PACKET_SIZE, NB_PACKET_CONTEXT, PacketContext,
    PmtudPolicy, Quic, RESET_SECRET_SIZE, State, StreamDataCallback, TransportError,
    TransportParameters, get_congestion_algorithm, is_handshake_error,
    register_all_congestion_control_algorithms,
};

const V1: u32 = Version::InternalTest1 as u32;
const QUALITY_UPDATE_CSV: &str = "quality_update.csv";
const QUALITY_UPDATE_REF: &str = "picoquictest/quality_update_ref.txt";
const RANDOM_PADDING_TICKET_FILE: &str = "random_padding_tickets.bin";
const RANDOM_PADDING_TEXT_LOG: &str = "random_padding_log.txt";
const QUALITY_UPDATE_CSV: &str = "quality_update.csv";
const QUALITY_UPDATE_REF: &str = "picoquictest/quality_update_ref.txt";
const RANDOM_PADDING_TICKET_FILE: &str = "random_padding_tickets.bin";
const RANDOM_PADDING_TEXT_LOG: &str = "random_padding_log.txt";

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
/// Starts a long transfer, simulates a vanished client, and verifies each
/// congestion controller stops retransmitting before the C repeat threshold.
fn excess_repeat_test_one(
    cc_algo: &'static CongestionAlgorithm,
    repeat_target: usize,
) -> crate::Result<()> {
    const TEST_LATENCY: u64 = 30_000;
    const PICOSEC_100MBPS: u64 = 80_000;
    const NB_INITIAL_LOOP_MAX: usize = 64;
    const NB_LOOPS_MAX: usize = 3000;
    const MAX_DELTA_T: u64 = 3_000_000;
    const MAX_DISCONNECTED_TIME: u64 = 30_000_000;
    const TEST_SCENARIO_VERY_LONG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut initial_cid_bytes = [0xe8, 0xce, 0x55, 0, 0, 0, 0, 0];
    initial_cid_bytes[7] = cc_algo.congestion_algorithm_number;
    let initial_cid =
        ConnectionId::clone_from_slice(&initial_cid_bytes).ok_or(crate::Error::Generic)?;

    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .ok_or(crate::Error::Generic)?;

    test_ctx.c_to_s_link.microsec_latency = TEST_LATENCY;
    test_ctx.c_to_s_link.picosec_per_byte = PICOSEC_100MBPS;
    test_ctx.s_to_c_link.microsec_latency = TEST_LATENCY;
    test_ctx.s_to_c_link.picosec_per_byte = PICOSEC_100MBPS;
    test_ctx.qserver.set_default_congestion_algorithm(cc_algo);
    test_ctx.qserver.set_qlog(".").ok();

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        return Err(crate::Error::Generic);
    }
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)?;

    let mut nb_initial_loop = 0usize;
    while nb_initial_loop < NB_INITIAL_LOOP_MAX {
        if test_ctx.has_cnx_server() && test_ctx.cnx_client().state() >= State::Ready {
            nb_initial_loop += 1;
        }
        tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 16)?;
        if !test_ctx.client_ready() || !test_ctx.server_ready() {
            return Err(crate::Error::Generic);
        }
    }

    let repeat_target =
        if cc_algo.congestion_algorithm_number == 3 || cc_algo.congestion_algorithm_number == 4 {
            200
        } else {
            repeat_target
        };
    let max_disconnected_time = simulated_time.ticks() + MAX_DISCONNECTED_TIME;
    let mut nb_loops = 0usize;
    let mut nb_repeated = 0usize;
    let mut packet_bytes = [0u8; MAX_PACKET_SIZE];

    while test_ctx.qserver.current_number_connections() > 0
        && test_ctx.has_cnx_server()
        && test_ctx.cnx_server().state() != State::Disconnected
    {
        let old_time = simulated_time.ticks();
        let next_time = test_ctx.qserver.next_wake_time(simulated_time);
        if next_time < old_time || next_time - old_time > MAX_DELTA_T {
            return Err(crate::Error::Generic);
        }

        simulated_time = Instant::from_ticks(next_time);
        if simulated_time.ticks() > max_disconnected_time {
            return Err(crate::Error::Generic);
        }

        let send_length = test_ctx
            .qserver
            .prepare_next_packet(simulated_time, &mut packet_bytes)?
            .send_length;
        if send_length > 0 {
            nb_repeated += 1;
            if nb_repeated > repeat_target {
                return Err(crate::Error::Generic);
            }
        }

        nb_loops += 1;
        if nb_loops > NB_LOOPS_MAX {
            return Err(crate::Error::Generic);
        }
    }

    Ok(())
}

#[test]
fn excess_repeat() {
    const NB_REPEAT_MAX: usize = 128;
    const ALGORITHMS: &[&str] = &["newreno", "cubic", "dcubic", "fastcc", "bbr", "prague"];

    register_all_congestion_control_algorithms();

    for algo_id in ALGORITHMS {
        let cc_algo = get_congestion_algorithm(algo_id).unwrap_or_else(|| {
            panic!("congestion algorithm {algo_id} must be registered");
        });
        excess_repeat_test_one(cc_algo, NB_REPEAT_MAX)
            .unwrap_or_else(|e| panic!("excess_repeat({algo_id}): {e:?}"));
    }
}

/// C: `false_migration_test` in `picoquictest/tls_api_test.c`.
///
/// Injects a packet with a spoofed source address and verifies the
/// implementation does not migrate to it without a successful path challenge.
const TEST_SCENARIO_Q_AND_R: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 2000,
}];

const TEST_SCENARIO_VERY_LONG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

const TEST_SCENARIO_Q2_AND_R2: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 0,
        q_len: 531,
        r_len: 11_000,
    },
];

const TEST_SCENARIO_MIGRATION_FAIL_VERY_LONG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

fn false_migration_context(false_pc: PacketContext) -> (PacketType, Epoch) {
    match false_pc {
        PacketContext::Application => (PacketType::OneRttProtected, Epoch::OneRtt),
        PacketContext::Handshake => (PacketType::Handshake, Epoch::Handshake),
        PacketContext::Initial => (PacketType::Initial, Epoch::Initial),
    }
}

fn false_migration_has_remote_cid(cnx: &Connection) -> bool {
    cnx.remote_connection_id_stashes
        .iter()
        .flat_map(|stash| stash.connection_ids.iter())
        .any(|remote_cid| !remote_cid.connection_id.is_empty())
}

fn false_migration_should_inject(
    test_ctx: &mut TestTlsApiCtx,
    target_client: bool,
    false_pc: PacketContext,
    false_rank: u64,
    require_client_remote_cid: bool,
) -> bool {
    if target_client {
        let Some(cnx) = test_ctx.qclient.first_cnx_mut() else {
            return false;
        };
        cnx.pkt_ctx[false_pc as usize].send_sequence > false_rank
            && (!require_client_remote_cid || false_migration_has_remote_cid(cnx))
    } else {
        let Some(cnx) = test_ctx.qserver.first_cnx_mut() else {
            return false;
        };
        cnx.pkt_ctx[false_pc as usize].send_sequence > false_rank
    }
}

fn false_migration_protect_packet(
    cnx: &mut Connection,
    packet_type: PacketType,
    false_pc: PacketContext,
    epoch: Epoch,
    simulated_time: Instant,
    send_buffer: &mut [u8; MAX_PACKET_SIZE],
) -> crate::Result<usize> {
    let checksum_overhead = cnx.get_checksum_length(epoch);
    let length = checksum_overhead + 32;
    let mut packet = cnx.allocate_packet().ok_or(crate::Error::Memory)?;
    packet.packet_type = packet_type;
    packet.packet_context = false_pc;
    packet.checksum_overhead = checksum_overhead;
    packet.bytes[..length].fill(0);

    if cnx.paths.is_empty() {
        return Err(crate::Error::Generic);
    }

    let mut send_length = 0usize;
    let mut path = cnx.paths.remove(0);
    cnx.finalize_and_protect_packet(
        &mut packet,
        0,
        length,
        0,
        checksum_overhead,
        &mut send_length,
        send_buffer,
        MAX_PACKET_SIZE,
        &mut path,
        simulated_time,
    );
    cnx.paths.insert(0, path);

    if send_length == 0 {
        return Err(crate::Error::Generic);
    }

    Ok(send_length)
}

fn false_migration_inject(
    test_ctx: &mut TestTlsApiCtx,
    target_client: bool,
    false_pc: PacketContext,
    simulated_time: Instant,
) -> crate::Result<()> {
    let (packet_type, epoch) = false_migration_context(false_pc);
    let mut send_buffer = [0u8; MAX_PACKET_SIZE];
    let send_length = if target_client {
        false_migration_protect_packet(
            test_ctx.cnx_client(),
            packet_type,
            false_pc,
            epoch,
            simulated_time,
            &mut send_buffer,
        )?
    } else {
        if !test_ctx.has_cnx_server() {
            return Err(crate::Error::Generic);
        }
        false_migration_protect_packet(
            test_ctx.cnx_server(),
            packet_type,
            false_pc,
            epoch,
            simulated_time,
            &mut send_buffer,
        )?
    };

    let mut sim_packet = TestSimPacket::create()?;
    if send_length > sim_packet.bytes.len() {
        return Err(crate::Error::Memory);
    }
    sim_packet.bytes[..send_length].copy_from_slice(&send_buffer[..send_length]);
    sim_packet.length = send_length;
    sim_packet.ecn_mark = test_ctx.packet_ecn_default;

    let mut false_address = if target_client {
        test_ctx.client_addr
    } else {
        test_ctx.server_addr
    };
    false_address.set_port(false_address.port() + 1234);
    sim_packet.addr_from = Some(false_address);
    sim_packet.addr_to = Some(if target_client {
        test_ctx.server_addr
    } else {
        test_ctx.client_addr
    });

    if target_client {
        test_ctx.c_to_s_link.submit(sim_packet, simulated_time);
    } else {
        test_ctx.s_to_c_link.submit(sim_packet, simulated_time);
    }

    Ok(())
}

fn false_migration_both_disconnected(test_ctx: &mut TestTlsApiCtx) -> bool {
    let client_disconnected = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.state() == State::Disconnected)
        .unwrap_or(true);
    let server_disconnected = test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| cnx.state() == State::Disconnected)
        .unwrap_or(true);

    client_disconnected && server_disconnected
}

fn false_migration_backlog_empty(test_ctx: &mut TestTlsApiCtx) -> bool {
    let client_empty = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.is_backlog_empty())
        .unwrap_or(true);
    let server_empty = test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| cnx.is_backlog_empty())
        .unwrap_or(true);

    client_empty && server_empty
}

fn false_migration_test_scenario(
    scenario: &[TestApiStreamDesc],
    target_client: bool,
    false_pc: PacketContext,
    false_rank: u64,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut nb_injected = 0usize;
    let mut nb_trials = 0usize;
    let mut nb_inactive = 0usize;
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).ok_or(crate::Error::Memory)?;

    while nb_trials < 1024
        && nb_inactive < 512
        && (!test_ctx.client_ready() || !test_ctx.server_ready())
    {
        let mut was_active = false;
        nb_trials += 1;

        if nb_injected == 0
            && false_migration_should_inject(
                &mut test_ctx,
                target_client,
                false_pc,
                false_rank,
                true,
            )
        {
            false_migration_inject(&mut test_ctx, target_client, false_pc, simulated_time)?;
            nb_injected += 1;
        }

        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;

        if false_migration_both_disconnected(&mut test_ctx) {
            break;
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    test_api_init_send_recv_scenario(&mut test_ctx, scenario)?;

    nb_trials = 0;
    nb_inactive = 0;
    while nb_trials < 1024
        && nb_inactive < 256
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let mut was_active = false;
        nb_trials += 1;

        if nb_injected == 0
            && false_migration_should_inject(
                &mut test_ctx,
                target_client,
                false_pc,
                false_rank,
                false,
            )
        {
            false_migration_inject(&mut test_ctx, target_client, false_pc, simulated_time)?;
            nb_injected += 1;
        }

        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if test_ctx.test_finished && false_migration_backlog_empty(&mut test_ctx) {
            break;
        }
    }

    if nb_injected == 0 {
        return Err(crate::Error::Generic);
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

#[test]
fn false_migration() {
    for target_client in [true, false] {
        false_migration_test_scenario(
            TEST_SCENARIO_Q2_AND_R2,
            target_client,
            PacketContext::Initial,
            0,
        )
        .unwrap_or_else(|e| panic!("false_migration initial target_client={target_client}: {e:?}"));

        false_migration_test_scenario(
            TEST_SCENARIO_Q2_AND_R2,
            target_client,
            PacketContext::Handshake,
            0,
        )
        .unwrap_or_else(|e| {
            panic!("false_migration handshake target_client={target_client}: {e:?}")
        });

        for seq in 0..4 {
            false_migration_test_scenario(
                TEST_SCENARIO_Q2_AND_R2,
                target_client,
                PacketContext::Application,
                seq,
            )
            .unwrap_or_else(|e| {
                panic!("false_migration application target_client={target_client} seq={seq}: {e:?}")
            });
        }
    }
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
    let valid_hash = "sha256";
    let invalid_hash = "no_such_hash_nada_niente";

    let hash_length = hash_get_length(valid_hash);
    assert!(
        hash_length > 0 && hash_length <= 1024,
        "valid hash length should fit the C test scratch buffer"
    );
    assert_eq!(hash_length, 32, "sha256 digest length");
    assert_eq!(
        hash_get_length(invalid_hash),
        0,
        "invalid hash names should not resolve to a digest length"
    );

    let algorithm = get_hash_algorithm_by_name(valid_hash).expect("valid hash algorithm lookup");
    assert_eq!(algorithm.output_size(), hash_length);
    assert!(
        get_hash_algorithm_by_name(invalid_hash).is_none(),
        "invalid hash algorithm lookup should fail"
    );

    let mut hash = hash_create(valid_hash).expect("valid hash creation");
    assert_eq!(hash.output_size(), hash_length);
    assert!(
        hash_create(invalid_hash).is_none(),
        "invalid hash creation should fail"
    );

    let mut outbuf = [0u8; 1024];
    hash.update(&[1, 2, 3, 4]);
    hash.finalize_into_reset(&mut outbuf[..hash_length])
        .expect("finalize sha256 digest");
    assert!(
        outbuf[..16].iter().any(|&b| b != 0),
        "digest prefix should not remain all zero"
    );
}

/// C: `get_tls_errors_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies the TLS API error surfaces for invalid algorithms, unloaded TLS
/// provider state, and bad key/certificate files.
#[test]
fn get_tls_errors() {
    struct TlsApiResetGuard;

    impl Drop for TlsApiResetGuard {
        fn drop(&mut self) {
            tls_api_init();
        }
    }

    let _guard = TlsApiResetGuard;
    tls_api_init();

    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x9e, 0x71, 0x5e, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut test_ctx =
        tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid)).expect("ctx");

    let invalid_stuff = "no_such_stuff_nada_niente";
    let invalid_id = 0xFFFE_8808u32 as i32;
    let ecb_key = [0xaau8; 16];

    assert!(
        ecb_create_by_name(false, &ecb_key, invalid_stuff).is_none(),
        "invalid ECB cipher name must be rejected"
    );
    assert!(
        picoquic_get_cipher_suite_by_id_v(invalid_id, false).is_none(),
        "invalid high-memory cipher suite ID must not resolve"
    );
    assert!(
        test_ctx
            .qserver
            .set_cipher_suite(invalid_id as u16)
            .is_err(),
        "invalid cipher suite ID must be rejected"
    );
    assert!(
        test_ctx
            .qserver
            .set_key_exchange(invalid_id as u16)
            .is_err(),
        "invalid key exchange ID must be rejected"
    );
    assert!(
        picoquic_get_cipher_suite_by_id_v(i32::from(AES_128_GCM_SHA256), true).is_some()
            || picoquic_get_cipher_suite_by_id_v(i32::from(AES_128_GCM_SHA256), false).is_some(),
        "AES_128_GCM_SHA256 must resolve in at least one memory mode"
    );
    assert!(
        picoquic_get_cipher_suite_by_id_v(invalid_id, true).is_none(),
        "invalid low-memory cipher suite ID must not resolve"
    );

    tls_api_unload();

    let cnx_created = test_ctx
        .qclient
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            Some(&test_ctx.server_addr),
            simulated_time,
            V1,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .is_some();
    assert!(
        !cnx_created,
        "connection creation must fail after TLS API unload"
    );
    assert!(
        test_ctx
            .qclient
            .set_private_key_from_file("some bad file name.not")
            .is_err(),
        "bad private-key filename must fail"
    );
    assert!(
        get_certs_from_file("some bad file name.not").is_none(),
        "bad certificate filename must fail"
    );

    tls_api_init();
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
fn immediate_ack_test_one() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x1a, 0x1a, 0x1a, 0x1a, 0x1a, 0x1a, 0x1a, 0x1a])
            .ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .ok_or(crate::Error::Generic)?;

    test_ctx.qserver.set_qlog(".").ok();
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;

    let immediate_ack_frame = [FrameType::ImmediateAck as u8, FrameType::Padding as u8];
    test_ctx.cnx_client().queue_misc_frame(
        &immediate_ack_frame,
        false,
        PacketContext::Application,
    )?;

    let mut immediate_received_at_server = None;
    for _ in 0..16 {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        if test_ctx.has_cnx_server()
            && test_ctx.cnx_server().ack_ctx[PacketContext::Application as usize].act[0]
                .is_immediate_ack_required
        {
            immediate_received_at_server = Some(simulated_time.ticks());
            break;
        }
    }
    let Some(immediate_received_at_server) = immediate_received_at_server else {
        panic!("Immediate ACK not received after 16 rounds");
    };

    let mut immediate_cleared_at_server = None;
    for _ in 0..16 {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        if test_ctx.has_cnx_server()
            && !test_ctx.cnx_server().ack_ctx[PacketContext::Application as usize].act[0]
                .is_immediate_ack_required
        {
            immediate_cleared_at_server = Some(simulated_time.ticks());
            break;
        }
    }
    let Some(immediate_cleared_at_server) = immediate_cleared_at_server else {
        panic!("Immediate ACK not cleared after 16 rounds");
    };
    assert_eq!(
        immediate_cleared_at_server, immediate_received_at_server,
        "ACK not quite immediate"
    );

    let mut all_acked = false;
    for _ in 0..32 {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        if test_ctx.cnx_client().is_cnx_backlog_empty() {
            all_acked = true;
            break;
        }
    }
    assert!(
        all_acked,
        "ACK was not received at {}",
        simulated_time.ticks()
    );

    Ok(())
}

#[test]
fn immediate_ack() {
    immediate_ack_test_one().expect("immediate_ack");
}

/// C: `immediate_close_test` in `picoquictest/tls_api_test.c`.
///
/// Closes the server immediately, then verifies client traffic after the close
/// does not make the server send any more packets and both endpoints disconnect.
fn immediate_close_test_one() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;
    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        return Err(crate::Error::Generic);
    }

    let nb_packet_sent_before_close = {
        let server = test_ctx.cnx_server();
        let nb_packet_sent_before_close = server.nb_packets_sent;
        server.close_immediate();
        nb_packet_sent_before_close
    };

    let buffer = [0xaau8; 128];
    test_ctx.cnx_client().add_to_stream(4, &buffer, true)?;

    for _ in 0..256 {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        if test_ctx.cnx_client().state() >= State::Disconnected {
            break;
        }
    }

    if test_ctx.cnx_client().state() != State::Disconnected {
        return Err(crate::Error::Generic);
    }

    if !test_ctx.has_cnx_server() {
        return Err(crate::Error::Generic);
    }

    let server = test_ctx.cnx_server();
    if server.state() != State::Disconnected
        || server.nb_packets_sent != nb_packet_sent_before_close
    {
        return Err(crate::Error::Generic);
    }

    Ok(())
}

#[test]
fn immediate_close() {
    immediate_close_test_one().expect("immediate_close");
}

/// C: `implicit_ack_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the handshake ACK queue is empty after the handshake
/// completes (implicit ACK).
#[test]
fn implicit_ack() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    let mut loss = 0u64;
    tls_api_connection_loop(&mut ctx, &mut loss, 0, &mut t).expect("implicit_ack");
    wait_client_connection_ready(&mut ctx, &mut t).expect("implicit_ack_ready");
    assert!(ctx.has_cnx_server(), "server connection not accepted");

    for pc in [PacketContext::Initial, PacketContext::Handshake] {
        let pc_index = pc as usize;
        assert!(
            ctx.cnx_client().pkt_ctx[pc_index].pending.is_empty(),
            "pending queue type {pc:?} not empty on client"
        );
        assert!(
            ctx.cnx_server().pkt_ctx[pc_index].pending.is_empty(),
            "pending queue type {pc:?} not empty on server"
        );
        assert!(
            ctx.cnx_client().pkt_ctx[pc_index].retransmitted.is_empty(),
            "retransmitted queue type {pc:?} not empty on client"
        );
        assert!(
            ctx.cnx_server().pkt_ctx[pc_index].retransmitted.is_empty(),
            "retransmitted queue type {pc:?} not empty on server"
        );
    }
}

/// C: `initial_close_test` in `picoquictest/tls_api_test.c`.
///
/// Sends only the first Initial packet, forces a client handshake failure, and
/// verifies the server observes the client's transport error.
fn initial_close_test_one() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut was_active = false;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_one_sim_round(
        &mut test_ctx,
        &mut simulated_time,
        Instant::from_ticks(0),
        &mut was_active,
    )?;

    {
        let client = test_ctx.cnx_client();
        client.connection_state = State::HandshakeFailure;
        client.local_error = 0xDEAD;
        client.next_wake_time = simulated_time;
    }

    for _ in 0..128 {
        was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        if test_ctx.has_cnx_server() {
            break;
        }
    }

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    assert!(
        test_ctx.has_cnx_server(),
        "server connection deleted, cannot verify error code"
    );
    assert_eq!(
        test_ctx.cnx_server().state(),
        State::Disconnected,
        "server must disconnect after Initial close"
    );
    assert_eq!(
        test_ctx.cnx_server().remote_error(),
        0xDEAD,
        "server must report the client close error"
    );
    assert_eq!(
        test_ctx.cnx_client().state(),
        State::Disconnected,
        "client must disconnect after sending Initial close"
    );
    assert!(
        simulated_time.ticks() <= 50_000,
        "simulated time exceeded C bound: {}",
        simulated_time.ticks()
    );

    Ok(())
}

#[test]
fn initial_close() {
    initial_close_test_one().expect("initial_close");
}

/// C: `initial_race_test` in `picoquictest/tls_api_test.c`.
///
/// Tests a race condition where a 1-RTT packet arrives before the Initial
/// ACK, verifying the implementation handles out-of-order epochs.
fn initial_race_test_one() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut was_active = false;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    tls_api_one_sim_round(
        &mut test_ctx,
        &mut simulated_time,
        Instant::from_ticks(0),
        &mut was_active,
    )?;
    assert!(
        !test_ctx.c_to_s_link.packets.is_empty(),
        "initial simulation round should queue the client's first Initial"
    );

    simulated_time = Instant::from_ticks(simulated_time.ticks().saturating_add(100));
    test_ctx.cnx_client().initial_repeat_needed = true;
    tls_api_one_sim_round(
        &mut test_ctx,
        &mut simulated_time,
        Instant::from_ticks(0),
        &mut was_active,
    )?;
    test_ctx.cnx_client().initial_repeat_needed = false;

    let queued_initials = test_ctx.c_to_s_link.packets.len();
    assert!(
        queued_initials >= 2,
        "forced Initial repeat should queue two client Initial packets, got {queued_initials}"
    );

    for _ in 0..1024 {
        if !test_ctx.s_to_c_link.packets.is_empty() {
            break;
        }
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
    }

    assert!(
        !test_ctx.s_to_c_link.packets.is_empty(),
        "server did not queue its first packet"
    );
    assert!(test_ctx.has_cnx_server(), "no server connection");

    {
        let server = test_ctx.cnx_server();
        server.next_wake_time =
            Instant::from_ticks(server.next_wake_time.ticks().saturating_add(2_000));
    }

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_Q2_AND_R2)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)
}

#[test]
fn initial_race() {
    initial_race_test_one().expect("initial_race");
}

/// C: `initial_server_close_test` in `picoquictest/tls_api_test.c`.
///
/// Server-initiated close during the Initial epoch.
fn initial_server_close_test_one() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut was_active = false;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    for _ in 0..32 {
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;

        if test_ctx.has_cnx_server() && test_ctx.cnx_server().state() == State::ServerAlmostReady {
            break;
        }
    }

    assert!(
        test_ctx.has_cnx_server(),
        "server connection not accepted before close; client_state={:?}, c_to_s_packets={}, s_to_c_packets={}, time={}",
        test_ctx.cnx_client().state(),
        test_ctx.c_to_s_link.packets.len(),
        test_ctx.s_to_c_link.packets.len(),
        simulated_time.ticks()
    );
    assert_eq!(
        test_ctx.cnx_server().state(),
        State::ServerAlmostReady,
        "server did not reach ServerAlmostReady"
    );

    {
        let server = test_ctx.cnx_server();
        server.connection_state = State::HandshakeFailure;
        server.local_error = 0xDEAD;
        // The Rust simulator polls `next_wake_time` directly in this path.
        server.next_wake_time = simulated_time;
    }

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    if test_ctx.has_cnx_server() {
        assert_eq!(
            test_ctx.cnx_server().state(),
            State::Disconnected,
            "server must disconnect after Initial close"
        );
    }
    assert_eq!(
        test_ctx.cnx_client().state(),
        State::Disconnected,
        "client must disconnect after server Initial close"
    );
    assert_eq!(
        test_ctx.cnx_client().remote_error(),
        0xDEAD,
        "client must report the server close error"
    );
    assert!(
        simulated_time.ticks() <= 50_000,
        "simulated time exceeded C bound: {}",
        simulated_time.ticks()
    );

    Ok(())
}

#[test]
fn initial_server_close() {
    initial_server_close_test_one().expect("initial_server_close");
}

/// C: `integrity_limit_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the AEAD integrity limit triggers a connection close when
/// too many decryption failures occur.
fn integrity_limit_test_one() -> crate::Result<()> {
    const TEST_SCENARIO_VERY_LONG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid = ConnectionId::clone_from_slice(&[0x15, 0x4e, 0x98, 0x14, 0, 0, 0, 1])
        .ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .ok_or(crate::Error::Generic)?;

    test_ctx.qserver.set_qlog(".").ok();
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_VERY_LONG)?;

    let mut nb_initial_loop = 0usize;
    while nb_initial_loop < 64 {
        if test_ctx.has_cnx_server()
            && test_ctx.cnx_server().crypto_context[Epoch::OneRtt as usize]
                .aead_decrypt
                .is_some()
        {
            nb_initial_loop += 1;
        }

        tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 16)?;
    }

    assert!(test_ctx.has_cnx_server(), "server connection not accepted");
    let limit = {
        let server = test_ctx.cnx_server();
        let aead = server.crypto_context[Epoch::OneRtt as usize]
            .aead_decrypt
            .as_deref()
            .ok_or(crate::Error::Generic)?;
        aead_confidentiality_limit(aead)
    };

    assert_eq!(
        test_ctx.cnx_server().crypto_epoch_length_max,
        limit,
        "server confidentiality limit"
    );
    assert_eq!(
        test_ctx.cnx_client().crypto_epoch_length_max,
        limit,
        "client confidentiality limit"
    );

    let (local_cid, peer_addr, local_addr) = {
        let server = test_ctx.cnx_server();
        let aead = server.crypto_context[Epoch::OneRtt as usize]
            .aead_decrypt
            .as_deref()
            .ok_or(crate::Error::Generic)?;
        server.crypto_failure_count = aead_integrity_limit(aead);
        (
            server.local_cnxid(),
            server.path_peer_addr_by_index(0),
            server.path_local_addr_by_index(0),
        )
    };

    let mut packet = [0u8; 256];
    let cid_bytes = local_cid.as_bytes();
    if packet.len() < 1 + cid_bytes.len() {
        return Err(crate::Error::BufferTooSmall);
    }
    packet[1..1 + cid_bytes.len()].copy_from_slice(cid_bytes);
    packet[0] |= 0x40;

    let _ = test_ctx.qserver.incoming_packet(
        &mut packet,
        &peer_addr,
        &local_addr,
        0,
        0,
        simulated_time,
    );

    assert_eq!(
        test_ctx.cnx_server().state(),
        State::Disconnecting,
        "server connection must disconnect after AEAD integrity limit"
    );
    assert_eq!(
        test_ctx.cnx_server().local_error,
        TransportError::AeadLimitReached as u64,
        "server local error"
    );

    Ok(())
}

#[test]
fn integrity_limit() {
    integrity_limit_test_one().expect("integrity_limit");
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
/// Runs manual key rotation with no injection, then with bogus rotation packets
/// injected toward the client and toward the server.
#[test]
fn key_rotation() {
    key_rotation_test_one(0).expect("key_rotation");
    key_rotation_test_one(2).expect("key_rotation injection toward client");
    key_rotation_test_one(1).expect("key_rotation injection toward server");
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
/// Stress-tests rapid client-driven key rotation during a sustained transfer.
fn key_rotation_stress_backlog_empty(test_ctx: &mut TestTlsApiCtx) -> bool {
    let client_empty = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.is_cnx_backlog_empty())
        .unwrap_or(true);
    let server_empty = test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| cnx.is_cnx_backlog_empty())
        .unwrap_or(true);

    client_empty && server_empty
}

fn key_rotation_stress_wait_for_server_close(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let closing_time = simulated_time.ticks().saturating_add(4_000_000);

    while simulated_time.ticks() < closing_time {
        let mut was_active = false;

        if test_ctx.qserver.current_number_connections() == 0 {
            break;
        }
        if !test_ctx.has_cnx_server() {
            return Err(crate::Error::Generic);
        }
        if test_ctx.cnx_server().state() == State::Disconnected {
            break;
        }

        tls_api_one_sim_round(
            test_ctx,
            simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
    }

    Ok(())
}

fn key_rotation_stress_test_one(nb_packets: u64) -> crate::Result<()> {
    const TEST_SCENARIO_SUSTAINED: &[TestApiStreamDesc] = &[
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
        TestApiStreamDesc {
            stream_id: 12,
            previous_stream_id: 8,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 16,
            previous_stream_id: 12,
            q_len: 257,
            r_len: 1_000_000,
        },
    ];

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut nb_trials = 0usize;
    let mut nb_inactive = 0usize;
    let max_trials = 100_000usize;
    let max_rotations = 100usize;
    let mut nb_rotation = 0usize;
    let mut rotation_sequence = 100u64;
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).ok_or(crate::Error::Generic)?;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_SUSTAINED)?;

    while nb_trials < max_trials
        && nb_inactive < 256
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let mut was_active = false;
        nb_trials += 1;

        let should_rotate = {
            let client = test_ctx.cnx_client();
            client.pkt_ctx[PacketContext::Application as usize].send_sequence > rotation_sequence
                && client.key_phase_enc == client.key_phase_dec
        };

        if should_rotate {
            let send_sequence =
                test_ctx.cnx_client().pkt_ctx[PacketContext::Application as usize].send_sequence;
            rotation_sequence = send_sequence.saturating_add(nb_packets);
            nb_rotation += 1;

            if nb_rotation > max_rotations {
                break;
            }

            test_ctx.cnx_client().start_key_rotation()?;
        }

        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if test_ctx.test_finished && key_rotation_stress_backlog_empty(&mut test_ctx) {
            break;
        }
    }

    if test_ctx.client_ready() {
        tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;
    }

    key_rotation_stress_wait_for_server_close(&mut test_ctx, &mut simulated_time)
}

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
    const TEST_KEYLOG_FILE_CLIENT: &str = "test_keylog_client.txt";
    const TEST_KEYLOG_FILE_SERVER: &str = "test_keylog_server.txt";
    const TEST_SCENARIO_Q_AND_R: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    std::fs::File::create(TEST_KEYLOG_FILE_SERVER).expect("reset server keylog file");
    std::fs::File::create(TEST_KEYLOG_FILE_CLIENT).expect("reset client keylog file");

    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x55, 0x17, 0xe9, 0x10, 0x90, 0x0, 0x0, 0x0])
            .expect("initial CID");
    let mut test_ctx =
        tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid)).expect("ctx");

    test_ctx.qserver.set_sslkeylog_enabled(true);
    test_ctx.qclient.set_sslkeylog_enabled(true);
    test_ctx
        .qserver
        .set_key_log_file(Some(TEST_KEYLOG_FILE_SERVER));
    test_ctx
        .qclient
        .set_key_log_file(Some(TEST_KEYLOG_FILE_CLIENT));
    assert!(test_ctx.qserver.is_sslkeylog_enabled());
    assert!(test_ctx.qclient.is_sslkeylog_enabled());

    tls_api_one_scenario_body_ex(
        &mut test_ctx,
        &mut simulated_time,
        TEST_SCENARIO_Q_AND_R,
        1_000_000,
        0,
        0,
        20_000,
        1_200_000,
        &[],
    )
    .expect("keylog q_and_r scenario");

    drop(test_ctx);

    let server_size = std::fs::metadata(TEST_KEYLOG_FILE_SERVER)
        .expect("server keylog metadata")
        .len();
    let client_size = std::fs::metadata(TEST_KEYLOG_FILE_CLIENT)
        .expect("client keylog metadata")
        .len();
    assert!(
        server_size >= 128,
        "server keylog should contain TLS secrets, got {server_size} bytes"
    );
    assert!(
        client_size >= 128,
        "client keylog should contain TLS secrets, got {client_size} bytes"
    );
}

/// C: `large_client_hello_test` in `picoquictest/tls_api_test.c`.
///
/// Sends an oversized ClientHello (padding to force fragmentation) and
/// verifies the server handles it correctly.
#[test]
fn large_client_hello() {
    const TEST_SCENARIO_Q_AND_R: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");

    test_ctx.cnx_client().test_large_chello = true;
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
    .expect("large_client_hello q_and_r scenario");

    assert!(test_ctx.has_cnx_server(), "server connection not accepted");
    assert_eq!(
        test_ctx.cnx_server().nb_retransmission_total,
        0,
        "server retransmitted during large ClientHello scenario"
    );
    assert_eq!(
        test_ctx.cnx_client().nb_retransmission_total,
        0,
        "client retransmitted during large ClientHello scenario"
    );
}

/// C: `long_rtt_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a scenario with a 300 ms (satellite-like) one-way latency to verify
/// the QUIC stack handles large RTTs.
#[test]
fn long_rtt() {
    const LATENCY: u64 = 300_000;
    const TEST_SCENARIO_VERY_LONG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    let mut t = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x10, 0x10, 30, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut ctx = tls_api_init_ctx_ex(&mut t, 0, None, Some(&initial_cid)).expect("ctx");
    ctx.c_to_s_link.microsec_latency = LATENCY;
    ctx.s_to_c_link.microsec_latency = LATENCY;
    ctx.qserver.set_qlog(".").ok();
    tls_api_one_scenario_body(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_VERY_LONG,
        0,
        0,
        0,
        2 * LATENCY,
        3_600_000,
    )
    .expect("long_rtt");
}

/// C: `loss_bit_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies all client/server `enable_loss_bit` transport-parameter
/// combinations complete the C many-streams scenario within the target time.
#[test]
fn loss_bit() {
    const TEST_SCENARIO_MANY_STREAMS: &[TestApiStreamDesc] = &[
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 1000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 1000,
        },
        TestApiStreamDesc {
            stream_id: 12,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 1000,
        },
        TestApiStreamDesc {
            stream_id: 16,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 1000,
        },
        TestApiStreamDesc {
            stream_id: 20,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 350,
        },
        TestApiStreamDesc {
            stream_id: 24,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 225,
        },
        TestApiStreamDesc {
            stream_id: 28,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 700,
        },
        TestApiStreamDesc {
            stream_id: 32,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
        TestApiStreamDesc {
            stream_id: 36,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
        TestApiStreamDesc {
            stream_id: 40,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
        TestApiStreamDesc {
            stream_id: 44,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
        TestApiStreamDesc {
            stream_id: 48,
            previous_stream_id: 0,
            q_len: 32,
            r_len: 32,
        },
    ];

    for i in 0..=3 {
        let mut client_parameters = TransportParameters::default();
        let mut server_parameters = TransportParameters::default();
        init_transport_parameters(&mut client_parameters);
        init_transport_parameters(&mut server_parameters);

        client_parameters.enable_loss_bit = i & 1;
        server_parameters.enable_loss_bit = ((i > 1) as i32) & 1;

        let mut simulated_time = Instant::from_ticks(0);
        let mut test_ctx = tls_api_one_scenario_init_ex(
            &mut simulated_time,
            Version::InternalTest1,
            Some(&client_parameters),
            Some(&server_parameters),
            None,
        )
        .expect("loss_bit context");

        let mut loss_mask = 0u64;
        tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
            .unwrap_or_else(|e| {
                panic!(
                    "loss_bit handshake client={} server={}: {e:?}",
                    client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
                )
            });
        wait_client_connection_ready(&mut test_ctx, &mut simulated_time).unwrap_or_else(|e| {
            panic!(
                "loss_bit wait-ready client={} server={}: {e:?}",
                client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
            )
        });
        assert!(
            test_ctx.client_ready() && test_ctx.server_ready(),
            "loss_bit handshake did not reach ready client={} server={} client_state={:?} server_state={:?} time={}",
            client_parameters.enable_loss_bit,
            server_parameters.enable_loss_bit,
            test_ctx.qclient.first_cnx_mut().map(|cnx| cnx.state()),
            test_ctx.qserver.first_cnx_mut().map(|cnx| cnx.state()),
            simulated_time.ticks()
        );
        test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MANY_STREAMS).unwrap_or_else(
            |e| {
                panic!(
                    "loss_bit scenario-init client={} server={}: {e:?}",
                    client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
                )
            },
        );
        tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
            .unwrap_or_else(|e| {
                panic!(
                    "loss_bit data-loop client={} server={}: {e:?}",
                    client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
                )
            });
        let client_state = test_ctx.qclient.first_cnx_mut().map(|cnx| cnx.state());
        let server_state = test_ctx.qserver.first_cnx_mut().map(|cnx| cnx.state());
        assert!(
            test_ctx.test_finished,
            "loss_bit did not finish many-streams scenario client={} server={} client_state={:?} server_state={:?} time={}",
            client_parameters.enable_loss_bit,
            server_parameters.enable_loss_bit,
            client_state,
            server_state,
            simulated_time.ticks()
        );
        tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 250_000)
            .unwrap_or_else(|e| {
                panic!(
                    "loss_bit verify client={} server={}: {e:?}",
                    client_parameters.enable_loss_bit, server_parameters.enable_loss_bit
                )
            });
    }
}

/// C: `tls_api_many_losses` in `picoquictest/tls_api_test.c`.
///
/// Exercises the C preprogrammed loss-mask matrix, then 50 deterministic
/// 30%-loss q_and_r scenario runs with `max_data=128000`.
const MANY_LOSSES_Q_AND_R: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 2000,
}];

fn many_losses_link_activity(test_ctx: &TestTlsApiCtx) -> (u64, u64) {
    (
        test_ctx.c_to_s_link.packets_sent + test_ctx.c_to_s_link.packets_dropped,
        test_ctx.s_to_c_link.packets_sent + test_ctx.s_to_c_link.packets_dropped,
    )
}

fn many_losses_set_shared_loss(test_ctx: &mut TestTlsApiCtx, loss_mask: u64) {
    test_ctx.c_to_s_link.loss_mask = Some(loss_mask);
    test_ctx.s_to_c_link.loss_mask = Some(loss_mask);
}

fn many_losses_sync_shared_loss(
    test_ctx: &mut TestTlsApiCtx,
    before: (u64, u64),
    loss_mask: &mut u64,
) {
    let after = many_losses_link_activity(test_ctx);
    if after.0 != before.0
        && let Some(mask) = test_ctx.c_to_s_link.loss_mask
    {
        *loss_mask = mask;
    }
    if after.1 != before.1
        && let Some(mask) = test_ctx.s_to_c_link.loss_mask
    {
        *loss_mask = mask;
    }
}

fn many_losses_connection_loop(
    test_ctx: &mut TestTlsApiCtx,
    loss_mask: &mut u64,
    queue_delay_max: u64,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    test_ctx.c_to_s_link.queue_delay_max = queue_delay_max;
    test_ctx.s_to_c_link.queue_delay_max = queue_delay_max;

    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while nb_trials < 1024
        && nb_inactive < 512
        && (!test_ctx.client_ready() || !test_ctx.server_ready())
    {
        let before = many_losses_link_activity(test_ctx);
        let mut was_active = false;
        nb_trials += 1;
        many_losses_set_shared_loss(test_ctx, *loss_mask);
        tls_api_one_sim_round(
            test_ctx,
            simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        many_losses_sync_shared_loss(test_ctx, before, loss_mask);

        let client_disc = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|c| c.connection_state == State::Disconnected)
            .unwrap_or(true);
        let server_disc = !test_ctx.has_cnx_server()
            || test_ctx
                .qserver
                .first_cnx_mut()
                .map(|c| c.connection_state == State::Disconnected)
                .unwrap_or(true);
        if client_disc && server_disc {
            break;
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    Ok(())
}

fn many_losses_data_sending_loop(
    test_ctx: &mut TestTlsApiCtx,
    loss_mask: &mut u64,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while nb_trials < 4_000_000
        && nb_inactive < 256
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let before = many_losses_link_activity(test_ctx);
        let mut was_active = false;
        nb_trials += 1;
        many_losses_set_shared_loss(test_ctx, *loss_mask);
        tls_api_one_sim_round(
            test_ctx,
            simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;
        many_losses_sync_shared_loss(test_ctx, before, loss_mask);

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

fn many_losses_loss_test(loss_mask: u64) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    let mut loss_mask = loss_mask;
    many_losses_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

fn many_losses_q_and_r_scenario(init_loss_mask: u64) -> crate::Result<()> {
    const MAX_DATA: u64 = 128_000;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    let mut handshake_loss_mask = 0u64;

    tls_api_connection_loop(
        &mut test_ctx,
        &mut handshake_loss_mask,
        0,
        &mut simulated_time,
    )?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;
    if !test_ctx.has_cnx_server() {
        return Err(crate::Error::InvalidState);
    }

    {
        let cnx = test_ctx.cnx_client();
        cnx.maxdata_local = MAX_DATA;
        cnx.maxdata_remote = MAX_DATA;
    }
    {
        let cnx = test_ctx.cnx_server();
        cnx.maxdata_local = MAX_DATA;
        cnx.maxdata_remote = MAX_DATA;
    }

    test_ctx.loss_mask_default = init_loss_mask;
    test_api_init_send_recv_scenario(&mut test_ctx, MANY_LOSSES_Q_AND_R)?;
    let mut loss_mask = test_ctx.loss_mask_default;
    many_losses_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)
}

#[test]
fn many_losses() {
    for i in 0..6u32 {
        for j in 0..4u32 {
            let j_mask = if j == 0 { 0 } else { (1u64 << j) - 1 };
            let loss_mask = j_mask << i;
            many_losses_loss_test(loss_mask)
                .unwrap_or_else(|e| panic!("many_losses mask {i}-{j}={loss_mask:#x}: {e:?}"));
        }

        for j in 8u64..11 {
            let loss_mask = (j | (j << 4) | (j << 8)) << i;
            many_losses_loss_test(loss_mask)
                .unwrap_or_else(|e| panic!("many_losses mask {i},{j}={loss_mask:#x}: {e:?}"));
        }
    }

    let mut random_context = 0x1055_ca45_c001_babau64;
    for i in 0..50 {
        let mut loss_mask = 0u64;
        for _ in 0..64 {
            loss_mask <<= 1;
            if test_uniform_random(&mut random_context, 1000) < 300 {
                loss_mask |= 1;
            }
        }

        many_losses_q_and_r_scenario(loss_mask)
            .unwrap_or_else(|e| panic!("many_losses random mask {i}={loss_mask:#x}: {e:?}"));
    }
}

/// C: `many_short_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Scenario test with a large pseudo-random loss mask to exercise short-burst
/// packet loss recovery.
const MANY_SHORT_LOSS_MORE_STREAMS: &[TestApiStreamDesc] = &[
    TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 8,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 12,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 16,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 20,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 24,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 28,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 32,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 36,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 40,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 44,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 48,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 52,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 56,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 60,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 64,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 68,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
    TestApiStreamDesc {
        stream_id: 72,
        previous_stream_id: 0,
        q_len: 32,
        r_len: 633,
    },
];

fn many_short_loss_scenario() -> crate::Result<()> {
    const LOSS_MASK: u64 = 0x882818A881288848;
    const MAX_DATA: u64 = 16_000;
    const QUEUE_DELAY_MAX: u64 = 2_000;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    let mut handshake_loss_mask = 0u64;

    tls_api_connection_loop(
        &mut test_ctx,
        &mut handshake_loss_mask,
        QUEUE_DELAY_MAX,
        &mut simulated_time,
    )?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;
    if !test_ctx.has_cnx_server() {
        return Err(crate::Error::InvalidState);
    }

    {
        let cnx = test_ctx.cnx_client();
        cnx.maxdata_local = MAX_DATA;
        cnx.maxdata_remote = MAX_DATA;
    }
    {
        let cnx = test_ctx.cnx_server();
        cnx.maxdata_local = MAX_DATA;
        cnx.maxdata_remote = MAX_DATA;
    }

    test_ctx.loss_mask_default = LOSS_MASK;
    test_api_init_send_recv_scenario(&mut test_ctx, MANY_SHORT_LOSS_MORE_STREAMS)?;
    let mut loss_mask = test_ctx.loss_mask_default;
    many_losses_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 0)
}

#[test]
fn many_short_loss() {
    many_short_loss_scenario().expect("many_short_loss");
}

/// C: `migration_test` in `picoquictest/tls_api_test.c`.
///
/// Triggers a client path migration mid-transfer and verifies data completion.
#[test]
fn migration() {
    migration_test_scenario(TEST_SCENARIO_Q_AND_R, 0, false).expect("migration");
}

/// C: `migration_disabled_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that migration is rejected when the `migration_disabled`
/// transport parameter is set.
#[test]
fn migration_disabled() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).expect("migration_disabled context");
    test_ctx.qserver.default_tp.migration_disabled = true;

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
    .expect("migration_disabled q_and_r scenario");

    assert!(
        test_ctx.cnx_client().remote_parameters.migration_disabled,
        "client did not receive migration_disabled from server transport parameters"
    );
}

/// C: `migration_fail_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that a failed path challenge causes the migration to abort and
/// the connection to continue on the original path.
fn migration_fail_scenario() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).ok_or(crate::Error::Generic)?;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MIGRATION_FAIL_VERY_LONG)?;
    wait_client_connection_ready(&mut test_ctx, &mut simulated_time)?;

    let server_addr = test_ctx.server_addr;
    let mut bogus_addr = test_ctx.client_addr;
    bogus_addr.set_port(bogus_addr.port() + 1);
    test_ctx
        .cnx_client()
        .probe_new_path(&server_addr, &bogus_addr, simulated_time)?;

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 1_100_000)
}

#[test]
fn migration_fail() {
    migration_fail_scenario().expect("migration_fail");
}

/// C: `migration_test_long` in `picoquictest/tls_api_test.c`.
///
/// Migration test over a longer (very-long) data scenario.
#[test]
fn migration_long() {
    migration_test_scenario(TEST_SCENARIO_VERY_LONG, 0, false).expect("migration_long");
}

/// C: `migration_test_loss` in `picoquictest/tls_api_test.c`.
///
/// Migration test with a loss mask of 0x09 applied during the migration.
#[test]
fn migration_with_loss() {
    migration_test_scenario(TEST_SCENARIO_Q_AND_R, 0x09, false).expect("migration_with_loss");
}

/// C: `migration_zero_test` in `picoquictest/tls_api_test.c`.
///
/// Migration test with zero-length connection IDs.
#[test]
fn migration_zero() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];
    migration_test_scenario(&scenario, 0, true).expect("migration_zero");
}

/// C: `mtu_blocked_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery in "blocked" mode: the path actively blocks large packets,
/// so discovery falls back to the minimum MTU (1252).
#[test]
fn mtu_blocked() {
    let scenario = [TestApiStreamDesc {
        stream_id: 2,
        previous_stream_id: 0,
        q_len: 100_000,
        r_len: 0,
    }];
    mtu_discovery_test_one(PmtudPolicy::Blocked, 1252, 1252, &scenario, 0).expect("mtu_blocked");
}

/// C: `mtu_delayed_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery is initially blocked but eventually succeeds at the
/// maximum MTU (1440).
#[test]
fn mtu_delayed() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];
    mtu_discovery_test_one(PmtudPolicy::Delayed, 1252, 1440, &scenario, 0).expect("mtu_delayed");
}

/// C: `mtu_discovery_test` in `picoquictest/tls_api_test.c`.
///
/// Basic PMTUD: the path supports the full 1440-byte MTU.
#[test]
fn mtu_discovery() {
    let scenario = [TestApiStreamDesc {
        stream_id: 2,
        previous_stream_id: 0,
        q_len: 100_000,
        r_len: 0,
    }];
    mtu_discovery_test_one(PmtudPolicy::Basic, 1440, 1440, &scenario, 0).expect("mtu_discovery");
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
    let scenario = [TestApiStreamDesc {
        stream_id: 2,
        previous_stream_id: 0,
        q_len: 100_000,
        r_len: 0,
    }];
    mtu_discovery_test_one(PmtudPolicy::Basic, 1420, 1392, &scenario, 1420).expect("mtu_max");
}

/// C: `mtu_required_test` in `picoquictest/tls_api_test.c`.
///
/// MTU discovery in "required" mode: PMTUD is mandatory, connection aborts
/// if the minimum cannot be validated.
#[test]
fn mtu_required() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2_000,
    }];
    mtu_discovery_test_one(PmtudPolicy::Required, 1440, 1440, &scenario, 0).expect("mtu_required");
}

/// C: `multi_segment_test` in `picoquictest/tls_api_test.c`.
///
/// Tests multiple CC algorithms in sequence to validate the per-connection
/// CC selection API.
fn multi_segment_test_one(
    algo_id: &'static str,
    target_time: u64,
    send_buffer_size: usize,
) -> crate::Result<()> {
    const LATENCY_TARGET: u64 = 35_000;
    const PICOSEC_PER_BYTE: u64 = (1_000_000u64 * 8) / 100;
    const SCENARIO: [TestApiStreamDesc; 8] = [
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
    ];

    crate::register_all_congestion_control_algorithms();
    let cc_algo = crate::get_congestion_algorithm(algo_id).ok_or(crate::Error::Generic)?;

    let mut simulated_time = Instant::from_ticks(0);
    let mut initial_cid_bytes = [0x5e, 0x90, 0xe0, 0x40, 0, 6, 7, 8];
    initial_cid_bytes[4] = cc_algo.congestion_algorithm_number;
    let initial_cid =
        ConnectionId::clone_from_slice(&initial_cid_bytes).ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .ok_or(crate::Error::Generic)?;

    test_ctx.set_send_buffer_size(send_buffer_size);
    test_ctx.c_to_s_link.microsec_latency = LATENCY_TARGET;
    test_ctx.c_to_s_link.picosec_per_byte = PICOSEC_PER_BYTE;
    test_ctx.s_to_c_link.microsec_latency = LATENCY_TARGET;
    test_ctx.s_to_c_link.picosec_per_byte = PICOSEC_PER_BYTE;
    test_ctx.qserver.set_default_congestion_algorithm(cc_algo);
    test_ctx.qserver.set_qlog(".")?;
    test_ctx.qserver.use_long_log = true;

    let mut loss_mask = 0u64;
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        LATENCY_TARGET,
        &mut simulated_time,
    )?;
    if !test_ctx.has_cnx_server() {
        return Err(crate::Error::Generic);
    }
    let server_cc_number = test_ctx
        .cnx_server()
        .congestion_alg
        .map(|alg| alg.congestion_algorithm_number)
        .ok_or(crate::Error::Generic)?;
    if server_cc_number != cc_algo.congestion_algorithm_number {
        return Err(crate::Error::Generic);
    }

    test_api_init_send_recv_scenario(&mut test_ctx, &SCENARIO)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
}

#[test]
fn multi_segment() {
    for (algo_id, target_time) in [
        ("newreno", 1_220_000),
        ("cubic", 1_050_000),
        ("dcubic", 1_250_000),
        ("fastcc", 1_350_000),
        ("bbr", 1_280_000),
    ] {
        multi_segment_test_one(algo_id, target_time, 65_536)
            .unwrap_or_else(|e| panic!("multi_segment({algo_id}): {e:?}"));
    }
}

/// C: `tls_api_multiple_versions_test` in `picoquictest/tls_api_test.c`.
///
/// Runs the q-and-r data scenario for each supported QUIC version after
/// index 0 in the supported-version list.
#[test]
fn multiple_versions() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2_000,
    }];

    for version in SUPPORTED_VERSIONS.iter().skip(1).copied() {
        let version_code = version as u32;
        let mut t = Instant::from_ticks(0);
        let mut ctx = tls_api_init_ctx(&mut t, version_code, None)
            .unwrap_or_else(|| panic!("multiple_versions ver={version_code:#x}: ctx"));
        tls_api_one_scenario_body(&mut ctx, &mut t, &scenario, 0, 0, 0, 0, 0)
            .unwrap_or_else(|e| panic!("multiple_versions ver={version_code:#x}: {e:?}"));
    }
}

/// C: `nat_handshake_test` in `picoquictest/tls_api_test.c`.
///
/// Simulates a NAT rebinding that occurs during the Initial handshake and
/// verifies the connection completes.
fn nat_handshake_client_remote_cid_is_set(test_ctx: &mut TestTlsApiCtx) -> bool {
    test_ctx
        .qclient
        .first_cnx_mut()
        .and_then(|cnx| {
            let path = cnx.paths.first()?;
            let tuple = path.tuples.first()?;
            let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
            cnx.remote_connection_id_stashes
                .iter()
                .find(|stash| stash.unique_path_id == path.unique_path_id)
                .and_then(|stash| stash.connection_ids.get(cid_index))
                .map(|remote_cid| !remote_cid.connection_id.is_empty())
        })
        .unwrap_or(false)
}

fn nat_handshake_test_one(test_rank: usize) -> crate::Result<()> {
    const SCENARIO_Q2_AND_R2: [TestApiStreamDesc; 2] = [
        TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 257,
            r_len: 2_000,
        },
        TestApiStreamDesc {
            stream_id: 8,
            previous_stream_id: 0,
            q_len: 531,
            r_len: 11_000,
        },
    ];

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut nb_inactive = 0;
    let mut nb_trials = 0;
    let mut natted = 0;
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).ok_or(crate::Error::Memory)?;

    while nb_trials < 1024
        && nb_inactive < 512
        && (!test_ctx.client_ready() || !test_ctx.has_cnx_server() || !test_ctx.server_ready())
    {
        let mut was_active = false;
        nb_trials += 1;

        if natted == 0 {
            let should_nat = match test_rank {
                0 => nat_handshake_client_remote_cid_is_set(&mut test_ctx),
                1 => test_ctx
                    .qclient
                    .first_cnx_mut()
                    .map(|cnx| {
                        cnx.crypto_context[Epoch::OneRtt as usize]
                            .aead_decrypt
                            .is_some()
                    })
                    .unwrap_or(false),
                _ => false,
            };

            if should_nat {
                let mut natted_addr = test_ctx.client_addr;
                natted_addr.set_port(natted_addr.port() + 17);
                test_ctx.client_addr_natted = natted_addr;
                test_ctx.client_use_nat = true;
                natted += 1;
            }
        }

        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;

        let client_disconnected = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|cnx| cnx.connection_state == State::Disconnected)
            .unwrap_or(true);
        let server_disconnected = test_ctx.has_cnx_server()
            && test_ctx
                .qserver
                .first_cnx_mut()
                .map(|cnx| cnx.connection_state == State::Disconnected)
                .unwrap_or(false);
        if client_disconnected || server_disconnected {
            break;
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    test_api_init_send_recv_scenario(&mut test_ctx, &SCENARIO_Q2_AND_R2)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;

    if natted == 0 {
        return Err(crate::Error::Generic);
    }

    Ok(())
}

#[test]
fn nat_handshake() {
    for test_rank in 0..2 {
        nat_handshake_test_one(test_rank)
            .unwrap_or_else(|e| panic!("nat_handshake({test_rank}): {e:?}"));
    }
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
fn fast_nat_rebinding_test() -> crate::Result<()> {
    const NB_SWITCHES_REQUIRED: usize = 6;
    const MAX_TRIALS: usize = 1_000_000;
    const SCENARIO_SUSTAINED: [TestApiStreamDesc; 4] = [
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
        TestApiStreamDesc {
            stream_id: 12,
            previous_stream_id: 8,
            q_len: 257,
            r_len: 1_000_000,
        },
        TestApiStreamDesc {
            stream_id: 16,
            previous_stream_id: 12,
            q_len: 257,
            r_len: 1_000_000,
        },
    ];

    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid = ConnectionId::clone_from_slice(&[0xfa, 0x57, 0x08, 0xa7, 0, 0, 0, 0])
        .ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .ok_or(crate::Error::Memory)?;

    test_ctx.qserver.set_qlog(".")?;
    test_ctx.qclient.set_qlog(".")?;

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, &SCENARIO_SUSTAINED)?;

    let delta_t =
        5 * (test_ctx.c_to_s_link.microsec_latency + test_ctx.s_to_c_link.microsec_latency);
    let next_time = Instant::from_ticks(simulated_time.ticks().saturating_add(200_000_000));
    let mut nb_trials = 0usize;
    let mut nb_inactive = 0usize;
    let mut switch_time = simulated_time.ticks();
    let mut switched = false;
    let mut nb_switched = 0usize;

    let mut natted_addr = test_ctx.client_addr;
    natted_addr.set_port(natted_addr.port() + 17);
    test_ctx.client_addr_natted = natted_addr;
    test_ctx.client_use_nat = true;

    while nb_trials < MAX_TRIALS
        && nb_inactive < 256
        && simulated_time.ticks() < next_time.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let mut was_active = false;
        nb_trials += 1;

        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            next_time,
            &mut was_active,
        )?;

        let server_peer_port = if test_ctx.has_cnx_server() {
            let cnx = test_ctx.cnx_server();
            if cnx.connection_state == State::Ready {
                Some(cnx.path_peer_addr_by_index(0).port())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(peer_port) = server_peer_port
            && peer_port != 0
            && peer_port == test_ctx.client_addr_natted.port()
        {
            if switched {
                if simulated_time.ticks() > switch_time.saturating_add(delta_t)
                    && nb_switched < NB_SWITCHES_REQUIRED
                {
                    switched = false;
                }
            } else {
                let mut next_natted_addr = test_ctx.client_addr_natted;
                next_natted_addr.set_port(next_natted_addr.port() + 17);
                test_ctx.client_addr_natted = next_natted_addr;
                switched = true;
                switch_time = simulated_time.ticks();
                nb_switched += 1;
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
                .map(|cnx| cnx.is_backlog_empty())
                .unwrap_or(true);
            let server_empty = test_ctx
                .qserver
                .first_cnx_mut()
                .map(|cnx| cnx.is_backlog_empty())
                .unwrap_or(true);
            if client_empty && server_empty {
                break;
            }
        }
    }

    if nb_switched < NB_SWITCHES_REQUIRED {
        return Err(crate::Error::Generic);
    }

    tls_api_one_scenario_verify(&test_ctx)
}

#[test]
fn nat_rebinding_fast() {
    fast_nat_rebinding_test().expect("nat_rebinding_fast");
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
fn nat_rebinding_stress_backlogs_empty(test_ctx: &mut TestTlsApiCtx) -> bool {
    let client_empty = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.is_backlog_empty())
        .unwrap_or(true);
    let server_empty = test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| cnx.is_backlog_empty())
        .unwrap_or(true);
    client_empty && server_empty
}

fn nat_rebinding_stress_server_sequence(test_ctx: &mut TestTlsApiCtx) -> u64 {
    test_ctx
        .qserver
        .first_cnx_mut()
        .map(|cnx| cnx.pkt_ctx[PacketContext::Application as usize].send_sequence)
        .unwrap_or(0)
}

fn nat_rebinding_stress_test() -> crate::Result<()> {
    const MAX_TRIALS: usize = 10_000;
    const SCENARIO_VERY_LONG: [TestApiStreamDesc; 1] = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut nb_trials = 0usize;
    let mut nb_inactive = 0usize;
    let mut client_rebinding_done = false;
    let mut last_inject_time = 0u64;
    let mut random_context = 0xBABA_C001_CAFEu64;
    let mut last_client_packet_processed = 0u64;
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).ok_or(crate::Error::Memory)?;

    let mut hack_address = test_ctx.client_addr;
    hack_address.set_port(hack_address.port().wrapping_add(1023));
    let mut hack_address_random = test_ctx.client_addr;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, &SCENARIO_VERY_LONG)?;
    test_ctx.client_use_multiple_addresses = true;

    while nb_trials < MAX_TRIALS
        && nb_inactive < 256
        && test_ctx.client_ready()
        && test_ctx.server_ready()
    {
        let mut was_active = false;
        nb_trials += 1;

        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )?;

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if test_ctx.test_finished && nat_rebinding_stress_backlogs_empty(&mut test_ctx) {
            break;
        }

        let mut injected_packet = None;
        if let Some(packet) = test_ctx.c_to_s_link.packets.back() {
            let server_arrival = packet.arrival_time.ticks();
            if server_arrival > last_inject_time {
                let rand100 = test_uniform_random(&mut random_context, 100);
                last_inject_time = server_arrival;
                if rand100 < 9 {
                    let bad_address = if rand100 < 5 {
                        hack_address
                    } else {
                        hack_address_random
                            .set_port(test_uniform_random(&mut random_context, 0x10000) as u16);
                        hack_address_random
                    };
                    let addr_to = packet.addr_to.unwrap_or(test_ctx.server_addr);
                    let bytes = packet.bytes[..packet.length].to_vec();
                    injected_packet = Some((bytes, bad_address, addr_to));
                }
            }
        }
        if let Some((mut bytes, bad_address, addr_to)) = injected_packet {
            test_ctx.qserver.incoming_packet(
                bytes.as_mut_slice(),
                &bad_address,
                &addr_to,
                0,
                0,
                simulated_time,
            )?;
        }

        let server_sequence = nat_rebinding_stress_server_sequence(&mut test_ctx);
        if server_sequence > 256 {
            test_ctx.client_use_multiple_addresses = false;
        }

        if test_ctx.client_use_multiple_addresses
            && test_ctx.s_to_c_link.packets_sent != last_client_packet_processed
        {
            last_client_packet_processed = test_ctx.s_to_c_link.packets_sent;
            let client_addr = test_ctx.client_addr;
            let client_addr_natted = test_ctx.client_addr_natted;
            let client_use_nat = test_ctx.client_use_nat;
            if let Some(packet) = test_ctx.s_to_c_link.packets.back_mut() {
                let addr_to = packet.addr_to.unwrap_or(client_addr);
                if addr_to == hack_address {
                    packet.addr_to = Some(client_addr);
                } else if client_use_nat {
                    if addr_to == client_addr_natted {
                        packet.addr_to = Some(client_addr);
                    } else {
                        packet.length = 1;
                    }
                } else if addr_to != client_addr {
                    packet.length = 1;
                }
            }
        }

        if !client_rebinding_done && server_sequence > 128 {
            let mut natted_addr = test_ctx.client_addr;
            natted_addr.set_port(natted_addr.port().wrapping_add(17));
            test_ctx.client_addr_natted = natted_addr;
            test_ctx.client_use_nat = true;
            client_rebinding_done = true;
        }
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;
    tls_api_one_scenario_verify(&test_ctx)
}

#[test]
fn nat_rebinding_stress() {
    nat_rebinding_stress_test().expect("nat_rebinding_stress");
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
/// Computes three rounds of rotated application keys on both endpoints and
/// verifies cross-direction traffic secrets and AEAD compatibility.
fn new_rotated_key_error(code: u64) -> crate::Error {
    crate::Error::Protocol(0x5B00_0000 | code)
}

fn new_rotated_key_server_aead_ready(test_ctx: &mut TestTlsApiCtx) -> bool {
    test_ctx.has_cnx_server()
        && test_ctx.cnx_server().crypto_context[Epoch::OneRtt as usize]
            .aead_decrypt
            .is_some()
}

fn wait_application_aead_ready(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks().saturating_add(4_000_000));
    let mut nb_trials = 0usize;
    let mut nb_inactive = 0usize;

    while simulated_time.ticks() < time_out.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
        && !new_rotated_key_server_aead_ready(test_ctx)
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

    if test_ctx.has_cnx_server() && !new_rotated_key_server_aead_ready(test_ctx) {
        Err(new_rotated_key_error(1))
    } else {
        Ok(())
    }
}

fn new_rotated_key_aead_pair_check(
    encrypt: &dyn crate::tls::PacketKey,
    decrypt: &dyn crate::tls::PacketKey,
    packet_number: u64,
) -> crate::Result<()> {
    let header = [0x40, 0, 0, 0, 0];
    let plaintext = b"picoquic rotated key compatibility".to_vec();
    let mut protected = plaintext.clone();

    encrypt.encrypt(packet_number, &header, &mut protected);
    if protected.len() != plaintext.len().saturating_add(encrypt.tag_len()) {
        return Err(new_rotated_key_error(2));
    }

    decrypt.decrypt(packet_number, &header, &mut protected)?;
    if protected == plaintext {
        Ok(())
    } else {
        Err(new_rotated_key_error(3))
    }
}

fn new_rotated_key_round(test_ctx: &mut TestTlsApiCtx, round: u64) -> crate::Result<()> {
    {
        let server = test_ctx
            .qserver
            .first_cnx_mut()
            .ok_or_else(|| new_rotated_key_error(4))?;
        server.compute_new_rotated_keys()?;
    }
    {
        let client = test_ctx
            .qclient
            .first_cnx_mut()
            .ok_or_else(|| new_rotated_key_error(5))?;
        client.compute_new_rotated_keys()?;
    }

    let client = test_ctx
        .qclient
        .first_cnx_mut()
        .ok_or_else(|| new_rotated_key_error(6))?;
    let server = test_ctx
        .qserver
        .first_cnx_mut()
        .ok_or_else(|| new_rotated_key_error(7))?;

    let key_size = client.app_secret_size();
    if key_size != server.app_secret_size() {
        return Err(new_rotated_key_error(8));
    }

    let server_enc = server.app_secret(true).to_vec();
    let client_dec = client.app_secret(false).to_vec();
    if server_enc != client_dec {
        return Err(new_rotated_key_error(9));
    }

    let server_dec = server.app_secret(false).to_vec();
    let client_enc = client.app_secret(true).to_vec();
    if server_dec != client_enc {
        return Err(new_rotated_key_error(10));
    }

    let server_encrypt = server
        .crypto_context_new
        .aead_encrypt
        .as_deref()
        .ok_or_else(|| new_rotated_key_error(11))?;
    let client_decrypt = client
        .crypto_context_new
        .aead_decrypt
        .as_deref()
        .ok_or_else(|| new_rotated_key_error(12))?;
    new_rotated_key_aead_pair_check(server_encrypt, client_decrypt, round << 1)?;

    let client_encrypt = client
        .crypto_context_new
        .aead_encrypt
        .as_deref()
        .ok_or_else(|| new_rotated_key_error(13))?;
    let server_decrypt = server
        .crypto_context_new
        .aead_decrypt
        .as_deref()
        .ok_or_else(|| new_rotated_key_error(14))?;
    new_rotated_key_aead_pair_check(client_encrypt, server_decrypt, (round << 1) | 1)
}

fn new_rotated_key_clear_contexts(test_ctx: &mut TestTlsApiCtx) {
    if let Some(server) = test_ctx.qserver.first_cnx_mut() {
        server.crypto_context_new.free_handles();
    }
    if let Some(client) = test_ctx.qclient.first_cnx_mut() {
        client.crypto_context_new.free_handles();
    }
}

fn new_rotated_key_impl() -> crate::Result<()> {
    let mut loss_mask = 0u64;
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).ok_or(crate::Error::Memory)?;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    wait_application_aead_ready(&mut test_ctx, &mut simulated_time)?;

    for round in 1..=3 {
        let round_result = new_rotated_key_round(&mut test_ctx, round);
        new_rotated_key_clear_contexts(&mut test_ctx);
        round_result?;
    }

    Ok(())
}

#[test]
fn new_rotated_key() {
    new_rotated_key_impl().expect("new_rotated_key");
}

/// C: `no_ack_frequency_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection works correctly when the ACK-frequency
/// extension is not negotiated (classic ACK behaviour).
#[test]
fn no_ack_frequency() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    for i in 1..=3 {
        let mut client_parameters = TransportParameters::default();
        let mut server_parameters = TransportParameters::default();
        crate::internal::init_transport_parameters(&mut client_parameters);
        crate::internal::init_transport_parameters(&mut server_parameters);

        client_parameters.min_ack_delay = Duration::from_ticks(if i & 1 == 1 { 0 } else { 1000 });
        server_parameters.enable_loss_bit = if i > 1 { 0 } else { 1 };

        let mut simulated_time = Instant::from_ticks(0);
        let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, 0, None, None)
            .unwrap_or_else(|| panic!("no_ack_frequency({i}): ctx"));
        test_ctx
            .cnx_client()
            .set_transport_parameters(&client_parameters);
        test_ctx
            .qserver
            .set_default_tp(&server_parameters)
            .unwrap_or_else(|e| panic!("no_ack_frequency({i}): server tp: {e:?}"));

        tls_api_one_scenario_body(
            &mut test_ctx,
            &mut simulated_time,
            &scenario,
            128,
            0,
            0,
            0,
            2_000_000,
        )
        .unwrap_or_else(|e| panic!("no_ack_frequency({i}): {e:?}"));
    }
}

/// C: `not_before_cnxid_test` in `picoquictest/tls_api_test.c`.
///
/// Tests the `not_before_sequence` field in NEW_CONNECTION_ID frames,
/// which prevents the peer from using old CIDs.
fn not_before_cnxid_error(code: u64) -> crate::Error {
    crate::Error::Protocol(0x5B10_0000 | code)
}

fn not_before_cnxid_local_count(cnx: &Connection) -> usize {
    cnx.local_connection_id_lists
        .first()
        .map(|list| list.connection_ids.len())
        .unwrap_or(0)
}

fn not_before_cnxid_stash_count(cnx: &Connection) -> usize {
    cnx.remote_connection_id_stashes
        .first()
        .map(|stash| stash.connection_ids.len())
        .unwrap_or(0)
}

fn not_before_cnxid_sequence_next(cnx: &Connection) -> Option<u64> {
    cnx.local_connection_id_lists
        .first()
        .map(|list| list.local_connection_id_sequence_next)
}

fn not_before_cnxid_ready(test_ctx: &mut TestTlsApiCtx) -> bool {
    let Some(client) = test_ctx.qclient.first_cnx_mut() else {
        return false;
    };
    let client_local_count = not_before_cnxid_local_count(client);
    let client_stash_count = not_before_cnxid_stash_count(client);
    let client_has_misc_frames = client.has_misc_frames();
    let client_backlog_empty = client.is_backlog_empty();

    let Some(server) = test_ctx.qserver.first_cnx_mut() else {
        return false;
    };
    let server_local_count = not_before_cnxid_local_count(server);
    let server_stash_count = not_before_cnxid_stash_count(server);
    let server_backlog_empty = server.is_backlog_empty();

    client_local_count >= NB_PATH_TARGET
        && server_local_count >= NB_PATH_TARGET
        && !client_has_misc_frames
        && client_stash_count >= NB_PATH_TARGET - 1
        && server_stash_count >= NB_PATH_TARGET - 1
        && client_backlog_empty
        && server_backlog_empty
}

fn not_before_cnxid_test_stash(
    cnx: &Connection,
    peer: &Connection,
    error_code: u64,
) -> crate::Result<()> {
    let stash = cnx
        .remote_connection_id_stashes
        .first()
        .ok_or_else(|| not_before_cnxid_error(error_code))?;
    let peer_list = peer
        .local_connection_id_lists
        .first()
        .ok_or_else(|| not_before_cnxid_error(error_code + 1))?;

    if stash.connection_ids.len() != peer_list.connection_ids.len() {
        return Err(not_before_cnxid_error(error_code + 2));
    }

    for (remote_cid, local_token) in stash.connection_ids.iter().zip(&peer_list.connection_ids) {
        let local_cid = peer
            .local_connection_ids
            .get(*local_token)
            .ok_or_else(|| not_before_cnxid_error(error_code + 3))?;
        if remote_cid.connection_id != local_cid.connection_id {
            return Err(not_before_cnxid_error(error_code + 4));
        }
    }

    Ok(())
}

fn not_before_cnxid_impl() -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).ok_or(crate::Error::Memory)?;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        NB_PATH_TARGET as i32,
        0,
    )?;

    let not_before = {
        let server = test_ctx
            .qserver
            .first_cnx_mut()
            .ok_or_else(|| not_before_cnxid_error(1))?;
        not_before_cnxid_sequence_next(server)
            .ok_or_else(|| not_before_cnxid_error(2))?
            .saturating_sub(1)
    };

    let transport_error = {
        let client = test_ctx
            .qclient
            .first_cnx_mut()
            .ok_or_else(|| not_before_cnxid_error(3))?;
        client.remove_not_before_cid(0, not_before, simulated_time)
    };
    if transport_error != 0 {
        return Err(not_before_cnxid_error(4));
    }

    let time_out = Instant::from_ticks(simulated_time.ticks().saturating_add(8_000_000));
    let mut nb_rounds = 0usize;
    while simulated_time.ticks() < time_out.ticks()
        && nb_rounds < 2048
        && test_ctx
            .qclient
            .first_cnx_mut()
            .map(|cnx| cnx.connection_state != State::Disconnected)
            .unwrap_or(false)
    {
        let mut was_active = false;
        let round_result = tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            time_out,
            &mut was_active,
        );
        nb_rounds += 1;
        if let Err(error) = round_result
            && nb_rounds != 30
        {
            return Err(error);
        }

        if not_before_cnxid_ready(&mut test_ctx) {
            break;
        }
    }

    let server_local_count = {
        let server = test_ctx
            .qserver
            .first_cnx_mut()
            .ok_or_else(|| not_before_cnxid_error(5))?;
        not_before_cnxid_local_count(server)
    };
    if server_local_count != NB_PATH_TARGET {
        return Err(not_before_cnxid_error(6));
    }

    let client = test_ctx
        .qclient
        .first_cnx_mut()
        .ok_or_else(|| not_before_cnxid_error(7))?;
    let server = test_ctx
        .qserver
        .first_cnx_mut()
        .ok_or_else(|| not_before_cnxid_error(8))?;
    not_before_cnxid_test_stash(client, server, 10)?;
    not_before_cnxid_test_stash(server, client, 20)
}

#[test]
fn not_before_cnxid() {
    not_before_cnxid_impl().expect("not_before_cnxid");
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
    pacing_update_impl().expect("pacing_update");
}

#[derive(Clone, Copy)]
struct PacingRateRow {
    time: u64,
    callback_rate: u64,
    pacing_rate: u64,
    cwin: u64,
    rtt: u64,
}

struct PacingUpdateCallback {
    rows: Rc<RefCell<Vec<PacingRateRow>>>,
}

impl StreamDataCallback for PacingUpdateCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        _bytes: &[u8],
        fin_or_event: CallbackEvent,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        if fin_or_event == CallbackEvent::PacingChanged {
            self.rows.borrow_mut().push(PacingRateRow {
                time: connection.quic_time().ticks(),
                callback_rate: stream_id,
                pacing_rate: connection.pacing_rate(),
                cwin: connection.cwin(),
                rtt: connection.rtt(),
            });
        }
        0
    }
}

fn pacing_update_impl() -> crate::Result<()> {
    const PACING_RATE_CSV: &str = "pacing_rate.csv";
    const PACING_RATE_REF: &str = "picoquictest/pacing_rate_ref.txt";
    const SCENARIO_Q_AND_R: [TestApiStreamDesc; 1] = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    let mut simulated_time = Instant::from_ticks(0);
    let rows = Rc::new(RefCell::new(Vec::new()));
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).ok_or(crate::Error::Generic)?;
    std::fs::write(
        PACING_RATE_CSV,
        "Time, Pacing_rate_CB, Pacing_rate, CWIN, RTT\n",
    )
    .map_err(|_| crate::Error::Generic)?;

    test_ctx
        .cnx_client()
        .set_callback(Some(Box::new(PacingUpdateCallback {
            rows: Rc::clone(&rows),
        })));
    test_ctx
        .cnx_client()
        .subscribe_pacing_rate_updates(0x8000, 0x10000);

    tls_api_one_scenario_body_ex(
        &mut test_ctx,
        &mut simulated_time,
        &SCENARIO_Q_AND_R,
        1_000_000,
        0,
        0,
        20_000,
        3_600_000,
        &[],
    )?;

    let mut csv = String::from("Time, Pacing_rate_CB, Pacing_rate, CWIN, RTT\n");
    for row in rows.borrow().iter() {
        writeln!(
            csv,
            "{}, {}, {}, {}, {}",
            row.time, row.callback_rate, row.pacing_rate, row.cwin, row.rtt
        )
        .map_err(|_| crate::Error::Generic)?;
    }
    std::fs::write(PACING_RATE_CSV, csv).map_err(|_| crate::Error::Generic)?;
    compare_text_files(PACING_RATE_CSV, PACING_RATE_REF)
}

const PACKET_TRACE_BIN: &str = "ace1020304050607.server.log";
const PACKET_TRACE_CSV: &str = "packet_trace.csv";
const PACKET_TRACE_REF: &str = "picoquictest/packet_trace_ref.txt";

fn packet_trace_read_varint_or_zero(s: &mut ByteStream<'_>) -> u64 {
    s.read_varint().unwrap_or(0)
}

fn packet_trace_cc_log_file_to_csv(
    bin_cc_log_name: &str,
    csv_cc_log_name: &str,
) -> crate::Result<()> {
    let mut f_binlog = File::open(bin_cc_log_name).map_err(|_| crate::Error::Generic)?;
    let mut header = [0u8; 16];
    f_binlog
        .read_exact(&mut header)
        .map_err(|_| crate::Error::Generic)?;
    let mut header_stream = ByteStream::from_slice(&mut header);
    let magic = header_stream.read_u32()?;
    if magic != crate::fourcc(b'q', b'l', b'o', b'g') {
        return Err(crate::Error::Generic);
    }
    let _flags = header_stream.read_u16()?;
    let version = header_stream.read_u16()?;
    if version != 1 {
        return Err(crate::Error::Generic);
    }
    let _log_time = header_stream.read_u64()?;
    f_binlog
        .seek(SeekFrom::Start(16))
        .map_err(|_| crate::Error::Generic)?;

    let mut csv = String::from(
        "time, path, sequence, highest ack, high ack time, last time ack, cwin, \
         one-way-delay, rtt-sample, SRTT, RTT min, Bandwidth (B/s), Receive rate (B/s), \
         Send MTU, pacing packet time(us), nb retrans, nb spurious, cwin blkd, flow blkd, \
         stream blkd, app limited, cc_state, cc_param, bw_max, transit, \n",
    );
    let mut idx = 0usize;
    let mut starttime = 0u64;

    loop {
        let mut head = [0u8; 4];
        match f_binlog.read_exact(&mut head) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => return Err(crate::Error::Generic),
        }
        let len = u32::from_be_bytes(head) as usize;
        if len > BYTESTREAM_MAX_BUFFER_SIZE {
            return Err(crate::Error::Generic);
        }
        let mut buf = vec![0u8; len];
        f_binlog
            .read_exact(&mut buf)
            .map_err(|_| crate::Error::Generic)?;
        let mut s = ByteStream::from_slice(&mut buf);

        let _cid = s.read_cid()?;
        let mut time = s.read_varint()?;
        let path_id = s.read_varint()?;
        let id = s.read_varint()?;

        if idx == 0 {
            starttime = time;
        }
        idx += 1;
        time = time.saturating_sub(starttime);

        if id == 0x0038 {
            let sequence = s.read_varint()?;
            let packet_rcvd = s.read_varint()?;
            let mut highest_ack = u64::MAX;
            let mut high_ack_time = 0;
            let mut last_time_ack = 0;
            if packet_rcvd != 0 {
                highest_ack = s.read_varint()?;
                high_ack_time = s.read_varint()?;
                last_time_ack = s.read_varint()?;
            }
            let cwin = s.read_varint()?;
            let one_way_delay = s.read_varint()?;
            let rtt_sample = s.read_varint()?;
            let srtt = s.read_varint()?;
            let rtt_min = s.read_varint()?;
            let bandwidth_estimate = s.read_varint()?;
            let receive_rate_estimate = s.read_varint()?;
            let send_mtu = s.read_varint()?;
            let pacing_packet_time = s.read_varint()?;
            let nb_retrans = s.read_varint()?;
            let nb_spurious = s.read_varint()?;
            let cwin_blkd = s.read_varint()?;
            let flow_blkd = s.read_varint()?;
            let stream_blkd = s.read_varint()?;
            let cc_state = packet_trace_read_varint_or_zero(&mut s);
            let cc_param = packet_trace_read_varint_or_zero(&mut s);
            let bw_max = packet_trace_read_varint_or_zero(&mut s);
            let bytes_in_transit = packet_trace_read_varint_or_zero(&mut s);
            let app_limited = packet_trace_read_varint_or_zero(&mut s);

            writeln!(
                csv,
                "{time}, {path_id}, {sequence}, {}, {high_ack_time}, {last_time_ack}, \
                 {cwin}, {one_way_delay}, {rtt_sample}, {srtt}, {rtt_min}, \
                 {bandwidth_estimate}, {receive_rate_estimate}, {send_mtu}, \
                 {pacing_packet_time}, {nb_retrans}, {nb_spurious}, {cwin_blkd}, \
                 {flow_blkd}, {stream_blkd}, {app_limited}, {cc_state}, {cc_param}, \
                 {bw_max}, {bytes_in_transit},",
                highest_ack as i64,
            )
            .map_err(|_| crate::Error::Generic)?;
        }
    }

    let mut f_csvlog = File::create(csv_cc_log_name).map_err(|_| crate::Error::Generic)?;
    f_csvlog
        .write_all(csv.as_bytes())
        .map_err(|_| crate::Error::Generic)
}

fn packet_trace_impl() -> crate::Result<()> {
    const SCENARIO_VERY_LONG: [TestApiStreamDesc; 1] = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];

    let _ = std::fs::remove_file(PACKET_TRACE_BIN);
    let _ = std::fs::remove_file(PACKET_TRACE_CSV);

    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid = ConnectionId::clone_from_slice(&[0xac, 0xe1, 2, 3, 4, 5, 6, 7])
        .ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .ok_or(crate::Error::Generic)?;

    test_ctx.qserver.set_binlog(Some("."))?;
    test_ctx
        .qserver
        .set_default_lossbit_policy(LossbitVersion::SendReceive);
    test_ctx
        .qclient
        .set_default_lossbit_policy(LossbitVersion::SendReceive);
    test_ctx.qserver.use_long_log = true;

    tls_api_one_scenario_body(
        &mut test_ctx,
        &mut simulated_time,
        &SCENARIO_VERY_LONG,
        0,
        0,
        0,
        20_000,
        1_000_000,
    )?;
    drop(test_ctx);

    packet_trace_cc_log_file_to_csv(PACKET_TRACE_BIN, PACKET_TRACE_CSV)?;
    compare_text_files(PACKET_TRACE_CSV, PACKET_TRACE_REF)
}

/// C: `packet_trace_test` in `picoquictest/tls_api_test.c`.
///
/// Runs the very-long scenario with server binlog/lossbit logging enabled,
/// converts the binary CC log to CSV, and compares it to the reference trace.
#[test]
fn packet_trace() {
    packet_trace_impl().expect("packet_trace");
}

/// C: `padding_null_test` in `picoquictest/tls_api_test.c`.
///
/// Padding test with both `padding_multiple` and `padding_min_size` = 0
/// (no padding).
fn padding_null_error(code: u64) -> crate::Error {
    crate::Error::Protocol(0x5B02_0000 | code)
}

fn padding_test_predict_pn_length(cnx: &Connection) -> usize {
    let pkt_ctx = &cnx.pkt_ctx[PacketContext::Application as usize];
    let mut pn_l = 4usize;
    let mut delta = if pkt_ctx.send_sequence == 0 {
        0i128
    } else {
        pkt_ctx.send_sequence.saturating_sub(1) as i128
    };

    if let Some(first_pending) = pkt_ctx.pending.keys().next().copied() {
        delta -= first_pending as i128;
    }

    if delta < 262_144 {
        pn_l = 3;
        if pkt_ctx.send_sequence < 1024 {
            pn_l = 2;
            if pkt_ctx.send_sequence < 16 {
                pn_l = 1;
            }
        }
    }

    pn_l
}

fn padding_null_check_packet_length(
    test_ctx: &mut TestTlsApiCtx,
    test_size: usize,
    length: usize,
) -> crate::Result<()> {
    let client = test_ctx.cnx_client();
    let checksum_length = client.get_checksum_length(Epoch::OneRtt);
    let pn_iv_length = client.crypto_context[Epoch::OneRtt as usize]
        .pn_enc
        .as_deref()
        .map(crate::tls_api::pn_iv_size)
        .ok_or_else(|| padding_null_error(1))?;
    let pn_length = padding_test_predict_pn_length(client);
    let header_length = client.predict_packet_header_length_for_pc(
        PacketType::OneRttProtected,
        PacketContext::Application,
    );
    let pn_offset = header_length
        .checked_sub(pn_length)
        .ok_or_else(|| padding_null_error(2))?;
    let raw_length = header_length
        .checked_add(test_size)
        .ok_or_else(|| padding_null_error(3))?;

    if pn_length == 1 && raw_length + checksum_length > length {
        Err(padding_null_error(0x1000 | test_size as u64))
    } else if pn_offset + 4 + pn_iv_length > length {
        Err(padding_null_error(0x2000 | test_size as u64))
    } else if raw_length + checksum_length + 6 < length && pn_offset + 4 + pn_iv_length != length {
        Err(padding_null_error(0x3000 | test_size as u64))
    } else {
        Ok(())
    }
}

fn padding_null_test_one() -> crate::Result<()> {
    const TEST_SIZES: [usize; 15] = [1, 2, 3, 5, 8, 13, 21, 44, 65, 109, 174, 283, 457, 740, 1023];

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, V1, None).ok_or_else(|| padding_null_error(4))?;
    test_ctx.qserver.set_default_padding(0, 0);
    test_ctx.cnx_client().set_padding_policy(0, 0);

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        let client_state = test_ctx
            .qclient
            .first_cnx_mut()
            .map(|cnx| cnx.connection_state as u64)
            .unwrap_or(0xff);
        let server_state = test_ctx
            .qserver
            .first_cnx_mut()
            .map(|cnx| cnx.connection_state as u64)
            .unwrap_or(0xff);
        return Err(padding_null_error(
            0x5000 | (client_state << 8) | server_state,
        ));
    }

    for test_size in TEST_SIZES {
        let mut data = vec![crate::frames::FrameType::Padding as u8; test_size];
        data[test_size - 1] = crate::frames::FrameType::Ping as u8;

        let mut nb_trials = 0;
        let mut nb_inactive = 0;
        let mut is_queued = false;
        let mut is_success = false;

        while nb_trials < 256
            && nb_inactive < 256
            && test_ctx.client_ready()
            && test_ctx.server_ready()
        {
            let mut was_active = false;
            nb_trials += 1;

            let client_empty = test_ctx.cnx_client().is_cnx_backlog_empty();
            let server_empty = test_ctx.cnx_server().is_cnx_backlog_empty();
            let links_empty =
                test_ctx.c_to_s_link.packets.is_empty() && test_ctx.s_to_c_link.packets.is_empty();

            if client_empty && server_empty && links_empty {
                if !is_queued {
                    test_ctx.cnx_client().queue_misc_frame(
                        &data,
                        false,
                        PacketContext::Application,
                    )?;
                    is_queued = true;
                } else {
                    is_success = true;
                    break;
                }
            }

            tls_api_one_sim_round(
                &mut test_ctx,
                &mut simulated_time,
                Instant::from_ticks(0),
                &mut was_active,
            )?;

            if is_queued
                && let Some(length) = test_ctx
                    .c_to_s_link
                    .packets
                    .front()
                    .map(|packet| packet.length)
            {
                padding_null_check_packet_length(&mut test_ctx, test_size, length)?;
            }

            if was_active {
                nb_inactive = 0;
            } else {
                nb_inactive += 1;
            }
        }

        if !is_success {
            return Err(padding_null_error(0x4000 | test_size as u64));
        }
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

#[test]
fn padding_null() {
    padding_null_test_one().expect("padding_null");
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

/// C: `perflog_test` Win32 branch in `picoquictest/tls_api_test.c`.
///
/// The mapped C body is a platform guard that returns success without running
/// the performance-log scenario.
#[test]
fn perflog() {}

/// C: `pn_enc_1rtt_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies the 1-RTT packet-number encryption round-trip (encode → transmit
/// → decode).
#[test]
fn pn_enc_1rtt() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("pn_enc_1rtt connection");
    wait_application_aead_ready(&mut test_ctx, &mut simulated_time)
        .expect("pn_enc_1rtt application aead");

    let seq_num_1: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
    let sample_1: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17,
        0x2a,
    ];
    let seq_num_2: [u8; 4] = [0xba, 0xba, 0xc0, 0x00];
    let sample_2: [u8; 16] = [
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a, 0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f,
        0x96,
    ];

    let epoch = Epoch::OneRtt as usize;
    let (qclient, qserver) = (&mut test_ctx.qclient, &mut test_ctx.qserver);
    let client = qclient.first_cnx_mut().expect("client connection");
    let server = qserver.first_cnx_mut().expect("server connection");

    for _i in [1, 2] {
        test_one_pn_enc_pair(
            &seq_num_1,
            client.crypto_context[epoch]
                .pn_enc
                .as_deref()
                .expect("client 1-RTT pn_enc"),
            server.crypto_context[epoch]
                .pn_dec
                .as_deref()
                .expect("server 1-RTT pn_dec"),
            &sample_1,
        );
        test_one_pn_enc_pair(
            &seq_num_2,
            server.crypto_context[epoch]
                .pn_enc
                .as_deref()
                .expect("server 1-RTT pn_enc"),
            client.crypto_context[epoch]
                .pn_dec
                .as_deref()
                .expect("client 1-RTT pn_dec"),
            &sample_2,
        );
    }
}

/// C: `wait_application_aead_ready` in `picoquictest/tls_api_test.c`.
fn wait_application_aead_ready(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
) -> crate::Result<()> {
    let time_out = Instant::from_ticks(simulated_time.ticks() + 4_000_000);
    let mut nb_trials = 0;
    let mut nb_inactive = 0;

    while simulated_time.ticks() < time_out.ticks()
        && test_ctx.client_ready()
        && test_ctx.server_ready()
        && !server_application_aead_ready(test_ctx)
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

    if server_application_aead_ready(test_ctx) {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

fn server_application_aead_ready(test_ctx: &mut TestTlsApiCtx) -> bool {
    test_ctx
        .qserver
        .first_cnx_mut()
        .map(|c| {
            c.crypto_context[Epoch::OneRtt as usize]
                .aead_decrypt
                .is_some()
        })
        .unwrap_or(false)
}

/// C: `test_one_pn_enc_pair` in `picoquictest/cleartext_aead_test.c`.
fn test_one_pn_enc_pair(
    seqnum: &[u8],
    pn_enc: &dyn crate::tls::HeaderKey,
    pn_dec: &dyn crate::tls::HeaderKey,
    sample: &[u8; 16],
) {
    let enc_mask = pn_enc.mask(*sample);
    let mut encoded = [0u8; 16];
    for (out, (pn, mask)) in encoded.iter_mut().zip(seqnum.iter().zip(enc_mask.iter())) {
        *out = pn ^ mask;
    }

    let dec_mask = pn_dec.mask(*sample);
    let mut decoded = [0u8; 16];
    for (out, (pn, mask)) in decoded.iter_mut().zip(encoded.iter().zip(dec_mask.iter())) {
        *out = pn ^ mask;
    }

    assert_eq!(
        &decoded[..seqnum.len()],
        seqnum,
        "PN enc/dec roundtrip failed"
    );
}

fn pn_random_check_sequence(cnx: &Connection, cnx_name: &str, randomize_all: bool) {
    for pc in 0..NB_PACKET_CONTEXT {
        let send_sequence = cnx.pkt_ctx[pc].send_sequence;
        if randomize_all || pc == PacketContext::Initial as usize {
            assert!(
                send_sequence >= PN_RANDOM_MIN as u64,
                "{cnx_name} packet context {pc} sequence {send_sequence} below random minimum"
            );
        } else {
            assert!(
                send_sequence < PN_RANDOM_MIN as u64,
                "{cnx_name} packet context {pc} sequence {send_sequence} unexpectedly randomized"
            );
        }
    }
}

fn pn_random_test_one(randomize_all: bool) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid = ConnectionId::clone_from_slice(&[0xff, 0x12, 0x34, 0, 0, 0, 0, 0])
        .ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .ok_or(crate::Error::Generic)?;

    let random_initial = if randomize_all { 2 } else { 1 };
    test_ctx.qclient.set_random_initial(random_initial);
    test_ctx.qserver.set_random_initial(random_initial);
    test_ctx.qserver.set_log_level(1);

    let first_pc = if randomize_all {
        PacketContext::Application as usize
    } else {
        PacketContext::Initial as usize
    };
    {
        let cnx_client = test_ctx.cnx_client();
        for pc in first_pc..NB_PACKET_CONTEXT {
            cnx_client.pkt_ctx[pc].send_sequence = PN_RANDOM_MIN as u64 + 17 + pc as u64;
        }
    }

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        return Err(crate::Error::Generic);
    }
    pn_random_check_sequence(test_ctx.cnx_client(), "client", randomize_all);
    pn_random_check_sequence(test_ctx.cnx_server(), "server", randomize_all);

    let scenario_q_and_r = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario_q_and_r)?;
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 1_000_000)
}

/// C: `pn_random_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that initial packet numbers are randomised as required by
/// RFC 9000 §12.3.
#[test]
fn pn_random() {
    pn_random_test_one(false).expect("pn_random initial-only");
    pn_random_test_one(true).expect("pn_random all packet number spaces");
}

fn port_blocked_is_retry(send_buffer: &[u8], send_length: usize) -> bool {
    if send_length < 5 || (send_buffer[0] & 0x80) == 0 {
        return false;
    }

    let packet_version = u32::from_be_bytes([
        send_buffer[1],
        send_buffer[2],
        send_buffer[3],
        send_buffer[4],
    ]);
    let Some(version_index) = SUPPORTED_VERSIONS
        .iter()
        .position(|version| *version as u32 == packet_version)
    else {
        return false;
    };

    parse_long_packet_type(send_buffer[0], version_index as i32) == PacketType::Retry
}

fn port_blocked_test_one(
    qserver: &mut crate::internal::Quic,
    packet: &mut [u8],
    addr_from: SocketAddr,
    addr_to: SocketAddr,
    expect_blocked: bool,
    retry_accepted: bool,
    current_time: Instant,
    label: &str,
) -> crate::Result<()> {
    qserver.incoming_packet_ex(packet, &addr_from, &addr_to, 0, 0, current_time)?;

    let mut send_buffer = [0u8; MAX_PACKET_SIZE];
    let send_length = qserver
        .prepare_next_packet_ex(current_time, &mut send_buffer)?
        .send_length;

    if expect_blocked {
        assert!(
            send_length == 0
                || (retry_accepted && port_blocked_is_retry(&send_buffer, send_length)),
            "{label}: server sent {send_length} bytes to blocked source {addr_from}"
        );
    } else {
        assert!(
            send_length > 0,
            "{label}: server did not respond to unblocked source {addr_from}"
        );
    }

    Ok(())
}

fn port_blocked_prepare_initial_packet(
    test_ctx: &mut TestTlsApiCtx,
    current_time: Instant,
    send_buffer: &mut [u8; MAX_PACKET_SIZE],
) -> crate::Result<usize> {
    test_ctx.cnx_client().initialize_tls_stream(current_time)?;

    let cnx = test_ctx.cnx_client();
    let crypto_data = cnx.tls_stream[Epoch::Initial as usize]
        .send_queue
        .front()
        .map(|node| node.bytes.clone())
        .ok_or(crate::Error::Generic)?;
    let sequence_number = cnx.pkt_ctx[PacketContext::Initial as usize].send_sequence;
    let header_length =
        cnx.predict_packet_header_length_for_pc(PacketType::Initial, PacketContext::Initial);

    let mut pn_offset = 0usize;
    let mut pn_length = 0usize;
    let actual_header_length = cnx.create_packet_header_at(
        PacketType::Initial,
        sequence_number,
        0,
        0,
        header_length,
        send_buffer,
        &mut pn_offset,
        &mut pn_length,
    );
    if actual_header_length != header_length || pn_length == 0 {
        return Err(crate::Error::Generic);
    }

    let mut cleartext_length = header_length;
    send_buffer[cleartext_length] = crate::frames::FrameType::CryptoHs as u8;
    cleartext_length += 1;
    cleartext_length += varint_encode(&mut send_buffer[cleartext_length..], 0);
    cleartext_length += varint_encode(
        &mut send_buffer[cleartext_length..],
        crypto_data.len() as u64,
    );
    let crypto_end = cleartext_length + crypto_data.len();
    if crypto_end > send_buffer.len() {
        return Err(crate::Error::BufferTooSmall);
    }
    send_buffer[cleartext_length..crypto_end].copy_from_slice(&crypto_data);
    cleartext_length = crypto_end;

    let checksum_length = cnx.get_checksum_length(Epoch::Initial);
    let padded_length = ENFORCED_INITIAL_MTU.saturating_sub(checksum_length);
    if cleartext_length < padded_length {
        send_buffer[cleartext_length..padded_length].fill(0);
        cleartext_length = padded_length;
    }

    let header = send_buffer[..header_length].to_vec();
    let mut payload = send_buffer[header_length..cleartext_length].to_vec();
    let aead = cnx.crypto_context[Epoch::Initial as usize]
        .aead_encrypt
        .as_deref()
        .ok_or(crate::Error::Tls)?;
    let pn_enc = cnx.crypto_context[Epoch::Initial as usize]
        .pn_enc
        .as_deref()
        .ok_or(crate::Error::Tls)?;
    aead.encrypt(sequence_number, &header, &mut payload);

    let send_length = header_length + payload.len();
    if send_length > send_buffer.len() {
        return Err(crate::Error::BufferTooSmall);
    }
    send_buffer[..header_length].copy_from_slice(&header);
    send_buffer[header_length..send_length].copy_from_slice(&payload);
    update_payload_length(send_buffer, pn_offset, pn_offset, send_length);
    protect_packet_header(&mut send_buffer[..send_length], pn_offset, 0x0f, pn_enc);

    Ok(send_length)
}

fn port_blocked_test_address(
    addr_from: SocketAddr,
    addr_to: SocketAddr,
    expect_blocked: bool,
    do_disable: bool,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid = ConnectionId::clone_from_slice(&[0x50, 0x0b, 0x10, 0xc0, 0, 0, 0, 0])
        .ok_or(crate::Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .ok_or(crate::Error::Generic)?;
    test_ctx.qserver.set_port_blocking_disabled(do_disable);

    let mut vn_probe = [0xaa; ENFORCED_INITIAL_MTU];
    vn_probe[1..5].copy_from_slice(&[0xa1, 0xa2, 0xa3, 0xa4]);
    vn_probe[5] = 8;
    vn_probe[14] = 8;
    port_blocked_test_one(
        &mut test_ctx.qserver,
        &mut vn_probe,
        addr_from,
        addr_to,
        expect_blocked,
        false,
        simulated_time,
        "version negotiation",
    )?;

    simulated_time = Instant::from_ticks(simulated_time.ticks() + 1_000);
    let mut one_rtt_probe = [0xbb; ENFORCED_INITIAL_MTU];
    one_rtt_probe[0] = 0x7f;
    port_blocked_test_one(
        &mut test_ctx.qserver,
        &mut one_rtt_probe,
        addr_from,
        addr_to,
        expect_blocked,
        false,
        simulated_time,
        "unexpected one-rtt",
    )?;

    simulated_time = Instant::from_ticks(simulated_time.ticks() + 1_000);
    let mut client_packet = [0u8; MAX_PACKET_SIZE];
    let client_send_length =
        port_blocked_prepare_initial_packet(&mut test_ctx, simulated_time, &mut client_packet)?;
    port_blocked_test_one(
        &mut test_ctx.qserver,
        &mut client_packet[..client_send_length],
        addr_from,
        addr_to,
        expect_blocked,
        true,
        simulated_time,
        "initial",
    )
}

fn port_blocked_test_port(port: u16, expect_blocked: bool) -> crate::Result<()> {
    let addresses = [
        (
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), port),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 961),
            "IPv4",
        ),
        (
            SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0x2001, 2, 3, 0, 0, 0, 0, 4)), port),
            SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0x2002, 3, 0, 0, 0, 0, 0, 4)), 961),
            "IPv6",
        ),
    ];

    for (addr_from, addr_to, family) in addresses {
        for do_disable in [false, true] {
            let actually_blocked = expect_blocked && !do_disable;
            port_blocked_test_address(addr_from, addr_to, actually_blocked, do_disable)
                .unwrap_or_else(|err| {
                    panic!("port_blocked port={port} family={family} disable={do_disable}: {err:?}")
                });
        }
    }

    Ok(())
}

/// C: `port_blocked_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the implementation does not send on ports known to be
/// amplification risks (53, 138, 1900, 5353, 11211).
#[test]
fn port_blocked() {
    const BLOCKED_PORTS_TO_TEST: [u16; 6] = [0, 53, 138, 1900, 5353, 11211];
    const UNBLOCKED_PORTS_TO_TEST: [u16; 3] = [443, 4433, 33721];

    for port in BLOCKED_PORTS_TO_TEST {
        assert!(
            crate::check_port_blocked(port),
            "test port {port} should be in the blocked set"
        );
        port_blocked_test_port(port, true).expect("blocked port");
    }

    for port in UNBLOCKED_PORTS_TO_TEST {
        assert!(
            !crate::check_port_blocked(port),
            "test port {port} should not be in the blocked set"
        );
        port_blocked_test_port(port, false).expect("unblocked port");
    }
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
/// Verifies that the path-probing API accepts probes until the path table is
/// full, then rejects the capacity trial.
#[test]
fn probe_api() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let t4: [SocketAddr; NB_PATH_TARGET] = core::array::from_fn(|i| {
        SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(i as u8, i as u8, i as u8, i as u8)),
            1000u16 + i as u16,
        )
    });
    let t6: [SocketAddr; NB_PATH_TARGET] = core::array::from_fn(|i| {
        SocketAddr::new(
            IpAddr::V6(Ipv6Addr::from([i as u8; 16])),
            2000u16 + i as u16,
        )
    });
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("probe_api ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("probe_api connection");
    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        NB_PATH_TARGET as i32,
        0,
    )
    .expect("probe_api sync");

    let client_local_cid_count = first_local_cnxid_count(test_ctx.cnx_client());
    assert!(
        client_local_cid_count >= NB_PATH_TARGET,
        "Only {client_local_cid_count} CID created on client."
    );
    let server_local_cid_count = first_local_cnxid_count(test_ctx.cnx_server());
    assert!(
        server_local_cid_count >= NB_PATH_TARGET,
        "Only {server_local_cid_count} CID created on server."
    );

    let client = test_ctx.cnx_client();
    let mut nb_trials = 0usize;

    for i in 1..NB_PATH_TARGET {
        if client.nb_paths() >= NB_PATH_TARGET {
            break;
        }

        for j in 0..2 {
            let ret_probe = if j == 0 {
                client.probe_new_path(&t4[0], &t4[i], simulated_time)
            } else {
                client.probe_new_path(&t6[0], &t6[i], simulated_time)
            };
            nb_trials += 1;

            if nb_trials < NB_PATH_TARGET {
                assert!(
                    ret_probe.is_ok(),
                    "Trial {nb_trials} ({i}, {j}) fails with ret = {ret_probe:?}"
                );
            } else {
                assert!(
                    ret_probe.is_err(),
                    "Trial {nb_trials} ({i}, {j}) succeeds unexpectedly"
                );
            }

            if ret_probe.is_ok() {
                seed_probe_path_challenges(client, i, j);
            }
        }
    }
}

fn first_local_cnxid_count(cnx: &Connection) -> usize {
    cnx.local_connection_id_lists
        .first()
        .map(|list| list.connection_ids.len())
        .unwrap_or(0)
}

fn first_remote_cnxid_stash_count(cnx: &Connection) -> usize {
    cnx.remote_connection_id_stashes
        .first()
        .map(|stash| stash.connection_ids.len())
        .unwrap_or(0)
}

fn retire_cnxid_refill_ready(test_ctx: &mut TestTlsApiCtx) -> bool {
    let (client_local, client_has_misc, client_stash_count, client_backlog_empty) = {
        let client = test_ctx.cnx_client();
        (
            first_local_cnxid_count(client),
            client.has_misc_frames(),
            first_remote_cnxid_stash_count(client),
            client.is_cnx_backlog_empty(),
        )
    };
    let (server_local, server_backlog_empty) = {
        let server = test_ctx.cnx_server();
        (
            first_local_cnxid_count(server),
            server.is_cnx_backlog_empty(),
        )
    };

    client_local >= NB_PATH_TARGET
        && server_local >= NB_PATH_TARGET
        && !client_has_misc
        && client_stash_count >= NB_PATH_TARGET - 1
        && client_backlog_empty
        && server_backlog_empty
}

fn assert_cnxid_stash_matches_peer(cnx: &Connection, peer: &Connection, cnx_text: &str) {
    let stash_count = first_remote_cnxid_stash_count(cnx);
    let peer_local_count = first_local_cnxid_count(peer);
    assert_eq!(
        stash_count, peer_local_count,
        "On {cnx_text}, {stash_count} items in stash instead of {peer_local_count}."
    );

    let Some(stash) = cnx.remote_connection_id_stashes.first() else {
        return;
    };
    let Some(peer_list) = peer.local_connection_id_lists.first() else {
        return;
    };

    for (rank, (stashed, peer_token)) in stash
        .connection_ids
        .iter()
        .zip(peer_list.connection_ids.iter())
        .enumerate()
    {
        let peer_cid = peer
            .local_connection_ids
            .get(*peer_token)
            .unwrap_or_else(|| panic!("On {cnx_text}, peer CID token #{rank} is missing."));
        assert_eq!(
            stashed.connection_id, peer_cid.connection_id,
            "On {cnx_text}, cnx ID of stash #{rank} does not match cid[{}] of peer.",
            peer_cid.sequence
        );
    }
}

fn seed_probe_path_challenges(cnx: &mut Connection, i: usize, j: usize) {
    let path_id = cnx.nb_paths() - 1;
    let tuple = cnx.paths[path_id]
        .tuples
        .first_mut()
        .expect("new probe path should have a first tuple");

    for ichal in 0..CHALLENGE_REPEAT_MAX {
        tuple.challenge[ichal] = 10000 + 10 * i as u64 + j as u64 + 1000 * ichal as u64;
    }
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
    use std::io::Write as _;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");
    let quality_update_scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    {
        let mut file = std::fs::File::create(QUALITY_UPDATE_CSV).expect("quality_update.csv");
        writeln!(
            file,
            "Time, Path_id, Sending_rate_CB, Pacing_rate, Receive_Rate, CWIN, RTT"
        )
        .expect("quality_update header");
    }

    test_ctx
        .cnx_client()
        .subscribe_to_quality_update(0x10000, crate::Duration::from_ticks(0x1000));

    tls_api_one_scenario_body_ex(
        &mut test_ctx,
        &mut simulated_time,
        &quality_update_scenario,
        1_000_000,
        0,
        0,
        20_000,
        3_600_000,
        &[],
    )
    .expect("quality_update scenario");

    compare_text_files(QUALITY_UPDATE_CSV, QUALITY_UPDATE_REF).expect("quality_update reference");
}

/// C: `tls_quant_params_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a very-long-stream scenario with quantised transport parameters to
/// verify interoperability with Quant.
#[test]
fn quant_params() {
    let mut t = Instant::from_ticks(0);
    let mut client_params = crate::TransportParameters::default();
    crate::internal::init_transport_parameters(&mut client_params);
    client_params.initial_max_data = 0x4000;
    client_params.initial_max_stream_id_bidir = 0;
    client_params.initial_max_stream_id_unidir = 16_384;
    client_params.initial_max_stream_data_bidi_local = 0x2000;
    client_params.initial_max_stream_data_bidi_remote = 0x2000;
    client_params.initial_max_stream_data_uni = 0x2000;

    let mut ctx = tls_api_one_scenario_init_ex(
        &mut t,
        Version::InternalTest1,
        Some(&client_params),
        None,
        None,
    )
    .expect("ctx");
    let quant_scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 10_000,
    }];
    tls_api_one_scenario_body(&mut ctx, &mut t, &quant_scenario, 0, 0, 0, 0, 3_510_000)
        .expect("quant_params");
}

/// C: `random_padding_test` in `picoquictest/tls_api_test.c`.
///
/// Mutates the first client packet by appending deterministic random bytes,
/// marks the first appended byte as non-QUIC, and verifies that the server
/// accepts the real packet and the connection completes.
#[test]
fn random_padding() {
    let mut random_context = 0x1234_5678_90ab_cdef;

    random_padding_test_one(128, &mut random_context, 0).expect("random_padding_128");
    random_padding_test_one(16, &mut random_context, 1).expect("random_padding_16");
}

fn random_padding_test_one(
    pad_length: usize,
    random_context: &mut u64,
    test_id: u8,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut initial_cid_bytes = [0x8a, 0x8d, 0x08, 0x9a, 0xdd, 0, 0, 0];
    initial_cid_bytes[5] = test_id;
    let initial_cid =
        ConnectionId::clone_from_slice(&initial_cid_bytes).ok_or(crate::Error::Generic)?;

    save_empty_tickets(RANDOM_PADDING_TICKET_FILE, simulated_time)?;

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        INTEROP_VERSION_LATEST as u32,
        Some(RANDOM_PADDING_TICKET_FILE),
        Some(&initial_cid),
    )
    .ok_or(crate::Error::Generic)?;

    test_ctx
        .qserver
        .set_textlog(Some(RANDOM_PADDING_TEXT_LOG))?;
    test_ctx.qserver.set_qlog(".")?;
    test_ctx.qserver.set_log_level(1);

    let mut packet = TestSimPacket::create()?;
    let prepared = test_ctx
        .cnx_client()
        .prepare_packet(simulated_time, &mut packet.bytes)?;
    if prepared.send_length == 0 {
        return Err(crate::Error::Generic);
    }

    let padded_length = prepared
        .send_length
        .checked_add(pad_length)
        .filter(|length| *length <= MAX_PACKET_SIZE)
        .ok_or(crate::Error::BufferTooSmall)?;
    packet.length = prepared.send_length;
    packet.addr_to = Some(prepared.addr_to);
    packet.addr_from = Some(test_ctx.client_addr);

    test_random_bytes(
        random_context,
        &mut packet.bytes[packet.length..padded_length],
    );
    packet.bytes[packet.length] |= 0x80;
    packet.length = padded_length;

    simulated_time = Instant::from_ticks(
        simulated_time
            .ticks()
            .saturating_add(test_ctx.c_to_s_link.microsec_latency),
    );
    let addr_from = packet.addr_from.ok_or(crate::Error::Generic)?;
    let addr_to = packet.addr_to.ok_or(crate::Error::Generic)?;
    test_ctx.qserver.incoming_packet(
        &mut packet.bytes[..packet.length],
        &addr_from,
        &addr_to,
        0,
        0,
        simulated_time,
    )?;
    if !test_ctx.has_cnx_server() {
        return Err(crate::Error::Generic);
    }

    let queue_delay_max = 2 * test_ctx.c_to_s_link.microsec_latency;
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        queue_delay_max,
        &mut simulated_time,
    )?;

    if test_ctx.client_ready() && test_ctx.server_ready() {
        Ok(())
    } else {
        Err(crate::Error::Generic)
    }
}

/// C: `random_public_tester_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies the seeded public uniform RNG distribution with a chi-squared
/// test over 11 buckets.
#[test]
fn random_public_tester() {
    const RANDOM_PUBLIC_TEST_CONST: usize = 11;
    const RANDOM_PUBLIC_TEST_ROUNDS: usize = 100;
    const RANDOM_PUBLIC_TEST_SEED: u64 = 0xDEAD_BEEF_CAFE_C001;
    const RANDOM_PUBLIC_CHI_SQUARE: f64 = 18.31;

    let mut r_count = [0usize; RANDOM_PUBLIC_TEST_CONST];

    crate::public_random_seed_64(RANDOM_PUBLIC_TEST_SEED, 1);

    for _ in 0..(RANDOM_PUBLIC_TEST_CONST * RANDOM_PUBLIC_TEST_ROUNDS) {
        let x = crate::picoquic_uniform_random(RANDOM_PUBLIC_TEST_CONST as u64);
        assert!(
            x < RANDOM_PUBLIC_TEST_CONST as u64,
            "Value {x} >= {RANDOM_PUBLIC_TEST_CONST}"
        );
        r_count[x as usize] += 1;
    }

    let mut chi_squared = 0.0;
    for count in r_count {
        let delta = RANDOM_PUBLIC_TEST_ROUNDS as f64 - count as f64;
        chi_squared += (delta * delta) / RANDOM_PUBLIC_TEST_ROUNDS as f64;
    }

    assert!(
        chi_squared <= RANDOM_PUBLIC_CHI_SQUARE,
        "Chi2 = {chi_squared}, larger than {RANDOM_PUBLIC_CHI_SQUARE}"
    );
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
/// RED (random early discard) test using BBR; target_time=500 ms, loss_target=170.
#[test]
fn red_bbr() {
    red_cc_algotest("bbr", 500_000, 170).expect("red_bbr");
}

/// C: `red_cubic_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using Cubic; target_time=510 ms, loss_target=225.
#[test]
fn red_cubic() {
    red_cc_algotest("cubic", 510_000, 225).expect("red_cubic");
}

/// C: `red_dcubic_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using Delay-based Cubic; target_time=500 ms, loss_target=275.
#[test]
fn red_dcubic() {
    red_cc_algotest("dcubic", 500_000, 275).expect("red_dcubic");
}

/// C: `red_fast_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using FastCC; target_time=500 ms, loss_target=250.
#[test]
fn red_fast() {
    red_cc_algotest("fast", 500_000, 250).expect("red_fast");
}

/// C: `red_newreno_test` in `picoquictest/tls_api_test.c`.
///
/// RED test using NewReno; target_time=500 ms, loss_target=150.
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
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("retire_cnxid ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("retire_cnxid connection");
    tls_api_synch_to_empty_loop(
        &mut test_ctx,
        &mut simulated_time,
        2048,
        NB_PATH_TARGET as i32,
        0,
    )
    .expect("retire_cnxid initial sync");

    let client_local_cid_count = first_local_cnxid_count(test_ctx.cnx_client());
    assert!(
        client_local_cid_count >= NB_PATH_TARGET,
        "Only {client_local_cid_count} cids created on client."
    );
    let server_local_cid_count = first_local_cnxid_count(test_ctx.cnx_server());
    assert!(
        server_local_cid_count >= NB_PATH_TARGET,
        "Only {server_local_cid_count} cids created on server."
    );

    for i in 2..NB_PATH_TARGET {
        let client = test_ctx.cnx_client();
        let (stash_index, cid_index) = client
            .obtain_stashed_connection_id(0)
            .unwrap_or_else(|| panic!("Could not retrieve cnx ID #{}.", i - 1));
        let sequence =
            client.remote_connection_id_stashes[stash_index].connection_ids[cid_index].sequence;
        client
            .queue_retire_connection_id_frame(0, sequence)
            .expect("queue RETIRE_CONNECTION_ID");
        let _ = client.remove_stashed_cnxid(0, cid_index, None);
    }

    let time_out = Instant::from_ticks(simulated_time.ticks() + 8_000_000);
    let mut nb_rounds = 0;
    let mut success = false;

    while simulated_time.ticks() < time_out.ticks()
        && nb_rounds < 2048
        && test_ctx
            .qclient
            .first_cnx_mut()
            .map(|c| c.connection_state != crate::State::Disconnected)
            .unwrap_or(false)
    {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            time_out,
            &mut was_active,
        )
        .expect("retire_cnxid refill round");
        nb_rounds += 1;

        if retire_cnxid_refill_ready(&mut test_ctx) {
            success = true;
            break;
        }
    }

    assert!(
        success,
        "Exit synch loop after {nb_rounds} rounds, backlog or not enough cids ({} & {}).",
        first_local_cnxid_count(test_ctx.cnx_client()),
        first_local_cnxid_count(test_ctx.cnx_server())
    );

    let server_local_cid_count = first_local_cnxid_count(test_ctx.cnx_server());
    assert_eq!(
        server_local_cid_count, NB_PATH_TARGET,
        "Found {server_local_cid_count} cids active on server instead of {NB_PATH_TARGET}."
    );

    {
        let (client, server) = (&test_ctx.qclient, &test_ctx.qserver);
        let client_cnx = client.connections.iter().next().expect("client connection");
        let server_cnx = server.connections.iter().next().expect("server connection");
        assert_cnxid_stash_matches_peer(client_cnx, server_cnx, "client");
        assert_cnxid_stash_matches_peer(server_cnx, client_cnx, "server");
    }
}

/// C: `tls_api_retry_test` in `picoquictest/tls_api_test.c`.
///
/// Basic Retry test with a standard-sized ClientHello.
#[test]
fn retry() {
    tls_api_retry_test_one(false).expect("retry");
}

fn retry_large_delayed_start() -> crate::Result<()> {
    const TARGET_TIME: u64 = 230_000;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        tls_api_init_ctx(&mut simulated_time, 0, None).ok_or(crate::Error::Generic)?;
    let old_initial_cid = test_ctx
        .qclient
        .first_cnx_mut()
        .map(|cnx| cnx.initial_connection_id)
        .ok_or(crate::Error::Generic)?;
    let (old_token, _) = test_ctx
        .qclient
        .connection_by_id(old_initial_cid)
        .ok_or(crate::Error::Generic)?;
    test_ctx.qclient.delete_connection(old_token);

    {
        let cnx = test_ctx
            .qclient
            .create_connection(
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                ConnectionId::with_size(0).ok_or(crate::Error::Generic)?,
                Some(&test_ctx.server_addr),
                simulated_time,
                0,
                Some(TEST_SNI),
                Some(TEST_ALPN),
                true,
            )
            .ok_or(crate::Error::Generic)?;
        cnx.test_large_chello = true;
    }

    test_ctx.qclient.set_qlog(".")?;
    test_ctx.cnx_client().start_client()?;
    test_ctx.qserver.set_cookie_mode(1);

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)?;

    assert!(
        simulated_time.ticks() <= TARGET_TIME,
        "Retry test completes in {} microsec, more than {}",
        simulated_time.ticks(),
        TARGET_TIME
    );

    Ok(())
}

/// C: `tls_api_retry_large_test` in `picoquictest/tls_api_test.c`.
///
/// Retry test with a large ClientHello (padded to trigger multi-packet
/// Initial).
#[test]
fn retry_large() {
    retry_large_delayed_start().expect("retry_large");
}

/// C: `tls_retry_token_test` in `picoquictest/tls_api_test.c`.
///
/// Retry-token test: exercises Retry-required mode, provide-token mode, and
/// duplicate-token rejection.
#[test]
fn retry_token() {
    tls_retry_token_test_one(1, false).expect("retry_token retry-required");
    tls_retry_token_test_one(2, false).expect("retry_token provide-token");
    tls_retry_token_test_one(1, true).expect("retry_token duplicate");
}

/// C: `tls_retry_token_valid_test` in `picoquictest/tls_api_test.c`.
///
/// Validates retry-token and new-token contents, expiry, address binding, RCID
/// and PN checks, and oversized-token rejection.
#[test]
fn retry_token_valid() {
    const TIME_BASE: u64 = 10_000 * 1_000_000;

    let mut simulated_time = Instant::from_ticks(TIME_BASE);
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, None).expect("ctx");
    let quic = &mut test_ctx.qserver;

    let addr = [
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 1234),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 3456),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(3, 3, 3, 3)), 1234),
    ];
    let n_cid = ConnectionId::default();
    let cid = [
        ConnectionId::clone_from_slice(&[1, 1, 1, 1, 1, 1, 1, 1]).expect("cid1"),
        n_cid,
        ConnectionId::clone_from_slice(&[2, 2, 2, 2, 2, 2, 2, 2, 2]).expect("cid2"),
    ];
    let odcid = [
        ConnectionId::clone_from_slice(&[3, 3, 3, 3, 3, 3, 3, 3]).expect("odcid"),
        n_cid,
    ];
    let pn = [0u32, 1, 2];

    for token_mode in 0..2 {
        let expected_new_token = odcid[token_mode].is_empty();
        let mut token_buffer = [0u8; 128];
        let token_size = quic
            .prepare_retry_token(
                &addr[0],
                Instant::from_ticks(TIME_BASE),
                &odcid[token_mode],
                &cid[token_mode],
                pn[1],
                &mut token_buffer,
            )
            .expect("prepare_retry_token");
        let token = &token_buffer[..token_size];

        let verified = quic
            .verify_retry_token(
                &addr[0],
                Instant::from_ticks(TIME_BASE),
                &cid[0],
                pn[2],
                token,
                false,
            )
            .expect("valid token");
        assert_retry_token_verified(
            verified,
            expected_new_token,
            &odcid[token_mode],
            "normal parameters",
        );

        let verified = quic
            .verify_retry_token(
                &addr[1],
                Instant::from_ticks(TIME_BASE),
                &cid[0],
                pn[2],
                token,
                false,
            )
            .expect("same-IP token");
        assert_retry_token_verified(
            verified,
            expected_new_token,
            &odcid[token_mode],
            "same IP, different port",
        );

        assert!(
            quic.verify_retry_token(
                &addr[2],
                Instant::from_ticks(TIME_BASE),
                &cid[0],
                pn[2],
                token,
                false,
            )
            .is_err(),
            "Token validation does not detect an address change."
        );

        let expired_delta = if token_mode == 0 {
            TOKEN_DELAY_SHORT.ticks() + 1
        } else {
            TOKEN_DELAY_LONG.ticks() + 1_000_000
        };
        assert!(
            quic.verify_retry_token(
                &addr[0],
                Instant::from_ticks(TIME_BASE + expired_delta),
                &cid[0],
                pn[2],
                token,
                false,
            )
            .is_err(),
            "Token validation does not detect elapsed time."
        );

        let rcid_mismatch = quic.verify_retry_token(
            &addr[0],
            Instant::from_ticks(TIME_BASE),
            &cid[2],
            pn[2],
            token,
            false,
        );
        if token_mode == 0 {
            assert!(rcid_mismatch.is_err(), "RCID invalidation fails");
        } else {
            let verified = rcid_mismatch.expect("new token ignores RCID");
            assert_retry_token_verified(verified, true, &n_cid, "new token RCID mismatch");
        }

        for &initial_pn in pn.iter().take(2) {
            let pn_mismatch = quic.verify_retry_token(
                &addr[0],
                Instant::from_ticks(TIME_BASE),
                &cid[0],
                initial_pn,
                token,
                false,
            );
            if token_mode == 0 {
                assert!(pn_mismatch.is_err(), "PN invalidation fails");
            } else {
                let verified = pn_mismatch.expect("new token ignores PN");
                assert_retry_token_verified(verified, true, &n_cid, "new token PN mismatch");
            }
        }

        if token_mode == 0 {
            let mut big_token = vec![0xa5; MAX_PACKET_SIZE];
            big_token[..token_size].copy_from_slice(token);
            assert!(
                quic.verify_retry_token(
                    &addr[0],
                    Instant::from_ticks(TIME_BASE),
                    &cid[0],
                    pn[2],
                    &big_token,
                    false,
                )
                .is_err(),
                "Bad length check fails"
            );
        }
    }
}

fn assert_retry_token_verified(
    verified: crate::tls_api::VerifiedRetryToken,
    expected_new_token: bool,
    expected_odcid: &ConnectionId,
    label: &str,
) {
    assert_eq!(
        verified.is_new_token, expected_new_token,
        "{label}: wrong new-token classification"
    );
    assert_eq!(verified.odcid, *expected_odcid, "{label}: wrong ODCID");
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
    let mut loss_mask = 0u64;
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("server_busy ctx");

    test_ctx.qserver.server_busy = true;
    let _ = tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time);

    if let Some(server) = test_ctx.qserver.first_cnx_mut() {
        assert_eq!(
            server.state(),
            crate::State::Disconnected,
            "server state {:?}, local error {:x}",
            server.state(),
            server.local_error()
        );
    }

    let client = test_ctx.cnx_client();
    assert_eq!(
        client.state(),
        crate::State::Disconnected,
        "client state {:?}, remote error {:x}",
        client.state(),
        client.remote_error()
    );
    assert_eq!(
        client.remote_error(),
        crate::TransportError::ServerBusy as u64,
        "client remote error {:x}",
        client.remote_error()
    );
    assert!(
        simulated_time.ticks() <= 500_000,
        "simulated time {}",
        simulated_time.ticks()
    );

    test_ctx.qserver.server_busy = false;
    delete_tls_api_test_connections(&mut test_ctx.qserver);
    delete_tls_api_test_connections(&mut test_ctx.qclient);

    test_ctx
        .qclient
        .create_connection(
            ConnectionId::with_size(0).expect("null initial CID"),
            ConnectionId::with_size(0).expect("null remote CID"),
            Some(&test_ctx.server_addr),
            simulated_time,
            0,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .expect("new client connection")
        .start_client()
        .expect("start new client connection");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("server_busy retry connection");
    assert!(test_ctx.client_ready(), "client did not reach ready state");
    assert!(test_ctx.server_ready(), "server did not reach ready state");

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("server_busy close");
}

fn delete_tls_api_test_connections(quic: &mut crate::internal::Quic) {
    while let Some(token) = quic.first_connection().and_then(|cnx| cnx.own_token) {
        quic.delete_connection(token);
    }
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
