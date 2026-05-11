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

## `picoquic/bytestream.c:bytestream_clear`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C resets ptr and zeroes existing data, while Rust clears an arena slots/free/len structure and does not zero bytestream data.
* C source: `picoquic/bytestream.c:103-107`
* C signature: `void bytestream_clear(bytestream *)`
* Rust source: `rs/fq/src/arena.rs:208-212`
* Rust item: `clear`

### C body
```c
{
    s->ptr = 0;
    memset(s->data, 0, s->size);
}
```

### Rust body
```rust
    pub fn clear(&mut self) {
        self.slots.clear();
        self.free = None;
        self.len = 0;
    }
```

## `picoquic/bytestream.c:bytestream_vint_len`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns a varint encoded length for value; Rust body is an empty test module placeholder.
* C source: `picoquic/bytestream.c:163-166`
* C signature: `size_t bytestream_vint_len(uint64_t)`
* Rust source: `rs/fq/src/bytestream.rs:656-661`
* Rust item: `bytestream_vint_len`

### C body
```c
{
    return picoquic_encode_varint_length(value);
}
```

### Rust body
```rust
mod test {}
```

## `picoquic/cc_common.c:picoquic_cc_update_cwin_for_long_rtt`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown is lowest_not_ack packet-context logic, not long-RTT congestion-window calculation.
* C source: `picoquic/cc_common.c:272-289`
* C signature: `uint64_t picoquic_cc_update_cwin_for_long_rtt(picoquic_path_t *)`
* Rust source: `rs/fq/src/cc_common.rs:341-355`
* Rust item: `update_cwin_for_long_rtt`

### C body
```c
uint64_t picoquic_cc_update_cwin_for_long_rtt(picoquic_path_t * path_x) {
    uint64_t min_cwnd;

    if (path_x->rtt_min > PICOQUIC_TARGET_SATELLITE_RTT) {
        min_cwnd = (uint64_t)((double)PICOQUIC_CWIN_INITIAL * (double)PICOQUIC_TARGET_SATELLITE_RTT / (double)PICOQUIC_TARGET_RENO_RTT);
    }
    else {
        min_cwnd = (uint64_t)((double)PICOQUIC_CWIN_INITIAL * (double)path_x->rtt_min / (double)PICOQUIC_TARGET_RENO_RTT);
    }

    /* Return increased cwin, if larger than current cwin. */
    if (min_cwnd > path_x->cwin) {
        return min_cwnd;
    }

    /* Otherwise, return current cwin. */
    return path_x->cwin;
}
```

### Rust body
```rust
    fn lowest_not_ack(&self) -> u64 {
        // C reads cnx->pkt_ctx[app] for single-path, path->pkt_ctx for multipath.
        // Path has no back-pointer to Connection, so we always use path->pkt_ctx
        // (exact for multipath; conservative approximation for single-path).
        self.pkt_ctx
            .pending
            .keys()
            .next()
            .copied()
            .unwrap_or(self.pkt_ctx.highest_acknowledged + 1)
    }
```

## `picoquic/dualq_aqm.c:dualq_enqueue`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body checks total queue limit, may drop to link, stamps arrival_time, and classifies by ECN; Rust body is a test that directly enqueues into selected queues.
* C source: `picoquic/dualq_aqm.c:142-167`
* C signature: `void dualq_enqueue(dualq_state_t *, picoquictest_sim_link_t *, picoquictest_sim_packet_t *, uint64_t)`
* Rust source: `rs/fq/src/tests/dualq_aqm.rs:117-144`
* Rust item: `dualq_enqueue`

### C body
```c
{
    /* Test limit and classify lq or cq */
    if (dualq->cq.queue_bytes + dualq->lq.queue_bytes + packet->length > dualq->limit)
    {
        /* drop packet if buffer is full */
        picoquictest_sim_link_enqueue(link, packet, 0, 1);
    }
    else {
        /* 4 : timestamp(pkt) % only needed if using the sojourn technique */
        packet->arrival_time = current_time;
        /* Packet classifier */
        if (packet->ecn_mark == PICOQUIC_ECN_ECT_1 ||
            packet->ecn_mark == PICOQUIC_ECN_CE) {
            /* Add to L4S queue */
            dualq_enqueue_queue(&dualq->lq, packet);
        }
        else
        {
            /* add to classic queue */
            dualq_enqueue_queue(&dualq->cq, packet);
        }
    }
}
```

### Rust body
```rust
fn dualq_enqueue() -> crate::Result<()> {
    let mut ctx = dualq_test_get_ctx()?;

    with_dualq(&mut ctx.link, |dualq, link| {
        for i in 0..ECN_SEQUENCE.len() {
            let packet = dualq_test_get_packet(ECN_SEQUENCE[i], 1000)?;
            let new_key = packet_key(&packet);
            let link_queue_len = link.packets.len();
            let xq = queue_for_mut(dualq, QUEUE_ID[i]);
            let old_bytes = xq.queue_bytes;
            let old_len = xq.packets.len();
            let old_front = xq.packets.front().map(packet_key);

            xq.enqueue(packet);

            check(xq.packets.len() == old_len + 1)?;
            check(xq.queue_bytes == old_bytes + 1000)?;
            check(xq.packets.back().map(packet_key) == Some(new_key))?;
            if old_len == 0 {
                check(xq.packets.front().map(packet_key) == Some(new_key))?;
            } else {
                check(xq.packets.front().map(packet_key) == old_front)?;
            }
            check(link.packets.len() == link_queue_len)?;
        }
        Ok(())
    })
}
```

## `picoquic/ech.c:picoquic_ech_create_config_from_private_key`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body calls tls_api_init only, while C extracts a public key, infers group id from key length, creates a config, wraps it in a config list, frees key bytes, and returns status.
* C source: `picoquic/ech.c:879-925`
* C signature: `int picoquic_ech_create_config_from_private_key(uint8_t **, size_t *, const char *, const char *)`
* Rust source: `rs/fq/src/lib.rs:4770-4781`
* Rust item: `ech_create_config_from_private_key`

### C body
```c
{
    int ret = 0;

    *config = NULL;
    *config_len = 0;

    if (picoquic_get_public_key_from_private_fn != NULL) {
        uint16_t group_id = 0;
        ptls_iovec_t public_key_bits = ptls_iovec_init(NULL, 0);

        ret = picoquic_get_public_key_from_private_fn(private_key_file, &public_key_bits.base, &public_key_bits.len);

        if (ret == 0) {
            switch (public_key_bits.len) {
            case 0x21: /* x25519 */
                group_id = 0x001d;
                break;
            case 0x41: /* secp265r1 */
                group_id = 0x0017;
                break;
            case 0x61: /* x25519 */
                group_id = 0x0018;
                break;
            default:
                DBG_PRINTF("Cannot find group ID from pubkey length 0x%02x from %s",
                    public_key_bits.len, private_key_file);
                ret = -1;
                break;
            }
        }

        if (ret == 0) {
            ptls_buffer_t config_buf;
            ptls_buffer_init(&config_buf, "", 0);
            ret = picoquic_ech_create_config_from_pk(&config_buf, group_id, public_key_bits, public_name);
            if (ret == 0) {
                ret = picoquic_ech_create_config_list_from_config(&config_buf, config, config_len);
            }
            ptls_buffer_dispose(&config_buf);
        }
        if (public_key_bits.base != NULL) {
            free(public_key_bits.base);
        }
    }
    return ret;
}
```

### Rust body
```rust
pub fn tls_api_init() {
    crate::tls_api::tls_api_init();
}
```
