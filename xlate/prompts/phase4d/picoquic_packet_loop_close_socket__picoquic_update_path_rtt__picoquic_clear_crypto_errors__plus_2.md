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

## `picoquic/sockloop.c:picoquic_packet_loop_close_socket`
* Phase 4C status: `suspect`
* Phase 4C rationale: C explicitly closes the socket and Windows event/buffer resources; Rust only sets fd to None and is_started to false.
* C source: `picoquic/sockloop.c:344-361`
* C signature: `void picoquic_packet_loop_close_socket(picoquic_socket_ctx_t *)`
* Rust source: `rs/fq/src/packet_loop.rs:1640-1643`
* Rust item: `close`

### C body
```c
{
    if (s_ctx->fd != INVALID_SOCKET) {
        SOCKET_CLOSE(s_ctx->fd);
        s_ctx->fd = INVALID_SOCKET;
    }
#ifdef _WINDOWS
    if (s_ctx->overlap.hEvent != WSA_INVALID_EVENT) {
        WSACloseEvent(s_ctx->overlap.hEvent);
        s_ctx->overlap.hEvent = WSA_INVALID_EVENT;
    }

    if (s_ctx->recv_buffer != NULL) {
        free(s_ctx->recv_buffer);
        s_ctx->recv_buffer = NULL;
    }
#endif
}
```

### Rust body
```rust
    pub fn close(&mut self) {
        self.fd = None;
        self.is_started = false;
    }
```

## `picoquic/timing.c:picoquic_update_path_rtt`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust omits visible C behavior for packet-period gating/counters reset, one-way delay update, ACK frequency update, congestion notification, path quality callback, and uses a different retransmit formula multiplier.
* C source: `picoquic/timing.c:175-321`
* C signature: `void picoquic_update_path_rtt(picoquic_cnx_t *, picoquic_path_t *, int, uint64_t, uint64_t, uint64_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:9191-9255`
* Rust item: `update_path_rtt`

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

## `picoquic/tls_api.c:picoquic_clear_crypto_errors`
* Phase 4C status: `suspect`
* Phase 4C rationale: C conditionally invokes a registered clear-errors callback; Rust directly clears the OpenSSL error stack.
* C source: `picoquic/tls_api.c:691-699`
* C signature: `void picoquic_clear_crypto_errors(void)`
* Rust source: `rs/fq/src/sys/openssl.rs:467-469`
* Rust item: `clear_crypto_errors`

### C body
```c
{
    if (picoquic_clear_crypto_errors_fn != NULL) {
        picoquic_clear_crypto_errors_fn();
    }
}
```

### Rust body
```rust
pub fn clear_crypto_errors() {
    drop(openssl::error::ErrorStack::get());
}
```

## `picoquic/tls_api.c:picoquic_master_tlscontext`
* Phase 4C status: `suspect`
* Phase 4C rationale: C visibly allocates and initializes a TLS context, providers, callbacks, ticket handling, certificate verifier, save-ticket callback, EOED setting, and random seed; Rust only checks certificate/key/root files and installs ticket AEAD contexts.
* C source: `picoquic/tls_api.c:1725-1861`
* C signature: `int picoquic_master_tlscontext(picoquic_quic_t *, const char *, const char *, const char *, const uint8_t *, size_t)`
* Rust source: `rs/fq/src/tls_api.rs:932-952`
* Rust item: `init_master_tls_context`

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

## `picoquic/tls_api.c:picoquic_server_decrypt_retry_token`
* Phase 4C status: `suspect`
* Phase 4C rationale: C treats decrypt output length >= ciphertext length as failure; Rust accepts decrypted payload length after decrypt and only checks destination buffer size.
* C source: `picoquic/tls_api.c:2888-2921`
* C signature: `int picoquic_server_decrypt_retry_token(picoquic_quic_t *, const struct sockaddr *, int *, const uint8_t *, size_t, uint8_t *, size_t *)`
* Rust source: `rs/fq/src/tls_api.rs:1782-1809`
* Rust item: `server_decrypt_retry_token`

### C body
```c
{
    int ret = 0;
    uint64_t sequence;
    uint8_t* auth_data;
    size_t auth_data_length;

    if (addr_peer->sa_family == AF_INET) {
        auth_data = (uint8_t*)&((struct sockaddr_in *)addr_peer)->sin_addr;
        auth_data_length = 4;
    }
    else {
        auth_data = (uint8_t*)&((struct sockaddr_in6 *)addr_peer)->sin6_addr;
        auth_data_length = 16;
    }

    if (token_length < 8) {
        *is_new_token = 0;
        ret = -1;
    }
    else {
        *is_new_token = ((token[0] & 0x80) == 0) ? 0: 1;
        sequence = PICOPARSE_64(token);

        *text_length = picoquic_aead_decrypt_generic(text, token+8, token_length-8,
            sequence, auth_data, auth_data_length, quic->aead_decrypt_ticket_ctx);
        if (*text_length >= token_length - 8) {
            ret = -1;
        }
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<DecryptedRetryToken, Error> {
        ensure_ticket_aead_contexts(self)?;
        if token.len() < 8 {
            return Err(Error::InvalidArgument);
        }
        let is_new_token = (token[0] & 0x80) != 0;
        let sequence = u64::from_be_bytes(token[..8].try_into().unwrap());
        let aad = ip_auth_data(addr_peer);
        let mut payload = token[8..].to_vec();
        let aead = self
            .aead_decrypt_ticket_ctx
            .as_ref()
            .ok_or(Error::InvalidState)?;
        aead.decrypt(sequence, &aad, &mut payload)?;
        if payload.len() > text.len() {
            return Err(Error::BufferTooSmall);
        }
        text[..payload.len()].copy_from_slice(&payload);
        Ok(DecryptedRetryToken {
            is_new_token,
            text_length: payload.len(),
        })
    }
```
