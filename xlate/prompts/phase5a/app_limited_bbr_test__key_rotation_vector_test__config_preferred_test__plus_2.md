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

## `picoquictest/app_limited.c:app_limited_bbr_test`
* C test-table name: `app_limited_bbr`
* C entry function: `app_limited_bbr_test`
* Rust test: `app_limited_bbr`
* C source: `picoquictest/app_limited.c:600-607`
* Rust source: `rs/fq/src/tests/app_limited.rs:503-507`

### C test body
```c
{
    app_limited_test_config_t config;
    app_limited_config_set_default(&config, 3);
    config.ccalgo = picoquic_bbr_algorithm;

    return app_limited_test_one(&config);
}
```

### Rust test body
```rust
fn app_limited_bbr() {
    let mut config = AppLimitedConfig::default_config(3);
    config.ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    app_limited_test_one(config);
}
```

## `picoquictest/cleartext_aead_test.c:key_rotation_vector_test`
* C test-table name: `key_rotation_vector`
* C entry function: `key_rotation_vector_test`
* Rust test: `key_rotation_vector`
* C source: `picoquictest/cleartext_aead_test.c:1081-1115`
* Rust source: `rs/fq/src/tests/cleartext_aead.rs:866-921`

### C test body
```c
{
    int ret = 0;
    ptls_cipher_suite_t* key_rotation_test_suites[4] = {
        NULL, NULL, NULL, NULL };
    uint8_t new_secret[PTLS_MAX_DIGEST_SIZE];

    key_rotation_test_suites[0] = (ptls_cipher_suite_t*)picoquic_get_cipher_suite_by_id_v(PICOQUIC_AES_256_GCM_SHA384, 0);
    key_rotation_test_suites[1] = (ptls_cipher_suite_t*)picoquic_get_cipher_suite_by_id_v(PICOQUIC_AES_128_GCM_SHA256, 0);
    key_rotation_test_suites[2] = (ptls_cipher_suite_t*)picoquic_get_cipher_suite_by_id_v(PICOQUIC_CHACHA20_POLY1305_SHA256, 0);
    key_rotation_test_suites[3] = NULL;

    memcpy(new_secret, key_rotation_test_init, PTLS_MAX_DIGEST_SIZE);

    for (int i = 0; ret == 0 && key_rotation_test_suites[i] != NULL; i++) {
        memset(new_secret, 0, sizeof(new_secret));
        memcpy(new_secret, key_rotation_test_init, key_rotation_test_suites[i]->hash->digest_size);
        /* TODO: update to use the test vector of draft 25 and up */
        ret = picoquic_rotate_app_secret(key_rotation_test_suites[i], new_secret, PICOQUIC_LABEL_V1_TRAFFIC_UPDATE);
        if (ret != 0) {
            DBG_PRINTF("Cannot rotate secret[%d], ret=%x\n", i, ret);
        }
        else if (key_rotation_test_suites[i]->hash->digest_size != key_rotation_test_target_size[i]) {
            DBG_PRINTF("Wrong size for secret[%d], %d vs %d\n", i, (int) key_rotation_test_suites[i]->hash->digest_size, 
                (int) key_rotation_test_target_size[i]);
            ret = -1;
        }
        else if (memcmp(new_secret, key_rotation_test_target[i], key_rotation_test_target_size[i]) != 0) {
            DBG_PRINTF("Values don't match for secret[%d]\n", i);
            ret = -1;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn key_rotation_vector() {
    // Input secret: bytes 1..=64.
    let key_rotation_test_init: [u8; HASH_SIZE_MAX] = {
        let mut a = [0u8; HASH_SIZE_MAX];
        for (i, b) in a.iter_mut().enumerate() {
            *b = (i + 1) as u8;
        }
        a
    };

    // Expected outputs after rotation with LABEL_V1_TRAFFIC_UPDATE.
    #[rustfmt::skip]
    let target_sha384: [u8; 48] = [
        0xa1, 0xb5, 0xbd, 0xa2, 0x55, 0xf0, 0x7b, 0x68,
        0xdb, 0xe0, 0xa0, 0x39, 0x86, 0x94, 0xd9, 0x0d,
        0xe1, 0xf9, 0x46, 0xe4, 0x68, 0xf6, 0x87, 0xeb,
        0x19, 0x22, 0x5c, 0x92, 0x45, 0xe1, 0xf4, 0xe4,
        0x17, 0x73, 0xf6, 0x46, 0x5c, 0xb2, 0x24, 0xe0,
        0x5d, 0xb0, 0x40, 0x7a, 0x9b, 0x67, 0x47, 0xd1,
    ];
    #[rustfmt::skip]
    let target_sha256: [u8; 32] = [
        0x00, 0x70, 0x0d, 0x33, 0x5b, 0x1c, 0x49, 0xd1,
        0xe6, 0x37, 0x1e, 0x22, 0xd4, 0xa0, 0x17, 0x6d,
        0x0e, 0x34, 0x09, 0x19, 0x1b, 0x28, 0x46, 0x3c,
        0x38, 0xaf, 0x43, 0x34, 0x99, 0x43, 0x72, 0x57,
    ];
    // ChaCha20-Poly1305 also uses SHA-256.
    let target_poly = target_sha256;

    // (hash_algorithm_name, digest_size, expected_output)
    let cases: &[(&str, usize, &[u8])] = &[
        ("sha384", 48, &target_sha384),
        ("sha256", 32, &target_sha256),
        ("sha256", 32, &target_poly),
    ];

    for &(hash_name, digest_size, target) in cases {
        let mut new_secret = [0u8; HASH_SIZE_MAX];
        new_secret[..digest_size].copy_from_slice(&key_rotation_test_init[..digest_size]);

        let mut hasher = hash_create(hash_name).expect("hash_create");
        rotate_app_secret(
            &mut *hasher,
            &mut new_secret[..digest_size],
            LABEL_V1_TRAFFIC_UPDATE,
        )
        .expect("rotate_app_secret");

        assert_eq!(
            &new_secret[..digest_size],
            target,
            "key rotation mismatch for {hash_name}"
        );
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

## `picoquictest/congestion_test.c:bbr_long_test`
* C test-table name: `bbr_long`
* C entry function: `bbr_long_test`
* Rust test: `bbr_long`
* C source: `picoquictest/congestion_test.c:236-239`
* Rust source: `rs/fq/src/tests/congestion.rs:789-792`

### C test body
```c
{
    return congestion_long_test(picoquic_bbr_algorithm);
}
```

### Rust test body
```rust
fn bbr_long() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    congestion_long_test(ccalgo);
}
```

## `picoquictest/congestion_test.c:bdp_ip_test`
* C test-table name: `bdp_ip`
* C entry function: `bdp_ip_test`
* Rust test: `bdp_ip`
* C source: `picoquictest/congestion_test.c:687-690`
* Rust source: `rs/fq/src/tests/congestion.rs:900-902`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_ip);
}
```

### Rust test body
```rust
fn bdp_ip() {
    bdp_option_test_one(BdpTestOption::Ip);
}
```
