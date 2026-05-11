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

## `picoquictest/multipath_test.c:multipath_break1_test`
* C test-table name: `multipath_break1`
* C entry function: `multipath_break1_test`
* Rust test: `multipath_break1`
* C source: `picoquictest/multipath_test.c:1388-1395`
* Rust source: `rs/fq/src/tests/multipath.rs:1504-1506`

### C test body
```c
{
    uint64_t max_completion_microsec = 10800000;

    return  multipath_test_one(max_completion_microsec, multipath_test_break1);
}
```

### Rust test body
```rust
fn multipath_break1() {
    multipath_test_one(10_800_000, MultipathTestId::Break1);
}
```

## `picoquictest/multipath_test.c:multipath_nat_test`
* C test-table name: `multipath_nat`
* C entry function: `multipath_nat_test`
* Rust test: `multipath_nat`
* C source: `picoquictest/multipath_test.c:1372-1378`
* Rust source: `rs/fq/src/tests/multipath.rs:1570-1572`

### C test body
```c
{
    uint64_t max_completion_microsec = 3000000;

    return  multipath_test_one(max_completion_microsec, multipath_test_nat);
}
```

### Rust test body
```rust
fn multipath_nat() {
    multipath_test_one(3_000_000, MultipathTestId::Nat);
}
```

## `picoquictest/multipath_test.c:multipath_socket0_error_test`
* C test-table name: `multipath_socket0_error`
* C entry function: `multipath_socket0_error_test`
* Rust test: `multipath_socket0_error`
* C source: `picoquictest/multipath_test.c:1406-1413`
* Rust source: `rs/fq/src/tests/multipath.rs:1627-1629`

### C test body
```c
{
    uint64_t max_completion_microsec = 10900000;

    return  multipath_test_one(max_completion_microsec, multipath_test_break3);
}
```

### Rust test body
```rust
fn multipath_socket0_error() {
    multipath_test_one(10_900_000, MultipathTestId::Break3);
}
```

## `picoquictest/openssl_test.c:openssl_cert_test`
* C test-table name: `openssl_cert`
* C entry function: `openssl_cert_test`
* Rust test: `openssl_cert`
* C source: `picoquictest/openssl_test.c:45-50`
* Rust source: `rs/fq/src/tests/openssl.rs:26-30`

### C test body
```c
{
    /* Nothing to do, as the module is not loaded. */
    return 0;
}
```

### Rust test body
```rust
fn openssl_cert() {
    openssl_cert_test_one("cert.pem", 1);
    openssl_cert_test_one("chain.pem", 1);
    openssl_cert_test_one("fullchain.pem", 2);
}
```
