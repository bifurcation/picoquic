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

## Pair `picoquic/fastcc.c:fastcc_notify_congestion`
C: `picoquic/fastcc.c:119-160 fastcc_notify_congestion`
Rust: `rs/fq/src/fastcc.rs:129-142 fastcc_notify_congestion`

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

## Pair `picoquic/frame_names.c:picoquic_frame_name`
C: `picoquic/frame_names.c:25-116 picoquic_frame_name`
Rust: `rs/fq/src/frames.rs:63-108 name`

### C body
```c
{
    if ((int)ftype >= picoquic_frame_type_stream_range_min &&
        (int)ftype <= picoquic_frame_type_stream_range_max) {
        return "stream";
    }

    switch (ftype) {
    case picoquic_frame_type_padding:
        return "padding";
    case picoquic_frame_type_reset_stream:
        return "reset_stream";
    case picoquic_frame_type_reset_stream_at:
        return "reset_stream_at";
    case picoquic_frame_type_connection_close:
    case picoquic_frame_type_application_close:
        return "connection_close";
    case picoquic_frame_type_max_data:
        return "max_data";
    case picoquic_frame_type_max_stream_data:
        return "max_stream_data";
    case picoquic_frame_type_max_streams_bidir:
    case picoquic_frame_type_max_streams_unidir:
        return "max_streams";
    case picoquic_frame_type_ping:
        return "ping";
    case picoquic_frame_type_data_blocked:
        return "data_blocked";
    case picoquic_frame_type_stream_data_blocked:
        return "stream_data_blocked";
    case picoquic_frame_type_streams_blocked_bidir:
    case picoquic_frame_type_streams_blocked_unidir:
        return "streams_blocked";
    case picoquic_frame_type_new_connection_id:
        return "new_connection_id";
    case picoquic_frame_type_path_new_connection_id:
        return "path_new_connection_id";
    case picoquic_frame_type_stop_sending:
        return "stop_sending";
    case picoquic_frame_type_ack:
        return "ack";
    case picoquic_frame_type_path_challenge:
        return "path_challenge";
    case picoquic_frame_type_path_response:
        return "path_response";
    case picoquic_frame_type_crypto_hs:
        return "crypto";
    case picoquic_frame_type_new_token:
        return "new_token";
    case picoquic_frame_type_ack_ecn:
        return "ack";
    case picoquic_frame_type_path_ack:
        return "path_ack";
    case picoquic_frame_type_path_ack_ecn:
        return "path_ack";
    case picoquic_frame_type_retire_connection_id:
        return "retire_connection_id";
    case picoquic_frame_type_path_retire_connection_id:
        return "path_retire_connection_id";
    case picoquic_frame_type_handshake_done:
        return "handshake_done";
    case picoquic_frame_type_datagram:
    case picoquic_frame_type_datagram_l:
        return "datagram";
    case picoquic_frame_type_ack_frequency:
        return "ack_frequency";
    case picoquic_frame_type_immediate_ack:
        return "immediate_ack";
    case picoquic_frame_type_time_stamp:
        return "time_stamp";
    case picoquic_frame_type_path_abandon:
        return "path_abandon";
    case picoquic_frame_type_path_backup:
        return "path_backup";
    case picoquic_frame_type_path_available:
        return "path_available";
    case picoquic_frame_type_bdp:
        return "bdp";
    case picoquic_frame_type_max_path_id:
        return "max_path_id";
    case picoquic_frame_type_paths_blocked:
        return "paths_blocked";
    case picoquic_frame_type_path_cid_blocked:
        return "path_cid_blocked";
    case picoquic_frame_type_observed_address_v4:
        return "observed_address_v4";
    case picoquic_frame_type_observed_address_v6:
        return "observed_address_v6";
    default:
        return "unknown";
    }
}
```

### Rust body
```rust
    pub fn name(frame_type: u64) -> Option<&'static str> {
        if frame_type >= FrameType::StreamRangeMin as u64
            && frame_type <= FrameType::StreamRangeMax as u64
        {
            return Some("stream");
        }
        match frame_type {
            0x00 => Some("padding"),
            0x01 => Some("ping"),
            0x02 | 0x03 => Some("ack"),
            0x04 => Some("reset_stream"),
            0x05 => Some("stop_sending"),
            0x06 => Some("crypto"),
            0x07 => Some("new_token"),
            0x10 => Some("max_data"),
            0x11 => Some("max_stream_data"),
            0x12 | 0x13 => Some("max_streams"),
            0x14 => Some("data_blocked"),
            0x15 => Some("stream_data_blocked"),
            0x16 | 0x17 => Some("streams_blocked"),
            0x18 => Some("new_connection_id"),
            0x19 => Some("retire_connection_id"),
            0x1a => Some("path_challenge"),
            0x1b => Some("path_response"),
            0x1c | 0x1d => Some("connection_close"),
            0x1e => Some("handshake_done"),
            0x1f => Some("immediate_ack"),
            0x24 => Some("reset_stream_at"),
            0x30 | 0x31 => Some("datagram"),
            0x3e | 0x3f => Some("path_ack"),
            0xaf => Some("ack_frequency"),
            757 => Some("time_stamp"),
            0x3e75 => Some("path_abandon"),
            0x3e76 => Some("path_backup"),
            0x3e77 => Some("path_available"),
            0x3e78 => Some("path_new_connection_id"),
            0x3e79 => Some("path_retire_connection_id"),
            0x3e7a => Some("max_path_id"),
            0x3e7b => Some("paths_blocked"),
            0x3e7c => Some("path_cid_blocked"),
            0xebd9 => Some("bdp"),
            0x9f81a6 => Some("observed_address_v4"),
            0x9f81a7 => Some("observed_address_v6"),
            _ => None,
        }
    }
```

## Pair `picoquic/frames.c:picoquic_update_stream_initial_remote`
C: `picoquic/frames.c:162-186 picoquic_update_stream_initial_remote`
Rust: `rs/fq/src/internal.rs:9537-9561 update_stream_initial_remote`

### C body
```c
{
    picoquic_stream_head_t* stream = picoquic_first_stream(cnx);

    while (stream) {
        if (IS_LOCAL_STREAM_ID(stream->stream_id, cnx->client_mode)) {
            if (IS_BIDIR_STREAM_ID(stream->stream_id)) {
                if (stream->maxdata_remote < cnx->remote_parameters.initial_max_stream_data_bidi_remote) {
                    stream->maxdata_remote = cnx->remote_parameters.initial_max_stream_data_bidi_remote;
                }
            }
            else {
                if (stream->maxdata_remote < cnx->remote_parameters.initial_max_stream_data_uni) {
                    stream->maxdata_remote = cnx->remote_parameters.initial_max_stream_data_uni;
                }
            }
        }
        else if (IS_BIDIR_STREAM_ID(stream->stream_id)) {
            if (stream->maxdata_remote < cnx->remote_parameters.initial_max_stream_data_bidi_local) {
                stream->maxdata_remote = cnx->remote_parameters.initial_max_stream_data_bidi_local;
            }
        }
        stream = picoquic_next_stream(stream);
    };
}
```

### Rust body
```rust
    pub fn update_stream_initial_remote(&mut self) {
        use crate::stream::{Role, StreamId};
        let local_role = if self.client_mode {
            Role::Client
        } else {
            Role::Server
        };
        let bidi_remote = self.remote_parameters.initial_max_stream_data_bidi_remote;
        let bidi_local = self.remote_parameters.initial_max_stream_data_bidi_local;
        let uni = self.remote_parameters.initial_max_stream_data_uni;
        for s in self.streams.iter_mut() {
            let sid = StreamId(s.stream_id);
            if sid.is_local(local_role) {
                if sid.is_bidir() {
                    if s.maxdata_remote < bidi_remote {
                        s.maxdata_remote = bidi_remote;
                    }
                } else if s.maxdata_remote < uni {
                    s.maxdata_remote = uni;
                }
            } else if sid.is_bidir() && s.maxdata_remote < bidi_local {
                s.maxdata_remote = bidi_local;
            }
        }
    }
```

## Pair `picoquic/frames.c:picoquic_signal_stream_reset`
C: `picoquic/frames.c:305-328 picoquic_signal_stream_reset`
Rust: `rs/fq/src/internal.rs:10055-10088 signal_stream_reset`

### C body
```c
{
    picoquic_update_max_stream_ID_local(cnx, stream);

    if (cnx->callback_fn != NULL && !stream->reset_signalled) {
        if (!stream->is_discarded) {
            /* Update the flow control data to reflect that buffers are now discarded */
            if (stream->consumed_offset < stream->fin_offset) {
                uint64_t delta = stream->fin_offset - stream->consumed_offset;
                if (cnx->offset_received > delta) {
                    cnx->offset_received -= delta;
                }
            }
            /* Signal the reset to the application. */
            if (cnx->callback_fn(cnx, stream->stream_id, NULL, 0, picoquic_callback_stream_reset,
                cnx->callback_ctx, stream->app_stream_ctx) != 0) {
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR,
                    picoquic_frame_type_reset_stream);
            }
        }
        stream->reset_signalled = 1;
        (void)picoquic_delete_stream_if_closed(cnx, stream);
    }
}
```

### Rust body
```rust
    pub fn signal_stream_reset(&mut self, stream: &mut StreamHead) {
        self.update_max_stream_id_local(stream);

        if self.callback_fn.is_some() && !stream.reset_signalled {
            if !stream.is_discarded {
                if stream.consumed_offset < stream.fin_offset {
                    let delta = stream.fin_offset - stream.consumed_offset;
                    if self.offset_received > delta {
                        self.offset_received -= delta;
                    }
                }

                if let Some(mut callback) = self.callback_fn.take() {
                    let ret = callback.callback(
                        self,
                        stream.stream_id,
                        &[],
                        CallbackEvent::StreamReset,
                        stream.app_stream_ctx.as_deref_mut(),
                    );
                    self.callback_fn = Some(callback);

                    if ret != 0 {
                        self.connection_error(
                            crate::errors::TransportError::InternalError as u64,
                            crate::frames::FrameType::ResetStream as u64,
                        );
                    }
                }
            }
            stream.reset_signalled = true;
            let _ = self.delete_stream_if_closed(stream);
        }
    }
```

## Pair `picoquic/frames.c:picoquic_format_retire_connection_id_frame`
C: `picoquic/frames.c:818-835 picoquic_format_retire_connection_id_frame`
Rust: `rs/fq/src/internal.rs:13408-13433 format_retire_connection_id_frame`

### C body
```c
{
    uint8_t * bytes0 = bytes;

    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max,
        (is_mp)?picoquic_frame_type_path_retire_connection_id:picoquic_frame_type_retire_connection_id)) == NULL ||
        (is_mp && (bytes = picoquic_frames_varint_encode(bytes, bytes_max, unique_path_id)) == NULL) ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, sequence)) == NULL){
        bytes = bytes0;
        *more_data = 1;
    }
    else {
        *is_pure_ack = 0;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    let frame_type = if is_mp {
        crate::frames::FrameType::PathRetireConnectionId as u64
    } else {
        crate::frames::FrameType::RetireConnectionId as u64
    };

    if !encode_varint_at(bytes, &mut off, frame_type)
        || (is_mp && !encode_varint_at(bytes, &mut off, unique_path_id))
        || !encode_varint_at(bytes, &mut off, sequence)
    {
        *more_data = 1;
        Some(bytes)
    } else {
        *is_pure_ack = 0;
        Some(&mut bytes[off..])
    }
}
```

## Pair `picoquic/frames.c:picoquic_format_stop_sending_frame`
C: `picoquic/frames.c:1087-1112 picoquic_format_stop_sending_frame`
Rust: `rs/fq/src/internal.rs:13593-13626 format_stop_sending_frame`

### C body
```c
{
    if (!stream->stop_sending_requested || stream->stop_sending_sent || stream->fin_received || stream->reset_received) {
        /* set this, so we will not be called again */
        stream->stop_sending_sent = 1;
    }
    else
    {
        uint8_t* bytes0 = bytes;

        if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_stop_sending)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, (uint64_t)stream->stream_id)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->local_stop_error)) != NULL
            ) {
            *is_pure_ack = 0;
            stream->stop_sending_sent = 1;
        }
        else {
            bytes = bytes0;
            *more_data = 1;
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    if !stream.stop_sending_requested
        || stream.stop_sending_sent
        || stream.fin_received
        || stream.reset_received
    {
        stream.stop_sending_sent = true;
        return Some(bytes);
    }

    let mut off = 0;
    if bytes.is_empty() {
        *more_data = 1;
        return Some(bytes);
    }

    bytes[off] = crate::frames::FrameType::StopSending as u8;
    off += 1;
    if !encode_varint_at(bytes, &mut off, stream.stream_id)
        || !encode_varint_at(bytes, &mut off, stream.local_stop_error)
    {
        *more_data = 1;
        Some(bytes)
    } else {
        *is_pure_ack = 0;
        stream.stop_sending_sent = true;
        Some(&mut bytes[off..])
    }
}
```

## Pair `picoquic/frames.c:picoquic_decode_stream_frame`
C: `picoquic/frames.c:1527-1560 picoquic_decode_stream_frame`
Rust: `rs/fq/src/internal.rs:9952-9996 decode_stream_frame`

### C body
```c
{
    uint64_t stream_id;
    size_t   data_length;
    uint64_t offset;
    int      fin;
    size_t   consumed;
    if (picoquic_parse_stream_header(bytes, bytes_max - bytes, &stream_id, &offset, &data_length, &fin, &consumed) != 0) {
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR, picoquic_frame_type_stream_range_min);
        bytes = NULL;
    }else if (offset + data_length >= (1ull<<62)){
        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR, picoquic_frame_type_stream_range_min);
        bytes = NULL;
    }
    else {
        /* Skip the header bytes, and try to deliver the content of the frame.
        * The "is last" indication is set when we are certain that no other data
        * follows. It is used to manage the queue of stream chunks awaiting delivery.
         */
        bytes += consumed;
        if (picoquic_stream_network_input(cnx, stream_id, offset,
            fin, bytes, data_length, received_data,
            picoquic_is_last_stream_frame(bytes + data_length, bytes_max),
            current_time) != 0) {
            bytes = NULL;
        }
        else {
            bytes += data_length;
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    let mut stream_id = 0;
    let mut offset = 0;
    let mut data_length = 0;
    let mut fin = 0;
    let mut consumed = 0;
    if parse_stream_header(
        bytes,
        bytes.len(),
        &mut stream_id,
        &mut offset,
        &mut data_length,
        &mut fin,
        &mut consumed,
    ) != 0
    {
        return None;
    }
    let data_end = consumed.checked_add(data_length)?;
    if data_end > bytes.len() {
        return None;
    }
    let data = &bytes[consumed..data_end];
    let tok = connection
        .find_stream(stream_id)
        .map(Ok)
        .unwrap_or_else(|| connection.create_missing_streams(stream_id, true))
        .ok()?;
    if let Some(stream) = connection.streams.get_mut(tok) {
        if data_length > 0 {
            queue_received_stream_data(stream, offset, data, received_data).ok()?;
        }
        if fin != 0 {
            stream.fin_received = true;
            stream.fin_offset = offset.saturating_add(data_length as u64);
        }
        stream.last_time_data_sent = current_time;
    }
    Some(&bytes[data_end..])
}
```

## Pair `picoquic/frames.c:picoquic_format_stream_data_blocked_frame`
C: `picoquic/frames.c:1692-1710 picoquic_format_stream_data_blocked_frame`
Rust: `rs/fq/src/internal.rs:13629-13653 format_stream_data_blocked_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes=picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_stream_data_blocked)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->stream_id)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->maxdata_remote)) != NULL)
    {
        *is_pure_ack = 0;
        stream->stream_data_blocked_sent = 1;
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
    if bytes.is_empty() {
        *more_data = 1;
        return Some(bytes);
    }

    bytes[off] = crate::frames::FrameType::StreamDataBlocked as u8;
    off += 1;
    if !encode_varint_at(bytes, &mut off, stream.stream_id)
        || !encode_varint_at(bytes, &mut off, stream.maxdata_remote)
    {
        *more_data = 1;
        Some(bytes)
    } else {
        *is_pure_ack = 0;
        stream.stream_data_blocked_sent = true;
        Some(&mut bytes[off..])
    }
}
```

## Pair `picoquic/frames.c:picoquic_encode_length_of_stream_frame`
C: `picoquic/frames.c:1810-1838 picoquic_encode_length_of_stream_frame`
Rust: `rs/fq/src/internal.rs:12081-12092 encode_length_of_stream_frame`

### C body
```c
{
    if (length < byte_space) {
        if (length == byte_space - 1) {
            /* Special case: there are N bytes available, the application wants to write N-1 bytes.
             * We can encode N bytes because then we don't need a length field, just a flag in the
             * first byte. But if we had to encode "length=N-1", that would typically require 2
             * bytes, for a total of (2 + N-1)=N+1 bytes, larger than the packet size. We also
             * don't want to avoid the length field, because the encoding would be shorter than
             * the packet size, and other parts of the code might add a byte after that, e.g. padding,
             * which the receiver would mistake as data because of the "implicit length" encoding.
             * So we work against that issue by inserting a single padding byte in front of the
             * stream header.*/
            memmove(bytes + 1, bytes, byte_index);
            bytes[0] = picoquic_frame_type_padding;
            *start_index = 1;
            byte_index++;
        }
        else {
            /* Short frame, length field is required */
            /* We checked above that there are enough bytes to encode length */
            byte_index += picoquic_varint_encode(bytes + byte_index, byte_space, (uint64_t)length);
            bytes[0] |= 2; /* Indicates presence of length */
        }
    }

    return byte_index;
}
```

### Rust body
```rust
            if byte_index >= bytes.len() {
                return None;
            }
```

## Pair `picoquic/frames.c:picoquic_format_available_stream_frames`
C: `picoquic/frames.c:2079-2120 picoquic_format_available_stream_frames`
Rust: `rs/fq/src/internal.rs:10388-10424 format_available_stream_frames`

### C body
```c
{
    uint8_t* bytes_previous = bytes_next;
    picoquic_stream_head_t* stream = picoquic_find_ready_stream_path(cnx,
        (cnx->is_multipath_enabled)?path_x: NULL, 0);
    int more_stream_data = 0;

    while (*ret == 0 && stream != NULL && stream->stream_priority <= current_priority && bytes_next < bytes_max) {
        int is_still_active = 0;
        bytes_next = picoquic_format_stream_frame(cnx, stream, bytes_next, bytes_max, &more_stream_data, is_pure_ack, &is_still_active, ret);

        /* TODO: if stream is marked "no_coal", do not add anything */
        if (*ret == 0 && !stream->is_not_coalesced) {
            stream = picoquic_find_ready_stream_path(cnx,
                (cnx->is_multipath_enabled)?path_x: NULL, 1);
            if (stream != NULL && bytes_next + 17 >= bytes_max) {
                more_stream_data = 1;
                break;
            }
        }
        else {
            break;
        }
    }

    *stream_tried_and_failed = (!more_stream_data && bytes_next == bytes_previous);

    if (!more_stream_data && current_priority != UINT64_MAX) {
        more_stream_data |= (picoquic_find_ready_stream_path(cnx, NULL, 0) != NULL);
    }

    *more_data |= more_stream_data;

    return bytes_next;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let tok = connection.output_streams.pop_front()?;
    let stream = connection.streams.get_mut(tok)?;
    let before = bytes.len();
    let mut still_active = 0;
    let tail = encode_stream_like_frame(
        stream,
        bytes,
        more_data,
        is_pure_ack,
        &mut still_active,
        ret,
        false,
    )?;
    if tail.len() == before {
        *stream_tried_and_failed = 1;
    }
    if !stream.send_queue.is_empty()
        || still_active != 0
        || (stream.fin_requested && !stream.fin_sent)
    {
        connection.output_streams.push_back(tok);
        stream.is_output_stream = true;
    } else {
        stream.is_output_stream = false;
    }
    Some(tail)
}
```

## Pair `picoquic/frames.c:picoquic_queue_data_repeat_init`
C: `picoquic/frames.c:2188-2191 picoquic_queue_data_repeat_init`
Rust: `rs/fq/src/internal.rs:10427-10430 queue_data_repeat_init`

### C body
```c
void picoquic_queue_data_repeat_init(picoquic_cnx_t* cnx) {
    picosplay_init_tree(&cnx->queue_data_repeat_tree, picoquic_queue_data_repeat_compare,
        picoquic_queue_data_repeat_node_create, picoquic_queue_data_repeat_delete, picoquic_queue_data_repeat_node_value);
} 
```

### Rust body
```rust
    pub fn queue_data_repeat_init(&mut self) {
        // Reset the data-repeat splay tree (already empty on new connection).
        self.queue_data_repeat_tree.clear();
    }
```

## Pair `picoquic/frames.c:picoquic_first_data_repeat_packet`
C: `picoquic/frames.c:2270-2275 picoquic_first_data_repeat_packet`
Rust: `rs/fq/src/internal.rs:10475-10479 first_data_repeat_packet`

### C body
```c
{
    picosplay_node_t * first_node = picosplay_first(&cnx->queue_data_repeat_tree);
    picoquic_packet_t* first_packet = (first_node == NULL) ? NULL : picoquic_queue_data_repeat_node_value(first_node);
    return first_packet;
}
```

### Rust body
```rust
    pub fn first_data_repeat_packet(&self) -> Option<PacketToken> {
        // Return the token stored at the minimum key in the repeat tree.
        let st = self.queue_data_repeat_tree.first()?;
        self.queue_data_repeat_tree.get(st).copied()
    }
```

## Pair `picoquic/frames.c:picoquic_is_tls_stream_ready`
C: `picoquic/frames.c:2508-2524 picoquic_is_tls_stream_ready`
Rust: `rs/fq/src/internal.rs:9718-9763 is_tls_stream_ready`

### C body
```c
{
    int ret = 0;

    for (int epoch = 0; epoch < 4; epoch++) {
        picoquic_stream_head_t* stream = &cnx->tls_stream[epoch];

        if (stream->send_queue != NULL &&
            stream->send_queue->length > stream->send_queue->offset &&
            cnx->crypto_context[epoch].aead_encrypt != NULL) {
            ret = 1;
            break;
        }
    }

    return ret;
}
```

### Rust body
```rust
impl StreamHead {
    /// True when `stream` has finished sending and receiving all data.
    /// C: `picoquic_is_stream_closed`.
    pub fn is_stream_closed(&self, client_mode: bool) -> bool {
        use crate::stream::{Role, StreamId};
        let sid = StreamId(self.stream_id);
        let local_role = if client_mode {
            Role::Client
        } else {
            Role::Server
        };
        if sid.is_bidir() {
            ((self.fin_requested && self.fin_sent) || (self.reset_requested && self.reset_sent))
                && ((self.fin_received && self.fin_signalled)
                    || (self.reset_received && self.reset_signalled))
        } else if sid.is_local(local_role) {
            // Unidir from local host.
            (self.fin_requested && self.fin_sent) || (self.reset_requested && self.reset_sent)
        } else {
            // Unidir from remote.
            (self.fin_received && self.fin_signalled)
                || (self.reset_received && self.reset_signalled)
        }
    }

    /// Mark the stream as reset and drop all queued send data beyond
    /// `reliable_size` bytes.  Data already in the queue with
    /// `offset < reliable_size` is kept so it can be sent before the
    /// RESET_STREAM frame.
    ///
    /// C: `picoquic_enforce_reset_stream_frame` (`frames.c:252-282`)
    pub fn enforce_reset_stream_frame(&mut self, reliable_size: u64) {
        self.reset_sent = true;
        // send_queue is ordered by offset; find the first node that lies
        // at or beyond reliable_size and truncate everything from there.
        let keep = self
            .send_queue
            .partition_point(|node| node.offset < reliable_size);
        self.send_queue.truncate(keep);
    }
}
```

## Pair `picoquic/frames.c:picoquic_estimate_path_bandwidth`
C: `picoquic/frames.c:2865-2922 picoquic_estimate_path_bandwidth`
Rust: `rs/fq/src/internal.rs:1104-1171 picoquic_estimate_path_bandwidth`

### C body
```c
{
    if (send_time >= path_x->delivered_sent_last) {
        if (path_x->delivered_time_last == 0) {
            /* No estimate yet, need to initialize the variables */
            path_x->delivered_last = path_x->delivered;
            path_x->delivered_time_last = delivery_time;
            path_x->delivered_sent_last = send_time;
        }
        else {
            uint64_t receive_interval = delivery_time - delivered_time_prior;

            if (receive_interval > PICOQUIC_BANDWIDTH_TIME_INTERVAL_MIN) {
                uint64_t delivered = path_x->delivered - delivered_prior;
                uint64_t send_interval = send_time - delivered_sent_prior;
                uint64_t bw_estimate;

                if (send_interval > receive_interval) {
                    receive_interval = send_interval;
                }

                bw_estimate = PICOQUIC_RATE_FROM_BYTES(delivered, receive_interval);

                path_x->bandwidth_estimate = bw_estimate;
                if (!rs_is_path_limited || bw_estimate > path_x->bandwidth_estimate) {
                    if (path_x == cnx->path[0]){
                        if (cnx->is_ack_frequency_negotiated) {
                            /* Compute the desired value of the ack frequency*/
                            uint64_t ack_gap;
                            uint64_t ack_delay_max;
                            picoquic_compute_ack_gap_and_delay(cnx, cnx->path[0]->rtt_min, cnx->remote_parameters.min_ack_delay,
                                bw_estimate, &ack_gap, &ack_delay_max);
                            if (ack_gap != cnx->ack_gap_local) {
                                cnx->is_ack_frequency_updated = 1;
                            }
                        }
                    }
                }

                /* Bandwidth was estimated, update the references */
                path_x->delivered_last = path_x->delivered;
                path_x->delivered_time_last = delivery_time;
                path_x->delivered_sent_last = send_time;
                path_x->delivered_last_packet = delivered_prior;
                path_x->last_bw_estimate_path_limited = rs_is_path_limited;
                if (path_x->delivered_last_packet > path_x->delivered_limited_index) {
                    path_x->delivered_limited_index = 0;
                }
                /* Statistics */
                if (bw_estimate > path_x->bandwidth_estimate_max) {
                    path_x->bandwidth_estimate_max = bw_estimate;
                }
            }
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

## Pair `picoquic/frames.c:picoquic_compute_ack_delay_max`
C: `picoquic/frames.c:3047-3064 picoquic_compute_ack_delay_max`
Rust: `rs/fq/src/internal.rs:9021-9032 compute_ack_delay_max`

### C body
```c
{
    uint64_t ack_delay_max = rtt / 4;

    if (!cnx->is_ack_frequency_negotiated && !cnx->path[0]->is_ssthresh_initialized) {
        ack_delay_max /= 2;
    }

    if (ack_delay_max > PICOQUIC_ACK_DELAY_MAX) {
        ack_delay_max = PICOQUIC_ACK_DELAY_MAX;
    }

    if (ack_delay_max < remote_min_ack_delay) {
        ack_delay_max = remote_min_ack_delay;
    }

    return ack_delay_max;
}
```

### Rust body
```rust
        {
            ack_delay_max /= 2;
        }
```

## Pair `picoquic/frames.c:picoquic_check_frame_needs_repeat`
C: `picoquic/frames.c:3412-3674 picoquic_check_frame_needs_repeat`
Rust: `rs/fq/src/internal.rs:10337-10385 check_frame_needs_repeat`

### C body
```c
{
    int ret = 0;
    int fin;
    size_t data_length;
    uint64_t stream_id;
    uint64_t offset;
    uint64_t maxdata;
    uint64_t max_stream_rank;
    picoquic_stream_head_t* stream = NULL;
    size_t consumed = 0;

    *no_need_to_repeat = 0;

    if (PICOQUIC_IN_RANGE(bytes[0], picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max)) {
        ret = picoquic_parse_stream_header(bytes, bytes_max,
            &stream_id, &offset, &data_length, &fin, &consumed);

        if (ret == 0) {
            stream = picoquic_find_stream(cnx, stream_id);
            if (stream == NULL) {
                /* the stream was destroyed. That only happens if it was fully acked. */
                *no_need_to_repeat = 1;
            }
            else {
                if (stream->reset_sent) {
                    *no_need_to_repeat = 1;
                }
                else {
                    /* Check whether the ack was already received */
                    *no_need_to_repeat = picoquic_check_sack_list(&stream->sack_list, offset, offset + data_length - ((fin) ? 0 : 1));
                }

                if (is_preemptive_needed != NULL && stream->fin_sent) {
                    *is_preemptive_needed |= 1;
                }
            }
        }
    }
    else {
        const uint8_t* p_last_byte = bytes + bytes_max;
        switch (bytes[0]) {
        case picoquic_frame_type_max_data:
            if ((bytes = picoquic_frames_varint_decode(bytes + 1, p_last_byte, &maxdata)) == NULL) {
                /* Malformed frame, do not retransmit */
                *no_need_to_repeat = 1;
            }
            else if (maxdata < cnx->maxdata_local || maxdata <= cnx->maxdata_local_acked) {
                /* already updated or already acknowledged */
                *no_need_to_repeat = 1;
            }
            break;
        case picoquic_frame_type_max_stream_data:
            if ((bytes = picoquic_frames_varint_decode(bytes + 1, p_last_byte, &stream_id)) == NULL ||
                (bytes = picoquic_frames_varint_decode(bytes, p_last_byte, &maxdata)) == NULL) {
                /* Malformed frame, do not retransmit */
                *no_need_to_repeat = 1;
            }
            else if ((stream = picoquic_find_stream(cnx, stream_id)) == NULL) {
                /* No such stream do not retransmit */
                *no_need_to_repeat = 1;
            }
            else if (stream->fin_received || stream->reset_received || stream->stop_sending_sent) {
                /* Stream stopped, no need to increase the window */
                *no_need_to_repeat = 1;
            }
            else if (maxdata < stream->maxdata_local || maxdata <= stream->maxdata_local_acked) {
                /* Stream max data already increased or acked */
                *no_need_to_repeat = 1;
            }
            break;
        case picoquic_frame_type_max_streams_bidir:
        case picoquic_frame_type_max_streams_unidir:
            ret = picoquic_check_max_streams_frame_needs_repeat(cnx, bytes, p_last_byte, no_need_to_repeat);
            break;
        case picoquic_frame_type_data_blocked:
            if ((bytes = picoquic_frames_varint_decode(bytes + 1, p_last_byte, &maxdata)) == NULL) {
                /* Malformed frame, do not retransmit */
                *no_need_to_repeat = 1;
            }
            else if (maxdata < cnx->maxdata_remote) {
                /* already updated */
                *no_need_to_repeat = 1;
            }
            else {
                /* Only repeat if the sent flag is still there */
                *no_need_to_repeat = !cnx->sent_blocked_frame;
            }
            break;
        case picoquic_frame_type_streams_blocked_bidir:
            if ((bytes = picoquic_frames_varint_decode(bytes + 1, p_last_byte, &max_stream_rank)) == NULL) {
                /* Malformed frame, do not retransmit */
                *no_need_to_repeat = 1;
            }
            else if (cnx->max_stream_id_bidir_remote > STREAM_ID_FROM_RANK(max_stream_rank, cnx->client_mode, 0)) {
                /* Streams bidir already increased */
                *no_need_to_repeat = 1;
            }
            else {
                /* Only repeat if the sent flag is still there */
                *no_need_to_repeat = !cnx->stream_blocked_bidir_sent;
            }
            break;
        case picoquic_frame_type_streams_blocked_unidir:
            if ((bytes = picoquic_frames_varint_decode(bytes + 1, p_last_byte, &max_stream_rank)) == NULL) {
                /* Malformed frame, do not retransmit */
                *no_need_to_repeat = 1;
            }
            else if (cnx->max_stream_id_unidir_remote > STREAM_ID_FROM_RANK(max_stream_rank, cnx->client_mode, 1)) {
                /* Streams unidir already increased */
                *no_need_to_repeat = 1;
            }
            else {
                /* Only repeat if the sent flag is still there */
                *no_need_to_repeat = !cnx->stream_blocked_unidir_sent;
            }
            break;
        case picoquic_frame_type_stream_data_blocked:
            if ((bytes = picoquic_frames_varint_decode(bytes + 1, p_last_byte, &stream_id)) == NULL ||
                (bytes = picoquic_frames_varint_decode(bytes, p_last_byte, &maxdata)) == NULL) {
                /* Malformed frame, do not retransmit */
                *no_need_to_repeat = 1;
            }
            else if ((stream = picoquic_find_stream(cnx, stream_id)) == NULL) {
                /* No such stream do not retransmit */
                *no_need_to_repeat = 1;
            }
            else if (stream->fin_requested || stream->fin_sent || stream->reset_sent) {
                /* Stream stopped, no need to increase the window */
                *no_need_to_repeat = 1;
            }
            else if (maxdata < stream->maxdata_remote || !stream->stream_data_blocked_sent) {
                /* Stream max data already increased */
                *no_need_to_repeat = 1;
            }
            break;
        case picoquic_frame_type_path_challenge:
            /* Path challenge repeat follows its own logic. */
            *no_need_to_repeat = 1;
            break;
        case picoquic_frame_type_path_response:
            /* On the client side, challenge responses generally ought to be repeated in order to maximise
             * chances of handshake success. However, doing so on the server side may create a "blowback"
             * in case of attacks, if the initial challenge was set from an unreachable address, or if the
             * source address of the path challenge was forged.
             * If the node has sent several path responses, only the last one ought to be repeated.
             * If the path on which the response was sent is abandoned, there is no need to repeat
             * this frame. If the path is validated, then the response should always be repeated.
             */
            *no_need_to_repeat = picoquic_should_repeat_path_response_frame(cnx, bytes, bytes_max);
            break;
        case picoquic_frame_type_datagram:
        case picoquic_frame_type_datagram_l:
            /* Datagrams are never repeated. */
            *no_need_to_repeat = 1;
            *do_not_detect_spurious = 0;
            break;
        case picoquic_frame_type_handshake_done:
            /* No need to retransmit if one was previously acked */
            if (cnx->is_handshake_done_acked) {
                *no_need_to_repeat = 1;
            }
            break;
        case picoquic_frame_type_new_token:
            /* No need to retransmit if one was previously acked */
            if (cnx->is_new_token_acked) {
                *no_need_to_repeat = 1;
            }
            break;
        case picoquic_frame_type_crypto_hs:
            ret = picoquic_check_crypto_frame_needs_repeat(cnx, bytes, bytes_max, p_type, no_need_to_repeat);
            break;
        case picoquic_frame_type_new_connection_id:
            ret = picoquic_check_new_cid_needs_repeat(cnx, bytes, bytes_max, 0, no_need_to_repeat);
            break;
        case picoquic_frame_type_retire_connection_id:
            ret = picoquic_check_retire_connection_id_needs_repeat(cnx, bytes, bytes_max, no_need_to_repeat, 0);
            break;
        case picoquic_frame_type_reset_stream:
            ret = picoquic_check_reset_stream_needs_repeat(cnx, bytes, bytes_max, no_need_to_repeat);
            break;
        case picoquic_frame_type_stop_sending:
            ret = picoquic_check_stop_sending_needs_repeat(cnx, bytes, bytes_max, no_need_to_repeat);
            break;
        case picoquic_frame_type_reset_stream_at:
            ret = picoquic_check_reset_stream_at_needs_repeat(cnx, bytes, bytes_max, no_need_to_repeat);
            break;
        default: {
            uint64_t frame_id64;
            const uint8_t* type_bytes = bytes;
            const uint8_t* p_bytes_max = bytes + bytes_max;
            *no_need_to_repeat = 0;
            if ((bytes = picoquic_frames_varint_decode(bytes, p_bytes_max, &frame_id64)) != NULL) {
                switch (frame_id64) {
                case picoquic_frame_type_ack_frequency: {
                    uint64_t seq;
                    uint64_t packets;
                    uint64_t microsec;
                    uint8_t ignore_order;
                    uint64_t reordering_threshold;

                    if ((bytes = picoquic_parse_ack_frequency_frame(bytes, p_bytes_max,
                        &seq, &packets, &microsec, &ignore_order, &reordering_threshold)) == NULL) {
                        ret = -1;
                    } else if (seq == cnx->ack_frequency_sequence_local) {
                        *no_need_to_repeat = 1;
                    }
                    break;
                }
                case picoquic_frame_type_immediate_ack:
                    *no_need_to_repeat = 0;
                    break;
                case picoquic_frame_type_path_ack:
                case picoquic_frame_type_path_ack_ecn:
                case picoquic_frame_type_time_stamp:
                    *no_need_to_repeat = 1;
                    break;
                case picoquic_frame_type_path_abandon:
                    /* TODO: check whether there is still a need to abandon the path */
                    *no_need_to_repeat = 0;
                    break;
                case picoquic_frame_type_path_backup:
                case picoquic_frame_type_path_available:
                    (void)picoquic_path_available_or_backup_frame_need_repeat(cnx, bytes,
                        p_bytes_max, no_need_to_repeat);
                    break;
                case picoquic_frame_type_max_path_id:
                    (void)picoquic_max_path_id_frame_needs_repeat(cnx, bytes,
                        p_bytes_max, no_need_to_repeat);
                    break;
                case picoquic_frame_type_paths_blocked:
                    (void)picoquic_paths_blocked_frame_needs_repeat(cnx, bytes,
                        p_bytes_max, no_need_to_repeat);
                    break;
                case picoquic_frame_type_path_cid_blocked:
                    (void)picoquic_path_cid_blocked_frame_needs_repeat(cnx, bytes,
                        p_bytes_max, no_need_to_repeat);
                    break;
                case picoquic_frame_type_path_new_connection_id:
                    ret = picoquic_check_new_cid_needs_repeat(cnx, type_bytes, bytes_max, 1, no_need_to_repeat);
                    break;
                case picoquic_frame_type_path_retire_connection_id:
                    ret = picoquic_check_retire_connection_id_needs_repeat(cnx, type_bytes, bytes_max, no_need_to_repeat, 1);
                    break;
                case picoquic_frame_type_observed_address_v4:
                case picoquic_frame_type_observed_address_v6:
                    /* These frames have a special case processing, tied to path challenge */
                    ret = 0;
                    break;
                default:
                    *no_need_to_repeat = 0;
                    break;
                }
            }
            break;
        }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        *no_need_to_repeat = 0;
        *do_not_detect_spurious = 0;
        *is_preemptive_needed = 0;
        let max = bytes_max.min(bytes.len());
        if max == 0 {
            *no_need_to_repeat = 1;
            return 0;
        }
        let mut frame_type = 0;
        if frames_varint_decode(&bytes[..max], &mut frame_type).is_none() {
            return -1;
        }
        match frame_type {
            x if x == crate::frames::FrameType::Padding as u64
                || x == crate::frames::FrameType::Ack as u64
                || x == crate::frames::FrameType::AckEcn as u64
                || x == crate::frames::FrameType::PathAck as u64
                || x == crate::frames::FrameType::PathAckEcn as u64 =>
            {
                *no_need_to_repeat = 1;
            }
            x if x >= crate::frames::FrameType::StreamRangeMin as u64
                && x <= crate::frames::FrameType::StreamRangeMax as u64 =>
            {
                *is_preemptive_needed = if p_type == PacketType::OneRttProtected {
                    1
                } else {
                    0
                };
            }
            x if x == crate::frames::FrameType::PathResponse as u64
                || x == crate::frames::FrameType::ConnectionClose as u64
                || x == crate::frames::FrameType::ApplicationClose as u64 =>
            {
                *do_not_detect_spurious = 1;
            }
            _ => {}
        }
        0
    }
```

## Pair `picoquic/frames.c:picoquic_format_ack_frame`
C: `picoquic/frames.c:4218-4265 picoquic_format_ack_frame`
Rust: `rs/fq/src/internal.rs:12654-12740 format_ack_frame`

### C body
```c
{
    int need_time_stamp = (pc == picoquic_packet_context_application && cnx->is_time_stamp_sent);
    picoquic_ack_context_t* ack_ctx = NULL;

    if (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) {
        int ack_still_needed = 0;
        int ack_after_fin = 0;
        for (int path_id = 0; path_id < cnx->nb_paths; path_id++) {
            if (bytes != NULL) {
                ack_ctx = &cnx->path[path_id]->ack_ctx;
                /* Adding test to verify that we do not send too many acks after demotion. */
                if (cnx->path[path_id]->path_is_demoted &&
                    !ack_ctx->act[is_opportunistic].ack_needed &&
                    ack_ctx->sack_list.ack_tree.size == 1) {
                    picoquic_sack_item_t* last_sack = picoquic_sack_last_item(&ack_ctx->sack_list);
                    if (last_sack->nb_times_sent[is_opportunistic] >= PICOQUIC_MIN_ACK_RANGE_REPEAT) {
                        continue;
                    }
                }
                bytes = picoquic_format_ack_frame_in_context(cnx, bytes, bytes_max, more_data,
                    current_time, ack_ctx, &need_time_stamp, cnx->path[path_id]->unique_path_id, is_opportunistic);
                if (is_opportunistic) {
                    ack_still_needed |= ack_ctx->act[1].ack_needed;
                    ack_after_fin |= ack_ctx->act[1].ack_after_fin;
                } else {
                    ack_still_needed |= ack_ctx->act[0].ack_needed;
                    ack_after_fin |= ack_ctx->act[0].ack_after_fin;
                }
            }
        }
        if (is_opportunistic) {
            cnx->ack_ctx[pc].act[1].ack_needed = ack_still_needed;
            cnx->ack_ctx[pc].act[1].ack_after_fin = ack_after_fin;
        }
        else {
            cnx->ack_ctx[pc].act[0].ack_needed = ack_still_needed;
            cnx->ack_ctx[pc].act[0].ack_after_fin = ack_after_fin;
        }
    }
    else {
        bytes = picoquic_format_ack_frame_in_context(cnx, bytes, bytes_max, more_data,
            current_time, &cnx->ack_ctx[pc], &need_time_stamp, UINT64_MAX, is_opportunistic);
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut need_time_stamp =
        (pc == PacketContext::Application && connection.is_time_stamp_sent) as i32;
    let idx = is_opportunistic.clamp(0, 1) as usize;
    let ack_delay_exponent = connection.local_parameters.ack_delay_exponent;
    let start_time = connection.start_time;

    if connection.is_multipath_enabled && pc == PacketContext::Application {
        let mut ack_still_needed = false;
        let mut ack_after_fin = false;

        for path_id in 0..connection.paths.len() {
            let skip_demoted = {
                let path = &connection.paths[path_id];
                path.path_is_demoted
                    && !path.ack_ctx.act[idx].ack_needed
                    && path.ack_ctx.sack_list.size() == 1
                    && path
                        .ack_ctx
                        .sack_list
                        .first_range()
                        .and_then(|tok| path.ack_ctx.sack_list.sack_items.get(tok))
                        .map(|item| {
                            item.nb_times_sent(is_opportunistic) >= MIN_ACK_RANGE_REPEAT as i32
                        })
                        .unwrap_or(false)
            };
            if skip_demoted {
                continue;
            }

            let unique_path_id = connection.paths[path_id].unique_path_id;
            let (tail, clear_immediate_ack) = {
                let ack_ctx = &mut connection.paths[path_id].ack_ctx;
                format_ack_frame_in_context_impl(
                    bytes,
                    more_data,
                    current_time,
                    ack_ctx,
                    &mut need_time_stamp,
                    unique_path_id,
                    is_opportunistic,
                    ack_delay_exponent,
                    start_time,
                )
            };
            if clear_immediate_ack {
                connection.is_immediate_ack_required = false;
            }
            bytes = tail?;

            let track = &connection.paths[path_id].ack_ctx.act[idx];
            ack_still_needed |= track.ack_needed;
            ack_after_fin |= track.ack_after_fin;
        }

        connection.ack_ctx[pc as usize].act[idx].ack_needed = ack_still_needed;
        connection.ack_ctx[pc as usize].act[idx].ack_after_fin = ack_after_fin;
        Some(bytes)
    } else {
        let (tail, clear_immediate_ack) = {
            let ack_ctx = &mut connection.ack_ctx[pc as usize];
            format_ack_frame_in_context_impl(
                bytes,
                more_data,
                current_time,
                ack_ctx,
                &mut need_time_stamp,
                u64::MAX,
                is_opportunistic,
                ack_delay_exponent,
                start_time,
            )
        };
        if clear_immediate_ack {
            connection.is_immediate_ack_required = false;
        }
        tail
    }
}
```

## Pair `picoquic/frames.c:picoquic_is_ack_needed`
C: `picoquic/frames.c:4355-4371 picoquic_is_ack_needed`
Rust: `rs/fq/src/internal.rs:7960-8011 is_ack_needed`

### C body
```c
{
    int ret = picoquic_is_ack_needed_in_ctx(cnx, &cnx->ack_ctx[pc], current_time, 0, next_wake_time, 
        pc, is_opportunistic);

    if (pc == picoquic_packet_context_application) {
        if (cnx->is_multipath_enabled) {
            for (int i = 0; ret == 0 && i < cnx->nb_paths; i++) {
                ret |= picoquic_is_ack_needed_in_ctx(cnx, &cnx->path[i]->ack_ctx, current_time, i,
                    next_wake_time, pc, is_opportunistic);
            }
        }
    }
    return ret;

}
```

### Rust body
```rust
    ) -> bool {
        let limits = AckNeededLimits {
            ack_ignore_order_remote: self.ack_ignore_order_remote,
            ack_delay_remote: self.ack_delay_remote,
        };
        let pc_index = pc as usize;
        let main_ack_gap = self.ack_gap_override_if_needed(0);
        let mut ret = is_ack_needed_in_ctx_inner(
            &mut self.ack_ctx[pc_index],
            current_time,
            main_ack_gap,
            next_wake_time,
            pc,
            is_opportunistic,
            limits,
        );

        if pc == PacketContext::Application && self.is_multipath_enabled {
            let ack_gap_remote = self.ack_gap_remote;
            let is_multipath_enabled = self.is_multipath_enabled;
            let nb_packets_received = self.nb_packets_received;

            for i in 0..self.paths.len() {
                if ret {
                    break;
                }
                let path_ack_gap = ack_gap_override_for_path(
                    ack_gap_remote,
                    is_multipath_enabled,
                    nb_packets_received,
                    Some(&self.paths[i]),
                );
                ret = is_ack_needed_in_ctx_inner(
                    &mut self.paths[i].ack_ctx,
                    current_time,
                    path_ack_gap,
                    next_wake_time,
                    pc,
                    is_opportunistic,
                    limits,
                );
            }
        }

        ret
    }
```

## Pair `picoquic/frames.c:picoquic_format_max_stream_data_frame`
C: `picoquic/frames.c:4545-4565 picoquic_format_max_stream_data_frame`
Rust: `rs/fq/src/internal.rs:12861-12885 format_max_stream_data_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_max_stream_data)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->stream_id)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, new_max_data)) != NULL) {
        stream->maxdata_local = new_max_data;
        if (new_max_data > cnx->max_stream_data_local) {
            cnx->max_stream_data_local = new_max_data;
        }
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
    if bytes.is_empty() {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[0] = crate::frames::FrameType::MaxStreamData as u8;
    let mut off = 1;
    if !encode_varint_at(bytes, &mut off, stream.stream_id)
        || !encode_varint_at(bytes, &mut off, new_max_data)
    {
        *more_data = 1;
        return Some(bytes);
    }
    stream.maxdata_local = new_max_data;
    connection.max_stream_data_local = connection.max_stream_data_local.max(new_max_data);
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## Pair `picoquic/frames.c:picoquic_format_first_misc_or_dg_frame`
C: `picoquic/frames.c:4827-4842 picoquic_format_first_misc_or_dg_frame`
Rust: `rs/fq/src/internal.rs:13752-13769 format_first_misc_or_dg_frame`

### C body
```c
{
    if (bytes + misc_frame->length > bytes_max) {
        *more_data = 1;
    } else {
        uint8_t* frame = ((uint8_t*)misc_frame) + sizeof(picoquic_misc_frame_header_t);
        memcpy(bytes, frame, misc_frame->length);
        bytes += misc_frame->length;
        *is_pure_ack &= misc_frame->is_pure_ack;
        picoquic_delete_misc_or_dg(first, last, misc_frame);
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    // Consume the head of the queue and copy its bytes into the output buffer.
    let frame = queue.pop_front()?;
    let len = frame.bytes.len();
    if bytes.len() >= len {
        bytes[..len].copy_from_slice(&frame.bytes);
        Some(&mut bytes[len..])
    } else {
        // Frame doesn't fit — put it back and return None.
        queue.push_front(frame);
        None
    }
}
```
