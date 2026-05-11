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

## `picoquic/ticket_store.c:picoquic_load_tickets`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C filters tickets using current_time, while Rust keeps any ticket with time_valid_until.ticks() > 0; Rust also clears stored_tickets before loading, which is not visible in the C body.
* C source: `picoquic/ticket_store.c:433-498`
* C signature: `int picoquic_load_tickets(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/internal.rs:1552-1590`
* Rust item: `load_tickets`

### C body
```c
{
    picoquic_stored_ticket_t** pp_first_ticket = &quic->p_first_ticket;
    uint64_t current_time = picoquic_get_tls_time(quic);
    int ret = 0;
    int file_err = 0;
    FILE* F = NULL;
    picoquic_stored_ticket_t* previous = NULL;
    picoquic_stored_ticket_t* next = NULL;
    uint32_t record_size;
    uint32_t storage_size;


    if ((F = picoquic_file_open_ex(ticket_file_name, "rb", &file_err)) == NULL) {
        ret = (file_err == ENOENT) ? PICOQUIC_ERROR_NO_SUCH_FILE : -1;
    }

    while (ret == 0) {
        if (fread(&storage_size, 4, 1, F) != 1) {
            /* end of file */
            break;
        }
        else if (storage_size > 2048 ||
            (record_size = storage_size + offsetof(struct st_picoquic_stored_ticket_t, time_valid_until)) > 2048) {
            ret = PICOQUIC_ERROR_INVALID_FILE;
            break;
        }
        else {
            uint8_t buffer[2048];
            if (fread(buffer, 1, storage_size, F)
                != storage_size) {
                ret = PICOQUIC_ERROR_INVALID_FILE;
            }
            else {
                size_t consumed = 0;
                ret = picoquic_deserialize_ticket(&next, buffer, storage_size, &consumed);

                if (ret == 0 && (consumed != storage_size || next == NULL)) {
                    ret = PICOQUIC_ERROR_INVALID_FILE;
                }

                if (ret == 0 && next != NULL) {
                    if (next->time_valid_until < current_time) {
                        free(next);
                        next = NULL;
                    }
                    else {
                        next->next_ticket = NULL;
                        if (previous == NULL) {
                            *pp_first_ticket = next;
                        }
                        else {
                            previous->next_ticket = next;
                        }

                        previous = next;
                    }
                }
            }
        }
    }

    picoquic_file_close(F);

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), crate::Error> {
        let data = match std::fs::read(ticket_file_name) {
            Ok(data) => data,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(crate::Error::NoSuchFile);
            }
            Err(_) => return Err(crate::Error::InvalidFile),
        };

        let mut off = 0;
        self.stored_tickets.clear();
        while off < data.len() {
            let record_len_bytes = data.get(off..off + 4).ok_or(crate::Error::InvalidFile)?;
            let record_len = u32::from_ne_bytes([
                record_len_bytes[0],
                record_len_bytes[1],
                record_len_bytes[2],
                record_len_bytes[3],
            ]) as usize;
            off += 4;
            if record_len > 2048 {
                return Err(crate::Error::InvalidFile);
            }
            let record = data
                .get(off..off + record_len)
                .ok_or(crate::Error::InvalidFile)?;
            off += record_len;

            let ticket = deserialize_ticket(record)?;
            if ticket.time_valid_until.ticks() > 0 {
                self.stored_tickets.push(ticket);
            }
        }

        Ok(())
    }
```

## `picoquic/tls_api.c:picoquic_create_retry_protection_context`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body is a one-line create_retry_protection_context wrapper around picoquic_setup_test_aead_context; Rust snippet instead contains find_retry_protection_context and delete_retry_protection_contexts, not the matching function body.
* C source: `picoquic/tls_api.c:3116-3119`
* C signature: `void * picoquic_create_retry_protection_context(int, uint8_t *, const char *)`
* Rust source: `rs/fq/src/tls_api.rs:2035-2085`
* Rust item: `create_retry_protection_context`

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

## `picoquic/tls_api.c:picoquic_get_aes128gcm_v`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns an AEAD pointer from an AES128-GCM cipher suite; Rust checks whether the key loader provider is Minicrypto.
* C source: `picoquic/tls_api.c:720-729`
* C signature: `void * picoquic_get_aes128gcm_v(int)`
* Rust source: `rs/fq/src/tls_api.rs:2594-2603`
* Rust item: `picoquic_get_aes128gcm_v`

### C body
```c
{
    void* aead = NULL;
    ptls_cipher_suite_t* cipher = picoquic_get_aes128gcm_sha256(use_low_memory);

    if (cipher != NULL) {
        aead = (void*)(cipher->aead);
    }
    return aead;
}
```

### Rust body
```rust
pub fn is_minicrypto_key_loader() -> bool {
    let state = tls_api_state();
    state.private_key_provider == Some(CryptoProvider::Minicrypto)
}
```

## `picoquic/tls_api.c:picoquic_is_tls_complete`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns TLS handshake completion status; Rust body initializes TLS stream, queues bytes, and transitions state.
* C source: `picoquic/tls_api.c:2760-2767`
* C signature: `int picoquic_is_tls_complete(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1120-1139`
* Rust item: `is_tls_complete`

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

## `picoquic/tls_api.c:picoquic_setup_test_aead_context`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C creates an AEAD context using picoquic_set_aead_from_secret and is_encrypt; Rust returns a HeaderKey from header_key_from_secret and ignores is_encrypt.
* C source: `picoquic/tls_api.c:2403-2412`
* C signature: `void * picoquic_setup_test_aead_context(int, const uint8_t *, const char *)`
* Rust source: `rs/fq/src/tls_api.rs:1625-1640`
* Rust item: `setup_test_aead_context`

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
