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

## `picoquictest/tls_api_test.c:qlog_trace_test`
* C test-table name: `qlog_trace`
* C entry function: `qlog_trace_test`
* Rust test: `qlog_trace`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6569-6571`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust does not test qlog generation or reference matching; it runs a tiny generic scenario with ECN stored in test context only, missing deterministic CID, qlog setup, policies, loss pattern, bad-packet logging, and file comparison.
* Phase 5A fix note: Implement qlog_trace_test_one to mirror the C qlog trace setup, run q2_and_r2 with loss mask 0x00010a04, inject the bad packet, close the log, and compare generated qlog output to the expected reference while honoring ECN/parallel variants.
* Phase 5B analysis: Rust #[test] qlog_trace calls qlog_trace_test_one(0, false), matching the C entry qlog_trace_test_one(0, 0). The helper expresses the same API-level qlog setup, deterministic CID/reset/cipher configuration, q2_and_r2 loss scenario, bad-packet injection, and reference comparison. Any missing qlog generation/runtime failure is Phase 5C, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    return qlog_trace_test_one(0, 0);
}
```

### Current Rust test body
```rust
fn qlog_trace() {
    qlog_trace_test_one(0, false).expect("qlog_trace");
}
```

## `picoquictest/tls_api_test.c:ready_to_zero_test`
* C test-table name: `ready_to_zero`
* C entry function: `ready_to_zero_test`
* Rust test: `ready_to_zero`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:6824-6826`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust calls ready_to_send_test_one(4), but that helper ignores the option and does not model the C stream0 prepare-to-send zero-byte path or C scenario sizes.
* Phase 5A fix note: Implement ready_to_send_test_one so option 4 drives the stream0 zero-byte prepare-to-send behavior, uses the C q_and_r scenario, and verifies completion like the C helper.
* Phase 5B analysis: Rust ready_to_zero is present as a Rust #[test] and calls ready_to_send_test_one(4), matching the C wrapper. The shared helper already expresses the C q_and_r stream0 scenario; any CannotSetActiveStream or zero-byte prepare-to-send failure is a Phase 5C runtime/library gap, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = ready_to_send_test_one(4);
    return ret;
}
```

### Current Rust test body
```rust
fn ready_to_zero() {
    ready_to_send_test_one(4).expect("ready_to_zero");
}
```

## `picoquictest/tls_api_test.c:tls_retry_token_test`
* C test-table name: `retry_token`
* C entry function: `tls_retry_token_test`
* Rust test: `retry_token`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7060-7064`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-03`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: C runs three cases: (1,false), (2,false), and (1,true). Rust only runs (1,false), and its helper is a single cookie-mode connection/close without token-file reset/load/save, second connection, reuse validation, or duplicate-token checks.
* Phase 5A fix note: Have retry_token run all three C cases and implement tls_retry_token_test_one's token-file, reconnect, stored-token reuse, and duplicate-token rejection/new-token checks.
* Phase 5B analysis: Rust test is present, compiles as a harness test, and matches the C entry sequence and API-visible retry-token contract. Any failure where the second connection retries instead of consuming a stored token is a Phase 5C library behavior issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    int ret = tls_retry_token_test_one(1,0);

    if (ret != 0) {
        DBG_PRINTF("Retry token test returns %d", ret);
    }
    else {
        ret = tls_retry_token_test_one(2,0);

        if (ret != 0) {
            DBG_PRINTF("Provide token test returns %d", ret);
        }
        else {
            ret = tls_retry_token_test_one(1, 1);
            if (ret != 0){
                DBG_PRINTF("Duplicate token test returns %d", ret);
            }
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn retry_token() {
    tls_retry_token_test_one(1, false).expect("retry_token retry-required");
    tls_retry_token_test_one(2, false).expect("retry_token provide-token");
    tls_retry_token_test_one(1, true).expect("retry_token duplicate");
}
```

## `picoquictest/tls_api_test.c:stateless_reset_client_test`
* C test-table name: `stateless_reset_client`
* C entry function: `stateless_reset_client_test`
* Rust test: `stateless_reset_client`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:7803-7852`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only performs a normal handshake/final close. The C test injects a bogus short packet with a mutated server-side local CID into qserver and asserts the server connection stays no later than ready.
* Phase 5A fix note: After connection setup, build the 256-byte bogus reset-like packet with mutated server local CID, submit it to qserver.incoming_packet from client to server, and assert the server remains present and not beyond Ready.
* Phase 5B analysis: Rust test is present, compiles/runs under the Rust test harness, and matches the C API-level contract: init, connection loop, bogus 256-byte short packet using mutated server-local CID, qserver.incoming_packet, and server still present/not past Ready. Failure to obtain an accepted server connection after the loop is a Phase 5C runtime implementation issue, not a Phase 5B block.
* Phase 5B fix note: 

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    int ret = tls_api_init_ctx(&test_ctx, 0, PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0);
    uint8_t buffer[256];

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 0, &simulated_time);
    }

    /* Prepare the bogus reset */
    if (ret == 0) {
        size_t byte_index = 0;
        buffer[byte_index++] = 0x41;
        /* Copy the client ID */
        byte_index += picoquic_format_connection_id(&buffer[byte_index], sizeof(buffer) - byte_index, test_ctx->cnx_server->path[0]->first_tuple->p_local_cnxid->cnx_id);
        /* Change one byte of the client ID */
        if (byte_index > 5) {
            buffer[5] ^= 0xff;
        }
        else {
            buffer[1] ^= 0xff;
        }
        /* rest of packet is null */
        memset(buffer + byte_index, 0xcc, sizeof(buffer) - byte_index);

        /* Submit bogus request to server */
        ret = picoquic_incoming_packet(test_ctx->qserver, buffer, sizeof(buffer),
            (struct sockaddr*)(&test_ctx->client_addr),
            (struct sockaddr*)(&test_ctx->server_addr), 0, test_ctx->recv_ecn_server,
            simulated_time);
    }

    /* check that the server is still up */
    if (ret == 0 && test_ctx->cnx_server != NULL && test_ctx->cnx_server->cnx_state > picoquic_state_ready) {
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
fn stateless_reset_client() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0u64;
    let mut test_ctx = tls_api_init_ctx(&mut simulated_time, 0, None).expect("ctx");

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 0, &mut simulated_time)
        .expect("connection loop");
    assert!(
        test_ctx.has_cnx_server(),
        "server connection was not accepted after connection loop"
    );

    let local_cid = test_ctx.cnx_server().local_cnxid();
    let mut buffer = [0u8; 256];
    let mut byte_index = 0usize;
    buffer[byte_index] = 0x41;
    byte_index += 1;
    byte_index += crate::utils::format_connection_id(&mut buffer[byte_index..], local_cid) as usize;
    if byte_index > 5 {
        buffer[5] ^= 0xff;
    } else {
        buffer[1] ^= 0xff;
    }
    buffer[byte_index..].fill(0xcc);

    let client_addr = test_ctx.client_addr;
    let server_addr = test_ctx.server_addr;
    test_ctx
        .qserver
        .incoming_packet(
            &mut buffer,
            &client_addr,
            &server_addr,
            0,
            0,
            simulated_time,
        )
        .expect("incoming bogus reset-like packet");

    let server_state = test_ctx.qserver.first_cnx_mut().map(|cnx| cnx.state());
    assert!(
        server_state.is_some(),
        "server connection disappeared after bogus reset-like packet"
    );
    let server_state = server_state.expect("server connection state");
    assert!(
        server_state <= State::Ready,
        "server connection advanced past ready: {server_state:?}"
    );
}
```

## `picoquictest/tls_api_test.c:tls_api_q2_and_r2_stream_test`
* C test-table name: `tls_api_q2_and_r2_stream`
* C entry function: `tls_api_q2_and_r2_stream_test`
* Rust test: `tls_api_q2_and_r2_stream`
* Expected Rust file: `rs/fq/src/tests/tls_api.rs`
* Current Rust span: `rs/fq/src/tests/tls_api.rs:8097-8111`
* Baseline outcome: `ok`
* Baseline source: `/private/tmp/picoquic-5b-04`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust passes an empty scenario and 75000 completion target. C uses two stream descriptors {4,0,257,2000} and {8,0,531,11000} with an 86000 target.
* Phase 5A fix note: Use a Rust TestApiStreamDesc slice matching test_scenario_q2_and_r2 and pass max_completion_microsec=86000.
* Phase 5B analysis: Rust test is present as a #[test], compiles under the Rust test harness, and matches the C API-level contract: q2/r2 scenario table, zeroed scenario controls, proposed_version 0 via context init, and 86_000us target. Prior Error::Generic runtime failure is Phase 5C implementation behavior, not a Phase 5B blocker.
* Phase 5B fix note: 

### C test body
```c
{
    return tls_api_one_scenario_test(test_scenario_q2_and_r2, sizeof(test_scenario_q2_and_r2), 0, 0, 0, 0, 0, 86000, NULL, NULL);
}
```

### Current Rust test body
```rust
fn tls_api_q2_and_r2_stream() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, 0, None).expect("ctx");
    tls_api_one_scenario_body(
        &mut ctx,
        &mut t,
        TEST_SCENARIO_Q2_AND_R2,
        0,
        0,
        0,
        0,
        86_000,
    )
    .expect("q2_and_r2_stream");
}
```
