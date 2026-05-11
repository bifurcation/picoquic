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

## `picoquic/quicctx.c:picoquic_get_path_addr`
* Phase 4C status: `suspect`
* Phase 4C rationale: Valid local selector address mapping matches, but C returns 0 when the path id is not found while Rust returns an error.
* C source: `picoquic/quicctx.c:2812-2840`
* C signature: `int picoquic_get_path_addr(picoquic_cnx_t *, uint64_t, int, struct sockaddr_storage *)`
* Rust source: `rs/fq/src/lib.rs:2675-2688`
* Rust item: `path_addr`

### C body
```c
{
    int ret = 0;
    int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        struct sockaddr_storage* local_addr = NULL;
        switch (local) {
        case 1:
            local_addr = &cnx->path[path_id]->first_tuple->local_addr;
            break;
        case 2:
            local_addr = &cnx->path[path_id]->first_tuple->peer_addr;
            break;
        case 3:
            local_addr = &cnx->path[path_id]->first_tuple->observed_addr;
            break;
        default:
            break;
        }
        if (local_addr == NULL) {
            ret = -1;
        }
        else {
            picoquic_store_addr(addr, (struct sockaddr*)local_addr);
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn path_addr(&self, unique_path_id: u64, local: i32) -> Result<SocketAddr, Error> {
        let path = self
            .paths
            .iter()
            .find(|p| p.unique_path_id == unique_path_id)
            .ok_or(Error::InvalidArgument)?;
        let tuple = path.tuples.first().ok_or(Error::InvalidArgument)?;
        match local {
            1 => Ok(tuple.local_addr),
            2 => Ok(tuple.peer_addr),
            3 => Ok(tuple.observed_addr),
            _ => Err(Error::InvalidArgument),
        }
    }
```

## `picoquic/quicctx.c:picoquic_is_handshake_error`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks crypto-error range or exact TLS_HANDSHAKE_FAILED value; Rust only checks whether the high byte equals 1.
* C source: `picoquic/quicctx.c:5175-5179`
* C signature: `int picoquic_is_handshake_error(uint64_t)`
* Rust source: `rs/fq/src/lib.rs:999-1002`
* Rust item: `is_handshake_error`

### C body
```c
{
    return ((error_code & 0xFF00) == PICOQUIC_TRANSPORT_CRYPTO_ERROR(0) ||
        error_code == PICOQUIC_TLS_HANDSHAKE_FAILED);
}
```

### Rust body
```rust
pub fn is_handshake_error(error_code: u64) -> bool {
    // TLS handshake errors occupy the range 0x0100..=0x01ff
    (error_code >> 8) == 1
}
```

## `picoquic/quicctx.c:picoquic_registered_token_clear`
* Phase 4C status: `suspect`
* Phase 4C rationale: C repeatedly removes first tokens only until the first non-expired token; Rust scans the whole tree and removes all expired entries it finds.
* C source: `picoquic/quicctx.c:597-610`
* C signature: `void picoquic_registered_token_clear(picoquic_quic_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:2216-2241`
* Rust item: `registered_token_clear`

### C body
```c
{
    int end_reached = 0;
    do {
        picoquic_registered_token_t* rt_first = (picoquic_registered_token_t*)
            picoquic_registered_token_value(picosplay_first(&quic->token_reuse_tree));
        if (rt_first == NULL || rt_first->token_time >= expiry_time_max) {
            end_reached = 1;
        }
        else {
            picosplay_delete_hint(&quic->token_reuse_tree, &rt_first->registered_token_node);
        }
    } while (!end_reached);
}
```

### Rust body
```rust
    pub fn registered_token_clear(&mut self, expiry_time_max: Instant) {
        let mut expired = Vec::new();
        let mut current = self.token_reuse_tree.first();
        while let Some(st) = current {
            current = self.token_reuse_tree.next(st);
            if let Some(&arena_tok) = self.token_reuse_tree.get(st) {
                let is_expired = self
                    .registered_tokens
                    .get(arena_tok)
                    .map(|rt| rt.token_time < expiry_time_max)
                    .unwrap_or(true);
                if is_expired {
                    expired.push((st, Some(arena_tok)));
                }
            } else {
                expired.push((st, None));
            }
        }

        for (st, arena_tok) in expired {
            self.token_reuse_tree.remove(st);
            if let Some(arena_tok) = arena_tok {
                self.registered_tokens.remove(arena_tok);
            }
        }
    }
```

## `picoquic/quicctx.c:picoquic_renew_connection_id`
* Phase 4C status: `suspect`
* Phase 4C rationale: C delegates to picoquic_renew_path_connection_id and returns its error; Rust manually obtains a stashed ID and always returns Ok after a valid index, even if no replacement occurs.
* C source: `picoquic/quicctx.c:3325-3337`
* C signature: `int picoquic_renew_connection_id(picoquic_cnx_t *, int)`
* Rust source: `rs/fq/src/internal.rs:4696-4714`
* Rust item: `renew_connection_id`

### C body
```c
{
    int ret;

    if (path_id >= cnx->nb_paths) {
        ret = -1;
    }
    else {
        ret = picoquic_renew_path_connection_id(cnx, cnx->path[path_id]);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn renew_connection_id(&mut self, path_id: i32) -> Result<(), crate::Error> {
        let idx = path_id as usize;
        if idx >= self.paths.len() {
            return Err(crate::Error::InvalidArgument);
        }
        if let Some((stash_idx, cid_idx)) =
            self.obtain_stashed_connection_id(self.paths[idx].unique_path_id)
            && let Some(tuple) = self.paths[idx].tuples.first_mut()
        {
            tuple.remote_connection_id_index = Some(cid_idx);
            if let Some(cid) = self.remote_connection_id_stashes[stash_idx]
                .connection_ids
                .get_mut(cid_idx)
            {
                cid.nb_path_references += 1;
            }
        }
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_set_first_if_index`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only sets the first tuple if the connection state is client_init and otherwise still returns success; Rust sets when the first path/tuple exists regardless of state and errors if missing.
* C source: `picoquic/quicctx.c:1774-1784`
* C signature: `int picoquic_set_first_if_index(picoquic_cnx_t *, unsigned long)`
* Rust source: `rs/fq/src/lib.rs:2661-2669`
* Rust item: `set_first_if_index`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx->cnx_state == picoquic_state_client_init) {
        cnx->path[0]->first_tuple->if_index = if_index;
    }
    return ret;
}
```

### Rust body
```rust
    pub fn set_first_if_index(&mut self, if_index: u32) -> Result<(), Error> {
        if let Some(path) = self.paths.first_mut()
            && let Some(tuple) = path.tuples.first_mut()
        {
            tuple.if_index = if_index as core::ffi::c_ulong;
            return Ok(());
        }
        Err(Error::InvalidArgument)
    }
```
