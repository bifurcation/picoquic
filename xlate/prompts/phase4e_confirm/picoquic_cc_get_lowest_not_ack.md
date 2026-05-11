# Phase 4E repair confirmation

This is a read-only re-triage after a Phase 4E repair or
repair-level `ok` claim.  Do not edit files.

For each entry, inspect directly relevant C and Rust context
and decide whether the current Rust translation is now
acceptable.

Report:

* `ok` when the current Rust behavior is acceptable.
* `needs_fix` when a real mismatch remains.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short confirmation conclusion","fix_summary":"remaining mismatch if any, or empty","files_changed":[],"verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/cc_common.c:picoquic_cc_get_lowest_not_ack`
* Phase 4C status: `suspect`
* Phase 4C rationale: The visible lowest_not_ack method matches the pending-or-highest+1 shape, but the Rust body shown is a larger trait impl with additional unrelated methods rather than only the target function.
* Prior Phase 4D analysis: Rust always reads path.pkt_ctx, but C reads connection.pkt_ctx[application] in single-path mode. The Rust c4 era_check caller has a Connection available, so the approximation can produce wrong single-path behavior.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: The Rust path-only method could not match C single-path behavior because C selects through path_x->cnx. The method now uses the provided Connection to choose the same packet context as C.
* Phase 4E fix summary: Changed PathCc::lowest_not_ack to accept &Connection, select path.pkt_ctx only for multipath and connection.pkt_ctx[Application] for single-path, and use wrapping_add(1) for C unsigned increment behavior.
* C source: `picoquic/cc_common.c:55-61`
* C signature: `uint64_t picoquic_cc_get_lowest_not_ack(picoquic_path_t *)`
* Current Rust source: `rs/fq/src/c4.rs:346-374`
* Current Rust item: `lowest_not_ack`

### C body
```c
{
    picoquic_packet_context_t* pkt_ctx = (path_x->cnx->is_multipath_enabled) ? &path_x->pkt_ctx : &path_x->cnx->pkt_ctx[picoquic_packet_context_application];
    uint64_t lowest_not_ack = (pkt_ctx->pending_first != NULL) ? pkt_ctx->pending_first->sequence_number : pkt_ctx->highest_acknowledged + 1;

    return lowest_not_ack;
}
```

### Current Rust body
```rust
    fn era_reset(&mut self, path_x: &Path, connection: &Connection) {
        self.era_sequence = connection.sequence_number(path_x);
        self.era_max_rtt = 0;
        self.era_min_rtt = u64::MAX;
        self.alpha_1024_previous = self.alpha_1024_current;
        self.update_ecn_alpha(path_x, connection);
    }
```
