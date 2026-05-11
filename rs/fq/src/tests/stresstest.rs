//! Test cases for `picoquictest/stresstest.c`.
//!
//! Exercises the random-number generator, the Gaussian distribution
//! helper, and the full stress / fuzz simulation loop.

#![allow(non_snake_case)]

use std::{
    cell::RefCell,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    rc::Rc,
};

use super::util::{
    TEST_ALPN, TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_KEY, TEST_SNI, TestSimLink, TestSimPacket,
    save_empty_tickets, test_gauss_random, test_random, test_uniform_random,
};
use crate::errors::TransportError;
use crate::frames::FrameType;
use crate::{
    CallbackEvent, Connection, ConnectionId, Error, Instant, PacketContext, Quic,
    RESET_SECRET_SIZE, State, StreamDataCallback, current_time,
};

// ---------------------------------------------------------------------------
// Stress / fuzz harness.
// C: `stress_or_fuzz_test` — drives `duration` µs of simulated time with
// a set of client wake times ordered by the next simulated action.

const STRESS_NB_CLIENTS: usize = 4;
const STRESS_MAX_CLIENTS: u32 = 1024;
const STRESS_MAX_TRACKED_STREAMS: usize = 16;
const STRESS_MINIMAL_QUERY_SIZE: usize = 127;
const STRESS_DEFAULT_RESPONSE_SIZE: usize = 257;
const STRESS_RESPONSE_LENGTH_MAX: u64 = 1_000_000;
const STRESS_MESSAGE_BUFFER_SIZE: usize = 0x10000;
const STRESS_MAX_CLIENT_STREAMS: usize = 16;
const STRESS_MAX_BIDIR: u64 = 8 * 4;
const STRESS_MAX_OPEN_STREAMS: u64 = 4;
const STRESS_MAX_MESSAGE_BEFORE_DROP: u32 = 25;
const STRESS_MAX_MESSAGE_BEFORE_MIGRATE: u32 = 8;
const STRESS_TICKET_ENCRYPT_KEY: [u8; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];

#[derive(Debug)]
struct BasicFuzzer {
    nb_packets: u32,
    nb_fuzzed: u32,
    nb_fuzzed_length: u32,
    random_context: u64,
    highest_state_fuzzed: crate::State,
}

impl BasicFuzzer {
    fn new(duration: u64) -> Self {
        Self {
            nb_packets: 0,
            nb_fuzzed: 0,
            nb_fuzzed_length: 0,
            random_context: 0xDEADBEEFBABACAFE_u64 ^ duration,
            highest_state_fuzzed: crate::State::ClientInit,
        }
    }

    fn fuzz_state(
        &mut self,
        connection_state: crate::State,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32 {
        let mut length = length.min(bytes.len());
        let header_length = header_length.min(bytes.len());
        let mut fuzz_pilot = test_random(&mut self.random_context);
        let should_fuzz;

        self.nb_packets = self.nb_packets.wrapping_add(1);

        if connection_state > self.highest_state_fuzzed {
            should_fuzz = true;
            self.highest_state_fuzzed = connection_state;
        } else {
            should_fuzz = (fuzz_pilot & 0xF) == 0xD;
            fuzz_pilot >>= 4;
        }

        if should_fuzz && length != 0 {
            if (fuzz_pilot & 0xF) == 0xD {
                let fuzz_length_max = (length + 16).min(bytes.len());
                fuzz_pilot >>= 4;
                let fuzzed_length =
                    (16 + ((fuzz_pilot & 0xFFFF) as usize % fuzz_length_max)).min(bytes.len());
                fuzz_pilot >>= 16;
                if fuzzed_length > length {
                    bytes[length..fuzzed_length].fill(fuzz_pilot as u8);
                }
                length = fuzzed_length.max(header_length);
                self.nb_fuzzed_length = self.nb_fuzzed_length.wrapping_add(1);
            }

            let mut fuzz_index = (fuzz_pilot & 0xFFFF) as usize % length;
            fuzz_pilot >>= 16;
            while fuzz_pilot != 0 && fuzz_index < length {
                bytes[fuzz_index] = (fuzz_pilot & 0xFF) as u8;
                fuzz_index += 1;
                fuzz_pilot >>= 8;
                self.nb_fuzzed = self.nb_fuzzed.wrapping_add(1);
            }
        }

        length as u32
    }
}

impl crate::Fuzz for BasicFuzzer {
    fn fuzz(
        &mut self,
        connection: &mut crate::internal::Connection,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32 {
        self.fuzz_state(connection.state(), bytes, length, header_length)
    }
}

const INITIAL_SKIP_FRAMES: &[&[u8]] = &[
    &[0, 0, 0],
    &[FrameType::ResetStream as u8, 17, 1, 1],
    &[
        FrameType::ConnectionClose as u8,
        0x80,
        0x00,
        0xcf,
        0xff,
        0,
        9,
        b'1',
        b'2',
        b'3',
        b'4',
        b'5',
        b'6',
        b'7',
        b'8',
        b'9',
    ],
    &[FrameType::ApplicationClose as u8, 0, 0],
    &[
        FrameType::ApplicationClose as u8,
        0x44,
        4,
        4,
        b't',
        b'e',
        b's',
        b't',
    ],
    &[FrameType::MaxData as u8, 0xc0, 0, 0x01, 0, 0, 0, 0, 0],
    &[FrameType::MaxStreamData as u8, 1, 0x80, 0x01, 0, 0],
    &[FrameType::MaxStreamsBidir as u8, 0x41, 0],
    &[FrameType::MaxStreamsUnidir as u8, 0x41, 7],
    &[FrameType::Ping as u8],
    &[FrameType::DataBlocked as u8, 0x80, 0x01, 0, 0],
    &[
        FrameType::StreamDataBlocked as u8,
        0x80,
        1,
        0,
        0,
        0x80,
        0x02,
        0,
        0,
    ],
    &[FrameType::StreamsBlockedBidir as u8, 0x41, 0x00],
    &[FrameType::StreamsBlockedUnidir as u8, 0x42, 0x00],
    &[
        FrameType::NewConnectionId as u8,
        7,
        0,
        8,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        0xa0,
        0xa1,
        0xa2,
        0xa3,
        0xa4,
        0xa5,
        0xa6,
        0xa7,
        0xa8,
        0xa9,
        0xaa,
        0xab,
        0xac,
        0xad,
        0xae,
        0xaf,
    ],
    &[FrameType::StopSending as u8, 17, 0x17],
    &[FrameType::PathChallenge as u8, 1, 2, 3, 4, 5, 6, 7, 8],
    &[FrameType::PathResponse as u8, 1, 2, 3, 4, 5, 6, 7, 8],
    &[
        FrameType::NewToken as u8,
        17,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
    ],
    &[
        FrameType::Ack as u8,
        0xc0,
        0,
        0,
        1,
        2,
        3,
        4,
        5,
        0x44,
        0,
        2,
        5,
        0,
        0,
        5,
        12,
    ],
    &[
        FrameType::AckEcn as u8,
        0xc0,
        0,
        0,
        1,
        2,
        3,
        4,
        5,
        0x44,
        0,
        2,
        5,
        0,
        0,
        5,
        12,
        3,
        0,
        1,
    ],
    &[
        FrameType::StreamRangeMin as u8,
        1,
        0xa0,
        0xa1,
        0xa2,
        0xa3,
        0xa4,
        0xa5,
        0xa6,
        0xa7,
        0xa8,
        0xa9,
        0xaa,
        0xab,
        0xac,
        0xad,
        0xae,
        0xaf,
    ],
    &[
        FrameType::StreamRangeMin as u8 + 2 + 4,
        1,
        0x44,
        0,
        0x10,
        0xa0,
        0xa1,
        0xa2,
        0xa3,
        0xa4,
        0xa5,
        0xa6,
        0xa7,
        0xa8,
        0xa9,
        0xaa,
        0xab,
        0xac,
        0xad,
        0xae,
        0xaf,
    ],
    &[
        FrameType::CryptoHs as u8,
        0,
        0x10,
        0xa0,
        0xa1,
        0xa2,
        0xa3,
        0xa4,
        0xa5,
        0xa6,
        0xa7,
        0xa8,
        0xa9,
        0xaa,
        0xab,
        0xac,
        0xad,
        0xae,
        0xaf,
    ],
    &[FrameType::RetireConnectionId as u8, 1],
    &[
        FrameType::Datagram as u8,
        0xa0,
        0xa1,
        0xa2,
        0xa3,
        0xa4,
        0xa5,
        0xa6,
        0xa7,
        0xa8,
        0xa9,
        0xaa,
        0xab,
        0xac,
        0xad,
        0xae,
        0xaf,
    ],
    &[
        FrameType::DatagramL as u8,
        0x10,
        0xa0,
        0xa1,
        0xa2,
        0xa3,
        0xa4,
        0xa5,
        0xa6,
        0xa7,
        0xa8,
        0xa9,
        0xaa,
        0xab,
        0xac,
        0xad,
        0xae,
        0xaf,
    ],
    &[FrameType::HandshakeDone as u8],
    &[
        0x40,
        FrameType::AckFrequency as u8,
        17,
        0x0a,
        0x44,
        0x20,
        0x00,
    ],
    &[
        0x40,
        FrameType::AckFrequency as u8,
        17,
        0x0a,
        0x44,
        0x20,
        0x40,
        0x05,
    ],
    &[FrameType::ImmediateAck as u8],
    &[
        0x40u8 | ((FrameType::TimeStamp as u64 >> 8) as u8),
        (FrameType::TimeStamp as u64 & 0xff) as u8,
        0x44,
        0,
    ],
    &[
        0x40u8 | ((FrameType::PathAbandon as u64 >> 8) as u8),
        (FrameType::PathAbandon as u64 & 0xff) as u8,
        0x01,
        0x00,
    ],
    &[
        0x40u8 | ((FrameType::PathAbandon as u64 >> 8) as u8),
        (FrameType::PathAbandon as u64 & 0xff) as u8,
        0x01,
        0x11,
    ],
    &[
        0x40u8 | ((FrameType::PathBackup as u64 >> 8) as u8),
        (FrameType::PathBackup as u64 & 0xff) as u8,
        0x00,
        0x0f,
    ],
    &[
        0x40u8 | ((FrameType::PathAvailable as u64 >> 8) as u8),
        (FrameType::PathAvailable as u64 & 0xff) as u8,
        0x00,
        0x0f,
    ],
    &[
        0x40u8 | ((FrameType::MaxPathId as u64 >> 8) as u8),
        (FrameType::MaxPathId as u64 & 0xff) as u8,
        0x11,
    ],
    &[
        0x40u8 | ((FrameType::PathNewConnectionId as u64 >> 8) as u8),
        (FrameType::PathNewConnectionId as u64 & 0xff) as u8,
        1,
        7,
        0,
        8,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        0xa1,
        0xa2,
        0xa3,
        0xa4,
        0xa5,
        0xa6,
        0xa7,
        0xa8,
        0xa9,
        0xaa,
        0xab,
        0xac,
        0xad,
        0xae,
        0xaf,
        0xb0,
    ],
    &[
        0x40u8 | ((FrameType::PathRetireConnectionId as u64 >> 8) as u8),
        (FrameType::PathRetireConnectionId as u64 & 0xff) as u8,
        0,
        2,
    ],
    &[
        0x40u8 | ((FrameType::PathsBlocked as u64 >> 8) as u8),
        (FrameType::PathsBlocked as u64 & 0xff) as u8,
        0x11,
    ],
    &[
        0x40u8 | ((FrameType::PathCidBlocked as u64 >> 8) as u8),
        (FrameType::PathCidBlocked as u64 & 0xff) as u8,
        0x07,
        0x01,
    ],
    &[
        0x80u8 | ((FrameType::Bdp as u64 >> 24) as u8),
        (FrameType::Bdp as u64 >> 16) as u8,
        (FrameType::Bdp as u64 >> 8) as u8,
        (FrameType::Bdp as u64 & 0xff) as u8,
        0x01,
        0x02,
        0x03,
        0x04,
        0x0a,
        0x0,
        0x0,
        0x01,
    ],
    &[
        0x80u8 | ((FrameType::ObservedAddressV4 as u64 >> 24) as u8),
        (FrameType::ObservedAddressV4 as u64 >> 16) as u8,
        (FrameType::ObservedAddressV4 as u64 >> 8) as u8,
        (FrameType::ObservedAddressV4 as u64 & 0xff) as u8,
        1,
        0x1,
        0x2,
        0x3,
        0x4,
        0x12,
        0x34,
    ],
    &[
        0x80u8 | ((FrameType::ObservedAddressV6 as u64 >> 24) as u8),
        (FrameType::ObservedAddressV6 as u64 >> 16) as u8,
        (FrameType::ObservedAddressV6 as u64 >> 8) as u8,
        (FrameType::ObservedAddressV6 as u64 & 0xff) as u8,
        2,
        0x1,
        0x2,
        0x3,
        0x4,
        0x5,
        0x6,
        0x7,
        0x8,
        0x9,
        0xa,
        0xb,
        0xc,
        0xd,
        0xe,
        0xf,
        0x0,
        0x45,
        0x67,
    ],
    &[
        FrameType::PathAck as u8,
        0,
        0xc0,
        0,
        0,
        1,
        2,
        3,
        4,
        5,
        0x44,
        0,
        2,
        5,
        0,
        0,
        5,
        12,
    ],
    &[
        FrameType::PathAckEcn as u8,
        0,
        0xc0,
        0,
        0,
        1,
        2,
        3,
        4,
        5,
        0x44,
        0,
        2,
        5,
        0,
        0,
        5,
        12,
        3,
        0,
        1,
    ],
    &[FrameType::ResetStreamAt as u8, 17, 1, 0x40, 128, 13],
];

#[derive(Debug)]
struct InitialFuzzer {
    current_frame: usize,
    fuzz_position: u32,
    initial_fuzzing_done: bool,
    random_context: u64,
    initial_packets: u32,
    append_count: u32,
    prepend_count: u32,
    replace_count: u32,
    random_count: u32,
    random_bytes: u32,
}

impl InitialFuzzer {
    fn new(duration: u64) -> Self {
        Self {
            current_frame: 0,
            fuzz_position: 0,
            initial_fuzzing_done: false,
            random_context: 0x0123_4567_DEAD_BEEF_u64 ^ duration,
            initial_packets: 0,
            append_count: 0,
            prepend_count: 0,
            replace_count: 0,
            random_count: 0,
            random_bytes: 0,
        }
    }

    fn frame_count(&self) -> u32 {
        INITIAL_SKIP_FRAMES.len() as u32
    }

    fn fuzz_state(
        &mut self,
        connection_state: crate::State,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32 {
        let bytes_max = bytes.len();
        let mut length = length.min(bytes_max);
        let header_length = header_length.min(length);

        if connection_state != crate::State::ClientInitSent {
            return length as u32;
        }

        self.initial_packets = self.initial_packets.wrapping_add(1);

        if !self.initial_fuzzing_done && self.current_frame >= INITIAL_SKIP_FRAMES.len() {
            self.fuzz_position = self.fuzz_position.wrapping_add(1);
            self.current_frame = 0;

            if self.fuzz_position > 2 {
                self.fuzz_position = 0;
                self.initial_fuzzing_done = true;
            }
        }

        if !self.initial_fuzzing_done {
            let frame = INITIAL_SKIP_FRAMES[self.current_frame];
            let frame_len = frame.len();

            if length + frame_len <= bytes_max {
                match self.fuzz_position {
                    0 => {
                        bytes[length..length + frame_len].copy_from_slice(frame);
                        length += frame_len;
                        self.append_count = self.append_count.wrapping_add(1);
                    }
                    1 => {
                        let source_end = header_length + frame_len;
                        let target_start = header_length + frame_len;
                        let target_end = target_start + frame_len;
                        if target_end <= bytes_max && source_end <= bytes_max {
                            bytes.copy_within(header_length..source_end, target_start);
                            bytes[header_length..source_end].copy_from_slice(frame);
                            length += frame_len;
                            self.prepend_count = self.prepend_count.wrapping_add(1);
                        }
                    }
                    2 => {
                        let frame_end = header_length + frame_len;
                        if frame_end <= bytes_max {
                            bytes[header_length..frame_end].copy_from_slice(frame);

                            if length > frame_end {
                                bytes[frame_end..length].fill(0);
                            } else {
                                length = frame_end;
                            }
                            self.replace_count = self.replace_count.wrapping_add(1);
                        }
                    }
                    _ => {}
                }
            }

            self.current_frame += 1;
        } else if length != 0 {
            let mut fuzz_pilot = test_random(&mut self.random_context);
            let mut fuzz_index = (fuzz_pilot & 0xFFFF) as usize % length;
            fuzz_pilot >>= 16;
            let mut fuzz_length = ((fuzz_pilot & 0xFF) as u8 % 5) + 1;
            fuzz_pilot >>= 8;
            self.random_count = self.random_count.wrapping_add(1);

            while fuzz_length != 0 && fuzz_index < length {
                bytes[fuzz_index] = (fuzz_pilot & 0xFF) as u8;
                fuzz_index += 1;
                fuzz_pilot >>= 8;
                fuzz_length -= 1;
                self.random_bytes = self.random_bytes.wrapping_add(1);
            }
        }

        length as u32
    }
}

impl crate::Fuzz for InitialFuzzer {
    fn fuzz(
        &mut self,
        connection: &mut crate::internal::Connection,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32 {
        self.fuzz_state(connection.state(), bytes, length, header_length)
    }
}

#[derive(Clone)]
enum StressFuzzerConfig {
    Basic(Rc<RefCell<BasicFuzzer>>),
    Initial(Rc<RefCell<InitialFuzzer>>),
}

impl StressFuzzerConfig {
    fn boxed(&self) -> Box<dyn crate::Fuzz> {
        match self {
            Self::Basic(inner) => Box::new(SharedBasicFuzzer {
                inner: Rc::clone(inner),
            }),
            Self::Initial(inner) => Box::new(SharedInitialFuzzer {
                inner: Rc::clone(inner),
            }),
        }
    }
}

struct SharedBasicFuzzer {
    inner: Rc<RefCell<BasicFuzzer>>,
}

impl crate::Fuzz for SharedBasicFuzzer {
    fn fuzz(
        &mut self,
        connection: &mut Connection,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32 {
        self.inner
            .borrow_mut()
            .fuzz_state(connection.state(), bytes, length, header_length)
    }
}

struct SharedInitialFuzzer {
    inner: Rc<RefCell<InitialFuzzer>>,
}

impl crate::Fuzz for SharedInitialFuzzer {
    fn fuzz(
        &mut self,
        connection: &mut Connection,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32 {
        self.inner
            .borrow_mut()
            .fuzz_state(connection.state(), bytes, length, header_length)
    }
}

#[derive(Default)]
struct StressShared {
    sum_data_received_from_server: i32,
    nb_connections_complete: i32,
}

struct StressClientControl {
    message_disconnect_trigger: u32,
    message_migration_trigger: u32,
}

struct StressServerCallback {
    data_received_on_stream: [usize; STRESS_MAX_TRACKED_STREAMS],
    data_sum_of_stream: [u32; STRESS_MAX_TRACKED_STREAMS],
    buffer: [u8; STRESS_MESSAGE_BUFFER_SIZE],
    is_default: bool,
}

impl StressServerCallback {
    fn new(is_default: bool) -> Self {
        Self {
            data_received_on_stream: [0; STRESS_MAX_TRACKED_STREAMS],
            data_sum_of_stream: [0; STRESS_MAX_TRACKED_STREAMS],
            buffer: [0; STRESS_MESSAGE_BUFFER_SIZE],
            is_default,
        }
    }

    fn handle_event(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
    ) -> i32 {
        match fin_or_event {
            CallbackEvent::Close
            | CallbackEvent::StatelessReset
            | CallbackEvent::ApplicationClose => {
                connection.set_callback(None);
                0
            }
            CallbackEvent::VersionNegotiation
            | CallbackEvent::AlmostReady
            | CallbackEvent::Ready => 0,
            CallbackEvent::PrepareToSend => -1,
            CallbackEvent::StopSending | CallbackEvent::StreamReset => {
                connection.reset_stream(stream_id, 0).map_or(-1, |_| 0)
            }
            CallbackEvent::StreamData | CallbackEvent::StreamFin => {
                self.handle_stream_data(connection, stream_id, bytes, fin_or_event)
            }
            _ => connection
                .reset_stream(stream_id, TransportError::ProtocolViolation as u64)
                .map_or(-1, |_| 0),
        }
    }

    fn handle_stream_data(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
    ) -> i32 {
        if (stream_id & 3) != 0 {
            return 0;
        }

        let bidir_id = (stream_id / 4) as usize;
        let mut response_length = 0usize;

        if bidir_id < STRESS_MAX_TRACKED_STREAMS {
            let previous = self.data_received_on_stream[bidir_id];
            let received = previous.saturating_add(bytes.len());
            if previous < STRESS_MINIMAL_QUERY_SIZE {
                let mut processed = bytes.len();
                if received >= STRESS_MINIMAL_QUERY_SIZE {
                    processed = received - STRESS_MINIMAL_QUERY_SIZE;
                }
                for byte in bytes.iter().take(processed) {
                    self.data_sum_of_stream[bidir_id] = self.data_sum_of_stream[bidir_id]
                        .wrapping_mul(101)
                        .wrapping_add(u32::from(*byte));
                }
                if received >= STRESS_MINIMAL_QUERY_SIZE {
                    response_length = (u64::from(self.data_sum_of_stream[bidir_id])
                        % STRESS_RESPONSE_LENGTH_MAX)
                        as usize;
                }
            }
            self.data_received_on_stream[bidir_id] = received;
        }

        if fin_or_event == CallbackEvent::StreamFin
            && (bidir_id >= STRESS_MAX_TRACKED_STREAMS
                || self.data_received_on_stream[bidir_id] < STRESS_MINIMAL_QUERY_SIZE)
        {
            response_length = STRESS_DEFAULT_RESPONSE_SIZE;
        }

        while response_length > STRESS_MESSAGE_BUFFER_SIZE {
            if connection
                .add_to_stream(stream_id, &self.buffer, false)
                .is_err()
            {
                return -1;
            }
            response_length -= STRESS_MESSAGE_BUFFER_SIZE;
        }

        if response_length > 0
            && connection
                .add_to_stream(stream_id, &self.buffer[..response_length], true)
                .is_err()
        {
            return -1;
        }

        0
    }
}

impl StreamDataCallback for StressServerCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        bytes: &[u8],
        fin_or_event: CallbackEvent,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        if self.is_default {
            match fin_or_event {
                CallbackEvent::Close
                | CallbackEvent::StatelessReset
                | CallbackEvent::ApplicationClose
                | CallbackEvent::VersionNegotiation
                | CallbackEvent::AlmostReady
                | CallbackEvent::Ready => 0,
                _ => {
                    let mut per_connection = StressServerCallback::new(false);
                    let ret =
                        per_connection.handle_event(connection, stream_id, bytes, fin_or_event);
                    if ret == 0 {
                        connection.set_callback(Some(Box::new(per_connection)));
                    }
                    ret
                }
            }
        } else {
            self.handle_event(connection, stream_id, bytes, fin_or_event)
        }
    }
}

struct StressClientCallback {
    shared: Rc<RefCell<StressShared>>,
    test_id: u64,
    max_bidir: u64,
    next_bidir: u64,
    max_open_streams: usize,
    nb_open_streams: usize,
    stream_id: [u64; STRESS_MAX_CLIENT_STREAMS],
    last_interaction_time: u64,
    nb_client_streams: u32,
    progress_observed: bool,
}

impl StressClientCallback {
    fn new(shared: Rc<RefCell<StressShared>>, test_id: u64) -> Self {
        Self {
            shared,
            test_id,
            max_bidir: STRESS_MAX_BIDIR,
            next_bidir: 4,
            max_open_streams: STRESS_MAX_OPEN_STREAMS as usize,
            nb_open_streams: 0,
            stream_id: [u64::MAX; STRESS_MAX_CLIENT_STREAMS],
            last_interaction_time: 0,
            nb_client_streams: 0,
            progress_observed: false,
        }
    }

    fn new_control(random_ctx: &mut u64) -> Rc<RefCell<StressClientControl>> {
        fn trigger(random_ctx: &mut u64, max_before: u32) -> u32 {
            let value = test_uniform_random(random_ctx, u64::from(max_before) * 2) as u32;
            if value >= max_before { 0 } else { value + 1 }
        }

        Rc::new(RefCell::new(StressClientControl {
            message_disconnect_trigger: trigger(random_ctx, STRESS_MAX_MESSAGE_BEFORE_DROP),
            message_migration_trigger: trigger(random_ctx, STRESS_MAX_MESSAGE_BEFORE_MIGRATE),
        }))
    }

    fn prepare_streams(&mut self) -> crate::Result<Vec<(u64, [u8; 32])>> {
        let mut streams = Vec::new();
        while self.nb_open_streams < self.max_open_streams && self.next_bidir <= self.max_bidir {
            let Some(stream_index) = self
                .stream_id
                .iter()
                .take(self.max_open_streams)
                .position(|&id| id == u64::MAX)
            else {
                return Err(Error::InvalidState);
            };

            let stream_id = self.next_bidir;
            let mut buf = [0u8; 32];
            buf[..8].copy_from_slice(&self.test_id.to_be_bytes());
            buf[8..16].copy_from_slice(&stream_id.to_be_bytes());

            self.stream_id[stream_index] = stream_id;
            self.next_bidir += 4;
            self.nb_open_streams += 1;
            self.nb_client_streams = self.nb_client_streams.saturating_add(1);
            streams.push((stream_id, buf));
        }
        Ok(streams)
    }

    fn start_streams(&mut self, connection: &mut Connection) -> crate::Result<()> {
        for (stream_id, buf) in self.prepare_streams()? {
            connection.add_to_stream(stream_id, &buf, true)?;
        }
        Ok(())
    }
}

impl StreamDataCallback for StressClientCallback {
    fn callback(
        &mut self,
        connection: &mut Connection,
        stream_id: u64,
        _bytes: &[u8],
        length_or_event: CallbackEvent,
        _stream_ctx: Option<&mut dyn core::any::Any>,
    ) -> i32 {
        match length_or_event {
            CallbackEvent::VersionNegotiation | CallbackEvent::AlmostReady => 0,
            CallbackEvent::Close
            | CallbackEvent::ApplicationClose
            | CallbackEvent::StatelessReset => {
                connection.set_callback(None);
                0
            }
            CallbackEvent::Ready => {
                self.shared.borrow_mut().nb_connections_complete += 1;
                0
            }
            _ => {
                self.last_interaction_time = current_time();
                self.progress_observed = true;
                let Some(stream_index) = self
                    .stream_id
                    .iter()
                    .take(self.max_open_streams)
                    .position(|&id| id == stream_id)
                else {
                    return 0;
                };

                if matches!(
                    length_or_event,
                    CallbackEvent::StreamData | CallbackEvent::StreamFin
                ) {
                    self.shared.borrow_mut().sum_data_received_from_server += _bytes.len() as i32;
                }

                let mut is_finished = false;
                match length_or_event {
                    CallbackEvent::StreamReset | CallbackEvent::StopSending => {
                        if connection.reset_stream(stream_id, 0).is_err() {
                            return -1;
                        }
                        is_finished = true;
                    }
                    CallbackEvent::StreamFin => is_finished = true,
                    _ => {}
                }

                if is_finished {
                    if self.nb_open_streams == 0 {
                        return -1;
                    }
                    self.nb_open_streams -= 1;
                    self.stream_id[stream_index] = u64::MAX;
                    if self.next_bidir >= self.max_bidir {
                        if self.nb_open_streams == 0 && connection.close(0).is_err() {
                            return -1;
                        }
                    } else if self.start_streams(connection).is_err() {
                        return -1;
                    }
                }
                0
            }
        }
    }
}

struct StressClientContext {
    qclient: Box<Quic>,
    client_addr: SocketAddr,
    client_control: Option<Rc<RefCell<StressClientControl>>>,
    c_to_s_link: Box<TestSimLink>,
    s_to_c_link: Box<TestSimLink>,
    client_next_time: u64,
}

struct StressCtx {
    shared: Rc<RefCell<StressShared>>,
    qserver: Box<Quic>,
    server_addr: SocketAddr,
    simulated_time: u64,
    nb_connections: u64,
    next_test_id: u64,
    random_ctx: u64,
    clients: Vec<StressClientContext>,
}

fn stress_addr_from_index(index: i32) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::from(index as u32)), 4321)
}

fn stress_index_from_addr(addr: SocketAddr) -> Option<usize> {
    match addr.ip() {
        IpAddr::V4(ip) => {
            let index = u32::from(ip) as usize;
            (index < STRESS_NB_CLIENTS).then_some(index)
        }
        IpAddr::V6(_) => None,
    }
}

fn stress_client_eval_next_time(client: &mut StressClientContext, simulated_time: u64) -> u64 {
    if client.qclient.current_number_connections() == 0 {
        return simulated_time;
    }

    let now = Instant::from_ticks(simulated_time);
    let delay = client.qclient.next_wake_delay(now, 100_000_000).max(0) as u64;
    let mut best = simulated_time.saturating_add(delay);
    if let Some(packet) = client.s_to_c_link.packets.front() {
        best = best.min(packet.arrival_time.ticks());
    }
    if let Some(packet) = client.c_to_s_link.packets.front() {
        best = best.min(packet.arrival_time.ticks());
    }
    best
}

fn stress_create_client_context(
    client_index: usize,
    random_ctx: &mut u64,
    simulated_time: u64,
    fuzzer: Option<&StressFuzzerConfig>,
) -> crate::Result<StressClientContext> {
    let ticket_file_name = format!("stress_ticket_{client_index:03}.bin");
    save_empty_tickets(&ticket_file_name, Instant::from_ticks(simulated_time))?;

    const TARGET_BANDWIDTH: [f64; 4] = [0.001, 0.01, 0.03, 0.1];
    let random_latency = 1_000 + test_uniform_random(random_ctx, 99_000);
    let bandwidth_index = test_uniform_random(random_ctx, 4) as usize;
    let bandwidth = TARGET_BANDWIDTH[bandwidth_index];
    let now = Instant::from_ticks(simulated_time);
    let c_to_s_link = Box::new(TestSimLink::create(
        bandwidth,
        random_latency,
        None,
        2 * random_latency,
        now,
    )?);
    let s_to_c_link = Box::new(TestSimLink::create(
        bandwidth,
        random_latency,
        None,
        2 * random_latency,
        now,
    )?);

    let mut qclient = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        now,
        Some(&ticket_file_name),
        None,
    )
    .ok_or(Error::Memory)?;
    if let Some(fuzzer) = fuzzer {
        qclient.set_fuzz(Some(fuzzer.boxed()));
    }

    Ok(StressClientContext {
        qclient,
        client_addr: stress_addr_from_index(client_index as i32),
        client_control: None,
        c_to_s_link,
        s_to_c_link,
        client_next_time: simulated_time,
    })
}

fn stress_create_ctx(fuzzer: Option<StressFuzzerConfig>) -> crate::Result<StressCtx> {
    if STRESS_NB_CLIENTS > STRESS_MAX_CLIENTS as usize {
        return Err(Error::InvalidArgument);
    }

    let simulated_time = 0;
    let shared = Rc::new(RefCell::new(StressShared::default()));
    let default_cb = Box::new(StressServerCallback::new(true));
    let qserver = Quic::new(
        STRESS_MAX_CLIENTS,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        None,
        Some(TEST_ALPN),
        Some(default_cb),
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(simulated_time),
        None,
        Some(&STRESS_TICKET_ENCRYPT_KEY),
    )
    .ok_or(Error::Memory)?;

    let mut random_ctx = 0xBABAC001BADDBAB1_u64;
    let mut clients = Vec::with_capacity(STRESS_NB_CLIENTS);
    for i in 0..STRESS_NB_CLIENTS {
        clients.push(stress_create_client_context(
            i,
            &mut random_ctx,
            simulated_time,
            fuzzer.as_ref(),
        )?);
    }

    Ok(StressCtx {
        shared,
        qserver,
        server_addr: stress_addr_from_index(-1),
        simulated_time,
        nb_connections: 0,
        next_test_id: 0,
        random_ctx,
        clients,
    })
}

fn stress_packet_from_stateless(
    sp: crate::internal::StatelessPacket,
) -> crate::Result<TestSimPacket> {
    let mut packet = TestSimPacket::create()?;
    packet.length = sp.length;
    packet.addr_from = Some(sp.addr_local);
    packet.addr_to = Some(sp.addr_to);
    packet.ecn_mark = sp.received_ecn;
    packet.bytes[..sp.length].copy_from_slice(&sp.bytes[..sp.length]);
    Ok(packet)
}

fn stress_submit_sp_packets_client(
    client: &mut StressClientContext,
    current_time: Instant,
) -> crate::Result<()> {
    while let Some(sp) = client.qclient.dequeue_stateless_packet() {
        if sp.length > 0 {
            let packet = stress_packet_from_stateless(sp)?;
            client.c_to_s_link.submit(packet, current_time);
        }
    }
    Ok(())
}

fn stress_submit_sp_packets_server(
    qserver: &mut Quic,
    clients: &mut [StressClientContext],
    current_time: Instant,
) -> crate::Result<()> {
    while let Some(sp) = qserver.dequeue_stateless_packet() {
        if sp.length > 0 {
            let index = stress_index_from_addr(sp.addr_to).ok_or(Error::InvalidState)?;
            let packet = stress_packet_from_stateless(sp)?;
            clients[index].s_to_c_link.submit(packet, current_time);
            clients[index].client_next_time =
                stress_client_eval_next_time(&mut clients[index], current_time.ticks());
        }
    }
    Ok(())
}

fn stress_handle_packet_arrival(
    quic: &mut Quic,
    link: &mut TestSimLink,
    dest_addr: SocketAddr,
    current_time: Instant,
) -> crate::Result<()> {
    if let Some(mut packet) = link.dequeue(current_time)
        && packet.addr_to == Some(dest_addr)
    {
        quic.incoming_packet(
            &mut packet.bytes[..packet.length],
            &packet
                .addr_from
                .unwrap_or(SocketAddr::from(([0u8; 4], 0u16))),
            &packet.addr_to.unwrap_or(SocketAddr::from(([0u8; 4], 0u16))),
            0,
            packet.ecn_mark,
            current_time,
        )?;
    }
    Ok(())
}

fn stress_prepare_client_packet(
    client: &mut StressClientContext,
    server_addr: SocketAddr,
    current_time: Instant,
) -> crate::Result<()> {
    if stress_maybe_disconnect_client(client) {
        return Ok(());
    }
    stress_maybe_migrate_client(client, server_addr, current_time)?;

    let mut packet = TestSimPacket::create()?;
    let prepared = client
        .qclient
        .prepare_next_packet(current_time, &mut packet.bytes)?;
    if prepared.send_length > 0 {
        let addr_to = if prepared.addr_to.ip().is_unspecified() {
            server_addr
        } else {
            prepared.addr_to
        };
        let addr_from = if prepared.addr_from.ip().is_unspecified() {
            client.client_addr
        } else {
            prepared.addr_from
        };
        packet.length = prepared.send_length;
        packet.addr_to = Some(addr_to);
        packet.addr_from = Some(addr_from);
        client.c_to_s_link.submit(packet, current_time);
    }
    Ok(())
}

fn stress_maybe_disconnect_client(client: &mut StressClientContext) -> bool {
    let Some(control) = client.client_control.as_ref() else {
        return false;
    };
    let trigger = control.borrow().message_disconnect_trigger;
    if trigger == 0 {
        return false;
    }

    let token = {
        let Some(cnx) = client.qclient.earliest_cnx_to_wake(Instant::from_ticks(0)) else {
            return false;
        };
        let nb_sent = cnx.pkt_ctx.iter().fold(0u64, |sum, pkt_ctx| {
            sum.saturating_add(pkt_ctx.send_sequence)
        });
        (cnx.state() != State::Disconnected && nb_sent > u64::from(trigger))
            .then_some(cnx.own_token)
            .flatten()
    };

    if let Some(token) = token {
        client.qclient.delete_connection(token);
        client.client_control = None;
        true
    } else {
        false
    }
}

fn stress_maybe_migrate_client(
    client: &mut StressClientContext,
    server_addr: SocketAddr,
    current_time: Instant,
) -> crate::Result<()> {
    let Some(control) = client.client_control.as_ref().map(Rc::clone) else {
        return Ok(());
    };
    let trigger = control.borrow().message_migration_trigger;
    if trigger == 0 {
        return Ok(());
    }

    let should_migrate = {
        let Some(cnx) = client.qclient.earliest_cnx_to_wake(Instant::from_ticks(0)) else {
            return Ok(());
        };
        cnx.state() == State::Ready
            && cnx.pkt_ctx[PacketContext::Application as usize].send_sequence > u64::from(trigger)
    };
    if !should_migrate {
        return Ok(());
    }

    let new_addr = SocketAddr::new(
        client.client_addr.ip(),
        client.client_addr.port().saturating_add(1),
    );
    if let Some(cnx) = client.qclient.earliest_cnx_to_wake(Instant::from_ticks(0)) {
        cnx.probe_new_path(&server_addr, &new_addr, current_time)?;
    }
    client.client_addr = new_addr;
    control.borrow_mut().message_migration_trigger = trigger.saturating_add(32);
    Ok(())
}

fn stress_prepare_server_packet(
    qserver: &mut Quic,
    clients: &mut [StressClientContext],
    server_addr: SocketAddr,
    current_time: Instant,
) -> crate::Result<()> {
    let mut packet = TestSimPacket::create()?;
    let prepared = qserver.prepare_next_packet(current_time, &mut packet.bytes)?;
    if prepared.send_length > 0 {
        let index = stress_index_from_addr(prepared.addr_to).ok_or(Error::InvalidState)?;
        let addr_from = if prepared.addr_from.ip().is_unspecified() {
            server_addr
        } else {
            prepared.addr_from
        };
        packet.length = prepared.send_length;
        packet.addr_to = Some(prepared.addr_to);
        packet.addr_from = Some(addr_from);
        clients[index].s_to_c_link.submit(packet, current_time);
        clients[index].client_next_time =
            stress_client_eval_next_time(&mut clients[index], current_time.ticks());
    }
    Ok(())
}

fn stress_delete_disconnected_client_cnx(client: &mut StressClientContext) {
    let token = {
        let Some(cnx) = client.qclient.first_connection() else {
            return;
        };
        (cnx.state() == State::Disconnected)
            .then_some(cnx.own_token)
            .flatten()
    };
    if let Some(token) = token {
        client.qclient.delete_connection(token);
        client.client_control = None;
    }
}

fn stress_start_client_connection(ctx: &mut StressCtx, client_index: usize) -> crate::Result<()> {
    let cnx_id_null = ConnectionId::with_size(0).ok_or(Error::Generic)?;
    let simulated_time = ctx.simulated_time;
    let server_addr = ctx.server_addr;
    let test_id = ctx.next_test_id;
    ctx.next_test_id += 1;

    let client = &mut ctx.clients[client_index];
    let control = StressClientCallback::new_control(&mut ctx.random_ctx);
    let mut callback = StressClientCallback::new(Rc::clone(&ctx.shared), test_id);
    let streams = callback.prepare_streams()?;
    let cnx = client
        .qclient
        .create_connection(
            cnx_id_null,
            cnx_id_null,
            Some(&server_addr),
            Instant::from_ticks(simulated_time),
            0,
            Some(TEST_SNI),
            Some(TEST_ALPN),
            true,
        )
        .ok_or(Error::Memory)?;
    client.client_control = Some(control);
    cnx.set_callback(Some(Box::new(callback)));
    for (stream_id, buf) in streams {
        cnx.add_to_stream(stream_id, &buf, true)?;
    }
    cnx.start_client()?;
    ctx.nb_connections = ctx.nb_connections.saturating_add(1);
    Ok(())
}

fn stress_loop_poll_context(ctx: &mut StressCtx) -> crate::Result<()> {
    let now = Instant::from_ticks(ctx.simulated_time);
    stress_submit_sp_packets_server(&mut ctx.qserver, &mut ctx.clients, now)?;

    let delay_max = 100_000_000i64;
    let server_delay = ctx.qserver.next_wake_delay(now, delay_max).max(0) as u64;
    let mut best_wake_time = ctx.simulated_time.saturating_add(server_delay);
    let mut client_index = None;
    if let Some((index, client)) = ctx
        .clients
        .iter()
        .enumerate()
        .min_by_key(|(_, client)| client.client_next_time)
        && client.client_next_time < best_wake_time
    {
        best_wake_time = client.client_next_time;
        client_index = Some(index);
    }

    ctx.simulated_time = best_wake_time;
    let now = Instant::from_ticks(ctx.simulated_time);

    let Some(index) = client_index else {
        return stress_prepare_server_packet(
            &mut ctx.qserver,
            &mut ctx.clients,
            ctx.server_addr,
            now,
        );
    };

    if ctx.clients[index].qclient.current_number_connections() == 0 {
        stress_start_client_connection(ctx, index)?;
    } else {
        let client_addr = ctx.clients[index].client_addr;
        if ctx.clients[index]
            .s_to_c_link
            .packets
            .front()
            .is_some_and(|packet| packet.arrival_time <= now)
        {
            let client = &mut ctx.clients[index];
            stress_handle_packet_arrival(
                &mut client.qclient,
                &mut client.s_to_c_link,
                client_addr,
                now,
            )?;
        }

        if ctx.clients[index]
            .c_to_s_link
            .packets
            .front()
            .is_some_and(|packet| packet.arrival_time <= now)
        {
            let client = &mut ctx.clients[index];
            stress_handle_packet_arrival(
                &mut ctx.qserver,
                &mut client.c_to_s_link,
                ctx.server_addr,
                now,
            )?;
        }

        {
            let client = &mut ctx.clients[index];
            stress_submit_sp_packets_client(client, now)?;
        }

        let should_prepare = {
            let client = &mut ctx.clients[index];
            client
                .qclient
                .earliest_cnx_to_wake(Instant::from_ticks(0))
                .is_some_and(|cnx| cnx.next_wake_time <= now)
        };
        if should_prepare {
            let client = &mut ctx.clients[index];
            stress_prepare_client_packet(client, ctx.server_addr, now)?;
        }
    }

    let client = &mut ctx.clients[index];
    stress_delete_disconnected_client_cnx(client);
    client.client_next_time = stress_client_eval_next_time(client, ctx.simulated_time);
    Ok(())
}

fn stress_or_fuzz_test(
    duration: u64,
    wall_time_max: u64,
    fuzzer: Option<StressFuzzerConfig>,
) -> crate::Result<()> {
    let wall_time_start = current_time();
    let mut stress_ctx = stress_create_ctx(fuzzer)?;
    let mut sim_time_next_log = stress_ctx.simulated_time + 1_000_000;

    while stress_ctx.simulated_time < duration {
        if current_time().saturating_sub(wall_time_start) > wall_time_max {
            return Err(Error::InvalidState);
        }

        if stress_ctx.simulated_time > sim_time_next_log {
            sim_time_next_log = stress_ctx.simulated_time.saturating_add(1_000_000);
        }

        stress_loop_poll_context(&mut stress_ctx)?;
    }

    if stress_ctx.simulated_time < duration {
        Err(Error::InvalidState)
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Exported tests.

/// C: `random_tester_test` in `picoquictest/stresstest.c`.
///
/// Verifies that the PRNG produces the same sequence on every platform
/// for a set of known seeds.
#[test]
fn random_tester() {
    // Known (seed, [trial; 3], [uniform(31,32,100,1000); 4]) vectors.
    struct Case {
        seed: u64,
        trials: [u64; 3],
        uniform: [u64; 4],
    }
    let uniform_ranges: [u64; 4] = [31, 32, 100, 1000];
    let cases: &[Case] = &[
        Case {
            seed: 0xdeadbeefbabac001u64,
            trials: [
                0x5e15223d01b20defu64,
                0x9ede0d895c9bd2a6u64,
                0xe3a0ed91f612c17fu64,
            ],
            uniform: [0, 0, 70, 197],
        },
        Case {
            seed: 0x56df77dd5d6000efu64,
            trials: [
                0xdfccc8d428187e18u64,
                0x7d7552fd225a16d7u64,
                0x32dabe642e7390cu64,
            ],
            uniform: [30, 5, 34, 751],
        },
        Case {
            seed: 0x6fbbeeaeb00077abu64,
            trials: [
                0x43131e190d5c97fu64,
                0x42fb1ccc58b906du64,
                0x610a3b5abef97be4u64,
            ],
            uniform: [26, 16, 12, 939],
        },
        Case {
            seed: 0xddf75758003bd5b7u64,
            trials: [
                0x3a8d9a1a727aba2du64,
                0xe9279c9bb67c725cu64,
                0x1acf0953978b79e8u64,
            ],
            uniform: [3, 11, 41, 82],
        },
        Case {
            seed: 0xfbabac001deadbeeu64,
            trials: [
                0x5112b0a7de31f1b7u64,
                0xd691b591d3598619u64,
                0xf1b42dc66cf4f215u64,
            ],
            uniform: [17, 10, 44, 527],
        },
        Case {
            seed: 0xd5d6000ef56df77du64,
            trials: [
                0xb699f9cadcb2a474u64,
                0xc2213dfa4ec1c973u64,
                0x843f0e6573dda32eu64,
            ],
            uniform: [9, 30, 52, 680],
        },
        Case {
            seed: 0xeb00077ab6fbbeeau64,
            trials: [
                0x6dd0c0b399bae357u64,
                0xa5a6b1ec22fa894bu64,
                0x85f25e84ba0843a0u64,
            ],
            uniform: [16, 5, 5, 899],
        },
        Case {
            seed: 0x8003bd5b7ddf7575u64,
            trials: [
                0xf7745169aa75f266u64,
                0x551964d08e2c25e0u64,
                0x17b86c9be72f96bbu64,
            ],
            uniform: [4, 24, 48, 21],
        },
        Case {
            seed: 0x1deadbeefbabac0u64,
            trials: [
                0xc51696cc9c124ff9u64,
                0x1b9d1372c2f72058u64,
                0xe539681abb702c48u64,
            ],
            uniform: [20, 21, 96, 865],
        },
        Case {
            seed: 0xef56df77dd5d6000u64,
            trials: [
                0xf40b816f8efc0ec8u64,
                0xd8a949c49d03c01cu64,
                0x170902fde977c269u64,
            ],
            uniform: [2, 30, 55, 720],
        },
    ];

    for (i, c) in cases.iter().enumerate() {
        let mut ctx = c.seed;
        for (j, &expected) in c.trials.iter().enumerate() {
            let r = test_random(&mut ctx);
            assert_eq!(
                r,
                expected,
                "case {i}, seed {seed:#x}, trial[{j}] = {r:#x}, expected {expected:#x}",
                seed = c.seed,
            );
        }
        for (j, &expected) in c.uniform.iter().enumerate() {
            let r = test_uniform_random(&mut ctx, uniform_ranges[j]);
            assert_eq!(
                r,
                expected,
                "case {i}, seed {seed:#x}, uniform({urange}) = {r}, expected {expected}",
                seed = c.seed,
                urange = uniform_ranges[j],
            );
        }
    }
}

/// C: `random_gauss_test` in `picoquictest/stresstest.c`.
///
/// Verifies that the Gaussian generator has mean ≈ 0 and variance ≈ 1
/// over 255 samples.
#[test]
fn random_gauss() {
    const NB_TESTS: usize = 255;
    let mut t_seed: u64 = 0xDEADBEEFBABAC001u64;
    let mut x_sum: f64 = 0.0;
    let mut x2: f64 = 0.0;

    for _ in 0..NB_TESTS {
        let x = test_gauss_random(&mut t_seed);
        x_sum += x;
        x2 += x * x;
    }

    let mean = x_sum / NB_TESTS as f64;
    let var = x2 / NB_TESTS as f64;

    assert!(
        (-0.02..=0.02).contains(&mean),
        "Gaussian mean {mean} out of range [-0.02, 0.02]"
    );
    assert!(
        (0.97..=1.03).contains(&var),
        "Gaussian variance {var} out of range [0.97, 1.03]"
    );
}

/// C: `stress_test` in `picoquictest/stresstest.c`.
#[test]
fn stress() {
    let duration: u64 = 60_000_000; // 1 minute
    let wall_time_max: u64 = 10 * duration;
    stress_or_fuzz_test(duration, wall_time_max, None).expect("stress_test");
}

/// C: `fuzz_test` in `picoquictest/stresstest.c`.
#[test]
fn fuzz() {
    let duration: u64 = 60_000_000;
    let fuzz_ctx = Rc::new(RefCell::new(BasicFuzzer::new(duration)));

    stress_or_fuzz_test(
        duration,
        duration,
        Some(StressFuzzerConfig::Basic(Rc::clone(&fuzz_ctx))),
    )
    .expect("fuzz_test");

    let fuzz_ctx = fuzz_ctx.borrow();
    assert!(fuzz_ctx.nb_packets > 0, "fuzzer was never called");
    assert!(fuzz_ctx.nb_fuzzed > 0, "fuzzer never mutated packet bytes");
    assert!(
        fuzz_ctx.nb_fuzzed_length > 0,
        "fuzzer never changed packet length"
    );
    assert_eq!(
        fuzz_ctx.highest_state_fuzzed,
        crate::State::Ready,
        "fuzzer did not observe the ready state"
    );
}

/// C: `fuzz_initial_test` in `picoquictest/stresstest.c`.
#[test]
fn fuzz_initial() {
    let duration: u64 = 60_000_000;
    let fuzz_ctx = Rc::new(RefCell::new(InitialFuzzer::new(duration)));

    stress_or_fuzz_test(
        2 * duration,
        4 * duration,
        Some(StressFuzzerConfig::Initial(Rc::clone(&fuzz_ctx))),
    )
    .expect("fuzz_initial_test");

    let fuzz_ctx = fuzz_ctx.borrow();
    let frame_count = fuzz_ctx.frame_count();
    assert!(
        fuzz_ctx.initial_fuzzing_done,
        "initial fuzzer did not finish the skip-frame mutation pass"
    );
    assert!(
        fuzz_ctx.initial_packets > 3 * frame_count,
        "initial fuzzer did not reach the random fallback"
    );
    assert_eq!(
        fuzz_ctx.append_count, frame_count,
        "initial fuzzer did not append every skip-frame vector"
    );
    assert_eq!(
        fuzz_ctx.prepend_count, frame_count,
        "initial fuzzer did not prepend every skip-frame vector"
    );
    assert_eq!(
        fuzz_ctx.replace_count, frame_count,
        "initial fuzzer did not replace every skip-frame vector"
    );
    assert!(
        fuzz_ctx.random_count > 0,
        "random initial fuzzing did not run"
    );
    assert!(
        fuzz_ctx.random_bytes > 0,
        "random initial fuzzing did not mutate packet bytes"
    );
}
