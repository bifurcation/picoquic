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

## Pair `picoquic/quicctx.c:picoquic_refresh_path_quality_thresholds`
C: `picoquic/quicctx.c:2628-2658 picoquic_refresh_path_quality_thresholds`
Rust: `rs/fq/src/internal.rs:6100-6103 refresh_quality_thresholds`

### C body
```c
{
    if (path_x->rtt_update_delta > 0) {
        if (path_x->smoothed_rtt > path_x->rtt_update_delta) {
            path_x->rtt_threshold_low = path_x->smoothed_rtt - path_x->rtt_update_delta;
        }
        else {
            path_x->rtt_threshold_low = 0;
        }
        path_x->rtt_threshold_high = path_x->smoothed_rtt + path_x->rtt_update_delta;
    }

    if (path_x->pacing_rate_update_delta > 0) {
        if (path_x->pacing.rate > path_x->pacing_rate_update_delta) {
            path_x->pacing_rate_threshold_low = path_x->pacing.rate - path_x->pacing_rate_update_delta;
        }
        else {
            path_x->pacing_rate_threshold_low = 0;
        }
        path_x->pacing_rate_threshold_high = path_x->pacing.rate + path_x->pacing_rate_update_delta;
        if (path_x->receive_rate_estimate > path_x->pacing_rate_update_delta) {
            path_x->receive_rate_threshold_low = path_x->receive_rate_estimate - path_x->pacing_rate_update_delta;
        }
        else {
            path_x->receive_rate_threshold_low = 0;
        }
        path_x->receive_rate_threshold_high = path_x->receive_rate_estimate + path_x->pacing_rate_update_delta;
    }
}
```

### Rust body
```rust
        let rtt = if self.smoothed_rtt.ticks() > 0 {
            self.smoothed_rtt
        } else {
```

## Pair `picoquic/quicctx.c:picoquic_get_default_path_quality`
C: `picoquic/quicctx.c:2715-2719 picoquic_get_default_path_quality`
Rust: `rs/fq/src/lib.rs:2704-2732 default_path_quality`

### C body
```c
{
    picoquic_path_t* path_x = cnx->path[0];
    picoquic_get_path_quality_from_context(path_x, quality);
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

## Pair `picoquic/quicctx.c:picoquic_default_quality_update`
C: `picoquic/quicctx.c:2762-2767 picoquic_default_quality_update`
Rust: `rs/fq/src/lib.rs:1719-1728 default_quality_update`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->pacing_rate_update_delta = pacing_rate_delta;
    quic->rtt_update_delta = rtt_delta;
}
```

### Rust body
```rust
    pub fn set_cwin_max(&mut self, cwin_max: u64) {
        self.cwin_max = if cwin_max == 0 { u64::MAX } else { cwin_max };
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_path_addr`
C: `picoquic/quicctx.c:2812-2840 picoquic_get_path_addr`
Rust: `rs/fq/src/lib.rs:2675-2688 path_addr`

### C body
```c
{
    int ret = 0;
    int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        struct sockaddr_storage* local_addr = NULL;
        switch (local) {
        case 1:
            local_addr = &cnx->path[path_id]->first_tuple->local_addr;
            break;
        case 2:
            local_addr = &cnx->path[path_id]->first_tuple->peer_addr;
            break;
        case 3:
            local_addr = &cnx->path[path_id]->first_tuple->observed_addr;
            break;
        default:
            break;
        }
        if (local_addr == NULL) {
            ret = -1;
        }
        else {
            picoquic_store_addr(addr, (struct sockaddr*)local_addr);
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn path_addr(&self, unique_path_id: u64, local: i32) -> Result<SocketAddr, Error> {
        let path = self
            .paths
            .iter()
            .find(|p| p.unique_path_id == unique_path_id)
            .ok_or(Error::InvalidArgument)?;
        let tuple = path.tuples.first().ok_or(Error::InvalidArgument)?;
        match local {
            1 => Ok(tuple.local_addr),
            2 => Ok(tuple.peer_addr),
            3 => Ok(tuple.observed_addr),
            _ => Err(Error::InvalidArgument),
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_init_packet_ctx`
C: `picoquic/quicctx.c:2875-2890 picoquic_init_packet_ctx`
Rust: `rs/fq/src/internal.rs:8758-8769 init_packet_ctx`

### C body
```c
{
    if (cnx->quic->random_initial && 
        (pc == picoquic_packet_context_initial || cnx->quic->random_initial > 1)){
        pkt_ctx->send_sequence = picoquic_crypto_uniform_random(cnx->quic, PICOQUIC_PN_RANDOM_RANGE) +
            PICOQUIC_PN_RANDOM_MIN;
    }
    else {
        pkt_ctx->send_sequence = 0;
    }
    pkt_ctx->pending_last = NULL;
    pkt_ctx->pending_first = NULL;
    pkt_ctx->highest_acknowledged = pkt_ctx->send_sequence - 1;
    pkt_ctx->latest_time_acknowledged = cnx->start_time;
    pkt_ctx->highest_acknowledged_time = cnx->start_time;
}
```

### Rust body
```rust
        if self.random_initial != 0 && (pc == PacketContext::Initial || self.random_initial > 1) {
            let mut rnd = [0u8; 8];
            if fill_system_random(&mut rnd).is_ok() {
                pkt_ctx.send_sequence =
                    u64::from_le_bytes(rnd) % PN_RANDOM_RANGE as u64 + PN_RANDOM_MIN as u64;
            } else {
                pkt_ctx.send_sequence =
                    crate::public_random_64() % PN_RANDOM_RANGE as u64 + PN_RANDOM_MIN as u64;
            }
        } else {
```

## Pair `picoquic/quicctx.c:picoquic_stash_remote_cnxid`
C: `picoquic/quicctx.c:3038-3053 picoquic_stash_remote_cnxid`
Rust: `rs/fq/src/internal.rs:16260-16304 stash_remote_cnxid`

### C body
```c
{
    uint64_t transport_error = 0;
    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = picoquic_find_or_create_remote_cnxid_stash(cnx, unique_path_id, 1);

    if (remote_cnxid_stash == NULL) {
        transport_error = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
    }
    else {
        transport_error = picoquic_add_remote_cnxid_to_stash(cnx, remote_cnxid_stash, retire_before_next,
            sequence, cid_length, cnxid_bytes, secret_bytes, pstashed);
    }
    return transport_error;
}
```

### Rust body
```rust
) -> crate::Result<()> {
    use crate::RESET_SECRET_SIZE;
    // Find or create a default stash (path_id 0).
    let stash_idx = cnx
        .find_or_create_remote_connection_id_stash(0, true)
        .ok_or(crate::Error::Memory)?;
    let cid = ConnectionId::clone_from_slice(cid_bytes).ok_or(crate::Error::Memory)?;
    let mut secret = [0u8; RESET_SECRET_SIZE];
    let copy_len = reset_secret.len().min(RESET_SECRET_SIZE);
    secret[..copy_len].copy_from_slice(&reset_secret[..copy_len]);
    let r_cid = RemoteConnectionId {
        sequence,
        connection_id: cid,
        reset_secret: secret,
        nb_path_references: 0,
        needs_removal: false,
        retire_sent: false,
        retire_acked: false,
        pkt_ctx: PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: u64::MAX,
            latest_time_acknowledged: crate::Instant::from_ticks(0),
            highest_acknowledged_time: crate::Instant::from_ticks(0),
            pending: std::collections::BTreeMap::new(),
            retransmitted: std::collections::BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        },
    };
    cnx.remote_connection_id_stashes[stash_idx]
        .connection_ids
        .push(r_cid);
    Ok(())
}
```

## Pair `picoquic/quicctx.c:picoquic_obtain_stashed_cnxid`
C: `picoquic/quicctx.c:3115-3121 picoquic_obtain_stashed_cnxid`
Rust: `rs/fq/src/internal.rs:16309-16316 obtain_stashed_cnxid`

### C body
```c
{
    picoquic_remote_cnxid_stash_t* stash = picoquic_find_or_create_remote_cnxid_stash(cnx, unique_path_id, 0);
    picoquic_remote_cnxid_t* stashed = picoquic_get_cnxid_from_stash(stash);
    
    return stashed;
}
```

### Rust body
```rust
    if let Some(stash) = cnx.remote_connection_id_stashes.first_mut() {
        if stash.connection_ids.is_empty() {
            return Ok(false);
        }
        stash.connection_ids.remove(0);
        return Ok(true);
    }
```

## Pair `picoquic/quicctx.c:picoquic_remove_not_before_cid`
C: `picoquic/quicctx.c:3236-3246 picoquic_remove_not_before_cid`
Rust: `rs/fq/src/internal.rs:5322-5355 remove_not_before_cid`

### C body
```c
{
    uint64_t transport_error = 0;
    picoquic_remote_cnxid_stash_t* cnxid_stash = picoquic_find_or_create_remote_cnxid_stash(cnx, unique_path_id, 0);

    if (cnxid_stash != NULL) {
        transport_error = picoquic_remove_not_before_from_stash(cnx, cnxid_stash, not_before, current_time);
    }

    return transport_error;
}
```

### Rust body
```rust
    ) -> u64 {
        // C: picoquic_remove_not_before_cid
        let eff_path_id = if self.is_multipath_enabled {
            unique_path_id
        } else {
            0
        };
        if let Some(stash_idx) = self
            .remote_connection_id_stashes
            .iter()
            .position(|s| s.unique_path_id == eff_path_id)
        {
            // Temporarily extract the stash to avoid borrow conflicts.
            let mut stash = std::mem::replace(
                &mut self.remote_connection_id_stashes[stash_idx],
                RemoteConnectionIdStash {
                    unique_path_id: eff_path_id,
                    retire_connection_id_before: 0,
                    connection_ids: Vec::new(),
                    is_in_use: false,
                },
            );
            let removed = self.remove_not_before_from_stash(&mut stash, not_before, current_time);
            self.remote_connection_id_stashes[stash_idx] = stash;
            removed
        } else {
            0
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_stream_data_node_recycle`
C: `picoquic/quicctx.c:3357-3368 picoquic_stream_data_node_recycle`
Rust: `rs/fq/src/internal.rs:12944-12962 stream_data_node_recycle`

### C body
```c
{
    if (stream_data->quic->nb_data_nodes_in_pool < PICOQUIC_MAX_PACKETS_IN_POOL) {
        stream_data->next_stream_data = stream_data->quic->p_first_data_node;
        stream_data->quic->p_first_data_node = stream_data;
        stream_data->quic->nb_data_nodes_in_pool++;
    }
    else {
        stream_data->quic->nb_data_nodes_allocated--;
        free(stream_data);
    }
}
```

### Rust body
```rust
    pub fn stream_data_node_alloc(&mut self) -> Result<StreamDataNode, crate::Error> {
        // Allocate a fresh StreamDataNode.  In C a free-list is maintained;
        // in Rust we simply allocate on the heap.
        Ok(StreamDataNode {
            stream_data_membership: None,
            offset: 0,
            data: [0u8; crate::MAX_PACKET_SIZE],
            length: 0,
        })
    }
```

## Pair `picoquic/quicctx.c:picoquic_first_stream`
C: `picoquic/quicctx.c:3468-3475 picoquic_first_stream`
Rust: `rs/fq/src/internal.rs:9626-9629 first_stream`

### C body
```c
{
#ifdef TOO_CAUTIOUS
    return picoquic_stream_from_node(picosplay_first(&cnx->stream_tree));
#else
    return (picoquic_stream_head_t *)picosplay_first(&cnx->stream_tree);
#endif
}
```

### Rust body
```rust
    pub fn first_stream(&self) -> Option<StreamToken> {
        let st = self.stream_tree.first()?;
        self.stream_tree.get(st).copied()
    }
```

## Pair `picoquic/quicctx.c:picoquic_remove_output_stream`
C: `picoquic/quicctx.c:3564-3585 picoquic_remove_output_stream`
Rust: `rs/fq/src/internal.rs:9599-9609 remove_output_stream`

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

## Pair `picoquic/quicctx.c:picoquic_add_output_streams`
C: `picoquic/quicctx.c:3619-3636 picoquic_add_output_streams`
Rust: `rs/fq/src/internal.rs:9646-9659 add_output_streams`

### C body
```c
{
    uint64_t old_rank = STREAM_RANK_FROM_ID(old_limit);
    uint64_t first_new_id = STREAM_ID_FROM_RANK(old_rank + 1ull, cnx->client_mode, !is_bidir);
    picoquic_stream_head_t* stream = picoquic_find_stream(cnx, first_new_id );

    while (stream) {
        if (stream->stream_id > old_limit) {
            if (stream->stream_id > new_limit) {
                break;
            }
            if (IS_LOCAL_STREAM_ID(stream->stream_id, cnx->client_mode) && IS_BIDIR_STREAM_ID(stream->stream_id) == is_bidir) {
                picoquic_insert_output_stream(cnx, stream);
            }
        }
        stream = picoquic_next_stream(stream);
    }
}
```

### Rust body
```rust
            .filter_map(|s| {
                if s.stream_id > old_limit && s.stream_id <= new_limit {
                    s.stream_tree_membership
                        .and_then(|st| self.stream_tree.get(st).copied())
                } else {
                    None
                }
            })
```

## Pair `picoquic/quicctx.c:picoquic_find_or_create_local_cnxid_list`
C: `picoquic/quicctx.c:3766-3793 picoquic_find_or_create_local_cnxid_list`
Rust: `rs/fq/src/internal.rs:13044-13154 find_or_create_local_connection_id_list`

### C body
```c
{
    picoquic_local_cnxid_list_t* local_cnxid_list = cnx->first_local_cnxid_list;
    picoquic_local_cnxid_list_t** p_previous = &cnx->first_local_cnxid_list;

    while (local_cnxid_list != NULL) {
        if (local_cnxid_list->unique_path_id == unique_path_id) {
            break;
        }
        p_previous = &local_cnxid_list->next_list;
        local_cnxid_list = local_cnxid_list->next_list;
    }

    if (local_cnxid_list == NULL && do_create) {
        local_cnxid_list = (picoquic_local_cnxid_list_t*)malloc(sizeof(picoquic_local_cnxid_list_t));
        if (local_cnxid_list != NULL) {
            memset(local_cnxid_list, 0, sizeof(picoquic_local_cnxid_list_t));
            local_cnxid_list->unique_path_id = unique_path_id;
            *p_previous = local_cnxid_list;
            cnx->nb_local_cnxid_lists++;
            if (unique_path_id >= cnx->next_path_id_in_lists) {
                cnx->next_path_id_in_lists = unique_path_id + 1;
            }
        }
    }

    return local_cnxid_list;
}
```

### Rust body
```rust
impl Connection {
    fn has_local_connection_id_value(&self, connection_id: &ConnectionId) -> bool {
        self.local_connection_id_lists.iter().any(|list| {
            list.connection_ids.iter().any(|&tok| {
                self.local_connection_ids
                    .get(tok)
                    .map(|l| &l.connection_id == connection_id)
                    .unwrap_or(false)
            })
        })
    }

    fn random_local_connection_id(&self) -> crate::Result<ConnectionId> {
        let copy_len = self.local_cid_length as usize;
        let mut bytes = [0u8; crate::CONNECTION_ID_MAX_SIZE];
        fill_system_random(&mut bytes[..copy_len])?;
        ConnectionId::clone_from_slice(&bytes[..copy_len]).ok_or(crate::Error::Generic)
    }

    pub fn create_local_connection_id(
        &mut self,
        unique_path_id: u64,
        suggested_value: Option<&ConnectionId>,
        current_time: Instant,
    ) -> Result<LocalConnectionIdToken, crate::Error> {
        // Find or create the per-path list.
        let list_idx = match self
            .local_connection_id_lists
            .iter()
            .position(|l| l.unique_path_id == unique_path_id)
        {
            Some(i) => i,
            None => {
                self.local_connection_id_lists.push(LocalConnectionIdList {
                    unique_path_id,
                    local_connection_id_sequence_next: 0,
                    local_connection_id_retire_before: 0,
                    local_connection_id_oldest_created: current_time.ticks(),
                    nb_local_connection_id_expired: 0,
                    is_demoted: false,
                    demotion_time: crate::Instant::from_ticks(u64::MAX),
                    connection_ids: Vec::new(),
                });
                self.local_connection_id_lists.len() - 1
            }
        };

        let connection_id = if self.local_cid_length == 0 {
            ConnectionId::default()
        } else {
            let mut selected = None;
            for attempt in 0..32 {
                let candidate = if attempt == 0 {
                    match suggested_value {
                        Some(suggested) => *suggested,
                        None => self.random_local_connection_id()?,
                    }
                } else {
                    self.random_local_connection_id()?
                };
                if !self.has_local_connection_id_value(&candidate) {
                    selected = Some(candidate);
                    break;
                }
            }
            selected.ok_or(crate::Error::Generic)?
        };

        let seq = self.local_connection_id_lists[list_idx].local_connection_id_sequence_next;

        let l_cid = LocalConnectionId {
            connection_by_id_membership: None,
            path_id: unique_path_id,
            sequence: seq,
            create_time: current_time,
            connection_id,
            is_acked: false,
        };

        let token = self
            .local_connection_ids
            .insert(l_cid)
            .map_err(|_| crate::Error::Memory)?;

        self.local_connection_id_lists[list_idx].local_connection_id_sequence_next += 1;
        self.local_connection_id_lists[list_idx]
            .connection_ids
            .push(token);

        if seq == 0 {
            self.local_connection_id_lists[list_idx].local_connection_id_oldest_created =
                current_time.ticks();
            if unique_path_id > self.max_path_id_in_connection_id_lists {
                self.max_path_id_in_connection_id_lists = unique_path_id;
            }
        }

        Ok(token)
    }
}
```

## Pair `picoquic/quicctx.c:picoquic_delete_local_cnxid_lists`
C: `picoquic/quicctx.c:3958-3963 picoquic_delete_local_cnxid_lists`
Rust: `rs/fq/src/internal.rs:13207-13213 delete_local_connection_id_lists`

### C body
```c
{
    while (cnx->first_local_cnxid_list != NULL) {
        picoquic_delete_local_cnxid_list(cnx, cnx->first_local_cnxid_list);
    }
}
```

### Rust body
```rust
        for list in self.local_connection_id_lists.drain(..) {
            for tok in list.connection_ids {
                self.local_connection_ids.remove(tok);
            }
        }
```

## Pair `picoquic/quicctx.c:picoquic_create_cnx_internal`
C: `picoquic/quicctx.c:4039-4348 picoquic_create_cnx_internal`
Rust: `rs/fq/src/internal.rs:3053-3878 create_cnx_internal`

### C body
```c
{
    picoquic_cnx_t* cnx = (picoquic_cnx_t*)malloc(sizeof(picoquic_cnx_t));

    if (cnx != NULL) {
        int ret;
        picoquic_local_cnxid_t* cnxid0;

        memset(cnx, 0, sizeof(picoquic_cnx_t));
        cnx->start_time = start_time;
        cnx->phase_delay = INT64_MAX;
        cnx->client_mode = client_mode;
        if (client_mode) {
            if (picoquic_is_connection_id_null(&initial_cnx_id)) {
                picoquic_create_random_cnx_id(quic, &initial_cnx_id, 8);
            }
        }
        cnx->initial_cnxid = initial_cnx_id;
        cnx->quic = quic;
        cnx->pmtud_policy = quic->default_pmtud_policy;
        /* Create the connection ID number 0 */
        cnxid0 = picoquic_create_local_cnxid(cnx, 0, NULL, start_time);

        /* Initialize path updates and quality updates before creating the first path */
        cnx->are_path_callbacks_enabled = quic->are_path_callbacks_enabled;
        cnx->rtt_update_delta = quic->rtt_update_delta;
        cnx->pacing_rate_update_delta = quic->pacing_rate_update_delta;

        /* Initialize the stream data repeat queue */
        picoquic_queue_data_repeat_init(cnx);

        /* Initialize the connection ID stash */
        ret = picoquic_create_path(cnx, start_time, NULL, addr_to, 0, 0);
        if (ret == 0) {
            /* Should return 0, since this is the first path */
            ret = picoquic_init_cnxid_stash(cnx);
        }

        if (ret != 0 || cnxid0 == NULL) {
            picoquic_delete_cnx(cnx);
            /* free(cnx); */
            cnx = NULL;
        } else {
            cnx->next_wake_time = start_time;
            SET_LAST_WAKE(quic, PICOQUIC_QUICCTX);
            picoquic_insert_cnx_in_list(quic, cnx);
            picoquic_insert_cnx_by_wake_time(quic, cnx);
            /* Do not require verification for default path */
            cnx->path[0]->first_tuple->p_local_cnxid = cnxid0;
            cnx->path[0]->first_tuple->challenge_verified = 1;

            cnx->datagram_priority = cnx->quic->default_datagram_priority;
            cnx->high_priority_stream_id = UINT64_MAX;
            for (int i = 0; i < 4; i++) {
                cnx->next_stream_id[i] = i;
            }
            picoquic_register_path(cnx, cnx->path[0]);
        }
    }

    if (cnx != NULL) {
        memcpy(&cnx->local_parameters, &quic->default_tp, sizeof(picoquic_tp_t));
        /* If the default parameters include preferred address, document it */
        if (cnx->local_parameters.preferred_address.is_defined) {
            /* Create an additional CID -- always for path 0, even if multipath */
            picoquic_local_cnxid_t* cnxid1 = picoquic_create_local_cnxid(cnx, 0, NULL, start_time);
            if (cnxid1 != NULL){
                /* copy the connection ID into the local parameter */
                cnx->local_parameters.preferred_address.connection_id = cnxid1->cnx_id;
                /* Create the reset secret */
                (void)picoquic_create_cnxid_reset_secret(cnx->quic, &cnxid1->cnx_id,
                    cnx->local_parameters.preferred_address.statelessResetToken);
            }
        }

        /* Apply the defined MTU MAX if specified and not set in defaults. */
        if (cnx->local_parameters.max_packet_size == 0 && cnx->quic->mtu_max > 0)
        {
            cnx->local_parameters.max_packet_size = cnx->quic->mtu_max -
                PICOQUIC_MTU_OVERHEAD(addr_to);
        }

        /* If local connection ID size is null, don't allow migration */
        if (!cnx->client_mode && quic->local_cnxid_length == 0) {
            cnx->local_parameters.migration_disabled = 1;
        }

        /* Initialize BDP transport parameter */
        if (quic->default_send_receive_bdp_frame) {
           /* Accept and send BDP extension frame */
            cnx->local_parameters.enable_bdp_frame = 1;
        }
 
        /* Initialize local flow control variables to advertised values */
        cnx->maxdata_local = ((uint64_t)cnx->local_parameters.initial_max_data);
        cnx->max_stream_id_bidir_local = STREAM_ID_FROM_RANK(
            cnx->local_parameters.initial_max_stream_id_bidir, cnx->client_mode, 0);
        cnx->max_stream_id_bidir_local_computed = STREAM_TYPE_FROM_ID(cnx->max_stream_id_bidir_local);
        cnx->max_stream_id_unidir_local = STREAM_ID_FROM_RANK(
            cnx->local_parameters.initial_max_stream_id_unidir, cnx->client_mode, 1);
        cnx->max_stream_id_unidir_local_computed = STREAM_TYPE_FROM_ID(cnx->max_stream_id_unidir_local);
       
        /* Initialize padding policy to default for context */
        cnx->padding_multiple = quic->padding_multiple_default;
        cnx->padding_minsize = quic->padding_minsize_default;

        /* Initialize spin policy, ensure that at least 1/8th of connections do not spin */
        cnx->spin_policy = quic->default_spin_policy;
        if (cnx->spin_policy == picoquic_spinbit_basic) {
            uint8_t rand256 = (uint8_t)picoquic_public_random_64();
            if (rand256 < PICOQUIC_SPIN_RESERVE_MOD_256) {
                cnx->spin_policy = picoquic_spinbit_null;
            }
        }
        else if (cnx->spin_policy == picoquic_spinbit_on) {
            /* Option used in test to avoid randomizing spin bit on/off */
            cnx->spin_policy = picoquic_spinbit_basic;
        }

        if (sni != NULL) {
            cnx->sni = picoquic_string_duplicate(sni);
        }

        if (alpn != NULL) {
            cnx->alpn = picoquic_string_duplicate(alpn);
        }

        cnx->callback_fn = quic->default_callback_fn;
        cnx->callback_ctx = quic->default_callback_ctx;
        cnx->congestion_alg = quic->default_congestion_alg;
        cnx->is_preemptive_repeat_enabled = quic->is_preemptive_repeat_enabled;

        /* Initialize key rotation interval to default value */
        cnx->crypto_epoch_length_max = quic->crypto_epoch_length_max;

        for (int epoch = 0; epoch < PICOQUIC_NUMBER_OF_EPOCHS; epoch++) {
            cnx->tls_stream[epoch].send_queue = NULL;
        }

        /* Perform different initializations for clients and servers */
        if (cnx->client_mode) {
            if (preferred_version == 0) {
                cnx->proposed_version = picoquic_supported_versions[0].version;
                cnx->version_index = 0;
            } else {
                cnx->version_index = picoquic_get_version_index(preferred_version);
                if (cnx->version_index < 0) {
                    cnx->version_index = PICOQUIC_INTEROP_VERSION_INDEX;
                    if ((preferred_version & 0x0A0A0A0A) == 0x0A0A0A0A) {
                        /* This is a hack, to allow greasing the cnx ID */
                        cnx->proposed_version = preferred_version;

                    } else {
                        cnx->proposed_version = picoquic_supported_versions[PICOQUIC_INTEROP_VERSION_INDEX].version;
                    }
                } else {
                    cnx->proposed_version = preferred_version;
                }
            }

            cnx->cnx_state = picoquic_state_client_init;

            if (!quic->is_cert_store_not_empty) {
                /* The open SSL certifier always fails if no certificate is stored, so we just use a NULL verifier */
                picoquic_log_app_message(cnx, "No root crt list specified -- certificate will not be verified.\n");

                picoquic_set_null_verifier(quic);
            }
        } else {
            cnx->is_half_open = 1;
            cnx->quic->current_number_half_open += 1;
            if (cnx->quic->current_number_half_open > cnx->quic->max_half_open_before_retry) {
                cnx->quic->check_token = 1;
            }
            cnx->cnx_state = picoquic_state_server_init;
            cnx->initial_cnxid = initial_cnx_id;
            cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id = remote_cnx_id;

            cnx->version_index = picoquic_get_version_index(preferred_version);
            if (cnx->version_index < 0) {
                /* TODO: this is an internal error condition, should not happen */
                cnx->version_index = 0;
                cnx->proposed_version = picoquic_supported_versions[0].version;
            } else {
                cnx->proposed_version = preferred_version;
            }
        }

        for (picoquic_packet_context_enum pc = 0;
            pc < picoquic_nb_packet_context; pc++) {
            picoquic_init_ack_ctx(cnx, &cnx->ack_ctx[pc]);
            picoquic_init_packet_ctx(cnx, &cnx->pkt_ctx[pc], pc);
        }
        /* Initialize the ACK behavior. By default, picoquic abides with the recommendation to send
         * ACK immediately if packets are received out of order (ack_ignore_order_remote = 0),
         * but this behavior creates too many ACKS on high speed links, so picoquic will request
         * the peer to not do that if the "delayed ACK" extension is available (ack_ignore_order_local = 1)
         */
        cnx->ack_ignore_order_local = 1;
        cnx->ack_ignore_order_remote = 0;

        cnx->latest_progress_time = start_time;
        cnx->latest_receive_time = start_time;

        for (int epoch = 0; epoch < PICOQUIC_NUMBER_OF_EPOCHS; epoch++) {
            cnx->tls_stream[epoch].stream_id = 0;
            cnx->tls_stream[epoch].consumed_offset = 0;
            cnx->tls_stream[epoch].fin_offset = 0;
            cnx->tls_stream[epoch].stream_node.left = NULL;
            cnx->tls_stream[epoch].stream_node.parent = NULL;
            cnx->tls_stream[epoch].stream_node.right = NULL;
            cnx->tls_stream[epoch].sent_offset = 0;
            cnx->tls_stream[epoch].local_error = 0;
            cnx->tls_stream[epoch].remote_error = 0;
            cnx->tls_stream[epoch].maxdata_local = UINT64_MAX;
            cnx->tls_stream[epoch].maxdata_remote = UINT64_MAX;

            picosplay_init_tree(&cnx->tls_stream[epoch].stream_data_tree, picoquic_stream_data_node_compare, picoquic_stream_data_node_create, picoquic_stream_data_node_delete, picoquic_stream_data_node_value);
            picoquic_sack_list_init(&cnx->tls_stream[epoch].sack_list);
            /* No need to reset the state flags, as they are not used for the crypto stream */
        }
        
        cnx->ack_frequency_sequence_local = UINT64_MAX;
        cnx->ack_gap_local = 2;
        cnx->ack_frequency_delay_local = PICOQUIC_ACK_DELAY_MAX_DEFAULT;
        cnx->ack_frequency_sequence_remote = UINT64_MAX;
        cnx->ack_gap_remote = 2;
        cnx->ack_delay_remote = PICOQUIC_ACK_DELAY_MIN;
        cnx->max_ack_delay_remote = cnx->ack_delay_remote;
        cnx->max_ack_gap_remote = cnx->ack_gap_remote;
        cnx->max_ack_delay_local = cnx->ack_frequency_delay_local;
        cnx->max_ack_gap_local = cnx->ack_gap_local;
        cnx->min_ack_delay_remote = cnx->ack_delay_remote;
        cnx->min_ack_delay_local = cnx->ack_frequency_delay_local;


        picosplay_init_tree(&cnx->stream_tree, picoquic_stream_node_compare, picoquic_stream_node_create, picoquic_stream_node_delete, picoquic_stream_node_value);

        cnx->congestion_alg = cnx->quic->default_congestion_alg;
        cnx->congestion_alg_option_string = cnx->quic->default_congestion_alg_option_string;
        if (cnx->congestion_alg != NULL) {
            cnx->congestion_alg->alg_init(cnx->path[0], cnx->congestion_alg_option_string, start_time);
        }
    }

    /* Only initialize TLS after all parameters have been set */
    if (cnx != NULL && picoquic_tlscontext_create(quic, cnx) != 0) {
        /* Cannot just do partial creation! */
        picoquic_delete_cnx(cnx);
        cnx = NULL;
    }

    if (cnx != NULL) {
        if (initial_aead_dec != NULL && initial_pn_dec != NULL) {
            cnx->crypto_context[picoquic_epoch_initial].aead_decrypt = initial_aead_dec;
            cnx->crypto_context[picoquic_epoch_initial].pn_dec = initial_pn_dec;
            if (picoquic_get_initial_aead_context(quic, cnx->version_index, &cnx->initial_cnxid,
                cnx->client_mode, 1 /* encoding mode */,
                &cnx->crypto_context[picoquic_epoch_initial].aead_encrypt,
                &cnx->crypto_context[picoquic_epoch_initial].pn_enc) != 0) {
                /* Cannot initialize aead encrypt for initial packets */
                /* Make sure that we do not delete the already allocated
                * initial_aead_dec and initial_pn_dec when clearing the
                * connection, so as not to mess the application management
                * of memory.
                 */
                cnx->crypto_context[picoquic_epoch_initial].aead_decrypt = NULL;
                cnx->crypto_context[picoquic_epoch_initial].pn_dec = NULL;
                picoquic_delete_cnx(cnx);
                cnx = NULL;
            }
        }
        else if (picoquic_setup_initial_traffic_keys(cnx)) {
            /* Cannot initialize aead for initial packets */
            picoquic_delete_cnx(cnx);
            cnx = NULL;
        }
    }

    if (cnx != NULL && !client_mode && quic->local_cnxid_length > 0) {
        if (picoquic_register_net_icid(cnx) != 0) {
            DBG_PRINTF("%s", "Could not register the ICID in table.\n"); 
            if (initial_aead_dec != NULL && initial_pn_dec != NULL) {
                /* Make sure that we do not delete the already allocated
                * initial_aead_dec and initial_pn_dec when clearing the
                * connection, so as not to mess the application management
                * of memory.
                 */
                cnx->crypto_context[picoquic_epoch_initial].aead_decrypt = NULL;
                cnx->crypto_context[picoquic_epoch_initial].pn_dec = NULL;
            }
            picoquic_delete_cnx(cnx);
            cnx = NULL;
        }
    }

    if (quic->use_unique_log_names) {
        picoquic_crypto_random(quic, &cnx->log_unique, sizeof(cnx->log_unique));
    }

    if (cnx != NULL && !cnx->client_mode) {
        picoquic_log_new_connection(cnx);
    }

    return cnx;
}
```

### Rust body
```rust
    ) -> Result<ConnectionToken, crate::Error> {
        use core::net::{IpAddr, Ipv4Addr};

        let zero_instant = crate::Instant::from_ticks(0);
        let zero_dur = crate::Duration::from_ticks(0);

        if client_mode && initial_cnx_id.is_empty() {
            initial_cnx_id = crate::create_random_cnx_id(self, 8);
        }

        let supported_version_index = |version: u32| -> Option<i32> {
            SUPPORTED_VERSIONS
                .iter()
                .position(|v| *v as u32 == version)
                .map(|i| i as i32)
        };
        let interop_index = SUPPORTED_VERSIONS
            .iter()
            .position(|v| *v == INTEROP_VERSION_LATEST)
            .unwrap_or(0) as i32;
        let (version_index, proposed_version) = if client_mode {
            if preferred_version == 0 {
                (0, SUPPORTED_VERSIONS[0] as u32)
            } else if let Some(idx) = supported_version_index(preferred_version) {
                (idx, preferred_version)
            } else if (preferred_version & 0x0A0A0A0A) == 0x0A0A0A0A {
                (interop_index, preferred_version)
            } else {
                (interop_index, INTEROP_VERSION_LATEST as u32)
            }
        } else if let Some(idx) = supported_version_index(preferred_version) {
            (idx, preferred_version)
        } else {
            (0, SUPPORTED_VERSIONS[0] as u32)
        };

        // Determine connection state.
        let connection_state = if client_mode {
            crate::State::ClientInit
        } else {
            crate::State::ServerInit
        };

        // Build a null path/tuple for the initial path.
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let peer_addr = addr_to.copied().unwrap_or(default_addr);
        let mut local_connection_ids = Arena::new();
        let initial_lcid_token = local_connection_ids.insert(LocalConnectionId {
            connection_by_id_membership: None,
            path_id: 0,
            sequence: 0,
            create_time: start_time,
            connection_id: initial_cnx_id,
            is_acked: false,
        })?;
        let initial_tuple = Tuple {
            unique_path_id: 0,
            peer_addr,
            local_addr: default_addr,
            if_index: 0,
            observed_addr: default_addr,
            remote_connection_id_index: None,
            local_connection_id: Some(initial_lcid_token),
            nb_observed_repeat: 0,
            observed_time: zero_instant,
            challenge_response: 0,
            challenge: [0u64; CHALLENGE_REPEAT_MAX],
            challenge_time: zero_instant,
            demotion_time: zero_instant,
            challenge_time_first: zero_instant,
            is_nat_rebinding: 0,
            challenge_repeat_count: 0,
            is_backup: 0,
            challenge_required: false,
            challenge_verified: true, // C: path[0] first_tuple verified
            challenge_failed: false,
            response_required: false,
            to_preferred_address: false,
        };
        let initial_path = Path {
            registered_peer_addr: peer_addr,
            connection_by_net_membership: None,
            unique_path_id: 0,
            app_path_ctx: None,
            ack_ctx: AckContext {
                sack_list: SackList::new(),
                time_stamp_largest_received: crate::Instant::from_ticks(u64::MAX),
                act: [
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start_time,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start_time,
                        time_oldest_unack_packet_received: zero_instant,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                ],
                crypto_rotation_sequence: 0,
                ecn_ect0_total_local: 0,
                ecn_ect1_total_local: 0,
                ecn_ce_total_local: 0,
                sending_ecn_ack: false,
            },
            pkt_ctx: PacketContextState {
                send_sequence: 0,
                next_sequence_hole: 0,
                retransmit_sequence: 0,
                highest_acknowledged: 0u64.wrapping_sub(1),
                latest_time_acknowledged: start_time,
                highest_acknowledged_time: start_time,
                pending: BTreeMap::new(),
                retransmitted: BTreeMap::new(),
                preemptive_repeat_seq: None,
                retransmitted_queue_size: 0,
                ecn_ect0_total_remote: 0,
                ecn_ect1_total_remote: 0,
                ecn_ce_total_remote: 0,
                ack_of_ack_requested: false,
            },
            tuples: vec![initial_tuple],
            observed_address_received: 0,
            observed_sequence_sent: 0,
            observed_addr_acked: false,
            last_non_path_probing_pn: 0,
            demotion_time: zero_instant,
            last_sent_time: zero_instant,
            status_sequence_to_receive_next: 0,
            status_sequence_sent_last: 0,
            mtu_probe_sent: false,
            path_is_published: false,
            path_is_backup: false,
            path_is_demoted: false,
            path_abandon_received: false,
            path_abandon_sent: false,
            current_spin: false,
            last_bw_estimate_path_limited: false,
            path_cid_rotated: false,
            is_nat_challenge: false,
            is_cc_data_updated: false,
            is_multipath_probe_needed: false,
            is_ssthresh_initialized: false,
            is_token_published: false,
            is_ticket_seeded: false,
            is_bdp_sent: false,
            is_nominal_ack_path: false,
            is_ack_lost: false,
            is_ack_expected: false,
            is_datagram_ready: false,
            is_pto_required: false,
            is_probing_nat: false,
            is_lost_feedback_notified: false,
            is_cca_probing_up: false,
            rtt_is_initialized: false,
            sending_path_cid_blocked_frame: false,
            last_packet_received_at: zero_instant,
            last_loss_event_detected: zero_instant,
            nb_retransmit: 0,
            total_bytes_lost: 0,
            nb_losses_found: 0,
            nb_timer_losses: 0,
            nb_spurious: 0,
            nb_losses_reported: 0,
            q_square: 0,
            max_ack_delay: ACK_DELAY_MAX_DEFAULT,
            rtt_sample: zero_dur,
            one_way_delay_sample: zero_dur,
            smoothed_rtt: INITIAL_RTT,
            rtt_variant: zero_dur,
            retransmit_timer: INITIAL_RETRANSMIT_TIMER,
            rtt_min: INITIAL_RTT,
            rtt_max: zero_dur,
            max_spurious_rtt: zero_dur,
            max_reorder_delay: zero_dur,
            max_reorder_gap: 0,
            latest_sent_time: zero_instant,
            rtt_packet_previous_period: zero_dur,
            rtt_time_previous_period: zero_dur,
            nb_rtt_estimate_in_period: 0,
            sum_rtt_estimate_in_period: zero_dur,
            max_rtt_estimate_in_period: zero_dur,
            min_rtt_estimate_in_period: zero_dur,
            send_mtu: ENFORCED_INITIAL_MTU,
            send_mtu_max_tried: 0,
            delivered: 0,
            delivered_last: 0,
            delivered_time_last: zero_instant,
            delivered_sent_last: zero_instant.ticks(),
            delivered_limited_index: 0,
            delivered_last_packet: 0,
            bandwidth_estimate: 0,
            bandwidth_estimate_max: 0,
            max_sample_acked_time: zero_instant,
            max_sample_sent_time: zero_instant,
            max_sample_delivered: 0,
            peak_bandwidth_estimate: 0,
            bytes_sent: 0,
            received: 0,
            receive_rate_epoch: 0,
            received_prior: 0,
            receive_rate_estimate: 0,
            receive_rate_max: 0,
            cwin: CWIN_INITIAL,
            bytes_in_transit: 0,
            last_sender_limited_time: zero_instant,
            last_cwin_blocked_time: zero_instant,
            last_time_acked_data_frame_sent: zero_instant,
            congestion_alg_state: None,
            pacing: Pacing {
                rate: 0,
                evaluation_time: zero_instant,
                bucket_max: 0,
                packet_time_microsec: zero_dur,
                quantum_max: 0,
                rate_max: 0,
                bandwidth_pause: 0,
                bucket_nanosec: 0,
                packet_time_nanosec: 0,
            },
            nb_mtu_losses: 0,
            lost_after_delivered: 0,
            responder: 0,
            challenger: 0,
            polled: 0,
            paced: 0,
            congested: 0,
            selected: 0,
            nb_delay_outliers: 0,
            rtt_update_delta: self.rtt_update_delta,
            pacing_rate_update_delta: self.pacing_rate_update_delta,
            rtt_threshold_low: zero_dur,
            rtt_threshold_high: zero_dur,
            pacing_rate_threshold_low: 0,
            pacing_rate_threshold_high: 0,
            receive_rate_threshold_low: 0,
            receive_rate_threshold_high: 0,
            rtt_min_remote: zero_dur,
            cwin_remote: 0,
            ip_client_remote: [0u8; 16],
            ip_client_remote_length: 0,
        };

        // Build the initial CID list for path 0.
        let initial_cid_list = LocalConnectionIdList {
            unique_path_id: 0,
            local_connection_id_sequence_next: 1,
            local_connection_id_retire_before: 0,
            local_connection_id_oldest_created: start_time.ticks(),
            nb_local_connection_id_expired: 0,
            is_demoted: false,
            demotion_time: zero_instant,
            connection_ids: vec![initial_lcid_token],
        };

        // Build the initial remote CID stash for path 0.
        let zero_pkt_ctx = PacketContextState {
            send_sequence: 0,
            next_sequence_hole: 0,
            retransmit_sequence: 0,
            highest_acknowledged: 0u64.wrapping_sub(1),
            latest_time_acknowledged: start_time,
            highest_acknowledged_time: start_time,
            pending: BTreeMap::new(),
            retransmitted: BTreeMap::new(),
            preemptive_repeat_seq: None,
            retransmitted_queue_size: 0,
            ecn_ect0_total_remote: 0,
            ecn_ect1_total_remote: 0,
            ecn_ce_total_remote: 0,
            ack_of_ack_requested: false,
        };
        let initial_remote_cid = RemoteConnectionId {
            sequence: 0,
            connection_id: remote_cnx_id,
            reset_secret: [0u8; RESET_SECRET_SIZE],
            nb_path_references: 1,
            needs_removal: false,
            retire_sent: false,
            retire_acked: false,
            pkt_ctx: zero_pkt_ctx,
        };
        let initial_stash = RemoteConnectionIdStash {
            unique_path_id: 0,
            retire_connection_id_before: 0,
            connection_ids: vec![initial_remote_cid],
            is_in_use: true,
        };

        // Build the null AckContext for connection-level use.
        fn make_ack_ctx(start: crate::Instant) -> AckContext {
            let zi = crate::Instant::from_ticks(0);
            AckContext {
                sack_list: SackList::new(),
                time_stamp_largest_received: crate::Instant::from_ticks(u64::MAX),
                act: [
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start,
                        time_oldest_unack_packet_received: zi,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                    AckContextTrack {
                        highest_ack_sent: 0,
                        highest_ack_sent_time: start,
                        time_oldest_unack_packet_received: zi,
                        ack_needed: false,
                        ack_after_fin: false,
                        out_of_order_received: false,
                        is_immediate_ack_required: false,
                    },
                ],
                crypto_rotation_sequence: 0,
                ecn_ect0_total_local: 0,
                ecn_ect1_total_local: 0,
                ecn_ce_total_local: 0,
                sending_ecn_ack: false,
            }
        }
        fn make_pkt_ctx(start: crate::Instant) -> PacketContextState {
            PacketContextState {
                send_sequence: 0,
                next_sequence_hole: 0,
                retransmit_sequence: 0,
                highest_acknowledged: 0u64.wrapping_sub(1),
                latest_time_acknowledged: start,
                highest_acknowledged_time: start,
                pending: BTreeMap::new(),
                retransmitted: BTreeMap::new(),
                preemptive_repeat_seq: None,
                retransmitted_queue_size: 0,
                ecn_ect0_total_remote: 0,
                ecn_ect1_total_remote: 0,
                ecn_ce_total_remote: 0,
                ack_of_ack_requested: false,
            }
        }
        fn make_tls_stream(_start: crate::Instant) -> StreamHead {
            StreamHead {
                stream_tree_membership: None,
                stream_id: 0,
                affinity_path: None,
                consumed_offset: 0,
                fin_offset: 0,
                reset_offset: 0,
                maxdata_local: u64::MAX,
                maxdata_local_acked: 0,
                maxdata_remote: u64::MAX,
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
            }
        }
        let _ = start_time; // used in make_*
        let tls_streams = core::array::from_fn::<StreamHead, NUMBER_OF_EPOCHS, _>(|_| {
            make_tls_stream(start_time)
        });
        let crypto_contexts =
            core::array::from_fn::<CryptoContext, NUMBER_OF_EPOCHS, _>(|_| CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            });

        let mut local_params = self.default_tp.clone();
        if local_params.max_packet_size == 0 && self.mtu_max > 0 {
            local_params.max_packet_size =
                self.mtu_max.saturating_sub(crate::mtu_overhead(&peer_addr));
        }
        if !client_mode && self.local_connection_id_length == 0 {
            local_params.migration_disabled = true;
        }
        if self.default_send_receive_bdp_frame {
            local_params.enable_bdp_frame = true;
        }
        let maxdata_local = local_params.initial_max_data;
        // C: STREAM_ID_FROM_RANK(rank, client_mode, unidir)
        //  = 4*rank + (client_mode ? 0 : 1) + (unidir ? 2 : 0)
        let role_bit = if client_mode { 0u64 } else { 1u64 };
        let max_stream_id_bidir_local = 4 * local_params.initial_max_stream_id_bidir + role_bit;
        let max_stream_id_unidir_local =
            4 * local_params.initial_max_stream_id_unidir + role_bit + 2;

        let mut spin_policy = self.default_spin_policy;
        if spin_policy == SpinbitVersion::Basic {
            let rand256 = crate::public_random_64() as u8;
            if rand256 < SPIN_RESERVE_MOD_256 {
                spin_policy = SpinbitVersion::Null;
            }
        } else if spin_policy == SpinbitVersion::On {
            spin_policy = SpinbitVersion::Basic;
        }

        let mut cnx = Connection {
            proposed_version,
            rejected_version: 0,
            desired_version: 0,
            version_index,

            is_0rtt_accepted: false,
            remote_parameters_received: false,
            client_mode,
            key_phase_enc: false,
            key_phase_dec: false,
            zero_rtt_data_accepted: false,
            sending_ecn_ack: false,
            sent_blocked_frame: false,
            stream_blocked_bidir_sent: false,
            stream_blocked_unidir_sent: false,
            max_stream_data_needed: false,
            path_demotion_needed: false,
            tuple_demotion_needed: false,
            alt_path_challenge_needed: false,
            is_handshake_finished: false,
            is_handshake_done_acked: false,
            is_new_token_acked: false,
            is_1rtt_received: false,
            is_1rtt_acked: false,
            has_successful_probe: false,
            grease_transport_parameters: false,
            test_large_chello: false,
            initial_validated: false,
            initial_repeat_needed: false,
            is_loss_bit_enabled_incoming: false,
            is_loss_bit_enabled_outgoing: false,
            is_ack_frequency_negotiated: false,
            is_ack_frequency_updated: false,
            recycle_sooner_needed: false,
            is_time_stamp_enabled: false,
            is_time_stamp_sent: false,
            is_pacing_update_requested: false,
            is_path_quality_update_requested: false,
            is_hcid_verified: false,
            do_grease_quic_bit: false,
            quic_bit_greased: false,
            quic_bit_received_0: false,
            is_half_open: !client_mode,
            did_receive_short_initial: false,
            ack_ignore_order_local: true,
            ack_ignore_order_remote: false,
            are_path_callbacks_enabled: self.are_path_callbacks_enabled,
            is_sending_large_buffer: false,
            is_preemptive_repeat_enabled: self.is_preemptive_repeat_enabled,
            do_version_negotiation: false,
            send_receive_bdp_frame: false,
            cwin_notified_from_seed: false,
            is_datagram_ready: false,
            is_immediate_ack_required: false,
            is_multipath_enabled: false,
            is_lost_feedback_notification_required: false,
            is_forced_probe_up_required: false,
            is_address_discovery_provider: false,
            is_address_discovery_receiver: false,
            is_subscribed_to_path_allowed: false,
            is_notified_that_path_is_allowed: false,
            is_reset_stream_at_enabled: false,

            pmtud_policy: self.default_pmtud_policy,
            spin_policy,
            idle_timeout: crate::Duration::from_ticks(0),
            local_parameters: local_params,
            remote_parameters: crate::tp::TransportParameters::default(),
            padding_multiple: self.padding_multiple_default,
            padding_minsize: self.padding_minsize_default,
            seed_ip_addr: None,
            seed_rtt_min: zero_dur,
            seed_cwin: 0,

            issued_ticket_id: 0,
            resumed_ticket_id: 0,

            sni: sni.map(|s| s.to_owned()),
            alpn: alpn.map(|s| s.to_owned()),
            alpn_proposals: Vec::new(),
            max_early_data_size: 0,

            callback_fn: None, // C: = quic->default_callback_fn (not clonable)
            callback_ctx: None,

            connection_state,
            initial_connection_id: initial_cnx_id,
            original_connection_id: initial_cnx_id,
            registered_icid_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            connection_by_icid_membership: None,
            registered_secret_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            registered_reset_secret: [0u8; RESET_SECRET_SIZE],
            connection_by_secret_membership: None,

            local_cid_length: self.local_connection_id_length,
            local_connection_id_ttl: self.local_connection_id_ttl,
            random_initial: self.random_initial,

            start_time,
            phase_delay: i64::MAX,
            application_error: 0,
            local_error: 0,
            local_error_reason: None,
            remote_application_error: 0,
            remote_error: 0,
            offending_frame_type: 0,
            remote_error_reason: None,
            retry_token: Vec::new(),

            next_wake_time: start_time,
            connection_wake_membership: None,
            app_wake_time: crate::Instant::from_ticks(u64::MAX),

            tls_ctx: None,
            crypto_epoch_length_max: self.crypto_epoch_length_max,
            crypto_epoch_sequence: 0,
            crypto_rotation_time_guard: zero_instant,
            tls_sendbuf: Vec::new(),
            psk_cipher_suite_id: 0,
            ech_client_config: None,

            tls_stream: tls_streams,
            crypto_context: crypto_contexts,
            crypto_context_old: CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            },
            crypto_context_new: CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            },
            app_secret_enc: [0u8; 64],
            app_secret_dec: [0u8; 64],
            app_secret_len: 32,
            crypto_failure_count: 0,

            latest_progress_time: start_time,
            latest_receive_time: start_time,
            last_close_sent: zero_instant,
            pkt_ctx: core::array::from_fn(|_| make_pkt_ctx(start_time)),
            ack_ctx: core::array::from_fn(|_| make_ack_ctx(start_time)),
            observed_number: 0,

            nb_bytes_queued: 0,
            nb_zero_rtt_sent: 0,
            nb_zero_rtt_acked: 0,
            nb_zero_rtt_received: 0,
            max_mtu_sent: 0,
            max_mtu_received: 0,
            nb_packets_received: 0,
            nb_trains_sent: 0,
            nb_trains_short: 0,
            nb_trains_blocked_cwin: 0,
            nb_trains_blocked_pacing: 0,
            nb_trains_blocked_others: 0,
            nb_packets_sent: 0,
            nb_packets_logged: 0,
            use_long_log: self.use_long_log,
            nb_retransmission_total: 0,
            nb_preemptive_repeat: 0,
            nb_spurious: 0,
            nb_crypto_key_rotations: 0,
            nb_packet_holes_inserted: 0,
            max_ack_delay_remote: ACK_DELAY_MAX,
            max_ack_gap_remote: 2,
            max_ack_delay_local: ACK_DELAY_MAX_DEFAULT,
            max_ack_gap_local: 2,
            min_ack_delay_remote: ACK_DELAY_MAX,
            min_ack_delay_local: ACK_DELAY_MAX_DEFAULT,
            cwin_blocked: false,
            flow_blocked: false,
            stream_blocked: false,

            congestion_alg: self.default_congestion_alg,
            congestion_alg_option_string: None,

            rtt_update_delta: self.rtt_update_delta,
            pacing_rate_update_delta: self.pacing_rate_update_delta,
            pacing_rate_signalled: 0,
            pacing_increase_threshold: 0,
            pacing_decrease_threshold: 0,
            pacing_change_threshold: 0,

            initial_data_received: 0,
            initial_data_sent: 0,

            data_sent: 0,
            data_received: 0,
            offset_received: 0,
            maxdata_local,
            maxdata_local_acked: 0,
            maxdata_remote: 0,
            max_stream_data_local: 0,
            max_stream_data_remote: 0,
            max_stream_id_bidir_local,
            max_stream_id_bidir_rank_acked: 0,
            max_stream_id_bidir_local_computed: 0,
            max_stream_id_bidir_remote: 0,
            max_stream_id_unidir_local,
            max_stream_id_unidir_rank_acked: 0,
            max_stream_id_unidir_local_computed: 0,
            max_stream_id_unidir_remote: 0,

            misc_frames: std::collections::VecDeque::new(),

            stream_tree: crate::splay::SplayTree::default(),
            streams: crate::arena::Arena::new(),
            output_streams: std::collections::VecDeque::new(),
            high_priority_stream_id: u64::MAX,
            next_stream_id: [0, 1, 2, 3],
            priority_limit_for_bypass: 0,

            queue_data_repeat_tree: crate::splay::SplayTree::default(),
            queued_packets: crate::arena::Arena::new(),

            datagrams: std::collections::VecDeque::new(),
            datagram_priority: self.default_datagram_priority as u64,
            datagram_conflicts_count: 0,
            datagram_conflicts_max: 0,

            keep_alive_interval: zero_dur,

            paths: vec![initial_path],
            last_path_polled: 0,
            unique_path_id_next: 1,
            nominal_path_for_ack: None,
            status_sequence_to_send_next: 0,
            max_path_id_local: 0,
            max_path_id_acknowledged: 0,
            max_path_id_remote: 0,
            paths_blocked_acknowledged: 0,

            remote_connection_id_stashes: vec![initial_stash],

            next_path_id_in_lists: 1,
            max_path_id_in_connection_id_lists: 0,
            local_connection_id_lists: vec![initial_cid_list],
            local_connection_ids,

            ack_frequency_sequence_local: u64::MAX,
            ack_gap_local: 2,
            ack_frequency_delay_local: ACK_DELAY_MAX_DEFAULT,
            ack_frequency_sequence_remote: u64::MAX,
            ack_gap_remote: 2,
            ack_delay_remote: ACK_DELAY_MAX,
            ack_reordering_threshold_remote: 0,

            sooner_stateless: std::collections::VecDeque::new(),

            log_unique: 0,
            f_binlog: None,
            binlog_file_name: None,
            text_log_fns: self.text_log_fns.clone(),
            bin_log_fns: self.bin_log_fns.clone(),
            qlog_fns: self.qlog_fns.clone(),
            memlog_call_back: None,
            memlog_ctx: None,
            qlog_ctx: None,
            own_token: None,
            quic_ptr: std::ptr::null_mut(),
        };
        cnx.create_tls_context(self)?;

        // Insert into the connection arena.
        let token = self
            .connections
            .insert(cnx)
            .map_err(|_| crate::Error::Memory)?;
        let quic_ptr = self as *mut Quic;
        if let Some(cnx) = self.connections.get_mut(token) {
            cnx.own_token = Some(token);
            cnx.quic_ptr = quic_ptr;
            cnx.setup_initial_traffic_keys()?;
            if let Some(alg) = cnx.congestion_alg {
                let option = cnx.congestion_alg_option_string.as_deref();
                alg.algorithm
                    .alg_init(&mut cnx.paths[0], option, start_time);
            }
        }

        // Update half-open count for server connections.
        if !client_mode {
            self.current_number_half_open += 1;
            if self.current_number_half_open > self.max_half_open_before_retry {
                self.check_token = true;
            }
        }
        self.insert_cnx_in_list(token);

        {
            let cid = self
                .connections
                .get(token)
                .map(|c| c.initial_connection_id)
                .unwrap_or(initial_cnx_id);
            if !cid.is_empty() {
                if self.connection_by_id.lookup(&cid).is_some() {
                    self.delete_connection(token);
                    return Err(crate::Error::Generic);
                }
                let (membership, _) = self.connection_by_id.insert(cid, token)?;
                if let Some(cnx) = self.connections.get_mut(token)
                    && let Some(l_cid) = cnx.local_connection_ids.get_mut(initial_lcid_token)
                {
                    l_cid.connection_by_id_membership = Some(membership);
                }
            }
        }

        let has_preferred_address = self
            .connections
            .get(token)
            .map(|cnx| {
                cnx.local_parameters.preferred_address.v4.is_some()
                    || cnx.local_parameters.preferred_address.v6.is_some()
            })
            .unwrap_or(false);
        if has_preferred_address {
            let cid_token = self.create_local_cnxid(token, 0, None, start_time)?;
            let preferred_cid = self
                .connections
                .get(token)
                .and_then(|cnx| cnx.local_connection_ids.get(cid_token))
                .map(|local_cid| local_cid.connection_id)
                .ok_or(crate::Error::Generic)?;
            let mut reset_token = [0u8; crate::RESET_SECRET_SIZE];
            self.create_connection_id_reset_secret(&preferred_cid, &mut reset_token)?;
            if let Some(cnx) = self.connections.get_mut(token) {
                cnx.local_parameters.preferred_address.connection_id = preferred_cid;
                cnx.local_parameters.preferred_address.stateless_reset_token = reset_token;
            }
        }

        if let Some(cnx) = self.connections.get_mut(token)
            && let Some(path) = cnx.paths.first_mut()
        {
            path.path_is_published = true;
        }

        if self.local_connection_id_length == 0 {
            let should_register = self
                .connections
                .get(token)
                .and_then(|cnx| cnx.paths.first())
                .and_then(|path| path.tuples.first())
                .map(|tuple| !crate::socket_addr_is_unspecified(&tuple.peer_addr))
                .unwrap_or(false);
            if should_register && self.register_net_id(token, 0).is_err() {
                self.delete_connection(token);
                return Err(crate::Error::Generic);
            }
        }

        if !client_mode
            && self.local_connection_id_length > 0
            && self.register_net_icid(token).is_err()
        {
            self.delete_connection(token);
            return Err(crate::Error::Generic);
        }

        if self.use_unique_log_names
            && let Some(cnx) = self.connections.get_mut(token)
        {
            let mut bytes = [0u8; 2];
            rand_core::RngCore::fill_bytes(&mut *self.rng, &mut bytes);
            cnx.log_unique = u16::from_le_bytes(bytes);
        }

        Ok(token)
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_transport_parameters`
C: `picoquic/quicctx.c:4415-4433 picoquic_set_transport_parameters`
Rust: `rs/fq/src/lib.rs:1752-1754 set_transport_parameters`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->local_parameters = *tp;

    if (cnx->quic->mtu_max > 0 && cnx->local_parameters.max_packet_size == 0)
    {
        cnx->local_parameters.max_packet_size = cnx->quic->mtu_max - 
            PICOQUIC_MTU_OVERHEAD((struct sockaddr*)&(cnx->path[0])->first_tuple->peer_addr);
    }

    /* Initialize local flow control variables to advertised values */

    cnx->maxdata_local = ((uint64_t)cnx->local_parameters.initial_max_data);
    cnx->max_stream_id_bidir_local = STREAM_ID_FROM_RANK(
        cnx->local_parameters.initial_max_stream_id_bidir, cnx->client_mode, 0);
    cnx->max_stream_id_unidir_local = STREAM_ID_FROM_RANK(
        cnx->local_parameters.initial_max_stream_id_unidir, cnx->client_mode, 1);
}
```

### Rust body
```rust
    pub fn set_transport_parameters(&mut self, tp: &TransportParameters) {
        self.local_parameters = tp.clone();
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_local_if_index`
C: `picoquic/quicctx.c:4453-4457 picoquic_get_local_if_index`
Rust: `rs/fq/src/lib.rs:2916-2940 local_if_index`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->first_tuple->if_index;
}
```

### Rust body
```rust
    pub fn set_local_addr(&mut self, addr: &SocketAddr) -> Result<(), Error> {
        if let Some(path) = self.paths.first_mut()
            && let Some(tuple) = path.tuples.first_mut()
        {
            if !socket_addr_is_unspecified(&tuple.local_addr) {
                return Err(Error::Generic);
            }
            tuple.local_addr = *addr;
            return if socket_addr_is_unspecified(&tuple.local_addr) {
                Err(Error::Generic)
            } else {
                Ok(())
            };
        }
        Err(Error::InvalidArgument)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_client_cnxid`
C: `picoquic/quicctx.c:4477-4481 picoquic_get_client_cnxid`
Rust: `rs/fq/src/lib.rs:3051-3057 client_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return (cnx->client_mode)?cnx->path[0]->first_tuple->p_local_cnxid->cnx_id : cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id;
}
```

### Rust body
```rust
    pub fn client_connection_id(&self) -> ConnectionId {
        if self.client_mode {
            self.initial_connection_id
        } else {
            self.original_connection_id
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_cnx_state`
C: `picoquic/quicctx.c:4501-4505 picoquic_get_cnx_state`
Rust: `rs/fq/src/lib.rs:2835-2843 state`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->cnx_state;
}
```

### Rust body
```rust
    pub fn set_padding_policy(&mut self, padding_multiple: u32, padding_minsize: u32) {
        self.padding_multiple = padding_multiple;
        self.padding_minsize = padding_minsize;
    }
```

## Pair `picoquic/quicctx.c:picoquic_cnx_get_padding_policy`
C: `picoquic/quicctx.c:4526-4531 picoquic_cnx_get_padding_policy`
Rust: `rs/fq/src/lib.rs:2848-2850 padding_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    *padding_multiple = cnx->padding_multiple;
    *padding_minsize = cnx->padding_minsize;
}
```

### Rust body
```rust
    pub fn padding_policy(&self) -> (u32, u32) {
        (self.padding_multiple, self.padding_minsize)
    }
```
