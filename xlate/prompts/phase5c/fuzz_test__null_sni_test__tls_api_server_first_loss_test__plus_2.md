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

## `picoquictest/stresstest.c:fuzz_test`
* C test-table name: `fuzz`
* C entry function: `fuzz_test`
* Rust test: `fuzz`
* Expected Rust file: `rs/fq/src/tests/stresstest.rs`
* Current Rust span: `rs/fq/src/tests/stresstest.rs:1018-1035`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-09`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs the stress loop with the fuzz wall-time limit but never installs or implements the basic_fuzzer callback, fuzz context, seeded randomness, packet mutation, or fuzz counters that define the C test.
* Phase 5A fix note: Add a Rust BasicFuzzer/Fuzz path for fuzz: seed 0xDEADBEEFBABACAFE ^ duration, track packets/fuzzed bytes/length changes/highest state, mutate packet bytes and lengths per C basic_fuzzer, and install it on stress clients during fuzz runs.
* Phase 5B analysis: Rust fuzz now exercises a C-shaped basic fuzzer during the stress loop instead of only using the fuzz wall-time limit.
* Phase 5B fix note: Added BasicFuzzer with C seed/counters/state gating/packet length and byte mutation rules, installed it on stress clients during fuzz runs, and asserted the fuzz path ran.

### C test body
```c
{
    basic_fuzzer_ctx_t fuzz_ctx;
    int ret = 0;

    fuzz_ctx.nb_packets = 0;
    fuzz_ctx.nb_fuzzed = 0;
    fuzz_ctx.nb_fuzzed_length = 0;
    fuzz_ctx.highest_state_fuzzed = 0;
    /* Random seed depends on duration, so different durations do not all start 
     * with exactly the same message sequences. */
    fuzz_ctx.random_context = 0xDEADBEEFBABACAFEull;
    fuzz_ctx.random_context ^= picoquic_stress_test_duration;

    ret = stress_or_fuzz_test(basic_fuzzer, &fuzz_ctx, picoquic_stress_test_duration, picoquic_stress_test_duration);

    DBG_PRINTF("Fuzzed %d packets out of %d, changed %d lengths, ret = %d\n",
        fuzz_ctx.nb_fuzzed, fuzz_ctx.nb_packets, fuzz_ctx.nb_fuzzed_length, ret);

    return ret;
}
```

### Current Rust test body
```rust
fn fuzz() {
    let duration: u64 = 60_000_000;
    let mut fuzz_ctx = BasicFuzzer::new(duration);

    stress_or_fuzz_test(duration, duration, Some(&mut fuzz_ctx)).expect("fuzz_test");

    assert!(fuzz_ctx.nb_packets > 0, "fuzzer was never called");
    assert!(fuzz_ctx.nb_fuzzed > 0, "fuzzer never mutated packet bytes");
    assert!(
        fuzz_ctx.nb_fuzzed_length > 0,
        "fuzzer never changed packet length"
    );
    assert_eq!(
        fuzz_ctx.highest_state_fuzzed,
        crate::State::Ready,
        "fuzzer did not observe the ready state"
    );
}
```

## `picoquictest/tls_api_test.c:null_sni_test`
* C test-table name: `null_sni`
* C entry function: `null_sni_test`
* Rust test: `null_sni`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5430-5432`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust wrapper passes None for SNI, but tls_api_test_with_loss ignores the SNI/ALPN inputs when creating the context and tls_api_test_with_loss_final does not verify SNI/ALPN, so this does not actually test a null-SNI connection like C does.
* Phase 5A fix note: Make the Rust helper pass the supplied SNI/ALPN into context creation, preserve None as no SNI, and verify the negotiated SNI/ALPN/final connection checks matching the C helper.
* Phase 5B analysis: Rust null_sni now creates the context with SNI=None and verifies C-equivalent final checks for transport parameters, SNI, ALPN, and negotiated version.
* Phase 5B fix note: Updated tls_api_test_with_loss and tls_api_test_with_loss_final helpers to preserve caller SNI/ALPN and validate final negotiated state before close.

### C test body
```c
{
    return tls_api_test_with_loss(NULL, PICOQUIC_INTERNAL_TEST_VERSION_1, NULL, PICOQUIC_TEST_ALPN);
}
```

### Current Rust test body
```rust
fn null_sni() {
    tls_api_test_with_loss(None, V1, None, Some(TEST_ALPN)).expect("null_sni");
}
```

## `picoquictest/tls_api_test.c:tls_api_server_first_loss_test`
* C test-table name: `SH_loss`
* C entry function: `tls_api_server_first_loss_test`
* Rust test: `sh_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1040-1042`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes loss mask 14, but tls_api_loss_test ignores its _loss_mask parameter and always runs tls_api_test_with_loss with None/no loss, so it does not drop the first server flight packet.
* Phase 5A fix note: Thread the supplied loss mask into tls_api_test_with_loss/connection_loop so packet 14 is actually lost and recovery is verified.
* Phase 5B analysis: Rust test body was correct, but the helper discarded the mask and did not preserve the C shared loss-mask sequence across both links.
* Phase 5B fix note: Threaded the initial mask through tls_api_loss_test and synchronized one shared loss mask around simulated submit/admit operations in the TLS API loops.

### C test body
```c
{
    return tls_api_loss_test(14ull);
}
```

### Current Rust test body
```rust
fn sh_loss() {
    tls_api_loss_test(14).expect("sh_loss");
}
```

## `picoquictest/cert_verify_test.c:cert_verify_bad_sni_test`
* C test-table name: `cert_verify_bad_sni`
* C entry function: `cert_verify_bad_sni_test`
* Rust test: `cert_verify_bad_sni`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Current Rust span: `rs/fq/src/tests/cert_verify.rs:54-62`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The fixture inputs match, but the Rust helper expects tls_api_connection_loop to return Err, while C treats either loop error or not-both-ready as the expected rejection.
* Phase 5A fix note: Mirror C expect_success logic: after the loop, classify lack of client/server ready as failure/rejection; success should require both ready.
* Phase 5B analysis: Current Rust helper already mirrors C: success requires tls_api_connection_loop to return Ok and both client/server ready; rejection includes loop error or not-both-ready.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = cert_verify_test_one(0, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_BAD_SNI);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_bad_sni() {
    cert_verify_test_one(
        false,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_BAD_SNI),
    );
}
```

## `picoquictest/congestion_test.c:bbr_long_test`
* C test-table name: `bbr_long`
* C entry function: `bbr_long_test`
* Rust test: `bbr_long`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:795-798`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The BBR wrapper and congestion_long setup match, but the shared Rust tls_api_one_scenario_body_verify ignores the max-completion bound and full scenario verification that the C helper enforces.
* Phase 5A fix note: Implement/use a Rust tls_api_one_scenario_body_verify equivalent that validates scenario stream completion/callback state/data pools and checks elapsed completion time against max_completion_microsec before close.
* Phase 5B analysis: Rust bbr_long already uses congestion_long_test, and the shared verifier now checks scenario completion/callback error state/data pools plus the 15,000,000 usec completion bound before close, matching the C helper behavior.
* Phase 5B fix note: 

### C test body
```c
{
    return congestion_long_test(picoquic_bbr_algorithm);
}
```

### Current Rust test body
```rust
fn bbr_long() {
    let ccalgo = cc_algo("bbr");
    congestion_long_test(ccalgo);
}
```
