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

## `picoquic/ticket_store.c:picoquic_deserialize_ticket`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C parses one ticket prefix and reports consumed length, allowing trailing bytes; Rust rejects any trailing bytes with off != bytes.len().
* C source: `picoquic/ticket_store.c:160-257`
* C signature: `int picoquic_deserialize_ticket(picoquic_stored_ticket_t **, uint8_t *, size_t, size_t *)`
* Rust source: `rs/fq/src/internal.rs:1393-1433`
* Rust item: `deserialize_ticket`

### C body
```c
{
    int ret = 0;
    uint64_t time_valid_until = 0;
    size_t required_length = 8 + 2 + 2 + 4 + 1 + 1 + PICOQUIC_NB_TP_0RTT * 8 + 2;
    size_t byte_index = 0;
    size_t sni_index = 0;
    size_t alpn_index = 0;
    size_t ip_addr_index = 0;
    size_t ip_addr_client_index = 0;
    size_t ticket_index = 0;
    uint16_t sni_length = 0;
    uint16_t alpn_length = 0;
    uint32_t version = 0;
    uint16_t ticket_length = 0;
    uint8_t ip_addr_length = 0;
    uint8_t ip_addr_client_length = 0;
    uint64_t tp_0rtt[PICOQUIC_NB_TP_0RTT] = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 };

    *consumed = 0;
    *ticket = NULL;

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
        alpn_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        alpn_index = byte_index;
        required_length += alpn_length;
        byte_index += alpn_length;
    }

    if (required_length < bytes_max) {
        version = PICOPARSE_32(bytes + byte_index);
        byte_index += 4;
    }

    if (required_length < bytes_max) {
        ip_addr_length = bytes[byte_index++];
        ip_addr_index = byte_index;
        required_length += ip_addr_length;
        byte_index += ip_addr_length;
    }

    if (required_length < bytes_max) {
        ip_addr_client_length = bytes[byte_index++];
        ip_addr_client_index = byte_index;
        required_length += ip_addr_client_length;
        byte_index += ip_addr_client_length;
    }

    if (required_length < bytes_max) {
        for (int i = 0; i < PICOQUIC_NB_TP_0RTT; i++) {
            tp_0rtt[i] = PICOPARSE_64(bytes + byte_index);
            byte_index += 8;
        }
    }

    if (required_length < bytes_max) {
        ticket_length = PICOPARSE_16(bytes + byte_index);
        byte_index += 2;
        ticket_index = byte_index;
        required_length += ticket_length;
    }

    if (required_length > bytes_max) {
        *ticket = NULL;
        ret = PICOQUIC_ERROR_INVALID_TICKET;
    } else {
        *ticket = picoquic_format_ticket(time_valid_until,
            (const char *)(bytes + sni_index), sni_length,
            (const char *)(bytes + alpn_index), alpn_length,
            version,
            (const uint8_t*)(bytes + ip_addr_index), ip_addr_length,
            (const uint8_t*)(bytes + ip_addr_client_index), ip_addr_client_length,
            bytes + ticket_index, ticket_length,
            NULL);
        if (*ticket == NULL) {
            ret = PICOQUIC_ERROR_MEMORY;
        }
        else {
            for (int i=0; i< PICOQUIC_NB_TP_0RTT; i++) {
                (*ticket)->tp_0rtt[i] = tp_0rtt[i];
            }
            *consumed = required_length;
        }
    }

    return ret;
}
```

### Rust body
```rust
fn deserialize_ticket(bytes: &[u8]) -> Result<StoredTicket, crate::Error> {
    let mut off = 0;
    let time_valid_until = Instant::from_ticks(parse_64(take_slice(bytes, &mut off, 8)?));

    let sni_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let sni = optional_string_from_bytes(take_slice(bytes, &mut off, sni_len)?)?;

    let alpn_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let alpn = optional_string_from_bytes(take_slice(bytes, &mut off, alpn_len)?)?;

    let version = parse_32(take_slice(bytes, &mut off, 4)?);

    let ip_len = take_slice(bytes, &mut off, 1)?[0] as usize;
    let ip_addr = stored_ip_from_bytes(take_slice(bytes, &mut off, ip_len)?)?;

    let ip_client_len = take_slice(bytes, &mut off, 1)?[0] as usize;
    let ip_addr_client = stored_ip_from_bytes(take_slice(bytes, &mut off, ip_client_len)?)?;

    let mut tp_0rtt = [0u64; NB_TP_0RTT];
    for value in tp_0rtt.iter_mut() {
        *value = parse_64(take_slice(bytes, &mut off, 8)?);
    }

    let ticket_len = parse_16(take_slice(bytes, &mut off, 2)?) as usize;
    let ticket = take_slice(bytes, &mut off, ticket_len)?.to_vec();
    if off != bytes.len() {
        return Err(crate::Error::InvalidFile);
    }

    Ok(StoredTicket {
        sni,
        alpn,
        ip_addr,
        ip_addr_client,
        tp_0rtt,
        ticket,
        time_valid_until,
        version,
        was_used: false,
    })
}
```

## `picoquic/timing.c:picoquic_current_retransmit_timer`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust omits the C initial-max and large/satellite retransmit caps and adds a minimum clamp.
* C source: `picoquic/timing.c:42-88`
* C signature: `uint64_t picoquic_current_retransmit_timer(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:9111-9145`
* Rust item: `current_retransmit_timer`

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

## `picoquic/tls_api.c:picoquic_export_secret`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is for setting a spinbit policy and never checks TLS context or exports a secret.
* C source: `picoquic/tls_api.c:2225-2238`
* C signature: `int picoquic_export_secret(picoquic_cnx_t *, const char *, uint8_t *, size_t)`
* Rust source: `rs/fq/src/lib.rs:1768-1781`
* Rust item: `export_secret`

### C body
```c
{
    if (cnx == NULL || label == NULL || out == NULL || outlen == 0) {
        return -1;
    }
    PICOQUIC_THREAD_CHECK(cnx->quic);
    picoquic_tls_ctx_t *tls_ctx = (picoquic_tls_ctx_t *)cnx->tls_ctx;
    if (tls_ctx == NULL || tls_ctx->tls == NULL) {
        return PTLS_ERROR_IN_PROGRESS;
    }

    ptls_t* tls = tls_ctx->tls;
    return ptls_export_secret(tls, out, outlen, label, ptls_iovec_init(NULL, 0), 0);
}
```

### Rust body
```rust
    pub fn set_spinbit_policy(&mut self, spinbit_policy: SpinbitVersion) -> Result<(), Error> {
        // SpinbitVersion::On is server-only.
        if spinbit_policy == SpinbitVersion::On {
            return Err(Error::InvalidArgument);
        }
        self.spin_policy = spinbit_policy;
        Ok(())
    }
```

## `picoquic/tls_api.c:picoquic_hash_create`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a hash length lookup, not hash context creation.
* C source: `picoquic/tls_api.c:521-530`
* C signature: `void * picoquic_hash_create(const char *)`
* Rust source: `rs/fq/src/tls_api.rs:1953-1973`
* Rust item: `hash_create`

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

## `picoquic/tls_api.c:picoquic_set_tls_certificate_chain`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C frees the existing certificate list and stores the new certs/count; Rust body ignores the certs and contains only a placeholder-like comment.
* C source: `picoquic/tls_api.c:2804-2813`
* C signature: `void picoquic_set_tls_certificate_chain(picoquic_quic_t *, ptls_iovec_t *, size_t)`
* Rust source: `rs/fq/src/lib.rs:1627-1629`
* Rust item: `set_tls_certificate_chain`

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
