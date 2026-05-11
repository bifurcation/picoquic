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

## `picoquic/quicctx.c:picoquic_get_callback_function`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns callback_fn; Rust body returns callback_ctx.
* C source: `picoquic/quicctx.c:4785-4789`
* C signature: `picoquic_stream_data_cb_fn picoquic_get_callback_function(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:3100-3108`
* Rust item: `callback`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->callback_fn;
}
```

### Rust body
```rust
    pub fn callback_ctx(&self) -> Option<&dyn core::any::Any> {
        self.callback_ctx.as_deref()
    }
```

## `picoquic/quicctx.c:picoquic_get_data_sent`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns data_sent; Rust body returns data_received.
* C source: `picoquic/quicctx.c:5531-5535`
* C signature: `uint64_t picoquic_get_data_sent(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:4525-4532`
* Rust item: `data_sent`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->data_sent;
}
```

### Rust body
```rust
    pub fn data_received(&self) -> u64 {
        self.data_received
    }
```

## `picoquic/quicctx.c:picoquic_get_local_cnxid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C unconditionally returns the CID from path[0]->first_tuple->p_local_cnxid, ignoring parameters, while Rust returns an Option token from find_local_connection_id(unique_path_id, connection_id). The visible behavior and return value are clearly different.
* C source: `picoquic/quicctx.c:4459-4463`
* C signature: `picoquic_connection_id_t picoquic_get_local_cnxid(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/lib.rs:2953-2971`
* Rust item: `local_cnxid`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    return cnx->path[0]->first_tuple->p_local_cnxid->cnx_id;
}
```

### Rust body
```rust
    ) -> Option<crate::internal::LocalConnectionIdToken> {
        self.find_local_connection_id(unique_path_id, connection_id)
    }
```

## `picoquic/quicctx.c:picoquic_get_quic_time`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns current or simulated time; Rust body contains unrelated connection/callback/policy methods and no quic time return.
* C source: `picoquic/quicctx.c:4618-4630`
* C signature: `uint64_t picoquic_get_quic_time(picoquic_quic_t *)`
* Rust source: `rs/fq/src/lib.rs:3158-3253`
* Rust item: `quic_time`

### C body
```c
{
    uint64_t now;
    PICOQUIC_THREAD_CHECK(quic);
    if (quic->p_simulated_time == NULL) {
        now = picoquic_current_time();
    }
    else {
        now = *quic->p_simulated_time;
    }

    return now;
}
```

### Rust body
```rust
impl Quic {
    /// Borrow the first connection registered with this context.
    pub fn first_connection(&mut self) -> Option<&mut Connection> {
        self.connections.iter_mut().next()
    }

    /// Return the connection that follows the one identified by `current_token`
    /// in arena insertion order, or `None` when `current_token` is the last
    /// live connection.
    ///
    /// C: `picoquic_get_next_cnx` — `cnx->next_in_table`.
    ///
    /// The C intrusive linked list (`next_in_table` / `previous_in_table`) is
    /// replaced by an arena; this method scans forward from the slot after
    /// `current_token.idx` to find the next occupied slot.  Typical usage:
    ///
    /// ```ignore
    /// let mut tok = quic.first_connection().and_then(|c| c.own_token);
    /// while let Some(t) = tok {
    ///     let cnx = quic.connections.get_mut(t).unwrap();
    ///     // ... process cnx ...
    ///     tok = quic.next_cnx(t).and_then(|c| c.own_token);
    /// }
    /// ```
    pub fn next_cnx(&mut self, current_token: ConnectionToken) -> Option<&mut Connection> {
        let next_idx = current_token.slot_idx() + 1;
        self.connections.next_after_idx(next_idx)
    }

    /// Compute the number of microseconds until *any* connection on
    /// this context next needs attention, capped at `delay_max`.
    pub fn next_wake_delay(&self, current_time: Instant, delay_max: i64) -> i64 {
        let now = current_time.ticks();
        // Find the minimum next_wake_time across all connections.
        let earliest = self
            .connections
            .iter()
            .map(|c| c.next_wake_time.ticks())
            .min();
        match earliest {
            None => delay_max,
            Some(t) if t <= now => 0,
            Some(t) => ((t - now) as i64).min(delay_max),
        }
    }

    /// Wall-clock time at which the next event is scheduled.
    pub fn next_wake_time(&self, current_time: Instant) -> u64 {
        let now = current_time.ticks();
        self.connections
            .iter()
            .map(|c| c.next_wake_time.ticks())
            .min()
            .unwrap_or(now)
    }

    /// Return the earliest connection that wakes before `wake_time`,
    /// or `None` if none qualify.
    /// C: `picoquic_get_earliest_cnx_to_wake`.
    pub fn earliest_cnx_to_wake(&mut self, wake_time: Instant) -> Option<&mut Connection> {
        let threshold = wake_time.ticks();
        self.connections
            .iter_mut()
            .filter(|c| c.next_wake_time.ticks() <= threshold)
            .min_by_key(|c| c.next_wake_time.ticks())
    }

    /// Borrow the connection currently advancing through its state
    /// machine, if any (`get_cnx_in_progress` in C).
    pub fn connection_in_progress(&mut self) -> Option<&mut Connection> {
        let tok = self.connection_in_progress?;
        self.connections.get_mut(tok)
    }

    /// Default PMTUD policy applied to new connections.
    pub fn set_default_pmtud_policy(&mut self, pmtud_policy: PmtudPolicy) {
        self.default_pmtud_policy = pmtud_policy;
    }

    /// Borrow the default stream callback installed on this context.
    /// C: `picoquic_get_default_callback_function` — `quic->default_callback_fn`.
    pub fn default_callback(&self) -> Option<&dyn StreamDataCallback> {
        self.default_callback_fn.as_deref()
    }

    /// Borrow the opaque context associated with the default callback.
    /// C: `picoquic_get_default_callback_context` — `quic->default_callback_ctx`.
    pub fn default_callback_ctx(&self) -> Option<&dyn core::any::Any> {
        self.default_callback_ctx.as_deref()
    }
}
```

## `picoquic/quicctx.c:picoquic_init_packet_ctx`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown is incomplete/truncated and omits visible C initialization of pending fields and acknowledgement timestamps.
* C source: `picoquic/quicctx.c:2875-2890`
* C signature: `void picoquic_init_packet_ctx(picoquic_cnx_t *, picoquic_packet_context_t *, picoquic_packet_context_enum)`
* Rust source: `rs/fq/src/internal.rs:8758-8769`
* Rust item: `init_packet_ctx`

### C body
```c
{
    if (cnx->quic->random_initial && 
        (pc == picoquic_packet_context_initial || cnx->quic->random_initial > 1)){
        pkt_ctx->send_sequence = picoquic_crypto_uniform_random(cnx->quic, PICOQUIC_PN_RANDOM_RANGE) +
            PICOQUIC_PN_RANDOM_MIN;
    }
    else {
        pkt_ctx->send_sequence = 0;
    }
    pkt_ctx->pending_last = NULL;
    pkt_ctx->pending_first = NULL;
    pkt_ctx->highest_acknowledged = pkt_ctx->send_sequence - 1;
    pkt_ctx->latest_time_acknowledged = cnx->start_time;
    pkt_ctx->highest_acknowledged_time = cnx->start_time;
}
```

### Rust body
```rust
        if self.random_initial != 0 && (pc == PacketContext::Initial || self.random_initial > 1) {
            let mut rnd = [0u8; 8];
            if fill_system_random(&mut rnd).is_ok() {
                pkt_ctx.send_sequence =
                    u64::from_le_bytes(rnd) % PN_RANDOM_RANGE as u64 + PN_RANDOM_MIN as u64;
            } else {
                pkt_ctx.send_sequence =
                    crate::public_random_64() % PN_RANDOM_RANGE as u64 + PN_RANDOM_MIN as u64;
            }
        } else {
```
