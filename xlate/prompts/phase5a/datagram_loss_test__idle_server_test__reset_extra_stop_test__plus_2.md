# Phase 5A C/Rust test correspondence audit

Review each C/Rust test pair and decide whether the Rust
`#[test]` checks the same behavior as the C test.  This is
a read-only pass: do not edit files.

You may inspect directly relevant context when needed:
Rust test helpers, fixtures, translated implementation under
`rs/fq/`, C helper functions in `picoquictest/`, constants,
and nearby tests.  Do not require byte-for-byte structure;
idiomatic Rust is fine when it preserves the C test intent,
inputs, expected results, and important edge cases.

Classify each entry as:

* `ok` when the Rust test is an acceptable translation.
* `needs_fix` when the Rust test is missing checks, checks
  materially different behavior, weakens assertions, skips
  cases the C test covers, or has placeholder-like logic.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"reviews":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short conclusion","fix_summary":"what 5B should change, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/datagram_tests.c:datagram_loss_test`
* C test-table name: `datagram_loss`
* C entry function: `datagram_loss_test`
* Rust test: `datagram_loss`
* C source: `picoquictest/datagram_tests.c:635-646`
* Rust source: `rs/fq/src/tests/datagram.rs:712-721`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 100;
    dg_ctx.send_delay = 20000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;

    return datagram_test_one(4, &dg_ctx, 0x040080100200400ull);
}
```

### Rust test body
```rust
fn datagram_loss() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        ..Default::default()
    };
    datagram_test_one(4, &mut dg_ctx, 0x040080100200400);
}
```

## `picoquictest/edge_cases.c:idle_server_test`
* C test-table name: `idle_server`
* C entry function: `idle_server_test`
* Rust test: `idle_server`
* C source: `picoquictest/edge_cases.c:846-860`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1226-1234`

### C test body
```c
{
    int ret = 0;

    if ((ret = idle_server_test_one(1, 30000, 0, 30100000)) == 0 &&
        (ret = idle_server_test_one(2, 60000, 0, 60100000)) == 0 &&
        (ret = idle_server_test_one(3, 5000, 0, 5100000)) == 0 &&
        (ret = idle_server_test_one(4, 0, 0, 30100000)) == 0 &&
        (ret = idle_server_test_one(5, 0, 10000, 10100000)) == 0 &&
        (ret = idle_server_test_one(6, 20000, 60000, 60100000)) == 0 &&
        (ret = idle_server_test_one(7, 60000, 5000, 5100000)) == 0){
        DBG_PRINTF("%s", "All idle timeout tests pass.\n");
    }
    return ret;
}
```

### Rust test body
```rust
fn idle_server() {
    idle_server_test_one(1, 30_000, 0, 30_100_000).expect("case 1");
    idle_server_test_one(2, 60_000, 0, 60_100_000).expect("case 2");
    idle_server_test_one(3, 5_000, 0, 5_100_000).expect("case 3");
    idle_server_test_one(4, 0, 0, 30_100_000).expect("case 4");
    idle_server_test_one(5, 0, 10_000, 10_100_000).expect("case 5");
    idle_server_test_one(6, 20_000, 60_000, 60_100_000).expect("case 6");
    idle_server_test_one(7, 60_000, 5_000, 5_100_000).expect("case 7");
}
```

## `picoquictest/edge_cases.c:reset_extra_stop_test`
* C test-table name: `reset_extra_stop`
* C entry function: `reset_extra_stop_test`
* Rust test: `reset_extra_stop`
* C source: `picoquictest/edge_cases.c:1213-1216`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1361-1363`

### C test body
```c
{
    return reset_repeat_test_one(reset_extra_stop_sending);
}
```

### Rust test body
```rust
fn reset_extra_stop() {
    reset_repeat_test_one(ResetTestKind::ExtraStop).expect("reset_extra_stop");
}
```

## `picoquictest/flow_control_test.c:flow_control_test`
* C test-table name: `flow_control`
* C entry function: `flow_control_test`
* Rust test: `flow_control`
* C source: `picoquictest/flow_control_test.c:361-374`
* Rust source: `rs/fq/src/tests/flow_control.rs:368-373`

### C test body
```c
{
	fctest_spec_t spec = { 0 };
	spec.test_id = 1;
	spec.transfer_size = 1000000;
	spec.microsecs_per_byte = 10;
	spec.credit_quantum = 0x4000;
	spec.initial_credit = 0x10000;
	spec.bytes_buffered_max = 0x4000;
	spec.completion_target = 11000000;
	spec.ccalgo = picoquic_bbr_algorithm;

	return fctest_one(&spec);
}
```

### Rust test body
```rust
fn flow_control() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr algorithm");
    fctest_one(
        1, bbr, 0, 1_000_000, 10, 0x4000, 0x1_0000, 0x4000, 11_000_000,
    );
}
```

## `picoquictest/high_latency_test.c:high_latency_cubic_test`
* C test-table name: `high_latency_cubic`
* C entry function: `high_latency_cubic_test`
* Rust test: `high_latency_cubic`
* C source: `picoquictest/high_latency_test.c:306-311`
* Rust source: `rs/fq/src/tests/high_latency.rs:298-314`

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Rust test body
```rust
fn high_latency_cubic() {
    let latency = 5_000_000u64;
    let cubic = get_congestion_algorithm("cubic").expect("cubic");
    high_latency_one(
        0xcb,
        cubic,
        HILAT_SCENARIO_100MB,
        200_000_000,
        latency,
        10,
        10,
        0,
        false,
        false,
        false,
    );
}
```
