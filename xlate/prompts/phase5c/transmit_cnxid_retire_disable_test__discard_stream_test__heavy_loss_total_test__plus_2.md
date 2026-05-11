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

## `picoquictest/tls_api_test.c:transmit_cnxid_retire_disable_test`
* C test-table name: `cnxid_transmit_r_disable`
* C entry function: `transmit_cnxid_retire_disable_test`
* Rust test: `cnxid_transmit_r_disable`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:1689-1691`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes the same booleans, but transmit_cnxid_test_one ignores retire_before/disable/early and only runs a generic connect/sync/close path, missing the C CID TTL, migration-disabled setup, stash, count, and retire-before checks.
* Phase 5A fix note: Implement transmit_cnxid_test_one to honor retire_before=true and disable_migration=true, drive the C-equivalent loops, and verify CID counts/stashes and retired sequence filtering.
* Phase 5B analysis: Rust #[test] maps the C entry's transmit_cnxid_test_one(1, 1, 0) to transmit_cnxid_test_one(true, true, false), and the shared helper expresses the API-visible retire-before, migration-disabled, CID-count, stash, and retired-sequence checks. Early runtime disconnect/server removal is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return transmit_cnxid_test_one(1, 1, 0);
}
```

### Current Rust test body
```rust
fn cnxid_transmit_r_disable() {
    transmit_cnxid_test_one(true, true, false).expect("cnxid_transmit_r_disable");
}
```

## `picoquictest/tls_api_test.c:discard_stream_test`
* C test-table name: `discard_stream`
* C entry function: `discard_stream_test`
* Rust test: `discard_stream`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2043-2045`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-00`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper passes the same flags, but the Rust stop_sending_test_one helper is materially weaker: wrong scenario shape, no latency/loss setup, no wait for partial first response, and no C-equivalent completion/leak assertions.
* Phase 5A fix note: Repair stop_sending_test_one for the discard case to use the two-stream stop-sending scenario, C loss/latency behavior, discard/reset notification semantics, and final partial/complete stream plus data-node checks.
* Phase 5B analysis: Rust wrapper matches C entry flags, and the shared helper already expresses the C API-level discard-stream contract. Any early failure is from incomplete stream/reset behavior and belongs to Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = stop_sending_test_one(1, 0);
    return ret;
}
```

### Current Rust test body
```rust
fn discard_stream() {
    stop_sending_test_one(true, false).expect("discard_stream");
}
```

## `picoquictest/tls_api_test.c:heavy_loss_total_test`
* C test-table name: `heavy_loss_total`
* C entry function: `heavy_loss_total_test`
* Rust test: `heavy_loss_total`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2832-2834`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes mode 2 and the target time, but the helper ignores the mode and does not simulate the C total-loss phase, BBR setup, initial CID, sustained multi-stream scenario, ramp period, or recovery-after-loss flow.
* Phase 5A fix note: Repair heavy_loss_test_one so mode 2 uses the C sustained scenario, BBR/log setup, 0.1s ramp, UINT64_MAX loss for the timed loss window, then clears loss and verifies completion within 25s.
* Phase 5B analysis: Rust #[test] is present and calls heavy_loss_test_one(2, 25_000_000), matching the C wrapper's heavy_loss_test_one(2, 25000000). Shared helper review shows the same API-level total-loss scenario and completion-target contract; any early Generic/runtime failure is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    return heavy_loss_test_one(2, 25000000);
}
```

### Current Rust test body
```rust
fn heavy_loss_total() {
    heavy_loss_test_one(2, 25_000_000).expect("heavy_loss_total");
}
```

## `picoquictest/tls_api_test.c:key_rotation_test`
* C test-table name: `key_rotation`
* C entry function: `key_rotation_test`
* Rust test: `key_rotation`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:3371-3375`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C runs key_rotation_test_one three times: no injected packet, bad packet injected toward client, and bad packet injected toward server. Rust runs only key_rotation_test_one(false), and the Rust helper itself only starts one client rotation and ignores bad-packet injection semantics.
* Phase 5A fix note: Update the Rust test/helper to cover all three C cases, including false-rotation packet injection on both directions, and preserve the C expectation of three rotations: client, server, and simultaneous rotation before close.
* Phase 5B analysis: Reclassified ok: current Rust #[test] runs key_rotation_test_one for C cases 0, 2, and 1, and the helper expresses the C API-level sustained-transfer, false-packet injection, client/server/simultaneous key-rotation, nb_rotation >= 3, and close contract. Connection::start_key_rotation returning Error::Generic is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = key_rotation_test_one(0);

    if (ret == 0) {
        /* test rotation with injection of bad packets on client */
        ret = key_rotation_test_one(2);
        if (ret != 0) {
            DBG_PRINTF("%s", "Packet injection on client defeats rotation.\n", ret);
        }
    }

    if (ret == 0) {
        /* test rotation with injection of bad packets on server */
        ret = key_rotation_test_one(1);
        if (ret != 0) {
            DBG_PRINTF("%s", "Packet injection on server defeats rotation.\n", ret);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn key_rotation() {
    key_rotation_test_one(0).expect("key_rotation");
    key_rotation_test_one(2).expect("key_rotation injection toward client");
    key_rotation_test_one(1).expect("key_rotation injection toward server");
}
```

## `picoquictest/tls_api_test.c:many_short_loss_test`
* C test-table name: `many_short_loss`
* C entry function: `many_short_loss_test`
* Rust test: `many_short_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4221-4223`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls tls_api_loss_test, but that helper ignores its loss_mask and only performs a generic connection/close. C runs test_scenario_more_streams with 18 streams, loss mask 0x882818A881288848, max_data=16000, and queue_delay_max=2000.
* Phase 5A fix note: Implement the many_short_loss Rust test with the more_streams scenario and preserve the C loss mask, max_data, and queue-delay behavior; fix/reuse helpers so the loss mask is actually applied.
* Phase 5B analysis: Rust many_short_loss is present, compiles as a harness test, and matches the C API-level scenario: more_streams fixture, loss mask 0x882818A881288848, max_data=16000, queue_delay_max=2000, version 0, and no completion deadline. The observed missing server connection under queue_delay_max is a Phase 5C implementation failure, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_more_streams, sizeof(test_scenario_more_streams), 0, 0x882818A881288848ull, 16000, 2000, 0, 0, NULL, NULL);
}
```

### Current Rust test body
```rust
fn many_short_loss() {
    many_short_loss_scenario().expect("many_short_loss");
}
```
