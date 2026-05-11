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

## `picoquic/bytestream.c:bytestream_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the current pointer length, while Rust shown returns whether len is zero.
* C source: `picoquic/bytestream.c:88-91`
* C signature: `size_t bytestream_length(bytestream *)`
* Rust source: `rs/fq/src/bytestream.rs:197-204`
* Rust item: `len`

### C body
```c
{
    return s->ptr;
}
```

### Rust body
```rust
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
```

## `picoquic/c4.c:c4_handle_ack`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body includes ACK gating, rate update, initial handling, era transitions, and state-machine actions; Rust body only contains the inner rate-measurement block.
* C source: `picoquic/c4.c:802-883`
* C signature: `void c4_handle_ack(picoquic_path_t *, c4_state_t *, picoquic_per_ack_state_t *)`
* Rust source: `rs/fq/src/c4.rs:752-780`
* Rust item: `handle_ack`

### C body
```c
{
    uint64_t previous_rate = c4_state->nominal_rate;
    uint64_t rate_measurement = 0;

    if (ack_state->rtt_measurement > 0 && ack_state->nb_bytes_delivered_since_packet_sent > 0) {

        rate_measurement = path_x->bandwidth_estimate;
        C4_LOGGER(path_x, rate_measurement, c4_state, ack_state, 0, 0);

        /* Assessment of rate limited status */
        if (rate_measurement > c4_state->nominal_rate &&
            !(c4_state->alg_state == c4_recovery && c4_state->congestion_notified != 0)) {
            c4_state->push_was_not_limited = 1;
            c4_state->nominal_rate = rate_measurement;
            c4_state->delay_threshold = c4_delay_threshold(c4_state);
        }
        else {
            /* The ACK rate did not grow, but that's not a proof.
                * If the number of bytes sent are larger than the corrected bytes,
                * we know the delivery was slowed by the network, not the app.
                */
            uint64_t target_cwin = PICOQUIC_BYTES_FROM_RATE(c4_state->running_min_rtt, previous_rate);
            if (ack_state->nb_bytes_delivered_since_packet_sent > target_cwin) {
                c4_state->push_was_not_limited = 1;
            }
        }
    }

    if (c4_state->alg_state == c4_initial) {
        c4_initial_handle_ack(path_x, c4_state, ack_state);
    }
    else {
        if (c4_era_check(path_x, c4_state)) {
            /* Update max rtt and running min rtt */
            c4_update_min_max_rtt(path_x, c4_state);
            /* The initial phase may have exited too early if we have both high jitter and competition
            * from other flows. Finding an RTT higher than the previous max is an indication that
            * the previous initial might have exited too soon, especially if the difference between
            * max RTT and min RTT is large. Reentering Initial remedies that.
            * However, reentering Initial is a bit of a hack. It is OK in the high jitter or
            * competition secnarios, but it can backfire and cause congestion and losses. So we don't
            * do that if the RTT is low (lower than 50ms) or if the data rate is high enough
            * (higher than 1Mbps, i.e., 8Mbps). And we only do that once per connection.
            */
            if (!c4_state->initial_after_jitter &&
                c4_state->nominal_max_rtt > 50000 &&
                c4_state->nominal_rate < 1000000 &&
                5 * c4_state->running_min_rtt < 2 * c4_state->nominal_max_rtt) {
                c4_state->initial_after_jitter = 1;
                c4_enter_initial(path_x, c4_state);
            }
            else
            {
                /* Manage the transition to the next state */
                switch (c4_state->alg_state) {
                case c4_recovery:
                    c4_exit_recovery(path_x, c4_state);
                    break;
                case c4_cruising:
                    if (c4_state->nb_cruise_left_before_push > 0) {
                        c4_state->nb_cruise_left_before_push--;
                    }
                    c4_era_reset(path_x, c4_state);
                    if (c4_state->nb_cruise_left_before_push <= 0 &&
                        path_x->last_time_acked_data_frame_sent > path_x->last_sender_limited_time) {
                        c4_enter_push(path_x, c4_state);
                    }
                    break;
                case c4_pushing:
                    c4_enter_recovery(path_x, c4_state, c4_congestion_none);
                    break;
                default:
                    c4_era_reset(path_x, c4_state);
                    break;
                }
            }
        }
    }
}
```

### Rust body
```rust
        {
            let rate_measurement = path_x.bandwidth_estimate;
            c4_logger(
                path_x,
                rate_measurement,
                self,
                Some(ack_state),
                0,
                C4Congestion::None,
            );

            if rate_measurement > self.nominal_rate
                && !(self.alg_state == C4AlgState::Recovery && self.congestion_notified)
            {
                self.push_was_not_limited = true;
                self.nominal_rate = rate_measurement;
                self.delay_threshold = self.delay_threshold();
            } else {
                let target_cwin = bytes_from_rate(self.running_min_rtt, previous_rate);
                if ack_state.nb_bytes_delivered_since_packet_sent > target_cwin {
                    self.push_was_not_limited = true;
                }
            }
        }
```

## `picoquic/config.c:config_parse_target_version`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C parses a hexadecimal string into an integer; Rust body copies bytes into a buffer and returns a UTF-8 string.
* C source: `picoquic/config.c:116-146`
* C signature: `uint32_t config_parse_target_version(const char *)`
* Rust source: `rs/fq/src/config.rs:544-560`
* Rust item: `parse_hex_version`

### C body
```c
{
    /* Expect the version to be encoded in base 16 */
    uint32_t v = 0;
    char const* x = v_arg;

    while (*x != 0) {
        int c = *x;

        if (c >= '0' && c <= '9') {
            c -= '0';
        }
        else if (c >= 'a' && c <= 'f') {
            c -= 'a';
            c += 10;
        }
        else if (c >= 'A' && c <= 'F') {
            c -= 'A';
            c += 10;
        }
        else {
            v = 0;
            break;
        }
        v *= 16;
        v += c;
        x++;
    }

    return v;
}
```

### Rust body
```rust
fn config_optval_string<'a>(buffer: &'a mut [u8], p: &[u8]) -> &'a str {
    let len = p.len().min(buffer.len().saturating_sub(1));
    buffer[..len].copy_from_slice(&p[..len]);
    if !buffer.is_empty() {
        buffer[len] = 0;
    }
    core::str::from_utf8(&buffer[..len]).unwrap_or("")
}
```

## `picoquic/dualq_aqm.c:dualq_laqm`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body computes and returns pprime based on lq.count, queue age, minTh/maxTh/range; Rust body shown is only a time subtraction fragment.
* C source: `picoquic/dualq_aqm.c:210-227`
* C signature: `double dualq_laqm(dualq_state_t *, uint64_t)`
* Rust source: `rs/fq/src/tests/dualq.rs:332-340`
* Rust item: `laqm`

### C body
```c
{
    double pprime = 0;
    uint64_t lq_time = 0;
    /* Returns Native L4S AQM probability */
    if (dualq->lq.count > 1) {
        if (dualq->lq.queue_first->arrival_time < current_time) {
            lq_time = current_time - dualq->lq.queue_first->arrival_time;
        }
        if (lq_time >= dualq->maxTh) {
            pprime = 1.0;
        }
        else if (lq_time > dualq->minTh) {
            pprime = ((double)(lq_time - dualq->minTh)) / dualq->range;
        }
    }
    return pprime;
}
```

### Rust body
```rust
        {
            lq_time = current_time.ticks() - packet.arrival_time.ticks();
        }
```

## `picoquic/ech.c:picoquic_ech_get_ciphers_from_kem`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C selects target/default cipher suites and clears the output vector; Rust body shown only checks output length.
* C source: `picoquic/ech.c:672-736`
* C signature: `int picoquic_ech_get_ciphers_from_kem(ptls_hpke_cipher_suite_t **, size_t, uint16_t)`
* Rust source: `rs/fq/src/ech.rs:156-162`
* Rust item: `ech_get_ciphers_from_kem`

### C body
```c
{
    int ret = 0;
    size_t nb_ciphers = 0;
    uint16_t target_kdf_id = PTLS_HPKE_HKDF_SHA256;
    uint16_t target_aead_id = PTLS_HPKE_AEAD_AES_128_GCM;
    ptls_hpke_cipher_suite_t* target_cipher = NULL;
    ptls_hpke_cipher_suite_t* default_cipher = NULL;

    if (cipher_vec_nb_max < 2) {
        return -1;
    }

    switch (kem_id) {
    case PTLS_HPKE_KEM_P256_SHA256:
        target_kdf_id = PTLS_HPKE_HKDF_SHA256;
        target_aead_id = PTLS_HPKE_AEAD_AES_128_GCM;
        break;
    case PTLS_HPKE_KEM_P384_SHA384:
        target_kdf_id = PTLS_HPKE_HKDF_SHA384;
        target_aead_id = PTLS_HPKE_AEAD_AES_256_GCM;
        break;
    case PTLS_HPKE_KEM_X25519_SHA256:
        target_kdf_id = PTLS_HPKE_HKDF_SHA256;
        target_aead_id = PTLS_HPKE_AEAD_CHACHA20POLY1305;
        break;
    default:
        break;
    }

    for (size_t i = 0; i < PICOQUIC_HPKE_CIPHER_SUITE_NB_MAX; i++) {
        if (picoquic_hpke_cipher_suites[i] == NULL) {
            break;
        }
        else if (picoquic_hpke_cipher_suites[i]->id.aead == target_aead_id &&
            picoquic_hpke_cipher_suites[i]->id.kdf == target_kdf_id) {
            target_cipher = picoquic_hpke_cipher_suites[i];
            break;
        }
        else if (picoquic_hpke_cipher_suites[i]->id.aead == PTLS_HPKE_AEAD_AES_128_GCM &&
            picoquic_hpke_cipher_suites[i]->id.kdf == PTLS_HPKE_HKDF_SHA256) {
            default_cipher = picoquic_hpke_cipher_suites[i];
        }
    }
    if (target_cipher == NULL) {
        if (default_cipher != NULL) {
            cipher_vec[0] = default_cipher;
            nb_ciphers = 1;
        }
        else {
            ret = -1;
        }
    } else {
        cipher_vec[0] = target_cipher;
        nb_ciphers = 1;
        if (default_cipher != NULL && default_cipher != target_cipher && cipher_vec_nb_max > 2) {
            cipher_vec[1] = default_cipher;
            nb_ciphers++;
        }
    }
    for (size_t i = nb_ciphers; i < cipher_vec_nb_max; i++) {
        cipher_vec[i] = NULL;
    }
    return ret;
}
```

### Rust body
```rust
    if out.len() < 2 {
        return Err(Error::Generic);
    }
```
