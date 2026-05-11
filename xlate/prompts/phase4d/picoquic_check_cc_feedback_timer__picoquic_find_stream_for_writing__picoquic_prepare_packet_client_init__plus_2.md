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

## `picoquic/sender.c:picoquic_check_cc_feedback_timer`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C loops over paths, computes lost-feedback times, updates wake time, and notifies congestion control; Rust body only checks for a congestion algorithm then stops.
* C source: `picoquic/sender.c:3848-3879`
* C signature: `int picoquic_check_cc_feedback_timer(picoquic_cnx_t *, uint64_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:16676-16686`
* Rust item: `check_cc_feedback_timer`

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

## `picoquic/sender.c:picoquic_find_stream_for_writing`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C handles missing streams, stream ID validation, closed-stream errors, and allocation errors; shown Rust only returns an existing stream if found.
* C source: `picoquic/sender.c:50-78`
* C signature: `picoquic_stream_head_t * picoquic_find_stream_for_writing(picoquic_cnx_t *, uint64_t, int *)`
* Rust source: `rs/fq/src/lib.rs:3830-3838`
* Rust item: `find_stream_for_writing`

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

## `picoquic/sender.c:picoquic_prepare_packet_client_init`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is much shorter and omits many visible C branches and side effects, including token lookup, forced handshake ping timing, old-context ACK/retransmit handling, misc-frame formatting, padding, implicit handshake ACK, crypto context freeing, and finalization in several paths.
* C source: `picoquic/sender.c:1932-2213`
* C signature: `int picoquic_prepare_packet_client_init(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:18130-18258`
* Rust item: `prepare_packet_client_init`

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

## `picoquic/sender.c:picoquic_set_default_priority`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets default_stream_priority; Rust body sets default_datagram_priority in a differently named function.
* C source: `picoquic/sender.c:207-211`
* C signature: `void picoquic_set_default_priority(picoquic_quic_t *, uint8_t)`
* Rust source: `rs/fq/src/lib.rs:4075-4082`
* Rust item: `set_default_priority`

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

## `picoquic/sockloop.c:picoquic_start_custom_network_thread`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust starts a thread that only drains a channel and marks thread_is_ready true; C stores the context on quic, opens the wakeup mechanism, applies default thread functions, and starts picoquic_packet_loop_v3.
* C source: `picoquic/sockloop.c:1768-1812`
* C signature: `picoquic_network_thread_ctx_t * picoquic_start_custom_network_thread(picoquic_quic_t *, picoquic_packet_loop_param_t *, picoquic_custom_thread_create_fn, picoquic_custom_thread_delete_fn, picoquic_custom_thread_setname_fn, const char *, picoquic_packet_loop_cb_fn, void *, int *)`
* Rust source: `rs/fq/src/packet_loop.rs:1371-1415`
* Rust item: `spawn_custom`

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
