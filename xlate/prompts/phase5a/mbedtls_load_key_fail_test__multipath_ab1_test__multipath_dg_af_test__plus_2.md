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

## `picoquictest/mbedtls_test.c:mbedtls_load_key_fail_test`
* C test-table name: `mbedtls_load_key_fail`
* C entry function: `mbedtls_load_key_fail_test`
* Rust test: `mbedtls_load_key_fail`
* C source: `picoquictest/mbedtls_test.c:737-780`
* Rust source: `rs/fq/src/tests/mbedtls.rs:440-442`

### C test body
```c
{
    int ret = 0;


    if ((ret = ptls_mbedtls_init()) != 0) {
        DBG_PRINTF("%s", "psa_crypto_init fails.");
    }
    else {
        if (ret == 0 && mbedtls_test_load_one_der_key(ASSET_NO_SUCH_FILE) == 0)
        {
            ret = -1;
        }

        if (ret == 0 && mbedtls_test_load_one_der_key(ASSET_NOT_A_PEM_FILE) == 0)
        {
            ret = -1;
        }

        if (ret == 0 && mbedtls_test_load_one_der_key(ASSET_RSA_CERT) == 0)
        {
            ret = -1;
        }

        if (ret == 0 && mbedtls_test_load_one_der_key(ASSET_ED25519_KEY) == 0)
        {
            ret = -1;
        }

        /* Deinitialize the PSA crypto library. */
        ptls_mbedtls_free();
    }

    return ret;
}
```

### Rust test body
```rust
fn mbedtls_load_key_fail() {
    mbedtls_test_load_key_fail_cases().expect("load_key_fail_cases");
}
```

## `picoquictest/multipath_test.c:multipath_ab1_test`
* C test-table name: `multipath_ab1`
* C entry function: `multipath_ab1_test`
* Rust test: `multipath_ab1`
* C source: `picoquictest/multipath_test.c:1315-1320`
* Rust source: `rs/fq/src/tests/multipath.rs:1400-1402`

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return multipath_test_one(max_completion_microsec, multipath_test_ab1);
}
```

### Rust test body
```rust
fn multipath_ab1() {
    multipath_test_one(3_000_000, MultipathTestId::Ab1);
}
```

## `picoquictest/multipath_test.c:multipath_dg_af_test`
* C test-table name: `multipath_dg_af`
* C entry function: `multipath_dg_af_test`
* Rust test: `multipath_dg_af`
* C source: `picoquictest/multipath_test.c:1497-1502`
* Rust source: `rs/fq/src/tests/multipath.rs:1528-1530`

### C test body
```c
{
    uint64_t max_completion_microsec = 1100000;

    return multipath_test_one(max_completion_microsec, multipath_test_dg_af);
}
```

### Rust test body
```rust
fn multipath_dg_af() {
    multipath_test_one(1_100_000, MultipathTestId::DgAf);
}
```

## `picoquictest/multipath_test.c:multipath_nat_challenge_test`
* C test-table name: `multipath_nat_challenge`
* C entry function: `multipath_nat_challenge_test`
* Rust test: `multipath_nat_challenge`
* C source: `picoquictest/multipath_test.c:1380-1385`
* Rust source: `rs/fq/src/tests/multipath.rs:1576-1578`

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_nat_challenge);
}
```

### Rust test body
```rust
fn multipath_nat_challenge() {
    multipath_test_one(3_000_000, MultipathTestId::NatChallenge);
}
```

## `picoquictest/multipath_test.c:multipath_socket_error_test`
* C test-table name: `multipath_socket_error`
* C entry function: `multipath_socket_error_test`
* Rust test: `multipath_socket_error`
* C source: `picoquictest/multipath_test.c:1397-1404`
* Rust source: `rs/fq/src/tests/multipath.rs:1633-1635`

### C test body
```c
{
    uint64_t max_completion_microsec = 11000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_break2);
}
```

### Rust test body
```rust
fn multipath_socket_error() {
    multipath_test_one(11_000_000, MultipathTestId::Break2);
}
```
