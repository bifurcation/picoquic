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

## `picoquic/loss_recovery.c:picoquic_count_and_notify_loss`
* Phase 4C status: `suspect`
* Phase 4C rationale: C logs packet_lost even when send_path is NULL, while Rust logs app_message in the no-path case; the visible logging behavior differs.
* C source: `picoquic/loss_recovery.c:895-932`
* C signature: `void picoquic_count_and_notify_loss(picoquic_cnx_t *, picoquic_packet_t *, int, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:11117-11219`
* Rust item: `count_and_notify_loss`

### C body
```c
{
    if (timer_based_retransmit < 2) {
        picoquic_log_packet_lost(cnx, old_p->send_path, old_p->ptype, old_p->sequence_number,
            (timer_based_retransmit) ? "timer" : "repeat",
            (old_p->send_path == NULL || old_p->send_path->first_tuple->p_remote_cnxid == NULL) ? NULL : &old_p->send_path->first_tuple->p_remote_cnxid->cnx_id,
            old_p->length, current_time);

        if (!old_p->is_preemptive_repeat) {
            cnx->nb_retransmission_total++;
        }
    }

    if (old_p->send_path != NULL) {
        old_p->send_path->nb_losses_found++;
        if (timer_based_retransmit) {
            old_p->send_path->nb_timer_losses++;
        }
        if ((old_p->send_path->smoothed_rtt != PICOQUIC_INITIAL_RTT ||
            old_p->send_path->rtt_variant != 0) &&
            old_p->send_time > cnx->start_time + old_p->send_path->smoothed_rtt) {
            /* we do not count losses occruring before ready state, because the 
             * timers are not reliable yet */
            old_p->send_path->total_bytes_lost += old_p->length;
        }

        if (cnx->congestion_alg != NULL && cnx->cnx_state >= picoquic_state_ready && old_p->send_path != NULL) {
            picoquic_per_ack_state_t ack_state = { 0 };
            ack_state.pc = old_p->pc;
            ack_state.lost_packet_number = old_p->sequence_number;
            ack_state.nb_bytes_newly_lost = old_p->length;
            cnx->congestion_alg->alg_notify(cnx, old_p->send_path,
                (timer_based_retransmit == 0) ? picoquic_congestion_notification_repeat : picoquic_congestion_notification_timeout,
                &ack_state, current_time);
        }
    }
}
```

### Rust body
```rust
    ) {
        let path_idx = old_p
            .send_path
            .and_then(|send_path| self.path_index_from_token(send_path));
        let dcid = path_idx.and_then(|idx| {
            let path = &self.paths[idx];
            let cid_idx = path
                .tuples
                .first()
                .and_then(|tuple| tuple.remote_connection_id_index)?;
            self.remote_connection_id_stashes
                .iter()
                .find(|stash| stash.unique_path_id == path.unique_path_id)
                .and_then(|stash| stash.connection_ids.get(cid_idx))
                .map(|remote| remote.connection_id)
        });

        if let Some(idx) = path_idx {
            let mut path = self.paths.remove(idx);
            if timer_based_retransmit < 2 {
                crate::logger::Log::packet_lost(
                    self,
                    &mut path,
                    old_p.packet_type,
                    old_p.sequence_number,
                    if timer_based_retransmit != 0 {
                        "timer"
                    } else {
                        "repeat"
                    },
                    dcid.as_ref(),
                    old_p.length,
                    current_time,
                );
            }

            path.nb_losses_found = path.nb_losses_found.saturating_add(1);
            if timer_based_retransmit != 0 {
                path.nb_timer_losses = path.nb_timer_losses.saturating_add(1);
            }
            if (path.smoothed_rtt != INITIAL_RTT || path.rtt_variant.ticks() != 0)
                && old_p.send_time
                    > Instant::from_ticks(
                        self.start_time
                            .ticks()
                            .saturating_add(path.smoothed_rtt.ticks()),
                    )
            {
                path.total_bytes_lost = path.total_bytes_lost.saturating_add(old_p.length as u64);
            }

            let mut cc_notified = false;
            if let Some(cc_alg) = self.congestion_alg
                && self.connection_state >= State::Ready
            {
                let ack_state = PerAckState {
                    pc: old_p.packet_context as i32,
                    lost_packet_number: old_p.sequence_number,
                    nb_bytes_newly_lost: old_p.length as u64,
                    ..PerAckState::default()
                };
                cc_alg.algorithm.alg_notify(
                    self,
                    &mut path,
                    if timer_based_retransmit == 0 {
                        CongestionNotification::Repeat
                    } else {
                        CongestionNotification::Timeout
                    },
                    &ack_state,
                    current_time,
                );
                cc_notified = true;
            }
            self.paths.insert(idx, path);
            if cc_notified {
                self.report_pacing_update(idx);
            }
        } else if timer_based_retransmit < 2 {
            crate::logger::Log::app_message(
                self,
                format_args!(
                    "Packet lost, type {:?}, seq {}, trigger {}",
                    old_p.packet_type,
                    old_p.sequence_number,
                    if timer_based_retransmit != 0 {
                        "timer"
                    } else {
                        "repeat"
                    }
                ),
            );
        }

        if timer_based_retransmit < 2 && !old_p.is_preemptive_repeat {
            self.nb_retransmission_total = self.nb_retransmission_total.saturating_add(1);
        }
    }
```

## `picoquic/pacing.c:picoquic_report_pacing_update`
* Phase 4C status: `suspect`
* Phase 4C rationale: The C body contains the actual threshold checks and callback invocations; the Rust body shown only removes a path, calls report_pacing_update_for_path, and reinserts it, so equivalence depends on an unshown helper.
* C source: `picoquic/pacing.c:107-135`
* C signature: `void picoquic_report_pacing_update(picoquic_pacing_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:6220-6225`
* Rust item: `report_pacing_update`

### C body
```c
{
    picoquic_cnx_t* cnx = path_x->cnx;

    if (cnx->is_pacing_update_requested && path_x == cnx->path[0] &&
        cnx->callback_fn != NULL) {
        if ((pacing->rate > cnx->pacing_rate_signalled &&
            (pacing->rate - cnx->pacing_rate_signalled >= cnx->pacing_increase_threshold)) ||
            (pacing->rate < cnx->pacing_rate_signalled &&
                (cnx->pacing_rate_signalled - pacing->rate > cnx->pacing_decrease_threshold))){
            (void)cnx->callback_fn(cnx, pacing->rate, NULL, 0, picoquic_callback_pacing_changed, cnx->callback_ctx, NULL);
            cnx->pacing_rate_signalled = pacing->rate;
        }
    }
    if (cnx->is_path_quality_update_requested &&
        cnx->callback_fn != NULL) {
        /* TODO: add a function "export path quality" */
        /* TODO: remember previous signalled value for change tests */
        if (path_x->smoothed_rtt < path_x->rtt_threshold_low ||
            path_x->smoothed_rtt > path_x->rtt_threshold_high ||
            pacing->rate < path_x->pacing_rate_threshold_low ||
            pacing->rate > path_x->pacing_rate_threshold_high) {
            (void)cnx->callback_fn(cnx, path_x->unique_path_id, NULL, 0, picoquic_callback_path_quality_changed, cnx->callback_ctx, path_x->app_path_ctx);
            picoquic_refresh_path_quality_thresholds(path_x);
        }
    }
}
```

### Rust body
```rust
        if path_index < self.paths.len() {
            let mut path = self.paths.remove(path_index);
            self.report_pacing_update_for_path(&mut path, path_index == 0);
            self.paths.insert(path_index, path);
        }
```

## `picoquic/packet.c:picoquic_incoming_1rtt`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust removes the path from self.paths before frame decoding and reinserts it afterward, while C decodes with the path still present in cnx->path[path_id].
* C source: `picoquic/packet.c:1910-2017`
* C signature: `int picoquic_incoming_1rtt(picoquic_cnx_t *, int, uint8_t *, picoquic_stream_data_node_t *, picoquic_packet_header *, struct sockaddr *, struct sockaddr *, int, int, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:7167-7303`
* Rust item: `incoming_1rtt`

### C body
```c
{
    int ret = 0;

    /* Check the packet */
    if (cnx->cnx_state < picoquic_state_client_almost_ready) {
        /* handshake is not complete. Just ignore the packet */
        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
    }
    else if (cnx->cnx_state == picoquic_state_disconnected) {
        /* Connection is disconnected. Just ignore the packet */
        ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
    }
    else {
        /* Packet is correct */

        /* TODO: consider treatment of migration during closing mode */

        /* Do not process data in closing or draining modes */
        if (cnx->cnx_state >= picoquic_state_disconnecting) {
            /* only look for closing frames in closing modes */
            if (cnx->cnx_state == picoquic_state_closing || cnx->cnx_state == picoquic_state_disconnecting) {
                int closing_received = 0;

                ret = picoquic_decode_closing_frames(
                    bytes + ph->offset, ph->payload_length, &closing_received);

                if (ret == 0) {
                    if (closing_received) {
                        if (cnx->client_mode) {
                            picoquic_connection_disconnect(cnx);
                        }
                        else {
                            cnx->cnx_state = picoquic_state_draining;
                        }
                    }
                    else {
                        picoquic_set_ack_needed(cnx, current_time, ph->pc, cnx->path[path_id], 0);
                    }
                }
            }
            else {
                /* Just ignore the packets in closing received or draining mode */
                ret = PICOQUIC_ERROR_UNEXPECTED_PACKET;
            }
        }
        else if (ret == 0) {
            picoquic_path_t* path_x = cnx->path[path_id];

            path_x->first_tuple->if_index = if_index_to;
            cnx->is_1rtt_received = 1;
            picoquic_spin_function_table[cnx->spin_policy].spinbit_incoming(cnx, path_x, ph);
            /* Accept the incoming frames */
            ret = picoquic_decode_frames(cnx, cnx->path[path_id],
                bytes + ph->offset, ph->payload_length, received_data,
                ph->epoch, addr_from, addr_to, ph->pn64,
                path_is_not_allocated, current_time);

            if (ret == 0) {
                /* Compute receive bandwidth */
                path_x->received += (uint64_t)ph->offset + ph->payload_length +
                    picoquic_get_checksum_length(cnx, picoquic_epoch_1rtt);
                if (path_x->receive_rate_epoch == 0) {
                    path_x->received_prior = cnx->path[path_id]->received;
                    path_x->receive_rate_epoch = current_time;
                }
                else {
                    uint64_t delta = current_time - cnx->path[path_id]->receive_rate_epoch;
                    if (delta > path_x->smoothed_rtt && delta > PICOQUIC_BANDWIDTH_TIME_INTERVAL_MIN) {
                        path_x->receive_rate_estimate = PICOQUIC_RATE_FROM_BYTES(
                            cnx->path[path_id]->received - cnx->path[path_id]->received_prior, delta);
                        path_x->received_prior = cnx->path[path_id]->received;
                        path_x->receive_rate_epoch = current_time;
                        if (path_x->receive_rate_estimate > cnx->path[path_id]->receive_rate_max) {
                            path_x->receive_rate_max = cnx->path[path_id]->receive_rate_estimate;
                            if (path_id == 0 && !cnx->is_ack_frequency_negotiated) {
                                picoquic_compute_ack_gap_and_delay(cnx, cnx->path[0]->rtt_min, PICOQUIC_ACK_DELAY_MIN,
                                    cnx->path[0]->receive_rate_max, &cnx->ack_gap_remote, &cnx->ack_delay_remote);
                            }
                        }
                    }
                }

                /* Processing of TLS messages  */
                ret = picoquic_tls_stream_process(cnx, NULL, current_time);
            }

            if (ret == 0 && picoquic_cnx_is_still_logging(cnx)) {
                picoquic_log_cc_dump(cnx, current_time);
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        if self.connection_state < State::ClientAlmostReady {
            return InternalError::UnexpectedPacket as i32;
        }
        if self.connection_state == State::Disconnected {
            return InternalError::UnexpectedPacket as i32;
        }

        if self.connection_state >= State::Disconnecting {
            if self.connection_state == State::Closing
                || self.connection_state == State::Disconnecting
            {
                let payload = Self::packet_payload_mut(bytes, ph);
                let mut closing_received = 0;
                let ret = crate::internal::decode_closing_frames(
                    payload,
                    payload.len(),
                    &mut closing_received,
                );
                if ret == 0 {
                    if closing_received != 0 {
                        if self.client_mode {
                            self.connection_disconnect();
                        } else {
                            self.connection_state = State::Draining;
                        }
                    } else {
                        self.set_ack_needed_on_path(current_time, ph.packet_context, path_id, 0);
                    }
                }
                ret
            } else {
                InternalError::UnexpectedPacket as i32
            }
        } else {
            if path_id >= self.paths.len() {
                return InternalError::UnexpectedPacket as i32;
            }

            let payload = Self::packet_payload(bytes, ph);
            let mut path = self.paths.remove(path_id);
            if let Some(tuple) = path.tuples.first_mut() {
                tuple.if_index = if_index_to as core::ffi::c_ulong;
            }
            self.is_1rtt_received = true;
            if let Some(policy) =
                crate::internal::SPIN_FUNCTION_TABLE.get(self.spin_policy as usize)
            {
                policy.incoming(self, &mut path, ph);
            }

            let mut ret = self.decode_frames(
                &mut path,
                payload,
                payload.len(),
                received_data,
                ph.epoch as i32,
                addr_from,
                addr_to,
                ph.packet_number_full,
                path_is_not_allocated,
                current_time,
            );

            let mut recompute_ack_frequency = None;
            if ret == 0 {
                path.received = path.received.saturating_add(
                    (ph.offset as u64)
                        .saturating_add(ph.payload_length as u64)
                        .saturating_add(
                            self.get_checksum_length(crate::internal::Epoch::OneRtt) as u64
                        ),
                );
                if path.receive_rate_epoch == 0 {
                    path.received_prior = path.received;
                    path.receive_rate_epoch = current_time.ticks();
                } else {
                    let delta = current_time.ticks().saturating_sub(path.receive_rate_epoch);
                    if delta > path.smoothed_rtt.ticks()
                        && delta > crate::internal::BANDWIDTH_TIME_INTERVAL_MIN
                    {
                        path.receive_rate_estimate = crate::utils::rate_from_bytes(
                            path.received.saturating_sub(path.received_prior),
                            delta,
                        );
                        path.received_prior = path.received;
                        path.receive_rate_epoch = current_time.ticks();
                        if path.receive_rate_estimate > path.receive_rate_max {
                            path.receive_rate_max = path.receive_rate_estimate;
                            if path_id == 0 && !self.is_ack_frequency_negotiated {
                                recompute_ack_frequency =
                                    Some((path.rtt_min, path.receive_rate_max));
                            }
                        }
                    }
                }
            }

            self.paths.insert(path_id, path);

            if let Some((rtt_min, receive_rate_max)) = recompute_ack_frequency {
                let mut ack_gap = self.ack_gap_remote;
                let mut ack_delay = self.ack_delay_remote.ticks();
                self.compute_ack_gap_and_delay(
                    rtt_min,
                    crate::internal::ACK_DELAY_MIN.ticks(),
                    receive_rate_max,
                    &mut ack_gap,
                    &mut ack_delay,
                );
                self.ack_gap_remote = ack_gap;
                self.ack_delay_remote = Duration::from_ticks(ack_delay);
            }

            if ret == 0 {
                let (tls_ret, _) = self.process_tls_stream_status(current_time);
                ret = tls_ret;
            }

            if ret == 0 && self.is_still_logging() {
                crate::logger::Log::cc_dump(self, current_time);
            }

            ret
        }
    }
```

## `picoquic/paths.c:picoquic_prepare_path_challenge_frames`
* Phase 4C status: `suspect`
* Phase 4C rationale: C always delegates using path_x->first_tuple, while Rust returns Some(bytes) without delegating when the tuple list is empty.
* C source: `picoquic/paths.c:140-148`
* C signature: `uint8_t * picoquic_prepare_path_challenge_frames(picoquic_cnx_t *, picoquic_path_t *, uint8_t *, uint8_t *, int *, int *, int *, uint64_t, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:4464-4488`
* Rust item: `prepare_path_challenge_frames`

### C body
```c
{
    return picoquic_prepare_tuple_challenge_frames(cnx, path_x, path_x->first_tuple,
        bytes_next, bytes_max, more_data, is_pure_ack, is_challenge_padding_needed,
        current_time, next_wake_time);
}
```

### Rust body
```rust
    ) -> Option<&'a mut [u8]> {
        if path_x.tuples.is_empty() {
            Some(bytes)
        } else {
            self.prepare_tuple_challenge_frames(
                path_x,
                0,
                bytes,
                more_data,
                is_pure_ack,
                is_challenge_padding_needed,
                current_time,
                next_wake_time,
            )
        }
    }
```

## `picoquic/picoquic_lb.c:picoquic_lb_compat_cid_verify`
* Phase 4C status: `suspect`
* Phase 4C rationale: C visibly checks CID length before method verification and returns UINT64_MAX on failure; Rust delegates to ctx.verify without a visible length check and returns Option.
* C source: `picoquic/picoquic_lb.c:207-234`
* C signature: `uint64_t picoquic_lb_compat_cid_verify(void *, const picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/internal.rs:16145-16154`
* Rust item: `lb_verify_cid`

### C body
```c
{
    picoquic_load_balancer_cid_context_t* lb_ctx = (picoquic_load_balancer_cid_context_t*)cnx_id_cb_data;
    uint64_t server_id64;

    if (cnx_id->id_len != lb_ctx->connection_id_length) {
        server_id64 = UINT64_MAX;
    }
    else {
        switch (lb_ctx->method) {
        case picoquic_load_balancer_cid_clear:
            server_id64 = picoquic_lb_compat_cid_verify_clear(lb_ctx, cnx_id);
            break;
        case picoquic_load_balancer_cid_stream_cipher:
            server_id64 = picoquic_lb_compat_cid_verify_stream_cipher(lb_ctx, cnx_id);
            break;
        case picoquic_load_balancer_cid_block_cipher:
            server_id64 = picoquic_lb_compat_cid_verify_block_cipher(lb_ctx, cnx_id);
            break;
        default:
            /* Error, unknown method */
            server_id64 = UINT64_MAX;
            break;
        }
    }

    return server_id64;
}
```

### Rust body
```rust
    pub fn lb_verify_cid(&mut self, cid: &ConnectionId) -> Option<u64> {
        let mut ctx_box = self.connection_id_callback_ctx.take()?;
        let result = if let Some(ctx) = ctx_box.downcast_mut::<crate::lb::ConnectionIdContext>() {
            ctx.verify(cid)
        } else {
            None
        };
        self.connection_id_callback_ctx = Some(ctx_box);
        result
    }
```
