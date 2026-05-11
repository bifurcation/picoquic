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

## `picoquic/quicctx.c:picoquic_set_max_data_control`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C updates max data fields and eligible existing connections; Rust body shown sets the default idle timeout instead.
* C source: `picoquic/quicctx.c:970-990`
* C signature: `void picoquic_set_max_data_control(picoquic_quic_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:1738-1746`
* Rust item: `set_max_data_control`

### C body
```c
{
    picoquic_cnx_t* cnx;
    PICOQUIC_THREAD_CHECK(quic);
    cnx = quic->cnx_list;
    quic->max_data_limit = max_data;

    quic->default_tp.initial_max_data = max_data;

    while (cnx != NULL) {
        /* If the connection is not yet initialized, reset the maxdata parameter */
        if (cnx->client_mode &&
            cnx->cnx_state == picoquic_state_client_init &&
            cnx->tls_stream[0].sent_offset == 0 &&
            cnx->tls_stream[0].send_queue == NULL){
            cnx->local_parameters.initial_max_data = max_data;
            cnx->maxdata_local = max_data;
        }
        cnx = cnx->next_in_table;
    }
}
```

### Rust body
```rust
    pub fn set_default_idle_timeout(&mut self, idle_timeout: Duration) {
        self.default_tp.max_idle_timeout = idle_timeout;
    }
```

## `picoquic/quicctx.c:picoquic_set_preemptive_repeat_per_cnx`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets the preemptive repeat enabled flag; Rust returns a preemptive repeat count.
* C source: `picoquic/quicctx.c:5366-5370`
* C signature: `void picoquic_set_preemptive_repeat_per_cnx(picoquic_cnx_t *, int)`
* Rust source: `rs/fq/src/lib.rs:4446-4454`
* Rust item: `set_preemptive_repeat`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->is_preemptive_repeat_enabled = (do_repeat) ? 1 : 0;
}
```

### Rust body
```rust
    pub fn preemptive_repeat_count(&self) -> u64 {
        self.nb_preemptive_repeat
    }
```

## `picoquic/quicctx.c:picoquic_stream_data_node_alloc`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C allocates or reuses a stream data node and updates pool/allocation counters; Rust body is an impl method clearing StreamHead state, which is visibly unrelated to node allocation/reuse.
* C source: `picoquic/quicctx.c:3377-3405`
* C signature: `picoquic_stream_data_node_t * picoquic_stream_data_node_alloc(picoquic_quic_t *)`
* Rust source: `rs/fq/src/internal.rs:12953-13018`
* Rust item: `stream_data_node_alloc`

### C body
```c
{
    picoquic_stream_data_node_t* stream_data = quic->p_first_data_node;
    
    if (stream_data == NULL) {
        stream_data = (picoquic_stream_data_node_t*)
            malloc(sizeof(picoquic_stream_data_node_t));

        if (stream_data != NULL) {
            /* It might be sufficient to zero the metadata, but zeroing everything
             * appears safer, and does not confuse checkers like valgrind.
             */
            memset(stream_data, 0, sizeof(picoquic_stream_data_node_t));
            stream_data->quic = quic;
            quic->nb_data_nodes_allocated++;
            if (quic->nb_data_nodes_allocated > quic->nb_data_nodes_allocated_max) {
                quic->nb_data_nodes_allocated_max = quic->nb_data_nodes_allocated;
            }
        }
    }
    else {
        quic->p_first_data_node = stream_data->next_stream_data;
        stream_data->next_stream_data = NULL;
        stream_data->bytes = NULL;
        quic->nb_data_nodes_in_pool--;
    }

    return stream_data;
}
```

### Rust body
```rust
impl StreamHead {
    /// Reset all stream state to defaults, keeping the stream_id.
    /// C: `picoquic_clear_stream`.
    pub fn clear_stream(&mut self) {
        let stream_id = self.stream_id;
        *self = StreamHead {
            stream_tree_membership: None,
            stream_id,
            affinity_path: None,
            consumed_offset: 0,
            fin_offset: 0,
            reset_offset: 0,
            maxdata_local: 0,
            maxdata_local_acked: 0,
            maxdata_remote: 0,
            local_error: 0,
            remote_error: 0,
            local_stop_error: 0,
            remote_stop_error: 0,
            last_time_data_sent: crate::Instant::from_ticks(0),
            stream_data_tree: crate::splay::SplayTree::default(),
            stream_data_nodes: crate::arena::Arena::new(),
            sent_offset: 0,
            reliable_size: 0,
            send_queue: std::collections::VecDeque::new(),
            app_stream_ctx: None,
            direct_receive_fn: None,
            direct_receive_ctx: None,
            sack_list: SackList::new(),
            stream_priority: 0,
            is_active: false,
            fin_requested: false,
            fin_sent: false,
            fin_received: false,
            fin_signalled: false,
            reset_requested: false,
            reset_sent: false,
            reset_acked: false,
            reset_received: false,
            reset_signalled: false,
            stop_sending_requested: false,
            stop_sending_sent: false,
            stop_sending_received: false,
            stop_sending_signalled: false,
            max_stream_updated: false,
            stream_data_blocked_sent: false,
            is_output_stream: false,
            is_closed: false,
            is_discarded: false,
            use_app_flow_control: false,
            is_not_coalesced: false,
        };
    }
}
```

## `picoquic/sacks.c:picoquic_check_sack_list`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown is only a fragment, not a complete function body matching C's range lookup and conditional -1/0 return.
* C source: `picoquic/sacks.c:323-340`
* C signature: `int picoquic_check_sack_list(picoquic_sack_list_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:8512-8520`
* Rust item: `picoquic_check_sack_list`

### C body
```c
{
    int ret = 0;
    picoquic_sack_item_t* sack = picoquic_sack_find_range_below_number(sack_list, NULL, pn64_min);

    if (sack != NULL) {
        if (pn64_max <= sack->end_of_sack_range) {
            ret = -1;
        }
    }
    return ret;
}
```

### Rust body
```rust
    {
        -1
    } else {
```

## `picoquic/sacks.c:picoquic_sack_last_item`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the last SACK item from the ack tree; Rust inserts a range and returns Result<(), Error>.
* C source: `picoquic/sacks.c:74-77`
* C signature: `picoquic_sack_item_t * picoquic_sack_last_item(picoquic_sack_list_t *)`
* Rust source: `rs/fq/src/internal.rs:8489-8506`
* Rust item: `picoquic_sack_last_item`

### C body
```c
{
    return picoquic_sack_item_value(picosplay_last(&sack_list->ack_tree));
}
```

### Rust body
```rust
) -> Result<(), crate::Error> {
    sack_list.insert_item(range_min, range_max, current_time)
}
```
