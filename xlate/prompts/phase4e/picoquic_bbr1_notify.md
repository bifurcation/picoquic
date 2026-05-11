# Phase 4E repair confirmed translation mismatches

You are repairing Phase 4D `needs_fix` entries.  Phase 4D
already performed deeper classification and concluded that
these Rust translations need repair.

Rules:

* Edit Rust only.  Do not edit C sources.
* Keep edits limited to the owned Rust file(s) for this batch
  unless a directly related helper in `rs/fq/` must change.
* Preserve safe, idiomatic Rust and existing public API shape
  unless the current shape cannot express the C behavior.
* Do not replace code with stubs, placeholders, fabricated
  defaults, or weaker behavior.
* If deeper repair inspection proves Phase 4D was mistaken,
  report outcome `ok` and do not edit source.
* The driver will run a separate read-only re-triage before
  recording any `fixed` or `ok` result as resolved.
* Report `blocked` only with a concrete human-actionable
  reason.

Owned Rust file(s): `rs/fq/src/bbr1.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/bbr1.c:picoquic_bbr1_notify`
* Phase 4C status: `suspect`
* Phase 4C rationale: In the long-RTT hystart test, C uses cnx->path[0]->pacing.packet_time_microsec while Rust uses path_x.pacing.packet_time_microsec; bodies show different packet-time source.
* Phase 4D analysis: Rust uses path_x pacing packet_time in the long-RTT HyStart test, but C uses cnx->path[0]. Comparable Rust congestion translations preserve the connection first-path source. This can change HyStart exit behavior on non-default paths.
* Phase 4D fix note: Use cnx.paths.first().pacing.packet_time_microsec for the long-RTT HyStart packet_time, falling back to path_x only if no first path exists.
* C source: `picoquic/bbr1.c:1191-1319`
* C signature: `void picoquic_bbr1_notify(picoquic_cnx_t *, picoquic_path_t *, picoquic_congestion_notification_t, picoquic_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr1.rs:1203-1336`
* Rust item: `notify`

### C body
```c
{
    picoquic_bbr1_state_t* bbr1_state = (picoquic_bbr1_state_t*)path_x->congestion_alg_state;
    path_x->is_cc_data_updated = 1;

    if (bbr1_state != NULL) {
        switch (notification) {
        case picoquic_congestion_notification_ecn_ec:
            /* Non standard code to react on ECN_EC */
            if (ack_state->lost_packet_number >= bbr1_state->congestion_sequence) {
                picoquic_bbr1_notify_congestion(bbr1_state, cnx, path_x, current_time, 0);
            }
            break;
        case picoquic_congestion_notification_repeat:
        case picoquic_congestion_notification_timeout:
            /* Non standard code to react to high rate of packet loss, or timeout loss */
            if (ack_state->lost_packet_number >= bbr1_state->congestion_sequence &&
                picoquic_cc_hystart_loss_test(&bbr1_state->rtt_filter, notification, ack_state->lost_packet_number, 0.20)) {
                picoquic_bbr1_notify_congestion(bbr1_state, cnx, path_x, current_time,
                    (notification == picoquic_congestion_notification_timeout) ? 1 : 0);
            }
            break;
        case picoquic_congestion_notification_spurious_repeat:
            if (bbr1_state->is_suspended) {
                picoquic_bbr1_suspension_almost_over(bbr1_state, ack_state->lost_packet_number);
            }
            break;
        case picoquic_congestion_notification_acknowledgement:
            /* sum the amount of data acked per packet */
            if (bbr1_state->is_suspended) {
                picoquic_bbr1_suspension_exit(bbr1_state, path_x);
            }
            bbr1_state->bytes_delivered += ack_state->nb_bytes_acknowledged;

            if (bbr1_state->state == picoquic_bbr1_alg_startup && path_x->rtt_min > BBR1_HYSTART_THRESHOLD_RTT) {
                BBR1EnterStartupLongRTT(bbr1_state, path_x);
            }

            if (bbr1_state->state == picoquic_bbr1_alg_startup_long_rtt) {
                if (picoquic_cc_hystart_test(&bbr1_state->rtt_filter, (cnx->is_time_stamp_enabled) ? ack_state->one_way_delay : ack_state->rtt_measurement,
                    cnx->path[0]->pacing.packet_time_microsec, current_time, cnx->is_time_stamp_enabled)) {
                    BBR1ExitStartupLongRtt(bbr1_state, path_x, current_time);
                }
            }

            /* RTT measurements will happen after the bandwidth is estimated */
            if (bbr1_state->state == picoquic_bbr1_alg_startup_long_rtt) {
                uint64_t max_win;
                uint64_t min_win;

                BBR1UpdateBtlBw(bbr1_state, path_x, current_time);
                if (ack_state->rtt_measurement <= bbr1_state->rt_prop) {
                    bbr1_state->rt_prop = ack_state->rtt_measurement;
                    bbr1_state->rt_prop_stamp = current_time;
                }
                if (path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                    path_x->cwin += picoquic_cc_slow_start_increase(path_x, bbr1_state->bytes_delivered);
                }
                bbr1_state->bytes_delivered = 0;

                max_win = PICOQUIC_BYTES_FROM_RATE(bbr1_state->rt_prop, path_x->peak_bandwidth_estimate);
                min_win = max_win /= 2;

                if (path_x->cwin < min_win) {
                    path_x->cwin = min_win;
                }
                else if (path_x->smoothed_rtt > PICOQUIC_TARGET_RENO_RTT) {
                    path_x->pacing.bandwidth_pause = 1;
                }

                picoquic_update_pacing_data(path_x, 1);
            } else {
                BBR1UpdateOnACK(bbr1_state, path_x,
                    ack_state->rtt_measurement, path_x->bytes_in_transit, 0 /* packets_lost */, bbr1_state->bytes_delivered,
                    current_time);
                /* Remember the number in flight before the next ACK -- TODO: update after send instead. */
                bbr1_state->prior_in_flight = path_x->bytes_in_transit;
                /* Reset the number of bytes delivered */
                bbr1_state->bytes_delivered = 0;

                if (bbr1_state->pacing_rate > 0) {
                    /* Set the pacing rate in picoquic sender */
                    picoquic_update_pacing_rate(path_x, bbr1_state->pacing_rate, bbr1_state->send_quantum);
                }
            }
            break;
        case picoquic_congestion_notification_cwin_blocked:
            break;
        case picoquic_congestion_notification_reset:
            picoquic_bbr1_reset(bbr1_state, path_x, current_time);
            break;
        case picoquic_congestion_notification_seed_cwin:
            if (bbr1_state->state == picoquic_bbr1_alg_startup_long_rtt) {
                BBR1ExitStartupSeedBDP(bbr1_state, path_x, ack_state->nb_bytes_acknowledged, current_time);
                picoquic_update_pacing_data(path_x, 1);
            }
            else if (bbr1_state->state == picoquic_bbr1_alg_startup){
                /* If in initial startup phase, do something */
                double seed_bw_estimate = (double)ack_state->nb_bytes_acknowledged;
                uint64_t bwe;
                seed_bw_estimate /= (double)path_x->smoothed_rtt;
                seed_bw_estimate *= 1000000;
                bwe = (uint64_t)seed_bw_estimate*2; /* Hack -- account for div by two in BBR1UpdateBtlBw */
                if (path_x->bandwidth_estimate_max < bwe) {
                    path_x->bandwidth_estimate_max = bwe;
                    BBR1UpdateBtlBw(bbr1_state, path_x, current_time);
                    BBR1SetPacingRate(bbr1_state);
                    if (bbr1_state->pacing_rate > 0) {
                        /* Set the pacing rate in picoquic sender */
                        picoquic_update_pacing_rate(path_x, bbr1_state->pacing_rate, bbr1_state->send_quantum);
                    }
                }
            }
            break;
        default:
            /* ignore */
            break;
        }
    }
}
```

### Rust body
```rust
    ) {
        path_x.is_cc_data_updated = true;

        match notification {
            CongestionNotification::EcnEc
                if ack_state.lost_packet_number >= self.congestion_sequence =>
            {
                self.notify_congestion(cnx, path_x, current_time, false);
            }
            CongestionNotification::Repeat | CongestionNotification::Timeout
                if ack_state.lost_packet_number >= self.congestion_sequence
                    && self.rtt_filter.hystart_loss_test(
                        notification,
                        ack_state.lost_packet_number,
                        0.20,
                    ) =>
            {
                let is_timeout = notification == CongestionNotification::Timeout;
                self.notify_congestion(cnx, path_x, current_time, is_timeout);
            }
            CongestionNotification::SpuriousRepeat if self.is_suspended => {
                self.suspension_almost_over(ack_state.lost_packet_number);
            }
            CongestionNotification::Acknowledgement => {
                if self.is_suspended {
                    self.suspension_exit(path_x);
                }
                self.bytes_delivered += ack_state.nb_bytes_acknowledged;

                if self.state == Bbr1AlgState::Startup
                    && path_x.rtt_min.ticks() > BBR1_HYSTART_THRESHOLD_RTT
                {
                    self.enter_startup_long_rtt(path_x);
                }

                if self.state == Bbr1AlgState::StartupLongRtt {
                    let rtt_meas = if cnx.is_time_stamp_enabled {
                        ack_state.one_way_delay
                    } else {
                        ack_state.rtt_measurement
                    };
                    let packet_time =
                        Instant::from_ticks(path_x.pacing.packet_time_microsec.ticks());
                    if self.rtt_filter.hystart_test(
                        rtt_meas,
                        packet_time,
                        Instant::from_ticks(current_time),
                        cnx.is_time_stamp_enabled,
                    ) {
                        self.exit_startup_long_rtt(path_x, current_time);
                    }
                }

                if self.state == Bbr1AlgState::StartupLongRtt {
                    self.update_btl_bw(path_x, current_time);
                    if ack_state.rtt_measurement.ticks() <= self.rt_prop {
                        self.rt_prop = ack_state.rtt_measurement.ticks();
                        self.rt_prop_stamp = current_time;
                    }
                    if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
                        let increase = path_x.slow_start_increase(self.bytes_delivered);
                        path_x.cwin += increase;
                    }
                    self.bytes_delivered = 0;

                    let max_win = bytes_from_rate(self.rt_prop, path_x.peak_bandwidth_estimate);
                    let min_win = max_win / 2;

                    if path_x.cwin < min_win {
                        path_x.cwin = min_win;
                    } else if path_x.smoothed_rtt > TARGET_RENO_RTT {
                        path_x.pacing.bandwidth_pause = 1;
                    }
                    path_x.update_pacing_data(1);
                } else {
                    let rtt_sample = ack_state.rtt_measurement.ticks();
                    let bytes_in_transit = path_x.bytes_in_transit;
                    let bytes_delivered = self.bytes_delivered;
                    self.update_on_ack(
                        path_x,
                        rtt_sample,
                        bytes_in_transit,
                        0,
                        bytes_delivered,
                        current_time,
                    );
                    self.prior_in_flight = path_x.bytes_in_transit;
                    self.bytes_delivered = 0;
                    if self.pacing_rate > 0.0 {
                        path_x.update_pacing_rate(self.pacing_rate, self.send_quantum);
                    }
                }
            }
            CongestionNotification::CwinBlocked => {}
            CongestionNotification::Reset => {
                self.reset(path_x, current_time);
            }
            CongestionNotification::SeedCwin => {
                if self.state == Bbr1AlgState::StartupLongRtt {
                    self.exit_startup_seed_bdp(
                        path_x,
                        ack_state.nb_bytes_acknowledged,
                        current_time,
                    );
                    path_x.update_pacing_data(1);
                } else if self.state == Bbr1AlgState::Startup {
                    // Estimate bandwidth from the seeded BDP; account for the
                    // div-by-two inside BBR1UpdateBtlBw with *2 (C comment: "Hack").
                    let smoothed_rtt_us = path_x.smoothed_rtt.ticks() as f64;
                    if smoothed_rtt_us > 0.0 {
                        let seed_bw =
                            ack_state.nb_bytes_acknowledged as f64 / smoothed_rtt_us * 1_000_000.0;
                        let bwe = (seed_bw as u64) * 2;
                        if path_x.bandwidth_estimate_max < bwe {
                            path_x.bandwidth_estimate_max = bwe;
                            self.update_btl_bw(path_x, current_time);
                            self.set_pacing_rate();
                            if self.pacing_rate > 0.0 {
                                path_x.update_pacing_rate(self.pacing_rate, self.send_quantum);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
```
