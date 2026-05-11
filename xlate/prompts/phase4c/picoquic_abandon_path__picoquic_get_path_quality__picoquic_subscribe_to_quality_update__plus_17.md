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

## Pair `picoquic/quicctx.c:picoquic_abandon_path`
C: `picoquic/quicctx.c:2583-2626 picoquic_abandon_path`
Rust: `rs/fq/src/lib.rs:2548-2556 abandon_path`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (!cnx->is_multipath_enabled) {
        ret = -1;
    }
    else if (unique_path_id > cnx->max_path_id_remote ||
        unique_path_id > cnx->max_path_id_local) {
        /* that path has not been created yet */
        ret = -1;
    }
    else {
        /* Check whether there is a path by that ID */
        int path_index = picoquic_get_path_id_from_unique(cnx, unique_path_id);

        if (path_index >= 0) {
            /* Check whether this is the last path */
            if (cnx->nb_paths <= 1) {
                /* That would mean deleting the last path. Don't do that */
                ret = -1;
            }
            else if (!cnx->path[path_index]->path_is_demoted) {
                /* if demotion is not already in progress, demote the path,
                * and if the path can be properly identified, post a path abandon frame.
                */
                picoquic_demote_path(cnx, path_index, current_time, reason);
            }
        }
        else {
            /* The path ID is not in use yet, but local cid have been allocated.
             * We need to send an abandon if not sent yet, mark the local CID
             * as demoted, and delete the stash. The stash has to remain deleted
             * even if we receive new CID for that path.
             */
            ret = picoquic_demote_local_cnxid_list(cnx, unique_path_id,
                reason);
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves path teardown signalling.
        Err(Error::Generic)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_path_quality`
C: `picoquic/quicctx.c:2701-2713 picoquic_get_path_quality`
Rust: `rs/fq/src/lib.rs:2692-2700 path_quality`

### C body
```c
{
    int ret = -1;
    int path_id;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        picoquic_path_t* path_x = cnx->path[path_id];
        picoquic_get_path_quality_from_context(path_x, quality);
        ret = 0;
    }
    return ret;
}
```

### Rust body
```rust
    pub fn path_quality(&mut self, unique_path_id: u64) -> Result<PathQuality, Error> {
        let sent = self.pkt_ctx[PacketContext::Application as usize].send_sequence;
        let path = self
            .paths
            .iter_mut()
            .find(|p| p.unique_path_id == unique_path_id)
            .ok_or(Error::InvalidArgument)?;
        Ok(get_path_quality_from_context(path, sent))
    }
```

## Pair `picoquic/quicctx.c:picoquic_subscribe_to_quality_update`
C: `picoquic/quicctx.c:2749-2760 picoquic_subscribe_to_quality_update`
Rust: `rs/fq/src/lib.rs:2737-2743 subscribe_to_quality_update`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->pacing_rate_update_delta = pacing_rate_delta;
    cnx->rtt_update_delta = rtt_delta;
    cnx->is_path_quality_update_requested = 1;

    for (int i = 0; i < cnx->nb_paths; i++) {
        picoquic_subscribe_to_quality_update_per_path_context(cnx->path[i],
            pacing_rate_delta, rtt_delta);
    }
}
```

### Rust body
```rust
    pub fn subscribe_to_quality_update(&mut self, pacing_rate_delta: u64, rtt_delta: Duration) {
        self.rtt_update_delta = rtt_delta;
        self.pacing_rate_update_delta = pacing_rate_delta;
        for path in &mut self.paths {
            path.subscribe_to_quality_update_per_path_context(pacing_rate_delta, rtt_delta);
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_path_status`
C: `picoquic/quicctx.c:2801-2810 picoquic_set_path_status`
Rust: `rs/fq/src/lib.rs:2575-2590 set_path_status`

### C body
```c
{
    int ret = 0;
    int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        cnx->path[path_id]->path_is_backup = (status != picoquic_path_status_available);
        ret = picoquic_queue_path_available_or_backup_frame(cnx, cnx->path[path_id], status);
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
            path.path_is_backup = status == PathStatus::Backup;
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_init_ack_ctx`
C: `picoquic/quicctx.c:2862-2873 picoquic_init_ack_ctx`
Rust: `rs/fq/src/internal.rs:7864-7874 init_ack_ctx`

### C body
```c
{
    picoquic_sack_list_init(&ack_ctx->sack_list);
    ack_ctx->time_stamp_largest_received = UINT64_MAX;
    ack_ctx->act[0].highest_ack_sent = 0;
    ack_ctx->act[0].highest_ack_sent_time = cnx->start_time;
    ack_ctx->act[0].ack_needed = 0;
    ack_ctx->act[1].highest_ack_sent = 0;
    ack_ctx->act[1].highest_ack_sent_time = cnx->start_time;
    ack_ctx->act[1].ack_needed = 0;
}
```

### Rust body
```rust
    pub fn init_ack_ctx(&mut self, ack_ctx: &mut AckContext) {
        // C: picoquic_init_ack_ctx
        ack_ctx.sack_list = SackList::new();
        ack_ctx.time_stamp_largest_received = crate::Instant::from_ticks(u64::MAX);
        ack_ctx.act[0].highest_ack_sent = 0;
        ack_ctx.act[0].highest_ack_sent_time = self.start_time;
        ack_ctx.act[0].ack_needed = false;
        ack_ctx.act[1].highest_ack_sent = 0;
        ack_ctx.act[1].highest_ack_sent_time = self.start_time;
        ack_ctx.act[1].ack_needed = false;
    }
```

## Pair `picoquic/quicctx.c:picoquic_add_remote_cnxid_to_stash`
C: `picoquic/quicctx.c:2944-3036 picoquic_add_remote_cnxid_to_stash`
Rust: `rs/fq/src/internal.rs:5022-5121 add_remote_connection_id_to_stash`

### C body
```c
{
    int ret = 0;
    int is_duplicate = 0;
    size_t nb_cid_received = 0;
    picoquic_connection_id_t cnx_id;
    picoquic_remote_cnxid_t* next_stash = remote_cnxid_stash->cnxid_stash_first;
    picoquic_remote_cnxid_t* last_stash = NULL;
    picoquic_remote_cnxid_t* stashed = NULL;
    int nb_cid_retired_before = 0;

    if (retire_before < remote_cnxid_stash->retire_cnxid_before) {
        retire_before = remote_cnxid_stash->retire_cnxid_before;
    }

    /* verify the format */
    if (picoquic_parse_connection_id(cnxid_bytes, cid_length, &cnx_id) == 0) {
        ret = PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR;
    }

    if (ret == 0 && cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len == 0) {
        /* Protocol error. The peer is using null length cnx_id */
        ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
    }

    while (ret == 0 && is_duplicate == 0 && next_stash != NULL) {
        if (picoquic_compare_connection_id(&cnx_id, &next_stash->cnx_id) == 0)
        {
            if (next_stash->sequence == sequence &&
                cnx_id.id_len == next_stash->cnx_id.id_len &&
                (cnx_id.id_len == 0 || memcmp(cnx_id.id, next_stash->cnx_id.id, cnx_id.id_len) == 0) &&
                memcmp(secret_bytes, next_stash->reset_secret, PICOQUIC_RESET_SECRET_SIZE) == 0) {
                is_duplicate = 1;
            }
            else {
                ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
            }
            break;
        }
        else if (next_stash->sequence == sequence) {
            ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
        }
        else if (memcmp(secret_bytes, next_stash->reset_secret, PICOQUIC_RESET_SECRET_SIZE) == 0) {
            ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
        }
        else {
            if (next_stash->sequence < retire_before || next_stash->retire_sent) {
                nb_cid_retired_before++;
            }
            nb_cid_received++;
        }
        last_stash = next_stash;
        next_stash = next_stash->next;
    }

    if (ret == 0 && is_duplicate == 0) {
        if (nb_cid_received >= cnx->local_parameters.active_connection_id_limit + nb_cid_retired_before ||
            nb_cid_received >= 2*cnx->local_parameters.active_connection_id_limit) {
            ret = PICOQUIC_TRANSPORT_CONNECTION_ID_LIMIT_ERROR;
        }
        else {
            stashed = (picoquic_remote_cnxid_t*)malloc(sizeof(picoquic_remote_cnxid_t));

            if (stashed == NULL) {
                ret = PICOQUIC_TRANSPORT_INTERNAL_ERROR;
            }
            else {
                memset(stashed, 0, sizeof(picoquic_remote_cnxid_t));
                (void)picoquic_parse_connection_id(cnxid_bytes, cid_length, &stashed->cnx_id);
                stashed->sequence = sequence;
                memcpy(stashed->reset_secret, secret_bytes, PICOQUIC_RESET_SECRET_SIZE);
                stashed->next = NULL;

                if (last_stash == NULL) {
                    remote_cnxid_stash->cnxid_stash_first = stashed;
                }
                else {
                    last_stash->next = stashed;
                }
            }
        }
    }

    /* the return argument is only used in tests */

    if (pstashed != NULL) {
        *pstashed = stashed;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> StashResult {
        // C: picoquic_add_remote_cnxid_to_stash
        // C transport error codes: 0x1=INTERNAL, 0x7=FRAME_FORMAT, 0xA=PROTOCOL_VIOLATION
        const INTERNAL_ERROR: u64 = 0x1;
        const FRAME_FORMAT_ERROR: u64 = 0x7;
        const PROTOCOL_VIOLATION: u64 = 0xA;

        let stash = match self.remote_connection_id_stashes.get_mut(stash_index) {
            Some(s) => s,
            None => {
                return StashResult {
                    status: INTERNAL_ERROR,
                    stashed_index: None,
                };
            }
        };

        let cnx_id = match crate::ConnectionId::clone_from_slice(connection_id_bytes) {
            Some(id) => id,
            None => {
                return StashResult {
                    status: FRAME_FORMAT_ERROR,
                    stashed_index: None,
                };
            }
        };

        // Ensure retire_connection_id_before moves forward.
        if retire_before_next > stash.retire_connection_id_before {
            stash.retire_connection_id_before = retire_before_next;
        }

        // Check for duplicates / sequence collision.
        let mut secret_arr = [0u8; RESET_SECRET_SIZE];
        let slen = secret_bytes.len().min(RESET_SECRET_SIZE);
        secret_arr[..slen].copy_from_slice(&secret_bytes[..slen]);

        for (idx, r) in stash.connection_ids.iter().enumerate() {
            if r.connection_id == cnx_id {
                if r.sequence == sequence && r.reset_secret == secret_arr {
                    // Duplicate — not an error, just no-op.
                    return StashResult {
                        status: 0,
                        stashed_index: Some(idx),
                    };
                } else {
                    return StashResult {
                        status: PROTOCOL_VIOLATION,
                        stashed_index: None,
                    };
                }
            } else if r.sequence == sequence || r.reset_secret == secret_arr {
                return StashResult {
                    status: PROTOCOL_VIOLATION,
                    stashed_index: None,
                };
            }
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
        let stash = &mut self.remote_connection_id_stashes[stash_index];
        let new_rcid = RemoteConnectionId {
            sequence,
            connection_id: cnx_id,
            reset_secret: secret_arr,
            nb_path_references: 0,
            needs_removal: false,
            retire_sent: false,
            retire_acked: false,
            pkt_ctx: zero_pkt_ctx,
        };
        stash.connection_ids.push(new_rcid);
        let stashed_index = stash.connection_ids.len() - 1;
        StashResult {
            status: 0,
            stashed_index: Some(stashed_index),
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_cnxid_from_stash`
C: `picoquic/quicctx.c:3102-3113 picoquic_get_cnxid_from_stash`
Rust: `rs/fq/src/internal.rs:5202-5208 get_connection_id_from_stash`

### C body
```c
{
    picoquic_remote_cnxid_t* stashed = NULL;
    if (stash != NULL) {
        stashed = stash->cnxid_stash_first;
        while (stashed != NULL && stashed->cnx_id.id_len > 0
            && (stashed->nb_path_references != 0 || stashed->needs_removal)) {
            stashed = stashed->next;
        }
    }
    return stashed;
}
```

### Rust body
```rust
        for (i, r) in self.connection_ids.iter().enumerate() {
            if r.connection_id.is_empty() || (r.nb_path_references == 0 && !r.needs_removal) {
                return Some(i);
            }
        }
```

## Pair `picoquic/quicctx.c:picoquic_remove_not_before_from_stash`
C: `picoquic/quicctx.c:3154-3234 picoquic_remove_not_before_from_stash`
Rust: `rs/fq/src/internal.rs:5292-5309 remove_not_before_from_stash`

### C body
```c
{
    uint64_t ret = 0;
    if (cnxid_stash != NULL) {

        picoquic_remote_cnxid_t* next_stash = cnxid_stash->cnxid_stash_first;
        picoquic_remote_cnxid_t* previous_stash = NULL;

        while (ret == 0 && next_stash != NULL) {
            next_stash->needs_removal |= (next_stash->sequence < not_before);
            if (next_stash->needs_removal && next_stash->nb_path_references == 0) {
                if (!next_stash->retire_sent) {
                    ret = picoquic_queue_retire_connection_id_frame(cnx, cnxid_stash->unique_path_id, next_stash->sequence);
                    if (ret == 0) {
                        next_stash->retire_sent = 1;
                    }
                }
                if (ret == 0 && next_stash->retire_acked) {
                    next_stash = picoquic_remove_cnxid_from_stash(cnx, cnxid_stash, next_stash, previous_stash);
                }
                else {
                    previous_stash = next_stash;
                    next_stash = next_stash->next;
                }
            }
            else {
                previous_stash = next_stash;
                next_stash = next_stash->next;
            }
        }

        /* We need to stop transmitting data to the old CID. But we cannot just delete
        * the correspondng paths,because there may be some data in transit. We must
        * also ensure that at least one default path migrates successfully to a
        * valid CID. As long as new CID are available, we can simply replace the
        * old one by a new one. If no CID is available, the old path should be marked
        * as failing, and thus scheduled for deletion after a time-out */

        if (cnx->is_multipath_enabled) {
            int path_id = picoquic_find_path_by_unique_id(cnx, cnxid_stash->unique_path_id);
            if (path_id >= 0) {
                if (cnx->path[path_id]->first_tuple->p_remote_cnxid->sequence < not_before &&
                    cnx->path[path_id]->first_tuple->p_remote_cnxid->cnx_id.id_len > 0 &&
                    !cnx->path[path_id]->path_is_demoted) {
                    ret = picoquic_renew_connection_id(cnx, path_id);
                    if (ret != 0) {
                        DBG_PRINTF("Renew CNXID returns %x\n", ret);
                        if (path_id == 0) {
                            ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
                        }
                        else {
                            ret = 0;
                            picoquic_demote_path(cnx, path_id, current_time, 0);
                        }
                    }
                }
            }
        }
        else {
            for (int i = 0; ret == 0 && i < cnx->nb_paths; i++) {
                if (cnx->path[i]->first_tuple->p_remote_cnxid->sequence < not_before &&
                    cnx->path[i]->first_tuple->p_remote_cnxid->cnx_id.id_len > 0 &&
                    !cnx->path[i]->path_is_demoted) {
                    ret = picoquic_renew_connection_id(cnx, i);
                    if (ret != 0) {
                        DBG_PRINTF("Renew CNXID returns %x\n", ret);
                        if (i == 0) {
                            ret = PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION;
                        }
                        else {
                            ret = 0;
                            picoquic_demote_path(cnx, i, current_time, 0);
                        }
                    }
                }
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> u64 {
        // Remove all CIDs whose sequence < not_before.
        let mut removed = 0u64;
        connection_id_stash.connection_ids.retain(|r| {
            if r.sequence < not_before {
                removed += 1;
                false
            } else {
                true
            }
        });
        removed
    }
```

## Pair `picoquic/quicctx.c:picoquic_renew_connection_id`
C: `picoquic/quicctx.c:3325-3337 picoquic_renew_connection_id`
Rust: `rs/fq/src/internal.rs:4696-4714 renew_connection_id`

### C body
```c
{
    int ret;

    if (path_id >= cnx->nb_paths) {
        ret = -1;
    }
    else {
        ret = picoquic_renew_path_connection_id(cnx, cnx->path[path_id]);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn renew_connection_id(&mut self, path_id: i32) -> Result<(), crate::Error> {
        let idx = path_id as usize;
        if idx >= self.paths.len() {
            return Err(crate::Error::InvalidArgument);
        }
        if let Some((stash_idx, cid_idx)) =
            self.obtain_stashed_connection_id(self.paths[idx].unique_path_id)
            && let Some(tuple) = self.paths[idx].tuples.first_mut()
        {
            tuple.remote_connection_id_index = Some(cid_idx);
            if let Some(cid) = self.remote_connection_id_stashes[stash_idx]
                .connection_ids
                .get_mut(cid_idx)
            {
                cid.nb_path_references += 1;
            }
        }
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_stream_from_node`
C: `picoquic/quicctx.c:3459-3466 picoquic_stream_from_node`
Rust: `rs/fq/src/lib.rs:7347-7511 stream_from_node`

### C body
```c
{
#ifdef TOO_CAUTIOUS
    return(picoquic_stream_head_t *)((node == NULL)?NULL:picoquic_stream_node_value(node));
#else
    return (picoquic_stream_head_t *)node;
#endif
}
```

### Rust body
```rust
impl Quic {
    /// Queue an immediate close packet for `connection` if packet preparation
    /// produces bytes to send.
    ///
    /// C: `picoquic_queue_immediate_close` (picoquic/packet.c:1317-1334).
    fn queue_immediate_close(&mut self, connection: ConnectionToken, current_time: Instant) {
        let Ok(mut sp) = self.create_stateless_packet() else {
            return;
        };
        let prepared = self
            .connections
            .get_mut(connection)
            .and_then(|cnx| cnx.prepare_packet_ex(current_time, &mut sp.bytes).ok());

        if let Some(prepared) = prepared
            && prepared.send_length > 0
        {
            sp.length = prepared.send_length;
            sp.addr_to = prepared.addr_to;
            sp.addr_local = prepared.addr_from;
            sp.if_index_local = prepared.if_index;
            self.queue_stateless_packet(sp);
        }
    }

    /// Process an incoming client Initial packet on a server connection.
    /// Returns the C-style status code and the still-live connection token.
    ///
    /// C: `picoquic_incoming_client_initial` (picoquic/packet.c:1394-1505).
    pub fn incoming_client_initial(
        &mut self,
        connection: ConnectionToken,
        bytes: &[u8],
        packet_length: usize,
        received_data: &mut crate::internal::StreamDataNode,
        addr_from: Option<&SocketAddr>,
        addr_to: Option<&SocketAddr>,
        if_index_to: u64,
        ph: &crate::internal::PacketHeader,
        current_time: Instant,
        new_context_created: bool,
    ) -> (i32, Option<ConnectionToken>) {
        let server_busy = self.server_busy;
        let over_connection_limit =
            self.current_number_connections > self.tentative_max_number_connections;
        let mut ret = 0;
        let mut queue_close = false;
        let mut delete_created_connection = false;

        {
            let Some(cnx) = self.connections.get_mut(connection) else {
                return (InternalError::UnexpectedPacket as i32, None);
            };

            if cnx
                .path_local_connection_id(0)
                .is_some_and(|cid| !cid.is_empty() && cid == ph.dest_connection_id)
            {
                cnx.initial_validated = true;
            }

            if !cnx.initial_validated
                && !cnx.pkt_ctx[PacketContext::Initial as usize]
                    .pending
                    .is_empty()
                && packet_length >= crate::internal::ENFORCED_INITIAL_MTU
            {
                cnx.initial_repeat_needed = true;
            }

            if cnx.connection_state == State::ServerInit && (server_busy || over_connection_limit) {
                cnx.local_error = TransportError::ServerBusy as u64;
                cnx.connection_state = State::HandshakeFailure;
            } else if cnx.connection_state == State::ServerInit
                && cnx.initial_connection_id.len()
                    < crate::internal::ENFORCED_INITIAL_CID_LENGTH as usize
            {
                cnx.local_error = TransportError::ProtocolViolation as u64;
                cnx.connection_state = State::HandshakeFailure;
            } else if cnx.connection_state < State::ServerAlmostReady {
                if let Some(path) = cnx.paths.get_mut(0)
                    && let Some(tuple) = path.tuples.first_mut()
                {
                    if Connection::socket_addr_is_unspecified(&tuple.local_addr)
                        && let Some(addr) = addr_to
                    {
                        tuple.local_addr = *addr;
                    }
                    if Connection::socket_addr_is_unspecified(&tuple.peer_addr)
                        && let Some(addr) = addr_from
                    {
                        tuple.peer_addr = *addr;
                    }
                    tuple.if_index = if_index_to as core::ffi::c_ulong;
                }

                let highest_ack_before =
                    cnx.pkt_ctx[PacketContext::Initial as usize].highest_acknowledged;
                let payload = Connection::packet_payload(bytes, ph);
                ret = cnx.decode_frames_on_path(
                    0,
                    payload,
                    received_data,
                    ph.epoch,
                    addr_from,
                    addr_to,
                    ph.packet_number_full,
                    0,
                    current_time,
                );
                if cnx.pkt_ctx[PacketContext::Initial as usize].highest_acknowledged
                    > highest_ack_before
                    && cnx.random_initial > 1
                {
                    cnx.initial_validated = true;
                }

                if ret == 0 {
                    let (tls_ret, data_consumed) = cnx.process_tls_stream_status(current_time);
                    ret = tls_ret;
                    if data_consumed > 0 {
                        cnx.initial_repeat_needed = false;
                    }
                }
            } else if cnx.connection_state < State::Ready {
                cnx.ignore_incoming_handshake(bytes, ph, current_time);
            } else {
                ret = InternalError::UnexpectedPacket as i32;
            }

            if ret == InternalError::InvalidToken as i32
                && cnx.connection_state == State::HandshakeFailure
            {
                ret = 0;
            }

            if ret == 0 && cnx.connection_state == State::HandshakeFailure && new_context_created {
                queue_close = true;
            }

            if ret != 0 || cnx.connection_state == State::Disconnected {
                delete_created_connection = new_context_created;
            }
        }

        if queue_close {
            self.queue_immediate_close(connection, current_time);
        }

        if delete_created_connection {
            self.delete_connection(connection);
            (InternalError::ConnectionDeleted as i32, None)
        } else {
            (ret, Some(connection))
        }
    }
}
```

## Pair `picoquic/quicctx.c:picoquic_insert_output_stream`
C: `picoquic/quicctx.c:3502-3562 picoquic_insert_output_stream`
Rust: `rs/fq/src/internal.rs:9567-9595 insert_output_stream`

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

## Pair `picoquic/quicctx.c:picoquic_find_stream`
C: `picoquic/quicctx.c:3611-3617 picoquic_find_stream`
Rust: `rs/fq/src/internal.rs:9639-9642 find_stream`

### C body
```c
{
    picoquic_stream_head_t target;
    target.stream_id = stream_id;

    return (picoquic_stream_head_t *)picosplay_find(&cnx->stream_tree, (void*)&target);
}
```

### Rust body
```rust
    pub fn find_stream(&mut self, stream_id: u64) -> Option<StreamToken> {
        let st = self.stream_tree.find(&stream_id)?;
        self.stream_tree.get(st).copied()
    }
```

## Pair `picoquic/quicctx.c:picoquic_mark_direct_receive_stream`
C: `picoquic/quicctx.c:3703-3759 picoquic_mark_direct_receive_stream`
Rust: `rs/fq/src/lib.rs:3856-3947 mark_direct_receive_stream`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream;
    picoquic_stream_data_node_t* data;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if ((stream = picoquic_find_stream(cnx, stream_id)) == NULL) {
        ret = PICOQUIC_ERROR_INVALID_STREAM_ID;
    }
    else if (!IS_BIDIR_STREAM_ID(stream_id) && IS_LOCAL_STREAM_ID(stream_id, cnx->client_mode)) {
        ret = PICOQUIC_ERROR_INVALID_STREAM_ID;
    }
    else if (direct_receive_fn == NULL) {
        /* This is illegal! */
        ret = PICOQUIC_ERROR_NO_CALLBACK_PROVIDED;
    }
    else {
        stream->direct_receive_fn = direct_receive_fn;
        stream->direct_receive_ctx = direct_receive_ctx;
        /* If there is pending data, pass it. */
        while ((data = (picoquic_stream_data_node_t*)picosplay_first(&stream->stream_data_tree)) != NULL) {
            size_t length = data->length;
            uint64_t offset = data->offset;

            if (offset < stream->consumed_offset) {
                if (offset + length < stream->consumed_offset) {
                    length = 0;
                }
                else {
                    size_t delta_offset = (size_t)(stream->consumed_offset - offset);
                    length -= delta_offset;
                    offset += delta_offset;
                }
            }

            if (length > 0) {
                ret = direct_receive_fn(cnx, stream_id, 0, data->bytes, offset, length, direct_receive_ctx);
            }

            if (ret == 0) {
                picosplay_delete_hint(&stream->stream_data_tree, &data->stream_data_node);
            }
            else {
                break;
            }
        }

        /* If there is a fin offset, pass it. */
        if (ret == 0 && stream->fin_received && !stream->fin_signalled) {
            uint8_t fin_bytes[8];
            ret = direct_receive_fn(cnx, stream_id, 1, fin_bytes, stream->fin_offset, 0, direct_receive_ctx);
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        use crate::stream::{Role, StreamId};

        let stream_token = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let sid = StreamId(stream_id);
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };
        if !sid.is_bidir() && sid.is_local(local_role) {
            return Err(Error::Protocol(InternalError::InvalidStreamId as u64));
        }

        loop {
            let next = {
                let stream = self.streams.get(stream_token).ok_or(Error::Memory)?;
                let Some(tree_token) = stream.stream_data_tree.first() else {
                    break;
                };
                let Some(data_token) = stream.stream_data_tree.get(tree_token).copied() else {
                    break;
                };
                let Some(data) = stream.stream_data_nodes.get(data_token) else {
                    break;
                };
                let mut offset = data.offset;
                let mut length = data.length;
                let mut start = 0usize;
                if offset < stream.consumed_offset {
                    let end = offset.saturating_add(length as u64);
                    if end < stream.consumed_offset {
                        length = 0;
                    } else {
                        start = (stream.consumed_offset - offset) as usize;
                        length -= start;
                        offset = stream.consumed_offset;
                    }
                }
                let bytes = data.data[start..start + length].to_vec();
                (tree_token, data_token, offset, bytes)
            };

            let (tree_token, data_token, offset, bytes) = next;
            if !bytes.is_empty() {
                let ret = direct_receive.receive(self, stream_id, false, &bytes, offset);
                if ret != 0 {
                    if let Some(stream) = self.streams.get_mut(stream_token) {
                        stream.direct_receive_fn = Some(direct_receive);
                    }
                    return Err(Error::Protocol(ret as u64));
                }
            }
            if let Some(stream) = self.streams.get_mut(stream_token) {
                stream.stream_data_tree.remove(tree_token);
                stream.stream_data_nodes.remove(data_token);
            }
        }

        let fin_to_signal = self
            .streams
            .get(stream_token)
            .map(|stream| stream.fin_received && !stream.fin_signalled)
            .unwrap_or(false);
        if fin_to_signal {
            let fin_offset = self
                .streams
                .get(stream_token)
                .map(|stream| stream.fin_offset)
                .unwrap_or(0);
            let ret = direct_receive.receive(self, stream_id, true, &[], fin_offset);
            if ret != 0 {
                if let Some(stream) = self.streams.get_mut(stream_token) {
                    stream.direct_receive_fn = Some(direct_receive);
                }
                return Err(Error::Protocol(ret as u64));
            }
            if let Some(stream) = self.streams.get_mut(stream_token) {
                stream.fin_signalled = true;
            }
        }

        let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;
        stream.direct_receive_fn = Some(direct_receive);
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_delete_local_cnxid_list`
C: `picoquic/quicctx.c:3934-3956 picoquic_delete_local_cnxid_list`
Rust: `rs/fq/src/internal.rs:13191-13194 delete_local_connection_id_list`

### C body
```c
{
    while (local_cnxid_list->local_cnxid_first != NULL) {
        picoquic_delete_local_cnxid_listed(cnx, local_cnxid_list, local_cnxid_list->local_cnxid_first);
    }

    if (local_cnxid_list == cnx->first_local_cnxid_list) {
        cnx->first_local_cnxid_list = local_cnxid_list->next_list;
    }
    else {
        picoquic_local_cnxid_list_t* previous = cnx->first_local_cnxid_list;

        while (previous != NULL) {
            if (previous->next_list == local_cnxid_list) {
                previous->next_list = local_cnxid_list->next_list;
            }
            previous = previous->next_list;
        }
    }

    free(local_cnxid_list);
    cnx->nb_local_cnxid_lists--;
}
```

### Rust body
```rust
        if list_index >= self.local_connection_id_lists.len() {
            return;
        }
```

## Pair `picoquic/quicctx.c:picoquic_find_local_cnxid`
C: `picoquic/quicctx.c:4017-4034 picoquic_find_local_cnxid`
Rust: `rs/fq/src/lib.rs:2965-2977 find_local_cnxid`

### C body
```c
{
    picoquic_local_cnxid_t* local_cnxid = NULL;
    picoquic_local_cnxid_list_t* local_cnxid_list = picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 0);
    
    if (local_cnxid_list != NULL && (local_cnxid = local_cnxid_list->local_cnxid_first) != NULL) {
        while (local_cnxid != NULL) {
            if (picoquic_compare_connection_id(&local_cnxid->cnx_id, cnxid) == 0) {
                break;
            }
            else {
                local_cnxid = local_cnxid->next;
            }
        }
    }
    
    return local_cnxid;
}
```

### Rust body
```rust
    pub fn retire_local_cnxid(&mut self, unique_path_id: u64, sequence: u64) {
        self.retire_local_connection_id(unique_path_id, sequence);
    }
```

## Pair `picoquic/quicctx.c:picoquic_start_client_cnx`
C: `picoquic/quicctx.c:4384-4413 picoquic_start_client_cnx`
Rust: `rs/fq/src/lib.rs:2334-2338 start_client`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx->cnx_state != picoquic_state_client_init ||
        cnx->tls_stream[0].sent_offset > 0 ||
        cnx->tls_stream[0].send_queue != NULL) {
        DBG_PRINTF("%s", "picoquic_start_client_cnx called twice.");
        return -1;
    }

    picoquic_log_new_connection(cnx);
        
    ret = picoquic_initialize_tls_stream(cnx, picoquic_get_quic_time(cnx->quic));
    /* A remote session ticket may have been loaded as part of initializing TLS,
     * and remote parameters may have been initialized to the initial value
     * of the previous session. Apply these new parameters. */
    cnx->maxdata_remote = cnx->remote_parameters.initial_max_data;
    cnx->max_stream_id_bidir_remote =
        STREAM_ID_FROM_RANK(cnx->remote_parameters.initial_max_stream_id_bidir, cnx->client_mode, 0);
    cnx->max_stream_id_unidir_remote = 
        STREAM_ID_FROM_RANK(cnx->remote_parameters.initial_max_stream_id_unidir, cnx->client_mode, 1);
    cnx->max_stream_data_remote = cnx->remote_parameters.initial_max_data;
    cnx->max_stream_data_local = cnx->local_parameters.initial_max_stream_data_bidi_local;

    picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));

    return ret;
}
```

### Rust body
```rust
    pub fn start_client(&mut self) -> Result<(), Error> {
        self.setup_initial_traffic_keys()?;
        self.connection_state = State::ClientInitSent;
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_local_addr`
C: `picoquic/quicctx.c:4447-4451 picoquic_get_local_addr`
Rust: `rs/fq/src/lib.rs:4961-4972 get_local_addr`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    *addr = (struct sockaddr*)&cnx->path[0]->first_tuple->local_addr;
}
```

### Rust body
```rust
    pub fn path_is_backup(&self, index: usize) -> bool {
        self.paths
            .get(index)
            .map(|p| p.path_is_backup)
            .unwrap_or(false)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_initial_cnxid`
C: `picoquic/quicctx.c:4471-4475 picoquic_get_initial_cnxid`
Rust: `rs/fq/src/lib.rs:3045-3047 initial_connection_id`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->initial_cnxid;
}
```

### Rust body
```rust
    pub fn initial_connection_id(&self) -> ConnectionId {
        self.initial_connection_id
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_cnx_start_time`
C: `picoquic/quicctx.c:4495-4499 picoquic_get_cnx_start_time`
Rust: `rs/fq/src/lib.rs:3075-3082 start_time`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->start_time;
}
```

### Rust body
```rust
    pub fn is_0rtt_available(&self) -> bool {
        self.zero_rtt_data_accepted
    }
```

## Pair `picoquic/quicctx.c:picoquic_cnx_set_padding_policy`
C: `picoquic/quicctx.c:4519-4524 picoquic_cnx_set_padding_policy`
Rust: `rs/fq/src/lib.rs:2840-2850 set_padding_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->padding_multiple = padding_multiple;
    cnx->padding_minsize = padding_minsize;
}
```

### Rust body
```rust
    pub fn padding_policy(&self) -> (u32, u32) {
        (self.padding_multiple, self.padding_minsize)
    }
```
