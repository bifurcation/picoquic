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

## Pair `picoquic/quicctx.c:picoquic_set_default_handshake_timeout`
C: `picoquic/quicctx.c:998-1002 picoquic_set_default_handshake_timeout`
Rust: `rs/fq/src/lib.rs:1786-1788 set_default_handshake_timeout`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_handshake_timeout = handshake_timeout_us;
}
```

### Rust body
```rust
    pub fn set_default_handshake_timeout(&mut self, handshake_timeout: Duration) {
        self.default_handshake_timeout = handshake_timeout;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_crypto_epoch_length`
C: `picoquic/quicctx.c:1024-1028 picoquic_get_crypto_epoch_length`
Rust: `rs/fq/src/lib.rs:2870-2877 crypto_epoch_length`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->crypto_epoch_length_max;
}
```

### Rust body
```rust
    pub fn set_pmtud_policy(&mut self, pmtud_policy: PmtudPolicy) {
        self.pmtud_policy = pmtud_policy;
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_max_simultaneous_logs`
C: `picoquic/quicctx.c:1050-1054 picoquic_get_max_simultaneous_logs`
Rust: `rs/fq/src/lib.rs:1910-1912 max_simultaneous_logs`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->max_simultaneous_logs;
}
```

### Rust body
```rust
    pub fn max_simultaneous_logs(&self) -> u32 {
        self.max_simultaneous_logs
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_null_verifier`
C: `picoquic/quicctx.c:1182-1185 picoquic_set_null_verifier`
Rust: `rs/fq/src/lib.rs:1641-1643 set_null_verifier`

### C body
```c
void picoquic_set_null_verifier(picoquic_quic_t* quic) {
    PICOQUIC_THREAD_CHECK(quic);
    picoquic_dispose_verify_certificate_callback(quic);
}
```

### Rust body
```rust
    pub fn set_null_verifier(&mut self) {
        // Delegates to TLS backend; complex.
    }
```

## Pair `picoquic/quicctx.c:picoquic_create_stateless_packet`
C: `picoquic/quicctx.c:1218-1224 picoquic_create_stateless_packet`
Rust: `rs/fq/src/internal.rs:610-630 create_stateless_packet`

### C body
```c
{
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(quic);
#endif
    return (picoquic_stateless_packet_t*)malloc(sizeof(picoquic_stateless_packet_t));
}
```

### Rust body
```rust
    pub fn queue_stateless_packet(&mut self, sp: StatelessPacket) {
        self.pending_stateless_packets.push_back(sp);
    }
```

## Pair `picoquic/quicctx.c:picoquic_register_cnx_id`
C: `picoquic/quicctx.c:1263-1278 picoquic_register_cnx_id`
Rust: `rs/fq/src/internal.rs:3973-3990 register_cnx_id`

### C body
```c
{
    int ret = 0;
    picohash_item* item;

    item = picohash_retrieve(quic->table_cnx_by_id, l_cid);
    if (item != NULL) {
        ret = -1;
    } else {
        l_cid->registered_cnx = cnx;
        ret = picohash_insert(quic->table_cnx_by_id, l_cid);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        if self.connection_by_id.lookup(&l_cid.connection_id).is_some() {
            return Err(crate::Error::Generic);
        }
        let token = self
            .connections
            .iter()
            .position(|c| c.initial_connection_id == connection.initial_connection_id)
            .map(|idx| ConnectionToken::synthetic(idx as u32, idx as u32))
            .ok_or(crate::Error::Generic)?;
        let (ht, _) = self.connection_by_id.insert(l_cid.connection_id, token)?;
        l_cid.connection_by_id_membership = Some(ht);
        Ok(())
    }
```

## Pair `picoquic/quicctx.c:picoquic_unregister_net_icid`
C: `picoquic/quicctx.c:1356-1363 picoquic_unregister_net_icid`
Rust: `rs/fq/src/lib.rs:2128-2140 unregister_net_icid`

### C body
```c
{
    if (cnx->registered_icid_item.key != 0) {
        picohash_delete_item(cnx->quic->table_cnx_by_icid, &cnx->registered_icid_item, 0);
        memset(&cnx->registered_icid_addr, 0, sizeof(struct sockaddr_storage));
        memset(&cnx->registered_icid_item, 0, sizeof(picohash_item));
    }
}
```

### Rust body
```rust
    pub fn unregister_net_icid(&mut self, connection: ConnectionToken) {
        let membership = self
            .connections
            .get(connection)
            .and_then(|cnx| cnx.connection_by_icid_membership);
        if let Some(membership) = membership {
            self.connection_by_icid.remove(membership);
        }
        if let Some(cnx) = self.connections.get_mut(connection) {
            cnx.registered_icid_addr = unspecified_socket_addr();
            cnx.connection_by_icid_membership = None;
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_quic_ctx`
C: `picoquic/quicctx.c:1419-1422 picoquic_get_quic_ctx`
Rust: `rs/fq/src/lib.rs:2804-2809 quic`

### C body
```c
{
    return (cnx == NULL)?NULL:cnx->quic;
}
```

### Rust body
```rust
    pub unsafe fn quic(&mut self) -> &mut Quic {
        debug_assert!(!self.quic_ptr.is_null(), "quic_ptr not initialised");
        // SAFETY: quic_ptr is set to `self as *mut Quic` in
        // create_cnx_internal and remains valid for the connection's lifetime.
        unsafe { &mut *self.quic_ptr }
    }
```

## Pair `picoquic/quicctx.c:picoquic_remove_cnx_from_list`
C: `picoquic/quicctx.c:1450-1469 picoquic_remove_cnx_from_list`
Rust: `rs/fq/src/lib.rs:2167-2173 remove_cnx_from_list`

### C body
```c
{
    if (cnx->next_in_table == NULL) {
        cnx->quic->cnx_last = cnx->previous_in_table;
    } else {
        cnx->next_in_table->previous_in_table = cnx->previous_in_table;
    }

    if (cnx->previous_in_table == NULL) {
        cnx->quic->cnx_list = cnx->next_in_table;
    }
    else {
        cnx->previous_in_table->next_in_table = cnx->next_in_table;
    }

    picoquic_unregister_net_icid(cnx);
    picoquic_unregister_net_secret(cnx);

    cnx->quic->current_number_connections--;
}
```

### Rust body
```rust
    pub(crate) fn remove_cnx_from_list(&mut self, connection: ConnectionToken) {
        self.unregister_net_icid(connection);
        self.unregister_net_secret(connection);
        if self.current_number_connections > 0 {
            self.current_number_connections -= 1;
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_reinsert_by_wake_time`
C: `picoquic/quicctx.c:1515-1520 picoquic_reinsert_by_wake_time`
Rust: `rs/fq/src/internal.rs:6278-6297 reinsert_by_wake_time`

### C body
```c
{
    picoquic_remove_cnx_from_wake_list(cnx);
    cnx->next_wake_time = next_time;
    picoquic_insert_cnx_by_wake_time(quic, cnx);
}
```

### Rust body
```rust
    pub fn reinsert_by_wake_time(&mut self, connection: &mut Connection, next_time: Instant) {
        connection.next_wake_time = next_time;
        let token = self
            .connections
            .iter()
            .position(|c| c.initial_connection_id == connection.initial_connection_id)
            .map(|idx| ConnectionToken::synthetic(idx as u32, idx as u32));
        if let Some(token) = token
            && let Ok((tree_token, old)) =
                self.connection_wake_tree.insert(next_time.ticks(), token)
        {
            connection.connection_wake_membership = Some(tree_token);
            if let Some(old_token) = old
                && old_token != token
                && let Some(old_connection) = self.connections.get_mut(old_token)
            {
                old_connection.connection_wake_membership = None;
            }
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_wake_time`
C: `picoquic/quicctx.c:1579-1591 picoquic_get_wake_time`
Rust: `rs/fq/src/lib.rs:3222-3228 earliest_cnx_to_wake`

### C body
```c
{
    uint64_t wake_time = UINT64_MAX;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx->quic->pending_stateless_packet != NULL) {
        wake_time = current_time;
    } else {
        wake_time = cnx->next_wake_time;
    }

    return wake_time;
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

## Pair `picoquic/quicctx.c:picoquic_create_local_cnx_id`
C: `picoquic/quicctx.c:1641-1649 picoquic_create_local_cnx_id`
Rust: `rs/fq/src/internal.rs:4049-4059 create_local_cnx_id`

### C body
```c
{
    /* First call fills the CID with a random value */
    picoquic_create_random_cnx_id(quic, cnx_id, quic->local_cnxid_length);
    /* if required for application, call to function update that definition */
    if (quic->cnx_id_callback_fn) {
        quic->cnx_id_callback_fn(quic, *cnx_id, cnx_id_remote, quic->cnx_id_callback_ctx, cnx_id);
    }
}
```

### Rust body
```rust
    pub fn create_local_cnx_id(&mut self, cnx_id: &mut ConnectionId, cnx_id_remote: ConnectionId) {
        let len = self.local_connection_id_length as usize;
        let mut generated = ConnectionId::with_size(len).unwrap_or_default();
        rand_core::RngCore::fill_bytes(&mut *self.rng, generated.as_bytes_mut());
        if let Some(mut cb) = self.connection_id_callback_fn.take() {
            *cnx_id = cb.produce(self, generated, cnx_id_remote);
            self.connection_id_callback_fn = Some(cb);
        } else {
            *cnx_id = generated;
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_last_packet`
C: `picoquic/quicctx.c:1700-1704 picoquic_get_last_packet`
Rust: `rs/fq/src/internal.rs:7850-7875 get_last_packet`

### C body
```c
{
    return (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) ? path_x->pkt_ctx.pending_last :
        cnx->pkt_ctx[pc].pending_last;
}
```

### Rust body
```rust
impl Connection {
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
}
```

## Pair `picoquic/quicctx.c:picoquic_set_first_tuple`
C: `picoquic/quicctx.c:1766-1772 picoquic_set_first_tuple`
Rust: `rs/fq/src/internal.rs:4137-4141 set_first_tuple`

### C body
```c
{
    picoquic_tuple_t* old_first = path_x->first_tuple;
    picoquic_unchain_tuple(path_x, tuple);
    path_x->first_tuple = tuple;
    tuple->next_tuple = old_first;
}
```

### Rust body
```rust
        if index > 0 && index < self.tuples.len() {
            let t = self.tuples.remove(index);
            self.tuples.insert(0, t);
        }
```

## Pair `picoquic/quicctx.c:picoquic_clear_path_data`
C: `picoquic/quicctx.c:1885-1899 picoquic_clear_path_data`
Rust: `rs/fq/src/lib.rs:2070-2094 clear_path_data`

### C body
```c
{
    picoquic_unregister_net_id(cnx, path_x);
    /* Remove the congestion data */
    if (cnx->congestion_alg != NULL) {
        cnx->congestion_alg->alg_delete(path_x);
    }
    /* Remove the list of tuples */
    while (path_x->first_tuple != NULL) {
        picoquic_delete_tuple(path_x, path_x->first_tuple, 1);
    }

    /* Free the record */
    free(path_x);
}
```

### Rust body
```rust
    pub(crate) fn clear_path_data(&mut self, connection: ConnectionToken, path_index: usize) {
        self.unregister_net_id(connection, path_index);

        let congestion_alg = self
            .connections
            .get(connection)
            .and_then(|cnx| cnx.congestion_alg);

        if let Some(cnx) = self.connections.get_mut(connection)
            && let Some(path) = cnx.paths.get_mut(path_index)
        {
            if let Some(alg) = congestion_alg {
                alg.algorithm.alg_delete(path);
            }
            while !path.tuples.is_empty() {
                path.delete_tuple(0, true);
            }
        }

        if let Some(cnx) = self.connections.get_mut(connection)
            && path_index < cnx.paths.len()
        {
            cnx.paths.remove(path_index);
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_set_tuple_challenge`
C: `picoquic/quicctx.c:2118-2134 picoquic_set_tuple_challenge`
Rust: `rs/fq/src/internal.rs:4763-4777 set_tuple_challenge`

### C body
```c
{

    /* Reset the tuple challenge */
    tuple->challenge_time_first = current_time;
    for (int ichal = 0; ichal < PICOQUIC_CHALLENGE_REPEAT_MAX; ichal++) {
        if (use_constant_challenges) {
            tuple->challenge[ichal] = current_time * (0xdeadbeefull + ichal);
        }
        else {
            tuple->challenge[ichal] = picoquic_public_random_64();
        }
    }
    tuple->challenge_time = current_time;
    tuple->challenge_repeat_count = 0;
}
```

### Rust body
```rust
pub fn set_tuple_challenge(tuple: &mut Tuple, current_time: Instant, use_constant_challenges: i32) {
    // C: picoquic_set_tuple_challenge
    tuple.challenge_time_first = current_time;
    for ichal in 0..CHALLENGE_REPEAT_MAX {
        tuple.challenge[ichal] = if use_constant_challenges != 0 {
            current_time
                .ticks()
                .wrapping_mul(0xdeadbeef_u64.wrapping_add(ichal as u64))
        } else {
            crate::public_random_64()
        };
    }
    tuple.challenge_time = current_time;
    tuple.challenge_repeat_count = 0;
}
```

## Pair `picoquic/quicctx.c:picoquic_notify_destination_unreachable`
C: `picoquic/quicctx.c:2217-2244 picoquic_notify_destination_unreachable`
Rust: `rs/fq/src/lib.rs:3715-3736 notify_destination_unreachable`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx != NULL && addr_peer != NULL) {
        int no_path_left = 1;
        int partial_match = 0;
        int path_id = picoquic_find_path_by_address(cnx, addr_local, addr_peer, &partial_match);

        if (path_id >= 0) {
            for (int i = 0; no_path_left && i < cnx->nb_paths; i++) {
                no_path_left &= !cnx->path[i]->path_is_demoted;         
            }
            if (no_path_left) {
                /* Caution here: ICMP packets could be forged */
                if (cnx->cnx_state == picoquic_state_ready) {
                    picoquic_set_path_challenge(cnx, path_id, current_time);
                }
            }
            else {
                picoquic_log_app_message(cnx, "Demoting path %d after socket error %d, if %d", path_id, socket_err, if_index);
                picoquic_demote_path(cnx, path_id, current_time, 0);
            }
        }
    }
}
```

### Rust body
```rust
    ) {
        let mut partial_match = 0;
        let path_id =
            self.find_path_by_address(Some(addr_local), Some(addr_peer), &mut partial_match);
        if path_id >= 0 {
            let no_path_left = self.paths.iter().all(|path| !path.path_is_demoted);
            if no_path_left {
                if self.connection_state == State::Ready {
                    self.set_path_challenge(path_id, current_time);
                }
            } else {
                self.demote_path(path_id, current_time, 0);
            }
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_check_new_path_allowed`
C: `picoquic/quicctx.c:2300-2342 picoquic_check_new_path_allowed`
Rust: `rs/fq/src/lib.rs:2594-2625 check_new_path_allowed`

### C body
```c
{
    int ret = 0;

    if ((cnx->remote_parameters.migration_disabled && !to_preferred_address) ||
        cnx->local_parameters.migration_disabled) {
        /* Do not create new paths if migration is disabled */
        DBG_PRINTF("Tried to create probe with migration disabled = %d", cnx->remote_parameters.migration_disabled);
        ret = PICOQUIC_ERROR_MIGRATION_DISABLED;
    }
    else if (cnx->cnx_state < picoquic_state_client_almost_ready) {
        ret = PICOQUIC_ERROR_PATH_NOT_READY;
    }
    else if (cnx->nb_paths >= PICOQUIC_NB_PATH_TARGET) {
        /* Too many paths created already */
        ret = PICOQUIC_ERROR_PATH_LIMIT_EXCEEDED;
    }
    else {
        /* testing availability of connection ID is sufficient.
         * If multipath is enabled, connection IDs will
         * only be received if both peers have negotiated a sufficient path ID.
         * In any case, connection IDs can only be received if the connection
         * is almost ready.
         */
        uint64_t unique_path_id = 0;
        if (cnx->is_multipath_enabled) {
            unique_path_id = cnx->unique_path_id_next;
        }
        if (picoquic_obtain_stashed_cnxid(cnx, unique_path_id) == NULL) {
            if (cnx->unique_path_id_next > cnx->max_path_id_remote) {
                ret = PICOQUIC_ERROR_PATH_ID_BLOCKED;
            }
            else
            {
                ret = PICOQUIC_ERROR_PATH_CID_BLOCKED;
            }
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn check_new_path_allowed(&self, to_preferred_address: bool) -> Result<(), Error> {
        if (self.remote_parameters.migration_disabled && !to_preferred_address)
            || self.local_parameters.migration_disabled
        {
            return Err(Error::Protocol(InternalError::MigrationDisabled as u64));
        }
        if self.connection_state < State::ClientAlmostReady {
            return Err(Error::Protocol(InternalError::PathNotReady as u64));
        }
        if self.paths.len() >= crate::internal::NB_PATH_TARGET {
            return Err(Error::Protocol(InternalError::PathLimitExceeded as u64));
        }

        let unique_path_id = if self.is_multipath_enabled {
            self.unique_path_id_next
        } else {
            0
        };
        let has_available_cid = self
            .remote_connection_id_stashes
            .iter()
            .find(|stash| stash.unique_path_id == unique_path_id)
            .and_then(|stash| stash.get_connection_id_from_stash())
            .is_some();
        if has_available_cid {
            Ok(())
        } else if self.unique_path_id_next > self.max_path_id_remote {
            Err(Error::Protocol(InternalError::PathIdBlocked as u64))
        } else {
            Err(Error::Protocol(InternalError::PathCidBlocked as u64))
        }
    }
```

## Pair `picoquic/quicctx.c:picoquic_probe_new_tuple`
C: `picoquic/quicctx.c:2437-2466 picoquic_probe_new_tuple`
Rust: `rs/fq/src/lib.rs:2462-2473 probe_new_tuple`

### C body
```c
{
    int ret;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    ret = picoquic_verify_proposed_tuple(cnx, &addr_peer, &addr_local, &if_index);

    /* TODO: check whether that tuple already exists */

    /* Verify that a CID is available */
    ret = picoquic_check_cid_for_new_tuple(cnx, path_x->unique_path_id);

    if (ret == 0) {
        picoquic_tuple_t * tuple = picoquic_create_tuple(path_x, addr_local, addr_peer, if_index);
        if (tuple == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
        else {
            ret = picoquic_assign_peer_cnxid_to_tuple(cnx, path_x, tuple);
            if (ret == 0) {
                /* There was no NAT ongoing NAT rebinding, we created one, we need to initiate path challenges. */
                picoquic_set_tuple_challenge(tuple, current_time, cnx->quic->use_constant_challenges);
                tuple->challenge_required = 1;
                tuple->to_preferred_address = to_preferred_address;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // Complex: involves tuple creation within a path.
        Err(Error::Generic)
    }
```

## Pair `picoquic/quicctx.c:picoquic_get_path_id_from_unique`
C: `picoquic/quicctx.c:2525-2538 picoquic_get_path_id_from_unique`
Rust: `rs/fq/src/internal.rs:4896-4902 get_path_id_from_unique`

### C body
```c
{
    int ret = -1;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    for (int i = 0; i < cnx->nb_paths; i++) {
        if (cnx->path[i]->unique_path_id == unique_path_id) {
            ret = i;
            break;
        }
    }

    return ret;
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
