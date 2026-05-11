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

## `picoquictest/tls_api_test.c:virtual_time_test`
* C test-table name: `virtual_time`
* C entry function: `virtual_time_test`
* Rust test: `virtual_time`
* C source: `picoquictest/tls_api_test.c:5431-5531`
* Rust source: `rs/fq/src/tests/tls_api.rs:1526-1528`

### C test body
```c
{
    int ret = 0;
    uint64_t test_time = 0;
    uint64_t simulated_time = 0;
    uint64_t current_time = picoquic_current_time();
    uint64_t ptls_time = 0;
    uint8_t callback_ctx[256];
    char test_server_cert_store_file[512];
    picoquic_quic_t * qsimul = NULL;
    picoquic_quic_t * qdirect = NULL;

    ret = picoquic_get_input_path(test_server_cert_store_file, sizeof(test_server_cert_store_file), picoquic_solution_dir, PICOQUIC_TEST_FILE_CERT_STORE);

    if (ret != 0) {
        DBG_PRINTF("%s", "Cannot set the cert, key or store file names.\n");
    }
    else {
        qsimul = picoquic_create(8, NULL, NULL, test_server_cert_store_file,
            NULL, test_api_callback,
            (void*)callback_ctx, NULL, NULL, NULL, simulated_time,
            &simulated_time, ticket_file_name, NULL, 0);
        qdirect = picoquic_create(8, NULL, NULL, PICOQUIC_TEST_FILE_CERT_STORE,
            NULL, test_api_callback,
            (void*)callback_ctx, NULL, NULL, NULL, current_time,
            NULL, ticket_file_name, NULL, 0);

        if (qsimul == NULL || qdirect == NULL)
        {
            ret = -1;
        }
        else
        {
            /* Check that the simulated time follows the simulation */
            for (int i = 0; ret == 0 && i < 5; i++) {
                simulated_time += 12345678000;
                test_time = picoquic_get_quic_time(qsimul);
                ptls_time = picoquic_get_tls_time(qsimul);
                if (test_time != simulated_time) {
                    DBG_PRINTF("Test time: %llu != Simulated: %llu",
                        (unsigned long long)test_time,
                        (unsigned long long)simulated_time);
                    ret = -1;
                }
                else if (ptls_time < test_time || ptls_time > test_time + 1000) {
                    DBG_PRINTF("Test time: %llu does match ptls time: %llu",
                        (unsigned long long)test_time,
                        (unsigned long long)ptls_time);
                    ret = -1;
                }
            }
        }

        if (ret == 0) {
            int64_t delta, delta_low, delta_high;
            uint64_t current_previous = picoquic_current_time();
            uint64_t test_previous = picoquic_current_time();
            uint64_t ptls_previous = picoquic_get_tls_time(qdirect);

            /* Check that the non simulated time follows the current time */
            for (int i = 0; ret == 0 && i < 5; i++) {
#ifdef _WINDOWS
                Sleep(1);
#else
                usleep(1000);
#endif
                current_time = picoquic_current_time();
                test_time = picoquic_get_quic_time(qdirect);
                ptls_time = picoquic_get_tls_time(qdirect);

                delta = current_time - current_previous;
                delta_low = delta - 1000;
                delta_high = delta + 1000;
                if (test_time < test_previous + delta_low || test_time > test_previous + delta_high ) {
                    DBG_PRINTF("Test time: %" PRIu64 " does not match previous test time : %" PRIu64 " + delta : %" PRId64,
                        test_time, test_previous, delta);
                    ret = -1;
                }
                else if (ptls_time < ptls_previous + delta_low || ptls_time > ptls_previous + delta_high) {
                    DBG_PRINTF("Test time: %" PRIu64 " does not match previous test time : %" PRIu64 " + delta : %" PRId64,
                        ptls_time, ptls_previous, delta);
                    ret = -1;
                }
            }
        }
    }

    if (qsimul != NULL)
    {
        picoquic_free(qsimul);
        qsimul = NULL;
    }

    if (qdirect != NULL)
    {
        picoquic_free(qdirect);
        qdirect = NULL;
    }

    return ret;
}
```

### Rust test body
```rust
fn virtual_time() {
    tls_api_test_with_loss(None, V1, Some(TEST_SNI), Some(TEST_ALPN)).expect("virtual_time");
}
```
