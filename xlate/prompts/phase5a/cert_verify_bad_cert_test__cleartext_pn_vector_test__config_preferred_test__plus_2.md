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

## `picoquictest/cleartext_aead_test.c:cleartext_pn_vector_test`
* C test-table name: `pn_vector`
* C entry function: `cleartext_pn_vector_test`
* Rust test: `pn_vector`
* C source: `picoquictest/cleartext_aead_test.c:541-620`
* Rust source: `rs/fq/src/tests/cleartext_aead.rs:488-490`

### C test body
```c
{
    int ret = 0;
#if 0
    static const uint8_t cid[] = { 0x77, 0x0d, 0xc2, 0x6c, 0x17, 0x50, 0x9b, 0x35 };
    static const uint8_t sample[] = { 0x05, 0x80, 0x24, 0xa9, 0x72, 0x75, 0xf0, 0x1d, 0x2a, 0x1e, 0xc9, 0x1f, 0xd1, 0xc2, 0x65, 0xbb };
    static const uint8_t encrypted_pn[] = { 0x02, 0x6c, 0xe6, 0xde };
    static const uint8_t expected_pn[] = { 0xc0, 0x00, 0x00, 0x00 };

    struct sockaddr_in test_addr_s;
    picoquic_connection_id_t initial_cnxid;
    picoquic_cnx_t* cnx_server = NULL;
    picoquic_quic_t* qserver = NULL;
    char test_server_cert_file[512];
    char test_server_key_file[512];

    ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_CERT);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert or key file names.\n");
    }
    else {
        qserver = picoquic_create(8, test_server_cert_file, test_server_key_file,
            NULL, "test", NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0);
        if (qserver == NULL) {
            DBG_PRINTF("%s", "Could not create Quic contexts.\n");
            ret = -1;
        }
    }

    if (ret == 0 && picoquic_parse_connection_id(cid, sizeof(cid), &initial_cnxid) != sizeof(cid)) {
        ret = -1;
    }


    if (ret == 0) {

        memset(&test_addr_s, 0, sizeof(struct sockaddr_in));
        test_addr_s.sin_family = AF_INET;
        memcpy(&test_addr_s.sin_addr, addr2, 4);
        test_addr_s.sin_port = 4433;

        cnx_server = picoquic_create_cnx(qserver, initial_cnxid, initial_cnxid,
            (struct sockaddr*)&test_addr_s, 0, PICOQUIC_NINTH_INTEROP_VERSION, NULL, NULL, 0);

        if (cnx_server == NULL) {
            DBG_PRINTF("%s", "Could not create server connection context.\n");
            ret = -1;
        }
    }

    /* Try to decrypt the test vector */
    if (ret == 0) {
        uint8_t decrypted[8];

        memset(decrypted, 0, sizeof(decrypted));

        picoquic_pn_encrypt(cnx_server->crypto_context[0].pn_dec, sample, decrypted, encrypted_pn, sizeof(encrypted_pn));

        if (memcmp(decrypted, expected_pn, sizeof(expected_pn)) != 0)
        {
            DBG_PRINTF("%s", "Test of encoding PN vector failed.\n");
            ret = -1;
        }
    }

    if (cnx_server != NULL) {
        picoquic_delete_cnx(cnx_server);
    }

    if (qserver != NULL) {
        picoquic_free(qserver);
    }
#endif
    return ret;
}
```

### Rust test body
```rust
fn pn_vector() {
    // C body is entirely within #if 0; no-op.
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

## `picoquictest/congestion_test.c:bbr_jitter_test`
* C test-table name: `bbr_jitter`
* C entry function: `bbr_jitter_test`
* Rust test: `bbr_jitter`
* C source: `picoquictest/congestion_test.c:145-148`
* Rust source: `rs/fq/src/tests/congestion.rs:782-785`

### C test body
```c
{
    return congestion_control_test(picoquic_bbr_algorithm, 3600000, 5000, 5);
}
```

### Rust test body
```rust
fn bbr_jitter() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    congestion_control_test(ccalgo, 3_600_000, 5_000, 5);
}
```

## `picoquictest/congestion_test.c:bdp_delay_test`
* C test-table name: `bdp_delay`
* C entry function: `bdp_delay_test`
* Rust test: `bdp_delay`
* C source: `picoquictest/congestion_test.c:692-695`
* Rust source: `rs/fq/src/tests/congestion.rs:894-896`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_delay);
}
```

### Rust test body
```rust
fn bdp_delay() {
    bdp_option_test_one(BdpTestOption::Delay);
}
```
