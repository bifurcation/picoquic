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

## `picoquic/ticket_store.c:picoquic_seed_ticket`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is only a partial expression returning path_x.cwin in an else arm, missing the client/server branching, ticket update/remembering, and is_ticket_seeded assignment visible in C.
* C source: `picoquic/ticket_store.c:573-591`
* C signature: `void picoquic_seed_ticket(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:1624-1633`
* Rust item: `seed_ticket`

### C body
```c
{
    if (cnx->client_mode) {
        picoquic_update_stored_ticket(cnx, path_x);
    }
    else {
        uint8_t* ip_addr;
        uint8_t ip_addr_length;
        uint64_t target_cwin = path_x->cwin;

        if (path_x->bandwidth_estimate_max > 0) {
            target_cwin = PICOQUIC_BYTES_FROM_RATE(path_x->rtt_min, path_x->bandwidth_estimate_max);
        }
        picoquic_get_ip_addr((struct sockaddr*) & path_x->first_tuple->peer_addr, &ip_addr, &ip_addr_length);
        (void) picoquic_remember_issued_ticket(cnx->quic, cnx->issued_ticket_id,
            path_x->rtt_min, target_cwin, ip_addr, ip_addr_length);
    }
    path_x->is_ticket_seeded = 1;
}
```

### Rust body
```rust
        } else {
            path_x.cwin
        };
```

## `picoquic/tls_api.c:picoquic_encode_retry_protection`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C formats a retry pseudo-packet and appends an AEAD encryption tag when space permits; Rust body only checks byte_index/tag length and returns byte_index, with no pseudo-packet formatting or encryption visible.
* C source: `picoquic/tls_api.c:3188-3199`
* C signature: `size_t picoquic_encode_retry_protection(void *, uint8_t *, size_t, size_t, const picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/tls_api.rs:2090-2098`
* Rust item: `encode_retry_protection`

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

## `picoquic/tls_api.c:picoquic_get_hash_algorithm_by_name`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a private-key file loader, not a hash-algorithm lookup by name.
* C source: `picoquic/tls_api.c:493-508`
* C signature: `ptls_hash_algorithm_t * picoquic_get_hash_algorithm_by_name(const char *)`
* Rust source: `rs/fq/src/tls_api.rs:1980-2003`
* Rust item: `get_hash_algorithm_by_name`

### C body
```c
{
    ptls_hash_algorithm_t* hash = NULL;

    for (int i = 0; i < PICOQUIC_CIPHER_SUITES_NB_MAX && hash == NULL; i++) {
        if (picoquic_cipher_suites[i].high_memory_suite == NULL) {
            break;
        }
        if (strcmp(picoquic_cipher_suites[i].high_memory_suite->hash->name, hash_algorithm_name) == 0) {
            hash = picoquic_cipher_suites[i].high_memory_suite->hash;
            break;
        }
    }
    return hash;
}
```

### Rust body
```rust
impl Quic {
    /// Load a PEM-encoded private key from `file_name` and install
    /// it in this context's master TLS context.  C:
    /// `set_private_key_from_file`.
    pub fn set_private_key_from_file(&mut self, file_name: &str) -> Result<(), Error> {
        let contents = std::fs::read_to_string(file_name).map_err(|_| Error::NoSuchFile)?;
        if contents.contains("-----BEGIN ") && contents.contains("PRIVATE KEY-----") {
            Ok(())
        } else {
            Err(Error::InvalidFile)
        }
    }
}
```

## `picoquic/tls_api.c:picoquic_set_pn_enc_from_secret`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust replaces the slot only after helper success, while C frees/nulls the old slot before HKDF and creates a cipher using is_enc.
* C source: `picoquic/tls_api.c:1311-1330`
* C signature: `int picoquic_set_pn_enc_from_secret(void **, ptls_cipher_suite_t *, int, const void *, const char *)`
* Rust source: `rs/fq/src/tls_api.rs:535-544`
* Rust item: `set_pn_enc_from_secret`

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

## `picoquic/tls_api.c:picoquic_tls_get_negotiated_alpn`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the negotiated ALPN from the TLS object; Rust body is tls_get_sni and returns SNI instead.
* C source: `picoquic/tls_api.c:2135-2142`
* C signature: `const char * picoquic_tls_get_negotiated_alpn(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1148-1158`
* Rust item: `tls_get_negotiated_alpn`

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
