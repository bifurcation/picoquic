# Phase 4E repair confirmation

This is a read-only re-triage after a Phase 4E repair or
repair-level `ok` claim.  Do not edit files.

For each entry, inspect directly relevant C and Rust context
and decide whether the current Rust translation is now
acceptable.

Report:

* `ok` when the current Rust behavior is acceptable.
* `needs_fix` when a real mismatch remains.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short confirmation conclusion","fix_summary":"remaining mismatch if any, or empty","files_changed":[],"verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/frames.c:picoquic_copy_stream_frame_for_retransmit`
* Phase 4C status: `suspect`
* Phase 4C rationale: C has a visible cnx == NULL path that keeps the frame needed; Rust always performs connection stream lookup and suppresses retransmit when the stream is missing.
* Prior Phase 4D analysis: C's nullable cnx is meaningful: the C unit test passes NULL to exercise pure frame copying, which bypasses stream lookup. Rust requires a Connection and treats a missing stream as not needed, so the no-connection/missing-stream behavior is not preserved.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Rust now represents C's nullable connection path; no-connection retransmit copying skips stream lookup instead of suppressing missing-stream data.
* Phase 4E fix summary: Added StreamFrameRetransmitContext with Connection and NoConnection modes, kept production caller using a real connection, and added owned-file unit coverage for pure no-connection copy behavior.
* C source: `picoquic/frames.c:2277-2397`
* C signature: `uint8_t * picoquic_copy_stream_frame_for_retransmit(picoquic_cnx_t *, picoquic_packet_t *, uint8_t *, uint8_t *)`
* Current Rust source: `rs/fq/src/internal.rs:11120-11226`
* Current Rust item: `copy_stream_frame_for_retransmit`

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

### Current Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut connection = connection.into();
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

    let is_needed = connection.is_stream_frame_needed(stream_id, offset, data_available, fin);

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
