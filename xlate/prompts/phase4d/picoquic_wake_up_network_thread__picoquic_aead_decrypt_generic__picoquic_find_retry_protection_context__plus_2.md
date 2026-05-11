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

## `picoquic/sockloop.c:picoquic_wake_up_network_thread`
* Phase 4C status: `suspect`
* Phase 4C rationale: C wakes via platform event or writes to a wake-up pipe; Rust only sends on wake_up_sender and errors if it is absent.
* C source: `picoquic/sockloop.c:1820-1849`
* C signature: `int picoquic_wake_up_network_thread(picoquic_network_thread_ctx_t *)`
* Rust source: `rs/fq/src/packet_loop.rs:1423-1431`
* Rust item: `wake_up`

### C body
```c
{
    int ret = 0;

    if (thread_ctx->wake_up_defined) {
#ifdef _WINDOWS
        if (SetEvent(thread_ctx->wake_up_event) == 0) {
            DWORD err = WSAGetLastError();
            DBG_PRINTF("Set network event fails, error 0x%x", err);
            ret = (int)err;
        }
#else
        /* TODO: write to network pipe */
        ssize_t written = 0;
        if ((written = write(thread_ctx->wake_up_pipe_fd[1], &ret, 1)) != 1) {
            if (written == 0) {
                ret = EPIPE;
            }
            else {
                ret = errno;
            }
        }
#endif
    }
    else {
        DBG_PRINTF("%s", "Wake up event not defined.");
        ret = -1;
    }
    return ret;
}
```

### Rust body
```rust
    pub fn wake_up(&mut self) -> Result<(), OsError> {
        if !self.wake_up_defined {
            return Err(OsError(-1));
        }
        let Some(sender) = self.wake_up_sender.as_ref() else {
            return Err(OsError(-1));
        };
        sender.send(()).map_err(|_| OsError(32))
    }
```

## `picoquic/tls_api.c:picoquic_aead_decrypt_generic`
* Phase 4C status: `suspect`
* Phase 4C rationale: C returns SIZE_MAX when aead_ctx is NULL and otherwise returns the decrypt result directly; Rust returns an error for missing context and also performs an explicit output buffer size check before copying.
* C source: `picoquic/tls_api.c:2456-2471`
* C signature: `size_t picoquic_aead_decrypt_generic(uint8_t *, const uint8_t *, size_t, uint64_t, const uint8_t *, size_t, void *)`
* Rust source: `rs/fq/src/tls_api.rs:1269-1286`
* Rust item: `aead_decrypt_generic`

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

## `picoquic/tls_api.c:picoquic_find_retry_protection_context`
* Phase 4C status: `suspect`
* Phase 4C rationale: C only creates a context for version_index when the retry key is non-null and leaves other vector entries null; Rust fills every vector slot up to idx with created contexts.
* C source: `picoquic/tls_api.c:3121-3152`
* C signature: `void * picoquic_find_retry_protection_context(picoquic_quic_t *, int, int)`
* Rust source: `rs/fq/src/tls_api.rs:2047-2077`
* Rust item: `find_retry_protection_context`

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

## `picoquic/tls_api.c:picoquic_public_random_seed`
* Phase 4C status: `suspect`
* Phase 4C rationale: C seeds with the first word and separately assigns the obfuscator; Rust passes both values to a different helper.
* C source: `picoquic/tls_api.c:859-866`
* C signature: `void picoquic_public_random_seed(picoquic_quic_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1204-1210`
* Rust item: `picoquic_public_random_seed`

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

## `picoquic/tls_api.c:picoquic_set_cipher_suite`
* Phase 4C status: `suspect`
* Phase 4C rationale: C delegates to set the cipher suite in the TLS context using low-memory state; Rust only validates a fixed set of ids and has no visible context update.
* C source: `picoquic/tls_api.c:427-433`
* C signature: `int picoquic_set_cipher_suite(picoquic_quic_t *, int)`
* Rust source: `rs/fq/src/lib.rs:1556-1561`
* Rust item: `set_cipher_suite`

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
