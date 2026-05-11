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

## `picoquic/tls_api.c:picoquic_tls_set_client_authentication`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C setter assigns require_client_authentication from an argument; Rust body is a getter returning client_authentication.
* C source: `picoquic/tls_api.c:2815-2817`
* C signature: `void picoquic_tls_set_client_authentication(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/tls_api.rs:1698-1706`
* Rust item: `tls_set_client_authentication`

### C body
```c
void picoquic_tls_set_client_authentication(picoquic_quic_t* quic, int client_authentication) {
    ((ptls_context_t*)quic->tls_master_ctx)->require_client_authentication = client_authentication;
}
```

### Rust body
```rust
    pub fn tls_client_authentication_activated(&self) -> bool {
        self.client_authentication
    }
```

## `picoquic/transport.c:picoquic_receive_transport_extensions`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body handles duplicate detection, defaults, validation, connection-ID checks, version upgrade, negotiated feature flags, idle timeout, and consumed bytes; Rust body only decodes a subset of parameters with little validation and omits most post-processing.
* C source: `picoquic/transport.c:534-1024`
* C signature: `int picoquic_receive_transport_extensions(picoquic_cnx_t *, int, uint8_t *, size_t, size_t *)`
* Rust source: `rs/fq/src/internal.rs:15502-15653`
* Rust item: `receive_transport_extensions`

### C body
```c
{
    int ret = 0;
    size_t byte_index = 0;
    uint64_t present_flag = 0;
    picoquic_connection_id_t original_connection_id = picoquic_null_connection_id;
    picoquic_connection_id_t handshake_connection_id = picoquic_null_connection_id;
    picoquic_connection_id_t retry_connection_id = picoquic_null_connection_id;

    cnx->remote_parameters_received = 1;
    picoquic_clear_transport_extensions(cnx);

    picoquic_log_transport_extension(cnx, 0, bytes_max, bytes);

    /* Set the parameters to default value zero */
    memset(&cnx->remote_parameters, 0, sizeof(picoquic_tp_t));
    /* Except for ack_delay_exponent, whose default is 3 */
    cnx->remote_parameters.ack_delay_exponent = 3;

    while (ret == 0 && byte_index < bytes_max) {
        size_t ll_type = 0;
        size_t ll_length = 0;
        uint64_t extension_type = UINT64_MAX;
        uint64_t extension_length = 0;

        if (byte_index + 2 > bytes_max) {
            ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "TP length");
        }
        else {
            ll_type = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, &extension_type);
            byte_index += ll_type;
            ll_length = picoquic_varint_decode(bytes + byte_index, bytes_max - byte_index, &extension_length);
            byte_index += ll_length;

            if (ll_type == 0 || ll_length == 0 || byte_index + extension_length > bytes_max) {
                ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0);
            }
            else {
                if (extension_type < 64) {
                    if ((present_flag & (1ull << extension_type)) != 0) {
                        /* Malformed, already present */
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Malformed TP");
                    }
                    else {
                        present_flag |= (1ull << extension_type);
                    }
                }

                switch (extension_type) {
                case picoquic_tp_initial_max_stream_data_bidi_local:
                    cnx->remote_parameters.initial_max_stream_data_bidi_local =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);

                    /* If we sent zero rtt data, the streams were created with the
                     * old value of the remote parameter. We need to update that.
                     */
                    picoquic_update_stream_initial_remote(cnx);
                    break;
                case picoquic_tp_initial_max_stream_data_bidi_remote:
                    cnx->remote_parameters.initial_max_stream_data_bidi_remote =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    /* If we sent zero rtt data, the streams were created with the
                    * old value of the remote parameter. We need to update that.
                    */
                    picoquic_update_stream_initial_remote(cnx);
                    break;
                case picoquic_tp_initial_max_stream_data_uni: {
                    cnx->remote_parameters.initial_max_stream_data_uni =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    /* If we sent zero rtt data, the streams were created with the
                    * old value of the remote parameter. We need to update that.
                    */
                    picoquic_update_stream_initial_remote(cnx);
                    break;
                }
                case picoquic_tp_initial_max_data:
                    cnx->remote_parameters.initial_max_data =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    cnx->maxdata_remote = cnx->remote_parameters.initial_max_data;
                    break;
                case picoquic_tp_initial_max_streams_bidi: {
                    uint64_t old_limit = cnx->max_stream_id_bidir_remote;
                    cnx->remote_parameters.initial_max_stream_id_bidir =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    if (cnx->remote_parameters.initial_max_stream_id_bidir >= (1ull << 60)) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Max streams bidir");
                    }
                    else {
                        cnx->max_stream_id_bidir_remote = STREAM_ID_FROM_RANK(cnx->remote_parameters.initial_max_stream_id_bidir,
                            cnx->client_mode, 0);
                        cnx->max_stream_data_remote = cnx->remote_parameters.initial_max_stream_data_bidi_remote;
                        picoquic_add_output_streams(cnx, old_limit, cnx->max_stream_id_bidir_remote, 1);
                    }
                    break;
                }
                case picoquic_tp_idle_timeout:
                    cnx->remote_parameters.max_idle_timeout = 
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    break;

                case picoquic_tp_max_packet_size: {
                    /* The default for this parameter is the maximum permitted UDP payload of 65527. Values below 1200 are invalid. */
                    uint64_t max_packet_size = picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    if (ret == 0){
                        if (max_packet_size < 1200 || max_packet_size > 65527) {
                            ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Max packet size TP");
                        }
                        else {
                            cnx->remote_parameters.max_packet_size = (uint32_t)max_packet_size;
                        }
                    }
                    break;
                }
                case picoquic_tp_stateless_reset_token:
                    if (extension_mode != 1) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Reset token from client");
                    }
                    else if (extension_length != PICOQUIC_RESET_SECRET_SIZE) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Reset token TP");
                    }
                    else {
                        memcpy(cnx->path[0]->first_tuple->p_remote_cnxid->reset_secret, bytes + byte_index, PICOQUIC_RESET_SECRET_SIZE);
                    }
                    break;
                case picoquic_tp_ack_delay_exponent:
                {
                    uint64_t ad_exponent = picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    if (ad_exponent > 20){ 
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0,
                            "ack delay exponent over 20");
                    }
                    else {
                        cnx->remote_parameters.ack_delay_exponent = (uint8_t)ad_exponent;
                    }
                    break;
                }
                case picoquic_tp_initial_max_streams_uni: {
                    uint64_t old_limit = cnx->max_stream_id_unidir_remote;
                    cnx->remote_parameters.initial_max_stream_id_unidir =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    if (cnx->remote_parameters.initial_max_stream_id_unidir >= (1ull << 60)) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Max streams unidir");
                    }
                    else {
                        cnx->max_stream_id_unidir_remote = STREAM_ID_FROM_RANK(cnx->remote_parameters.initial_max_stream_id_unidir,
                            cnx->client_mode, 1);
                        picoquic_add_output_streams(cnx, old_limit, cnx->max_stream_id_unidir_remote, 0);
                    }
                    break;
                }
                case picoquic_tp_server_preferred_address:
                {
                    uint64_t coded_length = picoquic_decode_transport_preferred_address_address(
                        bytes + byte_index, (size_t)extension_length, &cnx->remote_parameters.preferred_address);

                    if (coded_length != extension_length) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Preferred address TP");
                    }
                    break;
                }
                case picoquic_tp_disable_migration:
                    if (extension_length != 0) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Disable migration TP");
                    }
                    else {
                        cnx->remote_parameters.migration_disabled = 1;
                    }
                    break;
                case picoquic_tp_max_ack_delay: {
                    uint64_t max_ack_delay_ms = picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    if (max_ack_delay_ms > PICOQUIC_MAX_ACK_DELAY_MAX_MS) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Max ack delay too large");
                    }
                    else {
                        cnx->remote_parameters.max_ack_delay = (uint32_t)max_ack_delay_ms * 1000;
                    }
                    break;
                }
                case picoquic_tp_original_connection_id:
                    ret = picoquic_transport_param_cid_decode(cnx, bytes + byte_index, extension_length, &original_connection_id);
                    break;
                case picoquic_tp_retry_connection_id:
                    ret = picoquic_transport_param_cid_decode(cnx, bytes + byte_index, extension_length, &retry_connection_id);
                    break;
                case picoquic_tp_handshake_connection_id:
                    ret = picoquic_transport_param_cid_decode(cnx, bytes + byte_index, extension_length, &handshake_connection_id);
                    if (ret == 0) {
                        if (picoquic_compare_connection_id(&cnx->path[0]->first_tuple->p_remote_cnxid->cnx_id, &handshake_connection_id) != 0) {
                            ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "HCID check");
                        }
                        else {
                            cnx->is_hcid_verified = 1;
                        }
                    }
                    break;
                case picoquic_tp_active_connection_id_limit:
                    cnx->remote_parameters.active_connection_id_limit = (uint32_t)
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    if (cnx->remote_parameters.active_connection_id_limit < 2) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "CID limit too small.");
                    }
                    break;
                case picoquic_tp_max_datagram_frame_size:
                    cnx->remote_parameters.max_datagram_frame_size = (uint32_t)
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    break;
                case picoquic_tp_enable_loss_bit: {
                    uint64_t enabled = picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    if (ret == 0) {
                        if (enabled == 0) {
                            /* Send only variant of loss bit */
                            cnx->remote_parameters.enable_loss_bit = 1;
                        }
                        else if (enabled == 1) {
                            /* Both send and receive are enabled */
                            cnx->remote_parameters.enable_loss_bit = 2;
                        }
                        else {
                            /* Only values 0 and 1 are expected */
                            ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Loss bit TP");
                        }
                    }
                    break;
                }
                case picoquic_tp_min_ack_delay:
                    cnx->remote_parameters.min_ack_delay =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    /* Values of 0 and values larger that 2^24 are not expected */
                    if (ret == 0 &&
                        (cnx->remote_parameters.min_ack_delay == 0 ||
                            cnx->remote_parameters.min_ack_delay > PICOQUIC_ACK_DELAY_MIN_MAX_VALUE)) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0, "Min ack delay TP");
                    }
                    else {
                        if (cnx->local_parameters.min_ack_delay > 0) {
                            cnx->is_ack_frequency_negotiated = 1;
                        }
                    }
                    break;
                case picoquic_tp_enable_time_stamp: {
                    uint64_t tp_time_stamp =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);

                    if (ret == 0) {
                        if (tp_time_stamp < 1 || tp_time_stamp > 3) {
                            ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0);
                        }
                        else {
                            cnx->remote_parameters.enable_time_stamp = (int)tp_time_stamp;
                        }
                    }
                    break;
                }
                case picoquic_tp_grease_quic_bit:
                    if (extension_length != 0) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Grease TP");
                    }
                    else {
                        cnx->remote_parameters.do_grease_quic_bit = 1;
                    }
                    break;
                case picoquic_tp_initial_max_path_id: {
                    cnx->remote_parameters.initial_max_path_id = 
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    break;
                }

                case picoquic_tp_version_negotiation: {
                    uint64_t error_found;
                    uint32_t negotiated_vn;
                    int negotiated_index;
                    const uint8_t* final = picoquic_process_tp_version_negotiation(bytes + byte_index,
                        bytes + byte_index + extension_length, extension_mode,
                        picoquic_supported_versions[cnx->version_index].version,
                        &negotiated_vn, &negotiated_index, &error_found);
                    if (final == NULL) {
                        ret = picoquic_connection_error_ex(cnx, error_found, 0, "V. Negotiation TP");
                    }
                    else {
                        cnx->do_version_negotiation = 1;
                        if (negotiated_vn != 0 && cnx->version_index != negotiated_index){
                            ret = picoquic_process_version_upgrade(cnx, cnx->version_index, negotiated_index);
                        }
                    }
                    break;
                }
                case picoquic_tp_enable_bdp_frame: {
                    uint64_t enable_bdp =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    if (ret == 0) {
                        if (enable_bdp > 1) {
                            ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "BDP parameter");
                        }
                        else {
                            cnx->remote_parameters.enable_bdp_frame = (int)enable_bdp;
                        }
                    }
                    break;
                }
                case picoquic_tp_address_discovery: {
                    uint64_t address_discovery_mode =
                        picoquic_transport_param_varint_decode(cnx, bytes + byte_index, extension_length, &ret);
                    if (ret == 0) {
                        if (address_discovery_mode > 2) {
                            ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Address discovery parameter");
                        }
                        else {
                            /* After doing +1, we get the following:
                            * address_discovery_mode == 0: nothing goes (TP is absent)
                            * address_discovery_mode == 1: send only (TP value 0)
                            * address_discovery_mode == 2: receive only (TP value 1)
                            * address_discovery_mode == 3: both (TP value 2)
                            */
                            cnx->remote_parameters.address_discovery_mode = (int)(address_discovery_mode + 1);
                            cnx->is_address_discovery_provider = ((cnx->remote_parameters.address_discovery_mode & 2) != 0 &&
                                (cnx->local_parameters.address_discovery_mode & 1) != 0);
                            cnx->is_address_discovery_receiver = ((cnx->remote_parameters.address_discovery_mode & 1) != 0 &&
                                (cnx->local_parameters.address_discovery_mode & 2) != 0);
                        }
                    }
                    break;
                }
                case picoquic_tp_reset_stream_at:
                    if (extension_length != 0) {
                        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "Reset Stream At TP");
                    }
                    else {
                        cnx->remote_parameters.is_reset_stream_at_enabled = 1;
                    }
                    break;
                default:
                    /* ignore unknown extensions */
                    break;
                }

                if (ret == 0) {
                    byte_index += (size_t)extension_length;
                }
            }
        }
    }

    /* Compute the negotiated version of the time out.
     * The parameter values are expressed in milliseconds,
     * but the connection context variable is in microseconds.
     * If the keep alive interval was set to a too short value,
     * reset it.
     */
    cnx->idle_timeout = cnx->local_parameters.max_idle_timeout*1000ull;
    if (cnx->local_parameters.max_idle_timeout == 0 ||
        (cnx->remote_parameters.max_idle_timeout > 0 && cnx->remote_parameters.max_idle_timeout < 
            cnx->local_parameters.max_idle_timeout)) {
        cnx->idle_timeout = cnx->remote_parameters.max_idle_timeout*1000ull;
    }
    if (cnx->idle_timeout == 0) {
        cnx->idle_timeout = UINT64_MAX;
    }
    else if (cnx->keep_alive_interval != 0 &&
        cnx->keep_alive_interval > cnx->idle_timeout / 2) {
        cnx->keep_alive_interval = cnx->idle_timeout / 2;
    }

    if (ret == 0 && (present_flag & (1ull << picoquic_tp_max_ack_delay)) == 0) {
        cnx->remote_parameters.max_ack_delay = PICOQUIC_ACK_DELAY_MAX_DEFAULT;
    }

    if (ret == 0 && (present_flag & (1ull << picoquic_tp_active_connection_id_limit)) == 0) {
        if (cnx->path[0]->first_tuple->p_local_cnxid->cnx_id.id_len == 0) {
            cnx->remote_parameters.active_connection_id_limit = 0;
        }
        else {
            cnx->remote_parameters.active_connection_id_limit = PICOQUIC_NB_PATH_DEFAULT;
        }
    }

    /* Clients must not include reset token, server address, retry cid or original cid  */

    if (ret == 0 && extension_mode == 0 &&
        ((present_flag & (1ull << picoquic_tp_stateless_reset_token)) != 0 ||
        (present_flag & (1ull << picoquic_tp_server_preferred_address)) != 0 ||
            (present_flag & (1ull << picoquic_tp_original_connection_id)) != 0 ||
            (present_flag & (1ull << picoquic_tp_retry_connection_id)) != 0)) {
        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "T. Param. unexpected on client");
    }

    /* In the old versions, there was only one parameter: original CID. In the new versions,
     * there are also retry CID and handshake CID, and the verification logic changed. 
     * If the new extensions are not used and the version is old, we support the
     * old behavior. If the HCID extension is present, we support the new behavior.
     * Most of the verifications happen on the client side, upon receiving server
     * parameters. 
     * TODO: clean up when removing support for version 27.
     */

    if (ret == 0 && picoquic_supported_versions[cnx->version_index].version != PICOQUIC_SEVENTEENTH_INTEROP_VERSION &&
        (present_flag & (1ull << picoquic_tp_handshake_connection_id)) == 0) {
        /* HCID extension becomes mandatory after draft 27 */
        ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "HCID missing");
    }

    if (ret == 0 && extension_mode == 1) {
        /* Reeciving server parameters */
        if ((present_flag & (1ull << picoquic_tp_handshake_connection_id)) != 0) {
            /* The HCID extension is present. Verify that the original and retry cnxid are as expected */
            if (cnx->original_cnxid.id_len != 0) {
                /* OCID should be present and match original_cid.
                 * RCID should be present and match initial_cid, since token parsing
                 * verified that initial_cid matches source CID of retry packet. */
                if ((present_flag & (1ull << picoquic_tp_retry_connection_id)) == 0 ||
                    (present_flag & (1ull << picoquic_tp_original_connection_id)) == 0 ||
                    picoquic_compare_connection_id(&cnx->original_cnxid, &original_connection_id) != 0 ||
                    picoquic_compare_connection_id(&cnx->initial_cnxid, &retry_connection_id) != 0) {
                    ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "OCID verification");
                }
            }
            else {
                /* RCID should not be present, OCID should be present and match initial_cid */
                if ((present_flag & (1ull << picoquic_tp_retry_connection_id)) != 0 ||
                    (present_flag & (1ull << picoquic_tp_original_connection_id)) == 0 ||
                    picoquic_compare_connection_id(&cnx->initial_cnxid, &original_connection_id) != 0) {
                    ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "HCID or no OCID");
                }
            }
        }
        else  if (picoquic_supported_versions[cnx->version_index].version == PICOQUIC_SEVENTEENTH_INTEROP_VERSION) {
            /* Old behavior. Original CID only present if retry */
            if (cnx->original_cnxid.id_len != 0 &&
                ((present_flag & (1ull << picoquic_tp_original_connection_id)) == 0 ||
                    picoquic_compare_connection_id(&cnx->original_cnxid, &original_connection_id) != 0)) {
                ret = picoquic_connection_error_ex(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0, "old draft version");
            }
        }
    }

    if (ret == 0) {
        /* Negotiate the multipath option */
        ret = picoquic_negotiate_multipath_option(cnx);
    }

    /* Loss bit is only enabled if negotiated by both parties */
    cnx->is_loss_bit_enabled_outgoing = (cnx->local_parameters.enable_loss_bit > 1) && (cnx->remote_parameters.enable_loss_bit > 0);
    cnx->is_loss_bit_enabled_incoming = (cnx->local_parameters.enable_loss_bit > 0) && (cnx->remote_parameters.enable_loss_bit > 1);

    /* Send-receive BDP frame is only enabled if negotiated by both parties */
    cnx->send_receive_bdp_frame = (cnx->local_parameters.enable_bdp_frame > 0) && (cnx->remote_parameters.enable_bdp_frame > 0);

    /* One way delay, Quic_bit_grease and Multipath only enabled if asked by client and accepted by server */
    if (cnx->client_mode) {
        cnx->is_time_stamp_enabled = 
            (cnx->local_parameters.enable_time_stamp&1) && (cnx->remote_parameters.enable_time_stamp&2);
        cnx->is_time_stamp_sent =
            (cnx->local_parameters.enable_time_stamp & 2) && (cnx->remote_parameters.enable_time_stamp & 1);
        cnx->do_grease_quic_bit = cnx->local_parameters.do_grease_quic_bit && cnx->remote_parameters.do_grease_quic_bit;
    }
    else
    {
        if (cnx->remote_parameters.enable_time_stamp) {
            int v_local = 0;
            if (cnx->remote_parameters.enable_time_stamp & 1) {
                /* Peer wants TS. Say that we can send. */
                v_local |= 2;
                cnx->is_time_stamp_sent = 1;
            }
            if (cnx->remote_parameters.enable_time_stamp & 2) {
                /* Peer can do TS. Say that we want to receive. */
                v_local |= 1;
                cnx->is_time_stamp_enabled = 1;
            }
            cnx->local_parameters.enable_time_stamp = v_local;
        }
        /* When the one way option is set, the server will grease the quic bit if the client supports that,
         * but will not announce support of the grease quic bit, thus asking the client to not set it */
        cnx->local_parameters.do_grease_quic_bit = cnx->remote_parameters.do_grease_quic_bit && !cnx->quic->one_way_grease_quic_bit;
        cnx->do_grease_quic_bit = cnx->remote_parameters.do_grease_quic_bit;
    }

    /* ACK Frequency is only enabled on server if negotiated by client */
    if (!cnx->client_mode && !cnx->is_ack_frequency_negotiated) {
        cnx->local_parameters.min_ack_delay = 0;
    }

    /* Reset Stream At enabled if both local and remote are set */
    cnx->is_reset_stream_at_enabled =
        cnx->remote_parameters.is_reset_stream_at_enabled &&
        cnx->local_parameters.is_reset_stream_at_enabled;

    *consumed = byte_index;

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        *consumed = 0;
        self.remote_parameters_received = true;
        self.picoquic_clear_transport_extensions();
        let mut tail = &bytes[..bytes_max.min(bytes.len())];
        let mut tp = self.remote_parameters.clone();
        while !tail.is_empty() {
            let before = tail.len();
            let mut id = 0;
            let Some(after_id) = frames_varint_decode(tail, &mut id) else {
                return -1;
            };
            let mut len = 0;
            let Some(after_len) = frames_varint_decode(after_id, &mut len) else {
                return -1;
            };
            if after_len.len() < len as usize {
                return -1;
            }
            let value = &after_len[..len as usize];
            match id {
                x if x == crate::tp::TransportParameter::InitialMaxData as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_data = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamDataBidiLocal as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_data_bidi_local = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamDataBidiRemote as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_data_bidi_remote = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamDataUni as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_data_uni = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamsBidi as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_id_bidir = v;
                    }
                }
                x if x == crate::tp::TransportParameter::InitialMaxStreamsUni as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_stream_id_unidir = v;
                    }
                }
                x if x == crate::tp::TransportParameter::IdleTimeout as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.max_idle_timeout = Duration::from_ticks(v);
                    }
                }
                x if x == crate::tp::TransportParameter::MaxPacketSize as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.max_packet_size = v as u32;
                    }
                }
                x if x == crate::tp::TransportParameter::MaxAckDelay as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.max_ack_delay = v as u32;
                    }
                }
                x if x == crate::tp::TransportParameter::AckDelayExponent as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.ack_delay_exponent = v as u8;
                    }
                }
                x if x == crate::tp::TransportParameter::ActiveConnectionIdLimit as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.active_connection_id_limit = v as u32;
                    }
                }
                x if x == crate::tp::TransportParameter::DisableMigration as u64 => {
                    tp.migration_disabled = true;
                }
                x if x == crate::tp::TransportParameter::MaxDatagramFrameSize as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.max_datagram_frame_size = v as u32;
                    }
                }
                x if x == crate::tp::TransportParameter::EnableLossBit as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.enable_loss_bit = v as i32;
                    }
                }
                x if x == crate::tp::TransportParameter::EnableTimeStamp as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.enable_time_stamp = v as i32;
                    }
                }
                x if x == crate::tp::TransportParameter::MinAckDelay as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.min_ack_delay = Duration::from_ticks(v);
                    }
                }
                x if x == crate::tp::TransportParameter::GreaseQuicBit as u64 => {
                    tp.do_grease_quic_bit = true;
                }
                x if x == crate::tp::TransportParameter::EnableBdpFrame as u64 => {
                    tp.enable_bdp_frame = true;
                }
                x if x == crate::tp::TransportParameter::InitialMaxPathId as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.initial_max_path_id = v;
                    }
                }
                x if x == crate::tp::TransportParameter::AddressDiscovery as u64 => {
                    if let Some(v) = decode_single_varint(value) {
                        tp.address_discovery_mode = v as i32;
                    }
                }
                x if x == crate::tp::TransportParameter::ResetStreamAt as u64 => {
                    tp.is_reset_stream_at_enabled = true;
                }
                x if x == crate::tp::TransportParameter::VersionNegotiation as u64 => {
                    let mut negotiated = 0;
                    let mut negotiated_index = -1;
                    let mut vn_error = 0;
                    if process_tp_version_negotiation(
                        value,
                        extension_mode,
                        self.proposed_version,
                        &mut negotiated,
                        &mut negotiated_index,
                        &mut vn_error,
                    )
                    .is_none()
                    {
                        return -1;
                    }
                    tp.version_negotiation.current = negotiated;
                    let _ = negotiated_index;
                    let _ = vn_error;
                }
                _ => {}
            }
            tail = &after_len[len as usize..];
            *consumed += before - tail.len();
        }
        self.remote_parameters = tp;
        0
    }
```

## `picoquic/util.c:get_debug_out`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns debug_out; Rust body is get_debug_suspended and returns DEBUG_SUSPENDED, not the debug output stream.
* C source: `picoquic/util.c:111-114`
* C signature: `FILE * get_debug_out(void)`
* Rust source: `rs/fq/src/utils.rs:134-143`
* Rust item: `get_debug_out`

### C body
```c
{
    return debug_out;
}
```

### Rust body
```rust
pub fn get_debug_suspended() -> bool {
    DEBUG_SUSPENDED.with(|s| s.get())
}
```

## `picoquic/util.c:picoquic_frames_uint8_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C writes the byte and advances on success; Rust body only checks for empty input and has no write or success path shown.
* C source: `picoquic/util.c:990-1000`
* C signature: `uint8_t * picoquic_frames_uint8_encode(uint8_t *, const uint8_t *, uint8_t)`
* Rust source: `rs/fq/src/utils.rs:859-862`
* Rust item: `frames_uint8_encode`

### C body
```c
{
    if (bytes + sizeof(n) > bytes_max) {
        bytes = NULL;
    }
    else {
        *bytes++ = n;
    }

    return (bytes);
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## `picoquic/util.c:picoquic_parse_hexa`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C parses a full even-length hex input into output bytes and returns the byte count; Rust body only defines a helper for one hex digit and shows no loop, output write, validation of length, or return count.
* C source: `picoquic/util.c:286-307`
* C signature: `size_t picoquic_parse_hexa(const char *, size_t, uint8_t *, size_t)`
* Rust source: `rs/fq/src/utils.rs:414-422`
* Rust item: `parse_hexa`

### C body
```c
{
    size_t ret = 0;
    if (input_length > 0 && (input_length & 1) == 0 && 2*output_max >= input_length) {
        size_t offset = 0;

        while (offset < input_length) {
            int a = picoquic_parse_hexa_digit(hex_input[offset++]);
            int b = picoquic_parse_hexa_digit(hex_input[offset++]);

            if (a < 0 || b < 0) {
                ret = 0;
                break;
            }
            else {
                bin_output[ret++] = (uint8_t)((a << 4) | b);
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
    fn hexa_digit(x: u8) -> Option<u8> {
        match x {
            b'0'..=b'9' => Some(x - b'0'),
            b'A'..=b'F' => Some(x - b'A' + 10),
            b'a'..=b'f' => Some(x - b'a' + 10),
            _ => None,
        }
    }
```
