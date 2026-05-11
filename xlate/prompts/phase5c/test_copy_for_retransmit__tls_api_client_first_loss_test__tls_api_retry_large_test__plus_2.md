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

## `picoquictest/skip_frame_test.c:test_copy_for_retransmit`
* C test-table name: `stream_retransmit_copy`
* C entry function: `test_copy_for_retransmit`
* Rust test: `stream_retransmit_copy`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:4011-4016`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust is only a placeholder-like smoke check. It does not port the C case table, stream data setup, expected copied bytes/lengths, pure-ack cases, MTU probe, ack trap, data-repeat queue checks, or misc-frame checks; it also passes a different force-queue argument.
* Phase 5A fix note: Port the C `copy_retransmit_case` table and loop into Rust, seed stream 0 data, call `copy_before_retransmit` with matching arguments, and assert pure-ack, length/bytes, data-repeat queue, and misc-frame expectations for every case.
* Phase 5B analysis: Rust now faithfully ports the C copy_retransmit_case loop and assertions.
* Phase 5B fix note: Added the C retransmit-copy case table, stream-0 setup, force_queue=0 call, and checks for pure ACK, copied length/bytes, stream-repeat flag, and misc-frame expectations.

### C test body
```c
{
    picoquic_quic_t * qtest = NULL;
    picoquic_cnx_t * cnx = NULL;
    int ret = 0;
    picoquic_packet_t old_p;
    uint8_t new_bytes[PICOQUIC_MAX_PACKET_SIZE];
    size_t length = 0;
    int packet_is_pure_ack = 0;
    int do_not_detect_spurious = 1;
    uint64_t simulated_time = 0;
    struct sockaddr_in saddr;

    memset(&saddr, 0, sizeof(struct sockaddr_in));

    /* Initialize the connection context */
    qtest = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    if (qtest == NULL) {
        DBG_PRINTF("%s", "Cannot create QUIC context\n");
        ret = -1;
    }

    /* Perform the tests */
    for (size_t i = 0; ret == 0 && i < nb_copy_retransmit_case; i++) {
        int add_to_data_repeat_queue = 0;

        cnx = picoquic_create_cnx(qtest,
            picoquic_null_connection_id, picoquic_null_connection_id, (struct sockaddr *) &saddr,
            simulated_time, 0, "test-sni", "test-alpn", 1);

        if (cnx == NULL) {
            DBG_PRINTF("%s", "Cannot create QUIC CNX context\n");
            ret = -1;
            break;
        }
        /* Initialize stream 0 */
        if ((ret = picoquic_add_to_stream(cnx, 0, ct_stream0_data, sizeof(ct_stream0_data), 0)) != 0) {
            DBG_PRINTF("%s", "Cannot initialize stream 0\n");
            ret = -1;
            break;
        }

        /* Initialize the old packet */
        memset(&old_p, 0, sizeof(picoquic_packet_t));
        if (copy_retransmit_case[i].packet_length > 0) {
            memcpy(old_p.bytes, copy_retransmit_case[i].packet, copy_retransmit_case[i].packet_length);
            old_p.length = copy_retransmit_case[i].packet_length;
        }
        old_p.offset = copy_retransmit_case[i].offset;
        old_p.is_mtu_probe = copy_retransmit_case[i].is_mtu_probe;
        old_p.is_ack_trap = copy_retransmit_case[i].is_ack_trap;
        old_p.send_path = cnx->path[0];

        length = copy_retransmit_case[i].b1_offset;

        ret = picoquic_copy_before_retransmit(&old_p, cnx, new_bytes,
            copy_retransmit_case[i].copy_max,
            &packet_is_pure_ack,
            &do_not_detect_spurious, 0,
            &length,
            &add_to_data_repeat_queue);

        if (ret != 0) {
            DBG_PRINTF("Cannot perform copy for test[%d]\n", i);
        } else if (packet_is_pure_ack != copy_retransmit_case[i].is_pure_ack_expected) {
            /* Check whether pure ack matches expectation */
            DBG_PRINTF("Is pure ack mismatch on test[%d], got %d\n", i,
                packet_is_pure_ack);
            ret = -1;
        }
        else if (!packet_is_pure_ack) {
            /* Compare bytes and length to expected */
            if (length != copy_retransmit_case[i].b1_length) {
                DBG_PRINTF("Length mismatch on test[%d], got %d vs %d\n", i,
                    packet_is_pure_ack, length,
                    copy_retransmit_case[i].b1_length);
                ret = -1;
            }
            else if (memcmp(new_bytes + copy_retransmit_case[i].b1_offset,
                copy_retransmit_case[i].b1_expected + copy_retransmit_case[i].b1_offset,
                length - copy_retransmit_case[i].b1_offset) != 0) {
                DBG_PRINTF("Value mismatch on test[%d]\n", i);
                ret = -1;
            }
            else {
                if (copy_retransmit_case[i].b2_expected == NULL) {
                    if (add_to_data_repeat_queue) {
                        DBG_PRINTF("Unexpected stream frame in test[%d]\n", i);
                        ret = -1;
                    }
                }
                else if (!add_to_data_repeat_queue) {
                    DBG_PRINTF("Missing stream frame in test[%d]\n", i);
                    ret = -1;
                }

                if (ret == 0) {
                    if (copy_retransmit_case[i].b3_expected == NULL) {
                        if (cnx->first_misc_frame != NULL) {
                            DBG_PRINTF("Unexpected misc frame in test[%d]\n", i);
                            ret = -1;
                        }
                    }
                    else if (cnx->first_misc_frame == NULL) {
                        DBG_PRINTF("Missing misc frame in test[%d]\n", i);
                        ret = -1;
                    }
                    else if (copy_retransmit_case[i].b3_length != cnx->first_misc_frame->length) {
                        DBG_PRINTF("Mismatching misc frame lenght in test[%d]\n", i);
                        ret = -1;
                    }
                    else if (memcmp(((uint8_t*)cnx->first_misc_frame) + sizeof(picoquic_misc_frame_header_t),
                        copy_retransmit_case[i].b3_expected, cnx->first_misc_frame->length) != 0) {
                        DBG_PRINTF("Mismatching misc frame in test[%d]\n", i);
                        ret = -1;
                    }
                }
            }
        }
        /* Free the extra frames */
        if (cnx != NULL) {
            picoquic_delete_cnx(cnx);
        }
    }

    /* Free the connection context */
    if (qtest != NULL) {
        picoquic_free(qtest);
    }
    return ret;
}
```

### Current Rust test body
```rust
fn stream_retransmit_copy() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    run_stream_retransmit_copy_test(&mut quic, &mut simulated_time)
        .expect("copy_before_retransmit");
}
```

## `picoquictest/tls_api_test.c:tls_api_client_first_loss_test`
* C test-table name: `first_loss`
* C entry function: `tls_api_client_first_loss_test`
* Rust test: `first_loss`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:2654-2656`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-01`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test passes 1 like C, but tls_api_loss_test ignores the mask and calls tls_api_test_with_loss with no loss, so it does not drop the first client packet.
* Phase 5A fix note: Make tls_api_loss_test propagate the supplied mask into the connection loop so first_loss actually exercises loss_mask = 1 recovery.
* Phase 5B analysis: Rust first_loss already passed mask 1; the helper was discarding it, so the loss path did not run.
* Phase 5B fix note: Changed tls_api_loss_test to create a local mutable loss mask, pass it into tls_api_connection_loop, then run the same final close/verification helper.

### C test body
```c
{
    return tls_api_loss_test(1ull);
}
```

### Current Rust test body
```rust
fn first_loss() {
    tls_api_loss_test(1).expect("first_loss");
}
```

## `picoquictest/tls_api_test.c:tls_api_retry_large_test`
* C test-table name: `retry_large`
* C entry function: `tls_api_retry_large_test`
* Rust test: `retry_large`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7051-7053`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes true but tls_api_retry_test_one ignores large_client_hello. It also starts the client during init instead of using delayed init, omits qlog setup, and does not enforce the C target_time <= 230000 check.
* Phase 5A fix note: Set test_large_chello before starting the client, preserve delayed-start retry setup, enable qlog as in C, run retry/close, and assert completion time stays within 230000 us.
* Phase 5B analysis: Rust test now faithfully sets the large ClientHello flag before starting the client and checks the C retry close/time behavior. The narrow test exposes an implementation timing failure: 30000000 us > 230000 us.
* Phase 5B fix note: Added delayed-start retry_large path in tls_api.rs with client qlog, server cookie mode, retry/close, and the 230000 us assertion.

### C test body
```c
{
    return tls_api_retry_test_one(1);
}
```

### Current Rust test body
```rust
fn retry_large() {
    retry_large_delayed_start().expect("retry_large");
}
```

## `picoquictest/tls_api_test.c:tls_zero_share_test`
* C test-table name: `tls_zero_share`
* C entry function: `tls_zero_share_test`
* Rust test: `tls_zero_share`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8354-8366`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C forces zero-share with tls_api_init_ctx(..., proposed_version=0, force_zero_share=1) and then checks handshake plus close. Rust calls the generic TLS helper with V1 and never sets client_zero_share, so it does not exercise the zero-share/HelloRetryRequest path.
* Phase 5A fix note: Add a zero-share-specific Rust test/helper that creates the context with proposed_version=0/default negotiation, sets qclient.client_zero_share before starting/running the handshake, then runs the connection loop and close checks.
* Phase 5B analysis: Rust test now mirrors the C zero-share setup using proposed_version=0, force-zero-share before client start, handshake loop, and close.
* Phase 5B fix note: Added a zero-share TLS API initializer in the Rust test helper and updated tls_zero_share to use it instead of the generic V1 helper.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 1, 0, 0);

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    if (ret == 0) {
        ret = tls_api_attempt_to_close(test_ctx, &simulated_time);
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
fn tls_zero_share() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx_zero_share(&mut simulated_time).expect("ctx");

    assert!(
        test_ctx.qclient.client_zero_share,
        "zero-share flag was not set before starting the client connection"
    );
    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("tls_zero_share connection loop");
    tls_api_close_with_losses(&mut test_ctx, &mut simulated_time, 0).expect("tls_zero_share close");
}
```

## `picoquictest/congestion_test.c:bbr1_test`
* C test-table name: `bbr1`
* C entry function: `bbr1_test`
* Rust test: `bbr1`
* Expected Rust file: `rs/fq/src/tests/congestion.rs`
* Current Rust span: `rs/fq/src/tests/congestion.rs:880-883`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust selects bbr1 and passes the same parameters, but the shared Rust scenario verifier ignores stream-completion verification and max_completion_time that C enforces.
* Phase 5A fix note: Make the Rust scenario helper verify all streams completed and enforce the 3,600,000 us completion bound; preserve C delayed-start ordering so the selected algorithm is installed before the scenario starts.
* Phase 5B analysis: Rust bbr1 is a #[test], selects the bbr1 congestion algorithm, and calls congestion_control_test with the same 3,600,000 us max time and zero jitter inputs as C. The shared helper covers the API-visible scenario contract; any scenario-body/runtime failure is Phase 5C.
* Phase 5B fix note: 

### C test body
```c
{
    return congestion_control_test(picoquic_bbr1_algorithm, 3600000, 0, 0);
}
```

### Current Rust test body
```rust
fn bbr1() {
    let ccalgo = cc_algo("bbr1");
    congestion_control_test(ccalgo, 3_600_000, 0, 0);
}
```
