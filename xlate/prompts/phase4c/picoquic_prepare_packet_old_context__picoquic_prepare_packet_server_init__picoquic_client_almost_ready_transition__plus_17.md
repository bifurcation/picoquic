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

## Pair `picoquic/sender.c:picoquic_prepare_packet_old_context`
C: `picoquic/sender.c:1771-1833 picoquic_prepare_packet_old_context`
Rust: `rs/fq/src/internal.rs:17727-17828 picoquic_prepare_packet_old_context`

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

## Pair `picoquic/sender.c:picoquic_prepare_packet_server_init`
C: `picoquic/sender.c:2215-2349 picoquic_prepare_packet_server_init`
Rust: `rs/fq/src/internal.rs:18265-18336 prepare_packet_server_init`

### C body
```c
{
    int ret = 0;
    int tls_ready = 0;
    picoquic_epoch_enum epoch = picoquic_epoch_initial;
    picoquic_packet_type_enum packet_type = picoquic_packet_initial;
    picoquic_packet_context_enum pc = picoquic_packet_context_initial;
    size_t checksum_overhead = 8;
    size_t header_length = 0;
    uint8_t* bytes = packet->bytes;
    uint8_t* bytes_max;
    uint8_t* bytes_next;
    size_t length = 0;
    int more_data = 0;
    int is_pure_ack = 1;

    if (*next_wake_time > cnx->start_time + PICOQUIC_MICROSEC_HANDSHAKE_MAX) {
        *next_wake_time = cnx->start_time + PICOQUIC_MICROSEC_HANDSHAKE_MAX;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
    }

    /* The only purpose of the test below is to appease the static analyzer, so it
     * wont complain of possible NULL deref. On windows we could use "__assume(path_x != NULL)"
     * but the documentation does not say anything about that for GCC and CLANG */
    if (path_x == NULL) {
        return PICOQUIC_ERROR_UNEXPECTED_ERROR;
    }

    if (cnx->crypto_context[picoquic_epoch_handshake].aead_encrypt != NULL &&
        cnx->tls_stream[picoquic_epoch_initial].send_queue == NULL) {
        epoch = picoquic_epoch_handshake;
        pc = picoquic_packet_context_handshake;
        packet_type = picoquic_packet_handshake;
    }

    send_buffer_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;

    if (!cnx->initial_validated &&
        (cnx->initial_data_sent + send_buffer_max) > 3 * cnx->initial_data_received){
        /* Sending more data now would break the amplication limit */
        *send_length = 0;
        return 0;
    }

    /* If context is handshake, verify first that there is no need for retransmit or ack
     * on initial context */
    if (pc == picoquic_packet_context_handshake) {
        length = picoquic_prepare_packet_old_context(cnx, picoquic_packet_context_initial,
            path_x, packet, send_buffer_max, current_time, next_wake_time, &header_length);
    }

    if (length == 0) {
        checksum_overhead = picoquic_get_checksum_length(cnx, epoch);
        bytes_max = bytes + send_buffer_max - checksum_overhead;
        tls_ready = (cnx->tls_stream[epoch].send_queue != NULL &&
            cnx->tls_stream[epoch].send_queue->length > cnx->tls_stream[epoch].send_queue->offset);
        length = picoquic_predict_packet_header_length(cnx, packet_type, &cnx->pkt_ctx[pc]);
        packet->ptype = packet_type;
        packet->offset = length;
        header_length = length;
        packet->sequence_number = cnx->pkt_ctx[pc].send_sequence;
        packet->send_time = current_time;
        packet->send_path = path_x;
        packet->pc = pc;
        bytes_next = bytes + length;

        if (((tls_ready || picoquic_find_first_misc_frame(cnx, pc) != NULL)
            && path_x->cwin > path_x->bytes_in_transit && cnx->quic->cwin_max > path_x->bytes_in_transit) 
            || cnx->ack_ctx[pc].act[0].ack_needed) {
            bytes_next = picoquic_format_ack_frame(cnx, bytes_next, bytes_max, &more_data, current_time, pc, 0);
            /* Encode misc frames if present */
            bytes_next = picoquic_format_misc_frames_in_context(cnx, bytes_next, bytes_max,
                &more_data, &is_pure_ack, pc);
            /* Encode the crypto frame if present */
            bytes_next = picoquic_format_crypto_hs_frame(&cnx->tls_stream[epoch],
                bytes_next, bytes_max, &more_data, &is_pure_ack);
            length = bytes_next - bytes;
            *is_initial_sent |= (epoch == picoquic_epoch_initial && !is_pure_ack);

            /* progress the state if the epoch data is all sent */
            if (ret == 0 && tls_ready != 0 && cnx->tls_stream[epoch].send_queue == NULL) {
                if (epoch == picoquic_epoch_handshake && picoquic_tls_client_authentication_activated(cnx->quic) == 0) {
                    picoquic_false_start_transition(cnx, current_time);

                    if (cnx->callback_fn != NULL) {
                        if (cnx->callback_fn(cnx, 0, NULL, 0, picoquic_callback_almost_ready, cnx->callback_ctx, NULL) != 0) {
                            picoquic_log_app_message(cnx, "Callback almost ready returns error 0x%x", PICOQUIC_TRANSPORT_INTERNAL_ERROR);
                            picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0);
                        }
                    }
                }
                else {
                    cnx->cnx_state = picoquic_state_server_handshake;
                }
            }
            packet->length = length;
        }
        else if ((length = picoquic_retransmit_needed(cnx, pc, path_x, current_time, next_wake_time, packet, send_buffer_max, &header_length)) > 0) {
            /* Set the new checksum length */
            checksum_overhead = picoquic_get_checksum_length(cnx, epoch);
            cnx->initial_repeat_needed = 0;
            /* Check whether it makes sens to add an ACK at the end of the retransmission */
            bytes_max = bytes + send_buffer_max - checksum_overhead;
            bytes_next = picoquic_format_ack_frame(cnx, bytes + length, bytes_max, &more_data, current_time, pc, 0);
            length = bytes_next - bytes;
            packet->length = length;
            /* document the send time & overhead */
            packet->send_time = current_time;
            packet->checksum_overhead = checksum_overhead;
        } else {
            length = 0;
            packet->length = 0;
        }
    }

    if (ret == 0 && length == 0 && more_data) {
        *next_wake_time = current_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
    }

    picoquic_finalize_and_protect_packet(cnx, packet,
        ret, length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_max,
        path_x, current_time);

    /* Account for data sent during handshake */
    if (!cnx->initial_validated) {
        cnx->initial_data_sent += *send_length;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let handshake_deadline = Instant::from_ticks(
            self.start_time
                .ticks()
                .saturating_add(MICROSEC_HANDSHAKE_MAX.ticks()),
        );
        if handshake_deadline < *next_wake_time {
            *next_wake_time = handshake_deadline;
        }
        if !self.initial_validated
            && self
                .initial_data_sent
                .saturating_add(send_buffer_max.min(path_x.send_mtu) as u64)
                > 3u64.saturating_mul(self.initial_data_received)
        {
            *send_length = 0;
            return 0;
        }
        let epoch = if self.crypto_context[Epoch::Handshake as usize]
            .aead_encrypt
            .is_some()
            && self.tls_stream[Epoch::Initial as usize]
                .send_queue
                .is_empty()
        {
            Epoch::Handshake
        } else {
            Epoch::Initial
        };
        let ret = self.prepare_handshake_packet_for_epoch(
            path_x,
            packet,
            current_time,
            send_buffer,
            send_buffer_max,
            send_length,
            next_wake_time,
            epoch,
            false,
            is_initial_sent,
        );
        if ret == 0 && *send_length > 0 && !self.initial_validated {
            self.initial_data_sent = self.initial_data_sent.saturating_add(*send_length as u64);
        }
        if ret == 0 && *send_length > 0 && self.tls_stream[epoch as usize].send_queue.is_empty() {
            if epoch == Epoch::Handshake
                && !self
                    .quic_ref()
                    .map(|q| q.client_authentication)
                    .unwrap_or(false)
            {
                self.false_start_transition(current_time);
                if let Some(mut cb) = self.callback_fn.take() {
                    let _ = cb.callback(self, 0, &[], CallbackEvent::AlmostReady, None);
                    self.callback_fn = Some(cb);
                }
            } else {
                self.connection_state = State::ServerHandshake;
            }
        }
        ret
    }
```

## Pair `picoquic/sender.c:picoquic_client_almost_ready_transition`
C: `picoquic/sender.c:2685-2696 picoquic_client_almost_ready_transition`
Rust: `rs/fq/src/internal.rs:7691-7714 client_almost_ready_transition`

### C body
```c
{
    cnx->cnx_state = picoquic_state_client_almost_ready;
    /* If client, make sure that 0-RTT packets are in correct context */
    if (cnx->is_multipath_enabled) {
        picoquic_packet_context_t* o_pkt_ctx = &cnx->pkt_ctx[0];
        picoquic_packet_context_t* n_pkt_ctx = &cnx->path[0]->pkt_ctx;

        *n_pkt_ctx = *o_pkt_ctx;
        picoquic_init_packet_ctx(cnx, o_pkt_ctx, picoquic_packet_context_application);
    }
}
```

### Rust body
```rust
        if self.is_multipath_enabled && !self.paths.is_empty() {
            let app_ctx = core::mem::replace(
                &mut self.pkt_ctx[PacketContext::Application as usize],
                PacketContextState {
                    send_sequence: 0,
                    next_sequence_hole: 0,
                    retransmit_sequence: 0,
                    highest_acknowledged: u64::MAX,
                    latest_time_acknowledged: self.start_time,
                    highest_acknowledged_time: self.start_time,
                    pending: BTreeMap::new(),
                    retransmitted: BTreeMap::new(),
                    preemptive_repeat_seq: None,
                    retransmitted_queue_size: 0,
                    ecn_ect0_total_remote: 0,
                    ecn_ect1_total_remote: 0,
                    ecn_ce_total_remote: 0,
                    ack_of_ack_requested: false,
                },
            );
            self.paths[0].pkt_ctx = app_ctx;
        }
```

## Pair `picoquic/sender.c:picoquic_prepare_packet_almost_ready`
C: `picoquic/sender.c:2964-3284 picoquic_prepare_packet_almost_ready`
Rust: `rs/fq/src/internal.rs:19306-19418 prepare_packet_almost_ready`

### C body
```c
{
    int ret = 0;
    picoquic_packet_type_enum packet_type = picoquic_packet_1rtt_protected;
    picoquic_packet_context_enum pc = picoquic_packet_context_application;
    int tls_ready = 0;
    int is_pure_ack = 1;
    size_t header_length = 0;
    uint8_t* bytes = packet->bytes;
    size_t length = 0;
    size_t checksum_overhead = picoquic_get_checksum_length(cnx, picoquic_epoch_1rtt);
    size_t send_buffer_min_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;
    uint8_t* bytes_max = bytes + send_buffer_min_max - checksum_overhead;
    uint8_t* bytes_next = NULL;
    int more_data = 0;
    int no_data_to_send = 0;
    int is_challenge_padding_needed = 0;

    /* Perform amplification prevention check */
    if (!cnx->initial_validated &&
        (cnx->initial_data_sent + send_buffer_min_max) > 3 * cnx->initial_data_received) {
        *send_length = 0;
        return 0;
    }

    /* Verify first that there is no need for retransmit or ack
     * on initial or handshake context. */
    if (path_x->first_tuple->p_local_cnxid != NULL) {
        if (cnx->crypto_context[picoquic_epoch_initial].aead_encrypt != NULL) {
            length = picoquic_prepare_packet_old_context(cnx, picoquic_packet_context_initial,
                path_x, packet, send_buffer_min_max, current_time, next_wake_time, &header_length);
        }
        else {
            length = 0;
        }

        if (length == 0) {
            length = picoquic_prepare_packet_old_context(cnx, picoquic_packet_context_handshake,
                path_x, packet, send_buffer_min_max, current_time, next_wake_time, &header_length);
            if (length == 0 && (*is_initial_sent == 1) && cnx->cnx_state == picoquic_state_server_false_start) {
                /* Add a simple Handshake Ping to work around bugs in some implementations */
                packet->ptype = picoquic_packet_handshake;
                length = picoquic_predict_packet_header_length(cnx, packet->ptype, &cnx->pkt_ctx[picoquic_packet_context_handshake]);
                header_length = length;
                packet->offset = length;
                packet->sequence_number = cnx->pkt_ctx[picoquic_packet_context_handshake].send_sequence;
                packet->send_time = current_time;
                packet->send_path = path_x;
                packet->bytes[length] = picoquic_frame_type_ping;
                length++;
                checksum_overhead = picoquic_get_checksum_length(cnx, picoquic_epoch_handshake);
                packet->checksum_overhead = checksum_overhead;
                packet->pc = picoquic_packet_context_handshake;
                is_pure_ack = 0;
                *is_initial_sent += 1;
            }
            if (length > 0) {
                checksum_overhead = picoquic_get_checksum_length(cnx, picoquic_epoch_handshake);
                bytes_max = bytes + send_buffer_min_max - checksum_overhead;
            }
        }
        else {
            checksum_overhead = picoquic_get_checksum_length(cnx, picoquic_epoch_initial);
            bytes_max = bytes + send_buffer_min_max - checksum_overhead;

            *is_initial_sent = 1;
        }

        if (length > 0) {
            cnx->initial_repeat_needed = 0;

            if (cnx->client_mode && *is_initial_sent && send_buffer_min_max < length + checksum_overhead + PICOQUIC_MIN_SEGMENT_SIZE) {
                length = picoquic_pad_to_target_length(packet->bytes, length, send_buffer_min_max - checksum_overhead);
            }
        }
    }

    if (length == 0) {
        picoquic_packet_context_t* pkt_ctx = &cnx->pkt_ctx[pc];
        if (cnx->is_multipath_enabled) {
            pkt_ctx = &path_x->pkt_ctx;
        }

        tls_ready = picoquic_is_tls_stream_ready(cnx);
        packet->pc = pc;
        length = picoquic_predict_packet_header_length(
            cnx, packet_type, pkt_ctx);
        packet->ptype = packet_type;
        packet->offset = length;
        header_length = length;
        packet->sequence_number = pkt_ctx->send_sequence;
        packet->send_time = current_time;
        packet->send_path = path_x;
        bytes_next = bytes + length;

        bytes_next = picoquic_prepare_path_challenge_frames(cnx, path_x,
            bytes_next, bytes_max,
            &more_data, &is_pure_ack, &is_challenge_padding_needed,
            current_time, next_wake_time);

        length = bytes_next - bytes;

        if (cnx->cnx_state != picoquic_state_disconnected && path_x->first_tuple->challenge_verified != 0) {
            /* There are no frames yet that would be exempt from pacing control, but if there
             * was they should be sent here. */

            if (picoquic_is_sending_authorized_by_pacing(cnx, path_x, current_time, next_wake_time)) {
                /* There should not be any retransmission at the server if not ready.
                 * At the client, it mostly makes sense to retransmit zero_rtt data, if lost,
                 * but other data might be lost too if the client lingers in that state for
                 * several RTT.
                 */
                if (length <= header_length && cnx->client_mode &&
                    picoquic_find_first_misc_frame(cnx, pc) == NULL &&
                    (length = picoquic_retransmit_needed(cnx, pc, path_x, current_time, next_wake_time, packet,
                        send_buffer_min_max, &header_length)) > 0) {
                    /* Check whether it makes sense to add an ACK at the end of the retransmission */
                    /* Testing header length for defense in depth -- avoid creating new packet if
                     * picoquic_retransmit_needed erroneously returns length <= header_length */
                    if (bytes + length + 256 < bytes_max && length > header_length) {
                        /* Don't do that if it risks mixing clear text and encrypted ack */
                        bytes_next = picoquic_format_ack_frame(cnx, bytes + length, bytes_max, &more_data,
                            current_time, pc, 0);
                        length = bytes_next - bytes;
                    }
                    /* document the send time & overhead */
                    is_pure_ack = 0;
                    packet->send_time = current_time;
                    packet->checksum_overhead = checksum_overhead;
                }

                /* Send here the frames that are not exempt from the pacing control,
                 * but are exempt for congestion control */
                if (picoquic_is_ack_needed(cnx, current_time, next_wake_time, pc, 0)) {
                    bytes_next = picoquic_format_ack_frame(cnx, bytes_next, bytes_max, &more_data,
                        current_time, pc, 0);
                }

                length = bytes_next - bytes;
                if (path_x->cwin < path_x->bytes_in_transit) {
                    picoquic_per_ack_state_t ack_state = { 0 };
                    ack_state.pc = pc;
                    cnx->cwin_blocked = 1;
                    path_x->last_cwin_blocked_time = current_time;
                    if (cnx->congestion_alg != NULL) {
                        cnx->congestion_alg->alg_notify(cnx, path_x,
                            picoquic_congestion_notification_cwin_blocked,
                            &ack_state, current_time);
                    }
                }
                else {
                    /* Send here the frames that are subject to both congestion and pacing control.
                     * this includes the PMTU probes.
                     * Check whether PMTU discovery is required. The call will return
                     * three values: not needed at all, optional, or required.
                     * If required, PMTU discovery takes priority over sending stream data.
                     */
                    picoquic_pmtu_discovery_status_enum pmtu_discovery_needed = picoquic_is_mtu_probe_needed(cnx, path_x);

                    /* if present, send tls data */
                    if (tls_ready) {
                        bytes_next = picoquic_format_crypto_hs_frame(&cnx->tls_stream[picoquic_epoch_1rtt],
                            bytes_next, bytes_max, &more_data, &is_pure_ack);
                    }


                    if (pc != picoquic_packet_context_application) {
                        bytes_next = picoquic_format_misc_frames_in_context(cnx, bytes_next, bytes_max,
                            &more_data, &is_pure_ack, pc);
                        length = bytes_next - bytes;
                    }
                    else {
                        length = bytes_next - bytes;
                        if (length > header_length || pmtu_discovery_needed != picoquic_pmtu_discovery_required ||
                            send_buffer_max <= path_x->send_mtu) {
                            /* No need or no way to do pmtu discovery */
                            /* If present, send misc frame */
                            bytes_next = picoquic_format_misc_frames_in_context(cnx, bytes_next, bytes_max,
                                &more_data, &is_pure_ack, pc);

                            if (cnx->is_address_discovery_provider) {
                                /* If a new address was learned, prepare an observed address frame */
                                /* TODO: tie this code to path challenge/response */
                                bytes_next = picoquic_prepare_observed_address_frame(bytes_next, bytes_max,
                                    path_x, path_x->first_tuple, current_time, next_wake_time, &more_data, &is_pure_ack);
                            }

                            /* If there are not enough published CID, create and advertise */
                            if (ret == 0) {
                                bytes_next = picoquic_format_new_local_id_as_needed(cnx, bytes_next, bytes_max,
                                    current_time, next_wake_time, &more_data, &is_pure_ack);
                            }
                            if (cnx->is_ack_frequency_updated && cnx->is_ack_frequency_negotiated) {
                                bytes_next = picoquic_format_ack_frequency_frame(cnx, bytes_next, bytes_max, &more_data);
                            }
                            if (ret == 0) {
                                bytes_next = picoquic_prepare_stream_and_datagrams(cnx, path_x, bytes_next, bytes_max,
                                    (size_t)(bytes_next - bytes) <= packet->offset, UINT64_MAX,
                                    &more_data, &is_pure_ack, &no_data_to_send, &ret);
                            }
                            /* TODO: replace this by posting of frame when CWIN estimated */
                            /* Send bdp frames if there are no stream frames to send
                             * and if client wishes to receive bdp frames */
                            if (!cnx->client_mode && cnx->send_receive_bdp_frame) {
                                bytes_next = picoquic_format_bdp_frame(cnx, bytes_next, bytes_max, path_x, &more_data, &is_pure_ack);
                            }

                            length = bytes_next - bytes;
                            if (length <= header_length) {
                                /* Mark the bandwidth estimation as application limited */
                                path_x->delivered_limited_index = path_x->delivered;
                                /* Notify the peer if something is blocked */
                                bytes_next = picoquic_format_blocked_frames(cnx, &bytes[length], bytes_max, &more_data, &is_pure_ack);
                                length = bytes_next - bytes;
                            }

                            if (no_data_to_send) {
                                path_x->last_sender_limited_time = current_time;
                            }
                        } /* end of PMTU not required */

                        if (ret == 0 && length <= header_length
                            && path_x->cwin > path_x->bytes_in_transit && cnx->quic->cwin_max > path_x->bytes_in_transit
                            && pmtu_discovery_needed != picoquic_pmtu_discovery_not_needed) {
                            if (send_buffer_max > path_x->send_mtu) {
                                /* Since there is no data to send, this is an opportunity to send an MTU probe */
                                length = picoquic_prepare_mtu_probe(cnx, path_x, header_length, checksum_overhead, bytes, send_buffer_max);
                                packet->length = length;
                                packet->send_path = path_x;
                                packet->is_mtu_probe = 1;
                                path_x->mtu_probe_sent = 1;
                                is_pure_ack = 0;
                            }
                            else if (cnx->is_sending_large_buffer) {
                                /* Should attempt PMTU discovery at next opportunity */
                                *next_wake_time = current_time;
                                SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                            }
                        }
                    }
                } /* end of congestion blocked */
            } /* end of CC */
        } /* End of pacing */
        if (length <= header_length) {
            length = 0;
        }

        if (cnx->cnx_state != picoquic_state_disconnected) {
            /* If necessary, encode and send the keep alive packet!
             * We only send keep alive packets when no other data is sent!
             */
            if (is_pure_ack == 0)
            {
                cnx->latest_progress_time = current_time;
            }
            else if (cnx->keep_alive_interval != 0) {
                if (cnx->latest_progress_time + cnx->keep_alive_interval <= current_time && length == 0) {
                    length = picoquic_predict_packet_header_length(
                        cnx, packet_type, pkt_ctx);
                    packet->ptype = packet_type;
                    packet->pc = pc;
                    packet->offset = length;
                    header_length = length;
                    packet->sequence_number = pkt_ctx->send_sequence;
                    packet->send_path = path_x;
                    packet->send_time = current_time;
                    bytes[length++] = picoquic_frame_type_ping;
                    bytes[length++] = 0;
                    cnx->latest_progress_time = current_time;
                }
                else if (cnx->latest_progress_time + cnx->keep_alive_interval < *next_wake_time) {
                    *next_wake_time = cnx->latest_progress_time + cnx->keep_alive_interval;
                    SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                }
            }
        }
    }

    if (ret == 0 && length > header_length) {
        if (more_data) {
            *next_wake_time = current_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
            ret = 0;
        }

        /* Ensure that all packets are properly padded before being sent. */
        if ((*is_initial_sent && (packet->ptype != picoquic_packet_initial || length + checksum_overhead + PICOQUIC_MIN_SEGMENT_SIZE > send_buffer_min_max || cnx->quic->dont_coalesce_init)) ||
            (is_challenge_padding_needed && length < PICOQUIC_ENFORCED_INITIAL_MTU) ){
            length = picoquic_pad_to_target_length(bytes, length, (uint32_t)(send_buffer_min_max - checksum_overhead));
        }
        else {
            length = picoquic_pad_to_policy(cnx, bytes, length, (uint32_t)(send_buffer_min_max - checksum_overhead));
        }
    }

    picoquic_finalize_and_protect_packet(cnx, packet,
        ret, length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_min_max,
        path_x, current_time);

    if (*send_length > 0) {
        /* Account for data sent during handshake */
        if (!cnx->initial_validated) {
            cnx->initial_data_sent += *send_length;
        }
        *next_wake_time = current_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);

        if (picoquic_cnx_is_still_logging(cnx)) {
            picoquic_log_cc_dump(cnx, current_time);
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        if !self.initial_validated
            && self
                .initial_data_sent
                .saturating_add(send_buffer_max.min(path_x.send_mtu) as u64)
                > 3u64.saturating_mul(self.initial_data_received)
        {
            *send_length = 0;
            return 0;
        }

        let mut header_length = 0usize;
        if path_x
            .tuples
            .first()
            .and_then(|t| t.local_connection_id)
            .is_some()
        {
            if self.crypto_context[Epoch::Initial as usize]
                .aead_encrypt
                .is_some()
            {
                let length = self.prepare_packet_old_context_like(
                    PacketContext::Initial,
                    path_x,
                    packet,
                    send_buffer_max.min(path_x.send_mtu),
                    current_time,
                    next_wake_time,
                    &mut header_length,
                );
                if length > 0 {
                    let checksum_overhead = self.get_checksum_length(Epoch::Initial);
                    self.finalize_and_protect_packet(
                        packet,
                        0,
                        length,
                        header_length,
                        checksum_overhead,
                        send_length,
                        send_buffer,
                        send_buffer_max.min(path_x.send_mtu),
                        path_x,
                        current_time,
                    );
                    *is_initial_sent = 1;
                    return 0;
                }
            }
            let length = self.prepare_packet_old_context_like(
                PacketContext::Handshake,
                path_x,
                packet,
                send_buffer_max.min(path_x.send_mtu),
                current_time,
                next_wake_time,
                &mut header_length,
            );
            if length > 0 {
                let checksum_overhead = self.get_checksum_length(Epoch::Handshake);
                self.finalize_and_protect_packet(
                    packet,
                    0,
                    length,
                    header_length,
                    checksum_overhead,
                    send_length,
                    send_buffer,
                    send_buffer_max.min(path_x.send_mtu),
                    path_x,
                    current_time,
                );
                return 0;
            } else if *is_initial_sent == 1 && self.connection_state == State::ServerFalseStart {
                return self.prepare_handshake_packet_for_epoch(
                    path_x,
                    packet,
                    current_time,
                    send_buffer,
                    send_buffer_max,
                    send_length,
                    next_wake_time,
                    Epoch::Handshake,
                    true,
                    is_initial_sent,
                );
            }
        }

        let ret = self.prepare_packet_ready(
            path_x,
            packet,
            current_time,
            send_buffer,
            send_buffer_max,
            send_length,
            next_wake_time,
        );
        if *send_length > 0 && !self.initial_validated {
            self.initial_data_sent = self.initial_data_sent.saturating_add(*send_length as u64);
        }
        ret
    }
```

## Pair `picoquic/sender.c:picoquic_set_path_addresses_from_tuple`
C: `picoquic/sender.c:3832-3846 picoquic_set_path_addresses_from_tuple`
Rust: `rs/fq/src/internal.rs:4117-4125 picoquic_set_path_addresses_from_tuple`

### C body
```c
{
    if (p_addr_to != NULL) {
        picoquic_store_addr(p_addr_to, (struct sockaddr*)&tuple->peer_addr);
    }

    if (p_addr_from != NULL) {
        picoquic_store_addr(p_addr_from, (struct sockaddr*)&tuple->local_addr);
    }

    if (if_index != NULL) {
        *if_index = tuple->if_index;
    }
}
```

### Rust body
```rust
    if let Some(addr_to) = p_addr_to {
        *addr_to = tuple.peer_addr;
    }
```

## Pair `picoquic/sender.c:picoquic_handle_send_timers`
C: `picoquic/sender.c:3905-3932 picoquic_handle_send_timers`
Rust: `rs/fq/src/internal.rs:17008-17044 handle_send_timers`

### C body
```c
{
    int ret = 0;

    *next_wake_time = cnx->latest_receive_time + 2 * PICOQUIC_MICROSEC_SILENCE_MAX;

    if (cnx->local_parameters.max_idle_timeout > (PICOQUIC_MICROSEC_SILENCE_MAX / 500)) {
        *next_wake_time = cnx->latest_receive_time + cnx->local_parameters.max_idle_timeout * 1000ull;
    }

    SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);

    if (cnx->recycle_sooner_needed) {
        picoquic_process_sooner_packets(cnx, current_time);
    }

    ret = picoquic_handle_app_wake_time(cnx, current_time);

    if (ret == 0) {
        ret = picoquic_check_idle_timer(cnx, next_wake_time, current_time);
    }

    if (ret == 0) {
        ret = picoquic_check_cc_feedback_timer(cnx, next_wake_time, current_time);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        *next_wake_time = Instant::from_ticks(
            self.latest_receive_time
                .ticks()
                .saturating_add(2u64.saturating_mul(MICROSEC_SILENCE_MAX.ticks())),
        );

        if self.local_parameters.max_idle_timeout.ticks() > (MICROSEC_SILENCE_MAX.ticks() / 500) {
            *next_wake_time = Instant::from_ticks(
                self.latest_receive_time.ticks().saturating_add(
                    self.local_parameters
                        .max_idle_timeout
                        .ticks()
                        .saturating_mul(1000),
                ),
            );
        }
        self.next_wake_time = *next_wake_time;

        if self.recycle_sooner_needed {
            self.process_sooner_packets(current_time);
        }

        let mut ret = self.handle_app_wake_time(current_time);
        if ret == 0 {
            ret = self.check_idle_timer(next_wake_time, current_time);
        }
        if ret == 0 {
            ret = self.check_cc_feedback_timer(next_wake_time, current_time);
        }
        self.next_wake_time = *next_wake_time;
        ret
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_packet`
C: `picoquic/sender.c:4183-4194 picoquic_prepare_packet`
Rust: `rs/fq/src/lib.rs:3701-3709 prepare_packet`

### C body
```c
{
    int if_index_null;
    if (if_index == NULL) {
        if_index = &if_index_null;
    }

    return picoquic_prepare_packet_ex(cnx, current_time, send_buffer, send_buffer_max, send_length,
        p_addr_to, p_addr_from, if_index, NULL);
}
```

### Rust body
```rust
    ) -> Result<PreparedCnxPacket, Error> {
        let mut prepared = self.prepare_packet_ex(current_time, send_buffer)?;
        prepared.send_msg_size = None;
        Ok(prepared)
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_next_packet_ex`
C: `picoquic/sender.c:4244-4324 picoquic_prepare_next_packet_ex`
Rust: `rs/fq/src/lib.rs:3392-3505 prepare_next_packet_ex`

### C body
```c
{
    int ret = 0;
    picoquic_stateless_packet_t* sp;
    PICOQUIC_THREAD_CHECK(quic);
    
    sp = picoquic_dequeue_stateless_packet(quic);
    if (p_last_cnx) {
        *p_last_cnx = NULL;
    }

    if (sp != NULL) {
        if (sp->length > send_buffer_max) {
            *send_length = 0;
        }
        else {
            memcpy(send_buffer, sp->bytes, sp->length);
            *send_length = sp->length;
            picoquic_store_addr(p_addr_to, (struct sockaddr*) & sp->addr_to);
            picoquic_store_addr(p_addr_from, (struct sockaddr*) & sp->addr_local);
            *if_index = sp->if_index_local;
            if (log_cid != NULL) {
                *log_cid = sp->initial_cid;
            }
        }
        picoquic_delete_stateless_packet(sp);
    }
    else {
        picoquic_cnx_t* cnx = picoquic_get_earliest_cnx_to_wake(quic, current_time);

        if (cnx == NULL) {
            *send_length = 0;
        }
        else {
            ret = picoquic_prepare_packet_ex(cnx, current_time, send_buffer, send_buffer_max, send_length, p_addr_to, p_addr_from, 
                if_index, send_msg_size);
            if (log_cid != NULL) {
                *log_cid = cnx->initial_cnxid;
            }

            if (ret == PICOQUIC_ERROR_DISCONNECTED) {
                ret = 0;

                picoquic_log_app_message(cnx, "Closed. Retrans= %d, spurious= %d, max sp gap = %d, max sp delay = %d, dg-coal: %f",
                    (int)cnx->nb_retransmission_total, (int)cnx->nb_spurious,
                    (int)cnx->path[0]->max_reorder_gap, (int)cnx->path[0]->max_spurious_rtt,
                    (cnx->nb_trains_sent > 0) ? ((double)cnx->nb_packets_sent / (double)cnx->nb_trains_sent) : 0.0);

                if (quic->F_log != NULL) {
                    fflush(quic->F_log);
                }

                if (cnx->f_binlog != NULL) {
                    fflush(cnx->f_binlog);
                }

                if (cnx->client_mode) {
                    /* Do not unilaterally delete the connection context, as it was set by the application */
                    picoquic_reinsert_by_wake_time(cnx->quic, cnx, UINT64_MAX);
                    SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                }
                else {
                    picoquic_delete_cnx(cnx);
                }
            }
            else {
                if (*if_index == -1) {
                    *if_index = picoquic_get_local_if_index(cnx);
                }
                if (p_last_cnx) {
                    *p_last_cnx = cnx;
                }
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<PreparedPacket<'_>, Error> {
        let default_addr = unspecified_socket_addr();

        if let Some(sp) = self.dequeue_stateless_packet() {
            if sp.length > send_buffer.len() {
                return Ok(PreparedPacket {
                    send_length: 0,
                    addr_to: default_addr,
                    addr_from: default_addr,
                    if_index: -1,
                    log_cid: ConnectionId::default(),
                    last_connection: None,
                    send_msg_size: None,
                });
            }
            send_buffer[..sp.length].copy_from_slice(&sp.bytes[..sp.length]);
            return Ok(PreparedPacket {
                send_length: sp.length,
                addr_to: sp.addr_to,
                addr_from: sp.addr_local,
                if_index: sp.if_index_local,
                log_cid: sp.initial_connection_id,
                last_connection: None,
                send_msg_size: None,
            });
        }

        let token = self
            .connections
            .iter()
            .filter(|cnx| cnx.next_wake_time <= current_time)
            .filter_map(|cnx| cnx.own_token.map(|tok| (tok, cnx.next_wake_time)))
            .min_by_key(|(_, wake)| wake.ticks())
            .map(|(tok, _)| tok);

        let Some(token) = token else {
            return Ok(PreparedPacket {
                send_length: 0,
                addr_to: default_addr,
                addr_from: default_addr,
                if_index: -1,
                log_cid: ConnectionId::default(),
                last_connection: None,
                send_msg_size: None,
            });
        };

        let log_cid = self
            .connections
            .get(token)
            .map(|cnx| cnx.initial_connection_id)
            .unwrap_or_default();
        let prepared = {
            let cnx = self
                .connections
                .get_mut(token)
                .ok_or(Error::InvalidArgument)?;
            cnx.prepare_packet_ex(current_time, send_buffer)
        };

        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(Error::Disconnected) => {
                let is_client = self
                    .connections
                    .get(token)
                    .map(|cnx| cnx.client_mode)
                    .unwrap_or(true);
                if is_client {
                    self.reinsert_by_wake_time_token(token, Instant::from_ticks(u64::MAX));
                } else {
                    self.delete_connection(token);
                }
                return Ok(PreparedPacket {
                    send_length: 0,
                    addr_to: default_addr,
                    addr_from: default_addr,
                    if_index: -1,
                    log_cid,
                    last_connection: None,
                    send_msg_size: None,
                });
            }
            Err(error) => return Err(error),
        };

        let if_index = if prepared.if_index == -1 {
            self.connections
                .get(token)
                .map(|cnx| cnx.local_if_index() as i32)
                .unwrap_or(-1)
        } else {
            prepared.if_index
        };
        let send_length = prepared.send_length;
        let addr_to = prepared.addr_to;
        let addr_from = prepared.addr_from;
        let send_msg_size = prepared.send_msg_size;
        let last_connection = self.connections.get_mut(token);

        Ok(PreparedPacket {
            send_length,
            addr_to,
            addr_from,
            if_index,
            log_cid,
            last_connection,
            send_msg_size,
        })
    }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_next_arrival`
C: `picoquic/sim_link.c:93-103 picoquictest_sim_link_next_arrival`
Rust: `rs/fq/src/tests/util.rs:382-391 next_arrival`

### C body
```c
{
    picoquictest_sim_packet_t* packet;
    packet = link->first_packet;

    if (packet != NULL && packet->arrival_time < current_time) {
        current_time = packet->arrival_time;
    }

    return current_time;
}
```

### Rust body
```rust
    pub fn next_arrival(&mut self, current_time: Instant) -> u64 {
        let ct = current_time.ticks();
        if let Some(front) = self.packets.front() {
            let at = front.arrival_time.ticks();
            if at < ct {
                return at;
            }
        }
        ct
    }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_testloss`
C: `picoquic/sim_link.c:144-158 picoquictest_sim_link_testloss`
Rust: `rs/fq/src/tests/harness.rs:17-22 testloss`

### C body
```c
{
    uint64_t loss_bit = 0;

    if (loss_mask != NULL) {
        /* Last bit indicates loss or not */
        loss_bit = (uint64_t)((*loss_mask) & 1ull);

        /* Rotate loss mask by 1 to prepare next round */
        *loss_mask >>= 1;
        *loss_mask |= (loss_bit << 63);
    }

    return (int)loss_bit;
}
```

### Rust body
```rust
    if let Some(mask) = loss_mask.as_mut() {
        let loss_bit = *mask & 1;
        *mask = (*mask >> 1) | (loss_bit << 63);
        loss_bit != 0
    } else {
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_enqueue`
C: `picoquic/sim_link.c:253-294 picoquictest_sim_link_enqueue`
Rust: `rs/fq/src/tests/util.rs:462-492 enqueue`

### C body
```c
{
    if (should_drop) {
        /* simulate congestion loss or random drop on queue full */
        link->packets_dropped++;
        free(packet);
    }
    else {
        uint64_t transmit_time = picoquictest_sim_link_transmit_time(link, packet);
        uint64_t queue_delay = picoquictest_sim_link_queue_delay(link, current_time);

        if (transmit_time <= 0)
            transmit_time = 1;

        link->queue_time = current_time + queue_delay + transmit_time;

        if (packet->length > link->path_mtu || picoquictest_sim_link_testloss(link->loss_mask) != 0 ||
            link->is_switched_off || picoquictest_sim_link_simloss(link, current_time)) {
            link->packets_dropped++;
            free(packet);
        }
        else {
            link->packets_sent++;
            if (link->last_packet == NULL) {
                link->first_packet = packet;
            }
            else {
                link->last_packet->next_packet = packet;
            }
            link->last_packet = packet;
            packet->next_packet = NULL;
            packet->arrival_time = link->queue_time + link->microsec_latency;
            if (link->jitter != 0) {
                packet->arrival_time += picoquictest_sim_link_jitter(link);
            }
            if (packet->arrival_time < link->resume_time) {
                packet->arrival_time = link->resume_time;
            }
        }
    }
}
```

### Rust body
```rust
    pub fn enqueue(&mut self, packet: TestSimPacket, current_time: Instant, should_drop: bool) {
        if should_drop {
            self.packets_dropped += 1;
            return;
        }
        let transmit_time = self.transmit_time(&packet);
        let queue_delay = self.queue_delay(current_time);
        let transmit_time = if transmit_time == 0 { 1 } else { transmit_time };
        self.queue_time = Instant::from_ticks(current_time.ticks() + queue_delay + transmit_time);

        if packet.length > self.path_mtu
            || self.sim_testloss()
            || self.is_switched_off
            || self.sim_simloss(current_time)
        {
            self.packets_dropped += 1;
            return;
        }

        self.packets_sent += 1;
        let arrival_ticks = self.queue_time.ticks() + self.microsec_latency;
        let arrival_ticks = if self.jitter != 0 {
            arrival_ticks + self.sim_jitter()
        } else {
            arrival_ticks
        };
        let arrival_ticks = arrival_ticks.max(self.resume_time.ticks());
        let mut p = packet;
        p.arrival_time = Instant::from_ticks(arrival_ticks);
        self.packets.push_back(p);
    }
```

## Pair `picoquic/sim_link.c:picoquic_test_simlink_suspend`
C: `picoquic/sim_link.c:340-373 picoquic_test_simlink_suspend`
Rust: `rs/fq/src/tests/util.rs:517-533 suspend`

### C body
```c
{
    picoquictest_sim_packet_t* packet;
    picoquictest_sim_packet_t* first_old;

    if (simulate_receive) {
        /* specify the resume time */
        link->resume_time = time_end_of_interval;
        /* packets scheduled to arrive before the end of the interval are rescheduled to
         * that end of interval time.
         */
        packet = link->first_packet;
        while (packet != NULL && packet->arrival_time < time_end_of_interval) {
            packet->arrival_time = time_end_of_interval;
            packet = packet->next_packet;
        }
    }
    else {
        /* Reset the queue delay to the end of interval */
        link->queue_time = time_end_of_interval;
        /* stash the old queue, and reset the queue pointers */
        first_old = link->first_packet;
        link->first_packet = NULL;
        link->last_packet = NULL;
        /* resubmit all packets at the end of interval time */
        packet = first_old;
        while (packet != NULL) {
            picoquictest_sim_packet_t* next_packet = packet->next_packet;
            packet->next_packet = NULL;
            picoquictest_sim_link_submit(link, packet, time_end_of_interval);
            packet = next_packet;
        }
    }
}
```

### Rust body
```rust
    pub fn suspend(&mut self, time_end_of_interval: Instant, simulate_receive: bool) {
        if simulate_receive {
            self.resume_time = time_end_of_interval;
            let end = time_end_of_interval.ticks();
            for p in &mut self.packets {
                if p.arrival_time.ticks() < end {
                    p.arrival_time = time_end_of_interval;
                }
            }
        } else {
            self.queue_time = time_end_of_interval;
            let old: Vec<TestSimPacket> = self.packets.drain(..).collect();
            for p in old {
                self.submit(p, time_end_of_interval);
            }
        }
    }
```

## Pair `picoquic/siphash.c:siphash`
C: `picoquic/siphash.c:85-189 siphash`
Rust: `rs/fq/src/siphash.rs:22-38 siphash`

### C body
```c
            const size_t outlen) {

    const unsigned char *ni = (const unsigned char *)in;
    const unsigned char *kk = (const unsigned char *)k;

    assert((outlen == 8) || (outlen == 16));
    uint64_t v0 = UINT64_C(0x736f6d6570736575);
    uint64_t v1 = UINT64_C(0x646f72616e646f6d);
    uint64_t v2 = UINT64_C(0x6c7967656e657261);
    uint64_t v3 = UINT64_C(0x7465646279746573);
    uint64_t k0 = U8TO64_LE(kk);
    uint64_t k1 = U8TO64_LE(kk + 8);
    uint64_t m;
    int i;
    const unsigned char *end = ni + inlen - (inlen % sizeof(uint64_t));
    const int left = inlen & 7;
    uint64_t b = ((uint64_t)inlen) << 56;
    v3 ^= k1;
    v2 ^= k0;
    v1 ^= k1;
    v0 ^= k0;

    if (outlen == 16)
        v1 ^= 0xee;

    for (; ni != end; ni += 8) {
        m = U8TO64_LE(ni);
        v3 ^= m;

        TRACE;
        for (i = 0; i < cROUNDS; ++i)
            SIPROUND;

        v0 ^= m;
    }

    switch (left) {
    case 7:
        b |= ((uint64_t)ni[6]) << 48;
        /* FALLTHRU */
    case 6:
        b |= ((uint64_t)ni[5]) << 40;
        /* FALLTHRU */
    case 5:
        b |= ((uint64_t)ni[4]) << 32;
        /* FALLTHRU */
    case 4:
        b |= ((uint64_t)ni[3]) << 24;
        /* FALLTHRU */
    case 3:
        b |= ((uint64_t)ni[2]) << 16;
        /* FALLTHRU */
    case 2:
        b |= ((uint64_t)ni[1]) << 8;
        /* FALLTHRU */
    case 1:
        b |= ((uint64_t)ni[0]);
        break;
    case 0:
        break;
    }

    v3 ^= b;

    TRACE;
    for (i = 0; i < cROUNDS; ++i)
        SIPROUND;

    v0 ^= b;

    if (outlen == 16)
        v2 ^= 0xee;
    else
        v2 ^= 0xff;

    TRACE;
    for (i = 0; i < dROUNDS; ++i)
        SIPROUND;

    b = v0 ^ v1 ^ v2 ^ v3;
    U64TO8_LE(out, b);

    if (outlen == 8)
        return 0;

    v1 ^= 0xdd;

    TRACE;
    for (i = 0; i < dROUNDS; ++i)
        SIPROUND;

    b = v0 ^ v1 ^ v2 ^ v3;
    U64TO8_LE(out + 8, b);

    return 0;
}
```

### Rust body
```rust
    fn sip_round(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
        *v0 = v0.wrapping_add(*v1);
        *v1 = v1.rotate_left(13);
        *v1 ^= *v0;
        *v0 = v0.rotate_left(32);
        *v2 = v2.wrapping_add(*v3);
        *v3 = v3.rotate_left(16);
        *v3 ^= *v2;
        *v0 = v0.wrapping_add(*v3);
        *v3 = v3.rotate_left(21);
        *v3 ^= *v0;
        *v2 = v2.wrapping_add(*v1);
        *v1 = v1.rotate_left(17);
        *v1 ^= *v2;
        *v2 = v2.rotate_left(32);
    }
```

## Pair `picoquic/sockloop.c:picoquic_packet_loop_set_fds`
C: `picoquic/sockloop.c:884-905 picoquic_packet_loop_set_fds`
Rust: `rs/fq/src/packet_loop.rs:731-773 packet_loop_set_fds`

### C body
```c
{
    memset(poll_list, 0, sizeof(struct pollfd)* (PICOQUIC_PACKET_LOOP_SOCKETS_MAX+1));
    int i_poll = 0;

    if (thread_ctx->wake_up_defined) {
        poll_list[0].fd = (int)thread_ctx->wake_up_pipe_fd[0];
        poll_list[0].events = POLLIN;
        i_poll = 1;
    }
    for (int i = 0; i < nb_sockets && i < PICOQUIC_PACKET_LOOP_SOCKETS_MAX; i++, i_poll++) {
        poll_list[i_poll].fd = (int)s_ctx[i].fd;
        poll_list[i_poll].events = POLLIN;
    }
    for (; i_poll < PICOQUIC_PACKET_LOOP_SOCKETS_MAX + 1; i_poll++) {
        poll_list[i_poll].fd = -1;
    }
}
```

### Rust body
```rust
) {
    for entry in poll_list.iter_mut() {
        *entry = libc::pollfd {
            fd: 0,
            events: 0,
            revents: 0,
        };
    }

    let mut i_poll = 0usize;

    if thread_ctx.wake_up_defined {
        if let Some(entry) = poll_list.get_mut(0) {
            entry.fd = thread_ctx.wake_up_pipe_fd[0];
            entry.events = libc::POLLIN;
        }
        i_poll = 1;
    }

    for i in 0..nb_sockets.min(PACKET_LOOP_SOCKETS_MAX) {
        if let Some(entry) = poll_list.get_mut(i_poll) {
            entry.fd = s_ctx
                .get(i)
                .and_then(|c| c.fd.as_ref())
                .map(|s| s.raw_fd())
                .unwrap_or(-1);
            entry.events = libc::POLLIN;
        }
        i_poll += 1;
    }

    while i_poll < PACKET_LOOP_SOCKETS_MAX + 1 {
        if let Some(entry) = poll_list.get_mut(i_poll) {
            entry.fd = -1;
        }
        i_poll += 1;
    }
}
```

## Pair `picoquic/sockloop.c:picoquic_packet_loop_v2`
C: `picoquic/sockloop.c:1600-1614 picoquic_packet_loop_v2`
Rust: `rs/fq/src/packet_loop.rs:1265-1282 run_v2`

### C body
```c
{
    picoquic_network_thread_ctx_t thread_ctx = { 0 };

    thread_ctx.quic = quic;
    thread_ctx.param = param;
    thread_ctx.loop_callback = loop_callback;
    thread_ctx.loop_callback_ctx = loop_callback_ctx;

    (void)picoquic_packet_loop_v3((void*)&thread_ctx);
    return thread_ctx.return_code;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let mut thread_ctx = NetworkThreadCtx::default();
        thread_ctx.param = Some(Box::new(*param));
        thread_ctx.loop_callback = loop_callback;
        let mut owned_param = thread_ctx.param.take().ok_or(Error::Memory)?;
        let result = run_packet_loop::<crate::socks_socket2::Socket2Udp>(
            self,
            &mut owned_param,
            &mut thread_ctx,
        );
        *param = *owned_param;
        thread_ctx.param = Some(owned_param);
        result
    }
```

## Pair `picoquic/sockloop.c:picoquic_internal_thread_create`
C: `picoquic/sockloop.c:1727-1731 picoquic_internal_thread_create`
Rust: `rs/fq/src/packet_loop.rs:1464-1468 internal_thread_create`

### C body
```c
{
    int ret = picoquic_create_thread((picoquic_thread_t*)thread_id, thread_fn, thread_arg);
    return ret;
}
```

### Rust body
```rust
) -> Result<JoinHandle<()>, OsError> {
    Ok(std::thread::spawn(thread_fn))
}
```

## Pair `picoquic/sockloop.c:picoquic_start_network_thread`
C: `picoquic/sockloop.c:1814-1818 picoquic_start_network_thread`
Rust: `rs/fq/src/packet_loop.rs:1358-1415 spawn`

### C body
```c
{
    return picoquic_start_custom_network_thread(quic, param, NULL, NULL, NULL, NULL, loop_callback, loop_callback_ctx, ret);
}
```

### Rust body
```rust
    ) -> Result<Box<Self>, OsError> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut thread_ctx = Box::new(NetworkThreadCtx::default());
        thread_ctx.param = Some(Box::new(param));
        thread_ctx.loop_callback = loop_callback;
        thread_ctx.thread_delete_fn = thread_delete_fn;
        thread_ctx.thread_setname_fn = thread_setname_fn;
        thread_ctx.thread_name = thread_name.map(str::to_owned);
        thread_ctx.wake_up_pipe_fd = [-1, -1];
        thread_ctx.wake_up_sender = Some(sender);
        thread_ctx.wake_up_receiver = None;
        thread_ctx.wake_up_defined = true;
        thread_ctx.is_threaded = true;

        let name_for_thread = thread_ctx.thread_name.clone();
        let thread_fn: Box<dyn FnOnce() + Send + 'static> = Box::new(move || {
            if let Some(name) = name_for_thread {
                let _ = name;
            }
            while receiver.recv().is_ok() {}
        });

        let handle = if let Some(mut create_fn) = thread_create_fn {
            create_fn.create(thread_fn)?
        } else if let Some(name) = thread_ctx.thread_name.clone() {
            std::thread::Builder::new()
                .name(name)
                .spawn(thread_fn)
                .map_err(|error| OsError(error.raw_os_error().unwrap_or(-1)))?
        } else {
            internal_thread_create(thread_fn)?
        };
        thread_ctx.pthread = Some(handle);
        thread_ctx.thread_is_ready = true;
        let _ = quic.time();
        Ok(thread_ctx)
    }
```

## Pair `picoquic/sockloop.c:picoquic_start_server_threads`
C: `picoquic/sockloop.c:1966-2065 picoquic_start_server_threads`
Rust: `rs/fq/src/packet_loop.rs:1551-1630 start_server_threads`

### C body
```c
{
    int ret = 0;
    int nb_threads = 0;
    uint8_t default_ticket_key[16] = { 0 };
    /* Set the thread functions to default value if not set */
    if (thread_create_fn == NULL) {
        thread_create_fn = picoquic_internal_thread_create;
    }
    if (thread_delete_fn == NULL) {
        thread_delete_fn = picoquic_internal_thread_delete;
    }
    if (thread_setname_fn == NULL) {
        thread_setname_fn = picoquic_internal_thread_setname;
    }
    /* Check that the number of threads matches the config value
     */
    if (config->nb_threads > nb_threads_max) {
        fprintf(stdout, "Cannot start %d threads, max is %d.\n", config->nb_threads, nb_threads_max);

        ret = -1;
    }
    else if (config->nb_threads < 1) {
        nb_threads = 1;
    }
    else {
        nb_threads = (int)config->nb_threads;
    }
    /* set the ticket encryption key if not present in config */
    if (ret == 0 && config->ticket_encryption_key == NULL) {
        picoquic_quic_t* quic = picoquic_create(1, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, default_ticket_key, current_time, NULL,
            NULL, NULL, 0);
        if (quic != NULL) {
            picoquic_crypto_random(quic, default_ticket_key, sizeof(default_ticket_key));
            picoquic_free(quic);
            config->ticket_encryption_key = default_ticket_key;
            config->ticket_encryption_key_length = sizeof(default_ticket_key);
        }
        else {
            fprintf(stdout, "Cannot create a temporary QUIC context to generate the default ticket encryption key.\n");
        }
    }

    for (int i = 0; ret == 0 && i < nb_threads; i++) {
        /* Allocate and initiate the params field. */
        picoquic_quic_t* qserver = NULL;
        picoquic_packet_loop_param_t* param = NULL;

        ret = picoquic_server_set_context(&qserver, config, current_time, default_callback_fn, default_callback_ctx, alpn_select_fn);
        if (ret != 0) {
            fprintf(stdout, "Cannot create the QUIC context for thread %d.\n", i);
            nb_threads = i;
        }
        else if ((param = (picoquic_packet_loop_param_t*)malloc(sizeof(picoquic_packet_loop_param_t))) == NULL) {
            fprintf(stdout, "Cannot create the packet loop param for thread %d.\n", i);
            nb_threads = i;
            ret = -1;
        }
        else {
            memset(param, 0, sizeof(picoquic_packet_loop_param_t));
            if (param->local_port != 0) {
                param->local_port = (uint16_t)(config->local_port + i);
            }
            param->public_port = config->server_port;
            param->is_port_shared = config->is_port_shared;
            param->local_af = 0;
            param->dest_if = config->dest_if;
            param->socket_buffer_size = config->socket_buffer_size;
            param->do_not_use_gso = config->do_not_use_gso;

            thread_ctxs[i] = picoquic_start_custom_network_thread(qserver, param,
                thread_create_fn, thread_delete_fn, thread_setname_fn, NULL,
                loop_callback_fn, loop_callback_ctx,
                &ret);
            if (thread_ctxs[i] == NULL) {
                picoquic_free(qserver);
                free(param);
                nb_threads = i;
            }
            else {
                thread_ctxs[i]->is_param_allocated = 1;
            }
        }
    }
    *nb_threads_created = nb_threads;
    return ret;
}
```

### Rust body
```rust
    ) -> Result<usize, Error> {
        let nb_threads = if self.nb_threads > thread_ctxs.len() as i32 {
            return Err(Error::InvalidArgument);
        } else if self.nb_threads < 1 {
            1usize
        } else {
            self.nb_threads as usize
        };

        if self.ticket_encryption_key.is_none() {
            let mut key = vec![0u8; 16];
            if let Some(mut quic) = Quic::new(
                1,
                None,
                None,
                None,
                None,
                None,
                None,
                [0u8; 16],
                current_time,
                None,
                None,
            ) {
                rand_core::RngCore::fill_bytes(&mut *quic.rng, &mut key);
            }
            self.ticket_encryption_key = Some(key);
        }

        let mut created = 0usize;
        for slot in thread_ctxs.iter_mut().take(nb_threads) {
            let qserver = Quic::create_server(
                self,
                current_time,
                default_callback.take(),
                alpn_select_fn.take(),
            )?;

            let local_port = if self.local_port != 0 {
                self.local_port + created as u16
            } else {
                0
            };
            let param = LoopParam {
                local_port,
                public_port: self.server_port,
                is_port_shared: self.is_port_shared,
                local_af: 0,
                dest_if: self.dest_if,
                socket_buffer_size: self.socket_buffer_size,
                do_not_use_gso: self.do_not_use_gso,
                ..LoopParam::default()
            };

            let mut qserver = qserver;
            let thread_ctx = NetworkThreadCtx::spawn_custom(
                &mut qserver,
                param,
                thread_create_fn.take(),
                thread_delete_fn.take(),
                thread_setname_fn.take(),
                None,
                loop_callback.take(),
            )
            .map_err(|_| Error::Generic)?;
            *slot = Some(thread_ctx);
            created += 1;
        }
        Ok(created)
    }
```

## Pair `picoquic/spinbit.c:picoquic_spinbit_null_outgoing`
C: `picoquic/spinbit.c:55-59 picoquic_spinbit_null_outgoing`
Rust: `rs/fq/src/spinbit.rs:53-55 outgoing`

### C body
```c
{
    UNREFERENCED_PARAMETER(cnx);
    return 0;
}
```

### Rust body
```rust
    fn outgoing(&self, _connection: &mut Connection) -> u8 {
        0
    }
```

## Pair `picoquic/ticket_store.c:picoquic_serialize_ticket`
C: `picoquic/ticket_store.c:101-158 picoquic_serialize_ticket`
Rust: `rs/fq/src/internal.rs:1324-1384 serialize_ticket`

### C body
```c
{
    int ret = 0;
    size_t byte_index = 0;
    size_t required_length;

    /* Compute serialized length */
    required_length = (size_t)(8 + 2 + 2 + 2 + 4 + 1 + 1) +
        ticket->sni_length + ticket->alpn_length + ticket->ticket_length + 
        ticket->ip_addr_length + ticket->ip_addr_client_length +
        + 8* PICOQUIC_NB_TP_0RTT;
    /* Serialize */
    if (required_length > bytes_max) {
        ret = PICOQUIC_ERROR_FRAME_BUFFER_TOO_SMALL;
        *consumed = 0;
    } else {
        picoformat_64(bytes + byte_index, ticket->time_valid_until);
        byte_index += 8;

        picoformat_16(bytes + byte_index, ticket->sni_length);
        byte_index += 2;
        memcpy(bytes + byte_index, ticket->sni, ticket->sni_length);
        byte_index += ticket->sni_length;

        picoformat_16(bytes + byte_index, ticket->alpn_length);
        byte_index += 2;
        memcpy(bytes + byte_index, ticket->alpn, ticket->alpn_length);
        byte_index += ticket->alpn_length;

        picoformat_32(bytes + byte_index, ticket->version);
        byte_index += 4;

        bytes[byte_index++] = ticket->ip_addr_length;
        if (ticket->ip_addr_length > 0) {
            memcpy(bytes + byte_index, ticket->ip_addr, ticket->ip_addr_length);
            byte_index += ticket->ip_addr_length;
        }
        bytes[byte_index++] = ticket->ip_addr_client_length;
        if (ticket->ip_addr_client_length > 0) {
            memcpy(bytes + byte_index, ticket->ip_addr_client, ticket->ip_addr_client_length);
            byte_index += ticket->ip_addr_client_length;
        }

        for (int i = 0; i < PICOQUIC_NB_TP_0RTT; i++) {
            picoformat_64(bytes + byte_index, ticket->tp_0rtt[i]);
            byte_index += 8;
        }

        picoformat_16(bytes + byte_index, ticket->ticket_length);
        byte_index += 2;
        memcpy(bytes + byte_index, ticket->ticket, ticket->ticket_length);
        byte_index += ticket->ticket_length;

        *consumed = byte_index;
    }

    return ret;
}
```

### Rust body
```rust
fn serialize_ticket(ticket: &StoredTicket) -> Result<Vec<u8>, crate::Error> {
    let sni = optional_string_bytes(ticket.sni.as_deref());
    let alpn = optional_string_bytes(ticket.alpn.as_deref());
    let ip_addr = stored_ip_bytes(ticket.ip_addr);
    let ip_addr_client = stored_ip_bytes(ticket.ip_addr_client);

    if sni.len() > u16::MAX as usize
        || alpn.len() > u16::MAX as usize
        || ticket.ticket.len() > u16::MAX as usize
        || ip_addr.len() > u8::MAX as usize
        || ip_addr_client.len() > u8::MAX as usize
    {
        return Err(crate::Error::BufferTooSmall);
    }

    let required = 8
        + 2
        + sni.len()
        + 2
        + alpn.len()
        + 4
        + 1
        + ip_addr.len()
        + 1
        + ip_addr_client.len()
        + 8 * NB_TP_0RTT
        + 2
        + ticket.ticket.len();
    let mut bytes = vec![0u8; required];
    let mut off = 0;

    format_64(&mut bytes[off..off + 8], ticket.time_valid_until.ticks());
    off += 8;
    format_16(&mut bytes[off..off + 2], sni.len() as u16);
    off += 2;
    bytes[off..off + sni.len()].copy_from_slice(sni);
    off += sni.len();
    format_16(&mut bytes[off..off + 2], alpn.len() as u16);
    off += 2;
    bytes[off..off + alpn.len()].copy_from_slice(alpn);
    off += alpn.len();
    format_32(&mut bytes[off..off + 4], ticket.version);
    off += 4;
    bytes[off] = ip_addr.len() as u8;
    off += 1;
    bytes[off..off + ip_addr.len()].copy_from_slice(&ip_addr);
    off += ip_addr.len();
    bytes[off] = ip_addr_client.len() as u8;
    off += 1;
    bytes[off..off + ip_addr_client.len()].copy_from_slice(&ip_addr_client);
    off += ip_addr_client.len();
    for value in ticket.tp_0rtt {
        format_64(&mut bytes[off..off + 8], value);
        off += 8;
    }
    format_16(&mut bytes[off..off + 2], ticket.ticket.len() as u16);
    off += 2;
    bytes[off..off + ticket.ticket.len()].copy_from_slice(&ticket.ticket);

    Ok(bytes)
}
```
