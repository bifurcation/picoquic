# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/tls_api_test.c:client_only_test`
* C test-table name: `client_only`
* C entry function: `client_only_test`
* Rust test: `client_only`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1544-1567`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test runs the generic successful TLS connection helper. It never enables client-only enforcement on the server and does not assert client disconnection or absence of a server connection context.
* Phase 5A fix note: Initialize with the C initial CID, call qserver.enforce_client_only(true), run the connection loop, and assert the connection is refused and no server connection exists.
* Phase 5B analysis: Rust test now faithfully checks the C client-only refusal behavior: client does not complete successfully and no server connection context is created.
* Phase 5B fix note: Replaced generic TLS success helper with explicit C initial CID setup, server client-only enforcement, qlog setup, connection loop, and refusal/no-server-context assertions. Also cleaned same-file error logging to satisfy clippy.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0xc1, 0x10, 0, 0, 0, 0, 0, 0}, 8 };
    int ret;

    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid);

    if (ret == 0) {
        /* First, try enforcement. We do set log on the server side, but it is expected to be empty */
        int connection_ret = 0;
        picoquic_enforce_client_only(test_ctx->qserver, 1);
        picoquic_set_qlog(test_ctx->qserver, ".");
        picoquic_set_qlog(test_ctx->qclient, ".");
        connection_ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
        if (connection_ret == 0 && test_ctx->cnx_client->cnx_state < picoquic_state_disconnected) {
            DBG_PRINTF("Connection unexpectedly succeeds, state=%d, ret=%d (0x%x)",
                test_ctx->cnx_client->cnx_state, connection_ret, connection_ret);
            ret = -1;
        }
        else if (test_ctx->cnx_server != NULL) {
            DBG_PRINTF("Connection context created on client-only note, ret=%d (0x%x)",
                connection_ret, connection_ret);
            ret = -1;
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn client_only() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xc1, 0x10, 0, 0, 0, 0, 0, 0]).expect("initial CID");
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .expect("tls_api_init_ctx_ex");

    test_ctx.qserver.enforce_client_only(true);
    test_ctx.qserver.set_qlog(".").expect("server qlog");
    test_ctx.qclient.set_qlog(".").expect("client qlog");

    let connection_ret =
        tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time);
    let client_state = test_ctx.cnx_client().state();
    assert!(
        connection_ret.is_err() || client_state >= State::Disconnected,
        "connection unexpectedly succeeded: state={client_state:?}, ret={connection_ret:?}"
    );
    assert!(
        !test_ctx.has_cnx_server(),
        "server connection context created despite client-only enforcement"
    );
}
```

## `picoquictest/tls_api_test.c:random_public_tester_test`
* C test-table name: `random_public_tester`
* C entry function: `random_public_tester_test`
* Rust test: `random_public_tester`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6773-6802`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs 100 generic TLS handshakes, which is unrelated to the C test's seeded public RNG uniform-range and chi-square distribution check over 1100 samples.
* Phase 5A fix note: Replace the Rust body with the seeded public-random test: seed with RANDOM_PUBLIC_TEST_SEED, draw 11*100 values from the public uniform RNG, assert all are < 11, count buckets, compute chi-square, and fail above 18.31.
* Phase 5B analysis: Rust test now matches the C seeded public RNG uniform-range and chi-square distribution check.
* Phase 5B fix note: Replaced the unrelated 100-handshake loop with deterministic seeding, 1100 uniform draws over 11 buckets, per-draw range assertion, bucket counts, and chi-square threshold 18.31.

### C test body
```c
{
#define RANDOM_PUBLIC_TEST_CONST 11
#define RANDOM_PUBLIC_TEST_ROUNDS 100
#define RANDOM_PUBLIC_CHI_SQUARE 18.31 /* Fail if significance of bias < P = 0.05 */
    int ret = 0;
    int r_count[RANDOM_PUBLIC_TEST_CONST];

    picoquic_public_random_seed_64(RANDOM_PUBLIC_TEST_SEED, 1);

    memset(r_count, 0, sizeof(r_count));

    for (int i = 0; i < RANDOM_PUBLIC_TEST_CONST*RANDOM_PUBLIC_TEST_ROUNDS; i++) {
        uint64_t x = picoquic_public_uniform_random(RANDOM_PUBLIC_TEST_CONST);

        if (x >= RANDOM_PUBLIC_TEST_CONST) {
            DBG_PRINTF("Value %d >= %d\n", x, RANDOM_PUBLIC_TEST_CONST);
            ret = -1;
            break;
        }
        else {
            r_count[x] += 1;
        }
    }

    if (ret == 0) {
        double chi_squared = 0;

        for (int i = 0; i < RANDOM_PUBLIC_TEST_CONST; i++) {
            double delta = ((double)RANDOM_PUBLIC_TEST_ROUNDS - r_count[i]);
            double d2 = delta * delta;
            d2 /= ((double)RANDOM_PUBLIC_TEST_ROUNDS);
            chi_squared += d2;
        }

        if (chi_squared > RANDOM_PUBLIC_CHI_SQUARE) {
            DBG_PRINTF("Chi2 = %f, larger than %f\n", chi_squared, RANDOM_PUBLIC_CHI_SQUARE);
            ret = -1;
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn random_public_tester() {
    const RANDOM_PUBLIC_TEST_CONST: usize = 11;
    const RANDOM_PUBLIC_TEST_ROUNDS: usize = 100;
    const RANDOM_PUBLIC_TEST_SEED: u64 = 0xDEAD_BEEF_CAFE_C001;
    const RANDOM_PUBLIC_CHI_SQUARE: f64 = 18.31;

    let mut r_count = [0usize; RANDOM_PUBLIC_TEST_CONST];

    crate::public_random_seed_64(RANDOM_PUBLIC_TEST_SEED, 1);

    for _ in 0..(RANDOM_PUBLIC_TEST_CONST * RANDOM_PUBLIC_TEST_ROUNDS) {
        let x = crate::picoquic_uniform_random(RANDOM_PUBLIC_TEST_CONST as u64);
        assert!(
            x < RANDOM_PUBLIC_TEST_CONST as u64,
            "Value {x} >= {RANDOM_PUBLIC_TEST_CONST}"
        );
        r_count[x as usize] += 1;
    }

    let mut chi_squared = 0.0;
    for count in r_count {
        let delta = RANDOM_PUBLIC_TEST_ROUNDS as f64 - count as f64;
        chi_squared += (delta * delta) / RANDOM_PUBLIC_TEST_ROUNDS as f64;
    }

    assert!(
        chi_squared <= RANDOM_PUBLIC_CHI_SQUARE,
        "Chi2 = {chi_squared}, larger than {RANDOM_PUBLIC_CHI_SQUARE}"
    );
}
```

## `picoquictest/tls_api_test.c:spurious_retransmit_test`
* C test-table name: `spurious_retransmit`
* C entry function: `spurious_retransmit_test`
* Rust test: `spurious_retransmit`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7521-7553`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a normal handshake/close through the generic helper; it does not set 50 ms link latency, simulate the 1 second silent period, or assert client/server nb_spurious == 0.
* Phase 5A fix note: Add a dedicated Rust test body matching the C scenario: initialize context, set both links to 50_000 us latency, run connection loop, drive 1 second of silence, close, and assert both spurious counters remain zero.
* Phase 5B analysis: Rust test now faithfully matches the C scenario and passes.
* Phase 5B fix note: Replaced generic handshake helper with dedicated setup: 50 ms link latency, handshake loop, 1 second silent simulator loop, close, and client/server nb_spurious == 0 assertions.

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    uint64_t next_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        test_ctx->c_to_s_link->microsec_latency = 50000ull;
        test_ctx->s_to_c_link->microsec_latency = 50000ull;

        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* simulate 1 second of silence */
    next_time = simulated_time + 1000000ull;
    while (ret == 0 && simulated_time < next_time && TEST_CLIENT_READY && TEST_SERVER_READY) {
        int was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, next_time, &was_active);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        /* verify the absence of any spurious retransmission */
        if (test_ctx->cnx_client->nb_spurious != 0) {
            ret = -1;
        } else if (test_ctx->cnx_server != NULL && test_ctx->cnx_server->nb_spurious != 0) {
            ret = -1;
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn spurious_retransmit() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    test_ctx.c_to_s_link.microsec_latency = 50_000;
    test_ctx.s_to_c_link.microsec_latency = 50_000;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    let next_time = Instant::from_ticks(simulated_time.ticks() + 1_000_000);
    while simulated_time < next_time && test_ctx.client_ready() && test_ctx.server_ready() {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            next_time,
            &mut was_active,
        )
        .expect("silent simulation round");
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("close");

    let client_spurious = test_ctx.cnx_client().nb_spurious;
    assert_eq!(client_spurious, 0, "client had spurious retransmissions");

    if test_ctx.has_cnx_server() {
        let server_spurious = test_ctx.cnx_server().nb_spurious;
        assert_eq!(server_spurious, 0, "server had spurious retransmissions");
    }
}
```

## `picoquictest/congestion_test.c:app_limit_cc_test`
* C test-table name: `app_limit_cc`
* C entry function: `app_limit_cc_test`
* Rust test: `app_limit_cc`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:956-966`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The six algorithms, max completion times, flow-control setup, and bytes-in-flight cap are present, but the Rust shared scenario helper ignores the C queue-delay argument and its verifier does not enforce stream completion or max_completion_time.
* Phase 5A fix note: Fix tls_api_one_scenario_body/body_verify to preserve queue_delay_max, run the C-equivalent scenario verification, and enforce max_completion_microsec so the per-algorithm limits are meaningful.
* Phase 5B analysis: Rust #[test] app_limit_cc is present, compiles under the Rust test harness, iterates the same six congestion algorithms with the same max completion times, and delegates to a helper that preserves the C API-level setup and qlog bytes-in-flight assertion. Any early scenario failure is incomplete Rust transport behavior for Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgos[] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_bbr_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr1_algorithm
    };
    uint64_t max_completion_times[] = {
        22000000,
        23500000,
        22000000,
        21000000,
        25000000,
        25000000
    };
    int ret = 0;

    for (size_t i = 0; i < sizeof(ccalgos) / sizeof(picoquic_congestion_algorithm_t*); i++) {
        ret = app_limit_cc_test_one(ccalgos[i], max_completion_times[i]);
        if (ret != 0) {
            DBG_PRINTF("Appplication limited congestion test fails for <%s>", ccalgos[i]->congestion_algorithm_id);
            break;
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn app_limit_cc() {
    let algo_names = ["newreno", "cubic", "dcubic", "bbr", "fastcc", "bbr1"];
    let max_completion_times: [u64; 6] = [
        22_000_000, 23_500_000, 22_000_000, 21_000_000, 25_000_000, 25_000_000,
    ];

    for (name, &max_time) in algo_names.iter().zip(max_completion_times.iter()) {
        let ccalgo = cc_algo(name);
        app_limit_cc_test_one(ccalgo, max_time);
    }
}
```

## `picoquictest/congestion_test.c:bbr_performance_test`
* C test-table name: `bbr_performance`
* C entry function: `bbr_performance_test`
* Rust test: `bbr_performance`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:811-816`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the same BBR performance parameters, but the shared Rust scenario verifier currently does not enforce the C helper's 10MB scenario completion and max completion time checks.
* Phase 5A fix note: Restore Rust scenario-body verification so bbr_performance fails unless the full scenario completes within 1,050,000 us.
* Phase 5B analysis: Rust already passes the C parameters and the shared scenario body now verifies all 10 MB streams plus the 1,050,000 us completion limit.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_time = 1050000;
    uint64_t latency = 10000;
    uint64_t jitter = 3000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 100;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Current Rust test body
```rust
fn bbr_performance() {
    let latency = 10_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(1_050_000, 100, latency, jitter, buffer);
}
```
