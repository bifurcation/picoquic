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

## Pair `picoquic/quicctx.c:picoquic_refresh_path_connection_id`
C: `picoquic/quicctx.c:2769-2777 picoquic_refresh_path_connection_id`
Rust: `rs/fq/src/lib.rs:2559-2562 refresh_path_connection_id`

### C body
```c
{
    int ret = -1;
    int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        ret = picoquic_renew_path_connection_id(cnx, cnx->path[path_id]);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn refresh_path_connection_id(&mut self, _unique_path_id: u64) -> Result<(), Error> {
        // Complex: involves CID generation and registration.
        Err(Error::Generic)
    }
```

## Pair `picoquic/quicctx.c:picoquic_dereference_stashed_cnxid_tuple`
C: `picoquic/quicctx.c:3123-3147 picoquic_dereference_stashed_cnxid_tuple`
Rust: `rs/fq/src/internal.rs:5262-5270 dereference_stashed_connection_id_tuple`

### C body
```c
{
    if (tuple->p_remote_cnxid != NULL) {
        if (tuple->p_remote_cnxid->nb_path_references <= 1) {
            uint64_t unique_path_id = (cnx->is_multipath_enabled) ? path_x->unique_path_id : 0;
            if (!is_deleting_cnx && !tuple->p_remote_cnxid->retire_sent) {
                /* if this was the last reference, retire the old cnxid */
                if (picoquic_queue_retire_connection_id_frame(cnx, unique_path_id, tuple->p_remote_cnxid->sequence) != 0) {
                    DBG_PRINTF("Could not properly retire CID[%" PRIu64 "]", tuple->p_remote_cnxid->sequence);
                }
                else {
                    tuple->p_remote_cnxid->retire_sent = 1;
                }
            }
            if (is_deleting_cnx || tuple->p_remote_cnxid->retire_acked) {
                /* Delete and perhaps recycle the queued packets */
                (void)picoquic_remove_stashed_cnxid(cnx, path_x->unique_path_id, tuple->p_remote_cnxid, NULL);
            }
        }
        else {
            tuple->p_remote_cnxid->nb_path_references--;
        }
    }
    tuple->p_remote_cnxid = NULL;
}
```

### Rust body
```rust
        let Some(cid_idx) = tuple.remote_connection_id_index.take() else {
            return;
        };
```

## Pair `picoquic/quicctx.c:picoquic_reorder_output_stream`
C: `picoquic/quicctx.c:3587-3604 picoquic_reorder_output_stream`
Rust: `rs/fq/src/internal.rs:9615-9622 reorder_output_stream`

### C body
```c
{
    if (stream->is_output_stream) {
        if ((stream->previous_output_stream != NULL &&
            picoquic_compare_stream_priority(stream, stream->previous_output_stream) < 0) ||
            (stream->next_output_stream != NULL &&
                picoquic_compare_stream_priority(stream, stream->next_output_stream) > 0)) {
            picoquic_remove_output_stream(cnx, stream);
            stream->is_output_stream = 0;
            picoquic_insert_output_stream(cnx, stream);
        }
    }
}
```

### Rust body
```rust
    pub fn reorder_output_stream(&mut self, stream: &mut StreamHead) {
        if stream.is_output_stream {
            // Simple re-insert: remove then add back at the end.
            self.remove_output_stream(stream);
            stream.is_output_stream = false;
            self.insert_output_stream(stream);
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_create_cnx`
C: `picoquic/quicctx.c:4350-4358 picoquic_create_cnx`
Rust: `rs/fq/src/lib.rs:1927-1953 create_connection`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return picoquic_create_cnx_internal(quic, initial_cnx_id, remote_cnx_id, addr_to, start_time, preferred_version,
        sni, alpn, client_mode, NULL, NULL);
}
```

### Rust body
```rust
    ) -> Option<&mut Connection> {
        let token = self
            .create_cnx_internal(
                initial_cnx_id,
                remote_cnx_id,
                addr_to,
                start_time,
                preferred_version,
                sni,
                alpn,
                client_mode,
                None,
                None,
            )
            .ok()?;
        self.connections.get_mut(token)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_cnx_in_progress`
C: `picoquic/quicctx.c:4507-4511 picoquic_get_cnx_in_progress`
Rust: `rs/fq/src/lib.rs:3232-3240 connection_in_progress`

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
