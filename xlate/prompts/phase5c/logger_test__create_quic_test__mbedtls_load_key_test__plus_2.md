# Phase 5C post-merge test revalidation

This is a read-only final-tree audit after test repair
worktree merges. Do not edit files. Do not run full
`cargo test`.

For each C/Rust test pair, decide whether the current
merged Rust test faithfully expresses the C test's intent
and calls the right Rust API or test-harness surface.

Important standard:

* Phase 5C is about test/API correspondence, not runtime
  success.
* Report `ok` if the Rust test is present and faithfully
  checks the C test's API-visible behavior, even if the
  current library implementation would make it fail.
* Report `needs_fix` if the Rust test is missing checks,
  checks materially different behavior, weakens assertions,
  skips C cases, calls the wrong API/harness surface, or a
  worker repair appears lost in the merge.
* Report `blocked` only when the faithful Rust test cannot be
  written, compiled, or exposed as a runnable Rust test
  because the required Rust API or harness surface is missing
  or ambiguous.
* Do not report `blocked` for incomplete handshake behavior,
  wrong state transitions, callback counters not updating,
  or other implementation failures; those are Phase 6.

Return final JSON with this shape:

```json
{"results":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short final-tree conclusion","regression_risk":"none|possible|likely","fix_summary":"remaining test mismatch if any, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquictest/skip_frame_test.c:logger_test`
* C test-table name: `logger`
* C entry function: `logger_test`
* Rust test: `logger`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:4364-4366`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only opens a text log, creates one connection, logs a new connection and one app message. It omits the C test's frame corpus logging, reference log comparison, TLS ticket, packet/PDU logging, randomized no-Unknown checks, known bad-frame logging, and fuzz logging loops.
* Phase 5A fix note: Expand run_logger_test to cover the C logger_test scenarios: skip/error frame tables, reference text comparison, TLS ticket/app messages, packet/PDU logs, random packet Unknown-frame scan, bad-frame padding cases, and fuzz loops.
* Phase 5B analysis: Prior set_textlog/backend runtime failures are Phase 5C, but the C test directly calls picoquic_textlog_frames for the frame corpus and Rust has no faithful textlog frame API/harness surface to call from skip_frame.rs.
* Phase 5B fix note: 

### C test body
```c
{
    FILE* F = NULL;
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t fuzz_buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint64_t random_context = 0xF00BAB;
    struct sockaddr_in6 saddr = { 0 };
    picoquic_cnx_t * cnx = NULL;
    picoquic_quic_t * quic = NULL;
    uint64_t simulated_time = 123456789;
    uint64_t running_sum = 0;

    quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    saddr.sin6_family = AF_INET6;
    saddr.sin6_port = 443;
    memset(&saddr.sin6_addr, 0x20, 16);

    if (quic == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }
    else if ((cnx = picoquic_create_cnx(quic, logger_test_cid, logger_test_cid, (struct sockaddr*)&saddr,
        simulated_time, 0, "test-sni", "test-alpn", 1)) == NULL) {
        DBG_PRINTF("%s", "Cannot create CNX context\n");
        ret = -1;
    }
    else if (picoquic_set_textlog(quic, log_test_file) != 0) {
        DBG_PRINTF("failed to open file:%s\n", log_test_file);
        ret = -1;
    }
    else {
        for (size_t i = 0; i < nb_test_skip_list; i++) {
            picoquic_textlog_frames(quic->F_log, 0, test_skip_list[i].val, test_skip_list[i].len);
        }
        for (size_t i = 0; i < nb_test_frame_error_list; i++) {
            picoquic_textlog_frames(quic->F_log, 0, test_frame_error_list[i].val, test_frame_error_list[i].len);
        }
        fprintf(quic->F_log, "\n");
        picoquic_log_tls_ticket(cnx,
            log_test_ticket, (uint16_t) sizeof(log_test_ticket));

        picoquic_log_app_message(cnx, "%s.", "This is an app message test");
        picoquic_log_app_message(cnx, "This is app message test #%d, severity %d.", 1, 2);

        fprintf(quic->F_log, "\n");
        logger_test_packets(cnx);
        logger_test_pdus(quic, cnx);

        quic->F_log = picoquic_file_close(quic->F_log);
    }

    if (ret == 0) {
        char log_test_ref[512];

        ret = picoquic_get_input_path(log_test_ref, sizeof(log_test_ref), picoquic_solution_dir, LOG_TEST_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the log ref file name.\n");
        }
        else {
            ret = picoquic_test_compare_text_files(log_test_file, log_test_ref);
        }
    }

    /* Create a set of randomized packets. Verify that they can be logged without
     * causing the dreaded "Unknown frame" message */

    for (size_t i = 0; ret == 0 && i < 100; i++) {
        char log_line[1024];
        size_t bytes_max = format_random_packet(buffer, sizeof(buffer), &random_context, -1);

        if (picoquic_set_textlog(quic, log_packet_test_file) != 0) {
            DBG_PRINTF("failed to open file:%s\n", log_packet_test_file);
            ret = -1;
        }
        else {
            ret &= fprintf(quic->F_log, "Log packet test #%d\n", (int)i);
            picoquic_textlog_frames(quic->F_log, 0, buffer, bytes_max);
            quic->F_log = picoquic_file_close(quic->F_log);
        }

        if ((F = picoquic_file_open(log_packet_test_file, "r")) == NULL) {
            DBG_PRINTF("failed to open file:%s\n", log_packet_test_file);
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        } else {
            while (fgets(log_line, (int)sizeof(log_line), F) != NULL) {
                /* skip blanks */
                size_t byte_index = 0;

                while (byte_index < sizeof(log_line) &&
                    (log_line[byte_index] == ' ' || log_line[byte_index] == '\t')) {
                    byte_index++;
                }

                if (byte_index + 7u < sizeof(log_line) &&
                    memcmp(&log_line[byte_index], "Unknown", 7) == 0)
                {
                    DBG_PRINTF("Packet log test #%d failed, unknown frame.\n", (int)i);
                    ret = -1;
                    break;
                }
            }
            (void)picoquic_file_close(F);
        }
    }

    /* Log a series of known bad packets  */
    for (size_t i = 0; ret == 0 && i < nb_test_frame_error_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            uint8_t extra_bytes[4] = { 0, 0, 0, 0 };
            size_t bytes_max = 0;

            if (picoquic_set_textlog(quic, log_error_test_file) != 0) {
                DBG_PRINTF("failed to open file:%s\n", log_error_test_file);
                ret = -1;
                break;
            }
            fprintf(quic->F_log, "Running_sum: %" PRIx64 "\n", running_sum);
            memcpy(buffer, test_frame_error_list[i].val, test_frame_error_list[i].len);
            bytes_max = test_frame_error_list[i].len;
            if (test_frame_error_list[i].must_be_last == 0 && sharp_end == 0) {
                /* add some padding to check that the end of frame is detected properly */
                memcpy(buffer + bytes_max, extra_bytes, sizeof(extra_bytes));
                bytes_max += sizeof(extra_bytes);
            }

            picoquic_textlog_frames(quic->F_log, 0, buffer, bytes_max);

            quic->F_log = picoquic_file_close(quic->F_log);
            running_sum += picoquic_sum_text_file(log_error_test_file);
        }
    }

    /* Do a minimal fuzz test */
    for (size_t i = 0; ret == 0 && i < 100; i++) {
        size_t bytes_max = format_random_packet(buffer, sizeof(buffer), &random_context, -1);

        if (picoquic_set_textlog(quic, log_fuzz_test_file) != 0) {
            DBG_PRINTF("failed to open file:%s\n", log_fuzz_test_file);
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        }

        ret &= (fprintf(quic->F_log, "Log fuzz test #%d, sum: %" PRIx64 "\n",
            (int)i, running_sum) > 0);
        picoquic_textlog_frames(quic->F_log, 0, buffer, bytes_max);

        /* Attempt to log fuzzed packets, and hope nothing crashes */
        for (size_t j = 0; j < 100; j++) {
            ret &= fprintf(quic->F_log, "Log fuzz test #%d, packet %d\n", (int)i, (int)j);
            fflush(quic->F_log);
            skip_test_fuzz_packet(fuzz_buffer, buffer, bytes_max, &random_context);
            picoquic_textlog_frames(quic->F_log, 0, fuzz_buffer, bytes_max);
        }
        quic->F_log = picoquic_file_close(quic->F_log);
        running_sum += picoquic_sum_text_file(log_fuzz_test_file);
    }

    if (quic != NULL) {
        picoquic_free(quic);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn logger() {
    run_logger_test().expect("logger_test");
}
```

## `picoquictest/cnx_creation_test.c:create_quic_test`
* C test-table name: `create_quic`
* C entry function: `create_quic_test`
* Rust test: `create_quic`
* Expected Rust file: `rs/fq/src/tests/cnx_creation.rs`
* Current Rust span: `rs/fq/src/tests/cnx_creation.rs:207-328`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Most checks match, but the NULL transport-parameter reset is not faithfully tested: C calls picoquic_set_default_tp(quic, NULL), which reloads initialized defaults, while Rust passes TransportParameters::default(), the zeroed struct, and only checks success.
* Phase 5A fix note: Phase 5B should test the Rust equivalent of NULL/default reset, using initialized transport defaults via init_transport_parameters or an Option-style API, and assert the resulting default_tp matches initialized defaults.
* Phase 5B analysis: Rust now checks the initialized transport-parameter default state corresponding to C's picoquic_set_default_tp(quic, NULL), instead of passing a zeroed TransportParameters value and only checking success.
* Phase 5B fix note: Added initialized TP helper using init_transport_parameters, added full TP field comparison helper, and updated create_quic to perturb default_tp to zeroed values before resetting to initialized defaults and asserting the result.

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

### Current Rust test body
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

## `picoquictest/mbedtls_test.c:mbedtls_load_key_test`
* C test-table name: `mbedtls_load_key`
* C entry function: `mbedtls_load_key_test`
* Rust test: `mbedtls_load_key`
* Expected Rust file: `rs/fq/src/tests/mbedtls.rs`
* Current Rust span: `rs/fq/src/tests/mbedtls.rs:644-652`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust names the same six key fixtures, but the helper is placeholder-like: it checks PEM text and hashes bytes instead of initializing mbedTLS/PSA, loading the private key into a ptls context, requiring sign_certificate, and invoking the mbedTLS signer.
* Phase 5A fix note: Replace the Rust helper with a real mbedTLS-backed load-and-sign check, including init/free behavior or the Rust provider equivalent, while keeping the six positive key cases.
* Phase 5B analysis: Rust test now mirrors the Phase 5B API-visible contract: mbedTLS-only setup/teardown plus the same six successful key-load/sign helper assertions. Any real PSA/mbedTLS loader or sign_certificate incompleteness is Phase 5C.
* Phase 5B fix note: Added TlsApiResetGuard::mbedtls_only() at the start of mbedtls_load_key so reset_tls_api(0) runs on drop, matching C ptls_mbedtls_init()/ptls_mbedtls_free() around the key-load sequence.

### C test body
```c
{
    int ret = 0;


    /* Initialize the PSA crypto library. */
    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_RSA_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP256R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP384R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP521R1_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_SECP256R1_PKCS8_KEY);
        }

        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_RSA_PKCS8_KEY);
        }
#if 0
        /* Commenting out ED25519 for now, probably not supported yet in MBEDTLS/PSA */
        if (ret == 0) {
            ret = mbedtls_test_load_one_der_key(ASSET_ED25519_KEY);
        }
#endif
        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }

    return ret;
}
```

### Current Rust test body
```rust
fn mbedtls_load_key() {
    let _tls_api_reset = TlsApiResetGuard::mbedtls_only();
    mbedtls_test_load_one_der_key("certs/rsa/key.pem").expect("rsa key");
    mbedtls_test_load_one_der_key("certs/secp256r1/key.pem").expect("secp256r1 key");
    mbedtls_test_load_one_der_key("certs/secp384r1/key.pem").expect("secp384r1 key");
    mbedtls_test_load_one_der_key("certs/secp521r1/key.pem").expect("secp521r1 key");
    mbedtls_test_load_one_der_key("certs/secp256r1-pkcs8/key.pem").expect("secp256r1 pkcs8 key");
    mbedtls_test_load_one_der_key("certs/rsa-pkcs8/key.pem").expect("rsa pkcs8 key");
}
```

## `picoquictest/picoquic_lb_test.c:cid_for_lb_cli_test`
* C test-table name: `cid_for_lb_cli`
* C entry function: `cid_for_lb_cli_test`
* Rust test: `cid_for_lb_cli`
* Expected Rust file: `rs/fq/src/tests/picoquic_lb.rs`
* Current Rust span: `rs/fq/src/tests/picoquic_lb.rs:920-988`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Good-string and bad-string checks match, but the fuzz loop changes C inputs: C uses strlen after mutation, so NUL truncates the tested string, and raw 0xff is passed as a byte; Rust uses from_utf8_lossy over the full buffer.
* Phase 5A fix note: Make the Rust fuzz loop emulate C strlen truncation and treat invalid UTF-8 mutations as parse errors instead of lossy replacement.
* Phase 5B analysis: Rust fuzz loop now matches C strlen truncation and treats invalid UTF-8 mutations as parse failures instead of lossy replacement.
* Phase 5B fix note: Updated cid_for_lb_cli fuzz parsing to truncate at first NUL byte before parsing and count non-UTF-8 mutated prefixes as errors.

### C test body
```c
{
    int ret = 0;
    picoquic_load_balancer_config_t config;
    char buf[256];
    size_t fuzz_res[3] = { 0, 0, 0 };

    /* Parse each of the test strings and compare to corresponding config */
    if (nb_cid_for_lb_cli_test_config != nb_cid_for_lb_test_txt) {
        ret = -1;
    }
    for (size_t i = 0; ret == 0 &&  i < nb_cid_for_lb_cli_test_config; i++) {
        size_t txt_length = strlen(cid_for_lb_test_txt[i]);
        if (picoquic_lb_compat_cid_config_parse(&config, cid_for_lb_test_txt[i], txt_length) != 0) {
            ret = -1;
        }
        else if (config.method != cid_for_lb_cli_test_config[i].method) {
            ret = -1;
        }
        else if (config.rotation_bits != cid_for_lb_cli_test_config[i].rotation_bits) {
            ret = -1;
        }
        else if (config.first_byte_encodes_length != cid_for_lb_cli_test_config[i].first_byte_encodes_length) {
            ret = -1;
        }
        else if (config.server_id_length != cid_for_lb_cli_test_config[i].server_id_length) {
            ret = -1;
        }
        else if (config.nonce_length != cid_for_lb_cli_test_config[i].nonce_length) {
            ret = -1;
        }
        else if (config.connection_id_length != cid_for_lb_cli_test_config[i].connection_id_length) {
            ret = -1;
        }
        else if (config.server_id64 != cid_for_lb_cli_test_config[i].server_id64) {
            ret = -1;
        }
        else if (memcmp(config.cid_encryption_key, cid_for_lb_cli_test_config[i].cid_encryption_key, 16) != 0){
            ret = -1;
        }
    }
    /* Parse each of the bad strings and verify an error is returned */
    for (size_t i = 0; ret == 0 && i < nb_cid_for_lb_bad_txt; i++) {
        size_t txt_length = strlen(cid_for_lb_bad_txt[i]);
        if (picoquic_lb_compat_cid_config_parse(&config, cid_for_lb_bad_txt[i], txt_length) == 0) {
            ret = -1;
        }
    }
    /* Fuzz test */
    for (size_t i = 0; ret == 0 && i < nb_cid_for_lb_cli_test_config; i++) {
        size_t txt_length = strlen(cid_for_lb_test_txt[i]);
        if (txt_length < 255) {
            fuzz_res[0] += txt_length * nb_fuzz_c;
            for (size_t f = 0; f < txt_length; f++) {
                for (size_t fu = 0; fu < nb_fuzz_c; fu++) {
                    memcpy(buf, cid_for_lb_test_txt[i], txt_length);
                    buf[txt_length] = 0;
                    buf[f] = fuzz_c[fu];

                    if (picoquic_lb_compat_cid_config_parse(&config, buf, strlen(buf)) == 0) {
                        fuzz_res[1] += 1;
                    }
                    else {
                        fuzz_res[2] += 1;
                    }
                }
            }
        }
    }
    if (ret == 0 && fuzz_res[2] == 0) {
        ret = -1;
    }
    if (ret == 0 && fuzz_res[0] != fuzz_res[1] + fuzz_res[2]) {
        ret = -1;
    }
    /* Done */
    return ret;
}
```

### Current Rust test body
```rust
fn cid_for_lb_cli() {
    let expected = cli_test_configs();
    assert_eq!(expected.len(), CID_FOR_LB_TEST_TXT.len());

    for (i, txt) in CID_FOR_LB_TEST_TXT.iter().enumerate() {
        let config = Config::parse(txt)
            .unwrap_or_else(|_| panic!("parse failed for good string #{i}: {txt}"));
        let exp = &expected[i];
        assert_eq!(config.method, exp.method, "#{i} method");
        assert_eq!(
            config.rotation_bits, exp.rotation_bits,
            "#{i} rotation_bits"
        );
        assert_eq!(
            config.first_byte_encodes_length, exp.first_byte_encodes_length,
            "#{i} first_byte_encodes_length"
        );
        assert_eq!(
            config.server_id_length, exp.server_id_length,
            "#{i} server_id_length"
        );
        assert_eq!(config.nonce_length, exp.nonce_length, "#{i} nonce_length");
        assert_eq!(
            config.connection_id_length, exp.connection_id_length,
            "#{i} connection_id_length"
        );
        assert_eq!(config.server_id, exp.server_id, "#{i} server_id");
        assert_eq!(
            config.cid_encryption_key, exp.cid_encryption_key,
            "#{i} cid_encryption_key"
        );
    }

    for (i, bad) in CID_FOR_LB_BAD_TXT.iter().enumerate() {
        assert!(
            Config::parse(bad).is_err(),
            "bad string #{i} should fail to parse: {bad}"
        );
    }

    let mut fuzz_total = 0usize;
    let mut fuzz_ok = 0usize;
    let mut fuzz_err = 0usize;

    for txt in CID_FOR_LB_TEST_TXT.iter() {
        let bytes = txt.as_bytes();
        if bytes.len() < 255 {
            fuzz_total += bytes.len() * FUZZ_C.len();
            for f in 0..bytes.len() {
                for &fc in FUZZ_C {
                    let mut buf = bytes.to_vec();
                    buf[f] = fc;
                    let c_len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                    let parsed = core::str::from_utf8(&buf[..c_len])
                        .map_err(|_| ())
                        .and_then(|s| Config::parse(s).map_err(|_| ()));
                    if parsed.is_ok() {
                        fuzz_ok += 1;
                    } else {
                        fuzz_err += 1;
                    }
                }
            }
        }
    }

    assert!(fuzz_err > 0, "fuzz produced no parse errors");
    assert_eq!(fuzz_total, fuzz_ok + fuzz_err, "fuzz count mismatch");
}
```

## `picoquictest/skip_frame_test.c:skip_frame_test`
* C test-table name: `frames_skip`
* C entry function: `skip_frame_test`
* Rust test: `frames_skip`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:3521-3599`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only covers the good-frame test_skip_list loop with sharp_end/trailing bytes. It omits the C test's known-bad frame skip_fails loop, derived bad-varint cases, and deterministic random/fuzz packet pass.
* Phase 5A fix note: Extend frames_skip or its helpers to cover test_frame_error_list with both sharp_end variants, call/port the bad-varint skip checks, and add the minimal random packet plus 100x100 fuzz exercise while preserving C expectations for fuzzed failures.
* Phase 5B analysis: Rust frames_skip now covers the C good-frame loop plus known skip-failing bad frames, derived malformed-varint cases, and deterministic random/fuzz packet skipping.
* Phase 5B fix note: Added C test_frame_error_list fixtures, varint-count driven malformed-varint checks, C-compatible deterministic RNG helpers, random packet formatting, packet skipping, and 100x100 fuzz exercise.

### C test body
```c
{
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t fuzz_buffer[PICOQUIC_MAX_PACKET_SIZE];
    const uint8_t extra_bytes[4] = { 0xFF, 0, 0, 0 };
    uint64_t random_context = 0xBABED011;
    int fuzz_count = 0;
    int fuzz_fail = 0;
    picoquic_cnx_t cnx;

    memset(&cnx, 0, sizeof(cnx)); /* Null value gets default test version */

    for (size_t i = 0; ret == 0 && i < nb_test_skip_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            size_t consumed = 0;
            size_t byte_max = 0;
            int pure_ack;
            int t_ret = 0;

            memcpy(buffer, test_skip_list[i].val, test_skip_list[i].len);
            byte_max = test_skip_list[i].len;
            if (test_skip_list[i].must_be_last == 0 && sharp_end == 0) {
                memcpy(buffer + byte_max, extra_bytes, sizeof(extra_bytes));
                byte_max += sizeof(extra_bytes);
            }

            t_ret = picoquic_skip_frame(buffer, byte_max, &consumed, &pure_ack);

            if (t_ret != 0) {
                DBG_PRINTF("Skip frame <%s> fails, ret = %d\n", test_skip_list[i].name, t_ret);
                ret = t_ret;
            }
            else if (consumed != test_skip_list[i].len) {
                DBG_PRINTF("Skip frame <%s> fails, wrong length, %d instead of %d\n",
                    test_skip_list[i].name, (int)consumed, (int)test_skip_list[i].len);
                ret = -1;
            }
            else if (pure_ack != test_skip_list[i].is_pure_ack) {
                DBG_PRINTF("Skip frame <%s> fails, wrong pure ack, %d instead of %d\n",
                    test_skip_list[i].name, (int)pure_ack, (int)test_skip_list[i].is_pure_ack);
                ret = -1;
            }
        }
    }

    /* Check a series of known bad packets. We are checking that an error is
     * detected and no adverse code issue happens. */
    for (size_t i = 0; ret == 0 && i < nb_test_frame_error_list; i++) {
        for (int sharp_end = 0; ret == 0 && sharp_end < 2; sharp_end++) {
            size_t consumed = 0;
            size_t byte_max = 0;
            int pure_ack;
            int t_ret = 0;
            memcpy(buffer, test_frame_error_list[i].val, test_frame_error_list[i].len);
            byte_max = test_frame_error_list[i].len;
            if (test_frame_error_list[i].must_be_last == 0 && sharp_end == 0) {
                memcpy(buffer + byte_max, extra_bytes, sizeof(extra_bytes));
                byte_max += sizeof(extra_bytes);
            }

            t_ret = picoquic_skip_frame(buffer, byte_max, &consumed, &pure_ack);

            if (t_ret == 0 && test_frame_error_list[i].skip_fails) {
                DBG_PRINTF("Skip error frame <%s> does not fails, ret = %d\n", test_frame_error_list[i].name, t_ret);
                ret = -1;
            }
        }
    }
    /* Derive and test a series of packets with bad varint encodings */
    if (ret == 0) {
        ret = skip_frame_varint_test(buffer, PICOQUIC_MAX_PACKET_SIZE);
    }

    /* Do a minimal fuzz test */
    for (size_t i = 0; ret == 0 && i < 100; i++) {
        size_t bytes_max = format_random_packet(buffer, sizeof(buffer), &random_context, -1);

        ret = skip_test_packet(buffer, bytes_max);
        if (ret != 0) {
            DBG_PRINTF("Skip packet <%d> fails, ret = %d\n", i, ret);
        } else {
            /* do the actual fuzz test */
            int suspended = debug_printf_reset(1);
            for (size_t j = 0; j < 100; j++) {
                skip_test_fuzz_packet(fuzz_buffer, buffer, bytes_max, &random_context);
                if (skip_test_packet(fuzz_buffer, bytes_max) != 0) {
                    fuzz_fail++;
                }
                fuzz_count++;
            }
            (void)debug_printf_reset(suspended);
        }
    }

    if (ret == 0) {
        DBG_PRINTF("Fuzz skip test passes after %d trials, %d error detected\n",
            fuzz_count, fuzz_fail);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn frames_skip() {
    let extra = [0xff, 0, 0, 0];
    let frames = test_skip_frames();
    assert_eq!(frames.len(), TEST_SKIP_FRAME_VARINT_COUNTS.len());

    for case in &frames {
        for sharp_end in [false, true] {
            let mut frame = case.bytes.clone();
            let byte_max = if !case.must_be_last && !sharp_end {
                frame.extend_from_slice(&extra);
                frame.len()
            } else {
                case.bytes.len()
            };
            let mut consumed = 0usize;
            let mut pure_ack = 0i32;
            let ret = skip_frame(&frame, byte_max, &mut consumed, &mut pure_ack);
            assert_eq!(ret, 0, "skip_frame({})", case.name);
            assert_eq!(consumed, case.bytes.len(), "consumed({})", case.name);
            assert_eq!(pure_ack, case.pure_ack, "pure_ack({})", case.name);
        }
    }

    for case in test_frame_errors() {
        for sharp_end in [false, true] {
            let mut frame = case.bytes.clone();
            let byte_max = if !case.must_be_last && !sharp_end {
                frame.extend_from_slice(&extra);
                frame.len()
            } else {
                case.bytes.len()
            };
            let mut consumed = 0usize;
            let mut pure_ack = 0i32;
            let ret = skip_frame(&frame, byte_max, &mut consumed, &mut pure_ack);
            if case.skip_fails {
                assert_ne!(ret, 0, "skip error frame {} unexpectedly passed", case.name);
            }
        }
    }

    for (case, nb_varints) in frames
        .iter()
        .zip(TEST_SKIP_FRAME_VARINT_COUNTS.iter().copied())
    {
        for varint_idx in 1..=nb_varints {
            let frame = create_test_varint_frame(&case.bytes, varint_idx);
            if !frame.is_empty() {
                let mut consumed = 0usize;
                let mut pure_ack = 0i32;
                let ret = skip_frame(&frame, frame.len(), &mut consumed, &mut pure_ack);
                assert_ne!(
                    ret, 0,
                    "bad varint frame {} index {} unexpectedly passed",
                    case.name, varint_idx
                );
            }
        }
    }

    let mut random_context = 0xbabed011u64;
    let mut fuzz_count = 0usize;
    let mut fuzz_fail = 0usize;
    for i in 0..100 {
        let packet = format_random_packet(&frames, crate::MAX_PACKET_SIZE, &mut random_context);
        let ret = skip_test_packet(&packet);
        assert_eq!(ret, 0, "skip packet {} fails, ret = {}", i, ret);

        for _ in 0..100 {
            let fuzz_packet = skip_test_fuzz_packet(&packet, &mut random_context);
            if skip_test_packet(&fuzz_packet) != 0 {
                fuzz_fail += 1;
            }
            fuzz_count += 1;
        }
    }
    assert_eq!(fuzz_count, 10_000);
    assert!(fuzz_fail <= fuzz_count);
}
```
