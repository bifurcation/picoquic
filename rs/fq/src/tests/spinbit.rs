//! Test cases for `picoquictest/spinbit_test.c`.
//!
//! Verifies spin-bit rotation counts for various server/client
//! spinbit policies (basic, on, null, random).

#![allow(non_snake_case)]

use super::util::{
    TestApiStreamDesc, test_api_init_send_recv_scenario, tls_api_connection_loop, tls_api_init_ctx,
    tls_api_one_sim_round,
};
use crate::internal::Version;
use crate::{Instant, SpinbitVersion};

const TEST_SCENARIO_SPIN: &[TestApiStreamDesc] = &[TestApiStreamDesc {
    stream_id: 4,
    previous_stream_id: 0,
    q_len: 257,
    r_len: 1_000_000,
}];

// ---------------------------------------------------------------------------
// Core helper.
// C: `spinbit_test_one` in `picoquictest/spinbit_test.c`.

fn spinbit_test_one(
    spin_policy: SpinbitVersion,
    spin_policy_server: SpinbitVersion,
) -> crate::Result<()> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;

    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, Version::InternalTest1 as u32, None)
        .ok_or(crate::Error::Generic)?;

    let _ = test_ctx
        .qserver
        .set_default_spinbit_policy(spin_policy_server);
    test_ctx.cnx_client().set_spinbit_policy(spin_policy)?;

    test_ctx.cnx_client().start_client()?;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)?;

    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_SPIN)?;

    // Observe spin-bit transitions over a 10-second window.
    let spin_begin_time = simulated_time;
    let next_time = Instant::from_ticks(simulated_time.ticks() + 10_000_000);
    let max_trials = 100_000;
    let mut nb_trials = 0;
    let mut nb_inactive = 0;
    let mut spin_count: i32 = 0;
    let mut current_spin = test_ctx.cnx_client().primary_path_current_spin();

    test_ctx.c_to_s_link.loss_mask = Some(loss_mask);
    test_ctx.s_to_c_link.loss_mask = Some(loss_mask);

    while nb_trials < max_trials
        && simulated_time < next_time
        && nb_inactive < 256
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

        let new_spin = test_ctx.cnx_client().primary_path_current_spin();
        if new_spin != current_spin {
            spin_count += 1;
            current_spin = new_spin;
        }

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
        }

        if test_ctx.test_finished
            && test_ctx.cnx_client().is_cnx_backlog_empty()
            && test_ctx.cnx_server().is_cnx_backlog_empty()
        {
            break;
        }
    }

    let spin_duration = simulated_time.ticks() - spin_begin_time.ticks();
    let _ = spin_duration;

    test_ctx.cnx_client().close(0)?;

    // Validate spin counts against policy expectations.
    if matches!(spin_policy, SpinbitVersion::Basic) {
        match spin_policy_server {
            SpinbitVersion::On => {
                assert!(
                    spin_count >= 6,
                    "implausible spin bit: {} rotations, rtt_min = {}, duration = {}",
                    spin_count,
                    test_ctx.cnx_client().primary_path_rtt_min(),
                    spin_duration,
                );
            }
            SpinbitVersion::Random => {
                assert!(
                    spin_count >= 100,
                    "implausible spin bit: {} rotations",
                    spin_count,
                );
            }
            SpinbitVersion::Null => {
                assert_eq!(
                    spin_count, 0,
                    "implausible spin bit: {} rotations with null policy",
                    spin_count,
                );
            }
            _ => {}
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Exported tests.

/// C: `spinbit_test` in `picoquictest/spinbit_test.c`.
#[test]
fn spinbit() {
    spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::On).expect("spinbit");
}

/// C: `spinbit_random_test` in `picoquictest/spinbit_test.c`.
#[test]
fn spinbit_random() {
    spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::Random).expect("spinbit_random");
}

/// C: `spinbit_randclient_test` in `picoquictest/spinbit_test.c`.
#[test]
fn spinbit_randclient() {
    spinbit_test_one(SpinbitVersion::Random, SpinbitVersion::Basic).expect("spinbit_randclient");
}

/// C: `spinbit_null_test` in `picoquictest/spinbit_test.c`.
#[test]
fn spinbit_null() {
    spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::Null).expect("spinbit_null");
}

/// C: `spinbit_bad_test` in `picoquictest/spinbit_test.c`.
/// Verifies that invalid spinbit policy codes are rejected.
///
/// The C test passes raw out-of-range integer values (123456, 123455) which
/// the Rust type system prevents.  The equivalent Rust check uses
/// `SpinbitVersion::On`, which is documented as "not valid as a per-connection
/// override (server only)" — passing it as a client policy must return `Err`.
#[test]
fn spinbit_bad() {
    let r1 = spinbit_test_one(SpinbitVersion::On, SpinbitVersion::Basic);
    let r2 = spinbit_test_one(SpinbitVersion::Basic, SpinbitVersion::On);
    assert!(
        r1.is_err() || r2.is_err(),
        "expected at least one invalid per-connection policy to be rejected"
    );
}
