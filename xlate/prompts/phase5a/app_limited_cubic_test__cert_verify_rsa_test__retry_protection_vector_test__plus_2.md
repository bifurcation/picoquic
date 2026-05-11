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

## `picoquictest/app_limited.c:app_limited_cubic_test`
* C test-table name: `app_limited_cubic`
* C entry function: `app_limited_cubic_test`
* Rust test: `app_limited_cubic`
* C source: `picoquictest/app_limited.c:589-598`
* Rust source: `rs/fq/src/tests/app_limited.rs:511-517`

### C test body
```c
{
    app_limited_test_config_t config;
    app_limited_config_set_default(&config, 2);
    config.ccalgo = picoquic_cubic_algorithm;
    config.nb_losses_max = 64;
    config.data_rate_max = 4013000;

    return app_limited_test_one(&config);
}
```

### Rust test body
```rust
fn app_limited_cubic() {
    let mut config = AppLimitedConfig::default_config(2);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    config.nb_losses_max = 64;
    config.data_rate_max = 4_013_000;
    app_limited_test_one(config);
}
```

## `picoquictest/cert_verify_test.c:cert_verify_rsa_test`
* C test-table name: `cert_verify_rsa`
* C entry function: `cert_verify_rsa_test`
* Rust test: `cert_verify_rsa`
* C source: `picoquictest/cert_verify_test.c:219-226`
* Rust source: `rs/fq/src/tests/cert_verify.rs:93-101`

### C test body
```c
{
    int ret = cert_verify_test_one(1, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Rust test body
```rust
fn cert_verify_rsa() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_SNI),
    );
}
```

## `picoquictest/cleartext_aead_test.c:retry_protection_vector_test`
* C test-table name: `retry_protection_vector`
* C entry function: `retry_protection_vector_test`
* Rust test: `retry_protection_vector`
* C source: `picoquictest/cleartext_aead_test.c:1173-1314`
* Rust source: `rs/fq/src/tests/cleartext_aead.rs:657-815`

### C test body
```c
{
    /* First, create a protection context to test the basic mechanisms */
    int ret = 0;
    void* protection_ctx = picoquic_create_retry_protection_context(1, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);

    if (protection_ctx == NULL) {
        DBG_PRINTF("%s", "Cannot create protection context!");
        ret = -1;
    }
    else if (0 != cleartext_iv_cmp(protection_ctx, retry_protection_test_iv, sizeof(retry_protection_test_iv))) {
        DBG_PRINTF("%s", "Clear protection IV does not match expected value.\n");
            ret = -1;
    } 
    else {
        uint8_t encoded[256];
        size_t encoded_length = picoquic_aead_encrypt_generic(encoded, encoded, 0, 0, retry_protection_pseudo_packet, sizeof(retry_protection_pseudo_packet), protection_ctx);

        if (encoded_length != 16) {
            DBG_PRINTF("Encoded length = %d instead of 16", (int)encoded_length);
            ret = -1;
        }
        else if (memcmp(encoded, retry_protection_test_checksum, 16) != 0) {
            DBG_PRINTF("%s", "Test vector does not match!");
            ret = -1;
        }

        picoquic_aead_free(protection_ctx);

        if (ret == 0) {
            void* verification_ctx = picoquic_create_retry_protection_context(0, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
            if (verification_ctx == NULL) {
                DBG_PRINTF("%s", "Cannot create verification context!");
                ret = -1;
            }
            else if (0 != cleartext_iv_cmp(verification_ctx, retry_protection_test_iv, sizeof(retry_protection_test_iv))) {
                DBG_PRINTF("%s", "Clear verification IV does not match expected value.\n");
                    ret = -1;
            }
            else {
                uint8_t decoded[256];
                size_t decoded_length = picoquic_aead_decrypt_generic(decoded, encoded, encoded_length, 0, retry_protection_pseudo_packet, sizeof(retry_protection_pseudo_packet), verification_ctx);

                if (decoded_length != 0) {
                    DBG_PRINTF("Decoded length = %d instead of 0", (int)decoded_length);
                    ret = -1;
                }
                else {
                    /* Positive test succeeded, now do a negative test */
                    encoded[0] ^= 1;
                    decoded_length = picoquic_aead_decrypt_generic(decoded, encoded, encoded_length, 0, retry_protection_pseudo_packet, sizeof(retry_protection_pseudo_packet), verification_ctx);
                    if (decoded_length == 0) {
                        DBG_PRINTF("Decoded length = 0 instead of expected error", (int)decoded_length);
                        ret = -1;
                    }
                }

                picoquic_aead_free(verification_ctx);
            }
        }
    }

    if (ret == 0) {
        /* Test the verification functions */
        void* protection_ctx = picoquic_create_retry_protection_context(1, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
        uint8_t packet[PICOQUIC_MAX_PACKET_SIZE];
        size_t packet_index = sizeof(retry_protection_test_input);

        if (protection_ctx == NULL) {
            DBG_PRINTF("%s", "Cannot create protection context!");
            ret = -1;
        }
        else {
            size_t length;
            memcpy(packet, retry_protection_test_input, packet_index);

            length = picoquic_encode_retry_protection(protection_ctx, packet, PICOQUIC_MAX_PACKET_SIZE, packet_index, &retry_protection_test_odcid);

            if (length != packet_index + sizeof(retry_protection_test_checksum)) {
                DBG_PRINTF("Packet length = %d instead of %d+16", (int)length, (int)packet_index);
                ret = -1;
            }
            else if (memcmp(packet + packet_index, retry_protection_test_checksum, sizeof(retry_protection_test_checksum)) != 0) {
                DBG_PRINTF("%s", "Packet checksum does not match!");
                ret = -1;
            }
            
            picoquic_aead_free(protection_ctx);

            if (ret == 0) {
                void* verification_ctx = picoquic_create_retry_protection_context(0, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
                if (verification_ctx == NULL) {
                    DBG_PRINTF("%s", "Cannot create verification context!");
                    ret = -1;
                }
                else {
                    size_t data_length = length;
                    size_t bytes_index = sizeof(retry_protection_test_input) - RETRY_PROTECTION_TEST_RETRY_TOKEN_LENGTH;

                    ret = picoquic_verify_retry_protection(verification_ctx, packet, &data_length, bytes_index, &retry_protection_test_odcid);

                    if (ret != 0) {
                        DBG_PRINTF("Verification returns %d (0x%d)!", ret, ret);
                    }
                    else if (data_length != sizeof(retry_protection_test_input)) {
                        DBG_PRINTF("Verification returns length %d instead of %d!", (int)data_length, (int)sizeof(retry_protection_test_input));
                        ret = -1;
                    }

                    if (ret == 0) {
                        /* Try verification with a different odcid. It should fail */
                        picoquic_connection_id_t bad_odcid = retry_protection_test_odcid;
                        bad_odcid.id[0] ^= 1;
                        data_length = length;
                        if (picoquic_verify_retry_protection(verification_ctx, packet, &data_length, bytes_index, &bad_odcid) == 0) {
                            DBG_PRINTF("%s", "Bad odcid not detected!");
                            ret = -1;
                        }
                    }


                    if (ret == 0) {
                        /* Verify that the draft 25 vector passes */
                        data_length = sizeof(retry_protection_packet_draft25);
                        memcpy(packet, retry_protection_packet_draft25, data_length);
                        bytes_index = data_length - RETRY_PROTECTION_TEST_RETRY_TOKEN_LENGTH;

                        ret = picoquic_verify_retry_protection(verification_ctx, packet, &data_length, bytes_index, &retry_protection_odcid_draft25);

                        if (ret != 0) {
                            DBG_PRINTF("Testing vector in draft 25 returns %d (0x%x)!", ret, ret);
                        }
                    }

                    picoquic_aead_free(verification_ctx);
                }
            }
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn retry_protection_vector() {
    // Retry pseudo-packet constants.
    const RETRY_TOKEN_LENGTH: usize = 24;

    #[rustfmt::skip]
    let retry_protection_test_input: [u8; 41] = [
        // FIRST_BYTE
        0xF5,
        // VERSION = 0xFF000019
        0xFF, 0x00, 0x00, 0x19,
        // DCID_LENGTH = 6, DCID_BYTES = 61..66
        6, 61, 62, 63, 64, 65, 66,
        // SCID_LENGTH = 4, SCID_BYTES = 44..47
        4, 44, 45, 46, 47,
        // RETRY_TOKEN (24 bytes) = 101..124
        101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
        113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124,
    ];

    let retry_protection_test_odcid =
        ConnectionId::clone_from_slice(&[81, 82, 83, 84, 85, 86, 87, 88]).unwrap();

    // Pseudo-packet = ODCID_LENGTH | ODCID_BYTES | retry_protection_test_input.
    let retry_protection_pseudo_packet: Vec<u8> = {
        let mut v = Vec::with_capacity(1 + 8 + retry_protection_test_input.len());
        v.push(8u8); // ODCID length
        v.extend_from_slice(&[81, 82, 83, 84, 85, 86, 87, 88]);
        v.extend_from_slice(&retry_protection_test_input);
        v
    };

    #[rustfmt::skip]
    let retry_protection_test_checksum: [u8; 16] = [
        0xf9, 0x50, 0xf8, 0x85, 0x71, 0x4b, 0xae, 0x7a,
        0xf1, 0xe2, 0x86, 0x7d, 0xd8, 0xf7, 0x83, 0x92,
    ];

    // Draft-25 retry packet vector.
    let retry_protection_odcid_draft25 =
        ConnectionId::clone_from_slice(&[0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08]).unwrap();
    #[rustfmt::skip]
    let retry_protection_packet_draft25: [u8; 36] = [
        0xff, 0xff, 0x00, 0x00, 0x19, 0x00, 0x08, 0xf0, 0x67, 0xa5, 0x50, 0x2a, 0x42, 0x62,
        0xb5, 0x74, 0x6f, 0x6b, 0x65, 0x6e, 0x1e, 0x5e, 0xc5, 0xb0, 0x14, 0xcb, 0xb1, 0xf0,
        0xfd, 0x93, 0xdf, 0x40, 0x48, 0xc4, 0x46, 0xa6,
    ];

    // Obtain the QUIC-v1 retry integrity key via the version parameters.
    let v1_params = Version::V1.parameters();
    let retry_key = v1_params.version_retry_key;
    let prefix_label = v1_params.tls_prefix_label;

    // Phase 1: low-level AEAD encrypt/decrypt against the known checksum.
    {
        let protection_ctx = create_retry_protection_context(true, retry_key, prefix_label)
            .expect("create protection ctx");

        // Encrypt empty plaintext; the only output is the 16-byte AEAD tag.
        let mut tag: Vec<u8> = Vec::new();
        protection_ctx.encrypt(0, &retry_protection_pseudo_packet, &mut tag);
        assert_eq!(tag.len(), 16, "tag length != 16");
        assert_eq!(
            &tag[..],
            &retry_protection_test_checksum,
            "tag does not match expected"
        );

        // Verify: decrypt the tag (should succeed with 0 plaintext bytes).
        let verification_ctx = create_retry_protection_context(false, retry_key, prefix_label)
            .expect("create verification ctx");

        let mut tag_verify = tag.clone();
        verification_ctx
            .decrypt(0, &retry_protection_pseudo_packet, &mut tag_verify)
            .expect("AEAD decrypt should succeed");
        assert!(tag_verify.is_empty(), "decrypted plaintext should be empty");

        // Negative test: corrupt one byte → decryption must fail.
        tag_verify = tag.clone();
        tag_verify[0] ^= 1;
        assert!(
            verification_ctx
                .decrypt(0, &retry_protection_pseudo_packet, &mut tag_verify)
                .is_err(),
            "corrupted tag should fail verification"
        );
    }

    // Phase 2: encode_retry_protection / verify_retry_protection.
    {
        let protection_ctx = create_retry_protection_context(true, retry_key, prefix_label)
            .expect("create protection ctx");

        let mut packet = vec![0u8; MAX_PACKET_SIZE];
        let packet_index = retry_protection_test_input.len();
        packet[..packet_index].copy_from_slice(&retry_protection_test_input);

        let length = encode_retry_protection(
            protection_ctx.as_ref(),
            &mut packet,
            packet_index,
            &retry_protection_test_odcid,
        );
        assert_eq!(length, packet_index + 16, "encoded length mismatch");
        assert_eq!(
            &packet[packet_index..length],
            &retry_protection_test_checksum,
            "appended checksum mismatch"
        );

        let verification_ctx = create_retry_protection_context(false, retry_key, prefix_label)
            .expect("create verification ctx");

        // Positive verify.
        let bytes_index = packet_index - RETRY_TOKEN_LENGTH; // = 17
        let new_length = verify_retry_protection(
            verification_ctx.as_ref(),
            &mut packet,
            length,
            bytes_index,
            &retry_protection_test_odcid,
        )
        .expect("verify retry protection");
        assert_eq!(
            new_length, packet_index,
            "verified length should equal input length"
        );

        // Bad ODCID → must fail.
        let mut bad_odcid = retry_protection_test_odcid;
        bad_odcid.as_bytes_mut()[0] ^= 1;
        packet[..packet_index].copy_from_slice(&retry_protection_test_input);
        packet[packet_index..length].copy_from_slice(&retry_protection_test_checksum);
        assert!(
            verify_retry_protection(
                verification_ctx.as_ref(),
                &mut packet,
                length,
                bytes_index,
                &bad_odcid,
            )
            .is_err(),
            "bad ODCID should fail verification"
        );

        // Draft-25 vector.
        let draft25_len = retry_protection_packet_draft25.len();
        packet[..draft25_len].copy_from_slice(&retry_protection_packet_draft25);
        let draft25_bytes_index = draft25_len - RETRY_TOKEN_LENGTH; // = 12
        verify_retry_protection(
            verification_ctx.as_ref(),
            &mut packet,
            draft25_len,
            draft25_bytes_index,
            &retry_protection_odcid_draft25,
        )
        .expect("draft-25 vector verification");
    }
}
```

## `picoquictest/config_test.c:config_preferred_test`
* C test-table name: `config_preferred`
* C entry function: `config_preferred_test`
* Rust test: `config_preferred`
* C source: `picoquictest/config_test.c:897-922`
* Rust source: `rs/fq/src/tests/config.rs:599-695`

### C test body
```c
{
    int ret = 0;
    for (size_t i = 0; ret == 0 && i < sizeof(test_preferred_address_cases) / sizeof(test_preferred_addr_t); i++) {
        picoquic_tp_preferred_address_t preferred_address;
        memset(&preferred_address, 0, sizeof(preferred_address));
        int is_valid = (picoquic_set_preferred_address(&preferred_address, test_preferred_address_cases[i].v4_text,
            test_preferred_address_cases[i].v6_text, test_preferred_address_cases[i].port) == 0);
        if (is_valid != test_preferred_address_cases[i].is_valid) {
            DBG_PRINTF("Test case %s: expected validity %d, got %d", test_preferred_address_cases[i].test_name,
                test_preferred_address_cases[i].is_valid, is_valid);
            ret = -1;
        }
        else if (is_valid) {
            if (preferred_address.is_defined != test_preferred_address_cases[i].preferred_address.is_defined ||
                memcmp(test_preferred_address_cases[i].preferred_address.ipv4Address, preferred_address.ipv4Address, 4) != 0 ||
                test_preferred_address_cases[i].preferred_address.ipv4Port != preferred_address.ipv4Port ||
                memcmp(test_preferred_address_cases[i].preferred_address.ipv6Address, preferred_address.ipv6Address, 16) != 0 ||
                test_preferred_address_cases[i].preferred_address.ipv6Port != preferred_address.ipv6Port) {
                DBG_PRINTF("Test case %s: expected and actual preferred addresses differ", test_preferred_address_cases[i].test_name);
                ret = -1;
            }
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn config_preferred() {
    struct Case {
        name: &'static str,
        v4_text: Option<&'static str>,
        v6_text: Option<&'static str>,
        port: u16,
        is_valid: bool,
        expected_v4: Option<SocketAddr>,
        expected_v6: Option<SocketAddr>,
    }

    let v4_192_0_2_1: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 4433);
    let v6_2001_db8_1: SocketAddr = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1)),
        4433,
    );

    let cases = [
        Case {
            name: "none",
            v4_text: None,
            v6_text: None,
            port: 0,
            is_valid: true,
            expected_v4: None,
            expected_v6: None,
        },
        Case {
            name: "v4_only",
            v4_text: Some("192.0.2.1"),
            v6_text: None,
            port: 4433,
            is_valid: true,
            expected_v4: Some(v4_192_0_2_1),
            expected_v6: None,
        },
        Case {
            name: "v6_only",
            v4_text: None,
            v6_text: Some("2001:db8::1"),
            port: 4433,
            is_valid: true,
            expected_v4: None,
            expected_v6: Some(v6_2001_db8_1),
        },
        Case {
            name: "both",
            v4_text: Some("192.0.2.1"),
            v6_text: Some("2001:db8::1"),
            port: 4433,
            is_valid: true,
            expected_v4: Some(v4_192_0_2_1),
            expected_v6: Some(v6_2001_db8_1),
        },
        Case {
            name: "bad v4",
            v4_text: Some("192.a.b.c"),
            v6_text: Some("2001:db8::1"),
            port: 4433,
            is_valid: false,
            expected_v4: None,
            expected_v6: None,
        },
        Case {
            name: "bad v6",
            v4_text: Some("192.0.2.1"),
            v6_text: Some("2001:local"),
            port: 4433,
            is_valid: false,
            expected_v4: None,
            expected_v6: None,
        },
    ];

    for case in &cases {
        let mut preferred = PreferredAddress::default();
        let result = set_preferred_address(&mut preferred, case.v4_text, case.v6_text, case.port);
        let is_valid = result.is_ok();
        assert_eq!(
            is_valid, case.is_valid,
            "Test case '{}': validity mismatch",
            case.name
        );
        if is_valid {
            assert_eq!(
                preferred.v4, case.expected_v4,
                "Test case '{}': v4 mismatch",
                case.name
            );
            assert_eq!(
                preferred.v6, case.expected_v6,
                "Test case '{}': v6 mismatch",
                case.name
            );
        }
    }
}
```

## `picoquictest/congestion_test.c:bbr_asym100_test`
* C test-table name: `bbr_asym100`
* C entry function: `bbr_asym100_test`
* Rust test: `bbr_asym100`
* C source: `picoquictest/congestion_test.c:389-406`
* Rust source: `rs/fq/src/tests/congestion.rs:849-851`

### C test body
```c
{
    uint64_t max_completion_time = 8500000;
    uint64_t latency = 1000;
    uint64_t jitter = 750;
    uint64_t buffer = 50000;
    uint64_t mbps = 10;
    uint64_t kbps = 100;

    int ret = performance_test_one(max_completion_time, mbps, kbps, latency, jitter, buffer, NULL);

    return ret;
}
```

### Rust test body
```rust
fn bbr_asym100() {
    performance_test_one(8_500_000, 10, 100, 1_000, 750, 50_000, None);
}
```
