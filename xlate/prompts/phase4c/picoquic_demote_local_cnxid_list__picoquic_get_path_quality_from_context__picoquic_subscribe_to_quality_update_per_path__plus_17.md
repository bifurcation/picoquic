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

## Pair `picoquic/quicctx.c:picoquic_demote_local_cnxid_list`
C: `picoquic/quicctx.c:2558-2580 picoquic_demote_local_cnxid_list`
Rust: `rs/fq/src/internal.rs:13157-13166 demote_local_connection_id_list`

### C body
```c
{
    int ret = 0;
    picoquic_local_cnxid_list_t* local_cnxid_list =
        picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 0);

    if (local_cnxid_list != NULL &&
        !local_cnxid_list->is_demoted) {
        if ((ret = picoquic_queue_path_abandon_frame(cnx, unique_path_id, reason)) == 0) {
            picoquic_remote_cnxid_stash_t* remote_cnxid_stash =
                picoquic_find_or_create_remote_cnxid_stash(cnx, unique_path_id, 0);
            if (remote_cnxid_stash != NULL) {
                picoquic_delete_remote_cnxid_stash(cnx, remote_cnxid_stash);
            }
            local_cnxid_list->is_demoted = 1;
        }
        else {
            DBG_PRINTF("Cannot abandon path %" PRIu64, unique_path_id);
        }
    }
    return ret;
}
```

### Rust body
```rust
        {
            list.is_demoted = true;
            return 1;
        }
```

## Pair `picoquic/quicctx.c:picoquic_get_path_quality_from_context`
C: `picoquic/quicctx.c:2678-2699 picoquic_get_path_quality_from_context`
Rust: `rs/fq/src/lib.rs:2747-2769 get_path_quality_from_context`

### C body
```c
{
    picoquic_refresh_path_quality_thresholds(path_x);
    quality->cwin = path_x->cwin;
    quality->rtt = path_x->smoothed_rtt;
    quality->rtt_sample = path_x->rtt_sample;
    quality->rtt_min = path_x->rtt_min;
    quality->rtt_max = path_x->rtt_max;
    quality->rtt_variant = path_x->rtt_variant;
    quality->pacing_rate = path_x->pacing.rate;
    quality->receive_rate_estimate = path_x->receive_rate_estimate;
    quality->sent = picoquic_get_sequence_number(path_x->cnx, path_x, picoquic_packet_context_application);
    quality->lost = path_x->nb_losses_found;
    quality->timer_losses = path_x->nb_timer_losses;
    quality->spurious_losses = path_x->nb_spurious;
    quality->max_spurious_rtt = path_x->max_spurious_rtt;
    quality->max_reorder_delay = path_x->max_reorder_delay;
    quality->max_reorder_gap = path_x->max_reorder_gap;
    quality->bytes_in_transit = path_x->bytes_in_transit;
    quality->bytes_sent = path_x->bytes_sent;
    quality->bytes_received = path_x->received;
}
```

### Rust body
```rust
fn get_path_quality_from_context(path_x: &mut Path, sent: u64) -> PathQuality {
    path_x.refresh_quality_thresholds();
    PathQuality {
        receive_rate_estimate: path_x.receive_rate_estimate,
        pacing_rate: path_x.pacing.rate,
        cwin: path_x.cwin,
        rtt: path_x.smoothed_rtt,
        rtt_sample: path_x.rtt_sample,
        rtt_variant: path_x.rtt_variant,
        rtt_min: path_x.rtt_min,
        rtt_max: path_x.rtt_max,
        sent,
        lost: path_x.nb_losses_found,
        timer_losses: path_x.nb_timer_losses,
        spurious_losses: path_x.nb_spurious,
        max_spurious_rtt: path_x.max_spurious_rtt,
        max_reorder_delay: path_x.max_reorder_delay,
        max_reorder_gap: path_x.max_reorder_gap,
        bytes_in_transit: path_x.bytes_in_transit,
        bytes_sent: path_x.bytes_sent,
        bytes_received: path_x.received,
    }
}
```

## Pair `picoquic/quicctx.c:picoquic_subscribe_to_quality_update_per_path`
C: `picoquic/quicctx.c:2729-2747 picoquic_subscribe_to_quality_update_per_path`
Rust: `rs/fq/src/lib.rs:2716-2732 subscribe_to_quality_update_per_path`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    cnx->is_path_quality_update_requested = 1;

    int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        picoquic_subscribe_to_quality_update_per_path_context(cnx->path[path_id],
            pacing_rate_delta, rtt_delta);
    }
    else {
        ret = -1;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        if let Some(path) = self
            .paths
            .iter_mut()
            .find(|p| p.unique_path_id == unique_path_id)
        {
            path.subscribe_to_quality_update_per_path_context(pacing_rate_delta, rtt_delta);
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_stream_path_affinity`
C: `picoquic/quicctx.c:2779-2799 picoquic_set_stream_path_affinity`
Rust: `rs/fq/src/lib.rs:2565-2572 set_stream_path_affinity`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream = picoquic_find_stream(cnx, stream_id);

    if (stream == NULL) {
        ret = -1;
    } else if (unique_path_id == UINT64_MAX) {
        stream->affinity_path = NULL;
    }
    else {
        int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
        if (path_id >= 0) {
            stream->affinity_path = cnx->path[path_id];
        }
        else {
            ret = -1;
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves stream lookup and path pinning.
        Err(Error::Generic)
    }
```

## Pair `picoquic/quicctx.c:picoquic_reset_path_mtu`
C: `picoquic/quicctx.c:2851-2860 picoquic_reset_path_mtu`
Rust: `rs/fq/src/internal.rs:4887-4904 reset_path_mtu`

### C body
```c
{
    /* Re-initialize the MTU */
    path_x->send_mtu = (path_x->first_tuple->peer_addr.ss_family == 0 || path_x->first_tuple->peer_addr.ss_family == AF_INET) ?
        PICOQUIC_INITIAL_MTU_IPV4 : PICOQUIC_INITIAL_MTU_IPV6;
    /* Reset the MTU discovery context */
    path_x->send_mtu_max_tried = 0;
    path_x->mtu_probe_sent = 0;
}
```

### Rust body
```rust
    pub fn get_path_id_from_unique(&self, unique_path_id: u64) -> i32 {
        // C: picoquic_get_path_id_from_unique
        for (i, p) in self.paths.iter().enumerate() {
            if p.unique_path_id == unique_path_id {
                return i as i32;
            }
        }
        -1
    }
```

## Pair `picoquic/quicctx.c:picoquic_init_cnxid_stash`
C: `picoquic/quicctx.c:2917-2942 picoquic_init_cnxid_stash`
Rust: `rs/fq/src/internal.rs:4965-5009 init_connection_id_stash`

### C body
```c
{
    int ret = 0;
    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = picoquic_find_or_create_remote_cnxid_stash(cnx, 0, 1);
    if (remote_cnxid_stash == NULL || remote_cnxid_stash->cnxid_stash_first != NULL) {
        ret = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
    }
    else {
        remote_cnxid_stash->cnxid_stash_first = (picoquic_remote_cnxid_t*)malloc(sizeof(picoquic_remote_cnxid_t));
        cnx->path[0]->first_tuple->p_remote_cnxid = remote_cnxid_stash->cnxid_stash_first;
        if (remote_cnxid_stash->cnxid_stash_first == NULL) {
            ret = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
        }
        else {
            memset(remote_cnxid_stash->cnxid_stash_first, 0, sizeof(picoquic_remote_cnxid_t));
            remote_cnxid_stash->cnxid_stash_first->nb_path_references++;

            /* Initialize the reset secret to a random value. This
            * will prevent spurious matches to an all zero value, for example.
            * The real value will be set when receiving the transport parameters.
            */
            picoquic_public_random(remote_cnxid_stash->cnxid_stash_first->reset_secret, PICOQUIC_RESET_SECRET_SIZE);
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn init_connection_id_stash(&mut self) -> Result<(), crate::Error> {
        // C: picoquic_init_cnxid_stash
        // Ensure the path-0 stash exists and has an initial (empty) entry that
        // `paths[0].tuples[0]` can reference.
        let stash_idx = self
            .find_or_create_remote_connection_id_stash(0, true)
            .ok_or(crate::Error::Memory)?;
        // If there's already a CID in the stash, leave it.
        if !self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .is_empty()
        {
            return Err(crate::Error::Generic); // transport internal error
        }
        let zero_pkt_ctx = PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: 0u64.wrapping_sub(1),
            latest_time_acknowledged: self.start_time,
            highest_acknowledged_time: self.start_time,
            pending: std::collections::BTreeMap::new(),
            retransmitted: std::collections::BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        };
        let initial_rcid = RemoteConnectionId {
            sequence: 0,
            connection_id: crate::ConnectionId::default(),
            reset_secret: [0u8; RESET_SECRET_SIZE],
            nb_path_references: 1,
            needs_removal: false,
            retire_sent: false,
            retire_acked: false,
            pkt_ctx: zero_pkt_ctx,
        };
        self.remote_connection_id_stashes[stash_idx]
            .connection_ids
            .push(initial_rcid);
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_remove_stashed_cnxid`
C: `picoquic/quicctx.c:3093-3100 picoquic_remove_stashed_cnxid`
Rust: `rs/fq/src/lib.rs:2981-3027 remove_stashed_cnxid`

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

## Pair `picoquic/quicctx.c:picoquic_dereference_stashed_cnxid`
C: `picoquic/quicctx.c:3149-3152 picoquic_dereference_stashed_cnxid`
Rust: `rs/fq/src/lib.rs:3030-3036 picoquic_dereference_stashed_cnxid`

### C body
```c
{
    picoquic_dereference_stashed_cnxid_tuple(cnx, path_x, path_x->first_tuple, is_deleting_cnx);
}
```

### Rust body
```rust
    ) {
        self.dereference_stashed_cnxid(path_index, is_deleting_connection);
    }
```

## Pair `picoquic/quicctx.c:picoquic_renew_path_connection_id`
C: `picoquic/quicctx.c:3278-3323 picoquic_renew_path_connection_id`
Rust: `rs/fq/src/internal.rs:5361-5378 renew_path_connection_id`

### C body
```c
{
    int ret = 0;
    picoquic_remote_cnxid_t* stashed = NULL;
    uint64_t cid_path_id = (cnx->is_multipath_enabled) ? path_x->unique_path_id : 0;
    picoquic_remote_cnxid_stash_t* cnxid_stash = picoquic_find_or_create_remote_cnxid_stash(cnx, cid_path_id, 0);

    if (cnxid_stash == NULL) {
        ret = PICOQUIC_ERROR_CNXID_NOT_AVAILABLE;
    }
    else if ((cnx->remote_parameters.migration_disabled != 0 &&
            path_x->first_tuple->p_remote_cnxid != NULL &&
            path_x->first_tuple->p_remote_cnxid->sequence >= cnxid_stash->retire_cnxid_before) ||
            cnx->local_parameters.migration_disabled != 0) {
            /* Do not switch cnx_id if migration is disabled */
            ret = PICOQUIC_ERROR_MIGRATION_DISABLED;
        }
    else {
        stashed = picoquic_obtain_stashed_cnxid(cnx, cid_path_id);

        if (stashed == NULL) {
            ret = PICOQUIC_ERROR_CNXID_NOT_AVAILABLE;
        }
        else if (path_x->first_tuple->p_remote_cnxid != NULL &&
            stashed->sequence == path_x->first_tuple->p_remote_cnxid->sequence) {
            /* If the available cnx_id is same as old one, we do nothing */
            ret = PICOQUIC_ERROR_CNXID_NOT_AVAILABLE;
        }
        else {
            picoquic_dereference_stashed_cnxid(cnx, path_x, 0);

            /* Install the new value */
            path_x->first_tuple->p_remote_cnxid = stashed;
            stashed->nb_path_references++;

            /* If default path, reset the secret pointer */
            if (path_x == cnx->path[0]) {
                ret = picoquic_register_net_secret(cnx);
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn renew_path_connection_id(&mut self, path_x: &mut Path) -> Result<(), crate::Error> {
        let old_sequence = path_x
            .tuples
            .first()
            .and_then(|tuple| tuple.local_connection_id)
            .and_then(|tok| self.local_connection_ids.get(tok))
            .map(|lcid| lcid.sequence);
        let token =
            self.create_local_connection_id(path_x.unique_path_id, None, self.start_time)?;
        if let Some(tuple) = path_x.tuples.first_mut() {
            tuple.local_connection_id = Some(token);
        }
        path_x.path_cid_rotated = true;
        if let Some(sequence) = old_sequence {
            self.queue_retire_connection_id_frame(path_x.unique_path_id, sequence)?;
        }
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_clear_stream`
C: `picoquic/quicctx.c:3427-3445 picoquic_clear_stream`
Rust: `rs/fq/src/internal.rs:12968-13017 clear_stream`

### C body
```c
{
    picoquic_stream_queue_node_t* ready = stream->send_queue;
    picoquic_stream_queue_node_t* next;

    while ((next = ready) != NULL) {
        ready = next->next_stream_data;
        if (next->bytes != NULL) {
            free(next->bytes);
        }
        free(next);
    }
    stream->send_queue = NULL;
    if (stream->is_output_stream) {
        picoquic_remove_output_stream(stream->cnx, stream);
    }
    picosplay_empty_tree(&stream->stream_data_tree);
    picoquic_sack_list_free(&stream->sack_list);
}
```

### Rust body
```rust
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
```

## Pair `picoquic/quicctx.c:picoquic_compare_stream_priority`
C: `picoquic/quicctx.c:3486-3500 picoquic_compare_stream_priority`
Rust: `rs/fq/src/lib.rs:3791-3827 compare_stream_priority`

### C body
```c
int picoquic_compare_stream_priority(picoquic_stream_head_t * stream, picoquic_stream_head_t * other) {
    int ret = 1;
    if (stream->stream_priority < other->stream_priority) {
        ret = -1;
    }
    else if (stream->stream_priority == other->stream_priority) {
        if (stream->stream_id < other->stream_id) {
            ret = -1;
        }
        else if (stream->stream_id == other->stream_id) {
            ret = 0;
        }
    }
    return ret;
}
```

### Rust body
```rust
fn enqueue_output_stream_token(connection: &mut Connection, token: internal::StreamToken) {
    if connection
        .output_streams
        .iter()
        .any(|&existing| existing == token)
    {
        return;
    }

    let pos = connection
        .output_streams
        .iter()
        .position(|&existing| {
            let Some(left) = connection.streams.get(token) else {
                return false;
            };
            let Some(right) = connection.streams.get(existing) else {
                return true;
            };
            left.stream_priority
                .cmp(&right.stream_priority)
                .then_with(|| left.stream_id.cmp(&right.stream_id))
                == core::cmp::Ordering::Less
        })
        .unwrap_or(connection.output_streams.len());
    connection.output_streams.insert(pos, token);
}
```

## Pair `picoquic/quicctx.c:picoquic_next_stream`
C: `picoquic/quicctx.c:3606-3609 picoquic_next_stream`
Rust: `rs/fq/src/internal.rs:9831-9836 next_stream`

### C body
```c
{
    return (picoquic_stream_head_t *)picosplay_next((picosplay_node_t *)stream);
}
```

### Rust body
```rust
    pub fn next_stream(&self, stream: StreamToken) -> Option<StreamToken> {
        // Find the splay token for this stream via its membership field.
        let splay_tok = self.streams.get(stream)?.stream_tree_membership?;
        let next_splay = self.stream_tree.next(splay_tok)?;
        self.stream_tree.get(next_splay).copied()
    }
```

## Pair `picoquic/quicctx.c:picoquic_delete_stream`
C: `picoquic/quicctx.c:3698-3701 picoquic_delete_stream`
Rust: `rs/fq/src/internal.rs:13023-13037 delete_stream`

### C body
```c
{
    picosplay_delete(&cnx->stream_tree, stream);
}
```

### Rust body
```rust
        {
            // Remove from output queue.
            if stream.is_output_stream {
                if let Some(pos) = self.output_streams.iter().position(|&t| t == stream_tok.1) {
                    self.output_streams.remove(pos);
                }
                stream.is_output_stream = false;
            }
            // Remove from the streams arena.
            self.streams.remove(stream_tok.1);
        }
```

## Pair `picoquic/quicctx.c:picoquic_delete_local_cnxid`
C: `picoquic/quicctx.c:3927-3932 picoquic_delete_local_cnxid`
Rust: `rs/fq/src/lib.rs:2943-2945 local_connection_id`

### C body
```c
{
    picoquic_local_cnxid_list_t* local_cnxid_list = picoquic_find_or_create_local_cnxid_list(cnx, l_cid->path_id, 0);

    picoquic_delete_local_cnxid_listed(cnx, local_cnxid_list, l_cid);
}
```

### Rust body
```rust
    pub fn local_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }
```

## Pair `picoquic/quicctx.c:picoquic_check_local_cnxid_ttl`
C: `picoquic/quicctx.c:3987-4015 picoquic_check_local_cnxid_ttl`
Rust: `rs/fq/src/internal.rs:13240-13292 check_local_connection_id_ttl`

### C body
```c
{
    if (current_time - local_cnxid_list->local_cnxid_oldest_created >= cnx->quic->local_cnxid_ttl) {
        picoquic_local_cnxid_t* l_cid = local_cnxid_list->local_cnxid_first;
        local_cnxid_list->local_cnxid_oldest_created = current_time;

        local_cnxid_list->nb_local_cnxid_expired = 0;
        while (l_cid != NULL) {
            if ((current_time - l_cid->create_time) >= cnx->quic->local_cnxid_ttl) {
                local_cnxid_list->nb_local_cnxid_expired++;
                if (l_cid->sequence >= local_cnxid_list->local_cnxid_retire_before) {
                    local_cnxid_list->local_cnxid_retire_before = l_cid->sequence + 1;
                }
            }
            else if (l_cid->create_time < local_cnxid_list->local_cnxid_oldest_created) {
                local_cnxid_list->local_cnxid_oldest_created = l_cid->create_time;
            }
            l_cid = l_cid->next;
        }

        cnx->next_wake_time = current_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_QUICCTX);
    } else {
        if (*next_wake_time - local_cnxid_list->local_cnxid_oldest_created > cnx->quic->local_cnxid_ttl) {
            *next_wake_time = local_cnxid_list->local_cnxid_oldest_created + cnx->quic->local_cnxid_ttl;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_QUICCTX);
        }
    }
}
```

### Rust body
```rust
    ) {
        let ttl = self.local_connection_id_ttl;
        if ttl == u64::MAX {
            return;
        }

        if current_time
            .ticks()
            .saturating_sub(local_connection_id_list.local_connection_id_oldest_created)
            >= ttl
        {
            local_connection_id_list.local_connection_id_oldest_created = current_time.ticks();
            local_connection_id_list.nb_local_connection_id_expired = 0;

            for &tok in &local_connection_id_list.connection_ids {
                if let Some(l_cid) = self.local_connection_ids.get(tok) {
                    if current_time
                        .ticks()
                        .saturating_sub(l_cid.create_time.ticks())
                        >= ttl
                    {
                        local_connection_id_list.nb_local_connection_id_expired += 1;
                        if l_cid.sequence
                            >= local_connection_id_list.local_connection_id_retire_before
                        {
                            local_connection_id_list.local_connection_id_retire_before =
                                l_cid.sequence + 1;
                        }
                    } else if l_cid.create_time.ticks()
                        < local_connection_id_list.local_connection_id_oldest_created
                    {
                        local_connection_id_list.local_connection_id_oldest_created =
                            l_cid.create_time.ticks();
                    }
                }
            }

            self.next_wake_time = current_time;
        } else if next_wake_time
            .ticks()
            .saturating_sub(local_connection_id_list.local_connection_id_oldest_created)
            > ttl
        {
            *next_wake_time = Instant::from_ticks(
                local_connection_id_list.local_connection_id_oldest_created + ttl,
            );
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_create_client_cnx`
C: `picoquic/quicctx.c:4360-4382 picoquic_create_client_cnx`
Rust: `rs/fq/src/lib.rs:1967-1991 create_client_connection`

### C body
```c
{
    picoquic_cnx_t* cnx = picoquic_create_cnx(quic, picoquic_null_connection_id, picoquic_null_connection_id, addr, start_time, preferred_version, sni, alpn, 1);

    if (cnx != NULL) {
        int ret;

        if (callback_fn != NULL)
            cnx->callback_fn = callback_fn;
        if (callback_ctx != NULL)
            cnx->callback_ctx = callback_ctx;
        ret = picoquic_start_client_cnx(cnx);
        if (ret != 0) {
            /* Cannot just do partial initialization! */
            picoquic_delete_cnx(cnx);
            cnx = NULL;
        }
    }

    return cnx;
}
```

### Rust body
```rust
    ) -> Option<&mut Connection> {
        // C: picoquic_create_client_cnx — wraps picoquic_create_cnx with
        // null CIDs, then runs picoquic_start_client_cnx and rolls back
        // on failure.  Callback installation is folded in here; the Rust
        // shape stores the boxed callback on Connection, which is the
        // moral equivalent of `cnx->callback_fn`/`cnx->callback_ctx`.
        self.create_connection(
            ConnectionId::with_size(0)?,
            ConnectionId::with_size(0)?,
            Some(addr),
            start_time,
            preferred_version,
            sni,
            alpn,
            true,
        )
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_peer_addr`
C: `picoquic/quicctx.c:4441-4445 picoquic_get_peer_addr`
Rust: `rs/fq/src/lib.rs:4955-4963 get_peer_addr`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    *addr = (struct sockaddr*)&cnx->path[0]->first_tuple->peer_addr;
}
```

### Rust body
```rust
    pub fn get_local_addr(&self) -> std::net::SocketAddr {
        self.local_addr()
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_remote_cnxid`
C: `picoquic/quicctx.c:4465-4469 picoquic_get_remote_cnxid`
Rust: `rs/fq/src/lib.rs:3039-3047 remote_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id;
}
```

### Rust body
```rust
    pub fn initial_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_logging_cnxid`
C: `picoquic/quicctx.c:4489-4493 picoquic_get_logging_cnxid`
Rust: `rs/fq/src/lib.rs:3070-3077 logging_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->initial_cnxid;
}
```

### Rust body
```rust
    pub fn start_time(&self) -> u64 {
        self.start_time.ticks()
    }
```

## Pair `picoquic/quicctx.c:picoquic_is_0rtt_available`
C: `picoquic/quicctx.c:4513-4517 picoquic_is_0rtt_available`
Rust: `rs/fq/src/lib.rs:3080-3088 is_0rtt_available`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return (cnx->crypto_context[picoquic_epoch_0rtt].aead_encrypt == NULL) ? 0 : 1;
}
```

### Rust body
```rust
    pub fn is_backlog_empty(&self) -> bool {
        self.nb_bytes_queued == 0 && self.misc_frames.is_empty() && self.output_streams.is_empty()
    }
```
