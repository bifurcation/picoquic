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

## `picoquic/ticket_store.c:picoquic_get_stored_ticket`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C checks time_valid_until against current_time, while Rust only checks time_valid_until.ticks() > 0.
* C source: `picoquic/ticket_store.c:328-352`
* C signature: `picoquic_stored_ticket_t * picoquic_get_stored_ticket(picoquic_quic_t *, const char *, uint16_t, const char *, uint16_t, uint32_t, int, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:1483-1498`
* Rust item: `get_stored_ticket`

### C body
```c
{
    picoquic_stored_ticket_t* next = quic->p_first_ticket;
    uint64_t current_time = picoquic_get_tls_time(quic);

    while (next != NULL) {
        if (next->time_valid_until > current_time&&
            next->sni_length == sni_length &&
            next->alpn_length == alpn_length &&
            memcmp(next->sni, sni, sni_length) == 0 &&
            memcmp(next->alpn, alpn, alpn_length) == 0 &&
            (version == 0 || next->version == version) &&
            (!need_unused || !next->was_used)) {
            uint64_t stored_id = (next->ticket_length < 8) ? 0 : PICOPARSE_64(next->ticket);
            if (ticket_id == 0 || stored_id == ticket_id) {
                break;
            }
        }
        next = next->next_ticket;
    }

    return next;
}
```

### Rust body
```rust
        self.stored_tickets.iter_mut().find(|ticket| {
            ticket.time_valid_until.ticks() > 0
                && ticket.sni.as_deref() == sni
                && ticket.alpn.as_deref() == alpn
                && (version == 0 || ticket.version == version)
                && (!need_unused || !ticket.was_used)
                && (ticket_id == 0 || stored_ticket_id(ticket) == ticket_id)
        })
```

## `picoquic/timing.c:picoquic_validate_bdp_seed`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C performs several checks and may notify congestion control; Rust body only returns immediately.
* C source: `picoquic/timing.c:90-114`
* C signature: `void picoquic_validate_bdp_seed(picoquic_cnx_t *, picoquic_path_t *, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:9152-9162`
* Rust item: `validate_bdp_seed`

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

## `picoquic/tls_api.c:picoquic_get_aes128gcm_sha256`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C always requests the AES_128_GCM_SHA256 suite id, while Rust passes through a variable cipher_suite_id.
* C source: `picoquic/tls_api.c:709-713`
* C signature: `ptls_cipher_suite_t * picoquic_get_aes128gcm_sha256(int)`
* Rust source: `rs/fq/src/tls_api.rs:2574-2586`
* Rust item: `picoquic_get_aes128gcm_sha256`

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

## `picoquic/tls_api.c:picoquic_hash_update`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C updates a hash context with input; the Rust body shown is hash_finalize and finalizes/copies output instead.
* C source: `picoquic/tls_api.c:736-738`
* C signature: `void picoquic_hash_update(uint8_t *, size_t, void *)`
* Rust source: `rs/fq/src/tls_api.rs:615-641`
* Rust item: `hash_update`

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

## `picoquic/tls_api.c:picoquic_set_tls_root_certificates`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns an error unless a backend function exists and marks the cert store non-empty only on success; Rust ignores certs and always returns Ok.
* C source: `picoquic/tls_api.c:666-678`
* C signature: `int picoquic_set_tls_root_certificates(picoquic_quic_t *, ptls_iovec_t *, size_t)`
* Rust source: `rs/fq/src/lib.rs:1635-1638`
* Rust item: `set_tls_root_certificates`

### C body
```c
{
    int ret = -1;
    PICOQUIC_THREAD_CHECK(quic);

    if (picoquic_set_tls_root_certificates_fn != NULL) {
        if ((ret = picoquic_set_tls_root_certificates_fn(quic->tls_master_ctx, certs, count)) == 0){
            quic->is_cert_store_not_empty = 1;
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn set_tls_root_certificates(&mut self, _certs: Vec<Vec<u8>>) -> Result<(), Error> {
        // Delegates to TLS backend configuration; complex.
        Ok(())
    }
```
