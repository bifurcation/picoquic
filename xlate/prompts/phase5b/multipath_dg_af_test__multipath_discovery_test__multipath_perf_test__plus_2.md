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

## `picoquictest/multipath_test.c:multipath_dg_af_test`
* C test-table name: `multipath_dg_af`
* C entry function: `multipath_dg_af_test`
* Rust test: `multipath_dg_af`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1602-1604`
* Phase 5A analysis: The Rust wrapper uses the correct timeout and DgAf enum, and datagram-affinity checks mostly mirror C, but the shared scenario verifier drops C's stream-completion and completion-time assertions.
* Phase 5A fix note: Repair shared scenario verification and path-readiness checks so the datagram-affinity test also proves the underlying transfer completed within 1,100,000 us.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Wrapper arguments match C, but datagram-affinity verification is weaker than C: Rust queues datagrams directly and pre-increments receive/path counters from intended path instead of using datagram send/receive callbacks and actual unique_path_id.
* Phase 5C fix note: Use a C-like datagram provider/receive harness for DgAf and record actual received path IDs before asserting all received datagrams stayed on path 0.

### C test body
```c
{
    uint64_t max_completion_microsec = 1100000;

    return multipath_test_one(max_completion_microsec, multipath_test_dg_af);
}
```

### Current Rust test body
```rust

    let aead_encrypt = setup_test_aead_context(true, &MP_AEAD_SECRET, LABEL_QUIC_V1_KEY_BASE)
        .expect("aead_encrypt context");
```

## `picoquictest/multipath_test.c:multipath_discovery_test`
* C test-table name: `multipath_discovery`
* C entry function: `multipath_discovery_test`
* Rust test: `multipath_discovery`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1608-1610`
* Phase 5A analysis: Discovery setup and address-observation assertions are present, but the shared final verifier still skips C's stream completion and 2.0s deadline checks.
* Phase 5A fix note: Fix the shared Rust scenario verifier; keep the existing discovery-specific nb_address_observed and local-vs-observed address checks.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust discovery test uses the right max-completion bound and preserves final scenario verification plus observed-address assertions, but the merged shared Rust test harness is currently malformed and not runnable.
* Phase 5C fix note: Repair rs/fq/src/tests/util.rs shared harness scope/duplicate-field regression; no pair-specific C/Rust intent mismatch found.

### C test body
```c
{
    uint64_t max_completion_microsec = 2000000;

    return multipath_test_one(max_completion_microsec, multipath_test_discovery);
}
```

### Current Rust test body
```rust
    let path_id_test: &[u64] = &[0, 1, 2, 0x0123456789abcdef];
    let sequence = 12345u64;
    let aad = b"This is a test";
```

## `picoquictest/multipath_test.c:multipath_perf_test`
* C test-table name: `multipath_perf`
* C entry function: `multipath_perf_test`
* Rust test: `multipath_perf`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1656-1658`
* Phase 5A analysis: Rust passes Perf and sets perf links/BBR/long scenario, but it drops C's send_buffer_size=65536 setup and the 1.65s completion limit is ignored by the shared verifier.
* Phase 5A fix note: Preserve the perf send-buffer setup or an equivalent Rust harness behavior, and fix shared scenario completion/time verification.
* Phase 5C outcome: blocked
* Phase 5C analysis: Rust wrapper/helper match the C Perf case, including send buffer intent, perf links, BBR, long scenario, wait_multipath_ready, and max-completion verification, but set_send_buffer_size is not actually exposed due the shared util.rs mismerge.
* Phase 5C fix note: Move/restore TestTlsApiCtx::set_send_buffer_size into the impl and remove duplicated harness fields.

### C test body
```c
{
    uint64_t max_completion_microsec = 1650000;

    return  multipath_test_one(max_completion_microsec, multipath_test_perf);
}
```

### Current Rust test body
```rust
                    enc_path
                );
            }
```

## `picoquictest/multipath_test.c:multipath_quality_test`
* C test-table name: `multipath_quality`
* C entry function: `multipath_quality_test`
* Rust test: `multipath_quality`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1676-1676`
* Phase 5A analysis: The no-op C body shown is Win32-only; the in-scope non-Win32 C branch runs Quality with 1000000 us and callback-log comparison, which Rust maps. It still inherits the weak Rust scenario verifier that omits C completion and timing checks.
* Phase 5A fix note: Keep the non-Win32 Quality scenario, but repair the shared Rust verifier to check scenario completion and max_completion_microsec.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The final Rust test is an empty body, matching only the Win32 no-op branch; the in-scope x86_64 non-Windows C test runs the Quality scenario with 1000000 us and callback-log verification.
* Phase 5C fix note: Restore the Quality scenario invocation with max_completion_microsec 1000000 and the shared completion/callback verification.

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Current Rust test body
```rust
#[test]
```

## `picoquictest/multipath_test.c:multipath_socket_error_test`
* C test-table name: `multipath_socket_error`
* C entry function: `multipath_socket_error_test`
* Rust test: `multipath_socket_error`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Rust span: `rs/fq/src/tests/multipath.rs:1704-1706`
* Phase 5A analysis: The Rust wrapper maps to Break2 with the correct 11,000,000 us timeout and does assert one remaining path after the socket error, but the shared verifier omits C's stream completion and timeout checks.
* Phase 5A fix note: Repair shared scenario verification/path readiness so the socket-error case checks both recovery/removal and successful transfer within the C timeout.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust Break2 wrapper and shared multipath verifier match the C socket-error transfer, timeout, and one-path recovery checks, but shared util.rs compile damage prevents runnable exposure.
* Phase 5C fix note: Repair shared util.rs compile damage. No socket-error-specific mismatch found.

### C test body
```c
{
    uint64_t max_completion_microsec = 11000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_break2);
}
```

### Current Rust test body
```rust

/// C: `multipath_datagram_test`.
#[test]
```
