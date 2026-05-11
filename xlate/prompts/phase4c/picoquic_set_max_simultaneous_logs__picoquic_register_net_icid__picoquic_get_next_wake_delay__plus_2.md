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
