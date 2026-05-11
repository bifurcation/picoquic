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

## `picoquic/bbr.c:BBRSetOptions`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust unconditionally enables and parses experiment flags, while the C body gates those parts under BBRExperiment; also Rust sets quantum_ratio to 0 for a trailing Q whereas C's outer loop would not assign when no character follows Q.
* C source: `picoquic/bbr.c:441-556`
* C signature: `void BBRSetOptions(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:855-917`
* Rust item: `set_options`

### C body
```c
{
    const char* x = bbr_state->option_string;
#ifdef BBRExperiment
    bbr_state->exp_flags.do_early_exit = 1;
    bbr_state->exp_flags.do_rapid_start = 1;
    bbr_state->exp_flags.do_handle_suspension = 1;
    bbr_state->exp_flags.do_control_lost = 1;
    bbr_state->exp_flags.do_exit_probeBW_up_on_delay = 1;
    bbr_state->exp_flags.do_enter_probeBW_after_limited = 1;
#endif

    if (x != NULL) {
        char c;
        while ((c = *x) != 0) {
            x++;
            switch (c) {
#ifdef BBRExperiment
            case 'E':
                bbr_state->exp_flags.do_early_exit = 0;
                break;
            case 'R':
                bbr_state->exp_flags.do_rapid_start = 0;
                break;
            case 'H':
                bbr_state->exp_flags.do_handle_suspension = 0;
                break;
            case 'L':
                bbr_state->exp_flags.do_control_lost = 0;
                break;
            case 'D':
                bbr_state->exp_flags.do_exit_probeBW_up_on_delay = 0;
                break;
            case 'A':
                bbr_state->exp_flags.do_enter_probeBW_after_limited = 0;
                break;
#endif
            case 'T': {
                /* Reading digits into an uint64_t  */
                uint64_t u = 0;
                while ((c = *x) != 0) {
                    if (c >= '0' && c <= '9') {
                        u *= 10;
                        u += c - '0';
                        x++;
                    }
                    else {
                        break;
                    }
                }
                bbr_state->wifi_shadow_rtt = u;
                break;
            }
            case 'Q': {
                /* Reading digits ad one dot into a double  */
                while ((c = *x) != 0) {
                    double d = 0;
                    double div = 1.0;
                    int dotted = 0;
                    while ((c = *x) != 0) {
                        if (c >= '0' && c <= '9') {
                            if (!dotted) {
                                d *= 10;
                                d += c - '0';
                            }
                            else {
                                div /= 10.0;
                                d += div * (c - '0');
                            }
                            x++;
                        }
                        else if (c == '.') {
                            if (dotted) {
                                break;
                            }
                            else {
                                dotted = 1;
                                x++;
                            }
                        }
                        else {
                            break;
                        }
                    }
                    bbr_state->quantum_ratio = d;
                    break;
                }
            }
            case ':':
                /* Ignore */
                break;
            default:
                break;
            }
        }
    }
}
```

### Rust body
```rust
    pub fn set_options(&mut self) {
        // BBRExperiment is always on.
        self.exp_flags.do_early_exit = true;
        self.exp_flags.do_rapid_start = true;
        self.exp_flags.do_handle_suspension = true;
        self.exp_flags.do_control_lost = true;
        self.exp_flags.do_exit_probe_bw_up_on_delay = true;
        self.exp_flags.do_enter_probe_bw_after_limited = true;

        let s = match self.option_string.clone() {
            Some(s) => s,
            None => return,
        };
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                'E' => self.exp_flags.do_early_exit = false,
                'R' => self.exp_flags.do_rapid_start = false,
                'H' => self.exp_flags.do_handle_suspension = false,
                'L' => self.exp_flags.do_control_lost = false,
                'D' => self.exp_flags.do_exit_probe_bw_up_on_delay = false,
                'A' => self.exp_flags.do_enter_probe_bw_after_limited = false,
                'T' => {
                    let mut u: u64 = 0;
                    while let Some(&d) = chars.peek() {
                        if d.is_ascii_digit() {
                            u = u * 10 + (d as u64 - b'0' as u64);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    self.wifi_shadow_rtt = u;
                }
                'Q' => {
                    let mut d: f64 = 0.0;
                    let mut div: f64 = 1.0;
                    let mut dotted = false;
                    while let Some(&ch) = chars.peek() {
                        if ch.is_ascii_digit() {
                            if !dotted {
                                d = d * 10.0 + (ch as u8 - b'0') as f64;
                            } else {
                                div /= 10.0;
                                d += div * (ch as u8 - b'0') as f64;
                            }
                            chars.next();
                        } else if ch == '.' {
                            if dotted {
                                break;
                            }
                            dotted = true;
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    self.quantum_ratio = d;
                }
                _ => {}
            }
        }
    }
```

## `picoquic/bbr1.c:picoquic_bbr1_notify`
* Phase 4C status: `suspect`
* Phase 4C rationale: In the long-RTT hystart test, C uses cnx->path[0]->pacing.packet_time_microsec while Rust uses path_x.pacing.packet_time_microsec; bodies show different packet-time source.
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

## `picoquic/bytestream.c:bytewrite_cid`
* Phase 4C status: `suspect`
* Phase 4C rationale: C attempts both length and buffer writes and ORs the return codes; Rust stops after write_u8 fails, so later side effects can differ after a first-write error.
* C source: `picoquic/bytestream.c:317-322`
* C signature: `int bytewrite_cid(bytestream *, const picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/bytestream.rs:493-496`
* Rust item: `write_cid`

### C body
```c
{
    int ret = bytewrite_int8(s, cid->id_len);
    ret |= bytewrite_buffer(s, cid->id, cid->id_len);
    return ret;
}
```

### Rust body
```rust
    pub fn write_cid(&mut self, cid: &ConnectionId) -> Result<(), Error> {
        self.write_u8(cid.id_len)?;
        self.write_bytes(&cid.id[..cid.id_len as usize])
    }
```

## `picoquic/cc_common.c:picoquic_cc_get_sequence_number`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only returns the selected sequence number, while Rust stores it in round_start_pn and also updates next_round_delivered.
* C source: `picoquic/cc_common.c:27-39`
* C signature: `uint64_t picoquic_cc_get_sequence_number(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/bbr.rs:1041-1048`
* Rust item: `start_round`

### C body
```c
{
    uint64_t sequence_number;

    if (cnx->is_multipath_enabled) {
            sequence_number = path_x->pkt_ctx.send_sequence;
        }
    else {
       sequence_number = cnx->pkt_ctx[picoquic_packet_context_application].send_sequence;
    }

    return sequence_number;
}
```

### Rust body
```rust
    pub(crate) fn start_round(&mut self, connection: &Connection, path_x: &Path) {
        self.round_start_pn = if connection.is_multipath_enabled {
            path_x.pkt_ctx.send_sequence
        } else {
            connection.pkt_ctx[PacketContext::Application as usize].send_sequence
        };
        self.next_round_delivered = path_x.delivered;
    }
```

## `picoquic/config.c:picoquic_config_get_option_name_index`
* Phase 4C status: `suspect`
* Phase 4C rationale: C uses strncmp(s, option_name, l); Rust requires e.name.len() >= l and slices s, which can differ for short option names or non-byte-aligned strings.
* C source: `picoquic/config.c:654-665`
* C signature: `int picoquic_config_get_option_name_index(const char *, size_t)`
* Rust source: `rs/fq/src/config.rs:690-697`
* Rust item: `picoquic_config_get_option_name_index`

### C body
```c
{
    int option_index = -1;

    for (size_t i = 0; i < option_table_size; i++) {
        if (strncmp(s, option_table[i].option_name, l) == 0) {
            option_index = (int)i;
            break;
        }
    }
    return option_index;
}
```

### Rust body
```rust
pub fn picoquic_config_get_option_name_index(s: &str, l: usize) -> i32 {
    let l = l.min(s.len());
    let prefix = &s[..l];
    OPTION_TABLE
        .iter()
        .position(|e| e.name.len() >= l && &e.name[..l] == prefix)
        .map_or(-1, |i| i as i32)
}
```
