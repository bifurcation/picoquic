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

## `picoquictest/cert_verify_test.c:cert_verify_bad_cert_test`
* C test-table name: `cert_verify_bad_cert`
* C entry function: `cert_verify_bad_cert_test`
* Rust test: `cert_verify_bad_cert`
* C source: `picoquictest/cert_verify_test.c:228-235`
* Rust source: `rs/fq/src/tests/cert_verify.rs:42-50`

### C test body
```c
{
    int ret = cert_verify_test_one(0, CERT_VERIFY_RSA_BAD_CERT, CERT_VERIFY_RSA_KEY,
        CERT_VERIFY_TEST_CA, CERT_VERIFY_TEST_SNI);
    return ret;
}
```

### Rust test body
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

## `picoquictest/config_test.c:config_quic_test`
* C test-table name: `config_quic`
* C entry function: `config_quic_test`
* Rust test: `config_quic`
* C source: `picoquictest/config_test.c:772-782`
* Rust source: `rs/fq/src/tests/config.rs:478-573`

### C test body
```c
{
    int ret = 0;
    config_test_register_cc_algorithms();

    if (config_quic_test_one(&param1) != 0 ||
        config_quic_test_one(&param2) != 0) {
        ret = -1;
    }
    return ret;
}
```

### Rust test body
```rust
fn config_quic() {
    fn config_quic_test_one(mut config: Config) {
        use crate::tests::util::{
            TEST_ECH_CONFIG, TEST_ECH_PRIVATE_KEY, TEST_FILE_CERT_STORE, TEST_FILE_SERVER_CERT,
            TEST_FILE_SERVER_KEY,
        };

        // Swap in test-fixture paths where the config has non-None file fields,
        // mirroring the C `picoquic_get_input_path` substitutions.
        if config.server_cert_file.is_some() {
            config.server_cert_file = Some(fixture_path(TEST_FILE_SERVER_CERT));
        }
        if config.server_key_file.is_some() {
            config.server_key_file = Some(fixture_path(TEST_FILE_SERVER_KEY));
        }
        if config.root_trust_file.is_some() {
            config.root_trust_file = Some(fixture_path(TEST_FILE_CERT_STORE));
        }
        if config.ech_key_file.is_some() {
            config.ech_key_file = Some(fixture_path(TEST_ECH_PRIVATE_KEY));
        }
        if config.ech_config_file.is_some() {
            config.ech_config_file = Some(fixture_path(TEST_ECH_CONFIG));
        }

        let quic = config
            .create_and_configure(None, Instant::from_ticks(0), None)
            .expect("create_and_configure");

        // Check max connections.
        if config.nb_connections > 0 {
            assert_eq!(
                quic.max_nb_connections(),
                config.nb_connections,
                "max_nb_connections"
            );
        }

        // Check default ALPN.
        if let Some(alpn) = config.alpn.as_deref() {
            assert_eq!(quic.default_alpn_string(), Some(alpn), "default_alpn");
        }

        // Check reset seed.
        if config.has_reset_seed {
            assert_eq!(
                quic.reset_seed_bytes(),
                config.reset_seed.as_ref(),
                "reset_seed"
            );
        }

        // Check default congestion algorithm.
        if let Some(cc_id) = config.cc_algo_id.as_deref() {
            assert_eq!(
                quic.default_congestion_algorithm_id(),
                Some(cc_id),
                "cc_algo_id"
            );
        }

        // Check flow-control / initial max data.
        if config.flow_control_max != 0 {
            assert_eq!(
                quic.max_data_limit(),
                config.flow_control_max,
                "max_data_limit"
            );
            assert_eq!(
                quic.default_tp().initial_max_data,
                config.flow_control_max,
                "initial_max_data"
            );
        } else {
            assert_eq!(quic.max_data_limit(), 0, "max_data_limit (default)");
            assert_eq!(
                quic.default_tp().initial_max_data,
                0x0010_0000,
                "initial_max_data (default)"
            );
        }

        // Check preferred address is populated when either V4 or V6 is set.
        if config.preferred_address_v4.is_some() || config.preferred_address_v6.is_some() {
            let pa = &quic.default_tp().preferred_address;
            assert!(
                pa.v4.is_some() || pa.v6.is_some(),
                "preferred_address should be defined"
            );
        }
    }

    config_test_register_cc_algorithms();
    config_quic_test_one(parse_argv(ARGV1).expect("param1 config"));
    config_quic_test_one(parse_argv(ARGV2).expect("param2 config"));
}
```

## `picoquictest/congestion_test.c:bbr_slow_long_test`
* C test-table name: `bbr_slow_long`
* C entry function: `bbr_slow_long_test`
* Rust test: `bbr_slow_long`
* C source: `picoquictest/congestion_test.c:342-353`
* Rust source: `rs/fq/src/tests/congestion.rs:816-821`

### C test body
```c
{
    uint64_t max_completion_time = 81000000;
    uint64_t latency = 300000;
    uint64_t jitter = 3000;
    uint64_t buffer = 2 * (latency + jitter);
    uint64_t mbps = 1;

    int ret = performance_test(max_completion_time, mbps, latency, jitter, buffer);

    return ret;
}
```

### Rust test body
```rust
fn bbr_slow_long() {
    let latency = 300_000u64;
    let jitter = 3_000u64;
    let buffer = 2 * (latency + jitter);
    performance_test(81_000_000, 1, latency, jitter, buffer);
}
```

## `picoquictest/congestion_test.c:bdp_short_test`
* C test-table name: `bdp_short`
* C entry function: `bdp_short_test`
* Rust test: `bdp_short`
* C source: `picoquictest/congestion_test.c:702-705`
* Rust source: `rs/fq/src/tests/congestion.rs:924-926`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_short);
}
```

### Rust test body
```rust
fn bdp_short() {
    bdp_option_test_one(BdpTestOption::Short);
}
```

## `picoquictest/congestion_test.c:cubic_jitter_test`
* C test-table name: `cubic_jitter`
* C entry function: `cubic_jitter_test`
* Rust test: `cubic_jitter`
* C source: `picoquictest/congestion_test.c:115-118`
* Rust source: `rs/fq/src/tests/congestion.rs:740-743`

### C test body
```c
{
    return congestion_control_test(picoquic_cubic_algorithm, 3550000, 5000, 5);
}
```

### Rust test body
```rust
fn cubic_jitter() {
    let ccalgo = get_congestion_algorithm("cubic").expect("cubic cc algo");
    congestion_control_test(ccalgo, 3_550_000, 5_000, 5);
}
```
