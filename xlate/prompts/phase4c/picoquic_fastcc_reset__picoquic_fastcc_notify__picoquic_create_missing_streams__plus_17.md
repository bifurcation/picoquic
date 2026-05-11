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

## Pair `picoquic/fastcc.c:picoquic_fastcc_reset`
C: `picoquic/fastcc.c:74-83 picoquic_fastcc_reset`
Rust: `rs/fq/src/fastcc.rs:89-97 picoquic_fastcc_reset`

### C body
```c
{
    memset(fastcc_state, 0, sizeof(picoquic_fastcc_state_t));
    fastcc_state->alg_state = picoquic_fastcc_initial;
    fastcc_state->rtt_min = path_x->smoothed_rtt;
    fastcc_state->rolling_rtt_min = fastcc_state->rtt_min;
    fastcc_state->delay_threshold = picoquic_fastcc_delay_threshold(fastcc_state->rtt_min);
    fastcc_state->end_of_epoch = current_time + FASTCC_PERIOD;
    path_x->cwin = PICOQUIC_CWIN_INITIAL;
}
```

### Rust body
```rust
pub fn picoquic_fastcc_reset(state: &mut FastccState, path_x: &mut Path, current_time: Instant) {
    *state = FastccState::default();
    state.alg_state = FastccAlgState::Initial;
    state.rtt_min = path_x.smoothed_rtt.ticks();
    state.rolling_rtt_min = state.rtt_min;
    state.delay_threshold = fastcc_delay_threshold(state.rtt_min);
    state.end_of_epoch = current_time.ticks().saturating_add(FASTCC_PERIOD);
    path_x.cwin = CWIN_INITIAL;
}
```

## Pair `picoquic/fastcc.c:picoquic_fastcc_notify`
C: `picoquic/fastcc.c:162-306 picoquic_fastcc_notify`
Rust: `rs/fq/src/fastcc.rs:168-179 picoquic_fastcc_notify`

### C body
```c
{
    picoquic_fastcc_state_t* fastcc_state = (picoquic_fastcc_state_t*)path_x->congestion_alg_state;
    path_x->is_cc_data_updated = 1;

    if (fastcc_state != NULL) {
        if (fastcc_state->alg_state == picoquic_fastcc_freeze && 
            (current_time > fastcc_state->end_of_freeze ||
                fastcc_state->recovery_sequence <= picoquic_cc_get_ack_number(cnx, path_x))) {
            if (fastcc_state->last_freeze_was_timeout) {
                fastcc_state->alg_state = picoquic_fastcc_initial;
            }
            else {
                fastcc_state->alg_state = picoquic_fastcc_eval;
            }
            fastcc_state->last_freeze_was_not_delay = 0;
            fastcc_state->last_freeze_was_timeout = 0;

            fastcc_state->nb_cc_events = 0;
            fastcc_state->nb_bytes_ack_since_rtt = 0;
        }

        switch (notification) {
        case picoquic_congestion_notification_acknowledgement: 
            if (fastcc_state->alg_state != picoquic_fastcc_freeze) {
                /* Count the bytes since last RTT measurement */
                fastcc_state->nb_bytes_ack_since_rtt += ack_state->nb_bytes_acknowledged;
                /* Compute pacing data. */
                picoquic_update_pacing_data(path_x, 0);
            }
            break;

        case picoquic_congestion_notification_ecn_ec:
            fastcc_notify_congestion(cnx, path_x, fastcc_state, current_time, 0, 0);
            break;
        case picoquic_congestion_notification_repeat:
        case picoquic_congestion_notification_timeout:
            if (picoquic_cc_hystart_loss_test(&fastcc_state->rtt_filter, notification, ack_state->lost_packet_number, PICOQUIC_SMOOTHED_LOSS_THRESHOLD)) {
                fastcc_notify_congestion(cnx, path_x, fastcc_state, current_time, 0,
                    (notification == picoquic_congestion_notification_timeout) ? 1 : 0);
            }
            break;
        case picoquic_congestion_notification_spurious_repeat:
            if (fastcc_state->nb_cc_events > 0) {
                fastcc_state->nb_cc_events--;
            }
            break;
        case picoquic_congestion_notification_rtt_measurement:
        {
            uint64_t delta_rtt = 0;

            picoquic_cc_filter_rtt_min_max(&fastcc_state->rtt_filter, ack_state->rtt_measurement);

            if (fastcc_state->rtt_filter.is_init) {
                /* We use the maximum of the last samples as the candidate for the
                 * min RTT, in order to filter the rtt jitter */
                if (current_time > fastcc_state->end_of_epoch) {
                    /* If end of epoch, reset the min RTT to min of remembered periods,
                     * and roll the period. */
                    fastcc_state->rtt_min = UINT64_MAX;
                    for (int i = FASTCC_NB_PERIOD - 1; i > 0; i--) {
                        fastcc_state->last_rtt_min[i] = fastcc_state->last_rtt_min[i - 1];
                        if (fastcc_state->last_rtt_min[i] > 0 &&
                            fastcc_state->last_rtt_min[i] < fastcc_state->rtt_min) {
                            fastcc_state->rtt_min = fastcc_state->last_rtt_min[i];
                        }
                    }
                    fastcc_state->delay_threshold = picoquic_fastcc_delay_threshold(fastcc_state->rtt_min);
                    fastcc_state->last_rtt_min[0] = fastcc_state->rolling_rtt_min;
                    fastcc_state->rolling_rtt_min = fastcc_state->rtt_filter.sample_max;
                    fastcc_state->end_of_epoch = current_time + FASTCC_PERIOD;
                }
                else if (fastcc_state->rtt_filter.sample_max < fastcc_state->rolling_rtt_min || fastcc_state->rolling_rtt_min == 0) {
                    /* If not end of epoch, update the rolling minimum */
                    fastcc_state->rolling_rtt_min = fastcc_state->rtt_filter.sample_max;
                    if (fastcc_state->rolling_rtt_min < fastcc_state->rtt_min) {
                        fastcc_state->rtt_min = fastcc_state->rolling_rtt_min;
                    }
                }
            }

            if (fastcc_state->alg_state != picoquic_fastcc_freeze) {
                if (ack_state->rtt_measurement < fastcc_state->rtt_min) {
                    fastcc_state->delay_threshold = picoquic_fastcc_delay_threshold(fastcc_state->rtt_min);
                }
                else if (fastcc_state->rtt_min_is_trusted){
                    delta_rtt = ack_state->rtt_measurement - fastcc_state->rtt_min;
                }
                else {
                    fastcc_state->rtt_min = ack_state->rtt_measurement; 
                    fastcc_state->rolling_rtt_min = ack_state->rtt_measurement;
                    fastcc_state->rtt_min_is_trusted = 1;
                    delta_rtt = 0;
                }

                if (delta_rtt < fastcc_state->delay_threshold) {
                    double alpha = 1.0;
                    fastcc_state->nb_cc_events = 0;

                    if (fastcc_state->alg_state != picoquic_fastcc_initial) {
                        alpha -= ((double)delta_rtt / (double)fastcc_state->delay_threshold);
                        alpha *= FASTCC_EVAL_ALPHA;
                    }

                    /* Increase the window if it is not frozen */
                    if (path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                        path_x->cwin += (uint64_t)(alpha * (double)fastcc_state->nb_bytes_ack_since_rtt);
                    }
                    fastcc_state->nb_bytes_ack_since_rtt = 0;
                }
                else {
                    /* May well be congested */
                    fastcc_state->nb_cc_events++;
                    if (fastcc_state->nb_cc_events >= FASTCC_REPEAT_THRESHOLD) {
                        /* Too many events, reduce the window */
                        fastcc_notify_congestion(cnx, path_x, fastcc_state, current_time, 1, 0);
                    }
                }
            }
        }
        break;
        case picoquic_congestion_notification_cwin_blocked:
            break;
        case picoquic_congestion_notification_reset:
            picoquic_fastcc_reset(fastcc_state, path_x, current_time);
            break;
        case picoquic_congestion_notification_seed_cwin:
            picoquic_fastcc_seed_cwin(fastcc_state, path_x, ack_state->nb_bytes_acknowledged);
            break;
        default:
            /* ignore */
            break;
        }
    }
}
```

### Rust body
```rust
    let Some(boxed_state) = path_x.congestion_alg_state.take() else {
        return;
    };
```

## Pair `picoquic/frames.c:picoquic_create_missing_streams`
C: `picoquic/frames.c:57-96 picoquic_create_missing_streams`
Rust: `rs/fq/src/internal.rs:9489-9505 create_missing_streams`

### C body
```c
{
    /* Verify the stream ID control conditions */
    picoquic_stream_head_t* stream = NULL;
    unsigned int expect_client_stream = cnx->client_mode ^ is_remote;

    if (is_remote && stream_id < cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]) {
        return NULL;
    } else if (IS_CLIENT_STREAM_ID(stream_id) != expect_client_stream){
        /* TODO: not an error if lower than next stream, would be just an old stream. */
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_STREAM_LIMIT_ERROR, 0);
    }
    else if (is_remote && stream_id > (IS_BIDIR_STREAM_ID(stream_id) ? cnx->max_stream_id_bidir_local : cnx->max_stream_id_unidir_local)){
        /* Protocol error, stream ID too high */
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_STREAM_LIMIT_ERROR, 0);
    } 
    else if (stream_id < cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]) {
        /* Stream already closed locally; do not escalate to transport error here. */
        return NULL;
    } else {
        while (stream_id >= cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]) {
            stream = picoquic_create_stream(cnx, cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]);
            if (stream == NULL) {
                picoquic_log_app_message(cnx, "Create stream %" PRIu64 " returns error 0x%x",
                    stream_id, PICOQUIC_TRANSPORT_INTERNAL_ERROR);
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0);
                break;
            }
            else if (!IS_BIDIR_STREAM_ID(stream_id)) {
                if (!IS_LOCAL_STREAM_ID(stream_id, cnx->client_mode)) {
                    /* Mark the stream as already finished in our direction */
                    stream->fin_requested = 1;
                    stream->fin_sent = 1;
                }
            }
        }
    }

    return stream;
}
```

### Rust body
```rust
    ) -> Result<StreamToken, crate::Error> {
        use crate::stream::StreamId;
        let type_idx = (stream_id & 3) as usize;
        let mut first_new_id = self.next_stream_id[type_idx];
        let mut last_tok: Option<StreamToken> = None;
        // Walk forward creating streams until we reach stream_id.
        while first_new_id <= stream_id {
            let tok = self.create_stream(first_new_id)?;
            last_tok = Some(tok);
            first_new_id = StreamId(first_new_id).next_with_same_kind().0;
        }
        last_tok.ok_or(crate::Error::Memory)
    }
```

## Pair `picoquic/frames.c:picoquic_find_or_create_stream`
C: `picoquic/frames.c:188-197 picoquic_find_or_create_stream`
Rust: `rs/fq/src/internal.rs:16331-16340 new`

### C body
```c
{
    picoquic_stream_head_t* stream = picoquic_find_stream(cnx, stream_id);

    if (stream == NULL) {
        stream = picoquic_create_missing_streams(cnx, stream_id, is_remote);
    }

    return stream;
}
```

### Rust body
```rust
    pub fn len(&self) -> usize {
        self.inner.len()
    }
```

## Pair `picoquic/frames.c:picoquic_apply_reset_stream_frame`
C: `picoquic/frames.c:330-371 picoquic_apply_reset_stream_frame`
Rust: `rs/fq/src/internal.rs:10257-10331 picoquic_apply_reset_stream_frame`

### C body
```c
{
    picoquic_stream_head_t* stream;
    
    if (!IS_BIDIR_STREAM_ID(stream_id) && IS_LOCAL_STREAM_ID(stream_id, cnx->client_mode)) {
        /* the peer cannot send data, and thus cannot reset the stream */
        bytes = NULL;
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_STREAM_STATE_ERROR,
            picoquic_frame_type_reset_stream);
    }
    if ((stream = picoquic_find_or_create_stream(cnx, stream_id, 1)) == NULL) {
        /* Not finding the stream is only an error if the stream
         * was expected to be present, or created on demand. If the
         * stream was already created and then deleted, there is no harm.
         * If the "return NULL" is in a normal scenario, the connection state
         * will remain "ready" or "almost ready"
         */
        if (cnx->cnx_state > picoquic_state_ready) {
            bytes = NULL;  /* error already signaled */
        }
    }
    else if ((stream->fin_received || stream->reset_received) && final_offset != stream->fin_offset) {
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_FINAL_OFFSET_ERROR,
            picoquic_frame_type_reset_stream);
        bytes = NULL;

    }
    else if (picoquic_flow_control_check_stream_offset(cnx, stream, final_offset) != 0) {
        bytes = NULL;  // error already signaled
    }
    else if (!stream->reset_received) {
        stream->reset_received = 1;
        stream->reset_offset = reliable_size;
        stream->remote_error = error_code_64;

        if (stream->consumed_offset >= stream->reset_offset) {
            picoquic_signal_stream_reset(cnx, stream);
        }
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    use crate::stream::{Role, StreamId};

    let local_role = if connection.client_mode {
        Role::Client
    } else {
        Role::Server
    };
    let sid = StreamId(stream_id);
    if !sid.is_bidir() && sid.is_local(local_role) {
        connection.connection_error(
            crate::errors::TransportError::StreamStateError as u64,
            crate::frames::FrameType::ResetStream as u64,
        );
        return None;
    }

    let stream_token = match connection.find_stream(stream_id) {
        Some(token) => token,
        None => match connection.create_missing_streams(stream_id, true) {
            Ok(token) => token,
            Err(_) => {
                if connection.connection_state > State::Ready {
                    return None;
                }
                return Some(bytes);
            }
        },
    };

    let final_offset_mismatch = connection
        .streams
        .get(stream_token)
        .map(|stream| {
            (stream.fin_received || stream.reset_received) && final_offset != stream.fin_offset
        })
        .unwrap_or(false);
    if final_offset_mismatch {
        connection.connection_error(
            crate::errors::TransportError::FinalOffsetError as u64,
            crate::frames::FrameType::ResetStream as u64,
        );
        return None;
    }

    if flow_control_check_stream_offset_token(connection, stream_token, final_offset) != 0 {
        return None;
    }

    let should_signal = if let Some(stream) = connection.streams.get_mut(stream_token) {
        if !stream.reset_received {
            stream.reset_received = true;
            stream.reset_offset = reliable_size;
            stream.remote_error = error_code_64;
            stream.consumed_offset >= stream.reset_offset
        } else {
            false
        }
    } else {
        false
    };

    if should_signal {
        signal_stream_reset_token(connection, stream_token);
    }

    Some(bytes)
}
```

## Pair `picoquic/frames.c:picoquic_queue_retire_connection_id_frame`
C: `picoquic/frames.c:842-858 picoquic_queue_retire_connection_id_frame`
Rust: `rs/fq/src/internal.rs:13510-13539 queue_retire_connection_id_frame`

### C body
```c
{
    int ret = 0;
    size_t consumed = 0;
    uint8_t frame_buffer[258];
    int is_pure_ack = 1;
    int more_data = 0;
    uint8_t * bytes_next = picoquic_format_retire_connection_id_frame(frame_buffer, frame_buffer + sizeof(frame_buffer),
        &more_data, &is_pure_ack, cnx->is_multipath_enabled, unique_path_id, sequence);
    
    if ((consumed = bytes_next - frame_buffer) > 0) {
        ret = picoquic_queue_misc_frame(cnx, frame_buffer, consumed, is_pure_ack,
            picoquic_packet_context_application);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let mut frame = [0u8; 258];
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let tail_len = format_retire_connection_id_frame(
            &mut frame,
            &mut more_data,
            &mut is_pure_ack,
            self.is_multipath_enabled,
            unique_path_id,
            sequence,
        )
        .map(|tail| tail.len())
        .ok_or(crate::Error::BufferTooSmall)?;
        if more_data != 0 {
            return Err(crate::Error::BufferTooSmall);
        }
        let frame_len = frame.len() - tail_len;
        encode_misc_frame(
            self,
            frame[..frame_len].to_vec(),
            is_pure_ack != 0,
            PacketContext::Application,
        );
        Ok(())
    }
```

## Pair `picoquic/frames.c:picoquic_is_stream_frame_unlimited`
C: `picoquic/frames.c:1199-1202 picoquic_is_stream_frame_unlimited`
Rust: `rs/fq/src/internal.rs:12243-12250 is_stream_frame_unlimited`

### C body
```c
{
    return PICOQUIC_BITS_CLEAR_IN_RANGE(bytes[0], picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max, 0x02);
}
```

### Rust body
```rust
        .map(|b| {
            (*b as u64) >= crate::frames::FrameType::StreamRangeMin as u64
                && (*b as u64) <= crate::frames::FrameType::StreamRangeMax as u64
                && (*b & 0x02) == 0
        })
```

## Pair `picoquic/frames.c:picoquic_find_ready_stream_path`
C: `picoquic/frames.c:1562-1664 picoquic_find_ready_stream_path`
Rust: `rs/fq/src/internal.rs:9673-9697 find_ready_stream_path`

### C body
```c
{
    picoquic_stream_head_t* first_stream = cnx->first_output_stream;
    picoquic_stream_head_t* stream = first_stream;
    picoquic_stream_head_t* found_stream = NULL;


    /* Look for a ready stream */
    while (stream != NULL) {
        int has_data = 0;
        picoquic_stream_head_t* next_stream = stream->next_output_stream;

        if (next_stream != NULL && is_coalesced && next_stream->is_not_coalesced) {
            stream = next_stream->next_output_stream;
            continue;
        }

        if (found_stream != NULL && stream->stream_priority > found_stream->stream_priority) {
            /* All the streams at that priority level have been examined,
             * the current selection is validated */
            break;
        }

        /* The tests for "have data" should excatly replicate the tests in
         * the formating of a stream frame */
        if (stream->stop_sending_requested && !stream->stop_sending_sent) {
            /* will send a stop sending frame.
            * this takes precedence over FIFO vs round-robin processing */
            found_stream = stream;
            has_data = 1;
            break;
        }
        else if (stream->reset_sent) {
            /* No data will be sent after a reset */
            has_data = 0;
        }
        else if (stream->reset_requested && 
            (stream->reliable_size == 0 || picoquic_check_sack_list(&stream->sack_list, 0, stream->reliable_size))) {
            /* will queue a reset frame --
            * this takes precedence over FIFO vs round-robin processing */
            found_stream = stream;
            has_data = 1;
            break;
        }
        else if (cnx->maxdata_remote > cnx->data_sent && stream->sent_offset < stream->maxdata_remote && (stream->is_active ||
            (stream->send_queue != NULL && stream->send_queue->length > stream->send_queue->offset) ||
            (stream->fin_requested && !stream->fin_sent))) {
            has_data = 1;
        }
        else {
            has_data = 0;
        }

        /* implement affinity scheduling */
        if (has_data && path_x != NULL && stream->affinity_path != path_x && stream->affinity_path != NULL) {
            /* Only consider the streams that meet path affinity requirements */
            has_data = 0;
        }
        
        if (has_data) {
            /* Check that this stream is actually available for sending data */
            if (stream->sent_offset == 0) {
                if (IS_CLIENT_STREAM_ID(stream->stream_id) == cnx->client_mode) {
                    if (stream->stream_id > ((IS_BIDIR_STREAM_ID(stream->stream_id)) ? cnx->max_stream_id_bidir_remote : cnx->max_stream_id_unidir_remote)) {
                        has_data = 0;
                    }
                }
            }
            if (has_data) {
                /* Something can be sent */
                if ((stream->stream_priority & 1) != 0) {
                    /* This priority level requests FIFO processing, so we return the first available stream */
                    found_stream = stream;
                    break;
                }
                else if (found_stream == NULL || stream->last_time_data_sent < found_stream->last_time_data_sent) {
                    /* Select this stream, but need to check if another stream should go before in round robin order */
                    found_stream = stream;
                }
            }
        }
        else if (((stream->fin_requested && stream->fin_sent) || (stream->reset_requested && stream->reset_sent)) && (!stream->stop_sending_requested || stream->stop_sending_sent)) {
            /* If stream is exhausted, remove from output list */
            picoquic_remove_output_stream(cnx, stream);

            picoquic_delete_stream_if_closed(cnx, stream);
        }
        else {
            if (stream->is_active ||
                (stream->send_queue != NULL && stream->send_queue->length > stream->send_queue->offset)) {
                if (stream->sent_offset >= stream->maxdata_remote) {
                    cnx->stream_blocked = 1;
                }
                else if (cnx->maxdata_remote <= cnx->data_sent) {
                    cnx->flow_blocked = 1;
                }
            }
        }
        stream = next_stream;
    }

    return found_stream;
}
```

### Rust body
```rust
        self.output_streams.iter().copied().find(|tok| {
            self.streams.get(*tok).is_some_and(|stream| {
                let path_ok = stream
                    .affinity_path
                    .map(|affinity| {
                        affinity
                            == PathToken::synthetic(
                                path_x.unique_path_id as u32,
                                path_x.unique_path_id as u32,
                            )
                    })
                    .unwrap_or(true);
                let coalescing_ok = is_coalesced || !stream.is_not_coalesced;
                path_ok
                    && coalescing_ok
                    && (!stream.send_queue.is_empty()
                        || (stream.fin_requested && !stream.fin_sent)
                        || stream.is_active)
            })
        })
```

## Pair `picoquic/frames.c:picoquic_format_stream_blocked_frame`
C: `picoquic/frames.c:1712-1748 picoquic_format_stream_blocked_frame`
Rust: `rs/fq/src/internal.rs:13656-13702 format_stream_blocked_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    uint8_t f_type = 0;
    uint64_t stream_limit = 0;
    int should_not_send = 0;

    if (IS_BIDIR_STREAM_ID(stream->stream_id)) {
        f_type = picoquic_frame_type_streams_blocked_bidir;
        stream_limit = STREAM_RANK_FROM_ID(cnx->max_stream_id_bidir_remote);
        should_not_send = cnx->stream_blocked_bidir_sent;
    }
    else {
        f_type = picoquic_frame_type_streams_blocked_unidir;
        stream_limit = STREAM_RANK_FROM_ID(cnx->max_stream_id_unidir_remote);
        should_not_send = cnx->stream_blocked_unidir_sent;
    }
    if (!should_not_send) {
        if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, f_type)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream_limit)) != NULL) {
            *is_pure_ack = 0;
            if (IS_BIDIR_STREAM_ID(stream->stream_id)) {
                cnx->stream_blocked_bidir_sent = 1;
            }
            else {
                cnx->stream_blocked_unidir_sent = 1;
            }
        }
        else {
            *more_data = 1;
            bytes = bytes0;
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let stream_id = crate::stream::StreamId(stream.stream_id);
    let (frame_type, stream_limit, should_not_send) = if stream_id.is_bidir() {
        (
            crate::frames::FrameType::StreamsBlockedBidir as u8,
            crate::stream::StreamId(connection.max_stream_id_bidir_remote).rank(),
            connection.stream_blocked_bidir_sent,
        )
    } else {
        (
            crate::frames::FrameType::StreamsBlockedUnidir as u8,
            crate::stream::StreamId(connection.max_stream_id_unidir_remote).rank(),
            connection.stream_blocked_unidir_sent,
        )
    };

    if should_not_send {
        return Some(bytes);
    }

    let mut off = 0;
    if bytes.is_empty() {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off] = frame_type;
    off += 1;

    if !encode_varint_at(bytes, &mut off, stream_limit) {
        *more_data = 1;
        Some(bytes)
    } else {
        *is_pure_ack = 0;
        if stream_id.is_bidir() {
            connection.stream_blocked_bidir_sent = true;
        } else {
            connection.stream_blocked_unidir_sent = true;
        }
        Some(&mut bytes[off..])
    }
}
```

## Pair `picoquic/frames.c:picoquic_provide_stream_data_buffer`
C: `picoquic/frames.c:1840-1864 picoquic_provide_stream_data_buffer`
Rust: `rs/fq/src/lib.rs:4092-4142 provide_stream_data_buffer`

### C body
```c
{
    picoquic_stream_data_buffer_argument_t * data_ctx = (picoquic_stream_data_buffer_argument_t*)context;
    uint8_t* buffer = NULL;
    size_t start_index = 0;

    if (length <= data_ctx->allowed_space) {
        data_ctx->length = length;

        if (is_fin) {
            data_ctx->is_fin = 1;
            data_ctx->bytes[0] |= 1;
        }

        data_ctx->is_still_active = is_still_active;

        data_ctx->byte_index = picoquic_encode_length_of_stream_frame(data_ctx->bytes,
            data_ctx->byte_index, data_ctx->byte_space, length, &start_index);

        buffer = data_ctx->bytes + data_ctx->byte_index;
        data_ctx->app_buffer = buffer;
    }

    return buffer;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    if nb_bytes > context.allowed_space {
        return None;
    }

    context.length = nb_bytes;
    if is_fin {
        context.is_fin = 1;
        if let Some(first) = context.bytes.first_mut() {
            *first |= 1;
        }
    } else {
        context.is_fin = 0;
    }
    context.is_still_active = is_still_active as i32;

    if nb_bytes < context.byte_space {
        if nb_bytes == context.byte_space.saturating_sub(1) {
            if context.byte_index >= context.bytes.len() {
                return None;
            }
            context.bytes.copy_within(0..context.byte_index, 1);
            context.bytes[0] = crate::frames::FrameType::Padding as u8;
            context.byte_index += 1;
        } else {
            let encoded = crate::internal::varint_encode(
                &mut context.bytes[context.byte_index..context.byte_space],
                nb_bytes as u64,
            );
            if encoded == 0 {
                return None;
            }
            context.byte_index += encoded;
            if let Some(first) = context.bytes.first_mut() {
                *first |= 2;
            }
        }
    }

    let end = context.byte_index + nb_bytes;
    if end > context.bytes.len() {
        return None;
    }
    let slice = &mut context.bytes[context.byte_index..end];
    Some(slice)
}
```

## Pair `picoquic/frames.c:picoquic_queue_data_repeat_node_create`
C: `picoquic/frames.c:2129-2136 picoquic_queue_data_repeat_node_create`
Rust: `rs/fq/src/internal.rs:5698-5716 queue_data_repeat_node_create`

### C body
```c
{
    return &((picoquic_packet_t*)value)->queue_data_repeat_node;
}
```

### Rust body
```rust
    pub fn queue_data_repeat_node_value(&self, splay_tok: SplayToken) -> Option<PacketToken> {
        self.queue_data_repeat_tree.get(splay_tok).copied()
    }
```

## Pair `picoquic/frames.c:picoquic_dequeue_data_repeat_packet`
C: `picoquic/frames.c:2197-2201 picoquic_dequeue_data_repeat_packet`
Rust: `rs/fq/src/internal.rs:10463-10466 dequeue_data_repeat_packet`

### C body
```c
{
    picosplay_delete_hint(&cnx->queue_data_repeat_tree, &packet->queue_data_repeat_node);
}
```

### Rust body
```rust
        if let Some(st) = packet.queue_data_repeat_membership.take() {
            self.queue_data_repeat_tree.remove(st);
        } else if let Some(st) = self
```

## Pair `picoquic/frames.c:picoquic_copy_stream_frame_for_retransmit`
C: `picoquic/frames.c:2277-2397 picoquic_copy_stream_frame_for_retransmit`
Rust: `rs/fq/src/internal.rs:10541-10658 copy_stream_frame_for_retransmit`

### C body
```c
{
    uint8_t* frame = packet->bytes + packet->data_repeat_frame;
    size_t frame_length_max = packet->length - packet->data_repeat_frame;
    uint64_t stream_id;
    uint64_t offset;
    size_t data_length;
    size_t consumed;
    size_t bytes_not_sent = 0;
    int fin;

    if (picoquic_parse_stream_header(frame, frame_length_max, &stream_id, &offset, &data_length, &fin, &consumed) != 0) {
        /* Malformed stream frame. Error. */
        bytes_next = NULL;
    }
    else {
        uint8_t* bytes_first = bytes_next;
        /* Need to find out how much is really available, based on the index in the packet */
        size_t data_available = data_length;
        uint8_t* frame_bytes = frame + consumed;
        int is_needed = 1;
        if (packet->data_repeat_index > packet->data_repeat_frame + consumed) {
            size_t already_sent = packet->data_repeat_index - packet->data_repeat_frame - consumed;
            if (already_sent <= data_length) {
                offset += already_sent;
                frame_bytes += already_sent;
                data_available -= already_sent;
            }
            else {
                /* This is really an internal error! */
                offset += data_length;
                frame_bytes += data_length;
                data_available = 0;
            }
        }
        /* Check that these bytes are needed.
         * The code only deletes a stream context if all the stream bytes have been acknowledged,
         * including the FIN flag which is counted as a final octet after the max offset.
         * If the stream is deleted or reset, there is no need to send again any stream data frame for that stream.
         * If all the octets in the frame are acknowledged, including the FIN bit if present, there is
         * also no need to send the frame again.
         */
        if (cnx != NULL) {
            picoquic_stream_head_t* stream = picoquic_find_stream(cnx, stream_id);
            if (stream == NULL || stream->reset_sent || 
                picoquic_check_sack_list(&stream->sack_list, offset, offset + data_available - ((fin) ? 0 : 1))) {
                /* That frame is not needed anymore */
                is_needed = 0;
            }
        }
        if (is_needed) {
            /* Need to check how much can be encoded in the packet:
             * Header (with or without FIN), stream_id, offset, length.
             */
            if ((bytes_next = picoquic_format_stream_frame_header(bytes_next, bytes_max, stream_id, offset)) == NULL ||
                bytes_next == bytes_max) {
                /* Cannot encode anything! -- need to wait for another opportunity */
                bytes_not_sent = data_available;
                bytes_next = bytes_first;
            }
            else {
                uint8_t* before_length = bytes_next;
                if ((bytes_next = picoquic_frames_varint_encode(bytes_next, bytes_max, data_available)) != NULL &&
                    bytes_next + data_available <= bytes_max) {
                    /* Can encode everything in a natural way */
                    *bytes_first |= 2; /* length is present */
                    *bytes_first |= fin; /* fin OK */
                    memcpy(bytes_next, frame_bytes, data_available);
                    bytes_next += data_available;
                }
                else if (before_length + data_available <= bytes_max) {
                    /* everything fits if we remove the length, but we may need to insert initial padding */
                    size_t space_available = bytes_max - before_length;
                    size_t pad_required = space_available - data_available;
                    bytes_next = before_length;
                    *bytes_first |= fin; /* fin OK */
                    if (pad_required > 0) {
                        memmove(bytes_first + pad_required, bytes_first, before_length - bytes_first);
                        for (size_t i = 0; i < pad_required; i++) {
                            bytes_first[i] = 0;
                        }
                        bytes_next += pad_required;
                    }
                    memcpy(bytes_next, frame_bytes, data_available);
                    bytes_next += data_available;
                }
                else {
                    /* buffer is too short -- do not send the FIN bit, do not set the length, just copy bytes */
                    size_t available = bytes_max - before_length;
                    if (available < PICOQUIC_MIN_STREAM_DATA_FRAGMENT) {
                        bytes_not_sent = data_available;
                        bytes_next = bytes_first;
                    }
                    else {
                        bytes_next = before_length;
                        memcpy(bytes_next, frame_bytes, available);
                        bytes_next += available;
                        bytes_not_sent = data_available - available;
                    }
                }
            }
        }

        if (bytes_not_sent == 0) {
            /* Progress frame index to next byte after data frame */
            packet->data_repeat_index = packet->data_repeat_frame + consumed + data_length;
            packet->data_repeat_frame = packet->data_repeat_index;
        }
        else if (bytes_not_sent < data_length) {
            /* Progress index to next byte not sent */
            packet->data_repeat_index = packet->data_repeat_frame + consumed + data_length - bytes_not_sent;
        }
    }

    return bytes_next;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let frame_offset = packet.data_repeat_frame;
    let frame_length_max = packet.length.checked_sub(frame_offset)?;
    let frame = &packet.bytes[frame_offset..packet.length];
    let mut stream_id = 0;
    let mut offset = 0;
    let mut data_length = 0;
    let mut consumed = 0;
    let mut fin = 0;

    if parse_stream_header(
        frame,
        frame_length_max,
        &mut stream_id,
        &mut offset,
        &mut data_length,
        &mut fin,
        &mut consumed,
    ) != 0
    {
        return None;
    }

    let mut data_available = data_length;
    let mut frame_bytes_index = frame_offset + consumed;
    if packet.data_repeat_index > frame_offset + consumed {
        let already_sent = packet.data_repeat_index - frame_offset - consumed;
        if already_sent <= data_length {
            offset += already_sent as u64;
            frame_bytes_index += already_sent;
            data_available -= already_sent;
        } else {
            offset += data_length as u64;
            frame_bytes_index += data_length;
            data_available = 0;
        }
    }

    let mut is_needed = true;
    if let Some(stream_token) = connection.find_stream(stream_id) {
        if let Some(stream) = connection.streams.get_mut(stream_token) {
            let ack_end = offset
                .saturating_add(data_available as u64)
                .saturating_sub(if fin != 0 { 0 } else { 1 });
            if stream.reset_sent || stream.sack_list.check(offset, ack_end) {
                is_needed = false;
            }
        }
    } else {
        is_needed = false;
    }

    let mut bytes_not_sent = 0usize;
    let mut written = 0usize;
    if is_needed {
        let mut header = [0u8; 32];
        let before_len = header.len();
        let header_tail = format_stream_frame_header(&mut header, stream_id, offset)?;
        let header_len = before_len - header_tail.len();
        if header_len >= bytes.len() {
            bytes_not_sent = data_available;
        } else {
            bytes[..header_len].copy_from_slice(&header[..header_len]);
            let length_len = encode_varint_length(data_available as u64);
            if header_len + length_len + data_available <= bytes.len() {
                bytes[0] |= 0x02;
                bytes[0] |= fin as u8;
                let encoded = varint_encode(&mut bytes[header_len..], data_available as u64);
                if encoded == 0 {
                    return None;
                }
                let data_start = header_len + encoded;
                bytes[data_start..data_start + data_available].copy_from_slice(
                    &packet.bytes[frame_bytes_index..frame_bytes_index + data_available],
                );
                written = data_start + data_available;
            } else if header_len + data_available <= bytes.len() {
                let space_available = bytes.len() - header_len;
                let pad_required = space_available - data_available;
                bytes[0] |= fin as u8;
                if pad_required > 0 {
                    bytes.copy_within(0..header_len, pad_required);
                    bytes[..pad_required].fill(0);
                }
                let data_start = header_len + pad_required;
                bytes[data_start..data_start + data_available].copy_from_slice(
                    &packet.bytes[frame_bytes_index..frame_bytes_index + data_available],
                );
                written = data_start + data_available;
            } else {
                let available = bytes.len() - header_len;
                if available < MIN_STREAM_DATA_FRAGMENT {
                    bytes_not_sent = data_available;
                } else {
                    bytes[header_len..header_len + available].copy_from_slice(
                        &packet.bytes[frame_bytes_index..frame_bytes_index + available],
                    );
                    written = header_len + available;
                    bytes_not_sent = data_available - available;
                }
            }
        }
    }

    if bytes_not_sent == 0 {
        packet.data_repeat_index = packet.data_repeat_frame + consumed + data_length;
        packet.data_repeat_frame = packet.data_repeat_index;
    } else if bytes_not_sent < data_length {
        packet.data_repeat_index =
            packet.data_repeat_frame + consumed + data_length - bytes_not_sent;
    }

    Some(&mut bytes[written..])
}
```

## Pair `picoquic/frames.c:picoquic_decode_crypto_hs_frame`
C: `picoquic/frames.c:2541-2574 picoquic_decode_crypto_hs_frame`
Rust: `rs/fq/src/internal.rs:12395-12417 decode_crypto_hs_frame`

### C body
```c
{
    uint64_t offset;
    uint64_t data_length;
    const uint8_t* data_bytes;

    if ((bytes = picoquic_parse_crypto_hs_frame(bytes, bytes_max, &offset, &data_length, &data_bytes)) == NULL) {
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR, picoquic_frame_type_crypto_hs);
    } else {
        picoquic_stream_head_t* stream = &cnx->tls_stream[epoch];

        if (stream->consumed_offset < offset &&
            stream->consumed_offset + PICOQUIC_MAX_CRYPTO_BUFFER_GAP < offset + data_length) {
            picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_CRYPTO_BUFFER_EXCEEDED, picoquic_frame_type_crypto_hs);
            bytes = NULL;
        }
        else {
            int new_data_available;
            int ret = picoquic_queue_network_input(cnx->quic, &stream->stream_data_tree, stream->consumed_offset,
                offset, data_bytes, (size_t)data_length, picoquic_is_last_stream_frame(bytes + data_length, bytes_max),
                received_data, &new_data_available);

            if (ret != 0) {
                picoquic_connection_error(cnx, (int64_t)ret, picoquic_frame_type_crypto_hs);
                bytes = NULL;
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    let (&frame_type, mut tail) = bytes.split_first()?;
    if frame_type != crate::frames::FrameType::CryptoHs as u8 {
        return None;
    }
    let mut offset = 0;
    tail = frames_varint_decode(tail, &mut offset)?;
    let mut length = 0;
    tail = frames_varint_decode(tail, &mut length)?;
    if tail.len() < length as usize {
        return None;
    }
    let data = &tail[..length as usize];
    if let Some(stream) = connection.tls_stream.get_mut(epoch.clamp(0, 3) as usize) {
        queue_received_stream_data(stream, offset, data, received_data).ok()?;
    }
    Some(&tail[length as usize..])
}
```

## Pair `picoquic/frames.c:picoquic_estimate_max_path_bandwidth`
C: `picoquic/frames.c:2924-2961 picoquic_estimate_max_path_bandwidth`
Rust: `rs/fq/src/internal.rs:6130-6136 estimate_max_path_bandwidth`

### C body
```c
{
    /* Test whether there is enough time since the last max bandwidth estimate */
    if (send_time >= path_x->max_sample_sent_time) {
        if (path_x->max_sample_sent_time == 0) {
            /* No sample set yet, need to initialize the variables */
            path_x->max_sample_delivered = path_x->delivered;
            path_x->max_sample_acked_time = delivery_time;
            path_x->max_sample_sent_time = send_time;
        }
        else {
            /* Compute a max bandwidth estimate */
            uint64_t receive_interval = delivery_time - path_x->max_sample_acked_time;

            if (receive_interval > PICOQUIC_MAX_BANDWIDTH_TIME_INTERVAL_MIN) {
                uint64_t delivered = path_x->delivered - path_x->max_sample_delivered;
                uint64_t send_interval = send_time - path_x->max_sample_sent_time;
                uint64_t bw_estimate;

                if (send_interval > receive_interval) {
                    receive_interval = send_interval;
                }

                bw_estimate = PICOQUIC_RATE_FROM_BYTES(delivered, receive_interval);
                /* Retain if larger than previous estimate */
                if (bw_estimate > path_x->peak_bandwidth_estimate) {
                    path_x->peak_bandwidth_estimate = bw_estimate;
                }

                /* Change the reference point if estimate duration is long enough */
                path_x->max_sample_delivered = path_x->delivered;
                path_x->max_sample_acked_time = delivery_time;
                path_x->max_sample_sent_time = send_time;
            }
        }
    }
}
```

### Rust body
```rust
            if self.max_sample_sent_time.ticks() == 0 {
                self.max_sample_delivered = self.delivered;
                self.max_sample_acked_time = delivery_time;
                self.max_sample_sent_time = send_time;
            } else {
```

## Pair `picoquic/frames.c:picoquic_compute_ack_gap_and_delay`
C: `picoquic/frames.c:3066-3130 picoquic_compute_ack_gap_and_delay`
Rust: `rs/fq/src/internal.rs:8852-8925 compute_ack_gap_and_delay`

### C body
```c
{
    uint64_t nb_packets = picoquic_compute_packets_in_window(cnx, data_rate);

    *ack_delay_max = picoquic_compute_ack_delay_max(cnx, rtt, remote_min_ack_delay);
    *ack_gap = picoquic_compute_ack_gap(cnx, data_rate, nb_packets);

    if (2 * cnx->path[0]->smoothed_rtt > 3 * cnx->path[0]->rtt_min) {
        uint64_t return_data_rate = 0;

        /* This code kicks in when the smoothed RTT is larger than 1.5 times the RTT Min.
         * If that is the case, the default computation of ACK gap and ACK delay may
         * be wrong, and a more conservative computation is required.
         * This code assume that ACK gap and ACK delay are already computed using
         * the default algorithms.
         */
        if (cnx->is_ack_frequency_negotiated) {
            return_data_rate = cnx->path[0]->receive_rate_max;
        }
        else {
            return_data_rate = cnx->path[0]->bandwidth_estimate;
        }

        if (nb_packets < 2) {
            nb_packets = 2;
        }
        if (return_data_rate > 0) {
            /* Estimate of ACK size = L2 + IPv6 + UDP + padded ACK */
            const uint64_t ack_size = 12 + 40 + 8 + 55;
            /* Estimate of ACK transmission time *in microseconds */
            uint64_t ack_transmission_time = (ack_size * 1000000) / return_data_rate;
            /* if ACK transmission time > ack delay, perform correction */
            if (ack_transmission_time > * ack_delay_max) {
                *ack_delay_max = ack_transmission_time;
                if (*ack_delay_max > PICOQUIC_ACK_DELAY_MAX) {
                    *ack_delay_max = PICOQUIC_ACK_DELAY_MAX;
                }
            }
            /* if ack gap smaller than ack time fraction of CWIN, perform correction */
            uint64_t rtt_target = (cnx->path[0]->smoothed_rtt + cnx->path[0]->rtt_min) / 2;

            if (!cnx->path[0]->is_ssthresh_initialized) {
                nb_packets /= 2;
            }

            uint64_t nb_ack_per_rtt = (*ack_gap > 0) ? (nb_packets + *ack_gap - 1) / (*ack_gap):nb_packets;
            if (nb_ack_per_rtt * (*ack_delay_max) > rtt_target) {
                uint64_t nb_acks_max = cnx->path[0]->smoothed_rtt / (*ack_delay_max);
                if (nb_acks_max <= 1) {
                    *ack_gap = nb_packets;
                }
                else {
                    uint64_t ack_gap_min = (nb_packets + nb_acks_max - 1) / nb_acks_max;
                    if (*ack_gap < ack_gap_min) {
                        *ack_gap = ack_gap_min;
                    }
                }
            }
        }
    }
    if (cnx->path[0]->rtt_min < *ack_delay_max * 4 && *ack_gap > 32) {
        *ack_gap = 32;
    }
}
```

### Rust body
```rust
    ) {
        let first_path = self.paths.first();
        let rtt_ticks = rtt.ticks();
        let bytes_in_window = data_rate
            .saturating_mul(rtt_ticks)
            .saturating_div(1_000_000);
        let mut nb_packets = (bytes_in_window / MAX_PACKET_SIZE as u64).max(2);

        *ack_delay_max = (rtt_ticks / 4).min(ACK_DELAY_MAX.ticks());
        if !self.is_ack_frequency_negotiated
            && first_path
                .map(|p| !p.is_ssthresh_initialized)
                .unwrap_or(true)
        {
            *ack_delay_max /= 2;
        }
        *ack_delay_max = (*ack_delay_max).max(remote_min_ack_delay);

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

        let mut gap = nb_packets.div_ceil(4);
        let mut gap_min = 2;
        if data_rate > BANDWIDTH_MEDIUM {
            gap_min = if first_path
                .map(|p| p.rtt_min > TARGET_RENO_RTT)
                .unwrap_or(false)
            {
                10
            } else {
                4
            };
        }
        if gap < gap_min {
            gap = gap_min;
        } else if gap > 32 {
            let cc_number = self
                .congestion_alg
                .map(|cc| cc.congestion_algorithm_number)
                .unwrap_or(CC_ALGO_NUMBER_NEW_RENO);
            if self.is_multipath_enabled
                || cc_number == CC_ALGO_NUMBER_NEW_RENO
                || cc_number == CC_ALGO_NUMBER_FAST
            {
                gap = 32;
            } else {
                gap = (32 + nb_packets.saturating_sub(128) / 8).min(64);
            }
        }
        *ack_gap = gap;
    }
```

## Pair `picoquic/frames.c:picoquic_process_ack_of_stream_frame`
C: `picoquic/frames.c:3676-3704 picoquic_process_ack_of_stream_frame`
Rust: `rs/fq/src/tests/skip_frame.rs:644-688 stream_ack_test_one`

### C body
```c
{
    int ret;
    int fin;
    size_t data_length;
    uint64_t stream_id;
    uint64_t offset;
    picoquic_stream_head_t* stream = NULL;

    /* skip stream frame */
    ret = picoquic_parse_stream_header(bytes, bytes_max,
        &stream_id, &offset, &data_length, &fin, consumed);

    if (ret == 0) {
        *consumed += data_length;

        /* record the ack range for the stream */
        stream = picoquic_find_stream(cnx, stream_id);
        if (stream != NULL) {
            (void)picoquic_update_sack_list(&stream->sack_list,
                offset, offset + data_length - ((fin) ? 0 : 1), 0);

            picoquic_delete_stream_if_closed(cnx, stream);
        }
    }

    return ret;
}
```

### Rust body
```rust
fn stream_ack_test_one(quic: &mut Quic) -> crate::Result<()> {
    let packets: &[(&[u8], bool)] = &[
        (&[0x0b, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0x0b, 4, 0], true),
        (&[0x0a, 12, 8, 0, 1, 2, 3, 4, 5, 6, 7], true),
        (&[0x0f, 16, 32, 8, 1, 2, 3, 4, 5, 6, 7, 8], true),
        (&[0x09, 20, 4, 0, 0, 0, 0, 1, 2, 3, 4, 5], false),
    ];
    let mut simulated_time = Instant::from_ticks(0);
    let mut cnx = quic.create_test_cnx(&mut simulated_time)?;
    for stream_id in [0, 4, 8, 12, 16, 20] {
        let _ = cnx.create_stream(stream_id)?;
    }
    for (packet, should_ack) in packets {
        let mut byte_index = 0usize;
        while byte_index < packet.len() {
            let mut consumed = 0usize;
            let mut pure_ack = 0i32;
            if skip_frame(
                &packet[byte_index..],
                packet.len() - byte_index,
                &mut consumed,
                &mut pure_ack,
            ) != 0
            {
                return Err(crate::Error::InvalidFrame);
            }
            let mut no_need_to_repeat = 0;
            let mut do_not_detect_spurious = 0;
            let mut is_preemptive_needed = 0;
            let ret = cnx.check_frame_needs_repeat(
                &packet[byte_index..byte_index + consumed],
                consumed,
                crate::internal::PacketType::OneRttProtected,
                &mut no_need_to_repeat,
                &mut do_not_detect_spurious,
                &mut is_preemptive_needed,
            );
            if ret != 0 || (*should_ack && pure_ack != 0) {
                return Err(crate::Error::Generic);
            }
            byte_index += consumed;
        }
    }
    Ok(())
}
```

## Pair `picoquic/frames.c:picoquic_set_ack_needed`
C: `picoquic/frames.c:4267-4288 picoquic_set_ack_needed`
Rust: `rs/fq/src/internal.rs:11913-11942 set_ack_needed`

### C body
```c
{
    if (pc == picoquic_packet_context_application &&
        cnx->is_multipath_enabled) {
        /* TODO: this code seems wrong */
        path_x->ack_ctx.act[0].is_immediate_ack_required |= is_immediate_ack_required;
        if (!path_x->ack_ctx.act[0].ack_needed) {
            path_x->ack_ctx.act[0].ack_needed = 1;
            path_x->ack_ctx.act[0].time_oldest_unack_packet_received = current_time;
            path_x->ack_ctx.act[1].ack_needed = 1;
            path_x->ack_ctx.act[1].time_oldest_unack_packet_received = current_time;
        }
    }
    if (!cnx->ack_ctx[pc].act[0].ack_needed) {
        cnx->ack_ctx[pc].act[0].is_immediate_ack_required |= is_immediate_ack_required;
        cnx->ack_ctx[pc].act[0].ack_needed = 1;
        cnx->ack_ctx[pc].act[0].time_oldest_unack_packet_received = current_time;
        cnx->ack_ctx[pc].act[1].ack_needed = 1;
        cnx->ack_ctx[pc].act[1].time_oldest_unack_packet_received = current_time;
    }
}
```

### Rust body
```rust
    ) {
        if pc == PacketContext::Application && self.is_multipath_enabled {
            if is_immediate_ack_required != 0 {
                path_x.ack_ctx.act[0].is_immediate_ack_required = true;
            }
            if !path_x.ack_ctx.act[0].ack_needed {
                path_x.ack_ctx.act[0].ack_needed = true;
                path_x.ack_ctx.act[0].time_oldest_unack_packet_received = current_time;
                path_x.ack_ctx.act[1].ack_needed = true;
                path_x.ack_ctx.act[1].time_oldest_unack_packet_received = current_time;
            }
        }

        let ack_ctx = &mut self.ack_ctx[pc as usize];
        if !ack_ctx.act[0].ack_needed {
            if is_immediate_ack_required != 0 {
                ack_ctx.act[0].is_immediate_ack_required = true;
            }
            ack_ctx.act[0].ack_needed = true;
            ack_ctx.act[0].time_oldest_unack_packet_received = current_time;
            ack_ctx.act[1].ack_needed = true;
            ack_ctx.act[1].time_oldest_unack_packet_received = current_time;
        }
    }
```

## Pair `picoquic/frames.c:picoquic_format_connection_close_frame`
C: `picoquic/frames.c:4377-4393 picoquic_format_connection_close_frame`
Rust: `rs/fq/src/internal.rs:12742-12767 format_connection_close_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_connection_close)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, cnx->local_error)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, cnx->offending_frame_type)) != NULL &&
        (bytes = picoquic_frames_charz_encode(bytes, bytes_max, cnx->local_error_reason)) != NULL) {
        *is_pure_ack = 0;
    }
    else {
        bytes = bytes0;
        *more_data = 1;
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let reason = connection.local_error_reason.as_deref().unwrap_or("");
    let reason_bytes = reason.as_bytes();
    let mut off = 0;
    if !encode_varint_at(
        bytes,
        &mut off,
        crate::frames::FrameType::ConnectionClose as u64,
    ) || !encode_varint_at(bytes, &mut off, connection.local_error)
        || !encode_varint_at(bytes, &mut off, connection.offending_frame_type)
        || !encode_varint_at(bytes, &mut off, reason_bytes.len() as u64)
        || bytes.len() < off + reason_bytes.len()
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + reason_bytes.len()].copy_from_slice(reason_bytes);
    off += reason_bytes.len();
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_format_required_max_stream_data_frames`
C: `picoquic/frames.c:4631-4658 picoquic_format_required_max_stream_data_frames`
Rust: `rs/fq/src/internal.rs:12795-12836 format_required_max_stream_data_frames`

### C body
```c
{
    uint8_t* bytes0;
    picoquic_stream_head_t* stream = picoquic_first_stream(cnx);

    while (stream != NULL) {
        if (!stream->fin_received && !stream->use_app_flow_control) {
            uint64_t new_window = picoquic_cc_increased_window(cnx, stream->maxdata_local);

            if (!stream->reset_received && 2 * stream->consumed_offset > stream->maxdata_local) {
                bytes0 = bytes;

                if ((bytes = picoquic_format_max_stream_data_frame(cnx, stream, bytes, bytes_max, more_data, is_pure_ack, stream->maxdata_local + new_window)) == bytes0) {
                    /* not enough space for this frame. */
                    break;
                }
            }
        }
        stream = picoquic_next_stream(stream);
    }

    if (stream == NULL) {
        cnx->max_stream_data_needed = 0;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let tokens: Vec<_> = connection
        .streams
        .iter()
        .filter_map(|stream| {
            stream
                .max_stream_updated
                .then_some(stream.stream_tree_membership)
                .flatten()
                .and_then(|st| connection.stream_tree.get(st).copied())
        })
        .collect();
    for tok in tokens {
        let Some(stream) = connection.streams.get_mut(tok) else {
            continue;
        };
        if !stream.max_stream_updated {
            continue;
        }
        let mut off = 0;
        if !encode_varint_at(
            bytes,
            &mut off,
            crate::frames::FrameType::MaxStreamData as u64,
        ) || !encode_varint_at(bytes, &mut off, stream.stream_id)
            || !encode_varint_at(bytes, &mut off, stream.maxdata_local)
        {
            *more_data = 1;
            return Some(bytes);
        }
        stream.maxdata_local_acked = stream.maxdata_local;
        stream.max_stream_updated = false;
        *is_pure_ack = 0;
        bytes = &mut bytes[off..];
    }
    Some(bytes)
}
```
