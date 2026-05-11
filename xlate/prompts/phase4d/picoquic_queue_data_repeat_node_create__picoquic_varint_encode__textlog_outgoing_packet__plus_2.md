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

## `picoquic/frames.c:picoquic_queue_data_repeat_node_create`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the packet's queue_data_repeat_node address; Rust returns a PacketToken from a tree lookup by splay token.
* C source: `picoquic/frames.c:2129-2136`
* C signature: `picosplay_node_t * picoquic_queue_data_repeat_node_create(void *)`
* Rust source: `rs/fq/src/internal.rs:5698-5716`
* Rust item: `queue_data_repeat_node_create`

### C body
```c
{
    return &((picoquic_packet_t*)value)->queue_data_repeat_node;
}
```

### Rust body
```rust
    pub fn queue_data_repeat_node_value(&self, splay_tok: SplayToken) -> Option<PacketToken> {
        self.queue_data_repeat_tree.get(splay_tok).copied()
    }
```

## `picoquic/intformat.c:picoquic_varint_encode`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown only checks empty input and returns 0, missing the visible C varint encoding branches.
* C source: `picoquic/intformat.c:90-126`
* C signature: `size_t picoquic_varint_encode(uint8_t *, size_t, uint64_t)`
* Rust source: `rs/fq/src/internal.rs:6345-6350`
* Rust item: `varint_encode`

### C body
```c
{
    uint8_t* x = bytes;

    if (n64 < 16384) {
        if (n64 < 64) {
            if (max_bytes > 0) {
                *x++ = (uint8_t)(n64);
            }
        } else {
            if (max_bytes >= 2) {
                *x++ = (uint8_t)((n64 >> 8) | 0x40);
                *x++ = (uint8_t)(n64);
            }
        }
    } else if (n64 < 1073741824) {
        if (max_bytes >= 4) {
            *x++ = (uint8_t)((n64 >> 24) | 0x80);
            *x++ = (uint8_t)(n64 >> 16);
            *x++ = (uint8_t)(n64 >> 8);
            *x++ = (uint8_t)(n64);
        }
    } else {
        if (max_bytes >= 8) {
            *x++ = (uint8_t)((n64 >> 56) | 0xC0);
            *x++ = (uint8_t)(n64 >> 48);
            *x++ = (uint8_t)(n64 >> 40);
            *x++ = (uint8_t)(n64 >> 32);
            *x++ = (uint8_t)(n64 >> 24);
            *x++ = (uint8_t)(n64 >> 16);
            *x++ = (uint8_t)(n64 >> 8);
            *x++ = (uint8_t)(n64);
        }
    }

    return (x - bytes);
}
```

### Rust body
```rust
            if bytes.is_empty() {
                return 0;
            }
```

## `picoquic/logger.c:textlog_outgoing_packet`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body conditionally calls textlog_outgoing_segment when logging is enabled; the Rust body only returns early when not logging and has no visible outgoing-segment logging call.
* C source: `picoquic/logger.c:2298-2306`
* C signature: `void textlog_outgoing_packet(picoquic_cnx_t *, picoquic_path_t *, uint8_t *, uint64_t, size_t, size_t, uint8_t *, size_t, uint64_t)`
* Rust source: `rs/fq/src/logger.rs:643-654`
* Rust item: `outgoing_packet`

### C body
```c
{
    if (cnx->quic->F_log != NULL && picoquic_cnx_is_still_logging(cnx)) {
        textlog_outgoing_segment(cnx->quic->F_log, 1,
            cnx, bytes, sequence_number, length, send_buffer, send_length, pn_length);
    }
}
```

### Rust body
```rust
        if !self.is_still_logging() {
            return;
        }
```

## `picoquic/logwriter.c:binlog_picotls_ticket_ex`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: The C body writes to the binary log when binlog logging is active, while the Rust body shown forwards the ticket to text_log_fns and has no binlog check or binlog ticket write.
* C source: `picoquic/logwriter.c:1003-1009`
* C signature: `void binlog_picotls_ticket_ex(picoquic_cnx_t *, uint8_t *, uint16_t)`
* Rust source: `rs/fq/src/logger.rs:796-799`
* Rust item: `tls_ticket`

### C body
```c
{
    if (cnx != NULL && cnx->f_binlog != NULL && picoquic_cnx_is_still_logging(cnx)) {
        binlog_picotls_ticket(cnx->f_binlog, cnx->initial_cnxid, ticket, ticket_length);
    }
}
```

### Rust body
```rust
        if let Some(text) = logger_ref(&self.text_log_fns) {
            text.borrow_mut().tls_ticket(self, ticket);
        }
```

## `picoquic/loss_recovery.c:picoquic_set_wake_up_from_packet_retransmit`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C computes retransmit timing and updates next_wake_time; Rust body fragment only contains an else return and lacks the visible logic.
* C source: `picoquic/loss_recovery.c:646-662`
* C signature: `void picoquic_set_wake_up_from_packet_retransmit(picoquic_cnx_t *, picoquic_packet_t *, uint64_t, uint64_t *)`
* Rust source: `rs/fq/src/internal.rs:11537-11549`
* Rust item: `set_wake_up_from_packet_retransmit`

### C body
```c
{
    uint64_t next_retransmit_time = *next_wake_time;
    int is_timer_expired = 0;
    int is_probably_lost = picoquic_is_packet_probably_lost(cnx, old_p, current_time, &next_retransmit_time,
        &is_timer_expired);

    if (is_probably_lost || is_timer_expired) {
        *next_wake_time = current_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_LOSS_RECOVERY);
    }
    else if (next_retransmit_time < *next_wake_time) {
        *next_wake_time = next_retransmit_time;
        SET_LAST_WAKE(cnx->quic, PICOQUIC_LOSS_RECOVERY);
    }
}
```

### Rust body
```rust
        else {
            return;
        };
```
