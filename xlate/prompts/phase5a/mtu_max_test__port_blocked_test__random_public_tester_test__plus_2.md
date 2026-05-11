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

## `picoquictest/tls_api_test.c:mtu_max_test`
* C test-table name: `mtu_max`
* C entry function: `mtu_max_test`
* Rust test: `mtu_max`
* C source: `picoquictest/tls_api_test.c:5002-5007`
* Rust source: `rs/fq/src/tests/tls_api.rs:690-692`

### C test body
```c
{
    int ret = mtu_discovery_test_one(picoquic_pmtud_basic, 1420, 1392,
        test_scenario_mtu_discovery, sizeof(test_scenario_mtu_discovery), 1420);
    return ret;
}
```

### Rust test body
```rust
fn mtu_max() {
    mtu_discovery_test_one(0, 1420, 1392, 2_500_000, 1420).expect("mtu_max");
}
```

## `picoquictest/tls_api_test.c:port_blocked_test`
* C test-table name: `port_blocked`
* C entry function: `port_blocked_test`
* Rust test: `port_blocked`
* C source: `picoquictest/tls_api_test.c:12670-12687`
* Rust source: `rs/fq/src/tests/tls_api.rs:912-914`

### C test body
```c
{
    int ret = 0;
    const uint16_t blocked_port_to_test[] = { 0, 53, 138, 1900, 5353, 11211 };
    const uint16_t unblocked_port_to_test[] = { 443, 4433, 33721 };
    size_t nb_blocked = sizeof(blocked_port_to_test) / sizeof(uint16_t);
    size_t nb_unblocked = sizeof(unblocked_port_to_test) / sizeof(uint16_t);

    for (size_t i = 0; ret == 0 && i < nb_blocked; i++) {
        ret = port_blocked_test_port(blocked_port_to_test[i], 1);
    }

    for (size_t i = 0; ret == 0 && i < nb_unblocked; i++) {
        ret = port_blocked_test_port(unblocked_port_to_test[i], 0);
    }

    return ret;
}
```

### Rust test body
```rust
fn port_blocked() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("port_blocked");
}
```

## `picoquictest/tls_api_test.c:random_public_tester_test`
* C test-table name: `random_public_tester`
* C entry function: `random_public_tester_test`
* Rust test: `random_public_tester`
* C source: `picoquictest/tls_api_test.c:10088-10131`
* Rust source: `rs/fq/src/tests/tls_api.rs:1027-1032`

### C test body
```c
{
#define RANDOM_PUBLIC_TEST_CONST 11
#define RANDOM_PUBLIC_TEST_ROUNDS 100
#define RANDOM_PUBLIC_CHI_SQUARE 18.31 /* Fail if significance of bias < P = 0.05 */
    int ret = 0;
    int r_count[RANDOM_PUBLIC_TEST_CONST];

    picoquic_public_random_seed_64(RANDOM_PUBLIC_TEST_SEED, 1);

    memset(r_count, 0, sizeof(r_count));

    for (int i = 0; i < RANDOM_PUBLIC_TEST_CONST*RANDOM_PUBLIC_TEST_ROUNDS; i++) {
        uint64_t x = picoquic_public_uniform_random(RANDOM_PUBLIC_TEST_CONST);

        if (x >= RANDOM_PUBLIC_TEST_CONST) {
            DBG_PRINTF("Value %d >= %d\n", x, RANDOM_PUBLIC_TEST_CONST);
            ret = -1;
            break;
        }
        else {
            r_count[x] += 1;
        }
    }

    if (ret == 0) {
        double chi_squared = 0;

        for (int i = 0; i < RANDOM_PUBLIC_TEST_CONST; i++) {
            double delta = ((double)RANDOM_PUBLIC_TEST_ROUNDS - r_count[i]);
            double d2 = delta * delta;
            d2 /= ((double)RANDOM_PUBLIC_TEST_ROUNDS);
            chi_squared += d2;
        }

        if (chi_squared > RANDOM_PUBLIC_CHI_SQUARE) {
            DBG_PRINTF("Chi2 = %f, larger than %f\n", chi_squared, RANDOM_PUBLIC_CHI_SQUARE);
            ret = -1;
        }
    }

    return ret;
}
```

### Rust test body
```rust
fn random_public_tester() {
    for _ in 0..100 {
        tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN))
            .expect("random_public_tester");
    }
}
```

## `picoquictest/tls_api_test.c:red_newreno_test`
* C test-table name: `red_newreno`
* C entry function: `red_newreno_test`
* Rust test: `red_newreno`
* C source: `picoquictest/tls_api_test.c:11114-11118`
* Rust source: `rs/fq/src/tests/tls_api.rs:1102-1104`

### C test body
```c
{
    int ret = red_cc_algotest(picoquic_newreno_algorithm, 500000, 150);
    return ret;
}
```

### Rust test body
```rust
fn red_newreno() {
    red_cc_algotest("newreno", 500_000, 150).expect("red_newreno");
}
```

## `picoquictest/tls_api_test.c:tls_api_server_losses_test`
* C test-table name: `server_losses`
* C entry function: `tls_api_server_losses_test`
* Rust test: `server_losses`
* C source: `picoquictest/tls_api_test.c:4175-4178`
* Rust source: `rs/fq/src/tests/tls_api.rs:1180-1182`

### C test body
```c
{
    return tls_api_loss_test(6ull);
}
```

### Rust test body
```rust
fn server_losses() {
    tls_api_loss_test(6).expect("server_losses");
}
```
