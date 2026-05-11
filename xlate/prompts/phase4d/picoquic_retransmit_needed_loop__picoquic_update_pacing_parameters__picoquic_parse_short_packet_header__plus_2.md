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

## `picoquic/loss_recovery.c:picoquic_retransmit_needed_loop`
* Phase 4C status: `suspect`
* Phase 4C rationale: Both loop pending packets and keep the last retransmit length, but Rust filters the saved next token through queued_packets before continuing, an extra body-visible condition absent in C.
* C source: `picoquic/loss_recovery.c:203-223`
* C signature: `size_t picoquic_retransmit_needed_loop(picoquic_cnx_t *, picoquic_packet_context_t *, picoquic_packet_context_enum, picoquic_path_t *, uint64_t, uint64_t *, picoquic_packet_t *, size_t, size_t *)`
* Rust source: `rs/fq/src/internal.rs:11822-11858`
* Rust item: `retransmit_needed_loop`

### C body
```c
{
    int continue_next = 1;
    size_t length = 0;
    picoquic_packet_t* old_p = pkt_ctx->pending_first;

    /* Call the per packet routine in a loop */
    while (old_p != 0 && continue_next) {
        picoquic_packet_t* p_next = old_p->packet_next;

        length = picoquic_retransmit_needed_packet(cnx, pkt_ctx, old_p, pc, path_x, current_time,
            next_wake_time, packet, send_buffer_max, header_length, &continue_next);
        old_p = p_next;
    }
    /* TODO: manage the pto flag for the path. */

    return length;
}
```

### Rust body
```rust
    ) -> usize {
        let mut continue_next = 1;
        let mut length = 0usize;
        let mut old_p = self.pending_first_token(selection);

        while let Some(old_token) = old_p {
            if continue_next == 0 {
                break;
            }
            let p_next = self.pending_next_token(selection, old_token);
            length = self.retransmit_needed_packet(
                selection,
                old_token,
                pc,
                path_x,
                current_time,
                next_wake_time,
                packet,
                send_buffer_max,
                header_length,
                &mut continue_next,
            );
            old_p = p_next.filter(|token| self.queued_packets.contains(*token));
        }

        length
    }
```

## `picoquic/pacing.c:picoquic_update_pacing_parameters`
* Phase 4C status: `suspect`
* Phase 4C rationale: C always computes packet_time/bucket_max and optionally reports pacing updates when signalled_path is non-null; Rust skips updates unless pacing_rate > 0.0 and send_mtu > 0 and has no visible report call.
* C source: `picoquic/pacing.c:137-180`
* C signature: `void picoquic_update_pacing_parameters(picoquic_pacing_t *, double, uint64_t, size_t, uint64_t, picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:5964-6008`
* Rust item: `update_parameters`

### C body
```c
{
    double packet_time = (double)send_mtu / pacing_rate;
    double quantum_time = (double)quantum / pacing_rate;
    uint64_t rtt_nanosec = smoothed_rtt * 1000;

    pacing->rate = (uint64_t)pacing_rate;

    if (quantum > pacing->quantum_max) {
        pacing->quantum_max = quantum;
    }
    if (pacing->rate > pacing->rate_max) {
        pacing->rate_max = pacing->rate;
    }

    pacing->packet_time_nanosec = (uint64_t)(packet_time * 1000000000.0);

    if (pacing->packet_time_nanosec <= 0) {
        pacing->packet_time_nanosec = 1;
        pacing->packet_time_microsec = 1;
    }
    else {
        if ((uint64_t)pacing->packet_time_nanosec > rtt_nanosec) {
            pacing->packet_time_nanosec = rtt_nanosec;
        }
        pacing->packet_time_microsec = (pacing->packet_time_nanosec + 999ull) / 1000;
    }

    pacing->bucket_max = (uint64_t)(quantum_time * 1000000000.0);
    if (pacing->bucket_max <= 0) {
        pacing->bucket_max = 16 * pacing->packet_time_nanosec;
    }

    if (pacing->bucket_nanosec > pacing->bucket_max) {
        pacing->bucket_nanosec = pacing->bucket_max;
    }

    if (signalled_path != NULL) {
        picoquic_report_pacing_update(pacing, signalled_path);
    }
}
```

### Rust body
```rust
    ) {
        // C: picoquic_update_pacing_parameters
        let rtt_nanosec = smoothed_rtt.ticks().saturating_mul(1000);
        self.rate = if pacing_rate.is_sign_positive() {
            pacing_rate as u64
        } else {
            0
        };
        if quantum > self.quantum_max {
            self.quantum_max = quantum;
        }
        if self.rate > self.rate_max {
            self.rate_max = self.rate;
        }

        if pacing_rate > 0.0 && send_mtu > 0 {
            let mut packet_time_nanosec = (send_mtu as f64 / pacing_rate * 1_000_000_000.0) as i64;
            if packet_time_nanosec <= 0 {
                packet_time_nanosec = 1;
                self.packet_time_microsec = crate::Duration::from_ticks(1);
            } else {
                if packet_time_nanosec as u64 > rtt_nanosec {
                    packet_time_nanosec = rtt_nanosec as i64;
                }
                self.packet_time_microsec =
                    crate::Duration::from_ticks(((packet_time_nanosec + 999) / 1000) as u64);
            }
            self.packet_time_nanosec = packet_time_nanosec;

            self.bucket_max = (quantum as f64 / pacing_rate * 1_000_000_000.0) as i64;
            if self.bucket_max <= 0 {
                self.bucket_max = 16 * self.packet_time_nanosec;
            }
            if self.bucket_nanosec > self.bucket_max {
                self.bucket_nanosec = self.bucket_max;
            }
        }
    }
```

## `picoquic/packet.c:picoquic_parse_short_packet_header`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust always uses local_connection_id_length, while C uses the existing connection remote CID length for outgoing packets; Rust also combines incoming/outgoing loss-bit flags instead of selecting by receiving.
* C source: `picoquic/packet.c:396-473`
* C signature: `int picoquic_parse_short_packet_header(picoquic_quic_t *, const uint8_t *, size_t, const struct sockaddr *, picoquic_packet_header *, picoquic_cnx_t **, int)`
* Rust source: `rs/fq/src/internal.rs:6749-6827`
* Rust item: `parse_short_packet_header_inner`

### C body
```c
{
    int ret = 0;
    /* If this is a short header, it should be possible to retrieve the connection
     * context. This depends on whether the quic context requires cnx_id or not.
     */
    uint8_t cnxid_length = (receiving == 0 && *pcnx != NULL) ? (*pcnx)->path[0]->first_tuple->p_remote_cnxid->cnx_id.id_len : quic->local_cnxid_length;
    ph->pc = picoquic_packet_context_application;
    ph->pl_val = 0; /* No actual payload length in short headers */

    if ((int)length >= 1 + cnxid_length) {
        /* We can identify the connection by its ID */
        ph->offset = (size_t)1 + picoquic_parse_connection_id(bytes + 1, cnxid_length, &ph->dest_cnx_id);
        /* TODO: should consider using combination of CNX ID and ADDR_FROM */
        if (*pcnx == NULL)
        {
            if (quic->local_cnxid_length > 0) {
                *pcnx = picoquic_cnx_by_id(quic, ph->dest_cnx_id, &ph->l_cid);
            }
            else {
                *pcnx = picoquic_cnx_by_net(quic, addr_from);
            }
        }
    }
    else {
        ph->ptype = picoquic_packet_error;
        ph->offset = length;
        ph->payload_length = 0;
    }

    if (*pcnx != NULL) {
        int has_loss_bit = (receiving && (*pcnx)->is_loss_bit_enabled_incoming) || ((!receiving && (*pcnx)->is_loss_bit_enabled_outgoing));
        ph->epoch = picoquic_epoch_1rtt;
        ph->version_index = (*pcnx)->version_index;
        ph->quic_bit_is_zero = (bytes[0] & 0x40) == 0;

        if (!ph->quic_bit_is_zero ||(*pcnx)->local_parameters.do_grease_quic_bit) {
            /* We do not check the quic bit if the local endpoint advertised greasing. */
            ph->ptype = picoquic_packet_1rtt_protected;
        } else {
            /* Check for QUIC bit failed! */
            ph->ptype = picoquic_packet_error;
        }

        ph->has_spin_bit = 1;
        ph->spin = (bytes[0] >> 5) & 1;
        ph->pn_offset = ph->offset;
        ph->pn = 0;
        ph->pnmask = 0;
        ph->key_phase = ((bytes[0] >> 2) & 1); /* Initialize here so that simple tests with unencrypted headers can work */

        if (has_loss_bit) {
            ph->has_loss_bits = 1;
            ph->loss_bit_L = (bytes[0] >> 3) & 1;
            ph->loss_bit_Q = (bytes[0] >> 4) & 1;
        }
        if (length < ph->offset || ph->ptype == picoquic_packet_error) {
            ret = -1;
            ph->payload_length = 0;
        }
        else {
            ph->payload_length = (uint16_t)(length - ph->offset);
        }
    }
    else {
        /* This may be a packet to a forgotten connection, or a packet bound to a proxied connection */
        ph->ptype = picoquic_packet_1rtt_protected;
        ph->payload_length = (uint16_t)((length > ph->offset) ? length - ph->offset : 0);
    }
    return ret;
}
```

### Rust body
```rust
    ) -> Option<ConnectionToken> {
        let cnxid_length = self.local_connection_id_length as usize;
        ph.packet_context = PacketContext::Application;
        ph.payload_length_value = 0;

        if bytes.len() < 1 + cnxid_length {
            ph.packet_type = PacketType::Error;
            ph.offset = bytes.len();
            ph.payload_length = 0;
            return None;
        }

        let dcid = ConnectionId::clone_from_slice(&bytes[1..1 + cnxid_length]).unwrap_or_default();
        ph.dest_connection_id = dcid;
        ph.offset = 1 + cnxid_length;
        ph.packet_number_offset = ph.offset;

        // Lookup connection
        let conn_tok = if cnxid_length > 0 {
            let cid = ph.dest_connection_id;
            self.connection_by_id(cid).map(|(tok, _)| tok)
        } else {
            self.connection_by_net(addr_from)
        };

        ph.epoch = Epoch::OneRtt;
        ph.quic_bit_is_zero = (bytes[0] & 0x40) == 0;

        // Check QUIC bit (allow grease mode)
        let do_grease = conn_tok
            .and_then(|tok| self.connections.get(tok))
            .map(|c| c.local_parameters.do_grease_quic_bit)
            .unwrap_or(false);

        if !ph.quic_bit_is_zero || do_grease {
            ph.packet_type = PacketType::OneRttProtected;
        } else {
            ph.packet_type = PacketType::Error;
        }

        ph.has_spin_bit = true;
        ph.spin = (bytes[0] >> 5) & 1 != 0;
        ph.key_phase = ((bytes[0] >> 2) & 1) != 0;
        ph.packet_number_mask = 0;
        ph.packet_number_truncated = 0;

        if conn_tok.is_some() {
            let is_loss_bit = conn_tok
                .and_then(|tok| self.connections.get(tok))
                .map(|c| c.is_loss_bit_enabled_incoming || c.is_loss_bit_enabled_outgoing)
                .unwrap_or(false);
            if is_loss_bit {
                ph.has_loss_bits = true;
                ph.loss_bit_l = (bytes[0] >> 3) & 1 != 0;
                ph.loss_bit_q = (bytes[0] >> 4) & 1 != 0;
            }
        }

        ph.payload_length = if bytes.len() > ph.offset {
            bytes.len() - ph.offset
        } else {
            0
        };

        // Set version_index from connection if found
        if let Some(tok) = conn_tok
            && let Some(cnx) = self.connections.get(tok)
        {
            ph.version_index = cnx.version_index;
        }

        conn_tok
    }
```

## `picoquic/picohash.c:picohash_insert`
* Phase 4C status: `suspect`
* Phase 4C rationale: C unconditionally prepends a new item and increments count; Rust first searches for an existing key and replaces its value without incrementing len.
* C source: `picoquic/picohash.c:84-109`
* C signature: `int picohash_insert(picohash_table *, const void *)`
* Rust source: `rs/fq/src/hash.rs:334-355`
* Rust item: `insert`

### C body
```c
{
    uint64_t hash = hash_table->picohash_hash(key, hash_table->hash_seed);
    uint32_t bin = (uint32_t)(hash % hash_table->nb_bin);
    int ret = 0;
    picohash_item* item;
    
    if (hash_table->picohash_key_to_item == NULL) {
        item = (picohash_item*)malloc(sizeof(picohash_item));
    }
    else {
        item = hash_table->picohash_key_to_item(key);
    }

    if (item == NULL) {
        ret = -1;
    } else {
        item->hash = hash;
        item->key = key;
        item->next_in_bin = hash_table->hash_bin[bin];
        hash_table->hash_bin[bin] = item;
        hash_table->count++;
    }

    return ret;
}
```

### Rust body
```rust
    pub fn insert(&mut self, key: K, value: V) -> Result<(HashToken, Option<V>), Error> {
        let hash = self.hash_key(&key);
        let bin = self.bin_of(hash);

        // Check for existing key.
        let mut cur = self.bins[bin];
        while let Some(idx) = cur {
            if self.slot_key(idx) == &key {
                let old = core::mem::replace(self.slot_value_mut(idx), value);
                return Ok((self.token_of(idx), Some(old)));
            }
            cur = self.slot_next(idx);
        }

        // New entry: prepend to bin chain.
        let idx = self.alloc_slot(hash, key, value)?;
        let old_head = self.bins[bin];
        self.set_next(idx, old_head);
        self.bins[bin] = Some(idx);
        self.len += 1;
        Ok((self.token_of(idx), None))
    }
```

## `picoquic/picoquic_ptls_openssl.c:openssl_keyex_dispose`
* Phase 4C status: `suspect`
* Phase 4C rationale: C disposes by invoking keyex->on_exchange with dispose flag; Rust only drops the KeyExchangeContext.
* C source: `picoquic/picoquic_ptls_openssl.c:367-371`
* C signature: `void openssl_keyex_dispose(ptls_key_exchange_context_t *)`
* Rust source: `rs/fq/src/sys/openssl.rs:322-324`
* Rust item: `openssl_keyex_dispose`

### C body
```c
{
    ptls_iovec_t dummy = ptls_iovec_init(NULL, 0);
    keyex->on_exchange(&keyex, 1, NULL, dummy);
}
```

### Rust body
```rust
pub fn openssl_keyex_dispose(keyex: KeyExchangeContext) {
    drop(keyex);
}
```
