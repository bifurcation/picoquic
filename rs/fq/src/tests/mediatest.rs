//! Test cases for `picoquictest/mediatest.c`.
//!
//! Media-stream simulation tests: video, audio, and data streams are
//! generated with realistic frame rates and sizes; the tests verify that
//! latency and jitter stay within specified bounds under various network
//! conditions (bandwidth, Wi-Fi jitter, suspension, etc.).
//!
//! The C source builds its own simulation loop (`mediatest_ctx_t`) separate
//! from the generic `picoquic_test_tls_api_ctx_t` infrastructure.  The Rust
//! translation mirrors that structure via [`MediatestSpec`] /
//! [`mediatest_one`].

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, TestSimPacket, TestTlsApiCtx, test_api_init_send_recv_scenario,
    tls_api_close_with_losses, tls_api_connection_loop, tls_api_data_sending_loop,
    tls_api_init_ctx_ex2, tls_api_one_scenario_body_verify, tls_api_one_sim_round,
};
use crate::internal::Version;
use crate::{
    Connection, ConnectionId, Instant, MAX_PACKET_SIZE, StreamDirectReceive, TransportParameters,
};
use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Media-test identifiers.  C: `mediatest_id_enum`.

/// Which media-test scenario to run.  C: `mediatest_id_enum`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
pub enum MediatestId {
    Video = 1,
    VideoAudio = 2,
    VideoDataAudio = 3,
    Worst = 4,
    Video2Down = 5,
    Wifi = 6,
    Video2Back = 7,
    Suspension = 8,
    Video2Probe = 9,
    Suspension2 = 10,
    NoCoal = 11,
}

// ---------------------------------------------------------------------------
// Test specification.  C: `st_mediatest_spec_t`.

/// Test parameters for a single media-test run.  C: `mediatest_spec_t`.
///
/// Fields mirror the C struct of the same name; unused fields default to 0/false.
#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub struct MediatestSpec {
    /// Congestion-control algorithm to use.
    /// C: `ccalgo` (`picoquic_congestion_algorithm_t*`).
    pub ccalgo: Option<&'static crate::CongestionAlgorithm>,
    /// Include audio stream.  C: `do_audio`.
    pub do_audio: bool,
    /// Include video (normal rate) stream.  C: `do_video`.
    pub do_video: bool,
    /// Include video2 (higher rate) stream.  C: `do_video2`.
    pub do_video2: bool,
    /// Trigger a BBR/Cubic probe-up event.  C: `do_probe_up`.
    pub do_probe_up: bool,
    /// Data stream size (bytes).  C: `data_size`.
    pub data_size: usize,
    /// Datagram data size (bytes).  C: `datagram_data_size`.
    pub datagram_data_size: usize,
    /// Simulated link bandwidth (Gbps).  C: `bandwidth`.
    pub bandwidth: f64,
    /// Link latency (µs).  C: `link_latency`.
    pub link_latency: u64,
    /// Expected average audio/video latency (µs).  C: `latency_average`.
    pub latency_average: u64,
    /// Maximum allowed audio/video latency (µs).  C: `latency_max`.
    pub latency_max: u64,
    /// Stream priority below which streams bypass coalescing.
    /// C: `priority_limit_for_bypass`.
    pub priority_limit_for_bypass: u8,
    /// Skip video2 statistics verification.  C: `do_not_check_video2`.
    pub do_not_check_video2: bool,
    /// Number of simulated Wi-Fi suspensions.  C: `nb_suspensions`.
    pub nb_suspensions: i32,
    /// Time of the first suspension (µs).  C: `suspension_start_time`.
    pub suspension_start_time: u64,
    /// Inter-suspension gap (µs).  C: `suspension_up_time`.
    pub suspension_up_time: u64,
    /// Suspension duration (µs).  C: `suspension_down_time`.
    pub suspension_down_time: u64,
    /// Disable coalescing of audio/video frames.  C: `no_coal`.
    pub no_coal: bool,
}

// ---------------------------------------------------------------------------
// Core test driver.

const MEDIATEST_HEADER_SIZE: usize = 13;
const MEDIATEST_DURATION: u64 = 10_000_000;
const MEDIATEST_AUDIO_PERIOD: u64 = 20_000;
const MEDIATEST_VIDEO_PERIOD: u64 = 33_333;
const MEDIATEST_VIDEO2_PERIOD: u64 = 16_666;
const MEDIATEST_DATA_FRAME_SIZE: usize = 0x4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum MediaTestType {
    Data = 0,
    Audio = 1,
    Video = 2,
    Video2 = 3,
}

impl MediaTestType {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Data),
            1 => Some(Self::Audio),
            2 => Some(Self::Video),
            3 => Some(Self::Video2),
            _ => None,
        }
    }

    fn stats_index(self) -> usize {
        self as usize
    }

    fn period(self) -> u64 {
        match self {
            Self::Data => 0,
            Self::Audio => MEDIATEST_AUDIO_PERIOD,
            Self::Video => MEDIATEST_VIDEO_PERIOD,
            Self::Video2 => MEDIATEST_VIDEO2_PERIOD,
        }
    }

    fn frame_size(self, frame_index: u64) -> usize {
        match self {
            Self::Data => MEDIATEST_DATA_FRAME_SIZE,
            Self::Audio => 32,
            Self::Video => {
                if frame_index.is_multiple_of(100) {
                    0x8000
                } else {
                    0x800
                }
            }
            Self::Video2 => {
                if frame_index.is_multiple_of(100) {
                    0x10000
                } else {
                    0x1800
                }
            }
        }
    }

    fn frames_to_send(self, spec: &MediatestSpec) -> u64 {
        match self {
            Self::Data => (spec.data_size / MEDIATEST_DATA_FRAME_SIZE) as u64,
            Self::Audio => MEDIATEST_DURATION / MEDIATEST_AUDIO_PERIOD,
            Self::Video => MEDIATEST_DURATION / MEDIATEST_VIDEO_PERIOD,
            Self::Video2 => MEDIATEST_DURATION / MEDIATEST_VIDEO2_PERIOD,
        }
    }

    fn stream_priority(self) -> Option<u8> {
        match self {
            Self::Data => None,
            Self::Audio => Some(2),
            Self::Video => Some(4),
            Self::Video2 => Some(6),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MediaStats {
    nb_frames: u64,
    sum_delays: u64,
    sum_square_delays: u64,
    min_delay: u64,
    max_delay: u64,
}

impl MediaStats {
    const fn new() -> Self {
        Self {
            nb_frames: 0,
            sum_delays: 0,
            sum_square_delays: 0,
            min_delay: u64::MAX,
            max_delay: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MediaMessageRx {
    bytes_received: usize,
    message_type: Option<MediaTestType>,
    message_size: usize,
    sent_time: u64,
    header: [u8; MEDIATEST_HEADER_SIZE],
}

impl Default for MediaMessageRx {
    fn default() -> Self {
        Self {
            bytes_received: 0,
            message_type: None,
            message_size: 0,
            sent_time: 0,
            header: [0; MEDIATEST_HEADER_SIZE],
        }
    }
}

#[derive(Debug)]
struct MediatestRecvStream {
    stream_id: u64,
    stream_type: MediaTestType,
    frames_received: u64,
    server_fin_received: bool,
    client_fin_received: bool,
    message: MediaMessageRx,
}

impl MediatestRecvStream {
    fn new(stream_id: u64, stream_type: MediaTestType) -> Self {
        Self {
            stream_id,
            stream_type,
            frames_received: 0,
            server_fin_received: false,
            client_fin_received: false,
            message: MediaMessageRx::default(),
        }
    }
}

#[derive(Debug)]
struct MediatestRuntime {
    simulated_time: u64,
    disruption_clear: u64,
    stats: [MediaStats; 4],
    streams: Vec<MediatestRecvStream>,
    error: bool,
}

impl MediatestRuntime {
    fn new(streams: Vec<MediatestRecvStream>, disruption_clear: u64) -> Self {
        Self {
            simulated_time: 0,
            disruption_clear,
            stats: [MediaStats::new(); 4],
            streams,
            error: false,
        }
    }

    fn stream_index(&self, stream_id: u64) -> Option<usize> {
        self.streams
            .iter()
            .position(|stream| stream.stream_id == stream_id)
    }

    fn record_stats(&mut self, media_type: MediaTestType, sent_time: u64) {
        if sent_time > self.disruption_clear {
            let delay = self.simulated_time.saturating_sub(sent_time);
            let stats = &mut self.stats[media_type.stats_index()];
            stats.nb_frames += 1;
            stats.sum_delays += delay;
            stats.sum_square_delays += delay * delay;
            stats.min_delay = stats.min_delay.min(delay);
            stats.max_delay = stats.max_delay.max(delay);
        }
    }

    fn receive_server(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        fin: bool,
        bytes: &[u8],
        offset: u64,
    ) -> i32 {
        mediatest_mark_stream_consumed(connection, stream_id, offset, bytes.len());

        if !bytes.is_empty() && !self.receive_media_bytes(stream_id, bytes) {
            self.error = true;
        }

        if fin {
            if let Some(stream_index) = self.stream_index(stream_id) {
                self.streams[stream_index].server_fin_received = true;
                if connection.add_to_stream(stream_id, &[], true).is_err() {
                    self.error = true;
                }
            } else {
                self.error = true;
            }
        }

        if self.error { -1 } else { 0 }
    }

    fn receive_media_bytes(&mut self, stream_id: u64, bytes: &[u8]) -> bool {
        let Some(stream_index) = self.stream_index(stream_id) else {
            return false;
        };
        let mut msg_read = 0usize;

        while msg_read < bytes.len() {
            let completed = {
                let stream = &mut self.streams[stream_index];

                while msg_read < bytes.len()
                    && stream.message.bytes_received < MEDIATEST_HEADER_SIZE
                {
                    stream.message.header[stream.message.bytes_received] = bytes[msg_read];
                    stream.message.bytes_received += 1;
                    msg_read += 1;

                    if stream.message.bytes_received == MEDIATEST_HEADER_SIZE {
                        let Some(message_type) = MediaTestType::from_u8(stream.message.header[0])
                        else {
                            return false;
                        };
                        let message_size = u32::from_be_bytes([
                            stream.message.header[1],
                            stream.message.header[2],
                            stream.message.header[3],
                            stream.message.header[4],
                        ]) as usize;
                        let sent_time = u64::from_be_bytes([
                            stream.message.header[5],
                            stream.message.header[6],
                            stream.message.header[7],
                            stream.message.header[8],
                            stream.message.header[9],
                            stream.message.header[10],
                            stream.message.header[11],
                            stream.message.header[12],
                        ]);
                        if message_size < MEDIATEST_HEADER_SIZE
                            || message_type != stream.stream_type
                        {
                            return false;
                        }
                        stream.message.message_type = Some(message_type);
                        stream.message.message_size = message_size;
                        stream.message.sent_time = sent_time;
                    }
                }

                if msg_read >= bytes.len() {
                    None
                } else if stream.message.message_size < stream.message.bytes_received {
                    return false;
                } else {
                    let required = stream.message.message_size - stream.message.bytes_received;
                    let available = bytes.len() - msg_read;
                    let consumed = required.min(available);
                    stream.message.bytes_received += consumed;
                    msg_read += consumed;

                    if consumed == required {
                        let Some(message_type) = stream.message.message_type else {
                            return false;
                        };
                        let sent_time = stream.message.sent_time;
                        stream.frames_received += 1;
                        stream.message = MediaMessageRx::default();
                        Some((message_type, sent_time))
                    } else {
                        None
                    }
                }
            };

            if let Some((message_type, sent_time)) = completed {
                self.record_stats(message_type, sent_time);
            }
        }

        true
    }

    fn receive_client(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        fin: bool,
        bytes: &[u8],
        offset: u64,
    ) -> i32 {
        mediatest_mark_stream_consumed(connection, stream_id, offset, bytes.len());

        if !bytes.is_empty() {
            self.error = true;
        }
        if fin {
            if let Some(stream_index) = self.stream_index(stream_id) {
                self.streams[stream_index].client_fin_received = true;
            } else {
                self.error = true;
            }
        }

        if self.error { -1 } else { 0 }
    }

    fn all_fins_received(&self) -> bool {
        self.streams
            .iter()
            .all(|stream| stream.server_fin_received && stream.client_fin_received)
    }
}

fn mediatest_mark_stream_consumed(
    connection: &mut Connection,
    stream_id: u64,
    offset: u64,
    length: usize,
) {
    let consumed_to = offset.saturating_add(length as u64);
    if let Some(stream_token) = connection.find_stream(stream_id)
        && let Some(stream) = connection.streams.get_mut(stream_token)
    {
        stream.consumed_offset = stream.consumed_offset.max(consumed_to);
    }
}

#[derive(Clone, Copy)]
enum MediatestEndpoint {
    Server,
    Client,
}

struct MediatestDirectReceive {
    runtime: Rc<RefCell<MediatestRuntime>>,
    endpoint: MediatestEndpoint,
}

impl StreamDirectReceive for MediatestDirectReceive {
    fn receive(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        fin: bool,
        bytes: &[u8],
        offset: u64,
    ) -> i32 {
        match self.endpoint {
            MediatestEndpoint::Server => self
                .runtime
                .borrow_mut()
                .receive_server(connection, stream_id, fin, bytes, offset),
            MediatestEndpoint::Client => self
                .runtime
                .borrow_mut()
                .receive_client(connection, stream_id, fin, bytes, offset),
        }
    }
}

#[derive(Debug)]
struct MediatestSender {
    stream_id: u64,
    media_type: MediaTestType,
    frames_to_send: u64,
    frames_sent: u64,
    frames_skipped: u64,
    next_frame_time: u64,
    total_bytes: u64,
    is_reset: bool,
    finished_sending: bool,
}

impl MediatestSender {
    fn new(stream_id: u64, media_type: MediaTestType, frames_to_send: u64) -> Self {
        let total_bytes = (0..frames_to_send)
            .map(|frame| media_type.frame_size(frame) as u64)
            .sum();
        Self {
            stream_id,
            media_type,
            frames_to_send,
            frames_sent: 0,
            frames_skipped: 0,
            next_frame_time: 0,
            total_bytes,
            is_reset: false,
            finished_sending: false,
        }
    }

    fn is_finished(&self) -> bool {
        self.finished_sending
    }

    fn simulate_video2_reset(&mut self, current_time: u64) {
        if self.media_type != MediaTestType::Video2 {
            return;
        }

        let frame_rank = self.frames_sent % 100;
        if frame_rank > 0 && self.frames_sent > 20 {
            if self
                .frames_sent
                .saturating_sub(20)
                .saturating_mul(MEDIATEST_VIDEO2_PERIOD)
                > current_time
            {
                self.is_reset = true;
            }
            if self.is_reset {
                let skipped = 100 - frame_rank;
                self.next_frame_time = self
                    .next_frame_time
                    .saturating_add(skipped.saturating_mul(MEDIATEST_VIDEO2_PERIOD));
                self.frames_sent = self.frames_sent.saturating_add(skipped);
                self.frames_skipped = self.frames_skipped.saturating_add(skipped);
            }
        } else {
            self.is_reset = false;
        }
    }

    fn queue_ready_frame(
        &mut self,
        connection: &mut Connection,
        current_time: u64,
    ) -> crate::Result<bool> {
        if self.is_finished() || self.next_frame_time > current_time {
            return Ok(false);
        }

        self.simulate_video2_reset(current_time);
        if self.next_frame_time > current_time {
            return Ok(false);
        }

        if self.frames_sent == 0 {
            self.next_frame_time = current_time;
        }
        let sent_time = self.next_frame_time;
        let payload = mediatest_frame_payload(self.media_type, self.frames_sent, sent_time);

        match self.media_type {
            MediaTestType::Data => {
                self.next_frame_time = current_time;
            }
            _ => {
                self.next_frame_time = self
                    .next_frame_time
                    .saturating_add(self.media_type.period());
            }
        }

        let set_fin = self.frames_sent + 1 >= self.frames_to_send;
        connection.add_to_stream(self.stream_id, &payload, set_fin)?;
        self.frames_sent += 1;
        self.finished_sending = set_fin;
        Ok(true)
    }
}

fn mediatest_frame_payload(media_type: MediaTestType, frame_index: u64, sent_time: u64) -> Vec<u8> {
    let message_size = media_type.frame_size(frame_index);
    let mut payload = vec![media_type as u8; message_size];
    payload[0] = media_type as u8;
    payload[1..5].copy_from_slice(&(message_size as u32).to_be_bytes());
    payload[5..MEDIATEST_HEADER_SIZE].copy_from_slice(&sent_time.to_be_bytes());
    payload
}

fn mediatest_transport_parameters() -> TransportParameters {
    TransportParameters {
        initial_max_stream_data_bidi_local: 0x200000,
        initial_max_stream_data_bidi_remote: 65635,
        initial_max_stream_data_uni: 65535,
        initial_max_data: 0x100000,
        initial_max_stream_id_bidir: 512,
        initial_max_stream_id_unidir: 512,
        max_idle_timeout: crate::Duration::from_ticks(30_000),
        max_packet_size: MAX_PACKET_SIZE as u32,
        ack_delay_exponent: 3,
        active_connection_id_limit: 4,
        max_ack_delay: 10_000,
        enable_loss_bit: 2,
        min_ack_delay: crate::Duration::from_ticks(1000),
        enable_time_stamp: 0,
        max_datagram_frame_size: MAX_PACKET_SIZE as u32,
        ..TransportParameters::default()
    }
}

fn mediatest_configure_sender(
    connection: &mut Connection,
    media_type: MediaTestType,
    spec: &MediatestSpec,
) -> crate::Result<(MediatestSender, MediatestRecvStream)> {
    let stream_id = connection.get_next_local_stream_id(false);
    connection.add_to_stream(stream_id, &[], false)?;
    if let Some(priority) = media_type.stream_priority() {
        connection.set_stream_priority(stream_id, priority)?;
    }
    if spec.no_coal && matches!(media_type, MediaTestType::Audio | MediaTestType::Video) {
        connection.set_stream_not_coalesced(stream_id, true)?;
    }

    let frames_to_send = media_type.frames_to_send(spec);
    Ok((
        MediatestSender::new(stream_id, media_type, frames_to_send),
        MediatestRecvStream::new(stream_id, media_type),
    ))
}

fn mediatest_drain_receives(
    test_ctx: &mut TestTlsApiCtx,
    runtime: &Rc<RefCell<MediatestRuntime>>,
    senders: &[MediatestSender],
    current_time: u64,
) -> crate::Result<()> {
    if !test_ctx.has_cnx_server() {
        return Ok(());
    }

    runtime.borrow_mut().simulated_time = current_time;

    for sender in senders {
        {
            let connection = test_ctx.cnx_server();
            if connection.find_stream(sender.stream_id).is_some() {
                connection.open_flow_control(sender.stream_id, sender.total_bytes)?;
                connection.mark_direct_receive_stream(
                    sender.stream_id,
                    Box::new(MediatestDirectReceive {
                        runtime: Rc::clone(runtime),
                        endpoint: MediatestEndpoint::Server,
                    }),
                )?;
            }
        }
        {
            let connection = test_ctx.cnx_client();
            if connection.find_stream(sender.stream_id).is_some() {
                connection.mark_direct_receive_stream(
                    sender.stream_id,
                    Box::new(MediatestDirectReceive {
                        runtime: Rc::clone(runtime),
                        endpoint: MediatestEndpoint::Client,
                    }),
                )?;
            }
        }
    }

    if runtime.borrow().error {
        Err(crate::Error::Generic)
    } else {
        Ok(())
    }
}

fn mediatest_check_stats(
    runtime: &MediatestRuntime,
    spec: &MediatestSpec,
    media_type: MediaTestType,
) -> crate::Result<()> {
    let stats = runtime.stats[media_type.stats_index()];
    let expected = MEDIATEST_DURATION / media_type.period();

    if stats.nb_frames != expected && runtime.disruption_clear == 0 {
        return Err(crate::Error::Generic);
    }

    if stats.nb_frames != 0 {
        let average = stats
            .sum_delays
            .checked_div(stats.nb_frames)
            .ok_or(crate::Error::Generic)?;
        let variance = stats
            .sum_square_delays
            .checked_div(stats.nb_frames)
            .ok_or(crate::Error::Generic)?
            .saturating_sub(average * average);
        let sigma = (variance as f64).sqrt() as u64;

        if spec.latency_average == 0 {
            if average > 25_000 || sigma > 12_500 || stats.max_delay > 100_000 {
                return Err(crate::Error::Generic);
            }
        } else if average > spec.latency_average
            || (spec.latency_max > 0 && stats.max_delay > spec.latency_max)
        {
            return Err(crate::Error::Generic);
        }
    }

    Ok(())
}

fn mediatest_is_finished(
    senders: &[MediatestSender],
    runtime: &Rc<RefCell<MediatestRuntime>>,
) -> bool {
    senders.iter().all(MediatestSender::is_finished) && runtime.borrow().all_fins_received()
}

fn mediatest_drain_suspended_packets(
    test_ctx: &mut TestTlsApiCtx,
    current_time: Instant,
) -> crate::Result<()> {
    while let Some(mut packet) = test_ctx.s_to_c_link.dequeue(current_time) {
        let addr_from = packet.addr_from.unwrap_or(test_ctx.server_addr);
        let addr_to = packet.addr_to.unwrap_or(test_ctx.client_addr);
        let ecn = packet.ecn_mark;
        test_ctx.qclient.incoming_packet(
            &mut packet.bytes[..packet.length],
            &addr_from,
            &addr_to,
            0,
            ecn,
            current_time,
        )?;
    }

    while let Some(mut packet) = test_ctx.c_to_s_link.dequeue(current_time) {
        let addr_from = packet.addr_from.unwrap_or(test_ctx.client_addr);
        let addr_to = packet.addr_to.unwrap_or(test_ctx.server_addr);
        let ecn = packet.ecn_mark;
        test_ctx.qserver.incoming_packet(
            &mut packet.bytes[..packet.length],
            &addr_from,
            &addr_to,
            0,
            ecn,
            current_time,
        )?;
    }

    Ok(())
}

fn mediatest_run_suspensions(
    test_ctx: &mut TestTlsApiCtx,
    runtime: &Rc<RefCell<MediatestRuntime>>,
    senders: &mut [MediatestSender],
    simulated_time: &mut Instant,
    spec: &MediatestSpec,
    is_finished: &mut bool,
) -> crate::Result<()> {
    let mut suspension_time = spec.suspension_start_time;

    for _ in 0..spec.nb_suspensions {
        if *is_finished {
            break;
        }

        mediatest_loop(
            test_ctx,
            runtime,
            senders,
            simulated_time,
            suspension_time,
            false,
            is_finished,
        )?;

        suspension_time = suspension_time.saturating_add(spec.suspension_down_time);
        let resume_time = Instant::from_ticks(suspension_time);
        test_ctx.s_to_c_link.suspend(resume_time, false);
        test_ctx.c_to_s_link.suspend(resume_time, true);

        mediatest_loop(
            test_ctx,
            runtime,
            senders,
            simulated_time,
            suspension_time,
            false,
            is_finished,
        )?;

        if spec.suspension_up_time == 0 {
            mediatest_drain_suspended_packets(test_ctx, resume_time)?;
        }

        suspension_time = suspension_time.saturating_add(spec.suspension_up_time);
    }

    Ok(())
}

fn mediatest_loop(
    test_ctx: &mut TestTlsApiCtx,
    runtime: &Rc<RefCell<MediatestRuntime>>,
    senders: &mut [MediatestSender],
    simulated_time: &mut Instant,
    simulated_time_max: u64,
    is_losing_data: bool,
    is_finished: &mut bool,
) -> crate::Result<()> {
    let mut nb_steps = 0;
    let mut nb_inactive = 0;

    while !*is_finished
        && nb_steps < 100_000
        && nb_inactive < 512
        && simulated_time.ticks() < simulated_time_max
    {
        nb_steps += 1;
        let before = simulated_time.ticks();
        let mut is_active = false;

        if test_ctx.client_ready() {
            let cnx = test_ctx.cnx_client();
            for sender in senders.iter_mut() {
                is_active |= sender.queue_ready_frame(cnx, simulated_time.ticks())?;
            }
        }

        mediatest_drain_receives(test_ctx, runtime, senders, simulated_time.ticks())?;

        let next_media = if test_ctx.client_ready() {
            senders
                .iter()
                .filter(|sender| !sender.is_finished())
                .map(|sender| sender.next_frame_time)
                .filter(|time| *time > simulated_time.ticks())
                .min()
        } else {
            None
        };
        let time_out = next_media
            .map(Instant::from_ticks)
            .unwrap_or_else(|| Instant::from_ticks(0));
        let mut was_active = false;
        if is_losing_data {
            mediatest_one_lossy_sim_round(test_ctx, simulated_time, time_out, &mut was_active)?;
        } else {
            tls_api_one_sim_round(test_ctx, simulated_time, time_out, &mut was_active)?;
        }

        mediatest_drain_receives(test_ctx, runtime, senders, simulated_time.ticks())?;

        is_active |= was_active || simulated_time.ticks() != before;
        *is_finished = mediatest_is_finished(senders, runtime);

        if is_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }
    }

    Ok(())
}

fn mediatest_one_lossy_sim_round(
    test_ctx: &mut TestTlsApiCtx,
    simulated_time: &mut Instant,
    time_out: Instant,
    was_active: &mut bool,
) -> crate::Result<()> {
    if test_ctx.qserver.pending_stateless_packets.front().is_some() {
        if let Some(sp) = test_ctx.qserver.dequeue_stateless_packet()
            && sp.length > 0
        {
            *was_active = true;
            let mut pkt = TestSimPacket::create()?;
            pkt.addr_from = Some(sp.addr_local);
            pkt.addr_to = Some(sp.addr_to);
            pkt.ecn_mark = test_ctx.packet_ecn_default;
            pkt.length = sp.length;
            pkt.bytes[..sp.length].copy_from_slice(&sp.bytes[..sp.length]);
            test_ctx.s_to_c_link.submit(pkt, *simulated_time);
        }
        return Ok(());
    }

    #[derive(Copy, Clone, PartialEq, Eq)]
    enum Act {
        None,
        ClientDep,
        ServerDep,
        ClientArr,
        ServerArr,
        ClientAdm,
        ServerAdm,
    }

    let base = simulated_time.ticks().saturating_add(120_000_000);
    let mut next_time;
    let mut next_action;

    loop {
        next_time = base;
        next_action = Act::None;

        if let Some(t) = test_ctx
            .qclient
            .first_cnx_mut()
            .filter(|cnx| cnx.connection_state != crate::State::Disconnected)
            .map(|cnx| cnx.next_wake_time.ticks())
            && t < next_time
        {
            next_time = t;
            next_action = Act::ClientDep;
        }

        if let Some(t) = test_ctx
            .qserver
            .first_cnx_mut()
            .filter(|cnx| cnx.connection_state != crate::State::Disconnected)
            .map(|cnx| cnx.next_wake_time.ticks())
            && t < next_time
        {
            next_time = t;
            next_action = Act::ServerDep;
        }

        let t = test_ctx
            .s_to_c_link
            .next_arrival(Instant::from_ticks(next_time));
        if t < next_time {
            next_time = t;
            next_action = Act::ClientArr;
        }

        let t = test_ctx
            .s_to_c_link
            .next_admission(*simulated_time, Instant::from_ticks(next_time));
        if t < next_time {
            next_time = t;
            next_action = Act::ClientAdm;
        }

        let t = test_ctx
            .c_to_s_link
            .next_arrival(Instant::from_ticks(next_time));
        if t < next_time {
            next_time = t;
            next_action = Act::ServerArr;
        }

        let t = test_ctx
            .c_to_s_link
            .next_admission(*simulated_time, Instant::from_ticks(next_time));
        if t < next_time {
            next_time = t;
            next_action = Act::ServerAdm;
        }

        match next_action {
            Act::ClientArr => {
                if test_ctx
                    .s_to_c_link
                    .dequeue(Instant::from_ticks(next_time))
                    .is_some()
                {
                    *was_active = true;
                }
                continue;
            }
            Act::ServerArr => {
                if test_ctx
                    .c_to_s_link
                    .dequeue(Instant::from_ticks(next_time))
                    .is_some()
                {
                    *was_active = true;
                }
                continue;
            }
            Act::ClientAdm => {
                test_ctx
                    .s_to_c_link
                    .admit_pending(Instant::from_ticks(next_time));
                continue;
            }
            Act::ServerAdm => {
                test_ctx
                    .c_to_s_link
                    .admit_pending(Instant::from_ticks(next_time));
                continue;
            }
            _ => break,
        }
    }

    let timeout = time_out.ticks();
    if timeout > 0 && next_time > timeout {
        *simulated_time = time_out;
        return Ok(());
    } else if next_time > simulated_time.ticks() {
        *simulated_time = Instant::from_ticks(next_time);
    }

    match next_action {
        Act::ClientDep => {
            let mut buf = [0u8; MAX_PACKET_SIZE];
            let prep = test_ctx
                .qclient
                .first_cnx_mut()
                .and_then(|cnx| cnx.prepare_packet(*simulated_time, &mut buf).ok());
            if let Some(prep) = prep
                && prep.send_length > 0
            {
                *was_active = true;
                let mut pkt = TestSimPacket::create()?;
                let addr_from = if prep.addr_from.ip().is_unspecified() {
                    test_ctx.client_addr
                } else {
                    prep.addr_from
                };
                pkt.addr_from = Some(addr_from);
                pkt.addr_to = Some(prep.addr_to);
                pkt.ecn_mark = test_ctx.packet_ecn_default;
                pkt.length = prep.send_length;
                pkt.bytes[..prep.send_length].copy_from_slice(&buf[..prep.send_length]);
                test_ctx.c_to_s_link.submit(pkt, *simulated_time);
            }
        }
        Act::ServerDep => {
            let mut buf = [0u8; MAX_PACKET_SIZE];
            let prep = test_ctx
                .qserver
                .first_cnx_mut()
                .and_then(|cnx| cnx.prepare_packet(*simulated_time, &mut buf).ok());
            if let Some(prep) = prep
                && prep.send_length > 0
            {
                *was_active = true;
                let mut pkt = TestSimPacket::create()?;
                let addr_from = if prep.addr_from.ip().is_unspecified() {
                    test_ctx.server_addr
                } else {
                    prep.addr_from
                };
                pkt.addr_from = Some(addr_from);
                pkt.addr_to = Some(prep.addr_to);
                pkt.ecn_mark = test_ctx.packet_ecn_default;
                pkt.length = prep.send_length;
                pkt.bytes[..prep.send_length].copy_from_slice(&buf[..prep.send_length]);
                test_ctx.s_to_c_link.submit(pkt, *simulated_time);
            }
        }
        _ => {}
    }

    Ok(())
}

fn mediatest_media_one(id: MediatestId, spec: &MediatestSpec) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut initial_cid = [0xed, 0x1a, 0x7e, 0x57, 0, 0, 0, 0];
    initial_cid[4] = id as u8;
    let initial_cid = ConnectionId::clone_from_slice(&initial_cid).ok_or(crate::Error::Generic)?;

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some("picoquic_mediatest"),
        None,
        Some(&initial_cid),
    )
    .ok_or(crate::Error::Generic)?;

    let tp = mediatest_transport_parameters();
    test_ctx.cnx_client().set_transport_parameters(&tp);

    let link_latency = if spec.link_latency == 0 {
        10_000
    } else {
        spec.link_latency
    };
    let bandwidth = if spec.bandwidth > 0.0 {
        spec.bandwidth
    } else {
        0.01
    };
    for link in [&mut test_ctx.c_to_s_link, &mut test_ctx.s_to_c_link] {
        **link =
            super::util::TestSimLink::create(bandwidth, link_latency, None, 0, simulated_time)?;
    }

    if let Some(algo) = spec.ccalgo {
        test_ctx.qclient.set_default_congestion_algorithm(algo);
        test_ctx.qserver.set_default_congestion_algorithm(algo);
        test_ctx.cnx_client().set_congestion_algorithm(algo);
    }

    {
        let cnx = test_ctx.cnx_client();
        cnx.set_feedback_loss_notification(true);
        if spec.priority_limit_for_bypass > 0 {
            cnx.set_priority_limit_for_bypass(spec.priority_limit_for_bypass);
        }
        if spec.do_probe_up {
            cnx.request_forced_probe_up(true);
        }
    }

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;
    if !test_ctx.client_ready() || !test_ctx.server_ready() {
        return Err(crate::Error::Generic);
    }

    let mut senders = Vec::new();
    let mut recv_streams = Vec::new();
    {
        let cnx = test_ctx.cnx_client();
        if spec.data_size > 0 {
            let (sender, stream) = mediatest_configure_sender(cnx, MediaTestType::Data, spec)?;
            senders.push(sender);
            recv_streams.push(stream);
        }
        if spec.do_audio {
            let (sender, stream) = mediatest_configure_sender(cnx, MediaTestType::Audio, spec)?;
            senders.push(sender);
            recv_streams.push(stream);
        }
        if spec.do_video {
            let (sender, stream) = mediatest_configure_sender(cnx, MediaTestType::Video, spec)?;
            senders.push(sender);
            recv_streams.push(stream);
        }
        if spec.do_video2 {
            let (sender, stream) = mediatest_configure_sender(cnx, MediaTestType::Video2, spec)?;
            senders.push(sender);
            recv_streams.push(stream);
        }
    }
    let disruption_clear = if id == MediatestId::Worst {
        2_500_000
    } else {
        0
    };
    let runtime = Rc::new(RefCell::new(MediatestRuntime::new(
        recv_streams,
        disruption_clear,
    )));
    let mut is_finished = false;

    if id == MediatestId::Worst {
        mediatest_loop(
            &mut test_ctx,
            &runtime,
            &mut senders,
            &mut simulated_time,
            1_000_000,
            false,
            &mut is_finished,
        )?;
        mediatest_loop(
            &mut test_ctx,
            &runtime,
            &mut senders,
            &mut simulated_time,
            2_000_000,
            true,
            &mut is_finished,
        )?;
    } else if matches!(id, MediatestId::Video2Down | MediatestId::Video2Back) {
        let down_time = if id == MediatestId::Video2Down {
            4_000_000
        } else {
            2_000_000
        };
        let back_time = if id == MediatestId::Video2Down {
            24_000_000
        } else {
            4_000_000
        };
        mediatest_loop(
            &mut test_ctx,
            &runtime,
            &mut senders,
            &mut simulated_time,
            down_time,
            false,
            &mut is_finished,
        )?;

        let c_to_s_picosec_per_byte = test_ctx.c_to_s_link.picosec_per_byte;
        let s_to_c_picosec_per_byte = test_ctx.s_to_c_link.picosec_per_byte;
        let c_to_s_latency = test_ctx.c_to_s_link.microsec_latency;
        let s_to_c_latency = test_ctx.s_to_c_link.microsec_latency;
        test_ctx.c_to_s_link.picosec_per_byte = 8_000_000;
        test_ctx.s_to_c_link.picosec_per_byte = 8_000_000;

        mediatest_loop(
            &mut test_ctx,
            &runtime,
            &mut senders,
            &mut simulated_time,
            back_time,
            false,
            &mut is_finished,
        )?;

        test_ctx.c_to_s_link.picosec_per_byte = c_to_s_picosec_per_byte;
        test_ctx.s_to_c_link.picosec_per_byte = s_to_c_picosec_per_byte;
        test_ctx.c_to_s_link.microsec_latency = c_to_s_latency;
        test_ctx.s_to_c_link.microsec_latency = s_to_c_latency;
    } else if spec.nb_suspensions > 0 {
        mediatest_run_suspensions(
            &mut test_ctx,
            &runtime,
            &mut senders,
            &mut simulated_time,
            spec,
            &mut is_finished,
        )?;
    }

    mediatest_loop(
        &mut test_ctx,
        &runtime,
        &mut senders,
        &mut simulated_time,
        30_000_000,
        false,
        &mut is_finished,
    )?;
    mediatest_drain_receives(&mut test_ctx, &runtime, &senders, simulated_time.ticks())?;
    is_finished = mediatest_is_finished(&senders, &runtime);

    let runtime_ref = runtime.borrow();
    if !is_finished {
        return Err(crate::Error::Generic);
    }
    if spec.do_audio {
        mediatest_check_stats(&runtime_ref, spec, MediaTestType::Audio)?;
    }
    if spec.do_video {
        mediatest_check_stats(&runtime_ref, spec, MediaTestType::Video)?;
    }
    if spec.do_video2 && !spec.do_not_check_video2 {
        mediatest_check_stats(&runtime_ref, spec, MediaTestType::Video2)?;
    }
    drop(runtime_ref);

    if id == MediatestId::Wifi {
        let quality = test_ctx.cnx_client().default_path_quality();
        if quality.lost == 0 || quality.spurious_losses == 0 || quality.timer_losses == 0 {
            return Err(crate::Error::Generic);
        }
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0)
}

/// Run one media-test scenario.  C: `mediatest_one`.
///
/// Creates a custom simulation context (`mediatest_ctx_t` in C), configures
/// client/server QUIC contexts with the specified media streams, runs the
/// simulation loop, and checks that frame latencies meet `spec` bounds.
pub fn mediatest_one(id: MediatestId, spec: &MediatestSpec) -> crate::Result<()> {
    if matches!(
        id,
        MediatestId::VideoAudio
            | MediatestId::VideoDataAudio
            | MediatestId::Worst
            | MediatestId::Video2Down
            | MediatestId::Wifi
            | MediatestId::Video2Back
            | MediatestId::Video2Probe
            | MediatestId::Suspension
            | MediatestId::Suspension2
            | MediatestId::NoCoal
    ) {
        return mediatest_media_one(id, spec);
    }

    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut initial_cid = [0xed, 0x1a, 0x7e, 0x57, 0, 0, 0, 0];
    initial_cid[4] = id as u8;
    let initial_cid = ConnectionId::clone_from_slice(&initial_cid).ok_or(crate::Error::Generic)?;

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some("picoquic-mediatest"),
        None,
        Some(&initial_cid),
    )
    .ok_or(crate::Error::Generic)?;

    let link_latency = if spec.link_latency == 0 {
        10_000
    } else {
        spec.link_latency
    };
    let bandwidth = if spec.bandwidth > 0.0 {
        spec.bandwidth
    } else {
        0.01
    };
    for link in [&mut test_ctx.c_to_s_link, &mut test_ctx.s_to_c_link] {
        **link =
            super::util::TestSimLink::create(bandwidth, link_latency, None, 0, simulated_time)?;
    }

    if let Some(algo) = spec.ccalgo {
        test_ctx.qclient.set_default_congestion_algorithm(algo);
        test_ctx.qserver.set_default_congestion_algorithm(algo);
    }

    {
        let cnx = test_ctx.cnx_client();
        cnx.set_feedback_loss_notification(true);
        if spec.priority_limit_for_bypass > 0 {
            cnx.set_priority_limit_for_bypass(spec.priority_limit_for_bypass);
        }
        if spec.do_probe_up {
            cnx.request_forced_probe_up(true);
        }
    }

    let mut scenario = Vec::new();
    let mut next_stream_id = 4u64;
    if spec.do_audio {
        scenario.push(TestApiStreamDesc {
            stream_id: next_stream_id,
            previous_stream_id: 0,
            q_len: 48_000,
            r_len: 0,
        });
        next_stream_id += 4;
    }
    if spec.do_video {
        scenario.push(TestApiStreamDesc {
            stream_id: next_stream_id,
            previous_stream_id: 0,
            q_len: 800_000,
            r_len: 0,
        });
        next_stream_id += 4;
    }
    if spec.do_video2 {
        scenario.push(TestApiStreamDesc {
            stream_id: next_stream_id,
            previous_stream_id: 0,
            q_len: 1_600_000,
            r_len: 0,
        });
        next_stream_id += 4;
    }
    if spec.data_size > 0 {
        scenario.push(TestApiStreamDesc {
            stream_id: next_stream_id,
            previous_stream_id: 0,
            q_len: spec.data_size,
            r_len: 0,
        });
    }
    if scenario.is_empty() && spec.datagram_data_size == 0 {
        scenario.push(TestApiStreamDesc {
            stream_id: 4,
            previous_stream_id: 0,
            q_len: 1,
            r_len: 0,
        });
    }

    if id == MediatestId::Worst {
        loss_mask = u64::MAX;
    }

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 20_000, &mut simulated_time)?;
    test_api_init_send_recv_scenario(&mut test_ctx, &scenario)?;

    if matches!(id, MediatestId::Video2Down | MediatestId::Video2Back) {
        for link in [&mut test_ctx.c_to_s_link, &mut test_ctx.s_to_c_link] {
            link.picosec_per_byte = 8_000_000;
        }
    }

    if spec.nb_suspensions > 0 {
        let mut suspension_time = spec.suspension_start_time;
        for _ in 0..spec.nb_suspensions {
            suspension_time = suspension_time.saturating_add(spec.suspension_down_time);
            let resume = Instant::from_ticks(suspension_time);
            test_ctx.c_to_s_link.suspend(resume, false);
            test_ctx.s_to_c_link.suspend(resume, true);
            suspension_time = suspension_time.saturating_add(spec.suspension_up_time);
        }
    }

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)?;

    let completion_bound = spec.latency_max.max(spec.latency_average);
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, completion_bound)
}

// ---------------------------------------------------------------------------
// Test entries.

/// Basic video-only media test.  C: `mediatest_video_test`.
#[test]
fn mediatest_video() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video, &spec).expect("mediatest_video");
}

/// Video + audio media test.  C: `mediatest_video_audio_test`.
#[test]
fn mediatest_video_audio() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::VideoAudio, &spec).expect("mediatest_video_audio");
}

/// Video + data + audio media test.  C: `mediatest_video_data_audio_test`.
#[test]
fn mediatest_video_data_audio() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::VideoDataAudio, &spec).expect("mediatest_video_data_audio");
}

/// Video + video2 + audio with bandwidth drop-and-recover.
/// C: `mediatest_video2_down_test`.
#[test]
fn mediatest_video2_down() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 100_000,
        latency_max: 600_000,
        do_not_check_video2: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Down, &spec).expect("mediatest_video2_down");
}

/// Video + video2 + audio with bandwidth drop-and-back.
/// C: `mediatest_video2_back_test`.
#[test]
fn mediatest_video2_back() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 80_000,
        latency_max: 500_000,
        do_not_check_video2: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Back, &spec).expect("mediatest_video2_back");
}

/// Video + video2 + audio with probe-up.  C: `mediatest_video2_probe_test`.
#[test]
fn mediatest_video2_probe() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 25_000,
        latency_max: 150_000,
        do_probe_up: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Probe, &spec).expect("mediatest_video2_probe");
}

/// Wi-Fi jitter test with periodic suspension intervals.
/// C: `mediatest_wifi_test`.
#[test]
fn mediatest_wifi() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        link_latency: 15_000,
        latency_average: 60_000,
        latency_max: 350_000,
        priority_limit_for_bypass: 5,
        do_not_check_video2: true,
        nb_suspensions: 20,
        suspension_start_time: 4_000_000,
        suspension_down_time: 150_000,
        suspension_up_time: 0,
        ..Default::default()
    };
    mediatest_one(MediatestId::Wifi, &spec).expect("mediatest_wifi");
}

/// Worst-case video + data + audio test.  C: `mediatest_worst_test`.
#[test]
fn mediatest_worst() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::Worst, &spec).expect("mediatest_worst");
}

/// Video + data + audio with stream coalescing disabled.
/// C: `mediatest_no_coal_test`.
#[test]
fn mediatest_no_coal() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        no_coal: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::NoCoal, &spec).expect("mediatest_no_coal");
}

/// Video + video2 + audio with a single link suspension.
/// C: `mediatest_suspension_test`.
#[test]
fn mediatest_suspension() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 50_000,
        latency_max: 300_000,
        do_not_check_video2: true,
        nb_suspensions: 1,
        suspension_start_time: 4_000_000,
        suspension_down_time: 150_000,
        suspension_up_time: 50_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::Suspension, &spec).expect("mediatest_suspension");
}

/// Video + video2 + audio with suspension and probe-up.
/// C: `mediatest_suspension2_test`.
#[test]
fn mediatest_suspension2() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 50_000,
        latency_max: 300_000,
        do_not_check_video2: true,
        do_probe_up: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Suspension2, &spec).expect("mediatest_suspension2");
}
