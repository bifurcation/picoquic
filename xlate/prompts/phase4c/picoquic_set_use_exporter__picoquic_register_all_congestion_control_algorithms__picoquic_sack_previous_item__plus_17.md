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

## Pair `picoquic/quicctx.c:picoquic_set_use_exporter`
C: `picoquic/quicctx.c:5548-5551 picoquic_set_use_exporter`
Rust: `rs/fq/src/lib.rs:1660-1662 set_use_exporter`

### C body
```c
void picoquic_set_use_exporter(picoquic_quic_t* quic, int use_exporter) {
    PICOQUIC_THREAD_CHECK(quic);
    picoquic_tls_set_use_exporter(quic, use_exporter);
}
```

### Rust body
```rust
    pub fn set_use_exporter(&mut self, use_exporter: bool) {
        self.use_exporter = use_exporter;
    }
```

## Pair `picoquic/register_all_cc_algorithms.c:picoquic_register_all_congestion_control_algorithms`
C: `picoquic/register_all_cc_algorithms.c:40-51 picoquic_register_all_congestion_control_algorithms`
Rust: `rs/fq/src/lib.rs:4563-4565 register_all_congestion_control_algorithms`

### C body
```c
{
    getter_test_cc_algo_list[0] = picoquic_newreno_algorithm;
    getter_test_cc_algo_list[1] = picoquic_cubic_algorithm;
    getter_test_cc_algo_list[2] = picoquic_dcubic_algorithm;
    getter_test_cc_algo_list[3] = picoquic_fastcc_algorithm;
    getter_test_cc_algo_list[4] = picoquic_bbr_algorithm;
    getter_test_cc_algo_list[5] = picoquic_prague_algorithm;
    getter_test_cc_algo_list[6] = picoquic_bbr1_algorithm;
    getter_test_cc_algo_list[7] = c4_algorithm;
    picoquic_register_congestion_control_algorithms(getter_test_cc_algo_list, 8);
}
```

### Rust body
```rust
pub fn register_all_congestion_control_algorithms() {
    let _ = CC_ALGORITHM_REGISTRY.set(ALL_CC_ALGORITHMS.to_vec());
}
```

## Pair `picoquic/sacks.c:picoquic_sack_previous_item`
C: `picoquic/sacks.c:84-87 picoquic_sack_previous_item`
Rust: `rs/fq/src/internal.rs:8423-8427 sack_previous_item`

### C body
```c
{
    return picoquic_sack_item_value(picosplay_previous(&sack->node));
}
```

### Rust body
```rust
    pub fn sack_previous_item(&mut self, sack: SackItemToken) -> Option<SackItemToken> {
        let st = self.sack_items.get(sack)?.ack_tree_membership?;
        let prev_st = self.ack_tree.next(st)?;
        self.ack_tree.get(prev_st).copied()
    }
```

## Pair `picoquic/sacks.c:picoquic_sack_list_from_cnx_context`
C: `picoquic/sacks.c:148-154 picoquic_sack_list_from_cnx_context`
Rust: `rs/fq/src/internal.rs:8460-8467 sack_list_from_cnx_context`

### C body
```c
{
    return &picoquic_ack_ctx_from_cnx_context(cnx, pc, l_cid)->sack_list;
}
```

### Rust body
```rust
    ) -> Option<&mut SackList> {
        let ack_ctx = self.ack_ctx_from_cnx_context(packet_context, local_connection_id)?;
        Some(&mut ack_ctx.sack_list)
    }
```

## Pair `picoquic/sacks.c:picoquic_record_pn_received`
C: `picoquic/sacks.c:258-294 picoquic_record_pn_received`
Rust: `rs/fq/src/internal.rs:8047-8060 record_pn_received`

### C body
```c
{
    int ret = 0;
    picoquic_sack_list_t* sack_list = picoquic_sack_list_from_cnx_context(cnx, pc, l_cid);

    if (sack_list != NULL) {
        if (picoquic_sack_list_is_empty(sack_list)) {
            /* This is the first packet ever received.. */
            cnx->ack_ctx[pc].time_stamp_largest_received = current_microsec;
        }
        else {
            uint64_t pn_last = picoquic_sack_list_last(sack_list);
            if (pn64 > pn_last) {
                if (pn64 > pn_last + 1) {
                    cnx->ack_ctx[pc].act[0].out_of_order_received = 1;
                    cnx->ack_ctx[pc].act[1].out_of_order_received = 1;
                }
                cnx->ack_ctx[pc].time_stamp_largest_received = current_microsec;
            }
            else
            {
                if (cnx->ack_ctx[pc].act[0].ack_needed && pn64 < cnx->ack_ctx[pc].act[0].highest_ack_sent) {
                    cnx->ack_ctx[pc].act[0].out_of_order_received = 1;
                }
                if (cnx->ack_ctx[pc].act[1].ack_needed && pn64 < cnx->ack_ctx[pc].act[1].highest_ack_sent) {
                    cnx->ack_ctx[pc].act[1].out_of_order_received = 1;
                }
            }
        }

        ret = picoquic_update_sack_list(sack_list, pn64, pn64, current_microsec);
    }
    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        // C: picoquic_record_pn_received — insert `pn64` into SACK list.
        let ack_ctx = &mut self.ack_ctx[pc as usize];
        match ack_ctx.sack_list.update(pn64, pn64, current_microsec) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    }
```

## Pair `picoquic/sacks.c:picoquic_update_ack_horizon`
C: `picoquic/sacks.c:382-403 picoquic_update_ack_horizon`
Rust: `rs/fq/src/internal.rs:8357-8364 update_ack_horizon`

### C body
```c
{
    picoquic_sack_item_t* first_sack = picoquic_sack_first_item(sack_list);

    while (first_sack != NULL && first_sack->nb_times_sent[0] >= PICOQUIC_MAX_ACK_RANGE_REPEAT) {
        int64_t delay = current_time - first_sack->time_created;
        if (delay > sack_list->horizon_delay) {
            picoquic_sack_item_t* next_sack = picoquic_sack_next_item(first_sack);
            if (next_sack != NULL) {
                /* Always keep the last range */
                sack_list->ack_horizon = first_sack->end_of_sack_range + 1;
                picoquic_sack_delete_item(sack_list, first_sack);
            }
            first_sack = next_sack;
        }
        else {
            break;
        }
    }
}
```

### Rust body
```rust
    pub fn update_ack_horizon(&mut self, current_time: Instant) {
        if self.horizon_delay > 0 {
            let horizon_ticks = current_time
                .ticks()
                .saturating_sub(self.horizon_delay as u64);
            self.ack_horizon = crate::Instant::from_ticks(horizon_ticks);
        }
    }
```

## Pair `picoquic/sacks.c:picoquic_sack_list_init`
C: `picoquic/sacks.c:430-437 picoquic_sack_list_init`
Rust: `rs/fq/src/internal.rs:8605-8614 new`

### C body
```c
{
    memset(sack_list, 0, sizeof(picoquic_sack_list_t));
    picosplay_init_tree(&sack_list->ack_tree, picoquic_sack_item_compare,
        picoquic_sack_node_create, picoquic_sack_node_delete, picoquic_sack_node_value);
}
```

### Rust body
```rust
                SackRangeCount {
                    range_counts: [0; MAX_ACK_RANGE_REPEAT],
                },
```

## Pair `picoquic/sacks.c:picoquic_sack_item_nb_times_sent`
C: `picoquic/sacks.c:471-474 picoquic_sack_item_nb_times_sent`
Rust: `rs/fq/src/internal.rs:8673-8718 nb_times_sent`

### C body
```c
{
    return sack_item->nb_times_sent[is_opportunistic];
}
```

### Rust body
```rust
impl SackList {
    /// Bump the per-range send counter for the item at `token` and keep
    /// the per-bucket histogram in `rc` consistent.
    /// C: `picoquic_sack_item_record_sent`
    pub fn item_record_sent(&mut self, token: SackItemToken, is_opportunistic: i32) {
        let idx = is_opportunistic.clamp(0, 1) as usize;
        let old = match self.sack_items.get(token) {
            Some(item) => item.nb_times_sent[idx],
            None => return,
        };
        if (old as usize) < MAX_ACK_RANGE_REPEAT {
            self.rc[idx].range_counts[old as usize] -= 1;
        }
        let new = old + 1;
        if let Some(item) = self.sack_items.get_mut(token) {
            item.nb_times_sent[idx] = new;
        }
        if (new as usize) < MAX_ACK_RANGE_REPEAT {
            self.rc[idx].range_counts[new as usize] += 1;
        }
    }

    /// Reset the per-range send counters for the item at `token` to zero
    /// and keep the per-bucket histogram in `rc` consistent.
    /// C: `picoquic_sack_item_record_reset`
    pub fn item_record_reset(&mut self, token: SackItemToken) {
        let old = match self.sack_items.get(token) {
            Some(item) => item.nb_times_sent,
            None => return,
        };
        for (r, &old_count) in old.iter().enumerate() {
            if (old_count as usize) < MAX_ACK_RANGE_REPEAT {
                self.rc[r].range_counts[old_count as usize] -= 1;
            }
            self.rc[r].range_counts[0] += 1;
        }
        if let Some(item) = self.sack_items.get_mut(token) {
            item.nb_times_sent = [0; 2];
        }
    }
}
```

## Pair `picoquic/sender.c:picoquic_find_stream_for_writing`
C: `picoquic/sender.c:50-78 picoquic_find_stream_for_writing`
Rust: `rs/fq/src/lib.rs:3830-3838 find_stream_for_writing`

### C body
```c
{
    picoquic_stream_head_t* stream = picoquic_find_stream(cnx, stream_id);

    *ret = 0;

    if (stream == NULL) {
        /* Need to check that the ID is authorized */

        /* Check parity */
        if (IS_CLIENT_STREAM_ID(stream_id) != cnx->client_mode) {
            *ret = PICOQUIC_ERROR_INVALID_STREAM_ID;
        }

        if (*ret == 0) {
            if (stream_id < cnx->next_stream_id[STREAM_TYPE_FROM_ID(stream_id)]) {
                *ret = PICOQUIC_ERROR_STREAM_ALREADY_CLOSED;
            } else {
                stream = picoquic_create_missing_streams(cnx, stream_id, 0);
                if (stream == NULL) {
                    *ret = PICOQUIC_ERROR_MEMORY;
                }
            }
        }
    }

    return stream;
}
```

### Rust body
```rust
        if let Some(stream) = self.find_stream(stream_id) {
            return Ok(stream);
        }
```

## Pair `picoquic/sender.c:picoquic_mark_datagram_ready_path`
C: `picoquic/sender.c:124-144 picoquic_mark_datagram_ready_path`
Rust: `rs/fq/src/lib.rs:4391-4406 mark_datagram_ready_path`

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

## Pair `picoquic/sender.c:picoquic_set_datagram_priority`
C: `picoquic/sender.c:201-205 picoquic_set_datagram_priority`
Rust: `rs/fq/src/lib.rs:4068-4077 set_datagram_priority`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->datagram_priority = datagram_priority;
}
```

### Rust body
```rust
    pub fn set_default_priority(&mut self, default_stream_priority: u8) {
        self.default_stream_priority = default_stream_priority;
    }
```

## Pair `picoquic/sender.c:picoquic_add_to_stream_with_ctx`
C: `picoquic/sender.c:245-309 picoquic_add_to_stream_with_ctx`
Rust: `rs/fq/src/lib.rs:4167-4221 add_to_stream_with_ctx`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic);
        
    stream = picoquic_find_stream_for_writing(cnx, stream_id, &ret);
    if (ret == 0 && set_fin) {
        if (stream->fin_requested) {
            /* app error, notified the fin twice*/
            if (length > 0) {
                ret = -1;
            }
        } else {
            stream->fin_requested = 1;
        }
    }

    /* If our side has sent RST_STREAM or received STOP_SENDING, we should not send anymore data. */
    if (ret == 0 && (stream->reset_sent || stream->stop_sending_received)) {
        ret = -1;
    }

    if (ret == 0 && length > 0) {
        picoquic_stream_queue_node_t* stream_data = (picoquic_stream_queue_node_t*)
            malloc(sizeof(picoquic_stream_queue_node_t));
        if (stream_data == 0) {
            ret = -1;
        } else {
            stream_data->bytes = (uint8_t*)malloc(length);

            if (stream_data->bytes == NULL) {
                free(stream_data);
                stream_data = NULL;
                ret = -1;
            } else {
                picoquic_stream_queue_node_t** pprevious = &stream->send_queue;
                picoquic_stream_queue_node_t* next = stream->send_queue;

                memcpy(stream_data->bytes, data, length);
                stream_data->length = length;
                stream_data->offset = 0;
                stream_data->next_stream_data = NULL;

                while (next != NULL) {
                    pprevious = &next->next_stream_data;
                    next = next->next_stream_data;
                }

                *pprevious = stream_data;
            }
        }

        picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));
    }

    if (ret == 0) {
        cnx->nb_bytes_queued += length;
        stream->is_active = 0;
        stream->app_stream_ctx = app_stream_ctx;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let stream_token = self.find_stream_for_writing(stream_id)?;
        let mut should_enqueue = false;

        {
            let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;

            if set_fin {
                if stream.fin_requested {
                    if !data.is_empty() {
                        return Err(Error::InvalidState);
                    }
                } else {
                    stream.fin_requested = true;
                }
            }

            if stream.reset_sent || stream.stop_sending_received {
                return Err(Error::InvalidState);
            }

            if !data.is_empty() {
                let offset = stream
                    .send_queue
                    .back()
                    .map(|node| node.offset.saturating_add(node.bytes.len() as u64))
                    .unwrap_or(stream.sent_offset);
                stream.send_queue.push_back(internal::StreamQueueNode {
                    offset,
                    bytes: data.to_vec(),
                });
            }

            stream.is_active = false;
            stream.app_stream_ctx = app_stream_ctx;
            if !stream.is_output_stream
                && (!stream.send_queue.is_empty() || (stream.fin_requested && !stream.fin_sent))
            {
                stream.is_output_stream = true;
                should_enqueue = true;
            }
        }

        self.nb_bytes_queued = self.nb_bytes_queued.saturating_add(data.len() as u64);
        if should_enqueue {
            enqueue_output_stream_token(self, stream_token);
        }
        Ok(())
    }
```

## Pair `picoquic/sender.c:picoquic_reset_stream_ctx`
C: `picoquic/sender.c:369-377 picoquic_reset_stream_ctx`
Rust: `rs/fq/src/lib.rs:4157-4162 reset_stream_ctx`

### C body
```c
{
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if ((stream = picoquic_find_stream(cnx, stream_id)) != NULL) {
        stream->app_stream_ctx = NULL;
    }
}
```

### Rust body
```rust
        {
            stream.app_stream_ctx = None;
        }
```

## Pair `picoquic/sender.c:picoquic_stop_sending`
C: `picoquic/sender.c:430-458 picoquic_stop_sending`
Rust: `rs/fq/src/lib.rs:4328-4352 stop_sending`

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

## Pair `picoquic/sender.c:picoquic_create_packet`
C: `picoquic/sender.c:533-559 picoquic_create_packet`
Rust: `rs/fq/src/internal.rs:1059-1171 create_packet`

### C body
```c
{
    picoquic_packet_t* packet = quic->p_first_packet;
    
    if (packet == NULL) {
        packet = (picoquic_packet_t*)malloc(sizeof(picoquic_packet_t));
        if (packet != NULL) {
            quic->nb_packets_allocated++;
            if (quic->nb_packets_allocated > quic->nb_packets_allocated_max) {
                quic->nb_packets_allocated_max = quic->nb_packets_allocated;
            }
        }
    }
    else {
        quic->p_first_packet = packet->packet_previous;
        quic->nb_packets_in_pool--;
    }

    if (packet != NULL) {
        /* It might be sufficient to zero the metadata, but zeroing everything
         * appears safer, and does not confuse checkers like valgrind.
         */
        memset(packet, 0, sizeof(picoquic_packet_t));
    }

    return packet;
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

## Pair `picoquic/sender.c:picoquic_create_long_header`
C: `picoquic/sender.c:641-709 picoquic_create_long_header`
Rust: `rs/fq/src/internal.rs:6831-6894 create_long_header`

### C body
```c
{
    /* Create a long packet */
    size_t length = 0;

    /* The first byte is defined in RFC 9000 as:
    *     Header Form (1) = 1,
    *     Fixed Bit (1) = 1,
    *     Long Packet Type (2),
    *     Type-Specific Bits (4)
    * The packet type is version dependent. In fact, the whole first byte is version
    * dependent, the invariant draft only specifies the "header form" bit = 1 for long
    * header. In version 1, the packet specific bytes are two reserved bytes +
    * sequence number length, always set to 3 in picoquic (i.e., 4 bytes).
    *
    */
    bytes[0] = picoquic_create_long_packet_type(packet_type, version_index);

    if (do_grease_quic_bit) {
        bytes[0] &= 0xBF;
    }

    length = 1;
    picoformat_32(&bytes[length], version);
    length += 4;

    bytes[length++] = dest_cnx_id->id_len;
    length += picoquic_format_connection_id(&bytes[length], PICOQUIC_MAX_PACKET_SIZE - length, *dest_cnx_id);
    bytes[length++] = srce_cnx_id->id_len;
    length += picoquic_format_connection_id(&bytes[length], PICOQUIC_MAX_PACKET_SIZE - length, *srce_cnx_id);

    /* Special case of packet initial -- encode token as part of header */
    if (packet_type == picoquic_packet_initial) {
        length += picoquic_varint_encode(&bytes[length], PICOQUIC_MAX_PACKET_SIZE - length, retry_token_length);
        if (retry_token_length > 0) {
            memcpy(&bytes[length], retry_token, retry_token_length);
            length += retry_token_length;
        }
    }

    if (packet_type == picoquic_packet_retry) {
        /* No payload length and no sequence number for Retry */
        *pn_offset = 0;
        *pn_length = 0;
    }
    else {
        /* Reserve two bytes for payload length */
        bytes[length++] = 0;
        bytes[length++] = 0;
        /* Encode the sequence number */
        *pn_offset = length;
        *pn_length = 4;
        picoformat_32(&bytes[length], (uint32_t)sequence_number);
        length += 4;
    }
    return length;
}
```

### Rust body
```rust
) -> usize {
    // C: picoquic_create_long_header — encode a QUIC long header into `bytes`.
    let is_v2 = version == Version::V2 as u32 || version == Version::V2Draft as u32;
    let first_byte: u8 = match (packet_type, is_v2) {
        (PacketType::Initial, false) => 0xC3,
        (PacketType::ZeroRttProtected, false) => 0xD3,
        (PacketType::Handshake, false) => 0xE3,
        (PacketType::Retry, false) => 0xF0,
        (PacketType::Initial, true) => 0xD3,
        (PacketType::ZeroRttProtected, true) => 0xE3,
        (PacketType::Handshake, true) => 0xF3,
        (PacketType::Retry, true) => 0xC0,
        _ => 0xFF,
    };
    bytes[0] = first_byte;
    let mut length = 1;
    let ver_bytes = version.to_be_bytes();
    bytes[length..length + 4].copy_from_slice(&ver_bytes);
    length += 4;
    // Dest CID
    bytes[length] = dest_cnx_id.len() as u8;
    length += 1;
    let dlen = dest_cnx_id.len();
    bytes[length..length + dlen].copy_from_slice(dest_cnx_id.as_bytes());
    length += dlen;
    // Src CID
    bytes[length] = srce_cnx_id.len() as u8;
    length += 1;
    let slen = srce_cnx_id.len();
    bytes[length..length + slen].copy_from_slice(srce_cnx_id.as_bytes());
    length += slen;
    // Token for Initial
    if packet_type == PacketType::Initial {
        length += varint_encode(&mut bytes[length..], retry_token.len() as u64);
        bytes[length..length + retry_token.len()].copy_from_slice(retry_token);
        length += retry_token.len();
    }
    if packet_type != PacketType::Retry {
        bytes[length] = 0;
        bytes[length + 1] = 0;
        length += 2;
        *pn_offset = length;
        *pn_length = 4;
        let pn32 = sequence_number as u32;
        bytes[length..length + 4].copy_from_slice(&pn32.to_be_bytes());
        length += 4;
    } else {
        *pn_offset = 0;
        *pn_length = 0;
    }
    length
}
```

## Pair `picoquic/sender.c:picoquic_protect_packet_header`
C: `picoquic/sender.c:874-895 picoquic_protect_packet_header`
Rust: `rs/fq/src/internal.rs:6985-6993 protect_packet_header`

### C body
```c
{
    /* The sample is located after the pn_offset */
    size_t sample_offset = /* header_length */ pn_offset + 4;

    if (pn_offset < sample_offset)
    {
        /* This is always true, as we use pn_length = 4 */
        uint8_t mask_bytes[5] = { 0, 0, 0, 0, 0 };
        uint8_t pn_l;

        picoquic_pn_encrypt(pn_enc, send_buffer + sample_offset, mask_bytes, mask_bytes, 5);
        /* Encode the first byte */
        pn_l = (send_buffer[0] & 3) + 1;
        send_buffer[0] ^= (mask_bytes[0] & first_mask);

        /* Packet encoding is 1 to 4 bytes */
        for (uint8_t i = 0; i < pn_l; i++) {
            send_buffer[pn_offset+i] ^= mask_bytes[i+1];
        }
    }
}
```

### Rust body
```rust
    if pn_offset >= send_buffer.len() {
        return;
    }
```

## Pair `picoquic/sender.c:picoquic_dequeue_retransmitted_packet`
C: `picoquic/sender.c:1107-1131 picoquic_dequeue_retransmitted_packet`
Rust: `rs/fq/src/internal.rs:5645-5659 dequeue_retransmitted_packet`

### C body
```c
{
    pkt_ctx->retransmitted_queue_size -= 1;
    if (p->packet_previous == NULL) {
        pkt_ctx->retransmitted_newest = p->packet_next;
    }
    else {
        p->packet_previous->packet_next = p->packet_next;
    }

    if (p->packet_next == NULL) {
        pkt_ctx->retransmitted_oldest = p->packet_previous;
    }
    else {
        p->packet_next->packet_previous = p->packet_previous;
    }

    /* Packets can be queued simultaneously for data repeat and 
    * for detection of spurious losses, so should only be recycled
    * when removed from both queues */
    p->is_queued_for_spurious_detection = 0;
    if (!p->is_queued_for_data_repeat) {
        picoquic_recycle_packet(cnx->quic, p);
    }
}
```

### Rust body
```rust
            .or_else(|| {
                pkt_ctx
                    .retransmitted
                    .iter()
                    .find_map(|(seq, tok)| (*tok == packet).then_some(*seq))
            });
```

## Pair `picoquic/sender.c:picoquic_is_pkt_ctx_backlog_empty`
C: `picoquic/sender.c:1274-1308 picoquic_is_pkt_ctx_backlog_empty`
Rust: `rs/fq/src/internal.rs:17088-17141 picoquic_is_pkt_ctx_backlog_empty`

### C body
```c
{
    int backlog_empty = 1;
    picoquic_packet_t* p = pkt_ctx->pending_first;

    while (p != NULL && backlog_empty == 1) {
        /* check if this is an ACK only packet */
        int ret = 0;
        int frame_is_pure_ack = 0;
        size_t frame_length = 0;
        size_t byte_index = 0; /* Used when parsing the old packet */

        byte_index = p->offset;

        if (!p->is_ack_trap && !p->is_multipath_probe && !p->is_mtu_probe) {
            while (ret == 0 && byte_index < p->length) {
                ret = picoquic_skip_frame(&p->bytes[byte_index],
                    p->length - p->offset, &frame_length, &frame_is_pure_ack);

                if (!frame_is_pure_ack) {
                    backlog_empty = 0;
                    break;
                }
                byte_index += frame_length;
            }
        }

        p = p->packet_next;
    }

    return backlog_empty;
}
```

### Rust body
```rust
    ) {
        let period = self.sequence_hole_pseudo_period();
        if period == 0 {
            pkt_ctx.next_sequence_hole = u64::MAX;
            return;
        }
        if self.connection_state == State::Ready
            && pkt_ctx.pending.values().next_back().is_some()
            && pkt_ctx.send_sequence >= pkt_ctx.next_sequence_hole
        {
            let pending_last_is_trap = pkt_ctx
                .pending
                .values()
                .next_back()
                .and_then(|tok| self.queued_packets.get(*tok))
                .map(|packet| packet.is_ack_trap)
                .unwrap_or(false);
            if pkt_ctx.next_sequence_hole != 0 && !pending_last_is_trap {
                let mut packet = Self::empty_sender_packet(current_time);
                packet.is_ack_trap = true;
                packet.packet_context = PacketContext::Application;
                packet.packet_type = PacketType::OneRttProtected;
                packet.send_time = current_time;
                packet.send_path = None;
                packet.sequence_number = pkt_ctx.send_sequence;
                pkt_ctx.send_sequence = pkt_ctx.send_sequence.saturating_add(1);
                self.queue_for_retransmit(path_x, &mut packet, 0, current_time);
                self.set_sender_wake_now(next_wake_time, current_time);
                path_x.q_square = path_x.q_square.saturating_add(1);
                self.nb_packet_holes_inserted = self.nb_packet_holes_inserted.saturating_add(1);
            }
            let random_bound = (period as u64)
                .checked_shl(self.nb_packet_holes_inserted.min(63) as u32)
                .unwrap_or(u64::MAX);
            pkt_ctx.next_sequence_hole = pkt_ctx
                .send_sequence
                .saturating_add(3)
                .saturating_add(public_uniform_random(random_bound.max(1)));
        }
    }
```

## Pair `picoquic/sender.c:picoquic_preemptive_retransmit_as_needed`
C: `picoquic/sender.c:1494-1543 picoquic_preemptive_retransmit_as_needed`
Rust: `rs/fq/src/internal.rs:17555-17612 picoquic_preemptive_retransmit_as_needed`

### C body
```c
{
    /* If there is a single packet context for application frames,
     * the code just has to track the preemptive_repeat_ptr for
     * that context. If there are multiple paths, this gets a bit
     * more complicated, because packets that need to be premptively
     * repeated might be found in many context, and also because some
     * paths may be only used for primary repeats. In that case, we
     * want to try all available packet contexts.
     */
    int ret = 0;
    int has_data = 0;
    picoquic_packet_context_t* pkt_ctx;
    uint64_t rtt = path_x->smoothed_rtt;

    if (pc == picoquic_packet_context_application &&
        cnx->is_multipath_enabled) {
        for (int i = 0; i < cnx->nb_paths; i++) {
            pkt_ctx = &cnx->path[i]->pkt_ctx;
            ret = picoquic_preemptive_retransmit_in_context(
                cnx, pkt_ctx, rtt, current_time, next_wake_time,
                new_bytes, send_buffer_max_minus_checksum, length, &has_data, more_data, is_pure_ack == NULL);
            if (ret != 0 || has_data != 0) {
                break;
            }
        }
    }
    else {
        pkt_ctx = &cnx->pkt_ctx[pc];
        ret = picoquic_preemptive_retransmit_in_context(
            cnx, pkt_ctx, rtt, current_time, next_wake_time,
            new_bytes, send_buffer_max_minus_checksum, length, &has_data, more_data, is_pure_ack == NULL);
    }
    
    if (ret == 0 &&  is_pure_ack != NULL) {
        *is_pure_ack &= !has_data;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut ret = 0;
        let mut has_data = 0;
        let rtt = path_x.smoothed_rtt;
        let test_only = is_pure_ack.is_none();

        if pc == PacketContext::Application && self.is_multipath_enabled {
            for path_index in 0..self.paths.len() {
                ret = self.preemptive_retransmit_in_selection(
                    PacketContextSelection::Path(path_index),
                    rtt,
                    current_time,
                    next_wake_time,
                    new_bytes,
                    send_buffer_max_minus_checksum,
                    length,
                    &mut has_data,
                    more_data,
                    test_only,
                );
                if ret != 0 || has_data != 0 {
                    break;
                }
            }
        } else {
            ret = self.preemptive_retransmit_in_selection(
                PacketContextSelection::Connection(pc),
                rtt,
                current_time,
                next_wake_time,
                new_bytes,
                send_buffer_max_minus_checksum,
                length,
                &mut has_data,
                more_data,
                test_only,
            );
        }

        if ret == 0
            && let Some(is_pure_ack) = is_pure_ack
        {
            *is_pure_ack &= i32::from(has_data == 0);
        }

        ret
    }
```
