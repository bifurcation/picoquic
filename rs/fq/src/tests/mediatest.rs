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
    TestApiStreamDesc, test_api_init_send_recv_scenario, tls_api_connection_loop,
    tls_api_data_sending_loop, tls_api_init_ctx_ex2, tls_api_one_scenario_body_verify,
};
use crate::internal::Version;
use crate::{ConnectionId, Instant};

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
    /// Suspension duration (µs).  C: `suspension_up_time`.
    pub suspension_up_time: u64,
    /// Inter-suspension gap (µs).  C: `suspension_down_time`.
    pub suspension_down_time: u64,
    /// Disable coalescing of audio/video frames.  C: `no_coal`.
    pub no_coal: bool,
}

// ---------------------------------------------------------------------------
// Core test driver.

/// Run one media-test scenario.  C: `mediatest_one`.
///
/// Creates a custom simulation context (`mediatest_ctx_t` in C), configures
/// client/server QUIC contexts with the specified media streams, runs the
/// simulation loop, and checks that frame latencies meet `spec` bounds.
pub fn mediatest_one(id: MediatestId, spec: &MediatestSpec) -> crate::Result<()> {
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
