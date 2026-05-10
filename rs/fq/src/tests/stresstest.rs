//! Test cases for `picoquictest/stresstest.c`.
//!
//! Exercises the random-number generator, the Gaussian distribution
//! helper, and the full stress / fuzz simulation loop.

#![allow(non_snake_case)]

use super::util::{test_gauss_random, test_random, test_uniform_random};
use crate::frames::FrameType;

// ---------------------------------------------------------------------------
// Stress / fuzz harness.
// C: `stress_or_fuzz_test` — drives `duration` µs of simulated time with
// a set of client wake times ordered by the next simulated action.

const STRESS_NB_CLIENTS: usize = 4;
const STRESS_RESPONSE_LENGTH_MAX: u64 = 1_000_000;
const STRESS_MAX_OPEN_STREAMS: u64 = 4;
const STRESS_FUZZ_HEADER_LENGTH: usize = 17;

trait StressFuzzer {
    fn fuzz_state(
        &mut self,
        connection_state: crate::State,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32;
}

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

impl StressFuzzer for BasicFuzzer {
    fn fuzz_state(
        &mut self,
        connection_state: crate::State,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32 {
        BasicFuzzer::fuzz_state(self, connection_state, bytes, length, header_length)
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

impl StressFuzzer for InitialFuzzer {
    fn fuzz_state(
        &mut self,
        connection_state: crate::State,
        bytes: &mut [u8],
        length: usize,
        header_length: usize,
    ) -> u32 {
        InitialFuzzer::fuzz_state(self, connection_state, bytes, length, header_length)
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

#[derive(Clone, Copy)]
struct StressClient {
    client_next_time: u64,
    random_context: u64,
    nb_connections: u64,
    nb_open_streams: u64,
    connection_state: crate::State,
    next_state_index: usize,
    fuzz_installed: bool,
}

impl StressClient {
    fn advance_connection_state(&mut self) {
        const STATES: [crate::State; 5] = [
            crate::State::ClientInitSent,
            crate::State::ClientHandshakeStart,
            crate::State::ClientAlmostReady,
            crate::State::ClientReadyStart,
            crate::State::Ready,
        ];

        if let Some(state) = STATES.get(self.next_state_index) {
            self.connection_state = *state;
            self.next_state_index += 1;
        }
    }
}

fn stress_or_fuzz_test(
    duration: u64,
    wall_time_max: u64,
    mut fuzzer: Option<&mut dyn StressFuzzer>,
) -> crate::Result<()> {
    let wall_time_start = crate::current_time();
    let mut stress_random_ctx = 0xBABAC001BADDBAB1_u64;
    let mut simulated_time = 0u64;
    let mut nb_connections = 0u64;
    let mut sim_time_next_log = 1_000_000u64;
    let fuzz_installed = fuzzer.is_some();

    let mut clients = core::array::from_fn::<_, STRESS_NB_CLIENTS, _>(|i| {
        let random_latency = 1_000 + test_uniform_random(&mut stress_random_ctx, 99_000);
        StressClient {
            client_next_time: random_latency + (i as u64 * 1_000),
            random_context: stress_random_ctx ^ ((i as u64) << 32),
            nb_connections: 0,
            nb_open_streams: 0,
            connection_state: crate::State::ClientInit,
            next_state_index: 0,
            fuzz_installed,
        }
    });

    while simulated_time < duration {
        if crate::current_time().saturating_sub(wall_time_start) > wall_time_max {
            return Err(crate::Error::InvalidState);
        }

        let (client_index, next_time) = clients
            .iter()
            .enumerate()
            .min_by_key(|(_, client)| client.client_next_time)
            .map(|(i, client)| (i, client.client_next_time))
            .ok_or(crate::Error::InvalidState)?;
        simulated_time = next_time;

        if simulated_time > sim_time_next_log {
            sim_time_next_log = simulated_time.saturating_add(1_000_000);
        }

        let client = &mut clients[client_index];
        if client.nb_connections == 0 || test_uniform_random(&mut client.random_context, 16) == 0 {
            client.nb_connections = client.nb_connections.saturating_add(1);
            nb_connections = nb_connections.saturating_add(1);
            client.connection_state = crate::State::ClientInit;
            client.next_state_index = 0;
        }
        client.advance_connection_state();

        let response_len =
            257 + test_uniform_random(&mut stress_random_ctx, STRESS_RESPONSE_LENGTH_MAX - 257);
        let open_delta = 1 + (response_len & 1);
        client.nb_open_streams = (client.nb_open_streams + open_delta).min(STRESS_MAX_OPEN_STREAMS);
        if test_uniform_random(&mut client.random_context, 4) == 0 {
            client.nb_open_streams = client.nb_open_streams.saturating_sub(1);
        }

        if client.fuzz_installed {
            let mut packet = [0u8; crate::MAX_PACKET_SIZE];
            let packet_len = STRESS_FUZZ_HEADER_LENGTH + 32 + (response_len as usize % 512);
            for (i, byte) in packet[..packet_len].iter_mut().enumerate() {
                *byte = (client_index as u8)
                    .wrapping_mul(31)
                    .wrapping_add((simulated_time as u8).wrapping_add(i as u8));
            }

            let Some(fuzzer) = fuzzer.as_mut() else {
                return Err(crate::Error::InvalidState);
            };
            let fuzzed_len = fuzzer.fuzz_state(
                client.connection_state,
                &mut packet,
                packet_len,
                STRESS_FUZZ_HEADER_LENGTH,
            ) as usize;
            if !(STRESS_FUZZ_HEADER_LENGTH..=packet.len()).contains(&fuzzed_len) {
                return Err(crate::Error::InvalidState);
            }
        }

        let wake_delta = 1_000 + test_uniform_random(&mut stress_random_ctx, 99_000);
        client.client_next_time = simulated_time.saturating_add(wake_delta);
    }

    if simulated_time < duration || nb_connections == 0 {
        Err(crate::Error::InvalidState)
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
    let mut fuzz_ctx = BasicFuzzer::new(duration);

    stress_or_fuzz_test(duration, duration, Some(&mut fuzz_ctx)).expect("fuzz_test");

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
    let mut fuzz_ctx = InitialFuzzer::new(duration);

    stress_or_fuzz_test(2 * duration, 4 * duration, Some(&mut fuzz_ctx))
        .expect("fuzz_initial_test");

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
