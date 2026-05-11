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

## Pair `picoquic/sender.c:picoquic_packet_type_from_epoch`
C: `picoquic/sender.c:1745-1769 picoquic_packet_type_from_epoch`
Rust: `rs/fq/src/internal.rs:498-506 packet_type_from_epoch`

### C body
```c
{
    picoquic_packet_type_enum ptype;

    switch (epoch) {
    case 0:
        ptype = picoquic_packet_initial;
        break;
    case 1:
        ptype = picoquic_packet_0rtt_protected;
        break;
    case 2:
        ptype = picoquic_packet_handshake;
        break;
    case 3:
        ptype = picoquic_packet_1rtt_protected;
        break;
    default:
        ptype = picoquic_packet_error;
        break;
    }

    return ptype;
}
```

### Rust body
```rust
pub fn packet_type_from_epoch(epoch: i32) -> PacketType {
    match epoch {
        0 => PacketType::Initial,
        1 => PacketType::ZeroRttProtected,
        2 => PacketType::Handshake,
        3 => PacketType::OneRttProtected,
        _ => PacketType::Error,
    }
}
```

## Pair `picoquic/sender.c:picoquic_prepare_packet_client_init`
C: `picoquic/sender.c:1932-2213 picoquic_prepare_packet_client_init`
Rust: `rs/fq/src/internal.rs:18130-18258 prepare_packet_client_init`

### C body
```c
{
    int ret = 0;
    int tls_ready = 0;
    size_t checksum_overhead = 16;
    int is_cleartext_mode = 1;
    int retransmit_possible = 0;
    size_t header_length = 0;
    uint8_t* bytes = packet->bytes;
    uint8_t* bytes_max;
    uint8_t* bytes_next;
    size_t length = 0;
    int is_pure_ack = 1;
    int more_data = 0;
    int epoch = picoquic_epoch_initial;
    picoquic_packet_type_enum packet_type = picoquic_packet_initial;
    picoquic_packet_context_enum pc = picoquic_packet_context_initial;

    cnx->initial_validated = 1; /* always validated on client */

    if (cnx->tls_stream[picoquic_epoch_initial].send_queue == NULL) {
        if (cnx->crypto_context[picoquic_epoch_0rtt].aead_encrypt != NULL &&
            cnx->tls_stream[picoquic_epoch_0rtt].send_queue != NULL) {
            epoch = picoquic_epoch_0rtt;
            pc = picoquic_packet_context_application;
            packet_type = picoquic_packet_0rtt_protected;
        } else if (cnx->crypto_context[picoquic_epoch_handshake].aead_encrypt != NULL && 
            cnx->tls_stream[picoquic_epoch_0rtt].send_queue == NULL) {
            epoch = picoquic_epoch_handshake;
            pc = picoquic_packet_context_handshake;
            packet_type = picoquic_packet_handshake;
        } 
    }

    send_buffer_max = (send_buffer_max > path_x->send_mtu) ? path_x->send_mtu : send_buffer_max;

    /* Prepare header parameters -- depend on connection state */
    switch (cnx->cnx_state) {
    case picoquic_state_client_init:
        if (cnx->retry_token_length == 0 && cnx->sni != NULL) {
            (void)picoquic_get_token(cnx->quic, cnx->sni, (uint16_t)strlen(cnx->sni),
                NULL, 0, &cnx->retry_token, &cnx->retry_token_length, 1);
        }
        break;
    case picoquic_state_client_init_sent:
    case picoquic_state_client_init_resent:
        retransmit_possible = 1;
        break;
    case picoquic_state_client_renegotiate:
        packet_type = picoquic_packet_initial;
        break;
    case picoquic_state_client_handshake_start:
        retransmit_possible = 1;
        break;
    case picoquic_state_client_almost_ready:
        break;
    default:
        ret = -1;
        break;
    }

    /* If context is handshake, verify first that there is no need for retransmit or ack
     * on initial context */
    int force_handshake_ping = 0;

    if (ret == 0) {
        if (epoch > picoquic_epoch_initial) {
            if (cnx->crypto_context[picoquic_epoch_handshake].aead_encrypt != NULL) {
                if (cnx->ack_ctx[picoquic_packet_context_initial].act[0].ack_needed) {
                    /* Apply some ack delay, because handshake from server arrive in trains */
                    uint64_t ack_delay = cnx->path[0]->smoothed_rtt / 8;
                    uint64_t ack_time;
                    if (ack_delay > PICOQUIC_ACK_DELAY_MAX) {
                        ack_delay = PICOQUIC_ACK_DELAY_MAX;
                    }
                    ack_time = cnx->ack_ctx[picoquic_packet_context_initial].act[0].time_oldest_unack_packet_received + ack_delay;
                    if (ack_time <= current_time) {
                        force_handshake_ping = 1;
                    }
                    else if (ack_time < *next_wake_time) {
                        *next_wake_time = ack_time;
                        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                    }
                }
                else if (cnx->pkt_ctx[pc].pending_last != NULL) {
                    /* There is a risk of deadlock if the server is doing DDOS mitigation
                     * and does not receive the Handshake sent by the client. If more than RTT has elapsed since
                     * the last handshake packet was sent, force another one to be sent. */
                    uint64_t rto = picoquic_current_retransmit_timer(cnx, cnx->path[0]);
                    uint64_t repeat_time = cnx->pkt_ctx[pc].pending_last->send_time + rto;

                    if (repeat_time <= current_time) {
                        force_handshake_ping = 1;
                        cnx->path[0]->nb_retransmit++;
                        cnx->path[0]->last_loss_event_detected = current_time;
                    }
                    else if (repeat_time < *next_wake_time) {
                        *next_wake_time = repeat_time;
                        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
                    }
                }
            }
            else {
                length = picoquic_prepare_packet_old_context(cnx, picoquic_packet_context_initial,
                    path_x, packet, send_buffer_max, current_time, next_wake_time, &header_length);
                *is_initial_sent |= (length > 0);
            }
        }
        else {
            /* There is a risk of deadlock if the server is doing DDOS mitigation
             * and does not repeat an initial or handshake packet that was lost. If more than RTT has elapsed since
             * the last initial packet was sent, force another one to be sent. */
            uint64_t rto = picoquic_current_retransmit_timer(cnx, cnx->path[0]);
            uint64_t repeat_time = cnx->path[0]->latest_sent_time + rto;
            if (repeat_time <= current_time) {
                force_handshake_ping = 1;
                *is_initial_sent = 1;
            } else if (*next_wake_time > repeat_time) {
                *next_wake_time = repeat_time;
                SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
            }
        }
    }

    if (ret == 0 && epoch > picoquic_epoch_0rtt && length == 0 &&
        cnx->crypto_context[picoquic_epoch_0rtt].aead_encrypt != NULL) {
        length = picoquic_prepare_packet_old_context(cnx, picoquic_packet_context_application,
            path_x, packet, send_buffer_max, current_time, next_wake_time, &header_length);
    }

    /* If there is nothing to send in previous context, check this one too */
    if (length == 0) {
        checksum_overhead = picoquic_get_checksum_length(cnx, epoch);
        packet->checksum_overhead = checksum_overhead;
        bytes_max = bytes + send_buffer_max - checksum_overhead;
        packet->pc = pc;

        tls_ready = picoquic_is_tls_stream_ready(cnx);

        if (ret == 0 && retransmit_possible &&
            (length = picoquic_retransmit_needed(cnx, pc, path_x, current_time, next_wake_time, packet, send_buffer_max, &header_length)) > 0) {
            /* Check whether it makes sense to add an ACK at the end of the retransmission */
            if (epoch != picoquic_epoch_0rtt && length > header_length) {
                bytes_next = picoquic_format_ack_frame(cnx, bytes + length, bytes_max, &more_data, current_time, pc, 0);
                length = bytes_next - bytes;
            } 
            /* document the send time & overhead */
            packet->length = length;
            packet->send_time = current_time;
            packet->checksum_overhead = checksum_overhead;
            *is_initial_sent = (packet->ptype == picoquic_packet_initial);
        }
        else if (ret == 0 && is_cleartext_mode && tls_ready == 0
            && picoquic_find_first_misc_frame(cnx, pc) == NULL
            && !cnx->ack_ctx[pc].act[0].ack_needed && !force_handshake_ping) {
            /* when in a clear text mode, only send packets if there is
            * actually something to send, or resend. */

            packet->length = 0;
        }
        else if (ret == 0) {
            if (cnx->crypto_context[epoch].aead_encrypt == NULL) {
                packet->length = 0;
            }
            else {
                length = picoquic_predict_packet_header_length(cnx, packet_type, &cnx->pkt_ctx[pc]);
                packet->ptype = packet_type;
                packet->offset = length;
                header_length = length;
                packet->sequence_number = cnx->pkt_ctx[pc].send_sequence;
                packet->send_time = current_time;
                packet->send_path = path_x;
                bytes_next = bytes + length;
                bytes_max = bytes + send_buffer_max - checksum_overhead;

                if ((tls_ready == 0 || path_x->cwin <= path_x->bytes_in_transit || cnx->quic->cwin_max <= path_x->bytes_in_transit)
                    && (cnx->cnx_state == picoquic_state_client_almost_ready
                        || picoquic_is_ack_needed(cnx, current_time, next_wake_time, pc, 0) == 0)
                    && picoquic_find_first_misc_frame(cnx, pc) == NULL && !force_handshake_ping) {
                    length = 0;
                }
                else {
                    if (force_handshake_ping) {
                        /* Add PING if handshake is forced */
                        *bytes_next++ = picoquic_frame_type_ping;
                    }
                    if (epoch != picoquic_epoch_0rtt && 
                        (cnx->ack_ctx[pc].act[0].ack_needed ||
                            (force_handshake_ping && picoquic_sack_list_last(&cnx->ack_ctx[pc].sack_list) != UINT64_MAX))) {
                        bytes_next = picoquic_format_ack_frame(cnx, bytes_next, bytes_max, &more_data, current_time, pc, 0);
                    }

                    /* If present, send misc frame -- but only if for the current packet context */
                    bytes_next = picoquic_format_misc_frames_in_context(cnx, bytes_next, bytes_max,
                        &more_data, &is_pure_ack, pc);
                    length = bytes_next - bytes;

                    if (ret == 0 && path_x->cwin > path_x->bytes_in_transit && cnx->quic->cwin_max > path_x->bytes_in_transit) {
                        /* Encode the crypto handshake frame */
                        if (tls_ready != 0) {
                            /* Encode the crypto frame */
                            bytes_next = picoquic_format_crypto_hs_frame(&cnx->tls_stream[epoch],
                                bytes_next, bytes_max, &more_data, &is_pure_ack);
                            length = bytes_next - bytes;
                        }

                        if (packet_type == picoquic_packet_initial) {
                            *is_initial_sent = 1;
                        }
                    }

                    if (length > header_length && epoch == picoquic_epoch_handshake) {
                        cnx->ack_ctx[picoquic_packet_context_initial].act[0].ack_needed = 0;
                    }

                    /* If TLS packets are sent, progress the state */
                    if (ret == 0 && tls_ready != 0 && 
                        cnx->tls_stream[epoch].send_queue == NULL) {
                        switch (cnx->cnx_state) {
                        case picoquic_state_client_init:
                            cnx->cnx_state = picoquic_state_client_init_sent;
                            break;
                        case picoquic_state_client_renegotiate:
                            cnx->cnx_state = picoquic_state_client_init_resent;
                            break;
                        case picoquic_state_client_almost_ready:
                            if (cnx->tls_stream[0].send_queue == NULL &&
                                cnx->tls_stream[1].send_queue == NULL &&
                                cnx->tls_stream[2].send_queue == NULL) {
                                cnx->cnx_state = picoquic_state_client_ready_start;
                                /* Signal the application, because data can now be sent. */
                                if (cnx->callback_fn != NULL) {
                                    if (cnx->callback_fn(cnx, 0, NULL, 0, picoquic_callback_almost_ready, cnx->callback_ctx, NULL) != 0) {
                                        picoquic_log_app_message(cnx, "Callback almost ready returns error 0x%x", PICOQUIC_TRANSPORT_INTERNAL_ERROR);
                                        picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0);
                                    }
                                }
                            }
                            break;
                        default:
                            break;
                        }
                    }
                }
            }
        }
    }

    if (ret == 0 && length == 0 && cnx->crypto_context[1].aead_encrypt != NULL) {
        ret = picoquic_prepare_packet_0rtt(cnx, path_x, packet, current_time, send_buffer, send_buffer_max, send_length,
                *is_initial_sent, next_wake_time);
    }
    else {
        if (ret == 0 && more_data) {
            *next_wake_time = current_time;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }

        if (ret == 0 && *is_initial_sent && packet->ptype == picoquic_packet_1rtt_protected) {
            /* Special case of padding to target length.
            * TODO: this the "client init" case. Is it even possible to send 1RTT packets?
            */
            length = picoquic_pad_to_target_length(bytes, length, send_buffer_max - checksum_overhead);
        }

        if (length > 0 && packet->ptype == picoquic_packet_handshake && !is_pure_ack) {
            /* Sending an ack eliciting handshake packet terminates the use of the initial context */
            picoquic_implicit_handshake_ack(cnx, picoquic_packet_context_initial, current_time);
            picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_initial]);
        }

        picoquic_finalize_and_protect_packet(cnx, packet,
            ret, length, header_length, checksum_overhead,
            send_length, send_buffer, send_buffer_max,
            path_x, current_time);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        self.initial_validated = true;
        let mut epoch = Epoch::Initial;
        if self.tls_stream[Epoch::Initial as usize]
            .send_queue
            .is_empty()
        {
            if self.crypto_context[Epoch::ZeroRtt as usize]
                .aead_encrypt
                .is_some()
                && !self.tls_stream[Epoch::ZeroRtt as usize]
                    .send_queue
                    .is_empty()
            {
                epoch = Epoch::ZeroRtt;
            } else if self.crypto_context[Epoch::Handshake as usize]
                .aead_encrypt
                .is_some()
                && self.tls_stream[Epoch::ZeroRtt as usize]
                    .send_queue
                    .is_empty()
            {
                epoch = Epoch::Handshake;
            }
        }

        let retransmit_possible = matches!(
            self.connection_state,
            State::ClientInitSent | State::ClientInitResent | State::ClientHandshakeStart
        );
        if !matches!(
            self.connection_state,
            State::ClientInit
                | State::ClientInitSent
                | State::ClientInitResent
                | State::ClientRenegotiate
                | State::ClientHandshakeStart
                | State::ClientAlmostReady
        ) {
            return crate::errors::InternalError::UnexpectedState as i32;
        }

        let pc = packet_context_for_epoch(epoch);
        let mut header_length = 0;
        if retransmit_possible {
            let length = self.prepare_packet_old_context_like(
                pc,
                path_x,
                packet,
                send_buffer_max.min(path_x.send_mtu),
                current_time,
                next_wake_time,
                &mut header_length,
            );
            if length > 0 {
                let checksum_overhead = self.get_checksum_length(epoch);
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
            }
        }

        if epoch == Epoch::ZeroRtt {
            return self.prepare_packet_0rtt(
                path_x,
                packet,
                current_time,
                send_buffer,
                send_buffer_max,
                send_length,
                *is_initial_sent,
                next_wake_time,
            );
        }

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

        if ret == 0 && *send_length > 0 && self.tls_stream[epoch as usize].send_queue.is_empty() {
            match self.connection_state {
                State::ClientInit => self.connection_state = State::ClientInitSent,
                State::ClientRenegotiate => self.connection_state = State::ClientInitResent,
                State::ClientAlmostReady
                    if self
                        .tls_stream
                        .iter()
                        .all(|stream| stream.send_queue.is_empty()) =>
                {
                    self.connection_state = State::ClientReadyStart;
                    if let Some(mut cb) = self.callback_fn.take() {
                        let _ = cb.callback(self, 0, &[], CallbackEvent::AlmostReady, None);
                        self.callback_fn = Some(cb);
                    }
                }
                _ => {}
            }
        }
        ret
    }
```

## Pair `picoquic/sender.c:picoquic_false_start_transition`
C: `picoquic/sender.c:2664-2683 picoquic_false_start_transition`
Rust: `rs/fq/src/internal.rs:7685-7715 false_start_transition`

### C body
```c
{
    /* Transition to false start state. */
    cnx->cnx_state = picoquic_state_server_false_start;

    /* On a server that does address validation, send a NEW TOKEN frame */
    if (!cnx->client_mode && (cnx->quic->check_token || cnx->quic->provide_token)) {
        uint8_t token_buffer[256];
        size_t token_size;
        picoquic_connection_id_t n_cid = picoquic_null_connection_id;

        if (picoquic_prepare_retry_token(cnx->quic, (struct sockaddr*) & cnx->path[0]->first_tuple->peer_addr,
            current_time, &n_cid, &n_cid, 0,
            token_buffer, sizeof(token_buffer), &token_size) == 0) {
            if (picoquic_queue_new_token_frame(cnx, token_buffer, token_size) != 0) {
                picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, picoquic_frame_type_new_token);
            }
        }
    }
}
```

### Rust body
```rust
    pub fn client_almost_ready_transition(&mut self) {
        self.connection_state = crate::State::ClientAlmostReady;
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
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_stream_and_datagrams`
C: `picoquic/sender.c:2848-2962 picoquic_prepare_stream_and_datagrams`
Rust: `rs/fq/src/internal.rs:17661-17709 prepare_stream_and_datagrams`

### C body
```c
{
    int datagram_sent = 0;
    int datagram_tried_and_failed = 0;
    int stream_tried_and_failed = 0;
    int more_data_this_round = 0;
    int is_first_round = 1;

    while (bytes_next + 8 < bytes_max && *ret == 0) {
        /* Find the highest priority level for which there is something to send, then
        * format the frames to send at that level. Repeat in a loop until the
        * packet is full or there is nothing more to send. */
        uint64_t datagram_present = cnx->first_datagram != NULL || cnx->is_datagram_ready || path_x->is_datagram_ready;
        picoquic_stream_head_t* first_stream = picoquic_find_ready_stream_path(cnx,
            (cnx->is_multipath_enabled) ? path_x : NULL, 0);
        picoquic_packet_t* first_repeat = picoquic_first_data_repeat_packet(cnx);
        uint64_t current_priority = UINT64_MAX;
        uint64_t stream_priority = UINT64_MAX;
        int something_sent = 0;
        int conflict_found = 0;

        more_data_this_round = 0;

        int datagram_first = (cnx->datagram_conflicts_max >= cnx->datagram_conflicts_count);
        if (datagram_present) {
            current_priority = cnx->datagram_priority;
        }
        if (first_stream != NULL) {
            stream_priority = first_stream->stream_priority;
        }
        if (first_repeat != NULL && first_repeat->data_repeat_priority < stream_priority) {
            stream_priority = first_repeat->data_repeat_priority;
        }
        if (stream_priority < current_priority) {
            current_priority = stream_priority;
        }

        if (current_priority == UINT64_MAX || current_priority >= max_priority_allowed) {
            /* Nothing to send! */
            if (is_first_round) {
                *no_data_to_send = 1;
            }
            break;
        }

        if (datagram_present &&
            cnx->datagram_priority == current_priority &&
            (cnx->datagram_priority < stream_priority || datagram_first)) {
            bytes_next = picoquic_prepare_datagram_ready(cnx, path_x, bytes_next, bytes_max, is_first_in_packet,
                &more_data_this_round, is_pure_ack, &datagram_tried_and_failed, &datagram_sent, ret);
            something_sent = datagram_sent;
        }

        if (first_repeat != NULL && first_repeat->data_repeat_priority == current_priority) {
            uint8_t* bytes_first = bytes_next;
            if (bytes_next + 8 < bytes_max) {
                bytes_next = picoquic_copy_stream_frames_for_retransmit(cnx, bytes_next, bytes_max,
                    UINT64_MAX, &more_data_this_round, is_pure_ack);
                if (bytes_next > bytes_first) {
                    cnx->datagram_conflicts_count = 0;
                    something_sent = 1;
                }
            }
            else {
                more_data_this_round |= 1;
                conflict_found = 1;
            }
        }

        if (first_stream != NULL && first_stream->stream_priority == current_priority &&
            (!first_stream->is_not_coalesced || !something_sent)) {
            /* Encode the stream frame, or frames */
            uint8_t* bytes_first = bytes_next;
            if (bytes_next + 8 < bytes_max) {
                bytes_next = picoquic_format_available_stream_frames(cnx, path_x, bytes_next, bytes_max, UINT64_MAX,
                    &more_data_this_round, is_pure_ack, &stream_tried_and_failed, ret);
                if (bytes_next > bytes_first) {
                    cnx->datagram_conflicts_count = 0;
                    something_sent = 1;
                }
            }
            else {
                more_data_this_round |= 1;
                conflict_found = 1;
            }
        }

        if (datagram_sent && conflict_found) {
            cnx->datagram_conflicts_count += 1;
        }

        if (datagram_present &&
            cnx->datagram_priority == current_priority &&
            cnx->datagram_priority <= stream_priority &&
            !datagram_first) {
            bytes_next = picoquic_prepare_datagram_ready(cnx, path_x, bytes_next, bytes_max, is_first_in_packet,
                more_data, is_pure_ack, &datagram_tried_and_failed, &datagram_sent, ret);
            something_sent = datagram_sent;
        }

        if (is_first_round) {
            *no_data_to_send = ((first_stream == NULL && first_repeat == NULL) || stream_tried_and_failed) &&
                (!datagram_present || datagram_tried_and_failed);
        }
        is_first_round = 0;
        if (!something_sent) {
            break;
        }
    }
    *more_data |= more_data_this_round;

    return bytes_next;
}
```

### Rust body
```rust
    ) -> Option<&'a mut [u8]> {
        *no_data_to_send = 1;
        if self
            .find_ready_stream_path(path_x, is_first_in_packet)
            .is_some()
        {
            let before = bytes.len();
            bytes = format_available_stream_frames(
                self,
                path_x,
                bytes,
                current_priority,
                more_data,
                is_pure_ack,
                no_data_to_send,
                ret,
            )?;
            if bytes.len() != before {
                *no_data_to_send = 0;
            }
        }
        let before = bytes.len();
        let mut datagram_tried_and_failed = 0;
        let mut datagram_sent = 0;
        bytes = self.picoquic_prepare_datagram_ready(
            path_x,
            bytes,
            i32::from(is_first_in_packet),
            more_data,
            is_pure_ack,
            &mut datagram_tried_and_failed,
            &mut datagram_sent,
            ret,
        )?;
        if bytes.len() != before {
            *no_data_to_send = 0;
        }
        Some(bytes)
    }
```

## Pair `picoquic/sender.c:picoquic_prepare_segment`
C: `picoquic/sender.c:3761-3829 picoquic_prepare_segment`
Rust: `rs/fq/src/internal.rs:19424-19529 prepare_segment`

### C body
```c
{
    int ret = 0;

    /* Reset the blocked indicators */
    cnx->cwin_blocked = 0;
    cnx->flow_blocked = 0;
    cnx->stream_blocked = 0;

    /* Prepare header -- depend on connection state */
    /* TODO: 0-RTT work. */
    switch (cnx->cnx_state) {
    case picoquic_state_client_init:
    case picoquic_state_client_init_sent:
    case picoquic_state_client_init_resent:
    case picoquic_state_client_renegotiate:
    case picoquic_state_client_handshake_start:
    case picoquic_state_client_almost_ready:
        ret = picoquic_prepare_packet_client_init(cnx, path_x, packet, current_time, send_buffer, send_buffer_max, send_length, next_wake_time, is_initial_sent);
        break;
    case picoquic_state_server_almost_ready:
    case picoquic_state_server_init:
    case picoquic_state_server_handshake:
        ret = picoquic_prepare_packet_server_init(cnx, path_x, packet, current_time, send_buffer, send_buffer_max, send_length, next_wake_time, is_initial_sent);
        break;
    case picoquic_state_server_false_start:
        /*
         * Manage the end of false start transition, and if needed start
         * preparing packet in ready state.
         */
        if (cnx->cnx_state == picoquic_state_server_false_start &&
            cnx->crypto_context[3].aead_decrypt != NULL) {
            picoquic_ready_state_transition(cnx, current_time);
            return picoquic_prepare_packet_ready(cnx, path_x, packet, current_time, send_buffer, send_buffer_max, send_length, next_wake_time);
        }
        /* Else, just fall through to almost ready behavior.
         */
    case picoquic_state_client_ready_start:
        ret = picoquic_prepare_packet_almost_ready(cnx, path_x, packet, current_time, send_buffer, send_buffer_max, send_length, next_wake_time, is_initial_sent);
        break;
    case picoquic_state_ready:
        ret = picoquic_prepare_packet_ready(cnx, path_x, packet, current_time, send_buffer, send_buffer_max, send_length, next_wake_time);
        break;
    case picoquic_state_handshake_failure:
    case picoquic_state_handshake_failure_resend:
    case picoquic_state_disconnecting:
    case picoquic_state_closing_received:
    case picoquic_state_closing:
    case picoquic_state_draining:
        ret = picoquic_prepare_packet_closing(cnx, path_x, packet, current_time, send_buffer, send_buffer_max, send_length, next_wake_time);
        break;
    case picoquic_state_disconnected:
        ret = PICOQUIC_ERROR_DISCONNECTED;
        break;
    case picoquic_state_client_retry_received:
        DBG_PRINTF("Unexpected connection state: %d\n", cnx->cnx_state);
        ret = PICOQUIC_ERROR_UNEXPECTED_STATE;
        break;
    default:
        DBG_PRINTF("Unexpected connection state: %d\n", cnx->cnx_state);
        ret = PICOQUIC_ERROR_UNEXPECTED_STATE;
        break;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        self.cwin_blocked = false;
        self.flow_blocked = false;
        self.stream_blocked = false;
        match self.connection_state {
            State::ClientInit
            | State::ClientInitSent
            | State::ClientInitResent
            | State::ClientRenegotiate
            | State::ClientHandshakeStart
            | State::ClientAlmostReady => self.prepare_packet_client_init(
                path_x,
                packet,
                current_time,
                send_buffer,
                send_buffer_max,
                send_length,
                next_wake_time,
                is_initial_sent,
            ),
            State::ServerAlmostReady | State::ServerInit | State::ServerHandshake => self
                .prepare_packet_server_init(
                    path_x,
                    packet,
                    current_time,
                    send_buffer,
                    send_buffer_max,
                    send_length,
                    next_wake_time,
                    is_initial_sent,
                ),
            State::ServerFalseStart => {
                if self.crypto_context[Epoch::OneRtt as usize]
                    .aead_decrypt
                    .is_some()
                {
                    self.ready_state_transition(current_time);
                    self.prepare_packet_ready(
                        path_x,
                        packet,
                        current_time,
                        send_buffer,
                        send_buffer_max,
                        send_length,
                        next_wake_time,
                    )
                } else {
                    self.prepare_packet_almost_ready(
                        path_x,
                        packet,
                        current_time,
                        send_buffer,
                        send_buffer_max,
                        send_length,
                        next_wake_time,
                        is_initial_sent,
                    )
                }
            }
            State::ClientReadyStart => self.prepare_packet_almost_ready(
                path_x,
                packet,
                current_time,
                send_buffer,
                send_buffer_max,
                send_length,
                next_wake_time,
                is_initial_sent,
            ),
            State::Ready => self.prepare_packet_ready(
                path_x,
                packet,
                current_time,
                send_buffer,
                send_buffer_max,
                send_length,
                next_wake_time,
            ),
            State::HandshakeFailure
            | State::HandshakeFailureResend
            | State::Disconnecting
            | State::ClosingReceived
            | State::Closing
            | State::Draining => self.prepare_packet_closing(
                path_x,
                packet,
                current_time,
                send_buffer,
                send_buffer_max,
                send_length,
                next_wake_time,
            ),
            State::Disconnected => crate::errors::InternalError::Disconnected as i32,
            _ => crate::errors::InternalError::UnexpectedState as i32,
        }
    }
```

## Pair `picoquic/sender.c:picoquic_program_app_wake_time`
C: `picoquic/sender.c:3894-3903 picoquic_program_app_wake_time`
Rust: `rs/fq/src/internal.rs:16757-16760 program_app_wake_time`

### C body
```c
{
    int ret = 0;

    if (cnx->app_wake_time != 0 && cnx->app_wake_time < *next_wake_time) {
        *next_wake_time = cnx->app_wake_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
    }
    return ret;
}
```

### Rust body
```rust
        if self.app_wake_time.ticks() != 0 && self.app_wake_time < *next_wake_time {
            *next_wake_time = self.app_wake_time;
        }
```

## Pair `picoquic/sender.c:picoquic_prepare_packet_ex`
C: `picoquic/sender.c:3981-4181 picoquic_prepare_packet_ex`
Rust: `rs/fq/src/lib.rs:3539-3695 prepare_packet_ex`

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

## Pair `picoquic/sender.c:picoquic_close_immediate`
C: `picoquic/sender.c:4224-4238 picoquic_close_immediate`
Rust: `rs/fq/src/lib.rs:2357-2410 close_immediate`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    if (cnx->cnx_state < picoquic_state_draining) {
        /* Behave exactly as if having received a closing message from the peer */
        uint64_t current_time = picoquic_get_quic_time(cnx->quic);
        uint64_t exit_time = current_time + 3 * cnx->path[0]->retransmit_timer;
        cnx->cnx_state = picoquic_state_draining;
        cnx->local_error = UINT64_MAX;
        cnx->latest_progress_time = current_time;
        cnx->last_close_sent = current_time;
        picoquic_reinsert_by_wake_time(cnx->quic, cnx, exit_time);
        SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
    }
}
```

### Rust body
```rust
    pub fn reset_cnx(&mut self, current_time: Instant) -> Result<(), Error> {
        for pc in 0..crate::NB_PACKET_CONTEXT {
            if pc != PacketContext::Application as usize {
                let pkt_ctx = &mut self.pkt_ctx[pc];
                pkt_ctx.pending.clear();
                pkt_ctx.retransmitted.clear();
                pkt_ctx.send_sequence = 0;
                pkt_ctx.retransmit_sequence = 0;
                pkt_ctx.next_sequence_hole = 0;
                pkt_ctx.retransmitted_queue_size = 0;
                pkt_ctx.highest_acknowledged = u64::MAX;
                pkt_ctx.latest_time_acknowledged = current_time;
                pkt_ctx.highest_acknowledged_time = current_time;
                self.ack_ctx[pc].reset_ack_context();
            }
        }

        for stream in &mut self.tls_stream {
            stream.clear_stream();
            stream.consumed_offset = 0;
            stream.fin_offset = 0;
            stream.sent_offset = 0;
        }

        fn empty_crypto_context() -> crate::internal::CryptoContext {
            crate::internal::CryptoContext {
                aead_encrypt: None,
                aead_decrypt: None,
                pn_enc: None,
                pn_dec: None,
            }
        }
        for ctx in &mut self.crypto_context {
            *ctx = empty_crypto_context();
        }
        self.crypto_context_new = empty_crypto_context();

        self.setup_initial_traffic_keys()?;
        self.tls_ctx = None;
        if self.quic_ptr.is_null() {
            return Err(Error::InvalidState);
        }
        // SAFETY: quic_ptr is installed by Quic::create_cnx_internal and the
        // owning Quic outlives every connection stored in its arena.
        let quic = unsafe { &mut *self.quic_ptr };
        self.create_tls_context(quic)?;
        self.initialize_tls_stream(current_time)
    }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_create_packet`
C: `picoquic/sim_link.c:80-91 picoquictest_sim_link_create_packet`
Rust: `rs/fq/src/tests/util.rs:149-158 create`

### C body
```c
{
    picoquictest_sim_packet_t* packet = (picoquictest_sim_packet_t*)malloc(sizeof(picoquictest_sim_packet_t));
    if (packet != NULL) {
        packet->next_packet = NULL;
        packet->arrival_time = 0;
        packet->length = 0;
        packet->ecn_mark = 0;
    }

    return packet;
}
```

### Rust body
```rust
    pub fn create() -> Result<Self, crate::Error> {
        Ok(Self {
            arrival_time: Instant::from_ticks(0),
            length: 0,
            addr_from: None,
            addr_to: None,
            ecn_mark: 0,
            bytes: [0u8; MAX_PACKET_SIZE],
        })
    }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_dequeue`
C: `picoquic/sim_link.c:127-142 picoquictest_sim_link_dequeue`
Rust: `rs/fq/src/tests/util.rs:426-431 dequeue`

### C body
```c
{
    picoquictest_sim_packet_t* packet = link->first_packet;

    if (packet != NULL && packet->arrival_time <= current_time) {
        link->first_packet = packet->next_packet;
        if (link->first_packet == NULL) {
            link->last_packet = NULL;
        }
    } else {
        packet = NULL;
    }

    return packet;
}
```

### Rust body
```rust
        {
            return self.packets.pop_front();
        }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_jitter`
C: `picoquic/sim_link.c:230-247 picoquictest_sim_link_jitter`
Rust: `rs/fq/src/tests/harness.rs:90-101 picoquictest_sim_link_jitter`

### C body
```c
{
    uint64_t jitter;

    if (link->jitter_mode == jitter_wifi) {
        jitter = picoquictest_sim_link_wifi_jitter(link);
    }
    else {
        double x = picoquic_test_gauss_random(&link->jitter_seed);
        jitter = link->jitter;
        if (x < -3.0) {
            x = -3.0;
        }
        x /= 3.0;
        jitter += (int64_t)(x * (double)jitter);
    }
    return jitter;
}
```

### Rust body
```rust
    } else {
        let mut x = test_gauss_random(&mut link.jitter_seed);
        if x < -3.0 {
            x = -3.0;
        }
        x /= 3.0;
        let jitter = link.jitter as i64 + (x * link.jitter as f64) as i64;
        jitter.max(0) as u64
    }
```

## Pair `picoquic/sim_link.c:picoquictest_sim_link_submit`
C: `picoquic/sim_link.c:306-332 picoquictest_sim_link_submit`
Rust: `rs/fq/src/tests/util.rs:440-455 submit`

### C body
```c
{
    uint64_t queue_delay = picoquictest_sim_link_queue_delay(link, current_time);
    int should_drop = 0;

    if (link->is_suspended) {
        packet->arrival_time = UINT64_MAX;
        if (link->last_packet == NULL) {
            link->first_packet = packet;
        }
        else {
            link->last_packet->next_packet = packet;
        }
        link->last_packet = packet;
        return;
    }
    if (link->aqm_state != NULL) {
        link->aqm_state->submit(link->aqm_state, link, packet, current_time);
    }
    else {
        if (link->queue_delay_max > 0 && queue_delay >= link->queue_delay_max) {
            should_drop = 1;
        }
        picoquictest_sim_link_enqueue(link, packet, current_time, should_drop);
    }
}
```

### Rust body
```rust
    pub fn submit(&mut self, packet: TestSimPacket, current_time: Instant) {
        if self.is_suspended {
            let mut p = packet;
            p.arrival_time = Instant::from_ticks(u64::MAX);
            self.packets.push_back(p);
            return;
        }
        if let Some(mut aqm) = self.aqm_state.take() {
            aqm.submit(self, packet, current_time);
            self.aqm_state = Some(aqm);
        } else {
            let queue_delay = self.queue_delay(current_time);
            let should_drop = self.queue_delay_max > 0 && queue_delay >= self.queue_delay_max;
            self.enqueue(packet, current_time, should_drop);
        }
    }
```

## Pair `picoquic/sim_link.c:picoquic_set_test_address`
C: `picoquic/sim_link.c:451-462 picoquic_set_test_address`
Rust: `rs/fq/src/tests/util.rs:755-758 set_client_addr`

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

## Pair `picoquic/sockloop.c:picoquic_packet_loop_open_sockets`
C: `picoquic/sockloop.c:464-522 picoquic_packet_loop_open_sockets`
Rust: `rs/fq/src/packet_loop.rs:1653-1736 open_sockets`

### C body
```c
{
    /* Compute how many sockets are necessary, and set the intial value of AF and port per socket */
    int nb_sockets = 0;
    int af[2];
    int nb_af;
    uint16_t current_port = local_port;
    int sock_ret = 0;

    if (local_af == 0) {
#ifdef ESP_PLATFORM
        nb_af = 1;
        af[0] = AF_INET;
#else
        nb_af = 2;
        af[0] = AF_INET;
        af[1] = AF_INET6;
#endif
    }
    else {
        nb_af = 1;
        af[0] = local_af;
    }
    for (int iteration = 0; sock_ret == 0 && iteration < 1 + (extra_socket_required); iteration++) {
        for (int i_af = 0; sock_ret == 0 && i_af < nb_af; i_af++) {
            s_ctx[nb_sockets].af = af[i_af];
            s_ctx[nb_sockets].port = current_port;
            s_ctx[nb_sockets].n_port = htons(current_port);
            s_ctx[nb_sockets].is_port_shared = 0;
            if ((sock_ret = picoquic_packet_loop_open_socket(socket_buffer_size, do_not_use_gso, &s_ctx[nb_sockets], ecn_value)) == 0) {
                if (current_port == 0) {
                    current_port = s_ctx[nb_sockets].port;
                    s_ctx[nb_sockets].n_port = htons(current_port);
                }
                nb_sockets++;
            }
            if (sock_ret == 0 && public_port != 0) {
                s_ctx[nb_sockets].af = af[i_af];
                s_ctx[nb_sockets].port = public_port;
                s_ctx[nb_sockets].n_port = htons(public_port);
                s_ctx[nb_sockets].is_port_shared = (is_shared != 0);
                if ((sock_ret = picoquic_packet_loop_open_socket(socket_buffer_size, do_not_use_gso, &s_ctx[nb_sockets], ecn_value)) == 0) {
                    nb_sockets++;
                }
            }
        }
        current_port = 0;
    }

    if (sock_ret != 0) {
        DBG_PRINTF("Cannot set socket (af=%d, port = %d)\n", s_ctx[nb_sockets].af, s_ctx[nb_sockets].port);
        for (int j = 0; j < nb_sockets; j++) {
            picoquic_packet_loop_close_socket(&s_ctx[j]);
        }
        nb_sockets = 0;
    }
    return nb_sockets;
}
```

### Rust body
```rust
) -> Result<usize, Error> {
    if s_ctx.len() < PACKET_LOOP_SOCKETS_MAX {
        return Err(Error::BufferTooSmall);
    }

    let af = if local_af == 0 {
        [AF_INET, AF_INET6]
    } else {
        [local_af, 0]
    };
    let nb_af = if local_af == 0 { 2 } else { 1 };
    let mut nb_sockets = 0usize;
    let mut current_port = local_port;

    for _iteration in 0..(1 + usize::from(extra_socket_required)) {
        for socket_af in af.iter().take(nb_af).copied() {
            if nb_sockets >= s_ctx.len() {
                return Err(Error::BufferTooSmall);
            }
            s_ctx[nb_sockets] = SocketCtx::default();
            s_ctx[nb_sockets].af = socket_af;
            s_ctx[nb_sockets].port = current_port;
            s_ctx[nb_sockets].n_port = current_port.to_be();
            s_ctx[nb_sockets].is_port_shared = false;

            if let Err(error) = packet_loop_open_socket(
                socket_buffer_size,
                do_not_use_gso,
                &mut s_ctx[nb_sockets],
                ecn_value,
            ) {
                for ctx in s_ctx.iter_mut().take(nb_sockets) {
                    ctx.close();
                }
                return Err(error);
            }
            if current_port == 0 {
                current_port = s_ctx[nb_sockets].port;
                s_ctx[nb_sockets].n_port = current_port.to_be();
            }
            nb_sockets += 1;

            if public_port != 0 {
                if nb_sockets >= s_ctx.len() {
                    for ctx in s_ctx.iter_mut().take(nb_sockets) {
                        ctx.close();
                    }
                    return Err(Error::BufferTooSmall);
                }
                s_ctx[nb_sockets] = SocketCtx::default();
                s_ctx[nb_sockets].af = socket_af;
                s_ctx[nb_sockets].port = public_port;
                s_ctx[nb_sockets].n_port = public_port.to_be();
                s_ctx[nb_sockets].is_port_shared = is_shared;

                if let Err(error) = packet_loop_open_socket(
                    socket_buffer_size,
                    do_not_use_gso,
                    &mut s_ctx[nb_sockets],
                    ecn_value,
                ) {
                    for ctx in s_ctx.iter_mut().take(nb_sockets) {
                        ctx.close();
                    }
                    return Err(error);
                }
                nb_sockets += 1;
            }
        }
        current_port = 0;
    }

    Ok(nb_sockets)
}
```

## Pair `picoquic/sockloop.c:picoquic_packet_loop_v3`
C: `picoquic/sockloop.c:1130-1598 picoquic_packet_loop_v3`
Rust: `rs/fq/src/packet_loop.rs:1320-1325 run`

### C body
```c
{
    picoquic_network_thread_ctx_t* thread_ctx = (picoquic_network_thread_ctx_t*)v_ctx;
    picoquic_quic_t* quic = thread_ctx->quic;
    picoquic_packet_loop_param_t* param = thread_ctx->param;
    picoquic_packet_loop_cb_fn loop_callback = thread_ctx->loop_callback;
    void* loop_callback_ctx = thread_ctx->loop_callback_ctx;
    int ret = 0;
    uint64_t current_time = picoquic_get_quic_time(quic);
    int64_t delay_max = 10000000;
    struct sockaddr_storage addr_from;
    struct sockaddr_storage addr_to;
    int if_index_to;
    uint8_t ecn_value = (quic->default_congestion_alg == NULL) ? 0 : quic->default_congestion_alg->ecn_mark;
#if !defined(_WINDOWS) && !defined(PICOQUIC_WITH_IO_URING)
    uint8_t buffer[1536];
#endif
    uint8_t* send_buffer = NULL;
    size_t send_length = 0;
    size_t send_msg_size = 0;
    size_t send_buffer_size = param->socket_buffer_size;
    size_t* send_msg_ptr = NULL;
    int bytes_recv;
    picoquic_connection_id_t log_cid;
    picoquic_socket_ctx_t s_ctx[PICOQUIC_PACKET_LOOP_SOCKETS_MAX];
    int nb_sockets = 0;
    int nb_sockets_available = 0;
    picoquic_cnx_t* last_cnx = NULL;
    int loop_immediate = 0;
    unsigned int nb_loop_immediate = 0;
    picoquic_packet_loop_options_t options = { 0 };
    packet_loop_system_call_duration_t sc_duration = { 0 };

    int is_wake_up_event;
#if defined(_WINDOWS)
    WSADATA wsaData = { 0 };
    (void)WSA_START(MAKEWORD(2, 2), &wsaData);
#elif defined(PICOQUIC_WITH_IO_URING)
    struct io_uring ring = { 0 };
    int io_uring_is_init = 0;
#elif defined(PICOQUIC_WITH_POLL)
    struct pollfd poll_list[PICOQUIC_PACKET_LOOP_SOCKETS_MAX + 1];
#endif

    PICOQUIC_THREAD_SET_CHECK(thread_ctx->quic);

    if (thread_ctx->thread_name != NULL) {
        thread_ctx->thread_setname_fn(thread_ctx->thread_name);
    }

    if (send_buffer_size == 0) {
        send_buffer_size = 0xffff;
    }

    memset(s_ctx, 0, sizeof(s_ctx));
    if ((nb_sockets = picoquic_packet_loop_open_sockets(param->local_port,
        param->local_af, param->public_port, param->is_port_shared,
        param->socket_buffer_size,
        param->extra_socket_required, param->do_not_use_gso, s_ctx, ecn_value)) <= 0) {
        ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
    }
    else if (loop_callback != NULL) {
        struct sockaddr_storage l_addr;
        ret = loop_callback(quic, picoquic_packet_loop_ready, loop_callback_ctx, &options);

        if (picoquic_store_loopback_addr(&l_addr, s_ctx[0].af, s_ctx[0].port) == 0) {
            ret = loop_callback(quic, picoquic_packet_loop_port_update, loop_callback_ctx, &l_addr);
        }
        if (ret == 0 && options.provide_alt_port) {
            int alt_sock = (nb_sockets > 2 && param->local_af == 0) ? 2 : 1;
            uint16_t alt_port = s_ctx[alt_sock].port;
            ret = loop_callback(quic, picoquic_packet_loop_alt_port, loop_callback_ctx, &alt_port);
        }
    }
#if defined(_WINDOWS)
#elif defined(PICOQUIC_WITH_IO_URING)
    if (ret == 0 &&
        (ret = picoquic_packet_loop_prep_uring(&ring)) == 0) {
        io_uring_is_init = 1;
    }
#elif defined(PICOQUIC_WITH_POLL)
    if (ret == 0) {
        picoquic_packet_loop_set_fds(poll_list, s_ctx, nb_sockets, thread_ctx);
    }
#endif

    if (ret == 0) {
        nb_sockets_available = nb_sockets;

        if (udp_gso_available && !param->do_not_use_gso) {
            send_buffer_size = 0xFFFF;
            send_msg_ptr = &send_msg_size;
        }
        send_buffer = malloc(send_buffer_size);
        if (send_buffer == NULL) {
            ret = -1;
        }
    }

    if (ret == 0) {
        thread_ctx->thread_is_ready = 1;
    }
    else {
        DBG_PRINTF("%s", "Thread cannot run");
    }

    /* Wait for packets */
    /* TODO: add stopping condition, was && (!just_once || !connection_done) */
    /* Actually, no, rely on the callback return code for that? */
    while (ret == 0 && !thread_ctx->thread_should_close) {
        int socket_rank = -1;
        int64_t delta_t = 0;
        uint8_t received_ecn;
        uint8_t* received_buffer;
        uint64_t previous_time;

        if_index_to = 0;
        /* The "loop immediate" condition is set when a packet has been
        * received and processed successfully. We call select again with
        * a delay set to zero to check whether more packets need to be
        * received, trying to empty the receive queue before sending
        * more packet. However, this code is a bit dangerous, 
        * because it can lead to long series of receiving packets without
        * ever sending responses or ACKs. We moderate that by counting the number
        * of loops in "immediate" mode, and ignoring the "loop
        * immediate" condition if that number reaches a limit */
        current_time = picoquic_current_time();
        if (!loop_immediate) {
            nb_loop_immediate = 1;
            delta_t = picoquic_get_next_wake_delay(quic, current_time, delay_max);
            if (options.do_time_check) {
                packet_loop_time_check_arg_t time_check_arg;
                time_check_arg.current_time = current_time;
                time_check_arg.delta_t = delta_t;
                ret = loop_callback(quic, picoquic_packet_loop_time_check, loop_callback_ctx, &time_check_arg);
                if (time_check_arg.delta_t < delta_t) {
                    delta_t = time_check_arg.delta_t;
                }
            }
        }
        else {
            nb_loop_immediate++;
        }
        /* The "loop immediate flag is set by default to zero. It will be
        * set to 1 if a packet has been received and the number of
        * packets received "immediately" does not exceed the limit.
         */
        loop_immediate = 0;
        /* Remember the time before the select call, so it duration be monitored */
        previous_time = current_time;
        /* Initialize the dest addr family to UNSPEC to handle systems that cannot set it. */
        addr_to.ss_family = AF_UNSPEC;
#if defined(_WINDOWS)
        bytes_recv = picoquic_packet_loop_wait(s_ctx, nb_sockets_available,
            &addr_from, &addr_to, &if_index_to, &received_ecn, &received_buffer,
            delta_t, &is_wake_up_event, thread_ctx, &socket_rank);
#elif defined(PICOQUIC_WITH_IO_URING)
        bytes_recv = picoquic_packet_loop_uring(
            &ring, s_ctx, nb_sockets_available, delta_t, thread_ctx,
            &addr_from, &addr_to, &if_index_to, &is_wake_up_event, &received_ecn,
            &received_buffer, &socket_rank);
#elif defined(PICOQUIC_WITH_POLL)
        bytes_recv = picoquic_packet_loop_poll(
            s_ctx, nb_sockets_available,
            poll_list,
            & addr_from,
            & addr_to, & if_index_to, & received_ecn,
            buffer, sizeof(buffer),
            delta_t, & is_wake_up_event, thread_ctx, & socket_rank);
        received_buffer = buffer;
#else
        bytes_recv = picoquic_packet_loop_select(s_ctx, nb_sockets_available,
            &addr_from,
            &addr_to, &if_index_to, &received_ecn,
            buffer, sizeof(buffer),
            delta_t, &is_wake_up_event, thread_ctx, &socket_rank);
        received_buffer = buffer;
#endif
        current_time = picoquic_current_time();
        if (options.do_system_call_duration && delta_t == 0 &&
            monitor_system_call_duration(&sc_duration, current_time, previous_time)) {
            ret = loop_callback(quic, picoquic_packet_loop_system_call_duration,
                loop_callback_ctx, &sc_duration);
        }

        if (bytes_recv < 0) {
            /* The interrupt error is expected if the loop is closing. */
            ret = (thread_ctx->thread_should_close) ? PICOQUIC_NO_ERROR_TERMINATE_PACKET_LOOP : -1;
        }
        else if (bytes_recv == 0 && is_wake_up_event) {
            ret = loop_callback(quic, picoquic_packet_loop_wake_up, loop_callback_ctx, NULL);
        }
        else {
            uint64_t loop_time = current_time;
            size_t bytes_sent = 0;
            size_t nb_packets_sent = 0;

            if (bytes_recv > 0) {
#ifdef _WINDOWS
                size_t recv_bytes = 0;
                while (recv_bytes < (size_t)bytes_recv && ret == 0) {
                    size_t recv_length = (size_t)(bytes_recv - recv_bytes);

                    if (s_ctx[socket_rank].udp_coalesced_size > 0 &&
                        recv_length > s_ctx[socket_rank].udp_coalesced_size) {
                        recv_length = s_ctx[socket_rank].udp_coalesced_size;
                    }
                    /* Submit the packet to the client */
                    ret = picoquic_incoming_packet_ex(quic, s_ctx[socket_rank].recv_buffer + recv_bytes,
                        recv_length, (struct sockaddr*)&addr_from,
                        (struct sockaddr*)&addr_to,
                        s_ctx[socket_rank].dest_if,
                        s_ctx[socket_rank].received_ecn, &last_cnx, current_time);
                    recv_bytes += recv_length;
                }
                if (ret == 0) {
                    ret = picoquic_win_recvmsg_async_start(&s_ctx[socket_rank]);
                }
#else
                /* Submit the packet to the server */
                ret = picoquic_incoming_packet_ex(quic, received_buffer,
                    (size_t)bytes_recv, (struct sockaddr*)&addr_from,
                    (struct sockaddr*)&addr_to, if_index_to, received_ecn,
                    &last_cnx, current_time);
#endif
                if (loop_callback != NULL) {
                    size_t b_recvd = (size_t)bytes_recv;
                    ret = loop_callback(quic, picoquic_packet_loop_after_receive, loop_callback_ctx, &b_recvd);
                }

                /* If the number of packets received in immediate mode has not
                * reached the threshold, set the "immediate" flag and bypass
                * the sending code.
                 */
                if (ret == 0 && nb_loop_immediate < PICOQUIC_PACKET_LOOP_RECV_MAX) {
                    loop_immediate = 1;
                    continue;
                }
            }

            if (ret == PICOQUIC_NO_ERROR_SIMULATE_NAT) {
                if (param->extra_socket_required) {
                    /* Stop using the extra socket.
                     * This will simulate a NAT:
                     * - on the receive side, packets arriving to the old address will be ignored.
                     * - on the send side, client packets will be sent through the main socket,
                     *   and appear to come from that port instead of the extra port.
                     * - since the CID does not change, the server will execute the NAT behavior.
                     * The client will have to update its path -- but that can be avoided if the
                     * test code overrides the value of the "local" address that the client
                     * memorized for that path.
                     */
                    nb_sockets_available = nb_sockets / 2;

#if defined(_WINDOWS)
#elif defined(PICOQUIC_WITH_IO_URING)
#elif defined(PICOQUIC_WITH_POLL)
                    picoquic_packet_loop_set_fds(poll_list, s_ctx, nb_sockets_available, thread_ctx);
#endif
                }
                ret = 0;
            }
            /* We limit the number of packets sent in a loop, no make sure that
            * the code will not spend a lot of time sending packets while
            * packets may be adding in the receive queue.
             */

            while (ret == 0 && nb_packets_sent < PICOQUIC_PACKET_LOOP_SEND_MAX) {
                struct sockaddr_storage peer_addr;
                struct sockaddr_storage local_addr = { 0 };
                int if_index = param->dest_if;
                int sock_ret = 0;
                int sock_err = 0;

                ret = picoquic_prepare_next_packet_ex(quic, loop_time,
                    send_buffer, send_buffer_size, &send_length,
                    &peer_addr, &local_addr, &if_index, &log_cid, &last_cnx,
                    send_msg_ptr);

                if (ret == 0 && send_length > 0) {
                    /* If send_msg_size is defined, sendmsg may send more than one packet.
                     * We compute that to update the number of packets sent in the loop.
                     */
                    nb_packets_sent += (send_msg_size == 0) ? 1 :
                        (send_length + send_msg_size - 1) / (send_msg_size);
                    if (send_length > param->send_length_max) {
                        param->send_length_max = send_length;
                    }
                    /* We have multiple sockets, with support for
                    * either IPv6, or IPv4, or both, and binding to a port number.
                    * Find the first socket where:
                    * - the destination AF is supported.
                    * - either the source port is not specified, or it matches the local port.
                    */
                    SOCKET_TYPE send_socket = INVALID_SOCKET;
                    uint16_t send_port = (peer_addr.ss_family == AF_INET) ?
                        ((struct sockaddr_in*)&local_addr)->sin_port :
                        ((struct sockaddr_in6*)&local_addr)->sin6_port;

                    bytes_sent += send_length;

                    /* TODO: verify htons/ntohs */
                    for (int i = 0; i < nb_sockets_available; i++) {
                        if (s_ctx[i].af == peer_addr.ss_family) {
                            send_socket = s_ctx[i].fd;
                            if (send_port == 0 && !param->prefer_extra_socket) {
                                break;
                            }
                            if (s_ctx[i].n_port == send_port) {
                                break;
                            }
                        }
                    }

                    if (send_socket == INVALID_SOCKET) {
                        if (nb_sockets_available < PICOQUIC_PACKET_LOOP_SOCKETS_MAX) {
                            picoquic_socket_ctx_t* new_ctx = &s_ctx[nb_sockets_available];
                            memset(new_ctx, 0, sizeof(*new_ctx));
                            new_ctx->af = peer_addr.ss_family;
                            if (peer_addr.ss_family == AF_INET6) {
                                new_ctx->port = ntohs(((struct sockaddr_in6*)&peer_addr)->sin6_port);
                            }
                            else {
                                new_ctx->port = ntohs(((struct sockaddr_in*)&peer_addr)->sin_port);
                            }
                            new_ctx->n_port = htons(new_ctx->port);
                            if (picoquic_packet_loop_open_socket(param->socket_buffer_size, param->do_not_use_gso, new_ctx, ecn_value) == 0) {
                                send_socket = new_ctx->fd;
                                send_port = new_ctx->n_port;
                                nb_sockets_available++;
                                if (nb_sockets < nb_sockets_available) {
                                    DBG_PRINTF("new socket, nb = %d", nb_sockets_available);
                                    nb_sockets = nb_sockets_available;

#if defined(_WINDOWS)
#elif defined(PICOQUIC_WITH_IO_URING)
#elif defined(PICOQUIC_WITH_POLL)
                                    picoquic_packet_loop_set_fds(poll_list, s_ctx, nb_sockets_available, thread_ctx);
#endif
                                }
                            }
                        }
                    }

                    if (send_socket == INVALID_SOCKET) {
                        sock_ret = -1;
                        sock_err = -1;
                    }
                    else
                    {
                        if (param->simulate_eio && send_length > PICOQUIC_MAX_PACKET_SIZE) {
                            /* Test hook, simulating a driver that does not support GSO */
                            sock_ret = -1;
                            sock_err = EIO;
                            param->simulate_eio = 0;
                            DBG_PRINTF("Simulating EIO, send length = %zu", send_length);
                        }
                        else {
                            sock_ret = picoquic_sendmsg(send_socket,
                                (struct sockaddr*)&peer_addr, (struct sockaddr*)&local_addr, if_index,
                                (const char*)send_buffer, (int)send_length, (int)send_msg_size, &sock_err);
                        }
                    }
                    if (sock_ret <= 0) {
                        /* TODO: add a test in which the socket fails. */
                        if (last_cnx == NULL) {
                            picoquic_log_context_free_app_message(quic, &log_cid, "Could not send message to AF_to=%d, AF_from=%d, if=%d, ret=%d, err=%d",
                                peer_addr.ss_family, local_addr.ss_family, if_index, sock_ret, sock_err);
                        }
                        else {
                            picoquic_log_app_message(last_cnx, "Could not send message to AF_to=%d, AF_from=%d, if=%d, ret=%d, err=%d",
                                peer_addr.ss_family, local_addr.ss_family, if_index, sock_ret, sock_err);

                            if (picoquic_socket_error_implies_unreachable(sock_err)) {
                                picoquic_notify_destination_unreachable(last_cnx, current_time,
                                    (struct sockaddr*)&peer_addr, (struct sockaddr*)&local_addr, if_index,
                                    sock_err);
                            }
                            else if (sock_err == EIO) {
                                /* TODO: this is an error encountered if the system supports GSO, but
                                 * the specific interface driver does not. Main example is Mininet.
                                 * Not sure that we can treat that correctly. Try to minimize the
                                 * amount of untested code? Rely on config flag? Rely on error
                                 * recovery? */
                                size_t packet_index = 0;
                                size_t packet_size = send_msg_size;

                                while (packet_index < send_length) {
                                    DBG_PRINTF("EIO, length= %zu/%zu", packet_index, send_length);
                                    if (packet_index + packet_size > send_length) {
                                        packet_size = send_length - packet_index;
                                    }
                                    sock_ret = picoquic_sendmsg(send_socket,
                                        (struct sockaddr*)&peer_addr, (struct sockaddr*)&local_addr, if_index,
                                        (const char*)(send_buffer + packet_index), (int)packet_size, 0, &sock_err);
                                    if (sock_ret > 0) {
                                        packet_index += packet_size;
                                    }
                                    else {
                                        DBG_PRINTF("Retry with packet size=%zu fails at index %zu, ret=%d, err=%d.",
                                            packet_size, packet_index, sock_ret, sock_err);
                                        picoquic_log_app_message(last_cnx, "Retry with packet size=%zu fails at index %zu, ret=%d, err=%d.",
                                            packet_size, packet_index, sock_ret, sock_err);
                                        break;
                                    }
                                }
                                if (sock_ret > 0) {
                                    picoquic_log_app_message(last_cnx, "Retry of %zu bytes by chunks of %zu bytes succeeds.",
                                        send_length, send_msg_size);
                                }
                                if (send_msg_ptr != NULL) {
                                    /* Make sure that we do not use GSO anymore in this run */
                                    send_msg_ptr = NULL;
                                    picoquic_log_app_message(last_cnx, "%s", "UDP GSO was disabled");
                                }
                            }
                        }
                    }
                }
                else {
                    break;
                }
            }

            if (ret == 0 && loop_callback != NULL) {
                ret = loop_callback(quic, picoquic_packet_loop_after_send, loop_callback_ctx, &bytes_sent);
            }
        }
    }

    thread_ctx->thread_is_ready = 0;
#if defined(_WINDOWS)
#elif defined(PICOQUIC_WITH_IO_URING)
    if (io_uring_is_init) {
        /* Free the memory allocated for IO_URING */
        io_uring_cancel_and_free(&ring, s_ctx, nb_sockets, thread_ctx);
    }
#elif defined(PICOQUIC_WITH_POLL)
#else
#endif

    if (ret == PICOQUIC_NO_ERROR_TERMINATE_PACKET_LOOP) {
        /* Normal termination requested by the application, returns no error */
        ret = 0;
    }

    /* Close the sockets */
    for (int i = 0; i < nb_sockets; i++) {
        picoquic_packet_loop_close_socket(&s_ctx[i]);
    }

    if (send_buffer != NULL) {
        free(send_buffer);
    }
    thread_ctx->return_code = ret;

#ifdef _WINDOWS
    return (DWORD)ret;
#else
    if (thread_ctx->is_threaded) {
        pthread_exit((void*)&thread_ctx->return_code);
    }
    return(NULL);
#endif
}
```

### Rust body
```rust
        {
            setname.set_name(thread_name);
        }
```

## Pair `picoquic/sockloop.c:picoquic_open_network_wake_up`
C: `picoquic/sockloop.c:1705-1725 picoquic_open_network_wake_up`
Rust: `rs/fq/src/packet_loop.rs:710-716 open_network_wake_up`

### C body
```c
{
    thread_ctx->wake_up_defined = 0;
#ifdef _WINDOWS
    thread_ctx->wake_up_event = CreateEvent(NULL, TRUE, FALSE, NULL);
    if (thread_ctx->wake_up_event == NULL) {
        *ret = GetLastError();
    }
    else {
        thread_ctx->wake_up_defined = 1;
    }
#else
    if (pipe(thread_ctx->wake_up_pipe_fd) != 0) {
        *ret = errno;
    }
    else
    {
        thread_ctx->wake_up_defined = 1;
    }
#endif
}
```

### Rust body
```rust
fn open_network_wake_up(thread_ctx: &mut NetworkThreadCtx, _ret: &mut i32) {
    thread_ctx.wake_up_defined = false;
    let (sender, receiver) = std::sync::mpsc::channel();
    thread_ctx.wake_up_sender = Some(sender);
    thread_ctx.wake_up_receiver = Some(receiver);
    thread_ctx.wake_up_defined = true;
}
```

## Pair `picoquic/sockloop.c:picoquic_start_custom_network_thread`
C: `picoquic/sockloop.c:1768-1812 picoquic_start_custom_network_thread`
Rust: `rs/fq/src/packet_loop.rs:1371-1415 spawn_custom`

### C body
```c
{
    picoquic_network_thread_ctx_t* thread_ctx = (picoquic_network_thread_ctx_t*)malloc(sizeof(picoquic_network_thread_ctx_t));
    *ret = 0;

    if (thread_ctx == NULL) {
        /* Error, no memory */
    }
    else {
        memset(thread_ctx, 0, sizeof(picoquic_network_thread_ctx_t));
        /* Set the thread context in the quic context */
        quic->v_thread_ctx = thread_ctx;
        /* Fill the arguments in the context */
        thread_ctx->quic = quic;
        thread_ctx->param = param;
        thread_ctx->loop_callback = loop_callback;
        thread_ctx->loop_callback_ctx = loop_callback_ctx;
        /* Open the wake up pipe or event */
        picoquic_open_network_wake_up(thread_ctx, ret);
        /* Start thread at specified entry point */
        if (thread_ctx->wake_up_defined){
            thread_ctx->is_threaded = 1;
            if (thread_create_fn == NULL) {
                thread_create_fn = picoquic_internal_thread_create;
            }
            if ((thread_ctx->thread_setname_fn = thread_setname_fn) == NULL) {
                thread_ctx->thread_setname_fn = picoquic_internal_thread_setname;
            }
            if ((thread_ctx->thread_delete_fn = thread_delete_fn) == NULL) {
                thread_ctx->thread_delete_fn = picoquic_internal_thread_delete;
            }
            thread_ctx->thread_name = thread_name;
            if ((*ret = thread_create_fn((void **)&thread_ctx->pthread, picoquic_packet_loop_v3, (void*)thread_ctx)) != 0) {
                /* Free the context and return error condition if something went wrong */
                thread_ctx->is_threaded = 0;
                picoquic_delete_network_thread(thread_ctx);
                thread_ctx = NULL;
            }
        }
    }
    return thread_ctx;
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

## Pair `picoquic/sockloop.c:picoquic_get_thread_ctx`
C: `picoquic/sockloop.c:1935-1940 picoquic_get_thread_ctx`
Rust: `rs/fq/src/packet_loop.rs:1496-1499 thread_ctx`

### C body
```c
{
    return (struct st_picoquic_network_thread_ctx_t*) quic->v_thread_ctx;
}
```

### Rust body
```rust
    pub fn thread_ctx(&mut self) -> Option<&mut NetworkThreadCtx> {
        let b = self.v_thread_ctx.as_mut()?;
        b.as_mut().downcast_mut::<NetworkThreadCtx>()
    }
```

## Pair `picoquic/spinbit.c:picoquic_spinbit_null_incoming`
C: `picoquic/spinbit.c:48-53 picoquic_spinbit_null_incoming`
Rust: `rs/fq/src/spinbit.rs:50-55 incoming`

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
        0
    }
```

## Pair `picoquic/ticket_store.c:picoquic_format_ticket`
C: `picoquic/ticket_store.c:29-99 picoquic_format_ticket`
Rust: `rs/fq/src/tls_api.rs:2702-2736 format_ticket`

### C body
```c
{
    size_t ticket_size = sizeof(picoquic_stored_ticket_t) + sni_length + 1 + alpn_length + 1 + ticket_length
        + 1 + 2*PICOQUIC_STORED_IP_MAX;
    picoquic_stored_ticket_t* stored = (picoquic_stored_ticket_t*)malloc(ticket_size);
    
    if (stored != NULL) {
        char* next_p = ((char*)stored) + sizeof(picoquic_stored_ticket_t);

        memset(stored, 0, ticket_size);
        stored->time_valid_until = time_valid_until;
        stored->sni = next_p;
        stored->sni_length = sni_length;
        memcpy(next_p, sni, sni_length);
        next_p += sni_length;
        *next_p++ = 0;

        stored->alpn = next_p;
        stored->alpn_length = alpn_length;
        memcpy(next_p, alpn, alpn_length);
        next_p += alpn_length;
        *next_p++ = 0;

        stored->version = version;

        stored->ip_addr = (uint8_t *)next_p;
        if (ip_addr == NULL || ip_addr_length == 0) {
            stored->ip_addr_length = 0;
        }
        else {
            if (ip_addr_length > PICOQUIC_STORED_IP_MAX) {
                ip_addr_length = PICOQUIC_STORED_IP_MAX;
            }
            stored->ip_addr_length = ip_addr_length;
            memcpy(next_p, ip_addr, ip_addr_length);
        }
        next_p += PICOQUIC_STORED_IP_MAX;

        stored->ip_addr_client = (uint8_t*)next_p;
        if (ip_addr_client == NULL || ip_addr_client_length == 0) {
            stored->ip_addr_length = 0;
        }
        else {
            if (ip_addr_client_length > PICOQUIC_STORED_IP_MAX) {
                ip_addr_client_length = PICOQUIC_STORED_IP_MAX;
            }
            stored->ip_addr_client_length = ip_addr_client_length;
            memcpy(next_p, ip_addr_client, ip_addr_client_length);
        }
        next_p += PICOQUIC_STORED_IP_MAX;

        if (tp != NULL) {
            stored->tp_0rtt[picoquic_tp_0rtt_max_data] = tp->initial_max_data;
            stored->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_local] = tp->initial_max_stream_data_bidi_local;
            stored->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_remote] = tp->initial_max_stream_data_bidi_remote;
            stored->tp_0rtt[picoquic_tp_0rtt_max_stream_data_uni] = tp->initial_max_stream_data_uni;
            stored->tp_0rtt[picoquic_tp_0rtt_max_streams_id_bidir] = tp->initial_max_stream_id_bidir;
            stored->tp_0rtt[picoquic_tp_0rtt_max_streams_id_unidir] = tp->initial_max_stream_id_unidir;
        }

        stored->ticket = (uint8_t*)next_p;
        stored->ticket_length = ticket_length;
        memcpy(next_p, ticket, ticket_length);
    }

    return stored;
}
```

### Rust body
```rust
) -> StoredTicket {
    use crate::tp::TransportParameter0RttKind::*;
    let tp_0rtt = if let Some(tp) = tp {
        let mut arr = [0u64; NB_TP_0RTT];
        arr[MaxData as usize] = tp.initial_max_data;
        arr[MaxStreamDataBidiLocal as usize] = tp.initial_max_stream_data_bidi_local;
        arr[MaxStreamDataBidiRemote as usize] = tp.initial_max_stream_data_bidi_remote;
        arr[MaxStreamDataUni as usize] = tp.initial_max_stream_data_uni;
        arr[MaxStreamsIdBidir as usize] = tp.initial_max_stream_id_bidir;
        arr[MaxStreamsIdUnidir as usize] = tp.initial_max_stream_id_unidir;
        arr
    } else {
        [0u64; NB_TP_0RTT]
    };
    StoredTicket {
        sni: sni.map(str::to_owned),
        alpn: alpn.map(str::to_owned),
        ip_addr,
        ip_addr_client,
        tp_0rtt,
        ticket: ticket.to_vec(),
        time_valid_until,
        version,
        was_used: false,
    }
}
```
