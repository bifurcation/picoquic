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

## `picoquictest/l4s_test.c:l4s_reno_test`
* C test-table name: `l4s_reno`
* C entry function: `l4s_reno_test`
* Rust test: `l4s_reno`
* C source: `picoquictest/l4s_test.c:131-141`
* Rust source: `rs/fq/src/tests/l4s.rs:154-157`

### C test body
```c
{
    picoquic_congestion_algorithm_t* ccalgo = picoquic_newreno_algorithm;

    int ret = l4s_congestion_test(ccalgo, 1, 5600000, 45, 3000, 0, NULL);

    return ret;
}
```

### Rust test body
```rust
fn l4s_reno() {
    let ccalgo = get_congestion_algorithm("newreno").expect("newreno cc algo");
    l4s_congestion_test(ccalgo, true, 5_600_000, 45, 3_000, &[]);
}
```

## `picoquictest/mediatest.c:mediatest_video2_probe_test`
* C test-table name: `mediatest_video2_probe`
* C entry function: `mediatest_video2_probe_test`
* Rust test: `mediatest_video2_probe`
* C source: `picoquictest/mediatest.c:1418-1434`
* Rust source: `rs/fq/src/tests/mediatest.rs:304-317`

### C test body
```c
{
    int ret;
    mediatest_spec_t spec = { 0 };
    spec.ccalgo = picoquic_bbr_algorithm;
    spec.bandwidth = 0.1;
    spec.do_video = 1;
    spec.do_video2 = 1;
    spec.do_audio = 1;
    spec.data_size = 0;
    spec.latency_average = 25000;
    spec.latency_max = 150000;
    spec.do_probe_up = 1;
    ret = mediatest_one(mediatest_video2_probe, &spec);

    return ret;
}
```

### Rust test body
```rust
fn mediatest_video2_probe() {
    let spec = MediatestSpec {
        ccalgo: crate::get_congestion_algorithm("bbr"),
        bandwidth: 0.1,
        do_video: true,
        do_video2: true,
        do_audio: true,
        latency_average: 25_000,
        latency_max: 150_000,
        do_probe_up: true,
        ..Default::default()
    };
    mediatest_one(MediatestId::Video2Probe, &spec).expect("mediatest_video2_probe");
}
```

## `picoquictest/multipath_test.c:monopath_0rtt_loss_test`
* C test-table name: `monopath_0rtt_loss`
* C entry function: `monopath_0rtt_loss_test`
* Rust test: `monopath_0rtt_loss`
* C source: `picoquictest/multipath_test.c:1708-1723`
* Rust source: `rs/fq/src/tests/multipath.rs:1348-1358`

### C test body
```c
{
    int ret = 0;

    for (unsigned int i = 1; ret == 0 && i < 16; i++) {
        zero_rtt_test_t zrt = { 0 };
        zrt.early_loss = 1ull << i;
        zrt.do_multipath = 1;
        ret = zero_rtt_test_one(&zrt);
        if (ret != 0) {
            DBG_PRINTF("Monopath 0 RTT test fails when packet #%d is lost.\n", i);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn monopath_0rtt_loss() {
    for i in 1u32..16 {
        let zrt = ZeroRttTest {
            early_loss: 1u64 << i,
            do_multipath: true,
            ..ZeroRttTest::default()
        };
        zero_rtt_test_one(&zrt)
            .unwrap_or_else(|_| panic!("monopath_0rtt_loss fails at packet #{i}"));
    }
}
```

## `picoquictest/multipath_test.c:multipath_aead_test`
* C test-table name: `multipath_aead`
* C entry function: `multipath_aead_test`
* Rust test: `multipath_aead`
* C source: `picoquictest/multipath_test.c:1725-1797`
* Rust source: `rs/fq/src/tests/multipath.rs:1412-1476`

### C test body
```c
{
    int ret = 0;
    const uint8_t mp_aead_secret[32] = {
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        16, 17, 18, 19, 20, 21, 22, 23, 24, 35, 26, 27, 28, 29, 30, 31
    };
    /* Create AEAD contexts for encryption and decryption */
    void* aead_encrypt = picoquic_setup_test_aead_context(1, mp_aead_secret, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);
    void* aead_decrypt = picoquic_setup_test_aead_context(0, mp_aead_secret, PICOQUIC_LABEL_QUIC_V1_KEY_BASE);

    if (aead_encrypt == NULL || aead_decrypt == NULL) {
        DBG_PRINTF("%s", "Could not create the AEAD contexts.\n");
        ret = -1;
    }
    else {
        /* For a series of path_id, verify that encryption and decryption works */
        const uint64_t path_id_test[] = { 0, 1, 2, 0x0123456789abcdefull };
        const size_t nb_paths = sizeof(path_id_test) / sizeof(uint64_t);
        uint64_t sequence = 12345;
        const char* aad_str = "This is a test";
        const size_t aad_len = strlen(aad_str);
        const uint8_t* aad = (const uint8_t*)aad_str;
        const char* test_input_str = "The quick brown fox jumps over the lazy dog";
        const size_t test_input_len = strlen(test_input_str);
        const uint8_t* test_input = (const uint8_t*)test_input_str;
        uint8_t encrypted[256];
        uint8_t decrypted[256];
        size_t encrypted_length;
        size_t decrypted_length;

        for (size_t i = 0; ret == 0 &&  i < nb_paths; i++) {
            encrypted_length = picoquic_aead_encrypt_mp(encrypted, test_input, test_input_len,
                path_id_test[i], sequence, aad, aad_len, aead_encrypt);
            for (size_t j = 0; ret == 0 && j < nb_paths; j++) {
                decrypted_length = picoquic_aead_decrypt_mp(decrypted, encrypted, encrypted_length,
                    path_id_test[j], sequence, aad, aad_len, aead_decrypt);
                if (i != j) {
                    if (decrypted_length <= encrypted_length) {
                        DBG_PRINTF("Unexpected success, path id encode 0x%" PRIx64 ", decode 0x%"PRIx64"\n",
                            path_id_test[i], path_id_test[j]);
                        ret = -1;
                    }
                }
                else if (decrypted_length > encrypted_length) {
                    DBG_PRINTF("Unexpected error, path id 0x%" PRIx64 "\n", path_id_test[i]);
                    ret = -1;
                }
                else if (decrypted_length != test_input_len) {
                    DBG_PRINTF("Length don't match, path id 0x%" PRIx64 ", in: %zu, out %zu\n",
                        path_id_test[i], test_input_len, decrypted_length);
                    ret = -1;
                }
                else if (memcmp(decrypted, test_input, test_input_len) != 0) {
                    DBG_PRINTF("Decoded doesn't match encoded, path id 0x%" PRIx64 "\n", path_id_test[i]);
                    ret = -1;
                }
            }
        }
    }

    if (aead_encrypt != NULL) {
        picoquic_aead_free(aead_encrypt);
    }
    if (aead_decrypt != NULL) {
        picoquic_aead_free(aead_decrypt);
    }

    return ret;
}
```

### Rust test body
```rust
fn multipath_aead() {
    const MP_AEAD_SECRET: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        35, 26, 27, 28, 29, 30, 31,
    ];

    let aead_encrypt = setup_test_aead_context(true, &MP_AEAD_SECRET, LABEL_QUIC_V1_KEY_BASE)
        .expect("aead_encrypt context");
    let aead_decrypt = setup_test_aead_context(false, &MP_AEAD_SECRET, LABEL_QUIC_V1_KEY_BASE)
        .expect("aead_decrypt context");

    let path_id_test: &[u64] = &[0, 1, 2, 0x0123456789abcdef];
    let sequence = 12345u64;
    let aad = b"This is a test";
    let test_input = b"The quick brown fox jumps over the lazy dog";

    let mut encrypted = [0u8; 256];
    let mut decrypted = [0u8; 256];

    for (i, &enc_path) in path_id_test.iter().enumerate() {
        let encrypted_len = aead_encrypt_mp(
            &mut encrypted,
            test_input,
            enc_path,
            sequence,
            aad,
            aead_encrypt.as_ref(),
        );

        for (j, &dec_path) in path_id_test.iter().enumerate() {
            let result = aead_decrypt_mp(
                &mut decrypted,
                &encrypted[..encrypted_len],
                dec_path,
                sequence,
                aad,
                aead_decrypt.as_ref(),
            );

            if i != j {
                assert!(
                    result.is_none(),
                    "unexpected decrypt success: enc path 0x{:x}, dec path 0x{:x}",
                    enc_path,
                    dec_path
                );
            } else {
                let dec_len = result
                    .unwrap_or_else(|| panic!("unexpected decrypt error, path 0x{:x}", enc_path));
                assert_eq!(
                    dec_len,
                    test_input.len(),
                    "length mismatch at path 0x{:x}",
                    enc_path
                );
                assert_eq!(
                    &decrypted[..dec_len],
                    test_input,
                    "decoded mismatch at path 0x{:x}",
                    enc_path
                );
            }
        }
    }
}
```

## `picoquictest/multipath_test.c:multipath_datagram_test`
* C test-table name: `multipath_datagram`
* C entry function: `multipath_datagram_test`
* Rust test: `multipath_datagram`
* C source: `picoquictest/multipath_test.c:1489-1495`
* Rust source: `rs/fq/src/tests/multipath.rs:1522-1524`

### C test body
```c
{
    /* TODO: investigate why 1.15 instead of 1.12 with prior implementation of multipath */
    uint64_t max_completion_microsec = 1150000;

    return multipath_test_one(max_completion_microsec, multipath_test_datagram);
}
```

### Rust test body
```rust
fn multipath_datagram() {
    multipath_test_one(1_150_000, MultipathTestId::Datagram);
}
```
