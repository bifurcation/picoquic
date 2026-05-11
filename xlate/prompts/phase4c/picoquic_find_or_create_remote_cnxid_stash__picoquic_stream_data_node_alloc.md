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

## Pair `picoquic/quicctx.c:picoquic_find_or_create_remote_cnxid_stash`
C: `picoquic/quicctx.c:2892-2915 picoquic_find_or_create_remote_cnxid_stash`
Rust: `rs/fq/src/internal.rs:4911-4922 find_or_create_remote_connection_id_stash`

### C body
```c
{
    picoquic_remote_cnxid_stash_t* remote_cnxid_stash = cnx->first_remote_cnxid_stash;
    picoquic_remote_cnxid_stash_t** p_previous = &cnx->first_remote_cnxid_stash;

    while (remote_cnxid_stash != NULL && remote_cnxid_stash->unique_path_id != unique_path_id) {
        p_previous = &remote_cnxid_stash->next_stash;
        remote_cnxid_stash = remote_cnxid_stash->next_stash;
    }

    if (remote_cnxid_stash == NULL && do_create) {
        remote_cnxid_stash = (picoquic_remote_cnxid_stash_t*)malloc(sizeof(picoquic_remote_cnxid_stash_t));
        if (remote_cnxid_stash != NULL) {
            memset(remote_cnxid_stash, 0, sizeof(picoquic_remote_cnxid_stash_t));
            remote_cnxid_stash->unique_path_id = unique_path_id;
            *p_previous = remote_cnxid_stash;
        }
    }

    return remote_cnxid_stash;
}
```

### Rust body
```rust
        {
            return Some(idx);
        }
```

## Pair `picoquic/quicctx.c:picoquic_stream_data_node_alloc`
C: `picoquic/quicctx.c:3377-3405 picoquic_stream_data_node_alloc`
Rust: `rs/fq/src/internal.rs:12953-13018 stream_data_node_alloc`

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
