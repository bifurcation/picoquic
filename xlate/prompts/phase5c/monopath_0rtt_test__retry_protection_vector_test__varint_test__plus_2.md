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

## `picoquictest/multipath_test.c:monopath_0rtt_test`
* C test-table name: `monopath_0rtt`
* C entry function: `monopath_0rtt_test`
* Rust test: `monopath_0rtt_2`
* Expected Rust file: `rs/fq/src/tests/multipath.rs`
* Current Rust span: `rs/fq/src/tests/multipath.rs:1412-1418`
* Baseline outcome: `blocked`
* Baseline source: `/private/tmp/picoquic-5b-05`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: The Rust wrapper sets do_multipath like C, but zero_rtt_test_one manually seeds 0-RTT counters and sets client/server is_multipath_enabled before checking them, making the key 0-RTT and multipath-negotiation assertions placeholder-like.
* Phase 5A fix note: Make zero_rtt_test_one observe real 0-RTT send/ack/PSK/ticket state and verify negotiated multipath without manually forcing the flags being asserted.
* Phase 5B analysis: agent response did not include this test
* Phase 5B fix note: 

### C test body
```c
{
    zero_rtt_test_t zrt = { 0 };
    zrt.do_multipath = 1;
    return zero_rtt_test_one(&zrt);
}
```

### Current Rust test body
```rust
fn monopath_0rtt_2() {
    let zrt = ZeroRttTest {
        do_multipath: true,
        ..ZeroRttTest::default()
    };
    zero_rtt_test_one(&zrt).expect("monopath_0rtt_2");
}
```

## `picoquictest/cleartext_aead_test.c:retry_protection_vector_test`
* C test-table name: `retry_protection_vector`
* C entry function: `retry_protection_vector_test`
* Rust test: `retry_protection_vector`
* Expected Rust file: `rs/fq/src/tests/cleartext_aead.rs`
* Current Rust span: `rs/fq/src/tests/cleartext_aead.rs:657-815`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers the same phases, but uses Version::V1 retry key while the C vector explicitly uses picoquic_retry_protection_key_25; that is a different test vector. Rust also omits the explicit IV comparison.
* Phase 5A fix note: Use the draft-25 retry-protection key bytes from picoquic_retry_protection_key_25 for this test, and add an equivalent expected-IV check if the Rust API exposes or can derive it.
* Phase 5B analysis: Rust now checks the C draft-25 retry vector instead of the modern QUIC v1 retry key path. PacketKey does not expose IV, so the test derives and compares the expected IV with the same HKDF label inputs.
* Phase 5B fix note: Added draft-25 retry key and expected IV constants, switched retry_protection_vector to use LABEL_QUIC_V1_KEY_BASE with that key, and added protection/verification IV assertions.

### C test body
```c
{
    /* First, create a protection context to test the basic mechanisms */
    int ret = 0;
    void* protection_ctx = picoquic_create_retry_protection_context(1, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);

    if (protection_ctx == NULL) {
        DBG_PRINTF("%s", "Cannot create protection context!");
        ret = -1;
    }
    else if (0 != cleartext_iv_cmp(protection_ctx, retry_protection_test_iv, sizeof(retry_protection_test_iv))) {
        DBG_PRINTF("%s", "Clear protection IV does not match expected value.\n");
            ret = -1;
    } 
    else {
        uint8_t encoded[256];
        size_t encoded_length = picoquic_aead_encrypt_generic(encoded, encoded, 0, 0, retry_protection_pseudo_packet, sizeof(retry_protection_pseudo_packet), protection_ctx);

        if (encoded_length != 16) {
            DBG_PRINTF("Encoded length = %d instead of 16", (int)encoded_length);
            ret = -1;
        }
        else if (memcmp(encoded, retry_protection_test_checksum, 16) != 0) {
            DBG_PRINTF("%s", "Test vector does not match!");
            ret = -1;
        }

        picoquic_aead_free(protection_ctx);

        if (ret == 0) {
            void* verification_ctx = picoquic_create_retry_protection_context(0, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
            if (verification_ctx == NULL) {
                DBG_PRINTF("%s", "Cannot create verification context!");
                ret = -1;
            }
            else if (0 != cleartext_iv_cmp(verification_ctx, retry_protection_test_iv, sizeof(retry_protection_test_iv))) {
                DBG_PRINTF("%s", "Clear verification IV does not match expected value.\n");
                    ret = -1;
            }
            else {
                uint8_t decoded[256];
                size_t decoded_length = picoquic_aead_decrypt_generic(decoded, encoded, encoded_length, 0, retry_protection_pseudo_packet, sizeof(retry_protection_pseudo_packet), verification_ctx);

                if (decoded_length != 0) {
                    DBG_PRINTF("Decoded length = %d instead of 0", (int)decoded_length);
                    ret = -1;
                }
                else {
                    /* Positive test succeeded, now do a negative test */
                    encoded[0] ^= 1;
                    decoded_length = picoquic_aead_decrypt_generic(decoded, encoded, encoded_length, 0, retry_protection_pseudo_packet, sizeof(retry_protection_pseudo_packet), verification_ctx);
                    if (decoded_length == 0) {
                        DBG_PRINTF("Decoded length = 0 instead of expected error", (int)decoded_length);
                        ret = -1;
                    }
                }

                picoquic_aead_free(verification_ctx);
            }
        }
    }

    if (ret == 0) {
        /* Test the verification functions */
        void* protection_ctx = picoquic_create_retry_protection_context(1, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
        uint8_t packet[PICOQUIC_MAX_PACKET_SIZE];
        size_t packet_index = sizeof(retry_protection_test_input);

        if (protection_ctx == NULL) {
            DBG_PRINTF("%s", "Cannot create protection context!");
            ret = -1;
        }
        else {
            size_t length;
            memcpy(packet, retry_protection_test_input, packet_index);

            length = picoquic_encode_retry_protection(protection_ctx, packet, PICOQUIC_MAX_PACKET_SIZE, packet_index, &retry_protection_test_odcid);

            if (length != packet_index + sizeof(retry_protection_test_checksum)) {
                DBG_PRINTF("Packet length = %d instead of %d+16", (int)length, (int)packet_index);
                ret = -1;
            }
            else if (memcmp(packet + packet_index, retry_protection_test_checksum, sizeof(retry_protection_test_checksum)) != 0) {
                DBG_PRINTF("%s", "Packet checksum does not match!");
                ret = -1;
            }
            
            picoquic_aead_free(protection_ctx);

            if (ret == 0) {
                void* verification_ctx = picoquic_create_retry_protection_context(0, picoquic_retry_protection_key_25, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
                if (verification_ctx == NULL) {
                    DBG_PRINTF("%s", "Cannot create verification context!");
                    ret = -1;
                }
                else {
                    size_t data_length = length;
                    size_t bytes_index = sizeof(retry_protection_test_input) - RETRY_PROTECTION_TEST_RETRY_TOKEN_LENGTH;

                    ret = picoquic_verify_retry_protection(verification_ctx, packet, &data_length, bytes_index, &retry_protection_test_odcid);

                    if (ret != 0) {
                        DBG_PRINTF("Verification returns %d (0x%d)!", ret, ret);
                    }
                    else if (data_length != sizeof(retry_protection_test_input)) {
                        DBG_PRINTF("Verification returns length %d instead of %d!", (int)data_length, (int)sizeof(retry_protection_test_input));
                        ret = -1;
                    }

                    if (ret == 0) {
                        /* Try verification with a different odcid. It should fail */
                        picoquic_connection_id_t bad_odcid = retry_protection_test_odcid;
                        bad_odcid.id[0] ^= 1;
                        data_length = length;
                        if (picoquic_verify_retry_protection(verification_ctx, packet, &data_length, bytes_index, &bad_odcid) == 0) {
                            DBG_PRINTF("%s", "Bad odcid not detected!");
                            ret = -1;
                        }
                    }


                    if (ret == 0) {
                        /* Verify that the draft 25 vector passes */
                        data_length = sizeof(retry_protection_packet_draft25);
                        memcpy(packet, retry_protection_packet_draft25, data_length);
                        bytes_index = data_length - RETRY_PROTECTION_TEST_RETRY_TOKEN_LENGTH;

                        ret = picoquic_verify_retry_protection(verification_ctx, packet, &data_length, bytes_index, &retry_protection_odcid_draft25);

                        if (ret != 0) {
                            DBG_PRINTF("Testing vector in draft 25 returns %d (0x%x)!", ret, ret);
                        }
                    }

                    picoquic_aead_free(verification_ctx);
                }
            }
        }
    }

    return ret;
}
```

### Current Rust test body
```rust
fn retry_protection_vector() {
    // Retry pseudo-packet constants.
    const RETRY_TOKEN_LENGTH: usize = 24;

    #[rustfmt::skip]
    let retry_protection_test_input: [u8; 41] = [
        // FIRST_BYTE
        0xF5,
        // VERSION = 0xFF000019
        0xFF, 0x00, 0x00, 0x19,
        // DCID_LENGTH = 6, DCID_BYTES = 61..66
        6, 61, 62, 63, 64, 65, 66,
        // SCID_LENGTH = 4, SCID_BYTES = 44..47
        4, 44, 45, 46, 47,
        // RETRY_TOKEN (24 bytes) = 101..124
        101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
        113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124,
    ];

    let retry_protection_test_odcid =
        ConnectionId::clone_from_slice(&[81, 82, 83, 84, 85, 86, 87, 88]).unwrap();

    // Pseudo-packet = ODCID_LENGTH | ODCID_BYTES | retry_protection_test_input.
    let retry_protection_pseudo_packet: Vec<u8> = {
        let mut v = Vec::with_capacity(1 + 8 + retry_protection_test_input.len());
        v.push(8u8); // ODCID length
        v.extend_from_slice(&[81, 82, 83, 84, 85, 86, 87, 88]);
        v.extend_from_slice(&retry_protection_test_input);
        v
    };

    #[rustfmt::skip]
    let retry_protection_test_checksum: [u8; 16] = [
        0xf9, 0x50, 0xf8, 0x85, 0x71, 0x4b, 0xae, 0x7a,
        0xf1, 0xe2, 0x86, 0x7d, 0xd8, 0xf7, 0x83, 0x92,
    ];

    // Draft-25 retry packet vector.
    let retry_protection_odcid_draft25 =
        ConnectionId::clone_from_slice(&[0x83, 0x94, 0xc8, 0xf0, 0x3e, 0x51, 0x57, 0x08]).unwrap();
    #[rustfmt::skip]
    let retry_protection_packet_draft25: [u8; 36] = [
        0xff, 0xff, 0x00, 0x00, 0x19, 0x00, 0x08, 0xf0, 0x67, 0xa5, 0x50, 0x2a, 0x42, 0x62,
        0xb5, 0x74, 0x6f, 0x6b, 0x65, 0x6e, 0x1e, 0x5e, 0xc5, 0xb0, 0x14, 0xcb, 0xb1, 0xf0,
        0xfd, 0x93, 0xdf, 0x40, 0x48, 0xc4, 0x46, 0xa6,
    ];

    // Obtain the QUIC-v1 retry integrity key via the version parameters.
    let v1_params = Version::V1.parameters();
    let retry_key = v1_params.version_retry_key;
    let prefix_label = v1_params.tls_prefix_label;

    // Phase 1: low-level AEAD encrypt/decrypt against the known checksum.
    {
        let protection_ctx = create_retry_protection_context(true, retry_key, prefix_label)
            .expect("create protection ctx");

        // Encrypt empty plaintext; the only output is the 16-byte AEAD tag.
        let mut tag: Vec<u8> = Vec::new();
        protection_ctx.encrypt(0, &retry_protection_pseudo_packet, &mut tag);
        assert_eq!(tag.len(), 16, "tag length != 16");
        assert_eq!(
            &tag[..],
            &retry_protection_test_checksum,
            "tag does not match expected"
        );

        // Verify: decrypt the tag (should succeed with 0 plaintext bytes).
        let verification_ctx = create_retry_protection_context(false, retry_key, prefix_label)
            .expect("create verification ctx");

        let mut tag_verify = tag.clone();
        verification_ctx
            .decrypt(0, &retry_protection_pseudo_packet, &mut tag_verify)
            .expect("AEAD decrypt should succeed");
        assert!(tag_verify.is_empty(), "decrypted plaintext should be empty");

        // Negative test: corrupt one byte → decryption must fail.
        tag_verify = tag.clone();
        tag_verify[0] ^= 1;
        assert!(
            verification_ctx
                .decrypt(0, &retry_protection_pseudo_packet, &mut tag_verify)
                .is_err(),
            "corrupted tag should fail verification"
        );
    }

    // Phase 2: encode_retry_protection / verify_retry_protection.
    {
        let protection_ctx = create_retry_protection_context(true, retry_key, prefix_label)
            .expect("create protection ctx");

        let mut packet = vec![0u8; MAX_PACKET_SIZE];
        let packet_index = retry_protection_test_input.len();
        packet[..packet_index].copy_from_slice(&retry_protection_test_input);

        let length = encode_retry_protection(
            protection_ctx.as_ref(),
            &mut packet,
            packet_index,
            &retry_protection_test_odcid,
        );
        assert_eq!(length, packet_index + 16, "encoded length mismatch");
        assert_eq!(
            &packet[packet_index..length],
            &retry_protection_test_checksum,
            "appended checksum mismatch"
        );

        let verification_ctx = create_retry_protection_context(false, retry_key, prefix_label)
            .expect("create verification ctx");

        // Positive verify.
        let bytes_index = packet_index - RETRY_TOKEN_LENGTH; // = 17
        let new_length = verify_retry_protection(
            verification_ctx.as_ref(),
            &mut packet,
            length,
            bytes_index,
            &retry_protection_test_odcid,
        )
        .expect("verify retry protection");
        assert_eq!(
            new_length, packet_index,
            "verified length should equal input length"
        );

        // Bad ODCID → must fail.
        let mut bad_odcid = retry_protection_test_odcid;
        bad_odcid.as_bytes_mut()[0] ^= 1;
        packet[..packet_index].copy_from_slice(&retry_protection_test_input);
        packet[packet_index..length].copy_from_slice(&retry_protection_test_checksum);
        assert!(
            verify_retry_protection(
                verification_ctx.as_ref(),
                &mut packet,
                length,
                bytes_index,
                &bad_odcid,
            )
            .is_err(),
            "bad ODCID should fail verification"
        );

        // Draft-25 vector.
        let draft25_len = retry_protection_packet_draft25.len();
        packet[..draft25_len].copy_from_slice(&retry_protection_packet_draft25);
        let draft25_bytes_index = draft25_len - RETRY_TOKEN_LENGTH; // = 12
        verify_retry_protection(
            verification_ctx.as_ref(),
            &mut packet,
            draft25_len,
            draft25_bytes_index,
            &retry_protection_odcid_draft25,
        )
        .expect("draft-25 vector verification");
    }
}
```

## `picoquictest/intformattest.c:varint_test`
* C test-table name: `varint`
* C entry function: `varint_test`
* Rust test: `varint`
* Expected Rust file: `rs/fq/src/tests/intformattest.rs`
* Current Rust span: `rs/fq/src/tests/intformattest.rs:108-292`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-06`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust covers the same cases for legacy varint encode/decode, but skips the C checks for frames_varint_decode, frames_varint_encode, and frames_varint_encode_length.
* Phase 5A fix note: Extend the Rust test to exercise the frame-style decode/encode APIs and length prediction for canonical cases, matching the C loops.
* Phase 5B analysis: Rust now checks the C varint table through both legacy and frame-style decode/encode paths, with noncanonical cases remaining decode-only as in C.
* Phase 5B fix note: Extended `varint` to exercise `frames_varint_decode`, `frames_varint_encode`, and `frames_varint_encode_length` alongside the existing legacy varint checks.

### C test body
```c
{
    int ret = 0;
    uint8_t test_buf[16];
    const picoquic_varintformat_test_t* max_test = varint_test_cases + nb_varint_test_cases;
    memset(test_buf, 0xcc, 16);

    for (picoquic_varintformat_test_t* test = varint_test_cases; ret == 0 && test < max_test; test++) {
        for (int is_new_decode = 0; ret == 0 && is_new_decode <= 1; is_new_decode++) {
            for (size_t buf_size = 0; ret == 0 && buf_size <= test->length + 2 && buf_size < 16; buf_size++) {
                int test_ret = 0;
                uint64_t n64 = 0;
                size_t length;

                memcpy(test_buf, test->encoding, test->length);

                if (is_new_decode) {
                    const uint8_t* bytes = picoquic_frames_varint_decode(test_buf, test_buf + buf_size, &n64);
                    length = bytes != NULL ? bytes - test_buf : 0;
                }
                else {
                    length = picoquic_varint_decode(test_buf, buf_size, &n64);
                }

                if (length != (buf_size < test->length ? 0 : test->length)) {
                    DBG_PRINTF("Varint: unexpected length %u", (unsigned)length);
                    test_ret = -1;
                }
                else if (length == 0) {
                    continue;
                }
                else if (n64 != test->decoded) {
                    DBG_PRINTF("Varint: unexpected value %llu [expected %llu]",
                        (unsigned long long)n64, (unsigned long long)test->decoded);
                    test_ret = -1;
                }

                if (test_ret != 0) {
                    DBG_PRINTF(" (is_new=%d, test=%u, buf_size=%u/%u)\n",
                        is_new_decode, (unsigned)(max_test - test), (unsigned)buf_size, (unsigned)test->length);
                    ret = -1;
                }
            }
        }

        for (int is_new_encode = 0; ret == 0 && is_new_encode <= 1; is_new_encode++) {
            if (test->is_canonical != 0) {
                uint8_t encoding[8];
                size_t coded_length = 0;

                if (is_new_encode) {
                    uint8_t *bytes = picoquic_frames_varint_encode(encoding, &encoding[0] + sizeof(encoding), test->decoded);
                    if (bytes == NULL) {
                        coded_length = SIZE_MAX;
                    }
                    else {
                        coded_length = bytes - encoding;
                    }
                }
                else {
                    coded_length = picoquic_varint_encode(encoding, test->length, test->decoded);
                }

                if (coded_length != test->length) {
                    DBG_PRINTF("Varint, is_new=%d: unexpected coded_length=%"PRIst, is_new_encode, coded_length);
                    ret = -1;
                }
                else if (coded_length > sizeof(encoding)) {
                    DBG_PRINTF("Unexpected coded_length=%"PRIst", > sizeof(buffer)", coded_length);
                    ret = -1;
                }
                else if (memcmp(encoding, test->encoding, coded_length) != 0) {
                    DBG_PRINTF("Varint, is_new=%d: unexpected coded value", is_new_encode);
                    ret = -1;
                }
            }
        }
    }

    if (ret == 0) {
        /* test the length prediction */
        size_t picoquic_frames_varint_encode_length(uint64_t n64);

        for (picoquic_varintformat_test_t* test = varint_test_cases; ret == 0 && test < max_test; test++) {
            if (test->is_canonical) {
                size_t predicted_length = picoquic_frames_varint_encode_length(test->decoded);
                if (predicted_length != test->length) {
                    ret = -1;
                }
            }
        }
    }
 
    return ret;
}
```

### Current Rust test body
```rust
fn varint() {
    use crate::internal::{
        frames_varint_decode, frames_varint_encode_length, varint_decode, varint_encode,
    };
    use crate::utils::frames_varint_encode;

    struct Case {
        encoding: [u8; 8],
        length: usize,
        decoded: u64,
        is_canonical: bool,
    }

    let cases = [
        Case {
            encoding: [0, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 1,
            decoded: 0,
            is_canonical: true,
        },
        Case {
            encoding: [1, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 1,
            decoded: 1,
            is_canonical: true,
        },
        Case {
            encoding: [63, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 1,
            decoded: 63,
            is_canonical: true,
        },
        Case {
            encoding: [0x40, 64, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 2,
            decoded: 64,
            is_canonical: true,
        },
        Case {
            encoding: [0x7F, 0xFF, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 2,
            decoded: 0x3FFF,
            is_canonical: true,
        },
        Case {
            encoding: [0x80, 0, 0x40, 0, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 4,
            decoded: 0x4000,
            is_canonical: true,
        },
        Case {
            encoding: [0xBF, 0xFF, 0xFF, 0xFF, 0xCC, 0xCC, 0xCC, 0xCC],
            length: 4,
            decoded: 0x3FFF_FFFF,
            is_canonical: true,
        },
        Case {
            encoding: [0xC0, 0, 0, 0, 0x40, 0, 0, 0],
            length: 8,
            decoded: 0x4000_0000,
            is_canonical: true,
        },
        Case {
            encoding: [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            length: 8,
            decoded: 0x3FFF_FFFF_FFFF_FFFF,
            is_canonical: true,
        },
        Case {
            encoding: [0xc2, 0x19, 0x7c, 0x5e, 0xff, 0x14, 0xe8, 0x8c],
            length: 8,
            decoded: 151_288_809_941_952_652,
            is_canonical: true,
        },
        Case {
            encoding: [0x9d, 0x7f, 0x3e, 0x7d, 0xff, 0x14, 0xe8, 0x8c],
            length: 4,
            decoded: 494_878_333,
            is_canonical: true,
        },
        Case {
            encoding: [0xC0, 0, 0, 0, 0x1d, 0x7f, 0x3e, 0x7d],
            length: 8,
            decoded: 494_878_333,
            is_canonical: false,
        },
        Case {
            encoding: [0x7b, 0xbd, 0x3e, 0x7d, 0xff, 0x14, 0xe8, 0x8c],
            length: 2,
            decoded: 15_293,
            is_canonical: true,
        },
        Case {
            encoding: [0x80, 0, 0x3b, 0xbd, 0x3e, 0x7d, 0xff, 0x14],
            length: 4,
            decoded: 15_293,
            is_canonical: false,
        },
        Case {
            encoding: [0xC0, 0, 0, 0, 0, 0, 0x3b, 0xbd],
            length: 8,
            decoded: 15_293,
            is_canonical: false,
        },
        Case {
            encoding: [0x25, 0xbd, 0x3e, 0x7d, 0xff, 0x14, 0xe8, 0x8c],
            length: 1,
            decoded: 37,
            is_canonical: true,
        },
        Case {
            encoding: [0x40, 0x25, 0xbd, 0x3e, 0x7d, 0xff, 0x14, 0xe8],
            length: 2,
            decoded: 37,
            is_canonical: false,
        },
    ];

    let mut test_buf = [0xCCu8; 16];

    for (idx, case) in cases.iter().enumerate() {
        for is_new_decode in [false, true] {
            for buf_size in 0..=(case.length + 2).min(15) {
                test_buf[..case.length].copy_from_slice(&case.encoding[..case.length]);
                let mut n64 = 0u64;
                let length = if is_new_decode {
                    frames_varint_decode(&test_buf[..buf_size], &mut n64)
                        .map_or(0, |rest| buf_size - rest.len())
                } else {
                    varint_decode(&test_buf[..buf_size], &mut n64)
                };
                let expected_length = if buf_size < case.length {
                    0
                } else {
                    case.length
                };
                assert_eq!(
                    length, expected_length,
                    "case {idx}, is_new_decode={is_new_decode}, buf_size={buf_size}: wrong length"
                );
                if length != 0 {
                    assert_eq!(
                        n64, case.decoded,
                        "case {idx}, is_new_decode={is_new_decode}, buf_size={buf_size}: wrong value"
                    );
                }
            }
        }

        if case.is_canonical {
            for is_new_encode in [false, true] {
                let mut encoding = [0u8; 8];
                let coded_length = if is_new_encode {
                    frames_varint_encode(&mut encoding, case.decoded)
                        .map_or(usize::MAX, |rest| 8 - rest.len())
                } else {
                    varint_encode(&mut encoding[..case.length], case.decoded)
                };
                assert_eq!(
                    coded_length, case.length,
                    "case {idx}, is_new_encode={is_new_encode}: wrong encoded length"
                );
                assert!(
                    coded_length <= encoding.len(),
                    "case {idx}, is_new_encode={is_new_encode}: encoded length exceeds buffer"
                );
                assert_eq!(
                    &encoding[..coded_length],
                    &case.encoding[..coded_length],
                    "case {idx}, is_new_encode={is_new_encode}: wrong bytes"
                );
            }
        }
    }

    for (idx, case) in cases.iter().enumerate() {
        if case.is_canonical {
            assert_eq!(
                frames_varint_encode_length(case.decoded),
                case.length,
                "case {idx}: wrong predicted frame varint length"
            );
        }
    }
}
```

## `picoquictest/parseheadertest.c:packet_enc_dec_test`
* C test-table name: `packet_enc_dec`
* C entry function: `packet_enc_dec_test`
* Rust test: `packet_enc_dec`
* Expected Rust file: `rs/fq/src/tests/parseheadertest.rs`
* Current Rust span: `rs/fq/src/tests/parseheadertest.rs:737-880`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-10`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust only exercises Initial and Handshake packets; C also tests 0-RTT with null remote CID fallback and 1-RTT, and its decrypt helper verifies the expected server connection.
* Phase 5A fix note: Extend packet_enc_dec to add the C 0-RTT and 1-RTT cases with the same secrets, lengths, and remote-CID reset/restore; strengthen the helper to validate the decrypted packet uses the expected server connection where possible.
* Phase 5B analysis: Rust packet_enc_dec now covers the C Initial, Handshake, 0-RTT null-remote-CID fallback, and 1-RTT cases, and validates the decrypted packet maps to the expected server connection.
* Phase 5B fix note: Added C test secrets as constants, fixed helper length/AAD setup before encryption, added expected server-token validation and decrypted payload length check, then added 0-RTT and 1-RTT cases with remote-CID reset/restore.

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

### Current Rust test body
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
        test_packet_encrypt_one(&addr, cnx, &mut qserver, None, PacketType::Initial, 1256)
            .expect("initial enc_dec");
    }

    let expected_server = qserver
        .first_cnx_mut()
        .and_then(|server| server.own_token)
        .expect("server cnx");
    let server_local_cid = {
        let cnx_server = qserver.first_cnx_mut().expect("server cnx");
        cnx_server
            .paths
            .first()
            .and_then(|path| path.tuples.first())
            .and_then(|tuple| tuple.local_connection_id)
            .and_then(|token| cnx_server.local_connection_ids.get(token))
            .map(|cid| cid.connection_id)
            .expect("server local cid")
    };

    // Handshake packet
    {
        {
            let cnx = qclient.first_cnx_mut().unwrap();
            cnx.set_test_aead_encrypt(Epoch::Handshake, TEST_HANDSHAKE_SECRET);
            cnx.set_test_pn_enc(Epoch::Handshake, TEST_HANDSHAKE_SECRET);
        }
        let cnx_server = qserver.first_cnx_mut().expect("server cnx");
        cnx_server.set_test_aead_decrypt(Epoch::Handshake, TEST_HANDSHAKE_SECRET);
        cnx_server.set_test_pn_dec(Epoch::Handshake, TEST_HANDSHAKE_SECRET);

        let cnx = qclient.first_cnx_mut().unwrap();
        test_packet_encrypt_one(
            &addr,
            cnx,
            &mut qserver,
            Some(expected_server),
            PacketType::Handshake,
            1256,
        )
        .expect("handshake enc_dec");
    }

    // 0-RTT packet, using a null remote CID to trigger the initial-ID fallback.
    {
        {
            let cnx = qclient.first_cnx_mut().unwrap();
            cnx.set_test_aead_encrypt(Epoch::ZeroRtt, TEST_0RTT_SECRET);
            cnx.set_test_pn_enc(Epoch::ZeroRtt, TEST_0RTT_SECRET);
            cnx.set_path_tuple_remote_cid(0, 0, ConnectionId::default());
        }
        let cnx_server = qserver.first_cnx_mut().expect("server cnx");
        cnx_server.set_test_aead_decrypt(Epoch::ZeroRtt, TEST_0RTT_SECRET);
        cnx_server.set_test_pn_dec(Epoch::ZeroRtt, TEST_0RTT_SECRET);

        let cnx = qclient.first_cnx_mut().unwrap();
        test_packet_encrypt_one(
            &addr,
            cnx,
            &mut qserver,
            Some(expected_server),
            PacketType::ZeroRttProtected,
            256,
        )
        .expect("0rtt enc_dec");

        let cnx = qclient.first_cnx_mut().unwrap();
        cnx.set_path_tuple_remote_cid(0, 0, server_local_cid);
    }

    // 1-RTT packet.
    {
        {
            let cnx = qclient.first_cnx_mut().unwrap();
            cnx.set_test_aead_encrypt(Epoch::OneRtt, TEST_1RTT_SECRET);
            cnx.set_test_pn_enc(Epoch::OneRtt, TEST_1RTT_SECRET);
        }
        let cnx_server = qserver.first_cnx_mut().expect("server cnx");
        cnx_server.set_test_aead_decrypt(Epoch::OneRtt, TEST_1RTT_SECRET);
        cnx_server.set_test_pn_dec(Epoch::OneRtt, TEST_1RTT_SECRET);

        let cnx = qclient.first_cnx_mut().unwrap();
        test_packet_encrypt_one(
            &addr,
            cnx,
            &mut qserver,
            Some(expected_server),
            PacketType::OneRttProtected,
            1024,
        )
        .expect("1rtt enc_dec");
    }
}
```

## `picoquictest/skip_frame_test.c:frames_format_test`
* C test-table name: `frames_format`
* C entry function: `frames_format_test`
* Rust test: `frames_format`
* Expected Rust file: `rs/fq/src/tests/skip_frame.rs`
* Current Rust span: `rs/fq/src/tests/skip_frame.rs:3749-3751`
* Baseline outcome: `fixed`
* Baseline source: `/private/tmp/picoquic-5b-07`
* Phase 5A outcome: `needs_fix`
* Phase 5A analysis: Rust frames_format delegates to a helper that only exercises a small subset of the C formatters and does not reproduce the FRAME_FORMAT_TEST shrinking-buffer/more_data checks for the full C list.
* Phase 5A fix note: Expand run_frames_format_test to cover all C formatter calls, including reset/new cid/retire cid/new token/stop sending/blocked/application close/max stream data/path response/datagram/ack frequency/immediate ack/timestamp/multipath frames, and the short-buffer more_data behavior.
* Phase 5B analysis: Rust now mirrors the C formatter table and shrinking-buffer more_data checks.
* Phase 5B fix note: Expanded frames_format helper to cover all C formatter calls with FRAME_FORMAT_TEST/ONCE-equivalent behavior and matching connection/stream/CID setup.

### C test body
```c
{
    int ret = 0;
    uint8_t buffer[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t data[] = { 0xaa, 0xaa };
    uint8_t* bytes = NULL;
    uint8_t* bytes_max;
    int more_data;
    uint64_t current_time = 0;
    int is_pure_ack = 0;
    picoquic_stream_head_t* stream = NULL;
    int round;
    uint64_t simulated_time = 0;
    picoquic_quic_t* qclient = picoquic_create(8, NULL, NULL, NULL, NULL, NULL,
        NULL, NULL, NULL, NULL, simulated_time,
        &simulated_time, NULL, NULL, 0);
    struct sockaddr_in saddr = { 0 };
    uint8_t addr_bytes[4] = { 1, 2, 3, 4 };
    picoquic_cnx_t* cnx;
    picoquic_local_cnxid_list_t* local_cnxid_list = NULL;
    picoquic_local_cnxid_t* l_cid = NULL; 

    if (qclient == NULL) {
        ret = -1;
    }
    else {
        cnx = frames_format_test_get_cnx(qclient, (struct sockaddr *)&saddr, picoquic_epoch_1rtt, simulated_time, 1);
        if (cnx == NULL) {
            ret = -1;
        }
    }

    if (ret == 0)  {
        local_cnxid_list = cnx->first_local_cnxid_list;
        l_cid = picoquic_create_local_cnxid(cnx, local_cnxid_list->unique_path_id, NULL, current_time);
        picoquic_add_to_stream(cnx, 0, data, 2, 0);
        stream = picoquic_find_stream(cnx, 0);
        if (stream == NULL) {
            ret = -1;
        }
    }
    if (ret == 0) {
        stream->reset_requested = 1;
        FRAME_FORMAT_TEST_ONCE(picoquic_format_reset_stream_frame, 2, stream, bytes, bytes_max, &more_data, &is_pure_ack);
        stream->reset_requested = 0;
        FRAME_FORMAT_TEST(picoquic_format_new_connection_id_frame, cnx, local_cnxid_list, bytes, bytes_max, &more_data, &is_pure_ack, l_cid);
        FRAME_FORMAT_TEST(picoquic_format_retire_connection_id_frame, bytes, bytes_max, &more_data, &is_pure_ack, 1, 0, 17);
        FRAME_FORMAT_TEST(picoquic_format_new_token_frame, bytes, bytes_max, &more_data, &is_pure_ack, data, 2);
        stream->stop_sending_requested = 1;
        FRAME_FORMAT_TEST_ONCE(picoquic_format_stop_sending_frame, 2, stream, bytes, bytes_max, &more_data, &is_pure_ack);
        stream->stop_sending_requested = 0;
        stream->stop_sending_sent = 0;
        FRAME_FORMAT_TEST_ONCE(picoquic_format_data_blocked_frame, 1, cnx, bytes, bytes_max, &more_data, &is_pure_ack);
        FRAME_FORMAT_TEST(picoquic_format_stream_data_blocked_frame, bytes, bytes_max, &more_data, &is_pure_ack, stream);
        stream->stream_data_blocked_sent = 0;
        FRAME_FORMAT_TEST_ONCE(picoquic_format_stream_blocked_frame, 1, cnx, bytes, bytes_max, &more_data, &is_pure_ack, stream);
        cnx->stream_blocked_bidir_sent = 0;
        FRAME_FORMAT_TEST(picoquic_format_connection_close_frame, cnx, bytes, bytes_max, &more_data, &is_pure_ack);
        FRAME_FORMAT_TEST(picoquic_format_application_close_frame, cnx, bytes, bytes_max, &more_data, &is_pure_ack);
        FRAME_FORMAT_TEST(picoquic_format_max_stream_data_frame, cnx, stream, bytes, bytes_max, &more_data, &is_pure_ack, 100000000);
        FRAME_FORMAT_TEST(picoquic_format_path_challenge_frame, bytes, bytes_max, &more_data, &is_pure_ack, 0xaabbccddeeff0011ull);
        FRAME_FORMAT_TEST(picoquic_format_path_response_frame, bytes, bytes_max, &more_data, &is_pure_ack, 0xaabbccddeeff0011ull);
        FRAME_FORMAT_TEST(picoquic_format_datagram_frame, bytes, bytes_max, &more_data, &is_pure_ack, 2, data);
        FRAME_FORMAT_TEST_ONCE(picoquic_format_ack_frequency_frame, 2, cnx, bytes, bytes_max, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_immediate_ack_frame, bytes, bytes_max, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_time_stamp_frame, cnx, buffer, bytes_max, &more_data, simulated_time);
        FRAME_FORMAT_TEST(picoquic_format_path_abandon_frame, bytes, bytes_max, &more_data, 1, 3);
        FRAME_FORMAT_TEST(picoquic_format_path_available_or_backup_frame, bytes, bytes_max, picoquic_frame_type_path_available, 1, 17, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_max_path_id_frame, bytes, bytes_max, 123, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_paths_blocked_frame, bytes, bytes_max, 123, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_path_cid_blocked_frame, bytes, bytes_max, 123, 0, &more_data);
        FRAME_FORMAT_TEST(picoquic_format_observed_address_frame, bytes, bytes_max, picoquic_frame_type_observed_address_v4, 13, addr_bytes, 4433, &more_data);
    }

    if (qclient != NULL) {
        picoquic_free(qclient);
    }

    return ret;
}
```

### Current Rust test body
```rust
fn frames_format() {
    run_frames_format_test().expect("frames_format");
}
```
