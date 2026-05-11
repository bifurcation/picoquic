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

## `picoquictest/hashtest.c:picohash_embedded_test`
* C test-table name: `picohash_embedded`
* C entry function: `picohash_embedded_test`
* Rust test: `picohash_embedded`
* C source: `picoquictest/hashtest.c:202-205`
* Rust source: `rs/fq/src/tests/hashtest.rs:86-132`

### C test body
```c
{
    return(picohash_test_one(1));
}
```

### Rust test body
```rust
fn picohash_embedded() {
    use crate::hash::HashTable;

    let hash_seed: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut t: HashTable<u64, ()> =
        HashTable::with_seed(32, &hash_seed).expect("create hash table with seed");

    assert_eq!(t.len(), 0);

    for i in (1u64..10).step_by(2) {
        assert!(t.insert(i, ()).is_ok(), "insert({i}) failed");
    }
    assert_eq!(t.len(), 5);

    for i in (1u64..10).step_by(2) {
        assert!(t.lookup(&i).is_some(), "lookup({i}) failed");
    }

    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.insert(key, ()).is_ok(), "insert({key}) failed");
        }
    }
    assert_eq!(t.len(), 11);

    for k in (1u64..6).step_by(4) {
        for j in 1u64..=k {
            let key = k + 32 * j;
            assert!(t.lookup(&key).is_some(), "lookup({key}) failed");
        }
    }

    for i in (0u64..=10).step_by(2) {
        assert!(t.lookup(&i).is_none(), "lookup({i}) returned invalid item");
    }

    for i in (1u64..10).step_by(4) {
        let tok = t.lookup(&i).expect("pre-delete lookup");
        t.remove(tok);
    }
    assert_eq!(t.len(), 8);

    for i in (1u64..10).step_by(4) {
        assert!(t.lookup(&i).is_none(), "deleted value {i} still found");
    }
}
```

## `picoquictest/mbedtls_test.c:mbedtls_test`
* C test-table name: `mbedtls`
* C entry function: `mbedtls_test`
* Rust test: `mbedtls`
* C source: `picoquictest/mbedtls_test.c:73-118`
* Rust source: `rs/fq/src/tests/mbedtls.rs:377-409`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t target_time = 1000000;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x99, 0xbe, 0xd7, 0x15, 0, 0, 0, 0}, 8 };
    int ret = 0;

    picoquic_tls_api_reset(TLS_API_INIT_FLAGS_NO_OPENSSL|TLS_API_INIT_FLAGS_NO_FUSION);

    ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid, 8, 0, 0, 1);
    if (ret == 0) {
        picoquic_set_binlog(test_ctx->qserver, ".");
        test_ctx->qserver->use_long_log = 1;
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 20000, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_mbedtls, sizeof(test_scenario_mbedtls));
    }

    /* Try to complete the data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, target_time);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    picoquic_tls_api_reset(0);

    return ret;
}
```

### Rust test body
```rust
fn mbedtls() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;
    let target_time: u64 = 1_000_000;

    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL | TLS_API_INIT_FLAGS_NO_FUSION);

    let initial_cid = crate::ConnectionId::clone_from_slice(&[0x99, 0xbe, 0xd7, 0x15, 0, 0, 0, 0])
        .expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex2(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 20_000, &mut simulated_time)
        .expect("connection loop");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MBEDTLS).expect("init scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
        .expect("scenario verify");

    reset_tls_api(0);
}
```

## `picoquictest/mediatest.c:mediatest_suspension2_test`
* C test-table name: `mediatest_suspension2`
* C entry function: `mediatest_suspension2_test`
* Rust test: `mediatest_suspension2`
* C source: `picoquictest/mediatest.c:1512-1529`
* Rust source: `rs/fq/src/tests/mediatest.rs:398-412`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.1;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.latency_average = 50000;
    spec.latency_max = 300000;
    spec.do_not_check_video2 = 1;
    spec.do_probe_up = 1;
    ret = mediatest_one(mediatest_suspension2, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_suspension2() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 50_000,
        latency_max: 300_000,
        do_not_check_video2: true,
        do_probe_up: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Suspension2, &spec).expect("mediatest_suspension2");
}
```

## `picoquictest/mediatest.c:mediatest_worst_test`
* C test-table name: `mediatest_worst`
* C entry function: `mediatest_worst_test`
* Rust test: `mediatest_worst`
* C source: `picoquictest/mediatest.c:1436-1448`
* Rust source: `rs/fq/src/tests/mediatest.rs:345-355`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    spec.data_size = 10000000;
    ret = mediatest_one(mediatest_worst, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_worst() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        data_size: 10_000_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::Worst, &spec).expect("mediatest_worst");
}
```

## `picoquictest/multipath_test.c:monopath_0rtt_loss_test`
* C test-table name: `monopath_0rtt_loss`
* C entry function: `monopath_0rtt_loss_test`
* Rust test: `monopath_0rtt_loss`
* C source: `picoquictest/multipath_test.c:1708-1723`
* Rust source: `rs/fq/src/tests/multipath.rs:1348-1358`

### C test body
```c
{
    int ret = 0;

    for (unsigned int i = 1; ret == 0 && i < 16; i++) {
        zero_rtt_test_t zrt = { 0 };
        zrt.early_loss = 1ull << i;
        zrt.do_multipath = 1;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Monopath 0 RTT test fails when packet #%d is lost.\n", i);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn monopath_0rtt_loss() {
    for i in 1u32..16 {
        let zrt = ZeroRttTest {
            early_loss: 1u64 << i,
            do_multipath: true,
            ..ZeroRttTest::default()
        };
        zero_rtt_test_one(&zrt)
            .unwrap_or_else(|_| panic!("monopath_0rtt_loss fails at packet #{i}"));
    }
}
```
