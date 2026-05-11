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

## `picoquic/fastcc.c:fastcc_notify_congestion`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is an immediate return, while C updates freeze state, recovery sequence, congestion window, pacing data, and ssthresh flag.
* C source: `picoquic/fastcc.c:119-160`
* C signature: `void fastcc_notify_congestion(picoquic_cnx_t *, picoquic_path_t *, picoquic_fastcc_state_t *, uint64_t, int, int)`
* Rust source: `rs/fq/src/fastcc.rs:129-142`
* Rust item: `fastcc_notify_congestion`

### C body
```c
{
    if (fastcc_state->alg_state == picoquic_fastcc_freeze &&
        (!is_timeout || !fastcc_state->last_freeze_was_timeout) &&
        (!is_delay || !fastcc_state->last_freeze_was_not_delay)) {
        /* Do not treat additional events during same freeze interval */
        return;
    }
    fastcc_state->last_freeze_was_not_delay = !is_delay;
    fastcc_state->last_freeze_was_timeout = is_timeout;
    fastcc_state->alg_state = picoquic_fastcc_freeze;
    fastcc_state->end_of_freeze = current_time + fastcc_state->rtt_min;
    fastcc_state->recovery_sequence = picoquic_cc_get_sequence_number(cnx, path_x);
    fastcc_state->nb_cc_events = 0;

    if (is_delay) {
        path_x->cwin -= (uint64_t)(FASTCC_BETA * (double)path_x->cwin);
    }
    else {
        path_x->cwin = path_x->cwin / 2;
    }

    if (is_timeout || path_x->cwin < PICOQUIC_CWIN_MINIMUM) {
        path_x->cwin = PICOQUIC_CWIN_MINIMUM;
    }

    picoquic_update_pacing_data(path_x, 0);

    path_x->is_ssthresh_initialized = 1;
}
```

### Rust body
```rust
    {
        return;
    }
```

## `picoquic/frames.c:picoquic_compute_packets_in_window`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body as shown is incomplete and does not show the C function's final minimum clamp to 2 or return.
* C source: `picoquic/frames.c:2974-2995`
* C signature: `uint64_t picoquic_compute_packets_in_window(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:8938-8948`
* Rust item: `compute_packets_in_window`

### C body
```c
{
    uint64_t nb_packets = 0;

    if (cnx->is_ack_frequency_negotiated) {
        nb_packets = ((cnx->path[0]->cwin) / cnx->path[0]->send_mtu);
        /* TODO: in the case of BBR, the number of packets in transit is not
         * a function of CWIN, but rather rtt estimate * bottleneck bandwidth.
         * The current formulation works, but we could be more precise.
         */
    }
    else {
        /* Estimate the number of packets in flight from datarate and RTT */
        uint64_t rtt_bytes_times_1000000 = data_rate * cnx->path[0]->smoothed_rtt;
        uint64_t rtt_packets_times_1000000 = rtt_bytes_times_1000000 / cnx->path[0]->send_mtu;
        nb_packets = (rtt_packets_times_1000000 + 999999) / 1000000;
    }
    if (nb_packets < 2) {
        nb_packets = 2;
    }
    return nb_packets;
}
```

### Rust body
```rust
        let nb_packets = if let Some(path) = self.paths.first() {
            let send_mtu = path.send_mtu as u64;
            if self.is_ack_frequency_negotiated {
                path.cwin / send_mtu.max(1)
            } else {
                let rtt_bytes_times_1000000 = data_rate.saturating_mul(path.smoothed_rtt.ticks());
                let rtt_packets_times_1000000 = rtt_bytes_times_1000000 / send_mtu.max(1);
                rtt_packets_times_1000000.div_ceil(1_000_000)
            }
        } else {
```

## `picoquic/frames.c:picoquic_find_first_misc_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C scans and returns the first misc frame with matching pc without modifying the list; Rust formats/drains matching front datagram/misc frames into the byte buffer and returns remaining buffer space.
* C source: `picoquic/frames.c:4844-4854`
* C signature: `picoquic_misc_frame_header_t * picoquic_find_first_misc_frame(picoquic_cnx_t *, picoquic_packet_context_enum)`
* Rust source: `rs/fq/src/internal.rs:13774-13810`
* Rust item: `find_first_misc_frame`

### C body
```c
{
    picoquic_misc_frame_header_t* misc_frame = cnx->first_misc_frame;

    while (misc_frame != NULL && misc_frame->pc != pc) {
        misc_frame = misc_frame->next_misc_frame;
    }
    return misc_frame;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    // Drain all misc frames whose packet_context matches pc into bytes.
    let mut remaining = bytes;
    loop {
        let front_matches = connection
            .misc_frames
            .front()
            .map(|f| f.packet_context == pc)
            .unwrap_or(false);
        if !front_matches {
            break;
        }
        remaining = format_first_misc_or_dg_frame(
            remaining,
            more_data,
            is_pure_ack,
            &mut connection.misc_frames,
        )?;
    }
    Some(remaining)
}
```

## `picoquic/frames.c:picoquic_format_one_blocked_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust formats stream-blocked unconditionally once over the stream limit, while C checks the corresponding blocked-sent flag; Rust also returns after data-blocked instead of allowing a following stream-data-blocked frame.
* C source: `picoquic/frames.c:1750-1780`
* C signature: `uint8_t * picoquic_format_one_blocked_frame(picoquic_cnx_t *, uint8_t *, uint8_t *, int *, int *, picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/internal.rs:13704-13746`
* Rust item: `format_one_blocked_frame`

### C body
```c
{
    if (stream->is_active ||
        (stream->send_queue != NULL && stream->send_queue->length > stream->send_queue->offset)) {
        /* The stream has some data to send */
        /* if the stream is not active yet, verify that it fits under
            * the max stream id limit, which depends of the type of stream */
        if (IS_CLIENT_STREAM_ID(stream->stream_id) == cnx->client_mode &&
            stream->stream_id > ((IS_BIDIR_STREAM_ID(stream->stream_id)) ? cnx->max_stream_id_bidir_remote : cnx->max_stream_id_unidir_remote)) {
            if (!(IS_BIDIR_STREAM_ID(stream->stream_id) ? cnx->stream_blocked_bidir_sent : cnx->stream_blocked_unidir_sent))
            {
                /* Prepare a stream blocked frame */
                bytes = picoquic_format_stream_blocked_frame(cnx, bytes, bytes_max, more_data, is_pure_ack, stream);
            }
        }
        else {
            if (cnx->maxdata_remote <= cnx->data_sent && !cnx->sent_blocked_frame) {
                /* Prepare a blocked frame */
                bytes = picoquic_format_data_blocked_frame(cnx, bytes, bytes_max, more_data, is_pure_ack);
            }

            if (stream->sent_offset >= stream->maxdata_remote && !stream->stream_data_blocked_sent) {
                /* Prepare a stream data blocked frame */
                bytes = picoquic_format_stream_data_blocked_frame(bytes, bytes_max, more_data, is_pure_ack, stream);
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let sid = crate::stream::StreamId(stream.stream_id);
    let local_role = if connection.client_mode {
        crate::stream::Role::Client
    } else {
        crate::stream::Role::Server
    };
    let has_data = stream.is_active
        || stream
            .send_queue
            .front()
            .map(|q| (q.offset as usize) < q.bytes.len())
            .unwrap_or(false);
    if !has_data {
        return Some(bytes);
    }

    if sid.is_local(local_role)
        && stream.stream_id
            > if sid.is_bidir() {
                connection.max_stream_id_bidir_remote
            } else {
                connection.max_stream_id_unidir_remote
            }
    {
        return format_stream_blocked_frame(connection, bytes, more_data, is_pure_ack, stream);
    }

    if connection.maxdata_remote <= connection.data_sent && !connection.sent_blocked_frame {
        return format_data_blocked_frame(connection, bytes, more_data, is_pure_ack);
    }

    if stream.sent_offset >= stream.maxdata_remote && !stream.stream_data_blocked_sent {
        return format_stream_data_blocked_frame(bytes, more_data, is_pure_ack, stream);
    }
    Some(bytes)
}
```

## `picoquic/frames.c:picoquic_process_ack_of_frames`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C parses and processes all acknowledged frames in a loop; Rust body only clears packet queue flags.
* C source: `picoquic/frames.c:3706-3850`
* C signature: `void picoquic_process_ack_of_frames(picoquic_cnx_t *, picoquic_packet_t *, int)`
* Rust source: `rs/fq/src/internal.rs:11946-11953`
* Rust item: `process_ack_of_frames`

### C body
```c
{
    int ret = 0;
    size_t byte_index;
    int frame_is_pure_ack = 0;
    size_t frame_length = 0;

    if (p->ptype == picoquic_packet_0rtt_protected) {
        cnx->nb_zero_rtt_acked++;
    }

    byte_index = p->offset;

    while (ret == 0 && byte_index < p->length) {
        uint64_t ftype;
        size_t l_ftype = picoquic_varint_decode(&p->bytes[byte_index], p->length - byte_index, &ftype);
        if (l_ftype == 0) {
            break;
        }

        switch (ftype) {
        case picoquic_frame_type_ack:
            ret = picoquic_process_ack_of_ack_frame(&cnx->ack_ctx[p->pc].sack_list,
                &p->bytes[byte_index], p->length - byte_index, &frame_length, 0);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_ack_ecn:
            ret = picoquic_process_ack_of_ack_frame(&cnx->ack_ctx[p->pc].sack_list,
                &p->bytes[byte_index], p->length - byte_index, &frame_length, 1);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_path_ack:
            ret = picoquic_process_ack_of_path_ack_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length, 0);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_path_ack_ecn:
            ret = picoquic_process_ack_of_path_ack_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length, 1);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_handshake_done:
            cnx->is_handshake_done_acked = 1;
            byte_index += l_ftype;
            break;
        case picoquic_frame_type_new_connection_id:
            ret = picoquic_process_ack_of_new_cid_frame(cnx, &p->bytes[byte_index], p->length - byte_index, 0, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_path_new_connection_id:
            ret = picoquic_process_ack_of_new_cid_frame(cnx, &p->bytes[byte_index], p->length - byte_index, 1, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_retire_connection_id:
            ret = picoquic_process_ack_of_retire_connection_id_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length, 0);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_path_retire_connection_id:
            ret = picoquic_process_ack_of_retire_connection_id_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length, 1);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_crypto_hs:
            ret = picoquic_process_ack_of_crypto_frame(cnx, &p->bytes[byte_index], p->length - byte_index, p->ptype, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_new_token:
            ret = picoquic_skip_frame(&p->bytes[byte_index],
                p->length - byte_index, &frame_length, &frame_is_pure_ack);
            byte_index += frame_length;
            cnx->is_new_token_acked = 1;
            break;
        case picoquic_frame_type_max_data:
            ret = picoquic_process_ack_of_max_data_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_max_stream_data:
            ret = picoquic_process_ack_of_max_stream_data_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_max_streams_bidir:
        case picoquic_frame_type_max_streams_unidir:
            ret = picoquic_process_ack_of_max_streams_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_reset_stream:
            ret = picoquic_process_ack_of_reset_stream_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_max_path_id:
            ret = picoquic_process_ack_of_max_path_id_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_paths_blocked:
            ret = picoquic_process_ack_of_paths_blocked_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_path_cid_blocked:
            ret = picoquic_process_ack_of_path_cid_blocked_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length);
            byte_index += frame_length;
            break;
        case picoquic_frame_type_observed_address_v4:
        case picoquic_frame_type_observed_address_v6:
            ret = picoquic_process_ack_of_observed_address_frame(p->send_path, &p->bytes[byte_index], p->length - byte_index, ftype, &frame_length);
            byte_index += frame_length;
            break;
        default:
            if (PICOQUIC_IN_RANGE(ftype, picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max)) {
                ret = picoquic_process_ack_of_stream_frame(cnx, &p->bytes[byte_index], p->length - byte_index, &frame_length);
                byte_index += frame_length;
                if (p->send_path != NULL) {
                    if (p->send_time > p->send_path->last_time_acked_data_frame_sent) {
                        p->send_path->last_time_acked_data_frame_sent = p->send_time;
                    }
                }
            }
            else {
                if (PICOQUIC_IN_RANGE(ftype, picoquic_frame_type_datagram, picoquic_frame_type_datagram_l)) {
                    if (p->send_path != NULL && p->send_time > p->send_path->last_time_acked_data_frame_sent) {
                        p->send_path->last_time_acked_data_frame_sent = p->send_time;
                    }
                    if (cnx->callback_fn != NULL) {
                        uint8_t frame_id;
                        uint64_t content_length;
                        uint8_t* content_bytes;

                        /* Parse and skip type and length */
                        content_bytes = picoquic_decode_datagram_frame_header(&p->bytes[byte_index], &p->bytes[p->length],
                            &frame_id, &content_length);

                        ret = (cnx->callback_fn)(cnx, p->send_time, content_bytes, (size_t)content_length,
                            (is_spurious) ? picoquic_callback_datagram_spurious : picoquic_callback_datagram_acked,
                            cnx->callback_ctx, NULL);
                    }
                }

                ret = picoquic_skip_frame(&p->bytes[byte_index],
                    p->length - byte_index, &frame_length, &frame_is_pure_ack);
                byte_index += frame_length;
            }
            break;
        }
    }
}
```

### Rust body
```rust
    pub fn process_ack_of_frames(&mut self, p: &mut Packet, is_spurious: i32) {
        p.is_queued_for_retransmit = false;
        p.is_queued_for_spurious_detection = false;
        if is_spurious == 0 {
            p.is_queued_for_data_repeat = false;
            p.queue_data_repeat_membership = None;
        }
    }
```
