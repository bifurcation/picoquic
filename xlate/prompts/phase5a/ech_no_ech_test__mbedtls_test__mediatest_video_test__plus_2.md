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

## `picoquictest/ech_test.c:ech_no_ech_test`
* C test-table name: `ech_no_ech`
* C entry function: `ech_no_ech_test`
* Rust test: `ech_no_ech`
* C source: `picoquictest/ech_test.c:465-473`
* Rust source: `rs/fq/src/tests/ech.rs:297-305`

### C test body
```c
{
    ech_e2e_spec_t spec = { 0 };
    spec.expect_grease = 0;
    spec.no_ech_server = 1;
    spec.complete_cnx = 1;
    spec.try_twice = 1;
    return ech_e2e_test_one(&spec);
}
```

### Rust test body
```rust
fn ech_no_ech() {
    let spec = EchE2eSpec {
        no_ech_server: true,
        complete_cnx: true,
        try_twice: true,
        ..Default::default()
    };
    ech_e2e_test_one(&spec);
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

## `picoquictest/mediatest.c:mediatest_video_test`
* C test-table name: `mediatest_video`
* C entry function: `mediatest_video_test`
* Rust test: `mediatest_video`
* C source: `picoquictest/mediatest.c:1342-1353`
* Rust source: `rs/fq/src/tests/mediatest.rs:229-237`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    ret = mediatest_one(mediatest_video, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_video() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video, &spec).expect("mediatest_video");
}
```

## `picoquictest/memlog_test.c:memlog_test`
* C test-table name: `memlog`
* C entry function: `memlog_test`
* Rust test: `memlog`
* C source: `picoquictest/memlog_test.c:159-171`
* Rust source: `rs/fq/src/tests/memlog.rs:137-141`

### C test body
```c
{
    int ret = memlog_test_one(0, MEMLOG_FILE, 0);

    if (ret == 0) {
        ret = memlog_test_one(1, MEMLOG_FILE_MP, 0);
    }

    if (ret == 0) {
        ret = memlog_test_one(1, MEMLOG_FILE_BAD, 1);
    }
    return ret;
}
```

### Rust test body
```rust
fn memlog() {
    memlog_test_one(false, MEMLOG_FILE, false);
    memlog_test_one(true, MEMLOG_FILE_MP, false);
    memlog_test_one(true, MEMLOG_FILE_BAD, true);
}
```

## `picoquictest/multipath_test.c:monopath_0rtt_loss_test`
* C test-table name: `monopath_0rtt_loss`
* C entry function: `monopath_0rtt_loss_test`
* Rust test: `monopath_0rtt_loss_2`
* C source: `picoquictest/multipath_test.c:1708-1723`
* Rust source: `rs/fq/src/tests/multipath.rs:1362-1372`

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
fn monopath_0rtt_loss_2() {
    for i in 1u32..16 {
        let zrt = ZeroRttTest {
            early_loss: 1u64 << i,
            do_multipath: true,
            ..ZeroRttTest::default()
        };
        zero_rtt_test_one(&zrt)
            .unwrap_or_else(|_| panic!("monopath_0rtt_loss_2 fails at packet #{i}"));
    }
}
```
