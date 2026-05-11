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

## `picoquic/c4.c:c4_era_check`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns a boolean-like check based on connection state and lowest-not-ack; Rust resets era fields and ECN state with no returned check.
* Prior Phase 4D analysis: The actual Rust era_check is not era_reset, but deeper helper review found a real mismatch: it calls path_x.lowest_not_ack(), which always reads the path packet context, while C uses the connection application packet context when multipath is disabled.
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Confirmed mismatch: Rust era_check used the path-only lowest_not_ack helper, which cannot select the connection application packet context when multipath is disabled.
* Phase 4E fix summary: Added a c4-local lowest_not_ack selector matching picoquic_cc_get_lowest_not_ack: path packet context for multipath, connection Application packet context otherwise; era_check now uses it.
* C source: `picoquic/c4.c:459-473`
* C signature: `int c4_era_check(picoquic_path_t *, c4_state_t *)`
* Current Rust source: `rs/fq/src/c4.rs:299-320`
* Current Rust item: `era_check`

### C body
```c
{
    if (path_x->cnx->cnx_state < picoquic_state_ready) {
        return 0;
    }
    else {
        return (picoquic_cc_get_lowest_not_ack(path_x) > c4_state->era_sequence);
    }
}
```

### Current Rust body
```rust
    fn lowest_not_ack(path_x: &Path, connection: &Connection) -> u64 {
        let pkt_ctx = if connection.is_multipath_enabled {
            &path_x.pkt_ctx
        } else {
            &connection.pkt_ctx[PacketContext::Application as usize]
        };

        pkt_ctx
            .pending
            .keys()
            .next()
            .copied()
            .unwrap_or(pkt_ctx.highest_acknowledged.wrapping_add(1))
    }
```
