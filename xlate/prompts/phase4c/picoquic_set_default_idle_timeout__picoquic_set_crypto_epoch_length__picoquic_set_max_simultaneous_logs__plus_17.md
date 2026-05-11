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

## Pair `picoquic/quicctx.c:picoquic_set_default_idle_timeout`
C: `picoquic/quicctx.c:992-996 picoquic_set_default_idle_timeout`
Rust: `rs/fq/src/lib.rs:1744-1754 set_default_idle_timeout`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_tp.max_idle_timeout = idle_timeout_ms;
}
```

### Rust body
```rust
    pub fn set_transport_parameters(&mut self, tp: &TransportParameters) {
        self.local_parameters = tp.clone();
    }
```

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

## Pair `picoquic/quicctx.c:picoquic_set_max_simultaneous_logs`
C: `picoquic/quicctx.c:1044-1048 picoquic_set_max_simultaneous_logs`
Rust: `rs/fq/src/lib.rs:1905-1912 set_max_simultaneous_logs`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->max_simultaneous_logs = max_simultaneous_logs;
}
```

### Rust body
```rust
    pub fn max_simultaneous_logs(&self) -> u32 {
        self.max_simultaneous_logs
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_low_memory_mode`
C: `picoquic/quicctx.c:1175-1180 picoquic_set_low_memory_mode`
Rust: `rs/fq/src/lib.rs:1543-1546 set_low_memory_mode`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->use_low_memory = (low_memory_mode == 0) ? 0 : 1;
    return picoquic_set_cipher_suite(quic, 0);
}
```

### Rust body
```rust
    pub fn set_low_memory_mode(&mut self, low_memory_mode: bool) -> Result<(), Error> {
        self.use_low_memory = low_memory_mode;
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_max_half_open_retry_threshold`
C: `picoquic/quicctx.c:1212-1216 picoquic_get_max_half_open_retry_threshold`
Rust: `rs/fq/src/lib.rs:1184-1192 max_half_open_retry_threshold`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->max_half_open_before_retry;
}
```

### Rust body
```rust
    pub fn set_port_blocking_disabled(&mut self, is_port_blocking_disabled: bool) {
        self.is_port_blocking_disabled = is_port_blocking_disabled;
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

## Pair `picoquic/quicctx.c:picoquic_register_net_icid`
C: `picoquic/quicctx.c:1340-1354 picoquic_register_net_icid`
Rust: `rs/fq/src/internal.rs:4031-4045 register_net_icid`

### C body
```c
{
    int ret = 0;
    picohash_item* item;
    picoquic_store_addr(&cnx->registered_icid_addr, (struct sockaddr*)&cnx->path[0]->first_tuple->peer_addr);
    item = picohash_retrieve(cnx->quic->table_cnx_by_icid, cnx);

    if (item != NULL) {
        ret = -1;
    }
    else {
        ret = picohash_insert(cnx->quic->table_cnx_by_icid, cnx);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn register_net_icid(&mut self) -> Result<(), crate::Error> {
        if self.connection_by_icid_membership.is_some()
            || !crate::socket_addr_is_unspecified(&self.registered_icid_addr)
        {
            return Err(crate::Error::Generic);
        }
        let peer_addr = self
            .paths
            .first()
            .and_then(|path| path.tuples.first())
            .map(|tuple| tuple.peer_addr)
            .ok_or(crate::Error::InvalidArgument)?;
        self.registered_icid_addr = peer_addr;
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_init_transport_parameters`
C: `picoquic/quicctx.c:1395-1414 picoquic_init_transport_parameters`
Rust: `rs/fq/src/internal.rs:3950-3968 init_transport_parameters`

### C body
```c
{
    memset(tp, 0, sizeof(picoquic_tp_t));
    tp->initial_max_stream_data_bidi_local = 0x200000;
    tp->initial_max_stream_data_bidi_remote = 65635;
    tp->initial_max_stream_data_uni = 65535;
    tp->initial_max_data = PICOQUIC_INITIAL_FLOW_CONTROL_MAX;
    tp->initial_max_stream_id_bidir = 512;
    tp->initial_max_stream_id_unidir = 512;
    tp->max_idle_timeout = PICOQUIC_MICROSEC_HANDSHAKE_MAX/1000;
    tp->max_packet_size = PICOQUIC_PRACTICAL_MAX_MTU;
    tp->max_datagram_frame_size = 0;
    tp->ack_delay_exponent = 3;
    tp->active_connection_id_limit = PICOQUIC_NB_PATH_TARGET;
    tp->max_ack_delay = PICOQUIC_ACK_DELAY_MAX;
    tp->enable_loss_bit = 2;
    tp->min_ack_delay = PICOQUIC_ACK_DELAY_MIN;
    tp->enable_time_stamp = 0;
    tp->enable_bdp_frame = 0;
}
```

### Rust body
```rust
pub fn init_transport_parameters(tp: &mut TransportParameters) {
    *tp = TransportParameters::default();
    tp.initial_max_stream_data_bidi_local = 0x20_0000;
    tp.initial_max_stream_data_bidi_remote = 65_635;
    tp.initial_max_stream_data_uni = 65_535;
    tp.initial_max_data = INITIAL_FLOW_CONTROL_MAX;
    tp.initial_max_stream_id_bidir = 512;
    tp.initial_max_stream_id_unidir = 512;
    tp.max_idle_timeout = Duration::from_ticks(MICROSEC_HANDSHAKE_MAX.ticks() / 1000);
    tp.max_packet_size = PRACTICAL_MAX_MTU as u32;
    tp.max_datagram_frame_size = 0;
    tp.ack_delay_exponent = 3;
    tp.active_connection_id_limit = NB_PATH_TARGET as u32;
    tp.max_ack_delay = ACK_DELAY_MAX.ticks() as u32;
    tp.enable_loss_bit = 2;
    tp.min_ack_delay = ACK_DELAY_MIN;
    tp.enable_time_stamp = 0;
    tp.enable_bdp_frame = false;
}
```

## Pair `picoquic/quicctx.c:picoquic_insert_cnx_in_list`
C: `picoquic/quicctx.c:1436-1448 picoquic_insert_cnx_in_list`
Rust: `rs/fq/src/internal.rs:3888-3936 insert_cnx_in_list`

### C body
```c
{
    if (quic->cnx_list != NULL) {
        quic->cnx_list->previous_in_table = cnx;
        cnx->next_in_table = quic->cnx_list;
    } else {
        quic->cnx_last = cnx;
        cnx->next_in_table = NULL;
    }
    quic->cnx_list = cnx;
    cnx->previous_in_table = NULL;
    quic->current_number_connections++;
}
```

### Rust body
```rust
    pub fn delete_connection(&mut self, token: ConnectionToken) {
        let Some(cnx) = self.connections.get(token) else {
            return;
        };
        let initial_cid = cnx.initial_connection_id;
        let local_cid_memberships: Vec<_> = cnx
            .local_connection_id_lists
            .iter()
            .flat_map(|list| list.connection_ids.iter().copied())
            .filter_map(|tok| {
                cnx.local_connection_ids
                    .get(tok)
                    .and_then(|l_cid| l_cid.connection_by_id_membership)
            })
            .collect();
        let was_half_open = cnx.is_half_open;

        for membership in local_cid_memberships {
            self.connection_by_id.remove(membership);
        }
        if !initial_cid.is_empty()
            && let Some(ht) = self.connection_by_id.lookup(&initial_cid)
            && self.connection_by_id.get(ht).copied() == Some(token)
        {
            self.connection_by_id.remove(ht);
        }
        while self
            .connections
            .get(token)
            .is_some_and(|cnx| !cnx.paths.is_empty())
        {
            self.clear_path_data(token, 0);
        }
        self.remove_cnx_from_list(token);
        self.remove_cnx_from_wake_list(token);

        // Update accounting.
        if was_half_open {
            self.current_number_half_open = self.current_number_half_open.saturating_sub(1);
        }

        self.connections.remove(token);
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

## Pair `picoquic/quicctx.c:picoquic_get_next_wake_delay`
C: `picoquic/quicctx.c:1553-1577 picoquic_get_next_wake_delay`
Rust: `rs/fq/src/lib.rs:3194-3207 next_wake_delay`

### C body
```c
{
    /* We assume that "current time" is no more than 100,000 years in the
     * future, which implies the time in microseconds is less than 2^62.
     * The delay MAX is lower than INT64_MAX, i.e., 2^63.
     * The next wake time is often set to UINT64_MAX, and might sometime
     * be just under that value, so we make sure to avoid integer
     * overflow in the computation.
     */
    uint64_t next_wake_time = picoquic_get_next_wake_time(quic, current_time);
    int64_t wake_delay = 0;

    if (next_wake_time > current_time) {
        uint64_t delta_m = current_time + delay_max;

        if (next_wake_time >= delta_m) {
            wake_delay = delay_max;
        }
        else {
            wake_delay = (int64_t)(next_wake_time - current_time);
        }
    }
    return wake_delay;
}
```

### Rust body
```rust
    pub fn next_wake_delay(&self, current_time: Instant, delay_max: i64) -> i64 {
        let now = current_time.ticks();
        // Find the minimum next_wake_time across all connections.
        let earliest = self
            .connections
            .iter()
            .map(|c| c.next_wake_time.ticks())
            .min();
        match earliest {
            None => delay_max,
            Some(t) if t <= now => 0,
            Some(t) => ((t - now) as i64).min(delay_max),
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_create_random_cnx_id`
C: `picoquic/quicctx.c:1630-1639 picoquic_create_random_cnx_id`
Rust: `rs/fq/src/lib.rs:1300-1307 create_random_cnx_id`

### C body
```c
{
    if (id_length > 0) {
        picoquic_crypto_random(quic, cnx_id->id, id_length);
    }
    if (id_length < sizeof(cnx_id->id)) {
        memset(cnx_id->id + id_length, 0, sizeof(cnx_id->id) - id_length);
    }
    cnx_id->id_len = id_length;
}
```

### Rust body
```rust
pub(crate) fn create_random_cnx_id(quic: &mut Quic, id_length: u8) -> ConnectionId {
    let len = (id_length as usize).min(CONNECTION_ID_MAX_SIZE);
    let mut cnx_id = ConnectionId::with_size(len).unwrap_or_default();
    if len > 0 {
        rand_core::RngCore::fill_bytes(&mut *quic.rng, cnx_id.as_bytes_mut());
    }
    cnx_id
}
```

## Pair `picoquic/quicctx.c:picoquic_get_ack_number`
C: `picoquic/quicctx.c:1694-1698 picoquic_get_ack_number`
Rust: `rs/fq/src/internal.rs:7843-7858 get_ack_number`

### C body
```c
{
    return (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) ? path_x->pkt_ctx.highest_acknowledged :
        cnx->pkt_ctx[pc].highest_acknowledged;
}
```

### Rust body
```rust
impl Connection {
    pub fn get_last_packet(&self, _path_x: &mut Path, pc: PacketContext) -> Option<PacketToken> {
        // C: picoquic_get_last_packet — last (highest seq) in pending queue
        self.pkt_ctx[pc as usize]
            .pending
            .values()
            .next_back()
            .copied()
    }
}
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

## Pair `picoquic/quicctx.c:picoquic_register_path`
C: `picoquic/quicctx.c:1868-1877 picoquic_register_path`
Rust: `rs/fq/src/internal.rs:4365-4369 register_path`

### C body
```c
{
    if (path_x->first_tuple->peer_addr.ss_family != 0 && cnx->quic->local_cnxid_length == 0) {
        (void)picoquic_register_net_id(cnx->quic, cnx, path_x);
    }
}
```

### Rust body
```rust
        if let Some(tuple) = path_x.tuples.first() {
            path_x.registered_peer_addr = tuple.peer_addr;
        }
```

## Pair `picoquic/quicctx.c:picoquic_demote_path`
C: `picoquic/quicctx.c:2052-2116 picoquic_demote_path`
Rust: `rs/fq/src/internal.rs:4726-4731 demote_path`

### C body
```c
{
    if (!cnx->path[path_index]->path_is_demoted) {
        uint64_t demote_timer = cnx->path[path_index]->retransmit_timer;

        if (demote_timer < PICOQUIC_INITIAL_MAX_RETRANSMIT_TIMER &&
            !cnx->is_multipath_enabled) {
            demote_timer = PICOQUIC_INITIAL_MAX_RETRANSMIT_TIMER;
        }

        cnx->path[path_index]->path_is_demoted = 1;
        cnx->path[path_index]->demotion_time = current_time + 3* demote_timer;
        cnx->path_demotion_needed = 1;

        /* TODO: add suspended callback */
        if (cnx->is_multipath_enabled) {
             /* Special case for path 0: we want to reorder the paths so the path[0]
             * is always a valid path.
             */
            if (path_index == 0) {
                int alt_path0 = 0;
                for (int i = 1; i < cnx->nb_paths; i++) {
                    if (cnx->path[i]->first_tuple->p_remote_cnxid != NULL) {
                        alt_path0 = i;
                        break;
                    }
                }
                if (alt_path0 != 0) {
                    picoquic_path_t* path_x = cnx->path[0];
                    cnx->path[0] = cnx->path[alt_path0];
                    cnx->path[alt_path0] = path_x;
                    path_index = alt_path0;
                }
            }
            if (path_index == 0) {
                picoquic_log_app_message(cnx, "Cannot demote path index 0, unique_id %" PRIu64", was reason % " PRIu64,
                    cnx->path[path_index]->unique_path_id, reason);
            }
            else if (!cnx->path[path_index]->path_abandon_sent) {
                uint64_t path_id = cnx->path[path_index]->unique_path_id;
                if (picoquic_queue_path_abandon_frame(cnx, path_id, reason) == 0){
                    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = 
                        picoquic_find_or_create_remote_cnxid_stash(cnx, 
                            cnx->path[path_index]->unique_path_id, 0);
                    if (remote_cnxid_stash != NULL && path_index != 0) {
                        cnx->path[path_index]->first_tuple->p_remote_cnxid = NULL;
                        picoquic_delete_remote_cnxid_stash(cnx, remote_cnxid_stash);
                    }
                    else {
                        DBG_PRINTF("Cannot abandon path[%d]", cnx->path[path_index]->unique_path_id);
                    }
                    picoquic_log_app_message(cnx, "Abandon path, unique_id %" PRIu64", reason % " PRIu64,
                        cnx->path[path_index]->unique_path_id, reason);
                    cnx->path[path_index]->path_abandon_sent = 1;
                } else {
                    picoquic_log_app_message(cnx, "Cannot queue abandon path [%" PRIu64 "]",
                        cnx->path[path_index]->unique_path_id);
                }
            }
        }
    }
}
```

### Rust body
```rust
        if let Some(p) = self.paths.get_mut(idx) {
            p.path_is_demoted = true;
            p.demotion_time = current_time;
        }
```

## Pair `picoquic/quicctx.c:picoquic_find_path_by_unique_id`
C: `picoquic/quicctx.c:2203-2215 picoquic_find_path_by_unique_id`
Rust: `rs/fq/src/internal.rs:4833-4838 find_path_by_unique_id`

### C body
```c
{
    int path_index = -1;
    
    for (int i = 0; i < cnx->nb_paths; i++) {
        if (cnx->path[i]->unique_path_id == unique_path_id) {
            path_index = i;
            break;
        }
    }

    return path_index;
}
```

### Rust body
```rust
        for (i, p) in self.paths.iter().enumerate() {
            if p.unique_path_id == unique_path_id {
                return i as i32;
            }
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

## Pair `picoquic/quicctx.c:picoquic_verify_proposed_tuple`
C: `picoquic/quicctx.c:2385-2435 picoquic_verify_proposed_tuple`
Rust: `rs/fq/src/lib.rs:2482-2520 verify_proposed_tuple`

### C body
```c
{
    int ret = 0;
    struct sockaddr const* addr_peer = *p_addr_peer;
    struct sockaddr const* addr_local = *p_addr_local;
    int if_index = *p_if_index;

    /* verify that the peer and local addresses are correctly set */
    if (addr_peer == NULL || addr_peer->sa_family == 0) {
        if (addr_local == NULL || addr_local->sa_family == 0) {
            ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
        }
        else {
            /* Find the peer address from existing paths */
            for (int i = 0; i < cnx->nb_paths; i++) {
                if (cnx->path[i]->first_tuple->peer_addr.ss_family == addr_local->sa_family) {
                    addr_peer = (struct sockaddr*)&cnx->path[i]->first_tuple->peer_addr;
                    if_index = cnx->path[i]->first_tuple->if_index;
                    break;
                }
            }
            if (addr_peer == NULL || addr_peer->sa_family == 0) {
                ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
            }
        }
    }
    else if (addr_local == NULL || addr_local->sa_family == 0) {
        /* Find the local address from existing paths */
        for (int i = 0; i < cnx->nb_paths; i++) {
            if (cnx->path[i]->first_tuple->local_addr.ss_family == addr_peer->sa_family) {
                addr_local = (struct sockaddr*)&cnx->path[i]->first_tuple->local_addr;
                if_index = cnx->path[i]->first_tuple->if_index;
                break;
            }
        }
        if (addr_peer == NULL) {
            ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
        }
    }
    else if (addr_peer->sa_family != addr_local->sa_family) {
        ret = PICOQUIC_ERROR_PATH_ADDRESS_FAMILY;
    }

    if (ret == 0) {
        *p_addr_peer = addr_peer;
        *p_addr_local = addr_local;
        *p_if_index = if_index;
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(SocketAddr, SocketAddr, i32), Error> {
        let mut if_index = if_index;
        match (addr_peer, addr_local) {
            (None, None) => Err(Error::Generic),
            (None, Some(local)) => {
                let t = self
                    .paths
                    .iter()
                    .filter_map(|p| p.tuples.first())
                    .find(|t| t.peer_addr.is_ipv4() == local.is_ipv4())
                    .ok_or(Error::Generic)?;
                if_index = t.if_index as i32;
                Ok((t.peer_addr, local, if_index))
            }
            (Some(peer), None) => {
                // C: checks addr_peer == NULL after this loop (copy-paste error; addr_peer
                // is non-NULL in this branch).  Translate defensively: fail if no local found.
                let t = self
                    .paths
                    .iter()
                    .filter_map(|p| p.tuples.first())
                    .find(|t| t.local_addr.is_ipv4() == peer.is_ipv4())
                    .ok_or(Error::Generic)?;
                if_index = t.if_index as i32;
                Ok((peer, t.local_addr, if_index))
            }
            (Some(peer), Some(local)) => {
                if peer.is_ipv4() != local.is_ipv4() {
                    return Err(Error::InvalidArgument);
                }
                Ok((peer, local, if_index))
            }
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_enable_path_callbacks_default`
C: `picoquic/quicctx.c:2519-2523 picoquic_enable_path_callbacks_default`
Rust: `rs/fq/src/lib.rs:1712-1714 enable_path_callbacks_default`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->are_path_callbacks_enabled = are_enabled;
}
```

### Rust body
```rust
    pub fn enable_path_callbacks_default(&mut self, enabled: bool) {
        self.are_path_callbacks_enabled = enabled;
    }
```
