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

## `picoquic/ticket_store.c:picoquic_save_retry_tokens`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C delegates to token saving, while Rust ignores the filename and returns Ok with a placeholder comment.
* C source: `picoquic/ticket_store.c:523-527`
* C signature: `int picoquic_save_retry_tokens(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/lib.rs:1825-1828`
* Rust item: `save_retry_tokens`

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

## `picoquic/tls_api.c:picoquic_crypto_context_free`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C frees and nulls crypto context handles; Rust body creates a packet key from a secret and returns it.
* C source: `picoquic/tls_api.c:1697-1718`
* C signature: `void picoquic_crypto_context_free(picoquic_crypto_context_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1607-1631`
* Rust item: `free_handles`

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

## `picoquic/tls_api.c:picoquic_get_app_secret_size`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body computes rotated keys, while C body returns the TLS cipher hash digest size.
* C source: `picoquic/tls_api.c:1599-1606`
* C signature: `size_t picoquic_get_app_secret_size(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1503-1547`
* Rust item: `app_secret_size`

### C body
```c
{
    picoquic_tls_ctx_t * tls_ctx = (picoquic_tls_ctx_t *)cnx->tls_ctx;

    ptls_cipher_suite_t * cipher = ptls_get_cipher(tls_ctx->tls);

    return (cipher->hash->digest_size);
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

## `picoquic/tls_api.c:picoquic_pn_encrypt`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C initializes a cipher with an IV and encrypts input bytes; Rust derives a mask from a sample and copies mask bytes to output.
* C source: `picoquic/tls_api.c:2374-2378`
* C signature: `void picoquic_pn_encrypt(void *, const void *, void *, const void *, size_t)`
* Rust source: `rs/fq/src/tls_api.rs:2155-2159`
* Rust item: `pn_encrypt`

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

## `picoquic/tls_api.c:picoquic_tls_api_init_providers`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust only handles minicrypto; C also handles OpenSSL, Fusion, and MbedTLS branches.
* C source: `picoquic/tls_api.c:143-181`
* C signature: `void picoquic_tls_api_init_providers(int)`
* Rust source: `rs/fq/src/tls_api.rs:2457-2460`
* Rust item: `tls_api_init_providers_locked`

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
