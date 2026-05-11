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

## `picoquictest/tls_api_test.c:tls_api_client_second_loss_test`
* C test-table name: `second_loss`
* C entry function: `tls_api_client_second_loss_test`
* Rust test: `second_loss`
* C source: `picoquictest/tls_api_test.c:4160-4163`
* Rust source: `rs/fq/src/tests/tls_api.rs:1163-1165`

### C test body
```c
{
    return tls_api_loss_test(2ull);
}
```

### Rust test body
```rust
fn second_loss() {
    tls_api_loss_test(2).expect("second_loss");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_ech_test`
* C test-table name: `zero_rtt_ech`
* C entry function: `zero_rtt_ech_test`
* Rust test: `zero_rtt_ech`
* C source: `picoquictest/tls_api_test.c:4781-4786`
* Rust source: `rs/fq/src/tests/tls_api.rs:1578-1584`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.propose_ech = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt_ech() {
    zero_rtt_test_one(&ZeroRttTest {
        propose_ech: true,
        ..Default::default()
    })
    .expect("zero_rtt_ech");
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

## `picoquictest/warptest.c:warptest_worst_test`
* C test-table name: `warptest_worst`
* C entry function: `warptest_worst_test`
* Rust test: `warptest_worst`
* C source: `picoquictest/warptest.c:1538-1550`
* Rust source: `rs/fq/src/tests/warptest.rs:73-83`

### C test body
```c
{
    int ret;
    warptest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.01;
    spec.do_video = 1;
    spec.do_audio = 1;
    spec.datagram_data_size = 10000000;
    ret = warptest_one(4, &spec);

    return ret;
}
```

### Rust test body
```rust
fn warptest_worst() {
    let spec = WarptestSpec {
        ccalgo_id: Some("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_audio: true,
        datagram_data_size: 10_000_000,
        ..Default::default()
    };
    warptest_one(4, &spec).expect("warptest_worst");
}
```

## `picoquictest/wifitest.c:wifi_bbr_shadow_test`
* C test-table name: `wifi_bbr_shadow`
* C entry function: `wifi_bbr_shadow_test`
* Rust test: `wifi_bbr_shadow`
* C source: `picoquictest/wifitest.c:384-395`
* Rust source: `rs/fq/src/tests/wifitest.rs:196-207`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_bbr_algorithm, 2750000);
    spec.cc_algo_option = "T250000";
    spec.queue_max_delay = 600000;
    spec.simulate_receive_block = 1;

    int ret = wifi_test_one(wifi_test_bbr_shadow, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_bbr_shadow() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr",
        cc_algo_option: Some("T250000"),
        target_time: 2_750_000,
        simulate_receive_block: true,
        queue_max_delay: 600_000,
    };
    wifi_test_one(WIFI_TEST_BBR_SHADOW, &spec).expect("wifi_bbr_shadow");
}
```
