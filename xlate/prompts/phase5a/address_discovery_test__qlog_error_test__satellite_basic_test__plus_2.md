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

## `picoquictest/p2p_test.c:address_discovery_test`
* C test-table name: `address_discovery`
* C entry function: `address_discovery_test`
* Rust test: `address_discovery`
* C source: `picoquictest/p2p_test.c:36-150`
* Rust source: `rs/fq/src/tests/p2p.rs:24-122`

### C test body
```c
{
    uint64_t simulated_time = 0;
    picoquic_connection_id_t initial_cid = { {0xad, 0xd8, 0xd1, 0x5c, 0, 0, 0, 0}, 8 };
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    uint64_t loss_mask = 0;
    int ret = 0;

    ret = tls_api_one_scenario_init_ex(&test_ctx, &simulated_time, PICOQUIC_INTERNAL_TEST_VERSION_1, NULL, NULL, &initial_cid);

    if (ret == 0 && test_ctx == NULL) {
        ret = -1;
    }

    if (ret == 0) {
        /* set the qlogs on both sides */
        picoquic_set_qlog(test_ctx->qclient, ".");
        picoquic_set_qlog(test_ctx->qserver, ".");
        picoquic_set_log_level(test_ctx->qserver, 1);
        picoquic_set_log_level(test_ctx->qclient, 1);
        test_ctx->qclient->use_long_log = 1;
        test_ctx->qserver->use_long_log = 1;
        /* Set the address discovery option */
        picoquic_set_default_address_discovery_mode(test_ctx->qclient, 3);
        picoquic_set_default_address_discovery_mode(test_ctx->qserver, 1);
        /* Delete the client connection and create a new one,
        * so it picks the parameters.
         */
        picoquic_delete_cnx(test_ctx->cnx_client);
        test_ctx->cnx_client = picoquic_create_cnx(test_ctx->qclient,
            initial_cid, picoquic_null_connection_id,
            (struct sockaddr*)&test_ctx->server_addr, simulated_time,
            0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, 1);

        if (test_ctx->cnx_client == NULL) {
            ret = -1;
        }
        else {
            ret = picoquic_start_client_cnx(test_ctx->cnx_client);
        }
    }

    /* establish the connection */
    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* wait until the client (and thus the server) is ready */
    if (ret == 0) {
        ret = wait_client_connection_ready(test_ctx, &simulated_time);
    }

    /* Check that the address discovery option is negotiated */
    if (ret == 0) {
        if (test_ctx->cnx_client->is_address_discovery_provider ||
            !test_ctx->cnx_client->is_address_discovery_receiver ||
            !test_ctx->cnx_server->is_address_discovery_provider ||
            test_ctx->cnx_server->is_address_discovery_receiver) {
            DBG_PRINTF("Address discovery not properly negotiated, C:(%u,%u), S:(%u,%u)",
                test_ctx->cnx_client->is_address_discovery_provider,
                test_ctx->cnx_client->is_address_discovery_receiver,
                test_ctx->cnx_server->is_address_discovery_provider,
                test_ctx->cnx_server->is_address_discovery_receiver);
            ret = -1;
        }
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_address_discovery, sizeof(test_scenario_address_discovery));

        if (ret != 0)
        {
            DBG_PRINTF("Init send receive scenario returns %d\n", ret);
        }
    }

    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* Check that the transmission succeeded */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, 1000000);
    }

    /* Check that the address discovery callback was called.
     */
    if (ret == 0) {
        if (test_ctx->nb_address_observed == 0) {
            DBG_PRINTF("Got % addresses observed", test_ctx->nb_address_observed);
            ret = -1;
        }
    }

    /* Check that the observed address was set on the client connection */
    if (ret == 0 && picoquic_compare_addr(
        (struct sockaddr*)&test_ctx->cnx_client->path[0]->first_tuple->local_addr,
        (struct sockaddr*)&test_ctx->cnx_client->path[0]->first_tuple->observed_addr) != 0) {
        char text1[256];
        char text2[256];

        DBG_PRINTF("Local: %s, observed: %s",
            picoquic_addr_text((struct sockaddr*)&test_ctx->cnx_client->path[0]->first_tuple->local_addr, text1, sizeof(text1)),
            picoquic_addr_text((struct sockaddr*)&test_ctx->cnx_client->path[0]->first_tuple->observed_addr, text2, sizeof(text2)));
        ret = -1;
    }

    /* Delete the context */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
    }

    return ret;
}
```

### Rust test body
```rust
fn address_discovery() {
    let mut simulated_time = crate::Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xad, 0xd8, 0xd1, 0x5c, 0, 0, 0, 0]).expect("initial CID");
    let mut loss_mask = 0u64;

    let mut test_ctx = tls_api_one_scenario_init_ex(
        &mut simulated_time,
        Version::InternalTest1,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_one_scenario_init_ex");

    // Set QLOG on both sides.
    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.qserver.set_qlog(".").ok();
    test_ctx.qserver.set_log_level(1);
    test_ctx.qclient.set_log_level(1);
    test_ctx.qclient.use_long_log = true;
    test_ctx.qserver.use_long_log = true;

    // Enable address discovery: client receives, server provides.
    test_ctx.qclient.set_default_address_discovery_mode(3);
    test_ctx.qserver.set_default_address_discovery_mode(1);

    // Delete the initial client connection and re-create it so the new
    // transport parameters are picked up.
    {
        let cnx = test_ctx.cnx_client();
        // The C code calls picoquic_delete_cnx then picoquic_create_cnx.
        // In Rust, the full re-creation is handled through the test context;
        // for now, just start the client connection.
        cnx.start_client().expect("start client");
    }

    // Establish the connection.
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    // Wait until the client (and server) are ready.
    super::util::wait_client_connection_ready(&mut test_ctx, &mut simulated_time)
        .expect("wait client ready");

    // Check that address discovery was negotiated.
    {
        let cnx_c = test_ctx.cnx_client();
        assert!(
            !cnx_c.is_address_discovery_provider,
            "client should not be a provider"
        );
        assert!(
            cnx_c.is_address_discovery_receiver,
            "client should be a receiver"
        );
    }
    {
        let cnx_s = test_ctx.cnx_server();
        assert!(
            cnx_s.is_address_discovery_provider,
            "server should be a provider"
        );
        assert!(
            !cnx_s.is_address_discovery_receiver,
            "server should not be a receiver"
        );
    }

    // Send test data.
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_ADDRESS_DISCOVERY)
        .expect("init send/recv scenario");

    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");

    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, 1_000_000)
        .expect("scenario body verify");

    // Verify the address-discovery callback fired.
    assert!(
        test_ctx.nb_address_observed > 0,
        "Got {} addresses observed",
        test_ctx.nb_address_observed
    );

    // Verify that the observed address matches the local address on path 0.
    {
        let cnx_c = test_ctx.cnx_client();
        let path0 = &cnx_c.paths[0];
        // The first tuple's local_addr and observed_addr should agree.
        let local = path0.tuples[0].local_addr;
        let observed = path0.tuples[0].observed_addr;
        assert_eq!(
            local, observed,
            "path[0] local addr ({local}) != observed addr ({observed})"
        );
    }
}
```

## `picoquictest/qlog_test.c:qlog_error_test`
* C test-table name: `qlog_error`
* C entry function: `qlog_error_test`
* Rust test: `qlog_error`
* C source: `picoquictest/qlog_test.c:333-363`
* Rust source: `rs/fq/src/tests/qlog.rs:182-199`

### C test body
```c
{
	FILE* F = picoquic_file_open(QLOG_ERROR_FILE, "w");
	int ret = (F == NULL) ? -1 : 0;

	if (ret == 0) {
		fprintf(F, "\n");
		ret = qlog_error_string(F);
		fprintf(F, "\n");
	}

	if (ret == 0) {
		ret = qlog_pref_addr_test(F);
		fprintf(F, "\n");
	}

	if (ret == 0) {
		ret = qlog_pref_vnego_test(F);
		fprintf(F, "\n");
	}

	if (ret == 0) {
		ret = qlog_tp_extension_test(F);
		fprintf(F, "\n");
	}

	if (F != NULL) {
		F = picoquic_file_close(F);
	}
	return ret;
}
```

### Rust test body
```rust
fn qlog_error() {
    let mut out: Vec<u8> = Vec::new();

    out.push(b'\n');
    qlog_error_string(&mut out).expect("qlog_error_string");
    out.push(b'\n');

    qlog_pref_addr_test(&mut out).expect("qlog_pref_addr_test");
    out.push(b'\n');

    qlog_pref_vnego_test(&mut out).expect("qlog_pref_vnego_test");
    out.push(b'\n');

    qlog_tp_extension_test(&mut out).expect("qlog_tp_extension_test");
    out.push(b'\n');

    std::fs::write(QLOG_ERROR_FILE, &out).expect("write qlog_error_test.txt");
}
```

## `picoquictest/satellite_test.c:satellite_basic_test`
* C test-table name: `satellite_basic`
* C entry function: `satellite_basic_test`
* Rust test: `satellite_basic`
* C source: `picoquictest/satellite_test.c:210-216`
* Rust source: `rs/fq/src/tests/satellite.rs:190-205`

### C test body
```c
{
    /* Should be less than 7 sec per draft etosat. */
    /* TODO test changed, app limited, verify. */
    /* return satellite_test_one(picoquic_bbr_algorithm, 100000000, 5300000, 250, 3, 0, 0, 0, 0, 0, 0); */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 5500000, 250, 3, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_basic() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        5_500_000,
        250,
        3,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/satellite_test.c:satellite_medium_test`
* C test-table name: `satellite_medium`
* C entry function: `satellite_medium_test`
* Rust test: `satellite_medium`
* C source: `picoquictest/satellite_test.c:259-263`
* Rust source: `rs/fq/src/tests/satellite.rs:323-338`

### C test body
```c
{
    /* Should be less than 20 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 18200000, 50, 10, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_medium() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        18_200_000,
        50,
        10,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/skip_frame_test.c:app_message_overflow_test`
* C test-table name: `app_message_overflow`
* C entry function: `app_message_overflow_test`
* Rust test: `app_message_overflow`
* C source: `picoquictest/skip_frame_test.c:3586-3660`
* Rust source: `rs/fq/src/tests/skip_frame.rs:1062-1094`

### C test body
```c
{
    int ret = 0;
    uint64_t simulated_time = 0;
    picoquic_cnx_t* cnx = NULL;
    struct sockaddr_storage addr;
    const picoquic_connection_id_t initial_cid = { { 8, 9, 0, 1, 2, 3, 4, 5 }, 8 };
    const picoquic_connection_id_t dest_cid = { { 16, 17, 18, 19, 20, 21, 22, 23 }, 8 };
    char qlog_test_ref[512];
    int ret_qlog = picoquic_get_input_path(qlog_test_ref, sizeof(qlog_test_ref),
        picoquic_solution_dir, QLOG_OVERFLOW_REF);
    picoquic_quic_t* quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    if (quic == NULL || ret_qlog != 0) {
        ret = -1;
    }
    else {
        picoquic_set_binlog(quic, ".");
        ret = picoquic_store_text_addr(&addr, "10.0.0.1", 1234);
        if (ret == 0) {
            cnx = picoquic_create_cnx(quic, initial_cid, dest_cid, (struct sockaddr*) & addr,
                simulated_time, 0, "test-sni", "test-alpn", 1);
            if (cnx == NULL) {
                ret = -1;
            }
            else {
                picoquic_log_new_connection(cnx);
            }
        }
    }
    if (ret == 0) {
        char test[BYTESTREAM_MAX_BUFFER_SIZE];

        memset(test, 'x', sizeof(test) - 3);
        test[sizeof(test) - 3] = '!';
        test[sizeof(test) - 2] = 0;

        for (int i = 0; i < 16; i++) {
            picoquic_log_app_message(cnx, "s:%s", &test[15 - i]);
        }

        picoquic_delete_cnx(cnx);
    }

    picoquic_free(quic);

    if (ret == 0) {
        /* Convert to QLOG and verify */
        uint64_t log_time = 0;
        uint16_t flags = 0;
        FILE* f_binlog = picoquic_open_cc_log_file_for_read(qlog_overflow_bin, &flags, &log_time);

        if (f_binlog == NULL) {
            DBG_PRINTF("Cannot open binlog file: %s.", qlog_overflow_bin);
            ret = -1;
        }
        else {
            ret = qlog_convert(&initial_cid, f_binlog, qlog_overflow_file, NULL, ".", flags);
            if (ret != 0) {
                DBG_PRINTF("%s", "Cannot convert the binary log into QLOG.\n");
            }
            else {
                ret = picoquic_test_compare_text_files(qlog_overflow_file, qlog_test_ref);
                if (ret != 0) {
                    DBG_PRINTF("%s", "Unexpected content in QLOG log file.\n");
                }
            }
            picoquic_file_close(f_binlog);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn app_message_overflow() {
    use crate::ConnectionId as InternalCid;

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);

    quic.set_binlog(Some(".")).ok();

    let initial_cid =
        InternalCid::clone_from_slice(&[8, 9, 0, 1, 2, 3, 4, 5]).expect("initial CID");
    let dest_cid =
        InternalCid::clone_from_slice(&[16, 17, 18, 19, 20, 21, 22, 23]).expect("dest CID");
    let addr: core::net::SocketAddr = "10.0.0.1:1234".parse().unwrap();

    let mut cnx = quic
        .create_connection_with_cids(
            initial_cid,
            dest_cid,
            Some(&addr),
            simulated_time,
            "test-sni",
            "test-alpn",
        )
        .expect("cnx");

    cnx.log_new_connection();

    // Fill a large string and log it 16 times to exercise overflow handling.
    let fill = "x".repeat(65533);
    for i in 0..16usize {
        cnx.log_app_message(&format!("s:{}", &fill[15 - i.min(15)..]));
    }
}
```
