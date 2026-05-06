//! Test cases for `picoquictest/stream0_frame_test.c`.
//!
//! Exercises STREAM-0 / TLS-stream frame decode, provide-stream-buffer
//! round-trip, output-stream list management, stream ID rank encoding,
//! splay-tree integrity, and local-stream-ID reuse.

#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Internal helper stubs.
// Each wraps a corresponding C helper that manipulates raw connection state.
// Phase 4 will implement these; for now they are `todo!()` stubs.

/// C: `StreamZeroFrameOneTest` — decode STREAM-0 frames from `name`'s
/// packet list into a fresh connection and verify the splay-tree content.
fn stream_zero_frame_one_test(_name: &str) -> crate::Result<()> {
    todo!("StreamZeroFrameOneTest")
}

/// C: `TlsStreamFrameOneTest` — decode CRYPTO handshake frames from
/// `name`'s packet list and verify the TLS-stream data tree.
fn tls_stream_frame_one_test(_name: &str) -> crate::Result<()> {
    todo!("TlsStreamFrameOneTest")
}

/// C: `provide_stream_buffer_test_one` — sets up a stream frame header,
/// calls `picoquic_provide_stream_data_buffer`, fills with test data,
/// decodes the header, and verifies the round-trip.
fn provide_stream_buffer_test_one(
    _stream_id: u64,
    _stream_offset: u64,
    _size_test: usize,
    _is_fin: bool,
) -> crate::Result<()> {
    todo!("provide_stream_buffer_test_one")
}

/// C: `stream_output_test` body — creates eight streams, exercises the
/// output-stream list ordering (including a `MAX_STREAMS` bump), marks
/// streams active, then deletes them in a specific order verifying
/// `find_ready_stream` behaviour at each step.
fn stream_output_test_body() -> crate::Result<()> {
    todo!("stream_output_test")
}

/// C: `stream_splay_test` body — inserts seven streams in non-monotone
/// order, verifies splay-tree structural invariants and first/last
/// pointers after each insertion, then deletes them in insertion order
/// and checks the invariants again.
fn stream_splay_test_body() -> crate::Result<()> {
    todo!("stream_splay_test")
}

/// C: `stream_state_local_reuse_test` body — opens a local stream,
/// deletes it, then verifies that calling `set_app_stream_ctx` on the
/// recycled ID returns `STREAM_ALREADY_CLOSED` (not `STREAM_STATE_ERROR`).
fn stream_state_local_reuse_body() -> crate::Result<()> {
    todo!("stream_state_local_reuse_test")
}

// ---------------------------------------------------------------------------
// Exported tests.

/// C: `StreamZeroFrameTest` in `picoquictest/stream0_frame_test.c`.
#[test]
fn streamzeroframe() {
    for name in &["test_v1", "test_v2", "test_v3"] {
        stream_zero_frame_one_test(name).unwrap_or_else(|_| panic!("{name}"));
    }
}

/// C: `TlsStreamFrameTest` in `picoquictest/stream0_frame_test.c`.
#[test]
fn tlsstreamframe() {
    for name in &["tlstest_v1", "tlstest_v2", "tlstest_v3", "tlstest_v4"] {
        tls_stream_frame_one_test(name).unwrap_or_else(|_| panic!("{name}"));
    }
}

/// C: `provide_stream_buffer_test` in `picoquictest/stream0_frame_test.c`.
#[test]
fn provide_stream_buffer() {
    let stream_ids: [u64; 4] = [0, 7, 127, 0x10000];
    let offsets: [u64; 4] = [0, 1, 65, 0x10000];
    for &sid in &stream_ids {
        for &off in &offsets {
            for is_fin in [false, true] {
                for size_test in 0..7usize {
                    provide_stream_buffer_test_one(sid, off, size_test, is_fin)
                        .expect("provide_stream_buffer_test_one");
                }
            }
        }
    }
}

/// C: `stream_output_test` in `picoquictest/stream0_frame_test.c`.
#[test]
fn stream_output() {
    stream_output_test_body().expect("stream_output_test");
}

/// C: `stream_rank_test` in `picoquictest/stream0_frame_test.c`.
///
/// Verifies the QUIC stream ID rank macros:
///   `STREAM_RANK_FROM_ID(id)  = (id >> 2) + 1`
///   `STREAM_ID_FROM_RANK(rank, client, uni) = ((rank-1) << 2) | (uni << 1) | server`
#[test]
fn stream_rank() {
    let ranks: [u64; 5] = [1, 2, 3, 1000, 10000];
    let client_bidir: [u64; 5] = [0, 4, 8, 3996, 39996];
    let client_unidir: [u64; 5] = [2, 6, 10, 3998, 39998];
    let server_bidir: [u64; 5] = [1, 5, 9, 3997, 39997];
    let server_unidir: [u64; 5] = [3, 7, 11, 3999, 39999];

    let rank_from_id = |id: u64| -> u64 { (id >> 2) + 1 };
    let id_from_rank = |rank: u64, client_mode: bool, is_unidir: bool| -> u64 {
        ((rank - 1) << 2) | (u64::from(is_unidir) << 1) | u64::from(!client_mode)
    };

    for i in 0..5 {
        let r = ranks[i];

        assert_eq!(
            rank_from_id(client_bidir[i]),
            r,
            "rank_from_id client_bidir[{i}]"
        );
        assert_eq!(
            id_from_rank(r, true, false),
            client_bidir[i],
            "id_from_rank client_bidir[{i}]"
        );

        assert_eq!(
            rank_from_id(client_unidir[i]),
            r,
            "rank_from_id client_unidir[{i}]"
        );
        assert_eq!(
            id_from_rank(r, true, true),
            client_unidir[i],
            "id_from_rank client_unidir[{i}]"
        );

        assert_eq!(
            rank_from_id(server_bidir[i]),
            r,
            "rank_from_id server_bidir[{i}]"
        );
        assert_eq!(
            id_from_rank(r, false, false),
            server_bidir[i],
            "id_from_rank server_bidir[{i}]"
        );

        assert_eq!(
            rank_from_id(server_unidir[i]),
            r,
            "rank_from_id server_unidir[{i}]"
        );
        assert_eq!(
            id_from_rank(r, false, true),
            server_unidir[i],
            "id_from_rank server_unidir[{i}]"
        );
    }
}

/// C: `stream_splay_test` in `picoquictest/stream0_frame_test.c`.
#[test]
fn stream_splay() {
    stream_splay_test_body().expect("stream_splay_test");
}

/// C: `stream_state_local_reuse_test` in `picoquictest/stream0_frame_test.c`.
#[test]
fn stream_state_local_reuse() {
    stream_state_local_reuse_body().expect("stream_state_local_reuse_test");
}
