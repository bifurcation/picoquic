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

## `picoquictest/stresstest.c:fuzz_initial_test`
* C test-table name: `fuzz_initial`
* C entry function: `fuzz_initial_test`
* Rust test: `fuzz_initial`
* Expected Rust file: `rs/fq/src/tests/stresstest.rs`
* Current Rust span: `rs/fq/src/tests/stresstest.rs:1039-1075`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a simplified stress loop for 2x duration and 4x wall time. It does not create the initial_fuzzer_ctx, seed it, pass an initial_fuzzer callback, or exercise the skip-frame based initial-packet mutations from the C test.
* Phase 5A fix note: Implement/port the initial_fuzzer context and callback behavior and pass it through the Rust stress_or_fuzz_test path for fuzz_initial, preserving the seed and duration choices.
* Phase 5B analysis: Rust now seeds and runs an initial-packet fuzzer matching the C test's skip-frame mutation sequence before falling back to random initial-packet byte fuzzing.
* Phase 5B fix note: Added InitialFuzzer with the C skip-frame vectors, append/prepend/replace mutation passes, C seed choice, random fallback, and wired fuzz_initial through stress_or_fuzz_test with assertions that all passes ran.

### C test body
```c
{
    initial_fuzzer_ctx_t fuzz_ctx;
    int ret = 0;

    memset(&fuzz_ctx, 0, sizeof(initial_fuzzer_ctx_t));
    fuzz_ctx.random_context = 0x01234567DEADBEEFull;
    fuzz_ctx.random_context ^= picoquic_stress_test_duration;

    ret = stress_or_fuzz_test(initial_fuzzer, &fuzz_ctx, 2*picoquic_stress_test_duration, 4*picoquic_stress_test_duration);

    return ret;
}
```

### Current Rust test body
```rust
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
```

## `picoquictest/tls_api_test.c:perflog_test`
* C test-table name: `perflog`
* C entry function: `perflog_test`
* Rust test: `perflog`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5911-5911`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The mapped C body is a no-op return, while Rust runs a normal TLS connection. The non-Win32 C branch also sets up performance logs, runs a sustained scenario, and compares generated logs; Rust does none of that.
* Phase 5A fix note: Either correct the map to the intended non-Win32 C body and port the perflog setup/scenario/reference comparisons, or make this mapped no-op test return Ok without exercising unrelated TLS behavior.
* Phase 5B analysis: Rust now matches the supplied mapped C no-op branch instead of running an unrelated TLS connection.
* Phase 5B fix note: Changed perflog test to an empty passing test and updated its comment to identify the mapped platform-guard C branch.

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Current Rust test body
```rust
fn perflog() {}
```

## `picoquictest/tls_api_test.c:tls_api_silence_test`
* C test-table name: `silence_test`
* C entry function: `tls_api_silence_test`
* Rust test: `silence_test`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7479-7514`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs the generic handshake/close helper; it skips the 5-second silent simulation and the client/server retransmission-count assertions.
* Phase 5A fix note: Add a Rust silence-specific body that handshakes, advances simulated time for 5 seconds while ready, closes, and asserts both retransmission totals remain zero.
* Phase 5B analysis: Rust test now faithfully covers the C silence-specific behavior.
* Phase 5B fix note: Replaced generic handshake/close helper with handshake, 5-second silent simulation while both endpoints are ready, close, and zero retransmission assertions for client and server.

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    uint64_t next_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* simulate 5 seconds of silence */
    next_time = simulated_time + 5000000;
    while (ret == 0 && simulated_time < next_time && TEST_CLIENT_READY && TEST_SERVER_READY) {
        int was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, next_time, &was_active);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        /* verify the absence of any spurious retransmission */
        if (test_ctx->cnx_client->nb_retransmission_total != 0) {
            ret = -1;
        } else if (test_ctx->cnx_server != NULL && test_ctx->cnx_server->nb_retransmission_total != 0) {
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
fn silence_test() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    let next_time = Instant::from_ticks(simulated_time.ticks() + 5_000_000);
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

    let client_retransmissions = test_ctx.cnx_client().nb_retransmission_total;
    assert_eq!(
        client_retransmissions, 0,
        "client had spurious retransmissions"
    );

    if test_ctx.has_cnx_server() {
        let server_retransmissions = test_ctx.cnx_server().nb_retransmission_total;
        assert_eq!(
            server_retransmissions, 0,
            "server had spurious retransmissions"
        );
    }
}
```

## `picoquictest/cert_verify_test.c:cert_verify_null_test`
* C test-table name: `cert_verify_null`
* C entry function: `cert_verify_null_test`
* Rust test: `cert_verify_null`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Current Rust span: `rs/fq/src/tests/cert_verify.rs:67-75`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Inputs match, but Rust only asserts tls_api_connection_loop success; C also fails if client/server are not both ready after the loop.
* Phase 5A fix note: Update cert_verify_test_one to treat Ok-but-not-ready as failure, matching C TEST_CLIENT_READY and TEST_SERVER_READY logic.
* Phase 5B analysis: Current Rust helper already matches C: success requires tls_api_connection_loop Ok plus both client_ready and server_ready.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = cert_verify_test_one(1, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        NULL, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_null() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        None,
        Some(TEST_SNI),
    );
}
```

## `picoquictest/congestion_test.c:bbr_one_second_test`
* C test-table name: `bbr_one_second`
* C entry function: `bbr_one_second_test`
* Rust test: `bbr_one_second`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:833-838`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper preserves the C inputs, but the shared Rust scenario verifier ignores max_completion_microsec and does not verify scenario completion like the C helper does, so the 90,000,000us performance bound is not actually checked.
* Phase 5A fix note: Implement/use a Rust tls_api_one_scenario_body_verify equivalent that verifies stream/scenario completion and enforces completion_time <= max_completion_microsec.
* Phase 5B analysis: Current Rust path already passes the C inputs through and `tls_api_one_scenario_body_verify` checks stream completion plus `completion_time <= max_completion_microsec`, so the 90,000,000us bound is enforced.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t max_completion_time = 90000000;
    uint64_t latency = 1000000;
    uint64_t jitter = 3000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 1;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Current Rust test body
```rust
fn bbr_one_second() {
    let latency = 1_000_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(90_000_000, 1, latency, jitter, buffer);
}
```
