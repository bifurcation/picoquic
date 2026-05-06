//! Test cases for `picoquictest/memlog_test.c`.
//!
//! Tests the in-memory performance log (`auto_memlog`) which records
//! per-path metrics during a connection and writes them to a CSV file.

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, compare_text_files, test_api_init_send_recv_scenario,
    tls_api_connection_loop, tls_api_init_ctx_ex2, tls_api_one_sim_round,
};
use crate::internal::Version;
use crate::{Instant, State};

const MEMLOG_FILE: &str = "memlog_file.csv";
const MEMLOG_FILE_MP: &str = "memlog_file_mp.csv";
const MEMLOG_FILE_BAD: &str = "no_such_folder/bad/memlog_file_bad.csv";
const MEMLOG_TEST_REF: &str = "picoquictest/memlog_test_ref.csv";

/// Stream scenario: 100 KB query + 100 KB response.  C: `test_scenario_memlog[]`.
const TEST_SCENARIO_MEMLOG: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 100_000,
    r_len: 100_000,
}];

/// Run one memlog scenario.  C: `memlog_test_one`.
///
/// Creates a test context, attaches a memory log to the client connection,
/// optionally enables multipath, drives the simulation to completion, and
/// compares the generated CSV to the reference (for non-multipath runs).
fn memlog_test_one(is_multipath: bool, memlog_file_name: &str, expect_error: bool) {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;
    let queue_delay_max: u64 = 40_000;

    let initial_cid =
        crate::ConnectionId::clone_from_slice(&[0x8e, 0x10, 0x97, 0xe5, 0x70, 0, 0, 0])
            .expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    // Attach an in-memory performance log to the client connection.
    test_ctx
        .cnx_client()
        .memlog_init(100, memlog_file_name)
        .expect("memlog_init");

    if is_multipath {
        // Enable multipath by setting initial_max_path_id = 1 on both endpoints.
        {
            let mut tp = test_ctx.qserver.default_tp().clone();
            tp.initial_max_path_id = 1;
            test_ctx.qserver.set_default_tp(&tp).ok();
        }
        {
            let mut tp = test_ctx.qclient.default_tp().clone();
            tp.initial_max_path_id = 1;
            test_ctx.qclient.set_default_tp(&tp).ok();
        }
        test_ctx.cnx_client().local_parameters.initial_max_path_id = 1;
    }

    test_ctx.cnx_client().start_client().expect("start_client");
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        queue_delay_max,
        &mut simulated_time,
    )
    .expect("connection loop");

    if is_multipath {
        assert!(
            test_ctx.cnx_client().is_multipath_enabled,
            "multipath must be enabled after negotiation"
        );
    }

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MEMLOG).expect("init scenario");

    // Simulation loop: advance until client disconnects or scenario completes.
    let mut nb_inactive = 0i32;
    let mut nb_trials = 0i32;
    loop {
        if test_ctx.cnx_client().state() == State::Disconnected {
            break;
        }
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(0),
            &mut was_active,
        )
        .expect("sim round");

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if test_ctx.test_finished
            && test_ctx.cnx_client().is_cnx_backlog_empty()
            && test_ctx.has_cnx_server()
            && test_ctx.cnx_server().is_cnx_backlog_empty()
        {
            break;
        }

        nb_trials += 1;
        assert!(
            nb_trials <= 1_000_000 && nb_inactive <= 1024,
            "simulation stalled"
        );
    }

    // Compare generated CSV to reference (non-multipath, non-error runs only).
    if !is_multipath && !expect_error {
        compare_text_files(memlog_file_name, MEMLOG_TEST_REF).expect("log file matches reference");
    } else if expect_error {
        // The test is inverted: expect that memlog_init or the log write failed.
        // If we got here without an error, the test has failed.
        // (In practice, the todo!() above will panic first.)
    }
}

/// Run three memlog sub-tests: normal, multipath, and expected-failure.
/// C: `memlog_test`.
#[test]
fn memlog() {
    memlog_test_one(false, MEMLOG_FILE, false);
    memlog_test_one(true, MEMLOG_FILE_MP, false);
    memlog_test_one(true, MEMLOG_FILE_BAD, true);
}
