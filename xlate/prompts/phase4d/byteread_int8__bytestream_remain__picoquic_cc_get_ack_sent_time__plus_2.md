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

## `picoquic/bytestream.c:byteread_int8`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C reads one byte, advances ptr, and returns success; Rust shown only performs the empty-buffer error check.
* C source: `picoquic/bytestream.c:188-197`
* C signature: `int byteread_int8(bytestream *, uint8_t *)`
* Rust source: `rs/fq/src/bytestream.rs:258-262`
* Rust item: `read_u8`

### C body
```c
{
    size_t max_bytes = s->size - s->ptr;
    if (max_bytes < 1) {
        return bytestream_error(s);
    } else {
        *value = s->data[s->ptr++];
        return 0;
    }
}
```

### Rust body
```rust
        if self.ptr >= self.data_ref().len() {
            self.set_error();
            return Err(Error::BufferTooSmall);
        }
```

## `picoquic/bytestream.c:bytestream_remain`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns size minus ptr; Rust resets ptr to zero and returns nothing.
* C source: `picoquic/bytestream.c:93-96`
* C signature: `size_t bytestream_remain(bytestream *)`
* Rust source: `rs/fq/src/bytestream.rs:208-216`
* Rust item: `remaining`

### C body
```c
{
    return s->size - s->ptr;
}
```

### Rust body
```rust
    pub fn reset(&mut self) {
        self.ptr = 0;
    }
```

## `picoquic/cc_common.c:picoquic_cc_get_ack_sent_time`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns latest_time_acknowledged, choosing path or connection context by multipath flag; Rust returns send_sequence from similarly selected contexts.
* C source: `picoquic/cc_common.c:63-75`
* C signature: `uint64_t picoquic_cc_get_ack_sent_time(picoquic_cnx_t *, picoquic_path_t *)`
* Rust source: `rs/fq/src/cc_common.rs:275-285`
* Rust item: `ack_sent_time`

### C body
```c
{
    uint64_t latest_time_acknowledged;

    if (cnx->is_multipath_enabled) {
        latest_time_acknowledged = path_x->pkt_ctx.latest_time_acknowledged;
    }
    else {
        latest_time_acknowledged = cnx->pkt_ctx[picoquic_packet_context_application].latest_time_acknowledged;
    }

    return latest_time_acknowledged;
}
```

### Rust body
```rust
    fn sequence_number(&self, path_x: &Path) -> u64 {
        if self.is_multipath_enabled {
            path_x.pkt_ctx.send_sequence
        } else {
            self.pkt_ctx[crate::PacketContext::Application as usize].send_sequence
        }
    }
```

## `picoquic/config.c:picoquic_config_get_option_char_index`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C searches option letters by opt; Rust function searches option names by string prefix and has a different function name.
* C source: `picoquic/config.c:641-652`
* C signature: `int picoquic_config_get_option_char_index(int)`
* Rust source: `rs/fq/src/config.rs:678-697`
* Rust item: `picoquic_config_get_option_char_index`

### C body
```c
{
    int option_index = -1;

    for (size_t i = 0; i < option_table_size; i++) {
        if (option_table[i].option_letter == opt) {
            option_index = (int)i;
            break;
        }
    }
    return option_index;
}
```

### Rust body
```rust
pub fn picoquic_config_get_option_name_index(s: &str, l: usize) -> i32 {
    let l = l.min(s.len());
    let prefix = &s[..l];
    OPTION_TABLE
        .iter()
        .position(|e| e.name.len() >= l && &e.name[..l] == prefix)
        .map_or(-1, |i| i as i32)
}
```

## `picoquic/dualq_aqm.c:dualq_submit`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown is a test function looping over packets and assertions, not the submit implementation body that enqueues and updates.
* C source: `picoquic/dualq_aqm.c:360-372`
* C signature: `void dualq_submit(picoquictest_aqm_t *, picoquictest_sim_link_t *, picoquictest_sim_packet_t *, uint64_t)`
* Rust source: `rs/fq/src/tests/dualq_aqm.rs:182-214`
* Rust item: `dualq_submit`

### C body
```c
{
    dualq_state_t* dualq = (dualq_state_t*)self;

    /* queue the packet. */
    dualq_enqueue(dualq, link, packet, current_time);

    /* submit data if possible, and compute the new value of pi2 parameters if it is time */
    dualq->last_input_time = current_time;
    dualq_update_it(dualq, link, current_time);
}
```

### Rust body
```rust
fn dualq_submit() -> crate::Result<()> {
    let mut ctx = dualq_test_get_ctx()?;
    let mut one_was_dropped = false;

    with_dualq(&mut ctx.link, |dualq, link| {
        for i in 0..50 {
            let i_queue = i % ECN_SEQUENCE.len();
            let packet = dualq_test_get_packet(ECN_SEQUENCE[i_queue], 1000)?;
            let old_bytes = queue_bytes(dualq, QUEUE_ID[i_queue]);
            let old_queue_time = link.queue_time;
            let old_total = dualq.cq.queue_bytes + dualq.lq.queue_bytes + packet.length as u64;

            dualq.submit(link, packet, ctx.simulated_time);

            if old_queue_time.ticks() <= ctx.simulated_time.ticks() {
                check(link.queue_time != old_queue_time)?;
            } else {
                check(link.queue_time == old_queue_time)?;
                if old_total > dualq.limit {
                    check(queue_bytes(dualq, QUEUE_ID[i_queue]) == old_bytes)?;
                    one_was_dropped = true;
                    break;
                } else {
                    check(queue_bytes(dualq, QUEUE_ID[i_queue]) != old_bytes)?;
                    dualq_test_check_queue(link)?;
                }
            }
        }
        Ok(())
    })?;

    check(one_was_dropped)
}
```
