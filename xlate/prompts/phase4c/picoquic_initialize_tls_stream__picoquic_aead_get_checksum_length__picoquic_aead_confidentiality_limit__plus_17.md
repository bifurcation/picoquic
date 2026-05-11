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

## Pair `picoquic/tls_api.c:picoquic_initialize_tls_stream`
C: `picoquic/tls_api.c:2243-2353 picoquic_initialize_tls_stream`
Rust: `rs/fq/src/tls_api.rs:1127-1139 initialize_tls_stream`

### C body
```c
{
    int ret = 0;
    struct st_ptls_buffer_t sendbuf;
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;
    size_t epoch_offsets[PICOQUIC_NUMBER_OF_EPOCH_OFFSETS] = { 0, 0, 0, 0, 0 };

    if (cnx->sni != NULL) {
        ptls_set_server_name(ctx->tls, cnx->sni, strlen(cnx->sni));
    }

    if (cnx->alpn != NULL) {
        ctx->alpn_vec[0].base = (uint8_t*)cnx->alpn;
        ctx->alpn_vec[0].len = strlen(cnx->alpn);
        ctx->handshake_properties.client.negotiated_protocols.count = 1;
        ctx->handshake_properties.client.negotiated_protocols.list = ctx->alpn_vec;
    }
    else if (cnx->callback_fn != NULL) {
        /* Get the default ALPN list for the callback function */
        ret = cnx->callback_fn(cnx, 0, (uint8_t*)ctx, 0, picoquic_callback_request_alpn_list, cnx->callback_ctx, NULL);

        ctx->handshake_properties.client.negotiated_protocols.count = ctx->alpn_count;
        ctx->handshake_properties.client.negotiated_protocols.list = ctx->alpn_vec;

        if (ret != 0) {
            DBG_PRINTF("ALPN list callback returns 0x%x", ret);
        }
    }

    /* ALPN is mandatory, there should be at least one */
    if (ret == 0 && ctx->handshake_properties.client.negotiated_protocols.count == 0) {
        ret = PICOQUIC_ERROR_NO_ALPN_PROVIDED;
        DBG_PRINTF("No ALPN provided, error 0x%x", ret);
    }

    picoquic_log_negotiated_alpn(cnx,
            1, (const uint8_t *)cnx->sni, (cnx->sni == NULL)?0:strlen(cnx->sni), NULL, 0,
            ctx->handshake_properties.client.negotiated_protocols.list, 
            ctx->handshake_properties.client.negotiated_protocols.count);

    /* No resumption if no alpn specified upfront, because it would make the negotiation and
     * the handling of 0-RTT way too messy */
    if (cnx->sni != NULL && cnx->alpn != NULL && !cnx->quic->client_zero_share) {
        picoquic_stored_ticket_t* stored_ticket = picoquic_get_stored_ticket(cnx->quic, 
            cnx->sni, (uint16_t)strlen(cnx->sni), cnx->alpn, (uint16_t)strlen(cnx->alpn),
            picoquic_supported_versions[cnx->version_index].version, 1, 0);
        if (stored_ticket != NULL) {
            ctx->handshake_properties.client.session_ticket.base = stored_ticket->ticket;
            ctx->handshake_properties.client.session_ticket.len = stored_ticket->ticket_length;
            ctx->handshake_properties.client.max_early_data_size = &cnx->max_early_data_size;
            /* Remember first 8 bytes of ticket as ticket ID, and set psk suite from ticket */
            cnx->resumed_ticket_id = PICOPARSE_64(stored_ticket->ticket);
            cnx->psk_cipher_suite_id = PICOPARSE_16(stored_ticket->ticket + 8);
            /* Set initial transport parameters from stored values */
            cnx->remote_parameters.initial_max_data = stored_ticket->tp_0rtt[picoquic_tp_0rtt_max_data];
            cnx->remote_parameters.initial_max_stream_data_bidi_local = stored_ticket->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_local];
            cnx->remote_parameters.initial_max_stream_data_bidi_remote = stored_ticket->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_remote];
            cnx->remote_parameters.initial_max_stream_data_uni = stored_ticket->tp_0rtt[picoquic_tp_0rtt_max_stream_data_uni];
            cnx->remote_parameters.initial_max_stream_id_bidir = stored_ticket->tp_0rtt[picoquic_tp_0rtt_max_streams_id_bidir];
            cnx->remote_parameters.initial_max_stream_id_unidir = stored_ticket->tp_0rtt[picoquic_tp_0rtt_max_streams_id_unidir];

            if (stored_ticket->time_valid_until > current_time) {
                /* Seed connection with remembered data */
                picoquic_seed_bandwidth(cnx, stored_ticket->tp_0rtt[picoquic_tp_0rtt_rtt_local],
                    stored_ticket->tp_0rtt[picoquic_tp_0rtt_cwin_local],
                    stored_ticket->ip_addr, stored_ticket->ip_addr_length);
            }
        }
    }

    if (cnx->quic->client_zero_share &&
        cnx->cnx_state == picoquic_state_client_init)
    {
        ctx->handshake_properties.client.negotiate_before_key_exchange = 1;
    }
    else
    {
        ctx->handshake_properties.client.negotiate_before_key_exchange = 0;
    }

    if (ret != 0) {
        DBG_PRINTF("Could not set up TLS parameters, error 0x%x, abandoning connection", ret);
        picoquic_connection_disconnect(cnx);
    } else {
        picoquic_tls_set_extensions(cnx, ctx);

        ptls_buffer_init(&sendbuf, "", 0);

        /* Clearing the global error state of the crypto provider before calling handle message.
         * This allows detection of errors during processing. */
        picoquic_clear_crypto_errors();
        ret = ptls_handle_message(ctx->tls, &sendbuf, epoch_offsets, 0, NULL, 0, &ctx->handshake_properties);

        /* assume that all the data goes to epoch 0, initial */
        if ((ret == 0 || ret == PTLS_ERROR_IN_PROGRESS)) {
            if (sendbuf.off > 0) {
                ret = picoquic_add_to_tls_stream(cnx, sendbuf.base, sendbuf.off, 0);
            }
            else {
                ret = 0;
            }
        }
        else {
            picoquic_log_crypto_errors(cnx, ret);
            ret = -1;
        }
        ptls_buffer_dispose(&sendbuf);
    }

    return ret;
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

## Pair `picoquic/tls_api.c:picoquic_aead_get_checksum_length`
C: `picoquic/tls_api.c:2393-2401 picoquic_aead_get_checksum_length`
Rust: `rs/fq/src/tls_api.rs:1250-1260 aead_get_checksum_length`

### C body
```c
{
    size_t tag_size = ((ptls_aead_context_t*)aead_context)->algo->tag_size;
    /* TODO: remove this temporary fix to deal with Feb 2019 change in picotls */
    if (tag_size > 16) {
        tag_size = 16;
    }
    return tag_size;
}
```

### Rust body
```rust
pub fn aead_confidentiality_limit(aead_ctx: &dyn crate::tls::PacketKey) -> u64 {
    aead_ctx.confidentiality_limit()
}
```

## Pair `picoquic/tls_api.c:picoquic_aead_confidentiality_limit`
C: `picoquic/tls_api.c:2450-2454 picoquic_aead_confidentiality_limit`
Rust: `rs/fq/src/tls_api.rs:1258-1260 aead_confidentiality_limit`

### C body
```c
{
    return ((ptls_aead_context_t*)aead_ctx)->algo->confidentiality_limit;
}
```

### Rust body
```rust
pub fn aead_confidentiality_limit(aead_ctx: &dyn crate::tls::PacketKey) -> u64 {
    aead_ctx.confidentiality_limit()
}
```

## Pair `picoquic/tls_api.c:picoquic_aead_encrypt_mp`
C: `picoquic/tls_api.c:2507-2521 picoquic_aead_encrypt_mp`
Rust: `rs/fq/src/lib.rs:5184-5199 aead_encrypt_mp`

### C body
```c
{
    size_t encrypted = 0;
    uint8_t seq32[4];

    picoformat_32(seq32, (uint32_t)path_id);
    ptls_aead_xor_iv((ptls_aead_context_t*)aead_context, seq32, sizeof(seq32));
    encrypted = ptls_aead_encrypt((ptls_aead_context_t*)aead_context,
        (void*)output, (const void*)input, input_length, seq_num,
        (void*)auth_data, auth_data_length);
    ptls_aead_xor_iv((ptls_aead_context_t*)aead_context, seq32, sizeof(seq32));

    return encrypted;
}
```

### Rust body
```rust
) -> usize {
    let mut payload = input.to_vec();
    ctx.encrypt_mp(path_id, sequence, aad, &mut payload);
    if payload.len() > out.len() {
        return 0;
    }
    out[..payload.len()].copy_from_slice(&payload);
    payload.len()
}
```

## Pair `picoquic/tls_api.c:picoquic_create_cnxid_reset_secret`
C: `picoquic/tls_api.c:2776-2802 picoquic_create_cnxid_reset_secret`
Rust: `rs/fq/src/tls_api.rs:1670-1687 create_connection_id_reset_secret`

### C body
```c
{
    int ret = 0;
    ptls_hash_algorithm_t* algo = picoquic_get_sha256();

    if (algo == NULL) {
        ret = -1;
    }
    else {
        ptls_hash_context_t* hash_ctx = algo->create();
        uint8_t final_hash[PTLS_MAX_DIGEST_SIZE];

        if (hash_ctx == NULL) {
            ret = -1;
            memset(reset_secret, 0, PICOQUIC_RESET_SECRET_SIZE);
        }
        else {
            hash_ctx->update(hash_ctx, quic->reset_seed, sizeof(quic->reset_seed));
            hash_ctx->update(hash_ctx, cnx_id, sizeof(picoquic_connection_id_t));
            hash_ctx->final(hash_ctx, final_hash, PTLS_HASH_FINAL_MODE_FREE);
            memcpy(reset_secret, final_hash, PICOQUIC_RESET_SECRET_SIZE);
        }
    }

    return (ret);
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        use digest::Digest;

        let mut serialized = [0u8; crate::CONNECTION_ID_MAX_SIZE + 1];
        serialized[..cnx_id.len()].copy_from_slice(cnx_id.as_bytes());
        serialized[crate::CONNECTION_ID_MAX_SIZE] = cnx_id.len() as u8;

        let mut hasher = sha2::Sha256::new();
        hasher.update(self.reset_seed);
        hasher.update(serialized);
        let digest = hasher.finalize();
        reset_secret.copy_from_slice(&digest[..RESET_SECRET_SIZE]);
        Ok(())
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_set_use_exporter`
C: `picoquic/tls_api.c:2823-2825 picoquic_tls_set_use_exporter`
Rust: `rs/fq/src/tls_api.rs:1710-1712 tls_set_use_exporter`

### C body
```c
void picoquic_tls_set_use_exporter(picoquic_quic_t* quic, int use_exporter) {
    ((ptls_context_t*)quic->tls_master_ctx)->use_exporter = use_exporter;
}
```

### Rust body
```rust
    pub fn tls_set_use_exporter(&mut self, use_exporter: bool) {
        self.use_exporter = use_exporter;
    }
```

## Pair `picoquic/tls_api.c:picoquic_verify_retry_token`
C: `picoquic/tls_api.c:2965-3029 picoquic_verify_retry_token`
Rust: `rs/fq/src/tls_api.rs:1879-1890 verify_retry_token`

### C body
```c
{
    int ret = 0;
    uint8_t text[128];
    size_t text_len = 0;
    picoquic_connection_id_t cid;
    uint64_t token_pn;

    odcid->id_len = 0;

    /* decode the encrypted token */
    if (token_size > sizeof(text)) {
        /* regular tokens produced by picoquic are always short, and a short decoding
        * buffer should be sufficient. If this text fires, it probably because of
        * an attack, or buggy code at the peer. */
        ret = -1;
    }
    else {
        ret = picoquic_server_decrypt_retry_token(quic, addr_peer, is_new_token, token, token_size,
            text, &text_len);
    }

    if (ret == 0) {
        /* Decode the clear text components */
        const uint8_t* bytes = text;
        const uint8_t* bytes_max = text + text_len;
        uint64_t token_time = PICOPARSE_64(text);

        if ((bytes = picoquic_frames_uint64_decode(bytes, bytes_max, &token_time)) != NULL &&
            (bytes = picoquic_frames_cid_decode(bytes, bytes_max, odcid)) != NULL &&
            (bytes = picoquic_frames_cid_decode(bytes, bytes_max, &cid)) != NULL &&
            (bytes = picoquic_frames_varint_decode(bytes, bytes_max, &token_pn)) != NULL) {
            if (token_time < current_time) {
                /* Invalid token, too old */
                ret = -1;
            }
            /* If the PN value is not yet decrypted, setting it to UINT32_MAX
             * bypasses the verification */
            else if (initial_pn != UINT32_MAX && odcid->id_len > 0 && token_pn >= initial_pn) {
                /* Invalid PN number */
                ret = -1;
            }
            else {
                /* Remove old tickets before testing this one. */
                picoquic_registered_token_clear(quic, current_time);
                if (check_reuse && (ret = picoquic_registered_token_check_reuse(quic, token, token_size, token_time)) != 0) {
                    picoquic_log_context_free_app_message(quic, rcid, "Duplicate token test returns %d", ret);
                }
                else if (odcid->id_len > 0 &&
                    picoquic_compare_connection_id(rcid, &cid) != 0) {
                    /* Invalid token, bad rcid */
                    ret = -1;
                }
            }
        }
        else {
            *odcid = picoquic_null_connection_id;
        }
    }

    return ret;
}
```

### Rust body
```rust
        if token.len() > 128 {
            return Err(Error::InvalidArgument);
        }
```

## Pair `picoquic/tls_api.c:picoquic_format_retry_protection_pseudo_packet`
C: `picoquic/tls_api.c:3173-3186 picoquic_format_retry_protection_pseudo_packet`
Rust: `rs/fq/src/tls_api.rs:865-872 retry_protection_pseudo_packet`

### C body
```c
{
    size_t pseudo_index = 0;

    if (byte_index + odcid->id_len + 1 < PICOQUIC_MAX_PACKET_SIZE) {
        pseudo_packet[pseudo_index++] = odcid->id_len;
        memcpy(&pseudo_packet[pseudo_index], odcid->id, odcid->id_len);
        pseudo_index += odcid->id_len;
        memcpy(&pseudo_packet[pseudo_index], bytes, byte_index);
        pseudo_index += byte_index;
    }

    return pseudo_index;
}
```

### Rust body
```rust
    if byte_index > bytes.len() || byte_index + odcid.len() + 1 >= MAX_PACKET_SIZE {
        return None;
    }
```

## Pair `picoquic/token_store.c:picoquic_serialize_token`
C: `picoquic/token_store.c:61-96 picoquic_serialize_token`
Rust: `rs/fq/src/internal.rs:1655-1684 serialize_token`

### C body
```c
{
    int ret = 0;
    size_t byte_index = 0;
    size_t required_length;

    /* Compute serialized length */
    required_length = (size_t)(8 + 2 + 2 + 2) + token->sni_length + token->ip_addr_length + token->token_length;
    /* Serialize */
    if (required_length > bytes_max) {
        ret = PICOQUIC_ERROR_FRAME_BUFFER_TOO_SMALL;
        *consumed = 0;
    } else {
        picoformat_64(bytes + byte_index, token->time_valid_until);
        byte_index += 8;

        picoformat_16(bytes + byte_index, token->sni_length);
        byte_index += 2;
        memcpy(bytes + byte_index, token->sni, token->sni_length);
        byte_index += token->sni_length;

        picoformat_16(bytes + byte_index, token->ip_addr_length);
        byte_index += 2;
        memcpy(bytes + byte_index, token->ip_addr, token->ip_addr_length);
        byte_index += token->ip_addr_length;

        picoformat_16(bytes + byte_index, token->token_length);
        byte_index += 2;
        memcpy(bytes + byte_index, token->token, token->token_length);
        byte_index += token->token_length;

        *consumed = byte_index;
    }

    return ret;
}
```

### Rust body
```rust
fn serialize_token(token: &StoredToken) -> Result<Vec<u8>, crate::Error> {
    let sni = optional_string_bytes(token.sni.as_deref());
    let ip_addr = stored_ip_bytes(token.ip_addr);
    if sni.len() > u16::MAX as usize
        || ip_addr.len() > u16::MAX as usize
        || token.token.len() > u16::MAX as usize
    {
        return Err(crate::Error::BufferTooSmall);
    }

    let required = 8 + 2 + sni.len() + 2 + ip_addr.len() + 2 + token.token.len();
    let mut bytes = vec![0u8; required];
    let mut off = 0;

    format_64(&mut bytes[off..off + 8], token.time_valid_until.ticks());
    off += 8;
    format_16(&mut bytes[off..off + 2], sni.len() as u16);
    off += 2;
    bytes[off..off + sni.len()].copy_from_slice(sni);
    off += sni.len();
    format_16(&mut bytes[off..off + 2], ip_addr.len() as u16);
    off += 2;
    bytes[off..off + ip_addr.len()].copy_from_slice(&ip_addr);
    off += ip_addr.len();
    format_16(&mut bytes[off..off + 2], token.token.len() as u16);
    off += 2;
    bytes[off..off + token.token.len()].copy_from_slice(&token.token);

    Ok(bytes)
}
```

## Pair `picoquic/token_store.c:picoquic_save_tokens`
C: `picoquic/token_store.c:246-280 picoquic_save_tokens`
Rust: `rs/fq/src/internal.rs:1767-1785 save_tokens`

### C body
```c
{
    int ret = 0;
    FILE* F = NULL;
    const picoquic_stored_token_t* first_token = quic->p_first_token;
    const picoquic_stored_token_t* next = first_token;
    uint64_t current_time = picoquic_get_tls_time(quic);

    if ((F = picoquic_file_open(token_file_name, "wb")) == NULL) {
        ret = -1;
    } else {
        while (ret == 0 && next != NULL) {
            /* Only store the tokens that are valid going forward */
            if (next->time_valid_until > current_time && next->was_used == 0) {
                /* Compute the serialized size */
                uint8_t buffer[2048];
                size_t record_size;

                ret = picoquic_serialize_token(next, buffer, sizeof(buffer), &record_size);

                if (ret == 0) {
                    if (fwrite(&record_size, 4, 1, F) != 1 || fwrite(buffer, 1, record_size, F) != record_size) {
                        ret = PICOQUIC_ERROR_INVALID_FILE;
                        break;
                    }
                }
            }
            next = next->next_token;
        }
        (void)picoquic_file_close(F);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let mut file = File::create(token_file_name).map_err(|_| crate::Error::InvalidFile)?;
        for token in &self.stored_tokens {
            if token.time_valid_until.ticks() > 0 && !token.was_used {
                let record = serialize_token(token)?;
                if record.len() > 2048 {
                    return Err(crate::Error::InvalidFile);
                }
                file.write_all(&(record.len() as u32).to_ne_bytes())
                    .map_err(|_| crate::Error::InvalidFile)?;
                file.write_all(&record)
                    .map_err(|_| crate::Error::InvalidFile)?;
            }
        }
        Ok(())
    }
```

## Pair `picoquic/transport.c:picoquic_transport_param_varint_encode`
C: `picoquic/transport.c:43-57 picoquic_transport_param_varint_encode`
Rust: `rs/fq/src/internal.rs:15004-15007 picoquic_transport_param_varint_encode`

### C body
```c
{
    if (bytes + 1 > bytes_max) {
        bytes = NULL;
    }
    else {
        uint8_t* byte_l = bytes++;
        bytes = picoquic_frames_varint_encode(bytes, bytes_max, n64);
        if (bytes != NULL) {
            *byte_l = (uint8_t)((bytes - byte_l) - 1);
        }
    }

    return bytes;
}
```

### Rust body
```rust
    if bytes.is_empty() {
        return None;
    }
```

## Pair `picoquic/transport.c:picoquic_transport_param_cid_decode`
C: `picoquic/transport.c:87-96 picoquic_transport_param_cid_decode`
Rust: `rs/fq/src/internal.rs:15091-15108 picoquic_transport_param_cid_decode`

### C body
```c
{
    int ret = 0;
    cid->id_len = (uint8_t)picoquic_parse_connection_id(bytes, (uint8_t)extension_length, cid);
    if ((size_t)cid->id_len != extension_length) {
        ret = picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0);
    }

    return ret;
}
```

### Rust body
```rust
) -> i32 {
    let len = usize::try_from(extension_length).unwrap_or(usize::MAX);
    let Some(value) = bytes.get(..len) else {
        return cnx.connection_error(crate::errors::TransportError::ParameterError as u64, 0);
    };
    match ConnectionId::clone_from_slice(value) {
        Some(parsed) if parsed.len() == len => {
            *cid = parsed;
            0
        }
        _ => cnx.connection_error(crate::errors::TransportError::ParameterError as u64, 0),
    }
}
```

## Pair `picoquic/transport.c:picoquic_process_tp_version_negotiation`
C: `picoquic/transport.c:226-281 picoquic_process_tp_version_negotiation`
Rust: `rs/fq/src/internal.rs:15270-15306 process_tp_version_negotiation`

### C body
```c
{
    uint32_t current;

    *negotiated_vn = 0;
    *negotiated_index = -1;
    *vn_error = 0;

    if ((bytes = picoquic_frames_uint32_decode(bytes, bytes_max, &current)) == NULL) {
        *vn_error = PICOQUIC_TRANSPORT_PARAMETER_ERROR;
    } else {
        if (current != envelop_vn) {
            /* Packet was tempered with */
            *vn_error = PICOQUIC_TRANSPORT_VERSION_NEGOTIATION_ERROR;
            bytes = NULL;
        }
        else if (extension_mode == 0) {
            /* Processing the client extensions */
            while (bytes < bytes_max) {
                uint32_t proposed;
                if ((bytes = picoquic_frames_uint32_decode(bytes, bytes_max, &proposed)) == NULL) {
                    /* Decoding error */
                    *vn_error = PICOQUIC_TRANSPORT_PARAMETER_ERROR;
                    break;
                }
                else {
                    /* Select the first version proposed by the client that is locally supported,
                     * and is deemed compatible with the current version
                     */
                    int this_rank = picoquic_get_version_index(proposed);
                    if (this_rank >= 0) {
                        *negotiated_vn = proposed;
                        *negotiated_index = this_rank;
                        break;
                    }
                }
            }
        }
        else {
            /* Processing the server extensions */
            /* TODO: Check whether the chosen version corresponds to something the client wanted */
            /* TODO: Check whether the chosen version is officially supported, could be reused in 0-RTT */
            while (bytes < bytes_max) {
                uint32_t proposed;
                if ((bytes = picoquic_frames_uint32_decode(bytes, bytes_max, &proposed)) == NULL) {
                    /* Decoding error */
                    *vn_error = PICOQUIC_TRANSPORT_PARAMETER_ERROR;
                    break;
                }
            }
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a [u8]> {
    *negotiated_vn = 0;
    *negotiated_index = -1;
    *vn_error = 0;
    if bytes.len() < 4 || !bytes.len().is_multiple_of(4) {
        *vn_error = 0x7;
        return None;
    }
    let current = u32::from_be_bytes(bytes[0..4].try_into().ok()?);
    if current != envelop_vn {
        *vn_error = 0xA;
        return None;
    }
    if extension_mode == 0 {
        *negotiated_vn = current;
        *negotiated_index = Version::try_from_wire(current).map(|_| 0).unwrap_or(-1);
        return Some(&bytes[bytes.len()..]);
    }
    for (idx, chunk) in bytes[4..].chunks_exact(4).enumerate() {
        let candidate = u32::from_be_bytes(chunk.try_into().ok()?);
        if Version::try_from_wire(candidate).is_some() {
            *negotiated_vn = candidate;
            *negotiated_index = idx as i32;
            return Some(&bytes[bytes.len()..]);
        }
    }
    *negotiated_vn = current;
    *negotiated_index = Version::try_from_wire(current).map(|_| 0).unwrap_or(-1);
    Some(&bytes[bytes.len()..])
}
```

## Pair `picoquic/transport.c:picoquic_receive_transport_extensions`
C: `picoquic/transport.c:534-1024 picoquic_receive_transport_extensions`
Rust: `rs/fq/src/internal.rs:15502-15653 receive_transport_extensions`

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

## Pair `picoquic/unified_log.c:picoquic_log_app_message`
C: `picoquic/unified_log.c:75-98 picoquic_log_app_message`
Rust: `rs/fq/src/lib.rs:7332-7334 log_app_message`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    if (cnx->quic->F_log != NULL) {
        va_list args;
        va_start(args, fmt);
        cnx->quic->text_log_fns->log_app_message(cnx, fmt, args);
        va_end(args);
    }

    if (cnx->f_binlog != NULL) {
        va_list args;
        va_start(args, fmt);
        cnx->quic->bin_log_fns->log_app_message(cnx, fmt, args);
        va_end(args);
    }

    if (cnx->qlog_ctx != NULL) {
        va_list args;
        va_start(args, fmt);
        cnx->quic->qlog_fns->log_app_message(cnx, fmt, args);
        va_end(args);
    }
}
```

### Rust body
```rust
    pub fn log_app_message(&mut self, msg: &str) {
        crate::logger::Log::app_message(self, format_args!("{}", msg));
    }
```

## Pair `picoquic/unified_log.c:picoquic_log_dropped_packet`
C: `picoquic/unified_log.c:152-169 picoquic_log_dropped_packet`
Rust: `rs/fq/src/logger.rs:354-884 dropped_packet`

### C body
```c
{
    if (picoquic_cnx_is_still_logging(cnx)) {
        if (cnx->quic->F_log != NULL) {
            cnx->quic->text_log_fns->log_dropped_packet(cnx, path_x, ph, packet_size, err, current_time);
        }

        if (cnx->f_binlog != NULL) {
            cnx->quic->bin_log_fns->log_dropped_packet(cnx, path_x, ph, packet_size, err, current_time);
        }

        if (cnx->qlog_ctx != NULL) {
            cnx->quic->qlog_fns->log_dropped_packet(cnx, path_x, ph, packet_size, err, current_time);
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

## Pair `picoquic/unified_log.c:picoquic_log_negotiated_alpn`
C: `picoquic/unified_log.c:233-249 picoquic_log_negotiated_alpn`
Rust: `rs/fq/src/logger.rs:404-884 negotiated_alpn`

### C body
```c
{
    if (cnx->quic->F_log != NULL) {
        cnx->quic->text_log_fns->log_negotiated_alpn(cnx, is_local, sni, sni_len, alpn, alpn_len, alpn_list, alpn_count);
    }

    if (cnx->f_binlog != NULL) {
        cnx->quic->bin_log_fns->log_negotiated_alpn(cnx, is_local, sni, sni_len, alpn, alpn_len, alpn_list, alpn_count);
    }

    if (cnx->qlog_ctx != NULL) {
        cnx->quic->qlog_fns->log_negotiated_alpn(cnx, is_local, sni, sni_len, alpn, alpn_len, alpn_list, alpn_count);
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

## Pair `picoquic/unified_log.c:picoquic_log_close_connection`
C: `picoquic/unified_log.c:299-313 picoquic_log_close_connection`
Rust: `rs/fq/src/logger.rs:426-884 close_connection`

### C body
```c
{
    if (cnx->quic->F_log != NULL) {
        cnx->quic->text_log_fns->log_close_connection(cnx);
    }

    if (cnx->f_binlog != NULL) {
        cnx->quic->bin_log_fns->log_close_connection(cnx);
    }

    if (cnx->qlog_ctx != NULL) {
        cnx->quic->qlog_fns->log_close_connection(cnx);
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

## Pair `picoquic/util.c:debug_set_stream`
C: `picoquic/util.c:102-109 debug_set_stream`
Rust: `rs/fq/src/utils.rs:126-128 debug_set_stream`

### C body
```c
{
    debug_out = F;
#if 0
    debug_callback = NULL;
    debug_callback_argp = NULL;
#endif
}
```

### Rust body
```rust
pub fn debug_set_stream(stream: Option<Box<dyn core::fmt::Write>>) {
    DEBUG_OUT.with(|o| *o.borrow_mut() = stream);
}
```

## Pair `picoquic/util.c:debug_printf_push_stream`
C: `picoquic/util.c:198-205 debug_printf_push_stream`
Rust: `rs/fq/src/utils.rs:165-174 debug_printf_push_stream`

### C body
```c
{
    if (debug_out) {
        fprintf(stderr, "Nested err out not supported\n");
        exit(1);
    }
    debug_out = f;
}
```

### Rust body
```rust
pub fn debug_printf_push_stream(stream: Box<dyn core::fmt::Write>) {
    DEBUG_OUT.with(|o| {
        let mut guard = o.borrow_mut();
        assert!(
            guard.is_none(),
            "nested debug_printf_push_stream not supported"
        );
        *guard = Some(stream);
    });
}
```
