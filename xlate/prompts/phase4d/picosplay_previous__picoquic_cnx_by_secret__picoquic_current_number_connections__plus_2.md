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

## `picoquic/picosplay.c:picosplay_previous`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C computes the predecessor by walking left/rightmost/parents; Rust body only checks token validity and returns None.
* C source: `picoquic/picosplay.c:237-243`
* C signature: `picosplay_node_t * picosplay_previous(picosplay_node_t *)`
* Rust source: `rs/fq/src/splay.rs:520-523`
* Rust item: `previous`

### C body
```c
picosplay_node_t* picosplay_previous(picosplay_node_t* node) {
    if (node->left != NULL)
        return rightmost(node->left);
    while (node->parent != NULL && node == node->parent->left)
        node = node->parent;
    return node->parent;
}
```

### Rust body
```rust
        if !self.is_valid(token) {
            return None;
        }
```

## `picoquic/quicctx.c:picoquic_cnx_by_secret`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C builds a lookup key and retrieves a connection; Rust body only checks reset_secret length and returns None if short.
* C source: `picoquic/quicctx.c:5277-5292`
* C signature: `picoquic_cnx_t * picoquic_cnx_by_secret(picoquic_quic_t *, const uint8_t *, const struct sockaddr *)`
* Rust source: `rs/fq/src/internal.rs:5872-5879`
* Rust item: `connection_by_secret`

### C body
```c
{
    picoquic_cnx_t* ret = NULL;
    picohash_item* item;
    picoquic_cnx_t dummy_cnx = { 0 };

    picoquic_store_addr(&dummy_cnx.registered_secret_addr, addr);
    memcpy(dummy_cnx.registered_reset_secret, reset_secret, PICOQUIC_RESET_SECRET_SIZE);

    item = picohash_retrieve(quic->table_cnx_by_secret, &dummy_cnx);

    if (item != NULL) {
        ret = ((picoquic_cnx_t*)item->key);
    }
    return ret;
}
```

### Rust body
```rust
        if reset_secret.len() < RESET_SECRET_SIZE {
            return None;
        }
```

## `picoquic/quicctx.c:picoquic_current_number_connections`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns current_number_connections; Rust body shown is a setter for max_half_open_before_retry.
* C source: `picoquic/quicctx.c:624-628`
* C signature: `uint32_t picoquic_current_number_connections(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:1173-1181`
* Rust item: `current_number_connections`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->current_number_connections;
}
```

### Rust body
```rust
    pub fn set_max_half_open_retry_threshold(&mut self, max_half_open_before_retry: u32) {
        self.max_half_open_before_retry = max_half_open_before_retry;
    }
```

## `picoquic/quicctx.c:picoquic_delete_tuple`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body conditionally dereferences a remote connection ID, unchains the tuple from the path, and frees it; Rust body only removes an indexed element from a vector.
* C source: `picoquic/quicctx.c:1753-1764`
* C signature: `void picoquic_delete_tuple(picoquic_path_t *, picoquic_tuple_t *, int)`
* Rust source: `rs/fq/src/internal.rs:4106-4109`
* Rust item: `delete_tuple`

### C body
```c
{
    /* TODO: dereference local CID, retire remote CID ??? */
    /* Dereference the remote CID */
    if (tuple->p_remote_cnxid != NULL) {
        picoquic_dereference_stashed_cnxid_tuple(path_x->cnx, path_x, tuple, is_deleting_path);
    }
    /* Remove from chain to the path */
    picoquic_unchain_tuple(path_x, tuple);
    /* And finally free */
    free(tuple);
}
```

### Rust body
```rust
        if index < self.tuples.len() {
            self.tuples.remove(index);
        }
```

## `picoquic/quicctx.c:picoquic_enforce_client_only`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets enforce_client_only from do_enforce; Rust body sets default padding fields in a differently named function.
* C source: `picoquic/quicctx.c:5553-5557`
* C signature: `void picoquic_enforce_client_only(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1666-1674`
* Rust item: `enforce_client_only`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->enforce_client_only = (do_enforce)?1:0;
}
```

### Rust body
```rust
    pub fn set_default_padding(&mut self, padding_multiple: u32, padding_minsize: u32) {
        self.padding_multiple_default = padding_multiple;
        self.padding_minsize_default = padding_minsize;
    }
```
