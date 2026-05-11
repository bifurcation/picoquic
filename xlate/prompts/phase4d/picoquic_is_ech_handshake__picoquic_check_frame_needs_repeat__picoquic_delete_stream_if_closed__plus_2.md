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

## `picoquic/ech.c:picoquic_is_ech_handshake`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns whether TLS is in an ECH handshake; Rust body returns retry config bytes.
* C source: `picoquic/ech.c:415-422`
* C signature: `int picoquic_is_ech_handshake(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4722-4733`
* Rust item: `is_ech_handshake`

### C body
```c
{
    picoquic_tls_ctx_t* tls_ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return ptls_is_ech_handshake(tls_ctx->tls, NULL, NULL, NULL);
}
```

### Rust body
```rust
    pub fn ech_retry_config(&self) -> &[u8] {
        self.tls_ctx.as_ref().map_or(&[], |s| s.retry_configs())
    }
```

## `picoquic/frames.c:picoquic_check_frame_needs_repeat`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust implements only a small subset of repeat decisions and does not visibly handle most C cases such as max data, stream data blocked, connection ID, reset, stop sending, crypto, datagram, token, or multipath frames.
* C source: `picoquic/frames.c:3412-3674`
* C signature: `int picoquic_check_frame_needs_repeat(picoquic_cnx_t *, const uint8_t *, size_t, picoquic_packet_type_enum, int *, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:10337-10385`
* Rust item: `check_frame_needs_repeat`

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

## `picoquic/frames.c:picoquic_delete_stream_if_closed`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust only marks the stream closed and sets ret; the C body also updates max stream ID, conditionally deletes the stream, and returns ret.
* C source: `picoquic/frames.c:134-156`
* C signature: `int picoquic_delete_stream_if_closed(picoquic_cnx_t *, picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/internal.rs:9509-9514`
* Rust item: `delete_stream_if_closed`

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

## `picoquic/frames.c:picoquic_format_first_datagram_frame`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C tries a DatagramL-to-Datagram size squeeze with padding and deletes an oversized first datagram when is_first_in_packet; Rust lacks both behaviors and simply requeues on insufficient space.
* C source: `picoquic/frames.c:5342-5379`
* C signature: `uint8_t * picoquic_format_first_datagram_frame(picoquic_cnx_t *, uint8_t *, uint8_t *, int, int *, int *)`
* Rust source: `rs/fq/src/internal.rs:13912-13947`
* Rust item: `format_first_datagram_frame`

### C body
```c
{
    if (bytes + cnx->first_datagram->length <= bytes_max) {
        bytes = picoquic_format_first_misc_or_dg_frame(bytes, bytes_max, more_data, is_pure_ack,
            cnx->first_datagram, &cnx->first_datagram, &cnx->last_datagram);
    } else {
        int is_sent = 0;
        uint8_t* frame_content = ((uint8_t*)cnx->first_datagram) + sizeof(picoquic_misc_frame_header_t);

        if (frame_content[0] == picoquic_frame_type_datagram_l) {
            /* It might be possible to squeeze the size by removing the length */
            size_t header_length = 1 + picoquic_varint_skip(frame_content + 1);
            size_t data_length = cnx->first_datagram->length - header_length;
            size_t min_length = data_length + 1;

            if (bytes + min_length <= bytes_max) {
                while (bytes + min_length < bytes_max) {
                    *bytes++ = picoquic_frame_type_padding;
                }
                *bytes++ = picoquic_frame_type_datagram;
                memcpy(bytes, frame_content + header_length, data_length);
                bytes += data_length;
                picoquic_delete_misc_or_dg(&cnx->first_datagram, &cnx->last_datagram, cnx->first_datagram);
                is_sent= 1;
            }
        }
        if (!is_sent && is_first_in_packet) {
            picoquic_log_app_message(cnx, "Deleting datagram length %zu, larger than %zu, MTU %zu",
                cnx->first_datagram->length, bytes_max - bytes, cnx->path[0]->send_mtu);
            picoquic_delete_misc_or_dg(&cnx->first_datagram, &cnx->last_datagram, cnx->first_datagram);
        }

        *more_data |= (cnx->first_datagram != NULL);
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let frame = connection.datagrams.pop_front()?;
    if frame.bytes.first().is_some_and(|b| {
        *b == crate::frames::FrameType::Datagram as u8
            || *b == crate::frames::FrameType::DatagramL as u8
    }) {
        let len = frame.bytes.len();
        if bytes.len() < len {
            connection.datagrams.push_front(frame);
            *more_data = 1;
            return Some(bytes);
        }
        bytes[..len].copy_from_slice(&frame.bytes);
        *is_pure_ack = 0;
        return Some(&mut bytes[len..]);
    }
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, crate::frames::FrameType::DatagramL as u64)
        || !encode_varint_at(bytes, &mut off, frame.bytes.len() as u64)
        || bytes.len() < off + frame.bytes.len()
    {
        connection.datagrams.push_front(frame);
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + frame.bytes.len()].copy_from_slice(&frame.bytes);
    off += frame.bytes.len();
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## `picoquic/frames.c:picoquic_parse_stream_header`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: On length-present data past packet end, C returns an error while preserving consumed at the header end and data_length as decoded; Rust sets consumed to max and data_length to 0.
* C source: `picoquic/frames.c:1204-1260`
* C signature: `int picoquic_parse_stream_header(const uint8_t *, size_t, uint64_t *, uint64_t *, size_t *, int *, size_t *)`
* Rust source: `rs/fq/src/internal.rs:12280-12338`
* Rust item: `parse_stream_header`

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
