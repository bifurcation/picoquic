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

## `picoquictest/multipath_test.c:multipath_ab1_test`
* C test-table name: `multipath_ab1`
* C entry function: `multipath_ab1_test`
* Rust test: `multipath_ab1`
* C source: `picoquictest/multipath_test.c:1315-1320`
* Rust source: `rs/fq/src/tests/multipath.rs:1400-1402`

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return multipath_test_one(max_completion_microsec, multipath_test_ab1);
}
```

### Rust test body
```rust
fn multipath_ab1() {
    multipath_test_one(3_000_000, MultipathTestId::Ab1);
}
```

## `picoquictest/multipath_test.c:multipath_nat_test`
* C test-table name: `multipath_nat`
* C entry function: `multipath_nat_test`
* Rust test: `multipath_nat`
* C source: `picoquictest/multipath_test.c:1372-1378`
* Rust source: `rs/fq/src/tests/multipath.rs:1570-1572`

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_nat);
}
```

### Rust test body
```rust
fn multipath_nat() {
    multipath_test_one(3_000_000, MultipathTestId::Nat);
}
```

## `picoquictest/multipath_test.c:multipath_standup_test`
* C test-table name: `multipath_standup`
* C entry function: `multipath_standup_test`
* Rust test: `multipath_standup`
* C source: `picoquictest/multipath_test.c:1511-1516`
* Rust source: `rs/fq/src/tests/multipath.rs:1639-1641`

### C test body
```c
{
    uint64_t max_completion_microsec = 7200000;

    return multipath_test_one(max_completion_microsec, multipath_test_standup);
}
```

### Rust test body
```rust
fn multipath_standup() {
    multipath_test_one(7_200_000, MultipathTestId::Standup);
}
```

## `picoquictest/pacing_test.c:pacing_bbr_test`
* C test-table name: `pacing_bbr`
* C entry function: `pacing_bbr_test`
* Rust test: `pacing_bbr`
* C source: `picoquictest/pacing_test.c:214-224`
* Rust source: `rs/fq/src/tests/pacing.rs:610-612`

### C test body
```c
{
    /* BBRv3 includes a short term loop that detects losses and tune the
    * sending rate accordingly. The packet losses cause startup to 
    * give up too soon, but this is fixed by probing up "quickly"
    * after exiting startup. The packet losses occur during startup
    * and during the probing periods.
    */
    int ret = pacing_cc_algotest(picoquic_bbr_algorithm, 900000, 160);
    return ret;
}
```

### Rust test body
```rust
fn pacing_bbr() {
    pacing_cc_algotest("bbr", 900_000, 160);
}
```

## `picoquictest/parseheadertest.c:packet_enc_dec_test`
* C test-table name: `packet_enc_dec`
* C entry function: `packet_enc_dec_test`
* Rust test: `packet_enc_dec`
* C source: `picoquictest/parseheadertest.c:778-909`
* Rust source: `rs/fq/src/tests/parseheadertest.rs:704-783`

### C test body
```c
{
    int ret = 0;
    struct sockaddr_in test_addr_c;
    picoquic_cnx_t* cnx_client = NULL;
    picoquic_cnx_t* cnx_server = NULL;
    picoquic_quic_t* qclient = NULL;
    picoquic_quic_t* qserver = NULL;
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_server_cert_store_file[512];
    const char *prefix_label;

    ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_CERT);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }
    else {
        qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL, NULL,
            NULL, NULL, NULL, 0, NULL, NULL, NULL, 0);
        qserver = picoquic_create(8,
            test_server_cert_file, test_server_key_file, test_server_cert_store_file,
            "test", NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0);
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
        }
        else {
            ret = picoquic_start_client_cnx(cnx_client);
        }
    }

    /* Test with a series of packets */
    /* First, client initial */
    if (ret == 0) {
        ret = test_packet_encrypt_one(
            (struct sockaddr *) &test_addr_c,
            cnx_client, qserver, NULL, picoquic_packet_initial, 1256);
    }
    /* If that work, update the connection context */
    if (ret == 0) {
        cnx_server = qserver->cnx_list;
        if (cnx_server == NULL) {
            DBG_PRINTF("%s", "Did not create the server connection context.\n");
            ret = -1;
        } else {
            /* Set the remote context ID for the client */
            cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id = cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id;
        }
    }

    prefix_label = picoquic_supported_versions[cnx_client->version_index].tls_prefix_label;

    /* Try handshake packet from client */
    if (ret == 0) {
        cnx_client->crypto_context[2].aead_encrypt = picoquic_setup_test_aead_context(1, test_handshake_secret, prefix_label);
        cnx_server->crypto_context[2].aead_decrypt = picoquic_setup_test_aead_context(0, test_handshake_secret, prefix_label);
        cnx_client->crypto_context[2].pn_enc = picoquic_pn_enc_create_for_test(test_handshake_secret, prefix_label);
        cnx_server->crypto_context[2].pn_dec = picoquic_pn_enc_create_for_test(test_handshake_secret, prefix_label);
        ret = test_packet_encrypt_one(
            (struct sockaddr *) &test_addr_c,
            cnx_client, qserver, cnx_server, picoquic_packet_handshake, 1256);
    }

    /* Now try a zero RTT packet */
    if (ret == 0) {
        cnx_client->crypto_context[1].aead_encrypt = picoquic_setup_test_aead_context(1, test_0rtt_secret, prefix_label);
        cnx_server->crypto_context[1].aead_decrypt = picoquic_setup_test_aead_context(0, test_0rtt_secret, prefix_label);
        cnx_client->crypto_context[1].pn_enc = picoquic_pn_enc_create_for_test(test_0rtt_secret, prefix_label);
        cnx_server->crypto_context[1].pn_dec = picoquic_pn_enc_create_for_test(test_0rtt_secret, prefix_label);

        /* Use a null connection ID to trigger use of initial ID */
        cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id = picoquic_null_connection_id;

        ret = test_packet_encrypt_one(
            (struct sockaddr *) &test_addr_c,
            cnx_client, qserver, cnx_server, picoquic_packet_0rtt_protected, 256);


        /* Set the remote context ID for the next test  */
        cnx_client->path[0]->first_tuple->p_remote_cnxid->cnx_id = cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id;
    }

    /* And try a 1 RTT packet */
    if (ret == 0) {
        cnx_client->crypto_context[3].aead_encrypt = picoquic_setup_test_aead_context(1, test_1rtt_secret, prefix_label);
        cnx_server->crypto_context[3].aead_decrypt = picoquic_setup_test_aead_context(0, test_1rtt_secret, prefix_label);
        cnx_client->crypto_context[3].pn_enc = picoquic_pn_enc_create_for_test(test_1rtt_secret, prefix_label);
        cnx_server->crypto_context[3].pn_dec = picoquic_pn_enc_create_for_test(test_1rtt_secret, prefix_label);

        ret = test_packet_encrypt_one(
            (struct sockaddr *) &test_addr_c,
            cnx_client, qserver, cnx_server, picoquic_packet_1rtt_protected, 1024);
    }

    if (cnx_client != NULL) {
        picoquic_delete_cnx(cnx_client);
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
fn packet_enc_dec() {
    let current_time = Instant::from_ticks(0);
    let addr: SocketAddr = "10.0.0.1:12345".parse().unwrap();

    let mut qclient = Quic::new(
        8,
        None,
        None,
        None,
        None,
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("create qclient");
    let mut qserver = Quic::new(
        8,
        Some(util::TEST_FILE_SERVER_CERT),
        Some(util::TEST_FILE_SERVER_KEY),
        Some(util::TEST_FILE_CERT_STORE),
        Some("test"),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        current_time,
        None,
        None,
    )
    .expect("create qserver");

    qclient
        .create_connection(
            ConnectionId::default(),
            ConnectionId::default(),
            Some(&addr),
            current_time,
            0,
            None,
            Some("picoquic-test"),
            true,
        )
        .expect("create client cnx");

    {
        let cnx = qclient.first_cnx_mut().expect("client cnx");
        cnx.start_client().expect("start client");

        // Initial packet
        test_packet_encrypt_one(&addr, cnx, &mut qserver, PacketType::Initial, 1256)
            .expect("initial enc_dec");
    }

    // Handshake packet
    {
        let prefix_label = {
            let cnx = qclient.first_cnx_mut().unwrap();
            cnx.version_tls_prefix_label()
        };
        let hs_secret: &[u8] = &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 1, 2, 3,
            4, 5, 6, 7, 8, 9, 10,
        ];
        {
            let cnx = qclient.first_cnx_mut().unwrap();
            cnx.set_test_aead_encrypt(Epoch::Handshake, hs_secret);
            cnx.set_test_pn_enc(Epoch::Handshake, hs_secret);
            let _ = prefix_label; // used in C to derive the test context
        }
        let cnx_server = qserver.first_cnx_mut().expect("server cnx");
        cnx_server.set_test_aead_decrypt(Epoch::Handshake, hs_secret);
        cnx_server.set_test_pn_dec(Epoch::Handshake, hs_secret);

        let cnx = qclient.first_cnx_mut().unwrap();
        test_packet_encrypt_one(&addr, cnx, &mut qserver, PacketType::Handshake, 1256)
            .expect("handshake enc_dec");
    }
}
```
