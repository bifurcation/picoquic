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

## `picoquic/quicctx.c:picoquic_set_mtu_max`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets mtu_max and default max_packet_size; Rust body sets ALPN selection fields.
* C source: `picoquic/quicctx.c:4720-4725`
* C signature: `void picoquic_set_mtu_max(picoquic_quic_t *, uint32_t)`
* Rust source: `rs/fq/src/lib.rs:1864-1878`
* Rust item: `set_mtu_max`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->mtu_max = mtu_max;
    quic->default_tp.max_packet_size = mtu_max;
}
```

### Rust body
```rust
    pub fn set_alpn_select_fn(&mut self, alpn_select_fn: Option<Box<dyn AlpnSelect>>) {
        self.default_alpn = None;
        self.alpn_select_fn = alpn_select_fn;
    }
```

## `picoquic/quicctx.c:picoquic_set_stream_path_affinity`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust is placeholder-like and always returns Err(Error::Generic), while C performs stream lookup, clears affinity for UINT64_MAX, or sets affinity to a matching path.
* C source: `picoquic/quicctx.c:2779-2799`
* C signature: `int picoquic_set_stream_path_affinity(picoquic_cnx_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:2565-2572`
* Rust item: `set_stream_path_affinity`

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

## `picoquic/quicctx.c:picoquic_subscribe_pacing_rate_updates`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C stores pacing threshold arguments and request flag; Rust body shown only returns current pacing rate.
* C source: `picoquic/quicctx.c:5418-5424`
* C signature: `void picoquic_subscribe_pacing_rate_updates(picoquic_cnx_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:4650-4663`
* Rust item: `subscribe_pacing_rate_updates`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->pacing_decrease_threshold = decrease_threshold;
    cnx->pacing_increase_threshold = increase_threshold;
    cnx->is_pacing_update_requested = (decrease_threshold != UINT64_MAX || increase_threshold != UINT64_MAX);
}
```

### Rust body
```rust
    pub fn pacing_rate(&self) -> u64 {
        self.paths.first().map(|p| p.pacing.rate).unwrap_or(0)
    }
```

## `picoquic/sacks.c:picoquic_sack_first_item`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the first ack_tree item; Rust body is a last-item function returning ack_tree.last().
* C source: `picoquic/sacks.c:68-72`
* C signature: `picoquic_sack_item_t * picoquic_sack_first_item(picoquic_sack_list_t *)`
* Rust source: `rs/fq/src/internal.rs:8479-8494`
* Rust item: `picoquic_sack_first_item`

### C body
```c
{
    return picoquic_sack_item_value(picosplay_first(&sack_list->ack_tree));
}
```

### Rust body
```rust
pub fn picoquic_sack_last_item(sack_list: &SackList) -> Option<SackItemToken> {
    sack_list
        .ack_tree
        .last()
        .and_then(|st| sack_list.resolve_splay(st))
}
```

## `picoquic/sacks.c:picoquic_sack_list_is_empty`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C checks whether the ack tree size is zero; Rust body shown implements sack_next_item successor logic.
* C source: `picoquic/sacks.c:121-126`
* C signature: `int picoquic_sack_list_is_empty(picoquic_sack_list_t *)`
* Rust source: `rs/fq/src/internal.rs:8405-8418`
* Rust item: `is_empty`

### C body
```c
{
    return (sack_list->ack_tree.size == 0);
}
```

### Rust body
```rust
impl SackList {
    /// Splay-tree successor of `sack` in `list`.  C: `sack_next_item`.
    /// In picoquic, "next" means the range with the next lower PN (predecessor in our key-order).
    pub fn sack_next_item(&mut self, sack: SackItemToken) -> Option<SackItemToken> {
        let st = self.sack_items.get(sack)?.ack_tree_membership?;
        let next_st = self.ack_tree.previous(st)?;
        self.ack_tree.get(next_st).copied()
    }
}
```
