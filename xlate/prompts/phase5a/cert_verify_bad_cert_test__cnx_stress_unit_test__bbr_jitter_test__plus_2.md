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

## `picoquictest/cnxstress.c:cnx_stress_unit_test`
* C test-table name: `cnx_stress`
* C entry function: `cnx_stress_unit_test`
* Rust test: `cnx_stress`
* C source: `picoquictest/cnxstress.c:967-973`
* Rust source: `rs/fq/src/tests/cnxstress.rs:965-967`

### C test body
```c
{
    return cnx_stress_do_test(120000000, 100, 0);
}
```

### Rust test body
```rust
fn cnx_stress() {
    cnx_stress_do_test(120_000_000, 100, false).expect("cnx_stress_do_test");
}
```

## `picoquictest/congestion_test.c:bbr_jitter_test`
* C test-table name: `bbr_jitter`
* C entry function: `bbr_jitter_test`
* Rust test: `bbr_jitter`
* C source: `picoquictest/congestion_test.c:145-148`
* Rust source: `rs/fq/src/tests/congestion.rs:782-785`

### C test body
```c
{
    return congestion_control_test(picoquic_bbr_algorithm, 3600000, 5000, 5);
}
```

### Rust test body
```rust
fn bbr_jitter() {
    let ccalgo = get_congestion_algorithm("bbr").expect("bbr cc algo");
    congestion_control_test(ccalgo, 3_600_000, 5_000, 5);
}
```

## `picoquictest/congestion_test.c:bdp_delay_test`
* C test-table name: `bdp_delay`
* C entry function: `bdp_delay_test`
* Rust test: `bdp_delay`
* C source: `picoquictest/congestion_test.c:692-695`
* Rust source: `rs/fq/src/tests/congestion.rs:894-896`

### C test body
```c
{
    return bdp_option_test_one(bdp_test_option_delay);
}
```

### Rust test body
```rust
fn bdp_delay() {
    bdp_option_test_one(BdpTestOption::Delay);
}
```

## `picoquictest/congestion_test.c:c4_test`
* C test-table name: `c4`
* C entry function: `c4_test`
* Rust test: `c4`
* C source: `picoquictest/congestion_test.c:120-123`
* Rust source: `rs/fq/src/tests/congestion.rs:747-750`

### C test body
```c
{
    return congestion_control_test(c4_algorithm, 3600000, 0, 0);
}
```

### Rust test body
```rust
fn c4() {
    let ccalgo = get_congestion_algorithm("c4").expect("c4 cc algo");
    congestion_control_test(ccalgo, 3_600_000, 0, 0);
}
```
