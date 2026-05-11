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

## `picoquic/tls_api.c:picoquic_set_key_log_file`
* Phase 4C status: `suspect`
* Phase 4C rationale: C allocates or reuses a TLS log_event object and installs it on ctx; Rust only opens or clears self.f_log, with no visible log_event callback/context update.
* C source: `picoquic/tls_api.c:2040-2070`
* C signature: `void picoquic_set_key_log_file(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/lib.rs:1129-1144`
* Rust item: `set_key_log_file`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;
    struct st_picoquic_log_event_t* log_event = (struct st_picoquic_log_event_t*)ctx->log_event;

    if (log_event == NULL) {
        log_event = (struct st_picoquic_log_event_t*)malloc(sizeof(struct st_picoquic_log_event_t));
        if (log_event != NULL) {
            log_event->super.cb = picoquic_log_event_call_back;
        }
    }
    else {
        if (log_event->fp != NULL) {
            picoquic_file_close(log_event->fp);
            log_event->fp = NULL;
        }
    }

    if (log_event != NULL) {
        log_event->fp = picoquic_file_open(keylog_filename, "a");
        log_event->super.cb = picoquic_log_event_call_back;
        ctx->log_event = (ptls_log_event_t*)log_event;
    }

    ctx->log_event = (ptls_log_event_t*)log_event;
}
```

### Rust body
```rust
    pub fn set_key_log_file(&mut self, keylog_filename: Option<&str>) {
        // Open or clear the SSL keylog file.  When a path is given, open it
        // for appending; errors are silently ignored (matching the C behaviour
        // of falling back to no logging rather than crashing).
        use std::fs::OpenOptions;
        match keylog_filename {
            Some(path) => {
                if let Ok(f) = OpenOptions::new().create(true).append(true).open(path) {
                    self.f_log = Some(Box::new(f));
                }
            }
            None => {
                self.f_log = None;
            }
        }
    }
```

## `picoquic/token_store.c:picoquic_deserialize_token`
* Phase 4C status: `suspect`
* Phase 4C rationale: C permits consumed to be less than bytes_max and reports consumed; Rust requires off == bytes.len() inside deserialization. The field parsing order is otherwise visibly similar.
* C source: `picoquic/token_store.c:98-155`
* C signature: `int picoquic_deserialize_token(picoquic_stored_token_t **, uint8_t *, size_t, size_t *)`
* Rust source: `rs/fq/src/internal.rs:1686-1709`
* Rust item: `deserialize_token`

### C body
```c
{
    int ret = 0;
    uint64_t time_valid_until = 0;
    size_t required_length = 8 + 2 + 2 + 2;
    size_t byte_index = 0;
    size_t sni_index = 0;
    size_t ip_addr_index = 0;
    size_t token_index = 0;
    uint16_t sni_length = 0;
    uint8_t ip_addr_length = 0;
    uint16_t token_length = 0;


    *consumed = 0;
    *token = NULL;

    if (required_length < bytes_max) {
        time_valid_until = PICOPARSE_64(bytes);
        byte_index = 8;
        sni_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        sni_index = byte_index;
        required_length += sni_length;
        byte_index += sni_length;
    }
    
    if (required_length < bytes_max) {
        ip_addr_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        ip_addr_index = byte_index;
        required_length += ip_addr_length;
        byte_index += ip_addr_length;
    }

    if (required_length < bytes_max) {
        token_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        token_index = byte_index;
        required_length += token_length;
    }

    if (required_length > bytes_max) {
        *token = NULL;
        ret = PICOQUIC_ERROR_INVALID_TOKEN;
    } else {
        *token = picoquic_format_token(time_valid_until, (const char *)(bytes + sni_index), sni_length,
            bytes + ip_addr_index, ip_addr_length, bytes + token_index, token_length);
        if (*token == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
        else {
            *consumed = required_length;
        }
    }

    return ret;
}
```

### Rust body
```rust
fn deserialize_token(bytes: &[u8]) -> Result<StoredToken, crate::Error> {
    let mut off = 0;
    let time_valid_until = Instant::from_ticks(parse_64(take_slice(bytes, &mut off, 8)?));

    let sni_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let sni = optional_string_from_bytes(take_slice(bytes, &mut off, sni_len)?)?;

    let ip_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let ip_addr = stored_ip_from_bytes(take_slice(bytes, &mut off, ip_len)?)?;

    let token_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let token = take_slice(bytes, &mut off, token_len)?.to_vec();
    if off != bytes.len() {
        return Err(crate::Error::InvalidFile);
    }

    Ok(StoredToken {
        sni,
        token,
        ip_addr,
        time_valid_until,
        was_used: false,
    })
}
```

## `picoquic/util.c:debug_printf_reset`
* Phase 4C status: `suspect`
* Phase 4C rationale: C reads and writes a plain shared debug_suspended value, while Rust uses DEBUG_SUSPENDED.with, which is visibly thread-local-style storage.
* C source: `picoquic/util.c:226-231`
* C signature: `int debug_printf_reset(int)`
* Rust source: `rs/fq/src/utils.rs:205-211`
* Rust item: `debug_printf_reset`

### C body
```c
{
    int ret = debug_suspended;
    debug_suspended = suspended;
    return ret;
}
```

### Rust body
```rust
pub fn debug_printf_reset(suspended: bool) -> bool {
    DEBUG_SUSPENDED.with(|s| {
        let old = s.get();
        s.set(suspended);
        old
    })
}
```

## `picoquic/util.c:picoquic_print_connection_id_hexa`
* Phase 4C status: `suspect`
* Phase 4C rationale: both emit lowercase hex bytes, but C performs an explicit buffer length check and writes a terminating NUL while Rust writes to a formatter with no visible terminator.
* C source: `picoquic/util.c:253-268`
* C signature: `int picoquic_print_connection_id_hexa(char *, size_t, const picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/utils.rs:464-472`
* Rust item: `print_connection_id_hexa`

### C body
```c
{
    static const char hex_to_char[16] = { '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f' };
    if (buf_len < ((size_t)cnxid->id_len) * 2u + 1u) {
        return -1;  
    }

    for (unsigned i = 0; i < cnxid->id_len; i++) {
        buf[i * 2u] = hex_to_char[cnxid->id[i] >> 4];
        buf[i * 2u + 1u] = hex_to_char[cnxid->id[i] & 0x0f];
    }

    buf[cnxid->id_len * 2u] = 0;

    return 0;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    for b in connection_id.as_bytes() {
        write!(w, "{:02x}", b).map_err(|_| Error::Generic)?;
    }
    Ok(())
}
```
