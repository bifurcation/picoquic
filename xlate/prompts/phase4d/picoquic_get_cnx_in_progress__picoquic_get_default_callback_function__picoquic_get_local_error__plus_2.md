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

## `picoquic/quicctx.c:picoquic_get_cnx_in_progress`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body sets default PMTUD policy and returns nothing visible, while C returns quic->cnx_in_progress.
* C source: `picoquic/quicctx.c:4507-4511`
* C signature: `picoquic_cnx_t * picoquic_get_cnx_in_progress(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:3232-3240`
* Rust item: `connection_in_progress`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->cnx_in_progress;
}
```

### Rust body
```rust
    pub fn set_default_pmtud_policy(&mut self, pmtud_policy: PmtudPolicy) {
        self.default_pmtud_policy = pmtud_policy;
    }
```

## `picoquic/quicctx.c:picoquic_get_default_callback_function`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns default_callback_fn; Rust returns default_callback_ctx, a different value.
* C source: `picoquic/quicctx.c:4773-4777`
* C signature: `picoquic_stream_data_cb_fn picoquic_get_default_callback_function(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:3244-3252`
* Rust item: `default_callback`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->default_callback_fn;
}
```

### Rust body
```rust
    pub fn default_callback_ctx(&self) -> Option<&dyn core::any::Any> {
        self.default_callback_ctx.as_deref()
    }
```

## `picoquic/quicctx.c:picoquic_get_local_error`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns local_error; Rust body returns remote_error.
* C source: `picoquic/quicctx.c:5502-5506`
* C signature: `uint64_t picoquic_get_local_error(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4472-4479`
* Rust item: `local_error`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->local_error;
}
```

### Rust body
```rust
    pub fn remote_error(&self) -> u64 {
        self.remote_error
    }
```

## `picoquic/quicctx.c:picoquic_get_remote_cnxid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the current remote CID from path 0; Rust returns initial_connection_id.
* C source: `picoquic/quicctx.c:4465-4469`
* C signature: `picoquic_connection_id_t picoquic_get_remote_cnxid(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3039-3047`
* Rust item: `remote_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id;
}
```

### Rust body
```rust
    pub fn initial_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }
```

## `picoquic/quicctx.c:picoquic_insert_cnx_by_wake_time`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body inserts the connection into the wake tree; Rust body only looks up next_wake_time and returns if missing, with no visible insertion.
* C source: `picoquic/quicctx.c:1510-1513`
* C signature: `void picoquic_insert_cnx_by_wake_time(picoquic_quic_t *, picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:5573-5576`
* Rust item: `picoquic_insert_cnx_by_wake_time`

### C body
```c
{
    picosplay_insert(&quic->cnx_wake_tree, cnx);
}
```

### Rust body
```rust
        let Some(next_time) = self.connections.get(token).map(|cnx| cnx.next_wake_time) else {
            return;
        };
```
