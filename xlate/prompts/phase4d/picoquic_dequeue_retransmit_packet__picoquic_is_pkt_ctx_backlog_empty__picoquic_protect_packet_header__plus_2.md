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

## `picoquic/sender.c:picoquic_dequeue_retransmit_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body removes/requeues/accounts a packet and returns it; Rust body shown is only a small lookup expression.
* C source: `picoquic/sender.c:1033-1105`
* C signature: `picoquic_packet_t * picoquic_dequeue_retransmit_packet(picoquic_cnx_t *, picoquic_packet_context_t *, picoquic_packet_t *, int, int)`
* Rust source: `rs/fq/src/internal.rs:5609-5625`
* Rust item: `dequeue_retransmit_packet`

### C body
```c
{
    size_t dequeued_length = p->length + p->checksum_overhead;

    if (p->is_queued_for_retransmit) {
        /* Remove from list */
        if (p->packet_next == NULL) {
            pkt_ctx->pending_last = p->packet_previous;
        }
        else {
            p->packet_next->packet_previous = p->packet_previous;
        }

        if (p->packet_previous == NULL) {
            pkt_ctx->pending_first = p->packet_next;
        }
        else {
            p->packet_previous->packet_next = p->packet_next;
        }
        p->is_queued_for_retransmit = 0;
    }

    /* Account for bytes in transit, for congestion control */

    if (p->send_path != NULL && !p->is_ack_trap) {
        if (p->send_path->bytes_in_transit > dequeued_length) {
            p->send_path->bytes_in_transit -= dequeued_length;
        }
        else {
            p->send_path->bytes_in_transit = 0;
        }
        p->send_path->is_cc_data_updated = 1;
    }

    /* Replace head of preemptive repeat list if it was this packet. */
    if (pkt_ctx->preemptive_repeat_ptr == p) {
        pkt_ctx->preemptive_repeat_ptr = p->packet_next;
    }

    if (should_free || p->is_ack_trap) {
        if (add_to_data_repeat_queue) {
            picoquic_queue_data_repeat_packet(cnx, p);
        }
        else {
            picoquic_recycle_packet(cnx->quic, p);
            p = NULL;
        }
    } 
    else {
        p->packet_previous = NULL;
        /* add this packet to the retransmitted list */
        if (pkt_ctx->retransmitted_oldest == NULL) {
            pkt_ctx->retransmitted_newest = p;
            pkt_ctx->retransmitted_oldest = p;
            p->packet_next = NULL;
        }
        else {
            pkt_ctx->retransmitted_newest->packet_previous = p;
            p->packet_next = pkt_ctx->retransmitted_newest;
            pkt_ctx->retransmitted_newest = p;
        }
        pkt_ctx->retransmitted_queue_size += 1;
        p->is_queued_for_spurious_detection = 1;

        if (add_to_data_repeat_queue) {
            picoquic_queue_data_repeat_packet(cnx, p);
        }
    }

    return p;
}
```

### Rust body
```rust
            .or_else(|| {
                pkt_ctx
                    .pending
                    .iter()
                    .find_map(|(seq, tok)| (*tok == packet).then_some(*seq))
            })?;
```

## `picoquic/sender.c:picoquic_is_pkt_ctx_backlog_empty`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C scans pending packets and frames to return whether backlog is ACK-only; shown Rust inserts sequence holes and has no backlog-empty return.
* C source: `picoquic/sender.c:1274-1308`
* C signature: `int picoquic_is_pkt_ctx_backlog_empty(picoquic_packet_context_t *)`
* Rust source: `rs/fq/src/internal.rs:17088-17141`
* Rust item: `picoquic_is_pkt_ctx_backlog_empty`

### C body
```c
{
    int backlog_empty = 1;
    picoquic_packet_t* p = pkt_ctx->pending_first;

    while (p != NULL && backlog_empty == 1) {
        /* check if this is an ACK only packet */
        int ret = 0;
        int frame_is_pure_ack = 0;
        size_t frame_length = 0;
        size_t byte_index = 0; /* Used when parsing the old packet */

        byte_index = p->offset;

        if (!p->is_ack_trap && !p->is_multipath_probe && !p->is_mtu_probe) {
            while (ret == 0 && byte_index < p->length) {
                ret = picoquic_skip_frame(&p->bytes[byte_index],
                    p->length - p->offset, &frame_length, &frame_is_pure_ack);

                if (!frame_is_pure_ack) {
                    backlog_empty = 0;
                    break;
                }
                byte_index += frame_length;
            }
        }

        p = p->packet_next;
    }

    return backlog_empty;
}
```

### Rust body
```rust
    ) {
        let period = self.sequence_hole_pseudo_period();
        if period == 0 {
            pkt_ctx.next_sequence_hole = u64::MAX;
            return;
        }
        if self.connection_state == State::Ready
            && pkt_ctx.pending.values().next_back().is_some()
            && pkt_ctx.send_sequence >= pkt_ctx.next_sequence_hole
        {
            let pending_last_is_trap = pkt_ctx
                .pending
                .values()
                .next_back()
                .and_then(|tok| self.queued_packets.get(*tok))
                .map(|packet| packet.is_ack_trap)
                .unwrap_or(false);
            if pkt_ctx.next_sequence_hole != 0 && !pending_last_is_trap {
                let mut packet = Self::empty_sender_packet(current_time);
                packet.is_ack_trap = true;
                packet.packet_context = PacketContext::Application;
                packet.packet_type = PacketType::OneRttProtected;
                packet.send_time = current_time;
                packet.send_path = None;
                packet.sequence_number = pkt_ctx.send_sequence;
                pkt_ctx.send_sequence = pkt_ctx.send_sequence.saturating_add(1);
                self.queue_for_retransmit(path_x, &mut packet, 0, current_time);
                self.set_sender_wake_now(next_wake_time, current_time);
                path_x.q_square = path_x.q_square.saturating_add(1);
                self.nb_packet_holes_inserted = self.nb_packet_holes_inserted.saturating_add(1);
            }
            let random_bound = (period as u64)
                .checked_shl(self.nb_packet_holes_inserted.min(63) as u32)
                .unwrap_or(u64::MAX);
            pkt_ctx.next_sequence_hole = pkt_ctx
                .send_sequence
                .saturating_add(3)
                .saturating_add(public_uniform_random(random_bound.max(1)));
        }
    }
```

## `picoquic/sender.c:picoquic_protect_packet_header`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C encrypts a sample, masks the first byte, and masks packet number bytes; shown Rust only returns if pn_offset is out of bounds.
* C source: `picoquic/sender.c:874-895`
* C signature: `void picoquic_protect_packet_header(uint8_t *, size_t, uint8_t, void *)`
* Rust source: `rs/fq/src/internal.rs:6985-6993`
* Rust item: `protect_packet_header`

### C body
```c
{
    /* The sample is located after the pn_offset */
    size_t sample_offset = /* header_length */ pn_offset + 4;

    if (pn_offset < sample_offset)
    {
        /* This is always true, as we use pn_length = 4 */
        uint8_t mask_bytes[5] = { 0, 0, 0, 0, 0 };
        uint8_t pn_l;

        picoquic_pn_encrypt(pn_enc, send_buffer + sample_offset, mask_bytes, mask_bytes, 5);
        /* Encode the first byte */
        pn_l = (send_buffer[0] & 3) + 1;
        send_buffer[0] ^= (mask_bytes[0] & first_mask);

        /* Packet encoding is 1 to 4 bytes */
        for (uint8_t i = 0; i < pn_l; i++) {
            send_buffer[pn_offset+i] ^= mask_bytes[i+1];
        }
    }
}
```

### Rust body
```rust
    if pn_offset >= send_buffer.len() {
        return;
    }
```

## `picoquic/sim_link.c:picoquictest_sim_link_transmit_time`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C computes transmit time from packet length and picoseconds per byte; Rust body computes queue delay from queue_time and current_time.
* C source: `picoquic/sim_link.c:296-299`
* C signature: `uint64_t picoquictest_sim_link_transmit_time(picoquictest_sim_link_t *, picoquictest_sim_packet_t *)`
* Rust source: `rs/fq/src/tests/util.rs:497-507`
* Rust item: `transmit_time`

### C body
```c
{
    return ((link->picosec_per_byte * ((uint64_t)packet->length)) >> 20);
}
```

### Rust body
```rust
    pub fn queue_delay(&mut self, current_time: Instant) -> u64 {
        let qt = self.queue_time.ticks();
        let ct = current_time.ticks();
        qt.saturating_sub(ct)
    }
```

## `picoquic/spinbit.c:picoquic_spinbit_null_incoming`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C incoming body does nothing and returns no value; Rust body shown is outgoing and returns 0.
* C source: `picoquic/spinbit.c:48-53`
* C signature: `void picoquic_spinbit_null_incoming(picoquic_cnx_t *, picoquic_path_t *, picoquic_packet_header *)`
* Rust source: `rs/fq/src/spinbit.rs:50-55`
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
        0
    }
```
