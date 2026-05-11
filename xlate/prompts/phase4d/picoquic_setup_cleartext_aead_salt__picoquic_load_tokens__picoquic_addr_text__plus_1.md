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

## `picoquic/tls_api.c:picoquic_setup_cleartext_aead_salt`
* Phase 4C status: `suspect`
* Phase 4C rationale: C always assigns either a version-specific salt or picoquic_cleartext_null_salt fallback; Rust only returns the version key when present and shows no visible fallback assignment or return.
* C source: `picoquic/tls_api.c:2532-2541`
* C signature: `void picoquic_setup_cleartext_aead_salt(size_t, ptls_iovec_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1385-1392`
* Rust item: `setup_cleartext_aead_salt`

### C body
```c
{
    if (picoquic_supported_versions[version_index].version_aead_key != NULL && picoquic_supported_versions[version_index].version_aead_key_length > 0) {
        salt->base = picoquic_supported_versions[version_index].version_aead_key;
        salt->len = picoquic_supported_versions[version_index].version_aead_key_length;
    } else {
        salt->base = picoquic_cleartext_null_salt;
        salt->len = sizeof(picoquic_cleartext_null_salt);
    }
}
```

### Rust body
```rust
    if let Some(version) = version_from_index(version_index) {
        let params = version.parameters();
        if !params.version_aead_key.is_empty() {
            return params.version_aead_key;
        }
    }
```

## `picoquic/token_store.c:picoquic_load_tokens`
* Phase 4C status: `suspect`
* Phase 4C rationale: C skips tokens whose time_valid_until is less than current_time; Rust pushes tokens when time_valid_until.ticks() > 0, with no visible comparison to current time.
* C source: `picoquic/token_store.c:282-348`
* C signature: `int picoquic_load_tokens(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/internal.rs:1788-1821`
* Rust item: `load_tokens`

### C body
```c
{
    int ret = 0;
    int file_ret = 0;
    FILE* F = NULL;
    picoquic_stored_token_t* previous = NULL;
    picoquic_stored_token_t* next = NULL;
    uint32_t record_size;
    uint32_t storage_size;
    uint64_t current_time = picoquic_get_tls_time(quic);
    picoquic_stored_token_t** pp_first_token = &quic->p_first_token;

    if ((F = picoquic_file_open_ex(token_file_name, "rb", &file_ret)) == NULL) {
        ret = (file_ret == ENOENT) ? PICOQUIC_ERROR_NO_SUCH_FILE : -1;
    }

    while (ret == 0) {
        if (fread(&storage_size, 4, 1, F) != 1) {
            /* end of file */
            break;
        }
        else if (storage_size > 2048 ||
            (record_size = storage_size + offsetof(struct st_picoquic_stored_token_t, time_valid_until)) > 2048) {
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        }
        else {
            uint8_t buffer[2048];
            if (fread(buffer, 1, storage_size, F) != storage_size) {
                ret = PICOQUIC_ERROR_INVALID_FILE;
            }
            else {
                size_t consumed = 0;
                ret = picoquic_deserialize_token(&next, buffer, storage_size, &consumed);

                if (ret == 0 && (consumed != storage_size || next == NULL)) {
                    ret = PICOQUIC_ERROR_INVALID_FILE;
                }

                if (ret == 0 && next != NULL) {
                    if (next->time_valid_until < current_time) {
                        free(next);
                        next = NULL;
                    }
                    else {
                        next->sni = ((char*)next) + sizeof(picoquic_stored_token_t);
                        next->ip_addr = ((uint8_t*)next->sni) + next->sni_length + 1;
                        next->token = (uint8_t*)(next->ip_addr + next->ip_addr_length + 1);
                        next->next_token = NULL;
                        if (previous == NULL) {
                            *pp_first_token = next;
                        }
                        else {
                            previous->next_token = next;
                        }

                        previous = next;
                    }
                }
            }
        }
    }

    (void)picoquic_file_close(F);

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let data = match std::fs::read(token_file_name) {
            Ok(data) => data,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(crate::Error::NoSuchFile);
            }
            Err(_) => return Err(crate::Error::InvalidFile),
        };

        let mut off = 0;
        self.stored_tokens.clear();
        while off < data.len() {
            let len_bytes = data.get(off..off + 4).ok_or(crate::Error::InvalidFile)?;
            let record_len =
                u32::from_ne_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]])
                    as usize;
            off += 4;
            if record_len > 2048 {
                return Err(crate::Error::InvalidFile);
            }
            let record = data
                .get(off..off + record_len)
                .ok_or(crate::Error::InvalidFile)?;
            off += record_len;
            let token = deserialize_token(record)?;
            if token.time_valid_until.ticks() > 0 {
                self.stored_tokens.push(token);
            }
        }
        Ok(())
    }
```

## `picoquic/util.c:picoquic_addr_text`
* Phase 4C status: `suspect`
* Phase 4C rationale: both format IPv4 as host:port and IPv6 as [host]:port, but C has null/unknown fallback behavior and writes into a provided buffer while Rust requires an address and writer.
* C source: `picoquic/util.c:608-638`
* C signature: `const char * picoquic_addr_text(const struct sockaddr *, char *, size_t)`
* Rust source: `rs/fq/src/utils.rs:565-570`
* Rust item: `addr_text`

### C body
```c
{
    char addr_buffer[128];
    char const* addr_text;
    char const* ret_text = "?:?";

    if (addr != NULL) {
        switch (addr->sa_family) {
        case AF_INET:
            addr_text = inet_ntop(AF_INET,
                (const void*)(&((struct sockaddr_in*)addr)->sin_addr),
                addr_buffer, sizeof(addr_buffer));
            if (picoquic_sprintf(text, text_size, NULL, "%s:%d", addr_text, ((struct sockaddr_in*)addr)->sin_port) == 0) {
                ret_text = text;
            }
            break;
        case AF_INET6:
            addr_text = inet_ntop(AF_INET6,
                (const void*)(&((struct sockaddr_in6*)addr)->sin6_addr),
                addr_buffer, sizeof(addr_buffer));
            if (picoquic_sprintf(text, text_size, NULL, "[%s]:%d", addr_text, ((struct sockaddr_in6*)addr)->sin6_port) == 0) {
                ret_text = text;
            }
        default:
            break;
        }
    }

    return ret_text;
}
```

### Rust body
```rust
pub fn addr_text(addr: &SocketAddr, w: &mut dyn core::fmt::Write) -> Result<(), core::fmt::Error> {
    match addr {
        SocketAddr::V4(a) => write!(w, "{}:{}", a.ip(), a.port()),
        SocketAddr::V6(a) => write!(w, "[{}]:{}", a.ip(), a.port()),
    }
}
```

## `picoquic/util.c:picoquic_set_solution_dir`
* Phase 4C status: `suspect`
* Phase 4C rationale: C stores the provided pointer directly, while Rust clones/leaks the string into a static location under a lock.
* C source: `picoquic/util.c:703-706`
* C signature: `void picoquic_set_solution_dir(const char *)`
* Rust source: `rs/fq/src/utils.rs:636-639`
* Rust item: `set_solution_dir`

### C body
```c
{
    picoquic_solution_dir = solution_dir;
}
```

### Rust body
```rust
pub fn set_solution_dir(solution_dir: Option<&str>) {
    let mut lock = SOLUTION_DIR.lock().unwrap();
    *lock = solution_dir.map(|s| -> &'static str { Box::leak(s.to_owned().into_boxed_str()) });
}
```
