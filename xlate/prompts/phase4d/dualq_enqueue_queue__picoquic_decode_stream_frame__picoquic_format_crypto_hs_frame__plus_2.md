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

## `picoquic/dualq_aqm.c:dualq_enqueue_queue`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both enqueue and add packet length, but C also increments count and explicitly maintains first/last/next pointers; Rust only updates queue_bytes and pushes to a deque.
* C source: `picoquic/dualq_aqm.c:99-112`
* C signature: `void dualq_enqueue_queue(dualq_queue_t *, picoquictest_sim_packet_t *)`
* Rust source: `rs/fq/src/tests/dualq.rs:80-83`
* Rust item: `enqueue`

### C body
```c
{
    if (xq->queue_first == NULL) {
        xq->queue_first = packet;
        xq->queue_last = packet;
    }
    else {
        xq->queue_last->next_packet = packet;
        xq->queue_last = packet;
    }
    packet->next_packet = 0;
    xq->count += 1;
    xq->queue_bytes += packet->length;
}
```

### Rust body
```rust
    pub fn enqueue(&mut self, packet: TestSimPacket) {
        self.queue_bytes += packet.length as u64;
        self.packets.push_back(packet);
    }
```

## `picoquic/frames.c:picoquic_decode_stream_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C calls picoquic_connection_error on malformed headers or offset overflow and delegates delivery to picoquic_stream_network_input with last-frame detection; Rust returns None without visible connection_error and manually queues data without the same visible checks.
* C source: `picoquic/frames.c:1527-1560`
* C signature: `const uint8_t * picoquic_decode_stream_frame(picoquic_cnx_t *, const uint8_t *, const uint8_t *, picoquic_stream_data_node_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:9952-9996`
* Rust item: `decode_stream_frame`

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

## `picoquic/frames.c:picoquic_format_crypto_hs_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body explicitly encodes and consumes crypto stream data; Rust body delegates all behavior to encode_stream_like_frame, whose body is not shown.
* C source: `picoquic/frames.c:2654-2706`
* C signature: `uint8_t * picoquic_format_crypto_hs_frame(picoquic_stream_head_t *, uint8_t *, uint8_t *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:12419-12436`
* Rust item: `format_crypto_hs_frame`

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

## `picoquic/frames.c:picoquic_provide_stream_data_buffer`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust manually encodes length and returns a slice, but the C body calls the length helper and stores app_buffer; Rust also clears is_fin when false while C does not visibly do so.
* C source: `picoquic/frames.c:1840-1864`
* C signature: `uint8_t * picoquic_provide_stream_data_buffer(void *, size_t, int, int)`
* Rust source: `rs/fq/src/lib.rs:4092-4142`
* Rust item: `provide_stream_data_buffer`

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

## `picoquic/frames.c:picoquic_skip_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust broadly mirrors frame skipping, but visible differences include PathAck/PathAckEcn passing 0 to parse_ack_header while C calls path-aware skip helpers, and datagram without length consuming all remaining bytes.
* C source: `picoquic/frames.c:7112-7289`
* C signature: `int picoquic_skip_frame(const uint8_t *, size_t, size_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:14578-14849`
* Rust item: `skip_frame`

### C body
```c
{
    const uint8_t *bytes_max = bytes + bytes_maxsize;
    uint8_t first_byte = bytes[0];

    *pure_ack = 1;

    if (PICOQUIC_IN_RANGE(first_byte, picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max)) {
        *pure_ack = 0;
        bytes = picoquic_skip_stream_frame(bytes, bytes_max);
    } else {
        switch (first_byte) {
        case picoquic_frame_type_ack:
            bytes = picoquic_skip_ack_frame(bytes, bytes_max);
            break;
        case picoquic_frame_type_ack_ecn:
            bytes = picoquic_skip_ack_ecn_frame(bytes, bytes_max);
            break;
        case picoquic_frame_type_padding:
            bytes = picoquic_skip_0len_frame(bytes, bytes_max);
            break;
        case picoquic_frame_type_reset_stream:
            bytes = picoquic_skip_reset_stream_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_connection_close: {
            bytes = picoquic_skip_connection_close_frame(bytes, bytes_max);
            *pure_ack = 1;
            break;
        }
        case picoquic_frame_type_application_close: {
            bytes = picoquic_skip_application_close_frame(bytes, bytes_max);
            *pure_ack = 1;
            break;
        }
        case picoquic_frame_type_max_data:
            bytes = picoquic_frames_varint_skip(bytes+1, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_max_stream_data:
            bytes = picoquic_skip_max_stream_data_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_max_streams_bidir:
        case picoquic_frame_type_max_streams_unidir:
            bytes = picoquic_frames_varint_skip(bytes+1, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_ping:
            bytes = picoquic_skip_0len_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_data_blocked:
            bytes = picoquic_frames_varint_skip(bytes+1, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_stream_data_blocked:
            bytes = picoquic_skip_stream_blocked_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_streams_blocked_bidir:
        case picoquic_frame_type_streams_blocked_unidir:
            bytes = picoquic_frames_varint_skip(bytes+1, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_new_connection_id:
            bytes = picoquic_skip_new_connection_id_frame(bytes, bytes_max, 0);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_stop_sending:
            bytes = picoquic_skip_stop_sending_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_path_challenge:
            bytes = picoquic_frames_fixed_skip(bytes+1, bytes_max, challenge_length);
            break;
        case picoquic_frame_type_path_response:
            bytes = picoquic_frames_fixed_skip(bytes+1, bytes_max, challenge_length);
            break;
        case picoquic_frame_type_crypto_hs:
            bytes = picoquic_skip_crypto_hs_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_new_token:
            bytes = picoquic_skip_new_token_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_retire_connection_id:
            bytes = picoquic_skip_retire_connection_id_frame(bytes, bytes_max, 0);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_handshake_done:
            bytes = bytes + 1;
            *pure_ack = 0;
            break;
        case picoquic_frame_type_datagram:
        case picoquic_frame_type_datagram_l:
            bytes = picoquic_skip_datagram_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        case picoquic_frame_type_reset_stream_at:
            bytes = picoquic_skip_reset_stream_at_frame(bytes, bytes_max);
            *pure_ack = 0;
            break;
        default: {
            uint64_t frame_id64;
            const uint8_t * bytes_before_type = bytes;
            if ((bytes = picoquic_frames_varint_decode(bytes, bytes_max, &frame_id64)) != NULL) {
                switch (frame_id64) {
                case picoquic_frame_type_ack_frequency:
                    bytes = picoquic_skip_ack_frequency_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_immediate_ack:
                    bytes = picoquic_skip_immediate_ack_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_time_stamp:
                    bytes = picoquic_skip_time_stamp_frame(bytes, bytes_max);
                    break;
                case picoquic_frame_type_path_ack:
                    bytes = picoquic_skip_ack_frame_maybe_ecn(bytes_before_type, bytes_max, 0, 1);
                    break;
                case picoquic_frame_type_path_ack_ecn:
                    bytes = picoquic_skip_ack_frame_maybe_ecn(bytes_before_type, bytes_max, 1, 1);
                    break;
                case picoquic_frame_type_path_abandon:
                    bytes = picoquic_skip_path_abandon_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_path_backup:
                case picoquic_frame_type_path_available:
                    bytes = picoquic_skip_path_available_or_backup_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_max_path_id:
                    bytes = picoquic_skip_max_path_id_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_paths_blocked:
                    bytes = picoquic_skip_paths_blocked_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_path_cid_blocked:
                    bytes = picoquic_skip_path_cid_blocked_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_bdp:
                    bytes = picoquic_skip_bdp_frame(bytes, bytes_max);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_path_new_connection_id:
                    bytes = picoquic_skip_new_connection_id_frame(bytes_before_type, bytes_max, 1);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_path_retire_connection_id:
                    bytes = picoquic_skip_retire_connection_id_frame(bytes_before_type, bytes_max, 1);
                    *pure_ack = 0;
                    break;
                case picoquic_frame_type_observed_address_v4:
                case picoquic_frame_type_observed_address_v6:
                    bytes = picoquic_skip_observed_address_frame(bytes, bytes_max, frame_id64);
                    *pure_ack = 0;
                    break;
                default:
                    /* Not implemented yet! */
                    bytes = NULL;
                }
            }
            break;
        }
        }
    }

    *consumed = (bytes != NULL) ? bytes_maxsize - (bytes_max - bytes) : bytes_maxsize;

    return bytes == NULL;
}
```

### Rust body
```rust
pub fn skip_frame(bytes: &[u8], bytes_max: usize, consumed: &mut usize, pure_ack: &mut i32) -> i32 {
    let max = bytes_max.min(bytes.len());
    *consumed = 0;
    *pure_ack = 1;
    if max == 0 {
        return -1;
    }

    let mut frame_type = 0;
    let mut tail = match frames_varint_decode(&bytes[..max], &mut frame_type) {
        Some(tail) => tail,
        None => return -1,
    };
    let rest = match frame_type {
        x if x == crate::frames::FrameType::Padding as u64 => {
            *pure_ack = 1;
            let mut off = max - tail.len();
            while off < max && bytes[off] == 0 {
                off += 1;
            }
            &bytes[off..max]
        }
        x if x == crate::frames::FrameType::Ping as u64
            || x == crate::frames::FrameType::HandshakeDone as u64
            || x == crate::frames::FrameType::ImmediateAck as u64 =>
        {
            *pure_ack = 0;
            tail
        }
        x if x == crate::frames::FrameType::Ack as u64
            || x == crate::frames::FrameType::AckEcn as u64
            || x == crate::frames::FrameType::PathAck as u64
            || x == crate::frames::FrameType::PathAckEcn as u64 =>
        {
            let mut num_block = 0;
            let mut path_id = 0;
            let mut largest = 0;
            let mut ack_delay = 0;
            let mut header_len = 0;
            if parse_ack_header(
                &bytes[..max],
                max,
                &mut num_block,
                &mut path_id,
                &mut largest,
                &mut ack_delay,
                &mut header_len,
                0,
            ) != 0
            {
                return -1;
            }
            tail = &bytes[header_len..max];
            let Some(mut t) = skip_n_varints(tail, 1 + (num_block as usize).saturating_mul(2))
            else {
                return -1;
            };
            if frame_type == crate::frames::FrameType::AckEcn as u64
                || frame_type == crate::frames::FrameType::PathAckEcn as u64
            {
                t = match skip_n_varints(t, 3) {
                    Some(t) => t,
                    None => return -1,
                };
            }
            *pure_ack = 1;
            t
        }
        x if x >= crate::frames::FrameType::StreamRangeMin as u64
            && x <= crate::frames::FrameType::StreamRangeMax as u64 =>
        {
            let mut stream_id = 0;
            let mut offset = 0;
            let mut data_length = 0;
            let mut fin = 0;
            let mut header_len = 0;
            if parse_stream_header(
                &bytes[..max],
                max,
                &mut stream_id,
                &mut offset,
                &mut data_length,
                &mut fin,
                &mut header_len,
            ) != 0
                || header_len.saturating_add(data_length) > max
            {
                return -1;
            }
            *pure_ack = 0;
            &bytes[header_len + data_length..max]
        }
        x if x == crate::frames::FrameType::CryptoHs as u64 => {
            let mut ignored = 0;
            tail = match frames_varint_decode(tail, &mut ignored) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            tail = match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            };
            *pure_ack = 0;
            tail
        }
        x if x == crate::frames::FrameType::NewToken as u64 => {
            let mut length = 0;
            tail = match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            };
            *pure_ack = 0;
            tail
        }
        x if x == crate::frames::FrameType::Datagram as u64 => {
            *pure_ack = 0;
            &[]
        }
        x if x == crate::frames::FrameType::DatagramL as u64 => {
            let mut length = 0;
            *pure_ack = 0;
            match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            }
        }
        x if x == crate::frames::FrameType::PathChallenge as u64
            || x == crate::frames::FrameType::PathResponse as u64 =>
        {
            if tail.len() < 8 {
                return -1;
            }
            &tail[8..]
        }
        x if x == crate::frames::FrameType::ResetStream as u64 => {
            *pure_ack = 0;
            match skip_n_varints(tail, 3) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::ResetStreamAt as u64 => {
            *pure_ack = 0;
            match skip_n_varints(tail, 4) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::StopSending as u64
            || x == crate::frames::FrameType::MaxStreamData as u64
            || x == crate::frames::FrameType::StreamDataBlocked as u64 =>
        {
            *pure_ack = 0;
            match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::MaxData as u64
            || x == crate::frames::FrameType::MaxStreamsBidir as u64
            || x == crate::frames::FrameType::MaxStreamsUnidir as u64
            || x == crate::frames::FrameType::DataBlocked as u64
            || x == crate::frames::FrameType::StreamsBlockedBidir as u64
            || x == crate::frames::FrameType::StreamsBlockedUnidir as u64
            || x == crate::frames::FrameType::RetireConnectionId as u64
            || x == crate::frames::FrameType::MaxPathId as u64
            || x == crate::frames::FrameType::PathsBlocked as u64 =>
        {
            *pure_ack = 0;
            match skip_n_varints(tail, 1) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::TimeStamp as u64 => match skip_n_varints(tail, 1) {
            Some(t) => t,
            None => return -1,
        },
        x if x == crate::frames::FrameType::PathRetireConnectionId as u64
            || x == crate::frames::FrameType::PathAbandon as u64
            || x == crate::frames::FrameType::PathAvailable as u64
            || x == crate::frames::FrameType::PathBackup as u64
            || x == crate::frames::FrameType::PathCidBlocked as u64 =>
        {
            *pure_ack = 0;
            match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::NewConnectionId as u64
            || x == crate::frames::FrameType::PathNewConnectionId as u64 =>
        {
            if frame_type == crate::frames::FrameType::PathNewConnectionId as u64 {
                tail = match skip_n_varints(tail, 1) {
                    Some(t) => t,
                    None => return -1,
                };
            }
            *pure_ack = 0;
            tail = match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            };
            let (&cid_len, t) = match tail.split_first() {
                Some(v) => v,
                None => return -1,
            };
            let skip = cid_len as usize + RESET_SECRET_SIZE;
            if t.len() < skip {
                return -1;
            }
            &t[skip..]
        }
        x if x == crate::frames::FrameType::ConnectionClose as u64 => {
            tail = match skip_n_varints(tail, 2) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            }
        }
        x if x == crate::frames::FrameType::ApplicationClose as u64 => {
            tail = match skip_n_varints(tail, 1) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            match frames_varint_decode(tail, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            }
        }
        x if x == crate::frames::FrameType::AckFrequency as u64 => {
            *pure_ack = 0;
            match skip_n_varints(tail, 4) {
                Some(t) => t,
                None => return -1,
            }
        }
        x if x == crate::frames::FrameType::Bdp as u64 => {
            *pure_ack = 0;
            let mut t = match skip_n_varints(tail, 3) {
                Some(t) => t,
                None => return -1,
            };
            let mut length = 0;
            t = match frames_varint_decode(t, &mut length) {
                Some(t) if t.len() >= length as usize => &t[length as usize..],
                _ => return -1,
            };
            t
        }
        x if x == crate::frames::FrameType::ObservedAddressV4 as u64
            || x == crate::frames::FrameType::ObservedAddressV6 as u64 =>
        {
            *pure_ack = 0;
            match parse_observed_address_frame(tail, frame_type) {
                Some((_, rest)) => rest,
                None => return -1,
            }
        }
        _ => return -1,
    };

    *consumed = max - rest.len();
    0
}
```
