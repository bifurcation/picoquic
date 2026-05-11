# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/mbedtls_test.c:mbedtls_sign_verify_test`
* C test-table name: `mbedtls_sign_verify`
* C entry function: `mbedtls_sign_verify_test`
* Rust test: `mbedtls_sign_verify`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:680-716`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers the same five fixture paths and names, but the helper is placeholder-like: it only checks PEM markers and compares a synthetic hash to itself, not real mbedTLS init, key loading, certificate verification, selected signature algorithm, or signature verification.
* Phase 5A fix note: Make the Rust helper exercise the real translated mbedTLS/picotls sign-and-verify path for all five fixtures, including init/free, key/cert/CA loading, server-name verification, signing the test message, and verifying the signature.
* Phase 5B analysis: Still blocked under the corrected Phase 5B standard: the Rust test is present and covers the five C fixture cases, but its helper only reads PEM fixtures and compares synthetic hashes. The crate exposes only registry-level mbedTLS stubs, with no mbedTLS signer, certificate verifier, server-name verification, or signature-verification callback surface needed to express the C API-level contract. This is an API-surface gap, not a Phase 5C runtime failure.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = 0;

    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_RSA_KEY, ASSET_RSA_CERT, ASSET_TEST_CA, ASSET_RSA_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP256R1_KEY, ASSET_SECP256R1_CERT, ASSET_TEST_CA, ASSET_SECP256R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP384R1_KEY, ASSET_SECP384R1_CERT, ASSET_TEST_CA, ASSET_SECP384R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP521R1_KEY, ASSET_SECP521R1_CERT, ASSET_TEST_CA, ASSET_SECP521R1_NAME, 0, 0);
        }

        if (ret == 0) {
            ret = test_sign_verify_one(ASSET_SECP256R1_PKCS8_KEY, ASSET_SECP256R1_PKCS8_CERT, ASSET_TEST_CA, ASSET_SECP256R1_PKCS8_NAME, 0, 0);
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }
    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls_sign_verify() {
    mbedtls_test_sign_verify_one(
        "certs/rsa/key.pem",
        "certs/rsa/cert.pem",
        "certs/test-ca.crt",
        "rsa.test.example.com",
    )
    .expect("rsa sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1/key.pem",
        "certs/secp256r1/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp384r1/key.pem",
        "certs/secp384r1/cert.pem",
        "certs/test-ca.crt",
        "secp384r1.test.example.com",
    )
    .expect("secp384r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp521r1/key.pem",
        "certs/secp521r1/cert.pem",
        "certs/test-ca.crt",
        "secp521r1.test.example.com",
    )
    .expect("secp521r1 sign_verify");
    mbedtls_test_sign_verify_one(
        "certs/secp256r1-pkcs8/key.pem",
        "certs/secp256r1-pkcs8/cert.pem",
        "certs/test-ca.crt",
        "test.example.com",
    )
    .expect("secp256r1-pkcs8 sign_verify");
}
```

## `picoquictest/cleartext_aead_test.c:pn_ctr_test`
* C test-table name: `pn_ctr`
* C entry function: `pn_ctr_test`
* Rust test: `pn_ctr`
* Expected Rust file: `rs/fq/src/tests/cleartext_aead.rs`
* Current Rust span: `rs/fq/src/tests/cleartext_aead.rs:271-374`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Known-answer mask and packet vectors match, but the variable-length loop derives ciphertext from EXPECTED instead of exercising the Rust PN helper for lengths 1,2,4,8,16, and omits the C in-place check.
* Phase 5A fix note: Drive pn_encrypt/HeaderKey output for each tested length, verify prefix masks and round-trip XOR, and add an in-place-style second XOR check.
* Phase 5B analysis: Rust test now exercises the PN helper for lengths 1,2,4,8,16 and covers the C in-place round-trip behavior.
* Phase 5B fix note: Imported pn_encrypt and updated pn_ctr's variable-length loop to derive mask prefixes through pn_encrypt, verify them against EXPECTED, check XOR round-trip, and add an in-place-style second XOR check.

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

### Current Rust test body
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

## `picoquictest/intformattest.c:intformattest`
* C test-table name: `intformat`
* C entry function: `intformattest`
* Rust test: `intformat`
* Expected Rust file: `rs/fq/src/tests/intformattest.rs`
* Current Rust span: `rs/fq/src/tests/intformattest.rs:36-99`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test covers the old format/parse round trips and test numbers, but it omits the C second pass over picoquic_frames_uint*_encode and the exact encoded-length checks.
* Phase 5A fix note: Add a second pass using frames_uint16_encode, frames_uint24_encode, frames_uint32_encode, and frames_uint64_encode; assert 2/3/4/8 byte advances and then run the same decode and parse checks.
* Phase 5B analysis: Rust now covers both C encoding passes and checks exact frame-encoder advance lengths before decode/parse assertions.
* Phase 5B fix note: Added frames_uint16/24/32/64 encoder pass to intformat and asserted 2/3/4/8 byte advances.

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

### Current Rust test body
```rust
fn intformat() {
    let mut buf = [0u8; 8];

    for new_encoding in 0..2 {
        for &n in TEST_NUMBERS {
            let n16 = n as u16;
            if new_encoding == 0 {
                format_16(&mut buf, n16);
            } else {
                let advance = {
                    let rest = frames_uint16_encode(&mut buf, n16).expect("u16 encode fits");
                    8 - rest.len()
                };
                assert_eq!(advance, 2, "u16 encoder advanced wrong length");
            }
            assert_eq!(decode_number(&buf, 2), n16 as u64, "u16 BE bytes mismatch");
            assert_eq!(parse_16(&buf), n16, "parse_16 roundtrip");
        }

        for &n in TEST_NUMBERS {
            let n24 = (n & 0xFF_FFFF) as u32;
            if new_encoding == 0 {
                format_24(&mut buf, n24);
            } else {
                let advance = {
                    let rest = frames_uint24_encode(&mut buf, n24).expect("u24 encode fits");
                    8 - rest.len()
                };
                assert_eq!(advance, 3, "u24 encoder advanced wrong length");
            }
            assert_eq!(decode_number(&buf, 3), n24 as u64, "u24 BE bytes mismatch");
            assert_eq!(parse_24(&buf), n24, "parse_24 roundtrip");
        }

        for &n in TEST_NUMBERS {
            let n32 = n as u32;
            if new_encoding == 0 {
                format_32(&mut buf, n32);
            } else {
                let advance = {
                    let rest = frames_uint32_encode(&mut buf, n32).expect("u32 encode fits");
                    8 - rest.len()
                };
                assert_eq!(advance, 4, "u32 encoder advanced wrong length");
            }
            assert_eq!(decode_number(&buf, 4), n32 as u64, "u32 BE bytes mismatch");
            assert_eq!(parse_32(&buf), n32, "parse_32 roundtrip");
        }

        for &n in TEST_NUMBERS {
            if new_encoding == 0 {
                format_64(&mut buf, n);
            } else {
                let advance = {
                    let rest = frames_uint64_encode(&mut buf, n).expect("u64 encode fits");
                    8 - rest.len()
                };
                assert_eq!(advance, 8, "u64 encoder advanced wrong length");
            }
            assert_eq!(decode_number(&buf, 8), n, "u64 BE bytes mismatch");
            assert_eq!(parse_64(&buf), n, "parse_64 roundtrip");
        }
    }
}
```

## `picoquictest/openssl_test.c:openssl_cert_test`
* C test-table name: `openssl_cert`
* C entry function: `openssl_cert_test`
* Rust test: `openssl_cert`
* Expected Rust file: `rs/fq/src/tests/openssl.rs`
* Current Rust span: `rs/fq/src/tests/openssl.rs:9-9`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The mapped C source is the PTLS_WITHOUT_OPENSSL no-op branch, but the Rust test actively parses three certificate fixtures and asserts counts, so it checks materially different behavior for this mapped entry.
* Phase 5A fix note: Make the Rust test match the no-OpenSSL mapped C branch, or gate/remap the certificate-count checks to an OpenSSL-enabled C branch explicitly.
* Phase 5B analysis: Rust test now matches the mapped no-OpenSSL C branch, which performs no certificate-loading work and returns success.
* Phase 5B fix note: Removed the certificate parsing helper/import and made openssl_cert a no-op test with updated module documentation.

### C test body
```c
{
    /* Nothing to do, as the module is not loaded. */
    return 0;
}
```

### Current Rust test body
```rust
fn openssl_cert() {}
```

## `picoquictest/skip_frame_test.c:frames_ackack_error_test`
* C test-table name: `frames_ackack_error`
* C entry function: `frames_ackack_error_test`
* Rust test: `frames_ackack_error`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:3755-3798`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C sweeps every `test_skip_list` entry and every varint index for each entry, while Rust exercises only one PING sample and ignores the result, making it placeholder-like coverage.
* Phase 5A fix note: Iterate the translated skip-frame table and all varint positions `1..=nb_varints`, using each case's epoch and multipath flag; preserve the C trial/disconnection accumulation or equivalent assertions/no-panic coverage.
* Phase 5B analysis: Rust now sweeps the translated skip-frame table and each varint index like the C test.
* Phase 5B fix note: Updated ack-ack helper to use table epoch/mpath setup and return Result<bool>; updated test to count all trials/disconnections across every varint-bearing frame.

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    picoquic_quic_t* qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    struct sockaddr_in saddr = { 0 };
    picoquic_packet_t p;
    int nb_trials = 0;
    int nb_disconnected = 0;

    if (qclient == NULL) {
        ret = -1;
    }
    else {
        for (size_t i = 0; ret == 0 && i < nb_test_skip_list; i++) {
            for (int v = 1; v <= test_skip_list[i].nb_varints; v++) {
                int disconnected = 0;
                
                frame_ackack_error_packet(qclient, (struct sockaddr*)&saddr, simulated_time, &p, i, v,
                        test_skip_list[i].epoch, test_skip_list[i].mpath, &disconnected);
                nb_trials++;
                nb_disconnected += disconnected;
            }
        }
        picoquic_free(qclient);
    }
    DBG_PRINTF("%d ackack trials, %d disconnections", nb_trials, nb_disconnected);

    return ret;
}
```

### Current Rust test body
```rust
fn frames_ackack_error() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let frames = test_skip_frames();

    assert_eq!(frames.len(), TEST_SKIP_FRAME_VARINT_COUNTS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_EPOCHS.len());
    assert_eq!(frames.len(), TEST_SKIP_FRAME_MPATH.len());

    let mut nb_trials = 0usize;
    let mut nb_disconnected = 0usize;
    for (i, case) in frames.iter().enumerate() {
        for varint_idx in 1..=TEST_SKIP_FRAME_VARINT_COUNTS[i] {
            let disconnected = frame_ackack_error_packet(
                &mut quic,
                &case.bytes,
                TEST_SKIP_FRAME_EPOCHS[i],
                TEST_SKIP_FRAME_MPATH[i],
                varint_idx,
            )
            .unwrap_or_else(|err| {
                panic!(
                    "ack-ack frame <{}> varint {} failed: {:?}",
                    case.name, varint_idx, err
                )
            });
            nb_trials += 1;
            nb_disconnected += usize::from(disconnected);
        }
    }

    assert_eq!(
        nb_trials,
        TEST_SKIP_FRAME_VARINT_COUNTS
            .iter()
            .copied()
            .map(usize::try_from)
            .collect::<Result<Vec<_>, _>>()
            .expect("varint count conversion")
            .into_iter()
            .sum::<usize>()
    );
    assert!(nb_disconnected <= nb_trials);
}
```
