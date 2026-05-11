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

## `picoquic/ticket_store.c:picoquic_update_stored_ticket`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only extracts peer_ip and returns; C searches for a matching stored ticket and updates IP address and 0-RTT path metrics.
* C source: `picoquic/ticket_store.c:529-571`
* C signature: `void picoquic_update_stored_ticket(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/tls_api.rs:2654-2659`
* Rust item: `picoquic_update_stored_ticket`

### C body
```c
{
    char const* sni = (cnx->sni == NULL) ? "" : cnx->sni;
    size_t sni_length = strlen(sni);
    char const* alpn = (cnx->alpn == NULL) ? "" : cnx->alpn;
    size_t alpn_length = strlen(alpn);
    uint8_t* ip_addr;
    uint8_t ip_addr_length;
    uint32_t version = picoquic_supported_versions[cnx->version_index].version;

    picoquic_get_ip_addr((struct sockaddr *)&path_x->first_tuple->peer_addr, &ip_addr, &ip_addr_length);

    if (ip_addr != NULL && ip_addr_length <= PICOQUIC_STORED_IP_MAX) {
        picoquic_stored_ticket_t* next = picoquic_get_stored_ticket(
            cnx->quic, sni, (uint16_t)sni_length,
            alpn, (uint16_t)alpn_length, version, 0, cnx->issued_ticket_id);
        while (next != NULL) {
            if (next->sni_length == sni_length &&
                next->alpn_length == alpn_length &&
                memcmp(next->sni, sni, sni_length) == 0 &&
                memcmp(next->alpn, alpn, alpn_length) == 0 &&
                next->version == version) {
                uint64_t ticket_id = (next->ticket_length < 8) ? 0 : PICOPARSE_64(next->ticket);
                if (cnx->issued_ticket_id == 0 || cnx->issued_ticket_id == ticket_id) {
                    break;
                }
            }
            else {
                next = next->next_ticket;
            }
        }
        if (next != NULL) {
            next->ip_addr_length = ip_addr_length;
            memcpy(next->ip_addr, ip_addr, ip_addr_length);
            next->tp_0rtt[picoquic_tp_0rtt_rtt_local] = path_x->rtt_min;
            next->tp_0rtt[picoquic_tp_0rtt_cwin_local] = path_x->cwin;
            next->tp_0rtt[picoquic_tp_0rtt_rtt_remote] = path_x->rtt_min_remote;
            next->tp_0rtt[picoquic_tp_0rtt_cwin_remote] = path_x->cwin_remote;
            next->ip_addr_client_length = path_x->ip_client_remote_length;
            memcpy(next->ip_addr_client, path_x->ip_client_remote, path_x->ip_client_remote_length);
        }
    }
}
```

### Rust body
```rust
        let Some(peer_ip) = path_x.tuples.first().map(|tuple| tuple.peer_addr.ip()) else {
            return;
        };
```

## `picoquic/tls_api.c:picoquic_explain_crypto_error`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C calls an optional explain function and returns an int; Rust maps an error object into CryptoError fields.
* C source: `picoquic/tls_api.c:679-689`
* C signature: `int picoquic_explain_crypto_error(const char **, int *)`
* Rust source: `rs/fq/src/sys/openssl.rs:451-461`
* Rust item: `explain_crypto_error`

### C body
```c
{
    int ret = 0;
    if (picoquic_explain_crypto_error_fn != NULL) {
        ret = picoquic_explain_crypto_error_fn(err_file, err_line);
    }
    return ret;
}
```

### Rust body
```rust
        .map(|e| CryptoError {
            code: e.code(),
            reason: e.reason().map(str::to_owned),
            library: e.library().map(str::to_owned),
            file: e.file().to_owned(),
            line: e.line(),
        })
```

## `picoquic/tls_api.c:picoquic_get_sha256`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the hash algorithm named sha256; Rust body is a digest_size method matching existing enum variants.
* C source: `picoquic/tls_api.c:510-515`
* C signature: `ptls_hash_algorithm_t * picoquic_get_sha256(void)`
* Rust source: `rs/fq/src/tls_api.rs:592-610`
* Rust item: `new`

### C body
```c
{
    return picoquic_get_hash_algorithm_by_name("sha256");
}
```

### Rust body
```rust
    pub fn digest_size(&self) -> usize {
        match self {
            HashContext::Sha256(_) => 32,
            HashContext::Sha384(_) => 48,
            HashContext::Sha512(_) => 64,
        }
    }
```

## `picoquic/tls_api.c:picoquic_set_private_key_from_file`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust only reads and validates file text and never passes the file into the TLS master context helper as C does.
* C source: `picoquic/tls_api.c:611-614`
* C signature: `int picoquic_set_private_key_from_file(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/tls_api.rs:1995-2002`
* Rust item: `set_private_key_from_file`

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

## `picoquic/tls_api.c:picoquic_tls_get_sni`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body contains unrelated time, random, uniform-random, and public-random-seed methods, not an SNI getter.
* C source: `picoquic/tls_api.c:2144-2151`
* C signature: `const char * picoquic_tls_get_sni(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1156-1211`
* Rust item: `tls_get_sni`

### C body
```c
{
    picoquic_tls_ctx_t* ctx;
    PICOQUIC_THREAD_CHECK(cnx->quic);
    
    ctx = (picoquic_tls_ctx_t*)cnx->tls_ctx;
    return ptls_get_server_name(ctx->tls);
}
```

### Rust body
```rust
impl Quic {
    /// Read the virtual time tls sees through its `get_time`
    /// callback (microseconds).  C: `get_tls_time`.
    ///
    /// The C body asks the TLS context for milliseconds and converts
    /// them back to microseconds.  Rust keeps QUIC time in the context
    /// helper, so this returns the same microsecond value exposed by
    /// [`Quic::time`].
    pub fn tls_time(&self) -> u64 {
        self.time()
    }

    /// Fill `buf` with cryptographically secure random bytes.
    /// The C implementation called `ctx->random_bytes()` on the picotls
    /// master context; in Rust we delegate to [`Quic::rng`], a
    /// `Box<dyn CryptoRng>` installed at context creation.
    /// C: `picoquic_crypto_random`
    pub fn crypto_random(&mut self, buf: &mut [u8]) {
        use rand_core::RngCore;
        self.rng.fill_bytes(buf);
    }

    /// Return a cryptographically random value in `0..rnd_max`, using the
    /// same rejection-sampling threshold as the C implementation.
    ///
    /// C: `picoquic/tls_api.c:picoquic_crypto_uniform_random`.
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

    /// Seed the public non-cryptographic random generator from this context's
    /// cryptographic RNG.
    ///
    /// C: `picoquic/tls_api.c:picoquic_public_random_seed`.
    pub fn picoquic_public_random_seed(&mut self) {
        let mut bytes = [0u8; 24];
        self.crypto_random(&mut bytes);
        let seed = u64::from_ne_bytes(bytes[0..8].try_into().unwrap());
        let obfuscator = u64::from_ne_bytes(bytes[8..16].try_into().unwrap());
        crate::public_random_seed_from_crypto(seed, obfuscator);
    }
}
```
