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

## `picoquictest/multipath_test.c:multipath_back0_test`
* C test-table name: `multipath_back0`
* C entry function: `multipath_back0_test`
* Rust test: `multipath_back0`
* C source: `picoquictest/multipath_test.c:1424-1432`
* Rust source: `rs/fq/src/tests/multipath.rs:1480-1482`

### C test body
```c
{
    uint64_t max_completion_microsec = 3300000;

    return  multipath_test_one(max_completion_microsec, multipath_test_back0);
}
```

### Rust test body
```rust
fn multipath_back0() {
    multipath_test_one(3_300_000, MultipathTestId::Back0);
}
```

## `picoquictest/multipath_test.c:multipath_quality_test`
* C test-table name: `multipath_quality`
* C entry function: `multipath_quality_test`
* Rust test: `multipath_quality`
* C source: `picoquictest/multipath_test.c:1467-1472`
* Rust source: `rs/fq/src/tests/multipath.rs:1603-1605`

### C test body
```c
{
    /* we do not run this test on Win32 builds */
    return 0;
}
```

### Rust test body
```rust
fn multipath_quality() {
    multipath_test_one(1_000_000, MultipathTestId::Quality);
}
```

## `picoquictest/netperf_test.c:netperf_bbr_test`
* C test-table name: `netperf_bbr`
* C entry function: `netperf_bbr_test`
* Rust test: `netperf_bbr`
* C source: `picoquictest/netperf_test.c:493-500`
* Rust source: `rs/fq/src/tests/netperf.rs:237-246`

### C test body
```c
{
    int ret = netperf_one_scenario(netperf_scenario_basic, sizeof(netperf_scenario_basic),
        picoquic_bbr_algorithm,
        0, 0, 0, 0, 0, 1000000, NULL, NULL, 10 * PICOQUIC_MAX_PACKET_SIZE);

    return ret;
}
```

### Rust test body
```rust
fn netperf_bbr() {
    let algo = get_congestion_algorithm("bbr").expect("bbr algorithm");
    netperf_one_scenario(
        NETPERF_SCENARIO_BASIC,
        Some(algo),
        0,
        1_000_000,
        10 * MAX_PACKET_SIZE,
    );
}
```

## `picoquictest/pacing_test.c:pacing_newreno_test`
* C test-table name: `pacing_newreno`
* C entry function: `pacing_newreno_test`
* Rust test: `pacing_newreno`
* C source: `picoquictest/pacing_test.c:244-248`
* Rust source: `rs/fq/src/tests/pacing.rs:634-636`

### C test body
```c
{
    int ret = pacing_cc_algotest(picoquic_newreno_algorithm, 900000, 100);
    return ret;
}
```

### Rust test body
```rust
fn pacing_newreno() {
    pacing_cc_algotest("newreno", 900_000, 100);
}
```

## `picoquictest/picoquic_lb_test.c:cid_for_lb_cli_test`
* C test-table name: `cid_for_lb_cli`
* C entry function: `cid_for_lb_cli_test`
* Rust test: `cid_for_lb_cli`
* C source: `picoquictest/picoquic_lb_test.c:1137-1214`
* Rust source: `rs/fq/src/tests/picoquic_lb.rs:920-985`

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

### Rust test body
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
                    let s = String::from_utf8_lossy(&buf);
                    if Config::parse(&s).is_ok() {
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
