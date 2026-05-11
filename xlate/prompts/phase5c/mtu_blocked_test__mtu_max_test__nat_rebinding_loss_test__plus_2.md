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

## `picoquictest/tls_api_test.c:mtu_blocked_test`
* C test-table name: `mtu_blocked`
* C entry function: `mtu_blocked_test`
* Rust test: `mtu_blocked`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4327-4335`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust checks materially different behavior. It passes policy value 1, which the Rust helper maps to Required, while C uses blocked policy value 3. The helper also uses a different stream scenario and never asserts the expected client/server MTUs.
* Phase 5A fix note: Use the blocked PMTUD policy, match test_scenario_mtu_discovery, set both client/server PMTUD settings like C, and assert client/server send_mtu equal 1252.
* Phase 5B analysis: Rust #[test] is present and runnable, uses PmtudPolicy::Blocked, the same one-stream MTU scenario, and the same 1252/1252 API-visible MTU assertions through the translated helper. The observed 1200 runtime MTU mismatch is a Phase 5C implementation note, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_blocked, 1252, 1252, 
        test_scenario_mtu_discovery, sizeof(test_scenario_mtu_discovery), 0);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_blocked() {
    let scenario = [TestApiStreamDesc {
        stream_id: 2,
        previous_stream_id: 0,
        q_len: 100_000,
        r_len: 0,
    }];
    mtu_discovery_test_one(PmtudPolicy::Blocked, 1252, 1252, &scenario, 0).expect("mtu_blocked");
}
```

## `picoquictest/tls_api_test.c:mtu_max_test`
* C test-table name: `mtu_max`
* C entry function: `mtu_max_test`
* Rust test: `mtu_max`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4410-4418`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls an MTU helper with similar visible values, but the helper does not preserve the C test: it ignores expected client/server MTUs, sets mtu_max on qclient instead of qserver, uses a different fixed scenario, and never asserts send_mtu == 1420/1392.
* Phase 5A fix note: Make the Rust MTU helper use the C scenario, set client/server PMTUD and server mtu_max as in C, and assert both final send MTUs exactly.
* Phase 5B analysis: Rust mtu_max and shared helper match the C API-level setup and assertions; the known client send_mtu 1200 vs expected 1420 runtime failure is Phase 5C implementation behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_basic, 1420, 1392,
        test_scenario_mtu_discovery, sizeof(test_scenario_mtu_discovery), 1420);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_max() {
    let scenario = [TestApiStreamDesc {
        stream_id: 2,
        previous_stream_id: 0,
        q_len: 100_000,
        r_len: 0,
    }];
    mtu_discovery_test_one(PmtudPolicy::Basic, 1420, 1392, &scenario, 1420).expect("mtu_max");
}
```

## `picoquictest/tls_api_test.c:nat_rebinding_loss_test`
* C test-table name: `nat_rebinding_loss`
* C entry function: `nat_rebinding_loss_test`
* Rust test: `nat_rebinding_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4864-4866`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the same top-level arguments, but the helper does not implement the C NAT rebinding behavior: it applies the loss mask during the handshake, never switches to a NAT address, ignores the zero-CID path, and omits path challenge/remote CID verification.
* Phase 5A fix note: Implement Rust nat_rebinding_test_one to match C: handshake with zero loss, switch client address/port for NAT rebinding, run data transfer with loss_mask_data, and verify scenario completion, challenge renewal/verification, path count, and remote CID expectations.
* Phase 5B analysis: Rust #[test] nat_rebinding_loss is present, harness-runnable, and calls nat_rebinding_test_one(0x2012, false, 0), matching the C API-level contract. Prior NAT rebinding runtime/library gaps are Phase 5C notes, not Phase 5B blockers.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t loss_mask = 0x2012;

    return nat_rebinding_test_one(loss_mask, 0, 0);
}
```

### Current Rust test body
```rust
fn nat_rebinding_loss() {
    nat_rebinding_test_one(0x2012, false, 0).expect("nat_rebinding_loss");
}
```

## `picoquictest/tls_api_test.c:pacing_update_test`
* C test-table name: `pacing_update`
* C entry function: `pacing_update_test`
* Rust test: `pacing_update`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5457-5459`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic handshake/close path; it does not subscribe to pacing updates, log callback rows, run the 1 MB scenario, or compare pacing_rate.csv to the reference.
* Phase 5A fix note: Implement the pacing-update scenario: create the CSV/header or equivalent capture, call subscribe_pacing_rate_updates(0x8000, 0x10000), run test_scenario_q_and_r with stream0_target 1,000,000 and 3,600,000 us limit, then compare against picoquictest/pacing_rate_ref.txt or an equivalent deterministic expected record set.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, and expresses the C API-level contract: pacing callback capture, same pacing subscription thresholds, q-and-r scenario, 1 MB stream0 target, timing limits, CSV generation, and reference comparison. Missing callback dispatch/pacing rows or runtime mismatch are Phase 5C library-behavior notes, not Phase 5B blockers.
* Phase 5B fix note: 

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
        test_ctx->bw_update = picoquic_file_open(PACING_RATE_CSV, "w");
        if (test_ctx->bw_update == NULL) {
            DBG_PRINTF("Could not write file <%s>", PACING_RATE_CSV);
            ret = -1;
        }
        else {
            fprintf(test_ctx->bw_update, "Time, Pacing_rate_CB, Pacing_rate, CWIN, RTT\n");
            /* Request bandwidth updates */
            picoquic_subscribe_pacing_rate_updates(test_ctx->cnx_client, 0x8000, 0x10000);

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
        char pacing_rate_ref[512];

        ret = picoquic_get_input_path(pacing_rate_ref, sizeof(pacing_rate_ref), picoquic_solution_dir, PACING_RATE_TEST_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the pacing rate test ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(PACING_RATE_CSV, pacing_rate_ref);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn pacing_update() {
    pacing_update_impl().expect("pacing_update");
}
```

## `picoquictest/tls_api_test.c:preferred_address_test`
* C test-table name: `preferred_address`
* C entry function: `preferred_address_test`
* Rust test: `preferred_address`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6362-6364`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust sets a preferred IPv4 address and runs a scenario, but it does not assert the C test's required outcomes: client/server address promotion, migrated CID sequences, and migration-disabled flags cleared/authorized.
* Phase 5A fix note: After the scenario, verify the preferred address is installed on client and server paths, verify new CID sequence use when cid_zero is false, and verify migration is not blocked for this false,false case.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, and calls preferred_address_test_one(false, false), matching the C entry preferred_address_test_one(0, 0). The helper expresses the C API-level setup and postconditions; early Generic/runtime failure is a Phase 5C library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return preferred_address_test_one(0, 0);
}
```

### Current Rust test body
```rust
fn preferred_address() {
    preferred_address_test_one(false, false).expect("preferred_address");
}
```
