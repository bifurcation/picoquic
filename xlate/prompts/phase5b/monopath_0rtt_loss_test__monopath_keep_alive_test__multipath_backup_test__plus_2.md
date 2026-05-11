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

Owned Rust test file(s): `rs/fq/src/tests/multipath.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/multipath_test.c:monopath_0rtt_loss_test`
* C test-table name: `monopath_0rtt_loss`
* C entry function: `monopath_0rtt_loss_test`
* Rust test: `monopath_0rtt_loss_2`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1436-1446`
* Phase 5A analysis: The outer Rust loop matches i=1..15, early_loss, and do_multipath, but the Rust zero_rtt_test_one weakens the C behavior: it starts the client before setting some parameters and manually seeds 0RTT/PSK and multipath flags, so packet-loss recovery and multipath negotiation are not actually observed.
* Phase 5A fix note: Repair zero_rtt_test_one to use delayed init like C, inject early_loss through the simulator, and assert actual 0RTT counters, PSK handshake, ticket handling, and multipath negotiation without manual state seeding.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust loop matches the C early_loss range and do_multipath flag, but zero_rtt_test_one still starts the client before applying the delayed-init 0-RTT parameters, then starts it again; that does not faithfully mirror the C harness setup.
* Phase 5C fix note: Use delayed client start with ticket-key setup, apply multipath/no-coal/retry/parameter changes before start_client, and repair the shared util.rs compile damage.

### C test body
```c
{
    int ret = 0;

    for (unsigned int i = 1; ret == 0 && i < 16; i++) {
        zero_rtt_test_t zrt = { 0 };
        zrt.early_loss = 1ull << i;
        zrt.do_multipath = 1;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Monopath 0 RTT test fails when packet #%d is lost.\n", i);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn monopath_0rtt_loss_2() {
    for i in 1u32..16 {
        let zrt = ZeroRttTest {
            early_loss: 1u64 << i,
            do_multipath: true,
            ..ZeroRttTest::default()
        };
        zero_rtt_test_one(&zrt)
            .unwrap_or_else(|_| panic!("monopath_0rtt_loss_2 fails at packet #{i}"));
    }
}
```

## `picoquictest/multipath_test.c:monopath_keep_alive_test`
* C test-table name: `monopath_keep_alive`
* C entry function: `monopath_keep_alive_test`
* Rust test: `monopath_keep_alive`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1462-1464`
* Phase 5A analysis: The Rust wrapper and helper follow the C keep-alive scenario, including parameters, scenario transfer, NAT change, PING loop, readiness checks, and extended completion bound, but the final shared verifier does not enforce the C stream completion or time-bound verification.
* Phase 5A fix note: Fix the shared Rust scenario/body verifier; the monopath keep-alive-specific flow appears acceptable once that verifier checks the same completion conditions as C.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust test calls the right monopath keep-alive harness and preserves the PING/NAT-change/final verification flow, but the merged shared Rust test harness is currently malformed and not runnable.
* Phase 5C fix note: Repair rs/fq/src/tests/util.rs shared harness scope/duplicate-field regression; no pair-specific C/Rust intent mismatch found.

### C test body
```c
{
    return monopath_test_one(monopath_keep_alive);
}
```

### Current Rust test body
```rust
fn monopath_keep_alive() {
    monopath_test_one(MonopathTestId::KeepAlive);
}
```

## `picoquictest/multipath_test.c:multipath_backup_test`
* C test-table name: `multipath_backup`
* C entry function: `multipath_backup_test`
* Rust test: `multipath_backup`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1566-1568`
* Phase 5A analysis: The wrapper and Backup enum value match C, and Rust includes backup-path assertions, but it still relies on placeholder shared helpers that skip scenario completion and max-completion checks from C.
* Phase 5A fix note: Restore shared scenario verification so multipath_test_one checks completed transfers, payloads, close result, and max_completion_microsec in addition to backup-path assertions.
* Phase 5C outcome: blocked
* Phase 5C analysis: Rust wrapper/helper match the C Backup case, including max-completion scenario verification and backup-path assertions, but shared util.rs currently prevents runnable test exposure.
* Phase 5C fix note: Repair shared TestTlsApiCtx harness corruption; no multipath_backup semantic mismatch observed.

### C test body
```c
{
    uint64_t max_completion_microsec = 2000000;

    return multipath_test_one(max_completion_microsec, multipath_test_backup);
}
```

### Current Rust test body
```rust
fn multipath_backup() {
    multipath_test_one(2_000_000, MultipathTestId::Backup);
}
```

## `picoquictest/multipath_test.c:multipath_callback_test`
* C test-table name: `multipath_callback`
* C entry function: `multipath_callback_test`
* Rust test: `multipath_callback`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1590-1592`
* Phase 5A analysis: The mapped C body is the Win32 no-op branch returning 0, while the Rust test runs the non-Win callback scenario and log comparison.
* Phase 5A fix note: Fix the mapping/source span to the active non-Win C branch for the v1 target, or gate/change the Rust test to no-op if the Win32 branch is truly intended.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: For the Linux target, the Rust wrapper matches the active C branch and callback log path, but the shared util test module is not currently compilable in the merged tree, so this test is not exposed as runnable.
* Phase 5C fix note: Repair shared util.rs compile damage. No callback-specific mismatch found.

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Current Rust test body
```rust
fn multipath_callback() {
    multipath_test_one(1_000_000, MultipathTestId::Callback);
}
```

## `picoquictest/multipath_test.c:multipath_datagram_test`
* C test-table name: `multipath_datagram`
* C entry function: `multipath_datagram_test`
* Rust test: `multipath_datagram`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1596-1598`
* Phase 5A analysis: The entry constants match, but Rust helper behavior is weaker: common scenario verification ignores max_completion_microsec, and datagram receive/path counters are updated when queueing datagrams rather than by real receive callbacks as in C.
* Phase 5A fix note: Make multipath datagram helpers use real datagram send/receive/ack callbacks and real path accounting, and make scenario verification enforce completion and the 1,150,000 us bound.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust entry uses the right test id and 1150000 us bound, but the datagram helper uses queued datagrams and increments receive/path counters at queue time instead of exercising the C prepare/receive/ack callback harness and counting real CallbackEvent::Datagram delivery.
* Phase 5C fix note: Wire TestDatagramCtx through the real Rust prepare_datagram/Datagram/ack callback surface, use provide_datagram_buffer_ex for sends, and update dg_recv/path counters on actual receive callbacks rather than when queueing.

### C test body
```c
{
    /* TODO: investigate why 1.15 instead of 1.12 with prior implementation of multipath */
    uint64_t max_completion_microsec = 1150000;

    return multipath_test_one(max_completion_microsec, multipath_test_datagram);
}
```

### Current Rust test body
```rust
fn multipath_datagram() {
    multipath_test_one(1_150_000, MultipathTestId::Datagram);
}
```
