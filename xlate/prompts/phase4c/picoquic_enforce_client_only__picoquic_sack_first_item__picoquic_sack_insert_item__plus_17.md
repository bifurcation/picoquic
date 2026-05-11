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

## Pair `picoquic/quicctx.c:picoquic_enforce_client_only`
C: `picoquic/quicctx.c:5553-5557 picoquic_enforce_client_only`
Rust: `rs/fq/src/lib.rs:1666-1674 enforce_client_only`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->enforce_client_only = (do_enforce)?1:0;
}
```

### Rust body
```rust
    pub fn set_default_padding(&mut self, padding_multiple: u32, padding_minsize: u32) {
        self.padding_multiple_default = padding_multiple;
        self.padding_minsize_default = padding_minsize;
    }
```

## Pair `picoquic/sacks.c:picoquic_sack_first_item`
C: `picoquic/sacks.c:68-72 picoquic_sack_first_item`
Rust: `rs/fq/src/internal.rs:8479-8494 picoquic_sack_first_item`

### C body
```c
{
    return picoquic_sack_item_value(picosplay_first(&sack_list->ack_tree));
}
```

### Rust body
```rust
pub fn picoquic_sack_last_item(sack_list: &SackList) -> Option<SackItemToken> {
    sack_list
        .ack_tree
        .last()
        .and_then(|st| sack_list.resolve_splay(st))
}
```

## Pair `picoquic/sacks.c:picoquic_sack_insert_item`
C: `picoquic/sacks.c:89-108 picoquic_sack_insert_item`
Rust: `rs/fq/src/internal.rs:8499-8506 picoquic_sack_insert_item`

### C body
```c
{
    int ret = 0;
    picoquic_sack_item_t* sack_new = (picoquic_sack_item_t*)malloc(sizeof(picoquic_sack_item_t));
    if (sack_new == NULL) {
        ret = -1;
    }
    else
    {
        memset(sack_new, 0, sizeof(picoquic_sack_item_t));
        sack_new->start_of_sack_range = range_min;
        sack_new->end_of_sack_range = range_max;
        sack_new->time_created = current_time;
        sack_list->rc[0].range_counts[0] += 1;
        sack_list->rc[1].range_counts[0] += 1;
        (void)picosplay_insert(&sack_list->ack_tree, sack_new);
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<(), crate::Error> {
    sack_list.insert_item(range_min, range_max, current_time)
}
```

## Pair `picoquic/sacks.c:picoquic_sack_find_range_below_number`
C: `picoquic/sacks.c:156-168 picoquic_sack_find_range_below_number`
Rust: `rs/fq/src/internal.rs:8103-8115 find_range_below_number`

### C body
```c
{
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(previous);
#endif
    picoquic_sack_item_t v = { 0 };
    v.start_of_sack_range = pn64;
    v.end_of_sack_range = pn64;
    return(picoquic_sack_item_value(picosplay_find_previous(&sack_list->ack_tree, &v)));
}
```

### Rust body
```rust
    fn item_above(&mut self, token: SackItemToken) -> Option<SackItemToken> {
        self.sack_previous_item(token)
    }
```

## Pair `picoquic/sacks.c:picoquic_sack_select_ack_ranges`
C: `picoquic/sacks.c:301-321 picoquic_sack_select_ack_ranges`
Rust: `rs/fq/src/internal.rs:8069-8097 select_ack_ranges`

### C body
```c
{
    int cumul_sent = 0;
    int first_sack_count = (first_sack == NULL) ? PICOQUIC_MAX_ACK_RANGE_REPEAT :
        first_sack->nb_times_sent[is_opportunistic];
    *nb_sent_max = PICOQUIC_MAX_ACK_RANGE_REPEAT;
    *nb_sent_max_skip = 0;

    for (int i = 0; i < PICOQUIC_MAX_ACK_RANGE_REPEAT; i++) {
        cumul_sent += sack_list->rc[is_opportunistic].range_counts[i];
        if (i == first_sack_count) {
            cumul_sent -= 1;
        }
        if (cumul_sent >= max_ranges) {
            *nb_sent_max = i;
            *nb_sent_max_skip = cumul_sent - max_ranges;
            break;
        }
    }
}
```

### Rust body
```rust
    ) {
        let idx = is_opportunistic.clamp(0, 1) as usize;
        let first_sack_count = first_sack
            .and_then(|tok| self.sack_items.get(tok))
            .map(|s| s.nb_times_sent[idx])
            .unwrap_or(MAX_ACK_RANGE_REPEAT as i32);
        let mut cumul_sent = 0;
        *nb_sent_max = MAX_ACK_RANGE_REPEAT as i32;
        *nb_sent_max_skip = 0;

        for i in 0..MAX_ACK_RANGE_REPEAT {
            cumul_sent += self.rc[idx].range_counts[i];
            if i as i32 == first_sack_count {
                cumul_sent -= 1;
            }
            if cumul_sent >= max_ranges {
                *nb_sent_max = i as i32;
                *nb_sent_max_skip = cumul_sent - max_ranges;
                break;
            }
        }
    }
```

## Pair `picoquic/sacks.c:picoquic_sack_list_first`
C: `picoquic/sacks.c:407-412 picoquic_sack_list_first`
Rust: `rs/fq/src/internal.rs:8538-8553 picoquic_sack_list_first`

### C body
```c
{
    picoquic_sack_item_t* first = picoquic_sack_first_item(sack_list);
    return (first == NULL)? UINT64_MAX:first->start_of_sack_range;
}
```

### Rust body
```rust
pub fn picoquic_sack_list_last(sack_list: &SackList) -> u64 {
    picoquic_sack_last_item(sack_list)
        .and_then(|tok| sack_list.sack_items.get(tok))
        .map(|item| item.end_of_sack_range)
        .unwrap_or(0)
}
```

## Pair `picoquic/sacks.c:picoquic_sack_list_reset`
C: `picoquic/sacks.c:439-447 picoquic_sack_list_reset`
Rust: `rs/fq/src/internal.rs:8558-8565 picoquic_sack_list_reset`

### C body
```c
{
    int ret = 0;
    picoquic_sack_list_free(sack_list);
    ret = picoquic_sack_insert_item(sack_list, range_min, range_max, current_time);
    return ret;
}
```

### Rust body
```rust
) -> Result<(), crate::Error> {
    sack_list.reset(range_min, range_max, current_time)
}
```

## Pair `picoquic/sacks.c:picoquic_sack_item_record_sent`
C: `picoquic/sacks.c:476-485 picoquic_sack_item_record_sent`
Rust: `rs/fq/src/internal.rs:8682-8698 item_record_sent`

### C body
```c
{
    if (sack_item->nb_times_sent[is_opportunistic] < PICOQUIC_MAX_ACK_RANGE_REPEAT) {
        sack_list->rc[is_opportunistic].range_counts[sack_item->nb_times_sent[is_opportunistic]] -= 1;
    }
    sack_item->nb_times_sent[is_opportunistic]++;
    if (sack_item->nb_times_sent[is_opportunistic] < PICOQUIC_MAX_ACK_RANGE_REPEAT) {
        sack_list->rc[is_opportunistic].range_counts[sack_item->nb_times_sent[is_opportunistic]] += 1;
    }
}
```

### Rust body
```rust
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
```

## Pair `picoquic/sender.c:picoquic_set_app_stream_ctx`
C: `picoquic/sender.c:80-93 picoquic_set_app_stream_ctx`
Rust: `rs/fq/src/lib.rs:3950-3959 set_app_stream_ctx`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    
    stream = picoquic_find_stream_for_writing(cnx, stream_id, &ret);
    if (ret == 0) {
        stream->app_stream_ctx = app_stream_ctx;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let stream = self.find_stream_for_writing(stream_id)?;
        let stream = self.streams.get_mut(stream).ok_or(Error::Memory)?;
        stream.app_stream_ctx = app_stream_ctx;
        Ok(())
    }
```

## Pair `picoquic/sender.c:picoquic_mark_active_stream`
C: `picoquic/sender.c:147-178 picoquic_mark_active_stream`
Rust: `rs/fq/src/lib.rs:3973-4007 mark_active_stream`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    stream = picoquic_find_stream_for_writing(cnx, stream_id, &ret);
    if (ret == 0) {
        if (is_active) {
            /* The call only fails if the stream was closed or reset */
            if (!stream->fin_requested && 
                (!stream->reset_requested || picoquic_check_sack_list(&stream->sack_list, 0, stream->reliable_size) == 0) &&
                cnx->callback_fn != NULL) {
                stream->app_stream_ctx = app_stream_ctx;
                if (!stream->is_active) {
                    stream->is_active = 1;
                    picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));
                }
            }
            else {
                ret = PICOQUIC_ERROR_CANNOT_SET_ACTIVE_STREAM;
            }
        }
        else {
            stream->is_active = 0;
            stream->app_stream_ctx = app_stream_ctx;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let stream_token = self.find_stream_for_writing(stream_id)?;
        let has_callback = self.callback_fn.is_some();
        let mut should_enqueue = false;

        {
            let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;

            if is_active {
                if !stream.fin_requested && !stream.reset_requested && has_callback {
                    stream.app_stream_ctx = v_stream_ctx;
                    stream.is_active = true;
                    if !stream.is_output_stream {
                        stream.is_output_stream = true;
                        should_enqueue = true;
                    }
                } else {
                    return Err(Error::Protocol(InternalError::CannotSetActiveStream as u64));
                }
            } else {
                stream.is_active = false;
                stream.app_stream_ctx = v_stream_ctx;
            }
        }

        if should_enqueue {
            enqueue_output_stream_token(self, stream_token);
        }
        Ok(())
    }
```

## Pair `picoquic/sender.c:picoquic_set_default_priority`
C: `picoquic/sender.c:207-211 picoquic_set_default_priority`
Rust: `rs/fq/src/lib.rs:4075-4082 set_default_priority`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_stream_priority = default_stream_priority;
}
```

### Rust body
```rust
    pub fn set_default_datagram_priority(&mut self, default_datagram_priority: u8) {
        self.default_datagram_priority = default_datagram_priority;
    }
```

## Pair `picoquic/sender.c:picoquic_add_to_stream`
C: `picoquic/sender.c:311-315 picoquic_add_to_stream`
Rust: `rs/fq/src/lib.rs:4147-4154 add_to_stream`

### C body
```c
{
    return picoquic_add_to_stream_with_ctx(cnx, stream_id, data, length, set_fin, NULL);
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        self.add_to_stream_with_ctx(stream_id, data, set_fin, None)
    }
```

## Pair `picoquic/sender.c:picoquic_reset_stream_at`
C: `picoquic/sender.c:379-407 picoquic_reset_stream_at`
Rust: `rs/fq/src/lib.rs:4230-4265 reset_stream_at`

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

## Pair `picoquic/sender.c:picoquic_discard_stream`
C: `picoquic/sender.c:460-490 picoquic_discard_stream`
Rust: `rs/fq/src/lib.rs:4356-4382 discard_stream`

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
        if (IS_BIDIR_STREAM_ID(stream_id) || !IS_CLIENT_STREAM_ID(stream_id)) {
            ret = picoquic_stop_sending(cnx, stream_id, local_stream_error);
            if (ret == PICOQUIC_ERROR_STREAM_ALREADY_CLOSED) {
                ret = 0;
            }
        }
        if (ret == 0 &&
            (IS_BIDIR_STREAM_ID(stream_id) || IS_CLIENT_STREAM_ID(stream_id))) {
            ret = picoquic_reset_stream(cnx, stream_id, local_stream_error);
            if (ret == PICOQUIC_ERROR_STREAM_ALREADY_CLOSED) {
                ret = 0;
            }
        }
        stream->app_stream_ctx = NULL;
        stream->is_discarded = 1;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn discard_stream(&mut self, stream_id: u64, local_stream_error: u16) -> Result<(), Error> {
        use crate::stream::StreamId;

        let stream_token = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let sid = StreamId(stream_id);
        if sid.is_bidir() || !sid.is_client() {
            match self.stop_sending(stream_id, local_stream_error as u64) {
                Err(Error::Protocol(code)) if code == InternalError::StreamAlreadyClosed as u64 => {
                }
                result => result?,
            }
        }
        if sid.is_bidir() || sid.is_client() {
            match self.reset_stream(stream_id, local_stream_error as u64) {
                Err(Error::Protocol(code)) if code == InternalError::StreamAlreadyClosed as u64 => {
                }
                result => result?,
            }
        }
        if let Some(stream) = self.streams.get_mut(stream_token) {
            stream.app_stream_ctx = None;
            stream.is_discarded = true;
        }
        Ok(())
    }
```

## Pair `picoquic/sender.c:picoquic_recycle_packet`
C: `picoquic/sender.c:561-575 picoquic_recycle_packet`
Rust: `rs/fq/src/internal.rs:1059-1171 create_packet`

### C body
```c
{
    if (packet != NULL) {
        if (quic->nb_packets_in_pool >= PICOQUIC_MAX_PACKETS_IN_POOL) {
            free(packet);
            quic->nb_packets_allocated--;
        }
        else {
            memset(packet, 0, offsetof(struct st_picoquic_packet_t, bytes));
            packet->packet_previous = quic->p_first_packet;
            quic->p_first_packet = packet;
            quic->nb_packets_in_pool++;
        }
    }
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

## Pair `picoquic/sender.c:picoquic_create_packet_header`
C: `picoquic/sender.c:711-790 picoquic_create_packet_header`
Rust: `rs/fq/src/internal.rs:6897-6926 create_packet_header`

### C body
```c
{
    size_t length = 0;

    /* Prepare the packet header */
    if (packet_type == picoquic_packet_1rtt_protected) {
        /* Create a short packet -- using 32 bit sequence numbers for now */
        uint8_t K = (cnx->key_phase_enc) ? 0x04 : 0;
        uint8_t C = 0x40; /* set the QUIC bit */
        size_t pn_l = 4;  /* default packet length to 4 bytes */

        if (cnx->do_grease_quic_bit) {
            /* we grease the quic bit if both local and remote agreed to do so */
            C &= (uint8_t)picoquic_public_random_64();
            cnx->quic_bit_greased |= (C == 0);
        }

        length = 0;
        bytes[length++] = (K | C | picoquic_spin_function_table[cnx->spin_policy].spinbit_outgoing(cnx));
        length += picoquic_format_connection_id(&bytes[length], PICOQUIC_MAX_PACKET_SIZE - length, tuple->p_remote_cnxid->cnx_id);

        *pn_offset = length;
        if (header_length > length && header_length < length + 4) {
            pn_l = header_length - length;
        }
        *pn_length = pn_l;
        bytes[0] |= (pn_l - 1);
        switch (pn_l) {
        case 1:
            bytes[length] = (uint8_t)sequence_number;
            break;
        case 2:
            picoformat_16(&bytes[length], (uint16_t)sequence_number);
            break;
        case 3:
            picoformat_24(&bytes[length], (uint32_t)sequence_number);
            break;
        default:
            picoformat_32(&bytes[length], (uint32_t)sequence_number);
            break;
        }
        length += pn_l;
    }
    else {
        /* Create a long packet */
        picoquic_connection_id_t * dest_cnx_id =
            (cnx->client_mode && (packet_type == picoquic_packet_initial ||
                packet_type == picoquic_packet_0rtt_protected)
                && picoquic_is_connection_id_null(&path_x->first_tuple->p_remote_cnxid->cnx_id)) ?
            &cnx->initial_cnxid : &path_x->first_tuple->p_remote_cnxid->cnx_id;
        picoquic_connection_id_t* srce_cnx_id = &path_x->first_tuple->p_local_cnxid->cnx_id;
        uint32_t version = ((cnx->cnx_state == picoquic_state_client_init || cnx->cnx_state == picoquic_state_client_init_sent) && packet_type == picoquic_packet_initial) ?
            cnx->proposed_version : picoquic_supported_versions[cnx->version_index].version;

        length = picoquic_create_long_header(
            packet_type,
            dest_cnx_id,
            srce_cnx_id,
            cnx->do_grease_quic_bit,
            version,
            cnx->version_index,
            sequence_number,
            cnx->retry_token_length,
            cnx->retry_token,
            bytes,
            pn_offset,
            pn_length);
    }

    return length;
}
```

### Rust body
```rust
    ) -> usize {
        // Delegate to create_packet_header_at using tuple's path context.
        // Find tuple's path index by matching tuple's unique_path_id.
        let path_idx = self
            .paths
            .iter()
            .position(|p| p.unique_path_id == tuple.unique_path_id)
            .unwrap_or(0);
        let _ = tuple;
        self.create_packet_header_at(
            packet_type,
            sequence_number,
            path_idx,
            0,
            header_length,
            bytes,
            pn_offset,
            pn_length,
        )
    }
```

## Pair `picoquic/sender.c:picoquic_protect_packet`
C: `picoquic/sender.c:897-994 picoquic_protect_packet`
Rust: `rs/fq/src/internal.rs:7009-7059 protect_packet`

### C body
```c
{
    size_t send_length;
    size_t h_length;
    size_t pn_offset = 0;
    size_t pn_length = 0;
    size_t aead_checksum_length = picoquic_aead_get_checksum_length(aead_context);
    size_t pn_iv_size = picoquic_pn_iv_size(pn_enc);
    size_t pn_sample_start;
    size_t pn_sample_end;
    uint8_t first_mask = 0x0F;

    if (tuple == NULL) {
        tuple = path_x->first_tuple;
    }

    /* Create the packet header just before encrypting the content */
    h_length = picoquic_create_packet_header(cnx, ptype,
        sequence_number, path_x, tuple, header_length, send_buffer, &pn_offset, &pn_length);

    if (h_length != header_length) {
#ifdef HUNTING_FOR_BUFFER_OVERFLOW
        char* x = NULL;
        *x++;
#endif
        picoquic_log_app_message(cnx, "BUFFER OVERFLOW? Packet header prediction fails, %zu instead of %zu\n", h_length, header_length);
    }

    // https://datatracker.ietf.org/doc/html/rfc9001#section-5.4.2
    // ensure there are enough iv bytes for pn encryption
    pn_sample_start = pn_offset + 4;
    pn_sample_end = pn_sample_start + pn_iv_size;
    length = picoquic_pad_to_target_length(bytes, length, pn_sample_end - aead_checksum_length); // discount aead checksum length added later

    if (ptype == picoquic_packet_1rtt_protected) {
        if (cnx->is_loss_bit_enabled_outgoing) {
            first_mask = 0x07;
            path_x->q_square++;
            if ((path_x->q_square & PICOQUIC_LOSS_BIT_Q_HALF_PERIOD) != 0) {
                send_buffer[0] |= 0x10;
            }
            if (path_x->nb_losses_found > path_x->nb_losses_reported) {
                send_buffer[0] |= 0x08;
                path_x->nb_losses_reported++;
            }
        }
        else {
            first_mask = 0x1F;
        }
    }

    /* Make sure that the payload length is encoded in the header */
    /* Using encryption, the "payload" length also includes the encrypted packet length */
    picoquic_update_payload_length(send_buffer, pn_offset, h_length - pn_length, length + aead_checksum_length);

    /* If fuzzing is required, apply it */
    if (cnx->quic->fuzz_fn != NULL) {
        if (h_length == header_length) {
            memcpy(bytes, send_buffer, header_length);
        }
        length = cnx->quic->fuzz_fn(cnx->quic->fuzz_ctx, cnx, bytes,
            send_buffer_max - aead_checksum_length, length, header_length);
        if (h_length == header_length) {
            memcpy(send_buffer, bytes, header_length);
        }
    }

    /* Encrypt the packet */
    if (cnx->is_multipath_enabled && ptype == picoquic_packet_1rtt_protected) {
        send_length = picoquic_aead_encrypt_mp(send_buffer + /* header_length */ h_length,
            bytes + header_length, length - header_length, path_x->unique_path_id,
            sequence_number, send_buffer, /* header_length */ h_length, aead_context);
    }
    else {
        send_length = picoquic_aead_encrypt_generic(send_buffer + /* header_length */ h_length,
            bytes + header_length, length - header_length,
            sequence_number, send_buffer, /* header_length */ h_length, aead_context);
    }

    send_length += /* header_length */ h_length;

    /* if needed, log the segment before header protection is applied */
    picoquic_log_outgoing_packet(cnx, path_x,
        bytes, sequence_number, pn_length, length,
        send_buffer, send_length, current_time);

    /* Next, encrypt the PN -- The sample is located after the pn_offset */
    picoquic_protect_packet_header(send_buffer, pn_offset, first_mask, pn_enc);

    return send_length;
}
```

### Rust body
```rust
    ) -> usize {
        if header_length > length || length > bytes.len() || header_length > send_buffer_max {
            return 0;
        }
        let header = &bytes[..header_length];
        let mut payload = bytes[header_length..length].to_vec();
        aead_context.encrypt(sequence_number, header, &mut payload);
        let packet_length = header_length + payload.len();
        if packet_length > send_buffer_max || packet_length > send_buffer.len() {
            return 0;
        }
        send_buffer[..header_length].copy_from_slice(header);
        send_buffer[header_length..packet_length].copy_from_slice(&payload);
        update_payload_length(
            send_buffer,
            header_length.saturating_sub(4),
            header_length.saturating_sub(4),
            packet_length,
        );
        let first_mask = if (send_buffer[0] & 0x80) != 0 {
            0x0f
        } else {
            0x1f
        };
        let pn_offset = if header_length >= 4 {
            header_length - 4
        } else {
            header_length
        };
        protect_packet_header(
            &mut send_buffer[..packet_length],
            pn_offset,
            first_mask,
            pn_enc,
        );
        packet_length
    }
```

## Pair `picoquic/sender.c:picoquic_insert_hole_in_send_sequence_if_needed`
C: `picoquic/sender.c:1133-1169 picoquic_insert_hole_in_send_sequence_if_needed`
Rust: `rs/fq/src/internal.rs:17096-17141 insert_hole_in_send_sequence_if_needed`

### C body
```c
{
    if (cnx->quic->sequence_hole_pseudo_period == 0) {
        /* Holing disabled. Set to max value, never worry about it later */
        pkt_ctx->next_sequence_hole = UINT64_MAX;
    } else if (cnx->cnx_state == picoquic_state_ready &&
        pkt_ctx->pending_last != NULL &&
        pkt_ctx->send_sequence >= pkt_ctx->next_sequence_hole) {
        if (pkt_ctx->next_sequence_hole != 0 &&
            !pkt_ctx->pending_last->is_ack_trap) {
            /* Insert a hole in sequence */
            picoquic_packet_t* packet = picoquic_create_packet(cnx->quic);

            if (packet != NULL) {
                packet->is_ack_trap = 1;
                packet->pc = picoquic_packet_context_application;
                packet->ptype = picoquic_packet_1rtt_protected;
                packet->send_time = current_time;
                packet->send_path = NULL;
                packet->sequence_number = pkt_ctx->send_sequence++;
                picoquic_queue_for_retransmit(cnx, path_x, packet, 0, current_time);
                *next_wake_time = current_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                /* Simulate local loss on the Q bit square function. */
                path_x->q_square++;
                cnx->nb_packet_holes_inserted++;
            }
        }
        /* Predict the next hole*/
        pkt_ctx->next_sequence_hole = pkt_ctx->send_sequence + 3 + picoquic_public_uniform_random(((uint64_t)cnx->quic->sequence_hole_pseudo_period)<<cnx->nb_packet_holes_inserted);
    }
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

## Pair `picoquic/sender.c:picoquic_is_cnx_backlog_empty`
C: `picoquic/sender.c:1310-1330 picoquic_is_cnx_backlog_empty`
Rust: `rs/fq/src/lib.rs:4825-4827 is_cnx_backlog_empty`

### C body
```c
{
    int backlog_empty = 1;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx->cnx_state < picoquic_state_ready) {
        backlog_empty = picoquic_is_pkt_ctx_backlog_empty(&cnx->pkt_ctx[picoquic_packet_context_initial]) &&
            picoquic_is_pkt_ctx_backlog_empty(&cnx->pkt_ctx[picoquic_packet_context_handshake]);
    }

    if (cnx->is_multipath_enabled) {
        for (int i=0; backlog_empty && i < cnx->nb_paths; i++) {
            backlog_empty &= picoquic_is_pkt_ctx_backlog_empty(&cnx->path[i]->pkt_ctx);
        }
    }
    else if (backlog_empty) {
        backlog_empty = picoquic_is_pkt_ctx_backlog_empty(&cnx->pkt_ctx[picoquic_packet_context_application]);
    }

    return backlog_empty;
}
```

### Rust body
```rust
    pub fn is_cnx_backlog_empty(&self) -> bool {
        self.nb_bytes_queued == 0 && self.misc_frames.is_empty() && self.output_streams.is_empty()
    }
```

## Pair `picoquic/sender.c:picoquic_next_mtu_probe_length`
C: `picoquic/sender.c:1545-1585 picoquic_next_mtu_probe_length`
Rust: `rs/fq/src/internal.rs:16793-16796 next_mtu_probe_length`

### C body
```c
{
    size_t probe_length;

    if (path_x->send_mtu_max_tried == 0) {
        if (cnx->remote_parameters.max_packet_size > 0) {
            probe_length = cnx->remote_parameters.max_packet_size;

            if (cnx->quic->mtu_max > 0 && probe_length >
                cnx->quic->mtu_max - PICOQUIC_MTU_OVERHEAD((struct sockaddr*)&path_x->first_tuple->peer_addr)) {
                probe_length = cnx->quic->mtu_max - PICOQUIC_MTU_OVERHEAD((struct sockaddr*)&path_x->first_tuple->peer_addr);
            }
            else if (probe_length > PICOQUIC_MAX_PACKET_SIZE) {
                probe_length = PICOQUIC_MAX_PACKET_SIZE;
            }
            if (probe_length < path_x->send_mtu) {
                probe_length = path_x->send_mtu;
            }
        }
        else if (cnx->quic->mtu_max > 0) {
            probe_length = cnx->quic->mtu_max - PICOQUIC_MTU_OVERHEAD((struct sockaddr*)&path_x->first_tuple->peer_addr);
        }
        else {
            probe_length = PICOQUIC_PRACTICAL_MAX_MTU;
        }
    }
    else {
        if (path_x->send_mtu_max_tried > 1500) {
            probe_length = 1500;
        }
        else if (path_x->send_mtu_max_tried > 1400) {
            probe_length = 1400;
        }
        else {
            probe_length = (path_x->send_mtu + path_x->send_mtu_max_tried) / 2;
        }
    }

    return probe_length;
}
```

### Rust body
```rust
        let overhead = if path.tuples.first().is_some_and(|t| t.peer_addr.is_ipv6()) {
            48usize // IPv6 (40-byte IP header + 8-byte UDP header)
        } else {
```
