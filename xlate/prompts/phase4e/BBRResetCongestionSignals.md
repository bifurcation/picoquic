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

Owned Rust file(s): `rs/fq/src/bbr.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/bbr.c:BBRResetCongestionSignals`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C resets loss/latest congestion signal fields, while Rust resets lower-bound fields bw_lo and inflight_lo to max values.
* Phase 4D analysis: Phase 4C read the adjacent lower-bound reset, but the actual Rust `reset_congestion_signals` still omits the C `RTTJitterBuffer` reset of `rtt_too_high_in_round`.
* Phase 4D fix note: Add `self.rtt_too_high_in_round = false;` to `reset_congestion_signals`.
* C source: `picoquic/bbr.c:1014-1022`
* C signature: `void BBRResetCongestionSignals(picoquic_bbr_state_t *)`
* Rust source: `rs/fq/src/bbr.rs:652-665`
* Rust item: `reset_congestion_signals`

### C body
```c
{
    bbr_state->loss_in_round = 0;
#ifdef RTTJitterBuffer
    bbr_state->rtt_too_high_in_round = 0;
#endif
    bbr_state->bw_latest = 0;
    bbr_state->inflight_latest = 0;
}
```

### Rust body
```rust
    pub fn reset_lower_bounds(&mut self) {
        self.bw_lo = u64::MAX;
        self.inflight_lo = u64::MAX;
    }
```
