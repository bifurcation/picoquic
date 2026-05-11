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

## `picoquic/transport.c:picoquic_prepare_transport_extensions`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C conditionally encodes many parameters, CIDs, reset token, grease, large CHLO padding, version negotiation, and logs consumed bytes; Rust always encodes many varint parameters, omits visible CID/reset-token/large-padding handling, and encodes some C flag parameters as varints with value 1.
* C source: `picoquic/transport.c:306-501`
* C signature: `int picoquic_prepare_transport_extensions(picoquic_cnx_t *, int, uint8_t *, size_t, size_t *)`
* Rust source: `rs/fq/src/internal.rs:15327-15464`
* Rust item: `prepare_transport_extensions`

### C body
```c
{
    int ret = 0;
    uint8_t* bytes_zero = bytes;
    uint8_t* bytes_max = bytes + bytes_length;

    bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_initial_max_stream_data_bidi_local,
        cnx->local_parameters.initial_max_stream_data_bidi_local);

    bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_initial_max_data,
        cnx->local_parameters.initial_max_data);

    if (cnx->local_parameters.initial_max_stream_id_bidir > 0) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_initial_max_streams_bidi,
            cnx->local_parameters.initial_max_stream_id_bidir);
    }

    if (cnx->local_parameters.max_idle_timeout > 0) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_idle_timeout,
            cnx->local_parameters.max_idle_timeout);
    }

    bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_max_packet_size,
        cnx->local_parameters.max_packet_size);

    if (cnx->local_parameters.ack_delay_exponent != 3) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_ack_delay_exponent,
            cnx->local_parameters.ack_delay_exponent);
    }

    if (cnx->local_parameters.initial_max_stream_id_unidir > 0) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_initial_max_streams_uni,
            cnx->local_parameters.initial_max_stream_id_unidir);
    }

    if (cnx->local_parameters.preferred_address.is_defined) {
        bytes = picoquic_encode_transport_preferred_address_address(
            bytes, bytes_max, &cnx->local_parameters.preferred_address);
    }

    if (cnx->local_parameters.migration_disabled != 0 && bytes != NULL) {
        bytes = picoquic_transport_param_type_flag_encode(bytes, bytes_max, picoquic_tp_disable_migration);
    }

    if (cnx->local_parameters.initial_max_stream_data_bidi_remote > 0) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_initial_max_stream_data_bidi_remote,
            cnx->local_parameters.initial_max_stream_data_bidi_remote);
    }

    if (cnx->local_parameters.initial_max_stream_data_uni > 0) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_initial_max_stream_data_uni,
            cnx->local_parameters.initial_max_stream_data_uni);
    }

    if (cnx->local_parameters.active_connection_id_limit > 0) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_active_connection_id_limit,
            cnx->local_parameters.active_connection_id_limit);
    }

    if (cnx->local_parameters.max_ack_delay != PICOQUIC_ACK_DELAY_MAX_DEFAULT) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_max_ack_delay,
            (cnx->local_parameters.max_ack_delay + 999) / 1000); /* Max ACK delay in milliseconds */
    }
    bytes = picoquic_transport_param_cid_encode(bytes, bytes_max, picoquic_tp_handshake_connection_id, &cnx->path[0]->first_tuple->p_local_cnxid->cnx_id);

    if (extension_mode == 1){
        if (cnx->original_cnxid.id_len > 0) {
            bytes = picoquic_transport_param_cid_encode(bytes, bytes_max, picoquic_tp_original_connection_id, &cnx->original_cnxid);
            bytes = picoquic_transport_param_cid_encode(bytes, bytes_max, picoquic_tp_retry_connection_id, &cnx->initial_cnxid);
        }
        else if (cnx->is_hcid_verified) {
            bytes = picoquic_transport_param_cid_encode(bytes, bytes_max, picoquic_tp_original_connection_id, &cnx->initial_cnxid);
        }
    }

    if (extension_mode == 1) {
        if (bytes != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_tp_stateless_reset_token)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, PICOQUIC_RESET_SECRET_SIZE)) != NULL) {
            if (bytes + PICOQUIC_RESET_SECRET_SIZE < bytes_max) {
                (void)picoquic_create_cnxid_reset_secret(cnx->quic, &cnx->path[0]->first_tuple->p_local_cnxid->cnx_id, bytes);
                bytes += PICOQUIC_RESET_SECRET_SIZE;
            }
            else {
                bytes = NULL;
            }
        }
    }

    if (!cnx->client_mode && cnx->local_parameters.max_datagram_frame_size == 0 &&
        cnx->remote_parameters.max_datagram_frame_size > 0) {
        cnx->local_parameters.max_datagram_frame_size = PICOQUIC_MAX_PACKET_SIZE;
    }

    if (cnx->local_parameters.max_datagram_frame_size > 0 && bytes != NULL) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_max_datagram_frame_size,
            cnx->local_parameters.max_datagram_frame_size);
    }

    if (cnx->grease_transport_parameters) {
        /* Do not use a purely random value, so we can repetitive tests */
        int n = 31 * (cnx->initial_cnxid.id[0] + cnx->client_mode) + 27;
        uint64_t v = cnx->initial_cnxid.id[1];
        while (n == picoquic_tp_test_large_chello) {
            n += 31;
        }
        v = (v << 8) + cnx->initial_cnxid.id[2];
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, n, v);
    }

    if (cnx->test_large_chello && bytes != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_tp_test_large_chello)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, 1200)) != NULL){
        if (bytes + 1200 > bytes_max) {
            bytes = NULL;
        }
        else {
            memset(bytes, 'Q', 1200);
            bytes += 1200;
        }
    }

    if (cnx->local_parameters.enable_loss_bit > 0 && bytes != NULL) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_enable_loss_bit,
            (cnx->local_parameters.enable_loss_bit > 1) ? 1 : 0);
    }

    if (bytes != NULL && cnx->local_parameters.min_ack_delay > 0) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_min_ack_delay,
            cnx->local_parameters.min_ack_delay);
    }

    if (cnx->local_parameters.enable_time_stamp > 0 && bytes != NULL) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_enable_time_stamp,
            cnx->local_parameters.enable_time_stamp);
    }

    if (cnx->local_parameters.do_grease_quic_bit && bytes != NULL) {
        bytes = picoquic_transport_param_type_flag_encode(bytes, bytes_max, picoquic_tp_grease_quic_bit);
    }

    if (cnx->do_version_negotiation && bytes != NULL) {
        bytes = picoquic_encode_transport_param_version_negotiation(bytes, bytes_max, extension_mode, cnx);
    }

    if (cnx->local_parameters.enable_bdp_frame > 0 && bytes != NULL) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, picoquic_tp_enable_bdp_frame,
            (uint64_t)cnx->local_parameters.enable_bdp_frame);
    }

    if (cnx->local_parameters.initial_max_path_id > 0 && bytes != NULL){
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max, 
            picoquic_tp_initial_max_path_id,
            (uint64_t)cnx->local_parameters.initial_max_path_id);
    }

    if (cnx->local_parameters.address_discovery_mode > 0 && bytes != NULL) {
        bytes = picoquic_transport_param_type_varint_encode(bytes, bytes_max,
            picoquic_tp_address_discovery,
            (uint64_t)(cnx->local_parameters.address_discovery_mode - 1));
    }

    if (cnx->local_parameters.is_reset_stream_at_enabled != 0 && bytes != NULL) {
        bytes = picoquic_transport_param_type_flag_encode(bytes, bytes_max, picoquic_tp_reset_stream_at);
    }

    /* This test extension must be the last one in the encoding, as it consumes all the available space */
    if (extension_mode == 1 && !cnx->test_large_chello &&
        cnx->quic->test_large_server_flight && bytes != NULL){
        size_t available = bytes_max - bytes;
        size_t pad_length = (available > 24) ? available - 24 : 1;

        if ((bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_tp_test_large_chello)) != NULL &&
            (bytes = picoquic_frames_varint_encode(bytes, bytes_max, pad_length)) != NULL) {
            if (bytes + pad_length > bytes_max) {
                bytes = NULL;
            }
            else {
                memset(bytes, 'Q', pad_length);
                bytes += pad_length;
            }
        }
    }

    if (bytes == NULL) {
        *consumed = 0;
        ret = PICOQUIC_ERROR_EXTENSION_BUFFER_TOO_SMALL;
    }
    else {
        *consumed = bytes - bytes_zero;
        picoquic_log_transport_extension(cnx, 1, *consumed, bytes_zero);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> i32 {
        let tp = &self.local_parameters;
        let mut out = Vec::new();
        let ok = encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxData as u64,
            tp.initial_max_data,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamDataBidiLocal as u64,
            tp.initial_max_stream_data_bidi_local,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamDataBidiRemote as u64,
            tp.initial_max_stream_data_bidi_remote,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamDataUni as u64,
            tp.initial_max_stream_data_uni,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamsBidi as u64,
            tp.initial_max_stream_id_bidir,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxStreamsUni as u64,
            tp.initial_max_stream_id_unidir,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::IdleTimeout as u64,
            tp.max_idle_timeout.ticks(),
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::MaxPacketSize as u64,
            tp.max_packet_size as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::MaxAckDelay as u64,
            tp.max_ack_delay as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::AckDelayExponent as u64,
            tp.ack_delay_exponent as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::ActiveConnectionIdLimit as u64,
            tp.active_connection_id_limit as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::MaxDatagramFrameSize as u64,
            tp.max_datagram_frame_size as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::EnableLossBit as u64,
            tp.enable_loss_bit as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::EnableTimeStamp as u64,
            tp.enable_time_stamp as u64,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::MinAckDelay as u64,
            tp.min_ack_delay.ticks(),
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::InitialMaxPathId as u64,
            tp.initial_max_path_id,
        ) && encode_tp_varint_param(
            &mut out,
            crate::tp::TransportParameter::AddressDiscovery as u64,
            tp.address_discovery_mode as u64,
        );
        if !ok {
            return -1;
        }
        if tp.migration_disabled
            && !encode_tp_param(
                &mut out,
                crate::tp::TransportParameter::DisableMigration as u64,
                &[],
            )
        {
            return -1;
        }
        if tp.do_grease_quic_bit
            && !encode_tp_varint_param(
                &mut out,
                crate::tp::TransportParameter::GreaseQuicBit as u64,
                1,
            )
        {
            return -1;
        }
        if tp.enable_bdp_frame
            && !encode_tp_varint_param(
                &mut out,
                crate::tp::TransportParameter::EnableBdpFrame as u64,
                1,
            )
        {
            return -1;
        }
        if tp.is_reset_stream_at_enabled
            && !encode_tp_varint_param(
                &mut out,
                crate::tp::TransportParameter::ResetStreamAt as u64,
                1,
            )
        {
            return -1;
        }
        if extension_mode != 0 {
            let mut vn = Vec::new();
            vn.extend_from_slice(&self.proposed_version.to_be_bytes());
            if self.desired_version != 0 {
                vn.extend_from_slice(&self.desired_version.to_be_bytes());
            }
            if !encode_tp_param(
                &mut out,
                crate::tp::TransportParameter::VersionNegotiation as u64,
                &vn,
            ) {
                return -1;
            }
        }
        if out.len() > bytes_max || out.len() > bytes.len() {
            return -1;
        }
        bytes[..out.len()].copy_from_slice(&out);
        *consumed = out.len();
        0
    }
```

## `picoquic/unified_log.c:picoquic_log_pdu`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C logs connection PDU events to text, binlog, and qlog after an is_still_logging check; Rust body logs only text quic_pdu and lacks binlog/qlog and still-logging behavior.
* C source: `picoquic/unified_log.c:110-131`
* C signature: `void picoquic_log_pdu(picoquic_cnx_t *, int, uint64_t, const struct sockaddr *, const struct sockaddr *, size_t, uint64_t, unsigned char)`
* Rust source: `rs/fq/src/logger.rs:264-284`
* Rust item: `log_pdu`

### C body
```c
{
    if (picoquic_cnx_is_still_logging(cnx)) {
        if (cnx->quic->F_log != NULL) {
            cnx->quic->text_log_fns->log_pdu(cnx, receiving, current_time, addr_peer, addr_local, packet_length,
                unique_path_id, ecn);
        }

        if (cnx->f_binlog != NULL) {
            cnx->quic->bin_log_fns->log_pdu(cnx, receiving, current_time, addr_peer, addr_local, packet_length,
                unique_path_id, ecn);
        }

        if (cnx->qlog_ctx != NULL) {
            cnx->quic->qlog_fns->log_pdu(cnx, receiving, current_time, addr_peer, addr_local, packet_length,
                unique_path_id, ecn);
        }
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().quic_pdu(
                self,
                receiving,
                current_time,
                cid64,
                addr_peer,
                addr_local,
                packet_length,
            );
            self.text_log_fns = Some(text);
        }
```

## `picoquic/util.c:picoquic_frames_uint24_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C writes three big-endian bytes, while the Rust body shown only checks buffer length and appears incomplete.
* C source: `picoquic/util.c:1014-1025`
* C signature: `uint8_t * picoquic_frames_uint24_encode(uint8_t *, const uint8_t *, uint32_t)`
* Rust source: `rs/fq/src/utils.rs:879-882`
* Rust item: `frames_uint24_encode`

### C body
```c
{
    if (bytes + 3 > bytes_max) {
        bytes = NULL;
    }
    else {
        *bytes++ = (uint8_t)(n >> 16);
        *bytes++ = (uint8_t)(n >> 8);
        *bytes++ = (uint8_t)n;
    }
    return (bytes);
}
```

### Rust body
```rust
    if bytes.len() < 3 {
        return None;
    }
```

## `picoquic/util.c:picoquic_get_ip_addr`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns a pointer and length for IPv4 or IPv6 addresses; Rust always returns None.
* C source: `picoquic/util.c:564-579`
* C signature: `void picoquic_get_ip_addr(struct sockaddr *, uint8_t **, uint8_t *)`
* Rust source: `rs/fq/src/utils.rs:541-547`
* Rust item: `get_ip_addr`

### C body
```c
{
    if (addr->sa_family == AF_INET) {
        *ip_addr = (uint8_t *)&((struct sockaddr_in *)addr)->sin_addr;
        *ip_addr_len = 4;
    }
    else if(addr->sa_family == AF_INET6) {
        *ip_addr = (uint8_t *)&((struct sockaddr_in6 *)addr)->sin6_addr;
        *ip_addr_len = 16;
    }
    else {
        *ip_addr = NULL;
        *ip_addr_len = 0;
    }
}
```

### Rust body
```rust
pub fn get_ip_addr(_addr: &SocketAddr) -> Option<&[u8]> {
    // `SocketAddr` exposes IP octets only by value (`IpAddr::octets()`),
    // not by reference; safe Rust cannot produce the required borrow.
    // Callers should use `addr.ip()` with `Ipv4Addr::octets()` /
    // `Ipv6Addr::octets()` directly.
    None
}
```

## `picoquic/bbr.c:BBRCheckStartupDone`
* Phase 4C status: `suspect`
* Phase 4C rationale: Core flow matches, but C wraps the RTT-too-high filled_pipe update in an #ifdef while Rust executes it unconditionally.
* C source: `picoquic/bbr.c:2042-2059`
* C signature: `void BBRCheckStartupDone(picoquic_bbr_state_t *, picoquic_path_t *, bbr_per_ack_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:1514-1529`
* Rust item: `check_startup_done`

### C body
```c
{
    if (bbr_state->state == picoquic_bbr_alg_startup) {
        BBRCheckStartupFullBandwidth(bbr_state, rs);
        BBRCheckStartupHighLoss(bbr_state, path_x, rs);
#ifdef RTTJitterBufferStartup
        if (bbr_state->min_rtt > PICOQUIC_MINRTT_THRESHOLD && IsRTTTooHigh(bbr_state)) {
            bbr_state->filled_pipe = 1;
        }
#endif
        if (bbr_state->filled_pipe) {
            bbr_state->probe_probe_bw_quickly = 1;
            bbr_state->full_bw_count = 0;
            BBREnterDrain(bbr_state, path_x);
        }
    }
}
```

### Rust body
```rust
    pub fn check_startup_done(&mut self, path_x: &mut Path, rs: &BbrPerAckState) {
        if self.state != BbrAlgState::Startup {
            return;
        }
        self.check_startup_full_bandwidth(rs);
        self.check_startup_high_loss(path_x, rs);
        // RTTJitterBufferStartup is defined in bbr.c:37.
        if self.min_rtt > MINRTT_THRESHOLD && self.is_rtt_too_high() {
            self.filled_pipe = true;
        }
        if self.filled_pipe {
            self.probe_probe_bw_quickly = true;
            self.full_bw_count = 0;
            self.enter_drain(path_x);
        }
    }
```
