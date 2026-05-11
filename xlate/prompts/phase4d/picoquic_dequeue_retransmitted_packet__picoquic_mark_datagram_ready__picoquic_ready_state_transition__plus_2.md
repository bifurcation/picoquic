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

## `picoquic/sender.c:picoquic_dequeue_retransmitted_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C removes a packet from the retransmitted queue, updates links and counters, clears a flag, and may recycle; shown Rust only searches for a sequence/token.
* C source: `picoquic/sender.c:1107-1131`
* C signature: `void picoquic_dequeue_retransmitted_packet(picoquic_cnx_t *, picoquic_packet_context_t *, picoquic_packet_t *)`
* Rust source: `rs/fq/src/internal.rs:5645-5659`
* Rust item: `dequeue_retransmitted_packet`

### C body
```c
{
    pkt_ctx->retransmitted_queue_size -= 1;
    if (p->packet_previous == NULL) {
        pkt_ctx->retransmitted_newest = p->packet_next;
    }
    else {
        p->packet_previous->packet_next = p->packet_next;
    }

    if (p->packet_next == NULL) {
        pkt_ctx->retransmitted_oldest = p->packet_previous;
    }
    else {
        p->packet_next->packet_previous = p->packet_previous;
    }

    /* Packets can be queued simultaneously for data repeat and 
    * for detection of spurious losses, so should only be recycled
    * when removed from both queues */
    p->is_queued_for_spurious_detection = 0;
    if (!p->is_queued_for_data_repeat) {
        picoquic_recycle_packet(cnx->quic, p);
    }
}
```

### Rust body
```rust
            .or_else(|| {
                pkt_ctx
                    .retransmitted
                    .iter()
                    .find_map(|(seq, tok)| (*tok == packet).then_some(*seq))
            });
```

## `picoquic/sender.c:picoquic_mark_datagram_ready`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C can return an error when becoming ready with max_datagram_frame_size == 0 and reinserts by wake time; Rust only assigns the flag and returns Ok.
* C source: `picoquic/sender.c:106-122`
* C signature: `int picoquic_mark_datagram_ready(picoquic_cnx_t *, int)`
* Rust source: `rs/fq/src/lib.rs:4385-4388`
* Rust item: `mark_datagram_ready`

### C body
```c
{
    int ret = 0;
    int was_ready = cnx->is_datagram_ready;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    cnx->is_datagram_ready = is_ready;
    if (!was_ready && is_ready) {
        if (cnx->remote_parameters.max_datagram_frame_size == 0) {
            ret = -1;
        }
        else {
            picoquic_reinsert_by_wake_time(cnx->quic, cnx, picoquic_get_quic_time(cnx->quic));
        }
    }
    return ret;
}
```

### Rust body
```rust
    pub fn mark_datagram_ready(&mut self, is_ready: bool) -> Result<(), Error> {
        self.is_datagram_ready = is_ready;
        Ok(())
    }
```

## `picoquic/sender.c:picoquic_ready_state_transition`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust omits many visible C side effects including net secret/random handling, crypto frees, TLS trim, confidentiality limit, migration, callback, and half-open counter/check_token updates.
* C source: `picoquic/sender.c:2698-2775`
* C signature: `void picoquic_ready_state_transition(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:7719-7755`
* Rust item: `ready_state_transition`

### C body
```c
{
    /* Transition to server ready state.
     * The handshake is complete, all the handshake packets are implicitly acknowledged */
    cnx->cnx_state = picoquic_state_ready;
    cnx->is_handshake_finished = 1;
    picoquic_implicit_handshake_ack(cnx, picoquic_packet_context_initial, current_time);
    picoquic_implicit_handshake_ack(cnx, picoquic_packet_context_handshake, current_time);

    (void)picoquic_register_net_secret(cnx);
    if (!cnx->quic->use_predictable_random) {
        picoquic_public_random_seed(cnx->quic);
    }

    if (!cnx->client_mode) {
        (void)picoquic_queue_handshake_done_frame(cnx);
    }

    if (cnx->is_half_open){
        if (cnx->quic->current_number_half_open > 0) {
            cnx->quic->current_number_half_open--;
        }
        cnx->is_half_open = 0;
        if (cnx->quic->current_number_half_open < cnx->quic->max_half_open_before_retry) {
            cnx->quic->check_token = cnx->quic->force_check_token;
        }
    }

    /* Remove handshake and initial keys if they are still around */
    picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_initial]);
    picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_0rtt]);
    picoquic_crypto_context_free(&cnx->crypto_context[picoquic_epoch_handshake]);

    /* Remove the frames queued in initial and handshake contexts */
    picoquic_purge_misc_frames_after_ready(cnx);

    /* Trim the memory buffers allocated during handshake */
    picoquic_tlscontext_trim_after_handshake(cnx);

    /* Set the confidentiality limit if not already set */
    if (cnx->crypto_epoch_length_max == 0) {
        cnx->crypto_epoch_length_max = 
            picoquic_aead_confidentiality_limit(cnx->crypto_context[picoquic_epoch_1rtt].aead_decrypt);
    }

    /* Start migration to server preferred address if present */
    if (cnx->client_mode) {
        (void)picoquic_prepare_server_address_migration(cnx);
    }

    /* Notify the application */
    if (cnx->callback_fn != NULL) {
        if (cnx->callback_fn(cnx, 0, NULL, 0, picoquic_callback_ready, cnx->callback_ctx, NULL) != 0) {
            picoquic_log_app_message(cnx, "Callback ready returns error 0x%x", PICOQUIC_TRANSPORT_INTERNAL_ERROR);
            picoquic_connection_error(cnx, PICOQUIC_TRANSPORT_INTERNAL_ERROR, 0);
        }
    }

    /* Ask for ACK frequency update, or initialize variables if not available */
    if (cnx->is_ack_frequency_negotiated) {
        cnx->is_ack_frequency_updated = 1;
    }
    else {
        picoquic_compute_ack_gap_and_delay(cnx, cnx->path[0]->rtt_min, PICOQUIC_ACK_DELAY_MIN,
            cnx->path[0]->receive_rate_max, &cnx->ack_gap_remote, &cnx->ack_delay_remote);

        /* Keep track of statistics on ACK parameters */
        if (cnx->ack_gap_remote > cnx->max_ack_gap_remote) {
            cnx->max_ack_gap_remote = cnx->ack_gap_remote;
        }
        if (cnx->ack_delay_remote > cnx->max_ack_delay_remote) {
            cnx->max_ack_delay_remote = cnx->ack_delay_remote;
        }
        else if (cnx->ack_delay_remote < cnx->min_ack_delay_remote) {
            cnx->min_ack_delay_remote = cnx->ack_delay_remote;
        }
    }
}
```

### Rust body
```rust
    pub fn ready_state_transition(&mut self, current_time: Instant) {
        self.connection_state = crate::State::Ready;
        self.is_handshake_finished = true;
        self.implicit_handshake_ack(PacketContext::Initial, current_time);
        self.implicit_handshake_ack(PacketContext::Handshake, current_time);
        if !self.client_mode {
            let _ = self.queue_handshake_done_frame();
        }
        if self.is_half_open {
            self.is_half_open = false;
        }
        self.purge_misc_frames_after_ready();
        if self.is_ack_frequency_negotiated {
            self.is_ack_frequency_updated = true;
        } else {
            let rtt = self.paths.first().map(|p| p.rtt_min).unwrap_or(INITIAL_RTT);
            let rate = self.paths.first().map(|p| p.receive_rate_max).unwrap_or(0);
            let mut ack_gap = 0;
            let mut ack_delay = 0;
            self.compute_ack_gap_and_delay(
                rtt,
                ACK_DELAY_MIN.ticks(),
                rate,
                &mut ack_gap,
                &mut ack_delay,
            );
            self.ack_gap_remote = ack_gap;
            self.ack_delay_remote = Duration::from_ticks(ack_delay);
            self.max_ack_gap_remote = self.max_ack_gap_remote.max(ack_gap);
            self.max_ack_delay_remote = self
                .max_ack_delay_remote
                .max(Duration::from_ticks(ack_delay));
            self.min_ack_delay_remote = self
                .min_ack_delay_remote
                .min(Duration::from_ticks(ack_delay));
        }
    }
```

## `picoquic/siphash.c:siphash`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The shown Rust body only defines sip_round and lacks the visible SipHash processing, finalization, output writing, and return logic from C.
* C source: `picoquic/siphash.c:85-189`
* C signature: `int siphash(const void *, const size_t, const void *, uint8_t *, const size_t)`
* Rust source: `rs/fq/src/siphash.rs:22-38`
* Rust item: `siphash`

### C body
```c
            const size_t outlen) {

    const unsigned char *ni = (const unsigned char *)in;
    const unsigned char *kk = (const unsigned char *)k;

    assert((outlen == 8) || (outlen == 16));
    uint64_t v0 = UINT64_C(0x736f6d6570736575);
    uint64_t v1 = UINT64_C(0x646f72616e646f6d);
    uint64_t v2 = UINT64_C(0x6c7967656e657261);
    uint64_t v3 = UINT64_C(0x7465646279746573);
    uint64_t k0 = U8TO64_LE(kk);
    uint64_t k1 = U8TO64_LE(kk + 8);
    uint64_t m;
    int i;
    const unsigned char *end = ni + inlen - (inlen % sizeof(uint64_t));
    const int left = inlen & 7;
    uint64_t b = ((uint64_t)inlen) << 56;
    v3 ^= k1;
    v2 ^= k0;
    v1 ^= k1;
    v0 ^= k0;

    if (outlen == 16)
        v1 ^= 0xee;

    for (; ni != end; ni += 8) {
        m = U8TO64_LE(ni);
        v3 ^= m;

        TRACE;
        for (i = 0; i < cROUNDS; ++i)
            SIPROUND;

        v0 ^= m;
    }

    switch (left) {
    case 7:
        b |= ((uint64_t)ni[6]) << 48;
        /* FALLTHRU */
    case 6:
        b |= ((uint64_t)ni[5]) << 40;
        /* FALLTHRU */
    case 5:
        b |= ((uint64_t)ni[4]) << 32;
        /* FALLTHRU */
    case 4:
        b |= ((uint64_t)ni[3]) << 24;
        /* FALLTHRU */
    case 3:
        b |= ((uint64_t)ni[2]) << 16;
        /* FALLTHRU */
    case 2:
        b |= ((uint64_t)ni[1]) << 8;
        /* FALLTHRU */
    case 1:
        b |= ((uint64_t)ni[0]);
        break;
    case 0:
        break;
    }

    v3 ^= b;

    TRACE;
    for (i = 0; i < cROUNDS; ++i)
        SIPROUND;

    v0 ^= b;

    if (outlen == 16)
        v2 ^= 0xee;
    else
        v2 ^= 0xff;

    TRACE;
    for (i = 0; i < dROUNDS; ++i)
        SIPROUND;

    b = v0 ^ v1 ^ v2 ^ v3;
    U64TO8_LE(out, b);

    if (outlen == 8)
        return 0;

    v1 ^= 0xdd;

    TRACE;
    for (i = 0; i < dROUNDS; ++i)
        SIPROUND;

    b = v0 ^ v1 ^ v2 ^ v3;
    U64TO8_LE(out + 8, b);

    return 0;
}
```

### Rust body
```rust
    fn sip_round(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
        *v0 = v0.wrapping_add(*v1);
        *v1 = v1.rotate_left(13);
        *v1 ^= *v0;
        *v0 = v0.rotate_left(32);
        *v2 = v2.wrapping_add(*v3);
        *v3 = v3.rotate_left(16);
        *v3 ^= *v2;
        *v0 = v0.wrapping_add(*v3);
        *v3 = v3.rotate_left(21);
        *v3 ^= *v0;
        *v2 = v2.wrapping_add(*v1);
        *v1 = v1.rotate_left(17);
        *v1 ^= *v2;
        *v2 = v2.rotate_left(32);
    }
```

## `picoquic/spinbit.c:picoquic_spinbit_random_incoming`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C incoming is a no-op; Rust body is outgoing logic that returns a random spin bit.
* C source: `picoquic/spinbit.c:65-70`
* C signature: `void picoquic_spinbit_random_incoming(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_header *)`
* Rust source: `rs/fq/src/spinbit.rs:68-75`
* Rust item: `incoming`

### C body
```c
{
    UNREFERENCED_PARAMETER(cnx);
    UNREFERENCED_PARAMETER(path_x);
    UNREFERENCED_PARAMETER(ph);
}
```

### Rust body
```rust
    fn outgoing(&self, _connection: &mut Connection) -> u8 {
        // C: `(uint8_t)(picoquic_public_random_64() & 0x20)` — bit 5 is the
        // spin-bit position in the QUIC short header first byte.
        crate::public_random_64() as u8 & 0x20
    }
```
