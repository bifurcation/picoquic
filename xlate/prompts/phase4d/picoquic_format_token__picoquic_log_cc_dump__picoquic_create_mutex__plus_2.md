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

## `picoquic/token_store.c:picoquic_format_token`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C allocates and formats a stored token; Rust body adds a proposed ALPN string.
* C source: `picoquic/token_store.c:29-59`
* C signature: `picoquic_stored_token_t * picoquic_format_token(uint64_t, const char *, uint16_t, const uint8_t *, uint8_t, const uint8_t *, uint16_t)`
* Rust source: `rs/fq/src/tls_api.rs:2747-2778`
* Rust item: `format_token`

### C body
```c
{
    size_t token_size = sizeof(picoquic_stored_token_t) + sni_length + 1 + ip_addr_length + 1 + token_length;
    picoquic_stored_token_t* stored = (picoquic_stored_token_t*)malloc(token_size);
    
    if (stored != NULL) {
        uint8_t* next_p = ((uint8_t*)stored) + sizeof(picoquic_stored_token_t);

        memset(stored, 0, token_size);
        stored->time_valid_until = time_valid_until;
        stored->sni = (char const *)next_p;
        stored->sni_length = sni_length;
        memcpy(next_p, sni, sni_length);
        next_p += sni_length;
        *next_p++ = 0;

        stored->ip_addr = next_p;
        stored->ip_addr_length = ip_addr_length;
        memcpy(next_p, ip_addr, ip_addr_length);
        next_p += ip_addr_length;
        *next_p++ = 0;

        stored->token = next_p;
        stored->token_length = token_length;
        memcpy(next_p, token, token_length);
    }

    return stored;
}
```

### Rust body
```rust
impl Connection {
    /// Push a proposed ALPN string onto the connection's handshake ALPN list.
    /// C: `picoquic_add_proposed_alpn` (tls_api.c:2207).
    ///
    /// In C: pushed into `tls_ctx->alpn_vec[alpn_count]` with a bounds check
    /// against `alpn_vec_size` (max [`crate::internal::ALPN_NUMBER_MAX`]).
    /// In Rust: stored in `Connection::alpn_proposals`; the TLS backend reads
    /// this slice when starting the handshake.  For server-side ALPN selection
    /// use [`crate::Quic::set_alpn_select_fn`] or `Quic::default_alpn`.
    pub fn add_proposed_alpn(&mut self, alpn: &str) -> Result<(), Error> {
        if self.alpn_proposals.len() >= crate::internal::ALPN_NUMBER_MAX {
            return Err(Error::BufferTooSmall);
        }
        self.alpn_proposals.push(alpn.to_owned());
        Ok(())
    }
}
```

## `picoquic/unified_log.c:picoquic_log_cc_dump`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body invokes a memlog callback, iterates updated paths, dispatches cc_dump to loggers, and clears update flags; Rust snippet is app_message, not cc_dump.
* C source: `picoquic/unified_log.c:315-345`
* C signature: `void picoquic_log_cc_dump(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/logger.rs:434-454`
* Rust item: `cc_dump`

### C body
```c
{
    if (cnx->memlog_call_back != NULL) {
        cnx->memlog_call_back(cnx, cnx->path[0], cnx->memlog_ctx, 0, current_time);
    }

    if (picoquic_cnx_is_still_logging(cnx)) {
        picoquic_path_t* path_x;
        for (int path_index = 0; path_index < cnx->nb_paths; path_index++)
        {
            path_x = cnx->path[path_index];

            if (!path_x->is_cc_data_updated) {
                continue;
            }

            if (cnx->quic->F_log != NULL) {
                cnx->quic->text_log_fns->log_cc_dump(cnx, path_x, current_time);
            }
            if (cnx->f_binlog != NULL) {
                cnx->quic->bin_log_fns->log_cc_dump(cnx, path_x, current_time);
            }
            if (cnx->qlog_ctx != NULL) {
                cnx->quic->qlog_fns->log_cc_dump(cnx, path_x, current_time);
            }

            path_x->is_cc_data_updated = 0;
        }
    }
}
```

### Rust body
```rust
    fn app_message(&mut self, args: core::fmt::Arguments<'_>) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().app_message(self, args);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().app_message(self, args);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().app_message(self, args);
        }
    }
```

## `picoquic/util.c:picoquic_create_mutex`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C creates/initializes a mutex, while the Rust body shown only defines a test data struct.
* C source: `picoquic/util.c:1176-1188`
* C signature: `int picoquic_create_mutex(pthread_mutex_t *)`
* Rust source: `rs/fq/src/tests/util_test.rs:225-232`
* Rust item: `threading`

### C body
```c
{
#ifdef _WINDOWS
    int ret = 0;
    *mutex = CreateMutex(NULL, FALSE, NULL);
    if (*mutex == NULL) {
        ret = -1;
    }
#else
    int ret = pthread_mutex_init(mutex, NULL);
#endif
    return ret;
}
```

### Rust body
```rust
    struct ThreadTestData {
        data: u64,
    }
```

## `picoquic/util.c:picoquic_frames_varlen_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C delegates to varint encoding of n; Rust body shown is frames_uint8_encode and writes a single u8.
* C source: `picoquic/util.c:985-988`
* C signature: `uint8_t * picoquic_frames_varlen_encode(uint8_t *, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/utils.rs:854-865`
* Rust item: `frames_varlen_encode`

### C body
```c
{
    return picoquic_frames_varint_encode(bytes, bytes_max, n);
}
```

### Rust body
```rust
pub fn frames_uint8_encode(bytes: &mut [u8], n: u8) -> Option<&mut [u8]> {
    if bytes.is_empty() {
        return None;
    }
    bytes[0] = n;
    Some(&mut bytes[1..])
}
```

## `picoquic/bbr.c:BBRAdaptLowerBoundsFromCongestion`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body shown is only a function header with no visible implementation, while Rust body has an early return when probing bandwidth; comparison is incomplete at body level.
* C source: `picoquic/bbr.c:1051-1051`
* C signature: `void BBRAdaptLowerBoundsFromCongestion(picoquic_bbr_state_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:960-963`
* Rust item: `adapt_lower_bounds_from_congestion`

### C body
```c
static void BBRAdaptLowerBoundsFromCongestion(picoquic_bbr_state_t* bbr_state, picoquic_path_t* path_x)
```

### Rust body
```rust
        if self.is_probing_bw() {
            return;
        }
```
