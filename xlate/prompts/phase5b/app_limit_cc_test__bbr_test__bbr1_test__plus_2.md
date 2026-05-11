# Phase 5B repair Rust test mismatches

You are repairing Phase 5A `needs_fix` entries.  The goal
is to make the Rust tests faithfully check the same behavior
as the C tests.

Rules:

* Edit Rust tests, Rust test helpers, and Rust test fixtures
  only: `rs/fq/src/tests/` and `rs/fq/tests/fixtures/`.
* Do not edit C sources.
* Do not weaken assertions, skip important C cases, or replace
  tests with placeholders.
* If the test already matches after closer inspection, report
  `ok` and do not edit source.
* Phase 5B is about test/API correspondence, not test success.
  The Rust test must exist, compile as a test, and be runnable
  by the Rust test harness, but it may fail arbitrarily early
  because the Rust library implementation is incomplete.
* Do not report `blocked` merely because the implementation
  returns the wrong state, fails a handshake, lacks protocol
  behavior, or would fail the test. Those are Phase 5C issues.
* Report `blocked` only when the faithful test cannot be
  written, compiled, or exposed as a runnable Rust test because
  the necessary Rust API/test-harness surface is missing or
  ambiguous.
* Do not run full `cargo test` in this pass. Use source review
  and, if needed, `cargo check --tests` for compile validation.

Owned Rust test file(s): `rs/fq/src/tests/congestion.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/congestion_test.c:app_limit_cc_test`
* C test-table name: `app_limit_cc`
* C entry function: `app_limit_cc_test`
* Rust test: `app_limit_cc`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Rust span: `rs/fq/src/tests/congestion.rs:959-969`
* Phase 5A analysis: The six algorithms, max completion times, flow-control setup, and bytes-in-flight cap are present, but the Rust shared scenario helper ignores the C queue-delay argument and its verifier does not enforce stream completion or max_completion_time.
* Phase 5A fix note: Fix tls_api_one_scenario_body/body_verify to preserve queue_delay_max, run the C-equivalent scenario verification, and enforce max_completion_microsec so the per-algorithm limits are meaningful.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust wrapper has the six algorithms, max times, flow-control setup, qlog check, and completion verification, but the merged shared tls_api_one_scenario_body still ignores the C queue_delay_max argument and calls tls_api_connection_loop with 0.
* Phase 5C fix note: Restore the Rust scenario helper/body-connect argument mapping so the 2*latency queue_delay_max is passed to tls_api_connection_loop.

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

## `picoquictest/congestion_test.c:bbr_test`
* C test-table name: `bbr`
* C entry function: `bbr_test`
* Rust test: `bbr`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Rust span: `rs/fq/src/tests/congestion.rs:784-787`
* Phase 5A analysis: The wrapper passes bbr, 3500000, 0, 0 like C, but the Rust shared verifier ignores max_completion_time and stream/error checks; the registered bbr entry also points at BASELINE_CC.
* Phase 5A fix note: Make scenario verification enforce stream completion/errors and completion_time <= max_completion_time; ensure bbr maps to the real translated BBR behavior before relying on this test.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Wrapper selects bbr and passes the C inputs, but the merged tls_api_one_scenario_body no longer propagates queue_delay_max; C runs this scenario with 20000 usec queue delay, current Rust hard-codes 0.
* Phase 5C fix note: Restore queue_delay_max propagation through tls_api_one_scenario_body/body_connect into tls_api_connection_loop.

### C test body
```c
{
    return congestion_control_test(picoquic_bbr_algorithm, 3500000, 0, 0);
}
```

### Current Rust test body
```rust
fn bbr() {
    let ccalgo = cc_algo("bbr");
    congestion_control_test(ccalgo, 3_500_000, 0, 0);
}
```

## `picoquictest/congestion_test.c:bbr1_test`
* C test-table name: `bbr1`
* C entry function: `bbr1_test`
* Rust test: `bbr1`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Rust span: `rs/fq/src/tests/congestion.rs:883-886`
* Phase 5A analysis: Rust selects bbr1 and passes the same parameters, but the shared Rust scenario verifier ignores stream-completion verification and max_completion_time that C enforces.
* Phase 5A fix note: Make the Rust scenario helper verify all streams completed and enforce the 3,600,000 us completion bound; preserve C delayed-start ordering so the selected algorithm is installed before the scenario starts.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust bbr1 selects bbr1 and keeps stream/time verification, but the shared Rust scenario helper still ignores the C queue_delay_max argument and calls tls_api_connection_loop with 0 instead of 20000; the context is also not using the C delayed-start setup before scenario connection start.
* Phase 5C fix note: Thread the congestion helper's queue_delay_max through tls_api_one_scenario_body/tls_api_connection_loop and preserve the delayed-start ordering used by the C harness while keeping the existing stream completion and 3600000 us checks.

### C test body
```c
{
    return congestion_control_test(picoquic_bbr1_algorithm, 3600000, 0, 0);
}
```

### Current Rust test body
```rust
fn bbr1() {
    let ccalgo = cc_algo("bbr1");
    congestion_control_test(ccalgo, 3_600_000, 0, 0);
}
```

## `picoquictest/congestion_test.c:bbr_performance_test`
* C test-table name: `bbr_performance`
* C entry function: `bbr_performance_test`
* Rust test: `bbr_performance`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Rust span: `rs/fq/src/tests/congestion.rs:814-819`
* Phase 5A analysis: Rust passes the same BBR performance parameters, but the shared Rust scenario verifier currently does not enforce the C helper's 10MB scenario completion and max completion time checks.
* Phase 5A fix note: Restore Rust scenario-body verification so bbr_performance fails unless the full scenario completes within 1,050,000 us.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust passes the C BBR performance parameters and verifies 10 MB completion time, but the shared scenario helper ignores the buffer_size/queue_delay_max argument, so the C network buffer cap is not exercised.
* Phase 5C fix note: Restore queue_delay_max propagation in tls_api_one_scenario_body/body_connect for the performance scenario.

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

## `picoquictest/congestion_test.c:bbr_slow_long_test`
* C test-table name: `bbr_slow_long`
* C entry function: `bbr_slow_long_test`
* Rust test: `bbr_slow_long`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Rust span: `rs/fq/src/tests/congestion.rs:825-830`
* Phase 5A analysis: The test passes the same BBR slow-long parameters, but the shared Rust scenario verifier ignores max_completion_microsec and omits the C scenario completion/stream verification, so the performance bound is not checked.
* Phase 5A fix note: Implement Rust tls_api_one_scenario_body_verify to enforce scenario completion/stream results and max completion time, then this test can rely on the same checks as C.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust derives the same slow-long parameters, but the C buffer value is passed as queue_delay_max; current merged Rust helper ignores that argument and runs with queue_delay_max 0.
* Phase 5C fix note: Restore use of the buffer/queue_delay_max argument when performance_test calls the shared scenario helper.

### C test body
```c
{
    uint64_t max_completion_time = 81000000;
    uint64_t latency = 300000;
    uint64_t jitter = 3000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 1;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Current Rust test body
```rust
fn bbr_slow_long() {
    let latency = 300_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(81_000_000, 1, latency, jitter, buffer);
}
```
