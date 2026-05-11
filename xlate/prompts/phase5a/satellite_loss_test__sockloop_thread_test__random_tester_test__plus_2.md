# Phase 5A C/Rust test correspondence audit

Review each C/Rust test pair and decide whether the Rust
`#[test]` checks the same behavior as the C test.  This is
a read-only pass: do not edit files.

You may inspect directly relevant context when needed:
Rust test helpers, fixtures, translated implementation under
`rs/fq/`, C helper functions in `picoquictest/`, constants,
and nearby tests.  Do not require byte-for-byte structure;
idiomatic Rust is fine when it preserves the C test intent,
inputs, expected results, and important edge cases.

Classify each entry as:

* `ok` when the Rust test is an acceptable translation.
* `needs_fix` when the Rust test is missing checks, checks
  materially different behavior, weakens assertions, skips
  cases the C test covers, or has placeholder-like logic.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"reviews":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short conclusion","fix_summary":"what 5B should change, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/satellite_test.c:satellite_loss_test`
* C test-table name: `satellite_loss`
* C entry function: `satellite_loss_test`
* Rust test: `satellite_loss`
* C source: `picoquictest/satellite_test.c:232-236`
* Rust source: `rs/fq/src/tests/satellite.rs:247-262`

### C test body
```c
{
    /* Should be less than 10 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 8000000, 250, 3, 0, 1, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_loss() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        8_000_000,
        250,
        3,
        0,
        true,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/sockloop_test.c:sockloop_thread_test`
* C test-table name: `sockloop_thread`
* C entry function: `sockloop_thread_test`
* Rust test: `sockloop_thread`
* C source: `picoquictest/sockloop_test.c:710-720`
* Rust source: `rs/fq/src/tests/sockloop.rs:618-624`

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 7);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.use_background_thread = 1;

    return(sockloop_test_one(&spec));
}
```

### Rust test body
```rust
fn sockloop_thread() {
    let mut spec = SockloopTestSpec::new(7);
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.use_background_thread = true;
    sockloop_test_one(&spec);
}
```

## `picoquictest/stresstest.c:random_tester_test`
* C test-table name: `random_tester`
* C entry function: `random_tester_test`
* Rust test: `random_tester`
* C source: `picoquictest/stresstest.c:1311-1361`
* Rust source: `rs/fq/src/tests/stresstest.rs:94-217`

### C test body
```c
{
    /* This is the initial run, so we merely write the expected value */
    uint64_t t_seed = 0xDEADBEEFBABAC001ull;
    int ret = 0;

    if (nb_random_cases < 2) {
        /* This code was used to generate the table of random cases */
        for (int i = 0; i < 10; i++)
        {
            /* Rotate the seed */
            uint64_t stress_ctx = t_seed;
            /* Generate the values */
            printf("{ 0x%llxull, \n{ ", (unsigned long long)t_seed);
            for (int j = 0; j < 3; j++) {
                printf("0x%llxull%s", (unsigned long long)picoquic_test_random(&stress_ctx), (j < 2) ? ", " : "},\n{ ");
            }
            for (int j = 0; j < 4; j++) {
                printf("%d%s", (int)picoquic_test_uniform_random(&stress_ctx, uniform_test[j]),
                    (j < 3) ? ", " : "}},\n");
            }
            t_seed = (t_seed << 7) | (t_seed >> 57);
        }
    }
    else {
        for (int i = 0; ret == 0 && i < (int)nb_random_cases; i++)
        {
            uint64_t stress_ctx = random_cases[i].seed;
            for (int j = 0; ret == 0 && j < 3; j++) {
                uint64_t r = picoquic_test_random(&stress_ctx);
                if (r != random_cases[i].trials[j]) {
                    DBG_PRINTF("Case %d, seed %llx, trial[%d] = %llx, expected %llx\n",
                        i, (unsigned long long)random_cases[i].seed, j,
                        (unsigned long long)r, (unsigned long long)random_cases[i].trials[j]);
                    ret = -1;
                }
            }
            for (int j = 0; ret == 0 && j < 4; j++) {
                int r = (int)picoquic_test_uniform_random(&stress_ctx, uniform_test[j]);
                if (r != random_cases[i].uniform[j]) {
                    DBG_PRINTF("Case %d, seed %llx, uniform(%d) = %d, expected %d\n",
                        i, (unsigned long long)random_cases[i].seed, uniform_test[j],
                        (unsigned long long)r, (unsigned long long)random_cases[i].uniform[j]);
                    ret = -1;
                }
            }
        }
    }

    return ret;
}
```

### Rust test body
```rust
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
```

## `picoquictest/tls_api_test.c:chacha20_test`
* C test-table name: `chacha20`
* C entry function: `chacha20_test`
* Rust test: `chacha20`
* C source: `picoquictest/tls_api_test.c:10868-10904`
* Rust source: `rs/fq/src/tests/tls_api.rs:100-102`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int has_chacha_poly = 0;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    /* Set the cipher suite to chacha20
     */
    if (ret == 0) {
        has_chacha_poly = (picoquic_set_cipher_suite(test_ctx->qclient, PICOQUIC_CHACHA20_POLY1305_SHA256) == 0);

        if (has_chacha_poly) {

            /* Run a basic test scenario */
            ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
                test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 250000);
        }
        else {
            DBG_PRINTF("%s", "Could not test CHACHA20, not supported on this platform.");
        }
    }

    /* And then free the resource
     */

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn chacha20() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("chacha20");
}
```

## `picoquictest/tls_api_test.c:cnx_ddos_unit_test`
* C test-table name: `cnx_ddos`
* C entry function: `cnx_ddos_unit_test`
* Rust test: `cnx_ddos`
* C source: `picoquictest/tls_api_test.c:11806-11809`
* Rust source: `rs/fq/src/tests/tls_api.rs:188-190`

### C test body
```c
{
    return cnx_ddos_test_loop(1000, 1000, NULL);
}
```

### Rust test body
```rust
fn cnx_ddos() {
    cnx_ddos_test_loop(1000, 1000).expect("cnx_ddos");
}
```
