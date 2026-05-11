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

## Pair `picoquic/ticket_store.c:picoquic_store_ticket`
C: `picoquic/ticket_store.c:259-326 picoquic_store_ticket`
Rust: `rs/fq/src/internal.rs:1439-1479 store_ticket`

### C body
```c
{
    uint64_t current_time = picoquic_get_tls_time(quic);
    picoquic_stored_ticket_t** pp_first_ticket = &quic->p_first_ticket;
    int ret = 0;

    if (ticket_length < 17) {
        ret = PICOQUIC_ERROR_INVALID_TICKET;
    } else {
        uint64_t ticket_issued_time;
        uint64_t ttl_seconds;
        uint64_t time_valid_until;

        ticket_issued_time = PICOPARSE_64(ticket);
        ttl_seconds = PICOPARSE_32(ticket + 13);

        if (ttl_seconds > (7 * 24 * 3600)) {
            ttl_seconds = (7 * 24 * 3600);
        }

        time_valid_until = (ticket_issued_time * 1000) + (ttl_seconds * 1000000);

        if (current_time != 0 && time_valid_until < current_time) {
            ret = PICOQUIC_ERROR_INVALID_TICKET;
        } else {
            picoquic_stored_ticket_t* stored = picoquic_format_ticket(time_valid_until, sni, sni_length,
                alpn, alpn_length, version, ip_addr, ip_addr_length,
                ip_addr_client, ip_addr_client_length,
                ticket, ticket_length, tp);
            if (stored == NULL) {
                ret = PICOQUIC_ERROR_MEMORY;
            }
            else {
                picoquic_stored_ticket_t* next;
                picoquic_stored_ticket_t** pprevious;

                stored->next_ticket = next = *pp_first_ticket;
                *pp_first_ticket = stored;
                pprevious = &stored->next_ticket;

                /* Now remove the old tickets for that SNI & ALPN & version */
                while (next != NULL) {
                    if (next->time_valid_until <= stored->time_valid_until &&
                        next->sni_length == sni_length &&
                        next->alpn_length == alpn_length &&
                        memcmp(next->sni, sni, sni_length) == 0 &&
                        memcmp(next->alpn, alpn, alpn_length) == 0 &&
                        next->version == version) {
                        picoquic_stored_ticket_t* deleted = next;
                        next = next->next_ticket;
                        *pprevious = next;
                        memset(&deleted->ticket, 0, deleted->ticket_length);
                        free(deleted);
                    } else {
                        pprevious = &next->next_ticket;
                        next = next->next_ticket;
                    }
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
        let time_valid_until = ticket_valid_until(ticket)?;
        let stored = StoredTicket {
            sni: sni.map(str::to_owned),
            alpn: alpn.map(str::to_owned),
            ip_addr,
            ip_addr_client,
            tp_0rtt: {
                use crate::tp::TransportParameter0RttKind::*;
                let mut arr = [0u64; crate::tp::NB_TP_0RTT];
                arr[MaxData as usize] = tp.initial_max_data;
                arr[MaxStreamDataBidiLocal as usize] = tp.initial_max_stream_data_bidi_local;
                arr[MaxStreamDataBidiRemote as usize] = tp.initial_max_stream_data_bidi_remote;
                arr[MaxStreamDataUni as usize] = tp.initial_max_stream_data_uni;
                arr[MaxStreamsIdBidir as usize] = tp.initial_max_stream_id_bidir;
                arr[MaxStreamsIdUnidir as usize] = tp.initial_max_stream_id_unidir;
                arr
            },
            ticket: ticket.to_vec(),
            time_valid_until,
            version,
            was_used: false,
        };
        self.stored_tickets.retain(|old| {
            old.sni.as_deref() != sni
                || old.alpn.as_deref() != alpn
                || old.version != version
                || old.time_valid_until > time_valid_until
        });
        self.stored_tickets.insert(0, stored);
        Ok(())
    }
```

## Pair `picoquic/ticket_store.c:picoquic_save_tickets`
C: `picoquic/ticket_store.c:398-431 picoquic_save_tickets`
Rust: `rs/fq/src/internal.rs:1595-1615 save_tickets`

### C body
```c
{
    int ret = 0;
    FILE* F = NULL;
    const picoquic_stored_ticket_t* next = first_ticket;

    if ((F = picoquic_file_open(ticket_file_name, "wb")) == NULL) {
        ret = -1;
    } else {
        while (ret == 0 && next != NULL) {
            /* Only store the tickets that are valid going forward */
            if (next->time_valid_until > current_time && next->was_used == 0) {
                /* Compute the serialized size */
                uint8_t buffer[2048];
                size_t record_size;

                ret = picoquic_serialize_ticket(next, buffer, sizeof(buffer), &record_size);

                if (ret == 0) {
                    if (fwrite(&record_size, 4, 1, F) != 1 || fwrite(buffer, 1, record_size, F) != record_size) {
                        ret = PICOQUIC_ERROR_INVALID_FILE;
                        break;
                    }
                }
            }
            next = next->next_ticket;
        }
        (void)picoquic_file_close(F);
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let mut file = File::create(ticket_file_name).map_err(|_| crate::Error::InvalidFile)?;
        for ticket in &self.stored_tickets {
            if ticket.time_valid_until > current_time && !ticket.was_used {
                let record = serialize_ticket(ticket)?;
                if record.len() > 2048 {
                    return Err(crate::Error::InvalidFile);
                }
                let len = (record.len() as u32).to_ne_bytes();
                file.write_all(&len)
                    .map_err(|_| crate::Error::InvalidFile)?;
                file.write_all(&record)
                    .map_err(|_| crate::Error::InvalidFile)?;
            }
        }
        Ok(())
    }
```

## Pair `picoquic/ticket_store.c:picoquic_save_retry_tokens`
C: `picoquic/ticket_store.c:523-527 picoquic_save_retry_tokens`
Rust: `rs/fq/src/lib.rs:1825-1828 save_retry_tokens`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return picoquic_save_tokens(quic, ticket_store_filename);
}
```

### Rust body
```rust
    pub fn save_retry_tokens(&mut self, _token_store_filename: &str) -> Result<(), Error> {
        // Complex: involves file I/O and token serialization.
        Ok(())
    }
```

## Pair `picoquic/timing.c:picoquic_validate_bdp_seed`
C: `picoquic/timing.c:90-114 picoquic_validate_bdp_seed`
Rust: `rs/fq/src/internal.rs:9152-9162 validate_bdp_seed`

### C body
```c
{
    if (path_x == cnx->path[0] && cnx->seed_cwin != 0 &&
        !cnx->cwin_notified_from_seed){
        uint64_t rtt_margin = rtt_sample / 4;
        if (cnx->seed_rtt_min >= rtt_sample - rtt_margin &&
            cnx->seed_rtt_min <= rtt_sample + rtt_margin) {
            uint8_t* ip_addr;
            uint8_t ip_addr_length;
            picoquic_get_ip_addr((struct sockaddr*)&path_x->first_tuple->peer_addr, &ip_addr, &ip_addr_length);

            if (ip_addr_length == cnx->seed_ip_addr_length &&
                memcmp(ip_addr, cnx->seed_ip_addr, ip_addr_length) == 0) {
                picoquic_per_ack_state_t ack_state = { 0 };
                ack_state.pc = picoquic_packet_context_application; /* Arbitrary! */
                ack_state.nb_bytes_acknowledged = (uint64_t)cnx->seed_cwin;
                cnx->cwin_notified_from_seed = 1;
                cnx->congestion_alg->alg_notify(cnx, path_x,
                    picoquic_congestion_notification_seed_cwin,
                    &ack_state, current_time);
            }
        }
    }
}
```

### Rust body
```rust
        {
            return;
        }
```

## Pair `picoquic/tls_api.c:picoquic_tls_api_zero`
C: `picoquic/tls_api.c:183-202 picoquic_tls_api_zero`
Rust: `rs/fq/src/tls_api.rs:2325-2331 zero`

### C body
```c
{
    memset(picoquic_cipher_suites, 0, sizeof(picoquic_cipher_suites));
    memset((void*)picoquic_key_exchanges, 0, sizeof(picoquic_key_exchanges));
    memset((void*)picoquic_key_exchange_secp256r1, 0, sizeof(picoquic_key_exchange_secp256r1));

    picoquic_set_private_key_from_file_fn = NULL;
    picoquic_dispose_sign_certificate_fn = NULL;
    picoquic_get_certs_from_file_fn = NULL;
    picoquic_get_public_key_from_private_fn = NULL;

    picoquic_get_certificate_verifier_fn = NULL;
    picoquic_dispose_certificate_verifier_fn = NULL;
    picoquic_set_tls_root_certificates_fn = NULL;

    picoquic_explain_crypto_error_fn = NULL;
    picoquic_clear_crypto_errors_fn = NULL;
 
    picoquic_crypto_random_provider_fn = NULL;
}
```

### Rust body
```rust
    fn zero(&mut self) {
        self.cipher_suites = [CipherSuiteSlot::EMPTY; PICOQUIC_CIPHER_SUITES_NB_MAX];
        self.key_exchanges = [KeyExchangeSlot::EMPTY; PICOQUIC_KEY_EXCHANGES_NB_MAX];
        self.key_exchange_secp256r1 = None;
        self.crypto_random_provider = None;
        self.private_key_provider = None;
    }
```

## Pair `picoquic/tls_api.c:picoquic_register_ciphersuite`
C: `picoquic/tls_api.c:258-274 picoquic_register_ciphersuite`
Rust: `rs/fq/src/tls_api.rs:2336-2350 register_ciphersuite`

### C body
```c
{
    for (int i = 0; i < PICOQUIC_CIPHER_SUITES_NB_MAX; i++) {
        if (picoquic_cipher_suites[i].high_memory_suite == NULL ||
            picoquic_cipher_suites[i].high_memory_suite->id == suite->id) {
            /* Replace the lower priority provider if present! */
            picoquic_cipher_suites[i].high_memory_suite = suite;
            if (is_low_memory) {
                picoquic_cipher_suites[i].low_memory_suite = suite;
            }
            break;
        }
    }
}
```

### Rust body
```rust
            if slot.high_memory_suite.is_none() || slot.id == suite_id {
                slot.id = suite_id;
                slot.high_memory_suite = Some(provider);
                if is_low_memory {
                    slot.low_memory_suite = Some(provider);
                }
                break;
            }
```

## Pair `picoquic/tls_api.c:picoquic_set_cipher_suite`
C: `picoquic/tls_api.c:427-433 picoquic_set_cipher_suite`
Rust: `rs/fq/src/lib.rs:1556-1561 set_cipher_suite`

### C body
```c
{
    ptls_context_t* ctx;
    PICOQUIC_THREAD_CHECK(quic);
    ctx = (ptls_context_t*)quic->tls_master_ctx;
    return (picoquic_set_cipher_suite_in_ctx(ctx, cipher_suite_id, quic->use_low_memory));
}
```

### Rust body
```rust
    pub fn set_cipher_suite(&mut self, cipher_suite_id: u16) -> Result<(), Error> {
        match cipher_suite_id {
            0 | AES_128_GCM_SHA256 | AES_256_GCM_SHA384 | CHACHA20_POLY1305_SHA256 => Ok(()),
            _ => Err(Error::InvalidArgument),
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_aes128_ecb_create`
C: `picoquic/tls_api.c:489-491 picoquic_aes128_ecb_create`
Rust: `rs/fq/src/tls_api.rs:2196-2206 new`

### C body
```c
void* picoquic_aes128_ecb_create(int is_enc, const void* ecb_key) {
    return picoquic_ecb_create_by_name(is_enc, ecb_key, "AES128-ECB");
}
```

### Rust body
```rust
        } else {
            Aes128EcbContext::Decrypt(
                aes::Aes128Dec::new_from_slice(ecb_key).expect("key is 16 bytes"),
            )
        }
```

## Pair `picoquic/tls_api.c:picoquic_hash_get_length`
C: `picoquic/tls_api.c:532-541 picoquic_hash_get_length`
Rust: `rs/fq/src/tls_api.rs:1966-1973 hash_get_length`

### C body
```c
size_t picoquic_hash_get_length(char const* algorithm_name) {
    size_t len = 0;
    ptls_hash_algorithm_t*hash = picoquic_get_hash_algorithm_by_name(algorithm_name);

    if (hash != NULL) {
        len = hash->digest_size;
    }

    return len;
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

## Pair `picoquic/tls_api.c:picoquic_get_certs_from_file`
C: `picoquic/tls_api.c:631-641 picoquic_get_certs_from_file`
Rust: `rs/fq/src/sys/openssl.rs:346-350 get_certs_from_file`

### C body
```c
{
    if (picoquic_get_certs_from_file_fn == NULL) {
        return NULL;
    }
    else {
        return picoquic_get_certs_from_file_fn(file_name, count);
    }
}
```

### Rust body
```rust
    let pem = match std::fs::read(file_name) {
        Ok(data) => data,
        Err(_) => return Vec::new(),
    };
```

## Pair `picoquic/tls_api.c:picoquic_get_aes128gcm_sha256`
C: `picoquic/tls_api.c:709-713 picoquic_get_aes128gcm_sha256`
Rust: `rs/fq/src/tls_api.rs:2574-2586 picoquic_get_aes128gcm_sha256`

### C body
```c
{
    return picoquic_get_cipher_suite_by_id(PICOQUIC_AES_128_GCM_SHA256, use_low_memory);
}
```

### Rust body
```rust
) -> Option<u16> {
    picoquic_get_cipher_suite_by_id(cipher_suite_id, use_low_memory)
}
```

## Pair `picoquic/tls_api.c:picoquic_hash_update`
C: `picoquic/tls_api.c:736-738 picoquic_hash_update`
Rust: `rs/fq/src/tls_api.rs:615-641 hash_update`

### C body
```c
void picoquic_hash_update(uint8_t* input, size_t input_length, void* hash_context) {
    ((ptls_hash_context_t*)hash_context)->update((ptls_hash_context_t*)hash_context, input, input_length);
}
```

### Rust body
```rust
pub fn hash_finalize(output: &mut [u8], hash_context: HashContext) {
    use digest::Digest;
    let copy = |result: &[u8], out: &mut [u8]| {
        let n = result.len().min(out.len());
        out[..n].copy_from_slice(&result[..n]);
    };
    match hash_context {
        HashContext::Sha256(h) => copy(&h.finalize(), output),
        HashContext::Sha384(h) => copy(&h.finalize(), output),
        HashContext::Sha512(h) => copy(&h.finalize(), output),
    }
}
```

## Pair `picoquic/tls_api.c:picoquic_public_random_step`
C: `picoquic/tls_api.c:820-831 picoquic_public_random_step`
Rust: `rs/fq/src/lib.rs:5116-5125 step`

### C body
```c
{
    uint64_t s1;
    const uint64_t s0 = public_random_seed[public_random_index++];
    public_random_index &= 15;
    s1 = public_random_seed[public_random_index];
    s1 ^= (s1 << 31); // a
    s1 ^= (s1 >> 11); // b
    s1 ^= (s0 ^ (s0 >> 30)); // c
    public_random_seed[public_random_index] = s1;
    return s1;
}
```

### Rust body
```rust
    fn step(&mut self) -> u64 {
        let s0 = self.seed[self.index];
        self.index = (self.index + 1) & 15;
        let mut s1 = self.seed[self.index];
        s1 ^= s1 << 31;
        s1 ^= s1 >> 11;
        s1 ^= s0 ^ (s0 >> 30);
        self.seed[self.index] = s1;
        s1
    }
```

## Pair `picoquic/tls_api.c:picoquic_public_random`
C: `picoquic/tls_api.c:868-880 picoquic_public_random`
Rust: `rs/fq/src/internal.rs:211-223 public_random`

### C body
```c
{
    uint8_t* x = buf;

    while (len > 0) {
        uint64_t y = picoquic_public_random_64();
        for (int i = 0; i < 8 && len > 0; i++) {
            *x++ = (uint8_t)(y & 255);
            y >>= 8;
            len--;
        }
    }
}
```

### Rust body
```rust
    while off < bytes.len() {
        let mut random = crate::public_random_64();
        for _ in 0..8 {
            if off >= bytes.len() {
                break;
            }
            bytes[off] = random as u8;
            random >>= 8;
            off += 1;
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_aes128_ecb_encrypt`
C: `picoquic/tls_api.c:1337-1340 picoquic_aes128_ecb_encrypt`
Rust: `rs/fq/src/tls_api.rs:2213-2221 process`

### C body
```c
{
    ptls_cipher_encrypt((ptls_cipher_context_t*)v_aesecb, output, input, len);
}
```

### Rust body
```rust
    pub fn process(&self, block: &mut [u8; 16]) {
        use cipher::{BlockDecrypt, BlockEncrypt};
        let mut b = cipher::generic_array::GenericArray::clone_from_slice(block);
        match self {
            Aes128EcbContext::Encrypt(enc) => enc.encrypt_block(&mut b),
            Aes128EcbContext::Decrypt(dec) => dec.decrypt_block(&mut b),
        }
        block.copy_from_slice(&b);
    }
```

## Pair `picoquic/tls_api.c:picoquic_compute_initial_secrets`
C: `picoquic/tls_api.c:1478-1498 picoquic_compute_initial_secrets`
Rust: `rs/fq/src/tls_api.rs:685-698 picoquic_compute_initial_secrets`

### C body
```c
{
    int ret = 0;
    ptls_iovec_t salt;
    uint8_t master_secret[256]; /* secret_max */
    *cipher = picoquic_get_aes128gcm_sha256(quic->use_low_memory);
    if (*cipher == NULL) {
        ret = -1;
    }
    else {
        picoquic_setup_cleartext_aead_salt(version_index, &salt);

        /* Extract the master key -- key length will be 32 per SHA256 */
        ret = picoquic_setup_initial_master_secret(*cipher, salt, *initial_cnxid, master_secret);
        if (ret == 0) {
            ret = picoquic_setup_initial_secrets(*cipher, master_secret, client_secret, server_secret);
        }
    }
    return ret;
}
```

### Rust body
```rust
) -> Result<(AeadSuiteId, [u8; SHA256_SIZE], [u8; SHA256_SIZE]), Error> {
    let suite = AeadSuiteId::Aes128GcmSha256;
    let salt = setup_cleartext_aead_salt(version_index);
    let mut master = [0u8; SHA256_SIZE];
    let mut client = [0u8; SHA256_SIZE];
    let mut server = [0u8; SHA256_SIZE];
    setup_initial_master_secret(salt, *initial_cnxid, &mut master)?;
    setup_initial_secrets(&master, &mut client, &mut server)?;
    Ok((suite, client, server))
}
```

## Pair `picoquic/tls_api.c:picoquic_get_app_secret`
C: `picoquic/tls_api.c:1592-1597 picoquic_get_app_secret`
Rust: `rs/fq/src/tls_api.rs:1488-1497 app_secret`

### C body
```c
{
    picoquic_tls_ctx_t * tls_ctx = (picoquic_tls_ctx_t *)cnx->tls_ctx;

    return (is_enc) ?tls_ctx->app_secret_enc:tls_ctx->app_secret_dec;
}
```

### Rust body
```rust
    pub fn app_secret(&mut self, is_enc: bool) -> &mut [u8] {
        if self.app_secret_len == 0 || self.app_secret_len > HASH_SIZE_MAX {
            self.app_secret_len = SHA256_SIZE;
        }
        if is_enc {
            &mut self.app_secret_enc[..self.app_secret_len]
        } else {
            &mut self.app_secret_dec[..self.app_secret_len]
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_crypto_context_free`
C: `picoquic/tls_api.c:1697-1718 picoquic_crypto_context_free`
Rust: `rs/fq/src/tls_api.rs:1607-1631 free_handles`

### C body
```c
{
    if (ctx->aead_encrypt != NULL) {
        ptls_aead_free((ptls_aead_context_t *)ctx->aead_encrypt);
        ctx->aead_encrypt = NULL;
    }

    if (ctx->aead_decrypt != NULL) {
        ptls_aead_free((ptls_aead_context_t *)ctx->aead_decrypt);
        ctx->aead_decrypt = NULL;
    }

    if (ctx->pn_enc != NULL) {
        ptls_cipher_free((ptls_cipher_context_t *)ctx->pn_enc);
        ctx->pn_enc = NULL;
    }

    if (ctx->pn_dec != NULL) {
        ptls_cipher_free((ptls_cipher_context_t *)ctx->pn_dec);
        ctx->pn_dec = NULL;
    }
}
```

### Rust body
```rust
) -> Option<Box<dyn crate::tls::PacketKey>> {
    packet_key_from_secret(secret, prefix_label).ok()
}
```

## Pair `picoquic/tls_api.c:picoquic_tlscontext_create`
C: `picoquic/tls_api.c:1924-1997 picoquic_tlscontext_create`
Rust: `rs/fq/src/tls_api.rs:986-1021 create_tls_context`

### C body
```c
{
    int ret = 0;
    /* allocate a context structure, but only if checks are correct */
    picoquic_tls_ctx_t* ctx = NULL;

    if (!cnx->client_mode && ((ptls_context_t*)quic->tls_master_ctx)->encrypt_ticket == NULL) {
        /* A server side connection, but no cert/key where given for the master context */
        ret = PICOQUIC_ERROR_TLS_SERVER_CON_WITHOUT_CERT;
    }
    else {
        ctx = (picoquic_tls_ctx_t*)malloc(sizeof(picoquic_tls_ctx_t));
        if (ctx == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
    }

    /* Create the TLS context */
    if (ctx != NULL) {
        memset(ctx, 0, sizeof(picoquic_tls_ctx_t));
        ctx->ext_data_size = PICOQUIC_TRANSPORT_PARAMETERS_MAX_SIZE;
        if (!cnx->client_mode && quic->test_large_server_flight) {
            ctx->ext_data_size += 4096;
        }
        ctx->ext_data = (uint8_t*)malloc(ctx->ext_data_size);
        ctx->alpn_vec = (ptls_iovec_t*)malloc(sizeof(ptls_iovec_t) * PICOQUIC_ALPN_NUMBER_MAX);
        if (ctx->ext_data == NULL || ctx->alpn_vec == NULL) {
            ret = -1;
        }
        else {
            ctx->alpn_vec_size = PICOQUIC_ALPN_NUMBER_MAX;
            ctx->cnx = cnx;

            ctx->handshake_properties.collect_extension = picoquic_tls_collect_extensions_cb;
            ctx->handshake_properties.collected_extensions = picoquic_tls_collected_extensions_cb;
            ctx->client_mode = cnx->client_mode;

            ctx->tls = ptls_new((ptls_context_t*)quic->tls_master_ctx,
                (ctx->client_mode) ? 0 : 1);
            if (ctx->tls == NULL) {
                picoquic_tlscontext_free(ctx, cnx->client_mode);
                ctx = NULL;
                ret = PICOQUIC_ERROR_MEMORY;
            }
            else{
                *ptls_get_data_ptr(ctx->tls) = cnx;
                if (!ctx->client_mode) {
                    /* The server should never attempt a stateless retry */
                    ctx->handshake_properties.server.enforce_retry = 0;
                    ctx->handshake_properties.server.retry_uses_cookie = 0;
                    ctx->handshake_properties.server.cookie.key = NULL;
                    ctx->handshake_properties.server.cookie.additional_data.base = NULL;
                    ctx->handshake_properties.server.cookie.additional_data.len = 0;
                }
                else {
                    ctx->handshake_properties.client.ech.retry_configs = &ctx->retry_configs;
                }
            }
        }
    }

    if (cnx->tls_ctx != NULL) {
        picoquic_tlscontext_free(cnx->tls_ctx, cnx->client_mode);
    }

    cnx->tls_ctx = (void*)ctx;

    return ret;
}
```

### Rust body
```rust
    pub fn create_tls_context(&mut self, quic: &mut Quic) -> Result<(), Error> {
        let version = connection_version(self);
        let transport_params = Vec::new();
        let session: Box<dyn crate::tls::Session> = if self.client_mode {
            if let Some(config) = quic.tls_client_config.as_ref() {
                let sni = self.sni.as_deref().unwrap_or("");
                config
                    .start_session(version as u32, sni, &transport_params)
                    .map_err(|_| Error::Tls)?
            } else {
                Box::new(LocalSession::new(
                    true,
                    version.parameters().tls_prefix_label,
                ))
            }
        } else if let Some(config) = quic.tls_server_config.as_ref() {
            config
                .start_session(version as u32, &transport_params)
                .map_err(|_| Error::Tls)?
        } else {
            if quic.enforce_client_only {
                return Err(Error::Protocol(
                    InternalError::TlsServerConWithoutCert as u64,
                ));
            }
            Box::new(LocalSession::new(
                false,
                version.parameters().tls_prefix_label,
            ))
        };

        self.tls_ctx = Some(session);
        self.tls_sendbuf.clear();
        self.app_secret_len = SHA256_SIZE;
        Ok(())
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_get_negotiated_alpn`
C: `picoquic/tls_api.c:2135-2142 picoquic_tls_get_negotiated_alpn`
Rust: `rs/fq/src/tls_api.rs:1148-1158 tls_get_negotiated_alpn`

### C body
```c
{
    picoquic_tls_ctx_t* ctx;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;

    return ptls_get_negotiated_protocol(ctx->tls);
}
```

### Rust body
```rust
    pub fn tls_get_sni(&self) -> Option<&str> {
        self.sni.as_deref()
    }
```
