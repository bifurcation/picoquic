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

## `picoquic/frames.c:picoquic_queue_network_input`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust does not visibly apply consumed_offset or pass through is_last_frame/received_data behavior that the C body uses when adding chunks.
* C source: `picoquic/frames.c:1345-1405`
* C signature: `int picoquic_queue_network_input(picoquic_quic_t *, picosplay_tree_t *, uint64_t, uint64_t, const uint8_t *, size_t, int, picoquic_stream_data_node_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:16358-16412`
* Rust item: `queue_network_input`

### C body
```c
{
    const uint64_t input_begin = frame_data_offset;
    const uint64_t input_end = frame_data_offset + length;

    int ret = 0;

    /* Remove data that is already consumed */
    if (frame_data_offset < consumed_offset) {
        frame_data_offset = consumed_offset;
    }

    /* check for data that is already received in chunks with offset <= end */
    if (frame_data_offset < input_end) {

        picoquic_stream_data_node_t target;
        memset(&target, 0, sizeof(picoquic_stream_data_node_t));
        target.offset = frame_data_offset;

        picoquic_stream_data_node_t* prev = (picoquic_stream_data_node_t*)picosplay_find_previous(tree, &target);
        if (prev != NULL) {
            /* By definition, prev->offset <= frame_data_offset. Check whether the
             * beginning of the frame is already received and skip if necessary */
            const uint64_t prev_end = prev->offset + prev->length;
            frame_data_offset = frame_data_offset > prev_end ? frame_data_offset : prev_end;
        }

        picoquic_stream_data_node_t* next = (prev == NULL) ?
            (picoquic_stream_data_node_t*)picosplay_first(tree) :
            (picoquic_stream_data_node_t*)picosplay_next(&prev->stream_data_node);

        /* Check whether parts of the new frame are covered by already received chunks */
        while (ret == 0 && frame_data_offset < input_end && next != NULL && next->offset < input_end) {

            /* the tail of the frame overlaps with the next frame received */
            const uint64_t chunk_ofs = frame_data_offset;
            const uint64_t chunk_len = next->offset > frame_data_offset ? next->offset - frame_data_offset : 0;

            if (chunk_len > 0) {
                /* There is a gap between previous and next frame, and it will be at least partially filled */
                ret = add_chunk_node(quic, tree, chunk_ofs, (size_t)chunk_len, is_last_frame,
                    bytes + frame_data_offset - input_begin, new_data_available, received_data);
            }

            frame_data_offset = next->offset + next->length;
            next = (picoquic_stream_data_node_t*)picosplay_next(&next->stream_data_node);
        }

        /* no further already received chunk within the new frame */
        if (ret == 0 && frame_data_offset < input_end) {
            const uint64_t chunk_ofs = frame_data_offset;
            const uint64_t chunk_len = input_end - frame_data_offset;
            ret = add_chunk_node(quic, tree, chunk_ofs, (size_t)chunk_len, is_last_frame,
                bytes + frame_data_offset - input_begin, new_data_available, received_data);
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> crate::Result<()> {
    *new_data = false;
    if data.is_empty() {
        return Ok(());
    }

    let input_begin = offset;
    let input_end = offset.saturating_add(data.len() as u64);
    let mut cursor = input_begin;

    let mut existing = Vec::new();
    let mut tok = tree.inner.first();
    while let Some(st) = tok {
        if let Some((key, node)) = tree.inner.get_key_value(st) {
            existing.push((*key, node.offset.saturating_add(node.length as u64)));
        }
        tok = tree.inner.next(st);
    }

    for (seg_begin, seg_end) in existing {
        if seg_end <= cursor {
            continue;
        }
        if seg_begin >= input_end {
            break;
        }
        if cursor < seg_begin {
            let chunk_end = seg_begin.min(input_end);
            let src_off = (cursor - input_begin) as usize;
            let len = (chunk_end - cursor) as usize;
            insert_stream_data_chunk(tree, cursor, &data[src_off..src_off + len])?;
            *new_data = true;
        }
        cursor = cursor.max(seg_end);
        if cursor >= input_end {
            break;
        }
    }

    if cursor < input_end {
        let src_off = (cursor - input_begin) as usize;
        insert_stream_data_chunk(tree, cursor, &data[src_off..])?;
        *new_data = true;
    }

    Ok(())
}
```

## `picoquic/logger.c:picoquic_textlog_close`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C closes and clears a log file state; Rust body shown is log_app_message_v and logs an app message instead.
* C source: `picoquic/logger.c:2383-2391`
* C signature: `void picoquic_textlog_close(picoquic_quic_t *)`
* Rust source: `rs/fq/src/textlog.rs:85-100`
* Rust item: `textlog_close`

### C body
```c
{
    if (quic->F_log != NULL && quic->should_close_log) {
        (void)picoquic_file_close(quic->F_log);
    }

    quic->F_log = NULL;
    quic->should_close_log = 0;
}
```

### Rust body
```rust
    pub fn log_app_message_v(&mut self, args: core::fmt::Arguments<'_>) {
        crate::logger::Log::app_message(self, args);
    }
```

## `picoquic/logwriter.c:binlog_cc_dump`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body emits congestion-control update fields, while the Rust body shown emits a PacketDropped event with packet type, size, and error fields.
* C source: `picoquic/logwriter.c:1153-1231`
* C signature: `void binlog_cc_dump(picoquic_cnx_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/binlog.rs:1048-1092`
* Rust item: `cc_dump`

### C body
```c
{
    bytestream_buf stream_msg;
    bytestream* ps_msg = bytestream_buf_init(&stream_msg, BYTESTREAM_MAX_BUFFER_SIZE);

    picoquic_packet_context_t* pkt_ctx = &cnx->pkt_ctx[picoquic_packet_context_application];

    if (cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }

    /* Common chunk header */
    /* TODO: understand how to provide per path data -- most probably do a loop on
     * all available paths, and write the data for each path if multipath is enabled.
     * verify that it works for CSV and QLOG formats.
     */
    binlog_compose_event_header(ps_msg, &cnx->initial_cnxid, current_time,
        binlog_get_path_id(cnx, path_x), picoquic_log_event_cc_update);

    bytewrite_vint(ps_msg, pkt_ctx->send_sequence);

    if (pkt_ctx->highest_acknowledged != UINT64_MAX) {
        bytewrite_vint(ps_msg, 1);
        bytewrite_vint(ps_msg, pkt_ctx->highest_acknowledged);
        bytewrite_vint(ps_msg, pkt_ctx->highest_acknowledged_time - cnx->start_time);
        bytewrite_vint(ps_msg, pkt_ctx->latest_time_acknowledged - cnx->start_time);
    }
    else {
        bytewrite_vint(ps_msg, 0);
    }

    bytewrite_vint(ps_msg, path_x->cwin);
    bytewrite_vint(ps_msg, path_x->one_way_delay_sample);
    bytewrite_vint(ps_msg, path_x->rtt_sample);
    bytewrite_vint(ps_msg, path_x->smoothed_rtt);
    bytewrite_vint(ps_msg, path_x->rtt_min);
    bytewrite_vint(ps_msg, path_x->bandwidth_estimate);
    bytewrite_vint(ps_msg, path_x->receive_rate_estimate);
    bytewrite_vint(ps_msg, path_x->send_mtu);
    bytewrite_vint(ps_msg, path_x->pacing.packet_time_microsec);
    if (cnx->is_multipath_enabled) {
        bytewrite_vint(ps_msg, path_x->nb_losses_found);
        bytewrite_vint(ps_msg, path_x->nb_spurious);
    }
    else {
        bytewrite_vint(ps_msg, cnx->nb_retransmission_total);
        bytewrite_vint(ps_msg, cnx->nb_spurious);
    }
    bytewrite_vint(ps_msg, cnx->cwin_blocked);
    bytewrite_vint(ps_msg, cnx->flow_blocked);
    bytewrite_vint(ps_msg, cnx->stream_blocked);

    if (cnx->congestion_alg == NULL) {
        bytewrite_vint(ps_msg, 0);
        bytewrite_vint(ps_msg, 0);
    }
    else {
        uint64_t cc_state = 0;
        uint64_t cc_param = 0;

        if (cnx->path[0]->congestion_alg_state != NULL) {
            cnx->congestion_alg->alg_observe(cnx->path[0], &cc_state, &cc_param);
        }
        bytewrite_vint(ps_msg, cc_state);
        bytewrite_vint(ps_msg, cc_param);
    }

    bytewrite_vint(ps_msg, path_x->peak_bandwidth_estimate);
    bytewrite_vint(ps_msg, path_x->bytes_in_transit);
    bytewrite_vint(ps_msg, path_x->last_bw_estimate_path_limited);

    bytestream_buf stream_head;
    bytestream* ps_head = bytestream_buf_init(&stream_head, BYTESTREAM_MAX_BUFFER_SIZE);

    bytewrite_int32(ps_head, (uint32_t)bytestream_length(ps_msg));

    (void)fwrite(bytestream_data(ps_head), bytestream_length(ps_head), 1, cnx->f_binlog);
    (void)fwrite(bytestream_data(ps_msg), bytestream_length(ps_msg), 1, cnx->f_binlog);
}
```

### Rust body
```rust
    ) {
        if self.f_binlog.is_none() {
            return;
        }
        let cid = self.initial_connection_id;
        let path_id = get_path_id(self, path_x);

        let mut buf = ByteStreamBuf::default();
        let Some(mut msg) = buf.stream(BYTESTREAM_MAX_BUFFER_SIZE) else {
            return;
        };
        // Reserve four bytes for the chunk size; patch afterwards
        // — mirrors the C source.
        let _ = msg.write_u32(0);
        compose_event_header(
            &mut msg,
            &cid,
            current_time,
            path_id,
            LogEventType::PacketDropped,
        );
        let _ = msg.write_varint(ph.packet_type as u64);
        let _ = msg.write_varint(packet_size as u64);
        let _ = msg.write_varint(err as u64);

        let body_len = (msg.len().saturating_sub(4)) as u32;
        let mut payload = msg.as_bytes().to_vec();
        drop(msg);
        payload[..4].copy_from_slice(&body_len.to_be_bytes());

        if let Some(f) = self.f_binlog.as_mut() {
            let _ = f.write_all(&payload);
        }
    }
```

## `picoquic/logwriter.c:picoquic_enable_binlog`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C assigns quic->bin_log_fns, while Rust only takes a reference to self.bin_log_fns and leaves it unchanged; the body is placeholder-like.
* C source: `picoquic/logwriter.c:1346-1349`
* C signature: `void picoquic_enable_binlog(picoquic_quic_t *)`
* Rust source: `rs/fq/src/binlog.rs:1584-1597`
* Rust item: `enable_binlog`

### C body
```c
{
    quic->bin_log_fns = &binlog_functions;
}
```

### Rust body
```rust
    pub fn enable_binlog(&mut self) {
        // C: `quic->bin_log_fns = &binlog_functions;` installs the
        // unified-logging vtable.  In Rust the per-event binlog
        // dispatch lives directly on the [`Binlog`] trait
        // (implemented for [`Connection`]); the unified [`Logger`]
        // dispatch path on `Quic` is a no-op for an unconfigured
        // backend per `Connection::log_new_connection`.  Leaving
        // `bin_log_fns` as `None` matches the established pattern
        // in `set_qlog` / `set_textlog` and avoids an empty-shell
        // trait object — `is_still_logging` keys off `f_binlog`,
        // which is the source of truth for whether a record-writing
        // call should fire.
        let _ = &self.bin_log_fns;
    }
```

## `picoquic/newreno.c:picoquic_newreno_notify`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C handles many congestion notifications, updates cwin/state, resets, and recomputes pacing; Rust only takes congestion_alg_state or returns.
* C source: `picoquic/newreno.c:207-296`
* C signature: `void picoquic_newreno_notify(picoquic_cnx_t *, picoquic_path_t *, picoquic_congestion_notification_t, picoquic_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/newreno.rs:74-85`
* Rust item: `picoquic_newreno_notify`

### C body
```c
{
    picoquic_newreno_state_t* nr_state = (picoquic_newreno_state_t*)path_x->congestion_alg_state;

    path_x->is_cc_data_updated = 1;

    if (nr_state != NULL) {
        switch (notification) {
        /* RTT measurements will happen before acknowledgement is signalled */
        case picoquic_congestion_notification_acknowledgement:
            if (nr_state->nrss.alg_state == picoquic_newreno_alg_slow_start &&
                nr_state->nrss.ssthresh == UINT64_MAX) {
                /* Increase cwin based on bandwidth estimation. */
                path_x->cwin = picoquic_cc_update_target_cwin_estimation(path_x);
                nr_state->nrss.cwin = path_x->cwin;
            }

            if (path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                /* TODO app limited. */
                picoquic_newreno_sim_notify(&nr_state->nrss, cnx, path_x, notification, ack_state, current_time);
                path_x->cwin = nr_state->nrss.cwin;
            }
            break;
        case picoquic_congestion_notification_seed_cwin:
            picoquic_newreno_sim_notify(&nr_state->nrss, cnx, path_x, notification, ack_state, current_time);
            path_x->cwin = nr_state->nrss.cwin;
            break;
        case picoquic_congestion_notification_ecn_ec:
        case picoquic_congestion_notification_repeat:
        case picoquic_congestion_notification_timeout:
            /* TODO fix test cases first. */
            /* packet_trace qlog_trace qlog_trace_auto qlog_trace_only qlog_trace_ecn l4s_reno pacing_update
             * quality_update app_limited_reno multipath_callback multipath_quality  */
            /* if (picoquic_cc_hystart_loss_test(&nr_state->rtt_filter, notification, ack_state->lost_packet_number,
                PICOQUIC_SMOOTHED_LOSS_THRESHOLD)) { */
                picoquic_newreno_sim_notify(&nr_state->nrss, cnx, path_x, notification, ack_state, current_time);
                path_x->cwin = nr_state->nrss.cwin;
            /* } */
            break;
        case picoquic_congestion_notification_spurious_repeat:
            picoquic_newreno_sim_notify(&nr_state->nrss, cnx, path_x, notification, ack_state, current_time);
            path_x->cwin = nr_state->nrss.cwin;
            path_x->is_ssthresh_initialized = 1;
            break;
        case picoquic_congestion_notification_rtt_measurement:
            if (nr_state->nrss.alg_state == picoquic_newreno_alg_slow_start &&
                nr_state->nrss.ssthresh == UINT64_MAX){

                /* if in slow start, increase the window for long delay RTT */
                if (path_x->rtt_min > PICOQUIC_TARGET_RENO_RTT) {
                    path_x->cwin = picoquic_cc_update_cwin_for_long_rtt(path_x);
                    nr_state->nrss.cwin = path_x->cwin;
                }

                /* HyStart. */
                /* Using RTT increases as signal to get out of initial slow start */
                if (picoquic_cc_hystart_test(&nr_state->rtt_filter, (cnx->is_time_stamp_enabled) ? ack_state->one_way_delay : ack_state->rtt_measurement,
                    cnx->path[0]->pacing.packet_time_microsec, current_time, cnx->is_time_stamp_enabled)) {
                    /* RTT increased too much, get out of slow start! */
                    nr_state->nrss.ssthresh = nr_state->nrss.cwin;
                    nr_state->nrss.alg_state = picoquic_newreno_alg_congestion_avoidance;
                    path_x->cwin = nr_state->nrss.cwin;
                    path_x->is_ssthresh_initialized = 1;
                }
            }
            break;
        case picoquic_congestion_notification_reset:
            picoquic_newreno_reset(nr_state, path_x);
            break;
        default:
            /* ignore */
            break;
        }

        /* Compute pacing data */
        picoquic_update_pacing_data(path_x, nr_state->nrss.alg_state == picoquic_newreno_alg_slow_start &&
            nr_state->nrss.ssthresh == UINT64_MAX);
    }
}
```

### Rust body
```rust
    let Some(boxed_state) = path_x.congestion_alg_state.take() else {
        return;
    };
```
