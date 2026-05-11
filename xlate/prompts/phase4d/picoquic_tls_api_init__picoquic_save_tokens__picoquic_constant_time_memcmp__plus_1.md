# Phase 4D deep translation classification

You are classifying Phase 4C non-OK C/Rust function-pair
audit entries.  Phase 4C was intentionally body-only and
shallow; Phase 4D classification is allowed to inspect
broader context.

For each entry:

1. Read the C function and any directly relevant C context:
   types, constants/macros, helper callees, and callers when
   needed to understand observable behavior.
2. Read the Rust function in context, including local types,
   helpers, tests, and nearby translated functions.
3. Decide whether the Phase 4C concern is a false positive.

Do not edit files in this classification pass.  Report:

* `ok` when the Rust behavior is acceptable after deeper
  inspection.
* `needs_fix` when the Rust translation is actually wrong and
  should be repaired in a later 4D repair pass.
* `blocked` only when the analysis cannot be completed without
  a concrete external decision or missing dependency.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short deeper-review conclusion","fix_summary":"empty unless outcome is needs_fix","files_changed":[],"verification":[]}]}
```

Entries:

## `picoquic/tls_api.c:picoquic_tls_api_init`
* Phase 4C status: `suspect`
* Phase 4C rationale: C guards initialization with tls_api_is_init and calls zero/init provider functions; Rust delegates to another function, so the shown body does not visibly perform the same steps.
* C source: `picoquic/tls_api.c:229-236`
* C signature: `void picoquic_tls_api_init(void)`
* Rust source: `rs/fq/src/lib.rs:4779-4781`
* Rust item: `tls_api_init`

### C body
```c
{
    if (!tls_api_is_init) {
        picoquic_tls_api_zero();
        picoquic_tls_api_init_providers(0);
        tls_api_is_init = 1;
    }
}
```

### Rust body
```rust
pub fn tls_api_init() {
    crate::tls_api::tls_api_init();
}
```

## `picoquic/token_store.c:picoquic_save_tokens`
* Phase 4C status: `suspect`
* Phase 4C rationale: C saves only tokens valid beyond current_time and unused; Rust checks time_valid_until.ticks() > 0 instead of comparing to current time.
* C source: `picoquic/token_store.c:246-280`
* C signature: `int picoquic_save_tokens(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/internal.rs:1767-1785`
* Rust item: `save_tokens`

### C body
```c
{
    int ret = 0;
    FILE* F = NULL;
    const picoquic_stored_token_t* first_token = quic->p_first_token;
    const picoquic_stored_token_t* next = first_token;
    uint64_t current_time = picoquic_get_tls_time(quic);

    if ((F = picoquic_file_open(token_file_name, "wb")) == NULL) {
        ret = -1;
    } else {
        while (ret == 0 && next != NULL) {
            /* Only store the tokens that are valid going forward */
            if (next->time_valid_until > current_time && next->was_used == 0) {
                /* Compute the serialized size */
                uint8_t buffer[2048];
                size_t record_size;

                ret = picoquic_serialize_token(next, buffer, sizeof(buffer), &record_size);

                if (ret == 0) {
                    if (fwrite(&record_size, 4, 1, F) != 1 || fwrite(buffer, 1, record_size, F) != record_size) {
                        ret = PICOQUIC_ERROR_INVALID_FILE;
                        break;
                    }
                }
            }
            next = next->next_token;
        }
        (void)picoquic_file_close(F);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let mut file = File::create(token_file_name).map_err(|_| crate::Error::InvalidFile)?;
        for token in &self.stored_tokens {
            if token.time_valid_until.ticks() > 0 && !token.was_used {
                let record = serialize_token(token)?;
                if record.len() > 2048 {
                    return Err(crate::Error::InvalidFile);
                }
                file.write_all(&(record.len() as u32).to_ne_bytes())
                    .map_err(|_| crate::Error::InvalidFile)?;
                file.write_all(&record)
                    .map_err(|_| crate::Error::InvalidFile)?;
            }
        }
        Ok(())
    }
```

## `picoquic/util.c:picoquic_constant_time_memcmp`
* Phase 4C status: `suspect`
* Phase 4C rationale: C iterates exactly l bytes and returns 0 or -1; Rust shown loop only zips slice lengths and shows no return/status conversion.
* C source: `picoquic/util.c:1100-1110`
* C signature: `int picoquic_constant_time_memcmp(const uint8_t *, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/utils.rs:995-999`
* Rust item: `constant_time_memcmp`

### C body
```c
{
    uint64_t ret = 0;

    while (l > 0) {
        ret += (*x++ ^ *y++);
        l--;
    }

    return (ret == 0)?0:-1;
}
```

### Rust body
```rust
    for (&xi, &yi) in x.iter().zip(y.iter()) {
        acc += (xi ^ yi) as u64;
    }
```

## `picoquic/util.c:picoquic_sprintf`
* Phase 4C status: `suspect`
* Phase 4C rationale: C performs vsnprintf formatting and returns 0/truncation/error status while Rust copies an already-built string and returns Ok length or BufferTooSmall.
* C source: `picoquic/util.c:233-251`
* C signature: `int picoquic_sprintf(char *, size_t, size_t *, const char *)`
* Rust source: `rs/fq/src/utils.rs:285-299`
* Rust item: `sprintf`

### C body
```c
{
    va_list args;
    va_start(args, fmt);
#ifdef _WINDOWS
    int res = vsnprintf_s(buf, buf_len, _TRUNCATE, fmt, args);
#else
    int res = vsnprintf(buf, buf_len, fmt, args);
#endif
    va_end(args);

    if (nb_chars != NULL) {
        *nb_chars = res;
    }

    // vsnprintf returns <0 for errors and >=0 for nb of characters required.
    // We return 0 when printing was successful.
    return res >= 0 ? ((size_t)res >= buf_len) : res;
}
```

### Rust body
```rust
pub fn sprintf(buf: &mut [u8], msg: &str) -> Result<usize, Error> {
    let bytes = msg.as_bytes();
    if bytes.len() < buf.len() {
        buf[..bytes.len()].copy_from_slice(bytes);
        buf[bytes.len()] = 0;
        Ok(bytes.len())
    } else if !buf.is_empty() {
        let n = buf.len() - 1;
        buf[..n].copy_from_slice(&bytes[..n]);
        buf[n] = 0;
        Err(Error::BufferTooSmall)
    } else {
        Err(Error::BufferTooSmall)
    }
}
```
