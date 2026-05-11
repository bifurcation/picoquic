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

## Pair `picoquic/quicctx.c:picoquic_set_default_crypto_epoch_length`
C: `picoquic/quicctx.c:1004-1009 picoquic_set_default_crypto_epoch_length`
Rust: `rs/fq/src/lib.rs:1792-1799 set_default_crypto_epoch_length`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->crypto_epoch_length_max = (crypto_epoch_length_max == 0) ?
        PICOQUIC_DEFAULT_CRYPTO_EPOCH_LENGTH : crypto_epoch_length_max;
}
```

### Rust body
```rust
    pub fn default_crypto_epoch_length(&self) -> u64 {
        self.crypto_epoch_length_max
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_local_cid_length`
C: `picoquic/quicctx.c:1031-1035 picoquic_get_local_cid_length`
Rust: `rs/fq/src/lib.rs:1803-1810 local_cid_length`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->local_cnxid_length;
}
```

### Rust body
```rust
    pub fn is_local_cid(&self, cid: &ConnectionId) -> bool {
        self.connection_by_id.contains_key(cid)
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_default_bdp_frame_option`
C: `picoquic/quicctx.c:1056-1060 picoquic_set_default_bdp_frame_option`
Rust: `rs/fq/src/lib.rs:1832-1850 set_default_bdp_frame_option`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_send_receive_bdp_frame = bdp_option;
}
```

### Rust body
```rust
    pub fn set_default_connection_id_length(&mut self, cid_length: u8) -> Result<(), Error> {
        if cid_length as usize > CONNECTION_ID_MAX_SIZE {
            return Err(Error::Protocol(InternalError::CnxidCheck as u64));
        }
        if self.current_number_connections > 0 {
            return Err(Error::Protocol(
                InternalError::CannotChangeActiveContext as u64,
            ));
        }
        self.local_connection_id_length = cid_length;
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_cookie_mode`
C: `picoquic/quicctx.c:1187-1204 picoquic_set_cookie_mode`
Rust: `rs/fq/src/lib.rs:1549-1553 set_cookie_mode`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    if (cookie_mode&1) {
        quic->force_check_token = 1;
    } else {
        quic->force_check_token = 0;
    }

    if (cookie_mode & 2) {
        quic->provide_token = 1;
    }
    else {
        quic->provide_token = 0;
    }

    quic->check_token = (quic->force_check_token || quic->max_half_open_before_retry <= quic->current_number_half_open);
}
```

### Rust body
```rust
    pub fn set_cookie_mode(&mut self, cookie_mode: i32) {
        self.check_token = (cookie_mode & 1) != 0;
        self.force_check_token = (cookie_mode & 2) != 0;
        self.provide_token = (cookie_mode & 4) != 0;
    }
```

## Pair `picoquic/quicctx.c:picoquic_queue_stateless_packet`
C: `picoquic/quicctx.c:1231-1241 picoquic_queue_stateless_packet`
Rust: `rs/fq/src/internal.rs:628-630 queue_stateless_packet`

### C body
```c
{
    picoquic_stateless_packet_t** pnext = &quic->pending_stateless_packet;

    while ((*pnext) != NULL) {
        pnext = &(*pnext)->next_packet;
    }

    *pnext = sp;
    sp->next_packet = NULL;
}
```

### Rust body
```rust
    pub fn queue_stateless_packet(&mut self, sp: StatelessPacket) {
        self.pending_stateless_packets.push_back(sp);
    }
```

## Pair `picoquic/quicctx.c:picoquic_unregister_net_id`
C: `picoquic/quicctx.c:1280-1290 picoquic_unregister_net_id`
Rust: `rs/fq/src/lib.rs:2007-2013 unregister_net_id`

### C body
```c
{
    if (path_x->net_id_hash_item.key != NULL) {
        picohash_item* item = picohash_retrieve(cnx->quic->table_cnx_by_net, path_x);
        if (item != NULL) {
            picohash_delete_item(cnx->quic->table_cnx_by_net, item, 0);
        }
        memset(&path_x->registered_peer_addr, 0, sizeof(struct sockaddr_storage));
        memset(&path_x->net_id_hash_item, 0, sizeof(path_x->net_id_hash_item));
    }
}
```

### Rust body
```rust
            self.connections.get(connection).and_then(|cnx| {
                cnx.paths
                    .get(path_index)
                    .map(|path| (path.connection_by_net_membership, path.registered_peer_addr))
            })
```

## Pair `picoquic/quicctx.c:picoquic_unregister_net_secret`
C: `picoquic/quicctx.c:1365-1372 picoquic_unregister_net_secret`
Rust: `rs/fq/src/lib.rs:2144-2157 unregister_net_secret`

### C body
```c
{
    if (cnx->registered_secret_addr.ss_family != 0) {
        picohash_delete_key(cnx->quic->table_cnx_by_secret, cnx, 0);
        memset(&cnx->registered_secret_addr, 0, sizeof(struct sockaddr_storage));
        memset(&cnx->registered_reset_secret, 0, sizeof(PICOQUIC_RESET_SECRET_SIZE));
    }
}
```

### Rust body
```rust
    pub fn unregister_net_secret(&mut self, connection: ConnectionToken) {
        let membership = self
            .connections
            .get(connection)
            .and_then(|cnx| cnx.connection_by_secret_membership);
        if let Some(membership) = membership {
            self.connection_by_secret.remove(membership);
        }
        if let Some(cnx) = self.connections.get_mut(connection) {
            cnx.registered_secret_addr = unspecified_socket_addr();
            cnx.registered_reset_secret = [0; RESET_SECRET_SIZE];
            cnx.connection_by_secret_membership = None;
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_first_cnx`
C: `picoquic/quicctx.c:1424-1428 picoquic_get_first_cnx`
Rust: `rs/fq/src/lib.rs:3165-3167 first_connection`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->cnx_list;
}
```

### Rust body
```rust
    pub fn first_connection(&mut self) -> Option<&mut Connection> {
        self.connections.iter_mut().next()
    }
```

## Pair `picoquic/quicctx.c:picoquic_wake_list_init`
C: `picoquic/quicctx.c:1499-1503 picoquic_wake_list_init`
Rust: `rs/fq/src/lib.rs:3368-3373 wake_list_init`

### C body
```c
{
    picosplay_init_tree(&quic->cnx_wake_tree, picoquic_wake_list_compare,
        picoquic_wake_list_create_node, picoquic_wake_list_delete_node, picoquic_wake_list_node_value);
}
```

### Rust body
```rust
    fn wake_list_init(&mut self) {
        self.connection_wake_tree = crate::splay::SplayTree::new();
        for cnx in self.connections.iter_mut() {
            cnx.connection_wake_membership = None;
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_earliest_cnx_to_wake`
C: `picoquic/quicctx.c:1522-1531 picoquic_get_earliest_cnx_to_wake`
Rust: `rs/fq/src/lib.rs:3222-3228 earliest_cnx_to_wake`

### C body
```c
{
    picoquic_cnx_t* cnx = (picoquic_cnx_t *)picoquic_wake_list_node_value(picosplay_first(&quic->cnx_wake_tree));
    if (cnx != NULL && max_wake_time != 0 && cnx->next_wake_time > max_wake_time)
    {
        cnx = NULL;
    }

    return cnx;
}
```

### Rust body
```rust
    pub fn earliest_cnx_to_wake(&mut self, wake_time: Instant) -> Option<&mut Connection> {
        let threshold = wake_time.ticks();
        self.connections
            .iter_mut()
            .filter(|c| c.next_wake_time.ticks() <= threshold)
            .min_by_key(|c| c.next_wake_time.ticks())
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_wake_delay`
C: `picoquic/quicctx.c:1593-1612 picoquic_get_wake_delay`
Rust: `rs/fq/src/lib.rs:2822-2831 wake_delay`

### C body
```c
{
    /* See get_next_wake_delay for reasoning about integer overflow */
    uint64_t next_wake_time = picoquic_get_wake_time(cnx, current_time);
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
    pub fn wake_delay(&self, current_time: Instant, delay_max: i64) -> i64 {
        let next = self.next_wake_time.ticks();
        let now = current_time.ticks();
        if next <= now {
            0
        } else {
            let delta = (next - now) as i64;
            delta.min(delay_max)
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_find_avalaible_unique_path_id`
C: `picoquic/quicctx.c:1651-1684 picoquic_find_avalaible_unique_path_id`
Rust: `rs/fq/src/lib.rs:4937-4945 path_unique_id`

### C body
```c
{
    uint64_t unique_path_id = requested_id;

    if (!cnx->is_multipath_enabled) {
        if (requested_id != 0 && requested_id != UINT64_MAX) {
            unique_path_id = UINT64_MAX;
        }
        else {
            unique_path_id = 0;
        }
    }
    else {
        /* Unique path ID are allocated in sequence on the client. The server should
         * always use the number proposed by the client in incoming packets */
        if (requested_id == UINT64_MAX && (cnx->client_mode || cnx->nb_paths == 0)) {
            while (cnx->unique_path_id_next <= cnx->max_path_id_remote &&
                cnx->unique_path_id_next <= cnx->max_path_id_local &&
                cnx->unique_path_id_next <= cnx->max_path_id_in_cnxid_lists) {
                /* Find next non used CID */
                unique_path_id = cnx->unique_path_id_next++;
                /* There should be an available of CNX_ID for this path_id, 
                * and that path_id should not be already created.
                */
                if (picoquic_find_or_create_local_cnxid_list(cnx, unique_path_id, 0) != NULL &&
                    picoquic_find_path_by_unique_id(cnx, unique_path_id) < 0) {
                    /* this CID was not already deleted */
                    break;
                }
             }
        }
    }
    return unique_path_id;
}
```

### Rust body
```rust
    pub fn nb_crypto_key_rotations(&self) -> u64 {
        self.nb_crypto_key_rotations
    }
```

## Pair `picoquic/quicctx.c:picoquic_create_tuple`
C: `picoquic/quicctx.c:1706-1731 picoquic_create_tuple`
Rust: `rs/fq/src/internal.rs:4068-4103 create_tuple`

### C body
```c
{
    picoquic_tuple_t* tuple = (picoquic_tuple_t*)malloc(sizeof(picoquic_tuple_t));
    if (tuple != NULL) {
        memset(tuple, 0, sizeof(picoquic_tuple_t));
        /* Add the tuple to the path */
        if (path_x->first_tuple == 0) {
            path_x->first_tuple = tuple;
        }
        else {
            picoquic_tuple_t* next = path_x->first_tuple;
            while (next->next_tuple != NULL) {
                next = next->next_tuple;
            }
            next->next_tuple = tuple;
        }
        /* Set the addresses */
        tuple->if_index = if_index;
        picoquic_store_addr(&tuple->peer_addr, peer_addr);
        picoquic_store_addr(&tuple->local_addr, local_addr);
    }
    return tuple;
}
```

### Rust body
```rust
    ) -> Result<usize, crate::Error> {
        use core::net::{IpAddr, Ipv4Addr};
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let t = Tuple {
            unique_path_id: self.unique_path_id,
            peer_addr: peer_addr.copied().unwrap_or(default_addr),
            local_addr: local_addr.copied().unwrap_or(default_addr),
            if_index: if_index as core::ffi::c_ulong,
            observed_addr: default_addr,
            remote_connection_id_index: None,
            local_connection_id: None,
            nb_observed_repeat: 0,
            observed_time: crate::Instant::from_ticks(0),
            challenge_response: 0,
            challenge: [0u64; CHALLENGE_REPEAT_MAX],
            challenge_time: crate::Instant::from_ticks(0),
            demotion_time: crate::Instant::from_ticks(0),
            challenge_time_first: crate::Instant::from_ticks(0),
            is_nat_rebinding: 0,
            challenge_repeat_count: 0,
            is_backup: 0,
            challenge_required: false,
            challenge_verified: false,
            challenge_failed: false,
            response_required: false,
            to_preferred_address: false,
        };
        let idx = self.tuples.len();
        self.tuples.push(t);
        Ok(idx)
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_first_if_index`
C: `picoquic/quicctx.c:1774-1784 picoquic_set_first_if_index`
Rust: `rs/fq/src/lib.rs:2661-2669 set_first_if_index`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx->cnx_state == picoquic_state_client_init) {
        cnx->path[0]->first_tuple->if_index = if_index;
    }
    return ret;
}
```

### Rust body
```rust
    pub fn set_first_if_index(&mut self, if_index: u32) -> Result<(), Error> {
        if let Some(path) = self.paths.first_mut()
            && let Some(tuple) = path.tuples.first_mut()
        {
            tuple.if_index = if_index as core::ffi::c_ulong;
            return Ok(());
        }
        Err(Error::InvalidArgument)
    }
```

## Pair `picoquic/quicctx.c:picoquic_delete_path`
C: `picoquic/quicctx.c:1901-1961 picoquic_delete_path`
Rust: `rs/fq/src/internal.rs:4717-4721 delete_path`

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

## Pair `picoquic/quicctx.c:picoquic_set_path_challenge`
C: `picoquic/quicctx.c:2136-2151 picoquic_set_path_challenge`
Rust: `rs/fq/src/internal.rs:4781-4790 set_path_challenge`

### C body
```c
{
    if (!cnx->path[path_id]->first_tuple->challenge_required || cnx->path[path_id]->first_tuple->challenge_verified) {
        /* Reset the path challenge */
        cnx->path[path_id]->first_tuple->challenge_required = 1;
        picoquic_set_tuple_challenge(cnx->path[path_id]->first_tuple, current_time, cnx->quic->use_constant_challenges);
        if (cnx->path[path_id]->first_tuple->challenge_verified && cnx->are_path_callbacks_enabled && cnx->callback_fn != NULL) {
            if (cnx->callback_fn(cnx, cnx->path[path_id]->unique_path_id, NULL, 0, picoquic_callback_path_suspended,
                cnx->callback_ctx, cnx->path[path_id]->app_path_ctx) != 0) {
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, picoquic_frame_type_path_challenge);
            }
        }
        cnx->path[path_id]->first_tuple->challenge_verified = 0;
    }
}
```

### Rust body
```rust
        {
            tuple.challenge_required = true;
            set_tuple_challenge(tuple, current_time, 0);
            tuple.challenge_verified = false;
        }
```

## Pair `picoquic/quicctx.c:picoquic_notify_destination_unreachable_by_cnxid`
C: `picoquic/quicctx.c:2246-2262 picoquic_notify_destination_unreachable_by_cnxid`
Rust: `rs/fq/src/lib.rs:3745-3756 notify_destination_unreachable_by_connection_id`

### C body
```c
{
    picoquic_cnx_t* cnx = NULL;
    PICOQUIC_THREAD_CHECK(quic);

    if (quic->local_cnxid_length == 0 || cnxid->id_len == 0) {
        cnx = picoquic_cnx_by_net(quic, addr_peer);
    }
    else if (cnxid->id_len == quic->local_cnxid_length) {
        cnx = picoquic_cnx_by_id(quic, *cnxid, NULL);
    }

    if (cnx != NULL) {
        picoquic_notify_destination_unreachable(cnx, current_time, addr_peer, addr_local, if_index, socket_err);
    }
}
```

### Rust body
```rust
        let connection = if self.local_connection_id_length == 0 || connection_id.is_empty() {
            self.connection_by_net(Some(addr_peer))
        } else if connection_id.len() == self.local_connection_id_length as usize {
```

## Pair `picoquic/quicctx.c:picoquic_subscribe_new_path_allowed`
C: `picoquic/quicctx.c:2344-2368 picoquic_subscribe_new_path_allowed`
Rust: `rs/fq/src/lib.rs:2634-2651 subscribe_new_path_allowed`

### C body
```c
{
    int ret;
    PICOQUIC_THREAD_CHECK(cnx->quic);
        
    ret = picoquic_check_new_path_allowed(cnx, 0);

    *is_already_allowed = 0;
    if (ret == 0) {
        /* is allowed. Just say so -- get return code. */
        *is_already_allowed = 1;
        cnx->is_subscribed_to_path_allowed = 0;
        cnx->is_notified_that_path_is_allowed = 0;
    }
    else if (ret == PICOQUIC_ERROR_PATH_NOT_READY ||
        ret == PICOQUIC_ERROR_PATH_LIMIT_EXCEEDED ||
        ret == PICOQUIC_ERROR_PATH_ID_BLOCKED ||
        ret == PICOQUIC_ERROR_PATH_CID_BLOCKED) {
        /* transient error. Subscribe to the event and return 0 */
        cnx->is_subscribed_to_path_allowed = 1;
        cnx->is_notified_that_path_is_allowed = 0;
        ret = 0;
    }
    return ret;
}
```

### Rust body
```rust
        match self.check_new_path_allowed(false) {
            Ok(()) => {
                self.is_subscribed_to_path_allowed = false;
                Ok(true)
            }
            Err(Error::Protocol(code))
                if code == InternalError::PathNotReady as u64
                    || code == InternalError::PathLimitExceeded as u64
                    || code == InternalError::PathIdBlocked as u64
                    || code == InternalError::PathCidBlocked as u64 =>
            {
                self.is_subscribed_to_path_allowed = true;
                Ok(false)
            }
            Err(e) => Err(e),
        }
```

## Pair `picoquic/quicctx.c:picoquic_probe_new_path_ex`
C: `picoquic/quicctx.c:2468-2509 picoquic_probe_new_path_ex`
Rust: `rs/fq/src/lib.rs:2448-2458 probe_new_path_ex`

### C body
```c
{
    int path_id = -1;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (!cnx->is_multipath_enabled || to_preferred_address) {
        return picoquic_probe_new_tuple(cnx, cnx->path[0], addr_peer, addr_local, if_index, current_time, to_preferred_address);
    }

    int ret = picoquic_check_new_path_allowed(cnx, to_preferred_address);

    if (ret == 0) {
        /* verify that the peer and local addresses are correctly set */
        ret = picoquic_verify_proposed_tuple(cnx, &addr_peer, &addr_local, &if_index);
    }

    if (ret == 0) {
        if (picoquic_create_path(cnx, current_time, addr_local, addr_peer, if_index, UINT64_MAX) > 0) {
            path_id = cnx->nb_paths - 1;
            picoquic_path_t* path_x = cnx->path[path_id];
            ret = picoquic_assign_peer_cnxid_to_tuple(cnx, path_x, path_x->first_tuple);

            if (ret != 0) {
                /* delete the path that was just created! */
                picoquic_delete_path(cnx, path_id);
            }
            else {
                path_x->path_is_published = 1;
                picoquic_register_path(cnx, path_x);
                picoquic_set_path_challenge(cnx, path_id, current_time);
                path_x->is_nat_challenge = 0;
                // path_x->first_tuple->if_index = if_index;
            }
        }
        else {
            ret = PICOQUIC_ERROR_MEMORY;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves path creation and network probing.
        Err(Error::Generic)
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_app_path_ctx`
C: `picoquic/quicctx.c:2540-2550 picoquic_set_app_path_ctx`
Rust: `rs/fq/src/lib.rs:2530-2545 set_app_path_ctx`

### C body
```c
{
    int ret = 0;
    int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        cnx->path[path_id]->app_path_ctx = app_path_ctx;
    } else {
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
            path.app_path_ctx = app_path_ctx;
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }
```
