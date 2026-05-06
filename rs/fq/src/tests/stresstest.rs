//! Test cases for `picoquictest/stresstest.c`.
//!
//! Exercises the random-number generator, the Gaussian distribution
//! helper, and the full stress / fuzz simulation loop.

#![allow(non_snake_case)]

use super::util::{test_gauss_random, test_random, test_uniform_random};

// ---------------------------------------------------------------------------
// Stress / fuzz harness stub.
// C: `stress_or_fuzz_test` — drives `duration` µs of simulated time with
// an optional per-packet fuzzer.  Phase 4 will wire up the full
// multi-client simulation; for now the body is `todo!()`.

fn stress_or_fuzz_test(_duration: u64, _wall_time_max: u64) -> crate::Result<()> {
    todo!("stress_or_fuzz_test")
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
    stress_or_fuzz_test(duration, wall_time_max).expect("stress_test");
}

/// C: `fuzz_test` in `picoquictest/stresstest.c`.
#[test]
fn fuzz() {
    let duration: u64 = 60_000_000;
    stress_or_fuzz_test(duration, duration).expect("fuzz_test");
}

/// C: `fuzz_initial_test` in `picoquictest/stresstest.c`.
#[test]
fn fuzz_initial() {
    let duration: u64 = 60_000_000;
    stress_or_fuzz_test(2 * duration, 4 * duration).expect("fuzz_initial_test");
}
