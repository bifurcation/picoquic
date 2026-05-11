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

## `picoquic/sender.c:picoquic_close_immediate`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is reset_cnx, clearing packet contexts, streams, crypto contexts, and TLS setup; C body transitions to draining, records close times/errors, and schedules wake time.
* C source: `picoquic/sender.c:4224-4238`
* C signature: `void picoquic_close_immediate(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2357-2410`
* Rust item: `close_immediate`

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

## `picoquic/sender.c:picoquic_implicit_handshake_ack`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C walks each pending packet, updates congestion/delivered state conditionally, and dequeues each packet; Rust just clears pending/retransmitted queues and sizes.
* C source: `picoquic/sender.c:1835-1866`
* C signature: `void picoquic_implicit_handshake_ack(picoquic_cnx_t *, picoquic_packet_context_enum, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:7582-7587`
* Rust item: `implicit_handshake_ack`

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

## `picoquic/sender.c:picoquic_prepare_packet_ready`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust omits visible C blocks for CID refresh, key rotation, priority bypass under cwin block, preemptive retransmit/forced probe handling, and differs in some ACK ping conditions.
* C source: `picoquic/sender.c:3286-3717`
* C signature: `int picoquic_prepare_packet_ready(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:18674-19300`
* Rust item: `prepare_packet_ready`

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

## `picoquic/sim_link.c:picoquictest_sim_link_dequeue`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust unconditionally pops the front packet, while C only dequeues when the first packet exists and arrival_time is less than or equal to current_time.
* C source: `picoquic/sim_link.c:127-142`
* C signature: `picoquictest_sim_packet_t * picoquictest_sim_link_dequeue(picoquictest_sim_link_t *, uint64_t)`
* Rust source: `rs/fq/src/tests/util.rs:426-431`
* Rust item: `dequeue`

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

## `picoquic/sockloop.c:picoquic_start_server_threads`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust consumes callback/thread function options with take across the loop and sets a default ticket key even if temporary QUIC creation fails; C reuses the provided functions and only sets the key after successful generation.
* C source: `picoquic/sockloop.c:1966-2065`
* C signature: `int picoquic_start_server_threads(struct st_picoquic_quic_config_t *, uint64_t, picoquic_alpn_select_fn_v2, picoquic_stream_data_cb_fn, void *, picoquic_packet_loop_cb_fn, void *, picoquic_custom_thread_create_fn, picoquic_custom_thread_delete_fn, picoquic_custom_thread_setname_fn, picoquic_network_thread_ctx_t **, int, int *)`
* Rust source: `rs/fq/src/packet_loop.rs:1551-1630`
* Rust item: `start_server_threads`

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
