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

## `picoquic/quicctx.c:picoquic_set_max_half_open_retry_threshold`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets max_half_open_before_retry from an argument; Rust is a getter returning the field.
* C source: `picoquic/quicctx.c:1206-1210`
* C signature: `void picoquic_set_max_half_open_retry_threshold(picoquic_quic_t *, uint32_t)`
* Rust source: `rs/fq/src/lib.rs:1179-1186`
* Rust item: `set_max_half_open_retry_threshold`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->max_half_open_before_retry = max_half_open_before_retry;
}
```

### Rust body
```rust
    pub fn max_half_open_retry_threshold(&self) -> u32 {
        self.max_half_open_before_retry
    }
```

## `picoquic/quicctx.c:picoquic_set_priority_limit_for_bypass`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets priority_limit_for_bypass; Rust body sets lost feedback notification.
* C source: `picoquic/quicctx.c:5400-5404`
* C signature: `void picoquic_set_priority_limit_for_bypass(picoquic_cnx_t *, uint8_t)`
* Rust source: `rs/fq/src/lib.rs:4633-4641`
* Rust item: `set_priority_limit_for_bypass`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->priority_limit_for_bypass = priority_limit;
}
```

### Rust body
```rust
    pub fn set_feedback_loss_notification(&mut self, should_notify: bool) {
        self.is_lost_feedback_notification_required = should_notify;
    }
```

## `picoquic/quicctx.c:picoquic_stream_data_node_recycle`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C recycles or frees an existing stream data node; Rust allocates and returns a fresh StreamDataNode.
* C source: `picoquic/quicctx.c:3357-3368`
* C signature: `void picoquic_stream_data_node_recycle(picoquic_stream_data_node_t *)`
* Rust source: `rs/fq/src/internal.rs:12944-12962`
* Rust item: `stream_data_node_recycle`

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

## `picoquic/sacks.c:picoquic_record_pn_received`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C updates largest-received timestamp and out-of-order flags before updating the SACK list; Rust only updates the SACK list and maps the result.
* C source: `picoquic/sacks.c:258-294`
* C signature: `int picoquic_record_pn_received(picoquic_cnx_t *, picoquic_packet_context_enum, picoquic_local_cnxid_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:8047-8060`
* Rust item: `record_pn_received`

### C body
```c
{
    int ret = 0;
    picoquic_sack_list_t* sack_list = picoquic_sack_list_from_cnx_context(cnx, pc, l_cid);

    if (sack_list != NULL) {
        if (picoquic_sack_list_is_empty(sack_list)) {
            /* This is the first packet ever received.. */
            cnx->ack_ctx[pc].time_stamp_largest_received = current_microsec;
        }
        else {
            uint64_t pn_last = picoquic_sack_list_last(sack_list);
            if (pn64 > pn_last) {
                if (pn64 > pn_last + 1) {
                    cnx->ack_ctx[pc].act[0].out_of_order_received = 1;
                    cnx->ack_ctx[pc].act[1].out_of_order_received = 1;
                }
                cnx->ack_ctx[pc].time_stamp_largest_received = current_microsec;
            }
            else
            {
                if (cnx->ack_ctx[pc].act[0].ack_needed && pn64 < cnx->ack_ctx[pc].act[0].highest_ack_sent) {
                    cnx->ack_ctx[pc].act[0].out_of_order_received = 1;
                }
                if (cnx->ack_ctx[pc].act[1].ack_needed && pn64 < cnx->ack_ctx[pc].act[1].highest_ack_sent) {
                    cnx->ack_ctx[pc].act[1].out_of_order_received = 1;
                }
            }
        }

        ret = picoquic_update_sack_list(sack_list, pn64, pn64, current_microsec);
    }
    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        // C: picoquic_record_pn_received — insert `pn64` into SACK list.
        let ack_ctx = &mut self.ack_ctx[pc as usize];
        match ack_ctx.sack_list.update(pn64, pn64, current_microsec) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
```

## `picoquic/sacks.c:picoquic_sack_list_first`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns first range start or UINT64_MAX; Rust body is a last-list function returning last range end or 0.
* C source: `picoquic/sacks.c:407-412`
* C signature: `uint64_t picoquic_sack_list_first(picoquic_sack_list_t *)`
* Rust source: `rs/fq/src/internal.rs:8538-8553`
* Rust item: `picoquic_sack_list_first`

### C body
```c
{
    picoquic_sack_item_t* first = picoquic_sack_first_item(sack_list);
    return (first == NULL)? UINT64_MAX:first->start_of_sack_range;
}
```

### Rust body
```rust
pub fn picoquic_sack_list_last(sack_list: &SackList) -> u64 {
    picoquic_sack_last_item(sack_list)
        .and_then(|tok| sack_list.sack_items.get(tok))
        .map(|item| item.end_of_sack_range)
        .unwrap_or(0)
}
```
