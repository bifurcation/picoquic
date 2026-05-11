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

## Pair `picoquic/cc_common.c:picoquic_cc_hystart_test`
C: `picoquic/cc_common.c:167-208 picoquic_cc_hystart_test`
Rust: `rs/fq/src/cc_common.rs:193-202 hystart_test`

### C body
```c
{
    int ret = 0;

    /* Add silly instruction to bypass "argument not use" warning without changing the signature */
    if (is_one_way_delay_enabled && rtt_measurement == 0) {
        return 0;
    }

    if(current_time > rtt_track->last_rtt_sample_time + 1000) {
        picoquic_cc_filter_rtt_min_max(rtt_track, rtt_measurement);
        rtt_track->last_rtt_sample_time = current_time;

        if (rtt_track->is_init) {
            uint64_t delta_max;

            if (rtt_track->rtt_filtered_min == 0 ||
                rtt_track->rtt_filtered_min > rtt_track->sample_max) {
                rtt_track->rtt_filtered_min = rtt_track->sample_max;
            }
            delta_max = rtt_track->rtt_filtered_min / 4;
            if (delta_max < packet_time) {
                delta_max = packet_time;
            }

            if (rtt_track->sample_min > rtt_track->rtt_filtered_min) {
                if (rtt_track->sample_min > rtt_track->rtt_filtered_min + delta_max) {
                    rtt_track->nb_rtt_excess++;
                    if (rtt_track->nb_rtt_excess >= PICOQUIC_MIN_MAX_RTT_SCOPE) {
                        /* RTT increased too much, get out of slow start! */
                        ret = 1;
                    }
                }
            }
            else {
                rtt_track->nb_rtt_excess = 0;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
        if is_one_way_delay_enabled && rtt_measurement == Duration::from_ticks(0) {
            return false;
        }
```

## Pair `picoquic/cc_common.c:picoquic_cc_update_target_cwin_estimation`
C: `picoquic/cc_common.c:258-270 picoquic_cc_update_target_cwin_estimation`
Rust: `rs/fq/src/cc_common.rs:335-416 update_target_cwin_estimation`

### C body
```c
uint64_t picoquic_cc_update_target_cwin_estimation(picoquic_path_t* path_x) {
    /* RTT measurements will happen after the bandwidth is estimated. */
    uint64_t max_win = PICOQUIC_BYTES_FROM_RATE(path_x->smoothed_rtt, path_x->peak_bandwidth_estimate);
    uint64_t min_win = max_win / 2;

    /* Return increased cwin, if larger than current cwin. */
    if (min_win > path_x->cwin) {
        return min_win;
    }

    /* Otherwise, return current cwin. */
    return path_x->cwin;
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

## Pair `picoquic/config.c:config_set_string_param`
C: `picoquic/config.c:148-179 config_set_string_param`
Rust: `rs/fq/src/config.rs:664-672 config_set_string_param`

### C body
```c
{
    int ret = 0;
    char* p_dup = NULL;

    if (*v != NULL) {
        free((void*)*v);
        *v = NULL;
    }

    if (params != NULL && x >= 0 && x < nb_param && params[x].param != NULL)
    {
        size_t alloc_length = params[x].length + 1;

        if (params[x].length > 0 && alloc_length > params[x].length) {
            p_dup = (char *)malloc(alloc_length);
        }
        if (p_dup != NULL) {
            memcpy(p_dup, params[x].param, params[x].length);
            p_dup[params[x].length] = 0;
            *v = (char const*)p_dup;
        }
        else {
            fprintf(stderr, "Cannot allocate %zu characters\n", params[x].length);
            ret = -1;
        }
    }
    else {
        ret = -1;
    }
    return ret;
}
```

### Rust body
```rust
fn config_set_string_param(v: &mut Option<String>, params: &[&str], x: usize) -> Result<(), Error> {
    *v = None;
    if x < params.len() && !params[x].is_empty() {
        *v = Some(params[x].to_string());
        Ok(())
    } else {
        Err(Error::InvalidArgument)
    }
}
```

## Pair `picoquic/config.c:config_set_port`
C: `picoquic/config.c:226-272 config_set_port`
Rust: `rs/fq/src/tests/config.rs:704-756 config_set_port`

### C body
```c
{
    int ret = 0;
    char opval_buffer[256];
    char const* p = port_string;
    int is_port_shared = 0;
    int p1 = 0;
    int p2 = 0;
    int nb_threads = 0;

    if (*p == 'S') {
        is_port_shared = 1;
        p++;
    }
    while (*p >= '0' && *p <= '9') {
        p1 *= 10;
        p1 += (*p - '0');
        p++;
    }
    if (*p == ':') {
        p++;
        while (*p >= '0' && *p <= '9') {
            p2 *= 10;
            p2 += (*p - '0');
            p++;
        }
    }
    if (*p == '*') {
        p++;
        while (*p >= '0' && *p <= '9') {
            nb_threads *= 10;
            nb_threads += (*p - '0');
            p++;
        }
    }
    if (*p != 0 || p1 < 0 || p1 > 65535 || p2 < 0 || p2 > 65535) {
        fprintf(stderr, "Invalid port: %s\n", config_optval_string(opval_buffer, 256, port_string, strlen(port_string)));
        ret = -1;
    }
    else {
        config->server_port = (uint16_t)p1;
        config->local_port = (uint16_t)p2;
        config->is_port_shared = is_port_shared;
        config->nb_threads = nb_threads;
    }
    return ret;
}
```

### Rust body
```rust
fn config_set_port() {
    // (port_string, is_valid, server_port, local_port, is_port_shared)
    let cases: &[(&str, bool, u16, u16, bool)] = &[
        ("4433", true, 4433, 0, false),
        ("443:4434", true, 443, 4434, false),
        ("S4433", true, 4433, 0, true),
        ("S443:4434", true, 443, 4434, true),
        ("S443:4434*", true, 443, 4434, true),
        ("S443:4434*1", true, 443, 4434, true),
        ("S443:4434*256", true, 443, 4434, true),
        ("4433*7", true, 4433, 0, false),
        ("", true, 0, 0, false),
        ("0", true, 0, 0, false),
        ("*5", true, 0, 0, false),
        ("0*3", true, 0, 0, false),
        ("65535", true, 65535, 0, false),
        ("S65535", true, 65535, 0, true),
        ("S65534:65535", true, 65534, 65535, true),
        ("65536", false, 0, 0, false),
        ("-1", false, 0, 0, false),
        ("abc", false, 0, 0, false),
        ("4433:abc", false, 0, 0, false),
        ("abc:4433", false, 0, 0, false),
        ("S4433:abc", false, 0, 0, false),
        ("Sabc:4433", false, 0, 0, false),
    ];

    for (i, &(port_string, is_valid, exp_server_port, exp_local_port, exp_is_port_shared)) in
        cases.iter().enumerate()
    {
        let mut config = Config::default();
        let result = config.set_port(port_string);
        let got_valid = result.is_ok();
        assert_eq!(
            got_valid, is_valid,
            "Case {i} ({port_string:?}): validity mismatch"
        );
        if got_valid {
            assert_eq!(
                config.server_port, exp_server_port,
                "Case {i} ({port_string:?}): server_port"
            );
            assert_eq!(
                config.local_port, exp_local_port,
                "Case {i} ({port_string:?}): local_port"
            );
            assert_eq!(
                config.is_port_shared, exp_is_port_shared,
                "Case {i} ({port_string:?}): is_port_shared"
            );
        }
    }
}
```

## Pair `picoquic/config.c:picoquic_config_usage`
C: `picoquic/config.c:609-612 picoquic_config_usage`
Rust: `rs/fq/src/tests/config.rs:582-591 config_usage`

### C body
```c
{
    picoquic_config_usage_file(stderr);
}
```

### Rust body
```rust
fn config_usage() {
    config_test_register_cc_algorithms();
    let mut buf = String::new();
    Config::write_usage(&mut buf);
    let expected = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/config_usage_ref.txt"
    ));
    assert_eq!(buf, expected);
}
```

## Pair `picoquic/config.c:picoquic_config_get_command_line_option_index`
C: `picoquic/config.c:667-681 picoquic_config_get_command_line_option_index`
Rust: `rs/fq/src/config.rs:705-742 picoquic_config_get_command_line_option_index`

### C body
```c
{
    int option_index = -1;

    if (opt_string[0] == '-' && opt_string[1] != 0) {
        if (opt_string[2] == 0) {
            option_index = picoquic_config_get_option_char_index(opt_string[1]);
        }
        else if (opt_string[1] == '-' && opt_string[2] != 0) {
            char const* opt_name = opt_string + 2;
            option_index = picoquic_config_get_option_name_index(opt_name, strlen(opt_name));
        }
    }
    return option_index;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    let option_index = usize::try_from(option_index).map_err(|_| Error::InvalidArgument)?;
    let entry = OPTION_TABLE
        .get(option_index)
        .ok_or(Error::InvalidArgument)?;
    let argc = argc.min(argv.len());
    let mut params = Vec::new();

    if entry.nb_params > 0 {
        params.push(optarg.ok_or(Error::InvalidArgument)?);
        while params.len() < entry.nb_params {
            if *p_optind >= argc {
                return Err(Error::InvalidArgument);
            }
            params.push(argv[*p_optind]);
            *p_optind += 1;
        }
    }

    apply_option(config, entry, &params)
}
```

## Pair `picoquic/config.c:picoquic_create_and_configure`
C: `picoquic/config.c:860-1033 picoquic_create_and_configure`
Rust: `rs/fq/src/config.rs:1438-1569 create_and_configure`

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

## Pair `picoquic/cubic.c:cubic_init`
C: `picoquic/cubic.c:70-81 cubic_init`
Rust: `rs/fq/src/cubic.rs:154-158 init`

### C body
```c
{
    /* Initialize the state of the congestion control algorithm */
    picoquic_cubic_state_t* cubic_state = (picoquic_cubic_state_t*)malloc(sizeof(picoquic_cubic_state_t));
#ifdef _WINDOWS
    UNREFERENCED_PARAMETER(option_string);
#endif
    path_x->congestion_alg_state = (void*)cubic_state;
    if (cubic_state != NULL) {
        cubic_reset(cubic_state, path_x, current_time);
    }
}
```

### Rust body
```rust
    pub fn init(path_x: &mut Path, _option_string: Option<&str>, current_time: u64) {
        let mut state = Box::new(CubicState::default());
        state.reset(path_x, current_time);
        path_x.congestion_alg_state = Some(state);
    }
```

## Pair `picoquic/cubic.c:cubic_enter_recovery`
C: `picoquic/cubic.c:144-204 cubic_enter_recovery`
Rust: `rs/fq/src/cubic.rs:166-215 enter_recovery`

### C body
```c
{
    cubic_state->recovery_sequence = picoquic_cc_get_sequence_number(cnx, path_x);
    cubic_state->previous_start_of_epoch = cubic_state->start_of_epoch;
    cubic_state->previous_alg_state = cubic_state->alg_state;
    cubic_state->previous_ssthresh = cubic_state->ssthresh;
    cubic_state->previous_cwin = path_x->cwin;
    /* Update similar to new reno, but different beta */
    cubic_state->W_max = (double)path_x->cwin / (double)path_x->send_mtu;
    /* Apply fast convergence */
    if (cubic_state->W_max < cubic_state->W_last_max) {
        cubic_state->W_last_max = cubic_state->W_max;
        cubic_state->W_max = cubic_state->W_max * PICOQUIC_CUBIC_BETA_ECN;
    }
    else {
        cubic_state->W_last_max = cubic_state->W_max;
    }
    /* Compute the new ssthresh */
    cubic_state->ssthresh = (uint64_t)(cubic_state->W_max * PICOQUIC_CUBIC_BETA_ECN * (double)path_x->send_mtu);
    if (cubic_state->ssthresh < PICOQUIC_CWIN_MINIMUM) {
        /* If things are that bad, fall back to slow start */

        cubic_state->alg_state = picoquic_cubic_alg_slow_start;
        cubic_state->ssthresh = UINT64_MAX;
        path_x->is_ssthresh_initialized = 0;
        cubic_state->previous_start_of_epoch = cubic_state->start_of_epoch;
        cubic_state->start_of_epoch = current_time;
        cubic_state->W_reno = PICOQUIC_CWIN_MINIMUM;
        path_x->cwin = PICOQUIC_CWIN_MINIMUM;
    }
    else {
        if (notification == picoquic_congestion_notification_timeout) {
            path_x->cwin = PICOQUIC_CWIN_MINIMUM;
            cubic_state->previous_start_of_epoch = cubic_state->start_of_epoch;
            cubic_state->start_of_epoch = current_time;
            cubic_state->alg_state = picoquic_cubic_alg_slow_start;
        }
        else {
            /* Enter congestion avoidance immediately */
            cubic_enter_avoidance(cubic_state, current_time);
            /* Compute the initial window for both Reno and Cubic */
            double W_cubic = cubic_W_cubic(cubic_state, current_time);
            uint64_t win_cubic = (uint64_t)(W_cubic * (double)path_x->send_mtu);
            cubic_state->W_reno = ((double)path_x->cwin) / 2.0;

            /* The formulas that compute "W_cubic" at the beginning of congestion avoidance
            * guarantee that "w_cubic" is larger than "w_reno" even if "fast convergence"
            * is applied as long as "beta_cubic" is greater than
            * (-1 + sqrt(1+4))/2, about 0.618033988749895.
            * Since beta_cubic is set to 3/4, we do not need to compare "w_cubic" and
            * "w_reno" to pick the largest. */
            path_x->cwin = win_cubic;
        }
    }
}
```

### Rust body
```rust
    ) {
        self.recovery_sequence = connection.sequence_number(path_x);
        self.previous_start_of_epoch = self.start_of_epoch;
        self.previous_alg_state = self.alg_state as u64;
        self.previous_ssthresh = self.ssthresh;
        self.previous_cwin = path_x.cwin;

        self.w_max = path_x.cwin as f64 / path_x.send_mtu as f64;

        // Fast convergence.
        if self.w_max < self.w_last_max {
            self.w_last_max = self.w_max;
            self.w_max *= CUBIC_BETA_ECN;
        } else {
            self.w_last_max = self.w_max;
        }

        self.ssthresh = (self.w_max * CUBIC_BETA_ECN * path_x.send_mtu as f64) as u64;

        if self.ssthresh < CWIN_MINIMUM {
            // Things are very bad — fall back to slow start.
            self.alg_state = CubicAlgState::SlowStart;
            self.ssthresh = u64::MAX;
            path_x.is_ssthresh_initialized = false;
            self.previous_start_of_epoch = self.start_of_epoch;
            self.start_of_epoch = current_time;
            self.w_reno = CWIN_MINIMUM as f64;
            path_x.cwin = CWIN_MINIMUM;
        } else if notification == CongestionNotification::Timeout {
            path_x.cwin = CWIN_MINIMUM;
            self.previous_start_of_epoch = self.start_of_epoch;
            self.start_of_epoch = current_time;
            self.alg_state = CubicAlgState::SlowStart;
        } else {
            // Enter congestion avoidance immediately.
            self.enter_avoidance(current_time);
            let w_cubic = self.w_cubic(current_time);
            let win_cubic = (w_cubic * path_x.send_mtu as f64) as u64;
            self.w_reno = path_x.cwin as f64 / 2.0;
            // See comment in cubic.c:195-200: w_cubic >= w_reno holds for
            // CUBIC_BETA_ECN > 0.618, so we pick win_cubic unconditionally.
            path_x.cwin = win_cubic;
        }
    }
```

## Pair `picoquic/cubic.c:dcubic_notify`
C: `picoquic/cubic.c:458-548 dcubic_notify`
Rust: `rs/fq/src/cubic.rs:474-605 dcubic_notify`

### C body
```c
{
    picoquic_cubic_state_t* cubic_state = (picoquic_cubic_state_t*)path_x->congestion_alg_state;
    path_x->is_cc_data_updated = 1;
    if (cubic_state != NULL) {
        switch (notification) {
            case picoquic_congestion_notification_repeat:
            case picoquic_congestion_notification_timeout:
                switch (cubic_state->alg_state) {
                    case picoquic_cubic_alg_slow_start:
                        /* In contrast to Cubic, only exit on high losses */
                        if (picoquic_cc_hystart_loss_test(&cubic_state->rtt_filter, notification, ack_state->lost_packet_number, PICOQUIC_SMOOTHED_LOSS_THRESHOLD)) {
                            dcubic_exit_slow_start(cnx, path_x, notification, cubic_state, current_time);
                        }
                        break;
                    case picoquic_cubic_alg_recovery:
                        break;
                    case picoquic_cubic_alg_congestion_avoidance:
                        /* In contrast to Cubic, only exit on high losses */
                        if (picoquic_cc_hystart_loss_test(&cubic_state->rtt_filter, notification, ack_state->lost_packet_number, PICOQUIC_SMOOTHED_LOSS_THRESHOLD) &&
                            ack_state->lost_packet_number > cubic_state->recovery_sequence) {
                            /* re-enter recovery */
                            cubic_enter_recovery(cnx, path_x, notification, cubic_state, current_time);
                        }
                        break;
                }
                break;
            case picoquic_congestion_notification_rtt_measurement:
                switch (cubic_state->alg_state) {
                    case picoquic_cubic_alg_slow_start:
                        /* if in slow start, increase the window for long delay RTT */
                        if (path_x->rtt_min > PICOQUIC_TARGET_RENO_RTT && cubic_state->ssthresh == UINT64_MAX) {
                            path_x->cwin = picoquic_cc_update_cwin_for_long_rtt(path_x);
                        }

                        /* HyStart. */
                        /* Using RTT increases as congestion signal. This is used
                         * for getting out of slow start, but also for ending a cycle
                         * during congestion avoidance */
                        if (picoquic_cc_hystart_test(&cubic_state->rtt_filter, (cnx->is_time_stamp_enabled) ? ack_state->one_way_delay : ack_state->rtt_measurement,
                            cnx->path[0]->pacing.packet_time_microsec, current_time, cnx->is_time_stamp_enabled)) {
                            dcubic_exit_slow_start(cnx, path_x, notification, cubic_state, current_time);
                        }
                        break;
                    case picoquic_cubic_alg_recovery:
                        /* if in slow start, increase the window for long delay RTT */
                        if (path_x->rtt_min > PICOQUIC_TARGET_RENO_RTT && cubic_state->ssthresh == UINT64_MAX) {
                            path_x->cwin = picoquic_cc_update_cwin_for_long_rtt(path_x);
                        }
                        /* continue */
                    case picoquic_cubic_alg_congestion_avoidance:
                        /* Using RTT increases as congestion signal. This is used
                         * for getting out of slow start, but also for ending a cycle
                         * during congestion avoidance */
                        if (picoquic_cc_hystart_test(&cubic_state->rtt_filter, (cnx->is_time_stamp_enabled) ? ack_state->one_way_delay : ack_state->rtt_measurement,
                                cnx->path[0]->pacing.packet_time_microsec, current_time, cnx->is_time_stamp_enabled)) {
                            if (current_time - cubic_state->start_of_epoch > path_x->smoothed_rtt ||
                                cubic_state->recovery_sequence <= picoquic_cc_get_ack_number(cnx, path_x)) {
                                /* re-enter recovery */
                                cubic_enter_recovery(cnx, path_x, notification, cubic_state, current_time);
                            }
                        }
                        break;
                }
                break;
            case picoquic_congestion_notification_spurious_repeat:
                /* In contrast to Cubic, do nothing here */
                break;
            case picoquic_congestion_notification_ecn_ec:
                /* In contrast to Cubic, do nothing here */
                break;
            default:
                cubic_notify(cnx, path_x, notification, ack_state, current_time);
                /* return immediately to avoid calculation of pacing rate twice. */
                return;
        }

        /* Compute pacing data */
        picoquic_update_pacing_data(path_x, 
            cubic_state->alg_state == picoquic_cubic_alg_slow_start && cubic_state->ssthresh == UINT64_MAX);
    }
}
```

### Rust body
```rust
    ) {
        use crate::Instant;

        path_x.is_cc_data_updated = true;

        match notification {
            CongestionNotification::Repeat | CongestionNotification::Timeout => {
                match self.alg_state {
                    CubicAlgState::SlowStart => {
                        // In contrast to Cubic, only exit on high losses.
                        if self.rtt_filter.hystart_loss_test(
                            notification,
                            ack_state.lost_packet_number,
                            SMOOTHED_LOSS_THRESHOLD,
                        ) {
                            self.dcubic_exit_slow_start(
                                connection,
                                path_x,
                                notification,
                                current_time,
                            );
                        }
                    }
                    CubicAlgState::Recovery => {
                        // Do nothing in recovery.
                    }
                    CubicAlgState::CongestionAvoidance => {
                        // In contrast to Cubic, only exit on high losses.
                        if self.rtt_filter.hystart_loss_test(
                            notification,
                            ack_state.lost_packet_number,
                            SMOOTHED_LOSS_THRESHOLD,
                        ) && ack_state.lost_packet_number > self.recovery_sequence
                        {
                            self.enter_recovery(connection, path_x, notification, current_time);
                        }
                    }
                }
            }

            CongestionNotification::RttMeasurement => {
                match self.alg_state {
                    CubicAlgState::SlowStart => {
                        // Increase window for long-delay RTT if still in
                        // the unconstrained phase.
                        if path_x.rtt_min > TARGET_RENO_RTT && self.ssthresh == u64::MAX {
                            path_x.cwin = path_x.update_cwin_for_long_rtt();
                        }
                        // HyStart RTT-based exit test.
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
                            self.dcubic_exit_slow_start(
                                connection,
                                path_x,
                                notification,
                                current_time,
                            );
                        }
                    }
                    // Recovery falls through to the same hystart logic as
                    // CongestionAvoidance (C `/* continue */` fall-through).
                    CubicAlgState::Recovery | CubicAlgState::CongestionAvoidance => {
                        if matches!(self.alg_state, CubicAlgState::Recovery)
                            && path_x.rtt_min > TARGET_RENO_RTT
                            && self.ssthresh == u64::MAX
                        {
                            path_x.cwin = path_x.update_cwin_for_long_rtt();
                        }
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
                        ) && (current_time.wrapping_sub(self.start_of_epoch)
                            > path_x.smoothed_rtt.ticks()
                            || self.recovery_sequence <= connection.ack_number(path_x))
                        {
                            self.enter_recovery(connection, path_x, notification, current_time);
                        }
                    }
                }
            }

            CongestionNotification::SpuriousRepeat | CongestionNotification::EcnEc => {
                // In contrast to Cubic, do nothing here.
            }

            // All other notifications delegate to the regular cubic handler.
            _ => {
                self.notify(connection, path_x, notification, ack_state, current_time);
                // Return immediately to avoid calculating pacing data twice.
                return;
            }
        }

        // Update pacing data.
        let in_slow_start = self.alg_state == CubicAlgState::SlowStart && self.ssthresh == u64::MAX;
        path_x.update_pacing_data(in_slow_start as i32);
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_enqueue`
C: `picoquic/dualq_aqm.c:142-167 dualq_enqueue`
Rust: `rs/fq/src/tests/dualq_aqm.rs:117-144 dualq_enqueue`

### C body
```c
{
    /* Test limit and classify lq or cq */
    if (dualq->cq.queue_bytes + dualq->lq.queue_bytes + packet->length > dualq->limit)
    {
        /* drop packet if buffer is full */
        picoquictest_sim_link_enqueue(link, packet, 0, 1);
    }
    else {
        /* 4 : timestamp(pkt) % only needed if using the sojourn technique */
        packet->arrival_time = current_time;
        /* Packet classifier */
        if (packet->ecn_mark == PICOQUIC_ECN_ECT_1 ||
            packet->ecn_mark == PICOQUIC_ECN_CE) {
            /* Add to L4S queue */
            dualq_enqueue_queue(&dualq->lq, packet);
        }
        else
        {
            /* add to classic queue */
            dualq_enqueue_queue(&dualq->cq, packet);
        }
    }
}
```

### Rust body
```rust
fn dualq_enqueue() -> crate::Result<()> {
    let mut ctx = dualq_test_get_ctx()?;

    with_dualq(&mut ctx.link, |dualq, link| {
        for i in 0..ECN_SEQUENCE.len() {
            let packet = dualq_test_get_packet(ECN_SEQUENCE[i], 1000)?;
            let new_key = packet_key(&packet);
            let link_queue_len = link.packets.len();
            let xq = queue_for_mut(dualq, QUEUE_ID[i]);
            let old_bytes = xq.queue_bytes;
            let old_len = xq.packets.len();
            let old_front = xq.packets.front().map(packet_key);

            xq.enqueue(packet);

            check(xq.packets.len() == old_len + 1)?;
            check(xq.queue_bytes == old_bytes + 1000)?;
            check(xq.packets.back().map(packet_key) == Some(new_key))?;
            if old_len == 0 {
                check(xq.packets.front().map(packet_key) == Some(new_key))?;
            } else {
                check(xq.packets.front().map(packet_key) == old_front)?;
            }
            check(link.packets.len() == link_queue_len)?;
        }
        Ok(())
    })
}
```

## Pair `picoquic/dualq_aqm.c:dualq_dequeue_one`
C: `picoquic/dualq_aqm.c:229-280 dualq_dequeue_one`
Rust: `rs/fq/src/tests/dualq.rs:466-491 dequeue_one`

### C body
```c
{
    picoquictest_sim_packet_t* packet = NULL;
    int is_lq = 0;
    /* Couples L4S& Classic queues */
    *should_drop = 0;

    if ((packet = dualq_scheduler(dualq, &is_lq)) != NULL) {
        if (is_lq) {
            /* scheduler chose lq */
            /* Check for overload saturation */
            if (dualq->p_CL < dualq->p_Lmax) {
                dualq->pprime_L = dualq_laqm(dualq, current_time); /* Native LAQM */
                dualq->p_L = (dualq->pprime_L > dualq->p_CL) ? dualq->pprime_L : dualq->p_CL; /* Combining function */
                if (dualq_recur(&dualq->lq, dualq->pprime_L)) {
                    /* Linear marking */
                    packet->ecn_mark = PICOQUIC_ECN_CE;
                }
            }
            else {
                /* overload saturation */
                if (dualq_recur(&dualq->lq, dualq->p_C)) {
                    /* probability p_C = p'^2 */
                    /* revert to Classic drop due to overload */
                    *should_drop = 1; 
                }
                else if (dualq_recur(&dualq->lq, dualq->p_CL)) {
                    /* probability p_CL = k * p' */
                    /* linear marking of remaining packets */
                    packet->ecn_mark = PICOQUIC_ECN_CE;
                }
            }
        }
        else {
            /* probability p_C = p'^2 */
            if (dualq_recur(&dualq->cq, dualq->p_C)) {
                if (packet->ecn_mark == 0 ||
                    dualq->p_C >= dualq->p_Cmax) {
                    /* Overload disables ECN */
                    /* Packet is not marked ECN at all, just drop it */
                    *should_drop = 1; /* squared drop */
                }
                else
                {
                    /* Square marking */
                    packet->ecn_mark = PICOQUIC_ECN_CE;
                }
            }
        }
    }
    return packet;
}
```

### Rust body
```rust
    pub fn dequeue_one(&mut self, current_time: Instant) -> Option<(TestSimPacket, bool)> {
        let (mut packet, is_lq) = self.scheduler()?;
        let mut should_drop = false;

        if is_lq {
            if self.p_cl < self.p_l_max {
                self.p_prime_l = self.laqm(current_time);
                self.p_l = self.p_prime_l.max(self.p_cl);
                if Self::recur(&mut self.lq, self.p_prime_l) {
                    packet.ecn_mark = PICOQUIC_ECN_CE;
                }
            } else if Self::recur(&mut self.lq, self.p_c) {
                should_drop = true;
            } else if Self::recur(&mut self.lq, self.p_cl) {
                packet.ecn_mark = PICOQUIC_ECN_CE;
            }
        } else if Self::recur(&mut self.cq, self.p_c) {
            if packet.ecn_mark == 0 || self.p_c >= self.p_c_max {
                should_drop = true;
            } else {
                packet.ecn_mark = PICOQUIC_ECN_CE;
            }
        }

        Some((packet, should_drop))
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_admit_pending`
C: `picoquic/dualq_aqm.c:354-358 dualq_admit_pending`
Rust: `rs/fq/src/tests/dualq.rs:252-254 admit_pending`

### C body
```c
{
    dualq_state_t* dualq = (dualq_state_t*)self;
    dualq_update_it(dualq, link, current_time);
}
```

### Rust body
```rust
    fn admit_pending(&mut self, link: &mut TestSimLink, current_time: Instant) {
        self.update_it(link, current_time);
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_params_init`
C: `picoquic/dualq_aqm.c:396-452 dualq_params_init`
Rust: `rs/fq/src/tests/dualq.rs:396-403 params_init`

### C body
```c
{
    /* Set input parameter defaults */
    /* DualQ Coupled framework parameters */
    if (link->queue_delay_max <= link->microsec_latency || link->picosec_per_byte == 0) {
        dualq->limit = PICOQUIC_BYTES_FROM_RATE(250ull, DUALQ_MAX_LINK_RATE); /* Dual buffer size */
    }
    else {
        uint64_t queue_delay = link->queue_delay_max - link->microsec_latency;
        dualq->limit = (queue_delay * 1000000) / link->picosec_per_byte;
    }
    dualq->k = 2.0; /* Coupling factor */
    /* NOT SHOWN % scheduler - dependent weight or equival't parameter */
    /* PI2 Classic AQM parameters */
    dualq->target = 15000; /* Queue delay target for Classic queue, microseconds */
    uint64_t RTT_max = 100000;  /* Worst case RTT expected, microseconds */
    /* PI2 constants derived from above PI2 parameters */
    dualq->p_Cmax = 1.0 / (dualq->k * dualq->k);
    if (dualq->p_Cmax > 1.0) {
        dualq->p_Cmax = 1;
    }
    /* PI sampling interval */
    dualq->Tupdate = RTT_max / 3;
    if (dualq->Tupdate > dualq->target) {
        dualq->Tupdate = dualq->target;
    }
    /* PI coefficients */
    /* The spec says Hz, we measure in 1/us because times are in microseconds */
    dualq->pi2_alpha = (0.1 * (double)dualq->Tupdate) / ((double)RTT_max * (double)RTT_max);
    dualq->pi2_beta = (0.3) / RTT_max; /* PI proportional gain in Hz */
    /* L4S ramp AQM parameters */
    dualq->minTh = 800; /* L4S min marking threshold in micros seconds */
    if (l4s_max == 0) {
        dualq->maxTh = 1200;
        dualq->minTh = 800;
    }
    else {
        dualq->maxTh = l4s_max;
        if (l4s_max > 1200) {
            dualq->minTh = 800;
        }
        else {
            dualq->minTh = l4s_max / 3;
        }
    }
    dualq->range = dualq->maxTh - dualq->minTh; /* Range of L4S ramp in time units */

    /* 19 : Th_len = 1 pkt % Min L4S marking threshold in packets */
    /* L4S constants */
    dualq->p_Lmax = 1.0; /* Max L4S marking prob */
}
```

### Rust body
```rust
        } else {
            let queue_delay = link.queue_delay_max - link.microsec_latency;
            (queue_delay * 1_000_000) / link.picosec_per_byte
        };
```

## Pair `picoquic/ech.c:picoquic_ech_read_config`
C: `picoquic/ech.c:95-124 picoquic_ech_read_config`
Rust: `rs/fq/src/lib.rs:4749-4757 ech_read_config`

### C body
```c
{
    int ret = 0;
    char buffer[1024];
    FILE* F = picoquic_file_open(config_file_name, "r");
    if (F == NULL) {
        ret = -1;
    }
    else {
        ptls_base64_decode_state_t d_state;
        ptls_base64_decode_init(&d_state);
        while (fgets(buffer, sizeof(buffer), F) != NULL) {
            ret = ptls_base64_decode(buffer, &d_state, config);
        }
        if (d_state.status == PTLS_BASE64_DECODE_DONE || (d_state.status == PTLS_BASE64_DECODE_IN_PROGRESS && d_state.nbc == 0)) {
            ret = 0;
        }
        else {
            ret = PTLS_ERROR_INCORRECT_BASE64;
        }
        F=picoquic_file_close(F);
        if (ret == 0) {
            DBG_PRINTF("Got %zu bytes from %s", config->off, config_file_name);
        }
    }
    return ret;
}
```

### Rust body
```rust
pub fn ech_save_config(config: &[u8], ech_config_file: &str) -> Result<(), Error> {
    crate::ech::ech_save_config_file(config, ech_config_file)
}
```

## Pair `picoquic/ech.c:picoquic_ech_configure_quic_ctx`
C: `picoquic/ech.c:333-366 picoquic_ech_configure_quic_ctx`
Rust: `rs/fq/src/ech.rs:941-957 picoquic_ech_configure_quic_ctx`

### C body
```c
{
    int ret = 0;
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;
    PICOQUIC_THREAD_CHECK(quic);

    picoquic_release_quic_ech_ctx(quic);
    ctx->ech.client.ciphers = picoquic_hpke_cipher_suites;
    ctx->ech.client.kems = picoquic_hpke_kems;
    if (private_key_file != NULL) {
        ech_opener_callback_t* ech_cb = NULL;
        if ((ret = ech_init_opener_callback(&ech_cb, private_key_file, config_file_name)) == 0) {
            ctx->ech.server.create_opener = &ech_cb->super;
            ctx->ech.server.retry_configs.base = ech_cb->config.base;
            ctx->ech.server.retry_configs.len = ech_cb->config.off;
        }
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    picoquic_release_quic_ech_ctx(quic);
    quic.ech_client_enabled = true;

    if let Some(private_key_file) = private_key_file {
        let config_file_name = config_file_name.ok_or(Error::InvalidArgument)?;
        let opener = ech_init_opener(private_key_file, config_file_name)?;
        quic.ech_server_retry_config = Some(opener.config.clone());
        quic.ech_opener = Some(opener);
    }

    Ok(())
}
```

## Pair `picoquic/ech.c:picoquic_parse_public_key_asn1`
C: `picoquic/ech.c:502-593 picoquic_parse_public_key_asn1`
Rust: `rs/fq/src/ech.rs:513-549 parse_public_key_asn1`

### C body
```c
{
    uint8_t* bytes = public_key_asn1.base;
    size_t bytes_max = public_key_asn1.len;

    /* read the ASN1 messages */
    size_t byte_index = 0;
    uint32_t seq0_length = 0;
    size_t last_byte0;
    uint32_t seq1_length = 0;
    size_t last_byte1 = 0;
    uint32_t oid_length;
    size_t last_oid_byte;
    uint32_t key_data_length;
    size_t key_data_last;

    /* start with sequence */
    byte_index = ptls_asn1_get_expected_type_and_length(bytes, bytes_max, byte_index, 0x30, &seq0_length, NULL, &last_byte0,
        decode_error, log_ctx);

    if (*decode_error == 0 && bytes_max != last_byte0) {
        byte_index = ptls_asn1_error_message("Length larger than message", bytes_max, byte_index, 0, log_ctx);
        *decode_error = PTLS_ERROR_BER_EXCESSIVE_LENGTH;
    }

    if (*decode_error == 0) {
        /* open embedded sequence */
        byte_index = ptls_asn1_get_expected_type_and_length(bytes, bytes_max, byte_index, 0x30, &seq1_length, NULL, &last_byte1,
            decode_error, log_ctx);
    }

    if (*decode_error == 0) {
        if (log_ctx != NULL) {
            log_ctx->fn(log_ctx->ctx, "   Algorithm Identifier:\n");
        }
        /* get length of OID */
        byte_index = ptls_asn1_get_expected_type_and_length(bytes, last_byte1, byte_index, 0x06, &oid_length, NULL, &last_oid_byte,
            decode_error, log_ctx);

        if (*decode_error == 0) {
            if (log_ctx != NULL) {
                /* print the OID value */
                log_ctx->fn(log_ctx->ctx, "      Algorithm:");
                ptls_asn1_dump_content(bytes + byte_index, oid_length, 0, log_ctx);
                log_ctx->fn(log_ctx->ctx, ",\n");
            }
            public_key_algo->base = bytes + byte_index;
            public_key_algo->len = oid_length;
            byte_index += oid_length;
        }
    }

    if (*decode_error == 0) {
        /* get parameters, ANY */
        if (log_ctx != NULL) {
            log_ctx->fn(log_ctx->ctx, "      Parameters:\n");
        }

        public_key_param->base = bytes + byte_index;
        if (last_byte1 <= byte_index) {
            public_key_param->len = 0;
        }
        else {
            public_key_param->len = last_byte1 - byte_index;
        }
        byte_index = last_byte1;
    }

    /* get bit string, key */
    if (*decode_error == 0) {
        byte_index = ptls_asn1_get_expected_type_and_length(bytes, last_byte0, byte_index, 0x03, &key_data_length, NULL,
            &key_data_last, decode_error, log_ctx);
        if (*decode_error == 0) {
            public_key_bit_string->base = bytes + byte_index;
            public_key_bit_string->len = key_data_length;
            byte_index += key_data_length;
        }
    }

    if (*decode_error == 0 && byte_index != last_byte0) {
        byte_index = ptls_asn1_error_message("Length larger than element", bytes_max, byte_index, 0, log_ctx);
        *decode_error = PTLS_ERROR_BER_ELEMENT_TOO_SHORT;
    }

    if (log_ctx != NULL) {
        log_ctx->fn(log_ctx->ctx, "\n");
    }
    return byte_index;
}
```

### Rust body
```rust
pub fn parse_public_key_asn1(input: &[u8]) -> Result<PublicKeyAsn1<'_>, Error> {
    let bytes_max = input.len();

    // Outer SEQUENCE must span the entire buffer.
    let (outer_val, outer_end) = asn1_tlv(input, bytes_max, 0, 0x30)?;
    if outer_end != bytes_max {
        return Err(Error::Generic); // PTLS_ERROR_BER_EXCESSIVE_LENGTH
    }

    // AlgorithmIdentifier SEQUENCE.
    let (inner_val, inner_end) = asn1_tlv(input, outer_end, outer_val, 0x30)?;

    // Algorithm OID — value bytes only (tag + length consumed by asn1_tlv).
    let (oid_val, oid_end) = asn1_tlv(input, inner_end, inner_val, 0x06)?;
    let algo = &input[oid_val..oid_end];

    // Parameters: everything remaining in the inner SEQUENCE after the OID.
    let param = if inner_end <= oid_end {
        &input[0..0]
    } else {
        &input[oid_end..inner_end]
    };

    // BIT STRING (public key), must consume the rest of the outer SEQUENCE.
    let (bs_val, bs_end) = asn1_tlv(input, outer_end, inner_end, 0x03)?;
    if bs_end != outer_end {
        return Err(Error::Generic); // PTLS_ERROR_BER_ELEMENT_TOO_SHORT
    }
    let bit_string = &input[bs_val..bs_end];

    Ok(PublicKeyAsn1 {
        algo,
        param,
        bit_string,
        consumed: bs_end,
    })
}
```

## Pair `picoquic/ech.c:ech_config_id_from_config`
C: `picoquic/ech.c:738-759 ech_config_id_from_config`
Rust: `rs/fq/src/ech.rs:226-250 ech_config_id_from_config`

### C body
```c
{
    /* compute the config ID */
    uint64_t config_sum = 0;
    uint8_t config_id = 0;
    for (size_t i = 0; i < public_key_bits.len; i++) {
        config_sum += public_key_bits.base[i];
    }
    config_sum += kem->id;
    for (size_t i = 0; cipher_vec[i] != NULL; i++) {
        config_sum += cipher_vec[i]->id.aead;
        config_sum += cipher_vec[i]->id.kdf;
    }
    for (size_t i = 0; public_name[i] != 0; i++) {
        config_sum += (uint8_t)public_name[i];
    }
    while (config_sum > 0) {
        config_id ^= (uint8_t)(config_sum & 0xff);
        config_sum >>= 8;
    }
    return config_id;
}
```

### Rust body
```rust
) -> u8 {
    let mut config_sum: u64 = 0;
    for &b in public_key_bits {
        config_sum = config_sum.wrapping_add(b as u64);
    }
    config_sum = config_sum.wrapping_add(kem_id as u64);
    for cs in cipher_suites {
        config_sum = config_sum.wrapping_add(cs.aead as u64);
        config_sum = config_sum.wrapping_add(cs.kdf as u64);
    }
    for b in public_name.bytes() {
        config_sum = config_sum.wrapping_add(b as u64);
    }
    let mut config_id: u8 = 0;
    while config_sum > 0 {
        config_id ^= (config_sum & 0xff) as u8;
        config_sum >>= 8;
    }
    config_id
}
```

## Pair `picoquic/ech.c:picoquic_ech_create_config_from_binary`
C: `picoquic/ech.c:844-855 picoquic_ech_create_config_from_binary`
Rust: `rs/fq/src/ech.rs:1030-1036 ech_create_config_from_binary`

### C body
```c
{
    int ret = 0;
    ptls_buffer_t config_buf;
    ptls_buffer_init(&config_buf, "", 0);

    if ((ret = picoquic_ech_create_rr_from_binary(&config_buf, public_key_asn1, public_name)) == 0) {
        ret = picoquic_ech_create_config_list_from_config(&config_buf, config, config_len);
    }
    ptls_buffer_dispose(&config_buf);
    return ret;
}
```

### Rust body
```rust
) -> Result<Vec<u8>, Error> {
    let config = ech_create_rr_from_binary(public_key_asn1, public_name)?;
    ech_create_config_list_from_config(&config)
}
```

## Pair `picoquic/ech.c:picoquic_ech_get_retry_config`
C: `picoquic/ech.c:1067-1075 picoquic_ech_get_retry_config`
Rust: `rs/fq/src/lib.rs:4731-4733 ech_retry_config`

### C body
```c
{
    picoquic_tls_ctx_t* tls_ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    *retry_config = tls_ctx->retry_configs.base;
    *retry_config_len = tls_ctx->retry_configs.len;
}
```

### Rust body
```rust
    pub fn ech_retry_config(&self) -> &[u8] {
        self.tls_ctx.as_ref().map_or(&[], |s| s.retry_configs())
```
