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

## Pair `picoquic/quicctx.c:picoquic_issue_path_quality_update`
C: `picoquic/quicctx.c:2660-2676 picoquic_issue_path_quality_update`
Rust: `rs/fq/src/internal.rs:6249-6252 issue_path_quality_update`

### C body
```c
{
    int ret = 0;

    if ((path_x->rtt_update_delta > 0 && (
        path_x->smoothed_rtt < path_x->rtt_threshold_low || 
        path_x->smoothed_rtt > path_x->rtt_threshold_high)) ||
        (path_x->pacing_rate_update_delta > 0 && (
            path_x->pacing.rate < path_x->pacing_rate_threshold_low ||
            path_x->pacing.rate > path_x->pacing_rate_threshold_high ||
            path_x->receive_rate_estimate < path_x->receive_rate_threshold_low ||
            path_x->receive_rate_estimate > path_x->receive_rate_threshold_high))) {
        picoquic_refresh_path_quality_thresholds(path_x);
        ret = cnx->callback_fn(cnx, path_x->unique_path_id, NULL, 0, picoquic_callback_path_quality_changed, cnx->callback_ctx, NULL);
    }
    return ret;
}
```

### Rust body
```rust
        let rtt = if path_x.smoothed_rtt.ticks() > 0 {
            path_x.smoothed_rtt
        } else {
```

## Pair `picoquic/quicctx.c:picoquic_subscribe_to_quality_update_per_path_context`
C: `picoquic/quicctx.c:2721-2727 picoquic_subscribe_to_quality_update_per_path_context`
Rust: `rs/fq/src/lib.rs:2774-2782 subscribe_to_quality_update_per_path_context`

### C body
```c
{
    path_x->pacing_rate_update_delta = pacing_rate_delta;
    path_x->rtt_update_delta = rtt_delta;
    picoquic_refresh_path_quality_thresholds(path_x);
}
```

### Rust body
```rust
    ) {
        self.pacing_rate_update_delta = pacing_rate_delta;
        self.rtt_update_delta = rtt_delta;
        self.refresh_quality_thresholds();
    }
```

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

## Pair `picoquic/quicctx.c:picoquic_find_or_create_remote_cnxid_stash`
C: `picoquic/quicctx.c:2892-2915 picoquic_find_or_create_remote_cnxid_stash`
Rust: `rs/fq/src/internal.rs:4911-4922 find_or_create_remote_connection_id_stash`

### C body
```c
{
    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = cnx->first_remote_cnxid_stash;
    picoquic_remote_cnxid_stash_t** p_previous = &cnx->first_remote_cnxid_stash;

    while (remote_cnxid_stash != NULL && remote_cnxid_stash->unique_path_id != unique_path_id) {
        p_previous = &remote_cnxid_stash->next_stash;
        remote_cnxid_stash = remote_cnxid_stash->next_stash;
    }

    if (remote_cnxid_stash == NULL && do_create) {
        remote_cnxid_stash = (picoquic_remote_cnxid_stash_t*)malloc(sizeof(picoquic_remote_cnxid_stash_t));
        if (remote_cnxid_stash != NULL) {
            memset(remote_cnxid_stash, 0, sizeof(picoquic_remote_cnxid_stash_t));
            remote_cnxid_stash->unique_path_id = unique_path_id;
            *p_previous = remote_cnxid_stash;
        }
    }

    return remote_cnxid_stash;
}
```

### Rust body
```rust
        {
            return Some(idx);
        }
```

## Pair `picoquic/quicctx.c:picoquic_remove_cnxid_from_stash`
C: `picoquic/quicctx.c:3055-3091 picoquic_remove_cnxid_from_stash`
Rust: `rs/fq/src/internal.rs:5158-5174 remove_connection_id_from_stash`

### C body
```c
{
    picoquic_remote_cnxid_t* stashed = NULL;

    if (cnx != NULL && remote_cnxid_stash != NULL && remote_cnxid_stash->cnxid_stash_first != NULL && removed != NULL) {
        stashed = remote_cnxid_stash->cnxid_stash_first;
        /* Verify the value of the previous pointer */
        if (previous != NULL) {
            if (previous->next == removed) {
                stashed = removed;
            }
            else {
                previous = NULL;
            }
        }
        /* If the previous pointer was NULL or invalid, reset it */
        if (previous == NULL) {
            while (stashed != NULL && removed != stashed) {
                previous = stashed;
                stashed = stashed->next;
            }
        }
        /* Actually remove the element from the stash */
        if (stashed != NULL) {
            stashed = stashed->next;
            if (previous == NULL) {
                remote_cnxid_stash->cnxid_stash_first = stashed;
            }
            else {
                previous->next = stashed;
            }
            free(removed);
        }
    }
    return stashed;
}
```

### Rust body
```rust
    ) -> Option<usize> {
        let stash = self.remote_connection_id_stashes.get_mut(stash_index)?;
        if removed_index >= stash.connection_ids.len() {
            return None;
        }
        stash.connection_ids.remove(removed_index);
        // Return the index of the next live entry (same index since we removed one).
        if removed_index < stash.connection_ids.len() {
            Some(removed_index)
        } else {
            None
        }
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

## Pair `picoquic/quicctx.c:picoquic_stream_data_node_alloc`
C: `picoquic/quicctx.c:3377-3405 picoquic_stream_data_node_alloc`
Rust: `rs/fq/src/internal.rs:12953-13018 stream_data_node_alloc`

### C body
```c
{
    picoquic_stream_data_node_t* stream_data = quic->p_first_data_node;
    
    if (stream_data == NULL) {
        stream_data = (picoquic_stream_data_node_t*)
            malloc(sizeof(picoquic_stream_data_node_t));

        if (stream_data != NULL) {
            /* It might be sufficient to zero the metadata, but zeroing everything
             * appears safer, and does not confuse checkers like valgrind.
             */
            memset(stream_data, 0, sizeof(picoquic_stream_data_node_t));
            stream_data->quic = quic;
            quic->nb_data_nodes_allocated++;
            if (quic->nb_data_nodes_allocated > quic->nb_data_nodes_allocated_max) {
                quic->nb_data_nodes_allocated_max = quic->nb_data_nodes_allocated;
            }
        }
    }
    else {
        quic->p_first_data_node = stream_data->next_stream_data;
        stream_data->next_stream_data = NULL;
        stream_data->bytes = NULL;
        quic->nb_data_nodes_in_pool--;
    }

    return stream_data;
}
```

### Rust body
```rust
impl StreamHead {
    /// Reset all stream state to defaults, keeping the stream_id.
    /// C: `picoquic_clear_stream`.
    pub fn clear_stream(&mut self) {
        let stream_id = self.stream_id;
        *self = StreamHead {
            stream_tree_membership: None,
            stream_id,
            affinity_path: None,
            consumed_offset: 0,
            fin_offset: 0,
            reset_offset: 0,
            maxdata_local: 0,
            maxdata_local_acked: 0,
            maxdata_remote: 0,
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
            stream_priority: 0,
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
    }
}
```

## Pair `picoquic/quicctx.c:picoquic_last_stream`
C: `picoquic/quicctx.c:3477-3484 picoquic_last_stream`
Rust: `rs/fq/src/internal.rs:9633-9636 last_stream`

### C body
```c
{
#ifdef TOO_CAUTIOUS
    return picoquic_stream_from_node(picosplay_last(&cnx->stream_tree));
#else
    return (picoquic_stream_head_t *)picosplay_last(&cnx->stream_tree);
#endif
}
```

### Rust body
```rust
    pub fn last_stream(&self) -> Option<StreamToken> {
        let st = self.stream_tree.last()?;
        self.stream_tree.get(st).copied()
    }
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

## Pair `picoquic/quicctx.c:picoquic_create_local_cnxid`
C: `picoquic/quicctx.c:3795-3866 picoquic_create_local_cnxid`
Rust: `rs/fq/src/lib.rs:2226-2329 create_local_cnxid`

### C body
```c
{
    picoquic_local_cnxid_list_t* local_cnxid_list = picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 1);
    picoquic_local_cnxid_t* l_cid = NULL;
    int is_unique = 0;

    if (local_cnxid_list != NULL) {
        l_cid = (picoquic_local_cnxid_t*)malloc(sizeof(picoquic_local_cnxid_t));

        if (l_cid != NULL) {
            memset(l_cid, 0, sizeof(picoquic_local_cnxid_t));
            l_cid->create_time = current_time;

            if (cnx->quic->local_cnxid_length == 0) {
                is_unique = 1;
            }
            else {
                for (int i = 0; i < 32; i++) {
                    if (i == 0 && suggested_value != NULL) {
                        l_cid->cnx_id = *suggested_value;
                    }
                    else {
                        picoquic_create_local_cnx_id(cnx->quic, &l_cid->cnx_id, cnx->initial_cnxid);
                    }

                    if (picoquic_cnx_by_id(cnx->quic, l_cid->cnx_id, NULL) == NULL) {
                        is_unique = 1;
                        break;
                    }
                }
            }

            if (is_unique) {
                picoquic_local_cnxid_t* previous = NULL;
                picoquic_local_cnxid_t* next = local_cnxid_list->local_cnxid_first;

                while (next != NULL) {
                    previous = next;
                    next = next->next;
                }

                if (previous == NULL) {
                    local_cnxid_list->local_cnxid_first = l_cid;
                }
                else {
                    previous->next = l_cid;
                }

                l_cid->sequence = local_cnxid_list->local_cnxid_sequence_next++;
                l_cid->path_id = unique_path_id;
                local_cnxid_list->nb_local_cnxid++;

                if (cnx->quic->local_cnxid_length > 0) {
                    picoquic_register_cnx_id(cnx->quic, cnx, l_cid);
                }
                if (l_cid->sequence == 0) {
                    local_cnxid_list->local_cnxid_oldest_created = current_time;
                    if (local_cnxid_list->unique_path_id > cnx->max_path_id_in_cnxid_lists) {
                        cnx->max_path_id_in_cnxid_lists = local_cnxid_list->unique_path_id;
                    }
                }
            }
            else {
                free(l_cid);
                l_cid = NULL;
            }
        }
    }

    return l_cid;
}
```

### Rust body
```rust
    ) -> Result<crate::internal::LocalConnectionIdToken, Error> {
        let initial_connection_id = self
            .connections
            .get(connection)
            .map(|cnx| cnx.initial_connection_id)
            .ok_or(Error::InvalidArgument)?;

        let connection_id = if self.local_connection_id_length == 0 {
            ConnectionId::default()
        } else {
            let mut selected = None;
            for attempt in 0..32 {
                let candidate = if attempt == 0 {
                    if let Some(suggested) = suggested_value {
                        suggested
                    } else {
                        let mut generated = ConnectionId::default();
                        self.create_local_cnx_id(&mut generated, initial_connection_id);
                        generated
                    }
                } else {
                    let mut generated = ConnectionId::default();
                    self.create_local_cnx_id(&mut generated, initial_connection_id);
                    generated
                };
                if self.connection_by_id.lookup(&candidate).is_none() {
                    selected = Some(candidate);
                    break;
                }
            }
            selected.ok_or(Error::Generic)?
        };

        let token = {
            let cnx = self
                .connections
                .get_mut(connection)
                .ok_or(Error::InvalidArgument)?;
            let list_idx = match cnx
                .local_connection_id_lists
                .iter()
                .position(|list| list.unique_path_id == unique_path_id)
            {
                Some(idx) => idx,
                None => {
                    cnx.local_connection_id_lists
                        .push(crate::internal::LocalConnectionIdList {
                            unique_path_id,
                            local_connection_id_sequence_next: 0,
                            local_connection_id_retire_before: 0,
                            local_connection_id_oldest_created: current_time.ticks(),
                            nb_local_connection_id_expired: 0,
                            is_demoted: false,
                            demotion_time: Instant::from_ticks(u64::MAX),
                            connection_ids: Vec::new(),
                        });
                    cnx.local_connection_id_lists.len() - 1
                }
            };
            let sequence =
                cnx.local_connection_id_lists[list_idx].local_connection_id_sequence_next;
            let local_cid = crate::internal::LocalConnectionId {
                connection_by_id_membership: None,
                path_id: unique_path_id,
                sequence,
                create_time: current_time,
                connection_id,
                is_acked: false,
            };
            let token = cnx
                .local_connection_ids
                .insert(local_cid)
                .map_err(|_| Error::Memory)?;
            cnx.local_connection_id_lists[list_idx].local_connection_id_sequence_next += 1;
            cnx.local_connection_id_lists[list_idx]
                .connection_ids
                .push(token);
            if sequence == 0 {
                cnx.local_connection_id_lists[list_idx].local_connection_id_oldest_created =
                    current_time.ticks();
                if unique_path_id > cnx.max_path_id_in_connection_id_lists {
                    cnx.max_path_id_in_connection_id_lists = unique_path_id;
                }
            }
            token
        };

        if self.local_connection_id_length > 0 {
            let (membership, _) = self.connection_by_id.insert(connection_id, connection)?;
            if let Some(cnx) = self.connections.get_mut(connection)
                && let Some(local_cid) = cnx.local_connection_ids.get_mut(token)
            {
                local_cid.connection_by_id_membership = Some(membership);
            }
        }

        Ok(token)
    }
```

## Pair `picoquic/quicctx.c:picoquic_retire_local_cnxid`
C: `picoquic/quicctx.c:3965-3985 picoquic_retire_local_cnxid`
Rust: `rs/fq/src/lib.rs:2975-2977 retire_local_cnxid`

### C body
```c
{
    picoquic_local_cnxid_list_t* local_cnxid_list = picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 0);

    if (local_cnxid_list != NULL) {
        picoquic_local_cnxid_t* local_cnxid = local_cnxid_list->local_cnxid_first;

        while (local_cnxid != NULL) {
            if (local_cnxid->sequence == sequence) {
                break;
            }
            else {
                local_cnxid = local_cnxid->next;
            }
        }

        if (local_cnxid != NULL) {
            picoquic_delete_local_cnxid_listed(cnx, local_cnxid_list, local_cnxid);
        }
    }
}
```

### Rust body
```rust
    pub fn retire_local_cnxid(&mut self, unique_path_id: u64, sequence: u64) {
        self.retire_local_connection_id(unique_path_id, sequence);
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

## Pair `picoquic/quicctx.c:picoquic_get_local_cnxid`
C: `picoquic/quicctx.c:4459-4463 picoquic_get_local_cnxid`
Rust: `rs/fq/src/lib.rs:2953-2971 local_cnxid`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->first_tuple->p_local_cnxid->cnx_id;
}
```

### Rust body
```rust
    ) -> Option<crate::internal::LocalConnectionIdToken> {
        self.find_local_connection_id(unique_path_id, connection_id)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_server_cnxid`
C: `picoquic/quicctx.c:4483-4487 picoquic_get_server_cnxid`
Rust: `rs/fq/src/lib.rs:3061-3072 server_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return (cnx->client_mode) ? cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id : cnx->path[0]->first_tuple->p_local_cnxid->cnx_id;
}
```

### Rust body
```rust
    pub fn logging_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
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
