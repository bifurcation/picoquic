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

## `picoquictest/mediatest.c:mediatest_video2_down_test`
* C test-table name: `mediatest_video2_down`
* C entry function: `mediatest_video2_down_test`
* Rust test: `mediatest_video2_down`
* C source: `picoquictest/mediatest.c:1382-1398`
* Rust source: `rs/fq/src/tests/mediatest.rs:269-282`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.latency_average = 100000;
    spec.latency_max = 600000;
    spec.do_not_check_video2 = 1;
    ret = mediatest_one(mediatest_video2_down, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_video2_down() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 100_000,
        latency_max: 600_000,
        do_not_check_video2: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Down, &spec).expect("mediatest_video2_down");
}
```

## `picoquictest/multipath_test.c:migration_controlled_test`
* C test-table name: `migration_controlled`
* C entry function: `migration_controlled_test`
* Rust test: `migration_controlled`
* C source: `picoquictest/multipath_test.c:369-372`
* Rust source: `rs/fq/src/tests/multipath.rs:1316-1318`

### C test body
```c
{
    return migration_test_one(0);
}
```

### Rust test body
```rust
fn migration_controlled() {
    migration_test_one(false);
}
```

## `picoquictest/multipath_test.c:monopath_keep_alive_test`
* C test-table name: `monopath_keep_alive`
* C entry function: `monopath_keep_alive_test`
* Rust test: `monopath_keep_alive`
* C source: `picoquictest/multipath_test.c:1684-1688`
* Rust source: `rs/fq/src/tests/multipath.rs:1388-1390`

### C test body
```c
{
    return monopath_test_one(monopath_keep_alive);
}
```

### Rust test body
```rust
fn monopath_keep_alive() {
    monopath_test_one(MonopathTestId::KeepAlive);
}
```

## `picoquictest/multipath_test.c:multipath_basic_test`
* C test-table name: `multipath_basic`
* C entry function: `multipath_basic_test`
* Rust test: `multipath_basic`
* C source: `picoquictest/multipath_test.c:1292-1297`
* Rust source: `rs/fq/src/tests/multipath.rs:1498-1500`

### C test body
```c
{
    uint64_t max_completion_microsec = 1060000;

    return multipath_test_one(max_completion_microsec, multipath_test_basic);
}
```

### Rust test body
```rust
fn multipath_basic() {
    multipath_test_one(1_060_000, MultipathTestId::Basic);
}
```
