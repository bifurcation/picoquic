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

## `picoquic/quicctx.c:picoquic_register_net_secret`
* Phase 4C status: `suspect`
* Phase 4C rationale: C unregisters existing secret and inserts into a secret table; Rust errors if already registered and only stores fields locally, with no visible table insert.
* C source: `picoquic/quicctx.c:1374-1393`
* C signature: `int picoquic_register_net_secret(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:3997-4026`
* Rust item: `register_net_secret`

### C body
```c
{
    int ret = 0;

    if (cnx->path[0]->first_tuple->peer_addr.ss_family != 0) {
        picohash_item* item;
        picoquic_unregister_net_secret(cnx);
        picoquic_store_addr(&cnx->registered_secret_addr, (struct sockaddr *)&cnx->path[0]->first_tuple->peer_addr);
        memcpy(&cnx->registered_reset_secret, cnx->path[0]->first_tuple->p_remote_cnxid->reset_secret, PICOQUIC_RESET_SECRET_SIZE);

        item = picohash_retrieve(cnx->quic->table_cnx_by_secret, cnx);
        if (item != NULL) {
            ret = -1;
        } 
        else {
            ret = picohash_insert(cnx->quic->table_cnx_by_secret, cnx);
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn register_net_secret(&mut self) -> Result<(), crate::Error> {
        if !crate::socket_addr_is_unspecified(&self.registered_secret_addr) {
            return Err(crate::Error::Generic);
        }
        let Some(path) = self.paths.first() else {
            return Err(crate::Error::InvalidArgument);
        };
        let Some(tuple) = path.tuples.first() else {
            return Err(crate::Error::InvalidArgument);
        };
        if crate::socket_addr_is_unspecified(&tuple.peer_addr) {
            return Ok(());
        }
        let unique_path_id = if self.is_multipath_enabled {
            path.unique_path_id
        } else {
            0
        };
        let cid_index = tuple.remote_connection_id_index.unwrap_or(0);
        let reset_secret = self
            .remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == unique_path_id)
            .and_then(|stash| stash.connection_ids.get(cid_index))
            .map(|remote_cid| remote_cid.reset_secret)
            .unwrap_or([0u8; RESET_SECRET_SIZE]);
        self.registered_secret_addr = tuple.peer_addr;
        self.registered_reset_secret = reset_secret;
        Ok(())
    }
```
