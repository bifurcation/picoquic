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

## `picoquic/quicctx.c:picoquic_insert_output_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: C visibly maintains first/last/previous/next output-stream ordering by priority; Rust marks the stream and enqueues a token without visible equivalent linked-list insertion logic.
* C source: `picoquic/quicctx.c:3502-3562`
* C signature: `void picoquic_insert_output_stream(picoquic_cnx_t *, picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/internal.rs:9567-9595`
* Rust item: `insert_output_stream`

### C body
```c
{
    if (stream->is_output_stream == 0)  
    {
        if (IS_CLIENT_STREAM_ID(stream->stream_id) == cnx->client_mode) {
            if (stream->stream_id > ((IS_BIDIR_STREAM_ID(stream->stream_id)) ? cnx->max_stream_id_bidir_remote : cnx->max_stream_id_unidir_remote)) {
                return;
            }
        }

        if (cnx->last_output_stream == NULL) {
            /* insert first stream */
            cnx->last_output_stream = stream;
            cnx->first_output_stream = stream;
        }
        else if (picoquic_compare_stream_priority(stream, cnx->last_output_stream) >= 0) {
            /* insert after last stream. Common case for most applications. */
            stream->previous_output_stream = cnx->last_output_stream;
            cnx->last_output_stream->next_output_stream = stream;
            cnx->last_output_stream = stream;
        }
        else {
            picoquic_stream_head_t* current = cnx->first_output_stream;

            while (current != NULL) {
                int cmp = picoquic_compare_stream_priority(stream, current);

                if (cmp < 0) {
                    /* insert before the current stream, then break */
                    stream->previous_output_stream = current->previous_output_stream;
                    if (stream->previous_output_stream == NULL) {
                        cnx->first_output_stream = stream;
                    }
                    else {
                        stream->previous_output_stream->next_output_stream = stream;
                    }
                    current->previous_output_stream = stream;
                    stream->next_output_stream = current;
                    break;
                }
                else if (cmp == 0) {
                    /* Stream is already there. This is unexpected */
                    break;
                }
                else {
                    current = current->next_output_stream;
                }
            }
            if (current == NULL) {
                /* insert after last stream */
                stream->previous_output_stream = cnx->last_output_stream;
                cnx->last_output_stream->next_output_stream = stream;
                cnx->last_output_stream = stream;
            }
        }

        stream->is_output_stream = 1;
    }
}
```

### Rust body
```rust
    pub fn insert_output_stream(&mut self, stream: &mut StreamHead) {
        if !stream.is_output_stream {
            // Check remote flow-control limit.
            use crate::stream::{Role, StreamId};
            let sid = StreamId(stream.stream_id);
            let local_role = if self.client_mode {
                Role::Client
            } else {
                Role::Server
            };
            if sid.is_local(local_role) {
                let max = if sid.is_bidir() {
                    self.max_stream_id_bidir_remote
                } else {
                    self.max_stream_id_unidir_remote
                };
                if stream.stream_id > max {
                    return;
                }
            }
            stream.is_output_stream = true;
            // Find the token for this stream via its tree membership.
            if let Some(splay_tok) = stream.stream_tree_membership
                && let Some(tok) = self.stream_tree.get(splay_tok).copied()
            {
                self.enqueue_output_stream_token(tok);
            }
        }
    }
```

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

## `picoquic/quicctx.c:picoquic_remove_output_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only removes when is_output_stream is set and also clears stream linkage/state; Rust removes a token from a VecDeque with no visible state clearing.
* C source: `picoquic/quicctx.c:3564-3585`
* C signature: `void picoquic_remove_output_stream(picoquic_cnx_t *, picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/internal.rs:9599-9609`
* Rust item: `remove_output_stream`

### C body
```c
{
    if (stream->is_output_stream) {
        stream->is_output_stream = 0;

        if (stream->previous_output_stream == NULL) {
            cnx->first_output_stream = stream->next_output_stream;
        }
        else {
            stream->previous_output_stream->next_output_stream = stream->next_output_stream;
        }

        if (stream->next_output_stream == NULL) {
            cnx->last_output_stream = stream->previous_output_stream;
        }
        else {
            stream->next_output_stream->previous_output_stream = stream->previous_output_stream;
        }
        stream->previous_output_stream = NULL;
        stream->next_output_stream = NULL;
    }
}
```

### Rust body
```rust
            {
                // Remove by value from the VecDeque.
                if let Some(pos) = self.output_streams.iter().position(|&t| t == tok) {
                    self.output_streams.remove(pos);
                }
            }
```

## `picoquic/quicctx.c:picoquic_set_default_tp`
* Phase 4C status: `suspect`
* Phase 4C rationale: C initializes default transport parameters when tp is NULL; Rust only clones a provided parameter value, with no null/default branch visible.
* C source: `picoquic/quicctx.c:798-811`
* C signature: `int picoquic_set_default_tp(picoquic_quic_t *, picoquic_tp_t *)`
* Rust source: `rs/fq/src/lib.rs:1573-1576`
* Rust item: `set_default_tp`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(quic);

    if (tp == NULL) {
        picoquic_init_transport_parameters(&quic->default_tp);
    }
    else {
        memcpy(&quic->default_tp, tp, sizeof(picoquic_tp_t));
    }

    return ret;
}
```

### Rust body
```rust
    pub fn set_default_tp(&mut self, tp: &TransportParameters) -> Result<(), Error> {
        self.default_tp = tp.clone();
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_set_use_exporter`
* Phase 4C status: `suspect`
* Phase 4C rationale: C delegates to picoquic_tls_set_use_exporter after a thread check; Rust directly assigns self.use_exporter.
* C source: `picoquic/quicctx.c:5548-5551`
* C signature: `void picoquic_set_use_exporter(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1660-1662`
* Rust item: `set_use_exporter`

### C body
```c
void picoquic_set_use_exporter(picoquic_quic_t* quic, int use_exporter) {
    PICOQUIC_THREAD_CHECK(quic);
    picoquic_tls_set_use_exporter(quic, use_exporter);
}
```

### Rust body
```rust
    pub fn set_use_exporter(&mut self, use_exporter: bool) {
        self.use_exporter = use_exporter;
    }
```
