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

## `picoquic/frames.c:picoquic_check_frame_needs_repeat`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust implements only a small subset of repeat decisions and does not visibly handle most C cases such as max data, stream data blocked, connection ID, reset, stop sending, crypto, datagram, token, or multipath frames.
* Prior Phase 4D analysis: Deeper context confirms a real mismatch: Rust implements only a shallow subset and omits C's stateful repeat decisions for streams, flow control, datagrams, token/crypto/CID/reset/stop, ack-frequency, and multipath frames.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Rust retransmission decision logic now mirrors the C stateful cases instead of only handling a shallow subset.
* Phase 4E fix summary: Expanded check_frame_needs_repeat with stream SACK/reset handling, flow-control and blocked-frame checks, datagram/token/crypto/CID/reset/stop decisions, ack-frequency and multipath path-frame repeat handling, and preserved C-style output flag semantics.
* C source: `picoquic/frames.c:3412-3674`
* C signature: `int picoquic_check_frame_needs_repeat(picoquic_cnx_t *, const uint8_t *, size_t, picoquic_packet_type_enum, int *, int *, int *)`
* Current Rust source: `rs/fq/src/internal.rs:10657-10913`
* Current Rust item: `check_frame_needs_repeat`

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

### Current Rust body
```rust
    ) -> i32 {
        *no_need_to_repeat = 0;
        let max = bytes_max.min(bytes.len());
        if max == 0 {
            *no_need_to_repeat = 1;
            return 0;
        }
        let bytes = &bytes[..max];
        if bytes[0] >= crate::frames::FrameType::StreamRangeMin as u8
            && bytes[0] <= crate::frames::FrameType::StreamRangeMax as u8
        {
            let mut fin = 0;
            let mut data_length = 0;
            let mut stream_id = 0;
            let mut offset = 0;
            let mut consumed = 0;
            let ret = parse_stream_header(
                bytes,
                max,
                &mut stream_id,
                &mut offset,
                &mut data_length,
                &mut fin,
                &mut consumed,
            );
            if ret == 0 {
                match self
                    .find_stream(stream_id)
                    .and_then(|token| self.streams.get(token))
                {
                    None => *no_need_to_repeat = 1,
                    Some(stream) => {
                        if stream.reset_sent {
                            *no_need_to_repeat = 1;
                        } else {
                            let last = offset
                                .wrapping_add(data_length as u64)
                                .wrapping_sub(u64::from(fin == 0));
                            *no_need_to_repeat =
                                picoquic_check_sack_list(&stream.sack_list, offset, last);
                        }
                        if stream.fin_sent {
                            *is_preemptive_needed |= 1;
                        }
                    }
                }
            }
            return ret;
        }

        match bytes[0] {
            x if x == crate::frames::FrameType::MaxData as u8 => {
                let mut maxdata = 0;
                if Self::decode_varint_after_one_byte_type(bytes, &mut maxdata).is_none()
                    || maxdata < self.maxdata_local
                    || maxdata <= self.maxdata_local_acked
                {
                    *no_need_to_repeat = 1;
                }
            }
            x if x == crate::frames::FrameType::MaxStreamData as u8 => {
                let mut stream_id = 0;
                let mut maxdata = 0;
                let stream_state = Self::decode_varint_after_one_byte_type(bytes, &mut stream_id)
                    .and_then(|tail| frames_varint_decode(tail, &mut maxdata))
                    .and_then(|_| self.find_stream(stream_id))
                    .and_then(|token| self.streams.get(token));
                match stream_state {
                    Some(stream)
                        if !(stream.fin_received
                            || stream.reset_received
                            || stream.stop_sending_sent)
                            && maxdata >= stream.maxdata_local
                            && maxdata > stream.maxdata_local_acked => {}
                    _ => *no_need_to_repeat = 1,
                }
            }
            x if x == crate::frames::FrameType::MaxStreamsBidir as u8
                || x == crate::frames::FrameType::MaxStreamsUnidir as u8 =>
            {
                return self.check_max_streams_frame_needs_repeat(bytes, no_need_to_repeat);
            }
            x if x == crate::frames::FrameType::DataBlocked as u8 => {
                let mut maxdata = 0;
                if Self::decode_varint_after_one_byte_type(bytes, &mut maxdata).is_none()
                    || maxdata < self.maxdata_remote
                {
                    *no_need_to_repeat = 1;
                } else {
                    *no_need_to_repeat = i32::from(!self.sent_blocked_frame);
                }
            }
            x if x == crate::frames::FrameType::StreamsBlockedBidir as u8 => {
                let mut max_stream_rank = 0;
                if Self::decode_varint_after_one_byte_type(bytes, &mut max_stream_rank).is_none()
                    || self.max_stream_id_bidir_remote
                        > self.stream_id_from_rank_for_local_role(max_stream_rank, false)
                {
                    *no_need_to_repeat = 1;
                } else {
                    *no_need_to_repeat = i32::from(!self.stream_blocked_bidir_sent);
                }
            }
            x if x == crate::frames::FrameType::StreamsBlockedUnidir as u8 => {
                let mut max_stream_rank = 0;
                if Self::decode_varint_after_one_byte_type(bytes, &mut max_stream_rank).is_none()
                    || self.max_stream_id_unidir_remote
                        > self.stream_id_from_rank_for_local_role(max_stream_rank, true)
                {
                    *no_need_to_repeat = 1;
                } else {
                    *no_need_to_repeat = i32::from(!self.stream_blocked_unidir_sent);
                }
            }
            x if x == crate::frames::FrameType::StreamDataBlocked as u8 => {
                let mut stream_id = 0;
                let mut maxdata = 0;
                let stream_state = Self::decode_varint_after_one_byte_type(bytes, &mut stream_id)
                    .and_then(|tail| frames_varint_decode(tail, &mut maxdata))
                    .and_then(|_| self.find_stream(stream_id))
                    .and_then(|token| self.streams.get(token));
                match stream_state {
                    Some(stream)
                        if !(stream.fin_requested || stream.fin_sent || stream.reset_sent)
                            && maxdata >= stream.maxdata_remote
                            && stream.stream_data_blocked_sent => {}
                    _ => *no_need_to_repeat = 1,
                }
            }
            x if x == crate::frames::FrameType::PathChallenge as u8 => {
                *no_need_to_repeat = 1;
            }
            x if x == crate::frames::FrameType::PathResponse as u8 => {
                *no_need_to_repeat = i32::from(self.should_repeat_path_response_frame(bytes, max));
            }
            x if x == crate::frames::FrameType::Datagram as u8
                || x == crate::frames::FrameType::DatagramL as u8 =>
            {
                *no_need_to_repeat = 1;
                *do_not_detect_spurious = 0;
            }
            x if x == crate::frames::FrameType::HandshakeDone as u8 => {
                if self.is_handshake_done_acked {
                    *no_need_to_repeat = 1;
                }
            }
            x if x == crate::frames::FrameType::NewToken as u8 => {
                if self.is_new_token_acked {
                    *no_need_to_repeat = 1;
                }
            }
            x if x == crate::frames::FrameType::CryptoHs as u8 => {
                return self.check_crypto_frame_needs_repeat(bytes, p_type, no_need_to_repeat);
            }
            x if x == crate::frames::FrameType::NewConnectionId as u8 => {
                return self.check_new_cid_needs_repeat(bytes, false, no_need_to_repeat);
            }
            x if x == crate::frames::FrameType::RetireConnectionId as u8 => {
                return self.check_retire_connection_id_needs_repeat(
                    bytes,
                    no_need_to_repeat,
                    false,
                );
            }
            x if x == crate::frames::FrameType::ResetStream as u8 => {
                return self.check_reset_stream_needs_repeat(bytes, no_need_to_repeat);
            }
            x if x == crate::frames::FrameType::StopSending as u8 => {
                return self.check_stop_sending_needs_repeat(bytes, no_need_to_repeat);
            }
            x if x == crate::frames::FrameType::ResetStreamAt as u8 => {
                return self.check_reset_stream_at_needs_repeat(bytes, no_need_to_repeat);
            }
            _ => {
                let mut frame_id64 = 0;
                *no_need_to_repeat = 0;
                if let Some(tail) = frames_varint_decode(bytes, &mut frame_id64) {
                    match frame_id64 {
                        x if x == crate::frames::FrameType::AckFrequency as u64 => {
                            let mut seq = 0;
                            let mut packets = 0;
                            let mut microsec = 0;
                            let mut ignore_order = 0;
                            let mut reordering_threshold = 0;
                            if parse_ack_frequency_frame(
                                tail,
                                &mut seq,
                                &mut packets,
                                &mut microsec,
                                &mut ignore_order,
                                &mut reordering_threshold,
                            )
                            .is_none()
                            {
                                return -1;
                            } else if seq == self.ack_frequency_sequence_local {
                                *no_need_to_repeat = 1;
                            }
                        }
                        x if x == crate::frames::FrameType::ImmediateAck as u64 => {
                            *no_need_to_repeat = 0;
                        }
                        x if x == crate::frames::FrameType::PathAck as u64
                            || x == crate::frames::FrameType::PathAckEcn as u64
                            || x == crate::frames::FrameType::TimeStamp as u64 =>
                        {
                            *no_need_to_repeat = 1;
                        }
                        x if x == crate::frames::FrameType::PathAbandon as u64 => {
                            *no_need_to_repeat = 0;
                        }
                        x if x == crate::frames::FrameType::PathBackup as u64
                            || x == crate::frames::FrameType::PathAvailable as u64 =>
                        {
                            self.path_available_or_backup_frame_need_repeat(
                                tail,
                                no_need_to_repeat,
                            );
                        }
                        x if x == crate::frames::FrameType::MaxPathId as u64 => {
                            self.max_path_id_frame_needs_repeat(tail, no_need_to_repeat);
                        }
                        x if x == crate::frames::FrameType::PathsBlocked as u64 => {
                            self.paths_blocked_frame_needs_repeat(tail, no_need_to_repeat);
                        }
                        x if x == crate::frames::FrameType::PathCidBlocked as u64 => {
                            self.path_cid_blocked_frame_needs_repeat(tail, no_need_to_repeat);
                        }
                        x if x == crate::frames::FrameType::PathNewConnectionId as u64 => {
                            return self.check_new_cid_needs_repeat(bytes, true, no_need_to_repeat);
                        }
                        x if x == crate::frames::FrameType::PathRetireConnectionId as u64 => {
                            return self.check_retire_connection_id_needs_repeat(
                                bytes,
                                no_need_to_repeat,
                                true,
                            );
                        }
                        x if x == crate::frames::FrameType::ObservedAddressV4 as u64
                            || x == crate::frames::FrameType::ObservedAddressV6 as u64 => {}
                        _ => {
                            *no_need_to_repeat = 0;
                        }
                    }
                }
            }
        }
        0
    }
```
