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

Owned Rust file(s): `rs/fq/src/config.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/config.c:config_set_option`
* Phase 4C status: `suspect`
* Phase 4C rationale: Mostly similar option handling, but Rust EchServer treats the second parameter as optional while C calls config_set_string_param for parameter 1 after setting parameter 0.
* Phase 4D analysis: The command-line and config-file paths enforce nb_params=2, but apply_option is also reached by Config::set_option with only one value. C then returns -1 when setting param 1, while Rust succeeds and leaves ech_config_file unchanged.
* Phase 4D fix note: Make EchServer require parameter 1 and update ech_config_file with config_set_string_param-equivalent semantics so missing or empty param 1 returns Err and clears/replaces consistently with C.
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
            *p_optind += 1;
        }
    }

    apply_option(config, entry, &params)
}

/// C: `config_set_option` (picoquic/config.c:274)
///
/// Applies a single option — identified by `entry.id` — to `config`.  Each
/// arm mirrors the corresponding `case` in the C `switch (option_desc->option_num)`.
/// Error conditions that C prints to `stderr` via `config_optval_param_string`
/// are surfaced as `Err(Error::InvalidArgument)` instead; the C `int` return
/// (`0` / `-1`) maps to `Ok(())` / `Err`.
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
```
