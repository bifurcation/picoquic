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

## `picoquic/sockloop.c:picoquic_packet_loop_v2`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust calls a different loop helper and lacks body-visible callback context handling and return_code extraction shown in C.
* C source: `picoquic/sockloop.c:1600-1614`
* C signature: `int picoquic_packet_loop_v2(picoquic_quic_t *, picoquic_packet_loop_param_t *, picoquic_packet_loop_cb_fn, void *)`
* Rust source: `rs/fq/src/packet_loop.rs:1265-1282`
* Rust item: `run_v2`

### C body
```c
{
    picoquic_network_thread_ctx_t thread_ctx = { 0 };

    thread_ctx.quic = quic;
    thread_ctx.param = param;
    thread_ctx.loop_callback = loop_callback;
    thread_ctx.loop_callback_ctx = loop_callback_ctx;

    (void)picoquic_packet_loop_v3((void*)&thread_ctx);
    return thread_ctx.return_code;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        let mut thread_ctx = NetworkThreadCtx::default();
        thread_ctx.param = Some(Box::new(*param));
        thread_ctx.loop_callback = loop_callback;
        let mut owned_param = thread_ctx.param.take().ok_or(Error::Memory)?;
        let result = run_packet_loop::<crate::socks_socket2::Socket2Udp>(
            self,
            &mut owned_param,
            &mut thread_ctx,
        );
        *param = *owned_param;
        thread_ctx.param = Some(owned_param);
        result
    }
```

## `picoquic/tls_api.c:picoquic_add_to_tls_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: C appends copied bytes with node offset 0; Rust uses and increments sent_offset and updates nb_bytes_queued.
* C source: `picoquic/tls_api.c:2165-2204`
* C signature: `int picoquic_add_to_tls_stream(picoquic_cnx_t *, const uint8_t *, size_t, int)`
* Rust source: `rs/fq/src/tls_api.rs:701-720`
* Rust item: `queue_tls_bytes`

### C body
```c
{
    int ret = 0;
    picoquic_stream_head_t* stream = &cnx->tls_stream[epoch];

    if (length > 0) {
        picoquic_stream_queue_node_t* stream_data = (picoquic_stream_queue_node_t*)
            malloc(sizeof(picoquic_stream_queue_node_t));
        if (stream_data == 0) {
            ret = -1;
        }
        else {
            stream_data->bytes = (uint8_t*)malloc(length);

            if (stream_data->bytes == NULL) {
                free(stream_data);
                stream_data = NULL;
                ret = -1;
            }
            else {
                picoquic_stream_queue_node_t** pprevious = &stream->send_queue;
                picoquic_stream_queue_node_t* next = stream->send_queue;

                memcpy(stream_data->bytes, data, length);
                stream_data->length = length;
                stream_data->offset = 0;
                stream_data->next_stream_data = NULL;

                while (next != NULL) {
                    pprevious = &next->next_stream_data;
                    next = next->next_stream_data;
                }

                *pprevious = stream_data;
            }
        }
    }

    return ret;
}
```

### Rust body
```rust
fn queue_tls_bytes(cnx: &mut Connection, epoch: usize, bytes: &[u8]) -> Result<(), Error> {
    if epoch >= NUMBER_OF_EPOCHS {
        return Err(Error::InvalidArgument);
    }
    if bytes.is_empty() {
        return Ok(());
    }
    let stream = &mut cnx.tls_stream[epoch];
    let offset = stream.sent_offset;
    stream.sent_offset = stream
        .sent_offset
        .checked_add(bytes.len() as u64)
        .ok_or(Error::InvalidArgument)?;
    stream.send_queue.push_back(StreamQueueNode {
        offset,
        bytes: bytes.to_vec(),
    });
    cnx.nb_bytes_queued = cnx.nb_bytes_queued.saturating_add(bytes.len() as u64);
    Ok(())
}
```

## `picoquic/tls_api.c:picoquic_compute_new_rotated_keys`
* Phase 4C status: `suspect`
* Phase 4C rationale: The high-level flow matches, but C uses the TLS cipher hash while Rust visibly hardcodes Sha256 for rotating both secrets.
* C source: `picoquic/tls_api.c:1608-1666`
* C signature: `int picoquic_compute_new_rotated_keys(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1513-1547`
* Rust item: `compute_new_rotated_keys`

### C body
```c
{
    int ret = 0;
    picoquic_tls_ctx_t * tls_ctx = (picoquic_tls_ctx_t *)cnx->tls_ctx;
    ptls_cipher_suite_t * cipher = ptls_get_cipher(tls_ctx->tls);
    const char *prefix_label = picoquic_supported_versions[cnx->version_index].tls_prefix_label;
    const char *traffic_update_label = picoquic_supported_versions[cnx->version_index].tls_traffic_update_label;

    /* Verify that the previous transition is complete */
    if (cnx->crypto_context_new.aead_decrypt != NULL ||
        cnx->crypto_context_new.aead_encrypt != NULL) {
        if (cnx->crypto_context_new.aead_decrypt == NULL ||
            cnx->crypto_context_new.aead_encrypt == NULL) {
            ret = PICOQUIC_ERROR_CANNOT_COMPUTE_KEY;
        }
        else {
            /* already computed */
            return 0;
        }
    }

    /* Recompute the secrets */
    if (ret == 0) {
        ret = picoquic_rotate_app_secret(cipher, tls_ctx->app_secret_enc, traffic_update_label);
#ifdef _DEBUG
        if (ret == 0) {
            DBG_PRINTF("Rotated Encryption Secret (%d):\n", (int)cipher->hash->digest_size);
            debug_dump(tls_ctx->app_secret_enc, (int)cipher->hash->digest_size);
        }
        else {
            DBG_PRINTF("Encryption secret rotation fails, ret=%x\n", ret);
        }
#endif
    }

    if (ret == 0) {
        ret = picoquic_set_key_from_secret(cipher, 1, 1, &cnx->crypto_context_new, tls_ctx->app_secret_enc, prefix_label);
    }

    if (ret == 0) {
        ret = picoquic_rotate_app_secret(cipher, tls_ctx->app_secret_dec, traffic_update_label);
#ifdef _DEBUG
        if (ret == 0) {
            DBG_PRINTF("Rotated Decryption Secret (%d):\n", (int)cipher->hash->digest_size);
            debug_dump(tls_ctx->app_secret_dec, (int)cipher->hash->digest_size);
        }
        else {
            DBG_PRINTF("Decryption secret rotation fails, ret=%x\n", ret);
        }
#endif

    }

    if (ret == 0) {
        ret = picoquic_set_key_from_secret(cipher, 0, 1, &cnx->crypto_context_new, tls_ctx->app_secret_dec, prefix_label);
    }

    return (ret == 0)?0: PICOQUIC_ERROR_CANNOT_COMPUTE_KEY;
}
```

### Rust body
```rust
    pub fn compute_new_rotated_keys(&mut self) -> Result<(), Error> {
        let has_enc = self.crypto_context_new.aead_encrypt.is_some();
        let has_dec = self.crypto_context_new.aead_decrypt.is_some();
        if has_enc || has_dec {
            return if has_enc && has_dec {
                Ok(())
            } else {
                Err(Error::Protocol(InternalError::CannotComputeKey as u64))
            };
        }

        let version = connection_version(self);
        let params = version.parameters();
        let secret_len = self.app_secret_size();
        rotate_app_secret(
            &mut sha2::Sha256::new(),
            &mut self.app_secret_enc[..secret_len],
            params.tls_traffic_update_label,
        )?;
        self.crypto_context_new.aead_encrypt = Some(packet_key_from_secret(
            &self.app_secret_enc[..secret_len],
            params.tls_prefix_label,
        )?);

        rotate_app_secret(
            &mut sha2::Sha256::new(),
            &mut self.app_secret_dec[..secret_len],
            params.tls_traffic_update_label,
        )?;
        self.crypto_context_new.aead_decrypt = Some(packet_key_from_secret(
            &self.app_secret_dec[..secret_len],
            params.tls_prefix_label,
        )?);
        Ok(())
    }
```

## `picoquic/tls_api.c:picoquic_pn_enc_create_for_test`
* Phase 4C status: `suspect`
* Phase 4C rationale: C explicitly selects aes128gcm_sha256 and passes is_enc=1 into picoquic_set_pn_enc_from_secret; Rust only calls header_key_from_secret(secret, prefix_label).ok(), with no visible cipher or direction argument.
* C source: `picoquic/tls_api.c:2359-2367`
* C signature: `void * picoquic_pn_enc_create_for_test(const uint8_t *, const char *)`
* Rust source: `rs/fq/src/tls_api.rs:1635-1640`
* Rust item: `pn_enc_create_for_test`

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

## `picoquic/tls_api.c:picoquic_set_aead_from_secret`
* Phase 4C status: `suspect`
* Phase 4C rationale: C frees an existing AEAD before creating a new one and uses the is_enc argument; Rust replaces the slot with a new key and does not visibly use is_enc.
* C source: `picoquic/tls_api.c:1296-1309`
* C signature: `int picoquic_set_aead_from_secret(void **, ptls_cipher_suite_t *, int, const void *, const char *)`
* Rust source: `rs/fq/src/tls_api.rs:510-525`
* Rust item: `set_aead_from_secret`

### C body
```c
{
    int ret = 0;

    if (*v_aead != NULL) {
        ptls_aead_free((ptls_aead_context_t*)*v_aead);
    }

    if ((*v_aead = ptls_aead_new(cipher->aead, cipher->hash, is_enc, secret, prefix_label)) == NULL) {
        ret = PTLS_ERROR_NO_MEMORY;
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    let key: Box<dyn crate::tls::PacketKey> = match suite {
        AeadSuiteId::Aes128GcmSha256 => packet_key_from_secret(secret, prefix_label)?,
        AeadSuiteId::Aes256GcmSha384 => {
            Box::new(Aes256GcmPacketKey::from_secret(secret, prefix_label)?)
        }
    };
    *aead_slot = Some(key);
    Ok(())
}
```
