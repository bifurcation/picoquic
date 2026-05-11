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

## `picoquic/sacks.c:picoquic_update_ack_horizon`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C loops over old SACK items, checks nb_times_sent and delay, deletes ranges, and sets ack_horizon to a packet number; Rust only derives ack_horizon from current_time and horizon_delay.
* C source: `picoquic/sacks.c:382-403`
* C signature: `void picoquic_update_ack_horizon(picoquic_sack_list_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:8357-8364`
* Rust item: `update_ack_horizon`

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

## `picoquic/sender.c:picoquic_finalize_and_protect_packet_tuple`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C assigns sequence numbers, updates path send metrics, handles version negotiation/default packet types distinctly, queues protected packets for retransmit, and updates bytes_sent; Rust does not show those steps and maps unsupported packet types to OneRtt.
* C source: `picoquic/sender.c:1175-1260`
* C signature: `void picoquic_finalize_and_protect_packet_tuple(picoquic_cnx_t *, picoquic_packet_t *, int, size_t, size_t, size_t, size_t *, uint8_t *, size_t, picoquic_path_t *, uint64_t, picoquic_tuple_t *)`
* Rust source: `rs/fq/src/internal.rs:7437-7496`
* Rust item: `finalize_and_protect_packet_tuple`

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

## `picoquic/sender.c:picoquic_prepare_packet_almost_ready`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust returns immediately after old-context packets without body-visible padding, initial_repeat_needed reset, accounting, next-wake update, or the main almost-ready packet construction logic shown in C.
* C source: `picoquic/sender.c:2964-3284`
* C signature: `int picoquic_prepare_packet_almost_ready(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_t *, uint64_t, uint8_t *, size_t, size_t *, uint64_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:19306-19418`
* Rust item: `prepare_packet_almost_ready`

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

## `picoquic/sender.c:picoquic_set_datagram_priority`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets cnx->datagram_priority; Rust shown is set_default_priority and sets default_stream_priority.
* C source: `picoquic/sender.c:201-205`
* C signature: `void picoquic_set_datagram_priority(picoquic_cnx_t *, uint8_t)`
* Rust source: `rs/fq/src/lib.rs:4068-4077`
* Rust item: `set_datagram_priority`

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

## `picoquic/sockloop.c:picoquic_packet_loop_v3`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only sets a thread name; C body opens sockets, runs the packet receive/send loop, invokes callbacks, handles wakeups, sends packets, cleans up sockets and buffer, and stores the return code.
* C source: `picoquic/sockloop.c:1130-1598`
* C signature: `void * picoquic_packet_loop_v3(void *)`
* Rust source: `rs/fq/src/packet_loop.rs:1320-1325`
* Rust item: `run`

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
