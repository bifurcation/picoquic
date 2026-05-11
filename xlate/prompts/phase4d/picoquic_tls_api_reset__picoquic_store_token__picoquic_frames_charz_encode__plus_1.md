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

## `picoquic/tls_api.c:picoquic_tls_api_reset`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust calls zero unconditionally, while C zeroes only inside the is_init branch.
* C source: `picoquic/tls_api.c:247-257`
* C signature: `void picoquic_tls_api_reset(uint64_t)`
* Rust source: `rs/fq/src/tls_api.rs:2502-2512`
* Rust item: `tls_api_reset`

### C body
```c
{
    if (tls_api_is_init) {
        tls_api_is_init = 0;
        picoquic_tls_api_init_providers(2);
        picoquic_tls_api_zero();
    }
    tls_api_init_flags = init_flags;
    picoquic_tls_api_init_providers(0);
    tls_api_is_init = 1;
}
```

### Rust body
```rust
pub fn tls_api_reset(init_flags: u64) {
    let mut state = tls_api_state();
    if state.is_init {
        state.is_init = false;
        tls_api_init_providers_locked(&mut state, 2);
    }
    state.zero();
    state.init_flags = init_flags;
    tls_api_init_providers_locked(&mut state, 0);
    state.is_init = true;
}
```

## `picoquic/token_store.c:picoquic_store_token`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets validity to current_time plus 24 hours and removes older/equal matching tokens; Rust sets time_valid_until to TOKEN_DELAY_LONG ticks without adding current time and removes all matching tokens.
* C source: `picoquic/token_store.c:157-202`
* C signature: `int picoquic_store_token(picoquic_quic_t *, const char *, uint16_t, const uint8_t *, uint8_t, const uint8_t *, uint16_t)`
* Rust source: `rs/fq/src/internal.rs:1714-1737`
* Rust item: `store_token`

### C body
```c
{
    int ret = 0;
    picoquic_stored_token_t** pp_first_token = &quic->p_first_token;
    uint64_t current_time = picoquic_get_tls_time(quic);

    if (token_length < 1 || sni == NULL || sni_length == 0) {
        ret = PICOQUIC_ERROR_INVALID_TOKEN;
    }
    else {
        /* There is no explicit TTL for tokens. We assume they are OK for 24 hours */
        uint64_t time_valid_until = current_time + ((uint64_t)24 * 3600) * ((uint64_t)1000000);
        picoquic_stored_token_t* stored = picoquic_format_token(time_valid_until, sni, sni_length,
            ip_addr, ip_addr_length, token, token_length);
        if (stored == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
        else {
            picoquic_stored_token_t* next;
            picoquic_stored_token_t** pprevious;

            stored->next_token = next = *pp_first_token;
            *pp_first_token = stored;
            pprevious = &stored->next_token;

            /* Now remove the old tokens for that SNI & ip_addr */
            while (next != NULL) {
                if (next->time_valid_until <= stored->time_valid_until && next->sni_length == sni_length && next->ip_addr_length == ip_addr_length && memcmp(next->sni, sni, sni_length) == 0 && memcmp(next->ip_addr, ip_addr, ip_addr_length) == 0) {
                    picoquic_stored_token_t* deleted = next;
                    next = next->next_token;
                    *pprevious = next;
                    free(deleted);
                }
                else {
                    pprevious = &next->next_token;
                    next = next->next_token;
                }
            }
        }
    } 

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        if sni.unwrap_or("").is_empty() || token.is_empty() {
            return Err(crate::Error::Protocol(
                crate::errors::InternalError::InvalidToken as u64,
            ));
        }

        // Replace any existing token for the same (sni, ip_addr).
        self.stored_tokens
            .retain(|t| t.sni.as_deref() != sni || t.ip_addr != ip_addr);
        self.stored_tokens.push(StoredToken {
            sni: sni.map(str::to_owned),
            token: token.to_vec(),
            ip_addr,
            time_valid_until: crate::Instant::from_ticks(TOKEN_DELAY_LONG.ticks()),
            was_used: false,
        });
        Ok(())
    }
```

## `picoquic/util.c:picoquic_frames_charz_encode`
* Phase 4C status: `suspect`
* Phase 4C rationale: C handles NULL by encoding length 0; Rust takes &str and always encodes the string bytes, so the NULL case has no visible equivalent.
* C source: `picoquic/util.c:1079-1089`
* C signature: `uint8_t * picoquic_frames_charz_encode(uint8_t *, const uint8_t *, const char *)`
* Rust source: `rs/fq/src/utils.rs:939-941`
* Rust item: `frames_charz_encode`

### C body
```c
{
    if (s == NULL) {
        bytes = picoquic_frames_varlen_encode(bytes, bytes_max, 0);
    }
    else {
        size_t l = strlen(s);
        bytes = picoquic_frames_length_data_encode(bytes, bytes_max, l, (const uint8_t*)s);
    }
    return bytes;
}
```

### Rust body
```rust
pub fn frames_charz_encode<'a>(bytes: &'a mut [u8], s: &str) -> Option<&'a mut [u8]> {
    frames_length_data_encode(bytes, s.as_bytes())
}
```

## `picoquic/util.c:picoquic_store_loopback_addr`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns -1 for an unsupported family, while Rust returns Error::InvalidArgument; the main IPv4/IPv6 loopback selection matches.
* C source: `picoquic/util.c:640-651`
* C signature: `int picoquic_store_loopback_addr(struct sockaddr_storage *, int, uint16_t)`
* Rust source: `rs/fq/src/utils.rs:580-595`
* Rust item: `store_loopback_addr`

### C body
```c
{
    int ret = -1;
    if (addr_family == AF_INET) {
        ret = picoquic_store_text_addr(stored_addr, "127.0.0.1", port);
    }
    else if (addr_family == AF_INET6) {
        ret = picoquic_store_text_addr(stored_addr, "::1", port);
    }
    return ret;
}
```

### Rust body
```rust
pub fn store_loopback_addr(addr_family: i32, port: u16) -> Result<SocketAddr, Error> {
    // AF_INET=2 is universal; AF_INET6=10 on Linux, 30 on macOS.
    const AF_INET: i32 = 2;
    #[cfg(target_os = "macos")]
    const AF_INET6: i32 = 30;
    #[cfg(not(target_os = "macos"))]
    const AF_INET6: i32 = 10;

    if addr_family == AF_INET {
        store_text_addr("127.0.0.1", port)
    } else if addr_family == AF_INET6 {
        store_text_addr("::1", port)
    } else {
        Err(Error::InvalidArgument)
    }
}
```
