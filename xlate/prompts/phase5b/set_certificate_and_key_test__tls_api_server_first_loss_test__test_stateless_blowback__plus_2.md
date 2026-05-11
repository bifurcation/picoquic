# Phase 5B repair Rust test mismatches

You are repairing Phase 5A `needs_fix` entries.  The goal
is to make the Rust tests faithfully check the same behavior
as the C tests.

Rules:

* Edit Rust tests, Rust test helpers, and Rust test fixtures
  only: `rs/fq/src/tests/` and `rs/fq/tests/fixtures/`.
* Do not edit C sources.
* Do not weaken assertions, skip important C cases, or replace
  tests with placeholders.
* If the test already matches after closer inspection, report
  `ok` and do not edit source.
* Phase 5B is about test/API correspondence, not test success.
  The Rust test must exist, compile as a test, and be runnable
  by the Rust test harness, but it may fail arbitrarily early
  because the Rust library implementation is incomplete.
* Do not report `blocked` merely because the implementation
  returns the wrong state, fails a handshake, lacks protocol
  behavior, or would fail the test. Those are Phase 5C issues.
* Report `blocked` only when the faithful test cannot be
  written, compiled, or exposed as a runnable Rust test because
  the necessary Rust API/test-harness surface is missing or
  ambiguous.
* Do not run full `cargo test` in this pass. Use source review
  and, if needed, `cargo check --tests` for compile validation.

Owned Rust test file(s): `rs/fq/src/tests/tls_api.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/tls_api_test.c:set_certificate_and_key_test`
* C test-table name: `set_certificate_and_key`
* C entry function: `set_certificate_and_key_test`
* Rust test: `set_certificate_and_key`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:7336-7397`
* Phase 5A analysis: Rust only runs a default handshake. The C test recreates the server without cert/key/root files, installs them through setter APIs, then checks the connection reaches ready state.
* Phase 5A fix note: Add a Rust test/helper that recreates qserver without certificate inputs, calls private-key/certificate-chain/root-certificate setter paths, runs the connection loop, and asserts client/server ready.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust body faithfully recreates qserver without cert inputs, sets private key, certificate chain, and root certs through the Rust API, re-enables server mode, runs the connection loop, and asserts both endpoints ready. However the containing tls_api.rs has duplicate top-level scenario constants, so the test module is not cleanly runnable.
* Phase 5C fix note: De-duplicate the top-level TEST_SCENARIO_Q_AND_R and TEST_SCENARIO_VERY_LONG definitions in rs/fq/src/tests/tls_api.rs; no set_certificate_and_key body mismatch found.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_server_cert_store_file[512];
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_CERT);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }

    /* Delete the server context, and recreate it. */
    if (ret == 0)
    {
        if (test_ctx->qserver != NULL) {
            picoquic_free(test_ctx->qserver);
        }

        test_ctx->qserver = picoquic_create(8,
            NULL, NULL, NULL,
            PICOQUIC_TEST_ALPN, test_api_callback, (void*)&test_ctx->server_callback, NULL, NULL, NULL,
            simulated_time, &simulated_time, NULL,
            test_ticket_encrypt_key, sizeof(test_ticket_encrypt_key));

        if (test_ctx->qserver == NULL) {
            ret = -1;
        }

        if (ret == 0) {
            ret = picoquic_set_private_key_from_file(test_ctx->qserver, test_server_key_file);
        }

        if (ret == 0) {
            size_t count = 0;
            ptls_iovec_t* chain = picoquic_get_certs_from_file(test_server_cert_file, &count);
            if (chain == NULL) {
                ret = -1;
            } else {
                picoquic_set_tls_certificate_chain(test_ctx->qserver, chain, count);
            }
        }

        if (ret == 0) {
            size_t count = 0;
            ptls_iovec_t* chain = picoquic_get_certs_from_file(test_server_cert_store_file, &count);

            if (chain == NULL) {
                ret = -1;
            } else {
                picoquic_set_tls_root_certificates(test_ctx->qserver, chain, count);
                for (size_t i = 0; i < count; i++) {
                    free(chain[i].base);
                }
                free(chain);
            }
        }
    }

    /* Proceed with the connection loop. */
    if (ret == 0) {
        picoquic_enforce_client_only(test_ctx->qserver, 0);
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0 && (!TEST_CLIENT_READY || !TEST_SERVER_READY)) {
        ret = -1;
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
    while let Some(token) = quic.first_connection().and_then(|cnx| cnx.own_token) {
        quic.delete_connection(token);
    }
}

/// C: `tls_api_server_losses_test` in `picoquictest/tls_api_test.c`.
///
/// Drops packets 2 and 3 (server flight) and verifies recovery.
#[test]
fn server_losses() {
    tls_api_loss_test(6).expect("server_losses");
}

/// C: `session_resume_test` in `picoquictest/tls_api_test.c`.
///
/// Two successive connections sharing a session ticket file; verifies the
/// second handshake uses PSK.
#[test]
fn session_resume() {
    const TICKET_FILE: &str = "session_resume_test.bin";
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;

    save_empty_tickets(TICKET_FILE, simulated_time).expect("save_empty");

    for i in 0..2 {
        let mut test_ctx =
            tls_api_init_ctx(&mut simulated_time, 0, Some(TICKET_FILE)).expect("ctx");
        test_ctx.cnx_client().max_early_data_size = 0;

        tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
            .unwrap_or_else(|e| panic!("session_resume pass {i}: connection loop: {e:?}"));

        if i == 1 {
            let server_psk = test_ctx.cnx_server().tls_is_psk_handshake();
            let client_psk = test_ctx.cnx_client().tls_is_psk_handshake();
            assert!(
                server_psk && client_psk,
                "session_resume pass {i}: expected PSK handshake, client={client_psk} server={server_psk}",
            );
        }

        if i == 0 {
            session_resume_wait_for_ticket(&mut test_ctx, &mut simulated_time)
                .expect("wait_for_ticket");
        }

        tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("close");

        assert!(
            !test_ctx.qclient.stored_tickets.is_empty(),
            "session_resume pass {i}: no ticket received",
        );
        test_ctx
            .qclient
            .save_tickets(simulated_time, TICKET_FILE)
            .expect("save_tickets");
    }
}

/// C: `set_certificate_and_key_test` in `picoquictest/tls_api_test.c`.
///
```

## `picoquictest/tls_api_test.c:tls_api_server_first_loss_test`
* C test-table name: `SH_loss`
* C entry function: `tls_api_server_first_loss_test`
* Rust test: `sh_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:1025-1027`
* Phase 5A analysis: Rust passes loss mask 14, but tls_api_loss_test ignores its _loss_mask parameter and always runs tls_api_test_with_loss with None/no loss, so it does not drop the first server flight packet.
* Phase 5A fix note: Thread the supplied loss mask into tls_api_test_with_loss/connection_loop so packet 14 is actually lost and recovery is verified.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust test calls tls_api_loss_test(14), but the final helper does not apply the shared loss mask to ordinary prepared packet submissions, so the first server-flight packet is not actually dropped.
* Phase 5C fix note: Apply/synchronize the loss mask for all normal client/server packet submits, including submit_prepared_sim_packets/GSO chunks, not only stateless/coalesced/admission paths.

### C test body
```c
{
    return tls_api_loss_test(14ull);
}
```

### Current Rust test body
```rust
#[test]
fn sh_loss() {
    tls_api_loss_test(14).expect("sh_loss");
```

## `picoquictest/tls_api_test.c:test_stateless_blowback`
* C test-table name: `stateless_blowback`
* C entry function: `test_stateless_blowback`
* Rust test: `stateless_blowback`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:7525-7607`
* Phase 5A analysis: Rust only performs a generic handshake/close; it does not exercise stateless reset blowback throttling, default interval checks, interval updates, or sent/not-sent outcomes for random 1-RTT packets.
* Phase 5A fix note: Add a Rust blowback helper that submits unknown-CID 1-RTT packets, drains prepare_next_packet, checks the C sequence of sent/not-sent outcomes, verifies the default interval, updates it to 2x default and then zero, and tests the immediate/+1 us cases.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The stateless blowback sequence matches the C sent/not-sent interval checks, but the current tls_api.rs module has duplicate top-level definitions and is not runnable as-is.
* Phase 5C fix note: Resolve duplicate top-level definitions in rs/fq/src/tests/tls_api.rs; no blowback assertion mismatch found.

### C test body
```c
{
    int was_sent = 0;
    uint64_t new_interval = 2 * PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT;

    /* Create a context with the default timer. */
    uint64_t simulated_time = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;

    picoquic_connection_id_t initial_cid = { {0xb1, 0x08, 0xba, 0xcc, 0, 0, 0, 0}, 8 };
    int ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN,
        &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);

    if (test_ctx->qserver->stateless_reset_min_interval != PICOQUIC_MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT) {
        DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
        ret = -1;

    }

    /* Format a random packet and submit it, verify that the stateless reset is queued */
    if (ret == 0) {
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("First stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Progress by 1/2 specified interval, retry, it should not work  */
    if (ret == 0) {
        simulated_time += test_ctx->qserver->stateless_reset_min_interval;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("Second stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }
    
    /* Progress by 1x specified interval, retry, it should work  */
    if (ret == 0) {
        simulated_time += test_ctx->qserver->stateless_reset_min_interval / 2;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && was_sent) {
            DBG_PRINTF("Third stateless reset was sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Reset the interval to twice the previous value */
    if (ret == 0) {
        picoquic_set_default_stateless_reset_min_interval(test_ctx->qserver, new_interval);
        if (test_ctx->qserver->stateless_reset_min_interval != new_interval) {
            DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
            ret = -1;
        }
    }
    
    /* Progress by 0.75x new interval, retry, it should work  */
    if (ret == 0) {
        simulated_time += (new_interval - new_interval / 4);
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After new interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Reset the interval to zero */
    if (ret == 0) {
        picoquic_set_default_stateless_reset_min_interval(test_ctx->qserver, 0);
        if (test_ctx->qserver->stateless_reset_min_interval != 0) {
            DBG_PRINTF("Stateless reset interval set to T=%" PRIu64, test_ctx->qserver->stateless_reset_min_interval);
            ret = -1;
        }
    }

    /* Try immediately, it should not work  */
    if (ret == 0) {
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After zero interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Add 1 microsec, it should work  */
    if (ret == 0) {
        simulated_time += 1;
        ret = test_stateless_blowback_one(test_ctx->qserver, &simulated_time, &was_sent);
        if (ret == 0 && !was_sent) {
            DBG_PRINTF("After zero +1 interval,  stateless reset was not sent at T=%" PRIu64, simulated_time);
            ret = -1;
        }
    }

    /* Free the resurce and return */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
        let server_retransmissions = test_ctx.cnx_server().nb_retransmission_total;
        assert_eq!(
            server_retransmissions, 0,
            "server had spurious retransmissions"
        );
    }
}

/// C: `spurious_retransmit_test` in `picoquictest/tls_api_test.c`.
///
/// Runs a 1-second silent period on 50ms links after the handshake and
/// verifies that neither endpoint records a spurious retransmission.
#[test]
fn spurious_retransmit() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    test_ctx.c_to_s_link.microsec_latency = 50_000;
    test_ctx.s_to_c_link.microsec_latency = 50_000;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");

    let next_time = Instant::from_ticks(simulated_time.ticks() + 1_000_000);
    while simulated_time < next_time && test_ctx.client_ready() && test_ctx.server_ready() {
        let mut was_active = false;
        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            next_time,
            &mut was_active,
        )
        .expect("silent simulation round");
    }

    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("close");

    let client_spurious = test_ctx.cnx_client().nb_spurious;
    assert_eq!(client_spurious, 0, "client had spurious retransmissions");

    if test_ctx.has_cnx_server() {
        let server_spurious = test_ctx.cnx_server().nb_spurious;
        assert_eq!(server_spurious, 0, "server had spurious retransmissions");
    }
}

fn stateless_blowback_one(quic: &mut crate::Quic, simulated_time: Instant) -> crate::Result<bool> {
    let filler = 0xff ^ (simulated_time.ticks() as u8);
    let mut bytes = [filler; crate::MAX_PACKET_SIZE];
    let addr_peer = SocketAddr::from(([1, 1, 1, 1], 1234));
    let addr_srv = SocketAddr::from(([2, 2, 2, 2], 4567));

    bytes[0] &= 0x7f;
    quic.incoming_packet(&mut bytes, &addr_peer, &addr_srv, 0, 0, simulated_time)?;

    let mut send_buffer = [0u8; crate::MAX_PACKET_SIZE];
    let prepared = quic.prepare_next_packet(simulated_time, &mut send_buffer)?;
    Ok(prepared.send_length > 0)
}

/// C: `test_stateless_blowback` in `picoquictest/tls_api_test.c`.
///
/// Verifies that stateless resets do not create an amplification loop.
#[test]
fn stateless_blowback() {
    let mut simulated_time = Instant::from_ticks(0);
    let new_interval = Duration::from_ticks(2 * MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT.ticks());
    let initial_cid =
        ConnectionId::clone_from_slice(&[0xb1, 0x08, 0xba, 0xcc, 0, 0, 0, 0]).expect("initial cid");
    let mut test_ctx = tls_api_init_ctx_ex(&mut simulated_time, V1, None, Some(&initial_cid))
        .expect("stateless_blowback ctx");

    assert_eq!(
        test_ctx.qserver.stateless_reset_min_interval, MICROSEC_STATELESS_RESET_INTERVAL_DEFAULT,
        "default stateless reset interval"
    );

    assert!(
        stateless_blowback_one(&mut test_ctx.qserver, simulated_time).expect("first packet"),
        "first stateless reset was not sent at T={}",
        simulated_time.ticks()
    );
```

## `picoquictest/tls_api_test.c:stop_sending_loss_test`
* C test-table name: `stop_sending_loss`
* C entry function: `stop_sending_loss_test`
* Rust test: `stop_sending_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:7899-7901`
* Phase 5A analysis: Rust calls the matching helper flags, but the helper does not preserve the C scenario: wrong stream set, missing pre-STOP partial-receive loop, different loss masks/latency, ignored stop_sending result, and missing final stream/data-node assertions.
* Phase 5A fix note: Rework Rust stop_sending_test_one to use the C two-stream scenario, 100ms latency, calibrated loss masks including RESET loss, partial first-stream receive before STOP_SENDING, and the same completion/leak checks.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: stop_sending_loss calls stop_sending_test_one(false, true), and the helper preserves the C two-stream scenario, 100ms latency, initial and reset-loss masks, partial first-stream receive before STOP_SENDING, final stream assertions, and data-node pool checks. However duplicate top-level constants in tls_api.rs prevent the test module from being cleanly runnable.
* Phase 5C fix note: De-duplicate the top-level TEST_SCENARIO_Q_AND_R and TEST_SCENARIO_VERY_LONG definitions in rs/fq/src/tests/tls_api.rs; no stop_sending_loss correspondence mismatch found.

### C test body
```c
{
    int ret = stop_sending_test_one(0, 1);
    return ret;
}
```

### Current Rust test body
```rust
            .expect("server remote connection id");
        (local_cid, remote_cid)
    };
```

## `picoquictest/tls_api_test.c:tls_api_test`
* C test-table name: `tls_api`
* C entry function: `tls_api_test`
* Rust test: `tls_api`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Rust span: `rs/fq/src/tests/tls_api.rs:7939-7966`
* Phase 5A analysis: Rust covers basic connection setup and close, but its tls_api_test_with_loss_final omits the C final assertions for transport parameters, SNI, ALPN, and negotiated version.
* Phase 5A fix note: Extend the Rust TLS final helper or this test to verify client/server transport parameters, SNI, ALPN, and negotiated version before closing, matching tls_api_test_with_loss_final.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The Rust test includes the final TP/SNI/ALPN/version checks matching C, but duplicate definitions in tls_api.rs prevent exposing it as a runnable test.
* Phase 5C fix note: Resolve duplicate top-level definitions in rs/fq/src/tests/tls_api.rs; no tls_api final-check mismatch found.

### C test body
```c
{
    return tls_api_test_with_loss(NULL, PICOQUIC_INTERNAL_TEST_VERSION_1, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN);
}
```

### Current Rust test body
```rust
    );
    let server_state = server_state.expect("server connection state");
    assert!(
        server_state <= State::Ready,
        "server connection advanced past ready: {server_state:?}"
    );
    assert!(
        test_ctx.qserver.pending_stateless_packets.is_empty(),
        "server queued a stateless packet for bogus long-header packet"
    );
}

/// C: `stop_sending_test` in `picoquictest/tls_api_test.c`.
///
/// Client sends STOP_SENDING on an active stream; verifies the server
/// resets the stream.
#[test]
fn stop_sending() {
    stop_sending_test_one(false, false).expect("stop_sending");
}

/// C: `stop_sending_loss_test` in `picoquictest/tls_api_test.c`.
///
/// Stop-sending test with loss injected on the RESET_STREAM response.
#[test]
fn stop_sending_loss() {
    stop_sending_test_one(false, true).expect("stop_sending_loss");
}
```
