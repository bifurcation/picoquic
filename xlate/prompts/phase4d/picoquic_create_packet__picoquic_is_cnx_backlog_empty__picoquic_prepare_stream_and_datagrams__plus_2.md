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

## `picoquic/sender.c:picoquic_create_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C allocates or reuses and zeroes a packet; Rust shown computes bandwidth/delivery state and is unrelated.
* C source: `picoquic/sender.c:533-559`
* C signature: `picoquic_packet_t * picoquic_create_packet(picoquic_quic_t *)`
* Rust source: `rs/fq/src/internal.rs:1059-1171`
* Rust item: `create_packet`

### C body
```c
{
    picoquic_packet_t* packet = quic->p_first_packet;
    
    if (packet == NULL) {
        packet = (picoquic_packet_t*)malloc(sizeof(picoquic_packet_t));
        if (packet != NULL) {
            quic->nb_packets_allocated++;
            if (quic->nb_packets_allocated > quic->nb_packets_allocated_max) {
                quic->nb_packets_allocated_max = quic->nb_packets_allocated;
            }
        }
    }
    else {
        quic->p_first_packet = packet->packet_previous;
        quic->nb_packets_in_pool--;
    }

    if (packet != NULL) {
        /* It might be sufficient to zero the metadata, but zeroing everything
         * appears safer, and does not confuse checkers like valgrind.
         */
        memset(packet, 0, sizeof(picoquic_packet_t));
    }

    return packet;
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

## `picoquic/sender.c:picoquic_is_cnx_backlog_empty`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C checks packet-context backlogs based on connection state and multipath paths; Rust only checks queued bytes, misc frames, and output streams.
* C source: `picoquic/sender.c:1310-1330`
* C signature: `int picoquic_is_cnx_backlog_empty(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4825-4827`
* Rust item: `is_cnx_backlog_empty`

### C body
```c
{
    int backlog_empty = 1;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx->cnx_state < picoquic_state_ready) {
        backlog_empty = picoquic_is_pkt_ctx_backlog_empty(&cnx->pkt_ctx[picoquic_packet_context_initial]) &&
            picoquic_is_pkt_ctx_backlog_empty(&cnx->pkt_ctx[picoquic_packet_context_handshake]);
    }

    if (cnx->is_multipath_enabled) {
        for (int i=0; backlog_empty && i < cnx->nb_paths; i++) {
            backlog_empty &= picoquic_is_pkt_ctx_backlog_empty(&cnx->path[i]->pkt_ctx);
        }
    }
    else if (backlog_empty) {
        backlog_empty = picoquic_is_pkt_ctx_backlog_empty(&cnx->pkt_ctx[picoquic_packet_context_application]);
    }

    return backlog_empty;
}
```

### Rust body
```rust
    pub fn is_cnx_backlog_empty(&self) -> bool {
        self.nb_bytes_queued == 0 && self.misc_frames.is_empty() && self.output_streams.is_empty()
    }
```

## `picoquic/sender.c:picoquic_prepare_stream_and_datagrams`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only tries one stream formatting pass and one datagram pass, while C loops by priority, handles retransmitted stream frames, datagram conflict accounting, max_priority_allowed, and first-round no-data logic.
* C source: `picoquic/sender.c:2848-2962`
* C signature: `uint8_t * picoquic_prepare_stream_and_datagrams(picoquic_cnx_t *, picoquic_path_t *, uint8_t *, uint8_t *, int, uint64_t, int *, int *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:17661-17709`
* Rust item: `prepare_stream_and_datagrams`

### C body
```c
{
    int datagram_sent = 0;
    int datagram_tried_and_failed = 0;
    int stream_tried_and_failed = 0;
    int more_data_this_round = 0;
    int is_first_round = 1;

    while (bytes_next + 8 < bytes_max && *ret == 0) {
        /* Find the highest priority level for which there is something to send, then
        * format the frames to send at that level. Repeat in a loop until the
        * packet is full or there is nothing more to send. */
        uint64_t datagram_present = cnx->first_datagram != NULL || cnx->is_datagram_ready || path_x->is_datagram_ready;
        picoquic_stream_head_t* first_stream = picoquic_find_ready_stream_path(cnx,
            (cnx->is_multipath_enabled) ? path_x : NULL, 0);
        picoquic_packet_t* first_repeat = picoquic_first_data_repeat_packet(cnx);
        uint64_t current_priority = UINT64_MAX;
        uint64_t stream_priority = UINT64_MAX;
        int something_sent = 0;
        int conflict_found = 0;

        more_data_this_round = 0;

        int datagram_first = (cnx->datagram_conflicts_max >= cnx->datagram_conflicts_count);
        if (datagram_present) {
            current_priority = cnx->datagram_priority;
        }
        if (first_stream != NULL) {
            stream_priority = first_stream->stream_priority;
        }
        if (first_repeat != NULL && first_repeat->data_repeat_priority < stream_priority) {
            stream_priority = first_repeat->data_repeat_priority;
        }
        if (stream_priority < current_priority) {
            current_priority = stream_priority;
        }

        if (current_priority == UINT64_MAX || current_priority >= max_priority_allowed) {
            /* Nothing to send! */
            if (is_first_round) {
                *no_data_to_send = 1;
            }
            break;
        }

        if (datagram_present &&
            cnx->datagram_priority == current_priority &&
            (cnx->datagram_priority < stream_priority || datagram_first)) {
            bytes_next = picoquic_prepare_datagram_ready(cnx, path_x, bytes_next, bytes_max, is_first_in_packet,
                &more_data_this_round, is_pure_ack, &datagram_tried_and_failed, &datagram_sent, ret);
            something_sent = datagram_sent;
        }

        if (first_repeat != NULL && first_repeat->data_repeat_priority == current_priority) {
            uint8_t* bytes_first = bytes_next;
            if (bytes_next + 8 < bytes_max) {
                bytes_next = picoquic_copy_stream_frames_for_retransmit(cnx, bytes_next, bytes_max,
                    UINT64_MAX, &more_data_this_round, is_pure_ack);
                if (bytes_next > bytes_first) {
                    cnx->datagram_conflicts_count = 0;
                    something_sent = 1;
                }
            }
            else {
                more_data_this_round |= 1;
                conflict_found = 1;
            }
        }

        if (first_stream != NULL && first_stream->stream_priority == current_priority &&
            (!first_stream->is_not_coalesced || !something_sent)) {
            /* Encode the stream frame, or frames */
            uint8_t* bytes_first = bytes_next;
            if (bytes_next + 8 < bytes_max) {
                bytes_next = picoquic_format_available_stream_frames(cnx, path_x, bytes_next, bytes_max, UINT64_MAX,
                    &more_data_this_round, is_pure_ack, &stream_tried_and_failed, ret);
                if (bytes_next > bytes_first) {
                    cnx->datagram_conflicts_count = 0;
                    something_sent = 1;
                }
            }
            else {
                more_data_this_round |= 1;
                conflict_found = 1;
            }
        }

        if (datagram_sent && conflict_found) {
            cnx->datagram_conflicts_count += 1;
        }

        if (datagram_present &&
            cnx->datagram_priority == current_priority &&
            cnx->datagram_priority <= stream_priority &&
            !datagram_first) {
            bytes_next = picoquic_prepare_datagram_ready(cnx, path_x, bytes_next, bytes_max, is_first_in_packet,
                more_data, is_pure_ack, &datagram_tried_and_failed, &datagram_sent, ret);
            something_sent = datagram_sent;
        }

        if (is_first_round) {
            *no_data_to_send = ((first_stream == NULL && first_repeat == NULL) || stream_tried_and_failed) &&
                (!datagram_present || datagram_tried_and_failed);
        }
        is_first_round = 0;
        if (!something_sent) {
            break;
        }
    }
    *more_data |= more_data_this_round;

    return bytes_next;
}
```

### Rust body
```rust
    ) -> Option<&'a mut [u8]> {
        *no_data_to_send = 1;
        if self
            .find_ready_stream_path(path_x, is_first_in_packet)
            .is_some()
        {
            let before = bytes.len();
            bytes = format_available_stream_frames(
                self,
                path_x,
                bytes,
                current_priority,
                more_data,
                is_pure_ack,
                no_data_to_send,
                ret,
            )?;
            if bytes.len() != before {
                *no_data_to_send = 0;
            }
        }
        let before = bytes.len();
        let mut datagram_tried_and_failed = 0;
        let mut datagram_sent = 0;
        bytes = self.picoquic_prepare_datagram_ready(
            path_x,
            bytes,
            i32::from(is_first_in_packet),
            more_data,
            is_pure_ack,
            &mut datagram_tried_and_failed,
            &mut datagram_sent,
            ret,
        )?;
        if bytes.len() != before {
            *no_data_to_send = 0;
        }
        Some(bytes)
    }
```

## `picoquic/sim_link.c:picoquictest_sim_link_testloss`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The shown Rust body is incomplete, ending at an else with no visible return/body.
* C source: `picoquic/sim_link.c:144-158`
* C signature: `int picoquictest_sim_link_testloss(uint64_t *)`
* Rust source: `rs/fq/src/tests/harness.rs:17-22`
* Rust item: `testloss`

### C body
```c
{
    uint64_t loss_bit = 0;

    if (loss_mask != NULL) {
        /* Last bit indicates loss or not */
        loss_bit = (uint64_t)((*loss_mask) & 1ull);

        /* Rotate loss mask by 1 to prepare next round */
        *loss_mask >>= 1;
        *loss_mask |= (loss_bit << 63);
    }

    return (int)loss_bit;
}
```

### Rust body
```rust
    if let Some(mask) = loss_mask.as_mut() {
        let loss_bit = *mask & 1;
        *mask = (*mask >> 1) | (loss_bit << 63);
        loss_bit != 0
    } else {
```

## `picoquic/spinbit.c:picoquic_spinbit_basic_incoming`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C incoming updates path current_spin from packet spin XOR client_mode; Rust body is outgoing logic returning a header bit and does not update path state.
* C source: `picoquic/spinbit.c:29-35`
* C signature: `void picoquic_spinbit_basic_incoming(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_header *)`
* Rust source: `rs/fq/src/spinbit.rs:25-37`
* Rust item: `incoming`

### C body
```c
{
    path_x->current_spin = ph->spin ^ cnx->client_mode;
}
```

### Rust body
```rust
    fn outgoing(&self, connection: &mut Connection) -> u8 {
        let spin = connection
            .paths
            .first()
            .map(|p| p.current_spin)
            .unwrap_or(false);
        (spin as u8) << 5
    }
```
