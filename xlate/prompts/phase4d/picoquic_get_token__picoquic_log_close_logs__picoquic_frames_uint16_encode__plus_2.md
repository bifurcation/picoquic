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

## `picoquic/token_store.c:picoquic_get_token`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C checks token expiration against current time, supports SNI-only best match when ip length is zero, copies the token, and marks it used; Rust body only finds a position using fixed predicates and no copy or mark.
* C source: `picoquic/token_store.c:204-244`
* C signature: `int picoquic_get_token(picoquic_quic_t *, const char *, uint16_t, const uint8_t *, uint8_t, uint8_t **, uint16_t *, int)`
* Rust source: `rs/fq/src/internal.rs:1743-1758`
* Rust item: `get_token`

### C body
```c
{
    int ret = 0;

    uint64_t current_time = picoquic_get_tls_time(quic);
    picoquic_stored_token_t* p_first_token = quic->p_first_token;
    picoquic_stored_token_t* next = p_first_token;
    picoquic_stored_token_t* best_match = NULL;

    while (next != NULL) {
        if (next->time_valid_until > current_time && next->sni_length == sni_length && memcmp(next->sni, sni, sni_length) == 0 && next->was_used == 0){
            if (ip_addr_length > 0) {
                if (next->ip_addr_length == ip_addr_length && memcmp(next->ip_addr, ip_addr, ip_addr_length) == 0) {
                    best_match = next;
                    break;
                }
            }
            else {
                if (best_match == NULL || next->time_valid_until > best_match->time_valid_until) {
                    best_match = next;
                }
            }
        } 
        next = next->next_token;
    }

    if (best_match == NULL || best_match->token_length == 0 || (*token = (uint8_t *)malloc(best_match->token_length)) == NULL) {
        *token = NULL;
        *token_length = 0;
        ret = -1;
    } else {
        *token_length = best_match->token_length;
        memcpy(*token, (uint8_t*)best_match->token, best_match->token_length);
        best_match->was_used = mark_used;
    }

    return ret;
}
```

### Rust body
```rust
            .position(|token| {
                token.time_valid_until.ticks() > 0
                    && token.sni.as_deref() == sni
                    && token.ip_addr == ip_addr
                    && !token.was_used
                    && !token.token.is_empty()
            })
```

## `picoquic/unified_log.c:picoquic_log_close_logs`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C calls text, binary, and qlog close callbacks when present; Rust only calls the text logger close callback.
* C source: `picoquic/unified_log.c:32-46`
* C signature: `void picoquic_log_close_logs(picoquic_quic_t *)`
* Rust source: `rs/fq/src/logger.rs:292-295`
* Rust item: `close_logs`

### C body
```c
{
    if (quic->text_log_fns != NULL) {
        quic->text_log_fns->log_quic_close(quic);
    }

    if (quic->bin_log_fns != NULL) {
        quic->bin_log_fns->log_quic_close(quic);
    }

    if (quic->qlog_fns != NULL) {
        quic->qlog_fns->log_quic_close(quic);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().quic_close(self);
        }
```

## `picoquic/util.c:picoquic_frames_uint16_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C writes two bytes and returns the advanced pointer, while the shown Rust body only checks length and performs no encoding.
* C source: `picoquic/util.c:1002-1012`
* C signature: `uint8_t * picoquic_frames_uint16_encode(uint8_t *, const uint8_t *, uint16_t)`
* Rust source: `rs/fq/src/utils.rs:868-871`
* Rust item: `frames_uint16_encode`

### C body
```c
{
    if (bytes + sizeof(n) > bytes_max) {
        bytes = NULL;
    }
    else {
        *bytes++ = (uint8_t)(n >> 8);
        *bytes++ = (uint8_t)n;
    }
    return (bytes);
}
```

### Rust body
```rust
    if bytes.len() < 2 {
        return None;
    }
```

## `picoquic/util.c:picoquic_get_addr_port`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C gets and returns a port, while the Rust body shown is set_addr_port and mutates the port instead.
* C source: `picoquic/util.c:510-515`
* C signature: `uint16_t picoquic_get_addr_port(const struct sockaddr *)`
* Rust source: `rs/fq/src/utils.rs:501-511`
* Rust item: `get_addr_port`

### C body
```c
{
    uint16_t port = (addr->sa_family == AF_INET6) ? ((struct sockaddr_in6*)addr)->sin6_port : ((struct sockaddr_in*)addr)->sin_port;

    return port;
}
```

### Rust body
```rust
pub fn set_addr_port(addr: &mut SocketAddr, port: u16) {
    addr.set_port(port);
}
```

## `picoquic/bbr.c:BBRCheckDrain`
* Phase 4C status: `suspect`
* Phase 4C rationale: C calls BBRInflight(bbr_state, path_x, 1.0) directly; Rust computes via inflight_with_bw(path_x, 1.0, self.bw). This may be equivalent, but the body shows an extra explicit bw path.
* C source: `picoquic/bbr.c:1943-1948`
* C signature: `void BBRCheckDrain(picoquic_bbr_state_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:1602-1610`
* Rust item: `check_drain`

### C body
```c
{
    if (bbr_state->state == picoquic_bbr_alg_drain && path_x->bytes_in_transit <= BBRInflight(bbr_state, path_x, 1.0)) {
        BBREnterProbeBW(bbr_state, path_x, current_time);  /* we estimate that the queue is drained */
    }
}
```

### Rust body
```rust
    fn check_drain(&mut self, connection: &Connection, path_x: &mut Path, current_time: u64) {
        if self.state == BbrAlgState::Drain {
            let bw = self.bw;
            let target = self.inflight_with_bw(path_x, 1.0, bw);
            if path_x.bytes_in_transit <= target {
                self.enter_probe_bw(connection, path_x, current_time);
            }
        }
    }
```
