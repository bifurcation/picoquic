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
The `test_id` field MUST be one of the exact canonical
test_id values shown below. Do not substitute C function
names, Rust test names, or table names.

```json
{"reviews":[{"test_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short conclusion","fix_summary":"what 5B should change, or empty","verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/sim_link.c:sim_link_test`
* Canonical test_id: `picoquic/sim_link.c:sim_link_test`
* C test-table name: `sim_link`
* C entry function: `sim_link_test`
* Rust test: `sim_link`
* C source: `picoquic/sim_link.c:431-449`
* Rust source: `rs/fq/src/tests/harness.rs:148-152`

### C test body
```c
{
    int ret = 0;
    uint64_t loss_mask = 0;
    
    ret = sim_link_one_test(&loss_mask, 0, 0);

    if (ret == 0) {
        loss_mask = 8;
        ret = sim_link_one_test(&loss_mask, 0, 1);
    }

    if (ret == 0) {
        loss_mask = 0x18;
        ret = sim_link_one_test(&loss_mask, 0, 2);
    }

    return ret;
}
```

### Rust test body
```rust
fn sim_link() {
    sim_link_one_test(Some(0), 0, 0);
    sim_link_one_test(Some(8), 0, 1);
    sim_link_one_test(Some(0x18), 0, 2);
}
```
