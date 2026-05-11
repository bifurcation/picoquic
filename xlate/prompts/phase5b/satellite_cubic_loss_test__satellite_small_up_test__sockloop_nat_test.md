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

Owned Rust test file(s): `rs/fq/src/tests/satellite.rs`, `rs/fq/src/tests/sockloop.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/satellite_test.c:satellite_cubic_loss_test`
* C test-table name: `satellite_cubic_loss`
* C entry function: `satellite_cubic_loss_test`
* Rust test: `satellite_cubic_loss`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Rust span: `rs/fq/src/tests/satellite.rs:446-461`
* Phase 5A analysis: The wrapper parameters match C, but the Rust satellite helper passes data_size into an ignored helper parameter, uses an empty scenario, never sets stream0_target, and ignores the completion-time check, so it does not test the C 100MB lossy cubic transfer within 7.5s.
* Phase 5A fix note: Map data_size to stream0_target, drive and verify the stream0 transfer with the loss mask, and enforce max_completion_time in the Rust scenario helper/verification.
* Phase 5C outcome: blocked
* Phase 5C analysis: Rust satellite_cubic_loss matches the C cubic/loss parameters and helper now maps data_size to stream0_target, loss mask, and max completion verification, but shared util.rs currently prevents runnable test exposure.
* Phase 5C fix note: Repair shared TestTlsApiCtx harness corruption; no satellite_cubic_loss semantic mismatch observed.

### C test body
```c
{
    /* Should be less than 10 sec per draft etosat, but cubic is a bit slower */
    return satellite_test_one(picoquic_cubic_algorithm, 100000000, 7500000, 250, 3, 0, 1, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_cubic_loss() {
    let cubic = satellite_ccalgo("cubic");
    satellite_test_one(
        cubic,
        100_000_000,
        7_500_000,
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

## `picoquictest/satellite_test.c:satellite_small_up_test`
* C test-table name: `satellite_small_up`
* C entry function: `satellite_small_up_test`
* Rust test: `satellite_small_up`
* Expected Rust file: `rs/fq/src/tests/satellite.rs`
* Rust span: `rs/fq/src/tests/satellite.rs:370-385`
* Phase 5A analysis: The visible call arguments match C, but the same Rust helper issue means the intended 100MB BBR transfer over the small upstream path is not actually queued or verified, and the 400s completion bound is ignored.
* Phase 5A fix note: Map data_size to stream0_target, ensure the stream0 transfer is initialized and verified, and enforce the max_completion_time bound for this no-loss BBR case.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test body and satellite helper match the C intent, but the shared Rust test harness is not currently runnable due merged util.rs duplicate fields/misplaced helper definitions.
* Phase 5C fix note: Repair rs/fq/src/tests/util.rs merge damage so the faithful satellite test can compile/run as a Rust test.

### C test body
```c
{
    /* Should be less than 420 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 400000000, 2, 10, 0, 0, 0, 0, 0, 0);
}
```

### Current Rust test body
```rust
fn satellite_small_up() {
    let bbr = satellite_ccalgo("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        400_000_000,
        2,
        10,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/sockloop_test.c:sockloop_nat_test`
* C test-table name: `sockloop_nat`
* C entry function: `sockloop_nat_test`
* Rust test: `sockloop_nat`
* Expected Rust file: `rs/fq/src/tests/sockloop.rs`
* Rust span: `rs/fq/src/tests/sockloop.rs:1018-1027`
* Phase 5A analysis: Rust sets the same spec fields, but sockloop_test_one_result never programs or verifies spec.scenario and its received-finished check only requires ready/established. The C test sends the 1M scenario, verifies completion, and validates NAT migration.
* Phase 5A fix note: Make the Rust sockloop driver initialize and verify the scenario streams like C, then keep the IPv4 extra-socket/prefer-extra-socket force_migration=1 NAT checks.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust sockloop_nat preserves the C spec fields and scenario/migration checks, but it imports the currently broken shared util test harness.
* Phase 5C fix note: Repair rs/fq/src/tests/util.rs merge damage; sockloop_nat body itself matches the C test intent.

### C test body
```c
{
    sockloop_test_spec_t spec;
    sockloop_test_set_spec(&spec, 6);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = sockloop_test_scenario_1M;
    spec.scenario_size = sizeof(sockloop_test_scenario_1M);
    spec.extra_socket_required = 1;
    spec.prefer_extra_socket = 1;
    spec.force_migration = 1;

    return(sockloop_test_one(&spec));
}
```

### Current Rust test body
```rust
fn sockloop_nat() {
    let mut spec = SockloopTestSpec::new(6);
    spec.af = AF_INET;
    spec.socket_buffer_size = 0xffff;
    spec.scenario = SOCKLOOP_SCENARIO_1M;
    spec.extra_socket_required = true;
    spec.prefer_extra_socket = true;
    spec.force_migration = 1;
    sockloop_test_one(&spec);
}
```
