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

## `picoquictest/tls_api_test.c:spurious_retransmit_test`
* C test-table name: `spurious_retransmit`
* C entry function: `spurious_retransmit_test`
* Rust test: `spurious_retransmit`
* C source: `picoquictest/tls_api_test.c:5128-5170`
* Rust source: `rs/fq/src/tests/tls_api.rs:1242-1244`

### C test body
```c
{
    uint64_t loss_mask = 0;
    uint64_t simulated_time = 0;
    uint64_t next_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        test_ctx->c_to_s_link->microsec_latency = 50000ull;
        test_ctx->s_to_c_link->microsec_latency = 50000ull;

        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* simulate 1 second of silence */
    next_time = simulated_time + 1000000ull;
    while (ret == 0 && simulated_time < next_time && TEST_CLIENT_READY && TEST_SERVER_READY) {
        int was_active = 0;

        ret = tls_api_one_sim_round(test_ctx, &simulated_time, next_time, &was_active);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
    }

    if (ret == 0) {
        /* verify the absence of any spurious retransmission */
        if (test_ctx->cnx_client->nb_spurious != 0) {
            ret = -1;
        } else if (test_ctx->cnx_server != NULL && test_ctx->cnx_server->nb_spurious != 0) {
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
fn spurious_retransmit() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("spurious_retransmit");
}
```

## `picoquictest/tls_api_test.c:tls_api_wrong_alpn_test`
* C test-table name: `tls_api_wrong_alpn`
* C entry function: `tls_api_wrong_alpn_test`
* Rust test: `tls_api_wrong_alpn`
* C source: `picoquictest/tls_api_test.c:2993-3039`
* Rust source: `rs/fq/src/tests/tls_api.rs:1448-1450`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_WRONG_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        /* By default, client and servers are using the same ALPN. Correct that on the server side 
         * so we can test the wrong ALPN condition */
        free((void*)test_ctx->qserver->default_alpn);
        test_ctx->qserver->default_alpn = picoquic_string_duplicate(PICOQUIC_TEST_ALPN);
    }

    if (ret != 0)
    {
        DBG_PRINTF("Could not create the QUIC test contexts for V=%x\n", 0);
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, 0, 0, &simulated_time);

        if (ret == 0)
        {
            if (test_ctx->cnx_client != NULL) {
                if (test_ctx->cnx_client->cnx_state == picoquic_state_disconnected &&
                    test_ctx->cnx_client->remote_error == PICOQUIC_TLS_ALERT_WRONG_ALPN) {
                    ret = 0;
                }
                else {
                    DBG_PRINTF("Connection loop returns 0x%" PRIx64, test_ctx->cnx_client->remote_error);
                    ret = -1;
                }
            }
            else {
                DBG_PRINTF("%s", "Could not establish a client connection");
                ret = -1;
            }
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
fn tls_api_wrong_alpn() {
    tls_api_test_with_loss(None, 0, Some(TEST_SNI), Some("wrong-alpn")).expect("wrong_alpn");
}
```

## `picoquictest/tls_api_test.c:zero_rtt_delay_test`
* C test-table name: `zero_rtt_delay`
* C entry function: `zero_rtt_delay_test`
* Rust test: `zero_rtt_delay`
* C source: `picoquictest/tls_api_test.c:4751-4775`
* Rust source: `rs/fq/src/tests/tls_api.rs:1565-1571`

### C test body
```c
{
    int ret = 0;
    int bad_ret;
    const uint64_t nominal_delay_sec = 100000;
    const uint64_t nominal_delay = nominal_delay_sec * 1000000;
    zero_rtt_test_t zrt = { 0 };
    zrt.long_data = 1;
    zrt.extra_delay = nominal_delay + 1000000;

    bad_ret = zero_rtt_test_one(&zrt);
    if (bad_ret == 0) {
        DBG_PRINTF("Zero RTT succeed despite delay = %" PRIu64, " + 1 second.", nominal_delay_sec);
        ret = -1;
    }
    else {
        zrt.extra_delay = nominal_delay - 2000000;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Zero RTT fails for delay = %" PRIu64, " - 2 seconds.", nominal_delay_sec);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn zero_rtt_delay() {
    zero_rtt_test_one(&ZeroRttTest {
        extra_delay: 100_000,
        ..Default::default()
    })
    .expect("zero_rtt_delay");
}
```

## `picoquictest/transport_param_test.c:transport_param_test`
* C test-table name: `transport_param`
* C entry function: `transport_param_test`
* Rust test: `transport_param`
* C source: `picoquictest/transport_param_test.c:854-983`
* Rust source: `rs/fq/src/tests/transport_param.rs:19-45`

### C test body
```c
{
    int ret = 0;
    uint64_t proof = 0;
    uint32_t version_default = picoquic_supported_versions[0].version;

    ret = transport_param_one_test(0, 0, version_default, version_default,
        &transport_param_test1, client_param1, sizeof(client_param1));
    if (ret != 0) {
        DBG_PRINTF("Param test TP1, CP1 returns %x\n", ret);
    } else {
        ret = transport_param_one_test(0, 0, version_default, 0x0A1A0A1A,
            &transport_param_test2, client_param2, sizeof(client_param2));
        if (ret != 0) {
            DBG_PRINTF("Param test TP2, CP2 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test3, client_param3, sizeof(client_param3));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP3, CP3 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(1, 0, version_default, version_default,
            &transport_param_test4, server_param1, sizeof(server_param1));
        if (ret != 0) {
            DBG_PRINTF("Param test TP4, SP1 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(1, 0, version_default, 0x0A1A0A1A,
            &transport_param_test5, server_param2, sizeof(server_param2));
        if (ret != 0) {
            DBG_PRINTF("Param test TP5, SP2 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test6, client_param4, sizeof(client_param4));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP6, CP4 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0xBABABABA,
            &transport_param_test7, client_param5, sizeof(client_param5));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP7, CP5 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test8, client_param8, sizeof(client_param8));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP8, CP8 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(1, version_default, 0x0A1A0A1A,
            &transport_param_test9, server_param3, sizeof(server_param3));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP9, SP3 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(0, 0, version_default, version_default,
            &transport_param_test10, client_param9, sizeof(client_param9));
        if (ret != 0) {
            DBG_PRINTF("Param test TP10, CP9 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_decode_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test8, client_param10, sizeof(client_param10));
        if (ret != 0) {
            DBG_PRINTF("Decode test TP8, CP10 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(0, 1, version_default, version_default,
            &transport_param_test1, client_param11, sizeof(client_param11));
        if (ret != 0) {
            DBG_PRINTF("Param test TP1, CP1 returns %x\n", ret);
        }
    }

    if (ret == 0) {
        ret = transport_param_one_test(0, 0, version_default, version_default,
            &transport_param_test11, client_param12, sizeof(client_param12));
        if (ret != 0) {
            DBG_PRINTF("Param test TP11, CP12 returns %x\n", ret);
        }
    }

    for (size_t i = 0; ret == 0 && i < nb_transport_param_error_case; i++) {
        ret = transport_param_error_test(transport_param_error_case[i].mode, transport_param_error_case[i].target, 
            transport_param_error_case[i].target_length, transport_param_error_case[i].local_error);
        if (ret != 0) {
            DBG_PRINTF("Param error test %d fails\n", (int)i);
        }
    }

    if (ret == 0)
    {
        DBG_PRINTF("%s", "Starting transport parameters fuzz test.\n");
        
        ret = transport_param_fuzz_test(0, version_default, 0x0A1A0A1A,
            &transport_param_test2, client_param2, sizeof(client_param2), &proof);

        if (ret == 0) {
            ret = transport_param_fuzz_test(1, version_default, 0x0A1A0A1A,
                &transport_param_test2, server_param2, sizeof(server_param2), &proof);
        }

        DBG_PRINTF("%s", "End of transport parameters fuzz test.\n");
    }
    return ret;
}
```

### Rust test body
```rust
fn transport_param() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");

    let test_cases: &[TransportParameters] = &[
        TransportParameters::default(),
        TransportParameters {
            initial_max_stream_data_bidi_local: 65535,
            ..Default::default()
        },
        TransportParameters {
            initial_max_data: 0x0040_0000,
            ..Default::default()
        },
        TransportParameters {
            max_idle_timeout: Duration::from_ticks(30_000), // 30 ms in µs
            ..Default::default()
        },
    ];

    for (i, tp) in test_cases.iter().enumerate() {
        transport_param_test_one(&mut ctx.qclient, tp, true)
            .unwrap_or_else(|e| panic!("client tp[{i}]: {e:?}"));
        transport_param_test_one(&mut ctx.qserver, tp, false)
            .unwrap_or_else(|e| panic!("server tp[{i}]: {e:?}"));
    }
}
```

## `picoquictest/util_test.c:util_memcmp_test`
* C test-table name: `util_memcmp`
* C entry function: `util_memcmp_test`
* Rust test: `util_memcmp`
* C source: `picoquictest/util_test.c:163-328`
* Rust source: `rs/fq/src/tests/util_test.rs:89-141`

### C test body
```c
{
    int ret = 0;
    size_t nb8 = (1 << 20) / sizeof(uint64_t);
    size_t l_total = nb8 * sizeof(uint64_t);
    uint64_t* x8 = (uint64_t*)malloc(l_total);
    uint8_t* x = (uint8_t*)x8;
    uint8_t* y = (uint8_t*)malloc(l_total);
    uint64_t random_seed = 0xbabac001;
    uint64_t time_start;
    uint64_t const_compare_time[16];
#if 0
    uint64_t memcmp_time[2];
#endif
    uint64_t carry;
    uint64_t nb_round = 2;

    if (x == NULL || y == NULL) {
        ret = -1;
    }
    else {
        x8[0] = 0;
        x8[1] = 0;
        x8[2] = 0xffffffffffffffffull;
        x8[3] = 0xffffffffffffffffull;

        for (size_t i = 4; i < nb8; i++) {
            x8[i] = picoquic_test_random(&random_seed);
        }
        /* test for correct detection of equality */
        memcpy(y, x, l_total);
        for (size_t j = 0; j < l_total; j += 16) {
            if (picoquic_constant_time_memcmp(x + j, y + j, 16) != 0) {
                DBG_PRINTF("Unexpected mismatch, rank %d\n", (int)j);
                ret = -1;
                break;
            }
        }

        for (size_t i = 0; ret == 0 && i < 16; i++) {
            /* prepare the y string */
            memcpy(y, x, l_total);
            for (size_t j = i; j < l_total; j += 16) {
                y[j] ^= (uint8_t)0xff;
            }

            /* test for correct detection of differences */
            for (size_t j = 0; j < l_total; j += 16) {
                if (picoquic_constant_time_memcmp(x + j, y + j, 16) == 0) {
                    DBG_PRINTF("Unexpected match, step %d, rank %d\n", (int)i, (int)j);
                    ret = -1;
                    break;
                }
            }
        }


        /* Time measurement: Compare a long string, at 16 different intervals. */
        while (ret == 0) {
            int zero_found = 0;
            for (size_t i = 0; ret == 0 && i < 16; i++) {
                /* prepare the y string */
                memcpy(y, x, l_total);
                y[1 + ((i * l_total) / 16)] ^= 0xff;

                carry = 1;
                time_start = picoquic_current_time();
                for (uint64_t j = 0; j < nb_round; j++) {
                    x[j] ^= 1;
                    y[j] ^= 1;
                    carry &= (picoquic_constant_time_memcmp(x, y, l_total) != 0);
                }
                const_compare_time[i] = picoquic_current_time() - time_start;
                if (carry != 1) {
                    DBG_PRINTF("Unexpected match, step %d\n", (int)i);
                    ret = -1;
                    break;
                }
                if (i == 0 && const_compare_time[i] < 2000) {
                    zero_found = 1;
                    break;
                }
            }

            if (zero_found) {
                nb_round *= 2;
            }
            else {
                break;
            }
        }
    }

    if (ret == 0) {
        DBG_PRINTF("%s", "Delta at, const memcmp (ns)\n");
        for (size_t i = 0; ret == 0 && i < 16; i++) {
            double d = 1000.0*(double)(const_compare_time[i]) / (double)(nb_round * l_total / 16);
            DBG_PRINTF("%d, %f\n", 1 + ((i * l_total) / 16), d);
        }
    }

    for (size_t i = 0; ret == 0 && i < 16; i++) {
        /* The time tests are information only, because measuring time is to susceptible to random noise */
        if (i > 0 && const_compare_time[0] >= 1000 && const_compare_time[i] >= 1000 && ((const_compare_time[i] > 2 * const_compare_time[0]) || (const_compare_time[0] > 2 * const_compare_time[i]))) {
            DBG_PRINTF("Step %d, const cmp time different from step 0, %d vs %d\n", (int)i, (int)const_compare_time[i], (int)const_compare_time[0]);
        }
    }

    #if 0
    while (ret == 0) {
        for (size_t i = 0; ret == 0 && i < 2; i++) {
            /* prepare the y string */
            memcpy(y, x, l_total);

            for (size_t j = 15*i; j < l_total; j += 16) {
                y[j] ^= (uint8_t)0xff;
            }


            carry = 1;
            time_start = picoquic_current_time();

            for (int r = 0; r < nb_round; r++) {
                /* measure compare time */
                for (size_t j = 0; j < l_total; j += 16) {
                    carry &= (memcmp(x, y, 16) != 0);
                }

                if (carry != 1) {
                    DBG_PRINTF("Unexpected memcmp match, step %d\n", (int)i);
                    ret = -1;
                    break;
                }
            }

            memcmp_time[i] = picoquic_current_time() - time_start;
        }

        if (memcmp_time[0] > 2000 && memcmp_time[1] > 2000) {
            break;
        }
        nb_round *= 2;
    } 

    if (ret == 0){
        if (memcmp_time[1] > 2 * memcmp_time[0]) {
            DBG_PRINTF("Memcmp not constant time on 16 bytes: t[0] = %d, t[15] = %d\n", (int)memcmp_time[0], (int)memcmp_time[1]);
            DBG_PRINTF("%s", "Need to compile with -DPICOQUIC_USE_CONSTANT_TIME_MEMCMP");
            ret = -1;
        }
        else {
            DBG_PRINTF("Memcmp constant time on 16 bytes: t[0] = %d, t[15] = %d\n", (int)memcmp_time[0], (int)memcmp_time[1]);
        }
    }
#endif

    if (x != NULL) {
        free(x);
    }

    if (y != NULL) {
        free(y);
    }

    return ret;
}
```

### Rust test body
```rust
fn util_memcmp() {
    use core::cmp::Ordering;

    use crate::utils::constant_time_memcmp;

    let nb_words = (1 << 17) / 8;
    let l_total = nb_words * 8;
    let mut x = vec![0u8; l_total];
    // Borrow a deterministic test RNG so the data isn't all zeros.
    let mut seed = 0xbabac001u64;
    let next = |seed: &mut u64| -> u64 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *seed
    };
    for chunk in x[32..].chunks_mut(8) {
        chunk.copy_from_slice(&next(&mut seed).to_le_bytes());
    }
    x[16..32].copy_from_slice(&[0xff; 16]);

    let y = x.clone();

    // Equality detection.
    let mut j = 0;
    while j < l_total {
        assert_eq!(
            constant_time_memcmp(&x[j..j + 16], &y[j..j + 16]),
            Ordering::Equal,
            "unexpected mismatch at byte {j}",
        );
        j += 16;
    }

    // Difference detection: flip one byte per 16-byte group.
    for offset in 0..16 {
        let mut y = x.clone();
        let mut j = offset;
        while j < l_total {
            y[j] ^= 0xff;
            j += 16;
        }
        let mut j = 0;
        while j < l_total {
            assert_ne!(
                constant_time_memcmp(&x[j..j + 16], &y[j..j + 16]),
                Ordering::Equal,
                "unexpected match at offset={offset}, byte={j}",
            );
            j += 16;
        }
    }
}
```
