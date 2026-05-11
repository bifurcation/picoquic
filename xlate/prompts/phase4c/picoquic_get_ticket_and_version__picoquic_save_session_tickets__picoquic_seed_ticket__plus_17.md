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

## Pair `picoquic/ticket_store.c:picoquic_get_ticket_and_version`
C: `picoquic/ticket_store.c:354-383 picoquic_get_ticket_and_version`
Rust: `rs/fq/src/internal.rs:1520-1549 get_ticket_and_version`

### C body
```c
{
    int ret = 0;
    picoquic_stored_ticket_t* next = picoquic_get_stored_ticket(
        quic, sni, sni_length, alpn, alpn_length, version, mark_used, 0);

    if (next == NULL) {
        *ticket = NULL;
        *ticket_length = 0;
        ret = -1;
    } else {
        if (tp != NULL) {
            tp->initial_max_data = next->tp_0rtt[picoquic_tp_0rtt_max_data];
            tp->initial_max_stream_data_bidi_local = next->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_local];
            tp->initial_max_stream_data_bidi_remote = next->tp_0rtt[picoquic_tp_0rtt_max_stream_data_bidi_remote];
            tp->initial_max_stream_data_uni = next->tp_0rtt[picoquic_tp_0rtt_max_stream_data_uni];
            tp->initial_max_stream_id_bidir = next->tp_0rtt[picoquic_tp_0rtt_max_streams_id_bidir];
            tp->initial_max_stream_id_unidir = next->tp_0rtt[picoquic_tp_0rtt_max_streams_id_unidir];
            *ticket_version = next->version;
        }
        *ticket = next->ticket;
        *ticket_length = next->ticket_length;
        next->was_used = mark_used;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(u32, &[u8], TransportParameters), crate::Error> {
        let idx = self
            .stored_tickets
            .iter()
            .position(|ticket| {
                ticket.time_valid_until.ticks() > 0
                    && ticket.sni.as_deref() == sni
                    && ticket.alpn.as_deref() == alpn
                    && (version == 0 || ticket.version == version)
                    && (!mark_used || !ticket.was_used)
            })
            .ok_or(crate::Error::Generic)?;

        if mark_used {
            self.stored_tickets[idx].was_used = true;
        }

        let ticket = &self.stored_tickets[idx];
        Ok((
            ticket.version,
            ticket.ticket.as_slice(),
            stored_ticket_tp(ticket),
        ))
    }
```

## Pair `picoquic/ticket_store.c:picoquic_save_session_tickets`
C: `picoquic/ticket_store.c:511-515 picoquic_save_session_tickets`
Rust: `rs/fq/src/lib.rs:1819-1822 save_session_tickets`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return picoquic_save_tickets(quic->p_first_ticket, picoquic_get_tls_time(quic), ticket_store_filename);
}
```

### Rust body
```rust
    pub fn save_session_tickets(&mut self, _ticket_store_filename: &str) -> Result<(), Error> {
        // Complex: involves file I/O and ticket serialization.
        Ok(())
    }
```

## Pair `picoquic/ticket_store.c:picoquic_seed_ticket`
C: `picoquic/ticket_store.c:573-591 picoquic_seed_ticket`
Rust: `rs/fq/src/internal.rs:1624-1633 seed_ticket`

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

## Pair `picoquic/timing.c:picoquic_update_path_rtt`
C: `picoquic/timing.c:175-321 picoquic_update_path_rtt`
Rust: `rs/fq/src/internal.rs:9191-9255 update_path_rtt`

### C body
```c
{
    if (old_path != NULL && (!old_path->rtt_is_initialized || epoch >= 0)) {
        uint64_t rtt_estimate = 0;
        int is_first = !old_path->rtt_is_initialized;
        picoquic_packet_context_t* pkt_ctx = NULL;
        if (cnx->is_multipath_enabled) {
            pkt_ctx = &old_path->pkt_ctx;
        }
        else if (epoch == picoquic_epoch_1rtt || epoch == picoquic_epoch_0rtt) {
            pkt_ctx = &cnx->pkt_ctx[picoquic_packet_context_application];
        }

        if (current_time > send_time) {
            rtt_estimate = current_time - send_time;
            /* We cannot blindly trust the ack delay,
             * and especially not for the first sample */
            if (!is_first && ack_delay > 0 && cnx->cnx_state >= picoquic_state_ready) {
                if (ack_delay > cnx->local_parameters.max_ack_delay) {
                    ack_delay = cnx->local_parameters.max_ack_delay;
                }
                if (old_path->rtt_min + ack_delay < rtt_estimate) {
                    rtt_estimate -= ack_delay;
                }
            }
        }
        old_path->rtt_sample = rtt_estimate;
        /* During a measurement period, accumulate data:
        * - number of estimates since update
        * - sum of all estimates since update
        * - min estimate
        * - max estimate
        * If the PTO is lower than the max estimate, increase it.
        * If the min RTT is lower than the current low, lower it.
        */
        old_path->nb_rtt_estimate_in_period += 1;
        old_path->sum_rtt_estimate_in_period += rtt_estimate;
        if (old_path->nb_rtt_estimate_in_period == 1) {
            old_path->min_rtt_estimate_in_period = rtt_estimate;
            old_path->max_rtt_estimate_in_period = rtt_estimate;
        }
        else {
            if (rtt_estimate > old_path->max_rtt_estimate_in_period) {
                old_path->max_rtt_estimate_in_period = rtt_estimate;
            }
            if (rtt_estimate < old_path->min_rtt_estimate_in_period) {
                old_path->min_rtt_estimate_in_period = rtt_estimate;
            }
        }
        if (old_path->retransmit_timer < rtt_estimate) {
            old_path->retransmit_timer = rtt_estimate;
        }
        if (old_path->rtt_min > rtt_estimate) {
            old_path->rtt_min = rtt_estimate;
        }
        if (old_path->rtt_max < rtt_estimate) {
            old_path->rtt_max = rtt_estimate;
        }

        /* if one way delay measured, use it */
        if (time_stamp > 0) {
            picoquic_update_path_rtt_one_way(cnx, old_path, send_time, current_time, ack_delay, time_stamp);
        }
        /* At the end of the period, update the smoothed and variants statistics.
        */
        if (pkt_ctx == NULL || pkt_ctx->highest_acknowledged > old_path->rtt_packet_previous_period ||
            old_path->rtt_time_previous_period + (rtt_estimate / 4) > current_time) {
            old_path->rtt_time_previous_period = current_time;

            if (old_path->nb_rtt_estimate_in_period > 1) {
                rtt_estimate = old_path->sum_rtt_estimate_in_period / old_path->nb_rtt_estimate_in_period;
            }

            if (is_first) {
                old_path->smoothed_rtt = rtt_estimate;
                old_path->rtt_variant = rtt_estimate / 2;
                old_path->rtt_min = old_path->min_rtt_estimate_in_period;
                old_path->rtt_is_initialized = 1;
            }
            else {
                /* use the average of all samples to adjust the average */
                /* use the lowest and highest samples in the period to adjust the variant */
                uint64_t rtt_var_sample = 0;
                if (old_path->smoothed_rtt > old_path->max_rtt_estimate_in_period) {
                    rtt_var_sample = old_path->smoothed_rtt - old_path->min_rtt_estimate_in_period;
                }
                else if (old_path->smoothed_rtt < old_path->min_rtt_estimate_in_period) {
                    rtt_var_sample = old_path->max_rtt_estimate_in_period - old_path->smoothed_rtt;
                }
                else {
                    uint64_t rtt_var_sample_min = old_path->smoothed_rtt - old_path->min_rtt_estimate_in_period;

                    rtt_var_sample = old_path->max_rtt_estimate_in_period - old_path->smoothed_rtt;
                    if (rtt_var_sample_min > rtt_var_sample) {
                        rtt_var_sample = rtt_var_sample_min;
                    }
                }
                old_path->rtt_variant = (3 * old_path->rtt_variant + rtt_var_sample) / 4;
                old_path->smoothed_rtt = (7 * old_path->smoothed_rtt + rtt_estimate) / 8;
            }
            old_path->retransmit_timer = old_path->smoothed_rtt + 3 * old_path->rtt_variant +
                cnx->remote_parameters.max_ack_delay;

            /* if RTT updated, reset delayed ACK parameters */
            if (old_path == cnx->path[0] && cnx->cnx_state >= picoquic_state_ready) {
                cnx->is_ack_frequency_updated = cnx->is_ack_frequency_negotiated;
                if (!cnx->is_ack_frequency_negotiated || cnx->cnx_state != picoquic_state_ready) {
                    picoquic_compute_ack_gap_and_delay(cnx, cnx->path[0]->rtt_min, PICOQUIC_ACK_DELAY_MIN,
                        cnx->path[0]->receive_rate_max, &cnx->ack_gap_remote, &cnx->ack_delay_remote);
                }
            }

            /* reset the period counters */
            if (pkt_ctx != NULL) {
                old_path->rtt_packet_previous_period = pkt_ctx->highest_acknowledged;
            }
            old_path->nb_rtt_estimate_in_period = 0;
            old_path->sum_rtt_estimate_in_period = 0;
            old_path->max_rtt_estimate_in_period = 0;
            old_path->min_rtt_estimate_in_period = UINT64_MAX;
        }

        /* Pass the new values to the congestion algorithm */
        if (cnx->congestion_alg != NULL) {
            picoquic_per_ack_state_t ack_state = { 0 };
            ack_state.pc = picoquic_context_from_epoch(epoch);
            ack_state.rtt_measurement = rtt_estimate;
            ack_state.one_way_delay = (cnx->is_time_stamp_enabled) ? old_path->one_way_delay_sample : 0;
            cnx->congestion_alg->alg_notify(cnx, old_path,
                picoquic_congestion_notification_rtt_measurement,
                &ack_state, current_time);
        }

        /* On very first sample, apply the saved BDP */
        if (is_first) {
            picoquic_validate_bdp_seed(cnx, old_path, rtt_estimate, current_time);
        }
        /* Perform a quality changed callback if needed */
        (void)picoquic_issue_path_quality_update(cnx, old_path);
    }
}
```

### Rust body
```rust
    ) {
        if old_path.rtt_is_initialized && epoch < 0 {
            return;
        }

        let is_first = !old_path.rtt_is_initialized;
        let mut rtt_estimate = current_time.ticks().saturating_sub(send_time.ticks());
        if !is_first && ack_delay > 0 && self.connection_state >= State::Ready {
            ack_delay = ack_delay.min(self.local_parameters.max_ack_delay as u64);
            if old_path.rtt_min.ticks().saturating_add(ack_delay) < rtt_estimate {
                rtt_estimate = rtt_estimate.saturating_sub(ack_delay);
            }
        }

        let estimate = Duration::from_ticks(rtt_estimate);
        old_path.rtt_sample = estimate;
        old_path.nb_rtt_estimate_in_period += 1;
        old_path.sum_rtt_estimate_in_period += estimate;
        if old_path.nb_rtt_estimate_in_period == 1 {
            old_path.min_rtt_estimate_in_period = estimate;
            old_path.max_rtt_estimate_in_period = estimate;
        } else {
            old_path.min_rtt_estimate_in_period = old_path.min_rtt_estimate_in_period.min(estimate);
            old_path.max_rtt_estimate_in_period = old_path.max_rtt_estimate_in_period.max(estimate);
        }
        if old_path.retransmit_timer < estimate {
            old_path.retransmit_timer = estimate;
        }
        if old_path.rtt_min.ticks() == 0 || old_path.rtt_min > estimate {
            old_path.rtt_min = estimate;
        }
        old_path.rtt_max = old_path.rtt_max.max(estimate);

        if is_first {
            old_path.smoothed_rtt = estimate;
            old_path.rtt_variant = estimate / 2;
            old_path.rtt_is_initialized = true;
        } else {
            let smoothed = old_path.smoothed_rtt.ticks();
            let sample = estimate.ticks();
            let abs_delta = smoothed.abs_diff(sample);
            old_path.rtt_variant =
                Duration::from_ticks((3 * old_path.rtt_variant.ticks() + abs_delta) / 4);
            old_path.smoothed_rtt = Duration::from_ticks((7 * smoothed + sample) / 8);
        }
        old_path.retransmit_timer = Duration::from_ticks(
            old_path
                .smoothed_rtt
                .ticks()
                .saturating_add(4 * old_path.rtt_variant.ticks())
                .saturating_add(self.max_ack_delay_remote.ticks())
                .max(MIN_RETRANSMIT_TIMER.ticks()),
        );
        if is_first {
            self.validate_bdp_seed(old_path, rtt_estimate, current_time);
        }
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_api_unload`
C: `picoquic/tls_api.c:238-245 picoquic_tls_api_unload`
Rust: `rs/fq/src/tls_api.rs:2491-2498 tls_api_unload`

### C body
```c
{
    if (tls_api_is_init) {
        picoquic_tls_api_init_providers(1);
        picoquic_tls_api_zero();
        tls_api_is_init = 0;
    }
}
```

### Rust body
```rust
pub fn tls_api_unload() {
    let mut state = tls_api_state();
    if state.is_init {
        tls_api_init_providers_locked(&mut state, 1);
        state.zero();
        state.is_init = false;
    }
}
```

## Pair `picoquic/tls_api.c:picoquic_register_tls_key_provider_fn`
C: `picoquic/tls_api.c:321-337 picoquic_register_tls_key_provider_fn`
Rust: `rs/fq/src/tls_api.rs:2378-2380 register_tls_key_provider`

### C body
```c
{
    DBG_PRINTF("%s", "Loading set key functions.");
    if (set_key_from_key_file_fn != NULL) {
        picoquic_set_private_key_from_file_fn = set_key_from_key_file_fn;
        picoquic_dispose_sign_certificate_fn = dispose_sign_certificate_fn;
        picoquic_get_certs_from_file_fn = get_certs_from_file_fn;
    }

    if (get_public_key_from_private_fn != NULL) {
        picoquic_get_public_key_from_private_fn = get_public_key_from_private_fn;
    }
}
```

### Rust body
```rust
    fn register_tls_key_provider(&mut self, provider: CryptoProvider) {
        self.private_key_provider = Some(provider);
    }
```

## Pair `picoquic/tls_api.c:picoquic_get_ecb_cipher_by_id`
C: `picoquic/tls_api.c:451-469 picoquic_get_ecb_cipher_by_id`
Rust: `rs/fq/src/tls_api.rs:2229-2231 get_ecb_cipher_by_name`

### C body
```c
{
    ptls_cipher_algorithm_t* ecb_cipher = NULL;

    for (int j = 0; j < 2 && ecb_cipher == NULL; j++) {
        for (int i = 0; i < PICOQUIC_CIPHER_SUITES_NB_MAX && ecb_cipher == NULL; i++) {
            ptls_cipher_suite_t* suite = (j == 0) ?
                picoquic_cipher_suites[i].high_memory_suite :
                picoquic_cipher_suites[i].low_memory_suite;

            if (suite != NULL && suite->aead != NULL && suite->aead->ecb_cipher != NULL &&
                strcmp(suite->aead->ecb_cipher->name, ecb_cipher_name) == 0){
                ecb_cipher = suite->aead->ecb_cipher;
                break;
            }
        }
    }
    return ecb_cipher;
}
```

### Rust body
```rust
fn get_ecb_cipher_by_name(ecb_cipher_name: &str) -> bool {
    ecb_cipher_name == "AES128-ECB"
}
```

## Pair `picoquic/tls_api.c:picoquic_get_sha256`
C: `picoquic/tls_api.c:510-515 picoquic_get_sha256`
Rust: `rs/fq/src/tls_api.rs:592-610 new`

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

## Pair `picoquic/tls_api.c:set_private_key_from_file`
C: `picoquic/tls_api.c:600-609 set_private_key_from_file`
Rust: `rs/fq/src/tls_api.rs:1995-2002 set_private_key_from_file`

### C body
```c
{
    if (picoquic_set_private_key_from_file_fn == NULL) {
        return -1;
    }
    else {
        return picoquic_set_private_key_from_file_fn(keypem, ctx);
    }
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

## Pair `picoquic/tls_api.c:picoquic_explain_crypto_error`
C: `picoquic/tls_api.c:679-689 picoquic_explain_crypto_error`
Rust: `rs/fq/src/sys/openssl.rs:451-461 explain_crypto_error`

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

## Pair `picoquic/tls_api.c:picoquic_get_aes128gcm_v`
C: `picoquic/tls_api.c:720-729 picoquic_get_aes128gcm_v`
Rust: `rs/fq/src/tls_api.rs:2594-2603 picoquic_get_aes128gcm_v`

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

## Pair `picoquic/tls_api.c:picoquic_crypto_random`
C: `picoquic/tls_api.c:771-776 picoquic_crypto_random`
Rust: `rs/fq/src/tls_api.rs:1178-1181 crypto_random`

### C body
```c
{
    ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;

    ctx->random_bytes(buf, len);
}
```

### Rust body
```rust
    pub fn crypto_random(&mut self, buf: &mut [u8]) {
        use rand_core::RngCore;
        self.rng.fill_bytes(buf);
    }
```

## Pair `picoquic/tls_api.c:picoquic_public_random_seed_64`
C: `picoquic/tls_api.c:841-856 picoquic_public_random_seed_64`
Rust: `rs/fq/src/lib.rs:5130-5142 public_random_seed_64`

### C body
```c
{
    if (reset) {
        public_random_index = 0;
        for (uint64_t i = 0; i < 16; i++) {
            public_random_seed[i] = i + 1u;
        }
        public_random_obfuscator = 0x5555555555555555ull;
    }

    public_random_seed[public_random_index] ^= seed;

    for (int i = 0; i < 16; i++) {
        (void)picoquic_public_random_step();
    }
}
```

### Rust body
```rust
pub fn public_random_seed_64(seed: u64, reset_context: i32) {
    let mut state = PUBLIC_RANDOM_STATE
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    if reset_context != 0 {
        *state = INITIAL_PUBLIC_RANDOM;
    }
    let idx = state.index;
    state.seed[idx] ^= seed;
    for _ in 0..16 {
        state.step();
    }
}
```

## Pair `picoquic/tls_api.c:picoquic_set_aead_from_secret`
C: `picoquic/tls_api.c:1296-1309 picoquic_set_aead_from_secret`
Rust: `rs/fq/src/tls_api.rs:510-525 set_aead_from_secret`

### C body
```c
{
    int ret = 0;

    if (*v_aead != NULL) {
        ptls_aead_free((ptls_aead_context_t*)*v_aead);
    }

    if ((*v_aead = ptls_aead_new(cipher->aead, cipher->hash, is_enc, secret, prefix_label)) == NULL) {
        ret = PTLS_ERROR_NO_MEMORY;
    }

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    let key: Box<dyn crate::tls::PacketKey> = match suite {
        AeadSuiteId::Aes128GcmSha256 => packet_key_from_secret(secret, prefix_label)?,
        AeadSuiteId::Aes256GcmSha384 => {
            Box::new(Aes256GcmPacketKey::from_secret(secret, prefix_label)?)
        }
    };
    *aead_slot = Some(key);
    Ok(())
}
```

## Pair `picoquic/tls_api.c:picoquic_setup_initial_master_secret`
C: `picoquic/tls_api.c:1433-1451 picoquic_setup_initial_master_secret`
Rust: `rs/fq/src/tls_api.rs:1336-1348 setup_initial_master_secret`

### C body
```c
{
    int ret = 0;
    ptls_iovec_t ikm;
    uint8_t cnx_id_serialized[PICOQUIC_CONNECTION_ID_MAX_SIZE];

    ikm.len = picoquic_format_connection_id(cnx_id_serialized, PICOQUIC_CONNECTION_ID_MAX_SIZE,
        initial_cnxid);
    ikm.base = cnx_id_serialized;

    /* Extract the master key -- key length will be 32 per SHA256 */
    ret = ptls_hkdf_extract(cipher->hash, master_secret, salt, ikm);

    return ret;
}
```

### Rust body
```rust
) -> Result<(), Error> {
    if master_secret.len() < SHA256_SIZE {
        return Err(Error::BufferTooSmall);
    }
    let (prk, _) =
        hkdf::Hkdf::<sha2::Sha256>::extract(Some(salt), initial_connection_id.as_bytes());
    master_secret[..SHA256_SIZE].copy_from_slice(&prk);
    Ok(())
}
```

## Pair `picoquic/tls_api.c:picoquic_get_initial_aead_context`
C: `picoquic/tls_api.c:1531-1561 picoquic_get_initial_aead_context`
Rust: `rs/fq/src/tls_api.rs:1450-1470 initial_aead_context`

### C body
```c
{
    int ret = 0;
    ptls_cipher_suite_t* cipher = NULL;
    uint8_t client_secret[256];
    uint8_t server_secret[256];
    const char *prefix_label = picoquic_supported_versions[version_index].tls_prefix_label;

    *aead_ctx = NULL;
    *pn_enc_ctx = NULL;

    ret = picoquic_compute_initial_secrets(quic, version_index, initial_cnxid, &cipher, client_secret, server_secret);

    if (ret == 0) {
        uint8_t* selected_secret;

        if (!is_client) {
            selected_secret = (is_enc) ? server_secret : client_secret;
        }
        else {
            selected_secret = (is_enc) ? client_secret : server_secret;
        }

        ret = picoquic_set_aead_from_secret(aead_ctx, cipher, is_enc, selected_secret, prefix_label);
        if (ret == 0) {
            ret = picoquic_set_pn_enc_from_secret(pn_enc_ctx, cipher, is_enc, selected_secret, prefix_label);
        }
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Result<InitialAeadContext, Error> {
        let version = version_from_index(version_index).ok_or(Error::InvalidArgument)?;
        let params = version.parameters();
        let (_suite, client_secret, server_secret) =
            picoquic_compute_initial_secrets(self, version_index, initial_connection_id)?;
        let selected_secret = if is_client == is_enc {
            &client_secret[..]
        } else {
            &server_secret[..]
        };
        Ok(InitialAeadContext {
            aead_ctx: packet_key_from_secret(selected_secret, params.tls_prefix_label)?,
            pn_enc_ctx: header_key_from_secret(selected_secret, params.tls_prefix_label)?,
        })
    }
```

## Pair `picoquic/tls_api.c:picoquic_compute_new_rotated_keys`
C: `picoquic/tls_api.c:1608-1666 picoquic_compute_new_rotated_keys`
Rust: `rs/fq/src/tls_api.rs:1513-1547 compute_new_rotated_keys`

### C body
```c
{
    int ret = 0;
    picoquic_tls_ctx_t * tls_ctx = (picoquic_tls_ctx_t *)cnx->tls_ctx;
    ptls_cipher_suite_t * cipher = ptls_get_cipher(tls_ctx->tls);
    const char *prefix_label = picoquic_supported_versions[cnx->version_index].tls_prefix_label;
    const char *traffic_update_label = picoquic_supported_versions[cnx->version_index].tls_traffic_update_label;

    /* Verify that the previous transition is complete */
    if (cnx->crypto_context_new.aead_decrypt != NULL ||
        cnx->crypto_context_new.aead_encrypt != NULL) {
        if (cnx->crypto_context_new.aead_decrypt == NULL ||
            cnx->crypto_context_new.aead_encrypt == NULL) {
            ret = PICOQUIC_ERROR_CANNOT_COMPUTE_KEY;
        }
        else {
            /* already computed */
            return 0;
        }
    }

    /* Recompute the secrets */
    if (ret == 0) {
        ret = picoquic_rotate_app_secret(cipher, tls_ctx->app_secret_enc, traffic_update_label);
#ifdef _DEBUG
        if (ret == 0) {
            DBG_PRINTF("Rotated Encryption Secret (%d):\n", (int)cipher->hash->digest_size);
            debug_dump(tls_ctx->app_secret_enc, (int)cipher->hash->digest_size);
        }
        else {
            DBG_PRINTF("Encryption secret rotation fails, ret=%x\n", ret);
        }
#endif
    }

    if (ret == 0) {
        ret = picoquic_set_key_from_secret(cipher, 1, 1, &cnx->crypto_context_new, tls_ctx->app_secret_enc, prefix_label);
    }

    if (ret == 0) {
        ret = picoquic_rotate_app_secret(cipher, tls_ctx->app_secret_dec, traffic_update_label);
#ifdef _DEBUG
        if (ret == 0) {
            DBG_PRINTF("Rotated Decryption Secret (%d):\n", (int)cipher->hash->digest_size);
            debug_dump(tls_ctx->app_secret_dec, (int)cipher->hash->digest_size);
        }
        else {
            DBG_PRINTF("Decryption secret rotation fails, ret=%x\n", ret);
        }
#endif

    }

    if (ret == 0) {
        ret = picoquic_set_key_from_secret(cipher, 0, 1, &cnx->crypto_context_new, tls_ctx->app_secret_dec, prefix_label);
    }

    return (ret == 0)?0: PICOQUIC_ERROR_CANNOT_COMPUTE_KEY;
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

## Pair `picoquic/tls_api.c:picoquic_master_tlscontext_free`
C: `picoquic/tls_api.c:1874-1913 picoquic_master_tlscontext_free`
Rust: `rs/fq/src/tls_api.rs:968-971 free_master_tls_context`

### C body
```c
{
    if (quic->tls_master_ctx != NULL) {
        ptls_context_t* ctx = (ptls_context_t*)quic->tls_master_ctx;

        if (quic->p_simulated_time != NULL && ctx->get_time != NULL) {
            free(ctx->get_time);
            ctx->get_time = NULL;
        }

        free_certificates_list(ctx->certificates.list, ctx->certificates.count);

        picoquic_dispose_sign_certificate(ctx);

        picoquic_dispose_verify_certificate_callback(quic);

        if (ctx->on_client_hello != NULL) {
            free(ctx->on_client_hello);
        }

        if (ctx->encrypt_ticket != NULL) {
            free(ctx->encrypt_ticket);
        }

        if (ctx->update_traffic_key != NULL) {
            free(ctx->update_traffic_key);
        }

        /* Need to be tested */
        if (ctx->save_ticket != NULL) {
            free(ctx->save_ticket);
        }

        if (ctx->cipher_suites != NULL) {
            free((void*)ctx->cipher_suites);
        }

        picoquic_free_log_event(quic);
    }
}
```

### Rust body
```rust
    pub fn free_master_tls_context(&mut self) {
        self.tls_client_config = None;
        self.tls_server_config = None;
    }
```

## Pair `picoquic/tls_api.c:picoquic_tlscontext_remove_ticket`
C: `picoquic/tls_api.c:2072-2084 picoquic_tlscontext_remove_ticket`
Rust: `rs/fq/src/tls_api.rs:1033-1037 remove_tls_ticket`

### C body
```c
{
    /* allocate a context structure */
    picoquic_tls_ctx_t* ctx = (picoquic_tls_ctx_t*)(cnx->tls_ctx);

    ctx->handshake_properties.client.session_ticket.base = NULL;
    ctx->handshake_properties.client.session_ticket.len = 0;
}
```

### Rust body
```rust
    pub fn remove_tls_ticket(&mut self) {
        self.resumed_ticket_id = 0;
        self.psk_cipher_suite_id = 0;
        self.max_early_data_size = 0;
    }
```

## Pair `picoquic/tls_api.c:picoquic_tls_is_psk_handshake`
C: `picoquic/tls_api.c:2153-2158 picoquic_tls_is_psk_handshake`
Rust: `rs/fq/src/lib.rs:2891-2904 tls_is_psk_handshake`

### C body
```c
{
    /* int ret = cnx->is_psk_handshake; */
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return ptls_is_psk_handshake(((picoquic_tls_ctx_t*)(cnx->tls_ctx))->tls);
}
```

### Rust body
```rust
    pub fn peer_addr(&self) -> SocketAddr {
        self.paths
            .first()
            .and_then(|p| p.tuples.first())
            .map(|t| t.peer_addr)
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap())
    }
```
