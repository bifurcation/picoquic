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

## `picoquictest/app_limited.c:app_limited_rpr_test`
* C test-table name: `app_limited_rpr`
* C entry function: `app_limited_rpr_test`
* Rust test: `app_limited_rpr`
* C source: `picoquictest/app_limited.c:609-621`
* Rust source: `rs/fq/src/tests/app_limited.rs:529-538`

### C test body
```c
{
    app_limited_test_config_t config;
    app_limited_config_set_default(&config, 4);
    config.ccalgo = picoquic_cubic_algorithm;
    config.do_preemptive_repeat = 1;
    config.loss_mask = 0x1482481224818214ull;
    config.completion_target = 47200000;
    config.nb_losses_max = 1980;
    config.rtt_max = 275000;

    return app_limited_test_one(&config);
}
```

### Rust test body
```rust
fn app_limited_rpr() {
    let mut config = AppLimitedConfig::default_config(4);
    config.ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    config.do_preemptive_repeat = true;
    config.loss_mask = 0x1482_4812_2481_8214_u64;
    config.completion_target = 47_200_000;
    config.nb_losses_max = 1_980;
    config.rtt_max = 275_000;
    app_limited_test_one(config);
}
```

## `picoquictest/cleartext_aead_test.c:cleartext_pn_enc_test`
* C test-table name: `cleartext_pn_enc`
* C entry function: `cleartext_pn_enc_test`
* Rust test: `cleartext_pn_enc`
* C source: `picoquictest/cleartext_aead_test.c:431-537`
* Rust source: `rs/fq/src/tests/cleartext_aead.rs:378-482`

### C test body
```c
{
    int ret = 0;
    struct sockaddr_in test_addr_c, test_addr_s;
    picoquic_cnx_t* cnx_client = NULL;
    picoquic_cnx_t* cnx_server = NULL;
    picoquic_quic_t* qclient = NULL;
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
        qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL, NULL,
            NULL, NULL, NULL, 0, NULL, NULL, NULL, 0);
        qserver = picoquic_create(8, test_server_cert_file, test_server_key_file,
            NULL, PICOQUIC_TEST_ALPN, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0);
        if (qclient == NULL || qserver == NULL) {
            DBG_PRINTF("%s", "Could not create Quic contexts.\n");
            ret = -1;
        }
    }

    if (ret == 0) {
        memset(&test_addr_c, 0, sizeof(struct sockaddr_in));
        test_addr_c.sin_family = AF_INET;
        memcpy(&test_addr_c.sin_addr, addr1, 4);
        test_addr_c.sin_port = 12345;

        cnx_client = picoquic_create_cnx(qclient, picoquic_null_connection_id, picoquic_null_connection_id,
            (struct sockaddr*)&test_addr_c, 0, 0, NULL, PICOQUIC_TEST_ALPN, 1);
        if (cnx_client == NULL) {
            DBG_PRINTF("%s", "Could not create client connection context.\n");
            ret = -1;
        } else {
            ret = picoquic_start_client_cnx(cnx_client);
        }
    }

    if (ret == 0) {

        memset(&test_addr_s, 0, sizeof(struct sockaddr_in));
        test_addr_s.sin_family = AF_INET;
        memcpy(&test_addr_s.sin_addr, addr2, 4);
        test_addr_s.sin_port = 4433;

        cnx_server = picoquic_create_cnx(qserver, cnx_client->initial_cnxid, cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id,
            (struct sockaddr*)&test_addr_s, 0,
            cnx_client->proposed_version, NULL, NULL, 0);

        if (cnx_server == NULL) {
            DBG_PRINTF("%s", "Could not create server connection context.\n");
            ret = -1;
        }
    }

    /* Try to encrypt a sequence number */
    if (ret == 0) {
        uint8_t seq_num_1[4] = { 0xde, 0xad, 0xbe, 0xef };
        uint8_t sample_1[16] = {
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
            0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a };
        uint8_t seq_num_2[4] = { 0xba, 0xba, 0xc0, 0x0l };
        uint8_t sample_2[16] = {
            0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96};

        ret = test_one_pn_enc_pair(seq_num_1, 4, 
            cnx_client->crypto_context[0].pn_enc, cnx_server->crypto_context[0].pn_dec, sample_1);

        if (ret != 0) {
            DBG_PRINTF("%s", "Test of encoding PN sample 1 failed.\n");
        } else {
            ret = test_one_pn_enc_pair(seq_num_2, 4, cnx_server->crypto_context[0].pn_enc, 
                cnx_client->crypto_context[0].pn_dec, sample_2);
            if (ret != 0) {
                DBG_PRINTF("%s", "Test of encoding PN sample 2 failed.\n");
            }
        }
    }

    if (cnx_client != NULL) {
        picoquic_delete_cnx(cnx_client);
    }

    if (cnx_server != NULL) {
        picoquic_delete_cnx(cnx_server);
    }

    if (qclient != NULL) {
        picoquic_free(qclient);
    }

    if (qserver != NULL) {
        picoquic_free(qserver);
    }

    return ret;
}
```

### Rust test body
```rust
fn cleartext_pn_enc() {
    let t0 = Instant::from_ticks(0);
    let client_addr: SocketAddr = SocketAddr::from(([10u8, 0, 0, 1], 12345u16));
    let server_addr: SocketAddr = SocketAddr::from(([10u8, 0, 0, 2], 4433u16));

    let mut qclient = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("client quic");

    let mut qserver = Quic::new(
        8,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        None,
        Some(TEST_ALPN),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("server quic");

    let cnx_client = qclient
        .create_connection(
            ConnectionId::with_size(0).unwrap(),
            ConnectionId::with_size(0).unwrap(),
            Some(&client_addr),
            t0,
            0,
            None,
            Some(TEST_ALPN),
            true,
        )
        .expect("client connection");

    cnx_client.start_client().expect("start client");

    let client_initial_cnxid = cnx_client.initial_connection_id;
    let client_remote_cnxid = cnx_client.remote_connection_id();
    let client_proposed_version = cnx_client.proposed_version;

    let cnx_server = qserver
        .create_connection(
            client_initial_cnxid,
            client_remote_cnxid,
            Some(&server_addr),
            t0,
            client_proposed_version,
            None,
            None,
            false,
        )
        .expect("server connection");

    // Test PN encryption pair 1: client enc → server dec.
    let seq_num_1: [u8; 4] = [0xde, 0xad, 0xbe, 0xef];
    let sample_1: [u8; 16] = [
        0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17,
        0x2a,
    ];
    test_one_pn_enc_pair(
        &seq_num_1,
        cnx_client.crypto_context[0]
            .pn_enc
            .as_deref()
            .expect("client pn_enc"),
        cnx_server.crypto_context[0]
            .pn_dec
            .as_deref()
            .expect("server pn_dec"),
        &sample_1,
    );

    // Test PN encryption pair 2: server enc → client dec.
    let seq_num_2: [u8; 4] = [0xba, 0xba, 0xc0, 0x00];
    let sample_2: [u8; 16] = [
        0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a, 0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f,
        0x96,
    ];
    test_one_pn_enc_pair(
        &seq_num_2,
        cnx_server.crypto_context[0]
            .pn_enc
            .as_deref()
            .expect("server pn_enc"),
        cnx_client.crypto_context[0]
            .pn_dec
            .as_deref()
            .expect("client pn_dec"),
        &sample_2,
    );
}
```

## `picoquictest/cnx_creation_test.c:create_quic_test`
* C test-table name: `create_quic`
* C entry function: `create_quic_test`
* Rust test: `create_quic`
* C source: `picoquictest/cnx_creation_test.c:236-334`
* Rust source: `rs/fq/src/tests/cnx_creation.rs:207-328`

### C test body
```c
{
    int ret = 0;
    char const* bad_dir = "..";
    char const* bad_file = "no_such_file_should_exist.pem";
    picoquic_quic_t* quic = NULL;

    /* Check that 0 connection == 1 */
    if (ret == 0) {
        quic = picoquic_create(0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0);
        if (quic == NULL || quic->max_number_connections != 1) {
            ret = -1;
        }
        picoquic_free(quic);
        quic = NULL;
    }

    /* Check that bad context, bad key or bad store crashes connection */
    if (ret == 0) {
        char test_server_cert_file[512];
        char test_server_key_file[512];

        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir,
            PICOQUIC_TEST_FILE_SERVER_CERT);

        if (ret == 0) {
            ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir,
                PICOQUIC_TEST_FILE_SERVER_KEY);
        }

        if (ret == 0) {
            if ((quic = picoquic_create(8, bad_file, test_server_key_file, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0)) != NULL ||
                (quic = picoquic_create(8, test_server_cert_file, bad_file, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0)) != NULL) {
                ret = -1;
                picoquic_free(quic);
                quic = NULL;
            }
        }
    }

    /* Check that bad ticket store does not crash a client connection */
    if (ret == 0) {
        if ((quic = picoquic_create(0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, bad_file, NULL, 0)) == NULL) {
            ret = -1;
        }
        else {
            picoquic_free(quic);
            if ((quic = picoquic_create(0, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, bad_dir, NULL, 0)) == NULL) {
                ret = -1;
            }
            else {
                picoquic_free(quic);
                quic = NULL;
            }
        }
    }

    /* Check loading of token file (always work) and not a valid file name (always fail).
    * However, this test is not very portable, because reading a bad directory only
    * fails on Windows.
     */
    if (ret == 0) {
        if ((quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0)) == NULL) {
            ret = -1;
        }
        else
        {
            int rbf = 0;
            int rbd = 0;
            if ((rbf = picoquic_load_token_file(quic, bad_file)) != 0 &&
                (rbd = picoquic_load_token_file(quic, bad_dir)) == 0) {
                ret = -1;
            }
            DBG_PRINTF("Load token %s %s",
                bad_file, (rbf == 0) ? "Succeeds" : "Fails");
            DBG_PRINTF("Load token %s %s",
                bad_dir, (rbd == 0) ? "Succeeds" : "Fails");
            picoquic_free(quic);
            quic = NULL;
        }
    }

    /* Check that loading a NULL TP loads the default */
    if (ret == 0) {
        if ((quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0)) == NULL) {
            ret = -1;
        }
        else
        {
            if (picoquic_set_default_tp(quic, NULL) != 0) {
                ret = -1;
            }
            picoquic_free(quic);
            quic = NULL;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn create_quic() {
    let bad_file = "no_such_file_should_exist.pem";
    let bad_dir = "..";

    // 0 connections clamps to 1.
    // C: `quic->max_number_connections != 1` → fail.
    {
        let quic = Quic::new(
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .expect("0 max_nb_connections must still create a context (clamped to 1)");
        assert_eq!(
            quic.max_number_connections, 1,
            "0 max_nb_connections must clamp to 1"
        );
    }

    // Bad cert or key path must cause creation to fail.
    let cert_file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/cert.pem");
    let key_file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../certs/key.pem");

    assert!(
        Quic::new(
            8,
            Some(bad_file),
            Some(key_file),
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .is_none(),
        "bad cert path must reject context creation"
    );
    assert!(
        Quic::new(
            8,
            Some(cert_file),
            Some(bad_file),
            None,
            None,
            None,
            None,
            [0u8; RESET_SECRET_SIZE],
            Instant::from_ticks(0),
            None,
            None,
        )
        .is_none(),
        "bad key path must reject context creation"
    );

    // Bad ticket-store path must NOT crash a client context.
    // C: the ticket file is position 13 in `picoquic_create` (→ Rust `ticket_file_name`).
    Quic::new(
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        Some(bad_file),
        None,
    )
    .expect("bad ticket-store file name should still create a context");

    Quic::new(
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        Instant::from_ticks(0),
        Some(bad_dir),
        None,
    )
    .expect("bad ticket-store directory should still create a context");

    // Load-token-file edge cases.
    // C: fails if bad_file fails AND bad_dir succeeds (Windows-specific).
    // On Linux/macOS, bad_dir usually fails too, so the condition is always false.
    {
        let mut quic = default_quic().expect("create quic");
        let rbf = quic.load_token_file(bad_file);
        if rbf.is_err() {
            // Only check bad_dir when bad_file failed.
            let rbd = quic.load_token_file(bad_dir);
            assert!(
                rbd.is_err(),
                "load_token_file: bad_dir succeeded where bad_file failed (platform-specific)"
            );
        }
    }

    // Resetting transport parameters to their defaults must succeed.
    // C: `picoquic_set_default_tp(quic, NULL)` — NULL resets to defaults.
    {
        let mut quic = default_quic().expect("create quic");
        quic.set_default_tp(&TransportParameters::default())
            .expect("set_default_tp with default params");
    }
}
```

## `picoquictest/config_test.c:config_set_port_test`
* C test-table name: `config_set_port`
* C entry function: `config_set_port_test`
* Rust test: `config_set_port`
* C source: `picoquictest/config_test.c:965-985`
* Rust source: `rs/fq/src/tests/config.rs:704-756`

### C test body
```c
{
    int ret = 0;
    for (size_t i = 0; ret == 0 && i < sizeof(test_set_port_cases) / sizeof(test_set_port_t); i++) {
        picoquic_quic_config_t config = { 0 };
        int is_valid = (config_set_port(&config, test_set_port_cases[i].port_string) == 0);
        if (is_valid != test_set_port_cases[i].is_valid) {
            DBG_PRINTF("Test case %zu: expected validity %d, got %d", i, test_set_port_cases[i].is_valid, is_valid);
            ret = -1;
        }
        else if (is_valid) {
            if (config.server_port != test_set_port_cases[i].server_port ||
                config.local_port != test_set_port_cases[i].local_port ||
                config.is_port_shared != test_set_port_cases[i].is_port_shared) {
                DBG_PRINTF("Test case %zu: expected and actual port settings differ", i);
                ret = -1;
            }
        }
    }
    return ret;
}
```

### Rust test body
```rust
fn config_set_port() {
    // (port_string, is_valid, server_port, local_port, is_port_shared)
    let cases: &[(&str, bool, u16, u16, bool)] = &[
        ("4433", true, 4433, 0, false),
        ("443:4434", true, 443, 4434, false),
        ("S4433", true, 4433, 0, true),
        ("S443:4434", true, 443, 4434, true),
        ("S443:4434*", true, 443, 4434, true),
        ("S443:4434*1", true, 443, 4434, true),
        ("S443:4434*256", true, 443, 4434, true),
        ("4433*7", true, 4433, 0, false),
        ("", true, 0, 0, false),
        ("0", true, 0, 0, false),
        ("*5", true, 0, 0, false),
        ("0*3", true, 0, 0, false),
        ("65535", true, 65535, 0, false),
        ("S65535", true, 65535, 0, true),
        ("S65534:65535", true, 65534, 65535, true),
        ("65536", false, 0, 0, false),
        ("-1", false, 0, 0, false),
        ("abc", false, 0, 0, false),
        ("4433:abc", false, 0, 0, false),
        ("abc:4433", false, 0, 0, false),
        ("S4433:abc", false, 0, 0, false),
        ("Sabc:4433", false, 0, 0, false),
    ];

    for (i, &(port_string, is_valid, exp_server_port, exp_local_port, exp_is_port_shared)) in
        cases.iter().enumerate()
    {
        let mut config = Config::default();
        let result = config.set_port(port_string);
        let got_valid = result.is_ok();
        assert_eq!(
            got_valid, is_valid,
            "Case {i} ({port_string:?}): validity mismatch"
        );
        if got_valid {
            assert_eq!(
                config.server_port, exp_server_port,
                "Case {i} ({port_string:?}): server_port"
            );
            assert_eq!(
                config.local_port, exp_local_port,
                "Case {i} ({port_string:?}): local_port"
            );
            assert_eq!(
                config.is_port_shared, exp_is_port_shared,
                "Case {i} ({port_string:?}): is_port_shared"
            );
        }
    }
}
```

## `picoquictest/congestion_test.c:bbr_asym400_test`
* C test-table name: `bbr_asym400`
* C entry function: `bbr_asym400_test`
* Rust test: `bbr_asym400`
* C source: `picoquictest/congestion_test.c:431-446`
* Rust source: `rs/fq/src/tests/congestion.rs:868-870`

### C test body
```c
{
    uint64_t max_completion_time = 2350000;
    uint64_t latency = 1000;
    uint64_t jitter = 750;
    uint64_t buffer = 50000;
    uint64_t mbps = 40;
    uint64_t kbps = 400;

    int ret = performance_test_one(max_completion_time, mbps, kbps, latency, jitter, buffer, NULL);

    return ret;
}
```

### Rust test body
```rust
fn bbr_asym400() {
    performance_test_one(2_350_000, 40, 400, 1_000, 750, 50_000, None);
}
```
