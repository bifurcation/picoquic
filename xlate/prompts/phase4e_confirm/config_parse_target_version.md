# Phase 4E repair confirmation

This is a read-only re-triage after a Phase 4E repair or
repair-level `ok` claim.  Do not edit files.

For each entry, inspect directly relevant C and Rust context
and decide whether the current Rust translation is now
acceptable.

Report:

* `ok` when the current Rust behavior is acceptable.
* `needs_fix` when a real mismatch remains.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short confirmation conclusion","fix_summary":"remaining mismatch if any, or empty","files_changed":[],"verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/config.c:config_parse_target_version`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C parses a hexadecimal string into an integer; Rust body copies bytes into a buffer and returns a UTF-8 string.
* Prior Phase 4D analysis: The quoted Rust body was the adjacent config_optval_string helper, but the actual parse_hex_version exists. However it uses u32::from_str_radix, which rejects overflowing hex strings, while the C uint32_t loop wraps modulo 2^32 and only returns 0 on invalid characters or wrapped zero.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Rust now matches the C uint32_t digit loop, including wrapping overflow and zero on invalid hex characters.
* Phase 4E fix summary: Replaced u32::from_str_radix with byte-by-byte hex parsing using wrapping u32 accumulation; added focused tests for valid digits, invalid input, and overlong wraparound.
* C source: `picoquic/config.c:116-146`
* C signature: `uint32_t config_parse_target_version(const char *)`
* Current Rust source: `rs/fq/src/config.rs:546-557`
* Current Rust item: `parse_hex_version`

### C body
```c
{
    /* Expect the version to be encoded in base 16 */
    uint32_t v = 0;
    char const* x = v_arg;

    while (*x != 0) {
        int c = *x;

        if (c >= '0' && c <= '9') {
            c -= '0';
        }
        else if (c >= 'a' && c <= 'f') {
            c -= 'a';
            c += 10;
        }
        else if (c >= 'A' && c <= 'F') {
            c -= 'A';
            c += 10;
        }
        else {
            v = 0;
            break;
        }
        v *= 16;
        v += c;
        x++;
    }

    return v;
}
```

### Current Rust body
```rust
    for b in s.bytes() {
        let c = match b {
            b'0'..=b'9' => u32::from(b - b'0'),
            b'a'..=b'f' => u32::from(b - b'a') + 10,
            b'A'..=b'F' => u32::from(b - b'A') + 10,
            _ => return 0,
        };
        v = v.wrapping_mul(16).wrapping_add(c);
    }
```
