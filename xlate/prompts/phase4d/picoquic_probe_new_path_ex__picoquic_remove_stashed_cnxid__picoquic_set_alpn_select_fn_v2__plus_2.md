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

## `picoquic/quicctx.c:picoquic_probe_new_path_ex`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C implements probing, path creation, registration, challenge setup, and error handling; Rust is placeholder-like and always returns a generic error.
* C source: `picoquic/quicctx.c:2468-2509`
* C signature: `int picoquic_probe_new_path_ex(picoquic_cnx_t *, const struct sockaddr *, const struct sockaddr *, int, uint64_t, int)`
* Rust source: `rs/fq/src/lib.rs:2448-2458`
* Rust item: `probe_new_path_ex`

### C body
```c
{
    int path_id = -1;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (!cnx->is_multipath_enabled || to_preferred_address) {
        return picoquic_probe_new_tuple(cnx, cnx->path[0], addr_peer, addr_local, if_index, current_time, to_preferred_address);
    }

    int ret = picoquic_check_new_path_allowed(cnx, to_preferred_address);

    if (ret == 0) {
        /* verify that the peer and local addresses are correctly set */
        ret = picoquic_verify_proposed_tuple(cnx, &addr_peer, &addr_local, &if_index);
    }

    if (ret == 0) {
        if (picoquic_create_path(cnx, current_time, addr_local, addr_peer, if_index, UINT64_MAX) > 0) {
            path_id = cnx->nb_paths - 1;
            picoquic_path_t* path_x = cnx->path[path_id];
            ret = picoquic_assign_peer_cnxid_to_tuple(cnx, path_x, path_x->first_tuple);

            if (ret != 0) {
                /* delete the path that was just created! */
                picoquic_delete_path(cnx, path_id);
            }
            else {
                path_x->path_is_published = 1;
                picoquic_register_path(cnx, path_x);
                picoquic_set_path_challenge(cnx, path_id, current_time);
                path_x->is_nat_challenge = 0;
                // path_x->first_tuple->if_index = if_index;
            }
        }
        else {
            ret = PICOQUIC_ERROR_MEMORY;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves path creation and network probing.
        Err(Error::Generic)
    }
```

## `picoquic/quicctx.c:picoquic_remove_stashed_cnxid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C removes a CID from a selected stash and returns that result; Rust body dereferences tuple CID indexes and queues retire frames, which is different behavior.
* C source: `picoquic/quicctx.c:3093-3100`
* C signature: `picoquic_remote_cnxid_t * picoquic_remove_stashed_cnxid(picoquic_cnx_t *, uint64_t, picoquic_remote_cnxid_t *, picoquic_remote_cnxid_t *)`
* Rust source: `rs/fq/src/lib.rs:2981-3027`
* Rust item: `remove_stashed_cnxid`

### C body
```c
{
    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = picoquic_find_or_create_remote_cnxid_stash(cnx,
        (cnx->is_multipath_enabled)?unique_path_id:0, 0);

    return picoquic_remove_cnxid_from_stash(cnx, remote_cnxid_stash, removed, previous);
}
```

### Rust body
```rust
    pub fn dereference_stashed_cnxid(&mut self, path_index: usize, is_deleting_connection: i32) {
        let Some(unique_path_id) = self.paths.get(path_index).map(|path| path.unique_path_id)
        else {
            return;
        };

        let mut retire_sequences = Vec::new();
        if let Some(path) = self.paths.get_mut(path_index) {
            for tuple in &mut path.tuples {
                let Some(cid_idx) = tuple.remote_connection_id_index.take() else {
                    continue;
                };
                if let Some(stash_idx) = self
                    .remote_connection_id_stashes
                    .iter()
                    .position(|s| s.unique_path_id == unique_path_id)
                    && let Some(cid) = self.remote_connection_id_stashes[stash_idx]
                        .connection_ids
                        .get_mut(cid_idx)
                {
                    cid.nb_path_references = cid.nb_path_references.saturating_sub(1);
                    if cid.needs_removal
                        && cid.nb_path_references == 0
                        && is_deleting_connection == 0
                    {
                        retire_sequences.push(cid.sequence);
                    }
                }
            }
        }

        for sequence in retire_sequences {
            let _ = self.queue_retire_connection_id_frame(unique_path_id, sequence);
        }
    }
```

## `picoquic/quicctx.c:picoquic_set_alpn_select_fn_v2`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C clears default ALPN and sets ALPN select callbacks; Rust sets a default stream callback, an unrelated field.
* C source: `picoquic/quicctx.c:4737-4748`
* C signature: `void picoquic_set_alpn_select_fn_v2(picoquic_quic_t *, picoquic_alpn_select_fn_v2)`
* Rust source: `rs/fq/src/lib.rs:1875-1885`
* Rust item: `set_alpn_select_fn`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    if (quic->default_alpn != NULL) {
        free((void *)quic->default_alpn);
        quic->default_alpn = NULL;
    }
    quic->alpn_select_fn_v2 = alpn_select_fn;
    if (alpn_select_fn != NULL) {
        quic->alpn_select_fn = NULL;
    }
}
```

### Rust body
```rust
    pub fn set_default_callback(&mut self, callback: Option<Box<dyn StreamDataCallback>>) {
        self.default_callback_fn = callback;
    }
```

## `picoquic/quicctx.c:picoquic_set_default_connection_id_ttl`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets the default connection ID TTL; Rust returns the current TTL and performs no set.
* C source: `picoquic/quicctx.c:4708-4712`
* C signature: `void picoquic_set_default_connection_id_ttl(picoquic_quic_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:1854-1861`
* Rust item: `set_default_connection_id_ttl`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->local_cnxid_ttl = ttl_usec;
}
```

### Rust body
```rust
    pub fn default_connection_id_ttl(&self) -> u64 {
        self.local_connection_id_ttl
    }
```

## `picoquic/quicctx.c:picoquic_set_desired_version`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets desired_version and enables version negotiation; Rust body sets rejected_version.
* C source: `picoquic/quicctx.c:5201-5207`
* C signature: `void picoquic_set_desired_version(picoquic_cnx_t *, uint32_t)`
* Rust source: `rs/fq/src/lib.rs:2426-2433`
* Rust item: `set_desired_version`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->desired_version = desired_version;
    cnx->do_version_negotiation = 1;
}
```

### Rust body
```rust
    pub fn set_rejected_version(&mut self, rejected_version: u32) {
        self.rejected_version = rejected_version;
    }
```
