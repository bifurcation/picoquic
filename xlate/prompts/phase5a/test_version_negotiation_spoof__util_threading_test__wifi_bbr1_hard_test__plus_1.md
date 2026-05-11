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

## `picoquictest/tls_api_test.c:test_version_negotiation_spoof`
* C test-table name: `version_negotiation_spoof`
* C entry function: `test_version_negotiation_spoof`
* Rust test: `version_negotiation_spoof`
* C source: `picoquictest/tls_api_test.c:2879-2897`
* Rust source: `rs/fq/src/tests/tls_api.rs:1516-1519`

### C test body
```c
{
    int ret = 0;

    if (test_version_negotiation_spoof_one(0) == 0) {
        DBG_PRINTF("%s", "VN spoof mode 0 has no effect");
        ret = -1;
    }

    for (int spoof_mode = 1; ret == 0 && spoof_mode < 8; spoof_mode++) {
        ret = test_version_negotiation_spoof_one(spoof_mode);
        if (ret != 0) {
            DBG_PRINTF("VN spoof mode %d caused failure", spoof_mode);
            ret = -1;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn version_negotiation_spoof() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
        .expect("version_negotiation_spoof");
}
```

## `picoquictest/util_test.c:util_threading_test`
* C test-table name: `threading`
* C entry function: `util_threading_test`
* Rust test: `threading`
* C source: `picoquictest/util_test.c:389-456`
* Rust source: `rs/fq/src/tests/util_test.rs:225-283`

### C test body
```c
{
    thread_test_data_t ctx;
    picoquic_thread_t thread;
    uint64_t x;
    int ret;

    memset(&ctx, 0, sizeof(ctx));
    ret = picoquic_create_mutex(&ctx.mutex);
    if (ret != 0) {
        DBG_PRINTF("Create mutex returns %d (0x%x)", ret, ret);
    }

    if (ret == 0){
        ret = picoquic_create_event(&ctx.event);
        if (ret != 0) {
            DBG_PRINTF("Create event returns %d (0x%x)", ret, ret);
        }
    }

    if (ret == 0) {
        ret = picoquic_create_thread(&thread, thread_test_function, &ctx);
        if (ret != 0) {
            DBG_PRINTF("Create thread returns %d (0x%x)", ret, ret);
        }
    }

    while (ret == 0 && ctx.data == 0) {
        ret = picoquic_wait_for_event(&ctx.event, 10000);
        if (ret != 0) {
            DBG_PRINTF("Cannot wait for event, ret = %d (0x%x)", ret, ret);
        }
    }

    for (int i = 0; ret == 0 && i < 10; i++)
    {
        ret = picoquic_lock_mutex(&ctx.mutex);
        if (ret == 0) {
            x = ctx.data;
            ctx.data = x + 1;
            ret = picoquic_unlock_mutex(&ctx.mutex);
            if (ret != 0) {
                DBG_PRINTF("Cannot unlock the mutex, ret = %d (0x%x)", ret, ret);
            }
        }
        else {
            DBG_PRINTF("Cannot lock the mutex, ret = %d (0x%x)", ret, ret);
        }
    }

    while (ret == 0 && ctx.data < 30) {
        ret = picoquic_wait_for_event(&ctx.event, 10000);
        if (ret != 0) {
            DBG_PRINTF("Cannot wait for event, ret = %d (0x%x)", ret, ret);
        }
    }

    picoquic_delete_thread(&thread);
    picoquic_delete_event(&ctx.event);
    picoquic_delete_mutex(&ctx.mutex);

    if (ret == 0 && ctx.data != 30) {
        DBG_PRINTF("Could not count to %d, got %d", 30, ctx.data);
        ret = -1;
    }

    return ret;
}
```

### Rust test body
```rust
fn threading() {
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    #[derive(Default)]
    struct ThreadTestData {
        data: u64,
    }

    let ctx = Arc::new((Mutex::new(ThreadTestData::default()), Condvar::new()));
    let worker_ctx = Arc::clone(&ctx);
    let thread = std::thread::spawn(move || {
        let (mutex, event) = &*worker_ctx;
        for _ in 0..20 {
            let mut guard = mutex.lock().expect("worker mutex");
            let x = guard.data;
            guard.data = x + 1;
            drop(guard);
            event.notify_one();
        }
    });

    let (mutex, event) = &*ctx;
    let mut guard = mutex.lock().expect("main mutex");
    while guard.data == 0 {
        let wait = event
            .wait_timeout(guard, Duration::from_micros(10_000))
            .expect("wait for first event");
        guard = wait.0;
        assert!(
            !wait.1.timed_out() || guard.data != 0,
            "first event timed out"
        );
    }
    drop(guard);

    for _ in 0..10 {
        let mut guard = mutex.lock().expect("main increment mutex");
        let x = guard.data;
        guard.data = x + 1;
    }

    let mut guard = mutex.lock().expect("final wait mutex");
    while guard.data < 30 {
        let wait = event
            .wait_timeout(guard, Duration::from_micros(10_000))
            .expect("wait for final event");
        guard = wait.0;
        assert!(
            !wait.1.timed_out() || guard.data >= 30,
            "final event timed out"
        );
    }
    let data = guard.data;
    drop(guard);

    thread.join().expect("join worker thread");
    assert_eq!(data, 30);
}
```

## `picoquictest/wifitest.c:wifi_bbr1_hard_test`
* C test-table name: `wifi_bbr1_hard`
* C entry function: `wifi_bbr1_hard_test`
* Rust test: `wifi_bbr1_hard`
* C source: `picoquictest/wifitest.c:281-295`
* Rust source: `rs/fq/src/tests/wifitest.rs:119-130`

### C test body
```c
{
    wifi_test_spec_t spec = {
        nb_suspension_hard,
        3000,
        suspension_hard,
        picoquic_bbr1_algorithm,
        NULL,
        4060000,
        0,
        0 };
    int ret = wifi_test_one(wifi_test_bbr1_hard, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_bbr1_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "bbr1",
        cc_algo_option: None,
        target_time: 4_060_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR1_HARD, &spec).expect("wifi_bbr1_hard");
}
```

## `picoquictest/wifitest.c:wifi_reno_test`
* C test-table name: `wifi_reno`
* C entry function: `wifi_reno_test`
* Rust test: `wifi_reno`
* C source: `picoquictest/wifitest.c:245-252`
* Rust source: `rs/fq/src/tests/wifitest.rs:248-251`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_newreno_algorithm, 2800000);
    int ret = wifi_test_one(wifi_test_reno, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_reno() {
    let spec = default_spec("newreno", SUSPENSION_BASIC, 2_800_000);
    wifi_test_one(WIFI_TEST_RENO, &spec).expect("wifi_reno");
}
```
