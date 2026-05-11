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

## `picoquictest/tls_api_test.c:unidir_test`
* C test-table name: `unidir`
* C entry function: `unidir_test`
* Rust test: `unidir`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8462-8492`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs no unidirectional stream scenario, uses different timing/flow parameters, and lacks the C postcondition that both stream trees are empty.
* Phase 5A fix note: Add the two unidirectional stream descriptors, run with max_data 128000 and queue_delay_max 10000, enforce the 100000 us bound, and assert client/server stream_tree is empty after close.
* Phase 5B analysis: Rust #[test] is present, compiles under the test harness, and matches the C scenario inputs plus client/server post-close stream_tree assertions. Prior Generic runtime failure is a Phase 5C implementation/data-delivery issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;

    int ret = tls_api_one_scenario_init(&test_ctx, &simulated_time,
        0, NULL, NULL);

    if (ret == 0) {
        ret = tls_api_one_scenario_body(test_ctx, &simulated_time,
            test_scenario_unidir, sizeof(test_scenario_unidir), 0, 0, 128000, 10000,
            100000);
    }

    /* Verify that the unidir streams are properly closed. */
    if (ret == 0 && test_ctx->cnx_client != NULL && test_ctx->cnx_client->stream_tree.size != 0) {
        DBG_PRINTF("There are %d streams left open on client at the end of test.",
            test_ctx->cnx_client->stream_tree.size);
        ret = -1;
    }

    if (ret == 0 && test_ctx->cnx_server != NULL && test_ctx->cnx_server->stream_tree.size != 0) {
        DBG_PRINTF("There are %d streams left open on client at the end of test.",
            test_ctx->cnx_server->stream_tree.size);
        ret = -1;
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
fn unidir() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body_ex(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_UNIDIR,
        0,
        0,
        128_000,
        10_000,
        100_000,
        &[],
    )
    .expect("unidir");

    if let Some(client) = ctx.qclient.first_cnx_mut() {
        assert_eq!(
            client.stream_tree.len(),
            0,
            "unidir: streams left open on client"
        );
    }
    if ctx.has_cnx_server() {
        assert_eq!(
            ctx.cnx_server().stream_tree.len(),
            0,
            "unidir: streams left open on server"
        );
    }
}
```

## `picoquictest/tls_api_test.c:zero_rtt_delay_test`
* C test-table name: `zero_rtt_delay`
* C entry function: `zero_rtt_delay_test`
* Rust test: `zero_rtt_delay`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:9003-9023`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C sets long_data, first asserts that 0-RTT fails for a ticket age of 100000 seconds plus 1 second, then asserts success for 100000 seconds minus 2 seconds. Rust only runs one success case with extra_delay 100000 us, leaves long_data false, and does not check the rejection boundary.
* Phase 5A fix note: Add both delay-boundary cases with long_data=true: expect zero_rtt_test_one to fail for 100001000000 us and succeed for 99998000000 us.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract: long_data=true, reject nominal delay + 1s, then accept nominal delay - 2s. Any failure of the accepted boundary is a Phase 5C 0-RTT/PSK runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: No Rust test change needed; recorded this pass in COMMANDS.log.

### C test body
```c
{
    int ret = 0;
    int bad_ret;
    const uint64_t nominal_delay_sec = 100000;
    const uint64_t nominal_delay = nominal_delay_sec * 1000000;
    zero_rtt_test_t zrt = { 0 };
    zrt.long_data = 1;
    zrt.extra_delay = nominal_delay + 1000000;

    bad_ret = zero_rtt_test_one(&zrt);
    if (bad_ret == 0) {
        DBG_PRINTF("Zero RTT succeed despite delay = %" PRIu64, " + 1 second.", nominal_delay_sec);
        ret = -1;
    }
    else {
        zrt.extra_delay = nominal_delay - 2000000;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Zero RTT fails for delay = %" PRIu64, " - 2 seconds.", nominal_delay_sec);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn zero_rtt_delay() {
    const NOMINAL_DELAY_SEC: u64 = 100_000;
    const NOMINAL_DELAY: u64 = NOMINAL_DELAY_SEC * 1_000_000;

    assert!(
        zero_rtt_test_one(&ZeroRttTest {
            long_data: true,
            extra_delay: NOMINAL_DELAY + 1_000_000,
            ..Default::default()
        })
        .is_err(),
        "zero_rtt_delay accepted ticket age {NOMINAL_DELAY_SEC} seconds + 1 second"
    );

    zero_rtt_test_one(&ZeroRttTest {
        long_data: true,
        extra_delay: NOMINAL_DELAY - 2_000_000,
        ..Default::default()
    })
    .expect("zero_rtt_delay");
}
```

## `picoquictest/transport_param_test.c:transport_param_default_test`
* C test-table name: `transport_param_default`
* C entry function: `transport_param_default_test`
* Rust test: `transport_param_default`
* Expected Rust file: `rs/fq/src/tests/transport_param.rs`
* Current Rust span: `rs/fq/src/tests/transport_param.rs:721-745`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only tries a shortened list of IDs with value 0 and expects success; C covers 28 id/value/expected-return cases, including expected failures, nonzero values, and post-set field validation.
* Phase 5A fix note: Port the C tp_default_test_case table, assert each expected Ok/Err, and verify the corresponding default_tp field for successful cases.
* Phase 5B analysis: Rust test mirrors the C 28-case default-TP table, expected Ok/Err handling, fresh context per case, and tp_value_check-style API-visible field assertions. grease_quic_bit/address_discovery runtime divergence is a Phase 5C implementation issue, not a Phase 5B block. cargo check currently fails on unrelated missing textlog_transport_extension_content in util.rs.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    for (size_t i = 0; ret == 0 && i < nb_default_test_case; i++) {
        picoquic_quic_t quic = { 0 };
        int r = picoquic_set_default_tp_value(&quic, tp_default_test_case[i].tp_id, tp_default_test_case[i].tp_val);

        if (r != tp_default_test_case[i].ret) {
            ret = -1;
        }
        else if (r == 0) {
            ret = tp_value_check(&quic, tp_default_test_case[i].tp_id, tp_default_test_case[i].tp_val);
        }
        if (ret != 0) {
            DBG_PRINTF("param default test fails for test %zu: 0x%" PRIu64 ", 0x%"  PRIu64,
                i, tp_default_test_case[i].tp_id, tp_default_test_case[i].tp_val);
        }
    }
    return ret;        
}
```

### Current Rust test body
```rust
fn transport_param_default() {
    for (i, case) in DEFAULT_TP_TEST_CASES.iter().enumerate() {
        let mut quic = default_tp_test_context();
        let result = quic.set_default_tp_value(case.tp_id, case.tp_value);

        match (result, case.expected) {
            (Ok(()), DefaultTpExpected::Ok(field)) => {
                default_tp_value_check(i, case, quic.default_tp(), field);
            }
            (Err(_), DefaultTpExpected::Err) => {}
            (Ok(()), DefaultTpExpected::Err) => {
                panic!(
                    "default_tp[{i}] {}: set_default_tp_value({:#x}, {}) succeeded, expected error",
                    case.name, case.tp_id, case.tp_value
                );
            }
            (Err(err), DefaultTpExpected::Ok(_)) => {
                panic!(
                    "default_tp[{i}] {}: set_default_tp_value({:#x}, {}) failed: {err:?}",
                    case.name, case.tp_id, case.tp_value
                );
            }
        }
    }
}
```

## `picoquictest/wifitest.c:wifi_bbr1_test`
* C test-table name: `wifi_bbr1`
* C entry function: `wifi_bbr1_test`
* Rust test: `wifi_bbr1`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:111-116`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses the BBR1 default spec and Wi-Fi scenario, but passes a different test_id byte than C and relies on a verify helper that only closes the connection instead of checking transfer completion within target_time.
* Phase 5A fix note: Use the C-equivalent test id input or document the intentional CID-label change, and restore the scenario verification for completed streams plus the 2800000 us target.
* Phase 5B analysis: Rust test matches the C API-level contract: #[test], BBR1 default spec, basic Wi-Fi suspension, 2800000 us target, and WIFI_TEST_BBR CID seed. Runtime Generic failure is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_bbr1_algorithm, 2800000);
    int ret = wifi_test_one(wifi_test_bbr, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_bbr1() {
    let spec = default_spec("bbr1", SUSPENSION_BASIC, 2_800_000);
    // C's `wifi_bbr1_test` configures BBR1 but reuses `wifi_test_bbr`
    // as the CID seed passed to `wifi_test_one`.
    wifi_test_one(WIFI_TEST_BBR, &spec).expect("wifi_bbr1");
}
```

## `picoquictest/wifitest.c:wifi_reno_hard_test`
* C test-table name: `wifi_reno_hard`
* C entry function: `wifi_reno_hard_test`
* Rust test: `wifi_reno_hard`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:256-267`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust entry uses the same hard suspension list, latency, NewReno algorithm, no option, target time, receive-block flag, queue delay, and test ID as the C test. However the shared Rust wifi helper is materially weaker: its `tls_api_one_scenario_body_verify` only closes the connection and ignores the C checks for scenario completion and `target_time`, so this #[test] does not fully check the C test behavior.
* Phase 5A fix note: In Phase 5B, implement the Rust equivalent of C `tls_api_one_scenario_body_verify`/`tls_api_one_scenario_verify` in `rs/fq/src/tests/util.rs`, including stream completion/error checks and max completion time enforcement; keep the `wifi_reno_hard` spec values as-is.
* Phase 5B analysis: Rust wifi_reno_hard is present as a runnable #[test], matches the C spec values and wifi_test_one API contract. The callback/state counter failure is a Phase 5C runtime harness/library issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_hard,
        3000,
        suspension_hard,
        picoquic_newreno_algorithm,
        NULL,
        4250000,
        0,
        0 };
    int ret = wifi_test_one(wifi_test_reno_hard, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_reno_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "newreno",
        cc_algo_option: None,
        target_time: 4_250_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_RENO_HARD, &spec).expect("wifi_reno_hard");
}
```
