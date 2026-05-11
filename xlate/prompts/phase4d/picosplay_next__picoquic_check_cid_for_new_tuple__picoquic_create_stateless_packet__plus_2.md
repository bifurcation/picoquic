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

## `picoquic/picosplay.c:picosplay_next`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C computes the successor using right subtree or parent traversal; Rust body shown only validates the token and returns None if invalid.
* C source: `picoquic/picosplay.c:245-256`
* C signature: `picosplay_node_t * picosplay_next(picosplay_node_t *)`
* Rust source: `rs/fq/src/splay.rs:545-548`
* Rust item: `next`

### C body
```c
picosplay_node_t* picosplay_next(picosplay_node_t *node) {
    if(node->right != NULL)
        return leftmost(node->right);
    while(node->parent != NULL && node == node->parent->right)
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

## `picoquic/quicctx.c:picoquic_check_cid_for_new_tuple`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body returns 0 or one of two blocked error codes based on stash availability and path limits; Rust body only begins computing a stash_id and has no visible return/error logic.
* C source: `picoquic/quicctx.c:2284-2298`
* C signature: `int picoquic_check_cid_for_new_tuple(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:4844-4848`
* Rust item: `check_cid_for_new_tuple`

### C body
```c
{
    int ret = 0;
    /* testing availability of connection ID is sufficient. */
    if (picoquic_obtain_stashed_cnxid(cnx, unique_path_id) == NULL) {
        if (cnx->unique_path_id_next > cnx->max_path_id_remote) {
            ret = PICOQUIC_ERROR_PATH_ID_BLOCKED;
        }
        else
        {
            ret = PICOQUIC_ERROR_PATH_CID_BLOCKED;
        }
    }
    return ret;
}
```

### Rust body
```rust
        let stash_id = if self.is_multipath_enabled {
            unique_path_id
        } else {
```

## `picoquic/quicctx.c:picoquic_create_stateless_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C allocates and returns a stateless packet, while Rust queues an existing packet and returns nothing.
* C source: `picoquic/quicctx.c:1218-1224`
* C signature: `picoquic_stateless_packet_t * picoquic_create_stateless_packet(picoquic_quic_t *)`
* Rust source: `rs/fq/src/internal.rs:610-630`
* Rust item: `create_stateless_packet`

### C body
```c
{
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(quic);
#endif
    return (picoquic_stateless_packet_t*)malloc(sizeof(picoquic_stateless_packet_t));
}
```

### Rust body
```rust
    pub fn queue_stateless_packet(&mut self, sp: StatelessPacket) {
        self.pending_stateless_packets.push_back(sp);
    }
```

## `picoquic/quicctx.c:picoquic_delete_path`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C resets packet/ack contexts, clears stream and retransmission references, invokes callbacks, clears path data, deletes related CID state, and compacts the path table; Rust only swap-removes the path.
* C source: `picoquic/quicctx.c:1901-1961`
* C signature: `void picoquic_delete_path(picoquic_cnx_t *, int)`
* Rust source: `rs/fq/src/internal.rs:4717-4721`
* Rust item: `delete_path`

### C body
```c
{
    picoquic_path_t * path_x = cnx->path[path_index];
    picoquic_packet_t* p = NULL;
    picoquic_stream_head_t* stream = NULL;

    picoquic_reset_packet_context(cnx, &path_x->pkt_ctx);
    picoquic_reset_ack_context(&path_x->ack_ctx);

    if (cnx->quic->F_log != NULL) {
        fflush(cnx->quic->F_log);
    }

    /* if there are references to path in streams, remove them */
    stream = picoquic_first_stream(cnx);
    while (stream != NULL) {
        if (stream->affinity_path == path_x) {
            stream->affinity_path = NULL;
        }
        stream = picoquic_next_stream(stream);
    }

    /* Signal to the application */
    if (cnx->are_path_callbacks_enabled && cnx->callback_fn != NULL &&
        cnx->callback_fn(cnx, path_x->unique_path_id, NULL, 0, picoquic_callback_path_deleted,
        cnx->callback_ctx, path_x->app_path_ctx) != 0) {
        picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0, "Path deleted callback failed.");
    }
    /* Remove old path data from retransmitted queue */
    /* TODO: what if using multiple number spaces? */
    for (picoquic_packet_context_enum pc = 0; pc < picoquic_nb_packet_context; pc++)
    {
        p = cnx->pkt_ctx[pc].retransmitted_newest;
        while (p != NULL) {
            if (p->send_path == path_x) {
                DBG_PRINTF("Erase path for old packet pc: %d, seq:%" PRIu64 "\n", pc, p->sequence_number);
                p->send_path = NULL;
            }
            p = p->packet_next;
        }
    }

    if (cnx->is_multipath_enabled) {
        /* delete the local CID context used by the path */
        picoquic_local_cnxid_list_t* local_cnxid_list = picoquic_find_or_create_local_cnxid_list(cnx, path_x->unique_path_id, 0);
        if (local_cnxid_list != NULL) {
            picoquic_delete_local_cnxid_list(cnx, local_cnxid_list);
        }
    }

    /* Free the data and free the path context. */
    picoquic_clear_path_data(cnx, path_x);

    /* Compact the path table  */
    for (int i = path_index + 1; i < cnx->nb_paths; i++) {
        cnx->path[i-1] = cnx->path[i];
    }

    cnx->nb_paths--;
    cnx->path[cnx->nb_paths] = NULL;
}
```

### Rust body
```rust
        if idx < self.paths.len() {
            self.paths.swap_remove(idx);
        }
```

## `picoquic/quicctx.c:picoquic_enable_sslkeylog`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets enable_sslkeylog from an argument; Rust body is a getter returning enable_sslkeylog.
* C source: `picoquic/quicctx.c:4652-4657`
* C signature: `void picoquic_enable_sslkeylog(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1101-1108`
* Rust item: `set_sslkeylog_enabled`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->enable_sslkeylog = (enable_sslkeylog != 0);
}
```

### Rust body
```rust
    pub fn is_sslkeylog_enabled(&self) -> bool {
        self.enable_sslkeylog
    }
```
