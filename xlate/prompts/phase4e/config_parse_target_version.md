# Phase 4E repair confirmed translation mismatches

You are repairing Phase 4D `needs_fix` entries.  Phase 4D
already performed deeper classification and concluded that
these Rust translations need repair.

Rules:

* Edit Rust only.  Do not edit C sources.
* Keep edits limited to the owned Rust file(s) for this batch
  unless a directly related helper in `rs/fq/` must change.
* Preserve safe, idiomatic Rust and existing public API shape
  unless the current shape cannot express the C behavior.
* Do not replace code with stubs, placeholders, fabricated
  defaults, or weaker behavior.
* If deeper repair inspection proves Phase 4D was mistaken,
  report outcome `ok` and do not edit source.
* The driver will run a separate read-only re-triage before
  recording any `fixed` or `ok` result as resolved.
* Report `blocked` only with a concrete human-actionable
  reason.

Owned Rust file(s): `rs/fq/src/config.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/config.c:config_parse_target_version`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C parses a hexadecimal string into an integer; Rust body copies bytes into a buffer and returns a UTF-8 string.
* Phase 4D analysis: The quoted Rust body was the adjacent config_optval_string helper, but the actual parse_hex_version exists. However it uses u32::from_str_radix, which rejects overflowing hex strings, while the C uint32_t loop wraps modulo 2^32 and only returns 0 on invalid characters or wrapped zero.
* Phase 4D fix note: Replace from_str_radix with digit-by-digit u32 wrapping accumulation so valid overlong hex strings match C modulo behavior while invalid characters still return 0.
* C source: `picoquic/config.c:116-146`
* C signature: `uint32_t config_parse_target_version(const char *)`
* Rust source: `rs/fq/src/config.rs:544-560`
* Rust item: `parse_hex_version`

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

### Rust body
```rust
fn config_optval_string<'a>(buffer: &'a mut [u8], p: &[u8]) -> &'a str {
    let len = p.len().min(buffer.len().saturating_sub(1));
    buffer[..len].copy_from_slice(&p[..len]);
    if !buffer.is_empty() {
        buffer[len] = 0;
    }
    core::str::from_utf8(&buffer[..len]).unwrap_or("")
}
```
