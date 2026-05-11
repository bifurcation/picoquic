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

## `picoquic/quicctx.c:picoquic_set_packet_train_mode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets packet_train_mode based on train_mode; Rust body shown sets padding policy fields instead.
* C source: `picoquic/quicctx.c:4673-4679`
* C signature: `void picoquic_set_packet_train_mode(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1118-1126`
* Rust item: `set_packet_train_mode`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    /* TODO: consider setting high water mark for pacing. */
    /* If set, wait until pacing bucket is full enough to allow further transmissions. */
    quic->packet_train_mode = (train_mode > 0) ? 1 : 0;
}
```

### Rust body
```rust
    pub fn set_padding_policy(&mut self, padding_min_size: u32, padding_multiple: u32) {
        self.padding_minsize_default = padding_min_size;
        self.padding_multiple_default = padding_multiple;
    }
```

## `picoquic/quicctx.c:picoquic_start_key_rotation`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C conditionally computes and applies rotated keys; Rust is placeholder-like and always returns a generic error.
* C source: `picoquic/quicctx.c:5036-5058`
* C signature: `int picoquic_start_key_rotation(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2790-2793`
* Rust item: `start_key_rotation`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    /* Verify that a packet of the previous rotation was acked */
    if (cnx->cnx_state != picoquic_state_ready ||
        cnx->crypto_epoch_sequence >
        picoquic_sack_list_last(&cnx->ack_ctx[picoquic_packet_context_application].sack_list)) {
        ret = PICOQUIC_ERROR_KEY_ROTATION_NOT_READY;
    }
    else {
        ret = picoquic_compute_new_rotated_keys(cnx);
    }

    if (ret == 0) {
        picoquic_apply_rotated_keys(cnx, 1);
        picoquic_crypto_context_free(&cnx->crypto_context_old);
        cnx->crypto_epoch_sequence = cnx->pkt_ctx[picoquic_packet_context_application].send_sequence;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn start_key_rotation(&mut self) -> Result<(), Error> {
        // Complex: initiates TLS key update state machine.
        Err(Error::Generic)
    }
```

## `picoquic/quicctx.c:picoquic_update_issued_ticket`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C updates ticket IP length/address, rtt, and cwin; Rust body shown is just an empty test module.
* C source: `picoquic/quicctx.c:444-459`
* C signature: `void picoquic_update_issued_ticket(picoquic_issued_ticket_t *, uint64_t, uint64_t, const uint8_t *, uint8_t)`
* Rust source: `rs/fq/src/lib.rs:7539-7547`
* Rust item: `update`

### C body
```c
{
    /* Update in place */
    if (ip_addr_length > PICOQUIC_STORED_IP_MAX) {
        ip_addr_length = PICOQUIC_STORED_IP_MAX;
    }
    ticket->ip_addr_length = ip_addr_length;
    memcpy(ticket->ip_addr, ip_addr, ip_addr_length);
    ticket->rtt = rtt;
    ticket->cwin = cwin;
}
```

### Rust body
```rust
mod test {}
```

## `picoquic/sacks.c:picoquic_sack_item_range_start`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns start_of_sack_range; Rust body returns end_of_sack_range.
* C source: `picoquic/sacks.c:459-464`
* C signature: `uint64_t picoquic_sack_item_range_start(picoquic_sack_item_t *)`
* Rust source: `rs/fq/src/internal.rs:8661-8669`
* Rust item: `range_start`

### C body
```c
{
    return sack_item->start_of_sack_range;
}
```

### Rust body
```rust
    pub fn range_end(&self) -> u64 {
        self.end_of_sack_range
    }
```

## `picoquic/sacks.c:picoquic_sack_next_item`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns picosplay_next, while Rust uses ack_tree.previous, the opposite visible tree direction.
* C source: `picoquic/sacks.c:79-82`
* C signature: `picoquic_sack_item_t * picoquic_sack_next_item(picoquic_sack_item_t *)`
* Rust source: `rs/fq/src/internal.rs:8413-8417`
* Rust item: `sack_next_item`

### C body
```c
{
    return picoquic_sack_item_value(picosplay_next(&sack->node));
}
```

### Rust body
```rust
    pub fn sack_next_item(&mut self, sack: SackItemToken) -> Option<SackItemToken> {
        let st = self.sack_items.get(sack)?.ack_tree_membership?;
        let next_st = self.ack_tree.previous(st)?;
        self.ack_tree.get(next_st).copied()
    }
```
