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

## `picoquictest/satellite_test.c:satellite_small_test`
* C test-table name: `satellite_small`
* C entry function: `satellite_small_test`
* Rust test: `satellite_small`
* C source: `picoquictest/satellite_test.c:265-269`
* Rust source: `rs/fq/src/tests/satellite.rs:342-357`

### C test body
```c
{
    /* Should be less than 85 sec per draft etosat. */
    return satellite_test_one(picoquic_bbr_algorithm, 100000000, 81500000, 10, 2, 0, 0, 0, 0, 0, 0);
}
```

### Rust test body
```rust
fn satellite_small() {
    let bbr = get_congestion_algorithm("bbr").expect("bbr");
    satellite_test_one(
        bbr,
        100_000_000,
        81_500_000,
        10,
        2,
        0,
        false,
        false,
        false,
        false,
        false,
    );
}
```

## `picoquictest/spinbit_test.c:spinbit_randclient_test`
* C test-table name: `spinbit_randclient`
* C entry function: `spinbit_randclient_test`
* Rust test: `spinbit_randclient`
* C source: `picoquictest/spinbit_test.c:200-203`
* Rust source: `rs/fq/src/tests/spinbit.rs:150-152`

### C test body
```c
{
    return spinbit_test_one(picoquic_spinbit_random, picoquic_spinbit_basic);
}
```

### Rust test body
```rust
fn spinbit_randclient() {
    spinbit_test_one(SpinbitVersion::Random, SpinbitVersion::Basic).expect("spinbit_randclient");
}
```

## `picoquictest/ticket_store_test.c:ticket_seed_test`
* C test-table name: `ticket_seed`
* C entry function: `ticket_seed_test`
* Rust test: `ticket_seed`
* C source: `picoquictest/ticket_store_test.c:734-737`
* Rust source: `rs/fq/src/tests/ticket_store.rs:16-18`

### C test body
```c
int ticket_seed_test(void) {
    
   return ticket_seed_test_one(1);
}
```

### Rust test body
```rust
fn ticket_seed() {
    ticket_seed_test_one(1).expect("ticket_seed");
}
```

## `picoquictest/tls_api_test.c:request_client_authentication_test`
* C test-table name: `client_auth`
* C entry function: `request_client_authentication_test`
* Rust test: `client_auth`
* C source: `picoquictest/tls_api_test.c:5750-5793`
* Rust source: `rs/fq/src/tests/tls_api.rs:128-131`

### C test body
```c
{
    char test_client_cert_file[512];
    char test_client_key_file[512];
    char test_server_cert_file[512];
    char test_server_key_file[512];
    char test_ca_cert_store_file[512];
    int ret = 0;

    ret = picoquic_get_input_path(test_client_cert_file, sizeof(test_client_cert_file),
                                  picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_CERT_RSA);

    if (ret == 0) {
        ret = picoquic_get_input_path(test_client_key_file, sizeof(test_client_key_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY_RSA);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_cert_file, sizeof(test_server_cert_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_CERT);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_server_key_file, sizeof(test_server_key_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_SERVER_KEY);
    }

    if (ret == 0) {
        ret = picoquic_get_input_path(test_ca_cert_store_file, sizeof(test_ca_cert_store_file),
                                      picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);
    }

    if (ret == 0) {
        ret = request_client_authentication_test_one(test_client_cert_file, test_client_key_file,
                                                     test_server_cert_file, test_server_key_file,
                                                     test_ca_cert_store_file);
    }

    if (ret != 0) {
        DBG_PRINTF("%s", "mTLS client-auth test failed RSA\n");
    }

    return ret;
}
```

### Rust test body
```rust
fn client_auth() {
    request_client_authentication_test_one(TEST_FILE_SERVER_CERT, TEST_FILE_SERVER_KEY)
        .expect("client_auth");
}
```

## `picoquictest/tls_api_test.c:transmit_cnxid_test`
* C test-table name: `cnxid_transmit`
* C entry function: `transmit_cnxid_test`
* Rust test: `cnxid_transmit`
* C source: `picoquictest/tls_api_test.c:6611-6614`
* Rust source: `rs/fq/src/tests/tls_api.rs:205-207`

### C test body
```c
{
    return transmit_cnxid_test_one(0, 0, 0);
}
```

### Rust test body
```rust
fn cnxid_transmit() {
    transmit_cnxid_test_one(false, false, false).expect("cnxid_transmit");
}
```
