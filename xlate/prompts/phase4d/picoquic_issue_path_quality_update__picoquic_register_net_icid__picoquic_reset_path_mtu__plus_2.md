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

## `picoquic/quicctx.c:picoquic_issue_path_quality_update`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is visibly incomplete and does not implement the C threshold checks, refresh, callback, or return behavior.
* C source: `picoquic/quicctx.c:2660-2676`
* C signature: `int picoquic_issue_path_quality_update(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:6249-6252`
* Rust item: `issue_path_quality_update`

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

## `picoquic/quicctx.c:picoquic_register_net_icid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body stores the peer address then retrieves/inserts into a connection-by-icid hash table; Rust body only checks local membership/address state and stores the peer address, with no visible table insertion.
* C source: `picoquic/quicctx.c:1340-1354`
* C signature: `int picoquic_register_net_icid(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:4031-4045`
* Rust item: `register_net_icid`

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

## `picoquic/quicctx.c:picoquic_reset_path_mtu`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C resets MTU fields; Rust body is a get_path_id_from_unique implementation and touches unrelated path lookup logic.
* C source: `picoquic/quicctx.c:2851-2860`
* C signature: `void picoquic_reset_path_mtu(picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:4887-4904`
* Rust item: `reset_path_mtu`

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

## `picoquic/quicctx.c:picoquic_set_cwin_max`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets cwin_max with zero mapped to UINT64_MAX; Rust only returns the current cwin_max.
* C source: `picoquic/quicctx.c:964-968`
* C signature: `void picoquic_set_cwin_max(picoquic_quic_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:1726-1734`
* Rust item: `set_cwin_max`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->cwin_max = (cwin_max == 0) ? UINT64_MAX : cwin_max;
}
```

### Rust body
```rust
    pub fn cwin_max(&self) -> u64 {
        self.cwin_max
    }
```

## `picoquic/quicctx.c:picoquic_set_default_pmtud_policy`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets default_pmtud_policy; Rust body returns default_callback_fn instead.
* C source: `picoquic/quicctx.c:4551-4555`
* C signature: `void picoquic_set_default_pmtud_policy(picoquic_quic_t *, picoquic_pmtud_policy_enum)`
* Rust source: `rs/fq/src/lib.rs:3238-3246`
* Rust item: `set_default_pmtud_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_pmtud_policy = pmtud_policy;
}
```

### Rust body
```rust
    pub fn default_callback(&self) -> Option<&dyn StreamDataCallback> {
        self.default_callback_fn.as_deref()
    }
```
