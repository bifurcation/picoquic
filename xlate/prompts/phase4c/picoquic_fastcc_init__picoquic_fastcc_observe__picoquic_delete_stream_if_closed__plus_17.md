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

## Pair `picoquic/fastcc.c:picoquic_fastcc_init`
C: `picoquic/fastcc.c:94-117 picoquic_fastcc_init`
Rust: `rs/fq/src/fastcc.rs:112-124 picoquic_fastcc_init`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    picoquic_fastcc_state_t* fastcc_state = path_x->congestion_alg_state;
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(option_string);
#endif
    
    if (fastcc_state == NULL) {
        fastcc_state = (picoquic_fastcc_state_t*)malloc(sizeof(picoquic_fastcc_state_t));
    }
    
    if (fastcc_state != NULL) {
        memset(fastcc_state, 0, sizeof(picoquic_fastcc_state_t));
        fastcc_state->alg_state = picoquic_fastcc_initial;
        fastcc_state->rtt_min = path_x->smoothed_rtt;
        fastcc_state->rolling_rtt_min = fastcc_state->rtt_min;
        fastcc_state->delay_threshold = picoquic_fastcc_delay_threshold(fastcc_state->rtt_min);
        fastcc_state->end_of_epoch = current_time + FASTCC_PERIOD;
        path_x->cwin = PICOQUIC_CWIN_INITIAL;
    }

    path_x->congestion_alg_state = (void*)fastcc_state;
}
```

### Rust body
```rust
) {
    let mut state = path_x
        .congestion_alg_state
        .take()
        .and_then(|boxed| boxed.downcast::<FastccState>().ok())
        .unwrap_or_default();
    picoquic_fastcc_reset(&mut state, path_x, current_time);
    path_x.congestion_alg_state = Some(state);
}
```

## Pair `picoquic/fastcc.c:picoquic_fastcc_observe`
C: `picoquic/fastcc.c:320-325 picoquic_fastcc_observe`
Rust: `rs/fq/src/fastcc.rs:325-333 fastcc_observe`

### C body
```c
{
    picoquic_fastcc_state_t* fastcc_state = (picoquic_fastcc_state_t*)path_x->congestion_alg_state;
    *cc_state = (uint64_t)fastcc_state->alg_state;
    *cc_param = fastcc_state->rolling_rtt_min;
}
```

### Rust body
```rust
    {
        *cc_state = state.alg_state as u64;
        *cc_param = state.rolling_rtt_min;
    }
```

## Pair `picoquic/frames.c:picoquic_delete_stream_if_closed`
C: `picoquic/frames.c:134-156 picoquic_delete_stream_if_closed`
Rust: `rs/fq/src/internal.rs:9509-9514 delete_stream_if_closed`

### C body
```c
{
    int ret = 0;

    if (!stream->is_closed && picoquic_is_stream_closed(stream, cnx->client_mode)) {
        picoquic_update_max_stream_ID_local(cnx, stream);
        stream->is_closed = 1;
        ret = 1;
    }
    
    /* We only delete the stream if there are no pending retransmissions.
    * For bidirectional streams, or for local unidir streams, that means
    * waiting for acknowledgements. For remote unidir streams, we won't have any ack,
    * so we can delete the stream immediately.
    */
    if (stream->is_closed && (
        (!IS_BIDIR_STREAM_ID(stream->stream_id) && !IS_LOCAL_STREAM_ID(stream->stream_id, cnx->client_mode)) ||
        picoquic_is_stream_acked(stream))) {
        picoquic_delete_stream(cnx, stream);
    }

    return ret;
}
```

### Rust body
```rust
        if !stream.is_closed && stream.is_stream_closed(self.client_mode) {
            stream.is_closed = true;
            ret = 1;
        }
```

## Pair `picoquic/frames.c:picoquic_format_reset_stream_frame`
C: `picoquic/frames.c:284-303 picoquic_format_reset_stream_frame`
Rust: `rs/fq/src/internal.rs:9766-9791 format_reset_stream_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_reset_stream)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->stream_id)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->local_error)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->sent_offset)) != NULL)
    {
        *is_pure_ack = 0;
        picoquic_enforce_reset_stream_frame(stream, stream->reliable_size);
    }
    else {
        *more_data = 1;
        bytes = bytes0;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let stream_id = stream.stream_id;
    let local_error = stream.local_error;
    let sent_offset = stream.sent_offset;
    let reliable_size = stream.reliable_size;
    let mut off = 0;
    for value in [
        crate::frames::FrameType::ResetStream as u64,
        stream_id,
        local_error,
        sent_offset,
    ] {
        if !encode_varint_at(bytes, &mut off, value) {
            *more_data = 1;
            return Some(bytes);
        }
    }
    *is_pure_ack = 0;
    stream.enforce_reset_stream_frame(reliable_size);
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_format_new_connection_id_frame`
C: `picoquic/frames.c:593-619 picoquic_format_new_connection_id_frame`
Rust: `rs/fq/src/internal.rs:13364-13405 format_new_connection_id_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    unsigned int is_mp = cnx->is_multipath_enabled;

    if (l_cid != NULL && l_cid->cnx_id.id_len > 0) {
        if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, 
            (is_mp)?picoquic_frame_type_path_new_connection_id:picoquic_frame_type_new_connection_id)) == NULL ||
            (is_mp && ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, l_cid->path_id)) == NULL)) ||
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, l_cid->sequence)) == NULL ||
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, local_cnxid_list->local_cnxid_retire_before)) == NULL ||
            (bytes = picoquic_frames_cid_encode(bytes, bytes_max, &l_cid->cnx_id)) == NULL ||
            (bytes + PICOQUIC_RESET_SECRET_SIZE) > bytes_max) {
            *more_data = 1;
            bytes = bytes0;
        }
        else {
            *is_pure_ack = 0;
            (void)picoquic_create_cnxid_reset_secret(cnx->quic, &l_cid->cnx_id, bytes);
            bytes += PICOQUIC_RESET_SECRET_SIZE;
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let token = l_cid.or_else(|| local_connection_id_list.connection_ids.first().copied())?;
    let cid = connection.local_connection_ids.get(token)?;
    let frame_type = if connection.is_multipath_enabled {
        crate::frames::FrameType::PathNewConnectionId as u64
    } else {
        crate::frames::FrameType::NewConnectionId as u64
    };
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, frame_type)
        || (connection.is_multipath_enabled
            && !encode_varint_at(bytes, &mut off, local_connection_id_list.unique_path_id))
        || !encode_varint_at(bytes, &mut off, cid.sequence)
        || !encode_varint_at(
            bytes,
            &mut off,
            local_connection_id_list.local_connection_id_retire_before,
        )
        || bytes.len() < off + 1 + cid.connection_id.len() + RESET_SECRET_SIZE
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off] = cid.connection_id.len() as u8;
    off += 1;
    bytes[off..off + cid.connection_id.len()].copy_from_slice(cid.connection_id.as_bytes());
    off += cid.connection_id.len();
    bytes[off..off + RESET_SECRET_SIZE].copy_from_slice(&connection.registered_reset_secret);
    off += RESET_SECRET_SIZE;
    if let Some(cid) = connection.local_connection_ids.get_mut(token) {
        cid.is_acked = true;
    }
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_queue_new_token_frame`
C: `picoquic/frames.c:1029-1043 picoquic_queue_new_token_frame`
Rust: `rs/fq/src/internal.rs:13575-13589 queue_new_token_frame`

### C body
```c
{
    int ret = 0;
    int more_data = 0;
    int is_pure_ack = 1;
    uint8_t frame_buffer[258];
    uint8_t* bytes = picoquic_format_new_token_frame(frame_buffer, frame_buffer + sizeof(frame_buffer), &more_data, &is_pure_ack, token, token_length);

    if (bytes > frame_buffer) {
        ret = picoquic_queue_misc_frame(cnx, frame_buffer, bytes - frame_buffer, 1,
            picoquic_packet_context_application);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn queue_new_token_frame(&mut self, token: &[u8]) -> Result<(), crate::Error> {
        let capacity = 1 + encode_varint_length(token.len() as u64) + token.len();
        let mut frame = vec![0u8; capacity];
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let tail_len = format_new_token_frame(&mut frame, &mut more_data, &mut is_pure_ack, token)
            .map(|tail| tail.len())
            .ok_or(crate::Error::BufferTooSmall)?;
        if more_data != 0 {
            return Err(crate::Error::BufferTooSmall);
        }
        frame.truncate(capacity - tail_len);
        encode_misc_frame(self, frame, true, PacketContext::Application);
        Ok(())
    }
```

## Pair `picoquic/frames.c:picoquic_queue_network_input`
C: `picoquic/frames.c:1345-1405 picoquic_queue_network_input`
Rust: `rs/fq/src/internal.rs:16358-16412 queue_network_input`

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

## Pair `picoquic/frames.c:picoquic_format_data_blocked_frame`
C: `picoquic/frames.c:1674-1690 picoquic_format_data_blocked_frame`
Rust: `rs/fq/src/internal.rs:13474-13493 format_data_blocked_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_data_blocked)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, cnx->maxdata_remote)) != NULL) {
        *is_pure_ack = 0;
        cnx->sent_blocked_frame = 1;
    }
    else {
        *more_data = 1;
        bytes = bytes0;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::DataBlocked as u64,
    ) || !encode_varint_at(bytes, &mut off, connection.maxdata_remote)
    {
        *more_data = 1;
        return Some(bytes);
    }
    *is_pure_ack = 0;
    connection.sent_blocked_frame = true;
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_format_blocked_frames`
C: `picoquic/frames.c:1782-1805 picoquic_format_blocked_frames`
Rust: `rs/fq/src/internal.rs:13495-13505 format_blocked_frames`

### C body
```c
{
    picoquic_stream_head_t* stream = picoquic_first_stream(cnx);
    picoquic_stream_head_t* hi_pri_stream = NULL;

    /* Check whether there is a high priority stream declared */
    if (cnx->high_priority_stream_id != UINT64_MAX) {
        hi_pri_stream = picoquic_find_stream(cnx, cnx->high_priority_stream_id);
    }

    /* Look for blocked streams, as long as there is message space available */
    while (stream != NULL) {
        if (hi_pri_stream == NULL || stream == hi_pri_stream) {
            bytes = picoquic_format_one_blocked_frame(cnx, bytes, bytes_max, more_data, is_pure_ack, stream);
            if (*more_data) {
                break;
            }
        }

        stream = picoquic_next_stream(stream);
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    if connection.maxdata_remote <= connection.data_sent && !connection.sent_blocked_frame {
        return format_data_blocked_frame(connection, bytes, more_data, is_pure_ack);
    }
    Some(bytes)
}
```

## Pair `picoquic/frames.c:picoquic_format_stream_frame`
C: `picoquic/frames.c:1880-2077 picoquic_format_stream_frame`
Rust: `rs/fq/src/internal.rs:9998-10016 format_stream_frame`

### C body
```c
{
    int may_close = 0;
    *ret = 0;

    /* Check parity */
    if (IS_CLIENT_STREAM_ID(stream->stream_id) == cnx->client_mode) {
        if (stream->stream_id > ((IS_BIDIR_STREAM_ID(stream->stream_id)) ? cnx->max_stream_id_bidir_remote : cnx->max_stream_id_unidir_remote)) {
            /* Attempting to send data on a forbidden stream is a protocol error */
            return NULL;
        }
    }

    if (stream->stop_sending_requested && !stream->stop_sending_sent) {
        return picoquic_format_stop_sending_frame(stream, bytes, bytes_max, more_data, is_pure_ack);
    }

    if (stream->reset_sent) {
        /* No data will be sent after a reset */
        return bytes;
    }
    else if (stream->reset_requested) {
        if (stream->reliable_size > 0) {
            if (picoquic_check_sack_list(&stream->sack_list, 0, stream->reliable_size)) {
                return picoquic_format_reset_stream_at_frame(stream, bytes, bytes_max, more_data, is_pure_ack);
            }
        }
        else {
            return picoquic_format_reset_stream_frame(stream, bytes, bytes_max, more_data, is_pure_ack);
        }
    }

    if (!stream->is_active &&
        (stream->send_queue == NULL || stream->send_queue->length <= stream->send_queue->offset) &&
        (!stream->fin_requested || stream->fin_sent)) {
        /* Nothing to send */
    }
    else {
        uint8_t* bytes0 = bytes;
        size_t byte_index = 0;
        size_t length = 0;

        if ((bytes = picoquic_format_stream_frame_header(bytes, bytes_max, stream->stream_id, stream->sent_offset)) == NULL) {
            bytes = bytes0;
            *more_data = 1;
        } else {
            /* Compute the length */
            size_t byte_space = bytes_max - bytes;
            size_t allowed_space = byte_space;

            /* Enforce maxdata per stream on all streams, including stream 0
             * This may result in very short encoding, but we still send whatever is
             * allowed by flow control. Doing otherwise may cause a loop if the
             * "find_ready_stream" function did not completely replicate the
             * flow control test */
            if (allowed_space > (stream->maxdata_remote - stream->sent_offset)) {
                allowed_space = (size_t)(stream->maxdata_remote - stream->sent_offset);
            }

            if (allowed_space > (cnx->maxdata_remote - cnx->data_sent)) {
                allowed_space = (size_t)(cnx->maxdata_remote - cnx->data_sent);
            }

            if (stream->is_active && stream->send_queue == NULL && !stream->fin_requested) {
                /* The application requested active polling for this stream */
                picoquic_stream_data_buffer_argument_t stream_data_context;

                stream_data_context.bytes = bytes0;
                stream_data_context.byte_index = bytes - bytes0;
                stream_data_context.allowed_space = allowed_space;
                stream_data_context.byte_space = bytes_max - bytes;
                stream_data_context.length = 0;
                stream_data_context.is_fin = 0;
                stream_data_context.is_still_active = 0;
                stream_data_context.app_buffer = NULL;

                if ((cnx->callback_fn)(cnx, stream->stream_id, (uint8_t*)&stream_data_context, allowed_space, picoquic_callback_prepare_to_send, cnx->callback_ctx, stream->app_stream_ctx) != 0) {
                    /* something went wrong */
                    picoquic_log_app_message(cnx, "Prepare to send returns error 0x%x", PICOQUIC_TRANSPORT_INTERNAL_ERROR);
                    *ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0,
                        "Prepare to send callback");
                    bytes = bytes0; /* CHECK: SHOULD THIS BE NULL ? */
                }
                else if (stream_data_context.length == 0 && stream_data_context.is_fin == 0) {
                    /* The application did not send any data */
                    bytes = bytes0;
                    stream->is_active = stream_data_context.is_still_active;
                }
                else
                {
                    bytes = bytes0 + stream_data_context.byte_index + stream_data_context.length;
                    stream->sent_offset += stream_data_context.length;
                    stream->last_time_data_sent = picoquic_get_quic_time(cnx->quic);
                    cnx->data_sent += stream_data_context.length;

                    if (stream_data_context.length > 0) {
                        if (stream_data_context.app_buffer == NULL ||
                            stream_data_context.app_buffer < bytes0 ||
                            stream_data_context.app_buffer >= bytes_max) {
                            long long delta_buf = (long long)(stream_data_context.app_buffer - bytes);
                            picoquic_log_app_message(cnx, "Stream data buffer corruption, delta = %lld\n", delta_buf);
                            *ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0,
                                "Stream data buffer corruption");
                        }
                    }

                    if (stream_data_context.is_fin) {
                        stream->is_active = 0;
                        stream->fin_requested = 1;
                        stream->fin_sent = 1;

                        picoquic_remove_output_stream(cnx, stream);

                        picoquic_update_max_stream_ID_local(cnx, stream);
                        may_close = 1;

                        if (is_still_active != NULL) {
                            *is_still_active = 0;
                        }
                    }
                    else {
                        stream->is_active = stream_data_context.is_still_active;
                        if (is_still_active != NULL) {
                            *is_still_active = stream_data_context.is_still_active;
                        }
                    }
                }
            }
            else {
                /* The application queued data for this stream */
                size_t start_index = 0;

                byte_index = bytes - bytes0;

                if (stream->send_queue == NULL) {
                    length = 0;
                }
                else {
                    length = (size_t)(stream->send_queue->length - stream->send_queue->offset);
                }

                if (length >= allowed_space) {
                    length = allowed_space;
                }

                byte_index = picoquic_encode_length_of_stream_frame(bytes0, byte_index, byte_space, length, &start_index);

                if (length > 0 && stream->send_queue != NULL && stream->send_queue->bytes != NULL) {
                    memcpy(&bytes0[byte_index], stream->send_queue->bytes + stream->send_queue->offset, length);
                    byte_index += length;

                    stream->send_queue->offset += length;
                    if (stream->send_queue->offset >= stream->send_queue->length) {
                        picoquic_stream_queue_node_t* next = stream->send_queue->next_stream_data;
                        free(stream->send_queue->bytes);
                        free(stream->send_queue);
                        stream->send_queue = next;
                    }

                    stream->sent_offset += length;
                    stream->last_time_data_sent = picoquic_get_quic_time(cnx->quic);
                    cnx->data_sent += length;
                }

                bytes = bytes0 + byte_index;

                if (stream->send_queue == NULL) {
                    if (stream->fin_requested) {
                        /* Set the fin bit -- target the start_index octet, to match behavior of length encoding */
                        stream->fin_sent = 1;
                        bytes0[start_index] |= 1;

                        picoquic_update_max_stream_ID_local(cnx, stream);
                        may_close = 1;
                    }
                }
                else if (length == 0) {
                    /* No point in sending a silly packet */
                    bytes = bytes0;
                    *more_data = 1;
                }
            }
        }

        if (*ret == 0) {
            *is_pure_ack &= (bytes == bytes0);

            if (!may_close || !picoquic_delete_stream_if_closed(cnx, stream)) {
                /* mark the stream as unblocked since we sent something */
                stream->stream_data_blocked_sent = 0;
                cnx->sent_blocked_frame = 0;
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    encode_stream_like_frame(
        stream,
        bytes,
        more_data,
        is_pure_ack,
        is_still_active,
        ret,
        false,
    )
}
```

## Pair `picoquic/frames.c:picoquic_queue_data_repeat_compare`
C: `picoquic/frames.c:2144-2172 picoquic_queue_data_repeat_compare`
Rust: `rs/fq/src/internal.rs:1032-1053 queue_data_repeat_compare`

### C body
```c
{
    picoquic_packet_t* lp = (picoquic_packet_t*)picoquic_queue_data_repeat_node_value(l);
    picoquic_packet_t* rp = (picoquic_packet_t*)picoquic_queue_data_repeat_node_value(r);
    int64_t ret = 0;
    /* TODO: comparison function is wrong, because the "data_repeat_frame" value
     * varies over time. Also, the result may not be unique.
     */

    /* Lower means more urgent, goes in front */
    if (lp->data_repeat_priority > rp->data_repeat_priority) {
        ret = 1;
    }
    else if (lp->data_repeat_priority < rp->data_repeat_priority) {
        ret = -1;
    }
    else {
        ret = lp->data_repeat_stream_id - rp->data_repeat_stream_id;
        if (ret == 0) {
            ret = lp->data_repeat_stream_offset - rp->data_repeat_stream_offset;
            if (ret == 0) {
                /* largest length goes in front */
                ret = rp->data_repeat_stream_data_length - lp->data_repeat_stream_data_length;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
pub fn queue_data_repeat_compare(left: &Packet, right: &Packet) -> i64 {
    if left.data_repeat_priority > right.data_repeat_priority {
        1
    } else if left.data_repeat_priority < right.data_repeat_priority {
        -1
    } else {
        let mut ret = signed_u64_diff(left.data_repeat_stream_id, right.data_repeat_stream_id);
        if ret == 0 {
            ret = signed_u64_diff(
                left.data_repeat_stream_offset,
                right.data_repeat_stream_offset,
            );
            if ret == 0 {
                ret = signed_usize_diff(
                    right.data_repeat_stream_data_length,
                    left.data_repeat_stream_data_length,
                );
            }
        }
        ret
    }
}
```

## Pair `picoquic/frames.c:picoquic_queue_data_repeat_packet`
C: `picoquic/frames.c:2254-2268 picoquic_queue_data_repeat_packet`
Rust: `rs/fq/src/internal.rs:10450-10460 queue_data_repeat_packet`

### C body
```c
{
    if (!packet->is_queued_for_data_repeat) {
        /* The stream frame, stream ID, priority are reset in the packet
         * header by the call to picoquic_queue_data_repeat_adjust */
        packet->data_repeat_frame = packet->offset;
        packet->data_repeat_index = packet->offset;
        if (picoquic_queue_data_repeat_adjust(cnx, packet) == 0 &&
            packet->data_repeat_frame < packet->length) {
            picosplay_insert(&cnx->queue_data_repeat_tree, packet);
            packet->is_queued_for_data_repeat = 1;
        }
    }
}
```

### Rust body
```rust
        {
            self.queue_data_repeat_packet_current(packet);
        }
```

## Pair `picoquic/frames.c:picoquic_copy_stream_frames_for_retransmit`
C: `picoquic/frames.c:2462-2501 picoquic_copy_stream_frames_for_retransmit`
Rust: `rs/fq/src/internal.rs:10711-10734 copy_stream_frames_for_retransmit`

### C body
```c
{
    int more_retransmit = 0;
    int packet_dequeued = 0;
    uint8_t* bytes_first = bytes_next;
    picoquic_packet_t* packet = NULL;
    do {
        packet_dequeued = 0;
        packet = picoquic_first_data_repeat_packet(cnx);
        if (packet == NULL) {
            break;
        } else if (packet->data_repeat_priority > current_priority) {
            more_retransmit = 1;
            break;
        }
        else {
            more_retransmit = 0;
            bytes_next = picoquic_copy_single_stream_frame_for_retransmit(cnx, packet, 
                bytes_next, bytes_max, &more_retransmit, &packet_dequeued, is_pure_ack);
        }
    } while (bytes_next != NULL && packet_dequeued /* bytes_first < bytes_next */ && bytes_next < bytes_max);

    /* The call to copy frame can fail if the data in memory is somehow corrupted,
    * which mainly happens if we are engaged in fuzzing. In that case, we 
    * need to generate an internal error, but also let the pointer to
    * a reasonable value */
    if (bytes_next == NULL) {
        (void)picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0, "data frame was fuzzed, cannot be resent");
        bytes_next = bytes_first;
    }

    if (packet_dequeued) {
        more_retransmit = (picoquic_first_data_repeat_packet(cnx) != NULL);
    }

    *more_data |= more_retransmit;

    return bytes_next;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let Some(packet_token) = connection.first_data_repeat_packet() else {
        return Some(bytes);
    };
    let packet = connection.queued_packets.get_mut(packet_token)?;
    let start = packet.data_repeat_frame;
    let end = packet.length.min(packet.bytes.len());
    if start >= end || bytes.len() < end - start {
        return None;
    }
    let len = end - start;
    bytes[..len].copy_from_slice(&packet.bytes[start..end]);
    *is_pure_ack = 0;
    if connection.first_data_repeat_packet().is_some() {
        *more_data = 1;
    }
    Some(&mut bytes[len..])
}
```

## Pair `picoquic/frames.c:picoquic_parse_ack_header`
C: `picoquic/frames.c:2712-2755 picoquic_parse_ack_header`
Rust: `rs/fq/src/internal.rs:12340-12393 parse_ack_header`

### C body
```c
{
    int ret = 0;
    size_t byte_index = picoquic_decode_varint_length(bytes[0]);
    size_t l_largest = 0;
    size_t l_delay = 0;
    size_t l_blocks = 0;
    size_t l_path_id = 0;

    if (path_id != NULL && bytes_max > byte_index) {
        l_path_id = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, path_id);
        byte_index += l_path_id;
    }

    if (bytes_max > byte_index) {
        l_largest = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, largest);
        byte_index += l_largest;
    }

    if (bytes_max > byte_index) {
        l_delay = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, ack_delay);
        *ack_delay <<= ack_delay_exponent;
        byte_index += l_delay;
    }

    if (bytes_max > byte_index) {
        l_blocks = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, num_block);
        byte_index += l_blocks;
    }

    if (l_largest == 0 || l_delay == 0 || l_blocks == 0 || bytes_max < byte_index ||
        (path_id != NULL && l_path_id == 0)) {
        DBG_PRINTF("ack frame fixed header too large: first_byte=0x%02x, bytes_max=%" PRIst,
            bytes[0], bytes_max);
        byte_index = bytes_max;
        ret = -1;
    }

    *consumed = byte_index;
    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let max = bytes_max.min(bytes.len());
    if max == 0 {
        *consumed = 0;
        return -1;
    }

    let mut frame_type = 0;
    let mut off = varint_decode(&bytes[..max], &mut frame_type);
    if off == 0 {
        *consumed = max;
        return -1;
    }

    let is_path_ack = frame_type == crate::frames::FrameType::PathAck as u64
        || frame_type == crate::frames::FrameType::PathAckEcn as u64;
    if is_path_ack {
        let l_path = varint_decode(&bytes[off..max], path_id);
        off += l_path;
        if l_path == 0 {
            *consumed = max;
            return -1;
        }
    } else {
        *path_id = 0;
    }

    let l_largest = varint_decode(&bytes[off..max], largest);
    off += l_largest;
    let l_delay = varint_decode(&bytes[off..max], ack_delay);
    *ack_delay = ack_delay
        .checked_shl(ack_delay_exponent as u32)
        .unwrap_or(0);
    off += l_delay;
    let l_blocks = varint_decode(&bytes[off..max], num_block);
    off += l_blocks;

    if l_largest == 0 || l_delay == 0 || l_blocks == 0 || off > max {
        *consumed = max;
        -1
    } else {
        *consumed = off;
        0
    }
}
```

## Pair `picoquic/frames.c:picoquic_compute_ack_gap`
C: `picoquic/frames.c:2997-3045 picoquic_compute_ack_gap`
Rust: `rs/fq/src/internal.rs:8960-9015 compute_ack_gap`

### C body
```c
{
    uint64_t ack_gap;
    uint64_t ack_gap_min = 2;

    if (cnx->is_ack_frequency_negotiated && !cnx->path[0]->is_ssthresh_initialized) {
        nb_packets /= 2;
    }
    if (cnx->path[0]->rtt_min < 4 * PICOQUIC_ACK_DELAY_MIN) {
        uint64_t mult = 4;
        if (cnx->path[0]->rtt_min > PICOQUIC_ACK_DELAY_MIN) {
            mult = ((uint64_t)(4 * PICOQUIC_ACK_DELAY_MIN)) / cnx->path[0] ->rtt_min;
        }
        nb_packets *= mult;
    }

    ack_gap = (nb_packets + 3) / 4;

    if (data_rate > PICOQUIC_BANDWIDTH_MEDIUM) {
        if (cnx->path[0]->rtt_min > PICOQUIC_TARGET_RENO_RTT) {
            ack_gap_min = 10;
        }
        else {
            ack_gap_min = 4;
        }
    }

    if (ack_gap < ack_gap_min) {
        ack_gap = ack_gap_min;
    }
    else if (ack_gap > 32) {
        if (cnx->is_multipath_enabled ||
            cnx->congestion_alg == NULL ||
            cnx->congestion_alg->congestion_algorithm_number == PICOQUIC_CC_ALGO_NUMBER_NEW_RENO ||
            cnx->congestion_alg->congestion_algorithm_number == PICOQUIC_CC_ALGO_NUMBER_FAST
            ) {
            /* TODO: better understand combination of ack delay and multipath! */
            ack_gap = 32;
        }
        else {
            ack_gap = 32 + ((nb_packets - 128) / 8);
            if (ack_gap > 64) {
                ack_gap = 64;
            }
        }
    }

    return ack_gap;
}
```

### Rust body
```rust
    fn compute_ack_gap(&self, data_rate: u64, nb_packets: u64) -> u64 {
        let mut nb_packets = nb_packets;
        let first_path = self.paths.first();

        if self.is_ack_frequency_negotiated
            && first_path
                .map(|p| !p.is_ssthresh_initialized)
                .unwrap_or(false)
        {
            nb_packets /= 2;
        }

        if let Some(path) = first_path
            && path.rtt_min < Duration::from_ticks(4 * ACK_DELAY_MIN.ticks())
        {
            let mult = if path.rtt_min > ACK_DELAY_MIN {
                (4 * ACK_DELAY_MIN.ticks()) / path.rtt_min.ticks().max(1)
            } else {
                4
            };
            nb_packets = nb_packets.saturating_mul(mult);
        }

        let mut ack_gap = nb_packets.div_ceil(4);
        let mut ack_gap_min: u64 = 2;

        if data_rate > BANDWIDTH_MEDIUM {
            ack_gap_min = if first_path
                .map(|p| p.rtt_min > TARGET_RENO_RTT)
                .unwrap_or(false)
            {
                10
            } else {
                4
            };
        }

        if ack_gap < ack_gap_min {
            ack_gap = ack_gap_min;
        } else if ack_gap > 32 {
            let cc_number = self
                .congestion_alg
                .map(|cc| cc.congestion_algorithm_number)
                .unwrap_or(CC_ALGO_NUMBER_NEW_RENO);
            if self.is_multipath_enabled
                || cc_number == CC_ALGO_NUMBER_NEW_RENO
                || cc_number == CC_ALGO_NUMBER_FAST
            {
                ack_gap = 32;
            } else {
                ack_gap = (32 + nb_packets.saturating_sub(128) / 8).min(64);
            }
        }

        ack_gap
    }
```

## Pair `picoquic/frames.c:picoquic_process_ack_of_ack_frame`
C: `picoquic/frames.c:3352-3367 picoquic_process_ack_of_ack_frame`
Rust: `rs/fq/src/internal.rs:8781-8804 process_ack_of_ack_frame`

### C body
```c
{
    int ret;
    uint64_t largest;
    uint64_t ack_delay;
    uint64_t num_block;

    ret = picoquic_parse_ack_header(bytes, bytes_max, &num_block, NULL, &largest, &ack_delay, consumed, 0);

    if (ret == 0) {
        ret = picoquic_process_ack_of_ack_body(sack_list, largest, num_block, bytes, bytes_max, consumed, is_ecn);
    }

    return ret;
}
```

### Rust body
```rust
        {
            return -1;
        }
```

## Pair `picoquic/frames.c:picoquic_format_ack_frame_in_context`
C: `picoquic/frames.c:4074-4215 picoquic_format_ack_frame_in_context`
Rust: `rs/fq/src/internal.rs:12627-12652 format_ack_frame_in_context`

### C body
```c
{
    uint64_t num_block = 0;
    uint64_t ack_delay = 0;
    uint64_t ack_range = 0;
    uint64_t ack_gap = 0;
    uint64_t lowest_acknowledged = 0;
    int is_ecn = ack_ctx->sending_ecn_ack;
    uint8_t* after_stamp = bytes;
    uint64_t ack_type_byte = (multipath_sequence == UINT64_MAX) ?
        (((is_ecn) ? picoquic_frame_type_ack_ecn : picoquic_frame_type_ack)) :
        (((is_ecn) ? picoquic_frame_type_path_ack_ecn : picoquic_frame_type_path_ack));

    /* Check that there something to acknowledge */
    int not_needed = picoquic_sack_list_is_empty(&ack_ctx->sack_list);
    if (!not_needed && !ack_ctx->act[is_opportunistic].ack_needed &&
        ack_ctx->sack_list.ack_tree.size == 1) {
        picoquic_sack_item_t* last_sack = picoquic_sack_last_item(&ack_ctx->sack_list);
        not_needed = (last_sack->nb_times_sent[is_opportunistic] >= PICOQUIC_MAX_ACK_RANGE_REPEAT);
    }
    if (!not_needed){
        uint8_t* num_block_byte = NULL;
        picoquic_sack_item_t* last_sack = picoquic_sack_last_item(&ack_ctx->sack_list);

        if (current_time > ack_ctx->time_stamp_largest_received) {
            ack_delay = current_time - ack_ctx->time_stamp_largest_received;
            ack_delay >>= cnx->local_parameters.ack_delay_exponent;
        }

        if (*need_time_stamp) {
            /* When sending multiple acks in a frame, send the time stamp only once */
            bytes = picoquic_format_time_stamp_frame(cnx, bytes, bytes_max, more_data, current_time);
            after_stamp = bytes;
            *need_time_stamp = 0;
        }

        if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_type_byte)) != NULL &&
            (multipath_sequence == UINT64_MAX ||
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, multipath_sequence)) != NULL) &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_sack_item_range_end(last_sack))) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_delay)) != NULL) {
            /* Reserve one byte for the number of blocks */
            num_block_byte = bytes++;
            /* Encode the size of the first ack range */
            ack_range = picoquic_sack_item_range_end(last_sack) - picoquic_sack_item_range_start(last_sack);
            bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_range);
        }
        if (bytes == NULL || num_block_byte == NULL) {
            bytes = after_stamp;
            *more_data = 1;
        }
        else {
            /* Implement adaptive tuning of lowest repeat range */
            int nb_sent_max_acked = 0;
            int nb_sent_max_skip = 0;
            picoquic_sack_item_t* next_sack = picoquic_sack_previous_item(last_sack);

            /* Update send count for the top range */
            picoquic_sack_item_record_sent(&ack_ctx->sack_list, last_sack, is_opportunistic);

            /* Find the parameters of range selection: max number of repeats, 
             * highest range splits required.
             */
            picoquic_sack_select_ack_ranges(&ack_ctx->sack_list, last_sack, 32, 
                is_opportunistic, &nb_sent_max_acked, &nb_sent_max_skip);

            /* Set the lowest acknowledged */
            lowest_acknowledged = picoquic_sack_item_range_start(last_sack);
            while (num_block < 32 && next_sack != NULL) {
                if (picoquic_sack_item_nb_times_sent(next_sack, is_opportunistic) <= nb_sent_max_acked) {
                    if (picoquic_sack_item_nb_times_sent(next_sack, is_opportunistic) == nb_sent_max_acked &&
                        nb_sent_max_skip > 0) {
                        nb_sent_max_skip--;
                    }
                    else {
                        uint8_t* bytes_start_range = bytes;
                        ack_gap = lowest_acknowledged - picoquic_sack_item_range_end(next_sack) - 2; /* per spec */
                        ack_range = picoquic_sack_item_range_end(next_sack) - picoquic_sack_item_range_start(next_sack);

                        if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_gap)) == NULL ||
                            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_range)) == NULL) {
                            bytes = bytes_start_range;
                            *more_data = 1;
                            break;
                        }
                        else {
                            picoquic_sack_item_record_sent(&ack_ctx->sack_list, next_sack, is_opportunistic);
                            lowest_acknowledged = picoquic_sack_item_range_start(next_sack);
                            num_block++;
                        }
                    }
                }
                next_sack = picoquic_sack_previous_item(next_sack);
            }
            /* When numbers are lower than 64, varint encoding fits on one byte */
            *num_block_byte = (uint8_t)num_block;

            /* Remember the ACK value and time */
            if (!is_opportunistic) {
                ack_ctx->act[0].highest_ack_sent = picoquic_sack_list_last(&ack_ctx->sack_list);
                ack_ctx->act[0].highest_ack_sent_time = current_time;
            }
            else {
                ack_ctx->act[1].highest_ack_sent = picoquic_sack_list_last(&ack_ctx->sack_list);
                ack_ctx->act[1].highest_ack_sent_time = current_time;
            }
        }

        if (bytes > after_stamp && is_ecn) {
            /* Try to encode the ECN bytes */
            uint8_t* bytes_ecn = bytes;
            if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_ctx->ecn_ect0_total_local)) == NULL ||
                (bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_ctx->ecn_ect1_total_local)) == NULL ||
                (bytes = picoquic_frames_varint_encode(bytes, bytes_max, ack_ctx->ecn_ce_total_local)) == NULL)
            {
                bytes = bytes_ecn;
                *more_data = 1;
                *after_stamp = picoquic_frame_type_ack;
            }
        }
    }

    if (bytes > after_stamp){
        if (is_opportunistic) {
            /* TODO: should non opportunistic sending also reset these flags? */
            ack_ctx->act[1].ack_needed = 0;
            ack_ctx->act[1].ack_after_fin = 0;
            ack_ctx->act[1].out_of_order_received = 0;
        }
        else {
            cnx->is_immediate_ack_required = 0;
            ack_ctx->act[0].ack_needed = 0;
            ack_ctx->act[0].ack_after_fin = 0;
            ack_ctx->act[0].out_of_order_received = 0;
            ack_ctx->act[0].is_immediate_ack_required = 0;
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let (tail, clear_immediate_ack) = format_ack_frame_in_context_impl(
        bytes,
        more_data,
        current_time,
        ack_ctx,
        need_time_stamp,
        multipath_sequence,
        is_opportunistic,
        connection.local_parameters.ack_delay_exponent,
        connection.start_time,
    );
    if clear_immediate_ack {
        connection.is_immediate_ack_required = false;
    }
    tail
}
```

## Pair `picoquic/frames.c:picoquic_is_ack_needed_in_ctx`
C: `picoquic/frames.c:4309-4353 picoquic_is_ack_needed_in_ctx`
Rust: `rs/fq/src/internal.rs:7934-7956 is_ack_needed_in_ctx`

### C body
```c
{
    int ret = 0;

    if (ack_ctx->act[is_opportunistic].ack_needed) {
        if (ack_ctx->act[is_opportunistic].is_immediate_ack_required) {
            ret = 1;
        }
        else if (pc != picoquic_packet_context_application || ack_ctx->act[is_opportunistic].ack_after_fin) {
            ret = 1;
            ack_ctx->act[is_opportunistic].ack_after_fin = 0;
        }
        else if (ack_ctx->act[is_opportunistic].out_of_order_received && !cnx->ack_ignore_order_remote) {
            ret = 1;
        }
        else
        {
            uint64_t ack_gap = picoquic_ack_gap_override_if_needed(cnx, path_index);

            if (ack_ctx->act[is_opportunistic].highest_ack_sent + ack_gap <= picoquic_sack_list_last(&ack_ctx->sack_list) ||
                ack_ctx->act[is_opportunistic].time_oldest_unack_packet_received + cnx->ack_delay_remote <= current_time) {
                ret = 1;
            }
            else {
                if (ack_ctx->act[is_opportunistic].time_oldest_unack_packet_received + cnx->ack_delay_remote < *next_wake_time) {
                    *next_wake_time = ack_ctx->act[is_opportunistic].time_oldest_unack_packet_received + cnx->ack_delay_remote;
                    SET_LAST_WAKE(cnx->quic, PICOQUIC_FRAME);
                }
            }
        }
    }
    else if (ack_ctx->act[is_opportunistic].highest_ack_sent + 8 <= picoquic_sack_list_last(&ack_ctx->sack_list) &&
        ack_ctx->act[is_opportunistic].highest_ack_sent_time + cnx->ack_delay_remote <= current_time) {
        /* Force sending an ack-of-ack from time to time, as a low priority action */
        if (picoquic_sack_list_last(&ack_ctx->sack_list) == UINT64_MAX) {
            ret = 0;
        }
        else {
            ret = 1;
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> bool {
    let limits = AckNeededLimits {
        ack_ignore_order_remote: connection.ack_ignore_order_remote,
        ack_delay_remote: connection.ack_delay_remote,
    };
    is_ack_needed_in_ctx_inner(
        ack_ctx,
        current_time,
        connection.ack_gap_override_if_needed(path_index),
        next_wake_time,
        pc,
        is_opportunistic,
        limits,
    )
}
```

## Pair `picoquic/frames.c:picoquic_format_max_data_frame`
C: `picoquic/frames.c:4485-4501 picoquic_format_max_data_frame`
Rust: `rs/fq/src/internal.rs:12838-12859 format_max_data_frame`

### C body
```c
{
    uint8_t * bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_max_data)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, cnx->maxdata_local + maxdata_increase)) != NULL) {
        cnx->maxdata_local = (cnx->maxdata_local + maxdata_increase);
        *is_pure_ack = 0;
    }
    else {
        *more_data = 1;
        bytes = bytes0;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let new_max = connection.maxdata_local.saturating_add(maxdata_increase);
    if bytes.is_empty() {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[0] = crate::frames::FrameType::MaxData as u8;
    let mut off = 1;
    if !encode_varint_at(bytes, &mut off, new_max) {
        *more_data = 1;
        return Some(bytes);
    }
    connection.maxdata_local = new_max;
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_update_max_stream_ID_local`
C: `picoquic/frames.c:4702-4722 picoquic_update_max_stream_ID_local`
Rust: `rs/fq/src/internal.rs:10019-10049 update_max_stream_id_local`

### C body
```c
{
    if (cnx->client_mode != IS_CLIENT_STREAM_ID(stream->stream_id) && !stream->max_stream_updated) {
        /* This is a remotely initiated stream */
        if (stream->consumed_offset >= stream->fin_offset && (stream->fin_received || stream->reset_received)) {
            /* Receive is complete */
            if (IS_BIDIR_STREAM_ID(stream->stream_id)) {
                if (stream->fin_sent || stream->reset_sent)
                {
                    /* Sending is complete */
                    stream->max_stream_updated = 1;
                    cnx->max_stream_id_bidir_local_computed += 4;
                }
            } else {
                /* No need to check receive complete on uni directional streams */
                stream->max_stream_updated = 1;
                cnx->max_stream_id_unidir_local_computed += 4;
            }
        }
    }
}
```

### Rust body
```rust
    pub fn update_max_stream_id_local(&mut self, stream: &mut StreamHead) {
        use crate::stream::{Role, StreamId};
        let sid = StreamId(stream.stream_id);
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };
        if sid.is_local(local_role) {
            return;
        }
        if sid.is_bidir() {
            if stream.stream_id >= self.max_stream_id_bidir_local {
                let old = self.max_stream_id_bidir_local;
                self.max_stream_id_bidir_local_computed = self
                    .max_stream_id_bidir_local_computed
                    .max(stream.stream_id.saturating_add(4));
                if self.max_stream_id_bidir_local_computed > old {
                    stream.max_stream_updated = true;
                }
            }
        } else if stream.stream_id >= self.max_stream_id_unidir_local {
            let old = self.max_stream_id_unidir_local;
            self.max_stream_id_unidir_local_computed = self
                .max_stream_id_unidir_local_computed
                .max(stream.stream_id.saturating_add(4));
            if self.max_stream_id_unidir_local_computed > old {
                stream.max_stream_updated = true;
            }
        }
    }
```
