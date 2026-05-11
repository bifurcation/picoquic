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

## `picoquictest/tls_api_test.c:zero_rtt_bad_param_test`
* C test-table name: `zero_rtt_bad_param`
* C entry function: `zero_rtt_bad_param_test`
* Rust test: `zero_rtt_bad_param`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:8941-8947`
* Phase 5A analysis: The Rust wrapper passes change_params, but zero_rtt_test_one synthesizes 0-RTT counters instead of observing real behavior and omits the C server-data-received check.
* Phase 5A fix note: Make zero_rtt_test_one drive real 0-RTT state for changed transport parameters; assert 0-RTT was sent, not acked/accepted, data was eventually received, no short Initial, and tickets were saved.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust entry sets change_params, but the current helper repair for the server-data-received check is not exposed as a valid TestTlsApiCtx method, so zero_rtt_test_one calls a broken harness surface.
* Phase 5C fix note: Restore server_received_stream_data as a valid TestTlsApiCtx helper/method so zero_rtt_test_one compiles and enforces sent, not-acked, data-received, no-short-initial, and ticket checks.

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.change_params = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust

/// C: `vn_compat_test` in `picoquictest/tls_api_test.c`.
///
/// Starts with QUIC v1, requests compatible upgrades to v2 and v2 draft, and
/// verifies that an incompatible InternalTest1 target is rejected.
#[test]
fn vn_compat() {
```

## `picoquictest/tls_api_test.c:zero_rtt_ech_test`
* C test-table name: `zero_rtt_ech`
* C entry function: `zero_rtt_ech_test`
* Rust test: `zero_rtt_ech`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:8982-8988`
* Phase 5A analysis: C sets propose_ech and configures client ECH before verifying 0-RTT/PSK behavior; Rust sets the flag but the helper only toggles client_zero_share and synthesizes 0-RTT/PSK state, so ECH proposal behavior is not actually checked.
* Phase 5A fix note: Implement/use the Rust equivalent of picoquic_ech_configure_quic_ctx for propose_ech and verify real 0-RTT plus ECH behavior without forcing counters/state.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test is present, but zero_rtt_test_one builds a start-immediate context, then configures qclient ECH and calls start_client again. C uses delayed init, configures ECH before picoquic_start_client_cnx, then runs the 0-RTT path.
* Phase 5C fix note: Use a delayed/not-yet-started zero-RTT context, configure qclient.ech_configure(None, None) before start_client, and keep the existing ticket/PSK/0-RTT assertions.

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.propose_ech = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
        .first_cnx_mut()
        .and_then(connection_supported_version)
        .ok_or(crate::Error::Generic)?;

    if client_version != target || server_version != target {
        return Err(crate::Error::Generic);
    }
```

## `picoquictest/tls_api_test.c:zero_rtt_no_coal_test`
* C test-table name: `zero_rtt_no_coal`
* C entry function: `zero_rtt_no_coal_test`
* Rust test: `zero_rtt_no_coal`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:9044-9050`
* Phase 5A analysis: Rust sets no_coal=true, but the shared helper omits the C no_coal-specific assertion that server zero-RTT received count equals client zero-RTT sent count, and it manually seeds zero-RTT counters before the loop.
* Phase 5A fix note: Make zero_rtt_test_one observe real zero-RTT send/ack/receive counters and add the no_coal received-count assertion equivalent to the C helper.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test and helper faithfully set no_coal and check real zero-RTT sent, acked, and server-received counters, but merged tls_api.rs duplicate constants block clean test exposure.
* Phase 5C fix note: Remove/merge duplicate top-level scenario constants in rs/fq/src/tests/tls_api.rs; keep the current zero_rtt_no_coal helper path.

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.no_coal = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
        extra_delay: NOMINAL_DELAY - 2_000_000,
        ..Default::default()
    })
    .expect("zero_rtt_delay");
}

/// C: `zero_rtt_ech_test` in `picoquictest/tls_api_test.c`.
```

## `picoquictest/tls_api_test.c:zero_rtt_retry_test`
* C test-table name: `zero_rtt_retry`
* C entry function: `zero_rtt_retry_test`
* Rust test: `zero_rtt_retry`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:9057-9063`
* Phase 5A analysis: The hardreset flag is mapped, but the shared Rust helper still pre-sets 0-RTT counters and skips important C retry-path checks such as real data receipt under rejection/retry behavior.
* Phase 5A fix note: After fixing zero_rtt_test_one, ensure the hardreset path actually exercises server retry/cookie mode and checks the C rejection/data-receipt conditions without synthetic counters.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust entry passes hardreset=true, but zero_rtt_test_one creates an already-started context and then calls start_client again; C uses delayed init, configures retry cookie mode, then starts once.
* Phase 5C fix note: Use a non-starting ticket-key init so pass setup precedes one start_client call, preserving retry, data-receipt, and short-initial checks.

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.hardreset = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
        propose_ech: true,
        ..Default::default()
    })
    .expect("zero_rtt_ech");
}

/// C: `zero_rtt_long_test` in `picoquictest/tls_api_test.c`.
```
