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

## `picoquic/bbr.c:BBRStartProbeBW_DOWN`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust matches most assignments but replaces BBRExpTest(bbr_state, do_rapid_start) with direct exp_flags.do_rapid_start, which may skip visible test behavior.
* C source: `picoquic/bbr.c:1793-1814`
* C signature: `void BBRStartProbeBW_DOWN(picoquic_bbr_state_t *, picoquic_path_t *, uint64_t)`
* Rust source: `rs/fq/src/bbr.rs:1052-1077`
* Rust item: `start_probe_bw_down`

### C body
```c
{
    bbr_state->pacing_gain = BBRProbeBwDownPacingGain;  /* pace a bit slowly */
    bbr_state->cwnd_gain = BBRProbeBwDownCwndGain;   /* maintain cwnd */
    BBRResetCongestionSignals(bbr_state);
    bbr_state->bw_probe_up_cnt = UINT32_MAX; /* not growing inflight_hi */
    if (bbr_state->probe_probe_bw_quickly && BBRExpTest(bbr_state, do_rapid_start)) {
        BBRPickProbeWaitEarly(bbr_state);
    }
    else {
        BBRPickProbeWait(bbr_state);
    }
    bbr_state->cycle_stamp = current_time;  /* start wall clock */
    bbr_state->ack_phase = picoquic_bbr_acks_probe_stopping;
    BBRStartRound(bbr_state, path_x);
    bbr_state->state = picoquic_bbr_alg_probe_bw_down;
    bbr_state->nb_rtt_excess = 0;
    bbr_state->app_limited_round_count = 0;
    bbr_state->app_limited_this_round = 0;

    path_x->is_cca_probing_up = 0;
}
```

### Rust body
```rust
    ) {
        const BBR_PROBE_BW_DOWN_PACING_GAIN: f64 = 0.9;
        const BBR_PROBE_BW_DOWN_CWND_GAIN: f64 = 2.0;
        self.pacing_gain = BBR_PROBE_BW_DOWN_PACING_GAIN;
        self.cwnd_gain = BBR_PROBE_BW_DOWN_CWND_GAIN;
        self.reset_congestion_signals();
        self.bw_probe_up_cnt = u32::MAX; // not growing inflight_hi
        if self.probe_probe_bw_quickly && self.exp_flags.do_rapid_start {
            self.pick_probe_wait_early();
        } else {
            self.pick_probe_wait();
        }
        self.cycle_stamp = current_time;
        self.ack_phase = BbrAckPhase::ProbeStopping;
        self.start_round(connection, path_x);
        self.state = BbrAlgState::ProbeBwDown;
        self.nb_rtt_excess = 0;
        self.app_limited_round_count = 0;
        self.app_limited_this_round = 0;
        path_x.is_cca_probing_up = false;
    }
```

## `picoquic/bytestream.c:byteread_int32`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust snippet only shows the 4-byte bounds check and error path; the visible body does not show reading big-endian bytes, advancing ptr, or success return.
* C source: `picoquic/bytestream.c:248-260`
* C signature: `int byteread_int32(bytestream *, uint32_t *)`
* Rust source: `rs/fq/src/bytestream.rs:318-322`
* Rust item: `read_u32`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 4) {
        return bytestream_error(s);
    }
    else {
        const uint8_t * ptr = s->data + s->ptr;
        *value = (((uint32_t)ptr[0]) << 24) | (((uint32_t)ptr[1]) << 16) | (((uint32_t)ptr[2]) << 8) | ptr[3];
        s->ptr += 4;
        return 0;
    }
}
```

### Rust body
```rust
        if self.data_ref().len() - self.ptr < 4 {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
```

## `picoquic/c4.c:c4_enter_push`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both bodies set alpha from probe level, push_alpha, reset era, and enter pushing, but Rust clamps probe_level before indexing while C indexes directly.
* C source: `picoquic/c4.c:744-754`
* C signature: `void c4_enter_push(picoquic_path_t *, c4_state_t *)`
* Rust source: `rs/fq/src/c4.rs:738-744`
* Rust item: `enter_push`

### C body
```c
{
    c4_state->alpha_1024_current = c4_push_rate_by_probe_level[c4_state->probe_level];
    c4_state->push_alpha = c4_state->alpha_1024_current;
    c4_era_reset(path_x, c4_state);
    c4_state->alg_state = c4_pushing;
}
```

### Rust body
```rust
    fn enter_push(&mut self, path_x: &mut Path, connection: &Connection) {
        let level = self.probe_level.clamp(0, C4_PROBE_LEVEL_MAX) as usize;
        self.alpha_1024_current = C4_PUSH_RATE_BY_PROBE_LEVEL[level];
        self.push_alpha = self.alpha_1024_current;
        self.era_reset(path_x, connection);
        self.alg_state = C4AlgState::Pushing;
    }
```

## `picoquic/config.c:config_optval_param_string`
* Phase 4C status: `suspect`
* Phase 4C rationale: C explicitly handles params == NULL and negative x against nb_param; Rust takes a slice and usize, so only the x >= params.len() case is body-visible.
* C source: `picoquic/config.c:191-200`
* C signature: `char * config_optval_param_string(char *, size_t, const option_param_t *, int, int)`
* Rust source: `rs/fq/src/config.rs:575-583`
* Rust item: `config_optval_param_string`

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

## `picoquic/config.c:picoquic_create_and_configure`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust omits visible C actions such as setting preferred address and BBR fallback, and treats missing ECH key/config as non-failing while C leaves ret unchanged after printing.
* C source: `picoquic/config.c:860-1033`
* C signature: `picoquic_quic_t * picoquic_create_and_configure(picoquic_quic_config_t *, picoquic_stream_data_cb_fn, void *, uint64_t, uint64_t *)`
* Rust source: `rs/fq/src/config.rs:1438-1569`
* Rust item: `create_and_configure`

### C body
```c
{
    /* Create context */
    /* TODO: padding policy
     * TODO: mtu max accessor
     * TODO: set supported CC without linking every option
     * TODO: set logging option without linking every option
     * TODO: set key log file option
     */
    picoquic_quic_t* quic = picoquic_create(
        config->nb_connections,
        config->server_cert_file,
        config->server_key_file,
        config->root_trust_file,
        config->alpn,
        default_callback_fn,
        default_callback_ctx,
        NULL,
        NULL,
        (config->has_reset_seed)?(uint8_t*)config->reset_seed : NULL,
        current_time,
        p_simulated_time,
        config->ticket_file_name,
        config->ticket_encryption_key,
        config->ticket_encryption_key_length);

    if (quic != NULL) {
        int ret = 0;
        picoquic_congestion_algorithm_t const* cc_algo = NULL;

        /* Additional configuration options */
        /* picoquic_set_alpn_select_fn(qserver, picoquic_demo_server_callback_select_alpn); */
        if (config->do_retry) {
            picoquic_set_cookie_mode(quic, 1);
        }
        else {
            /* TODO: option to provide cookie by default or not */
            picoquic_set_cookie_mode(quic, 2);
        }

        if (config->cc_algo_id != NULL) {
            cc_algo = picoquic_get_congestion_algorithm(config->cc_algo_id);
            if (cc_algo == NULL) {
                fprintf(stderr, "Unrecognized congestion algorithm: %s. Using BBR isntead.\n", config->cc_algo_id);
            }
        }
        if (cc_algo == NULL) {
            cc_algo = picoquic_bbr_algorithm;
        }

        picoquic_set_default_congestion_algorithm_ex(quic, cc_algo, config->cc_algo_option_string);

        picoquic_set_default_spinbit_policy(quic, config->spinbit_policy);
        picoquic_set_default_lossbit_policy(quic, config->lossbit_policy);

        picoquic_set_default_multipath_option(quic, config->multipath_option);
        picoquic_set_default_idle_timeout(quic, (uint64_t)config->idle_timeout);

        picoquic_set_cwin_max(quic, config->cwin_max);
        picoquic_set_default_address_discovery_mode(quic, config->address_discovery_mode);

        picoquic_set_preferred_address(&quic->default_tp.preferred_address,
            config->preferred_address_v4, config->preferred_address_v6, config->local_port);

        if (config->token_file_name) {
            if (picoquic_load_retry_tokens(quic, config->token_file_name) != 0) {
                fprintf(stderr, "No token file present. Will create one as <%s>.\n", config->token_file_name);
            }
        }

        if (config->force_zero_share) {
            quic->client_zero_share = 1;
        }

        if (config->mtu_max > 0) {
            picoquic_set_mtu_max(quic, config->mtu_max);
        }

        if (config->cnx_id_length != -1) {
            if (picoquic_set_default_connection_id_length(quic, (uint8_t)config->cnx_id_length) != 0) {
                fprintf(stderr, "Could not set CNX-ID length #%d.\n", config->cnx_id_length);
            }
        }

        /* Cannot set the cnx_id callback here, because it requires libraries
         * that are not linked by default */

         /* TODO: parameters to define padding policy */
        picoquic_set_padding_policy(quic, 39, 128);

        if (config->bin_dir != NULL) {
            picoquic_set_binlog(quic, config->bin_dir);
        }

        if (config->qlog_dir != NULL) {
            picoquic_set_qlog(quic, config->qlog_dir);
        }

        if (config->log_file != NULL) {
            picoquic_set_textlog(quic, config->log_file);
        }

        picoquic_set_log_level(quic, config->use_long_log);

        picoquic_set_preemptive_repeat_policy(quic, config->do_preemptive_repeat);

        picoquic_disable_port_blocking(quic, config->disable_port_blocking);

#ifndef PICOQUIC_WITHOUT_SSLKEYLOG
        picoquic_enable_sslkeylog(quic, config->enable_sslkeylog);
#endif

        if (config->initial_random >= 0 && config->initial_random <= 2) {
            picoquic_set_random_initial(quic, config->initial_random);
        }

        if (config->cipher_suite_id != 0) {
            int iana_cipher_suite_code = config->cipher_suite_id;
            if (config->cipher_suite_id == 20) {
                iana_cipher_suite_code = PICOQUIC_CHACHA20_POLY1305_SHA256;
            }
            else if (config->cipher_suite_id == 128) {
                iana_cipher_suite_code = PICOQUIC_AES_128_GCM_SHA256;
            }
            else if (config->cipher_suite_id == 256) {
                iana_cipher_suite_code = PICOQUIC_AES_256_GCM_SHA384;
            }
            if (picoquic_set_cipher_suite(quic, iana_cipher_suite_code) != 0) {
                fprintf(stderr, "Could not set cipher suite #%d.\n", config->cipher_suite_id);
            }
        }

        if (config->do_retry != 0) {
            picoquic_set_cookie_mode(quic, 1);
        }
        else {
            picoquic_set_cookie_mode(quic, 2);
        }
        /* TODO: control whether to */
        /* picoquic_set_key_log_file_from_env(quic); */

        picoquic_set_default_bdp_frame_option(quic, config->bdp_frame_option);

        if (config->ech_public_name != NULL) {
            if (config->ech_key_file == NULL || config->ech_config_file) {
                fprintf(stderr, "Cannot create a configuration if key and config file are not specified.\n");
            } else {
                ret = picoquic_ech_create_config_file(config->ech_public_name, config->ech_key_file, config->ech_config_file);
            }
        }

        if (ret == 0 && (config->ech_key_file != NULL || config->ech_target != NULL)) {
            ret = picoquic_ech_configure_quic_ctx(quic, config->ech_key_file, config->ech_config_file);
        }

        if (ret == 0 && config->flow_control_max > 0)
        {
            picoquic_set_max_data_control(quic, config->flow_control_max);
        }

        if (ret != 0) {
            /* Something went wrong */
            DBG_PRINTF("QUIC configuration fails, ret = %d (0x%x)", ret, ret);
            picoquic_free(quic);
            quic = NULL;
        }
    }

    return quic;
}
```

### Rust body
```rust
    ) -> Option<Box<Quic>> {
        let reset_seed = if self.has_reset_seed {
            self.reset_seed
        } else {
            [0u8; 16]
        };

        let mut quic = Quic::new(
            self.nb_connections,
            self.server_cert_file.as_deref(),
            self.server_key_file.as_deref(),
            self.root_trust_file.as_deref(),
            self.alpn.as_deref(),
            default_callback,
            None,
            reset_seed,
            current_time,
            self.ticket_file_name.as_deref(),
            self.ticket_encryption_key.as_deref(),
        )?;

        if self.do_retry {
            quic.set_cookie_mode(1);
        } else {
            quic.set_cookie_mode(2);
        }

        if let Some(ref cc_id) = self.cc_algo_id {
            let _ = quic.set_default_congestion_algorithm_by_name(cc_id);
        }

        let _ = quic.set_default_spinbit_policy(self.spinbit_policy);
        quic.set_default_lossbit_policy(self.lossbit_policy);
        quic.set_default_multipath_option(self.multipath_option);
        quic.set_default_idle_timeout(crate::Duration::from_ticks(self.idle_timeout as u64 * 1000));
        quic.set_cwin_max(self.cwin_max);
        quic.set_default_address_discovery_mode(self.address_discovery_mode);

        if let Some(ref token_file) = self.token_file_name {
            let _ = quic.load_retry_tokens(token_file);
        }

        if self.force_zero_share {
            quic.client_zero_share = true;
        }

        if self.mtu_max > 0 {
            quic.set_mtu_max(self.mtu_max as u32);
        }

        if self.connection_id_length != -1 {
            let _ = quic.set_default_connection_id_length(self.connection_id_length as u8);
        }

        quic.set_padding_policy(39, 128);

        if let Some(ref bin_dir) = self.bin_dir {
            let _ = quic.set_binlog(Some(bin_dir.as_str()));
        }

        if let Some(ref qlog_dir) = self.qlog_dir {
            let _ = quic.set_qlog(qlog_dir.as_str());
        }

        if let Some(ref log_file) = self.log_file {
            let _ = quic.set_textlog(Some(log_file.as_str()));
        }

        quic.set_log_level(if self.use_long_log { 1 } else { 0 });
        quic.set_preemptive_repeat_policy(self.do_preemptive_repeat);
        quic.set_port_blocking_disabled(self.disable_port_blocking);
        quic.set_sslkeylog_enabled(self.enable_sslkeylog);

        if self.initial_random <= 2 {
            quic.set_random_initial(self.initial_random as i32);
        }

        if self.cipher_suite_id != 0 {
            let iana_code = match self.cipher_suite_id {
                20 => crate::CHACHA20_POLY1305_SHA256,
                128 => crate::AES_128_GCM_SHA256,
                256 => crate::AES_256_GCM_SHA384,
                v => v as u16,
            };
            let _ = quic.set_cipher_suite(iana_code);
        }

        if self.do_retry {
            quic.set_cookie_mode(1);
        } else {
            quic.set_cookie_mode(2);
        }

        quic.set_default_bdp_frame_option(self.bdp_frame_option != 0);

        let mut failed = false;

        if let Some(ref public_name) = self.ech_public_name {
            if self.ech_key_file.is_none() || self.ech_config_file.is_none() {
                // key file or config file not specified — cannot create ECH config
            } else {
                let key_file = self.ech_key_file.as_deref().unwrap();
                let cfg_file = self.ech_config_file.as_deref().unwrap();
                if crate::ech_create_config_file(public_name, key_file, cfg_file).is_err() {
                    failed = true;
                }
            }
        }

        if !failed
            && (self.ech_key_file.is_some() || self.ech_target.is_some())
            && quic
                .ech_configure(
                    self.ech_key_file.as_deref(),
                    self.ech_config_file.as_deref(),
                )
                .is_err()
        {
            failed = true;
        }

        if !failed && self.flow_control_max > 0 {
            quic.set_max_data_control(self.flow_control_max);
        }

        if failed { None } else { Some(quic) }
    }
```
