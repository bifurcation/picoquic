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

## `picoquic/quicctx.c:picoquic_subscribe_to_quality_update_per_path`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust finds the path and subscribes or errors like C, but the C body also sets is_path_quality_update_requested before path lookup; Rust body does not show that state update.
* C source: `picoquic/quicctx.c:2729-2747`
* C signature: `int picoquic_subscribe_to_quality_update_per_path(picoquic_cnx_t *, uint64_t, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:2716-2732`
* Rust item: `subscribe_to_quality_update_per_path`

### C body
```c
{
    int ret = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    cnx->is_path_quality_update_requested = 1;

    int path_id = picoquic_get_path_id_from_unique(cnx, unique_path_id);
    if (path_id >= 0) {
        picoquic_subscribe_to_quality_update_per_path_context(cnx->path[path_id],
            pacing_rate_delta, rtt_delta);
    }
    else {
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
            path.subscribe_to_quality_update_per_path_context(pacing_rate_delta, rtt_delta);
            Ok(())
        } else {
            Err(Error::InvalidArgument)
        }
    }
```

## `picoquic/sacks.c:picoquic_update_sack_list`
* Phase 4C status: `suspect`
* Phase 4C rationale: C sets ret to the insert result when creating a new item, while Rust always sets ret = 0 after insert_item succeeds.
* C source: `picoquic/sacks.c:197-256`
* C signature: `int picoquic_update_sack_list(picoquic_sack_list_t *, uint64_t, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:8189-8275`
* Rust item: `update_sack_list`

### C body
```c
{
    int ret = 1; /* duplicate by default, reset to 0 if update found */
    picoquic_sack_item_t* previous = picoquic_sack_find_range_below_number(sack_list, NULL, pn64_min);

    if (previous == NULL || previous->end_of_sack_range + 1 < pn64_min) {
        /* No overlap with a range below */
        picoquic_sack_item_t* next = (previous == NULL) ?
            picoquic_sack_first_item(sack_list) : picoquic_sack_next_item(previous);
        if (next == NULL || next->start_of_sack_range - 1 > pn64_max) {
            /* create a new item in the list */
            ret = picoquic_sack_insert_item(sack_list, pn64_min, pn64_max, current_time);
            /* set previous to null to bypass the next block */
            previous = NULL;
        }
        else {
            /* extend the existing item towards the min. */
            next->start_of_sack_range = pn64_min;
            /* record that this item was modified. */
            picoquic_sack_item_record_reset(sack_list, next);
            next->time_created = current_time;
            ret = 0;
            /* set previous to next and do the extension part */
            previous = next;
        }
    }
    while (previous != NULL && previous->end_of_sack_range < pn64_max) {
        /* we found or created an item that includes the beginning
         * of the acked range. Check the next one */
        picoquic_sack_item_t* next = picoquic_sack_next_item(previous);
        if (next == NULL || next->start_of_sack_range - 1 > pn64_max) {
            /* No overlap. Extend the previous item up to the max of the range */
            previous->end_of_sack_range = pn64_max;
            /* record that this item was modified. */
            picoquic_sack_item_record_reset(sack_list, previous);
            previous->time_created = current_time;
            ret = 0;
        }
        else {
            /* Overlap. */
            /* Extend the range of the previous item to include the next one. */
            previous->end_of_sack_range = next->end_of_sack_range;
            /* record that this item was modified. */
            picoquic_sack_item_record_reset(sack_list, previous);
            if (next->time_created > previous->time_created) {
                previous->time_created = next->time_created;
            }
            ret = 0;
            /* Delete the next item, accounting of ack times, etc. */
            picoquic_sack_delete_item(sack_list, next);
        }
    }

    if (sack_list->horizon_delay > 0) {
        picoquic_update_ack_horizon(sack_list, current_time);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<i32, crate::Error> {
        let mut ret = 1;
        let mut previous = self.find_range_below_number(None, pn64_min);
        let no_overlap_below = previous
            .and_then(|token| self.sack_items.get(token))
            .map(|item| item.end_of_sack_range.saturating_add(1) < pn64_min)
            .unwrap_or(true);

        if no_overlap_below {
            let next = if let Some(previous) = previous {
                self.item_above(previous)
            } else {
                self.last_item()
            };
            let no_overlap_above = next
                .and_then(|token| self.sack_items.get(token))
                .map(|item| item.start_of_sack_range > pn64_max.saturating_add(1))
                .unwrap_or(true);

            if no_overlap_above {
                self.insert_item(pn64_min, pn64_max, current_time)?;
                ret = 0;
                previous = None;
            } else if let Some(next) = next {
                self.rekey_item_start(next, pn64_min)?;
                self.item_record_reset(next);
                if let Some(item) = self.sack_items.get_mut(next) {
                    item.time_created = current_time;
                }
                ret = 0;
                previous = Some(next);
            }
        }

        while let Some(previous_token) = previous {
            let previous_end = match self.sack_items.get(previous_token) {
                Some(item) => item.end_of_sack_range,
                None => break,
            };
            if previous_end >= pn64_max {
                break;
            }

            let next = self.item_above(previous_token);
            let no_overlap_above = next
                .and_then(|token| self.sack_items.get(token))
                .map(|item| item.start_of_sack_range > pn64_max.saturating_add(1))
                .unwrap_or(true);

            if no_overlap_above {
                if let Some(item) = self.sack_items.get_mut(previous_token) {
                    item.end_of_sack_range = pn64_max;
                    item.time_created = current_time;
                }
                self.item_record_reset(previous_token);
                ret = 0;
            } else if let Some(next) = next {
                let (next_end, next_time) = self
                    .sack_items
                    .get(next)
                    .map(|item| (item.end_of_sack_range, item.time_created))
                    .ok_or(crate::Error::Generic)?;
                if let Some(item) = self.sack_items.get_mut(previous_token) {
                    item.end_of_sack_range = next_end;
                    if next_time > item.time_created {
                        item.time_created = next_time;
                    }
                }
                self.item_record_reset(previous_token);
                self.delete_item(next)?;
                ret = 0;
            } else {
                break;
            }
        }

        if self.horizon_delay > 0 {
            self.update_ack_horizon(current_time);
        }

        Ok(ret)
    }
```

## `picoquic/sender.c:picoquic_mark_high_priority_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: C uses cnx->quic->default_stream_priority when clearing high priority, while Rust uses DEFAULT_STREAM_PRIORITY.
* C source: `picoquic/sender.c:228-243`
* C signature: `int picoquic_mark_high_priority_stream(picoquic_cnx_t *, uint64_t, int)`
* Rust source: `rs/fq/src/lib.rs:4046-4064`
* Rust item: `mark_high_priority_stream`

### C body
```c
{
    int ret;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (is_high_priority) {
        cnx->high_priority_stream_id = stream_id;
    }
    else if (cnx->high_priority_stream_id == stream_id) {
        cnx->high_priority_stream_id = UINT64_MAX;
    }

    ret = picoquic_set_stream_priority(cnx, stream_id, (is_high_priority) ? 0 : cnx->quic->default_stream_priority);

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        if is_high_priority {
            self.high_priority_stream_id = stream_id;
        } else if self.high_priority_stream_id == stream_id {
            self.high_priority_stream_id = u64::MAX;
        }
        self.set_stream_priority(
            stream_id,
            if is_high_priority {
                0
            } else {
                DEFAULT_STREAM_PRIORITY
            },
        )
    }
```

## `picoquic/sender.c:picoquic_prepare_packet_ex`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust follows the broad timer, path selection, segment preparation, padding, statistics, app wake, and result flow, but visibly omits or changes several C details such as path-control packet handling for non-first tuples, packet batch sizing/coalescing rules, address logging, picomask interception, socket message size loop behavior, and wake reinsertion.
* C source: `picoquic/sender.c:3981-4181`
* C signature: `int picoquic_prepare_packet_ex(picoquic_cnx_t *, uint64_t, uint8_t *, size_t, size_t *, struct sockaddr_storage *, struct sockaddr_storage *, int *, size_t *)`
* Rust source: `rs/fq/src/lib.rs:3539-3695`
* Rust item: `prepare_packet_ex`

### C body
```c
{
    uint64_t next_wake_time;
    int ret;
    PICOQUIC_THREAD_CHECK(cnx->quic);
        
    ret = picoquic_handle_send_timers(cnx, current_time, &next_wake_time);
    *send_length = 0;

    if (send_buffer_max < PICOQUIC_ENFORCED_INITIAL_MTU) {
        DBG_PRINTF("Invalid buffer size: %zu", send_buffer_max);
        ret = -1;
    }

    if (ret == 0) {
        picoquic_path_t* path_x = NULL;
        picoquic_tuple_t* tuple = NULL;
        uint64_t initial_next_time;
        size_t coalesced_packet_size = 0;

        picoquic_handle_send_paths(cnx, current_time, &next_wake_time,
            &path_x, &tuple, p_addr_to, p_addr_from, if_index,
            send_buffer_max, send_msg_size);
        initial_next_time = next_wake_time;

        while (ret == 0)
        {
            /* Create a new packet, which may include several segments */
            int is_initial_sent = 0;
            size_t packet_max = send_buffer_max - *send_length;
            uint8_t* packet_buffer = send_buffer + *send_length;

            coalesced_packet_size = 0;
#if TODO
            if (if_index == PICOQUIC_RESERVED_IF_INDEX && quic->proxy_ctx) {
                /* Reset the max packet size to what can be absorbed by the proxy connection. */
                /* This may become more complicated if we support multiple encapsulations. */
            }
#endif
            /* Reset the wake time to the initial value after sending packets */
            next_wake_time = initial_next_time;

            if (send_msg_size != NULL && *send_msg_size > 0 && *send_length > 0 &&
                packet_max > * send_msg_size) {
                /* Consecutive packets should not be larger than first packet */
                packet_max = *send_msg_size;
            }

            /* Send the available segments in that packet. */
            while (ret == 0)
            {
                /* Create the segments that fit in the new packet */
                size_t available = packet_max;
                size_t segment_length = 0;
                picoquic_packet_t* packet = NULL;

                if (coalesced_packet_size > 0) {
                    packet_max = path_x->send_mtu;

                    if (packet_max < coalesced_packet_size + PICOQUIC_MIN_SEGMENT_SIZE) {
                        break;
                    }
                    else {
                        available = packet_max - coalesced_packet_size;
                    }
                }

                packet = picoquic_create_packet(cnx->quic);

                if (packet == NULL) {
                    ret = PICOQUIC_ERROR_MEMORY;
                    break;
                }
                else {
                    if (tuple != path_x->first_tuple) {
                        ret = picoquic_prepare_path_control_packet(cnx, path_x, tuple,
                            packet, current_time, 
                            send_buffer, send_buffer_max, send_length,
                            &next_wake_time);
                    }
                    else {
                        ret = picoquic_prepare_segment(cnx, path_x, packet, current_time,
                            packet_buffer + coalesced_packet_size, available, &segment_length, &next_wake_time, &is_initial_sent);
                    }

                    if (ret == 0) {
                        coalesced_packet_size += segment_length;
                        if (packet->length == 0) {
                            /* Nothing more to send */
                            picoquic_recycle_packet(cnx->quic, packet);
                            break;
                        }
                        else if (packet->ptype == picoquic_packet_1rtt_protected) {
                            /* Cannot coalesce packets after 1 rtt packet */
                            break;
                        }
                        else if (segment_length == 0) {
                            DBG_PRINTF("Send bug: segment length = %zu, packet length = %zu\n", segment_length, packet->length);
                            break;
                        }
                    }
                    else {
                        picoquic_recycle_packet(cnx->quic, packet);
                        if (coalesced_packet_size != 0) {
                            ret = 0;
                        }
                        break;
                    }

                    if (cnx->quic->dont_coalesce_init || tuple != path_x->first_tuple) {
                        break;
                    }
                }
            }

            if (is_initial_sent &&
                cnx->cnx_state < picoquic_state_client_almost_ready &&
                coalesced_packet_size > 0 &&
                coalesced_packet_size < PICOQUIC_ENFORCED_INITIAL_MTU) {
                /* This is bad */
                size_t padding = packet_max - coalesced_packet_size;
                picoquic_public_random(packet_buffer + coalesced_packet_size, padding);
                coalesced_packet_size += padding;
            }

            if (coalesced_packet_size > packet_max) {
#ifdef HUNTING_FOR_BUFFER_OVERFLOW
                int* x = NULL;
                *x += 1;
#endif
                picoquic_log_app_message(cnx, "BUFFER OVERFLOW? Packet size %zu larger than %zu", coalesced_packet_size, packet_max);
            }
            if (coalesced_packet_size > 0) {
                if (coalesced_packet_size > cnx->max_mtu_sent) {
                    cnx->max_mtu_sent = coalesced_packet_size;
                }
                cnx->nb_packets_sent++;
                /* if needed, log that the packet is sent */
                if (p_addr_to != NULL && p_addr_from != NULL) {
                    picoquic_log_pdu(cnx, 0, current_time,
                        (struct sockaddr*)p_addr_to, (struct sockaddr*)p_addr_from, coalesced_packet_size,
                        path_x->unique_path_id, 0);
                }
            }

            /* Update the wake up time for the connection */
            if (coalesced_packet_size > 0 || cnx->cnx_state == picoquic_state_disconnected) {
                next_wake_time = current_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
            }

            /* Account for the bytes in the packet. */
            *send_length += coalesced_packet_size;

            if (*if_index == PICOQUIC_RESERVED_IF_INDEX && *send_length > 0 && cnx->quic->picomask_ctx != NULL && cnx->quic->picomask_fns != NULL) {
                /* Ask the proxy to handle the packet */
                /* if we queue it as a datagram, set packet_size to 0 */
                /* If we can do some short cut, rewrite the packet in place per shortcut spec. */
                ret = (cnx->quic->picomask_fns->picomask_intercept_fn)
                    (cnx->quic, cnx->quic->picomask_ctx, current_time, send_buffer, send_length, send_msg_size,
                    p_addr_to, p_addr_from, if_index);
                if (ret == 0) {

                }

                if (ret < 0 || *send_length == 0) {
                    break;
                }
            }

            /* Check whether to keep coalescing multiple packets in the send buffer */
            if (send_msg_size == NULL) {
                break;
            }
            else if (coalesced_packet_size > *send_msg_size) {
                /* This can only happen for the first packet in a batch. */
                *send_msg_size = coalesced_packet_size;
            }
            else if (coalesced_packet_size != *send_msg_size) {
                break;
            }
            else if (*send_length + *send_msg_size > send_buffer_max) {
                break;
            }
        }
        if (*send_length > 0) {
            picoquic_handle_send_train_statistics(cnx, path_x, coalesced_packet_size, send_length, send_msg_size);
        }
    }

    if (ret == 0) {
        ret = picoquic_program_app_wake_time(cnx, &next_wake_time);
    }

    picoquic_reinsert_by_wake_time(cnx->quic, cnx, next_wake_time);

    return ret;
}
```

### Rust body
```rust
    ) -> Result<PreparedCnxPacket, Error> {
        let mut next_wake_time = Instant::from_ticks(0);
        let mut ret = self.handle_send_timers(current_time, &mut next_wake_time);
        let mut send_length = 0usize;
        let mut send_msg_size = None;
        let default_addr = unspecified_socket_addr();
        let mut addr_to = default_addr;
        let mut addr_from = default_addr;
        let mut if_index = -1;

        if send_buffer.len() < crate::internal::ENFORCED_INITIAL_MTU {
            ret = crate::errors::InternalError::SendBufferTooSmall as i32;
        }

        if ret == 0 {
            if self.path_demotion_needed {
                self.delete_abandoned_paths(current_time, &mut next_wake_time);
            }
            if self.tuple_demotion_needed {
                self.delete_demoted_tuples(current_time, &mut next_wake_time);
            }

            if let Some((path_token, tuple_index)) =
                self.select_next_path_tuple(current_time, &mut next_wake_time)
            {
                let path_idx = path_token.slot_idx();
                if let Some(path) = self.paths.get(path_idx)
                    && let Some(tuple) = path.tuples.get(tuple_index)
                {
                    addr_to = tuple.peer_addr;
                    addr_from = tuple.local_addr;
                    if_index = tuple.if_index as i32;
                    send_msg_size = Some(path.send_mtu);
                    if send_buffer.len() > path.send_mtu {
                        self.is_sending_large_buffer = true;
                    }
                }

                let initial_next_time = next_wake_time;
                let mut coalesced_packet_size = 0usize;
                let mut is_initial_sent = 0;
                let packet_max = send_buffer.len();

                while ret == 0 && send_length < send_buffer.len() {
                    next_wake_time = initial_next_time;
                    let available = packet_max.saturating_sub(coalesced_packet_size);
                    if available == 0 {
                        break;
                    }
                    let mut packet = crate::internal::Connection::empty_sender_packet(current_time);
                    let mut segment_length = 0usize;
                    let packet_buffer_start = send_length.saturating_add(coalesced_packet_size);
                    let packet_buffer_end = packet_buffer_start
                        .saturating_add(available)
                        .min(send_buffer.len());
                    if packet_buffer_start >= packet_buffer_end {
                        break;
                    }

                    // SAFETY: `path_ptr` points into `self.paths[path_idx]`.
                    // This mirrors the C call shape (`cnx` plus `path_x`).
                    // The selected path is the only path mutably modified by
                    // this segment-formatting call.
                    let path_ptr: *mut Path = &raw mut self.paths[path_idx];
                    ret = unsafe {
                        self.prepare_segment(
                            &mut *path_ptr,
                            &mut packet,
                            current_time,
                            &mut send_buffer[packet_buffer_start..packet_buffer_end],
                            available,
                            &mut segment_length,
                            &mut next_wake_time,
                            &mut is_initial_sent,
                        )
                    };

                    if ret == 0 {
                        coalesced_packet_size =
                            coalesced_packet_size.saturating_add(segment_length);
                        if packet.length == 0
                            || packet.packet_type == crate::internal::PacketType::OneRttProtected
                            || segment_length == 0
                        {
                            break;
                        }
                    } else if coalesced_packet_size != 0 {
                        ret = 0;
                        break;
                    } else {
                        break;
                    }

                    if self
                        .quic_ref()
                        .map(|q| q.dont_coalesce_init)
                        .unwrap_or(false)
                    {
                        break;
                    }
                }

                if is_initial_sent != 0
                    && self.connection_state < State::ClientAlmostReady
                    && coalesced_packet_size > 0
                    && coalesced_packet_size < crate::internal::ENFORCED_INITIAL_MTU
                {
                    let padding = packet_max.saturating_sub(coalesced_packet_size);
                    let start = coalesced_packet_size;
                    let end = start.saturating_add(padding).min(send_buffer.len());
                    crate::internal::public_random(&mut send_buffer[start..end]);
                    coalesced_packet_size = end;
                }

                if coalesced_packet_size > 0 {
                    self.max_mtu_sent = self.max_mtu_sent.max(coalesced_packet_size);
                    self.nb_packets_sent = self.nb_packets_sent.saturating_add(1);
                    next_wake_time = current_time;
                }
                send_length = send_length.saturating_add(coalesced_packet_size);

                if send_length > 0 {
                    let path_ptr: *const Path = &raw const self.paths[path_idx];
                    // SAFETY: immutable borrow of selected path for statistics
                    // after segment formatting has completed.
                    let path = unsafe { &*path_ptr };
                    self.handle_send_train_statistics(
                        path,
                        coalesced_packet_size,
                        send_length,
                        send_msg_size,
                    );
                }
            }
        }

        if ret == 0 {
            self.program_app_wake_time(&mut next_wake_time);
        }
        self.next_wake_time = next_wake_time;

        if ret == 0 {
            Ok(PreparedCnxPacket {
                send_length,
                addr_to,
                addr_from,
                if_index,
                send_msg_size,
            })
        } else {
            Err(crate::internal::sender_status_to_error(ret))
        }
    }
```

## `picoquic/sender.c:picoquic_stop_sending`
* Phase 4C status: `suspect`
* Phase 4C rationale: Core stream state changes match, but C always reinserts by wake time after the operation while Rust only conditionally enqueues an output stream.
* C source: `picoquic/sender.c:430-458`
* C signature: `int picoquic_stop_sending(picoquic_cnx_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:4328-4352`
* Rust item: `stop_sending`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream = NULL;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    stream = picoquic_find_stream(cnx, stream_id);

    if (stream == NULL) {
        ret = PICOQUIC_ERROR_INVALID_STREAM_ID;
    }
    else {
        stream->app_stream_ctx = NULL;

        if (stream->reset_received) {
            ret = PICOQUIC_ERROR_STREAM_ALREADY_CLOSED;
        }
        else if (!stream->stop_sending_requested) {
            stream->local_stop_error = local_stream_error;
            stream->stop_sending_requested = 1;
            picoquic_insert_output_stream(cnx, stream);
        }
    }

    picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));

    return ret;
}
```

### Rust body
```rust
    pub fn stop_sending(&mut self, stream_id: u64, local_stream_error: u64) -> Result<(), Error> {
        let stream_token = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let mut should_enqueue = false;
        {
            let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;
            stream.app_stream_ctx = None;
            if stream.reset_received {
                return Err(Error::Protocol(InternalError::StreamAlreadyClosed as u64));
            }
            if !stream.stop_sending_requested {
                stream.local_stop_error = local_stream_error;
                stream.stop_sending_requested = true;
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
