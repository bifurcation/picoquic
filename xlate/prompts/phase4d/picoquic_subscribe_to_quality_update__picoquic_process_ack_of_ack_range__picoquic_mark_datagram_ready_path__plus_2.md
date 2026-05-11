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

## `picoquic/quicctx.c:picoquic_subscribe_to_quality_update`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets is_path_quality_update_requested = 1; Rust sets the deltas and updates each path but has no visible equivalent flag assignment.
* C source: `picoquic/quicctx.c:2749-2760`
* C signature: `void picoquic_subscribe_to_quality_update(picoquic_cnx_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:2737-2743`
* Rust item: `subscribe_to_quality_update`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->pacing_rate_update_delta = pacing_rate_delta;
    cnx->rtt_update_delta = rtt_delta;
    cnx->is_path_quality_update_requested = 1;

    for (int i = 0; i < cnx->nb_paths; i++) {
        picoquic_subscribe_to_quality_update_per_path_context(cnx->path[i],
            pacing_rate_delta, rtt_delta);
    }
}
```

### Rust body
```rust
    pub fn subscribe_to_quality_update(&mut self, pacing_rate_delta: u64, rtt_delta: Duration) {
        self.rtt_update_delta = rtt_delta;
        self.pacing_rate_update_delta = pacing_rate_delta;
        for path in &mut self.paths {
            path.subscribe_to_quality_update_per_path_context(pacing_rate_delta, rtt_delta);
        }
    }
```

## `picoquic/sacks.c:picoquic_process_ack_of_ack_range`
* Phase 4C status: `suspect`
* Phase 4C rationale: When deleting a matching non-highest range, C returns previous after deletion, while Rust removes it and returns the next token.
* C source: `picoquic/sacks.c:347-380`
* C signature: `picoquic_sack_item_t * picoquic_process_ack_of_ack_range(picoquic_sack_list_t *, picoquic_sack_item_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:8304-8354`
* Rust item: `process_ack_of_ack_range`

### C body
```c
{
    /* Find if the range is inside the tree */
    previous = picoquic_sack_find_range_below_number(sack_list, NULL, start_of_range);

    if (previous != NULL && previous->start_of_sack_range == start_of_range){
        picoquic_sack_item_t* next = picoquic_sack_item_value(picosplay_next(&previous->node));
        if (next == NULL) {
            /* Matching the highest range, which shall not be deleted */
            if (end_of_range < previous->end_of_sack_range) {
                previous->start_of_sack_range = end_of_range + 1;
            }
            else {
                previous->start_of_sack_range = previous->end_of_sack_range;
            }
        }
        else if (previous->end_of_sack_range == end_of_range) {
            /* Matching ACK */
            if (sack_list->horizon_delay > 0) {
                for (int r = 0; r < 2; r++) {
                    if (previous->nb_times_sent[r] < PICOQUIC_MAX_ACK_RANGE_REPEAT) {
                        sack_list->rc[r].range_counts[previous->nb_times_sent[r]] -= 1;
                        previous->nb_times_sent[r] = PICOQUIC_MAX_ACK_RANGE_REPEAT;
                    }
                }
            } else {
                picoquic_sack_delete_item(sack_list, previous);
            }
        }
    }

    return previous;
}
```

### Rust body
```rust
    ) -> Option<SackItemToken> {
        let st = self.ack_tree.find(&start_of_range)?;
        let token = self.ack_tree.get(st).copied()?;
        let item = self.sack_items.get(token)?;
        if item.start_of_sack_range != start_of_range {
            return Some(token);
        }

        let next = self.ack_tree.next(st);
        if next.is_none() {
            let (_, tok) = self.ack_tree.remove(st)?;
            let key = {
                let item = self.sack_items.get_mut(tok)?;
                item.start_of_sack_range = if end_of_range < item.end_of_sack_range {
                    end_of_range + 1
                } else {
                    item.end_of_sack_range
                };
                item.start_of_sack_range
            };
            let (new_st, _) = self.ack_tree.insert(key, tok).ok()?;
            if let Some(inserted) = self.sack_items.get_mut(tok) {
                inserted.ack_tree_membership = Some(new_st);
            }
            Some(tok)
        } else if item.end_of_sack_range == end_of_range {
            if self.horizon_delay > 0 {
                if let Some(item) = self.sack_items.get_mut(token) {
                    for r in 0..2 {
                        let sent = item.nb_times_sent[r];
                        if sent >= 0 && (sent as usize) < MAX_ACK_RANGE_REPEAT {
                            self.rc[r].range_counts[sent as usize] -= 1;
                            item.nb_times_sent[r] = MAX_ACK_RANGE_REPEAT as i32;
                        }
                    }
                }
                Some(token)
            } else {
                self.ack_tree.remove(st);
                self.sack_items.remove(token);
                next.and_then(|next_st| self.ack_tree.get(next_st).copied())
            }
        } else {
            Some(token)
        }
    }
```

## `picoquic/sender.c:picoquic_mark_datagram_ready_path`
* Phase 4C status: `suspect`
* Phase 4C rationale: C checks transition from not-ready to ready, errors if max_datagram_frame_size is zero, and reinserts by wake time; Rust only sets the path flag or errors if the path is absent.
* C source: `picoquic/sender.c:124-144`
* C signature: `int picoquic_mark_datagram_ready_path(picoquic_cnx_t *, uint64_t, int)`
* Rust source: `rs/fq/src/lib.rs:4391-4406`
* Rust item: `mark_datagram_ready_path`

### C body
```c
{
    int ret = 0;
    int path_id;
    PICOQUIC_THREAD_CHECK(cnx->quic); 
    if ((path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id)) >= 0) {
        int was_ready = cnx->path[path_id]->is_datagram_ready;
        cnx->path[path_id]->is_datagram_ready = is_path_ready;
        if (!was_ready && is_path_ready) {
            if (cnx->remote_parameters.max_datagram_frame_size == 0) {
                ret = -1;
            }
            else {
                picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));
            }
        }
    } else {
        ret = -1;
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        if let Some(path) = self
            .paths
            .iter_mut()
            .find(|p| p.unique_path_id == unique_path_id)
        {
            path.is_datagram_ready = is_path_ready;
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }
```

## `picoquic/sender.c:picoquic_prepare_packet_0rtt`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust passes path_x to format_available_stream_frames where C passes NULL, and Rust adds a send_buffer_max <= checksum_overhead early-zero condition not present in C.
* C source: `picoquic/sender.c:1649-1743`
* C signature: `int picoquic_prepare_packet_0rtt(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, int, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:17856-17994`
* Rust item: `prepare_packet_0rtt`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream = NULL;
    picoquic_packet_type_enum packet_type = picoquic_packet_0rtt_protected;
    size_t header_length = 0;
    uint8_t* bytes = packet->bytes;
    size_t length = 0;
    size_t checksum_overhead = picoquic_aead_get_checksum_length(cnx->crypto_context[1].aead_encrypt);
    uint8_t* bytes_max;
    uint8_t* bytes_next;
    int more_data = 0;
    int is_pure_ack = 1;
    int stream_tried_and_failed = 0;

    send_buffer_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;
    if (path_x->bytes_in_transit + send_buffer_max > PICOQUIC_DEFAULT_0RTT_WINDOW) {
        if (path_x->bytes_in_transit > PICOQUIC_DEFAULT_0RTT_WINDOW) {
            send_buffer_max = 0;
        }
        else {
            send_buffer_max = (size_t)PICOQUIC_DEFAULT_0RTT_WINDOW - (size_t)path_x->bytes_in_transit;
        }
    }
    bytes_max = bytes + send_buffer_max - checksum_overhead;

    stream = picoquic_find_ready_stream(cnx);
    length = picoquic_predict_packet_header_length(cnx, packet_type, &cnx->pkt_ctx[picoquic_packet_context_application]);
    packet->ptype = picoquic_packet_0rtt_protected;
    packet->offset = length;
    header_length = length;
    packet->pc = picoquic_packet_context_application;
    packet->sequence_number = cnx->pkt_ctx[picoquic_packet_context_application].send_sequence;
    packet->send_time = current_time;
    packet->send_path = path_x;
    packet->checksum_overhead = checksum_overhead;
    bytes_next = bytes + length;


    
    /* Consider sending 0-RTT */
    if ((stream == NULL && cnx->first_misc_frame == NULL && padding_required == 0) || 
        send_buffer_max < PICOQUIC_MIN_SEGMENT_SIZE) {
        length = 0;
    } else {
        /* If present, send misc frame */
        bytes_next = picoquic_format_misc_frames_in_context(cnx, bytes_next, bytes_max,
            &more_data, &is_pure_ack, picoquic_packet_context_application);

        /* We assume that if BDP data is associated with the zero RTT ticket, it can be sent */
        /* Encode the bdp frame */
        if (cnx->local_parameters.enable_bdp_frame) {
            bytes_next = picoquic_format_bdp_frame(cnx, bytes_next, bytes_max, path_x, &more_data, &is_pure_ack);
        }

        /* Encode the stream frame, or frames */
        bytes_next = picoquic_format_available_stream_frames(cnx, NULL, bytes_next, bytes_max, UINT64_MAX,
            &more_data, &is_pure_ack, &stream_tried_and_failed, &ret);

        length = bytes_next - bytes;

        if (more_data) {
            *next_wake_time = current_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }

        if (stream_tried_and_failed) {
            path_x->last_sender_limited_time = current_time;
        }

        /* Add padding if required */
        if (padding_required) {
            length = picoquic_pad_to_target_length(bytes, length, send_buffer_max - checksum_overhead);
        }
    }

    picoquic_finalize_and_protect_packet(cnx, packet,
        ret, length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_max,
        path_x, current_time);

    if (length > 0) {
        /* Accounting of zero rtt packets sent */
        cnx->nb_zero_rtt_sent++;
    }

    /* the reinsertion by wake up time will happen in the calling function */

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut ret = 0;
        let packet_type = PacketType::ZeroRttProtected;
        let checksum_overhead = self.get_checksum_length(Epoch::ZeroRtt);
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let mut stream_tried_and_failed = 0;

        send_buffer_max = send_buffer_max.min(path_x.send_mtu);
        if path_x
            .bytes_in_transit
            .saturating_add(send_buffer_max as u64)
            > DEFAULT_0RTT_WINDOW as u64
        {
            send_buffer_max = if path_x.bytes_in_transit > DEFAULT_0RTT_WINDOW as u64 {
                0
            } else {
                DEFAULT_0RTT_WINDOW.saturating_sub(path_x.bytes_in_transit as usize)
            };
        }
        let header_length =
            self.predict_packet_header_length_for_pc(packet_type, PacketContext::Application);
        let mut length = header_length;
        packet.packet_type = packet_type;
        packet.offset = header_length;
        packet.packet_context = PacketContext::Application;
        packet.sequence_number = self.pkt_ctx[PacketContext::Application as usize].send_sequence;
        packet.send_time = current_time;
        packet.send_path = Some(Self::path_token_for_path(path_x));
        packet.checksum_overhead = checksum_overhead;

        if (self.find_ready_stream().is_none()
            && self.misc_frames.is_empty()
            && padding_required == 0)
            || send_buffer_max < MIN_SEGMENT_SIZE
            || send_buffer_max <= checksum_overhead
        {
            length = 0;
        } else {
            let bytes_limit = send_buffer_max
                .saturating_sub(checksum_overhead)
                .min(packet.bytes.len());
            if length <= bytes_limit {
                let mut offset = length;
                let tail_len = {
                    let tail = &mut packet.bytes[offset..bytes_limit];
                    match format_misc_frames_in_context(
                        self,
                        tail,
                        &mut more_data,
                        &mut is_pure_ack,
                        PacketContext::Application,
                    ) {
                        Some(next) => next.len(),
                        None => {
                            ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                            tail.len()
                        }
                    }
                };
                offset = bytes_limit.saturating_sub(tail_len);
                if ret == 0 && self.local_parameters.enable_bdp_frame {
                    let tail_len = {
                        let tail = &mut packet.bytes[offset..bytes_limit];
                        match format_bdp_frame(self, tail, path_x, &mut more_data, &mut is_pure_ack)
                        {
                            Some(next) => next.len(),
                            None => {
                                ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                                tail.len()
                            }
                        }
                    };
                    offset = bytes_limit.saturating_sub(tail_len);
                }
                if ret == 0 {
                    let tail_len = {
                        let tail = &mut packet.bytes[offset..bytes_limit];
                        match format_available_stream_frames(
                            self,
                            path_x,
                            tail,
                            u64::MAX,
                            &mut more_data,
                            &mut is_pure_ack,
                            &mut stream_tried_and_failed,
                            &mut ret,
                        ) {
                            Some(next) => next.len(),
                            None => {
                                ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                                tail.len()
                            }
                        }
                    };
                    offset = bytes_limit.saturating_sub(tail_len);
                }
                length = offset;
            }
            self.note_more_data(more_data, next_wake_time, current_time);
            if stream_tried_and_failed != 0 {
                path_x.last_sender_limited_time = current_time;
            }
            if padding_required != 0 && length > 0 {
                length = pad_to_target_length(
                    &mut packet.bytes,
                    length,
                    send_buffer_max.saturating_sub(checksum_overhead),
                );
            }
        }

        self.finalize_and_protect_packet(
            packet,
            ret,
            length,
            header_length,
            checksum_overhead,
            send_length,
            send_buffer,
            send_buffer_max,
            path_x,
            current_time,
        );
        if length > 0 {
            self.nb_zero_rtt_sent = self.nb_zero_rtt_sent.saturating_add(1);
        }
        ret
    }
```

## `picoquic/sender.c:picoquic_reset_stream_at`
* Phase 4C status: `suspect`
* Phase 4C rationale: C always reinserts by wake time after the checks, while Rust only enqueues conditionally when reset_requested changes.
* C source: `picoquic/sender.c:379-407`
* C signature: `int picoquic_reset_stream_at(picoquic_cnx_t *, uint64_t, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:4230-4265`
* Rust item: `reset_stream_at`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream = NULL;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (reliable_size > 0 && !cnx->is_reset_stream_at_enabled) {
        ret = PICOQUIC_ERROR_ILLEGAL_TRANSPORT_EXTENSION;
    }
    else if ((stream = picoquic_find_stream(cnx, stream_id)) == NULL) {
        ret = PICOQUIC_ERROR_INVALID_STREAM_ID;
    }
    else {
        stream->app_stream_ctx = NULL;
        if (stream->fin_sent && picoquic_check_sack_list(&stream->sack_list, 0, stream->fin_offset) == 0) {
            ret = PICOQUIC_ERROR_STREAM_ALREADY_CLOSED;
        }
        else if (!stream->reset_requested) {
            stream->local_error = local_stream_error;
            stream->reset_requested = 1;
            stream->reliable_size = reliable_size;
        }
    }

    picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        if reliable_size > 0 && !self.is_reset_stream_at_enabled {
            return Err(Error::Protocol(
                InternalError::IllegalTransportExtension as u64,
            ));
        }
        let stream_token = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let mut should_enqueue = false;
        {
            let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;
            stream.app_stream_ctx = None;
            if stream.fin_sent && !stream.sack_list.check(0, stream.fin_offset) {
                return Err(Error::Protocol(InternalError::StreamAlreadyClosed as u64));
            }
            if !stream.reset_requested {
                stream.local_error = local_stream_error;
                stream.reset_requested = true;
                stream.reliable_size = reliable_size;
                if !stream.is_output_stream {
                    stream.is_output_stream = true;
                    should_enqueue = true;
                }
            }
        }
        if should_enqueue {
            enqueue_output_stream_token(self, stream_token);
        }
        Ok(())
    }
```
