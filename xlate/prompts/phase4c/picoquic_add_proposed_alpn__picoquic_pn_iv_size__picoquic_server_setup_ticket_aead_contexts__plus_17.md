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

## Pair `picoquic/tls_api.c:picoquic_add_proposed_alpn`
C: `picoquic/tls_api.c:2206-2223 picoquic_add_proposed_alpn`
Rust: `rs/fq/src/tls_api.rs:2771-2777 add_proposed_alpn`

### C body
```c
{
    int ret = 0;
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)tls_context;
    if (ctx == NULL) {
        ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
    }
    else if (ctx->alpn_count >= ctx->alpn_vec_size) {
        ret = PICOQUIC_ERROR_SEND_BUFFER_TOO_SMALL;
    } else {
        ctx->alpn_vec[ctx->alpn_count].base = (uint8_t*)alpn;
        ctx->alpn_vec[ctx->alpn_count].len = strlen(alpn);
        ctx->alpn_count++;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn add_proposed_alpn(&mut self, alpn: &str) -> Result<(), Error> {
        if self.alpn_proposals.len() >= crate::internal::ALPN_NUMBER_MAX {
            return Err(Error::BufferTooSmall);
        }
        self.alpn_proposals.push(alpn.to_owned());
        Ok(())
```

## Pair `picoquic/tls_api.c:picoquic_pn_iv_size`
C: `picoquic/tls_api.c:2369-2372 picoquic_pn_iv_size`
Rust: `rs/fq/src/tls_api.rs:2144-2146 pn_iv_size`

### C body
```c
{
    return ((ptls_cipher_context_t *)pn_enc)->algo->iv_size;
}
```

### Rust body
```rust
pub fn pn_iv_size(_pn_enc: &dyn crate::tls::HeaderKey) -> usize {
    16
}
```

## Pair `picoquic/tls_api.c:picoquic_server_setup_ticket_aead_contexts`
C: `picoquic/tls_api.c:2414-2442 picoquic_server_setup_ticket_aead_contexts`
Rust: `rs/fq/src/tls_api.rs:958-963 picoquic_server_setup_ticket_aead_contexts`

### C body
```c
{
    int ret = 0;
    uint8_t temp_secret[256]; /* secret_max */
    ptls_cipher_suite_t *cipher = picoquic_get_aes128gcm_sha256(0);

    if (cipher->hash->digest_size > sizeof(temp_secret)) {
        ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
    } else {
        if (secret != NULL && secret_length > 0) {
            memset(temp_secret, 0, cipher->hash->digest_size);
            memcpy(temp_secret, secret, (secret_length > cipher->hash->digest_size) ? cipher->hash->digest_size : secret_length);
        } else {
            tls_ctx->random_bytes(temp_secret, cipher->hash->digest_size);
        }

        /* Create the AEAD contexts */
        ret = picoquic_set_aead_from_secret(&quic->aead_encrypt_ticket_ctx, cipher, 1, temp_secret, "random label");
        if (ret == 0) {
            ret = picoquic_set_aead_from_secret(&quic->aead_decrypt_ticket_ctx, cipher, 0, temp_secret, "random label");
        }

        /* erase the temporary secret */
        ptls_clear_memory(temp_secret, cipher->hash->digest_size);
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        install_ticket_aead_contexts(self, secret)
    }
```

## Pair `picoquic/tls_api.c:picoquic_aead_encrypt_generic`
C: `picoquic/tls_api.c:2473-2483 picoquic_aead_encrypt_generic`
Rust: `rs/fq/src/tls_api.rs:1292-1305 aead_encrypt_generic`

### C body
```c
{
    size_t encrypted = 0;

    encrypted = ptls_aead_encrypt((ptls_aead_context_t*)aead_context,
        (void*)output, (const void*)input, input_length, seq_num,
        (void*)auth_data, auth_data_length);

    return encrypted;
}
```

### Rust body
```rust
) -> usize {
    let mut buf = input.to_vec();
    aead_ctx.encrypt(seq_num, auth_data, &mut buf);
    let len = buf.len();
    let copy_len = len.min(output.len());
    output[..copy_len].copy_from_slice(&buf[..copy_len]);
    len
}
```

## Pair `picoquic/tls_api.c:picoquic_tls_stream_process`
C: `picoquic/tls_api.c:2551-2758 picoquic_tls_stream_process`
Rust: `rs/fq/src/tls_api.rs:1056-1115 process_tls_stream`

### C body
```c
{
    int ret = 0;
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;
    size_t next_epoch = 0;

    /* Provide indication of current connection for later callbacks */
    cnx->quic->cnx_in_progress = cnx;

    for (size_t epoch = 0; epoch < PICOQUIC_NUMBER_OF_EPOCHS && ret == 0; epoch++) {
        picoquic_stream_head_t* stream = &cnx->tls_stream[epoch];
        picoquic_stream_data_node_t* data = (picoquic_stream_data_node_t*)picosplay_first(&stream->stream_data_tree);
        size_t processed = 0;
        int data_pushed = 0;

        next_epoch = ptls_get_read_epoch(ctx->tls);

        if (epoch != next_epoch) {
            if (epoch > next_epoch) {
                break;
            } else {
                if (data != NULL && data->offset > stream->consumed_offset) {
                    /* Protocol error: data received that could not be read */
#ifdef _DEBUG
                    DBG_PRINTF("Connection error - TLS data at epoch %d, expected %d.\n",
                        epoch, next_epoch);
#endif
                    ret = picoquic_connection_error(cnx,
                        PICOQUIC_TRANSPORT_PROTOCOL_VIOLATION, 0);
                }
                continue;
            }
        }

        while ((ret == 0 || ret == PTLS_ERROR_IN_PROGRESS) &&
            data != NULL && data->offset <= stream->consumed_offset) {
            struct st_ptls_buffer_t sendbuf;
            size_t start = (size_t)(stream->consumed_offset - data->offset);
            size_t epoch_data = data->length - start;
            size_t send_offset[PICOQUIC_NUMBER_OF_EPOCH_OFFSETS] = { 0, 0, 0, 0, 0 };

            if (data_consumed != NULL) {
                *data_consumed = 1;
            }

            ptls_buffer_init(&sendbuf, "", 0);

            /* Clearing the global error state of the crypto provider before calling handle message.
             * This allows detection of errors during processing. */
            picoquic_clear_crypto_errors();

            ret = ptls_handle_message(ctx->tls, &sendbuf, send_offset, epoch,
                data->bytes + start, epoch_data, &ctx->handshake_properties);

            if ((ret == 0 || ret == PTLS_ERROR_IN_PROGRESS ||
                ret == PTLS_ERROR_STATELESS_RETRY)) {
                for (int i = 0; i < PICOQUIC_NUMBER_OF_EPOCHS; i++) {
                    if (send_offset[i] < send_offset[i + 1]) {
                        data_pushed = 1;
                        ret = picoquic_add_to_tls_stream(cnx,
                            sendbuf.base + send_offset[i], send_offset[i + 1] - send_offset[i], i);
                    }
                }
                if (cnx->client_mode) {
                    if (cnx->alpn == NULL) {
                        const char* alpn = ptls_get_negotiated_protocol(ctx->tls);

                        if (alpn != NULL){
                            cnx->alpn = picoquic_string_duplicate(alpn);

                            picoquic_log_negotiated_alpn(cnx, 0, NULL, 0, (const uint8_t*)alpn, strlen(alpn), NULL, 0);

                            if (cnx->callback_fn != NULL) {
                                cnx->callback_fn(cnx, 0, (uint8_t*)alpn, 0, picoquic_callback_set_alpn, cnx->callback_ctx, NULL);
                            }
                            else {
                                DBG_PRINTF("Negotiated ALPN: %s", alpn);
                            }
                        }
                    }
                    switch (ctx->handshake_properties.client.early_data_acceptance) {
                    case PTLS_EARLY_DATA_REJECTED:
                        cnx->zero_rtt_data_accepted = 0;
                        break;
                    case PTLS_EARLY_DATA_ACCEPTED:
                        cnx->zero_rtt_data_accepted = 1;
                        break;
                    default:
                        break;
                    }
                }
            }
            else {
                picoquic_log_crypto_errors(cnx, ret);
            }

            stream->consumed_offset += epoch_data;
            processed += epoch_data;

            if (start + epoch_data >= data->length) {
                picosplay_delete_hint(&cnx->tls_stream[epoch].stream_data_tree, &data->stream_data_node);
                data = (picoquic_stream_data_node_t*)picosplay_first(&cnx->tls_stream[epoch].stream_data_tree);
            }

            ptls_buffer_dispose(&sendbuf);
        }

        if (processed > 0) {
            if (ret == 0) {
                switch (cnx->cnx_state) {
                case picoquic_state_client_retry_received:
                    /* This is not supposed to happen -- HRR should generate "error in progress" */
                    break;
                case picoquic_state_client_init:
                case picoquic_state_client_init_sent:
                case picoquic_state_client_renegotiate:
                case picoquic_state_client_init_resent:
                case picoquic_state_client_handshake_start:
                    if (ptls_handshake_is_complete(ctx->tls)) {
                        if (cnx->remote_parameters_received == 0) {

#ifdef _DEBUG
                            DBG_PRINTF("%s", "Connection error - no transport parameter received.\n");
#endif
                            ret = picoquic_connection_error(cnx,
                                PICOQUIC_TRANSPORT_PARAMETER_ERROR, 0);
                        }
                        else {
                            if (cnx->crypto_context[3].aead_encrypt != NULL) {
                                picoquic_client_almost_ready_transition(cnx);
                            }
                        }
                    }
                    break;
                case picoquic_state_server_init:
                case picoquic_state_server_handshake:
                    /* If client authentication is activated, the client sends the certificates with its `Finished` packet.
                       The server does not send any further packets, so, we can switch into false start state here.
                    */
                    if (data_pushed == 0 && ((ptls_context_t*)cnx->quic->tls_master_ctx)->require_client_authentication == 1) {
                        picoquic_false_start_transition(cnx, current_time);
                    }
                    else {
                        if (cnx->crypto_context[3].aead_encrypt != NULL) {
                            cnx->cnx_state = picoquic_state_server_almost_ready;
                        }
                    }
                    break;
                case picoquic_state_client_almost_ready:
                case picoquic_state_handshake_failure:
                case picoquic_state_handshake_failure_resend:
                case picoquic_state_client_ready_start:
                case picoquic_state_server_almost_ready:
                case picoquic_state_server_false_start:
                case picoquic_state_ready:
                case picoquic_state_disconnecting:
                case picoquic_state_closing_received:
                case picoquic_state_closing:
                case picoquic_state_draining:
                case picoquic_state_disconnected:
                    break;
                default:
                    DBG_PRINTF("Unexpected connection state: %d\n", cnx->cnx_state);
                    break;
                }
            }
            else if (ret == PTLS_ERROR_IN_PROGRESS && (cnx->cnx_state == picoquic_state_client_init || cnx->cnx_state == picoquic_state_client_init_sent || cnx->cnx_state == picoquic_state_client_init_resent)) {
                /* Extract and install the client 0-RTT key */
#ifdef _DEBUG
                DBG_PRINTF("%s", "Handshake not yet complete.\n");
#endif
            }
            else if (ret == PTLS_ERROR_IN_PROGRESS &&
                (cnx->cnx_state == picoquic_state_server_init ||
                    cnx->cnx_state == picoquic_state_server_handshake))
            {
                if (ptls_handshake_is_complete(ctx->tls))
                {
                    cnx->cnx_state = picoquic_state_server_almost_ready;
                }
            }

            if ((ret == 0 || ret == PTLS_ERROR_IN_PROGRESS || ret == PTLS_ERROR_STATELESS_RETRY)) {
                ret = 0;
            }
            else {
                uint16_t error_code = PICOQUIC_TRANSPORT_INTERNAL_ERROR;

                if (PTLS_ERROR_GET_CLASS(ret) == PTLS_ERROR_CLASS_SELF_ALERT) {
                    error_code = PICOQUIC_TRANSPORT_CRYPTO_ERROR(ret);
                }
#ifdef _DEBUG
                DBG_PRINTF("Handshake failed, ret = 0x%x.\n", ret);
#endif
                (void)picoquic_connection_error(cnx, error_code, 0);
                /* Log the version numbers of SSL/TLS packages to facilitate debugging. */
                picoquic_tls_api_log_versions(cnx);
                ret = 0;
            }
        }
    }

    /* Reset indication of current connection */
    cnx->quic->cnx_in_progress = NULL;


    return ret;
}
```

### Rust body
```rust
    pub fn process_tls_stream(&mut self, current_time: Instant) -> Result<usize, Error> {
        let mut session = self.tls_ctx.take().ok_or(Error::InvalidState)?;
        let mut chunks = Vec::new();
        let mut consumed = 0usize;

        for epoch in 0..NUMBER_OF_EPOCHS {
            let stream = &mut self.tls_stream[epoch];
            while let Some(node_token) = stream.stream_data_tree.first() {
                let data_token = match stream.stream_data_tree.get(node_token).copied() {
                    Some(token) => token,
                    None => {
                        stream.stream_data_tree.remove(node_token);
                        continue;
                    }
                };
                let Some(node) = stream.stream_data_nodes.get(data_token) else {
                    stream.stream_data_tree.remove(node_token);
                    continue;
                };
                if node.offset > stream.consumed_offset {
                    break;
                }
                let start = (stream.consumed_offset - node.offset) as usize;
                if start >= node.length {
                    stream.stream_data_tree.remove(node_token);
                    stream.stream_data_nodes.remove(data_token);
                    continue;
                }
                let data = node.data[start..node.length].to_vec();
                stream.consumed_offset += data.len() as u64;
                consumed += data.len();
                chunks.push(data);
                stream.stream_data_tree.remove(node_token);
                stream.stream_data_nodes.remove(data_token);
            }
        }

        for chunk in &chunks {
            if session.read_handshake(chunk)?
                && let Some(keys) = session.write_handshake(&mut self.tls_sendbuf)
            {
                install_key_pair(&mut self.crypto_context[3], keys);
            }
        }

        if let Some(keys) = session.write_handshake(&mut self.tls_sendbuf) {
            install_key_pair(&mut self.crypto_context[3], keys);
        }

        if !self.tls_sendbuf.is_empty() {
            let out = core::mem::take(&mut self.tls_sendbuf);
            queue_tls_bytes(self, 0, &out)?;
        }

        if !session.is_handshaking() {
            self.ready_state_transition(current_time);
        }
        self.tls_ctx = Some(session);
        Ok(consumed)
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_set_client_authentication`
C: `picoquic/tls_api.c:2815-2817 picoquic_tls_set_client_authentication`
Rust: `rs/fq/src/tls_api.rs:1698-1706 tls_set_client_authentication`

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

## Pair `picoquic/tls_api.c:picoquic_server_decrypt_retry_token`
C: `picoquic/tls_api.c:2888-2921 picoquic_server_decrypt_retry_token`
Rust: `rs/fq/src/tls_api.rs:1782-1809 server_decrypt_retry_token`

### C body
```c
{
    int ret = 0;
    uint64_t sequence;
    uint8_t* auth_data;
    size_t auth_data_length;

    if (addr_peer->sa_family == AF_INET) {
        auth_data = (uint8_t*)&((struct sockaddr_in *)addr_peer)->sin_addr;
        auth_data_length = 4;
    }
    else {
        auth_data = (uint8_t*)&((struct sockaddr_in6 *)addr_peer)->sin6_addr;
        auth_data_length = 16;
    }

    if (token_length < 8) {
        *is_new_token = 0;
        ret = -1;
    }
    else {
        *is_new_token = ((token[0] & 0x80) == 0) ? 0: 1;
        sequence = PICOPARSE_64(token);

        *text_length = picoquic_aead_decrypt_generic(text, token+8, token_length-8,
            sequence, auth_data, auth_data_length, quic->aead_decrypt_ticket_ctx);
        if (*text_length >= token_length - 8) {
            ret = -1;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<DecryptedRetryToken, Error> {
        ensure_ticket_aead_contexts(self)?;
        if token.len() < 8 {
            return Err(Error::InvalidArgument);
        }
        let is_new_token = (token[0] & 0x80) != 0;
        let sequence = u64::from_be_bytes(token[..8].try_into().unwrap());
        let aad = ip_auth_data(addr_peer);
        let mut payload = token[8..].to_vec();
        let aead = self
            .aead_decrypt_ticket_ctx
            .as_ref()
            .ok_or(Error::InvalidState)?;
        aead.decrypt(sequence, &aad, &mut payload)?;
        if payload.len() > text.len() {
            return Err(Error::BufferTooSmall);
        }
        text[..payload.len()].copy_from_slice(&payload);
        Ok(DecryptedRetryToken {
            is_new_token,
            text_length: payload.len(),
        })
    }
```

## Pair `picoquic/tls_api.c:picoquic_find_retry_protection_context`
C: `picoquic/tls_api.c:3121-3152 picoquic_find_retry_protection_context`
Rust: `rs/fq/src/tls_api.rs:2047-2077 find_retry_protection_context`

### C body
```c
{
    void * aead_ctx = NULL;
    void ** aead_vector = (sending) ? quic->retry_integrity_sign_ctx : quic->retry_integrity_verify_ctx;

    if (picoquic_supported_versions[version_index].version_retry_key != NULL) {
        if (aead_vector == NULL) {
            if (sending) {
                quic->retry_integrity_sign_ctx = (void**)malloc(sizeof(void*)*picoquic_nb_supported_versions);
                aead_vector = quic->retry_integrity_sign_ctx;
            }
            else {
                quic->retry_integrity_verify_ctx = (void**)malloc(sizeof(void*)*picoquic_nb_supported_versions);
                aead_vector = quic->retry_integrity_verify_ctx;
            }
            if (aead_vector != NULL) {
                memset(aead_vector, 0, sizeof(void*)*picoquic_nb_supported_versions);
            }
        }

        if (aead_vector != NULL) {
            aead_ctx = aead_vector[version_index];
            if (aead_ctx == NULL) {
                aead_ctx = picoquic_create_retry_protection_context(sending, picoquic_supported_versions[version_index].version_retry_key,
                                                                    picoquic_supported_versions[version_index].tls_prefix_label);
                aead_vector[version_index] = aead_ctx;
            }
        }
    }

    return aead_ctx;
}
```

### Rust body
```rust
    ) -> Option<&mut (dyn crate::tls::PacketKey + 'static)> {
        let version = version_from_index(version_index)?;
        let params = version.parameters();
        let vec = if sending {
            &mut self.retry_integrity_sign_ctx
        } else {
            &mut self.retry_integrity_verify_ctx
        };
        let idx = usize::try_from(version_index).ok()?;
        while vec.len() <= idx {
            let version = version_from_index(vec.len() as i32)?;
            let params = version.parameters();
            vec.push(create_retry_protection_context(
                sending,
                params.version_retry_key,
                params.tls_prefix_label,
            )?);
        }
        if vec.get(idx).is_none() {
            vec.push(create_retry_protection_context(
                sending,
                params.version_retry_key,
                params.tls_prefix_label,
            )?);
        }
        vec.get_mut(idx).map(|b| b.as_mut())
    }
```

## Pair `picoquic/tls_api.c:picoquic_verify_retry_protection`
C: `picoquic/tls_api.c:3201-3218 picoquic_verify_retry_protection`
Rust: `rs/fq/src/tls_api.rs:2114-2135 verify_retry_protection`

### C body
```c
{
    int ret = PICOQUIC_ERROR_AEAD_CHECK;
    size_t pseudo_index;
    uint8_t pseudo_packet[PICOQUIC_MAX_PACKET_SIZE];
    uint8_t decoded[PICOQUIC_MAX_PACKET_SIZE];
    size_t checksum_length = picoquic_aead_get_checksum_length(integrity_aead);

    if (byte_index + checksum_length < *length) {
        *length -= checksum_length;
        if ((pseudo_index = picoquic_format_retry_protection_pseudo_packet(pseudo_packet, bytes, *length, odcid)) > 0 &&
            picoquic_aead_decrypt_generic(decoded, bytes + *length, checksum_length, 0, pseudo_packet, pseudo_index, integrity_aead) == 0) {
            ret = 0;
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<usize, Error> {
    let tag_len = integrity_aead.tag_len();
    if length > bytes.len() || length < tag_len || byte_index + tag_len >= length {
        return Err(Error::Protocol(InternalError::AeadCheck as u64));
    }
    let payload_len = length - tag_len;
    let pseudo_packet = retry_protection_pseudo_packet(bytes, payload_len, odcid)
        .ok_or(Error::Protocol(InternalError::AeadCheck as u64))?;
    let mut tag = bytes[payload_len..length].to_vec();
    integrity_aead.decrypt(0, &pseudo_packet, &mut tag)?;
    if tag.is_empty() {
        Ok(payload_len)
    } else {
        Err(Error::Protocol(InternalError::AeadCheck as u64))
    }
}
```

## Pair `picoquic/token_store.c:picoquic_store_token`
C: `picoquic/token_store.c:157-202 picoquic_store_token`
Rust: `rs/fq/src/internal.rs:1714-1737 store_token`

### C body
```c
{
    int ret = 0;
    picoquic_stored_token_t** pp_first_token = &quic->p_first_token;
    uint64_t current_time = picoquic_get_tls_time(quic);

    if (token_length < 1 || sni == NULL || sni_length == 0) {
        ret = PICOQUIC_ERROR_INVALID_TOKEN;
    }
    else {
        /* There is no explicit TTL for tokens. We assume they are OK for 24 hours */
        uint64_t time_valid_until = current_time + ((uint64_t)24 * 3600) * ((uint64_t)1000000);
        picoquic_stored_token_t* stored = picoquic_format_token(time_valid_until, sni, sni_length,
            ip_addr, ip_addr_length, token, token_length);
        if (stored == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
        else {
            picoquic_stored_token_t* next;
            picoquic_stored_token_t** pprevious;

            stored->next_token = next = *pp_first_token;
            *pp_first_token = stored;
            pprevious = &stored->next_token;

            /* Now remove the old tokens for that SNI & ip_addr */
            while (next != NULL) {
                if (next->time_valid_until <= stored->time_valid_until && next->sni_length == sni_length && next->ip_addr_length == ip_addr_length && memcmp(next->sni, sni, sni_length) == 0 && memcmp(next->ip_addr, ip_addr, ip_addr_length) == 0) {
                    picoquic_stored_token_t* deleted = next;
                    next = next->next_token;
                    *pprevious = next;
                    free(deleted);
                }
                else {
                    pprevious = &next->next_token;
                    next = next->next_token;
                }
            }
        }
    } 

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        if sni.unwrap_or("").is_empty() || token.is_empty() {
            return Err(crate::Error::Protocol(
                crate::errors::InternalError::InvalidToken as u64,
            ));
        }

        // Replace any existing token for the same (sni, ip_addr).
        self.stored_tokens
            .retain(|t| t.sni.as_deref() != sni || t.ip_addr != ip_addr);
        self.stored_tokens.push(StoredToken {
            sni: sni.map(str::to_owned),
            token: token.to_vec(),
            ip_addr,
            time_valid_until: crate::Instant::from_ticks(TOKEN_DELAY_LONG.ticks()),
            was_used: false,
        });
        Ok(())
    }
```

## Pair `picoquic/tp_names.c:picoquic_tp_name`
C: `picoquic/tp_names.c:3-97 picoquic_tp_name`
Rust: `rs/fq/src/tp.rs:59-91 name`

### C body
```c
{
    char const* tp_name = "unknown";

    switch (tp_number) {
    case picoquic_tp_original_connection_id:
        tp_name = "original_connection_id";
        break;
    case picoquic_tp_idle_timeout:
        tp_name = "idle_timeout";
        break;
    case picoquic_tp_stateless_reset_token:
        tp_name = "stateless_reset_token";
        break;
    case picoquic_tp_max_packet_size:
        tp_name = "max_packet_size";
        break;
    case picoquic_tp_initial_max_data:
        tp_name = "initial_max_data";
        break;
    case picoquic_tp_initial_max_stream_data_bidi_local:
        tp_name = "initial_max_stream_data_bidi_local";
        break;
    case picoquic_tp_initial_max_stream_data_bidi_remote:
        tp_name = "initial_max_stream_data_bidi_remote";
        break;
    case picoquic_tp_initial_max_stream_data_uni:
        tp_name = "initial_max_stream_data_uni";
        break;
    case picoquic_tp_initial_max_streams_bidi:
        tp_name = "initial_max_streams_bidi";
        break;
    case picoquic_tp_initial_max_streams_uni:
        tp_name = "initial_max_streams_uni";
        break;
    case picoquic_tp_ack_delay_exponent:
        tp_name = "ack_delay_exponent";
        break;
    case picoquic_tp_max_ack_delay:
        tp_name = "max_ack_delay";
        break;
    case picoquic_tp_disable_migration:
        tp_name = "disable_migration";
        break;
    case picoquic_tp_server_preferred_address:
        tp_name = "server_preferred_address";
        break;
    case picoquic_tp_active_connection_id_limit:
        tp_name = "active_connection_id_limit";
        break;
    case picoquic_tp_retry_connection_id:
        tp_name = "retry_connection_id";
        break;
    case picoquic_tp_handshake_connection_id:
        tp_name = "handshake_connection_id";
        break;
    case picoquic_tp_max_datagram_frame_size:
        tp_name = "max_datagram_frame_size";
        break;
    case picoquic_tp_test_large_chello:
        tp_name = "large_chello";
        break;
    case picoquic_tp_enable_loss_bit:
        tp_name = "enable_loss_bit";
        break;
    case picoquic_tp_min_ack_delay:
        tp_name = "min_ack_delay";
        break;
    case picoquic_tp_enable_time_stamp:
        tp_name = "enable_time_stamp";
        break;
    case picoquic_tp_grease_quic_bit:
        tp_name = "grease_quic_bit";
        break;
    case picoquic_tp_version_negotiation:
        tp_name = "version_negotiation";
        break;
    case picoquic_tp_enable_bdp_frame:
        tp_name = "enable_bdp_frame";
        break;
    case picoquic_tp_initial_max_path_id:
        tp_name = "initial_max_path_id";
        break;
    case picoquic_tp_address_discovery:
        tp_name = "address_discovery";
        break;
    case picoquic_tp_reset_stream_at:
        tp_name = "reset_stream_at";
        break;
    default:
        break;
    }

    return tp_name;
}
```

### Rust body
```rust
    pub fn name(tp_number: u64) -> Option<&'static str> {
        match tp_number {
            0 => Some("original_connection_id"),
            1 => Some("idle_timeout"),
            2 => Some("stateless_reset_token"),
            3 => Some("max_packet_size"),
            4 => Some("initial_max_data"),
            5 => Some("initial_max_stream_data_bidi_local"),
            6 => Some("initial_max_stream_data_bidi_remote"),
            7 => Some("initial_max_stream_data_uni"),
            8 => Some("initial_max_streams_bidi"),
            9 => Some("initial_max_streams_uni"),
            10 => Some("ack_delay_exponent"),
            11 => Some("max_ack_delay"),
            12 => Some("disable_migration"),
            13 => Some("server_preferred_address"),
            14 => Some("active_connection_id_limit"),
            15 => Some("handshake_connection_id"),
            16 => Some("retry_connection_id"),
            0x11 => Some("version_negotiation"),
            32 => Some("max_datagram_frame_size"),
            3127 => Some("large_chello"),
            0x1057 => Some("enable_loss_bit"),
            0x7158 => Some("enable_time_stamp"),
            0x2ab2 => Some("grease_quic_bit"),
            0xebd9 => Some("enable_bdp_frame"),
            0x3e => Some("initial_max_path_id"),
            0xff04de1b => Some("min_ack_delay"),
            0x9f81a176 => Some("address_discovery"),
            0x17f7586d2cb571 => Some("reset_stream_at"),
            _ => None,
        }
    }
```

## Pair `picoquic/transport.c:picoquic_transport_param_type_flag_encode`
C: `picoquic/transport.c:68-75 picoquic_transport_param_type_flag_encode`
Rust: `rs/fq/src/internal.rs:15055-15064 picoquic_transport_param_type_flag_encode`

### C body
```c
{
    if (bytes != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, tp_type)) != NULL) {
        bytes = picoquic_frames_varint_encode(bytes, bytes_max, 0);
    }
    return bytes;
}
```

### Rust body
```rust
    {
        return None;
    }
```

## Pair `picoquic/transport.c:picoquic_decode_transport_preferred_address_address`
C: `picoquic/transport.c:130-162 picoquic_decode_transport_preferred_address_address`
Rust: `rs/fq/src/internal.rs:15160-15215 picoquic_decode_transport_preferred_address_address`

### C body
```c
{
    /* first compute the minimal length */
    size_t byte_index = 0;
    uint8_t cnx_id_length = 0;
    size_t minimal_length = 4u + 2u + 16u + 2u + 1u /* + preferred_address->connection_id.id_len */ + 16u;
    size_t ret = 0;

    if (bytes_max >= minimal_length) {
        memcpy(preferred_address->ipv4Address, bytes + byte_index, 4);
        byte_index += 4;
        preferred_address->ipv4Port = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        memcpy(preferred_address->ipv6Address, bytes + byte_index, 16);
        byte_index += 16;
        preferred_address->ipv6Port = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        cnx_id_length = bytes[byte_index++];
        if (cnx_id_length > 0 && cnx_id_length <= PICOQUIC_CONNECTION_ID_MAX_SIZE &&
            byte_index + (size_t)cnx_id_length + 16u <= bytes_max &&
            cnx_id_length == picoquic_parse_connection_id(bytes + byte_index, cnx_id_length,
                &preferred_address->connection_id)){
            byte_index += cnx_id_length;
            memcpy(preferred_address->statelessResetToken, bytes + byte_index, 16);
            byte_index += 16;
            ret = byte_index;
            preferred_address->is_defined = 1;
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> usize {
    let bytes_max = bytes_max.min(bytes.len());
    let minimal_length = 4 + 2 + 16 + 2 + 1 + 16;
    if bytes_max < minimal_length {
        return 0;
    }

    let mut byte_index = 0;
    let ipv4 = core::net::Ipv4Addr::new(
        bytes[byte_index],
        bytes[byte_index + 1],
        bytes[byte_index + 2],
        bytes[byte_index + 3],
    );
    byte_index += 4;
    let ipv4_port = parse_16(&bytes[byte_index..byte_index + 2]);
    byte_index += 2;

    let mut ipv6_octets = [0u8; 16];
    ipv6_octets.copy_from_slice(&bytes[byte_index..byte_index + 16]);
    byte_index += 16;
    let ipv6_port = parse_16(&bytes[byte_index..byte_index + 2]);
    byte_index += 2;

    let cnx_id_length = bytes[byte_index] as usize;
    byte_index += 1;
    if cnx_id_length == 0
        || cnx_id_length > crate::CONNECTION_ID_MAX_SIZE
        || byte_index + cnx_id_length + 16 > bytes_max
    {
        return 0;
    }
    let Some(connection_id) =
        ConnectionId::clone_from_slice(&bytes[byte_index..byte_index + cnx_id_length])
    else {
        return 0;
    };
    byte_index += cnx_id_length;

    let mut stateless_reset_token = [0u8; 16];
    stateless_reset_token.copy_from_slice(&bytes[byte_index..byte_index + 16]);
    byte_index += 16;

    preferred_address.v4 = Some(SocketAddr::from((ipv4, ipv4_port)));
    preferred_address.v6 = Some(SocketAddr::from((
        core::net::Ipv6Addr::from(ipv6_octets),
        ipv6_port,
    )));
    preferred_address.connection_id = connection_id;
    preferred_address.stateless_reset_token = stateless_reset_token;
    byte_index
}
```

## Pair `picoquic/transport.c:picoquic_prepare_transport_extensions`
C: `picoquic/transport.c:306-501 picoquic_prepare_transport_extensions`
Rust: `rs/fq/src/internal.rs:15327-15464 prepare_transport_extensions`

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

## Pair `picoquic/unified_log.c:picoquic_log_quic_pdu`
C: `picoquic/unified_log.c:48-55 picoquic_log_quic_pdu`
Rust: `rs/fq/src/logger.rs:264-284 log_pdu`

### C body
```c
{
    if (quic->F_log != NULL) {
        quic->text_log_fns->log_quic_pdu(quic, receiving, current_time, cid64, addr_peer, addr_local, packet_length);
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

## Pair `picoquic/unified_log.c:picoquic_log_pdu`
C: `picoquic/unified_log.c:110-131 picoquic_log_pdu`
Rust: `rs/fq/src/logger.rs:264-284 log_pdu`

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

## Pair `picoquic/unified_log.c:picoquic_log_outgoing_packet`
C: `picoquic/unified_log.c:189-210 picoquic_log_outgoing_packet`
Rust: `rs/fq/src/logger.rs:375-884 outgoing_packet`

### C body
```c
{
    if (picoquic_cnx_is_still_logging(cnx)) {
        if (cnx->quic->F_log != NULL) {
            cnx->quic->text_log_fns->log_outgoing_packet(cnx, path_x, bytes, sequence_number, pn_length, length,
                send_buffer, send_length, current_time);
        }

        if (cnx->f_binlog != NULL) {
            cnx->quic->bin_log_fns->log_outgoing_packet(cnx, path_x, bytes, sequence_number, pn_length, length,
                send_buffer, send_length, current_time);
        }

        if (cnx->qlog_ctx != NULL) {
            cnx->quic->qlog_fns->log_outgoing_packet(cnx, path_x, bytes, sequence_number, pn_length, length,
                send_buffer, send_length, current_time);
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

## Pair `picoquic/unified_log.c:picoquic_log_tls_ticket`
C: `picoquic/unified_log.c:268-282 picoquic_log_tls_ticket`
Rust: `rs/fq/src/logger.rs:416-884 tls_ticket`

### C body
```c
{
    if (cnx->quic->F_log != NULL) {
        cnx->quic->text_log_fns->log_picotls_ticket(cnx, ticket, ticket_length);
    }

    if (cnx->f_binlog != NULL) {
        cnx->quic->bin_log_fns->log_picotls_ticket(cnx, ticket, ticket_length);
    }

    if (cnx->qlog_ctx != NULL) {
        cnx->quic->qlog_fns->log_picotls_ticket(cnx, ticket, ticket_length);
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

## Pair `picoquic/util.c:picoquic_string_create`
C: `picoquic/util.c:45-71 picoquic_string_create`
Rust: `rs/fq/src/utils.rs:252-255 string_create`

### C body
```c
{
    size_t allocated = len + 1;
    char * str = NULL;

    /* tests to protect against integer overflow */
    if (allocated > 0) {
        str = (char*)malloc(allocated);

        if (str != NULL) {
            if (original == NULL || len == 0) {
                str[0] = 0;
            }
            else if (allocated > len) {
                memcpy(str, original, len);
                str[allocated - 1] = 0;
            }
            else {
                /* This could happen only in case of integer overflow */
                free(str);
                str = NULL;
            }
        }
    }

    return str;
}
```

### Rust body
```rust
    let Some(src) = original else {
        return String::new();
    };
```

## Pair `picoquic/util.c:get_debug_suspended`
C: `picoquic/util.c:116-119 get_debug_suspended`
Rust: `rs/fq/src/utils.rs:141-159 get_debug_suspended`

### C body
```c
{
    return debug_suspended;
}
```

### Rust body
```rust
pub fn debug_printf(msg: &str) {
    if DEBUG_SUSPENDED.with(|s| s.get()) {
        return;
    }
    DEBUG_OUT.with(|o| {
        if let Some(w) = o.borrow_mut().as_mut() {
            let _ = w.write_str(msg);
        }
    });
}
```
