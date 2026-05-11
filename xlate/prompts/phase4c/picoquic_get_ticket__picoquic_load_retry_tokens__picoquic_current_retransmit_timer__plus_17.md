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

## Pair `picoquic/ticket_store.c:picoquic_get_ticket`
C: `picoquic/ticket_store.c:385-396 picoquic_get_ticket`
Rust: `rs/fq/src/internal.rs:1507-1516 get_ticket`

### C body
```c
{
    uint32_t ticket_version = 0;
    int ret = picoquic_get_ticket_and_version(quic,
        sni, sni_length, alpn, alpn_length, version, &ticket_version,
        ticket, ticket_length, tp, mark_used);

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(&[u8], TransportParameters), crate::Error> {
        let (_, ticket, tp) = self.get_ticket_and_version(sni, alpn, version, mark_used)?;
        Ok((ticket, tp))
    }
```

## Pair `picoquic/ticket_store.c:picoquic_load_retry_tokens`
C: `picoquic/ticket_store.c:517-521 picoquic_load_retry_tokens`
Rust: `rs/fq/src/lib.rs:1813-1816 load_retry_tokens`

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

## Pair `picoquic/timing.c:picoquic_current_retransmit_timer`
C: `picoquic/timing.c:42-88 picoquic_current_retransmit_timer`
Rust: `rs/fq/src/internal.rs:9111-9145 current_retransmit_timer`

### C body
```c
{
    uint64_t rto = path_x->retransmit_timer;

    if (path_x->nb_retransmit > 0) {
        if (path_x->nb_retransmit < 3) {
            rto <<= path_x->nb_retransmit;
        }
        else {
            uint64_t n1 = path_x->nb_retransmit - 2;
            if (n1 > 18) {
                n1 = 18;
            }
            rto <<= (2 + (n1 / 4));
            n1 &= 3;
            rto += (n1*rto) >> 2;
        }
        if (cnx->idle_timeout > 15) {
            if (rto > (cnx->idle_timeout >> 4)) {
                rto = cnx->idle_timeout >> 4;
            }
        }
    }

    if (cnx->cnx_state < picoquic_state_client_ready_start) {
        if (PICOQUIC_MICROSEC_HANDSHAKE_MAX / 1000 < cnx->local_parameters.max_idle_timeout) {
            /* Special case of very long delays */
            rto = path_x->retransmit_timer << path_x->nb_retransmit;
            if (rto > cnx->local_parameters.max_idle_timeout * 100) {
                rto = cnx->local_parameters.max_idle_timeout * 100;
            }
        } else if (rto > PICOQUIC_INITIAL_MAX_RETRANSMIT_TIMER) {
            rto = PICOQUIC_INITIAL_MAX_RETRANSMIT_TIMER;
        }
    }
    else if (rto > PICOQUIC_LARGE_RETRANSMIT_TIMER){
        uint64_t alt_rto = PICOQUIC_LARGE_RETRANSMIT_TIMER;
        if (path_x->rtt_min > PICOQUIC_TARGET_SATELLITE_RTT) {
            alt_rto = (path_x->smoothed_rtt * 3) >> 1;
        }
        if (alt_rto < rto) {
            rto = alt_rto;
        }
    }

    return rto;
}
```

### Rust body
```rust
    pub fn current_retransmit_timer(&self, path_x: &mut Path) -> u64 {
        let mut rto = path_x.retransmit_timer.ticks();
        if path_x.nb_retransmit > 0 {
            if path_x.nb_retransmit < 3 {
                rto = saturating_shl_u64(rto, path_x.nb_retransmit as u32);
            } else {
                let mut n1 = path_x.nb_retransmit.saturating_sub(2).min(18);
                rto = saturating_shl_u64(rto, (2 + (n1 / 4)) as u32);
                n1 &= 3;
                rto = rto.saturating_add((n1.saturating_mul(rto)) >> 2);
            }
            let idle = self.idle_timeout.ticks();
            if idle > 15 {
                rto = rto.min(idle >> 4);
            }
        }

        if self.connection_state < State::ClientReadyStart
            && MICROSEC_HANDSHAKE_MAX.ticks() / 1000
                < self.local_parameters.max_idle_timeout.ticks()
        {
            rto = path_x
                .retransmit_timer
                .ticks()
                .checked_shl(path_x.nb_retransmit.min(18) as u32)
                .unwrap_or(u64::MAX);
            rto = rto.min(
                self.local_parameters
                    .max_idle_timeout
                    .ticks()
                    .saturating_mul(100),
            );
        }
        rto.max(MIN_RETRANSMIT_TIMER.ticks())
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_api_init_providers`
C: `picoquic/tls_api.c:143-181 picoquic_tls_api_init_providers`
Rust: `rs/fq/src/tls_api.rs:2457-2460 tls_api_init_providers_locked`

### C body
```c
{
    if ((tls_api_init_flags & TLS_API_INIT_FLAGS_NO_MINICRYPTO) == 0) {
        DBG_PRINTF("%s minicrypto", (unload)?"Unloading":"Loading");
        picoquic_ptls_minicrypto_load(unload);
    }
#ifndef PTLS_WITHOUT_OPENSSL
    if ((tls_api_init_flags & TLS_API_INIT_FLAGS_NO_OPENSSL) == 0) {
        DBG_PRINTF("%s openssl", (unload)?"Unloading":"Loading");
        picoquic_ptls_openssl_load(unload);
    }
#else
    if (unload == 0 && tls_api_is_init == 0) {
        DBG_PRINTF("%s", "Picoquic was compiled without OpenSSL");
    }
#endif
    // picoquic_bcrypt_load(unload);
#if (!defined(_WINDOWS) || defined(_WINDOWS64)) && !defined(PTLS_WITHOUT_FUSION)
    if ((tls_api_init_flags & TLS_API_INIT_FLAGS_NO_FUSION) == 0) {
        DBG_PRINTF("%s fusion", (unload)?"Unloading":"Loading");
        picoquic_ptls_fusion_load(unload);
    }
#else
    if (unload == 0 && tls_api_is_init == 0) {
        DBG_PRINTF("%s", "Picoquic was compiled without Fusion");
    }
#endif

#ifdef PICOQUIC_WITH_MBEDTLS
    if ((tls_api_init_flags & TLS_API_INIT_FLAGS_NO_MBEDTLS) == 0) {
        DBG_PRINTF("%s MbedTLS", (unload)?"Unloading":"Loading");
        picoquic_mbedtls_load(unload);
    }
#endif
}
```

### Rust body
```rust
    if (state.init_flags & crate::TLS_API_INIT_FLAGS_NO_MINICRYPTO) == 0 {
        ptls_minicrypto_load_locked(state, unload);
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_api_reset`
C: `picoquic/tls_api.c:247-257 picoquic_tls_api_reset`
Rust: `rs/fq/src/tls_api.rs:2502-2512 tls_api_reset`

### C body
```c
{
    if (tls_api_is_init) {
        tls_api_is_init = 0;
        picoquic_tls_api_init_providers(2);
        picoquic_tls_api_zero();
    }
    tls_api_init_flags = init_flags;
    picoquic_tls_api_init_providers(0);
    tls_api_is_init = 1;
}
```

### Rust body
```rust
pub fn tls_api_reset(init_flags: u64) {
    let mut state = tls_api_state();
    if state.is_init {
        state.is_init = false;
        tls_api_init_providers_locked(&mut state, 2);
    }
    state.zero();
    state.init_flags = init_flags;
    tls_api_init_providers_locked(&mut state, 0);
    state.is_init = true;
}
```

## Pair `picoquic/tls_api.c:picoquic_register_crypto_random_provider_fn`
C: `picoquic/tls_api.c:355-358 picoquic_register_crypto_random_provider_fn`
Rust: `rs/fq/src/tls_api.rs:2372-2374 register_crypto_random_provider`

### C body
```c
{
    picoquic_crypto_random_provider_fn = crypto_random_provider_fn;
}
```

### Rust body
```rust
    fn register_crypto_random_provider(&mut self, provider: CryptoProvider) {
        self.crypto_random_provider = Some(provider);
    }
```

## Pair `picoquic/tls_api.c:picoquic_ecb_create_by_name`
C: `picoquic/tls_api.c:478-488 picoquic_ecb_create_by_name`
Rust: `rs/fq/src/tls_api.rs:2237-2244 ecb_create_by_name`

### C body
```c
{
    void* created = NULL;
    ptls_cipher_algorithm_t* ecb_cipher = picoquic_get_ecb_cipher_by_id(alg_name);

    if (ecb_cipher != NULL) {
        created = (void*)ptls_cipher_new(ecb_cipher, is_enc, ecb_key);
    }
    
    return created;
}
```

### Rust body
```rust
    if get_ecb_cipher_by_name(alg_name) {
        Some(Aes128EcbContext::new(is_enc, ecb_key))
    } else {
```

## Pair `picoquic/tls_api.c:picoquic_hash_create`
C: `picoquic/tls_api.c:521-530 picoquic_hash_create`
Rust: `rs/fq/src/tls_api.rs:1953-1973 hash_create`

### C body
```c
void* picoquic_hash_create(char const* algorithm_name) {
    ptls_hash_context_t* ctx = NULL;
    ptls_hash_algorithm_t*hash = picoquic_get_hash_algorithm_by_name(algorithm_name);

    if (hash != NULL) {
        ctx = hash->create();
    }

    return (void*)ctx;
}
```

### Rust body
```rust
pub fn hash_get_length(algorithm_name: &str) -> usize {
    match algorithm_name {
        "sha256" | "SHA256" | "SHA-256" => 32,
        "sha384" | "SHA384" | "SHA-384" => 48,
        "sha512" | "SHA512" | "SHA-512" => 64,
        _ => 0,
    }
}
```

## Pair `picoquic/tls_api.c:picoquic_set_private_key_from_file`
C: `picoquic/tls_api.c:611-614 picoquic_set_private_key_from_file`
Rust: `rs/fq/src/tls_api.rs:1995-2002 set_private_key_from_file`

### C body
```c
{
    return set_private_key_from_file(file_name, quic->tls_master_ctx);
}
```

### Rust body
```rust
    pub fn set_private_key_from_file(&mut self, file_name: &str) -> Result<(), Error> {
        let contents = std::fs::read_to_string(file_name).map_err(|_| Error::NoSuchFile)?;
        if contents.contains("-----BEGIN ") && contents.contains("PRIVATE KEY-----") {
            Ok(())
        } else {
            Err(Error::InvalidFile)
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_clear_crypto_errors`
C: `picoquic/tls_api.c:691-699 picoquic_clear_crypto_errors`
Rust: `rs/fq/src/sys/openssl.rs:467-469 clear_crypto_errors`

### C body
```c
{
    if (picoquic_clear_crypto_errors_fn != NULL) {
        picoquic_clear_crypto_errors_fn();
    }
}
```

### Rust body
```rust
pub fn clear_crypto_errors() {
    drop(openssl::error::ErrorStack::get());
}
```

## Pair `picoquic/tls_api.c:picoquic_get_cipher_suite_by_id_v`
C: `picoquic/tls_api.c:731-734 picoquic_get_cipher_suite_by_id_v`
Rust: `rs/fq/src/tls_api.rs:2581-2586 picoquic_get_cipher_suite_by_id_v`

### C body
```c
{
    return (void*)picoquic_get_cipher_suite_by_id(cipher_suite_id, use_low_memory);
}
```

### Rust body
```rust
) -> Option<u16> {
    picoquic_get_cipher_suite_by_id(cipher_suite_id, use_low_memory)
}
```

## Pair `picoquic/tls_api.c:picoquic_crypto_uniform_random`
C: `picoquic/tls_api.c:778-788 picoquic_crypto_uniform_random`
Rust: `rs/fq/src/tls_api.rs:1187-1198 picoquic_crypto_uniform_random`

### C body
```c
{
    uint64_t rnd;
    uint64_t rnd_min = UINT64_MAX % rnd_max;

    do {
        picoquic_crypto_random(quic, &rnd, sizeof(rnd));
    } while (rnd < rnd_min);

    return rnd % rnd_max;
}
```

### Rust body
```rust
    pub fn picoquic_crypto_uniform_random(&mut self, rnd_max: u64) -> u64 {
        assert!(rnd_max > 0, "rnd_max must be positive");
        let rnd_min = u64::MAX % rnd_max;
        loop {
            let mut bytes = [0u8; 8];
            self.crypto_random(&mut bytes);
            let rnd = u64::from_ne_bytes(bytes);
            if rnd >= rnd_min {
                return rnd % rnd_max;
            }
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_public_random_seed`
C: `picoquic/tls_api.c:859-866 picoquic_public_random_seed`
Rust: `rs/fq/src/tls_api.rs:1204-1210 picoquic_public_random_seed`

### C body
```c
{
    uint64_t seed[3];
    picoquic_crypto_random(quic, &seed, sizeof(seed));

    picoquic_public_random_seed_64(seed[0], 0);
    public_random_obfuscator = seed[1];
}
```

### Rust body
```rust
    pub fn picoquic_public_random_seed(&mut self) {
        let mut bytes = [0u8; 24];
        self.crypto_random(&mut bytes);
        let seed = u64::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let obfuscator = u64::from_ne_bytes(bytes[8..16].try_into().unwrap());
        crate::public_random_seed_from_crypto(seed, obfuscator);
    }
```

## Pair `picoquic/tls_api.c:picoquic_set_pn_enc_from_secret`
C: `picoquic/tls_api.c:1311-1330 picoquic_set_pn_enc_from_secret`
Rust: `rs/fq/src/tls_api.rs:535-544 set_pn_enc_from_secret`

### C body
```c
{
    uint8_t pnekey[PTLS_MAX_SECRET_SIZE];
    int ret;

    if (*v_pn_enc != NULL) {
        ptls_cipher_free((ptls_cipher_context_t *)*v_pn_enc);
        *v_pn_enc = NULL;
    }

    if ((ret = ptls_hkdf_expand_label(cipher->hash, pnekey, 
        cipher->aead->ctr_cipher->key_size, ptls_iovec_init(secret, cipher->hash->digest_size), 
        PICOQUIC_LABEL_HP, ptls_iovec_init(NULL, 0), prefix_label)) == 0) {
        if ((*v_pn_enc = ptls_cipher_new(cipher->aead->ctr_cipher, is_enc, pnekey)) == NULL) {
            ret = PTLS_ERROR_NO_MEMORY;
        }
    }
    
    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    *pn_enc_slot = Some(header_key_from_secret(secret, prefix_label)?);
    Ok(())
}
```

## Pair `picoquic/tls_api.c:picoquic_setup_initial_secrets`
C: `picoquic/tls_api.c:1453-1476 picoquic_setup_initial_secrets`
Rust: `rs/fq/src/tls_api.rs:1353-1376 setup_initial_secrets`

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

## Pair `picoquic/tls_api.c:picoquic_rotate_app_secret`
C: `picoquic/tls_api.c:1563-1589 picoquic_rotate_app_secret`
Rust: `rs/fq/src/tls_api.rs:1569-1598 rotate_app_secret`

### C body
```c
{
    int ret = 0;
    uint8_t new_secret[PTLS_MAX_DIGEST_SIZE];

    ret = ptls_hkdf_expand_label(cipher->hash, new_secret,
        cipher->hash->digest_size, ptls_iovec_init(secret, cipher->hash->digest_size), traffic_update_label,
        ptls_iovec_init(NULL, 0), PICOQUIC_LABEL_QUIC_BASE);
    if (ret == 0) {
        memcpy(secret, new_secret, cipher->hash->digest_size);
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    let mut new_secret = vec![0u8; secret.len()];
    match hash.output_size() {
        32 => hkdf_expand_label_sha256(
            traffic_update_label,
            TLS13_LABEL_PREFIX,
            secret,
            &mut new_secret,
        )?,
        48 => hkdf_expand_label_sha384(
            traffic_update_label,
            TLS13_LABEL_PREFIX,
            secret,
            &mut new_secret,
        )?,
        64 => hkdf_expand_label_sha512(
            traffic_update_label,
            TLS13_LABEL_PREFIX,
            secret,
            &mut new_secret,
        )?,
        _ => return Err(Error::InvalidArgument),
    }
    secret.copy_from_slice(&new_secret);
    Ok(())
}
```

## Pair `picoquic/tls_api.c:picoquic_apply_rotated_keys`
C: `picoquic/tls_api.c:1668-1691 picoquic_apply_rotated_keys`
Rust: `rs/fq/src/tls_api.rs:1552-1561 apply_rotated_keys`

### C body
```c
{
    if (is_enc) {
        if (cnx->crypto_context[3].aead_encrypt != NULL) {
            ptls_aead_free((ptls_aead_context_t *)cnx->crypto_context[3].aead_encrypt);
        }

        cnx->crypto_context[3].aead_encrypt = cnx->crypto_context_new.aead_encrypt;
        cnx->crypto_context_new.aead_encrypt = NULL;

        cnx->key_phase_enc ^= 1;
    }
    else {
        if (cnx->crypto_context_old.aead_decrypt != NULL) {
            ptls_aead_free((ptls_aead_context_t *)cnx->crypto_context_old.aead_decrypt);
        }

        cnx->crypto_context_old.aead_decrypt = cnx->crypto_context[3].aead_decrypt;
        cnx->crypto_context[3].aead_decrypt = cnx->crypto_context_new.aead_decrypt;
        cnx->crypto_context_new.aead_decrypt = NULL;

        cnx->key_phase_dec ^= 1;
    }
}
```

### Rust body
```rust
    pub fn apply_rotated_keys(&mut self, is_enc: bool) {
        if is_enc {
            self.crypto_context[3].aead_encrypt = self.crypto_context_new.aead_encrypt.take();
            self.key_phase_enc = !self.key_phase_enc;
        } else {
            self.crypto_context_old.aead_decrypt = self.crypto_context[3].aead_decrypt.take();
            self.crypto_context[3].aead_decrypt = self.crypto_context_new.aead_decrypt.take();
            self.key_phase_dec = !self.key_phase_dec;
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_get_tls_time`
C: `picoquic/tls_api.c:1915-1922 picoquic_get_tls_time`
Rust: `rs/fq/src/tls_api.rs:1169-1171 tls_time`

### C body
```c
{
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;
    uint64_t now = ctx->get_time->cb(ctx->get_time)*1000;

    return now;
}
```

### Rust body
```rust
    pub fn tls_time(&self) -> u64 {
        self.time()
    }
```

## Pair `picoquic/tls_api.c:picoquic_tlscontext_trim_after_handshake`
C: `picoquic/tls_api.c:2118-2133 picoquic_tlscontext_trim_after_handshake`
Rust: `rs/fq/src/tls_api.rs:1026-1029 trim_tls_context_after_handshake`

### C body
```c
{
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;

    if (ctx->ext_data != NULL) {
        free(ctx->ext_data);
        ctx->ext_data = NULL;
        ctx->ext_data_size = 0;
    }

    if (ctx->alpn_vec != NULL) {
        free(ctx->alpn_vec);
        ctx->alpn_vec = NULL;
        ctx->alpn_vec_size = 0;
    }
}
```

### Rust body
```rust
    pub fn trim_tls_context_after_handshake(&mut self) {
        self.tls_sendbuf.clear();
        self.tls_sendbuf.shrink_to_fit();
    }
```

## Pair `picoquic/tls_api.c:picoquic_add_to_tls_stream`
C: `picoquic/tls_api.c:2165-2204 picoquic_add_to_tls_stream`
Rust: `rs/fq/src/tls_api.rs:701-720 queue_tls_bytes`

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
