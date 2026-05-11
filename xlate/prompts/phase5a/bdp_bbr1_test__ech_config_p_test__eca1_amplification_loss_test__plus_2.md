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

## `picoquictest/congestion_test.c:bdp_bbr1_test`
* C test-table name: `bdp_bbr1`
* C entry function: `bdp_bbr1_test`
* Rust test: `bdp_bbr1`
* C source: `picoquictest/congestion_test.c:730-733`
* Rust source: `rs/fq/src/tests/congestion.rs:942-944`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_bbr1);
}
```

### Rust test body
```rust
fn bdp_bbr1() {
    bdp_option_test_one(BdpTestOption::Bbr1);
}
```

## `picoquictest/ech_test.c:ech_config_p_test`
* C test-table name: `ech_config_p`
* C entry function: `ech_config_p_test`
* Rust test: `ech_config_p`
* C source: `picoquictest/ech_test.c:124-158`
* Rust source: `rs/fq/src/tests/ech.rs:251-257`

### C test body
```c
{
    int ret = 0;
    char test_server_key_file[512];
    const char* public_name = "test.example.com";
    uint8_t* config = NULL;
    size_t config_len = 0;

    if (picoquic_hpke_kems[0] == NULL) {
        picoquic_tls_api_init();
    }

    ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir,
        PICOQUIC_TEST_ECH_PRIVATE_KEY);
    if (ret != 0) {
        DBG_PRINTF("Cannot locate %s", PICOQUIC_TEST_ECH_PRIVATE_KEY);
    }
    else if ((ret = picoquic_ech_create_config_from_private_key(&config, &config_len, test_server_key_file, public_name)) != 0) {
        DBG_PRINTF("Cannot create ECH record from <%s>, err: %d (0x%x)", test_server_key_file, ret, ret);
    }

    /* Save a config representation in ech_config.txt */
    if (ret == 0) {
        ret = picoquic_ech_save_config(config, config_len, ECH_CONFIG_FILE_TXT);
        if (ret == 0) {
            ret = ech_test_check_buf(config, config_len, PICOQUIC_TEST_ECH_CONFIG_REF);
        }
    }

    if (config != NULL) {
        free(config);
    }

    return ret;
}
```

### Rust test body
```rust
fn ech_config_p() {
    tls_api_init();
    let config = ech_create_config_from_private_key(TEST_ECH_PRIVATE_KEY, ECH_PUBLIC_NAME)
        .expect("create ECH config from private key");
    ech_save_config(&config, ECH_CONFIG_FILE).expect("save ECH config");
    ech_test_check_buf(&config, TEST_ECH_CONFIG_REF).expect("config matches reference");
}
```

## `picoquictest/edge_cases.c:eca1_amplification_loss_test`
* C test-table name: `eca1_amplification_loss`
* C entry function: `eca1_amplification_loss_test`
* Rust test: `eca1_amplification_loss`
* C source: `picoquictest/edge_cases.c:418-439`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1145-1150`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t initial_losses = 0x0FF4;
    uint8_t test_case_id = 0xa1;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 0, &simulated_time, initial_losses, 16);

    if (ret == 0) {
        ret = edge_case_complete(test_ctx, &simulated_time, 15000000);
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
fn eca1_amplification_loss() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut test_ctx =
        edge_case_prepare(0xa1, false, &mut simulated_time, 0x0FF4, 16).expect("edge_case_prepare");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 15_000_000).expect("edge_case_complete");
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

## `picoquictest/edge_cases.c:reset_need_stop_test`
* C test-table name: `reset_need_stop`
* C entry function: `reset_need_stop_test`
* Rust test: `reset_need_stop`
* C source: `picoquictest/edge_cases.c:1228-1231`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1379-1381`

### C test body
```c
{
    return reset_repeat_test_one(reset_need_stop_sending);
}
```

### Rust test body
```rust
fn reset_need_stop() {
    reset_repeat_test_one(ResetTestKind::NeedStop).expect("reset_need_stop");
}
```
