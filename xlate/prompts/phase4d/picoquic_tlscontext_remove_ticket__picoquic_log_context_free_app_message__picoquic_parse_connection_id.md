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

## `picoquic/tls_api.c:picoquic_tlscontext_remove_ticket`
* Phase 4C status: `suspect`
* Phase 4C rationale: C clears the handshake session_ticket pointer and length; Rust clears resumed_ticket_id, psk_cipher_suite_id, and max_early_data_size instead.
* C source: `picoquic/tls_api.c:2072-2084`
* C signature: `void picoquic_tlscontext_remove_ticket(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1033-1037`
* Rust item: `remove_tls_ticket`

### C body
```c
{
    /* allocate a context structure */
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)(cnx->tls_ctx);

    ctx->handshake_properties.client.session_ticket.base = NULL;
    ctx->handshake_properties.client.session_ticket.len = 0;
}
```

### Rust body
```rust
    pub fn remove_tls_ticket(&mut self) {
        self.resumed_ticket_id = 0;
        self.psk_cipher_suite_id = 0;
        self.max_early_data_size = 0;
    }
```

## `picoquic/unified_log.c:picoquic_log_context_free_app_message`
* Phase 4C status: `suspect`
* Phase 4C rationale: C gates logging on quic->F_log but calls through text_log_fns; Rust gates on text_log_fns directly. Both visibly dispatch only the text app-message logger.
* C source: `picoquic/unified_log.c:100-108`
* C signature: `void picoquic_log_context_free_app_message(picoquic_quic_t *, const picoquic_connection_id_t *, const char *)`
* Rust source: `rs/fq/src/logger.rs:253-256`
* Rust item: `log_app_message`

### C body
```c
{
    if (quic->F_log != NULL) {
        va_list args;
        va_start(args, fmt);
        quic->text_log_fns->log_quic_app_message(quic, cid, fmt, args);
        va_end(args);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().quic_app_message(self, cid, args);
        }
```

## `picoquic/util.c:picoquic_parse_connection_id`
* Phase 4C status: `suspect`
* Phase 4C rationale: On oversized input, C clears id_len and returns 0, while Rust returns an error.
* C source: `picoquic/util.c:333-343`
* C signature: `uint8_t picoquic_parse_connection_id(const uint8_t *, uint8_t, picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/utils.rs:338-349`
* Rust item: `parse_connection_id`

### C body
```c
{
    if (len <= PICOQUIC_CONNECTION_ID_MAX_SIZE) {
        cnx_id->id_len = len;
        memcpy(cnx_id->id, bytes, len);
    } else {
        len = 0;
        cnx_id->id_len = 0;
    }
    return len;
}
```

### Rust body
```rust
pub fn parse_connection_id(bytes: &[u8]) -> Result<ConnectionId, Error> {
    let len = bytes.len();
    if len > CONNECTION_ID_MAX_SIZE {
        return Err(Error::InvalidArgument);
    }
    let mut id = [0u8; CONNECTION_ID_MAX_SIZE];
    id[..len].copy_from_slice(bytes);
    Ok(ConnectionId {
        id,
        id_len: len as u8,
    })
}
```
