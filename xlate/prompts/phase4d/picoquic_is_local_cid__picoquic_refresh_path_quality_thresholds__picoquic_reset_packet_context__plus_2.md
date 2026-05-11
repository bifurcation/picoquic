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

## `picoquic/quicctx.c:picoquic_is_local_cid`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C checks CID length and connection lookup; Rust body is load_retry_tokens placeholder-like file/token stub.
* C source: `picoquic/quicctx.c:1037-1042`
* C signature: `int picoquic_is_local_cid(picoquic_quic_t *, picoquic_connection_id_t *)`
* Rust source: `rs/fq/src/lib.rs:1808-1816`
* Rust item: `is_local_cid`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    return (cid->id_len == quic->local_cnxid_length &&
        picoquic_cnx_by_id(quic, *cid, NULL) != NULL);
}
```

### Rust body
```rust
    pub fn load_retry_tokens(&mut self, _token_store_filename: &str) -> Result<(), Error> {
        // Complex: involves file I/O and token deserialization.
        Ok(())
    }
```

## `picoquic/quicctx.c:picoquic_refresh_path_quality_thresholds`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body shown is incomplete/truncated and does not show the threshold assignments present in C.
* C source: `picoquic/quicctx.c:2628-2658`
* C signature: `void picoquic_refresh_path_quality_thresholds(picoquic_path_t *)`
* Rust source: `rs/fq/src/internal.rs:6100-6103`
* Rust item: `refresh_quality_thresholds`

### C body
```c
{
    if (path_x->rtt_update_delta > 0) {
        if (path_x->smoothed_rtt > path_x->rtt_update_delta) {
            path_x->rtt_threshold_low = path_x->smoothed_rtt - path_x->rtt_update_delta;
        }
        else {
            path_x->rtt_threshold_low = 0;
        }
        path_x->rtt_threshold_high = path_x->smoothed_rtt + path_x->rtt_update_delta;
    }

    if (path_x->pacing_rate_update_delta > 0) {
        if (path_x->pacing.rate > path_x->pacing_rate_update_delta) {
            path_x->pacing_rate_threshold_low = path_x->pacing.rate - path_x->pacing_rate_update_delta;
        }
        else {
            path_x->pacing_rate_threshold_low = 0;
        }
        path_x->pacing_rate_threshold_high = path_x->pacing.rate + path_x->pacing_rate_update_delta;
        if (path_x->receive_rate_estimate > path_x->pacing_rate_update_delta) {
            path_x->receive_rate_threshold_low = path_x->receive_rate_estimate - path_x->pacing_rate_update_delta;
        }
        else {
            path_x->receive_rate_threshold_low = 0;
        }
        path_x->receive_rate_threshold_high = path_x->receive_rate_estimate + path_x->pacing_rate_update_delta;
    }
}
```

### Rust body
```rust
        let rtt = if self.smoothed_rtt.ticks() > 0 {
            self.smoothed_rtt
        } else {
```

## `picoquic/quicctx.c:picoquic_reset_packet_context`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C clears pending/retransmitted queues and resets ECN remote totals; Rust clears queues and resets sequence/queue-size fields instead.
* C source: `picoquic/quicctx.c:4905-4922`
* C signature: `void picoquic_reset_packet_context(picoquic_cnx_t *, picoquic_packet_context_t *)`
* Rust source: `rs/fq/src/internal.rs:5679-5687`
* Rust item: `reset_packet_context`

### C body
```c
{
    while (pkt_ctx->pending_last != NULL) {
        (void)picoquic_dequeue_retransmit_packet(cnx, pkt_ctx, pkt_ctx->pending_last, 1, 0);
    }
    
    while (pkt_ctx->retransmitted_newest != NULL) {
        picoquic_dequeue_retransmitted_packet(cnx, pkt_ctx, pkt_ctx->retransmitted_newest);
    }

    pkt_ctx->retransmitted_oldest = NULL;

    /* Reset the ECN data */
    pkt_ctx->ecn_ect0_total_remote = 0;
    pkt_ctx->ecn_ect1_total_remote = 0;
    pkt_ctx->ecn_ce_total_remote = 0;
}
```

### Rust body
```rust
    pub fn reset_packet_context(&mut self, pkt_ctx: &mut PacketContextState) {
        // C: picoquic_reset_packet_context
        pkt_ctx.pending.clear();
        pkt_ctx.retransmitted.clear();
        pkt_ctx.send_sequence = 0;
        pkt_ctx.retransmit_sequence = 0;
        pkt_ctx.next_sequence_hole = 0;
        pkt_ctx.retransmitted_queue_size = 0;
    }
```

## `picoquic/quicctx.c:picoquic_set_crypto_epoch_length`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body is a setter that assigns crypto_epoch_length_max with a default for zero; Rust body is a getter returning crypto_epoch_length_max.
* C source: `picoquic/quicctx.c:1017-1022`
* C signature: `void picoquic_set_crypto_epoch_length(picoquic_cnx_t *, uint64_t)`
* Rust source: `rs/fq/src/lib.rs:2865-2872`
* Rust item: `set_crypto_epoch_length`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(cnx->quic);
    cnx->crypto_epoch_length_max = (crypto_epoch_length_max == 0) ?
        PICOQUIC_DEFAULT_CRYPTO_EPOCH_LENGTH : crypto_epoch_length_max;
}
```

### Rust body
```rust
    pub fn crypto_epoch_length(&self) -> u64 {
        self.crypto_epoch_length_max
    }
```

## `picoquic/quicctx.c:picoquic_set_default_padding`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C sets padding minimum and multiple; Rust validates and sets default spin policy instead.
* C source: `picoquic/quicctx.c:899-904`
* C signature: `void picoquic_set_default_padding(picoquic_quic_t *, uint32_t, uint32_t)`
* Rust source: `rs/fq/src/lib.rs:1671-1687`
* Rust item: `set_default_padding`

### C body
```c
{
    PICOQUIC_THREAD_CHECK(quic);
    quic->padding_minsize_default = padding_minsize;
    quic->padding_multiple_default = padding_multiple;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        // SpinbitVersion::On is server-only and must not be set as a default.
        if default_spinbit_policy == SpinbitVersion::On {
            return Err(Error::InvalidArgument);
        }
        self.default_spin_policy = default_spinbit_policy;
        Ok(())
    }
```
