//! Test cases for `picoquictest/warptest*.c`.

#![allow(non_snake_case)]

use crate::tests::util::{WarptestSpec, warptest_one};

/// C: `warptest_param_test` in `picoquictest/warptest.c`.
///
/// BBR, 10 Mbps, video + audio, max_streams_client/server = 4.
#[test]
fn warptest_param() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        max_streams_client: 4,
        max_streams_server: 4,
        ..Default::default()
    };
    warptest_one(5, &spec).expect("warptest_param");
}

/// C: `warptest_video_test` in `picoquictest/warptest.c`.
///
/// BBR, 10 Mbps, video only.
#[test]
fn warptest_video() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        ..Default::default()
    };
    warptest_one(1, &spec).expect("warptest_video");
}

/// C: `warptest_video_audio_test` in `picoquictest/warptest.c`.
///
/// BBR, 10 Mbps, video + audio.
#[test]
fn warptest_video_audio() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        ..Default::default()
    };
    warptest_one(2, &spec).expect("warptest_video_audio");
}

/// C: `warptest_video_data_audio_test` in `picoquictest/warptest.c`.
///
/// BBR, 10 Mbps, video + audio + 10 MB bulk data.
#[test]
fn warptest_video_data_audio() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        ..Default::default()
    };
    warptest_one(3, &spec).expect("warptest_video_data_audio");
}

/// C: `warptest_worst_test` in `picoquictest/warptest.c`.
///
/// BBR, 10 Mbps, video + audio + 10 MB datagram payload (worst-case contention).
#[test]
fn warptest_worst() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        datagram_data_size: 10_000_000,
        ..Default::default()
    };
    warptest_one(4, &spec).expect("warptest_worst");
}
