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

## Pair `picoquic/quicctx.c:picoquic_uniform_random`
C: `picoquic/quicctx.c:5600-5604 picoquic_uniform_random`
Rust: `rs/fq/src/lib.rs:5168-5179 picoquic_uniform_random`

### C body
```c
{
    return picoquic_public_uniform_random(rnd_max);
}
```

### Rust body
```rust
pub fn picoquic_uniform_random(rnd_max: u64) -> u64 {
    if rnd_max == 0 {
        return 0;
    }
    let rnd_min = u64::MAX % rnd_max;
    loop {
        let rnd = public_random_64();
        if rnd >= rnd_min {
            return rnd % rnd_max;
        }
    }
}
```

## Pair `picoquic/sacks.c:picoquic_sack_next_item`
C: `picoquic/sacks.c:79-82 picoquic_sack_next_item`
Rust: `rs/fq/src/internal.rs:8413-8417 sack_next_item`

### C body
```c
{
    return picoquic_sack_item_value(picosplay_next(&sack->node));
}
```

### Rust body
```rust
    pub fn sack_next_item(&mut self, sack: SackItemToken) -> Option<SackItemToken> {
        let st = self.sack_items.get(sack)?.ack_tree_membership?;
        let next_st = self.ack_tree.previous(st)?;
        self.ack_tree.get(next_st).copied()
    }
```

## Pair `picoquic/sacks.c:picoquic_ack_ctx_from_cnx_context`
C: `picoquic/sacks.c:128-147 picoquic_ack_ctx_from_cnx_context`
Rust: `rs/fq/src/internal.rs:8434-8450 ack_ctx_from_cnx_context`

### C body
```c
{
    picoquic_ack_context_t* ack_ctx = &cnx->ack_ctx[pc];

    if (cnx->is_multipath_enabled && pc == picoquic_packet_context_application) {
        int path_id = 0;
        if (l_cid != NULL) {
            path_id = picoquic_find_path_by_unique_id(cnx, l_cid->path_id);
        }

        if (path_id >= 0) {
            ack_ctx = &cnx->path[path_id]->ack_ctx;
        }
    }
    return ack_ctx;

}
```

### Rust body
```rust
        if self.is_multipath_enabled && packet_context == PacketContext::Application {
            let path_id = local_connection_id
                .and_then(|tok| self.local_connection_ids.get(tok))
                .map(|l| l.path_id)
                .unwrap_or(0);
            let path_idx = self.paths.iter().position(|p| p.unique_path_id == path_id);
            if let Some(idx) = path_idx {
                return Some(&mut self.paths[idx].ack_ctx);
            }
        }
```

## Pair `picoquic/sacks.c:picoquic_update_sack_list`
C: `picoquic/sacks.c:197-256 picoquic_update_sack_list`
Rust: `rs/fq/src/internal.rs:8189-8275 update_sack_list`

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

## Pair `picoquic/sacks.c:picoquic_process_ack_of_ack_range`
C: `picoquic/sacks.c:347-380 picoquic_process_ack_of_ack_range`
Rust: `rs/fq/src/internal.rs:8304-8354 process_ack_of_ack_range`

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

## Pair `picoquic/sacks.c:picoquic_sack_list_first_range`
C: `picoquic/sacks.c:422-428 picoquic_sack_list_first_range`
Rust: `rs/fq/src/internal.rs:8528-8533 picoquic_sack_list_first_range`

### C body
```c
{
    picoquic_sack_item_t* first = picoquic_sack_first_item(sack_list);
    return(first == NULL) ? NULL : picoquic_sack_item_value(picosplay_next(&first->node));
}
```

### Rust body
```rust
pub fn picoquic_sack_list_first_range(sack_list: &SackList) -> Option<SackItemToken> {
    let first = picoquic_sack_first_item(sack_list)?;
    let first_membership = sack_list.sack_items.get(first)?.ack_tree_membership?;
    let next = sack_list.ack_tree.next(first_membership)?;
    sack_list.resolve_splay(next)
}
```

## Pair `picoquic/sacks.c:picoquic_sack_item_range_end`
C: `picoquic/sacks.c:466-469 picoquic_sack_item_range_end`
Rust: `rs/fq/src/internal.rs:8667-8675 range_end`

### C body
```c
{
    return sack_item->end_of_sack_range;
}
```

### Rust body
```rust
    pub fn nb_times_sent(&self, is_opportunistic: i32) -> i32 {
        self.nb_times_sent[is_opportunistic.clamp(0, 1) as usize]
    }
```

## Pair `picoquic/sacks.c:picoquic_sack_list_size`
C: `picoquic/sacks.c:498-501 picoquic_sack_list_size`
Rust: `rs/fq/src/internal.rs:8653-8676 size`

### C body
```c
{
    return (size_t)sack_list->ack_tree.size;
}
```

### Rust body
```rust
impl SackItem {
    /// Inclusive start of this SACK range.  C:
    /// `sack_item_range_start`.
    pub fn range_start(&self) -> u64 {
        self.start_of_sack_range
    }

    /// Exclusive end of this SACK range.  C:
    /// `sack_item_range_end`.
    pub fn range_end(&self) -> u64 {
        self.end_of_sack_range
    }

    /// Number of times this range has been sent in an ACK frame.
    /// C: `sack_item_nb_times_sent`.
    pub fn nb_times_sent(&self, is_opportunistic: i32) -> i32 {
        self.nb_times_sent[is_opportunistic.clamp(0, 1) as usize]
    }
}
```

## Pair `picoquic/sender.c:picoquic_mark_datagram_ready`
C: `picoquic/sender.c:106-122 picoquic_mark_datagram_ready`
Rust: `rs/fq/src/lib.rs:4385-4388 mark_datagram_ready`

### C body
```c
{
    int ret = 0;
    int was_ready = cnx->is_datagram_ready;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    cnx->is_datagram_ready = is_ready;
    if (!was_ready && is_ready) {
        if (cnx->remote_parameters.max_datagram_frame_size == 0) {
            ret = -1;
        }
        else {
            picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn mark_datagram_ready(&mut self, is_ready: bool) -> Result<(), Error> {
        self.is_datagram_ready = is_ready;
        Ok(())
    }
```

## Pair `picoquic/sender.c:picoquic_set_default_datagram_priority`
C: `picoquic/sender.c:195-199 picoquic_set_default_datagram_priority`
Rust: `rs/fq/src/lib.rs:4080-4082 set_default_datagram_priority`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->default_datagram_priority = default_datagram_priority;
}
```

### Rust body
```rust
    pub fn set_default_datagram_priority(&mut self, default_datagram_priority: u8) {
        self.default_datagram_priority = default_datagram_priority;
    }
```

## Pair `picoquic/sender.c:picoquic_mark_high_priority_stream`
C: `picoquic/sender.c:228-243 picoquic_mark_high_priority_stream`
Rust: `rs/fq/src/lib.rs:4046-4064 mark_high_priority_stream`

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

## Pair `picoquic/sender.c:picoquic_open_flow_control`
C: `picoquic/sender.c:332-367 picoquic_open_flow_control`
Rust: `rs/fq/src/lib.rs:4269-4303 open_flow_control`

### C body
```c
{
    int ret = 0;
    uint8_t buffer[512];
    size_t length = 0;
    size_t consumed = 0;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx->cnx_state == picoquic_state_ready && cnx->quic->max_data_limit == 0){
        /* Only send the update in ready state, so that the misc frame is not picked by the
         * wrong transport context.
         * TODO: find way to queue the update so it is only sent as 0RTT or 1RTT packet.
         */
        picoquic_stream_head_t* stream = picoquic_find_stream(cnx, stream_id);
        if (stream == NULL) {
            ret = PICOQUIC_ERROR_INVALID_STREAM_ID;
        }
        else {
            uint64_t max_required = stream->consumed_offset + expected_data_size;
            uint8_t* bytes_max = buffer + sizeof(buffer);
            int more_data = 0;
            int is_pure_ack = 1;

            if (max_required > stream->maxdata_local) {
                uint8_t* bytes_next = picoquic_format_max_stream_data_frame(cnx, stream, buffer + consumed, bytes_max, &more_data, &is_pure_ack, max_required);
                bytes_next = picoquic_format_max_data_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack, expected_data_size);
                if ((length = bytes_next - buffer) > 0) {
                    ret = picoquic_queue_misc_frame(cnx, buffer, length, is_pure_ack,
                        picoquic_packet_context_application);
                }
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        if self.connection_state != State::Ready {
            return Ok(());
        }

        let stream_token = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let new_stream_max = {
            let stream = self.streams.get_mut(stream_token).ok_or(Error::Memory)?;
            let max_required = stream.consumed_offset.saturating_add(expected_data_size);
            if max_required <= stream.maxdata_local {
                return Ok(());
            }
            stream.maxdata_local = max_required;
            stream.maxdata_local_acked = max_required;
            stream.max_stream_updated = false;
            max_required
        };
        self.max_stream_data_local = self.max_stream_data_local.max(new_stream_max);

        let new_data_max = self.maxdata_local.saturating_add(expected_data_size);
        self.maxdata_local = new_data_max;
        let mut frame = Vec::new();
        append_varint(&mut frame, crate::frames::FrameType::MaxStreamData as u64);
        append_varint(&mut frame, stream_id);
        append_varint(&mut frame, new_stream_max);
        append_varint(&mut frame, crate::frames::FrameType::MaxData as u64);
        append_varint(&mut frame, new_data_max);
        self.queue_misc_frame(&frame, false, PacketContext::Application)
    }
```

## Pair `picoquic/sender.c:picoquic_get_next_local_stream_id`
C: `picoquic/sender.c:414-428 picoquic_get_next_local_stream_id`
Rust: `rs/fq/src/lib.rs:4833-4841 get_next_local_stream_id`

### C body
```c
{
    /* This code could be written as:
     * int stream_type_id = ((cnx->client_mode ^ 1) | ((is_unidir) ? 2 : 0)); 
     * but Visual Studio produces an obnoxious error message about
     * mixing bitwise or and logical or. */
    int stream_type_id = cnx->client_mode ^ 1;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (is_unidir) {
        stream_type_id |= 2;
    }

    return cnx->next_stream_id[stream_type_id];     
}
```

### Rust body
```rust
    pub fn primary_path_rtt_max(&self) -> u64 {
        self.paths.first().map(|p| p.rtt_max.ticks()).unwrap_or(0)
    }
```

## Pair `picoquic/sender.c:picoquic_pad_to_policy`
C: `picoquic/sender.c:506-526 picoquic_pad_to_policy`
Rust: `rs/fq/src/internal.rs:1177-1191 pad_to_policy`

### C body
```c
{
    size_t target = cnx->padding_minsize;

    if (length > target && cnx->padding_multiple != 0) {
        uint32_t delta = (length - target) % cnx->padding_multiple;

        if (delta == 0) {
            target = length;
        }
        else {
            target = length + cnx->padding_multiple - delta;
        }
    }

    if (target > max_length) {
        target = max_length;
    }

    return picoquic_pad_to_target_length(bytes, length, target);
}
```

### Rust body
```rust
    pub fn pad_to_policy(&mut self, bytes: &mut [u8], length: usize, max_length: u32) -> usize {
        let mut target = self.padding_minsize as usize;
        if length > target && self.padding_multiple != 0 {
            let delta = (length - target) % self.padding_multiple as usize;
            if delta == 0 {
                target = length;
            } else {
                target = length + self.padding_multiple as usize - delta;
            }
        }
        if target > max_length as usize {
            target = max_length as usize;
        }
        pad_to_target_length(bytes, length, target)
    }
```

## Pair `picoquic/sender.c:picoquic_create_long_packet_type`
C: `picoquic/sender.c:586-639 picoquic_create_long_packet_type`
Rust: `rs/fq/src/internal.rs:6533-6571 create_long_packet_type`

### C body
```c
{
    uint8_t flags = 0xFF; /* Will cause an error... */
    if (version_index < 0) {
        version_index = 0;
    }
    switch (picoquic_supported_versions[version_index].packet_type_version) {
    case PICOQUIC_V1_VERSION:
        switch (pt) {
        case picoquic_packet_initial:
            flags = 0xC3;
            break;
        case picoquic_packet_0rtt_protected:
            flags = 0xD3;
            break;
        case picoquic_packet_handshake:
            flags = 0xE3;
            break;
        case picoquic_packet_retry:
            /* Do not set PP in retry header, the bits are later used for ODCIL */
            flags = 0xF0;
            break;
        default:
            break;
        }
        break;
    case PICOQUIC_V2_VERSION:
        /* Initial packets use a packet type field of 0b01. */
        /* 0-RTT packets use a packet type field of 0b10. */
        /* Handshake packets use a packet type field of 0b11. */
        /* Retry packets use a packet type field of 0b00.*/
        switch (pt) {
        case picoquic_packet_initial:
            flags = 0xD3;
            break;
        case picoquic_packet_0rtt_protected:
            flags = 0xE3;
            break;
        case picoquic_packet_handshake:
            flags = 0xF3;
            break;
        case picoquic_packet_retry:
            /* Do not set PP in retry header, the bits are later used for ODCIL */
            flags = 0xC0;
            break;
        default:
            break;
        }
        break;
    default:
        break;
    }
    return flags;
}
```

### Rust body
```rust
pub fn create_long_packet_type(pt: PacketType, version_index: i32) -> u8 {
    let version_index = version_index.max(0);
    let version = match version_index {
        0 => Version::V1,
        1 => Version::V2,
        2 => Version::V2Draft,
        3 => Version::PostIesg,
        4 => Version::TwentyFirstInterop,
        5 => Version::TwentiethInterop,
        6 => Version::TwentiethPreInterop,
        7 => Version::NineteenthInterop,
        8 => Version::NineteenthBisInterop,
        9 => Version::EighteenthInterop,
        10 => Version::SeventeenthInterop,
        11 => Version::InternalTest2,
        12 => Version::InternalTest1,
        _ => Version::V1,
    };
    match version.parameters().packet_type_version {
        // QUIC v1 packet-type encoding (RFC 9000 §17.2).
        0x0000_0001 => match pt {
            PacketType::Initial => 0xC3,
            PacketType::ZeroRttProtected => 0xD3,
            PacketType::Handshake => 0xE3,
            // Retry: PP bits are left clear (used later for ODCIL).
            PacketType::Retry => 0xF0,
            _ => 0xFF,
        },
        // QUIC v2 packet-type encoding (RFC 9369 §3.2) — type bits rotated.
        0x6b33_43cf => match pt {
            PacketType::Initial => 0xD3,
            PacketType::ZeroRttProtected => 0xE3,
            PacketType::Handshake => 0xF3,
            PacketType::Retry => 0xC0,
            _ => 0xFF,
        },
        _ => 0xFF,
    }
}
```

## Pair `picoquic/sender.c:picoquic_get_checksum_length`
C: `picoquic/sender.c:857-872 picoquic_get_checksum_length`
Rust: `rs/fq/src/internal.rs:6976-6982 get_checksum_length`

### C body
```c
{
    size_t ret = 16;

    if (cnx->crypto_context[epoch].aead_encrypt != NULL) {
        ret = picoquic_aead_get_checksum_length(cnx->crypto_context[epoch].aead_encrypt);
    }
    else {
        DBG_PRINTF("Try getting checksum for empty context, epoch %d", epoch);
    }

    return ret;
}
```

### Rust body
```rust
    pub fn get_checksum_length(&self, is_cleartext_mode: Epoch) -> usize {
        // C: picoquic_get_checksum_length — returns the AEAD tag length.
        // Cleartext (initial epoch) uses 16 bytes (AES-128-GCM tag).
        // All other epochs also use 16 bytes in practice.
        let _ = is_cleartext_mode;
        16
    }
```

## Pair `picoquic/sender.c:picoquic_dequeue_retransmit_packet`
C: `picoquic/sender.c:1033-1105 picoquic_dequeue_retransmit_packet`
Rust: `rs/fq/src/internal.rs:5609-5625 dequeue_retransmit_packet`

### C body
```c
{
    size_t dequeued_length = p->length + p->checksum_overhead;

    if (p->is_queued_for_retransmit) {
        /* Remove from list */
        if (p->packet_next == NULL) {
            pkt_ctx->pending_last = p->packet_previous;
        }
        else {
            p->packet_next->packet_previous = p->packet_previous;
        }

        if (p->packet_previous == NULL) {
            pkt_ctx->pending_first = p->packet_next;
        }
        else {
            p->packet_previous->packet_next = p->packet_next;
        }
        p->is_queued_for_retransmit = 0;
    }

    /* Account for bytes in transit, for congestion control */

    if (p->send_path != NULL && !p->is_ack_trap) {
        if (p->send_path->bytes_in_transit > dequeued_length) {
            p->send_path->bytes_in_transit -= dequeued_length;
        }
        else {
            p->send_path->bytes_in_transit = 0;
        }
        p->send_path->is_cc_data_updated = 1;
    }

    /* Replace head of preemptive repeat list if it was this packet. */
    if (pkt_ctx->preemptive_repeat_ptr == p) {
        pkt_ctx->preemptive_repeat_ptr = p->packet_next;
    }

    if (should_free || p->is_ack_trap) {
        if (add_to_data_repeat_queue) {
            picoquic_queue_data_repeat_packet(cnx, p);
        }
        else {
            picoquic_recycle_packet(cnx->quic, p);
            p = NULL;
        }
    } 
    else {
        p->packet_previous = NULL;
        /* add this packet to the retransmitted list */
        if (pkt_ctx->retransmitted_oldest == NULL) {
            pkt_ctx->retransmitted_newest = p;
            pkt_ctx->retransmitted_oldest = p;
            p->packet_next = NULL;
        }
        else {
            pkt_ctx->retransmitted_newest->packet_previous = p;
            p->packet_next = pkt_ctx->retransmitted_newest;
            pkt_ctx->retransmitted_newest = p;
        }
        pkt_ctx->retransmitted_queue_size += 1;
        p->is_queued_for_spurious_detection = 1;

        if (add_to_data_repeat_queue) {
            picoquic_queue_data_repeat_packet(cnx, p);
        }
    }

    return p;
}
```

### Rust body
```rust
            .or_else(|| {
                pkt_ctx
                    .pending
                    .iter()
                    .find_map(|(seq, tok)| (*tok == packet).then_some(*seq))
            })?;
```

## Pair `picoquic/sender.c:picoquic_finalize_and_protect_packet`
C: `picoquic/sender.c:1262-1272 picoquic_finalize_and_protect_packet`
Rust: `rs/fq/src/internal.rs:7500-7578 finalize_and_protect_packet`

### C body
```c
{
    picoquic_finalize_and_protect_packet_tuple(cnx, packet, ret,
        length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_max,
        path_x, current_time, path_x->first_tuple);
}
```

### Rust body
```rust
    ) {
        use core::net::{IpAddr, Ipv4Addr};
        let default_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let mut tuple_copy = path_x.tuples.first().map_or(
            Tuple {
                unique_path_id: path_x.unique_path_id,
                peer_addr: default_addr,
                local_addr: default_addr,
                if_index: 0,
                observed_addr: default_addr,
                remote_connection_id_index: None,
                local_connection_id: None,
                nb_observed_repeat: 0,
                observed_time: Instant::from_ticks(0),
                challenge_response: 0,
                challenge: [0; CHALLENGE_REPEAT_MAX],
                challenge_time: Instant::from_ticks(0),
                demotion_time: Instant::from_ticks(0),
                challenge_time_first: Instant::from_ticks(0),
                is_nat_rebinding: 0,
                challenge_repeat_count: 0,
                is_backup: 0,
                challenge_required: false,
                challenge_verified: false,
                challenge_failed: false,
                response_required: false,
                to_preferred_address: false,
            },
            |tuple| Tuple {
                unique_path_id: tuple.unique_path_id,
                peer_addr: tuple.peer_addr,
                local_addr: tuple.local_addr,
                if_index: tuple.if_index,
                observed_addr: tuple.observed_addr,
                remote_connection_id_index: tuple.remote_connection_id_index,
                local_connection_id: tuple.local_connection_id,
                nb_observed_repeat: tuple.nb_observed_repeat,
                observed_time: tuple.observed_time,
                challenge_response: tuple.challenge_response,
                challenge: tuple.challenge,
                challenge_time: tuple.challenge_time,
                demotion_time: tuple.demotion_time,
                challenge_time_first: tuple.challenge_time_first,
                is_nat_rebinding: tuple.is_nat_rebinding,
                challenge_repeat_count: tuple.challenge_repeat_count,
                is_backup: tuple.is_backup,
                challenge_required: tuple.challenge_required,
                challenge_verified: tuple.challenge_verified,
                challenge_failed: tuple.challenge_failed,
                response_required: tuple.response_required,
                to_preferred_address: tuple.to_preferred_address,
            },
        );
        self.finalize_and_protect_packet_tuple(
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
            &mut tuple_copy,
        );
    }
```

## Pair `picoquic/sender.c:picoquic_preemptive_retransmit_in_context`
C: `picoquic/sender.c:1421-1492 picoquic_preemptive_retransmit_in_context`
Rust: `rs/fq/src/internal.rs:17518-17543 picoquic_preemptive_retransmit_in_context`

### C body
```c
{
    /* If there is a single packet context for application frames,
     * the code just has to track the preemptive_repeat_ptr for
     * that context. If there are multiple paths, we need to consider
     * packets from every plausible path.
     */
    int ret = 0;

    /* Check that the connection is still active before adding more preemptive repeats */
    if (cnx->latest_progress_time + rtt < current_time ||
        cnx->latest_receive_time + 2*rtt < current_time) {
        return 0;
    }

    /* Find the first packet that might be repeated */
    if (pkt_ctx->preemptive_repeat_ptr == NULL) {
        pkt_ctx->preemptive_repeat_ptr = pkt_ctx->pending_first;
    }
    /* Skip all packets that are too old to be repeated */
    while (pkt_ctx->preemptive_repeat_ptr != NULL) {
        if (pkt_ctx->preemptive_repeat_ptr->send_time + rtt / 2 >= current_time) {
            break;
        }
        pkt_ctx->preemptive_repeat_ptr = pkt_ctx->preemptive_repeat_ptr->packet_next;
    }
    /* Try to format the repeated packet */
    while (pkt_ctx->preemptive_repeat_ptr != NULL) {
        uint64_t early_delay = (rtt > 8 * PICOQUIC_ACK_DELAY_MAX) ? rtt / 8 : PICOQUIC_ACK_DELAY_MAX;
        uint64_t early_time = pkt_ctx->preemptive_repeat_ptr->send_time + early_delay;

        if (!pkt_ctx->preemptive_repeat_ptr->was_preemptively_repeated) {
            if (early_time > current_time) {
                /* Wait until the next repeat */
                if (*next_wake_time > early_time) {
                    *next_wake_time = early_time;
                    SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                }
                break;
            }
            if (test_only) {
                *more_data = 1;
                break;
            }
            ret = picoquic_preemptive_retransmit_packet(pkt_ctx->preemptive_repeat_ptr, cnx,
                new_bytes, send_buffer_max_minus_checksum, length, has_data);
            if (ret != 0) {
                break;
            }
        }
        pkt_ctx->preemptive_repeat_ptr = pkt_ctx->preemptive_repeat_ptr->packet_next;
        if (*has_data) {
            cnx->nb_preemptive_repeat++;
            if (pkt_ctx->preemptive_repeat_ptr != NULL) {
                *more_data = 1;
            }
            break;
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        self.preemptive_retransmit_in_selection(
            PacketContextSelection::Connection(pc),
            rtt,
            current_time,
            next_wake_time,
            new_bytes,
            send_buffer_max_minus_checksum,
            length,
            has_data,
            more_data,
            test_only,
        )
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_mtu_probe`
C: `picoquic/sender.c:1630-1647 picoquic_prepare_mtu_probe`
Rust: `rs/fq/src/internal.rs:17177-17198 prepare_mtu_probe`

### C body
```c
{
    size_t probe_length = picoquic_next_mtu_probe_length(cnx, path_x);
    size_t length = header_length;

    if (probe_length > bytes_max) {
        probe_length = bytes_max;
    }

    bytes[length++] = picoquic_frame_type_ping;
    memset(&bytes[length], 0, probe_length - checksum_length - length);

    return probe_length - checksum_length;
}
```

### Rust body
```rust
    ) -> usize {
        let mut probe_length = self.next_mtu_probe_length(path_x, self.quic_mtu_max());
        probe_length = probe_length.min(bytes_max).min(bytes.len());
        if probe_length <= checksum_length
            || header_length >= probe_length.saturating_sub(checksum_length)
        {
            return header_length.min(probe_length.saturating_sub(checksum_length));
        }
        let payload_end = probe_length - checksum_length;
        bytes[header_length] = crate::frames::FrameType::Ping as u8;
        if header_length + 1 < payload_end {
            bytes[header_length + 1..payload_end].fill(crate::frames::FrameType::Padding as u8);
        }
        payload_end
    }
```
