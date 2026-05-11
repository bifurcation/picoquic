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

## Pair `picoquic/quicctx.c:picoquic_process_version_upgrade`
C: `picoquic/quicctx.c:5572-5598 picoquic_process_version_upgrade`
Rust: `rs/fq/src/internal.rs:15669-15700 process_version_upgrade`

### C body
```c
{
    int ret = -1;
    /* Check whether upgrade is supported */
    if (new_version_index == old_version_index) {
        /* not an upgrade, nothing to do. */
        ret = 0;
    } else if (picoquic_supported_versions[new_version_index].upgrade_from != NULL) {
        int i = 0;

        while (picoquic_supported_versions[new_version_index].upgrade_from[i] != 0) {
            if (picoquic_supported_versions[new_version_index].upgrade_from[i] ==
                picoquic_supported_versions[old_version_index].version) {
                /* Supported */
                ret = 0;
                if (cnx != NULL) {
                    /* Install the new keys */
                    cnx->version_index = new_version_index;
                    picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_initial]);
                    ret = picoquic_setup_initial_traffic_keys(cnx);
                    break;
                }
            }
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        self.rejected_version = if old_version_index >= 0 {
            self.proposed_version
        } else {
            self.rejected_version
        };
        self.version_index = new_version_index;
        let version = match new_version_index {
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
        self.proposed_version = version as u32;
        self.desired_version = self.proposed_version;
        self.local_parameters.version_negotiation.current = self.proposed_version;
        0
    }
```

## Pair `picoquic/sacks.c:picoquic_sack_last_item`
C: `picoquic/sacks.c:74-77 picoquic_sack_last_item`
Rust: `rs/fq/src/internal.rs:8489-8506 picoquic_sack_last_item`

### C body
```c
{
    return picoquic_sack_item_value(picosplay_last(&sack_list->ack_tree));
}
```

### Rust body
```rust
) -> Result<(), crate::Error> {
    sack_list.insert_item(range_min, range_max, current_time)
}
```

## Pair `picoquic/sacks.c:picoquic_sack_list_is_empty`
C: `picoquic/sacks.c:121-126 picoquic_sack_list_is_empty`
Rust: `rs/fq/src/internal.rs:8405-8418 is_empty`

### C body
```c
{
    return (sack_list->ack_tree.size == 0);
}
```

### Rust body
```rust
impl SackList {
    /// Splay-tree successor of `sack` in `list`.  C: `sack_next_item`.
    /// In picoquic, "next" means the range with the next lower PN (predecessor in our key-order).
    pub fn sack_next_item(&mut self, sack: SackItemToken) -> Option<SackItemToken> {
        let st = self.sack_items.get(sack)?.ack_tree_membership?;
        let next_st = self.ack_tree.previous(st)?;
        self.ack_tree.get(next_st).copied()
    }
}
```

## Pair `picoquic/sacks.c:picoquic_is_pn_already_received`
C: `picoquic/sacks.c:170-189 picoquic_is_pn_already_received`
Rust: `rs/fq/src/internal.rs:8015-8043 is_pn_already_received`

### C body
```c
{
    int is_received = 0;
    picoquic_sack_list_t* sack_list = picoquic_sack_list_from_cnx_context(cnx, pc, l_cid);

    if (sack_list->horizon_delay > 0 && pn64 < sack_list->ack_horizon) {
        is_received = 1;
    }
    else {
        picoquic_sack_item_t* sack_found = picoquic_sack_find_range_below_number(sack_list, NULL, pn64);
        is_received = (sack_found != NULL && pn64 <= sack_found->end_of_sack_range);
    }
    return is_received;
}
```

### Rust body
```rust
    ) -> bool {
        // C: picoquic_is_pn_already_received — check SACK list for duplicate PN.
        let ack_ctx = &self.ack_ctx[pc as usize];
        // Check: is pn64 covered by any SACK range?
        let mut st_opt = ack_ctx.sack_list.ack_tree.last();
        while let Some(st) = st_opt {
            let item_tok = match ack_ctx.sack_list.ack_tree.get(st).copied() {
                Some(t) => t,
                None => break,
            };
            let item = match ack_ctx.sack_list.sack_items.get(item_tok) {
                Some(i) => i,
                None => break,
            };
            if item.start_of_sack_range <= pn64 && item.end_of_sack_range >= pn64 {
                return true;
            }
            if item.end_of_sack_range < pn64 {
                break;
            }
            st_opt = ack_ctx.sack_list.ack_tree.previous(st);
        }
        false
    }
```

## Pair `picoquic/sacks.c:picoquic_check_sack_list`
C: `picoquic/sacks.c:323-340 picoquic_check_sack_list`
Rust: `rs/fq/src/internal.rs:8512-8520 picoquic_check_sack_list`

### C body
```c
{
    int ret = 0;
    picoquic_sack_item_t* sack = picoquic_sack_find_range_below_number(sack_list, NULL, pn64_min);

    if (sack != NULL) {
        if (pn64_max <= sack->end_of_sack_range) {
            ret = -1;
        }
    }
    return ret;
}
```

### Rust body
```rust
    {
        -1
    } else {
```

## Pair `picoquic/sacks.c:picoquic_sack_list_last`
C: `picoquic/sacks.c:414-420 picoquic_sack_list_last`
Rust: `rs/fq/src/internal.rs:8548-8565 picoquic_sack_list_last`

### C body
```c
{
    picoquic_sack_item_t* last = picoquic_sack_last_item(sack_list);
    return (last == NULL) ? 0 : last->end_of_sack_range;
}
```

### Rust body
```rust
) -> Result<(), crate::Error> {
    sack_list.reset(range_min, range_max, current_time)
}
```

## Pair `picoquic/sacks.c:picoquic_sack_item_range_start`
C: `picoquic/sacks.c:459-464 picoquic_sack_item_range_start`
Rust: `rs/fq/src/internal.rs:8661-8669 range_start`

### C body
```c
{
    return sack_item->start_of_sack_range;
}
```

### Rust body
```rust
    pub fn range_end(&self) -> u64 {
        self.end_of_sack_range
    }
```

## Pair `picoquic/sacks.c:picoquic_sack_item_record_reset`
C: `picoquic/sacks.c:487-496 picoquic_sack_item_record_reset`
Rust: `rs/fq/src/internal.rs:8703-8707 item_record_reset`

### C body
```c
{
    for (int r = 0; r < 2; r++) {
        if (sack_item->nb_times_sent[r] < PICOQUIC_MAX_ACK_RANGE_REPEAT) {
            sack_list->rc[r].range_counts[sack_item->nb_times_sent[r]] -= 1;
        }
        sack_item->nb_times_sent[r] = 0;
        sack_list->rc[r].range_counts[sack_item->nb_times_sent[r]] += 1;
    }
}
```

### Rust body
```rust
        let old = match self.sack_items.get(token) {
            Some(item) => item.nb_times_sent,
            None => return,
        };
```

## Pair `picoquic/sender.c:picoquic_unlink_app_stream_ctx`
C: `picoquic/sender.c:95-104 picoquic_unlink_app_stream_ctx`
Rust: `rs/fq/src/lib.rs:3963-3968 unlink_app_stream_ctx`

### C body
```c
{
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic); 
    
    stream = picoquic_find_stream(cnx, stream_id);
    if (stream != NULL) {
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

## Pair `picoquic/sender.c:picoquic_set_stream_not_coalesced`
C: `picoquic/sender.c:180-192 picoquic_set_stream_not_coalesced`
Rust: `rs/fq/src/lib.rs:4011-4020 set_stream_not_coalesced`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    stream = picoquic_find_stream_for_writing(cnx, stream_id, &ret);
    if (ret == 0) {
        stream->is_not_coalesced = is_not_coalesced;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let stream = self.find_stream_for_writing(stream_id)?;
        let stream = self.streams.get_mut(stream).ok_or(Error::Memory)?;
        stream.is_not_coalesced = is_not_coalesced;
        Ok(())
    }
```

## Pair `picoquic/sender.c:picoquic_set_stream_priority`
C: `picoquic/sender.c:213-226 picoquic_set_stream_priority`
Rust: `rs/fq/src/lib.rs:4023-4042 set_stream_priority`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    stream = picoquic_find_stream_for_writing(cnx, stream_id, &ret);
    if (ret == 0) {
        stream->stream_priority = stream_priority;
        picoquic_reorder_output_stream(cnx, stream);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let stream_token = self.find_stream_for_writing(stream_id)?;
        let was_output = self
            .streams
            .get(stream_token)
            .map(|stream| stream.is_output_stream)
            .unwrap_or(false);
        self.output_streams.retain(|&token| token != stream_token);
        if let Some(stream) = self.streams.get_mut(stream_token) {
            stream.stream_priority = stream_priority;
        }
        if was_output {
            enqueue_output_stream_token(self, stream_token);
        }
        Ok(())
    }
```

## Pair `picoquic/sender.c:picoquic_set_app_flow_control`
C: `picoquic/sender.c:317-330 picoquic_set_app_flow_control`
Rust: `rs/fq/src/lib.rs:4306-4317 set_app_flow_control`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream;
    PICOQUIC_THREAD_CHECK(cnx->quic); 

    if ((stream = picoquic_find_stream(cnx, stream_id)) == NULL) {
        ret = PICOQUIC_ERROR_INVALID_STREAM_ID;
    }
    else {
        stream->use_app_flow_control = use_app_flow_control;
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let stream = self
            .find_stream(stream_id)
            .ok_or(Error::Protocol(InternalError::InvalidStreamId as u64))?;
        let stream = self.streams.get_mut(stream).ok_or(Error::Memory)?;
        stream.use_app_flow_control = use_app_flow_control;
        Ok(())
    }
```

## Pair `picoquic/sender.c:picoquic_reset_stream`
C: `picoquic/sender.c:408-412 picoquic_reset_stream`
Rust: `rs/fq/src/lib.rs:4224-4226 reset_stream`

### C body
```c
{
    return picoquic_reset_stream_at(cnx, stream_id, local_stream_error, 0);
}
```

### Rust body
```rust
    pub fn reset_stream(&mut self, stream_id: u64, local_stream_error: u64) -> Result<(), Error> {
        self.reset_stream_at(stream_id, local_stream_error, 0)
    }
```

## Pair `picoquic/sender.c:picoquic_pad_to_target_length`
C: `picoquic/sender.c:496-504 picoquic_pad_to_target_length`
Rust: `rs/fq/src/internal.rs:7427-7434 pad_to_target_length`

### C body
```c
{
    if (length < target) {
        memset(bytes + length, 0, target - length);
        length = target;
    }

    return length;
}
```

### Rust body
```rust
pub fn pad_to_target_length(bytes: &mut [u8], length: usize, target: usize) -> usize {
    if length < target {
        bytes[length..target].fill(0);
        target
    } else {
        length
    }
}
```

## Pair `picoquic/sender.c:picoquic_update_payload_length`
C: `picoquic/sender.c:577-584 picoquic_update_payload_length`
Rust: `rs/fq/src/internal.rs:6958-6972 update_payload_length`

### C body
```c
{
    if ((bytes[0] & 0x80) != 0 && header_length > 6 && packet_length > header_length && packet_length < 0x4000)
    {
        picoquic_varint_encode_16(bytes + pnum_index - 2, (uint16_t)(packet_length - header_length));
    }
}
```

### Rust body
```rust
    {
        let payload_len = (packet_length - header_length) as u16;
        varint_encode_16(&mut bytes[pnum_index - 2..], payload_len);
    }
```

## Pair `picoquic/sender.c:picoquic_predict_packet_header_length`
C: `picoquic/sender.c:792-855 picoquic_predict_packet_header_length`
Rust: `rs/fq/src/internal.rs:6930-6955 predict_packet_header_length`

### C body
```c
{
    uint32_t header_length = 0;

    /* The only purpose of the test below is to appease the static analyzer, so it
     * wont complain of possible NULL deref. On windows we could use "__assume(cnx != NULL)
     * but the documentation does not say anything about that for GCC and CLANG */
    if (cnx == NULL) {
        return 0;
    }

    if (packet_type == picoquic_packet_1rtt_protected) {
        /* Predict acceptable length of packet number */
        uint8_t pn_l = 4;
        int64_t delta = pkt_ctx->send_sequence;
        if (pkt_ctx->pending_first != NULL) {
            delta -= pkt_ctx->pending_first->sequence_number;
        }
        if (delta < 262144) {
            pn_l = 3;
            if (pkt_ctx->send_sequence < 1024) {
                pn_l = 2;
                if (pkt_ctx->send_sequence < 16) {
                    pn_l = 1;
                }
            }
        }

        /* Compute length of a short packet header */
        header_length = 1 + cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len + pn_l;
    }
    else {
        /* Compute length of a long packet header */
        header_length = 1 + /* version */ 4 + /* cnx_id length bytes */ 2;

        /* add dest-id length */
        if (cnx->client_mode && (packet_type == picoquic_packet_initial ||
            packet_type == picoquic_packet_0rtt_protected)
            && picoquic_is_connection_id_null(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id)) {
            header_length += cnx->initial_cnxid.id_len;
        }
        else {
            header_length += cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len;
        }

        /* add srce-id length */
        header_length += cnx->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len;

        /* add length of payload length and packet number */
        header_length += 2 + 4;

        /* add length of tokens for initial packets */
        if (packet_type == picoquic_packet_initial) {
            uint8_t useless[16];
            header_length += (uint32_t)picoquic_varint_encode(useless, 16, cnx->retry_token_length);
            header_length += (uint32_t)cnx->retry_token_length;
        }
    }

    return header_length;
}
```

### Rust body
```rust
    ) -> usize {
        // Delegate to the PacketContext-based version.
        // Determine which PacketContext this pkt_ctx corresponds to.
        // We can identify it by pointer equality via index.
        let pc = {
            let addr = pkt_ctx as *const PacketContextState;
            let mut found = PacketContext::Application;
            for i in 0..self.pkt_ctx.len() {
                if std::ptr::eq(&self.pkt_ctx[i], addr) {
                    found = match i {
                        0 => PacketContext::Application,
                        1 => PacketContext::Handshake,
                        2 => PacketContext::Initial,
                        _ => PacketContext::Application,
                    };
                    break;
                }
            }
            found
        };
        self.predict_packet_header_length_for_pc(packet_type, pc)
    }
```

## Pair `picoquic/sender.c:picoquic_queue_for_retransmit`
C: `picoquic/sender.c:1000-1031 picoquic_queue_for_retransmit`
Rust: `rs/fq/src/internal.rs:5543-5601 queue_for_retransmit`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = NULL;
    
    if (packet->ptype == picoquic_packet_1rtt_protected && cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }
    else {
        pkt_ctx = &cnx->pkt_ctx[packet->pc];
    }

    /* Manage the double linked packet list for retransmissions */
    packet->packet_next = NULL;
    if (pkt_ctx->pending_last == NULL) {
        packet->packet_previous = NULL;
        pkt_ctx->pending_first = packet;
    } else {
        packet->packet_previous = pkt_ctx->pending_last;
        packet->packet_previous->packet_next = packet;
    }
    pkt_ctx->pending_last = packet;
    packet->is_queued_for_retransmit = 1;

    if (!packet->is_ack_trap) {
        /* Account for bytes in transit, for congestion control */
        path_x->bytes_in_transit += length;
        path_x->is_cc_data_updated = 1;
        /* Update the pacing data */
        picoquic_update_pacing_after_send(path_x, length, current_time);
    }
}
```

### Rust body
```rust
    ) {
        packet.length = length;
        packet.send_time = current_time;
        packet.send_path = Some(PathToken::synthetic(
            path_x.unique_path_id as u32,
            path_x.unique_path_id as u32,
        ));
        packet.is_queued_for_retransmit = true;
        packet.is_queued_to_path = false;
        let pc = packet.packet_context as usize;
        let sequence = packet.sequence_number;
        if let Ok(token) = self.queued_packets.insert(core::mem::replace(
            packet,
            Packet {
                queue_data_repeat_membership: None,
                send_path: None,
                sequence_number: 0,
                send_time: current_time,
                delivered_prior: 0,
                delivered_time_prior: current_time,
                delivered_sent_prior: 0,
                lost_prior: 0,
                inflight_prior: 0,
                data_repeat_frame: 0,
                data_repeat_index: 0,
                data_repeat_priority: 0,
                data_repeat_stream_id: 0,
                data_repeat_stream_offset: 0,
                data_repeat_stream_data_length: 0,
                length: 0,
                checksum_overhead: 0,
                offset: 0,
                packet_type: PacketType::Error,
                packet_context: PacketContext::Application,
                is_evaluated: false,
                is_ack_eliciting: false,
                is_mtu_probe: false,
                is_multipath_probe: false,
                is_ack_trap: false,
                delivered_app_limited: false,
                sent_cwin_limited: false,
                is_preemptive_repeat: false,
                was_preemptively_repeated: false,
                is_queued_to_path: false,
                is_queued_for_retransmit: false,
                is_queued_for_spurious_detection: false,
                is_queued_for_data_repeat: false,
                bytes: [0u8; MAX_PACKET_SIZE],
            },
        )) {
            self.pkt_ctx[pc].pending.insert(sequence, token);
        }
    }
```

## Pair `picoquic/sender.c:picoquic_finalize_and_protect_packet_tuple`
C: `picoquic/sender.c:1175-1260 picoquic_finalize_and_protect_packet_tuple`
Rust: `rs/fq/src/internal.rs:7437-7496 finalize_and_protect_packet_tuple`

### C body
```c
{
    if (length != 0 && length < header_length) {
        length = 0;
    }

    if (ret == 0 && length > 0) {
        packet->length = length;
        
        if (packet->ptype == picoquic_packet_1rtt_protected && cnx->is_multipath_enabled) {
            packet->sequence_number = path_x->pkt_ctx.send_sequence++;
        } else {
            packet->sequence_number = cnx->pkt_ctx[packet->pc].send_sequence++;
        }
        path_x->latest_sent_time = current_time;
        path_x->path_cid_rotated = 0;
        packet->delivered_prior = path_x->delivered_last;
        packet->delivered_time_prior = path_x->delivered_time_last;
        packet->delivered_sent_prior = path_x->delivered_sent_last;
        packet->lost_prior = path_x->total_bytes_lost;
        packet->inflight_prior = path_x->bytes_in_transit;
        packet->delivered_app_limited = (cnx->cnx_state < picoquic_state_ready || path_x->delivered_limited_index != 0);
        if (path_x->bytes_in_transit >= path_x->cwin && cnx->cnx_state == picoquic_state_ready) {
            packet->sent_cwin_limited = 1;
        }

        switch (packet->ptype) {
        case picoquic_packet_version_negotiation:
            /* Packet is not encrypted */
            break;
        case picoquic_packet_initial:
            length = picoquic_protect_packet(cnx, packet->ptype, packet->bytes, packet->sequence_number,
                length, header_length,
                send_buffer, send_buffer_max, cnx->crypto_context[picoquic_epoch_initial].aead_encrypt, cnx->crypto_context[picoquic_epoch_initial].pn_enc,
                path_x, NULL, current_time);
            break;
        case picoquic_packet_handshake:
            length = picoquic_protect_packet(cnx, packet->ptype, packet->bytes, packet->sequence_number,
                length, header_length,
                send_buffer, send_buffer_max, cnx->crypto_context[picoquic_epoch_handshake].aead_encrypt, cnx->crypto_context[picoquic_epoch_handshake].pn_enc,
                path_x, NULL, current_time);
            break;
        case picoquic_packet_retry:
            length = picoquic_protect_packet(cnx, packet->ptype, packet->bytes, packet->sequence_number,
                length, header_length,
                send_buffer, send_buffer_max, cnx->crypto_context[picoquic_epoch_0rtt].aead_encrypt, cnx->crypto_context[picoquic_epoch_0rtt].pn_enc,
                path_x, NULL, current_time);
            break;
        case picoquic_packet_0rtt_protected:
            length = picoquic_protect_packet(cnx, packet->ptype, packet->bytes, packet->sequence_number,
                length, header_length,
                send_buffer, send_buffer_max, cnx->crypto_context[picoquic_epoch_0rtt].aead_encrypt, cnx->crypto_context[picoquic_epoch_0rtt].pn_enc,
                path_x, NULL, current_time);
            break;
        case picoquic_packet_1rtt_protected:
            /* TODO: if multipath, use 96 bit nonce */
            length = picoquic_protect_packet(cnx, packet->ptype, packet->bytes, packet->sequence_number,
                length, header_length,
                send_buffer, send_buffer_max, cnx->crypto_context[picoquic_epoch_1rtt].aead_encrypt, cnx->crypto_context[picoquic_epoch_1rtt].pn_enc,
                path_x, tuple, current_time);
            break;
        default:
            /* Packet type error. Do nothing at all. */
            length = 0;
            break;
        }

        *send_length = length;

        if (length > 0) {
            packet->checksum_overhead = checksum_overhead;
            picoquic_queue_for_retransmit(cnx, path_x, packet, length, current_time);
            path_x->last_sent_time = current_time;
            path_x->bytes_sent += length;
        } else {
            *send_length = 0;
        }
    }
    else {
        *send_length = 0;
    }
}
```

### Rust body
```rust
    ) {
        *send_length = 0;
        if ret != 0 || length > packet.bytes.len() {
            return;
        }
        let epoch = match packet.packet_type {
            PacketType::Initial => Epoch::Initial,
            PacketType::Handshake => Epoch::Handshake,
            PacketType::ZeroRttProtected => Epoch::ZeroRtt,
            PacketType::OneRttProtected => Epoch::OneRtt,
            _ => Epoch::OneRtt,
        } as usize;
        let Some(aead) = self.crypto_context[epoch].aead_encrypt.take() else {
            if length <= send_buffer_max && length <= send_buffer.len() {
                send_buffer[..length].copy_from_slice(&packet.bytes[..length]);
                *send_length = length;
            }
            return;
        };
        let Some(pn_enc) = self.crypto_context[epoch].pn_enc.take() else {
            self.crypto_context[epoch].aead_encrypt = Some(aead);
            if length <= send_buffer_max && length <= send_buffer.len() {
                send_buffer[..length].copy_from_slice(&packet.bytes[..length]);
                *send_length = length;
            }
            return;
        };
        packet.checksum_overhead = checksum_overhead;
        packet.length = length.saturating_add(checksum_overhead);
        let protected = self.protect_packet(
            packet.packet_type,
            &mut packet.bytes,
            packet.sequence_number,
            length,
            header_length,
            send_buffer,
            send_buffer_max,
            &*aead,
            &*pn_enc,
            path_x,
            tuple,
            current_time,
        );
        self.crypto_context[epoch].aead_encrypt = Some(aead);
        self.crypto_context[epoch].pn_enc = Some(pn_enc);
        *send_length = protected;
    }
```

## Pair `picoquic/sender.c:picoquic_preemptive_retransmit_packet`
C: `picoquic/sender.c:1332-1419 picoquic_preemptive_retransmit_packet`
Rust: `rs/fq/src/internal.rs:17204-17280 preemptive_retransmit_packet`

### C body
```c
{
    /* check if this is an ACK only packet */
    int ret = 0;
    int frame_is_pure_ack = 0;
    size_t frame_length = 0;
    size_t byte_index = 0; /* Used when parsing the old packet */
    size_t write_index = 0;
    int is_repeated = 1;
    int do_not_detect_spurious = 0;
    int is_preemptive_needed = 0;
    size_t initial_length = *length;
    *has_data = 0;

    if (!old_p->is_mtu_probe &&
        !old_p->is_ack_trap &&
        !old_p->is_multipath_probe) {
        /* Copy the relevant bytes from one packet to the next */
        byte_index = old_p->offset;

        while (ret == 0 && byte_index < old_p->length) {
            ret = picoquic_skip_frame(&old_p->bytes[byte_index],
                old_p->length - byte_index, &frame_length, &frame_is_pure_ack);

            /* Check whether the data was already acked, which may happen in
             * case of spurious retransmissions */
            if (ret == 0 && frame_is_pure_ack == 0) {
                ret = picoquic_check_frame_needs_repeat(cnx, &old_p->bytes[byte_index],
                    frame_length, old_p->ptype, &frame_is_pure_ack, &do_not_detect_spurious, &is_preemptive_needed);
            }

            /* Prepare retransmission if needed */
            if (ret == 0 && !frame_is_pure_ack) {
                if (PICOQUIC_IN_RANGE(old_p->bytes[byte_index], picoquic_frame_type_stream_range_min, picoquic_frame_type_stream_range_max) &&
                    picoquic_is_stream_frame_unlimited(&old_p->bytes[byte_index])) {
                    /* If length is not present, check whether needed */
                    if (write_index + frame_length < send_buffer_max_minus_checksum) {
                        size_t pad_needed = send_buffer_max_minus_checksum - write_index - frame_length;
                        memset(&new_bytes[write_index], picoquic_frame_type_padding, pad_needed);
                        *length += pad_needed;
                        write_index += pad_needed;
                    }
                }
                /* copy the frame */
                if (write_index + frame_length <= send_buffer_max_minus_checksum) {
                    memcpy(&new_bytes[write_index], &old_p->bytes[byte_index], frame_length);
                    write_index += frame_length;
                    *length += frame_length;
                    *has_data = 1;
                }
                else {
                    is_repeated = 0;
                }
            }
            byte_index += frame_length;
        }
    }

    if (*has_data) {
        if (!is_preemptive_needed) {
            /* If the packet does not contain any frame requiring preemptive repeat, do not repeat it. */
            *length = initial_length;
            *has_data = 0;
            is_repeated = 0;
        } else if (is_repeated) {
            old_p->was_preemptively_repeated = 1;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut ret = 0;
        let mut write_index = 0usize;
        let mut is_repeated = 1;
        let mut do_not_detect_spurious = 0;
        let mut is_preemptive_needed = 0;
        let initial_length = *length;
        *has_data = 0;

        if !old_p.is_mtu_probe && !old_p.is_ack_trap && !old_p.is_multipath_probe {
            let mut byte_index = old_p.offset;
            while ret == 0 && byte_index < old_p.length {
                let mut frame_length = 0usize;
                let mut frame_is_pure_ack = 0;
                ret = skip_frame(
                    &old_p.bytes[byte_index..old_p.length],
                    old_p.length - byte_index,
                    &mut frame_length,
                    &mut frame_is_pure_ack,
                );
                if ret == 0 && frame_is_pure_ack == 0 {
                    ret = self.check_frame_needs_repeat(
                        &old_p.bytes[byte_index..old_p.length],
                        frame_length,
                        old_p.packet_type,
                        &mut frame_is_pure_ack,
                        &mut do_not_detect_spurious,
                        &mut is_preemptive_needed,
                    );
                }
                if ret == 0 && frame_is_pure_ack == 0 {
                    if old_p.bytes[byte_index] >= crate::frames::FrameType::StreamRangeMin as u8
                        && old_p.bytes[byte_index] <= crate::frames::FrameType::StreamRangeMax as u8
                        && is_stream_frame_unlimited(&old_p.bytes[byte_index..old_p.length])
                        && write_index + frame_length < send_buffer_max_minus_checksum
                    {
                        let pad_needed =
                            send_buffer_max_minus_checksum - write_index - frame_length;
                        new_bytes[write_index..write_index + pad_needed]
                            .fill(crate::frames::FrameType::Padding as u8);
                        *length = (*length).saturating_add(pad_needed);
                        write_index += pad_needed;
                    }
                    if write_index + frame_length <= send_buffer_max_minus_checksum
                        && write_index + frame_length <= new_bytes.len()
                    {
                        new_bytes[write_index..write_index + frame_length]
                            .copy_from_slice(&old_p.bytes[byte_index..byte_index + frame_length]);
                        write_index += frame_length;
                        *length = (*length).saturating_add(frame_length);
                        *has_data = 1;
                    } else {
                        is_repeated = 0;
                    }
                }
                byte_index = byte_index.saturating_add(frame_length.max(1));
            }
        }

        if *has_data != 0 {
            if is_preemptive_needed == 0 {
                *length = initial_length;
                *has_data = 0;
            } else if is_repeated != 0 {
                old_p.was_preemptively_repeated = true;
            }
        }

        ret
    }
```

## Pair `picoquic/sender.c:picoquic_is_mtu_probe_needed`
C: `picoquic/sender.c:1587-1628 picoquic_is_mtu_probe_needed`
Rust: `rs/fq/src/internal.rs:17146-17170 is_mtu_probe_needed`

### C body
```c
{
    int ret = picoquic_pmtu_discovery_not_needed;

    if ((cnx->cnx_state == picoquic_state_ready || 
        cnx->cnx_state == picoquic_state_client_ready_start || 
        cnx->cnx_state == picoquic_state_server_false_start)
        && path_x->mtu_probe_sent == 0 && cnx->pmtud_policy != picoquic_pmtud_blocked) {
        if (path_x->send_mtu_max_tried == 0 || path_x->send_mtu_max_tried > 1400) {
            /* MTU discovery is required if the chances of success are large enough
             * and there are enough packets to send to amortize the discovery cost.
             * Of course we don't know at this stage how much data will be sent 
             * on the connection; we take the amount of data queued as a proxy
             * for that. */
            uint64_t next_probe = picoquic_next_mtu_probe_length(cnx, path_x);
            if (next_probe > path_x->send_mtu) {
                if (cnx->pmtud_policy == picoquic_pmtud_required) {
                    ret = picoquic_pmtu_discovery_required;
                }
                else {
                    uint64_t packets_to_send_before = cnx->nb_bytes_queued / path_x->send_mtu;
                    uint64_t packets_to_send_after = cnx->nb_bytes_queued / next_probe;
                    uint64_t delta = (packets_to_send_before - packets_to_send_after) * 60;
                    if (delta > next_probe) {
                        ret = picoquic_pmtu_discovery_required;
                    }
                    else {
                        if (cnx->pmtud_policy == picoquic_pmtud_basic) {
                            ret = picoquic_pmtu_discovery_optional;
                        }
                        else {
                            ret = picoquic_pmtu_discovery_not_needed;
                        }
                    }
                }
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
        {
            let next_probe = self.next_mtu_probe_length(path_x, self.quic_mtu_max());
            if next_probe > path_x.send_mtu {
                if self.pmtud_policy == PmtudPolicy::Required {
                    ret = PmtuDiscoveryStatus::Required;
                } else {
                    let before = self.nb_bytes_queued / path_x.send_mtu.max(1) as u64;
                    let after = self.nb_bytes_queued / next_probe.max(1) as u64;
                    let delta = before.saturating_sub(after).saturating_mul(60);
                    if delta > next_probe as u64 {
                        ret = PmtuDiscoveryStatus::Required;
                    } else if self.pmtud_policy == PmtudPolicy::Basic {
                        ret = PmtuDiscoveryStatus::Optional;
                    }
                }
            }
        }
```
