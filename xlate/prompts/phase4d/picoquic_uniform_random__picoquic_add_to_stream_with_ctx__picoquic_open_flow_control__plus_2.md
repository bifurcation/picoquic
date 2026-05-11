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

## `picoquic/quicctx.c:picoquic_uniform_random`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only delegates to picoquic_public_uniform_random, while Rust inlines separate rejection-sampling logic; equivalence is not visible from these bodies.
* C source: `picoquic/quicctx.c:5600-5604`
* C signature: `uint64_t picoquic_uniform_random(uint64_t)`
* Rust source: `rs/fq/src/lib.rs:5168-5179`
* Rust item: `picoquic_uniform_random`

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

## `picoquic/sender.c:picoquic_add_to_stream_with_ctx`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust mirrors much of the queueing and FIN/reset logic, but visibly adds offset calculation and output-stream enqueue behavior while C only reinserts wake time after data enqueue.
* C source: `picoquic/sender.c:245-309`
* C signature: `int picoquic_add_to_stream_with_ctx(picoquic_cnx_t *, uint64_t, const uint8_t *, size_t, int, void *)`
* Rust source: `rs/fq/src/lib.rs:4167-4221`
* Rust item: `add_to_stream_with_ctx`

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

## `picoquic/sender.c:picoquic_open_flow_control`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only performs work when max_data_limit == 0 and formats MAX_DATA with expected_data_size, while Rust has no visible max_data_limit guard and computes a new data max from maxdata_local + expected_data_size.
* C source: `picoquic/sender.c:332-367`
* C signature: `int picoquic_open_flow_control(picoquic_cnx_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:4269-4303`
* Rust item: `open_flow_control`

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

## `picoquic/sender.c:picoquic_prepare_packet_old_context`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust adds a length <= bytes_limit guard before formatting/updating packet fields, while C proceeds for any length > 0.
* C source: `picoquic/sender.c:1771-1833`
* C signature: `size_t picoquic_prepare_packet_old_context(picoquic_cnx_t *, picoquic_packet_context_enum, picoquic_path_t *, picoquic_packet_t *, size_t, uint64_t, uint64_t *, size_t *)`
* Rust source: `rs/fq/src/internal.rs:17727-17828`
* Rust item: `picoquic_prepare_packet_old_context`

### C body
```c
{
    picoquic_epoch_enum epoch = (pc == picoquic_packet_context_initial) ? picoquic_epoch_initial :
        (pc == picoquic_packet_context_application) ? picoquic_epoch_0rtt : picoquic_epoch_handshake;
    size_t length = 0;

    /* Safety check: do not attempt to repeat old packets if the crypto
     * context has been deleted */
    if (cnx->crypto_context[epoch].aead_encrypt != NULL) {
        uint8_t* bytes = packet->bytes;
        int more_data = 0;
        size_t checksum_overhead = picoquic_get_checksum_length(cnx, epoch);
        uint8_t* bytes_max = bytes + send_buffer_max - checksum_overhead;
        uint8_t* bytes_next;
        size_t this_header_length = 0;
        int is_pure_ack = 0;

        send_buffer_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;
        length = picoquic_retransmit_needed(cnx, pc, path_x, current_time, next_wake_time, packet, send_buffer_max, &this_header_length);
        if (length > 0 && (pc == picoquic_packet_context_handshake || cnx->pkt_ctx[picoquic_packet_context_handshake].pending_first == NULL ||
            cnx->cnx_state == picoquic_state_server_init || cnx->cnx_state == picoquic_state_server_handshake)) {
            cnx->initial_repeat_needed = 0;
        }

        if (length == 0 && cnx->ack_ctx[pc].act[0].ack_needed != 0 &&
            pc != picoquic_packet_context_application) {
            packet->ptype =
                (pc == picoquic_packet_context_initial) ? picoquic_packet_initial :
                (pc == picoquic_packet_context_handshake) ? picoquic_packet_handshake :
                picoquic_packet_0rtt_protected;
            length = picoquic_predict_packet_header_length(cnx, packet->ptype, &cnx->pkt_ctx[pc]);
            packet->offset = length;
            this_header_length = length;
            packet->sequence_number = cnx->pkt_ctx[pc].send_sequence;
            packet->send_time = current_time;
            packet->send_path = path_x;
        }

        if (length > 0) {
            bytes_next = bytes + length;
            /* If present, send misc frame */
            bytes_next = picoquic_format_misc_frames_in_context(cnx, bytes_next, bytes_max,
                &more_data, &is_pure_ack, pc);
            if (packet->ptype != picoquic_packet_0rtt_protected) {
                /* Check whether it makes sense to add an ACK at the end of the retransmission */
                bytes_next = picoquic_format_ack_frame(cnx, bytes_next, bytes_max, &more_data,
                    current_time, pc, 0);
            }
            length = bytes_next - bytes;
            packet->length = length;
            /* document the send time & overhead */
            packet->send_time = current_time;
            packet->checksum_overhead = checksum_overhead;
            packet->pc = pc;
            *header_length = this_header_length;
        }
    }

    return length;
}
```

### Rust body
```rust
    ) -> usize {
        let epoch = match pc {
            PacketContext::Initial => Epoch::Initial,
            PacketContext::Application => Epoch::ZeroRtt,
            PacketContext::Handshake => Epoch::Handshake,
        };
        let mut length = 0usize;

        if self.crypto_context[epoch as usize].aead_encrypt.is_some() {
            let checksum_overhead = self.get_checksum_length(epoch);
            let send_buffer_max = send_buffer_max.min(path_x.send_mtu);
            let bytes_limit = send_buffer_max
                .saturating_sub(checksum_overhead)
                .min(packet.bytes.len());
            let mut more_data = 0;
            let mut this_header_length = 0usize;
            let mut is_pure_ack = 0;

            length = self.retransmit_needed(
                pc,
                path_x,
                current_time,
                next_wake_time,
                packet,
                send_buffer_max,
                &mut this_header_length,
            ) as usize;

            if length > 0
                && (pc == PacketContext::Handshake
                    || self.pkt_ctx[PacketContext::Handshake as usize]
                        .pending
                        .is_empty()
                    || self.connection_state == State::ServerInit
                    || self.connection_state == State::ServerHandshake)
            {
                self.initial_repeat_needed = false;
            }

            if length == 0
                && self.ack_ctx[pc as usize].act[0].ack_needed
                && pc != PacketContext::Application
            {
                packet.packet_type = match pc {
                    PacketContext::Initial => PacketType::Initial,
                    PacketContext::Handshake => PacketType::Handshake,
                    PacketContext::Application => PacketType::ZeroRttProtected,
                };
                length = self.predict_packet_header_length_for_pc(packet.packet_type, pc);
                packet.offset = length;
                this_header_length = length;
                packet.sequence_number = self.pkt_ctx[pc as usize].send_sequence;
                packet.send_time = current_time;
                packet.send_path = Some(Self::path_token_for_path(path_x));
            }

            if length > 0 && length <= bytes_limit {
                let mut offset = length;
                let tail_len = {
                    let tail = &mut packet.bytes[offset..bytes_limit];
                    match format_misc_frames_in_context(
                        self,
                        tail,
                        &mut more_data,
                        &mut is_pure_ack,
                        pc,
                    ) {
                        Some(next) => next.len(),
                        None => tail.len(),
                    }
                };
                offset = bytes_limit.saturating_sub(tail_len);
                if packet.packet_type != PacketType::ZeroRttProtected {
                    let tail_len = {
                        let tail = &mut packet.bytes[offset..bytes_limit];
                        match format_ack_frame(self, tail, &mut more_data, current_time, pc, 0) {
                            Some(next) => next.len(),
                            None => tail.len(),
                        }
                    };
                    offset = bytes_limit.saturating_sub(tail_len);
                }
                length = offset;
                packet.length = length;
                packet.send_time = current_time;
                packet.checksum_overhead = checksum_overhead;
                packet.packet_context = pc;
                *header_length = this_header_length;
            }
        }

        length
    }
```

## `picoquic/sim_link.c:picoquic_set_test_address`
* Phase 4C status: `suspect`
* Phase 4C rationale: C assigns sin_addr.s_addr directly from addr_val and sin_port directly from port, while Rust constructs Ipv4Addr from addr_be.to_be_bytes() and SocketAddr from port, making byte-order equivalence unclear from the bodies alone.
* C source: `picoquic/sim_link.c:451-462`
* C signature: `void picoquic_set_test_address(struct sockaddr_in *, uint32_t, uint16_t)`
* Rust source: `rs/fq/src/tests/util.rs:755-758`
* Rust item: `set_client_addr`

### C body
```c
{
    /* Init of the IP addresses */
    memset(addr, 0, sizeof(struct sockaddr_in));
    addr->sin_family = AF_INET;
#ifdef _WINDOWS
    addr->sin_addr.S_un.S_addr = addr_val;
#else
    addr->sin_addr.s_addr = addr_val;
#endif
    addr->sin_port = port;
}
```

### Rust body
```rust
    pub fn set_client_addr(&mut self, addr_be: u32, port: u16) {
        use core::net::Ipv4Addr;
        self.client_addr = SocketAddr::from((Ipv4Addr::from(addr_be.to_be_bytes()), port));
    }
```
