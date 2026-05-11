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

## `picoquic/quicctx.c:picoquic_set_optimistic_ack_policy`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C assigns sequence_hole_pseudo_period; Rust body shown sets is_preemptive_repeat_enabled.
* C source: `picoquic/quicctx.c:5354-5358`
* C signature: `void picoquic_set_optimistic_ack_policy(picoquic_quic_t *, uint32_t)`
* Rust source: `rs/fq/src/lib.rs:4434-4441`
* Rust item: `set_optimistic_ack_policy`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->sequence_hole_pseudo_period = sequence_hole_pseudo_period;
}
```

### Rust body
```rust
    pub fn set_preemptive_repeat_policy(&mut self, do_repeat: bool) {
        self.is_preemptive_repeat_enabled = do_repeat;
    }
```

## `picoquic/quicctx.c:picoquic_set_verify_certificate_callback`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C disposes and sets certificate verification callback; Rust sets stateless reset minimum interval, an unrelated field.
* C source: `picoquic/quicctx.c:5486-5492`
* C signature: `void picoquic_set_verify_certificate_callback(picoquic_quic_t *, ptls_verify_certificate_t *, picoquic_free_verify_certificate_ctx)`
* Rust source: `rs/fq/src/lib.rs:1890-1901`
* Rust item: `set_verify_certificate_callback`

### C body
```c
    ptls_verify_certificate_t * cb, picoquic_free_verify_certificate_ctx free_fn) {
    PICOQUIC_THREAD_CHECK(quic);
    picoquic_dispose_verify_certificate_callback(quic);

    picoquic_tls_set_verify_certificate_callback(quic, cb, free_fn);
}
```

### Rust body
```rust
    pub fn set_default_stateless_reset_min_interval(&mut self, min_interval: Duration) {
        self.stateless_reset_min_interval = min_interval;
    }
```

## `picoquic/quicctx.c:picoquic_unregister_net_id`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C deletes a network hash entry and clears stored address/hash item fields; Rust body only reads and returns membership/address data.
* C source: `picoquic/quicctx.c:1280-1290`
* C signature: `void picoquic_unregister_net_id(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/lib.rs:2007-2013`
* Rust item: `unregister_net_id`

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

## `picoquic/sacks.c:picoquic_sack_item_range_end`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns end_of_sack_range, while the Rust body shown is nb_times_sent and returns a send-count entry.
* C source: `picoquic/sacks.c:466-469`
* C signature: `uint64_t picoquic_sack_item_range_end(picoquic_sack_item_t *)`
* Rust source: `rs/fq/src/internal.rs:8667-8675`
* Rust item: `range_end`

### C body
```c
{
    return sack_item->end_of_sack_range;
}
```

### Rust body
```rust
    pub fn nb_times_sent(&self, is_opportunistic: i32) -> i32 {
        self.nb_times_sent[is_opportunistic.clamp(0, 1) as usize]
    }
```

## `picoquic/sacks.c:picoquic_sack_list_size`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns ack_tree.size, while the Rust body shown contains SackItem accessors and no list size implementation.
* C source: `picoquic/sacks.c:498-501`
* C signature: `size_t picoquic_sack_list_size(picoquic_sack_list_t *)`
* Rust source: `rs/fq/src/internal.rs:8653-8676`
* Rust item: `size`

### C body
```c
{
    return (size_t)sack_list->ack_tree.size;
}
```

### Rust body
```rust
impl SackItem {
    /// Inclusive start of this SACK range.  C:
    /// `sack_item_range_start`.
    pub fn range_start(&self) -> u64 {
        self.start_of_sack_range
    }

    /// Exclusive end of this SACK range.  C:
    /// `sack_item_range_end`.
    pub fn range_end(&self) -> u64 {
        self.end_of_sack_range
    }

    /// Number of times this range has been sent in an ACK frame.
    /// C: `sack_item_nb_times_sent`.
    pub fn nb_times_sent(&self, is_opportunistic: i32) -> i32 {
        self.nb_times_sent[is_opportunistic.clamp(0, 1) as usize]
    }
}
```
