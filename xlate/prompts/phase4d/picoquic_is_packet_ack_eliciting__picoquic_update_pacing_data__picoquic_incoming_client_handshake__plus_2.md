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

## `picoquic/loss_recovery.c:picoquic_is_packet_ack_eliciting`
* Phase 4C status: `suspect`
* Phase 4C rationale: C body visibly skips frames and handles ack-trap/malformed cases inline; Rust delegates that logic to packet_is_ack_eliciting_from_snapshot, so equivalence is not visible from this body alone.
* C source: `picoquic/loss_recovery.c:934-967`
* C signature: `int picoquic_is_packet_ack_eliciting(picoquic_packet_t *)`
* Rust source: `rs/fq/src/internal.rs:10766-10776`
* Rust item: `is_packet_ack_eliciting`

### C body
```c
{
    /* check if this is an ACK eliciting packet */
    int is_ack_eliciting = 0;

    if (packet->is_evaluated) {
        is_ack_eliciting = packet->is_ack_eliciting;
    } else {
        /* Trap packets are never supposed to elicit an ACK. */
        /* For other packets, we need to look at the frames inside. */
        if (!packet->is_ack_trap) {
            size_t frame_length = 0;
            size_t byte_index = packet->offset;

            while (byte_index < packet->length) {
                int frame_is_pure_ack = 0;
                if (picoquic_skip_frame(&packet->bytes[byte_index],
                    packet->length - byte_index, &frame_length, &frame_is_pure_ack) != 0) {
                    /* Malformed packet. Ignore it. Do not expect an ack */
                    break;
                }
                if (!frame_is_pure_ack) {
                    is_ack_eliciting = 1;
                    break;
                }
                byte_index += frame_length;
            }
        }
        packet->is_ack_eliciting = is_ack_eliciting;
        packet->is_evaluated = 1;
    }

    return is_ack_eliciting;
}
```

### Rust body
```rust
fn is_packet_ack_eliciting(packet: &mut Packet) -> bool {
    if packet.is_evaluated {
        return packet.is_ack_eliciting;
    }

    let snapshot = PacketRetransmitSnapshot::from(&*packet);
    let is_ack_eliciting = packet_is_ack_eliciting_from_snapshot(&snapshot);
    packet.is_ack_eliciting = is_ack_eliciting;
    packet.is_evaluated = true;
    is_ack_eliciting
}
```

## `picoquic/pacing.c:picoquic_update_pacing_data`
* Phase 4C status: `suspect`
* Phase 4C rationale: C passes path_x to update_pacing_window; Rust passes None to update_window, a body-visible argument mismatch.
* C source: `picoquic/pacing.c:265-270`
* C signature: `void picoquic_update_pacing_data(picoquic_path_t *, int)`
* Rust source: `rs/fq/src/internal.rs:6074-6082`
* Rust item: `update_pacing_data`

### C body
```c
{
    picoquic_update_pacing_window(&path_x->pacing, slow_start, path_x->cwin, path_x->send_mtu, path_x->smoothed_rtt,
        path_x);
}
```

### Rust body
```rust
    pub fn update_pacing_data(&mut self, slow_start: i32) {
        self.pacing.update_window(
            slow_start,
            self.cwin,
            self.send_mtu,
            self.smoothed_rtt,
            None,
        );
    }
```

## `picoquic/packet.c:picoquic_incoming_client_handshake`
* Phase 4C status: `suspect`
* Phase 4C rationale: After TLS processing, C may run the ready-state transition regardless of the TLS return value, while Rust gates that transition on ret == 0.
* C source: `picoquic/packet.c:1753-1811`
* C signature: `int picoquic_incoming_client_handshake(picoquic_cnx_t *, uint8_t *, picoquic_stream_data_node_t *, picoquic_packet_header *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:7063-7113`
* Rust item: `incoming_client_handshake`

### C body
```c
{
    int ret = 0;

    cnx->initial_validated = 1;
    cnx->initial_repeat_needed = 0;

    if (cnx->cnx_state < picoquic_state_server_almost_ready) {
        if (picoquic_compare_connection_id(&ph->srce_cnx_id, &cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id) != 0) {
            ret = PICOQUIC_ERROR_CNXID_CHECK;
        } else {
            /* Accept the incoming frames */
            if (ph->payload_length == 0) {
                /* empty payload! */
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
            }
            else {
                ret = picoquic_decode_frames(cnx, cnx->path[0],
                    bytes + ph->offset, ph->payload_length, received_data,
                    ph->epoch, NULL, NULL, ph->pn64, 0, current_time);
            }
            /* processing of client clear text packet */
            if (ret == 0) {
                /* Any successful handshake packet is an explicit ack of initial packets */
                picoquic_implicit_handshake_ack(cnx, picoquic_packet_context_initial, current_time);
                picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_initial]);

                /* If TLS data present, progress the TLS state */
                ret = picoquic_tls_stream_process(cnx, NULL, current_time);

                /* If TLS FIN has been received, the server side handshake is ready */
                if (!cnx->client_mode && cnx->cnx_state < picoquic_state_ready && picoquic_is_tls_complete(cnx)) {
                    picoquic_ready_state_transition(cnx, current_time);
                }
            }
        }
    }
    else if (cnx->cnx_state <= picoquic_state_ready) {
        /* Because the client is never guaranteed to discard handshake keys,
         * we need to keep it for the duration of the connection.
         * Process the incoming frames, ignore them, but 
         * require an acknowledgement if the packet contains ackable frames */
        picoquic_ignore_incoming_handshake(cnx, bytes, ph, current_time);
    } 
    else {
        /* Not expected. Log and ignore. */
        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        self.initial_validated = true;
        self.initial_repeat_needed = false;

        let mut ret = 0;
        if self.connection_state < State::ServerAlmostReady {
            if self.path_remote_connection_id(0) != Some(ph.src_connection_id) {
                ret = InternalError::CnxidCheck as i32;
            } else if ph.payload_length == 0 {
                ret = self.connection_error(TransportError::ProtocolViolation as u64, 0);
            } else {
                let payload = Self::packet_payload(bytes, ph);
                ret = self.decode_frames_on_path(
                    0,
                    payload,
                    received_data,
                    ph.epoch,
                    None,
                    None,
                    ph.packet_number_full,
                    0,
                    current_time,
                );
                if ret == 0 {
                    self.implicit_handshake_ack(PacketContext::Initial, current_time);
                    self.crypto_context[crate::internal::Epoch::Initial as usize].free_handles();
                    let (tls_ret, _) = self.process_tls_stream_status(current_time);
                    ret = tls_ret;
                    if ret == 0
                        && !self.client_mode
                        && self.connection_state < State::Ready
                        && self.is_tls_complete()
                    {
                        self.ready_state_transition(current_time);
                    }
                }
            }
        } else if self.connection_state <= State::Ready {
            self.ignore_incoming_handshake(bytes, ph, current_time);
        } else {
            ret = InternalError::UnexpectedPacket as i32;
        }

        ret
    }
```

## `picoquic/paths.c:picoquic_sort_available_paths`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust largely mirrors the selection logic, but the stream affinity comparison uses a synthetic PathToken from the index rather than visibly comparing the path object/pointer as in C.
* C source: `picoquic/paths.c:379-486`
* C signature: `void picoquic_sort_available_paths(picoquic_cnx_t *, uint64_t, uint64_t *, picoquic_path_t **, uint64_t, picoquic_tuple_t **)`
* Rust source: `rs/fq/src/internal.rs:16551-16669`
* Rust item: `picoquic_sort_available_paths`

### C body
```c
{
    int data_path_cwin = -1;
    int data_path_pacing = -1;
    uint64_t pacing_time_next = UINT64_MAX;
    uint64_t last_sent_pacing = UINT64_MAX;
    uint64_t last_sent_cwin = UINT64_MAX;
    int i_min_rtt = -1;
    int is_min_rtt_pacing_ok = 0;
    int is_ack_needed = 0;
    picoquic_stream_head_t* next_stream = picoquic_find_ready_stream(cnx);
    int affinity_path_id = -1;

    /* Several paths are available. We will chose from that.
     */
    for (int path_index = 0; path_index < cnx->nb_paths; path_index++) {
        picoquic_path_t* path_x = cnx->path[path_index];
        /* Clear the nominal ack path flag from all path -- it will be reset to the low RTT path later */
        path_x->is_nominal_ack_path = 0;
        /* Only continue processing if the path is available */
        if (path_x->path_is_backup || !path_x->first_tuple->challenge_verified || path_x->path_is_demoted || path_x->nb_retransmit > min_retransmit) {
            continue;
        }
        /* This path is a candidate for min rtt */
        if (i_min_rtt < 0 ||
            path_x->nb_retransmit < cnx->path[i_min_rtt]->nb_retransmit ||
            (path_x->nb_retransmit == cnx->path[i_min_rtt]->nb_retransmit &&
                path_x->rtt_min < cnx->path[i_min_rtt]->rtt_min)) {
            i_min_rtt = path_index;
            is_min_rtt_pacing_ok = 0;
        }
        path_x->polled++;
        /* Find the best path authorized by pacing and then by congestion control,
         * taking into account affinity, datagrams, etc.
         */
        if (picoquic_is_sending_authorized_by_pacing(cnx, path_x, current_time, &pacing_time_next)) {
            if (path_x->last_sent_time < last_sent_pacing) {
                last_sent_pacing = path_x->last_sent_time;
                data_path_pacing = path_index;
                if (path_index == i_min_rtt) {
                    is_min_rtt_pacing_ok = 1;
                }
            }
            if (path_x->bytes_in_transit < path_x->cwin &&
                path_x->bytes_in_transit < cnx->quic->cwin_max) {
                if (path_x->last_sent_time < last_sent_cwin) {
                    last_sent_cwin = path_x->last_sent_time;
                    data_path_cwin = path_index;
                }
                if (affinity_path_id < 0) {
                    /* we select here the first path that is either ready to send on
                        * the highest priority stream with affinity on this path, or
                        * ready to send datagrams on this path. */
                    if (next_stream != NULL && path_x == next_stream->affinity_path) {
                        affinity_path_id = path_index;
                    }
                    else if (path_x->is_datagram_ready || cnx->is_datagram_ready) {
                        affinity_path_id = path_index;
                    }
                }
            }
            else {
                path_x->congested++;
            }
        }
        else {
            path_x->paced++;
        }
    }
    /* Putting it all together:
     * - fix the nominal ACK path that we erased.
     * - retain the ACK
     */

    if (i_min_rtt >= 0) {
        is_ack_needed = picoquic_is_ack_needed(cnx, current_time, next_wake_time, 0, 0);
        cnx->path[i_min_rtt]->is_nominal_ack_path = 1;
    }

    if (is_ack_needed && is_min_rtt_pacing_ok) {
        *next_path = cnx->path[i_min_rtt];
    }
    else if (data_path_cwin >= 0) {
        /* if there is a path ready to send the most urgent data, select it */
        if (affinity_path_id >= 0) {
            *next_path = cnx->path[affinity_path_id];
        }
        else {
            *next_path = cnx->path[data_path_cwin];
        }
    }
    else if (data_path_pacing >= 0) {
        *next_path = cnx->path[data_path_pacing];
    }
    else {
        /* No path is ready at all. Set the next wake time to the min of current
         * value and next pacing time.
         */
        if (pacing_time_next < *next_wake_time) {
            *next_wake_time = pacing_time_next;
            SET_LAST_WAKE(cnx->quic, PICOQUIC_SENDER);
        }
        *next_path = cnx->path[0];
    }
    (*next_path)->selected++;
    *next_tuple = (*next_path)->first_tuple;
}
```

### Rust body
```rust
    ) -> Option<(PathToken, usize)> {
        if self.paths.is_empty() {
            return None;
        }

        let mut data_path_cwin: Option<usize> = None;
        let mut data_path_pacing: Option<usize> = None;
        let mut pacing_time_next = Instant::from_ticks(u64::MAX);
        let mut last_sent_pacing = Instant::from_ticks(u64::MAX);
        let mut last_sent_cwin = Instant::from_ticks(u64::MAX);
        let mut i_min_rtt: Option<usize> = None;
        let mut is_min_rtt_pacing_ok = false;
        let next_stream = self.find_ready_stream();
        let mut affinity_path_id: Option<usize> = None;
        let cwin_max = self.quic_cwin_max();

        for path_index in 0..self.paths.len() {
            {
                let path_x = &mut self.paths[path_index];
                path_x.is_nominal_ack_path = false;
                let challenge_verified = path_x
                    .tuples
                    .first()
                    .is_some_and(|tuple| tuple.challenge_verified);
                if path_x.path_is_backup
                    || !challenge_verified
                    || path_x.path_is_demoted
                    || path_x.nb_retransmit > min_retransmit
                {
                    continue;
                }
            }

            let replace_min_rtt = match i_min_rtt {
                None => true,
                Some(min_idx) => {
                    self.paths[path_index].nb_retransmit < self.paths[min_idx].nb_retransmit
                        || (self.paths[path_index].nb_retransmit
                            == self.paths[min_idx].nb_retransmit
                            && self.paths[path_index].rtt_min < self.paths[min_idx].rtt_min)
                }
            };
            if replace_min_rtt {
                i_min_rtt = Some(path_index);
                is_min_rtt_pacing_ok = false;
            }

            self.paths[path_index].polled = self.paths[path_index].polled.saturating_add(1);

            if self.is_sending_authorized_by_pacing(path_index, current_time, &mut pacing_time_next)
            {
                if self.paths[path_index].last_sent_time < last_sent_pacing {
                    last_sent_pacing = self.paths[path_index].last_sent_time;
                    data_path_pacing = Some(path_index);
                    if Some(path_index) == i_min_rtt {
                        is_min_rtt_pacing_ok = true;
                    }
                }
                if self.paths[path_index].bytes_in_transit < self.paths[path_index].cwin
                    && self.paths[path_index].bytes_in_transit < cwin_max
                {
                    if self.paths[path_index].last_sent_time < last_sent_cwin {
                        last_sent_cwin = self.paths[path_index].last_sent_time;
                        data_path_cwin = Some(path_index);
                    }
                    if affinity_path_id.is_none() {
                        let path_token = PathToken::synthetic(path_index as u32, path_index as u32);
                        let stream_affinity_matches = next_stream
                            .and_then(|stream| self.streams.get(stream))
                            .and_then(|stream| stream.affinity_path)
                            .is_some_and(|affinity| affinity == path_token);
                        if stream_affinity_matches
                            || self.paths[path_index].is_datagram_ready
                            || self.is_datagram_ready
                        {
                            affinity_path_id = Some(path_index);
                        }
                    }
                } else {
                    self.paths[path_index].congested =
                        self.paths[path_index].congested.saturating_add(1);
                }
            } else {
                self.paths[path_index].paced = self.paths[path_index].paced.saturating_add(1);
            }
        }

        let mut is_ack_needed = false;
        if let Some(min_idx) = i_min_rtt {
            is_ack_needed =
                self.is_ack_needed(current_time, next_wake_time, PacketContext::Application, 0);
            self.paths[min_idx].is_nominal_ack_path = true;
        }

        let selected = if is_ack_needed && is_min_rtt_pacing_ok {
            i_min_rtt
        } else if let Some(cwin_idx) = data_path_cwin {
            Some(affinity_path_id.unwrap_or(cwin_idx))
        } else if let Some(pacing_idx) = data_path_pacing {
            Some(pacing_idx)
        } else {
            if pacing_time_next < *next_wake_time {
                *next_wake_time = pacing_time_next;
            }
            Some(0)
        }?;

        self.paths[selected].selected = self.paths[selected].selected.saturating_add(1);
        if self.paths[selected].tuples.is_empty() {
            None
        } else {
            Some((PathToken::synthetic(selected as u32, selected as u32), 0))
        }
    }
```

## `picoquic/picoquic_ptls_minicrypto.c:picoquic_ptls_minicrypto_load`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust delegates to ptls_minicrypto_load_locked, so the visible body does not show the C body's unload clear path or registration/init operations.
* C source: `picoquic/picoquic_ptls_minicrypto.c:54-83`
* C signature: `void picoquic_ptls_minicrypto_load(int)`
* Rust source: `rs/fq/src/tls_api.rs:2528-2531`
* Rust item: `ptls_minicrypto_load`

### C body
```c
{
    if (unload) {
        picoquic_clear_minicrypto();
    }
    else {
        picoquic_init_minicrypto();

        picoquic_register_ciphersuite(&ptls_minicrypto_aes128gcmsha256, 1);
        picoquic_register_ciphersuite(&ptls_minicrypto_aes256gcmsha384, 1);
        picoquic_register_ciphersuite(&ptls_minicrypto_chacha20poly1305sha256, 1);
        picoquic_register_key_exchange_algorithm(&ptls_minicrypto_secp256r1);
        picoquic_register_key_exchange_algorithm(&ptls_minicrypto_x25519);

        picoquic_register_crypto_random_provider_fn(ptls_minicrypto_random_bytes);

        picoquic_register_tls_key_provider_fn(
            set_minicrypto_private_key_from_key_file,
            NULL, NULL, NULL);
#if 0
        picoquic_register_verify_certificate_fn(picoquic_openssl_get_certificate_verifier,
            picoquic_openssl_dispose_certificate_verifier,
            picoquic_openssl_set_tls_root_certificates);
        picoquic_register_explain_crypto_error_fn(picoquic_open_ssl_explain_crypto_error,
            picoquic_openssl_clear_crypto_errors);
#endif
    }
}
```

### Rust body
```rust
pub fn ptls_minicrypto_load(unload: bool) {
    let mut state = tls_api_state();
    ptls_minicrypto_load_locked(&mut state, i32::from(unload));
}
```
