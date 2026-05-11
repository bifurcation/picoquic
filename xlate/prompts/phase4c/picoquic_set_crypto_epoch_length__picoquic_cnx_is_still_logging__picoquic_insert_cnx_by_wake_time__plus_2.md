# Phase 4C body-only translation audit

Compare each C/Rust pair using only the function bodies shown
below. Do not infer from dependencies, type definitions, callers,
module context, tests, or external knowledge. This is a cheap
superficial check for obvious inconsistencies.

Return only JSON with this shape:

```json
{"reviews":[{"c_id":"...","status":"ok|suspect|definitely_not_ok","rationale":"body-visible reason"}]}
```

Status meanings:
* `ok`: no obvious body-level concern.
* `suspect`: possible mismatch visible from the bodies.
* `definitely_not_ok`: clear mismatch or placeholder-like code.

## Pair `picoquic/quicctx.c:picoquic_set_crypto_epoch_length`
C: `picoquic/quicctx.c:1017-1022 picoquic_set_crypto_epoch_length`
Rust: `rs/fq/src/lib.rs:2865-2872 set_crypto_epoch_length`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->crypto_epoch_length_max = (crypto_epoch_length_max == 0) ?
        PICOQUIC_DEFAULT_CRYPTO_EPOCH_LENGTH : crypto_epoch_length_max;
}
```

### Rust body
```rust
    pub fn crypto_epoch_length(&self) -> u64 {
        self.crypto_epoch_length_max
    }
```

## Pair `picoquic/quicctx.c:picoquic_cnx_is_still_logging`
C: `picoquic/quicctx.c:1257-1261 picoquic_cnx_is_still_logging`
Rust: `rs/fq/src/lib.rs:4536-4538 is_still_logging`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return (cnx->nb_packets_logged < PICOQUIC_LOG_PACKET_MAX_SEQUENCE || cnx->quic->use_long_log);
}
```

### Rust body
```rust
    pub fn is_still_logging(&self) -> bool {
        self.nb_packets_logged < LOG_PACKET_MAX_SEQUENCE as u64 || self.use_long_log
    }
```

## Pair `picoquic/quicctx.c:picoquic_insert_cnx_by_wake_time`
C: `picoquic/quicctx.c:1510-1513 picoquic_insert_cnx_by_wake_time`
Rust: `rs/fq/src/lib.rs:5573-5576 picoquic_insert_cnx_by_wake_time`

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

## Pair `picoquic/quicctx.c:picoquic_delete_tuple`
C: `picoquic/quicctx.c:1753-1764 picoquic_delete_tuple`
Rust: `rs/fq/src/internal.rs:4106-4109 delete_tuple`

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

## Pair `picoquic/quicctx.c:picoquic_check_cid_for_new_tuple`
C: `picoquic/quicctx.c:2284-2298 picoquic_check_cid_for_new_tuple`
Rust: `rs/fq/src/internal.rs:4844-4848 check_cid_for_new_tuple`

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
