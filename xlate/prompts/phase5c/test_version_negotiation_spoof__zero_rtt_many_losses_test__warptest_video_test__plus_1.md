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

## `picoquictest/tls_api_test.c:test_version_negotiation_spoof`
* C test-table name: `version_negotiation_spoof`
* C entry function: `test_version_negotiation_spoof`
* Rust test: `version_negotiation_spoof`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8655-8672`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a normal TLS handshake via tls_api_test_with_loss; it never builds or injects spoofed Version Negotiation packets and does not cover modes 0 through 7 or the opposite expectation for mode 0.
* Phase 5A fix note: Add a Rust equivalent of test_version_negotiation_get_spoofed/test_version_negotiation_spoof_one. The test should expect mode 0 not to be ignored, and modes 1..7 to leave the client in the initial-sent state after VN injection.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, compiles, is harness-runnable, and expresses the C API-level contract for mode 0 versus modes 1..7. The known mode 7 state mismatch is a Phase 5C library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    if (test_version_negotiation_spoof_one(0) == 0) {
        DBG_PRINTF("%s", "VN spoof mode 0 has no effect");
        ret = -1;
    }

    for (int spoof_mode = 1; ret == 0 && spoof_mode < 8; spoof_mode++) {
        ret = test_version_negotiation_spoof_one(spoof_mode);
        if (ret != 0) {
            DBG_PRINTF("VN spoof mode %d caused failure", spoof_mode);
            ret = -1;
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn version_negotiation_spoof() {
    let state = version_negotiation_spoof_one(0).expect("version_negotiation_spoof mode 0");
    assert_eq!(
        state,
        State::ClientRenegotiate,
        "VN spoof mode 0 has no effect"
    );

    for spoof_mode in 1..8 {
        let state = version_negotiation_spoof_one(spoof_mode)
            .unwrap_or_else(|e| panic!("version_negotiation_spoof mode {spoof_mode}: {e:?}"));
        assert_eq!(
            state,
            State::ClientInitSent,
            "VN spoof mode {spoof_mode} caused failure"
        );
    }
}
```

## `picoquictest/tls_api_test.c:zero_rtt_many_losses_test`
* C test-table name: `zero_rtt_many_losses`
* C entry function: `zero_rtt_many_losses_test`
* Rust test: `zero_rtt_many_losses`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:9070-9086`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs 50 zero-RTT attempts, but it does not preserve the C loss pattern: C uses seed 0x1055ca45c001baba and 64 calls to picoquic_test_uniform_random(...,1000) per iteration to build a full 64-bit mask with ~30% drops; Rust uses a different one-step LCG seed and only the high byte, giving at most 8 active loss bits and a much lower effective drop rate.
* Phase 5A fix note: Update the Rust test to use the translated test_uniform_random helper, the C seed 0x1055ca45c001baba, and the same 50 x 64 loop that shifts loss_mask and sets a bit when uniform_random(1000) < 300.
* Phase 5B analysis: Rust #[test] is present, compiles under the Rust test harness, and matches the C API-level contract: same seed, 50x64 pseudo-random 30% loss-mask construction, early_loss assignment, and zero_rtt_test_one success assertion. The known iteration-0 runtime failure is a Phase 5C implementation/harness behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    uint64_t random_context = 0x1055ca45c001babaull;

    for (int i = 0; ret == 0 && i < 50; i++)
    {
        uint64_t loss_mask = 0;
        zero_rtt_test_t zrt = { 0 };

        for (int j = 0; j < 64; j++)
        {
            loss_mask <<= 1;

            if (picoquic_test_uniform_random(&random_context, 1000) < 300) {
                loss_mask |= 1;
            }
        }
        zrt.early_loss = loss_mask;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Handshake fails for mask %d, mask = %llx", i, (unsigned long long)loss_mask);
        }
    }
    return ret;
}
```

### Current Rust test body
```rust
fn zero_rtt_many_losses() {
    let mut random_context = 0x1055_ca45_c001_babau64;
    for i in 0..50 {
        let mut loss_mask = 0u64;
        for _ in 0..64 {
            loss_mask <<= 1;
            if test_uniform_random(&mut random_context, 1000) < 300 {
                loss_mask |= 1;
            }
        }
        zero_rtt_test_one(&ZeroRttTest {
            early_loss: loss_mask,
            ..Default::default()
        })
        .unwrap_or_else(|e| panic!("zero_rtt_many_losses i={i}, mask={loss_mask:016x}: {e:?}"));
    }
}
```

## `picoquictest/warptest.c:warptest_video_test`
* C test-table name: `warptest_video`
* C entry function: `warptest_video_test`
* Rust test: `warptest_video`
* Expected Rust file: `rs/fq/src/tests/warptest.rs`
* Current Rust span: `rs/fq/src/tests/warptest.rs:28-36`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Wrapper inputs match C, but Rust warptest_one is a synthetic counter loop after handshake. It does not create/receive per-frame unidirectional streams, parse frame headers, drive packet simulation, or validate real media delivery through QUIC as the C test does.
* Phase 5A fix note: Implement warptest_one around the translated QUIC simulation/callback path: configure contexts, generate video frames on uni streams, record receive-side stats, run until real completion, and apply the C delay/stat checks.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract: BBR, bandwidth 0.01, do_video true, and warptest_one(1). Earlier handshake/Initial-processing failure is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret;
    warptest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    ret = warptest_one(1, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn warptest_video() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        ..Default::default()
    };
    warptest_one(1, &spec).expect("warptest_video");
}
```

## `picoquictest/wifitest.c:wifi_bbr_hard_test`
* C test-table name: `wifi_bbr_hard`
* C entry function: `wifi_bbr_hard_test`
* Rust test: `wifi_bbr_hard`
* Expected Rust file: `rs/fq/src/tests/wifitest.rs`
* Current Rust span: `rs/fq/src/tests/wifitest.rs:150-161`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust spec values and suspension schedule match C, but the shared Rust scenario verifier only closes the connection and ignores stream-completion and target-time checks that C tls_api_one_scenario_body_verify performs. This weakens the Wi-Fi hard test's main success criteria.
* Phase 5A fix note: Restore Rust tls_api_one_scenario_body_verify/tls_api_one_scenario_verify behavior for Wi-Fi use: verify all scenario streams, callback errors, data-node pool state, and completion time <= target_time before closing.
* Phase 5B analysis: Rust wifi_bbr_hard is present as a #[test], compiles under the Rust test harness, constructs the same hard BBR WifiTestSpec as C, and calls the shared wifi_test_one verifier with the matching test id. Any callback/state or runtime behavior failure is Phase 5C, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_hard,
        3000,
        suspension_hard,
        picoquic_bbr_algorithm,
        NULL,
        4060000,
        0,
        0 };
    int ret = wifi_test_one(wifi_test_bbr_hard, &spec);

    return ret;
}
```

### Current Rust test body
```rust
fn wifi_bbr_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "bbr",
        cc_algo_option: None,
        target_time: 4_060_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR_HARD, &spec).expect("wifi_bbr_hard");
}
```
