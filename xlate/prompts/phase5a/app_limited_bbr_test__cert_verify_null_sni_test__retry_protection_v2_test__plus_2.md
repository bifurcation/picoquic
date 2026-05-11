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

## `picoquictest/cert_verify_test.c:cert_verify_null_sni_test`
* C test-table name: `cert_verify_null_sni`
* C entry function: `cert_verify_null_sni_test`
* Rust test: `cert_verify_null_sni`
* C source: `picoquictest/cert_verify_test.c:247-255`
* Rust source: `rs/fq/src/tests/cert_verify.rs:80-88`

### C test body
```c
{
    int ret = cert_verify_test_one(1, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, NULL);
    return ret;
}
```

### Rust test body
```rust
fn cert_verify_null_sni() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        None,
    );
}
```

## `picoquictest/cleartext_aead_test.c:retry_protection_v2_test`
* C test-table name: `retry_protection_v2`
* C entry function: `retry_protection_v2_test`
* Rust test: `retry_protection_v2`
* C source: `picoquictest/cleartext_aead_test.c:1326-1366`
* Rust source: `rs/fq/src/tests/cleartext_aead.rs:819-862`

### C test body
```c
{
    int ret = 0;
    void* protection_ctx_v2 = NULL;

    /* Create a simple protection context from the V2 parameters */
    for (size_t i = 0; i < picoquic_nb_supported_versions; i++) {
        if (picoquic_supported_versions[i].version == PICOQUIC_V2_VERSION) {
            protection_ctx_v2 = picoquic_create_retry_protection_context(1,
                picoquic_supported_versions[i].version_retry_key,
                picoquic_supported_versions[i].tls_prefix_label);
            break;
        }
    }

    if (protection_ctx_v2 == NULL) {
        DBG_PRINTF("%s", "Cannot create protection context!");
        ret = -1;
    }
    else{ 
        uint8_t bytes[PICOQUIC_MAX_PACKET_SIZE] = { 0 };
        size_t byte_index = sizeof(v2_sample_retry) - 16;
        /* Copy the first bytes of the retry sample to the buffer */
        memcpy(bytes, v2_sample_retry, byte_index);
        /* Generate the signature */
        byte_index = picoquic_encode_retry_protection(protection_ctx_v2, bytes, PICOQUIC_MAX_PACKET_SIZE, byte_index, &v2_sample_odcid);
        /* Compare result and expectation */
        if (byte_index != sizeof(v2_sample_retry)) {
            DBG_PRINTF("Wrong retry packet size, expected %zu, got %zu", sizeof(v2_sample_retry), byte_index);
            ret = -1;
        }
        else if (memcmp(bytes, v2_sample_retry, byte_index) != 0) {
            DBG_PRINTF("%s", "Wrong retry packet value");
            ret = -1;
        }

        picoquic_aead_free(protection_ctx_v2);
    }

    return ret;
}
```

### Rust test body
```rust
fn retry_protection_v2() {
    let v2_sample_odcid =
        ConnectionId::clone_from_slice(&[0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08]).unwrap();
    #[rustfmt::skip]
    let v2_sample_retry: [u8; 36] = [
        0xcf, 0x6b, 0x33, 0x43, 0xcf, 0x00, 0x08, 0xf0,
        0x67, 0xa5, 0x50, 0x2a, 0x42, 0x62, 0xb5, 0x74,
        0x6f, 0x6b, 0x65, 0x6e, 0xc8, 0x64, 0x6c, 0xe8,
        0xbf, 0xe3, 0x39, 0x52, 0xd9, 0x55, 0x54, 0x36,
        0x65, 0xdc, 0xc7, 0xb6,
    ];

    let v2_params = Version::V2.parameters();
    let protection_ctx_v2 = create_retry_protection_context(
        true,
        v2_params.version_retry_key,
        v2_params.tls_prefix_label,
    )
    .expect("create V2 protection context");

    // The C test copies v2_sample_retry[..byte_index] into `bytes`, then calls
    // encode_retry_protection, and compares the full result to v2_sample_retry.
    let byte_index = v2_sample_retry.len() - 16; // payload before the integrity tag
    let mut bytes = vec![0u8; MAX_PACKET_SIZE];
    bytes[..byte_index].copy_from_slice(&v2_sample_retry[..byte_index]);

    let new_index = encode_retry_protection(
        protection_ctx_v2.as_ref(),
        &mut bytes,
        byte_index,
        &v2_sample_odcid,
    );

    assert_eq!(
        new_index,
        v2_sample_retry.len(),
        "V2 retry packet length mismatch"
    );
    assert_eq!(
        &bytes[..new_index],
        &v2_sample_retry[..],
        "V2 retry packet content mismatch"
    );
}
```

## `picoquictest/config_test.c:config_option_letters_test`
* C test-table name: `config_option_letters`
* C entry function: `config_option_letters_test`
* Rust test: `config_option_letters`
* C source: `picoquictest/config_test.c:38-53`
* Rust source: `rs/fq/src/tests/config.rs:438-441`

### C test body
```c
{
    char option_text[256];
    int ret = picoquic_config_option_letters(option_text, sizeof(option_text), NULL);

    if (ret != 0) {
        DBG_PRINTF("picoquic_config_option_letters returns %d", ret);
    }
    else if (strcmp(option_text, ref_option_text) != 0) {
        DBG_PRINTF("picoquic_config_option_letters returns %s", option_text);
        ret = -1;
    }

    return ret;
}
```

### Rust test body
```rust
fn config_option_letters() {
    let expected = "c:k:p:v:o:w:x:rR:s:XS:G:H:P:O:Me:C:i:l:Lb:q:m:n:a:t:zI:d:DQT:N:B:F:VU:0j:W:8J:E:y:K:Z:4:6:h";
    assert_eq!(Config::option_letters(), expected);
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
