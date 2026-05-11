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

Owned Rust test file(s): `rs/fq/src/tests/pacing.rs`, `rs/fq/src/tests/transport_param.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/pacing_test.c:pacing_bbr_test`
* C test-table name: `pacing_bbr`
* C entry function: `pacing_bbr_test`
* Rust test: `pacing_bbr`
* Expected Rust file: `rs/fq/src/tests/pacing.rs`
* Rust span: `rs/fq/src/tests/pacing.rs:858-860`
* Phase 5A analysis: The pacing helper and thresholds match, but Rust's registered bbr descriptor is backed by BASELINE_CC, so the test does not actually exercise BBRv3 behavior like the C test.
* Phase 5A fix note: Wire the Rust bbr congestion descriptor to the translated BBR CongestionControl, or block this test until that implementation is available; also mirror the C helper's algorithm-number CID byte when touching the helper.
* Phase 5C outcome: blocked
* Phase 5C analysis: Rust pacing_bbr calls the intended BBR pacing helper and checks CID algorithm byte, selected CC, scenario completion, and loss target, but shared util.rs currently prevents runnable test exposure.
* Phase 5C fix note: Repair shared TestTlsApiCtx harness corruption; no pacing_bbr semantic mismatch observed.

### C test body
```c
{
    /* BBRv3 includes a short term loop that detects losses and tune the
    * sending rate accordingly. The packet losses cause startup to 
    * give up too soon, but this is fixed by probing up "quickly"
    * after exiting startup. The packet losses occur during startup
    * and during the probing periods.
    */
    int ret = pacing_cc_algotest(picoquic_bbr_algorithm, 900000, 160);
    return ret;
}
```

### Current Rust test body
```rust
fn pacing_bbr() {
    pacing_cc_algotest(&PACING_BBR_ALGORITHM, 900_000, 160);
}
```

## `picoquictest/pacing_test.c:pacing_newreno_test`
* C test-table name: `pacing_newreno`
* C entry function: `pacing_newreno_test`
* Rust test: `pacing_newreno`
* Expected Rust file: `rs/fq/src/tests/pacing.rs`
* Rust span: `rs/fq/src/tests/pacing.rs:882-884`
* Phase 5A analysis: Rust uses the intended newreno algorithm and the same 900000 us and loss_target 100 parameters, but the shared Rust verifier ignores the C target-time and scenario-completion checks.
* Phase 5A fix note: Repair tls_api_one_scenario_body_verify to verify delivered scenario data and completion_time <= target_time; preserve the newreno pacing parameters.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust NewReno pacing wrapper and scenario verifier match the C target-time, completion, and loss-bound checks, but shared util.rs compile damage prevents runnable exposure.
* Phase 5C fix note: Repair shared util.rs compile damage. No pacing_newreno-specific mismatch found.

### C test body
```c
{
    int ret = pacing_cc_algotest(picoquic_newreno_algorithm, 900000, 100);
    return ret;
}
```

### Current Rust test body
```rust
fn pacing_newreno() {
    pacing_cc_algotest(&PACING_NEWRENO_ALGORITHM, 900_000, 100);
}
```

## `picoquictest/transport_param_test.c:transport_param_log_test`
* C test-table name: `transport_param_log`
* C entry function: `transport_param_log_test`
* Rust test: `transport_param_log`
* Expected Rust file: `rs/fq/src/tests/transport_param.rs`
* Rust span: `rs/fq/src/tests/transport_param.rs:1047-1050`
* Phase 5A analysis: The Rust helper includes the eight fixed TP vectors, reference-file comparison, and client/server fuzz loops, but it logs via a nested test-only textlog_transport_extension_content implementation. The C test exercises the production picoquic_textlog_transport_extension_content, so the Rust test can pass without checking the translated textlog logger behavior.
* Phase 5A fix note: Have the Rust test/helper call the production Rust textlog transport-extension logger, or add/expose that translated implementation in rs/fq/src/textlog.rs and route the same fixed vectors and fuzz cases through it. Keep the reference comparison and client_param2/server_param2 fuzz coverage.
* Phase 5C outcome: blocked
* Phase 5C analysis: Rust test/helper covers fixed vectors, reference comparison, and client/server fuzz cases, but it calls crate::textlog::textlog_transport_extension_content, which is not present in the current textlog API, so the faithful test is not runnable.
* Phase 5C fix note: Expose or implement the production Rust textlog transport-extension logger required by the helper.

### C test body
```c
{
    FILE* F = NULL;
    int ret = 0;

    if ((F = picoquic_file_open(log_tp_test_file, "w")) == NULL) {
        fprintf(stderr, "failed to open file:%s\n", log_tp_test_file);
        ret = PICOQUIC_ERROR_INVALID_FILE;
    }

    if (F != NULL) {
        char log_tp_test_ref[512];

        transport_param_log_test_one(F, client_param1, sizeof(client_param1));
        transport_param_log_test_one(F, client_param2, sizeof(client_param2));
        transport_param_log_test_one(F, client_param3, sizeof(client_param3));
        transport_param_log_test_one(F, server_param1, sizeof(server_param1));
        transport_param_log_test_one(F, server_param2, sizeof(server_param2));
        transport_param_log_test_one(F, client_param4, sizeof(client_param4));
        transport_param_log_test_one(F, client_param5, sizeof(client_param5));
        transport_param_log_test_one(F, server_param3, sizeof(server_param3));

        fclose(F);

        ret = picoquic_get_input_path(log_tp_test_ref, sizeof(log_tp_test_ref), picoquic_solution_dir, LOG_TP_TEST_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the log TP ref file name.\n");
        } else {
            ret = picoquic_test_compare_text_files(log_tp_test_file, log_tp_test_ref);
        }
    }

    if (ret == 0)
    {
        DBG_PRINTF("Doing fuzz test of transport parameter logging into %s\n", log_tp_fuzz_file);

        ret = transport_param_log_fuzz_test(client_param2, sizeof(client_param2));

        if (ret == 0) {
            ret = transport_param_log_fuzz_test(server_param2, sizeof(server_param2));
        }

        DBG_PRINTF("Fuzz test of transport parameter was successful.\n", log_tp_fuzz_file);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn transport_param_log() {
    transport_param_log_test_one("log_tp_test.txt").expect("log_tp");
    compare_text_files("log_tp_test.txt", "picoquictest/log_tp_test_ref.txt").expect("compare_log");
}
```
