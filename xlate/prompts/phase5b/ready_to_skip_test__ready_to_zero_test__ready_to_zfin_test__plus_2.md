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

Owned Rust test file(s): `rs/fq/src/tests/tls_api.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/tls_api_test.c:ready_to_skip_test`
* C test-table name: `ready_to_skip`
* C entry function: `ready_to_skip_test`
* Rust test: `ready_to_skip`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6768-6770`
* Phase 5A analysis: Rust passes option 3 but the helper ignores it and runs unrelated small stream transfers, so it does not exercise the prepare-to-send skip behavior covered by the C test.
* Phase 5A fix note: Add Rust support for the stream0 prepare-to-send test option, implement option 3 skip/no-data behavior, and run the C-equivalent q_and_r scenario with stream0_target 1000000 and matching timing parameters.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust calls ready_to_send_test_one(3), but the helper only range-checks the option and never stores or uses option 3, so it does not exercise the C PrepareToSend skip/no-data behavior.
* Phase 5C fix note: Implement stream0 option 3 PrepareToSend skip behavior and verify the 1,000,000-byte stream0 q_and_r scenario with the C timing parameters.

### C test body
```c
{
    int ret = ready_to_send_test_one(3);
    return ret;
}
```

### Current Rust test body
```rust
    }

    let queue_delay_max = 2 * test_ctx.c_to_s_link.microsec_latency;
```

## `picoquictest/tls_api_test.c:ready_to_zero_test`
* C test-table name: `ready_to_zero`
* C entry function: `ready_to_zero_test`
* Rust test: `ready_to_zero`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6776-6778`
* Phase 5A analysis: Rust calls ready_to_send_test_one(4), but that helper ignores the option and does not model the C stream0 prepare-to-send zero-byte path or C scenario sizes.
* Phase 5A fix note: Implement ready_to_send_test_one so option 4 drives the stream0 zero-byte prepare-to-send behavior, uses the C q_and_r scenario, and verifies completion like the C helper.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test is present, but ready_to_send_test_one(4) only validates the option and runs a generic stream0 scenario; it does not model the C option-4 zero-byte prepare-to-send path.
* Phase 5C fix note: Implement option-specific stream0 prepare-to-send behavior for option 4, including the zero-byte provide_stream_data_buffer path after the C threshold and continued completion checks.

### C test body
```c
{
    int ret = ready_to_send_test_one(4);
    return ret;
}
```

### Current Rust test body
```rust
    )?;

    if test_ctx.client_ready() && test_ctx.server_ready() {
```

## `picoquictest/tls_api_test.c:ready_to_zfin_test`
* C test-table name: `ready_to_zfin`
* C entry function: `ready_to_zfin_test`
* Rust test: `ready_to_zfin`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6784-6786`
* Phase 5A analysis: Rust passes option 2, but the helper ignores the option and does not exercise the C direct stream-0 prepare-to-send FIN behavior. It also uses a different small stream scenario and does not pass/use the C stream0_target path.
* Phase 5A fix note: Add stream0_test_option/prepare-to-send behavior for option 2, run the q_and_r scenario with stream0_target=1000000, and verify the zero-length FIN-after-data path completes like the C helper.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust calls ready_to_send_test_one(2), but the merged helper only validates the option range and never stores or uses option 2 in stream0 prepare-to-send behavior, so the zero-length FIN-after-data path is not checked.
* Phase 5C fix note: Add stream0_test_option or equivalent prepare-to-send harness behavior and have option 2 send the final zero-length FIN after stream0 data.

### C test body
```c
{
    int ret = ready_to_send_test_one(2);
    return ret;
}
```

### Current Rust test body
```rust

/// C: `random_public_tester_test` in `picoquictest/tls_api_test.c`.
///
```

## `picoquictest/tls_api_test.c:red_cubic_test`
* C test-table name: `red_cubic`
* C entry function: `red_cubic_test`
* Rust test: `red_cubic`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6800-6802`
* Phase 5A analysis: The wrapper arguments resemble the C call, but the Rust helper does not preserve the C RED test: it treats the C loss_target argument as MTU, ignores the algorithm id, omits RED AQM setup, uses a different scenario, and does not check observed retransmission loss.
* Phase 5A fix note: Repair red_cc_algotest to select Cubic, configure RED AQM/latency/bandwidth like C, run test_scenario_sustained, enforce target completion time, and assert observed server retransmissions are <= loss_target 225.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: red_cubic itself faithfully calls red_cc_algotest("cubic", 510000, 225), and the helper preserves the C RED AQM setup, sustained scenario, completion bound, and retransmission-loss assertion. However current merged tls_api.rs has duplicate top-level scenario constants, preventing this test module from being exposed as a runnable Rust test.
* Phase 5C fix note: De-duplicate the top-level TEST_SCENARIO_Q_AND_R and TEST_SCENARIO_VERY_LONG definitions in rs/fq/src/tests/tls_api.rs; no red_cubic body mismatch found.

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_cubic_algorithm, 510000, 225);
    return ret;
}
```

### Current Rust test body
```rust
    for _ in 0..(RANDOM_PUBLIC_TEST_CONST * RANDOM_PUBLIC_TEST_ROUNDS) {
        let x = crate::picoquic_uniform_random(RANDOM_PUBLIC_TEST_CONST as u64);
        assert!(
```

## `picoquictest/tls_api_test.c:red_fast_test`
* C test-table name: `red_fast`
* C entry function: `red_fast_test`
* Rust test: `red_fast`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6816-6818`
* Phase 5A analysis: Rust passes the visible scalar parameters, but red_cc_algotest ignores the algorithm id and does not configure RED AQM, latency/bandwidth, server FastCC, qlog/long log, or the retransmission loss threshold checked by C.
* Phase 5A fix note: Make Rust red_cc_algotest select the requested congestion algorithm, configure RED on both simulated links with the C latency/queue parameters, run the sustained scenario, verify target completion time, and assert server retransmissions do not exceed the loss target.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: red_fast delegates to the repaired RED/FastCC helper with the C parameters, but tls_api.rs duplicate definitions prevent the test module from compiling cleanly.
* Phase 5C fix note: Resolve duplicate top-level definitions in rs/fq/src/tests/tls_api.rs; no red_fast correspondence mismatch found.

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_fastcc_algorithm, 500000, 250);
    return ret;
}
```

### Current Rust test body
```rust
        chi_squared <= RANDOM_PUBLIC_CHI_SQUARE,
        "Chi2 = {chi_squared}, larger than {RANDOM_PUBLIC_CHI_SQUARE}"
    );
```
