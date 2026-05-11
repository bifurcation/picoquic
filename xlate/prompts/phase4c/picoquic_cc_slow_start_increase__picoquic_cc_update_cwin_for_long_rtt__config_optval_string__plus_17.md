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

## Pair `picoquic/cc_common.c:picoquic_cc_slow_start_increase`
C: `picoquic/cc_common.c:210-222 picoquic_cc_slow_start_increase`
Rust: `rs/fq/src/cc_common.rs:317-416 slow_start_increase`

### C body
```c
uint64_t picoquic_cc_slow_start_increase(picoquic_path_t * path_x, uint64_t nb_delivered) {
    /* App limited. */
    /* TODO discuss
     * path_x->cwin < path_x->bytes_in_transit returns false in cc code
     * path_x->cnx->cwin_blocked is set to true
     * (path_x->cwin < path_x->bytes_in_transit) != path_x->cnx->cwin_blocked?
     */
    if (!path_x->cnx->cwin_blocked) {
        return 0;
    }

    return nb_delivered;
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

## Pair `picoquic/cc_common.c:picoquic_cc_update_cwin_for_long_rtt`
C: `picoquic/cc_common.c:272-289 picoquic_cc_update_cwin_for_long_rtt`
Rust: `rs/fq/src/cc_common.rs:341-355 update_cwin_for_long_rtt`

### C body
```c
uint64_t picoquic_cc_update_cwin_for_long_rtt(picoquic_path_t * path_x) {
    uint64_t min_cwnd;

    if (path_x->rtt_min > PICOQUIC_TARGET_SATELLITE_RTT) {
        min_cwnd = (uint64_t)((double)PICOQUIC_CWIN_INITIAL * (double)PICOQUIC_TARGET_SATELLITE_RTT / (double)PICOQUIC_TARGET_RENO_RTT);
    }
    else {
        min_cwnd = (uint64_t)((double)PICOQUIC_CWIN_INITIAL * (double)path_x->rtt_min / (double)PICOQUIC_TARGET_RENO_RTT);
    }

    /* Return increased cwin, if larger than current cwin. */
    if (min_cwnd > path_x->cwin) {
        return min_cwnd;
    }

    /* Otherwise, return current cwin. */
    return path_x->cwin;
}
```

### Rust body
```rust
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
```

## Pair `picoquic/config.c:config_optval_string`
C: `picoquic/config.c:181-189 config_optval_string`
Rust: `rs/fq/src/config.rs:553-560 config_optval_string`

### C body
```c
{
    if (p_length + 1 > buffer_max) {
        p_length = buffer_max - 1;
    }
    memcpy(buffer, p, p_length);
    buffer[p_length] = 0;
    return buffer;
}
```

### Rust body
```rust
fn config_optval_string<'a>(buffer: &'a mut [u8], p: &[u8]) -> &'a str {
    let len = p.len().min(buffer.len().saturating_sub(1));
    buffer[..len].copy_from_slice(&p[..len]);
    if !buffer.is_empty() {
        buffer[len] = 0;
    }
    core::str::from_utf8(&buffer[..len]).unwrap_or("")
}
```

## Pair `picoquic/config.c:config_set_option`
C: `picoquic/config.c:274-553 config_set_option`
Rust: `rs/fq/src/config.rs:751-1026 apply_option`

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

## Pair `picoquic/config.c:picoquic_config_set_option`
C: `picoquic/config.c:614-639 picoquic_config_set_option`
Rust: `rs/fq/src/config.rs:1261-1268 set_option`

### C body
```c
{
    int ret = 0;
    option_table_line_t* option_desc = NULL;
    option_param_t params[1];
    int nb_params = 0;

    for (size_t i = 0; i < option_table_size; i++) {
        if (option_table[i].option_num == option_num) {
            option_desc = &option_table[i];
        }
    }
    if (option_desc == NULL) {
        fprintf(stderr, "Unknow option number: %d\n", option_num);
        ret = -1;
    }
    else{
        if (opt_val != NULL) {
            params[0].param = opt_val;
            params[0].length = strlen(opt_val);
            nb_params = 1;
        }
        ret = config_set_option(option_desc, params, nb_params, config);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn set_option(&mut self, option: OptionId, value: Option<&str>) -> Result<(), Error> {
        let entry = option_entry(option).ok_or(Error::InvalidArgument)?;
        let params: &[&str] = match value {
            Some(v) => &[v],
            None => &[],
        };
        apply_option(self, entry, params)
    }
```

## Pair `picoquic/config.c:picoquic_get_command_line_option_value`
C: `picoquic/config.c:683-720 picoquic_get_command_line_option_value`
Rust: `rs/fq/src/config.rs:714-742 picoquic_get_command_line_option_value`

### C body
```c
{
    int ret = 0;
    option_param_t params[5];
    int nb_params = 0;

    if (option_table[option_index].nb_params_required > 0) {
        params[0].param = optarg;
        if (optarg == NULL) {
            fprintf(stderr, "option %s requires %d arguments\n", opt_string, option_table[option_index].nb_params_required);
            ret = -1;
        }
        else {
            params[0].length = strlen(optarg);
            nb_params++;
            while (nb_params < option_table[option_index].nb_params_required) {
                if (*p_optind + 1 > argc) {
                    fprintf(stderr, "option %s requires %d arguments\n", opt_string, option_table[option_index].nb_params_required);
                    ret = -1;
                    break;
                }
                else {
                    params[nb_params].param = argv[*p_optind];
                    params[nb_params].length = (int)strlen(argv[*p_optind]);
                    nb_params++;
                    *p_optind += 1;
                }
            }
        }
    }

    if (ret == 0) {
        ret = config_set_option(&option_table[option_index], params, nb_params, config);
    }

    return ret;
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

## Pair `picoquic/config.c:picoquic_config_init`
C: `picoquic/config.c:1035-1044 picoquic_config_init`
Rust: `rs/fq/src/config.rs:1176-1236 default`

### C body
```c
{
    memset(config, 0, sizeof(picoquic_quic_config_t));
    config->cnx_id_length = -1;
    config->nb_connections = 256;
    config->initial_random = 3;
    config->cwin_max = UINT64_MAX;
    config->idle_timeout = PICOQUIC_MICROSEC_HANDSHAKE_MAX / 1000;
    config->flow_control_max = 0;
}
```

### Rust body
```rust
    fn default() -> Self {
        Config {
            nb_connections: 256,
            solution_dir: None,
            server_cert_file: None,
            server_key_file: None,
            log_file: None,
            bin_dir: None,
            qlog_dir: None,
            performance_log: None,
            server_port: 0,
            local_port: 0,
            is_port_shared: false,
            nb_threads: 0,
            dest_if: 0,
            mtu_max: 0,
            connection_id_length: -1,
            idle_timeout: (MICROSEC_HANDSHAKE_MAX.ticks() / 1000) as i32,
            socket_buffer_size: 0,
            cc_algo_id: None,
            cc_algo_option_string: None,
            connection_id_cbdata: None,
            spinbit_policy: SpinbitVersion::default(),
            lossbit_policy: LossbitVersion::default(),
            multipath_option: 0,
            multipath_alt_config: None,
            bdp_frame_option: 0,
            cwin_max: u64::MAX,
            address_discovery_mode: 0,
            initial_random: 3,
            use_long_log: false,
            do_preemptive_repeat: false,
            do_not_use_gso: false,
            disable_port_blocking: false,
            enable_sslkeylog: false,
            www_dir: None,
            reset_seed: [0; 16],
            ticket_encryption_key: None,
            do_retry: false,
            has_reset_seed: false,
            ticket_file_name: None,
            token_file_name: None,
            sni: None,
            alpn: None,
            out_dir: None,
            root_trust_file: None,
            cipher_suite_id: 0,
            proposed_version: 0,
            desired_version: 0,
            force_zero_share: false,
            no_disk: false,
            large_client_hello: false,
            ech_key_file: None,
            ech_config_file: None,
            ech_public_name: None,
            ech_target: None,
            flow_control_max: 0,
            preferred_address_v4: None,
            preferred_address_v6: None,
        }
    }
```

## Pair `picoquic/cubic.c:cubic_root`
C: `picoquic/cubic.c:83-119 cubic_root`
Rust: `rs/fq/src/cubic.rs:615-633 cubic_root`

### C body
```c
{
    /* First find an approximation */
    double v = 1;
    double y = 1.0;
    double y2;
    double y3;

    /*
     * v = 1
     *
     * x = (cubic_state->W_max * (1.0 - PICOQUIC_CUBIC_BETA)) / PICOQUIC_CUBIC_C
     * PICOQUIC_CUBIC_C = 0.4
     * (1.0 - PICOQUIC_CUBIC_BETA) = 1 - 7/8 = 1/8
     *
     * v > x * 8
     * 1 > (cubic_state->W_max * (1/8) / 0.4) * 8
     * cubic_state->W_max < 2/5
     */
    while (v > x * 8) {
        v /= 8;
        y /= 2;
    }

    while (v < x) {
        v *= 8;
        y *= 2;
    }

    for (int i = 0; i < 3; i++) {
        y2 = y * y;
        y3 = y2 * y;
        y += (x - y3) / (3.0*y2);
    }

    return y;
}
```

### Rust body
```rust
fn cubic_root(x: f64) -> f64 {
    let mut v: f64 = 1.0;
    let mut y: f64 = 1.0;

    while v > x * 8.0 {
        v /= 8.0;
        y /= 2.0;
    }
    while v < x {
        v *= 8.0;
        y *= 2.0;
    }
    for _ in 0..3 {
        let y2 = y * y;
        let y3 = y2 * y;
        y += (x - y3) / (3.0 * y2);
    }
    y
}
```

## Pair `picoquic/cubic.c:cubic_correct_spurious`
C: `picoquic/cubic.c:206-235 cubic_correct_spurious`
Rust: `rs/fq/src/cubic.rs:221-242 correct_spurious`

### C body
```c
{
    if (cubic_state->ssthresh != UINT64_MAX) {
        cubic_state->W_max = cubic_state->W_last_max;
        cubic_state->start_of_epoch = cubic_state->previous_start_of_epoch;
        cubic_state->alg_state = cubic_state->previous_alg_state;
        if (cubic_state->alg_state != picoquic_cubic_alg_slow_start) {
            cubic_enter_avoidance(cubic_state, cubic_state->previous_start_of_epoch);
            double W_cubic = cubic_W_cubic(cubic_state, current_time);
            cubic_state->W_reno = W_cubic * (double)path_x->send_mtu;
            cubic_state->ssthresh = (uint64_t)(cubic_state->W_max * PICOQUIC_CUBIC_BETA * (double)path_x->send_mtu);
            path_x->cwin = (uint64_t)cubic_state->W_reno;
        }
        else {
            cubic_state->ssthresh = cubic_state->previous_ssthresh;
            path_x->cwin = cubic_state->previous_cwin;
            cubic_state->W_reno = (double)cubic_state->previous_cwin;
        }
    }
}
```

### Rust body
```rust
    pub fn correct_spurious(&mut self, path_x: &mut Path, current_time: u64) {
        if self.ssthresh != u64::MAX {
            self.w_max = self.w_last_max;
            self.start_of_epoch = self.previous_start_of_epoch;
            self.alg_state = match self.previous_alg_state {
                0 => CubicAlgState::SlowStart,
                1 => CubicAlgState::Recovery,
                _ => CubicAlgState::CongestionAvoidance,
            };
            if self.alg_state != CubicAlgState::SlowStart {
                self.enter_avoidance(self.previous_start_of_epoch);
                let w_cubic = self.w_cubic(current_time);
                self.w_reno = w_cubic * path_x.send_mtu as f64;
                self.ssthresh = (self.w_max * CUBIC_BETA * path_x.send_mtu as f64) as u64;
                path_x.cwin = self.w_reno as u64;
            } else {
                self.ssthresh = self.previous_ssthresh;
                path_x.cwin = self.previous_cwin;
                self.w_reno = self.previous_cwin as f64;
            }
        }
    }
```

## Pair `picoquic/cubic.c:cubic_observe`
C: `picoquic/cubic.c:562-567 cubic_observe`
Rust: `rs/fq/src/cubic.rs:135-137 observe`

### C body
```c
{
    picoquic_cubic_state_t* cubic_state = (picoquic_cubic_state_t*)path_x->congestion_alg_state;
    *cc_state = (uint64_t)cubic_state->alg_state;
    *cc_param = (uint64_t)cubic_state->W_max;
}
```

### Rust body
```rust
    pub fn observe(&self) -> (u64, u64) {
        (self.alg_state as u64, self.w_max as u64)
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_recur`
C: `picoquic/dualq_aqm.c:182-191 dualq_recur`
Rust: `rs/fq/src/tests/dualq.rs:289-297 recur`

### C body
```c
int dualq_recur(dualq_queue_t* xq, double likelihood) {
    /* Returns TRUE with a certain likelihood */
    int ret = 0;
    xq->sum_p += likelihood;
    if (xq->sum_p > 1.0) {
        xq->sum_p -= 1.0;
        ret = 1;
    }
    return ret;
}
```

### Rust body
```rust
    fn recur(queue: &mut DualqQueue, likelihood: f64) -> bool {
        queue.sum_p += likelihood;
        if queue.sum_p > 1.0 {
            queue.sum_p -= 1.0;
            true
        } else {
            false
        }
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_pi2_update`
C: `picoquic/dualq_aqm.c:284-320 dualq_pi2_update`
Rust: `rs/fq/src/tests/dualq.rs:349-379 pi2_update`

### C body
```c
{
    int64_t target_delta;
    int64_t delta_q;
    uint64_t cq_time = 0;
    uint64_t lq_time = 0;

    if (dualq->cq.queue_first != NULL && dualq->cq.queue_first->arrival_time < current_time) {
        cq_time = current_time - dualq->cq.queue_first->arrival_time;
    }
    if (dualq->lq.queue_first != NULL && dualq->lq.queue_first->arrival_time < current_time) {
        lq_time = current_time - dualq->lq.queue_first->arrival_time;
    }

    dualq->curq = (cq_time > lq_time) ?
        cq_time : lq_time;

    target_delta = dualq->curq - dualq->target;
    delta_q = dualq->curq - dualq->prevq;

    dualq->pprime = dualq->pprime +
        dualq->pi2_alpha * target_delta +
        dualq->pi2_beta * delta_q;
    /* Bounding p' to [0..1] */
    if (dualq->pprime < 0) {
        dualq->pprime = 0;
    }
    else if (dualq->pprime > 1.0) {
        dualq->pprime = 1.0;
    }
    /* Coupled L4S prob = base prob * coupling factor */
    if ((dualq->p_CL = dualq->pprime * dualq->k) > 1.0) {
        dualq->p_CL = 1.0;
    }
    dualq->p_C = dualq->pprime * dualq->pprime_L;
    dualq->prevq = dualq->curq;
}
```

### Rust body
```rust
    fn pi2_update(&mut self, current_time: Instant) {
        let current_ticks = current_time.ticks();
        let cq_time = self
            .cq
            .packets
            .front()
            .filter(|packet| packet.arrival_time.ticks() < current_ticks)
            .map(|packet| current_ticks - packet.arrival_time.ticks())
            .unwrap_or(0);
        let lq_time = self
            .lq
            .packets
            .front()
            .filter(|packet| packet.arrival_time.ticks() < current_ticks)
            .map(|packet| current_ticks - packet.arrival_time.ticks())
            .unwrap_or(0);

        self.curq = cq_time.max(lq_time) as i64;
        let target_delta = self.curq - self.target as i64;
        let delta_q = self.curq - self.prevq;

        self.p_prime += self.pi2_alpha * target_delta as f64 + self.pi2_beta * delta_q as f64;
        self.p_prime = self.p_prime.clamp(0.0, 1.0);

        self.p_cl = self.p_prime * self.k;
        if self.p_cl > 1.0 {
            self.p_cl = 1.0;
        }
        self.p_c = self.p_prime * self.p_prime_l;
        self.prevq = self.curq;
    }
```

## Pair `picoquic/dualq_aqm.c:dualq_submit`
C: `picoquic/dualq_aqm.c:360-372 dualq_submit`
Rust: `rs/fq/src/tests/dualq_aqm.rs:182-214 dualq_submit`

### C body
```c
{
    dualq_state_t* dualq = (dualq_state_t*)self;

    /* queue the packet. */
    dualq_enqueue(dualq, link, packet, current_time);

    /* submit data if possible, and compute the new value of pi2 parameters if it is time */
    dualq->last_input_time = current_time;
    dualq_update_it(dualq, link, current_time);
}
```

### Rust body
```rust
fn dualq_submit() -> crate::Result<()> {
    let mut ctx = dualq_test_get_ctx()?;
    let mut one_was_dropped = false;

    with_dualq(&mut ctx.link, |dualq, link| {
        for i in 0..50 {
            let i_queue = i % ECN_SEQUENCE.len();
            let packet = dualq_test_get_packet(ECN_SEQUENCE[i_queue], 1000)?;
            let old_bytes = queue_bytes(dualq, QUEUE_ID[i_queue]);
            let old_queue_time = link.queue_time;
            let old_total = dualq.cq.queue_bytes + dualq.lq.queue_bytes + packet.length as u64;

            dualq.submit(link, packet, ctx.simulated_time);

            if old_queue_time.ticks() <= ctx.simulated_time.ticks() {
                check(link.queue_time != old_queue_time)?;
            } else {
                check(link.queue_time == old_queue_time)?;
                if old_total > dualq.limit {
                    check(queue_bytes(dualq, QUEUE_ID[i_queue]) == old_bytes)?;
                    one_was_dropped = true;
                    break;
                } else {
                    check(queue_bytes(dualq, QUEUE_ID[i_queue]) != old_bytes)?;
                    dualq_test_check_queue(link)?;
                }
            }
        }
        Ok(())
    })?;

    check(one_was_dropped)
}
```

## Pair `picoquic/dualq_aqm.c:dualq_configure`
C: `picoquic/dualq_aqm.c:455-500 dualq_configure`
Rust: `rs/fq/src/tests/dualq.rs:436-450 install`

### C body
```c
{
    int ret = 0;
    dualq_state_t* dualq = NULL;

    /* Check whether the link is already configured */
    if (link->aqm_state != NULL) {
        /* use the function pointers as signature to recognize dualq */
        if (link->aqm_state->submit == dualq_submit &&
            link->aqm_state->has_pending == dualq_has_pending &&
            link->aqm_state->admit_pending == dualq_admit_pending &&
            link->aqm_state->reset == dualq_reset &&
            link->aqm_state->release == dualq_release
            ) {
            /* already using dualq! */
            dualq = (dualq_state_t*)link->aqm_state;
        }
        else
        {
            link->aqm_state->release(link->aqm_state, link);
        }
    }
    if (dualq == NULL) {
        /* Create a configuration */
        dualq = (dualq_state_t*)malloc(sizeof(dualq_state_t));

        if (dualq == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
        else {
            memset(dualq, 0, sizeof(dualq_state_t));
            dualq->super.submit = dualq_submit;
            dualq->super.has_pending = dualq_has_pending;
            dualq->super.admit_pending = dualq_admit_pending;
            dualq->super.reset = dualq_reset;
            dualq->super.release = dualq_release;
        }
    }

    if (ret == 0){
        /* reconfigure with the new parameter */
        link->aqm_state = &dualq->super;
        dualq_params_init(dualq, l4s_max, link);
    }
    return ret;
}
```

### Rust body
```rust
    pub fn install(link: &mut TestSimLink, l4s_max: u64) -> Result<(), Error> {
        if let Some(mut aqm) = link.aqm_state.take() {
            if let Some(dualq) = aqm.as_any_mut().downcast_mut::<Dualq>() {
                dualq.params_init(l4s_max, link);
                link.aqm_state = Some(aqm);
                return Ok(());
            }
            aqm.release(link);
        }

        let mut dualq = Dualq::default();
        dualq.params_init(l4s_max, link);
        link.aqm_state = Some(Box::new(dualq));
        Ok(())
    }
```

## Pair `picoquic/ech.c:picoquic_ech_save_config`
C: `picoquic/ech.c:127-169 picoquic_ech_save_config`
Rust: `rs/fq/src/lib.rs:4755-4757 ech_save_config`

### C body
```c
{
    int ret = 0;
    int last_err;
    size_t ech_text_size = 0;
    size_t ech_text_len = 0;
    char* ech_text = NULL;

    (void)picoquic_base64_encode(config, config_len, ech_text, ech_text_size, &ech_text_len);
    ech_text_size = ech_text_len + 1;
    ech_text = (char*)malloc(ech_text_size);

    if (ech_text == NULL) {
        DBG_PRINTF("Cannot allocate %d bytes for text buffer", ech_text_size);
        ret = -1;
    }
    else {
        ret = picoquic_base64_encode(config, config_len, ech_text, ech_text_size, &ech_text_len);
    }
    if (ret == 0) {
        FILE* F = picoquic_file_open_ex(file_name, "w", &last_err);
        if (F == NULL) {
            DBG_PRINTF("Cannot open file <%s>, err: %x", file_name, last_err);
            ret = -1;
        }
        else {
            size_t written = fwrite(ech_text, 1, ech_text_size, F);
            if (written != ech_text_size) {
                DBG_PRINTF("Cannot write %d bytes on file <%s>, err= %zu", ech_text, file_name, written);
                ret = -1;
            }
            else {
                (void)fprintf(F, "\n");
                DBG_PRINTF("Wrote %d bytes on file <%s>", ech_text_size + 1, file_name);
            }
            picoquic_file_close(F);
        }
    }
    if (ech_text != NULL) {
        free(ech_text);
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

## Pair `picoquic/ech.c:picoquic_release_quic_ech_ctx`
C: `picoquic/ech.c:375-389 picoquic_release_quic_ech_ctx`
Rust: `rs/fq/src/ech.rs:929-957 picoquic_release_quic_ech_ctx`

### C body
```c
{
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;
    PICOQUIC_THREAD_CHECK(quic);

    if (ctx != NULL) {
        ech_opener_callback_t* ech_cb = (ech_opener_callback_t*)ctx->ech.server.create_opener;
        ctx->ech.server.retry_configs.base = NULL;
        ctx->ech.server.retry_configs.len = 0;

        if (ech_cb != NULL) {
            ech_dispose_opener_callback(ech_cb);
        }
    }
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

## Pair `picoquic/ech.c:picoquic_ech_parse_public_key`
C: `picoquic/ech.c:601-653 picoquic_ech_parse_public_key`
Rust: `rs/fq/src/ech.rs:555-579 ech_parse_public_key`

### C body
```c
{
    int ret = 0;
    ptls_iovec_t public_key_algo;
    ptls_iovec_t public_key_param;
    ptls_iovec_t public_key_bit_string;
    size_t expected_key_length = 0;
    (void)picoquic_parse_public_key_asn1(public_key_asn1,
        &public_key_algo, &public_key_param,
        &public_key_bit_string, &ret, NULL);
    if (ret == 0) {
        if (public_key_algo.len == sizeof(oid_algo_secp) &&
            memcmp(public_key_algo.base, oid_algo_secp, sizeof(oid_algo_secp)) == 0) {
            if (public_key_param.len == sizeof(oid_pr_secp256r1) &&
                memcmp(public_key_param.base, oid_pr_secp256r1, sizeof(oid_pr_secp256r1)) == 0) {
                *group_id = PTLS_GROUP_SECP256R1;
                expected_key_length = 0x41;
            }
            else if (public_key_param.len == sizeof(oid_pr_secp384r1) &&
                memcmp(public_key_param.base, oid_pr_secp384r1, sizeof(oid_pr_secp384r1)) == 0) {
                *group_id = PTLS_GROUP_SECP384R1;
                expected_key_length = 0x61;
            }
            else {
                DBG_PRINTF("%s", "Unsupported SecP Group ID");
                ret = -1;
            }
        }
        else if (public_key_algo.len == sizeof(oid_x25519) &&
            memcmp(public_key_algo.base, oid_x25519, sizeof(oid_x25519)) == 0) {
            *group_id = PTLS_GROUP_X25519;
            expected_key_length = 0x20;
        }
    }
    else {
        DBG_PRINTF("%s", "Unsupported Algorithm ID");
        ret = -1;
    }
    if (ret == 0) {
        if (public_key_bit_string.len > 1 && public_key_bit_string.base[0] == 0) {
            public_key_bits->base = public_key_bit_string.base + 1;
            public_key_bits->len = public_key_bit_string.len - 1;
            if (public_key_bits->len != expected_key_length) {
                DBG_PRINTF("Invalid length for curve 0x%04x: %zu, expected %zu",
                    *group_id, public_key_bits->len, expected_key_length);
                ret = -1;
            }
        }
    }
    return ret;
}
```

### Rust body
```rust
pub fn ech_parse_public_key(public_key_asn1: &[u8]) -> Result<(u16, &[u8]), Error> {
    let parsed = parse_public_key_asn1(public_key_asn1)?;
    let (group_id, expected_key_length) = if parsed.algo == OID_ALGO_SECP {
        if parsed.param == OID_PR_SECP256R1 {
            (group_id::SECP256R1, 0x41usize)
        } else if parsed.param == OID_PR_SECP384R1 {
            (group_id::SECP384R1, 0x61usize)
        } else {
            return Err(Error::Generic);
        }
    } else if parsed.algo == OID_X25519 {
        (group_id::X25519, 0x20usize)
    } else {
        return Err(Error::Generic);
    };

    if parsed.bit_string.len() <= 1 || parsed.bit_string[0] != 0 {
        return Err(Error::Generic);
    }
    let public_key_bits = &parsed.bit_string[1..];
    if public_key_bits.len() != expected_key_length {
        return Err(Error::Generic);
    }
    Ok((group_id, public_key_bits))
}
```

## Pair `picoquic/ech.c:picoquic_ech_create_config_list_from_config`
C: `picoquic/ech.c:761-800 picoquic_ech_create_config_list_from_config`
Rust: `rs/fq/src/ech.rs:1008-1014 ech_create_config_list_from_config`

### C body
```c
{
    int ret = 0;
    size_t bin_size = config_buf->off + 2;
    uint8_t* bin_val = (uint8_t*)malloc(bin_size);
    if (bin_val == NULL) {
        DBG_PRINTF("Cannot allocate %d bytes for config bin buffer", bin_size);
        ret = -1;
    }
    else {
        bin_val[0] = (uint8_t)(((config_buf->off) >> 8) & 0xff);
        bin_val[1] = (uint8_t)(config_buf->off & 0xff);
        memcpy(bin_val + 2, config_buf->base, config_buf->off);
        *config_list = bin_val;
        *config_list_len = bin_size;
    }
    return ret;
}
```

### Rust body
```rust
pub fn ech_create_config_list_from_config(config: &[u8]) -> Result<Vec<u8>, Error> {
    let config_len = u16::try_from(config.len()).map_err(|_| Error::Generic)?;
    let mut out = Vec::with_capacity(config.len() + 2);
    out.extend_from_slice(&config_len.to_be_bytes());
    out.extend_from_slice(config);
    Ok(out)
}
```

## Pair `picoquic/ech.c:picoquic_ech_create_config_from_public_key`
C: `picoquic/ech.c:857-876 picoquic_ech_create_config_from_public_key`
Rust: `rs/fq/src/lib.rs:4761-4775 ech_create_config_from_public_key`

### C body
```c
{
    int ret = 0;
    ptls_iovec_t public_key_asn1 = ptls_iovec_init(NULL, 0);
    size_t pub_key_objects = 0;

    /* Read the public key from a file. */
    ret = ptls_load_pem_objects(public_key_file, "PUBLIC KEY", &public_key_asn1, 1, &pub_key_objects);
    if (ret != 0) {
        DBG_PRINTF("Cannot load pubkey from <%s>, err: %x", public_key_file, ret);
    }
    else
    {
        ret = picoquic_ech_create_config_from_binary(config, config_len, public_key_asn1, public_name);
    }
    if (public_key_asn1.base != NULL) {
        free(public_key_asn1.base);
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<Vec<u8>, Error> {
    crate::ech::ech_create_config_from_private_key_file(private_key_file, public_name)
}
```

## Pair `picoquic/error_names.c:picoquic_error_name`
C: `picoquic/error_names.c:25-132 picoquic_error_name`
Rust: `rs/fq/src/tests/edge_cases.rs:1531-1647 error_name`

### C body
```c
{
    char const* e_name = "unknown";
    switch (error_code) {
        /* Protocol errors defined in the QUIC spec */
    case PICOQUIC_TRANSPORT_INTERNAL_ERROR: e_name = "internal"; break;
    case PICOQUIC_TRANSPORT_SERVER_BUSY: e_name = "server busy"; break;
    case PICOQUIC_TRANSPORT_FLOW_CONTROL_ERROR: e_name = "flow control"; break;
    case PICOQUIC_TRANSPORT_STREAM_LIMIT_ERROR: e_name = "stream limit"; break;
    case PICOQUIC_TRANSPORT_STREAM_STATE_ERROR: e_name = "stream state"; break;
    case PICOQUIC_TRANSPORT_FINAL_OFFSET_ERROR: e_name = "final offset"; break;
    case PICOQUIC_TRANSPORT_FRAME_FORMAT_ERROR: e_name = "frame format"; break;
    case PICOQUIC_TRANSPORT_PARAMETER_ERROR: e_name = "parameter"; break;
    case PICOQUIC_TRANSPORT_CONNECTION_ID_LIMIT_ERROR: e_name = "connection_id limit"; break;
    case PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION: e_name = "protocol violation"; break;
    case PICOQUIC_TRANSPORT_INVALID_TOKEN: e_name = "invalid token"; break;
    case PICOQUIC_TRANSPORT_APPLICATION_ERROR: e_name = "application"; break;
    case PICOQUIC_TRANSPORT_CRYPTO_BUFFER_EXCEEDED: e_name = "crypto buffer exceeded"; break;
    case PICOQUIC_TRANSPORT_KEY_UPDATE_ERROR: e_name = "key update"; break;
    case PICOQUIC_TRANSPORT_AEAD_LIMIT_REACHED: e_name = "aead limit"; break;
    case PICOQUIC_TLS_ALERT_WRONG_ALPN: e_name = "wrong alpn"; break;
    case PICOQUIC_TLS_HANDSHAKE_FAILED: e_name = "tls handshake failed"; break;
    case PICOQUIC_TRANSPORT_VERSION_NEGOTIATION_ERROR: e_name = "version negotiation"; break;
    case PICOQUIC_TRANSPORT_APPLICATION_ABANDON: e_name = "application abandon"; break;
    case PICOQUIC_TRANSPORT_RESOURCE_LIMIT_REACHED: e_name = "resource limit reached"; break;
    case PICOQUIC_TRANSPORT_UNSTABLE_INTERFACE: e_name = "unstable interface"; break;
    case PICOQUIC_TRANSPORT_NO_CID_AVAILABLE: e_name = "no CID available"; break;
        /* Picoquic local error codes. */
    case PICOQUIC_ERROR_DUPLICATE: e_name = "duplicate"; break;
    case PICOQUIC_ERROR_AEAD_CHECK: e_name = "payload_decrypt_error"; break;
    case PICOQUIC_ERROR_UNEXPECTED_PACKET: e_name = "unexpected packet"; break;
    case PICOQUIC_ERROR_MEMORY: e_name = "memory"; break;
    case PICOQUIC_ERROR_CNXID_CHECK: e_name = "connection ID check"; break;
    case PICOQUIC_ERROR_INITIAL_TOO_SHORT: e_name = ""; break;
    case PICOQUIC_ERROR_VERSION_NEGOTIATION_SPOOFED: e_name = "version negotation spoofed"; break;
    case PICOQUIC_ERROR_MALFORMED_TRANSPORT_EXTENSION: e_name = "malformed transport extension"; break;
    case PICOQUIC_ERROR_EXTENSION_BUFFER_TOO_SMALL: e_name = "extension buffer too small"; break;
    case PICOQUIC_ERROR_ILLEGAL_TRANSPORT_EXTENSION: e_name = "illegal transport extension"; break;
    case PICOQUIC_ERROR_CANNOT_RESET_STREAM_ZERO: e_name = "cannot reset the crypto stream"; break;
    case PICOQUIC_ERROR_INVALID_STREAM_ID: e_name = "invalid stream id"; break;
    case PICOQUIC_ERROR_STREAM_ALREADY_CLOSED: e_name = "stream already closed"; break;
    case PICOQUIC_ERROR_FRAME_BUFFER_TOO_SMALL: e_name = "frame buffer too small"; break;
    case PICOQUIC_ERROR_INVALID_FRAME: e_name = "invalid frame"; break;
    case PICOQUIC_ERROR_CANNOT_CONTROL_STREAM_ZERO: e_name = "cannot control the crypto stream"; break;
    case PICOQUIC_ERROR_RETRY: e_name = "retry"; break;
    case PICOQUIC_ERROR_DISCONNECTED: e_name = "disconnected"; break;
    case PICOQUIC_ERROR_DETECTED: e_name = "error detected"; break;
    case PICOQUIC_ERROR_INVALID_TICKET: e_name = "invalid ticket"; break;
    case PICOQUIC_ERROR_INVALID_FILE: e_name = "invalid file"; break;
    case PICOQUIC_ERROR_SEND_BUFFER_TOO_SMALL: e_name = "send buffer too small"; break;
    case PICOQUIC_ERROR_UNEXPECTED_STATE: e_name = "unexpected state"; break;
    case PICOQUIC_ERROR_UNEXPECTED_ERROR: e_name = "unexpected error"; break;
    case PICOQUIC_ERROR_TLS_SERVER_CON_WITHOUT_CERT: e_name = "server configuration without cert"; break;
    case PICOQUIC_ERROR_NO_SUCH_FILE: e_name = "no such file"; break;
    case PICOQUIC_ERROR_STATELESS_RESET: e_name = "stateless reset"; break;
    case PICOQUIC_ERROR_CONNECTION_DELETED: e_name = "connection deleted"; break;
    case PICOQUIC_ERROR_CNXID_SEGMENT: e_name = "connection ID segment error"; break;
    case PICOQUIC_ERROR_CNXID_NOT_AVAILABLE: e_name = "connection ID not available"; break;
    case PICOQUIC_ERROR_MIGRATION_DISABLED: e_name = "migration disabled"; break;
    case PICOQUIC_ERROR_CANNOT_COMPUTE_KEY: e_name = "cannot compute key"; break;
    case PICOQUIC_ERROR_CANNOT_SET_ACTIVE_STREAM: e_name = "cannot set active stream"; break;
    case PICOQUIC_ERROR_CANNOT_CHANGE_ACTIVE_CONTEXT: e_name = "cannot change active context"; break;
    case PICOQUIC_ERROR_INVALID_TOKEN: e_name = "invalid token"; break;
    case PICOQUIC_ERROR_INITIAL_CID_TOO_SHORT: e_name = "initial CID too short"; break;
    case PICOQUIC_ERROR_KEY_ROTATION_NOT_READY: e_name = "key rotation not ready"; break;
    case PICOQUIC_ERROR_AEAD_NOT_READY: e_name = "aead not ready"; break;
    case PICOQUIC_ERROR_NO_ALPN_PROVIDED: e_name = "no ALPN provided"; break;
    case PICOQUIC_ERROR_NO_CALLBACK_PROVIDED: e_name = "no callback provided"; break;
    case PICOQUIC_STREAM_RECEIVE_COMPLETE: e_name = "stream receive complete"; break;
    case PICOQUIC_ERROR_PACKET_HEADER_PARSING: e_name = "packet header parsing"; break;
    case PICOQUIC_ERROR_QUIC_BIT_MISSING: e_name = "QUIC bit missing"; break;
    case PICOQUIC_NO_ERROR_TERMINATE_PACKET_LOOP: e_name = "terminate packet loop (not an error)"; break;
    case PICOQUIC_NO_ERROR_SIMULATE_NAT: e_name = "simulate NAT (not an error)"; break;
    case PICOQUIC_NO_ERROR_SIMULATE_MIGRATION: e_name = "simulate migration (not an error)"; break;
    case PICOQUIC_ERROR_VERSION_NOT_SUPPORTED: e_name = "version not supported"; break;
    case PICOQUIC_ERROR_IDLE_TIMEOUT: e_name = "idle timeout"; break;
    case PICOQUIC_ERROR_REPEAT_TIMEOUT: e_name = "repeat timeout"; break;
    case PICOQUIC_ERROR_HANDSHAKE_TIMEOUT: e_name = "handshake timeout"; break;
    case PICOQUIC_ERROR_SOCKET_ERROR: e_name = "socket"; break;
    case PICOQUIC_ERROR_VERSION_NEGOTIATION: e_name = "version negotiation"; break;
    case PICOQUIC_ERROR_PACKET_TOO_LONG: e_name = "packet too long"; break;
    case PICOQUIC_ERROR_PACKET_WRONG_VERSION: e_name = "wrong version"; break;
    case PICOQUIC_ERROR_PORT_BLOCKED: e_name = "port blocked"; break;
    case PICOQUIC_ERROR_DATAGRAM_TOO_LONG: e_name = "datagram too long"; break;
    case PICOQUIC_ERROR_PATH_ID_INVALID: e_name = "invalid path ID"; break;
    case PICOQUIC_ERROR_RETRY_NEEDED: e_name = "retry needed"; break;
    case PICOQUIC_ERROR_SERVER_BUSY: e_name = "server busy"; break;
    case PICOQUIC_ERROR_PATH_DUPLICATE: e_name = "duplicate path"; break;
    case PICOQUIC_ERROR_PATH_ID_BLOCKED: e_name = "blocked by lack of path ID"; break;
    case PICOQUIC_ERROR_PATH_CID_BLOCKED: e_name = "blocked by lack of CID"; break;
    case PICOQUIC_ERROR_PATH_ADDRESS_FAMILY: e_name = "path address family"; break;
    case PICOQUIC_ERROR_PATH_NOT_READY: e_name = "path not ready"; break;
    case PICOQUIC_ERROR_PATH_LIMIT_EXCEEDED: e_name = "path limit exceeded"; break;
    case PICOQUIC_ERROR_REDIRECTED: e_name = "redirected to proxy (not an error)"; break; /* Not an error: the packet was captured by a proxy, no further processing needed */
    case PICOQUIC_ERROR_PADDING_PACKET: e_name = "padding_packet"; break; /* Random bytes at end of datagram */
    default:
        if (error_code > 0x100 && error_code < 0x200) {
            /* Protocol errors defined in the QUIC spec */
            e_name = "crypto error alert";
        }
        else if (error_code > 0x400 && error_code < 0x500) {
            /* Picoquic error codes */
            e_name = "unknown picoquic error";
        }
        break;
    }
    return e_name;
}
```

### Rust body
```rust
fn error_name() {
    use crate::errors::InternalError;

    let cases: &[(u64, &str)] = &[
        // Protocol (transport / TLS) errors.
        (0x1, "internal"),
        (0x2, "server busy"),
        (0x3, "flow control"),
        (0x4, "stream limit"),
        (0x5, "stream state"),
        (0x6, "final offset"),
        (0x7, "frame format"),
        (0x8, "parameter"),
        (0x9, "connection_id limit"),
        (0xA, "protocol violation"),
        (0xB, "invalid token"),
        (0xC, "application"),
        (0xD, "crypto buffer exceeded"),
        (0xE, "key update"),
        (0xF, "aead limit"),
        (0x178, "wrong alpn"),
        (0x201, "tls handshake failed"),
        (0x11, "version negotiation"),
        (0x3e, "application abandon"),
        (0x3e75, "resource limit reached"),
        (0x3e76, "unstable interface"),
        (0x3e77, "no CID available"),
        // Picoquic-internal codes (0x400+ range).
        (0x401, "duplicate"),
        (0x403, "payload_decrypt_error"),
        (0x404, "unexpected packet"),
        (0x405, "memory"),
        (0x407, "connection ID check"),
        (0x408, ""),
        (0x409, "version negotation spoofed"),
        (0x40A, "malformed transport extension"),
        (0x40B, "extension buffer too small"),
        (0x40C, "illegal transport extension"),
        (0x40D, "cannot reset the crypto stream"),
        (0x40E, "invalid stream id"),
        (0x40F, "stream already closed"),
        (0x410, "frame buffer too small"),
        (0x411, "invalid frame"),
        (0x412, "cannot control the crypto stream"),
        (0x413, "retry"),
        (0x414, "disconnected"),
        (0x415, "error detected"),
        (0x417, "invalid ticket"),
        (0x418, "invalid file"),
        (0x419, "send buffer too small"),
        (0x41A, "unexpected state"),
        (0x41B, "unexpected error"),
        (0x41C, "server configuration without cert"),
        (0x41D, "no such file"),
        (0x41E, "stateless reset"),
        (0x41F, "connection deleted"),
        (0x420, "connection ID segment error"),
        (0x421, "connection ID not available"),
        (0x422, "migration disabled"),
        (0x423, "cannot compute key"),
        (0x424, "cannot set active stream"),
        (0x425, "cannot change active context"),
        (0x426, "invalid token"),
        (0x427, "initial CID too short"),
        (0x428, "key rotation not ready"),
        (0x429, "aead not ready"),
        (0x42A, "no ALPN provided"),
        (0x42B, "no callback provided"),
        (0x42C, "stream receive complete"),
        (0x42D, "packet header parsing"),
        (0x42E, "QUIC bit missing"),
        (0x42F, "terminate packet loop (not an error)"),
        (0x430, "simulate NAT (not an error)"),
        (0x431, "simulate migration (not an error)"),
        (0x432, "version not supported"),
        (0x433, "idle timeout"),
        (0x434, "repeat timeout"),
        (0x435, "handshake timeout"),
        (0x436, "socket"),
        (0x437, "version negotiation"),
        (0x438, "packet too long"),
        (0x439, "wrong version"),
        (0x43A, "port blocked"),
        (0x43B, "datagram too long"),
        (0x43C, "invalid path ID"),
        (0x43D, "retry needed"),
        (0x43E, "server busy"),
        (0x43F, "duplicate path"),
        (0x440, "blocked by lack of path ID"),
        (0x441, "blocked by lack of CID"),
        (0x442, "path address family"),
        (0x443, "path not ready"),
        (0x444, "path limit exceeded"),
        (0x445, "redirected to proxy (not an error)"),
        (0x446, "padding_packet"),
        // CRYPTO_ERROR alert range (default branch).
        (0x101, "crypto error alert"),
        (0x150, "crypto error alert"),
        (0x1FF, "crypto error alert"),
        // Unknown picoquic error range (default branch).
        (0x450, "unknown picoquic error"),
        (0x4FF, "unknown picoquic error"),
        // Truly unknown.
        (0x0, "unknown"),
        (0x200, "unknown"),
        (0x500, "unknown"),
        (0xFFFF_FFFF_FFFF_FFFF, "unknown"),
    ];

    for &(code, expected) in cases {
        let got = InternalError::name(code).unwrap_or("<none>");
        assert_eq!(
            got, expected,
            "error_code=0x{code:x} got={got:?} expected={expected:?}",
        );
    }
}
```
