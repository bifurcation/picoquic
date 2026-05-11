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

## Pair `picoquic/tls_api.c:picoquic_export_secret`
C: `picoquic/tls_api.c:2225-2238 picoquic_export_secret`
Rust: `rs/fq/src/lib.rs:1768-1781 export_secret`

### C body
```c
{
    if (cnx == NULL || label == NULL || out == NULL || outlen == 0) {
        return -1;
    }
    PICOQUIC_THREAD_CHECK(cnx->quic);
    picoquic_tls_ctx_t *tls_ctx = (picoquic_tls_ctx_t *)cnx->tls_ctx;
    if (tls_ctx == NULL || tls_ctx->tls == NULL) {
        return PTLS_ERROR_IN_PROGRESS;
    }

    ptls_t* tls = tls_ctx->tls;
    return ptls_export_secret(tls, out, outlen, label, ptls_iovec_init(NULL, 0), 0);
}
```

### Rust body
```rust
    pub fn set_spinbit_policy(&mut self, spinbit_policy: SpinbitVersion) -> Result<(), Error> {
        // SpinbitVersion::On is server-only.
        if spinbit_policy == SpinbitVersion::On {
            return Err(Error::InvalidArgument);
        }
        self.spin_policy = spinbit_policy;
        Ok(())
    }
```

## Pair `picoquic/tls_api.c:picoquic_pn_encrypt`
C: `picoquic/tls_api.c:2374-2378 picoquic_pn_encrypt`
Rust: `rs/fq/src/tls_api.rs:2155-2159 pn_encrypt`

### C body
```c
{
    ptls_cipher_init((ptls_cipher_context_t *) pn_enc, iv);
    ptls_cipher_encrypt((ptls_cipher_context_t *) pn_enc, output, input, len);
}
```

### Rust body
```rust
pub fn pn_encrypt(pn_enc: &dyn crate::tls::HeaderKey, sample: &[u8; 16], output: &mut [u8]) {
    let mask = pn_enc.mask(*sample);
    let n = output.len().min(mask.len());
    output[..n].copy_from_slice(&mask[..n]);
}
```

## Pair `picoquic/tls_api.c:picoquic_aead_integrity_limit`
C: `picoquic/tls_api.c:2444-2448 picoquic_aead_integrity_limit`
Rust: `rs/fq/src/tls_api.rs:1319-1321 aead_integrity_limit`

### C body
```c
{
    return ((ptls_aead_context_t*)aead_ctx)->algo->integrity_limit;
}
```

### Rust body
```rust
pub fn aead_integrity_limit(aead_ctx: &dyn crate::tls::PacketKey) -> u64 {
    aead_ctx.integrity_limit()
}
```

## Pair `picoquic/tls_api.c:picoquic_aead_decrypt_mp`
C: `picoquic/tls_api.c:2485-2505 picoquic_aead_decrypt_mp`
Rust: `rs/fq/src/lib.rs:5204-5219 aead_decrypt_mp`

### C body
```c
{
    size_t decrypted = 0;

    if (aead_context == NULL) {
        decrypted = SIZE_MAX;
    }
    else {
        uint8_t seq32[4];

        picoformat_32(seq32, (uint32_t)path_id);
        ptls_aead_xor_iv((ptls_aead_context_t*)aead_context, seq32, sizeof(seq32));
        decrypted = ptls_aead_decrypt((ptls_aead_context_t*)aead_context,
            (void*)output, (const void*)input, input_length, seq_num,
            (void*)auth_data, auth_data_length);
        ptls_aead_xor_iv((ptls_aead_context_t*)aead_context, seq32, sizeof(seq32));
    }

    return decrypted;
}
```

### Rust body
```rust
) -> Option<usize> {
    let mut payload = input.to_vec();
    ctx.decrypt_mp(path_id, sequence, aad, &mut payload).ok()?;
    if payload.len() > out.len() {
        return None;
    }
    out[..payload.len()].copy_from_slice(&payload);
    Some(payload.len())
}
```

## Pair `picoquic/tls_api.c:picoquic_is_tls_complete`
C: `picoquic/tls_api.c:2760-2767 picoquic_is_tls_complete`
Rust: `rs/fq/src/tls_api.rs:1120-1139 is_tls_complete`

### C body
```c
{
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;
    return ptls_handshake_is_complete(ctx->tls);
}
```

### Rust body
```rust
    pub fn initialize_tls_stream(&mut self, current_time: Instant) -> Result<(), Error> {
        let mut session = self.tls_ctx.take().ok_or(Error::InvalidState)?;
        if let Some(keys) = session.write_handshake(&mut self.tls_sendbuf) {
            install_key_pair(&mut self.crypto_context[3], keys);
        }
        let out = core::mem::take(&mut self.tls_sendbuf);
        queue_tls_bytes(self, 0, &out)?;
        if !session.is_handshaking() {
            self.ready_state_transition(current_time);
        }
        self.tls_ctx = Some(session);
        Ok(())
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_client_authentication_activated`
C: `picoquic/tls_api.c:2819-2821 picoquic_tls_client_authentication_activated`
Rust: `rs/fq/src/tls_api.rs:1704-1712 tls_client_authentication_activated`

### C body
```c
int picoquic_tls_client_authentication_activated(picoquic_quic_t* quic) {
    return ((ptls_context_t*)quic->tls_master_ctx)->require_client_authentication;
}
```

### Rust body
```rust
    pub fn tls_set_use_exporter(&mut self, use_exporter: bool) {
        self.use_exporter = use_exporter;
    }
```

## Pair `picoquic/tls_api.c:picoquic_prepare_retry_token`
C: `picoquic/tls_api.c:2923-2963 picoquic_prepare_retry_token`
Rust: `rs/fq/src/tls_api.rs:1814-1860 prepare_retry_token`

### C body
```c
{
    int ret = 0;
    uint8_t text[128];
    uint64_t token_time = current_time;
    uint8_t* bytes = text;
    uint8_t* bytes_max = text + sizeof(text);

    /* The prepare token function can prepare two kind of tokens: immediate retry
    * tokens, that are short lived, or "new tokens", that are valid for a
    * a longer delay. We differentiate between these two cases by testing the
    * presence of the Original DCID parameter, which must be present in
    * the retry tokens, but shall never be in the new tokens. */
    if (odcid->id_len == 0) {
        token_time += PICOQUIC_TOKEN_DELAY_LONG;
    }
    else {
        token_time += PICOQUIC_TOKEN_DELAY_SHORT;
    }
    /* serialize the token components */
    if ((bytes = picoquic_frames_uint64_encode(bytes, bytes_max, token_time)) != NULL &&
        (bytes = picoquic_frames_cid_encode(bytes, bytes_max, odcid)) != NULL &&
        (bytes = picoquic_frames_cid_encode(bytes, bytes_max, rcid)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, initial_pn)) != NULL) {
        /* Pad to min token size */
        while (bytes < text + PICOQUIC_RETRY_TOKEN_PAD_SIZE) {
            *bytes++ = 0;
        }
        /* Encode the clear text components */
        ret = picoquic_server_encrypt_retry_token(quic, addr_peer, odcid->id_len == 0,
            token, token_size, token_max, text, bytes - text);
    }
    else {
        ret = -1;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<usize, Error> {
        ensure_ticket_aead_contexts(self)?;
        let delay = if odcid.is_empty() {
            TOKEN_DELAY_LONG
        } else {
            TOKEN_DELAY_SHORT
        };
        let token_time = current_time + delay;
        let mut text = [0u8; 128];
        let mut offset = 0usize;
        text[offset..offset + 8].copy_from_slice(&token_time.ticks().to_be_bytes());
        offset += 8;
        {
            let rest = crate::utils::frames_cid_encode(&mut text[offset..], odcid)
                .ok_or(Error::BufferTooSmall)?;
            offset = 128 - rest.len();
        }
        {
            let rest = crate::utils::frames_cid_encode(&mut text[offset..], rcid)
                .ok_or(Error::BufferTooSmall)?;
            offset = 128 - rest.len();
        }
        {
            let rest = crate::utils::frames_varint_encode(&mut text[offset..], initial_pn as u64)
                .ok_or(Error::BufferTooSmall)?;
            offset = 128 - rest.len();
        }
        while offset < RETRY_TOKEN_PAD_SIZE {
            text[offset] = 0;
            offset += 1;
        }

        self.picoquic_server_encrypt_retry_token(
            addr_peer,
            odcid.is_empty(),
            token,
            &text[..offset],
        )
    }
```

## Pair `picoquic/tls_api.c:picoquic_delete_retry_protection_contexts`
C: `picoquic/tls_api.c:3167-3171 picoquic_delete_retry_protection_contexts`
Rust: `rs/fq/src/tls_api.rs:2081-2084 delete_retry_protection_contexts`

### C body
```c
{
    quic->retry_integrity_sign_ctx = picoquic_delete_one_retry_protection_context(quic->retry_integrity_sign_ctx);
    quic->retry_integrity_verify_ctx = picoquic_delete_one_retry_protection_context(quic->retry_integrity_verify_ctx);
}
```

### Rust body
```rust
    pub fn delete_retry_protection_contexts(&mut self) {
        self.retry_integrity_sign_ctx.clear();
        self.retry_integrity_verify_ctx.clear();
    }
```

## Pair `picoquic/token_store.c:picoquic_format_token`
C: `picoquic/token_store.c:29-59 picoquic_format_token`
Rust: `rs/fq/src/tls_api.rs:2747-2778 format_token`

### C body
```c
{
    size_t token_size = sizeof(picoquic_stored_token_t) + sni_length + 1 + ip_addr_length + 1 + token_length;
    picoquic_stored_token_t* stored = (picoquic_stored_token_t*)malloc(token_size);
    
    if (stored != NULL) {
        uint8_t* next_p = ((uint8_t*)stored) + sizeof(picoquic_stored_token_t);

        memset(stored, 0, token_size);
        stored->time_valid_until = time_valid_until;
        stored->sni = (char const *)next_p;
        stored->sni_length = sni_length;
        memcpy(next_p, sni, sni_length);
        next_p += sni_length;
        *next_p++ = 0;

        stored->ip_addr = next_p;
        stored->ip_addr_length = ip_addr_length;
        memcpy(next_p, ip_addr, ip_addr_length);
        next_p += ip_addr_length;
        *next_p++ = 0;

        stored->token = next_p;
        stored->token_length = token_length;
        memcpy(next_p, token, token_length);
    }

    return stored;
}
```

### Rust body
```rust
impl Connection {
    /// Push a proposed ALPN string onto the connection's handshake ALPN list.
    /// C: `picoquic_add_proposed_alpn` (tls_api.c:2207).
    ///
    /// In C: pushed into `tls_ctx->alpn_vec[alpn_count]` with a bounds check
    /// against `alpn_vec_size` (max [`crate::internal::ALPN_NUMBER_MAX`]).
    /// In Rust: stored in `Connection::alpn_proposals`; the TLS backend reads
    /// this slice when starting the handshake.  For server-side ALPN selection
    /// use [`crate::Quic::set_alpn_select_fn`] or `Quic::default_alpn`.
    pub fn add_proposed_alpn(&mut self, alpn: &str) -> Result<(), Error> {
        if self.alpn_proposals.len() >= crate::internal::ALPN_NUMBER_MAX {
            return Err(Error::BufferTooSmall);
        }
        self.alpn_proposals.push(alpn.to_owned());
        Ok(())
    }
}
```

## Pair `picoquic/token_store.c:picoquic_get_token`
C: `picoquic/token_store.c:204-244 picoquic_get_token`
Rust: `rs/fq/src/internal.rs:1743-1758 get_token`

### C body
```c
{
    int ret = 0;

    uint64_t current_time = picoquic_get_tls_time(quic);
    picoquic_stored_token_t* p_first_token = quic->p_first_token;
    picoquic_stored_token_t* next = p_first_token;
    picoquic_stored_token_t* best_match = NULL;

    while (next != NULL) {
        if (next->time_valid_until > current_time && next->sni_length == sni_length && memcmp(next->sni, sni, sni_length) == 0 && next->was_used == 0){
            if (ip_addr_length > 0) {
                if (next->ip_addr_length == ip_addr_length && memcmp(next->ip_addr, ip_addr, ip_addr_length) == 0) {
                    best_match = next;
                    break;
                }
            }
            else {
                if (best_match == NULL || next->time_valid_until > best_match->time_valid_until) {
                    best_match = next;
                }
            }
        } 
        next = next->next_token;
    }

    if (best_match == NULL || best_match->token_length == 0 || (*token = (uint8_t *)malloc(best_match->token_length)) == NULL) {
        *token = NULL;
        *token_length = 0;
        ret = -1;
    } else {
        *token_length = best_match->token_length;
        memcpy(*token, (uint8_t*)best_match->token, best_match->token_length);
        best_match->was_used = mark_used;
    }

    return ret;
}
```

### Rust body
```rust
            .position(|token| {
                token.time_valid_until.ticks() > 0
                    && token.sni.as_deref() == sni
                    && token.ip_addr == ip_addr
                    && !token.was_used
                    && !token.token.is_empty()
            })
```

## Pair `picoquic/transport.c:picoquic_transport_param_varint_decode`
C: `picoquic/transport.c:31-41 picoquic_transport_param_varint_decode`
Rust: `rs/fq/src/internal.rs:15021-15035 picoquic_transport_param_varint_decode`

### C body
```c
{
    uint64_t n64 = 0;
    uint64_t l_v = picoquic_varint_decode(bytes, (size_t)extension_length, &n64);

    if (l_v == 0 || l_v != extension_length) {
        *ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0);
    }

    return n64;
}
```

### Rust body
```rust
) -> u64 {
    let mut n64 = 0;
    let len = usize::try_from(extension_length).unwrap_or(usize::MAX);
    let slice = bytes.get(..len).unwrap_or(bytes);
    let l_v = varint_decode(slice, &mut n64) as u64;
    if l_v == 0 || l_v != extension_length {
        *ret = cnx.connection_error(crate::errors::TransportError::ParameterError as u64, 0);
    }
    n64
}
```

## Pair `picoquic/transport.c:picoquic_transport_param_cid_encode`
C: `picoquic/transport.c:77-85 picoquic_transport_param_cid_encode`
Rust: `rs/fq/src/internal.rs:15071-15086 picoquic_transport_param_cid_encode`

### C body
```c
{
    if (bytes != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, tp_type)) != NULL) {
        /* frame encoding includes length and the value. */
        bytes = picoquic_frames_cid_encode(bytes, bytes_max, cid);
    }
    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut offset = 0;
    if !encode_varint_at(bytes, &mut offset, tp_type as u64)
        || !encode_varint_at(bytes, &mut offset, cid.len() as u64)
        || bytes.len().saturating_sub(offset) < cid.len()
    {
        return None;
    }
    bytes[offset..offset + cid.len()].copy_from_slice(cid.as_bytes());
    offset += cid.len();
    Some(&mut bytes[offset..])
}
```

## Pair `picoquic/transport.c:picoquic_encode_transport_param_version_negotiation`
C: `picoquic/transport.c:173-224 picoquic_encode_transport_param_version_negotiation`
Rust: `rs/fq/src/internal.rs:15220-15268 picoquic_encode_transport_param_version_negotiation`

### C body
```c
{
    uint8_t* bytes_len;
    bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_tp_version_negotiation);
    bytes_len = bytes;


    if (bytes != NULL &&
        (bytes = picoquic_frames_uint16_encode(bytes, bytes_max, 0)) != NULL &&
        (bytes = picoquic_frames_uint32_encode(bytes, bytes_max,
            picoquic_supported_versions[cnx->version_index].version)) != NULL) {
        if (extension_mode == 0) {
            if (cnx->desired_version != 0 && cnx->desired_version != picoquic_supported_versions[cnx->version_index].version) {
                bytes = picoquic_frames_uint32_encode(bytes, bytes_max, cnx->desired_version);
            }
            if (bytes != NULL) {
                bytes = picoquic_frames_uint32_encode(bytes, bytes_max,
                    picoquic_supported_versions[cnx->version_index].version);
            }
        }
        else {
            for (size_t i = 0; i < picoquic_nb_supported_versions; i++) {
                if ((bytes = picoquic_frames_uint32_encode(bytes, bytes_max,
                    picoquic_supported_versions[i].version)) == NULL) {
                    break;
                }
            }
        }
    }

    if (bytes != NULL) {
        size_t len = bytes - (bytes_len + 2);

        if (len > 0x3FFF) {
            bytes = NULL;
        }
        else {
            bytes_len[0] = (uint8_t)((len >> 8) & 0x3f) | 0x40;
            bytes_len[1] = (uint8_t)(len & 0xff);
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let mut offset = 0;
    if !encode_varint_at(
        bytes,
        &mut offset,
        crate::tp::TransportParameter::VersionNegotiation as u64,
    ) || bytes.len().saturating_sub(offset) < 2
    {
        return None;
    }
    let bytes_len = offset;
    offset += 2;

    let current_version = supported_version_from_index(cnx.version_index)
        .map(|v| v as u32)
        .unwrap_or(cnx.proposed_version);
    if !encode_u32_at(bytes, &mut offset, current_version) {
        return None;
    }
    if extension_mode == 0 {
        if cnx.desired_version != 0
            && cnx.desired_version != current_version
            && !encode_u32_at(bytes, &mut offset, cnx.desired_version)
        {
            return None;
        }
        if !encode_u32_at(bytes, &mut offset, current_version) {
            return None;
        }
    } else {
        for version in SUPPORTED_VERSIONS {
            if !encode_u32_at(bytes, &mut offset, version as u32) {
                return None;
            }
        }
    }

    let len = offset - (bytes_len + 2);
    if len > 0x3fff {
        return None;
    }
    bytes[bytes_len] = ((len >> 8) as u8 & 0x3f) | 0x40;
    bytes[bytes_len + 1] = len as u8;
    Some(&mut bytes[offset..])
}
```

## Pair `picoquic/transport.c:picoquic_clear_transport_extensions`
C: `picoquic/transport.c:503-532 picoquic_clear_transport_extensions`
Rust: `rs/fq/src/internal.rs:15472-15500 picoquic_clear_transport_extensions`

### C body
```c
{
    cnx->remote_parameters.initial_max_stream_data_bidi_local = 0;
    picoquic_update_stream_initial_remote(cnx);
    cnx->remote_parameters.initial_max_stream_data_bidi_remote = 0;
    picoquic_update_stream_initial_remote(cnx);
    cnx->remote_parameters.initial_max_stream_data_uni = 0;
    picoquic_update_stream_initial_remote(cnx);
    cnx->remote_parameters.initial_max_data = 0;
    cnx->maxdata_remote = cnx->remote_parameters.initial_max_data;
    cnx->remote_parameters.initial_max_stream_id_bidir = 0;
    cnx->max_stream_id_bidir_remote = 0;
    cnx->remote_parameters.max_idle_timeout = 0;
    cnx->remote_parameters.max_packet_size = 1500;
    cnx->remote_parameters.ack_delay_exponent = 3;
    cnx->remote_parameters.initial_max_stream_id_unidir = 0;
    cnx->max_stream_id_unidir_remote = 0;
    cnx->remote_parameters.migration_disabled = 0;
    cnx->remote_parameters.max_ack_delay = PICOQUIC_ACK_DELAY_MAX_DEFAULT;
    cnx->remote_parameters.max_datagram_frame_size = 0;
    cnx->remote_parameters.active_connection_id_limit = 0;
    cnx->remote_parameters.enable_loss_bit = 0;
    cnx->remote_parameters.enable_time_stamp = 0;
    cnx->remote_parameters.min_ack_delay = 0;
    cnx->remote_parameters.do_grease_quic_bit = 0;
    cnx->remote_parameters.enable_bdp_frame = 0;
    cnx->remote_parameters.initial_max_path_id = 0;
    cnx->remote_parameters.address_discovery_mode = 0;
    cnx->remote_parameters.is_reset_stream_at_enabled = 0;
}
```

### Rust body
```rust
    pub fn picoquic_clear_transport_extensions(&mut self) {
        self.remote_parameters.initial_max_stream_data_bidi_local = 0;
        self.update_stream_initial_remote();
        self.remote_parameters.initial_max_stream_data_bidi_remote = 0;
        self.update_stream_initial_remote();
        self.remote_parameters.initial_max_stream_data_uni = 0;
        self.update_stream_initial_remote();
        self.remote_parameters.initial_max_data = 0;
        self.maxdata_remote = self.remote_parameters.initial_max_data;
        self.remote_parameters.initial_max_stream_id_bidir = 0;
        self.max_stream_id_bidir_remote = 0;
        self.remote_parameters.max_idle_timeout = Duration::from_ticks(0);
        self.remote_parameters.max_packet_size = 1500;
        self.remote_parameters.ack_delay_exponent = 3;
        self.remote_parameters.initial_max_stream_id_unidir = 0;
        self.max_stream_id_unidir_remote = 0;
        self.remote_parameters.migration_disabled = false;
        self.remote_parameters.max_ack_delay = ACK_DELAY_MAX_DEFAULT.ticks() as u32;
        self.remote_parameters.max_datagram_frame_size = 0;
        self.remote_parameters.active_connection_id_limit = 0;
        self.remote_parameters.enable_loss_bit = 0;
        self.remote_parameters.enable_time_stamp = 0;
        self.remote_parameters.min_ack_delay = Duration::from_ticks(0);
        self.remote_parameters.do_grease_quic_bit = false;
        self.remote_parameters.enable_bdp_frame = false;
        self.remote_parameters.initial_max_path_id = 0;
        self.remote_parameters.address_discovery_mode = 0;
        self.remote_parameters.is_reset_stream_at_enabled = false;
    }
```

## Pair `picoquic/unified_log.c:picoquic_log_app_message_v`
C: `picoquic/unified_log.c:59-73 picoquic_log_app_message_v`
Rust: `rs/fq/src/textlog.rs:98-100 log_app_message_v`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    if (cnx->quic->F_log != NULL) {
        cnx->quic->text_log_fns->log_app_message(cnx, fmt, vargs);
    }

    if (cnx->f_binlog != NULL) {
        cnx->quic->bin_log_fns->log_app_message(cnx, fmt, vargs);
    }

    if (cnx->qlog_ctx != NULL) {
        cnx->quic->qlog_fns->log_app_message(cnx, fmt, vargs);
    }
}
```

### Rust body
```rust
    pub fn log_app_message_v(&mut self, args: core::fmt::Arguments<'_>) {
        crate::logger::Log::app_message(self, args);
    }
```

## Pair `picoquic/unified_log.c:picoquic_log_packet`
C: `picoquic/unified_log.c:133-150 picoquic_log_packet`
Rust: `rs/fq/src/logger.rs:340-884 packet`

### C body
```c
{
    if (picoquic_cnx_is_still_logging(cnx)) {
        if (cnx->quic->F_log != NULL) {
            cnx->quic->text_log_fns->log_packet(cnx, path_x, receiving, current_time, ph, bytes, bytes_max);
        }

        if (cnx->f_binlog != NULL) {
            cnx->quic->bin_log_fns->log_packet(cnx, path_x, receiving, current_time, ph, bytes, bytes_max);
        }

        if (cnx->qlog_ctx != NULL) {
            cnx->quic->qlog_fns->log_packet(cnx, path_x, receiving, current_time, ph, bytes, bytes_max);
        }
    }
}
```

### Rust body
```rust
impl Log for Connection {
    fn app_message(&mut self, args: core::fmt::Arguments<'_>) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().app_message(self, args);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().app_message(self, args);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().app_message(self, args);
        }
    }

    fn pdu(
        &mut self,
        receiving: bool,
        current_time: Instant,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        packet_length: usize,
        unique_path_id: u64,
        ecn: u8,
    ) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().pdu(
                self,
                receiving,
                current_time,
                addr_peer,
                addr_local,
                packet_length,
                unique_path_id,
                ecn,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().pdu(
                self,
                receiving,
                current_time,
                addr_peer,
                addr_local,
                packet_length,
                unique_path_id,
                ecn,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().pdu(
                self,
                receiving,
                current_time,
                addr_peer,
                addr_local,
                packet_length,
                unique_path_id,
                ecn,
            );
        }
    }

    fn packet(
        &mut self,
        path_x: Option<&mut Path>,
        receiving: bool,
        current_time: Instant,
        ph: &PacketHeader,
        bytes: &[u8],
    ) {
        if !self.is_still_logging() {
            return;
        }

        let mut path_x = path_x;

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().packet(
                self,
                option_path(&mut path_x),
                receiving,
                current_time,
                ph,
                bytes,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().packet(
                self,
                option_path(&mut path_x),
                receiving,
                current_time,
                ph,
                bytes,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().packet(
                self,
                option_path(&mut path_x),
                receiving,
                current_time,
                ph,
                bytes,
            );
        }
    }

    fn dropped_packet(
        &mut self,
        path_x: Option<&mut Path>,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    ) {
        if !self.is_still_logging() {
            return;
        }

        let mut path_x = path_x;

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().dropped_packet(
                self,
                option_path(&mut path_x),
                ph,
                packet_size,
                err,
                current_time,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().dropped_packet(
                self,
                option_path(&mut path_x),
                ph,
                packet_size,
                err,
                current_time,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().dropped_packet(
                self,
                option_path(&mut path_x),
                ph,
                packet_size,
                err,
                current_time,
            );
        }
    }

    fn buffered_packet(&mut self, path_x: &mut Path, ptype: PacketType, current_time: Instant) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .buffered_packet(self, path_x, ptype, current_time);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut()
                .buffered_packet(self, path_x, ptype, current_time);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut()
                .buffered_packet(self, path_x, ptype, current_time);
        }
    }

    fn outgoing_packet(
        &mut self,
        path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: Instant,
    ) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().outgoing_packet(
                self,
                path_x,
                bytes,
                sequence_number,
                pn_length,
                send_buffer,
                current_time,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().outgoing_packet(
                self,
                path_x,
                bytes,
                sequence_number,
                pn_length,
                send_buffer,
                current_time,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().outgoing_packet(
                self,
                path_x,
                bytes,
                sequence_number,
                pn_length,
                send_buffer,
                current_time,
            );
        }
    }

    fn packet_lost(
        &mut self,
        path_x: &mut Path,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: Instant,
    ) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().packet_lost(
                self,
                path_x,
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().packet_lost(
                self,
                path_x,
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().packet_lost(
                self,
                path_x,
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }
    }

    fn negotiated_alpn(&mut self, is_local: bool, sni: &[u8], alpn: &[u8], alpn_list: &[&[u8]]) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }
    }

    fn transport_extension(&mut self, is_local: bool, params: &[u8]) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .transport_extension(self, is_local, params);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().transport_extension(self, is_local, params);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut()
                .transport_extension(self, is_local, params);
        }
    }

    fn tls_ticket(&mut self, ticket: &[u8]) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().tls_ticket(self, ticket);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().tls_ticket(self, ticket);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().tls_ticket(self, ticket);
        }
    }

    fn new_connection(&mut self) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().new_connection(self);
        }

        if let Some(bin) = logger_ref(&self.bin_log_fns) {
            bin.borrow_mut().new_connection(self);
        }

        if let Some(qlog) = logger_ref(&self.qlog_fns) {
            qlog.borrow_mut().new_connection(self);
        }
    }

    fn close_connection(&mut self) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().close_connection(self);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().close_connection(self);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().close_connection(self);
        }
    }

    fn cc_dump(&mut self, current_time: Instant) {
        let mut paths = core::mem::take(&mut self.paths);

        if let Some(mut memlog) = self.memlog_call_back.take() {
            if let Some(path0) = paths.first_mut() {
                memlog.callback(self, path0, 0, current_time);
            }
            self.memlog_call_back = Some(memlog);
        }

        if self.is_still_logging() {
            for path_x in &mut paths {
                if !path_x.is_cc_data_updated {
                    continue;
                }

                if let Some(text) = logger_ref(&self.text_log_fns) {
                    text.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.f_binlog.is_some()
                    && let Some(bin) = logger_ref(&self.bin_log_fns)
                {
                    bin.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.qlog_ctx.is_some()
                    && let Some(qlog) = logger_ref(&self.qlog_fns)
                {
                    qlog.borrow_mut().cc_dump(self, path_x, current_time);
                }

                path_x.is_cc_data_updated = false;
            }
        }

        self.paths = paths;
    }
}
```

## Pair `picoquic/unified_log.c:picoquic_log_packet_lost`
C: `picoquic/unified_log.c:212-231 picoquic_log_packet_lost`
Rust: `rs/fq/src/logger.rs:389-884 packet_lost`

### C body
```c
{
    if (picoquic_cnx_is_still_logging(cnx)) {
        if (cnx->quic->F_log != NULL) {
            cnx->quic->text_log_fns->log_packet_lost(cnx, path_x, ptype, sequence_number, trigger, dcid, packet_size, current_time);
        }

        if (cnx->f_binlog != NULL) {
            cnx->quic->bin_log_fns->log_packet_lost(cnx, path_x, ptype, sequence_number, trigger, dcid, packet_size, current_time);
        }

        if (cnx->qlog_ctx != NULL) {
            cnx->quic->qlog_fns->log_packet_lost(cnx, path_x, ptype, sequence_number, trigger, dcid, packet_size, current_time);
        }
    }
}
```

### Rust body
```rust
impl Log for Connection {
    fn app_message(&mut self, args: core::fmt::Arguments<'_>) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().app_message(self, args);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().app_message(self, args);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().app_message(self, args);
        }
    }

    fn pdu(
        &mut self,
        receiving: bool,
        current_time: Instant,
        addr_peer: &SocketAddr,
        addr_local: &SocketAddr,
        packet_length: usize,
        unique_path_id: u64,
        ecn: u8,
    ) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().pdu(
                self,
                receiving,
                current_time,
                addr_peer,
                addr_local,
                packet_length,
                unique_path_id,
                ecn,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().pdu(
                self,
                receiving,
                current_time,
                addr_peer,
                addr_local,
                packet_length,
                unique_path_id,
                ecn,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().pdu(
                self,
                receiving,
                current_time,
                addr_peer,
                addr_local,
                packet_length,
                unique_path_id,
                ecn,
            );
        }
    }

    fn packet(
        &mut self,
        path_x: Option<&mut Path>,
        receiving: bool,
        current_time: Instant,
        ph: &PacketHeader,
        bytes: &[u8],
    ) {
        if !self.is_still_logging() {
            return;
        }

        let mut path_x = path_x;

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().packet(
                self,
                option_path(&mut path_x),
                receiving,
                current_time,
                ph,
                bytes,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().packet(
                self,
                option_path(&mut path_x),
                receiving,
                current_time,
                ph,
                bytes,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().packet(
                self,
                option_path(&mut path_x),
                receiving,
                current_time,
                ph,
                bytes,
            );
        }
    }

    fn dropped_packet(
        &mut self,
        path_x: Option<&mut Path>,
        ph: &PacketHeader,
        packet_size: usize,
        err: i32,
        current_time: Instant,
    ) {
        if !self.is_still_logging() {
            return;
        }

        let mut path_x = path_x;

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().dropped_packet(
                self,
                option_path(&mut path_x),
                ph,
                packet_size,
                err,
                current_time,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().dropped_packet(
                self,
                option_path(&mut path_x),
                ph,
                packet_size,
                err,
                current_time,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().dropped_packet(
                self,
                option_path(&mut path_x),
                ph,
                packet_size,
                err,
                current_time,
            );
        }
    }

    fn buffered_packet(&mut self, path_x: &mut Path, ptype: PacketType, current_time: Instant) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .buffered_packet(self, path_x, ptype, current_time);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut()
                .buffered_packet(self, path_x, ptype, current_time);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut()
                .buffered_packet(self, path_x, ptype, current_time);
        }
    }

    fn outgoing_packet(
        &mut self,
        path_x: &mut Path,
        bytes: &[u8],
        sequence_number: u64,
        pn_length: usize,
        send_buffer: &[u8],
        current_time: Instant,
    ) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().outgoing_packet(
                self,
                path_x,
                bytes,
                sequence_number,
                pn_length,
                send_buffer,
                current_time,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().outgoing_packet(
                self,
                path_x,
                bytes,
                sequence_number,
                pn_length,
                send_buffer,
                current_time,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().outgoing_packet(
                self,
                path_x,
                bytes,
                sequence_number,
                pn_length,
                send_buffer,
                current_time,
            );
        }
    }

    fn packet_lost(
        &mut self,
        path_x: &mut Path,
        ptype: PacketType,
        sequence_number: u64,
        trigger: &str,
        dcid: Option<&ConnectionId>,
        packet_size: usize,
        current_time: Instant,
    ) {
        if !self.is_still_logging() {
            return;
        }

        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().packet_lost(
                self,
                path_x,
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().packet_lost(
                self,
                path_x,
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().packet_lost(
                self,
                path_x,
                ptype,
                sequence_number,
                trigger,
                dcid,
                packet_size,
                current_time,
            );
        }
    }

    fn negotiated_alpn(&mut self, is_local: bool, sni: &[u8], alpn: &[u8], alpn_list: &[&[u8]]) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut()
                .negotiated_alpn(self, is_local, sni, alpn, alpn_list);
        }
    }

    fn transport_extension(&mut self, is_local: bool, params: &[u8]) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut()
                .transport_extension(self, is_local, params);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().transport_extension(self, is_local, params);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut()
                .transport_extension(self, is_local, params);
        }
    }

    fn tls_ticket(&mut self, ticket: &[u8]) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().tls_ticket(self, ticket);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().tls_ticket(self, ticket);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().tls_ticket(self, ticket);
        }
    }

    fn new_connection(&mut self) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().new_connection(self);
        }

        if let Some(bin) = logger_ref(&self.bin_log_fns) {
            bin.borrow_mut().new_connection(self);
        }

        if let Some(qlog) = logger_ref(&self.qlog_fns) {
            qlog.borrow_mut().new_connection(self);
        }
    }

    fn close_connection(&mut self) {
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().close_connection(self);
        }

        if self.f_binlog.is_some()
            && let Some(bin) = logger_ref(&self.bin_log_fns)
        {
            bin.borrow_mut().close_connection(self);
        }

        if self.qlog_ctx.is_some()
            && let Some(qlog) = logger_ref(&self.qlog_fns)
        {
            qlog.borrow_mut().close_connection(self);
        }
    }

    fn cc_dump(&mut self, current_time: Instant) {
        let mut paths = core::mem::take(&mut self.paths);

        if let Some(mut memlog) = self.memlog_call_back.take() {
            if let Some(path0) = paths.first_mut() {
                memlog.callback(self, path0, 0, current_time);
            }
            self.memlog_call_back = Some(memlog);
        }

        if self.is_still_logging() {
            for path_x in &mut paths {
                if !path_x.is_cc_data_updated {
                    continue;
                }

                if let Some(text) = logger_ref(&self.text_log_fns) {
                    text.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.f_binlog.is_some()
                    && let Some(bin) = logger_ref(&self.bin_log_fns)
                {
                    bin.borrow_mut().cc_dump(self, path_x, current_time);
                }

                if self.qlog_ctx.is_some()
                    && let Some(qlog) = logger_ref(&self.qlog_fns)
                {
                    qlog.borrow_mut().cc_dump(self, path_x, current_time);
                }

                path_x.is_cc_data_updated = false;
            }
        }

        self.paths = paths;
    }
}
```

## Pair `picoquic/unified_log.c:picoquic_log_new_connection`
C: `picoquic/unified_log.c:284-298 picoquic_log_new_connection`
Rust: `rs/fq/src/lib.rs:7326-7328 log_new_connection`

### C body
```c
{
    if (cnx->quic->F_log != NULL) {
        cnx->quic->text_log_fns->log_new_connection(cnx);
    }

    if (cnx->quic->bin_log_fns != NULL) {
        cnx->quic->bin_log_fns->log_new_connection(cnx);
    }

    if (cnx->quic->qlog_fns != NULL) {
        cnx->quic->qlog_fns->log_new_connection(cnx);
    }
}
```

### Rust body
```rust
    pub fn log_new_connection(&mut self) {
        crate::logger::Log::new_connection(self);
    }
```

## Pair `picoquic/util.c:picoquic_string_duplicate`
C: `picoquic/util.c:73-84 picoquic_string_duplicate`
Rust: `rs/fq/src/performance_log.rs:349-363 perflog_setup`

### C body
```c
{
    char* str = NULL;

    if (original != NULL) {
        size_t len = strlen(original);

        str = picoquic_string_create(original, len);
    }

    return str;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let path = perflog_file_name.as_ref().to_path_buf();
        if file_is_empty(&path) {
            file_set_header(&path);
        }
        let ctx = PerflogCtx {
            items: Vec::new(),
            perflog_file_name: path,
        };
        self.perflog_fn = Some(Box::new(ctx));
        Ok(())
    }
```

## Pair `picoquic/util.c:debug_printf`
C: `picoquic/util.c:129-162 debug_printf`
Rust: `rs/fq/src/utils.rs:150-153 debug_printf`

### C body
```c
{
#if 1
    if (debug_suspended == 0 && debug_out != NULL) {
#else
    if (debug_suspended == 0 && (debug_out != NULL || debug_callback != NULL)) {
#endif
        if (debug_out) {
            va_list args;
            va_start(args, fmt);
            vfprintf(debug_out, fmt, args);
            va_end(args);
#if 1
        }
#else
        } else {
            char message[1024];
            size_t message_length;
            va_list args;
            va_start(args, fmt);
            vsnprintf(message, sizeof(message), fmt, args);
            va_end(args);
            message_length = strnlen(message, sizeof(message));
            if (message_length > 0) {
                // Strip any trailing newline
                if (message[message_length - 1] == '\n') {
                    message[message_length - 1] = '\0';
                }
            }
            debug_callback(message, debug_callback_argp);
        }
#endif
    }
}
```

### Rust body
```rust
    if DEBUG_SUSPENDED.with(|s| s.get()) {
        return;
    }
```
