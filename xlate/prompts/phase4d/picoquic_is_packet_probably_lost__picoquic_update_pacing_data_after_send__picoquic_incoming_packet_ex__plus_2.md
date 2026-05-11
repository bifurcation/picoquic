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

## `picoquic/loss_recovery.c:picoquic_is_packet_probably_lost`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust has an extra early path-token failure branch that sets next_retransmit_time to current_time and returns true; no analogous body-visible branch exists in C.
* C source: `picoquic/loss_recovery.c:535-644`
* C signature: `int picoquic_is_packet_probably_lost(picoquic_cnx_t *, picoquic_packet_t *, uint64_t, uint64_t *, int *)`
* Rust source: `rs/fq/src/internal.rs:11222-11348`
* Rust item: `is_packet_probably_lost`

### C body
```c
{
    uint64_t retransmit_time = UINT64_MAX;
    int64_t delta_seq = 0;
    int64_t delta_sent = 0;
    uint64_t rack_timer_min;
    int is_probably_lost = 0;

    *is_timer_expired = 0;

    if (old_p->ptype == picoquic_packet_0rtt_protected && !cnx->zero_rtt_data_accepted) {
        /* Zero RTT data was not accepted by the peer, the packets are considered lost */
        retransmit_time = current_time;
        is_probably_lost = 1;
    }
    else if (old_p->ptype == picoquic_packet_0rtt_protected && cnx->cnx_state != picoquic_state_ready &&
        cnx->cnx_state != picoquic_state_client_ready_start) {
        /* Set the retransmit time ahead of current time since the connection is not ready */
        retransmit_time = current_time + old_p->send_path->smoothed_rtt + PICOQUIC_RACK_DELAY;
    }
    else {
        picoquic_packet_context_t* pkt_ctx = (cnx->is_multipath_enabled && old_p->pc == picoquic_packet_context_application) ?
            &old_p->send_path->pkt_ctx : &cnx->pkt_ctx[old_p->pc];
        delta_seq = pkt_ctx->highest_acknowledged - old_p->sequence_number;

        if (delta_seq >= 3) {
            /* Last acknowledged packet is ways ahead. That means this packet
            * is most probably lost.
            */
            retransmit_time = current_time;
            is_probably_lost = 1;
        }
        else if (delta_seq > 0) {
            /* Set a timer relative to that last packet */
            int64_t rack_delay = (old_p->send_path->smoothed_rtt >> 2);
            delta_sent = pkt_ctx->latest_time_acknowledged - old_p->send_time;
            if (rack_delay > PICOQUIC_RACK_DELAY / 2) {
                rack_delay = PICOQUIC_RACK_DELAY / 2;
            }
            retransmit_time = old_p->send_time + old_p->send_path->retransmit_timer;
            rack_timer_min = pkt_ctx->highest_acknowledged_time + rack_delay
                - delta_sent + cnx->remote_parameters.max_ack_delay;
            if (retransmit_time > rack_timer_min) {
                retransmit_time = rack_timer_min;
            }
            if (retransmit_time <= current_time || old_p->is_ack_trap) {
                is_probably_lost = 1;
            }
        }
    }
    if (!is_probably_lost) {
        /* Find the last packet in the queue, which may be this one.
        * Compute a timer from the time this last packet was sent.
        * If the timer has elapsed, this packet should be resent,
        * in a timer based manner. If not, set the timer to
        * the specified value. */
        uint64_t retransmit_time_timer;
        picoquic_packet_t* last_packet = picoquic_get_last_packet(cnx, old_p->send_path, old_p->pc);

        if (last_packet == NULL) {
            last_packet = old_p;
        }
        retransmit_time_timer = last_packet->send_time + picoquic_current_retransmit_timer(cnx, old_p->send_path);

        if (current_time >= retransmit_time_timer) {
            if (old_p->send_path->path_is_demoted) {
                /* if the path is demoted, treat this as a simple loss */
                is_probably_lost = 1;
            }
            else {
                /* Do not set the "probably lost" condition, because timers are unreliable */
                *is_timer_expired = 1;
            }
        }
        else if (old_p->send_path->nb_retransmit == 0) {
            /* RACK has failure modes if the sender keeps adding small packets to the
             * retransmit queue. This may push the send time of the "last" packet
             * beyond a reasonable value.
             * In that case, we pick a safe timer based retransmit.
             * The "timer" condition will have consequences on congestion control;
             * we only set it if the packet is ack eliciting.
             */
            uint64_t alt_retransmit_timer = old_p->send_time + 2*picoquic_current_retransmit_timer(cnx, old_p->send_path);

            if (alt_retransmit_timer < last_packet->send_time) {
                retransmit_time_timer = alt_retransmit_timer;
                if (current_time >= retransmit_time_timer) {
                    if (picoquic_is_packet_ack_eliciting(old_p))
                    {
                        *is_timer_expired = 1;
                    }
                    else {
                        is_probably_lost = 1;
                    }
                }
            }
        }

        if (retransmit_time_timer < retransmit_time) {
            retransmit_time = retransmit_time_timer;
        }
    }
    if (*next_retransmit_time > retransmit_time) {
        *next_retransmit_time = retransmit_time;
    }

    return is_probably_lost;
}
```

### Rust body
```rust
    ) -> bool {
        let mut retransmit_time = Instant::from_ticks(u64::MAX);
        let mut is_probably_lost = false;
        *is_timer_expired = false;

        let path_idx = old_p
            .send_path
            .and_then(|send_path| self.path_index_from_token(send_path));
        let Some(path_idx) = path_idx else {
            *next_retransmit_time = current_time;
            return true;
        };
        let path = &self.paths[path_idx];

        if old_p.packet_type == PacketType::ZeroRttProtected && !self.zero_rtt_data_accepted {
            retransmit_time = current_time;
            is_probably_lost = true;
        } else if old_p.packet_type == PacketType::ZeroRttProtected
            && self.connection_state != State::Ready
            && self.connection_state != State::ClientReadyStart
        {
            retransmit_time = Instant::from_ticks(
                current_time
                    .ticks()
                    .saturating_add(path.smoothed_rtt.ticks())
                    .saturating_add(RACK_DELAY.ticks()),
            );
        } else {
            let selection = self.packet_context_for_packet(old_p);
            let pkt_ctx = self.packet_context_state(selection);
            let delta_seq = pkt_ctx
                .highest_acknowledged
                .wrapping_sub(old_p.sequence_number) as i64;

            if delta_seq >= 3 {
                retransmit_time = current_time;
                is_probably_lost = true;
            } else if delta_seq > 0 {
                let mut rack_delay = (path.smoothed_rtt.ticks() >> 2) as i64;
                if rack_delay > (RACK_DELAY.ticks() / 2) as i64 {
                    rack_delay = (RACK_DELAY.ticks() / 2) as i64;
                }
                let delta_sent = pkt_ctx
                    .latest_time_acknowledged
                    .ticks()
                    .wrapping_sub(old_p.send_time.ticks()) as i64;
                retransmit_time = Instant::from_ticks(
                    old_p
                        .send_time
                        .ticks()
                        .saturating_add(path.retransmit_timer.ticks()),
                );
                let mut rack_timer_min = pkt_ctx
                    .highest_acknowledged_time
                    .ticks()
                    .saturating_add(rack_delay.max(0) as u64);
                if delta_sent >= 0 {
                    rack_timer_min = rack_timer_min.saturating_sub(delta_sent as u64);
                } else {
                    rack_timer_min = rack_timer_min.saturating_add((-delta_sent) as u64);
                }
                rack_timer_min =
                    rack_timer_min.saturating_add(self.remote_parameters.max_ack_delay as u64);
                if retransmit_time.ticks() > rack_timer_min {
                    retransmit_time = Instant::from_ticks(rack_timer_min);
                }
                if retransmit_time <= current_time || old_p.is_ack_trap {
                    is_probably_lost = true;
                }
            }
        }

        if !is_probably_lost {
            let selection = self.packet_context_for_packet(old_p);
            let last_packet = self
                .packet_context_state(selection)
                .pending
                .values()
                .next_back()
                .and_then(|token| self.queued_packets.get(*token))
                .map(PacketRetransmitSnapshot::from)
                .unwrap_or(*old_p);
            let current_timer = self.current_retransmit_timer_ticks_for_path(path);
            let mut retransmit_time_timer =
                Instant::from_ticks(last_packet.send_time.ticks().saturating_add(current_timer));

            if current_time >= retransmit_time_timer {
                if path.path_is_demoted {
                    is_probably_lost = true;
                } else {
                    *is_timer_expired = true;
                }
            } else if path.nb_retransmit == 0 {
                let alt_retransmit_timer = Instant::from_ticks(
                    old_p
                        .send_time
                        .ticks()
                        .saturating_add(2u64.saturating_mul(current_timer)),
                );
                if alt_retransmit_timer < last_packet.send_time {
                    retransmit_time_timer = alt_retransmit_timer;
                    if current_time >= retransmit_time_timer {
                        if packet_is_ack_eliciting_from_snapshot(old_p) {
                            *is_timer_expired = true;
                        } else {
                            is_probably_lost = true;
                        }
                    }
                }
            }

            if retransmit_time_timer < retransmit_time {
                retransmit_time = retransmit_time_timer;
            }
        }

        if *next_retransmit_time > retransmit_time {
            *next_retransmit_time = retransmit_time;
        }
        is_probably_lost
    }
```

## `picoquic/pacing.c:picoquic_update_pacing_data_after_send`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both update the bucket and subtract a rounded packet time, but Rust adds a send_mtu > 0 guard and clamps/casts through u128/i64, which is body-visibly different from C's unsigned arithmetic.
* C source: `picoquic/pacing.c:235-245`
* C signature: `void picoquic_update_pacing_data_after_send(picoquic_pacing_t *, size_t, size_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:6060-6068`
* Rust item: `update_after_send`

### C body
```c
{
    uint64_t packet_time_nanosec;

    picoquic_update_pacing_bucket(pacing, current_time);
    packet_time_nanosec = ((pacing->packet_time_nanosec * (uint64_t)length) + (send_mtu - 1)) / send_mtu;
    pacing->bucket_nanosec -= packet_time_nanosec;
}
```

### Rust body
```rust
    pub fn update_after_send(&mut self, length: usize, send_mtu: usize, current_time: Instant) {
        // C: picoquic_update_pacing_data_after_send
        self.update_bucket(current_time);
        if send_mtu > 0 {
            let packet_time_nanosec = (self.packet_time_nanosec.max(0) as u128 * length as u128)
                .div_ceil(send_mtu as u128);
            self.bucket_nanosec -= packet_time_nanosec.min(i64::MAX as u128) as i64;
        }
    }
```

## `picoquic/packet.c:picoquic_incoming_packet_ex`
* Phase 4C status: `suspect`
* Phase 4C rationale: C resets ret to 0 when incoming_segment returns an error before breaking; Rust breaks on nonzero ret but still returns Ok(first_cnx...), discarding the segment error via Result success.
* C source: `picoquic/packet.c:2389-2430`
* C signature: `int picoquic_incoming_packet_ex(picoquic_quic_t *, uint8_t *, size_t, struct sockaddr *, struct sockaddr *, int, unsigned char, picoquic_cnx_t **, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:3289-3338`
* Rust item: `incoming_packet_ex`

### C body
```c
{
    size_t consumed_index = 0;
    int ret = 0;
    picoquic_connection_id_t previous_destid = picoquic_null_connection_id;
    PICOQUIC_THREAD_CHECK(quic);

    while (consumed_index < packet_length) {
        size_t consumed = 0;

        ret = picoquic_incoming_segment(quic, bytes + consumed_index, 
            packet_length - consumed_index, packet_length,
            &consumed, addr_from, addr_to, if_index_to, received_ecn, current_time, current_time,
            &previous_destid, first_cnx);

        if (ret == 0) {
            consumed_index += consumed;
            if (consumed == 0) {
                DBG_PRINTF("%s", "Receive bug, ret = 0 && consumed = 0\n");
                break;
            }
        } else {
            ret = 0;
            break;
        }
    }

    if (*first_cnx != NULL && packet_length > (*first_cnx)->max_mtu_received) {
        (*first_cnx)->max_mtu_received = packet_length;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<Option<&mut Connection>, Error> {
        let packet_length = bytes.len();
        let mut consumed_index = 0usize;
        let mut previous_dest_id = ConnectionId::default();
        let mut first_cnx = None;

        while consumed_index < packet_length {
            let mut consumed = 0usize;
            let ret = self.incoming_segment(
                &mut bytes[consumed_index..],
                packet_length - consumed_index,
                packet_length,
                &mut consumed,
                addr_from,
                addr_to,
                if_index_to,
                received_ecn,
                current_time,
                current_time,
                &mut previous_dest_id,
                &mut first_cnx,
            );

            if ret == 0 {
                consumed_index = consumed_index.saturating_add(consumed);
                if consumed == 0 {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(token) = first_cnx
            && let Some(cnx) = self.connections.get_mut(token)
            && packet_length > cnx.max_mtu_received
        {
            cnx.max_mtu_received = packet_length;
        }

        Ok(first_cnx.and_then(|token| self.connections.get_mut(token)))
    }
```

## `picoquic/performance_log.c:picoquic_perflog_record`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust records and pushes the item but lacks the C body's conditional save call when this is the last connection in the QUIC list.
* C source: `picoquic/performance_log.c:127-210`
* C signature: `int picoquic_perflog_record(picoquic_cnx_t *, picoquic_performance_log_ctx_t *)`
* Rust source: `rs/fq/src/performance_log.rs:212-282`
* Rust item: `record`

### C body
```c
{
    int ret = 0;
    picoquic_performance_log_item_t* perflog_item = (picoquic_performance_log_item_t*)
        malloc(sizeof(picoquic_performance_log_item_t));

    if (perflog_item == NULL) {
        ret = -1;
    }
    else {
        uint64_t start_time = picoquic_get_cnx_start_time(cnx);
        uint64_t close_time = picoquic_get_quic_time(cnx->quic);
        uint64_t duration_usec = close_time - start_time;
        memset(perflog_item, 0, sizeof(picoquic_performance_log_item_t));
        /* Compute the key performance metrics */
        perflog_item->duration_sec = ((double)duration_usec) / 1000000.0;
        if (perflog_item->duration_sec > 0) {
            perflog_item->data_sent = picoquic_get_data_sent(cnx);
            perflog_item->data_received = picoquic_get_data_received(cnx);
            perflog_item->send_mbps = ((double)perflog_item->data_sent) * 8.0 / ((double)duration_usec);
            perflog_item->recv_mbps = ((double)perflog_item->data_received) * 8.0 / ((double)duration_usec);
            /* TODO: nb streams.
            printf("Nb_transactions: %" PRIu64"\n", quicperf_ctx->nb_streams);
            printf("TPS: %f\n", ((double)quicperf_ctx->nb_streams) / duration_sec);
            */
        }
        /* Store identification data */
        perflog_item->alpn = picoquic_string_duplicate(cnx->alpn);
        perflog_item->quic_version = (cnx->version_index >= 0) ?
            picoquic_supported_versions[cnx->version_index].version : 0;
        perflog_item->cnxid = picoquic_get_logging_cnxid(cnx);
        perflog_item->cnx_time_64 = start_time;
        /* Store additional parameters */
        perflog_item->nb_values = PICOQUIC_PERF_LOG_MAX_ITEMS;
        perflog_item->v[picoquic_perflog_is_client] = cnx->client_mode;
        perflog_item->v[picoquic_perflog_nb_packets_received] = cnx->nb_packets_received;
        perflog_item->v[picoquic_perflog_nb_trains_sent] = cnx->nb_trains_sent;
        perflog_item->v[picoquic_perflog_nb_trains_short] = cnx->nb_trains_short;
        perflog_item->v[picoquic_perflog_nb_trains_blocked_cwin] = cnx->nb_trains_blocked_cwin;
        perflog_item->v[picoquic_perflog_nb_trains_blocked_pacing] = cnx->nb_trains_blocked_pacing;
        perflog_item->v[picoquic_perflog_nb_trains_blocked_others] = cnx->nb_trains_blocked_others;
        perflog_item->v[picoquic_perflog_nb_packets_sent] = cnx->nb_packets_sent;
        perflog_item->v[picoquic_perflog_nb_retransmission_total] = cnx->nb_retransmission_total;
        perflog_item->v[picoquic_perflog_nb_spurious] = cnx->nb_spurious;
        perflog_item->v[picoquic_perflog_delayed_ack_option] = cnx->is_ack_frequency_negotiated;
        perflog_item->v[picoquic_perflog_min_ack_delay_remote] = cnx->min_ack_delay_remote;
        perflog_item->v[picoquic_perflog_max_ack_delay_remote] = cnx->max_ack_delay_remote;
        perflog_item->v[picoquic_perflog_max_ack_gap_remote] = cnx->max_ack_gap_remote;
        perflog_item->v[picoquic_perflog_min_ack_delay_local] = cnx->min_ack_delay_local;
        perflog_item->v[picoquic_perflog_max_ack_delay_local] = cnx->max_ack_delay_local;
        perflog_item->v[picoquic_perflog_max_ack_gap_local] = cnx->max_ack_gap_local;
        perflog_item->v[picoquic_perflog_max_mtu_sent] = cnx->max_mtu_sent;
        perflog_item->v[picoquic_perflog_max_mtu_received] = cnx->max_mtu_received;
        perflog_item->v[picoquic_perflog_zero_rtt] = (cnx->nb_zero_rtt_received > 0) || (cnx->nb_zero_rtt_acked > 0);
        if (cnx->path != NULL && cnx->path[0] != NULL) {
            perflog_item->v[picoquic_perflog_srtt] = cnx->path[0]->smoothed_rtt;
            perflog_item->v[picoquic_perflog_minrtt] = cnx->path[0]->rtt_min;
            perflog_item->v[picoquic_perflog_cwin] = cnx->path[0]->cwin;
            perflog_item->v[picoquic_perflog_bwe_max] = cnx->path[0]->bandwidth_estimate_max;
            perflog_item->v[picoquic_perflog_pacing_quantum_max] = cnx->path[0]->pacing.quantum_max;
            perflog_item->v[picoquic_perflog_pacing_rate] = cnx->path[0]->pacing.rate_max;
        }
        if (cnx->congestion_alg != NULL) {
            perflog_item->v[picoquic_perflog_ccalgo] = cnx->congestion_alg->congestion_algorithm_number;
        }
        
        if (perflog_ctx->first == NULL) {
            perflog_ctx->first = perflog_item;
        }

        if (perflog_ctx->last == NULL) {
            perflog_ctx->last = perflog_item;
        }
        else {
            perflog_ctx->last->next = perflog_item;
        }

        if (cnx->quic->cnx_list == cnx && cnx->quic->cnx_last == cnx) {
            ret = picoquic_perflog_save(perflog_ctx);
        }
    }

    return ret;
}
```

### Rust body
```rust
    fn record(&mut self, connection: &Connection) {
        let start_time = connection.start_time.ticks();
        let close_time = connection.quic_time().ticks();
        let duration_usec = close_time.saturating_sub(start_time);
        let duration_sec = (duration_usec as f64) / 1_000_000.0;

        let mut item = PerflogItem {
            duration_sec,
            send_mbps: 0.0,
            recv_mbps: 0.0,
            data_sent: 0,
            data_received: 0,
            quic_version: 0,
            alpn: None,
            cnxid: connection.logging_connection_id(),
            cnx_time_64: start_time,
            v: [0; PERF_LOG_MAX_ITEMS],
        };

        if duration_usec > 0 {
            item.data_sent = connection.data_sent;
            item.data_received = connection.data_received;
            item.send_mbps = (connection.data_sent as f64) * 8.0 / (duration_usec as f64);
            item.recv_mbps = (connection.data_received as f64) * 8.0 / (duration_usec as f64);
        }

        item.alpn = connection.alpn.clone();
        item.quic_version = if connection.version_index >= 0 {
            connection.version_number()
        } else {
            0
        };

        let v = &mut item.v;
        v[PerflogColumn::IsClient as usize] = connection.client_mode as u64;
        v[PerflogColumn::NbPacketsReceived as usize] = connection.nb_packets_received;
        v[PerflogColumn::NbTrainsSent as usize] = connection.nb_trains_sent;
        v[PerflogColumn::NbTrainsShort as usize] = connection.nb_trains_short;
        v[PerflogColumn::NbTrainsBlockedCwin as usize] = connection.nb_trains_blocked_cwin;
        v[PerflogColumn::NbTrainsBlockedPacing as usize] = connection.nb_trains_blocked_pacing;
        v[PerflogColumn::NbTrainsBlockedOthers as usize] = connection.nb_trains_blocked_others;
        v[PerflogColumn::NbPacketsSent as usize] = connection.nb_packets_sent;
        v[PerflogColumn::NbRetransmissionTotal as usize] = connection.nb_retransmission_total;
        v[PerflogColumn::NbSpurious as usize] = connection.nb_spurious;
        v[PerflogColumn::DelayedAckOption as usize] = connection.is_ack_frequency_negotiated as u64;
        v[PerflogColumn::MinAckDelayRemote as usize] = connection.min_ack_delay_remote.ticks();
        v[PerflogColumn::MaxAckDelayRemote as usize] = connection.max_ack_delay_remote.ticks();
        v[PerflogColumn::MaxAckGapRemote as usize] = connection.max_ack_gap_remote;
        v[PerflogColumn::MinAckDelayLocal as usize] = connection.min_ack_delay_local.ticks();
        v[PerflogColumn::MaxAckDelayLocal as usize] = connection.max_ack_delay_local.ticks();
        v[PerflogColumn::MaxAckGapLocal as usize] = connection.max_ack_gap_local;
        v[PerflogColumn::MaxMtuSent as usize] = connection.max_mtu_sent as u64;
        v[PerflogColumn::MaxMtuReceived as usize] = connection.max_mtu_received as u64;
        v[PerflogColumn::ZeroRtt as usize] =
            (connection.nb_zero_rtt_received > 0 || connection.nb_zero_rtt_acked > 0) as u64;

        if let Some(path) = connection.paths.first() {
            v[PerflogColumn::Srtt as usize] = path.smoothed_rtt.ticks();
            v[PerflogColumn::Minrtt as usize] = path.rtt_min.ticks();
            v[PerflogColumn::Cwin as usize] = path.cwin;
            v[PerflogColumn::BweMax as usize] = path.bandwidth_estimate_max;
            v[PerflogColumn::PacingQuantumMax as usize] = path.pacing.quantum_max;
            v[PerflogColumn::PacingRate as usize] = path.pacing.rate_max;
        }

        if let Some(alg) = connection.congestion_alg {
            v[PerflogColumn::Ccalgo as usize] = alg.congestion_algorithm_number as u64;
        }

        self.items.push(item);
    }
```

## `picoquic/picoquic_ptls_minicrypto.c:set_minicrypto_private_key_from_key_file`
* Phase 4C status: `suspect`
* Phase 4C rationale: C directly delegates to ptls_minicrypto_load_private_key with ctx and keypem; Rust only reads the file and checks for private-key marker text, with no visible context update.
* C source: `picoquic/picoquic_ptls_minicrypto.c:35-38`
* C signature: `int set_minicrypto_private_key_from_key_file(const char *, ptls_context_t *)`
* Rust source: `rs/fq/src/tls_api.rs:2616-2623`
* Rust item: `set_minicrypto_private_key_from_key_file`

### C body
```c
{
    return ptls_minicrypto_load_private_key(ctx, keypem);
}
```

### Rust body
```rust
pub fn set_minicrypto_private_key_from_key_file(keypem: &str) -> Result<(), Error> {
    let contents = std::fs::read_to_string(keypem).map_err(|_| Error::NoSuchFile)?;
    if contents.contains("-----BEGIN ") && contents.contains("PRIVATE KEY-----") {
        Ok(())
    } else {
        Err(Error::InvalidFile)
    }
}
```
