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

## `picoquictest/transport_param_test.c:transport_param_log_test`
* C test-table name: `transport_param_log`
* C entry function: `transport_param_log_test`
* Rust test: `transport_param_log`
* C source: `picoquictest/transport_param_test.c:1055-1102`
* Rust source: `rs/fq/src/tests/transport_param.rs:94-97`

### C test body
```c
{
    FILE* F = NULL;
    int ret = 0;

    if ((F = picoquic_file_open(log_tp_test_file, "w")) == NULL) {
        fprintf(stderr, "failed to open file:%s\n", log_tp_test_file);
        ret = PICOQUIC_ERROR_INVALID_FILE;
    }

    if (F != NULL) {
        char log_tp_test_ref[512];

        transport_param_log_test_one(F, client_param1, sizeof(client_param1));
        transport_param_log_test_one(F, client_param2, sizeof(client_param2));
        transport_param_log_test_one(F, client_param3, sizeof(client_param3));
        transport_param_log_test_one(F, server_param1, sizeof(server_param1));
        transport_param_log_test_one(F, server_param2, sizeof(server_param2));
        transport_param_log_test_one(F, client_param4, sizeof(client_param4));
        transport_param_log_test_one(F, client_param5, sizeof(client_param5));
        transport_param_log_test_one(F, server_param3, sizeof(server_param3));

        fclose(F);

        ret = picoquic_get_input_path(log_tp_test_ref, sizeof(log_tp_test_ref), picoquic_solution_dir, LOG_TP_TEST_REF);

        if (ret != 0) {
            DBG_PRINTF("%s", "Cannot set the log TP ref file name.\n");
        } else {
            ret = picoquic_test_compare_text_files(log_tp_test_file, log_tp_test_ref);
        }
    }

    if (ret == 0)
    {
        DBG_PRINTF("Doing fuzz test of transport parameter logging into %s\n", log_tp_fuzz_file);

        ret = transport_param_log_fuzz_test(client_param2, sizeof(client_param2));

        if (ret == 0) {
            ret = transport_param_log_fuzz_test(server_param2, sizeof(server_param2));
        }

        DBG_PRINTF("Fuzz test of transport parameter was successful.\n", log_tp_fuzz_file);
    }

    return ret;
}
```

### Rust test body
```rust
fn transport_param_log() {
    transport_param_log_test_one("log_tp_test.txt").expect("log_tp");
    compare_text_files("log_tp_test.txt", "picoquictest/log_tp_test_ref.txt").expect("compare_log");
}
```
