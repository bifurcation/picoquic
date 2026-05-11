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

## `picoquictest/tls_api_test.c:migration_zero_test`
* C test-table name: `migration_zero`
* C entry function: `migration_zero_test`
* Rust test: `migration_zero`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4312-4320`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes an empty scenario to a helper whose default scenario differs from C, ignores cid_zero, and omits the C checks for challenge renewal/verification and expected connection-ID changes after migration.
* Phase 5A fix note: Make migration_zero use the very-long scenario with cid_zero=true, honor cid_zero in context creation, and add the C migration assertions for data completion, challenge verification, target remote CID, and local CID behavior.
* Phase 5B analysis: Rust migration_zero is a #[test], compiles under the Rust test harness, and expresses the C API-level contract: very-long single-stream scenario, zero loss, and cid_zero=true. probe_new_path returning Error::Generic is a Phase 5C runtime-library issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return migration_test_scenario(test_scenario_very_long, sizeof(test_scenario_q_and_r), 0, 1);
}
```

### Current Rust test body
```rust
fn migration_zero() {
    let scenario = [TestApiStreamDesc {
        stream_id: 4,
        previous_stream_id: 0,
        q_len: 257,
        r_len: 1_000_000,
    }];
    migration_test_scenario(&scenario, 0, true).expect("migration_zero");
}
```

## `picoquictest/tls_api_test.c:mtu_drop_newreno_test`
* C test-table name: `mtu_drop_newreno`
* C entry function: `mtu_drop_newreno_test`
* Rust test: `mtu_drop_newreno`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4402-4404`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Although the Rust test calls a similarly named helper with newreno and 11600000, that helper ignores the algorithm, does not simulate 100 ms/1 Mbps links, does not wait for MTU discovery, does not drop path MTU, and the completion target is not enforced.
* Phase 5A fix note: Implement mtu_drop_cc_algotest faithfully: set the selected CC algorithm/newreno, use the C initial CID behavior, configure 100 ms and 1 Mbps links, run connection with the queue delay, use the very-long scenario, wait 1 second, assert discovered MTUs, halve path MTUs, finish transfer, and enforce the 11600000 us target.
* Phase 5B analysis: Rust #[test] is present and calls the MTU-drop helper with newreno and 11_600_000, matching the C wrapper's NewReno API-level contract. Any early handshake timeout is a Phase 5C runtime-library issue, not a Phase 5B block.
* Phase 5B fix note: No Rust test changes; command log updated only.

### C test body
```c
{
    int ret = mtu_drop_cc_algotest(picoquic_newreno_algorithm, 11600000);
    return ret;
}
```

### Current Rust test body
```rust
fn mtu_drop_newreno() {
    mtu_drop_cc_algotest("newreno", 11_600_000).expect("mtu_drop_newreno");
}
```

## `picoquictest/tls_api_test.c:nat_rebinding_latency_test`
* C test-table name: `nat_rebinding_latency`
* C entry function: `nat_rebinding_latency_test`
* Rust test: `nat_rebinding_latency`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:4856-4858`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The wrapper arguments match, but the Rust helper does not perform NAT rebinding or verify challenge renewal/validation; it only runs a normal transfer with latency.
* Phase 5A fix note: Implement the NAT address switch, q_and_r transfer, completion check, post-rebinding wait, and server challenge renewal/verified assertions for latency=100000.
* Phase 5B analysis: Rust #[test] is present, compiles, is runnable by the Rust harness, and calls the NAT rebinding helper with the C-equivalent arguments: loss_mask=0, zero_cid=false, latency=100000. Any earlier failure to create the server connection is a Phase 5C implementation/runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    /* Test of NAT rebinding with zero-length client CID */
    uint64_t loss_mask = 0;

    return nat_rebinding_test_one(loss_mask, 0, 100000);
}
```

### Current Rust test body
```rust
fn nat_rebinding_latency() {
    nat_rebinding_test_one(0, false, 100_000).expect("nat_rebinding_latency");
}
```

## `picoquictest/tls_api_test.c:optimistic_hole_test`
* C test-table name: `optimistic_hole`
* C entry function: `optimistic_hole_test`
* Rust test: `optimistic_hole`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:5448-5450`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-02`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a generic transfer and ignores the optimistic ACK hole policy behavior that the C test validates, including hole insertion, trap packets, no server retransmissions, and full transfer verification.
* Phase 5A fix note: Implement optimistic_ack_test_one with the C setup: default hole period, deterministic seed/CID handling, very-long scenario, custom send loop inspecting ACK traps and hole counters, and final scenario verification for the non-spoof case.
* Phase 5B analysis: Rust test is present, compiles under the Rust test harness, and calls optimistic_ack_test_one(false), matching the C entry's optimistic_ack_test_one(0). Any missing ACK-trap insertion or hole counter behavior is a Phase 5C library/runtime issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = optimistic_ack_test_one(0);

    return ret;
}
```

### Current Rust test body
```rust
fn optimistic_hole() {
    optimistic_ack_test_one(false).expect("optimistic_hole");
}
```

## `picoquictest/tls_api_test.c:port_blocked_test`
* C test-table name: `port_blocked`
* C entry function: `port_blocked_test`
* Rust test: `port_blocked`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6336-6355`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only runs a normal TLS connection. The C test iterates blocked and unblocked port sets across IPv4/IPv6, with port-blocking enabled and disabled, and checks server packet behavior.
* Phase 5A fix note: Replace the placeholder handshake with a Rust port-blocking test covering ports 0,53,138,1900,5353,11211 as blocked and 443,4433,33721 as unblocked, including disable-port-blocking behavior and IPv4/IPv6 cases.
* Phase 5B analysis: Rust test is present as #[test], compiles under the Rust test harness, and already expresses the C API-level contract: blocked/unblocked port arrays plus IPv4/IPv6, port-blocking enabled/disabled, Version Negotiation, stateless, and Initial packet cases. The missing VN response when blocking is disabled for source port 0 is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;
    const uint16_t blocked_port_to_test[] = { 0, 53, 138, 1900, 5353, 11211 };
    const uint16_t unblocked_port_to_test[] = { 443, 4433, 33721 };
    size_t nb_blocked = sizeof(blocked_port_to_test) / sizeof(uint16_t);
    size_t nb_unblocked = sizeof(unblocked_port_to_test) / sizeof(uint16_t);

    for (size_t i = 0; ret == 0 && i < nb_blocked; i++) {
        ret = port_blocked_test_port(blocked_port_to_test[i], 1);
    }

    for (size_t i = 0; ret == 0 && i < nb_unblocked; i++) {
        ret = port_blocked_test_port(unblocked_port_to_test[i], 0);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn port_blocked() {
    const BLOCKED_PORTS_TO_TEST: [u16; 6] = [0, 53, 138, 1900, 5353, 11211];
    const UNBLOCKED_PORTS_TO_TEST: [u16; 3] = [443, 4433, 33721];

    for port in BLOCKED_PORTS_TO_TEST {
        assert!(
            crate::check_port_blocked(port),
            "test port {port} should be in the blocked set"
        );
        port_blocked_test_port(port, true).expect("blocked port");
    }

    for port in UNBLOCKED_PORTS_TO_TEST {
        assert!(
            !crate::check_port_blocked(port),
            "test port {port} should not be in the blocked set"
        );
        port_blocked_test_port(port, false).expect("unblocked port");
    }
}
```
