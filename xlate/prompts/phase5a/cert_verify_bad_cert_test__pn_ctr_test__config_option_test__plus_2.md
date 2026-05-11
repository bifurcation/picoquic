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

## `picoquictest/cert_verify_test.c:cert_verify_bad_cert_test`
* C test-table name: `cert_verify_bad_cert`
* C entry function: `cert_verify_bad_cert_test`
* Rust test: `cert_verify_bad_cert`
* C source: `picoquictest/cert_verify_test.c:228-235`
* Rust source: `rs/fq/src/tests/cert_verify.rs:42-50`

### C test body
```c
{
    int ret = cert_verify_test_one(0, CERT_VERIFY_RSA_BAD_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Rust test body
```rust
fn cert_verify_bad_cert() {
    cert_verify_test_one(
        false,
        Some(TEST_FILE_SERVER_BAD_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_SNI),
    );
}
```

## `picoquictest/cleartext_aead_test.c:pn_ctr_test`
* C test-table name: `pn_ctr`
* C entry function: `pn_ctr_test`
* Rust test: `pn_ctr`
* C source: `picoquictest/cleartext_aead_test.c:302-399`
* Rust source: `rs/fq/src/tests/cleartext_aead.rs:271-374`

### C test body
```c
{
    int ret = 0;

    static const uint8_t key[] = {
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
        0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c };
    static const uint8_t iv[] = {
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a };
    static const uint8_t expected[] = { 
        0x3a, 0xd7, 0x7b, 0xb4, 0x0d, 0x7a, 0x36, 0x60,
        0xa8, 0x9e, 0xca, 0xf3, 0x24, 0x66, 0xef, 0x97 };
    static const uint8_t packet_clear_pn[] = {
        0x5D,
        0xba, 0xba, 0xc0, 0x01,
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
        0x20, 0x3f, 0xbe, 0x2e, 0x32, 0x17, 0xfc, 0x5b, 
        0x88, 0x55
    };
    static const uint8_t packet_encrypted_pn[] = {
        0x5d,
        0x80, 0x6d, 0xbb, 0xb5,
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
        0x20, 0x3f, 0xbe, 0x2e, 0x32, 0x17, 0xfc, 0x5b,
        0x88, 0x55
    };

    uint8_t in_bytes[16];
    uint8_t out_bytes[16];
    uint8_t decoded[16];

    picoquic_tls_api_init();

    ptls_aead_algorithm_t* aead = (ptls_aead_algorithm_t*)picoquic_get_aes128gcm_v(0);
    ptls_cipher_context_t *pn_enc = (aead == NULL)?NULL:ptls_cipher_new(aead->ctr_cipher, 1, key);

    if (pn_enc == NULL) {
        ret = -1;
    } else {
        /* test against expected value, from PTLS test */
        ptls_cipher_init(pn_enc, iv);
        memset(in_bytes, 0, 16);
        ptls_cipher_encrypt(pn_enc, out_bytes, in_bytes, sizeof(in_bytes));
        if (memcmp(out_bytes, expected, 16) != 0) {
            ret = -1;
        }

        /* test for various values of the PN length */

        for (size_t i = 1; ret == 0 && i <= 16; i *= 2) {
            memset(in_bytes, (int)i, i);
            ptls_cipher_init(pn_enc, iv);
            ptls_cipher_encrypt(pn_enc, out_bytes, in_bytes, i);
            for (size_t j = 0; j < i; j++) {
                if (in_bytes[j] != (out_bytes[j] ^ expected[j])) {
                    ret = -1;
                    break;
                }
            }
            ptls_cipher_init(pn_enc, iv);
            ptls_cipher_encrypt(pn_enc, decoded, out_bytes, i);
            if (memcmp(in_bytes, decoded, i) != 0) {
                ret = -1;
            }

            ptls_cipher_init(pn_enc, iv);
            ptls_cipher_encrypt(pn_enc, out_bytes, out_bytes, i);
            if (memcmp(in_bytes, out_bytes, i) != 0) {
                ret = -1;
            }
        }

        /* Test with the encrypted value from the packet */
        if (ret == 0)
        {
            ptls_cipher_init(pn_enc, packet_clear_pn + 5);
            ptls_cipher_encrypt(pn_enc, out_bytes, packet_clear_pn + 1, 4);
            if (memcmp(out_bytes, packet_encrypted_pn + 1, 4) != 0)
            {
                ret = -1;
            } else {
                ptls_cipher_init(pn_enc, packet_encrypted_pn + 5);
                ptls_cipher_encrypt(pn_enc, out_bytes, packet_encrypted_pn + 1, 4);
                if (memcmp(out_bytes, packet_clear_pn + 1, 4) != 0)
                {
                    ret = -1;
                }
            }
        }
        // cleanup
        ptls_cipher_free(pn_enc);
    }

    return ret;
}
```

### Rust test body
```rust
fn pn_ctr() {
    #[rustfmt::skip]
    const KEY: [u8; 16] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
        0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
    ];
    #[rustfmt::skip]
    const IV: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
    ];
    // AES-128-ECB(KEY, IV) — the expected CTR keystream for all-zeros plaintext.
    #[rustfmt::skip]
    const EXPECTED: [u8; 16] = [
        0x3a, 0xd7, 0x7b, 0xb4, 0x0d, 0x7a, 0x36, 0x60,
        0xa8, 0x9e, 0xca, 0xf3, 0x24, 0x66, 0xef, 0x97,
    ];
    // A packet whose first byte is flags, bytes 1..5 are the PN, bytes 5..21
    // are the CTR sample, and the rest is payload.
    #[rustfmt::skip]
    const PACKET_CLEAR_PN: [u8; 31] = [
        0x5d,
        0xba, 0xba, 0xc0, 0x01,
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
        0x20, 0x3f, 0xbe, 0x2e, 0x32, 0x17, 0xfc, 0x5b,
        0x88, 0x55,
    ];
    #[rustfmt::skip]
    const PACKET_ENCRYPTED_PN: [u8; 31] = [
        0x5d,
        0x80, 0x6d, 0xbb, 0xb5,
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
        0x20, 0x3f, 0xbe, 0x2e, 0x32, 0x17, 0xfc, 0x5b,
        0x88, 0x55,
    ];

    let cipher = test_pn_enc_from_raw_key(&KEY).expect("create CTR cipher");

    // Verify the AES-128-ECB keystream against the NIST test vector.
    let keystream = cipher.mask(IV);
    assert_eq!(
        keystream, EXPECTED,
        "AES-128 keystream does not match expected"
    );

    // For each prefix length i in {1, 2, 4, 8, 16}: encrypt [i; i] bytes,
    // verify the XOR relationship with the keystream, then round-trip.
    let mut i = 1usize;
    while i <= 16 {
        let in_bytes: Vec<u8> = vec![i as u8; i];
        let out_bytes: Vec<u8> = in_bytes
            .iter()
            .zip(EXPECTED.iter())
            .map(|(a, b)| a ^ b)
            .collect();

        for j in 0..i {
            assert_eq!(
                in_bytes[j],
                out_bytes[j] ^ EXPECTED[j],
                "CTR XOR property failed at i={i}, j={j}"
            );
        }

        // Re-encrypt the ciphertext to recover plaintext.
        let decoded: Vec<u8> = out_bytes
            .iter()
            .zip(EXPECTED.iter())
            .map(|(a, b)| a ^ b)
            .collect();
        assert_eq!(&decoded, &in_bytes, "CTR roundtrip failed at i={i}");

        i *= 2;
    }

    // Verify PN encryption against the test packet vectors.
    let sample_clear: [u8; 16] = PACKET_CLEAR_PN[5..21].try_into().unwrap();
    let enc_mask = cipher.mask(sample_clear);
    let encrypted_pn: Vec<u8> = PACKET_CLEAR_PN[1..5]
        .iter()
        .zip(enc_mask.iter())
        .map(|(a, b)| a ^ b)
        .collect();
    assert_eq!(
        &encrypted_pn,
        &PACKET_ENCRYPTED_PN[1..5],
        "PN encryption does not match expected"
    );

    let sample_enc: [u8; 16] = PACKET_ENCRYPTED_PN[5..21].try_into().unwrap();
    let dec_mask = cipher.mask(sample_enc);
    let decrypted_pn: Vec<u8> = PACKET_ENCRYPTED_PN[1..5]
        .iter()
        .zip(dec_mask.iter())
        .map(|(a, b)| a ^ b)
        .collect();
    assert_eq!(
        &decrypted_pn,
        &PACKET_CLEAR_PN[1..5],
        "PN decryption does not match expected"
    );
}
```

## `picoquictest/config_test.c:config_option_test`
* C test-table name: `config_option`
* C entry function: `config_option_test`
* Rust test: `config_option`
* C source: `picoquictest/config_test.c:622-653`
* Rust source: `rs/fq/src/tests/config.rs:450-468`

### C test body
```c
{
    int ret = config_parse_command_line_test(&param1, config_argv1, (int)(sizeof(config_argv1) / sizeof(char const*)) - 1);
    if (ret != 0) {
        DBG_PRINTF("First config option test returns %d", ret);
    }
    if (ret == 0) {
        ret = config_parse_command_line_test(&param2, config_argv2, (int)(sizeof(config_argv2) / sizeof(char const*)) - 1);

        if (ret != 0) {
            DBG_PRINTF("Second config option test returns %d", ret);
        }
    }

    if (ret == 0) {
        ret = config_test_parse_command_line_ex(&param2, config_two, (int)(sizeof(config_two) / sizeof(char const*)) - 1);
        if (ret != 0) {
            DBG_PRINTF("Two dash config option test returns %d", ret);
        }
    }

    for (size_t i = 0; ret == 0 && i < nb_config_errors; i++) {
        picoquic_quic_config_t config = { 0 };
        if (config_parse_command_line(&config, config_errors[i].err_args,
            config_errors[i].nb_args, 1) == 0) {
            DBG_PRINTF("Did not detect config error %zu, %s", i, config_errors[i].err_args[0]);
            ret = -1;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn config_option() {
    // ARGV1 → param1 expected values.
    assert_param1(&parse_argv(ARGV1).expect("parse ARGV1"));

    // ARGV2 → param2 expected values.
    assert_param2(&parse_argv(ARGV2).expect("parse ARGV2"));

    // Long-form CONFIG_TWO → same param2 expected values.
    assert_param2(&parse_argv_ex(CONFIG_TWO).expect("parse CONFIG_TWO"));

    // Every entry in ERROR_CASES must fail to parse.
    for (i, args) in ERROR_CASES.iter().enumerate() {
        assert!(
            parse_argv(args).is_err(),
            "Expected parse error for case {i}: {:?}",
            args[0],
        );
    }
}
```

## `picoquictest/congestion_test.c:bbr1_long_test`
* C test-table name: `bbr1_long`
* C entry function: `bbr1_long_test`
* Rust test: `bbr1_long`
* C source: `picoquictest/congestion_test.c:251-254`
* Rust source: `rs/fq/src/tests/congestion.rs:881-884`

### C test body
```c
{
    return congestion_long_test(picoquic_bbr1_algorithm);
}
```

### Rust test body
```rust
fn bbr1_long() {
    let ccalgo = get_congestion_algorithm("bbr1").expect("bbr1 cc algo");
    congestion_long_test(ccalgo);
}
```

## `picoquictest/congestion_test.c:bbr_slow_long_test`
* C test-table name: `bbr_slow_long`
* C entry function: `bbr_slow_long_test`
* Rust test: `bbr_slow_long`
* C source: `picoquictest/congestion_test.c:342-353`
* Rust source: `rs/fq/src/tests/congestion.rs:816-821`

### C test body
```c
{
    uint64_t max_completion_time = 81000000;
    uint64_t latency = 300000;
    uint64_t jitter = 3000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 1;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Rust test body
```rust
fn bbr_slow_long() {
    let latency = 300_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(81_000_000, 1, latency, jitter, buffer);
}
```
