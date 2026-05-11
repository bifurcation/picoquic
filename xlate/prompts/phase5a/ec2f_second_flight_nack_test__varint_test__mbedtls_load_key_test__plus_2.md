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

## `picoquictest/mbedtls_test.c:mbedtls_load_key_test`
* C test-table name: `mbedtls_load_key`
* C entry function: `mbedtls_load_key_test`
* Rust test: `mbedtls_load_key`
* C source: `picoquictest/mbedtls_test.c:691-735`
* Rust source: `rs/fq/src/tests/mbedtls.rs:428-435`

### C test body
```c
{
    int ret = 0;


    /* Initialize the PSA crypto library. */
    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_RSA_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP256R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP384R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP521R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP256R1_PKCS8_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_RSA_PKCS8_KEY);
        }
#if 0
        /* Commenting out ED25519 for now, probably not supported yet in MBEDTLS/PSA */
        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_ED25519_KEY);
        }
#endif
        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }

    return ret;
}
```

### Rust test body
```rust
fn mbedtls_load_key() {
    mbedtls_test_load_one_der_key("certs/rsa/key.pem").expect("rsa key");
    mbedtls_test_load_one_der_key("certs/secp256r1/key.pem").expect("secp256r1 key");
    mbedtls_test_load_one_der_key("certs/secp384r1/key.pem").expect("secp384r1 key");
    mbedtls_test_load_one_der_key("certs/secp521r1/key.pem").expect("secp521r1 key");
    mbedtls_test_load_one_der_key("certs/secp256r1-pkcs8/key.pem").expect("secp256r1 pkcs8 key");
    mbedtls_test_load_one_der_key("certs/rsa-pkcs8/key.pem").expect("rsa pkcs8 key");
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

## `picoquictest/minicrypto_test.c:minicrypto_is_last_test`
* C test-table name: `minicrypto_is_last`
* C entry function: `minicrypto_is_last_test`
* Rust test: `minicrypto_is_last`
* C source: `picoquictest/minicrypto_test.c:115-162`
* Rust source: `rs/fq/src/tests/minicrypto.rs:74-105`

### C test body
```c
{
    int ret = 0;
    int expected_aes128gcm_sha256 = 1;
    int expected_aes128gcm_sha256_low = 1;
    int expected_set_key = 1;
    int using_aes128gcm_sha256;
    int using_aes128gcm_sha256_low;
    int actual_set_key;
    void* actual_aes128gcm_sha256 = picoquic_get_aes128gcm_sha256_v(0);
    void* actual_aes128gcm_sha256_low = picoquic_get_aes128gcm_sha256_v(1);

    picoquic_tls_api_reset(0);

#if defined(PICOQUIC_WITH_MBEDTLS) || !defined(PTLS_WITHOUT_OPENSSL) || !defined(PTLS_WITHOUT_FUSION)
    expected_aes128gcm_sha256 = 0;
    expected_set_key = 0;
#endif
#if defined(PICOQUIC_WITH_MBEDTLS) || !defined(PTLS_WITHOUT_OPENSSL)
    expected_aes128gcm_sha256_low = 0;
#endif
#if !defined(PTLS_WITHOUT_OPENSSL)
    expected_set_key = 0;
#endif
    using_aes128gcm_sha256 = (actual_aes128gcm_sha256 == (void*)&ptls_minicrypto_aes128gcmsha256);
    using_aes128gcm_sha256_low = (actual_aes128gcm_sha256_low == (void*)&ptls_minicrypto_aes128gcmsha256);
    actual_set_key = (picoquic_set_private_key_from_file_fn == picoquic_minicrypto_set_key_fn);
    if (using_aes128gcm_sha256 != expected_aes128gcm_sha256) {
        DBG_PRINTF("Wrong aes gcm 128 sha 256. Expected: %s, actual: %s",
            (expected_aes128gcm_sha256) ? "minicrypto" : "other",
            (using_aes128gcm_sha256) ? "minicrypto" : "other");
        ret = -1;
    }
    if (using_aes128gcm_sha256_low != expected_aes128gcm_sha256_low) {
        DBG_PRINTF("Wrong aes gcm 128 sha 256 low. Expected: %s, actual: %s",
            (expected_aes128gcm_sha256_low) ? "minicrypto" : "other",
            (using_aes128gcm_sha256_low) ? "minicrypto" : "other");
        ret = -1;
    }
    if (actual_set_key != expected_set_key) {
        DBG_PRINTF("Wrong set key function. Expected: %s, actual: %s",
            (expected_set_key) ? "minicrypto" : "other",
            (actual_set_key) ? "minicrypto" : "other");
        ret = -1;
    }

    return ret;
}
```

### Rust test body
```rust
fn minicrypto_is_last() {
    reset_tls_api(0);

    let using_high = is_minicrypto_aes128gcm_sha256(false);
    let using_low = is_minicrypto_aes128gcm_sha256(true);
    let using_key = is_minicrypto_key_loader();

    let expected_high = !cfg!(any(
        feature = "sys-mbedtls",
        feature = "sys-openssl",
        feature = "sys-fusion",
    ));
    let expected_low = !cfg!(any(feature = "sys-mbedtls", feature = "sys-openssl"));
    let expected_key = !cfg!(any(
        feature = "sys-mbedtls",
        feature = "sys-openssl",
        feature = "sys-fusion",
    ));

    assert_eq!(
        using_high, expected_high,
        "aes128gcm_sha256 (high-memory): expected minicrypto={expected_high}, got {using_high}",
    );
    assert_eq!(
        using_low, expected_low,
        "aes128gcm_sha256 (low-memory): expected minicrypto={expected_low}, got {using_low}",
    );
    assert_eq!(
        using_key, expected_key,
        "key loader: expected minicrypto={expected_key}, got {using_key}",
    );
}
```
