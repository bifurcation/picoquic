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

## `picoquictest/ack_frequency_test.c:ackfrq_short_test`
* C test-table name: `ackfrq_short`
* C entry function: `ackfrq_short_test`
* Rust test: `ackfrq_short`
* C source: `picoquictest/ack_frequency_test.c:194-208`
* Rust source: `rs/fq/src/tests/ack_frequency.rs:195-208`

### C test body
```c
{
    ackfrq_test_spec_t spec = { 0 };
    spec.test_id = ackfrq_test_basic;
    spec.latency = 10;
    spec.picosec_per_byte_up = 80000;
    spec.picosec_per_byte_down = 80000;
    spec.ccalgo = picoquic_cubic_algorithm;
    spec.max_ack_delay_remote = 1000;
    spec.max_ack_gap_remote = 32;
    spec.min_ack_delay_remote = 1000;
    spec.target_interval = 1000;

    return ackfrq_test_one(&spec);
}
```

### Rust test body
```rust
fn ackfrq_short() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    ackfrq_test_one(&AckfrqTestSpec {
        latency: 10,
        picosec_per_byte_up: 80_000,
        picosec_per_byte_down: 80_000,
        ccalgo,
        target_time: 0,
        max_ack_delay_remote: Duration::from_ticks(1_000),
        max_ack_gap_remote: 32,
        min_ack_delay_remote: Duration::from_ticks(1_000),
        target_interval: Duration::from_ticks(1_000),
    });
}
```

## `picoquictest/cert_verify_test.c:cert_verify_bad_sni_test`
* C test-table name: `cert_verify_bad_sni`
* C entry function: `cert_verify_bad_sni_test`
* Rust test: `cert_verify_bad_sni`
* C source: `picoquictest/cert_verify_test.c:237-245`
* Rust source: `rs/fq/src/tests/cert_verify.rs:54-62`

### C test body
```c
{
    int ret = cert_verify_test_one(0, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_BAD_SNI);
    return ret;
}
```

### Rust test body
```rust
fn cert_verify_bad_sni() {
    cert_verify_test_one(
        false,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_BAD_SNI),
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

## `picoquictest/code_version_test.c:code_version_test`
* C test-table name: `code_version`
* C entry function: `code_version_test`
* Rust test: `code_version`
* C source: `picoquictest/code_version_test.c:48-97`
* Rust source: `rs/fq/src/tests/code_version.rs:9-48`

### C test body
```c
{
    char cmake_file[512];
    int ret = picoquic_get_input_path(cmake_file, sizeof(cmake_file), picoquic_solution_dir, "CMakeLists.txt");

    if (ret >= 0) {
        int last_err = 0;
        FILE* F = picoquic_file_open_ex(cmake_file, "r", &last_err);

        ret = -1; /* will be set to zero if successful */

        if (F == NULL) {
            DBG_PRINTF("Cannot open <%s> error %d(0x%x)", cmake_file, last_err, last_err);
        }
        else {
            char line[512];
            char const* project = "project(picoquic";
            size_t project_len = strlen(project);
            char const* version = "VERSION";
            size_t version_len = strlen(version);

            size_t offset;

            while (fgets(line, sizeof(line), F) != NULL) {
                /* find line that include "project(picoquic" */
                if (starts_with(line, project, project_len, &offset) == 0) {
                    /* get next line, which should include <space>VERSION<space>x.y.z.t */
                    if (fgets(line, sizeof(line), F) != NULL &&
                        starts_with(line, version, version_len, &offset) == 0) {
                        char* x = skip_blank(&line[offset + version_len]);
                        size_t x_len = strlen(x);
                        const char* v = PICOQUIC_VERSION;
                        size_t v_len = strlen(v);
                        if (x_len >= v_len && memcmp(x, v, v_len) == 0) {
                            char* y = x + v_len;
                            if (*y == ' ' || *y == '\t' || *y == '\r' || *y == '\n') {
                                ret = 0;
                            }
                        }
                    }
                    /* Extract the version string. */
                    /* compare to version in picoquic.h */
                    break;
                }
            }
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn code_version() {
    use std::io::BufRead;

    let cmake_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../CMakeLists.txt");
    let file = std::fs::File::open(cmake_path)
        .unwrap_or_else(|e| panic!("cannot open CMakeLists.txt: {e}"));

    let mut lines = std::io::BufReader::new(file).lines();
    let mut found = false;

    while let Some(Ok(line)) = lines.next() {
        if line.trim_start().starts_with("project(picoquic") {
            // Next line: <whitespace>VERSION<whitespace>x.y.z.t
            if let Some(Ok(next)) = lines.next() {
                let trimmed = next.trim_start();
                if let Some(after_version_kw) = trimmed.strip_prefix("VERSION") {
                    let cmake_ver = after_version_kw.trim_start();
                    let rest = cmake_ver.strip_prefix(crate::VERSION).unwrap_or_else(|| {
                        panic!(
                            "CMakeLists.txt VERSION {cmake_ver:?} does not match crate::VERSION {:?}",
                            crate::VERSION
                        )
                    });
                    assert!(
                        rest.is_empty() || rest.starts_with(char::is_whitespace),
                        "CMakeLists.txt VERSION {cmake_ver:?} has unexpected suffix after {:?}",
                        crate::VERSION
                    );
                    found = true;
                }
            }
            break;
        }
    }

    assert!(
        found,
        "could not find `project(picoquic` VERSION line in CMakeLists.txt"
    );
}
```

## `picoquictest/congestion_test.c:bbr_test`
* C test-table name: `bbr`
* C entry function: `bbr_test`
* Rust test: `bbr`
* C source: `picoquictest/congestion_test.c:140-143`
* Rust source: `rs/fq/src/tests/congestion.rs:775-778`

### C test body
```c
{
    return congestion_control_test(picoquic_bbr_algorithm, 3500000, 0, 0);
}
```

### Rust test body
```rust
fn bbr() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    congestion_control_test(ccalgo, 3_500_000, 0, 0);
}
```
