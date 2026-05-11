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

## `picoquictest/tls_api_test.c:mtu_discovery_test`
* C test-table name: `mtu_discovery`
* C entry function: `mtu_discovery_test`
* Rust test: `mtu_discovery`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4356-4364`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust helper ignores the expected client/server MTU arguments, uses a different scenario, sets only the client PMTUD policy, and closes without checking send_mtu. C uses test_scenario_mtu_discovery and fails unless both path send_mtu values equal 1440.
* Phase 5A fix note: Update Rust mtu_discovery_test_one to accept/use the scenario equivalent to C, set both server default and client PMTUD policy, and assert client/server path send_mtu equals the expected values after the data loop.
* Phase 5B analysis: Rust #[test] mtu_discovery is present in the harness and matches the C entry: Basic PMTUD policy, expected client/server MTU 1440/1440, the {2,0,100000,0} stream scenario, and mtu_max 0. The helper mirrors the C API-visible setup and send_mtu assertions; known PMTUD/send_mtu runtime failures are Phase 5C implementation work, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_basic, 1440, 1440, 
        test_scenario_mtu_discovery, sizeof(test_scenario_mtu_discovery), 0);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_discovery() {
    let scenario = [TestApiStreamDesc {
        stream_id: 2,
        previous_stream_id: 0,
        q_len: 100_000,
        r_len: 0,
    }];
    mtu_discovery_test_one(PmtudPolicy::Basic, 1440, 1440, &scenario, 0).expect("mtu_discovery");
}
```

## `picoquictest/tls_api_test.c:multi_segment_test`
* C test-table name: `multi_segment`
* C entry function: `multi_segment_test`
* Rust test: `multi_segment`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4542-4553`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust runs one default context with an empty scenario and a loose completion target. The C test loops over five congestion algorithms, uses send_buffer_size 65536, configures latency/bandwidth/qlog/long-log, runs sustained2, and verifies each algorithm-specific completion target.
* Phase 5A fix note: Add a faithful multi_segment_test_one and have the test loop over newreno, cubic, dcubic, fastcc, and bbr with the C target times, initial CID encoding, 65536 send buffer, link settings, sustained2 scenario, and completion verification.
* Phase 5B analysis: Rust test is a compiling #[test] and matches the C API-level contract: five CC algorithms, target times, 65_536 send buffer/GSO path, initial CID CC byte, 35ms/100Mbps links, qlog/long-log setup, server CC verification, sustained2 stream scenario, data loop, and completion-time verification. Any large-buffer prepare_packet_ex/GSO handshake failure is Phase 5C runtime behavior, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    picoquic_congestion_algorithm_t* algo_list[5] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr_algorithm
    };
    uint64_t algo_time[5] = {
        1220000,
        1050000,
        1250000,
        1350000,
        1280000
    };
    int ret = 0;

    for (int i = 0; i < 5 && ret == 0; i++) {
        ret = multi_segment_test_one(algo_list[i], algo_time[i], 65536);
        if (ret != 0) {
            DBG_PRINTF("Multi segment test fails for CC=%s", algo_list[i]->congestion_algorithm_id);
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn multi_segment() {
    for (algo_id, target_time) in [
        ("newreno", 1_220_000),
        ("cubic", 1_050_000),
        ("dcubic", 1_250_000),
        ("fastcc", 1_350_000),
        ("bbr", 1_280_000),
    ] {
        multi_segment_test_one(algo_id, target_time, 65_536)
            .unwrap_or_else(|e| panic!("multi_segment({algo_id}): {e:?}"));
    }
}
```

## `picoquictest/tls_api_test.c:nat_rebinding_zero_test`
* C test-table name: `nat_rebinding_zero`
* C entry function: `nat_rebinding_zero_test`
* Rust test: `nat_rebinding_zero`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5027-5029`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes cid_zero=true but the helper ignores it, initializes a normal context, never switches client_use_nat/client_addr_natted, and only runs a generic data transfer/close. It misses the zero-CID setup and NAT rebinding challenge renewal/verification checks.
* Phase 5A fix note: Make nat_rebinding_test_one honor zero-CID initialization, perform the NAT port rebinding, run the q_and_r transfer, verify stream completion, and assert the server challenge was renewed and verified.
* Phase 5B analysis: Reclassified ok: the Rust #[test] is present, compiles, and calls nat_rebinding_test_one(0, true, 0), matching the C wrapper's loss_mask=0, zero_cid=1, latency=0. The helper expresses the NAT rebinding API-level contract: zero-CID setup, NAT port rebinding, q-and-r transfer, stream completion verification, and server challenge renewal/verification. The known stream callback/data-delivery failure is a Phase 5C runtime implementation issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    /* Test of NAT rebinding with zero-length client CID */
    uint64_t loss_mask = 0;

    return nat_rebinding_test_one(loss_mask, 1, 0);
}
```

### Current Rust test body
```rust
fn nat_rebinding_zero() {
    nat_rebinding_test_one(0, true, 0).expect("nat_rebinding_zero");
}
```

## `picoquictest/tls_api_test.c:padding_null_test`
* C test-table name: `padding_null`
* C entry function: `padding_null_test`
* Rust test: `padding_null`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5886-5888`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls the same-named helper with 0,0, but that helper ignores both padding arguments and only handshakes/closes. C sets padding policy and sends 15 packet-size probes, verifying the no-padding length rules for each.
* Phase 5A fix note: Implement padding_test_one logic for Rust: set server default and client padding policy, queue the padding/ping frames for the C test sizes, observe outgoing packet lengths, and assert the no-padding expectations for padding_null.
* Phase 5B analysis: Rust #[test] padding_null is present, compiles under the Rust test harness, and its helper expresses the C padding_test_one(0, 0) API contract: zero padding policy, the same 15 padding+ping probe sizes, client-to-server packet length observation, and the C no-padding length assertions. Any null-initial-CID handshake failure is a Phase 5C runtime/library issue, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    return padding_test_one(0, 0);
}
```

### Current Rust test body
```rust
fn padding_null() {
    padding_null_test_one().expect("padding_null");
}
```

## `picoquictest/tls_api_test.c:preferred_address_zero_test`
* C test-table name: `preferred_address_zero`
* C entry function: `preferred_address_zero_test`
* Rust test: `preferred_address_zero`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6379-6381`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust uses the right boolean arguments, but the helper ignores cid_zero and only runs a scenario. It does not verify preferred-address migration, client/server addresses, CID update rules, or migration-disabled flags as C does.
* Phase 5A fix note: Extend preferred_address_test_one to check preferred address promotion, server CID update, migration authorization, and the cid_zero-specific skipped client CID assertion.
* Phase 5B analysis: Rust #[test] calls preferred_address_test_one(false, true), matching the C wrapper preferred_address_test_one(0, 1). The helper expresses the same API-visible preferred-address, CID, and migration-flag assertions, skipping only the cid_zero client-CID assertion. Any runtime failure from incomplete preferred-address migration/path-probe behavior is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* test with zero length client cid */
    return preferred_address_test_one(0, 1);
}
```

### Current Rust test body
```rust
fn preferred_address_zero() {
    preferred_address_test_one(false, true).expect("preferred_address_zero");
}
```
