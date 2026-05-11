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

## `picoquictest/edge_cases.c:ec00_zero_test`
* C test-table name: `ec00_zero`
* C entry function: `ec00_zero_test`
* Rust test: `ec00_zero`
* C source: `picoquictest/edge_cases.c:295-321`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1083-1092`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = edge_case_prepare(&test_ctx, 0, 1, &simulated_time, 0, 4);

    if (ret == 0) {
        ret = edge_case_complete(test_ctx, &simulated_time, 100000);
    }

    if (ret == 0) {
        if (test_ctx->cnx_client->nb_zero_rtt_acked == 0) {
            DBG_PRINTF("Nb 0RTT acked = %d", test_ctx->cnx_client->nb_zero_rtt_acked);
            ret = -1;
        }
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn ec00_zero() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0x00, true, &mut simulated_time, 0, 4).expect("edge_case_prepare");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 100_000).expect("edge_case_complete");
    assert!(
        test_ctx.cnx_client().nb_zero_rtt_acked > 0,
        "expected at least one 0-RTT packet acked"
    );
}
```

## `picoquictest/edge_cases.c:reset_ack_max_test`
* C test-table name: `reset_ack_max`
* C entry function: `reset_ack_max_test`
* Rust test: `reset_ack_max`
* C source: `picoquictest/edge_cases.c:1193-1196`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1337-1339`

### C test body
```c
{
    return reset_repeat_test_one(reset_ack_max_stream);
}
```

### Rust test body
```rust
fn reset_ack_max() {
    reset_repeat_test_one(ResetTestKind::AckMaxStream).expect("reset_ack_max");
}
```

## `picoquictest/edge_cases.c:reset_stream_at_basic_test`
* C test-table name: `reset_stream_at_basic`
* C entry function: `reset_stream_at_basic_test`
* Rust test: `reset_stream_at_basic`
* C source: `picoquictest/edge_cases.c:2013-2016`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1510-1512`

### C test body
```c
{
    return reset_stream_at_test_one(rsat_basic);
}
```

### Rust test body
```rust
fn reset_stream_at_basic() {
    reset_stream_at_test_one(ResetStreamAtSpec::Basic).expect("reset_stream_at_basic");
}
```

## `picoquictest/hashtest.c:siphash_test`
* C test-table name: `siphash`
* C entry function: `siphash_test`
* Rust test: `siphash`
* C source: `picoquictest/hashtest.c:264-335`
* Rust source: `rs/fq/src/tests/hashtest.rs:174-201`

### C test body
```c
{
    uint8_t test[1024];
    uint8_t k[16];
    size_t test_lengths[12] = { 1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024 };
    uint64_t href[12] = {
        0xa9b786935f98d6b8,
        0x3fb64f2d81ebf107,
        0xcd34491a7b437e1b,
        0x5fbe917709286bc4,
        0xb2cc76e0f81d6e2f,
        0x09e69c0f70753651,
        0xc615b5349acc0cc2,
        0x965379fb0e26e150,
        0x85a286cfc4a62574,
        0x5f774367aeea9f83,
        0xd04ee1d420e9bc22,
        0x0a7ad6655680779e
    };
    int ret = 0;

    hash_test_init(test, sizeof(test), k, sizeof(k));
    /* Compute or check the reference siphash value */
    for (size_t i = 0; i < sizeof(test_lengths) / sizeof(size_t); i++) {
        uint64_t h = picohash_siphash(test, test_lengths[i], k);
        if (h != href[i]) {
            DBG_PRINTF("H[%zu] = %" PRIu64 "instead of %"PRIu64, i, h, href[i]);
#ifdef COMPUTING_REFERENCE_SIPASH_VALUE
            href[i] = h;
#else
            ret = -1;
            break;
#endif
        }
    }
#ifdef COMPARING_TIMES
    /* Compare execution time */
    uint64_t h;
    double sip_t[48];
    double basic_t[48];
    for (size_t lt=1; lt <= 48; lt++) {
        uint64_t start_siphash = picoquic_current_time();
        uint64_t siphash_sum = 0;
        uint64_t basic_sum = 0;
        size_t n = 0;

        for (size_t i = 0; i + lt < sizeof(test); i++) {
            h = picohash_siphash(test, lt, k);
            siphash_sum += h;
            n++;
        }
        uint64_t start_basic = picoquic_current_time();
        for (size_t i = 0; i + lt < sizeof(test); i++) {
            h = picohash_bytes(test, (uint32_t)lt, k);
            basic_sum += h;
        }
        uint64_t end_basic = picoquic_current_time();
        uint64_t siphash_time = start_basic - start_siphash;
        uint64_t basic_time = end_basic - start_basic;
        double siphash_one = ((double)siphash_time) / n;
        double basic_one = ((double)basic_time) / n;
        sip_t[lt - 1] = siphash_one;
        basic_t[lt - 1] = basic_one;
        printf("Sip hash time, %zu: %" PRIu64 ", sum: %" PRIu64", n = % zu, us=%f\n", lt, siphash_time, siphash_sum, n, siphash_one);
        printf("Basic hash time, %zu: %" PRIu64 ", sum: %" PRIu64", n = % zu, us=%f\n", lt, basic_time, basic_sum, n, basic_one);
    }
    for (int i = 0; i < 48; i++) {
        printf("%d, %f, %f\n", i + 1, basic_t[i], sip_t[i]);
    }
#endif /* COMPARING TIMES */
    return ret;
}
```

### Rust test body
```rust
fn siphash() {
    use crate::siphash::siphash;

    let mut test = [0u8; 1024];
    let mut k = [0u8; 16];
    hash_test_init(&mut test, &mut k);

    let lengths = [1, 3, 7, 8, 12, 16, 17, 31, 127, 257, 515, 1024];
    let expected: [u64; 12] = [
        0xa9b7_8693_5f98_d6b8,
        0x3fb6_4f2d_81eb_f107,
        0xcd34_491a_7b43_7e1b,
        0x5fbe_9177_0928_6bc4,
        0xb2cc_76e0_f81d_6e2f,
        0x09e6_9c0f_7075_3651,
        0xc615_b534_9acc_0cc2,
        0x9653_79fb_0e26_e150,
        0x85a2_86cf_c4a6_2574,
        0x5f77_4367_aeea_9f83,
        0xd04e_e1d4_20e9_bc22,
        0x0a7a_d665_5680_779e,
    ];

    for (i, &len) in lengths.iter().enumerate() {
        let h = siphash(&test[..len], &k);
        assert_eq!(h, expected[i], "siphash[{i}] for len={len}");
    }
}
```

## `picoquictest/l4s_test.c:l4s_bbr_test`
* C test-table name: `l4s_bbr`
* C entry function: `l4s_bbr_test`
* Rust test: `l4s_bbr`
* C source: `picoquictest/l4s_test.c:152-159`
* Rust source: `rs/fq/src/tests/l4s.rs:176-179`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgo = picoquic_bbr_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 3800000, 21, 3000, 0, NULL);

    return ret;
}
```

### Rust test body
```rust
fn l4s_bbr() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    l4s_congestion_test(ccalgo, true, 3_800_000, 21, 3_000, &[]);
}
```
