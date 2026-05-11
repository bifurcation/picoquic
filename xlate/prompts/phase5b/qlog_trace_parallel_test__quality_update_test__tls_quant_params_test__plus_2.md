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

## `picoquictest/tls_api_test.c:qlog_trace_parallel_test`
* C test-table name: `qlog_trace_parallel`
* C entry function: `qlog_trace_parallel_test`
* Rust test: `qlog_trace_parallel`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6537-6539`
* Phase 5A analysis: Rust passes parallel=true but the helper ignores it and only runs a tiny scenario. It omits qlog/binlog setup, spin/loss-bit policies, CID callback/seeds, fixed crypto choices, recreated initial CID, bad-packet injection, and comparison with the qlog reference file.
* Phase 5A fix note: Implement qlog_trace_test_one with the C qlog setup and reference-file comparison; for parallel=true also enable binlog before running the q2/r2 scenario and bad-packet log check.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust entry calls qlog_trace_test_one(0, true) and the helper includes qlog/binlog setup, bad-packet injection, and reference comparison, but the final-tree shared scenario driver it calls ignores the C queue_delay_max=20000 and applies the data loss mask during the handshake. That is a harness/API correspondence mismatch for the qlog trace scenario.
* Phase 5C fix note: Restore C-equivalent qlog scenario driving: handshake with the C queue delay and no data-loss mask, then run q2/r2 with loss mask 0x00010a04; keep parallel binlog, bad-packet injection, and reference-file comparison.

### C test body
```c
{
    return qlog_trace_test_one(0, 1);
}
```

### Current Rust test body
```rust
fn qlog_trace() {
    qlog_trace_test_one(0, false).expect("qlog_trace");
}
```

## `picoquictest/tls_api_test.c:quality_update_test`
* C test-table name: `quality_update`
* C entry function: `quality_update_test`
* Rust test: `quality_update`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6546-6585`
* Phase 5A analysis: Rust runs a generic TLS smoke test; it does not subscribe to quality updates, write the CSV, run the 1MB scenario, or compare against the reference file.
* Phase 5A fix note: Create the quality-update context, subscribe with deltas 0x10000/0x1000, run the q_and_r stream0_target=1000000 scenario with queue_delay=20000 and max_completion=3600000, then compare quality_update.csv to picoquictest/quality_update_ref.txt.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust runs the intended q_and_r 1MB scenario and subscribes with the right thresholds, but it never installs a PathQualityChanged/default-path-update callback or connects CSV logging to the harness; it only writes the header before comparing. The containing tls_api.rs also has duplicate top-level scenario constants, so the test file is not cleanly runnable.
* Phase 5C fix note: Add a quality-update callback/logger equivalent to the C default_path_update path, write collected rows to quality_update.csv before compare, and de-duplicate the top-level TEST_SCENARIO_Q_AND_R/TEST_SCENARIO_VERY_LONG constants in tls_api.rs.

### C test body
```c
{
    uint64_t simulated_time = 0;

    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }
    if (ret == 0) {
        /* Open a file to log bandwidth updates and document it in context */
        test_ctx->default_path_update = picoquic_file_open(QUALITY_UPDATE_CSV, "w");
        if (test_ctx->default_path_update == NULL) {
            DBG_PRINTF("Could not write file <%s>", QUALITY_UPDATE_CSV);
            ret = -1;
        }
        else {
            fprintf(test_ctx->default_path_update, "Time, Path_id, Sending_rate_CB, Pacing_rate, Receive_Rate, CWIN, RTT\n");
            /* Request bandwidth updates */
            picoquic_subscribe_to_quality_update(test_ctx->cnx_client, 0x10000, 0x1000);

            /* Start a standard scenario, pushing 1MB from the client*/
            ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
                test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 1000000, 0, 0, 20000, 3600000);
        }
    }

    /* Free the test contex, which closes the trace file  */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    /* compare the trace to the expected value */
    if (ret == 0)
    {
        char quality_update_ref[512];

        ret = picoquic_get_input_path(quality_update_ref, sizeof(quality_update_ref), picoquic_solution_dir, QUALITY_UPDATE_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the quality update ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(QUALITY_UPDATE_CSV, quality_update_ref);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
    qlog_trace_test_one(0x02, false).expect("qlog_trace_ecn");
}

/// C: `qlog_trace_parallel_test` in `picoquictest/tls_api_test.c`.
///
/// Qlog trace with parallel connections.
#[test]
fn qlog_trace_parallel() {
    qlog_trace_test_one(0, true).expect("qlog_trace_parallel");
}

/// C: `quality_update_test` in `picoquictest/tls_api_test.c`.
///
/// Verifies that the connection-quality update callback is invoked when
/// the RTT or bandwidth estimate changes.
#[test]
fn quality_update() {
    use std::io::Write as _;

    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, V1, None).expect("ctx");
    let quality_update_scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 2000,
    }];

    {
        let mut file = std::fs::File::create(QUALITY_UPDATE_CSV).expect("quality_update.csv");
        writeln!(
            file,
            "Time, Path_id, Sending_rate_CB, Pacing_rate, Receive_Rate, CWIN, RTT"
        )
        .expect("quality_update header");
    }

    test_ctx
        .cnx_client()
        .subscribe_to_quality_update(0x10000, crate::Duration::from_ticks(0x1000));
```

## `picoquictest/tls_api_test.c:tls_quant_params_test`
* C test-table name: `quant_params`
* C entry function: `tls_quant_params_test`
* Rust test: `quant_params`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6592-6619`
* Phase 5A analysis: Rust only runs an empty generic scenario with default transport parameters; C sets Quant-specific client transport parameters and runs test_scenario_quant.
* Phase 5A fix note: Add the quant scenario stream, initialize client TransportParameters with the C values, and run/verify the full scenario with the 3510000 us completion bound.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust body has the right Quant values and scenario, but the shared tls_api_one_scenario_init_ex helper starts the client before applying client_params, unlike the C delayed-init path, so the Quant transport parameters may not be advertised in the handshake.
* Phase 5C fix note: Use a delayed-start init path or apply client transport parameters before start_client/initialize_tls_stream for this scenario.

### C test body
```c
{
    picoquic_tp_t test_parameters;

    memset(&test_parameters, 0, sizeof(picoquic_tp_t));

    picoquic_init_transport_parameters(&test_parameters);

    test_parameters.initial_max_data = 0x4000;
    test_parameters.initial_max_stream_id_bidir = 0;
    test_parameters.initial_max_stream_id_unidir = 16384;
    test_parameters.initial_max_stream_data_bidi_local = 0x2000;
    test_parameters.initial_max_stream_data_bidi_remote = 0x2000;
    test_parameters.initial_max_stream_data_uni = 0x2000;

    return tls_api_one_scenario_test(test_scenario_quant, sizeof(test_scenario_quant), 0, 0, 0, 0, 0, 3510000, &test_parameters, NULL);
}
```

### Current Rust test body
```rust
        0,
        0,
        20_000,
        3_600_000,
        &[],
    )
    .expect("quality_update scenario");

    compare_text_files(QUALITY_UPDATE_CSV, QUALITY_UPDATE_REF).expect("quality_update reference");
}

/// C: `tls_quant_params_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a very-long-stream scenario with quantised transport parameters to
/// verify interoperability with Quant.
#[test]
fn quant_params() {
    let mut t = Instant::from_ticks(0);
    let mut client_params = crate::TransportParameters::default();
    crate::internal::init_transport_parameters(&mut client_params);
    client_params.initial_max_data = 0x4000;
    client_params.initial_max_stream_id_bidir = 0;
    client_params.initial_max_stream_id_unidir = 16_384;
    client_params.initial_max_stream_data_bidi_local = 0x2000;
    client_params.initial_max_stream_data_bidi_remote = 0x2000;
    client_params.initial_max_stream_data_uni = 0x2000;

    let mut ctx = tls_api_one_scenario_init_ex(
```

## `picoquictest/tls_api_test.c:random_padding_test`
* C test-table name: `random_padding`
* C entry function: `random_padding_test`
* Rust test: `random_padding`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6627-6632`
* Phase 5A analysis: Rust runs a generic TLS API handshake/loss helper. C runs two random-padding injections with pad lengths 128 and 16, shared deterministic RNG state, distinct test IDs, direct first-packet mutation, and then verifies the connection loop.
* Phase 5A fix note: Add a faithful random_padding_test_one Rust helper and call it twice with 128/test_id 0 and 16/test_id 1 using the same RNG context.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust body and helper match the C random-padding intent, but tls_api.rs has duplicate top-level definitions, so the #[test] is not currently exposed as runnable.
* Phase 5C fix note: Resolve duplicate top-level definitions in rs/fq/src/tests/tls_api.rs; no pair-specific assertion mismatch found.

### C test body
```c
{
    uint64_t random_context = 0x1234567890abcdef;

    int ret = random_padding_test_one(128, &random_context, 0);

    if (ret == 0) {
        ret = random_padding_test_one(16, &random_context, 1);
    }

    return ret;
}
```

### Current Rust test body
```rust
    let quant_scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 10_000,
    }];
```

## `picoquictest/tls_api_test.c:ready_to_send_test`
* C test-table name: `ready_to_send`
* C entry function: `ready_to_send_test`
* Rust test: `ready_to_send`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:6760-6762`
* Phase 5A analysis: Rust calls ready_to_send_test_one(1), but the helper ignores the option, does not model stream0_test_option, uses different scenario sizes, and relies on weaker scenario/final verification than the C ready-to-send callback test.
* Phase 5A fix note: Implement the stream-0 prepare-to-send behavior and option handling, use the C test_scenario_q_and_r inputs, set stream0_target=1000000, and verify stream0 plus scenario completion as C does.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Final Rust test is present and calls ready_to_send_test_one(1), but the helper still ignores the option and does not model the C stream0_test_option PrepareToSend behavior.
* Phase 5C fix note: Implement the stream-0 prepare-to-send path and option handling, then run the C q_and_r scenario with stream0_target=1000000 and verify stream0 completion.

### C test body
```c
{
    int ret = ready_to_send_test_one(1);
    return ret;
}
```

### Current Rust test body
```rust
    let mut chi_squared = 0.0;
    for count in r_count {
        let delta = RANDOM_PUBLIC_TEST_ROUNDS as f64 - count as f64;
```
