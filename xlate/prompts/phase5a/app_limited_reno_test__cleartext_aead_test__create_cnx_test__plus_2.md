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

## `picoquictest/app_limited.c:app_limited_reno_test`
* C test-table name: `app_limited_reno`
* C entry function: `app_limited_reno_test`
* Rust test: `app_limited_reno`
* C source: `picoquictest/app_limited.c:580-587`
* Rust source: `rs/fq/src/tests/app_limited.rs:521-525`

### C test body
```c
{
    app_limited_test_config_t config;
    app_limited_config_set_default(&config, 1);
    config.ccalgo = picoquic_newreno_algorithm;

    return app_limited_test_one(&config);
}
```

### Rust test body
```rust
fn app_limited_reno() {
    let mut config = AppLimitedConfig::default_config(1);
    config.ccalgo = get_congestion_algorithm("newreno").expect("newreno cc algo");
    app_limited_test_one(config);
}
```

## `picoquictest/cleartext_aead_test.c:cleartext_aead_test`
* C test-table name: `clear_text_aead`
* C entry function: `cleartext_aead_test`
* Rust test: `clear_text_aead`
* C source: `picoquictest/cleartext_aead_test.c:78-214`
* Rust source: `rs/fq/src/tests/cleartext_aead.rs:148-267`

### C test body
```c
{
    int ret = 0;
    uint8_t clear_text[1536];
    uint8_t incoming[1536];
    uint32_t seqnum = 0xdeadbeef;
    size_t clear_length = 1200;
    size_t encoded_length;
    size_t decoded_length;
    picoquic_packet_header ph_init;
    struct sockaddr_in test_addr_c, test_addr_s;
    picoquic_cnx_t* cnx_client = NULL;
    picoquic_cnx_t* cnx_server = NULL;
    picoquic_quic_t* qclient = NULL;
    picoquic_quic_t* qserver = NULL;
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_server_cert_store_file[512];

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
            (struct sockaddr*)&test_addr_c, 0, 0, NULL, NULL, 1);
        if (cnx_client == NULL) {
            DBG_PRINTF("%s", "Could not create client connection context.\n");
            ret = -1;
        }
    }

    if (ret == 0) {

        memset(&test_addr_s, 0, sizeof(struct sockaddr_in));
        test_addr_s.sin_family = AF_INET;
        memcpy(&test_addr_s.sin_addr, addr2, 4);
        test_addr_s.sin_port = 4433;

        cnx_server = picoquic_create_cnx(qserver, cnx_client->initial_cnxid, cnx_client->initial_cnxid,
            (struct sockaddr*)&test_addr_s, 0,
            cnx_client->proposed_version, NULL, NULL, 0);

        if (cnx_server == NULL) {
            DBG_PRINTF("%s", "Could not create server connection context.\n");
            ret = -1;
        } else if (picoquic_compare_connection_id(&cnx_client->initial_cnxid, &cnx_server->initial_cnxid) != 0) {
            DBG_PRINTF("Server Cnx-ID= %llx, differs from Client Cnx-ID = %llx\n",
                (unsigned long long) picoquic_val64_connection_id(cnx_client->initial_cnxid),
                (unsigned long long) picoquic_val64_connection_id(cnx_server->initial_cnxid));
            ret = -1;
        }
#if 0
        /* TODO: find replacement for this test */
        else if (picoquic_compare_cleartext_aead_contexts(cnx_client, cnx_server) != 0 ||
            picoquic_compare_cleartext_aead_contexts(cnx_server, cnx_client) != 0) {
            DBG_PRINTF("%s", "Cleartext encryption contexts no not match.\n");
            ret = -1;
        }
#endif
    }

    /* Create a packet from client to server, encrypt, decrypt */
    if (ret == 0) {
        cleartext_aead_packet_init_header(&ph_init,
            cnx_client->initial_cnxid, seqnum, cnx_client->proposed_version,
            picoquic_packet_initial);
        cleartext_aead_init_packet(&ph_init, clear_text, clear_length);

        /* AEAD Encrypt, to the send buffer */
        memcpy(incoming, clear_text, ph_init.offset);
        encoded_length = picoquic_aead_encrypt_generic(incoming + ph_init.offset,
            clear_text + ph_init.offset, clear_length - ph_init.offset,
            seqnum, incoming, ph_init.offset, cnx_client->crypto_context[0].aead_encrypt);
        encoded_length += ph_init.offset;

        /* AEAD Decrypt */
        decoded_length = picoquic_aead_decrypt_generic(incoming + ph_init.offset,
            incoming + ph_init.offset, encoded_length - ph_init.offset, seqnum,
            incoming, ph_init.offset, cnx_server->crypto_context[0].aead_decrypt);
        decoded_length += ph_init.offset;

        if (decoded_length != clear_length) {
            DBG_PRINTF("Decoded length (%d) does not match clear lenth (%d).\n", (int)decoded_length, (int)clear_length);
            ret = -1;
        } else if (memcmp(incoming, clear_text, clear_length) != 0) {
            DBG_PRINTF("%s", "Decoded message not match clear length.\n");
            ret = 1;
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
fn clear_text_aead() {
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
        Some(TEST_FILE_CERT_STORE),
        Some("test"),
        None,
        None,
        [0u8; RESET_SECRET_SIZE],
        t0,
        None,
        None,
    )
    .expect("server quic");

    // Create client connection and capture its initial CID and proposed version.
    let cnx_client = qclient
        .create_connection(
            ConnectionId::with_size(0).unwrap(),
            ConnectionId::with_size(0).unwrap(),
            Some(&client_addr),
            t0,
            0,
            None,
            None,
            true,
        )
        .expect("client connection");

    let client_initial_cnxid = cnx_client.initial_connection_id;
    let client_proposed_version = cnx_client.proposed_version;

    // Create server connection using the client's initial CID.
    let cnx_server = qserver
        .create_connection(
            client_initial_cnxid,
            client_initial_cnxid,
            Some(&server_addr),
            t0,
            client_proposed_version,
            None,
            None,
            false,
        )
        .expect("server connection");

    // Verify initial CIDs match.
    assert_eq!(
        cnx_client.initial_connection_id.as_bytes(),
        cnx_server.initial_connection_id.as_bytes(),
        "initial CID mismatch"
    );

    // Build a synthetic 1200-byte cleartext packet (17-byte header + payload).
    const CLEAR_LENGTH: usize = 1200;
    const SEQNUM: u32 = 0xdeadbeef;
    let mut clear_text = [0u8; 1536];
    let offset = init_aead_packet(
        client_initial_cnxid,
        SEQNUM,
        client_proposed_version,
        2, // picoquic_packet_initial
        &mut clear_text,
        CLEAR_LENGTH,
    );

    // AEAD encrypt (client → server): header = clear_text[..offset], plaintext = clear_text[offset..CLEAR_LENGTH].
    let mut payload: Vec<u8> = clear_text[offset..CLEAR_LENGTH].to_vec();
    cnx_client.crypto_context[0]
        .aead_encrypt
        .as_ref()
        .expect("client encrypt ctx")
        .encrypt(SEQNUM as u64, &clear_text[..offset], &mut payload);

    // payload now contains ciphertext + 16-byte AEAD tag.
    let encoded_length = offset + payload.len();
    let mut incoming = [0u8; 1536];
    incoming[..offset].copy_from_slice(&clear_text[..offset]);
    incoming[offset..encoded_length].copy_from_slice(&payload);

    // AEAD decrypt (server side).
    let mut ciphertext: Vec<u8> = incoming[offset..encoded_length].to_vec();
    cnx_server.crypto_context[0]
        .aead_decrypt
        .as_ref()
        .expect("server decrypt ctx")
        .decrypt(SEQNUM as u64, &incoming[..offset], &mut ciphertext)
        .expect("AEAD decrypt");

    // ciphertext now holds the decrypted plaintext (tag stripped).
    let decoded_length = offset + ciphertext.len();
    incoming[offset..decoded_length].copy_from_slice(&ciphertext);

    assert_eq!(decoded_length, CLEAR_LENGTH, "decoded length mismatch");
    assert_eq!(
        &incoming[..CLEAR_LENGTH],
        &clear_text[..CLEAR_LENGTH],
        "decrypted bytes do not match cleartext"
    );
}
```

## `picoquictest/cnx_creation_test.c:create_cnx_test`
* C test-table name: `create_cnx`
* C entry function: `create_cnx_test`
* Rust test: `create_cnx`
* C source: `picoquictest/cnx_creation_test.c:50-234`
* Rust source: `rs/fq/src/tests/cnx_creation.rs:50-193`

### C test body
```c
{
    int ret = 0;
    picoquic_quic_t* quic = NULL;
    picoquic_cnx_t* test_cnx[TEST_CNX_COUNT] = { NULL, NULL, NULL, NULL, NULL, NULL, NULL };
    struct sockaddr_in test4[5];
    struct sockaddr_in6 test6[3];
    const uint8_t test_ipv4[4] = { 192, 0, 2, 0 };
    const uint8_t test_ipv6[16] = { 0x20, 0x01, 0x0D, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01 };
    const uint8_t test_ipv4l[4] = { 127, 0, 0, 1 };
    const uint8_t test_ipv6l[16] = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01 };
    picoquic_connection_id_t test_cid[TEST_CNX_COUNT];

    const picoquic_connection_id_t test_cnx_id[TEST_CNX_COUNT] = {
        TEST_CNX_ID(1), TEST_CNX_ID(2), TEST_CNX_ID(3), TEST_CNX_ID(4),
        TEST_CNX_ID(5), TEST_CNX_ID(6), TEST_CNX_ID(7) };

    struct sockaddr* test_cnx_addr[TEST_CNX_COUNT] = {
        (struct sockaddr*)&test4[0],
        (struct sockaddr*)&test4[1],
        (struct sockaddr*)&test4[2],
        (struct sockaddr*)&test4[4],
        (struct sockaddr*)&test6[0],
        (struct sockaddr*)&test6[1],
        (struct sockaddr*)&test6[2]
    };

    /*
     * Initialize the sockaddr values
     */
    for (int i = 0; i < 5; i++) {
        uint8_t* addr = (uint8_t*)&test4[i].sin_addr;
        memset(&test4[i], 0, sizeof(test4[i]));
        test4[i].sin_family = AF_INET;
        if (i < 4) {
            addr[0] = test_ipv4[0];
            addr[1] = test_ipv4[1];
            addr[2] = test_ipv4[2];
            addr[3] = (i == 0) ? 1 : 2;
        }
        else {
            addr[0] = test_ipv4l[0];
            addr[1] = test_ipv4l[1];
            addr[2] = test_ipv4l[2];
            addr[3] = test_ipv4l[3];
        }
        test4[i].sin_port = 1000 + i;
    }

    for (int i = 0; i < 3; i++) {
        uint8_t* addr = (uint8_t*)&test6[i].sin6_addr;
        memset(&test6[i], 0, sizeof(test6[i]));
        test6[i].sin6_family = AF_INET6;
        for (int j = 0; j < 16; j++) {
            if (i < 2) {
                addr[j] = test_ipv6[j];
            }
            else {
                addr[j] = test_ipv6l[j];
            }
        }
        if (i < 2) {
            addr[15] = i + 1;
        }
        test6[i].sin6_port = 1000 + i;
    }

    for (int l = 0; ret == 0 && l < 2; l++) {
        /* Create QUIC context */
        quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, NULL, NULL, NULL, 0);
        if (quic == NULL) {
            ret = -1;
        }
        else if (l == 0) {
            quic->local_cnxid_length = 0;
        }
        /*
        * Create a set of connections, with variations :
        * -IPv4 or IPv6 address
        * -Different ports
        * -either no connection ID or a connection ID.
        */

        for (int i = 0; ret == 0 && i < TEST_CNX_COUNT; i++) {
            test_cnx[i] = picoquic_create_cnx(quic,
                (quic->local_cnxid_length == 0) ? picoquic_null_connection_id : test_cnx_id[i],
                picoquic_null_connection_id, test_cnx_addr[i], 0, 0, NULL, NULL, 1);
            if (test_cnx[i] == NULL) {
                ret = -1;
            }
            else {
                test_cid[i] = test_cnx[i]->path[0]->first_tuple->p_local_cnxid->cnx_id;
            }
        }

        /*
         *  -Verify that all these connections can be retrieved using their
         *    registered attributes.
         */
        if (quic->local_cnxid_length == 0) {
            for (int i = 0; ret == 0 && i < TEST_CNX_COUNT; i++) {
                picoquic_cnx_t* cnx = picoquic_cnx_by_net(quic, test_cnx_addr[i]);

                if (cnx == NULL) {
                    ret = -1;
                }
            }
        }

        /*
         * Verify that the iterator returns all connections.
         */
        if (ret == 0) {
            int counter = 0;
            for (picoquic_cnx_t* cnx = picoquic_get_first_cnx(quic); cnx != NULL; cnx = picoquic_get_next_cnx(cnx)) {
                counter += 1;
            }

            if (counter != TEST_CNX_COUNT) {
                ret = -1;
            }
        }

        /* TODO: cannot retrieve connections by initial ID yet, should work on it */
        /*
        *  -Verify that a non registered connection cannot be retrieved.
        */

        if (ret == 0) {
            if (quic->local_cnxid_length == 0) {
                picoquic_cnx_t* cnx = picoquic_cnx_by_net(quic, (struct sockaddr*)&test4[3]);
                if (cnx != NULL) {
                    ret = -1;
                }
            }
            else {
                picoquic_connection_id_t bad_target = { { 1,2,3,4,5,6,7,8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0}, 8 };
                picoquic_cnx_t* cnx = picoquic_cnx_by_id(quic, bad_target, NULL);
                if (cnx != NULL) {
                    ret = -1;
                }
            }
        }


        /* Delete connections first - middle - last. */
        for (int i = 0; ret == 0 && i < TEST_CNX_COUNT; i += 2) {
            picoquic_delete_cnx(test_cnx[i]);
            test_cnx[i] = NULL;
        }

        /* Verify that deleted connections cannot be retrieved, and the others can. */
        if (quic->local_cnxid_length == 0) {
            for (int i = 0; ret == 0 && i < TEST_CNX_COUNT; i++) {
                picoquic_cnx_t* cnx = picoquic_cnx_by_net(quic, test_cnx_addr[i]);

                if (cnx != NULL && (i & 1) == 0) {
                    ret = -1;
                }
                else if (cnx == NULL && (i & 1) != 0) {
                    ret = -1;
                }
            }
        }
        else {
            for (int i = 0; ret == 0 && i < TEST_CNX_COUNT; i++) {
                picoquic_cnx_t* cnx = picoquic_cnx_by_id(quic, test_cid[i], NULL);

                if (cnx != NULL && (i & 1) == 0) {
                    ret = -1;
                }
                else if (cnx == NULL && (i & 1) != 0) {
                    ret = -1;
                }
            }
        }

        /* delete QUIC context. */
        if (quic != NULL) {
            picoquic_free(quic);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn create_cnx() {
    const TEST_CNX_COUNT: usize = 7;

    let test_ipv4 = Ipv4Addr::new(192, 0, 2, 0);
    let test_ipv6 = Ipv6Addr::new(0x2001, 0x0DB8, 0, 0, 0, 0, 0, 0);
    let test_ipv4_local = Ipv4Addr::new(127, 0, 0, 1);
    let test_ipv6_local = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);

    // Build 5 IPv4 test addresses:
    //   [0] 192.0.2.1:1000  [1] 192.0.2.2:1001  [2] 192.0.2.2:1002
    //   [3] 192.0.2.2:1003  (used only for the "not found" check)
    //   [4] 127.0.0.1:1004
    let mut test4: [SocketAddr; 5] = [SocketAddr::new(IpAddr::V4(test_ipv4), 0); 5];
    for (i, addr) in test4.iter_mut().enumerate() {
        let ip = if i < 4 {
            let mut octets = test_ipv4.octets();
            octets[3] = if i == 0 { 1 } else { 2 };
            Ipv4Addr::from(octets)
        } else {
            test_ipv4_local
        };
        *addr = SocketAddr::new(IpAddr::V4(ip), 1000 + i as u16);
    }

    // Build 3 IPv6 test addresses:
    //   [0] 2001:db8::1:1000  [1] 2001:db8::2:1001  [2] ::1:1002
    let mut test6: [SocketAddr; 3] = [SocketAddr::new(IpAddr::V6(test_ipv6), 0); 3];
    for (i, addr) in test6.iter_mut().enumerate() {
        let ip = if i < 2 {
            let mut segments = test_ipv6.segments();
            segments[7] = (i as u16) + 1;
            Ipv6Addr::from(segments)
        } else {
            test_ipv6_local
        };
        *addr = SocketAddr::new(IpAddr::V6(ip), 1000 + i as u16);
    }

    // The 7 addresses used for connection creation (test4[3] is omitted).
    let test_cnx_addr: [SocketAddr; TEST_CNX_COUNT] = [
        test4[0], test4[1], test4[2], test4[4], test6[0], test6[1], test6[2],
    ];

    // Pre-built initial CIDs: {x,x,x,x,x,x,x,x} for x in 1..=7.
    let test_cnx_id: [ConnectionId; TEST_CNX_COUNT] =
        core::array::from_fn(|i| ConnectionId::clone_from_slice(&[(i + 1) as u8; 8]).unwrap());

    // C: loops l=0 (local_cnxid_length = 0, address-based) and
    //         l=1 (local_cnxid_length = 8, CID-based).
    for l in 0..2usize {
        let use_cid = l != 0;
        let mut quic = default_quic().expect("create quic");

        if !use_cid {
            // C: `quic->local_cnxid_length = 0`
            quic.local_connection_id_length = 0;
        }

        // Create 7 connections, record the initial CID assigned to each.
        let mut test_cid = [ConnectionId::default(); TEST_CNX_COUNT];
        for i in 0..TEST_CNX_COUNT {
            let initial_id = if use_cid {
                test_cnx_id[i]
            } else {
                ConnectionId::default()
            };
            let cnx = quic
                .create_connection(
                    initial_id,
                    ConnectionId::default(),
                    Some(&test_cnx_addr[i]),
                    Instant::from_ticks(0),
                    0,
                    None,
                    None,
                    true,
                )
                .expect("create_connection");
            // C: `test_cid[i] = test_cnx[i]->path[0]->first_tuple->p_local_cnxid->cnx_id`
            test_cid[i] = cnx.initial_connection_id();
        }

        // Verify that every connection can be retrieved by its registered attribute.
        if !use_cid {
            for addr in test_cnx_addr.iter() {
                assert!(
                    quic.connection_by_net(Some(addr)).is_some(),
                    "connection not found by net address"
                );
            }
        }

        // Verify the iterator visits all connections.
        // C: `for (cnx = first; cnx != NULL; cnx = next_cnx(cnx)) counter++`
        assert_eq!(
            quic.connections.len(),
            TEST_CNX_COUNT,
            "connection count mismatch after creation"
        );

        // Verify that an unregistered address / CID returns None.
        if !use_cid {
            // test4[3] was not used in test_cnx_addr.
            assert!(
                quic.connection_by_net(Some(&test4[3])).is_none(),
                "non-registered address must not be found"
            );
        } else {
            let bad_target = ConnectionId::clone_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
            assert!(
                quic.connection_by_id(bad_target).is_none(),
                "non-registered CID must not be found"
            );
        }

        // Delete connections at even indices (first, middle, last).
        // C: `picoquic_delete_cnx(test_cnx[i])` for i in {0,2,4,6}.
        for i in (0..TEST_CNX_COUNT).step_by(2) {
            let tok = if use_cid {
                quic.connection_by_id(test_cid[i]).map(|(t, _)| t)
            } else {
                quic.connection_by_net(Some(&test_cnx_addr[i]))
            };
            if let Some(t) = tok {
                quic.delete_connection(t);
            }
        }

        // Verify deleted connections are gone; surviving (odd-indexed) ones remain.
        for i in 0..TEST_CNX_COUNT {
            let found = if use_cid {
                quic.connection_by_id(test_cid[i]).is_some()
            } else {
                quic.connection_by_net(Some(&test_cnx_addr[i])).is_some()
            };
            if i % 2 == 0 {
                assert!(!found, "connection {i} (even) should have been deleted");
            } else {
                assert!(found, "connection {i} (odd) should still exist");
            }
        }
        // `quic` is dropped here — equivalent to `picoquic_free(quic)`.
    }
}
```

## `picoquictest/config_test.c:config_quic_test`
* C test-table name: `config_quic`
* C entry function: `config_quic_test`
* Rust test: `config_quic`
* C source: `picoquictest/config_test.c:772-782`
* Rust source: `rs/fq/src/tests/config.rs:478-573`

### C test body
```c
{
    int ret = 0;
    config_test_register_cc_algorithms();

    if (config_quic_test_one(&param1) != 0 ||
        config_quic_test_one(&param2) != 0) {
        ret = -1;
    }
    return ret;
}
```

### Rust test body
```rust
fn config_quic() {
    fn config_quic_test_one(mut config: Config) {
        use crate::tests::util::{
            TEST_ECH_CONFIG, TEST_ECH_PRIVATE_KEY, TEST_FILE_CERT_STORE, TEST_FILE_SERVER_CERT,
            TEST_FILE_SERVER_KEY,
        };

        // Swap in test-fixture paths where the config has non-None file fields,
        // mirroring the C `picoquic_get_input_path` substitutions.
        if config.server_cert_file.is_some() {
            config.server_cert_file = Some(fixture_path(TEST_FILE_SERVER_CERT));
        }
        if config.server_key_file.is_some() {
            config.server_key_file = Some(fixture_path(TEST_FILE_SERVER_KEY));
        }
        if config.root_trust_file.is_some() {
            config.root_trust_file = Some(fixture_path(TEST_FILE_CERT_STORE));
        }
        if config.ech_key_file.is_some() {
            config.ech_key_file = Some(fixture_path(TEST_ECH_PRIVATE_KEY));
        }
        if config.ech_config_file.is_some() {
            config.ech_config_file = Some(fixture_path(TEST_ECH_CONFIG));
        }

        let quic = config
            .create_and_configure(None, Instant::from_ticks(0), None)
            .expect("create_and_configure");

        // Check max connections.
        if config.nb_connections > 0 {
            assert_eq!(
                quic.max_nb_connections(),
                config.nb_connections,
                "max_nb_connections"
            );
        }

        // Check default ALPN.
        if let Some(alpn) = config.alpn.as_deref() {
            assert_eq!(quic.default_alpn_string(), Some(alpn), "default_alpn");
        }

        // Check reset seed.
        if config.has_reset_seed {
            assert_eq!(
                quic.reset_seed_bytes(),
                config.reset_seed.as_ref(),
                "reset_seed"
            );
        }

        // Check default congestion algorithm.
        if let Some(cc_id) = config.cc_algo_id.as_deref() {
            assert_eq!(
                quic.default_congestion_algorithm_id(),
                Some(cc_id),
                "cc_algo_id"
            );
        }

        // Check flow-control / initial max data.
        if config.flow_control_max != 0 {
            assert_eq!(
                quic.max_data_limit(),
                config.flow_control_max,
                "max_data_limit"
            );
            assert_eq!(
                quic.default_tp().initial_max_data,
                config.flow_control_max,
                "initial_max_data"
            );
        } else {
            assert_eq!(quic.max_data_limit(), 0, "max_data_limit (default)");
            assert_eq!(
                quic.default_tp().initial_max_data,
                0x0010_0000,
                "initial_max_data (default)"
            );
        }

        // Check preferred address is populated when either V4 or V6 is set.
        if config.preferred_address_v4.is_some() || config.preferred_address_v6.is_some() {
            let pa = &quic.default_tp().preferred_address;
            assert!(
                pa.v4.is_some() || pa.v6.is_some(),
                "preferred_address should be defined"
            );
        }
    }

    config_test_register_cc_algorithms();
    config_quic_test_one(parse_argv(ARGV1).expect("param1 config"));
    config_quic_test_one(parse_argv(ARGV2).expect("param2 config"));
}
```

## `picoquictest/congestion_test.c:bbr_asym100_nodelay_test`
* C test-table name: `bbr_asym100_nodelay`
* C entry function: `bbr_asym100_nodelay_test`
* Rust test: `bbr_asym100_nodelay`
* C source: `picoquictest/congestion_test.c:408-429`
* Rust source: `rs/fq/src/tests/congestion.rs:857-862`

### C test body
```c
{
    uint64_t max_completion_time = 8500000;
    uint64_t latency = 1000;
    uint64_t jitter = 750;
    uint64_t buffer = 50000;
    uint64_t mbps = 10;
    uint64_t kbps = 100;
    picoquic_tp_t server_parameters;

    memset(&server_parameters, 0, sizeof(picoquic_tp_t));
    picoquic_init_transport_parameters(&server_parameters);
    server_parameters.min_ack_delay = 0;

    int ret = performance_test_one(max_completion_time, mbps, kbps, latency, jitter, buffer,
        &server_parameters);

    return ret;
}
```

### Rust test body
```rust
fn bbr_asym100_nodelay() {
    let mut server_params = TransportParameters::default();
    init_transport_parameters(&mut server_params);
    server_params.min_ack_delay = Duration::from_ticks(0);
    performance_test_one(8_500_000, 10, 100, 1_000, 750, 50_000, Some(&server_params));
}
```
