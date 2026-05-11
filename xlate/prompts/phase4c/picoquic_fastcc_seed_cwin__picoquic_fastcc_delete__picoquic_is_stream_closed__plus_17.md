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

## Pair `picoquic/fastcc.c:picoquic_fastcc_seed_cwin`
C: `picoquic/fastcc.c:85-92 picoquic_fastcc_seed_cwin`
Rust: `rs/fq/src/fastcc.rs:103-124 fastcc_seed_cwin`

### C body
```c
{
    if (fastcc_state->alg_state == picoquic_fastcc_initial) {
        if (path_x->cwin < bytes_in_flight) {
            path_x->cwin = bytes_in_flight;
        }
    }
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

## Pair `picoquic/fastcc.c:picoquic_fastcc_delete`
C: `picoquic/fastcc.c:308-315 picoquic_fastcc_delete`
Rust: `rs/fq/src/fastcc.rs:355-365 alg_delete`

### C body
```c
{
    if (path_x->congestion_alg_state != NULL) {
        free(path_x->congestion_alg_state);
        path_x->congestion_alg_state = NULL;
    }
}
```

### Rust body
```rust
    fn alg_observe(&self, path_x: &Path) -> Option<(u64, u64)> {
        path_x
            .congestion_alg_state
            .as_ref()
            .and_then(|s| s.downcast_ref::<FastccState>())
            .map(|state| (state.alg_state as u64, state.rolling_rtt_min))
    }
```

## Pair `picoquic/frames.c:picoquic_is_stream_closed`
C: `picoquic/frames.c:98-115 picoquic_is_stream_closed`
Rust: `rs/fq/src/internal.rs:9726-9746 is_stream_closed`

### C body
```c
{
    int is_closed = 0;

    if (IS_BIDIR_STREAM_ID(stream->stream_id)) {
        is_closed = ((stream->fin_requested && stream->fin_sent) || (stream->reset_requested && stream->reset_sent)) &&
            ((stream->fin_received && stream->fin_signalled) || (stream->reset_received && stream->reset_signalled));
    }
    else if (IS_LOCAL_STREAM_ID(stream->stream_id, client_mode)) {
        /* Unidir from local host*/
        is_closed = ((stream->fin_requested && stream->fin_sent) || (stream->reset_requested && stream->reset_sent));
    }
    else {
        is_closed = ((stream->fin_received && stream->fin_signalled) || (stream->reset_received && stream->reset_signalled));
    }

    return is_closed;
}
```

### Rust body
```rust
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
```

## Pair `picoquic/frames.c:picoquic_enforce_reset_stream_frame`
C: `picoquic/frames.c:252-282 picoquic_enforce_reset_stream_frame`
Rust: `rs/fq/src/internal.rs:9754-9762 enforce_reset_stream_frame`

### C body
```c
{
    picoquic_stream_queue_node_t* next = stream->send_queue;
    picoquic_stream_queue_node_t* previous = NULL;

    stream->reset_sent = 1;

    /* Check if any of the data in the send queue should still be sent */
    while (next != NULL && next->offset < reliable_size) {
        previous = next;
        next = next->next_stream_data;
    }

    /* Free all the queued data after the previous pointer */
    while (next != NULL) {
        picoquic_stream_queue_node_t* not_needed = next;
        next = next->next_stream_data;

        if (not_needed->bytes != NULL) {
            free(not_needed->bytes);
        }
        free(not_needed);
    }
    /* reset the queue pointer */
    if (previous == NULL) {
        stream->send_queue = NULL;
    }
    else {
        previous->next_stream_data = NULL;
    }
}
```

### Rust body
```rust
    pub fn enforce_reset_stream_frame(&mut self, reliable_size: u64) {
        self.reset_sent = true;
        // send_queue is ordered by offset; find the first node that lies
        // at or beyond reliable_size and truncate everything from there.
        let keep = self
            .send_queue
            .partition_point(|node| node.offset < reliable_size);
        self.send_queue.truncate(keep);
    }
```

## Pair `picoquic/frames.c:picoquic_format_reset_stream_at_frame`
C: `picoquic/frames.c:485-505 picoquic_format_reset_stream_at_frame`
Rust: `rs/fq/src/internal.rs:9794-9820 format_reset_stream_at_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_reset_stream_at)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->stream_id)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->local_error)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->sent_offset)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->reliable_size)) != NULL)
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
        crate::frames::FrameType::ResetStreamAt as u64,
        stream_id,
        local_error,
        sent_offset,
        reliable_size,
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

## Pair `picoquic/frames.c:picoquic_format_new_token_frame`
C: `picoquic/frames.c:1013-1027 picoquic_format_new_token_frame`
Rust: `rs/fq/src/internal.rs:13542-13549 format_new_token_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes; 
    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_new_token)) != NULL &&
        (bytes = picoquic_frames_length_data_encode(bytes, bytes_max, token_length, token)) != NULL) {
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
        format_new_token_frame(bytes, more_data, is_pure_ack, token)
    }
```

## Pair `picoquic/frames.c:picoquic_parse_stream_header`
C: `picoquic/frames.c:1204-1260 picoquic_parse_stream_header`
Rust: `rs/fq/src/internal.rs:12280-12338 parse_stream_header`

### C body
```c
{
    int ret = 0;
    int len = bytes[0] & 2;
    int off = bytes[0] & 4;
    uint64_t length = 0;
    size_t l_stream = 0;
    size_t l_len = 0;
    size_t l_off = 0;
    size_t byte_index = 1;

    *fin = bytes[0] & 1;

    if (bytes_max > byte_index) {
        l_stream = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, stream_id);
        byte_index += l_stream;
    }

    if (off == 0) {
        *offset = 0;
    } else if (bytes_max > byte_index) {
        l_off = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, offset);
        byte_index += l_off;
    }

    if (bytes_max < byte_index || l_stream == 0 || (off != 0 && l_off == 0)) {
        DBG_PRINTF("stream frame header too large: first_byte=0x%02x, bytes_max=%" PRIst,
            bytes[0], bytes_max);
        *data_length = 0;
        byte_index = bytes_max;
        ret = -1;
    } else if (len == 0) {
        *data_length = bytes_max - byte_index;
    } else {
        if (bytes_max > byte_index) {
            l_len = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, &length);
            byte_index += l_len;
            *data_length = (size_t)length;
        }

        if (l_len == 0 || bytes_max < byte_index) {
            DBG_PRINTF("stream frame header too large: first_byte=0x%02x, bytes_max=%" PRIst,
                bytes[0], bytes_max);
            byte_index = bytes_max;
            ret = -1;
        } else if (byte_index + length > bytes_max) {
            DBG_PRINTF("stream data past the end of the packet: first_byte=0x%02x, data_length=%" PRIst ", max_bytes=%" PRIst,
                bytes[0], *data_length, bytes_max);
            ret = -1;
        }
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
        *data_length = 0;
        return -1;
    }

    let frame_type = bytes[0];
    let has_len = (frame_type & 0x02) != 0;
    let has_offset = (frame_type & 0x04) != 0;
    let mut off_idx = 1;
    *fin = (frame_type & 0x01) as i32;

    let l_stream = varint_decode(&bytes[off_idx..max], stream_id);
    off_idx += l_stream;
    if l_stream == 0 {
        *data_length = 0;
        *consumed = max;
        return -1;
    }

    if has_offset {
        let l_offset = varint_decode(&bytes[off_idx..max], offset);
        off_idx += l_offset;
        if l_offset == 0 {
            *data_length = 0;
            *consumed = max;
            return -1;
        }
    } else {
        *offset = 0;
    }

    if has_len {
        let mut length = 0;
        let l_len = varint_decode(&bytes[off_idx..max], &mut length);
        off_idx += l_len;
        if l_len == 0 || off_idx > max || off_idx.saturating_add(length as usize) > max {
            *data_length = 0;
            *consumed = max;
            return -1;
        }
        *data_length = length as usize;
    } else {
        *data_length = max - off_idx;
    }

    *consumed = off_idx;
    0
}
```

## Pair `picoquic/frames.c:picoquic_find_ready_stream`
C: `picoquic/frames.c:1666-1669 picoquic_find_ready_stream`
Rust: `rs/fq/src/internal.rs:9701-9714 find_ready_stream`

### C body
```c
{
    return picoquic_find_ready_stream_path(cnx, NULL, 0);
}
```

### Rust body
```rust
        self.output_streams.iter().copied().find(|tok| {
            self.streams.get(*tok).is_some_and(|stream| {
                let control_frame_ready = (stream.stop_sending_requested
                    && !stream.stop_sending_sent)
                    || (stream.reset_requested && !stream.reset_sent);
                let data_ready = self.maxdata_remote > self.data_sent
                    && stream.sent_offset < stream.maxdata_remote
                    && (stream.is_active
                        || !stream.send_queue.is_empty()
                        || (stream.fin_requested && !stream.fin_sent));
                control_frame_ready || data_ready
            })
        })
```

## Pair `picoquic/frames.c:picoquic_format_one_blocked_frame`
C: `picoquic/frames.c:1750-1780 picoquic_format_one_blocked_frame`
Rust: `rs/fq/src/internal.rs:13704-13746 format_one_blocked_frame`

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

## Pair `picoquic/frames.c:picoquic_format_stream_frame_header`
C: `picoquic/frames.c:1866-1878 picoquic_format_stream_frame_header`
Rust: `rs/fq/src/internal.rs:12254-12261 format_stream_frame_header`

### C body
```c
{
    uint8_t* bytes0 = bytes;
    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_stream_range_min)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream_id)) != NULL) {
        if (offset > 0) {
            *bytes0 |= 4; /* Indicates presence of offset */
            bytes = picoquic_frames_varint_encode(bytes, bytes_max, offset);
        }
    }

    return bytes;
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## Pair `picoquic/frames.c:picoquic_queue_data_repeat_node_value`
C: `picoquic/frames.c:2138-2141 picoquic_queue_data_repeat_node_value`
Rust: `rs/fq/src/internal.rs:5714-5716 queue_data_repeat_node_value`

### C body
```c
{
    return (void*)((char*)node - offsetof(struct st_picoquic_packet_t, queue_data_repeat_node));
}
```

### Rust body
```rust
    pub fn queue_data_repeat_node_value(&self, splay_tok: SplayToken) -> Option<PacketToken> {
        self.queue_data_repeat_tree.get(splay_tok).copied()
    }
```

## Pair `picoquic/frames.c:picoquic_queue_data_repeat_adjust`
C: `picoquic/frames.c:2203-2252 picoquic_queue_data_repeat_adjust`
Rust: `rs/fq/src/internal.rs:10486-10539 picoquic_queue_data_repeat_adjust`

### C body
```c
{
    int ret = 0;
    while (packet->data_repeat_frame < packet->length) {
        uint8_t* data_byte = packet->bytes + packet->data_repeat_frame;
        if (*data_byte >= picoquic_frame_type_stream_range_min && *data_byte <= picoquic_frame_type_stream_range_max) {
            /* next frame is a stream data frame. Make sure that the pointers point to it,
            * and adjust the packet priority */
            size_t consumed;
            int fin;

            packet->data_repeat_priority = 0;
            packet->data_repeat_stream_id = 0;
            packet->data_repeat_stream_offset = 0;
            packet->data_repeat_stream_data_length = 0;

            if (picoquic_parse_stream_header(data_byte, packet->length - packet->data_repeat_frame,
                &packet->data_repeat_stream_id, &packet->data_repeat_stream_offset, 
                &packet->data_repeat_stream_data_length, &fin, &consumed) == 0) {
                /* Find the stream and its priority */
                picoquic_stream_head_t* stream = picoquic_find_stream(cnx, packet->data_repeat_stream_id);
                if (stream == NULL) {
                    packet->data_repeat_priority = 0;
                }
                else {
                    packet->data_repeat_priority = stream->stream_priority;
                }
            }
            else {
                /* Malformed packet, internal error */
                ret = -1;
            }
            break;
        }
        else {
            int forget_about_ack = 0;
            size_t consumed = 0;
            if (picoquic_skip_frame(data_byte, packet->length - packet->data_repeat_frame, &consumed, &forget_about_ack) != 0) {
                /* Malformed frame, internal error! */
                ret = -1;
                break;
            }
            else {
                packet->data_repeat_frame += consumed;
                packet->data_repeat_index = packet->data_repeat_frame;
            }
        }
    }
    return ret;
}
```

### Rust body
```rust
pub fn picoquic_queue_data_repeat_adjust(connection: &mut Connection, packet: &mut Packet) -> i32 {
    let mut ret = 0;
    while packet.data_repeat_frame < packet.length {
        let data_offset = packet.data_repeat_frame;
        let data_byte = packet.bytes[data_offset];
        if data_byte >= crate::frames::FrameType::StreamRangeMin as u8
            && data_byte <= crate::frames::FrameType::StreamRangeMax as u8
        {
            let mut fin = 0;
            let mut consumed = 0;
            packet.data_repeat_priority = 0;
            packet.data_repeat_stream_id = 0;
            packet.data_repeat_stream_offset = 0;
            packet.data_repeat_stream_data_length = 0;

            if parse_stream_header(
                &packet.bytes[data_offset..packet.length],
                packet.length - data_offset,
                &mut packet.data_repeat_stream_id,
                &mut packet.data_repeat_stream_offset,
                &mut packet.data_repeat_stream_data_length,
                &mut fin,
                &mut consumed,
            ) == 0
            {
                if let Some(stream_token) = connection.find_stream(packet.data_repeat_stream_id)
                    && let Some(stream) = connection.streams.get(stream_token)
                {
                    packet.data_repeat_priority = stream.stream_priority as u64;
                }
            } else {
                ret = -1;
            }
            break;
        }

        let mut forget_about_ack = 0;
        let mut consumed = 0;
        if skip_frame(
            &packet.bytes[data_offset..packet.length],
            packet.length - data_offset,
            &mut consumed,
            &mut forget_about_ack,
        ) != 0
            || consumed == 0
        {
            ret = -1;
            break;
        }
        packet.data_repeat_frame += consumed;
        packet.data_repeat_index = packet.data_repeat_frame;
    }
    ret
}
```

## Pair `picoquic/frames.c:picoquic_copy_single_stream_frame_for_retransmit`
C: `picoquic/frames.c:2399-2460 picoquic_copy_single_stream_frame_for_retransmit`
Rust: `rs/fq/src/internal.rs:10664-10709 picoquic_copy_single_stream_frame_for_retransmit`

### C body
```c
{
    /* Assume that the "data_repeat_frame" and "data_repeat_index are
    * properly initialized when the packet is placed in the queue */
    size_t last_frame = packet->data_repeat_frame;
    if (packet->data_repeat_frame < packet->length) {
        /* Copy the current stream frame. */
        uint8_t* data_byte = packet->bytes + packet->data_repeat_frame;
        if (*data_byte >= picoquic_frame_type_stream_range_min && *data_byte <= picoquic_frame_type_stream_range_max) {
            /* next frame is a stream data frame. Try to add its content */
            uint8_t* bytes_first = bytes_next;
            bytes_next = picoquic_copy_stream_frame_for_retransmit(cnx, packet, bytes_next, bytes_max);
            if (bytes_next != NULL && bytes_next > bytes_first) {
                /* added something */
                *is_pure_ack &= 0;
            }
        }
    }
    /* Adjust to the next stream data boundary */
    if (packet->data_repeat_frame < packet->length &&
        picoquic_queue_data_repeat_adjust(cnx, packet) != 0) {
        /* signal an error */
        bytes_next = NULL;
    }
    /* Check whether the packet is completely processed, and can be dequeued */
    if (packet->data_repeat_frame >= packet->length) {
        /* Nothing left in this pasket. It can be safely dequeued */
        picoquic_dequeue_data_repeat_packet(cnx, packet);
        *packet_dequeued = 1;
    }
    else if (packet->data_repeat_frame > last_frame) {
        /* There is another stream frame after this one.  Dequeue with
        * caution, then requeue */
        int was_queued = packet->is_queued_for_spurious_detection;
        packet->is_queued_for_spurious_detection = 1;
        picosplay_delete_hint(&cnx->queue_data_repeat_tree, &packet->queue_data_repeat_node);
        packet->is_queued_for_spurious_detection = was_queued;
        (void)picosplay_insert(&cnx->queue_data_repeat_tree, packet);
        packet->is_queued_for_data_repeat = 1;
        *more_data |= 1;
    }
    else {
        *more_data |= 1;
    }

    return (bytes_next);
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let last_frame = packet.data_repeat_frame;
    let mut tail = bytes;

    if packet.data_repeat_frame < packet.length {
        let data_byte = packet.bytes[packet.data_repeat_frame];
        if data_byte >= crate::frames::FrameType::StreamRangeMin as u8
            && data_byte <= crate::frames::FrameType::StreamRangeMax as u8
        {
            let before = tail.len();
            tail = copy_stream_frame_for_retransmit(connection, packet, tail)?;
            if tail.len() < before {
                *is_pure_ack = 0;
            }
        }
    }

    if packet.data_repeat_frame < packet.length
        && picoquic_queue_data_repeat_adjust(connection, packet) != 0
    {
        return None;
    }

    if packet.data_repeat_frame >= packet.length {
        connection.dequeue_data_repeat_packet(packet);
        *packet_dequeued = 1;
    } else if packet.data_repeat_frame > last_frame {
        let was_queued = packet.is_queued_for_spurious_detection;
        packet.is_queued_for_spurious_detection = true;
        connection.dequeue_data_repeat_packet(packet);
        packet.is_queued_for_spurious_detection = was_queued;
        connection.queue_data_repeat_packet_current(packet);
        *more_data |= 1;
    } else {
        *more_data |= 1;
    }

    Some(tail)
}
```

## Pair `picoquic/frames.c:picoquic_format_crypto_hs_frame`
C: `picoquic/frames.c:2654-2706 picoquic_format_crypto_hs_frame`
Rust: `rs/fq/src/internal.rs:12419-12436 format_crypto_hs_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if (stream->send_queue != NULL && stream->send_queue->length > stream->send_queue->offset) {
        /* Check that there is enough room for at least 2 content bytes */
        if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_crypto_hs)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, stream->sent_offset)) != NULL) {
            /* As there is enough room, estimate the actual length, then encode the packet */
            size_t length = stream->send_queue->length - (size_t)stream->send_queue->offset;
            uint8_t* bytes_l;

            if (bytes + length > bytes_max) {
                length = bytes_max - bytes;
            }

            if ((bytes_l = picoquic_frames_varint_encode(bytes, bytes_max, length)) == NULL) {
                /* *more_data = 1; */
                bytes = bytes0;
            }
            else {
                if (bytes_l + length > bytes_max) {
                    length = bytes_max - bytes_l;
                    bytes = picoquic_frames_varint_encode(bytes, bytes_max, length);
                }
                else {
                    bytes = bytes_l;
                }
                if (bytes != NULL && length > 0) {
                    memcpy(bytes, stream->send_queue->bytes + stream->send_queue->offset, length);
                    bytes += length;

                    stream->send_queue->offset += length;
                    if (stream->send_queue->offset >= stream->send_queue->length) {
                        picoquic_stream_queue_node_t* next = stream->send_queue->next_stream_data;
                        free(stream->send_queue->bytes);
                        free(stream->send_queue);
                        stream->send_queue = next;
                    }

                    stream->sent_offset += length;
                    *is_pure_ack = 0;
                }
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
    let mut still_active = 0;
    let mut ret = 0;
    encode_stream_like_frame(
        stream,
        bytes,
        more_data,
        is_pure_ack,
        &mut still_active,
        &mut ret,
        true,
    )
}
```

## Pair `picoquic/frames.c:picoquic_compute_packets_in_window`
C: `picoquic/frames.c:2974-2995 picoquic_compute_packets_in_window`
Rust: `rs/fq/src/internal.rs:8938-8948 compute_packets_in_window`

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

## Pair `picoquic/frames.c:picoquic_record_ack_packet_data`
C: `picoquic/frames.c:3132-3168 picoquic_record_ack_packet_data`
Rust: `rs/fq/src/internal.rs:8721-8724 record_ack_packet_data`

### C body
```c
{
    picoquic_path_t* old_path = acked_packet->send_path;

    if (old_path != NULL) {
        /* Find the path index in the packet data structure */
        int path_i = 0;
        while (path_i < packet_data->nb_path_ack &&
            packet_data->path_ack[path_i].acked_path != old_path) {
            path_i++;
        }
        if (path_i == packet_data->nb_path_ack) {
            if (path_i > PICOQUIC_NB_PATH_TARGET) {
                /* Too many ACKs in this packet -- do not update path status. */
                return;
            }
            packet_data->nb_path_ack++;
            packet_data->path_ack[path_i].acked_path = old_path;
        }

        if (!packet_data->path_ack[path_i].is_set) {
            packet_data->path_ack[path_i].largest_sent_time = acked_packet->send_time;
            packet_data->path_ack[path_i].delivered_prior = acked_packet->delivered_prior;
            packet_data->path_ack[path_i].delivered_time_prior = acked_packet->delivered_time_prior;
            packet_data->path_ack[path_i].delivered_sent_prior = acked_packet->delivered_sent_prior;
            packet_data->path_ack[path_i].lost_prior = acked_packet->lost_prior;
            packet_data->path_ack[path_i].inflight_prior = acked_packet->inflight_prior;
            packet_data->path_ack[path_i].rs_is_path_limited = acked_packet->delivered_app_limited;
            packet_data->path_ack[path_i].rs_is_cwnd_limited = acked_packet->sent_cwin_limited;
            packet_data->path_ack[path_i].is_set = 1;
        }
        packet_data->path_ack[path_i].data_acked += acked_packet->length;
    }
}
```

### Rust body
```rust
        let Some(send_path) = acked_packet.send_path else {
            return;
        };
```

## Pair `picoquic/frames.c:picoquic_process_ack_of_frames`
C: `picoquic/frames.c:3706-3850 picoquic_process_ack_of_frames`
Rust: `rs/fq/src/internal.rs:11946-11953 process_ack_of_frames`

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

## Pair `picoquic/frames.c:picoquic_ack_gap_override_if_needed`
C: `picoquic/frames.c:4290-4307 picoquic_ack_gap_override_if_needed`
Rust: `rs/fq/src/internal.rs:9049-9084 ack_gap_override_if_needed`

### C body
```c
{
    uint64_t ack_gap = cnx->ack_gap_remote;
    if (cnx->is_multipath_enabled) {
        if (!cnx->path[path_index]->path_is_demoted &&
            !cnx->path[path_index]->first_tuple->challenge_failed &&
            !cnx->path[path_index]->first_tuple->response_required &&
            cnx->path[path_index]->first_tuple->challenge_verified &&
            cnx->path[path_index]->received < 100 * PICOQUIC_MAX_PACKET_SIZE) {
            ack_gap = 2;
        }
    }
    else if (cnx->nb_packets_received < 128) {
        ack_gap = 2;
    }

    return ack_gap;
}
```

### Rust body
```rust
) -> u64 {
    let mut ack_gap = ack_gap_remote;
    if is_multipath_enabled {
        if let Some(path) = path {
            let first_tuple_ok = path
                .tuples
                .first()
                .map(|t| !t.challenge_failed && !t.response_required && t.challenge_verified)
                .unwrap_or(false);
            if !path.path_is_demoted
                && first_tuple_ok
                && path.received < 100 * MAX_PACKET_SIZE as u64
            {
                ack_gap = 2;
            }
        }
    } else if nb_packets_received < 128 {
        ack_gap = 2;
    }
    ack_gap
}
```

## Pair `picoquic/frames.c:picoquic_format_application_close_frame`
C: `picoquic/frames.c:4430-4445 picoquic_format_application_close_frame`
Rust: `rs/fq/src/internal.rs:12769-12793 format_application_close_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_application_close)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, cnx->application_error)) != NULL &&
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
        crate::frames::FrameType::ApplicationClose as u64,
    ) || !encode_varint_at(bytes, &mut off, connection.application_error)
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

## Pair `picoquic/frames.c:picoquic_format_max_streams_frame_if_needed`
C: `picoquic/frames.c:4664-4700 picoquic_format_max_streams_frame_if_needed`
Rust: `rs/fq/src/internal.rs:12893-12941 format_max_streams_frame_if_needed`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if (cnx->max_stream_id_bidir_local_computed + 
        2*cnx->local_parameters.initial_max_stream_id_bidir > cnx->max_stream_id_bidir_local) {
        uint64_t new_bidir_local = cnx->max_stream_id_bidir_local +
            4 * cnx->local_parameters.initial_max_stream_id_bidir;
        if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_max_streams_bidir)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, STREAM_RANK_FROM_ID(new_bidir_local))) != NULL) {
            cnx->max_stream_id_bidir_local = new_bidir_local;
            *is_pure_ack = 0;
            bytes0 = bytes;
        } else {
            *more_data = 1;
            bytes = bytes0;
        }
    }
    
    if (cnx->max_stream_id_unidir_local_computed +
        2*cnx->local_parameters.initial_max_stream_id_unidir > cnx->max_stream_id_unidir_local) {
        uint64_t new_unidir_local = cnx->max_stream_id_unidir_local + 4*cnx->local_parameters.initial_max_stream_id_unidir;

        if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_max_streams_unidir)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, STREAM_RANK_FROM_ID(new_unidir_local))) != NULL) {
            cnx->max_stream_id_unidir_local = new_unidir_local;
            *is_pure_ack = 0;
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
    let mut off = 0;
    if connection.max_stream_id_bidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_bidir
        > connection.max_stream_id_bidir_local
    {
        let new_bidir = connection.max_stream_id_bidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_bidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsBidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_bidir).rank()) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.max_stream_id_bidir_local = new_bidir;
        *is_pure_ack = 0;
    }

    if connection.max_stream_id_unidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_unidir
        > connection.max_stream_id_unidir_local
    {
        let new_unidir = connection.max_stream_id_unidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_unidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsUnidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_unidir).rank()) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.max_stream_id_unidir_local = new_unidir;
        *is_pure_ack = 0;
    }

    Some(&mut bytes[off..])
}
```
