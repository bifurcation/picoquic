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

## `picoquictest/tls_api_test.c:grease_quic_bit_test`
* C test-table name: `grease_quic_bit`
* C entry function: `grease_quic_bit_test`
* Rust test: `grease_quic_bit`
* C source: `picoquictest/tls_api_test.c:11027-11030`
* Rust source: `rs/fq/src/tests/tls_api.rs:367-369`

### C test body
```c
{
    return  grease_quic_bit_test_one(0);
}
```

### Rust test body
```rust
fn grease_quic_bit() {
    grease_quic_bit_test_one(false).expect("grease_quic_bit");
}
```

## `picoquictest/tls_api_test.c:key_rotation_auto_server`
* C test-table name: `key_rotation_server`
* C entry function: `key_rotation_auto_server`
* Rust test: `key_rotation_server`
* C source: `picoquictest/tls_api_test.c:7976-7979`
* Rust source: `rs/fq/src/tests/tls_api.rs:498-500`

### C test body
```c
{
    return key_rotation_auto_one(300, 0);
}
```

### Rust test body
```rust
fn key_rotation_server() {
    key_rotation_auto_one(300, false).expect("key_rotation_server");
}
```

## `picoquictest/tls_api_test.c:migration_test`
* C test-table name: `migration`
* C entry function: `migration_test`
* Rust test: `migration`
* C source: `picoquictest/tls_api_test.c:6849-6852`
* Rust source: `rs/fq/src/tests/tls_api.rs:574-576`

### C test body
```c
{
    return migration_test_scenario(test_scenario_q_and_r, sizeof(test_scenario_q_and_r), 0, 0);
}
```

### Rust test body
```rust
fn migration() {
    migration_test_scenario(&[], 0, false).expect("migration");
}
```

## `picoquictest/tls_api_test.c:mtu_discovery_test`
* C test-table name: `mtu_discovery`
* C entry function: `mtu_discovery_test`
* Rust test: `mtu_discovery`
* C source: `picoquictest/tls_api_test.c:4974-4979`
* Rust source: `rs/fq/src/tests/tls_api.rs:642-644`

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_basic, 1440, 1440, 
        test_scenario_mtu_discovery, sizeof(test_scenario_mtu_discovery), 0);
    return ret;
}
```

### Rust test body
```rust
fn mtu_discovery() {
    mtu_discovery_test_one(0, 1440, 1440, 2_500_000, 0).expect("mtu_discovery");
}
```

## `picoquictest/tls_api_test.c:multi_segment_test`
* C test-table name: `multi_segment`
* C entry function: `multi_segment_test`
* Rust test: `multi_segment`
* C source: `picoquictest/tls_api_test.c:11205-11231`
* Rust source: `rs/fq/src/tests/tls_api.rs:708-712`

### C test body
```c
{
    picoquic_congestion_algorithm_t* algo_list[5] = {
        picoquic_newreno_algorithm,
        picoquic_cubic_algorithm,
        picoquic_dcubic_algorithm,
        picoquic_fastcc_algorithm,
        picoquic_bbr_algorithm
    };
    uint64_t algo_time[5] = {
        1220000,
        1050000,
        1250000,
        1350000,
        1280000
    };
    int ret = 0;

    for (int i = 0; i < 5 && ret == 0; i++) {
        ret = multi_segment_test_one(algo_list[i], algo_time[i], 65536);
        if (ret != 0) {
            DBG_PRINTF("Multi segment test fails for CC=%s", algo_list[i]->congestion_algorithm_id);
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn multi_segment() {
    let mut t = Instant::from_ticks(0);
    let mut ctx = tls_api_init_ctx(&mut t, V1, None).expect("ctx");
    tls_api_one_scenario_body(&mut ctx, &mut t, &[], 0, 0, 0, 0, 6_000_000).expect("multi_segment");
}
```
