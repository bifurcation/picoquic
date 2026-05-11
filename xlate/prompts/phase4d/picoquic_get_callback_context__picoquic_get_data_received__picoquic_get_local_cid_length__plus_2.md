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

## `picoquic/quicctx.c:picoquic_get_callback_context`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns callback_ctx; Rust appends a misc frame and returns Ok.
* C source: `picoquic/quicctx.c:4791-4795`
* C signature: `void * picoquic_get_callback_context(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3106-3124`
* Rust item: `callback_ctx`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->callback_ctx;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        use crate::internal::MiscFrameHeader;
        self.misc_frames.push_back(MiscFrameHeader {
            bytes: bytes.to_vec(),
            packet_context: pc,
            is_pure_ack: is_pure_ack as i32,
        });
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_get_data_received`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns data_received; Rust computes whether logging is still active.
* C source: `picoquic/quicctx.c:5537-5541`
* C signature: `uint64_t picoquic_get_data_received(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4530-4538`
* Rust item: `data_received`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->data_received;
}
```

### Rust body
```rust
    pub fn is_still_logging(&self) -> bool {
        self.nb_packets_logged < LOG_PACKET_MAX_SEQUENCE as u64 || self.use_long_log
    }
```

## `picoquic/quicctx.c:picoquic_get_local_cid_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns the local connection ID length; Rust checks whether a connection ID exists in a map and returns a bool.
* C source: `picoquic/quicctx.c:1031-1035`
* C signature: `uint8_t picoquic_get_local_cid_length(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:1803-1810`
* Rust item: `local_cid_length`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return quic->local_cnxid_length;
}
```

### Rust body
```rust
    pub fn is_local_cid(&self, cid: &ConnectionId) -> bool {
        self.connection_by_id.contains_key(cid)
    }
```

## `picoquic/quicctx.c:picoquic_get_peer_addr`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C writes the peer address from path 0; Rust returns the local address.
* C source: `picoquic/quicctx.c:4441-4445`
* C signature: `void picoquic_get_peer_addr(picoquic_cnx_t *, struct sockaddr **)`
* Rust source: `rs/fq/src/lib.rs:4955-4963`
* Rust item: `get_peer_addr`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    *addr = (struct sockaddr*)&cnx->path[0]->first_tuple->peer_addr;
}
```

### Rust body
```rust
    pub fn get_local_addr(&self) -> std::net::SocketAddr {
        self.local_addr()
    }
```

## `picoquic/quicctx.c:picoquic_get_wake_time`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns current_time if a pending stateless packet exists, otherwise cnx->next_wake_time; Rust finds a mutable connection with earliest wake time under a threshold.
* C source: `picoquic/quicctx.c:1579-1591`
* C signature: `uint64_t picoquic_get_wake_time(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:3222-3228`
* Rust item: `earliest_cnx_to_wake`

### C body
```c
{
    uint64_t wake_time = UINT64_MAX;
    PICOQUIC_THREAD_CHECK(cnx->quic);

    if (cnx->quic->pending_stateless_packet != NULL) {
        wake_time = current_time;
    } else {
        wake_time = cnx->next_wake_time;
    }

    return wake_time;
}
```

### Rust body
```rust
    pub fn earliest_cnx_to_wake(&mut self, wake_time: Instant) -> Option<&mut Connection> {
        let threshold = wake_time.ticks();
        self.connections
            .iter_mut()
            .filter(|c| c.next_wake_time.ticks() <= threshold)
            .min_by_key(|c| c.next_wake_time.ticks())
    }
```
