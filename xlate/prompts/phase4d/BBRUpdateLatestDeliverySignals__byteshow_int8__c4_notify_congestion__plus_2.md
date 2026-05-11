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

## `picoquic/bbr.c:BBRUpdateLatestDeliverySignals`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust uses wrapping_sub for prior_delivered where C uses unsigned subtraction; this is likely equivalent for unsigned wrap but visibly relies on explicit wrapping behavior.
* C source: `picoquic/bbr.c:985-1004`
* C signature: `void BBRUpdateLatestDeliverySignals(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:754-767`
* Rust item: `update_latest_delivery_signals`

### C body
```c
{
    /* BBR.bw_latest = max(BBR.bw_latest, rs.delivery_rate) */
    bbr_state->loss_round_start = 0;
    if (bbr_state->bw_latest < rs->delivery_rate) {
        bbr_state->bw_latest = rs->delivery_rate;
    }
    /* BBR.inflight_latest = max(BBR.inflight_latest, rs.delivered) */
    if (bbr_state->inflight_latest < rs->delivered) {
        bbr_state->inflight_latest = rs->delivered;
    }
    
    uint64_t prior_delivered = path_x->delivered - rs->delivered;
    if (prior_delivered >= bbr_state->loss_round_delivered) {
        bbr_state->loss_round_delivered = path_x->delivered;
        bbr_state->loss_round_start = 1;
    }
}
```

### Rust body
```rust
    pub fn update_latest_delivery_signals(&mut self, path_x: &Path, rs: &BbrPerAckState) {
        self.loss_round_start = false;
        if self.bw_latest < rs.delivery_rate {
            self.bw_latest = rs.delivery_rate;
        }
        if self.inflight_latest < rs.delivered {
            self.inflight_latest = rs.delivered;
        }
        let prior_delivered = path_x.delivered.wrapping_sub(rs.delivered);
        if prior_delivered >= self.loss_round_delivered {
            self.loss_round_delivered = path_x.delivered;
            self.loss_round_start = true;
        }
    }
```

## `picoquic/bytestream.c:byteshow_int8`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust snippet only shows the bounds check and error return; the visible body does not show assigning the byte to the output or success return.
* C source: `picoquic/bytestream.c:199-208`
* C signature: `int byteshow_int8(bytestream *, uint8_t *)`
* Rust source: `rs/fq/src/bytestream.rs:273-276`
* Rust item: `peek_u8`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 1) {
        return -1;
    } else {
        *value = s->data[s->ptr];
        return 0;
    }
}
```

### Rust body
```rust
        if self.ptr >= self.data_ref().len() {
            return Err(Error::BufferTooSmall);
        }
```

## `picoquic/c4.c:c4_notify_congestion`
* Phase 4C status: `suspect`
* Phase 4C rationale: Bodies mostly match, but Rust uses saturating_sub and checked arithmetic/division for ECN and delay beta while C performs direct unsigned arithmetic and division; visible edge behavior can differ.
* C source: `picoquic/c4.c:885-968`
* C signature: `void c4_notify_congestion(picoquic_path_t *, c4_state_t *, c4_congestion_t)`
* Rust source: `rs/fq/src/c4.rs:553-619`
* Rust item: `notify_congestion`

### C body
```c
{
    uint64_t beta = C4_BETA_LOSS_1024;
    c4_state->congestion_notified = 1;

    if (c_mode == c4_congestion_loss) {
        /* Make amount of slow down function of sensitivity,
        * for better fairness between C4 connections.
        */
        beta = (C4_BETA_LOSS_1024 + MULT1024(c4_sensitivity_1024(c4_state), C4_BETA_LOSS_1024))/2;
    }
    else if (c_mode == c4_congestion_ecn) {
        /* Apply proportional reduction.
         * The threshold value is a function of sensitivity, and thus the reduction
         * incorporates a sensitivity factor. There is no obvious need for
         * additional sensitivity related factors.
         */
        beta = (c4_state->ecn_alpha - c4_state->ecn_threshold) * 1024 / c4_state->ecn_threshold;
        if (beta > C4_BETA_LOSS_1024) {
            /* capping beta to the standard 1/4th. */
            beta = C4_BETA_LOSS_1024;
        }
    }
    
    if (c_mode == c4_congestion_delay) {
        /* TODO: we should really use bytes in flight! */
        beta = c4_state->recent_delay_excess*1024/c4_state->delay_threshold;

        if (beta > C4_BETA_LOSS_1024) {
            /* capping beta to the standard 1/4th. */
            beta = C4_BETA_LOSS_1024;
        }
    }
    else {
        /* Clear the excess delay to avoid spurious delay measurements */
        c4_state->recent_delay_excess = 0;
    }


    if (c4_state->alg_state == c4_recovery) {
        if (c4_state->alpha_1024_current == C4_ALPHA_RECOVER_1024) {
            /* Congestion notification after entering recovery 
             * indicates that queues are building up. It is thus
             * prudent to decrease "alpha_current" and to spend a bit
             * more time in recovery, to reduce these queues. */
            c4_state->alpha_1024_current = C4_ALPHA_RECOVER2_1024;
            c4_state->era_sequence = picoquic_cc_get_sequence_number(path_x->cnx, path_x);
            C4_LOGGER(path_x, 0, c4_state, NULL, beta, c_mode);
        }
        if (c_mode == c4_congestion_ecn) {
            c4_state->excess_ce_after_push = 1;
        }
    }
    else
    {
        if (c4_state->alg_state != c4_pushing) {
            c4_state->nominal_rate -= MULT1024(beta, c4_state->nominal_rate);
            if (c_mode == c4_congestion_loss) {
                c4_state->nominal_max_rtt -= MULT1024(beta, c4_state->nominal_max_rtt);
                if (c4_state->nominal_max_rtt < C4_MAX_RTT_MIN) {
                    c4_state->nominal_max_rtt = C4_MAX_RTT_MIN;
                }
                c4_state->delay_threshold = c4_delay_threshold(c4_state);
            }
            C4_LOGGER(path_x, 0, c4_state, NULL, beta, c_mode);
        }
        c4_enter_recovery(path_x, c4_state, c_mode);
    }

    c4_apply_rate_and_cwin(path_x, c4_state);

    path_x->is_ssthresh_initialized = 1;
}
```

### Rust body
```rust
    ) {
        let mut beta = C4_BETA_LOSS_1024;
        self.congestion_notified = true;

        if c_mode == C4Congestion::Loss {
            beta = (C4_BETA_LOSS_1024 + mult1024(self.sensitivity_1024(), C4_BETA_LOSS_1024)) / 2;
        } else if c_mode == C4Congestion::Ecn {
            if let Some(beta_ecn) = self
                .ecn_alpha
                .saturating_sub(self.ecn_threshold)
                .checked_mul(1024)
                .and_then(|v| v.checked_div(self.ecn_threshold))
            {
                beta = beta_ecn;
            }
            if beta > C4_BETA_LOSS_1024 {
                beta = C4_BETA_LOSS_1024;
            }
        }

        if c_mode == C4Congestion::Delay {
            if let Some(beta_delay) = self
                .recent_delay_excess
                .checked_mul(1024)
                .and_then(|v| v.checked_div(self.delay_threshold))
            {
                beta = beta_delay;
            }
            if beta > C4_BETA_LOSS_1024 {
                beta = C4_BETA_LOSS_1024;
            }
        } else {
            self.recent_delay_excess = 0;
        }

        if self.alg_state == C4AlgState::Recovery {
            if self.alpha_1024_current == C4_ALPHA_RECOVER_1024 {
                self.alpha_1024_current = C4_ALPHA_RECOVER2_1024;
                self.era_sequence = connection.sequence_number(path_x);
                c4_logger(path_x, 0, self, None, beta, c_mode);
            }
            if c_mode == C4Congestion::Ecn {
                self.excess_ce_after_push = true;
            }
        } else {
            if self.alg_state != C4AlgState::Pushing {
                self.nominal_rate -= mult1024(beta, self.nominal_rate);
                if c_mode == C4Congestion::Loss {
                    self.nominal_max_rtt -= mult1024(beta, self.nominal_max_rtt);
                    if self.nominal_max_rtt < C4_MAX_RTT_MIN {
                        self.nominal_max_rtt = C4_MAX_RTT_MIN;
                    }
                    self.delay_threshold = self.delay_threshold();
                }
                c4_logger(path_x, 0, self, None, beta, c_mode);
            }
            self.enter_recovery(path_x, connection, c_mode);
        }

        self.apply_rate_and_cwin(path_x);
        path_x.is_ssthresh_initialized = true;
    }
```

## `picoquic/config.c:config_set_option`
* Phase 4C status: `suspect`
* Phase 4C rationale: Mostly similar option handling, but Rust EchServer treats the second parameter as optional while C calls config_set_string_param for parameter 1 after setting parameter 0.
* C source: `picoquic/config.c:274-553`
* C signature: `int config_set_option(option_table_line_t *, option_param_t *, int, picoquic_quic_config_t *)`
* Rust source: `rs/fq/src/config.rs:751-1026`
* Rust item: `apply_option`

### C body
```c
{
    int ret = 0;
    char opval_buffer[256];

    switch (option_desc->option_num) {
    case picoquic_option_CERT:
        ret = config_set_string_param(&config->server_cert_file, params, nb_params, 0);
        break;
    case picoquic_option_KEY:
        ret = config_set_string_param(&config->server_key_file, params, nb_params, 0);
        break;
    case picoquic_option_SERVER_PORT:
        ret = config_set_port(config, params->param);
        if (ret != 0) {
            fprintf(stderr, "Invalid port: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
        }
        break;
    case picoquic_option_PROPOSED_VERSION:
        if ((config->proposed_version = config_parse_target_version(config_optval_param_string(opval_buffer, 256, params, nb_params, 0))) <= 0) {
            fprintf(stderr, "Invalid version: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = -1;
        }
        break;
    case picoquic_option_OUTDIR:
        ret = config_set_string_param(&config->out_dir, params, nb_params, 0);
        break;
    case picoquic_option_WWWDIR:
        ret = config_set_string_param(&config->www_dir, params, nb_params, 0);
        break;
    case picoquic_option_MAX_CONNECTIONS: {
        int v = config_atoi(params, nb_params, 0, &ret);
        if (ret != 0 || v <= 0 ) {
            fprintf(stderr, "Invalid max connections: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = (ret == 0) ? -1 : ret;
        }
        else {
            config->nb_connections = (uint32_t)v;
        }
        break;
    }
    case picoquic_option_DO_RETRY:
        config->do_retry = 1;
        break;
    case picoquic_option_INITIAL_RANDOM: {
        int v = config_atoi(params, nb_params, 0, &ret);
        if (ret != 0 || v < 0 || v > 2) {
            fprintf(stderr, "Invalid initial random value: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = (ret == 0) ? -1 : ret;
        }
        else {
            config->initial_random = v;
        }
        break;
    }
    case picoquic_option_RESET_SEED:
        config->has_reset_seed = 1;
        ret = (picoquic_parse_hexa(params[0].param, params[0].length, config->reset_seed, sizeof(config->reset_seed)) ==
            sizeof(config->reset_seed)) ? 0 : -1;
        if (ret != 0) {
            fprintf(stderr, "Invalid reset seed: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
        }
        break;
    case picoquic_option_DisablePortBlocking:
        config->disable_port_blocking = 1;
        break;
#ifndef PICOQUIC_WITHOUT_SSLKEYLOG
    case picoquic_option_SSLKEYLOG:
        config->enable_sslkeylog = 1;
        break;
#endif
    case picoquic_option_SOLUTION_DIR:
        ret = config_set_string_param(&config->solution_dir, params, nb_params, 0);
        break;
    case picoquic_option_CC_ALGO:
        ret = config_set_string_param(&config->cc_algo_id, params, nb_params, 0);
        break;
    case picoquic_option_CC_OPTION:
        ret = config_set_string_param(&config->cc_algo_option_string, params, nb_params, 0);
        break;
    case picoquic_option_SPINBIT: {
        int v = config_atoi(params, nb_params, 0, &ret);
        if (ret != 0 || v < 0 || v > (int)picoquic_spinbit_on) {
            fprintf(stderr, "Invalid spinbit policy: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = (ret == 0) ? -1 : ret;
        }
        else {
            config->spinbit_policy = (picoquic_spinbit_version_enum)v;
        }
        break;
    }
    case picoquic_option_LOSSBIT: {
        int v = config_atoi(params, nb_params, 0, &ret);
        if (ret != 0 || v < 0 || v >(int)picoquic_lossbit_send_receive) {
            fprintf(stderr, "Invalid lossbit policy: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = (ret == 0) ? -1 : ret;
        }
        else {
            config->lossbit_policy = (picoquic_lossbit_version_enum)v;
        }
        break;
    }
    case picoquic_option_MULTIPATH: 
        config->multipath_option = 1;
        break;
    case picoquic_option_DEST_IF:
        config->dest_if = config_atoi(params, nb_params, 0, &ret);
        break;
    case picoquic_option_CIPHER_SUITE:
        config->cipher_suite_id = config_atoi(params, nb_params, 0, &ret);
        break;
    case picoquic_option_INIT_CNXID:
        ret = config_set_string_param(&config->cnx_id_cbdata, params, nb_params, 0);
        break;
    case picoquic_option_LOG_FILE:
        ret = config_set_string_param(&config->log_file, params, nb_params, 0);
        break;
    case picoquic_option_LONG_LOG:
        config->use_long_log = 1;
        break;
    case picoquic_option_BINLOG_DIR:
        ret = config_set_string_param(&config->bin_dir, params, nb_params, 0);
        break;
    case picoquic_option_QLOG_DIR:
        ret = config_set_string_param(&config->qlog_dir, params, nb_params, 0);
        break;
    case picoquic_option_MTU_MAX:
        config->mtu_max = config_atoi(params, nb_params, 0, &ret);
        if (config->mtu_max <= 0 || config->mtu_max > PICOQUIC_MAX_PACKET_SIZE) {
            fprintf(stderr, "Invalid max mtu: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = -1;
        }
        break;
    case picoquic_option_SNI:
        ret = config_set_string_param(&config->sni, params, nb_params, 0);
        break;
    case picoquic_option_ALPN:
        ret = config_set_string_param(&config->alpn, params, nb_params, 0);
        break;
    case picoquic_option_ROOT_TRUST_FILE:
        ret = config_set_string_param(&config->root_trust_file, params, nb_params, 0);
        break;
    case picoquic_option_FORCE_ZERO_SHARE:
        config->force_zero_share = 1;
        break;
    case picoquic_option_CNXID_LENGTH:
        config->cnx_id_length = config_atoi(params, nb_params, 0, &ret);
        if (config->cnx_id_length < 0 || config->cnx_id_length > PICOQUIC_CONNECTION_ID_MAX_SIZE) {
            fprintf(stderr, "Invalid connection id length: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = -1;
        }
        break;
    case picoquic_option_Idle_Timeout:
        config->idle_timeout = config_atoi(params, nb_params, 0, &ret);
        if (config->idle_timeout < 0) {
            fprintf(stderr, "Invalid idle timer: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = -1;
        }
        break;
    case picoquic_option_NO_DISK:
        config->no_disk = 1;
        break;
    case picoquic_option_LARGE_CLIENT_HELLO:
        config->large_client_hello = 1;
        break;
    case picoquic_option_Ticket_File_Name:
        ret = config_set_string_param(&config->ticket_file_name, params, nb_params, 0);
        break;
    case picoquic_option_Token_File_Name:
        ret = config_set_string_param(&config->token_file_name, params, nb_params, 0);
        break;
    case picoquic_option_Socket_buffer_size:
        config->socket_buffer_size = config_atoi(params, nb_params, 0, &ret);
        if (config->socket_buffer_size < 0 ) {
            fprintf(stderr, "Invalid socket_buffer_size: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = -1;
        }
        break;
    case picoquic_option_Performance_Log:
        ret = config_set_string_param(&config->performance_log, params, nb_params, 0);
        break;
    case picoquic_option_Preemptive_Repeat:
        config->do_preemptive_repeat = 1;
        break;
    case picoquic_option_Version_Upgrade:
        if ((config->desired_version = config_parse_target_version(config_optval_param_string(opval_buffer, 256, params, nb_params, 0))) <= 0) {
            fprintf(stderr, "Invalid version: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = -1;
        }
        break;
    case picoquic_option_No_GSO:
        config->do_not_use_gso = 1;
        break;
    case picoquic_option_BDP_frame: {
        int v = config_atoi(params, nb_params, 0, &ret);
        if (ret != 0 || v < 0 || v > 1) {
            fprintf(stderr, "Invalid bdp option: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = (ret == 0) ? -1 : ret;
        }
        else {
            config->bdp_frame_option = v;
        }
        break;
    }
    case picoquic_option_CWIN_MAX:{
        int v = config_atoi(params, nb_params, 0, &ret);
        if (ret != 0 || v < 0) {
            fprintf(stderr, "Invalid cwin max option: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = (ret == 0) ? -1 : ret;
        }
        else {
            config->cwin_max = (v==0)?UINT64_MAX:v;
        }
        break;
    }
    case picoquic_option_AddressDiscovery: {
        int v = config_atoi(params, nb_params, 0, &ret);
        if (ret != 0 || v < 0 || v > 2) {
            fprintf(stderr, "Invalid address discovery option: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = (ret == 0) ? -1 : ret;
        }
        else {
            config->address_discovery_mode = v + 1;
        }
        break;
    }
    case picoquic_option_ECH_server: {
        ret = config_set_string_param(&config->ech_key_file, params, nb_params, 0);
        if (ret == 0) {
            ret = config_set_string_param(&config->ech_config_file, params, nb_params, 1);
        }
        break;
    }
    case picoquic_option_ECH_init: {
        ret = config_set_string_param(&config->ech_public_name, params, nb_params, 0);
        break;
    }
    case picoquic_option_ECH_client: {
        if (nb_params != 1) {
            ret = -1;
        }
        else if (strcmp(params[0].param, "-") == 0) {
            config->ech_target = NULL;
            config->ech_target_len = SIZE_MAX;
        }
        else{
            ret = picoquic_base64_decode(&config->ech_target, &config->ech_target_len, params[0].param);
        }
        if (ret != 0) {
            fprintf(stderr, "Incorrect base64 format: %s\n", params[0].param);
            ret = (ret == 0) ? -1 : ret;
        }
        break;
    }
    case picoquic_option_FLOW_CONTROL_MAX: {
        int v = config_atoi(params, nb_params, 0, &ret);
        if (ret != 0 || v < 0) {
            fprintf(stderr, "Invalid flow control max option: %s\n", config_optval_param_string(opval_buffer, 256, params, nb_params, 0));
            ret = (ret == 0) ? -1 : ret;
        }
        else {
            config->flow_control_max = v;
        }
        break;
    }
    case picoquic_option_Preferred_V4:
        ret = config_set_string_param(&config->preferred_address_v4, params, nb_params, 0);
        break;
    case picoquic_option_Preferred_V6:
        ret = config_set_string_param(&config->preferred_address_v6, params, nb_params, 0);
        break;
    case picoquic_option_HELP:
        ret = -1;
        break;
    default:
        ret = -1;
        break;
    }
    return ret;
}
```

### Rust body
```rust
fn apply_option(config: &mut Config, entry: &OptionEntry, params: &[&str]) -> Result<(), Error> {
    let p0 = params.first().copied();
    let p1 = params.get(1).copied();
    match entry.id {
        OptionId::Cert => {
            config.server_cert_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::Key => {
            config.server_key_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::ServerPort => {
            config.set_port(p0.ok_or(Error::InvalidArgument)?)?;
        }
        OptionId::ProposedVersion => {
            let v = parse_hex_version(p0.ok_or(Error::InvalidArgument)?);
            if v == 0 {
                return Err(Error::InvalidArgument);
            }
            config.proposed_version = v;
        }
        OptionId::OutDir => {
            config.out_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::WwwDir => {
            config.www_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::MaxConnections => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v <= 0 {
                return Err(Error::InvalidArgument);
            }
            config.nb_connections = v as u32;
        }
        OptionId::DoRetry => {
            config.do_retry = true;
        }
        OptionId::InitialRandom => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if !(0..=2).contains(&v) {
                return Err(Error::InvalidArgument);
            }
            config.initial_random = v as u32;
        }
        OptionId::ResetSeed => {
            config.has_reset_seed = true;
            let n =
                crate::utils::parse_hexa(p0.ok_or(Error::InvalidArgument)?, &mut config.reset_seed);
            if n != config.reset_seed.len() {
                return Err(Error::InvalidArgument);
            }
        }
        OptionId::DisablePortBlocking => {
            config.disable_port_blocking = true;
        }
        OptionId::SslKeyLog => {
            config.enable_sslkeylog = true;
        }
        OptionId::SolutionDir => {
            config.solution_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::CcAlgo => {
            config.cc_algo_id = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::CcOption => {
            config.cc_algo_option_string = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::Spinbit => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            config.spinbit_policy = match v {
                0 => SpinbitVersion::Basic,
                1 => SpinbitVersion::Random,
                2 => SpinbitVersion::Null,
                3 => SpinbitVersion::On,
                _ => return Err(Error::InvalidArgument),
            };
        }
        OptionId::Lossbit => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            config.lossbit_policy = match v {
                0 => LossbitVersion::None,
                1 => LossbitVersion::SendOnly,
                2 => LossbitVersion::SendReceive,
                _ => return Err(Error::InvalidArgument),
            };
        }
        OptionId::Multipath => {
            config.multipath_option = 1;
        }
        OptionId::DestIf => {
            config.dest_if = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
        }
        OptionId::CipherSuite => {
            config.cipher_suite_id = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
        }
        OptionId::InitCnxId => {
            config.connection_id_cbdata = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::LogFile => {
            config.log_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::LongLog => {
            config.use_long_log = true;
        }
        OptionId::BinlogDir => {
            config.bin_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::QlogDir => {
            config.qlog_dir = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::MtuMax => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v <= 0 || v > crate::internal::MAX_PACKET_SIZE as i32 {
                return Err(Error::InvalidArgument);
            }
            config.mtu_max = v;
        }
        OptionId::Sni => {
            config.sni = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::Alpn => {
            config.alpn = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::RootTrustFile => {
            config.root_trust_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::ForceZeroShare => {
            config.force_zero_share = true;
        }
        OptionId::CnxIdLength => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 || v > crate::CONNECTION_ID_MAX_SIZE as i32 {
                return Err(Error::InvalidArgument);
            }
            config.connection_id_length = v;
        }
        OptionId::IdleTimeout => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 {
                return Err(Error::InvalidArgument);
            }
            config.idle_timeout = v;
        }
        OptionId::NoDisk => {
            config.no_disk = true;
        }
        OptionId::LargeClientHello => {
            config.large_client_hello = true;
        }
        OptionId::TicketFileName => {
            config.ticket_file_name = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::TokenFileName => {
            config.token_file_name = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::SocketBufferSize => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 {
                return Err(Error::InvalidArgument);
            }
            config.socket_buffer_size = v;
        }
        OptionId::PerformanceLog => {
            config.performance_log = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::PreemptiveRepeat => {
            config.do_preemptive_repeat = true;
        }
        OptionId::VersionUpgrade => {
            let v = parse_hex_version(p0.ok_or(Error::InvalidArgument)?);
            if v == 0 {
                return Err(Error::InvalidArgument);
            }
            config.desired_version = v;
        }
        OptionId::NoGso => {
            config.do_not_use_gso = true;
        }
        OptionId::BdpFrame => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if !(0..=1).contains(&v) {
                return Err(Error::InvalidArgument);
            }
            config.bdp_frame_option = v;
        }
        OptionId::CwinMax => {
            let v: i64 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 {
                return Err(Error::InvalidArgument);
            }
            config.cwin_max = if v == 0 { u64::MAX } else { v as u64 };
        }
        OptionId::AddressDiscovery => {
            let v: i32 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if !(0..=2).contains(&v) {
                return Err(Error::InvalidArgument);
            }
            config.address_discovery_mode = v + 1;
        }
        OptionId::EchServer => {
            config.ech_key_file = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
            if let Some(s) = p1 {
                config.ech_config_file = Some(s.to_string());
            }
        }
        OptionId::EchInit => {
            config.ech_public_name = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::EchClient => {
            let s = p0.ok_or(Error::InvalidArgument)?;
            if s == "-" {
                config.ech_target = None;
            } else {
                config.ech_target = Some(base64_decode(s).map_err(|_| Error::InvalidArgument)?);
            }
        }
        OptionId::FlowControlMax => {
            let v: i64 = p0
                .ok_or(Error::InvalidArgument)?
                .parse()
                .map_err(|_| Error::InvalidArgument)?;
            if v < 0 {
                return Err(Error::InvalidArgument);
            }
            config.flow_control_max = v as u64;
        }
        OptionId::PreferredV4 => {
            config.preferred_address_v4 = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::PreferredV6 => {
            config.preferred_address_v6 = Some(p0.ok_or(Error::InvalidArgument)?.to_string());
        }
        OptionId::Help => {
            return Err(Error::InvalidArgument);
        }
    }
    Ok(())
}
```

## `picoquic/dualq_aqm.c:dualq_dequeue_queue`
* Phase 4C status: `suspect`
* Phase 4C rationale: C clears next_packet and resets both queue_bytes and count when empty and decrements count otherwise; Rust only updates queue_bytes visibly.
* C source: `picoquic/dualq_aqm.c:114-139`
* C signature: `picoquictest_sim_packet_t * dualq_dequeue_queue(dualq_queue_t *)`
* Rust source: `rs/fq/src/tests/dualq.rs:89-99`
* Rust item: `dequeue`

### C body
```c
{
    picoquictest_sim_packet_t* packet = xq->queue_first;

    if (packet != NULL) {
        xq->queue_first = packet->next_packet;
        packet->next_packet = NULL;
        if (xq->queue_first == NULL) {
            xq->queue_last = NULL;
            xq->queue_bytes = 0;
            xq->count = 0;
        }
        else
        {
            if (packet->length < xq->queue_bytes) {
                xq->queue_bytes -= packet->length;
            }
            else {
                /* error case. do not use 0, as that would stop dequeuing */
                xq->queue_bytes = 1;
            }
            xq->count -= 1;
        }
    }
    return packet;
}
```

### Rust body
```rust
    fn dequeue(&mut self) -> Option<TestSimPacket> {
        let packet = self.packets.pop_front()?;
        if self.packets.is_empty() {
            self.queue_bytes = 0;
        } else if (packet.length as u64) < self.queue_bytes {
            self.queue_bytes -= packet.length as u64;
        } else {
            self.queue_bytes = 1;
        }
        Some(packet)
    }
```
