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

## `picoquictest/tls_api_test.c:tls_api_q_and_r_stream_test`
* C test-table name: `tls_api_q_and_r_stream`
* C entry function: `tls_api_q_and_r_stream_test`
* Rust test: `tls_api_q_and_r_stream`
* C source: `picoquictest/tls_api_test.c:3271-3274`
* Rust source: `rs/fq/src/tests/tls_api.rs:1384-1388`

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0, 0, 0, 0, 75000, NULL, NULL);
}
```

### Rust test body
```rust
fn tls_api_q_and_r_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 75_000).expect("q_and_r_stream");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_bad_param_test`
* C test-table name: `zero_rtt_bad_param`
* C entry function: `zero_rtt_bad_param_test`
* Rust test: `zero_rtt_bad_param`
* C source: `picoquictest/tls_api_test.c:4669-4674`
* Rust source: `rs/fq/src/tests/tls_api.rs:1552-1558`

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.change_params = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Rust test body
```rust
fn zero_rtt_bad_param() {
    zero_rtt_test_one(&ZeroRttTest {
        change_params: true,
        ..Default::default()
    })
    .expect("zero_rtt_bad_param");
}
```

## `picoquictest/transport_param_test.c:transport_param_default_test`
* C test-table name: `transport_param_default`
* C entry function: `transport_param_default_test`
* Rust test: `transport_param_default`
* C source: `picoquictest/transport_param_test.c:1434-1453`
* Rust source: `rs/fq/src/tests/transport_param.rs:52-87`

### C test body
```c
{
    int ret = 0;
    for (size_t i = 0; ret == 0 && i < nb_default_test_case; i++) {
        picoquic_quic_t quic = { 0 };
        int r = picoquic_set_default_tp_value(&quic, tp_default_test_case[i].tp_id, tp_default_test_case[i].tp_val);

        if (r != tp_default_test_case[i].ret) {
            ret = -1;
        }
        else if (r == 0) {
            ret = tp_value_check(&quic, tp_default_test_case[i].tp_id, tp_default_test_case[i].tp_val);
        }
        if (ret != 0) {
            DBG_PRINTF("param default test fails for test %zu: 0x%" PRIu64 ", 0x%"  PRIu64,
                i, tp_default_test_case[i].tp_id, tp_default_test_case[i].tp_val);
        }
    }
    return ret;        
}
```

### Rust test body
```rust
fn transport_param_default() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    let param_ids: &[u64] = &[
        0x00,               // initial_max_stream_data_bidi_local
        0x01,               // initial_max_data
        0x02,               // initial_max_stream_id_bidi
        0x03,               // max_idle_timeout
        0x04,               // preferred_address (skip, complex)
        0x05,               // max_packet_size
        0x06,               // stateless_reset_token
        0x07,               // ack_delay_exponent
        0x08,               // initial_max_stream_id_uni
        0x09,               // migration_disabled
        0x0a,               // initial_max_stream_data_bidi_remote
        0x0b,               // initial_max_stream_data_uni
        0x0c,               // max_ack_delay
        0x0d,               // original_destination_connection_id
        0x0e,               // retry_source_connection_id
        0x0f,               // version_negotiation
        0x10,               // max_datagram_frame_size
        0x20,               // enable_loss_bit
        0x7157,             // grease_quic_bit
        0x2ab2,             // enable_time_stamp
        0x4143,             // min_ack_delay
        0xff02de1a,         // enable_bdp_frame
        0x0f739bbc1b666d04, // initial_max_path_id
    ];

    for &id in param_ids {
        ctx.qserver
            .set_default_tp_value(id, 0)
            .unwrap_or_else(|e| panic!("set_default_tp_value({id:#x}): {e:?}"));
    }
}
```

## `picoquictest/util_test.c:util_sprintf_test`
* C test-table name: `util_sprintf`
* C entry function: `util_sprintf_test`
* Rust test: `util_sprintf`
* C source: `picoquictest/util_test.c:91-114`
* Rust source: `rs/fq/src/tests/util_test.rs:150-186`

### C test body
```c
{
    int ret = 0;
    size_t nb_chars;
    char str[8];
    if (picoquic_sprintf(str, sizeof(str), NULL, "%s%s", "foo", "bar") != 0) {
        DBG_PRINTF("%s", "'foobar' test failed.");
        ret = -1;
    }
    if (picoquic_sprintf(str, sizeof(str), &nb_chars, "%s%s%s", "foo", PICOQUIC_FILE_SEPARATOR, "bar") != 0 ||
        nb_chars != 7) {
        DBG_PRINTF("'foo/bar' test failed. Nb_chars = %d", (int)nb_chars);
        ret = -1;
    }
    if (picoquic_sprintf(str, sizeof(str), NULL, "%s%s%s", "fooo", PICOQUIC_FILE_SEPARATOR, "bar") == 0) {
        DBG_PRINTF("%s", "'fooo/bar' test failed.");
        ret = -1;
    }
    if (picoquic_sprintf(str, sizeof(str), NULL, "%s%s%s", "fooo", PICOQUIC_FILE_SEPARATOR, "barr") == 0) {
        DBG_PRINTF("%s", "'fooo/barr' test failed.");
        ret = -1;
    }
    return ret;
}
```

### Rust test body
```rust
fn util_sprintf() {
    use crate::utils::{FILE_SEPARATOR, sprintf};

    let mut str_buf = [0u8; 8];
    let n = sprintf(&mut str_buf, &format!("{}{}", "foo", "bar")).expect("'foobar'");
    assert_eq!(n, 6);
    assert_eq!(&str_buf[..7], b"foobar\0");

    let mut str_buf = [0u8; 8];
    let n = sprintf(
        &mut str_buf,
        &format!("{}{}{}", "foo", FILE_SEPARATOR, "bar"),
    )
    .expect("'foo/bar'");
    assert_eq!(n, 7);
    assert_eq!(&str_buf, b"foo/bar\0");

    let mut str_buf = [0u8; 8];
    assert!(
        sprintf(
            &mut str_buf,
            &format!("{}{}{}", "fooo", FILE_SEPARATOR, "bar")
        )
        .is_err(),
        "'fooo/bar' should not fit"
    );

    let mut str_buf = [0u8; 8];
    assert!(
        sprintf(
            &mut str_buf,
            &format!("{}{}{}", "fooo", FILE_SEPARATOR, "barr")
        )
        .is_err(),
        "'fooo/barr' should not fit"
    );
}
```

## `picoquictest/wifitest.c:wifi_bbr1_test`
* C test-table name: `wifi_bbr1`
* C entry function: `wifi_bbr1_test`
* Rust test: `wifi_bbr1`
* C source: `picoquictest/wifitest.c:226-233`
* Rust source: `rs/fq/src/tests/wifitest.rs:112-115`

### C test body
```c
{
    wifi_test_spec_t spec;
    wifi_test_set_default_spec(&spec, picoquic_bbr1_algorithm, 2800000);
    int ret = wifi_test_one(wifi_test_bbr, &spec);

    return ret;
}
```

### Rust test body
```rust
fn wifi_bbr1() {
    let spec = default_spec("bbr1", SUSPENSION_BASIC, 2_800_000);
    wifi_test_one(WIFI_TEST_BBR1, &spec).expect("wifi_bbr1");
}
```
