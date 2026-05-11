# Phase 4E repair confirmed translation mismatches

You are repairing Phase 4D `needs_fix` entries.  Phase 4D
already performed deeper classification and concluded that
these Rust translations need repair.

Rules:

* Edit Rust only.  Do not edit C sources.
* Keep edits limited to the owned Rust file(s) for this batch
  unless a directly related helper in `rs/fq/` must change.
* Preserve safe, idiomatic Rust and existing public API shape
  unless the current shape cannot express the C behavior.
* Do not replace code with stubs, placeholders, fabricated
  defaults, or weaker behavior.
* If deeper repair inspection proves Phase 4D was mistaken,
  report outcome `ok` and do not edit source.
* The driver will run a separate read-only re-triage before
  recording any `fixed` or `ok` result as resolved.
* Report `blocked` only with a concrete human-actionable
  reason.

Owned Rust file(s): `rs/fq/src/c4.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/c4.c:c4_era_check`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns a boolean-like check based on connection state and lowest-not-ack; Rust resets era fields and ECN state with no returned check.
* Phase 4D analysis: The actual Rust era_check is not era_reset, but deeper helper review found a real mismatch: it calls path_x.lowest_not_ack(), which always reads the path packet context, while C uses the connection application packet context when multipath is disabled.
* Phase 4D fix note: Make era_check use a connection-aware lowest-not-ack selection, matching picoquic_cc_get_lowest_not_ack: path pkt_ctx for multipath, connection pkt_ctx[Application] otherwise.
* C source: `picoquic/c4.c:459-473`
* C signature: `int c4_era_check(picoquic_path_t *, c4_state_t *)`
* Rust source: `rs/fq/src/c4.rs:299-319`
* Rust item: `era_check`

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

### Rust body
```rust
    fn era_reset(&mut self, path_x: &Path, connection: &Connection) {
        self.era_sequence = connection.sequence_number(path_x);
        self.era_max_rtt = 0;
        self.era_min_rtt = u64::MAX;
        self.alpha_1024_previous = self.alpha_1024_current;
        self.update_ecn_alpha(path_x, connection);
    }
```
