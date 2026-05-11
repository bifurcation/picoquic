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

## `picoquictest/edge_cases.c:ec2f_second_flight_nack_test`
* C test-table name: `ec2f_second_flight`
* C entry function: `ec2f_second_flight_nack_test`
* Rust test: `ec2f_second_flight`
* C source: `picoquictest/edge_cases.c:334-361`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1098-1110`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t initial_losses = 0x1c1;
    uint8_t test_case_id = 0x2f;
    int ret = edge_case_prepare(&test_ctx, test_case_id, 1, &simulated_time, initial_losses, 9);

    if (ret == 0) {
        if (test_ctx->cnx_client->cnx_state >= picoquic_state_ready ||
            test_ctx->cnx_server->cnx_state != picoquic_state_ready) {
            DBG_PRINTF("Unexpected state, client: %d, server: %d",
                test_ctx->cnx_client->cnx_state, test_ctx->cnx_server->cnx_state);
            ret = -1;
        }
    }

    if (ret == 0) {
        ret = edge_case_complete(test_ctx, &simulated_time, 360000);
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
fn ec2f_second_flight() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_losses = 0x1c1u64;
    let mut test_ctx = edge_case_prepare(0x2f, true, &mut simulated_time, initial_losses, 9)
        .expect("edge_case_prepare");
    // After 9 rounds: client must not yet be Ready; server must be Ready.
    assert!(
        !test_ctx.client_ready(),
        "client should not be ready yet after partial handshake"
    );
    assert!(test_ctx.server_ready(), "server should be ready");
    edge_case_complete(&mut test_ctx, &mut simulated_time, 360_000).expect("edge_case_complete");
}
```

## `picoquictest/intformattest.c:intformattest`
* C test-table name: `intformat`
* C entry function: `intformattest`
* Rust test: `intformat`
* C source: `picoquictest/intformattest.c:48-161`
* Rust source: `rs/fq/src/tests/intformattest.rs:35-58`

### C test body
```c
{
    /* Test the formating routines */
    int ret = 0;
    uint8_t bytes[8];
    uint64_t decoded;
    uint64_t parsed;
    uint32_t test32;
    uint32_t test24;
    uint16_t test16;
    uint64_t test64;

    for (int new_encoding = 0; new_encoding < 2; new_encoding++) {
        /* First test with 16 bits macros */
        for (size_t i = 0; ret == 0 && i < nb_test_numbers; i++) {
            test16 = (uint16_t)test_number[i];
            if (new_encoding == 0) {
                picoformat_16(bytes, test16);
            }
            else {
                uint8_t* next_byte = picoquic_frames_uint16_encode(bytes, bytes + sizeof(bytes), test16);
                if ((next_byte - bytes) != 2) {
                    ret = -1;
                }
            }
            decoded = decode_number(bytes, 2);
            if (decoded != test16) {
                ret = -1;
            }
            else {
                parsed = PICOPARSE_16(bytes);
                if (parsed != test16) {
                    ret = -1;
                }
            }
        }

        /* Next test with 24 bits macros */
        for (size_t i = 0; ret == 0 && i < nb_test_numbers; i++) {

            test24 = (uint32_t)(test_number[i]&0xFFFFFF);
            if (new_encoding == 0) {
                picoformat_24(bytes, test24);
            }
            else {
                uint8_t* next_byte = picoquic_frames_uint24_encode(bytes, bytes + sizeof(bytes), test24);
                if ((next_byte - bytes) != 3) {
                    ret = -1;
                }
            }
            decoded = decode_number(bytes, 3);
            if (decoded != test24) {
                ret = -1;
            }
            else {
                parsed = PICOPARSE_24(bytes);
                if (parsed != test24) {
                    ret = -1;
                }
            }
        }

        /* Next test with 32 bits macros */
        for (size_t i = 0; ret == 0 && i < nb_test_numbers; i++) {
            test32 = (uint32_t)test_number[i];
            if (new_encoding == 0) {
                picoformat_32(bytes, test32);
            }
            else {
                uint8_t* next_byte = picoquic_frames_uint32_encode(bytes, bytes + sizeof(bytes), test32);
                if ((next_byte - bytes) != 4) {
                    ret = -1;
                }
            }
            decoded = decode_number(bytes, 4);
            if (decoded != test32) {
                ret = -1;
            }
            else {
                parsed = PICOPARSE_32(bytes);
                if (parsed != test32) {
                    ret = -1;
                }
            }
        }

        /* Test with 64 bits macros */
        for (size_t i = 0; ret == 0 && i < nb_test_numbers; i++) {
            test64 = test_number[i];
            picoformat_64(bytes, test64);
            if (new_encoding == 0) {
                picoformat_64(bytes, test64);
            }
            else {
                uint8_t* next_byte = picoquic_frames_uint64_encode(bytes, bytes + sizeof(bytes), test64);
                if ((next_byte - bytes) != 8) {
                    ret = -1;
                }
            }
            decoded = decode_number(bytes, 8);
            if (decoded != test64) {
                ret = -1;
            }
            else {
                parsed = PICOPARSE_64(bytes);
                if (parsed != test64) {
                    ret = -1;
                }
            }
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn intformat() {
    let mut buf = [0u8; 8];

    for &n in TEST_NUMBERS {
        let n16 = n as u16;
        format_16(&mut buf, n16);
        assert_eq!(decode_number(&buf, 2), n16 as u64, "u16 BE bytes mismatch");
        assert_eq!(parse_16(&buf), n16, "parse_16 roundtrip");

        let n24 = (n & 0xFF_FFFF) as u32;
        format_24(&mut buf, n24);
        assert_eq!(decode_number(&buf, 3), n24 as u64, "u24 BE bytes mismatch");
        assert_eq!(parse_24(&buf), n24, "parse_24 roundtrip");

        let n32 = n as u32;
        format_32(&mut buf, n32);
        assert_eq!(decode_number(&buf, 4), n32 as u64, "u32 BE bytes mismatch");
        assert_eq!(parse_32(&buf), n32, "parse_32 roundtrip");

        format_64(&mut buf, n);
        assert_eq!(decode_number(&buf, 8), n, "u64 BE bytes mismatch");
        assert_eq!(parse_64(&buf), n, "parse_64 roundtrip");
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

## `picoquictest/mediatest.c:mediatest_suspension_test`
* C test-table name: `mediatest_suspension`
* C entry function: `mediatest_suspension_test`
* Rust test: `mediatest_suspension`
* C source: `picoquictest/mediatest.c:1490-1510`
* Rust source: `rs/fq/src/tests/mediatest.rs:376-393`

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
    spec.nb_suspensions = 1;
    spec.suspension_start_time = 4000000;
    spec.suspension_down_time = 150000;
    spec.suspension_up_time = 50000;
    ret = mediatest_one(mediatest_suspension, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_suspension() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 50_000,
        latency_max: 300_000,
        do_not_check_video2: true,
        nb_suspensions: 1,
        suspension_start_time: 4_000_000,
        suspension_down_time: 150_000,
        suspension_up_time: 50_000,
        ..Default::default()
    };
    mediatest_one(MediatestId::Suspension, &spec).expect("mediatest_suspension");
}
```

## `picoquictest/mediatest.c:mediatest_wifi_test`
* C test-table name: `mediatest_wifi`
* C entry function: `mediatest_wifi_test`
* Rust test: `mediatest_wifi`
* C source: `picoquictest/mediatest.c:1465-1488`
* Rust source: `rs/fq/src/tests/mediatest.rs:322-341`

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
    spec.link_latency = 15000;
    spec.latency_average = 60000;
    spec.latency_max = 350000;
    spec.priority_limit_for_bypass = 5;
    spec.do_not_check_video2 = 1;
    spec.nb_suspensions = 20;
    spec.suspension_start_time = 4000000;
    spec.suspension_down_time = 150000;
    spec.suspension_up_time = 0;

    ret = mediatest_one(mediatest_wifi, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_wifi() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.01,
        do_video: true,
        do_video2: true,
        do_audio: true,
        link_latency: 15_000,
        latency_average: 60_000,
        latency_max: 350_000,
        priority_limit_for_bypass: 5,
        do_not_check_video2: true,
        nb_suspensions: 20,
        suspension_start_time: 4_000_000,
        suspension_down_time: 150_000,
        suspension_up_time: 0,
        ..Default::default()
    };
    mediatest_one(MediatestId::Wifi, &spec).expect("mediatest_wifi");
}
```
