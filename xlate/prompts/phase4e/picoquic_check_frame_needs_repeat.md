# Phase 4E repair confirmed translation mismatches

You are repairing Phase 4D `needs_fix` entries.  Phase 4D
already performed deeper classification and concluded that
these Rust translations need repair.

Rules:

* Edit Rust only.  Do not edit C sources.
* Keep edits limited to the owned Rust file(s) for this batch
  unless a directly related helper in `rs/fq/` must change.
* Preserve safe, idiomatic Rust and existing public API shape
  unless the current shape cannot express the C behavior.
* Do not replace code with stubs, placeholders, fabricated
  defaults, or weaker behavior.
* If deeper repair inspection proves Phase 4D was mistaken,
  report outcome `ok` and do not edit source.
* The driver will run a separate read-only re-triage before
  recording any `fixed` or `ok` result as resolved.
* Report `blocked` only with a concrete human-actionable
  reason.

Owned Rust file(s): `rs/fq/src/internal.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/frames.c:picoquic_check_frame_needs_repeat`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust implements only a small subset of repeat decisions and does not visibly handle most C cases such as max data, stream data blocked, connection ID, reset, stop sending, crypto, datagram, token, or multipath frames.
* Phase 4D analysis: Deeper context confirms a real mismatch: Rust implements only a shallow subset and omits C's stateful repeat decisions for streams, flow control, datagrams, token/crypto/CID/reset/stop, ack-frequency, and multipath frames.
* Phase 4D fix note: Reimplement check_frame_needs_repeat against the C logic, including stream ack/reset checks, flow-control and blocked-frame cases, path/datagram/token/crypto/CID/reset/stop/multipath handling, and correct output flag semantics.
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
    pub fn check_frame_needs_repeat(
        &mut self,
        bytes: &[u8],
        bytes_max: usize,
        p_type: PacketType,
        no_need_to_repeat: &mut i32,
        do_not_detect_spurious: &mut i32,
        is_preemptive_needed: &mut i32,
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
```
