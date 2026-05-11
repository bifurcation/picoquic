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

## `picoquictest/bytestream_test.c:bytestream_test`
* C test-table name: `bytestream`
* C entry function: `bytestream_test`
* Rust test: `bytestream`
* C source: `picoquictest/bytestream_test.c:781-814`
* Rust source: `rs/fq/src/tests/bytestream.rs:220-558`

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

### Rust test body
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

## `picoquictest/cleartext_aead_test.c:draft17_vector_test`
* C test-table name: `draft17_vector`
* C entry function: `draft17_vector_test`
* Rust test: `draft17_vector`
* C source: `picoquictest/cleartext_aead_test.c:914-1040`
* Rust source: `rs/fq/src/tests/cleartext_aead.rs:494-653`

### C test body
```c
{
    int ret = 0;
    int version_index = 0;
    ptls_iovec_t salt;
    uint8_t master_secret[256];
    uint8_t client_secret[256];
    uint8_t server_secret[256];
    ptls_cipher_suite_t* cipher = (ptls_cipher_suite_t*)picoquic_get_aes128gcm_sha256_v(0);

    if (cipher == NULL) {
        DBG_PRINTF("%s", "Could not find the default cipher suite.");
        ret = -1;
    }
    else {
        /* Check the label expansions */
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_KEY, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_server_initial_secret, sizeof(draft17_test_server_initial_secret),
            draft17_test_server_key, sizeof(draft17_test_server_key));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_IV, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_server_initial_secret, sizeof(draft17_test_server_initial_secret),
            draft17_test_server_iv, sizeof(draft17_test_server_iv));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_HP, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_server_initial_secret, sizeof(draft17_test_server_initial_secret),
            draft17_test_server_pn, sizeof(draft17_test_server_pn));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_KEY, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_client_initial_secret, sizeof(draft17_test_client_initial_secret),
            draft17_test_client_key, sizeof(draft17_test_client_key));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_IV, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_client_initial_secret, sizeof(draft17_test_client_initial_secret),
            draft17_test_client_iv, sizeof(draft17_test_client_iv));
    }

    if (ret == 0) {
        ret = draft17_label_expansion_test(cipher, PICOQUIC_LABEL_HP, PICOQUIC_LABEL_QUIC_V1_KEY_BASE,
            draft17_test_client_initial_secret, sizeof(draft17_test_client_initial_secret),
            draft17_test_client_pn, sizeof(draft17_test_client_pn));
    }

    /* Check the salt */
    version_index = picoquic_get_version_index(draft17_test_vn);
    if (version_index < 0) {
        DBG_PRINTF("Test version (%x) is not supported.\n", draft17_test_vn);
        ret = -1;
    }
    else if (picoquic_supported_versions[version_index].version_aead_key == NULL) {
        DBG_PRINTF("Test version (%x) has no salt.\n", draft17_test_vn);
        ret = -1;
    }
    else if (picoquic_supported_versions[version_index].version_aead_key_length != sizeof(draft17_test_salt))
    {
        DBG_PRINTF("Test version (%x) has no salt[%d], expected [%d].\n", draft17_test_vn,
            (int)picoquic_supported_versions[version_index].version_aead_key_length, (int) sizeof(draft17_test_salt));
        ret = -1;
    }
    else if (memcmp(picoquic_supported_versions[version_index].version_aead_key, draft17_test_salt, sizeof(draft17_test_salt)) != 0) {
        /* TODO: this test means that the reminder of the code will not be executed for new versions */
        DBG_PRINTF("Test version (%x) does not have matching salt.\n", draft17_test_vn);
    }
    else {

        /* Check the master secret and then client and server secret */
        if (ret == 0) {
            salt.base = draft17_test_salt;
            salt.len = sizeof(draft17_test_salt);

            ret = picoquic_setup_initial_master_secret(cipher, salt, draft17_test_cnx_id, master_secret);

            if (ret != 0) {
                DBG_PRINTF("Cannot compute master secret, ret = %x\n", ret);
            }
            else {
                if (memcmp(master_secret, draft17_test_initial_secret, sizeof(draft17_test_initial_secret)) != 0) {
                    DBG_PRINTF("%s", "Initial master secret does not match expected value");
                    ret = -1;
                }
            }

            if (ret == 0) {
                ret = picoquic_setup_initial_secrets(cipher, master_secret, client_secret, server_secret);

                if (ret != 0) {
                    DBG_PRINTF("Cannot derive client and server secrets, ret = %x\n", ret);
                }
                else {
                    if (memcmp(client_secret, draft17_test_client_initial_secret, sizeof(draft17_test_client_initial_secret)) != 0) {
                        DBG_PRINTF("%s", "Initial client secret does not match expected value");
                        ret = -1;
                    }

                    if (memcmp(server_secret, draft17_test_server_initial_secret, sizeof(draft17_test_server_initial_secret)) != 0) {
                        DBG_PRINTF("%s", "Initial server secret does not match expected value");
                        ret = -1;
                    }
                }
            }
        }

        /* First integration test: verify that the aead keys are as expected */
        if (ret == 0) {
            ret = cleartext_aead_vector_test_one(draft17_test_cnx_id, draft17_test_vn,
                draft17_test_client_iv, sizeof(draft17_test_client_iv),
                draft17_test_server_iv, sizeof(draft17_test_server_iv), "draft17_vector");
        }

#if 0
        /* TODO: reset this test once we have draft-17 samples. */
        /* Final integration test: verify that the incoming packet can be decrypted */
        if (ret == 0) {
            ret = draft31_incoming_initial_test(void);
        }
#endif
    }
    return ret;
}
```

### Rust test body
```rust
fn draft17_vector() {
    // Known-answer data for the draft-17 / QUIC-v1 interop vector.
    let draft17_test_cnx_id =
        ConnectionId::clone_from_slice(&[0x7d, 0xdc, 0x42, 0x90, 0xc4, 0xe7, 0xd2, 0x04]).unwrap();

    #[rustfmt::skip]
    let draft17_test_salt: [u8; 20] = [
        0xef, 0x4f, 0xb0, 0xab, 0xb4, 0x74, 0x70, 0xc4,
        0x1b, 0xef, 0xcf, 0x80, 0x31, 0x33, 0x4f, 0xae,
        0x48, 0x5e, 0x09, 0xa0,
    ];
    #[rustfmt::skip]
    let draft17_test_initial_secret: [u8; 32] = [
        0xe5, 0x6c, 0x75, 0x1d, 0xbc, 0x9a, 0xb8, 0xe7,
        0x9f, 0x61, 0x61, 0x42, 0xc0, 0xc0, 0x7a, 0xb8,
        0x30, 0xeb, 0x25, 0x96, 0x8f, 0xae, 0xb7, 0x40,
        0x4d, 0xa6, 0x9a, 0x80, 0xf7, 0x5f, 0x1c, 0x7c,
    ];
    #[rustfmt::skip]
    let draft17_test_server_initial_secret: [u8; 32] = [
        0x5e, 0xac, 0x74, 0x74, 0x78, 0x72, 0xfe, 0x6d,
        0x9e, 0xcb, 0xac, 0x75, 0xdf, 0x87, 0xab, 0xc4,
        0xbb, 0x43, 0x74, 0xc8, 0xe6, 0x63, 0x65, 0x49,
        0xda, 0x71, 0x8b, 0x9f, 0x72, 0x2f, 0x0d, 0x6a,
    ];
    #[rustfmt::skip]
    let draft17_test_server_key: [u8; 16] = [
        0xf3, 0x67, 0xa4, 0xc1, 0x2f, 0x77, 0x26, 0xd9,
        0x2c, 0xce, 0xa2, 0x1b, 0x93, 0x39, 0xa8, 0x71,
    ];
    #[rustfmt::skip]
    let draft17_test_server_iv: [u8; 12] = [
        0x44, 0x82, 0x14, 0xc9, 0x66, 0x31, 0x4d, 0x8f, 0x54, 0x0b, 0x7b, 0x43,
    ];
    #[rustfmt::skip]
    let draft17_test_server_pn: [u8; 16] = [
        0x92, 0x2b, 0x11, 0x3f, 0x1b, 0x2a, 0x81, 0x5f,
        0x08, 0x42, 0x54, 0xf9, 0x81, 0xa0, 0xb0, 0x97,
    ];
    #[rustfmt::skip]
    let draft17_test_client_initial_secret: [u8; 32] = [
        0xf8, 0x86, 0x16, 0x78, 0x10, 0x56, 0xa6, 0xac,
        0x00, 0x70, 0x87, 0xd1, 0x21, 0xce, 0x15, 0x8e,
        0xa8, 0xc7, 0x70, 0xa1, 0xe6, 0x28, 0x99, 0x61,
        0x6c, 0xde, 0x50, 0x7b, 0xb6, 0xd6, 0x0e, 0x08,
    ];
    #[rustfmt::skip]
    let draft17_test_client_key: [u8; 16] = [
        0x1b, 0x7e, 0x28, 0x58, 0x10, 0x18, 0x33, 0xce,
        0x98, 0x9a, 0x77, 0x25, 0x4f, 0x3f, 0xaa, 0x62,
    ];
    #[rustfmt::skip]
    let draft17_test_client_iv: [u8; 12] = [
        0x01, 0xa4, 0x1a, 0xa7, 0x3c, 0x43, 0x29, 0x8d, 0xcb, 0x38, 0xbc, 0xb6,
    ];
    #[rustfmt::skip]
    let draft17_test_client_pn: [u8; 16] = [
        0x9a, 0x85, 0x42, 0xef, 0x39, 0x90, 0x38, 0xab,
        0xa6, 0x6e, 0xf1, 0x33, 0x38, 0x09, 0xfc, 0x5b,
    ];

    // Check HKDF label expansions for server key, IV, and HP.
    let mut out = [0u8; 16];
    hkdf_expand_label(
        LABEL_KEY,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_server_initial_secret,
        &mut out,
    )
    .expect("expand server key");
    assert_eq!(&out, &draft17_test_server_key, "server key mismatch");

    let mut out_iv = [0u8; 12];
    hkdf_expand_label(
        LABEL_IV,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_server_initial_secret,
        &mut out_iv,
    )
    .expect("expand server IV");
    assert_eq!(&out_iv, &draft17_test_server_iv, "server IV mismatch");

    let mut out_hp = [0u8; 16];
    hkdf_expand_label(
        LABEL_HP,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_server_initial_secret,
        &mut out_hp,
    )
    .expect("expand server HP");
    assert_eq!(&out_hp, &draft17_test_server_pn, "server HP mismatch");

    let mut out_ck = [0u8; 16];
    hkdf_expand_label(
        LABEL_KEY,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_client_initial_secret,
        &mut out_ck,
    )
    .expect("expand client key");
    assert_eq!(&out_ck, &draft17_test_client_key, "client key mismatch");

    let mut out_civ = [0u8; 12];
    hkdf_expand_label(
        LABEL_IV,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_client_initial_secret,
        &mut out_civ,
    )
    .expect("expand client IV");
    assert_eq!(&out_civ, &draft17_test_client_iv, "client IV mismatch");

    let mut out_chp = [0u8; 16];
    hkdf_expand_label(
        LABEL_HP,
        LABEL_QUIC_V1_KEY_BASE,
        &draft17_test_client_initial_secret,
        &mut out_chp,
    )
    .expect("expand client HP");
    assert_eq!(&out_chp, &draft17_test_client_pn, "client HP mismatch");

    // Check the version salt and master secret derivation.
    let version_params = INTEROP_VERSION_LATEST.parameters();
    assert_eq!(
        version_params.version_aead_key.len(),
        draft17_test_salt.len(),
        "salt length mismatch for INTEROP_VERSION_LATEST"
    );
    if version_params.version_aead_key == draft17_test_salt {
        // The current INTEROP_VERSION_LATEST uses this salt — verify the full key tree.
        let mut master_secret = [0u8; 32];
        setup_initial_master_secret(&draft17_test_salt, draft17_test_cnx_id, &mut master_secret)
            .expect("setup master secret");
        assert_eq!(
            &master_secret, &draft17_test_initial_secret,
            "master secret mismatch"
        );

        let mut client_secret = [0u8; 32];
        let mut server_secret = [0u8; 32];
        setup_initial_secrets(&master_secret, &mut client_secret, &mut server_secret)
            .expect("setup initial secrets");
        assert_eq!(
            &client_secret, &draft17_test_client_initial_secret,
            "client initial secret mismatch"
        );
        assert_eq!(
            &server_secret, &draft17_test_server_initial_secret,
            "server initial secret mismatch"
        );
    }
    // (If the interop version salt no longer matches the draft-17 vector, skip the
    // master-secret check — this mirrors the C test's `else if memcmp(...) != 0` path
    // which logs a message but does not fail.)

    // Integration test: verify that the AEAD contexts are set up correctly
    // for the known CID and version.
    aead_vector_test_one(draft17_test_cnx_id, INTEROP_VERSION_LATEST as u32);
}
```

## `picoquictest/cnxstress.c:cnx_limit_test`
* C test-table name: `cnx_limit`
* C entry function: `cnx_limit_test`
* Rust test: `cnx_limit`
* C source: `picoquictest/cnxstress.c:975-1036`
* Rust source: `rs/fq/src/tests/cnxstress.rs:897-957`

### C test body
```c
{
    int ret = 0;
    int nb_clients = 4;
    uint64_t duration = 120000000;
    cnx_stress_ctx_t* stress_ctx = cnx_stress_create_ctx(duration, nb_clients, 1);

    if (stress_ctx == NULL) {
        ret = -1;
    }

    if (stress_ctx != NULL) {
        int is_done = 0;

        /* loop until time exhausted or all created */
        while (ret == 0 && stress_ctx->simulated_time < duration && !is_done) {
            ret = cnx_stress_loop_step(stress_ctx);
            if (stress_ctx->nb_clients == nb_clients &&
                stress_ctx->nb_servers == nb_clients) {
                is_done = 1;
                for (int c = 0; c < nb_clients; c++) {
                    if (stress_ctx->c_ctx[c] == NULL ||
                        stress_ctx->c_ctx[c]->cnx == NULL ||
                        stress_ctx->c_ctx[c]->cnx->cnx_state <
                        picoquic_state_client_almost_ready) {
                        is_done = 0;
                        break;
                    }
                }
            }
        }
        if (!is_done) {
            ret = -1;
        }

        if (ret == 0) {
            /* Try creating one more client. Loop until time exhausted or
             * verify new client refused because server busy */

            stress_ctx->is_limit_test = 1;
            stress_ctx->next_client_creation_time = stress_ctx->simulated_time;
            stress_ctx->nb_client_target++;
            while (ret == 0 && stress_ctx->simulated_time < duration &&
                !stress_ctx->limit_test_got_server_busy) {
                ret = cnx_stress_loop_step(stress_ctx);
            }
            if (!stress_ctx->limit_test_got_server_busy) {
                ret = -1;
            }
        }

        cnx_stress_delete_ctx(stress_ctx);
    }

    return ret;
}
```

### Rust test body
```rust
fn cnx_limit() {
    let nb_clients = 4usize;
    let duration = 120_000_000u64;

    let mut ctx = cnx_stress_create_ctx(duration, nb_clients, true).expect("create stress context");

    // Run until all `nb_clients` connections are up on both sides and at least
    // at `ClientAlmostReady`.
    let mut is_done = false;
    while !is_done && ctx.simulated_time() < duration {
        cnx_stress_loop_step(&mut ctx).expect("loop step");

        let (nb_c, nb_s) = {
            let shared = ctx.shared.borrow();
            (shared.nb_clients, shared.nb_servers)
        };
        if nb_c == nb_clients && nb_s == nb_clients {
            // Check that every client slot has a connection at AlmostReady or beyond.
            is_done = true;
            for c in 0..nb_clients {
                let ok = if let Some(Some(cid)) = ctx.client_connections.get(c) {
                    let cid = *cid;
                    if let Some(cnx) = ctx.qclient.connection_ref_by_id(cid) {
                        cnx.state() >= State::ClientAlmostReady
                    } else {
                        false
                    }
                } else {
                    false
                };
                if !ok {
                    is_done = false;
                    break;
                }
            }
        }
    }
    assert!(
        is_done,
        "failed to reach ClientAlmostReady for all connections"
    );

    // Now attempt one extra connection (beyond the server's limit).
    {
        let mut shared = ctx.shared.borrow_mut();
        shared.is_limit_test = true;
        shared.nb_client_target += 1;
    }
    ctx.next_client_creation_time = ctx.simulated_time();

    while ctx.simulated_time() < duration {
        cnx_stress_loop_step(&mut ctx).expect("limit loop step");
        if ctx.shared.borrow().limit_test_got_server_busy {
            break;
        }
    }
    assert!(
        ctx.shared.borrow().limit_test_got_server_busy,
        "server did not send SERVER_BUSY when at connection limit",
    );
}
```

## `picoquictest/config_test.c:config_usage_test`
* C test-table name: `config_usage`
* C entry function: `config_usage_test`
* Rust test: `config_usage`
* C source: `picoquictest/config_test.c:787-806`
* Rust source: `rs/fq/src/tests/config.rs:582-591`

### C test body
```c
{

    FILE* F = NULL;
    char config_usage_ref[512];
    int ret = picoquic_get_input_path(config_usage_ref, sizeof(config_usage_ref), picoquic_solution_dir, CONFIG_USAGE_REF);

    config_test_register_cc_algorithms();

    if (ret == 0 && (F = picoquic_file_open(CONFIG_USAGE_TXT, "wt")) != NULL){
        picoquic_config_usage_file(F);
        F = picoquic_file_close(F);
    }

    if (ret == 0) {
        ret = picoquic_test_compare_text_files(CONFIG_USAGE_TXT, config_usage_ref);
    }

    return ret;
}
```

### Rust test body
```rust
fn config_usage() {
    config_test_register_cc_algorithms();
    let mut buf = String::new();
    Config::write_usage(&mut buf);
    let expected = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/config_usage_ref.txt"
    ));
    assert_eq!(buf, expected);
}
```

## `picoquictest/congestion_test.c:gbps_performance_test`
* C test-table name: `bbr_gbps`
* C entry function: `gbps_performance_test`
* Rust test: `bbr_gbps`
* C source: `picoquictest/congestion_test.c:373-386`
* Rust source: `rs/fq/src/tests/congestion.rs:838-843`

### C test body
```c
{
    uint64_t max_completion_time = 250000;
    uint64_t latency = 4000;
    uint64_t jitter = 2000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 1000;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Rust test body
```rust
fn bbr_gbps() {
    let latency = 4_000u64;
    let jitter = 2_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(250_000, 1_000, latency, jitter, buffer);
}
```
