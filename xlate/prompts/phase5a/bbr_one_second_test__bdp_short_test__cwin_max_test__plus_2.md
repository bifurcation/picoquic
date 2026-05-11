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

## `picoquictest/congestion_test.c:bbr_one_second_test`
* C test-table name: `bbr_one_second`
* C entry function: `bbr_one_second_test`
* Rust test: `bbr_one_second`
* C source: `picoquictest/congestion_test.c:359-370`
* Rust source: `rs/fq/src/tests/congestion.rs:827-832`

### C test body
```c
{
    uint64_t max_completion_time = 90000000;
    uint64_t latency = 1000000;
    uint64_t jitter = 3000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 1;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Rust test body
```rust
fn bbr_one_second() {
    let latency = 1_000_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(90_000_000, 1, latency, jitter, buffer);
}
```

## `picoquictest/congestion_test.c:bdp_short_test`
* C test-table name: `bdp_short`
* C entry function: `bdp_short_test`
* Rust test: `bdp_short`
* C source: `picoquictest/congestion_test.c:702-705`
* Rust source: `rs/fq/src/tests/congestion.rs:924-926`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_short);
}
```

### Rust test body
```rust
fn bdp_short() {
    bdp_option_test_one(BdpTestOption::Short);
}
```

## `picoquictest/congestion_test.c:cwin_max_test`
* C test-table name: `cwin_max`
* C entry function: `cwin_max_test`
* Rust test: `cwin_max`
* C source: `picoquictest/congestion_test.c:1016-1045`
* Rust source: `rs/fq/src/tests/congestion.rs:967-978`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgos[] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_bbr_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr1_algorithm
    };
    uint64_t max_completion_times[] = {
        11000000,
        11000000,
        11000000,
        11000000,
        12100000,
        11000000
    };
    int ret = 0;

    for (size_t i = 0; i < sizeof(ccalgos) / sizeof(picoquic_congestion_algorithm_t*); i++) {
        ret = cwin_max_test_one(ccalgos[i], 68000, max_completion_times[i]);
        if (ret != 0) {
            DBG_PRINTF("CWIN Max test fails for <%s>", ccalgos[i]->congestion_algorithm_id);
            break;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn cwin_max() {
    let algo_names = ["newreno", "cubic", "dcubic", "bbr", "fastcc", "bbr1"];
    let max_completion_times: [u64; 6] = [
        11_000_000, 11_000_000, 11_000_000, 11_000_000, 12_100_000, 11_000_000,
    ];

    for (name, &max_time) in algo_names.iter().zip(max_completion_times.iter()) {
        let ccalgo =
            get_congestion_algorithm(name).unwrap_or_else(|| panic!("cc algo not found: {name}"));
        cwin_max_test_one(ccalgo, 68_000, max_time);
    }
}
```

## `picoquictest/datagram_tests.c:datagram_test`
* C test-table name: `datagram`
* C entry function: `datagram_test`
* Rust test: `datagram`
* C source: `picoquictest/datagram_tests.c:574-582`
* Rust source: `rs/fq/src/tests/datagram.rs:656-663`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = PICOQUIC_MAX_PACKET_SIZE;
    dg_ctx.dg_target[0] = 5;
    dg_ctx.dg_target[1] = 5;

    return datagram_test_one(1, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [5, 5],
        ..Default::default()
    };
    datagram_test_one(1, &mut dg_ctx, 0);
}
```

## `picoquictest/datagram_tests.c:datagram_small_packet_test`
* C test-table name: `datagram_small_packet`
* C entry function: `datagram_small_packet_test`
* Rust test: `datagram_small_packet`
* C source: `picoquictest/datagram_tests.c:712-732`
* Rust source: `rs/fq/src/tests/datagram.rs:770-787`

### C test body
```c
{
    test_datagram_send_recv_ctx_t dg_ctx = { 0 };
    dg_ctx.dg_max_size = 512;
    dg_ctx.dg_small_size = 64;
    dg_ctx.dg_target[0] = 100;
    dg_ctx.dg_target[1] = 20000;
    dg_ctx.send_delay = 100;
    dg_ctx.next_gen_time[0] = 50000;
    dg_ctx.next_gen_time[1] = 50000;
    dg_ctx.link_latency = 10000;
    dg_ctx.picosec_per_byte = 20000; /* 400 Mbps */
    dg_ctx.dg_latency_target[0] = 20000;
    dg_ctx.dg_latency_target[1] = 13500;
    dg_ctx.use_extended_provider_api = 1;
    dg_ctx.one_datagram_per_packet = 1;
    dg_ctx.nb_trials_max = 200000;
    dg_ctx.duration_max = 2060000;

    return datagram_test_one(9, &dg_ctx, 0);
}
```

### Rust test body
```rust
fn datagram_small_packet() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        dg_target: [100, 20_000],
        send_delay: 100,
        next_gen_time: [50_000, 50_000],
        link_latency: 10_000,
        picosec_per_byte: 20_000, // 400 Mbps
        dg_latency_target: [20_000, 13_500],
        use_extended_provider_api: true,
        one_datagram_per_packet: true,
        nb_trials_max: 200_000,
        duration_max: 2_060_000,
        ..Default::default()
    };
    datagram_test_one(9, &mut dg_ctx, 0);
}
```
