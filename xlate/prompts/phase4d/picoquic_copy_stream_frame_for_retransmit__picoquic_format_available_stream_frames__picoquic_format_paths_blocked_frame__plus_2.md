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

## `picoquic/frames.c:picoquic_copy_stream_frame_for_retransmit`
* Phase 4C status: `suspect`
* Phase 4C rationale: C has a visible cnx == NULL path that keeps the frame needed; Rust always performs connection stream lookup and suppresses retransmit when the stream is missing.
* C source: `picoquic/frames.c:2277-2397`
* C signature: `uint8_t * picoquic_copy_stream_frame_for_retransmit(picoquic_cnx_t *, picoquic_packet_t *, uint8_t *, uint8_t *)`
* Rust source: `rs/fq/src/internal.rs:10541-10658`
* Rust item: `copy_stream_frame_for_retransmit`

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

## `picoquic/frames.c:picoquic_format_available_stream_frames`
* Phase 4C status: `suspect`
* Phase 4C rationale: C loops over ready streams by path and priority, handles coalescing, more_stream_data, and priority probing; Rust pops one output stream and requeues it based on local activity only.
* C source: `picoquic/frames.c:2079-2120`
* C signature: `uint8_t * picoquic_format_available_stream_frames(picoquic_cnx_t *, picoquic_path_t *, uint8_t *, uint8_t *, uint64_t, int *, int *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:10388-10424`
* Rust item: `format_available_stream_frames`

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

## `picoquic/frames.c:picoquic_format_paths_blocked_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: The C comment says the frame type is already skipped, yet the C and Rust bodies both visibly encode the frame type and max_path_id; behavior otherwise matches, but the body-visible comment conflicts with the implementation.
* C source: `picoquic/frames.c:6114-6126`
* C signature: `uint8_t * picoquic_format_paths_blocked_frame(uint8_t *, const uint8_t *, uint64_t, int *)`
* Rust source: `rs/fq/src/internal.rs:14204-14220`
* Rust item: `format_paths_blocked_frame`

### C body
```c
{
    /* This code assumes that the frame type is already skipped */
    uint8_t* bytes0 = bytes;
    if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_frame_type_paths_blocked)) == NULL ||
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, max_path_id)) == NULL){
        bytes = bytes0;
        *more_data = 1;
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
        crate::frames::FrameType::PathsBlocked as u64,
    ) || !encode_varint_at(bytes, &mut off, max_path_id)
    {
        *more_data = 1;
        return Some(bytes);
    }
    Some(&mut bytes[off..])
}
```

## `picoquic/frames.c:picoquic_queue_retire_connection_id_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust returns BufferTooSmall on more_data and always queues after successful formatting, while C only queues when consumed bytes are positive and otherwise returns ret.
* C source: `picoquic/frames.c:842-858`
* C signature: `int picoquic_queue_retire_connection_id_frame(picoquic_cnx_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:13510-13539`
* Rust item: `queue_retire_connection_id_frame`

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

## `picoquic/logwriter.c:picoquic_log_path_abandon_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: C skips the frame type then delegates the abandon-frame skip to picoquic_skip_path_abandon_frame; Rust hard-codes exactly three varint skips, so equivalence is not body-visible.
* C source: `picoquic/logwriter.c:449-457`
* C signature: `const uint8_t * picoquic_log_path_abandon_frame(FILE *, const uint8_t *, const uint8_t *)`
* Rust source: `rs/fq/src/binlog.rs:593-601`
* Rust item: `log_path_abandon_frame`

### C body
```c
{
    const uint8_t* bytes_begin = bytes;
    bytes = picoquic_log_varint_skip(bytes, bytes_max); /* frame type as varint */
    bytes = picoquic_skip_path_abandon_frame(bytes, bytes_max); /* skip abandon frame */
    picoquic_binlog_frame(f, bytes_begin, bytes);

    return bytes;
}
```

### Rust body
```rust
fn log_path_abandon_frame<'a>(out: &mut Vec<u8>, bytes_in: &'a [u8]) -> Option<&'a [u8]> {
    let bytes_begin = bytes_in;
    let mut bytes = frames_varint_skip(bytes_in)?;
    bytes = frames_varint_skip(bytes)?;
    bytes = frames_varint_skip(bytes)?;
    let consumed = bytes_begin.len() - bytes.len();
    append_frame(out, &bytes_begin[..consumed]);
    Some(bytes)
}
```
