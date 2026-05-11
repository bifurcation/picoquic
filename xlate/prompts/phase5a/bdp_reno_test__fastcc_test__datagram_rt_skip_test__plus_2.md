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

## `picoquictest/congestion_test.c:bdp_reno_test`
* C test-table name: `bdp_reno`
* C entry function: `bdp_reno_test`
* Rust test: `bdp_reno`
* C source: `picoquictest/congestion_test.c:697-700`
* Rust source: `rs/fq/src/tests/congestion.rs:912-914`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_reno);
}
```

### Rust test body
```rust
fn bdp_reno() {
    bdp_option_test_one(BdpTestOption::Reno);
}
```

## `picoquictest/congestion_test.c:fastcc_test`
* C test-table name: `fastcc`
* C entry function: `fastcc_test`
* Rust test: `fastcc`
* C source: `picoquictest/congestion_test.c:130-133`
* Rust source: `rs/fq/src/tests/congestion.rs:761-764`

### C test body
```c
{
    return congestion_control_test(picoquic_fastcc_algorithm, 3700000, 0, 0);
}
```

### Rust test body
```rust
fn fastcc() {
    let ccalgo = get_congestion_algorithm("fastcc").expect("fastcc cc algo");
    congestion_control_test(ccalgo, 3_700_000, 0, 0);
}
```

## `picoquictest/datagram_tests.c:datagram_rt_skip_test`
* C test-table name: `datagram_rt_skip`
* C entry function: `datagram_rt_skip_test`
* Rust test: `datagram_rt_skip`
* C source: `picoquictest/datagram_tests.c:599-614`
* Rust source: `rs/fq/src/tests/datagram.rs:681-692`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 10;
    dg_ctx.send_delay = 20000;
    dg_ctx.next_gen_time[0] = 100000;
    dg_ctx.next_gen_time[1] = 100000;
    dg_ctx.dg_latency_target[0] = 13000;
    dg_ctx.dg_latency_target[1] = 20000;
    dg_ctx.do_skip_test[0] = 1;
    dg_ctx.do_skip_test[1] = 1;

    return datagram_test_one(3, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_rt_skip() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 10],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [13_000, 20_000],
        do_skip_test: [true, true],
        ..Default::default()
    };
    datagram_test_one(3, &mut dg_ctx, 0);
}
```

## `picoquictest/delay_tolerant_test.c:dtn_basic_test`
* C test-table name: `dtn_basic`
* C entry function: `dtn_basic_test`
* Rust test: `dtn_basic`
* C source: `picoquictest/delay_tolerant_test.c:182-190`
* Rust source: `rs/fq/src/tests/delay_tolerant.rs:184-188`

### C test body
```c
{
    /* Simple test. */
    dtn_test_spec_t spec;
    dtn_set_basic_test_spec(&spec);
    spec.max_number_of_packets = 120;

    return dtn_test_one(0xba, &spec);
}
```

### Rust test body
```rust
fn dtn_basic() {
    let mut spec = dtn_basic_spec();
    spec.max_number_of_packets = 120;
    dtn_test_one(0xba, &spec);
}
```

## `picoquictest/ech_test.c:ech_e2e_0rtt_test`
* C test-table name: `ech_e2e_0rtt`
* C entry function: `ech_e2e_0rtt_test`
* Rust test: `ech_e2e_0rtt`
* C source: `picoquictest/ech_test.c:449-456`
* Rust source: `rs/fq/src/tests/ech.rs:273-281`

### C test body
```c
{
    ech_e2e_spec_t spec = { 0 };
    spec.expect_grease = 1;
    spec.complete_cnx = 1;
    spec.try_twice = 1;
    return ech_e2e_test_one(&spec);
}
```

### Rust test body
```rust
fn ech_e2e_0rtt() {
    let spec = EchE2eSpec {
        expect_grease: true,
        complete_cnx: true,
        try_twice: true,
        ..Default::default()
    };
    ech_e2e_test_one(&spec);
}
```
