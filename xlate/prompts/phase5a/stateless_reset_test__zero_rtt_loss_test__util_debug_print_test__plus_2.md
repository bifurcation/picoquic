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

## `picoquictest/tls_api_test.c:stateless_reset_test`
* C test-table name: `stateless_reset`
* C entry function: `stateless_reset_test`
* Rust test: `stateless_reset`
* C source: `picoquictest/tls_api_test.c:3397-3457`
* Rust source: `rs/fq/src/tests/tls_api.rs:1258-1260`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    uint8_t buffer[128];
    int was_active = 0;

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    /* verify that client and server have the same reset secret */
    if (ret == 0) {
        uint8_t ref_secret[PICOQUIC_RESET_SECRET_SIZE];

        (void)picoquic_create_cnxid_reset_secret(test_ctx->qserver,
            &test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id, ref_secret);
        if (memcmp(test_ctx->cnx_client->path[0]->first_tuple->p_remote_cnxid->reset_secret, ref_secret,
            PICOQUIC_RESET_SECRET_SIZE) != 0) {
            ret = -1;
        }
    }

    /* Prepare to reset */
    if (ret == 0) {
        picoquic_delete_cnx(test_ctx->cnx_server);
        test_ctx->cnx_server = NULL;

        memset(buffer, 0xaa, sizeof(buffer));
        ret = picoquic_add_to_stream(test_ctx->cnx_client, 4,
            buffer, sizeof(buffer), 1);
    }

    /* Perform a couple rounds of sending data */
    for (int i = 0; ret == 0 && i < 64 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected; i++) {
        was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, 0, &was_active);
    }

    /* Client should now be in state disconnected */
    if (ret == 0 && test_ctx->cnx_client->cnx_state != picoquic_state_disconnected) {
        ret = -1;
    }

    if (ret == 0 && test_ctx->reset_received == 0) {
        ret = -1;
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
fn stateless_reset() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("stateless_reset");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_loss_test`
* C test-table name: `zero_rtt_loss`
* C entry function: `zero_rtt_loss_test`
* Rust test: `zero_rtt_loss`
* C source: `picoquictest/tls_api_test.c:4630-4645`
* Rust source: `rs/fq/src/tests/tls_api.rs:1603-1611`

### C test body
```c
{
    int ret = 0;

    for (unsigned int i = 1; ret == 0 && i < 16; i++) {
        zero_rtt_test_t zrt = { 0 };
        zrt.early_loss = 1ull << i;

        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Zero RTT test fails when packet #%d is lost.\n", i);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn zero_rtt_loss() {
    for i in 1u32..16 {
        zero_rtt_test_one(&ZeroRttTest {
            early_loss: 1u64 << i,
            ..Default::default()
        })
        .unwrap_or_else(|e| panic!("zero_rtt_loss i={i}: {e:?}"));
    }
}
```

## `picoquictest/util_test.c:util_debug_print_test`
* C test-table name: `util_debug_print`
* C entry function: `util_debug_print_test`
* Rust test: `util_debug_print`
* C source: `picoquictest/util_test.c:335-356`
* Rust source: `rs/fq/src/tests/util_test.rs:194-216`

### C test body
```c
{
    int ret = 0;
    int was_suspened = get_debug_suspended();
    FILE* old_debug_file = get_debug_out();
    FILE* F = picoquic_file_open(file_test_debug, "w");

    if (F == NULL) {
        ret = -1;
    }
    else {
        debug_set_stream(F);
        debug_printf_resume();
        debug_printf("debug set stream: %d\n", ret);
        F = picoquic_file_close(F);
        debug_set_stream(old_debug_file);
        if (was_suspened) {
            debug_printf_suspend();
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn util_debug_print() {
    use std::io::Write as _;

    struct DebugFile(std::fs::File);

    impl core::fmt::Write for DebugFile {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.0.write_all(s.as_bytes()).map_err(|_| core::fmt::Error)
        }
    }

    const FILE_TEST_DEBUG: &str = "file_test_debug.txt";

    let was_suspended = crate::utils::debug_printf_reset(false);
    let file = std::fs::File::create(FILE_TEST_DEBUG).expect("create debug output");
    crate::utils::debug_set_stream(Some(Box::new(DebugFile(file))));
    crate::utils::debug_printf("debug set stream: 0\n");
    crate::utils::debug_set_stream(None);
    crate::utils::debug_printf_reset(was_suspended);

    let written = std::fs::read_to_string(FILE_TEST_DEBUG).expect("read debug output");
    assert_eq!(written, "debug set stream: 0\n");
}
```

## `picoquictest/wifitest.c:wifi_bbr_test`
* C test-table name: `wifi_bbr`
* C entry function: `wifi_bbr_test`
* Rust test: `wifi_bbr`
* C source: `picoquictest/wifitest.c:217-224`
* Rust source: `rs/fq/src/tests/wifitest.rs:105-108`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_bbr_algorithm, 2800000);
    int ret = wifi_test_one(wifi_test_bbr, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_bbr() {
    let spec = default_spec("bbr", SUSPENSION_BASIC, 2_800_000);
    wifi_test_one(WIFI_TEST_BBR, &spec).expect("wifi_bbr");
}
```

## `picoquictest/wifitest.c:wifi_cubic_test`
* C test-table name: `wifi_cubic`
* C entry function: `wifi_cubic_test`
* Rust test: `wifi_cubic`
* C source: `picoquictest/wifitest.c:235-243`
* Rust source: `rs/fq/src/tests/wifitest.rs:211-214`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_cubic_algorithm, 2870000);

    int ret = wifi_test_one(wifi_test_cubic, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_cubic() {
    let spec = default_spec("cubic", SUSPENSION_BASIC, 2_870_000);
    wifi_test_one(WIFI_TEST_CUBIC, &spec).expect("wifi_cubic");
}
```
