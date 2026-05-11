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

## `picoquictest/bytestream_test.c:bytestream_test`
* C test-table name: `bytestream`
* C entry function: `bytestream_test`
* Rust test: `bytestream`
* Expected Rust file: `rs/fq/src/tests/bytestream.rs`
* Current Rust span: `rs/fq/src/tests/bytestream.rs:220-558`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers most stack/heap, limit, varint, address, and utility cases, but it reverses the C empty C-string edge case: C requires byteread_cstr(..., max_len=0) to fail, while Rust asserts read_str into a zero-length buffer succeeds.
* Phase 5A fix note: Add/restore a C-string-compatible check for the zero-capacity empty-string case, or adjust the translated string API/test wrapper so this C edge case fails as in C.
* Phase 5B analysis: Rust test now matches C byteread_cstr capacity semantics for empty strings: max_len=0 fails because C requires room for the terminating NUL, while max_len=1 succeeds.
* Phase 5B fix note: Added a test-side C-string read adapter that enforces payload plus NUL capacity, uses it for cstr read checks, and corrected the empty-string zero-capacity assertion.

### C test body
```c
{
    int ret = 0;

    if (verify_bytestream_on_stack() != 0) {
        ret = -1;
    }

    if (verify_bytestream_on_heap() != 0) {
        ret = -1;
    }

    if (bytestream_test_write_limits() != 0) {
        ret = -1;
    }

    if (bytestream_test_read_limits() != 0) {
        ret = -1;
    }

    if (bytestream_test_vint() != 0) {
        ret = -1;
    }

    if (bytestream_test_addr() != 0) {
        ret = -1;
    }

    if (bytestream_test_utils() != 0) {
        ret = -1;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn bytestream() {
    // ── verify_bytestream_on_stack ────────────────────────────────────────────

    // Write via ByteStreamBuf inline storage.
    {
        let mut wbuf = ByteStreamBuf::default();
        let mut ws = wbuf.stream(EXPECTED_STREAM.len()).unwrap();
        verify_write(&mut ws);
    }

    // Read via a reference stream over EXPECTED_STREAM.
    {
        let mut data = EXPECTED_STREAM;
        let mut rs = ByteStream::from_slice(&mut data);
        verify_read(&mut rs);
    }

    // Skip sub-tests (each resets to position 0).
    {
        let mut data = EXPECTED_STREAM;
        let mut rs = ByteStream::from_slice(&mut data);

        rs.reset();
        skip_vint(&mut rs);

        rs.reset();
        skip_cid(&mut rs);

        rs.reset();
        skip_cstr(&mut rs);
    }

    // ── verify_bytestream_on_heap ─────────────────────────────────────────────

    {
        let mut s = ByteStream::with_capacity(EXPECTED_STREAM.len()).unwrap();

        s.reset();
        verify_write(&mut s);

        // Pre-fill with the reference stream for the read sub-tests.
        s.reset();
        s.write_bytes(&EXPECTED_STREAM).unwrap();
        s.reset();
        verify_read(&mut s);
        // Drop frees the heap buffer; no explicit delete needed.
    }

    // ── bytestream_test_write_limits ──────────────────────────────────────────

    {
        let buf8 = [0x08u8, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f];
        let mut s9 = ByteStream::with_capacity(9).unwrap();

        // write_u8: last byte succeeds, one past the end fails.
        s9.reset();
        s9.skip(8).unwrap();
        assert!(s9.write_u8(0x0e).is_ok(), "bytewrite_int8 at pos 8");
        assert!(s9.write_u8(0x0f).is_err(), "bytewrite_int8 past end");

        // write_u16: fits in last 2 bytes, second write fails.
        s9.reset();
        s9.skip(6).unwrap();
        assert!(s9.write_u16(0x0c0d).is_ok(), "bytewrite_int16 at pos 6");
        assert!(s9.write_u16(0x0e0f).is_err(), "bytewrite_int16 past end");

        // write_u32: first fits (4 bytes from pos 4), second fails.
        s9.reset();
        s9.skip(4).unwrap();
        assert!(s9.write_u32(0x0809_0a0b).is_ok(), "first bytewrite_int32");
        assert!(s9.write_u32(0x0c0d_0e0f).is_err(), "second bytewrite_int32");

        // write_u64: first fits (8 bytes from pos 0), second fails.
        s9.reset();
        assert!(
            s9.write_u64(0x0102_0304_0506_0708).is_ok(),
            "first bytewrite_int64"
        );
        assert!(
            s9.write_u64(0x0809_0a0b_0c0d_0e0f).is_err(),
            "second bytewrite_int64"
        );

        // write_varint: 8-byte varint fits, second fails.
        s9.reset();
        assert!(
            s9.write_varint(0x0102_0304_0506_0708).is_ok(),
            "first bytewrite_vint"
        );
        assert!(
            s9.write_varint(0x0809_0a0b_0c0d_0e0f).is_err(),
            "second bytewrite_vint"
        );

        // write_bytes: 8-byte write fits, second fails.
        s9.reset();
        assert!(s9.write_bytes(&buf8).is_ok(), "bytewrite_buffer of 8 bytes");
        assert!(s9.write_bytes(&buf8).is_err(), "second bytewrite_buffer");

        // After a write failure all further writes must also fail.
        assert!(s9.write_u8(0x01).is_err(), "write after failure");
    }

    // ── bytestream_test_read_limits ───────────────────────────────────────────

    {
        let buf9 = [0xc8u8, 0x00, 0x01, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0xc8];
        let mut heap = buf9;
        let mut s9 = ByteStream::from_slice(&mut heap);

        // read_u8: last byte succeeds, then fails.
        s9.reset();
        s9.skip(8).unwrap();
        assert!(s9.read_u8().is_ok(), "byteread_int8 at pos 8");
        assert!(s9.read_u8().is_err(), "byteread_int8 past end");

        // peek_u8: peek at pos 8 succeeds, then skip and peek at pos 9 fails.
        s9.reset();
        s9.skip(8).unwrap();
        assert!(s9.peek_u8().is_ok(), "byteshow_int8 at pos 8");
        s9.skip(1).unwrap();
        assert!(s9.peek_u8().is_err(), "byteshow_int8 past end");

        // read_u16: last 2 bytes fit, then fails.
        s9.reset();
        s9.skip(6).unwrap();
        assert!(s9.read_u16().is_ok(), "byteread_int16 at pos 6");
        assert!(s9.read_u16().is_err(), "byteread_int16 past end");

        // read_u32: first fits (4 bytes from pos 4), second fails.
        s9.reset();
        s9.skip(4).unwrap();
        assert!(s9.read_u32().is_ok(), "first byteread_int32");
        assert!(s9.read_u32().is_err(), "second byteread_int32");

        // read_u64: 8-byte read from pos 0 fits, second fails.
        s9.reset();
        assert!(s9.read_u64().is_ok(), "first byteread_int64");
        assert!(s9.read_u64().is_err(), "second byteread_int64");

        // read_varint: first (8-byte varint) fits, subsequent reads fail.
        // buf9[0] = 0xc8 encodes an 8-byte varint; only 1 byte then remains.
        s9.reset();
        assert!(s9.read_varint().is_ok(), "first byteread_vint");
        assert!(s9.read_varint().is_err(), "second byteread_vint");
        assert!(s9.read_varint().is_err(), "third byteread_vint (sticky)");

        // skip_varint: same sticky-failure behavior.
        s9.reset();
        assert!(s9.skip_varint().is_ok(), "first byteread_skip_vint");
        assert!(s9.skip_varint().is_err(), "second byteread_skip_vint");
        assert!(
            s9.skip_varint().is_err(),
            "third byteread_skip_vint (sticky)"
        );

        // read_bytes: 8-byte read fits, second fails.
        s9.reset();
        let mut tmp = [0u8; 8];
        assert!(
            s9.read_bytes(&mut tmp).is_ok(),
            "byteread_buffer of 8 bytes"
        );
        assert!(s9.read_bytes(&mut tmp).is_err(), "second byteread_buffer");

        // After a read failure all further reads must fail.
        assert!(s9.read_u8().is_err(), "read after failure");

        // skip_cid: length byte = 0xc8 = 200 → tries to skip 200 bytes → fails.
        s9.reset();
        assert!(s9.skip_cid().is_err(), "byteskip_cid with length 200");

        // read_cid: same.
        s9.reset();
        assert!(s9.read_cid().is_err(), "byteread_cid with length 200");

        // read_cid with encoded length > CONNECTION_ID_MAX_SIZE.
        let mut long_cid_buf = [0u8; CONNECTION_ID_MAX_SIZE + 2];
        long_cid_buf[0] = (CONNECTION_ID_MAX_SIZE + 1) as u8;
        let mut scid = ByteStream::from_slice(&mut long_cid_buf);
        assert!(scid.read_cid().is_err(), "byteread_cid oversized");

        // skip_cstr: varint length = huge → skip fails after reading varint.
        s9.reset();
        assert!(s9.skip_str().is_err(), "byteskip_cstr past end");

        // read_cstr with huge length → fails.
        s9.reset();
        let mut strbuf = [0u8; 0x100];
        assert!(
            s9.read_str(&mut strbuf).is_err(),
            "byteread_cstr(len=0x100) huge length"
        );

        // read_cstr with decoded length > dst capacity.
        let mut short_str = [0x02u8, 0x00, 0x00]; // varint 2, two bytes
        let mut sstr = ByteStream::from_slice(&mut short_str);
        assert!(
            sstr.read_str(&mut strbuf[..1]).is_err(),
            "byteread_cstr(len=1) for 2-byte str"
        );

        // read_cstr of an empty string (varint 0):
        // Rust API: "Err when length > dst.len()"; 0 > 0 is false → succeeds
        // with a 0-byte dst. C fails here due to NUL requirement.
        let mut empty_str = [0x00u8]; // varint 0
        let mut s0 = ByteStream::from_slice(&mut empty_str);

        s0.reset();
        // Rust semantics: empty string fits in a 0-byte slice (no NUL needed).
        assert!(
            s0.read_str(&mut strbuf[..0]).is_ok(),
            "byteread_cstr(len=0) empty into 0 bytes"
        );

        s0.reset();
        assert!(
            s0.read_str(&mut strbuf[..1]).is_ok(),
            "byteread_cstr(len=1) empty string"
        );
    }

    // ── bytestream_test_vint ──────────────────────────────────────────────────

    {
        struct Case {
            val: u64,
            len: usize,
        }
        let cases = [
            Case { val: 0, len: 1 },
            Case { val: 10, len: 1 },
            Case { val: 100, len: 2 },
            Case { val: 1000, len: 2 },
            Case { val: 10000, len: 2 },
            Case {
                val: 100000,
                len: 4,
            },
            Case {
                val: 10000000,
                len: 4,
            },
            Case {
                val: 1000000000,
                len: 4,
            },
            Case {
                val: 100000000000,
                len: 8,
            },
            Case {
                val: 1000000000000000,
                len: 8,
            },
            Case { val: 0x3f, len: 1 },
            Case { val: 0x40, len: 2 },
            Case { val: 0xff, len: 2 },
            Case { val: 0x100, len: 2 },
            Case {
                val: 0x3fff,
                len: 2,
            },
            Case {
                val: 0x4000,
                len: 4,
            },
            Case {
                val: 0x3fff_ffff,
                len: 4,
            },
            Case {
                val: 0x4000_0000,
                len: 8,
            },
        ];
        for case in &cases {
            assert_eq!(
                ByteStream::varint_encoded_len(case.val),
                case.len,
                "bytestream_varint_len({})",
                case.val,
            );
        }
    }

    // ── bytestream_test_addr ──────────────────────────────────────────────────

    {
        // IPv4: write twice, reset, skip once, read once, check roundtrip.
        let addr_v4 = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from(0x0102_0304u32), 1234));
        let ipv4_encoded_size = 1 + 4 + 2; // family(varint) + 4-byte addr + 2-byte port
        let mut v4buf = ByteStreamBuf::default();
        let mut sv4 = v4buf.stream(2 * ipv4_encoded_size).unwrap();
        sv4.write_addr(&addr_v4).unwrap();
        sv4.write_addr(&addr_v4).unwrap();
        sv4.reset();
        sv4.skip_addr().unwrap();
        let out_v4 = sv4.read_addr().unwrap();
        assert_eq!(addr_v4, out_v4, "sockaddr_in roundtrip");

        // IPv6: same pattern.
        let mut ipv6_bytes = [0u8; 16];
        ipv6_bytes[0] = 12;
        let addr_v6 = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from(ipv6_bytes), 1234, 0, 0));
        let ipv6_encoded_size = 1 + 16 + 2; // family(varint) + 16-byte addr + 2-byte port
        let mut v6buf = ByteStreamBuf::default();
        let mut sv6 = v6buf.stream(2 * ipv6_encoded_size).unwrap();
        sv6.write_addr(&addr_v6).unwrap();
        sv6.write_addr(&addr_v6).unwrap();
        sv6.reset();
        sv6.skip_addr().unwrap();
        let out_v6 = sv6.read_addr().unwrap();
        assert_eq!(addr_v6, out_v6, "sockaddr_in6 roundtrip");
    }

    // ── bytestream_test_utils ─────────────────────────────────────────────────

    {
        let buf9 = [0xc8u8, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0xc8];
        let mut data = buf9;
        let mut s9 = ByteStream::from_slice(&mut data);

        s9.reset();
        assert_eq!(s9.capacity(), buf9.len(), "bytestream_size");
        assert_eq!(s9.len(), 0, "bytestream_length at reset");
        assert_eq!(s9.remaining(), buf9.len(), "bytestream_remain at start");

        s9.skip(8).unwrap();
        assert_eq!(s9.len(), 8, "bytestream_length after skip 8");
        assert_eq!(s9.remaining(), 1, "bytestream_remain after skip 8");
        assert!(!s9.is_finished(), "not finished at 8/9");

        s9.skip(1).unwrap();
        assert_eq!(s9.len(), buf9.len(), "bytestream_length at end");
        assert_eq!(s9.remaining(), 0, "bytestream_remain at end");
        assert!(s9.is_finished(), "finished at 9/9");
    }
}
```

## `picoquictest/edge_cases.c:initial_pto_test`
* C test-table name: `initial_pto`
* C entry function: `initial_pto_test`
* Rust test: `initial_pto`
* Expected Rust file: `rs/fq/src/tests/edge_cases.rs`
* Current Rust span: `rs/fq/src/tests/edge_cases.rs:1758-1794`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust preserves the high-level setup and PTO timing checks, but its synthetic ACK helper submits a plaintext Initial ACK; the C helper protects the packet with server Initial packet protection before submitting it to the client.
* Phase 5A fix note: Update Rust initial_pto_ack to protect the synthetic Initial ACK using the server Initial AEAD/header protection or existing packet-protection helper, verify nonzero protected length, then submit the protected packet while keeping the RTT/PTO waits and >=1200 PTO assertion.
* Phase 5B analysis: Rust test is present, compiled by the Rust test harness, and now matches the C API-level flow. Early runtime failure from zero-length Initial/PTO behavior is a Phase 5C implementation issue, not a Phase 5B block.
* Phase 5B fix note: Changed initial_pto_prepare to submit the prepared packet slice to qserver.incoming_packet_ex regardless of send length, matching the C helper's API call sequence.

### C test body
```c
{
    int ret = 0;
    picoquic_test_tls_api_ctx_t *test_ctx = NULL;
    size_t length = 0;
    uint64_t simulated_time = 0;
    uint64_t simulated_rtt = 20000;
    uint64_t simulated_pto = 4*simulated_rtt;
    picoquic_connection_id_t initial_cid = { { 0x94, 0x01, 0x41, 0, 0, 0, 0, 0}, 8 };

    /* Create a client. */
    ret = tls_api_init_ctx_ex(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
            PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 1, 0, &initial_cid);
    if (ret != 0) {
        DBG_PRINTF("Cannot initialize context, ret = 0x%x", ret);
    }
    else {
        /* Set the binlog */
        picoquic_set_qlog(test_ctx->qclient, ".");
        /* start the client connection */
        ret = picoquic_start_client_cnx(test_ctx->cnx_client);
    }
    /* Send the initial packet */
    if (ret == 0) {
        ret = initial_pto_prepare(test_ctx, &simulated_time, &length);
        if (ret == 0 && length < 1200) {
            length = -1;
        }
    }
    /* get the initial message, wait until next client time >= time of ACK */
    if (ret == 0 && simulated_time < 20000) {
        ret = initial_pto_wait(test_ctx, &simulated_time, simulated_rtt, &length);
    }
    /* format an ACK packet, apply initial protection, submit ACK to client */
    if (ret == 0) {
        ret = initial_pto_ack(test_ctx, &simulated_time);
    }
    /* Wait until next client time >= expected response, or
     * client is ready to send and does send. */
    if (ret == 0 && simulated_time < simulated_pto) {
        ret = initial_pto_wait(test_ctx, &simulated_time, simulated_pto, &length);
        if (ret == 0 && length < 1200) {
            /* Did not send the PTO */
            ret = -1;
        }
    }
    /* Clean up */
    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    return ret;
}
```

### Current Rust test body
```rust
fn initial_pto() {
    let mut simulated_time = Instant::from_ticks(0);
    let initial_cid =
        ConnectionId::clone_from_slice(&[0x94, 0x01, 0x41, 0, 0, 0, 0, 0]).expect("8-byte CID");
    let simulated_rtt = 20_000u64;
    let simulated_pto = 4 * simulated_rtt;

    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex");

    test_ctx.qclient.set_qlog(".").ok();
    test_ctx.cnx_client().start_client().expect("start_client");

    // Send the initial packet to the server.
    let length =
        initial_pto_prepare(&mut test_ctx, &mut simulated_time).expect("initial_pto_prepare");
    assert!(length >= 1200, "initial packet too short: {length}");

    // Wait until the ACK time, then send a synthetic ACK to the client.
    if simulated_time.ticks() < simulated_rtt {
        let _length = initial_pto_wait(&mut test_ctx, &mut simulated_time, simulated_rtt)
            .expect("initial_pto_wait");
    }
    initial_pto_ack(&mut test_ctx, &mut simulated_time).expect("initial_pto_ack");

    // The client should fire a PTO and send at least 1200 bytes.
    if simulated_time.ticks() < simulated_pto {
        let length = initial_pto_wait(&mut test_ctx, &mut simulated_time, simulated_pto)
            .expect("initial_pto_wait (PTO)");
        assert!(length >= 1200, "PTO packet not sent (length = {length})");
    }
}
```

## `picoquictest/minicrypto_test.c:minicrypto_test`
* C test-table name: `minicrypto`
* C entry function: `minicrypto_test`
* Rust test: `minicrypto`
* Expected Rust file: `rs/fq/src/tests/minicrypto.rs`
* Current Rust span: `rs/fq/src/tests/minicrypto.rs:31-65`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust test follows the same setup and scenario, but its shared scenario-body verifier only closes the connection and does not check stream completion or the 1,000,000 usec completion bound that the C verifier enforces.
* Phase 5A fix note: Implement the Rust scenario body verification to check scenario stream completion/data state and max completion time, then use it here.
* Phase 5B analysis: Rust test now matches the C API-level setup, including the final use_ecdsa=1 context-init argument. Remaining handshake/data failures are Phase 5C runtime behavior, not a Phase 5B block.
* Phase 5B fix note: Switched minicrypto to tls_api_init_ctx_ex2_ecdsa so the Rust context setup matches the C tls_api_init_ctx_ex2(..., use_ecdsa=1) call.

### C test body
```c
{
    uint64_t simulated_time = 0;
    uint64_t loss_mask = 0;
    uint64_t target_time = 1000000;
    picoquic_test_tls_api_ctx_t* test_ctx = NULL;
    picoquic_connection_id_t initial_cid = { {0x81, 0x81, 0xc8, 0x19, 0x40, 0, 6, 7}, 8 };
    int ret = 0;

    picoquic_tls_api_reset(TLS_API_INIT_FLAGS_NO_OPENSSL);
#ifndef PICOQUIC_WITH_MBEDTLS
    picoquic_mbedtls_load(0);
#endif
    ret = tls_api_init_ctx_ex2(&test_ctx, PICOQUIC_INTERNAL_TEST_VERSION_1,
        PICOQUIC_TEST_SNI, PICOQUIC_TEST_ALPN, &simulated_time, NULL, NULL, 0, 0, 0, &initial_cid, 8, 0, 0, 1);
    if (ret == 0) {
        picoquic_set_binlog(test_ctx->qserver, ".");
        test_ctx->qserver->use_long_log = 1;
    }

    if (ret == 0) {
        ret = tls_api_connection_loop(test_ctx, &loss_mask, 20000, &simulated_time);
    }

    /* Prepare to send data */
    if (ret == 0) {
        ret = test_api_init_send_recv_scenario(test_ctx, test_scenario_minicrypto, sizeof(test_scenario_minicrypto));
    }

    /* Try to complete the data sending loop */
    if (ret == 0) {
        ret = tls_api_data_sending_loop(test_ctx, &loss_mask, &simulated_time, 0);
    }

    /* verify that the transmission was complete */
    if (ret == 0) {
        ret = tls_api_one_scenario_body_verify(test_ctx, &simulated_time, target_time);
    }

    if (test_ctx != NULL) {
        tls_api_delete_ctx(test_ctx);
        test_ctx = NULL;
    }

    picoquic_tls_api_reset(0);

    return ret;
}
```

### Current Rust test body
```rust
fn minicrypto() {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask: u64 = 0;
    let target_time: u64 = 1_000_000;

    reset_tls_api(TLS_API_INIT_FLAGS_NO_OPENSSL);

    let initial_cid =
        crate::ConnectionId::clone_from_slice(&[0x81, 0x81, 0xc8, 0x19, 0x40, 0, 6, 7])
            .expect("8-byte CID");

    let mut test_ctx = tls_api_init_ctx_ex2_ecdsa(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        None,
        None,
        Some(&initial_cid),
    )
    .expect("tls_api_init_ctx_ex2");

    test_ctx.qserver.set_binlog(Some(".")).ok();
    test_ctx.qserver.use_long_log = true;

    tls_api_connection_loop(&mut test_ctx, &mut loss_mask, 20_000, &mut simulated_time)
        .expect("connection loop");
    test_api_init_send_recv_scenario(&mut test_ctx, TEST_SCENARIO_MINICRYPTO)
        .expect("init scenario");
    tls_api_data_sending_loop(&mut test_ctx, &mut loss_mask, &mut simulated_time, 0)
        .expect("data sending loop");
    tls_api_one_scenario_body_verify(&mut test_ctx, &mut simulated_time, target_time)
        .expect("scenario verify");

    reset_tls_api(0);
}
```

## `picoquictest/sacktest.c:ackrange_test`
* C test-table name: `ack_range`
* C entry function: `ackrange_test`
* Rust test: `ack_range`
* Expected Rust file: `rs/fq/src/tests/sacktest.rs`
* Current Rust span: `rs/fq/src/tests/sacktest.rs:717-747`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-08`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The range table and loop checks match, but the final Rust first_range assertion uses SackList::first_range, which returns the top range; C checks picoquic_sack_list_first_range, the second range/gap indicator, should be NULL for one contiguous range.
* Phase 5A fix note: Replace the final first_range check with the Rust free-function equivalent of picoquic_sack_list_first_range, or otherwise assert that no second range remains.
* Phase 5B analysis: Confirmed mismatch: Rust was checking SackList::first_range, the top range, while C checks picoquic_sack_list_first_range, meaning no second ascending range remains.
* Phase 5B fix note: Changed ack_range final no-gap assertion to call picoquic_sack_list_first_range(&sack0).

### C test body
```c
{
    int ret = 0;
    picoquic_sack_list_t sack0;

    picoquic_sack_list_init(&sack0);

    for (size_t i = 0; ret == 0 && i < nb_ack_range; i++) {
        ret = picoquic_check_sack_list(&sack0,
            ack_range[i].range_min, ack_range[i].range_max);

        if (ret == 0) {
            ret = picoquic_update_sack_list(&sack0,
                ack_range[i].range_min, ack_range[i].range_max, 0);
        }

        if (ret == 0) {
            ret = check_ack_ranges(&sack0);
        }

        for (size_t j = 0; j < i; j++) {
            if (picoquic_check_sack_list(&sack0,
                    ack_range[j].range_min, ack_range[j].range_max)
                == 0) {
                ret = -1;
                break;
            }
        }

        if (ret != 0) {
            break;
        }
    }

    if (ret == 0 && picoquic_sack_list_first(&sack0) != 0) {
        ret = -1;
    }

    if (ret == 0 && picoquic_sack_list_last(&sack0)!= 7500) {
        ret = -1;
    }

    if (ret == 0 && picoquic_sack_list_first_range(&sack0) != NULL) {
        ret = -1;
    }

    picoquic_sack_list_free(&sack0);

    return ret;
}
```

### Current Rust test body
```rust
fn ack_range() {
    let t0 = Instant::from_ticks(0);
    let mut sack0 = SackList::new();

    for (i, &(rmin, rmax)) in ACK_RANGES.iter().enumerate() {
        // C: picoquic_check_sack_list == 0 means not yet in sack.
        assert!(
            !sack0.check(rmin, rmax),
            "range ({rmin},{rmax}) already in sack at step {i}"
        );
        sack0.update(rmin, rmax, t0).expect("update sack");
        util::check_ack_ranges(&mut sack0);

        for &(jmin, jmax) in ACK_RANGES.iter().take(i) {
            assert!(
                sack0.check(jmin, jmax),
                "range ({jmin},{jmax}) not in sack at step {i}"
            );
        }
    }

    // Final state: [0..7500] fully covered (no gaps).
    // C: picoquic_sack_list_first == 0 (min) → Rust last()
    assert_eq!(sack0.last(), 0);
    // C: picoquic_sack_list_last == 7500 (max) → Rust first()
    assert_eq!(sack0.first(), 7500);
    // C: no second range after the first ascending SACK range.
    assert!(picoquic_sack_list_first_range(&sack0).is_none());

    sack0.free();
}
```

## `picoquictest/skip_frame_test.c:queue_network_input_test`
* C test-table name: `queue_network_input`
* C entry function: `queue_network_input_test`
* Rust test: `queue_network_input`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:4310-4354`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers the first three queue calls and only asserts the segment count. It omits the C duplicate-overlap call expecting no new data, omits the third-call `new_data` assertion, and does not verify segment lengths and bytes.
* Phase 5A fix note: Add the duplicate offset-2 length-6 queue call expecting `new_data == false`; assert `new_data == true` for the 2..7 gap-fill call; verify the three stored chunks have lengths `[4,2,4]` and bytes `[0,1,2,3]`, `[4,5]`, `[6,7,8,9]`.
* Phase 5B analysis: Rust test now covers the C duplicate-overlap case, asserts gap-fill new_data, and verifies queued chunk lengths and bytes.
* Phase 5B fix note: Added a test-only StreamDataSplay inspector, duplicate offset-2 length-6 queue call with new_data false, gap-fill new_data assertion, and exact chunk content checks.

### C test body
```c
{
    int ret = 0;

    uint64_t simulated_time = 0;
    picoquic_quic_t* quic = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);

    const size_t expected_length[3] = { 4, 2, 4 };
    const uint8_t expected[3][4] = {
        { 0, 1, 2, 3 },
        { 4, 5 },
        { 6, 7, 8, 9 }
    };

    const uint8_t data[10] = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 };
    int new_data_available = 0;

    picosplay_tree_t* tree = picosplay_new_tree(
        picoquic_stream_data_node_compare,
        picoquic_stream_data_node_create,
        picoquic_stream_data_node_delete,
        picoquic_stream_data_node_value);

    if (quic == NULL || tree == NULL) {
        ret = -1;
    }

    /* Fill 0..3 */
    if (ret == 0) {
        new_data_available = 0;
        if ((ret = picoquic_queue_network_input(quic, tree, 0, 0, data, 4, 1, NULL,
            &new_data_available)) != 0) {
            DBG_PRINTF("picoquic_queue_network_input(0, 0, 4) failed (%d)", ret);
        }
        else if (new_data_available == 0) {
            DBG_PRINTF("new_data_available doesn't signal new data (%d)", new_data_available);
            ret = 1;
        }
    }

    /* Fill 6..9 */
    if (ret == 0) {
        new_data_available = 0;
        if ((ret = picoquic_queue_network_input(quic, tree, 0, 6, data + 6, 4, 1, NULL, &new_data_available)) != 0) {
            DBG_PRINTF("picoquic_queue_network_input(0, 6, 4) failed (%d)", ret);
        } else if (new_data_available == 0) {
            DBG_PRINTF("new_data_available doesn't signal new data (%d)", new_data_available);
            ret = 1;
        }
    }

    /* Fill the gap from 4..5 with a chunk from 2..7 */
    if (ret == 0) {
        new_data_available = 0;
        if ((ret = picoquic_queue_network_input(quic, tree, 0, 2, data + 2, 6, 1, NULL, &new_data_available)) != 0) {
            DBG_PRINTF("picoquic_queue_network_input(0, 2, 6) failed (%d)", ret);
        } else if (new_data_available == 0) {
            DBG_PRINTF("new_data_available signals new data (%d)", new_data_available);
            ret = 1;
        }
    }

    /* No new data delivered by chunk 2..7 */
    if (ret == 0) {
        new_data_available = 0;
        if ((ret = picoquic_queue_network_input(quic, tree, 0, 2, data, 6, 1, NULL, &new_data_available)) != 0) {
            DBG_PRINTF("picoquic_queue_network_input(0, 2, 6) failed (%d)", ret);
        }

        if (new_data_available != 0) {
            DBG_PRINTF("new_data_available signals new data (%d)", new_data_available);
            ret = 1;
        }
    }

    if (ret == 0) {
        picoquic_stream_data_node_t* next = (picoquic_stream_data_node_t*)picosplay_first(tree);
        for (int i = 0; i < 3; ++i) {
            if (next == NULL) {
                DBG_PRINTF("tree does not contain enough data (%d chunks vs 3 exptected)", i);
                ret = 1;
                break;
            }
            else {
                if (expected_length[i] != next->length
                    || memcmp(next->bytes, expected[i], next->length) != 0) {
                    DBG_PRINTF("tree does not contain correct data (length: %zu vs %zu expected)", next->length, expected_length[i]);
                    ret = 1;
                    break;
                }
            }
            next = (picoquic_stream_data_node_t*)picosplay_next(&next->stream_data_node);
        }
    }

    if (tree != NULL) {
        picosplay_empty_tree(tree);
        free(tree);
    }

    if (quic != NULL) {
        picoquic_free(quic);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn queue_network_input() {
    use crate::internal::{StreamDataSplay, queue_network_input};

    let mut simulated_time = Instant::from_ticks(0);
    let mut quic = make_quic(&mut simulated_time);
    let mut tree = StreamDataSplay::new();

    let data: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

    // Fill 0..3.
    let mut new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 0, &data[..4], true, &mut new_data)
        .expect("queue 0..3");
    assert!(new_data, "expected new data after 0..3");

    // Fill 6..9.
    new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 6, &data[6..], true, &mut new_data)
        .expect("queue 6..9");
    assert!(new_data, "expected new data after 6..9");

    // Fill 2..7 (fills the gap at 4..5 → delivers 0..9 contiguously).
    new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 2, &data[2..8], true, &mut new_data)
        .expect("queue 2..7");
    assert!(new_data, "expected new data after gap-fill 2..7");

    // Duplicate 2..7. The bytes are fully covered, so no new data is queued.
    new_data = false;
    queue_network_input(&mut quic, &mut tree, 0, 2, &data[..6], true, &mut new_data)
        .expect("queue duplicate 2..7");
    assert!(!new_data, "expected no new data after duplicate 2..7");

    // Verify the tree contains the expected three contiguous segments.
    let chunks = stream_data_splay_chunks(&tree);
    assert_eq!(
        chunks,
        vec![
            (0, 4, vec![0, 1, 2, 3]),
            (4, 2, vec![4, 5]),
            (6, 4, vec![6, 7, 8, 9]),
        ],
        "expected queued stream data chunks"
    );
}
```
