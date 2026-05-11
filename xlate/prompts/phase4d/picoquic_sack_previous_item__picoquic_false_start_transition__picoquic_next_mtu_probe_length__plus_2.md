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

## `picoquic/sacks.c:picoquic_sack_previous_item`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the previous tree item; Rust calls self.ack_tree.next(st), which is visibly the opposite direction by name.
* C source: `picoquic/sacks.c:84-87`
* C signature: `picoquic_sack_item_t * picoquic_sack_previous_item(picoquic_sack_item_t *)`
* Rust source: `rs/fq/src/internal.rs:8423-8427`
* Rust item: `sack_previous_item`

### C body
```c
{
    return picoquic_sack_item_value(picosplay_previous(&sack->node));
}
```

### Rust body
```rust
    pub fn sack_previous_item(&mut self, sack: SackItemToken) -> Option<SackItemToken> {
        let st = self.sack_items.get(sack)?.ack_tree_membership?;
        let prev_st = self.ack_tree.next(st)?;
        self.ack_tree.get(prev_st).copied()
    }
```

## `picoquic/sender.c:picoquic_false_start_transition`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is for client_almost_ready_transition, sets ClientAlmostReady, and moves packet context state; C body sets server false start and optionally queues a NEW_TOKEN frame.
* C source: `picoquic/sender.c:2664-2683`
* C signature: `void picoquic_false_start_transition(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:7685-7715`
* Rust item: `false_start_transition`

### C body
```c
{
    /* Transition to false start state. */
    cnx->cnx_state = picoquic_state_server_false_start;

    /* On a server that does address validation, send a NEW TOKEN frame */
    if (!cnx->client_mode && (cnx->quic->check_token || cnx->quic->provide_token)) {
        uint8_t token_buffer[256];
        size_t token_size;
        picoquic_connection_id_t n_cid = picoquic_null_connection_id;

        if (picoquic_prepare_retry_token(cnx->quic, (struct sockaddr*) & cnx->path[0]->first_tuple->peer_addr,
            current_time, &n_cid, &n_cid, 0,
            token_buffer, sizeof(token_buffer), &token_size) == 0) {
            if (picoquic_queue_new_token_frame(cnx, token_buffer, token_size) != 0) {
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, picoquic_frame_type_new_token);
            }
        }
    }
}
```

### Rust body
```rust
    pub fn client_almost_ready_transition(&mut self) {
        self.connection_state = crate::State::ClientAlmostReady;
        if self.is_multipath_enabled && !self.paths.is_empty() {
            let app_ctx = core::mem::replace(
                &mut self.pkt_ctx[PacketContext::Application as usize],
                PacketContextState {
                    send_sequence: 0,
                    next_sequence_hole: 0,
                    retransmit_sequence: 0,
                    highest_acknowledged: u64::MAX,
                    latest_time_acknowledged: self.start_time,
                    highest_acknowledged_time: self.start_time,
                    pending: BTreeMap::new(),
                    retransmitted: BTreeMap::new(),
                    preemptive_repeat_seq: None,
                    retransmitted_queue_size: 0,
                    ecn_ect0_total_remote: 0,
                    ecn_ect1_total_remote: 0,
                    ecn_ce_total_remote: 0,
                    ack_of_ack_requested: false,
                },
            );
            self.paths[0].pkt_ctx = app_ctx;
        }
    }
```

## `picoquic/sender.c:picoquic_next_mtu_probe_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body computes probe_length through multiple branches and returns it; Rust snippet is only a partial overhead calculation and does not visibly implement the shown logic.
* C source: `picoquic/sender.c:1545-1585`
* C signature: `size_t picoquic_next_mtu_probe_length(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:16793-16796`
* Rust item: `next_mtu_probe_length`

### C body
```c
{
    size_t probe_length;

    if (path_x->send_mtu_max_tried == 0) {
        if (cnx->remote_parameters.max_packet_size > 0) {
            probe_length = cnx->remote_parameters.max_packet_size;

            if (cnx->quic->mtu_max > 0 && probe_length >
                cnx->quic->mtu_max - PICOQUIC_MTU_OVERHEAD((struct sockaddr*)&path_x->first_tuple->peer_addr)) {
                probe_length = cnx->quic->mtu_max - PICOQUIC_MTU_OVERHEAD((struct sockaddr*)&path_x->first_tuple->peer_addr);
            }
            else if (probe_length > PICOQUIC_MAX_PACKET_SIZE) {
                probe_length = PICOQUIC_MAX_PACKET_SIZE;
            }
            if (probe_length < path_x->send_mtu) {
                probe_length = path_x->send_mtu;
            }
        }
        else if (cnx->quic->mtu_max > 0) {
            probe_length = cnx->quic->mtu_max - PICOQUIC_MTU_OVERHEAD((struct sockaddr*)&path_x->first_tuple->peer_addr);
        }
        else {
            probe_length = PICOQUIC_PRACTICAL_MAX_MTU;
        }
    }
    else {
        if (path_x->send_mtu_max_tried > 1500) {
            probe_length = 1500;
        }
        else if (path_x->send_mtu_max_tried > 1400) {
            probe_length = 1400;
        }
        else {
            probe_length = (path_x->send_mtu + path_x->send_mtu_max_tried) / 2;
        }
    }

    return probe_length;
}
```

### Rust body
```rust
        let overhead = if path.tuples.first().is_some_and(|t| t.peer_addr.is_ipv6()) {
            48usize // IPv6 (40-byte IP header + 8-byte UDP header)
        } else {
```

## `picoquic/sender.c:picoquic_recycle_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C recycles or frees packet pool entries; Rust body performs bandwidth/delivery estimation logic and has no packet pool handling.
* C source: `picoquic/sender.c:561-575`
* C signature: `void picoquic_recycle_packet(picoquic_quic_t *, picoquic_packet_t *)`
* Rust source: `rs/fq/src/internal.rs:1059-1171`
* Rust item: `create_packet`

### C body
```c
{
    if (packet != NULL) {
        if (quic->nb_packets_in_pool >= PICOQUIC_MAX_PACKETS_IN_POOL) {
            free(packet);
            quic->nb_packets_allocated--;
        }
        else {
            memset(packet, 0, offsetof(struct st_picoquic_packet_t, bytes));
            packet->packet_previous = quic->p_first_packet;
            quic->p_first_packet = packet;
            quic->nb_packets_in_pool++;
        }
    }
}
```

### Rust body
```rust
) {
    if send_time < path_x.delivered_sent_last {
        return;
    }

    if path_x.delivered_time_last.ticks() == 0 {
        path_x.delivered_last = path_x.delivered;
        path_x.delivered_time_last = Instant::from_ticks(delivery_time);
        path_x.delivered_sent_last = send_time;
        return;
    }

    let mut receive_interval = delivery_time.saturating_sub(delivered_time_prior);
    if receive_interval <= BANDWIDTH_TIME_INTERVAL_MIN {
        return;
    }

    let delivered = path_x.delivered.saturating_sub(delivered_prior);
    let send_interval = send_time.saturating_sub(delivered_sent_prior);
    if send_interval > receive_interval {
        receive_interval = send_interval;
    }
    let bw_estimate = crate::utils::rate_from_bytes(delivered, receive_interval);

    path_x.bandwidth_estimate = bw_estimate;
    if !rs_is_path_limited || bw_estimate > path_x.bandwidth_estimate {
        let is_first_path = connection
            .paths
            .first()
            .map(|p| p.unique_path_id == path_x.unique_path_id)
            .unwrap_or(false);
        if is_first_path && connection.is_ack_frequency_negotiated {
            let mut ack_gap = 0;
            let mut ack_delay_max = 0;
            connection.compute_ack_gap_and_delay(
                path_x.rtt_min,
                connection.remote_parameters.min_ack_delay.ticks(),
                bw_estimate,
                &mut ack_gap,
                &mut ack_delay_max,
            );
            if ack_gap != connection.ack_gap_local {
                connection.is_ack_frequency_updated = true;
            }
        }
    }

    path_x.delivered_last = path_x.delivered;
    path_x.delivered_time_last = Instant::from_ticks(delivery_time);
    path_x.delivered_sent_last = send_time;
    path_x.delivered_last_packet = delivered_prior;
    path_x.last_bw_estimate_path_limited = rs_is_path_limited;
    if path_x.delivered_last_packet > path_x.delivered_limited_index {
        path_x.delivered_limited_index = 0;
    }
    if bw_estimate > path_x.bandwidth_estimate_max {
        path_x.bandwidth_estimate_max = bw_estimate;
    }
}
```

## `picoquic/sockloop.c:picoquic_internal_thread_setname`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets the current thread name on supported platforms; Rust is an empty no-op placeholder.
* C source: `picoquic/sockloop.c:1733-1761`
* C signature: `void picoquic_internal_thread_setname(const char *)`
* Rust source: `rs/fq/src/packet_loop.rs:1485-1485`
* Rust item: `internal_thread_setname`

### C body
```c
{
#ifdef _WINDOWS
    wchar_t wname[257];
    wname[0] = 0;

    if (swprintf(wname, 256, L"%S", thread_name) < 0) {
        DBG_PRINTF("Cannot convert thread name <%s> to wchar[256], err: 0x%x",
            thread_name, GetLastError());
    }
    else {
        HRESULT r = SetThreadDescription(GetCurrentThread(), wname);
        if (r != 0) {
            DBG_PRINTF("Set thread name <%S> returns: 0x%x", wname, r);
        }
    }
#else
#if defined(__APPLE__)
    pthread_setname_np(thread_name);
#elif defined(__FreeBSD__)
    pthread_setname_np(pthread_self(), thread_name);
#else
    int r=prctl(PR_SET_NAME, thread_name, 0, 0, 0);
    if (r != 0) {
        DBG_PRINTF("Set thread name <%s> returns: 0x%x", thread_name, r);
    }
#endif
#endif
}
```

### Rust body
```rust
pub fn internal_thread_setname(_thread_name: &str) {}
```
