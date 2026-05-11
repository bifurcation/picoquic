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

Owned Rust test file(s): `rs/fq/src/tests/ack_frequency.rs`, `rs/fq/src/tests/bytestream.rs`, `rs/fq/src/tests/cert_verify.rs`

Return final JSON with this shape:

```json
{"repairs":[{"test_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/tests/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquictest/ack_frequency_test.c:ackfrq_basic_test`
* C test-table name: `ackfrq_basic`
* C entry function: `ackfrq_basic_test`
* Rust test: `ackfrq_basic`
* Expected Rust file: `rs/fq/src/tests/ack_frequency.rs`
* Rust span: `rs/fq/src/tests/ack_frequency.rs:174-187`
* Phase 5A analysis: Entry parameters match C, but the Rust scenario verifier only closes the connection and does not perform C's stream/body completion checks before the ACK-frequency assertions.
* Phase 5A fix note: Make the Rust scenario verify helper check stream byte counts/received flags and callback errors like C tls_api_one_scenario_verify; target_time is 0 here, so no time-limit check is needed for this entry.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust entry parameters and ACK assertions match C, but final-tree setup is not runnable/faithful: cubic lookup lacks registry initialization and the merged scenario verifier references receive/error fields absent from the current harness structs.
* Phase 5C fix note: Restore congestion registry init and complete the util verifier merge by adding/initializing the receive counters, received flags, and callback-error flags used by tls_api_one_scenario_body_verify.

### C test body
```c
{
    ackfrq_test_spec_t spec = { 0 };
    spec.test_id = ackfrq_test_basic;
    spec.latency = 10000;
    spec.picosec_per_byte_up = 80000;
    spec.picosec_per_byte_down = 80000;
    spec.ccalgo = picoquic_cubic_algorithm;
    spec.max_ack_delay_remote = 6000;
    spec.max_ack_gap_remote = 40;
    spec.min_ack_delay_remote = 1000;
    spec.target_interval = 4000;

    return ackfrq_test_one(&spec);
}
```

### Current Rust test body
```rust
fn ackfrq_basic() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    ackfrq_test_one(&AckfrqTestSpec {
        latency: 10_000,
        picosec_per_byte_up: 80_000,
        picosec_per_byte_down: 80_000,
        ccalgo,
        target_time: 0,
        max_ack_delay_remote: Duration::from_ticks(6_000),
        max_ack_gap_remote: 40,
        min_ack_delay_remote: Duration::from_ticks(1_000),
        target_interval: Duration::from_ticks(4_000),
    });
}
```

## `picoquictest/ack_frequency_test.c:ackfrq_short_test`
* C test-table name: `ackfrq_short`
* C entry function: `ackfrq_short_test`
* Rust test: `ackfrq_short`
* Expected Rust file: `rs/fq/src/tests/ack_frequency.rs`
* Rust span: `rs/fq/src/tests/ack_frequency.rs:195-208`
* Phase 5A analysis: Entry parameters and ACK-frequency assertions match C, but the Rust scenario verifier only closes connections and does not perform the C stream/error completion checks.
* Phase 5A fix note: Make the Rust scenario verifier check stream completion/callback errors like C tls_api_one_scenario_verify, then close; preserve target_time handling.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust entry parameters match C and call the C-style scenario verifier, but the final tree lost required harness pieces, so the faithful verifier cannot compile/run as merged.
* Phase 5C fix note: Restore the Phase 5B ACK-frequency repair: congestion registry init plus complete TestApiStream/TestTlsApiCtx verifier fields and initialization.

### C test body
```c
{
    ackfrq_test_spec_t spec = { 0 };
    spec.test_id = ackfrq_test_basic;
    spec.latency = 10;
    spec.picosec_per_byte_up = 80000;
    spec.picosec_per_byte_down = 80000;
    spec.ccalgo = picoquic_cubic_algorithm;
    spec.max_ack_delay_remote = 1000;
    spec.max_ack_gap_remote = 32;
    spec.min_ack_delay_remote = 1000;
    spec.target_interval = 1000;

    return ackfrq_test_one(&spec);
}
```

### Current Rust test body
```rust
fn ackfrq_short() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    ackfrq_test_one(&AckfrqTestSpec {
        latency: 10,
        picosec_per_byte_up: 80_000,
        picosec_per_byte_down: 80_000,
        ccalgo,
        target_time: 0,
        max_ack_delay_remote: Duration::from_ticks(1_000),
        max_ack_gap_remote: 32,
        min_ack_delay_remote: Duration::from_ticks(1_000),
        target_interval: Duration::from_ticks(1_000),
    });
}
```

## `picoquictest/bytestream_test.c:bytestream_test`
* C test-table name: `bytestream`
* C entry function: `bytestream_test`
* Rust test: `bytestream`
* Expected Rust file: `rs/fq/src/tests/bytestream.rs`
* Rust span: `rs/fq/src/tests/bytestream.rs:220-558`
* Phase 5A analysis: Rust covers most stack/heap, limit, varint, address, and utility cases, but it reverses the C empty C-string edge case: C requires byteread_cstr(..., max_len=0) to fail, while Rust asserts read_str into a zero-length buffer succeeds.
* Phase 5A fix note: Add/restore a C-string-compatible check for the zero-capacity empty-string case, or adjust the translated string API/test wrapper so this C edge case fails as in C.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Final Rust tree still uses read_str directly for the empty C-string case and asserts a zero-length destination succeeds; C requires byteread_cstr(..., max_len=0) to fail because there is no room for the terminating NUL.
* Phase 5C fix note: Restore a C-string-compatible read check/adapter so empty payload with capacity 0 fails and capacity 1 succeeds.

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

## `picoquictest/cert_verify_test.c:cert_verify_bad_cert_test`
* C test-table name: `cert_verify_bad_cert`
* C entry function: `cert_verify_bad_cert_test`
* Rust test: `cert_verify_bad_cert`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Rust span: `rs/fq/src/tests/cert_verify.rs:42-50`
* Phase 5A analysis: The bad cert/key/CA/SNI inputs match, but Rust compares only tls_api_connection_loop Result. C treats success as loop OK plus both endpoints ready, so rejection also includes loop OK with not-ready endpoints.
* Phase 5A fix note: In cert_verify_test_one, compute success as result.is_ok() && client_ready && server_ready, then compare that boolean to expect_success.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: Rust uses the right bad-cert inputs, but the helper compares only tls_api_connection_loop result; C success semantics are loop OK plus client and server ready, so loop-OK/not-ready rejection is missed.
* Phase 5C fix note: Restore success = result.is_ok() && client_ready && server_ready before comparing with expect_success.

### C test body
```c
{
    int ret = cert_verify_test_one(0, CERT_VERIFY_RSA_BAD_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_bad_cert() {
    cert_verify_test_one(
        false,
        Some(TEST_FILE_SERVER_BAD_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_SNI),
    );
}
```

## `picoquictest/cert_verify_test.c:cert_verify_bad_sni_test`
* C test-table name: `cert_verify_bad_sni`
* C entry function: `cert_verify_bad_sni_test`
* Rust test: `cert_verify_bad_sni`
* Expected Rust file: `rs/fq/src/tests/cert_verify.rs`
* Rust span: `rs/fq/src/tests/cert_verify.rs:54-62`
* Phase 5A analysis: The fixture inputs match, but the Rust helper expects tls_api_connection_loop to return Err, while C treats either loop error or not-both-ready as the expected rejection.
* Phase 5A fix note: Mirror C expect_success logic: after the loop, classify lack of client/server ready as failure/rejection; success should require both ready.
* Phase 5C outcome: needs_fix
* Phase 5C analysis: The bad-SNI fixture matches C, but cert_verify_test_one only treats tls_api_connection_loop Err as rejection; C also treats Ok with not-both-ready as rejection.
* Phase 5C fix note: Classify success as loop Ok plus client_ready and server_ready; classify rejection as loop Err or Ok with either side not ready.

### C test body
```c
{
    int ret = cert_verify_test_one(0, CERT_VERIFY_RSA_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_BAD_SNI);
    return ret;
}
```

### Current Rust test body
```rust
fn cert_verify_bad_sni() {
    cert_verify_test_one(
        false,
        Some(TEST_FILE_SERVER_CERT),
        Some(TEST_FILE_SERVER_KEY),
        Some(TEST_FILE_CERT_STORE),
        Some(TEST_BAD_SNI),
    );
}
```
