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

## Pair `picoquic/cc_common.c:picoquic_cc_slow_start_increase_ex`
C: `picoquic/cc_common.c:224-233 picoquic_cc_slow_start_increase_ex`
Rust: `rs/fq/src/cc_common.rs:323-416 slow_start_increase_ex`

### C body
```c
{
    if (in_css) {
        /* In consecutive Slow Start. */
        return picoquic_cc_slow_start_increase(path_x, nb_delivered / PICOQUIC_HYSTART_PP_CSS_GROWTH_DIVISOR);
    }

    /* Fallback to traditional Slow Start. */
    return picoquic_cc_slow_start_increase(path_x, nb_delivered); /* nb_delivered; */
}
```

### Rust body
```rust
impl PathCc for Path {
    fn lowest_not_ack(&self) -> u64 {
        // C reads cnx->pkt_ctx[app] for single-path, path->pkt_ctx for multipath.
        // Path has no back-pointer to Connection, so we always use path->pkt_ctx
        // (exact for multipath; conservative approximation for single-path).
        self.pkt_ctx
            .pending
            .keys()
            .next()
            .copied()
            .unwrap_or(self.pkt_ctx.highest_acknowledged + 1)
    }

    fn slow_start_increase(&self, nb_delivered: u64) -> u64 {
        // C body checks cnx->cwin_blocked.  Path has no back-pointer to
        // Connection, so approximate with bytes_in_transit >= cwin, which is
        // the condition that sets cwin_blocked in the C library.
        if self.bytes_in_transit < self.cwin {
            0
        } else {
            nb_delivered
        }
    }

    fn slow_start_increase_ex(&self, nb_delivered: u64, in_css: bool) -> u64 {
        if in_css {
            self.slow_start_increase(nb_delivered / HYSTART_PP_CSS_GROWTH_DIVISOR)
        } else {
            self.slow_start_increase(nb_delivered)
        }
    }

    fn slow_start_increase_ex2(&self, nb_delivered: u64, in_css: bool, prague_alpha: u64) -> u64 {
        if prague_alpha != 0 {
            let delta = if self.smoothed_rtt <= TARGET_RENO_RTT {
                nb_delivered * (1024 - prague_alpha) / 1024
            } else {
                nb_delivered * self.smoothed_rtt.ticks() * (1024 - prague_alpha)
                    / TARGET_RENO_RTT.ticks()
                    / 1024
            };
            self.slow_start_increase_ex(delta, in_css)
        } else {
            self.slow_start_increase_ex(nb_delivered, in_css)
        }
    }

    fn update_target_cwin_estimation(&self) -> u64 {
        // BYTES_FROM_RATE(smoothed_rtt, peak_bandwidth_estimate) = rtt_us * bps / 1_000_000
        let max_win = self.smoothed_rtt.ticks() * self.peak_bandwidth_estimate / 1_000_000;
        let min_win = max_win / 2;
        if min_win > self.cwin {
            min_win
        } else {
            self.cwin
        }
    }

    fn update_cwin_for_long_rtt(&self) -> u64 {
        let rtt_cap = if self.rtt_min > TARGET_SATELLITE_RTT {
            TARGET_SATELLITE_RTT
        } else {
            self.rtt_min
        };
        let min_cwnd =
            (CWIN_INITIAL as f64 * rtt_cap.ticks() as f64 / TARGET_RENO_RTT.ticks() as f64) as u64;
        if min_cwnd > self.cwin {
            min_cwnd
        } else {
            self.cwin
        }
    }
}
```

## Pair `picoquic/cc_common.c:picoquic_cc_increased_window`
C: `picoquic/cc_common.c:291-304 picoquic_cc_increased_window`
Rust: `rs/fq/src/internal.rs:12888-12941 cc_increased_window`

### C body
```c
{
    uint64_t new_window;
    if (cnx->path[0]->rtt_min <= PICOQUIC_TARGET_RENO_RTT) {
        new_window = previous_window * 2;
    }
    else {
        double w = (double)previous_window;
        w /= (double)PICOQUIC_TARGET_RENO_RTT;
        w *= (cnx->path[0]->rtt_min > PICOQUIC_TARGET_SATELLITE_RTT) ? PICOQUIC_TARGET_SATELLITE_RTT : (double)cnx->path[0]->rtt_min;
        new_window = (uint64_t)w;
    }
    return new_window;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut off = 0;
    if connection.max_stream_id_bidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_bidir
        > connection.max_stream_id_bidir_local
    {
        let new_bidir = connection.max_stream_id_bidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_bidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsBidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_bidir).rank()) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.max_stream_id_bidir_local = new_bidir;
        *is_pure_ack = 0;
    }

    if connection.max_stream_id_unidir_local_computed
        + 2 * connection.local_parameters.initial_max_stream_id_unidir
        > connection.max_stream_id_unidir_local
    {
        let new_unidir = connection.max_stream_id_unidir_local
            + 4 * connection.local_parameters.initial_max_stream_id_unidir;
        if bytes.len() <= off {
            *more_data = 1;
            return Some(bytes);
        }
        bytes[off] = crate::frames::FrameType::MaxStreamsUnidir as u8;
        off += 1;
        if !encode_varint_at(bytes, &mut off, crate::stream::StreamId(new_unidir).rank()) {
            *more_data = 1;
            return Some(bytes);
        }
        connection.max_stream_id_unidir_local = new_unidir;
        *is_pure_ack = 0;
    }

    Some(&mut bytes[off..])
}
```

## Pair `picoquic/config.c:config_optval_param_string`
C: `picoquic/config.c:191-200 config_optval_param_string`
Rust: `rs/fq/src/config.rs:575-583 config_optval_param_string`

### C body
```c
{
    if (params == NULL || x < 0 || x >= nb_param) {
        buffer[0] = 0;
        return buffer;
    }
    else {
        return config_optval_string(buffer, buffer_max, params[x].param, params[x].length);
    }
}
```

### Rust body
```rust
fn config_optval_param_string<'a>(buffer: &'a mut [u8], params: &[&str], x: usize) -> &'a str {
    if x >= params.len() {
        if !buffer.is_empty() {
            buffer[0] = 0;
        }
        return "";
    }
    config_optval_string(buffer, params[x].as_bytes())
}
```

## Pair `picoquic/config.c:picoquic_config_option_letters`
C: `picoquic/config.c:555-578 picoquic_config_option_letters`
Rust: `rs/fq/src/tests/config.rs:438-441 config_option_letters`

### C body
```c
{
    size_t l = 0;
    int ret = 0;

    for (size_t i = 0; l + 1 < string_max && i < option_table_size; i++) {
        option_string[l++] = option_table[i].option_letter;
        if (option_table[i].nb_params_required > 0) {
            if (l + 1 < string_max) {
                option_string[l++] = ':';
            }
            else {
                l--;
                ret = -1;
                break;
            }
        }
    }
    option_string[l] = 0;
    if (string_length != NULL) {
        *string_length = l;
    }
    return ret;
}
```

### Rust body
```rust
fn config_option_letters() {
    let expected = "c:k:p:v:o:w:x:rR:s:XS:G:H:P:O:Me:C:i:l:Lb:q:m:n:a:t:zI:d:DQT:N:B:F:VU:0j:W:8J:E:y:K:Z:4:6:h";
    assert_eq!(Config::option_letters(), expected);
}
```

## Pair `picoquic/config.c:picoquic_config_get_option_char_index`
C: `picoquic/config.c:641-652 picoquic_config_get_option_char_index`
Rust: `rs/fq/src/config.rs:678-697 picoquic_config_get_option_char_index`

### C body
```c
{
    int option_index = -1;

    for (size_t i = 0; i < option_table_size; i++) {
        if (option_table[i].option_letter == opt) {
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

## Pair `picoquic/config.c:picoquic_config_command_line`
C: `picoquic/config.c:722-740 picoquic_config_command_line`
Rust: `rs/fq/src/config.rs:1281-1291 command_line`

### C body
```c
{
    int ret = 0;
    int option_index = -1;
    char opt_string[3] = { '-', 0, 0 };

    opt_string[1] = (char)opt;
    option_index = picoquic_config_get_option_char_index(opt);

    if (option_index == -1) {
        fprintf(stderr, "Unknown option: -%c\n", opt);
        ret = -1;
    }
    else {
        ret = picoquic_get_command_line_option_value(option_index, opt_string, p_optind,
            argv, argc, optarg, config);
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let (_, entry) = option_entry_by_letter(opt).ok_or(Error::InvalidArgument)?;
        let params = collect_params(entry, p_optind, argv, optarg)?;
        apply_option(self, entry, &params)
    }
```

## Pair `picoquic/config.c:picoquic_config_clear`
C: `picoquic/config.c:1046-1121 picoquic_config_clear`
Rust: `rs/fq/src/config.rs:1248-1250 clear`

### C body
```c
{
    if (config->solution_dir != NULL) {
        free((void*)config->solution_dir);
    }
    if (config->server_cert_file != NULL) {
        free((void*)config->server_cert_file);
    }
    if (config->server_key_file != NULL) {
        free((void*)config->server_key_file);
    }
    if (config->log_file != NULL) {
        free((void*)config->log_file);
    }
    if (config->bin_dir != NULL) {
        free((void*)config->bin_dir);
    }
    if (config->qlog_dir != NULL) {
        free((void*)config->qlog_dir);
    }
    if (config->performance_log != NULL) {
        free((void*)config->performance_log);
    }
    if (config->cc_algo_id != NULL) {
        free((void*)config->cc_algo_id);
    }
    if (config->cc_algo_option_string != NULL) {
        free((void*)config->cc_algo_option_string);
    }
    if (config->cnx_id_cbdata != NULL) {
        free((void*)config->cnx_id_cbdata);
    }
    if (config->multipath_alt_config != NULL) {
        free((void*)config->multipath_alt_config);
    }
    if (config->www_dir != NULL) {
        free((void*)config->www_dir);
    }
    if (config->ticket_file_name != NULL) {
        free((void*)config->ticket_file_name);
    }
    if (config->token_file_name != NULL) {
        free((void*)config->token_file_name);
    }
    if (config->sni != NULL) {
        free((void*)config->sni);
    }
    if (config->alpn != NULL) {
        free((void*)config->alpn);
    }
    if (config->out_dir != NULL) {
        free((void*)config->out_dir);
    }
    if (config->root_trust_file != NULL) {
        free((void*)config->root_trust_file);
    }
    if (config->ech_key_file != NULL) {
        free((void*)config->ech_key_file);
    }
    if (config->ech_config_file != NULL) {
        free((void*)config->ech_config_file);
    }
    if (config->ech_public_name != NULL) {
        free((void*)config->ech_public_name);
    }
    if (config->ech_target != NULL) {
        free((void*)config->ech_target);
    }
    if (config->preferred_address_v4 != NULL) {
        free((void*)config->preferred_address_v4);
    }
    if (config->preferred_address_v6 != NULL) {
        free((void*)config->preferred_address_v6);
    }
    picoquic_config_init(config);
}
```

### Rust body
```rust
    pub fn clear(&mut self) {
        *self = Config::default();
    }
```

## Pair `picoquic/cubic.c:cubic_W_cubic`
C: `picoquic/cubic.c:121-130 cubic_W_cubic`
Rust: `rs/fq/src/cubic.rs:112-116 w_cubic`

### C body
```c
{
    double delta_t_sec = ((double)(current_time - cubic_state->start_of_epoch) / 1000000.0) - cubic_state->K;
    double W_cubic = (PICOQUIC_CUBIC_C * (delta_t_sec * delta_t_sec * delta_t_sec)) + cubic_state->W_max;

    return W_cubic;
}
```

### Rust body
```rust
    fn w_cubic(&self, current_time: u64) -> f64 {
        let delta_t_sec =
            current_time.wrapping_sub(self.start_of_epoch) as f64 / 1_000_000.0 - self.k;
        CUBIC_C * (delta_t_sec * delta_t_sec * delta_t_sec) + self.w_max
    }
```

## Pair `picoquic/cubic.c:cubic_notify`
C: `picoquic/cubic.c:237-423 cubic_notify`
Rust: `rs/fq/src/cubic.rs:248-417 notify`

### C body
```c
{
    picoquic_cubic_state_t* cubic_state = (picoquic_cubic_state_t*)path_x->congestion_alg_state;
    path_x->is_cc_data_updated = 1;

    if (cubic_state != NULL) {
        switch (notification) {
            /* RTT measurements will happen before acknowledgement is signalled */
            case picoquic_congestion_notification_acknowledgement:
                switch (cubic_state->alg_state) {
                    case picoquic_cubic_alg_slow_start:
                        /* Increase cwin based on bandwidth estimation. */
                        path_x->cwin = picoquic_cc_update_target_cwin_estimation(path_x);

                        if (path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                            //if (path_x->bytes_in_transit > path_x->cwin) {
                                path_x->cwin += picoquic_cc_slow_start_increase_ex(path_x, ack_state->nb_bytes_acknowledged, 0);

                                /* if cnx->cwin exceeds SSTHRESH, exit and go to CA */
                                if (path_x->cwin >= cubic_state->ssthresh) {
                                    cubic_state->W_reno = ((double)path_x->cwin) / 2.0;
                                    path_x->is_ssthresh_initialized = 1;
                                    cubic_enter_avoidance(cubic_state, current_time);
                                }
                            //}
                        }
                        break;
                    /* TODO discuss
                     * picoquic_cubic_alg_recovery is not entered anyway
                     */
                    case picoquic_cubic_alg_recovery:
                        /* exit recovery, move to CA or SS, depending on CWIN */
                        cubic_state->alg_state = picoquic_cubic_alg_slow_start;
                        path_x->cwin += ack_state->nb_bytes_acknowledged;
                        /* if cnx->cwin exceeds SSTHRESH, exit and go to CA */
                        if (path_x->cwin >= cubic_state->ssthresh) {
                            cubic_state->alg_state = picoquic_cubic_alg_congestion_avoidance;
                        }
                        break;
                    case picoquic_cubic_alg_congestion_avoidance:
                        if (path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                            double W_cubic;
                            uint64_t win_cubic;
                            /* Protection against limited senders. */
                            if (cubic_state->start_of_epoch < path_x->last_sender_limited_time) {
                                cubic_state->start_of_epoch = path_x->last_sender_limited_time;
                            }
                            /* Compute the cubic formula */
                            W_cubic = cubic_W_cubic(cubic_state, current_time);
                            win_cubic = (uint64_t)(W_cubic * (double)path_x->send_mtu);
                            /* Also compute the Reno formula */
                            cubic_state->W_reno += ((double)ack_state->nb_bytes_acknowledged) * ((double)path_x->send_mtu) / cubic_state->W_reno;

                            /* Pick the largest */
                            if ((double)win_cubic > cubic_state->W_reno) {
                                /* if cubic is larger than threshold, switch to cubic mode */
                                path_x->cwin = win_cubic;
                            }
                            else {
                                path_x->cwin = (uint64_t)cubic_state->W_reno;
                            }
                        }
                        break;
                }
                break;
            case picoquic_congestion_notification_repeat:
            case picoquic_congestion_notification_timeout:
            case picoquic_congestion_notification_ecn_ec:
                switch (cubic_state->alg_state) {
                    case picoquic_cubic_alg_slow_start:
                        /* For compatibility with Linux-TCP deployments, we implement a filter so
                         * Cubic will only back off after repeated losses, not just after a single loss.
                         */
                        if ((notification == picoquic_congestion_notification_ecn_ec ||
                            picoquic_cc_hystart_loss_test(&cubic_state->rtt_filter, notification, ack_state->lost_packet_number, PICOQUIC_SMOOTHED_LOSS_THRESHOLD)) &&
                            (current_time - cubic_state->start_of_epoch > path_x->smoothed_rtt ||
                                cubic_state->recovery_sequence <= picoquic_cc_get_ack_number(cnx, path_x))) {
                            path_x->is_ssthresh_initialized = 1;
                            cubic_enter_recovery(cnx, path_x, notification, cubic_state, current_time);
                        }
                        break;
                    case picoquic_cubic_alg_recovery:
                    case picoquic_cubic_alg_congestion_avoidance:
                        /* For compatibility with Linux-TCP deployments, we implement a filter so
                         * Cubic will only back off after repeated losses, not just after a single loss.
                         */
                        if (ack_state->lost_packet_number >= cubic_state->recovery_sequence &&
                            (notification == picoquic_congestion_notification_ecn_ec ||
                                picoquic_cc_hystart_loss_test(&cubic_state->rtt_filter, notification, ack_state->lost_packet_number, PICOQUIC_SMOOTHED_LOSS_THRESHOLD))) {
                            /* Re-enter recovery */
                            cubic_enter_recovery(cnx, path_x, notification, cubic_state, current_time);
                        }
                        break;
                }
                break;
            case picoquic_congestion_notification_spurious_repeat:
                /* Reset CWIN based on ssthresh, not based on current value. */
                cubic_correct_spurious(path_x, cubic_state, current_time);
                break;
            case picoquic_congestion_notification_rtt_measurement:
                if (cubic_state->alg_state == picoquic_cubic_alg_slow_start &&
                    cubic_state->ssthresh == UINT64_MAX) {

                    /* HyStart. */
                    /* Using RTT increases as signal to get out of initial slow start */
                    if (picoquic_cc_hystart_test(&cubic_state->rtt_filter, (cnx->is_time_stamp_enabled) ? ack_state->one_way_delay : ack_state->rtt_measurement,
                            cnx->path[0]->pacing.packet_time_microsec, current_time, cnx->is_time_stamp_enabled)) {
                        /* RTT increased too much, get out of slow start! */

                        if (cubic_state->rtt_filter.rtt_filtered_min > PICOQUIC_TARGET_RENO_RTT){
                            double correction;
                            if (cubic_state->rtt_filter.rtt_filtered_min > PICOQUIC_TARGET_SATELLITE_RTT) {
                                correction = (double)PICOQUIC_TARGET_SATELLITE_RTT / (double)cubic_state->rtt_filter.rtt_filtered_min;
                            }
                            else {
                                correction = (double)PICOQUIC_TARGET_RENO_RTT / (double)cubic_state->rtt_filter.rtt_filtered_min;
                            }
                            uint64_t base_window = (uint64_t)(correction * (double)path_x->cwin);
                            uint64_t delta_window = path_x->cwin - base_window;
                            path_x->cwin -= (delta_window / 2);
                        }
                        else {
                            /* In the general case, compensate for the growth of the window after the acknowledged packet was sent. */
                            path_x->cwin /= 2;
                        }

                        cubic_state->ssthresh = path_x->cwin;
                        cubic_state->W_max = (double)path_x->cwin / (double)path_x->send_mtu;
                        cubic_state->W_last_max = cubic_state->W_max;
                        cubic_state->W_reno = ((double)path_x->cwin);
                        path_x->is_ssthresh_initialized = 1;
                        /* enter recovery to ignore the losses expected if the window grew
                        * too large after the acknowleded packet was sent. */
                        cubic_enter_recovery(cnx, path_x, notification, cubic_state, current_time);
                        /* apply a correction to enter the test phase immediately */
                        uint64_t K_micro = (uint64_t)(cubic_state->K * 1000000.0);
                        if (K_micro > current_time) {
                            cubic_state->K = ((double)current_time) / 1000000.0;
                            cubic_state->start_of_epoch = 0;
                        }
                        else {
                            cubic_state->start_of_epoch = current_time - K_micro;
                        }
                    }
                }
                break;
            case picoquic_congestion_notification_seed_cwin:
                if (cubic_state->alg_state == picoquic_cubic_alg_slow_start) {
                    if (cubic_state->ssthresh == UINT64_MAX) {
                        if (path_x->cwin < ack_state->nb_bytes_acknowledged) {
                            path_x->cwin = ack_state->nb_bytes_acknowledged;
                        }
                        cubic_state->ssthresh = ack_state->nb_bytes_acknowledged;
                        cubic_state->W_max = (double)path_x->cwin / (double)path_x->send_mtu;
                        cubic_state->W_last_max = cubic_state->W_max;
                        cubic_state->W_reno = ((double)path_x->cwin);
                        path_x->is_ssthresh_initialized = 1;
                        cubic_enter_avoidance(cubic_state, current_time);
                    }
                }
                break;
            /*
             * cover cubic_reset().
             */
            case picoquic_congestion_notification_reset:
                cubic_reset(cubic_state, path_x, current_time);
                break;
            default:
                break;

        }

        /* Compute pacing data */
        picoquic_update_pacing_data(path_x, cubic_state->alg_state == picoquic_cubic_alg_slow_start &&
            cubic_state->ssthresh == UINT64_MAX);
    }
}
```

### Rust body
```rust
    ) {
        use crate::Instant;

        path_x.is_cc_data_updated = true;

        match notification {
            CongestionNotification::Acknowledgement => match self.alg_state {
                CubicAlgState::SlowStart => {
                    path_x.cwin = path_x.update_target_cwin_estimation();
                    if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
                        path_x.cwin +=
                            path_x.slow_start_increase_ex(ack_state.nb_bytes_acknowledged, false);
                        if path_x.cwin >= self.ssthresh {
                            self.w_reno = path_x.cwin as f64 / 2.0;
                            path_x.is_ssthresh_initialized = true;
                            self.enter_avoidance(current_time);
                        }
                    }
                }
                CubicAlgState::Recovery => {
                    self.alg_state = CubicAlgState::SlowStart;
                    path_x.cwin += ack_state.nb_bytes_acknowledged;
                    if path_x.cwin >= self.ssthresh {
                        self.alg_state = CubicAlgState::CongestionAvoidance;
                    }
                }
                CubicAlgState::CongestionAvoidance => {
                    if path_x.last_time_acked_data_frame_sent > path_x.last_sender_limited_time {
                        // Protect against limited senders.
                        if self.start_of_epoch < path_x.last_sender_limited_time.ticks() {
                            self.start_of_epoch = path_x.last_sender_limited_time.ticks();
                        }
                        let w_cubic = self.w_cubic(current_time);
                        let win_cubic = (w_cubic * path_x.send_mtu as f64) as u64;
                        self.w_reno += ack_state.nb_bytes_acknowledged as f64
                            * path_x.send_mtu as f64
                            / self.w_reno;
                        if win_cubic as f64 > self.w_reno {
                            path_x.cwin = win_cubic;
                        } else {
                            path_x.cwin = self.w_reno as u64;
                        }
                    }
                }
            },

            CongestionNotification::Repeat
            | CongestionNotification::Timeout
            | CongestionNotification::EcnEc => match self.alg_state {
                CubicAlgState::SlowStart => {
                    if (notification == CongestionNotification::EcnEc
                        || self.rtt_filter.hystart_loss_test(
                            notification,
                            ack_state.lost_packet_number,
                            SMOOTHED_LOSS_THRESHOLD,
                        ))
                        && (current_time.wrapping_sub(self.start_of_epoch)
                            > path_x.smoothed_rtt.ticks()
                            || self.recovery_sequence <= connection.ack_number(path_x))
                    {
                        path_x.is_ssthresh_initialized = true;
                        self.enter_recovery(connection, path_x, notification, current_time);
                    }
                }
                CubicAlgState::Recovery | CubicAlgState::CongestionAvoidance => {
                    if ack_state.lost_packet_number >= self.recovery_sequence
                        && (notification == CongestionNotification::EcnEc
                            || self.rtt_filter.hystart_loss_test(
                                notification,
                                ack_state.lost_packet_number,
                                SMOOTHED_LOSS_THRESHOLD,
                            ))
                    {
                        self.enter_recovery(connection, path_x, notification, current_time);
                    }
                }
            },

            CongestionNotification::SpuriousRepeat => {
                self.correct_spurious(path_x, current_time);
            }

            CongestionNotification::RttMeasurement
                if self.alg_state == CubicAlgState::SlowStart && self.ssthresh == u64::MAX =>
            {
                let rtt_meas = if connection.is_time_stamp_enabled {
                    ack_state.one_way_delay
                } else {
                    ack_state.rtt_measurement
                };
                let ptime = Instant::from_ticks(
                    connection
                        .paths
                        .first()
                        .map_or(0, |p| p.pacing.packet_time_microsec.ticks()),
                );
                if self.rtt_filter.hystart_test(
                    rtt_meas,
                    ptime,
                    Instant::from_ticks(current_time),
                    connection.is_time_stamp_enabled,
                ) {
                    let rtt_min = self.rtt_filter.rtt_filtered_min;
                    let target_reno = crate::internal::TARGET_RENO_RTT;
                    let target_sat = crate::internal::TARGET_SATELLITE_RTT;

                    if rtt_min > target_reno {
                        let correction = if rtt_min > target_sat {
                            target_sat.ticks() as f64 / rtt_min.ticks() as f64
                        } else {
                            target_reno.ticks() as f64 / rtt_min.ticks() as f64
                        };
                        let base_window = (correction * path_x.cwin as f64) as u64;
                        let delta_window = path_x.cwin - base_window;
                        path_x.cwin -= delta_window / 2;
                    } else {
                        path_x.cwin /= 2;
                    }

                    self.ssthresh = path_x.cwin;
                    self.w_max = path_x.cwin as f64 / path_x.send_mtu as f64;
                    self.w_last_max = self.w_max;
                    self.w_reno = path_x.cwin as f64;
                    path_x.is_ssthresh_initialized = true;

                    self.enter_recovery(connection, path_x, notification, current_time);

                    // Adjust epoch so we enter the test phase immediately.
                    let k_micro = (self.k * 1_000_000.0) as u64;
                    if k_micro > current_time {
                        self.k = current_time as f64 / 1_000_000.0;
                        self.start_of_epoch = 0;
                    } else {
                        self.start_of_epoch = current_time - k_micro;
                    }
                }
            }

            CongestionNotification::SeedCwin
                if self.alg_state == CubicAlgState::SlowStart && self.ssthresh == u64::MAX =>
            {
                if path_x.cwin < ack_state.nb_bytes_acknowledged {
                    path_x.cwin = ack_state.nb_bytes_acknowledged;
                }
                self.ssthresh = ack_state.nb_bytes_acknowledged;
                self.w_max = path_x.cwin as f64 / path_x.send_mtu as f64;
                self.w_last_max = self.w_max;
                self.w_reno = path_x.cwin as f64;
                path_x.is_ssthresh_initialized = true;
                self.enter_avoidance(current_time);
            }

            CongestionNotification::Reset => {
                self.reset(path_x, current_time);
            }

            _ => {}
        }

        // Update pacing data.
        let in_slow_start = self.alg_state == CubicAlgState::SlowStart && self.ssthresh == u64::MAX;
        path_x.update_pacing_data(in_slow_start as i32);
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_enqueue_queue`
C: `picoquic/dualq_aqm.c:99-112 dualq_enqueue_queue`
Rust: `rs/fq/src/tests/dualq.rs:80-83 enqueue`

### C body
```c
{
    if (xq->queue_first == NULL) {
        xq->queue_first = packet;
        xq->queue_last = packet;
    }
    else {
        xq->queue_last->next_packet = packet;
        xq->queue_last = packet;
    }
    packet->next_packet = 0;
    xq->count += 1;
    xq->queue_bytes += packet->length;
}
```

### Rust body
```rust
    pub fn enqueue(&mut self, packet: TestSimPacket) {
        self.queue_bytes += packet.length as u64;
        self.packets.push_back(packet);
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_scheduler`
C: `picoquic/dualq_aqm.c:193-208 dualq_scheduler`
Rust: `rs/fq/src/tests/dualq.rs:306-324 scheduler`

### C body
```c
picoquictest_sim_packet_t* dualq_scheduler(dualq_state_t* dualq, int* is_lq) {
    picoquictest_sim_packet_t* packet = NULL;
    *is_lq = ((dualq->schedule_tick & 0x0f) == 0) ? 0 : 1;

    packet = dualq_dequeue_queue((*is_lq == 0)? &dualq->cq : &dualq->lq);
    if (packet == NULL) {
        *is_lq ^= 1;
        packet = dualq_dequeue_queue((*is_lq == 0) ? &dualq->cq : &dualq->lq);
    }
    
    dualq->schedule_tick += 1;
    dualq->schedule_tick &= 0x0f;
    return packet;
}
```

### Rust body
```rust
    fn scheduler(&mut self) -> Option<(TestSimPacket, bool)> {
        let mut is_lq = (self.schedule_tick & 0x0f) != 0;
        let mut packet = if is_lq {
            self.lq.dequeue()
        } else {
            self.cq.dequeue()
        };
        if packet.is_none() {
            is_lq = !is_lq;
            packet = if is_lq {
                self.lq.dequeue()
            } else {
                self.cq.dequeue()
            };
        }

        self.schedule_tick = (self.schedule_tick + 1) & 0x0f;
        packet.map(|packet| (packet, is_lq))
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_update_it`
C: `picoquic/dualq_aqm.c:323-345 dualq_update_it`
Rust: `rs/fq/src/tests/dualq.rs:381-388 update_it`

### C body
```c
{
    picoquictest_sim_packet_t* packet;
    int should_drop;
    
    while (link->queue_time <= current_time) {
        if ((packet = dualq_dequeue_one(dualq, current_time, &should_drop)) != NULL) {
            picoquictest_sim_link_enqueue(link, packet, current_time, should_drop);
        }
        else {
            break;
        }
    }

    if (current_time >= dualq->update_next) {
        dualq_pi2_update(dualq, current_time);
        dualq->update_next = current_time + dualq->Tupdate;
    }
}
```

### Rust body
```rust
        while link.queue_time.ticks() <= current_time.ticks() {
            if let Some((packet, should_drop)) = self.dequeue_one(current_time) {
                link.enqueue(packet, current_time, should_drop);
            } else {
                break;
            }
        }
```

## Pair `picoquic/dualq_aqm.c:dualq_release`
C: `picoquic/dualq_aqm.c:374-388 dualq_release`
Rust: `rs/fq/src/tests/dualq.rs:235-238 release`

### C body
```c
{
    dualq_state_t* dualq = (dualq_state_t*)self;
    picoquictest_sim_packet_t* packet;

    while ((packet = dualq_dequeue_queue(&dualq->lq)) != NULL){
        picoquictest_sim_link_enqueue(link, packet, 0, 1);
    }
    while ((packet = dualq_dequeue_queue(&dualq->cq)) != NULL) {
        picoquictest_sim_link_enqueue(link, packet, 0, 1);
    }

    free(self);
    link->aqm_state = NULL;
}
```

### Rust body
```rust
        while let Some(packet) = self.lq.dequeue() {
            link.enqueue(packet, Instant::from_ticks(0), true);
        }
```

## Pair `picoquic/ech.c:picoquic_base64_decode`
C: `picoquic/ech.c:46-75 picoquic_base64_decode`
Rust: `rs/fq/src/config.rs:607-614 base64_decode`

### C body
```c
{
    int ret = 0;
    ptls_buffer_t config;
    uint8_t short_buf[256];
    ptls_base64_decode_state_t d_state;
    *v = NULL;
    *v_len = 0;
    ptls_buffer_init(&config, short_buf, sizeof(short_buf));
    ptls_base64_decode_init(&d_state);
    ret = ptls_base64_decode(b64_txt, &d_state, &config);
    if (ret == 0 && (d_state.status == PTLS_BASE64_DECODE_DONE || (d_state.status == PTLS_BASE64_DECODE_IN_PROGRESS && d_state.nbc == 0))) {
        ret = 0;
        if (config.off > 0) {
            if ((*v = (uint8_t*)malloc(config.off)) == NULL) {
                ret = PICOQUIC_ERROR_MEMORY;
            }
            else {
                memcpy(*v, config.base, config.off);
                *v_len = config.off;
            }
        }
    }
    ptls_buffer_dispose(&config);
    return ret;
}
```

### Rust body
```rust
    {
        table[c as usize] = i as u8;
    }
```

## Pair `picoquic/ech.c:ech_opener_callback`
C: `picoquic/ech.c:230-267 ech_opener_callback`
Rust: `rs/fq/src/ech.rs:300-327 open`

### C body
```c
{
    ptls_aead_context_t* aead = NULL;
    ptls_buffer_t infobuf;
    int ret = 0;
    ech_opener_callback_t* ech_cb = (ech_opener_callback_t*)cb;

    *cipher = NULL;
    for (size_t i = 0; picoquic_hpke_cipher_suites[i] != NULL; ++i) {
        if (picoquic_hpke_cipher_suites[i]->id.kdf == cipher_id.kdf &&
            picoquic_hpke_cipher_suites[i]->id.aead == cipher_id.aead) {
            *cipher = picoquic_hpke_cipher_suites[i];
            break;
        }
    }
    if (*cipher == NULL)
        return NULL;
    *p_kem = ech_cb->kem;

    /* Compose the "info" field by combining the "info_prefix" and the selected configuration.
    * In the unit test example, the binary string is preceded by a two bytes of length, with
    * an added null byte at the end.
    */

    ptls_buffer_init(&infobuf, "", 0);
    ptls_buffer_pushv(&infobuf, info_prefix.base, info_prefix.len);
    ptls_buffer_pushv(&infobuf, ech_cb->config.base+2, ech_cb->config.off - 2);
    ret = ptls_hpke_setup_base_r(ech_cb->kem, *cipher, ech_cb->keyex, &aead, enc,
        ptls_iovec_init(infobuf.base, infobuf.off));
Exit:
    ptls_buffer_dispose(&infobuf);
    return aead;
}
```

### Rust body
```rust
    ) -> Option<EchOpenResult> {
        // Find the matching cipher suite in the supported list.
        let cipher = SUPPORTED_CIPHER_SUITES
            .iter()
            .copied()
            .find(|c| c.kdf == cipher_id.kdf && c.aead == cipher_id.aead)?;

        // Build info = info_prefix || config[2..]
        // (config[0..2] is the two-byte ECHConfigList outer length prefix,
        // skipped to match the `ptls_buffer_pushv(&infobuf, config.base+2, …)`
        // call in the C implementation.)
        let config_payload = self.config.get(2..).unwrap_or(&[]);
        let mut info = Vec::with_capacity(info_prefix.len() + config_payload.len());
        info.extend_from_slice(info_prefix);
        info.extend_from_slice(config_payload);

        // HPKE base-mode receiver setup (equivalent to ptls_hpke_setup_base_r).
        let key_material = hpke_setup_receiver(self.kem_id, cipher, &self.private_key, enc, &info)?;
        Some(EchOpenResult {
            cipher,
            key_material,
        })
    }
```

## Pair `picoquic/ech.c:picoquic_ech_configure_client`
C: `picoquic/ech.c:391-413 picoquic_ech_configure_client`
Rust: `rs/fq/src/lib.rs:4714-4717 ech_configure_client`

### C body
```c
{
    int ret = 0;
    picoquic_tls_ctx_t* tls_ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    tls_ctx->handshake_properties.client.ech.configs.base = (uint8_t*)malloc(config_length+1);
    if (tls_ctx->handshake_properties.client.ech.configs.base == NULL) {
        ret = PICOQUIC_ERROR_MEMORY;
    }
    else {
        if (config_length > 0) {
            memcpy(tls_ctx->handshake_properties.client.ech.configs.base, config_data, config_length);
        }
        tls_ctx->handshake_properties.client.ech.configs.len = config_length;
    }
    return ret;
}
```

### Rust body
```rust
    pub fn ech_configure_client(&mut self, config_data: &[u8]) -> Result<(), Error> {
        self.ech_client_config = Some(config_data.to_vec());
        Ok(())
    }
```

## Pair `picoquic/ech.c:picoquic_ech_get_kem_from_curve`
C: `picoquic/ech.c:655-670 picoquic_ech_get_kem_from_curve`
Rust: `rs/fq/src/ech.rs:424-462 ech_get_kem_from_curve`

### C body
```c
{
    int ret = -1;

    for (int i = 0; i < PICOQUIC_HPKE_KEM_NB_MAX; i++) {
        if (picoquic_hpke_kems[i] == NULL) {
            break;
        }
        else if (picoquic_hpke_kems[i]->keyex->id == group_id) {
            *kem = picoquic_hpke_kems[i];
            ret = 0;
            break;
        }
    }
    return ret;
}
```

### Rust body
```rust
pub struct PublicKeyAsn1<'a> {
    /// Algorithm OID value bytes (tag + length stripped).
    ///
    /// C: `public_key_algo.{base,len}`
    pub algo: &'a [u8],
    /// Parameters field: raw bytes from after the algorithm OID to the
    /// end of the inner AlgorithmIdentifier SEQUENCE.  Includes the
    /// tag + length of any nested TLV; empty when there are no parameters
    /// (e.g. X25519 keys).
    ///
    /// C: `public_key_param.{base,len}`
    pub param: &'a [u8],
    /// BIT STRING value bytes.  The first byte is the padding-count
    /// (always `0x00` for the curves picoquic supports); the key bytes
    /// follow immediately.
    ///
    /// C: `public_key_bit_string.{base,len}`
    pub bit_string: &'a [u8],
    /// Total number of bytes consumed from the input.
    pub consumed: usize,
}
```

## Pair `picoquic/ech.c:picoquic_ech_create_config_from_pk`
C: `picoquic/ech.c:802-825 picoquic_ech_create_config_from_pk`
Rust: `rs/fq/src/ech.rs:978-1003 ech_create_config_from_pk`

### C body
```c
{
    int ret = 0;
    ptls_hpke_kem_t* kem = NULL;
    ptls_hpke_cipher_suite_t* cipher_vec[PICOQUIC_HPKE_CIPHER_SUITE_NB_MAX + 1];

    if ((ret = picoquic_ech_get_kem_from_curve(&kem, group_id)) != 0) {
        DBG_PRINTF("Could not find KEM for group = 0x%04x", group_id);
    }
    /* Find the list of locally supported cipher suites, retaining only the most common */
    else if ((ret = picoquic_ech_get_ciphers_from_kem(cipher_vec, PICOQUIC_HPKE_CIPHER_SUITE_NB_MAX + 1, kem->id)) != 0) {
        DBG_PRINTF("Could not find Ciphers for kem = 0x%04x", kem->id);
    }
    else {
        /* Compute a config ID from public key bytes, kem-id, cipher-suite IDs and public name */
        uint8_t config_id = ech_config_id_from_config(public_key_bits, kem, cipher_vec, public_name);
        /* Encode the key config */
        if ((ret = ptls_ech_encode_config(config_buf, config_id, kem, public_key_bits,
            cipher_vec, 255, public_name)) != 0) {
            DBG_PRINTF("Could not encode the configuration, err: %x", ret);
        }
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<Vec<u8>, Error> {
    // Derive KEM id from the TLS named-group id.
    let kem_id = ech_get_kem_from_curve(group_id).ok_or(Error::Generic)?;

    // Collect the preferred cipher suites for this KEM.
    let mut cipher_buf = [HpkeCipherSuiteId { kdf: 0, aead: 0 }; 5];
    let n_ciphers = ech_get_ciphers_from_kem(kem_id, &mut cipher_buf)?;
    let cipher_suites = &cipher_buf[..n_ciphers];

    // Compute a one-byte config-ID from all the key material.
    let config_id = ech_config_id_from_config(public_key_bits, kem_id, cipher_suites, public_name);

    // Encode and return the ECHConfig wire bytes.
    ech_encode_config(
        config_id,
        kem_id,
        public_key_bits,
        cipher_suites,
        255,
        public_name,
    )
}
```

## Pair `picoquic/ech.c:picoquic_ech_create_config_from_private_key`
C: `picoquic/ech.c:879-925 picoquic_ech_create_config_from_private_key`
Rust: `rs/fq/src/lib.rs:4770-4781 ech_create_config_from_private_key`

### C body
```c
{
    int ret = 0;

    *config = NULL;
    *config_len = 0;

    if (picoquic_get_public_key_from_private_fn != NULL) {
        uint16_t group_id = 0;
        ptls_iovec_t public_key_bits = ptls_iovec_init(NULL, 0);

        ret = picoquic_get_public_key_from_private_fn(private_key_file, &public_key_bits.base, &public_key_bits.len);

        if (ret == 0) {
            switch (public_key_bits.len) {
            case 0x21: /* x25519 */
                group_id = 0x001d;
                break;
            case 0x41: /* secp265r1 */
                group_id = 0x0017;
                break;
            case 0x61: /* x25519 */
                group_id = 0x0018;
                break;
            default:
                DBG_PRINTF("Cannot find group ID from pubkey length 0x%02x from %s",
                    public_key_bits.len, private_key_file);
                ret = -1;
                break;
            }
        }

        if (ret == 0) {
            ptls_buffer_t config_buf;
            ptls_buffer_init(&config_buf, "", 0);
            ret = picoquic_ech_create_config_from_pk(&config_buf, group_id, public_key_bits, public_name);
            if (ret == 0) {
                ret = picoquic_ech_create_config_list_from_config(&config_buf, config, config_len);
            }
            ptls_buffer_dispose(&config_buf);
        }
        if (public_key_bits.base != NULL) {
            free(public_key_bits.base);
        }
    }
    return ret;
}
```

### Rust body
```rust
pub fn tls_api_init() {
    crate::tls_api::tls_api_init();
}
```

## Pair `picoquic/fastcc.c:picoquic_fastcc_delay_threshold`
C: `picoquic/fastcc.c:65-72 picoquic_fastcc_delay_threshold`
Rust: `rs/fq/src/fastcc.rs:81-97 fastcc_delay_threshold`

### C body
```c
{
    uint64_t delay = rtt_min / 8;
    if (delay > FASTCC_DELAY_THRESHOLD_MAX) {
        delay = FASTCC_DELAY_THRESHOLD_MAX;
    }
    return delay;
}
```

### Rust body
```rust
pub fn picoquic_fastcc_reset(state: &mut FastccState, path_x: &mut Path, current_time: Instant) {
    *state = FastccState::default();
    state.alg_state = FastccAlgState::Initial;
    state.rtt_min = path_x.smoothed_rtt.ticks();
    state.rolling_rtt_min = state.rtt_min;
    state.delay_threshold = fastcc_delay_threshold(state.rtt_min);
    state.end_of_epoch = current_time.ticks().saturating_add(FASTCC_PERIOD);
    path_x.cwin = CWIN_INITIAL;
}
```
