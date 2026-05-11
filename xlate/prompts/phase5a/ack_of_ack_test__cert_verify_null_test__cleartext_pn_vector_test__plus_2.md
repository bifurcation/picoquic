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

## `picoquictest/ack_of_ack_test.c:ack_of_ack_test`
* C test-table name: `ack_of_ack`
* C entry function: `ack_of_ack_test`
* Rust test: `ack_of_ack`
* C source: `picoquictest/ack_of_ack_test.c:216-229`
* Rust source: `rs/fq/src/tests/ack_of_ack.rs:105-122`

### C test body
```c
{
    int ret = 0;

    for (size_t i = 0; i < sizeof(test_ack_of_ack_list) / sizeof(test_ack_of_ack_t); i++) {
        ret = ack_of_ack_do_one_test(&test_ack_of_ack_list[i]);

        if (ret != 0) {
            break;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn ack_of_ack() {
    for case in CASES {
        let mut sack_list = SackList::new();
        fill_sack_list(&mut sack_list, case.initial);

        let mut ack_buf = [0u8; 1024];
        let ack_length = build_test_ack(case.ack, &mut ack_buf);
        let mut consumed = 0usize;
        let ret = sack_list.process_ack_of_ack_frame(&mut ack_buf, ack_length, &mut consumed, 0);
        assert_eq!(
            ret, 0,
            "process_ack_of_ack_frame failed for case '{}'",
            case.name
        );

        cmp_sack_list(&mut sack_list, case.result);
    }
}
```

## `picoquictest/cert_verify_test.c:cert_verify_null_test`
* C test-table name: `cert_verify_null`
* C entry function: `cert_verify_null_test`
* Rust test: `cert_verify_null`
* C source: `picoquictest/cert_verify_test.c:210-217`
* Rust source: `rs/fq/src/tests/cert_verify.rs:67-75`

### C test body
```c
{
    int ret = cert_verify_test_one(1, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        NULL, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Rust test body
```rust
fn cert_verify_null() {
    cert_verify_test_one(
        true,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        None,
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

## `picoquictest/congestion_test.c:bbr1_test`
* C test-table name: `bbr1`
* C entry function: `bbr1_test`
* Rust test: `bbr1`
* C source: `picoquictest/congestion_test.c:246-249`
* Rust source: `rs/fq/src/tests/congestion.rs:874-877`

### C test body
```c
{
    return congestion_control_test(picoquic_bbr1_algorithm, 3600000, 0, 0);
}
```

### Rust test body
```rust
fn bbr1() {
    let ccalgo = get_congestion_algorithm("bbr1").expect("bbr1 cc algo");
    congestion_control_test(ccalgo, 3_600_000, 0, 0);
}
```
