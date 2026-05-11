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

## `picoquic/ticket_store.c:picoquic_load_retry_tokens`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust ignores the filename and returns Ok with a placeholder comment instead of calling the token loader.
* C source: `picoquic/ticket_store.c:517-521`
* C signature: `int picoquic_load_retry_tokens(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/lib.rs:1813-1816`
* Rust item: `load_retry_tokens`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return picoquic_load_tokens(quic, token_store_filename);
}
```

### Rust body
```rust
    pub fn load_retry_tokens(&mut self, _token_store_filename: &str) -> Result<(), Error> {
        // Complex: involves file I/O and token deserialization.
        Ok(())
    }
```

## `picoquic/tls_api.c:picoquic_aead_get_checksum_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns AEAD tag size capped at 16, but Rust body is a confidentiality-limit function returning aead_ctx.confidentiality_limit().
* C source: `picoquic/tls_api.c:2393-2401`
* C signature: `size_t picoquic_aead_get_checksum_length(void *)`
* Rust source: `rs/fq/src/tls_api.rs:1250-1260`
* Rust item: `aead_get_checksum_length`

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

## `picoquic/tls_api.c:picoquic_get_aes128gcm_sha256_v`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the AES128-GCM-SHA256 cipher suite pointer; Rust body returns whether the minicrypto key loader is active.
* C source: `picoquic/tls_api.c:715-718`
* C signature: `void * picoquic_get_aes128gcm_sha256_v(int)`
* Rust source: `rs/fq/src/lib.rs:4806-4814`
* Rust item: `is_minicrypto_aes128gcm_sha256`

### C body
```c
{
    return (void*)picoquic_get_aes128gcm_sha256(use_low_memory);
}
```

### Rust body
```rust
pub fn is_minicrypto_key_loader() -> bool {
    crate::tls_api::is_minicrypto_key_loader()
}
```

## `picoquic/tls_api.c:picoquic_initialize_tls_stream`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body configures SNI/ALPN, ticket resumption, zero-share mode, TLS extensions, ptls_handle_message error handling, and disconnects on setup failure; Rust body only writes handshake bytes, queues them, possibly installs keys, and transitions state.
* C source: `picoquic/tls_api.c:2243-2353`
* C signature: `int picoquic_initialize_tls_stream(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/tls_api.rs:1127-1139`
* Rust item: `initialize_tls_stream`

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

## `picoquic/tls_api.c:picoquic_setup_initial_secrets`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sizes HKDF by cipher hash digest_size; Rust hardcodes SHA256_SIZE.
* C source: `picoquic/tls_api.c:1453-1476`
* C signature: `int picoquic_setup_initial_secrets(ptls_cipher_suite_t *, uint8_t *, uint8_t *, uint8_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1353-1376`
* Rust item: `setup_initial_secrets`

### C body
```c
{
    int ret = 0;
    ptls_iovec_t prk;

    prk.base = master_secret;
    prk.len = cipher->hash->digest_size;

    /* Get the client secret */
    ret = ptls_hkdf_expand_label(cipher->hash, client_secret, cipher->hash->digest_size,
        prk, PICOQUIC_LABEL_INITIAL_CLIENT, ptls_iovec_init(NULL, 0), NULL);

    if (ret == 0) {
        /* Get the server secret */
        ret = ptls_hkdf_expand_label(cipher->hash, server_secret, cipher->hash->digest_size,
            prk, PICOQUIC_LABEL_INITIAL_SERVER, ptls_iovec_init(NULL, 0), NULL);
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    if master_secret.len() < SHA256_SIZE
        || client_secret.len() < SHA256_SIZE
        || server_secret.len() < SHA256_SIZE
    {
        return Err(Error::BufferTooSmall);
    }
    hkdf_expand_label(
        LABEL_INITIAL_CLIENT,
        TLS13_LABEL_PREFIX,
        &master_secret[..SHA256_SIZE],
        &mut client_secret[..SHA256_SIZE],
    )?;
    hkdf_expand_label(
        LABEL_INITIAL_SERVER,
        TLS13_LABEL_PREFIX,
        &master_secret[..SHA256_SIZE],
        &mut server_secret[..SHA256_SIZE],
    )
}
```
