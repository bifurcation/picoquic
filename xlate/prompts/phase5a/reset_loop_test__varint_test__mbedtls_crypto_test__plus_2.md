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

## `picoquictest/edge_cases.c:reset_loop_test`
* C test-table name: `reset_loop_test`
* C entry function: `reset_loop_test`
* Rust test: `reset_loop_test`
* C source: `picoquictest/edge_cases.c:1757-1873`
* Rust source: `rs/fq/src/tests/edge_cases.rs:1388-1506`

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t timeout;
    uint64_t test_stream = 8;
    reset_loop_callback_t cb = { 0 };
    picoquic_stream_head_t* stream = NULL;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        uint8_t bogus_data[4] = { 0 };
        picoquic_set_default_callback(test_ctx->qserver, reset_loop_callback, &cb);
        picoquic_set_callback(test_ctx->cnx_client, reset_loop_callback, &cb);
        picoquic_start_client_cnx(test_ctx->cnx_client);
        picoquic_add_to_stream(test_ctx->cnx_client, 4, bogus_data, 4, 0);
        picoquic_add_to_stream(test_ctx->cnx_client, 8, bogus_data, 4, 0);
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        picoquic_mark_active_stream(test_ctx->cnx_client, 4, 1, &cb);
        picoquic_mark_active_stream(test_ctx->cnx_client, 8, 1, &cb);
        /* set priorities */
        if (ret == 0) {
            ret = picoquic_set_stream_priority(test_ctx->cnx_client, 4, 8);
        }
        if (ret == 0) {
            ret = picoquic_set_stream_priority(test_ctx->cnx_client, 8, 8);
        }
    }

    /* Perform a few rounds of sending loop, but not enough to send all the data */
    if (ret == 0) {
        timeout = simulated_time + 100000;
        ret = tls_api_wait_for_timeout(test_ctx, &simulated_time, timeout);
    }

    /* trigger a reset of tst stream */
    if (ret == 0) {
        ret = picoquic_reset_stream(test_ctx->cnx_server, test_stream, 0);
    }

    /* set priorities */
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_client, 4, 9);
    }
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_client, 8, 7);
    }
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_server, 4, 9);
    }
    if (ret == 0) {
        ret = picoquic_set_stream_priority(test_ctx->cnx_server, 8, 7);
    }

    /* make sure that reset is sent */
    if (ret == 0) {
        timeout = simulated_time + 100000;
        for (int i = 0; ret == 0 && i < 16; i++) {
            int was_active = 0;
            ret = tls_api_one_sim_round(test_ctx, &simulated_time, timeout, &was_active);
            if (ret == 0) {
                stream = picoquic_find_stream(test_ctx->cnx_server, test_stream);
                if (stream == NULL) {
                    ret = -1;
                    break;
                }
                else if (stream->reset_sent) {
                    break;
                }
            }
        }
    }
    if (ret == 0 && (stream == NULL || !stream->reset_sent)) {
        DBG_PRINTF("Could not reset stream %" PRIu64, test_stream);
        ret = -1;
    }

    if (ret == 0) {
        /* add data to the stream to elicit some bad behavior */
        uint8_t bogus_data[4] = { 1, 2, 3, 4 }; 
        if (picoquic_add_to_stream(test_ctx->cnx_server, test_stream, bogus_data, 4, 1) == 0) {
            DBG_PRINTF("Adding on stream %" PRIu64 " after reset should be forbidden", test_stream);
            ret = -1;
        }
    }

    if (ret == 0) {
        /* add data to the stream to elicit some bad behavior */
        uint8_t bogus_context[4] = { 0, 0, 0, 0 };
        if (picoquic_mark_active_stream(test_ctx->cnx_server, test_stream, 1, bogus_context) == 0) {
            DBG_PRINTF("Marking stream %" PRIu64 " active after reset should be forbidden", test_stream);
            ret = -1;
        }
    }

    /* Do a loop to check the behavior */
    if (ret == 0) {
        timeout = simulated_time + 2000000;
        ret = tls_api_wait_for_timeout(test_ctx, &simulated_time, timeout);
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
fn reset_loop_test() {
    let mut simulated_time = Instant::from_ticks(0);
    let test_stream: u64 = 8;

    let mut test_ctx = tls_api_init_ctx(
        &mut simulated_time,
        crate::internal::Version::InternalTest1 as u32,
        None,
    )
    .expect("tls_api_init_ctx");

    // The reset-loop callback state used by C is represented here by direct
    // stream inspection after the simulator has delivered the reset.
    test_ctx.qserver.set_default_callback(None);
    test_ctx.cnx_client().set_callback(None);

    test_ctx.cnx_client().start_client().expect("start_client");

    // Queue initial data on streams 4 and 8; triggers server-side stream creation.
    let bogus = [0u8; 4];
    test_ctx
        .cnx_client()
        .add_to_stream(4, &bogus, false)
        .expect("add_to_stream 4");
    test_ctx
        .cnx_client()
        .add_to_stream(8, &bogus, false)
        .expect("add_to_stream 8");

    let mut loss_mask = 0u64;
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    // Enable streaming on both client streams with equal priority.
    test_ctx
        .cnx_client()
        .mark_active_stream(4, true, None)
        .expect("mark active 4");
    test_ctx
        .cnx_client()
        .mark_active_stream(8, true, None)
        .expect("mark active 8");
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 8)
        .expect("set client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 8)
        .expect("set client priority 8");

    // Allow stream data to begin flowing before the reset.
    let timeout = simulated_time.ticks() + 100_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout).expect("100ms wait");

    // Server resets stream 8 while the transfer is in progress.
    test_ctx
        .cnx_server()
        .reset_stream(test_stream, 0)
        .expect("reset_stream");

    // Adjust priorities to expose the bug (different priorities post-reset).
    test_ctx
        .cnx_client()
        .set_stream_priority(4, 9)
        .expect("client priority 4");
    test_ctx
        .cnx_client()
        .set_stream_priority(8, 7)
        .expect("client priority 8");
    test_ctx
        .cnx_server()
        .set_stream_priority(4, 9)
        .expect("server priority 4");
    test_ctx
        .cnx_server()
        .set_stream_priority(8, 7)
        .expect("server priority 8");

    // Poll until the server's RESET_STREAM frame has actually been sent.
    let deadline = Instant::from_ticks(simulated_time.ticks() + 100_000);
    for _ in 0..16 {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            deadline,
            &mut was_active,
        )
        .expect("sim round");
        if check_stream_reset_sent(&mut test_ctx, test_stream) {
            break;
        }
    }
    assert!(
        check_stream_reset_sent(&mut test_ctx, test_stream),
        "server did not send RESET_STREAM for stream {test_stream}"
    );

    // After reset, adding data or marking the stream active must be rejected.
    assert!(
        test_ctx
            .cnx_server()
            .add_to_stream(test_stream, &[1, 2, 3, 4], true)
            .is_err(),
        "add_to_stream after reset should be forbidden on stream {test_stream}"
    );
    assert!(
        test_ctx
            .cnx_server()
            .mark_active_stream(test_stream, true, None)
            .is_err(),
        "mark_active_stream after reset should be forbidden on stream {test_stream}"
    );

    // Final loop: verify the connection settles within 2 seconds.
    let timeout2 = simulated_time.ticks() + 2_000_000;
    tls_api_wait_for_timeout(&mut test_ctx, &mut simulated_time, timeout2).expect("2s wait");
}
```

## `picoquictest/intformattest.c:varint_test`
* C test-table name: `varint`
* C entry function: `varint_test`
* Rust test: `varint`
* C source: `picoquictest/intformattest.c:249-343`
* Rust source: `rs/fq/src/tests/intformattest.rs:67-221`

### C test body
```c
{
    int ret = 0;
    uint8_t test_buf[16];
    const picoquic_varintformat_test_t* max_test = varint_test_cases + nb_varint_test_cases;
    memset(test_buf, 0xcc, 16);

    for (picoquic_varintformat_test_t* test = varint_test_cases; ret == 0 && test < max_test; test++) {
        for (int is_new_decode = 0; ret == 0 && is_new_decode <= 1; is_new_decode++) {
            for (size_t buf_size = 0; ret == 0 && buf_size <= test->length + 2 && buf_size < 16; buf_size++) {
                int test_ret = 0;
                uint64_t n64 = 0;
                size_t length;

                memcpy(test_buf, test->encoding, test->length);

                if (is_new_decode) {
                    const uint8_t* bytes = picoquic_frames_varint_decode(test_buf, test_buf + buf_size, &n64);
                    length = bytes != NULL ? bytes - test_buf : 0;
                }
                else {
                    length = picoquic_varint_decode(test_buf, buf_size, &n64);
                }

                if (length != (buf_size < test->length ? 0 : test->length)) {
                    DBG_PRINTF("Varint: unexpected length %u", (unsigned)length);
                    test_ret = -1;
                }
                else if (length == 0) {
                    continue;
                }
                else if (n64 != test->decoded) {
                    DBG_PRINTF("Varint: unexpected value %llu [expected %llu]",
                        (unsigned long long)n64, (unsigned long long)test->decoded);
                    test_ret = -1;
                }

                if (test_ret != 0) {
                    DBG_PRINTF(" (is_new=%d, test=%u, buf_size=%u/%u)\n",
                        is_new_decode, (unsigned)(max_test - test), (unsigned)buf_size, (unsigned)test->length);
                    ret = -1;
                }
            }
        }

        for (int is_new_encode = 0; ret == 0 && is_new_encode <= 1; is_new_encode++) {
            if (test->is_canonical != 0) {
                uint8_t encoding[8];
                size_t coded_length = 0;

                if (is_new_encode) {
                    uint8_t *bytes = picoquic_frames_varint_encode(encoding, &encoding[0] + sizeof(encoding), test->decoded);
                    if (bytes == NULL) {
                        coded_length = SIZE_MAX;
                    }
                    else {
                        coded_length = bytes - encoding;
                    }
                }
                else {
                    coded_length = picoquic_varint_encode(encoding, test->length, test->decoded);
                }

                if (coded_length != test->length) {
                    DBG_PRINTF("Varint, is_new=%d: unexpected coded_length=%"PRIst, is_new_encode, coded_length);
                    ret = -1;
                }
                else if (coded_length > sizeof(encoding)) {
                    DBG_PRINTF("Unexpected coded_length=%"PRIst", > sizeof(buffer)", coded_length);
                    ret = -1;
                }
                else if (memcmp(encoding, test->encoding, coded_length) != 0) {
                    DBG_PRINTF("Varint, is_new=%d: unexpected coded value", is_new_encode);
                    ret = -1;
                }
            }
        }
    }

    if (ret == 0) {
        /* test the length prediction */
        size_t picoquic_frames_varint_encode_length(uint64_t n64);

        for (picoquic_varintformat_test_t* test = varint_test_cases; ret == 0 && test < max_test; test++) {
            if (test->is_canonical) {
                size_t predicted_length = picoquic_frames_varint_encode_length(test->decoded);
                if (predicted_length != test->length) {
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
fn varint() {
    use crate::internal::{varint_decode, varint_encode};

    struct Case {
        encoding: [u8; 8],
        length: usize,
        decoded: u64,
        is_canonical: bool,
    }

    let cases = [
        Case {
            encoding: [0, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 1,
            decoded: 0,
            is_canonical: true,
        },
        Case {
            encoding: [1, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 1,
            decoded: 1,
            is_canonical: true,
        },
        Case {
            encoding: [63, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 1,
            decoded: 63,
            is_canonical: true,
        },
        Case {
            encoding: [0x40, 64, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 2,
            decoded: 64,
            is_canonical: true,
        },
        Case {
            encoding: [0x7F, 0xFF, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 2,
            decoded: 0x3FFF,
            is_canonical: true,
        },
        Case {
            encoding: [0x80, 0, 0x40, 0, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 4,
            decoded: 0x4000,
            is_canonical: true,
        },
        Case {
            encoding: [0xBF, 0xFF, 0xFF, 0xFF, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 4,
            decoded: 0x3FFF_FFFF,
            is_canonical: true,
        },
        Case {
            encoding: [0xC0, 0, 0, 0, 0x40, 0, 0, 0],
            length: 8,
            decoded: 0x4000_0000,
            is_canonical: true,
        },
        Case {
            encoding: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            length: 8,
            decoded: 0x3FFF_FFFF_FFFF_FFFF,
            is_canonical: true,
        },
        Case {
            encoding: [0xc2, 0x19, 0x7c, 0x5e, 0xff, 0x14, 0xe8, 0x8c],
            length: 8,
            decoded: 151_288_809_941_952_652,
            is_canonical: true,
        },
        Case {
            encoding: [0x9d, 0x7f, 0x3e, 0x7d, 0xff, 0x14, 0xe8, 0x8c],
            length: 4,
            decoded: 494_878_333,
            is_canonical: true,
        },
        Case {
            encoding: [0xC0, 0, 0, 0, 0x1d, 0x7f, 0x3e, 0x7d],
            length: 8,
            decoded: 494_878_333,
            is_canonical: false,
        },
        Case {
            encoding: [0x7b, 0xbd, 0x3e, 0x7d, 0xff, 0x14, 0xe8, 0x8c],
            length: 2,
            decoded: 15_293,
            is_canonical: true,
        },
        Case {
            encoding: [0x80, 0, 0x3b, 0xbd, 0x3e, 0x7d, 0xff, 0x14],
            length: 4,
            decoded: 15_293,
            is_canonical: false,
        },
        Case {
            encoding: [0xC0, 0, 0, 0, 0, 0, 0x3b, 0xbd],
            length: 8,
            decoded: 15_293,
            is_canonical: false,
        },
        Case {
            encoding: [0x25, 0xbd, 0x3e, 0x7d, 0xff, 0x14, 0xe8, 0x8c],
            length: 1,
            decoded: 37,
            is_canonical: true,
        },
        Case {
            encoding: [0x40, 0x25, 0xbd, 0x3e, 0x7d, 0xff, 0x14, 0xe8],
            length: 2,
            decoded: 37,
            is_canonical: false,
        },
    ];

    let mut test_buf = [0xCCu8; 16];

    for (idx, case) in cases.iter().enumerate() {
        // Decode: walk every prefix length up to length+2 and check
        // that under-reads return 0 while a sufficient buffer
        // returns the canonical length.
        for buf_size in 0..=(case.length + 2).min(15) {
            test_buf[..case.length].copy_from_slice(&case.encoding[..case.length]);
            let mut n64 = 0u64;
            let length = varint_decode(&test_buf[..buf_size], &mut n64);
            let expected_length = if buf_size < case.length {
                0
            } else {
                case.length
            };
            assert_eq!(
                length, expected_length,
                "case {idx}, buf_size={buf_size}: wrong length"
            );
            if length != 0 {
                assert_eq!(
                    n64, case.decoded,
                    "case {idx}, buf_size={buf_size}: wrong value"
                );
            }
        }

        // Encode: only canonical encodings round-trip.
        if case.is_canonical {
            let mut encoding = [0u8; 8];
            let coded = varint_encode(&mut encoding, case.decoded);
            assert_eq!(coded, case.length, "case {idx}: wrong encoded length");
            assert_eq!(
                &encoding[..coded],
                &case.encoding[..coded],
                "case {idx}: wrong bytes"
            );
        }
    }
}
```

## `picoquictest/mbedtls_test.c:mbedtls_crypto_test`
* C test-table name: `mbedtls_crypto`
* C entry function: `mbedtls_crypto_test`
* Rust test: `mbedtls_crypto`
* C source: `picoquictest/mbedtls_test.c:160-239`
* Rust source: `rs/fq/src/tests/mbedtls.rs:414-423`

### C test body
```c
{
    ptls_cipher_algorithm_t* cipher_test[5] = {
        &ptls_mbedtls_aes128ecb,
        &ptls_mbedtls_aes128ctr,
        &ptls_mbedtls_aes256ecb,
        &ptls_mbedtls_aes256ctr,
        &ptls_mbedtls_chacha20
    };
    ptls_cipher_algorithm_t* cipher_ref[5] = {
        &ptls_minicrypto_aes128ecb,
        &ptls_minicrypto_aes128ctr,
        &ptls_minicrypto_aes256ecb,
        &ptls_minicrypto_aes256ctr,
        &ptls_minicrypto_chacha20
    };
    int ret = 0;

    /* Initialize the PSA crypto library. */
    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        ret = test_random();
        DBG_PRINTF("test random returns: %d\n", ret);

        if (ret == 0) {
            ret = test_hash(&ptls_mbedtls_sha256, &ptls_minicrypto_sha256);
            DBG_PRINTF("test hash returns: %d\n", ret);
        }

        if (ret == 0) {
            ret = test_label(&ptls_mbedtls_sha256, &ptls_minicrypto_sha256);
            DBG_PRINTF("test label returns: %d\n", ret);
        }

        if (ret == 0) {
            for (int i = 0; i < 5; i++) {
                if (test_cipher(cipher_test[i], cipher_ref[i]) != 0) {
                    DBG_PRINTF("test cipher %d fails\n", i);
                    ret = -1;
                }
            }
            DBG_PRINTF("test ciphers returns: %d\n", ret);
        }

        if (ret == 0) {
            ret = test_aead(&ptls_mbedtls_aes128gcm, &ptls_mbedtls_sha256, &ptls_minicrypto_aes128gcm, &ptls_minicrypto_sha256);
            DBG_PRINTF("test aeads returns: %d\n", ret);
        }

        if (ret == 0) {
            ret = test_key_exchange(&ptls_mbedtls_secp256r1, &ptls_minicrypto_secp256r1);
            if (ret != 0) {
                DBG_PRINTF("%s", "test key exchange secp256r1 mbedtls to minicrypto fails\n");
            }
            else {
                ret = test_key_exchange(&ptls_minicrypto_secp256r1, &ptls_mbedtls_secp256r1);
                if (ret != 0) {
                    DBG_PRINTF("%s", "test key exchange secp256r1 minicrypto to mbedtls fails\n");
                }
            }
            ret = test_key_exchange(&ptls_mbedtls_x25519, &ptls_minicrypto_x25519);
            if (ret != 0) {
                DBG_PRINTF("%s", "test key exchange x25519 mbedtls to minicrypto fails\n");
            }
            else {
                ret = test_key_exchange(&ptls_minicrypto_x25519, &ptls_mbedtls_x25519);
                if (ret != 0) {
                    DBG_PRINTF("%s", "test key exchange x25519 minicrypto to mbedtls fails\n");
                }
            }
            DBG_PRINTF("test key exchange returns: %d\n", ret);
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }
    return (ret == 0) ? 0 : -1;
}
```

### Rust test body
```rust
fn mbedtls_crypto() {
    // Initialise the PSA/mbedTLS library.
    mbedtls_test_random().expect("test_random");
    mbedtls_test_hash().expect("test_hash");
    mbedtls_test_label().expect("test_label");
    mbedtls_test_ciphers().expect("test_ciphers");
    mbedtls_test_aead().expect("test_aead");
    mbedtls_test_key_exchange().expect("test_key_exchange");
    // PSA/mbedTLS library is de-initialised by the helper internals.
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
