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

## `picoquic/quicctx.c:picoquic_get_path_quality_from_context`
* Phase 4C status: `suspect`
* Phase 4C rationale: Most copied fields match, but C computes sent inside the body while Rust accepts sent as an argument, so equivalence is not body-visible.
* C source: `picoquic/quicctx.c:2678-2699`
* C signature: `void picoquic_get_path_quality_from_context(picoquic_path_t *, picoquic_path_quality_t *)`
* Rust source: `rs/fq/src/lib.rs:2747-2769`
* Rust item: `get_path_quality_from_context`

### C body
```c
{
    picoquic_refresh_path_quality_thresholds(path_x);
    quality->cwin = path_x->cwin;
    quality->rtt = path_x->smoothed_rtt;
    quality->rtt_sample = path_x->rtt_sample;
    quality->rtt_min = path_x->rtt_min;
    quality->rtt_max = path_x->rtt_max;
    quality->rtt_variant = path_x->rtt_variant;
    quality->pacing_rate = path_x->pacing.rate;
    quality->receive_rate_estimate = path_x->receive_rate_estimate;
    quality->sent = picoquic_get_sequence_number(path_x->cnx, path_x, picoquic_packet_context_application);
    quality->lost = path_x->nb_losses_found;
    quality->timer_losses = path_x->nb_timer_losses;
    quality->spurious_losses = path_x->nb_spurious;
    quality->max_spurious_rtt = path_x->max_spurious_rtt;
    quality->max_reorder_delay = path_x->max_reorder_delay;
    quality->max_reorder_gap = path_x->max_reorder_gap;
    quality->bytes_in_transit = path_x->bytes_in_transit;
    quality->bytes_sent = path_x->bytes_sent;
    quality->bytes_received = path_x->received;
}
```

### Rust body
```rust
fn get_path_quality_from_context(path_x: &mut Path, sent: u64) -> PathQuality {
    path_x.refresh_quality_thresholds();
    PathQuality {
        receive_rate_estimate: path_x.receive_rate_estimate,
        pacing_rate: path_x.pacing.rate,
        cwin: path_x.cwin,
        rtt: path_x.smoothed_rtt,
        rtt_sample: path_x.rtt_sample,
        rtt_variant: path_x.rtt_variant,
        rtt_min: path_x.rtt_min,
        rtt_max: path_x.rtt_max,
        sent,
        lost: path_x.nb_losses_found,
        timer_losses: path_x.nb_timer_losses,
        spurious_losses: path_x.nb_spurious,
        max_spurious_rtt: path_x.max_spurious_rtt,
        max_reorder_delay: path_x.max_reorder_delay,
        max_reorder_gap: path_x.max_reorder_gap,
        bytes_in_transit: path_x.bytes_in_transit,
        bytes_sent: path_x.bytes_sent,
        bytes_received: path_x.received,
    }
}
```

## `picoquic/quicctx.c:picoquic_notify_destination_unreachable_by_cnxid`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust body shown is incomplete, ending after an else-if condition, so the visible body cannot confirm the C behavior of finding a connection and notifying it.
* C source: `picoquic/quicctx.c:2246-2262`
* C signature: `void picoquic_notify_destination_unreachable_by_cnxid(picoquic_quic_t *, picoquic_connection_id_t *, uint64_t, struct sockaddr *, struct sockaddr *, int, int)`
* Rust source: `rs/fq/src/lib.rs:3745-3756`
* Rust item: `notify_destination_unreachable_by_connection_id`

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

## `picoquic/quicctx.c:picoquic_reinsert_by_wake_time`
* Phase 4C status: `suspect`
* Phase 4C rationale: C explicitly removes the connection from the wake list before setting next_wake_time and reinserting; Rust sets/inserts but has no visible prior remove.
* C source: `picoquic/quicctx.c:1515-1520`
* C signature: `void picoquic_reinsert_by_wake_time(picoquic_quic_t *, picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:6278-6297`
* Rust item: `reinsert_by_wake_time`

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

## `picoquic/quicctx.c:picoquic_set_callback`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets both callback function and callback context; Rust only sets callback_fn.
* C source: `picoquic/quicctx.c:4765-4771`
* C signature: `void picoquic_set_callback(picoquic_cnx_t *, picoquic_stream_data_cb_fn, void *)`
* Rust source: `rs/fq/src/lib.rs:3093-3095`
* Rust item: `set_callback`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->callback_fn = callback_fn;
    cnx->callback_ctx = callback_ctx;
}
```

### Rust body
```rust
    pub fn set_callback(&mut self, callback: Option<Box<dyn StreamDataCallback>>) {
        self.callback_fn = callback;
    }
```

## `picoquic/quicctx.c:picoquic_set_log_level`
* Phase 4C status: `suspect`
* Phase 4C rationale: C derives use_long_log from log_level; Rust only propagates existing self.use_long_log to connections with no visible use of log_level.
* C source: `picoquic/quicctx.c:4639-4644`
* C signature: `void picoquic_set_log_level(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1083-1087`
* Rust item: `set_log_level`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    /* Only two level for now: log first 100 packets, or log everything. */
    quic->use_long_log = (log_level > 0) ? 1 : 0;
}
```

### Rust body
```rust
        for connection in self.connections.iter_mut() {
            connection.use_long_log = self.use_long_log;
        }
```
