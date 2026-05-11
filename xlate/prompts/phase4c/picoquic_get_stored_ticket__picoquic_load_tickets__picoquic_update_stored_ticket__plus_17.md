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

## Pair `picoquic/ticket_store.c:picoquic_get_stored_ticket`
C: `picoquic/ticket_store.c:328-352 picoquic_get_stored_ticket`
Rust: `rs/fq/src/internal.rs:1483-1498 get_stored_ticket`

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

## Pair `picoquic/ticket_store.c:picoquic_load_tickets`
C: `picoquic/ticket_store.c:433-498 picoquic_load_tickets`
Rust: `rs/fq/src/internal.rs:1552-1590 load_tickets`

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

## Pair `picoquic/ticket_store.c:picoquic_update_stored_ticket`
C: `picoquic/ticket_store.c:529-571 picoquic_update_stored_ticket`
Rust: `rs/fq/src/tls_api.rs:2654-2659 picoquic_update_stored_ticket`

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

## Pair `picoquic/timing.c:picoquic_update_path_rtt_one_way`
C: `picoquic/timing.c:122-172 picoquic_update_path_rtt_one_way`
Rust: `rs/fq/src/internal.rs:9262-9323 update_path_rtt_one_way`

### C body
```c
{
    if (time_stamp != 0) {
        /* If the phase is not yet known, it should be set. */
        if (cnx->phase_delay == INT64_MAX) {
            cnx->phase_delay = old_path->rtt_sample / 2;

            if (!cnx->client_mode) {
                cnx->phase_delay = -cnx->phase_delay;
            }
        }
        /* TODO: some check on the validity of the one way delay */
        int64_t time_stamp_local = time_stamp - ack_delay + cnx->start_time + cnx->phase_delay;
        int is_time_stamp_valid = 1;

        /* The computation may indicate that the "local" value of the ack-stamping time
        * was earlier than the send time of the packet, or later than the current time.
        * This cannot happen, of course, which means that either the ack delay or
        * the phase estimate is wrong.
        */
        if (time_stamp_local < 0 || (uint64_t)time_stamp_local < send_time) {
            int64_t min_phase = send_time - time_stamp + ack_delay - cnx->start_time;
            time_stamp_local = time_stamp - ack_delay + cnx->start_time + min_phase;
            if (time_stamp_local > 0 && (uint64_t)time_stamp_local <= current_time) {
                /* looks plausible -- the computed stamp is earlier than now. */
                cnx->phase_delay = min_phase;
            }
            else {
                is_time_stamp_valid = 0;
            }
        }
        else if ((uint64_t)time_stamp_local > current_time) {
            int64_t max_phase = current_time - time_stamp + ack_delay - cnx->start_time;
            time_stamp_local = time_stamp - ack_delay + cnx->start_time + max_phase;
            if (time_stamp_local > 0 && (uint64_t)time_stamp_local >= send_time) {
                /* looks plausible -- the computed stamp is earlier than now. */
                cnx->phase_delay = max_phase;
            }
            else {
                is_time_stamp_valid = 0;
            }
        }
        if (is_time_stamp_valid) {
            old_path->one_way_delay_sample = time_stamp_local - send_time;
        }
        else {
            old_path->nb_delay_outliers++;
        }
    }
}
```

### Rust body
```rust
    ) {
        if time_stamp == 0 {
            return;
        }
        if self.phase_delay == i64::MAX {
            self.phase_delay = (old_path.rtt_sample.ticks() / 2) as i64;
            if !self.client_mode {
                self.phase_delay = -self.phase_delay;
            }
        }
        let mut time_stamp_local: i64 = (time_stamp as i64)
            .wrapping_sub(ack_delay as i64)
            .wrapping_add(self.start_time.ticks() as i64)
            .wrapping_add(self.phase_delay);
        let is_valid: bool;
        if time_stamp_local < 0 || (time_stamp_local as u64) < send_time.ticks() {
            let min_phase: i64 = (send_time.ticks() as i64)
                .wrapping_sub(time_stamp as i64)
                .wrapping_add(ack_delay as i64)
                .wrapping_sub(self.start_time.ticks() as i64);
            time_stamp_local = (time_stamp as i64)
                .wrapping_sub(ack_delay as i64)
                .wrapping_add(self.start_time.ticks() as i64)
                .wrapping_add(min_phase);
            if time_stamp_local > 0 && (time_stamp_local as u64) <= current_time.ticks() {
                self.phase_delay = min_phase;
                is_valid = true;
            } else {
                is_valid = false;
            }
        } else if (time_stamp_local as u64) > current_time.ticks() {
            let max_phase: i64 = (current_time.ticks() as i64)
                .wrapping_sub(time_stamp as i64)
                .wrapping_add(ack_delay as i64)
                .wrapping_sub(self.start_time.ticks() as i64);
            time_stamp_local = (time_stamp as i64)
                .wrapping_sub(ack_delay as i64)
                .wrapping_add(self.start_time.ticks() as i64)
                .wrapping_add(max_phase);
            if time_stamp_local > 0 && (time_stamp_local as u64) >= send_time.ticks() {
                self.phase_delay = max_phase;
                is_valid = true;
            } else {
                is_valid = false;
            }
        } else {
            is_valid = true;
        }
        if is_valid {
            old_path.one_way_delay_sample =
                Duration::from_ticks((time_stamp_local as u64).saturating_sub(send_time.ticks()));
        } else {
            old_path.nb_delay_outliers += 1;
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_api_init`
C: `picoquic/tls_api.c:229-236 picoquic_tls_api_init`
Rust: `rs/fq/src/lib.rs:4779-4781 tls_api_init`

### C body
```c
{
    if (!tls_api_is_init) {
        picoquic_tls_api_zero();
        picoquic_tls_api_init_providers(0);
        tls_api_is_init = 1;
    }
}
```

### Rust body
```rust
pub fn tls_api_init() {
    crate::tls_api::tls_api_init();
}
```

## Pair `picoquic/tls_api.c:picoquic_register_key_exchange_algorithm`
C: `picoquic/tls_api.c:276-292 picoquic_register_key_exchange_algorithm`
Rust: `rs/fq/src/tls_api.rs:2356-2362 register_key_exchange_algorithm`

### C body
```c
{
    for (int i = 0; i < PICOQUIC_KEY_EXCHANGES_NB_MAX; i++) {
        if (picoquic_key_exchanges[i] == NULL ||
            picoquic_key_exchanges[i]->id == key_exchange->id) {
            /* Replace the lower priority provider if present! */
            picoquic_key_exchanges[i] = key_exchange;
            break;
        }
    }

    if (key_exchange->id == PICOQUIC_GROUP_SECP256R1) {
        /* Replace the lower priority provider if present! */
        picoquic_key_exchange_secp256r1[0] = key_exchange;
    }
}
```

### Rust body
```rust
            if slot.provider.is_none() || slot.id == key_exchange_id {
                slot.id = key_exchange_id;
                slot.provider = Some(provider);
                break;
            }
```

## Pair `picoquic/tls_api.c:picoquic_get_cipher_suite_by_id`
C: `picoquic/tls_api.c:435-449 picoquic_get_cipher_suite_by_id`
Rust: `rs/fq/src/tls_api.rs:2546-2565 picoquic_get_cipher_suite_by_id`

### C body
```c
{
    ptls_cipher_suite_t* selected_suites[4];
    ptls_cipher_suite_t* cipher;
    int nb_suites = picoquic_set_cipher_suite_list(selected_suites, cipher_suite_id, use_low_memory);
    if (nb_suites <= 0) {
        cipher = NULL;
    }
    else {
        cipher = selected_suites[0];
    }

    return cipher;
}
```

### Rust body
```rust
pub fn picoquic_get_cipher_suite_by_id(cipher_suite_id: i32, use_low_memory: bool) -> Option<u16> {
    let state = tls_api_state();
    for slot in state.cipher_suites {
        if slot.high_memory_suite.is_none() {
            break;
        }
        if cipher_suite_id != 0 && cipher_suite_id != i32::from(slot.id) {
            continue;
        }
        let provider = if use_low_memory {
            slot.low_memory_suite
        } else {
            slot.high_memory_suite
        };
        if provider.is_some() {
            return Some(slot.id);
        }
    }
    None
}
```

## Pair `picoquic/tls_api.c:picoquic_get_hash_algorithm_by_name`
C: `picoquic/tls_api.c:493-508 picoquic_get_hash_algorithm_by_name`
Rust: `rs/fq/src/tls_api.rs:1980-2003 get_hash_algorithm_by_name`

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

## Pair `picoquic/tls_api.c:picoquic_set_key_exchange`
C: `picoquic/tls_api.c:573-582 picoquic_set_key_exchange`
Rust: `rs/fq/src/lib.rs:1564-1569 set_key_exchange`

### C body
```c
{
    int ret = 0;
    ptls_context_t* ctx;
    PICOQUIC_THREAD_CHECK(quic);
    ctx = (ptls_context_t*)quic->tls_master_ctx;

    ret = picoquic_set_key_exchange_in_ctx(ctx, key_exchange_id);
    return ret;
}
```

### Rust body
```rust
    pub fn set_key_exchange(&mut self, key_exchange_id: u16) -> Result<(), Error> {
        match key_exchange_id {
            0 | GROUP_SECP256R1 => Ok(()),
            _ => Err(Error::InvalidArgument),
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_set_tls_root_certificates`
C: `picoquic/tls_api.c:666-678 picoquic_set_tls_root_certificates`
Rust: `rs/fq/src/lib.rs:1635-1638 set_tls_root_certificates`

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

## Pair `picoquic/tls_api.c:picoquic_get_aes128gcm_sha256_v`
C: `picoquic/tls_api.c:715-718 picoquic_get_aes128gcm_sha256_v`
Rust: `rs/fq/src/lib.rs:4806-4814 is_minicrypto_aes128gcm_sha256`

### C body
```c
{
    return (void*)picoquic_get_aes128gcm_sha256(use_low_memory);
}
```

### Rust body
```rust
pub fn is_minicrypto_key_loader() -> bool {
    crate::tls_api::is_minicrypto_key_loader()
}
```

## Pair `picoquic/tls_api.c:picoquic_hash_finalize`
C: `picoquic/tls_api.c:740-742 picoquic_hash_finalize`
Rust: `rs/fq/src/tls_api.rs:630-641 hash_finalize`

### C body
```c
void picoquic_hash_finalize(uint8_t* output, void* hash_context) {
    ((ptls_hash_context_t*)hash_context)->final((ptls_hash_context_t*)hash_context, output, PTLS_HASH_FINAL_MODE_FREE);
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

## Pair `picoquic/tls_api.c:picoquic_public_random_64`
C: `picoquic/tls_api.c:833-839 picoquic_public_random_64`
Rust: `rs/fq/src/lib.rs:5156-5163 public_random_64`

### C body
```c
{
    uint64_t s1 = picoquic_public_random_step();
    s1 *= public_random_multiplier;
    s1 ^= public_random_obfuscator;
    return s1;
}
```

### Rust body
```rust
pub(crate) fn public_random_64() -> u64 {
    const PUBLIC_RANDOM_MULTIPLIER: u64 = 1_181_783_497_276_652_981;

    let mut state = PUBLIC_RANDOM_STATE
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    state.step().wrapping_mul(PUBLIC_RANDOM_MULTIPLIER) ^ state.obfuscator
}
```

## Pair `picoquic/tls_api.c:picoquic_public_uniform_random`
C: `picoquic/tls_api.c:882-892 picoquic_public_uniform_random`
Rust: `rs/fq/src/internal.rs:226-237 public_uniform_random`

### C body
```c
{
    uint64_t rnd;
    uint64_t rnd_min = UINT64_MAX % rnd_max;

    do {
        rnd = picoquic_public_random_64();
    } while (rnd < rnd_min);

    return rnd % rnd_max;
}
```

### Rust body
```rust
fn public_uniform_random(rnd_max: u64) -> u64 {
    if rnd_max == 0 {
        return 0;
    }
    let rnd_min = u64::MAX % rnd_max;
    loop {
        let rnd = crate::public_random_64();
        if rnd >= rnd_min {
            return rnd % rnd_max;
        }
    }
}
```

## Pair `picoquic/tls_api.c:picoquic_set_key_from_secret`
C: `picoquic/tls_api.c:1342-1361 picoquic_set_key_from_secret`
Rust: `rs/fq/src/tls_api.rs:550-570 picoquic_set_key_from_secret`

### C body
```c
{
    int ret = 0;

    if (is_enc != 0) {
        ret = picoquic_set_aead_from_secret(&ctx->aead_encrypt, cipher, is_enc, secret, prefix_label);
        
        if (ret == 0 && !is_rotation) {
            ret = picoquic_set_pn_enc_from_secret(&ctx->pn_enc, cipher, is_enc, secret, prefix_label);
        }
    } else {
        ret = picoquic_set_aead_from_secret(&ctx->aead_decrypt, cipher, is_enc, secret, prefix_label);
        
        if (ret == 0 && !is_rotation) {
            ret = picoquic_set_pn_enc_from_secret(&ctx->pn_dec, cipher, is_enc, secret, prefix_label);
        }
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    if is_enc {
        set_aead_from_secret(&mut ctx.aead_encrypt, suite, is_enc, secret, prefix_label)?;
        if !is_rotation {
            set_pn_enc_from_secret(&mut ctx.pn_enc, suite, is_enc, secret, prefix_label)?;
        }
    } else {
        set_aead_from_secret(&mut ctx.aead_decrypt, suite, is_enc, secret, prefix_label)?;
        if !is_rotation {
            set_pn_enc_from_secret(&mut ctx.pn_dec, suite, is_enc, secret, prefix_label)?;
        }
    }
    Ok(())
}
```

## Pair `picoquic/tls_api.c:picoquic_setup_initial_traffic_keys`
C: `picoquic/tls_api.c:1500-1529 picoquic_setup_initial_traffic_keys`
Rust: `rs/fq/src/tls_api.rs:1400-1430 setup_initial_traffic_keys`

### C body
```c
{
    int ret = 0;
    const char *prefix_label = picoquic_supported_versions[cnx->version_index].tls_prefix_label;
    ptls_cipher_suite_t* cipher = NULL;
    uint8_t client_secret[256];
    uint8_t server_secret[256];
    uint8_t *secret1, *secret2;

    ret = picoquic_compute_initial_secrets(cnx->quic, cnx->version_index, &cnx->initial_cnxid, &cipher, client_secret, server_secret);

    /* derive the initial keys */
    if (ret == 0) {
        if (!cnx->client_mode) {
            secret1 = server_secret;
            secret2 = client_secret;
        }
        else {
            secret1 = client_secret;
            secret2 = server_secret;
        }
        ret = picoquic_set_key_from_secret(cipher, 1, 0, &cnx->crypto_context[0], secret1, prefix_label);

        if (ret == 0) {
            ret = picoquic_set_key_from_secret(cipher, 0, 0, &cnx->crypto_context[0], secret2, prefix_label);
        }
    }

    return ret;
}
```

### Rust body
```rust
    pub fn setup_initial_traffic_keys(&mut self) -> Result<(), Error> {
        let (suite, client_secret, server_secret) = {
            let quic = self.quic_ref().ok_or(Error::InvalidState)?;
            picoquic_compute_initial_secrets(quic, self.version_index, &self.initial_connection_id)?
        };
        let params = version_from_index(self.version_index)
            .unwrap_or_else(|| connection_version(self))
            .parameters();
        let (local, remote) = if self.client_mode {
            (&client_secret[..], &server_secret[..])
        } else {
            (&server_secret[..], &client_secret[..])
        };
        picoquic_set_key_from_secret(
            suite,
            true,
            false,
            &mut self.crypto_context[0],
            local,
            params.tls_prefix_label,
        )?;
        picoquic_set_key_from_secret(
            suite,
            false,
            false,
            &mut self.crypto_context[0],
            remote,
            params.tls_prefix_label,
        )?;
        Ok(())
    }
```

## Pair `picoquic/tls_api.c:picoquic_get_app_secret_size`
C: `picoquic/tls_api.c:1599-1606 picoquic_get_app_secret_size`
Rust: `rs/fq/src/tls_api.rs:1503-1547 app_secret_size`

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

## Pair `picoquic/tls_api.c:picoquic_master_tlscontext`
C: `picoquic/tls_api.c:1725-1861 picoquic_master_tlscontext`
Rust: `rs/fq/src/tls_api.rs:932-952 init_master_tls_context`

### C body
```c
{
    /* Create a client context or a server context */
    int ret = 0;
    ptls_context_t* ctx;
    ptls_on_client_hello_t* och = NULL;
    ptls_encrypt_ticket_t* encrypt_ticket = NULL;
    ptls_save_ticket_t* save_ticket = NULL;
    unsigned int is_cert_store_not_empty = 0;

    picoquic_tls_api_init(); /* For example, init openSSL if in use. */

    ctx = (ptls_context_t*)malloc(sizeof(ptls_context_t));

    if (ctx == NULL) {
        ret = -1;
    }
    else {
        memset(ctx, 0, sizeof(ptls_context_t));
        picoquic_set_random_provider_in_ctx(ctx);
        
        ret = picoquic_set_key_exchange_in_ctx(ctx, 0); /* was: ctx->key_exchanges = picoquic_key_exchanges; */

        if (ret == 0) {
            ret = picoquic_set_cipher_suite_in_ctx(ctx, 0, quic->use_low_memory); /* was: ptls_openssl_cipher_suites; */
        }

        if (ret == 0) {
            ctx->send_change_cipher_spec = 0;

            ctx->hkdf_label_prefix__obsolete = NULL;
            ctx->update_traffic_key = picoquic_set_update_traffic_key_callback();

            if (quic->p_simulated_time == NULL) {
                ctx->get_time = &ptls_get_time;
            }
            else {
                ptls_get_time_t* time_getter = (ptls_get_time_t*)malloc(sizeof(ptls_get_time_t) + sizeof(uint64_t*));
                if (time_getter == NULL) {
                    ret = PICOQUIC_ERROR_MEMORY;
                }
                else {
                    uint64_t** pp_simulated_time = (uint64_t**)(((char*)time_getter) + sizeof(ptls_get_time_t));

                    time_getter->cb = picoquic_get_simulated_time_cb;
                    *pp_simulated_time = quic->p_simulated_time;
                    ctx->get_time = time_getter;
                }
            }

            if (cert_file_name != NULL && key_file_name != NULL) {
                /* Read the certificate file */
                if (ptls_load_certificates(ctx, (char*)cert_file_name) != 0) {
                    DBG_PRINTF("Cannot load certificate: %s", cert_file_name);
                    ret = -1;
                }
                else {
                    ret = set_private_key_from_file(key_file_name, ctx);
                    if (ret != 0){
                        DBG_PRINTF("Cannot load key: %s, ret = 0x%x", key_file_name, ret);
                    }
                }
            }
        }

        if (ret == 0) {
            och = (ptls_on_client_hello_t*)malloc(sizeof(ptls_on_client_hello_t) + sizeof(picoquic_quic_t*));
            if (och != NULL) {
                picoquic_quic_t** ppquic = (picoquic_quic_t**)(((char*)och) + sizeof(ptls_on_client_hello_t));

                och->cb = picoquic_client_hello_call_back;
                ctx->on_client_hello = och;
                *ppquic = quic;
            } else {
                ret = PICOQUIC_ERROR_MEMORY;
            }
        }

        if (ret == 0) {
            ret = picoquic_server_setup_ticket_aead_contexts(quic, ctx, ticket_key, ticket_key_length);
        }

        if (ret == 0) {
            encrypt_ticket = (ptls_encrypt_ticket_t*)malloc(sizeof(ptls_encrypt_ticket_t) + sizeof(picoquic_quic_t*));
            if (encrypt_ticket == NULL) {
                ret = PICOQUIC_ERROR_MEMORY;
            } else {
                picoquic_quic_t** ppquic = (picoquic_quic_t**)(((char*)encrypt_ticket) + sizeof(ptls_encrypt_ticket_t));

                encrypt_ticket->cb = picoquic_server_encrypt_ticket_call_back;
                *ppquic = quic;

                ctx->encrypt_ticket = encrypt_ticket;
                ctx->ticket_lifetime = 100000; /* 100,000 seconds, a bit more than one day */
                ctx->require_dhe_on_psk = 1;
                ctx->max_early_data_size = 0xFFFFFFFF;
            }
        }

        if (ret == 0) {
            ctx->verify_certificate = picoquic_get_certificate_verifier(cert_root_file_name,
                &is_cert_store_not_empty, (picoquic_free_verify_certificate_ctx*)
                &quic->free_verify_certificate_callback_fn);
            quic->is_cert_store_not_empty = is_cert_store_not_empty;
        }

        if (ret == 0 && quic->ticket_file_name != NULL) {
            save_ticket = (ptls_save_ticket_t*)malloc(sizeof(ptls_save_ticket_t) + sizeof(picoquic_quic_t*));
            if (save_ticket != NULL) {
                picoquic_quic_t** ppquic = (picoquic_quic_t**)(((char*)save_ticket) + sizeof(ptls_save_ticket_t));

                save_ticket->cb = picoquic_client_save_ticket_call_back;
                ctx->save_ticket = save_ticket;
                *ppquic = quic;
            }
        }

        if (ret == 0) {
            /* Tell Picotls to not require EOED messages during handshake */
            ctx->omit_end_of_early_data = 1;
        }

        if (ret == 0) {
            quic->tls_master_ctx = ctx;
            picoquic_public_random_seed(quic);
        } else {
            quic->tls_master_ctx = ctx;
            picoquic_master_tlscontext_free(quic);
            quic->tls_master_ctx = NULL;
            free(ctx);
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        if let (Some(cert), Some(key)) = (cert_file_name, key_file_name) {
            get_certs_from_file(cert).ok_or(Error::InvalidFile)?;
            self.set_private_key_from_file(key)?;
        }

        if let Some(root) = cert_root_file_name {
            self.is_cert_store_not_empty = get_certs_from_file(root).is_some();
            if !self.is_cert_store_not_empty {
                return Err(Error::InvalidFile);
            }
        }

        install_ticket_aead_contexts(self, ticket_key)
    }
```

## Pair `picoquic/tls_api.c:picoquic_set_key_log_file`
C: `picoquic/tls_api.c:2040-2070 picoquic_set_key_log_file`
Rust: `rs/fq/src/lib.rs:1129-1144 set_key_log_file`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;
    struct st_picoquic_log_event_t* log_event = (struct st_picoquic_log_event_t*)ctx->log_event;

    if (log_event == NULL) {
        log_event = (struct st_picoquic_log_event_t*)malloc(sizeof(struct st_picoquic_log_event_t));
        if (log_event != NULL) {
            log_event->super.cb = picoquic_log_event_call_back;
        }
    }
    else {
        if (log_event->fp != NULL) {
            picoquic_file_close(log_event->fp);
            log_event->fp = NULL;
        }
    }

    if (log_event != NULL) {
        log_event->fp = picoquic_file_open(keylog_filename, "a");
        log_event->super.cb = picoquic_log_event_call_back;
        ctx->log_event = (ptls_log_event_t*)log_event;
    }

    ctx->log_event = (ptls_log_event_t*)log_event;
}
```

### Rust body
```rust
    pub fn set_key_log_file(&mut self, keylog_filename: Option<&str>) {
        // Open or clear the SSL keylog file.  When a path is given, open it
        // for appending; errors are silently ignored (matching the C behaviour
        // of falling back to no logging rather than crashing).
        use std::fs::OpenOptions;
        match keylog_filename {
            Some(path) => {
                if let Ok(f) = OpenOptions::new().create(true).append(true).open(path) {
                    self.f_log = Some(Box::new(f));
                }
            }
            None => {
                self.f_log = None;
            }
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_get_sni`
C: `picoquic/tls_api.c:2144-2151 picoquic_tls_get_sni`
Rust: `rs/fq/src/tls_api.rs:1156-1211 tls_get_sni`

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
