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

## Pair `picoquic/sender.c:picoquic_implicit_handshake_ack`
C: `picoquic/sender.c:1835-1866 picoquic_implicit_handshake_ack`
Rust: `rs/fq/src/internal.rs:7582-7587 implicit_handshake_ack`

### C body
```c
{
    picoquic_packet_t* p = cnx->pkt_ctx[pc].pending_first;

    /* Remove packets from the retransmit queue */
    while (p != NULL) {
        picoquic_packet_t* p_next = p->packet_next;
        picoquic_path_t * old_path = p->send_path;

        /* Update the congestion control state for the path, but only for the packets sent
         * before the initial timer. */
        if (old_path != NULL && cnx->congestion_alg != NULL && p->send_time < cnx->start_time + PICOQUIC_INITIAL_RTT) {
            picoquic_per_ack_state_t ack_state = { 0 };
            ack_state.pc = pc;
            ack_state.rtt_measurement = old_path->rtt_sample;
            ack_state.nb_bytes_acknowledged = p->length;
            old_path->delivered += p->length;
            ack_state.nb_bytes_delivered_since_packet_sent = old_path->delivered - p->delivered_prior;
            ack_state.is_app_limited = 1;

            cnx->congestion_alg->alg_notify(cnx, old_path,
                picoquic_congestion_notification_acknowledgement,
                &ack_state, current_time);
        }
        /* Update the number of bytes in transit and remove old packet from queue */
        /* The packet will not be placed in the "retransmitted" queue */
        (void)picoquic_dequeue_retransmit_packet(cnx, &cnx->pkt_ctx[pc], p, 1, 0);

        p = p_next;
    }
}
```

### Rust body
```rust
    pub fn implicit_handshake_ack(&mut self, pc: PacketContext, _current_time: Instant) {
        let pkt_ctx = &mut self.pkt_ctx[pc as usize];
        pkt_ctx.pending.clear();
        pkt_ctx.retransmitted.clear();
        pkt_ctx.retransmitted_queue_size = 0;
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_packet_closing`
C: `picoquic/sender.c:2351-2594 picoquic_prepare_packet_closing`
Rust: `rs/fq/src/internal.rs:18469-18668 prepare_packet_closing`

### C body
```c
{
    int ret = 0;
    /* TODO: manage multiple streams. */
    picoquic_packet_type_enum packet_type = 0;
    size_t checksum_overhead = 8;
    size_t header_length = 0;
    uint8_t* bytes = packet->bytes;
    uint8_t* bytes_max;
    uint8_t* bytes_next;
    int more_data = 0;
    size_t length = 0;
    int is_pure_ack = 1;
    picoquic_packet_context_enum pc = picoquic_packet_context_application;
    picoquic_packet_context_t * pkt_ctx;
    picoquic_epoch_enum epoch = picoquic_epoch_1rtt;

    /* The only purpose of the test below is to appease the static analyzer, so it
     * wont complain of possible NULL deref. On windows we could use "__assume(path_x != NULL)"
     * but the documentation does not say anything about that for GCC and CLANG */
    if (path_x == NULL) {
        return PICOQUIC_ERROR_UNEXPECTED_ERROR;
    }

    send_buffer_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;

    /* Prepare header -- depend on connection state */
    /* TODO: 0-RTT work. */
    switch (cnx->cnx_state) {
    case picoquic_state_handshake_failure:
        /* TODO: check whether closing can be requested in "initial" mode */
        if (cnx->crypto_context[picoquic_epoch_handshake].aead_encrypt != NULL &&
            picoquic_sack_list_first(&cnx->ack_ctx[picoquic_packet_context_handshake].sack_list) != UINT64_MAX) {
            pc = picoquic_packet_context_handshake;
            packet_type = picoquic_packet_handshake;
            epoch = picoquic_epoch_handshake;
        }
        else {
            pc = picoquic_packet_context_initial;
            packet_type = picoquic_packet_initial;
        }
        break;
    case picoquic_state_handshake_failure_resend:
        pc = picoquic_packet_context_handshake;
        packet_type = picoquic_packet_handshake;
        epoch = picoquic_epoch_handshake;
        break;
    case picoquic_state_disconnecting:
        packet_type = picoquic_packet_1rtt_protected;
        break;
    case picoquic_state_closing_received:
        packet_type = picoquic_packet_1rtt_protected;
        break;
    case picoquic_state_closing:
        packet_type = picoquic_packet_1rtt_protected;
        break;
    case picoquic_state_draining:
        packet_type = picoquic_packet_1rtt_protected;
        break;
    case picoquic_state_disconnected:
        ret = PICOQUIC_ERROR_DISCONNECTED;
        break;
    default:
        ret = -1;
        break;
    }

    /* At this stage, we don't try to retransmit any old packet, whether in
     * the current context or in previous contexts. */

    if (packet_type == picoquic_packet_1rtt_protected && cnx->is_multipath_enabled) {
        pkt_ctx = &path_x->pkt_ctx;
    }
    else {
        pkt_ctx = &cnx->pkt_ctx[pc];
    }

    checksum_overhead = picoquic_get_checksum_length(cnx, epoch);
    packet->pc = pc;
    bytes_max = bytes + send_buffer_max - checksum_overhead;

    if (ret == 0 && cnx->cnx_state == picoquic_state_closing_received) {
        /* Send a closing frame, move to draining state */
        uint64_t exit_time = cnx->latest_progress_time + 3 * path_x->retransmit_timer;

        length = picoquic_predict_packet_header_length(cnx, packet_type, pkt_ctx);
        bytes_next = bytes + length;
        packet->ptype = packet_type;
        packet->offset = length;
        header_length = length;
        packet->sequence_number = pkt_ctx->send_sequence;
        packet->send_time = current_time;
        packet->send_path = path_x;

        /* Send the disconnect frame */
        bytes_next = picoquic_format_connection_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
        length = bytes_next - bytes;
        cnx->last_close_sent = current_time;
        cnx->cnx_state = picoquic_state_draining;
        *next_wake_time = exit_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
    } else if (ret == 0 && cnx->cnx_state == picoquic_state_closing) {
        /* if more than 3*RTO is elapsed, move to disconnected */
        uint64_t exit_time = cnx->latest_progress_time + 3 * path_x->retransmit_timer;
        uint64_t next_close_time = cnx->last_close_sent + path_x->smoothed_rtt;

        if (current_time >= exit_time) {
            picoquic_connection_disconnect(cnx);
            *next_wake_time = current_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        else if (current_time >= next_close_time) {
            uint64_t delta_t = path_x->rtt_min;
            uint64_t next_time = 0;

            if (delta_t * 2 < path_x->retransmit_timer) {
                delta_t = path_x->retransmit_timer / 2;
            }
            /* if more than N packet received, repeat and erase */
            if (cnx->ack_ctx[pc].act[0].ack_needed) {
                length = picoquic_predict_packet_header_length(
                    cnx, packet_type, pkt_ctx);
                packet->ptype = packet_type;
                packet->offset = length;
                header_length = length;
                packet->sequence_number = pkt_ctx->send_sequence;
                packet->send_time = current_time;
                packet->send_path = path_x;
                bytes_next = bytes + length;

                /* Resend the disconnect frame */
                if (cnx->local_error == 0) {
                    bytes_next = picoquic_format_application_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
                } else {
                    bytes_next = picoquic_format_connection_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
                }
                length = bytes_next - bytes;
                cnx->ack_ctx[pc].act[0].ack_needed = 0;
                cnx->ack_ctx[pc].act[0].out_of_order_received = 0;
                cnx->last_close_sent = current_time;
            }
            next_time = current_time + delta_t;
            if (next_time > exit_time) {
                next_time = exit_time;
            }

            *next_wake_time = next_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        else {
            if (next_close_time > exit_time) {
                next_close_time = exit_time;
            }
            if (*next_wake_time > next_close_time) {
                *next_wake_time = next_close_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
            }
        }
    } else if (ret == 0 && cnx->cnx_state == picoquic_state_draining) {
        /* Nothing is ever sent in the draining state */
        /* if more than 3*RTO is elapsed, move to disconnected */
        uint64_t exit_time = cnx->latest_progress_time + 3 * path_x->retransmit_timer;

        if (current_time >= exit_time) {
            picoquic_connection_disconnect(cnx);
            *next_wake_time = current_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        else {
            *next_wake_time = exit_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        length = 0;
    } else if (ret == 0 && (cnx->cnx_state == picoquic_state_disconnecting || 
        cnx->cnx_state == picoquic_state_handshake_failure || 
        cnx->cnx_state == picoquic_state_handshake_failure_resend)) {

        length = picoquic_predict_packet_header_length(
            cnx, packet_type, pkt_ctx);
        bytes_next = bytes + length;
        packet->ptype = packet_type;
        packet->offset = length;
        header_length = length;
        packet->sequence_number = pkt_ctx->send_sequence;
        packet->send_time = current_time;
        packet->send_path = path_x;

        /* send either app close or connection close, depending on error code */
        uint64_t delta_t = path_x->rtt_min;

        if (2 * delta_t < path_x->retransmit_timer) {
            delta_t = path_x->retransmit_timer / 2;
        }

        /* add a final ack so receiver gets clean state */
        bytes_next = picoquic_format_ack_frame(cnx, bytes_next, bytes_max, &more_data, current_time, pc, 0);

        /* Send the disconnect frame */
        if (cnx->local_error == 0) {
            bytes_next = picoquic_format_application_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
        }
        else {
            bytes_next = picoquic_format_connection_close_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
        }
        length = bytes_next - bytes;

        if (cnx->cnx_state == picoquic_state_handshake_failure) {
            if (pc == picoquic_packet_context_initial &&
                cnx->crypto_context[2].aead_encrypt != NULL) {
                cnx->cnx_state = picoquic_state_handshake_failure_resend;
            }
            else {
                picoquic_connection_disconnect(cnx);
            }
        }
        else if (cnx->cnx_state == picoquic_state_handshake_failure_resend) {
            picoquic_connection_disconnect(cnx);
        }
        else {
            cnx->cnx_state = picoquic_state_closing;
        }
        cnx->latest_progress_time = current_time;
        cnx->last_close_sent = current_time;
        *next_wake_time = current_time + delta_t;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        cnx->ack_ctx[pc].act[0].ack_needed = 0;
    }
    else {
        length = 0;
    }

    if (length > 0 && packet->ptype == picoquic_packet_initial && cnx->client_mode) {
        length = picoquic_pad_to_target_length(bytes, length, send_buffer_max - checksum_overhead);
    }

    picoquic_finalize_and_protect_packet(cnx, packet,
        ret, length, header_length, checksum_overhead,
        send_length, send_buffer, send_buffer_max,
        path_x, current_time);

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let mut ret = 0;
        let mut pc = PacketContext::Application;
        let mut packet_type = PacketType::OneRttProtected;
        let mut epoch = Epoch::OneRtt;
        match self.connection_state {
            State::HandshakeFailure => {
                if self.crypto_context[Epoch::Handshake as usize]
                    .aead_encrypt
                    .is_some()
                    && !self.ack_ctx[PacketContext::Handshake as usize]
                        .sack_list
                        .is_empty()
                {
                    pc = PacketContext::Handshake;
                    packet_type = PacketType::Handshake;
                    epoch = Epoch::Handshake;
                } else {
                    pc = PacketContext::Initial;
                    packet_type = PacketType::Initial;
                    epoch = Epoch::Initial;
                }
            }
            State::HandshakeFailureResend => {
                pc = PacketContext::Handshake;
                packet_type = PacketType::Handshake;
                epoch = Epoch::Handshake;
            }
            State::Disconnecting | State::ClosingReceived | State::Closing | State::Draining => {}
            State::Disconnected => ret = crate::errors::InternalError::Disconnected as i32,
            _ => ret = crate::errors::InternalError::UnexpectedState as i32,
        }

        let checksum_overhead = self.get_checksum_length(epoch);
        let send_buffer_max = send_buffer_max.min(path_x.send_mtu);
        let bytes_limit = send_buffer_max
            .saturating_sub(checksum_overhead)
            .min(packet.bytes.len());
        let mut header_length = self.predict_packet_header_length_for_pc(packet_type, pc);
        let mut length = 0usize;
        let mut more_data = 0;
        let mut is_pure_ack = 1;

        if ret == 0
            && matches!(
                self.connection_state,
                State::ClosingReceived
                    | State::Closing
                    | State::Disconnecting
                    | State::HandshakeFailure
                    | State::HandshakeFailureResend
            )
        {
            if self.connection_state == State::Closing {
                let exit_time = self
                    .latest_progress_time
                    .ticks()
                    .saturating_add(3u64.saturating_mul(path_x.retransmit_timer.ticks()));
                let next_close_time = self
                    .last_close_sent
                    .ticks()
                    .saturating_add(path_x.smoothed_rtt.ticks());
                if current_time.ticks() >= exit_time {
                    self.connection_disconnect();
                    self.set_sender_wake_now(next_wake_time, current_time);
                } else if current_time.ticks() < next_close_time {
                    *next_wake_time = Instant::from_ticks(next_close_time.min(exit_time));
                }
            }
            if self.connection_state != State::Disconnected
                && self.connection_state != State::Draining
            {
                length = header_length;
                packet.packet_type = packet_type;
                packet.offset = header_length;
                packet.sequence_number =
                    if packet_type == PacketType::OneRttProtected && self.is_multipath_enabled {
                        path_x.pkt_ctx.send_sequence
                    } else {
                        self.pkt_ctx[pc as usize].send_sequence
                    };
                packet.send_time = current_time;
                packet.send_path = Some(Self::path_token_for_path(path_x));
                packet.packet_context = pc;
                if length <= bytes_limit {
                    let mut offset = length;
                    if matches!(
                        self.connection_state,
                        State::Disconnecting
                            | State::HandshakeFailure
                            | State::HandshakeFailureResend
                    ) {
                        let tail_len = {
                            let tail = &mut packet.bytes[offset..bytes_limit];
                            match format_ack_frame(self, tail, &mut more_data, current_time, pc, 0)
                            {
                                Some(next) => next.len(),
                                None => tail.len(),
                            }
                        };
                        offset = bytes_limit.saturating_sub(tail_len);
                    }
                    let tail_len = {
                        let tail = &mut packet.bytes[offset..bytes_limit];
                        let next = if self.local_error == 0 {
                            format_application_close_frame(
                                self,
                                tail,
                                &mut more_data,
                                &mut is_pure_ack,
                            )
                        } else {
                            format_connection_close_frame(
                                self,
                                tail,
                                &mut more_data,
                                &mut is_pure_ack,
                            )
                        };
                        match next {
                            Some(next) => next.len(),
                            None => {
                                ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                                tail.len()
                            }
                        }
                    };
                    length = bytes_limit.saturating_sub(tail_len);
                }
                self.last_close_sent = current_time;
                match self.connection_state {
                    State::ClosingReceived => self.connection_state = State::Draining,
                    State::HandshakeFailure => {
                        if pc == PacketContext::Initial
                            && self.crypto_context[Epoch::Handshake as usize]
                                .aead_encrypt
                                .is_some()
                        {
                            self.connection_state = State::HandshakeFailureResend;
                        } else {
                            self.connection_disconnect();
                        }
                    }
                    State::HandshakeFailureResend => self.connection_disconnect(),
                    State::Disconnecting => self.connection_state = State::Closing,
                    _ => {}
                }
                self.latest_progress_time = current_time;
                let mut delta_t = path_x.rtt_min.ticks();
                if delta_t.saturating_mul(2) < path_x.retransmit_timer.ticks() {
                    delta_t = path_x.retransmit_timer.ticks() / 2;
                }
                *next_wake_time = Instant::from_ticks(current_time.ticks().saturating_add(delta_t));
                self.ack_ctx[pc as usize].act[0].ack_needed = false;
            }
        } else if ret == 0 && self.connection_state == State::Draining {
            let exit_time = self
                .latest_progress_time
                .ticks()
                .saturating_add(3u64.saturating_mul(path_x.retransmit_timer.ticks()));
            if current_time.ticks() >= exit_time {
                self.connection_disconnect();
                self.set_sender_wake_now(next_wake_time, current_time);
            } else {
                *next_wake_time = Instant::from_ticks(exit_time);
            }
            header_length = 0;
            length = 0;
        }

        if length > 0 && packet.packet_type == PacketType::Initial && self.client_mode {
            length = pad_to_target_length(
                &mut packet.bytes,
                length,
                send_buffer_max.saturating_sub(checksum_overhead),
            );
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
        ret
    }
```

## Pair `picoquic/sender.c:picoquic_ready_state_transition`
C: `picoquic/sender.c:2698-2775 picoquic_ready_state_transition`
Rust: `rs/fq/src/internal.rs:7719-7755 ready_state_transition`

### C body
```c
{
    /* Transition to server ready state.
     * The handshake is complete, all the handshake packets are implicitly acknowledged */
    cnx->cnx_state = picoquic_state_ready;
    cnx->is_handshake_finished = 1;
    picoquic_implicit_handshake_ack(cnx, picoquic_packet_context_initial, current_time);
    picoquic_implicit_handshake_ack(cnx, picoquic_packet_context_handshake, current_time);

    (void)picoquic_register_net_secret(cnx);
    if (!cnx->quic->use_predictable_random) {
        picoquic_public_random_seed(cnx->quic);
    }

    if (!cnx->client_mode) {
        (void)picoquic_queue_handshake_done_frame(cnx);
    }

    if (cnx->is_half_open){
        if (cnx->quic->current_number_half_open > 0) {
            cnx->quic->current_number_half_open--;
        }
        cnx->is_half_open = 0;
        if (cnx->quic->current_number_half_open < cnx->quic->max_half_open_before_retry) {
            cnx->quic->check_token = cnx->quic->force_check_token;
        }
    }

    /* Remove handshake and initial keys if they are still around */
    picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_initial]);
    picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_0rtt]);
    picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_handshake]);

    /* Remove the frames queued in initial and handshake contexts */
    picoquic_purge_misc_frames_after_ready(cnx);

    /* Trim the memory buffers allocated during handshake */
    picoquic_tlscontext_trim_after_handshake(cnx);

    /* Set the confidentiality limit if not already set */
    if (cnx->crypto_epoch_length_max == 0) {
        cnx->crypto_epoch_length_max = 
            picoquic_aead_confidentiality_limit(cnx->crypto_context[picoquic_epoch_1rtt].aead_decrypt);
    }

    /* Start migration to server preferred address if present */
    if (cnx->client_mode) {
        (void)picoquic_prepare_server_address_migration(cnx);
    }

    /* Notify the application */
    if (cnx->callback_fn != NULL) {
        if (cnx->callback_fn(cnx, 0, NULL, 0, picoquic_callback_ready, cnx->callback_ctx, NULL) != 0) {
            picoquic_log_app_message(cnx, "Callback ready returns error 0x%x", PICOQUIC_TRANSPORT_INTERNAL_ERROR);
            picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0);
        }
    }

    /* Ask for ACK frequency update, or initialize variables if not available */
    if (cnx->is_ack_frequency_negotiated) {
        cnx->is_ack_frequency_updated = 1;
    }
    else {
        picoquic_compute_ack_gap_and_delay(cnx, cnx->path[0]->rtt_min, PICOQUIC_ACK_DELAY_MIN,
            cnx->path[0]->receive_rate_max, &cnx->ack_gap_remote, &cnx->ack_delay_remote);

        /* Keep track of statistics on ACK parameters */
        if (cnx->ack_gap_remote > cnx->max_ack_gap_remote) {
            cnx->max_ack_gap_remote = cnx->ack_gap_remote;
        }
        if (cnx->ack_delay_remote > cnx->max_ack_delay_remote) {
            cnx->max_ack_delay_remote = cnx->ack_delay_remote;
        }
        else if (cnx->ack_delay_remote < cnx->min_ack_delay_remote) {
            cnx->min_ack_delay_remote = cnx->ack_delay_remote;
        }
    }
}
```

### Rust body
```rust
    pub fn ready_state_transition(&mut self, current_time: Instant) {
        self.connection_state = crate::State::Ready;
        self.is_handshake_finished = true;
        self.implicit_handshake_ack(PacketContext::Initial, current_time);
        self.implicit_handshake_ack(PacketContext::Handshake, current_time);
        if !self.client_mode {
            let _ = self.queue_handshake_done_frame();
        }
        if self.is_half_open {
            self.is_half_open = false;
        }
        self.purge_misc_frames_after_ready();
        if self.is_ack_frequency_negotiated {
            self.is_ack_frequency_updated = true;
        } else {
            let rtt = self.paths.first().map(|p| p.rtt_min).unwrap_or(INITIAL_RTT);
            let rate = self.paths.first().map(|p| p.receive_rate_max).unwrap_or(0);
            let mut ack_gap = 0;
            let mut ack_delay = 0;
            self.compute_ack_gap_and_delay(
                rtt,
                ACK_DELAY_MIN.ticks(),
                rate,
                &mut ack_gap,
                &mut ack_delay,
            );
            self.ack_gap_remote = ack_gap;
            self.ack_delay_remote = Duration::from_ticks(ack_delay);
            self.max_ack_gap_remote = self.max_ack_gap_remote.max(ack_gap);
            self.max_ack_delay_remote = self
                .max_ack_delay_remote
                .max(Duration::from_ticks(ack_delay));
            self.min_ack_delay_remote = self
                .min_ack_delay_remote
                .min(Duration::from_ticks(ack_delay));
        }
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_packet_ready`
C: `picoquic/sender.c:3286-3717 picoquic_prepare_packet_ready`
Rust: `rs/fq/src/internal.rs:18674-19300 prepare_packet_ready`

### C body
```c
{
    int ret = 0;
    picoquic_packet_type_enum packet_type = picoquic_packet_1rtt_protected;
    picoquic_packet_context_enum pc = picoquic_packet_context_application;
    int is_pure_ack = 1;
    size_t header_length = 0;
    size_t length = 0;
    size_t checksum_overhead = picoquic_get_checksum_length(cnx, picoquic_epoch_1rtt);
    size_t send_buffer_min_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;
    uint8_t* bytes = packet->bytes;
    uint8_t* bytes_max = bytes + send_buffer_min_max - checksum_overhead;
    uint8_t* bytes_next;
    int more_data = 0;
    int ack_sent = 0;
    int is_challenge_padding_needed = 0;
    int is_nominal_ack_path = (cnx->is_multipath_enabled) ?
        (path_x->is_nominal_ack_path || cnx->nb_paths == 1) : path_x == cnx->path[0];

    picoquic_packet_context_t* pkt_ctx = (cnx->is_multipath_enabled) ?
        &path_x->pkt_ctx :
        &cnx->pkt_ctx[picoquic_packet_context_application];

    /* Check whether to insert a hole in the sequence of packets */
    if (pkt_ctx->send_sequence >= pkt_ctx->next_sequence_hole) {
        picoquic_insert_hole_in_send_sequence_if_needed(cnx, path_x, pkt_ctx, current_time, next_wake_time);
    }

    packet->pc = picoquic_packet_context_application;

    /* If there was no packet sent on this path for a long time, rotate the
     * CID prior to sending a new packet. The point is to make it harder for
     * casual observers to track traffic, especially across NAT resets.
     * Long time is defined by either a 5 second refresh delay or 3 RTTs,
     * whichever is longer.
     */
    
    if (cnx->client_mode &&
        path_x->first_tuple->challenge_verified &&
        !path_x->path_cid_rotated &&
        path_x->latest_sent_time + PICOQUIC_CID_REFRESH_DELAY < current_time &&
        path_x->latest_sent_time + 3*path_x->rtt_min < current_time)
    {
        /* Ignore renewal failure mode, since this is an optional feature */
        (void)picoquic_renew_path_connection_id(cnx, path_x);
        path_x->path_cid_rotated = 1;
    }

    /* If the number of packets sent is larger that the max length of
     * a crypto epoch, prepare a key rotation */
    if ((cnx->nb_packets_sent - cnx->crypto_epoch_sequence >
        cnx->crypto_epoch_length_max) &&
        current_time > cnx->crypto_rotation_time_guard) {
        if (picoquic_start_key_rotation(cnx) != 0) {
            picoquic_log_app_message(cnx, "Cannot start key rotation after %"PRIu64" packets",
                cnx->pkt_ctx[picoquic_packet_context_application].send_sequence);
        }
    }

    /* The first action is normally to retransmit lost packets. These lost packets
     * are queued in the connection context as `cnx->data_repeat_first` when data 
     * frames need to be repeated, and under `cnx->first_misc_frame` when other
     * individual frames need repetition. */
    if (cnx->first_misc_frame == NULL && 
        (length = picoquic_retransmit_needed(cnx, pc, path_x, current_time, next_wake_time, packet, 
        send_buffer_min_max, &header_length)) > 0) {
        /* Check whether it makes sense to add an ACK at the end of the retransmission */
        /* Testing header length for defense in depth -- avoid creating new packet if
         * picoquic_retransmit_needed erroneously returns length <= header_length */
        if (bytes + length + 256 < bytes_max  && length > header_length) {
            /* Don't do that if it risks mixing clear text and encrypted ack */
            bytes_next = picoquic_format_ack_frame(cnx, bytes + length, bytes_max, &more_data,
                current_time, pc, !is_nominal_ack_path);
            length = bytes_next - bytes;
        }
        /* document the send time & overhead */
        is_pure_ack = 0;
        packet->send_time = current_time;
        packet->checksum_overhead = checksum_overhead;
    }
    else if (cnx->cnx_state == picoquic_state_disconnected) {
        DBG_PRINTF("%s", "Retransmission check caused a disconnect");
    }
    else {
        length = picoquic_predict_packet_header_length(
            cnx, packet_type, pkt_ctx);
        packet->ptype = packet_type;
        packet->offset = length;
        header_length = length;
        packet->sequence_number = pkt_ctx->send_sequence;
        packet->send_time = current_time;
        packet->send_path = path_x;
        bytes_next = bytes + length;

        /* If required, prepare challenge and response frames.
         * These frames will be sent immediately, regardless of pacing or flow control.
         */
        bytes_next = picoquic_prepare_path_challenge_frames(cnx, path_x,
            bytes_next, bytes_max,
            &more_data, &is_pure_ack, &is_challenge_padding_needed,
            current_time, next_wake_time);

        /* Compute the length before pacing block */
        length = bytes_next - bytes;

        if (path_x->is_multipath_probe_needed) {
            packet->is_multipath_probe = 1;
            path_x->is_multipath_probe_needed = 0;
            is_pure_ack = 0;
            *bytes_next = picoquic_frame_type_ping;
            length++;
            length = picoquic_pad_to_target_length(bytes, length, (uint32_t)(send_buffer_min_max - checksum_overhead));
            bytes_next = bytes + length;
        } else if (cnx->cnx_state != picoquic_state_disconnected && path_x->first_tuple->challenge_verified != 0) {
            /* There are no frames yet that would be exempt from pacing control, but if there
             * was they should be sent here. */

            if (picoquic_is_sending_authorized_by_pacing(cnx, path_x, current_time, next_wake_time)) {
                /* Send here the frames that are not exempt from the pacing control,
                 * but are exempt for congestion control */
                if (picoquic_is_ack_needed(cnx, current_time, next_wake_time, pc, !is_nominal_ack_path)) {
                    uint8_t* bytes_ack = bytes_next;
                    bytes_next = picoquic_format_ack_frame(cnx, bytes_next, bytes_max, &more_data,
                        current_time, pc, !is_nominal_ack_path);
                    ack_sent = (bytes_next > bytes_ack);
                }

                /* if necessary, prepare the MAX STREAM frames */
                if (ret == 0) {
                    bytes_next = picoquic_format_max_streams_frame_if_needed(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
                }

                /* If necessary, encode the max data frame */
                if (ret == 0){
                    if (cnx->quic->max_data_limit != 0) {
                        if (cnx->data_received + ((3 * cnx->quic->max_data_limit) / 4) > cnx->maxdata_local) {
                            uint64_t max_data_increase = cnx->data_received + cnx->quic->max_data_limit - cnx->maxdata_local;
                            bytes_next = picoquic_format_max_data_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack,
                                max_data_increase);
                        }
                    }
                    else if (2 * cnx->offset_received > cnx->maxdata_local) {
                        bytes_next = picoquic_format_max_data_frame(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack,
                            picoquic_cc_increased_window(cnx, cnx->maxdata_local));
                    }
                }

                /* If necessary, encode the max stream data frames */
                if (ret == 0 && cnx->max_stream_data_needed) {
                    bytes_next = picoquic_format_required_max_stream_data_frames(cnx, bytes_next, bytes_max, &more_data, &is_pure_ack);
                }
                /* Funky code alert:
                * if misc frames are present the function `picoquic_retransmit_needed` is bypassed.
                * if "more data" was not set, the code would not reset the wait time, and the
                * program could stall.
                * TODO: rework the way packets are repeated so this is not necessary.
                */
                if (cnx->first_misc_frame != NULL) {
                    more_data = 1;
                }
                /* If present, send misc frame */
                bytes_next = picoquic_format_misc_frames_in_context(cnx, bytes_next, bytes_max,
                    &more_data, &is_pure_ack, pc);

                /* Compute the length before entering the CC block */
                length = bytes_next - bytes;

                if ((path_x->cwin < path_x->bytes_in_transit || cnx->quic->cwin_max < path_x->bytes_in_transit)
                    && !path_x->is_pto_required) {
                    /* Implementation of experimental API, picoquic_set_priority_limit_for_bypass */
                    uint8_t* bytes_next_before_bypass = bytes_next;
                    int no_data_to_send = 0;
                    if (cnx->priority_limit_for_bypass > 0 && cnx->nb_paths == 1) {
                        bytes_next = picoquic_prepare_stream_and_datagrams(cnx, path_x, bytes_next, bytes_max,
                            (size_t)(bytes_next - bytes) <= packet->offset, cnx->priority_limit_for_bypass,
                            &more_data, &is_pure_ack, &no_data_to_send, &ret);
                    }
                    if (bytes_next != bytes_next_before_bypass) {
                        length = bytes_next - bytes;
                    }
                    else {
                        cnx->cwin_blocked = 1;
                        path_x->last_cwin_blocked_time = current_time;
                        if (cnx->congestion_alg != NULL) {
                            picoquic_per_ack_state_t ack_state = { 0 };
                            ack_state.pc = pc;
                            cnx->congestion_alg->alg_notify(cnx, path_x,
                                picoquic_congestion_notification_cwin_blocked,
                                &ack_state, current_time);
                        }
                    }
                }
                else {
                    /* Send here the frames that are subject to both congestion and pacing control.
                     * this includes the PMTU probes.
                     * Check whether PMTU discovery is required. The call will return
                     * three values: not needed at all, optional, or required.
                     * If required, PMTU discovery takes priority over sending stream data.
                     */
                    int no_data_to_send = 1;
                    int preemptive_repeat = 0;
                    picoquic_pmtu_discovery_status_enum pmtu_discovery_needed = picoquic_is_mtu_probe_needed(cnx, path_x);

                    /* if present, send tls data */
                    if (picoquic_is_tls_stream_ready(cnx)) {
                        bytes_next = picoquic_format_crypto_hs_frame(&cnx->tls_stream[picoquic_epoch_1rtt],
                            bytes_next, bytes_max, &more_data, &is_pure_ack);
                    }

                    if (cnx->is_address_discovery_provider) {
                        /* If a new address was learned, prepare an observed address frame */
                        /* TODO: tie this code to processing of paths */
                        bytes_next = picoquic_prepare_observed_address_frame(bytes_next, bytes_max,
                            path_x, path_x->first_tuple, current_time, next_wake_time, &more_data, &is_pure_ack);
                    }

                    if (length > header_length || pmtu_discovery_needed != picoquic_pmtu_discovery_required ||
                        send_buffer_max <= path_x->send_mtu) {
                        /* No need or no way to do path MTU discovery, just go on with formatting packets */
                        /* If there are not enough local CID published, create and advertise */
                        if (ret == 0) {
                            bytes_next = picoquic_format_new_local_id_as_needed(cnx, bytes_next, bytes_max,
                                current_time, next_wake_time, &more_data, &is_pure_ack);
                        }
                        if (ret == 0 && cnx->is_ack_frequency_updated && cnx->is_ack_frequency_negotiated) {
                            bytes_next = picoquic_format_ack_frequency_frame(cnx, bytes_next, bytes_max, &more_data);
                        }
                        if (ret == 0) {
                            bytes_next = picoquic_prepare_stream_and_datagrams(cnx, path_x, bytes_next, bytes_max,
                                (size_t)(bytes_next - bytes) <= packet->offset, UINT64_MAX, &more_data, &is_pure_ack, &no_data_to_send, &ret);
                        }

                        /* TODO: replace this by scheduling of BDP frame when window has been estimated */
                        /* Send bdp frames if there are no stream frames to send 
                         * and if peer wishes to receive bdp frames */
                        if(!cnx->client_mode && cnx->send_receive_bdp_frame) {
                           bytes_next = picoquic_format_bdp_frame(cnx, bytes_next, bytes_max, path_x, &more_data, &is_pure_ack);
                        }

                        length = bytes_next - bytes;

                        if (length <= header_length || is_pure_ack) {
                            /* Mark the bandwidth estimation as application limited */
                            path_x->delivered_limited_index = path_x->delivered;
                            /* Notify the peer if something is blocked */
                            bytes_next = picoquic_format_blocked_frames(cnx, &bytes[length], bytes_max, &more_data, &is_pure_ack);
                            length = bytes_next - bytes;
                        }

                        if (cnx->is_preemptive_repeat_enabled ||
                            (cnx->is_forced_probe_up_required && path_x->is_cca_probing_up)) {
                            if (length <= header_length) {
                                /* Consider redundant retransmission:
                                 * if the redundant retransmission index is null:
                                 * - if the packet loss rate is large enough compared to BDP, set index to last sent packet.
                                 * - if not, do not perform redundant retransmission.
                                 * if the packet contains a stream frame, if that stream is finished, and if the
                                 * data range has not been acked, and it fits: copy it to the data. Move the index to the previous packet.
                                 */
                                 ret = picoquic_preemptive_retransmit_as_needed(cnx, path_x, pc, current_time, next_wake_time, bytes_next,
                                    bytes_max - bytes_next, &length, &more_data, &is_pure_ack);
                                 if (length > header_length) {
                                     preemptive_repeat = 1;
                                     packet->is_preemptive_repeat = 1;
                                     bytes_next = bytes + length;
                                 }
                                 else if (cnx->is_forced_probe_up_required && path_x->is_cca_probing_up) {
                                     *bytes_next++ = picoquic_frame_type_ping;
                                     memset(bytes_next, picoquic_frame_type_padding, bytes_max - bytes_next);
                                     bytes_next = bytes_max;
                                     length = bytes_next - bytes;
                                     is_pure_ack = 0;
                                 }
                            }
                            else if (!more_data){
                                /* Check whether preemptive retrasmission is needed. Same code as above,
                                 * but in "test_only" mode, will set "more_data" or wait time if repeat is ready 
                                 */
                                ret = picoquic_preemptive_retransmit_as_needed(cnx, path_x, pc, current_time, next_wake_time, bytes_next,
                                    bytes_max - bytes_next, &length, &more_data, NULL);
                            }
                        }

                        if (no_data_to_send && !preemptive_repeat) {
                            path_x->last_sender_limited_time = current_time;
                        }
                    } /* end of PMTU not required */

                    if (ret == 0 && path_x->is_pto_required){
                        if ((length <= header_length || is_pure_ack) && bytes_next < bytes_max){
                            /* PTO probe required. */
                            *bytes_next++ = picoquic_frame_type_ping;
                            length++;
                            is_pure_ack = 0;
                        }
                    } 

                    if (ret == 0 && length <= header_length) {
                        if (send_buffer_max > path_x->send_mtu
                            && path_x->cwin > path_x->bytes_in_transit 
                            && cnx->quic->cwin_max > path_x->bytes_in_transit
                            && pmtu_discovery_needed != picoquic_pmtu_discovery_not_needed) {
                            /* Since there is no data to send, this is an opportunity to send an MTU probe */
                            length = picoquic_prepare_mtu_probe(cnx, path_x, header_length, checksum_overhead, bytes, send_buffer_max);
                            packet->length = length;
                            packet->send_path = path_x;
                            packet->is_mtu_probe = 1;
                            path_x->mtu_probe_sent = 1;
                            is_pure_ack = 0;
                        }
                    }
                } /* end of CC */
            } /* End of pacing */
            else if (cnx->priority_limit_for_bypass > 0 && cnx->nb_paths == 1) {
                /* If congestion bypass is implemented, also consider pacing bypass */
                int no_data_to_send = 0;

                if ((bytes_next = picoquic_prepare_stream_and_datagrams(cnx, path_x, bytes_next, bytes_max,
                    (size_t)(bytes_next - bytes) <= packet->offset, cnx->priority_limit_for_bypass,
                    &more_data, &is_pure_ack, &no_data_to_send, &ret)) != NULL) {
                    length = bytes_next - bytes;
                }
            }
        } /* End of challenge verified */
    }

    if (length <= header_length) {
        length = 0;
    }

    if (cnx->cnx_state != picoquic_state_disconnected) {
        if (length > 0){
            path_x->is_pto_required &= is_pure_ack;
            pkt_ctx->ack_of_ack_requested |= !is_pure_ack;
            if (!pkt_ctx->ack_of_ack_requested && ack_sent) {
                /* If we have sent many ACKs, add a PING to get an ack of ack */
                /* The number 24 is chosen to not break any of the unit tests. If the number is
                 * too small, the PING mechanism can cause delayed end of the connection, or 
                 * early breakage */
                const uint64_t ack_repeat_interval = 24;
                bytes_next = bytes + length;
                if (bytes_next < bytes_max &&
                    pkt_ctx->highest_acknowledged + ack_repeat_interval < pkt_ctx->send_sequence &&
                    path_x == cnx->path[0] &&
                    pkt_ctx->highest_acknowledged_time + path_x->smoothed_rtt < current_time) {
                    /* Bundle a Ping with ACK, so as to get trigger an Acknowledgement */
                    *bytes_next++ = picoquic_frame_type_ping;
                    pkt_ctx->ack_of_ack_requested = 1;
                    is_pure_ack = 0;
                    length = bytes_next - bytes;
                }
            }

            if (is_pure_ack && cnx->is_multipath_enabled && 
                path_x->is_ack_lost && !path_x->is_ack_expected) {
                /* In some multipath scenarios, we may need to ping a path if we see 
                 * non-ackable packets being lost. */
                bytes_next = bytes + length;
                if (bytes_next < bytes_max) {
                    is_pure_ack = 0;
                    *bytes_next = picoquic_frame_type_ping;
                    length++;
                }
            }

            if (!is_pure_ack) {
                path_x->is_ack_expected = 1;
            }
        }

        if (is_pure_ack == 0)
        {
            cnx->latest_progress_time = current_time;
        }
        else if (cnx->keep_alive_interval != 0) {
            /* If necessary, encode and send the keep alive packet.
             * We only send keep alive packets when no other data is sent.
             */
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

        if (more_data) {
            *next_wake_time = current_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
            ret = 0;
        }
    }

    if (ret == 0 && length > header_length) {
        /* Ensure that all packets are properly padded before being sent. */
        if (is_challenge_padding_needed && length < PICOQUIC_ENFORCED_INITIAL_MTU) {
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
        *next_wake_time = current_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);

        if (ret == 0 && picoquic_cnx_is_still_logging(cnx)) {
            picoquic_log_cc_dump(cnx, current_time);
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let packet_type = PacketType::OneRttProtected;
        let pc = PacketContext::Application;
        let checksum_overhead = self.get_checksum_length(Epoch::OneRtt);
        let send_buffer_min_max = send_buffer_max.min(path_x.send_mtu);
        if send_buffer_min_max <= checksum_overhead {
            *send_length = 0;
            return 0;
        }

        if self.is_multipath_enabled {
            if path_x.pkt_ctx.send_sequence >= path_x.pkt_ctx.next_sequence_hole {
                let mut pkt_ctx = core::mem::replace(
                    &mut path_x.pkt_ctx,
                    PacketContextState {
                        send_sequence: 0,
                        next_sequence_hole: u64::MAX,
                        retransmit_sequence: 0,
                        highest_acknowledged: u64::MAX,
                        latest_time_acknowledged: Instant::from_ticks(0),
                        highest_acknowledged_time: Instant::from_ticks(0),
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
                self.insert_hole_in_send_sequence_if_needed(
                    path_x,
                    &mut pkt_ctx,
                    current_time,
                    next_wake_time,
                );
                path_x.pkt_ctx = pkt_ctx;
            }
        } else {
            let app = PacketContext::Application as usize;
            if self.pkt_ctx[app].send_sequence >= self.pkt_ctx[app].next_sequence_hole {
                let mut pkt_ctx = core::mem::replace(
                    &mut self.pkt_ctx[app],
                    PacketContextState {
                        send_sequence: 0,
                        next_sequence_hole: u64::MAX,
                        retransmit_sequence: 0,
                        highest_acknowledged: u64::MAX,
                        latest_time_acknowledged: Instant::from_ticks(0),
                        highest_acknowledged_time: Instant::from_ticks(0),
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
                self.insert_hole_in_send_sequence_if_needed(
                    path_x,
                    &mut pkt_ctx,
                    current_time,
                    next_wake_time,
                );
                self.pkt_ctx[app] = pkt_ctx;
            }
        }

        let is_nominal_ack_path = if self.is_multipath_enabled {
            path_x.is_nominal_ack_path || self.paths.len() == 1
        } else {
            path_x.unique_path_id == self.paths.first().map(|p| p.unique_path_id).unwrap_or(0)
        };
        let mut header_length = 0usize;
        let mut more_data = 0;
        let mut is_pure_ack = 1;
        let mut ret = 0;
        let mut ack_sent = false;
        let mut is_challenge_padding_needed = 0;
        let mut length = 0usize;
        let bytes_limit = send_buffer_min_max
            .saturating_sub(checksum_overhead)
            .min(packet.bytes.len());

        if self.misc_frames.is_empty() {
            let retransmit_len = self.retransmit_needed(
                pc,
                path_x,
                current_time,
                next_wake_time,
                packet,
                send_buffer_min_max,
                &mut header_length,
            );
            if retransmit_len > 0 {
                length = retransmit_len as usize;
                if length > header_length && length + 256 < bytes_limit {
                    let tail_len = {
                        let tail = &mut packet.bytes[length..bytes_limit];
                        if let Some(next) = format_ack_frame(
                            self,
                            tail,
                            &mut more_data,
                            current_time,
                            pc,
                            i32::from(!is_nominal_ack_path),
                        ) {
                            next.len()
                        } else {
                            ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                            tail.len()
                        }
                    };
                    length = bytes_limit.saturating_sub(tail_len);
                }
                is_pure_ack = 0;
                packet.send_time = current_time;
                packet.checksum_overhead = checksum_overhead;
            }
        }

        if length == 0 && self.connection_state != State::Disconnected {
            let pkt_send_sequence = if self.is_multipath_enabled {
                path_x.pkt_ctx.send_sequence
            } else {
                self.pkt_ctx[pc as usize].send_sequence
            };
            length = self.predict_packet_header_length_for_pc(packet_type, pc);
            header_length = length;
            packet.packet_type = packet_type;
            packet.offset = length;
            packet.sequence_number = pkt_send_sequence;
            packet.send_time = current_time;
            packet.send_path = Some(Self::path_token_for_path(path_x));
            packet.packet_context = pc;

            if length <= bytes_limit {
                let mut offset = length;
                let tail_len = {
                    let tail = &mut packet.bytes[offset..bytes_limit];
                    match self.prepare_path_challenge_frames(
                        path_x,
                        tail,
                        &mut more_data,
                        &mut is_pure_ack,
                        &mut is_challenge_padding_needed,
                        current_time,
                        next_wake_time,
                    ) {
                        Some(next) => next.len(),
                        None => {
                            ret = crate::errors::InternalError::FrameBufferTooSmall as i32;
                            tail.len()
                        }
                    }
                };
                offset = bytes_limit.saturating_sub(tail_len);

                if ret == 0 {
                    if path_x.is_multipath_probe_needed && ret == 0 {
                        packet.is_multipath_probe = true;
                        path_x.is_multipath_probe_needed = false;
                        is_pure_ack = 0;
                        if offset < bytes_limit {
                            packet.bytes[offset] = crate::frames::FrameType::Ping as u8;
                            offset += 1;
                        }
                        offset = pad_to_target_length(
                            &mut packet.bytes,
                            offset,
                            send_buffer_min_max.saturating_sub(checksum_overhead),
                        );
                    } else if ret == 0
                        && path_x
                            .tuples
                            .first()
                            .map(|tuple| tuple.challenge_verified)
                            .unwrap_or(false)
                        && path_x.pacing.is_authorized(
                            current_time,
                            next_wake_time,
                            self.packet_train_mode(),
                            None,
                        )
                    {
                        if self.is_ack_needed(
                            current_time,
                            next_wake_time,
                            pc,
                            i32::from(!is_nominal_ack_path),
                        ) {
                            let before = offset;
                            let tail_len = {
                                let tail = &mut packet.bytes[offset..bytes_limit];
                                match format_ack_frame(
                                    self,
                                    tail,
                                    &mut more_data,
                                    current_time,
                                    pc,
                                    i32::from(!is_nominal_ack_path),
                                ) {
                                    Some(next) => next.len(),
                                    None => {
                                        ret = crate::errors::InternalError::FrameBufferTooSmall
                                            as i32;
                                        tail.len()
                                    }
                                }
                            };
                            offset = bytes_limit.saturating_sub(tail_len);
                            ack_sent = offset > before;
                        }
                        if ret == 0 {
                            let tail_len = {
                                let tail = &mut packet.bytes[offset..bytes_limit];
                                match format_max_streams_frame_if_needed(
                                    self,
                                    tail,
                                    &mut more_data,
                                    &mut is_pure_ack,
                                ) {
                                    Some(next) => next.len(),
                                    None => {
                                        ret = crate::errors::InternalError::FrameBufferTooSmall
                                            as i32;
                                        tail.len()
                                    }
                                }
                            };
                            offset = bytes_limit.saturating_sub(tail_len);
                        }
                        if ret == 0 {
                            if self.quic_ref().map(|q| q.max_data_limit).unwrap_or(0) != 0 {
                                let limit = self.quic_ref().map(|q| q.max_data_limit).unwrap_or(0);
                                if self.data_received.saturating_add((3 * limit) / 4)
                                    > self.maxdata_local
                                {
                                    let inc = self
                                        .data_received
                                        .saturating_add(limit)
                                        .saturating_sub(self.maxdata_local);
                                    let tail_len = {
                                        let tail = &mut packet.bytes[offset..bytes_limit];
                                        format_max_data_frame(
                                            self,
                                            tail,
                                            &mut more_data,
                                            &mut is_pure_ack,
                                            inc,
                                        )
                                        .map(|next| next.len())
                                        .unwrap_or_else(|| tail.len())
                                    };
                                    offset = bytes_limit.saturating_sub(tail_len);
                                }
                            } else if 2u64.saturating_mul(self.offset_received) > self.maxdata_local
                            {
                                let inc = self.cc_increased_window(self.maxdata_local);
                                let tail_len = {
                                    let tail = &mut packet.bytes[offset..bytes_limit];
                                    format_max_data_frame(
                                        self,
                                        tail,
                                        &mut more_data,
                                        &mut is_pure_ack,
                                        inc,
                                    )
                                    .map(|next| next.len())
                                    .unwrap_or_else(|| tail.len())
                                };
                                offset = bytes_limit.saturating_sub(tail_len);
                            }
                        }
                        if ret == 0 && self.max_stream_data_needed {
                            let tail_len = {
                                let tail = &mut packet.bytes[offset..bytes_limit];
                                format_required_max_stream_data_frames(
                                    self,
                                    tail,
                                    &mut more_data,
                                    &mut is_pure_ack,
                                )
                                .map(|next| next.len())
                                .unwrap_or_else(|| tail.len())
                            };
                            offset = bytes_limit.saturating_sub(tail_len);
                        }
                        if ret == 0 && !self.misc_frames.is_empty() {
                            more_data = 1;
                        }
                        if ret == 0 {
                            let tail_len = {
                                let tail = &mut packet.bytes[offset..bytes_limit];
                                format_misc_frames_in_context(
                                    self,
                                    tail,
                                    &mut more_data,
                                    &mut is_pure_ack,
                                    pc,
                                )
                                .map(|next| next.len())
                                .unwrap_or_else(|| tail.len())
                            };
                            offset = bytes_limit.saturating_sub(tail_len);
                        }

                        if path_x.cwin <= path_x.bytes_in_transit
                            || self.quic_cwin_max() <= path_x.bytes_in_transit
                        {
                            self.cwin_blocked = true;
                            path_x.last_cwin_blocked_time = current_time;
                            if let Some(cc_alg) = self.congestion_alg {
                                let ack_state = PerAckState {
                                    pc: pc as i32,
                                    ..PerAckState::default()
                                };
                                cc_alg.algorithm.alg_notify(
                                    self,
                                    path_x,
                                    CongestionNotification::CwinBlocked,
                                    &ack_state,
                                    current_time,
                                );
                            }
                        } else if ret == 0 {
                            let pmtu_discovery_needed = self.is_mtu_probe_needed(path_x);
                            if self.is_tls_stream_ready() {
                                let tail_len = {
                                    let tail = &mut packet.bytes[offset..bytes_limit];
                                    format_crypto_hs_frame(
                                        &mut self.tls_stream[Epoch::OneRtt as usize],
                                        tail,
                                        &mut more_data,
                                        &mut is_pure_ack,
                                    )
                                    .map(|next| next.len())
                                    .unwrap_or_else(|| tail.len())
                                };
                                offset = bytes_limit.saturating_sub(tail_len);
                            }
                            if ret == 0
                                && self.is_address_discovery_provider
                                && !path_x.tuples.is_empty()
                            {
                                let mut tuple = path_x.tuples.remove(0);
                                let tail_len = {
                                    let tail = &mut packet.bytes[offset..bytes_limit];
                                    prepare_observed_address_frame(
                                        tail,
                                        path_x,
                                        &mut tuple,
                                        current_time,
                                        next_wake_time,
                                        &mut more_data,
                                        &mut is_pure_ack,
                                    )
                                    .map(|next| next.len())
                                    .unwrap_or_else(|| tail.len())
                                };
                                offset = bytes_limit.saturating_sub(tail_len);
                                path_x.tuples.insert(0, tuple);
                            }
                            let used_before_payload = offset;
                            if used_before_payload > header_length
                                || pmtu_discovery_needed != PmtuDiscoveryStatus::Required
                                || send_buffer_max <= path_x.send_mtu
                            {
                                if ret == 0 {
                                    let tail_len = {
                                        let tail = &mut packet.bytes[offset..bytes_limit];
                                        self.format_new_local_id_as_needed(
                                            tail,
                                            current_time,
                                            next_wake_time,
                                            &mut more_data,
                                            &mut is_pure_ack,
                                        )
                                        .map(|next| next.len())
                                        .unwrap_or_else(|| tail.len())
                                    };
                                    offset = bytes_limit.saturating_sub(tail_len);
                                }
                                if ret == 0
                                    && self.is_ack_frequency_updated
                                    && self.is_ack_frequency_negotiated
                                {
                                    let tail_len = {
                                        let tail = &mut packet.bytes[offset..bytes_limit];
                                        format_ack_frequency_frame(self, tail, &mut more_data)
                                            .map(|next| next.len())
                                            .unwrap_or_else(|| tail.len())
                                    };
                                    offset = bytes_limit.saturating_sub(tail_len);
                                }
                                let mut no_data_to_send = 1;
                                if ret == 0 {
                                    let is_first = offset <= packet.offset;
                                    let tail_len = {
                                        let tail = &mut packet.bytes[offset..bytes_limit];
                                        self.prepare_stream_and_datagrams(
                                            path_x,
                                            tail,
                                            is_first,
                                            u64::MAX,
                                            &mut more_data,
                                            &mut is_pure_ack,
                                            &mut no_data_to_send,
                                            &mut ret,
                                        )
                                        .map(|next| next.len())
                                        .unwrap_or_else(|| tail.len())
                                    };
                                    offset = bytes_limit.saturating_sub(tail_len);
                                }
                                if !self.client_mode && self.send_receive_bdp_frame {
                                    let tail_len = {
                                        let tail = &mut packet.bytes[offset..bytes_limit];
                                        format_bdp_frame(
                                            self,
                                            tail,
                                            path_x,
                                            &mut more_data,
                                            &mut is_pure_ack,
                                        )
                                        .map(|next| next.len())
                                        .unwrap_or_else(|| tail.len())
                                    };
                                    offset = bytes_limit.saturating_sub(tail_len);
                                }
                                if offset <= header_length || is_pure_ack != 0 {
                                    path_x.delivered_limited_index = path_x.delivered;
                                    let tail_len = {
                                        let tail = &mut packet.bytes[offset..bytes_limit];
                                        format_blocked_frames(
                                            self,
                                            tail,
                                            &mut more_data,
                                            &mut is_pure_ack,
                                        )
                                        .map(|next| next.len())
                                        .unwrap_or_else(|| tail.len())
                                    };
                                    offset = bytes_limit.saturating_sub(tail_len);
                                }
                                if no_data_to_send != 0 {
                                    path_x.last_sender_limited_time = current_time;
                                }
                            }
                            if ret == 0
                                && path_x.is_pto_required
                                && (offset <= header_length || is_pure_ack != 0)
                                && offset < bytes_limit
                            {
                                packet.bytes[offset] = crate::frames::FrameType::Ping as u8;
                                offset += 1;
                                is_pure_ack = 0;
                            }
                            if ret == 0
                                && offset <= header_length
                                && send_buffer_max > path_x.send_mtu
                                && path_x.cwin > path_x.bytes_in_transit
                                && self.quic_cwin_max() > path_x.bytes_in_transit
                                && pmtu_discovery_needed != PmtuDiscoveryStatus::NotNeeded
                            {
                                let probe_len = self.prepare_mtu_probe(
                                    path_x,
                                    header_length,
                                    checksum_overhead,
                                    &mut packet.bytes,
                                    send_buffer_max,
                                );
                                packet.length = probe_len;
                                packet.send_path = Some(Self::path_token_for_path(path_x));
                                packet.is_mtu_probe = true;
                                path_x.mtu_probe_sent = true;
                                is_pure_ack = 0;
                                offset = probe_len;
                            }
                        }
                    } else if self.priority_limit_for_bypass > 0 && self.paths.len() == 1 {
                        let mut no_data_to_send = 0;
                        let is_first = offset <= packet.offset;
                        let tail_len = {
                            let tail = &mut packet.bytes[offset..bytes_limit];
                            self.prepare_stream_and_datagrams(
                                path_x,
                                tail,
                                is_first,
                                self.priority_limit_for_bypass,
                                &mut more_data,
                                &mut is_pure_ack,
                                &mut no_data_to_send,
                                &mut ret,
                            )
                            .map(|next| next.len())
                            .unwrap_or_else(|| tail.len())
                        };
                        offset = bytes_limit.saturating_sub(tail_len);
                    }
                }
                length = offset;
            }
        }

        if length <= header_length {
            length = 0;
        }
        if self.connection_state != State::Disconnected {
            if length > 0 {
                path_x.is_pto_required &= is_pure_ack != 0;
                let pkt_ctx = if self.is_multipath_enabled {
                    &mut path_x.pkt_ctx
                } else {
                    &mut self.pkt_ctx[pc as usize]
                };
                pkt_ctx.ack_of_ack_requested |= is_pure_ack == 0;
                if !pkt_ctx.ack_of_ack_requested
                    && ack_sent
                    && pkt_ctx.highest_acknowledged.saturating_add(24) < pkt_ctx.send_sequence
                    && pkt_ctx
                        .highest_acknowledged_time
                        .ticks()
                        .saturating_add(path_x.smoothed_rtt.ticks())
                        < current_time.ticks()
                    && length < bytes_limit
                {
                    packet.bytes[length] = crate::frames::FrameType::Ping as u8;
                    length += 1;
                    pkt_ctx.ack_of_ack_requested = true;
                    is_pure_ack = 0;
                }
                if is_pure_ack != 0
                    && self.is_multipath_enabled
                    && path_x.is_ack_lost
                    && !path_x.is_ack_expected
                    && length < bytes_limit
                {
                    packet.bytes[length] = crate::frames::FrameType::Ping as u8;
                    length += 1;
                    is_pure_ack = 0;
                }
                if is_pure_ack == 0 {
                    path_x.is_ack_expected = true;
                }
            }
            if is_pure_ack == 0 {
                self.latest_progress_time = current_time;
            } else if self.keep_alive_interval.ticks() != 0 {
                let keep_alive_time = self
                    .latest_progress_time
                    .ticks()
                    .saturating_add(self.keep_alive_interval.ticks());
                if keep_alive_time <= current_time.ticks() && length == 0 {
                    length = self.predict_packet_header_length_for_pc(packet_type, pc);
                    header_length = length;
                    packet.packet_type = packet_type;
                    packet.packet_context = pc;
                    packet.offset = length;
                    packet.sequence_number = if self.is_multipath_enabled {
                        path_x.pkt_ctx.send_sequence
                    } else {
                        self.pkt_ctx[pc as usize].send_sequence
                    };
                    packet.send_path = Some(Self::path_token_for_path(path_x));
                    packet.send_time = current_time;
                    if length + 1 < packet.bytes.len() {
                        packet.bytes[length] = crate::frames::FrameType::Ping as u8;
                        packet.bytes[length + 1] = crate::frames::FrameType::Padding as u8;
                        length += 2;
                    }
                    self.latest_progress_time = current_time;
                } else if keep_alive_time < next_wake_time.ticks() {
                    *next_wake_time = Instant::from_ticks(keep_alive_time);
                }
            }
            self.note_more_data(more_data, next_wake_time, current_time);
        }

        if ret == 0 && length > header_length {
            if is_challenge_padding_needed != 0 && length < ENFORCED_INITIAL_MTU {
                length = pad_to_target_length(
                    &mut packet.bytes,
                    length,
                    send_buffer_min_max.saturating_sub(checksum_overhead),
                );
            } else {
                length = self.pad_to_policy(
                    &mut packet.bytes,
                    length,
                    send_buffer_min_max.saturating_sub(checksum_overhead) as u32,
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
            send_buffer_min_max,
            path_x,
            current_time,
        );
        if *send_length > 0 {
            self.set_sender_wake_now(next_wake_time, current_time);
            if ret == 0 && self.is_still_logging() {
                crate::logger::Log::cc_dump(self, current_time);
            }
        }
        ret
    }
```

## Pair `picoquic/sender.c:picoquic_check_cc_feedback_timer`
C: `picoquic/sender.c:3848-3879 picoquic_check_cc_feedback_timer`
Rust: `rs/fq/src/internal.rs:16676-16686 check_cc_feedback_timer`

### C body
```c
{
    int ret = 0;

    if (cnx->is_lost_feedback_notification_required && cnx->congestion_alg != NULL) {
        for (int i = 0; i < cnx->nb_paths; i++) {
            picoquic_path_t* path_x = cnx->path[i];
            if (!path_x->is_lost_feedback_notified){
                picoquic_packet_context_t* pkt_ctx = (cnx->is_multipath_enabled)?
                    &path_x->pkt_ctx:&cnx->pkt_ctx[picoquic_packet_context_application];
                if (pkt_ctx->pending_first != NULL) {
                    uint64_t delta_sent = (pkt_ctx->pending_first->send_time <= path_x->last_time_acked_data_frame_sent) ? 0 :
                        (pkt_ctx->pending_first->send_time - path_x->last_time_acked_data_frame_sent);
                    uint64_t lost_feedback_time = pkt_ctx->highest_acknowledged_time + delta_sent + 2 * cnx->ack_frequency_delay_local;
                            
                    if (lost_feedback_time <= current_time) {
                        path_x->is_lost_feedback_notified = 1;
                        cnx->congestion_alg->alg_notify(cnx, path_x,
                            picoquic_congestion_notification_lost_feedback,
                            NULL, current_time);
                    }
                    else if (lost_feedback_time < *next_wake_time) {
                        *next_wake_time = lost_feedback_time;
                        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
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
        let Some(cc_alg) = self.congestion_alg else {
            return 0;
        };
```

## Pair `picoquic/sender.c:picoquic_handle_send_paths`
C: `picoquic/sender.c:3934-3957 picoquic_handle_send_paths`
Rust: `rs/fq/src/internal.rs:4655-4690 picoquic_handle_send_paths`

### C body
```c
{
    /* Remove delete paths */
    if (cnx->path_demotion_needed) {
        picoquic_delete_abandoned_paths(cnx, current_time, next_wake_time);
    }
    if (cnx->tuple_demotion_needed) {
        picoquic_delete_demoted_tuples(cnx, current_time, next_wake_time);
    }
    /* Select the next path, and the corresponding addresses */
    picoquic_select_next_path_tuple(cnx, current_time, next_wake_time, path_x, tuple);
    picoquic_set_path_addresses_from_tuple(*tuple, p_addr_to, p_addr_from, if_index);
    /* Send the available packets */
    if (send_msg_size != NULL) {
        *send_msg_size = (*path_x)->send_mtu;
    }
    if (send_buffer_max > (*path_x)->send_mtu) {
        cnx->is_sending_large_buffer = 1;
    }
}
```

### Rust body
```rust
    ) -> Option<(PathToken, usize, SocketAddr, SocketAddr, i32)> {
        if self.path_demotion_needed {
            self.delete_abandoned_paths(current_time, next_wake_time);
        }
        if self.tuple_demotion_needed {
            self.delete_demoted_tuples(current_time, next_wake_time);
        }

        let (path_token, tuple_index) =
            self.select_next_path_tuple(current_time, next_wake_time)?;
        let path_index = path_token.slot_idx();
        let path = self.paths.get(path_index)?;
        let tuple = path.tuples.get(tuple_index)?;
        let mut addr_to = crate::unspecified_socket_addr();
        let mut addr_from = crate::unspecified_socket_addr();
        let mut if_index = -1;
        picoquic_set_path_addresses_from_tuple(
            tuple,
            Some(&mut addr_to),
            Some(&mut addr_from),
            Some(&mut if_index),
        );
        if let Some(send_msg_size) = send_msg_size {
            *send_msg_size = path.send_mtu;
        }
        if send_buffer_max > path.send_mtu {
            self.is_sending_large_buffer = true;
        }
        Some((path_token, tuple_index, addr_to, addr_from, if_index))
    }
```

## Pair `picoquic/sender.c:picoquic_close`
C: `picoquic/sender.c:4196-4199 picoquic_close`
Rust: `rs/fq/src/lib.rs:2341-2343 close`

### C body
```c
{
    return picoquic_close_ex(cnx, application_reason_code, NULL);
}
```

### Rust body
```rust
    pub fn close(&mut self, application_reason_code: u64) -> Result<(), Error> {
        self.picoquic_close_ex(application_reason_code, None)
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_next_packet`
C: `picoquic/sender.c:4326-4333 picoquic_prepare_next_packet`
Rust: `rs/fq/src/lib.rs:3511-3519 prepare_next_packet`

### C body
```c
{
    return picoquic_prepare_next_packet_ex(quic, current_time, send_buffer, send_buffer_max, send_length,
        p_addr_to, p_addr_from, if_index, log_cid, p_last_cnx, NULL);
}
```

### Rust body
```rust
    ) -> Result<PreparedPacket<'_>, Error> {
        let mut prepared = self.prepare_next_packet_ex(current_time, send_buffer)?;
        prepared.send_msg_size = None;
        Ok(prepared)
    }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_next_admission`
C: `picoquic/sim_link.c:105-116 picoquictest_sim_link_next_admission`
Rust: `rs/fq/src/tests/util.rs:405-421 next_admission`

### C body
```c
{
    if (link->aqm_state != NULL &&
        link->aqm_state->has_pending != NULL &&
        link->aqm_state->has_pending(link->aqm_state)) {
        uint64_t candidate_time = (link->first_packet == NULL) ? current_time : link->queue_time;
        if (candidate_time < next_time) {
            next_time = candidate_time;
        }
    }
    return next_time;
}
```

### Rust body
```rust
    pub fn next_admission(&mut self, current_time: Instant, next_time: Instant) -> u64 {
        let mut nt = next_time.ticks();
        if let Some(mut aqm) = self.aqm_state.take() {
            if aqm.has_pending() {
                let candidate = if self.packets.is_empty() {
                    current_time.ticks()
                } else {
                    self.queue_time.ticks()
                };
                if candidate < nt {
                    nt = candidate;
                }
            }
            self.aqm_state = Some(aqm);
        }
        nt
    }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_simloss`
C: `picoquic/sim_link.c:160-184 picoquictest_sim_link_simloss`
Rust: `rs/fq/src/tests/harness.rs:32-54 simloss`

### C body
```c
{
    int loss = 0;

    if (link->nb_loss_in_burst > 0) {
        if (link->packets_sent > link->packets_sent_next_burst)
        {
            uint64_t picosec_wait = link->nb_loss_in_burst * link->picosec_per_byte * 1536;
            link->packets_sent_next_burst = link->packets_sent + link->packets_between_losses;
            link->nb_losses_this_burst = link->nb_loss_in_burst - 1;
            link->end_of_burst_time = current_time + (picosec_wait / 1000000);
            loss = 1;
        }
        else if (link->nb_losses_this_burst > 0) {
            if (current_time > link->end_of_burst_time) {
                link->nb_losses_this_burst = 0;
            }
            else {
                loss = 1;
                link->nb_losses_this_burst -= 1;
            }
        }
    }
    return loss;
}
```

### Rust body
```rust
fn simloss(link: &mut TestSimLink, current_time: Instant) -> bool {
    if link.nb_loss_in_burst == 0 {
        return false;
    }
    let ct = current_time.ticks();
    if link.packets_sent > link.packets_sent_next_burst {
        let picosec_wait = link.nb_loss_in_burst * link.picosec_per_byte * 1536;
        link.packets_sent_next_burst = link.packets_sent + link.packets_between_losses;
        link.nb_losses_this_burst = link.nb_loss_in_burst - 1;
        link.end_of_burst_time = Instant::from_ticks(ct + picosec_wait / 1_000_000);
        true
    } else if link.nb_losses_this_burst > 0 {
        if ct > link.end_of_burst_time.ticks() {
            link.nb_losses_this_burst = 0;
            false
        } else {
            link.nb_losses_this_burst -= 1;
            true
        }
    } else {
        false
    }
}
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_transmit_time`
C: `picoquic/sim_link.c:296-299 picoquictest_sim_link_transmit_time`
Rust: `rs/fq/src/tests/util.rs:497-507 transmit_time`

### C body
```c
{
    return ((link->picosec_per_byte * ((uint64_t)packet->length)) >> 20);
}
```

### Rust body
```rust
    pub fn queue_delay(&mut self, current_time: Instant) -> u64 {
        let qt = self.queue_time.ticks();
        let ct = current_time.ticks();
        qt.saturating_sub(ct)
    }
```

## Pair `picoquic/sim_link.c:sim_link_one_test`
C: `picoquic/sim_link.c:375-429 sim_link_one_test`
Rust: `rs/fq/src/tests/harness.rs:105-144 sim_link_one_test`

### C body
```c
{
    int ret = 0;
    uint64_t current_time = 0;
    uint64_t departure_time = 0;
    picoquictest_sim_link_t* link = picoquictest_sim_link_create(0.01, 10000, loss_mask, queue_delay_max, current_time);
    uint64_t dequeued = 0;
    uint64_t queued = 0;
    const uint64_t nb_packets = 16;

    if (link == NULL) {
        ret = -1;
    }
    else {

        while (ret == 0) {
            if (queued >= nb_packets) {
                departure_time = UINT64_MAX;
            }

            current_time = picoquictest_sim_link_next_arrival(link, departure_time);

            picoquictest_sim_packet_t* packet = picoquictest_sim_link_dequeue(link, current_time);

            if (packet != NULL) {
                dequeued++;
                free(packet);
            }
            else if (queued < nb_packets) {
                packet = picoquictest_sim_link_create_packet();

                if (packet == NULL) {
                    ret = -1;
                }
                else {
                    packet->length = sizeof(packet->bytes);
                    picoquictest_sim_link_submit(link, packet, departure_time);
                    departure_time += 250;
                    queued++;
                }
            }
            else {
                break;
            }
        }

        if ((dequeued + nb_losses) != nb_packets) {
            ret = -1;
        }
        
        picoquictest_sim_link_delete(link);
    }

    return ret;
}
```

### Rust body
```rust
fn sim_link_one_test(loss_mask: Option<u64>, queue_delay_max: u64, nb_losses: u64) {
    let mut departure_time = Instant::from_ticks(0);
    let mut link = TestSimLink::create(
        0.01,
        10_000,
        loss_mask,
        queue_delay_max,
        Instant::from_ticks(0),
    )
    .expect("sim link allocation");
    let mut dequeued = 0u64;
    let mut queued = 0u64;
    const NB_PACKETS: u64 = 16;

    loop {
        if queued >= NB_PACKETS {
            departure_time = Instant::from_ticks(u64::MAX);
        }

        let current_time = Instant::from_ticks(link.next_arrival(departure_time));

        if let Some(_packet) = link.dequeue(current_time) {
            dequeued += 1;
        } else if queued < NB_PACKETS {
            let mut packet = TestSimPacket::create().expect("sim packet allocation");
            packet.length = MAX_PACKET_SIZE;
            link.submit(packet, departure_time);
            departure_time = Instant::from_ticks(departure_time.ticks() + 250);
            queued += 1;
        } else {
            break;
        }
    }

    assert_eq!(
        dequeued + nb_losses,
        NB_PACKETS,
        "unexpected sim-link delivery count",
    );
}
```

## Pair `picoquic/sockloop.c:picoquic_packet_loop_close_socket`
C: `picoquic/sockloop.c:344-361 picoquic_packet_loop_close_socket`
Rust: `rs/fq/src/packet_loop.rs:1640-1643 close`

### C body
```c
{
    if (s_ctx->fd != INVALID_SOCKET) {
        SOCKET_CLOSE(s_ctx->fd);
        s_ctx->fd = INVALID_SOCKET;
    }
#ifdef _WINDOWS
    if (s_ctx->overlap.hEvent != WSA_INVALID_EVENT) {
        WSACloseEvent(s_ctx->overlap.hEvent);
        s_ctx->overlap.hEvent = WSA_INVALID_EVENT;
    }

    if (s_ctx->recv_buffer != NULL) {
        free(s_ctx->recv_buffer);
        s_ctx->recv_buffer = NULL;
    }
#endif
}
```

### Rust body
```rust
    pub fn close(&mut self) {
        self.fd = None;
        self.is_started = false;
    }
```

## Pair `picoquic/sockloop.c:picoquic_packet_loop_poll`
C: `picoquic/sockloop.c:907-992 picoquic_packet_loop_poll`
Rust: `rs/fq/src/packet_loop.rs:804-897 packet_loop_poll`

### C body
```c
{
    /* Picoquic expresses times in microseconds, but the timeout 
     * parameter of poll() is in milliseconds. The code below converts
     * the microsecond delay to the nearest millisecond, which is a
     * compromise. Return a smaller value than the timer incurs the
     * risk of waking up too soon, e.g., waiting "0" ms instead of
     * 499us, probably leading to an extra call to "poll". Returning a
     * larger value carries the opposite risk, waiting to long and thus
     * slowing down operations. We may need to change this code later
     * based on experience */
    int delta_t_ms = (int)((delta_t + 500) / 1000);
    int bytes_recv = 0;
    int i_poll = (thread_ctx->wake_up_defined) ? 1 : 0;
    int ret_poll = poll(poll_list, nb_sockets + i_poll, delta_t_ms);

    if (received_ecn != NULL) {
        *received_ecn = 0;
    }
    *is_wake_up_event = 0;

    if (ret_poll < 0) {
        bytes_recv = -1;
        DBG_PRINTF("Error: poll returns %d\n", ret_poll);
    }
    else if (ret_poll > 0) {
        /* Check if the 'wake up' pipe is full. If it is, read the data on it,
         * set the is_wake_up_event flag, and ignore the other file descriptors. */
        if (thread_ctx->wake_up_defined && poll_list[0].revents != 0) {
            /* Something was written on the "wakeup" pipe. Read it. */
            uint8_t eventbuf[8];
            int pipe_recv;
            DBG_PRINTF("Waking up -- defined: %d, nb_sockets: %d",
                (thread_ctx->wake_up_defined) ? 1 : 0, nb_sockets);
            if ((pipe_recv = read(thread_ctx->wake_up_pipe_fd[0], eventbuf, sizeof(eventbuf))) <= 0) {
                bytes_recv = -1;
                DBG_PRINTF("Error: read pipe returns %d\n", (pipe_recv == 0) ? EPIPE : errno);
            }
            else {
                DBG_PRINTF("Waking up -- received: %d", pipe_recv);
                *is_wake_up_event = 1;
            }
        }
        else
        {
            for (int i = 0; i < nb_sockets; i++) {
                if (poll_list[i+i_poll].revents != 0) {
                    *socket_rank = i;
                    bytes_recv = picoquic_recvmsg(s_ctx[i].fd, addr_from,
                        addr_dest, dest_if, received_ecn,
                        buffer, buffer_max);

                    if (bytes_recv <= 0) {
                        DBG_PRINTF("Could not receive packet on UDP socket[%d]= %d!\n",
                            i, (int)s_ctx[i].fd);
                        break;
                    }
                    else {
                        /* Document incoming port */
                        if (addr_dest->ss_family == AF_INET6) {
                            ((struct sockaddr_in6*)addr_dest)->sin6_port = s_ctx[i].n_port;
                        }
                        else if (addr_dest->ss_family == AF_INET) {
                            ((struct sockaddr_in*)addr_dest)->sin_port = s_ctx[i].n_port;
                        }
                        break;
                    }
                }
            }
        }
    }

    return bytes_recv;
}
```

### Rust body
```rust
) -> Result<Option<PacketLoopPollResult>, Error> {
    let i_poll = usize::from(thread_ctx.wake_up_defined);
    let nfds = nb_sockets.saturating_add(i_poll);
    if poll_list.len() < nfds || s_ctx.len() < nb_sockets {
        return Err(Error::BufferTooSmall);
    }

    let delta_t_ms = ((delta_t + 500) / 1000).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    // SAFETY: `poll_list.as_mut_ptr()` points to `nfds` initialized pollfd
    // entries for the duration of the syscall.
    let ret_poll = unsafe { libc::poll(poll_list.as_mut_ptr(), nfds as libc::nfds_t, delta_t_ms) };

    *is_wake_up_event = false;
    for entry in poll_list.iter_mut().take(nfds) {
        if entry.revents == libc::POLLNVAL {
            return Err(Error::Generic);
        }
    }

    if ret_poll < 0 {
        return Err(Error::Generic);
    }
    if ret_poll == 0 {
        return Ok(None);
    }

    if thread_ctx.wake_up_defined && poll_list[0].revents != 0 {
        let mut drained = false;
        if thread_ctx.wake_up_pipe_fd[0] >= 0 {
            let mut eventbuf = [0u8; 8];
            // SAFETY: `eventbuf` is a valid writable buffer and the fd is the
            // read end supplied by the caller's wake-up pipe.
            let n = unsafe {
                libc::read(
                    thread_ctx.wake_up_pipe_fd[0],
                    eventbuf.as_mut_ptr().cast(),
                    eventbuf.len(),
                )
            };
            if n <= 0 {
                return Err(Error::Generic);
            }
            drained = true;
        } else if let Some(receiver) = thread_ctx.wake_up_receiver.as_ref() {
            while receiver.try_recv().is_ok() {
                drained = true;
            }
        }
        if !drained {
            return Err(Error::Generic);
        }
        *is_wake_up_event = true;
        return Ok(None);
    }

    for i in 0..nb_sockets {
        if poll_list[i + i_poll].revents == 0 {
            continue;
        }
        let Some(fd) = s_ctx[i].fd.as_mut() else {
            continue;
        };
        let mut info = fd.recv(buffer)?;
        if info.bytes_recv == 0 {
            return Ok(None);
        }
        if let Some(addr_dest) = info.addr_dest.as_mut() {
            set_socket_addr_port(addr_dest, u16::from_be(s_ctx[i].n_port));
        }
        s_ctx[i].addr_from = info.addr_from;
        s_ctx[i].addr_dest = info.addr_dest;
        s_ctx[i].dest_if = info.dest_if;
        s_ctx[i].received_ecn = info.received_ecn;
        s_ctx[i].bytes_recv = info.bytes_recv;
        return Ok(Some(PacketLoopPollResult {
            bytes_recv: info.bytes_recv,
            addr_from: info.addr_from,
            addr_dest: info.addr_dest,
            dest_if: info.dest_if,
            received_ecn: info.received_ecn,
            socket_rank: i,
        }));
    }

    Ok(None)
}
```

## Pair `picoquic/sockloop.c:picoquic_packet_loop`
C: `picoquic/sockloop.c:1618-1636 picoquic_packet_loop`
Rust: `rs/fq/src/packet_loop.rs:1289-1307 run`

### C body
```c
{
    picoquic_packet_loop_param_t param = { 0 };

    param.local_port = (uint16_t)local_port;
    param.local_af = local_af;
    param.dest_if = dest_if;
    param.socket_buffer_size = socket_buffer_size;
    param.do_not_use_gso = do_not_use_gso;

    return picoquic_packet_loop_v2(quic, &param, loop_callback, loop_callback_ctx);
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let mut param = LoopParam {
            local_port: local_port as u16,
            local_af,
            dest_if,
            socket_buffer_size,
            do_not_use_gso,
            ..LoopParam::default()
        };
        self.run_v2(&mut param, loop_callback)
    }
```

## Pair `picoquic/sockloop.c:picoquic_internal_thread_setname`
C: `picoquic/sockloop.c:1733-1761 picoquic_internal_thread_setname`
Rust: `rs/fq/src/packet_loop.rs:1485-1485 internal_thread_setname`

### C body
```c
{
#ifdef _WINDOWS
    wchar_t wname[257];
    wname[0] = 0;

    if (swprintf(wname, 256, L"%S", thread_name) < 0) {
        DBG_PRINTF("Cannot convert thread name <%s> to wchar[256], err: 0x%x",
            thread_name, GetLastError());
    }
    else {
        HRESULT r = SetThreadDescription(GetCurrentThread(), wname);
        if (r != 0) {
            DBG_PRINTF("Set thread name <%S> returns: 0x%x", wname, r);
        }
    }
#else
#if defined(__APPLE__)
    pthread_setname_np(thread_name);
#elif defined(__FreeBSD__)
    pthread_setname_np(pthread_self(), thread_name);
#else
    int r=prctl(PR_SET_NAME, thread_name, 0, 0, 0);
    if (r != 0) {
        DBG_PRINTF("Set thread name <%s> returns: 0x%x", thread_name, r);
    }
#endif
#endif
}
```

### Rust body
```rust
pub fn internal_thread_setname(_thread_name: &str) {}
```

## Pair `picoquic/sockloop.c:picoquic_wake_up_network_thread`
C: `picoquic/sockloop.c:1820-1849 picoquic_wake_up_network_thread`
Rust: `rs/fq/src/packet_loop.rs:1423-1431 wake_up`

### C body
```c
{
    int ret = 0;

    if (thread_ctx->wake_up_defined) {
#ifdef _WINDOWS
        if (SetEvent(thread_ctx->wake_up_event) == 0) {
            DWORD err = WSAGetLastError();
            DBG_PRINTF("Set network event fails, error 0x%x", err);
            ret = (int)err;
        }
#else
        /* TODO: write to network pipe */
        ssize_t written = 0;
        if ((written = write(thread_ctx->wake_up_pipe_fd[1], &ret, 1)) != 1) {
            if (written == 0) {
                ret = EPIPE;
            }
            else {
                ret = errno;
            }
        }
#endif
    }
    else {
        DBG_PRINTF("%s", "Wake up event not defined.");
        ret = -1;
    }
    return ret;
}
```

### Rust body
```rust
    pub fn wake_up(&mut self) -> Result<(), OsError> {
        if !self.wake_up_defined {
            return Err(OsError(-1));
        }
        let Some(sender) = self.wake_up_sender.as_ref() else {
            return Err(OsError(-1));
        };
        sender.send(()).map_err(|_| OsError(32))
    }
```

## Pair `picoquic/spinbit.c:picoquic_spinbit_basic_incoming`
C: `picoquic/spinbit.c:29-35 picoquic_spinbit_basic_incoming`
Rust: `rs/fq/src/spinbit.rs:25-37 incoming`

### C body
```c
{
    path_x->current_spin = ph->spin ^ cnx->client_mode;
}
```

### Rust body
```rust
    fn outgoing(&self, connection: &mut Connection) -> u8 {
        let spin = connection
            .paths
            .first()
            .map(|p| p.current_spin)
            .unwrap_or(false);
        (spin as u8) << 5
    }
```

## Pair `picoquic/spinbit.c:picoquic_spinbit_random_incoming`
C: `picoquic/spinbit.c:65-70 picoquic_spinbit_random_incoming`
Rust: `rs/fq/src/spinbit.rs:68-75 incoming`

### C body
```c
{
    UNREFERENCED_PARAMETER(cnx);
    UNREFERENCED_PARAMETER(path_x);
    UNREFERENCED_PARAMETER(ph);
}
```

### Rust body
```rust
    fn outgoing(&self, _connection: &mut Connection) -> u8 {
        // C: `(uint8_t)(picoquic_public_random_64() & 0x20)` — bit 5 is the
        // spin-bit position in the QUIC short header first byte.
        crate::public_random_64() as u8 & 0x20
    }
```

## Pair `picoquic/ticket_store.c:picoquic_deserialize_ticket`
C: `picoquic/ticket_store.c:160-257 picoquic_deserialize_ticket`
Rust: `rs/fq/src/internal.rs:1393-1433 deserialize_ticket`

### C body
```c
{
    int ret = 0;
    uint64_t time_valid_until = 0;
    size_t required_length = 8 + 2 + 2 + 4 + 1 + 1 + PICOQUIC_NB_TP_0RTT * 8 + 2;
    size_t byte_index = 0;
    size_t sni_index = 0;
    size_t alpn_index = 0;
    size_t ip_addr_index = 0;
    size_t ip_addr_client_index = 0;
    size_t ticket_index = 0;
    uint16_t sni_length = 0;
    uint16_t alpn_length = 0;
    uint32_t version = 0;
    uint16_t ticket_length = 0;
    uint8_t ip_addr_length = 0;
    uint8_t ip_addr_client_length = 0;
    uint64_t tp_0rtt[PICOQUIC_NB_TP_0RTT] = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 };

    *consumed = 0;
    *ticket = NULL;

    if (required_length < bytes_max) {
        time_valid_until = PICOPARSE_64(bytes);
        byte_index = 8;
        sni_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        sni_index = byte_index;
        required_length += sni_length;
        byte_index += sni_length;
    }

    if (required_length < bytes_max) {
        alpn_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        alpn_index = byte_index;
        required_length += alpn_length;
        byte_index += alpn_length;
    }

    if (required_length < bytes_max) {
        version = PICOPARSE_32(bytes + byte_index);
        byte_index += 4;
    }

    if (required_length < bytes_max) {
        ip_addr_length = bytes[byte_index++];
        ip_addr_index = byte_index;
        required_length += ip_addr_length;
        byte_index += ip_addr_length;
    }

    if (required_length < bytes_max) {
        ip_addr_client_length = bytes[byte_index++];
        ip_addr_client_index = byte_index;
        required_length += ip_addr_client_length;
        byte_index += ip_addr_client_length;
    }

    if (required_length < bytes_max) {
        for (int i = 0; i < PICOQUIC_NB_TP_0RTT; i++) {
            tp_0rtt[i] = PICOPARSE_64(bytes + byte_index);
            byte_index += 8;
        }
    }

    if (required_length < bytes_max) {
        ticket_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        ticket_index = byte_index;
        required_length += ticket_length;
    }

    if (required_length > bytes_max) {
        *ticket = NULL;
        ret = PICOQUIC_ERROR_INVALID_TICKET;
    } else {
        *ticket = picoquic_format_ticket(time_valid_until,
            (const char *)(bytes + sni_index), sni_length,
            (const char *)(bytes + alpn_index), alpn_length,
            version,
            (const uint8_t*)(bytes + ip_addr_index), ip_addr_length,
            (const uint8_t*)(bytes + ip_addr_client_index), ip_addr_client_length,
            bytes + ticket_index, ticket_length,
            NULL);
        if (*ticket == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
        else {
            for (int i=0; i< PICOQUIC_NB_TP_0RTT; i++) {
                (*ticket)->tp_0rtt[i] = tp_0rtt[i];
            }
            *consumed = required_length;
        }
    }

    return ret;
}
```

### Rust body
```rust
fn deserialize_ticket(bytes: &[u8]) -> Result<StoredTicket, crate::Error> {
    let mut off = 0;
    let time_valid_until = Instant::from_ticks(parse_64(take_slice(bytes, &mut off, 8)?));

    let sni_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let sni = optional_string_from_bytes(take_slice(bytes, &mut off, sni_len)?)?;

    let alpn_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let alpn = optional_string_from_bytes(take_slice(bytes, &mut off, alpn_len)?)?;

    let version = parse_32(take_slice(bytes, &mut off, 4)?);

    let ip_len = take_slice(bytes, &mut off, 1)?[0] as usize;
    let ip_addr = stored_ip_from_bytes(take_slice(bytes, &mut off, ip_len)?)?;

    let ip_client_len = take_slice(bytes, &mut off, 1)?[0] as usize;
    let ip_addr_client = stored_ip_from_bytes(take_slice(bytes, &mut off, ip_client_len)?)?;

    let mut tp_0rtt = [0u64; NB_TP_0RTT];
    for value in tp_0rtt.iter_mut() {
        *value = parse_64(take_slice(bytes, &mut off, 8)?);
    }

    let ticket_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let ticket = take_slice(bytes, &mut off, ticket_len)?.to_vec();
    if off != bytes.len() {
        return Err(crate::Error::InvalidFile);
    }

    Ok(StoredTicket {
        sni,
        alpn,
        ip_addr,
        ip_addr_client,
        tp_0rtt,
        ticket,
        time_valid_until,
        version,
        was_used: false,
    })
}
```
