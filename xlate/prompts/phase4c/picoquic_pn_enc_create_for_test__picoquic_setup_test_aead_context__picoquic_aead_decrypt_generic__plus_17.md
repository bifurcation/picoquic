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

## Pair `picoquic/tls_api.c:picoquic_pn_enc_create_for_test`
C: `picoquic/tls_api.c:2359-2367 picoquic_pn_enc_create_for_test`
Rust: `rs/fq/src/tls_api.rs:1635-1640 pn_enc_create_for_test`

### C body
```c
{
    ptls_cipher_suite_t *cipher = picoquic_get_aes128gcm_sha256(1);
    void *v_pn_enc = NULL;
    
    (void)picoquic_set_pn_enc_from_secret(&v_pn_enc, cipher, 1, secret, prefix_label);

    return v_pn_enc;
}
```

### Rust body
```rust
) -> Option<Box<dyn crate::tls::HeaderKey>> {
    header_key_from_secret(secret, prefix_label).ok()
}
```

## Pair `picoquic/tls_api.c:picoquic_setup_test_aead_context`
C: `picoquic/tls_api.c:2403-2412 picoquic_setup_test_aead_context`
Rust: `rs/fq/src/tls_api.rs:1625-1640 setup_test_aead_context`

### C body
```c
{
    void * v_aead = NULL;
    ptls_cipher_suite_t* cipher = picoquic_get_aes128gcm_sha256(1);

    (void)picoquic_set_aead_from_secret(&v_aead, cipher, is_encrypt, secret, prefix_label);

    return v_aead;
}
```

### Rust body
```rust
) -> Option<Box<dyn crate::tls::HeaderKey>> {
    header_key_from_secret(secret, prefix_label).ok()
}
```

## Pair `picoquic/tls_api.c:picoquic_aead_decrypt_generic`
C: `picoquic/tls_api.c:2456-2471 picoquic_aead_decrypt_generic`
Rust: `rs/fq/src/tls_api.rs:1269-1286 aead_decrypt_generic`

### C body
```c
{
    size_t decrypted = 0;

    if (aead_ctx == NULL) {
        decrypted = SIZE_MAX;
    } else {
        decrypted = ptls_aead_decrypt((ptls_aead_context_t*)aead_ctx,
            (void*)output, (const void*)input, input_length, seq_num,
            (void*)auth_data, auth_data_length);
    }

    return decrypted;
}
```

### Rust body
```rust
) -> Result<usize, Error> {
    let aead_ctx = aead_ctx.ok_or(Error::Protocol(
        crate::errors::InternalError::AeadCheck as u64,
    ))?;
    let mut buf = input.to_vec();
    aead_ctx.decrypt(seq_num, auth_data, &mut buf)?;
    if buf.len() > output.len() {
        return Err(Error::BufferTooSmall);
    }
    output[..buf.len()].copy_from_slice(&buf);
    Ok(buf.len())
}
```

## Pair `picoquic/tls_api.c:picoquic_setup_cleartext_aead_salt`
C: `picoquic/tls_api.c:2532-2541 picoquic_setup_cleartext_aead_salt`
Rust: `rs/fq/src/tls_api.rs:1385-1392 setup_cleartext_aead_salt`

### C body
```c
{
    if (picoquic_supported_versions[version_index].version_aead_key != NULL && picoquic_supported_versions[version_index].version_aead_key_length > 0) {
        salt->base = picoquic_supported_versions[version_index].version_aead_key;
        salt->len = picoquic_supported_versions[version_index].version_aead_key_length;
    } else {
        salt->base = picoquic_cleartext_null_salt;
        salt->len = sizeof(picoquic_cleartext_null_salt);
    }
}
```

### Rust body
```rust
    if let Some(version) = version_from_index(version_index) {
        let params = version.parameters();
        if !params.version_aead_key.is_empty() {
            return params.version_aead_key;
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_set_tls_certificate_chain`
C: `picoquic/tls_api.c:2804-2813 picoquic_set_tls_certificate_chain`
Rust: `rs/fq/src/lib.rs:1627-1629 set_tls_certificate_chain`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;

    free_certificates_list(ctx->certificates.list, ctx->certificates.count);

    ctx->certificates.list = certs;
    ctx->certificates.count = count;
}
```

### Rust body
```rust
    pub fn set_tls_certificate_chain(&mut self, _certs: Vec<Vec<u8>>) {
        // Delegates to TLS backend configuration; complex.
    }
```

## Pair `picoquic/tls_api.c:picoquic_server_encrypt_retry_token`
C: `picoquic/tls_api.c:2849-2886 picoquic_server_encrypt_retry_token`
Rust: `rs/fq/src/tls_api.rs:1743-1775 picoquic_server_encrypt_retry_token`

### C body
```c
{
    int ret = 0;
    uint64_t sequence;
    uint8_t* auth_data;
    size_t auth_data_length;

    if (text_length + 1u + 16u > token_max) {
        ret = -1;
        *token_length = 0;
    }
    else {

        if (addr_peer->sa_family == AF_INET) {
            auth_data = (uint8_t*)&((struct sockaddr_in*)addr_peer)->sin_addr;
            auth_data_length = 4;
        }
        else {
            auth_data = (uint8_t*)&((struct sockaddr_in6*)addr_peer)->sin6_addr;
            auth_data_length = 16;
        }
        picoquic_crypto_random(quic, token, 8);
        if (is_new_token) {
            token[0] |= 0x80;
        }
        else {
            token[0] &= 0x7F;
        }
        sequence = PICOPARSE_64(token);

        *token_length = (size_t)8u + picoquic_aead_encrypt_generic(token + 8, text, text_length,
            sequence, auth_data, auth_data_length, quic->aead_encrypt_ticket_ctx);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<usize, Error> {
        ensure_ticket_aead_contexts(self)?;
        let required = 8usize
            .checked_add(text.len())
            .and_then(|n| n.checked_add(QUIC_AEAD_TAG_LEN))
            .ok_or(Error::BufferTooSmall)?;
        if token.len() < required {
            return Err(Error::BufferTooSmall);
        }

        self.crypto_random(&mut token[..8]);
        if is_new_token {
            token[0] |= 0x80;
        } else {
            token[0] &= 0x7f;
        }
        let sequence = u64::from_be_bytes(token[..8].try_into().unwrap());
        let aad = ip_auth_data(addr_peer);
        let mut payload = text.to_vec();
        let aead = self
            .aead_encrypt_ticket_ctx
            .as_ref()
            .ok_or(Error::InvalidState)?;
        aead.encrypt(sequence, &aad, &mut payload);
        token[8..8 + payload.len()].copy_from_slice(&payload);
        Ok(8 + payload.len())
    }
```

## Pair `picoquic/tls_api.c:picoquic_create_retry_protection_context`
C: `picoquic/tls_api.c:3116-3119 picoquic_create_retry_protection_context`
Rust: `rs/fq/src/tls_api.rs:2035-2085 create_retry_protection_context`

### C body
```c
{
    return (void *)picoquic_setup_test_aead_context(is_enc, key, prefix_label);
}
```

### Rust body
```rust
impl Quic {
    /// Look up (and lazily create) the retry-protection context for
    /// the chosen QUIC version, on the chosen direction.  C:
    /// `find_retry_protection_context`.
    pub fn find_retry_protection_context(
        &mut self,
        version_index: i32,
        sending: bool,
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

    /// Tear down every retry-protection AEAD context held by this
    /// context.  C: `delete_retry_protection_contexts`.
    pub fn delete_retry_protection_contexts(&mut self) {
        self.retry_integrity_sign_ctx.clear();
        self.retry_integrity_verify_ctx.clear();
    }
}
```

## Pair `picoquic/tls_api.c:picoquic_encode_retry_protection`
C: `picoquic/tls_api.c:3188-3199 picoquic_encode_retry_protection`
Rust: `rs/fq/src/tls_api.rs:2090-2098 encode_retry_protection`

### C body
```c
{
    size_t pseudo_index;
    uint8_t pseudo_packet[PICOQUIC_MAX_PACKET_SIZE];

    if (integrity_aead != NULL && byte_index + picoquic_aead_get_checksum_length(integrity_aead) < bytes_max &&
        (pseudo_index = picoquic_format_retry_protection_pseudo_packet(pseudo_packet, bytes, byte_index, odcid)) > 0){
        byte_index += picoquic_aead_encrypt_generic(bytes+byte_index, bytes+byte_index, 0, 0, pseudo_packet, pseudo_index, integrity_aead);
    }

    return byte_index;
}
```

### Rust body
```rust
    if byte_index > bytes.len() || byte_index + integrity_aead.tag_len() > bytes.len() {
        return byte_index;
    }
```

## Pair `picoquic/token_store.c:picoquic_deserialize_token`
C: `picoquic/token_store.c:98-155 picoquic_deserialize_token`
Rust: `rs/fq/src/internal.rs:1686-1709 deserialize_token`

### C body
```c
{
    int ret = 0;
    uint64_t time_valid_until = 0;
    size_t required_length = 8 + 2 + 2 + 2;
    size_t byte_index = 0;
    size_t sni_index = 0;
    size_t ip_addr_index = 0;
    size_t token_index = 0;
    uint16_t sni_length = 0;
    uint8_t ip_addr_length = 0;
    uint16_t token_length = 0;


    *consumed = 0;
    *token = NULL;

    if (required_length < bytes_max) {
        time_valid_until = PICOPARSE_64(bytes);
        byte_index = 8;
        sni_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        sni_index = byte_index;
        required_length += sni_length;
        byte_index += sni_length;
    }
    
    if (required_length < bytes_max) {
        ip_addr_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        ip_addr_index = byte_index;
        required_length += ip_addr_length;
        byte_index += ip_addr_length;
    }

    if (required_length < bytes_max) {
        token_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        token_index = byte_index;
        required_length += token_length;
    }

    if (required_length > bytes_max) {
        *token = NULL;
        ret = PICOQUIC_ERROR_INVALID_TOKEN;
    } else {
        *token = picoquic_format_token(time_valid_until, (const char *)(bytes + sni_index), sni_length,
            bytes + ip_addr_index, ip_addr_length, bytes + token_index, token_length);
        if (*token == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
        else {
            *consumed = required_length;
        }
    }

    return ret;
}
```

### Rust body
```rust
fn deserialize_token(bytes: &[u8]) -> Result<StoredToken, crate::Error> {
    let mut off = 0;
    let time_valid_until = Instant::from_ticks(parse_64(take_slice(bytes, &mut off, 8)?));

    let sni_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let sni = optional_string_from_bytes(take_slice(bytes, &mut off, sni_len)?)?;

    let ip_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let ip_addr = stored_ip_from_bytes(take_slice(bytes, &mut off, ip_len)?)?;

    let token_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let token = take_slice(bytes, &mut off, token_len)?.to_vec();
    if off != bytes.len() {
        return Err(crate::Error::InvalidFile);
    }

    Ok(StoredToken {
        sni,
        token,
        ip_addr,
        time_valid_until,
        was_used: false,
    })
}
```

## Pair `picoquic/token_store.c:picoquic_load_tokens`
C: `picoquic/token_store.c:282-348 picoquic_load_tokens`
Rust: `rs/fq/src/internal.rs:1788-1821 load_tokens`

### C body
```c
{
    int ret = 0;
    int file_ret = 0;
    FILE* F = NULL;
    picoquic_stored_token_t* previous = NULL;
    picoquic_stored_token_t* next = NULL;
    uint32_t record_size;
    uint32_t storage_size;
    uint64_t current_time = picoquic_get_tls_time(quic);
    picoquic_stored_token_t** pp_first_token = &quic->p_first_token;

    if ((F = picoquic_file_open_ex(token_file_name, "rb", &file_ret)) == NULL) {
        ret = (file_ret == ENOENT) ? PICOQUIC_ERROR_NO_SUCH_FILE : -1;
    }

    while (ret == 0) {
        if (fread(&storage_size, 4, 1, F) != 1) {
            /* end of file */
            break;
        }
        else if (storage_size > 2048 ||
            (record_size = storage_size + offsetof(struct st_picoquic_stored_token_t, time_valid_until)) > 2048) {
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        }
        else {
            uint8_t buffer[2048];
            if (fread(buffer, 1, storage_size, F) != storage_size) {
                ret = PICOQUIC_ERROR_INVALID_FILE;
            }
            else {
                size_t consumed = 0;
                ret = picoquic_deserialize_token(&next, buffer, storage_size, &consumed);

                if (ret == 0 && (consumed != storage_size || next == NULL)) {
                    ret = PICOQUIC_ERROR_INVALID_FILE;
                }

                if (ret == 0 && next != NULL) {
                    if (next->time_valid_until < current_time) {
                        free(next);
                        next = NULL;
                    }
                    else {
                        next->sni = ((char*)next) + sizeof(picoquic_stored_token_t);
                        next->ip_addr = ((uint8_t*)next->sni) + next->sni_length + 1;
                        next->token = (uint8_t*)(next->ip_addr + next->ip_addr_length + 1);
                        next->next_token = NULL;
                        if (previous == NULL) {
                            *pp_first_token = next;
                        }
                        else {
                            previous->next_token = next;
                        }

                        previous = next;
                    }
                }
            }
        }
    }

    (void)picoquic_file_close(F);

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let data = match std::fs::read(token_file_name) {
            Ok(data) => data,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(crate::Error::NoSuchFile);
            }
            Err(_) => return Err(crate::Error::InvalidFile),
        };

        let mut off = 0;
        self.stored_tokens.clear();
        while off < data.len() {
            let len_bytes = data.get(off..off + 4).ok_or(crate::Error::InvalidFile)?;
            let record_len =
                u32::from_ne_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]])
                    as usize;
            off += 4;
            if record_len > 2048 {
                return Err(crate::Error::InvalidFile);
            }
            let record = data
                .get(off..off + record_len)
                .ok_or(crate::Error::InvalidFile)?;
            off += record_len;
            let token = deserialize_token(record)?;
            if token.time_valid_until.ticks() > 0 {
                self.stored_tokens.push(token);
            }
        }
        Ok(())
    }
```

## Pair `picoquic/transport.c:picoquic_transport_param_type_varint_encode`
C: `picoquic/transport.c:59-66 picoquic_transport_param_type_varint_encode`
Rust: `rs/fq/src/internal.rs:15040-15048 picoquic_transport_param_type_varint_encode`

### C body
```c
{
    if (bytes != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, tp_type)) != NULL) {
        bytes = picoquic_transport_param_varint_encode(bytes, bytes_max, n64);
    }
    return bytes;
}
```

### Rust body
```rust
    if !encode_varint_at(bytes, &mut offset, tp_type as u64) {
        return None;
    }
```

## Pair `picoquic/transport.c:picoquic_encode_transport_preferred_address_address`
C: `picoquic/transport.c:98-128 picoquic_encode_transport_preferred_address_address`
Rust: `rs/fq/src/internal.rs:15113-15155 picoquic_encode_transport_preferred_address_address`

### C body
```c
{
    /* first compute the length */
    uint64_t coded_length = ((uint64_t)(4 + 2 + 16 + 2 + 1)) + preferred_address->connection_id.id_len + ((uint64_t)16);

    if (bytes != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, picoquic_tp_server_preferred_address)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, coded_length)) != NULL){
        if (bytes + coded_length > bytes_max) {
            bytes = NULL;
        }
        else {
            memcpy(bytes, preferred_address->ipv4Address, 4);
            bytes += 4;
            picoformat_16(bytes, preferred_address->ipv4Port);
            bytes += 2;
            memcpy(bytes, preferred_address->ipv6Address, 16);
            bytes += 16;
            picoformat_16(bytes, preferred_address->ipv4Port);
            bytes += 2;
            *bytes++ = preferred_address->connection_id.id_len;
            bytes += picoquic_format_connection_id(bytes, bytes_max - bytes,
                preferred_address->connection_id);
            memcpy(bytes, preferred_address->statelessResetToken, 16);
            bytes += 16;
        }
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let cid_len = preferred_address.connection_id.len();
    let coded_length = 4 + 2 + 16 + 2 + 1 + cid_len + 16;
    let mut offset = 0;
    if !encode_varint_at(
        bytes,
        &mut offset,
        crate::tp::TransportParameter::ServerPreferredAddress as u64,
    ) || !encode_varint_at(bytes, &mut offset, coded_length as u64)
        || bytes.len().saturating_sub(offset) < coded_length
    {
        return None;
    }

    let (ipv4, ipv4_port) = match preferred_address.v4 {
        Some(SocketAddr::V4(addr)) => (addr.ip().octets(), addr.port()),
        _ => ([0u8; 4], 0),
    };
    let ipv6 = match preferred_address.v6 {
        Some(SocketAddr::V6(addr)) => addr.ip().octets(),
        _ => [0u8; 16],
    };

    bytes[offset..offset + 4].copy_from_slice(&ipv4);
    offset += 4;
    format_16(&mut bytes[offset..offset + 2], ipv4_port);
    offset += 2;
    bytes[offset..offset + 16].copy_from_slice(&ipv6);
    offset += 16;
    format_16(&mut bytes[offset..offset + 2], ipv4_port);
    offset += 2;
    bytes[offset] = cid_len as u8;
    offset += 1;
    bytes[offset..offset + cid_len].copy_from_slice(preferred_address.connection_id.as_bytes());
    offset += cid_len;
    bytes[offset..offset + 16].copy_from_slice(&preferred_address.stateless_reset_token);
    offset += 16;

    Some(&mut bytes[offset..])
}
```

## Pair `picoquic/transport.c:picoquic_negotiate_multipath_option`
C: `picoquic/transport.c:283-304 picoquic_negotiate_multipath_option`
Rust: `rs/fq/src/internal.rs:15312-15325 negotiate_multipath_option`

### C body
```c
{
    int ret = 0;

    cnx->is_multipath_enabled = 0;

    if (cnx->remote_parameters.initial_max_path_id > 0 &&
        cnx->local_parameters.initial_max_path_id > 0) {
        /* Enable the multipath option */
        cnx->is_multipath_enabled = 1;
        cnx->max_path_id_acknowledged = cnx->local_parameters.initial_max_path_id;
        cnx->max_path_id_remote = cnx->remote_parameters.initial_max_path_id;
        cnx->max_path_id_local = cnx->local_parameters.initial_max_path_id;
    }
    else {
        if (!cnx->client_mode) {
            cnx->local_parameters.initial_max_path_id = 0;
        } 
    }

    return ret;
}
```

### Rust body
```rust
    pub fn negotiate_multipath_option(&mut self) -> i32 {
        self.is_multipath_enabled = false;
        if self.remote_parameters.initial_max_path_id > 0
            && self.local_parameters.initial_max_path_id > 0
        {
            self.is_multipath_enabled = true;
            self.max_path_id_acknowledged = self.local_parameters.initial_max_path_id;
            self.max_path_id_remote = self.remote_parameters.initial_max_path_id;
            self.max_path_id_local = self.local_parameters.initial_max_path_id;
        } else if !self.client_mode {
            self.local_parameters.initial_max_path_id = 0;
        }
        0
    }
```

## Pair `picoquic/unified_log.c:picoquic_log_close_logs`
C: `picoquic/unified_log.c:32-46 picoquic_log_close_logs`
Rust: `rs/fq/src/logger.rs:292-295 close_logs`

### C body
```c
{
    if (quic->text_log_fns != NULL) {
        quic->text_log_fns->log_quic_close(quic);
    }

    if (quic->bin_log_fns != NULL) {
        quic->bin_log_fns->log_quic_close(quic);
    }

    if (quic->qlog_fns != NULL) {
        quic->qlog_fns->log_quic_close(quic);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().quic_close(self);
        }
```

## Pair `picoquic/unified_log.c:picoquic_log_context_free_app_message`
C: `picoquic/unified_log.c:100-108 picoquic_log_context_free_app_message`
Rust: `rs/fq/src/logger.rs:253-256 log_app_message`

### C body
```c
{
    if (quic->F_log != NULL) {
        va_list args;
        va_start(args, fmt);
        quic->text_log_fns->log_quic_app_message(quic, cid, fmt, args);
        va_end(args);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().quic_app_message(self, cid, args);
        }
```

## Pair `picoquic/unified_log.c:picoquic_log_buffered_packet`
C: `picoquic/unified_log.c:171-187 picoquic_log_buffered_packet`
Rust: `rs/fq/src/logger.rs:366-884 buffered_packet`

### C body
```c
{
    if (picoquic_cnx_is_still_logging(cnx)) {
        if (cnx->quic->F_log != NULL) {
            cnx->quic->text_log_fns->log_buffered_packet(cnx, path_x, ptype, current_time);
        }

        if (cnx->f_binlog != NULL) {
            cnx->quic->bin_log_fns->log_buffered_packet(cnx, path_x, ptype, current_time);
        }

        if (cnx->qlog_ctx != NULL) {
            cnx->quic->qlog_fns->log_buffered_packet(cnx, path_x, ptype, current_time);
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

## Pair `picoquic/unified_log.c:picoquic_log_transport_extension`
C: `picoquic/unified_log.c:251-266 picoquic_log_transport_extension`
Rust: `rs/fq/src/logger.rs:411-884 transport_extension`

### C body
```c
{
    if (cnx->quic->F_log != NULL) {
        cnx->quic->text_log_fns->log_transport_extension(cnx, is_local, param_length, params);
    }

    if (cnx->f_binlog != NULL) {
        cnx->quic->bin_log_fns->log_transport_extension(cnx, is_local, param_length, params);
    }

    if (cnx->qlog_ctx != NULL) {
        cnx->quic->qlog_fns->log_transport_extension(cnx, is_local, param_length, params);
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

## Pair `picoquic/unified_log.c:picoquic_log_cc_dump`
C: `picoquic/unified_log.c:315-345 picoquic_log_cc_dump`
Rust: `rs/fq/src/logger.rs:434-454 cc_dump`

### C body
```c
{
    if (cnx->memlog_call_back != NULL) {
        cnx->memlog_call_back(cnx, cnx->path[0], cnx->memlog_ctx, 0, current_time);
    }

    if (picoquic_cnx_is_still_logging(cnx)) {
        picoquic_path_t* path_x;
        for (int path_index = 0; path_index < cnx->nb_paths; path_index++)
        {
            path_x = cnx->path[path_index];

            if (!path_x->is_cc_data_updated) {
                continue;
            }

            if (cnx->quic->F_log != NULL) {
                cnx->quic->text_log_fns->log_cc_dump(cnx, path_x, current_time);
            }
            if (cnx->f_binlog != NULL) {
                cnx->quic->bin_log_fns->log_cc_dump(cnx, path_x, current_time);
            }
            if (cnx->qlog_ctx != NULL) {
                cnx->quic->qlog_fns->log_cc_dump(cnx, path_x, current_time);
            }

            path_x->is_cc_data_updated = 0;
        }
    }
}
```

### Rust body
```rust
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
```

## Pair `picoquic/util.c:get_debug_out`
C: `picoquic/util.c:111-114 get_debug_out`
Rust: `rs/fq/src/utils.rs:134-143 get_debug_out`

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

## Pair `picoquic/util.c:debug_printf_pop_stream`
C: `picoquic/util.c:207-214 debug_printf_pop_stream`
Rust: `rs/fq/src/utils.rs:178-187 debug_printf_pop_stream`

### C body
```c
{
    if (debug_out == NULL) {
        fprintf(stderr, "No current err out\n");
        exit(1);
    }
    debug_out = NULL;
}
```

### Rust body
```rust
pub fn debug_printf_pop_stream() {
    DEBUG_OUT.with(|o| {
        let mut guard = o.borrow_mut();
        assert!(
            guard.is_some(),
            "debug_printf_pop_stream: no current stream"
        );
        *guard = None;
    });
}
```
