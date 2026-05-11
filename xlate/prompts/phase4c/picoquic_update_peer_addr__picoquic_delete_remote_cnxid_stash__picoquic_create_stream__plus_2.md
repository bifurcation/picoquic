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

## Pair `picoquic/quicctx.c:picoquic_update_peer_addr`
C: `picoquic/quicctx.c:2842-2849 picoquic_update_peer_addr`
Rust: `rs/fq/src/internal.rs:14571-14574 update_peer_addr`

### C body
```c
{
    /* Set the addresses */
    picoquic_store_addr(&path_x->first_tuple->peer_addr, peer_addr);
    /* Keep track of the update */
    path_x->observed_addr_acked = 0;
    path_x->first_tuple->nb_observed_repeat = 0;
}
```

### Rust body
```rust
        if let Some(addr) = peer_addr {
            self.registered_peer_addr = *addr;
        }
```

## Pair `picoquic/quicctx.c:picoquic_delete_remote_cnxid_stash`
C: `picoquic/quicctx.c:3248-3269 picoquic_delete_remote_cnxid_stash`
Rust: `rs/fq/src/internal.rs:5314-5317 delete_remote_connection_id_stash`

### C body
```c
{
    picoquic_remote_cnxid_stash_t* previous = cnx->first_remote_cnxid_stash;

    while (cnxid_stash->cnxid_stash_first != NULL) {
        picoquic_remove_cnxid_from_stash(cnx, cnxid_stash, cnxid_stash->cnxid_stash_first, NULL);
    }

    if (previous == cnxid_stash) {
        cnx->first_remote_cnxid_stash = cnxid_stash->next_stash;
    }
    else {
        while (previous != NULL) {
            if (previous->next_stash == cnxid_stash) {
                previous->next_stash = cnxid_stash->next_stash;
                break;
            }
            previous = previous->next_stash;
        }
    }
    free(cnxid_stash);
}
```

### Rust body
```rust
        if stash_index < self.remote_connection_id_stashes.len() {
            self.remote_connection_id_stashes.swap_remove(stash_index);
        }
```

## Pair `picoquic/quicctx.c:picoquic_create_stream`
C: `picoquic/quicctx.c:3638-3696 picoquic_create_stream`
Rust: `rs/fq/src/internal.rs:9366-9485 create_stream`

### C body
```c
{
    picoquic_stream_head_t* stream = (picoquic_stream_head_t*)malloc(sizeof(picoquic_stream_head_t));
    if (stream != NULL) {
        memset(stream, 0, sizeof(picoquic_stream_head_t));
        picoquic_sack_list_init(&stream->sack_list);
    }

    if (stream != NULL){
        int is_output_stream = 0;
        stream->stream_id = stream_id;
        stream->cnx = cnx;

        if (IS_LOCAL_STREAM_ID(stream_id, cnx->client_mode)) {
            if (IS_BIDIR_STREAM_ID(stream_id)) {
                stream->maxdata_local = cnx->local_parameters.initial_max_stream_data_bidi_local;
                stream->maxdata_remote = cnx->remote_parameters.initial_max_stream_data_bidi_remote;
                is_output_stream = stream->stream_id <= cnx->max_stream_id_bidir_remote;

            }
            else {
                stream->maxdata_local = 0;
                stream->maxdata_remote = cnx->remote_parameters.initial_max_stream_data_uni;
                is_output_stream = stream->stream_id <= cnx->max_stream_id_unidir_remote;
            }
        }
        else {
            if (IS_BIDIR_STREAM_ID(stream_id)) {
                stream->maxdata_local = cnx->local_parameters.initial_max_stream_data_bidi_remote;
                stream->maxdata_remote = cnx->remote_parameters.initial_max_stream_data_bidi_local;
                is_output_stream = 1;
            }
            else {
                stream->maxdata_local = cnx->local_parameters.initial_max_stream_data_uni;
                stream->maxdata_remote = 0;
                is_output_stream = 0;
            }
        }

        stream->stream_priority = cnx->quic->default_stream_priority;

        picosplay_init_tree(&stream->stream_data_tree, picoquic_stream_data_node_compare, picoquic_stream_data_node_create, picoquic_stream_data_node_delete, picoquic_stream_data_node_value);

        picosplay_insert(&cnx->stream_tree, stream);
        if (is_output_stream) {
            picoquic_insert_output_stream(cnx, stream);
        }
        else {
            picoquic_remove_output_stream(cnx, stream);
            picoquic_delete_stream_if_closed(cnx, stream);
        }

        if (stream_id >= cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]) {
            cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)] = NEXT_STREAM_ID_FOR_TYPE(stream_id);
        }
    }

    return stream;
}
```

### Rust body
```rust
    pub fn create_stream(&mut self, stream_id: u64) -> Result<StreamToken, crate::Error> {
        use crate::stream::{Role, StreamId};

        let sid = StreamId(stream_id);
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };

        // Determine flow-control limits and output eligibility.
        let (maxdata_local, maxdata_remote, is_output_stream) = if sid.is_local(local_role) {
            if sid.is_bidir() {
                (
                    self.local_parameters.initial_max_stream_data_bidi_local,
                    self.remote_parameters.initial_max_stream_data_bidi_remote,
                    stream_id <= self.max_stream_id_bidir_remote,
                )
            } else {
                (
                    0u64,
                    self.remote_parameters.initial_max_stream_data_uni,
                    stream_id <= self.max_stream_id_unidir_remote,
                )
            }
        } else if sid.is_bidir() {
            (
                self.local_parameters.initial_max_stream_data_bidi_remote,
                self.remote_parameters.initial_max_stream_data_bidi_local,
                true,
            )
        } else {
            (
                self.local_parameters.initial_max_stream_data_uni,
                0u64,
                false,
            )
        };

        let stream = StreamHead {
            stream_tree_membership: None,
            stream_id,
            affinity_path: None,
            consumed_offset: 0,
            fin_offset: 0,
            reset_offset: 0,
            maxdata_local,
            maxdata_local_acked: 0,
            maxdata_remote,
            local_error: 0,
            remote_error: 0,
            local_stop_error: 0,
            remote_stop_error: 0,
            last_time_data_sent: crate::Instant::from_ticks(0),
            stream_data_tree: crate::splay::SplayTree::default(),
            stream_data_nodes: crate::arena::Arena::new(),
            sent_offset: 0,
            reliable_size: 0,
            send_queue: std::collections::VecDeque::new(),
            app_stream_ctx: None,
            direct_receive_fn: None,
            direct_receive_ctx: None,
            sack_list: SackList::new(),
            stream_priority: crate::DEFAULT_STREAM_PRIORITY,
            is_active: false,
            fin_requested: false,
            fin_sent: false,
            fin_received: false,
            fin_signalled: false,
            reset_requested: false,
            reset_sent: false,
            reset_acked: false,
            reset_received: false,
            reset_signalled: false,
            stop_sending_requested: false,
            stop_sending_sent: false,
            stop_sending_received: false,
            stop_sending_signalled: false,
            max_stream_updated: false,
            stream_data_blocked_sent: false,
            is_output_stream: false,
            is_closed: false,
            is_discarded: false,
            use_app_flow_control: false,
            is_not_coalesced: false,
        };

        let tok = self
            .streams
            .insert(stream)
            .map_err(|_| crate::Error::Memory)?;

        // Insert into the splay tree keyed by stream_id.
        let (splay_tok, _) = self
            .stream_tree
            .insert(stream_id, tok)
            .map_err(|_| crate::Error::Memory)?;
        if let Some(s) = self.streams.get_mut(tok) {
            s.stream_tree_membership = Some(splay_tok);
            s.is_output_stream = false; // handled below
        }

        // Advance next_stream_id if needed.
        // C: STREAM_TYPE_FROM_ID = (stream_id & 3)
        let type_idx = (stream_id & 3) as usize;
        if stream_id >= self.next_stream_id[type_idx] {
            self.next_stream_id[type_idx] = stream_id + 4;
        }

        // Insert into output queue if applicable.
        if is_output_stream {
            // Borrow the stream and mark it, then enqueue its token.
            if let Some(s) = self.streams.get_mut(tok) {
                s.is_output_stream = true;
            }
            self.enqueue_output_stream_token(tok);
        }

        Ok(tok)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_transport_parameters`
C: `picoquic/quicctx.c:4435-4439 picoquic_get_transport_parameters`
Rust: `rs/fq/src/lib.rs:1758-1764 transport_parameters`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return(get_local) ? &cnx->local_parameters : &cnx->remote_parameters;
}
```

### Rust body
```rust
    pub fn transport_parameters(&self, get_local: bool) -> &TransportParameters {
        if get_local {
            &self.local_parameters
        } else {
            &self.remote_parameters
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_cnx_set_spinbit_policy`
C: `picoquic/quicctx.c:4533-4537 picoquic_cnx_set_spinbit_policy`
Rust: `rs/fq/src/lib.rs:2854-2862 set_cnx_spinbit_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->spin_policy = spinbit_policy;
}
```

### Rust body
```rust
    pub fn cnx_spinbit_policy(&self) -> SpinbitVersion {
        self.spin_policy
    }
```
